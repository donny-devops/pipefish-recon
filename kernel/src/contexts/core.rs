//! CoreContext — the privileged root of the kernel.
//!
//! Responsibilities: lifecycle management, watchdog supervision, policy load
//! and enforcement, and sole custody of the SLH-DSA private key handle. No
//! other context is ever permitted to hold a signing key.

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tokio::time::{interval, Duration};
use tracing::info;

use crate::events::{BusEvent, CommitType, ContextId, ContextScope};
use crate::mcp::policy::ToolPolicy;

/// Opaque private-key handle. When the `pq-crypto` feature is enabled this
/// would wrap an `oqs::sig::SecretKey`; in the stub build it carries nothing.
#[allow(dead_code)]
#[derive(Debug, Default)]
pub struct PrivateKeyHandle;

pub struct CoreContext {
    bus_tx: mpsc::Sender<BusEvent>,
    #[allow(dead_code)]
    signing_key: Arc<Mutex<Option<PrivateKeyHandle>>>,
    policy: Arc<ToolPolicy>,
}

impl CoreContext {
    pub fn new(bus_tx: mpsc::Sender<BusEvent>, policy: Arc<ToolPolicy>) -> Self {
        Self {
            bus_tx,
            signing_key: Arc::new(Mutex::new(None)),
            policy,
        }
    }

    pub fn policy(&self) -> &ToolPolicy {
        &self.policy
    }

    pub async fn run(&self) -> Result<()> {
        info!(
            rules = self.policy.allow_count(),
            "core context online — policy plane initializing"
        );

        let boot = BusEvent::new(
            ContextId::Core,
            CommitType::Chore,
            ContextScope::Core,
            format!(
                "policy loaded; {} allow-rules; watchdog armed",
                self.policy.allow_count()
            ),
        );
        let _ = self.bus_tx.send(boot).await;

        let mut tick = interval(Duration::from_secs(5));
        tick.tick().await;
        loop {
            tick.tick().await;
            let hb = BusEvent::new(
                ContextId::Core,
                CommitType::Chore,
                ContextScope::Core,
                "healthcheck ok",
            );
            if self.bus_tx.send(hb).await.is_err() {
                info!("bus closed; core exiting");
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
    async fn core_boot_mentions_allow_rule_count() {
        let mut policy = ToolPolicy::new();
        policy.allow("recon-a1", "cve-feed");
        let (tx, mut rx) = mpsc::channel(16);
        let core = CoreContext::new(tx, Arc::new(policy));
        let handle = tokio::spawn(async move { core.run().await });
        let evt = rx.recv().await.expect("boot event");
        assert!(evt.description.contains("1 allow-rules"));
        handle.abort();
    }
}
