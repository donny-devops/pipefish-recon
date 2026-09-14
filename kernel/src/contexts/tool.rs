//! ToolContext — MCP server registry and sandboxed tool dispatch (stub).
//!
//! In production every registered MCP server runs inside a microsandbox
//! microVM with kernel-issued mTLS for SSE transports. The skeleton holds
//! an empty registry and consults the loaded [`ToolPolicy`] before any
//! future dispatch.

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};
use tracing::info;

use crate::events::{BusEvent, CommitType, ContextId, ContextScope};
use crate::mcp::policy::ToolPolicy;
use crate::mcp::registry::ToolRegistry;

pub struct ToolContext {
    bus_tx: mpsc::Sender<BusEvent>,
    #[allow(dead_code)]
    registry: ToolRegistry,
    policy: Arc<ToolPolicy>,
}

impl ToolContext {
    pub fn new(bus_tx: mpsc::Sender<BusEvent>, policy: Arc<ToolPolicy>) -> Self {
        Self {
            bus_tx,
            registry: ToolRegistry::new(),
            policy,
        }
    }

    /// Policy gate for a future MCP dispatch. Deny-by-default.
    pub fn check_dispatch(&self, agent_id: &str, tool_name: &str) -> bool {
        self.policy.is_allowed(agent_id, tool_name)
    }

    pub async fn run(&self) -> Result<()> {
        info!("tool context online — MCP registry empty");

        let boot = BusEvent::new(
            ContextId::Tool,
            CommitType::Chore,
            ContextScope::Tool,
            format!(
                "MCP registry initialized (empty); policy allow-rules={}",
                self.policy.allow_count()
            ),
        );
        let _ = self.bus_tx.send(boot).await;

        let mut tick = interval(Duration::from_secs(60));
        tick.tick().await;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dispatch_respects_deny_by_default() {
        let mut policy = ToolPolicy::new();
        policy.allow("recon-a1", "cve-feed");
        let (tx, _rx) = mpsc::channel(8);
        let tool = ToolContext::new(tx, Arc::new(policy));
        assert!(tool.check_dispatch("recon-a1", "cve-feed"));
        assert!(!tool.check_dispatch("recon-a1", "nftables"));
        assert!(!tool.check_dispatch("recon-a4", "cve-feed"));
    }
}
