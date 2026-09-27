//! Thin HTTP handlers: parse request params, call into `atlas_core`, wrap
//! the result in `Json`. No business logic lives here beyond response-shape
//! assembly and the out-of-canon policy documented at each handler that
//! needs one (controller ruling 3).

use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::Json;

use atlas_core::data::{AtlasData, CanonBook, Event};
use atlas_core::history::resolve_display_name;
use atlas_core::refs::{ScriptureRef, VerseId};
use atlas_core::scene::to_scene_event;
use atlas_core::xrefs::{aggregate_span_xrefs, AggregatedXref};
use atlas_graph::window::{self, WindowDir};
use atlas_graph::GraphService;
use atlas_graph_types::edge::{Direction, EdgeKind, RelationId};
use atlas_graph_types::id::Position;
use atlas_graph_types::store::GraphQuery;
use atlas_graph_types::text::VerseRef;

use crate::error::ApiError;
use crate::wire;

#[utoipa::path(get, path = "/api/books", responses((status = 200, body = Vec<atlas_core::data::CanonBook>)), tag = "reading")]
pub async fn books(State(data): State<Arc<AtlasData>>) -> Json<Vec<CanonBook>> {
    Json(data.canon.books.clone())
}

/// `GET /api/chapter/{cref}`. `cref` must parse as exactly a
/// `ScriptureRef::Chapter` (book + chapter, e.g. `EXO.14`) — a book-only or
/// verse/passage-shaped path segment is the wrong shape for this endpoint
/// and 400s as `bad_ref`, same as an unparseable one. The optional
/// `?translation=kjv` query param (ruling 5, M1 is KJV-only) is never
/// extracted, so its presence or absence cannot affect this handler at all.
///
/// ruling-3-policy: once `cref` parses as a `Chapter`, an out-of-range
/// chapter number (or a book with no known chapters in this atlas) is NOT an
/// error — the verse-count bound comes from `canon.books[].chapters`, and an
/// unknown/short chapter just yields `verse_count = 0`, i.e. a 200 response
/// with an empty `verses` list. Same rationale as `scene_scripture`: a
/// reader showing "no verses in this chapter" is a meaningful response, not
/// a failure.
///
/// Batch M-A (brief requirement 5, "re-implement the OLD /api/chapter
/// handler as a VIEW over the window query"): the verse TEXT below now
/// comes from `GraphState::chapter_span` + `GraphState::window` -- the SAME
/// windowed reading-order query `GET /api/text?scope=chapter` calls --
/// instead of `data.verses.get(key)`. The verse-count bound and the
/// out-of-canon policy above are UNCHANGED and still sourced from
/// `AtlasData`; headings moved to `graph.heading_index` in M-C2, and
/// OVERLAY-1 Task 5 moved the PLACE-MENTION half onto the port too
/// (`graph.scene_source(&data)`'s own `places_for_verse`/`place`, the
/// materialised-from-the-graph successors of the deleted
/// `AtlasData::places_for_verse`/`place_by_id` -- identical ids in
/// identical order, see those methods' own doc comments). THIS endpoint
/// (the reader's own chapter view) was untouched by Batch M-B's own
/// event-world migration;
/// only `/api/narrative/event/{id}` (see that handler's own doc comment)
/// and the generic `/api/node`/`/edges` endpoints move to the graph this
/// batch. The WIRE SHAPE is byte-for-byte identical -- proven by
/// `tests/graph_equivalence.rs`'s own all-1,189-chapters comparison -- so
/// every existing caller of this endpoint (the reader's chapter view/
/// mini-reader/split view, `ChapterNode`, `PlaceCard`'s hover verse text,
/// `PassageBlock`, `PopoverSectionProviders`) now serves from the graph
/// with NO client-side change and no reader-visible behavior change.
#[utoipa::path(get, path = "/api/chapter/{cref}", params(("cref" = String, Path)), responses((status = 200, body = wire::Chapter), ApiError), tag = "reading")]
pub async fn chapter(
    State(data): State<Arc<AtlasData>>,
    State(graph): State<Arc<GraphService>>,
    Path(cref): Path<String>,
) -> Result<Json<wire::Chapter>, ApiError> {
    let (book, chapter) = match ScriptureRef::parse(&cref) {
        Ok(ScriptureRef::Chapter { book, chapter }) => (book, chapter),
        _ => return Err(ApiError::bad_ref(&cref)),
    };
    let code = book.code();

    let verse_count = data
        .canon
        .books
        .iter()
        .find(|b| b.code == code)
        .and_then(|b| b.chapters.get((chapter - 1) as usize))
        .copied()
        .unwrap_or(0);

    // The graph's own view of this chapter's text, keyed by verse number --
    // empty (not an error) when the graph has no such chapter, mirroring
    // `data.verses.get(key)`'s own prior "absent means skip this verse"
    // tolerance below. Reached entirely through THE PORT (design doc §9a;
    // fix round 1, C1) -- `window::window`/`window::render` take only
    // `&dyn atlas_graph_types::store::GraphQuery`, never a concrete graph
    // struct; `chapter_span`/the opened snapshot are `GraphService`'s own
    // adapter-side companions (ref-resolution isn't part of the generic
    // port -- see that module's own doc comment).
    let snap = graph.snapshot();
    let graph_texts: HashMap<u16, String> = graph
        .chapter_span(book.0, chapter)
        .map(|(start, n)| {
            window::window(&snap, atlas_graph::kjv_adapter::BIBLE_CORPUS, start, n, WindowDir::Onward)
                .iter()
                .filter_map(|id| atlas_graph::kjv_adapter::decode_text_unit(id).map(|(_, _, v)| (v, window::render(&snap, id).unwrap_or_default())))
                .collect()
        })
        .unwrap_or_default();

    // OVERLAY-1 Task 5: the place-mention half's own source -- the
    // graph-backed scene source, resolved ONCE for the whole chapter rather
    // than per verse (it is a single `OnceLock` read, but hoisting it keeps
    // the hot loop below free of any repeated lookup).
    let scene_source = graph.scene_source(&data);

    let mut verses = Vec::new();
    for v in 1..=verse_count {
        let key = format!("{code}.{chapter}.{v}");
        if let Some(text) = graph_texts.get(&v) {
            let places = scene_source
                .places_for_verse(&key)
                .iter()
                .filter_map(|pid| scene_source.place(pid))
                .map(|p| wire::PlaceRef {
                    id: p.id.clone(),
                    // Batch E3: resolved (period-history/KJV-alias-aware)
                    // name, not the bare Theographic default -- this is the
                    // "reader place mentions" surface (PlaceMentions.cs's own
                    // plain-text substring scan against THIS field is the
                    // app's only mention-detection mechanism, so an unaliased
                    // name here means a place whose KJV wording differs from
                    // its default name is silently never detected as
                    // mentioned in its own verse's text at all -- exactly
                    // the owner's bug report, one layer deeper). No window
                    // (`None`) -- same "scripture mode never resolves a
                    // period name" reasoning `compose_scripture_scene` uses;
                    // a chapter reading has no time window either.
                    name: resolve_display_name(&p.name, data.place_history_for(&p.id), None, data.place_name_alias_for(&p.id)),
                })
                .collect();
            // M-D3 (owner ruling U5): the SAME O(1) per-verse lookup
            // treatment as `heading`/`xref_count` below, off the
            // precomputed `graph.persons_by_verse` companion -- see that
            // field's own doc comment.
            // DB-3: through the port (`GraphService::persons_at_verse`,
            // `edges_with_nodes` over `mentions`), the retired
            // `persons_by_verse` companion's exact answer.
            let persons = graph
                .persons_at_verse(book.0, chapter, v)
                .into_iter()
                .map(|(id, name)| wire::PersonRef { id, name })
                .collect();
            // M-C2 (requirement 1, decisive-title law re-homed as a graph
            // query): `graph.heading_index` (precomputed at `GraphService::
            // assemble` time by `heading::build_heading_index`), not
            // `data.heading_for_verse` -- see that module's own doc
            // comment for the full re-homing (kept in lockstep with
            // CONTRACT.md and the atlas-core original).
            let heading = graph.heading_index.get(&key).cloned();
            // Batch M-D2: the generic port, inline -- `Position::Node` +
            // `GraphQuery::edge_summary` are the EXACT calls
            // `graph::node_card` makes for `GET /api/node/{id}`;
            // reused here as a library call (not a second HTTP round trip
            // per verse) so a whole chapter's worth of superscript counts
            // ships in the ONE fetch the reader already makes. `cites` is
            // ALWAYS the Forward direction from a verse's own locus (a verse
            // citing others, not "who cites me" -- `cited-by` is the
            // Inverse reading of the SAME relation, unused here).
            let verse_pos = Position::Node(atlas_graph::kjv_adapter::verse_node_id(book.0, chapter, v));
            let xref_count =
                snap.edge_summary(&verse_pos).get(&EdgeKind::Directed(RelationId::Cites, Direction::Forward)).copied().unwrap_or(0);
            // Batch RED-1: the SAME O(1) per-verse lookup treatment
            // `heading`/`xref_count`/`persons` above already get, off the
            // precomputed `graph.red_letter_spans` companion.
            let words_of_christ = graph.red_letter_spans.get(&key).map(|spans| spans.iter().map(|&(start, end)| wire::WordsOfChristSpan { start, end }).collect()).unwrap_or_default();
            verses.push(wire::Verse { verse: v, text: text.to_string(), places, persons, heading, xref_count, words_of_christ });
        }
    }

    Ok(Json(wire::Chapter { sref: format!("{code}.{chapter}"), book: book.name().to_string(), chapter, verses }))
}

