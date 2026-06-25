// An ERC-721 subgraph with an IPFS **file data source** for token metadata.
//
// A `Transfer` spawns a `file` template pointed at the token's metadata CID; the
// file handler receives the fetched bytes as `Bytes` and indexes them. This is
// the off-chain-metadata pattern (NFTs, content registries) — proven through the
// eject path with `./conformance/run.sh build PROJECT=examples/file-metadata`.

abi ERC721 from "./abis/ERC721.json"

entity Token {
  id: Id<Bytes>
  uri: String
}

source NFT {
  abi: ERC721
  network: mainnet
  address: 0xBC4CA0EdA7647A8aB7C2061c2E118A18a936f13D
  startBlock: 1
}

// A file/IPFS data source — no contract, no network; just a content handler.
template TokenMetadata {
  kind: file
}

handler on NFT.Transfer(event) {
  let token = Token.create(event.id, { uri: "ipfs://placeholder" })
  // Spawn the file data source for this token's metadata document.
  TokenMetadata.create("QmTokenMetadataCid")
}

handler file TokenMetadata(content) {
  let value = json.fromBytes(content)
  let cid = dataSource.stringParam()
  let token = Token.create(Bytes.fromUTF8(cid), { uri: cid })
}
