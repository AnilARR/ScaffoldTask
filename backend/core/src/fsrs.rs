//! FSRS (Free Spaced Repetition Scheduler) v4.5 implementation.
//!
//! This drives the "freshness" tracking described in the product spec: every
//! time a user interacts with an item (flashcard, quiz, comprehensible-input
//! document), we update the item's memory state and compute the next optimal
//! review time. Freshness at any moment is derived from retrievability.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

/// Grade a user gives (or that we infer) for an interaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Rating {
    Again = 1,
    Hard = 2,
    Good = 3,
    Easy = 4,
}

impl Rating {
    pub fn from_i16(v: i16) -> Option<Self> {
        match v {
            1 => Some(Rating::Again),
            2 => Some(Rating::Hard),
            3 => Some(Rating::Good),
            4 => Some(Rating::Easy),
            _ => None,
        }
    }
}

/// The persisted memory state for a single (user, item) pair.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MemoryState {
    /// Memory stability in days (time for retrievability to fall to 90%).
    pub stability: f64,
    /// Difficulty in [1, 10].
    pub difficulty: f64,
    /// Number of reviews so far.
    pub reps: u32,
    /// Number of lapses (Again ratings) so far.
    pub lapses: u32,
    /// Timestamp of the last review, if any.
    pub last_review: Option<DateTime<Utc>>,
}

impl Default for MemoryState {
    fn default() -> Self {
        MemoryState {
            stability: 0.0,
            difficulty: 0.0,
            reps: 0,
            lapses: 0,
            last_review: None,
        }
    }
}

/// Default FSRS-4.5 weights (17 parameters).
pub const DEFAULT_WEIGHTS: [f64; 17] = [
    0.4072, 1.1829, 3.1262, 15.4722, 7.2102, 0.5316, 1.0651, 0.0234, 1.616, 0.1544, 1.0824,
    1.9813, 0.0953, 0.2975, 2.2042, 0.2407, 2.9466,
];

/// Decay/factor constants for the FSRS forgetting curve.
const DECAY: f64 = -0.5;
/// FACTOR = 0.9^(1/DECAY) - 1
const FACTOR: f64 = 19.0 / 81.0;

#[derive(Debug, Clone)]
pub struct Fsrs {
    w: [f64; 17],
    /// Desired retention (probability of recall at review time).
    request_retention: f64,
    /// Cap on interval length in days.
    maximum_interval: i64,
}

impl Default for Fsrs {
    fn default() -> Self {
        Fsrs {
            w: DEFAULT_WEIGHTS,
            request_retention: 0.9,
            maximum_interval: 36500,
        }
    }
}

/// Result of scheduling a review.
#[derive(Debug, Clone, Copy)]
pub struct ScheduleResult {
    pub state: MemoryState,
    pub interval_days: i64,
    pub due: DateTime<Utc>,
}

impl Fsrs {
    pub fn new(weights: [f64; 17], request_retention: f64, maximum_interval: i64) -> Self {
        Fsrs {
            w: weights,
            request_retention,
            maximum_interval,
        }
    }

    /// Retrievability (0..1): probability the user still recalls the item now.
    /// This is the inverse of "staleness" — freshness = retrievability.
    pub fn retrievability(&self, state: &MemoryState, now: DateTime<Utc>) -> f64 {
        match state.last_review {
            None => 0.0,
            Some(last) => {
                if state.stability <= 0.0 {
                    return 0.0;
                }
                let elapsed_days = (now - last).num_seconds() as f64 / 86_400.0;
                let t = elapsed_days.max(0.0);
                (1.0 + FACTOR * t / state.stability).powf(DECAY)
            }
        }
    }

    fn init_difficulty(&self, rating: Rating) -> f64 {
        let d = self.w[4] - (self.w[5] * (rating as i32 as f64 - 1.0)).exp() + 1.0;
        d.clamp(1.0, 10.0)
    }

    fn init_stability(&self, rating: Rating) -> f64 {
        self.w[(rating as usize) - 1].max(0.1)
    }

    fn next_difficulty(&self, d: f64, rating: Rating) -> f64 {
        let delta = -self.w[6] * (rating as i32 as f64 - 3.0);
        let next = d + delta * ((10.0 - d) / 9.0);
        // Mean reversion toward the "Easy" initial difficulty.
        let d_easy = self.init_difficulty(Rating::Easy);
        let reverted = self.w[7] * d_easy + (1.0 - self.w[7]) * next;
        reverted.clamp(1.0, 10.0)
    }

    fn next_recall_stability(&self, d: f64, s: f64, r: f64, rating: Rating) -> f64 {
        let hard_penalty = if rating == Rating::Hard { self.w[15] } else { 1.0 };
        let easy_bonus = if rating == Rating::Easy { self.w[16] } else { 1.0 };
        s * (1.0
            + (self.w[8]).exp()
                * (11.0 - d)
                * s.powf(-self.w[9])
                * (((1.0 - r) * self.w[10]).exp() - 1.0)
                * hard_penalty
                * easy_bonus)
    }

