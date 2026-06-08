//! BusContext — the kernel's IPC broker.
//!
//! Receives [`BusEvent`]s from every other context over a single mpsc inbox,
//! logs each one as a Conventional Commit string, and (in a fuller build)
//! fans them out to subscribers. For the skeleton we just sink and log.
use anyhow::Result;
use tokio::sync::mpsc;
use tracing::info;
use crate::events::BusEvent;

const BUS_CHANNEL_CAPACITY: usize = 1_024;

pub struct BusContext {
    // Owned directly — no Mutex needed; BusContext is single-owner.
    rx: mpsc::Receiver<BusEvent>,
}

impl BusContext {
    pub fn new() -> (Self, mpsc::Sender<BusEvent>) {
        let (tx, rx) = mpsc::channel(BUS_CHANNEL_CAPACITY);
        (Self { rx }, tx)
    }

    /// Drive the bus until all senders are dropped.
    /// Takes `&mut self` because `mpsc::Receiver` requires exclusive access.
    pub async fn run(&mut self) -> Result<()> {
        info!("bus context online — IPC broker armed");
        while let Some(evt) = self.rx.recv().await {
            info!(
                event_id = %evt.id,
                source = evt.source.as_str(),
                "{}",
                evt.to_commit_string()
            );
        }
        info!("bus context: all senders dropped; shutting down");
        Ok(())
    }
}
