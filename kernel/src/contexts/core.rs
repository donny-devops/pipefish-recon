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

/// Opaque private-key handle. When the `pq-crypto` feature is enabled this
/// would wrap an `oqs::sig::SecretKey`; in the stub build it carries nothing.
#[allow(dead_code)]
#[derive(Debug, Default)]
pub struct PrivateKeyHandle;

pub struct CoreContext {
    bus_tx: mpsc::Sender<BusEvent>,
    #[allow(dead_code)]
    signing_key: Arc<Mutex<Option<PrivateKeyHandle>>>,
}

impl CoreContext {
    pub fn new(bus_tx: mpsc::Sender<BusEvent>) -> Self {
        Self {
            bus_tx,
            signing_key: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn run(&self) -> Result<()> {
        info!("core context online — policy plane initializing");

        let boot = BusEvent::new(
            ContextId::Core,
            CommitType::Chore,
            ContextScope::Core,
            "policy loaded; watchdog armed",
        );
        let _ = self.bus_tx.send(boot).await;

        let mut tick = interval(Duration::from_secs(5));
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
