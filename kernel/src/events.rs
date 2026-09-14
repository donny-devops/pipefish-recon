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
#[serde(rename_all = "lowercase")]
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
#[serde(rename_all = "lowercase")]
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum ContextId {
    Core,
    Bus,
    Llm,
    Tool,
    Dx,
    #[serde(rename = "recon-a1")]
    ReconA1,
    #[serde(rename = "recon-a2")]
    ReconA2,
    #[serde(rename = "recon-a3")]
    ReconA3,
    #[serde(rename = "recon-a4")]
    ReconA4,
    #[serde(rename = "recon-a5")]
    ReconA5,
}

impl ContextId {
    pub fn as_str(&self) -> &'static str {
        match self {
            ContextId::Core => "core",
            ContextId::Bus => "bus",
            ContextId::Llm => "llm",
            ContextId::Tool => "tool",
            ContextId::Dx => "dx",
            ContextId::ReconA1 => "recon-a1",
            ContextId::ReconA2 => "recon-a2",
            ContextId::ReconA3 => "recon-a3",
            ContextId::ReconA4 => "recon-a4",
            ContextId::ReconA5 => "recon-a5",
        }
    }

    pub fn is_agent(&self) -> bool {
        matches!(
            self,
            ContextId::ReconA1
                | ContextId::ReconA2
                | ContextId::ReconA3
                | ContextId::ReconA4
                | ContextId::ReconA5
        )
    }

    pub fn all_runtime() -> &'static [ContextId] {
        &[
            ContextId::Core,
            ContextId::Bus,
            ContextId::Llm,
            ContextId::Tool,
            ContextId::Dx,
            ContextId::ReconA1,
            ContextId::ReconA2,
            ContextId::ReconA3,
            ContextId::ReconA4,
            ContextId::ReconA5,
        ]
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
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
            "routed threat signal to RECON-A3",
        )
        .with_severity(Severity::High)
        .with_payload(serde_json::json!({"threat_id": "CVE-2026-1337"}));

        assert_eq!(
            evt.to_commit_string(),
            "feat(llm): routed threat signal to RECON-A3"
        );
        assert_eq!(evt.severity, Some(Severity::High));
        assert!(evt.payload.is_some());
    }

    #[test]
    fn commit_types_and_scopes() {
        assert_eq!(CommitType::Fix.as_str(), "fix");
        assert_eq!(CommitType::Chore.as_str(), "chore");
        assert_eq!(CommitType::Docs.as_str(), "docs");
        assert_eq!(CommitType::Refactor.as_str(), "refactor");
        assert_eq!(CommitType::Security.as_str(), "security");

        assert_eq!(ContextScope::Core.as_str(), "core");
        assert_eq!(ContextScope::Bus.as_str(), "bus");
        assert_eq!(ContextScope::Tool.as_str(), "tool");
        assert_eq!(ContextScope::Dx.as_str(), "dx");

        assert_eq!(ContextId::Core.as_str(), "core");
        assert_eq!(ContextId::Bus.as_str(), "bus");
        assert_eq!(ContextId::Tool.as_str(), "tool");
        assert_eq!(ContextId::Dx.as_str(), "dx");
        assert_eq!(ContextId::ReconA1.as_str(), "recon-a1");
        assert_eq!(ContextId::ReconA5.as_str(), "recon-a5");
        assert!(ContextId::ReconA3.is_agent());
        assert!(!ContextId::Core.is_agent());
    }
}
