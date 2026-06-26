// Horizon indexer — entry module.
//
// A faithful Redstart rewrite of PaulieB14's horizon-indexer-subgraph
// (https://github.com/PaulieB14/horizon-indexer-subgraph): three Arbitrum One
// contracts, one unified source of truth. Entities, helpers, and handlers live
// in sibling modules; this file wires the ABIs and data sources.

mod schema;
mod helpers;
mod staking;
mod delegation;
mod subgraph_service;
mod tests;

abi HorizonStaking from "./abis/HorizonStaking.json"
abi SubgraphService from "./abis/SubgraphService.json"
abi StakingExtension from "./abis/StakingExtension.json"

source HorizonStaking {
  abi: HorizonStaking
  network: "arbitrum-one"
  address: 0x00669A4CF01450B64E8A2A20E9b1FCB71E61eF03
  startBlock: 430000000
}

source SubgraphService {
  abi: SubgraphService
  network: "arbitrum-one"
  address: 0xb2Bb92d0DE618878E438b55D5846cfecD9301105
  startBlock: 397000000
}

source StakingExtension {
  abi: StakingExtension
  network: "arbitrum-one"
  address: 0x3bE385576d7C282070Ad91BF94366de9f9ba3571
  startBlock: 397000000
}
