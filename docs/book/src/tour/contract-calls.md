# Contract calls & `match`

Reading state from a contract can revert. In hand-written AssemblyScript an
unguarded call aborts the whole handler; the idiomatic fix (`try_*`) is easy to
forget. Redstart makes the safe path the only path: a contract call returns a
`Result`, and you must `match` it before touching the value.

```redstart
handler on Token.Approval(event) {
  let result = ERC20.bind(event.address).balanceOf(event.params.owner)
  match result {
    Ok(currentBalance) => {
      let owner = Account.loadOrCreate(event.params.owner, { balance: BigInt.zero })
      owner.balance = currentBalance
    }
    Err(e) => {
      // call reverted — leave balances untouched
    }
  }
}
```

This lowers to graph-ts's `try_balanceOf()` and a `.reverted` check — the correct
pattern, generated for you. Accessing `.value` without a surrounding `match` is a
compile error (`E…: .value-without-match`).

## Binding

`ERC20.bind(address)` produces a typed contract instance. Every function in the
imported ABI is available, with ABI-typed parameters and return values.
