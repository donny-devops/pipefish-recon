//! Minimal Google Sheets v4 client for audit-row append.
//!
//! Uses the service-account JWT-bearer OAuth flow described at
//! <https://developers.google.com/identity/protocols/oauth2/service-account>.
//! Tokens are cached in-process until five minutes before expiry; expired
//! tokens are refreshed lazily on the next call.
//!
//! Pure `reqwest` + `jsonwebtoken` — we deliberately avoid the
//! `google-sheets4` / `yup-oauth2` stack because we only need one endpoint
//! (`spreadsheets.values.append`) and the heavyweight crates would balloon
//! kernel build time.

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tracing::warn;

use crate::events::BusEvent;

const TOKEN_ENDPOINT: &str = "https://oauth2.googleapis.com/token";
const SCOPE: &str = "https://www.googleapis.com/auth/spreadsheets";
const AUDIT_SHEET_NAME: &str = "AuditLog";
const TOKEN_LIFETIME_SECS: i64 = 3600;
const TOKEN_REFRESH_MARGIN_SECS: i64 = 300;

#[derive(Debug, Clone)]
pub struct CachedToken {
    pub token: String,
    pub expires_at: DateTime<Utc>,
}

impl CachedToken {
    fn is_expired(&self) -> bool {
        Utc::now() + Duration::seconds(TOKEN_REFRESH_MARGIN_SECS) >= self.expires_at
    }
}

#[derive(Debug, Deserialize)]
struct ServiceAccountKey {
    client_email: String,
    private_key: String,
    #[serde(default = "default_token_uri")]
    token_uri: String,
}

fn default_token_uri() -> String {
    TOKEN_ENDPOINT.to_string()
}

#[derive(Debug, Serialize)]
struct JwtClaims {
    iss: String,
    scope: String,
    aud: String,
    iat: i64,
    exp: i64,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    #[serde(default)]
    expires_in: Option<i64>,
}

pub struct SheetsClient {
    http: reqwest::Client,
    spreadsheet_id: String,
    key: ServiceAccountKey,
    token: Arc<Mutex<Option<CachedToken>>>,
}

impl SheetsClient {
    /// Build a client from environment variables.
    ///
    /// Returns `Ok(None)` — and logs a warning — when either
    /// `GOOGLE_SERVICE_ACCOUNT_KEY` or `SKYNET_SHEETS_ID` is unset, so the
    /// kernel can boot and operate normally without Google credentials.
    pub fn from_env() -> Result<Option<Self>> {
        let key_path = match std::env::var("GOOGLE_SERVICE_ACCOUNT_KEY") {
            Ok(v) if !v.is_empty() => PathBuf::from(v),
            _ => {
                warn!("dx: GOOGLE_SERVICE_ACCOUNT_KEY not set — Sheets sync disabled");
                return Ok(None);
            }
        };
        let spreadsheet_id = match std::env::var("SKYNET_SHEETS_ID") {
            Ok(v) if !v.is_empty() => v,
            _ => {
                warn!("dx: SKYNET_SHEETS_ID not set — Sheets sync disabled");
                return Ok(None);
            }
        };

        let raw = std::fs::read_to_string(&key_path)
            .with_context(|| format!("reading service account key {:?}", key_path))?;
        let key: ServiceAccountKey =
            serde_json::from_str(&raw).context("parsing service account key JSON")?;

        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .context("building reqwest client")?;

        Ok(Some(Self {
            http,
            spreadsheet_id,
            key,
            token: Arc::new(Mutex::new(None)),
        }))
    }

