//! Postgres repository implementation (production store).
//!
//! Uses runtime queries (not the compile-time `query!` macro) so the crate
//! builds without a live database. Row mapping is explicit.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use scaffold_core::fsrs::MemoryState;
use scaffold_core::models::*;
use sqlx::{postgres::PgPoolOptions, PgPool, Row};
use uuid::Uuid;

use super::{EventRow, RepoError, Repository};

pub struct PostgresRepo {
    pool: PgPool,
}

impl PostgresRepo {
    pub async fn connect(url: &str) -> Result<Self, RepoError> {
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .connect(url)
            .await
            .map_err(|e| RepoError::Store(e.to_string()))?;
        Ok(PostgresRepo { pool })
    }

    pub async fn migrate(&self) -> Result<(), RepoError> {
        let sql = include_str!("../../../migrations/0001_init.sql");
        sqlx::raw_sql(sql)
            .execute(&self.pool)
            .await
            .map_err(|e| RepoError::Store(e.to_string()))?;
        Ok(())
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}

fn map_err(e: sqlx::Error) -> RepoError {
    match e {
        sqlx::Error::RowNotFound => RepoError::NotFound,
        other => RepoError::Store(other.to_string()),
    }
}

fn json_vec<T: serde::de::DeserializeOwned>(v: serde_json::Value) -> Vec<T> {
    serde_json::from_value(v).unwrap_or_default()
}

#[async_trait]
impl Repository for PostgresRepo {
    async fn insert_course(&self, c: Course) -> Result<(), RepoError> {
        let kind = match c.kind {
            CourseKind::Language => "language",
            CourseKind::Academic => "academic",
        };
        sqlx::query(
            "INSERT INTO courses (id, slug, title, kind, description) VALUES ($1,$2,$3,$4,$5) ON CONFLICT (id) DO NOTHING",
        )
        .bind(c.id).bind(&c.slug).bind(&c.title).bind(kind).bind(&c.description)
        .execute(&self.pool).await.map_err(map_err)?;
        Ok(())
    }

    async fn insert_concept(&self, c: Concept) -> Result<(), RepoError> {
        sqlx::query(
            "INSERT INTO concepts (id, course_id, name, detail, sequence, prerequisites) VALUES ($1,$2,$3,$4,$5,$6) ON CONFLICT (id) DO NOTHING",
        )
        .bind(c.id).bind(c.course_id).bind(&c.name).bind(&c.detail).bind(c.sequence)
        .bind(serde_json::to_value(&c.prerequisites).unwrap())
        .execute(&self.pool).await.map_err(map_err)?;
        Ok(())
    }

    async fn insert_item(&self, i: ContentItem) -> Result<(), RepoError> {
        let kind = match i.kind {
            ItemKind::Flashcard => "flashcard",
            ItemKind::Document => "document",
            ItemKind::Video => "video",
            ItemKind::Quiz => "quiz",
        };
        sqlx::query(
            "INSERT INTO content_items (id, course_id, kind, title, front, back, concept_ids, tags, difficulty, source_url) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10) ON CONFLICT (id) DO NOTHING",
        )
        .bind(i.id).bind(i.course_id).bind(kind).bind(&i.title).bind(&i.front).bind(&i.back)
        .bind(serde_json::to_value(&i.concept_ids).unwrap())
        .bind(serde_json::to_value(&i.tags).unwrap())
        .bind(i.difficulty).bind(&i.source_url)
        .execute(&self.pool).await.map_err(map_err)?;
        Ok(())
    }

    async fn list_profiles(&self) -> Result<Vec<Profile>, RepoError> {
        let rows = sqlx::query("SELECT id, name, level, interests, created_at FROM profiles ORDER BY created_at")
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(rows
            .into_iter()
            .map(|r| Profile {
                id: r.get("id"),
                name: r.get("name"),
                level: r.get("level"),
                interests: json_vec(r.get("interests")),
                created_at: r.get("created_at"),
            })
            .collect())
    }

