//! DxContext — Conventional Commits engine and audit-log sink.
//!
//! Subscribes to every bus event (fan-out from [`super::bus::BusContext`]),
//! appends JSONL rows to `audit_log.jsonl`, keeps an in-memory ring for the
//! operator API, and serves that API on localhost.

use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tokio::sync::{mpsc, Mutex};
use tokio::time::{interval, Duration};
use tracing::{info, warn};

use crate::commits::engine::CommitEngine;
use crate::events::{BusEvent, CommitType, ContextId, ContextScope};
use crate::mcp::policy::ToolPolicy;
use crate::operator::{self, OperatorState, DEFAULT_BIND, RING_CAPACITY};

pub struct DxContext {
    bus_tx: mpsc::Sender<BusEvent>,
    bus_rx: Mutex<mpsc::Receiver<BusEvent>>,
    audit_path: PathBuf,
    #[allow(dead_code)]
    engine: CommitEngine,
    state: OperatorState,
    bind: String,
}

impl DxContext {
    pub fn new(
        bus_tx: mpsc::Sender<BusEvent>,
        bus_rx: mpsc::Receiver<BusEvent>,
        policy: Arc<ToolPolicy>,
        bind: impl Into<String>,
    ) -> Self {
        Self {
            bus_tx,
            bus_rx: Mutex::new(bus_rx),
            audit_path: PathBuf::from("audit_log.jsonl"),
            engine: CommitEngine::new(),
            state: OperatorState::new(policy, RING_CAPACITY),
            bind: bind.into(),
        }
    }

    pub fn with_audit_path(mut self, path: PathBuf) -> Self {
        self.audit_path = path;
        self
    }

    pub fn state(&self) -> OperatorState {
        self.state.clone()
    }

    async fn append_audit(&self, evt: &BusEvent) -> Result<()> {
        let mut line = serde_json::to_vec(evt)?;
        line.push(b'\n');
        let mut f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.audit_path)
            .await?;
        f.write_all(&line).await?;
        Ok(())
    }

    pub async fn run(&self) -> Result<()> {
        info!(
            path = ?self.audit_path,
            bind = %self.bind,
            "dx context online — audit sink + operator HTTP"
        );

        let boot = BusEvent::new(
            ContextId::Dx,
            CommitType::Chore,
            ContextScope::Dx,
            "commit engine online; audit sink armed",
        );
        let _ = self.bus_tx.send(boot).await;

        let audit = self.audit_loop();
        let http = operator::serve(&self.bind, self.state.clone());

        tokio::select! {
            result = audit => result,
            result = http => result,
        }
    }

    async fn audit_loop(&self) -> Result<()> {
        let mut rx = self.bus_rx.lock().await;
        let mut tick = interval(Duration::from_secs(120));
        tick.tick().await;
        loop {
            tokio::select! {
                evt = rx.recv() => {
                    match evt {
                        Some(evt) => {
                            if let Err(e) = self.append_audit(&evt).await {
                                warn!("dx: failed to write audit row: {e}");
                            }
                            self.state.push(evt).await;
                        }
                        None => {
                            info!("dx: bus fan-out closed");
                            break;
                        }
                    }
                }
                _ = tick.tick() => {
                    let flush = BusEvent::new(
                        ContextId::Dx,
                        CommitType::Chore,
                        ContextScope::Dx,
                        "audit flush tick",
                    );
                    if self.bus_tx.send(flush).await.is_err() {
                        break;
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::bus::BusContext;
    use crate::events::CommitType;

    #[tokio::test]
    async fn dx_writes_fanned_out_bus_events_to_jsonl() {
        let path =
            std::env::temp_dir().join(format!("pipefish-audit-{}.jsonl", uuid::Uuid::new_v4()));
        let _ = std::fs::remove_file(&path);

        let (bus, tx, dx_rx) = BusContext::new();
        let policy = Arc::new(ToolPolicy::new());
        let dx =
            DxContext::new(tx.clone(), dx_rx, policy, "127.0.0.1:0").with_audit_path(path.clone());

        let bus_handle = tokio::spawn(async move { bus.run().await });
        let dx_handle = tokio::spawn(async move { dx.audit_loop().await });

        let evt = BusEvent::new(
            ContextId::ReconA1,
            CommitType::Feat,
            ContextScope::Bus,
            "RECON-A1 online (signal ingestion)",
        );
        tx.send(evt).await.expect("send");

        // Give dx a moment to write, then drop producer so loops exit.
        tokio::time::sleep(Duration::from_millis(50)).await;
        drop(tx);
        let _ = tokio::time::timeout(Duration::from_secs(2), bus_handle).await;
        let _ = tokio::time::timeout(Duration::from_secs(2), dx_handle).await;

        let body = std::fs::read_to_string(&path).expect("audit file");
        assert!(
            body.contains("RECON-A1 online (signal ingestion)"),
            "audit log missing fanned-out event: {body}"
        );
        let _ = std::fs::remove_file(&path);
    }
}
