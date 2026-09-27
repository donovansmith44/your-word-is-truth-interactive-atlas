use serde::Serialize;

/// Batch F ("the small catechism"), extended Batch F2 (question-level
/// citations): one catechism item citing a verse/span, as embedded in
/// `VerseDetail.catechism` AND returned (as a list) by `GET
/// /api/catechism/{sref}` -- deliberately lean (id + display name + an
/// optional question title, no preview text unlike `CrossRef`):
/// requirement 4's own UI description lists citing items (now
/// question-aware, "<Item> — <Question title>") as plain named entries,
/// nothing more, and the full item content is a separate fetch (`GET
/// /api/catechism/item/{id}`, see `CatechismItem` below), made only once
/// the user actually opens one. `question` is omitted from the wire
/// entirely (not null) when this hit came from Luther's own item-level
/// embedded citation rather than a question -- same conditional-presence
/// convention `CatechismItem.where_written` below and `wire/places.rs`'s own
/// `History.blurb` already use.
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CatechismRef {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub question: Option<String>,
    /// PROV-1 FIX ROUND 1 (review M-3): the `catechism` family's own
    /// distinct provenance set, so a PASSAGE node's THE SMALL CATECHISM
    /// section gets a "?" like a verse's. Same additive element-level move
    /// and same set-not-single-id reasoning as `CrossRef.provenance` --
    /// see that field's doc comment.
    ///
    /// The review judged deferring this half defensible because
    /// `catechism_for_span` took only `State<Arc<AtlasData>>`. Measured, it
    /// is not: `AppState` implements `FromRef` for `Arc<GraphService>` too,
    /// and six handlers in this crate already take BOTH extractors
    /// (`reading::verse` among them). So the second half was one extractor
    /// away, not a larger change, and it ships here.
    ///
    /// This family is the genuinely MULTI-sourced one --
    /// `{concord-sc-overlap, curated-catechism}`, pinned -- which is exactly
    /// why the field is a `Vec` on both endpoints.
    ///
    /// GRANULARITY (fix round 2, review M-NEW-1): SECTION-level, carried on
    /// the element because the endpoint is a bare array. Every element of
    /// one response receives the identical set. Not per-row, and not to be
    /// described as per-row -- see `CrossRef.provenance` for the full
    /// statement and for why per-row is unavailable at these call sites.
    pub provenance: Vec<String>,
}

impl CatechismRef {
    pub(crate) fn attributed(c: atlas_core::catechism::CatechismRef, provenance: &[String]) -> Self {
        CatechismRef { id: c.id, name: c.name, question: c.question, provenance: provenance.to_vec() }
    }
}

/// `GET /api/catechism/item/{id}`'s own wire shape -- `part_title` alongside
/// the item's own fields so the client never needs a second fetch to show
/// "Baptism" as this item's own chief-part context. `where_written` is
/// omitted (not `null`) when absent -- same conditional-presence wire
/// convention `History.blurb`/`established`/`destroyed` already use.
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CatechismItem {
    pub id: String,
    pub name: String,
    pub part_title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    pub explanation_heading: String,
    pub explanation: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub where_written: Option<String>,
    pub verses: Vec<CatechismProofVerse>,
}

/// Batch F: one resolved proof verse -- `vref` + its own FULL KJV text
/// (design-direction.md's house rendering, per requirement 4: "THE
/// SCRIPTURES -- the item's proof verses... full verse text per house
/// rendering" -- not a truncated preview the way `CrossRef.preview` is).
/// Batch F2: `question` (omitted when absent, same convention as
/// `CatechismRef.question`) names which question this proof verse came
/// from, when it came from one -- `None` for Luther's own item-level
/// embedded citations. This is requirement 4's own "if cheap, highlight/
/// deep-link the question context": a small caption next to the verse in
/// THE SCRIPTURES, cheap because it needs no new fetch or scroll machinery,
/// just this one extra field already available at merge time.
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CatechismProofVerse {
    pub vref: String,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub question: Option<String>,
}
