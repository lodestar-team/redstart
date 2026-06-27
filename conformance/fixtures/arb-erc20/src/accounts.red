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

// Tests run natively (no WASM, no Docker): `redstart test`.
test "a transfer debits the sender and credits the receiver" {
  Token.Transfer({ from: 0x01, to: 0x02, value: 100 })
  assertEq(Account.at(0x02).balance, 100)
  assert(Account.at(0x01).balance < 0)
}

test "approval writes the on-chain balance read via a contract call" {
  mockCall(ERC20.balanceOf(0x05), 4200)
  Token.Approval({ owner: 0x05, spender: 0x06, value: 1 })
  assertEq(Account.at(0x05).balance, 4200)
}
