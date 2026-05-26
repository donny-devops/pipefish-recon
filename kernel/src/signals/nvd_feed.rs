//! NVD CVE 2.0 API poller.
//!
//! Polls <https://services.nvd.nist.gov/rest/json/cves/2.0> for recently
//! modified CVEs and normalizes each item into a [`ThreatSignal`].

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Duration, Utc};
use serde_json::Value;
use tracing::{debug, warn};
use uuid::Uuid;

use crate::events::Severity;
use crate::signals::threat_signal::{SignalSource, SignalType, ThreatSignal};

const NVD_API_URL: &str = "https://services.nvd.nist.gov/rest/json/cves/2.0";
const DEFAULT_POLL_INTERVAL_SECS: u64 = 3600;
const DEFAULT_RESULTS_PER_PAGE: u32 = 100;
const UNAUTH_REQUEST_DELAY_SECS: u64 = 6;

pub struct NvdFeedPoller {
    client: reqwest::Client,
    api_key: Option<String>,
    poll_interval_secs: u64,
    last_modified: Option<DateTime<Utc>>,
}

impl NvdFeedPoller {
    pub fn new() -> Self {
        let api_key = std::env::var("NVD_API_KEY").ok().filter(|s| !s.is_empty());
        let poll_interval_secs = std::env::var("NVD_POLL_INTERVAL_SECS")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(DEFAULT_POLL_INTERVAL_SECS);

        let client = reqwest::Client::builder()
            .user_agent("skynet-kernel/0.1 (+https://skynet)")
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            client,
            api_key,
            poll_interval_secs,
            last_modified: None,
        }
    }

    pub fn poll_interval_secs(&self) -> u64 {
        self.poll_interval_secs
    }

    /// Poll the NVD CVE 2.0 API for CVEs modified since the last successful
    /// poll. On first invocation, looks back over the past 2 hours.
    pub async fn poll(&mut self) -> Result<Vec<ThreatSignal>> {
        let end = Utc::now();
        let start = self
            .last_modified
            .unwrap_or_else(|| end - Duration::hours(2));

        let mut req = self
            .client
            .get(NVD_API_URL)
            .query(&[
                ("lastModStartDate", start.to_rfc3339()),
                ("lastModEndDate", end.to_rfc3339()),
                ("resultsPerPage", DEFAULT_RESULTS_PER_PAGE.to_string()),
            ]);

        if let Some(key) = &self.api_key {
            req = req.header("apiKey", key);
        } else {
            tokio::time::sleep(std::time::Duration::from_secs(UNAUTH_REQUEST_DELAY_SECS))
                .await;
        }

        let resp = req
            .send()
            .await
            .context("NVD request failed")?
            .error_for_status()
            .context("NVD response status")?;

        let body: Value = resp.json().await.context("NVD response parse")?;

        let mut signals = Vec::new();
        let vulns = body
            .get("vulnerabilities")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        for item in vulns {
            let cve = match item.get("cve") {
                Some(c) => c,
                None => continue,
            };
            match Self::parse_cve(cve) {
                Ok(sig) => signals.push(sig),
                Err(e) => {
                    debug!(error = %e, "skipping malformed NVD CVE item");
                }
            }
        }

        self.last_modified = Some(end);
        Ok(signals)
    }

    /// Parse a single NVD CVE 2.0 `cve` object into a [`ThreatSignal`].
    fn parse_cve(cve: &Value) -> Result<ThreatSignal> {
        let cve_id = cve
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("NVD CVE missing id"))?
            .to_string();

        let description = cve
            .get("descriptions")
            .and_then(|v| v.as_array())
            .and_then(|arr| {
                arr.iter()
                    .find(|d| d.get("lang").and_then(|l| l.as_str()) == Some("en"))
                    .or_else(|| arr.first())
            })
            .and_then(|d| d.get("value").and_then(|v| v.as_str()))
            .unwrap_or("")
            .to_string();

        let (cvss_score, severity) = extract_cvss(cve);

        let affected_products = extract_affected_products(cve);

        let ingested_at = Utc::now();

        Ok(ThreatSignal {
            id: Uuid::new_v4(),
            ingested_at,
            source: SignalSource::NvdCve,
            signal_type: SignalType::Vulnerability,
            severity,
            title: cve_id.clone(),
            description,
            cve_id: Some(cve_id),
            cvss_score,
            affected_products,
            iocs: Vec::new(),
            raw: cve.clone(),
        })
    }
}

impl Default for NvdFeedPoller {
    fn default() -> Self {
        Self::new()
    }
}

fn extract_cvss(cve: &Value) -> (Option<f32>, Severity) {
    let metrics = match cve.get("metrics") {
        Some(m) => m,
        None => return (None, Severity::Info),
    };

    for key in ["cvssMetricV31", "cvssMetricV30", "cvssMetricV2"] {
        if let Some(arr) = metrics.get(key).and_then(|v| v.as_array()) {
            if let Some(first) = arr.first() {
                let data = first.get("cvssData");
                let score = data
                    .and_then(|d| d.get("baseScore"))
                    .and_then(|v| v.as_f64())
                    .map(|f| f as f32);
                let severity_str = data
                    .and_then(|d| d.get("baseSeverity"))
                    .and_then(|v| v.as_str());
                let severity = map_severity(severity_str, score);
                return (score, severity);
            }
        }
    }

    (None, Severity::Info)
}

