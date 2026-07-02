//! N+1 comprehensible-input selection engine.
//!
//! Core idea from the spec: serve content where ~90% of the tokens/concepts are
//! already comprehensible to the user and ~10% are new ("i+1"). We score every
//! candidate document against the user's current comprehension map plus their
//! interest tags, and pick the best-fitting material.

use std::collections::HashMap;
use uuid::Uuid;

/// The user's comprehension of a concept/word, expressed as freshness in 0..1.
/// (Typically derived from FSRS retrievability.)
pub type ComprehensionMap = HashMap<Uuid, f64>;

/// A candidate piece of comprehensible input (document, video, quiz, sentence).
#[derive(Debug, Clone)]
pub struct Candidate {
    pub id: Uuid,
    /// Concept/word ids that appear in this content.
    pub concept_ids: Vec<Uuid>,
    /// Tags describing the content's topic/interest area.
    pub tags: Vec<String>,
    /// Nominal difficulty 0..1 used as a tie-breaker / coarse gate.
    pub difficulty: f64,
}

/// Tunable parameters for the selection algorithm. These are exactly the kind of
/// gradations the spec expects PostHog experimentation to tune.
#[derive(Debug, Clone)]
pub struct SelectionParams {
    /// Freshness at/above which a concept is considered "known".
    pub known_threshold: f64,
    /// Target fraction of known concepts (comprehensible input ratio).
    pub target_comprehensible_ratio: f64,
    /// Acceptable window around the target ratio.
    pub ratio_tolerance: f64,
    /// Weight of interest-tag overlap in the final score.
    pub interest_weight: f64,
    /// Weight of ratio-fit in the final score.
    pub ratio_weight: f64,
}

