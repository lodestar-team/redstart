import { Transfer as TransferEvent } from "../generated/Token/Token";
import { Transfer } from "../generated/schema";
export function handleTransfer(event: TransferEvent): void {
  let e = new Transfer(event.transaction.hash.concatI32(event.logIndex.toI32()));
  e.from = event.params.from; e.to = event.params.to; e.value = event.params.value; e.save();
}
