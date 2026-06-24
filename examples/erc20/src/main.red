// ERC-20 balance tracker — the Redstart vertical slice.
//
// This single file declares the ABI to import, the data source, and the event
// handler. Entities live in a sibling module (`accounts.red`) to show off the
// multi-file module system. All of it generates schema.graphql + subgraph.yaml
// from one source of truth — no drift possible.

mod accounts;

abi ERC20 from "./abis/ERC20.json"

source Token {
  abi: ERC20
  network: mainnet
  // USDC on Ethereum mainnet.
  address: 0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48
  startBlock: 6082465
}

handler on Token.Transfer(event) {
  let sender = accounts::Account.loadOrCreate(event.params.from, { balance: BigInt.zero })
  let receiver = accounts::Account.loadOrCreate(event.params.to, { balance: BigInt.zero })

  sender.balance = sender.balance - event.params.value
  receiver.balance = receiver.balance + event.params.value
  // Both entities are dirty-tracked and auto-saved at handler end.

  let transfer = accounts::Transfer.create(event.id, {
    from: event.params.from,
    to: event.params.to,
    value: event.params.value,
    timestamp: event.block.timestamp,
  })
}

handler on Token.Approval(event) {
  // Contract calls return Result — you must `match` before touching the value,
  // so a reverted call can never silently abort the handler.
  let result = ERC20.bind(event.address).balanceOf(event.params.owner)
  match result {
    Ok(currentBalance) => {
      let owner = accounts::Account.loadOrCreate(event.params.owner, { balance: BigInt.zero })
      owner.balance = currentBalance
    }
    Err(e) => {
      // call reverted — leave balances untouched
    }
  }
}