/// `GET /api/kretzmann/chapter/{cref}`. `cref` must parse as exactly a
/// `ScriptureRef::Chapter` (book + chapter, e.g. `PSA.119`) -- same 400
/// `bad_ref` convention as `GET /api/chapter/{cref}` for a book-only or
/// verse/passage-shaped path segment. Same ruling-3-policy as `chapter`
/// above: an out-of-range chapter number (or a book with no known chapters)
/// is NOT an error -- `verse_count` resolves to 0 and this 200s with an
/// empty `verses` list, never a 404 for "this chapter has no commentary."
///
/// Replaces `Kretzmann.razor`'s own retired client-side fan-out (one
/// `commented-on-by` edges HTTP call PER VERSE, concurrently -- 176
/// simultaneous requests on every locus change for a chapter like PSA 119)
/// with ONE request, computed by walking the SAME `commented-on-by` edge
/// machinery server-side, in-process, via `atlas_graph::kretzmann_adapter::
/// chapter_commentary` -- see that function's own doc comment for why this
/// is additive, not a types-crate or artifact change: the underlying
/// `CommentsOn`/`RelationId::CommentsOn` KRETZ-1 vocabulary is completely
/// unchanged, this is a new READ path over data the graph already carries.
#[utoipa::path(get, path = "/api/kretzmann/chapter/{cref}", params(("cref" = String, Path)), responses((status = 200, body = wire::KretzmannChapter), ApiError), tag = "reading")]
pub async fn kretzmann_chapter(
    State(data): State<Arc<AtlasData>>,
    State(graph): State<Arc<GraphService>>,
    Path(cref): Path<String>,
) -> Result<Json<wire::KretzmannChapter>, ApiError> {
    let (book, chapter) = match ScriptureRef::parse(&cref) {
        Ok(ScriptureRef::Chapter { book, chapter }) => (book, chapter),
        _ => return Err(ApiError::bad_ref(&cref)),
    };
    let code = book.code();

    let verse_count = data
        .canon
        .books
        .iter()
        .find(|b| b.code == code)
        .and_then(|b| b.chapters.get((chapter - 1) as usize))
        .copied()
        .unwrap_or(0);

    let snap = graph.snapshot();
    let rows = atlas_graph::kretzmann_adapter::chapter_commentary(&snap, book.0, chapter, verse_count);

    // `chapter_commentary` already returns rows verse-ascending with every
    // verse's own items grouped consecutively (its own doc comment) -- this
    // loop just folds that flat Vec into the wire's own nested shape,
    // never re-sorting or re-grouping independently.
    let mut verses: Vec<wire::KretzmannChapterVerse> = Vec::new();
    for row in rows {
        let item = wire::KretzmannChapterItem { id: crate::graph_wire::encode_node_id(&row.item_id), heading: row.heading };
        match verses.last_mut() {
            Some(v) if v.verse == row.verse => v.items.push(item),
            _ => verses.push(wire::KretzmannChapterVerse { verse: row.verse, items: vec![item] }),
        }
    }

    Ok(Json(wire::KretzmannChapter { verses, version: atlas_graph::version_hex(graph.version()) }))
}

