#[cfg(feature = "client")]
mod client;
mod message;
#[cfg(feature = "server")]
mod server;

type ClientId = String;

#[cfg(feature = "server")]
pub use server::*;

#[cfg(feature = "client")]
pub use client::*;
pub use message::*;
