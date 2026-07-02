//! AnkiConnect integration.
//!
//! The real implementation talks to the AnkiConnect HTTP API (default
//! http://localhost:8765) to read the user's existing decks/cards so we can
//! establish a comprehension baseline. A mock implementation is used for tests
//! and offline E2E runs.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnkiCard {
    pub note_id: i64,
    pub front: String,
    pub back: String,
    pub deck: String,
    /// AnkiConnect "interval" (days) — a proxy for how well-learned the card is.
    pub interval: i64,
    /// Ease factor (per mille), also a maturity proxy.
    pub ease: i64,
}

#[derive(Debug, thiserror::Error)]
pub enum AnkiError {
    #[error("anki transport error: {0}")]
    Transport(String),
    #[error("anki api error: {0}")]
    Api(String),
}

#[async_trait]
pub trait AnkiClient: Send + Sync {
    async fn list_decks(&self) -> Result<Vec<String>, AnkiError>;
    async fn cards_in_deck(&self, deck: &str) -> Result<Vec<AnkiCard>, AnkiError>;
}

/// Real AnkiConnect client over HTTP.
pub struct HttpAnkiClient {
    endpoint: String,
    http: reqwest::Client,
}

impl HttpAnkiClient {
    pub fn new(endpoint: impl Into<String>) -> Self {
        HttpAnkiClient {
            endpoint: endpoint.into(),
            http: reqwest::Client::new(),
        }
    }

    async fn invoke(&self, action: &str, params: serde_json::Value) -> Result<serde_json::Value, AnkiError> {
        let body = serde_json::json!({ "action": action, "version": 6, "params": params });
        let resp = self
            .http
            .post(&self.endpoint)
            .json(&body)
            .send()
            .await
            .map_err(|e| AnkiError::Transport(e.to_string()))?;
        let val: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| AnkiError::Transport(e.to_string()))?;
        if let Some(err) = val.get("error").and_then(|e| e.as_str()) {
            if !err.is_empty() {
                return Err(AnkiError::Api(err.to_string()));
            }
        }
        Ok(val.get("result").cloned().unwrap_or(serde_json::Value::Null))
    }
}

#[async_trait]
impl AnkiClient for HttpAnkiClient {
    async fn list_decks(&self) -> Result<Vec<String>, AnkiError> {
        let result = self.invoke("deckNames", serde_json::json!({})).await?;
        Ok(serde_json::from_value(result).unwrap_or_default())
    }

    async fn cards_in_deck(&self, deck: &str) -> Result<Vec<AnkiCard>, AnkiError> {
        let query = serde_json::json!({ "query": format!("deck:\"{}\"", deck) });
        let ids_val = self.invoke("findCards", query).await?;
        let ids: Vec<i64> = serde_json::from_value(ids_val).unwrap_or_default();
        if ids.is_empty() {
            return Ok(vec![]);
        }
        let info = self
            .invoke("cardsInfo", serde_json::json!({ "cards": ids }))
            .await?;
        let raw: Vec<serde_json::Value> = serde_json::from_value(info).unwrap_or_default();
        let cards = raw
            .into_iter()
            .map(|c| AnkiCard {
                note_id: c.get("note").and_then(|v| v.as_i64()).unwrap_or_default(),
                front: c
                    .get("fields")
                    .and_then(|f| f.get("Front"))
                    .and_then(|v| v.get("value"))
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
                back: c
                    .get("fields")
                    .and_then(|f| f.get("Back"))
                    .and_then(|v| v.get("value"))
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
                deck: deck.to_string(),
                interval: c.get("interval").and_then(|v| v.as_i64()).unwrap_or_default(),
                ease: c.get("factor").and_then(|v| v.as_i64()).unwrap_or(2500),
            })
            .collect();
        Ok(cards)
    }
}

/// Deterministic mock used for tests / offline demo.
pub struct MockAnkiClient {
    pub decks: Vec<(String, Vec<AnkiCard>)>,
}

impl MockAnkiClient {
    pub fn with_sample() -> Self {
        let cards = vec![
            AnkiCard { note_id: 1, front: "水".into(), back: "water (mizu)".into(), deck: "Japanese::Core".into(), interval: 45, ease: 2500 },
            AnkiCard { note_id: 2, front: "食べる".into(), back: "to eat (taberu)".into(), deck: "Japanese::Core".into(), interval: 30, ease: 2400 },
            AnkiCard { note_id: 3, front: "行く".into(), back: "to go (iku)".into(), deck: "Japanese::Core".into(), interval: 12, ease: 2100 },
        ];
        MockAnkiClient { decks: vec![("Japanese::Core".into(), cards)] }
    }
}

#[async_trait]
impl AnkiClient for MockAnkiClient {
    async fn list_decks(&self) -> Result<Vec<String>, AnkiError> {
        Ok(self.decks.iter().map(|(n, _)| n.clone()).collect())
    }

    async fn cards_in_deck(&self, deck: &str) -> Result<Vec<AnkiCard>, AnkiError> {
        Ok(self
            .decks
            .iter()
            .find(|(n, _)| n == deck)
            .map(|(_, c)| c.clone())
            .unwrap_or_default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mock_lists_and_reads() {
        let c = MockAnkiClient::with_sample();
        let decks = c.list_decks().await.unwrap();
        assert_eq!(decks, vec!["Japanese::Core"]);
        let cards = c.cards_in_deck("Japanese::Core").await.unwrap();
        assert_eq!(cards.len(), 3);
        assert_eq!(cards[0].front, "水");
    }

    #[tokio::test]
    async fn unknown_deck_is_empty() {
        let c = MockAnkiClient::with_sample();
        assert!(c.cards_in_deck("Nope").await.unwrap().is_empty());
    }
}
