// A factory/pair-template subgraph — the canonical dynamic-data-source pattern.
//
// Exercises, in one project, every feature beyond the ERC-20 vertical slice:
//   - event, call, and block handlers (on both a source and a template)
//   - dynamic data sources: `<Template>.create` / `createWithContext` + context
//   - control flow: `if` and a `for` range loop
//   - an `enum` field and graph-ts namespaces (`log`)
//
// `./conformance/run.sh build PROJECT=examples/factory` proves the eject path:
// `graph codegen` + `graph build` compile all of it to WASM unmodified.

abi Factory from "./abis/Factory.json"
abi Pool from "./abis/Pool.json"

enum Side {
  Buy
  Sell
}

entity Pair {
  id: Id<Bytes>
  swapCount: BigInt
}

entity Swap {
  id: Id<Bytes>
  side: Side
  amount: BigInt
}

entity Snapshot {
  id: Id<Bytes>
  total: BigInt
}

source FactoryContract {
  abi: Factory
  network: mainnet
  address: 0x1F98431c8aD98523631AE4a59f267346ea31F984
  startBlock: 1
}

template PoolTemplate {
  abi: Pool
  network: mainnet
}

// Event handler on the factory: spawns a pair template carrying context.
handler on FactoryContract.PoolCreated(event) {
  let pair = Pair.create(event.params.pool, { swapCount: BigInt.zero })
  let ctx = DataSourceContext.new()
  ctx.setBytes("token0", event.params.token0)
  PoolTemplate.createWithContext(event.params.pool, ctx)
}

// Event handler on the template: control flow + an enum field.
handler on PoolTemplate.Swap(event) {
  let pair = Pair.loadOrCreate(event.address, { swapCount: BigInt.zero })
  let s = Swap.create(event.id, { side: "Buy", amount: event.params.amount0 })
  if event.params.amount0 > BigInt.zero {
    s.side = "Sell"
  }
  for i in 0..3 {
    pair.swapCount = pair.swapCount + BigInt.fromI32(1)
  }
}

// Call handler on the template.
handler call PoolTemplate.swap(call) {
  log.info("swap routed to {}", [call.inputs.to.toHexString()])
}

// Block handler on the template (polling).
handler block PoolTemplate(block) every 1000 {
  let snap = Snapshot.create(block.hash, { total: block.number })
}
