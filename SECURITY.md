# Security Policy

## Supported Versions

The published crate / binary is `pipefish-recon-kernel` **0.1.0**
(`kernel/Cargo.toml`). There is no 1.x release.

| Version | Supported |
| ------- | --------- |
| 0.1.x (`main`) | :white_check_mark: |
| unreleased 1.0.x | not shipped |

Default CI builds **do not** enable `--features pq-crypto`. Cryptographic
stubs in that configuration are for compilation and unit-test shape only;
do not treat empty ML-KEM / SLH-DSA keys as production crypto.

`POLICY.md` describes the intended ACL and key-handling rules. The running
kernel does not yet enforce that constitution (see the implementation note
in `POLICY.md`).

## Reporting a Vulnerability

Please report security vulnerabilities by emailing pipefish.labs@gmail.com.
We treat all security reports as a top priority and will acknowledge your
report within 48 hours.
Please do not report security vulnerabilities via public GitHub issues to
prevent premature disclosure.
