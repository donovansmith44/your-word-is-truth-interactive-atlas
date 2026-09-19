//! LEX-1 (spec §7): the lexicon reader over the vendored
//! `data/raw/brain-fuel-bible/{lexicon,morph}` tree at the SAME pinned
//! commit CORP-1a's `brainfuel.rs` reads (`94d44842cb242e8aa840330748e03d2803f2a7c1`).
//!
//! Two sources, one corpus:
//!
//! - `lexicon/{grc,hbo}/<STRONG>.json` -- 13,548 Strong's-keyed entries
//!   (5,122 Greek + 8,426 Hebrew): `strong, lemma, translit, lang, pos,
//!   glosses{en:[{text,src}]}, senses[{id,gloss_en,domain}], domains[],
//!   root, sources[]`. The `lemma-*.json` files beside them are LXX-only
//!   entries (the owner's standing "no apocrypha for now"; spec §7.4) and
//!   are never read: only `[GH]NNNN.json` names are.
//! - `morph/{nt,ot}/<UPSTREAM_CODE>/NNN.conllu` -- 452,689 tokens (140,610
//!   Greek + 312,079 Hebrew), one sentence per verse under a `# ref =
//!   CODE.c.v` comment, NINE tab-separated columns (`id FORM LEMMA UPOS
//!   XPOS FEATS HEAD DEPREL MISC` -- upstream omits CoNLL-U's DEPS), MISC
//!   `Strong=G3972|Translit=Paulos[|Align=...]`. A token is ALIGNED when
//!   MISC carries `Strong=`; `Align=unmatched` (4.70% NT / 4.74% OT, by
//!   upstream design) marks the ones that are not. `Align=source_extra:N`
//!   is an upstream alignment note on an aligned token and is not carried.
//!   `morph/lxx` is never read (spec §7.4).
//!
//! Upstream book codes (`JOH`, `SOS`, ...) resolve through the SAME
//! `brainfuel::book_code_map` over `data/books.json` CORP-1a built, so the
//! two readers can never disagree on which book a chapter file belongs to.
//!
//! Pure `&str`-in parsers (`parse_entry`, `parse_conllu`) plus the one
//! filesystem walk (`read_all`), the module shape every other corpus reader
//! in this crate has.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{bail, Context, Result};
use atlas_core::refs::BookId;
use serde::Deserialize;

/// The rendering-layer ids the tokens address (spec §7.3) -- the SAME
/// strings `brainfuel.rs` registers as `TranslationId`s for the Greek
/// Textus Receptus and the Hebrew Masoretic renderings, so a `TokenSpan`
/// built from a token names a layer every verse already carries.
pub const LAYER_GREEK: &str = "greek_textus_receptus";
pub const LAYER_HEBREW: &str = "hebrew_masoretic";

/// One lexicon entry, as published (spec §7.2's payload, field for field).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LexEntry {
    pub strong: String,
    /// `"grc"` | `"hbo"`.
    pub lang: String,
    /// Empty for sixteen upstream entries (three extended-Strong's Greek
    /// ids and thirteen Hebrew affix/particle pseudo-entries such as
    /// `H9033`) -- carried as published, never invented.
    pub lemma: String,
    pub translit: Option<String>,
    pub pos: Option<String>,
    /// English gloss texts in source order (`glosses.en[].text`).
    pub glosses: Vec<String>,
    /// `senses[].gloss_en` in `id` order.
    pub senses: Vec<String>,
    /// Sorted, deduplicated atomic domain codes (Louw-Nida for Greek, SDBH
    /// for Hebrew).
    pub domains: Vec<String>,
    /// The Strong's id of the root entry, when upstream gives one.
    pub root: Option<String>,
}

