//! DB-2b: the per-section DDL, verbatim from spec §5.1 and §5.3–5.6, and
//! the section -> row-table map: the common tables, every row family's
//! table (+ `_locus` / `_step` sub-tables) and `reading_spine`. DB-4b: the
//! node projections, `event_date`, `heading_index`, `red_letter_span` and
//! the folded sidecars (`extra_ddl`/`extra_index_ddl`; column specs in
//! `sqlite::extras`/`sqlite::sidecars`, pinned against this DDL by
//! `sqlite_laws.rs`; amendments to spec §5.3 in the DB-4b plan's judgment
//! calls 3–6).
//!
//! Spec §5.0: no `FOREIGN KEY`, no `CHECK`, no triggers -- the compiler
//! proves the laws, the file only describes the shape (a unit test below
//! pins that). Indexes are created AFTER the inserts (spec §6.1), hence
//! the table/index split in every constant pair.

use atlas_graph_types::canon::RowFamily;
use rusqlite::Connection;

use super::{stamp_pragmas, SqliteError};
// DB-4a: the section -> table map lives in graph-types (it is part of the
// version root); re-exported here for the sqlite module's callers.
pub use crate::sections::{has_spine, logical_table_order, row_tables_of};
use crate::sections::Section;

/// Spec §5.1 minus its three `CREATE … INDEX` lines (see `COMMON_INDEX_DDL`).
pub const COMMON_DDL: &str = "
CREATE TABLE meta (
  key   TEXT PRIMARY KEY,
  value TEXT NOT NULL
) WITHOUT ROWID;

CREATE TABLE node (
  id         TEXT    PRIMARY KEY,
  kind       INTEGER NOT NULL,
  pid        BLOB    NOT NULL,
  label      TEXT,
  provenance TEXT    NOT NULL,
  payload    BLOB    NOT NULL
) WITHOUT ROWID;

CREATE TABLE justification (
  id   INTEGER PRIMARY KEY,
  text TEXT
);
CREATE TABLE ground (
  justification_id INTEGER NOT NULL,
  ord              INTEGER NOT NULL,
  kind             INTEGER NOT NULL,
  scr_from_corpus TEXT, scr_from_a INTEGER, scr_from_b INTEGER, scr_from_c INTEGER,
  scr_from_layer TEXT, scr_from_start INTEGER, scr_from_end INTEGER,
  scr_to_corpus TEXT, scr_to_a INTEGER, scr_to_b INTEGER, scr_to_c INTEGER,
  scr_to_layer TEXT, scr_to_start INTEGER, scr_to_end INTEGER,
  anchor_id TEXT,
  source_id TEXT,
  PRIMARY KEY (justification_id, ord)
) WITHOUT ROWID;

CREATE TABLE edge_index (
  subject        TEXT    NOT NULL,
  rel            INTEGER NOT NULL,
  dir            INTEGER NOT NULL,
  ord            INTEGER NOT NULL,
  object         TEXT    NOT NULL,
  edge_id        BLOB    NOT NULL,
  meta_kind      INTEGER NOT NULL,
  meta_narrative TEXT,
  meta_votes     INTEGER,
  row_family     INTEGER NOT NULL,
  row_id         INTEGER NOT NULL,
  PRIMARY KEY (subject, rel, dir, ord)
) WITHOUT ROWID;
";

/// Spec §5.1's three indexes -- created after the inserts. `edge_by_id`
/// is NOT unique, deviating from the spec's text (ruled in DB-2b, spec
/// erratum): a symmetric entry is stored once under EACH end with the
/// same `(edge_id, dir = 2)` (spec §5.2's own shape), and a directed
/// relation whose rows mint one id twice (identical `(rel, subject,
/// object)`) keeps both entries in memory today; a unique index would
/// refuse both. `row_provenance` reads the first hit either way.
pub const COMMON_INDEX_DDL: &str = "
CREATE INDEX node_by_kind ON node (kind, id);
CREATE UNIQUE INDEX node_by_pid ON node (pid);
CREATE INDEX edge_by_id ON edge_index (edge_id, dir);
";

/// Spec §5.4 / §5.5: `reading_spine` (kjv: corpus 'bible'; concord: 'concord').
pub const SPINE_DDL: &str = "
CREATE TABLE reading_spine (
  ord INTEGER PRIMARY KEY, node_id TEXT NOT NULL
);
";
pub const SPINE_INDEX_DDL: &str = "
CREATE UNIQUE INDEX spine_by_node ON reading_spine (node_id);
";

// ---------------------------------------------------------------------
// Row families (spec §5.3–5.6), one table constant + one index constant
// each. The RANGE families spell all 14 columns exactly as the spec does.
// ---------------------------------------------------------------------

