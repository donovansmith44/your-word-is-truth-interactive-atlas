use atlas_graph_types::{EdgeKind, NodeKind};
use serde::Serialize;

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct NodeCard {
    pub id: String,
    pub kind: NodeKind,
    pub label: String,
    pub provenance: String,
    pub edge_summary: Vec<EdgeSummaryEntry>,
    pub version: String,
    /// D5: present for a Person only (omitted, never null, otherwise).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub person: Option<PersonLife>,
    /// ENT-1a: Easton's Bible Dictionary (1897, PD) prose, source-attested,
    /// `None` until a match exists -- same additive-JSON, same held-client
    /// disclosure as `wire::PlaceDetail::description`. This is the
    /// ONLY "detail" surface a Person or PeopleGroup node has at all
    /// (`graph_wire.rs`'s own doc comment: no dedicated per-kind endpoint
    /// exists for either), so widening the generic card here is what
    /// actually reaches them; it reaches Place/PeopleGroup for free too
    /// (the same payload fact, whichever kind carries it).
    ///
    /// Batch CORP-1b: the SAME field, widened again -- a CommentaryItem's
    /// own prose (`NodePayload::CommentaryItem.text`) rides here too now
    /// (`atlas_graph::legacy::node_description`'s own updated match), for
    /// the identical reason: no dedicated per-kind endpoint exists for
    /// CommentaryItem either, and the prose was already sitting on the
    /// compiled graph payload, just never read.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// D5 (owner, 2026-09-15; additive, AQC 0.7.0): a Person card's life facts,
/// straight off the payload. `birth_year`/`death_year` are the source's
/// own life dates (75 / 64 of 3,067 persons carry one); `first_year`/
/// `last_year` are the CORPUS-mention span, never a lifespan; `eternal`
/// with its Scripture `eternal_grounds` is the curated exception ("God
/// because he is eternal") -- an eternal person shows no years at all.
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PersonLife {
    pub gender: Option<String>,
    pub birth_year: Option<i32>,
    pub death_year: Option<i32>,
    pub first_year: Option<i32>,
    pub last_year: Option<i32>,
    pub eternal: bool,
    pub eternal_grounds: Vec<String>,
    pub also_called: Vec<String>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct EdgeSummaryEntry {
    pub kind: EdgeKind,
    pub count: usize,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct EdgePage {
    pub kind: EdgeKind,
    pub entries: Vec<EdgeEntry>,
    pub next: Option<usize>,
    pub version: String,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct EdgeEntry {
    /// The bijection witness travels on the wire (M-A brief requirement 4):
    /// the SAME id a caller sees here is what the target's own inverse-kind
    /// page carries back for this same connection.
    pub edge: String,
    pub node: NodeRef,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct NodeRef {
    pub id: String,
    /// A `String`, not a `NodeKind`: an edge takes focus too (design doc
    /// §0), so a frontier entry's position can be an edge -- a
    /// `justified-by` row reached through its own `justifies` frontier has
    /// no node kind at all, and carries `"Edge"` here.
    pub kind: String,
    pub label: String,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct TextWindow {
    pub units: Vec<TextUnit>,
    pub next: Option<String>,
    pub version: String,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct TextUnit {
    #[serde(rename = "ref")]
    pub sref: String,
    pub text: String,
    /// Batch RED-1: this unit's own aligned sub-verse red-letter spans --
    /// see `wire::Verse.words_of_christ`'s own doc comment
    /// (identical shape/convention). Always empty for `corpus=concord`
    /// (a wholly different corpus, never the KJV -- decision 5's own
    /// sub-verse precision is KJV-specific by construction).
    pub words_of_christ: Vec<super::reading::WordsOfChristSpan>,
    /// D3 (owner, 2026-09-15; additive, AQC 0.6.0): this unit's own
    /// inhabited frontier kinds and counts -- the SAME `edge_summary`
    /// shape `NodeCard` carries, so a corpus page can make ONLY units
    /// with edges clickable (no dead clicks) without an N+1 of node-card
    /// calls. The reader may ignore it.
    pub edge_summary: Vec<EdgeSummaryEntry>,
}
