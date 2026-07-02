//! Shared application state injected into every handler.

use std::sync::Arc;

use crate::repo::Repository;
use crate::services::Services;

#[derive(Clone)]
pub struct AppState {
    pub repo: Arc<dyn Repository>,
    pub services: Services,
}

impl AppState {
    pub fn new(repo: Arc<dyn Repository>, services: Services) -> Self {
        AppState { repo, services }
    }
}
