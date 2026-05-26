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
