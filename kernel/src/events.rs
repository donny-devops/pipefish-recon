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

    // --- CommitType::as_str ---

    #[test]
    fn commit_type_as_str_all_variants() {
        assert_eq!(CommitType::Feat.as_str(), "feat");
        assert_eq!(CommitType::Fix.as_str(), "fix");
        assert_eq!(CommitType::Chore.as_str(), "chore");
        assert_eq!(CommitType::Docs.as_str(), "docs");
        assert_eq!(CommitType::Refactor.as_str(), "refactor");
        assert_eq!(CommitType::Security.as_str(), "security");
    }

    // --- ContextScope::as_str ---

    #[test]
    fn context_scope_as_str_all_variants() {
        assert_eq!(ContextScope::Core.as_str(), "core");
        assert_eq!(ContextScope::Bus.as_str(), "bus");
        assert_eq!(ContextScope::Llm.as_str(), "llm");
        assert_eq!(ContextScope::Tool.as_str(), "tool");
        assert_eq!(ContextScope::Dx.as_str(), "dx");
    }

    // --- ContextId::as_str ---

    #[test]
    fn context_id_as_str_all_variants() {
        assert_eq!(ContextId::Core.as_str(), "core");
        assert_eq!(ContextId::Bus.as_str(), "bus");
        assert_eq!(ContextId::Llm.as_str(), "llm");
        assert_eq!(ContextId::Tool.as_str(), "tool");
        assert_eq!(ContextId::Dx.as_str(), "dx");
    }

    // --- BusEvent::new ---

    #[test]
    fn bus_event_new_sets_fields_correctly() {
        let evt = BusEvent::new(
            ContextId::Core,
            CommitType::Fix,
            ContextScope::Bus,
            "patched route",
        );
        assert_eq!(evt.source, ContextId::Core);
        assert_eq!(evt.commit_type, CommitType::Fix);
        assert_eq!(evt.scope, ContextScope::Bus);
        assert_eq!(evt.description, "patched route");
        assert!(evt.severity.is_none());
        assert!(evt.payload.is_none());
    }

    #[test]
    fn bus_event_new_generates_unique_ids() {
        let a = BusEvent::new(ContextId::Bus, CommitType::Chore, ContextScope::Bus, "x");
        let b = BusEvent::new(ContextId::Bus, CommitType::Chore, ContextScope::Bus, "x");
        assert_ne!(a.id, b.id);
    }

    #[test]
    fn bus_event_new_timestamp_is_recent() {
        let before = Utc::now();
        let evt = BusEvent::new(ContextId::Bus, CommitType::Chore, ContextScope::Bus, "ts");
        let after = Utc::now();
        assert!(evt.timestamp >= before);
        assert!(evt.timestamp <= after);
    }

    // --- BusEvent::with_severity ---

    #[test]
    fn bus_event_with_severity_sets_severity() {
        let evt = BusEvent::new(ContextId::Core, CommitType::Security, ContextScope::Core, "threat")
            .with_severity(Severity::Critical);
        assert_eq!(evt.severity, Some(Severity::Critical));
    }

    #[test]
    fn bus_event_with_severity_all_variants() {
        for sev in [
            Severity::Critical,
            Severity::High,
            Severity::Medium,
            Severity::Low,
            Severity::Info,
        ] {
            let evt = BusEvent::new(ContextId::Dx, CommitType::Feat, ContextScope::Dx, "s")
                .with_severity(sev);
            assert_eq!(evt.severity, Some(sev));
        }
    }

    // --- BusEvent::with_payload ---

    #[test]
    fn bus_event_with_payload_sets_payload() {
        let data = serde_json::json!({"key": "value", "count": 42});
        let evt = BusEvent::new(ContextId::Tool, CommitType::Feat, ContextScope::Tool, "p")
            .with_payload(data.clone());
        assert_eq!(evt.payload, Some(data));
    }

    // --- to_commit_string covers all commit types and scopes ---

    #[test]
    fn to_commit_string_security_scope() {
        let evt = BusEvent::new(
            ContextId::Core,
            CommitType::Security,
            ContextScope::Core,
            "blocked malicious IP",
        );
        assert_eq!(evt.to_commit_string(), "security(core): blocked malicious IP");
    }

    #[test]
    fn to_commit_string_refactor_scope() {
        let evt = BusEvent::new(
            ContextId::Llm,
            CommitType::Refactor,
            ContextScope::Llm,
            "reorganized prompt chain",
        );
        assert_eq!(evt.to_commit_string(), "refactor(llm): reorganized prompt chain");
    }

    // --- Serde round-trip ---

    #[test]
    fn bus_event_serde_round_trip() {
        let original = BusEvent::new(
            ContextId::Tool,
            CommitType::Fix,
            ContextScope::Tool,
            "serialized event",
        )
        .with_severity(Severity::High)
        .with_payload(serde_json::json!({"foo": "bar"}));

        let json = serde_json::to_string(&original).expect("serialization failed");
        let restored: BusEvent = serde_json::from_str(&json).expect("deserialization failed");

        assert_eq!(restored.id, original.id);
        assert_eq!(restored.source, original.source);
        assert_eq!(restored.commit_type, original.commit_type);
        assert_eq!(restored.scope, original.scope);
        assert_eq!(restored.description, original.description);
        assert_eq!(restored.severity, original.severity);
        assert_eq!(restored.payload, original.payload);
    }
}
