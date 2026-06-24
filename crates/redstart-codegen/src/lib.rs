//! Code generation for Redstart.
//!
//! From a loaded [`ModuleTree`], [`generate`] produces the artifacts a subgraph
//! needs — `schema.graphql`, `subgraph.yaml`, and the AssemblyScript mappings —
//! from a *single* unified source of truth spanning every module. Drift between
//! them is impossible because they are all projections of the same AST.

#![forbid(unsafe_code)]

mod abi;
mod lower;
mod manifest;
mod mappings;
mod schema;

use abi::{resolve_abi_path, AbiIndex};
use lower::{resolve_type, EntityInfo, Env, RTy};
use manifest::ManifestInput;
use redstart_loader::ModuleTree;
use redstart_parser::ast::{Expr, EntityDecl, HandlerDecl, SourceDecl, TemplateDecl};
use std::collections::HashMap;
use std::path::PathBuf;

/// The artifacts produced by a Redstart build.
#[derive(Debug)]
pub struct Generated {
    /// Contents of `schema.graphql`.
    pub schema: String,
    /// Contents of `subgraph.yaml`.
    pub manifest: String,
    /// Contents of `src/mappings.ts`.
    pub mappings: String,
    /// ABIs to copy into `abis/`, as `(file_name, source_path)`.
    pub abi_copies: Vec<(String, PathBuf)>,
    /// Non-fatal warnings raised during generation.
    pub warnings: Vec<String>,
}

impl Generated {
    /// Write all artifacts to `out_dir`, mirroring a hand-written subgraph layout
    /// so the output is directly ejectable into `graph codegen` / `graph build`.
    ///
    /// # Errors
    /// Returns any IO error encountered while creating files.
    pub fn write_to(&self, out_dir: &std::path::Path) -> std::io::Result<()> {
        std::fs::create_dir_all(out_dir.join("src"))?;
        std::fs::create_dir_all(out_dir.join("abis"))?;

        std::fs::write(out_dir.join("schema.graphql"), &self.schema)?;
        std::fs::write(out_dir.join("subgraph.yaml"), &self.manifest)?;
        std::fs::write(out_dir.join("src/mappings.ts"), &self.mappings)?;

        for (file_name, src) in &self.abi_copies {
            if src.exists() {
                std::fs::copy(src, out_dir.join("abis").join(file_name))?;
            }
        }
        Ok(())
    }
}

/// Generate all artifacts from a loaded module tree.
#[must_use]
pub fn generate(tree: &ModuleTree) -> Generated {
    // Aggregate declarations across every module, in deterministic order.
    let mut entities: Vec<&EntityDecl> = Vec::new();
    let mut sources: Vec<&SourceDecl> = Vec::new();
    let mut templates: Vec<&TemplateDecl> = Vec::new();
    let mut handlers: Vec<&HandlerDecl> = Vec::new();

    let mut abi_index = AbiIndex::default();

    for module in tree.ordered() {
        let module_dir = module
            .file_path
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."));
        for abi in &module.program.abis {
            abi_index.insert(abi.name.name.clone(), resolve_abi_path(module_dir, &abi.path));
        }
        entities.extend(module.program.entities.iter());
        sources.extend(module.program.sources.iter());
        templates.extend(module.program.templates.iter());
        handlers.extend(module.program.handlers.iter());
    }

    let entity_names: Vec<String> = entities.iter().map(|e| e.name.name.clone()).collect();

    // ---- schema ----
    let schema = schema::render(&entities);

    // ---- manifest ----
    let input = ManifestInput {
        name: &tree.name,
        description: tree.description.as_deref(),
        sources: &sources,
        templates: &templates,
        handlers: &handlers,
        entity_names: &entity_names,
    };
    let (manifest_src, mut warnings) = manifest::render(&input, &mut abi_index);

    // ---- type environment for lowering ----
    let entity_table = build_entity_table(&entities);
    let source_abi = build_source_abi(&sources, &templates);
    let mut env = Env {
        entities: entity_table,
        source_abi,
        abis: &mut abi_index,
    };
    let (mappings_src, mut map_warnings) = mappings::render(&handlers, &entity_names, &mut env);
    warnings.append(&mut map_warnings);

    // ---- ABI copies ----
    let abi_copies: Vec<(String, PathBuf)> = env
        .abis
        .paths
        .iter()
        .map(|(name, path)| (format!("{name}.json"), path.clone()))
        .collect();

    Generated {
        schema,
        manifest: manifest_src,
        mappings: mappings_src,
        abi_copies,
        warnings,
    }
}

