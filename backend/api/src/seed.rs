//! Deterministic seed data: 3 switchable profiles and two full course paths
//! (Physics 201, Japanese 103) with concepts and graded content items.
//!
//! Seeding is idempotent per store instance and produces stable UUIDs (v5) from
//! names so the frontend and tests can reference them reliably.

use chrono::{Duration, Utc};
use scaffold_core::fsrs::{Fsrs, MemoryState};
use scaffold_core::models::*;
use uuid::Uuid;

use crate::repo::{freshness_record, Repository};

/// Stable namespace for v5 UUIDs.
const NS: Uuid = Uuid::from_u128(0x1b671a64_40d5_491e_99b0_da01ff1f3341);

pub fn id(kind: &str, key: &str) -> Uuid {
    Uuid::new_v5(&NS, format!("{kind}:{key}").as_bytes())
}

pub struct SeedResult {
    pub physics: Course,
    pub japanese: Course,
}

/// Populate the repository with courses, concepts, items, profiles, and initial
/// freshness state reflecting each profile's comprehension tier.
pub async fn seed(repo: &dyn Repository) -> SeedResult {
    let now = Utc::now();
    let fsrs = Fsrs::default();

    // ---- Courses ------------------------------------------------------------
    let physics = Course {
        id: id("course", "physics-201"),
        slug: "physics-201".into(),
        title: "Physics 201: Mechanics & Calculus".into(),
        kind: CourseKind::Academic,
        description: "College-level mechanics grounded in single-variable calculus.".into(),
    };
    let japanese = Course {
        id: id("course", "japanese-103"),
        slug: "japanese-103".into(),
        title: "Japanese 103: Intermediate Vocabulary".into(),
        kind: CourseKind::Language,
        description: "Intermediate Japanese vocabulary & grammar via comprehensible input.".into(),
    };
    repo.insert_course(physics.clone()).await.ok();
    repo.insert_course(japanese.clone()).await.ok();

    // ---- Physics concepts ---------------------------------------------------
    let phys_concepts = vec![
        concept(&physics, "limits", "Limits", Some("Foundation of calculus"), 1, &[]),
        concept(&physics, "derivatives", "Derivatives", Some("Rate of change"), 2, &["limits"]),
        concept(&physics, "lhopital", "L'Hopital's Rule", Some("Indeterminate forms"), 3, &["limits", "derivatives"]),
        concept(&physics, "kinematics", "Kinematics", Some("Motion equations"), 4, &["derivatives"]),
        concept(&physics, "newton2", "Newton's Second Law", Some("F = ma"), 5, &["kinematics"]),
        concept(&physics, "energy", "Work & Energy", Some("Work-energy theorem"), 6, &["newton2"]),
    ];
    for c in &phys_concepts {
        repo.insert_concept(c.clone()).await.ok();
    }

    // ---- Japanese concepts --------------------------------------------------
    let jp_concepts = vec![
        concept(&japanese, "mizu", "水 (mizu)", Some("water"), 1, &[]),
        concept(&japanese, "taberu", "食べる (taberu)", Some("to eat"), 2, &[]),
        concept(&japanese, "iku", "行く (iku)", Some("to go"), 3, &[]),
        concept(&japanese, "te-form", "て-form", Some("connective verb form"), 4, &["taberu", "iku"]),
        concept(&japanese, "tara", "たら (conditional)", Some("if/when"), 5, &["te-form"]),
        concept(&japanese, "keigo", "敬語 (keigo)", Some("polite/honorific speech"), 6, &["te-form"]),
    ];
    for c in &jp_concepts {
        repo.insert_concept(c.clone()).await.ok();
    }

    // ---- Content items (graded complexity vectors) --------------------------
    for it in physics_items(&physics) {
        repo.insert_item(it).await.ok();
    }
    for it in japanese_items(&japanese) {
        repo.insert_item(it).await.ok();
    }

    // ---- Profiles -----------------------------------------------------------
    let beginner = Profile {
        id: id("profile", "beginner"),
        name: "Aiko (Beginner)".into(),
        level: "beginner".into(),
        interests: vec!["language".into(), "travel".into(), "cooking".into()],
        created_at: now - Duration::days(30),
    };
    let intermediate = Profile {
        id: id("profile", "intermediate"),
        name: "Ben (Intermediate)".into(),
        level: "intermediate".into(),
        interests: vec!["physics".into(), "mechanics".into(), "space".into()],
        created_at: now - Duration::days(20),
    };
    let advanced = Profile {
        id: id("profile", "advanced"),
        name: "Chen (Advanced)".into(),
        level: "advanced".into(),
        interests: vec!["calculus".into(), "grammar".into(), "physics".into()],
        created_at: now - Duration::days(10),
    };
    for p in [&beginner, &intermediate, &advanced] {
        repo.create_profile(p.clone()).await.ok();
    }

    // ---- Initial freshness: tier-appropriate comprehension ------------------
    // beginner: knows a couple of early concepts; intermediate: knows the first
    // half; advanced: knows almost everything.
    let all_concepts: Vec<Concept> = phys_concepts.iter().chain(jp_concepts.iter()).cloned().collect();
    seed_freshness(repo, &fsrs, &beginner, &all_concepts, 0.15, now).await;
    seed_freshness(repo, &fsrs, &intermediate, &all_concepts, 0.5, now).await;
    seed_freshness(repo, &fsrs, &advanced, &all_concepts, 0.85, now).await;

    SeedResult { physics, japanese }
}

