mod common;

use std::fs;
use std::path::{Path, PathBuf};

use common::source_scan::{repository_root, rust_sources_under, scanned_text, shown, walk, words_of};

const CLIENT_WORDS: [&str; 6] = [
    "explore",
    "explorable",
    "frontier",
    "card",
    "popover",
    "presentation",
];

fn is_client_word(word: &str) -> bool {
    CLIENT_WORDS.iter().any(|client| {
        word == *client
            || word.strip_suffix('s') == Some(*client)
            || (client.starts_with("explor") && word.starts_with("explor"))
    })
}

fn backend_sources() -> Vec<PathBuf> {
    rust_sources_under(&["server", "graph-types"])
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
