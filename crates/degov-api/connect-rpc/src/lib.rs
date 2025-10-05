pub mod client;
pub mod server;

// Re-export crates for generated code
pub use pbjson;
pub use pbjson_types;
pub use prost;
pub use serde;

pub mod prelude {
    pub use crate::client::*;
    pub use crate::server::error::*;
    pub use crate::server::parts::*;
    pub use crate::server::response::*;
    pub use crate::server::router::RpcRouterExt;
}
