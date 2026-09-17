//! DB-2a golden vectors for the canonical JSON core.
//!
//! These are the LAW for the on-disk node encoding: the byte strings
//! asserted here are pinned, not descriptive. `std` only -- `graph-types`
//! is a zero-dependency crate and its tests keep that promise.

use atlas_graph_types::canon::{parse, serialize, Canon, Value};
use atlas_graph_types::chrono::{TimePoint, Year};
use atlas_graph_types::id::{AnyNodeId, NodeKind, SourceId};
use atlas_graph_types::node::{
    EventWitnessPayload, Node, NodePayload, PolityDeltaPayload, PolityEraPayload,
};
use atlas_graph_types::text::TranslationId;
use std::collections::BTreeMap;

fn obj(pairs: &[(&str, Value)]) -> Value {
    Value::Obj(pairs.iter().map(|(k, v)| (k.to_string(), v.clone())).collect())
}

// ---------------------------------------------------------------- json core

#[test]
fn serialize_is_canonical_and_minimal() {
    let v = obj(&[
        ("b", Value::Int(2)),
        ("a", Value::Str("x\"y\\z\n".into())),
        ("c", Value::Arr(vec![Value::Null, Value::Bool(true)])),
    ]);
    assert_eq!(
        String::from_utf8(serialize(&v)).unwrap(),
        r#"{"a":"x\"y\\z\n","b":2,"c":[null,true]}"#
    );
}

#[test]
fn control_characters_use_the_shortest_legal_escape() {
    // The five JSON two-char control escapes are the minimal form; every
    // other C0 control goes out as \u00xx with LOWERCASE hex. Nothing
    // else is escaped -- '/' and all non-ASCII ride raw as UTF-8.
    let v = Value::Str("\u{8}\t\n\u{c}\r\u{0}\u{1f}\u{b}/\u{e9}".into());
    assert_eq!(
        String::from_utf8(serialize(&v)).unwrap(),
        "\"\\b\\t\\n\\f\\r\\u0000\\u001f\\u000b/\u{e9}\""
    );
    assert_eq!(parse(&serialize(&v)).unwrap(), v);
}

#[test]
fn floats_use_shortest_round_trip_and_reject_non_finite() {
    assert_eq!(serialize(&Value::float(31.7767).unwrap()), b"31.7767");
    // Rust's `{}` prints "1" for 1.0_f64 -- whatever `{}` yields IS the law.
    assert_eq!(serialize(&Value::float(1.0).unwrap()), b"1");
    assert_eq!(serialize(&Value::float(0.1 + 0.2).unwrap()), b"0.30000000000000004");
    assert!(Value::float(f64::NAN).is_err());
    assert!(Value::float(f64::INFINITY).is_err());
    assert!(Value::float(f64::NEG_INFINITY).is_err());
}

#[test]
fn negative_zero_survives_the_round_trip_as_a_float() {
    // `{}` prints "-0" for -0.0_f64, which has no i64 spelling; the parser
    // owes that exact token back as a Float or encode/decode is not total.
    let bytes = serialize(&Value::float(-0.0).unwrap());
    assert_eq!(bytes, b"-0");
    match parse(&bytes).unwrap() {
        Value::Float(f) => assert!(f.is_sign_negative() && f == 0.0),
        other => panic!("expected Float(-0.0), got {other:?}"),
    }
}

#[test]
fn parse_is_strict() {
    assert!(parse(b"{\"a\":1, \"b\":2}").is_err(), "whitespace");
    assert!(parse(b"{\"b\":1,\"a\":2}").is_err(), "unsorted keys");
    assert!(parse(b"{\"a\":1,\"a\":2}").is_err(), "duplicate key");
    assert!(parse(b"01").is_err(), "leading zero");
    assert!(parse(b"\"\\u0041\"").is_err(), "non-minimal escape of A");
    assert!(parse(b"{}x").is_err(), "trailing content");
    assert_eq!(
        parse(b"{\"a\":[1,-2,\"\\u0000\"]}").unwrap(),
        obj(&[(
            "a",
            Value::Arr(vec![Value::Int(1), Value::Int(-2), Value::Str("\u{0}".into())])
        )])
    );
}

