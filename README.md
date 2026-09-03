# PipeFish RECON - Agentic OS

> A living threat intelligence organism whose nervous system is the
> **Agentic OS kernel**. Five AI agents observing, reasoning, and acting
> under a cryptographically-signed constitutional policy.

## Monorepo layout

```
recon/
  kernel/        # Rust workspace crate — the Agentic OS kernel
  dashboard/     # Next.js 15 operator UI (placeholder)
  .github/
    workflows/   # CI + security scans
  RECON.md      # Mission and principles
  SOUL.md        # Agent role definitions (SKYNET-A1 .. A5)
  POLICY.md      # Operating constitution (escalation, ACL, key management)
```

## Building the kernel

```bash
cargo check --manifest-path kernel/Cargo.toml
cargo test  --manifest-path kernel/Cargo.toml
cargo run   --manifest-path kernel/Cargo.toml
```

The default build compiles **without** libOQS so CI passes on a vanilla
runner. Enable real post-quantum crypto with the `pq-crypto` feature:

```bash
cargo build --manifest-path kernel/Cargo.toml --features pq-crypto
```

## Principles

1. **Security-first at every layer.** Memory safety in the kernel (Rust),
   microVM isolation for every external tool, policy-gated execution.
2. **Post-quantum by default.** ML-KEM-768 (FIPS 203) for KEM, SLH-DSA
   (FIPS 205) for signatures. No "Q-Day" assumptions — harvest-now-decrypt-
   later is already active.
3. **Every action is a commit.** Every agentic decision emits a Conventional
   Commit event. Auditable, diff-able, revertible.

See `RECON.md` and the architecture blueprint for full context.
