// Changed from oxhttp 0.3.3 for Statac: exports only what the server uses.
//! The HTTP model encoded in Rust type system.
//!
//! This reexport the [`http`](https://docs.rs/http) crate except for [`Body`].
//!
//! The main entry points are [`Request`] and [`Response`].
mod body;

pub(crate) use body::Body;
pub(crate) use http::*;
