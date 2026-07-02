//! HTTP-level end-to-end tests driving the full router with mocks.
//!
//! Exercises the core learning loop: list profiles -> get course ->
//! recommend (N+1) -> review -> stats reflect the review.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use scaffold_api::{build_app, config::Settings, router::build_router};
use serde_json::Value;
use tower::ServiceExt;

async fn app() -> axum::Router {
    let mut settings = Settings::from_env();
    settings.use_mocks = true;
    settings.database_url = None;
    let state = build_app(&settings).await.unwrap();
    build_router(state)
}

async fn get_json(app: &axum::Router, uri: &str) -> (StatusCode, Value) {
    let resp = app
        .clone()
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let val: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, val)
}

async fn post_json(app: &axum::Router, uri: &str, body: Value) -> (StatusCode, Value) {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(uri)
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let val: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, val)
}

#[tokio::test]
async fn health_ok() {
    let app = app().await;
    let (status, body) = get_json(&app, "/api/health").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
async fn lists_seeded_profiles_and_courses() {
    let app = app().await;
    let (_, profiles) = get_json(&app, "/api/profiles").await;
    assert_eq!(profiles.as_array().unwrap().len(), 3);

    let (_, courses) = get_json(&app, "/api/courses").await;
    let slugs: Vec<&str> = courses
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["slug"].as_str().unwrap())
        .collect();
    assert!(slugs.contains(&"physics-201"));
    assert!(slugs.contains(&"japanese-103"));
}

#[tokio::test]
async fn full_learning_loop() {
    let app = app().await;

    // Grab beginner profile.
    let (_, profiles) = get_json(&app, "/api/profiles").await;
    let beginner = profiles
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["level"] == "beginner")
        .unwrap()
        .clone();
    let profile_id = beginner["id"].as_str().unwrap();

    // Recommendations for Japanese.
    let (status, recs) = get_json(
        &app,
        &format!("/api/profiles/{profile_id}/courses/japanese-103/recommend"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let recs = recs.as_array().unwrap();
    assert!(!recs.is_empty());
    let top = &recs[0];
    assert!(top["score"].as_f64().unwrap() > 0.0);
    let item_id = top["item"]["id"].as_str().unwrap().to_string();

    // Stats before review.
    let (_, before) = get_json(&app, &format!("/api/profiles/{profile_id}/stats")).await;
    let reviews_before = before["reviews_total"].as_u64().unwrap();

    // Review the top item with Good.
    let (status, updated) = post_json(
        &app,
        "/api/review",
        serde_json::json!({ "profile_id": profile_id, "item_id": item_id, "rating": 3 }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(!updated.as_array().unwrap().is_empty());

    // Stats after review should show more reviews.
    let (_, after) = get_json(&app, &format!("/api/profiles/{profile_id}/stats")).await;
    let reviews_after = after["reviews_total"].as_u64().unwrap();
    assert!(reviews_after > reviews_before);
}

#[tokio::test]
async fn invalid_rating_is_rejected() {
    let app = app().await;
    let (_, profiles) = get_json(&app, "/api/profiles").await;
    let profile_id = profiles.as_array().unwrap()[0]["id"].as_str().unwrap().to_string();
    let (status, _) = post_json(
        &app,
        "/api/review",
        serde_json::json!({ "profile_id": profile_id, "item_id": profile_id, "rating": 9 }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn anki_mock_decks_available() {
    let app = app().await;
    let (status, decks) = get_json(&app, "/api/anki/decks").await;
    assert_eq!(status, StatusCode::OK);
    assert!(!decks.as_array().unwrap().is_empty());
}

#[tokio::test]
async fn missing_course_returns_404() {
    let app = app().await;
    let (status, _) = get_json(&app, "/api/courses/nope").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn generate_test_returns_mcq() {
    let app = app().await;
    let (status, q) = post_json(
        &app,
        "/api/generate-test",
        serde_json::json!({ "concept": "Newton's second law", "context": "F=ma" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(q["choices"].as_array().unwrap().len(), 4);
}