#[test]
fn parse_rejects_every_non_canonical_spelling() {
    let bad: &[(&[u8], &str)] = &[
        (b"\"\\u000a\"", "newline must use the short escape"),
        (b"\"\\/\"", "solidus never needs escaping"),
        (b"\"\\x41\"", "unknown escape"),
        (b"\"\\u001F\"", "uppercase hex"),
        (b"\"a\nb\"", "raw control char in a string"),
        (b"\"unterminated", "unterminated string"),
        (b"[1,]", "trailing comma"),
        (b"[1 ,2]", "whitespace in an array"),
        (b" 1", "leading whitespace"),
        (b"1 ", "trailing whitespace"),
        (b"1.50", "non-minimal float"),
        (b"1e5", "exponent form"),
        (b"+1", "explicit plus"),
        (b"-01", "negative leading zero"),
        (b"1.", "bare decimal point"),
        (b"TRUE", "case"),
        (b"", "empty input"),
        (b"{\"a\":1", "unterminated object"),
        (b"\xff\xfe", "invalid utf-8"),
    ];
    for (bytes, why) in bad {
        let got = parse(bytes);
        assert!(got.is_err(), "expected rejection ({why}), got {got:?}");
        assert!(!got.unwrap_err().path.is_empty(), "error path missing for {why}");
    }
}

#[test]
fn parse_rejects_deep_nesting_without_panicking() {
    let deep: Vec<u8> = std::iter::repeat(b'[').take(100_000).collect();
    assert!(parse(&deep).is_err(), "depth limit, not a stack overflow");
}

#[test]
fn every_serialized_value_parses_back_identically() {
    let v = obj(&[
        ("f", Value::float(-0.5).unwrap()),
        ("s", Value::Str("héllo — ✓".into())),
        ("n", Value::Null),
        ("o", obj(&[("z", Value::Int(i64::MIN))])),
    ]);
    assert_eq!(parse(&serialize(&v)).unwrap(), v);
    for v in [Value::Null, Value::Bool(false), Value::Arr(vec![]), obj(&[])] {
        assert_eq!(parse(&serialize(&v)).unwrap(), v, "empty/scalar round trip");
    }
}

// ----------------------------------------------------------------- id forms

#[test]
fn ids_round_trip_including_colons_in_raw() {
    use atlas_graph_types::canon::ids::*;
    let id = AnyNodeId { kind: NodeKind::TextUnit, raw: "bible/JHN.3.16".into() };
    assert_eq!(any_node_id_str(&id), "TextUnit:bible/JHN.3.16");
    assert_eq!(parse_any_node_id("TextUnit:bible/JHN.3.16").unwrap(), id);
    let weird = AnyNodeId { kind: NodeKind::Container, raw: "a:b:c".into() };
    assert_eq!(parse_any_node_id(&any_node_id_str(&weird)).unwrap(), weird);
    assert!(parse_any_node_id("Nope:x").is_err());
    assert!(parse_any_node_id("Place").is_err(), "no separator");
}

#[test]
fn every_node_kind_names_itself_with_its_debug_name() {
    use atlas_graph_types::canon::ids::*;
    let all = [
        NodeKind::TextUnit,
        NodeKind::Container,
        NodeKind::Event,
        NodeKind::Narrative,
        NodeKind::Place,
        NodeKind::Person,
        NodeKind::Anchor,
        NodeKind::Era,
        NodeKind::Polity,
        NodeKind::CatechismItem,
        NodeKind::Source,
        NodeKind::Translation,
        NodeKind::PeopleGroup,
        NodeKind::CommentaryItem,
    ];
    for k in all {
        assert_eq!(node_kind_str(k), format!("{k:?}"));
        assert_eq!(parse_node_kind(node_kind_str(k)).unwrap(), k);
    }
}