const DDL_CONTAINS_BIBLE: &str = "
CREATE TABLE contains_bible (
  id INTEGER PRIMARY KEY, ord INTEGER NOT NULL,
  container_id TEXT NOT NULL,
  child_container_id TEXT,
  provenance TEXT NOT NULL, justification_id INTEGER
);
CREATE TABLE contains_bible_locus (
  contains_id INTEGER NOT NULL, ord INTEGER NOT NULL,
  a INTEGER NOT NULL, b INTEGER NOT NULL, c INTEGER NOT NULL, layer TEXT, start INTEGER, end_ INTEGER,
  PRIMARY KEY (contains_id, ord)
) WITHOUT ROWID;
";
const IDX_CONTAINS_BIBLE: &str = "CREATE UNIQUE INDEX contains_bible_ord ON contains_bible (ord);";

const DDL_CONTAINS_CONCORD: &str = "
CREATE TABLE contains_concord (
  id INTEGER PRIMARY KEY, ord INTEGER NOT NULL, container_id TEXT NOT NULL, child_container_id TEXT,
  provenance TEXT NOT NULL, justification_id INTEGER
);
CREATE TABLE contains_concord_locus (
  contains_id INTEGER NOT NULL, ord INTEGER NOT NULL,
  a INTEGER NOT NULL, b INTEGER NOT NULL, c INTEGER NOT NULL, layer TEXT, start INTEGER, end_ INTEGER,
  PRIMARY KEY (contains_id, ord)
) WITHOUT ROWID;
";
const IDX_CONTAINS_CONCORD: &str = "CREATE UNIQUE INDEX contains_concord_ord ON contains_concord (ord);";

const DDL_ATTESTS: &str = "
CREATE TABLE attests (
  id INTEGER PRIMARY KEY, ord INTEGER NOT NULL,
  event_id TEXT NOT NULL,
  att_from_corpus TEXT NOT NULL, att_from_a INTEGER NOT NULL, att_from_b INTEGER NOT NULL, att_from_c INTEGER NOT NULL,
  att_from_layer TEXT, att_from_start INTEGER, att_from_end INTEGER,
  att_to_corpus TEXT NOT NULL, att_to_a INTEGER NOT NULL, att_to_b INTEGER NOT NULL, att_to_c INTEGER NOT NULL,
  att_to_layer TEXT, att_to_start INTEGER, att_to_end INTEGER,
  provenance TEXT NOT NULL, justification_id INTEGER
);
";
const IDX_ATTESTS: &str = "
CREATE UNIQUE INDEX attests_ord ON attests (ord);
CREATE INDEX attests_by_from ON attests (att_from_a, att_from_b, att_from_c);
";

const DDL_SUCCESSION: &str = "
CREATE TABLE succession (
  id INTEGER PRIMARY KEY, ord INTEGER NOT NULL,
  narrative_id TEXT NOT NULL,
  provenance TEXT NOT NULL, justification_id INTEGER
);
CREATE TABLE succession_step (
  succession_id INTEGER NOT NULL, ord INTEGER NOT NULL, event_id TEXT NOT NULL,
  PRIMARY KEY (succession_id, ord)
) WITHOUT ROWID;
";
const IDX_SUCCESSION: &str = "CREATE UNIQUE INDEX succession_ord ON succession (ord);";

const DDL_CANON_SUCCESSION: &str = "
CREATE TABLE canon_succession (
  id INTEGER PRIMARY KEY, ord INTEGER NOT NULL, prior_id TEXT NOT NULL, next_id TEXT NOT NULL,
  provenance TEXT NOT NULL, justification_id INTEGER
);
";
const IDX_CANON_SUCCESSION: &str = "CREATE UNIQUE INDEX canon_succession_ord ON canon_succession (ord);";

const DDL_DATED_BY: &str = "
CREATE TABLE dated_by (
  id INTEGER PRIMARY KEY, ord INTEGER NOT NULL,
  event_id TEXT NOT NULL,
  placement_kind INTEGER NOT NULL,
  anchor_id TEXT, prior_event_id TEXT, era_id TEXT,
  years INTEGER, months INTEGER, days INTEGER,
  year_of_reign INTEGER,
  basis INTEGER NOT NULL,
  provenance TEXT NOT NULL, justification_id INTEGER
);
";
const IDX_DATED_BY: &str = "CREATE UNIQUE INDEX dated_by_ord ON dated_by (ord);";

const DDL_LOCATED_AT: &str = "
CREATE TABLE located_at (
  id INTEGER PRIMARY KEY, ord INTEGER NOT NULL,
  event_id TEXT NOT NULL, place_id TEXT NOT NULL,
  provenance TEXT NOT NULL, justification_id INTEGER
);
";
const IDX_LOCATED_AT: &str = "
CREATE UNIQUE INDEX located_at_ord ON located_at (ord);
CREATE INDEX located_at_by_place ON located_at (place_id, ord);
";

