//! Model Context Protocol server for Redstart (`redstart mcp`).
//!
//! The author-side keystone for AI agents: it exposes the toolchain the compiler
//! already owns — `check`, `explain`, `build`, `test` — as MCP tools an agent can
//! drive in a write → check → fix loop. `check` is the star: it returns the same
//! structured diagnostics as `redstart check --json` (errors *and* lint warnings),
//! so an agent gets machine-readable, precisely-located feedback on every edit.
//!
//! The transport is MCP-over-stdio: newline-delimited JSON-RPC 2.0 (one message
//! per line), hand-rolled — no async runtime, because the toolchain is synchronous
//! and subgraph projects are small. Start it with `redstart mcp`.

#![forbid(unsafe_code)]

use redstart_checker::explain;
use redstart_loader::{LoadError, ModuleTree};
use redstart_test::Outcome;
use serde_json::{json, Value};
use std::io::{BufRead, Write};
use std::path::Path;

/// The MCP protocol revision we default to when a client doesn't request one.
const DEFAULT_PROTOCOL: &str = "2024-11-05";

/// Serve the MCP protocol on stdio, blocking until stdin closes.
pub fn run() {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let msg: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => {
                write_msg(&mut out, &error(Value::Null, -32700, "parse error"));
                continue;
            }
        };
        let method = msg.get("method").and_then(Value::as_str).unwrap_or("");
        // A request carries an `id` and expects a response; a notification doesn't.
        if let Some(id) = msg.get("id").cloned() {
            let resp = handle_request(method, msg.get("params"), id);
            write_msg(&mut out, &resp);
        }
        // Notifications (`notifications/initialized`, `notifications/cancelled`, …)
        // need no reply and have no side effects here.
    }
}

/// Dispatch a JSON-RPC request to its handler, always producing a response.
fn handle_request(method: &str, params: Option<&Value>, id: Value) -> Value {
    match method {
        "initialize" => success(id, initialize_result(params)),
        "tools/list" => success(id, json!({ "tools": tool_defs() })),
        "tools/call" => success(id, tools_call(params)),
        "ping" => success(id, json!({})),
        "shutdown" => success(id, Value::Null),
        _ => error(id, -32601, &format!("method not found: {method}")),
    }
}

fn initialize_result(params: Option<&Value>) -> Value {
    // Echo the client's requested protocol version for maximum compatibility.
    let protocol = params
        .and_then(|p| p.get("protocolVersion"))
        .and_then(Value::as_str)
        .unwrap_or(DEFAULT_PROTOCOL);
    json!({
        "protocolVersion": protocol,
        "capabilities": { "tools": { "listChanged": false } },
        "serverInfo": { "name": "redstart", "version": env!("CARGO_PKG_VERSION") },
        "instructions": "Redstart authoring tools. Call `check` after every edit for \
            structured diagnostics, and `explain` to understand any code it reports.",
    })
}

/// The advertised tools and their input schemas.
fn tool_defs() -> Value {
    json!([
        {
            "name": "check",
            "description": "Type-check a Redstart project (or inline source) and return \
                structured diagnostics — errors and lint warnings, each with a code, \
                message, and file/line location. The primary write→check→fix loop tool; \
                `ok` is false only when there are errors (warnings never block a build).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Path to a project directory, redstart.toml, or a .red file (default: current directory)." },
                    "source": { "type": "string", "description": "Inline single-file .red source to check instead of a path (no on-disk ABIs)." }
                }
            }
        },
        {
            "name": "explain",
            "description": "Explain a Redstart diagnostic code — what triggers it, the \
                subgraph footgun it prevents, and the canonical fix. Omit `code` to list \
                every code.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "code": { "type": "string", "description": "A diagnostic code, e.g. E062 or W040." }
                }
            }
        },
        {
            "name": "build",
            "description": "Build a Redstart project: emit schema.graphql, subgraph.yaml, \
                and src/mappings.ts. Returns the generated artifacts and any optimisation \
                notes; set `write` to also write them to the project's out directory.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Path to a project (default: current directory)." },
                    "write": { "type": "boolean", "description": "Write artifacts to disk as well as returning them (default false)." }
                }
            }
        },
        {
            "name": "test",
            "description": "Run a Redstart project's `test` blocks natively against a mock \
                store (no WASM, no Docker) and return per-test pass/fail results.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Path to a project (default: current directory)." }
                }
            }
        }
    ])
}

