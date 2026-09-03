# RECON - Mission

RECON is a **living threat intelligence organism** - a multi-agent platform
whose nervous system is the **Agentic OS kernel**. The kernel treats AI
agents as first-class OS citizens, the way Unix treats processes. Agents
observe, reason, decide, and act under a constitutional policy boundary that
is itself cryptographically signed.

## Principles

1. **Security-first at every layer.** Memory safety in the kernel (Rust),
   microVM isolation for every external tool, and policy-gated execution
   for every privileged action. No "trusted internal network" - every
   inter-agent message is authenticated and encrypted.

2. **Post-quantum by default.** All long-lived key material, audit
   signatures, and inter-agent envelopes use the NIST-finalized post-
   quantum standards published August 13, 2024:
   - **ML-KEM-768** (FIPS 203) for key encapsulation
   - **SLH-DSA-SHA2-128s** (FIPS 205) for stateless hash-based signatures

   Harvest-now-decrypt-later is treated as already in progress.

3. **Every action is a commit.** Every agentic decision is emitted as a
   Conventional Commit event. The same primitive engineers already use for
   code becomes the universal substrate for autonomous action.

## The Five Agents

| Agent | Role |
|---|---|
| **RECON-A1** | Signal Ingestion & Normalization |
| **RECON-A2** | Threat Intelligence Synthesis (MITRE ATT&CK, TTP correlation) |
| **RECON-A3** | Decision & Routing Hub (severity-gated, HIL escalation) |
| **RECON-A4** | Autonomous Defense Engine (nftables, ACL, WAF, GH Actions) |
| **RECON-A5** | Governance & Audit Governor (FIPS, AI-BOM, signed reports) |

## Further reading

- Architecture blueprint: `../recon-agentic-os-blueprint.md`
- Agent specifications: [`SOUL.md`](./SOUL.md)
- Runtime constitution: [`POLICY.md`](./POLICY.md)
