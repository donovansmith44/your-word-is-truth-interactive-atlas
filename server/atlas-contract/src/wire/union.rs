use utoipa::openapi::extensions::Extensions;
use utoipa::openapi::schema::{AdditionalProperties, AllOfBuilder, ObjectBuilder, Schema, SchemaType, Type};
use utoipa::openapi::{Ref, RefOr};

pub(super) struct Case {
    pub tag: &'static str,
    pub name: &'static str,
    pub description: &'static str,
}

pub(super) fn tagged_by(tag: &str, cases: &[Case], description: &str) -> RefOr<Schema> {
    let tags = ObjectBuilder::new().schema_type(SchemaType::Type(Type::String)).enum_values(Some(cases.iter().map(|case| case.tag)));
    let mapping: serde_json::Map<String, serde_json::Value> =
        cases.iter().map(|case| (case.tag.to_string(), Ref::from_schema_name(case.name).ref_location.into())).collect();
    ObjectBuilder::new()
        .schema_type(SchemaType::Type(Type::Object))
        .description(Some(description))
        .property(tag, tags)
        .required(tag)
        .additional_properties(Some(AdditionalProperties::FreeForm(true)))
        .extensions(Some(Extensions::from_iter([("discriminator", serde_json::json!({ "propertyName": tag, "mapping": mapping }))])))
        .into()
}

pub(super) fn case_of<const N: usize>(base: &str, case: &Case, parts: [(&str, RefOr<Schema>); N]) -> RefOr<Schema> {
    let own = parts.into_iter().fold(ObjectBuilder::new().schema_type(SchemaType::Type(Type::Object)), |object, (name, part)| object.property(name, part).required(name));
    AllOfBuilder::new()
        .item(Ref::from_schema_name(base))
        .item(own)
        .description(Some(case.description))
        .extensions(Some(Extensions::from_iter([("unevaluatedProperties", false)])))
        .into()
}
