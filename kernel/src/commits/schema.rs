//! Commit schema — re-exports [`BusEvent`] as the canonical commit row type
//! and defines an in-memory `CommitLog` aggregate.

pub use crate::events::BusEvent;

#[derive(Debug, Default)]
pub struct CommitLog {
    pub entries: Vec<BusEvent>,
}

impl CommitLog {
    pub fn new() -> Self {
        Self::default()
    }

    #[allow(dead_code)]
    pub fn push(&mut self, evt: BusEvent) {
        self.entries.push(evt);
    }

    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::{CommitType, ContextId, ContextScope};

    #[test]
    fn commit_log_operations() {
        let mut log = CommitLog::new();
        assert!(log.is_empty());
        assert_eq!(log.len(), 0);

        let evt = BusEvent::new(
            ContextId::Core,
            CommitType::Feat,
            ContextScope::Core,
            "policy verified",
        );
        log.push(evt);
        assert!(!log.is_empty());
        assert_eq!(log.len(), 1);
    }
}