#[test]
fn positions_round_trip_for_nodes_and_edges() {
    use atlas_graph_types::canon::ids::*;
    use atlas_graph_types::id::Position;
    use atlas_graph_types::EdgeId;
    let n = Position::Node(AnyNodeId { kind: NodeKind::Place, raw: "jerusalem".into() });
    assert_eq!(position_str(&n), "n:Place:jerusalem");
    assert_eq!(parse_position(&position_str(&n)).unwrap(), n);
    let e = Position::Edge(EdgeId("located_at:00ff".into()));
    assert_eq!(position_str(&e), "e:located_at:00ff");
    assert_eq!(parse_position(&position_str(&e)).unwrap(), e);
    assert!(parse_position("x:Place:jerusalem").is_err());
    assert!(parse_position("n:Nope:x").is_err());
}

// -------------------------------------------------------------- node golden

#[test]
fn place_node_golden_bytes() {
    let n = Node {
        id: AnyNodeId { kind: NodeKind::Place, raw: "jerusalem".into() },
        payload: NodePayload::Place {
            canonical: "Jerusalem".into(),
            lat: 31.7767,
            lon: 35.2345,
            aliases: vec!["Salem".into()],
            description: None,
        },
        provenance: "openbible-geo".into(),
    };
    let bytes = n.encode();
    assert_eq!(
        String::from_utf8(bytes.clone()).unwrap(),
        r#"{"id":"Place:jerusalem","payload":{"Place":{"aliases":["Salem"],"canonical":"Jerusalem","description":null,"lat":31.7767,"lon":35.2345}},"provenance":"openbible-geo"}"#
    );
    let back = Node::decode(&bytes).unwrap();
    assert_eq!(back.id, n.id);
    assert_eq!(back.provenance, n.provenance);
    assert_eq!(format!("{:?}", back.payload), format!("{:?}", n.payload));
}

#[test]
fn node_decode_reports_the_failing_path() {
    let bad = br#"{"id":"Place:jerusalem","payload":{"Place":{"aliases":["Salem"],"canonical":"Jerusalem","description":null,"lat":"nope","lon":35.2345}},"provenance":"p"}"#;
    let err = Node::decode(bad).unwrap_err();
    assert_eq!(err.path, "payload.Place.lat", "got {err:?}");

    let unknown = br#"{"id":"Place:jerusalem","payload":{"Nope":{}},"provenance":"p"}"#;
    let err = Node::decode(unknown).unwrap_err();
    assert_eq!(err.path, "payload.Nope");

    let missing = br#"{"id":"Place:jerusalem","provenance":"p"}"#;
    assert_eq!(Node::decode(missing).unwrap_err().path, "payload");
}

#[test]
fn unknown_corpus_is_rejected_because_corpus_is_a_static_str() {
    let bad = br#"{"id":"TextUnit:x/1.1.1","payload":{"TextUnit":{"corpus":"vulgate","renderings":{}}},"provenance":"p"}"#;
    let err = Node::decode(bad).unwrap_err();
    assert_eq!(err.path, "payload.TextUnit.corpus");
}

fn nid(kind: NodeKind, raw: &str) -> AnyNodeId {
    AnyNodeId { kind, raw: raw.into() }
}

/// Encode, decode, and prove the decoded node is the node -- plus that
/// re-encoding it yields byte-identical output (the fixed point encode
/// and decode must share).
fn round_trip(n: &Node) {
    let bytes = n.encode();
    // The strict parser accepts our own output, and re-serializing what it
    // read gives the same bytes. (Byte identity, NOT `Value` identity: a
    // whole-valued f64 and the equal integer share one canonical spelling,
    // so `lat: -31.0` comes back as `Int(-31)` -- which is exactly why the
    // f64 field decoder accepts both. See the number law in canon/mod.rs.)
    let reparsed = parse(&bytes).unwrap_or_else(|e| panic!("self-parse {}: {e}", n.id.raw));
    assert_eq!(serialize(&reparsed), bytes, "self-parse is not a fixed point: {}", n.id.raw);
    let back = match Node::decode(&bytes) {
        Ok(b) => b,
        Err(e) => panic!(
            "decode failed for {:?}: {:?}\n{}",
            n.id,
            e,
            String::from_utf8_lossy(&bytes)
        ),
    };
    assert_eq!(back.id, n.id);
    assert_eq!(back.provenance, n.provenance);
    assert_eq!(
        format!("{:?}", back.payload),
        format!("{:?}", n.payload),
        "payload changed for {}",
        n.id.raw
    );
    assert_eq!(back.encode(), bytes, "re-encode is not a fixed point for {}", n.id.raw);
}

