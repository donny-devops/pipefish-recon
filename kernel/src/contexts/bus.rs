//! BusContext — the kernel's IPC broker.
//!
//! Receives [`BusEvent`]s from every other context over a single mpsc inbox,
//! logs each one as a Conventional Commit string, and fans a clone out to the
//! dx audit subscriber.

use anyhow::Result;
use tokio::sync::{mpsc, Mutex};
use tracing::{info, warn};

use crate::events::{BusEvent, CommitType, ContextId, ContextScope};

const BUS_CHANNEL_CAPACITY: usize = 1024;

pub struct BusContext {
    rx: Mutex<mpsc::Receiver<BusEvent>>,
    dx_tx: mpsc::Sender<BusEvent>,
}

impl BusContext {
    /// Returns `(bus, producer_tx, dx_subscriber_rx)`.
    pub fn new() -> (Self, mpsc::Sender<BusEvent>, mpsc::Receiver<BusEvent>) {
        let (tx, rx) = mpsc::channel(BUS_CHANNEL_CAPACITY);
        let (dx_tx, dx_rx) = mpsc::channel(BUS_CHANNEL_CAPACITY);
        (
            Self {
                rx: Mutex::new(rx),
                dx_tx,
            },
            tx,
            dx_rx,
        )
    }

    pub async fn run(&self) -> Result<()> {
        info!("bus context online — IPC broker armed");
        let boot = BusEvent::new(
            ContextId::Bus,
            CommitType::Chore,
            ContextScope::Bus,
            "IPC broker armed",
        );
        info!(
            event_id = %boot.id,
            source = boot.source.as_str(),
            "{}",
            boot.to_commit_string()
        );
        if self.dx_tx.send(boot).await.is_err() {
            warn!("dx subscriber dropped before drain");
            return Ok(());
        }
        let mut rx = self.rx.lock().await;
        while let Some(evt) = rx.recv().await {
            info!(
                event_id = %evt.id,
                source = evt.source.as_str(),
                "{}",
                evt.to_commit_string()
            );
            if self.dx_tx.send(evt).await.is_err() {
                warn!("dx subscriber dropped; bus exiting");
                break;
            }
        }
        info!("bus context: all senders dropped; shutting down");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::{CommitType, ContextId, ContextScope};

    #[tokio::test]
    async fn bus_fans_out_to_dx_subscriber() {
        let (bus, tx, mut dx_rx) = BusContext::new();
        let handle = tokio::spawn(async move { bus.run().await });
        let evt = BusEvent::new(
            ContextId::Core,
            CommitType::Chore,
            ContextScope::Core,
            "healthcheck ok",
        );
        tx.send(evt).await.expect("send");
        drop(tx);
        let boot = dx_rx.recv().await.expect("bus boot event");
        assert_eq!(boot.source, ContextId::Bus);
        let got = dx_rx.recv().await.expect("fan-out event");
        assert_eq!(got.description, "healthcheck ok");
        assert_eq!(got.source, ContextId::Core);
        handle.await.expect("bus join").expect("bus run");
    }
}
