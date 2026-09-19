//! `Canon for Node`: the node encoding.
//!
//! Shape: `{"id":"Place:jerusalem","payload":{"Place":{…}},"provenance":"…"}`.
//! The payload is a one-key enum object whose key is the `NodePayload`
//! variant name and whose members are the Rust field names -- so the
//! encoding reads like the type, and adding a variant is a compile error
//! here until it is encoded.
//!
//! Every object is CLOSED (ruling R9): each decoder declares its exact
//! member set through `expect_exact_keys`, so a key nobody asked for is a
//! located error rather than something silently dropped. Decoding starts
//! at the shared `ROOT`, so a failure reads as one continuous trail --
//! `$.payload.Place.lat` -- from the byte parser straight through to the
//! field that refused.
//!
//! Every arm is mechanical on purpose. The interesting decisions live in
//! the small helpers below: `corpus_from_value` (a `&'static str` field
//! can only decode to one of the two known literals), `time_point_from_value`
//! (the `Year` and `TimePoint` invariants are re-checked on the way in, so
//! a bad artifact cannot smuggle year zero or a day without a month past
//! the constructors), and `number` (f64 members).

use std::collections::BTreeMap;

use crate::chrono::{TimePoint, Year};
use crate::id::SourceId;
use crate::node::{
    EventWitnessPayload, Node, NodePayload, PolityDeltaPayload, PolityEraPayload,
};
use crate::text::{BibleTag, ConcordTag, Corpus, LayerMap, TranslationId};

use super::ids::{any_node_id_str, parse_any_node_id};
use super::{
    expect_arr, expect_exact_keys, expect_obj, expect_str, expect_variant, field, field_arr,
    field_f64, field_i32, field_obj, field_bool, field_opt_i32, field_opt_str, field_opt_u8, field_str,
    field_u8, field_vec_str, join, obj, opt_i32, opt_str, opt_u8, str_value, variant, vec_str,
    Canon, CanonError, Value, ROOT,
};

/// The member set of every closed object in the node encoding, named once
/// so the encoder and the decoder cannot drift apart.
const NODE_KEYS: &[&str] = &["id", "payload", "provenance"];
const TEXT_UNIT_KEYS: &[&str] = &["corpus", "renderings"];
const CONTAINER_KEYS: &[&str] = &["title"];
const EVENT_KEYS: &[&str] = &[
    "acts_section",
    "atlas_section",
    "kind",
    "kjv_superscription",
    "label",
    "ref_note",
    "robertson_section",
    "verses",
    "witnesses",
];
const NARRATIVE_KEYS: &[&str] = &["color", "label"];
const PLACE_KEYS: &[&str] = &["aliases", "canonical", "description", "lat", "lon"];
const PERSON_KEYS: &[&str] = &[
    "also_called",
    "birth_year",
    "death_year",
    "description",
    "eternal",
    "eternal_grounds",
    "first_year",
    "gender",
    "label",
    "last_year",
];
const PEOPLE_GROUP_KEYS: &[&str] = &["description", "label"];
const ANCHOR_KEYS: &[&str] = &["at", "citation"];
const ERA_KEYS: &[&str] = &["from_year", "label", "to_year"];
const POLITY_KEYS: &[&str] = &["color_key", "eras", "label"];
const LABEL_ONLY_KEYS: &[&str] = &["label"];
const COMMENTARY_ITEM_KEYS: &[&str] = &["heading", "text", "work"];
const LEXICON_ENTRY_KEYS: &[&str] =
    &["domains", "glosses", "lang", "lemma", "pos", "root", "senses", "strong", "translit"];
const WITNESS_KEYS: &[&str] = &["book", "ref_note", "robertson_section", "translations"];
const DELTA_KEYS: &[&str] = &["event", "ref_note", "verses"];
const POLITY_ERA_KEYS: &[&str] =
    &["fall", "from_year", "name", "ref_note", "rings", "to_year", "transition"];
const TIME_POINT_KEYS: &[&str] = &["day", "month", "year"];