/// Fraction `known_ratio` of each course's early concepts get high freshness.
async fn seed_freshness(
    repo: &dyn Repository,
    fsrs: &Fsrs,
    profile: &Profile,
    concepts: &[Concept],
    known_ratio: f64,
    now: chrono::DateTime<Utc>,
) {
    // Group by course, mark the earliest `known_ratio` fraction as learned.
    use std::collections::BTreeMap;
    let mut by_course: BTreeMap<Uuid, Vec<&Concept>> = BTreeMap::new();
    for c in concepts {
        by_course.entry(c.course_id).or_default().push(c);
    }
    for (_course, mut list) in by_course {
        list.sort_by_key(|c| c.sequence);
        let cutoff = (list.len() as f64 * known_ratio).ceil() as usize;
        for (idx, c) in list.iter().enumerate() {
            let (memory, freshness) = if idx < cutoff {
                // Simulate several successful reviews.
                let mut st = MemoryState::default();
                let mut t = now - Duration::days(20);
                for _ in 0..3 {
                    let r = fsrs.schedule(&st, scaffold_core::Rating::Good, t);
                    st = r.state;
                    t = t + Duration::days(r.interval_days.min(5));
                }
                let fresh = fsrs.retrievability(&st, now);
                (st, fresh)
            } else {
                (MemoryState::default(), 0.0)
            };
            let due = memory.last_review.map(|lr| lr + Duration::days(1));
            repo.upsert_freshness(freshness_record(
                profile.id, c.id, memory, due, freshness, now,
            ))
            .await
            .ok();
        }
    }
}

fn concept(course: &Course, key: &str, name: &str, detail: Option<&str>, seq: i32, prereqs: &[&str]) -> Concept {
    Concept {
        id: id("concept", &format!("{}:{}", course.slug, key)),
        course_id: course.id,
        name: name.to_string(),
        detail: detail.map(|s| s.to_string()),
        sequence: seq,
        prerequisites: prereqs
            .iter()
            .map(|p| id("concept", &format!("{}:{}", course.slug, p)))
            .collect(),
    }
}

