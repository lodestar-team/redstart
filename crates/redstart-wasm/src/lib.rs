//! WebAssembly bindings for the Redstart compiler.
//!
//! Exposes a single [`compile`] entry point that takes Redstart source (and any
//! ABIs, in-memory) and returns the generated `schema.graphql`, `subgraph.yaml`,
//! and `mappings.ts` — or the diagnostics that stopped it. This is the engine
//! behind the browser playground: the same loader → checker → codegen pipeline
//! the CLI runs, with the filesystem swapped for strings.

use std::collections::HashMap;
use wasm_bindgen::prelude::*;

#[derive(serde::Serialize)]
struct CompileResult {
    ok: bool,
    schema: String,
    manifest: String,
    mappings: String,
    diagnostics: Vec<String>,
    warnings: Vec<String>,
}

impl CompileResult {
    fn failure(diagnostics: Vec<String>) -> Self {
        Self {
            ok: false,
            schema: String::new(),
            manifest: String::new(),
            mappings: String::new(),
            diagnostics,
            warnings: Vec::new(),
        }
    }
}

/// Compile Redstart source to the three subgraph artifacts.
///
/// `abis_json` is a JSON object mapping each ABI name (as declared by
/// `abi Name from "..."`) to its ABI — either a JSON array or a string. Pass
/// `"{}"` if the source declares no ABIs.
///
/// Returns an object: `{ ok, schema, manifest, mappings, diagnostics, warnings }`.
#[wasm_bindgen]
pub fn compile(source: &str, abis_json: &str) -> JsValue {
    let result = compile_inner(source, abis_json);
    serde_wasm_bindgen::to_value(&result).unwrap_or(JsValue::NULL)
}

fn compile_inner(source: &str, abis_json: &str) -> CompileResult {
    // Decode the in-memory ABIs. A missing/blank map is fine.
    let abi_texts = match parse_abis(abis_json) {
        Ok(map) => map,
        Err(e) => return CompileResult::failure(vec![format!("invalid ABI JSON: {e}")]),
    };

    // load (parse) → check → generate, mirroring the CLI but from strings.
    let tree = match redstart_loader::load_str("playground", source) {
        Ok(tree) => tree,
        Err(errors) => {
            return CompileResult::failure(errors.iter().map(ToString::to_string).collect())
        }
    };

    let mut checked = match redstart_checker::check_with_abis(&tree, &abi_texts) {
        Ok(checked) => checked,
        Err(diags) => return CompileResult::failure(diags),
    };

    let generated = redstart_codegen::generate(&tree, &mut checked);
    CompileResult {
        ok: true,
        schema: generated.schema,
        manifest: generated.manifest,
        mappings: generated.mappings,
        diagnostics: Vec::new(),
        warnings: generated.warnings,
    }
}

/// Parse the `{ "Name": <abi> }` object into name -> JSON-text, where each value
/// may be given as a JSON array or as an already-stringified ABI.
fn parse_abis(abis_json: &str) -> Result<HashMap<String, String>, String> {
    let trimmed = abis_json.trim();
    if trimmed.is_empty() {
        return Ok(HashMap::new());
    }
    let value: serde_json::Value = serde_json::from_str(trimmed).map_err(|e| e.to_string())?;
    let obj = match value {
        serde_json::Value::Object(map) => map,
        serde_json::Value::Null => return Ok(HashMap::new()),
        _ => return Err("expected a JSON object of { abiName: abi }".into()),
    };
    Ok(obj
        .into_iter()
        .map(|(name, abi)| {
            let text = match abi {
                serde_json::Value::String(s) => s,
                other => other.to_string(),
            };
            (name, text)
        })
        .collect())
}
