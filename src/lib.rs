//! The library behind the `statac` program, a static site generator.
//!
//! It is Statac's own, there for the program and its tests, and not for
//! use in other projects: its modules aren't documented here, and any of
//! them can change in any release. To install Statac, see
//! [statac.dev/install](https://statac.dev/install/); to use it, see
//! [statac.dev](https://statac.dev/).

#![warn(unreachable_pub)]
// The modules are public only for the program and the tests, so their docs
// link to private items freely.
#![allow(rustdoc::private_intra_doc_links)]

mod address;
#[doc(hidden)]
pub mod album;
#[doc(hidden)]
pub mod assets;
#[doc(hidden)]
pub mod build;
#[doc(hidden)]
pub mod cards;
#[doc(hidden)]
pub mod cli;
#[doc(hidden)]
pub mod config;
#[doc(hidden)]
pub mod content;
mod data;
#[doc(hidden)]
pub mod date;
#[doc(hidden)]
pub mod dates;
#[doc(hidden)]
pub mod described;
mod disk;
mod elsewhere;
mod engines;
mod feeds;
#[doc(hidden)]
pub mod fetch;
mod fetched;
mod highlight;
mod hooks;
mod html;
#[doc(hidden)]
pub mod ids;
#[doc(hidden)]
pub mod import;
mod json;
mod language;
mod linked;
mod links;
mod lists;
mod lsp;
#[doc(hidden)]
pub mod markdown;
#[doc(hidden)]
pub mod new;
mod origin;
#[doc(hidden)]
pub mod pictures;
#[doc(hidden)]
pub mod platform;
#[doc(hidden)]
pub mod problem;
#[doc(hidden)]
pub mod publish;
mod published;
#[doc(hidden)]
pub mod reads;
#[doc(hidden)]
pub mod refresh;
#[doc(hidden)]
pub mod report;
mod rewrite;
#[doc(hidden)]
pub mod search;
#[doc(hidden)]
pub mod serve;
mod site;
mod slug;
#[doc(hidden)]
pub mod source;
#[doc(hidden)]
pub mod stamp;
mod suggest;
mod templates;
#[cfg(test)]
mod testing;
mod unnamed;
mod unsaved;
mod value;
mod wording;
#[doc(hidden)]
pub mod yaml;
