//! The `redstart.toml` project manifest.
//!
//! A Redstart project is described by a single `redstart.toml`. The manifest
//! pins the target spec/api versions so the developer never has to juggle the
//! `specVersion`/`apiVersion`/`graph-cli`/`graph-ts` compatibility matrix by
//! hand — Redstart owns it.

use serde::Deserialize;
use std::path::Path;

/// A parsed `redstart.toml`.
#[derive(Debug, Clone, Deserialize)]
pub struct ProjectManifest {
    /// The `[project]` table.
    pub project: ProjectSection,
}

/// The `[project]` table of the manifest.
#[derive(Debug, Clone, Deserialize)]
pub struct ProjectSection {
    /// The project name (also the subgraph name).
    pub name: String,
    /// The entry source file, relative to the manifest.
    #[serde(default = "default_entry")]
    pub entry: String,
    /// Optional human description, surfaced in the generated manifest.
    #[serde(default)]
    pub description: Option<String>,
    /// The output directory for generated artifacts.
    #[serde(default = "default_out_dir")]
    pub out_dir: String,
}

fn default_entry() -> String {
    "src/main.red".to_string()
}

fn default_out_dir() -> String {
    "build".to_string()
}

impl ProjectManifest {
    /// Load and parse a manifest from disk.
    ///
    /// # Errors
    /// Returns an error if the file cannot be read or is not valid TOML.
    pub fn load(path: &Path) -> Result<Self, ManifestError> {
        let text = std::fs::read_to_string(path).map_err(|source| ManifestError::Io {
            path: path.display().to_string(),
            source,
        })?;
        toml::from_str(&text).map_err(|source| ManifestError::Parse {
            path: path.display().to_string(),
            source,
        })
    }
}

/// An error loading a manifest.
#[derive(Debug, thiserror::Error)]
pub enum ManifestError {
    /// The manifest file could not be read.
    #[error("could not read manifest `{path}`: {source}")]
    Io {
        /// The manifest path.
        path: String,
        /// The underlying IO error.
        source: std::io::Error,
    },
    /// The manifest was not valid TOML.
    #[error("invalid manifest `{path}`: {source}")]
    Parse {
        /// The manifest path.
        path: String,
        /// The underlying TOML error.
        source: toml::de::Error,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn parses_minimal_manifest() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("redstart.toml");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(f, "[project]\nname = \"my-subgraph\"").unwrap();

        let m = ProjectManifest::load(&path).unwrap();
        assert_eq!(m.project.name, "my-subgraph");
        assert_eq!(m.project.entry, "src/main.red");
        assert_eq!(m.project.out_dir, "build");
    }
}
