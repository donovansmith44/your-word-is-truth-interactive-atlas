//! DB-2a: the canonical, versioned, DECODABLE encoding every node and
//! edge of the graph gets before it is written to the SQLite artifact.
//!
//! Two promises hold this module together:
//!
//! 1. **Canonical.** One value has exactly ONE byte spelling. Object keys
//!    ride in UTF-8 byte order (a `BTreeMap<String, _>` gives that for
//!    free), there is no whitespace anywhere, escapes are the shortest
//!    legal ones, and numbers are spelled the way Rust's `Display` spells
//!    them. So `serialize` is a function of the value alone -- safe to
//!    hash, safe to compare, safe to diff.
//! 2. **Decodable.** Whatever `encode` produced, `decode` reads back.
//!    Anything else is an `Err(CanonError)` carrying the PATH to the
//!    offending spot (`payload.Place.lat`) -- never a panic, at any depth.
//!
//! Nothing in the crate calls this yet; DB-2b wires it to the artifact.
//!
//! ## The number law
//!
//! * `Value::Int(i64)` is spelled by `i64`'s `Display` -- plain decimal,
//!   a leading `-` for negatives, no leading zeros.
//! * `Value::Float(f64)` is spelled by `f64`'s **`{}` Display**, and
//!   whatever `{}` yields IS the law. Rust's `Display` for `f64` emits the
//!   SHORTEST decimal string that round-trips back to the same bit
//!   pattern, never uses exponent notation, and drops a `.0` tail -- so
//!   `1.0_f64` prints `1`, `0.1 + 0.2` prints `0.30000000000000004`, and
//!   `-0.0` prints `-0`. This has one consequence worth naming: a
//!   whole-valued float and the equal integer share a spelling, so
//!   `parse` hands `1` back as `Value::Int(1)`. Every f64 field decoder in
//!   this module therefore accepts `Int` or `Float` (`expect_f64`), which
//!   is what keeps decode total on what encode produced.
//! * NaN and +/-inf have no canonical spelling and are refused at
//!   construction: `Value::float` returns `Err`, so `serialize` cannot
//!   fail.

use std::collections::BTreeMap;

mod json;
pub mod ids;
pub mod node;

pub use json::{parse, serialize};

/// The encoding's version. Bump ONLY for a breaking byte change; every
/// stored artifact records the version it was written under.
pub const CANON_VERSION: u32 = 1;

/// Domain separation for any hash taken over canonical bytes: prefix the
/// bytes with this so a digest of a node can never collide with a digest
/// of the same bytes meaning something else.
pub const DOMAIN_PREFIX: &[u8] = b"bible-atlas/canon/1\n";

/// The canonical JSON data model. Deliberately small: no integer/float
/// unification, no object ordering choice, no room for two spellings of
/// one value.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    Arr(Vec<Value>),
    Obj(BTreeMap<String, Value>),
}

impl Value {
    /// The ONLY way to build a `Value::Float`: non-finite doubles have no
    /// canonical spelling, so they are refused here rather than at
    /// serialization time. That is what lets `serialize` be infallible.
    pub fn float(f: f64) -> Result<Value, CanonError> {
        if f.is_finite() {
            Ok(Value::Float(f))
        } else {
            Err(CanonError::new("", "non-finite float has no canonical spelling"))
        }
    }

    /// The variant name, for error messages.
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Null => "null",
            Value::Bool(_) => "bool",
            Value::Int(_) => "int",
            Value::Float(_) => "float",
            Value::Str(_) => "string",
            Value::Arr(_) => "array",
            Value::Obj(_) => "object",
        }
    }
}

/// A decode failure, located. `path` is a dotted trail from the root of
/// the encoded value (`payload.Place.lat`, `payload.Polity.eras.0.rings`)
/// so a bad artifact row names its own bad field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanonError {
    pub path: String,
    pub msg: String,
}

impl CanonError {
    pub fn new(path: impl Into<String>, msg: impl Into<String>) -> Self {
        CanonError { path: path.into(), msg: msg.into() }
    }
}

impl std::fmt::Display for CanonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.path, self.msg)
    }
}

/// A type with a canonical JSON form. `encode`/`decode` are the byte
/// surface; `to_value`/`from_value` are the shape surface, so nested
/// types compose without going through bytes.
pub trait Canon: Sized {
    fn to_value(&self) -> Value;
    fn from_value(v: &Value) -> Result<Self, CanonError>;
    fn encode(&self) -> Vec<u8> {
        serialize(&self.to_value())
    }
    fn decode(bytes: &[u8]) -> Result<Self, CanonError> {
        Self::from_value(&parse(bytes)?)
    }
}

