//! RECON-A1 — Signal Ingestion & Normalization.
//!
//! Consumes CVE/NVD feeds, OSINT (Shodan, GreyNoise, AbuseIPDB), SIEM
//! webhooks, and honeypot events. Normalizes everything to a canonical
//! `ThreatSignal` schema and emits one commit per accepted signal.

use anyhow::Result;
use tokio::sync::mpsc;
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
            ContextId::Bus,
            CommitType::Feat,
            ContextScope::Bus,
            "RECON-A1 online (signal ingestion stub)",
        );
        let _ = self.bus_tx.send(boot).await;
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
        agent.run().await.expect("agent run succeeds");
        let evt = rx.recv().await.expect("event received");
        assert_eq!(evt.description, "RECON-A1 online (signal ingestion stub)");
    }
}
