# PipeFish RECON - Agentic OS

> A living threat intelligence organism whose nervous system is the
> **Agentic OS kernel**. Five AI agents observing, reasoning, and acting
> under a constitutional policy loaded from `POLICY.md`.

**v0.3.0** adds a canonical `ThreatSignal`, an NVD CVE 2.0 poller plus
webhook ingest on RECON-A1, and a deterministic RECON-A3 severity router.
LLM providers, nftables apply, and `POLICY.md.sig` verification are **not**
in this release — see [`ROADMAP.md`](./ROADMAP.md).

## Monorepo layout

```
.
  kernel/            # Rust package pipefish-recon-kernel 0.3.0 (lib + bin)
    src/main.rs      # Kernel entrypoint
    src/contexts/    # core, bus, llm, tool, dx
    src/agents/      # RECON-A1 .. A5 (spawned at boot)
    src/signals.rs   # ThreatSignal + NVD CVE 2.0 parser
    src/routing.rs   # A3 severity × POLICY §1 routes
    src/crypto/      # ML-KEM-768 + SLH-DSA (feature-gated)
    src/mcp/         # Tool registry + POLICY.md ACL parser
    src/operator.rs  # Localhost JSON API + POST /api/ingest
    src/commits/     # Conventional Commit formatter
    src/events.rs    # BusEvent schema
  dashboard/         # Next.js 15 operator UI
  .github/
    workflows/       # CI, release, cargo-audit
    dependabot.yml
  RECON.md           # Mission and principles
  SOUL.md            # Agent role definitions
  POLICY.md          # Operating constitution (ACL loaded at boot)
  ROADMAP.md         # What ships when
```

## Kernel architecture (what actually boots)

`kernel/src/main.rs` loads `POLICY.md`, constructs a bounded mpsc bus
(`capacity = 1024`), and runs five contexts plus five agents concurrently.
Shutdown is Ctrl+C on all platforms and SIGTERM on Unix.

```
CoreContext  -- healthcheck every 5s; holds loaded ToolPolicy + empty signing-key slot
BusContext   -- drains the inbox, logs type(scope): description, fans out to subscribers
LlmContext   -- idle tick every 30s; no LLM providers
ToolContext  -- empty MCP registry; check_dispatch uses deny-by-default ACL
DxContext    -- writes the full bus stream to ./audit_log.jsonl; operator HTTP
RECON-A1     -- NVD poll + POST /api/ingest webhook; 30-day CVE dedupe
RECON-A3     -- deterministic CVSS/KEV router (HIL decision recorded, not applied)
RECON-A2/A4/A5 -- long-running stubs
```

Contexts and agents talk **only** through `tokio::sync::mpsc::Sender<BusEvent>`.
`send().await` applies backpressure when the bus is full; events are not
silently dropped. `BusContext` fans every event to `DxContext` and RECON-A3.

Event format (`BusEvent::to_commit_string`):

```text
feat(llm): routed threat signal to RECON-A3
```

## Building the kernel

Requires a stable Rust toolchain with Clippy and rustfmt (same as CI).
MSRV for this crate is **1.83**.

```bash
cargo fmt    --manifest-path kernel/Cargo.toml -- --check
cargo check  --manifest-path kernel/Cargo.toml
cargo test   --manifest-path kernel/Cargo.toml
cargo clippy --manifest-path kernel/Cargo.toml -- -D warnings
cargo run    --manifest-path kernel/Cargo.toml
```

Binary name: `pipefish-recon-kernel`. With the repository workspace, Cargo
writes artifacts under the workspace-root `target/` by default; set
`CARGO_TARGET_DIR` to override it.

Policy path: `PIPEFISH_POLICY_PATH`, else `./POLICY.md`, else
`kernel/../POLICY.md`. Missing or unparseable ACL fails boot.

Operator bind: `PIPEFISH_OPERATOR_BIND` (default `127.0.0.1:8080`).

```
GET /health
GET /api/status
GET /api/events?limit=50
GET /api/policy
POST /api/ingest   { "cve_id", "cvss?", "summary?" }
```

NVD poller (RECON-A1): `PIPEFISH_NVD_POLL` (default `1`; set `0` to disable),
`PIPEFISH_NVD_URL`, `PIPEFISH_NVD_INTERVAL_SECS` (default 900). Failed polls
are logged and the kernel stays up.

