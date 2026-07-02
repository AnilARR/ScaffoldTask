//! Learning engine: orchestrates FSRS freshness updates, comprehension mapping,
//! and N+1 content selection on top of the repository + services.

use chrono::Utc;
use scaffold_core::fsrs::{Fsrs, MemoryState, Rating};
use scaffold_core::models::{ContentItem, FreshnessRecord};
use scaffold_core::nplus1::{Candidate, ComprehensionMap, ScoredCandidate, SelectionParams, Selector};
use uuid::Uuid;

use crate::repo::{freshness_record, RepoError, Repository};
use crate::services::telemetry::{success_weight, LearningEvent, TelemetrySink};

/// Build the current comprehension map (concept_id -> freshness) for a profile,
/// recomputing retrievability as of now.
pub async fn comprehension_map(
    repo: &dyn Repository,
    profile_id: Uuid,
) -> Result<ComprehensionMap, RepoError> {
    let fsrs = Fsrs::default();
    let now = Utc::now();
    let records = repo.freshness_for_profile(profile_id).await?;
    let mut map = ComprehensionMap::new();
    for r in records {
        let fresh = fsrs.retrievability(&r.memory, now);
        map.insert(r.concept_id, fresh);
    }
    Ok(map)
}

/// Convert content items to N+1 candidates.
pub fn items_to_candidates(items: &[ContentItem]) -> Vec<Candidate> {
    items
        .iter()
        .map(|i| Candidate {
            id: i.id,
            concept_ids: i.concept_ids.clone(),
            tags: i.tags.clone(),
            difficulty: i.difficulty,
        })
        .collect()
}

/// Rank a course's content for a profile by N+1 fit.
pub async fn recommend(
    repo: &dyn Repository,
    profile_id: Uuid,
    course_id: Uuid,
) -> Result<Vec<ScoredCandidate>, RepoError> {
    let profile = repo.get_profile(profile_id).await?;
    let comp = comprehension_map(repo, profile_id).await?;
    let items = repo.items_for_course(course_id).await?;
    let candidates = items_to_candidates(&items);
    let selector = Selector::new(SelectionParams::default());
    Ok(selector.rank(&candidates, &comp, &profile.interests))
}

/// Apply a review of `item_id` with `rating` for a profile. Updates freshness
/// for every concept the item touches, records the event, and captures
/// telemetry for the flywheel. Returns the updated freshness records.
pub async fn review(
    repo: &dyn Repository,
    telemetry: &dyn TelemetrySink,
    profile_id: Uuid,
    item_id: Uuid,
    rating: Rating,
) -> Result<Vec<FreshnessRecord>, RepoError> {
    let fsrs = Fsrs::default();
    let now = Utc::now();
    let item = repo.get_item(item_id).await?;
    let mut updated = Vec::new();

    for concept_id in &item.concept_ids {
        let prior = repo.get_freshness(profile_id, *concept_id).await?;
        let prior_memory = prior.as_ref().map(|r| r.memory).unwrap_or(MemoryState::default());
        let prior_fresh = prior.as_ref().map(|r| r.freshness).unwrap_or(0.0);

        let sched = fsrs.schedule(&prior_memory, rating, now);
        let fresh = fsrs.retrievability(&sched.state, now);
        let rec = freshness_record(profile_id, *concept_id, sched.state, Some(sched.due), fresh, now);
        repo.upsert_freshness(rec.clone()).await?;

        let weight = success_weight(rating as i16, prior_fresh);
        repo.record_event(profile_id, *concept_id, item_id, rating as i16, weight, now)
            .await?;
        telemetry
            .capture_learning(LearningEvent {
                profile_id: profile_id.to_string(),
                concept_id: concept_id.to_string(),
                item_id: item_id.to_string(),
                rating: rating as i16,
                success_weight: weight,
                timestamp: now.to_rfc3339(),
            })
            .await;
        updated.push(rec);
    }

    telemetry.metric("reviews_total", 1.0).await;
    Ok(updated)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repo::memory::MemoryRepo;
    use crate::seed;
    use crate::services::telemetry::MockTelemetry;

    #[tokio::test]
    async fn recommend_prefers_n_plus_one_for_beginner() {
        let repo = MemoryRepo::new();
        let res = seed::seed(&repo).await;
        let beginner = seed::id("profile", "beginner");
        let ranked = recommend(&repo, beginner, res.japanese.id).await.unwrap();
        assert!(!ranked.is_empty());
        // Top pick should be usable and introduce at least one new concept.
        assert!(ranked[0].score > 0.0);
    }

    #[tokio::test]
    async fn review_updates_freshness_and_telemetry() {
        let repo = MemoryRepo::new();
        let res = seed::seed(&repo).await;
        let tele = MockTelemetry::default();
        let profile = seed::id("profile", "beginner");
        let item = seed::id("item", &format!("{}:{}", res.japanese.slug, "fc-taberu"));

        let before = comprehension_map(&repo, profile).await.unwrap();
        let updated = review(&repo, &tele, profile, item, Rating::Good).await.unwrap();
        assert!(!updated.is_empty());

        let after = comprehension_map(&repo, profile).await.unwrap();
        // The reviewed concept's freshness should not decrease.
        for rec in &updated {
            let b = before.get(&rec.concept_id).copied().unwrap_or(0.0);
            let a = after.get(&rec.concept_id).copied().unwrap_or(0.0);
            assert!(a >= b - 1e-9);
        }
        assert_eq!(tele.learning.lock().unwrap().len(), updated.len());
    }
}
