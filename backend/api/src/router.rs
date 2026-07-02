//! Axum router wiring all handlers, with CORS for the frontend.

use axum::{
    routing::{get, post},
    Router,
};
use tower_http::cors::{Any, CorsLayer};

use crate::handlers;
use crate::state::AppState;

pub fn build_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/api/health", get(handlers::health))
        .route("/api/profiles", get(handlers::list_profiles).post(handlers::create_profile))
        .route("/api/courses", get(handlers::list_courses))
        .route("/api/courses/:slug", get(handlers::get_course))
        .route("/api/profiles/:profile_id/courses/:slug/recommend", get(handlers::recommend))
        .route("/api/profiles/:profile_id/stats", get(handlers::stats))
        .route("/api/review", post(handlers::review))
        .route("/api/test-in", post(handlers::test_in))
        .route("/api/generate-test", post(handlers::generate_test))
        .route("/api/anki/decks", get(handlers::anki_decks))
        .route("/api/anki/decks/:deck/cards", get(handlers::anki_deck_cards))
        .route("/api/ingest/url", post(handlers::ingest_url))
        .layer(cors)
        .with_state(state)
}
