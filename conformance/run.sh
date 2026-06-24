#!/usr/bin/env bash
#
# Redstart conformance harness — the Stage-0 kill/pivot gate.
#
# Tiers (run in order, each builds on the last):
#   build   redstart build -> graph codegen -> graph build
#           Proves the eject path: the canonical toolchain compiles our output
#           UNMODIFIED. Needs only Node + npm.
#   deploy  also deploys our subgraph AND a hand-written reference (same schema,
#           manifest, ABIs and source — only the mapping.ts differs) to a local
#           graph-node. Needs Docker (see docker-compose.yml) + an RPC endpoint.
#   diff    field-level store-diff of the two, pinned to a fixed block.
#           THE GATE: our lowered AssemblyScript must produce a store identical
#           to idiomatic hand-written AssemblyScript.
#   all     deploy + wait-for-sync + diff.
#
# Configure via environment variables (all have defaults except where noted):
#   PROJECT           Redstart project to test         (default: examples/erc20)
#   BLOCK             block to pin the diff at          (REQUIRED for diff/all)
#   RPC_URL           archive RPC for the network       (for deploy/all)
#   NETWORK           network name in the manifest      (default: mainnet)
#   GRAPH_NODE_ADMIN  graph-node admin endpoint         (default: http://localhost:8020)
#   GRAPH_NODE_QUERY  graph-node query endpoint         (default: http://localhost:8000)
#   IPFS              IPFS endpoint                      (default: http://localhost:5001)
#
# Tip: the example targets USDC on mainnet, which is huge. For a fast gate, point
# PROJECT at a small contract with a short [startBlock, BLOCK] window.

set -euo pipefail

PROJECT="${PROJECT:-examples/erc20}"
SUBGRAPH_NAME="${SUBGRAPH_NAME:-redstart/conformance}"
REF_NAME="${REF_NAME:-redstart/reference}"
GRAPH_NODE_ADMIN="${GRAPH_NODE_ADMIN:-http://localhost:8020}"
GRAPH_NODE_QUERY="${GRAPH_NODE_QUERY:-http://localhost:8000}"
IPFS="${IPFS:-http://localhost:5001}"
GRAPH_CLI="${GRAPH_CLI:-@graphprotocol/graph-cli}"
GRAPH_TS="${GRAPH_TS:-@graphprotocol/graph-ts}"
GRAPH_VERSION="${GRAPH_VERSION:-latest}"
BLOCK="${BLOCK:-}"

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
HERE="$ROOT/conformance"
BUILD_DIR="$ROOT/$PROJECT/build"
REF_DIR="$ROOT/$PROJECT/ref-build"

log()  { printf '\033[1;36m▸ %s\033[0m\n' "$*"; }
ok()   { printf '\033[1;32m✓ %s\033[0m\n' "$*"; }
die()  { printf '\033[1;31m✗ %s\033[0m\n' "$*" >&2; exit 1; }

need() { command -v "$1" >/dev/null 2>&1 || die "missing required tool: $1"; }

# ---- steps ----

build_redstart() {
  log "redstart build $PROJECT"
  ( cd "$ROOT" && cargo run --quiet -p redstart-cli -- build "$PROJECT" )
}

# Write a package.json pinning the graph toolchain, then codegen + build.
graph_compile() {
  local dir="$1"
  need node; need npm
  cat > "$dir/package.json" <<JSON
{
  "name": "redstart-conformance-build",
  "private": true,
  "devDependencies": {
    "$GRAPH_CLI": "$GRAPH_VERSION",
    "$GRAPH_TS": "$GRAPH_VERSION"
  }
}
JSON
  log "npm install (graph-cli + graph-ts) in $dir"
  ( cd "$dir" && npm install --silent --no-audit --no-fund )
  log "graph codegen + graph build in $dir"
  ( cd "$dir" && npx --no-install graph codegen subgraph.yaml && npx --no-install graph build subgraph.yaml )
}

# Reference = our generated build with the mapping swapped for a hand-written one.
build_reference() {
  log "assembling hand-written reference from $BUILD_DIR"
  rm -rf "$REF_DIR"
  cp -r "$BUILD_DIR" "$REF_DIR"
  rm -rf "$REF_DIR/node_modules" "$REF_DIR/generated" "$REF_DIR/build"
  cp "$HERE/reference/erc20/mappings.ts" "$REF_DIR/src/mappings.ts"
  graph_compile "$REF_DIR"
}

deploy_one() {
  local name="$1" dir="$2"
  log "deploy $name"
  ( cd "$dir"
    npx --no-install graph create --node "$GRAPH_NODE_ADMIN" "$name" 2>/dev/null || true
    npx --no-install graph deploy --node "$GRAPH_NODE_ADMIN" --ipfs "$IPFS" \
      "$name" subgraph.yaml --version-label "v0" )
}

graph_block() { # endpoint -> indexed block number (or 0)
  curl -s -X POST "$1" -H 'content-type: application/json' \
    -d '{"query":"{ _meta { block { number } } }"}' 2>/dev/null \
  | node -e 'let s="";process.stdin.on("data",d=>s+=d).on("end",()=>{try{console.log(JSON.parse(s).data._meta.block.number)}catch(e){console.log(0)}})'
}

wait_synced() { # endpoint target
  local ep="$1" target="$2" cur
  log "waiting for $ep to reach block $target"
  for _ in $(seq 1 720); do        # up to ~1h at 5s
    cur="$(graph_block "$ep")"
    printf '\r  indexed block %s / %s' "$cur" "$target"
    [ "${cur:-0}" -ge "$target" ] && { echo; return 0; }
    sleep 5
  done
  echo; die "timed out waiting for $ep to reach block $target"
}

run_diff() {
  need node
  [ -n "$BLOCK" ] || die "set BLOCK=<number> so the diff is deterministic"
  log "store-diff at block $BLOCK"
  node "$HERE/store-diff.mjs" \
    --a "$GRAPH_NODE_QUERY/subgraphs/name/$SUBGRAPH_NAME" \
    --b "$GRAPH_NODE_QUERY/subgraphs/name/$REF_NAME" \
    --schema "$BUILD_DIR/schema.graphql" \
    --block "$BLOCK"
}

# ---- orchestration ----

cmd="${1:-build}"
case "$cmd" in
  build)
    build_redstart
    graph_compile "$BUILD_DIR"
    ok "eject path OK — graph codegen + graph build accepted the generated subgraph"
    ;;
  deploy)
    need curl
    build_redstart
    graph_compile "$BUILD_DIR"
    build_reference
    deploy_one "$SUBGRAPH_NAME" "$BUILD_DIR"
    deploy_one "$REF_NAME" "$REF_DIR"
    ok "both subgraphs deployed"
    ;;
  diff)
    run_diff
    ;;
  all)
    need curl
    [ -n "$BLOCK" ] || die "set BLOCK=<number> for the gate"
    build_redstart
    graph_compile "$BUILD_DIR"
    build_reference
    deploy_one "$SUBGRAPH_NAME" "$BUILD_DIR"
    deploy_one "$REF_NAME" "$REF_DIR"
    wait_synced "$GRAPH_NODE_QUERY/subgraphs/name/$SUBGRAPH_NAME" "$BLOCK"
    wait_synced "$GRAPH_NODE_QUERY/subgraphs/name/$REF_NAME" "$BLOCK"
    run_diff
    ;;
  *)
    die "unknown command '$cmd' (use: build | deploy | diff | all)"
    ;;
esac
