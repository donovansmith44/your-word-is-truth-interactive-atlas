//! A frontier section shows when instances of it exist for the focus AND the focus kind has
//! not opted out of it; the matches below are exhaustive over both vocabularies, so a new
//! kind or capability cannot compile until every cell is an explicit decision.

/// Deliberately not the graph's own node vocabulary: that has no verse/passage/chapter/book
/// levels, and some foci here are synthesized views rather than nodes. `backing` below is
/// the type-checked join between the two.
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

/// Total and explicit, so "not a node today" and "never a node" cannot collapse into one
/// silent `None`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FocusBacking {
    /// A real node of this kind exists in the compiled artifact today.
    Node(crate::id::NodeKind),
    /// This focus will be backed by a node of this kind: the vocabulary has the kind, but no
    /// adapter emits one for this focus yet.
    NodeDesignate(crate::id::NodeKind),
    /// A parameterized query over the graph, not a node walk.
    ParameterizedView,
}

impl FocusKind {
    pub const fn backing(self) -> FocusBacking {
        use crate::id::NodeKind as G;
        use FocusBacking as B;
        match self {
            FocusKind::Verse | FocusKind::Passage | FocusKind::ConcordUnit => B::Node(G::TextUnit),
            FocusKind::Place => B::Node(G::Place),
            FocusKind::Person => B::Node(G::Person),
            FocusKind::Event => B::Node(G::Event),
            FocusKind::Catechism => B::Node(G::CatechismItem),
            FocusKind::CommentaryItem => B::Node(G::CommentaryItem),
            FocusKind::Chapter | FocusKind::Book => B::NodeDesignate(G::Container),
            FocusKind::Year => B::NodeDesignate(G::Anchor),
            FocusKind::Author | FocusKind::TimeAndPlace | FocusKind::PolityDelta => {
                B::ParameterizedView
            }
        }
    }

    /// `Some` only for a kind that exists in the artifact today: a designate deliberately
    /// does not surface, so a caller narrowing a focus can never walk a fictional cell.
    /// Match on `backing()` to tell "not a node" from "not a node yet".
    pub const fn graph_kind(self) -> Option<crate::id::NodeKind> {
        match self.backing() {
            FocusBacking::Node(k) => Some(k),
            FocusBacking::NodeDesignate(_) | FocusBacking::ParameterizedView => None,
        }
    }
}

/// A capability names the exact directed edge (or symmetric relation) it walks, so it can be
/// handed straight to the frontier walker. `target: None` means heterogeneous or not yet
/// narrowable -- never guessed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FrontierEdge {
    pub kind: crate::edge::EdgeKind,
    pub target: Option<crate::id::NodeKind>,
}

/// One per non-core frontier section family. Core sections are unconditional law and never
/// an opt-out decision, so they are not here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Capability {
    /// Not claimed by an event frontier: no event-level cross-reference data exists yet.
    CrossReferences,
    /// A two-hop: `Attests` forward to the event, then `Attests` inverse to its other
    /// witnesses. There is no parallel relation in the data.
    Parallels,
    /// Verse to event, forward: the same relation `Accounts` walks, from the opposite end.
    EventMembership,
    /// Today the same triple as `EventMembership`: one relation with two presentations,
    /// separated only by a payload discriminator the type system does not see.
    PassageMembership,
    /// Narrowed to `Person`: the underlying relation also targets places and people groups.
    Persons,
    /// One symmetric relation, claimed from both ends rather than given a second capability.
    CatechismSupport,
    /// Chain membership and timeline-neighbour pairing, from one fetch.
    Chronology,
    /// `DatedBy` and `LocatedAt`. The name matches `FocusKind::TimeAndPlace` deliberately:
    /// exploring place through this capability yields foci of that kind.
    TimeAndPlace,
    /// Event to verse, `Attests` inverse.
    Accounts,
    /// `LocatedAt` inverse, place to event: a real paginated edge walk, not focus body.
    EventsAt,
    /// `Mentions` inverse, person to text unit -- the paginated half; the card half is core.
    MentionsOf,
    /// `CommentsOn` inverse. Every real row in the artifact is verse-anchored, so verse is
    /// the one focus kind this is verified for.
    Commentary,
    /// `Contains` forward -- what this container contains.
    Members,
    /// Symmetric `Analogue` -- see `Graph::analogue` for what the relation admits. Kept apart
    /// from `Accounts` because conflating the two put a false parallel in front of a reader.
    Analogues,
    Kin,
    /// `Participates` forward, admitted only where the event is a real node. A person's years
    /// are payload on the card, not a capability.
    Participation,
}

