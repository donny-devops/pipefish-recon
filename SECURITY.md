Security Policy
Overview
SKYNET is a threat intelligence aggregation platform integrating multiple external security APIs
(VirusTotal, AbuseIPDB, Shodan, GreyNoise, PagerDuty, Wazuh). This policy defines how security
vulnerabilities in this project are reported, triaged, and resolved.

Supported Versions
Only the latest release branch receives security patches. Older versions are unsupported.
VersionSupportedmain / latest tag✅ Active supportPrior releases❌ No patches

Reporting a Vulnerability
Do not open a public GitHub issue for security vulnerabilities.
Report security issues privately via one of the following channels:

GitHub Private Advisory: Security Advisories

Please include the following in your report:

A clear description of the vulnerability
Affected component(s) — e.g., API key handling, webhook endpoint, integration adapter
Steps to reproduce or a proof-of-concept (sanitize any real credentials)
Potential impact assessment
Your suggested fix or mitigation, if available

You will receive an acknowledgment within 72 hours and a triage decision within 7 days.

Scope
In Scope
The following are considered in-scope for security reports:

API key exposure or insecure credential storage
Authentication/authorization bypass on any SKYNET endpoint
Server-Side Request Forgery (SSRF) via threat feed lookups
Injection vulnerabilities (SQL, command, template) in IOC processing pipelines
Insecure deserialization in alert ingestion from Wazuh or PagerDuty webhooks
Sensitive data leakage in logs, error responses, or API responses
Rate-limit bypass or denial-of-service vectors in the enrichment pipeline
Dependency vulnerabilities with exploitable CVEs (CVSS ≥ 7.0)

Out of Scope

Vulnerabilities in upstream third-party APIs (VirusTotal, Shodan, AbuseIPDB, etc.)
Rate limiting or quota exhaustion of external API keys (operational, not security)
Issues requiring physical access to the host system
Social engineering attacks


Sensitive Data & Secrets Handling
SKYNET integrates multiple external API credentials. The following rules apply to all contributors:

Never commit API keys, tokens, or secrets to this repository — not even in test files or comments.
All secrets must be stored in environment variables or a secrets manager (e.g., AWS Secrets Manager, HashiCorp Vault).
The .env file is listed in .gitignore and must never be tracked.
Rotate any accidentally exposed credential immediately and treat it as compromised.
Use the VIRUSTOTAL_API_KEY, ABUSEIPDB_API_KEY, SHODAN_API_KEY, GREYNOISE_API_KEY,
PAGERDUTY_API_KEY, and WAZUH_API_TOKEN environment variable names defined in .env.example.

If you discover a committed secret in the repository history, report it immediately via the
channels above. Do not attempt to scrub history publicly without coordinating with maintainers.

Dependency Security
This project uses automated dependency scanning:

Dependabot is enabled for Python (pip) and Docker dependencies.
Dependabot security alerts are reviewed within 14 days of notification.
Critical/High CVEs (CVSS ≥ 7.0) are patched within 7 days of a confirmed fix being available.
All PRs run CI checks including dependency audit steps before merge.

To manually audit dependencies:
bash# Python
pip audit

# Docker
docker scout cves dreamtechusa/skynet:latest

Infrastructure & Deployment Security

All SKYNET containers run as non-root users.
Docker images are built from pinned base image digests — no latest tags in production.
Network egress from the SKYNET container is restricted to required API endpoints only.
Wazuh and PagerDuty webhook endpoints must be protected by a shared HMAC secret validated
on every inbound request.
TLS 1.2+ is required for all external API connections; TLS 1.3 is preferred.


Disclosure Policy
SKYNET follows a coordinated disclosure model:

Reporter submits vulnerability privately.
Maintainers acknowledge within 72 hours.
Triage and severity assessment within 7 days.
Fix developed and tested in a private branch.
Patch released and a GitHub Security Advisory published.
Reporter credited in the advisory (unless anonymity is requested).

Target remediation timelines by severity:
SeverityCVSS RangeTarget Patch TimelineCritical9.0 – 10.048 hoursHigh7.0 – 8.97 daysMedium4.0 – 6.930 daysLow0.1 – 3.9Next scheduled release

Security Contacts
RoleContactPrimaryMaintainer @donny-devops SecurityEmail donnydev@outlook.com

Acknowledgments
We appreciate responsible security research. Reporters who follow this policy and submit valid,
in-scope vulnerabilities will be publicly credited in the associated Security Advisory unless
they prefer to remain anonymous.
