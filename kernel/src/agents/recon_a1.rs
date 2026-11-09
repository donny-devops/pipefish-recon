//! RECON-A1 — Signal Ingestion & Normalization.
//!
//! Polls NVD CVE 2.0 (optional), accepts webhook ingest, normalizes to
//! [`ThreatSignal`], de-duplicates on a 30-day window, and emits one
//! Conventional Commit per accepted signal.

use anyhow::Result;
use std::time::Duration as StdDuration;
use tokio::sync::{mpsc, Mutex};
use tokio::time::{interval, Duration};
use tracing::{info, warn};

use crate::events::{BusEvent, CommitType, ContextId, ContextScope};
use crate::signals::{parse_nvd_cve_2_0, Deduper, IngestRequest, SignalSource, ThreatSignal};

pub const DEFAULT_NVD_URL: &str =
    "https://services.nvd.nist.gov/rest/json/cves/2.0?resultsPerPage=20";
pub const DEFAULT_NVD_INTERVAL_SECS: u64 = 900;

pub struct NvdPoller {
    pub enabled: bool,
    pub url: String,
    pub interval: Duration,
}

impl NvdPoller {
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            url: DEFAULT_NVD_URL.to_string(),
            interval: Duration::from_secs(DEFAULT_NVD_INTERVAL_SECS),
        }
    }

    pub fn from_env() -> Self {
        let enabled = std::env::var("PIPEFISH_NVD_POLL")
            .map(|v| v != "0")
            .unwrap_or(true);
        let url = std::env::var("PIPEFISH_NVD_URL").unwrap_or_else(|_| DEFAULT_NVD_URL.to_string());
        let secs = std::env::var("PIPEFISH_NVD_INTERVAL_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_NVD_INTERVAL_SECS);
        Self {
            enabled,
            url,
            interval: Duration::from_secs(secs.max(5)),
        }
    }
}

pub struct ReconA1 {
    bus_tx: mpsc::Sender<BusEvent>,
    ingest_rx: Mutex<mpsc::Receiver<IngestRequest>>,
    poller: NvdPoller,
    deduper: Mutex<Deduper>,
}

impl ReconA1 {
    pub fn new(
        bus_tx: mpsc::Sender<BusEvent>,
        ingest_rx: mpsc::Receiver<IngestRequest>,
        poller: NvdPoller,
    ) -> Self {
        Self {
            bus_tx,
            ingest_rx: Mutex::new(ingest_rx),
            poller,
            deduper: Mutex::new(Deduper::new()),
        }
    }

    async fn emit_signal(&self, signal: ThreatSignal) -> Result<bool> {
        let mut deduper = self.deduper.lock().await;
        if !deduper.accept(&signal.cve_id, signal.ingested_at) {
            return Ok(false);
        }
        drop(deduper);
        let cvss = signal
            .cvss
            .map(|s| format!("{s:.1}"))
            .unwrap_or_else(|| "n/a".into());
        let evt = BusEvent::new(
            ContextId::ReconA1,
            CommitType::Feat,
            ContextScope::Bus,
            format!(
                "ingested {} signal (cvss={}, source={})",
                signal.cve_id,
                cvss,
                signal.source.as_str()
            ),
        )
        .with_payload(serde_json::to_value(&signal)?);
        let _ = self.bus_tx.send(evt).await;
        Ok(true)
    }

    async fn poll_nvd(&self) -> Result<usize> {
        let client = reqwest::Client::builder()
            .user_agent("pipefish-recon-kernel/0.3")
            .timeout(StdDuration::from_secs(20))
            .build()?;
        let body = client.get(&self.poller.url).send().await?.text().await?;
        let signals = parse_nvd_cve_2_0(&body)?;
        let mut accepted = 0;
        for signal in signals {
            if self.emit_signal(signal).await? {
                accepted += 1;
            }
        }
        Ok(accepted)
    }

