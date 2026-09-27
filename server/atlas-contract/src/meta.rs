//! The meta family -- `GET /health`, `GET /api/contract`, `GET /api/sources`.
//!
//! `GET /api/contract` -- Batch AQC-1's own ONE new behavioral surface
//! (design spec §2's versioning law: "The server advertises its supported
//! contract range at `/api/contract` (new, tiny endpoint); the client
//! checks at startup and fails LOUD on mismatch"). Every other AQC-1
//! deliverable is a SNAPSHOT (zero behavior change) -- this endpoint is the
//! sole exception, additive-only, no pre-existing route touched.
//!
//! The advertised range is a compile-time constant, not derived from
//! anything else in this crate (the AQC document itself,
//! `contracts/atlas-query-contract/VERSION`, is the one hand-maintained
//! source of truth for what version this server was built to serve --
//! keeping this endpoint's own constants in lockstep with that file is a
//! release-process discipline, the same as any other "generated from one
//! source" pairing in this repo; see `versioning.feature`'s own scenario
//! pinning `min_version`/`max_version` to "0.7.0"/"0.7.0" (D5; 0.6.0 D3; 0.5.0 LEX-1; 0.4.0 DB-4c, 0.3.0 DB-4b, 0.2.0 DB-4a, 0.1.0 before) for the drift-
//! failing mechanism the conformance corollary requires).

use std::sync::Arc;

use axum::extract::State;
use axum::Json;

use atlas_core::sources::SourcesDocument;

use crate::wire;

pub async fn health() -> &'static str {
    "ok"
}

/// `GET /api/sources` (batch-s-brief.md requirement 3): the Sources
/// page's entire single source of truth, straight off
/// `data/compiled/sources.json` (itself generated 1:1 from LICENSES.md by
/// `atlas_etl::sources`'s own fail-loud drift check -- see the
/// `gen_sources` binary). The client renders this directly; nothing here
/// is a hardcoded duplicate list.
pub async fn sources(State(sources): State<Arc<SourcesDocument>>) -> Json<SourcesDocument> {
    Json((*sources).clone())
}

/// The AQC version range THIS running server supports. Pre-launch (spec
/// §2's semver law), min == max == the one version this codebase currently
/// implements -- there is no "supports a range of prior versions" story
/// yet; that becomes meaningful once a second AQC version ships.
pub const MIN_SUPPORTED_VERSION: &str = "0.7.0";
pub const MAX_SUPPORTED_VERSION: &str = "0.7.0";

pub async fn contract() -> Json<wire::Contract> {
    Json(wire::Contract {
        min_version: MIN_SUPPORTED_VERSION.to_string(),
        max_version: MAX_SUPPORTED_VERSION.to_string(),
        manifest_schema: atlas_graph::sqlite::manifest::MANIFEST_SCHEMA,
        section_schema_version: atlas_graph::sections::SECTION_SCHEMA_VERSION,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn advertises_the_pinned_aqc_version_range() {
        let Json(body) = contract().await;
        assert_eq!(body.min_version, "0.7.0");
        assert_eq!(body.max_version, "0.7.0");
        assert_eq!((body.manifest_schema, body.section_schema_version), (1, 14));
    }
}