    fn next_forget_stability(&self, d: f64, s: f64, r: f64) -> f64 {
        self.w[11]
            * d.powf(-self.w[12])
            * ((s + 1.0).powf(self.w[13]) - 1.0)
            * ((1.0 - r) * self.w[14]).exp()
    }

    fn next_interval(&self, stability: f64) -> i64 {
        let ivl = (stability / FACTOR)
            * (self.request_retention.powf(1.0 / DECAY) - 1.0);
        (ivl.round() as i64).clamp(1, self.maximum_interval)
    }

    /// Apply a review at `now` with the given `rating`, returning the new
    /// memory state, the next interval, and the due date.
    pub fn schedule(
        &self,
        state: &MemoryState,
        rating: Rating,
        now: DateTime<Utc>,
    ) -> ScheduleResult {
        let mut next = *state;

        if state.reps == 0 || state.last_review.is_none() {
            // First-ever review.
            next.difficulty = self.init_difficulty(rating);
            next.stability = self.init_stability(rating);
        } else {
            let r = self.retrievability(state, now);
            next.difficulty = self.next_difficulty(state.difficulty, rating);
            next.stability = if rating == Rating::Again {
                next.lapses += 1;
                self.next_forget_stability(next.difficulty, state.stability, r)
            } else {
                self.next_recall_stability(next.difficulty, state.stability, r, rating)
            };
        }

        next.stability = next.stability.max(0.1);
        next.reps += 1;
        next.last_review = Some(now);

        let interval_days = self.next_interval(next.stability);
        let due = now + Duration::days(interval_days);

        ScheduleResult {
            state: next,
            interval_days,
            due,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fsrs() -> Fsrs {
        Fsrs::default()
    }

    #[test]
    fn first_review_sets_initial_state() {
        let f = fsrs();
        let now = Utc::now();
        let r = f.schedule(&MemoryState::default(), Rating::Good, now);
        assert!(r.state.stability > 0.0);
        assert!(r.state.difficulty >= 1.0 && r.state.difficulty <= 10.0);
        assert_eq!(r.state.reps, 1);
        assert_eq!(r.state.lapses, 0);
        assert!(r.interval_days >= 1);
    }

    #[test]
    fn easy_gives_longer_interval_than_hard() {
        let f = fsrs();
        let now = Utc::now();
        let easy = f.schedule(&MemoryState::default(), Rating::Easy, now);
        let hard = f.schedule(&MemoryState::default(), Rating::Hard, now);
        assert!(
            easy.interval_days >= hard.interval_days,
            "easy {} should be >= hard {}",
            easy.interval_days,
            hard.interval_days
        );
    }

    #[test]
    fn again_increments_lapses_and_shrinks_stability() {
        let f = fsrs();
        let now = Utc::now();
        let first = f.schedule(&MemoryState::default(), Rating::Good, now);
        let later = now + Duration::days(first.interval_days);
        let lapse = f.schedule(&first.state, Rating::Again, later);
        assert_eq!(lapse.state.lapses, 1);
        assert!(lapse.state.stability < first.state.stability);
    }

    #[test]
    fn retrievability_decays_over_time() {
        let f = fsrs();
        let now = Utc::now();
        let r = f.schedule(&MemoryState::default(), Rating::Good, now);
        let fresh = f.retrievability(&r.state, now);
        let stale = f.retrievability(&r.state, now + Duration::days(365));
        assert!(fresh > stale);
        assert!((0.0..=1.0).contains(&fresh));
        assert!((0.0..=1.0).contains(&stale));
    }

    #[test]
    fn retrievability_at_stability_is_near_target() {
        let f = fsrs();
        let now = Utc::now();
        let mut st = MemoryState::default();
        st.stability = 10.0;
        st.last_review = Some(now);
        // At t == stability, retrievability should equal ~0.9 by construction.
        let r = f.retrievability(&st, now + Duration::days(10));
        assert!((r - 0.9).abs() < 0.02, "r={} expected ~0.9", r);
    }

    #[test]
    fn unreviewed_item_has_zero_retrievability() {
        let f = fsrs();
        assert_eq!(f.retrievability(&MemoryState::default(), Utc::now()), 0.0);
    }

    #[test]
    fn repeated_good_reviews_grow_stability() {
        let f = fsrs();
        let mut now = Utc::now();
        let mut st = MemoryState::default();
        let mut prev = 0.0;
        for _ in 0..5 {
            let r = f.schedule(&st, Rating::Good, now);
            assert!(r.state.stability >= prev);
            prev = r.state.stability;
            st = r.state;
            now = now + Duration::days(r.interval_days);
        }
        assert!(prev > 1.0);
    }
}
