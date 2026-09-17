//! The canonical JSON codec: an infallible serializer and a strict,
//! non-recursive-blowup parser that accepts ONLY canonical bytes.
//!
//! "Strict" is the whole point. A lenient parser would let two different
//! byte strings mean one value, and the artifact's hashes would stop
//! meaning anything. So every non-canonical spelling -- whitespace, an
//! unsorted or repeated key, a redundant escape, `1.50`, `1e5`, `01` --
//! is an error, not a kindness.

use std::collections::BTreeMap;

use super::{CanonError, Value};

/// How deep a value may nest before the parser gives up. Node and edge
/// payloads nest fewer than ten levels; the limit exists so hostile or
/// corrupt bytes fail as an `Err`, not as a blown stack.
const MAX_DEPTH: u32 = 64;

/// The path the parser reports the top-level value under.
const ROOT: &str = "$";

// ------------------------------------------------------------- serialization

/// Canonical bytes for a value. Infallible: the only unrepresentable
/// doubles are refused by `Value::float` at construction.
pub fn serialize(v: &Value) -> Vec<u8> {
    let mut out = Vec::new();
    write_value(v, &mut out);
    out
}

fn write_value(v: &Value, out: &mut Vec<u8>) {
    match v {
        Value::Null => out.extend_from_slice(b"null"),
        Value::Bool(true) => out.extend_from_slice(b"true"),
        Value::Bool(false) => out.extend_from_slice(b"false"),
        Value::Int(i) => out.extend_from_slice(i.to_string().as_bytes()),
        // The float law: Rust's `{}` Display, whatever it yields. See the
        // module docs on `canon` for what that means and why it is safe.
        Value::Float(f) => out.extend_from_slice(format!("{f}").as_bytes()),
        Value::Str(s) => write_string(s, out),
        Value::Arr(items) => {
            out.push(b'[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(b',');
                }
                write_value(item, out);
            }
            out.push(b']');
        }
        Value::Obj(members) => {
            out.push(b'{');
            // BTreeMap iteration is `str` order, which is UTF-8 byte
            // order -- the key ordering law, for free.
            for (i, (k, val)) in members.iter().enumerate() {
                if i > 0 {
                    out.push(b',');
                }
                write_string(k, out);
                out.push(b':');
                write_value(val, out);
            }
            out.push(b'}');
        }
    }
}

/// Minimal escaping: the two mandatory escapes, the five two-character
/// control escapes (shorter than `\u00xx`, so they are the canonical
/// form), `\u00xx` in lowercase hex for the remaining C0 controls, and
/// everything else -- `/` included, non-ASCII included -- raw UTF-8.
fn write_string(s: &str, out: &mut Vec<u8>) {
    out.push(b'"');
    for ch in s.chars() {
        match ch {
            '"' => out.extend_from_slice(b"\\\""),
            '\\' => out.extend_from_slice(b"\\\\"),
            '\u{8}' => out.extend_from_slice(b"\\b"),
            '\t' => out.extend_from_slice(b"\\t"),
            '\n' => out.extend_from_slice(b"\\n"),
            '\u{c}' => out.extend_from_slice(b"\\f"),
            '\r' => out.extend_from_slice(b"\\r"),
            c if (c as u32) < 0x20 => {
                out.extend_from_slice(format!("\\u{:04x}", c as u32).as_bytes());
            }
            c => {
                let mut buf = [0u8; 4];
                out.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
            }
        }
    }
    out.push(b'"');
}

/// The C0 controls that have a two-character escape, so a `\u00xx`
/// spelling of them is non-minimal.
fn has_short_escape(code: u32) -> bool {
    matches!(code, 0x08 | 0x09 | 0x0a | 0x0c | 0x0d)
}

// ------------------------------------------------------------------- parsing

/// Parse canonical bytes. Any deviation from the canonical spelling is an
/// `Err` carrying the path of the offending value.
pub fn parse(bytes: &[u8]) -> Result<Value, CanonError> {
    let mut p = Parser { b: bytes, i: 0, depth: 0 };
    let v = p.value(ROOT)?;
    if p.i != bytes.len() {
        return Err(CanonError::new(ROOT, format!("trailing content at byte {}", p.i)));
    }
    Ok(v)
}

