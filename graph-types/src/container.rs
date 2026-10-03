crate::vocabulary! {
    ContainerLevel {
        Corpus => "corpus",
        Book => "book",
        Chapter => "chapter",
    }
}

pub trait Container {
    fn level(&self) -> ContainerLevel;
    fn title(&self) -> &str;
}

#[derive(Clone, Debug, PartialEq)]
pub enum CorpusContainer {
    Bible(BibleContainer),
    Concord(ConcordContainer),
}

impl CorpusContainer {
    pub fn container(&self) -> &dyn Container {
        match self {
            CorpusContainer::Bible(container) => container,
            CorpusContainer::Concord(container) => container,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum BibleContainer {
    Bible { title: String },
    Book { title: String },
    Chapter { title: String },
}

#[derive(Clone, Debug, PartialEq)]
pub enum ConcordContainer {
    BookOfConcord { title: String, description: String },
    Document { title: String },
    Article { title: String, section_title: Option<String> },
}

impl Container for BibleContainer {
    fn level(&self) -> ContainerLevel {
        match self {
            BibleContainer::Bible { .. } => ContainerLevel::Corpus,
            BibleContainer::Book { .. } => ContainerLevel::Book,
            BibleContainer::Chapter { .. } => ContainerLevel::Chapter,
        }
    }

    fn title(&self) -> &str {
        match self {
            BibleContainer::Bible { title } | BibleContainer::Book { title } | BibleContainer::Chapter { title } => title,
        }
    }
}

impl Container for ConcordContainer {
    fn level(&self) -> ContainerLevel {
        match self {
            ConcordContainer::BookOfConcord { .. } => ContainerLevel::Corpus,
            ConcordContainer::Document { .. } => ContainerLevel::Book,
            ConcordContainer::Article { .. } => ContainerLevel::Chapter,
        }
    }

    fn title(&self) -> &str {
        match self {
            ConcordContainer::BookOfConcord { title, .. } | ConcordContainer::Document { title } | ConcordContainer::Article { title, .. } => title,
        }
    }
}
