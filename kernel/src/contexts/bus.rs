//! BusContext — the kernel's IPC broker.
//!
//! Receives [`BusEvent`]s from every other context over a single mpsc inbox,
//! logs each one as a Conventional Commit string, and (in a fuller build)
//! fans them out to subscribers. For the skeleton we just sink and log.

use anyhow::Result;
use tokio::sync::{mpsc, Mutex};
use tracing::info;

use crate::events::BusEvent;

const BUS_CHANNEL_CAPACITY: usize = 1024;

pub struct BusContext {
    rx: Mutex<mpsc::Receiver<BusEvent>>,
}

impl BusContext {
    pub fn new() -> (Self, mpsc::Sender<BusEvent>) {
        let (tx, rx) = mpsc::channel(BUS_CHANNEL_CAPACITY);
        (
            Self {
                rx: Mutex::new(rx),
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
        }
        info!("bus context: all senders dropped; shutting down");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::{BusEvent, CommitType, ContextId, ContextScope};

    #[tokio::test]
    async fn bus_drains_all_sent_events_and_terminates() {
        let (bus, tx) = BusContext::new();

        // Send three events then drop the sender so the bus loop exits
        let events: Vec<BusEvent> = (0..3)
            .map(|i| {
                BusEvent::new(
                    ContextId::Core,
                    CommitType::Chore,
                    ContextScope::Core,
                    format!("event {i}"),
                )
            })
            .collect();

        for evt in events {
            tx.send(evt).await.expect("send should succeed");
        }
        // Drop tx so the receiver loop ends
        drop(tx);

        // BusContext::run should terminate because the sender side is gone
        let result = bus.run().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn bus_terminates_immediately_when_no_events_sent() {
        let (bus, tx) = BusContext::new();
        drop(tx);
        assert!(bus.run().await.is_ok());
    }
}
