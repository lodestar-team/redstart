//! Multi-file module tree construction.
//!
//! Resolution mirrors the proven model from `sage`: a `mod name;` declaration
//! resolves to either a sibling `name.red` or a nested `name/mod.red`, with
//! cycle detection along the way. The result is a flat map of every parsed
//! module keyed by its module path.

use crate::error::LoadError;
use crate::manifest::ProjectManifest;
use redstart_parser::ast::Program;
use redstart_parser::{lex_named, parse, ParseErrors};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// A module path such as `["tokens", "erc20"]`.
pub type ModulePath = Vec<String>;

/// A fully loaded module tree for a Redstart project.
#[derive(Debug)]
pub struct ModuleTree {
    /// Every parsed module, keyed by module path. The root module has key `[]`.
    pub modules: HashMap<ModulePath, ParsedModule>,
    /// The project root directory.
    pub project_root: PathBuf,
    /// The project name (from the manifest, or the file stem for single files).
    pub name: String,
    /// Optional project description (from the manifest).
    pub description: Option<String>,
    /// The output directory for generated artifacts.
    pub out_dir: PathBuf,
}

impl ModuleTree {
    /// The root module (`[]`), which is always present.
    #[must_use]
    pub fn root(&self) -> &ParsedModule {
        self.modules
            .get(&Vec::new())
            .expect("module tree always has a root")
    }

    /// Modules in a deterministic order (root first, then sorted by path).
    #[must_use]
    pub fn ordered(&self) -> Vec<&ParsedModule> {
        let mut keys: Vec<&ModulePath> = self.modules.keys().collect();
        keys.sort();
        keys.into_iter().map(|k| &self.modules[k]).collect()
    }
}

/// A single parsed module.
#[derive(Debug)]
pub struct ParsedModule {
    /// The module's path (`[]` for the root).
    pub path: ModulePath,
    /// The file this module was loaded from.
    pub file_path: PathBuf,
    /// The source text, shared for diagnostics.
    pub source: Arc<str>,
    /// The parsed AST.
    pub program: Program,
}

/// Load a project rooted at a directory (containing `redstart.toml`) or a single
/// `.red` file.
///
/// # Errors
/// Returns every error encountered (manifest, IO, lex, parse, resolution).
pub fn load(path: &Path) -> Result<ModuleTree, Vec<LoadError>> {
    load_with_overlay(path, HashMap::new())
}

/// Like [`load`], but reads in-memory contents from `overlay` (keyed by
/// canonicalized path) in preference to disk. Used by the language server to
/// analyze unsaved edits.
///
/// # Errors
/// Returns every error encountered (manifest, IO, lex, parse, resolution).
pub fn load_with_overlay(
    path: &Path,
    overlay: HashMap<PathBuf, String>,
) -> Result<ModuleTree, Vec<LoadError>> {
    if path.is_file() {
        if path.extension().is_some_and(|e| e == "red") {
            return load_single_file(path, overlay);
        }
        if path.file_name().is_some_and(|n| n == "redstart.toml") {
            return load_project(path.parent().unwrap_or(Path::new(".")), overlay);
        }
    }
    load_project(path, overlay)
}

/// Load a single `.red` file with no surrounding project.
fn load_single_file(path: &Path, overlay: HashMap<PathBuf, String>) -> Result<ModuleTree, Vec<LoadError>> {
    let mut loader = ModuleLoader::new(overlay);
    loader.load_module(&[], path)?;
    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("subgraph")
        .to_string();
    let project_root = path
        .parent()
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf);
    let out_dir = project_root.join("build");
    Ok(ModuleTree {
        modules: loader.modules,
        project_root,
        name,
        description: None,
        out_dir,
    })
}

/// Load a project from a directory containing `redstart.toml`.
fn load_project(dir: &Path, overlay: HashMap<PathBuf, String>) -> Result<ModuleTree, Vec<LoadError>> {
    let manifest_path = dir.join("redstart.toml");
    if !manifest_path.exists() {
        return Err(vec![LoadError::NoManifest {
            dir: dir.to_path_buf(),
        }]);
    }

    let manifest = ProjectManifest::load(&manifest_path).map_err(|e| vec![e.into()])?;
    let project_root = dir.to_path_buf();
    let entry = project_root.join(&manifest.project.entry);
    if !entry.exists() {
        return Err(vec![LoadError::MissingEntry { path: entry }]);
    }

    let mut loader = ModuleLoader::new(overlay);
    loader.load_module(&[], &entry)?;

    Ok(ModuleTree {
        modules: loader.modules,
        out_dir: project_root.join(&manifest.project.out_dir),
        project_root,
        name: manifest.project.name,
        description: manifest.project.description,
    })
}

struct ModuleLoader {
    modules: HashMap<ModulePath, ParsedModule>,
    loading: HashSet<PathBuf>,
    overlay: HashMap<PathBuf, String>,
}

