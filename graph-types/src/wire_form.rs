#[cfg(feature = "serde")]
mod serialize {
    use crate::edge::EdgeId;
    use crate::id::{KindTag, NodeId};
    use crate::EdgeKind;
    use core::marker::PhantomData;
    use serde::de::{Error, Visitor};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    impl<K: KindTag> Serialize for NodeId<K> {
        fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
            s.serialize_str(&self.0)
        }
    }

    impl<'de, K: KindTag> Deserialize<'de> for NodeId<K> {
        fn deserialize<D: Deserializer<'de>>(d: D) -> Result<NodeId<K>, D::Error> {
            struct Raw<K>(PhantomData<K>);

            impl<'a, K: KindTag> Visitor<'a> for Raw<K> {
                type Value = NodeId<K>;

                fn expecting(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
                    write!(formatter, "the {}", super::node_id_noun(K::KIND))
                }

                fn visit_str<E: Error>(self, value: &str) -> Result<NodeId<K>, E> {
                    Ok(NodeId::new(value))
                }
            }

            d.deserialize_str(Raw(PhantomData))
        }
    }

    impl Serialize for EdgeId {
        fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
            s.serialize_str(&self.0)
        }
    }

    impl Serialize for EdgeKind {
        fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
            s.serialize_str(self.label())
        }
    }

    impl<'de> Deserialize<'de> for EdgeKind {
        fn deserialize<D: Deserializer<'de>>(d: D) -> Result<EdgeKind, D::Error> {
            struct Label;

            impl<'a> Visitor<'a> for Label {
                type Value = EdgeKind;

                fn expecting(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
                    formatter.write_str("one of the EdgeKind labels")
                }

                fn visit_str<E: Error>(self, value: &str) -> Result<EdgeKind, E> {
                    EdgeKind::from_label(value).ok_or_else(|| E::custom(format_args!("unknown relation label `{value}`")))
                }
            }

            d.deserialize_str(Label)
        }
    }
}

#[cfg(feature = "openapi")]
mod schema {
    use crate::edge::{EdgeId, RelationId, SymRelationId};
    use crate::id::{ContentHash, KindTag, NodeId};
    use crate::vocabulary::string_enum;
    use crate::EdgeKind;
    use std::borrow::Cow;
    use utoipa::openapi::schema::{ObjectBuilder, SchemaType, Type};
    use utoipa::openapi::{RefOr, Schema};
    use utoipa::{PartialSchema, ToSchema};

    const EDGE_KIND: &str = "A relation between two nodes, named in the direction it is travelled: the label one page of a node's neighbours is asked for by.";

    impl PartialSchema for EdgeKind {
        fn schema() -> RefOr<Schema> {
            string_enum(EdgeKind::labels(), EDGE_KIND.to_string())
        }
    }

    impl ToSchema for EdgeKind {}

    const EDGE_ID: &str = "The id of one edge of the graph: the relation it is recorded in, a colon, and its content address. Either end's neighbour page and the element read carry the same id for the same connection.";

    impl PartialSchema for EdgeId {
        fn schema() -> RefOr<Schema> {
            ObjectBuilder::new().schema_type(SchemaType::Type(Type::String)).description(Some(EDGE_ID)).pattern(Some(edge_id_pattern())).into()
        }
    }

    impl ToSchema for EdgeId {}

    fn edge_id_pattern() -> String {
        let relations: Vec<&str> = RelationId::ALL.iter().map(|relation| relation.name()).chain(SymRelationId::ALL.iter().map(|relation| relation.name())).collect();
        format!("^(?:{}):[0-9a-f]{{{}}}$", relations.join("|"), ContentHash::HEX_WIDTH)
    }

    impl<K: KindTag> PartialSchema for NodeId<K> {
        fn schema() -> RefOr<Schema> {
            ObjectBuilder::new().schema_type(SchemaType::Type(Type::String)).description(Some(format!("The {}.", super::node_id_noun(K::KIND)))).into()
        }
    }

    impl<K: KindTag> ToSchema for NodeId<K> {
        fn name() -> Cow<'static, str> {
            Cow::Owned(format!("{}Id", K::KIND.name()))
        }
    }
}

#[cfg(any(feature = "serde", feature = "openapi"))]
fn node_id_noun(kind: crate::id::NodeKind) -> String {
    format!("local name of one {} node, without its kind", kind.name())
}
