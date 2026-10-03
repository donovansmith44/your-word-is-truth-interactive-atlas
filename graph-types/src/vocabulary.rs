
#[cfg(feature = "openapi")]
pub fn string_enum<V: IntoIterator<Item = &'static str>>(values: V, description: String) -> ::utoipa::openapi::RefOr<::utoipa::openapi::Schema> {
    ::utoipa::openapi::schema::ObjectBuilder::new()
        .schema_type(::utoipa::openapi::schema::SchemaType::Type(::utoipa::openapi::schema::Type::String))
        .enum_values(Some(values))
        .description(Some(description))
        .into()
}

#[macro_export]
macro_rules! vocabulary {
    (
        $(#[doc = $doc:expr])*
        $(#[derive($($also:path),+ $(,)?)])?
        $name:ident { $($member:ident => $wire:expr),+ $(,)? }
    ) => {
        $(#[doc = $doc])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq $($(, $also)+)?)]
        pub enum $name { $($member),+ }

        impl $name {
            pub const ALL: [$name; [$($wire),+].len()] = [$($name::$member),+];

            pub const fn name(self) -> &'static str {
                match self { $($name::$member => $wire),+ }
            }

            pub fn named(name: &str) -> Option<$name> {
                Self::ALL.into_iter().find(|member| member.name() == name)
            }
        }

        $crate::vocabulary_wire_form! { $name { $($member => $wire),+ } }
        $crate::vocabulary_schema! { $name { $($doc),* } }
    };
    (
        $(#[doc = $doc:expr])*
        $(#[derive($($also:path),+ $(,)?)])?
        $name:ident { $($member:ident),+ $(,)? }
    ) => {
        $crate::vocabulary! {
            $(#[doc = $doc])*
            $(#[derive($($also),+)])?
            $name { $($member => ::core::stringify!($member)),+ }
        }
    };
}

#[cfg(feature = "serde")]
#[doc(hidden)]
#[macro_export]
macro_rules! vocabulary_wire_form {
    ($name:ident { $($member:ident => $wire:expr),+ }) => {
        impl ::serde::Serialize for $name {
            fn serialize<S: ::serde::Serializer>(&self, serializer: S) -> ::core::result::Result<S::Ok, S::Error> {
                serializer.serialize_str(self.name())
            }
        }

        impl<'de> ::serde::Deserialize<'de> for $name {
            fn deserialize<D: ::serde::Deserializer<'de>>(deserializer: D) -> ::core::result::Result<$name, D::Error> {
                // A visitor rather than `String::deserialize`, which would need an
                // allocating serde this crate does not ask for.
                struct Word;

                impl<'a> ::serde::de::Visitor<'a> for Word {
                    type Value = $name;

                    fn expecting(&self, formatter: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                        formatter.write_str(::core::concat!("one of the ", ::core::stringify!($name), " values"))
                    }

                    fn visit_str<E: ::serde::de::Error>(self, value: &str) -> ::core::result::Result<$name, E> {
                        $name::named(value).ok_or_else(|| E::unknown_variant(value, &[$($wire),+]))
                    }
                }

                deserializer.deserialize_str(Word)
            }
        }
    };
}

#[cfg(not(feature = "serde"))]
#[doc(hidden)]
#[macro_export]
macro_rules! vocabulary_wire_form {
    ($($unused:tt)*) => {};
}

#[cfg(feature = "openapi")]
#[doc(hidden)]
#[macro_export]
macro_rules! vocabulary_schema {
    ($name:ident { $($doc:expr),* }) => {
        impl ::utoipa::PartialSchema for $name {
            fn schema() -> ::utoipa::openapi::RefOr<::utoipa::openapi::Schema> {
                $crate::vocabulary::string_enum(
                    Self::ALL.map(Self::name),
                    [$($doc),*].map(|line: &str| line.strip_prefix(' ').unwrap_or(line)).join("\n"),
                )
            }
        }

        impl ::utoipa::ToSchema for $name {}
    };
}

#[cfg(not(feature = "openapi"))]
#[doc(hidden)]
#[macro_export]
macro_rules! vocabulary_schema {
    ($($unused:tt)*) => {};
}
