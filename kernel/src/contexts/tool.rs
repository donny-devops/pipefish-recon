//! ToolContext — MCP server registry and sandboxed tool dispatch (stub).
//!
//! In production every registered MCP server runs inside a microsandbox
//! microVM with kernel-issued mTLS for SSE transports. The skeleton just
//! holds an empty registry and idles.

use anyhow::Result;
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};
use tracing::info;

use crate::events::{BusEvent, CommitType, ContextId, ContextScope};
use crate::mcp::registry::ToolRegistry;

pub struct ToolContext {
    bus_tx: mpsc::Sender<BusEvent>,
    #[allow(dead_code)]
    registry: ToolRegistry,
}

impl ToolContext {
    pub fn new(bus_tx: mpsc::Sender<BusEvent>) -> Self {
        Self {
            bus_tx,
            registry: ToolRegistry::new(),
        }
    }

    pub async fn run(&self) -> Result<()> {
        info!("tool context online — MCP registry empty");

        let boot = BusEvent::new(
            ContextId::Tool,
            CommitType::Chore,
            ContextScope::Tool,
            "MCP registry initialized (empty)",
        );
        let _ = self.bus_tx.send(boot).await;

        let mut tick = interval(Duration::from_secs(60));
        loop {
            tick.tick().await;
            let idle = BusEvent::new(
                ContextId::Tool,
                CommitType::Chore,
                ContextScope::Tool,
                "registry idle; 0 microVMs warm",
            );
            if self.bus_tx.send(idle).await.is_err() {
                break;
            }
        }
        Ok(())
    }
}
