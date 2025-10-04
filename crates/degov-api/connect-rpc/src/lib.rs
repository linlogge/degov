pub mod error;
pub mod handler;
pub mod parts;
pub mod response;
pub mod router;

pub mod client;

// Re-export several crates
pub use futures;
pub use pbjson;
pub use pbjson_types;
pub use prost;
pub use serde;

pub mod prelude {
    pub use crate::error::*;
    pub use crate::parts::*;
    pub use crate::response::*;
    pub use crate::router::RpcRouterExt;
    pub use crate::client::*;
}
