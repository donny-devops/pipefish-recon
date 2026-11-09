//! Deterministic severity classification and routing (RECON-A3).
//!
//! No LLM on the hot path. Severity comes from CVSS plus exploit/KEV
//! availability; routing follows POLICY.md §1 escalation gates.

use crate::events::Severity;
use crate::signals::ThreatSignal;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Route {
    A4Autonomous,
    A4PolicyGate,
    HilAndA4DryRun,
}

impl Route {
    pub fn as_str(&self) -> &'static str {
        match self {
            Route::A4Autonomous => "A4 autonomous",
            Route::A4PolicyGate => "A4 with policy gate",
            Route::HilAndA4DryRun => "HIL escalation queue",
        }
    }
}

impl Severity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Severity::Critical => "CRITICAL",
            Severity::High => "HIGH",
            Severity::Medium => "MEDIUM",
            Severity::Low => "LOW",
            Severity::Info => "INFO",
        }
    }
}

/// Classify from impact (CVSS) × exploit-availability (KEV / exploit flag).
pub fn classify(signal: &ThreatSignal) -> Severity {
    let mut sev = match signal.cvss.unwrap_or(0.0) {
        x if x >= 9.0 => Severity::Critical,
        x if x >= 7.0 => Severity::High,
        x if x >= 4.0 => Severity::Medium,
        x if x > 0.0 => Severity::Low,
        _ => Severity::Info,
    };
    if signal.exploit_available || signal.kev {
        sev = bump(sev);
    }
    sev
}

fn bump(sev: Severity) -> Severity {
    match sev {
        Severity::Info => Severity::Low,
        Severity::Low => Severity::Medium,
        Severity::Medium => Severity::High,
        Severity::High => Severity::Critical,
        Severity::Critical => Severity::Critical,
    }
}

pub fn route_for(severity: Severity) -> Route {
    match severity {
        Severity::Critical => Route::HilAndA4DryRun,
        Severity::High => Route::A4PolicyGate,
        Severity::Medium | Severity::Low | Severity::Info => Route::A4Autonomous,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signals::{SignalSource, ThreatSignal};
    use chrono::Utc;
    use uuid::Uuid;

    fn sig(cvss: Option<f32>, kev: bool) -> ThreatSignal {
        ThreatSignal {
            id: Uuid::new_v4(),
            cve_id: "CVE-2026-1337".into(),
            source: SignalSource::Nvd,
            cvss,
            summary: "test".into(),
            published: None,
            ingested_at: Utc::now(),
            exploit_available: kev,
            kev,
        }
    }

    #[test]
    fn cvss_buckets() {
        assert_eq!(classify(&sig(Some(9.8), false)), Severity::Critical);
        assert_eq!(classify(&sig(Some(7.5), false)), Severity::High);
        assert_eq!(classify(&sig(Some(5.0), false)), Severity::Medium);
        assert_eq!(classify(&sig(Some(2.1), false)), Severity::Low);
        assert_eq!(classify(&sig(None, false)), Severity::Info);
    }

    #[test]
    fn kev_bumps_one_level() {
        assert_eq!(classify(&sig(Some(7.5), true)), Severity::Critical);
        assert_eq!(classify(&sig(Some(5.0), true)), Severity::High);
        assert_eq!(classify(&sig(Some(2.1), true)), Severity::Medium);
    }

    #[test]
    fn policy_routes() {
        assert_eq!(route_for(Severity::Critical), Route::HilAndA4DryRun);
        assert_eq!(route_for(Severity::High), Route::A4PolicyGate);
        assert_eq!(route_for(Severity::Medium), Route::A4Autonomous);
        assert_eq!(route_for(Severity::Low), Route::A4Autonomous);
    }
}