impl Canon for Node {
    fn to_value(&self) -> Value {
        obj(vec![
            ("id", str_value(&any_node_id_str(&self.id))),
            ("payload", payload_to_value(&self.payload)),
            ("provenance", str_value(&self.provenance)),
        ])
    }

    fn from_value(v: &Value) -> Result<Self, CanonError> {
        let m = expect_obj(v, ROOT)?;
        expect_exact_keys(m, ROOT, NODE_KEYS)?;
        let id_str = field_str(m, ROOT, "id")?;
        let id = parse_any_node_id(&id_str, &join(ROOT, "id"))?;
        let (payload_value, payload_path) = field(m, ROOT, "payload")?;
        let payload = payload_from_value(payload_value, &payload_path)?;
        let provenance = field_str(m, ROOT, "provenance")?;
        Ok(Node { id, payload, provenance })
    }
}

// ------------------------------------------------------------------- payload

fn payload_to_value(p: &NodePayload) -> Value {
    match p {
        NodePayload::TextUnit { corpus, renderings } => variant(
            "TextUnit",
            obj(vec![
                ("corpus", str_value(corpus)),
                ("renderings", layer_map_to_value(renderings)),
            ]),
        ),
        NodePayload::Container { title } => {
            variant("Container", obj(vec![("title", str_value(title))]))
        }
        NodePayload::Event {
            label,
            kind,
            verses,
            witnesses,
            robertson_section,
            acts_section,
            atlas_section,
            kjv_superscription,
            ref_note,
        } => variant(
            "Event",
            obj(vec![
                ("acts_section", opt_str(acts_section)),
                ("atlas_section", opt_str(atlas_section)),
                ("kind", str_value(kind)),
                ("kjv_superscription", opt_str(kjv_superscription)),
                ("label", str_value(label)),
                ("ref_note", opt_str(ref_note)),
                ("robertson_section", opt_str(robertson_section)),
                ("verses", vec_str(verses)),
                ("witnesses", Value::Arr(witnesses.iter().map(witness_to_value).collect())),
            ]),
        ),
        NodePayload::Narrative { label, color } => variant(
            "Narrative",
            obj(vec![("color", str_value(color)), ("label", str_value(label))]),
        ),
        NodePayload::Place { canonical, lat, lon, aliases, description } => variant(
            "Place",
            obj(vec![
                ("aliases", vec_str(aliases)),
                ("canonical", str_value(canonical)),
                ("description", opt_str(description)),
                ("lat", number(*lat)),
                ("lon", number(*lon)),
            ]),
        ),
        NodePayload::Person {
            label,
            gender,
            birth_year,
            death_year,
            also_called,
            description,
            first_year,
            last_year,
            eternal,
            eternal_grounds,
        } => variant(
            "Person",
            obj(vec![
                ("also_called", vec_str(also_called)),
                ("birth_year", opt_i32(birth_year)),
                ("death_year", opt_i32(death_year)),
                ("description", opt_str(description)),
                ("eternal", Value::Bool(*eternal)),
                ("eternal_grounds", vec_str(eternal_grounds)),
                ("first_year", opt_i32(first_year)),
                ("gender", opt_str(gender)),
                ("label", str_value(label)),
                ("last_year", opt_i32(last_year)),
            ]),
        ),
        NodePayload::PeopleGroup { label, description } => variant(
            "PeopleGroup",
            obj(vec![("description", opt_str(description)), ("label", str_value(label))]),
        ),
        NodePayload::Anchor { at, citation } => variant(
            "Anchor",
            obj(vec![("at", time_point_to_value(at)), ("citation", str_value(citation))]),
        ),
        NodePayload::Era { label, from_year, to_year } => variant(
            "Era",
            obj(vec![
                ("from_year", Value::Int(i64::from(*from_year))),
                ("label", str_value(label)),
                ("to_year", Value::Int(i64::from(*to_year))),
            ]),
        ),
        NodePayload::Polity { label, color_key, eras } => variant(
            "Polity",
            obj(vec![
                ("color_key", Value::Int(i64::from(*color_key))),
                ("eras", Value::Arr(eras.iter().map(polity_era_to_value).collect())),
                ("label", str_value(label)),
            ]),
        ),
        NodePayload::CatechismItem { label } => {
            variant("CatechismItem", obj(vec![("label", str_value(label))]))
        }
        NodePayload::CommentaryItem { work, heading, text } => variant(
            "CommentaryItem",
            obj(vec![
                ("heading", opt_str(heading)),
                ("text", str_value(text)),
                ("work", str_value(&work.0)),
            ]),
        ),
        NodePayload::LexiconEntry { strong, lang, lemma, translit, pos, glosses, senses, domains, root } => variant(
            "LexiconEntry",
            obj(vec![
                ("domains", vec_str(domains)),
                ("glosses", vec_str(glosses)),
                ("lang", str_value(lang)),
                ("lemma", str_value(lemma)),
                ("pos", opt_str(pos)),
                ("root", opt_str(root)),
                ("senses", vec_str(senses)),
                ("strong", str_value(strong)),
                ("translit", opt_str(translit)),
            ]),
        ),
        NodePayload::Source { label } => {
            variant("Source", obj(vec![("label", str_value(label))]))
        }
        NodePayload::Translation { label } => {
            variant("Translation", obj(vec![("label", str_value(label))]))
        }
    }
}