    pub async fn run(&self) -> Result<()> {
        info!(nvd_poll = self.poller.enabled, "RECON-A1 online");
        let boot = BusEvent::new(
            ContextId::ReconA1,
            CommitType::Feat,
            ContextScope::Bus,
            if self.poller.enabled {
                "RECON-A1 online (NVD poller + webhook ingest)"
            } else {
                "RECON-A1 online (webhook ingest; NVD poll disabled)"
            },
        );
        let _ = self.bus_tx.send(boot).await;

        let mut ingest_rx = self.ingest_rx.lock().await;
        let mut hb = interval(Duration::from_secs(60));
        hb.tick().await;
        let mut poll = interval(self.poller.interval);
        if !self.poller.enabled {
            poll = interval(Duration::from_secs(u64::MAX / 4));
        }
        poll.tick().await; // skip immediate fire; first poll after one interval (or 5s min)
        if self.poller.enabled {
            // First poll shortly after boot so a local run is observable.
            match self.poll_nvd().await {
                Ok(n) => info!(accepted = n, "NVD poll complete"),
                Err(e) => warn!("NVD poll failed: {e}"),
            }
        }

        loop {
            tokio::select! {
                req = ingest_rx.recv() => {
                    match req {
                        Some(req) => {
                            let signal = ThreatSignal::from_ingest(req);
                            match self.emit_signal(signal).await {
                                Ok(true) => {}
                                Ok(false) => info!("duplicate signal dropped"),
                                Err(e) => warn!("ingest emit failed: {e}"),
                            }
                        }
                        None => break,
                    }
                }
                _ = poll.tick(), if self.poller.enabled => {
                    match self.poll_nvd().await {
                        Ok(n) => info!(accepted = n, "NVD poll complete"),
                        Err(e) => warn!("NVD poll failed: {e}"),
                    }
                }
                _ = hb.tick() => {
                    let beat = BusEvent::new(
                        ContextId::ReconA1,
                        CommitType::Chore,
                        ContextScope::Bus,
                        "RECON-A1 heartbeat",
                    );
                    if self.bus_tx.send(beat).await.is_err() {
                        break;
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn recon_a1_boot_emits_event() {
        let (tx, mut rx) = mpsc::channel(16);
        let (_ingest_tx, ingest_rx) = mpsc::channel(4);
        let agent = ReconA1::new(tx, ingest_rx, NvdPoller::disabled());
        let handle = tokio::spawn(async move { agent.run().await });
        let evt = rx.recv().await.expect("event received");
        assert_eq!(evt.source, ContextId::ReconA1);
        assert!(evt.description.contains("webhook ingest"));
        handle.abort();
    }

    #[tokio::test]
    async fn webhook_ingest_emits_threat_signal() {
        let (tx, mut rx) = mpsc::channel(16);
        let (ingest_tx, ingest_rx) = mpsc::channel(4);
        let agent = ReconA1::new(tx, ingest_rx, NvdPoller::disabled());
        let handle = tokio::spawn(async move { agent.run().await });
        let _boot = rx.recv().await.expect("boot");
        ingest_tx
            .send(IngestRequest {
                cve_id: "cve-2026-1337".into(),
                cvss: Some(9.8),
                summary: Some("rce".into()),
                source: Some(SignalSource::Webhook),
                published: None,
                exploit_available: Some(false),
                kev: Some(false),
            })
            .await
            .expect("ingest");
        let evt = rx.recv().await.expect("ingested");
        assert!(evt.description.contains("CVE-2026-1337"));
        assert!(evt.description.contains("cvss=9.8"));
        let payload = evt.payload.expect("payload");
        assert_eq!(payload["cve_id"], "CVE-2026-1337");
        handle.abort();
    }

    #[tokio::test]
    async fn duplicate_cve_is_dropped() {
        let (tx, mut rx) = mpsc::channel(16);
        let (ingest_tx, ingest_rx) = mpsc::channel(4);
        let agent = ReconA1::new(tx, ingest_rx, NvdPoller::disabled());
        let handle = tokio::spawn(async move { agent.run().await });
        let _boot = rx.recv().await.expect("boot");
        let req = IngestRequest {
            cve_id: "CVE-2026-1337".into(),
            cvss: Some(5.0),
            summary: None,
            source: None,
            published: None,
            exploit_available: None,
            kev: None,
        };
        ingest_tx.send(req.clone()).await.unwrap();
        ingest_tx.send(req).await.unwrap();
        let first = rx.recv().await.expect("first");
        assert!(first.description.contains("ingested"));
        let second = tokio::time::timeout(Duration::from_millis(80), rx.recv()).await;
        assert!(second.is_err() || second.unwrap().unwrap().description.contains("heartbeat"));
        handle.abort();
    }
}
