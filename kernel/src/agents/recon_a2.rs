//! RECON-A2 — Threat Intelligence Synthesis.
//!
//! Enriches `ThreatSignal` → `ThreatContext`. Maps to MITRE ATT&CK
//! techniques, attempts actor attribution, and correlates TTPs across the
//! last 90 days of ingested signals via the `llm` context.
//! v0.2: long-running stub (no LLM providers).

use anyhow::Result;
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};
use tracing::info;

use crate::events::{BusEvent, CommitType, ContextId, ContextScope};

pub struct ReconA2 {
    bus_tx: mpsc::Sender<BusEvent>,
}

impl ReconA2 {
    pub fn new(bus_tx: mpsc::Sender<BusEvent>) -> Self {
        Self { bus_tx }
    }

    pub async fn run(&self) -> Result<()> {
        info!("RECON-A2 online");
        let boot = BusEvent::new(
            ContextId::ReconA2,
            CommitType::Feat,
            ContextScope::Llm,
            "RECON-A2 online (threat synthesis stub)",
        );
        let _ = self.bus_tx.send(boot).await;

        let mut tick = interval(Duration::from_secs(60));
        tick.tick().await;
        loop {
            tick.tick().await;
            let hb = BusEvent::new(
                ContextId::ReconA2,
                CommitType::Chore,
                ContextScope::Llm,
                "RECON-A2 heartbeat",
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
    async fn recon_a2_boot_emits_event() {
        let (tx, mut rx) = mpsc::channel(16);
        let agent = ReconA2::new(tx);
        let handle = tokio::spawn(async move { agent.run().await });
        let evt = rx.recv().await.expect("event received");
        assert_eq!(evt.source, ContextId::ReconA2);
        assert_eq!(evt.description, "RECON-A2 online (threat synthesis stub)");
        handle.abort();
    }
}