const DDL_FULFILLS: &str = "
CREATE TABLE fulfills (
  id INTEGER PRIMARY KEY, ord INTEGER NOT NULL,
  prophecy_from_corpus TEXT NOT NULL, prophecy_from_a INTEGER NOT NULL, prophecy_from_b INTEGER NOT NULL, prophecy_from_c INTEGER NOT NULL, prophecy_from_layer TEXT, prophecy_from_start INTEGER, prophecy_from_end INTEGER,
  prophecy_to_corpus TEXT NOT NULL, prophecy_to_a INTEGER NOT NULL, prophecy_to_b INTEGER NOT NULL, prophecy_to_c INTEGER NOT NULL, prophecy_to_layer TEXT, prophecy_to_start INTEGER, prophecy_to_end INTEGER,
  fulfillment_from_corpus TEXT NOT NULL, fulfillment_from_a INTEGER NOT NULL, fulfillment_from_b INTEGER NOT NULL, fulfillment_from_c INTEGER NOT NULL, fulfillment_from_layer TEXT, fulfillment_from_start INTEGER, fulfillment_from_end INTEGER,
  fulfillment_to_corpus TEXT NOT NULL, fulfillment_to_a INTEGER NOT NULL, fulfillment_to_b INTEGER NOT NULL, fulfillment_to_c INTEGER NOT NULL, fulfillment_to_layer TEXT, fulfillment_to_start INTEGER, fulfillment_to_end INTEGER,
  provenance TEXT NOT NULL, justification_id INTEGER
);
";
const IDX_FULFILLS: &str = "CREATE UNIQUE INDEX fulfills_ord ON fulfills (ord);";

const DDL_TYPOLOGY: &str = "
CREATE TABLE typology (
  id INTEGER PRIMARY KEY, ord INTEGER NOT NULL,
  type_from_corpus TEXT NOT NULL, type_from_a INTEGER NOT NULL, type_from_b INTEGER NOT NULL, type_from_c INTEGER NOT NULL, type_from_layer TEXT, type_from_start INTEGER, type_from_end INTEGER,
  type_to_corpus TEXT NOT NULL, type_to_a INTEGER NOT NULL, type_to_b INTEGER NOT NULL, type_to_c INTEGER NOT NULL, type_to_layer TEXT, type_to_start INTEGER, type_to_end INTEGER,
  antitype_from_corpus TEXT NOT NULL, antitype_from_a INTEGER NOT NULL, antitype_from_b INTEGER NOT NULL, antitype_from_c INTEGER NOT NULL, antitype_from_layer TEXT, antitype_from_start INTEGER, antitype_from_end INTEGER,
  antitype_to_corpus TEXT NOT NULL, antitype_to_a INTEGER NOT NULL, antitype_to_b INTEGER NOT NULL, antitype_to_c INTEGER NOT NULL, antitype_to_layer TEXT, antitype_to_start INTEGER, antitype_to_end INTEGER,
  note TEXT,
  provenance TEXT NOT NULL, justification_id INTEGER
);
";
const IDX_TYPOLOGY: &str = "CREATE UNIQUE INDEX typology_ord ON typology (ord);";

const DDL_NAMED_AFTER: &str = "
CREATE TABLE named_after (
  id INTEGER PRIMARY KEY, ord INTEGER NOT NULL,
  namesake_kind INTEGER NOT NULL,
  namesake_id TEXT NOT NULL, eponym_id TEXT NOT NULL,
  provenance TEXT NOT NULL, justification_id INTEGER
);
";
const IDX_NAMED_AFTER: &str = "CREATE UNIQUE INDEX named_after_ord ON named_after (ord);";

const DDL_CATECHISM: &str = "
CREATE TABLE catechism (
  id INTEGER PRIMARY KEY, ord INTEGER NOT NULL,
  locus_corpus TEXT NOT NULL, locus_a INTEGER NOT NULL, locus_b INTEGER NOT NULL, locus_c INTEGER NOT NULL, locus_layer TEXT, locus_start INTEGER, locus_end INTEGER,
  item_id TEXT NOT NULL,
  provenance TEXT NOT NULL, justification_id INTEGER
);
";
const IDX_CATECHISM: &str = "
CREATE UNIQUE INDEX catechism_ord ON catechism (ord);
CREATE INDEX catechism_by_locus ON catechism (locus_a, locus_b, locus_c);
";

const DDL_MENTIONS: &str = "
CREATE TABLE mentions (
  id INTEGER PRIMARY KEY, ord INTEGER NOT NULL,
  locus_corpus TEXT NOT NULL, locus_a INTEGER NOT NULL, locus_b INTEGER NOT NULL, locus_c INTEGER NOT NULL, locus_layer TEXT, locus_start INTEGER, locus_end INTEGER,
  entity_kind INTEGER NOT NULL,
  entity_id TEXT NOT NULL,
  provenance TEXT NOT NULL
);
";
const IDX_MENTIONS: &str = "
CREATE UNIQUE INDEX mentions_ord ON mentions (ord);
CREATE INDEX mentions_by_locus ON mentions (locus_a, locus_b, locus_c, ord);
CREATE INDEX mentions_by_entity ON mentions (entity_id, ord);
";

