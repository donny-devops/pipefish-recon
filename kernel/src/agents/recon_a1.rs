//! RECON-A1 — Signal Ingestion & Normalization.
//!
//! Consumes CVE/NVD feeds, OSINT (Shodan, GreyNoise, AbuseIPDB), SIEM
//! webhooks, and honeypot events. Normalizes everything to a canonical
//! `ThreatSignal` schema and emits one commit per accepted signal.
//! v0.2: long-running stub (no external feeds).

use anyhow::Result;
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};
use tracing::info;

use crate::events::{BusEvent, CommitType, ContextId, ContextScope};

pub struct ReconA1 {
    bus_tx: mpsc::Sender<BusEvent>,
}

impl ReconA1 {
    pub fn new(bus_tx: mpsc::Sender<BusEvent>) -> Self {
        Self { bus_tx }
    }

    pub async fn run(&self) -> Result<()> {
        info!("RECON-A1 online");
        let boot = BusEvent::new(
            ContextId::ReconA1,
            CommitType::Feat,
            ContextScope::Bus,
            "RECON-A1 online (signal ingestion stub)",
        );
        let _ = self.bus_tx.send(boot).await;

        let mut tick = interval(Duration::from_secs(60));
        tick.tick().await;
        loop {
            tick.tick().await;
            let hb = BusEvent::new(
                ContextId::ReconA1,
                CommitType::Chore,
                ContextScope::Bus,
                "RECON-A1 heartbeat",
            );
            if self.bus_tx.send(hb).await.is_err() {
                break;
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
        let agent = ReconA1::new(tx);
        let handle = tokio::spawn(async move { agent.run().await });
        let evt = rx.recv().await.expect("event received");
        assert_eq!(evt.source, ContextId::ReconA1);
        assert_eq!(evt.description, "RECON-A1 online (signal ingestion stub)");
        handle.abort();
    }
}
