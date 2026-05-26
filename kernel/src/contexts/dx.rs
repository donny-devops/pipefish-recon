//! DxContext — Conventional Commits engine and audit-log sink.
//!
//! Consumes every [`BusEvent`] forwarded by [`super::bus::BusContext`] and
//! mirrors it to two sinks:
//!
//! 1. **Local JSONL** — `audit_log.jsonl` in the working directory.
//! 2. **Google Sheets** — an `AuditLog` tab on the spreadsheet identified by
//!    `SKYNET_SHEETS_ID`, authenticated via a service-account JSON key at
//!    `GOOGLE_SERVICE_ACCOUNT_KEY`. Disabled silently when either env var is
//!    absent.
//!
//! After mirroring each event, dx emits its own `chore(dx): audited <id> →
//! <sinks>` event so the audit trail is visible in the bus itself.

use anyhow::Result;
use std::path::PathBuf;
use tokio::sync::mpsc;
use tracing::{info, warn};

use crate::audit::{LocalAuditLog, SheetsClient};
use crate::commits::engine::CommitEngine;
use crate::events::{BusEvent, CommitType, ContextId, ContextScope};

pub struct DxContext {
    bus_rx: mpsc::Receiver<BusEvent>,
    bus_tx: mpsc::Sender<BusEvent>,
    local_log: LocalAuditLog,
    sheets: Option<SheetsClient>,
    #[allow(dead_code)]
    engine: CommitEngine,
}

impl DxContext {
    pub fn new(
        bus_rx: mpsc::Receiver<BusEvent>,
        bus_tx: mpsc::Sender<BusEvent>,
    ) -> Result<Self> {
        let audit_path = PathBuf::from("audit_log.jsonl");
        let local_log = LocalAuditLog::new(audit_path);
        let sheets = match SheetsClient::from_env() {
            Ok(s) => s,
            Err(e) => {
                warn!("dx: SheetsClient init failed, continuing without Sheets sync: {e:#}");
                None
            }
        };
        Ok(Self {
            bus_rx,
            bus_tx,
            local_log,
            sheets,
            engine: CommitEngine::new(),
        })
    }

    /// Mirror one event to every configured sink. Errors are logged and
    /// swallowed — the dx loop must never abort because Sheets is flaky.
    async fn mirror(&self, evt: &BusEvent) -> Vec<&'static str> {
        info!(
            event_id = %evt.id,
            source = evt.source.as_str(),
            "{}",
            evt.to_commit_string()
        );

        let mut sinks: Vec<&'static str> = Vec::with_capacity(2);
        match self.local_log.append(evt).await {
            Ok(()) => sinks.push("local"),
            Err(e) => warn!(event_id = %evt.id, "dx: local audit append failed: {e:#}"),
        }
        if let Some(sheets) = &self.sheets {
            match sheets.append_row(evt).await {
                Ok(()) => sinks.push("sheets"),
                Err(e) => warn!(event_id = %evt.id, "dx: Sheets append failed: {e:#}"),
            }
        }
        sinks
    }

    pub async fn run(&mut self) -> Result<()> {
        info!(
            "dx context online — audit sink at {:?} (sheets={})",
            self.local_log.path(),
            self.sheets.is_some()
        );

        let boot = BusEvent::new(
            ContextId::Dx,
            CommitType::Chore,
            ContextScope::Dx,
            "commit engine online; audit sink armed",
        );
        if self.bus_tx.send(boot).await.is_err() {
            warn!("dx: bus channel closed during boot announce");
            return Ok(());
        }

        while let Some(evt) = self.bus_rx.recv().await {
            if matches!(evt.source, ContextId::Dx) && evt.description.starts_with("audited ") {
                continue;
            }

            let sinks = self.mirror(&evt).await;
            let sinks_str = if sinks.is_empty() {
                "none".to_string()
            } else {
                sinks.join("+")
            };
            let confirm = BusEvent::new(
                ContextId::Dx,
                CommitType::Chore,
                ContextScope::Dx,
                format!("audited {} → {}", evt.id, sinks_str),
            );
            let _ = self.local_log.append(&confirm).await;
            if self.bus_tx.send(confirm).await.is_err() {
                warn!("dx: bus channel closed; dx context shutting down");
                break;
            }
        }

        info!("dx context: audit stream closed; shutting down");
        Ok(())
    }
}
