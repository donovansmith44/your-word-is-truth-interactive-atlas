use atlas_graph_types::container::{BibleContainer, ConcordContainer, Container, ContainerLevel, SectionTitle};
use atlas_graph_types::passage::{Passage, PassageMark, ReadingOrder};
use atlas_graph_types::text::{BibleTag, VerseRef};
use proptest::prelude::*;

const TITLE: &str = "[A-Za-z ]{0,12}";

proptest! {
    #[test]
    fn a_bible_container_and_a_concord_container_share_the_levels_of_book_and_chapter(title in TITLE, section in proptest::option::of(TITLE)) {
        // Arrange
        let bible = [
            BibleContainer::Bible { title: title.clone() },
            BibleContainer::Book { title: title.clone() },
            BibleContainer::Chapter { title: title.clone() },
            BibleContainer::Passage(two_verses()),
        ];
        let concord = [
            ConcordContainer::BookOfConcord { title: title.clone(), description: title.clone() },
            ConcordContainer::Document { title: title.clone() },
            ConcordContainer::Article { title, section_title: section.map(SectionTitle) },
        ];

        // Act
        let levels = (bible.map(|container| levelled(&container)), concord.map(|container| levelled(&container)));

        // Assert
        prop_assert_eq!(
            levels,
            (
                [ContainerLevel::Corpus, ContainerLevel::Book, ContainerLevel::Chapter, ContainerLevel::Passage],
                [ContainerLevel::Corpus, ContainerLevel::Book, ContainerLevel::Chapter],
            )
        );
    }
}

fn levelled(container: &impl Container) -> ContainerLevel {
    container.level()
}

struct TwoVerses;

impl ReadingOrder<BibleTag> for TwoVerses {
    fn position(&self, unit: &VerseRef) -> Option<usize> {
        [first_verse(), second_verse()].iter().position(|verse| verse == unit)
    }
}

fn two_verses() -> Passage<BibleTag> {
    Passage::new(first_verse(), second_verse(), PassageMark::Plain, &TwoVerses).expect("two verses are a passage")
}

fn first_verse() -> VerseRef {
    VerseRef { book: 1, chapter: 1, verse: 1 }
}

fn second_verse() -> VerseRef {
    VerseRef { book: 1, chapter: 1, verse: 2 }
}
