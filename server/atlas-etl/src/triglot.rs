use std::collections::{BTreeMap, HashMap};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::Path;

use anyhow::{Context, Result};

pub const SEARCH_TEXT: &str = "concordiatriglot00unse_hocr_searchtext.txt";
pub const PAGE_INDEX: &str = "concordiatriglot00unse_hocr_pageindex.json";
pub const PAGE_NUMBERS: &str = "concordiatriglot00unse_page_numbers.json";
pub const DATASET: &str = "triglot";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TriglotLeaf(pub u16);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TriglotPage(pub u16);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TriglotWord(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Coverage {
    pub shared: u32,
    pub total: u32,
}

impl Coverage {
    pub fn reaches(self, threshold_percent: u32) -> bool {
        self.total > 0 && self.shared * PERCENT >= threshold_percent * self.total
    }
}

const PERCENT: u32 = 100;

pub struct TriglotReference {
    words: Vec<String>,
    leaves: Vec<TriglotLeaf>,
    pages: BTreeMap<TriglotLeaf, TriglotPage>,
    shingles: HashMap<u64, Vec<u32>>,
    shingle_words: usize,
}

#[derive(serde::Deserialize)]
struct PageNumbers {
    pages: Vec<LeafPage>,
}

#[derive(serde::Deserialize)]
struct LeafPage {
    #[serde(rename = "leafNum")]
    leaf: u16,
    #[serde(rename = "pageNumber")]
    page: String,
}

impl TriglotReference {
    pub fn read(raw: &Path, shingle_words: usize) -> Result<TriglotReference> {
        let pinned = pinned_files(raw)?;
        let dir = raw.join(DATASET);
        let read = |name: &str| -> Result<String> {
            let bytes = std::fs::read(dir.join(name)).with_context(|| format!("reading {}", dir.join(name).display()))?;
            let found = hex(&atlas_graph_types::sha256::sha256(&bytes));
            match pinned.get(name) {
                Some(expected) if *expected == found => String::from_utf8(bytes).with_context(|| format!("{name} is not UTF-8")),
                expected => anyhow::bail!("data/raw/{DATASET}/{name}: sha256 {found} is not the one data/raw/MANIFEST.toml pins ({expected:?}); the Triglot reference changed"),
            }
        };
        let text = read(SEARCH_TEXT)?;
        let index: Vec<[usize; 4]> = serde_json::from_str(&read(PAGE_INDEX)?).with_context(|| format!("parsing {PAGE_INDEX}"))?;
        let numbers: PageNumbers = serde_json::from_str(&read(PAGE_NUMBERS)?).with_context(|| format!("parsing {PAGE_NUMBERS}"))?;
        Ok(TriglotReference::new(&text, &index, &numbers.pages, shingle_words))
    }

    fn new(text: &str, index: &[[usize; 4]], numbers: &[LeafPage], shingle_words: usize) -> TriglotReference {
        let starts: Vec<usize> = index.iter().map(|entry| entry[0]).collect();
        let (words, offsets) = words_with_offsets(text);
        let leaves = offsets.iter().map(|offset| TriglotLeaf((starts.partition_point(|start| start <= offset) - 1) as u16)).collect();
        let pages = numbers.iter().filter_map(|leaf| leaf.page.parse().ok().map(|page| (TriglotLeaf(leaf.leaf), TriglotPage(page)))).collect();
        let mut shingles: HashMap<u64, Vec<u32>> = HashMap::new();
        for (at, window) in words.windows(shingle_words).enumerate() {
            shingles.entry(shingle_of(window)).or_default().push(at as u32);
        }
        TriglotReference { words, leaves, pages, shingles, shingle_words }
    }

    #[cfg(test)]
    pub(crate) fn of_one_leaf(text: &str, page: &str, shingle_words: usize) -> TriglotReference {
        let index = [[0, text.chars().count(), 0, 0]];
        TriglotReference::new(text, &index, &[LeafPage { leaf: 0, page: page.to_string() }], shingle_words)
    }

    pub fn page_of(&self, word: TriglotWord) -> Option<TriglotPage> {
        self.pages.get(&self.leaves[word.0 as usize]).copied()
    }

    pub fn window(&self, first: TriglotLeaf, last: TriglotLeaf) -> std::ops::Range<u32> {
        let from = self.leaves.partition_point(|leaf| *leaf < first) as u32;
        let to = self.leaves.partition_point(|leaf| *leaf <= last) as u32;
        from..to
    }

    pub fn find(&self, text: &str, window: &std::ops::Range<u32>, after: TriglotWord) -> Found {
        let words = words_with_offsets(text).0;
        if words.len() < self.shingle_words {
            return self.find_whole(&words, window, after);
        }
        let mut starts: Vec<u32> = Vec::new();
        let windows: Vec<&[String]> = words.windows(self.shingle_words).collect();
        let mut shared = 0u32;
        for (offset, shingle) in windows.iter().enumerate() {
            let at = self.shingles.get(&shingle_of(shingle)).and_then(|places| places.iter().find(|place| window.contains(place) && **place >= after.0));
            if let Some(at) = at {
                shared += 1;
                starts.push(at.saturating_sub(offset as u32));
            }
        }
        starts.sort_unstable();
        Found { start: starts.get(starts.len() / 2).copied().map(TriglotWord), coverage: Coverage { shared, total: windows.len() as u32 } }
    }

    fn find_whole(&self, words: &[String], window: &std::ops::Range<u32>, after: TriglotWord) -> Found {
        let total = words.len() as u32;
        let start = (window.start.max(after.0) as usize..(window.end as usize).saturating_sub(words.len().saturating_sub(1))).find(|at| !words.is_empty() && self.words[*at..*at + words.len()] == *words);
        Found { start: start.map(|at| TriglotWord(at as u32)), coverage: Coverage { shared: if start.is_some() { total } else { 0 }, total } }
    }
}

#[derive(serde::Deserialize)]
struct RawManifest {
    dataset: Vec<RawDataset>,
}

#[derive(serde::Deserialize)]
struct RawDataset {
    name: String,
    #[serde(default)]
    file: Vec<RawFile>,
}

#[derive(serde::Deserialize)]
struct RawFile {
    name: String,
    sha256: String,
}

fn pinned_files(raw: &Path) -> Result<BTreeMap<String, String>> {
    let path = raw.join("MANIFEST.toml");
    let manifest: RawManifest = toml::from_str(&std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?).with_context(|| format!("parsing {}", path.display()))?;
    let dataset = manifest.dataset.into_iter().find(|dataset| dataset.name == DATASET).with_context(|| format!("{} records no '{DATASET}' dataset", path.display()))?;
    Ok(dataset.file.into_iter().map(|file| (file.name, file.sha256)).collect())
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Found {
    pub start: Option<TriglotWord>,
    pub coverage: Coverage,
}

fn shingle_of(words: &[String]) -> u64 {
    let mut hasher = DefaultHasher::new();
    words.hash(&mut hasher);
    hasher.finish()
}

fn words_with_offsets(text: &str) -> (Vec<String>, Vec<usize>) {
    let mut words: Vec<String> = Vec::new();
    let mut offsets: Vec<usize> = Vec::new();
    let mut current = String::new();
    let mut start = 0usize;
    let mut chars = text.chars().enumerate().peekable();
    while let Some((at, c)) = chars.next() {
        if c.is_alphanumeric() {
            if current.is_empty() {
                start = at;
            }
            current.extend(c.to_lowercase());
            continue;
        }
        if c == '-' && !current.is_empty() {
            let mut lookahead = chars.clone();
            while lookahead.peek().is_some_and(|(_, next)| next.is_whitespace()) {
                lookahead.next();
            }
            if lookahead.peek().is_some_and(|(_, next)| next.is_lowercase()) {
                chars = lookahead;
                continue;
            }
        }
        if !current.is_empty() {
            words.push(std::mem::take(&mut current));
            offsets.push(start);
        }
    }
    if !current.is_empty() {
        words.push(current);
        offsets.push(start);
    }
    (words, offsets)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHINGLE: usize = 3;

    fn reference(text: &str) -> TriglotReference {
        TriglotReference::of_one_leaf(text, "553", SHINGLE)
    }

    #[test]
    fn a_reference_whose_bytes_are_not_the_pinned_ones_is_refused() {
        // Arrange
        let raw = std::env::temp_dir().join(format!("triglot-pin-{}", std::process::id()));
        std::fs::create_dir_all(raw.join(DATASET)).unwrap();
        for name in [SEARCH_TEXT, PAGE_INDEX, PAGE_NUMBERS] {
            std::fs::write(raw.join(DATASET).join(name), "tampered").unwrap();
        }
        let pins = [SEARCH_TEXT, PAGE_INDEX, PAGE_NUMBERS].map(|name| format!("[[dataset.file]]\nname = \"{name}\"\nsha256 = \"{}\"\nbytes = 1\n", "0".repeat(64))).join("\n");
        std::fs::write(raw.join("MANIFEST.toml"), format!("[[dataset]]\nname = \"{DATASET}\"\n{pins}")).unwrap();
        // Act
        let refused = TriglotReference::read(&raw, SHINGLE).err().map(|refusal| refusal.to_string());
        // Assert
        std::fs::remove_dir_all(&raw).unwrap();
        assert!(refused.is_some_and(|refusal| refusal.contains("is not the one data/raw/MANIFEST.toml pins")));
    }

    #[test]
    fn a_hyphen_joins_its_word_whether_it_breaks_a_line_or_a_compound() {
        // Arrange
        let text = "confess be-\nfore you, a maid- servant; well-known";
        // Act
        let (words, _) = words_with_offsets(text);
        // Assert
        assert_eq!(words, vec!["confess", "before", "you", "a", "maidservant", "wellknown"]);
    }

    #[test]
    fn a_paragraph_the_scan_prints_is_found_whole_at_its_place_with_its_page() {
        // Arrange
        let triglot = reference("Proceed! I, a poor sinner, confess myself before God guilty of all sins.");
        let window = triglot.window(TriglotLeaf(0), TriglotLeaf(0));
        // Act
        let found = triglot.find("I, a poor sinner, confess myself before God", &window, TriglotWord(0));
        // Assert
        assert_eq!((found, found.start.and_then(|start| triglot.page_of(start))), (Found { start: Some(TriglotWord(1)), coverage: Coverage { shared: 6, total: 6 } }, Some(TriglotPage(553))));
    }

    #[test]
    fn a_paragraph_the_scan_does_not_print_shares_none_of_its_shingles() {
        // Arrange
        let triglot = reference("Proceed! I, a poor sinner, confess myself before God guilty of all sins.");
        let window = triglot.window(TriglotLeaf(0), TriglotLeaf(0));
        // Act
        let found = triglot.find("Do you believe that you are a sinner?", &window, TriglotWord(0));
        // Assert
        assert_eq!(found, Found { start: None, coverage: Coverage { shared: 0, total: 6 } });
    }

    #[test]
    fn a_unit_shorter_than_a_shingle_is_found_only_as_a_whole_run_of_words() {
        // Arrange
        let triglot = reference("Ernest, Duke of Lueneberg. Philip, Landgrave of Hesse.");
        let window = triglot.window(TriglotLeaf(0), TriglotLeaf(0));
        // Act
        let found = [triglot.find("Philip, Landgrave", &window, TriglotWord(0)), triglot.find("Landgrave Philip", &window, TriglotWord(0))];
        // Assert
        assert_eq!(found, [Found { start: Some(TriglotWord(4)), coverage: Coverage { shared: 2, total: 2 } }, Found { start: None, coverage: Coverage { shared: 0, total: 2 } }]);
    }

    #[test]
    fn a_passage_is_found_only_at_or_after_the_place_of_the_one_before_it() {
        // Arrange
        let triglot = reference("Thou shalt have no other gods. Then the meaning. Thou shalt have no other gods.");
        let window = triglot.window(TriglotLeaf(0), TriglotLeaf(0));
        // Act
        let found = [triglot.find("Thou shalt have no other gods", &window, TriglotWord(0)), triglot.find("Thou shalt have no other gods", &window, TriglotWord(7)), triglot.find("Thou shalt have no other gods", &window, TriglotWord(13))];
        // Assert
        assert_eq!(found.map(|found| found.start), [Some(TriglotWord(0)), Some(TriglotWord(9)), None]);
    }

    #[test]
    fn coverage_reaches_a_threshold_only_at_or_above_it_and_never_when_empty() {
        // Arrange
        let cases = [Coverage { shared: 2, total: 5 }, Coverage { shared: 1, total: 5 }, Coverage { shared: 0, total: 0 }];
        // Act
        let reached: Vec<bool> = cases.iter().map(|coverage| coverage.reaches(40)).collect();
        // Assert
        assert_eq!(reached, vec![true, false, false]);
    }
}