impl Capability {
    pub const ALL: &'static [Capability] = &[
        Capability::CrossReferences,
        Capability::Parallels,
        Capability::EventMembership,
        Capability::PassageMembership,
        Capability::Persons,
        Capability::CatechismSupport,
        Capability::Chronology,
        Capability::TimeAndPlace,
        Capability::Accounts,
        Capability::EventsAt,
        Capability::MentionsOf,
        Capability::Commentary,
        Capability::Members,
        Capability::Analogues,
        Capability::Kin,
        Capability::Participation,
    ];

    /// Real directed edges, so a capability composes with the frontier walker directly.
    pub const fn edges(self) -> &'static [FrontierEdge] {
        use crate::edge::{Direction as D, EdgeKind as EK, RelationId as R, SymRelationId as S};
        use crate::id::NodeKind as G;
        match self {
            Capability::CrossReferences => {
                &[FrontierEdge { kind: EK::Directed(R::Cites, D::Forward), target: Some(G::TextUnit) }]
            }
            Capability::Parallels => &[
                FrontierEdge { kind: EK::Directed(R::Attests, D::Forward), target: Some(G::Event) },
                FrontierEdge { kind: EK::Directed(R::Attests, D::Inverse), target: Some(G::TextUnit) },
            ],
            Capability::EventMembership => {
                &[FrontierEdge { kind: EK::Directed(R::Attests, D::Forward), target: Some(G::Event) }]
            }
            Capability::PassageMembership => {
                &[FrontierEdge { kind: EK::Directed(R::Attests, D::Forward), target: Some(G::Event) }]
            }
            Capability::Persons => {
                &[FrontierEdge { kind: EK::Directed(R::Mentions, D::Forward), target: Some(G::Person) }]
            }
            Capability::CatechismSupport => {
                &[FrontierEdge { kind: EK::Symmetric(S::CatechismLink), target: Some(G::CatechismItem) }]
            }
            Capability::Chronology => &[
                // Chain membership, not a pairwise endpoint, so the target is not one kind.
                FrontierEdge { kind: EK::Directed(R::Succession, D::Forward), target: None },
                FrontierEdge { kind: EK::Symmetric(S::TemporalAdjacency), target: Some(G::Event) },
            ],
            Capability::TimeAndPlace => &[
                // A dating target is heterogeneous, so it cannot be narrowed to one kind.
                FrontierEdge { kind: EK::Directed(R::DatedBy, D::Forward), target: None },
                FrontierEdge { kind: EK::Directed(R::LocatedAt, D::Forward), target: Some(G::Place) },
            ],
            Capability::Accounts => {
                &[FrontierEdge { kind: EK::Directed(R::Attests, D::Inverse), target: Some(G::TextUnit) }]
            }
            Capability::EventsAt => {
                &[FrontierEdge { kind: EK::Directed(R::LocatedAt, D::Inverse), target: Some(G::Event) }]
            }
            Capability::MentionsOf => {
                &[FrontierEdge { kind: EK::Directed(R::Mentions, D::Inverse), target: Some(G::TextUnit) }]
            }
            Capability::Commentary => {
                &[FrontierEdge { kind: EK::Directed(R::CommentsOn, D::Inverse), target: Some(G::CommentaryItem) }]
            }
            Capability::Members => {
                &[FrontierEdge { kind: EK::Directed(R::Contains, D::Forward), target: Some(G::TextUnit) }]
            }
            Capability::Analogues => {
                &[FrontierEdge { kind: EK::Symmetric(S::Analogue), target: Some(G::Event) }]
            }
            Capability::Kin => &[
                FrontierEdge { kind: EK::Directed(R::ParentOf, D::Forward), target: Some(G::Person) },
                FrontierEdge { kind: EK::Directed(R::ParentOf, D::Inverse), target: Some(G::Person) },
                FrontierEdge { kind: EK::Symmetric(S::Spouses), target: Some(G::Person) },
                FrontierEdge { kind: EK::Symmetric(S::Brethren), target: Some(G::Person) },
            ],
            Capability::Participation => {
                &[FrontierEdge { kind: EK::Directed(R::Participates, D::Forward), target: Some(G::Event) }]
            }
        }
    }
}

