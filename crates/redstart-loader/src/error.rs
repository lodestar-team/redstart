//! Loader errors.

use crate::manifest::ManifestError;
use std::path::PathBuf;

/// An error encountered while loading a Redstart project.
#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    /// A file could not be read.
    #[error("could not read `{path}`: {source}")]
    Io {
        /// The offending path.
        path: PathBuf,
        /// The underlying IO error.
        source: std::io::Error,
    },

    /// No `redstart.toml` was found in the given directory.
    #[error("no `redstart.toml` found in `{dir}`")]
    NoManifest {
        /// The directory searched.
        dir: PathBuf,
    },

    /// The manifest was invalid.
    #[error(transparent)]
    Manifest(#[from] ManifestError),

    /// The entry file declared in the manifest does not exist.
    #[error("entry file `{path}` does not exist")]
    MissingEntry {
        /// The expected entry path.
        path: PathBuf,
    },

    /// Lexing failed in a module.
    #[error("lex error in `{file}`:\n{report}")]
    Lex {
        /// The file that failed.
        file: PathBuf,
        /// The rendered diagnostic.
        report: String,
    },

    /// Parsing failed in a module.
    #[error("parse error in `{file}`:\n{report}")]
    Parse {
        /// The file that failed.
        file: PathBuf,
        /// The rendered diagnostic.
        report: String,
    },

    /// A `mod` declaration referenced a file that could not be found.
    #[error("module `{name}` not found; looked for {searched:?}")]
    ModuleNotFound {
        /// The module name.
        name: String,
        /// The candidate paths searched.
        searched: Vec<PathBuf>,
    },

    /// A `mod` declaration was ambiguous (both `name.red` and `name/mod.red` exist).
    #[error("module `{name}` is ambiguous; both {candidates:?} exist")]
    AmbiguousModule {
        /// The module name.
        name: String,
        /// The conflicting candidates.
        candidates: Vec<PathBuf>,
    },

    /// A cycle was detected in the module import graph.
    #[error("circular module dependency: {cycle:?}")]
    CircularDependency {
        /// The files involved in the cycle.
        cycle: Vec<String>,
    },
}
