//! Pure domain: Yjs document state and wire protocol. No I/O.

pub mod protocol;
mod sync_state;

pub use sync_state::{SyncError, SyncState};