    async fn get_profile(&self, id: Uuid) -> Result<Profile, RepoError> {
        let r = sqlx::query("SELECT id, name, level, interests, created_at FROM profiles WHERE id = $1")
            .bind(id)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(Profile {
            id: r.get("id"),
            name: r.get("name"),
            level: r.get("level"),
            interests: json_vec(r.get("interests")),
            created_at: r.get("created_at"),
        })
    }

    async fn create_profile(&self, p: Profile) -> Result<Profile, RepoError> {
        sqlx::query(
            "INSERT INTO profiles (id, name, level, interests, created_at) VALUES ($1,$2,$3,$4,$5)
             ON CONFLICT (id) DO UPDATE SET name=$2, level=$3, interests=$4",
        )
        .bind(p.id)
        .bind(&p.name)
        .bind(&p.level)
        .bind(serde_json::to_value(&p.interests).unwrap())
        .bind(p.created_at)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(p)
    }

    async fn list_courses(&self) -> Result<Vec<Course>, RepoError> {
        let rows = sqlx::query("SELECT id, slug, title, kind, description FROM courses ORDER BY slug")
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(rows
            .into_iter()
            .map(|r| Course {
                id: r.get("id"),
                slug: r.get("slug"),
                title: r.get("title"),
                kind: parse_course_kind(r.get("kind")),
                description: r.get("description"),
            })
            .collect())
    }

    async fn get_course_by_slug(&self, slug: &str) -> Result<Course, RepoError> {
        let r = sqlx::query("SELECT id, slug, title, kind, description FROM courses WHERE slug = $1")
            .bind(slug)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(Course {
            id: r.get("id"),
            slug: r.get("slug"),
            title: r.get("title"),
            kind: parse_course_kind(r.get("kind")),
            description: r.get("description"),
        })
    }

