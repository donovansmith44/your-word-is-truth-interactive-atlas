//! Presentation laws travel as server data, so a stylesheet can never re-decide one.

use crate::graph::Graph;
use crate::node::Card;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PresentationContext {
    Card,
    Entry,
    Inline,
    Heading,
    Pin,
    CitationRow,
    Marker,
}

/// A rendered form — the closed vocabulary the client knows how to draw.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Presentation {
    Text(String),
    Labeled { label: String, detail: String },
    PinLabel(String),
}

/// One implementation per (kind, context): a thing cannot wear two faces in one context.
pub trait Presentable {
    fn present(&self, ctx: PresentationContext, g: &Graph) -> Presentation;
}

impl Presentable for Card {
    fn present(&self, ctx: PresentationContext, _g: &Graph) -> Presentation {
        match ctx {
            PresentationContext::Card | PresentationContext::Entry => Presentation::Labeled {
                label: self.label.clone(),
                detail: self.provenance.clone(),
            },
            PresentationContext::Pin => Presentation::PinLabel(self.label.clone()),
            _ => Presentation::Text(self.label.clone()),
        }
    }
}
