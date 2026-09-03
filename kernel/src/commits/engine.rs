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
    use crate::events::{CommitType, ContextId, ContextScope};

    #[test]
    fn engine_format_event() {
        let engine = CommitEngine::new();
        let evt = BusEvent::new(
            ContextId::Dx,
            CommitType::Docs,
            ContextScope::Dx,
            "generated SBOM artifact",
        );
        assert_eq!(engine.format(&evt), "docs(dx): generated SBOM artifact");
    }
}