impl ModuleLoader {
    fn new(overlay: HashMap<PathBuf, String>) -> Self {
        Self {
            modules: HashMap::new(),
            loading: HashSet::new(),
            overlay,
        }
    }

    fn load_module(&mut self, path: &[String], file: &Path) -> Result<(), Vec<LoadError>> {
        let canonical = file.canonicalize().unwrap_or_else(|_| file.to_path_buf());

        if self.loading.contains(&canonical) {
            return Err(vec![LoadError::CircularDependency {
                cycle: self.loading.iter().map(|p| p.display().to_string()).collect(),
            }]);
        }
        if self.modules.contains_key(path) {
            return Ok(());
        }
        self.loading.insert(canonical.clone());

        // Prefer in-memory overlay contents (unsaved editor buffers) over disk.
        let source = match self.overlay.get(&canonical).or_else(|| self.overlay.get(file)) {
            Some(s) => s.clone(),
            None => std::fs::read_to_string(file).map_err(|source| {
                vec![LoadError::Io {
                    path: file.to_path_buf(),
                    source,
                }]
            })?,
        };
        let source_arc: Arc<str> = Arc::from(source.as_str());
        let filename = file.display().to_string();

        let lexed = lex_named(&source, &filename).map_err(|e| {
            vec![LoadError::Lex {
                file: file.to_path_buf(),
                report: render(&e),
            }]
        })?;

        let (program, parse_errors) = parse(lexed.tokens(), Arc::clone(&source_arc));
        if !parse_errors.is_empty() {
            let bundle = ParseErrors::new(&filename, source.clone(), parse_errors);
            return Err(vec![LoadError::Parse {
                file: file.to_path_buf(),
                report: render(&bundle),
            }]);
        }

        // Resolve child modules from `mod` declarations.
        let parent_dir = file.parent().unwrap_or(Path::new("."));
        let child_specs: Vec<(Vec<String>, PathBuf)> = program
            .mods
            .iter()
            .map(|m| {
                let mut child_path = path.to_vec();
                child_path.push(m.name.name.clone());
                self.find_module_file(parent_dir, &m.name.name)
                    .map(|file| (child_path, file))
            })
            .collect::<Result<_, _>>()?;

        self.loading.remove(&canonical);

        self.modules.insert(
            path.to_vec(),
            ParsedModule {
                path: path.to_vec(),
                file_path: file.to_path_buf(),
                source: source_arc,
                program,
            },
        );

        for (child_path, child_file) in child_specs {
            self.load_module(&child_path, &child_file)?;
        }

        Ok(())
    }

    fn find_module_file(&self, parent_dir: &Path, name: &str) -> Result<PathBuf, Vec<LoadError>> {
        let sibling = parent_dir.join(format!("{name}.red"));
        let nested = parent_dir.join(name).join("mod.red");
        match (sibling.exists(), nested.exists()) {
            (true, true) => Err(vec![LoadError::AmbiguousModule {
                name: name.to_string(),
                candidates: vec![sibling, nested],
            }]),
            (true, false) => Ok(sibling),
            (false, true) => Ok(nested),
            (false, false) => Err(vec![LoadError::ModuleNotFound {
                name: name.to_string(),
                searched: vec![sibling, nested],
            }]),
        }
    }
}

/// Render a `miette` diagnostic to a string using the graphical handler.
fn render(diag: &dyn miette::Diagnostic) -> String {
    let mut out = String::new();
    let handler = miette::GraphicalReportHandler::new();
    let _ = handler.render_report(&mut out, diag);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn loads_single_file() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("main.red");
        fs::write(&file, "entity Token { id: Id<Bytes> }").unwrap();

        let tree = load(&file).unwrap();
        assert_eq!(tree.modules.len(), 1);
        assert_eq!(tree.root().program.entities.len(), 1);
    }

    #[test]
    fn loads_project_with_submodule() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("redstart.toml"),
            "[project]\nname = \"demo\"\nentry = \"src/main.red\"",
        )
        .unwrap();
        fs::create_dir_all(dir.path().join("src")).unwrap();
        fs::write(dir.path().join("src/main.red"), "mod tokens;").unwrap();
        fs::write(
            dir.path().join("src/tokens.red"),
            "entity Token { id: Id<Bytes> }",
        )
        .unwrap();

        let tree = load(dir.path()).unwrap();
        assert_eq!(tree.modules.len(), 2);
        assert!(tree.modules.contains_key(&vec!["tokens".to_string()]));
        assert_eq!(tree.name, "demo");
    }

    #[test]
    fn reports_missing_module() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("main.red");
        fs::write(&file, "mod nope;").unwrap();
        let err = load(&file).unwrap_err();
        assert!(matches!(err[0], LoadError::ModuleNotFound { .. }));
    }
}