    async fn concepts_for_course(&self, course_id: Uuid) -> Result<Vec<Concept>, RepoError> {
        let rows = sqlx::query(
            "SELECT id, course_id, name, detail, sequence, prerequisites FROM concepts WHERE course_id = $1 ORDER BY sequence",
        )
        .bind(course_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(rows
            .into_iter()
            .map(|r| Concept {
                id: r.get("id"),
                course_id: r.get("course_id"),
                name: r.get("name"),
                detail: r.get("detail"),
                sequence: r.get("sequence"),
                prerequisites: json_vec(r.get("prerequisites")),
            })
            .collect())
    }

    async fn items_for_course(&self, course_id: Uuid) -> Result<Vec<ContentItem>, RepoError> {
        let rows = sqlx::query(
            "SELECT id, course_id, kind, title, front, back, concept_ids, tags, difficulty, source_url FROM content_items WHERE course_id = $1",
        )
        .bind(course_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(rows.into_iter().map(map_item).collect())
    }

    async fn get_item(&self, id: Uuid) -> Result<ContentItem, RepoError> {
        let r = sqlx::query(
            "SELECT id, course_id, kind, title, front, back, concept_ids, tags, difficulty, source_url FROM content_items WHERE id = $1",
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(map_item(r))
    }

    async fn freshness_for_profile(&self, profile_id: Uuid) -> Result<Vec<FreshnessRecord>, RepoError> {
        let rows = sqlx::query(
            "SELECT profile_id, concept_id, stability, difficulty, reps, lapses, last_review, due, freshness, updated_at FROM freshness WHERE profile_id = $1",
        )
        .bind(profile_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(rows.into_iter().map(map_freshness).collect())
    }

    async fn get_freshness(
        &self,
        profile_id: Uuid,
        concept_id: Uuid,
    ) -> Result<Option<FreshnessRecord>, RepoError> {
        let r = sqlx::query(
            "SELECT profile_id, concept_id, stability, difficulty, reps, lapses, last_review, due, freshness, updated_at FROM freshness WHERE profile_id = $1 AND concept_id = $2",
        )
        .bind(profile_id)
        .bind(concept_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r.map(map_freshness))
    }

    async fn upsert_freshness(&self, rec: FreshnessRecord) -> Result<(), RepoError> {
        sqlx::query(
            "INSERT INTO freshness (profile_id, concept_id, stability, difficulty, reps, lapses, last_review, due, freshness, updated_at)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
             ON CONFLICT (profile_id, concept_id) DO UPDATE SET
               stability=$3, difficulty=$4, reps=$5, lapses=$6, last_review=$7, due=$8, freshness=$9, updated_at=$10",
        )
        .bind(rec.profile_id)
        .bind(rec.concept_id)
        .bind(rec.memory.stability)
        .bind(rec.memory.difficulty)
        .bind(rec.memory.reps as i32)
        .bind(rec.memory.lapses as i32)
        .bind(rec.memory.last_review)
        .bind(rec.due)
        .bind(rec.freshness)
        .bind(rec.updated_at)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    async fn record_event(
        &self,
        profile_id: Uuid,
        concept_id: Uuid,
        item_id: Uuid,
        rating: i16,
        success_weight: f64,
        at: DateTime<Utc>,
    ) -> Result<(), RepoError> {
        sqlx::query(
            "INSERT INTO learning_events (id, profile_id, concept_id, item_id, rating, success_weight, created_at)
             VALUES ($1,$2,$3,$4,$5,$6,$7)",
        )
        .bind(Uuid::new_v4())
        .bind(profile_id)
        .bind(concept_id)
        .bind(item_id)
        .bind(rating)
        .bind(success_weight)
        .bind(at)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    async fn events_for_profile(&self, profile_id: Uuid) -> Result<Vec<EventRow>, RepoError> {
        let rows = sqlx::query(
            "SELECT profile_id, concept_id, item_id, rating, success_weight, created_at FROM learning_events WHERE profile_id = $1 ORDER BY created_at",
        )
        .bind(profile_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(rows
            .into_iter()
            .map(|r| EventRow {
                profile_id: r.get("profile_id"),
                concept_id: r.get("concept_id"),
                item_id: r.get("item_id"),
                rating: r.get("rating"),
                success_weight: r.get("success_weight"),
                created_at: r.get("created_at"),
            })
            .collect())
    }
}

fn parse_course_kind(s: String) -> CourseKind {
    match s.as_str() {
        "language" => CourseKind::Language,
        _ => CourseKind::Academic,
    }
}

fn parse_item_kind(s: &str) -> ItemKind {
    match s {
        "flashcard" => ItemKind::Flashcard,
        "document" => ItemKind::Document,
        "video" => ItemKind::Video,
        _ => ItemKind::Quiz,
    }
}

fn map_item(r: sqlx::postgres::PgRow) -> ContentItem {
    ContentItem {
        id: r.get("id"),
        course_id: r.get("course_id"),
        kind: parse_item_kind(r.get::<String, _>("kind").as_str()),
        title: r.get("title"),
        front: r.get("front"),
        back: r.get("back"),
        concept_ids: json_vec(r.get("concept_ids")),
        tags: json_vec(r.get("tags")),
        difficulty: r.get("difficulty"),
        source_url: r.get("source_url"),
    }
}

fn map_freshness(r: sqlx::postgres::PgRow) -> FreshnessRecord {
    FreshnessRecord {
        profile_id: r.get("profile_id"),
        concept_id: r.get("concept_id"),
        memory: MemoryState {
            stability: r.get("stability"),
            difficulty: r.get("difficulty"),
            reps: r.get::<i32, _>("reps") as u32,
            lapses: r.get::<i32, _>("lapses") as u32,
            last_review: r.get("last_review"),
        },
        due: r.get("due"),
        freshness: r.get("freshness"),
        updated_at: r.get("updated_at"),
    }
}
