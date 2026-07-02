//! Telemetry: the "flywheel" data capture layer.
//!
//! Emits events to Grafana (metrics), Sentry (errors), and PostHog (product
//! analytics). Real impls push to those SaaS endpoints; the mock records events
//! in memory and writes them to sample log files for inspection.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningEvent {
    pub profile_id: String,
    pub concept_id: String,
    pub item_id: String,
    pub rating: i16,
    /// Composite success signal used to weight input material.
    pub success_weight: f64,
    pub timestamp: String,
}

#[async_trait]
pub trait TelemetrySink: Send + Sync {
    async fn capture_learning(&self, ev: LearningEvent);
    async fn capture_error(&self, message: &str);
    /// Increment a named metric (Grafana/Prometheus style).
    async fn metric(&self, name: &str, value: f64);
}

/// In-memory mock sink; useful for asserting the flywheel captured events.
#[derive(Default)]
pub struct MockTelemetry {
    pub learning: Mutex<Vec<LearningEvent>>,
    pub errors: Mutex<Vec<String>>,
    pub metrics: Mutex<Vec<(String, f64)>>,
}

#[async_trait]
impl TelemetrySink for MockTelemetry {
    async fn capture_learning(&self, ev: LearningEvent) {
        self.learning.lock().unwrap().push(ev);
    }
    async fn capture_error(&self, message: &str) {
        self.errors.lock().unwrap().push(message.to_string());
    }
    async fn metric(&self, name: &str, value: f64) {
        self.metrics.lock().unwrap().push((name.to_string(), value));
    }
}

/// File-backed sink that appends PostHog-style JSONL events to disk. Used by the
/// running container so the flywheel's captured data is inspectable. Falls back
/// silently if the path is not writable (never breaks the request path).
pub struct FileTelemetry {
    path: std::path::PathBuf,
}

impl FileTelemetry {
    pub fn new(path: impl Into<std::path::PathBuf>) -> Self {
        FileTelemetry { path: path.into() }
    }

    fn append(&self, line: &str) {
        use std::io::Write;
        if let Some(parent) = self.path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&self.path) {
            let _ = writeln!(f, "{line}");
        }
    }
}

#[async_trait]
impl TelemetrySink for FileTelemetry {
    async fn capture_learning(&self, ev: LearningEvent) {
        if let Ok(json) = serde_json::to_string(&serde_json::json!({
            "event": "card_reviewed",
            "distinct_id": ev.profile_id,
            "timestamp": ev.timestamp,
            "properties": {
                "concept_id": ev.concept_id,
                "item_id": ev.item_id,
                "rating": ev.rating,
                "success_weight": ev.success_weight,
            }
        })) {
            self.append(&json);
        }
    }
    async fn capture_error(&self, message: &str) {
        self.append(&format!("{{\"event\":\"error\",\"message\":{:?}}}", message));
    }
    async fn metric(&self, name: &str, value: f64) {
        self.append(&format!("{{\"event\":\"metric\",\"name\":{:?},\"value\":{}}}", name, value));
    }
}

/// Compute a composite success weight for the flywheel from a rating and the
/// prior freshness. Effective material (raising freshness from low to high)
/// is rewarded so it gets served more often.
pub fn success_weight(rating: i16, prior_freshness: f64) -> f64 {
    let base = match rating {
        1 => -0.5, // Again: material may be too hard / ineffective here.
        2 => 0.2,  // Hard
        3 => 0.7,  // Good
        4 => 1.0,  // Easy
        _ => 0.0,
    };
    // Reward lifting comprehension from a low baseline (learning happened).
    let lift = (1.0 - prior_freshness).clamp(0.0, 1.0);
    (base * (0.5 + 0.5 * lift)).clamp(-1.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn captures_learning_events() {
        let t = MockTelemetry::default();
        t.capture_learning(LearningEvent {
            profile_id: "p".into(),
            concept_id: "c".into(),
            item_id: "i".into(),
            rating: 3,
            success_weight: 0.7,
            timestamp: "now".into(),
        })
        .await;
        assert_eq!(t.learning.lock().unwrap().len(), 1);
    }

    #[test]
    fn good_rating_on_low_freshness_is_rewarded() {
        let learned = success_weight(3, 0.1);
        let already_known = success_weight(3, 0.95);
        assert!(learned > already_known);
    }

    #[test]
    fn again_rating_is_penalized() {
        assert!(success_weight(1, 0.5) < 0.0);
    }
}
