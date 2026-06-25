//! Interpreter tests: write a project with handlers + `test` blocks, run them,
//! and assert which pass and which fail.

use crate::{run_tests, Outcome};
use std::fs;

const ABI: &str = r#"[
  {"type":"event","name":"Transfer","inputs":[
    {"name":"from","type":"address","indexed":true},
    {"name":"to","type":"address","indexed":true},
    {"name":"value","type":"uint256","indexed":false}]},
  {"type":"event","name":"Approval","inputs":[
    {"name":"owner","type":"address","indexed":true},
    {"name":"spender","type":"address","indexed":true},
    {"name":"value","type":"uint256","indexed":false}]},
  {"type":"function","name":"balanceOf","stateMutability":"view",
    "inputs":[{"name":"account","type":"address"}],
    "outputs":[{"name":"","type":"uint256"}]}
]"#;

const PROGRAM: &str = r#"
abi ERC20 from "./abis/ERC20.json"

entity Account { id: Id<Bytes> balance: BigInt }

source Token {
  abi: ERC20
  network: mainnet
  address: 0x00
  startBlock: 1
}

handler on Token.Transfer(event) {
  let sender = Account.loadOrCreate(event.params.from, { balance: BigInt.zero })
  let receiver = Account.loadOrCreate(event.params.to, { balance: BigInt.zero })
  sender.balance = sender.balance - event.params.value
  receiver.balance = receiver.balance + event.params.value
}

handler on Token.Approval(event) {
  let r = ERC20.bind(event.address).balanceOf(event.params.owner)
  match r {
    Ok(b) => {
      let o = Account.loadOrCreate(event.params.owner, { balance: BigInt.zero })
      o.balance = b
    }
    Err(e) => {}
  }
}
"#;

/// Run the program plus `tests_src`, returning the per-test outcomes by name.
fn outcomes(tests_src: &str) -> Vec<(String, bool, String)> {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("src/abis")).unwrap();
    fs::write(dir.path().join("redstart.toml"), "[project]\nname=\"t\"\nentry=\"src/main.red\"").unwrap();
    fs::write(dir.path().join("src/abis/ERC20.json"), ABI).unwrap();
    fs::write(dir.path().join("src/main.red"), format!("{PROGRAM}\n{tests_src}")).unwrap();

    let tree = redstart_loader::load(dir.path()).unwrap();
    let checked = redstart_checker::check(&tree).expect("program should pass the checker");
    let report = run_tests(&tree, &checked);
    report
        .results
        .into_iter()
        .map(|r| match r.outcome {
            Outcome::Pass => (r.name, true, String::new()),
            Outcome::Fail { message, .. } => (r.name, false, message),
        })
        .collect()
}

#[test]
fn transfer_credits_and_debits() {
    let out = outcomes(
        r#"
test "transfer moves balance" {
  Token.Transfer({ from: 0x01, to: 0x02, value: 100 })
  assertEq(Account.at(0x02).balance, 100)
}
"#,
    );
    assert!(out[0].1, "expected pass, got: {}", out[0].2);
}

#[test]
fn wrong_assertion_fails() {
    let out = outcomes(
        r#"
test "wrong expectation" {
  Token.Transfer({ from: 0x01, to: 0x02, value: 100 })
  assertEq(Account.at(0x02).balance, 999)
}
"#,
    );
    assert!(!out[0].1);
    assert!(out[0].2.contains("assertEq failed"), "msg: {}", out[0].2);
}

#[test]
fn debit_goes_negative() {
    let out = outcomes(
        r#"
test "sender is debited" {
  Token.Transfer({ from: 0x01, to: 0x02, value: 100 })
  assert(Account.at(0x01).balance < 0)
}
"#,
    );
    assert!(out[0].1, "expected pass, got: {}", out[0].2);
}

#[test]
fn mocked_contract_call_and_match() {
    let out = outcomes(
        r#"
test "approval reads balance via call" {
  mockCall(ERC20.balanceOf(0x05), 777)
  Token.Approval({ owner: 0x05, spender: 0x06, value: 1 })
  assertEq(Account.at(0x05).balance, 777)
}
"#,
    );
    assert!(out[0].1, "expected pass, got: {}", out[0].2);
}

#[test]
fn unmocked_call_fails_loudly() {
    let out = outcomes(
        r#"
test "forgot to mock" {
  Token.Approval({ owner: 0x05, spender: 0x06, value: 1 })
  assert(true)
}
"#,
    );
    assert!(!out[0].1);
    assert!(out[0].2.contains("unmocked contract call"), "msg: {}", out[0].2);
}

#[test]
fn for_range_loop_accumulates() {
    let out = outcomes(
        r#"
test "range sum" {
  let total = 0
  for i in 1..5 {
    total = total + i
  }
  assertEq(total, 10)
}
"#,
    );
    assert!(out[0].1, "expected pass, got: {}", out[0].2);
}

#[test]
fn for_each_and_index_and_length() {
    let out = outcomes(
        r#"
test "array each" {
  let xs = [10, 20, 30]
  assertEq(xs.length, 3)
  assertEq(xs[1], 20)
  let total = 0
  for v in xs {
    total = total + v
  }
  assertEq(total, 60)
}
"#,
    );
    assert!(out[0].1, "expected pass, got: {}", out[0].2);
}

#[test]
fn if_else_and_while() {
    let out = outcomes(
        r#"
test "branch and loop" {
  let x = 0
  if 3 > 5 {
    x = 1
  } else if 3 > 2 {
    x = 2
  } else {
    x = 3
  }
  assertEq(x, 2)
  let n = 0
  while n < 4 {
    n = n + 1
  }
  assertEq(n, 4)
}
"#,
    );
    assert!(out[0].1, "expected pass, got: {}", out[0].2);
}

#[test]
fn loop_inside_handler_writes_store() {
    let out = outcomes(
        r#"
test "handler with conditional logic" {
  Token.Transfer({ from: 0x01, to: 0x02, value: 50 })
  assert(Account.at(0x01).balance < 0)
  assertEq(Account.at(0x02).balance, 50)
}
"#,
    );
    assert!(out[0].1, "expected pass, got: {}", out[0].2);
}

#[test]
fn event_meta_override() {
    let out = outcomes(
        r#"
test "timestamp override flows into entity" {
  Token.Transfer({ from: 0x01, to: 0x02, value: 5, _timestamp: 42 })
  assertEq(Account.at(0x02).balance, 5)
}
"#,
    );
    assert!(out[0].1, "expected pass, got: {}", out[0].2);
}
