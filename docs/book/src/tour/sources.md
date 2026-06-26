# Sources & ABIs

A **source** is an on-chain contract you index. An **ABI** import gives Redstart
the contract's interface, which it uses to type events and contract calls — and
to derive the event signatures written into the manifest.

```redstart
abi ERC20 from "./abis/ERC20.json"

source Token {
  abi: ERC20
  network: mainnet
  address: 0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48
  startBlock: 6082465
}
```

Because the event signature in the generated `subgraph.yaml`
(`Transfer(indexed address,indexed address,uint256)`) is derived from the ABI
*by reference*, renaming or mistyping an event is a **compile** error rather than
a silent runtime mismatch.

## Templates (dynamic data sources)

When a factory contract spawns new contracts at runtime, declare a `template` and
instantiate it from a handler:

```redstart
template Pair {
  abi: UniswapV2Pair
  network: mainnet
}

handler on Factory.PairCreated(event) {
  Pair.create(event.params.pair)
  // or with context:
  // Pair.createWithContext(event.params.pair, ctx)
}
```

## File data sources (IPFS / off-chain metadata)

```redstart
template TokenMetadata { kind: file }

handler file TokenMetadata(content) {
  // `content` is the fetched file bytes
}
```

This renders to a `kind: file/ipfs` data source — the standard off-chain-metadata
pattern.
