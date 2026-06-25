//! Checker integration tests: each writes a tiny project and asserts whether
//! `check` accepts it or rejects it with the expected diagnostic.

use crate::check;
use std::fs;

const ABI: &str = r#"[
  {"type":"event","name":"Transfer","inputs":[
    {"name":"from","type":"address","indexed":true},
    {"name":"to","type":"address","indexed":true},
    {"name":"value","type":"uint256","indexed":false}]},
  {"type":"function","name":"balanceOf","stateMutability":"view",
    "inputs":[{"name":"account","type":"address"}],
    "outputs":[{"name":"","type":"uint256"}]}
]"#;

/// Build a one-file project from `src` and run the checker.
fn run(src: &str) -> Result<(), Vec<String>> {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("src/abis")).unwrap();
    fs::write(
        dir.path().join("redstart.toml"),
        "[project]\nname = \"t\"\nentry = \"src/main.red\"",
    )
    .unwrap();
    fs::write(dir.path().join("src/abis/ERC20.json"), ABI).unwrap();
    fs::write(dir.path().join("src/main.red"), src).unwrap();

    let tree = redstart_loader::load(dir.path()).unwrap();
    check(&tree).map(|_| ())
}

const PREAMBLE: &str = r#"
abi ERC20 from "./abis/ERC20.json"
entity Account { id: Id<Bytes> balance: BigInt }
source Token {
  abi: ERC20
  network: mainnet
  address: 0x1234567890abcdef1234567890abcdef12345678
  startBlock: 1
}
"#;

fn with_handler(body: &str) -> String {
    format!("{PREAMBLE}\nhandler on Token.Transfer(event) {{\n{body}\n}}\n")
}

fn assert_err_contains(result: Result<(), Vec<String>>, needle: &str) {
    let errs = result.expect_err("expected a check error");
    let joined = errs.join("\n");
    assert!(joined.contains(needle), "expected `{needle}` in:\n{joined}");
}

#[test]
fn valid_program_passes() {
    let ok = with_handler(
        "let acct = Account.loadOrCreate(event.params.to, { balance: BigInt.zero })\n\
         acct.balance = acct.balance + event.params.value",
    );
    assert!(run(&ok).is_ok());
}

#[test]
fn missing_required_field_is_rejected() {
    let src = with_handler("let acct = Account.loadOrCreate(event.params.to, {})");
    assert_err_contains(run(&src), "missing required field(s): balance");
}

#[test]
fn unknown_record_field_is_rejected() {
    let src = with_handler(
        "let acct = Account.loadOrCreate(event.params.to, { balance: BigInt.zero, bogus: BigInt.zero })",
    );
    assert_err_contains(run(&src), "has no field `bogus`");
}

#[test]
fn unknown_source_is_rejected() {
    let src = format!("{PREAMBLE}\nhandler on Nope.Transfer(event) {{ }}\n");
    assert_err_contains(run(&src), "unknown source `Nope`");
}

#[test]
fn unknown_event_is_rejected() {
    let src = format!("{PREAMBLE}\nhandler on Token.Nope(event) {{ }}\n");
    assert_err_contains(run(&src), "event `Nope` not found");
}

#[test]
fn assign_to_derived_field_is_rejected() {
    let src = format!(
        "{}\nentity Pool {{ id: Id<Bytes> accs: [Account] derived from owner }}\n\
         handler on Token.Transfer(event) {{\n\
           let p = Pool.loadOrCreate(event.address, {{}})\n\
           p.accs = event.params.value\n\
         }}\n",
        PREAMBLE
    );
    // Account needs an `owner` field for the derive to be valid; add it.
    let src = src.replace(
        "entity Account { id: Id<Bytes> balance: BigInt }",
        "entity Account { id: Id<Bytes> balance: BigInt owner: Account }",
    );
    assert_err_contains(run(&src), "cannot assign to derived field `accs`");
}

#[test]
fn reading_call_value_without_match_is_rejected() {
    let src = with_handler(
        "let r = ERC20.bind(event.address).balanceOf(event.params.to)\n\
         let b = r.value",
    );
    assert_err_contains(run(&src), "cannot read `.value` of a contract call");
}

