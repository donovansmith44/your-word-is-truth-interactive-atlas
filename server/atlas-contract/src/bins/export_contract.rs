//! Writes every published contract document (spec D8). `--check` writes
//! nothing and exits non-zero when a committed document has drifted, which
//! is what a gate runs.

const USAGE: &str = "usage: export_contract [--check]";
const MISUSE: i32 = 2;
const STALE: i32 = 1;

fn main() {
    let mode = match mode(std::env::args().skip(1)) {
        Some(mode) => mode,
        None => {
            eprintln!("{USAGE}");
            std::process::exit(MISUSE);
        }
    };

    let mut stale = Vec::new();
    for (path, contents) in atlas_contract::document::generated_files() {
        let current = std::fs::read_to_string(&path).ok();
        if current.as_deref() == Some(contents.as_str()) {
            continue;
        }
        match mode {
            Mode::Check => stale.push(path.display().to_string()),
            Mode::Write => {
                std::fs::write(&path, contents).unwrap_or_else(|e| panic!("writing {}: {e}", path.display()));
                println!("wrote {}", path.display());
            }
        }
    }

    if !stale.is_empty() {
        eprintln!("stale generated documents: {stale:?}");
        std::process::exit(STALE);
    }
}

#[derive(Debug, PartialEq)]
enum Mode {
    Write,
    Check,
}

/// `None` is a misuse: a gate that silently wrote when it meant to check
/// would erase the drift it exists to report.
fn mode(arguments: impl IntoIterator<Item = String>) -> Option<Mode> {
    match Vec::from_iter(arguments).as_slice() {
        [] => Some(Mode::Write),
        [one] if one == "--check" => Some(Mode::Check),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn arguments(given: &[&str]) -> Vec<String> {
        given.iter().map(|a| (*a).to_string()).collect()
    }

    #[test]
    fn no_arguments_writes_and_only_check_checks_and_anything_else_is_a_misuse() {
        // Arrange
        let given = [vec![], arguments(&["--check"]), arguments(&["--write"]), arguments(&["--check", "--check"]), arguments(&[""])];
        // Act
        let modes: Vec<Option<Mode>> = given.into_iter().map(mode).collect();
        // Assert
        assert_eq!(modes, vec![Some(Mode::Write), Some(Mode::Check), None, None, None]);
    }
}