Logging uses `tracing` + `EnvFilter`. Default is `info`:

```bash
RUST_LOG=pipefish_recon_kernel=debug,info cargo run --manifest-path kernel/Cargo.toml
```

On run, `DxContext` appends JSON lines to `./audit_log.jsonl` in the process
working directory (gitignored). Stop with Ctrl+C.

### Post-quantum feature

The default build compiles **without** libOQS so CI passes on a vanilla
runner. Crypto APIs still exist: `MlKemKeyPair::generate`, `encapsulate`,
`decapsulate`, `SlhDsaKeyPair::generate`, `sign`, and `verify` return empty
keys / empty signatures / `verify == true`.

```bash
cargo build --manifest-path kernel/Cargo.toml --features pq-crypto
cargo test  --manifest-path kernel/Cargo.toml --features pq-crypto
```

- Pulls optional crate `oqs` `0.9`.
- ML-KEM uses `oqs::kem::Algorithm::MlKem768`.
- Signatures use `oqs::sig::Algorithm::SphincsShake128sSimple` — **not**
  the SLH-DSA-SHA2-128s parameter set named in `POLICY.md` / `RECON.md`.
- CI does **not** enable this feature.

## Operator dashboard

```bash
cd dashboard
npm install
npm run dev
```

Open `http://localhost:3000`. The UI polls the kernel API (CORS allows
localhost:3000). Set `NEXT_PUBLIC_KERNEL_URL` to override
`http://127.0.0.1:8080`.

## CI, Dependabot, and releases

| Workflow | Trigger | What it does |
|---|---|---|
| `.github/workflows/ci.yml` | push + pull_request | `cargo fmt --check`, `check`, `test`, `clippy -D warnings` on `kernel/Cargo.toml`; `npm ci` + `npm run build` in `dashboard/`. Permissions: `contents: read`. |
| `.github/workflows/security-scan.yml` | push to `main`, Monday 06:00 UTC | `cargo generate-lockfile --manifest-path kernel/Cargo.toml`, `cargo check --locked`, then `cargo audit --file Cargo.lock`. |
| `.github/workflows/release.yml` | tag `v*`, or `workflow_dispatch` | Cross-compile release binaries. Uploads artifacts; does not create a GitHub Release. |

`Cargo.lock` is gitignored. Security scan generates a workspace-root lockfile
in CI before auditing.

## Troubleshooting

| Symptom | Cause / fix |
|---|---|
| CI clippy fails on warnings | Run `cargo clippy --manifest-path kernel/Cargo.toml -- -D warnings`. |
| Boot fails reading POLICY.md | Run from repo root or set `PIPEFISH_POLICY_PATH`. |
| A1 cannot use nftables | Expected: deny-by-default ACL from §2. |
| `pq-crypto` build fails | libOQS missing. Default (no feature) is the supported CI path. |
| Kernel runs but no CVE polling | Default poller is on (`PIPEFISH_NVD_POLL=1`). Check `PIPEFISH_NVD_URL` / network. Use `POST /api/ingest` or the dashboard Events form for a local signal. |
| Duplicate CVE ignored | A1 de-duplicates the same `cve_id` for 30 days. |
| POLICY.md.sig not checked | Signature boot gate is v0.4. |
| Dashboard cannot reach kernel | Start the kernel first; browser must hit localhost:3000 for CORS. |
| Release job cannot find the binary | Path is `target/<triple>/release/pipefish-recon-kernel` at the workspace root. |

## Principles

1. **Security-first at every layer.** Memory safety in the kernel (Rust),
   microVM isolation for every external tool, policy-gated execution.
2. **Post-quantum by default.** Design target: ML-KEM-768 (FIPS 203) for
   KEM, SLH-DSA (FIPS 205) for signatures. Default builds stub those APIs.
3. **Every action is a commit.** Every agentic decision emits a Conventional
   Commit event. Auditable, diff-able, revertible.

See [`RECON.md`](./RECON.md), [`SOUL.md`](./SOUL.md), [`POLICY.md`](./POLICY.md),
and [`ROADMAP.md`](./ROADMAP.md).