const DDL_CORRESPONDS_BIBLE: &str = "
CREATE TABLE corresponds_bible (
  id INTEGER PRIMARY KEY, ord INTEGER NOT NULL,
  a_corpus TEXT NOT NULL, a_a INTEGER NOT NULL, a_b INTEGER NOT NULL, a_c INTEGER NOT NULL, a_layer TEXT, a_start INTEGER, a_end INTEGER,
  b_corpus TEXT NOT NULL, b_a INTEGER NOT NULL, b_b INTEGER NOT NULL, b_c INTEGER NOT NULL, b_layer TEXT, b_start INTEGER, b_end INTEGER,
  provenance TEXT NOT NULL
);
";
const IDX_CORRESPONDS_BIBLE: &str = "CREATE UNIQUE INDEX corresponds_bible_ord ON corresponds_bible (ord);";

const DDL_TEMPORAL_ADJACENCY: &str = "
CREATE TABLE temporal_adjacency (
  id INTEGER PRIMARY KEY, ord INTEGER NOT NULL,
  earlier_id TEXT NOT NULL, later_id TEXT NOT NULL,
  provenance TEXT NOT NULL
);
";
const IDX_TEMPORAL_ADJACENCY: &str = "
CREATE UNIQUE INDEX temporal_adjacency_ord ON temporal_adjacency (ord);
CREATE INDEX temporal_by_earlier ON temporal_adjacency (earlier_id);
CREATE INDEX temporal_by_later ON temporal_adjacency (later_id);
";

const DDL_ANALOGUE: &str = "
CREATE TABLE analogue (
  id INTEGER PRIMARY KEY, ord INTEGER NOT NULL,
  a_id TEXT NOT NULL, b_id TEXT NOT NULL,
  provenance TEXT NOT NULL
);
";
const IDX_ANALOGUE: &str = "CREATE UNIQUE INDEX analogue_ord ON analogue (ord);";

const DDL_CROSS_REFS: &str = "
CREATE TABLE cross_refs (
  id INTEGER PRIMARY KEY, ord INTEGER NOT NULL,
  from_corpus TEXT NOT NULL, from_a INTEGER NOT NULL, from_b INTEGER NOT NULL, from_c INTEGER NOT NULL, from_layer TEXT, from_start INTEGER, from_end INTEGER,
  to_corpus TEXT NOT NULL, to_a INTEGER NOT NULL, to_b INTEGER NOT NULL, to_c INTEGER NOT NULL, to_layer TEXT, to_start INTEGER, to_end INTEGER,
  to_last_corpus TEXT, to_last_a INTEGER, to_last_b INTEGER, to_last_c INTEGER, to_last_layer TEXT, to_last_start INTEGER, to_last_end INTEGER,
  target_display TEXT NOT NULL,
  votes INTEGER NOT NULL,
  provenance TEXT NOT NULL
);
";
const IDX_CROSS_REFS: &str = "
CREATE UNIQUE INDEX cross_refs_ord ON cross_refs (ord);
CREATE INDEX xref_by_from ON cross_refs (from_a, from_b, from_c, ord);
";

const DDL_SPOKEN_BY: &str = "
CREATE TABLE spoken_by (
  id INTEGER PRIMARY KEY, ord INTEGER NOT NULL,
  locus_from_corpus TEXT NOT NULL, locus_from_a INTEGER NOT NULL, locus_from_b INTEGER NOT NULL, locus_from_c INTEGER NOT NULL, locus_from_layer TEXT, locus_from_start INTEGER, locus_from_end INTEGER,
  locus_to_corpus TEXT NOT NULL, locus_to_a INTEGER NOT NULL, locus_to_b INTEGER NOT NULL, locus_to_c INTEGER NOT NULL, locus_to_layer TEXT, locus_to_start INTEGER, locus_to_end INTEGER,
  speaker_id TEXT NOT NULL,
  provenance TEXT NOT NULL, justification_id INTEGER
);
";
const IDX_SPOKEN_BY: &str = "CREATE UNIQUE INDEX spoken_by_ord ON spoken_by (ord);";

const DDL_SPOKEN_AT: &str = "
CREATE TABLE spoken_at (
  id INTEGER PRIMARY KEY, ord INTEGER NOT NULL,
  locus_from_corpus TEXT NOT NULL, locus_from_a INTEGER NOT NULL, locus_from_b INTEGER NOT NULL, locus_from_c INTEGER NOT NULL, locus_from_layer TEXT, locus_from_start INTEGER, locus_from_end INTEGER,
  locus_to_corpus TEXT NOT NULL, locus_to_a INTEGER NOT NULL, locus_to_b INTEGER NOT NULL, locus_to_c INTEGER NOT NULL, locus_to_layer TEXT, locus_to_start INTEGER, locus_to_end INTEGER,
  place_id TEXT NOT NULL,
  provenance TEXT NOT NULL, justification_id INTEGER
);
";
const IDX_SPOKEN_AT: &str = "CREATE UNIQUE INDEX spoken_at_ord ON spoken_at (ord);";

