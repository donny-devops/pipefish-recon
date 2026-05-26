//! SKYNET-A2 — Threat Intelligence Synthesis.
//!
//! Enriches `ThreatSignal` → `ThreatContext`. Maps to MITRE ATT&CK
//! techniques, attempts actor attribution, and correlates TTPs across the
//! last 90 days of ingested signals via the `llm` context.

use anyhow::Result;
use tokio::sync::mpsc;
use tracing::info;

use crate::events::{BusEvent, CommitType, ContextId, ContextScope};

pub struct SkynetA2 {
    bus_tx: mpsc::Sender<BusEvent>,
}

impl SkynetA2 {
    pub fn new(bus_tx: mpsc::Sender<BusEvent>) -> Self {
        Self { bus_tx }
    }

    pub async fn run(&self) -> Result<()> {
        info!("SKYNET-A2 online");
        let boot = BusEvent::new(
            ContextId::Llm,
            CommitType::Feat,
            ContextScope::Llm,
            "SKYNET-A2 online (threat synthesis stub)",
        );
        let _ = self.bus_tx.send(boot).await;
        Ok(())
    }
}