struct Parser<'a> {
    b: &'a [u8],
    i: usize,
    depth: u32,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<u8> {
        self.b.get(self.i).copied()
    }

    fn err<T>(&self, path: &str, msg: impl Into<String>) -> Result<T, CanonError> {
        let msg = msg.into();
        Err(CanonError::new(path, format!("{msg} (at byte {})", self.i)))
    }

    fn literal(&mut self, path: &str, lit: &[u8], v: Value) -> Result<Value, CanonError> {
        if self.b[self.i..].starts_with(lit) {
            self.i += lit.len();
            Ok(v)
        } else {
            self.err(path, format!("expected `{}`", String::from_utf8_lossy(lit)))
        }
    }

    fn value(&mut self, path: &str) -> Result<Value, CanonError> {
        match self.peek() {
            None => self.err(path, "unexpected end of input"),
            Some(b'n') => self.literal(path, b"null", Value::Null),
            Some(b't') => self.literal(path, b"true", Value::Bool(true)),
            Some(b'f') => self.literal(path, b"false", Value::Bool(false)),
            Some(b'"') => Ok(Value::Str(self.string(path)?)),
            Some(b'[') => self.array(path),
            Some(b'{') => self.object(path),
            Some(b'-') | Some(b'0'..=b'9') => self.number(path),
            Some(c) => self.err(path, format!("unexpected byte 0x{c:02x}")),
        }
    }

    fn enter(&mut self, path: &str) -> Result<(), CanonError> {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return self.err(path, format!("nesting deeper than {MAX_DEPTH}"));
        }
        Ok(())
    }

    fn array(&mut self, path: &str) -> Result<Value, CanonError> {
        self.enter(path)?;
        self.i += 1; // '['
        let mut items = Vec::new();
        if self.peek() == Some(b']') {
            self.i += 1;
            self.depth -= 1;
            return Ok(Value::Arr(items));
        }
        loop {
            let item_path = super::join(path, &items.len().to_string());
            items.push(self.value(&item_path)?);
            match self.peek() {
                Some(b',') => self.i += 1,
                Some(b']') => {
                    self.i += 1;
                    break;
                }
                _ => return self.err(path, "expected `,` or `]`"),
            }
        }
        self.depth -= 1;
        Ok(Value::Arr(items))
    }

    fn object(&mut self, path: &str) -> Result<Value, CanonError> {
        self.enter(path)?;
        self.i += 1; // '{'
        let mut members: BTreeMap<String, Value> = BTreeMap::new();
        if self.peek() == Some(b'}') {
            self.i += 1;
            self.depth -= 1;
            return Ok(Value::Obj(members));
        }
        let mut prev: Option<String> = None;
        loop {
            if self.peek() != Some(b'"') {
                return self.err(path, "expected a quoted key");
            }
            let key = self.string(path)?;
            // Strictly increasing catches BOTH laws at once: an unsorted
            // key and a repeated key are the same violation.
            if let Some(p) = &prev {
                if key <= *p {
                    return self.err(
                        &super::join(path, &key),
                        format!(
                            "keys must strictly increase in UTF-8 byte order; `{key}` follows `{p}`"
                        ),
                    );
                }
            }
            if self.peek() != Some(b':') {
                return self.err(&super::join(path, &key), "expected `:`");
            }
            self.i += 1;
            let v = self.value(&super::join(path, &key))?;
            members.insert(key.clone(), v);
            prev = Some(key);
            match self.peek() {
                Some(b',') => self.i += 1,
                Some(b'}') => {
                    self.i += 1;
                    break;
                }
                _ => return self.err(path, "expected `,` or `}`"),
            }
        }
        self.depth -= 1;
        Ok(Value::Obj(members))
    }

    fn string(&mut self, path: &str) -> Result<String, CanonError> {
        self.i += 1; // opening quote
        let mut out = String::new();
        let mut run = self.i; // start of the current unescaped run
        loop {
            let c = match self.peek() {
                Some(c) => c,
                None => return self.err(path, "unterminated string"),
            };
            match c {
                b'"' => {
                    self.push_run(path, &mut out, run)?;
                    self.i += 1;
                    return Ok(out);
                }
                b'\\' => {
                    self.push_run(path, &mut out, run)?;
                    self.i += 1;
                    let decoded = self.escape(path)?;
                    out.push(decoded);
                    run = self.i;
                }
                0x00..=0x1f => {
                    return self.err(path, format!("raw control byte 0x{c:02x} must be escaped"))
                }
                _ => self.i += 1,
            }
        }
    }

    /// Append `b[run..i]` to `out`, rejecting invalid UTF-8 there. Raw
    /// runs are the only place non-ASCII bytes can appear.
    fn push_run(&self, path: &str, out: &mut String, run: usize) -> Result<(), CanonError> {
        match std::str::from_utf8(&self.b[run..self.i]) {
            Ok(s) => {
                out.push_str(s);
                Ok(())
            }
            Err(_) => self.err(path, "string is not valid UTF-8"),
        }
    }

    /// Decode one escape (the backslash is already consumed). Only the
    /// minimal forms are legal: a six-character spelling of a control
    /// that has a two-character escape, an escape of a character that
    /// needs none (a letter, a solidus), and uppercase hex are all
    /// rejected rather than silently accepted.
    fn escape(&mut self, path: &str) -> Result<char, CanonError> {
        let e = match self.peek() {
            Some(e) => e,
            None => return self.err(path, "unterminated escape"),
        };
        self.i += 1;
        let ch = match e {
            b'"' => '"',
            b'\\' => '\\',
            b'b' => '\u{8}',
            b't' => '\t',
            b'n' => '\n',
            b'f' => '\u{c}',
            b'r' => '\r',
            b'u' => {
                let hex = match self.b.get(self.i..self.i + 4) {
                    Some(h) => h,
                    None => return self.err(path, "truncated \\u escape"),
                };
                let mut code: u32 = 0;
                for d in hex {
                    // Lowercase only: `` is a second spelling of a
                    // value that already has one.
                    let v = match d {
                        b'0'..=b'9' => u32::from(d - b'0'),
                        b'a'..=b'f' => u32::from(d - b'a') + 10,
                        _ => {
                            return self
                                .err(path, "\\u escape needs four lowercase hex digits")
                        }
                    };
                    code = code * 16 + v;
                }
                self.i += 4;
                if code > 0x1f || has_short_escape(code) {
                    return self.err(
                        path,
                        format!("non-minimal escape \\u{code:04x}: that character has a shorter canonical spelling"),
                    );
                }
                // code <= 0x1f, so this is always a valid scalar value.
                char::from_u32(code).unwrap_or('\u{0}')
            }
            other => {
                return self.err(path, format!("unknown escape `\\{}`", other as char));
            }
        };
        Ok(ch)
    }

    /// A number. The canonical spelling is whatever `Display` produces,
    /// so the check is simply: parse it, print it back, demand the same
    /// bytes. That one rule subsumes leading zeros, `+`, trailing zeros,
    /// exponent forms and every other variant spelling.
    fn number(&mut self, path: &str) -> Result<Value, CanonError> {
        let start = self.i;
        while matches!(
            self.peek(),
            Some(b'-') | Some(b'+') | Some(b'0'..=b'9') | Some(b'.') | Some(b'e') | Some(b'E')
        ) {
            self.i += 1;
        }
        // Only ASCII bytes were consumed, so this cannot fail.
        let text = std::str::from_utf8(&self.b[start..self.i]).unwrap_or("");

        // `-0` is a float: it is what `{}` prints for -0.0_f64 and has no
        // i64 spelling. Everything with a `.` or an exponent marker is a
        // float too; anything else is an integer unless it overflows i64
        // (a large float prints as a long run of digits).
        let looks_float = text.contains('.') || text.contains('e') || text.contains('E');
        if !looks_float && text != "-0" {
            if let Ok(n) = text.parse::<i64>() {
                if n.to_string() == text {
                    return Ok(Value::Int(n));
                }
                return self.err(path, format!("non-canonical integer `{text}`"));
            }
        }
        let f: f64 = match text.parse::<f64>() {
            Ok(f) => f,
            Err(_) => return self.err(path, format!("not a number: `{text}`")),
        };
        if !f.is_finite() {
            return self.err(path, format!("non-finite number `{text}`"));
        }
        if format!("{f}") != text {
            return self.err(path, format!("non-canonical number `{text}`"));
        }
        Ok(Value::Float(f))
    }
}
