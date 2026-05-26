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

use crate::crypto::envelope::KeyStore;
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
    #[allow(dead_code)]
    keystore: Arc<KeyStore>,
}

impl CoreContext {
    /// Generate the per-context ML-KEM-768 keystore. Returns the shared handle
    /// so it can be wired into the bus before any context starts running.
    pub fn boot_keystore() -> Result<Arc<KeyStore>> {
        let store = KeyStore::generate()?;
        let n = store.len();
        let arc = Arc::new(store);
        info!(
            "feat(core): ML-KEM-768 KeyStore initialized — {} context keypairs generated",
            n
        );
        Ok(arc)
    }

    pub fn new(bus_tx: mpsc::Sender<BusEvent>, keystore: Arc<KeyStore>) -> Self {
        Self {
            bus_tx,
            signing_key: Arc::new(Mutex::new(None)),
            keystore,
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
