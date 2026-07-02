//! Content ingestion: web scraping (Playwright) and YouTube caption extraction.
//!
//! The real implementations shell out to a Playwright service and the YouTube
//! captions API. Mocks return deterministic fixtures for tests / offline runs.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrapedContent {
    pub url: String,
    pub title: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transcript {
    pub video_id: String,
    pub language: String,
    pub segments: Vec<TranscriptSegment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptSegment {
    pub start: f64,
    pub duration: f64,
    pub text: String,
}

#[derive(Debug, thiserror::Error)]
pub enum IngestError {
    #[error("scrape error: {0}")]
    Scrape(String),
    #[error("caption error: {0}")]
    Caption(String),
}

#[async_trait]
pub trait Scraper: Send + Sync {
    async fn scrape(&self, url: &str) -> Result<ScrapedContent, IngestError>;
}

#[async_trait]
pub trait CaptionFetcher: Send + Sync {
    async fn fetch_captions(&self, video_id: &str) -> Result<Transcript, IngestError>;
}

/// Real scraper delegating to a Playwright microservice (configurable URL).
pub struct PlaywrightScraper {
    service_url: String,
    http: reqwest::Client,
}

impl PlaywrightScraper {
    pub fn new(service_url: impl Into<String>) -> Self {
        PlaywrightScraper {
            service_url: service_url.into(),
            http: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl Scraper for PlaywrightScraper {
    async fn scrape(&self, url: &str) -> Result<ScrapedContent, IngestError> {
        let resp = self
            .http
            .post(&self.service_url)
            .json(&serde_json::json!({ "url": url }))
            .send()
            .await
            .map_err(|e| IngestError::Scrape(e.to_string()))?;
        resp.json().await.map_err(|e| IngestError::Scrape(e.to_string()))
    }
}

/// Real YouTube caption fetcher.
pub struct YoutubeCaptionFetcher {
    api_key: String,
    http: reqwest::Client,
}

impl YoutubeCaptionFetcher {
    pub fn new(api_key: impl Into<String>) -> Self {
        YoutubeCaptionFetcher {
            api_key: api_key.into(),
            http: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl CaptionFetcher for YoutubeCaptionFetcher {
    async fn fetch_captions(&self, video_id: &str) -> Result<Transcript, IngestError> {
        // Real integration would use the YouTube Data API captions.list /
        // download endpoints (requires OAuth). Structure is stubbed here.
        let _ = (&self.api_key, &self.http);
        Err(IngestError::Caption(format!(
            "live YouTube caption fetch not configured for {video_id}"
        )))
    }
}

/// Mock scraper with deterministic fixtures keyed by URL substring.
pub struct MockScraper;

#[async_trait]
impl Scraper for MockScraper {
    async fn scrape(&self, url: &str) -> Result<ScrapedContent, IngestError> {
        Ok(ScrapedContent {
            url: url.to_string(),
            title: "Sample Article".into(),
            text: "This is comprehensible sample text about limits and derivatives in calculus."
                .into(),
        })
    }
}

pub struct MockCaptionFetcher;

#[async_trait]
impl CaptionFetcher for MockCaptionFetcher {
    async fn fetch_captions(&self, video_id: &str) -> Result<Transcript, IngestError> {
        Ok(Transcript {
            video_id: video_id.to_string(),
            language: "en".into(),
            segments: vec![
                TranscriptSegment { start: 0.0, duration: 3.0, text: "Welcome to the lesson.".into() },
                TranscriptSegment { start: 3.0, duration: 4.0, text: "Today we discuss limits.".into() },
            ],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mock_scraper_returns_content() {
        let s = MockScraper;
        let c = s.scrape("https://example.com/post").await.unwrap();
        assert_eq!(c.url, "https://example.com/post");
        assert!(!c.text.is_empty());
    }

    #[tokio::test]
    async fn mock_captions_have_segments() {
        let f = MockCaptionFetcher;
        let t = f.fetch_captions("abc123").await.unwrap();
        assert_eq!(t.video_id, "abc123");
        assert_eq!(t.segments.len(), 2);
    }
}
