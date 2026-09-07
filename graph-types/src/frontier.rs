//! THE FRONTIER CONTRACT — the cross-language authority (owner-ordered in
//! Rust, 2026-09-07: "write it in rust and make sure it compiles before
//! implementing"). The C# client mirrors this module; a parity test holds
//! the mirror to it (the AQC scenario-count parity precedent).
//!
//! THE SMART-FRONTIER RULE (owner verbatim, spec §4e): "frontiers need to
//! be smart. I.e., they display instances of types of nodes on the
//! frontier iff both: 1) they exist for the focus node and 2) we have not
//! opted out of displaying that node type on the frontier of the focus
//! node."
//!
//!   display(section, focus) ⟺ exists(instances, focus)
//!                            ∧ allows(focus.kind, section.capability)
//!
//! `allows` below IS the opt-out registry. Rust buys us what no set-based
//! encoding can: the match is EXHAUSTIVE over both enums — adding a new
//! `FocusKind` (or a new `Capability`) refuses to compile until every cell
//! of the new row/column is an explicit decision. A cell edit is a
//! one-line change (the owner's standing acceptance bar).

/// THE focus-kind vocabulary — every EXPLORABLE kind the frontier serves.
/// NOT the graph's own `crate::id::NodeKind` (owner challenge 2026-09-07:
/// "we should just be reusing stuff that exists" — the answer: the graph
/// vocabulary has no Verse/Passage/Chapter/Book, which are LEVELS of
/// TextUnit/Container per id.rs's own doc, and Year/TimeAndPlace/
/// PolityDelta are client-synthesized views; the two vocabularies are
/// genuinely distinct, and the honest reuse is the type-checked BRIDGE
/// below, not a merged enum and not a colliding name).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FocusKind {
    Verse,
    Passage,
    Chapter,
    Book,
    Author,
    Place,
    Person,
    Event,
    Catechism,
    Year,
    TimeAndPlace,
    PolityDelta,
    CommentaryItem,
    ConcordUnit,
}

impl FocusKind {
    /// The type-checked bridge to the graph's own vocabulary
    /// (`crate::id::NodeKind`) — reuse, made explicit. `None` = a
    /// client-synthesized view with no single graph node kind behind it.
    pub const fn graph_kind(self) -> Option<crate::id::NodeKind> {
        use crate::id::NodeKind as G;
        Some(match self {
            FocusKind::Verse | FocusKind::Passage | FocusKind::ConcordUnit => G::TextUnit,
            FocusKind::Chapter | FocusKind::Book | FocusKind::Author => G::Container,
            FocusKind::Place => G::Place,
            FocusKind::Person => G::Person,
            FocusKind::Event => G::Event,
            FocusKind::Catechism => G::CatechismItem,
            FocusKind::CommentaryItem => G::CommentaryItem,
            FocusKind::Year | FocusKind::TimeAndPlace | FocusKind::PolityDelta => return None,
        })
    }
}

/// The segregated capability families (spec §4e) — one per non-core
/// frontier section family. Core (chrome, hatches, back/trail, the focus
/// body) is NOT here: core is unconditional law, never a capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Capability {
    CrossReferences,
    Parallels,
    EventMembership,
    PassageMembership,
    Persons,
    CatechismSupport,
    /// EVT-3: the refs-list under prior/following (accounts as refs).
    Chronology,
    /// EVT-3: explorable Time:/Place: rows; place exploration =
    /// f(location, event time) with the map-open side effect.
    TimeAndPlace,
    /// EVT-3: "events are keys that map to sets of Biblical accounts."
    Accounts,
}

/// THE OPT-OUT REGISTRY. `false` = opted out for that kind. Exhaustive
/// over both enums by construction — the compiler enforces the whole
/// matrix. One line per row; a cell edit is a one-line diff.
pub const fn allows(kind: FocusKind, cap: Capability) -> bool {
    use Capability as C;
    use FocusKind as K;
    match (kind, cap) {
        // Verse — the richest frontier.
        (K::Verse, C::CrossReferences | C::Parallels | C::EventMembership
            | C::PassageMembership | C::Persons | C::CatechismSupport) => true,
        (K::Verse, C::Chronology | C::TimeAndPlace | C::Accounts) => false,

        // Passage — verse-family, minus the verse-only memberships.
        (K::Passage, C::CrossReferences | C::Parallels | C::Persons
            | C::CatechismSupport) => true,
        (K::Passage, C::EventMembership | C::PassageMembership
            | C::Chronology | C::TimeAndPlace | C::Accounts) => false,

        // Event — the owner's calibration row: NO cross references until
        // event-level xref data exists ("we are not yet at the point of
        // being able ot provide lots of cross references for events").
        (K::Event, C::Chronology | C::TimeAndPlace | C::Accounts) => true,
        (K::Event, C::CrossReferences | C::Parallels | C::EventMembership
            | C::PassageMembership | C::Persons | C::CatechismSupport) => false,

        // Body-only kinds: their sections are core focus-presentation;
        // every capability is opted out until real data earns a cell.
        (
            K::Chapter | K::Book | K::Author | K::Place | K::Person
            | K::Catechism | K::Year | K::TimeAndPlace | K::PolityDelta
            | K::CommentaryItem | K::ConcordUnit,
            _,
        ) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The owner's calibration row, pinned: an event frontier does not
    /// implement cross references (until event-level data exists — the
    /// day it does, this test is UPDATED alongside the one-line cell).
    #[test]
    fn event_has_no_cross_references() {
        assert!(!allows(FocusKind::Event, Capability::CrossReferences));
    }

    /// EVT-3's three event capabilities, pinned.
    #[test]
    fn event_implements_chronology_time_and_place_accounts() {
        assert!(allows(FocusKind::Event, Capability::Chronology));
        assert!(allows(FocusKind::Event, Capability::TimeAndPlace));
        assert!(allows(FocusKind::Event, Capability::Accounts));
    }

    /// Verse always carries its six; never the event-family three.
    #[test]
    fn verse_row_matches_the_owner_reviewed_matrix() {
        use Capability as C;
        for cap in [C::CrossReferences, C::Parallels, C::EventMembership,
                    C::PassageMembership, C::Persons, C::CatechismSupport] {
            assert!(allows(FocusKind::Verse, cap));
        }
        for cap in [C::Chronology, C::TimeAndPlace, C::Accounts] {
            assert!(!allows(FocusKind::Verse, cap));
        }
    }
}
