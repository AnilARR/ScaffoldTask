//! Domain core for the N+1 comprehensible-input learning platform.
//!
//! This crate is intentionally free of I/O (no DB, no HTTP) so the learning
//! algorithms — FSRS freshness and N+1 selection — are fully unit-testable in
//! isolation.

pub mod fsrs;
pub mod models;
pub mod nplus1;

pub use fsrs::{Fsrs, MemoryState, Rating, ScheduleResult};
pub use nplus1::{Candidate, ComprehensionMap, ScoredCandidate, SelectionParams, Selector};
