# Redstart 2026 Roadmap

**From "a nicer AssemblyScript" to *the optimising, footgun-proof compiler for subgraphs*.**

This document is the strategy and backlog for making Redstart not 10% better than
hand-writing AssemblyScript subgraphs, but *categorically* better — safer, faster,
and the obvious default for both humans and AI agents. It is grounded in a cited
catalogue of real, documented AssemblyScript / graph-node footguns (see
[§7 Appendix](#7-appendix--the-cited-footgun-catalogue)), not vibes.

---

## 0. The thesis (read this first)

> **AssemblyScript is a *language*. Redstart is a *compiler that knows it is
> compiling a subgraph*.**

That single distinction is the entire unfair advantage, and almost nothing in the
roadmap below is reachable by "just write AssemblyScript":

1. **AS cannot prevent a subgraph bug it doesn't understand.** It has no concept of
   an entity, a `.save()`, a derived field, a non-deterministic handler, or a
   reverted `eth_call`. Redstart does — so it can make whole bug classes
   *unrepresentable*, not merely *lintable*.
2. **AS cannot optimise your indexing.** It doesn't know an `eth_call` is a 100 ms
   blocking RPC, that a stored array is O(n²) disk, or that your `startBlock` is
   scanning ten million dead blocks. Redstart owns the schema, the manifest, *and*
   the mappings — so it can apply every documented performance best-practice
   automatically.

The strongest external evidence for lever #1: The Graph already ships a
[**Subgraph Linter**](https://thegraph.com/docs/en/subgraphs/guides/subgraph-linter/)
with eight named checks — `entity-overwrite`, `unexpected-null`, `unchecked-load`,
`unchecked-nonnull`, `division-guard`, `derived-field-guard`,
`helper-return-contract`, `undeclared-eth-call`. **That linter is a list of things
The Graph admits are footguns but can only *warn* about.** Redstart's mission is to
turn every one of them into a *compile error or an absent grammar rule.*

**The one-line pitch we are building toward:**
*"Redstart makes the Subgraph Linter unrepresentable and applies every performance
best-practice automatically — which a language never can."*

---

## 1. Where we are — Stage 0, done (✅)

Shipped and eject-verified (the canonical `graph build` compiles our output to WASM
unmodified), proven end-to-end by porting a real subgraph
([`examples/horizon-indexer`](../examples/horizon-indexer), a port of
[PaulieB14/horizon-indexer-subgraph](https://github.com/PaulieB14/horizon-indexer-subgraph))
and **deploying it live to Subgraph Studio**.

- Unified language: schema + manifest + mappings from one `.red` source, multi-file
- Control flow, helper `fn`s (cross-module, return-typed, `return`-flushed auto-save)
- All handler kinds: event / call / block / file (IPFS)
- Dynamic data sources (templates + context), enums, interfaces, timeseries +
  aggregations, `Int8`/`Timestamp`
- **Footguns already killed by construction:** forgotten `.save()` (auto-save +
  return-flush), nullable host returns (`load`/`loadInBlock`/`ipfs.cat` →
  `Option<T>`, deref is a compile error E062), reverted calls (`Result` + forced
  `match`), arithmetic-on-`Option`, assign-to-derived, manifest/schema/handler drift
- Tooling: native test interpreter (no Docker), LSP, formatter, `dev` watch,
  `deploy`, tree-sitter

**The one unproven thing (still our #1 risk):** the field-level **store-diff** — that
our generated WASM indexes *byte-identically* to hand-written AssemblyScript against
a live graph-node. We have proven *compilation* and *native-interpreter behaviour*;
we have **not** proven *indexing fidelity*. Until that gate is green, "better" is a
hypothesis with strong compile-time evidence. See [§6](#6-the-kill-gate).

---

## 2. The three levers

| Lever | What it is | Why AS can't | Status |
|---|---|---|---|
| **1. Unrepresentable bugs** | Turn the 8 Subgraph-Linter checks + determinism hazards into compile errors / absent grammar | AS has no model of entities, saves, determinism, reverts | ~half done |
| **2. Optimising compiler** | Apply every documented perf best-practice automatically (startBlock, declared eth_calls, Bytes ids, immutable inference, derived-array rewrite, pruning) | AS doesn't know it's a subgraph | not started |
| **3. Agent-native** | Structured diagnostics, in-loop native tests, MCP server, scaffold-from-address | n/a — this is product surface | partially done |

The rest of this doc is the backlog for each, pillar by pillar, with citations.

---

## 3. Pillar: Correctness & Security — *make the linter unrepresentable*

Each item maps to a Subgraph-Linter check or a documented incident. Status: ✅ done ·
🔜 next · 📋 planned.

### 3.1 ✅ Forgotten `.save()` / stale-overwrite (`entity-overwrite`)
Auto-save with dirty-tracking + return-flush + matched-binding auto-save. The
classic "helper reloads+saves, handler then saves its stale copy, last write wins"
(linter `entity-overwrite`) is structurally impossible because there is one tracked
instance per (entity, id) per invocation.
*Refs:* [linter](https://thegraph.com/docs/en/subgraphs/guides/subgraph-linter/) ·
[AS mappings](https://thegraph.com/docs/en/subgraphs/developing/creating/assemblyscript-mappings/)

### 3.2 ✅ Nullable derefs (`unexpected-null`, `unchecked-load`, `unchecked-nonnull`)
`load`/`loadInBlock`/`ipfs.cat` return `Option<T>`; touching a field without `match`
is compile error **E062**. There is no `!` non-null override in the grammar to
abuse. **Extend (🔜):** `TypedMap.get` / `json` getters → `Option`; model
`loadInBlock`'s "null even if it exists in store" semantics so it can't be confused
with `load`.
*Refs:* [graph-ts API](https://thegraph.com/docs/en/subgraphs/developing/creating/graph-ts/api/) ·
[derivedFrom null incident](https://github.com/graphprotocol/graph-ts/issues/219)

### 3.3 ✅ Derived fields (`derived-field-guard`)
Assigning to a derived field is rejected; reading one in a handler forces `.load()`.

### 3.4 🔜 Division-by-zero (`division-guard`) — **HIGH**
Divide-by-zero is a **fatal, deterministic sync halt** (`attempted to divide
BigDecimal '86400' by zero`). Make `/` on `BigInt`/`BigDecimal` require a
provably-non-zero denominator (a literal, or a value guarded by a preceding
`if denom != 0` / `match`), else a compile error with a fix-it.
*Refs:* [graph-node #5281](https://github.com/graphprotocol/graph-node/issues/5281) ·
[linter](https://thegraph.com/docs/en/subgraphs/guides/subgraph-linter/)

### 3.5 🔜 BigInt-division precision trap — **HIGH**
`BigInt / BigInt` truncates fractions to **zero** (the canonical Uniswap bug:
ETH/MKR price returns `0` instead of `~0.27`). When a `BigInt` division result flows
into a `BigDecimal` field, *require* `.divDecimal()` — or warn and offer the fix.
*Refs:* [graph-node #720](https://github.com/graphprotocol/graph-node/issues/720)

### 3.6 🔜 BigDecimal 34-digit precision ceiling — **MEDIUM**
`BigDecimal` is decimal128 (34 significant digits); wider fixed-point (`ufixed256x18`)
loses precision silently. Warn statically when a value could exceed the ceiling
(e.g. raw `uint256` token amounts before scaling).
*Refs:* [graph-ts API](https://thegraph.com/docs/en/subgraphs/developing/creating/graph-ts/api/)

### 3.7 🔜 Determinism — forbid non-determinism in the grammar — **HIGH**
Non-determinism breaks Proof-of-Indexing across indexers and gets them slashed.
graph-node already *blocks* `Math.random` (`unknown import: env::seed`). Redstart
should make the whole class absent: **no `Date.now`, no float type, no RNG** in the
surface language; force `block.timestamp`, `BigInt`/`BigDecimal`. Also: unguarded
`assert`/array-length assumptions are a real divergence vector (the 28-ids-34-values
`TransferBatch` incident that halted one indexer while another reported 100% synced).
*Refs:* [determinism contract](https://thegraph.com/docs/en/subgraphs/developing/creating/advanced/) ·
[randomness blocked](https://forum.thegraph.com/t/indexing-error-subgraph-studio/5354) ·
[divergence incident](https://forum.thegraph.com/t/the-integrity-of-the-data-may-be-open-to-question/4470)

### 3.8 🔜 Entity-ID collisions — **HIGH**
Naive string-concat IDs lose field boundaries (`"ab"+"c"` == `"a"+"bc"`) and
tx-hash-only IDs collide when one tx emits N events → silent under-counting. Provide
typed, collision-resistant ID composition (`bytes.concatI32(logIndex)`), and
**forbid raw string `+` for IDs** — point users at the typed builder.
*Refs:* [bytes-as-ids](https://thegraph.com/docs/en/subgraphs/best-practices/immutable-entities-bytes-as-ids/)

### 3.9 🔜 Event-signature / topic0 drift — **HIGH**
A handler **silently never fires** if its event signature doesn't keccak-match the
on-chain one (and graph-node downgraded the "Skipping handler" log to `trace` — it's
invisible). codegen also mis-emits `tuple[]` for struct-array events so topic0 never
matches. We already derive signatures from the ABI by reference — **extend to expand
tuple/struct components** and verify indexed/non-indexed placement, so drift is a
compile error, not a silent no-op.
*Refs:* [manifest](https://thegraph.com/docs/en/subgraphs/developing/creating/subgraph-manifest/) ·
[tuple codegen #520](https://github.com/graphprotocol/graph-tooling/issues/520)

### 3.10 🔜 `eth_call` revert safety (`undeclared-eth-call` + reverts)
Already: contract calls are `Result`, must `match`. **Add:** revert detection is
*client-dependent* (Geth/Infura may miss reverts a Parity client catches) — a genuine
PoI-divergence vector; surface this as a deploy-time warning. The call-handler
"phantom data" incident (a reverted call inside a Safe `multiSend` was indexed as
real) argues for steering hard toward event handlers.
*Refs:* [client-dependent reverts](https://thegraph.com/docs/en/subgraphs/developing/creating/graph-ts/api/) ·
[phantom call #3701](https://github.com/graphprotocol/graph-node/issues/3701)

### 3.11 📋 Immutable / timeseries misuse
`@entity(timeseries:true)` auto-manages `id`/`timestamp` (silently overrides if you
set them) and is *always* immutable. Marking a mutable entity immutable traps you
forever. Make auto fields unassignable and reject updates to immutable/timeseries
entities at compile time.
*Refs:* [aggregations](https://github.com/graphprotocol/graph-node/blob/master/docs/aggregations.md)

---

## 4. Pillar: Performance — *be an optimising compiler*

This is the lever AS physically cannot pull. Legend: **[AUTO]** compiler rewrites it ·
**[WARN]** compiler flags it. Priority order (highest leverage first).

### 4.1 🔜 [AUTO] Auto-fill `startBlock` from the deployment block — **HIGHEST single win**
Omitting / lowballing `startBlock` scans millions of dead blocks; too high silently
skips real events. The deploy block is deterministically discoverable from the
contract's creation tx. Fetch it and default `startBlock`; **error loudly on `0`**.
*Refs:* [manifest](https://thegraph.com/docs/en/subgraphs/developing/creating/subgraph-manifest/) ·
[#2007](https://github.com/graphprotocol/graph-node/issues/2007)

### 4.2 🔜 [AUTO] Declare `eth_calls` for parallel pre-fetch
`eth_calls` are the **#1 sync killer** (100 ms–several seconds *each*, run serially
while the handler is paused). When a call's receiver+args are pure functions of
`event.address`/`event.params`, emit a manifest `calls:` block (specVersion ≥ 1.2.0)
so graph-node runs them **in parallel before handlers** and caches them. Vendor
(Goldsky) figure: 5–10×; official docs say "greatly speed up". Also **[AUTO]** rewrite
a call whose return is already in the triggering event → read the event param; and
**[WARN]** on metadata calls (`name`/`symbol`/`decimals`) not behind a `load()==null`
cache, and any `bind`/`try_*` inside a loop ("stuck at 3%").
*Refs:* [avoid eth_calls](https://thegraph.com/docs/en/subgraphs/best-practices/avoid-eth-calls/) ·
[reduce eth_calls](https://thegraph.com/blog/improve-subgraph-performance-reduce-eth-calls/) ·
[declared calls (Goldsky)](https://docs.goldsky.com/subgraphs/guides/declared-eth-calls)

### 4.3 🔜 [AUTO] Default `Bytes` ids + infer `immutable: true`
Measured (Edge & Node benchmark): immutable + Bytes ids → **28% faster indexing,
48% less disk** vs mutable + String ids. **[AUTO]**: when an id comes from a Bytes
source only `.toHexString()`'d to fit a `String!`, rewrite the field to `id: Bytes!`
and drop the conversion. When a type is only ever `new`-constructed + `.save()`d
(never load-then-mutate anywhere), emit `@entity(immutable: true)`. (Redstart already
does this for timeseries — generalise it.) Caveat: figures are "up to" and
workload-dependent ([#3534](https://github.com/graphprotocol/graph-node/issues/3534)
shows a regression case) — so gate behind a confidence check / opt-out.
*Refs:* [bytes-as-ids](https://thegraph.com/docs/en/subgraphs/best-practices/immutable-entities-bytes-as-ids/) ·
[benchmark](https://medium.com/edge-node-engineering/two-simple-subgraph-performance-improvements-a76c6b3e7eac)

### 4.4 🔜 [AUTO/WARN] Growing stored array → `@derivedFrom`
A stored `[Child!]!` mutated via `.push()`+`.save()` is O(n²) disk (every append
copies the whole array into a new versioned row; harmful beyond ~thousands, tolerable
< ~50). Detect it and synthesise the migration: add a back-ref field on the child,
annotate the parent `@derivedFrom`, rewrite `push` → `child.save()`.
*Refs:* [derivedFrom](https://thegraph.com/docs/en/subgraphs/best-practices/derivedfrom/) ·
[avoid large arrays](https://thegraph.com/blog/improve-subgraph-performance-avoiding-large-arrays/)

### 4.5 🔜 [AUTO] Default `indexerHints: prune: auto`
Absent `indexerHints`, the default is `prune: never` (largest DB, slowest queries).
Default to `prune: auto`; **[WARN]** and downgrade to `prune: <N>`/`never` when
grafting or time-travel queries are detected (pruning breaks both).
*Refs:* [pruning](https://thegraph.com/docs/en/subgraphs/best-practices/pruning/)

### 4.6 🔜 [AUTO] Coalesce redundant loads; hoist loop-invariant loads
Each `load(id)` is a SELECT + (on save) a versioned write. Coalesce repeated
load/save of the same id, hoist loop-invariant loads, and substitute `loadInBlock`
**only** when intra-block dataflow proves same-block creation (its null-semantics
differ, so it's unsafe otherwise).
*Refs:* [graph-ts API](https://thegraph.com/docs/en/subgraphs/developing/creating/graph-ts/api/)

### 4.7 🔜 [WARN] Handler-shape lints
Unfiltered `blockHandlers` (runs every block, whole chain — "very, very slow");
`callHandlers` / `kind: call` (need Parity tracing — *unsupported* on
BNB/Arbitrum/Polygon/Optimism, so the handler silently never fires there); mismatched
`startBlock`s across call-handler sources (graph-node bug → unrestricted
`trace_filter`, 4 h build → 24 h index). Flag all three; **error** on call handlers
targeting a non-tracing network.
*Refs:* [manifest](https://github.com/graphprotocol/graph-node/blob/master/docs/subgraph-manifest.md) ·
[#2007](https://github.com/graphprotocol/graph-node/issues/2007)

### 4.8 📋 [AUTO] Suggest timeseries/aggregations for load-mutate-save totals
A hand-rolled rolling sum (`load` totals entity → mutate → `save` per event) is a
write-contention hotspot. When that pattern appears, suggest a DB-computed
`@aggregation` (zero handler code). We already support the feature — add the
detection.
*Refs:* [timeseries](https://thegraph.com/docs/en/subgraphs/best-practices/timeseries/)

---

## 5. Pillar: Agent-native DX & developer lovability

Redstart should be the language an AI agent *prefers* to write subgraphs in — and
the one a human falls in love with. These overlap.

### 5.1 ✅ Structured, machine-readable diagnostics (`--json`) — **HIGHEST agent leverage**
`redstart check --json` emits `{ "ok": bool, "diagnostics": [ {code, severity,
message, label, help, file, line, column, offset, length}, … ] }` to stdout
(exit non-zero when not `ok`), so an agent loop *reads the error and applies the
fix* without parsing prose. Lex/parse/resolution failures are emitted too (ANSI
stripped). Elm-grade errors aren't just nice for humans — they're the agent's
feedback signal. **Next:** a `fix` field carrying a machine-applicable edit (span
+ replacement) once the auto-fixes in §3–4 land.

### 5.2 ✅→🔜 Native test interpreter as an in-loop tool
Already a moat over Matchstick (no Docker/WASM, sub-second). Keep investing: richer
assertions, fixture helpers, coverage. An agent can write-and-run a test in the same
turn — Matchstick can't touch that loop time.

### 5.3 🔜 MCP server
Expose `check` / `build` / `test` / `explain` / `new-from-address` as MCP tools so any
agent (Claude Code, Cursor, …) drives Redstart natively. This is the distribution
play for the agent era.

### 5.4 🔜 `redstart new --from 0x<address> [--network …]` — the no-brainer on-ramp
Fetch the ABI + deployment block (Etherscan/Sourcify), scaffold a working starter
subgraph (entities inferred from events, one handler per event, sensible
`startBlock`). Turns "start a subgraph" from an afternoon into ten seconds — for
humans *and* agents. This is probably the single biggest adoption lever in the doc.

### 5.5 ✅ `redstart explain E062` / inline fix-its
`redstart explain <CODE>` explains any diagnostic code — title, what triggered
it, **the bug it prevents**, and the canonical fix (`--json` too; bare `redstart
explain` lists all 24 codes). Done in v0.3.0. **Next:** inline fix-its as LSP
code actions once the auto-fixes in §3–4 land.

### 5.6 📋 LSP completeness + editor reach
Code actions (apply the auto-fixes from §3–4 as quick-fixes), inlay hints for
inferred types, rename, find-references; ship the VS Code extension to the
marketplace; tree-sitter to nvim/Helix/Zed/GitHub.

### 5.7 📋 One-binary install + great `--help`
`curl | sh`, Homebrew, prebuilt binaries (already drafted in the README). Frictionless
install is table stakes for lovability.

---

## 6. The kill-gate

**The store-diff is still the #1 risk and the highest-value single task in this
document.** `./conformance/run.sh all` deploys our generated subgraph *and* an
idiomatic hand-written reference to a real graph-node and field-level-diffs their
stores at a fixed block. Everything else proves *it compiles* and *the interpreter
agrees*; only this proves *it indexes identically*. It needs Docker + an archive RPC.
**Until it's green, every performance and correctness claim above carries an asterisk.**
Make it a one-command, CI-runnable gate and run it against `examples/horizon-indexer`
and `examples/factory`.

---

## 7. Prioritised backlog (the order to actually build)

**P0 — proves the bet / unblocks everything**
1. Store-diff conformance gate, green, in CI (§6) — *workflow wired
   (`conformance-storediff.yml`), awaiting an `RPC_URL` secret + Docker runner*
2. ✅ `--json` diagnostics (§5.1, v0.2.0) + ✅ `redstart explain` error-code docs
   (§5.5, v0.3.0) — the agent loop is unblocked.
3. `redstart new --from 0x<address>` (§5.4) — the adoption on-ramp

**P1 — the headline differentiators (lever 1 + 2)**
4. Division-guard + BigInt→Decimal precision (§3.4, §3.5)
5. Determinism: forbid `Date.now`/float/RNG (§3.7)
6. Auto `startBlock` from deployment block (§4.1)
7. Auto-declare `eth_calls` (§4.2)
8. Collision-resistant typed IDs; forbid string-concat IDs (§3.8)
9. Tuple/struct event-signature expansion (§3.9)

**P2 — the optimising-compiler depth + polish**
10. Bytes-id + immutable inference (§4.3)
11. Stored-array → `@derivedFrom` rewrite (§4.4)
12. `prune: auto` default + handler-shape lints (§4.5, §4.7)
13. MCP server (§5.3); LSP code actions (§5.6)
14. Load coalescing / loop-invariant hoist (§4.6)

---

## 8. Appendix — the cited footgun catalogue

The research underpinning §3–4, condensed. Every claim was verified against a primary
source; lower-confidence items are flagged. Full reports were produced by web
research against The Graph docs, forum, graph-node/graph-tooling GitHub, Messari
subgraph standards, and the Edge & Node engineering benchmark.

### 8.1 Non-determinism (PoI divergence → slashing)
- Unhandled `eth_call` revert aborts the handler — *"accessed value of a reverted
  call"* · [#5534](https://github.com/graphprotocol/graph-node/issues/5534)
- **Revert detection is client-dependent** (Geth/Infura miss reverts Parity catches) —
  two honest indexers diverge on identical code ·
  [graph-ts API](https://thegraph.com/docs/en/subgraphs/developing/creating/graph-ts/api/)
- `Math.random` blocked at instantiation (`env::seed`) ·
  [forum](https://forum.thegraph.com/t/indexing-error-subgraph-studio/5354)
- `Date.now`/float — *inferred* from the determinism contract (medium confidence)
- Fulltext search "not yet deterministic", panics on rewind ·
  [#5607](https://github.com/graphprotocol/graph-node/issues/5607)
- Unguarded `assert`/array-length divergence incident ·
  [forum](https://forum.thegraph.com/t/the-integrity-of-the-data-may-be-open-to-question/4470)

### 8.2 Entity IDs
- String ids ~2× storage + locale-aware comparison; Bytes+immutable = **28% / 48%** ·
  [bytes-as-ids](https://thegraph.com/docs/en/subgraphs/best-practices/immutable-entities-bytes-as-ids/)
- Concatenation collisions / tx-hash-only collisions → silent under-count ·
  [protofire](https://medium.com/protofire-blog/subgraph-development-part-2-handling-arrays-and-identifying-entities-30d63d4b1dc6)

### 8.3 `@derivedFrom`
- Read-only virtual; reading in a handler → runtime `unexpected null` ·
  [graph-ts #219](https://github.com/graphprotocol/graph-ts/issues/219)
- Unbounded stored arrays = O(n²) disk (time-travel copies the whole array per save) ·
  [avoid large arrays](https://thegraph.com/blog/improve-subgraph-performance-avoiding-large-arrays/)
- `loadInBlock` returns null for cross-block entities ·
  [graph-ts API](https://thegraph.com/docs/en/subgraphs/developing/creating/graph-ts/api/)

### 8.4 Save / partial updates
- Forgotten `save()` (silent loss); stale-overwrite (`entity-overwrite`); array
  getters return *copies* (`.push` doesn't persist); null-merge on unset required
  fields · [linter](https://thegraph.com/docs/en/subgraphs/guides/subgraph-linter/) ·
  [Messari ERRORS.md](https://github.com/messari/subgraphs/blob/master/docs/ERRORS.md)

### 8.5 eth_call + handlers
- ~100 ms–seconds each, serial; declare in manifest (specVersion 1.2.0) for parallel
  pre-fetch + caching · [reduce eth_calls](https://thegraph.com/blog/improve-subgraph-performance-reduce-eth-calls/)
- Call/block handlers "significantly slower", need tracing, unsupported on several
  chains; phantom-data incident ([#3701](https://github.com/graphprotocol/graph-node/issues/3701));
  `trace_filter` blowup ([#2007](https://github.com/graphprotocol/graph-node/issues/2007))

### 8.6 BigInt / BigDecimal
- `BigInt / BigInt` truncates to 0 (Uniswap case) ·
  [#720](https://github.com/graphprotocol/graph-node/issues/720)
- BigDecimal 34-sig-digit ceiling (silent precision loss) ·
  [graph-ts API](https://thegraph.com/docs/en/subgraphs/developing/creating/graph-ts/api/)
- Divide-by-zero = fatal sync halt ·
  [#5281](https://github.com/graphprotocol/graph-node/issues/5281)

### 8.7 Schema / manifest drift
- Handler silently never fires on signature/topic0 mismatch ·
  [manifest](https://thegraph.com/docs/en/subgraphs/developing/creating/subgraph-manifest/)
- Tuple/struct events get malformed codegen signature ·
  [graph-tooling #520](https://github.com/graphprotocol/graph-tooling/issues/520)
- `missing value for non-nullable field` across graph-cli versions ·
  [#5026](https://github.com/graphprotocol/graph-node/issues/5026)

### 8.8 Performance levers (measured / documented)
- Edge & Node benchmark (baseline mutable+String 268 ms/block): Bytes 225 ms (16%),
  immutable 217 ms (19%), both 194 ms (28%); storage 143→74 GB (48%) ·
  [benchmark](https://medium.com/edge-node-engineering/two-simple-subgraph-performance-improvements-a76c6b3e7eac)
- Auto `startBlock` = highest single win · `prune: auto` default · declared calls
  (5–10× vendor figure) · timeseries/aggregations push rollups into the DB

### 8.9 The Subgraph Linter (our compile-error target list)
`entity-overwrite` · `unexpected-null` · `unchecked-load` · `unchecked-nonnull` ·
`division-guard` · `derived-field-guard` · `helper-return-contract` ·
`undeclared-eth-call` ·
[subgraph-linter](https://thegraph.com/docs/en/subgraphs/guides/subgraph-linter/)

**Confidence notes:** the hard performance numbers (16/19/28/48%, 268→194 ms,
143→74 GB) come from a single Edge & Node benchmark and are "up to" / workload-
dependent (counter-example: [#3534](https://github.com/graphprotocol/graph-node/issues/3534)).
The 5–10× declared-calls figure is vendor-stated (Goldsky), not first-party.
`Date.now`/float determinism and a couple of Messari sidechain/timing items are the
lowest-confidence (inferred / search-surfaced) — everything else is verified against
a directly-fetched primary source.

---

*Owner: the Lodestar team. This roadmap is a living document — Stage 0 is done; the
2026 goal is to make "use Redstart" a no-brainer by turning the linter into compile
errors, shipping an optimising compiler, and being agent-native by default.*