#[test]
fn every_payload_variant_round_trips() {
    let mut renderings: BTreeMap<TranslationId, String> = BTreeMap::new();
    renderings.insert(TranslationId("kjv".into()), "In the beginning God created".into());
    renderings.insert(TranslationId("asv".into()), "In the beginning God created\u{a0}".into());

    let mut translations: BTreeMap<String, Vec<String>> = BTreeMap::new();
    translations.insert("kjv".into(), vec!["JHN.3.16".into(), "JHN.3.17".into()]);
    translations.insert("asv".into(), vec![]);

    let nodes = vec![
        // 1. TextUnit -- two renderings, the &'static str corpus.
        Node {
            id: nid(NodeKind::TextUnit, "bible/GEN.1.1"),
            payload: NodePayload::TextUnit { corpus: "bible", renderings },
            provenance: "kjv-ingest".into(),
        },
        // 1b. TextUnit -- the other corpus literal, empty LayerMap.
        Node {
            id: nid(NodeKind::TextUnit, "concord/1.1.1"),
            payload: NodePayload::TextUnit { corpus: "concord", renderings: BTreeMap::new() },
            provenance: "boc-ingest".into(),
        },
        // 2. Container
        Node {
            id: nid(NodeKind::Container, "bible/JHN"),
            payload: NodePayload::Container { title: "John".into() },
            provenance: "canon".into(),
        },
        // 3. Event -- every Option Some, a witness with a structured map.
        Node {
            id: nid(NodeKind::Event, "crossing-the-red-sea"),
            payload: NodePayload::Event {
                label: "Crossing the Red Sea".into(),
                kind: "deliverance".into(),
                verses: vec!["EXO.14.21".into(), "EXO.14.22".into()],
                witnesses: vec![
                    EventWitnessPayload {
                        book: "Exodus".into(),
                        translations,
                        ref_note: Some("Ex 14".into()),
                        robertson_section: Some("II.a".into()),
                    },
                    EventWitnessPayload {
                        book: "Psalms".into(),
                        translations: BTreeMap::new(),
                        ref_note: None,
                        robertson_section: None,
                    },
                ],
                robertson_section: Some("II".into()),
                acts_section: Some("n/a".into()),
                atlas_section: Some("3".into()),
                kjv_superscription: Some("A Song of Moses".into()),
                ref_note: Some("Ussher 2513 AM".into()),
            },
            provenance: "crockett".into(),
        },
        // 3b. Event -- every Option None, empty Vecs, empty strings.
        Node {
            id: nid(NodeKind::Event, "bare-event"),
            payload: NodePayload::Event {
                label: "Bare".into(),
                kind: String::new(),
                verses: vec![],
                witnesses: vec![],
                robertson_section: None,
                acts_section: None,
                atlas_section: None,
                kjv_superscription: None,
                ref_note: None,
            },
            provenance: String::new(),
        },
        // 4. Narrative
        Node {
            id: nid(NodeKind::Narrative, "exodus-journey"),
            payload: NodePayload::Narrative {
                label: "The Exodus".into(),
                color: "#c0392b".into(),
            },
            provenance: "atlas".into(),
        },
        // 5. Place -- description Some, unicode + quotes, empty aliases,
        //    a whole-number latitude (encodes as a bare integer).
        Node {
            id: nid(NodeKind::Place, "en-gedi"),
            payload: NodePayload::Place {
                canonical: "En-gedi".into(),
                lat: -31.0,
                lon: 35.3921,
                aliases: vec![],
                description: Some("A spring by the Dead Sea — \"kid's fountain\".".into()),
            },
            provenance: "openbible-geo".into(),
        },
        // 6. Person -- mixed Some/None numeric options.
        Node {
            id: nid(NodeKind::Person, "moses"),
            payload: NodePayload::Person {
                label: "Moses".into(),
                gender: Some("male".into()),
                birth_year: Some(-1571),
                death_year: None,
                also_called: vec!["Moshe".into(), "the man of God".into()],
                description: None,
            },
            provenance: "theographic".into(),
        },
        // 7. PeopleGroup
        Node {
            id: nid(NodeKind::PeopleGroup, "ammonites"),
            payload: NodePayload::PeopleGroup {
                label: "Ammonites".into(),
                description: Some("Descendants of Ben-ammi.".into()),
            },
            provenance: "easton".into(),
        },
        // 8. Anchor -- month + day present.
        Node {
            id: nid(NodeKind::Anchor, "creation"),
            payload: NodePayload::Anchor {
                at: TimePoint::new(Year::new(-4004).unwrap(), Some(10), Some(23)).unwrap(),
                citation: "Ussher, Annals, §1".into(),
            },
            provenance: "ussher".into(),
        },
        // 8b. Anchor -- year only (both Options None).
        Node {
            id: nid(NodeKind::Anchor, "flood"),
            payload: NodePayload::Anchor {
                at: TimePoint::year_only(Year::new(-2348).unwrap()),
                citation: "Ussher, Annals, §5".into(),
            },
            provenance: "ussher".into(),
        },
        // 9. Era
        Node {
            id: nid(NodeKind::Era, "divided-kingdom"),
            payload: NodePayload::Era {
                label: "Divided Kingdom".into(),
                from_year: -975,
                to_year: -586,
            },
            provenance: "atlas".into(),
        },
        // 10. Polity -- two eras, one with a transition, one with a fall;
        //     rings carry whole-number coordinates on purpose (they encode
        //     as bare integers and must decode back into f64).
        Node {
            id: nid(NodeKind::Polity, "egypt"),
            payload: NodePayload::Polity {
                label: "Egypt".into(),
                color_key: 7,
                eras: vec![
                    PolityEraPayload {
                        name: "Egypt".into(),
                        from_year: -2000,
                        to_year: -332,
                        rings: vec![vec![(31.0, 30.0), (32.5, 29.25), (31.0, 30.0)], vec![]],
                        ref_note: "Crockett pl. 2".into(),
                        transition: Some(PolityDeltaPayload {
                            event: "Alexander takes Egypt".into(),
                            verses: vec!["DAN.11.3".into()],
                            ref_note: "Dan 11".into(),
                        }),
                        fall: None,
                    },
                    PolityEraPayload {
                        name: "Ptolemaic Egypt".into(),
                        from_year: -332,
                        to_year: -30,
                        rings: vec![],
                        ref_note: String::new(),
                        transition: None,
                        fall: Some(PolityDeltaPayload {
                            event: "Actium".into(),
                            verses: vec![],
                            ref_note: String::new(),
                        }),
                    },
                ],
            },
            provenance: "crockett".into(),
        },
        // 10b. Polity -- no eras at all.
        Node {
            id: nid(NodeKind::Polity, "edom"),
            payload: NodePayload::Polity {
                label: "Edom".into(),
                color_key: 0,
                eras: vec![],
            },
            provenance: "crockett".into(),
        },
        // 11. CatechismItem
        Node {
            id: nid(NodeKind::CatechismItem, "sc/1st-commandment"),
            payload: NodePayload::CatechismItem { label: "The First Commandment".into() },
            provenance: "small-catechism".into(),
        },
        // 12. CommentaryItem -- SourceId + heading Some.
        Node {
            id: nid(NodeKind::CommentaryItem, "kretzmann/JHN.3.16"),
            payload: NodePayload::CommentaryItem {
                work: SourceId::new("kretzmann"),
                heading: Some("The love of God".into()),
                text: "The greatest of all statements\u{2014}".into(),
            },
            provenance: "kretzmann".into(),
        },
        // 12b. CommentaryItem -- heading None, empty text.
        Node {
            id: nid(NodeKind::CommentaryItem, "kretzmann/JHN.3.17"),
            payload: NodePayload::CommentaryItem {
                work: SourceId::new("kretzmann"),
                heading: None,
                text: String::new(),
            },
            provenance: "kretzmann".into(),
        },
        // 13. Source
        Node {
            id: nid(NodeKind::Source, "openbible-geo"),
            payload: NodePayload::Source { label: "OpenBible Geocoding".into() },
            provenance: "bootstrap".into(),
        },
        // 14. Translation
        Node {
            id: nid(NodeKind::Translation, "kjv"),
            payload: NodePayload::Translation { label: "King James Version".into() },
            provenance: "bootstrap".into(),
        },
    ];

    // All fourteen NodePayload variants are exercised; the extra rows are
    // the Some/None and empty/non-empty companions the plan asked for.
    let mut seen: Vec<&'static str> = nodes
        .iter()
        .map(|n| match n.payload {
            NodePayload::TextUnit { .. } => "TextUnit",
            NodePayload::Container { .. } => "Container",
            NodePayload::Event { .. } => "Event",
            NodePayload::Narrative { .. } => "Narrative",
            NodePayload::Place { .. } => "Place",
            NodePayload::Person { .. } => "Person",
            NodePayload::PeopleGroup { .. } => "PeopleGroup",
            NodePayload::Anchor { .. } => "Anchor",
            NodePayload::Era { .. } => "Era",
            NodePayload::Polity { .. } => "Polity",
            NodePayload::CatechismItem { .. } => "CatechismItem",
            NodePayload::CommentaryItem { .. } => "CommentaryItem",
            NodePayload::Source { .. } => "Source",
            NodePayload::Translation { .. } => "Translation",
        })
        .collect();
    seen.sort_unstable();
    seen.dedup();
    assert_eq!(seen.len(), 14, "every variant must appear: {seen:?}");

    for n in &nodes {
        round_trip(n);
    }
}