/// `GET /api/verse/{vref}`. `vref` must parse as exactly a
/// `ScriptureRef::Verse` (`VerseId::parse_canonical` enforces this) — any
/// other shape 400s as `bad_ref`.
///
/// ruling-3-policy: unlike the scene/chapter endpoints, a structurally valid
/// vref whose text is absent from this atlas's compiled KJV map is 404
/// `not_found`, not a 200-with-placeholder. A single verse is an
/// individually-addressed resource (like `/api/place/{id}`), not a
/// list/scene that can be gracefully empty — there is no non-misleading way
/// to represent "this verse doesn't exist" other than "not found", so this
/// endpoint intentionally follows `/api/place/{id}`'s precedent rather than
/// `scene_scripture`'s/`chapter`'s "out-of-canon is still 200" policy.
///
/// Cross-ref preview rows fail soft (ruling 4): ETL guarantees every
/// compiled cross-ref target's first verse exists in the verses map, but if
/// that's ever violated the row is skipped rather than panicking.
#[utoipa::path(get, path = "/api/verse/{vref}", params(("vref" = String, Path)), responses((status = 200, body = wire::VerseDetail), ApiError), tag = "reading")]
pub async fn verse(State(data): State<Arc<AtlasData>>, State(graph): State<Arc<GraphService>>, Path(vref): Path<String>) -> Result<Json<wire::VerseDetail>, ApiError> {
    let vid = VerseId::parse_canonical(&vref).map_err(|_| ApiError::bad_ref(&vref))?;
    let canonical = format!("{}.{}.{}", vid.book.code(), vid.chapter, vid.verse);

    // M-C2 (definitive surface list): the verse's own text now comes from
    // the graph's own TextUnit node -- the SAME `window::render` primitive
    // `reading::chapter` already uses -- not `data.verses.get(key)`.
    let snap = graph.snapshot();
    let text_id = atlas_graph::kjv_adapter::verse_node_id(vid.book.0, vid.chapter, vid.verse);
    let text = window::render(&snap, &text_id).ok_or_else(|| ApiError::not_found("verse"))?;
    // Batch PROV-1: the verse's own attribution, off the SAME node
    // `window::render` just read. `window::render` returning `Some` means
    // the node exists, so this lookup cannot legitimately miss -- but it is
    // written fail-loud rather than defaulted: an unattributed verse is a
    // 500 naming the id, never a silent blank at the reader (the fail-loud
    // law -- "never a silent blank and never a fabricated label").
    //
    // FIX ROUND 1 (review H-1): the node-absence guard was already here, but
    // a node carrying a BLANK provenance string slipped through it. The
    // `filter` closes that, so "no PROV-1 wire field is ever empty" is one
    // uniform rule across all four of them rather than three-quarters of one
    // -- `graph_api.rs::no_provenance_field_the_wire_serves_is_ever_blank`
    // is the standing assertion of it.
    let provenance = snap
        .node(&text_id)
        .map(|n| n.provenance)
        .filter(|p| !p.trim().is_empty())
        .ok_or_else(|| ApiError::internal(&format!("verse {canonical} rendered text with no node to attribute it to")))?;

    let book_meta = data.books_meta.iter().find(|b| b.book == vid.book.code()).cloned().unwrap_or_else(|| atlas_core::data::BookMeta {
        book: vid.book.code().to_string(),
        author: String::new(),
        write_place: None,
        write_from: None,
        write_to: None,
    });

    // M-C2: "which events attest this verse" is now the INVERSE
    // `attested-in` frontier at the verse's own TextUnit position -- real
    // per-verse `attests` rows (event_world.rs's own M-C2 fix, one row per
    // witness VERSE rather than per verse GROUP) close the exact gap that
    // would otherwise drop a witness-interior verse like `MAT.26.6`.
    // Deduped by event id (an event whose OWN top-level `verses` and a
    // witness both happen to name the identical verse must still surface
    // once, mirroring `AtlasData::finish()`'s own `verse_to_events`
    // dedup); sorted by `from_year` to match that index's own inherited
    // (from `self.events`'s own from_year-sorted order) iteration order.
    let mut seen_events: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut attesting_events: Vec<Event> = drain_edges(&snap, &Position::Node(text_id.clone()), EdgeKind::Directed(RelationId::Attests, Direction::Inverse))
        .into_iter()
        .filter_map(|entry| match entry.node {
            Position::Node(eid) => Some(eid),
            Position::Edge(_) => None,
        })
        .filter(|eid| seen_events.insert(eid.raw.clone()))
        .filter_map(|eid| atlas_graph::legacy::event_from_node(&eid, &snap, &graph.chronology.chrono))
        .collect();
    attesting_events.sort_by_key(|e| e.when.from_year);
    let events: Vec<wire::VerseEvent> = attesting_events
        .into_iter()
        .map(|e| {
            // Batch PROV-1: this row's own attribution -- the Event NODE's
            // provenance, off the same snapshot.
            //
            // FIX ROUND 1 (review H-1, HIGH). This read used to end in
            // `.unwrap_or_default()`, and three comments (here, at
            // `EventAnalogue` below, and in `client/Dtos.cs`) justified
            // the empty string on the ground that the client renders it as
            // a LOUD unresolved notice. THAT WAS FALSE: every client path
            // to `ProvenanceResolver.Resolve` filtered whitespace ids out
            // FIRST, so a blank provenance rendered as no "?" at all --
            // the exact silent blank requirement 3 forbids, and exactly how
            // the ATTEST-1 leper row hid in the first place. Both halves
            // are fixed: the client no longer filters an id it was HANDED
            // (a blank one now renders the loud notice), and the wire can
            // no longer emit the blank at all. `ApiError::internal` is the
            // same answer `events::event` already gives for the event's
            // own node provenance -- the resource exists, our data about it
            // is incomplete, which is our bug and not the caller's.
            let node_provenance = snap
                .node(&atlas_graph::event_world::event_node_id(&e.id))
                .map(|n| n.provenance)
                .filter(|p| !p.trim().is_empty())
                .ok_or_else(|| ApiError::internal(&format!("event {} has no provenance to attribute this membership row to", e.id)))?;
            let se = to_scene_event(&e);
            let when = if e.kind == "event" { Some(se.when) } else { None };
            Ok(wire::VerseEvent {
                id: se.id,
                label: se.label,
                when,
                verse_groups: se.verse_groups,
                places: e.places.clone(),
                kind: e.kind.clone(),
                provenance: node_provenance,
            })
        })
        .collect::<Result<Vec<_>, ApiError>>()?;

    // M-C2 (requirement 2): `graph.cross_refs_by_from` (the graph's own
    // `cites` rows, dot-ref keyed, `target` = each row's own
    // `target_display` -- the honest original citation string) instead of
    // `data.cross_refs` -- see that field's own doc comment.
    //
    // OVERLAY-1 Task 2: the preview TEXT itself now comes from
    // `graph.verse_text_of` -- a real, on-demand graph query (one node
    // lookup per row), not the retired `graph.verse_text` whole-spine
    // companion, and not `data.verses`. Same fail-soft behavior as before
    // (a missing preview skips the row, per this endpoint's own "ruling 4"
    // doc comment above): only the DATA SOURCE moved.
    // PROV-1 FIX ROUND 1 (review M-3): the family set, read ONCE off the
    // load-time companion index and cloned onto each element, so the two
    // endpoints that serve `CrossRef` say the identical thing about the
    // identical rows and a PASSAGE node (served by the bare-array endpoint,
    // which has no envelope) can carry a "?" at all.
    //
    // FIX ROUND 2 (review M-NEW-1) -- THE GRANULARITY, STATED CORRECTLY.
    // This comment used to end "so the rows INSIDE `VerseDetail` are
    // attributed per row, not only per section." THAT WAS FALSE, and the
    // field's own DTO doc comment (`CrossRef.provenance`) had it right
    // all along: this is SECTION-level attribution DUPLICATED onto each
    // element for transport, not per-row attribution.
    //
    // And per-row is not available here to be had. Measured, not assumed:
    // `graph.cross_refs_by_from` is `HashMap<String,
    // Vec<atlas_core::data::CrossRef>>` (`service.rs`), and
    // `atlas_core::data::CrossRef` (`data.rs`) carries `target` and `votes`
    // and NOTHING ELSE -- the graph-types row that does carry
    // `provenance: ProvenanceId` (`edge.rs`) has it projected away before it
    // reaches this map. Serving the family set is the honest available
    // answer; claiming it is per-row is not. Getting real per-row values
    // would mean widening the companion index, which is the contract-shaped
    // decision this batch deliberately refused, and it must not be implied
    // to have already happened.
    let cross_refs_provenance = graph.provenance.by_family(atlas_graph::provenance::family::CROSS_REFS);
    // DB-4c: a seek on the kjv section's `xref_by_from` (or the Mem arm's
    // retained map), never a whole-corpus companion.
    let by_from = graph.cross_refs_for_span(&ScriptureRef::Verse(vid));
    let cross_refs: Vec<wire::CrossRef> = by_from
        .get(&canonical)
        .map(Vec::as_slice)
        .unwrap_or(&[])
        .iter()
        .filter_map(|cr| {
            let first = first_verse_of_target(&cr.target)?;
            let preview = graph.verse_text_of(&VerseRef { book: first.book.0, chapter: first.chapter, verse: first.verse })?;
            Some(wire::CrossRef::attributed(AggregatedXref { target: cr.target.clone(), votes: cr.votes, preview }, &cross_refs_provenance))
        })
        .collect();

    // Batch F: same aggregation core `catechism::catechism_for_span` uses for
    // a passage, called here with a single-verse span -- see
    // `VerseDetail.catechism`'s own doc comment for why this is folded
    // into the already-shared verse-detail fetch rather than a second call.
    let catechism_provenance = graph.provenance.by_family(atlas_graph::provenance::family::CATECHISM);
    let catechism: Vec<wire::CatechismRef> = data
        .catechism_items_for_span(&ScriptureRef::Verse(vid))
        .into_iter()
        .map(|c| wire::CatechismRef::attributed(c, &catechism_provenance))
        .collect();

    // Batch RED-1: the SAME per-verse lookup `reading::chapter` uses, off
    // the precomputed `graph.red_letter_spans` companion.
    let words_of_christ: Vec<wire::WordsOfChristSpan> = graph.red_letter_spans.get(&canonical).map(|spans| spans.iter().map(|&(start, end)| wire::WordsOfChristSpan { start, end }).collect()).unwrap_or_default();

    Ok(Json(wire::VerseDetail {
        sref: canonical,
        text,
        words_of_christ,
        book_meta,
        events,
        cross_refs,
        catechism,
        provenance,
        // Batch PROV-1: the two section-level attributions, straight off
        // the load-time companion index -- no scan, no fetch, nothing added
        // to the per-request path (the sub-100ms frontier law binds here).
        // FIX ROUND 1: the same two values now also ride each `cross_refs`/
        // `catechism` ELEMENT (review M-3), which is what lets a PASSAGE
        // node -- served by the bare-array endpoints -- carry a "?" too.
        // These section fields stay for the wire-compatibility they always
        // had; nothing about them moved.
        cross_refs_provenance,
        catechism_provenance,
    }))
}

