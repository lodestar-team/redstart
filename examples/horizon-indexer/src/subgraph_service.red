// SubgraphService — allocation lifecycle, indexing rewards, and query fees.
// Nullable Option fields (closedAt, poi, …) are simply left unset on create —
// Redstart has no `null`, so "not yet closed" is the absence of a value.

handler on SubgraphService.AllocationCreated(event) {
  let indexer = getOrCreateIndexer(event.params.indexer)

  let alloc = Allocation.create(event.params.allocationId, {
    indexer: indexer,
    subgraphDeploymentID: event.params.subgraphDeploymentId,
    tokens: event.params.tokens,
    createdAtEpoch: event.params.currentEpoch,
    createdAt: event.block.timestamp,
    createdTx: event.transaction.hash,
    status: "Active",
    rewardsEarned: BigInt.zero,
    queryFeesCollected: BigInt.zero,
  })

  indexer.allocationCount = indexer.allocationCount + 1
  indexer.activeAllocationCount = indexer.activeAllocationCount + 1

  let stats = getOrCreateGlobalStats()
  stats.totalAllocations = stats.totalAllocations + 1
  stats.totalActiveAllocations = stats.totalActiveAllocations + 1
}

handler on SubgraphService.AllocationClosed(event) {
  let existing = Allocation.load(event.params.allocationId)
  match existing {
    Some(alloc) => {
      alloc.status = "Closed"
      alloc.closedAt = event.block.timestamp
      alloc.closedTx = event.transaction.hash

      let indexer = getOrCreateIndexer(event.params.indexer)
      indexer.activeAllocationCount = indexer.activeAllocationCount - 1

      let stats = getOrCreateGlobalStats()
      stats.totalActiveAllocations = stats.totalActiveAllocations - 1
    }
    None => {}
  }
}

handler on SubgraphService.AllocationResized(event) {
  let existing = Allocation.load(event.params.allocationId)
  match existing {
    Some(alloc) => {
      alloc.tokens = event.params.newTokens
    }
    None => {}
  }
}

handler on SubgraphService.IndexingRewardsCollected(event) {
  let reward = RewardEvent.create(event.id, {
    indexer: event.params.indexer,
    allocationID: event.params.allocationId,
    subgraphDeploymentID: event.params.subgraphDeploymentId,
    tokensRewards: event.params.tokensRewards,
    tokensIndexerRewards: event.params.tokensIndexerRewards,
    tokensDelegationRewards: event.params.tokensDelegationRewards,
    epoch: event.params.currentEpoch,
    timestamp: event.block.timestamp,
    tx: event.transaction.hash,
  })

  let existing = Allocation.load(event.params.allocationId)
  match existing {
    Some(alloc) => {
      alloc.rewardsEarned = alloc.rewardsEarned + event.params.tokensRewards
    }
    None => {}
  }

  let indexer = getOrCreateIndexer(event.params.indexer)
  indexer.totalRewardsEarned = indexer.totalRewardsEarned + event.params.tokensRewards

  let stats = getOrCreateGlobalStats()
  stats.totalRewardsDistributed = stats.totalRewardsDistributed + event.params.tokensRewards

  let ts = RewardData.create(event.block.timestamp.toI64(), {
    timestamp: event.block.timestamp.toI64(),
    indexer: event.params.indexer,
    subgraphDeploymentID: event.params.subgraphDeploymentId,
    tokensRewards: event.params.tokensRewards,
    tokensIndexerRewards: event.params.tokensIndexerRewards,
    tokensDelegationRewards: event.params.tokensDelegationRewards,
  })
}

handler on SubgraphService.QueryFeesCollected(event) {
  let fee = QueryFeeEvent.create(event.id, {
    indexer: event.params.serviceProvider,
    allocationID: event.params.allocationId,
    subgraphDeploymentID: event.params.subgraphDeploymentId,
    tokensCollected: event.params.tokensCollected,
    tokensCurators: event.params.tokensCurators,
    timestamp: event.block.timestamp,
    tx: event.transaction.hash,
  })

  let existing = Allocation.load(event.params.allocationId)
  match existing {
    Some(alloc) => {
      alloc.queryFeesCollected = alloc.queryFeesCollected + event.params.tokensCollected
    }
    None => {}
  }

  let indexer = getOrCreateIndexer(event.params.serviceProvider)
  indexer.totalQueryFeesCollected = indexer.totalQueryFeesCollected + event.params.tokensCollected

  let stats = getOrCreateGlobalStats()
  stats.totalQueryFeesCollected = stats.totalQueryFeesCollected + event.params.tokensCollected

  let ts = QueryFeeData.create(event.block.timestamp.toI64(), {
    timestamp: event.block.timestamp.toI64(),
    indexer: event.params.serviceProvider,
    subgraphDeploymentID: event.params.subgraphDeploymentId,
    tokensCollected: event.params.tokensCollected,
    tokensCurators: event.params.tokensCurators,
  })
}
