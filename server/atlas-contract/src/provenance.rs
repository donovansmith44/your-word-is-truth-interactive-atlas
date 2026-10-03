use atlas_core::data::AtlasData;

use crate::error::ApiError;
use crate::wire;

pub fn titled(id: &str, data: &AtlasData) -> Result<wire::Provenance, ApiError> {
    data.provenance_titles
        .title_of(id)
        .map(|title| wire::Provenance { id: id.to_string(), title: title.to_string() })
        .ok_or_else(|| ApiError::internal(&format!("the provenance {id} has no compiled title")))
}

pub fn all_titled(ids: &[String], data: &AtlasData) -> Result<Vec<wire::Provenance>, ApiError> {
    ids.iter().map(|id| titled(id, data)).collect()
}
