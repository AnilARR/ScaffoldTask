//! Backend API library for the N+1 comprehensible-input learning platform.

pub mod config;
pub mod engine;
pub mod handlers;
pub mod repo;
pub mod router;
pub mod seed;
pub mod services;
pub mod state;

use std::sync::Arc;

use config::Settings;
use repo::memory::MemoryRepo;
use repo::postgres::PostgresRepo;
use repo::Repository;
use services::{ai, anki, ingest, telemetry, Services};
use state::AppState;

/// Build the full application state from settings, wiring the appropriate store
/// (Postgres or in-memory) and real-or-mock external services, then seeding.
pub async fn build_app(settings: &Settings) -> anyhow::Result<AppState> {
    // The store (Postgres vs in-memory) is independent from whether external
    // services are mocked: production runs Postgres with mocked externals until
    // real credentials are supplied.
    let repo: Arc<dyn Repository> = match &settings.database_url {
        Some(url) => match PostgresRepo::connect(url).await {
            Ok(pg) => {
                pg.migrate().await?;
                tracing::info!("using Postgres store");
                Arc::new(pg)
            }
            Err(e) => {
                tracing::warn!("Postgres unavailable ({e}); falling back to in-memory store");
                Arc::new(MemoryRepo::new())
            }
        },
        None => {
            tracing::info!("no DATABASE_URL; using in-memory store");
            Arc::new(MemoryRepo::new())
        }
    };

    // Seed reference data (idempotent).
    seed::seed(repo.as_ref()).await;

    let services = if settings.use_mocks {
        let mut s = Services::all_mocks();
        // Capture the flywheel's events to disk so they are inspectable.
        s.telemetry = Arc::new(telemetry::FileTelemetry::new(
            "telemetry/captured/events.jsonl",
        ));
        s
    } else {
        Services {
            anki: Arc::new(anki::HttpAnkiClient::new(settings.anki_endpoint.clone())),
            ai: Arc::new(ai::HttpAiClient::new(
                settings.ai_endpoint.clone(),
                settings.ai_api_key.clone().unwrap_or_default(),
                settings.ai_model.clone(),
            )),
            scraper: Arc::new(ingest::PlaywrightScraper::new(settings.scraper_url.clone())),
            captions: Arc::new(ingest::YoutubeCaptionFetcher::new(
                settings.youtube_api_key.clone().unwrap_or_default(),
            )),
            telemetry: Arc::new(telemetry::MockTelemetry::default()),
        }
    };

    Ok(AppState::new(repo, services))
}
