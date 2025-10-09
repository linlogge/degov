pub mod client;
pub mod server;

// Shared modules at top level
pub mod error;
pub mod encoding;
pub mod protocol;
pub mod streaming;
pub mod response;

// Re-export crates for generated code
pub use pbjson;
pub use pbjson_types;
pub use prost;
pub use serde;

pub mod prelude {
    pub use crate::client::*;
    pub use crate::server::error::*;
    pub use crate::server::parts::*;
    pub use crate::server::router::RpcRouterExt;
    pub use crate::encoding::*;
    pub use crate::protocol::*;
}
