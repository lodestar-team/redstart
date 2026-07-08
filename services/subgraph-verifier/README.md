# Subgraph verifier

The "verified before you see it" gate for [The Generator](https://redstart-lang.com/generator).
A tiny, isolated sandbox that runs the **real** Graph toolchain
(`graph codegen && graph build`) on a generated subgraph and reports whether it
compiles — the genuine AssemblyScript compiler, against the genuine generated
types. No fidelity risk, and no handler code is ever executed (`graph build` only
runs `asc`).

Sits on a VPS behind the site's `/api/verify` proxy; the browser never talks to it
directly.

## Run it (Docker)

```bash
cd services/subgraph-verifier
docker build -t subgraph-verifier .

# VERIFIER_TOKEN is a shared secret with the Next.js proxy (set the same value as
# COMPILER_TOKEN in the web app's env).
docker run -d --name subgraph-verifier \
  --restart unless-stopped \
  -p 127.0.0.1:8787:8787 \
  -e VERIFIER_TOKEN="$(openssl rand -hex 24)" \
  --memory 1g --cpus 1 --pids-limit 256 \
  subgraph-verifier
```

Put it behind TLS (Caddy/nginx) so the proxy can reach `https://verify.your-vps.tld`.
Binding to `127.0.0.1` above assumes the reverse proxy is on the same host.

Health check: `curl localhost:8787/health` → `{"ok":true}`.

## Wire it to the site

In the web app's environment (Vercel):

- `COMPILER_URL` = `https://verify.your-vps.tld`
- `COMPILER_TOKEN` = the same value passed as `VERIFIER_TOKEN`

`web/app/api/verify/route.ts` proxies the browser's request here, adding the token.

## API

`POST /verify` with a JSON body:

```json
{ "files": { "schema.graphql": "...", "subgraph.yaml": "...", "src/mapping.ts": "...", "abis/Token.json": "..." } }
```

Response:

```json
{ "ok": true, "stage": "done", "log": "..." }
{ "ok": false, "stage": "codegen" | "build", "timedOut": false, "log": "<compiler errors>" }
```

Only `schema.graphql`, `subgraph.yaml`, `src/*.ts`, and `abis/*.json` are accepted;
anything else is rejected. Body capped at 2 MB, job capped at 90 s.

## Hardening notes

- Runs unprivileged inside the container; jobs live in ephemeral `/tmp` dirs and are
  removed after each run.
- Apply `--memory`, `--cpus`, `--pids-limit` (shown above). Add `--network none`
  **only after** the base image is built — the base install needs the network, the
  request path does not.
- Put a rate limit on the reverse proxy; the shared token blocks unauthenticated use.
