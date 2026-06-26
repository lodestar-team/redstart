# Testing

Redstart has a built-in test runner that executes natively — **no WASM, no
Docker, no Matchstick**. Tests live alongside your code in any `.red` module and
run with `redstart test`.

```redstart
test "a transfer debits the sender and credits the receiver" {
  Token.Transfer({ from: 0x01, to: 0x02, value: 100 })
  assertEq(Account.at(0x02).balance, 100)
  assert(Account.at(0x01).balance < 0)
}
```

A test fires events at your handlers against a mock store, then asserts on the
resulting entity state.

## Mocking contract calls

When a handler reads on-chain state, mock the call so the test stays
deterministic:

```redstart
test "approval writes the on-chain balance read via a contract call" {
  mockCall(ERC20.balanceOf(0x05), 4200)
  Token.Approval({ owner: 0x05, spender: 0x06, value: 1 })
  assertEq(Account.at(0x05).balance, 4200)
}
```

## Running

```sh
redstart test              # run every test in the project
redstart test examples/erc20
```

Because the runner is native, the inner loop is near-instant — pair it with
`redstart dev` to re-run check → build → test on every save.