/// One CoNLL-U token, matched or not (the `token` inventory, spec §5.7).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TokenRow {
    pub book: BookId,
    pub chapter: u16,
    pub verse: u16,
    /// [`LAYER_GREEK`] | [`LAYER_HEBREW`], by testament directory.
    pub layer: &'static str,
    /// The CoNLL-U token id within the verse, 1-based.
    pub ord: u16,
    pub form: String,
    pub lemma: Option<String>,
    pub xpos: Option<String>,
    pub translit: Option<String>,
    /// `Some` exactly when `aligned`.
    pub strong: Option<String>,
    pub aligned: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LexiconStats {
    pub entries_grc: usize,
    pub entries_hbo: usize,
    pub tokens_nt: usize,
    pub tokens_ot: usize,
    pub unmatched_nt: usize,
    pub unmatched_ot: usize,
    /// CoNLL-U chapter files read.
    pub files: usize,
}

#[derive(Clone, Debug, Default)]
pub struct LexiconCorpus {
    /// Sorted by `strong`, unique.
    pub entries: Vec<LexEntry>,
    /// Sorted by (book index, chapter, verse, ord) -- canonical reading
    /// order, independent of the directory listing.
    pub tokens: Vec<TokenRow>,
    pub stats: LexiconStats,
}

// ------------------------------------------------------------ the JSON

#[derive(Deserialize)]
struct RawEntry {
    strong: String,
    lang: String,
    #[serde(default)]
    lemma: String,
    #[serde(default)]
    translit: String,
    #[serde(default)]
    pos: String,
    #[serde(default)]
    glosses: RawGlosses,
    #[serde(default)]
    senses: Vec<RawSense>,
    #[serde(default)]
    domains: Vec<String>,
    #[serde(default)]
    root: Option<String>,
}

#[derive(Deserialize, Default)]
struct RawGlosses {
    #[serde(default)]
    en: Vec<RawGloss>,
}

#[derive(Deserialize)]
struct RawGloss {
    text: String,
}

#[derive(Deserialize)]
struct RawSense {
    id: u32,
    #[serde(default)]
    gloss_en: String,
}

