//! In-memory repository. Fully functional; powers offline E2E and unit tests.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use scaffold_core::models::*;
use std::collections::HashMap;
use std::sync::RwLock;
use uuid::Uuid;

use super::{EventRow, RepoError, Repository};

#[derive(Default)]
pub struct MemoryRepo {
    profiles: RwLock<HashMap<Uuid, Profile>>,
    courses: RwLock<HashMap<Uuid, Course>>,
    concepts: RwLock<Vec<Concept>>,
    items: RwLock<Vec<ContentItem>>,
    freshness: RwLock<HashMap<(Uuid, Uuid), FreshnessRecord>>,
    events: RwLock<Vec<EventRow>>,
}

impl MemoryRepo {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert_profile(&self, p: Profile) {
        self.profiles.write().unwrap().insert(p.id, p);
    }
}

#[async_trait]
impl Repository for MemoryRepo {
    async fn insert_course(&self, c: Course) -> Result<(), RepoError> {
        self.courses.write().unwrap().insert(c.id, c);
        Ok(())
    }
    async fn insert_concept(&self, c: Concept) -> Result<(), RepoError> {
        self.concepts.write().unwrap().push(c);
        Ok(())
    }
    async fn insert_item(&self, i: ContentItem) -> Result<(), RepoError> {
        self.items.write().unwrap().push(i);
        Ok(())
    }

    async fn list_profiles(&self) -> Result<Vec<Profile>, RepoError> {
        let mut v: Vec<_> = self.profiles.read().unwrap().values().cloned().collect();
        v.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        Ok(v)
    }

    async fn get_profile(&self, id: Uuid) -> Result<Profile, RepoError> {
        self.profiles
            .read()
            .unwrap()
            .get(&id)
            .cloned()
            .ok_or(RepoError::NotFound)
    }

    async fn create_profile(&self, p: Profile) -> Result<Profile, RepoError> {
        self.profiles.write().unwrap().insert(p.id, p.clone());
        Ok(p)
    }

    async fn list_courses(&self) -> Result<Vec<Course>, RepoError> {
        let mut v: Vec<_> = self.courses.read().unwrap().values().cloned().collect();
        v.sort_by(|a, b| a.slug.cmp(&b.slug));
        Ok(v)
    }

    async fn get_course_by_slug(&self, slug: &str) -> Result<Course, RepoError> {
        self.courses
            .read()
            .unwrap()
            .values()
            .find(|c| c.slug == slug)
            .cloned()
            .ok_or(RepoError::NotFound)
    }

    async fn concepts_for_course(&self, course_id: Uuid) -> Result<Vec<Concept>, RepoError> {
        let mut v: Vec<_> = self
            .concepts
            .read()
            .unwrap()
            .iter()
            .filter(|c| c.course_id == course_id)
            .cloned()
            .collect();
        v.sort_by_key(|c| c.sequence);
        Ok(v)
    }

    async fn items_for_course(&self, course_id: Uuid) -> Result<Vec<ContentItem>, RepoError> {
        Ok(self
            .items
            .read()
            .unwrap()
            .iter()
            .filter(|i| i.course_id == course_id)
            .cloned()
            .collect())
    }

    async fn get_item(&self, id: Uuid) -> Result<ContentItem, RepoError> {
        self.items
            .read()
            .unwrap()
            .iter()
            .find(|i| i.id == id)
            .cloned()
            .ok_or(RepoError::NotFound)
    }

    async fn freshness_for_profile(&self, profile_id: Uuid) -> Result<Vec<FreshnessRecord>, RepoError> {
        Ok(self
            .freshness
            .read()
            .unwrap()
            .values()
            .filter(|r| r.profile_id == profile_id)
            .cloned()
            .collect())
    }

    async fn get_freshness(
        &self,
        profile_id: Uuid,
        concept_id: Uuid,
    ) -> Result<Option<FreshnessRecord>, RepoError> {
        Ok(self
            .freshness
            .read()
            .unwrap()
            .get(&(profile_id, concept_id))
            .cloned())
    }

    async fn upsert_freshness(&self, rec: FreshnessRecord) -> Result<(), RepoError> {
        self.freshness
            .write()
            .unwrap()
            .insert((rec.profile_id, rec.concept_id), rec);
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
        self.events.write().unwrap().push(EventRow {
            profile_id,
            concept_id,
            item_id,
            rating,
            success_weight,
            created_at: at,
        });
        Ok(())
    }

    async fn events_for_profile(&self, profile_id: Uuid) -> Result<Vec<EventRow>, RepoError> {
        Ok(self
            .events
            .read()
            .unwrap()
            .iter()
            .filter(|e| e.profile_id == profile_id)
            .cloned()
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn profile_crud() {
        let r = MemoryRepo::new();
        let p = Profile {
            id: Uuid::new_v4(),
            name: "Test".into(),
            level: "beginner".into(),
            interests: vec!["physics".into()],
            created_at: Utc::now(),
        };
        r.create_profile(p.clone()).await.unwrap();
        assert_eq!(r.list_profiles().await.unwrap().len(), 1);
        assert_eq!(r.get_profile(p.id).await.unwrap().name, "Test");
    }

    #[tokio::test]
    async fn missing_profile_errors() {
        let r = MemoryRepo::new();
        assert!(matches!(r.get_profile(Uuid::new_v4()).await, Err(RepoError::NotFound)));
    }
}