/// Build the entity-name -> field-types table used by the lowering pass.
fn build_entity_table(entities: &[&EntityDecl]) -> HashMap<String, EntityInfo> {
    // First pass: register every entity name so reference types resolve.
    let mut table: HashMap<String, EntityInfo> = entities
        .iter()
        .map(|e| (e.name.name.clone(), EntityInfo::default()))
        .collect();

    // Second pass: resolve field types against the now-complete name set.
    let resolved: Vec<(String, HashMap<String, RTy>)> = entities
        .iter()
        .map(|e| {
            let fields = e
                .fields
                .iter()
                .map(|f| (f.name.name.clone(), resolve_type(&f.ty, &table)))
                .collect();
            (e.name.name.clone(), fields)
        })
        .collect();

    for (name, fields) in resolved {
        table.insert(name, EntityInfo { fields });
    }
    table
}

/// Build the source/template-name -> ABI-name map.
fn build_source_abi(
    sources: &[&SourceDecl],
    templates: &[&TemplateDecl],
) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for s in sources {
        if let Some(abi) = abi_setting(&s.settings) {
            map.insert(s.name.name.clone(), abi);
        }
    }
    for t in templates {
        if let Some(abi) = abi_setting(&t.settings) {
            map.insert(t.name.name.clone(), abi);
        }
    }
    map
}

fn abi_setting(settings: &[redstart_parser::ast::Setting]) -> Option<String> {
    settings.iter().find(|s| s.key.name == "abi").and_then(|s| {
        if let Expr::Path { segments, .. } = &s.value {
            segments.last().map(|seg| seg.name.clone())
        } else {
            None
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn build_demo() -> Generated {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("src/abis")).unwrap();
        fs::write(
            dir.path().join("redstart.toml"),
            "[project]\nname = \"erc20\"\nentry = \"src/main.red\"",
        )
        .unwrap();
        fs::write(
            dir.path().join("src/abis/ERC20.json"),
            r#"[{"type":"event","name":"Transfer","inputs":[
                {"name":"from","type":"address","indexed":true},
                {"name":"to","type":"address","indexed":true},
                {"name":"value","type":"uint256","indexed":false}]}]"#,
        )
        .unwrap();
        fs::write(
            dir.path().join("src/main.red"),
            r#"
abi ERC20 from "./abis/ERC20.json"

entity Account {
  id: Id<Bytes>
  balance: BigInt
}

source Token {
  abi: ERC20
  network: mainnet
  address: 0x1234567890abcdef1234567890abcdef12345678
  startBlock: 1
}

handler on Token.Transfer(event) {
  let acct = Account.loadOrCreate(event.params.to, { balance: BigInt.zero })
  acct.balance = acct.balance + event.params.value
}
"#,
        )
        .unwrap();

        let tree = redstart_loader::load(dir.path()).unwrap();
        generate(&tree)
    }

    #[test]
    fn schema_and_manifest_reflect_source() {
        let gen = build_demo();
        assert!(gen.schema.contains("type Account @entity {"));
        assert!(gen.schema.contains("balance: BigInt!"));
        assert!(gen
            .manifest
            .contains("event: Transfer(indexed address,indexed address,uint256)"));
        assert!(gen.manifest.contains("handler: handleTransfer"));
        assert!(gen.manifest.contains("file: ./src/mappings.ts"));
    }

    #[test]
    fn lowering_produces_faithful_assemblyscript() {
        let gen = build_demo();
        let m = &gen.mappings;

        // loadOrCreate -> load + null-check + new + init.
        assert!(m.contains("let acct = Account.load(event.params.to)"));
        assert!(m.contains("if (acct == null) {"));
        assert!(m.contains("acct = new Account(event.params.to)"));
        assert!(m.contains("acct.balance = BigInt.zero()"));

        // BigInt `+` lowers to `.plus()`, never native `+`.
        assert!(m.contains("acct.balance = acct.balance.plus(event.params.value)"));

        // Auto-save dirty-tracked entity.
        assert!(m.contains("acct.save()"));

        // Imports.
        assert!(m.contains("import { Transfer as TransferEvent } from \"../generated/Token/ERC20\""));
        assert!(m.contains("import { Account } from \"../generated/schema\""));
        assert!(m.contains("import { BigInt } from \"@graphprotocol/graph-ts\""));

        assert!(gen.warnings.is_empty(), "warnings: {:?}", gen.warnings);
    }
}
