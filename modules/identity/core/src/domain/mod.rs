//! Pure domain: no I/O.

mod user;
mod values;

pub use user::User;
pub use values::{AvatarUrl, Bio, DisplayName, Email, Password, ValueError};
