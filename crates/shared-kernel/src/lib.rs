//! Pure shared types: typed IDs and the `Clock` port. No I/O.

#![forbid(unsafe_code)]

mod clock;
mod id;

pub use clock::{Clock, SystemClock};
pub use id::{IdGenerator, UuidV7Generator};
pub use uuid::Uuid;

/// Declares a typed UUID newtype ID.
#[macro_export]
macro_rules! typed_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord,
                 ::serde::Serialize, ::serde::Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub $crate::Uuid);

        impl $name {
            /// Wraps an existing UUID.
            #[must_use]
            pub const fn from_uuid(value: $crate::Uuid) -> Self {
                Self(value)
            }

            /// Returns the inner UUID.
            #[must_use]
            pub const fn as_uuid(&self) -> $crate::Uuid {
                self.0
            }
        }

        impl ::std::fmt::Display for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                self.0.fmt(f)
            }
        }
    };
}
