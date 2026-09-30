//! Where the Book of Concord cites Scripture, found once here so the reader never searches for it.
//! The grammar is the one the reader used: a book, named in full or by an abbreviation, then a
//! chapter and a verse, and optionally the last verse of a range in that chapter or an "and
//! following" (`f`, `ff`, `sq`, `sqq`) written onto the verse. Each citation in a
//! paragraph becomes a `cites` row from the paragraph's words that spell it to the verses it names; a
//! citation that does not lie on whole words, or names a verse the Bible does not hold, is refused
//! and counted, never guessed at. An "and following" cites the stated verse only: how far it runs
//! is the author's to say, and the citation does not say it.

use std::ops::Range;
use std::sync::LazyLock;

use atlas_core::canon::{position_of, BOOKS};
use atlas_core::refs::{BookId, ScriptureRef, VerseId};
use atlas_graph_types::edge::CrossRef;
use atlas_graph_types::graph::Graph;
use atlas_graph_types::text::{BibleLocus, BibleLocusRange, ConcordRef, Locus, LocusRange, TextLocus, TextRef, TokenSpan, VerseRef};
use regex::{Captures, Regex};

use crate::concord_adapter::{self, CONCORD_CORPUS, CONCORD_TRANSLATION};
use crate::kjv_adapter;
use crate::tokens;

pub const PROVENANCE: &str = "concord-citations";

/// One citation: the Unicode-scalar range of the text that spells it, and the verses it names.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Citation {
    pub chars: Range<usize>,
    pub cites: BibleLocusRange,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CitationSpan {
    pub cites: VerseRef,
    pub words: TokenSpan,
}

impl CitationSpan {
    pub fn of(row: &CrossRef) -> Option<CitationSpan> {
        match (&row.from.span, &row.to.at) {
            (Some(words), TextRef::Bible(cites)) => Some(CitationSpan { cites: cites.clone(), words: words.clone() }),
            _ => None,
        }
    }
}

/// `cited` counts the rows written; `off_words` the citations refused because they do not start
/// where a word starts and end where a word ends ("Rom. 13:8a"); `no_such_verse` those refused
/// because a verse they name is not in the Bible.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CitationStats {
    pub cited: usize,
    pub off_words: usize,
    pub no_such_verse: usize,
}

/// Runs once the verses and the Concord's paragraphs are nodes. Rows follow the reading order of
/// the paragraphs, and a paragraph's citations follow one another in text order.
pub fn cite_scripture(graph: &mut Graph) -> CitationStats {
    let mut stats = CitationStats::default();
    let mut rows = Vec::new();
    for id in graph.reading.get(CONCORD_CORPUS).map(|spine| spine.order.as_slice()).unwrap_or_default() {
        let (part, article, paragraph) = concord_adapter::decode_text_unit(id).expect("the Concord's reading order holds its paragraphs");
        let text = graph.nodes.get(id).and_then(concord_adapter::concord_text).expect("a paragraph in the reading order is a node carrying its text");
        let words = tokens::tokenize(text);
        for citation in scan(text) {
            if ![&citation.cites.from, &citation.cites.to].iter().all(|end| graph.nodes.contains_key(&kjv_adapter::verse_node_id(end.unit.book, end.unit.chapter, end.unit.verse))) {
                stats.no_such_verse += 1;
                continue;
            }
            let Some(span) = tokens::words_covering(&citation.chars, &words, CONCORD_TRANSLATION) else {
                stats.off_words += 1;
                continue;
            };
            stats.cited += 1;
            rows.push(CrossRef {
                from: TextLocus { at: TextRef::Concord(ConcordRef { part, article, paragraph }), span: Some(span) },
                to: TextLocus::from(citation.cites.from.clone()),
                to_last: (citation.cites.to != citation.cites.from).then(|| TextLocus::from(citation.cites.to.clone())),
                target_display: target_display(&citation.cites),
                votes: 0,
                provenance: PROVENANCE.to_string(),
            });
        }
    }
    graph.cross_refs.extend(rows);
    stats
}

/// Every citation in `text`, left to right, none overlapping. A range whose last verse comes before
/// its first is not a citation.
pub fn scan(text: &str) -> Vec<Citation> {
    GRAMMAR.pattern.captures_iter(text).filter_map(|found| citation(text, &found)).collect()
}

/// The citation as the canon spells a reference: `ROM.13.1` for one verse, `ROM.13.1-4` for a range.
pub fn target_display(cites: &BibleLocusRange) -> String {
    let first = VerseId { book: BookId(cites.from.unit.book), chapter: cites.from.unit.chapter, verse: cites.from.unit.verse };
    let reference = if cites.from == cites.to {
        ScriptureRef::Verse(first)
    } else {
        ScriptureRef::Passage { book: first.book, chapter: first.chapter, from_verse: first.verse, to_verse: cites.to.unit.verse }
    };
    reference.to_string()
}

