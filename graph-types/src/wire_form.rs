/// `EdgeKind` is a relation paired with a direction rather than a flat list of
/// members, so it carries the wire form a closed vocabulary would have generated.
/// A typed node id travels as its raw id alone: the kind is the field's own type,
/// so writing it beside the id would say twice what the document already says.
#[cfg(feature = "serde")]
mod serialize {
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
    use crate::id::{KindTag, NodeId};
    use crate::vocabulary::string_enum;
    use crate::EdgeKind;
    use std::borrow::Cow;
    use utoipa::openapi::schema::{ObjectBuilder, SchemaType, Type};
    use utoipa::openapi::{RefOr, Schema};
    use utoipa::{PartialSchema, ToSchema};

    const EDGE_KIND: &str = "A relation between two nodes, named in the direction it is travelled: the label one frontier of a node is asked for by.";

    impl PartialSchema for EdgeKind {
        fn schema() -> RefOr<Schema> {
            string_enum(EdgeKind::labels(), EDGE_KIND.to_string())
        }
    }

    impl ToSchema for EdgeKind {}

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
    format!("id of one {} node", kind.name())
}