fn payload_from_value(v: &Value, path: &str) -> Result<NodePayload, CanonError> {
    let (name, body) = expect_variant(v, path)?;
    let p = join(path, name);
    let m = expect_obj(body, &p)?;
    match name {
        "TextUnit" => {
            expect_exact_keys(m, &p, TEXT_UNIT_KEYS)?;
            Ok(NodePayload::TextUnit {
                corpus: corpus_from_value(m, &p)?,
                renderings: layer_map_from_value(m, &p)?,
            })
        }
        "Container" => {
            expect_exact_keys(m, &p, CONTAINER_KEYS)?;
            Ok(NodePayload::Container { title: field_str(m, &p, "title")? })
        }
        "Event" => {
            expect_exact_keys(m, &p, EVENT_KEYS)?;
            let (witnesses, wp) = field_arr(m, &p, "witnesses")?;
            Ok(NodePayload::Event {
                label: field_str(m, &p, "label")?,
                kind: field_str(m, &p, "kind")?,
                verses: field_vec_str(m, &p, "verses")?,
                witnesses: witnesses
                    .iter()
                    .enumerate()
                    .map(|(i, w)| witness_from_value(w, &join(&wp, &i.to_string())))
                    .collect::<Result<Vec<_>, _>>()?,
                robertson_section: field_opt_str(m, &p, "robertson_section")?,
                acts_section: field_opt_str(m, &p, "acts_section")?,
                atlas_section: field_opt_str(m, &p, "atlas_section")?,
                kjv_superscription: field_opt_str(m, &p, "kjv_superscription")?,
                ref_note: field_opt_str(m, &p, "ref_note")?,
            })
        }
        "Narrative" => {
            expect_exact_keys(m, &p, NARRATIVE_KEYS)?;
            Ok(NodePayload::Narrative {
                label: field_str(m, &p, "label")?,
                color: field_str(m, &p, "color")?,
            })
        }
        "Place" => {
            expect_exact_keys(m, &p, PLACE_KEYS)?;
            Ok(NodePayload::Place {
                canonical: field_str(m, &p, "canonical")?,
                lat: field_f64(m, &p, "lat")?,
                lon: field_f64(m, &p, "lon")?,
                aliases: field_vec_str(m, &p, "aliases")?,
                description: field_opt_str(m, &p, "description")?,
            })
        }
        "Person" => {
            expect_exact_keys(m, &p, PERSON_KEYS)?;
            Ok(NodePayload::Person {
                label: field_str(m, &p, "label")?,
                gender: field_opt_str(m, &p, "gender")?,
                birth_year: field_opt_i32(m, &p, "birth_year")?,
                death_year: field_opt_i32(m, &p, "death_year")?,
                also_called: field_vec_str(m, &p, "also_called")?,
                description: field_opt_str(m, &p, "description")?,
                first_year: field_opt_i32(m, &p, "first_year")?,
                last_year: field_opt_i32(m, &p, "last_year")?,
                eternal: field_bool(m, &p, "eternal")?,
                eternal_grounds: field_vec_str(m, &p, "eternal_grounds")?,
            })
        }
        "PeopleGroup" => {
            expect_exact_keys(m, &p, PEOPLE_GROUP_KEYS)?;
            Ok(NodePayload::PeopleGroup {
                label: field_str(m, &p, "label")?,
                description: field_opt_str(m, &p, "description")?,
            })
        }
        "Anchor" => {
            expect_exact_keys(m, &p, ANCHOR_KEYS)?;
            let (at, at_path) = field_obj(m, &p, "at")?;
            Ok(NodePayload::Anchor {
                at: time_point_from_value(at, &at_path)?,
                citation: field_str(m, &p, "citation")?,
            })
        }
        "Era" => {
            expect_exact_keys(m, &p, ERA_KEYS)?;
            Ok(NodePayload::Era {
                label: field_str(m, &p, "label")?,
                from_year: field_i32(m, &p, "from_year")?,
                to_year: field_i32(m, &p, "to_year")?,
            })
        }
        "Polity" => {
            expect_exact_keys(m, &p, POLITY_KEYS)?;
            let (eras, ep) = field_arr(m, &p, "eras")?;
            Ok(NodePayload::Polity {
                label: field_str(m, &p, "label")?,
                color_key: field_u8(m, &p, "color_key")?,
                eras: eras
                    .iter()
                    .enumerate()
                    .map(|(i, e)| polity_era_from_value(e, &join(&ep, &i.to_string())))
                    .collect::<Result<Vec<_>, _>>()?,
            })
        }
        "CatechismItem" => {
            expect_exact_keys(m, &p, LABEL_ONLY_KEYS)?;
            Ok(NodePayload::CatechismItem { label: field_str(m, &p, "label")? })
        }
        "CommentaryItem" => {
            expect_exact_keys(m, &p, COMMENTARY_ITEM_KEYS)?;
            Ok(NodePayload::CommentaryItem {
                work: SourceId::new(field_str(m, &p, "work")?),
                heading: field_opt_str(m, &p, "heading")?,
                text: field_str(m, &p, "text")?,
            })
        }
        "LexiconEntry" => {
            expect_exact_keys(m, &p, LEXICON_ENTRY_KEYS)?;
            Ok(NodePayload::LexiconEntry {
                strong: field_str(m, &p, "strong")?,
                lang: field_str(m, &p, "lang")?,
                lemma: field_str(m, &p, "lemma")?,
                translit: field_opt_str(m, &p, "translit")?,
                pos: field_opt_str(m, &p, "pos")?,
                glosses: field_vec_str(m, &p, "glosses")?,
                senses: field_vec_str(m, &p, "senses")?,
                domains: field_vec_str(m, &p, "domains")?,
                root: field_opt_str(m, &p, "root")?,
            })
        }
        "Source" => {
            expect_exact_keys(m, &p, LABEL_ONLY_KEYS)?;
            Ok(NodePayload::Source { label: field_str(m, &p, "label")? })
        }
        "Translation" => {
            expect_exact_keys(m, &p, LABEL_ONLY_KEYS)?;
            Ok(NodePayload::Translation { label: field_str(m, &p, "label")? })
        }
        other => Err(CanonError::new(p, format!("unknown payload variant `{other}`"))),
    }
}

