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
    fn policy_allows_registered_tools() {
        let mut policy = ToolPolicy::new();
        policy.allow("recon-a1", "cve-feed");
        assert!(policy.is_allowed("recon-a1", "cve-feed"));
    }
}
