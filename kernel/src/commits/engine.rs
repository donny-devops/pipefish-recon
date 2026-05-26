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
