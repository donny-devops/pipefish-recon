//! LlmContext — LLM orchestration (stub).
//!
//! In production this is the only context permitted to make network egress
//! to allow-listed LLM endpoints. For the skeleton it just announces itself
//! on the bus and idles.

use anyhow::Result;
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};
use tracing::info;

use crate::events::{BusEvent, CommitType, ContextId, ContextScope};

pub struct LlmContext {
    bus_tx: mpsc::Sender<BusEvent>,
}

impl LlmContext {
    pub fn new(bus_tx: mpsc::Sender<BusEvent>) -> Self {
        Self { bus_tx }
    }

    pub async fn run(&self) -> Result<()> {
        info!("llm context online — orchestrator ready");

        let boot = BusEvent::new(
            ContextId::Llm,
            CommitType::Chore,
            ContextScope::Llm,
            "orchestrator ready; no providers wired",
        );
        let _ = self.bus_tx.send(boot).await;

        let mut tick = interval(Duration::from_secs(30));
        loop {
            tick.tick().await;
            let idle = BusEvent::new(
                ContextId::Llm,
                CommitType::Chore,
                ContextScope::Llm,
                "idle tick; budget=unspent",
            );
            if self.bus_tx.send(idle).await.is_err() {
                break;
            }
        }
        Ok(())
    }
}
