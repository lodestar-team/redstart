import { newMockEvent } from "matchstick-as/assembly/index";
import { ethereum, Address, BigInt } from "@graphprotocol/graph-ts";
import { Transfer } from "../generated/Token/Token";
export function createTransferEvent(from: string, to: string, value: i32): Transfer {
  let e = changetype<Transfer>(newMockEvent());
  e.parameters = new Array();
  e.parameters.push(new ethereum.EventParam("from", ethereum.Value.fromAddress(Address.fromString(from))));
  e.parameters.push(new ethereum.EventParam("to", ethereum.Value.fromAddress(Address.fromString(to))));
  e.parameters.push(new ethereum.EventParam("value", ethereum.Value.fromUnsignedBigInt(BigInt.fromI32(value))));
  return e;
}
