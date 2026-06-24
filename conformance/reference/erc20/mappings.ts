// Hand-written CANONICAL reference mapping for the ERC-20 conformance fixture.
//
// This is what a careful human writes by hand. The conformance harness swaps it
// into a copy of Redstart's generated build/ (same schema, manifest, ABIs and
// indexed source) — so a store-diff isolates exactly one thing: Redstart's
// lowered AssemblyScript vs this idiomatic hand-written AssemblyScript.
//
// Keep this independently idiomatic — do NOT mirror codegen quirks. If Redstart's
// output diverges from this, the gate should catch it.

import { BigInt } from "@graphprotocol/graph-ts";
import {
  Transfer as TransferEvent,
  Approval as ApprovalEvent,
  ERC20,
} from "../generated/Token/ERC20";
import { Account, Transfer } from "../generated/schema";

export function handleTransfer(event: TransferEvent): void {
  let sender = Account.load(event.params.from);
  if (sender == null) {
    sender = new Account(event.params.from);
    sender.balance = BigInt.zero();
  }

  let receiver = Account.load(event.params.to);
  if (receiver == null) {
    receiver = new Account(event.params.to);
    receiver.balance = BigInt.zero();
  }

  sender.balance = sender.balance.minus(event.params.value);
  receiver.balance = receiver.balance.plus(event.params.value);
  sender.save();
  receiver.save();

  let transfer = new Transfer(
    event.transaction.hash.concatI32(event.logIndex.toI32())
  );
  transfer.from = event.params.from;
  transfer.to = event.params.to;
  transfer.value = event.params.value;
  transfer.timestamp = event.block.timestamp;
  transfer.save();
}

export function handleApproval(event: ApprovalEvent): void {
  let contract = ERC20.bind(event.address);
  let result = contract.try_balanceOf(event.params.owner);
  if (!result.reverted) {
    let owner = Account.load(event.params.owner);
    if (owner == null) {
      owner = new Account(event.params.owner);
      owner.balance = BigInt.zero();
    }
    owner.balance = result.value;
    owner.save();
  }
}