// ------------------------------------------------------------------- helpers

/// An f64 member. `NodePayload`'s doubles are finite by construction
/// upstream; a non-finite one has NO canonical spelling, so it encodes as
/// `null` and `expect_f64` then refuses it BY PATH on the way back in --
/// a located decode error instead of a silent corruption. (`to_value` has
/// no `Result` to return, which is why the check lands here.)
fn number(f: f64) -> Value {
    Value::float(f).unwrap_or(Value::Null)
}

/// `TextUnit.corpus` is a `&'static str`, so decoding cannot mint one:
/// the string must match a corpus literal the binary already owns. Those
/// are `Corpus::ID` for the two corpora (`BibleTag`, `ConcordTag`).
///
/// FINAL REVIEW item 9: it now says that in code as well as in prose --
/// the arms ARE `BibleTag::ID`/`ConcordTag::ID`, not two copies of their
/// text, so a corpus that renamed its `ID` could not leave a stale
/// literal behind here. The consts are exactly `"bible"` and `"concord"`,
/// so the bytes do not move; `canon_row_vectors`/`canon_vectors`'
/// existing goldens are the proof.
fn corpus_from_value(
    m: &BTreeMap<String, Value>,
    path: &str,
) -> Result<&'static str, CanonError> {
    let s = field_str(m, path, "corpus")?;
    match s.as_str() {
        BibleTag::ID => Ok(BibleTag::ID),
        ConcordTag::ID => Ok(ConcordTag::ID),
        other => Err(CanonError::new(
            join(path, "corpus"),
            format!(
                "unknown corpus `{other}`; the known corpora are `{}` and `{}`",
                BibleTag::ID,
                ConcordTag::ID
            ),
        )),
    }
}

