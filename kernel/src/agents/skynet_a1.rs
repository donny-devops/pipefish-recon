//! SKYNET-A1 — Signal Ingestion & Normalization.
//!
//! Polls the NVD CVE 2.0 feed on a configurable interval, normalizes each item
//! into a [`ThreatSignal`], filters noise, and emits one [`BusEvent`] per
//! accepted signal carrying the signal as its JSON payload.

use anyhow::Result;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

use crate::events::{BusEvent, CommitType, ContextId, ContextScope, Severity};
use crate::signals::{NvdFeedPoller, ThreatSignal};

pub struct SkynetA1 {
    bus_tx: mpsc::Sender<BusEvent>,
    poller: tokio::sync::Mutex<NvdFeedPoller>,
}

impl SkynetA1 {
    pub fn new(bus_tx: mpsc::Sender<BusEvent>) -> Self {
        Self {
            bus_tx,
            poller: tokio::sync::Mutex::new(NvdFeedPoller::new()),
        }
    }

    pub async fn run(&self) -> Result<()> {
        info!("SKYNET-A1 online — signal ingestion armed");

        let boot = BusEvent::new(
            ContextId::Bus,
            CommitType::Feat,
            ContextScope::Bus,
            "SKYNET-A1 online (signal ingestion)",
        );
        if let Err(e) = self.bus_tx.send(boot).await {
            error!(error = %e, "SKYNET-A1 boot event send failed");
            return Ok(());
        }

        let debug = std::env::var("SKYNET_DEBUG").ok().as_deref() == Some("1");
        let interval_secs = {
            let p = self.poller.lock().await;
            p.poll_interval_secs()
        };

        loop {
            let poll_result = {
                let mut p = self.poller.lock().await;
                p.poll().await
            };

            match poll_result {
                Ok(signals) => {
                    info!(count = signals.len(), "SKYNET-A1 poll cycle complete");
                    for signal in signals {
                        if !debug && matches!(signal.severity, Severity::Info) {
                            continue;
                        }
                        if let Err(e) = self.emit_signal(signal).await {
                            warn!(error = %e, "failed to emit threat signal");
                        }
                    }
                }
                Err(e) => {
                    error!(error = %e, "SKYNET-A1 poll failed — continuing");
                }
            }

            if self.bus_tx.is_closed() {
                info!("SKYNET-A1: bus closed, shutting down");
                return Ok(());
            }

            tokio::time::sleep(std::time::Duration::from_secs(interval_secs)).await;
        }
    }

    async fn emit_signal(&self, signal: ThreatSignal) -> Result<()> {
        let cve_label = signal
            .cve_id
            .clone()
            .unwrap_or_else(|| signal.title.clone());
        let sev_label = severity_label(&signal.severity);
        let description = format!("ingested {} [{}]", cve_label, sev_label);

        let payload = serde_json::to_value(&signal)?;

        let event = BusEvent::new(
            ContextId::Bus,
            CommitType::Feat,
            ContextScope::Bus,
            description,
        )
        .with_severity(signal.severity)
        .with_payload(payload);

        info!("{}", event.to_commit_string());
        self.bus_tx
            .send(event)
            .await
            .map_err(|e| anyhow::anyhow!("bus send failed: {e}"))?;
        Ok(())
    }
}

fn severity_label(severity: &Severity) -> &'static str {
    match severity {
        Severity::Critical => "CRITICAL",
        Severity::High => "HIGH",
        Severity::Medium => "MEDIUM",
        Severity::Low => "LOW",
        Severity::Info => "INFO",
    }
}
