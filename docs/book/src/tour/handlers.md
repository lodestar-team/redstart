# Handlers

Handlers are where indexing logic lives. Redstart has three kinds, each mapping
to the corresponding manifest section.

## Event handlers

```redstart
handler on Token.Transfer(event) {
  let sender = Account.loadOrCreate(event.params.from, { balance: BigInt.zero })
  let receiver = Account.loadOrCreate(event.params.to, { balance: BigInt.zero })

  sender.balance = sender.balance - event.params.value
  receiver.balance = receiver.balance + event.params.value
  // Both entities are dirty-tracked and auto-saved at handler end.
}
```

`event.params` is typed from the ABI. `event.block`, `event.transaction`, and
`event.address` are available as usual.

## Auto-save, by construction

You never call `.save()`. Entities you load or create are dirty-tracked and
flushed when the handler returns (and at `return` from any helper). Forgetting to
save — one of the most common subgraph bugs — is simply not expressible.

## Call & block handlers

```redstart
handler call Token.transfer(call) {
  // call.inputs / call.outputs are ABI-typed
}

handler block Token every 100 {
  // runs every 100 blocks; `once` is also supported
}
```

These render to `callHandlers` and `blockHandlers` (with the polling/once filter)
respectively.

## Control flow

Handlers and helpers support `if`/`else if`/`else`, `while`, and `for` over
numeric ranges and lists, plus array literals and indexing — all lowered to
native AssemblyScript:

```redstart
for holder in holders {
  if holder.balance > BigInt.zero {
    activeCount = activeCount + 1
  }
}
```
