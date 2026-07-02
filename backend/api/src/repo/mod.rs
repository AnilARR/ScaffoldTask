//! Persistence abstraction. The `Repository` trait lets handlers stay agnostic
//! of the backing store. We provide an in-memory implementation (used for tests
//! and offline E2E) and a Postgres implementation for production.

pub mod memory;
pub mod postgres;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use scaffold_core::fsrs::MemoryState;
use scaffold_core::models::*;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum RepoError {
    #[error("not found")]
    NotFound,
    #[error("store error: {0}")]
    Store(String),
}

#[async_trait]
pub trait Repository: Send + Sync {
    // Seeding / write paths for reference data.
    async fn insert_course(&self, course: Course) -> Result<(), RepoError>;
    async fn insert_concept(&self, concept: Concept) -> Result<(), RepoError>;
    async fn insert_item(&self, item: ContentItem) -> Result<(), RepoError>;

    async fn list_profiles(&self) -> Result<Vec<Profile>, RepoError>;
    async fn get_profile(&self, id: Uuid) -> Result<Profile, RepoError>;
    async fn create_profile(&self, p: Profile) -> Result<Profile, RepoError>;

    async fn list_courses(&self) -> Result<Vec<Course>, RepoError>;
    async fn get_course_by_slug(&self, slug: &str) -> Result<Course, RepoError>;

    async fn concepts_for_course(&self, course_id: Uuid) -> Result<Vec<Concept>, RepoError>;
    async fn items_for_course(&self, course_id: Uuid) -> Result<Vec<ContentItem>, RepoError>;
    async fn get_item(&self, id: Uuid) -> Result<ContentItem, RepoError>;

    async fn freshness_for_profile(&self, profile_id: Uuid) -> Result<Vec<FreshnessRecord>, RepoError>;
    async fn get_freshness(&self, profile_id: Uuid, concept_id: Uuid) -> Result<Option<FreshnessRecord>, RepoError>;
    async fn upsert_freshness(&self, rec: FreshnessRecord) -> Result<(), RepoError>;

    async fn record_event(
        &self,
        profile_id: Uuid,
        concept_id: Uuid,
        item_id: Uuid,
        rating: i16,
        success_weight: f64,
        at: DateTime<Utc>,
    ) -> Result<(), RepoError>;

    async fn events_for_profile(&self, profile_id: Uuid) -> Result<Vec<EventRow>, RepoError>;
}

#[derive(Debug, Clone)]
pub struct EventRow {
    pub profile_id: Uuid,
    pub concept_id: Uuid,
    pub item_id: Uuid,
    pub rating: i16,
    pub success_weight: f64,
    pub created_at: DateTime<Utc>,
}

/// Helper to build a fresh freshness record from a memory state.
pub fn freshness_record(
    profile_id: Uuid,
    concept_id: Uuid,
    memory: MemoryState,
    due: Option<DateTime<Utc>>,
    freshness: f64,
    updated_at: DateTime<Utc>,
) -> FreshnessRecord {
    FreshnessRecord {
        profile_id,
        concept_id,
        memory,
        due,
        freshness,
        updated_at,
    }
}
