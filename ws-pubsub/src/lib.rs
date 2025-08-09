#[cfg(feature = "client")]
mod client;
mod future;
mod logger;
mod message;
#[cfg(feature = "server")]
mod server;

pub mod topic;

#[cfg(feature = "server")]
pub use server::*;

#[cfg(feature = "client")]
pub use client::*;
pub use message::*;
pub use topic::*;

#[async_trait::async_trait]
pub trait ServerFunctions {
    type Error: std::fmt::Debug;
    async fn subscribe(topic: String) -> Result<String, Self::Error>;
    async fn unsubscribe(subscription_id: String) -> Result<(), Self::Error>;
}

pub type Topic = String;
