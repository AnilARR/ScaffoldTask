//! HTTP handlers exposing the learning platform's REST API.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::Utc;
use scaffold_core::fsrs::Rating;
use scaffold_core::models::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::engine;
use crate::repo::RepoError;
use crate::state::AppState;

pub async fn health() -> impl IntoResponse {
    Json(serde_json::json!({ "status": "ok", "time": Utc::now().to_rfc3339() }))
}

fn err(e: RepoError) -> (StatusCode, Json<serde_json::Value>) {
    let code = match e {
        RepoError::NotFound => StatusCode::NOT_FOUND,
        RepoError::Store(_) => StatusCode::INTERNAL_SERVER_ERROR,
    };
    (code, Json(serde_json::json!({ "error": e.to_string() })))
}

// ---- Profiles ---------------------------------------------------------------

pub async fn list_profiles(State(s): State<AppState>) -> impl IntoResponse {
    match s.repo.list_profiles().await {
        Ok(p) => Json(p).into_response(),
        Err(e) => err(e).into_response(),
    }
}

#[derive(Deserialize)]
pub struct NewProfile {
    pub name: String,
    pub level: String,
    #[serde(default)]
    pub interests: Vec<String>,
}

pub async fn create_profile(
    State(s): State<AppState>,
    Json(body): Json<NewProfile>,
) -> impl IntoResponse {
    let p = Profile {
        id: Uuid::new_v4(),
        name: body.name,
        level: body.level,
        interests: body.interests,
        created_at: Utc::now(),
    };
    match s.repo.create_profile(p).await {
        Ok(p) => (StatusCode::CREATED, Json(p)).into_response(),
        Err(e) => err(e).into_response(),
    }
}

// ---- Courses ----------------------------------------------------------------

pub async fn list_courses(State(s): State<AppState>) -> impl IntoResponse {
    match s.repo.list_courses().await {
        Ok(c) => Json(c).into_response(),
        Err(e) => err(e).into_response(),
    }
}

#[derive(Serialize)]
pub struct CourseDetail {
    pub course: Course,
    pub concepts: Vec<Concept>,
    pub items: Vec<ContentItem>,
}

pub async fn get_course(
    State(s): State<AppState>,
    Path(slug): Path<String>,
) -> impl IntoResponse {
    let course = match s.repo.get_course_by_slug(&slug).await {
        Ok(c) => c,
        Err(e) => return err(e).into_response(),
    };
    let concepts = s.repo.concepts_for_course(course.id).await.unwrap_or_default();
    let items = s.repo.items_for_course(course.id).await.unwrap_or_default();
    Json(CourseDetail { course, concepts, items }).into_response()
}

// ---- Recommendations (N+1 selection) ---------------------------------------

#[derive(Serialize)]
pub struct Recommendation {
    pub item: ContentItem,
    pub score: f64,
    pub comprehensible_ratio: f64,
    pub new_concepts: usize,
}

pub async fn recommend(
    State(s): State<AppState>,
    Path((profile_id, slug)): Path<(Uuid, String)>,
) -> impl IntoResponse {
    let course = match s.repo.get_course_by_slug(&slug).await {
        Ok(c) => c,
        Err(e) => return err(e).into_response(),
    };
    let ranked = match engine::recommend(s.repo.as_ref(), profile_id, course.id).await {
        Ok(r) => r,
        Err(e) => return err(e).into_response(),
    };
    let mut out = Vec::new();
    for sc in ranked {
        if let Ok(item) = s.repo.get_item(sc.id).await {
            out.push(Recommendation {
                item,
                score: sc.score,
                comprehensible_ratio: sc.comprehensible_ratio,
                new_concepts: sc.new_concepts,
            });
        }
    }
    Json(out).into_response()
}

// ---- Review -----------------------------------------------------------------

#[derive(Deserialize)]
pub struct ReviewBody {
    pub profile_id: Uuid,
    pub item_id: Uuid,
    pub rating: i16,
}

pub async fn review(State(s): State<AppState>, Json(body): Json<ReviewBody>) -> impl IntoResponse {
    let rating = match Rating::from_i16(body.rating) {
        Some(r) => r,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "rating must be 1..4" })),
            )
                .into_response()
        }
    };
    match engine::review(
        s.repo.as_ref(),
        s.services.telemetry.as_ref(),
        body.profile_id,
        body.item_id,
        rating,
    )
    .await
    {
        Ok(updated) => Json(updated).into_response(),
        Err(e) => err(e).into_response(),
    }
}

