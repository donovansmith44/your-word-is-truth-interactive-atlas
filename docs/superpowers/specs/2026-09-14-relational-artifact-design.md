# Relational Artifact — design spec

**Status:** DRAFT, owner-approved section by section in the 2026-09-14
brainstorm; nothing here is implemented. Supersedes the storage half of
`.superpowers/sdd/2026-08-17-bible-atlas-m1/db1-plan.md` (the DB-1
research plan of 2026-09-08), whose nine open questions this spec
answers. Where this spec and DB-1 disagree, this spec wins; DB-1's
measurements and code citations remain the evidence base and are not
repeated here.

**Owner charter (2026-09-07, verbatim via the controller):** "we're
going to have to move. and also the data is a tiny fraction of
everything that we could possibly explore, which is why it's not gonna
be feasible to keep that stuff in memory forever. we will have to do a
migration. probably to RDB."

**Owner rulings (2026-09-14 brainstorm), each recorded where it binds:**
SQLite as the compiled read-only artifact (§1); prerequisites first as
standalone batches (§8); fingerprint fixes at cutover, toolchain pin
immediately (§3); all sidecar JSON folded in (§5.6); port widening
approved as ONE contract change, with every schema shown before
execution (§4, §6 — this document is that showing); full-text search
out of scope (§10); sectioned, content-addressed artifact with a
manifest in git and git-committed zstd distribution today (§2); laws
stay in the compiler, no schema constraints (§1.3); Strong's lexicon as
the first post-cutover section (§7).

---

## 0. THE ONE-PARAGRAPH ANSWER

The compiled artifact stops being one bincode file and becomes a
**manifest plus sections**: a small TOML manifest committed to git that
is the version root, and one read-only SQLite file per corpus, each
named by the hash of its logical content. The server attaches the
sections as one database and serves through the existing `GraphQuery`
port, widened by six methods. The in-memory graph is never rebuilt at
boot. The compiler keeps building the in-memory `Graph`, keeps running
every law over it unchanged, and gains a second encoder. Identity
becomes defensible in the same event: a defined canonical encoding
replaces debug-print bytes, a specification-fixed hash replaces
`DefaultHasher`, the root covers rows and spines as well as nodes, and
the toolchain is pinned. A new corpus is a new section, never a bigger
file; the Strong's lexicon is the first such section and the live proof.

---

## 1. THESIS, RESTATED WITH THE DECISIONS APPLIED

### 1.1 What SQLite is for here

All writes happen in the compiler. The runtime never writes. So the
hard 80% of relational-database adoption — transactions, locking, write
contention, cache invalidation — does not apply. What SQLite buys is:

- **page-level residency**: only touched pages are resident, so the
  751 MiB peak (DB-1 §1.2) becomes the working set of the queries
  actually run;
- **embeddability**: a system library on iOS/Android, a single crate
  binding in Rust, no daemon — the offline-tablet and LAN-box
  deployments (memory: DEPLOYMENT MODEL, 2026-08-26) fit unchanged;
- **one file per section**, attachable, so the artifact can be cut
  without inventing a container format;
- **a query planner** over stored order, so "one page of one kind at
  one position" is a prefix seek and nothing else.

Postgres, Neo4j and DuckDB are ruled out for the reasons DB-1 §1.6
gives; nothing in this spec reopens them.

### 1.2 What does NOT change

- The `GraphQuery` port is still the only read surface for serving.
- The compiler still builds the in-memory `Graph` from adapters and
  runs `law_check.rs` over it, unchanged. The database is an OUTPUT
  ENCODING of a graph already proved lawful.
- Every observable HTTP response is byte-identical across the cutover,
  except the fields that carry a content hash (pids, edge ids, the
  version root), which move exactly once (§3.6). The 25 pinned scene
  hashes (which carry no hash-derived field), the graph contract suite
  over both transports, the map project's edge contract, the query
  contract, the client contract tests and the Playwright suite are the
  proof, and none of them is loosened.
- `graph-types` stays zero-dependency (the C1 covenant surface
  map-generator path-deps from eight crates).

### 1.3 Laws stay in the compiler (owner ruling)

The design spec §9a already says it: "stores are dumb verified bytes +
indexes"; "no schema constraints, no triggers." DB-1 §5 proposed moving
some laws into foreign keys and partial unique indexes. **Rejected.**
The schema in §5 carries PRIMARY KEY and UNIQUE only where a query
needs that B-tree; never as a law. Consequences:

- `law_check.rs` (~1,069 lines) is not rewritten. DB-1's single largest
  cost item disappears.
- A section can reference a node in another section (Kretzmann rows
  cite KJV verses) with no cross-file constraint to fail — the compiler
  already proved every reference resolves (`every_authored_edge_resolves`).
- The admission gate stays what it is: `assert_answers_match` between
  the in-memory graph and the encoded artifact, at compile time, never
  at server start.

---

## 2. THE ARTIFACT: MANIFEST + SECTIONS

### 2.1 Sections at cutover

A **section** is one SQLite file holding everything one adapter family
authored: its nodes, its row tables, its slice of the derived edge
index, and any projections of its own nodes. Four sections ship at
cutover (DB-4), a fifth lands with LEX-1 (§7).

| Section | Node kinds | Row families (authored here) | Folded sidecars |
|---|---|---|---|
| `core` | Place, Person, PeopleGroup, Event, Narrative, Era, Polity, Anchor, CatechismItem, Source, Translation, Container (curated passages / atlas sections) | contains_bible (curated passage containers), attests, succession, dated_by, located_at, fulfills, typology, named_after, catechism, mentions, corresponds_bible, temporal_adjacency, analogue | canon, books_meta, chronology_anchors, book_narration_windows, landmarks, land_mask, catechism, place_history, place_names_kjv, sources (registry + provenance table), heading index, resolved chronology |
| `kjv` | TextUnit (corpus `bible`), Container (book/chapter) | canon_succession, cross_refs, spoken_by, spoken_at | red_letter_spans |
| `concord` | TextUnit (corpus `concord`), Container | contains_concord, quotes, confesses | — |
| `kretzmann` | CommentaryItem | comments_on | — |
| `lexicon` (LEX-1) | LexiconEntry | occurs | token inventory |

**Placement rule (binding):** a row lives in the section of the adapter
that authored it, and BOTH directions of its edge-index entries live in
that same section. The adapter→section map is a compile-time table in
`atlas-graph` (`sections.rs`), and `assert_answers_match` fails if any
row or index entry lands outside its section. Consequence: a section is
independently shippable, and its absence removes exactly its own
frontier sections from every node (a verse without the `kretzmann`
section has no "commentary" section — conditional presence by storage
layout).

**Where `mentions` lives:** with `core` (Theographic authored it about
persons/places; the loci are KJV positions but the assertion is about
the entity). Where `cross_refs` lives: with `kjv` (OpenBible's dataset
is about the text). Where `contains_bible` lives: curated passage
titles → `core`; book/chapter containers → `kjv`. The table in
`sections.rs` is the authority; this paragraph is its rationale.

### 2.2 The manifest

`data/compiled/manifest.toml`, committed to git:

```toml
schema = 1                       # manifest schema version (§9)
compiler = "atlas-graph-compile 0.x.y (rustc 1.97.1)"
built = "2026-10-01T00:00:00Z"   # informational; NOT part of the root
root = "9f3c...e1a0"             # 32 hex chars; see §2.3

[[section]]
name = "core"
required = true
logical = "6b1d...c204"          # identity: hash of the logical dump (§3.4)
blob = "0a77...91fe"             # transport: SHA-256 of the .sqlite.zst bytes
bytes = 18345021                 # compressed size, informational
schema_version = 14              # the section's PRAGMA user_version

[[section]]
name = "kjv"
required = true
logical = "..."
blob = "..."
bytes = 9012345
schema_version = 14

[[section]]
name = "concord"
required = false
# ...

[[section]]
name = "kretzmann"
required = false
# ...
```

- Section ORDER in the manifest is load-bearing (§5.2: cursor order
  across sections).
- `required = false` sections may be absent from a deployment; the
  loader records absence and nothing else. Every other failure is loud
  (§11).
- `root` is the hash over the canonical concatenation, in manifest
  order, of `name|logical|schema_version|required\n` for every section
  (§3.4). It therefore covers every node, row, spine and provenance
  in every section. Timestamps and byte sizes are deliberately outside
  it: a rebuild with identical content has an identical root.

### 2.3 Files on disk

```
data/compiled/manifest.toml
data/compiled/sections/core.6b1d...c204.sqlite.zst
data/compiled/sections/kjv.<logical>.sqlite.zst
data/compiled/sections/concord.<logical>.sqlite.zst
data/compiled/sections/kretzmann.<logical>.sqlite.zst
data/cache/sections/<logical>.sqlite            # gitignored unpack cache
data/exports/{gazetteer,chronology,kretzmann-chronology}.json   # unchanged; carry `root`
```

