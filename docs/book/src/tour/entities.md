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
