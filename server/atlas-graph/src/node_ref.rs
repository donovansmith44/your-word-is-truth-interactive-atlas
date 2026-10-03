use atlas_core::identity::{NodeId, UnreferencedUnit};
use atlas_graph_types::id::AnyNodeId;
use atlas_graph_types::store::GraphQuery;

pub fn node_ref(id: &AnyNodeId, label: String, query: &(impl GraphQuery + ?Sized)) -> Result<atlas_core::wire::NodeRef, UnreferencedUnit> {
    Ok(atlas_core::wire::NodeRef { id: NodeId::encoded_one(id, query)?, kind: id.kind, label })
}