/// Route a `tools/call` to the named tool. Tool-level failures come back as a
/// result with `isError: true` (the MCP convention), not a JSON-RPC error.
fn tools_call(params: Option<&Value>) -> Value {
    let name = params
        .and_then(|p| p.get("name"))
        .and_then(Value::as_str)
        .unwrap_or("");
    let args = params
        .and_then(|p| p.get("arguments"))
        .cloned()
        .unwrap_or_else(|| json!({}));
    let outcome = match name {
        "check" => tool_check(&args),
        "explain" => tool_explain(&args),
        "build" => tool_build(&args),
        "test" => tool_test(&args),
        other => Err(format!("unknown tool `{other}`")),
    };
    match outcome {
        Ok(text) => tool_text(&text, false),
        Err(text) => tool_text(&text, true),
    }
}

// ---- tools ----------------------------------------------------------------

/// `check`: always returns a `{ ok, diagnostics }` document — a load/parse failure
/// is itself a check result (`ok: false`), never a tool error, so an agent gets
/// structured feedback on broken input too.
fn tool_check(args: &Value) -> Result<String, String> {
    let doc = match load_tree(args) {
        Ok(tree) => {
            let found = redstart_checker::check_diags(&tree);
            let ok = found.iter().all(|d| !d.is_error());
            let diagnostics: Vec<Value> = found.iter().map(diag_json).collect();
            json!({ "ok": ok, "diagnostics": diagnostics })
        }
        Err(errs) => json!({ "ok": false, "diagnostics": load_errors_json(&errs) }),
    };
    Ok(pretty(&doc))
}

fn tool_explain(args: &Value) -> Result<String, String> {
    match args.get("code").and_then(Value::as_str) {
        Some(code) => match explain::explain(code) {
            Some(e) => Ok(pretty(&json!({
                "code": e.code,
                "title": e.title,
                "summary": e.summary,
                "prevents": e.prevents,
                "fix": e.fix,
            }))),
            None => Err(format!(
                "unknown diagnostic code `{code}` — omit `code` to list every code"
            )),
        },
        None => {
            let codes: Vec<Value> = explain::all()
                .iter()
                .map(|e| json!({ "code": e.code, "title": e.title }))
                .collect();
            Ok(pretty(&json!({ "codes": codes })))
        }
    }
}

fn tool_build(args: &Value) -> Result<String, String> {
    let tree = load_tree(args).map_err(|errs| join_load_errors(&errs))?;
    let mut checked = redstart_checker::check(&tree).map_err(|reports| reports.join("\n"))?;
    let generated = redstart_codegen::generate(&tree, &mut checked);
    let write = args.get("write").and_then(Value::as_bool).unwrap_or(false);
    if write {
        generated.write_to(&tree.out_dir).map_err(|e| {
            format!(
                "failed to write build output to {}: {e}",
                tree.out_dir.display()
            )
        })?;
    }
    Ok(pretty(&json!({
        "ok": true,
        "written": write,
        "out_dir": tree.out_dir.display().to_string(),
        "schema": generated.schema,
        "manifest": generated.manifest,
        "mappings": generated.mappings,
        "notes": generated.notes,
        "warnings": generated.warnings,
    })))
}

fn tool_test(args: &Value) -> Result<String, String> {
    let tree = load_tree(args).map_err(|errs| join_load_errors(&errs))?;
    let checked = redstart_checker::check(&tree).map_err(|reports| reports.join("\n"))?;
    let report = redstart_test::run_tests(&tree, &checked);
    let results: Vec<Value> = report
        .results
        .iter()
        .map(|r| {
            let (outcome, message, location) = match &r.outcome {
                Outcome::Pass => ("pass", None, None),
                Outcome::Fail { message, location } => {
                    ("fail", Some(message.clone()), location.clone())
                }
            };
            json!({ "name": r.name, "outcome": outcome, "message": message, "location": location })
        })
        .collect();
    Ok(pretty(&json!({
        "ok": report.ok(),
        "passed": report.passed(),
        "total": report.results.len(),
        "results": results,
    })))
}

// ---- shared helpers -------------------------------------------------------

/// Load a project from `path`, or an inline single-file `source` (no on-disk ABIs).
fn load_tree(args: &Value) -> Result<ModuleTree, Vec<LoadError>> {
    if let Some(source) = args.get("source").and_then(Value::as_str) {
        redstart_loader::load_str("main", source)
    } else {
        let path = args.get("path").and_then(Value::as_str).unwrap_or(".");
        redstart_loader::load(Path::new(path))
    }
}

