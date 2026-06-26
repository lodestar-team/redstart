#!/usr/bin/env bash
# Build the Redstart playground: compile the wasm engine into ./pkg.
#
#   playground/build.sh
#
# Requires wasm-pack (https://rustwasm.github.io/wasm-pack/). The static page
# (index.html, app.js, style.css) loads ./pkg/redstart_wasm.js at runtime.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

wasm-pack build crates/redstart-wasm \
  --target web \
  --out-dir "$ROOT/playground/pkg" \
  --no-typescript \
  --release

echo "✓ playground built — serve the playground/ directory, e.g.:"
echo "    python3 -m http.server -d playground 8080"
