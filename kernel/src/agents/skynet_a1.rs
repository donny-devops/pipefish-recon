//! SKYNET-A1 — Signal Ingestion & Normalization.
//!
//! Consumes CVE/NVD feeds, OSINT (Shodan, GreyNoise, AbuseIPDB), SIEM
//! webhooks, and honeypot events. Normalizes everything to a canonical
//! `ThreatSignal` schema and emits one commit per accepted signal.

use anyhow::Result;
use tokio::sync::mpsc;
use tracing::info;

use crate::events::{BusEvent, CommitType, ContextId, ContextScope};

pub struct SkynetA1 {
    bus_tx: mpsc::Sender<BusEvent>,
}

impl SkynetA1 {
    pub fn new(bus_tx: mpsc::Sender<BusEvent>) -> Self {
        Self { bus_tx }
    }

    pub async fn run(&self) -> Result<()> {
        info!("SKYNET-A1 online");
        let boot = BusEvent::new(
            ContextId::Bus,
            CommitType::Feat,
            ContextScope::Bus,
            "SKYNET-A1 online (signal ingestion stub)",
        );
        let _ = self.bus_tx.send(boot).await;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn run_sends_boot_event_to_bus() {
        let (tx, mut rx) = mpsc::channel(8);
        let agent = SkynetA1::new(tx);
        agent.run().await.expect("run should succeed");
        let evt = rx.recv().await.expect("expected a boot event");
        assert!(evt.description.contains("SKYNET-A1"));
    }

    #[tokio::test]
    async fn run_returns_ok() {
        let (tx, _rx) = mpsc::channel(8);
        let agent = SkynetA1::new(tx);
        assert!(agent.run().await.is_ok());
    }
}
