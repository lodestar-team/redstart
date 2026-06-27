// Conformance fixture: ARB token on Arbitrum One, a short settled window.
// Store-diffed against conformance/reference/erc20/mappings.ts.
mod accounts;

abi ERC20 from "./abis/ERC20.json"

source Token {
  abi: ERC20
  network: "arbitrum-one"
  address: 0x912CE59144191C1204E64559FE8253a0e49E6548
  startBlock: 477660236
}

handler on Token.Transfer(event) {
  let sender = accounts::Account.loadOrCreate(event.params.from, { balance: BigInt.zero })
  let receiver = accounts::Account.loadOrCreate(event.params.to, { balance: BigInt.zero })

  sender.balance = sender.balance - event.params.value
  receiver.balance = receiver.balance + event.params.value

  let transfer = accounts::Transfer.create(event.id, {
      from: event.params.from,
      to: event.params.to,
      value: event.params.value,
      timestamp: event.block.timestamp,
  })
}

handler on Token.Approval(event) {
  let result = ERC20.bind(event.address).balanceOf(event.params.owner)
  match result {
    Ok(currentBalance) => {
      let owner = accounts::Account.loadOrCreate(event.params.owner, { balance: BigInt.zero })
      owner.balance = currentBalance
    }
    Err(e) => {}
  }
}
