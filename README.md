# PipeFish RECON - Agentic OS

> A living threat intelligence organism whose nervous system is the
> **Agentic OS kernel**. Five AI agents observing, reasoning, and acting
> under a cryptographically-signed constitutional policy.

**v0.1.0 is a bootable skeleton.** The kernel starts five runtime contexts
on one Tokio runtime and logs Conventional Commit-shaped bus events. Agent
roles, MCP tool dispatch, POLICY.md signature checks, and the operator UI
are specified in `SOUL.md` / `POLICY.md` but are not yet wired into `main`.

## Monorepo layout

```
.
  kernel/            # Rust package pipefish-recon-kernel (lib + bin)
    src/main.rs      # Kernel entrypoint
    src/contexts/    # core, bus, llm, tool, dx
    src/agents/      # RECON-A1 .. A5 stubs (not started at boot)
    src/crypto/      # ML-KEM-768 + SLH-DSA (feature-gated)
    src/mcp/         # Tool registry + ACL policy types
    src/commits/     # Conventional Commit formatter
    src/events.rs    # BusEvent schema
  dashboard/         # Placeholder operator package (no Next.js app yet)
  .github/
    workflows/       # CI, release, cargo-audit
    dependabot.yml   # Weekly cargo / npm / actions updates
  RECON.md           # Mission and principles
  SOUL.md            # Agent role definitions (RECON-A1 .. A5)
  POLICY.md          # Operating constitution (design target)
```

## Kernel architecture (what actually boots)

`kernel/src/main.rs` constructs a bounded mpsc bus (`capacity = 1024`) and
runs five contexts concurrently. Shutdown is `Ctrl+C` on all platforms and
`SIGTERM` on Unix. Windows has no SIGTERM listener.

```
CoreContext  ── healthcheck every 5s; holds empty signing-key slot
BusContext   ── drains the inbox and logs `type(scope): description`
LlmContext   ── idle tick every 30s; no LLM providers
ToolContext  ── empty MCP registry; idle tick every 60s
DxContext    ── writes its own ticks to ./audit_log.jsonl every 120s
```

Contexts talk **only** through `tokio::sync::mpsc::Sender<BusEvent>`.
`send().await` applies backpressure when the bus is full; events are not
silently dropped. `BusContext` currently **sinks** events (log only). It
does not fan out to `DxContext`, so `audit_log.jsonl` contains dx's own
boot/flush rows, not the full bus stream.

`kernel/src/agents/` (`ReconA1` … `ReconA5`) each emit one boot `BusEvent`
and return. **`main` does not spawn them.** Use the types from the library
crate if you are wiring agents in tests.

Public library modules (`kernel/src/lib.rs`):

| Module | Role |
|---|---|
| `events` | `BusEvent`, `CommitType`, `ContextId`, `ContextScope`, `Severity` |
| `contexts` | Five runtime contexts |
| `agents` | RECON-A1 .. A5 stubs |
| `crypto` | `ml_kem`, `slh_dsa` |
| `mcp` | `ToolRegistry`, `ToolPolicy` |
| `commits` | `CommitEngine::format` → Conventional Commit string |

Event format (see `BusEvent::to_commit_string`):

```text
feat(llm): routed threat signal to RECON-A3
```

## Building the kernel

Requires a stable Rust toolchain with Clippy (same as CI).

```bash
cargo check --manifest-path kernel/Cargo.toml
cargo test  --manifest-path kernel/Cargo.toml
cargo clippy --manifest-path kernel/Cargo.toml -- -D warnings
cargo run   --manifest-path kernel/Cargo.toml
```

Binary name: `pipefish-recon-kernel` (package name; `main.rs` + `lib.rs`).
With the repository workspace, Cargo writes artifacts under the workspace-root
`target/` by default; set `CARGO_TARGET_DIR` to override it.

Logging uses `tracing` + `EnvFilter`. Default is `info`:

```bash
RUST_LOG=pipefish_recon_kernel=debug,info cargo run --manifest-path kernel/Cargo.toml
```

On run, `DxContext` appends JSON lines to `./audit_log.jsonl` in the **process
working directory** (gitignored). Stop with Ctrl+C.

### Post-quantum feature

