use std::fs;
use std::path::{Path, PathBuf};

const CLIENT_WORDS: [&str; 6] = [
    "explore",
    "explorable",
    "frontier",
    "card",
    "popover",
    "presentation",
];

fn words_of(text: &str) -> Vec<String> {
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

fn is_client_word(word: &str) -> bool {
    CLIENT_WORDS.iter().any(|client| {
        word == *client
            || word.strip_suffix('s') == Some(*client)
            || (client.starts_with("explor") && word.starts_with("explor"))
    })
}

fn is_ident_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn scanned_text(source: &str) -> Vec<(usize, String)> {
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

fn raw_string_hashes(chars: &[char], at: usize) -> Option<usize> {
    if at > 0 && is_ident_char(chars[at - 1]) {
        return None;
    }
    let hashes = chars[at + 1..].iter().take_while(|&&c| c == '#').count();
    (chars.get(at + 1 + hashes) == Some(&'"')).then_some(hashes)
}

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repository root")
}

fn walk(dir: &Path, keep: &dyn Fn(&Path) -> bool, out: &mut Vec<PathBuf>) {
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

fn backend_sources() -> Vec<PathBuf> {
    let root = repository_root();
    let mut out = Vec::new();
    for dir in ["server", "graph-types"] {
        walk(
            &root.join(dir),
            &|p| p.extension().is_some_and(|e| e == "rs"),
            &mut out,
        );
    }
    out.sort();
    out
}

fn published_contract() -> Vec<PathBuf> {
    let root = repository_root().join("contracts");
    let mut out = vec![
        root.join("openapi.yaml"),
        root.join("atlas-query-contract/aqc.schema.json"),
    ];
    walk(
        &root.join("atlas-query-contract/features"),
        &|p| p.extension().is_some_and(|e| e == "feature"),
        &mut out,
    );
    out.sort();
    out
}

fn contract_text(path: &Path, text: &str) -> Vec<(usize, String)> {
    let is_feature = path.extension().is_some_and(|e| e == "feature");
    text.lines()
        .enumerate()
        .filter(|(_, line)| {
            let t = line.trim_start();
            !is_feature
                || t.starts_with("Feature:")
                || t.starts_with("Scenario:")
                || t.starts_with("Scenario Outline:")
                || t.starts_with('#')
        })
        .map(|(i, line)| (i + 1, line.to_string()))
        .collect()
}

fn offences_in(text: &[(usize, String)], shown: &str) -> Vec<String> {
    text.iter()
        .flat_map(|(line, piece)| {
            words_of(piece)
                .into_iter()
                .filter(|w| is_client_word(w))
                .map(move |w| format!("{shown}:{line}: {w}"))
        })
        .collect()
}

fn shown(path: &Path) -> String {
    path.strip_prefix(repository_root())
        .unwrap_or(path)
        .display()
        .to_string()
}

#[test]
fn no_backend_source_borrows_a_client_word() {
    // Arrange
    let sources = backend_sources();

    // Act
    let offences: Vec<String> = sources
        .iter()
        .flat_map(|path| {
            let name = shown(path);
            let source = fs::read_to_string(path).expect("readable source");
            let mut hits = offences_in(&[(0, name.clone())], "path");
            hits.extend(offences_in(&scanned_text(&source), &name));
            hits
        })
        .collect();

    // Assert
    assert!(offences.is_empty(), "{}", offences.join("\n"));
}

#[test]
fn the_published_contract_borrows_no_client_word() {
    // Arrange
    let documents = published_contract();

    // Act
    let offences: Vec<String> = documents
        .iter()
        .flat_map(|path| {
            let text = fs::read_to_string(path).expect("readable contract document");
            let mut hits = offences_in(&[(0, shown(path))], "path");
            hits.extend(offences_in(&contract_text(path, &text), &shown(path)));
            hits
        })
        .collect();

    // Assert
    assert!(offences.is_empty(), "{}", offences.join("\n"));
}

#[test]
fn a_camel_case_name_is_split_into_its_words() {
    // Arrange
    let camel = "NodeRecord";
    let snake = "adjacency_at";

    // Act
    let split = (words_of(camel), words_of(snake));

    // Assert
    assert_eq!(
        (
            vec!["node".to_string(), "record".to_string()],
            vec!["adjacency".to_string(), "at".to_string()]
        ),
        split
    );
}

#[test]
fn a_word_that_merely_contains_a_client_word_is_not_one() {
    // Arrange
    let words = ["jaccard", "discard", "cardinal"];

    // Act
    let verdicts: Vec<bool> = words.iter().map(|w| is_client_word(w)).collect();

    // Assert
    assert_eq!(vec![false, false, false], verdicts);
}

#[test]
fn a_form_of_a_client_word_is_one() {
    // Arrange
    let words = ["cards", "frontiers", "exploration", "explored", "popover"];

    // Act
    let verdicts: Vec<bool> = words.iter().map(|w| is_client_word(w)).collect();

    // Assert
    assert_eq!(vec![true; 5], verdicts);
}

#[test]
fn a_string_literal_is_not_scanned_but_a_doc_comment_is() {
    // Arrange
    let source = "/// the card\nfn kept() { let s = \"frontier\"; let r = r#\"popover\"#; }\n";

    // Act
    let scanned = scanned_text(source);

    // Assert
    let expected: Vec<(usize, String)> = vec![
        (1, "/// the card".into()),
        (2, "fn".into()),
        (2, "kept".into()),
        (2, "let".into()),
        (2, "s".into()),
        (2, "let".into()),
        (2, "r".into()),
    ];
    assert_eq!(expected, scanned);
}
