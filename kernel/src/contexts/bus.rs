//! BusContext — the kernel's IPC broker.
//!
//! Receives [`BusEvent`]s from every other context over a single mpsc inbox,
//! logs each one as a Conventional Commit string, and fans them out to the
//! `dx` context's dedicated audit channel.

use anyhow::Result;
use tokio::sync::{mpsc, Mutex};
use tracing::{info, warn};

use crate::events::BusEvent;

const BUS_CHANNEL_CAPACITY: usize = 1024;

pub struct BusContext {
    rx: Mutex<mpsc::Receiver<BusEvent>>,
    dx_tx: mpsc::Sender<BusEvent>,
}

impl BusContext {
    /// Construct the bus. Returns the context plus the shared [`Sender`] that
    /// every other context clones to publish events.
    pub fn new(dx_tx: mpsc::Sender<BusEvent>) -> (Self, mpsc::Sender<BusEvent>) {
        let (tx, rx) = mpsc::channel(BUS_CHANNEL_CAPACITY);
        (
            Self {
                rx: Mutex::new(rx),
                dx_tx,
            },
            tx,
        )
    }

    pub async fn run(&self) -> Result<()> {
        info!("bus context online — IPC broker armed");
        let mut rx = self.rx.lock().await;
        while let Some(evt) = rx.recv().await {
            info!(
                event_id = %evt.id,
                source = evt.source.as_str(),
                "{}",
                evt.to_commit_string()
            );
            if let Err(e) = self.dx_tx.send(evt.clone()).await {
                warn!("bus: dx audit channel closed: {e}");
            }
        }
        info!("bus context: all senders dropped; shutting down");
        Ok(())
    }
}
