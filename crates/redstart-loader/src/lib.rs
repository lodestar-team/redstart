//! Project manifest parsing and multi-file module loading for Redstart.
//!
//! Entry point: [`load`], which accepts either a single `.red` file, a
//! `redstart.toml`, or a project directory, and returns a fully parsed
//! [`ModuleTree`].

#![forbid(unsafe_code)]

mod error;
mod manifest;
mod tree;

pub use error::LoadError;
pub use manifest::{ManifestError, ProjectManifest, ProjectSection};
pub use tree::{load, ModulePath, ModuleTree, ParsedModule};
