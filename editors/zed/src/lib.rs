//! Zed extension for Redstart: registers the language server (`redstart lsp`).
//! Syntax highlighting is provided by the bundled tree-sitter grammar and the
//! `languages/redstart/` queries — no Rust needed for that part.

use zed_extension_api::{self as zed, Command, LanguageServerId, Result, Worktree};

struct RedstartExtension;

impl zed::Extension for RedstartExtension {
    fn new() -> Self {
        Self
    }

    fn language_server_command(
        &mut self,
        _language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<Command> {
        // Use the `redstart` binary from the user's PATH and run its LSP.
        let path = worktree.which("redstart").ok_or_else(|| {
            "`redstart` not found in PATH — install it from https://redstart-lang.com".to_string()
        })?;
        Ok(Command {
            command: path,
            args: vec!["lsp".to_string()],
            env: worktree.shell_env(),
        })
    }
}

zed::register_extension!(RedstartExtension);
