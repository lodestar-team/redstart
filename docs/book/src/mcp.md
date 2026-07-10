# MCP server (for AI agents)

`redstart mcp` starts a [Model Context Protocol](https://modelcontextprotocol.io)
server over stdio, exposing the toolchain as tools an AI agent can call directly.
The point is the **write → check → fix loop**: an agent authoring a subgraph edits
`.red`, calls `check`, reads precisely-located diagnostics, and fixes them —
without a human relaying compiler output.

## Wiring it up

Register `redstart mcp` as an MCP server with your agent/host. For Claude Code:

```console
$ claude mcp add redstart -- redstart mcp
```

Any MCP-capable client works — the transport is standard newline-delimited
JSON-RPC 2.0 over stdio.

## Tools

| Tool | Arguments | Returns |
|------|-----------|---------|
| `check` | `path` *or* `source` | `{ ok, diagnostics }` — errors **and** lint warnings, each with a code, message, and file/line. `ok` is false only when there are errors. |
| `explain` | `code` (optional) | The code's meaning, the footgun it prevents, and the fix. Omit `code` to list every code. |
| `build` | `path`, `write` (optional) | The generated `schema.graphql`, `subgraph.yaml`, and `src/mappings.ts` (plus optimisation notes). `write: true` also writes them to disk. |
| `test` | `path` | Per-test pass/fail for the project's `test` blocks (native, no Docker). |

`check` is the keystone. It returns the same structured diagnostics as
`redstart check --json`, so the agent gets machine-readable feedback on every edit
— and a parse or load failure comes back as an ordinary `{ ok: false, diagnostics }`
result rather than an error, so the loop never stalls on broken input. `check` also
accepts inline `source` (a single `.red` file, no on-disk ABIs) for quick snippets.

## Why this matters

Redstart's guarantees — no nullable-arithmetic miscompiles, no non-deterministic
host calls, `Bytes` ids, `@derivedFrom` relations — are only useful if they reach
the author at the moment of writing. For a human that's `redstart check` and the
LSP; for an agent it's this MCP server. The compiler already owns `check`,
`explain`, `build`, and `test`; the MCP server just hands them to the agent.
