# horizon-indexer (Redstart rewrite)

A [Redstart](../../) rewrite of **[PaulieB14/horizon-indexer-subgraph](https://github.com/PaulieB14/horizon-indexer-subgraph)** —
a subgraph that indexes **The Graph Horizon** staking, allocations, delegations,
and rewards across three contracts on **Arbitrum One**.

> ### Credit
> The original subgraph — its schema design, contract selection, event handling
> logic, and the timeseries/aggregation model — is the work of
> **[PaulieB14](https://github.com/PaulieB14)**
> ([horizon-indexer-subgraph](https://github.com/PaulieB14/horizon-indexer-subgraph)).
> This is a faithful, behaviour-for-behaviour port to Redstart, kept here as a
> real-world example. All credit for the indexing design belongs to the original
> author; any bugs in the translation are ours.

## What it indexes

| Contract | Source | Handlers |
|---|---|---|
| `HorizonStaking` `0x00669A…eF03` | operator authorizations | `OperatorSet` |
| `SubgraphService` `0xb2Bb92…1105` | allocations, indexing rewards, query fees | `AllocationCreated` / `Closed` / `Resized`, `IndexingRewardsCollected`, `QueryFeesCollected` |
| `StakingExtension` `0x3bE385…3571` | delegation lifecycle | `StakeDelegated` / `Locked` / `Withdrawn` |

It maintains `Indexer`, `Operator`, `Allocation`, `Delegation`, immutable
`RewardEvent`/`QueryFeeEvent` logs, a `GlobalStats` singleton, and three
**timeseries + aggregation** pairs (`RewardData`→`RewardDailyAgg`, etc.).

## Why it's a good Redstart stress test

The port exercises nearly the whole language in one project:

- **Helper `fn`s** — `getOrCreateIndexer`, `getOrCreateGlobalStats`,
  `getOrCreateDelegation` (lowered to AssemblyScript, shared across modules).
- **Nullable `load`** — `Allocation.load(id)` returns `Option<Allocation>` and
  *must* be `match`ed; a null-deref is a compile error.
- **String composite ids** (`Operator`, `Delegation`, `GlobalStats`), **`Int`
  counters**, **`BigInt` arithmetic**, **multiple data sources**, **`@derivedFrom`**,
  **immutable entities**, and **timeseries/aggregations**.

What you *don't* see here are the AssemblyScript footguns: no manual `.save()`
bookkeeping (entities auto-save, even across a helper's early `return`), no
`null` handling boilerplate, no `==`/`===` confusion, no manifest/schema/handler
drift — the three artefacts are projections of these `.red` modules.

## Build, test, deploy

```sh
# from the repo root
cargo run -p redstart-cli -- check  examples/horizon-indexer
cargo run -p redstart-cli -- build  examples/horizon-indexer
cargo run -p redstart-cli -- test   examples/horizon-indexer   # 7 native handler tests, no Docker

# eject proof: the canonical toolchain compiles the output to WASM unmodified
cargo run -p redstart-cli -- deploy horizon-indexer examples/horizon-indexer --dry-run

# real deploy (needs `graph auth <DEPLOY_KEY>` for Subgraph Studio first):
cargo run -p redstart-cli -- deploy <your-studio-slug> examples/horizon-indexer --version-label v0.0.1
```

`redstart build` → `graph codegen` → `graph build` produces
`build/HorizonStaking/HorizonStaking.wasm` with **zero manual edits** — the eject
path holds for the full feature surface.
