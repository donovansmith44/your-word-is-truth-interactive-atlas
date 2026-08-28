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
//! pinning `min_version`/`max_version` to "0.1.0"/"0.1.0" for the drift-
//! failing mechanism the conformance corollary requires).

use axum::Json;
use serde::Serialize;

/// The AQC version range THIS running server supports. Pre-launch (spec
/// §2's semver law), min == max == the one version this codebase currently
/// implements -- there is no "supports a range of prior versions" story
/// yet; that becomes meaningful once a second AQC version ships.
pub const MIN_SUPPORTED_VERSION: &str = "0.1.0";
pub const MAX_SUPPORTED_VERSION: &str = "0.1.0";

#[derive(Debug, Serialize)]
pub struct ContractOut {
    pub min_version: String,
    pub max_version: String,
}

pub async fn contract() -> Json<ContractOut> {
    Json(ContractOut { min_version: MIN_SUPPORTED_VERSION.to_string(), max_version: MAX_SUPPORTED_VERSION.to_string() })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn advertises_the_pinned_aqc_version_range() {
        let Json(body) = contract().await;
        assert_eq!(body.min_version, "0.1.0");
        assert_eq!(body.max_version, "0.1.0");
    }
}
