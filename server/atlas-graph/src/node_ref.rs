use atlas_graph_types::id::{AnyNodeId, NodeKind};

pub fn encode_node_id(id: &AnyNodeId) -> String {
    match id.kind {
        NodeKind::TextUnit => match crate::kjv_adapter::decode_text_unit(id) {
            Some((book, chapter, verse)) => format!("text-unit:{}", crate::kjv_adapter::dot_ref(book, chapter, verse)),
            None => match crate::concord_adapter::decode_text_unit(id) {
                Some((part, article, paragraph)) => format!("text-unit:BoC {part}.{article}.{paragraph}"),
                None => format!("text-unit:{}", id.raw),
            },
        },
        other => format!("{other:?}:{}", id.raw),
    }
}

pub fn node_ref(id: &AnyNodeId, label: String) -> atlas_core::wire::NodeRef {
    atlas_core::wire::NodeRef { id: encode_node_id(id), kind: id.kind, label }
}
