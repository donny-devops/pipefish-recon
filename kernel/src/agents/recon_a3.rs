//! RECON-A3 — Decision & Routing Hub.
//!
//! Pure deterministic logic engine — no LLM on the hot path. Classifies
//! severity, applies NIST AI RMF risk-posture rules, and routes signals to
//! RECON-A4 directly or via the human-in-the-loop escalation queue.

use anyhow::Result;
use tokio::sync::mpsc;
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
            ContextId::Core,
            CommitType::Feat,
            ContextScope::Core,
            "RECON-A3 online (decision/routing stub)",
        );
        let _ = self.bus_tx.send(boot).await;
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
        agent.run().await.expect("agent run succeeds");
        let evt = rx.recv().await.expect("event received");
        assert_eq!(evt.description, "RECON-A3 online (decision/routing stub)");
    }
}
