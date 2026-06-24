//! The `redstart` command-line toolchain.
//!
//! A single binary, Gleam-style: `new`, `check`, `build`. More subcommands
//! (`dev`, `test`, `fmt`, `lsp`) follow in later stages.

#![forbid(unsafe_code)]

mod fmt;

use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// Redstart: a language for authoring The Graph subgraphs.
#[derive(Parser)]
#[command(name = "redstart", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Scaffold a new Redstart project.
    New {
        /// The project name (a directory of this name is created).
        name: String,
    },
    /// Parse and validate a project without emitting artifacts.
    Check {
        /// Path to a project directory, `redstart.toml`, or a `.red` file.
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Build a project: emit schema.graphql, subgraph.yaml, and mappings.ts.
    Build {
        /// Path to a project directory, `redstart.toml`, or a `.red` file.
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Run `test` blocks natively against a mock store (no WASM, no Docker).
    Test {
        /// Path to a project directory, `redstart.toml`, or a `.red` file.
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Format `.red` files into the canonical layout.
    Fmt {
        /// A `.red` file or a directory to format recursively.
        #[arg(default_value = ".")]
        path: PathBuf,
        /// Check formatting without writing; exit non-zero if any file differs.
        #[arg(long)]
        check: bool,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.command {
        Command::New { name } => cmd_new(&name),
        Command::Check { path } => cmd_check(&path),
        Command::Build { path } => cmd_build(&path),
        Command::Test { path } => cmd_test(&path),
        Command::Fmt { path, check } => cmd_fmt(&path, check),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(msg) => {
            eprintln!("{msg}");
            ExitCode::FAILURE
        }
    }
}

fn cmd_check(path: &Path) -> Result<(), String> {
    let tree = load(path)?;
    check(&tree)?;
    let modules = tree.modules.len();
    let entities: usize = tree.modules.values().map(|m| m.program.entities.len()).sum();
    let handlers: usize = tree.modules.values().map(|m| m.program.handlers.len()).sum();
    println!(
        "✓ {} — {modules} module(s), {entities} entit(ies), {handlers} handler(s), no errors",
        tree.name
    );
    Ok(())
}

fn cmd_build(path: &Path) -> Result<(), String> {
    let tree = load(path)?;
    let mut checked = check(&tree)?;
    let generated = redstart_codegen::generate(&tree, &mut checked);

    generated
        .write_to(&tree.out_dir)
        .map_err(|e| format!("failed to write build output to {}: {e}", tree.out_dir.display()))?;

    for warning in &generated.warnings {
        eprintln!("warning: {warning}");
    }

    println!("✓ built {} → {}", tree.name, tree.out_dir.display());
    println!("  • schema.graphql");
    println!("  • subgraph.yaml");
    println!("  • src/mappings.ts");
    println!("  • abis/ ({} file(s))", generated.abi_copies.len());
    Ok(())
}

/// Load a project, formatting any errors into a single message.
fn load(path: &Path) -> Result<redstart_loader::ModuleTree, String> {
    redstart_loader::load(path).map_err(|errors| {
        errors
            .iter()
            .map(std::string::ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n\n")
    })
}

/// Run semantic analysis, joining rendered diagnostics into one message.
fn check(tree: &redstart_loader::ModuleTree) -> Result<redstart_checker::Checked, String> {
    redstart_checker::check(tree).map_err(|reports| reports.join("\n"))
}

fn cmd_test(path: &Path) -> Result<(), String> {
    let tree = load(path)?;
    let checked = check(&tree)?;
    let report = redstart_test::run_tests(&tree, &checked);

    if report.results.is_empty() {
        println!("no tests found (add `test \"…\" {{ … }}` blocks)");
        return Ok(());
    }

    for r in &report.results {
        match &r.outcome {
            redstart_test::Outcome::Pass => println!("  \x1b[32m✓\x1b[0m {}", r.name),
            redstart_test::Outcome::Fail { message, location } => {
                let at = location.as_deref().map_or(String::new(), |l| format!(" ({l})"));
                println!("  \x1b[31m✗\x1b[0m {}{at}\n      {message}", r.name);
            }
        }
    }

    let total = report.results.len();
    let passed = report.passed();
    if report.ok() {
        println!("\n✓ {passed}/{total} passed");
        Ok(())
    } else {
        Err(format!("\n✗ {}/{total} failed", total - passed))
    }
}

fn cmd_fmt(path: &Path, check: bool) -> Result<(), String> {
    let files = collect_red_files(path)?;
    if files.is_empty() {
        return Err(format!("no `.red` files found at {}", path.display()));
    }

    let mut changed = Vec::new();
    for file in &files {
        let src = std::fs::read_to_string(file)
            .map_err(|e| format!("could not read {}: {e}", file.display()))?;
        let formatted = fmt::format(&src);
        if formatted != src {
            changed.push(file.clone());
            if !check {
                std::fs::write(file, &formatted)
                    .map_err(|e| format!("could not write {}: {e}", file.display()))?;
            }
        }
    }

    if check {
        if changed.is_empty() {
            println!("✓ all {} file(s) already formatted", files.len());
            Ok(())
        } else {
            for f in &changed {
                println!("would reformat {}", f.display());
            }
            Err(format!("{} file(s) need formatting", changed.len()))
        }
    } else {
        if changed.is_empty() {
            println!("✓ {} file(s) already formatted", files.len());
        } else {
            for f in &changed {
                println!("formatted {}", f.display());
            }
        }
        Ok(())
    }
}

/// Collect `.red` files: a single file as-is, or all under a directory
/// (skipping the `build` output directory and hidden folders).
fn collect_red_files(path: &Path) -> Result<Vec<PathBuf>, String> {
    if path.is_file() {
        return Ok(vec![path.to_path_buf()]);
    }
    let mut out = Vec::new();
    walk_red(path, &mut out).map_err(|e| format!("could not scan {}: {e}", path.display()))?;
    out.sort();
    Ok(out)
}

fn walk_red(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let p = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if p.is_dir() {
            if name == "build" || name.starts_with('.') {
                continue;
            }
            walk_red(&p, out)?;
        } else if p.extension().is_some_and(|e| e == "red") {
            out.push(p);
        }
    }
    Ok(())
}

fn cmd_new(name: &str) -> Result<(), String> {
    let root = PathBuf::from(name);
    if root.exists() {
        return Err(format!("`{name}` already exists"));
    }

    let src = root.join("src");
    let abis = src.join("abis");
    std::fs::create_dir_all(&abis).map_err(|e| format!("could not create {}: {e}", abis.display()))?;

    write_file(
        &root.join("redstart.toml"),
        &format!(
            "[project]\nname = \"{name}\"\nentry = \"src/main.red\"\nout_dir = \"build\"\n"
        ),
    )?;
    write_file(&root.join(".gitignore"), "/build\n")?;
    write_file(&src.join("main.red"), STARTER_MAIN)?;
    write_file(&src.join("accounts.red"), STARTER_ACCOUNTS)?;
    write_file(&abis.join("ERC20.json"), STARTER_ABI)?;

    println!("✓ created `{name}`");
    println!("  cd {name} && redstart build");
    Ok(())
}

fn write_file(path: &Path, contents: &str) -> Result<(), String> {
    std::fs::write(path, contents).map_err(|e| format!("could not write {}: {e}", path.display()))
}

const STARTER_MAIN: &str = r#"// Welcome to Redstart. One source of truth, split across modules:
// this generates schema.graphql, subgraph.yaml, and the mappings together.
//
// Entities live in `accounts.red`, pulled in here with `mod`.

mod accounts;

abi ERC20 from "./abis/ERC20.json"

source Token {
  abi: ERC20
  network: mainnet
  address: 0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48
  startBlock: 6082465
}

handler on Token.Transfer(event) {
  let to = accounts::Account.loadOrCreate(event.params.to, { balance: BigInt.zero })
  to.balance = to.balance + event.params.value
  // auto-saved at handler end (dirty-tracked)
}
"#;

const STARTER_ACCOUNTS: &str = r#"// Entities for the starter subgraph, loaded via `mod accounts;` in main.red.

entity Account {
  id: Id<Bytes>
  balance: BigInt
  label: Option<String>   // Option<T> is how nullability is expressed — no `null`
}
"#;

const STARTER_ABI: &str = r#"[
  {
    "type": "event",
    "name": "Transfer",
    "inputs": [
      { "name": "from", "type": "address", "indexed": true },
      { "name": "to", "type": "address", "indexed": true },
      { "name": "value", "type": "uint256", "indexed": false }
    ]
  }
]
"#;