/// A `LayerMap` is an OPEN map -- its keys are translation ids, not a
/// fixed schema -- so `expect_exact_keys` deliberately does not apply
/// here. Same for a witness's `translations`.
fn layer_map_to_value(map: &LayerMap) -> Value {
    Value::Obj(map.iter().map(|(k, v)| (k.0.clone(), Value::Str(v.clone()))).collect())
}

fn layer_map_from_value(
    m: &BTreeMap<String, Value>,
    path: &str,
) -> Result<LayerMap, CanonError> {
    let (members, p) = field_obj(m, path, "renderings")?;
    members
        .iter()
        .map(|(k, v)| Ok((TranslationId(k.clone()), expect_str(v, &join(&p, k))?)))
        .collect()
}

fn time_point_to_value(t: &TimePoint) -> Value {
    obj(vec![
        ("day", opt_u8(&t.day)),
        ("month", opt_u8(&t.month)),
        ("year", Value::Int(i64::from(t.year.get()))),
    ])
}

fn time_point_from_value(
    m: &BTreeMap<String, Value>,
    path: &str,
) -> Result<TimePoint, CanonError> {
    expect_exact_keys(m, path, TIME_POINT_KEYS)?;
    let raw_year = field_i32(m, path, "year")?;
    // Both constructors are re-run, not bypassed: no year zero, no day
    // without a month, however the bytes were written.
    let year = Year::new(raw_year).map_err(|e| {
        CanonError::new(join(path, "year"), format!("invalid year {raw_year}: {e:?}"))
    })?;
    let month = field_opt_u8(m, path, "month")?;
    let day = field_opt_u8(m, path, "day")?;
    TimePoint::new(year, month, day)
        .map_err(|e| CanonError::new(path, format!("invalid time point: {e:?}")))
}

fn witness_to_value(w: &EventWitnessPayload) -> Value {
    obj(vec![
        ("book", str_value(&w.book)),
        ("ref_note", opt_str(&w.ref_note)),
        ("robertson_section", opt_str(&w.robertson_section)),
        (
            "translations",
            Value::Obj(
                w.translations.iter().map(|(k, v)| (k.clone(), vec_str(v))).collect(),
            ),
        ),
    ])
}

fn witness_from_value(v: &Value, path: &str) -> Result<EventWitnessPayload, CanonError> {
    let m = expect_obj(v, path)?;
    expect_exact_keys(m, path, WITNESS_KEYS)?;
    let (translations, tp) = field_obj(m, path, "translations")?;
    Ok(EventWitnessPayload {
        book: field_str(m, path, "book")?,
        translations: translations
            .iter()
            .map(|(k, val)| {
                let vp = join(&tp, k);
                let arr = expect_arr(val, &vp)?;
                let verses = arr
                    .iter()
                    .enumerate()
                    .map(|(i, s)| expect_str(s, &join(&vp, &i.to_string())))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok((k.clone(), verses))
            })
            .collect::<Result<BTreeMap<_, _>, CanonError>>()?,
        ref_note: field_opt_str(m, path, "ref_note")?,
        robertson_section: field_opt_str(m, path, "robertson_section")?,
    })
}