The default build compiles **without** libOQS so CI passes on a vanilla
runner. Crypto APIs still exist: `MlKemKeyPair::generate`, `encapsulate`,
`decapsulate`, `SlhDsaKeyPair::generate`, `sign`, and `verify` return empty
keys / empty signatures / `verify == true`.

Enable real Open Quantum Safe bindings with:

```bash
cargo build --manifest-path kernel/Cargo.toml --features pq-crypto
cargo test  --manifest-path kernel/Cargo.toml --features pq-crypto
```

Constraints when `pq-crypto` is on:

- Pulls optional crate `oqs` `0.9` (see `kernel/Cargo.toml`).
- ML-KEM uses `oqs::kem::Algorithm::MlKem768` (FIPS 203 ML-KEM-768).
- Signatures use `oqs::sig::Algorithm::SphincsShake128sSimple` — **not**
  the SLH-DSA-SHA2-128s parameter set named in `POLICY.md` / `RECON.md`.
- You need a working libOQS install; CI does **not** enable this feature.

## CI, Dependabot, and releases

| Workflow | Trigger | What it does |
|---|---|---|
| `.github/workflows/ci.yml` | push + pull_request | `cargo check`, `test`, `clippy -D warnings` on `kernel/Cargo.toml`. Permissions: `contents: read`. |
| `.github/workflows/security-scan.yml` | push to `main`, Monday 06:00 UTC | `cargo audit --file kernel/Cargo.lock` then falls back to `cargo audit`. |
| `.github/workflows/release.yml` | tag `v*`, or `workflow_dispatch` | Cross-compile release binaries for linux-x86_64, macos-x86_64 (macos-13), macos-arm64, windows-x86_64. Permissions: `contents: write`. Uploads artifacts; does not create a GitHub Release. |

Dependabot (weekly) updates:

- `cargo` in `/kernel`
- `npm` in `/dashboard` (no lockfile yet — PRs may be empty until the UI exists)
- `github-actions` at repo root

`Cargo.lock` is gitignored. `cargo audit --file kernel/Cargo.lock` will miss
unless a lockfile is generated locally. Reproducible release builds need an
explicit lockfile policy if you want one.

## Troubleshooting and pitfalls

| Symptom | Cause / fix |
|---|---|
| CI clippy fails on warnings | CI uses `clippy -- -D warnings`. Run the same command locally. The crate allows `dead_code` / `unused_*` at the crate root so scaffold types compile. |
| `pq-crypto` build fails | libOQS / `oqs` native dependency missing. Default (no feature) is the supported CI path. |
| Kernel runs but no agents / no CVE polling | Agents are not started from `main`. Signal ingestion in `SOUL.md` is the design, not this binary. |
| `audit_log.jsonl` missing most events | `DxContext` does not subscribe to the bus; it only writes its own rows. `BusContext` logs to tracing. |
| POLICY.md not enforced | `ToolPolicy::is_allowed` returns `true` for every pair. No `POLICY.md.sig` check at boot. |
| Release job cannot find the binary | Workflow copies from `target/<triple>/release/pipefish-recon-kernel`. `cargo --manifest-path kernel/Cargo.toml` normally emits `kernel/target/...` unless `CARGO_TARGET_DIR=target`. |
| Dashboard `npm install` / `next dev` | `dashboard/package.json` only has `name` + `version`. There is no Next.js app to start. |
| Security policy “1.0.x supported” | Crate version is `0.1.0`. See `SECURITY.md`. |
| Stale SKYNET names | The project was renamed to PipeFish RECON. Prefer `RECON-A1` … `A5` in new code and docs. |

## Principles

1. **Security-first at every layer.** Memory safety in the kernel (Rust),
   microVM isolation for every external tool, policy-gated execution.
2. **Post-quantum by default.** Design target: ML-KEM-768 (FIPS 203) for
   KEM, SLH-DSA (FIPS 205) for signatures. Default builds stub those APIs;
   see [Post-quantum feature](#post-quantum-feature).
3. **Every action is a commit.** Every agentic decision emits a Conventional
   Commit event. Auditable, diff-able, revertible.

See [`RECON.md`](./RECON.md), [`SOUL.md`](./SOUL.md), and [`POLICY.md`](./POLICY.md).
