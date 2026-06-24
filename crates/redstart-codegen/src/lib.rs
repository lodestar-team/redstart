//! Code generation for Redstart.
//!
//! From a loaded [`ModuleTree`] and a validated [`Checked`] symbol table (from
//! `redstart-checker`), [`generate`] produces the artifacts a subgraph needs —
//! `schema.graphql`, `subgraph.yaml`, and the AssemblyScript mappings — from a
//! *single* unified source of truth spanning every module. Drift between them is
//! impossible because they are all projections of the same AST, and the resolved
//! types are computed once by the checker and shared here.

#![forbid(unsafe_code)]

mod lower;
mod manifest;
mod mappings;
mod schema;

use lower::Env;
use manifest::ManifestInput;
use redstart_checker::Checked;
use redstart_loader::ModuleTree;
use redstart_parser::ast::{EntityDecl, HandlerDecl, SourceDecl, TemplateDecl};
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

/// Generate all artifacts from a loaded module tree and its checked symbol table.
#[must_use]
pub fn generate(tree: &ModuleTree, checked: &mut Checked) -> Generated {
    // Aggregate declarations across every module, in deterministic order.
    let mut entities: Vec<&EntityDecl> = Vec::new();
    let mut sources: Vec<&SourceDecl> = Vec::new();
    let mut templates: Vec<&TemplateDecl> = Vec::new();
    let mut handlers: Vec<&HandlerDecl> = Vec::new();

    for module in tree.ordered() {
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
    let (manifest_src, mut warnings) = manifest::render(&input, &mut checked.abis);

    // ---- mappings (lowering uses the checked type tables) ----
    let (mappings_src, map_warnings) = {
        let mut env = Env {
            entities: checked.entities.clone(),
            source_abi: checked.source_abi.clone(),
            abis: &mut checked.abis,
        };
        mappings::render(&handlers, &entity_names, &mut env)
    };
    warnings.extend(map_warnings);

    // ---- ABI copies ----
    let abi_copies: Vec<(String, PathBuf)> = checked
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn build(main_red: &str, abi: &str) -> Generated {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("src/abis")).unwrap();
        fs::write(
            dir.path().join("redstart.toml"),
            "[project]\nname = \"erc20\"\nentry = \"src/main.red\"",
        )
        .unwrap();
        fs::write(dir.path().join("src/abis/ERC20.json"), abi).unwrap();
        fs::write(dir.path().join("src/main.red"), main_red).unwrap();

        let tree = redstart_loader::load(dir.path()).unwrap();
        let mut checked = redstart_checker::check(&tree).expect("check should pass");
        generate(&tree, &mut checked)
    }

    const TRANSFER_ABI: &str = r#"[{"type":"event","name":"Transfer","inputs":[
        {"name":"from","type":"address","indexed":true},
        {"name":"to","type":"address","indexed":true},
        {"name":"value","type":"uint256","indexed":false}]}]"#;

    fn build_demo() -> Generated {
        build(
            r#"
abi ERC20 from "./abis/ERC20.json"
entity Account { id: Id<Bytes> balance: BigInt }
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
            TRANSFER_ABI,
        )
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
        assert!(m.contains("let acct = Account.load(event.params.to)"));
        assert!(m.contains("if (acct == null) {"));
        assert!(m.contains("acct = new Account(event.params.to)"));
        assert!(m.contains("acct.balance = BigInt.zero()"));
        assert!(m.contains("acct.balance = acct.balance.plus(event.params.value)"));
        assert!(m.contains("acct.save()"));
        assert!(m.contains("import { Transfer as TransferEvent } from \"../generated/Token/ERC20\""));
        assert!(m.contains("import { Account } from \"../generated/schema\""));
        assert!(m.contains("import { BigInt } from \"@graphprotocol/graph-ts\""));
        assert!(gen.warnings.is_empty(), "warnings: {:?}", gen.warnings);
    }

    const CALLS_ABI: &str = r#"[
      {"type":"event","name":"Approval","inputs":[
        {"name":"owner","type":"address","indexed":true},
        {"name":"spender","type":"address","indexed":true},
        {"name":"value","type":"uint256","indexed":false}]},
      {"type":"function","name":"balanceOf","stateMutability":"view",
        "inputs":[{"name":"account","type":"address"}],
        "outputs":[{"name":"","type":"uint256"}]}
    ]"#;

    #[test]
    fn contract_calls_and_match_lower_correctly() {
        let gen = build(
            r#"
abi ERC20 from "./abis/ERC20.json"
entity Account { id: Id<Bytes> balance: BigInt }
source Token {
  abi: ERC20
  network: mainnet
  address: 0x1234567890abcdef1234567890abcdef12345678
  startBlock: 1
}
handler on Token.Approval(event) {
  let result = ERC20.bind(event.address).balanceOf(event.params.owner)
  match result {
    Ok(bal) => {
      let owner = Account.loadOrCreate(event.params.owner, { balance: BigInt.zero })
      owner.balance = bal
    }
    Err(e) => {}
  }
}
"#,
            CALLS_ABI,
        );
        let m = &gen.mappings;
        assert!(m.contains("let result = ERC20.bind(event.address).try_balanceOf(event.params.owner)"));
        assert!(m.contains("if (!result.reverted) {"));
        assert!(m.contains("let bal = result.value"));
        assert!(m.contains("owner.save()"));
        assert!(!m.contains("else {"));
        assert!(m.contains("import { ERC20, Approval as ApprovalEvent }"));
        assert!(gen.warnings.is_empty(), "warnings: {:?}", gen.warnings);
    }
}