/// `GET /api/xrefs/{sref}` (batch-g1-brief.md requirement 2, "passage
/// context -- passages give xrefs, not just geo"). `sref` must parse as
/// exactly a `ScriptureRef::Verse` or `ScriptureRef::Passage` -- the brief's
/// own two given examples, `GEN.1.1` and `GEN.1.1-5`, are read as an
/// exhaustive pair (a single verse or a same-chapter span) rather than a
/// representative sample of every `ScriptureRef` shape: a bare book or
/// chapter ref (`GEN`, `GEN.1`) has no defined "member verses" for this
/// endpoint to aggregate over, so both 400 as `bad_ref`, the same typed
/// error every other ref-shaped endpoint already uses (requirement 2:
/// "Typed errors (bad_ref) unchanged").
///
/// ruling-3-policy: unlike `/api/verse/{vref}`, an sref with no recorded
/// cross-references at all -- including one naming a verse outside this
/// atlas's compiled canon -- is NOT an error: 200 with an empty list, the
/// same "gracefully empty, never a 404" policy `scene_scripture`/`chapter`
/// already follow. This falls out of the aggregation itself needing no
/// special-casing: `aggregate_span_xrefs` only ever reads `cross_refs` by
/// key and calls `verse_text` by key, and a key simply absent/`None`
/// contributes nothing, which is exactly as true for a real, canonical
/// verse with zero curated cross-references (the overwhelmingly common
/// case) as for an out-of-canon one.
///
/// Business logic (the union-and-sum aggregation, self-target drop, sort,
/// cap-at-20) lives in `atlas_core::xrefs::aggregate_span_xrefs` -- this
/// handler is pure response-shape assembly, per this module's own file
/// header.
/// OVERLAY-1 Task 2: `aggregate_span_xrefs`'s own preview-text parameter
/// is now `impl Fn(&str) -> Option<String>`, not `&HashMap<String,
/// String>` -- `atlas_core` still has no `graph-types` dependency of its
/// own (the closure type crosses the boundary, not a graph type), and the
/// aggregation logic itself is unchanged. `graph.cross_refs_by_from` (the
/// graph's own `cites` rows -- `target` carries each row's own
/// `target_display`, the honest original citation string, graph_types::
/// edge::CrossRef's own M-C2 widening) still supplies the rows; the
/// preview text now comes from `graph.verse_text_of` called per candidate
/// key, on demand, instead of the retired `graph.verse_text` whole-spine
/// companion. `AtlasData` is still not read anywhere in this handler.
#[utoipa::path(get, path = "/api/xrefs/{sref}", params(("sref" = String, Path)), responses((status = 200, body = Vec<wire::CrossRef>), ApiError), tag = "reading")]
pub async fn xrefs(State(graph): State<Arc<GraphService>>, Path(sref): Path<String>) -> Result<Json<Vec<wire::CrossRef>>, ApiError> {
    let span = match ScriptureRef::parse(&sref) {
        Ok(span @ (ScriptureRef::Verse(_) | ScriptureRef::Passage { .. })) => span,
        _ => return Err(ApiError::bad_ref(&sref)),
    };

    let by_from = graph.cross_refs_for_span(&span);
    let aggregated = aggregate_span_xrefs(&span, &by_from, |key| {
        let v = VerseId::parse_canonical(key).ok()?;
        graph.verse_text_of(&VerseRef { book: v.book.0, chapter: v.chapter, verse: v.verse })
    });
    // PROV-1 FIX ROUND 1 (review M-3): one read off the load-time companion
    // index, cloned per row -- no scan, no fetch. See
    // `CrossRef.provenance` for why the value is the family SET.
    let provenance = graph.provenance.by_family(atlas_graph::provenance::family::CROSS_REFS);
    let out = aggregated
        .into_iter()
        .map(|x| wire::CrossRef::attributed(x, &provenance))
        .collect();
    Ok(Json(out))
}

