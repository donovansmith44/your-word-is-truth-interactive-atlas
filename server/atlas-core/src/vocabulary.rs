//! One declaration per closed vocabulary.
//!
//! A vocabulary the SERVER chooses from a fixed set is named once, as a list
//! of variants and the strings they ride the wire as. Everything else --
//! `ALL`, the parse, the serialised form, the schema's `enum` values -- is
//! read off that list, so no two of them can drift apart and none of them can
//! go stale when a variant is added. The generated `ALL` is the exhaustiveness
//! anchor: it is built from the same list the `match` in `name` is, so a new
//! variant appears in both or in neither.
//!
//! The shape follows `atlas_graph_types::id::node_kinds!`, the house pattern
//! for the same problem, and generalises the hand-written `Serialize`/
//! `PartialSchema` pair `atlas_contract::wire::PositionKind` already carries.

/// Declares a closed wire vocabulary. `$wire` is an expression, not a literal,
/// so a vocabulary whose strings already exist as constants elsewhere (the
/// graph's own reading-spine keys, say) names those constants rather than
/// re-typing their values.
#[macro_export]
macro_rules! vocabulary {
    (
        $(#[doc = $doc:expr])*
        $name:ident { $($variant:ident => $wire:expr),+ $(,)? }
    ) => {
        $(#[doc = $doc])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum $name { $($variant),+ }

        impl $name {
            pub const ALL: [$name; [$($wire),+].len()] = [$($name::$variant),+];

            /// The string this value rides the wire as.
            pub fn name(self) -> &'static str {
                match self { $($name::$variant => $wire),+ }
            }

            /// The total inverse of `name`.
            pub fn named(name: &str) -> Option<$name> {
                Self::ALL.into_iter().find(|value| value.name() == name)
            }
        }

        impl ::serde::Serialize for $name {
            fn serialize<S: ::serde::Serializer>(&self, serializer: S) -> ::core::result::Result<S::Ok, S::Error> {
                serializer.serialize_str(self.name())
            }
        }

        impl<'de> ::serde::Deserialize<'de> for $name {
            fn deserialize<D: ::serde::Deserializer<'de>>(deserializer: D) -> ::core::result::Result<$name, D::Error> {
                let name = <::std::string::String as ::serde::Deserialize>::deserialize(deserializer)?;
                $name::named(&name).ok_or_else(|| {
                    ::serde::de::Error::unknown_variant(&name, &[$($wire),+])
                })
            }
        }

        /// A FLAT string enum, never a `oneOf`: the generated client turns this
        /// component into one enum of its own.
        impl ::utoipa::PartialSchema for $name {
            fn schema() -> ::utoipa::openapi::RefOr<::utoipa::openapi::Schema> {
                // The doc comment above reaches the published document as this
                // component's description; utoipa's own derive is what carries
                // it for every hand-derived schema, and this one is written out.
                ::utoipa::openapi::schema::ObjectBuilder::new()
                    .schema_type(::utoipa::openapi::schema::SchemaType::Type(::utoipa::openapi::schema::Type::String))
                    .enum_values(Some(Self::ALL.map(Self::name)))
                    .description(Some([$($doc),*].map(|line: &str| line.strip_prefix(' ').unwrap_or(line)).join("\n")))
                    .into()
            }
        }

        impl ::utoipa::ToSchema for $name {}
    };
}
