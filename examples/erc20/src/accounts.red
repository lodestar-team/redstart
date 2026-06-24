// Entities for the ERC-20 tracker. Loaded as the `accounts` module via
// `mod accounts;` in main.red.

entity Account {
  id: Id<Bytes>
  balance: BigInt
  // `Option<String>` makes this nullable in the generated schema — there is no
  // `null` in Redstart, so nullability is always explicit.
  label: Option<String>
  transfersOut: [Transfer] derived from from
}

// Immutable entities can never be updated after creation — graph-node can store
// them far more cheaply. The modifier flows straight into the @entity directive.
entity Transfer immutable {
  id: Id<Bytes>
  from: Account
  to: Account
  value: BigInt
  timestamp: BigInt
}
