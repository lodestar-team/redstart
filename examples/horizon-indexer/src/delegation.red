// StakingExtension — delegation lifecycle, plus a DelegationData timeseries point
// per event that feeds the DelegationDailyAgg rollup.

fn getOrCreateDelegation(indexerId: Bytes, delegator: Bytes, ts: BigInt) -> Delegation {
  let id = indexerId.toHexString() + "-" + delegator.toHexString()
  let delegation = Delegation.loadOrCreate(id, {
    indexer: indexerId,
    delegator: delegator,
    tokens: BigInt.zero,
    shares: BigInt.zero,
    lockedTokens: BigInt.zero,
    lockedUntil: BigInt.zero,
    createdAt: ts,
    lastUpdatedAt: ts,
  })
  return delegation
}

handler on StakingExtension.StakeDelegated(event) {
  let indexer = getOrCreateIndexer(event.params.indexer)
  let delegation = getOrCreateDelegation(event.params.indexer, event.params.delegator, event.block.timestamp)

  delegation.tokens = delegation.tokens + event.params.tokens
  delegation.shares = delegation.shares + event.params.shares
  delegation.lastUpdatedAt = event.block.timestamp

  indexer.totalDelegated = indexer.totalDelegated + event.params.tokens

  let ts = DelegationData.create(event.block.timestamp.toI64(), {
    timestamp: event.block.timestamp.toI64(),
    indexer: event.params.indexer,
    delegator: event.params.delegator,
    tokens: event.params.tokens,
    eventType: "delegated",
  })
}

handler on StakingExtension.StakeDelegatedLocked(event) {
  let indexer = getOrCreateIndexer(event.params.indexer)
  let delegation = getOrCreateDelegation(event.params.indexer, event.params.delegator, event.block.timestamp)

  delegation.lockedTokens = delegation.lockedTokens + event.params.tokens
  delegation.lockedUntil = event.params.until
  delegation.lastUpdatedAt = event.block.timestamp

  let ts = DelegationData.create(event.block.timestamp.toI64(), {
    timestamp: event.block.timestamp.toI64(),
    indexer: event.params.indexer,
    delegator: event.params.delegator,
    tokens: event.params.tokens,
    eventType: "locked",
  })
}

handler on StakingExtension.StakeDelegatedWithdrawn(event) {
  let indexer = getOrCreateIndexer(event.params.indexer)
  let delegation = getOrCreateDelegation(event.params.indexer, event.params.delegator, event.block.timestamp)

  delegation.tokens = delegation.tokens - event.params.tokens
  delegation.lockedTokens = BigInt.zero
  delegation.lastUpdatedAt = event.block.timestamp

  indexer.totalDelegated = indexer.totalDelegated - event.params.tokens

  let ts = DelegationData.create(event.block.timestamp.toI64(), {
    timestamp: event.block.timestamp.toI64(),
    indexer: event.params.indexer,
    delegator: event.params.delegator,
    tokens: event.params.tokens,
    eventType: "withdrawn",
  })
}