The file name embeds the section name for humans and the logical hash
for machines; the loader trusts only the hash. Every committed blob
must stay under 100 MiB compressed; DB-4's report states each size
against that ceiling (§12).

### 2.4 The section source (the seam that makes growth cheap)

```rust
/// Resolve a section by its LOGICAL hash to an openable SQLite file.
/// Implementations verify the transport hash before returning.
pub trait SectionSource {
    fn resolve(&self, entry: &ManifestSection) -> Result<PathBuf, SectionError>;
}
```

Implementation #1 (DB-4): `CommittedZstdSource` — finds
`data/compiled/sections/<name>.<logical>.sqlite.zst`, verifies its
SHA-256 equals `blob`, decompresses into `data/cache/sections/
<logical>.sqlite` if not already present, returns the path. Cache hits
skip the verify (the file is named by its logical hash and was verified
when written; `bibex verify` re-checks on demand, §3.5).

When a section outgrows the git ceiling compressed, implementation #2 is
a fetching source (GitHub release asset, object store, the LAN box, a
USB path) keyed by the same manifest entry. The loader, the server and
the port do not change. This is the "what works when the data is
massive" answer: git holds sources and the manifest; sections are
content-addressed blobs from wherever is convenient; an update
downloads only the sections whose hash moved; a tablet ships a subset
and the manifest says what is absent.

### 2.5 Opening the database

At startup the server:

1. reads the manifest; recomputes `root`; refuses on mismatch;
2. resolves every section through the `SectionSource`; refuses if a
   `required` section is missing or fails its transport hash; records
   absent optional sections;
3. opens `core` as `main`, `ATTACH`es the rest under their section
   names in manifest order, `PRAGMA query_only = ON`, `PRAGMA
   mmap_size` = the sum of section sizes (capped by platform), one
   connection per worker thread (SQLite serializes per connection);
4. builds the per-connection `TEMP VIEW`s that union the attached
   sections' `edge_index` and `node` tables in manifest order (§5.2);
5. serves.

SQLite's default limit is 10 attached databases. Four (five after LEX-1)
is comfortably inside it. The bundled build raises `SQLITE_MAX_ATTACHED`
(compile-time, up to 125) when a sixth corpus is scheduled; this is a
one-line build flag and is named here so nobody rediscovers it.

---

## 3. IDENTITY

### 3.1 The three defects this migration fixes (owner ruling: all three)

1. **The version root hashes nodes only** (`store.rs:102-117`). Edge
   rows, spines and provenance are outside it; two batches recorded
   "version root does not move" after adding thousands of rows. The
   map project's C6 stale-check compares exactly this value.
2. **The hash is `DefaultHasher`** (`id.rs:177-181`): algorithm
   unspecified, may change between Rust releases, 64-bit.
3. **No toolchain pin**: a `rustup update` could move every id with
   zero data change.

Plus one DB-1 did not name:

4. **Canonical bytes are debug-print output** (`format!("{:?}|{:?}")`).
   They cannot be decoded, so a store cannot return a node from its
   stored bytes; and renaming a struct field silently changes every id.

### 3.2 Toolchain pin (TOOLCHAIN-1, immediate, no migration dependency)

`rust-toolchain.toml` pinning `1.97.1` in this repo and in
map-generator. One file each. Lands first because it is a latent
correctness bug today, not a migration decision.

### 3.3 The canonical encoding (`graph-types::canon`, zero-dep)

A hand-written, versioned, deterministic, self-describing, DECODABLE
encoding of every `Node` and every row, living in `graph-types` with no
dependencies. Rules:

- **Canonical JSON.** Objects with keys sorted by UTF-8 byte order; no
  whitespace; strings escaped minimally (`"`, `\`, and U+0000–U+001F as
  `\u00XX`; everything else raw UTF-8, NFC as stored); integers in
  decimal with no leading zeros; `f64` via Rust's shortest
  round-trip `Display` (NaN and infinities are rejected at encode —
  none exist in the data); `bool`/`null` as JSON.
- **Enums** as single-key objects `{"Variant":payload}`; unit variants
  as `{"Variant":null}`.
- **Sets/maps** in their `BTree*` order (already canonical in memory).
- **Ids** as their existing string forms (`"Place:jerusalem"`,
  `"bible/JHN.3.16"`); `&'static str` corpus names as strings.
- **Node bytes** = canonical JSON of `{"id":..,"payload":..,"provenance":..}`.
- **Row bytes** = canonical JSON of the row struct, family name outside.
- A **domain prefix** `bible-atlas/canon/1\n` is hashed before the
  bytes so a future encoding version cannot collide with this one.

Decode is total on what encode produced; the law
`decode(encode(x)) == x` is property-tested over every node kind and
every row family, and `derive(pid)` returns exactly the stored bytes.

### 3.4 The hash: SHA-256 truncated to 128 bits, hand-written

The brainstorm said BLAKE3. On checking the covenant: BLAKE3 is a crate
dependency, and `graph-types` must have none. SHA-256 (FIPS 180-4) is
~150 lines of plain Rust, fixed by public specification, and pinned by
NIST test vectors. Truncation to 128 bits keeps the birthday bound at
~2^64 items — design, not luck (DB-1 §4.4 on 64 bits). Every
`ContentHash` becomes `[u8; 16]`, rendered as 32 lowercase hex chars on
the wire. `Pid`, `EdgeId` (its content-addressed form), and
`GraphVersion` widen accordingly. Nothing outside `graph-types` chooses
a hash; nothing inside it depends on `std::hash`.

**Identity hierarchy:**

| Thing | Hash input |
|---|---|
| node pid | domain prefix + node canonical bytes |
| edge id | domain prefix + canonical edge bytes `{"object":<position>,"rel":"<name>","subject":<position>}` (DB-4a judgment call 1: the per-row key would give one id to every entry a multi-locus `Contains` row produces; `edge_index.row_family/row_id` still name the row) |
| section logical hash | domain prefix + the section's **logical dump**: for each table in the fixed schema order, for each row in primary-key order, `table\tcanonical row JSON\n` |
| version root | domain prefix + the manifest's canonical section lines (§2.2) |

The logical dump is computed by the compiler from the in-memory `Graph`
AND recomputable from the SQLite file by streaming its tables in
primary-key order. The two must agree; DB-2's gate proves it (§9).
SQLite's own file bytes vary with library version and page fill and are
NEVER an identity (DB-1 OQ-6, option a).

### 3.5 Verification on demand

`bibex verify [--section name]`: recompute the logical hash of each
cached section from its tables, the transport hash of each committed
blob, and the root from the manifest; print each against the manifest;
non-zero exit on any mismatch. This is the offline tablet's "verified
update" primitive and CI's integrity check after `graph.bin` is gone.

### 3.6 What moves at cutover, once

Every pid, every edge id, the version root, the CLI's byte-pinned
transcripts (they embed edge ids; re-blessed ONCE, disclosed in DB-4's
report), and map-generator's vendored `SOURCE.md` pin (its
`StaleAgainstAtlas` fires exactly once and it re-vendors). No HTTP
response body changes except where it carries an id or the root — and
the 19 AQC fixtures that embed the version root are re-recorded by the
recorder through the one assembly path, exactly as CDC-1 built it to
be (a MINOR bump of that suite, §9).

---

## 4. THE PORT, WIDENED (one contract change — owner ruling)

`GraphQuery` stays read-only and gains six methods. Default
implementations are given in terms of the existing five wherever a
composition is possible, so `MemSnapshot` and the concrete `Graph`
inherit them and the equivalence harness compares the SQLite
overrides against the compositions.

```rust
pub struct NodePage { pub ids: Vec<AnyNodeId>, pub next: Option<usize> }

pub struct RowRef {
    pub family: RowFamily,          // closed enum, one per row table
    pub row_id: u64,                // primary key in that family
    pub provenance: ProvenanceId,
}

pub struct EdgeEntryWithNode { pub entry: EdgeEntry, pub node: Option<Node> }
pub struct EdgePageWithNodes { pub kind: EdgeKind, pub entries: Vec<EdgeEntryWithNode>, pub next: Option<usize> }

pub trait GraphQuery {
    // existing five: node, derive, edge_summary, edges, reading_window

    /// All node ids of one kind, in id order, paged. Retires era_ids,
    /// polity_ids, narrative_ids, event_ids, place_ids, person_ids.
    fn nodes_of_kind(&self, kind: NodeKind, cursor: Option<usize>, limit: usize) -> NodePage;

    /// Batch lookup; position i answers ids[i]. Retires the 31,102
    /// single node() calls the overlay made.
    fn nodes(&self, ids: &[AnyNodeId]) -> Vec<Option<Node>> {
        ids.iter().map(|i| self.node(i)).collect()
    }

    /// One page of one kind WITH each target node. Kills the N+1 the
    /// reader and frontier work around. Edge positions have no node.
    fn edges_with_nodes(&self, p: &Position, q: &EdgeQuery) -> EdgePageWithNodes {
        let page = self.edges(p, q);
        /* compose via nodes() over the entries' node positions */
    }

    /// Which row produced this edge, and its provenance. Retires
    /// PROV-1's load-time companion index.
    fn row_provenance(&self, e: &EdgeId) -> Option<RowRef>;

    /// Index of a unit in a corpus's reading spine. Retires
    /// bible_position / concord_position.
    fn position_of(&self, corpus: &'static str, id: &AnyNodeId) -> Option<usize>;

    /// The frontier as a port method: one round trip for FQ-1.
    /// Default = the composition over edge_summary + edges that
    /// graph-types/src/frontier.rs already defines; the SQLite
    /// override answers it in one statement per inhabited kind.
    fn compose_frontier(&self, focus: &Position, caps: &FrontierCaps) -> Frontier {
        crate::frontier::compose(self, focus, caps)
    }
}
```