// ------------------------------------------------------------- build helpers

/// Extend a path with one more segment. The root path is `""`, so the
/// first segment carries no leading dot.
pub fn join(path: &str, seg: &str) -> String {
    if path.is_empty() {
        seg.to_string()
    } else {
        format!("{path}.{seg}")
    }
}

/// An object from `(key, value)` pairs -- the one spelling every
/// `to_value` impl in this module uses.
pub fn obj(pairs: Vec<(&str, Value)>) -> Value {
    Value::Obj(pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
}

/// A single-variant enum object: `{"Variant": payload}` (unit variants
/// pass `Value::Null`).
pub fn variant(name: &str, payload: Value) -> Value {
    obj(vec![(name, payload)])
}

pub fn str_value(s: &str) -> Value {
    Value::Str(s.to_string())
}

/// `Option<String>` -> string or `null`.
pub fn opt_str(o: &Option<String>) -> Value {
    match o {
        Some(s) => Value::Str(s.clone()),
        None => Value::Null,
    }
}

/// `Option<i32>` -> int or `null`.
pub fn opt_i32(o: &Option<i32>) -> Value {
    match o {
        Some(i) => Value::Int(i64::from(*i)),
        None => Value::Null,
    }
}

/// `Option<u8>` -> int or `null`.
pub fn opt_u8(o: &Option<u8>) -> Value {
    match o {
        Some(i) => Value::Int(i64::from(*i)),
        None => Value::Null,
    }
}

/// `&[String]` -> array of strings.
pub fn vec_str(v: &[String]) -> Value {
    Value::Arr(v.iter().map(|s| Value::Str(s.clone())).collect())
}

// -------------------------------------------------------------- read helpers
//
// Every accessor takes the path of the value it is reading and returns it
// inside any error, so a caller only ever has to `join` one more segment.

pub fn expect_obj<'a>(
    v: &'a Value,
    path: &str,
) -> Result<&'a BTreeMap<String, Value>, CanonError> {
    match v {
        Value::Obj(m) => Ok(m),
        other => Err(CanonError::new(path, format!("expected object, found {}", other.type_name()))),
    }
}

pub fn expect_arr<'a>(v: &'a Value, path: &str) -> Result<&'a Vec<Value>, CanonError> {
    match v {
        Value::Arr(a) => Ok(a),
        other => Err(CanonError::new(path, format!("expected array, found {}", other.type_name()))),
    }
}

/// Read a required member. The returned value's path is `path.key`, which
/// is also the path a missing key reports under -- so `payload` missing
/// from the root reports as `payload`, not as the root.
pub fn get<'a>(
    m: &'a BTreeMap<String, Value>,
    key: &str,
    path: &str,
) -> Result<&'a Value, CanonError> {
    m.get(key).ok_or_else(|| CanonError::new(join(path, key), "missing key"))
}

pub fn expect_str(v: &Value, path: &str) -> Result<String, CanonError> {
    match v {
        Value::Str(s) => Ok(s.clone()),
        other => Err(CanonError::new(path, format!("expected string, found {}", other.type_name()))),
    }
}

pub fn expect_opt_str(v: &Value, path: &str) -> Result<Option<String>, CanonError> {
    match v {
        Value::Null => Ok(None),
        Value::Str(s) => Ok(Some(s.clone())),
        other => Err(CanonError::new(
            path,
            format!("expected string or null, found {}", other.type_name()),
        )),
    }
}

pub fn expect_vec_str(v: &Value, path: &str) -> Result<Vec<String>, CanonError> {
    let arr = expect_arr(v, path)?;
    arr.iter()
        .enumerate()
        .map(|(i, item)| expect_str(item, &join(path, &i.to_string())))
        .collect()
}

pub fn expect_i64(v: &Value, path: &str) -> Result<i64, CanonError> {
    match v {
        Value::Int(i) => Ok(*i),
        other => Err(CanonError::new(path, format!("expected int, found {}", other.type_name()))),
    }
}

pub fn expect_i32(v: &Value, path: &str) -> Result<i32, CanonError> {
    let n = expect_i64(v, path)?;
    i32::try_from(n).map_err(|_| CanonError::new(path, format!("{n} is out of range for i32")))
}

pub fn expect_u8(v: &Value, path: &str) -> Result<u8, CanonError> {
    let n = expect_i64(v, path)?;
    u8::try_from(n).map_err(|_| CanonError::new(path, format!("{n} is out of range for u8")))
}

pub fn expect_opt_i32(v: &Value, path: &str) -> Result<Option<i32>, CanonError> {
    match v {
        Value::Null => Ok(None),
        _ => Ok(Some(expect_i32(v, path)?)),
    }
}