fn polity_delta_to_value(d: &PolityDeltaPayload) -> Value {
    obj(vec![
        ("event", str_value(&d.event)),
        ("ref_note", str_value(&d.ref_note)),
        ("verses", vec_str(&d.verses)),
    ])
}

fn polity_delta_from_value(v: &Value, path: &str) -> Result<PolityDeltaPayload, CanonError> {
    let m = expect_obj(v, path)?;
    expect_exact_keys(m, path, DELTA_KEYS)?;
    Ok(PolityDeltaPayload {
        event: field_str(m, path, "event")?,
        verses: field_vec_str(m, path, "verses")?,
        ref_note: field_str(m, path, "ref_note")?,
    })
}

fn opt_delta_from_value(
    m: &BTreeMap<String, Value>,
    path: &str,
    key: &str,
) -> Result<Option<PolityDeltaPayload>, CanonError> {
    let (v, p) = field(m, path, key)?;
    match v {
        Value::Null => Ok(None),
        _ => Ok(Some(polity_delta_from_value(v, &p)?)),
    }
}

fn polity_era_to_value(e: &PolityEraPayload) -> Value {
    obj(vec![
        ("fall", e.fall.as_ref().map_or(Value::Null, polity_delta_to_value)),
        ("from_year", Value::Int(i64::from(e.from_year))),
        ("name", str_value(&e.name)),
        ("ref_note", str_value(&e.ref_note)),
        // A ring point is the tuple `(f64, f64)` -- a 2-element array in
        // the tuple's OWN order, never reordered into lon/lat.
        (
            "rings",
            Value::Arr(
                e.rings
                    .iter()
                    .map(|ring| {
                        Value::Arr(
                            ring.iter()
                                .map(|(a, b)| Value::Arr(vec![number(*a), number(*b)]))
                                .collect(),
                        )
                    })
                    .collect(),
            ),
        ),
        ("to_year", Value::Int(i64::from(e.to_year))),
        ("transition", e.transition.as_ref().map_or(Value::Null, polity_delta_to_value)),
    ])
}

fn polity_era_from_value(v: &Value, path: &str) -> Result<PolityEraPayload, CanonError> {
    let m = expect_obj(v, path)?;
    expect_exact_keys(m, path, POLITY_ERA_KEYS)?;
    let (rings, rp) = field_arr(m, path, "rings")?;
    Ok(PolityEraPayload {
        name: field_str(m, path, "name")?,
        from_year: field_i32(m, path, "from_year")?,
        to_year: field_i32(m, path, "to_year")?,
        rings: rings
            .iter()
            .enumerate()
            .map(|(i, ring)| ring_from_value(ring, &join(&rp, &i.to_string())))
            .collect::<Result<Vec<_>, _>>()?,
        ref_note: field_str(m, path, "ref_note")?,
        transition: opt_delta_from_value(m, path, "transition")?,
        fall: opt_delta_from_value(m, path, "fall")?,
    })
}

fn ring_from_value(v: &Value, path: &str) -> Result<Vec<(f64, f64)>, CanonError> {
    expect_arr(v, path)?
        .iter()
        .enumerate()
        .map(|(i, point)| {
            let pp = join(path, &i.to_string());
            let coords = expect_arr(point, &pp)?;
            match coords.as_slice() {
                [a, b] => Ok((
                    super::expect_f64(a, &join(&pp, "0"))?,
                    super::expect_f64(b, &join(&pp, "1"))?,
                )),
                other => Err(CanonError::new(
                    pp,
                    format!("a ring point is a 2-element array, found {}", other.len()),
                )),
            }
        })
        .collect()
}
