//! AWS abstraction for user management (Cognito-style) and master database
//! bucketing (S3-style object store). Real impls would use the AWS SDK; the
//! mock is an in-memory store for E2E/local runs.

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Debug, thiserror::Error)]
pub enum AwsError {
    #[error("aws error: {0}")]
    Op(String),
}

/// User identity provider (maps to Cognito in production).
#[async_trait]
pub trait IdentityProvider: Send + Sync {
    async fn create_user(&self, email: &str) -> Result<String, AwsError>;
    async fn get_user(&self, id: &str) -> Result<Option<String>, AwsError>;
}

/// Object store for the master/telemetry data lake (maps to S3).
#[async_trait]
pub trait ObjectStore: Send + Sync {
    async fn put(&self, bucket: &str, key: &str, body: Vec<u8>) -> Result<(), AwsError>;
    async fn get(&self, bucket: &str, key: &str) -> Result<Option<Vec<u8>>, AwsError>;
}

/// In-memory mock identity provider.
#[derive(Default)]
pub struct MockIdentityProvider {
    users: Mutex<HashMap<String, String>>,
}

#[async_trait]
impl IdentityProvider for MockIdentityProvider {
    async fn create_user(&self, email: &str) -> Result<String, AwsError> {
        let id = uuid::Uuid::new_v4().to_string();
        self.users.lock().unwrap().insert(id.clone(), email.to_string());
        Ok(id)
    }

    async fn get_user(&self, id: &str) -> Result<Option<String>, AwsError> {
        Ok(self.users.lock().unwrap().get(id).cloned())
    }
}

/// In-memory mock object store.
#[derive(Default)]
pub struct MockObjectStore {
    objects: Mutex<HashMap<(String, String), Vec<u8>>>,
}

#[async_trait]
impl ObjectStore for MockObjectStore {
    async fn put(&self, bucket: &str, key: &str, body: Vec<u8>) -> Result<(), AwsError> {
        self.objects
            .lock()
            .unwrap()
            .insert((bucket.to_string(), key.to_string()), body);
        Ok(())
    }

    async fn get(&self, bucket: &str, key: &str) -> Result<Option<Vec<u8>>, AwsError> {
        Ok(self
            .objects
            .lock()
            .unwrap()
            .get(&(bucket.to_string(), key.to_string()))
            .cloned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn identity_roundtrip() {
        let idp = MockIdentityProvider::default();
        let id = idp.create_user("a@b.com").await.unwrap();
        assert_eq!(idp.get_user(&id).await.unwrap(), Some("a@b.com".to_string()));
    }

    #[tokio::test]
    async fn object_store_roundtrip() {
        let s = MockObjectStore::default();
        s.put("bkt", "k", b"hi".to_vec()).await.unwrap();
        assert_eq!(s.get("bkt", "k").await.unwrap(), Some(b"hi".to_vec()));
        assert_eq!(s.get("bkt", "missing").await.unwrap(), None);
    }
}
