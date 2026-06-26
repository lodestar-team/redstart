//! Lightweight ABI JSON reading.
//!
//! We read two things: the canonical event signature for the manifest
//! (`Transfer(indexed address,indexed address,uint256)` — note the `indexed`
//! qualifiers graph-node wants) and the typed parameter list, which the lowering
//! pass uses to infer that `event.params.value` is a `BigInt`. We stay tolerant
//! of missing files so `redstart build` still works offline.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// A single decoded event parameter.
#[derive(Debug, Clone)]
pub struct EventParam {
    /// The parameter name.
    pub name: String,
    /// The Solidity type, e.g. `uint256`, `address`.
    pub sol_type: String,
    /// Whether the parameter is indexed.
    pub indexed: bool,
}

/// An index of ABIs by their in-language name, with resolved file paths.
#[derive(Debug, Default)]
pub struct AbiIndex {
    /// Map from ABI name to the resolved JSON path on disk.
    pub paths: HashMap<String, PathBuf>,
    /// In-memory ABI JSON, keyed by name. Preferred over `paths` when present —
    /// this is how the WASM playground (no filesystem) supplies ABIs.
    texts: HashMap<String, String>,
    /// Cache of decoded event parameter lists, keyed by `(abi_name, event_name)`.
    cache: HashMap<(String, String), Option<Vec<EventParam>>>,
}

impl AbiIndex {
    /// Register an ABI name with a resolved path.
    pub fn insert(&mut self, name: String, path: PathBuf) {
        self.paths.insert(name, path);
    }

    /// Register an ABI name with its JSON contents directly (no filesystem).
    pub fn insert_text(&mut self, name: String, json: String) {
        self.texts.insert(name, json);
    }

    /// The raw ABI JSON for `abi_name`: in-memory text if registered, else read
    /// from the resolved path. Returns `None` if neither is available.
    fn json_text(&self, abi_name: &str) -> Option<String> {
        if let Some(text) = self.texts.get(abi_name) {
            return Some(text.clone());
        }
        let path = self.paths.get(abi_name)?;
        std::fs::read_to_string(path).ok()
    }

    /// The decoded parameters for `event_name` in ABI `abi_name`, cached.
    pub fn event_params(&mut self, abi_name: &str, event_name: &str) -> Option<Vec<EventParam>> {
        let key = (abi_name.to_string(), event_name.to_string());
        if let Some(cached) = self.cache.get(&key) {
            return cached.clone();
        }
        let result = self.read_event(abi_name, event_name);
        self.cache.insert(key, result.clone());
        result
    }

    /// The canonical event signature for the manifest, or `None` if unresolved.
    pub fn event_signature(&mut self, abi_name: &str, event_name: &str) -> Option<String> {
        let params = self.event_params(abi_name, event_name)?;
        let rendered: Vec<String> = params
            .iter()
            .map(|p| {
                if p.indexed {
                    format!("indexed {}", p.sol_type)
                } else {
                    p.sol_type.clone()
                }
            })
            .collect();
        Some(format!("{event_name}({})", rendered.join(",")))
    }

    /// Whether the ABI named `abi_name` resolves to a readable, parseable file.
    /// Lets the checker distinguish "event missing from ABI" (error) from
    /// "ABI file not available" (can't check — stay quiet).
    pub fn readable(&self, abi_name: &str) -> bool {
        self.json_text(abi_name)
            .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
            .is_some_and(|v| v.is_array())
    }

    /// Whether `name` is a callable function in ABI `abi_name`.
    pub fn is_function(&self, abi_name: &str, name: &str) -> bool {
        self.function_outputs(abi_name, name).is_some()
    }

    /// The decoded input parameters of function `name` in ABI `abi_name`.
    pub fn function_inputs(&self, abi_name: &str, name: &str) -> Option<Vec<EventParam>> {
        self.read_function_params(abi_name, name, "inputs")
    }

    /// The decoded output parameters of function `name` in ABI `abi_name`.
    pub fn function_output_params(&self, abi_name: &str, name: &str) -> Option<Vec<EventParam>> {
        self.read_function_params(abi_name, name, "outputs")
    }

    /// The canonical call-handler signature for the manifest, e.g.
    /// `transfer(address,uint256)` — canonical types, never `indexed`.
    pub fn function_signature(&self, abi_name: &str, name: &str) -> Option<String> {
        let inputs = self.function_inputs(abi_name, name)?;
        let types: Vec<String> = inputs.iter().map(|p| p.sol_type.clone()).collect();
        Some(format!("{name}({})", types.join(",")))
    }

    fn read_function_params(
        &self,
        abi_name: &str,
        name: &str,
        which: &str,
    ) -> Option<Vec<EventParam>> {
        let text = self.json_text(abi_name)?;
        let json: serde_json::Value = serde_json::from_str(&text).ok()?;
        for item in json.as_array()? {
            if item.get("type").and_then(|t| t.as_str()) != Some("function") {
                continue;
            }
            if item.get("name").and_then(|n| n.as_str()) != Some(name) {
                continue;
            }
            let params = item.get(which).and_then(|i| i.as_array())?;
            return Some(
                params
                    .iter()
                    .enumerate()
                    .map(|(i, inp)| EventParam {
                        name: inp
                            .get("name")
                            .and_then(|n| n.as_str())
                            .filter(|s| !s.is_empty())
                            .map_or_else(
                                || {
                                    if which == "outputs" {
                                        format!("value{i}")
                                    } else {
                                        format!("param{i}")
                                    }
                                },
                                str::to_string,
                            ),
                        sol_type: inp
                            .get("type")
                            .and_then(|t| t.as_str())
                            .unwrap_or("bytes")
                            .to_string(),
                        indexed: false,
                    })
                    .collect(),
            );
        }
        None
    }

    /// The Solidity output types of function `name` in ABI `abi_name`, if any.
    pub fn function_outputs(&self, abi_name: &str, name: &str) -> Option<Vec<String>> {
        let text = self.json_text(abi_name)?;
        let json: serde_json::Value = serde_json::from_str(&text).ok()?;
        for item in json.as_array()? {
            if item.get("type").and_then(|t| t.as_str()) != Some("function") {
                continue;
            }
            if item.get("name").and_then(|n| n.as_str()) != Some(name) {
                continue;
            }
            let outputs = item.get("outputs").and_then(|o| o.as_array())?;
            return Some(
                outputs
                    .iter()
                    .map(|o| {
                        o.get("type")
                            .and_then(|t| t.as_str())
                            .unwrap_or("bytes")
                            .to_string()
                    })
                    .collect(),
            );
        }
        None
    }

    fn read_event(&self, abi_name: &str, event_name: &str) -> Option<Vec<EventParam>> {
        let text = self.json_text(abi_name)?;
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
            return Some(
                inputs
                    .iter()
                    .enumerate()
                    .map(|(i, inp)| EventParam {
                        name: inp
                            .get("name")
                            .and_then(|n| n.as_str())
                            .filter(|s| !s.is_empty())
                            .map_or_else(|| format!("param{i}"), str::to_string),
                        sol_type: inp
                            .get("type")
                            .and_then(|t| t.as_str())
                            .unwrap_or("bytes")
                            .to_string(),
                        indexed: inp
                            .get("indexed")
                            .and_then(serde_json::Value::as_bool)
                            .unwrap_or(false),
                    })
                    .collect(),
            );
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
