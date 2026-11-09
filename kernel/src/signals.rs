//! Canonical `ThreatSignal` schema and NVD CVE 2.0 normalization.

use std::collections::HashMap;

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum SignalSource {
    Nvd,
    Webhook,
    CveFeed,
}

impl SignalSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            SignalSource::Nvd => "nvd",
            SignalSource::Webhook => "webhook",
            SignalSource::CveFeed => "cve-feed",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ThreatSignal {
    pub id: Uuid,
    pub cve_id: String,
    pub source: SignalSource,
    pub cvss: Option<f32>,
    pub summary: String,
    pub published: Option<DateTime<Utc>>,
    pub ingested_at: DateTime<Utc>,
    pub exploit_available: bool,
    pub kev: bool,
}

impl ThreatSignal {
    pub fn from_ingest(req: IngestRequest) -> Self {
        Self {
            id: Uuid::new_v4(),
            cve_id: req.cve_id.trim().to_uppercase(),
            source: req.source.unwrap_or(SignalSource::Webhook),
            cvss: req.cvss,
            summary: req.summary.unwrap_or_default(),
            published: req.published,
            ingested_at: Utc::now(),
            exploit_available: req.exploit_available.unwrap_or(false),
            kev: req.kev.unwrap_or(false),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IngestRequest {
    pub cve_id: String,
    pub cvss: Option<f32>,
    pub summary: Option<String>,
    pub source: Option<SignalSource>,
    pub published: Option<DateTime<Utc>>,
    pub exploit_available: Option<bool>,
    pub kev: Option<bool>,
}

/// 30-day rolling de-duplication window keyed by CVE id (SOUL.md A1).
#[derive(Debug, Default)]
pub struct Deduper {
    seen: HashMap<String, DateTime<Utc>>,
}

impl Deduper {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns true if this CVE is new (or older than 30 days) and records it.
    pub fn accept(&mut self, cve_id: &str, now: DateTime<Utc>) -> bool {
        let key = cve_id.trim().to_uppercase();
        let window = Duration::days(30);
        if let Some(prev) = self.seen.get(&key) {
            if now.signed_duration_since(*prev) < window {
                return false;
            }
        }
        self.seen.insert(key, now);
        true
    }
}

#[derive(Debug, Deserialize)]
struct NvdFeed {
    #[serde(default)]
    vulnerabilities: Vec<NvdItem>,
}

#[derive(Debug, Deserialize)]
struct NvdItem {
    cve: NvdCve,
}

#[derive(Debug, Deserialize)]
struct NvdCve {
    id: String,
    published: Option<String>,
    #[serde(default)]
    descriptions: Vec<NvdDesc>,
    metrics: Option<NvdMetrics>,
    #[serde(rename = "cisaExploitAdd")]
    cisa_exploit_add: Option<String>,
}

#[derive(Debug, Deserialize)]
struct NvdDesc {
    lang: Option<String>,
    value: Option<String>,
}

#[derive(Debug, Deserialize)]
struct NvdMetrics {
    #[serde(rename = "cvssMetricV40")]
    v40: Option<Vec<NvdCvssMetric>>,
    #[serde(rename = "cvssMetricV31")]
    v31: Option<Vec<NvdCvssMetric>>,
    #[serde(rename = "cvssMetricV30")]
    v30: Option<Vec<NvdCvssMetric>>,
    #[serde(rename = "cvssMetricV2")]
    v2: Option<Vec<NvdCvssMetric>>,
}

#[derive(Debug, Deserialize)]
struct NvdCvssMetric {
    #[serde(rename = "cvssData")]
    cvss_data: Option<NvdCvssData>,
}

#[derive(Debug, Deserialize)]
struct NvdCvssData {
    #[serde(rename = "baseScore")]
    base_score: Option<f32>,
}

pub fn parse_nvd_cve_2_0(json: &str) -> anyhow::Result<Vec<ThreatSignal>> {
    let feed: NvdFeed = serde_json::from_str(json)?;
    let now = Utc::now();
    Ok(feed
        .vulnerabilities
        .into_iter()
        .map(|item| nvd_item_to_signal(item, now))
        .collect())
}

fn nvd_item_to_signal(item: NvdItem, now: DateTime<Utc>) -> ThreatSignal {
    let cve = item.cve;
    let summary = cve
        .descriptions
        .iter()
        .find(|d| d.lang.as_deref() == Some("en"))
        .or(cve.descriptions.first())
        .and_then(|d| d.value.clone())
        .unwrap_or_default();
    let cvss = cve.metrics.as_ref().and_then(extract_cvss);
    let published = cve
        .published
        .as_deref()
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|d| d.with_timezone(&Utc));
    let kev = cve.cisa_exploit_add.is_some();
    ThreatSignal {
        id: Uuid::new_v4(),
        cve_id: cve.id.trim().to_uppercase(),
        source: SignalSource::Nvd,
        cvss,
        summary,
        published,
        ingested_at: now,
        exploit_available: kev,
        kev,
    }
}

fn extract_cvss(metrics: &NvdMetrics) -> Option<f32> {
    for group in [&metrics.v40, &metrics.v31, &metrics.v30, &metrics.v2] {
        if let Some(score) = group
            .as_ref()
            .and_then(|m| m.first())
            .and_then(|m| m.cvss_data.as_ref())
            .and_then(|d| d.base_score)
        {
            return Some(score);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = r#"{
      "vulnerabilities": [
        {
          "cve": {
            "id": "CVE-2026-1337",
            "published": "2026-01-15T00:00:00.000Z",
            "descriptions": [
              {"lang": "en", "value": "Remote code execution in example daemon"}
            ],
            "metrics": {
              "cvssMetricV31": [
                {"cvssData": {"baseScore": 9.8}}
              ]
            },
            "cisaExploitAdd": "2026-01-16"
          }
        },
        {
          "cve": {
            "id": "CVE-2026-0001",
            "descriptions": [{"lang": "en", "value": "low impact"}],
            "metrics": {
              "cvssMetricV31": [{"cvssData": {"baseScore": 3.1}}]
            }
          }
        }
      ]
    }"#;

    #[test]
    fn parses_nvd_cve_2_0_fixture() {
        let signals = parse_nvd_cve_2_0(FIXTURE).expect("parse");
        assert_eq!(signals.len(), 2);
        assert_eq!(signals[0].cve_id, "CVE-2026-1337");
        assert_eq!(signals[0].cvss, Some(9.8));
        assert!(signals[0].kev);
        assert!(signals[0].exploit_available);
        assert!(signals[0].summary.contains("Remote code execution"));
        assert_eq!(signals[1].cve_id, "CVE-2026-0001");
        assert_eq!(signals[1].cvss, Some(3.1));
        assert!(!signals[1].kev);
    }

    #[test]
    fn deduper_30_day_window() {
        let mut d = Deduper::new();
        let t0 = Utc::now();
        assert!(d.accept("CVE-2026-1337", t0));
        assert!(!d.accept("CVE-2026-1337", t0 + Duration::days(1)));
        assert!(d.accept("CVE-2026-1337", t0 + Duration::days(31)));
        assert!(d.accept("CVE-2026-0001", t0));
        assert!(!d.accept("cve-2026-0001", t0));
    }
}
