//! DB-2a golden vectors for the ROW encodings (all 21 families).
//!
//! The sibling `canon_real_data.rs` (in `atlas-graph`) proves the encoders
//! total over the real committed corpus; this file does the other half --
//! it PINS the bytes. The byte strings asserted here are the law, not a
//! description: a row's spelling is what its edge id and its section's
//! logical hash are taken over, so a silent change to one is a silent
//! change to every id derived from it.
//!
//! `std` only -- `graph-types` is a zero-dependency crate and its tests
//! keep that promise.
//!
//! FINAL REVIEW item 14 (parked minor M2-1): all 21 families now carry a
//! one-line byte golden. Seven did before; the other fourteen were
//! round-trip only, which proves the encoder and the decoder agree with
//! EACH OTHER but pins no byte. Each of the fourteen was generated from
//! the encoder as it stood at commit f6722f7, BEFORE the row `to_value`s
//! were rewritten to destructure -- so the goldens staying green IS the
//! proof that the destructuring changed nothing.

use std::collections::BTreeSet;

use atlas_graph_types::canon::{encode_row_in_family, Canon, RowFamily, Value, ROOT};
use atlas_graph_types::chrono::{DatePlacement, DatedBy, Duration, PlacementBasis};
use atlas_graph_types::edge::{
    Analogue, Attests, CanonSuccession, CatechismLink, CommentsOn, Confesses, ContainerContent,
    Contains, Corresponds, CrossRef, Fulfills, Ground, Justification, LocatedAt, MentionedEntity,
    Mentions, NamedAfter, Namesake, Occurs, Quotes, SpokenAt, SpokenBy, Succession, TemporalAdjacency,
    Typology,
};
use atlas_graph_types::id::{
    AnchorId, CatechismItemId, CommentaryItemId, ContainerNodeId, EraId, EventId, LexiconEntryId, NarrativeId,
    PeopleGroupId, PersonId, PlaceId, PolityId, SourceId,
};
use atlas_graph_types::text::{
    BibleLocus, BibleLocusRange, BibleTag, ConcordLocus, ConcordRef, ConcordTag, Locus, LocusRange,
    LocusSet, TextLocus, TextRef, TokenSpan, TranslationId, VerseRef,
};

// ------------------------------------------------------------- tiny builders

fn vr(book: u8, chapter: u16, verse: u16) -> VerseRef {
    VerseRef { book, chapter, verse }
}

fn bl(book: u8, chapter: u16, verse: u16) -> BibleLocus {
    Locus::whole(vr(book, chapter, verse))
}

fn blr(from: (u8, u16, u16), to: (u8, u16, u16)) -> BibleLocusRange {
    LocusRange::new(bl(from.0, from.1, from.2), bl(to.0, to.1, to.2))
        .expect("test range must be ordered")
}

fn tl(book: u8, chapter: u16, verse: u16) -> TextLocus {
    TextLocus { at: TextRef::Bible(vr(book, chapter, verse)), span: None }
}

fn cl(part: u8, article: u16, paragraph: u16) -> ConcordLocus {
    Locus::whole(ConcordRef { part, article, paragraph })
}

fn span(start: u16, end: u16) -> TokenSpan {
    TokenSpan::new(TranslationId("kjv".into()), start, end).expect("test span must be ordered")
}

/// A justification exercising all three `Ground` variants and the prose.
fn full_justification() -> Justification {
    let grounds: BTreeSet<Ground> = [
        Ground::Scripture(blr((40, 3, 13), (40, 3, 17))),
        Ground::Anchor(AnchorId::new("ussher-4004bc")),
        Ground::Source(SourceId::new("openbible-geo")),
    ]
    .into_iter()
    .collect();
    Justification { text: Some("Jordan, at Bethabara".into()), grounds }
}

// ---------------------------------------------------------------- the laws

/// One family's law: the bytes are a fixed point, the decode gives back
/// the same content, and the family wrapper is exactly the row's bytes
/// inside `{"family":…,"row":…}`.
fn round_trip<T: Canon + std::fmt::Debug>(row: &T, family: RowFamily) -> String {
    let bytes = row.encode();
    let text = String::from_utf8(bytes.clone()).expect("canonical bytes are UTF-8");
    let back = T::decode(&bytes)
        .unwrap_or_else(|e| panic!("{} failed to decode its own bytes: {e}", family.name()));
    assert_eq!(back.encode(), bytes, "{} is not a byte fixed point", family.name());
    assert_eq!(format!("{back:?}"), format!("{row:?}"), "{} lost content", family.name());
    assert_eq!(
        String::from_utf8(encode_row_in_family(family, row.to_value())).unwrap(),
        format!("{{\"family\":\"{}\",\"row\":{text}}}", family.name()),
        "{} must carry its family outside the row",
        family.name()
    );
    text
}

