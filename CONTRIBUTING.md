# Contributing to pipefish-recon

Thank you for your interest in contributing to pipefish-recon!

## How to Contribute

### Reporting Bugs

* Search existing issues to ensure the bug has not already been reported.
* Open a new issue with a clear title, a detailed description, and steps to reproduce the behavior.

Security vulnerabilities: follow [`SECURITY.md`](./SECURITY.md) (email, not a public issue).

### Suggesting Enhancements

* Open a new issue detailing the suggested enhancement.
* Explain the use case and why it would be beneficial for the project's reconnaissance or security operations.
* If the change is already described in `SOUL.md` / `POLICY.md`, say whether you are implementing that spec or changing it.

### Pull Requests

1. Fork the `donny-devops/pipefish-recon` repository.
2. Create a new branch from `main` (`git checkout -b feature/your-feature`).
3. Make your changes and add tests if applicable.
4. Ensure the test suite passes and your code conforms to the project's style guidelines.
5. Open a pull request with a clear description of the changes.

## Local checks (must match CI)

CI (`.github/workflows/ci.yml`) runs on every push and pull request:

```bash
cargo check  --manifest-path kernel/Cargo.toml
cargo test   --manifest-path kernel/Cargo.toml
cargo clippy --manifest-path kernel/Cargo.toml -- -D warnings
```

Do **not** require `--features pq-crypto` for a green PR. That feature is
optional and is not enabled in CI.

Release binaries are built from `kernel/Cargo.toml --release --target <triple>`.
The binary name is `pipefish-recon-kernel`.

## Kernel conventions

* Public modules live in `kernel/src/lib.rs`. The process entrypoint is
  `kernel/src/main.rs` — keep them in sync if you add a module.
* Cross-context communication is `BusEvent` on the mpsc bus (`events.rs`).
  Prefer `CommitType` + `ContextScope` that match Conventional Commits
  (`feat`/`fix`/`chore`/`docs`/`refactor`/`security`).
* `POLICY.md` is the design constitution. Runtime ACL is still a stub
  (`mcp::policy::ToolPolicy::is_allowed` currently allows everything).
  Do not document stubbed APIs as enforced until they load a real policy.
* Agent files under `kernel/src/agents/` are not started from `main`.
  If you add a real agent loop, wire it explicitly and cover it with a test.

## Docs

Update [`README.md`](./README.md) when you change boot behavior, Cargo
features, CI, or the release workflow. Keep mission prose in `RECON.md` /
`SOUL.md` / `POLICY.md` distinct from “what the binary does today.”
