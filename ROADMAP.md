# Roadmap

PipeFish RECON versioning. Mission and constitution stay in [`RECON.md`](./RECON.md),
[`SOUL.md`](./SOUL.md), and [`POLICY.md`](./POLICY.md). This file is **what ships
when**, not the design target.

## v0.2.0

Bootable kernel plus operator surface:

- Deny-by-default tool ACL loaded from `POLICY.md` §2 at boot
- RECON-A1 … A5 spawned from `main` as long-running bus citizens
- Every `BusEvent` fanned out to `./audit_log.jsonl`
- Operator HTTP on `127.0.0.1:8080`
- Next.js 15 dashboard in `dashboard/`

## v0.3.0 (this branch)

- Canonical `ThreatSignal` schema (`kernel/src/signals.rs`)
- RECON-A1 NVD CVE 2.0 poller (`PIPEFISH_NVD_POLL`, default on) plus
  `POST /api/ingest` webhook
- 30-day CVE de-duplication
- RECON-A3 deterministic severity router (CVSS + KEV bump → POLICY §1 routes)
- Bus fan-out to both dx and A3

Not in v0.3.0: LLM providers, nftables/WAF, `POLICY.md.sig` boot refusal,
SLH-DSA-SHA2-128s, Google Sheets/Drive, browser LibOQS WASM, HIL apply.

## v0.4

- Detached `POLICY.md.sig` (SLH-DSA) verified before the ACL is installed
- Switch `pq-crypto` signatures from `SphincsShake128sSimple` to SLH-DSA-SHA2-128s
- Track `Cargo.lock` in git so `cargo audit` does not generate a lockfile in CI

## v0.5

- Human-in-the-loop queue for HIGH/CRITICAL
- RECON-A4 dry-run artifacts (signed, persisted) — no production nftables apply

## v0.6

- Batch audit rows to Google Sheets / Drive
- Browser LibOQS WASM verification of SLH-DSA signatures on audit rows
