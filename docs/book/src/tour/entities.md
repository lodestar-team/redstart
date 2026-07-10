# Entities & the schema

Entities are the heart of a subgraph — they define what gets stored and queried.
In Redstart they're declared once and projected into `schema.graphql`
automatically.

```redstart
entity Account {
  id: Id<Bytes>
  balance: BigInt
  label: Option<String>          // nullable — see the nullability chapter
  transfersOut: [Transfer] derived from from
}
```

- `id: Id<Bytes>` marks the primary key. `Id<Bytes>` and `Id<String>` are the two
  forms graph-node supports.
- `[Transfer] derived from from` is a derived (virtual) field: it's computed from
  the `from` field on `Transfer`, never written directly. Assigning to it is a
  compile error.

## Immutable entities

```redstart
entity Transfer immutable {
  id: Id<Bytes>
  from: Account
  to: Account
  value: BigInt
  timestamp: BigInt
}
```

Immutable entities can never be updated after creation, so graph-node stores them
far more cheaply. The modifier flows straight into the `@entity(immutable: true)`
directive in the generated schema.

## Bytes ids vs String ids

`Id<Bytes>` and `Id<String>` are the two id forms graph-node supports, but they
are not equal: a `Bytes` id indexes ~28% faster and stores ~48% less than the
same value kept as a hex string (Edge & Node benchmark). So when an entity is
keyed on a single address or bytes value, key it on the raw value — not on
`value.toHexString()`.

The checker flags the stringified form with **W040**, and there's an opt-in
autofix:

```console
$ redstart fix --ids            # or --dry-run to preview
  ✓ Holder  (2 sites) → Id<Bytes>
  ⤫ Ledger  skipped: keyed on a literal string id (src/ledger.red:12)
```

It flips the entity's declaration to `Id<Bytes>` and drops the `.toHexString()`
at every construction site, in one pass — including the common `let id =
addr.toHexString(); E.create(id, …)` shape (when `id` is used only there). It is
deliberately conservative: an entity is only converted when *every* one of its id
sites is a single stringified address/bytes value — one literal-string or composite
(`a + "-" + b`) id and the whole entity is left untouched and reported. Genuine
composite keys are really strings and stay `Id<String>`.

Because a `Bytes` id changes the *stored* id representation (hex-string → raw
bytes), this is a real data change — redeploy affected subgraphs from the
relevant block.

## One-to-many relations: `derived from`, not stored arrays

To model "a Pool has many Accounts", reach for a derived relation — never a stored
array of entity references:

```redstart
entity Account {
  id: Id<Bytes>
  pool: Pool                       // the back-reference
}

entity Pool {
  id: Id<Bytes>
  accounts: [Account] derived from pool   // computed, never stored
}
```

A *stored* `[Account]` (an entity array without `derived from`) is kept inline by
graph-node, which rewrites the entire array into a new versioned row on every
append — **O(n²) disk** as the relation grows. A `derived from` field is a reverse
lookup computed on read, so appends stay O(1). The checker flags the stored form
with **W050**:

```console
$ redstart check
  ! `accounts` stores an array of `Account` entities
  help: model this one-to-many with `@derivedFrom`: add a back-ref field on
        `Account` (e.g. `pool: Pool`) and declare `accounts: [Account] derived from pool`
```

Scalar and enum arrays (`[String]`, `[BigInt]`, `[TokenStandard]`) are genuinely
stored values and are never flagged — only arrays of *entities* are.

## Enums, interfaces, and scalars

```redstart
enum TokenStandard { ERC20, ERC721, ERC1155 }

interface Token {
  id: Id<Bytes>
  symbol: String
}

entity FungibleToken implements Token {
  id: Id<Bytes>
  symbol: String
  decimals: Int8
}
```

`implements` is checked for field completeness — leave out a field the interface
requires and it won't compile. The full graph-node scalar set is available,
including `Int8`, `Timestamp`, `BigInt`, `BigDecimal`, `Bytes`, and `Boolean`.

## Timeseries & aggregations

```redstart
entity Swap timeseries {
  price: BigDecimal
}

aggregation PriceStats over Swap every [hour, day] {
  total: BigDecimal = sum(price)
}
```

Timeseries entities get an automatic `id`/`timestamp` and are implicitly
immutable. Aggregations render to `@aggregation`/`@aggregate` and automatically
bump the manifest `specVersion` to 1.1.0.
