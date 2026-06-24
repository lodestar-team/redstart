//! Code generation for Redstart.
//!
//! From a loaded [`ModuleTree`], [`generate`] produces the three artifacts a
//! subgraph needs — `schema.graphql`, `subgraph.yaml`, and the AssemblyScript
//! mappings — from a *single* unified source of truth. Drift between them is
//! impossible because they are all projections of the same AST.

#![forbid(unsafe_code)]

mod abi;
mod manifest;
mod mappings;
mod schema;

use abi::{resolve_abi_path, AbiIndex};
use manifest::ManifestInput;
use redstart_loader::ModuleTree;
use redstart_parser::ast::{EntityDecl, HandlerDecl, SourceDecl, TemplateDecl};
use std::collections::HashMap;

/// The artifacts produced by a Redstart build.
#[derive(Debug)]
pub struct Generated {
    /// Contents of `schema.graphql`.
    pub schema: String,
    /// Contents of `subgraph.yaml`.
    pub manifest: String,
    /// Contents of `mappings.ts` (Stage 0 skeleton).
    pub mappings: String,
    /// Non-fatal warnings raised during generation.
    pub warnings: Vec<String>,
}

impl Generated {
    /// Write all artifacts to `out_dir`, creating it if necessary.
    ///
    /// # Errors
    /// Returns any IO error encountered while creating files.
    pub fn write_to(&self, out_dir: &std::path::Path) -> std::io::Result<()> {
        std::fs::create_dir_all(out_dir)?;
        std::fs::write(out_dir.join("schema.graphql"), &self.schema)?;
        std::fs::write(out_dir.join("subgraph.yaml"), &self.manifest)?;
        std::fs::write(out_dir.join("mappings.ts"), &self.mappings)?;
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
    let mut abi_files: HashMap<String, String> = HashMap::new();

    for module in tree.ordered() {
        let module_dir = module
            .file_path
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."));

        for abi in &module.program.abis {
            abi_index.insert(
                abi.name.name.clone(),
                resolve_abi_path(module_dir, &abi.path),
            );
            abi_files.insert(abi.name.name.clone(), abi.path.clone());
        }

        entities.extend(module.program.entities.iter());
        sources.extend(module.program.sources.iter());
        templates.extend(module.program.templates.iter());
        handlers.extend(module.program.handlers.iter());
    }

    let entity_names: Vec<String> = entities.iter().map(|e| e.name.name.clone()).collect();

    let schema = schema::render(&entities);

    let input = ManifestInput {
        name: &tree.name,
        description: tree.description.as_deref(),
        sources: &sources,
        templates: &templates,
        handlers: &handlers,
        entity_names: &entity_names,
        abi_files: &abi_files,
    };
    let (manifest, warnings) = manifest::render(&input, &mut abi_index);

    let mappings = mappings::render(&handlers);

    Generated {
        schema,
        manifest,
        mappings,
        warnings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn generates_all_artifacts_for_a_project() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("src")).unwrap();
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
        let gen = generate(&tree);

        // Schema reflects the entity.
        assert!(gen.schema.contains("type Account @entity {"));
        assert!(gen.schema.contains("balance: BigInt!"));

        // Manifest resolved the real event signature (with `indexed`).
        assert!(gen
            .manifest
            .contains("event: Transfer(indexed address,indexed address,uint256)"));
        assert!(gen.manifest.contains("handler: handleTransfer"));
        assert!(gen.manifest.contains("address: \"0x1234567890abcdef1234567890abcdef12345678\""));

        // Mappings skeleton has the handler.
        assert!(gen.mappings.contains("export function handleTransfer"));

        // No warnings since the ABI resolved.
        assert!(gen.warnings.is_empty(), "warnings: {:?}", gen.warnings);
    }
}
