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

    #[test]
    fn tool_registry_lifecycle() {
        let mut registry = ToolRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);

        registry.register(ToolEntry {
            name: "shodan".to_string(),
            transport: Transport::Stdio,
            acl_tags: vec!["osint".to_string()],
        });

        assert!(!registry.is_empty());
        assert_eq!(registry.len(), 1);
        let entry = registry.get("shodan").expect("entry exists");
        assert_eq!(entry.transport, Transport::Stdio);
        assert_eq!(entry.acl_tags, vec!["osint".to_string()]);
    }
}