impl Default for SelectionParams {
    fn default() -> Self {
        SelectionParams {
            known_threshold: 0.6,
            target_comprehensible_ratio: 0.9,
            ratio_tolerance: 0.15,
            interest_weight: 0.35,
            ratio_weight: 0.65,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ScoredCandidate {
    pub id: Uuid,
    pub score: f64,
    pub comprehensible_ratio: f64,
    pub new_concepts: usize,
    pub interest_overlap: f64,
}

pub struct Selector {
    params: SelectionParams,
}

impl Selector {
    pub fn new(params: SelectionParams) -> Self {
        Selector { params }
    }

    /// Fraction of a candidate's concepts the user already knows.
    fn comprehensible_ratio(&self, c: &Candidate, comp: &ComprehensionMap) -> (f64, usize) {
        if c.concept_ids.is_empty() {
            return (1.0, 0);
        }
        let mut known = 0usize;
        for id in &c.concept_ids {
            let freshness = comp.get(id).copied().unwrap_or(0.0);
            if freshness >= self.params.known_threshold {
                known += 1;
            }
        }
        let total = c.concept_ids.len();
        let new = total - known;
        (known as f64 / total as f64, new)
    }

    fn interest_overlap(&self, c: &Candidate, interests: &[String]) -> f64 {
        if interests.is_empty() || c.tags.is_empty() {
            return 0.0;
        }
        let matches = c
            .tags
            .iter()
            .filter(|t| interests.iter().any(|i| i.eq_ignore_ascii_case(t)))
            .count();
        matches as f64 / c.tags.len() as f64
    }

    /// Score a single candidate. Higher is better; 0 means unusable.
    pub fn score(
        &self,
        c: &Candidate,
        comp: &ComprehensionMap,
        interests: &[String],
    ) -> ScoredCandidate {
        let (ratio, new_concepts) = self.comprehensible_ratio(c, comp);
        let interest = self.interest_overlap(c, interests);

        // Ratio fit: peaks at the target, falls off linearly outside tolerance.
        let dist = (ratio - self.params.target_comprehensible_ratio).abs();
        let ratio_fit = if dist <= self.params.ratio_tolerance {
            1.0 - (dist / self.params.ratio_tolerance) * 0.25
        } else {
            (1.0 - dist).max(0.0)
        };

        // Content with zero new concepts teaches nothing; penalize it.
        let novelty_gate = if new_concepts == 0 { 0.2 } else { 1.0 };

        let score = novelty_gate
            * (self.params.ratio_weight * ratio_fit + self.params.interest_weight * interest);

        ScoredCandidate {
            id: c.id,
            score,
            comprehensible_ratio: ratio,
            new_concepts,
            interest_overlap: interest,
        }
    }

    /// Rank candidates best-first.
    pub fn rank(
        &self,
        candidates: &[Candidate],
        comp: &ComprehensionMap,
        interests: &[String],
    ) -> Vec<ScoredCandidate> {
        let mut scored: Vec<ScoredCandidate> = candidates
            .iter()
            .map(|c| self.score(c, comp, interests))
            .collect();
        scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        scored
    }

    /// Pick the single best candidate, if any is usable (score > 0).
    pub fn best<'a>(
        &self,
        candidates: &'a [Candidate],
        comp: &ComprehensionMap,
        interests: &[String],
    ) -> Option<ScoredCandidate> {
        self.rank(candidates, comp, interests)
            .into_iter()
            .find(|s| s.score > 0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cid() -> Uuid {
        Uuid::new_v4()
    }

    fn cand(concepts: Vec<Uuid>, tags: &[&str]) -> Candidate {
        Candidate {
            id: cid(),
            concept_ids: concepts,
            tags: tags.iter().map(|s| s.to_string()).collect(),
            difficulty: 0.5,
        }
    }

    /// Build a candidate with `known` known concepts and `new` new concepts,
    /// returning the candidate and a matching comprehension map.
    fn make(known: usize, new: usize) -> (Candidate, ComprehensionMap) {
        let mut comp = ComprehensionMap::new();
        let mut ids = Vec::new();
        for _ in 0..known {
            let id = cid();
            comp.insert(id, 0.95);
            ids.push(id);
        }
        for _ in 0..new {
            ids.push(cid());
        }
        (cand(ids, &["physics"]), comp)
    }

    #[test]
    fn perfect_ratio_scores_high() {
        let s = Selector::new(SelectionParams::default());
        let (c, comp) = make(9, 1);
        let sc = s.score(&c, &comp, &["physics".into()]);
        assert!((sc.comprehensible_ratio - 0.9).abs() < 1e-9);
        assert_eq!(sc.new_concepts, 1);
        assert!(sc.score > 0.7, "score was {}", sc.score);
    }

    #[test]
    fn all_known_is_penalized_for_no_novelty() {
        let s = Selector::new(SelectionParams::default());
        let (c, comp) = make(10, 0);
        let sc = s.score(&c, &comp, &["physics".into()]);
        assert_eq!(sc.new_concepts, 0);
        let (c2, comp2) = make(9, 1);
        let sc2 = s.score(&c2, &comp2, &["physics".into()]);
        assert!(sc2.score > sc.score);
    }

    #[test]
    fn too_hard_content_scores_low() {
        let s = Selector::new(SelectionParams::default());
        // 3 known, 7 new -> ratio 0.3, far from target.
        let (c, comp) = make(3, 7);
        let sc = s.score(&c, &comp, &["physics".into()]);
        let (good, gcomp) = make(9, 1);
        let gsc = s.score(&good, &gcomp, &["physics".into()]);
        assert!(gsc.score > sc.score);
    }

    #[test]
    fn interest_overlap_breaks_ties() {
        let s = Selector::new(SelectionParams::default());
        let (mut c1, comp) = make(9, 1);
        c1.tags = vec!["cooking".into()];
        let mut c2 = c1.clone();
        c2.id = cid();
        c2.tags = vec!["physics".into()];
        // Same comprehension shape, different interest match.
        let ranked = s.rank(&[c1, c2.clone()], &comp, &["physics".into()]);
        assert_eq!(ranked[0].id, c2.id);
    }

    #[test]
    fn best_returns_none_when_nothing_usable() {
        let s = Selector::new(SelectionParams::default());
        let empty: Vec<Candidate> = vec![];
        let comp = ComprehensionMap::new();
        assert!(s.best(&empty, &comp, &[]).is_none());
    }

    #[test]
    fn rank_is_sorted_descending() {
        let s = Selector::new(SelectionParams::default());
        let (c1, comp1) = make(9, 1);
        let (c2, _c2comp) = make(2, 8);
        // Merge comprehension so both candidates are evaluated against it.
        let mut comp = comp1.clone();
        for (k, v) in _c2comp {
            comp.insert(k, v);
        }
        let ranked = s.rank(&[c1, c2], &comp, &[]);
        assert!(ranked[0].score >= ranked[1].score);
    }
}
