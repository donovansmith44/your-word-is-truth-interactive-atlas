#[cfg(feature = "serde")]
mod serialize {
    use crate::{EdgeKind, NodeKind};
    use serde::{Serialize, Serializer};

    impl Serialize for NodeKind {
        fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
            s.serialize_str(self.name())
        }
    }

    impl Serialize for EdgeKind {
        fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
            s.serialize_str(self.label())
        }
    }
}

#[cfg(feature = "openapi")]
mod schema {
    use crate::{EdgeKind, NodeKind};
    use utoipa::openapi::schema::{ObjectBuilder, SchemaType, Type};
    use utoipa::openapi::{RefOr, Schema};
    use utoipa::{PartialSchema, ToSchema};

    fn string_enum<'a>(values: impl Iterator<Item = &'a str>) -> RefOr<Schema> {
        ObjectBuilder::new()
            .schema_type(SchemaType::Type(Type::String))
            .enum_values(Some(values.collect::<Vec<_>>()))
            .into()
    }

    impl PartialSchema for NodeKind {
        fn schema() -> RefOr<Schema> {
            string_enum(NodeKind::ALL.iter().map(|k| k.name()))
        }
    }
    impl ToSchema for NodeKind {}

    impl PartialSchema for EdgeKind {
        fn schema() -> RefOr<Schema> {
            string_enum(EdgeKind::labels())
        }
    }
    impl ToSchema for EdgeKind {}
}