/// One family's BYTE GOLDEN, on top of the round-trip law: the encoder's
/// output IS this exact byte string, and this byte string decodes BACK to
/// the row.
///
/// The second half is the half that matters. `round_trip` decodes what
/// the encoder just produced, so encoder and decoder could drift together
/// and still agree; the golden is written down here, independent of both,
/// so `T::decode(golden)` is a decode of bytes NOTHING in this process
/// generated. Once DB-2b mints ids from these bytes, a silent change to
/// either direction is a silent change to every id derived from it --
/// which is why all 21 families are pinned, not just the seven that were.
///
/// What makes it fail: any change to a row's `to_value` key set, key
/// order, value spelling or nesting; any change to the shared parts those
/// rows embed (`Justification`, `Locus`, `LocusRange`, `TokenSpan`,
/// `TextLocus`, `Duration`, `DatePlacement`); and, for the decode half, a
/// member the decoder no longer accepts or a validating constructor that
/// no longer admits the specimen.
fn golden_row<T: Canon + std::fmt::Debug>(row: &T, family: RowFamily, golden: &str) {
    assert_eq!(round_trip(row, family), golden, "{} golden bytes moved", family.name());
    let back = T::decode(golden.as_bytes())
        .unwrap_or_else(|e| panic!("{} golden bytes failed to decode: {e}", family.name()));
    assert_eq!(
        format!("{back:?}"),
        format!("{row:?}"),
        "{} golden bytes must decode back to the row",
        family.name()
    );
}

#[test]
fn the_row_family_manifest_is_closed_and_ordinal_indexed() {
    assert_eq!(RowFamily::ALL.len(), 22, "spec 5 names 21 row tables + LEX-1's occurs (spec 5.7)");
    let names: Vec<&'static str> = RowFamily::ALL.iter().map(|f| f.name()).collect();
    let unique = {
        let mut n = names.clone();
        n.sort_unstable();
        n.dedup();
        n.len()
    };
    assert_eq!(unique, 22, "table names must be distinct: {names:?}");
    assert_eq!(
        names,
        vec![
            "contains_bible",
            "contains_concord",
            "attests",
            "succession",
            "canon_succession",
            "dated_by",
            "located_at",
            "fulfills",
            "typology",
            "named_after",
            "catechism",
            "comments_on",
            "spoken_by",
            "spoken_at",
            "mentions",
            "cross_refs",
            "quotes",
            "confesses",
            "corresponds_bible",
            "temporal_adjacency",
            "analogue",
            "occurs",
        ],
        "the table names and their ORDER are the spec's own (spec 5, then 5.7); the order IS the ordinal"
    );
    for (i, f) in RowFamily::ALL.iter().enumerate() {
        assert_eq!(f.ordinal() as usize, i, "{} sits at ordinal {i}", f.name());
        assert_eq!(RowFamily::from_ordinal(f.ordinal()), Some(*f));
    }
    assert_eq!(RowFamily::from_ordinal(22), None, "the enum is closed at 22");
}

