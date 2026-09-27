//! One declaration per closed vocabulary.

/// Declares a closed wire vocabulary. `$wire` is an expression rather than a
/// literal, so a vocabulary whose strings already exist as constants names those
/// constants instead of re-typing their values.
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

            pub fn name(self) -> &'static str {
                match self { $($name::$variant => $wire),+ }
            }

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

        // A FLAT string enum, never a `oneOf`: the generated client turns this
        // component into one enum of its own. The description is written out
        // because a hand-written schema gets none of utoipa's derive.
        impl ::utoipa::PartialSchema for $name {
            fn schema() -> ::utoipa::openapi::RefOr<::utoipa::openapi::Schema> {
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
