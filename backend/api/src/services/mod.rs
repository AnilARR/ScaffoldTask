//! External service integrations. Each service exposes a trait plus a real
//! (HTTP/SDK) implementation and a deterministic mock used for tests and
//! offline E2E runs.

pub mod ai;
pub mod anki;
pub mod aws;
pub mod ingest;
pub mod telemetry;

use std::sync::Arc;

/// Bundle of all external services, injected into the app state so handlers can
/// depend on traits (and be swapped for mocks in tests).
#[derive(Clone)]
pub struct Services {
    pub anki: Arc<dyn anki::AnkiClient>,
    pub ai: Arc<dyn ai::AiClient>,
    pub scraper: Arc<dyn ingest::Scraper>,
    pub captions: Arc<dyn ingest::CaptionFetcher>,
    pub telemetry: Arc<dyn telemetry::TelemetrySink>,
}

impl Services {
    /// All-mock service bundle for tests / offline demo.
    pub fn all_mocks() -> Self {
        Services {
            anki: Arc::new(anki::MockAnkiClient::with_sample()),
            ai: Arc::new(ai::MockAiClient),
            scraper: Arc::new(ingest::MockScraper),
            captions: Arc::new(ingest::MockCaptionFetcher),
            telemetry: Arc::new(telemetry::MockTelemetry::default()),
        }
    }
}
