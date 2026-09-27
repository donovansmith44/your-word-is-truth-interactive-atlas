//! One value has exactly one byte spelling -- keys in byte order, no whitespace, shortest
//! escapes, numbers as `Display` spells them -- and whatever `encode` produced, `decode`
//! reads back or returns an error carrying the path to the offending spot, never a panic.

use std::collections::BTreeMap;

mod json;
pub mod ids;
pub mod node;
pub mod rows;

pub use json::{parse, serialize};
pub use rows::{encode_row_in_family, RowFamily};

/// The encoding's version. Bump ONLY for a breaking byte change; every
pub const CANON_VERSION: u32 = 1;

/// Domain separation for any hash taken over canonical bytes: prefix the
pub const DOMAIN_PREFIX: &[u8] = b"bible-atlas/canon/1\n";

/// The one path root, shared by the byte parser and by every field decoder, so a decode error
/// reads as one continuous location whichever side raised it and no error carries an empty path.
pub const ROOT: &str = "$";

/// The canonical JSON data model. Deliberately small: no integer/float
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
    pub fn float(f: f64) -> Result<Value, CanonError> {
        if f.is_finite() {
            Ok(Value::Float(f))
        } else {
            Err(CanonError::new(ROOT, "non-finite float has no canonical spelling"))
        }
    }

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

/// `path` is a dotted trail from the root down to the offending spot -- `$.payload.Place.lat`
/// -- so a bad artifact row names its own bad field. It is never empty.
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

/// Extend a path with one more segment. An empty side contributes
pub fn join(path: &str, seg: &str) -> String {
    if path.is_empty() {
        seg.to_string()
    } else if seg.is_empty() {
        path.to_string()
    } else {
        format!("{path}.{seg}")
    }
}

/// An object from `(key, value)` pairs -- the one spelling every
pub fn obj(pairs: Vec<(&str, Value)>) -> Value {
    Value::Obj(pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
}

/// A single-variant enum object: `{"Variant": payload}` (unit variants
pub fn variant(name: &str, payload: Value) -> Value {
    obj(vec![(name, payload)])
}

pub fn str_value(s: &str) -> Value {
    Value::Str(s.to_string())
}

pub fn opt_str(o: &Option<String>) -> Value {
    match o {
        Some(s) => Value::Str(s.clone()),
        None => Value::Null,
    }
}

pub fn opt_i32(o: &Option<i32>) -> Value {
    match o {
        Some(i) => Value::Int(i64::from(*i)),
        None => Value::Null,
    }
}

pub fn opt_u8(o: &Option<u8>) -> Value {
    match o {
        Some(i) => Value::Int(i64::from(*i)),
        None => Value::Null,
    }
}

pub fn vec_str(v: &[String]) -> Value {
    Value::Arr(v.iter().map(|s| Value::Str(s.clone())).collect())
}

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

pub fn expect_bool(v: &Value, path: &str) -> Result<bool, CanonError> {
    match v {
        Value::Bool(b) => Ok(*b),
        other => Err(CanonError::new(path, format!("expected bool, found {}", other.type_name()))),
    }
}

pub fn field_bool(m: &BTreeMap<String, Value>, path: &str, key: &str) -> Result<bool, CanonError> {
    let (v, p) = field(m, path, key)?;
    expect_bool(v, &p)
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

pub fn expect_u16(v: &Value, path: &str) -> Result<u16, CanonError> {
    let n = expect_i64(v, path)?;
    u16::try_from(n).map_err(|_| CanonError::new(path, format!("{n} is out of range for u16")))
}

pub fn expect_u32(v: &Value, path: &str) -> Result<u32, CanonError> {
    let n = expect_i64(v, path)?;
    u32::try_from(n).map_err(|_| CanonError::new(path, format!("{n} is out of range for u32")))
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
pub fn expect_f64(v: &Value, path: &str) -> Result<f64, CanonError> {
    match v {
        Value::Float(f) => Ok(*f),
        Value::Int(i) => Ok(*i as f64),
        other => Err(CanonError::new(path, format!("expected number, found {}", other.type_name()))),
    }
}

/// Re-roots an error onto `path`, appending whatever nested location it already carried rather
/// than dropping it. This is how a nested decoder, which starts at the root because it cannot
/// know where its caller sits, splices into the caller's trail.
pub fn at_path(path: &str, e: CanonError) -> CanonError {
    let inner = e.path.strip_prefix(ROOT).unwrap_or(&e.path);
    let inner = inner.strip_prefix('.').unwrap_or(inner);
    CanonError { path: join(path, inner), msg: e.msg }
}

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

pub fn field_u16(m: &BTreeMap<String, Value>, path: &str, key: &str) -> Result<u16, CanonError> {
    let (v, p) = field(m, path, key)?;
    expect_u16(v, &p)
}

pub fn field_u32(m: &BTreeMap<String, Value>, path: &str, key: &str) -> Result<u32, CanonError> {
    let (v, p) = field(m, path, key)?;
    expect_u32(v, &p)
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

/// Every closed object here names its members exactly, so a key nobody asked for means the
/// bytes carry meaning this decoder cannot see; ignoring it silently would hide that. Missing
/// keys are reported by the readers above, each under its own path.
pub fn expect_exact_keys(
    m: &BTreeMap<String, Value>,
    path: &str,
    keys: &[&str],
) -> Result<(), CanonError> {
    for k in m.keys() {
        if !keys.contains(&k.as_str()) {
            return Err(CanonError::new(
                join(path, k),
                format!("unexpected member `{k}`; this object allows only {keys:?}"),
            ));
        }
    }
    Ok(())
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