const DDL_QUOTES: &str = "
CREATE TABLE quotes (
  id INTEGER PRIMARY KEY, ord INTEGER NOT NULL,
  quoting_corpus TEXT NOT NULL, quoting_a INTEGER NOT NULL, quoting_b INTEGER NOT NULL, quoting_c INTEGER NOT NULL, quoting_layer TEXT, quoting_start INTEGER, quoting_end INTEGER,
  quoted_from_corpus TEXT NOT NULL, quoted_from_a INTEGER NOT NULL, quoted_from_b INTEGER NOT NULL, quoted_from_c INTEGER NOT NULL, quoted_from_layer TEXT, quoted_from_start INTEGER, quoted_from_end INTEGER,
  quoted_to_corpus TEXT NOT NULL, quoted_to_a INTEGER NOT NULL, quoted_to_b INTEGER NOT NULL, quoted_to_c INTEGER NOT NULL, quoted_to_layer TEXT, quoted_to_start INTEGER, quoted_to_end INTEGER,
  provenance TEXT NOT NULL
);
";
const IDX_QUOTES: &str = "CREATE UNIQUE INDEX quotes_ord ON quotes (ord);";

const DDL_CONFESSES: &str = "
CREATE TABLE confesses (
  id INTEGER PRIMARY KEY, ord INTEGER NOT NULL,
  confessing_corpus TEXT NOT NULL, confessing_a INTEGER NOT NULL, confessing_b INTEGER NOT NULL, confessing_c INTEGER NOT NULL, confessing_layer TEXT, confessing_start INTEGER, confessing_end INTEGER,
  confessed_from_corpus TEXT NOT NULL, confessed_from_a INTEGER NOT NULL, confessed_from_b INTEGER NOT NULL, confessed_from_c INTEGER NOT NULL, confessed_from_layer TEXT, confessed_from_start INTEGER, confessed_from_end INTEGER,
  confessed_to_corpus TEXT NOT NULL, confessed_to_a INTEGER NOT NULL, confessed_to_b INTEGER NOT NULL, confessed_to_c INTEGER NOT NULL, confessed_to_layer TEXT, confessed_to_start INTEGER, confessed_to_end INTEGER,
  provenance TEXT NOT NULL, justification_id INTEGER
);
";
const IDX_CONFESSES: &str = "CREATE UNIQUE INDEX confesses_ord ON confesses (ord);";

const DDL_COMMENTS_ON: &str = "
CREATE TABLE comments_on (
  id INTEGER PRIMARY KEY, ord INTEGER NOT NULL,
  item_id TEXT NOT NULL,
  on_from_corpus TEXT NOT NULL, on_from_a INTEGER NOT NULL, on_from_b INTEGER NOT NULL, on_from_c INTEGER NOT NULL, on_from_layer TEXT, on_from_start INTEGER, on_from_end INTEGER,
  on_to_corpus TEXT NOT NULL, on_to_a INTEGER NOT NULL, on_to_b INTEGER NOT NULL, on_to_c INTEGER NOT NULL, on_to_layer TEXT, on_to_start INTEGER, on_to_end INTEGER,
  provenance TEXT NOT NULL, justification_id INTEGER
);
";
const IDX_COMMENTS_ON: &str = "
CREATE UNIQUE INDEX comments_on_ord ON comments_on (ord);
CREATE INDEX comments_on_by_from ON comments_on (on_from_a, on_from_b, on_from_c, ord);
";

// ---------------------------------------------------------------------
// DB-4b: the extra tables (spec §5.3–5.6, amended)
// ---------------------------------------------------------------------

/// Core: the graph-derived tables.
const EXTRA_DDL_CORE_GRAPH: &str = "
CREATE TABLE place (
  node_id TEXT PRIMARY KEY, canonical TEXT NOT NULL, lat REAL NOT NULL, lon REAL NOT NULL
) WITHOUT ROWID;
CREATE TABLE era (
  node_id TEXT PRIMARY KEY, label TEXT NOT NULL, from_year INTEGER NOT NULL, to_year INTEGER NOT NULL
) WITHOUT ROWID;
CREATE TABLE polity_era (
  node_id TEXT NOT NULL, ord INTEGER NOT NULL, name TEXT NOT NULL,
  from_year INTEGER NOT NULL, to_year INTEGER NOT NULL,
  PRIMARY KEY (node_id, ord)
) WITHOUT ROWID;
CREATE TABLE event_date (
  event_id  TEXT PRIMARY KEY,
  from_year INTEGER NOT NULL, to_year INTEGER NOT NULL,
  from_month INTEGER, from_day INTEGER, to_month INTEGER, to_day INTEGER,
  seq       INTEGER NOT NULL,
  basis     INTEGER NOT NULL
) WITHOUT ROWID;
CREATE TABLE heading_index (
  book INTEGER NOT NULL, chapter INTEGER NOT NULL, verse INTEGER NOT NULL,
  event_id TEXT NOT NULL, title TEXT NOT NULL, kind TEXT NOT NULL, continuation INTEGER NOT NULL,
  PRIMARY KEY (book, chapter, verse)
) WITHOUT ROWID;
";
const EXTRA_INDEX_DDL_CORE_GRAPH: &str = "
CREATE INDEX polity_era_by_span ON polity_era (from_year, to_year);
CREATE INDEX event_by_span ON event_date (from_year, to_year);
CREATE INDEX event_by_order ON event_date (seq);
CREATE INDEX heading_by_event ON heading_index (event_id);
";

