//! BusContext — the kernel's IPC broker.
//!
//! Receives [`BusEvent`]s from every other context over a single mpsc inbox,
//! logs each one as a Conventional Commit string, and fans a clone out to
//! every subscriber (dx audit sink, RECON-A3 router, …).

use anyhow::Result;
use tokio::sync::{mpsc, Mutex};
use tracing::{info, warn};

use crate::events::{BusEvent, CommitType, ContextId, ContextScope};

const BUS_CHANNEL_CAPACITY: usize = 1024;

pub struct BusContext {
    rx: Mutex<mpsc::Receiver<BusEvent>>,
    subscribers: std::sync::Mutex<Vec<mpsc::Sender<BusEvent>>>,
}

impl BusContext {
    /// Returns `(bus, producer_tx)`. Call [`subscribe`] before [`run`].
    pub fn new() -> (Self, mpsc::Sender<BusEvent>) {
        let (tx, rx) = mpsc::channel(BUS_CHANNEL_CAPACITY);
        (
            Self {
                rx: Mutex::new(rx),
                subscribers: std::sync::Mutex::new(Vec::new()),
            },
            tx,
        )
    }

    pub fn subscribe(&self) -> mpsc::Receiver<BusEvent> {
        let (tx, rx) = mpsc::channel(BUS_CHANNEL_CAPACITY);
        self.subscribers
            .lock()
            .expect("bus subscriber lock")
            .push(tx);
        rx
    }

    async fn fan_out(&self, evt: BusEvent) -> bool {
        let senders: Vec<mpsc::Sender<BusEvent>> = self
            .subscribers
            .lock()
            .expect("bus subscriber lock")
            .clone();
        if senders.is_empty() {
            return true;
        }
        let mut live = false;
        for tx in senders {
            if tx.send(evt.clone()).await.is_ok() {
                live = true;
            }
        }
        live
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
        if !self.fan_out(boot).await {
            warn!("all bus subscribers dropped before drain");
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
            if !self.fan_out(evt).await {
                warn!("all bus subscribers dropped; bus exiting");
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

    #[tokio::test]
    async fn bus_fans_out_to_all_subscribers() {
        let (bus, tx) = BusContext::new();
        let mut dx_rx = bus.subscribe();
        let mut a3_rx = bus.subscribe();
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
        assert_eq!(a3_rx.recv().await.expect("a3 boot").source, ContextId::Bus);
        let got = dx_rx.recv().await.expect("fan-out event");
        assert_eq!(got.description, "healthcheck ok");
        assert_eq!(
            a3_rx.recv().await.expect("a3 fan-out").description,
            "healthcheck ok"
        );
        handle.await.expect("bus join").expect("bus run");
    }
}