fn non_empty(s: String) -> Option<String> {
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

/// One `lexicon/<lang>/<STRONG>.json` body. Empty strings become `None`
/// on the optional fields; `senses` are ordered by their `id`; `domains`
/// are sorted and deduplicated.
pub fn parse_entry(json: &str) -> Result<LexEntry> {
    let raw: RawEntry = serde_json::from_str(json).context("lexicon entry is not the expected JSON shape")?;
    if !(raw.strong.starts_with('G') || raw.strong.starts_with('H')) || raw.strong.len() < 5 {
        bail!("lexicon entry has a malformed strong id {:?}", raw.strong);
    }
    if raw.lang != "grc" && raw.lang != "hbo" {
        bail!("lexicon entry {} has an unexpected lang {:?}", raw.strong, raw.lang);
    }
    let mut senses = raw.senses;
    senses.sort_by_key(|s| s.id);
    let mut domains = raw.domains;
    domains.sort();
    domains.dedup();
    Ok(LexEntry {
        strong: raw.strong,
        lang: raw.lang,
        lemma: raw.lemma,
        translit: non_empty(raw.translit),
        pos: non_empty(raw.pos),
        glosses: raw.glosses.en.into_iter().map(|g| g.text).collect(),
        senses: senses.into_iter().map(|s| s.gloss_en).collect(),
        domains,
        root: raw.root.and_then(non_empty),
    })
}

/// True for the `[GH]NNNN.json` names that are Strong's entries -- the
/// `lemma-*.json` LXX-only files beside them are skipped.
fn is_strongs_file_name(name: &str) -> bool {
    let Some(stem) = name.strip_suffix(".json") else { return false };
    let mut chars = stem.chars();
    matches!(chars.next(), Some('G') | Some('H')) && stem.len() >= 5 && chars.all(|c| c.is_ascii_digit())
}

// ------------------------------------------------------------ the CoNLL-U

fn absent(col: &str) -> Option<String> {
    if col == "_" {
        None
    } else {
        Some(col.to_string())
    }
}

/// One chapter file. `book` is the file's own book (from its directory,
/// through `book_code_map`); every `# ref = CODE.c.v` line must name the
/// same code. Tokens are emitted in file order.
pub fn parse_conllu(text: &str, book: BookId, expected_code: &str, layer: &'static str, out: &mut Vec<TokenRow>, unmatched: &mut usize) -> Result<usize> {
    let mut chapter_verse: Option<(u16, u16)> = None;
    let mut count = 0usize;
    for (i, line) in text.lines().enumerate() {
        let ln = i + 1;
        if line.is_empty() {
            continue;
        }
        if let Some(rest) = line.strip_prefix('#') {
            let rest = rest.trim();
            if let Some(r) = rest.strip_prefix("ref =") {
                let r = r.trim();
                let mut parts = r.split('.');
                let (Some(code), Some(c), Some(v), None) = (parts.next(), parts.next(), parts.next(), parts.next()) else {
                    bail!("line {ln}: malformed ref {r:?}");
                };
                if code != expected_code {
                    bail!("line {ln}: ref {r:?} names book {code} inside the {expected_code} directory");
                }
                let c: u16 = c.parse().with_context(|| format!("line {ln}: chapter in ref {r:?}"))?;
                let v: u16 = v.parse().with_context(|| format!("line {ln}: verse in ref {r:?}"))?;
                chapter_verse = Some((c, v));
            }
            continue;
        }
        let cols: Vec<&str> = line.split('\t').collect();
        if cols.len() != 9 {
            bail!("line {ln}: expected 9 tab-separated columns, found {}", cols.len());
        }
        let Some((chapter, verse)) = chapter_verse else {
            bail!("line {ln}: token before any `# ref =` line");
        };
        let ord: u16 = cols[0].parse().with_context(|| format!("line {ln}: token id {:?}", cols[0]))?;
        let mut strong = None;
        let mut translit = None;
        let mut align_unmatched = false;
        if cols[8] != "_" {
            for kv in cols[8].split('|') {
                match kv.split_once('=') {
                    Some(("Strong", s)) => strong = Some(s.to_string()),
                    Some(("Translit", t)) => translit = non_empty(t.to_string()),
                    Some(("Align", a)) => align_unmatched = a.split(',').any(|p| p == "unmatched"),
                    _ => bail!("line {ln}: unexpected MISC field {kv:?}"),
                }
            }
        }
        let aligned = strong.is_some();
        if aligned == align_unmatched {
            bail!("line {ln}: Strong= and Align=unmatched disagree ({:?})", cols[8]);
        }
        if !aligned {
            *unmatched += 1;
        }
        out.push(TokenRow {
            book,
            chapter,
            verse,
            layer,
            ord,
            form: cols[1].to_string(),
            lemma: absent(cols[2]),
            xpos: absent(cols[4]),
            translit,
            strong,
            aligned,
        });
        count += 1;
    }
    Ok(count)
}

// ------------------------------------------------------------ the walk

/// Reads `lexicon/{grc,hbo}` and `morph/{nt,ot}` under `root`
/// (`data/raw/brain-fuel-bible`). Fails loud on a malformed file, a
/// duplicate Strong's id, or an unresolvable book directory.
pub fn read_all(root: &Path) -> Result<LexiconCorpus> {
    let books_json = std::fs::read_to_string(root.join("data/books.json")).with_context(|| format!("reading {}", root.join("data/books.json").display()))?;
    let book_codes = crate::brainfuel::book_code_map(&books_json)?;

    let mut stats = LexiconStats::default();
    let mut entries: Vec<LexEntry> = Vec::new();
    for lang in ["grc", "hbo"] {
        let dir = root.join("lexicon").join(lang);
        let mut files: Vec<std::path::PathBuf> = std::fs::read_dir(&dir)
            .with_context(|| format!("reading directory {}", dir.display()))?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.file_name().and_then(|n| n.to_str()).map(is_strongs_file_name).unwrap_or(false))
            .collect();
        files.sort();
        for path in files {
            let text = std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
            let entry = parse_entry(&text).with_context(|| format!("parsing {}", path.display()))?;
            if entry.lang != lang {
                bail!("{}: lang {:?} inside the {lang} directory", path.display(), entry.lang);
            }
            match lang {
                "grc" => stats.entries_grc += 1,
                _ => stats.entries_hbo += 1,
            }
            entries.push(entry);
        }
    }
    entries.sort_by(|a, b| a.strong.cmp(&b.strong));
    if let Some(w) = entries.windows(2).find(|w| w[0].strong == w[1].strong) {
        bail!("duplicate lexicon entry {}", w[0].strong);
    }

    let mut tokens: Vec<TokenRow> = Vec::new();
    for (testament, layer) in [("nt", LAYER_GREEK), ("ot", LAYER_HEBREW)] {
        let testament_dir = root.join("morph").join(testament);
        let mut book_dirs: Vec<std::path::PathBuf> = std::fs::read_dir(&testament_dir)
            .with_context(|| format!("reading directory {}", testament_dir.display()))?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .collect();
        book_dirs.sort();
        let mut unmatched = 0usize;
        let mut count = 0usize;
        for book_dir in book_dirs {
            let code = book_dir.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
            let Some(book) = book_codes.get(&code).copied() else {
                bail!("{}: upstream book code {code:?} is not in data/books.json's OT/NT set", book_dir.display());
            };
            let mut files: Vec<std::path::PathBuf> = std::fs::read_dir(&book_dir)
                .with_context(|| format!("reading directory {}", book_dir.display()))?
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("conllu"))
                .collect();
            files.sort();
            for path in files {
                let text = std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
                count += parse_conllu(&text, book, &code, layer, &mut tokens, &mut unmatched).with_context(|| format!("parsing {}", path.display()))?;
                stats.files += 1;
            }
        }
        match testament {
            "nt" => {
                stats.tokens_nt = count;
                stats.unmatched_nt = unmatched;
            }
            _ => {
                stats.tokens_ot = count;
                stats.unmatched_ot = unmatched;
            }
        }
    }
    tokens.sort_by_key(|t| (t.book.0, t.chapter, t.verse, t.ord));

    Ok(LexiconCorpus { entries, tokens, stats })
}

