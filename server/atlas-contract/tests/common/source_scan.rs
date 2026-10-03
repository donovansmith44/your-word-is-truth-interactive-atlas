#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

pub fn words_of(text: &str) -> Vec<String> {
    let mut words = Vec::new();
    for piece in text.split(|c: char| !c.is_alphanumeric()) {
        let chars: Vec<char> = piece.chars().collect();
        let mut current = String::new();
        for (i, &c) in chars.iter().enumerate() {
            let boundary = i > 0
                && c.is_uppercase()
                && (chars[i - 1].is_lowercase()
                    || chars[i - 1].is_ascii_digit()
                    || chars.get(i + 1).is_some_and(|n| n.is_lowercase()));
            if boundary && !current.is_empty() {
                words.push(current.to_lowercase());
                current = String::new();
            }
            current.push(c);
        }
        if !current.is_empty() {
            words.push(current.to_lowercase());
        }
    }
    words
}

pub fn is_ident_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

pub fn scanned_text(source: &str) -> Vec<(usize, String)> {
    let chars: Vec<char> = source.chars().collect();
    let mut found = Vec::new();
    let mut line = 1;
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        if c == '\n' {
            line += 1;
            i += 1;
        } else if c == '/' && next == Some('/') {
            let start = i;
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            found.push((line, chars[start..i].iter().collect()));
        } else if c == '/' && next == Some('*') {
            let mut depth = 0;
            let mut text = String::new();
            let mut text_line = line;
            loop {
                if i >= chars.len() {
                    break;
                }
                if chars[i] == '/' && chars.get(i + 1) == Some(&'*') {
                    depth += 1;
                    text.push_str("/*");
                    i += 2;
                } else if chars[i] == '*' && chars.get(i + 1) == Some(&'/') {
                    depth -= 1;
                    text.push_str("*/");
                    i += 2;
                    if depth == 0 {
                        break;
                    }
                } else {
                    if chars[i] == '\n' {
                        found.push((text_line, std::mem::take(&mut text)));
                        line += 1;
                        text_line = line;
                    } else {
                        text.push(chars[i]);
                    }
                    i += 1;
                }
            }
            found.push((text_line, text));
        } else if c == '"' || (c == 'r' && raw_string_hashes(&chars, i).is_some()) {
            let (hashes, body_start) = if c == '"' {
                (0, i + 1)
            } else {
                let h = raw_string_hashes(&chars, i).unwrap_or(0);
                (h, i + 2 + h)
            };
            i = body_start;
            let raw = c != '"';
            while i < chars.len() {
                if chars[i] == '\n' {
                    line += 1;
                }
                if !raw && chars[i] == '\\' {
                    if chars.get(i + 1) == Some(&'\n') {
                        line += 1;
                    }
                    i += 2;
                    continue;
                }
                if chars[i] == '"' && (0..hashes).all(|k| chars.get(i + 1 + k) == Some(&'#')) {
                    i += 1 + hashes;
                    break;
                }
                i += 1;
            }
        } else if c == '\'' {
            let closes_at = if next == Some('\\') {
                (i + 2..chars.len().min(i + 12)).find(|&k| chars[k] == '\'')
            } else if chars.get(i + 2) == Some(&'\'') {
                Some(i + 2)
            } else {
                None
            };
            i = closes_at.map_or(i + 1, |k| k + 1);
        } else if is_ident_char(c) {
            let start = i;
            while i < chars.len() && is_ident_char(chars[i]) {
                i += 1;
            }
            found.push((line, chars[start..i].iter().collect()));
        } else {
            i += 1;
        }
    }
    found
}

pub fn raw_string_hashes(chars: &[char], at: usize) -> Option<usize> {
    if at > 0 && is_ident_char(chars[at - 1]) {
        return None;
    }
    let hashes = chars[at + 1..].iter().take_while(|&&c| c == '#').count();
    (chars.get(at + 1 + hashes) == Some(&'"')).then_some(hashes)
}

pub fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repository root")
}

pub fn walk(dir: &Path, keep: &dyn Fn(&Path) -> bool, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        if name == "target" || name == ".git" {
            continue;
        }
        if path.is_dir() {
            walk(&path, keep, out);
        } else if keep(&path) {
            out.push(path);
        }
    }
}

pub fn shown(path: &Path) -> String {
    path.strip_prefix(repository_root())
        .unwrap_or(path)
        .display()
        .to_string()
}

pub fn code_of(source: &str) -> String {
    let chars: Vec<char> = source.chars().collect();
    let mut code = String::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        if c == '/' && next == Some('/') {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
        } else if c == '/' && next == Some('*') {
            let mut depth = 0;
            while i < chars.len() {
                if chars[i] == '/' && chars.get(i + 1) == Some(&'*') {
                    depth += 1;
                    i += 2;
                } else if chars[i] == '*' && chars.get(i + 1) == Some(&'/') {
                    depth -= 1;
                    i += 2;
                    if depth == 0 {
                        break;
                    }
                } else {
                    i += 1;
                }
            }
        } else if c == '"' || (c == 'r' && raw_string_hashes(&chars, i).is_some()) {
            let hashes = if c == '"' { 0 } else { raw_string_hashes(&chars, i).unwrap_or(0) };
            let raw = c != '"';
            i = if raw { i + 2 + hashes } else { i + 1 };
            while i < chars.len() {
                if !raw && chars[i] == '\\' {
                    i += 2;
                    continue;
                }
                if chars[i] == '"' && (0..hashes).all(|k| chars.get(i + 1 + k) == Some(&'#')) {
                    i += 1 + hashes;
                    break;
                }
                i += 1;
            }
            code.push_str("\"\"");
        } else if is_ident_char(c) {
            if code.chars().last().is_some_and(is_ident_char) {
                code.push(' ');
            }
            while i < chars.len() && is_ident_char(chars[i]) {
                code.push(chars[i]);
                i += 1;
            }
        } else {
            if !c.is_whitespace() {
                code.push(c);
            }
            i += 1;
        }
    }
    code
}

pub fn rust_sources_under(dirs: &[&str]) -> Vec<PathBuf> {
    let root = repository_root();
    let mut out = Vec::new();
    for dir in dirs {
        let at = root.join(dir);
        if at.is_file() {
            out.push(at);
        } else {
            walk(&at, &|p| p.extension().is_some_and(|e| e == "rs"), &mut out);
        }
    }
    out.sort();
    out
}

pub fn read(path: &Path) -> String {
    fs::read_to_string(path).expect("readable source")
}

pub fn string_literals_of(source: &str) -> Vec<String> {
    let chars: Vec<char> = source.chars().collect();
    let mut literals = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        if c == '/' && next == Some('/') {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
        } else if c == '"' || (c == 'r' && raw_string_hashes(&chars, i).is_some()) {
            let hashes = if c == '"' { 0 } else { raw_string_hashes(&chars, i).unwrap_or(0) };
            let raw = c != '"';
            i = if raw { i + 2 + hashes } else { i + 1 };
            let mut literal = String::new();
            while i < chars.len() {
                if !raw && chars[i] == '\\' {
                    i += 2;
                    continue;
                }
                if chars[i] == '"' && (0..hashes).all(|k| chars.get(i + 1 + k) == Some(&'#')) {
                    i += 1 + hashes;
                    break;
                }
                literal.push(chars[i]);
                i += 1;
            }
            literals.push(literal);
        } else if is_ident_char(c) {
            while i < chars.len() && is_ident_char(chars[i]) {
                i += 1;
            }
        } else {
            i += 1;
        }
    }
    literals
}
