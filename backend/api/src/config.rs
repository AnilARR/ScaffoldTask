//! Runtime configuration, resolved from environment variables with sane
//! defaults for local development.

#[derive(Debug, Clone)]
pub struct Settings {
    pub bind_addr: String,
    /// If set, use Postgres; otherwise use the in-memory store (offline demo).
    pub database_url: Option<String>,
    /// If true, force all-mock external services regardless of other config.
    pub use_mocks: bool,
    pub anki_endpoint: String,
    pub ai_endpoint: String,
    pub ai_api_key: Option<String>,
    pub ai_model: String,
    pub scraper_url: String,
    pub youtube_api_key: Option<String>,
}

impl Settings {
    pub fn from_env() -> Self {
        let get = |k: &str| std::env::var(k).ok().filter(|v| !v.is_empty());
        Settings {
            bind_addr: get("BIND_ADDR").unwrap_or_else(|| "0.0.0.0:8080".into()),
            database_url: get("DATABASE_URL"),
            use_mocks: get("USE_MOCKS").map(|v| v == "1" || v == "true").unwrap_or(true),
            anki_endpoint: get("ANKI_ENDPOINT").unwrap_or_else(|| "http://localhost:8765".into()),
            ai_endpoint: get("AI_ENDPOINT")
                .unwrap_or_else(|| "https://api.openai.com/v1/chat/completions".into()),
            ai_api_key: get("AI_API_KEY"),
            ai_model: get("AI_MODEL").unwrap_or_else(|| "gpt-4o-mini".into()),
            scraper_url: get("SCRAPER_URL").unwrap_or_else(|| "http://localhost:9200/scrape".into()),
            youtube_api_key: get("YOUTUBE_API_KEY"),
        }
    }
}
