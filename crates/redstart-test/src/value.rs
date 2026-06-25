//! Runtime values for the test interpreter.

use bigdecimal::BigDecimal;
use num_bigint::BigInt;
use std::collections::BTreeMap;

/// A synthesised event passed to a handler under test.
#[derive(Clone, Debug)]
pub struct EventVal {
    /// Event parameters by name (`event.params.X`).
    pub params: BTreeMap<String, Value>,
    /// The emitting contract address (`event.address`).
    pub address: Vec<u8>,
    /// `event.block.number`.
    pub block_number: BigInt,
    /// `event.block.timestamp`.
    pub block_timestamp: BigInt,
    /// `event.transaction.hash`.
    pub tx_hash: Vec<u8>,
    /// `event.logIndex`, used to build the synthetic `event.id`.
    pub log_index: i64,
}

/// A runtime value.
#[derive(Clone, Debug)]
pub enum Value {
    /// A bare integer literal (coerces to `BigInt` in numeric context).
    Int(i64),
    /// A `BigInt`.
    Big(BigInt),
    /// A `BigDecimal`.
    Dec(BigDecimal),
    /// `Bytes` / `Address` / `Id` — raw bytes.
    Bytes(Vec<u8>),
    /// A string.
    Str(String),
    /// A boolean.
    Bool(bool),
    /// `null` / `None` / a missing `load`.
    Null,
    /// An array literal value.
    Array(Vec<Value>),
    /// A mutable working-entity handle (index into the current frame).
    Handle(usize),
    /// A read-only snapshot of a stored entity (`Entity.at(id)`).
    Stored(String, BTreeMap<String, Value>),
    /// A bound contract instance (`Abi.bind(addr)`).
    Contract(String),
    /// A contract call result.
    Result {
        /// Whether the call reverted.
        reverted: bool,
        /// The returned value (meaningful only when not reverted).
        value: Box<Value>,
    },
    /// The handler event object.
    Event(Box<EventVal>),
    /// `event.params`.
    EventParams(Box<EventVal>),
    /// `event.block`.
    EventBlock(Box<EventVal>),
    /// `event.transaction`.
    EventTx(Box<EventVal>),
    /// A statement with no value.
    Unit,
}

impl Value {
    /// Interpret this value as raw bytes (for entity ids and addresses).
    pub fn as_bytes(&self) -> Option<Vec<u8>> {
        match self {
            Value::Bytes(b) => Some(b.clone()),
            _ => None,
        }
    }

    /// Interpret this value as a boolean.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Coerce a numeric value to `BigInt` (`Int`/`Big`).
    pub fn to_bigint(&self) -> Option<BigInt> {
        match self {
            Value::Int(i) => Some(BigInt::from(*i)),
            Value::Big(b) => Some(b.clone()),
            _ => None,
        }
    }

    /// Coerce a numeric value to `BigDecimal`.
    pub fn to_bigdecimal(&self) -> Option<BigDecimal> {
        match self {
            Value::Int(i) => Some(BigDecimal::from(*i)),
            Value::Big(b) => Some(BigDecimal::from(b.clone())),
            Value::Dec(d) => Some(d.clone()),
            _ => None,
        }
    }

    /// A canonical, stable string form (for mock keys and assert messages).
    pub fn canonical(&self) -> String {
        match self {
            Value::Int(i) => i.to_string(),
            Value::Big(b) => b.to_string(),
            Value::Dec(d) => d.to_string(),
            Value::Bytes(b) => format!("0x{}", hex(b)),
            Value::Str(s) => s.clone(),
            Value::Bool(b) => b.to_string(),
            Value::Null => "null".to_string(),
            Value::Array(items) => {
                let parts = items.iter().map(Value::canonical).collect::<Vec<_>>();
                format!("[{}]", parts.join(","))
            }
            Value::Stored(name, _) => format!("<{name}>"),
            Value::Contract(a) => format!("<contract {a}>"),
            Value::Result { reverted, value } => {
                if *reverted {
                    "Err(reverted)".to_string()
                } else {
                    format!("Ok({})", value.canonical())
                }
            }
            Value::Handle(_) => "<entity>".to_string(),
            Value::Event(_) | Value::EventParams(_) | Value::EventBlock(_) | Value::EventTx(_) => {
                "<event>".to_string()
            }
            Value::Unit => "()".to_string(),
        }
    }
}

/// Structural/numeric equality with cross-numeric coercion.
pub fn value_eq(a: &Value, b: &Value) -> bool {
    use Value::{Bool, Bytes, Null, Str};
    // Numeric: compare as BigDecimal so Int/Big/Dec interoperate.
    if let (Some(x), Some(y)) = (a.to_bigdecimal(), b.to_bigdecimal()) {
        return x == y;
    }
    match (a, b) {
        (Bytes(x), Bytes(y)) => x == y,
        (Str(x), Str(y)) => x == y,
        (Bool(x), Bool(y)) => x == y,
        (Null, Null) => true,
        (Value::Array(x), Value::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(a, b)| value_eq(a, b))
        }
        _ => false,
    }
}

fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}
