// Native handler tests — `redstart test`. No WASM, no Docker, no graph-node:
// fire an event, assert on the mock store. These exercise the helper functions
// (getOrCreateIndexer / getOrCreateGlobalStats) end to end.

test "operator set creates the indexer, operator, and bumps global stats" {
  HorizonStaking.OperatorSet({ serviceProvider: 0x01, verifier: 0x02, operator: 0x03, allowed: true })
  assertEq(GlobalStats.at("global").totalIndexers, 1)
  assertEq(GlobalStats.at("global").totalOperators, 1)
}

test "delegation credits the indexer's total and records the delegation" {
  HorizonStaking.OperatorSet({ serviceProvider: 0x01, verifier: 0x02, operator: 0x03, allowed: true })
  StakingExtension.StakeDelegated({ indexer: 0x01, delegator: 0x09, tokens: 1000, shares: 500 })
  assertEq(Indexer.at(0x01).totalDelegated, 1000)
}

test "withdrawing debits the indexer's delegated total" {
  StakingExtension.StakeDelegated({ indexer: 0x01, delegator: 0x09, tokens: 1000, shares: 500 })
  StakingExtension.StakeDelegatedWithdrawn({ indexer: 0x01, delegator: 0x09, tokens: 400 })
  assertEq(Indexer.at(0x01).totalDelegated, 600)
}

test "allocation created bumps counts and global stats" {
  SubgraphService.AllocationCreated({ indexer: 0x01, allocationId: 0xAA, subgraphDeploymentId: 0xBB, tokens: 5000, currentEpoch: 7 })
  assertEq(Allocation.at(0xAA).tokens, 5000)
  assertEq(Indexer.at(0x01).allocationCount, 1)
  assertEq(Indexer.at(0x01).activeAllocationCount, 1)
  assertEq(GlobalStats.at("global").totalAllocations, 1)
}

test "closing an allocation flips status and decrements the active count" {
  SubgraphService.AllocationCreated({ indexer: 0x01, allocationId: 0xAA, subgraphDeploymentId: 0xBB, tokens: 5000, currentEpoch: 7 })
  SubgraphService.AllocationClosed({ indexer: 0x01, allocationId: 0xAA, subgraphDeploymentId: 0xBB, tokens: 5000, forceClosed: false })
  assertEq(Indexer.at(0x01).activeAllocationCount, 0)
  assertEq(GlobalStats.at("global").totalActiveAllocations, 0)
}

test "indexing rewards accumulate on the indexer and global stats" {
  SubgraphService.IndexingRewardsCollected({ indexer: 0x01, allocationId: 0xAA, subgraphDeploymentId: 0xBB, tokensRewards: 300, tokensIndexerRewards: 250, tokensDelegationRewards: 50, currentEpoch: 7 })
  assertEq(Indexer.at(0x01).totalRewardsEarned, 300)
  assertEq(GlobalStats.at("global").totalRewardsDistributed, 300)
}

test "query fees accumulate on the indexer and global stats" {
  SubgraphService.QueryFeesCollected({ serviceProvider: 0x01, payer: 0x08, allocationId: 0xAA, subgraphDeploymentId: 0xBB, tokensCollected: 120, tokensCurators: 20 })
  assertEq(Indexer.at(0x01).totalQueryFeesCollected, 120)
  assertEq(GlobalStats.at("global").totalQueryFeesCollected, 120)
}
