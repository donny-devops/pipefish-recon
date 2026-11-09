//! RECON-A3 — Decision & Routing Hub.
//!
//! Pure deterministic logic — no LLM on the hot path. Consumes
//! `ThreatSignal` payloads from the bus, classifies severity, and emits
//! a routing commit. HIL queue apply is v0.5; this slice records the decision.

use anyhow::Result;
use tokio::sync::{mpsc, Mutex};
use tokio::time::{interval, Duration};
use tracing::info;

use crate::events::{BusEvent, CommitType, ContextId, ContextScope};
use crate::routing::{classify, route_for};
use crate::signals::ThreatSignal;

pub struct ReconA3 {
    bus_tx: mpsc::Sender<BusEvent>,
    bus_rx: Mutex<mpsc::Receiver<BusEvent>>,
}

impl ReconA3 {
    pub fn new(bus_tx: mpsc::Sender<BusEvent>, bus_rx: mpsc::Receiver<BusEvent>) -> Self {
        Self {
            bus_tx,
            bus_rx: Mutex::new(bus_rx),
        }
    }

    fn route_event(signal: &ThreatSignal) -> BusEvent {
        let sev = classify(signal);
        let dest = route_for(sev);
        BusEvent::new(
            ContextId::ReconA3,
            CommitType::Feat,
            ContextScope::Core,
            format!(
                "routed signal {} sev={} to {}",
                signal.cve_id,
                sev.as_str(),
                dest.as_str()
            ),
        )
        .with_severity(sev)
        .with_payload(serde_json::json!({
            "cve_id": signal.cve_id,
            "route": dest.as_str(),
            "threat_signal": signal,
        }))
    }

    pub async fn run(&self) -> Result<()> {
        info!("RECON-A3 online");
        let boot = BusEvent::new(
            ContextId::ReconA3,
            CommitType::Feat,
            ContextScope::Core,
            "RECON-A3 online (deterministic severity router)",
        );
        let _ = self.bus_tx.send(boot).await;

        let mut rx = self.bus_rx.lock().await;
        let mut tick = interval(Duration::from_secs(60));
        tick.tick().await;
        loop {
            tokio::select! {
                evt = rx.recv() => {
                    match evt {
                        Some(evt) => {
                            if evt.source != ContextId::ReconA1 {
                                continue;
                            }
                            let Some(payload) = evt.payload else { continue };
                            let Ok(signal) = serde_json::from_value::<ThreatSignal>(payload) else {
                                continue;
                            };
                            let routed = Self::route_event(&signal);
                            if self.bus_tx.send(routed).await.is_err() {
                                break;
                            }
                        }
                        None => break,
                    }
                }
                _ = tick.tick() => {
                    let hb = BusEvent::new(
                        ContextId::ReconA3,
                        CommitType::Chore,
                        ContextScope::Core,
                        "RECON-A3 heartbeat",
                    );
                    if self.bus_tx.send(hb).await.is_err() {
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
    use crate::events::Severity;
    use crate::signals::{SignalSource, ThreatSignal};
    use chrono::Utc;
    use uuid::Uuid;

    #[tokio::test]
    async fn recon_a3_boot_emits_event() {
        let (tx, mut rx) = mpsc::channel(16);
        let (_bus_tx, bus_rx) = mpsc::channel(8);
        let agent = ReconA3::new(tx, bus_rx);
        let handle = tokio::spawn(async move { agent.run().await });
        let evt = rx.recv().await.expect("event received");
        assert_eq!(evt.source, ContextId::ReconA3);
        assert!(evt.description.contains("severity router"));
        handle.abort();
    }

    #[tokio::test]
    async fn routes_ingested_critical_signal_to_hil() {
        let (out_tx, mut out_rx) = mpsc::channel(16);
        let (in_tx, in_rx) = mpsc::channel(8);
        let agent = ReconA3::new(out_tx, in_rx);
        let handle = tokio::spawn(async move { agent.run().await });
        let _boot = out_rx.recv().await.expect("boot");

        let signal = ThreatSignal {
            id: Uuid::new_v4(),
            cve_id: "CVE-2026-1337".into(),
            source: SignalSource::Nvd,
            cvss: Some(9.8),
            summary: "rce".into(),
            published: None,
            ingested_at: Utc::now(),
            exploit_available: false,
            kev: false,
        };
        let inbound = BusEvent::new(
            ContextId::ReconA1,
            CommitType::Feat,
            ContextScope::Bus,
            "ingested CVE-2026-1337 signal (cvss=9.8, source=nvd)",
        )
        .with_payload(serde_json::to_value(&signal).unwrap());
        in_tx.send(inbound).await.unwrap();

        let routed = out_rx.recv().await.expect("routed");
        assert_eq!(routed.source, ContextId::ReconA3);
        assert_eq!(routed.severity, Some(Severity::Critical));
        assert!(routed.description.contains("sev=CRITICAL"));
        assert!(routed.description.contains("HIL escalation queue"));
        handle.abort();
    }
}
