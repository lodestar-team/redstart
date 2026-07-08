import { assert, describe, test, clearStore, afterEach } from "matchstick-as/assembly/index";
import { handleTransfer } from "../src/mapping";
import { createTransferEvent } from "./utils";
describe("Transfer", () => {
  afterEach(() => { clearStore(); });
  test("creates a Transfer entity", () => {
    handleTransfer(createTransferEvent("0x0000000000000000000000000000000000000001", "0x0000000000000000000000000000000000000002", 100));
    assert.entityCount("Transfer", 1);
  });
});
