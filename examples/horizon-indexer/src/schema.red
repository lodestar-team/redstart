// Entities, timeseries, and aggregations for the Horizon indexer.
//
// One source of truth: these declarations generate schema.graphql, and the
// handlers in the sibling modules are type-checked against them.

entity Indexer {
  id: Id<Bytes>
  operators: [Operator] derived from indexer
  allocations: [Allocation] derived from indexer
  delegations: [Delegation] derived from indexer
  totalStaked: BigInt
  totalDelegated: BigInt
  totalRewardsEarned: BigInt
  totalQueryFeesCollected: BigInt
  allocationCount: Int
  activeAllocationCount: Int
  registeredAt: BigInt
  url: Option<String>
}

entity Operator {
  id: Id<String>
  indexer: Indexer
  operator: Bytes
  verifier: Bytes
  active: Bool
  setAt: BigInt
  setTx: Bytes
}

entity Allocation {
  id: Id<Bytes>
  indexer: Indexer
  subgraphDeploymentID: Bytes
  tokens: BigInt
  createdAtEpoch: BigInt
  createdAt: BigInt
  createdTx: Bytes
  closedAtEpoch: Option<BigInt>
  closedAt: Option<BigInt>
  closedTx: Option<Bytes>
  status: String
  rewardsEarned: BigInt
  queryFeesCollected: BigInt
  poi: Option<Bytes>
}

entity Delegation {
  id: Id<String>
  indexer: Indexer
  delegator: Bytes
  tokens: BigInt
  shares: BigInt
  lockedTokens: BigInt
  lockedUntil: BigInt
  createdAt: BigInt
  lastUpdatedAt: BigInt
}

entity RewardEvent immutable {
  id: Id<Bytes>
  indexer: Indexer
  allocationID: Bytes
  subgraphDeploymentID: Bytes
  tokensRewards: BigInt
  tokensIndexerRewards: BigInt
  tokensDelegationRewards: BigInt
  epoch: BigInt
  timestamp: BigInt
  tx: Bytes
}

entity QueryFeeEvent immutable {
  id: Id<Bytes>
  indexer: Indexer
  allocationID: Bytes
  subgraphDeploymentID: Bytes
  tokensCollected: BigInt
  tokensCurators: BigInt
  timestamp: BigInt
  tx: Bytes
}

entity GlobalStats {
  id: Id<String>
  totalIndexers: Int
  totalOperators: Int
  totalAllocations: Int
  totalActiveAllocations: Int
  totalRewardsDistributed: BigInt
  totalQueryFeesCollected: BigInt
}

// ── Timeseries & aggregations ──

entity RewardData timeseries {
  id: Int8
  timestamp: Timestamp
  indexer: Bytes
  subgraphDeploymentID: Bytes
  tokensRewards: BigInt
  tokensIndexerRewards: BigInt
  tokensDelegationRewards: BigInt
}

aggregation RewardDailyAgg over RewardData every [hour, day] {
  totalRewards: BigInt = sum(tokensRewards)
  totalIndexerRewards: BigInt = sum(tokensIndexerRewards)
  totalDelegationRewards: BigInt = sum(tokensDelegationRewards)
  rewardCount: Int8 = count()
}

entity QueryFeeData timeseries {
  id: Int8
  timestamp: Timestamp
  indexer: Bytes
  subgraphDeploymentID: Bytes
  tokensCollected: BigInt
  tokensCurators: BigInt
}

aggregation QueryFeeDailyAgg over QueryFeeData every [hour, day] {
  totalCollected: BigInt = sum(tokensCollected)
  totalCurators: BigInt = sum(tokensCurators)
  feeCount: Int8 = count()
}

entity DelegationData timeseries {
  id: Int8
  timestamp: Timestamp
  indexer: Bytes
  delegator: Bytes
  tokens: BigInt
  eventType: String
}

aggregation DelegationDailyAgg over DelegationData every [day] {
  totalTokens: BigInt = sum(tokens)
  eventCount: Int8 = count()
}