#[test]
fn polity_rings_are_two_element_arrays_in_tuple_order() {
    let n = Node {
        id: nid(NodeKind::Polity, "moab"),
        payload: NodePayload::Polity {
            label: "Moab".into(),
            color_key: 3,
            eras: vec![PolityEraPayload {
                name: "Moab".into(),
                from_year: -1400,
                to_year: -582,
                rings: vec![vec![(35.75, 31.5)]],
                ref_note: String::new(),
                transition: None,
                fall: None,
            }],
        },
        provenance: "crockett".into(),
    };
    let s = String::from_utf8(n.encode()).unwrap();
    assert!(s.contains(r#""rings":[[[35.75,31.5]]]"#), "got {s}");
}

#[test]
fn anchor_time_point_is_year_month_day_and_rejects_year_zero() {
    let n = Node {
        id: nid(NodeKind::Anchor, "creation"),
        payload: NodePayload::Anchor {
            at: TimePoint::year_only(Year::new(-4004).unwrap()),
            citation: "Ussher".into(),
        },
        provenance: "ussher".into(),
    };
    let s = String::from_utf8(n.encode()).unwrap();
    assert!(s.contains(r#""at":{"day":null,"month":null,"year":-4004}"#), "got {s}");

    let zero = br#"{"id":"Anchor:x","payload":{"Anchor":{"at":{"day":null,"month":null,"year":0},"citation":"c"}},"provenance":"p"}"#;
    assert_eq!(Node::decode(zero).unwrap_err().path, "payload.Anchor.at.year");

    let day_without_month = br#"{"id":"Anchor:x","payload":{"Anchor":{"at":{"day":3,"month":null,"year":-4004},"citation":"c"}},"provenance":"p"}"#;
    assert_eq!(Node::decode(day_without_month).unwrap_err().path, "payload.Anchor.at");
}
