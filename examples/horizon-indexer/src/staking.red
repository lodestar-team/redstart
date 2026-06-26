// HorizonStaking — operator authorizations.

handler on HorizonStaking.OperatorSet(event) {
  let indexer = getOrCreateIndexer(event.params.serviceProvider)

  let id = event.params.serviceProvider.toHexString() + "-" + event.params.verifier.toHexString() + "-" + event.params.operator.toHexString()

  let existing = Operator.load(id)
  match existing {
    Some(op) => {
      let wasActive = op.active
      op.active = event.params.allowed
      op.setAt = event.block.timestamp
      op.setTx = event.transaction.hash

      let stats = getOrCreateGlobalStats()
      if !wasActive && event.params.allowed {
        stats.totalOperators = stats.totalOperators + 1
      } else if wasActive && !event.params.allowed {
        stats.totalOperators = stats.totalOperators - 1
      }
    }
    None => {
      let op = Operator.create(id, {
        indexer: indexer,
        operator: event.params.operator,
        verifier: event.params.verifier,
        active: event.params.allowed,
        setAt: event.block.timestamp,
        setTx: event.transaction.hash,
      })
      if event.params.allowed {
        let stats = getOrCreateGlobalStats()
        stats.totalOperators = stats.totalOperators + 1
      }
    }
  }
}