/// The `check --json` diagnostic shape, so MCP output matches the CLI exactly.
fn diag_json(d: &redstart_checker::Diag) -> Value {
    json!({
        "severity": d.severity_str(),
        "code": d.code_short(),
        "message": d.message,
        "label": d.label_str(),
        "help": d.help_str(),
        "file": d.file,
        "line": d.line,
        "column": d.col,
        "offset": d.offset,
        "length": d.len,
    })
}

fn load_errors_json(errs: &[LoadError]) -> Vec<Value> {
    errs.iter()
        .map(|e| json!({ "severity": "error", "message": strip_ansi(&e.to_string()) }))
        .collect()
}

fn join_load_errors(errs: &[LoadError]) -> String {
    errs.iter()
        .map(|e| strip_ansi(&e.to_string()))
        .collect::<Vec<_>>()
        .join("\n")
}

fn tool_text(text: &str, is_error: bool) -> Value {
    json!({ "content": [ { "type": "text", "text": text } ], "isError": is_error })
}

fn success(id: Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn error(id: Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

fn pretty(v: &Value) -> String {
    serde_json::to_string_pretty(v).unwrap_or_else(|_| "{}".to_string())
}

fn write_msg<W: Write>(out: &mut W, msg: &Value) {
    // One compact JSON message per line (stdio transport framing).
    let _ = writeln!(out, "{}", serde_json::to_string(msg).unwrap_or_default());
    let _ = out.flush();
}

/// Strip ANSI escapes so rendered loader errors are clean inside JSON strings.
fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            for d in chars.by_ref() {
                if d.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initialize_reports_server_info_and_echoes_protocol() {
        let params = json!({ "protocolVersion": "2025-06-18" });
        let resp = handle_request("initialize", Some(&params), json!(1));
        let result = &resp["result"];
        assert_eq!(result["serverInfo"]["name"], "redstart");
        assert_eq!(result["protocolVersion"], "2025-06-18");
        assert!(result["capabilities"]["tools"].is_object());
    }

    #[test]
    fn tools_list_advertises_the_four_tools() {
        let resp = handle_request("tools/list", None, json!(2));
        let tools = resp["result"]["tools"].as_array().unwrap();
        let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
        assert_eq!(names, ["check", "explain", "build", "test"]);
        // Every tool must carry an object input schema.
        assert!(tools.iter().all(|t| t["inputSchema"]["type"] == "object"));
    }

    #[test]
    fn unknown_method_is_a_jsonrpc_error() {
        let resp = handle_request("frobnicate", None, json!(3));
        assert_eq!(resp["error"]["code"], -32601);
    }

    /// Drive a full `tools/call` and parse the text payload back out.
    fn call(name: &str, args: Value) -> (bool, Value) {
        let params = json!({ "name": name, "arguments": args });
        let resp = handle_request("tools/call", Some(&params), json!(9));
        let result = &resp["result"];
        let is_error = result["isError"].as_bool().unwrap();
        let text = result["content"][0]["text"].as_str().unwrap();
        let parsed = serde_json::from_str(text).unwrap_or(Value::Null);
        (is_error, parsed)
    }

    #[test]
    fn explain_known_code_returns_the_explanation() {
        let (is_error, doc) = call("explain", json!({ "code": "W040" }));
        assert!(!is_error);
        assert_eq!(doc["code"], "W040");
        assert!(doc["fix"].as_str().unwrap().contains("Id<Bytes>"));
    }

    #[test]
    fn explain_unknown_code_is_a_tool_error() {
        let (is_error, _) = call("explain", json!({ "code": "Z999" }));
        assert!(is_error);
    }

    #[test]
    fn explain_without_code_lists_every_code() {
        let (is_error, doc) = call("explain", json!({}));
        assert!(!is_error);
        let codes = doc["codes"].as_array().unwrap();
        assert!(codes.iter().any(|c| c["code"] == "W050"));
    }

    #[test]
    fn check_inline_source_reports_a_parse_error_as_not_ok() {
        // Malformed source: a load/parse failure is still a check result.
        let (is_error, doc) = call("check", json!({ "source": "entity {" }));
        assert!(!is_error, "load failure must be a result, not a tool error");
        assert_eq!(doc["ok"], false);
        assert!(!doc["diagnostics"].as_array().unwrap().is_empty());
    }

    #[test]
    fn unknown_tool_is_a_tool_error() {
        let (is_error, _) = call("nope", json!({}));
        assert!(is_error);
    }
}