/// The section families present on every frontier of their kind without exception. Named
/// exhaustively so that `allows() == false` cannot be ambiguous between "opted out" and
/// "core, and so not governed here".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CoreSection {
    /// The text presentation of a verse, chapter or book.
    ChapterCard,
    CatechismText,
    CatechismExplanation,
    CatechismWhereWritten,
    PlaceDescription,
    PlaceDates,
    PlaceBlurb,
    PolityDeltaEvent,
    PolityDeltaScriptures,
    PolityDeltaGrounding,
    /// The card half; the paginated mentions half is `Capability::MentionsOf`.
    PersonCard,
    CommentaryItemProse,
}

/// Either core and unconditional or a governed capability -- never both, never neither.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Section {
    Core(CoreSection),
    Capability(Capability),
}

/// The other half of the frontier rule, as a real three-valued type: `Empty` and
/// `Unavailable` are meant to render identically, and naming them apart is what makes
/// "instances exist" testable at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SectionOutcome {
    Content,
    Empty,
    Unavailable,
}

/// `false` means opted out for that kind, or that the capability's section is core for it.
/// Exhaustive over both enums, so the compiler holds the whole matrix; a cell is one line.
pub const fn allows(kind: FocusKind, cap: Capability) -> bool {
    use Capability as C;
    use FocusKind as K;
    match (kind, cap) {
        (K::Verse, C::CrossReferences | C::Parallels | C::EventMembership
            | C::PassageMembership | C::Persons | C::CatechismSupport | C::Commentary) => true,
        (K::Verse, C::Chronology | C::TimeAndPlace | C::Accounts
            | C::EventsAt | C::MentionsOf | C::Members | C::Analogues | C::Kin | C::Participation) => false,

        (K::Passage, C::CrossReferences | C::Parallels | C::Persons
            | C::CatechismSupport) => true,
        (K::Passage, C::EventMembership | C::PassageMembership
            | C::Chronology | C::TimeAndPlace | C::Accounts | C::Commentary
            | C::EventsAt | C::MentionsOf | C::Members | C::Analogues | C::Kin | C::Participation) => false,

        (K::Event, C::Chronology | C::TimeAndPlace | C::Accounts
            | C::Analogues | C::MentionsOf) => true,
        (K::Event, C::CrossReferences | C::Parallels | C::EventMembership
            | C::PassageMembership | C::Persons | C::CatechismSupport | C::Commentary
            | C::EventsAt | C::Members | C::Kin | C::Participation) => false,

        (K::Place, C::EventsAt) => true,
        (K::Place, C::CrossReferences | C::Parallels | C::EventMembership
            | C::PassageMembership | C::Persons | C::CatechismSupport | C::Chronology
            | C::TimeAndPlace | C::Accounts | C::Commentary | C::MentionsOf | C::Members
            | C::Analogues | C::Kin | C::Participation) => false,

        (K::Person, C::MentionsOf | C::Kin | C::Participation) => true,
        (K::Person, C::CrossReferences | C::Parallels | C::EventMembership
            | C::PassageMembership | C::Persons | C::CatechismSupport | C::Chronology
            | C::TimeAndPlace | C::Accounts | C::Commentary | C::EventsAt | C::Members
            | C::Analogues) => false,

        (K::Catechism, C::CatechismSupport) => true,
        (K::Catechism, C::CrossReferences | C::Parallels | C::EventMembership
            | C::PassageMembership | C::Persons | C::Chronology | C::TimeAndPlace
            | C::Accounts | C::Commentary | C::EventsAt | C::MentionsOf | C::Members
            | C::Analogues | C::Kin | C::Participation) => false,

        (K::Chapter | K::Book, C::Members | C::Chronology) => true,
        (K::Chapter | K::Book, C::CrossReferences | C::Parallels | C::EventMembership
            | C::PassageMembership | C::Persons | C::CatechismSupport | C::TimeAndPlace
            | C::Accounts | C::Commentary | C::EventsAt | C::MentionsOf
            | C::Analogues | C::Kin | C::Participation) => false,

        (K::ConcordUnit, C::CatechismSupport) => true,
        (K::ConcordUnit, C::CrossReferences | C::Parallels | C::EventMembership
            | C::PassageMembership | C::Persons | C::Chronology | C::TimeAndPlace
            | C::Accounts | C::Commentary | C::EventsAt | C::MentionsOf | C::Members
            | C::Analogues | C::Kin | C::Participation) => false,

        // The kinds that govern no capability yet: every cell false, stated once.
        (
            K::Author | K::Year | K::TimeAndPlace | K::PolityDelta
            | K::CommentaryItem,
            _,
        ) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_edges_are_never_empty() {
        for cap in Capability::ALL {
            assert!(!cap.edges().is_empty(), "capability {cap:?} has no edges behind it");
        }
    }

    #[test]
    fn focus_backing_is_pinned_per_kind() {
        use crate::id::NodeKind as G;
        use FocusBacking as B;
        let expected: &[(FocusKind, FocusBacking)] = &[
            (FocusKind::Verse, B::Node(G::TextUnit)),
            (FocusKind::Passage, B::Node(G::TextUnit)),
            (FocusKind::ConcordUnit, B::Node(G::TextUnit)),
            (FocusKind::Chapter, B::NodeDesignate(G::Container)),
            (FocusKind::Book, B::NodeDesignate(G::Container)),
            (FocusKind::Author, B::ParameterizedView),
            (FocusKind::Place, B::Node(G::Place)),
            (FocusKind::Person, B::Node(G::Person)),
            (FocusKind::Event, B::Node(G::Event)),
            (FocusKind::Catechism, B::Node(G::CatechismItem)),
            (FocusKind::Year, B::NodeDesignate(G::Anchor)),
            (FocusKind::TimeAndPlace, B::ParameterizedView),
            (FocusKind::PolityDelta, B::ParameterizedView),
            (FocusKind::CommentaryItem, B::Node(G::CommentaryItem)),
        ];
        assert_eq!(expected.len(), 14, "every FocusKind must be pinned exactly once -- totality check on the test itself");
        for (kind, want) in expected {
            assert_eq!(kind.backing(), *want, "{kind:?}'s backing() drifted from the pinned classification");
        }
    }

    #[test]
    fn graph_kind_is_none_for_designates_and_parameterized_views() {
        for kind in [FocusKind::Chapter, FocusKind::Book, FocusKind::Year, FocusKind::Author,
                     FocusKind::TimeAndPlace, FocusKind::PolityDelta] {
            assert_eq!(kind.graph_kind(), None, "{kind:?} must not claim a real graph node yet");
        }
    }

    #[test]
    fn event_has_no_cross_references() {
        assert!(!allows(FocusKind::Event, Capability::CrossReferences));
    }

    #[test]
    fn event_implements_chronology_time_and_place_accounts() {
        assert!(allows(FocusKind::Event, Capability::Chronology));
        assert!(allows(FocusKind::Event, Capability::TimeAndPlace));
        assert!(allows(FocusKind::Event, Capability::Accounts));
    }

    #[test]
    fn event_implements_analogues_and_mentions_of() {
        assert!(allows(FocusKind::Event, Capability::Analogues), "the owner-ratified Analogue relation is an Event frontier");
        assert!(allows(FocusKind::Event, Capability::MentionsOf), "L3: a mention-only event's frontier shows its mentions");
        for kind in [FocusKind::Verse, FocusKind::Passage, FocusKind::Place, FocusKind::Person,
                     FocusKind::Catechism, FocusKind::Chapter, FocusKind::Book] {
            assert!(!allows(kind, Capability::Analogues), "{kind:?} must not claim Analogues -- events are analogous to events");
        }
        for kind in [FocusKind::Verse, FocusKind::Passage, FocusKind::Place,
                     FocusKind::Catechism, FocusKind::Chapter, FocusKind::Book] {
            assert!(!allows(kind, Capability::MentionsOf), "{kind:?} must not claim MentionsOf");
        }
    }

    #[test]
    fn accounts_analogues_and_mentions_walk_three_different_edges() {
        use crate::edge::{Direction as D, EdgeKind as EK, RelationId as R, SymRelationId as S};
        assert_eq!(Capability::Accounts.edges()[0].kind, EK::Directed(R::Attests, D::Inverse));
        assert_eq!(Capability::Analogues.edges()[0].kind, EK::Symmetric(S::Analogue));
        assert_eq!(Capability::MentionsOf.edges()[0].kind, EK::Directed(R::Mentions, D::Inverse));
        assert_eq!(Capability::Analogues.edges()[0].target, Some(crate::id::NodeKind::Event));
    }

    #[test]
    fn verse_row_matches_the_owner_reviewed_matrix() {
        use Capability as C;
        for cap in [C::CrossReferences, C::Parallels, C::EventMembership,
                    C::PassageMembership, C::Persons, C::CatechismSupport, C::Commentary] {
            assert!(allows(FocusKind::Verse, cap), "Verse must allow {cap:?}");
        }
        for cap in [C::Chronology, C::TimeAndPlace, C::Accounts,
                    C::EventsAt, C::MentionsOf, C::Members, C::Analogues] {
            assert!(!allows(FocusKind::Verse, cap), "Verse must not allow {cap:?}");
        }
    }

    #[test]
    fn escapee_capabilities_land_on_their_owner_kind_only() {
        assert!(allows(FocusKind::Place, Capability::EventsAt));
        assert!(allows(FocusKind::Person, Capability::MentionsOf));
        assert!(allows(FocusKind::Catechism, Capability::CatechismSupport));
    }

    #[test]
    fn kin_and_participation_are_person_only_and_walk_the_d5_rows() {
        use crate::edge::{Direction as D, EdgeKind as EK, RelationId as R, SymRelationId as S};
        assert!(allows(FocusKind::Person, Capability::Kin));
        assert!(allows(FocusKind::Person, Capability::Participation));
        for kind in [FocusKind::Verse, FocusKind::Passage, FocusKind::ConcordUnit, FocusKind::Chapter, FocusKind::Book,
                     FocusKind::Author, FocusKind::Place, FocusKind::Event, FocusKind::Catechism, FocusKind::Year,
                     FocusKind::TimeAndPlace, FocusKind::PolityDelta, FocusKind::CommentaryItem] {
            assert!(!allows(kind, Capability::Kin), "{kind:?} must not claim Kin");
            assert!(!allows(kind, Capability::Participation), "{kind:?} must not claim Participation");
        }
        let kin: Vec<EK> = Capability::Kin.edges().iter().map(|e| e.kind).collect();
        assert_eq!(kin, vec![EK::Directed(R::ParentOf, D::Forward), EK::Directed(R::ParentOf, D::Inverse), EK::Symmetric(S::Spouses), EK::Symmetric(S::Brethren)]);
        assert!(Capability::Kin.edges().iter().all(|e| e.target == Some(crate::id::NodeKind::Person)));
        assert_eq!(Capability::Participation.edges()[0].kind, EK::Directed(R::Participates, D::Forward));
        assert_eq!(Capability::Participation.edges()[0].target, Some(crate::id::NodeKind::Event));
        for kind in [FocusKind::Verse, FocusKind::Event, FocusKind::Chapter, FocusKind::Book] {
            assert!(!allows(kind, Capability::EventsAt), "{kind:?} must not claim EventsAt");
        }
        for kind in [FocusKind::Verse, FocusKind::Chapter, FocusKind::Book] {
            assert!(!allows(kind, Capability::MentionsOf), "{kind:?} must not claim MentionsOf");
        }
    }

    #[test]
    fn chapter_and_book_get_members_and_chronology() {
        for kind in [FocusKind::Chapter, FocusKind::Book] {
            assert!(allows(kind, Capability::Members), "{kind:?} must allow Members");
            assert!(allows(kind, Capability::Chronology), "{kind:?} must allow Chronology");
            assert!(!allows(kind, Capability::EventsAt), "{kind:?} must not allow EventsAt");
        }
    }
}
