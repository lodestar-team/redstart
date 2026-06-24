//! Lightweight ABI JSON reading, just enough to compute canonical event
//! signatures for the generated `subgraph.yaml`.
//!
//! graph-node wants event handler signatures in the form
//! `Transfer(indexed address,indexed address,uint256)` — note the `indexed`
//! qualifiers. We read only what we need and stay tolerant of missing files so
//! `redstart build` still produces a (placeholder-annotated) manifest offline.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// An index of ABIs by their in-language name, with resolved file paths.
#[derive(Debug, Default)]
pub struct AbiIndex {
    /// Map from ABI name to the resolved JSON path on disk.
    pub paths: HashMap<String, PathBuf>,
    /// Cache of parsed event signatures, keyed by `(abi_name, event_name)`.
    cache: HashMap<(String, String), Option<String>>,
}

impl AbiIndex {
    /// Register an ABI name with a resolved path.
    pub fn insert(&mut self, name: String, path: PathBuf) {
        self.paths.insert(name, path);
    }

    /// Compute the canonical signature for `event_name` in the ABI named
    /// `abi_name`, e.g. `Transfer(indexed address,indexed address,uint256)`.
    ///
    /// Returns `None` if the ABI file is missing/unreadable or the event is not
    /// present, so the caller can emit a placeholder.
    pub fn event_signature(&mut self, abi_name: &str, event_name: &str) -> Option<String> {
        let key = (abi_name.to_string(), event_name.to_string());
        if let Some(cached) = self.cache.get(&key) {
            return cached.clone();
        }
        let result = self.compute_signature(abi_name, event_name);
        self.cache.insert(key, result.clone());
        result
    }

    fn compute_signature(&self, abi_name: &str, event_name: &str) -> Option<String> {
        let path = self.paths.get(abi_name)?;
        let text = std::fs::read_to_string(path).ok()?;
        let json: serde_json::Value = serde_json::from_str(&text).ok()?;
        let items = json.as_array()?;

        for item in items {
            if item.get("type").and_then(|t| t.as_str()) != Some("event") {
                continue;
            }
            if item.get("name").and_then(|n| n.as_str()) != Some(event_name) {
                continue;
            }
            let inputs = item.get("inputs").and_then(|i| i.as_array())?;
            let params: Vec<String> = inputs
                .iter()
                .map(|inp| {
                    let ty = inp
                        .get("type")
                        .and_then(|t| t.as_str())
                        .unwrap_or("bytes");
                    let indexed = inp
                        .get("indexed")
                        .and_then(serde_json::Value::as_bool)
                        .unwrap_or(false);
                    if indexed {
                        format!("indexed {ty}")
                    } else {
                        ty.to_string()
                    }
                })
                .collect();
            return Some(format!("{event_name}({})", params.join(",")));
        }
        None
    }
}

/// Resolve an ABI path declared in a module against that module's directory.
#[must_use]
pub fn resolve_abi_path(module_dir: &Path, declared: &str) -> PathBuf {
    let p = Path::new(declared);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        module_dir.join(p)
    }
}
