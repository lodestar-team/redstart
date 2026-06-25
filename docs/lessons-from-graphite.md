# Lessons from graphite

[`cargopete/graphite`](https://github.com/cargopete/graphite) was a prior attempt
to author The Graph subgraphs in Rust by compiling **Rust directly to
AssemblyScript-ABI-compatible WASM** (via a `graph-as-runtime` crate that
emulates AssemblyScript's memory model). It got two subgraphs (ERC-20, ERC-721)
live on Arbitrum One — proof the *concept* of alternative authoring reaching
graph-node is sound — but stalled. Its experience directly shapes Redstart.

## The core lesson: never hand-match the AS ABI

graphite had to reproduce, byte-for-byte, what `asc` emits. The hard parts (all
in `graph-as-runtime/src/`):

- **Object headers** — every heap object needs a 16-byte header (`mm_info`,
  `gc_info`, `rt_id`, `rt_size`); the pointer handed to graph-node points *past*
  it. Off-by-one sizing → runtime "Size does not match".
- **`Value` enum padding** — `AscEnum<StoreValueKind>` is 16 bytes with an
  explicit 4-byte pad between `kind` and `payload`. Omitting it broke entity
  serialization.
- **UTF-16LE strings** with a byte-count `rt_size` (not char count).
- **`TypedMap` layout** — a pointer chain (TypedMap → Array(ArrayBufferView) →
  ArrayBuffer → entries), with the 0.0.5+ 16-byte array view.
- **Class IDs** (`ARRAY_STRING=27`, `TYPED_MAP=30`, `VALUE=35`, …) derived from
  `graph-ts@0.31` RTTI — they shift when graph-ts changes, with no auto-detection.

**Redstart's answer:** we emit readable AssemblyScript *source* and let the
canonical `asc`/`graph build` produce all of the above. The entire ABI-fidelity
surface — and the perpetual maintenance burden of tracking graph-ts versions —
simply does not exist for us. The [conformance gate](../conformance/) is the
backstop that proves the emitted source behaves identically to hand-written code.

## What graphite validates (we built equivalents independently)

- **Manifest generation** from ABI JSON, emitting event signatures with the
  `indexed` qualifier (`Transfer(indexed address,indexed address,uint256)`) — see
  our `redstart-codegen/src/manifest.rs` + `redstart-checker/src/abi.rs`.
- **Scalar mapping** (ID/Bytes/Address→Bytes, BigInt little-endian, etc.) — our
  `schema.rs` agrees.
- **`@derivedFrom` skipped in `save()`** — our lowering never writes derived fields.
- **Native testing without WASM/Postgres** — graphite's `MockHost` ≈ our
  `redstart-test` interpreter. Independent convergence on the same good idea.

## Why graphite stalled — and how Redstart differs

| graphite's friction | Redstart |
| --- | --- |
| Required a parallel SDK devs had to trust | Output is canonical AS; no runtime to trust |
| `graph-cli`/Studio didn't recognise Rust projects | We feed the official toolchain unmodified |
| Manifest said `language: wasm/assemblyscript` → confusion | It genuinely *is* AssemblyScript |
| `graph-as-runtime` needs constant sync with graph-ts | No such layer exists |
| Competed with official Substreams (Rust→WASM) on its own turf | We compete on *authoring ergonomics on the decentralized network*, not on being Rust |

The decisive difference is the **eject path**: abandoning Redstart costs nothing
but the generated AS, which keeps working. graphite had no such exit.

## Roadmap items graphite surfaces

Features graphite supported that Redstart's grammar/codegen doesn't yet:

- ~~**Call handlers** (`callHandlers` / `function: balanceOf(address)`).~~ ✅ done —
  `handler call Src.fn(call)`, with ABI-typed `call.inputs.*` / `call.outputs.*`.
- ~~**Block handlers** (`blockHandlers` with polling filters).~~ ✅ done —
  `handler block Src(block) [every N | once]`.
- ~~**File data sources** (IPFS/Arweave).~~ ✅ done (IPFS) —
  `template T { kind: file }` + `handler file T(content)`; see `examples/file-metadata`.
- **Transaction receipt context** (`receipt: true`).
- ~~**`redstart deploy`** — wrap `graph deploy` to Studio (graphite had `deploy`).~~
  ✅ done — `redstart deploy <slug> [--node --ipfs --version-label] [--dry-run]`.
- **Richer fixtures** — graphite ships ERC-721, ERC-1155, Uniswap V2, and
  multi-source examples; excellent conformance/coverage targets.
