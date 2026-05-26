//! MCP server registry — tracks every registered tool server and its
//! transport. In a fuller build entries are populated from a manifest
//! consulted at boot.

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Transport {
    Stdio,
    Sse,
}

#[derive(Debug, Clone)]
pub struct ToolEntry {
    pub name: String,
    pub transport: Transport,
    pub acl_tags: Vec<String>,
}

#[derive(Debug, Default)]
pub struct ToolRegistry {
    entries: HashMap<String, ToolEntry>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    #[allow(dead_code)]
    pub fn register(&mut self, entry: ToolEntry) {
        self.entries.insert(entry.name.clone(), entry);
    }

    #[allow(dead_code)]
    pub fn get(&self, name: &str) -> Option<&ToolEntry> {
        self.entries.get(name)
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

    fn make_entry(name: &str) -> ToolEntry {
        ToolEntry {
            name: name.to_string(),
            transport: Transport::Stdio,
            acl_tags: vec!["default".to_string()],
        }
    }

    #[test]
    fn new_registry_is_empty() {
        let reg = ToolRegistry::new();
        assert!(reg.is_empty());
        assert_eq!(reg.len(), 0);
    }

    #[test]
    fn default_produces_empty_registry() {
        let reg = ToolRegistry::default();
        assert!(reg.is_empty());
    }

    #[test]
    fn register_adds_entry() {
        let mut reg = ToolRegistry::new();
        reg.register(make_entry("grep"));
        assert_eq!(reg.len(), 1);
        assert!(!reg.is_empty());
    }

    #[test]
    fn get_returns_registered_entry() {
        let mut reg = ToolRegistry::new();
        reg.register(make_entry("bash"));
        let entry = reg.get("bash").expect("entry should exist");
        assert_eq!(entry.name, "bash");
    }

    #[test]
    fn get_unknown_returns_none() {
        let reg = ToolRegistry::new();
        assert!(reg.get("nonexistent").is_none());
    }

    #[test]
    fn register_multiple_entries() {
        let mut reg = ToolRegistry::new();
        reg.register(make_entry("tool_a"));
        reg.register(make_entry("tool_b"));
        reg.register(make_entry("tool_c"));
        assert_eq!(reg.len(), 3);
    }

    #[test]
    fn register_overwrites_existing_entry() {
        let mut reg = ToolRegistry::new();
        reg.register(ToolEntry {
            name: "my_tool".to_string(),
            transport: Transport::Stdio,
            acl_tags: vec![],
        });
        reg.register(ToolEntry {
            name: "my_tool".to_string(),
            transport: Transport::Sse,
            acl_tags: vec!["privileged".to_string()],
        });
        assert_eq!(reg.len(), 1);
        let entry = reg.get("my_tool").unwrap();
        assert_eq!(entry.transport, Transport::Sse);
    }

    #[test]
    fn transport_variants_are_comparable() {
        assert_eq!(Transport::Stdio, Transport::Stdio);
        assert_eq!(Transport::Sse, Transport::Sse);
        assert_ne!(Transport::Stdio, Transport::Sse);
    }
}
