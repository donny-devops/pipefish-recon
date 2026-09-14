# Security Policy

## Supported Versions

The published crate / binary is `pipefish-recon-kernel` **0.1.x**
(`kernel/Cargo.toml`). There is no 1.x release. This branch targets **0.2.0**.

| Version | Supported |
| ------- | --------- |
| 0.2.x (`main`, after merge) | :white_check_mark: |
| 0.1.x | :white_check_mark: |
| unreleased 1.0.x | not shipped |

Default CI builds **do not** enable `--features pq-crypto`. Cryptographic
stubs in that configuration are for compilation and unit-test shape only;
do not treat empty ML-KEM / SLH-DSA keys as production crypto.

`POLICY.md` is loaded at boot as the deny-by-default tool ACL. Signature
verification of `POLICY.md.sig` is not yet implemented.

## Reporting a Vulnerability

Please report security vulnerabilities by emailing pipefish.labs@gmail.com.
We treat all security reports as a top priority and will acknowledge your
report within 48 hours.
Please do not report security vulnerabilities via public GitHub issues to
prevent premature disclosure.
