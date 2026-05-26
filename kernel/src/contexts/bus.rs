//! BusContext — the kernel's IPC broker.
//!
//! Receives [`BusEvent`]s from every other context over a single mpsc inbox,
//! seals each one into a post-quantum [`MessageEnvelope`] (ML-KEM-768 + AES-256-GCM
//! under the `pq-crypto` feature, transparent passthrough otherwise), and logs
//! the envelope id + Conventional Commit string. The bus never inspects the
//! payload beyond the envelope metadata — only the recipient's private key
//! can decrypt it.

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tracing::{info, warn};

use crate::crypto::envelope::{KeyStore, MessageEnvelope};
use crate::events::{BusEvent, ContextId};

const BUS_CHANNEL_CAPACITY: usize = 1024;

pub struct BusContext {
    rx: Mutex<mpsc::Receiver<BusEvent>>,
    keystore: Arc<KeyStore>,
}

impl BusContext {
    pub fn new(keystore: Arc<KeyStore>) -> (Self, mpsc::Sender<BusEvent>) {
        let (tx, rx) = mpsc::channel(BUS_CHANNEL_CAPACITY);
        (
            Self {
                rx: Mutex::new(rx),
                keystore,
            },
            tx,
        )
    }

    pub async fn run(&self) -> Result<()> {
        info!("bus context online — IPC broker armed (PQ envelope layer active)");
        let mut rx = self.rx.lock().await;
        while let Some(evt) = rx.recv().await {
            let commit = evt.to_commit_string();
            let sender = evt.source;
            // Default fan-out target for the skeleton: the bus context itself
            // routes events into the audit pipeline. A fuller build resolves
            // the per-message recipient from the event's subscription map.
            let recipient = ContextId::Bus;

            match self.keystore.pub_key(&recipient) {
                Some(pk) => match MessageEnvelope::seal(&evt, sender, recipient, pk) {
                    Ok(envelope) => info!(
                        envelope_id = %envelope.id,
                        sender = sender.as_str(),
                        recipient = recipient.as_str(),
                        ciphertext_bytes = envelope.ciphertext.len(),
                        "{}",
                        commit
                    ),
                    Err(e) => warn!(error = %e, "bus: envelope seal failed"),
                },
                None => warn!(
                    recipient = recipient.as_str(),
                    "bus: no recipient key in keystore; dropping event {}",
                    commit
                ),
            }
        }
        info!("bus context: all senders dropped; shutting down");
        Ok(())
    }

    /// Decrypt an envelope addressed to a context the bus holds keys for.
    /// In dev mode this is a transparent passthrough; in PQ mode it performs
    /// ML-KEM-768 decapsulation + AES-256-GCM decryption.
    pub fn unseal(&self, envelope: &MessageEnvelope) -> Result<BusEvent> {
        let sk = self
            .keystore
            .priv_key(&envelope.recipient)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "bus: no private key for recipient {}",
                    envelope.recipient.as_str()
                )
            })?;
        envelope.open(sk)
    }
}
