// Redstart MCP server — the "power path". Add it as a connector in Claude Code /
// Claude Desktop, and drive The Generator from inside your Claude subscription:
// YOUR Claude does the generation (its inference, your Max plan); this server
// provides the tools it can't do itself — contract research and the real compile
// gate — plus the best-practices guide as a resource. Pete pays $0.
//
// Tools:  fetch_contract, compile_subgraph, list_supported_networks
// Resource: redstart://best-practices

import http from "node:http";
import { randomUUID } from "node:crypto";
import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StreamableHTTPServerTransport } from "@modelcontextprotocol/sdk/server/streamableHttp.js";
import { isInitializeRequest } from "@modelcontextprotocol/sdk/types.js";
import { z } from "zod";

const PORT = Number(process.env.PORT ?? 8788);
const MCP_TOKEN = process.env.MCP_TOKEN ?? ""; // bearer auth (set in the connector config)
const ETHERSCAN_KEY = process.env.ETHERSCAN_API_KEY ?? "";
const VERIFIER_URL = process.env.VERIFIER_URL ?? "http://127.0.0.1:8787";
const VERIFIER_TOKEN = process.env.VERIFIER_TOKEN ?? "";

// ── chains (chainId → graph network) ──
const CHAINS = [
  { id: 1, label: "Ethereum", graphNetwork: "mainnet" },
  { id: 42161, label: "Arbitrum One", graphNetwork: "arbitrum-one" },
  { id: 8453, label: "Base", graphNetwork: "base" },
  { id: 10, label: "Optimism", graphNetwork: "optimism" },
  { id: 137, label: "Polygon", graphNetwork: "matic" },
  { id: 56, label: "BNB Chain", graphNetwork: "bsc" },
  { id: 43114, label: "Avalanche", graphNetwork: "avalanche" },
  { id: 100, label: "Gnosis", graphNetwork: "gnosis" },
  { id: 534352, label: "Scroll", graphNetwork: "scroll" },
  { id: 59144, label: "Linea", graphNetwork: "linea" },
  { id: 11155111, label: "Sepolia", graphNetwork: "sepolia" },
  { id: 84532, label: "Base Sepolia", graphNetwork: "base-sepolia" },
  { id: 421614, label: "Arbitrum Sepolia", graphNetwork: "arbitrum-sepolia" },
];

// ── ABI helpers ──
const ADDRESS_RE = /^0x[a-fA-F0-9]{40}$/;
const ETHERSCAN_V2 = "https://api.etherscan.io/v2/api";

function paramSig(p) {
  if (p.type?.startsWith("tuple")) {
    const inner = (p.components ?? []).map(paramSig).join(",");
    return `(${inner})${p.type.slice("tuple".length)}`;
  }
  return p.type;
}
function eventSignature(item) {
  return `${item.name}(${(item.inputs ?? []).map(paramSig).join(",")})`;
}
function extractEvents(abi) {
  return abi
    .filter((i) => i.type === "event" && i.name)
    .map((i) => ({ name: i.name, signature: eventSignature(i), inputs: i.inputs ?? [] }));
}
function detectKind(abi) {
  const fns = new Set(abi.filter((i) => i.type === "function" && i.name).map((i) => i.name));
  const events = new Set(abi.filter((i) => i.type === "event" && i.name).map((i) => i.name));
  if (events.has("TransferSingle") || events.has("TransferBatch")) return "erc1155";
  if ((fns.has("ownerOf") || fns.has("tokenURI")) && events.has("Transfer")) return "erc721";
  if (fns.has("balanceOf") && fns.has("transfer") && (fns.has("decimals") || fns.has("totalSupply")))
    return "erc20";
  return "unknown";
}

