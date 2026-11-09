//! Optional PipeFish billing gate for recon scans.
//!
//! Off unless `PIPEFISH_BILLING_ENABLED` is set, so self-hosted kernels make
//! no billing HTTP calls and keep today's boot path.
//!
//! When enabled, each accepted scan:
//! 1. `POST /v1/access/authorize` with meter `recon_scan`
//! 2. `POST /v1/usage` after a successful authorize (scan already accepted)
//!
//! Auth is `Authorization: Bearer <PIPEFISH_API_KEY>` — the key identifies the
//! account. Request bodies have no `account_id`.
//!
//! Network errors and HTTP 5xx honor `PIPEFISH_BILLING_ON_ERROR` (default
//! **fail-closed**). 401 / 429 are application denials, not outages.

use std::time::Duration;

use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};
use uuid::Uuid;

/// Meter billed for one accepted recon scan.
pub const RECON_SCAN_METER: &str = "recon_scan";
/// Usage channel recorded for operator-triggered scans.
pub const RECON_SCAN_CHANNEL: &str = "api";

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);
const DEFAULT_BASE_URL: &str = "http://localhost:8410";
const AUTHORIZE_PATH: &str = "/v1/access/authorize";
const USAGE_PATH: &str = "/v1/usage";

/// How to treat transport / 5xx failures when billing is enabled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureMode {
    /// Deny the scan if billing cannot be reached (default).
    FailClosed,
    /// Accept the scan if billing cannot be reached.
    FailOpen,
}

impl FailureMode {
    pub fn as_str(self) -> &'static str {
        match self {
            FailureMode::FailClosed => "fail-closed",
            FailureMode::FailOpen => "fail-open",
        }
    }

    fn parse(raw: &str) -> Result<Self, String> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "" | "fail-closed" | "fail_closed" | "closed" => Ok(FailureMode::FailClosed),
            "fail-open" | "fail_open" | "open" => Ok(FailureMode::FailOpen),
            other => Err(format!(
                "invalid PIPEFISH_BILLING_ON_ERROR '{other}' (expected fail-closed or fail-open)"
            )),
        }
    }
}

/// Env-driven billing settings. `enabled` is false unless the flag is on.
#[derive(Debug, Clone)]
pub struct BillingConfig {
    pub enabled: bool,
    pub base_url: String,
    pub api_key: String,
    pub on_error: FailureMode,
    pub timeout: Duration,
}

impl BillingConfig {
    /// Load from process env. Missing / false flag → disabled, no error.
    pub fn from_env() -> Result<Self, String> {
        Self::from_get(|k| std::env::var(k).ok())
    }

    pub fn from_get<F>(mut get: F) -> Result<Self, String>
    where
        F: FnMut(&str) -> Option<String>,
    {
        let enabled_raw = get("PIPEFISH_BILLING_ENABLED").unwrap_or_default();
        if !truthy(&enabled_raw) {
            return Ok(Self::disabled());
        }

        let api_key = get("PIPEFISH_API_KEY")
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| {
                "PIPEFISH_BILLING_ENABLED is set but PIPEFISH_API_KEY is empty".to_string()
            })?;

        let base_url = get("BILLING_PUBLIC_URL")
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| DEFAULT_BASE_URL.to_string());

        let on_error = FailureMode::parse(&get("PIPEFISH_BILLING_ON_ERROR").unwrap_or_default())?;

        let timeout = match get("BILLING_CLIENT_TIMEOUT") {
            Some(raw) if !raw.trim().is_empty() => parse_timeout(&raw)?,
            _ => DEFAULT_TIMEOUT,
        };

        Ok(Self {
            enabled: true,
            base_url,
            api_key,
            on_error,
            timeout,
        })
    }

    pub fn disabled() -> Self {
        Self {
            enabled: false,
            base_url: String::new(),
            api_key: String::new(),
            on_error: FailureMode::FailClosed,
            timeout: DEFAULT_TIMEOUT,
        }
    }
}

