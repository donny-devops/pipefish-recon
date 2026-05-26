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
    use crate::events::{BusEvent, CommitType, ContextId, ContextScope};

    fn make_event(desc: &str) -> BusEvent {
        BusEvent::new(ContextId::Core, CommitType::Chore, ContextScope::Core, desc)
    }

    #[test]
    fn new_log_is_empty() {
        let log = CommitLog::new();
        assert!(log.is_empty());
        assert_eq!(log.len(), 0);
    }

    #[test]
    fn push_increases_length() {
        let mut log = CommitLog::new();
        log.push(make_event("first"));
        assert_eq!(log.len(), 1);
        assert!(!log.is_empty());
    }

    #[test]
    fn push_multiple_entries() {
        let mut log = CommitLog::new();
        log.push(make_event("a"));
        log.push(make_event("b"));
        log.push(make_event("c"));
        assert_eq!(log.len(), 3);
    }

    #[test]
    fn entries_are_accessible() {
        let mut log = CommitLog::new();
        let evt = make_event("hello");
        let desc = evt.description.clone();
        log.push(evt);
        assert_eq!(log.entries[0].description, desc);
    }

    #[test]
    fn default_produces_empty_log() {
        let log = CommitLog::default();
        assert!(log.is_empty());
    }
}