/// Core: the nine folded sidecars as 21 tables.
const EXTRA_DDL_CORE_SIDECARS: &str = "
CREATE TABLE canon_book (
  ord INTEGER PRIMARY KEY, code TEXT NOT NULL, name TEXT NOT NULL, testament TEXT NOT NULL,
  chapters INTEGER NOT NULL
);
CREATE TABLE canon_chapter_verses (
  book_ord INTEGER NOT NULL, chapter INTEGER NOT NULL, verses INTEGER NOT NULL,
  PRIMARY KEY (book_ord, chapter)
) WITHOUT ROWID;
CREATE TABLE book_meta (
  book TEXT PRIMARY KEY, author TEXT NOT NULL, write_place TEXT, write_from INTEGER, write_to INTEGER
) WITHOUT ROWID;
CREATE TABLE chronology_anchor (
  id TEXT PRIMARY KEY, ord INTEGER NOT NULL, label TEXT NOT NULL, year INTEGER NOT NULL,
  event_id TEXT, era_boundary INTEGER NOT NULL, source TEXT NOT NULL, note TEXT
) WITHOUT ROWID;
CREATE TABLE book_narration_window (
  book TEXT PRIMARY KEY, from_year INTEGER NOT NULL, to_year INTEGER NOT NULL, note TEXT
) WITHOUT ROWID;
CREATE TABLE landmark (
  ord INTEGER PRIMARY KEY, name TEXT NOT NULL, kind TEXT NOT NULL, lat REAL NOT NULL, lon REAL NOT NULL, size TEXT
);
CREATE TABLE land_mask_region (
  ord INTEGER PRIMARY KEY, name TEXT, ref_note TEXT, rings_json TEXT NOT NULL
);
CREATE TABLE catechism_part (
  id TEXT PRIMARY KEY, ord INTEGER NOT NULL, title TEXT NOT NULL
) WITHOUT ROWID;
CREATE TABLE catechism_item (
  id TEXT PRIMARY KEY, part_id TEXT NOT NULL, ord INTEGER NOT NULL, name TEXT NOT NULL,
  text TEXT, explanation_heading TEXT NOT NULL, explanation TEXT NOT NULL,
  where_written TEXT, ref_note TEXT
) WITHOUT ROWID;
CREATE TABLE catechism_item_verse (
  item_id TEXT NOT NULL, ord INTEGER NOT NULL, sref TEXT NOT NULL,
  PRIMARY KEY (item_id, ord)
) WITHOUT ROWID;
CREATE TABLE catechism_question (
  item_id TEXT NOT NULL, ord INTEGER NOT NULL, title TEXT NOT NULL, source TEXT NOT NULL,
  PRIMARY KEY (item_id, ord)
) WITHOUT ROWID;
CREATE TABLE catechism_question_verse (
  item_id TEXT NOT NULL, question_ord INTEGER NOT NULL, ord INTEGER NOT NULL, sref TEXT NOT NULL,
  PRIMARY KEY (item_id, question_ord, ord)
) WITHOUT ROWID;
CREATE TABLE place_history (
  place_id TEXT PRIMARY KEY,
  est_from INTEGER, est_to INTEGER, est_note TEXT,
  dest_from INTEGER, dest_to INTEGER, dest_note TEXT
) WITHOUT ROWID;
CREATE TABLE place_history_name (
  place_id TEXT NOT NULL, ord INTEGER NOT NULL, name TEXT NOT NULL, from_year INTEGER NOT NULL, to_year INTEGER NOT NULL,
  PRIMARY KEY (place_id, ord)
) WITHOUT ROWID;
CREATE TABLE place_history_blurb (
  place_id TEXT NOT NULL, ord INTEGER NOT NULL, text TEXT NOT NULL, from_year INTEGER NOT NULL, to_year INTEGER NOT NULL, breadth TEXT NOT NULL,
  PRIMARY KEY (place_id, ord)
) WITHOUT ROWID;
CREATE TABLE place_history_verse (
  place_id TEXT NOT NULL, owner_kind INTEGER NOT NULL, owner_ord INTEGER NOT NULL, ord INTEGER NOT NULL, sref TEXT NOT NULL,
  PRIMARY KEY (place_id, owner_kind, owner_ord, ord)
) WITHOUT ROWID;
CREATE TABLE place_name_alias (
  place_id TEXT NOT NULL, alias_ord INTEGER NOT NULL, translation TEXT NOT NULL, name TEXT NOT NULL,
  PRIMARY KEY (place_id, alias_ord, translation)
) WITHOUT ROWID;
CREATE TABLE place_name_alias_verse (
  place_id TEXT NOT NULL, alias_ord INTEGER NOT NULL, ord INTEGER NOT NULL, sref TEXT NOT NULL,
  PRIMARY KEY (place_id, alias_ord, ord)
) WITHOUT ROWID;
CREATE TABLE source_category (
  id TEXT PRIMARY KEY, ord INTEGER NOT NULL, label TEXT NOT NULL
) WITHOUT ROWID;
CREATE TABLE source_entry (
  id TEXT PRIMARY KEY, ord INTEGER NOT NULL, category TEXT NOT NULL, title TEXT NOT NULL,
  what_it_is TEXT NOT NULL, what_we_built TEXT NOT NULL, license TEXT NOT NULL, link TEXT,
  licenses_row_key TEXT NOT NULL
) WITHOUT ROWID;
CREATE TABLE provenance_entry (
  id TEXT PRIMARY KEY, ord INTEGER NOT NULL, source TEXT NOT NULL, confidence TEXT NOT NULL, locator TEXT
) WITHOUT ROWID;
";
/// The write-time materialisation of `AtlasData::finish()`'s verse->item
/// join (`/api/catechism/{sref}` seeks these at DB-4c).
const EXTRA_INDEX_DDL_CORE_SIDECARS: &str = "
CREATE INDEX catechism_item_by_part ON catechism_item (part_id, ord);
CREATE INDEX catechism_item_verse_by_sref ON catechism_item_verse (sref);
CREATE INDEX catechism_question_verse_by_sref ON catechism_question_verse (sref);
CREATE INDEX provenance_by_source ON provenance_entry (source);
";