async function etherscan(chainId, params) {
  const url = new URL(ETHERSCAN_V2);
  url.searchParams.set("chainid", String(chainId));
  url.searchParams.set("apikey", ETHERSCAN_KEY);
  for (const [k, v] of Object.entries(params)) url.searchParams.set(k, v);
  const res = await fetch(url, { headers: { accept: "application/json" } });
  if (!res.ok) throw new Error(`Etherscan HTTP ${res.status}`);
  return res.json();
}
function parseAbi(s) {
  if (!s || s.trim().startsWith("Contract source code not verified")) return null;
  try {
    const p = JSON.parse(s);
    return Array.isArray(p) ? p : null;
  } catch {
    return null;
  }
}
async function fetchSource(chainId, address) {
  const r = await etherscan(chainId, { module: "contract", action: "getsourcecode", address });
  if (r.status !== "1" || !Array.isArray(r.result) || !r.result.length) return null;
  return r.result[0];
}
async function fetchStartBlock(chainId, address) {
  try {
    const r = await etherscan(chainId, {
      module: "contract",
      action: "getcontractcreation",
      contractaddresses: address,
    });
    const n = r.status === "1" && r.result?.[0]?.blockNumber ? Number(r.result[0].blockNumber) : NaN;
    return Number.isFinite(n) ? n : undefined;
  } catch {
    return undefined;
  }
}

async function researchContract(address, chainId) {
  if (!ETHERSCAN_KEY) throw new Error("Server missing ETHERSCAN_API_KEY.");
  if (!ADDRESS_RE.test(address)) throw new Error("Invalid address.");
  const chain = CHAINS.find((c) => c.id === chainId);
  if (!chain) throw new Error(`Unsupported chain ${chainId}.`);

  const source = await fetchSource(chainId, address);
  if (!source) throw new Error("No contract found at that address on this chain.");

  const isProxy = source.Proxy === "1" && ADDRESS_RE.test(source.Implementation);
  const implementation = isProxy ? source.Implementation : undefined;
  let abi = parseAbi(source.ABI);
  if (isProxy && implementation) {
    const impl = await fetchSource(chainId, implementation);
    const implAbi = parseAbi(impl?.ABI);
    if (implAbi) abi = implAbi;
  }
  if (!abi)
    throw new Error("This contract isn't verified on Etherscan (bytecode ABI recovery not supported here yet).");

  const startBlock = await fetchStartBlock(chainId, address);
  return {
    address,
    chainId,
    network: chain.graphNetwork,
    name: source.ContractName || "Contract",
    verified: true,
    isProxy,
    implementation,
    startBlock,
    kind: detectKind(abi),
    events: extractEvents(abi),
    abi,
  };
}

// Compile is the expensive tool (a full graph build). Cap concurrency so a public
// endpoint can't be turned into a compute DoS.
let compiling = 0;
const MAX_COMPILE = 3;
async function compile(files) {
  if (compiling >= MAX_COMPILE) {
    return { ok: false, stage: "busy", error: "Too many concurrent compiles right now — try again in a moment." };
  }
  compiling++;
  try {
    const headers = { "content-type": "application/json" };
    if (VERIFIER_TOKEN) headers.authorization = `Bearer ${VERIFIER_TOKEN}`;
    const res = await fetch(`${VERIFIER_URL.replace(/\/$/, "")}/verify`, {
      method: "POST",
      headers,
      body: JSON.stringify({ files }),
    });
    return res.json();
  } finally {
    compiling--;
  }
}

// Per-IP rate limit (sliding window) — protects the Etherscan key + compile box.
const RL = new Map();
const RL_MAX = 40;
const RL_WINDOW = 60_000;
function rateLimited(ip) {
  const now = Date.now();
  const arr = (RL.get(ip) ?? []).filter((t) => now - t < RL_WINDOW);
  if (arr.length >= RL_MAX) {
    RL.set(ip, arr);
    return true;
  }
  arr.push(now);
  RL.set(ip, arr);
  if (RL.size > 20_000) RL.clear();
  return false;
}

