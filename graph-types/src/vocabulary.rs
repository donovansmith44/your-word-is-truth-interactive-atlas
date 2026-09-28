//! One mechanism for every closed vocabulary this workspace declares.

/// Declares a closed vocabulary: its members, the one string each is written as,
/// and the wire form and schema that publish both.
///
/// `$wire` is an expression rather than a literal, so a vocabulary whose strings
/// already exist as constants names those constants instead of re-typing their
/// values; a member list written without `=>` is spelled by its own member names.
/// `Deserialize` is generated for every vocabulary and not only for those a
/// caller reads back: a set that can be written and never read is half a type,
/// and one uniform expansion leaves no flag to set the wrong way.
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

            pub fn name(self) -> &'static str {
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

/// The wire half of [`vocabulary!`]. Its feature is read where it is declared
/// rather than at the invocation, so a crate that owns a vocabulary needs no
/// feature of its own to publish one.
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

/// The schema half of [`vocabulary!`], feature-read where it is declared for the
/// same reason as [`vocabulary_wire_form!`].
#[cfg(feature = "openapi")]
#[doc(hidden)]
#[macro_export]
macro_rules! vocabulary_schema {
    ($name:ident { $($doc:expr),* }) => {
        // A FLAT string enum, never a `oneOf`: the generated client turns this
        // component into one enum of its own. The description is assembled here
        // because a hand-written schema gets none of a derive's own reading of
        // the doc comment.
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

#[cfg(not(feature = "openapi"))]
#[doc(hidden)]
#[macro_export]
macro_rules! vocabulary_schema {
    ($($unused:tt)*) => {};
}