#[test]
fn arithmetic_on_option_is_rejected() {
    let src = format!(
        "{}\nentity Acc {{ id: Id<Bytes> bal: Option<BigInt> }}\n\
         handler on Token.Transfer(event) {{\n\
           let a = Acc.load(event.params.to)\n\
           let x = a.bal + event.params.value\n\
         }}\n",
        PREAMBLE
    );
    assert_err_contains(run(&src), "cannot do arithmetic on an `Option`");
}

#[test]
fn unknown_type_is_rejected() {
    let src = format!("{PREAMBLE}\nentity Bad {{ id: Id<Bytes> thing: Wibble }}\n");
    assert_err_contains(run(&src), "unknown type `Wibble`");
}

#[test]
fn duplicate_entity_is_rejected() {
    let src = format!("{PREAMBLE}\nentity Account {{ id: Id<Bytes> balance: BigInt }}\n");
    assert_err_contains(run(&src), "duplicate entity `Account`");
}

#[test]
fn missing_source_setting_is_rejected() {
    let src = "abi ERC20 from \"./abis/ERC20.json\"\n\
               entity Account { id: Id<Bytes> }\n\
               source Token { abi: ERC20 network: mainnet }\n";
    assert_err_contains(run(src), "missing `address`");
}

#[test]
fn unknown_entity_field_is_rejected() {
    let src = with_handler(
        "let acct = Account.loadOrCreate(event.params.to, { balance: BigInt.zero })\n\
         acct.blance = event.params.value",
    );
    assert_err_contains(run(&src), "`Account` has no field `blance`");
}

#[test]
fn non_exhaustive_match_is_rejected() {
    let src = with_handler(
        "let r = ERC20.bind(event.address).balanceOf(event.params.to)\n\
         match r {\n  Ok(b) => {}\n}",
    );
    assert_err_contains(run(&src), "non-exhaustive `match`: missing Err");
}

#[test]
fn wildcard_match_is_exhaustive() {
    let ok = with_handler(
        "let r = ERC20.bind(event.address).balanceOf(event.params.to)\n\
         match r {\n  Ok(b) => {}\n  _ => {}\n}",
    );
    assert!(run(&ok).is_ok());
}

#[test]
fn unknown_contract_function_is_rejected() {
    let src = with_handler("let r = ERC20.bind(event.address).totalSupply(event.params.to)");
    assert_err_contains(run(&src), "contract `ERC20` has no function `totalSupply`");
}

#[test]
fn derived_backref_must_exist() {
    let src = format!(
        "{PREAMBLE}\nentity Pool {{ id: Id<Bytes> accs: [Account] derived from nope }}\n"
    );
    assert_err_contains(run(&src), "has no field `nope`");
}

#[test]
fn enum_typed_field_is_accepted() {
    let src = format!(
        "{PREAMBLE}\nenum Kind {{ Mint, Burn }}\nentity Tx {{ id: Id<Bytes> kind: Kind at: Timestamp }}\n"
    );
    assert!(run(&src).is_ok(), "enum/Timestamp field should type-check");
}

#[test]
fn unknown_field_type_still_rejected() {
    let src = format!("{PREAMBLE}\nentity Tx {{ id: Id<Bytes> kind: Nonsense }}\n");
    assert_err_contains(run(&src), "unknown type `Nonsense`");
}

#[test]
fn entity_implementing_interface_passes() {
    let src = format!(
        "{PREAMBLE}\ninterface Named {{ id: Id<Bytes> name: String }}\n\
         entity Thing implements Named {{ id: Id<Bytes> name: String extra: BigInt }}\n"
    );
    assert!(run(&src).is_ok(), "valid implements should pass");
}

#[test]
fn missing_interface_field_is_rejected() {
    let src = format!(
        "{PREAMBLE}\ninterface Named {{ id: Id<Bytes> name: String }}\n\
         entity Thing implements Named {{ id: Id<Bytes> }}\n"
    );
    assert_err_contains(run(&src), "missing field `name`");
}

#[test]
fn implementing_unknown_interface_is_rejected() {
    let src = format!("{PREAMBLE}\nentity Thing implements Ghost {{ id: Id<Bytes> }}\n");
    assert_err_contains(run(&src), "unknown interface `Ghost`");
}
