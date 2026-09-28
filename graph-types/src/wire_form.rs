/// `EdgeKind` is a relation paired with a direction rather than a flat list of
/// members, so it carries the wire form a closed vocabulary would have generated.
#[cfg(feature = "serde")]
mod serialize {
    use crate::EdgeKind;
    use serde::de::{Error, Visitor};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

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
    use crate::EdgeKind;
    use utoipa::openapi::schema::{ObjectBuilder, SchemaType, Type};
    use utoipa::openapi::{RefOr, Schema};
    use utoipa::{PartialSchema, ToSchema};

    const EDGE_KIND: &str = "A relation between two nodes, named in the direction it is travelled: the label one frontier of a node is asked for by.";

    impl PartialSchema for EdgeKind {
        fn schema() -> RefOr<Schema> {
            ObjectBuilder::new()
                .schema_type(SchemaType::Type(Type::String))
                .enum_values(Some(EdgeKind::labels().collect::<Vec<_>>()))
                .description(Some(EDGE_KIND))
                .into()
        }
    }

    impl ToSchema for EdgeKind {}
}
