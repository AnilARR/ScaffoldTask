//! Shared domain models used across the API and persistence layers.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::fsrs::MemoryState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub id: Uuid,
    pub name: String,
    /// Human label for the comprehension tier, e.g. "beginner".
    pub level: String,
    /// Interest tags used to bias comprehensible-input selection.
    pub interests: Vec<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CourseKind {
    Language,
    Academic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Course {
    pub id: Uuid,
    pub slug: String,
    pub title: String,
    pub kind: CourseKind,
    pub description: String,
}

/// A concept or vocabulary word — the atomic unit of comprehension tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Concept {
    pub id: Uuid,
    pub course_id: Uuid,
    /// The word or concept name.
    pub name: String,
    /// Optional reading/definition for language cards.
    pub detail: Option<String>,
    /// Coarse ordering within the course (curriculum position).
    pub sequence: i32,
    /// Upstream prerequisite concept ids (limits -> derivatives, etc.).
    pub prerequisites: Vec<Uuid>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemKind {
    Flashcard,
    Document,
    Video,
    Quiz,
}

/// A piece of comprehensible input attached to concepts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentItem {
    pub id: Uuid,
    pub course_id: Uuid,
    pub kind: ItemKind,
    pub title: String,
    /// Front / prompt / body.
    pub front: String,
    /// Back / answer / transcript.
    pub back: String,
    pub concept_ids: Vec<Uuid>,
    pub tags: Vec<String>,
    /// Manually graded 0..1 complexity (later AI-graded).
    pub difficulty: f64,
    /// Optional external URL (blog, YouTube video, etc.).
    pub source_url: Option<String>,
}

/// Per-(profile, concept) freshness / FSRS state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FreshnessRecord {
    pub profile_id: Uuid,
    pub concept_id: Uuid,
    pub memory: MemoryState,
    pub due: Option<DateTime<Utc>>,
    /// Cached freshness (retrievability) at `updated_at`.
    pub freshness: f64,
    pub updated_at: DateTime<Utc>,
}
