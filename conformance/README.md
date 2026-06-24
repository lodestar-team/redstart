# Conformance harness — the Stage-0 kill/pivot gate

Redstart's entire bet is that its generated AssemblyScript is **faithful**: the
canonical `graph build` toolchain compiles it unmodified, and it produces the
*same store* a careful human would. This harness proves both — and the design
report is explicit that if this gate can't be met, the project stops.

It runs in three escalating tiers.

## Tier 1 — `build` (eject path) · needs only Node 18+

```sh
./conformance/run.sh build
```

Runs `redstart build`, then `graph codegen` + `graph build` on the output. If it
passes, the canonical Graph toolchain accepts our generated subgraph **without a
single manual edit** — the eject-path claim, demonstrated. This is the fast inner
loop; run it constantly.

## Tier 2 — `deploy` · needs Docker + an archive RPC

Bring up a local graph-node, IPFS and Postgres:

```sh
RPC_URL=https://your-archive-rpc NETWORK=mainnet \
  docker compose -f conformance/docker-compose.yml up -d
```

Then deploy **both** our subgraph and the hand-written reference:

```sh
RPC_URL=https://your-archive-rpc ./conformance/run.sh deploy
```

The reference is our generated `build/` with only `src/mappings.ts` swapped for
[`reference/erc20/mappings.ts`](reference/erc20/mappings.ts) — idiomatic,
independently hand-written AssemblyScript. Same schema, same manifest, same ABIs,
same indexed source. The *only* variable is generated-AS vs hand-AS.

## Tier 3 — `diff` / `all` · THE GATE

```sh
# build + deploy both + wait for sync + diff, pinned to a fixed block:
BLOCK=12500000 RPC_URL=https://your-archive-rpc ./conformance/run.sh all
```

`store-diff.mjs` reads the entity types from `schema.graphql`, queries every
entity from both endpoints **pinned to `BLOCK`** (so the comparison is
deterministic even while indexing continues), and reports any field that differs.
Exit code is non-zero on any divergence — wire it into CI.

```
✓ Account: 1423 (A) / 1423 (B), 0 diff(s)
✓ Transfer: 5102 (A) / 5102 (B), 0 diff(s)
✓ stores are identical at block 12500000
```

A green run is the gate met: **Redstart's lowering is byte-for-behavior identical
to hand-written code.** Any red line is a codegen bug with the exact entity, id,
and field that diverged.

## Choosing a fixture

The bundled `examples/erc20` targets USDC on mainnet — far too busy to index for
a quick gate. For a real run, point `PROJECT` at a small contract with a short
`[startBlock, BLOCK]` window:

```sh
PROJECT=path/to/small-token BLOCK=<startBlock+a few thousand> \
  RPC_URL=… ./conformance/run.sh all
```

## Configuration

All via environment variables — see the header of [`run.sh`](run.sh). Key ones:
`PROJECT`, `BLOCK` (required for diff/all), `RPC_URL`, `NETWORK`,
`GRAPH_NODE_ADMIN`, `GRAPH_NODE_QUERY`, `IPFS`.

## What this does and doesn't prove

- ✅ The eject path: canonical toolchain compiles our output unmodified.
- ✅ Behavioral equivalence to hand-written mappings, field by field, at a block.
- ⚠️ Only over the chosen fixture and block window — broaden fixtures (ERC-721, a
  DEX pair) to widen coverage. The report's plan is to re-implement Uniswap v3 /
  ENS / Aave and diff against their canonical deployments next.
