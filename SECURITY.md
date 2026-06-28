# SECURITY.md

## Overview

Security is a first-class concern for the **Skynet** project. This document explains how we handle security, how to report vulnerabilities, and what you can expect from us in terms of response and disclosure.

---

## Supported versions

We provide security updates only for the latest released version of Skynet.

| Version  | Supported |
|---------|-----------|
| main    | ✅        |
| others  | ❌        |

If you are running a fork or a modified version, please ensure you can reproduce any issue on the `main` branch before reporting.

---

## Reporting a vulnerability

**Please do not open public GitHub issues for security vulnerabilities.**

Instead, contact us privately:

- **Email:** `security@skynet.local` (replace with your real security contact)
- **GitHub Security Advisory:** Use the “Report a vulnerability” feature in the repository’s **Security** tab if enabled.

When reporting, include:

- **Description:** Clear explanation of the issue and potential impact.
- **Steps to reproduce:** Minimal, reproducible example or exact sequence of actions.
- **Environment details:** OS, runtime, Skynet version, configuration specifics.
- **Proof of concept:** If available, include PoC code or screenshots.
- **Suggested severity:** Your assessment (Low/Medium/High/Critical).

Please avoid sharing:

- Exploit code publicly.
- Sensitive data (real secrets, personal data, production logs) unless strictly necessary—redact wherever possible.

---

## Our response process

When we receive a report, we aim to:

1. **Acknowledge receipt** within 72 hours.
2. **Assess severity** and confirm reproducibility.
3. **Develop and test a fix** or mitigation.
4. **Coordinate disclosure**:
   - Prepare a patched release.
   - Publish a security advisory with CVE (if applicable).
   - Share upgrade or mitigation instructions.

We prioritize **Critical** and **High** severity issues that:

- Allow remote code execution.
- Bypass authentication or authorization.
- Expose sensitive data.
- Enable privilege escalation.

---

## Security best practices for Skynet deployments

To reduce risk when running Skynet:

- **Least privilege:**  
  - Run services with minimal OS and network permissions.
  - Avoid running as `root` unless strictly required.

- **Secrets management:**  
  - Store API keys, tokens, and credentials in a secure vault (e.g., Azure Key Vault, AWS Secrets Manager, HashiCorp Vault).
  - Never commit secrets to the repository.

- **Network hardening:**  
  - Restrict inbound access to administrative endpoints.
  - Use firewalls, security groups, or service meshes to limit communication.
  - Prefer TLS for all external and internal traffic
