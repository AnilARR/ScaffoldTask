//! AI API integration for grading complexity, extracting tags, and generating
//! comprehension tests. Real impl targets an OpenAI-compatible chat endpoint;
//! mock impl returns deterministic heuristics for tests / offline runs.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexityGrade {
    /// 0..1 difficulty estimate.
    pub difficulty: f64,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McqQuestion {
    pub prompt: String,
    pub choices: Vec<String>,
    pub answer_index: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum AiError {
    #[error("ai transport error: {0}")]
    Transport(String),
    #[error("ai response error: {0}")]
    Response(String),
}

#[async_trait]
pub trait AiClient: Send + Sync {
    /// Grade a piece of content's difficulty and extract interest tags.
    async fn grade_complexity(&self, text: &str) -> Result<ComplexityGrade, AiError>;
    /// Generate a multiple-choice comprehension test targeting a concept.
    async fn generate_mcq(&self, concept: &str, context: &str) -> Result<McqQuestion, AiError>;
}

/// Real OpenAI-compatible client.
pub struct HttpAiClient {
    endpoint: String,
    api_key: String,
    model: String,
    http: reqwest::Client,
}

impl HttpAiClient {
    pub fn new(endpoint: impl Into<String>, api_key: impl Into<String>, model: impl Into<String>) -> Self {
        HttpAiClient {
            endpoint: endpoint.into(),
            api_key: api_key.into(),
            model: model.into(),
            http: reqwest::Client::new(),
        }
    }

    async fn chat(&self, system: &str, user: &str) -> Result<String, AiError> {
        let body = serde_json::json!({
            "model": self.model,
            "messages": [
                { "role": "system", "content": system },
                { "role": "user", "content": user }
            ],
            "temperature": 0.2,
            "response_format": { "type": "json_object" }
        });
        let resp = self
            .http
            .post(&self.endpoint)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| AiError::Transport(e.to_string()))?;
        let val: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| AiError::Transport(e.to_string()))?;
        val.get("choices")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("message"))
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| AiError::Response("missing content".into()))
    }
}

#[async_trait]
impl AiClient for HttpAiClient {
    async fn grade_complexity(&self, text: &str) -> Result<ComplexityGrade, AiError> {
        let out = self
            .chat(
                "You grade learning content. Reply JSON: {\"difficulty\": number 0..1, \"tags\": string[]}.",
                text,
            )
            .await?;
        serde_json::from_str(&out).map_err(|e| AiError::Response(e.to_string()))
    }

    async fn generate_mcq(&self, concept: &str, context: &str) -> Result<McqQuestion, AiError> {
        let out = self
            .chat(
                "Write one multiple-choice comprehension question. Reply JSON: {\"prompt\": string, \"choices\": string[4], \"answer_index\": number}.",
                &format!("Concept: {concept}\nContext: {context}"),
            )
            .await?;
        serde_json::from_str(&out).map_err(|e| AiError::Response(e.to_string()))
    }
}

/// Deterministic heuristic mock.
pub struct MockAiClient;

#[async_trait]
impl AiClient for MockAiClient {
    async fn grade_complexity(&self, text: &str) -> Result<ComplexityGrade, AiError> {
        // Heuristic: longer text + rarer characters => harder.
        let len = text.chars().count() as f64;
        let difficulty = (len / 400.0).clamp(0.1, 0.95);
        let mut tags = Vec::new();
        let lower = text.to_lowercase();
        for (kw, tag) in [
            ("limit", "calculus"),
            ("force", "mechanics"),
            ("energy", "mechanics"),
            ("verb", "grammar"),
            ("particle", "grammar"),
        ] {
            if lower.contains(kw) {
                tags.push(tag.to_string());
            }
        }
        if tags.is_empty() {
            tags.push("general".into());
        }
        Ok(ComplexityGrade { difficulty, tags })
    }

    async fn generate_mcq(&self, concept: &str, _context: &str) -> Result<McqQuestion, AiError> {
        Ok(McqQuestion {
            prompt: format!("Which best describes '{concept}'?"),
            choices: vec![
                format!("The correct definition of {concept}"),
                "An unrelated distractor".into(),
                "Another distractor".into(),
                "A third distractor".into(),
            ],
            answer_index: 0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mock_grades_and_tags() {
        let ai = MockAiClient;
        let g = ai.grade_complexity("The limit of a function as x approaches zero").await.unwrap();
        assert!(g.difficulty > 0.0 && g.difficulty <= 1.0);
        assert!(g.tags.contains(&"calculus".to_string()));
    }

    #[tokio::test]
    async fn mock_generates_valid_mcq() {
        let ai = MockAiClient;
        let q = ai.generate_mcq("Newton's second law", "F = ma").await.unwrap();
        assert_eq!(q.choices.len(), 4);
        assert!(q.answer_index < q.choices.len());
    }
}