const EXTRA_DDL_KJV: &str = "
CREATE TABLE verse (
  node_id TEXT PRIMARY KEY, book INTEGER NOT NULL, chapter INTEGER NOT NULL, verse INTEGER NOT NULL
) WITHOUT ROWID;
CREATE TABLE red_letter_span (
  book INTEGER NOT NULL, chapter INTEGER NOT NULL, verse INTEGER NOT NULL, ord INTEGER NOT NULL,
  start INTEGER NOT NULL, end_ INTEGER NOT NULL,
  PRIMARY KEY (book, chapter, verse, ord)
) WITHOUT ROWID;
";
const EXTRA_INDEX_DDL_KJV: &str = "
CREATE UNIQUE INDEX verse_by_ref ON verse (book, chapter, verse);
";

const EXTRA_DDL_CONCORD: &str = "
CREATE TABLE concord_unit (
  node_id TEXT PRIMARY KEY, part INTEGER NOT NULL, article INTEGER NOT NULL, paragraph INTEGER NOT NULL
) WITHOUT ROWID;
";
const EXTRA_INDEX_DDL_CONCORD: &str = "
CREATE UNIQUE INDEX concord_by_ref ON concord_unit (part, article, paragraph);
";

/// DB-4b: the section's extra tables' `CREATE TABLE` text.
pub fn extra_ddl(section: Section) -> &'static [&'static str] {
    match section {
        Section::Core => &[EXTRA_DDL_CORE_GRAPH, EXTRA_DDL_CORE_SIDECARS],
        Section::Kjv => &[EXTRA_DDL_KJV],
        Section::Concord => &[EXTRA_DDL_CONCORD],
        Section::Kretzmann | Section::Lexicon => &[],
    }
}

/// DB-4b: the section's extra tables' indexes (created after the inserts).
pub fn extra_index_ddl(section: Section) -> &'static [&'static str] {
    match section {
        Section::Core => &[EXTRA_INDEX_DDL_CORE_GRAPH, EXTRA_INDEX_DDL_CORE_SIDECARS],
        Section::Kjv => &[EXTRA_INDEX_DDL_KJV],
        Section::Concord => &[EXTRA_INDEX_DDL_CONCORD],
        Section::Kretzmann | Section::Lexicon => &[],
    }
}

/// The `CREATE TABLE` text (plus sub-table) of spec §5.3–5.6 for a family.
pub fn family_ddl(f: RowFamily) -> &'static str {
    match f {
        RowFamily::ContainsBible => DDL_CONTAINS_BIBLE,
        RowFamily::ContainsConcord => DDL_CONTAINS_CONCORD,
        RowFamily::Attests => DDL_ATTESTS,
        RowFamily::Succession => DDL_SUCCESSION,
        RowFamily::CanonSuccession => DDL_CANON_SUCCESSION,
        RowFamily::DatedBy => DDL_DATED_BY,
        RowFamily::LocatedAt => DDL_LOCATED_AT,
        RowFamily::Fulfills => DDL_FULFILLS,
        RowFamily::Typology => DDL_TYPOLOGY,
        RowFamily::NamedAfter => DDL_NAMED_AFTER,
        RowFamily::Catechism => DDL_CATECHISM,
        RowFamily::CommentsOn => DDL_COMMENTS_ON,
        RowFamily::SpokenBy => DDL_SPOKEN_BY,
        RowFamily::SpokenAt => DDL_SPOKEN_AT,
        RowFamily::Mentions => DDL_MENTIONS,
        RowFamily::CrossRefs => DDL_CROSS_REFS,
        RowFamily::Quotes => DDL_QUOTES,
        RowFamily::Confesses => DDL_CONFESSES,
        RowFamily::CorrespondsBible => DDL_CORRESPONDS_BIBLE,
        RowFamily::TemporalAdjacency => DDL_TEMPORAL_ADJACENCY,
        RowFamily::Analogue => DDL_ANALOGUE,
    }
}