pub fn expect_opt_u8(v: &Value, path: &str) -> Result<Option<u8>, CanonError> {
    match v {
        Value::Null => Ok(None),
        _ => Ok(Some(expect_u8(v, path)?)),
    }
}

/// An `f64` field. `Int` is accepted alongside `Float` on purpose: a
/// whole-valued f64 serializes as a bare integer (see the number law
/// above), so refusing `Int` here would make decode partial on bytes
/// encode itself wrote.
pub fn expect_f64(v: &Value, path: &str) -> Result<f64, CanonError> {
    match v {
        Value::Float(f) => Ok(*f),
        Value::Int(i) => Ok(*i as f64),
        other => Err(CanonError::new(path, format!("expected number, found {}", other.type_name()))),
    }
}

/// Re-locate an error raised by a helper that did not know its own path
/// (the `ids` string parsers, `Year::new`, and friends).
pub fn at_path(path: &str, e: CanonError) -> CanonError {
    CanonError { path: path.to_string(), msg: e.msg }
}

// ------------------------------------------------------------ field accessors
//
// The layer every `from_value` impl actually uses: `path` is the OBJECT's
// path, `key` the member, and the error already carries `path.key`.

pub fn field<'a>(
    m: &'a BTreeMap<String, Value>,
    path: &str,
    key: &str,
) -> Result<(&'a Value, String), CanonError> {
    Ok((get(m, key, path)?, join(path, key)))
}

pub fn field_str(
    m: &BTreeMap<String, Value>,
    path: &str,
    key: &str,
) -> Result<String, CanonError> {
    let (v, p) = field(m, path, key)?;
    expect_str(v, &p)
}

pub fn field_opt_str(
    m: &BTreeMap<String, Value>,
    path: &str,
    key: &str,
) -> Result<Option<String>, CanonError> {
    let (v, p) = field(m, path, key)?;
    expect_opt_str(v, &p)
}

pub fn field_vec_str(
    m: &BTreeMap<String, Value>,
    path: &str,
    key: &str,
) -> Result<Vec<String>, CanonError> {
    let (v, p) = field(m, path, key)?;
    expect_vec_str(v, &p)
}

pub fn field_i32(m: &BTreeMap<String, Value>, path: &str, key: &str) -> Result<i32, CanonError> {
    let (v, p) = field(m, path, key)?;
    expect_i32(v, &p)
}

pub fn field_opt_i32(
    m: &BTreeMap<String, Value>,
    path: &str,
    key: &str,
) -> Result<Option<i32>, CanonError> {
    let (v, p) = field(m, path, key)?;
    expect_opt_i32(v, &p)
}

pub fn field_u8(m: &BTreeMap<String, Value>, path: &str, key: &str) -> Result<u8, CanonError> {
    let (v, p) = field(m, path, key)?;
    expect_u8(v, &p)
}

pub fn field_opt_u8(
    m: &BTreeMap<String, Value>,
    path: &str,
    key: &str,
) -> Result<Option<u8>, CanonError> {
    let (v, p) = field(m, path, key)?;
    expect_opt_u8(v, &p)
}

pub fn field_f64(m: &BTreeMap<String, Value>, path: &str, key: &str) -> Result<f64, CanonError> {
    let (v, p) = field(m, path, key)?;
    expect_f64(v, &p)
}

pub fn field_arr<'a>(
    m: &'a BTreeMap<String, Value>,
    path: &str,
    key: &str,
) -> Result<(&'a Vec<Value>, String), CanonError> {
    let (v, p) = field(m, path, key)?;
    Ok((expect_arr(v, &p)?, p))
}

pub fn field_obj<'a>(
    m: &'a BTreeMap<String, Value>,
    path: &str,
    key: &str,
) -> Result<(&'a BTreeMap<String, Value>, String), CanonError> {
    let (v, p) = field(m, path, key)?;
    Ok((expect_obj(v, &p)?, p))
}

/// Unwrap a `{"Variant": payload}` object into its name and payload.
pub fn expect_variant<'a>(v: &'a Value, path: &str) -> Result<(&'a str, &'a Value), CanonError> {
    let m = expect_obj(v, path)?;
    let mut it = m.iter();
    match (it.next(), it.next()) {
        (Some((name, payload)), None) => Ok((name.as_str(), payload)),
        (None, _) => Err(CanonError::new(path, "expected one variant key, found none")),
        (Some(_), Some(_)) => Err(CanonError::new(
            path,
            format!("expected exactly one variant key, found {}", m.len()),
        )),
    }
}
