//! Operator HTTP surface — localhost JSON API for the dashboard.
//!
//! Serves public status, the recent bus ring, and the loaded ACL matrix.
//! Private keys never appear on this API.

use std::collections::{HashMap, VecDeque};
use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::{Query, State};
use axum::http::{header, HeaderValue, Method, StatusCode};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use tokio::sync::{mpsc, Mutex};
use tower_http::cors::CorsLayer;
use uuid::Uuid;

use crate::billing::BillingGate;
use crate::events::{BusEvent, CommitType, ContextId, ContextScope};
use crate::mcp::policy::ToolPolicy;

pub const DEFAULT_BIND: &str = "127.0.0.1:8080";
pub const RING_CAPACITY: usize = 256;

#[derive(Clone)]
pub struct OperatorState {
    ring: Arc<Mutex<VecDeque<BusEvent>>>,
    policy: Arc<ToolPolicy>,
    capacity: usize,
    billing: BillingGate,
    bus_tx: Option<mpsc::Sender<BusEvent>>,
}

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
}

#[derive(Debug, Serialize)]
pub struct ComponentStatus {
    pub id: String,
    pub kind: &'static str,
    pub online: bool,
    pub last_event_at: Option<chrono::DateTime<chrono::Utc>>,
    pub last_commit: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct StatusResponse {
    pub kernel: &'static str,
    pub contexts: Vec<ComponentStatus>,
    pub agents: Vec<ComponentStatus>,
}

#[derive(Debug, Deserialize)]
pub struct EventsQuery {
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize, Default)]
pub struct ScanRequest {
    pub target: Option<String>,
}

impl OperatorState {
    pub fn new(policy: Arc<ToolPolicy>, capacity: usize) -> Self {
        Self {
            ring: Arc::new(Mutex::new(VecDeque::with_capacity(capacity))),
            policy,
            capacity,
            billing: BillingGate::disabled(),
            bus_tx: None,
        }
    }

    pub fn with_billing(mut self, billing: BillingGate) -> Self {
        self.billing = billing;
        self
    }

    pub fn with_bus(mut self, tx: mpsc::Sender<BusEvent>) -> Self {
        self.bus_tx = Some(tx);
        self
    }

    pub async fn push(&self, evt: BusEvent) {
        let mut ring = self.ring.lock().await;
        if ring.len() >= self.capacity {
            ring.pop_front();
        }
        ring.push_back(evt);
    }

    pub async fn events(&self, limit: usize) -> Vec<BusEvent> {
        let ring = self.ring.lock().await;
        let n = limit.min(ring.len());
        ring.iter()
            .rev()
            .take(n)
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect()
    }

    pub fn policy(&self) -> &ToolPolicy {
        &self.policy
    }

    pub async fn status(&self) -> StatusResponse {
        let ring = self.ring.lock().await;
        let mut last: HashMap<ContextId, &BusEvent> = HashMap::new();
        for evt in ring.iter() {
            last.insert(evt.source, evt);
        }
        let mut contexts = Vec::new();
        let mut agents = Vec::new();
        for id in ContextId::all_runtime() {
            let seen = last.get(id);
            let row = ComponentStatus {
                id: id.as_str().to_string(),
                kind: if id.is_agent() { "agent" } else { "context" },
                online: seen.is_some(),
                last_event_at: seen.map(|e| e.timestamp),
                last_commit: seen.map(|e| e.to_commit_string()),
            };
            if id.is_agent() {
                agents.push(row);
            } else {
                contexts.push(row);
            }
        }
        StatusResponse {
            kernel: "online",
            contexts,
            agents,
        }
    }
}

pub fn router(state: OperatorState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin([
            HeaderValue::from_static("http://localhost:3000"),
            HeaderValue::from_static("http://127.0.0.1:3000"),
        ])
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([header::CONTENT_TYPE]);

    Router::new()
        .route("/health", get(health))
        .route("/api/status", get(status))
        .route("/api/events", get(events))
        .route("/api/policy", get(policy))
        .route("/api/scans", post(start_scan))
        .layer(cors)
        .with_state(state)
}

