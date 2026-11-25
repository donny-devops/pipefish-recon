//! Human feedback loop support.
//!
//! Logs structured records when a human approves, rejects, or edits an output
//! and provides simple retrieval of past corrected examples.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackRecord {
    pub request_id: String,
    pub agent: String,
    pub human_action: String, // "approved" | "rejected" | "edited"
    pub original_output: serde_json::Value,
    pub corrected_output: Option<serde_json::Value>,
    pub rating: Option<i32>,
    pub comment: Option<String>,
    pub timestamp: String,
}

fn feedback_path() -> PathBuf {
    PathBuf::from("feedback.jsonl")
}

pub fn record_feedback(record: &FeedbackRecord) -> Result<()> {
    let path = feedback_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).ok();
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .with_context(|| format!("failed to open {}", path.display()))?;
    writeln!(file, "{}", serde_json::to_string(record)?)?;
    Ok(())
}

pub fn load_recent_corrections(limit: usize) -> Result<Vec<FeedbackRecord>> {
    let path = feedback_path();
    if !path.exists() {
        return Ok(vec![]);
    }
    let file = fs::File::open(&path)?;
    let reader = BufReader::new(file);
    let mut corrections = Vec::new();
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(rec) = serde_json::from_str::<FeedbackRecord>(&line) {
            if rec.human_action == "edited" && rec.corrected_output.is_some() {
                corrections.push(rec);
            }
        }
    }
    let start = corrections.len().saturating_sub(limit);
    Ok(corrections.split_off(start))
}

pub fn correction_rate() -> Result<(usize, usize, f64)> {
    let path = feedback_path();
    if !path.exists() {
        return Ok((0, 0, 0.0));
    }
    let file = fs::File::open(&path)?;
    let reader = BufReader::new(file);
    let mut total = 0usize;
    let mut edited = 0usize;
    for line in reader.lines() {
        let line = line?;
        if let Ok(rec) = serde_json::from_str::<FeedbackRecord>(&line) {
            total += 1;
            if rec.human_action == "edited" {
                edited += 1;
            }
        }
    }
    let rate = if total == 0 { 0.0 } else { edited as f64 / total as f64 };
    Ok((total, edited, rate))
}