#[test]
fn located_at_row_golden_bytes() {
    let row = LocatedAt {
        event: EventId::new("jesus-baptized"),
        place: PlaceId::new("jordan-river"),
        provenance: "curated/events".into(),
        justification: full_justification(),
    };
    assert_eq!(
        round_trip(&row, RowFamily::LocatedAt),
        r#"{"event":"Event:jesus-baptized","justification":{"grounds":[{"Scripture":{"from":{"span":null,"unit":{"book":40,"chapter":3,"verse":13}},"to":{"span":null,"unit":{"book":40,"chapter":3,"verse":17}}}},{"Anchor":"Anchor:ussher-4004bc"},{"Source":"Source:openbible-geo"}],"text":"Jordan, at Bethabara"},"place":"Place:jordan-river","provenance":"curated/events"}"#
    );
    // The hashed form (spec 3.4): the family rides OUTSIDE the row, so
    // two families that share a row shape can never share an id.
    assert!(String::from_utf8(encode_row_in_family(RowFamily::LocatedAt, row.to_value()))
        .unwrap()
        .starts_with(r#"{"family":"located_at","row":{"event":"#));
}

#[test]
fn every_row_family_round_trips_with_hand_built_data() {
    let mut covered: Vec<RowFamily> = Vec::new();
    macro_rules! law {
        ($row:expr, $family:expr, $golden:expr) => {{
            let f = $family;
            covered.push(f);
            golden_row(&$row, f, $golden)
        }};
    }

    // 1. contains_bible -- the flat-loci content, a non-empty set.
    law!(
        Contains::<BibleTag> {
            container: ContainerNodeId::new("bible/GEN.1"),
            content: ContainerContent::Loci(LocusSet(
                [bl(1, 1, 1), bl(1, 1, 2)].into_iter().collect()
            )),
            provenance: "kjv".into(),
            justification: Justification::default(),
        },
        RowFamily::ContainsBible,
        r#"{"container":"Container:bible/GEN.1","content":{"Loci":[{"span":null,"unit":{"book":1,"chapter":1,"verse":1}},{"span":null,"unit":{"book":1,"chapter":1,"verse":2}}]},"justification":{"grounds":[],"text":null},"provenance":"kjv"}"#
    );

    // 2. contains_concord -- the recursive child-container content.
    law!(
        Contains::<ConcordTag> {
            container: ContainerNodeId::new("concord/ac"),
            content: ContainerContent::Container(ContainerNodeId::new("concord/ac.1")),
            provenance: "concord".into(),
            justification: Justification::default(),
        },
        RowFamily::ContainsConcord,
        r#"{"container":"Container:concord/ac","content":{"Container":"Container:concord/ac.1"},"justification":{"grounds":[],"text":null},"provenance":"concord"}"#
    );

    // 3. attests
    law!(
        Attests {
            event: EventId::new("jesus-baptized"),
            attestation: blr((40, 3, 13), (40, 3, 17)),
            provenance: "curated/events".into(),
            justification: full_justification(),
        },
        RowFamily::Attests,
        r#"{"attestation":{"from":{"span":null,"unit":{"book":40,"chapter":3,"verse":13}},"to":{"span":null,"unit":{"book":40,"chapter":3,"verse":17}}},"event":"Event:jesus-baptized","justification":{"grounds":[{"Scripture":{"from":{"span":null,"unit":{"book":40,"chapter":3,"verse":13}},"to":{"span":null,"unit":{"book":40,"chapter":3,"verse":17}}}},{"Anchor":"Anchor:ussher-4004bc"},{"Source":"Source:openbible-geo"}],"text":"Jordan, at Bethabara"},"provenance":"curated/events"}"#
    );

    // 4. succession -- the chain goes through the validating constructor.
    law!(
        Succession::new(
            NarrativeId::new("life-of-christ"),
            vec![EventId::new("nativity"), EventId::new("jesus-baptized")],
            "curated/narratives".into(),
            Justification::default(),
        )
        .expect("a distinct, non-empty chain"),
        RowFamily::Succession,
        r#"{"chain":["Event:nativity","Event:jesus-baptized"],"justification":{"grounds":[],"text":null},"narrative":"Narrative:life-of-christ","provenance":"curated/narratives"}"#
    );

    // 5. canon_succession
    law!(
        CanonSuccession {
            prior: ContainerNodeId::new("bible/GEN.50"),
            next: ContainerNodeId::new("bible/EXO.1"),
            provenance: "canon".into(),
            justification: Justification::default(),
        },
        RowFamily::CanonSuccession,
        r#"{"justification":{"grounds":[],"text":null},"next":"Container:bible/EXO.1","prior":"Container:bible/GEN.50","provenance":"canon"}"#
    );

    // 6. dated_by
    law!(
        DatedBy {
            event: EventId::new("exodus"),
            placement: DatePlacement::AnchorBinding {
                anchor: AnchorId::new("abraham-called"),
                offset: Duration::years(430),
            },
            basis: PlacementBasis::Textual,
            justification: Justification::default(),
            provenance: "ussher".into(),
        },
        RowFamily::DatedBy,
        r#"{"basis":{"Textual":null},"event":"Event:exodus","justification":{"grounds":[],"text":null},"placement":{"AnchorBinding":{"anchor":"Anchor:abraham-called","offset":{"days":0,"months":0,"years":430}}},"provenance":"ussher"}"#
    );

    // 7. located_at (its own golden above)
    law!(
        LocatedAt {
            event: EventId::new("jesus-baptized"),
            place: PlaceId::new("jordan-river"),
            provenance: "curated/events".into(),
            justification: full_justification(),
        },
        RowFamily::LocatedAt,
        r#"{"event":"Event:jesus-baptized","justification":{"grounds":[{"Scripture":{"from":{"span":null,"unit":{"book":40,"chapter":3,"verse":13}},"to":{"span":null,"unit":{"book":40,"chapter":3,"verse":17}}}},{"Anchor":"Anchor:ussher-4004bc"},{"Source":"Source:openbible-geo"}],"text":"Jordan, at Bethabara"},"place":"Place:jordan-river","provenance":"curated/events"}"#
    );

    // 8. fulfills
    law!(
        Fulfills {
            prophecy: blr((23, 7, 14), (23, 7, 14)),
            fulfillment: blr((40, 1, 22), (40, 1, 23)),
            provenance: "curated/fulfillment".into(),
            justification: full_justification(),
        },
        RowFamily::Fulfills,
        r#"{"fulfillment":{"from":{"span":null,"unit":{"book":40,"chapter":1,"verse":22}},"to":{"span":null,"unit":{"book":40,"chapter":1,"verse":23}}},"justification":{"grounds":[{"Scripture":{"from":{"span":null,"unit":{"book":40,"chapter":3,"verse":13}},"to":{"span":null,"unit":{"book":40,"chapter":3,"verse":17}}}},{"Anchor":"Anchor:ussher-4004bc"},{"Source":"Source:openbible-geo"}],"text":"Jordan, at Bethabara"},"prophecy":{"from":{"span":null,"unit":{"book":23,"chapter":7,"verse":14}},"to":{"span":null,"unit":{"book":23,"chapter":7,"verse":14}}},"provenance":"curated/fulfillment"}"#
    );

    // 9. typology -- `note` present.
    law!(
        Typology {
            type_passage: blr((4, 21, 8), (4, 21, 9)),
            antitype_passage: blr((43, 3, 14), (43, 3, 14)),
            note: Some("the brasen serpent".into()),
            provenance: "curated/typology".into(),
            justification: Justification::default(),
        },
        RowFamily::Typology,
        r#"{"antitype_passage":{"from":{"span":null,"unit":{"book":43,"chapter":3,"verse":14}},"to":{"span":null,"unit":{"book":43,"chapter":3,"verse":14}}},"justification":{"grounds":[],"text":null},"note":"the brasen serpent","provenance":"curated/typology","type_passage":{"from":{"span":null,"unit":{"book":4,"chapter":21,"verse":8}},"to":{"span":null,"unit":{"book":4,"chapter":21,"verse":9}}}}"#
    );

    // 10. named_after
    law!(
        NamedAfter {
            namesake: Namesake::PeopleGroup(PeopleGroupId::new("tribe-of-judah")),
            eponym: PersonId::new("judah"),
            provenance: "curated/peoples".into(),
            justification: Justification::default(),
        },
        RowFamily::NamedAfter,
        r#"{"eponym":"Person:judah","justification":{"grounds":[],"text":null},"namesake":{"PeopleGroup":"PeopleGroup:tribe-of-judah"},"provenance":"curated/peoples"}"#
    );

    // 11. catechism -- a Concord-side TextLocus.
    law!(
        CatechismLink {
            locus: TextLocus { at: TextRef::Concord(ConcordRef { part: 1, article: 2, paragraph: 3 }), span: None },
            item: CatechismItemId::new("sc/1st-commandment"),
            provenance: "small-catechism".into(),
            justification: Justification::default(),
        },
        RowFamily::Catechism,
        r#"{"item":"CatechismItem:sc/1st-commandment","justification":{"grounds":[],"text":null},"locus":{"at":{"Concord":{"article":2,"paragraph":3,"part":1}},"span":null},"provenance":"small-catechism"}"#
    );

    // 12. comments_on
    law!(
        CommentsOn {
            item: CommentaryItemId::new("kretzmann/JHN.3.16"),
            on: blr((43, 3, 16), (43, 3, 16)),
            provenance: "kretzmann".into(),
            justification: Justification::default(),
        },
        RowFamily::CommentsOn,
        r#"{"item":"CommentaryItem:kretzmann/JHN.3.16","justification":{"grounds":[],"text":null},"on":{"from":{"span":null,"unit":{"book":43,"chapter":3,"verse":16}},"to":{"span":null,"unit":{"book":43,"chapter":3,"verse":16}}},"provenance":"kretzmann"}"#
    );

    // 13. spoken_by
    law!(
        SpokenBy {
            locus: blr((43, 3, 16), (43, 3, 21)),
            speaker: PersonId::new("jesus"),
            provenance: "red-letter".into(),
            justification: Justification::default(),
        },
        RowFamily::SpokenBy,
        r#"{"justification":{"grounds":[],"text":null},"locus":{"from":{"span":null,"unit":{"book":43,"chapter":3,"verse":16}},"to":{"span":null,"unit":{"book":43,"chapter":3,"verse":21}}},"provenance":"red-letter","speaker":"Person:jesus"}"#
    );

    // 14. spoken_at
    law!(
        SpokenAt {
            locus: blr((43, 3, 16), (43, 3, 21)),
            place: PlaceId::new("jerusalem"),
            provenance: "red-letter".into(),
            justification: Justification::default(),
        },
        RowFamily::SpokenAt,
        r#"{"justification":{"grounds":[],"text":null},"locus":{"from":{"span":null,"unit":{"book":43,"chapter":3,"verse":16}},"to":{"span":null,"unit":{"book":43,"chapter":3,"verse":21}}},"place":"Place:jerusalem","provenance":"red-letter"}"#
    );

    // 15. mentions -- a locus WITH a token span (the layer-tagged case).
    law!(
        Mentions {
            locus: TextLocus { at: TextRef::Bible(vr(7, 1, 2)), span: Some(span(3, 5)) },
            entity: MentionedEntity::PeopleGroup(PeopleGroupId::new("tribe-of-judah")),
            provenance: "theographic".into(),
        },
        RowFamily::Mentions,
        r#"{"entity":{"PeopleGroup":"PeopleGroup:tribe-of-judah"},"locus":{"at":{"Bible":{"book":7,"chapter":1,"verse":2}},"span":{"end":5,"layer":"kjv","start":3}},"provenance":"theographic"}"#
    );

    // 16. cross_refs -- `to_last` present, votes carried.
    law!(
        CrossRef {
            from: tl(51, 1, 15),
            to: tl(51, 1, 16),
            to_last: Some(tl(51, 1, 19)),
            target_display: "COL.1.16-19".into(),
            votes: 7,
            provenance: "openbible-xrefs".into(),
        },
        RowFamily::CrossRefs,
        r#"{"from":{"at":{"Bible":{"book":51,"chapter":1,"verse":15}},"span":null},"provenance":"openbible-xrefs","target_display":"COL.1.16-19","to":{"at":{"Bible":{"book":51,"chapter":1,"verse":16}},"span":null},"to_last":{"at":{"Bible":{"book":51,"chapter":1,"verse":19}},"span":null},"votes":7}"#
    );

    // 17. quotes -- uninhabited in the shipped graph; the encoder is real.
    law!(
        Quotes {
            quoting: tl(40, 4, 4),
            quoted: blr((5, 8, 3), (5, 8, 3)),
            provenance: "curated/quotes".into(),
        },
        RowFamily::Quotes,
        r#"{"provenance":"curated/quotes","quoted":{"from":{"span":null,"unit":{"book":5,"chapter":8,"verse":3}},"to":{"span":null,"unit":{"book":5,"chapter":8,"verse":3}}},"quoting":{"at":{"Bible":{"book":40,"chapter":4,"verse":4}},"span":null}}"#
    );

    // 18. confesses -- uninhabited today; a Concord locus confessing Scripture.
    law!(
        Confesses {
            confessing: cl(1, 2, 3),
            confessed: blr((45, 3, 28), (45, 3, 28)),
            provenance: "concord".into(),
            justification: full_justification(),
        },
        RowFamily::Confesses,
        r#"{"confessed":{"from":{"span":null,"unit":{"book":45,"chapter":3,"verse":28}},"to":{"span":null,"unit":{"book":45,"chapter":3,"verse":28}}},"confessing":{"span":null,"unit":{"article":2,"paragraph":3,"part":1}},"justification":{"grounds":[{"Scripture":{"from":{"span":null,"unit":{"book":40,"chapter":3,"verse":13}},"to":{"span":null,"unit":{"book":40,"chapter":3,"verse":17}}}},{"Anchor":"Anchor:ussher-4004bc"},{"Source":"Source:openbible-geo"}],"text":"Jordan, at Bethabara"},"provenance":"concord"}"#
    );

    // 19. corresponds_bible -- uninhabited today; span-level alignment.
    law!(
        Corresponds::<BibleTag> {
            a: Locus { unit: vr(43, 3, 16), span: Some(span(0, 4)) },
            b: Locus { unit: vr(43, 3, 16), span: Some(span(5, 9)) },
            provenance: "alignment".into(),
        },
        RowFamily::CorrespondsBible,
        r#"{"a":{"span":{"end":4,"layer":"kjv","start":0},"unit":{"book":43,"chapter":3,"verse":16}},"b":{"span":{"end":9,"layer":"kjv","start":5},"unit":{"book":43,"chapter":3,"verse":16}},"provenance":"alignment"}"#
    );

    // 20. temporal_adjacency
    law!(
        TemporalAdjacency {
            earlier: EventId::new("nativity"),
            later: EventId::new("jesus-baptized"),
            provenance: "derived/chronology".into(),
        },
        RowFamily::TemporalAdjacency,
        r#"{"earlier":"Event:nativity","later":"Event:jesus-baptized","provenance":"derived/chronology"}"#
    );

    // 21. analogue
    law!(
        Analogue {
            a: EventId::new("leper-healed-galilee"),
            b: EventId::new("leper-healed-capernaum"),
            provenance: "curated/analogues".into(),
        },
        RowFamily::Analogue,
        r#"{"a":"Event:leper-healed-galilee","b":"Event:leper-healed-capernaum","provenance":"curated/analogues"}"#
    );

    // 22. occurs (LEX-1) -- a ONE-token span on the Greek layer: John 3:16
    // token 4 (the article before "God").
    law!(
        Occurs {
            entry: LexiconEntryId::new("G3588"),
            locus: TextLocus {
                at: TextRef::Bible(vr(42, 3, 16)),
                span: Some(TokenSpan::new(TranslationId("greek_textus_receptus".into()), 4, 4).unwrap()),
            },
            provenance: "stepbible-tagnt".into(),
        },
        RowFamily::Occurs,
        r#"{"entry":"LexiconEntry:G3588","locus":{"at":{"Bible":{"book":42,"chapter":3,"verse":16}},"span":{"end":4,"layer":"greek_textus_receptus","start":4}},"provenance":"stepbible-tagnt"}"#
    );

    assert_eq!(
        covered,
        RowFamily::ALL.to_vec(),
        "every family must be pinned here, in ordinal order"
    );
}

#[test]
fn every_variant_of_every_shared_part_round_trips() {
    // DatePlacement x PlacementBasis: all four placements, both bases.
    let placements = [
        DatePlacement::AnchorBinding {
            anchor: AnchorId::new("abraham-called"),
            offset: Duration { years: 430, months: 2, days: 10 },
        },
        DatePlacement::ReignYear { reign: AnchorId::new("hezekiah"), year_of_reign: 14 },
        DatePlacement::SequenceAfter {
            prior: EventId::new("nativity"),
            spacing: Duration::years(30),
        },
        DatePlacement::EraOnly { era: EraId::new("divided-kingdom") },
    ];
    for placement in placements {
        for basis in [PlacementBasis::Textual, PlacementBasis::Traditional] {
            round_trip(
                &DatedBy {
                    event: EventId::new("e"),
                    placement: placement.clone(),
                    basis,
                    justification: Justification::default(),
                    provenance: "p".into(),
                },
                RowFamily::DatedBy,
            );
        }
    }
    assert_eq!(
        String::from_utf8(
            DatePlacement::ReignYear { reign: AnchorId::new("hezekiah"), year_of_reign: 14 }
                .encode()
        )
        .unwrap(),
        r#"{"ReignYear":{"reign":"Anchor:hezekiah","year_of_reign":14}}"#
    );
    assert_eq!(
        String::from_utf8(PlacementBasis::Traditional.encode()).unwrap(),
        r#"{"Traditional":null}"#
    );

    // Namesake: all three named things.
    for namesake in [
        Namesake::PeopleGroup(PeopleGroupId::new("moabites")),
        Namesake::Place(PlaceId::new("dan")),
        Namesake::Polity(PolityId::new("kingdom-of-judah")),
    ] {
        round_trip(
            &NamedAfter {
                namesake,
                eponym: PersonId::new("judah"),
                provenance: "p".into(),
                justification: Justification::default(),
            },
            RowFamily::NamedAfter,
        );
    }

    // MentionedEntity: all four attested senses.
    for entity in [
        MentionedEntity::Place(PlaceId::new("jerusalem")),
        MentionedEntity::Person(PersonId::new("judah")),
        MentionedEntity::PeopleGroup(PeopleGroupId::new("tribe-of-judah")),
        MentionedEntity::Event(EventId::new("annunciation")),
    ] {
        round_trip(
            &Mentions { locus: tl(7, 1, 2), entity, provenance: "p".into() },
            RowFamily::Mentions,
        );
    }

    // Ground: all three, and the empty justification.
    for grounds in [
        BTreeSet::new(),
        [Ground::Scripture(blr((1, 1, 1), (1, 1, 2)))].into_iter().collect(),
        [Ground::Anchor(AnchorId::new("a"))].into_iter().collect(),
        [Ground::Source(SourceId::new("s"))].into_iter().collect(),
    ] {
        for text in [None, Some("why".to_string())] {
            round_trip(
                &LocatedAt {
                    event: EventId::new("e"),
                    place: PlaceId::new("p"),
                    provenance: "prov".into(),
                    justification: Justification { text, grounds: grounds.clone() },
                },
                RowFamily::LocatedAt,
            );
        }
    }

    // ContainerContent: the empty locus set is lawful identity.
    round_trip(
        &Contains::<ConcordTag> {
            container: ContainerNodeId::new("concord/ac"),
            content: ContainerContent::Loci(LocusSet::default()),
            provenance: "p".into(),
            justification: Justification::default(),
        },
        RowFamily::ContainsConcord,
    );

    // CrossRef with no `to_last`.
    round_trip(
        &CrossRef {
            from: tl(1, 1, 1),
            to: tl(43, 1, 1),
            to_last: None,
            target_display: "JHN.1.1".into(),
            votes: 0,
            provenance: "p".into(),
        },
        RowFamily::CrossRefs,
    );
}

// --------------------------------------------------------------- malformed

/// Corrupt one canonical spelling into another. The replacement keeps the
/// object's key order canonical -- the point of each case below is the
/// DECODER's own check, not the byte parser's.
fn corrupt(bytes: &[u8], from: &str, to: &str) -> Vec<u8> {
    let s = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(s.contains(from), "corruption target `{from}` is not in {s}");
    s.replace(from, to).into_bytes()
}

fn simple_located_at() -> Vec<u8> {
    LocatedAt {
        event: EventId::new("e"),
        place: PlaceId::new("p"),
        provenance: "prov".into(),
        justification: Justification::default(),
    }
    .encode()
}

/// Every malformed-row case asserts its EXACT path, that the path is
/// root-anchored (R10), and that the message says something.
macro_rules! bad {
    ($ty:ty, $bytes:expr, $path:expr) => {{
        let bytes = $bytes;
        let err = <$ty>::decode(&bytes)
            .expect_err(&format!("expected a rejection for {}", String::from_utf8_lossy(&bytes)));
        assert_eq!(err.path, $path, "for {}", String::from_utf8_lossy(&bytes));
        assert!(err.path.starts_with(ROOT), "path must start at the root: {err:?}");
        assert!(!err.msg.is_empty(), "every error must say what went wrong: {err:?}");
    }};
}

#[test]
fn row_decode_reports_the_failing_path() {
    let base = simple_located_at();

    // R9: an unknown member, at the row and inside a nested object.
    bad!(LocatedAt, corrupt(&base, r#""provenance":"prov""#, r#""provenance":"prov","zz":1"#), "$.zz");
    bad!(
        LocatedAt,
        corrupt(&base, r#""grounds":[],"text":null"#, r#""grounds":[],"text":null,"zz":1"#),
        "$.justification.zz"
    );
    // A missing member reports under its own key, not the object's.
    bad!(LocatedAt, corrupt(&base, r#""justification":{"grounds":[],"text":null},"#, ""), "$.justification");
    // A typed id whose KIND disagrees with the field -- the whole reason
    // ids carry their kind in the bytes.
    bad!(LocatedAt, corrupt(&base, r#""place":"Place:p""#, r#""place":"Event:p""#), "$.place");
    bad!(LocatedAt, corrupt(&base, r#""place":"Place:p""#, r#""place":"Nope:p""#), "$.place");
    bad!(LocatedAt, corrupt(&base, r#""place":"Place:p""#, r#""place":"nokind""#), "$.place");
    bad!(LocatedAt, corrupt(&base, r#""place":"Place:p""#, r#""place":42"#), "$.place");
    bad!(LocatedAt, corrupt(&base, r#""text":null"#, r#""text":42"#), "$.justification.text");
    // A ground variant nobody knows, and a ground id of the wrong kind.
    bad!(
        LocatedAt,
        corrupt(&base, r#""grounds":[]"#, r#""grounds":[{"Nope":null}]"#),
        "$.justification.grounds.0.Nope"
    );
    bad!(
        LocatedAt,
        corrupt(&base, r#""grounds":[]"#, r#""grounds":[{"Anchor":"Event:a"}]"#),
        "$.justification.grounds.0.Anchor"
    );
    // A set is spelled strictly increasing -- a repeat or a re-ordering
    // would decode to a set that re-encodes to different bytes.
    bad!(
        LocatedAt,
        corrupt(&base, r#""grounds":[]"#, r#""grounds":[{"Anchor":"Anchor:a"},{"Anchor":"Anchor:a"}]"#),
        "$.justification.grounds.1"
    );
    bad!(
        LocatedAt,
        corrupt(&base, r#""grounds":[]"#, r#""grounds":[{"Source":"Source:s"},{"Anchor":"Anchor:a"}]"#),
        "$.justification.grounds.1"
    );
    // R10c: a root-level type failure is still located.
    bad!(LocatedAt, b"null".to_vec(), ROOT);

    // The validating constructors, re-run on the way in.
    let attests = Attests {
        event: EventId::new("e"),
        attestation: blr((40, 3, 13), (40, 3, 17)),
        provenance: "p".into(),
        justification: Justification::default(),
    }
    .encode();
    bad!(Attests, corrupt(&attests, r#""verse":17"#, r#""verse":2"#), "$.attestation");

    let succession = Succession::new(
        NarrativeId::new("n"),
        vec![EventId::new("a"), EventId::new("b")],
        "p".into(),
        Justification::default(),
    )
    .unwrap()
    .encode();
    bad!(Succession, corrupt(&succession, r#""Event:b""#, r#""Event:a""#), "$.chain");
    bad!(Succession, corrupt(&succession, r#"["Event:a","Event:b"]"#, "[]"), "$.chain");
    bad!(Succession, corrupt(&succession, r#""Event:b""#, r#""Place:b""#), "$.chain.1");

    let mentions = Mentions {
        locus: TextLocus { at: TextRef::Bible(vr(7, 1, 2)), span: Some(span(3, 5)) },
        entity: MentionedEntity::Event(EventId::new("e")),
        provenance: "p".into(),
    }
    .encode();
    bad!(Mentions, corrupt(&mentions, r#""end":5"#, r#""end":1"#), "$.locus.span");
    bad!(Mentions, corrupt(&mentions, r#""Bible""#, r#""Nope""#), "$.locus.at.Nope");
    bad!(Mentions, corrupt(&mentions, r#""chapter":1"#, r#""chapter":99999"#), "$.locus.at.Bible.chapter");

    let dated_by = DatedBy {
        event: EventId::new("e"),
        placement: DatePlacement::AnchorBinding {
            anchor: AnchorId::new("a"),
            offset: Duration::years(430),
        },
        basis: PlacementBasis::Textual,
        justification: Justification::default(),
        provenance: "p".into(),
    }
    .encode();
    bad!(DatedBy, corrupt(&dated_by, r#"{"Textual":null}"#, r#"{"Textual":1}"#), "$.basis.Textual");
    bad!(DatedBy, corrupt(&dated_by, r#""AnchorBinding""#, r#""Nope""#), "$.placement.Nope");
    bad!(
        DatedBy,
        corrupt(&dated_by, r#""offset":{"days":0,"months":0,"years":430}"#, r#""offset":{"days":0,"months":0,"weeks":1,"years":430}"#),
        "$.placement.AnchorBinding.offset.weeks"
    );

    let cross_ref = CrossRef {
        from: tl(1, 1, 1),
        to: tl(43, 1, 1),
        to_last: None,
        target_display: "JHN.1.1".into(),
        votes: 3,
        provenance: "p".into(),
    }
    .encode();
    bad!(CrossRef, corrupt(&cross_ref, r#""votes":3"#, r#""votes":-1"#), "$.votes");

    let contains = Contains::<BibleTag> {
        container: ContainerNodeId::new("bible/GEN.1"),
        content: ContainerContent::Loci(LocusSet([bl(1, 1, 1)].into_iter().collect())),
        provenance: "p".into(),
        justification: Justification::default(),
    }
    .encode();
    bad!(
        Contains::<BibleTag>,
        corrupt(&contains, r#""Loci""#, r#""Nope""#),
        "$.content.Nope"
    );
    bad!(
        Contains::<BibleTag>,
        corrupt(&contains, r#""book":1,"chapter":1,"verse":1"#, r#""book":1,"chapter":1,"chorus":9,"verse":1"#),
        "$.content.Loci.0.unit.chorus"
    );
    let two_loci = Contains::<BibleTag> {
        container: ContainerNodeId::new("bible/GEN.1"),
        content: ContainerContent::Loci(LocusSet([bl(1, 1, 1), bl(1, 1, 2)].into_iter().collect())),
        provenance: "p".into(),
        justification: Justification::default(),
    }
    .encode();
    bad!(
        Contains::<BibleTag>,
        corrupt(&two_loci, r#""verse":2"#, r#""verse":1"#),
        "$.content.Loci.1"
    );
}

#[test]
fn the_row_family_wrapper_is_the_hashed_form() {
    assert_eq!(
        String::from_utf8(encode_row_in_family(RowFamily::LocatedAt, Value::Null)).unwrap(),
        r#"{"family":"located_at","row":null}"#
    );
    // Two families whose ROWS are byte-identical still hash differently,
    // because the family is inside the bytes.
    let a = Analogue {
        a: EventId::new("x"),
        b: EventId::new("y"),
        provenance: "p".into(),
    };
    let t = TemporalAdjacency {
        earlier: EventId::new("x"),
        later: EventId::new("y"),
        provenance: "p".into(),
    };
    assert_ne!(
        encode_row_in_family(RowFamily::Analogue, a.to_value()),
        encode_row_in_family(RowFamily::TemporalAdjacency, t.to_value())
    );
}

#[test]
fn the_row_goldens_stay_control_character_free() {
    // The same discipline the node goldens keep (R5): a golden that gains
    // a control character must be re-pinned deliberately.
    let goldens: &[&[u8]] = &[
        br#"{"event":"Event:jesus-baptized","justification":{"grounds":[{"Scripture":{"from":{"span":null,"unit":{"book":40,"chapter":3,"verse":13}},"to":{"span":null,"unit":{"book":40,"chapter":3,"verse":17}}}},{"Anchor":"Anchor:ussher-4004bc"},{"Source":"Source:openbible-geo"}],"text":"Jordan, at Bethabara"},"place":"Place:jordan-river","provenance":"curated/events"}"#,
        br#"{"basis":{"Textual":null},"event":"Event:exodus","justification":{"grounds":[],"text":null},"placement":{"AnchorBinding":{"anchor":"Anchor:abraham-called","offset":{"days":0,"months":0,"years":430}}},"provenance":"ussher"}"#,
        br#"{"family":"located_at","row":null}"#,
    ];
    for g in goldens {
        assert!(
            !g.iter().any(|b| *b < 0x20),
            "golden holds a control character: {}",
            String::from_utf8_lossy(g)
        );
        assert!(
            !String::from_utf8_lossy(g).contains("\\u"),
            "golden holds an escape: {}",
            String::from_utf8_lossy(g)
        );
    }
}