pub async fn serve(bind: &str, state: OperatorState) -> anyhow::Result<()> {
    let addr: SocketAddr = bind.parse()?;
    let listener = TcpListener::bind(addr).await?;
    tracing::info!(%addr, "operator HTTP listening");
    axum::serve(listener, router(state)).await?;
    Ok(())
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

async fn status(State(state): State<OperatorState>) -> Json<StatusResponse> {
    Json(state.status().await)
}

async fn events(
    State(state): State<OperatorState>,
    Query(q): Query<EventsQuery>,
) -> Json<Vec<BusEvent>> {
    let limit = q.limit.unwrap_or(50).min(RING_CAPACITY);
    Json(state.events(limit).await)
}

async fn policy(State(state): State<OperatorState>) -> Json<crate::mcp::policy::PolicyMatrix> {
    Json(state.policy().matrix())
}

async fn start_scan(
    State(state): State<OperatorState>,
    body: Result<Json<ScanRequest>, axum::extract::rejection::JsonRejection>,
) -> (StatusCode, Json<serde_json::Value>) {
    let req = match body {
        Ok(Json(req)) => req,
        Err(_) => ScanRequest::default(),
    };
    let scan_id = Uuid::new_v4();
    match state.billing.accept_scan(scan_id).await {
        Ok(outcome) => {
            if let Some(tx) = &state.bus_tx {
                let description = match req
                    .target
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                {
                    Some(target) => format!("recon scan accepted for {target}"),
                    None => "recon scan accepted".to_string(),
                };
                let evt = BusEvent::new(
                    ContextId::Dx,
                    CommitType::Feat,
                    ContextScope::Dx,
                    description,
                )
                .with_payload(serde_json::json!({
                    "scan_id": scan_id,
                    "target": req.target,
                    "billing": outcome.as_str(),
                }));
                let _ = tx.try_send(evt);
            }
            (
                StatusCode::ACCEPTED,
                Json(serde_json::json!({
                    "accepted": true,
                    "scan_id": scan_id,
                    "billing": outcome.as_str(),
                })),
            )
        }
        Err(err) => {
            let status = StatusCode::from_u16(err.http_status()).unwrap_or(StatusCode::FORBIDDEN);
            let message = err.to_string();
            let code = err.code();
            (
                status,
                Json(serde_json::json!({
                    "accepted": false,
                    "error": code,
                    "message": message,
                })),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::{CommitType, ContextScope};
    use axum::body::Body;
    use axum::http::Request;
    use http_body_util::BodyExt;
    use tower::ServiceExt as _;

    async fn json_body(res: axum::response::Response) -> serde_json::Value {
        let bytes = res.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn health_ok() {
        let state = OperatorState::new(Arc::new(ToolPolicy::new()), 16);
        let app = router(state);
        let res = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let v = json_body(res).await;
        assert_eq!(v["status"], "ok");
    }

    #[tokio::test]
    async fn events_and_status_and_policy() {
        let mut policy = ToolPolicy::new();
        policy.allow("recon-a1", "cve-feed");
        let state = OperatorState::new(Arc::new(policy), 16);
        state
            .push(BusEvent::new(
                ContextId::ReconA1,
                CommitType::Feat,
                ContextScope::Bus,
                "RECON-A1 online",
            ))
            .await;

        let app = router(state.clone());
        let res = app
            .oneshot(
                Request::builder()
                    .uri("/api/events?limit=10")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let events = json_body(res).await;
        assert_eq!(events.as_array().unwrap().len(), 1);
        assert_eq!(events[0]["source"], "recon-a1");

        let app = router(state.clone());
        let res = app
            .oneshot(
                Request::builder()
                    .uri("/api/status")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = json_body(res).await;
        assert_eq!(status["kernel"], "online");
        let agents = status["agents"].as_array().unwrap();
        let a1 = agents.iter().find(|a| a["id"] == "recon-a1").unwrap();
        assert_eq!(a1["online"], true);

        let app = router(state);
        let res = app
            .oneshot(
                Request::builder()
                    .uri("/api/policy")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let matrix = json_body(res).await;
        assert!(matrix["allow"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p[0] == "recon-a1" && p[1] == "cve-feed"));
    }

    async fn post_scan(state: OperatorState) -> (StatusCode, serde_json::Value) {
        let app = router(state);
        let res = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/scans")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();
        (res.status(), json_body(res).await)
    }

    #[tokio::test]
    async fn scan_accepted_when_billing_disabled() {
        let state = OperatorState::new(Arc::new(ToolPolicy::new()), 16);
        let (status, v) = post_scan(state).await;
        assert_eq!(status, StatusCode::ACCEPTED);
        assert_eq!(v["accepted"], true);
        assert_eq!(v["billing"], "disabled");
        assert!(v["scan_id"].as_str().is_some());
    }

    #[tokio::test]
    async fn scan_unavailable_when_billing_unreachable_fail_closed() {
        let billing = crate::billing::BillingGate::from_config(crate::billing::BillingConfig {
            enabled: true,
            base_url: "http://127.0.0.1:1".into(),
            api_key: "test-api-key".into(),
            on_error: crate::billing::FailureMode::FailClosed,
            timeout: std::time::Duration::from_millis(200),
        })
        .unwrap();
        let state = OperatorState::new(Arc::new(ToolPolicy::new()), 16).with_billing(billing);
        let (status, v) = post_scan(state).await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(v["accepted"], false);
        assert_eq!(v["error"], "billing_unavailable");
    }
}
