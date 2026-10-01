mod common;

use atlas_graph::citations::CitationStats;

const OFF_WORDS: usize = 0;
const NO_SUCH_VERSE: usize = 0;

#[test]
fn every_citation_of_scripture_in_the_book_of_concord_cites_from_its_words_or_is_counted() {
    // Arrange
    let sources = common::RawSources::read(common::OptionalCorpora { kretzmann: false, red_letter: false });
    // Act
    let (_, stats, ..) = sources.build_graph(&common::real_atlas().eras);
    // Assert
    assert_eq!(stats.concord_citations, CitationStats { cited: common::CONCORD_CITATIONS, off_words: OFF_WORDS, no_such_verse: NO_SUCH_VERSE });
}
