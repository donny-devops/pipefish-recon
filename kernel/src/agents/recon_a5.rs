//! RECON-A5 — Governance & Audit Governor.
//!
//! Enforces FIPS 203/205 policy, generates daily/weekly/monthly compliance
//! reports, writes the canonical audit row to Google Sheets for every
//! action, and signs every outbound artifact with the kernel's SLH-DSA key.
//! v0.2: long-running stub (local JSONL only).

use anyhow::Result;
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};
use tracing::info;

use crate::events::{BusEvent, CommitType, ContextId, ContextScope};

pub struct ReconA5 {
    bus_tx: mpsc::Sender<BusEvent>,
}

impl ReconA5 {
    pub fn new(bus_tx: mpsc::Sender<BusEvent>) -> Self {
        Self { bus_tx }
    }

    pub async fn run(&self) -> Result<()> {
        info!("RECON-A5 online");
        let boot = BusEvent::new(
            ContextId::ReconA5,
            CommitType::Feat,
            ContextScope::Dx,
            "RECON-A5 online (governance/audit stub)",
        );
        let _ = self.bus_tx.send(boot).await;

        let mut tick = interval(Duration::from_secs(60));
        tick.tick().await;
        loop {
            tick.tick().await;
            let hb = BusEvent::new(
                ContextId::ReconA5,
                CommitType::Chore,
                ContextScope::Dx,
                "RECON-A5 heartbeat",
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
    async fn recon_a5_boot_emits_event() {
        let (tx, mut rx) = mpsc::channel(16);
        let agent = ReconA5::new(tx);
        let handle = tokio::spawn(async move { agent.run().await });
        let evt = rx.recv().await.expect("event received");
        assert_eq!(evt.source, ContextId::ReconA5);
        assert_eq!(evt.description, "RECON-A5 online (governance/audit stub)");
        handle.abort();
    }
}