/// Every name a citation may give a book: the canon's own names, then these abbreviations. A name
/// both lists give keeps the canon's book.
pub fn book_names() -> Vec<(&'static str, BookId)> {
    let mut names: Vec<(&'static str, BookId)> = Vec::new();
    let canon = BOOKS.iter().enumerate().map(|(index, book)| (book.name, BookId(index as u8)));
    for (name, book) in canon.chain(ABBREVIATIONS) {
        if !names.iter().any(|(listed, _)| *listed == name) {
            names.push((name, book));
        }
    }
    names
}

/// No bare "Cor", "Pet" or "Phil": each is ambiguous (a bare "Cor." is at least as often an elided
/// 2 Corinthians as 1; "Phil" is Philippians or Philemon), and an unlisted abbreviation is a silent
/// miss, never a guessed misattribution. Add one only after checking how the corpus uses it.
const ABBREVIATIONS: [(&str, BookId); 69] = [
    ("Gen", book("GEN")),
    ("Exod", book("EXO")),
    ("Exo", book("EXO")),
    ("Lev", book("LEV")),
    ("Num", book("NUM")),
    ("Deut", book("DEU")),
    ("Deu", book("DEU")),
    ("Josh", book("JOS")),
    ("Judg", book("JDG")),
    ("1 Sam", book("1SA")),
    ("2 Sam", book("2SA")),
    ("1 Kings", book("1KI")),
    ("1 Kgs", book("1KI")),
    ("2 Kings", book("2KI")),
    ("2 Kgs", book("2KI")),
    ("1 Chron", book("1CH")),
    ("1 Chr", book("1CH")),
    ("2 Chron", book("2CH")),
    ("2 Chr", book("2CH")),
    ("Neh", book("NEH")),
    ("Esth", book("EST")),
    ("Ps", book("PSA")),
    ("Pss", book("PSA")),
    ("Psalm", book("PSA")),
    ("Prov", book("PRO")),
    ("Eccl", book("ECC")),
    ("Isa", book("ISA")),
    ("Jer", book("JER")),
    ("Lam", book("LAM")),
    ("Ezek", book("EZK")),
    ("Eze", book("EZK")),
    ("Dan", book("DAN")),
    ("Hos", book("HOS")),
    ("Obad", book("OBA")),
    ("Mic", book("MIC")),
    ("Nah", book("NAM")),
    ("Hab", book("HAB")),
    ("Zeph", book("ZEP")),
    ("Hag", book("HAG")),
    ("Zech", book("ZEC")),
    ("Mal", book("MAL")),
    ("Matt", book("MAT")),
    ("Mat", book("MAT")),
    ("Mk", book("MRK")),
    ("Lk", book("LUK")),
    ("Rom", book("ROM")),
    ("1 Cor", book("1CO")),
    ("2 Cor", book("2CO")),
    ("Gal", book("GAL")),
    ("Eph", book("EPH")),
    ("Php", book("PHP")),
    ("Col", book("COL")),
    ("1 Thess", book("1TH")),
    ("1 Thes", book("1TH")),
    ("2 Thess", book("2TH")),
    ("2 Thes", book("2TH")),
    ("1 Tim", book("1TI")),
    ("2 Tim", book("2TI")),
    ("Tit", book("TIT")),
    ("Philem", book("PHM")),
    ("Heb", book("HEB")),
    ("Jas", book("JAS")),
    ("1 Pet", book("1PE")),
    ("2 Pet", book("2PE")),
    ("1 John", book("1JN")),
    ("2 John", book("2JN")),
    ("3 John", book("3JN")),
    ("Jude", book("JUD")),
    ("Rev", book("REV")),
];

const fn book(code: &str) -> BookId {
    BookId(position_of(code) as u8)
}

struct Grammar {
    pattern: Regex,
    names: Vec<(&'static str, BookId)>,
}

/// Longest name first: an alternation tries left to right, so "1 Corinthians" must be offered
/// before "1 Cor", or the shorter name would win and strand "inthians". Digits are ASCII, which is
/// what a chapter or verse number parses from. An "and following" must end its word, so the
/// citation still ends where a word ends; other letters after the verse stay outside it.
static GRAMMAR: LazyLock<Grammar> = LazyLock::new(|| {
    let names = book_names();
    let mut longest_first: Vec<&str> = names.iter().map(|(name, _)| *name).collect();
    longest_first.sort_by_key(|name| std::cmp::Reverse(name.len()));
    let alternation = longest_first.iter().map(|name| regex::escape(name)).collect::<Vec<_>>().join("|");
    let pattern = Regex::new(&format!(r"\b(?<book>{alternation})\.?\s+(?<chapter>[0-9]{{1,3}})[,:]\s*(?<verse>[0-9]{{1,3}})(?:-(?<last>[0-9]{{1,3}})|(?:ff?|sqq?)\b)?"))
        .expect("the citation grammar compiles");
    Grammar { pattern, names }
});

fn citation(text: &str, found: &Captures) -> Option<Citation> {
    let whole = found.get(0).expect("a match has its whole text");
    let name = &found["book"];
    let (_, book) = GRAMMAR.names.iter().find(|(listed, _)| *listed == name).expect("the pattern matches only the names it was built from");
    let number = |group: &str| found.name(group).map(|m| m.as_str().parse::<u16>().expect("one to three ASCII digits parse"));
    let chapter = number("chapter").expect("a citation names its chapter");
    let first = number("verse").expect("a citation names its verse");
    let last = number("last").unwrap_or(first);
    let at = |verse: u16| -> BibleLocus { Locus::whole(VerseRef { book: book.0, chapter, verse }) };
    let cites = LocusRange::new(at(first), at(last)).ok()?;
    let start = text[..whole.start()].chars().count();
    Some(Citation { chars: start..start + whole.as_str().chars().count(), cites })
}

#[cfg(test)]
mod tests {
    use atlas_core::canon::position_of;
    use atlas_graph_types::edge::CrossRef;
    use atlas_graph_types::graph::{Graph, ReadingSpine};
    use atlas_graph_types::text::{BibleLocus, TextLocus, TextRef, VerseRef};

    use super::*;
    use crate::concord_adapter::{self, CONCORD_CORPUS, CONCORD_TRANSLATION};
    use crate::kjv_adapter::{self, KjvVerse};
    use crate::tokens;

    const PARAGRAPH: ConcordRef = ConcordRef { part: 3, article: 20, paragraph: 9 };
    const CITED: [(&str, u16, u16); 5] = [("MAT", 5, 3), ("ROM", 13, 1), ("ROM", 13, 2), ("ROM", 13, 3), ("ROM", 13, 4)];

    #[test]
    fn a_paragraph_that_cites_a_verse_cites_it_from_the_words_of_the_citation() {
        // Act
        let cited = cite_in("As Christ says, Matt. 5:3.");
        // Assert
        assert_eq!(cited, (vec![citation(3, 5, verse("MAT", 5, 3), None, "MAT.5.3")], CitationStats { cited: 1, off_words: 0, no_such_verse: 0 }));
    }

    #[test]
    fn a_range_citation_cites_its_whole_range() {
        // Act
        let cited = cite_in("Rom. 13:1-4");
        // Assert
        assert_eq!(
            cited,
            (vec![citation(0, 2, verse("ROM", 13, 1), Some(verse("ROM", 13, 4)), "ROM.13.1-4")], CitationStats { cited: 1, off_words: 0, no_such_verse: 0 })
        );
    }

    #[test]
    fn a_citation_of_a_verse_the_bible_lacks_is_refused_and_counted() {
        // Act
        let cited = cite_in("Matt. 5:99");
        // Assert
        assert_eq!(cited, (vec![], CitationStats { cited: 0, off_words: 0, no_such_verse: 1 }));
    }

    #[test]
    fn a_range_that_runs_past_the_verses_the_bible_holds_is_refused_and_counted() {
        // Act
        let cited = cite_in("Rom. 13:1-99");
        // Assert
        assert_eq!(cited, (vec![], CitationStats { cited: 0, off_words: 0, no_such_verse: 1 }));
    }

    #[test]
    fn and_following_cites_the_stated_verse_from_the_whole_word_that_says_it() {
        // Act
        let cited = cite_in("Rom. 13:1ff.");
        // Assert
        assert_eq!(cited, (vec![citation(0, 2, verse("ROM", 13, 1), None, "ROM.13.1")], CitationStats { cited: 1, off_words: 0, no_such_verse: 0 }));
    }

    #[test]
    fn a_citation_that_ends_inside_a_word_is_refused_and_counted() {
        // Act
        let cited = cite_in("Rom. 13:1fold");
        // Assert
        assert_eq!(cited, (vec![], CitationStats { cited: 0, off_words: 1, no_such_verse: 0 }));
    }

    fn cite_in(paragraph: &str) -> (Vec<CrossRef>, CitationStats) {
        let mut graph = Graph::default();
        for (code, chapter, number) in CITED {
            let node = kjv_adapter::verse_node(&KjvVerse { book_index: position_of(code) as u8, chapter, verse: number, text: String::new() });
            graph.nodes.insert(node.id.clone(), node);
        }
        let unit = concord_adapter::paragraph_node(PARAGRAPH, paragraph);
        graph.reading.insert(CONCORD_CORPUS, ReadingSpine { order: vec![unit.id.clone()] });
        graph.nodes.insert(unit.id.clone(), unit);
        let stats = cite_scripture(&mut graph);
        (graph.cross_refs, stats)
    }

    fn verse(code: &str, chapter: u16, number: u16) -> TextLocus {
        TextLocus::from(BibleLocus::whole(VerseRef { book: position_of(code) as u8, chapter, verse: number }))
    }

    fn citation(first_word: u16, last_word: u16, to: TextLocus, to_last: Option<TextLocus>, target_display: &str) -> CrossRef {
        let words = tokens::span(CONCORD_TRANSLATION, first_word, last_word).expect("first <= last");
        CrossRef {
            from: TextLocus { at: TextRef::Concord(PARAGRAPH), span: Some(words) },
            to,
            to_last,
            target_display: target_display.to_string(),
            votes: 0,
            provenance: PROVENANCE.to_string(),
        }
    }
}
