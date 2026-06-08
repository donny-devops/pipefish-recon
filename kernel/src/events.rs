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
    Agent,
}

impl ContextScope {
    pub fn as_str(&self) -> &'static str {
        match self {
            ContextScope::Core => "core",
            ContextScope::Bus => "bus",
            ContextScope::Llm => "llm",
            ContextScope::Tool => "tool",
            ContextScope::Dx => "dx",
            ContextScope::Agent => "agent",
        }
    }
}

/// Identifies the source of a [`BusEvent`]. Each agent gets its own variant
/// so the audit log correctly attributes every emitted event.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ContextId {
    Core,
    Bus,
    Llm,
    Tool,
    Dx,
    A1,
    A2,
    A3,
    A4,
    A5,
}

impl ContextId {
    pub fn as_str(&self) -> &'static str {
        match self {
            ContextId::Core => "core",
            ContextId::Bus => "bus",
            ContextId::Llm => "llm",
            ContextId::Tool => "tool",
            ContextId::Dx => "dx",
            ContextId::A1 => "skynet-a1",
            ContextId::A2 => "skynet-a2",
            ContextId::A3 => "skynet-a3",
            ContextId::A4 => "skynet-a4",
            ContextId::A5 => "skynet-a5",
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

    #[test]
    fn agent_context_ids_have_correct_str() {
        assert_eq!(ContextId::A1.as_str(), "skynet-a1");
        assert_eq!(ContextId::A5.as_str(), "skynet-a5");
    }
}
