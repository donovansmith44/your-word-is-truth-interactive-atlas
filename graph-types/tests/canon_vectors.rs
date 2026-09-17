//! DB-2a golden vectors for the canonical JSON core.
//!
//! These are the LAW for the on-disk node encoding: the byte strings
//! asserted here are pinned, not descriptive. `std` only -- `graph-types`
//! is a zero-dependency crate and its tests keep that promise.

use atlas_graph_types::canon::{
    at_path, expect_exact_keys, parse, serialize, Canon, CanonError, Value, CANON_VERSION,
    DOMAIN_PREFIX, ROOT,
};
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
        r#"{"a":"x\"y\\z\u000a","b":2,"c":[null,true]}"#
    );
}

#[test]
fn every_control_character_uses_the_six_character_escape() {
    // Ruling R5: the ONLY two-character escapes are \" and \\. Every C0
    // control -- the five JSON gives short escapes included -- goes out as
    // \u00xx in LOWERCASE hex, so one character has exactly one spelling.
    // Nothing else is escaped: '/' and all non-ASCII ride raw as UTF-8.
    let v = Value::Str("\u{8}\t\n\u{c}\r\u{0}\u{1f}\u{b}/\u{e9}".into());
    assert_eq!(
        String::from_utf8(serialize(&v)).unwrap(),
        "\"\\u0008\\u0009\\u000a\\u000c\\u000d\\u0000\\u001f\\u000b/\u{e9}\""
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
        // R5: the two-character control escapes are a SECOND spelling of a
        // character that already has one, so the parser refuses them.
        (b"\"\\n\"", "two-character newline escape"),
        (b"\"\\t\"", "two-character tab escape"),
        (b"\"\\b\"", "two-character backspace escape"),
        (b"\"\\f\"", "two-character form-feed escape"),
        (b"\"\\r\"", "two-character carriage-return escape"),
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

/// `[[[…1…]]]` nested `depth` levels deep.
fn nested(depth: usize) -> Vec<u8> {
    let mut v = vec![b'['; depth];
    v.push(b'1');
    v.extend(std::iter::repeat(b']').take(depth));
    v
}

#[test]
fn parse_rejects_deep_nesting_without_panicking() {
    // A runaway `[` never reaches the end of input, so this would be a
    // blown stack rather than an `Err` if the depth cap were missing.
    let runaway: Vec<u8> = std::iter::repeat(b'[').take(100_000).collect();
    assert!(parse(&runaway).is_err(), "depth limit, not a stack overflow");
}

#[test]
fn the_depth_limit_sits_exactly_at_64() {
    // The boundary is pinned so the cap cannot drift silently: 64 levels
    // is the deepest value the artifact may carry.
    assert!(parse(&nested(64)).is_ok(), "64 levels must parse");
    let err = parse(&nested(65)).unwrap_err();
    assert!(!err.path.is_empty(), "the depth error must be located: {err:?}");
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
    assert_eq!(parse_any_node_id("TextUnit:bible/JHN.3.16", "$.id").unwrap(), id);
    let weird = AnyNodeId { kind: NodeKind::Container, raw: "a:b:c".into() };
    assert_eq!(parse_any_node_id(&any_node_id_str(&weird), "$.id").unwrap(), weird);
    // R10b: the caller's path rides into the error, both for a bad kind
    // and for a missing separator.
    assert_eq!(parse_any_node_id("Nope:x", "$.id").unwrap_err().path, "$.id");
    assert_eq!(parse_any_node_id("Place", "$.subject").unwrap_err().path, "$.subject");
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
        NodeKind::LexiconEntry,
    ];
    assert_eq!(all, NodeKind::ALL, "NodeKind::ALL is the one list; this array mirrors it");
    for k in all {
        assert_eq!(node_kind_str(k), format!("{k:?}"));
        assert_eq!(parse_node_kind(node_kind_str(k), "$.kind").unwrap(), k);
    }
    assert_eq!(parse_node_kind("Nope", "$.kind").unwrap_err().path, "$.kind");
}

#[test]
fn positions_round_trip_for_nodes_and_edges() {
    use atlas_graph_types::canon::ids::*;
    use atlas_graph_types::id::Position;
    use atlas_graph_types::EdgeId;
    let n = Position::Node(AnyNodeId { kind: NodeKind::Place, raw: "jerusalem".into() });
    assert_eq!(position_str(&n), "n:Place:jerusalem");
    assert_eq!(parse_position(&position_str(&n), "$.subject").unwrap(), n);
    let e = Position::Edge(EdgeId("located_at:00ff".into()));
    assert_eq!(position_str(&e), "e:located_at:00ff");
    assert_eq!(parse_position(&position_str(&e), "$.subject").unwrap(), e);
    for bad in ["x:Place:jerusalem", "n:Nope:x", "nope"] {
        assert_eq!(parse_position(bad, "$.subject").unwrap_err().path, "$.subject");
    }
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

/// Every malformed node this file feeds `Node::decode`. Each row is
/// `(bytes, expected path)`; the path is asserted exactly, and the
/// non-empty law is asserted over the whole table.
const MALFORMED_NODES: &[(&[u8], &str)] = &[
    (
        br#"{"id":"Place:jerusalem","payload":{"Place":{"aliases":["Salem"],"canonical":"Jerusalem","description":null,"lat":"nope","lon":35.2345}},"provenance":"p"}"#,
        "$.payload.Place.lat",
    ),
    (br#"{"id":"Place:jerusalem","payload":{"Nope":{}},"provenance":"p"}"#, "$.payload.Nope"),
    (br#"{"id":"Place:jerusalem","provenance":"p"}"#, "$.payload"),
    (br#"{"id":"Nope:jerusalem","payload":{"Source":{"label":"x"}},"provenance":"p"}"#, "$.id"),
    (
        br#"{"id":"TextUnit:x/1.1.1","payload":{"TextUnit":{"corpus":"vulgate","renderings":{}}},"provenance":"p"}"#,
        "$.payload.TextUnit.corpus",
    ),
    // R9: unknown members, at the node, in a payload, and in a sub-object.
    (
        br#"{"extra":1,"id":"Place:jerusalem","payload":{"Source":{"label":"x"}},"provenance":"p"}"#,
        "$.extra",
    ),
    (
        br#"{"id":"Source:s","payload":{"Source":{"label":"x","note":"nope"}},"provenance":"p"}"#,
        "$.payload.Source.note",
    ),
    (
        br#"{"id":"Anchor:x","payload":{"Anchor":{"at":{"day":null,"era":"AM","month":null,"year":-4004},"citation":"c"}},"provenance":"p"}"#,
        "$.payload.Anchor.at.era",
    ),
    // R10c: even a root-level type failure is located.
    (b"null", ROOT),
];

#[test]
fn node_decode_reports_the_failing_path() {
    for (bytes, expected) in MALFORMED_NODES {
        let err = Node::decode(bytes).unwrap_err();
        assert_eq!(&err.path, expected, "for {}", String::from_utf8_lossy(bytes));
    }
}

#[test]
fn no_node_decode_error_ever_carries_an_empty_path() {
    // R10c: the byte parser and the field decoders share one root, so
    // there is no way to get an error that cannot say where it happened.
    for (bytes, _) in MALFORMED_NODES {
        let err = Node::decode(bytes).unwrap_err();
        assert!(!err.path.is_empty(), "empty path for {}", String::from_utf8_lossy(bytes));
        assert!(err.path.starts_with(ROOT), "path must start at the root: {err:?}");
    }
    // Errors raised by the byte parser, before any decoder sees a value.
    for bytes in [&b"{"[..], b"{\"a\":1, \"b\":2}", b"01", b""] {
        let err = Node::decode(bytes).unwrap_err();
        assert!(!err.path.is_empty(), "empty path for {}", String::from_utf8_lossy(bytes));
    }
}

#[test]
fn unknown_corpus_is_rejected_because_corpus_is_a_static_str() {
    let bad = br#"{"id":"TextUnit:x/1.1.1","payload":{"TextUnit":{"corpus":"vulgate","renderings":{}}},"provenance":"p"}"#;
    let err = Node::decode(bad).unwrap_err();
    assert_eq!(err.path, "$.payload.TextUnit.corpus");
}

#[test]
fn a_non_finite_coordinate_encodes_as_null_and_fails_to_decode_by_path() {
    // R7: `to_value` has no Result, so a non-finite double -- which has no
    // canonical spelling -- goes out as `null` and is refused BY PATH on
    // the way back in. A located error, never a silent bad coordinate.
    let n = Node {
        id: AnyNodeId { kind: NodeKind::Place, raw: "nowhere".into() },
        payload: NodePayload::Place {
            canonical: "Nowhere".into(),
            lat: f64::NAN,
            lon: 35.2345,
            aliases: vec![],
            description: None,
        },
        provenance: "p".into(),
    };
    let bytes = n.encode();
    let text = String::from_utf8(bytes.clone()).unwrap();
    assert!(text.contains(r#""lat":null"#), "got {text}");
    assert_eq!(Node::decode(&bytes).unwrap_err().path, "$.payload.Place.lat");
}

#[test]
fn unknown_members_are_rejected_rather_than_ignored() {
    // R9 at the helper level -- Task 2 reuses this for the row families.
    let mut m = BTreeMap::new();
    m.insert("label".to_string(), Value::Str("x".into()));
    assert!(expect_exact_keys(&m, "$.payload.Source", &["label"]).is_ok());
    m.insert("note".to_string(), Value::Int(1));
    let err = expect_exact_keys(&m, "$.payload.Source", &["label"]).unwrap_err();
    assert_eq!(err.path, "$.payload.Source.note");
    // A missing key is NOT this helper's business; `field_*` reports those.
    assert!(expect_exact_keys(&BTreeMap::new(), "$", &["label"]).is_ok());
}

#[test]
fn at_path_prefixes_and_never_drops_the_nested_location() {
    // R10a: re-rooting a nested error keeps where it happened.
    let inner = CanonError::new("$.eras.0.rings", "bad ring");
    assert_eq!(at_path("$.payload.Polity", inner).path, "$.payload.Polity.eras.0.rings");
    // A root-level inner error contributes nothing but is not lost.
    let root_level = CanonError::new(ROOT, "expected object");
    assert_eq!(at_path("$.payload", root_level).path, "$.payload");
}

#[test]
fn the_canon_version_and_domain_prefix_are_pinned() {
    // Bumping either is a breaking change to every stored artifact, so
    // both are pinned here rather than left to drift.
    assert_eq!(CANON_VERSION, 1);
    assert_eq!(DOMAIN_PREFIX, b"bible-atlas/canon/1\n");
    let prefix = std::str::from_utf8(DOMAIN_PREFIX).unwrap();
    assert!(prefix.ends_with('\n'), "the prefix must terminate: {prefix:?}");
    assert!(
        prefix.contains(&CANON_VERSION.to_string()),
        "the prefix must embed the version it separates: {prefix:?}"
    );
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
        // 15. LexiconEntry (DB-3, spec 7.2) -- every optional Some, every list non-empty.
        Node {
            id: nid(NodeKind::LexiconEntry, "G3056"),
            payload: NodePayload::LexiconEntry {
                strong: "G3056".into(),
                lang: "grc".into(),
                lemma: "logos".into(),
                translit: Some("logos".into()),
                pos: Some("noun".into()),
                glosses: vec!["word".into(), "speech".into()],
                senses: vec!["a word, uttered by a living voice".into()],
                domains: vec!["33.98".into(), "33.99".into()],
                root: Some("G3004".into()),
            },
            provenance: "stepbible-tbesg".into(),
        },
        // 15b. LexiconEntry -- every optional None, every list empty.
        Node {
            id: nid(NodeKind::LexiconEntry, "H0430"),
            payload: NodePayload::LexiconEntry {
                strong: "H0430".into(),
                lang: "hbo".into(),
                lemma: "elohim".into(),
                translit: None,
                pos: None,
                glosses: vec![],
                senses: vec![],
                domains: vec![],
                root: None,
            },
            provenance: "stepbible-tahot".into(),
        },
    ];

    // All fifteen NodePayload variants are exercised; the extra rows are
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
            NodePayload::LexiconEntry { .. } => "LexiconEntry",
        })
        .collect();
    seen.sort_unstable();
    seen.dedup();
    assert_eq!(seen.len(), 15, "every variant must appear: {seen:?}");

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
    assert_eq!(Node::decode(zero).unwrap_err().path, "$.payload.Anchor.at.year");

    let day_without_month = br#"{"id":"Anchor:x","payload":{"Anchor":{"at":{"day":3,"month":null,"year":-4004},"citation":"c"}},"provenance":"p"}"#;
    assert_eq!(Node::decode(day_without_month).unwrap_err().path, "$.payload.Anchor.at");
}

#[test]
fn the_node_goldens_stay_control_character_free() {
    // R5 moved the control-character spelling, and the node goldens did
    // not move with it -- because none of them contains a control
    // character. This test is what keeps that true: if a future golden
    // gains one, it must be re-pinned deliberately, not by accident.
    let goldens: &[&[u8]] = &[
        br#"{"id":"Place:jerusalem","payload":{"Place":{"aliases":["Salem"],"canonical":"Jerusalem","description":null,"lat":31.7767,"lon":35.2345}},"provenance":"openbible-geo"}"#,
        br#""rings":[[[35.75,31.5]]]"#,
        br#""at":{"day":null,"month":null,"year":-4004}"#,
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

/// DB-3 (spec 4, 7.2-7.3): the lexicon vocabulary is present and its bytes
/// are pinned, while nothing inhabits it yet.
#[test]
fn the_lexicon_entry_vocabulary_is_present_and_pinned() {
    use atlas_graph_types::canon::ids::*;
    use atlas_graph_types::canon::Canon;
    use atlas_graph_types::edge::RelationId;
    assert_eq!(node_kind_str(NodeKind::LexiconEntry), "LexiconEntry");
    assert_eq!(parse_node_kind("LexiconEntry", "$.kind").unwrap(), NodeKind::LexiconEntry);
    // Appended LAST among directed relations: `edge_index.rel` codes are positional.
    assert_eq!(RelationId::ALL.last().copied(), Some(RelationId::Occurs));
    assert_eq!(RelationId::Occurs.forward_label(), "occurs-in");
    assert_eq!(RelationId::Occurs.inverse_label(), "words");
    assert_eq!(RelationId::ALL.len(), 18);
    assert_eq!(NodeKind::ALL.len(), 15);
    assert_eq!(NodeKind::ALL.last().copied(), Some(NodeKind::LexiconEntry));
    let full = Node {
        id: nid(NodeKind::LexiconEntry, "G3056"),
        payload: NodePayload::LexiconEntry {
            strong: "G3056".into(),
            lang: "grc".into(),
            lemma: "logos".into(),
            translit: Some("logos".into()),
            pos: Some("noun".into()),
            glosses: vec!["word".into(), "speech".into()],
            senses: vec!["a word, uttered by a living voice".into()],
            domains: vec!["33.98".into(), "33.99".into()],
            root: Some("G3004".into()),
        },
        provenance: "stepbible-tbesg".into(),
    };
    assert_eq!(
        String::from_utf8(full.encode()).unwrap(),
        r#"{"id":"LexiconEntry:G3056","payload":{"LexiconEntry":{"domains":["33.98","33.99"],"glosses":["word","speech"],"lang":"grc","lemma":"logos","pos":"noun","root":"G3004","senses":["a word, uttered by a living voice"],"strong":"G3056","translit":"logos"}},"provenance":"stepbible-tbesg"}"#
    );
    round_trip(&full);
    let bare = Node {
        id: nid(NodeKind::LexiconEntry, "H0430"),
        payload: NodePayload::LexiconEntry {
            strong: "H0430".into(),
            lang: "hbo".into(),
            lemma: "elohim".into(),
            translit: None,
            pos: None,
            glosses: vec![],
            senses: vec![],
            domains: vec![],
            root: None,
        },
        provenance: "stepbible-tahot".into(),
    };
    round_trip(&bare);
    assert!(String::from_utf8(bare.encode()).unwrap().contains(r#""root":null,"senses":[],"strong":"H0430","translit":null"#));
}
