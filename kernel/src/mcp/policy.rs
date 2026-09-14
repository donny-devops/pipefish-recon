//! Tool ACL policy — loaded from `POLICY.md` §2 at boot.
//!
//! Deny by default: any `(agent, tool)` pair not marked allow in the ACL
//! table is refused.

use anyhow::{bail, Context, Result};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

const AGENTS: [&str; 5] = ["recon-a1", "recon-a2", "recon-a3", "recon-a4", "recon-a5"];

#[derive(Debug, Default, Clone)]
pub struct ToolPolicy {
    /// agent_id -> allowed tool names
    allow: HashMap<String, HashSet<String>>,
    hil_gated: HashSet<(String, String)>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PolicyMatrix {
    pub agents: Vec<String>,
    pub tools: Vec<String>,
    pub allow: Vec<(String, String)>,
    pub hil_gated: Vec<(String, String)>,
}

impl ToolPolicy {
    pub fn new() -> Self {
        Self::default()
    }

    /// Deny-by-default: only pairs explicitly inserted via [`allow`] are permitted.
    pub fn is_allowed(&self, agent_id: &str, tool_name: &str) -> bool {
        self.allow
            .get(agent_id)
            .map(|tools| tools.contains(tool_name))
            .unwrap_or(false)
    }

    pub fn allow(&mut self, agent_id: impl Into<String>, tool_name: impl Into<String>) {
        self.allow
            .entry(agent_id.into())
            .or_default()
            .insert(tool_name.into());
    }

    pub fn allow_hil(&mut self, agent_id: impl Into<String>, tool_name: impl Into<String>) {
        let agent = agent_id.into();
        let tool = tool_name.into();
        self.hil_gated.insert((agent.clone(), tool.clone()));
        self.allow(agent, tool);
    }

    pub fn allow_count(&self) -> usize {
        self.allow.values().map(|s| s.len()).sum()
    }

    pub fn matrix(&self) -> PolicyMatrix {
        let mut tools: HashSet<String> = HashSet::new();
        let mut allow_pairs = Vec::new();
        for (agent, set) in &self.allow {
            for tool in set {
                tools.insert(tool.clone());
                allow_pairs.push((agent.clone(), tool.clone()));
            }
        }
        allow_pairs.sort();
        let mut tool_list: Vec<String> = tools.into_iter().collect();
        tool_list.sort();
        let mut hil: Vec<(String, String)> = self.hil_gated.iter().cloned().collect();
        hil.sort();
        PolicyMatrix {
            agents: AGENTS.iter().map(|s| (*s).to_string()).collect(),
            tools: tool_list,
            allow: allow_pairs,
            hil_gated: hil,
        }
    }

    pub fn load() -> Result<Self> {
        Self::load_from_path(&resolve_policy_path())
    }

    pub fn load_from_path(path: &Path) -> Result<Self> {
        let markdown = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read POLICY.md at {}", path.display()))?;
        Self::from_markdown(&markdown)
    }

    pub fn from_markdown(markdown: &str) -> Result<Self> {
        let table = extract_acl_section(markdown)?;
        let policy = parse_acl_table(table)?;
        if policy.allow_count() == 0 {
            bail!("chore(core): policy denied — ACL table empty or unparsed");
        }
        Ok(policy)
    }
}

pub fn resolve_policy_path() -> PathBuf {
    if let Ok(p) = std::env::var("PIPEFISH_POLICY_PATH") {
        return PathBuf::from(p);
    }
    let cwd = PathBuf::from("POLICY.md");
    if cwd.is_file() {
        return cwd;
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("POLICY.md")
}

fn extract_acl_section(markdown: &str) -> Result<&str> {
    let start = markdown
        .find("## 2. ACL")
        .context("POLICY.md missing §2 ACL Matrix heading")?;
    let rest = &markdown[start..];
    let end = rest[3..].find("\n## ").map(|i| i + 3).unwrap_or(rest.len());
    Ok(&rest[..end])
}

fn parse_acl_table(section: &str) -> Result<ToolPolicy> {
    let mut policy = ToolPolicy::new();
    let mut saw_header = false;

    for line in section.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with('|') {
            continue;
        }
        if trimmed.contains("---") {
            continue;
        }
        let cols: Vec<&str> = trimmed.split('|').map(|c| c.trim()).collect();
        // split("| a | b |") => ["", "a", "b", ""]
        if cols.len() < 7 {
            continue;
        }
        let tool_cell = cols[1];
        let agent_cells = &cols[2..7];
        if tool_cell.eq_ignore_ascii_case("tool") {
            saw_header = true;
            continue;
        }
        if !saw_header {
            continue;
        }
        let tools = extract_backticked(tool_cell);
        if tools.is_empty() {
            continue;
        }
        for (idx, cell) in agent_cells.iter().enumerate() {
            if !cell.contains('✅') {
                continue;
            }
            let agent = AGENTS[idx];
            let hil = cell.contains("HIL");
            for tool in &tools {
                if hil {
                    policy.allow_hil(agent, tool);
                } else {
                    policy.allow(agent, tool);
                }
            }
        }
    }

    Ok(policy)
}

fn extract_backticked(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'`' {
            i += 1;
            let start = i;
            while i < bytes.len() && bytes[i] != b'`' {
                i += 1;
            }
            if i < bytes.len() {
                let name = &s[start..i];
                if !name.is_empty()
                    && name
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
                {
                    out.push(name.to_string());
                }
                i += 1;
            }
        } else {
            i += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policy_allows_registered_tools_only() {
        let mut policy = ToolPolicy::new();
        policy.allow("recon-a1", "cve-feed");
        assert!(policy.is_allowed("recon-a1", "cve-feed"));
        assert!(!policy.is_allowed("recon-a1", "nftables"));
        assert!(!policy.is_allowed("recon-a4", "cve-feed"));
    }

    #[test]
    fn empty_policy_denies_everything() {
        let policy = ToolPolicy::new();
        assert!(!policy.is_allowed("recon-a1", "cve-feed"));
    }

    #[test]
    fn loads_real_policy_md() {
        let policy = ToolPolicy::load().expect("POLICY.md must parse");
        assert!(policy.is_allowed("recon-a1", "cve-feed"));
        assert!(policy.is_allowed("recon-a1", "nvd-feed"));
        assert!(policy.is_allowed("recon-a1", "shodan"));
        assert!(!policy.is_allowed("recon-a1", "nftables"));
        assert!(!policy.is_allowed("recon-a2", "cve-feed"));
        assert!(policy.is_allowed("recon-a2", "llm"));
        assert!(policy.is_allowed("recon-a2", "mitre-attack"));
        assert!(!policy.is_allowed("recon-a3", "llm"));
        assert!(policy.is_allowed("recon-a4", "nftables"));
        assert!(policy.is_allowed("recon-a4", "waf-cloudflare"));
        assert!(policy.is_allowed("recon-a4", "github-actions"));
        assert!(!policy.is_allowed("recon-a4", "google-sheets"));
        assert!(policy.is_allowed("recon-a5", "google-sheets"));
        assert!(policy.is_allowed("recon-a5", "slh-dsa-sign"));
        assert!(!policy.is_allowed("recon-a5", "nftables"));
        assert!(policy.allow_count() > 10);
        let matrix = policy.matrix();
        assert!(matrix
            .hil_gated
            .iter()
            .any(|(a, t)| a == "recon-a4" && t == "nftables"));
    }

    #[test]
    fn empty_markdown_fails() {
        let err = ToolPolicy::from_markdown("# no acl\n").unwrap_err();
        assert!(err.to_string().contains("§2") || err.to_string().contains("ACL"));
    }
}
