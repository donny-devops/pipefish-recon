//! Local JSON-lines audit log.
//!
//! Appends one serialized [`BusEvent`] per line to a file on disk. The file is
//! opened in append mode on every write so concurrent writers (multiple
//! contexts in the same process, or external tailers) interleave cleanly.

use std::path::PathBuf;

use anyhow::{Context, Result};
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;

use crate::events::BusEvent;

/// Appends [`BusEvent`]s as JSON lines to `audit_log.jsonl` (or any configured
/// path) in the working directory.
pub struct LocalAuditLog {
    path: PathBuf,
}

impl LocalAuditLog {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn path(&self) -> &std::path::Path {
        &self.path
    }

    /// Append one event as a single JSON line. Creates the file if missing.
    pub async fn append(&self, event: &BusEvent) -> Result<()> {
        let mut line = serde_json::to_vec(event).context("serializing BusEvent")?;
        line.push(b'\n');
        let mut f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .await
            .with_context(|| format!("opening audit log {:?}", self.path))?;
        f.write_all(&line)
            .await
            .with_context(|| format!("writing to audit log {:?}", self.path))?;
        f.flush().await.ok();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::{CommitType, ContextId, ContextScope};

    #[tokio::test]
    async fn append_writes_valid_json_line() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("audit_log.jsonl");
        let log = LocalAuditLog::new(&path);

        let evt = BusEvent::new(
            ContextId::Llm,
            CommitType::Feat,
            ContextScope::Llm,
            "routed signal",
        );
        log.append(&evt).await.unwrap();

        let contents = tokio::fs::read_to_string(&path).await.unwrap();
        assert!(contents.ends_with('\n'));
        let line = contents.trim_end_matches('\n');
        let parsed: BusEvent = serde_json::from_str(line).unwrap();
        assert_eq!(parsed.id, evt.id);
        assert_eq!(parsed.description, "routed signal");
    }

    #[tokio::test]
    async fn multiple_appends_produce_multiple_lines() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("audit_log.jsonl");
        let log = LocalAuditLog::new(&path);

        for i in 0..3 {
            let evt = BusEvent::new(
                ContextId::Dx,
                CommitType::Chore,
                ContextScope::Dx,
                format!("event {i}"),
            );
            log.append(&evt).await.unwrap();
        }

        let contents = tokio::fs::read_to_string(&path).await.unwrap();
        let lines: Vec<&str> = contents.lines().collect();
        assert_eq!(lines.len(), 3);
        for line in &lines {
            let _: BusEvent = serde_json::from_str(line).unwrap();
        }
    }
}
