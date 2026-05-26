//! Tool ACL policy — loaded from `tool_policy.toml` in production. The stub
//! permits all (agent, tool) pairs but exposes the same surface so callers
//! can be written against it now.

use std::collections::HashMap;

#[derive(Debug, Default, Clone)]
pub struct ToolPolicy {
    allow: HashMap<String, Vec<String>>,
}

impl ToolPolicy {
    pub fn new() -> Self {
        Self::default()
    }

    /// Stub: every agent is allowed every tool until a real policy is loaded.
    #[allow(dead_code)]
    pub fn is_allowed(&self, _agent_id: &str, _tool_name: &str) -> bool {
        true
    }

    #[allow(dead_code)]
    pub fn allow(&mut self, agent_id: impl Into<String>, tool_name: impl Into<String>) {
        self.allow
            .entry(agent_id.into())
            .or_default()
            .push(tool_name.into());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_policy_is_allowed_for_any_pair() {
        let policy = ToolPolicy::new();
        assert!(policy.is_allowed("agent-x", "tool-y"));
    }

    #[test]
    fn default_policy_is_allowed_for_any_pair() {
        let policy = ToolPolicy::default();
        assert!(policy.is_allowed("agent-alpha", "bash"));
    }

    #[test]
    fn is_allowed_returns_true_after_allow_entry() {
        let mut policy = ToolPolicy::new();
        policy.allow("agent-1", "grep");
        // stub always returns true regardless of recorded allows
        assert!(policy.is_allowed("agent-1", "grep"));
    }

    #[test]
    fn is_allowed_returns_true_for_unregistered_pair() {
        let mut policy = ToolPolicy::new();
        policy.allow("agent-1", "grep");
        // stub permits any pair, even ones not explicitly allowed
        assert!(policy.is_allowed("agent-2", "unknown_tool"));
    }

    #[test]
    fn allow_records_multiple_tools_per_agent() {
        let mut policy = ToolPolicy::new();
        policy.allow("a1", "grep");
        policy.allow("a1", "bash");
        policy.allow("a2", "diff");
        // No panic; stub always allows
        assert!(policy.is_allowed("a1", "grep"));
        assert!(policy.is_allowed("a2", "diff"));
    }
}
