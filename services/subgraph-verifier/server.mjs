// Subgraph verifier — a tiny, isolated sandbox that runs the REAL Graph toolchain
// (`graph codegen && graph build`) on a generated subgraph and reports whether it
// compiles. This is the "verified before you see it" gate for The Generator.
//
// It receives a small file map (schema, manifest, mappings, ABI) over HTTP, drops
// it into a copy of a pre-installed base project (graph-cli + graph-ts already in
// node_modules — no per-request npm install), compiles, and returns the result.
// No handler code is ever executed: `graph build` only runs the AssemblyScript
// compiler. Runs behind a token; meant to sit on a VPS behind the Next.js proxy.

import { createServer } from "node:http";
import { execFile } from "node:child_process";
import { mkdtemp, mkdir, writeFile, rm, cp } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

const PORT = Number(process.env.PORT ?? 8787);
const BASE_PROJECT = process.env.BASE_PROJECT ?? "/opt/base";
const AUTH_TOKEN = process.env.VERIFIER_TOKEN ?? ""; // shared secret with the proxy
const TIMEOUT_MS = 90_000;
const MAX_BODY = 2_000_000; // 2 MB is plenty for a subgraph

// Only these paths may be written — everything else is rejected. Prevents the
// untrusted file map from escaping the job directory.
const ALLOWED = [
  /^schema\.graphql$/,
  /^subgraph\.yaml$/,
  /^src\/[A-Za-z0-9_-]+\.ts$/,
  /^abis\/[A-Za-z0-9_-]+\.json$/,
];

function allowedPath(p) {
  return typeof p === "string" && ALLOWED.some((re) => re.test(p));
}

function run(cmd, args, cwd) {
  return new Promise((resolve) => {
    execFile(
      cmd,
      args,
      { cwd, timeout: TIMEOUT_MS, maxBuffer: 10_000_000, env: { ...process.env, PATH: process.env.PATH } },
      (err, stdout, stderr) => {
        resolve({
          ok: !err,
          code: err?.code ?? 0,
          timedOut: err?.killed === true && err?.signal === "SIGTERM",
          stdout: String(stdout ?? ""),
          stderr: String(stderr ?? ""),
        });
      },
    );
  });
}

async function verify(files) {
  const dir = await mkdtemp(join(tmpdir(), "sg-verify-"));
  try {
    // Copy the pre-installed base project (node_modules with graph-ts).
    await cp(BASE_PROJECT, dir, { recursive: true });

    for (const [path, content] of Object.entries(files)) {
      if (!allowedPath(path)) return { ok: false, stage: "input", error: `Disallowed file path: ${path}` };
      if (typeof content !== "string") return { ok: false, stage: "input", error: `File ${path} must be a string` };
      const full = join(dir, path);
      await mkdir(join(full, ".."), { recursive: true });
      await writeFile(full, content);
    }

    // graph codegen: synthesise the generated entity/event classes from schema+ABI.
    const codegen = await run("npx", ["--no-install", "graph", "codegen"], dir);
    if (!codegen.ok) {
      return {
        ok: false,
        stage: "codegen",
        timedOut: codegen.timedOut,
        log: tail(codegen.stderr || codegen.stdout),
      };
    }

    // graph build: run asc over the mappings against the generated types.
    const build = await run("npx", ["--no-install", "graph", "build"], dir);
    return {
      ok: build.ok,
      stage: build.ok ? "done" : "build",
      timedOut: build.timedOut,
      log: build.ok ? tail(build.stdout) : tail(build.stderr || build.stdout),
    };
  } catch (e) {
    return { ok: false, stage: "error", error: String(e?.message ?? e) };
  } finally {
    await rm(dir, { recursive: true, force: true });
  }
}

function tail(s, lines = 60) {
  const arr = String(s).split("\n");
  return arr.slice(-lines).join("\n").trim();
}

const server = createServer((req, res) => {
  const cors = {
    "access-control-allow-origin": "*",
    "access-control-allow-headers": "content-type, authorization",
    "access-control-allow-methods": "POST, OPTIONS",
  };
  if (req.method === "OPTIONS") {
    res.writeHead(204, cors);
    return res.end();
  }
  if (req.method === "GET" && req.url === "/health") {
    res.writeHead(200, { "content-type": "application/json", ...cors });
    return res.end(JSON.stringify({ ok: true }));
  }
  if (req.method !== "POST" || req.url !== "/verify") {
    res.writeHead(404, cors);
    return res.end("not found");
  }
  if (AUTH_TOKEN) {
    const auth = req.headers["authorization"] ?? "";
    if (auth !== `Bearer ${AUTH_TOKEN}`) {
      res.writeHead(401, cors);
      return res.end("unauthorized");
    }
  }

  let body = "";
  let tooBig = false;
  req.on("data", (chunk) => {
    body += chunk;
    if (body.length > MAX_BODY) {
      tooBig = true;
      req.destroy();
    }
  });
  req.on("end", async () => {
    if (tooBig) {
      res.writeHead(413, cors);
      return res.end("too large");
    }
    let files;
    try {
      files = JSON.parse(body).files;
    } catch {
      res.writeHead(400, { "content-type": "application/json", ...cors });
      return res.end(JSON.stringify({ ok: false, stage: "input", error: "Invalid JSON" }));
    }
    if (!files || typeof files !== "object") {
      res.writeHead(400, { "content-type": "application/json", ...cors });
      return res.end(JSON.stringify({ ok: false, stage: "input", error: "Missing files" }));
    }
    const result = await verify(files);
    res.writeHead(200, { "content-type": "application/json", ...cors });
    res.end(JSON.stringify(result));
  });
});

server.listen(PORT, () => {
  console.log(`subgraph-verifier listening on :${PORT} (base=${BASE_PROJECT})`);
});