// ---- Stats ------------------------------------------------------------------

#[derive(Serialize)]
pub struct Stats {
    pub profile_id: Uuid,
    pub concepts_tracked: usize,
    pub concepts_known: usize,
    pub average_freshness: f64,
    pub reviews_total: usize,
    /// GitHub-style activity: date -> review count.
    pub activity: Vec<ActivityDay>,
}

#[derive(Serialize)]
pub struct ActivityDay {
    pub date: String,
    pub count: usize,
}

pub async fn stats(
    State(s): State<AppState>,
    Path(profile_id): Path<Uuid>,
) -> impl IntoResponse {
    let comp = match engine::comprehension_map(s.repo.as_ref(), profile_id).await {
        Ok(c) => c,
        Err(e) => return err(e).into_response(),
    };
    let tracked = comp.len();
    let known = comp.values().filter(|v| **v >= 0.6).count();
    let avg = if tracked == 0 {
        0.0
    } else {
        comp.values().sum::<f64>() / tracked as f64
    };

    let events = s.repo.events_for_profile(profile_id).await.unwrap_or_default();
    use std::collections::BTreeMap;
    let mut by_day: BTreeMap<String, usize> = BTreeMap::new();
    for e in &events {
        *by_day.entry(e.created_at.format("%Y-%m-%d").to_string()).or_default() += 1;
    }
    let activity = by_day
        .into_iter()
        .map(|(date, count)| ActivityDay { date, count })
        .collect();

    Json(Stats {
        profile_id,
        concepts_tracked: tracked,
        concepts_known: known,
        average_freshness: avg,
        reviews_total: events.len(),
        activity,
    })
    .into_response()
}

// ---- Anki sync --------------------------------------------------------------

pub async fn anki_decks(State(s): State<AppState>) -> impl IntoResponse {
    match s.services.anki.list_decks().await {
        Ok(d) => Json(d).into_response(),
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn anki_deck_cards(
    State(s): State<AppState>,
    Path(deck): Path<String>,
) -> impl IntoResponse {
    match s.services.anki.cards_in_deck(&deck).await {
        Ok(c) => Json(c).into_response(),
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

// ---- Testing-in (baseline) --------------------------------------------------

#[derive(Deserialize)]
pub struct TestInBody {
    pub profile_id: Uuid,
    pub course_slug: String,
    /// concept_id -> self-rated known (true/false).
    pub answers: std::collections::HashMap<Uuid, bool>,
}

pub async fn test_in(State(s): State<AppState>, Json(body): Json<TestInBody>) -> impl IntoResponse {
    let now = Utc::now();
    let fsrs = scaffold_core::fsrs::Fsrs::default();
    let mut written = 0usize;
    for (concept_id, known) in body.answers {
        let rating = if known { Rating::Good } else { Rating::Again };
        let sched = fsrs.schedule(&scaffold_core::fsrs::MemoryState::default(), rating, now);
        let fresh = fsrs.retrievability(&sched.state, now);
        let rec = crate::repo::freshness_record(
            body.profile_id,
            concept_id,
            sched.state,
            Some(sched.due),
            fresh,
            now,
        );
        if s.repo.upsert_freshness(rec).await.is_ok() {
            written += 1;
        }
    }
    Json(serde_json::json!({ "baseline_written": written })).into_response()
}

// ---- Ingest (scrape URL / captions) ----------------------------------------

#[derive(Deserialize)]
pub struct IngestUrl {
    pub url: String,
}

pub async fn ingest_url(State(s): State<AppState>, Json(body): Json<IngestUrl>) -> impl IntoResponse {
    match s.services.scraper.scrape(&body.url).await {
        Ok(content) => {
            let grade = s.services.ai.grade_complexity(&content.text).await.ok();
            Json(serde_json::json!({ "content": content, "grade": grade })).into_response()
        }
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

// ---- Comprehension test generation -----------------------------------------

#[derive(Deserialize)]
pub struct GenTest {
    pub concept: String,
    #[serde(default)]
    pub context: String,
}

pub async fn generate_test(State(s): State<AppState>, Json(body): Json<GenTest>) -> impl IntoResponse {
    match s.services.ai.generate_mcq(&body.concept, &body.context).await {
        Ok(q) => Json(q).into_response(),
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}
