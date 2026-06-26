// Shared helpers — `fn` declarations Redstart lowers to AssemblyScript and the
// handlers call directly. `match` on the nullable `load` makes the
// "increment the indexer count only on first sight" logic explicit and
// null-safe; the entities are auto-saved at each `return`.

fn getOrCreateGlobalStats() -> GlobalStats {
  let stats = GlobalStats.loadOrCreate("global", {
    totalIndexers: 0,
    totalOperators: 0,
    totalAllocations: 0,
    totalActiveAllocations: 0,
    totalRewardsDistributed: BigInt.zero,
    totalQueryFeesCollected: BigInt.zero,
  })
  return stats
}

fn getOrCreateIndexer(addr: Bytes) -> Indexer {
  let existing = Indexer.load(addr)
  match existing {
    Some(found) => {
      return found
    }
    None => {
      let indexer = Indexer.create(addr, {
        totalStaked: BigInt.zero,
        totalDelegated: BigInt.zero,
        totalRewardsEarned: BigInt.zero,
        totalQueryFeesCollected: BigInt.zero,
        allocationCount: 0,
        activeAllocationCount: 0,
        registeredAt: BigInt.zero,
      })
      let stats = getOrCreateGlobalStats()
      stats.totalIndexers = stats.totalIndexers + 1
      return indexer
    }
  }
}
