use atlas_graph_types::id::{AnyNodeId, NodeKind};
use atlas_graph_types::store::GraphQuery;

const TEXT_UNIT: &str = "text-unit";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnreferencedUnit(pub AnyNodeId);

pub fn encode_node_id(id: &AnyNodeId, query: &(impl GraphQuery + ?Sized)) -> Result<String, UnreferencedUnit> {
    encode_node_ids(std::slice::from_ref(id), query).map(|mut encoded| encoded.remove(0))
}

pub fn encode_node_ids(ids: &[AnyNodeId], query: &(impl GraphQuery + ?Sized)) -> Result<Vec<String>, UnreferencedUnit> {
    let units: Vec<AnyNodeId> = ids.iter().filter(|id| id.kind == NodeKind::TextUnit).cloned().collect();
    let mut references = query.references(&units).into_iter();
    ids.iter()
        .map(|id| match id.kind {
            NodeKind::TextUnit => references.next().flatten().map(|reference| format!("{TEXT_UNIT}:{reference}")).ok_or_else(|| UnreferencedUnit(id.clone())),
            other => Ok(format!("{other:?}:{}", id.raw)),
        })
        .collect()
}

pub fn node_ref(id: &AnyNodeId, label: String, query: &(impl GraphQuery + ?Sized)) -> Result<atlas_core::wire::NodeRef, UnreferencedUnit> {
    Ok(atlas_core::wire::NodeRef { id: encode_node_id(id, query)?, kind: id.kind, label })
}