fn truthy(raw: &str) -> bool {
    matches!(
        raw.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

fn parse_timeout(raw: &str) -> Result<Duration, String> {
    let secs: f64 = raw.trim().parse().map_err(|_| {
        format!("invalid BILLING_CLIENT_TIMEOUT '{raw}' (expected seconds, e.g. 5 or 5.0)")
    })?;
    if secs <= 0.0 {
        return Err("BILLING_CLIENT_TIMEOUT must be positive".into());
    }
    Ok(Duration::from_secs_f64(secs))
}

/// Outcome of a scan that billing allowed (or skipped).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcceptOutcome {
    /// Billing unset/disabled — scan proceeds with no HTTP.
    Disabled,
    /// Authorize succeeded and usage was attempted.
    Authorized,
    /// Billing was unreachable and `fail-open` accepted the scan.
    FailOpen,
}

impl AcceptOutcome {
    pub fn as_str(self) -> &'static str {
        match self {
            AcceptOutcome::Disabled => "disabled",
            AcceptOutcome::Authorized => "authorized",
            AcceptOutcome::FailOpen => "fail-open",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum BillingError {
    #[error("{message}")]
    Denied { code: String, message: String },
    #[error("billing service unreachable: {0}")]
    Unreachable(String),
    #[error("billing misconfigured: {0}")]
    Config(String),
}

impl BillingError {
    pub fn quota(message: impl Into<String>) -> Self {
        Self::Denied {
            code: "quota_exceeded".into(),
            message: message.into(),
        }
    }

    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::Denied {
            code: "unauthorized".into(),
            message: message.into(),
        }
    }

    pub fn rate_limited(message: impl Into<String>) -> Self {
        Self::Denied {
            code: "rate_limited".into(),
            message: message.into(),
        }
    }

    pub fn code(&self) -> &str {
        match self {
            BillingError::Denied { code, .. } => code,
            BillingError::Unreachable(_) => "billing_unavailable",
            BillingError::Config(_) => "billing_misconfigured",
        }
    }

    pub fn http_status(&self) -> u16 {
        match self {
            BillingError::Denied { code, .. } if code == "unauthorized" => 401,
            BillingError::Denied { code, .. }
                if code == "quota_exceeded" || code == "rate_limited" =>
            {
                429
            }
            BillingError::Denied { .. } => 403,
            BillingError::Unreachable(_) => 503,
            BillingError::Config(_) => 500,
        }
    }
}

#[derive(Clone)]
pub struct BillingGate {
    config: BillingConfig,
    http: Option<reqwest::Client>,
}

impl BillingGate {
    pub fn disabled() -> Self {
        Self {
            config: BillingConfig::disabled(),
            http: None,
        }
    }

    pub fn from_env() -> anyhow::Result<Self> {
        let config = BillingConfig::from_env().map_err(anyhow::Error::msg)?;
        Self::from_config(config)
    }

    pub fn from_config(config: BillingConfig) -> anyhow::Result<Self> {
        if !config.enabled {
            return Ok(Self::disabled());
        }
        let http = reqwest::Client::builder()
            .timeout(config.timeout)
            .user_agent(concat!("pipefish-recon-kernel/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|e| anyhow::anyhow!("billing HTTP client: {e}"))?;
        Ok(Self {
            config,
            http: Some(http),
        })
    }

    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    pub fn failure_mode(&self) -> FailureMode {
        self.config.on_error
    }

    pub fn base_url(&self) -> &str {
        &self.config.base_url
    }

    /// Authorize one `recon_scan`, then record usage if allowed.
    ///
    /// Usage failures after a successful authorize are logged and do **not**
    /// reject the scan — entitlement was already granted.
    pub async fn accept_scan(&self, scan_id: Uuid) -> Result<AcceptOutcome, BillingError> {
        if !self.config.enabled {
            return Ok(AcceptOutcome::Disabled);
        }
        let http = self.http.as_ref().ok_or_else(|| {
            BillingError::Config("enabled billing is missing an HTTP client".into())
        })?;

        match self.authorize(http).await {
            Ok(()) => {
                if let Err(e) = self.record_usage(http, scan_id).await {
                    warn!(
                        error = %e,
                        scan_id = %scan_id,
                        "billing usage record failed after authorize"
                    );
                }
                Ok(AcceptOutcome::Authorized)
            }
            Err(BillingError::Unreachable(msg)) => match self.config.on_error {
                FailureMode::FailClosed => {
                    warn!(error = %msg, "billing unreachable; denying scan (fail-closed)");
                    Err(BillingError::Unreachable(msg))
                }
                FailureMode::FailOpen => {
                    warn!(error = %msg, "billing unreachable; accepting scan (fail-open)");
                    Ok(AcceptOutcome::FailOpen)
                }
            },
            Err(e) => Err(e),
        }
    }

    async fn authorize(&self, http: &reqwest::Client) -> Result<(), BillingError> {
        let url = join_url(&self.config.base_url, AUTHORIZE_PATH);
        let body = MeterRequest {
            channel: RECON_SCAN_CHANNEL,
            meter: RECON_SCAN_METER,
            quantity: 1,
            idempotency_key: None,
        };
        interpret_authorize(self.post_json(http, &url, &body).await?)
    }

    async fn record_usage(
        &self,
        http: &reqwest::Client,
        scan_id: Uuid,
    ) -> Result<(), BillingError> {
        let url = join_url(&self.config.base_url, USAGE_PATH);
        let body = MeterRequest {
            channel: RECON_SCAN_CHANNEL,
            meter: RECON_SCAN_METER,
            quantity: 1,
            idempotency_key: Some(format!("{RECON_SCAN_METER}:{scan_id}")),
        };
        let (status, _value) = self.post_json(http, &url, &body).await?;
        if status.is_success() {
            info!(scan_id = %scan_id, meter = RECON_SCAN_METER, "billing usage recorded");
            Ok(())
        } else {
            Err(BillingError::Unreachable(format!(
                "usage endpoint returned {}",
                status.as_u16()
            )))
        }
    }

    async fn post_json<T: Serialize>(
        &self,
        http: &reqwest::Client,
        url: &str,
        body: &T,
    ) -> Result<(StatusCode, serde_json::Value), BillingError> {
        let response = http
            .post(url)
            .bearer_auth(&self.config.api_key)
            .json(body)
            .send()
            .await
            .map_err(|e| BillingError::Unreachable(format!("request to {url} failed: {e}")))?;
        let status = response.status();
        let bytes = response.bytes().await.map_err(|e| {
            BillingError::Unreachable(format!("reading body from {url} failed: {e}"))
        })?;
        let value = if bytes.is_empty() {
            serde_json::Value::Null
        } else {
            serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null)
        };
        Ok((status, value))
    }
}

fn join_url(base: &str, path: &str) -> String {
    format!(
        "{}/{}",
        base.trim_end_matches('/'),
        path.trim_start_matches('/')
    )
}

#[derive(Debug, Serialize)]
struct MeterRequest {
    channel: &'static str,
    meter: &'static str,
    quantity: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    idempotency_key: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct ErrorBody {
    detail: Option<serde_json::Value>,
    meter: Option<serde_json::Value>,
    message: Option<String>,
}

fn detail_message(body: &ErrorBody, fallback: &str) -> String {
    match &body.detail {
        Some(serde_json::Value::String(s)) if !s.is_empty() => s.clone(),
        Some(other) if !other.is_null() => other.to_string(),
        _ => body
            .message
            .clone()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| fallback.to_string()),
    }
}

fn has_meter_field(body: &ErrorBody) -> bool {
    match &body.meter {
        Some(serde_json::Value::Null) => false,
        Some(serde_json::Value::String(s)) => !s.is_empty(),
        Some(_) => true,
        None => false,
    }
}

fn interpret_authorize(
    (status, value): (StatusCode, serde_json::Value),
) -> Result<(), BillingError> {
    if status.is_server_error() {
        return Err(BillingError::Unreachable(format!(
            "authorize returned {}",
            status.as_u16()
        )));
    }
    if status.is_success() {
        return Ok(());
    }

    let body: ErrorBody = serde_json::from_value(value).unwrap_or_default();

    if status == StatusCode::UNAUTHORIZED {
        return Err(BillingError::unauthorized(detail_message(
            &body,
            "billing API key was rejected",
        )));
    }

    if status == StatusCode::TOO_MANY_REQUESTS {
        if has_meter_field(&body) {
            return Err(BillingError::quota(detail_message(
                &body,
                "recon_scan quota exceeded",
            )));
        }
        return Err(BillingError::rate_limited(detail_message(
            &body,
            "billing rate limit exceeded",
        )));
    }

    let message = detail_message(
        &body,
        "recon_scan is not included in the current plan or quota",
    );
    let lower = message.to_ascii_lowercase();
    if lower.contains("quota") || lower.contains("limit") {
        return Err(BillingError::quota(message));
    }
    if lower.contains("plan") {
        return Err(BillingError::Denied {
            code: "plan_denied".into(),
            message,
        });
    }
    Err(BillingError::Denied {
        code: "entitlement_denied".into(),
        message,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::extract::State;
    use axum::http::{HeaderMap, StatusCode as AxumStatus};
    use axum::routing::post;
    use axum::{Json, Router};
    use serde_json::{json, Value};
    use std::sync::Arc;
    use tokio::net::TcpListener;
    use tokio::sync::Mutex;

    #[derive(Clone)]
    struct MockBilling {
        authorize: Arc<Mutex<Value>>,
        authorize_status: Arc<Mutex<u16>>,
        usage_status: Arc<Mutex<u16>>,
        captured: Arc<Mutex<Vec<(String, Option<String>, Value)>>>,
    }

    fn bearer(headers: &HeaderMap) -> Option<String> {
        headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .map(str::to_string)
    }

    async fn mock_authorize(
        State(state): State<MockBilling>,
        headers: HeaderMap,
        Json(body): Json<Value>,
    ) -> (AxumStatus, Json<Value>) {
        state
            .captured
            .lock()
            .await
            .push(("/v1/access/authorize".into(), bearer(&headers), body));
        let status = *state.authorize_status.lock().await;
        let body = state.authorize.lock().await.clone();
        (AxumStatus::from_u16(status).unwrap(), Json(body))
    }

    async fn mock_usage(
        State(state): State<MockBilling>,
        headers: HeaderMap,
        Json(body): Json<Value>,
    ) -> (AxumStatus, Json<Value>) {
        state
            .captured
            .lock()
            .await
            .push(("/v1/usage".into(), bearer(&headers), body));
        let status = *state.usage_status.lock().await;
        (
            AxumStatus::from_u16(status).unwrap(),
            Json(json!({"accepted": true})),
        )
    }

    async fn spawn_mock(state: MockBilling) -> String {
        let app = Router::new()
            .route("/v1/access/authorize", post(mock_authorize))
            .route("/v1/usage", post(mock_usage))
            .with_state(state);
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.ok();
        });
        format!("http://{addr}")
    }

    fn gate(base_url: &str, on_error: FailureMode) -> BillingGate {
        BillingGate::from_config(BillingConfig {
            enabled: true,
            base_url: base_url.into(),
            api_key: "test-api-key".into(),
            on_error,
            timeout: Duration::from_millis(500),
        })
        .unwrap()
    }

    fn mock_state(authorize: Value, authorize_status: u16) -> MockBilling {
        MockBilling {
            authorize: Arc::new(Mutex::new(authorize)),
            authorize_status: Arc::new(Mutex::new(authorize_status)),
            usage_status: Arc::new(Mutex::new(202)),
            captured: Arc::new(Mutex::new(Vec::new())),
        }
    }

    #[test]
    fn unset_env_disables_billing() {
        let cfg = BillingConfig::from_get(|_| None).unwrap();
        assert!(!cfg.enabled);
    }

    #[test]
    fn enabled_without_api_key_is_error() {
        let err = BillingConfig::from_get(|k| match k {
            "PIPEFISH_BILLING_ENABLED" => Some("true".into()),
            _ => None,
        })
        .unwrap_err();
        assert!(err.contains("PIPEFISH_API_KEY"));
    }

    #[test]
    fn enabled_defaults_url_and_timeout() {
        let cfg = BillingConfig::from_get(|k| match k {
            "PIPEFISH_BILLING_ENABLED" => Some("true".into()),
            "PIPEFISH_API_KEY" => Some("test-api-key".into()),
            _ => None,
        })
        .unwrap();
        assert!(cfg.enabled);
        assert_eq!(cfg.base_url, DEFAULT_BASE_URL);
        assert_eq!(cfg.timeout, DEFAULT_TIMEOUT);
        assert_eq!(cfg.on_error, FailureMode::FailClosed);
    }

    #[tokio::test]
    async fn disabled_gate_does_not_call_http() {
        let gate = BillingGate::disabled();
        let outcome = gate.accept_scan(Uuid::new_v4()).await.unwrap();
        assert_eq!(outcome, AcceptOutcome::Disabled);
    }

    #[tokio::test]
    async fn allowed_authorize_records_usage() {
        let state = mock_state(json!({"allowed": true}), 200);
        let captured = state.captured.clone();
        let base = spawn_mock(state).await;
        let scan_id = Uuid::new_v4();
        let outcome = gate(&base, FailureMode::FailClosed)
            .accept_scan(scan_id)
            .await
            .unwrap();
        assert_eq!(outcome, AcceptOutcome::Authorized);

        let calls = captured.lock().await.clone();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].0, "/v1/access/authorize");
        assert_eq!(calls[0].1.as_deref(), Some("Bearer test-api-key"));
        assert_eq!(calls[0].2["channel"], RECON_SCAN_CHANNEL);
        assert_eq!(calls[0].2["meter"], RECON_SCAN_METER);
        assert_eq!(calls[0].2["quantity"], 1);
        assert!(calls[0].2.get("account_id").is_none());
        assert!(calls[0].2.get("idempotency_key").is_none());
        assert_eq!(calls[1].0, "/v1/usage");
        assert_eq!(calls[1].1.as_deref(), Some("Bearer test-api-key"));
        assert_eq!(calls[1].2["channel"], RECON_SCAN_CHANNEL);
        assert_eq!(calls[1].2["meter"], RECON_SCAN_METER);
        assert_eq!(
            calls[1].2["idempotency_key"],
            format!("recon_scan:{scan_id}")
        );
    }

    #[tokio::test]
    async fn denied_quota_does_not_record_usage() {
        let state = mock_state(
            json!({
                "detail": "monthly recon_scan quota exhausted",
                "meter": "recon_scan"
            }),
            429,
        );
        let captured = state.captured.clone();
        let base = spawn_mock(state).await;
        let err = gate(&base, FailureMode::FailClosed)
            .accept_scan(Uuid::new_v4())
            .await
            .unwrap_err();
        assert_eq!(err.http_status(), 429);
        match &err {
            BillingError::Denied { code, message } => {
                assert_eq!(code, "quota_exceeded");
                assert!(message.contains("quota"));
            }
            other => panic!("expected denied, got {other:?}"),
        }
        let calls = captured.lock().await.clone();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "/v1/access/authorize");
    }

    #[tokio::test]
    async fn denied_rate_limit_without_meter() {
        let state = mock_state(json!({"detail": "too many requests"}), 429);
        let base = spawn_mock(state).await;
        let err = gate(&base, FailureMode::FailClosed)
            .accept_scan(Uuid::new_v4())
            .await
            .unwrap_err();
        assert_eq!(err.code(), "rate_limited");
        assert_eq!(err.http_status(), 429);
        assert!(format!("{err}").contains("too many requests"));
    }

    #[tokio::test]
    async fn unauthorized_is_not_fail_open() {
        let state = mock_state(json!({"detail": "invalid API key"}), 401);
        let base = spawn_mock(state).await;
        let err = gate(&base, FailureMode::FailOpen)
            .accept_scan(Uuid::new_v4())
            .await
            .unwrap_err();
        assert_eq!(err.code(), "unauthorized");
        assert_eq!(err.http_status(), 401);
    }

    #[tokio::test]
    async fn unreachable_fail_closed_denies() {
        let gate = gate("http://127.0.0.1:1", FailureMode::FailClosed);
        let err = gate.accept_scan(Uuid::new_v4()).await.unwrap_err();
        assert!(matches!(err, BillingError::Unreachable(_)));
        assert_eq!(err.code(), "billing_unavailable");
        assert_eq!(err.http_status(), 503);
    }

    #[tokio::test]
    async fn unreachable_fail_open_accepts_without_usage() {
        let gate = gate("http://127.0.0.1:1", FailureMode::FailOpen);
        let outcome = gate.accept_scan(Uuid::new_v4()).await.unwrap();
        assert_eq!(outcome, AcceptOutcome::FailOpen);
    }

    #[tokio::test]
    async fn authorize_5xx_is_unreachable() {
        let state = mock_state(json!({"detail": "boom"}), 503);
        let captured = state.captured.clone();
        let base = spawn_mock(state).await;
        let err = gate(&base, FailureMode::FailClosed)
            .accept_scan(Uuid::new_v4())
            .await
            .unwrap_err();
        assert!(matches!(err, BillingError::Unreachable(_)));
        assert_eq!(captured.lock().await.len(), 1);
    }
}
