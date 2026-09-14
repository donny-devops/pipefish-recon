# Roadmap

PipeFish RECON versioning. Mission and constitution stay in [`RECON.md`](./RECON.md),
[`SOUL.md`](./SOUL.md), and [`POLICY.md`](./POLICY.md). This file is **what ships
when**, not the design target.

## v0.2.0 (this branch)

Bootable kernel plus operator surface:

- Deny-by-default tool ACL loaded from `POLICY.md` §2 at boot
- RECON-A1 … A5 spawned from `main` as long-running bus citizens
- Every `BusEvent` fanned out to `./audit_log.jsonl` (full stream, not dx-only ticks)
- Operator HTTP on `127.0.0.1:8080` (`/health`, `/api/status`, `/api/events`, `/api/policy`)
- Next.js 15 dashboard in `dashboard/` reading that API

Not in v0.2.0: live CVE/NVD polling, LLM providers, nftables/WAF, `POLICY.md.sig`
boot refusal, SLH-DSA-SHA2-128s, Google Sheets/Drive, browser LibOQS WASM.

## v0.3

- Canonical `ThreatSignal` schema
- RECON-A1 NVD / CVE poller (and optional webhook ingest)
- RECON-A3 deterministic severity router (`CRITICAL`/`HIGH`/`MEDIUM`/`LOW`)

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
