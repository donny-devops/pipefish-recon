export const AGENT_ROLES: Record<string, { name: string; mission: string; scope: string }> = {
  "recon-a1": {
    name: "RECON-A1",
    mission: "Signal ingestion and normalization (CVE/NVD, OSINT, SIEM, honeypots).",
    scope: "bus",
  },
  "recon-a2": {
    name: "RECON-A2",
    mission: "Threat intelligence synthesis — MITRE ATT&CK, TTP correlation.",
    scope: "llm",
  },
  "recon-a3": {
    name: "RECON-A3",
    mission: "Deterministic decision and routing hub (no LLM on the hot path).",
    scope: "core",
  },
  "recon-a4": {
    name: "RECON-A4",
    mission: "Autonomous defense engine — nftables, ACL, WAF, git PRs (policy gated).",
    scope: "tool",
  },
  "recon-a5": {
    name: "RECON-A5",
    mission: "Governance and audit governor — signed reports, AI-BOM, Sheets/Drive.",
    scope: "dx",
  },
};
