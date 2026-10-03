use atlas_graph_types::container::{BibleContainer, ConcordContainer, Container, ContainerLevel};
use proptest::prelude::*;

const TITLE: &str = "[A-Za-z ]{0,12}";

proptest! {
    #[test]
    fn a_bible_container_and_a_concord_container_share_the_levels_of_book_and_chapter_and_keep_their_titles(title in TITLE, section in proptest::option::of(TITLE)) {
        // Arrange
        let bible = [BibleContainer::Bible { title: title.clone() }, BibleContainer::Book { title: title.clone() }, BibleContainer::Chapter { title: title.clone() }];
        let concord = [
            ConcordContainer::BookOfConcord { title: title.clone(), description: String::new() },
            ConcordContainer::Document { title: title.clone() },
            ConcordContainer::Article { title: title.clone(), section_title: section },
        ];

        // Act
        let read = (bible.map(|container| described(&container)), concord.map(|container| described(&container)));

        // Assert
        prop_assert_eq!(
            read,
            (
                [(ContainerLevel::Corpus, title.clone()), (ContainerLevel::Book, title.clone()), (ContainerLevel::Chapter, title.clone())],
                [(ContainerLevel::Corpus, title.clone()), (ContainerLevel::Book, title.clone()), (ContainerLevel::Chapter, title)],
            )
        );
    }
}

fn described(container: &impl Container) -> (ContainerLevel, String) {
    (container.level(), container.title().to_string())
}