/// The entries keyed by Strong's id (what the adapter resolves tokens
/// against).
pub fn entry_index(corpus: &LexiconCorpus) -> HashMap<&str, &LexEntry> {
    corpus.entries.iter().map(|e| (e.strong.as_str(), e)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const G3056: &str = r#"{"strong": "G3056", "lemma": "λόγος", "translit": "lógos", "lang": "grc", "pos": "G:N-M",
      "glosses": {"en": [{"text": "from G3004; something said", "src": "strongs-greek"}, {"text": "word", "src": "tbesg"}]},
      "senses": [{"id": 2, "gloss_en": "second", "domain": null}, {"id": 1, "gloss_en": "first", "domain": null}],
      "domains": ["33.99", "13.115", "33.99"], "root": "G3004", "sources": ["strongs-greek", "tbesg", "ln-map"]}"#;

    #[test]
    fn an_entry_parses_with_senses_by_id_and_domains_sorted_unique() {
        let e = parse_entry(G3056).unwrap();
        assert_eq!(e.strong, "G3056");
        assert_eq!(e.lemma, "λόγος");
        assert_eq!(e.translit.as_deref(), Some("lógos"));
        assert_eq!(e.pos.as_deref(), Some("G:N-M"));
        assert_eq!(e.glosses, vec!["from G3004; something said", "word"]);
        assert_eq!(e.senses, vec!["first", "second"]);
        assert_eq!(e.domains, vec!["13.115", "33.99"]);
        assert_eq!(e.root.as_deref(), Some("G3004"));
    }

    #[test]
    fn empty_strings_are_none_and_a_bare_entry_still_parses() {
        let e = parse_entry(r#"{"strong": "H9033", "lemma": "", "translit": "", "lang": "hbo", "pos": "", "glosses": {"en": []}, "senses": [{"id": 1, "gloss_en": "", "domain": null}], "domains": [], "root": null, "sources": []}"#).unwrap();
        assert_eq!(e.lemma, "");
        assert_eq!(e.translit, None);
        assert_eq!(e.pos, None);
        assert_eq!(e.root, None);
        assert!(e.glosses.is_empty());
        assert_eq!(e.senses, vec![""]);
    }

    #[test]
    fn lxx_only_lemma_files_are_not_strongs_files() {
        assert!(is_strongs_file_name("G3056.json"));
        assert!(is_strongs_file_name("H0430.json"));
        assert!(is_strongs_file_name("G10000.json"));
        assert!(!is_strongs_file_name("lemma-abc.json"));
        assert!(!is_strongs_file_name("G3056.txt"));
        assert!(!is_strongs_file_name("G30.json"));
    }

    const JOH3: &str = "# ref = JOH.3.1\n1\tἦν\tεἰμί\tVERB\tV-IAI-3S\tMood=Ind\t_\t_\tStrong=G1510|Translit=Ēn\n2\tδὲ\tδέ\tCCONJ\tCONJ\t_\t_\t_\tStrong=G1161|Translit=de|Align=source_extra:1\n3\tχριστοῦ\t_\tX\t_\t_\t_\t_\tAlign=unmatched\n\n# ref = JOH.3.2\n1\tοὗτος\t_\tX\t_\t_\t_\t_\tAlign=unmatched,source_extra:2\n";

    #[test]
    fn a_chapter_file_parses_tokens_with_alignment_and_verse_refs() {
        let mut out = Vec::new();
        let mut unmatched = 0;
        let book = atlas_core::canon::resolve_alias("John").unwrap();
        let n = parse_conllu(JOH3, book, "JOH", LAYER_GREEK, &mut out, &mut unmatched).unwrap();
        assert_eq!(n, 4);
        assert_eq!(unmatched, 2);
        assert_eq!(out[0].strong.as_deref(), Some("G1510"));
        assert_eq!(out[0].translit.as_deref(), Some("Ēn"));
        assert_eq!((out[0].chapter, out[0].verse, out[0].ord), (3, 1, 1));
        assert_eq!(out[0].xpos.as_deref(), Some("V-IAI-3S"));
        assert!(out[1].aligned, "Align=source_extra on a Strong= token is still aligned");
        assert!(!out[2].aligned);
        assert_eq!(out[2].strong, None);
        assert_eq!(out[2].lemma, None);
        assert_eq!(out[2].form, "χριστοῦ");
        assert_eq!((out[3].chapter, out[3].verse), (3, 2));
        assert!(!out[3].aligned, "Align=unmatched,source_extra:2 is unmatched");
        assert_eq!(out[3].strong, None);
        assert_eq!(out[3].form, "οὗτος");
    }

    #[test]
    fn a_ref_in_the_wrong_book_or_a_short_line_fails_loud() {
        let book = atlas_core::canon::resolve_alias("John").unwrap();
        let mut out = Vec::new();
        let mut u = 0;
        let err = parse_conllu("# ref = MAT.1.1\n1\ta\tb\tX\t_\t_\t_\t_\tStrong=G0001|Translit=a\n", book, "JOH", LAYER_GREEK, &mut out, &mut u).unwrap_err().to_string();
        assert!(err.contains("MAT"), "{err}");
        let err = parse_conllu("# ref = JOH.1.1\n1\ta\tb\n", book, "JOH", LAYER_GREEK, &mut out, &mut u).unwrap_err().to_string();
        assert!(err.contains("9 tab-separated"), "{err}");
        let err = parse_conllu("# ref = JOH.1.1\n1\ta\tb\tX\t_\t_\t_\t_\tStrong=G0001|Align=unmatched\n", book, "JOH", LAYER_GREEK, &mut out, &mut u).unwrap_err().to_string();
        assert!(err.contains("disagree"), "{err}");
    }
}