**Also in the same types change, uninhabited until LEX-1:** `NodeKind::
LexiconEntry`, `NodePayload::LexiconEntry { .. }` (§7.2), and the
`Occurs` relation in the `relations!` manifest (§7.3). Bundling them
here means map-generator coordinates ONCE (DB-3), not again at LEX-1.
`ContentHash` widening (§3.4) rides in the same change.

Retired after DB-3 (each with an equivalence test against the port
method that replaces it, then deleted): the six id lists, `bible_position`,
`concord_position`, `cross_refs_by_from` (becomes a `kjv.cross_refs`
seek), `persons_by_verse` (a `mentions` seek),
`temporal_neighbors` (a `temporal_adjacency` seek), the PROV-1 row
index. (`verse_text` and `LegacyAtlasFields.verses` are already gone
at OVERLAY-1, §8 — the KJV has one copy, in the `kjv` section's node
payloads.) Kept as stored tables because they encode policy, not
projection: `heading_index` (§5.3), resolved chronology (§5.3),
`red_letter_span` (§5.4, not derivable from the graph).

---

## 5. SCHEMA (complete — the owner asked to see every schema before execution)

### 5.0 Conventions

- Every table is `WITHOUT ROWID` unless it has an `INTEGER PRIMARY KEY`
  used as a row id. The primary key IS the B-tree; hot reads are prefix
  seeks on it.
