//! Presentation laws travel as server data, so a stylesheet can never re-decide one.

use crate::edge::EdgeKind;
use crate::graph::Graph;
use crate::id::PositionKind;
use crate::node::Card;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Surface {
    Reader,
    Map,
    Popover,
    Timeline,
}

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Renderer {
    EntryList,
    TextFlow,
    MapPins,
    TimelineRows,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SectionStyle {
    Standard,
    Quiet,
    SuperscriptMarker,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SectionOrder {
    VotesRanked,
    Chain,
    Canonical,
    ResolvedDate,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SectionSpec {
    pub kind: EdgeKind,
    pub renderer: Renderer,
    pub style: SectionStyle,
    /// Clamp; hidden remainder MUST be signaled with the true count.
    pub initial: u8,
    pub order: SectionOrder,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct FrontierPresentation {
    pub sections: Vec<SectionSpec>,
}

/// Which position kinds can take focus, per surface. "What can I
pub fn focusable(_surface: Surface, _kind: PositionKind) -> bool {
    true
}

/// How a focus of a given kind displays its frontier, per surface.
pub fn display(_surface: Surface, _kind: PositionKind) -> FrontierPresentation {
    FrontierPresentation::default()
}

/// A rendered form — the closed vocabulary the client knows how to draw.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Presentation {
    Text(String),
    Labeled { label: String, detail: String },
    PinLabel(String),
}

/// Form policy: one implementation per (kind, context) — a thing cannot
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
