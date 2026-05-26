//! Canonical `ThreatSignal` schema — the normalized form of every inbound
//! threat event consumed by SKYNET-A1.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::events::Severity;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatSignal {
    pub id: Uuid,
    pub ingested_at: DateTime<Utc>,
    pub source: SignalSource,
    pub signal_type: SignalType,
    pub severity: Severity,
    pub title: String,
    pub description: String,
    pub cve_id: Option<String>,
    pub cvss_score: Option<f32>,
    pub affected_products: Vec<String>,
    pub iocs: Vec<Ioc>,
    pub raw: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SignalSource {
    NvdCve,
    OsintFeed,
    SiemWebhook,
    Honeypot,
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SignalType {
    Vulnerability,
    Exploit,
    Malware,
    Phishing,
    Reconnaissance,
    PolicyViolation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ioc {
    pub ioc_type: IocType,
    pub value: String,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum IocType {
    IpAddress,
    Domain,
    FileHash,
    Url,
    Email,
}