const BEST_PRACTICES = `# Redstart subgraph best-practices (follow these when generating)

A subgraph = schema.graphql + subgraph.yaml + AssemblyScript mappings (+ Matchstick tests).
The mappings are AssemblyScript, NOT TypeScript — a strict subset compiled to WASM by \`asc\`.

## Schema
- \`Bytes!\` for ids from addresses/hashes; never string-concat a single-value id.
- Mark event-log / append-only entities \`@entity(immutable: true)\`; mutable only for real state (balances, owners).
- \`@derivedFrom(field: "...")\` for one-to-many; never store growing arrays on the parent.
- Initialise every non-null, non-derived field before \`.save()\`.

## Manifest
- specVersion 1.2.0, mapping apiVersion 0.0.9, language wasm/assemblyscript.
- \`indexerHints:\\n  prune: auto\` at top level. Exact network, address, startBlock.
- One eventHandler per event with the EXACT signature; prefer event handlers only.

## Mappings (AssemblyScript)
- \`==\` not \`===\` (no strict equality in AS). No \`console.log\` (use graph-ts \`log\`).
- Never force-unwrap \`Entity.load(id)!\` — load, null-check, construct (loadOrCreate).
- \`try_\` variants for contract calls; handle \`.reverted\`; never eth_call in a loop.
- \`.save()\` after mutation. Bytes id from \`event.transaction.hash.concatI32(event.logIndex.toI32())\`.
- Import event classes from \`../generated/<DataSource>/<Contract>\`, entities from \`../generated/schema\`.

## Tests (Matchstick)
- \`tests/utils.ts\`: a factory per event using \`newMockEvent()\` + \`ethereum.Value.fromX\` matching ABI types.
- \`tests/<name>.test.ts\`: describe/test with clearStore, assert.entityCount, assert.fieldEquals.

## Workflow with these tools
1. \`fetch_contract\` → ABI, events, proxy, startBlock, kind.
2. Write the files, then \`compile_subgraph({files})\` to run the REAL graph codegen+build+test.
3. If it fails, read the log, fix the AS, and compile again until green.
4. Write the files to the user's repo and \`git push\` (you can do this yourself in Claude Code).`;

// ── MCP server factory ──
function buildServer() {
  const server = new McpServer(
    { name: "redstart", version: "0.1.0" },
    { capabilities: { tools: {}, resources: {} } },
  );

  server.registerTool(
    "fetch_contract",
    {
      title: "Fetch contract",
      description:
        "Research a verified EVM contract for subgraph generation: resolves proxies to the implementation ABI, detects the token standard, finds the deployment (start) block, and lists indexable events. Returns the full ABI too. Use this first.",
      inputSchema: {
        address: z.string().describe("0x contract address"),
        chainId: z.number().int().describe("Chain id, e.g. 1 for Ethereum mainnet"),
      },
    },
    async ({ address, chainId }) => {
      try {
        const info = await researchContract(address.trim(), chainId);
        return { content: [{ type: "text", text: JSON.stringify(info, null, 2) }] };
      } catch (e) {
        return { content: [{ type: "text", text: `Error: ${e.message}` }], isError: true };
      }
    },
  );

  server.registerTool(
    "compile_subgraph",
    {
      title: "Compile subgraph",
      description:
        "Run the REAL Graph toolchain (graph codegen && graph build && graph test) on a set of generated subgraph files, and report whether it compiles. This is the compile gate — use it to verify your generated AssemblyScript before delivering it, and iterate on any errors. Accepts a map of relative path -> file contents (schema.graphql, subgraph.yaml, src/mapping.ts, tests/*.ts, abis/*.json).",
      inputSchema: {
        files: z.record(z.string()).describe("Map of path -> file contents"),
      },
    },
    async ({ files }) => {
      try {
        const r = await compile(files);
        const head = r.ok
          ? "✓ Compiles — graph codegen, build and test passed."
          : `✕ Failed at ${r.stage}${r.timedOut ? " (timed out)" : ""}.`;
        return { content: [{ type: "text", text: `${head}\n\n${r.log || r.error || ""}` }], isError: !r.ok };
      } catch (e) {
        return { content: [{ type: "text", text: `Verifier error: ${e.message}` }], isError: true };
      }
    },
  );

  server.registerTool(
    "list_supported_networks",
    {
      title: "List supported networks",
      description: "List the chains The Generator supports (chain id → Graph network name).",
      inputSchema: {},
    },
    async () => ({
      content: [{ type: "text", text: JSON.stringify(CHAINS, null, 2) }],
    }),
  );

  server.registerResource(
    "best-practices",
    "redstart://best-practices",
    {
      title: "Redstart subgraph best-practices",
      description: "The generation guide — read this before writing a subgraph.",
      mimeType: "text/markdown",
    },
    async (uri) => ({
      contents: [{ uri: uri.href, mimeType: "text/markdown", text: BEST_PRACTICES }],
    }),
  );

  return server;
}