/// Mirrors atlas-etl's private `xrefs::first_verse_of_target` (duplicated,
/// not shared, because atlas-contract does not and should not depend on
/// atlas-etl — that crate is a build-time-only ETL binary, not a runtime
/// library). Extracts the first verse id referenced by an
/// already-canonicalized cross-ref target string, which is either a single
/// verse (`"PSA.124.8"`), a same-chapter span (`"COL.1.16-19"`), or a
/// cross-chapter/book span (`"MAT.5.3-MAT.6.2"`).
fn first_verse_of_target(target: &str) -> Option<VerseId> {
    if let Ok(v) = VerseId::parse_canonical(target) {
        return Some(v);
    }
    if let Ok(ScriptureRef::Passage { book, chapter, from_verse, .. }) = ScriptureRef::parse(target) {
        return Some(VerseId { book, chapter, verse: from_verse });
    }
    let (left, _right) = target.split_once('-')?;
    VerseId::parse_canonical(left).ok()
}

/// Drains every page of one edge kind at one position -- the SAME
/// cursor-loop shape `atlas_graph_types::store`'s own (private) conformance
/// harness uses internally, needed here because a handler-side consumer
/// (unlike the generic `/api/node/{id}/edges` endpoint, which hands back
/// exactly one page) sometimes genuinely needs the WHOLE frontier of one
/// kind (e.g. every narrative an event is a leg of) rather than one honest
/// page of it.
pub(crate) fn drain_edges(
    snap: &impl atlas_graph_types::store::GraphQuery,
    p: &atlas_graph_types::id::Position,
    kind: atlas_graph_types::edge::EdgeKind,
) -> Vec<atlas_graph_types::explore::EdgeEntry> {
    let mut cursor = None;
    let mut out = Vec::new();
    loop {
        let page = snap.edges(p, &atlas_graph_types::explore::EdgeQuery { kind, cursor, limit: 200 });
        out.extend(page.entries);
        match page.next {
            Some(c) => cursor = Some(c),
            None => break,
        }
    }
    out
}

pub fn routes() -> utoipa_axum::router::OpenApiRouter<crate::app::AppState> {
    use utoipa_axum::routes;
    utoipa_axum::router::OpenApiRouter::new()
        .routes(routes!(books))
        .routes(routes!(chapter))
        .routes(routes!(kretzmann_chapter))
        .routes(routes!(verse))
        .routes(routes!(xrefs))
}
