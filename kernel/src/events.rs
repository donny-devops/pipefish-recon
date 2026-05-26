//! Cross-context event types — the lingua franca of the Agentic OS bus.
//!
//! Every action emitted by any context becomes a `BusEvent`, which the `dx`
//! context formats as a Conventional Commit string and appends to the audit
//! log. The schema mirrors the Conventional Commits 1.0 vocabulary.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusEvent {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub source: ContextId,
    pub commit_type: CommitType,
    pub scope: ContextScope,
    pub description: String,
    pub severity: Option<Severity>,
    pub payload: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum CommitType {
    Feat,
    Fix,
    Chore,
    Docs,
    Refactor,
    Security,
}

impl CommitType {
    pub fn as_str(&self) -> &'static str {
        match self {
            CommitType::Feat => "feat",
            CommitType::Fix => "fix",
            CommitType::Chore => "chore",
            CommitType::Docs => "docs",
            CommitType::Refactor => "refactor",
            CommitType::Security => "security",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ContextScope {
    Core,
    Bus,
    Llm,
    Tool,
    Dx,
}

impl ContextScope {
    pub fn as_str(&self) -> &'static str {
        match self {
            ContextScope::Core => "core",
            ContextScope::Bus => "bus",
            ContextScope::Llm => "llm",
            ContextScope::Tool => "tool",
            ContextScope::Dx => "dx",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ContextId {
    Core,
    Bus,
    Llm,
    Tool,
    Dx,
}

impl ContextId {
    pub fn as_str(&self) -> &'static str {
        match self {
            ContextId::Core => "core",
            ContextId::Bus => "bus",
            ContextId::Llm => "llm",
            ContextId::Tool => "tool",
            ContextId::Dx => "dx",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

impl BusEvent {
    pub fn new(
        source: ContextId,
        commit_type: CommitType,
        scope: ContextScope,
        description: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            source,
            commit_type,
            scope,
            description: description.into(),
            severity: None,
            payload: None,
        }
    }

    pub fn with_severity(mut self, severity: Severity) -> Self {
        self.severity = Some(severity);
        self
    }

    pub fn with_payload(mut self, payload: serde_json::Value) -> Self {
        self.payload = Some(payload);
        self
    }

    pub fn to_commit_string(&self) -> String {
        format!(
            "{}({}): {}",
            self.commit_type.as_str(),
            self.scope.as_str(),
            self.description
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commit_string_matches_conventional_commits_format() {
        let evt = BusEvent::new(
            ContextId::Llm,
            CommitType::Feat,
            ContextScope::Llm,
            "routed threat signal to SKYNET-A3",
        );
        assert_eq!(
            evt.to_commit_string(),
            "feat(llm): routed threat signal to SKYNET-A3"
        );
    }
}