/// The family's `CREATE UNIQUE INDEX <family>_ord` and every secondary
/// index the spec lists for it.
pub fn family_index_ddl(f: RowFamily) -> &'static str {
    match f {
        RowFamily::ContainsBible => IDX_CONTAINS_BIBLE,
        RowFamily::ContainsConcord => IDX_CONTAINS_CONCORD,
        RowFamily::Attests => IDX_ATTESTS,
        RowFamily::Succession => IDX_SUCCESSION,
        RowFamily::CanonSuccession => IDX_CANON_SUCCESSION,
        RowFamily::DatedBy => IDX_DATED_BY,
        RowFamily::LocatedAt => IDX_LOCATED_AT,
        RowFamily::Fulfills => IDX_FULFILLS,
        RowFamily::Typology => IDX_TYPOLOGY,
        RowFamily::NamedAfter => IDX_NAMED_AFTER,
        RowFamily::Catechism => IDX_CATECHISM,
        RowFamily::CommentsOn => IDX_COMMENTS_ON,
        RowFamily::SpokenBy => IDX_SPOKEN_BY,
        RowFamily::SpokenAt => IDX_SPOKEN_AT,
        RowFamily::Mentions => IDX_MENTIONS,
        RowFamily::CrossRefs => IDX_CROSS_REFS,
        RowFamily::Quotes => IDX_QUOTES,
        RowFamily::Confesses => IDX_CONFESSES,
        RowFamily::CorrespondsBible => IDX_CORRESPONDS_BIBLE,
        RowFamily::TemporalAdjacency => IDX_TEMPORAL_ADJACENCY,
        RowFamily::Analogue => IDX_ANALOGUE,
    }
}

/// Spec §6.1: pragmas first (page_size/encoding only bind on an empty
/// file), then every table -- and NO index yet.
pub fn create_tables(conn: &Connection, section: Section) -> Result<(), SqliteError> {
    stamp_pragmas(conn)?;
    let mut ddl = String::from(COMMON_DDL);
    for f in row_tables_of(section) {
        ddl.push_str(family_ddl(*f));
    }
    if has_spine(section) {
        ddl.push_str(SPINE_DDL);
    }
    for extra in extra_ddl(section) {
        ddl.push_str(extra);
    }
    conn.execute_batch(&ddl)?;
    Ok(())
}

/// Spec §6.1: every index, created AFTER the inserts.
pub fn create_indexes(conn: &Connection, section: Section) -> Result<(), SqliteError> {
    let mut ddl = String::from(COMMON_INDEX_DDL);
    for f in row_tables_of(section) {
        ddl.push_str(family_index_ddl(*f));
        ddl.push('\n');
    }
    if has_spine(section) {
        ddl.push_str(SPINE_INDEX_DDL);
    }
    for extra in extra_index_ddl(section) {
        ddl.push_str(extra);
    }
    conn.execute_batch(&ddl)?;
    Ok(())
}

#[cfg(test)]
mod laws {
    use super::*;

    /// Spec §1.3 / §5.0: the file describes the shape; the compiler proves
    /// the laws. No constraint machinery may sneak into any DDL string.
    #[test]
    fn no_ddl_string_carries_constraint_machinery() {
        let mut all: Vec<&str> = vec![COMMON_DDL, COMMON_INDEX_DDL, SPINE_DDL, SPINE_INDEX_DDL];
        for f in RowFamily::ALL {
            all.push(family_ddl(f));
            all.push(family_index_ddl(f));
        }
        for s in Section::MANIFEST_ORDER {
            all.extend(extra_ddl(s).iter().copied());
            all.extend(extra_index_ddl(s).iter().copied());
        }
        for ddl in all {
            let upper = ddl.to_ascii_uppercase();
            for forbidden in ["FOREIGN KEY", "CHECK (", "TRIGGER"] {
                assert!(!upper.contains(forbidden), "{forbidden} in:\n{ddl}");
            }
        }
    }

    #[test]
    fn every_family_index_ddl_names_its_ord_index() {
        for f in RowFamily::ALL {
            let expected = format!("CREATE UNIQUE INDEX {}_ord ON {} (ord);", f.name(), f.name());
            assert!(family_index_ddl(f).contains(&expected), "{f:?}");
            assert!(family_ddl(f).contains(&format!("CREATE TABLE {} (", f.name())), "{f:?}");
        }
    }
}
