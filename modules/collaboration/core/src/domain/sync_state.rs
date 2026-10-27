use thiserror::Error;
use yrs::{Doc, ReadTxn, StateVector, Transact, Update, updates::decoder::Decode};

/// A Yjs update could not be applied.
#[derive(Debug, Error, PartialEq, Eq)]
#[error("malformed update")]
pub struct SyncError;

/// Server-side replica of a collaborative document.
#[derive(Debug, Default)]
pub struct SyncState {
    doc: Doc,
}

impl SyncState {
    /// Creates an empty replica.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Applies a v1 update.
    ///
    /// # Errors
    /// [`SyncError`] when the bytes are not a valid update.
    pub fn apply_update(&self, bytes: &[u8]) -> Result<(), SyncError> {
        let update = Update::decode_v1(bytes).map_err(|_| SyncError)?;
        self.doc
            .transact_mut()
            .apply_update(update)
            .map_err(|_| SyncError)
    }

    /// Current state vector.
    #[must_use]
    pub fn state_vector(&self) -> StateVector {
        self.doc.transact().state_vector()
    }

    /// Everything the peer with `remote` is missing.
    #[must_use]
    pub fn diff_since(&self, remote: &StateVector) -> Vec<u8> {
        self.doc.transact().encode_diff_v1(remote)
    }

    /// The complete state as a single update (used for snapshots).
    #[must_use]
    pub fn encode_state(&self) -> Vec<u8> {
        self.doc
            .transact()
            .encode_state_as_update_v1(&StateVector::default())
    }
}

#[cfg(test)]
mod tests {
    use yrs::{GetString, Text};

    use super::*;

    /// Produces an update that inserts `text` at the start of the shared text "t".
    pub(crate) fn insert_update(doc: &Doc, text: &str) -> Vec<u8> {
        let before = doc.transact().state_vector();
        let handle = doc.get_or_insert_text("t");
        handle.insert(&mut doc.transact_mut(), 0, text);
        doc.transact().encode_diff_v1(&before)
    }

    fn content(state: &SyncState) -> String {
        let text = state.doc.get_or_insert_text("t");
        text.get_string(&state.doc.transact())
    }

    #[test]
    fn concurrent_edits_converge_in_any_order() {
        let (a, b) = (Doc::new(), Doc::new());
        let (ua, ub) = (insert_update(&a, "hello "), insert_update(&b, "world"));

        let (left, right) = (SyncState::new(), SyncState::new());
        left.apply_update(&ua).unwrap();
        left.apply_update(&ub).unwrap();
        right.apply_update(&ub).unwrap();
        right.apply_update(&ua).unwrap();

        assert_eq!(content(&left), content(&right));
        assert_eq!(content(&left).len(), "hello world".len());
    }

    #[test]
    fn diff_brings_a_fresh_replica_up_to_date() {
        let source = SyncState::new();
        source
            .apply_update(&insert_update(&Doc::new(), "abc"))
            .unwrap();

        let fresh = SyncState::new();
        fresh
            .apply_update(&source.diff_since(&fresh.state_vector()))
            .unwrap();
        assert_eq!(content(&fresh), "abc");

        let restored = SyncState::new();
        restored.apply_update(&source.encode_state()).unwrap();
        assert_eq!(content(&restored), "abc");
    }

    #[test]
    fn rejects_garbage() {
        assert_eq!(
            SyncState::new().apply_update(&[0xff, 0xff, 0xff]),
            Err(SyncError)
        );
    }
}
