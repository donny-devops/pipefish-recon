//! RECON-A4 — Autonomous Defense Engine.
//!
//! Generates and (under policy gate) executes defensive actions: nftables
//! rules, Tailscale ACL patches, WAF policy updates, network segmentation
//! recommendations, and GitHub Actions PRs for upstream patch application.
//! v0.2: long-running stub (no network mutation).

use anyhow::Result;
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};
use tracing::info;

use crate::events::{BusEvent, CommitType, ContextId, ContextScope};

pub struct ReconA4 {
    bus_tx: mpsc::Sender<BusEvent>,
}

impl ReconA4 {
    pub fn new(bus_tx: mpsc::Sender<BusEvent>) -> Self {
        Self { bus_tx }
    }

    pub async fn run(&self) -> Result<()> {
        info!("RECON-A4 online");
        let boot = BusEvent::new(
            ContextId::ReconA4,
            CommitType::Feat,
            ContextScope::Tool,
            "RECON-A4 online (autonomous defense stub)",
        );
        let _ = self.bus_tx.send(boot).await;

        let mut tick = interval(Duration::from_secs(60));
        tick.tick().await;
        loop {
            tick.tick().await;
            let hb = BusEvent::new(
                ContextId::ReconA4,
                CommitType::Chore,
                ContextScope::Tool,
                "RECON-A4 heartbeat",
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
    async fn recon_a4_boot_emits_event() {
        let (tx, mut rx) = mpsc::channel(16);
        let agent = ReconA4::new(tx);
        let handle = tokio::spawn(async move { agent.run().await });
        let evt = rx.recv().await.expect("event received");
        assert_eq!(evt.source, ContextId::ReconA4);
        assert_eq!(evt.description, "RECON-A4 online (autonomous defense stub)");
        handle.abort();
    }
}