    /// Fetch a cached bearer token, refreshing via the JWT-bearer flow when
    /// missing or close to expiry.
    async fn ensure_token(&self) -> Result<String> {
        let mut guard = self.token.lock().await;
        if let Some(cached) = guard.as_ref() {
            if !cached.is_expired() {
                return Ok(cached.token.clone());
            }
        }

        let now = Utc::now();
        let claims = JwtClaims {
            iss: self.key.client_email.clone(),
            scope: SCOPE.to_string(),
            aud: self.key.token_uri.clone(),
            iat: now.timestamp(),
            exp: now.timestamp() + TOKEN_LIFETIME_SECS,
        };

        let encoding_key = EncodingKey::from_rsa_pem(self.key.private_key.as_bytes())
            .context("parsing service account private key as RSA PEM")?;
        let header = Header::new(Algorithm::RS256);
        let assertion =
            encode(&header, &claims, &encoding_key).context("signing service account JWT")?;

        let resp = self
            .http
            .post(&self.key.token_uri)
            .form(&[
                ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
                ("assertion", &assertion),
            ])
            .send()
            .await
            .context("posting to Google token endpoint")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!(
                "token endpoint returned {status}: {body}"
            ));
        }

        let parsed: TokenResponse = resp
            .json()
            .await
            .context("parsing Google token response")?;
        let lifetime = parsed.expires_in.unwrap_or(TOKEN_LIFETIME_SECS);
        let cached = CachedToken {
            token: parsed.access_token,
            expires_at: Utc::now() + Duration::seconds(lifetime),
        };
        let token = cached.token.clone();
        *guard = Some(cached);
        Ok(token)
    }

    /// Format a [`BusEvent`] as the eight-column audit row.
    ///
    /// Column order: `id, timestamp, source, commit_type, scope, description,
    /// severity, payload_json`.
    pub(crate) fn row_for(event: &BusEvent) -> Vec<String> {
        let severity = event
            .severity
            .as_ref()
            .map(|s| format!("{:?}", s).to_uppercase())
            .unwrap_or_else(|| "-".to_string());
        let payload_json = event
            .payload
            .as_ref()
            .map(|p| p.to_string())
            .unwrap_or_default();
        vec![
            event.id.to_string(),
            event.timestamp.to_rfc3339(),
            event.source.as_str().to_string(),
            event.commit_type.as_str().to_string(),
            event.scope.as_str().to_string(),
            event.description.clone(),
            severity,
            payload_json,
        ]
    }

    /// Append one audit row to the spreadsheet.
    pub async fn append_row(&self, event: &BusEvent) -> Result<()> {
        let token = self.ensure_token().await?;
        let row = Self::row_for(event);
        let body = serde_json::json!({ "values": [row] });

        let range = format!("{}!A:H", AUDIT_SHEET_NAME);
        let url = format!(
            "https://sheets.googleapis.com/v4/spreadsheets/{}/values/{}:append",
            urlencode(&self.spreadsheet_id),
            urlencode(&range)
        );

        let resp = self
            .http
            .post(&url)
            .bearer_auth(&token)
            .query(&[
                ("valueInputOption", "RAW"),
                ("insertDataOption", "INSERT_ROWS"),
            ])
            .json(&body)
            .send()
            .await
            .context("posting append request to Sheets API")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("Sheets append failed {status}: {body}"));
        }
        Ok(())
    }
}

fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::{CommitType, ContextId, ContextScope, Severity};

    #[test]
    fn from_env_returns_none_when_unset() {
        // SAFETY: tests in this crate are not parallelised across env-var
        // mutations, but to be defensive we save and restore.
        let prev_key = std::env::var("GOOGLE_SERVICE_ACCOUNT_KEY").ok();
        let prev_id = std::env::var("SKYNET_SHEETS_ID").ok();
        std::env::remove_var("GOOGLE_SERVICE_ACCOUNT_KEY");
        std::env::remove_var("SKYNET_SHEETS_ID");

        let result = SheetsClient::from_env().expect("must not error when unset");
        assert!(result.is_none());

        if let Some(v) = prev_key {
            std::env::set_var("GOOGLE_SERVICE_ACCOUNT_KEY", v);
        }
        if let Some(v) = prev_id {
            std::env::set_var("SKYNET_SHEETS_ID", v);
        }
    }

    #[test]
    fn row_for_produces_eight_columns_in_order() {
        let evt = BusEvent::new(
            ContextId::Llm,
            CommitType::Feat,
            ContextScope::Llm,
            "routed signal",
        )
        .with_severity(Severity::High)
        .with_payload(serde_json::json!({"k": "v"}));

        let row = SheetsClient::row_for(&evt);
        assert_eq!(row.len(), 8);
        assert_eq!(row[0], evt.id.to_string());
        assert_eq!(row[1], evt.timestamp.to_rfc3339());
        assert_eq!(row[2], "llm");
        assert_eq!(row[3], "feat");
        assert_eq!(row[4], "llm");
        assert_eq!(row[5], "routed signal");
        assert_eq!(row[6], "HIGH");
        assert_eq!(row[7], r#"{"k":"v"}"#);
    }

    #[test]
    fn row_for_handles_missing_severity_and_payload() {
        let evt = BusEvent::new(
            ContextId::Dx,
            CommitType::Chore,
            ContextScope::Dx,
            "boot",
        );
        let row = SheetsClient::row_for(&evt);
        assert_eq!(row.len(), 8);
        assert_eq!(row[6], "-");
        assert_eq!(row[7], "");
    }
}