// ── HTTP (stateless Streamable HTTP) ──
function readBody(req) {
  return new Promise((resolve, reject) => {
    let b = "";
    req.on("data", (c) => {
      b += c;
      if (b.length > 5_000_000) reject(new Error("body too large"));
    });
    req.on("end", () => resolve(b));
    req.on("error", reject);
  });
}

// Active sessions: sessionId -> transport.
const SESSIONS = new Map();

const httpServer = http.createServer(async (req, res) => {
  const cors = {
    "access-control-allow-origin": "*",
    "access-control-allow-headers": "content-type, authorization, mcp-session-id, mcp-protocol-version",
    "access-control-allow-methods": "POST, GET, DELETE, OPTIONS",
  };
  if (req.method === "OPTIONS") {
    res.writeHead(204, cors);
    return res.end();
  }
  if (req.url === "/health") {
    res.writeHead(200, { "content-type": "application/json", ...cors });
    return res.end(JSON.stringify({ ok: true }));
  }
  if (!req.url?.startsWith("/mcp")) {
    res.writeHead(404, cors);
    return res.end("not found");
  }
  if (MCP_TOKEN && req.headers.authorization !== `Bearer ${MCP_TOKEN}`) {
    res.writeHead(401, cors);
    return res.end("unauthorized");
  }
  // Public endpoint (no token): rate-limit per client IP.
  const ip = (req.headers["x-forwarded-for"] ?? "").split(",")[0].trim() || req.socket.remoteAddress || "?";
  if (rateLimited(ip)) {
    res.writeHead(429, { "retry-after": "30", ...cors });
    return res.end("rate limited — slow down");
  }
  for (const [k, v] of Object.entries(cors)) res.setHeader(k, v);
  const sessionId = req.headers["mcp-session-id"];

  try {
    // GET (server-initiated SSE) / DELETE (terminate) — need an existing session.
    if (req.method === "GET" || req.method === "DELETE") {
      const t = sessionId && SESSIONS.get(sessionId);
      if (!t) {
        res.writeHead(400, { "content-type": "application/json" });
        return res.end(JSON.stringify({ error: "Unknown or missing session id" }));
      }
      return void (await t.handleRequest(req, res));
    }
    if (req.method !== "POST") {
      res.writeHead(405);
      return res.end("method not allowed");
    }

    const raw = await readBody(req);
    const body = raw ? JSON.parse(raw) : undefined;

    let transport;
    if (sessionId && SESSIONS.has(sessionId)) {
      transport = SESSIONS.get(sessionId);
    } else if (!sessionId && isInitializeRequest(body)) {
      transport = new StreamableHTTPServerTransport({
        sessionIdGenerator: () => randomUUID(),
        onsessioninitialized: (sid) => SESSIONS.set(sid, transport),
      });
      transport.onclose = () => {
        if (transport.sessionId) SESSIONS.delete(transport.sessionId);
      };
      await buildServer().connect(transport);
    } else {
      res.writeHead(400, { "content-type": "application/json" });
      return res.end(
        JSON.stringify({ jsonrpc: "2.0", error: { code: -32000, message: "No valid session; send initialize first." }, id: null }),
      );
    }
    await transport.handleRequest(req, res, body);
  } catch (e) {
    if (!res.headersSent) {
      res.writeHead(500, { "content-type": "application/json" });
      res.end(JSON.stringify({ jsonrpc: "2.0", error: { code: -32603, message: String(e?.message ?? e) }, id: null }));
    }
  }
});

const HOST = process.env.HOST ?? "0.0.0.0";
httpServer.listen(PORT, HOST, () => {
  console.log(`redstart-mcp on ${HOST}:${PORT} (verifier=${VERIFIER_URL})`);
});
