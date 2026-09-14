//! RECON-A3 — Decision & Routing Hub.
//!
//! Pure deterministic logic engine — no LLM on the hot path. Classifies
//! severity, applies NIST AI RMF risk-posture rules, and routes signals to
//! RECON-A4 directly or via the human-in-the-loop escalation queue.
//! v0.2: long-running stub (no live routing).

use anyhow::Result;
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};
use tracing::info;

use crate::events::{BusEvent, CommitType, ContextId, ContextScope};

pub struct ReconA3 {
    bus_tx: mpsc::Sender<BusEvent>,
}

impl ReconA3 {
    pub fn new(bus_tx: mpsc::Sender<BusEvent>) -> Self {
        Self { bus_tx }
    }

    pub async fn run(&self) -> Result<()> {
        info!("RECON-A3 online");
        let boot = BusEvent::new(
            ContextId::ReconA3,
            CommitType::Feat,
            ContextScope::Core,
            "RECON-A3 online (decision/routing stub)",
        );
        let _ = self.bus_tx.send(boot).await;

        let mut tick = interval(Duration::from_secs(60));
        tick.tick().await;
        loop {
            tick.tick().await;
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
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn recon_a3_boot_emits_event() {
        let (tx, mut rx) = mpsc::channel(16);
        let agent = ReconA3::new(tx);
        let handle = tokio::spawn(async move { agent.run().await });
        let evt = rx.recv().await.expect("event received");
        assert_eq!(evt.source, ContextId::ReconA3);
        assert_eq!(evt.description, "RECON-A3 online (decision/routing stub)");
        handle.abort();
    }
}
