//! Golden snapshots of the generated artifacts for every worked example.
//!
//! These lock the emitted `schema.graphql`, `subgraph.yaml`, and `mappings.ts`
//! so any change to the AssemblyScript lowering shows up as a reviewable diff
//! rather than slipping through to surface three hours into someone's sync.
//!
//! Update intentionally after a deliberate lowering change with:
//!   INSTA_UPDATE=always cargo test -p redstart-codegen --test snapshots
//! (or `cargo insta review` if you have cargo-insta installed), then commit the
//! changed `.snap` files.

use std::path::{Path, PathBuf};

/// Path to an example project, resolved relative to this crate (so the test is
/// independent of the working directory CI or `cargo` happens to run it from).
fn example_dir(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples")
        .join(name)
}

/// Load → check → generate, returning the three text artifacts. The `abi_copies`
/// are deliberately dropped: they carry absolute, machine-specific paths.
fn generate(name: &str) -> (String, String, String) {
    let dir = example_dir(name);
    let tree = redstart_loader::load(&dir).unwrap_or_else(|e| panic!("load {name}: {e:?}"));
    let mut checked =
        redstart_checker::check(&tree).unwrap_or_else(|e| panic!("check {name}: {}", e.join("\n")));
    let g = redstart_codegen::generate(&tree, &mut checked);
    (g.schema, g.manifest, g.mappings)
}

/// Snapshot all three artifacts for one example under stable names.
fn snapshot_example(name: &str) {
    let (schema, manifest, mappings) = generate(name);
    insta::assert_snapshot!(format!("{name}__schema_graphql"), schema);
    insta::assert_snapshot!(format!("{name}__subgraph_yaml"), manifest);
    insta::assert_snapshot!(format!("{name}__mappings_ts"), mappings);
}

#[test]
fn erc20() {
    snapshot_example("erc20");
}

#[test]
fn factory() {
    snapshot_example("factory");
}

#[test]
fn file_metadata() {
    snapshot_example("file-metadata");
}

#[test]
fn horizon_indexer() {
    snapshot_example("horizon-indexer");
}
