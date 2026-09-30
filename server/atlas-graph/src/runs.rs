//! An account's verses as a reader meets them: runs of verses that read on without a
//! break, a chapter running on into the next when it was read to its last verse.

use atlas_core::data::Canon;
use atlas_core::refs::BookId;
use atlas_graph_types::text::{BibleLocusRange, VerseRef};

const FIRST_VERSE: u16 = 1;

/// Every range the account attests, in any order, joined into the fewest runs that
/// cover them. A range with a word span at its joining end never joins across it:
/// part of a verse does not read on into the next.
pub fn coalesce(ranges: &[BibleLocusRange], canon: &Canon) -> Vec<BibleLocusRange> {
    let mut ordered = ranges.to_vec();
    ordered.sort();
    ordered.dedup();
    let mut runs: Vec<BibleLocusRange> = Vec::new();
    for range in ordered {
        match runs.last_mut() {
            Some(run) if reads_on(run, &range, canon) => {
                if range.to > run.to {
                    run.to = range.to;
                }
            }
            _ => runs.push(range),
        }
    }
    runs
}

fn reads_on(run: &BibleLocusRange, next: &BibleLocusRange, canon: &Canon) -> bool {
    run.to.span.is_none() && next.from.span.is_none() && (next.from.unit <= run.to.unit || follows(&run.to.unit, &next.from.unit, canon))
}

fn follows(last: &VerseRef, next: &VerseRef, canon: &Canon) -> bool {
    let in_the_same_chapter = next.book == last.book && next.chapter == last.chapter && next.verse == last.verse + 1;
    let opening_the_next_chapter = next.book == last.book
        && next.chapter == last.chapter + 1
        && next.verse == FIRST_VERSE
        && canon.verses_in(BookId(last.book), last.chapter) == Some(last.verse);
    in_the_same_chapter || opening_the_next_chapter
}

#[cfg(test)]
mod tests {
    use super::*;
    use atlas_graph_types::text::{Locus, TokenSpan, TranslationId};

    const GENESIS: u8 = 0;
    const FIRST_WORD: u16 = 0;
    const IN_THE_BEGINNING_ENDS: u16 = 2;

    #[test]
    fn part_of_a_verse_does_not_run_on_into_the_verse_beside_it() {
        // Arrange
        let ranges = [part_of(genesis(1, 1)), whole(genesis(1, 2)), part_of(genesis(1, 3))];
        // Act
        let runs = coalesce(&ranges, &Canon::default());
        // Assert
        assert_eq!(runs, ranges.to_vec());
    }

    fn part_of(verse: VerseRef) -> BibleLocusRange {
        let words = Locus { unit: verse, span: Some(TokenSpan { layer: TranslationId(crate::kjv_adapter::KJV_TRANSLATION.to_string()), start: FIRST_WORD, end: IN_THE_BEGINNING_ENDS }) };
        BibleLocusRange { from: words.clone(), to: words }
    }

    fn whole(verse: VerseRef) -> BibleLocusRange {
        BibleLocusRange { from: Locus::whole(verse.clone()), to: Locus::whole(verse) }
    }

    fn genesis(chapter: u16, verse: u16) -> VerseRef {
        VerseRef { book: GENESIS, chapter, verse }
    }
}
