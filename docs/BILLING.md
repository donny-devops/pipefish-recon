# Optional billing gate

PipeFish RECON can check a billing service before it accepts a recon scan.
This is **off by default**. Self-hosted kernels that do not set
`PIPEFISH_BILLING_ENABLED` never call billing and keep the current boot path.

## Enable

| Variable | Required when enabled | Default | Purpose |
|---|---|---|---|
| `PIPEFISH_BILLING_ENABLED` | yes (`true` / `1` / `yes` / `on`) | unset (off) | Master flag |
| `BILLING_PUBLIC_URL` | no | `http://localhost:8410` | Billing base URL |
| `PIPEFISH_API_KEY` | yes | — | Bearer token that identifies the account |
| `BILLING_CLIENT_TIMEOUT` | no | `5` (seconds) | HTTP timeout |
| `PIPEFISH_BILLING_ON_ERROR` | no | `fail-closed` | `fail-closed` or `fail-open` |

Do not put API keys in git. Use environment / secret storage.

## Scan path

`POST /api/scans` is the operator entry that accepts a recon scan.

1. `POST {BILLING_PUBLIC_URL}/v1/access/authorize`  
   `Authorization: Bearer <PIPEFISH_API_KEY>`  
   body: `{"channel":"api","meter":"recon_scan","quantity":1}`
2. If allowed, the scan is accepted and the kernel records usage:  
   `POST {BILLING_PUBLIC_URL}/v1/usage`  
   body: `{"meter":"recon_scan","quantity":1,"channel":"api","idempotency_key":"recon_scan:{scan_id}"}`

There is no `account_id` in the body. The API key is the account identity.

## Errors

Application denials (not outages):

| Billing HTTP | Kernel `error` | Operator HTTP |
|---|---|---|
| 401 | `unauthorized` | 401 |
| 429 JSON includes `meter` | `quota_exceeded` | 429 |
| 429 without `meter` | `rate_limited` | 429 |

The human-readable message is billing's `detail` field.

## Unreachable billing (fail-closed vs fail-open)

`PIPEFISH_BILLING_ON_ERROR` applies **only** to network errors and HTTP 5xx.
401 and 429 are never fail-open.

**Default is fail-closed:** if billing cannot be reached, the scan is refused
(`error=billing_unavailable`, HTTP 503). That is the right default for a
metered SaaS gate — an outage should not leak unpaid `recon_scan` usage.
Self-hosted deployments leave billing unset, so they never hit this path.

Set `PIPEFISH_BILLING_ON_ERROR=fail-open` if you would rather accept scans
when billing is down. Usage is not recorded in that case.

If authorize succeeds and usage recording fails, the scan stays accepted
(entitlement was already granted). The failure is logged.
