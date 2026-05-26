//! CommitEngine — turns [`BusEvent`]s into Conventional Commit strings and
//! (in a fuller build) appends them to the local audit branch via libgit2.

use crate::events::BusEvent;

#[derive(Debug, Default)]
pub struct CommitEngine;

impl CommitEngine {
    pub fn new() -> Self {
        Self
    }

    #[allow(dead_code)]
    pub fn format(&self, evt: &BusEvent) -> String {
        evt.to_commit_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::{BusEvent, CommitType, ContextId, ContextScope};

    #[test]
    fn format_delegates_to_commit_string() {
        let engine = CommitEngine::new();
        let evt = BusEvent::new(
            ContextId::Dx,
            CommitType::Docs,
            ContextScope::Dx,
            "updated audit schema",
        );
        assert_eq!(engine.format(&evt), evt.to_commit_string());
    }

    #[test]
    fn default_produces_engine() {
        let _engine: CommitEngine = CommitEngine::default();
    }

    #[test]
    fn format_all_commit_types() {
        let engine = CommitEngine::new();
        let cases = [
            (CommitType::Feat, "feat"),
            (CommitType::Fix, "fix"),
            (CommitType::Chore, "chore"),
            (CommitType::Docs, "docs"),
            (CommitType::Refactor, "refactor"),
            (CommitType::Security, "security"),
        ];
        for (ct, prefix) in cases {
            let evt = BusEvent::new(ContextId::Core, ct, ContextScope::Core, "desc");
            let formatted = engine.format(&evt);
            assert!(
                formatted.starts_with(prefix),
                "expected prefix {prefix}, got {formatted}"
            );
        }
    }
}