fn map_severity(base_severity: Option<&str>, score: Option<f32>) -> Severity {
    if let Some(s) = base_severity {
        return match s.to_ascii_uppercase().as_str() {
            "CRITICAL" => Severity::Critical,
            "HIGH" => Severity::High,
            "MEDIUM" => Severity::Medium,
            "LOW" => Severity::Low,
            _ => Severity::Info,
        };
    }
    match score {
        Some(s) if s >= 9.0 => Severity::Critical,
        Some(s) if s >= 7.0 => Severity::High,
        Some(s) if s >= 4.0 => Severity::Medium,
        Some(s) if s > 0.0 => Severity::Low,
        _ => Severity::Info,
    }
}

fn extract_affected_products(cve: &Value) -> Vec<String> {
    let mut products = Vec::new();
    let configurations = match cve.get("configurations").and_then(|v| v.as_array()) {
        Some(c) => c,
        None => return products,
    };

    for config in configurations {
        let nodes = match config.get("nodes").and_then(|v| v.as_array()) {
            Some(n) => n,
            None => continue,
        };
        for node in nodes {
            if let Some(matches) = node.get("cpeMatch").and_then(|v| v.as_array()) {
                for m in matches {
                    if let Some(cpe) = m.get("criteria").and_then(|v| v.as_str()) {
                        if !products.iter().any(|p: &String| p == cpe) {
                            products.push(cpe.to_string());
                        }
                    }
                }
            }
        }
    }

    if products.len() > 64 {
        warn!(count = products.len(), "truncating affected_products to 64");
        products.truncate(64);
    }
    products
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_cve(severity: &str, score: f64) -> Value {
        serde_json::json!({
            "id": "CVE-2026-12345",
            "descriptions": [
                {"lang": "en", "value": "A critical RCE in example-server"}
            ],
            "metrics": {
                "cvssMetricV31": [{
                    "cvssData": {
                        "baseScore": score,
                        "baseSeverity": severity
                    }
                }]
            },
            "configurations": [{
                "nodes": [{
                    "cpeMatch": [
                        {"criteria": "cpe:2.3:a:example:server:1.0:*:*:*:*:*:*:*"}
                    ]
                }]
            }],
            "published": "2026-05-01T00:00:00.000",
            "lastModified": "2026-05-02T00:00:00.000"
        })
    }

    #[test]
    fn parse_cve_maps_critical_severity() {
        let cve = sample_cve("CRITICAL", 9.8);
        let signal = NvdFeedPoller::parse_cve(&cve).expect("parses");
        assert!(matches!(signal.severity, Severity::Critical));
    }

    #[test]
    fn parse_cve_extracts_core_fields() {
        let cve = sample_cve("HIGH", 7.5);
        let signal = NvdFeedPoller::parse_cve(&cve).expect("parses");
        assert_eq!(signal.cve_id.as_deref(), Some("CVE-2026-12345"));
        assert_eq!(signal.cvss_score, Some(7.5));
        assert!(signal.description.contains("example-server"));
        assert_eq!(
            signal.affected_products.first().map(String::as_str),
            Some("cpe:2.3:a:example:server:1.0:*:*:*:*:*:*:*")
        );
        assert!(matches!(signal.severity, Severity::High));
        assert!(matches!(
            signal.source,
            crate::signals::threat_signal::SignalSource::NvdCve
        ));
    }

    #[test]
    fn parse_cve_maps_all_severity_buckets() {
        let cases = [
            ("CRITICAL", 9.8, Severity::Critical),
            ("HIGH", 7.5, Severity::High),
            ("MEDIUM", 5.0, Severity::Medium),
            ("LOW", 2.0, Severity::Low),
            ("NONE", 0.0, Severity::Info),
        ];
        for (label, score, expected) in cases {
            let cve = sample_cve(label, score);
            let signal = NvdFeedPoller::parse_cve(&cve).expect("parses");
            assert_eq!(
                std::mem::discriminant(&signal.severity),
                std::mem::discriminant(&expected),
                "severity mismatch for {}",
                label
            );
        }
    }

    #[test]
    fn parse_cve_missing_id_errors() {
        let cve = serde_json::json!({"descriptions": []});
        assert!(NvdFeedPoller::parse_cve(&cve).is_err());
    }

    #[test]
    fn new_initialises_with_defaults() {
        // Ensure env doesn't leak in if a previous test set it.
        std::env::remove_var("NVD_API_KEY");
        std::env::remove_var("NVD_POLL_INTERVAL_SECS");
        let poller = NvdFeedPoller::new();
        assert_eq!(poller.poll_interval_secs(), DEFAULT_POLL_INTERVAL_SECS);
        assert!(poller.api_key.is_none());
        assert!(poller.last_modified.is_none());
    }
}