- `ord INTEGER` = compile-time insertion order within the family. It is
  load-bearing (the map's arrow endpoints read `places[0]`) and it is
  the paging cursor.
- No `FOREIGN KEY`, no `CHECK`, no triggers (§1.3). `NOT NULL` is used,
  because it describes the shape, not a law.
- Node ids are `TEXT` in their existing string form. Positions are
  `TEXT`: `n:<Kind>:<raw>` for nodes, `e:<32 hex>` for edges.
- Provenance ids are `TEXT` naming `core.provenance_entry.id`; no
  constraint, the compiler proved resolution.
- **LOCUS(p)** expands to seven columns:
  `p_corpus TEXT NOT NULL` (`bible`|`concord`), `p_a INTEGER NOT NULL`,
  `p_b INTEGER NOT NULL`, `p_c INTEGER NOT NULL` (book/chapter/verse or
  part/article/paragraph), `p_layer TEXT`, `p_start INTEGER`, `p_end
  INTEGER` (the optional `TokenSpan`; all three NULL when whole-unit).
- **RANGE(p)** expands to LOCUS(p_from) + LOCUS(p_to).
- **AUTHORED** expands to `provenance TEXT NOT NULL, justification_id
  INTEGER` (NULL when the family has no justification — imported rows).
- Each section's `PRAGMA user_version = 14`, `PRAGMA application_id =
  0x424C4741` ('BLGA'), `PRAGMA page_size = 4096`, `PRAGMA encoding =
  'UTF-8'`, `PRAGMA journal_mode = OFF` at build (read-only forever after).
- Hash columns (`node.pid`, `edge_index.edge_id`) are `BLOB` of the
  current `ContentHash` width: 8 bytes while the SHA-256-128 flag is
  OFF (DB-2, DB-3 — files written only for the equivalence gate), 16
  bytes from DB-4 on. `meta.hash_width` records which.

### 5.1 Tables present in EVERY section

```sql
CREATE TABLE meta (
  key   TEXT PRIMARY KEY,
  value TEXT NOT NULL
) WITHOUT ROWID;
-- rows: section_name, schema_version, logical_hash (this section's own,
-- self-reported; the manifest is the authority), compiler, canon_version,
-- built (informational)

CREATE TABLE node (
  id         TEXT    PRIMARY KEY,     -- "Place:jerusalem"
  kind       INTEGER NOT NULL,        -- NodeKind ordinal
  pid        BLOB    NOT NULL,        -- 16 bytes, §3.4
  label      TEXT,                    -- display name hoisted from payload
  provenance TEXT    NOT NULL,
  payload    BLOB    NOT NULL         -- the node's canonical bytes, §3.3 (THE one copy of any text)
) WITHOUT ROWID;
CREATE INDEX node_by_kind ON node (kind, id);   -- nodes_of_kind
CREATE UNIQUE INDEX node_by_pid ON node (pid);  -- derive()

CREATE TABLE justification (
  id   INTEGER PRIMARY KEY,
  text TEXT
);
CREATE TABLE ground (
  justification_id INTEGER NOT NULL,
  ord              INTEGER NOT NULL,   -- BTreeSet order
  kind             INTEGER NOT NULL,   -- 0 Scripture | 1 Anchor | 2 Source
  -- RANGE(scr): Scripture; NULL otherwise
  scr_from_corpus TEXT, scr_from_a INTEGER, scr_from_b INTEGER, scr_from_c INTEGER,
  scr_from_layer TEXT, scr_from_start INTEGER, scr_from_end INTEGER,
  scr_to_corpus TEXT, scr_to_a INTEGER, scr_to_b INTEGER, scr_to_c INTEGER,
  scr_to_layer TEXT, scr_to_start INTEGER, scr_to_end INTEGER,
  anchor_id TEXT,                      -- Anchor
  source_id TEXT,                      -- Source
  PRIMARY KEY (justification_id, ord)
) WITHOUT ROWID;

-- THE DERIVED INDEX (Graph::build_indexes, materialised per section).
-- Rows are the truth; this derives from them by one INSERT…SELECT per
-- family in the writer, and the law index≡rows is asserted at compile.
CREATE TABLE edge_index (
  subject        TEXT    NOT NULL,   -- Position
  rel            INTEGER NOT NULL,   -- RelationId ordinal; 128 + SymRelationId ordinal for symmetric
  dir            INTEGER NOT NULL,   -- 0 forward | 1 inverse | 2 symmetric
  ord            INTEGER NOT NULL,   -- position within (subject, rel, dir): THE CURSOR
  object         TEXT    NOT NULL,   -- Position
  edge_id        BLOB    NOT NULL,   -- 16 bytes
  meta_kind      INTEGER NOT NULL,   -- 0 None | 1 Narrative | 2 Votes
  meta_narrative TEXT,
  meta_votes     INTEGER,
  row_family     INTEGER NOT NULL,   -- RowFamily ordinal  (PROV-1's gap, closed)
  row_id         INTEGER NOT NULL,
  PRIMARY KEY (subject, rel, dir, ord)
) WITHOUT ROWID;
CREATE UNIQUE INDEX edge_by_id ON edge_index (edge_id, dir);   -- row_provenance
```

### 5.2 Union views (per connection, TEMP, built from the manifest)

```sql
CREATE TEMP VIEW all_edge_index AS
  SELECT 0 AS sec, * FROM main.edge_index
  UNION ALL SELECT 1, * FROM kjv.edge_index
  UNION ALL SELECT 2, * FROM concord.edge_index      -- only if attached
  UNION ALL SELECT 3, * FROM kretzmann.edge_index;   -- only if attached
CREATE TEMP VIEW all_node AS /* same shape over node */;
```

`sec` is the manifest rank. **Cursor order across sections is `(sec,
ord)`** — deterministic, and identical to today's single-`Vec` order
when the compiler appends rows section by section in manifest order
(DB-2's equivalence gate proves it; if any family's authored order is
interleaved across adapters, the writer records a global `ord` instead
and the gate says which — no silent reorder).

The two hot reads:

```sql
-- edge_summary(p)
SELECT rel, dir, COUNT(*) FROM all_edge_index
 WHERE subject = ?1 GROUP BY rel, dir;

-- edges(p, kind, cursor, limit)
SELECT object, edge_id, meta_kind, meta_narrative, meta_votes, row_family, row_id
  FROM all_edge_index
 WHERE subject = ?1 AND rel = ?2 AND dir = ?3
 ORDER BY sec, ord LIMIT ?5 OFFSET ?4;
```

Both are prefix seeks on each section's clustered key; the union costs
one seek per attached section.

### 5.3 `core` section

```sql
-- projections of core node kinds
CREATE TABLE place (
  node_id TEXT PRIMARY KEY, canonical TEXT NOT NULL, lat REAL NOT NULL, lon REAL NOT NULL
) WITHOUT ROWID;
CREATE TABLE era (
  node_id TEXT PRIMARY KEY, label TEXT NOT NULL, from_year INTEGER NOT NULL, to_year INTEGER NOT NULL
) WITHOUT ROWID;
CREATE TABLE polity_era (                       -- Polity payload eras, one row each (rings stay in payload)
  node_id TEXT NOT NULL, ord INTEGER NOT NULL, name TEXT NOT NULL,
  from_year INTEGER NOT NULL, to_year INTEGER NOT NULL,
  PRIMARY KEY (node_id, ord)
) WITHOUT ROWID;
CREATE INDEX polity_era_by_span ON polity_era (from_year, to_year);   -- /api/polities?from&to

-- resolved chronology: DERIVED from dated_by at compile (the Chronology companion, relocated)
CREATE TABLE event_date (
  event_id  TEXT PRIMARY KEY,
  from_year INTEGER NOT NULL, to_year INTEGER NOT NULL,
  month INTEGER, day INTEGER,                    -- TimePoint precision when present
  seq       INTEGER NOT NULL,                    -- SeqKey
  order_key INTEGER NOT NULL,                    -- TOTAL traversal order (design §10)
  basis     INTEGER NOT NULL                     -- 0 Textual | 1 Traditional
) WITHOUT ROWID;
CREATE INDEX event_by_span ON event_date (from_year, to_year);
CREATE INDEX event_by_order ON event_date (order_key);

-- heading index: STORED because it encodes a 3-tier decisive-title policy (heading.rs)
CREATE TABLE heading_index (
  event_id TEXT PRIMARY KEY, title TEXT NOT NULL, kind TEXT NOT NULL, continuation INTEGER NOT NULL
) WITHOUT ROWID;

-- ---------- row families authored here ----------
CREATE TABLE contains_bible (           -- curated passage containers only (book/chapter → kjv)
  id INTEGER PRIMARY KEY, ord INTEGER NOT NULL,
  container_id TEXT NOT NULL,
  child_container_id TEXT,              -- ContainerContent::Container
  provenance TEXT NOT NULL, justification_id INTEGER
);
CREATE UNIQUE INDEX contains_bible_ord ON contains_bible (ord);
CREATE TABLE contains_bible_locus (     -- ContainerContent::Loci, expanded, set order
  contains_id INTEGER NOT NULL, ord INTEGER NOT NULL,
  a INTEGER NOT NULL, b INTEGER NOT NULL, c INTEGER NOT NULL, layer TEXT, start INTEGER, end_ INTEGER,
  PRIMARY KEY (contains_id, ord)
) WITHOUT ROWID;

CREATE TABLE attests (
  id INTEGER PRIMARY KEY, ord INTEGER NOT NULL,
  event_id TEXT NOT NULL,
  -- RANGE(att)
  att_from_corpus TEXT NOT NULL, att_from_a INTEGER NOT NULL, att_from_b INTEGER NOT NULL, att_from_c INTEGER NOT NULL,
  att_from_layer TEXT, att_from_start INTEGER, att_from_end INTEGER,
  att_to_corpus TEXT NOT NULL, att_to_a INTEGER NOT NULL, att_to_b INTEGER NOT NULL, att_to_c INTEGER NOT NULL,
  att_to_layer TEXT, att_to_start INTEGER, att_to_end INTEGER,
  provenance TEXT NOT NULL, justification_id INTEGER
);
CREATE UNIQUE INDEX attests_ord ON attests (ord);
CREATE INDEX attests_by_from ON attests (att_from_a, att_from_b, att_from_c);

CREATE TABLE succession (
  id INTEGER PRIMARY KEY, ord INTEGER NOT NULL,
  narrative_id TEXT NOT NULL,
  provenance TEXT NOT NULL, justification_id INTEGER
);
CREATE UNIQUE INDEX succession_ord ON succession (ord);
CREATE TABLE succession_step (          -- chain: Vec<EventId>, order-preserving
  succession_id INTEGER NOT NULL, ord INTEGER NOT NULL, event_id TEXT NOT NULL,
  PRIMARY KEY (succession_id, ord)
) WITHOUT ROWID;

CREATE TABLE dated_by (
  id INTEGER PRIMARY KEY, ord INTEGER NOT NULL,
  event_id TEXT NOT NULL,
  placement_kind INTEGER NOT NULL,      -- 0 AnchorBinding | 1 ReignYear | 2 SequenceAfter | 3 EraOnly
  anchor_id TEXT, prior_event_id TEXT, era_id TEXT,
  years INTEGER, months INTEGER, days INTEGER,   -- Duration (AnchorBinding.offset / SequenceAfter.spacing)
  year_of_reign INTEGER,
  basis INTEGER NOT NULL,
  provenance TEXT NOT NULL, justification_id INTEGER
);
CREATE UNIQUE INDEX dated_by_ord ON dated_by (ord);

CREATE TABLE located_at (
  id INTEGER PRIMARY KEY, ord INTEGER NOT NULL,
  event_id TEXT NOT NULL, place_id TEXT NOT NULL,
  provenance TEXT NOT NULL, justification_id INTEGER
);
CREATE UNIQUE INDEX located_at_ord ON located_at (ord);
CREATE INDEX located_at_by_place ON located_at (place_id, ord);

CREATE TABLE fulfills (
  id INTEGER PRIMARY KEY, ord INTEGER NOT NULL,
  -- RANGE(prophecy), RANGE(fulfillment)  (14 columns each, LOCUS convention)
  prophecy_from_corpus TEXT NOT NULL, prophecy_from_a INTEGER NOT NULL, prophecy_from_b INTEGER NOT NULL, prophecy_from_c INTEGER NOT NULL, prophecy_from_layer TEXT, prophecy_from_start INTEGER, prophecy_from_end INTEGER,
  prophecy_to_corpus TEXT NOT NULL, prophecy_to_a INTEGER NOT NULL, prophecy_to_b INTEGER NOT NULL, prophecy_to_c INTEGER NOT NULL, prophecy_to_layer TEXT, prophecy_to_start INTEGER, prophecy_to_end INTEGER,
  fulfillment_from_corpus TEXT NOT NULL, fulfillment_from_a INTEGER NOT NULL, fulfillment_from_b INTEGER NOT NULL, fulfillment_from_c INTEGER NOT NULL, fulfillment_from_layer TEXT, fulfillment_from_start INTEGER, fulfillment_from_end INTEGER,
  fulfillment_to_corpus TEXT NOT NULL, fulfillment_to_a INTEGER NOT NULL, fulfillment_to_b INTEGER NOT NULL, fulfillment_to_c INTEGER NOT NULL, fulfillment_to_layer TEXT, fulfillment_to_start INTEGER, fulfillment_to_end INTEGER,
  provenance TEXT NOT NULL, justification_id INTEGER
);
CREATE UNIQUE INDEX fulfills_ord ON fulfills (ord);

CREATE TABLE typology (                 -- RANGE(type_), RANGE(antitype), note
  id INTEGER PRIMARY KEY, ord INTEGER NOT NULL,
  type_from_corpus TEXT NOT NULL, type_from_a INTEGER NOT NULL, type_from_b INTEGER NOT NULL, type_from_c INTEGER NOT NULL, type_from_layer TEXT, type_from_start INTEGER, type_from_end INTEGER,
  type_to_corpus TEXT NOT NULL, type_to_a INTEGER NOT NULL, type_to_b INTEGER NOT NULL, type_to_c INTEGER NOT NULL, type_to_layer TEXT, type_to_start INTEGER, type_to_end INTEGER,
  antitype_from_corpus TEXT NOT NULL, antitype_from_a INTEGER NOT NULL, antitype_from_b INTEGER NOT NULL, antitype_from_c INTEGER NOT NULL, antitype_from_layer TEXT, antitype_from_start INTEGER, antitype_from_end INTEGER,
  antitype_to_corpus TEXT NOT NULL, antitype_to_a INTEGER NOT NULL, antitype_to_b INTEGER NOT NULL, antitype_to_c INTEGER NOT NULL, antitype_to_layer TEXT, antitype_to_start INTEGER, antitype_to_end INTEGER,
  note TEXT,
  provenance TEXT NOT NULL, justification_id INTEGER
);
CREATE UNIQUE INDEX typology_ord ON typology (ord);

CREATE TABLE named_after (
  id INTEGER PRIMARY KEY, ord INTEGER NOT NULL,
  namesake_kind INTEGER NOT NULL,       -- 0 PeopleGroup | 1 Place | 2 Polity
  namesake_id TEXT NOT NULL, eponym_id TEXT NOT NULL,
  provenance TEXT NOT NULL, justification_id INTEGER
);
CREATE UNIQUE INDEX named_after_ord ON named_after (ord);

CREATE TABLE catechism (                -- CatechismLink: LOCUS(locus), item
  id INTEGER PRIMARY KEY, ord INTEGER NOT NULL,
  locus_corpus TEXT NOT NULL, locus_a INTEGER NOT NULL, locus_b INTEGER NOT NULL, locus_c INTEGER NOT NULL, locus_layer TEXT, locus_start INTEGER, locus_end INTEGER,
  item_id TEXT NOT NULL,
  provenance TEXT NOT NULL, justification_id INTEGER
);
CREATE UNIQUE INDEX catechism_ord ON catechism (ord);
CREATE INDEX catechism_by_locus ON catechism (locus_a, locus_b, locus_c);   -- /api/catechism/{sref}

CREATE TABLE mentions (                 -- LOCUS(locus), entity (imported: no justification)
  id INTEGER PRIMARY KEY, ord INTEGER NOT NULL,
  locus_corpus TEXT NOT NULL, locus_a INTEGER NOT NULL, locus_b INTEGER NOT NULL, locus_c INTEGER NOT NULL, locus_layer TEXT, locus_start INTEGER, locus_end INTEGER,
  entity_kind INTEGER NOT NULL,         -- 0 Place | 1 Person | 2 PeopleGroup | 3 Event
  entity_id TEXT NOT NULL,
  provenance TEXT NOT NULL
);
CREATE UNIQUE INDEX mentions_ord ON mentions (ord);
CREATE INDEX mentions_by_locus ON mentions (locus_a, locus_b, locus_c, ord);   -- persons_by_verse
CREATE INDEX mentions_by_entity ON mentions (entity_id, ord);

CREATE TABLE corresponds_bible (        -- LOCUS(a), LOCUS(b); symmetric
  id INTEGER PRIMARY KEY, ord INTEGER NOT NULL,
  a_corpus TEXT NOT NULL, a_a INTEGER NOT NULL, a_b INTEGER NOT NULL, a_c INTEGER NOT NULL, a_layer TEXT, a_start INTEGER, a_end INTEGER,
  b_corpus TEXT NOT NULL, b_a INTEGER NOT NULL, b_b INTEGER NOT NULL, b_c INTEGER NOT NULL, b_layer TEXT, b_start INTEGER, b_end INTEGER,
  provenance TEXT NOT NULL
);
CREATE UNIQUE INDEX corresponds_bible_ord ON corresponds_bible (ord);

CREATE TABLE temporal_adjacency (       -- DERIVED at compile; symmetric
  id INTEGER PRIMARY KEY, ord INTEGER NOT NULL,
  earlier_id TEXT NOT NULL, later_id TEXT NOT NULL,
  provenance TEXT NOT NULL
);
CREATE UNIQUE INDEX temporal_adjacency_ord ON temporal_adjacency (ord);
CREATE INDEX temporal_by_earlier ON temporal_adjacency (earlier_id);
CREATE INDEX temporal_by_later ON temporal_adjacency (later_id);

CREATE TABLE analogue (                 -- symmetric; a < b canonical, asserted by the compiler
  id INTEGER PRIMARY KEY, ord INTEGER NOT NULL,
  a_id TEXT NOT NULL, b_id TEXT NOT NULL,
  provenance TEXT NOT NULL
);
CREATE UNIQUE INDEX analogue_ord ON analogue (ord);

-- ---------- folded sidecars (formerly data/compiled/*.json) ----------
CREATE TABLE canon_book (               -- canon.json
  ord INTEGER PRIMARY KEY, code TEXT NOT NULL, name TEXT NOT NULL, testament TEXT NOT NULL,
  chapters INTEGER NOT NULL
);
CREATE TABLE canon_chapter_verses (     -- verse counts per chapter (from canon.json)
  book_ord INTEGER NOT NULL, chapter INTEGER NOT NULL, verses INTEGER NOT NULL,
  PRIMARY KEY (book_ord, chapter)
) WITHOUT ROWID;
CREATE TABLE book_meta (                -- books-meta.json
  book TEXT PRIMARY KEY, author TEXT NOT NULL, write_place TEXT, write_from INTEGER, write_to INTEGER
) WITHOUT ROWID;
CREATE TABLE chronology_anchor (        -- chronology-anchors.json
  id TEXT PRIMARY KEY, label TEXT NOT NULL, year INTEGER NOT NULL, ord INTEGER NOT NULL
) WITHOUT ROWID;
CREATE TABLE book_narration_window (    -- book-narration-windows.json
  book TEXT PRIMARY KEY, from_year INTEGER NOT NULL, to_year INTEGER NOT NULL, note TEXT
) WITHOUT ROWID;
CREATE TABLE landmark (                 -- landmarks.json
  ord INTEGER PRIMARY KEY, name TEXT NOT NULL, kind TEXT NOT NULL, lat REAL NOT NULL, lon REAL NOT NULL, size TEXT
);
CREATE TABLE land_mask_region (         -- land-mask.json; rings as canonical JSON (render-only data)
  ord INTEGER PRIMARY KEY, name TEXT NOT NULL, ref_note TEXT NOT NULL, rings_json TEXT NOT NULL
);
CREATE TABLE catechism_part (           -- catechism.json
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
CREATE TABLE place_history (            -- place-history.json
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
CREATE TABLE place_history_verse (      -- verses cited by name entries and date claims
  place_id TEXT NOT NULL, owner_kind INTEGER NOT NULL, owner_ord INTEGER NOT NULL, ord INTEGER NOT NULL, sref TEXT NOT NULL,
  PRIMARY KEY (place_id, owner_kind, owner_ord, ord)
) WITHOUT ROWID;                        -- owner_kind: 0 name | 1 established | 2 destroyed
CREATE TABLE place_name_alias (         -- place-names-kjv.json
  place_id TEXT NOT NULL, translation TEXT NOT NULL, name TEXT NOT NULL,
  PRIMARY KEY (place_id, translation)
) WITHOUT ROWID;
CREATE TABLE place_name_alias_verse (
  place_id TEXT NOT NULL, ord INTEGER NOT NULL, sref TEXT NOT NULL,
  PRIMARY KEY (place_id, ord)
) WITHOUT ROWID;
CREATE TABLE source_category (          -- sources.json
  id TEXT PRIMARY KEY, ord INTEGER NOT NULL, label TEXT NOT NULL
) WITHOUT ROWID;
CREATE TABLE source_entry (
  id TEXT PRIMARY KEY, ord INTEGER NOT NULL, category TEXT NOT NULL, title TEXT NOT NULL,
  what_it_is TEXT NOT NULL, what_we_built TEXT NOT NULL, license TEXT NOT NULL, link TEXT,
  licenses_row_key TEXT NOT NULL
) WITHOUT ROWID;
CREATE TABLE provenance_entry (         -- the PROV-1 provenance table; every `provenance` column names one of these
  id TEXT PRIMARY KEY, ord INTEGER NOT NULL, source TEXT NOT NULL, confidence TEXT NOT NULL
) WITHOUT ROWID;
```

`polities.json` is loaded today and served by nothing (`/api/polities`
reads Polity payloads). It is **retired**, not folded — disclosed here
so the fold's inventory is honest: ten sidecars, nine folded, one
retired.

### 5.4 `kjv` section

```sql
CREATE TABLE verse (                    -- projection of TextUnit(bible)
  node_id TEXT PRIMARY KEY, book INTEGER NOT NULL, chapter INTEGER NOT NULL, verse INTEGER NOT NULL
) WITHOUT ROWID;
CREATE UNIQUE INDEX verse_by_ref ON verse (book, chapter, verse);

CREATE TABLE reading_spine (            -- corpus 'bible'
  ord INTEGER PRIMARY KEY, node_id TEXT NOT NULL
);
CREATE UNIQUE INDEX spine_by_node ON reading_spine (node_id);   -- position_of

CREATE TABLE contains_bible (           -- book/chapter containers (same DDL as core.contains_bible)
  id INTEGER PRIMARY KEY, ord INTEGER NOT NULL, container_id TEXT NOT NULL, child_container_id TEXT,
  provenance TEXT NOT NULL, justification_id INTEGER
);
CREATE UNIQUE INDEX contains_bible_ord ON contains_bible (ord);
CREATE TABLE contains_bible_locus (
  contains_id INTEGER NOT NULL, ord INTEGER NOT NULL,
  a INTEGER NOT NULL, b INTEGER NOT NULL, c INTEGER NOT NULL, layer TEXT, start INTEGER, end_ INTEGER,
  PRIMARY KEY (contains_id, ord)
) WITHOUT ROWID;

CREATE TABLE canon_succession (
  id INTEGER PRIMARY KEY, ord INTEGER NOT NULL, prior_id TEXT NOT NULL, next_id TEXT NOT NULL,
  provenance TEXT NOT NULL, justification_id INTEGER
);
CREATE UNIQUE INDEX canon_succession_ord ON canon_succession (ord);

CREATE TABLE cross_refs (               -- ~344k rows; LOCUS(from), LOCUS(to), LOCUS(to_last) nullable
  id INTEGER PRIMARY KEY, ord INTEGER NOT NULL,
  from_corpus TEXT NOT NULL, from_a INTEGER NOT NULL, from_b INTEGER NOT NULL, from_c INTEGER NOT NULL, from_layer TEXT, from_start INTEGER, from_end INTEGER,
  to_corpus TEXT NOT NULL, to_a INTEGER NOT NULL, to_b INTEGER NOT NULL, to_c INTEGER NOT NULL, to_layer TEXT, to_start INTEGER, to_end INTEGER,
  to_last_corpus TEXT, to_last_a INTEGER, to_last_b INTEGER, to_last_c INTEGER, to_last_layer TEXT, to_last_start INTEGER, to_last_end INTEGER,
  target_display TEXT NOT NULL,         -- the ORIGINAL citation string
  votes INTEGER NOT NULL,
  provenance TEXT NOT NULL
);
CREATE UNIQUE INDEX cross_refs_ord ON cross_refs (ord);
CREATE INDEX xref_by_from ON cross_refs (from_a, from_b, from_c, ord);   -- cross_refs_by_from

CREATE TABLE spoken_by (                -- RANGE(locus), speaker
  id INTEGER PRIMARY KEY, ord INTEGER NOT NULL,
  locus_from_corpus TEXT NOT NULL, locus_from_a INTEGER NOT NULL, locus_from_b INTEGER NOT NULL, locus_from_c INTEGER NOT NULL, locus_from_layer TEXT, locus_from_start INTEGER, locus_from_end INTEGER,
  locus_to_corpus TEXT NOT NULL, locus_to_a INTEGER NOT NULL, locus_to_b INTEGER NOT NULL, locus_to_c INTEGER NOT NULL, locus_to_layer TEXT, locus_to_start INTEGER, locus_to_end INTEGER,
  speaker_id TEXT NOT NULL,
  provenance TEXT NOT NULL, justification_id INTEGER
);
CREATE UNIQUE INDEX spoken_by_ord ON spoken_by (ord);
CREATE TABLE spoken_at (                -- RANGE(locus), place — same shape
  id INTEGER PRIMARY KEY, ord INTEGER NOT NULL,
  locus_from_corpus TEXT NOT NULL, locus_from_a INTEGER NOT NULL, locus_from_b INTEGER NOT NULL, locus_from_c INTEGER NOT NULL, locus_from_layer TEXT, locus_from_start INTEGER, locus_from_end INTEGER,
  locus_to_corpus TEXT NOT NULL, locus_to_a INTEGER NOT NULL, locus_to_b INTEGER NOT NULL, locus_to_c INTEGER NOT NULL, locus_to_layer TEXT, locus_to_start INTEGER, locus_to_end INTEGER,
  place_id TEXT NOT NULL,
  provenance TEXT NOT NULL, justification_id INTEGER
);
CREATE UNIQUE INDEX spoken_at_ord ON spoken_at (ord);

CREATE TABLE red_letter_span (          -- red-letter-spans.json; NOT derivable from the graph
  book INTEGER NOT NULL, chapter INTEGER NOT NULL, verse INTEGER NOT NULL, ord INTEGER NOT NULL,
  start INTEGER NOT NULL, end_ INTEGER NOT NULL,     -- KJV char offsets
  PRIMARY KEY (book, chapter, verse, ord)
) WITHOUT ROWID;
```

The KJV text and every parallel rendering (Vulgate, Masoretic, Douay,
Biblia 1776, Karl XII, Textus Receptus) live ONLY in `node.payload`
(the `TextUnit { renderings }` canonical bytes). There is no
`node_text` table: one copy, decoded on read. A chapter is ≤176 small
blobs; decode is microseconds each. (DB-1 §2.1's `node_text` is
withdrawn: it was a second copy.)

### 5.5 `concord` section

```sql
CREATE TABLE concord_unit (             -- projection of TextUnit(concord)
  node_id TEXT PRIMARY KEY, part INTEGER NOT NULL, article INTEGER NOT NULL, paragraph INTEGER NOT NULL
) WITHOUT ROWID;
CREATE UNIQUE INDEX concord_by_ref ON concord_unit (part, article, paragraph);
CREATE TABLE reading_spine (ord INTEGER PRIMARY KEY, node_id TEXT NOT NULL);   -- corpus 'concord'
CREATE UNIQUE INDEX spine_by_node ON reading_spine (node_id);

CREATE TABLE contains_concord (         -- same DDL as contains_bible, corpus concord
  id INTEGER PRIMARY KEY, ord INTEGER NOT NULL, container_id TEXT NOT NULL, child_container_id TEXT,
  provenance TEXT NOT NULL, justification_id INTEGER
);
CREATE UNIQUE INDEX contains_concord_ord ON contains_concord (ord);
CREATE TABLE contains_concord_locus (
  contains_id INTEGER NOT NULL, ord INTEGER NOT NULL,
  a INTEGER NOT NULL, b INTEGER NOT NULL, c INTEGER NOT NULL, layer TEXT, start INTEGER, end_ INTEGER,
  PRIMARY KEY (contains_id, ord)
) WITHOUT ROWID;

CREATE TABLE quotes (                   -- LOCUS(quoting), RANGE(quoted)
  id INTEGER PRIMARY KEY, ord INTEGER NOT NULL,
  quoting_corpus TEXT NOT NULL, quoting_a INTEGER NOT NULL, quoting_b INTEGER NOT NULL, quoting_c INTEGER NOT NULL, quoting_layer TEXT, quoting_start INTEGER, quoting_end INTEGER,
  quoted_from_corpus TEXT NOT NULL, quoted_from_a INTEGER NOT NULL, quoted_from_b INTEGER NOT NULL, quoted_from_c INTEGER NOT NULL, quoted_from_layer TEXT, quoted_from_start INTEGER, quoted_from_end INTEGER,
  quoted_to_corpus TEXT NOT NULL, quoted_to_a INTEGER NOT NULL, quoted_to_b INTEGER NOT NULL, quoted_to_c INTEGER NOT NULL, quoted_to_layer TEXT, quoted_to_start INTEGER, quoted_to_end INTEGER,
  provenance TEXT NOT NULL
);
CREATE UNIQUE INDEX quotes_ord ON quotes (ord);

CREATE TABLE confesses (                -- LOCUS(confessing: concord), RANGE(confessed: bible)
  id INTEGER PRIMARY KEY, ord INTEGER NOT NULL,
  confessing_corpus TEXT NOT NULL, confessing_a INTEGER NOT NULL, confessing_b INTEGER NOT NULL, confessing_c INTEGER NOT NULL, confessing_layer TEXT, confessing_start INTEGER, confessing_end INTEGER,
  confessed_from_corpus TEXT NOT NULL, confessed_from_a INTEGER NOT NULL, confessed_from_b INTEGER NOT NULL, confessed_from_c INTEGER NOT NULL, confessed_from_layer TEXT, confessed_from_start INTEGER, confessed_from_end INTEGER,
  confessed_to_corpus TEXT NOT NULL, confessed_to_a INTEGER NOT NULL, confessed_to_b INTEGER NOT NULL, confessed_to_c INTEGER NOT NULL, confessed_to_layer TEXT, confessed_to_start INTEGER, confessed_to_end INTEGER,
  provenance TEXT NOT NULL, justification_id INTEGER
);
CREATE UNIQUE INDEX confesses_ord ON confesses (ord);
```

### 5.6 `kretzmann` section

```sql
CREATE TABLE comments_on (              -- item, RANGE(on)
  id INTEGER PRIMARY KEY, ord INTEGER NOT NULL,
  item_id TEXT NOT NULL,
  on_from_corpus TEXT NOT NULL, on_from_a INTEGER NOT NULL, on_from_b INTEGER NOT NULL, on_from_c INTEGER NOT NULL, on_from_layer TEXT, on_from_start INTEGER, on_from_end INTEGER,
  on_to_corpus TEXT NOT NULL, on_to_a INTEGER NOT NULL, on_to_b INTEGER NOT NULL, on_to_c INTEGER NOT NULL, on_to_layer TEXT, on_to_start INTEGER, on_to_end INTEGER,
  provenance TEXT NOT NULL, justification_id INTEGER
);
CREATE UNIQUE INDEX comments_on_ord ON comments_on (ord);
CREATE INDEX comments_on_by_from ON comments_on (on_from_a, on_from_b, on_from_c, ord);   -- /api/kretzmann/chapter
```

Commentary prose lives in `node.payload` (`CommentaryItem { work,
heading, text }` canonical bytes). One copy.

### 5.7 `lexicon` section (LEX-1) — see §7 for the model

```sql
CREATE TABLE lexicon_entry (            -- projection of LexiconEntry
  node_id TEXT PRIMARY KEY,             -- "LexiconEntry:G3056" / "LexiconEntry:H0430"
  strong TEXT NOT NULL, lang TEXT NOT NULL,          -- 'grc' | 'hbo'
  lemma TEXT NOT NULL, translit TEXT, pos TEXT, root_strong TEXT
) WITHOUT ROWID;
CREATE UNIQUE INDEX lexicon_by_strong ON lexicon_entry (strong);
CREATE INDEX lexicon_by_lemma ON lexicon_entry (lang, lemma);
CREATE TABLE lexicon_domain (           -- semantic domain codes (Louw-Nida / SDBH), for a later thesaurus query
  node_id TEXT NOT NULL, ord INTEGER NOT NULL, code TEXT NOT NULL,
  PRIMARY KEY (node_id, ord)
) WITHOUT ROWID;
CREATE INDEX domain_by_code ON lexicon_domain (code, node_id);

CREATE TABLE token (                    -- the word inventory: every token, matched or not
  book INTEGER NOT NULL, chapter INTEGER NOT NULL, verse INTEGER NOT NULL,
  layer TEXT NOT NULL,                  -- 'greek_textus_receptus' | 'hebrew_masoretic'
  ord INTEGER NOT NULL,                 -- token index within the verse rendering
  form TEXT NOT NULL, lemma TEXT, xpos TEXT, translit TEXT,
  strong TEXT,                          -- NULL when Align=unmatched (by design, ~4.7%)
  aligned INTEGER NOT NULL,             -- 1 matched | 0 unmatched
  PRIMARY KEY (book, chapter, verse, layer, ord)
) WITHOUT ROWID;

CREATE TABLE occurs (                   -- the row family: entry → word locus (imported; no justification)
  id INTEGER PRIMARY KEY, ord INTEGER NOT NULL,
  entry_id TEXT NOT NULL,
  locus_corpus TEXT NOT NULL, locus_a INTEGER NOT NULL, locus_b INTEGER NOT NULL, locus_c INTEGER NOT NULL,
  locus_layer TEXT NOT NULL, locus_start INTEGER NOT NULL, locus_end INTEGER NOT NULL,   -- span = ONE token
  provenance TEXT NOT NULL
);
CREATE UNIQUE INDEX occurs_ord ON occurs (ord);
CREATE INDEX occurs_by_locus ON occurs (locus_a, locus_b, locus_c, locus_layer, locus_start);
```

---

## 6. THE COMPILER SIDE

### 6.1 One build, two encoders (DB-2), then one (DB-5)

`atlas-graph-compile` keeps its shape: build the `Graph` from
adapters, run laws, run the admission gate, write. DB-2 adds a second
writer beside `artifact::dump`; DB-5 deletes the first. The writer:

1. partitions nodes and rows by the `sections.rs` table;
2. for each section, in one transaction with `PRAGMA synchronous = OFF`
   and indexes created AFTER inserts: writes `node`, projections, row
   tables, `justification`/`ground`, then `edge_index` by one
   `INSERT … SELECT` per family from the rows just written (one path —
   nothing for a second path to drift from);
3. computes the logical dump from the in-memory partition (§3.4),
   writes `meta`, `VACUUM`s, compresses with zstd (level 19), names the
   file by the logical hash, records the transport hash;
4. writes `manifest.toml`; computes `root`; stamps `root` into the three
   `data/exports/*.json` files exactly as today.

The compile step gets slower. DB-2's report measures before/after
(§12); the plan does not pretend otherwise.

### 6.2 The admission gate, pointed at the new backend

`assert_answers_match(&SqliteSnapshot::open(sections), &graph)` — the
exact call `bins/compile_graph.rs:250` already makes — over the
compile binary's OWN graph (with Concord, Kretzmann and red-letter; the
test variants that omit them are not this gate). Plus:

- `logical_hash(from Graph partition) == logical_hash(from SQLite
  tables)` per section;
- every widened method equals its default composition over the full
  position inventory (§4);
- the six port methods against the companions they retire (DB-3), over
  the real artifact.

### 6.3 The ETL keeps the in-memory `Graph` (DB-1 OQ-8, option b)

`exports.rs`, every adapter and `law_check.rs` need whole-table access
that the read port deliberately does not offer. They keep it: the
compiler is the one process that holds the whole graph, and it is not
long-running. A separate scan/write port is deferred until a second
consumer wants one.

---

## 7. THE LEXICON SECTION (LEX-1 — the first post-cutover corpus)

### 7.1 What is already in, and what is new

Greek Textus Receptus, Hebrew Masoretic (full pointing) and Finnish
Biblia 1776 are ALREADY rendering layers on all 31,102 verses (CORP-1a,
`atlas-etl/src/brainfuel.rs`). Nothing changes for the texts. The
Greek being the Textus Receptus — the KJV's own base text — keeps the
inerrancy directive intact: no rival manuscript tradition enters.

New (from `brain-fuel/bible` at the SAME pinned commit
`94d44842cb242e8aa840330748e03d2803f2a7c1`, verified present):

| Layer | Upstream | Content | License |
|---|---|---|---|
| lexicon | `lexicon/grc/*.json`, `lexicon/hbo/*.json` | 13,548 entries (5,122 Greek + 8,426 Hebrew): strong, lemma, translit, lang, pos, glosses (en), senses, domains, root, sources | Strong's 1890 PD; glosses/domains CC BY 4.0 (STEPBible TBESG/TAHOT/TAGNT; MACULA) |
| morphology | `morph/nt/<CODE>/NNN.conllu`, `morph/ot/<CODE>/NNN.conllu` | 452,689 tokens (140,610 Greek + 312,079 Hebrew): FORM, LEMMA, XPOS, MISC `Strong=`, `Translit=`, `Align=matched|unmatched` (4.70% / 4.74% unmatched, by design) | CC BY 4.0 (STEPBible) |

`fetch-raw.ps1` extends its existing brain-fuel step to also copy
`lexicon/{grc,hbo}` and `morph/{nt,ot}` (not `morph/lxx`); `data/raw/
README.md` and `LICENSES.md` gain the STEPBible and MACULA attribution
rows verbatim from upstream's README (attribution is a license
condition, not a courtesy).

### 7.2 The node kind

```rust
NodePayload::LexiconEntry {
    strong: String,            // "G3056"
    lang: String,              // "grc" | "hbo"
    lemma: String,
    translit: Option<String>,
    pos: Option<String>,
    glosses: Vec<String>,      // English, source order
    senses: Vec<String>,
    domains: Vec<String>,      // sorted atomic codes
    root: Option<String>,      // Strong's id of root, or None
}
// id: AnyNodeId { kind: LexiconEntry, raw: "G3056" }
```

Corpus role: `Reference` (normed by Scripture, design §2). The
upstream "Yahweh" normalisation of Strong's own glosses is upstream's
build policy on its own PD text; we ingest the JSON as published, and
the seven KJV verses reading "Jehovah" are untouched because the KJV
column is never edited (KJV inerrancy directive).

### 7.3 Words are loci, not nodes

The graph design reserved sub-verse addressing: `Locus.span:
Option<TokenSpan { layer, start, end }>` already exists. A word is
`TextLocus { at: Bible(verse), span: Some(TokenSpan { layer:
greek_textus_receptus | hebrew_masoretic, start: i, end: i }) }`.
Token index `i` is the upstream CoNLL-U token id within the verse,
and the `token` table is the inventory that makes every index
resolvable to a surface form (including unmatched tokens, which have no
edge).

New relation in the `relations!` manifest: `Occurs`
(forward label `occurs-in`, inverse `words`). Row: `Occurs { entry:
LexiconEntryId, locus: TextLocus /* span = one token */, provenance }`.
Imported, no justification (the upstream alignment is the source's
assertion, provenance-tagged `stepbible-tagnt` / `stepbible-tahot`).

Frontier consequences, by construction and nothing else:

- verse → `words` (inverse of Occurs): its tagged tokens in `ord`,
  each explorable to its LexiconEntry;
- LexiconEntry → `occurs-in`: the concordance, at verse granularity,
  in canonical reading order;
- no English-word tagging: upstream aligns only the Greek/Hebrew
  surface, so the concordance is by original-language word. Said
  plainly in the section's `meta` and in LICENSES.md.

### 7.4 Deferred with reasons

- **Relation graph (L2b):** 11.6M edges (synonym/antonym/shared-root/
  domain-sibling/cross-language) with a 0–65535 rank model. Needs its
  own ranking-policy brainstorm (which rank is "an edge we assert"?).
  The shape fits: a `relations` section with a `Related` relation and
  `EdgeMeta::Rank`.
- **Septuagint (`bible/lxx`, `morph/lxx`):** owner's standing "no
  apocrypha for now"; LXX morphology upstream is itself gated on
  STEPBible TAGOT.
- **Thesaurus by domain, cross-translation queries:** `lexicon_domain`
  is written so a later batch can answer them; no endpoint now.

---

## 8. BATCHES (owner-approved order)

Each batch ships green on its own and is worth having if the next never
happens.

| # | Batch | Delivers | Gate |
|---|---|---|---|
| 0 | TOOLCHAIN-1 | `rust-toolchain.toml` (1.97.1) here and in map-generator | full suite green; no behaviour change; version bump none |
| 1 | CONTENTION-1 | the three wall-clock gates (artifact 4 s load, perf_smoke 50 ms, graph_conformance 60 s) moved into a serialized, process-isolated step; ceilings NOT loosened | ten consecutive full runs, zero flips, ceilings unchanged |
| 2 | OVERLAY-1 | `atlas_data_overlay` retired; `/api/scene` + `atlas_core::scene` compose from the port; `verse_text` and `LegacyAtlasFields.verses` deleted; the overlay-vs-compile equivalence test (`server/atlas-graph/tests/overlay_equivalence.rs`, which `legacy.rs:216-224`'s stale disclosure predates) extended FIRST with order and post-`finish()` checks | 25 scene hashes unchanged; Playwright; AQC 44/184; peak RSS before/after by `PeakWorkingSet64`, reported — must fall materially below 751 MiB |
| 3 | DB-2 | `graph-types::canon` (encode/decode + property law); SHA-256-128 `ContentHash` behind a feature flag OFF; `sections.rs`; the section writer beside `graph.bin`; `SqliteSnapshot: GraphQuery + GraphSnapshot`; `rusqlite` (bundled) in `atlas-graph` only | §6.2 gates; `graph-types` still zero-dep (a test asserts the Cargo metadata); version bump none (an unused file is not a contract) |
| 4 | DB-3 | the six port methods + `LexiconEntry`/`Occurs` uninhabited + `ContentHash` widening, as ONE `graph-types` change coordinated with map-generator; companions retired one per equivalence test | per-method equivalence over the real artifact; standing suite; map-generator's `atlas-edge` suite green against a live atlas |
| 5 | DB-4 | cutover: manifest + four sections written and READ; SHA-256-128 ids ON; root widened; sidecars folded; `CommittedZstdSource` + cache; `bibex verify`; `graph.bin` no longer read; AQC fixtures re-recorded through the one assembly path | every gate in §9; version bumps in §9 |
| 6 | DB-5 | delete `artifact.rs` (2,385 lines), `graph.bin`, the bincode encoder, `polities.json`, and the nine folded JSONs | after two green releases on the new backend; `git rm` of `graph.bin` is the commit that finally frees the 100 MiB headroom |
| 7 | LEX-1 | `lexicon` section: fetch extension, adapter, `Occurs` rows, `token` inventory, LICENSES rows | lands with NO change to the loader, the port, or any other section's hash — that invariance IS the acceptance test |

---

## 9. GATES AND VERSION BUMPS AT CUTOVER (DB-4)

**Gates, all existing, none loosened:**

- `scene_byte_identity.rs` — 25 windows, FNV-1a hash + byte length, unchanged.
- `contracts/atlas-graph-contract` (`identity.feature`, `edges.feature`,
  `version-root.feature`) over BOTH transports (HTTP, CLI).
- `contracts/atlas-edge` — map-generator's expectations, against a live atlas.
- `contracts/atlas-query-contract` 44/184; `client.ContractTests` 45/45;
  `client.Tests` 386/386; Playwright UX suite.
- `assert_answers_match` at compile (§6.2).
- Load ceiling 4 s; frontier < 100 ms; perf_smoke — unchanged numbers,
  run in CONTENTION-1's isolated step.

**One new artifact:** a recorded golden transcript of port answers over
a fixed query corpus (`tests/fixtures/port-transcript-v1.json`), so CI
can check the SQLite backend alone after DB-5 removes the second
backend (DB-1 OQ-7, option c).

**Version bumps under the CDC-1 policy** ("0.x: breaking = MINOR,
additive = PATCH"):

| Surface | Now | After | Why |
|---|---|---|---|
| `contracts/atlas-graph-contract` | 0.2.0 | **0.3.0** | root semantics widen; id width changes |
| `contracts/atlas-query-contract` | 0.1.0 | **0.2.0** | behaviour preserved, but 19 blessed fixtures embed the version root (`"version": "82bac0bde5a53ec2"` today) and it moves; under the policy a changed blessed fixture is MINOR. No scenario text changes — if one must, the migration failed |
| `contracts/map-api-consumer` | 0.1.0 | unchanged | our expectations of them |
| `contracts/atlas-edge` | — | unchanged | consumed projections unchanged |
| artifact | `FORMAT_VERSION = 13` (bincode field) | manifest `schema = 1`; section `user_version = 14` | identity moves to the manifest; an old `graph.bin` holder is refused exactly as today |
| `graph-types` | — | one MINOR (DB-3) | C1 covenant change, coordinated |

---

## 10. OUT OF SCOPE (owner rulings)

- Full-text search (FTS5). The migration's claim is "nothing
  observable changes." Search is a feature batch with its own
  brainstorm. The schema does not block it: an FTS5 external-content
  index can sit over a view that decodes `node.payload` through a
  registered scalar function, no second copy of the text.
- Out-of-git section distribution (release assets, object store).
  Implementation #2 of `SectionSource`; lands when a section first
  exceeds the ceiling compressed.
- Mobile shells. Enabled (embeddable, mmap, subset sections), not built.
- The L2b relation graph and the Septuagint (§7.4).

---

## 11. ERROR HANDLING

| Condition | Behaviour |
|---|---|
| manifest `root` mismatch on recompute | refuse to start; print expected/actual |
| `required` section missing | refuse; print name + logical hash |
| transport hash mismatch on unpack | refuse; delete the partial cache file; print both hashes |
| section `user_version` unknown to the binary | refuse, the same way an old `graph.bin` is refused today |
| optional section absent | start; log one line; the section's frontier kinds are simply uninhabited |
| cache directory unwritable | refuse with the path (the tablet path needs a writable cache; a read-only deployment ships pre-unpacked `.sqlite` files and the source finds them first) |
| `bibex verify` any mismatch | non-zero exit; per-section report |

Nothing degrades silently except optional-section absence, and that
silence is the feature.

---

## 12. MEASUREMENTS THE REPORTS MUST STATE

By the same methods DB-1 used, before and after:

- peak resident memory (`PeakWorkingSet64`) after OVERLAY-1 and after DB-4;
- cold start to first `/health` after DB-4 against the 4 s ceiling;
- compile wall time before/after DB-2 and DB-4;
- each compressed section size against 104,857,600 bytes;
- the frontier's p50/p99 on the FQ-1 corpus, before/after DB-4, against 100 ms;
- repository size delta at DB-5 (the `graph.bin` removal).

---

## 13. DB-1'S NINE OPEN QUESTIONS — RULED

| OQ | Ruling |
|---|---|
| 1 overlay retirement own batch | (a) yes — OVERLAY-1, before any schema |
| 2 widen the root | (a) yes, at cutover, one break |
| 3 port widening approval | (a) all six as one contract change; every schema shown before execution (this document) |
| 4 hash + toolchain | toolchain pin immediately; hash replaced at cutover — SHA-256-128 hand-written, not BLAKE3, because `graph-types` is zero-dep |
| 5 CONTENTION-1 prerequisite | (a) yes, before DB-2's gate is trusted |
| 6 bytes or answers | (a) logical identity; file bytes never an identity |
| 7 equivalence corpus | (c) both: `assert_answers_match` + a recorded transcript |
| 8 ETL seam | (b) the compiler keeps the in-memory `Graph`; DB is an output encoding |
| 9 sidecars | (a) all folded, one retired (`polities.json`) |

---

## 14. ACCEPTANCE OF THE DESIGN ITSELF

- A clone runs offline with no network and no build step (manifest +
  committed sections), as today.
- Every observable HTTP response is byte-identical across the cutover
  except hash-derived fields, which move exactly once.
- The version root moves when — and only when — any node, row, spine or
  provenance changes, in any section.
- `derive(pid)` returns bytes that decode to the node and hash to the pid.
- A new corpus (LEX-1) is a new section and changes no other section's
  hash, no loader code, and no port code.
- Peak resident memory is a function of queries run, not of artifact size.
- Every law still runs in the compiler, unchanged, over the in-memory graph.
