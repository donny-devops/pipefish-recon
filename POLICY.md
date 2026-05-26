# POLICY — Runtime Constitution

> **Signed by SLH-DSA at boot (stub).** In a future build the `core`
> context will verify a detached `POLICY.md.sig` (SLH-DSA-SHA2-128s, FIPS
> 205) before installing the enforcement hook in front of every
> cross-context dispatch.

This document is the authoritative source of truth for what SKYNET agents
may and may not do at runtime. The kernel's `core` context loads it at
boot, verifies its signature, and rejects any cross-context dispatch that
violates it. Violations are emitted as `chore(core): policy denied …`
commit events.

---

## 1. Escalation Gates

| Severity | Routing | Human-in-the-loop |
|---|---|---|
| `CRITICAL` | A4 staged dry-run + HIL queue | **Required** before apply |
| `HIGH` | A4 with policy gate | Required for irreversible action |
| `MEDIUM` | A4 autonomous | Not required |
| `LOW` | A4 autonomous | Not required |

**Irreversible actions** (anything that mutates production network surface,
external repos, or external accounts) **always** require human approval
regardless of severity.

---

## 2. ACL Matrix — Agent × Tool

| Tool | A1 | A2 | A3 | A4 | A5 |
|---|---|---|---|---|---|
| `cve-feed` / `nvd-feed` | ✅ | — | — | — | — |
| `shodan` / `greynoise` / `abuseipdb` | ✅ | — | — | — | — |
| `siem-webhook` / `honeypot-collector` | ✅ | — | — | — | — |
| `llm` (RAG, classification) | — | ✅ | — | — | — |
| `mitre-attack` / `actor-kb` | — | ✅ | — | — | — |
| `nftables` / `tailscale-acl` | — | — | — | ✅ (HIL gated) | — |
| `waf-cloudflare` | — | — | — | ✅ (HIL gated) | — |
| `github-actions` / `git-pr` | — | — | — | ✅ | — |
| `google-sheets` / `google-drive` | — | — | — | — | ✅ |
| `slh-dsa-sign` | — | — | — | — | ✅ (via `core`) |

Any (agent, tool) pair not explicitly listed is **denied by default**.

---

## 3. Network Mutation Policy

- **No agent** may mutate production network state without a corresponding
  dry-run artifact signed by A4 and persisted to the audit log.
- All mutations to `nftables`, Tailscale ACLs, and WAF rules are
  **rate-limited** to 5 distinct rule changes per 60 seconds at the
  kernel level. Bursts above this threshold automatically escalate to HIL.
- Rollback artifacts are generated and signed **before** the mutation is
  applied. Failed mutations auto-revert within 30 seconds.

---

## 4. Key Management Policy

- The **kernel SLH-DSA master keypair** lives only in the `core` context,
  inside an `Arc<Mutex<Option<PrivateKeyHandle>>>` that no other context
  has a reference to.
- Per-agent identity keypairs are issued **from** the master key via a
  signed certificate chain. Agents never see the master key.
- Per-agent ML-KEM keypairs are **ephemeral** — regenerated every agent
  boot. Old keys are zeroized.
- Key material is never written to disk unencrypted. At-rest keys are
  AES-256-GCM-wrapped with a KEK held in a hardware-isolated boundary
  (TPM 2.0 or equivalent).
- The browser dashboard receives **public keys only** via the WASM
  LibOQS module. There is no path by which a private key can leave Rust.

---

## 5. Audit Requirements

- **Every** cross-context dispatch emits a `BusEvent` to the bus. The bus
  logs envelope metadata (id, ts, sender, recipient, payload size, trace
  id) but never decrypts agent-to-agent payloads.
- **Every** `BusEvent` is appended to `audit_log.jsonl` on local disk by
  the `dx` context and (in production) batched to Google Sheets.
- **Every** outbound artifact (audit row, threat report, AI-BOM, SBOM) is
  SLH-DSA-signed before egress.
- The audit log is **append-only**. Any attempt to rewrite history is
  itself a `CRITICAL` policy violation that pages a human immediately.
- Retention: **7 years** for audit rows, **2 years** for raw signal
  payloads, **indefinite** for signed threat reports and AI-BOMs.

---

## 6. Amendment Procedure

Changes to this document require:

1. A pull request against `main` modifying `POLICY.md`.
2. Two human approvals (one operator, one security reviewer).
3. A new SLH-DSA signature over the resulting `POLICY.md` content.
4. A clean run of the security-scan workflow.

The kernel **will refuse to boot** if the on-disk `POLICY.md` signature
does not verify against the embedded kernel public key.
