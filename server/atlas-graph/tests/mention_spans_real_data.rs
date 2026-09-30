mod common;

use atlas_graph::mention_spans::MentionSpanStats;

const LOCATED: usize = 35_488;
const UNLOCATABLE: usize = 6_018;

#[test]
fn every_place_and_person_mention_is_located_on_its_words_or_counted() {
    // Arrange
    let sources = common::RawSources::read(common::OptionalCorpora { kretzmann: false, red_letter: false });
    // Act
    let (_, stats, ..) = sources.build_graph(&common::real_atlas().eras);
    // Assert
    assert_eq!(stats.mention_spans, MentionSpanStats { located: LOCATED, unlocatable: UNLOCATABLE });
}
