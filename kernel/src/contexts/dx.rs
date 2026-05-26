//! DxContext — Conventional Commits engine and audit-log sink (stub).
//!
//! In production this batches commits into Google Sheets / Drive and emits
//! SBOM + AI-BOM artifacts. The skeleton appends each event to
//! `audit_log.jsonl` in the current working directory.

use anyhow::Result;
use std::path::PathBuf;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};
use tracing::{info, warn};

use crate::commits::engine::CommitEngine;
use crate::events::{BusEvent, CommitType, ContextId, ContextScope};

pub struct DxContext {
    bus_tx: mpsc::Sender<BusEvent>,
    audit_path: PathBuf,
    #[allow(dead_code)]
    engine: CommitEngine,
}

impl DxContext {
    pub fn new(bus_tx: mpsc::Sender<BusEvent>) -> Self {
        Self {
            bus_tx,
            audit_path: PathBuf::from("audit_log.jsonl"),
            engine: CommitEngine::new(),
        }
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
        info!("dx context online — audit sink at {:?}", self.audit_path);

        let boot = BusEvent::new(
            ContextId::Dx,
            CommitType::Chore,
            ContextScope::Dx,
            "commit engine online; audit sink armed",
        );
        if let Err(e) = self.append_audit(&boot).await {
            warn!("dx: failed to write boot audit row: {e}");
        }
        let _ = self.bus_tx.send(boot).await;

        let mut tick = interval(Duration::from_secs(120));
        loop {
            tick.tick().await;
            let flush = BusEvent::new(
                ContextId::Dx,
                CommitType::Chore,
                ContextScope::Dx,
                "audit flush tick (no upstream sync configured)",
            );
            if let Err(e) = self.append_audit(&flush).await {
                warn!("dx: failed to write audit row: {e}");
            }
            if self.bus_tx.send(flush).await.is_err() {
                break;
            }
        }
        Ok(())
    }
}
