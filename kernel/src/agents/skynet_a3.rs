//! SKYNET-A3 — Decision & Routing Hub.
//!
//! Pure deterministic logic engine — no LLM on the hot path. Classifies
//! severity, applies NIST AI RMF risk-posture rules, and routes signals to
//! SKYNET-A4 directly or via the human-in-the-loop escalation queue.

use anyhow::Result;
use tokio::sync::mpsc;
use tracing::info;

use crate::events::{BusEvent, CommitType, ContextId, ContextScope};

pub struct SkynetA3 {
    bus_tx: mpsc::Sender<BusEvent>,
}

impl SkynetA3 {
    pub fn new(bus_tx: mpsc::Sender<BusEvent>) -> Self {
        Self { bus_tx }
    }

    pub async fn run(&self) -> Result<()> {
        info!("SKYNET-A3 online");
        let boot = BusEvent::new(
            ContextId::Core,
            CommitType::Feat,
            ContextScope::Core,
            "SKYNET-A3 online (decision/routing stub)",
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
        let agent = SkynetA3::new(tx);
        agent.run().await.expect("run should succeed");
        let evt = rx.recv().await.expect("expected a boot event");
        assert!(evt.description.contains("SKYNET-A3"));
    }

    #[tokio::test]
    async fn run_returns_ok() {
        let (tx, _rx) = mpsc::channel(8);
        let agent = SkynetA3::new(tx);
        assert!(agent.run().await.is_ok());
    }
}