fn item(
    course: &Course,
    key: &str,
    kind: ItemKind,
    title: &str,
    front: &str,
    back: &str,
    concept_keys: &[&str],
    tags: &[&str],
    difficulty: f64,
    url: Option<&str>,
) -> ContentItem {
    ContentItem {
        id: id("item", &format!("{}:{}", course.slug, key)),
        course_id: course.id,
        kind,
        title: title.to_string(),
        front: front.to_string(),
        back: back.to_string(),
        concept_ids: concept_keys
            .iter()
            .map(|k| id("concept", &format!("{}:{}", course.slug, k)))
            .collect(),
        tags: tags.iter().map(|s| s.to_string()).collect(),
        difficulty,
        source_url: url.map(|s| s.to_string()),
    }
}

fn physics_items(c: &Course) -> Vec<ContentItem> {
    vec![
        item(c, "fc-limits", ItemKind::Flashcard, "Limit definition",
            "What does lim(x->a) f(x) = L mean?",
            "f(x) gets arbitrarily close to L as x approaches a.",
            &["limits"], &["calculus"], 0.2, None),
        item(c, "fc-deriv", ItemKind::Flashcard, "Derivative as limit",
            "Define f'(x) as a limit.",
            "f'(x) = lim(h->0) [f(x+h) - f(x)] / h",
            &["derivatives", "limits"], &["calculus"], 0.35, None),
        item(c, "fc-lhopital", ItemKind::Flashcard, "L'Hopital's Rule",
            "When can you apply L'Hopital's Rule?",
            "For 0/0 or inf/inf indeterminate forms: lim f/g = lim f'/g'.",
            &["lhopital", "derivatives", "limits"], &["calculus"], 0.55, None),
        item(c, "doc-kinematics", ItemKind::Document, "Kinematics primer",
            "A short reading connecting derivatives to velocity and acceleration.",
            "Velocity is the derivative of position; acceleration is the derivative of velocity.",
            &["kinematics", "derivatives"], &["mechanics", "space"], 0.45,
            Some("https://example.com/kinematics")),
        item(c, "quiz-newton2", ItemKind::Quiz, "Newton's Second Law check",
            "If F = 10 N and m = 2 kg, what is a?",
            "a = F/m = 5 m/s^2",
            &["newton2", "kinematics"], &["mechanics"], 0.5, None),
        item(c, "vid-energy", ItemKind::Video, "Work-energy theorem",
            "Video: how work relates to kinetic energy.",
            "Net work equals the change in kinetic energy.",
            &["energy", "newton2"], &["mechanics", "space"], 0.65,
            Some("https://youtube.com/watch?v=example")),
    ]
}

fn japanese_items(c: &Course) -> Vec<ContentItem> {
    vec![
        item(c, "fc-mizu", ItemKind::Flashcard, "水", "水", "water (mizu)",
            &["mizu"], &["language", "cooking"], 0.15, None),
        item(c, "fc-taberu", ItemKind::Flashcard, "食べる", "食べる", "to eat (taberu)",
            &["taberu"], &["language", "cooking"], 0.2, None),
        item(c, "fc-iku", ItemKind::Flashcard, "行く", "行く", "to go (iku)",
            &["iku"], &["language", "travel"], 0.25, None),
        item(c, "doc-teform", ItemKind::Document, "て-form usage",
            "水を飲んで、食べて、行きます。",
            "Drinking water, eating, and going. (chained actions with て-form)",
            &["te-form", "taberu", "iku"], &["language", "grammar"], 0.45,
            Some("https://example.com/te-form")),
        item(c, "quiz-tara", ItemKind::Quiz, "たら conditional",
            "食べたら、行きます means?",
            "After eating, I will go.",
            &["tara", "te-form"], &["language", "grammar"], 0.55, None),
        item(c, "vid-keigo", ItemKind::Video, "敬語 introduction",
            "Video introducing polite speech patterns.",
            "召し上がる is the honorific form of 食べる.",
            &["keigo", "te-form"], &["language", "grammar"], 0.7,
            Some("https://youtube.com/watch?v=keigo")),
    ]
}


