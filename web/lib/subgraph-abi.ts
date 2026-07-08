// Minimal ABI model + helpers shared by the proxy route and the generator UI.
// We only need enough of the ABI to (a) list events for the checkbox UI and
// (b) hand a clean, decluttered ABI to the LLM.

export interface AbiParam {
  name: string;
  type: string;
  indexed?: boolean;
  internalType?: string;
  components?: AbiParam[];
}

export interface AbiItem {
  type: string; // "event" | "function" | "constructor" | "fallback" | ...
  name?: string;
  inputs?: AbiParam[];
  outputs?: AbiParam[];
  anonymous?: boolean;
  stateMutability?: string;
}

export interface EventDef {
  name: string;
  /** Human signature, e.g. `Transfer(address,address,uint256)`. */
  signature: string;
  inputs: AbiParam[];
}

/** The normalized contract payload the proxy returns and the UI/LLM consume. */
export interface ContractInfo {
  address: string;
  chainId: number;
  graphNetwork: string;
  name: string;
  /** True when the ABI was recovered rather than read from a verified source. */
  reconstructed: boolean;
  verified: boolean;
  isProxy: boolean;
  implementation?: string;
  startBlock?: number;
  abi: AbiItem[];
  events: EventDef[];
  /** Rough token-standard guess to bias entity templates. */
  kind: "erc20" | "erc721" | "erc1155" | "unknown";
}

export function paramSig(p: AbiParam): string {
  if (p.type.startsWith("tuple")) {
    const inner = (p.components ?? []).map(paramSig).join(",");
    // preserve any array suffix on the tuple, e.g. tuple[] -> (..)[]
    const suffix = p.type.slice("tuple".length);
    return `(${inner})${suffix}`;
  }
  return p.type;
}

export function eventSignature(item: AbiItem): string {
  const args = (item.inputs ?? []).map(paramSig).join(",");
  return `${item.name}(${args})`;
}

export function extractEvents(abi: AbiItem[]): EventDef[] {
  return abi
    .filter((i) => i.type === "event" && i.name)
    .map((i) => ({
      name: i.name as string,
      signature: eventSignature(i),
      inputs: i.inputs ?? [],
    }));
}

/** Heuristic token-standard detection from the function/interface surface. */
export function detectKind(abi: AbiItem[]): ContractInfo["kind"] {
  const fns = new Set(
    abi.filter((i) => i.type === "function" && i.name).map((i) => i.name as string),
  );
  const events = new Set(
    abi.filter((i) => i.type === "event" && i.name).map((i) => i.name as string),
  );
  // ERC-1155: TransferSingle/TransferBatch are the tell.
  if (events.has("TransferSingle") || events.has("TransferBatch")) return "erc1155";
  // ERC-721: Transfer + ownerOf/tokenURI, and a non-value Transfer (tokenId).
  if (fns.has("ownerOf") || fns.has("tokenURI") || fns.has("safeTransferFrom")) {
    if (events.has("Transfer")) return "erc721";
  }
  // ERC-20: the classic quartet.
  if (fns.has("balanceOf") && fns.has("transfer") && (fns.has("decimals") || fns.has("totalSupply"))) {
    return "erc20";
  }
  return "unknown";
}
