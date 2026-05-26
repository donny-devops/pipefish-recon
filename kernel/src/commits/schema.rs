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
