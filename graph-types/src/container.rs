use crate::passage::Passage;
use crate::text::{BibleTag, ConcordTag, Corpus};

crate::vocabulary! {
    ContainerLevel {
        Corpus => "corpus",
        Book => "book",
        Chapter => "chapter",
        Passage => "passage",
    }
}

pub trait Container {
    type Corpus: Corpus;

    fn level(&self) -> ContainerLevel;
}

pub enum BibleContainer {
    Bible { title: String },
    Book { title: String },
    Chapter { title: String },
    Passage(Passage<BibleTag>),
}

pub struct SectionTitle(pub String);

pub enum ConcordContainer {
    BookOfConcord { title: String, description: String },
    Document { title: String },
    Article { title: String, section_title: Option<SectionTitle> },
}

impl Container for BibleContainer {
    type Corpus = BibleTag;

    fn level(&self) -> ContainerLevel {
        match self {
            BibleContainer::Bible { .. } => ContainerLevel::Corpus,
            BibleContainer::Book { .. } => ContainerLevel::Book,
            BibleContainer::Chapter { .. } => ContainerLevel::Chapter,
            BibleContainer::Passage(_) => ContainerLevel::Passage,
        }
    }
}

impl Container for ConcordContainer {
    type Corpus = ConcordTag;

    fn level(&self) -> ContainerLevel {
        match self {
            ConcordContainer::BookOfConcord { .. } => ContainerLevel::Corpus,
            ConcordContainer::Document { .. } => ContainerLevel::Book,
            ConcordContainer::Article { .. } => ContainerLevel::Chapter,
        }
    }
}
