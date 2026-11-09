//! hourglazs: a personal project tracker.
//!
//! Pipeline: [`data`] reads, validates and saves the JSON files → [`domain`]
//! indexes the task tree and schedule → [`render`] produces HTML. [`edit`]
//! holds the changes the local app can make, and [`serve`] exposes them over
//! a localhost-only HTTP server.

pub mod cli;
pub mod data;
pub mod domain;
pub mod edit;
pub mod model;
pub mod render;
pub mod serve;
