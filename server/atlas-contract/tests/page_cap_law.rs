mod common;

use atlas_contract::graph::LARGEST_PAGE;
use common::source_scan::{read, repository_root, shown, walk};

const BEFORE_A_CAP: [&str; 8] = ["limit", "caps at", "more than", "max_", "page", "min(", "clamp(", "largest"];

fn is_source(path: &std::path::Path) -> bool {
    let parts: Vec<_> = path.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect();
    !parts.iter().any(|part| part == "bin" || part == "obj" || part == "node_modules")
        && path.extension().is_some_and(|ext| ext == "rs" || ext == "cs" || ext == "razor")
}

fn names_a_cap_of_two_hundred(line: &str) -> bool {
    let lower = line.to_lowercase();
    lower.match_indices("200").any(|(at, _)| {
        let before = &lower[..at];
        let after = &lower[at + 3..];
        !before.ends_with(|c: char| c.is_ascii_digit() || c == '.' || c == '_')
            && !after.starts_with(|c: char| c.is_ascii_digit() || c == '.' || c == '_')
            && BEFORE_A_CAP.iter().any(|word| before.contains(word))
    })
}

#[test]
fn no_page_cap_is_written_anywhere_but_the_servers_one_constant() {
    // Arrange
    let mut sources = Vec::new();
    walk(&repository_root().join("server"), &is_source, &mut sources);
    for dir in ["client", "client.Tests", "client.ContractTests"] {
        walk(&repository_root().join(dir), &is_source, &mut sources);
    }
    sources.sort();

    let the_cap_definition = format!("pub const LARGEST_PAGE: usize = {LARGEST_PAGE};");

    // Act
    let offenders: Vec<String> = sources
        .iter()
        .flat_map(|path| {
            read(path)
                .lines()
                .enumerate()
                .filter(|(_, line)| line.trim() != the_cap_definition && names_a_cap_of_two_hundred(line))
                .map(|(n, line)| format!("{}:{}: {}", shown(path), n + 1, line.trim()))
                .collect::<Vec<_>>()
        })
        .collect();

    // Assert
    assert_eq!(offenders, Vec::<String>::new());
}
