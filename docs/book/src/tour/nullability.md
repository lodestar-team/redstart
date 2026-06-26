# Nullability & no `null`

There is no `null` in Redstart. Anything that might be absent is `Option<T>`, and
the compiler forces you to handle the empty case before you touch the value.

```redstart
entity Account {
  id: Id<Bytes>
  label: Option<String>     // nullable in the generated schema
}
```

`label` renders as `label: String` (nullable) in `schema.graphql`, while a plain
`balance: BigInt` renders as `balance: BigInt!` (required).

## Why this matters

In hand-written AssemblyScript, arithmetic on a nullable value silently
miscompiles, and a forgotten null check aborts the handler at runtime. Redstart
makes both **compile** errors:

- **Arithmetic on `Option`** is rejected — you must unwrap first.
- **Dereferencing a nullable host return** is rejected. `store.get`-style loads
  (`Entity.load`, `loadInBlock`) and `ipfs.cat` return `Option<T>`; you must
  `match` them before use:

```redstart
match Account.load(id) {
  Some(account) => {
    account.balance = account.balance + amount   // matched binding auto-saves
  }
  None => {
    // nothing to update
  }
}
```

`loadOrCreate` exists precisely so the common case doesn't need a `match` — it
always returns a live entity.
