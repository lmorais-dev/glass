use serde::de::DeserializeOwned;
use std::pin::Pin;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct User {
    pub id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct GreetAllResponseItem {
    pub user_id: u64,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct GreetAllRequest {
    pub people: Vec<User>,
}

#[async_trait::async_trait]
pub trait Greeter {
    type Error: Send + Sync + serde::Serialize + serde::Deserialize<'static> + 'static;
    type OutputStream<T>: futures::stream::Stream<Item = T> + Send + Sync
    where
        T: serde::Serialize + DeserializeOwned + Send + Sync;

    async fn say_hello(&self, request: User) -> Result<String, Self::Error>
    where
        User: serde::Serialize + DeserializeOwned + Send + Sync,
        String: serde::Serialize + DeserializeOwned + Send + Sync;

    async fn greet_all(
        &self,
        request: GreetAllRequest,
    ) -> Result<Self::OutputStream<GreetAllResponseItem>, Self::Error>
    where
        GreetAllRequest: serde::Serialize + DeserializeOwned + Send + Sync;
}

pub struct GreeterImpl;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Error)]
pub enum GreeterError {
    #[error("Invalid user ID: {id}")]
    InvalidUserId { id: u64 },

    #[error("Empty request")]
    EmptyRequest,
}

#[async_trait::async_trait]
impl Greeter for GreeterImpl {
    type Error = GreeterError;
    type OutputStream<T>
        = Pin<Box<dyn futures::stream::Stream<Item = T> + Send + Sync>>
    where
        T: serde::Serialize + DeserializeOwned + Send + Sync;

    async fn say_hello(&self, request: User) -> Result<String, Self::Error> {
        if request.id == 0 {
            return Err(GreeterError::InvalidUserId { id: request.id });
        }

        Ok(format!("Hello, user {}!", request.id))
    }

    async fn greet_all(
        &self,
        request: GreetAllRequest,
    ) -> Result<Self::OutputStream<GreetAllResponseItem>, Self::Error> {
        if request.people.is_empty() {
            return Err(GreeterError::EmptyRequest);
        }

        let responses: Vec<GreetAllResponseItem> = request
            .people
            .into_iter()
            .map(|user| GreetAllResponseItem {
                user_id: user.id,
                message: format!("Hello, user {}!", user.id),
            })
            .collect();

        let stream = futures::stream::iter(responses);
        Ok(Box::pin(stream))
    }
}
