# Redstart MCP server

The **power path** for [The Generator](https://redstart-lang.com/generator): add it as
a connector in Claude Code / Claude Desktop and generate subgraphs from inside your
Claude subscription. **Your Claude does the generation** (its inference, your Max/Pro
plan); this server provides the two things it can't do itself:

- **`fetch_contract`** — research a verified contract (proxy → implementation ABI,
  token-standard detection, deployment start block, indexable events, full ABI).
- **`compile_subgraph`** — run the REAL `graph codegen && graph build && graph test`
  on the generated files and report pass/fail. The compile gate, as a tool your
  Claude can call and iterate against.
- **`list_supported_networks`** — chain id → Graph network.
- Resource **`redstart://best-practices`** — the generation guide.

No per-token cost to anyone — inference is your subscription.

## Run (Docker, on the VPS next to the verifier)

```bash
cd services/redstart-mcp
docker build -t redstart-mcp .

# --network host so it can reach the verifier on 127.0.0.1:8787;
# HOST=127.0.0.1 keeps :8788 private (only Caddy reaches it).
docker run -d --name redstart-mcp --restart unless-stopped \
  --network host \
  -e HOST=127.0.0.1 -e PORT=8788 \
  -e MCP_TOKEN="$(openssl rand -hex 24)" \
  -e ETHERSCAN_API_KEY=... \
  -e VERIFIER_URL=http://127.0.0.1:8787 \
  -e VERIFIER_TOKEN=... \
  --memory 512m --cpus 1 --pids-limit 128 \
  redstart-mcp
```

Front it with Caddy (auto-TLS): `mcp.<host>.sslip.io { reverse_proxy 127.0.0.1:8788 }`.
Health: `curl localhost:8788/health`.

## Connect (public — anyone, on their own Claude plan)

The hosted endpoint is **public** (no token). Anyone adds it to their Claude and
generates subgraphs on their own subscription — inference is their plan, this server
just provides the tools. It's rate-limited per IP (40 req/min) with a compile
concurrency cap, so it can't be turned into a compute DoS.

**Claude Code:**

```bash
claude mcp add --transport http redstart https://mcp.89.167.109.4.sslip.io/mcp
```

**Claude Desktop / claude.ai:** Settings → Connectors → Add custom connector → the
same URL, no auth. (claude.ai web has historically been flaky at surfacing
custom-connector tools — Claude Code / Desktop are the reliable hosts today.)

Then: *"Use the redstart tools to build a subgraph for WETH
(0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2) on Ethereum — read the best-practices
resource, fetch the contract, write the files, and compile_subgraph until green."*

### Private / self-hosted

Set `MCP_TOKEN` on the container to require `Authorization: Bearer <token>` instead
of running open.

## Smoke test

```bash
URL=https://mcp.<host>.sslip.io/mcp MCP_TOKEN=<token> node test-client.mjs
```
