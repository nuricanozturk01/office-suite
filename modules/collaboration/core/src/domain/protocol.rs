//! y-protocols framing (sync + awareness) on top of `yrs`.

use thiserror::Error;
use yrs::{
    StateVector,
    sync::{Message, SyncMessage},
    updates::{decoder::Decode, encoder::Encode},
};

/// A frame could not be decoded.
#[derive(Debug, Error, PartialEq, Eq)]
#[error("malformed frame")]
pub struct ProtocolError;

/// What the server cares about in a client frame.
#[derive(Debug, PartialEq, Eq)]
pub enum Incoming {
    /// Peer announces what it has and asks for the rest.
    SyncStep1(StateVector),
    /// Document changes (both `SyncStep2` and `Update` carry updates).
    Update(Vec<u8>),
    /// Presence/cursor state, relayed verbatim to other peers.
    Awareness,
    /// Valid but irrelevant to the server.
    Ignored,
}

/// Decodes a client frame.
///
/// # Errors
/// [`ProtocolError`] when the frame is not valid y-protocols data.
pub fn parse(bytes: &[u8]) -> Result<Incoming, ProtocolError> {
    Ok(
        match Message::decode_v1(bytes).map_err(|_| ProtocolError)? {
            Message::Sync(SyncMessage::SyncStep1(sv)) => Incoming::SyncStep1(sv),
            Message::Sync(SyncMessage::SyncStep2(update) | SyncMessage::Update(update)) => {
                Incoming::Update(update)
            }
            Message::Awareness(_) => Incoming::Awareness,
            _ => Incoming::Ignored,
        },
    )
}

/// Frame: "this is what I have".
#[must_use]
pub fn step1(sv: StateVector) -> Vec<u8> {
    Message::Sync(SyncMessage::SyncStep1(sv)).encode_v1()
}

/// Frame: reply to a step 1 with the missing data.
#[must_use]
pub fn step2(update: Vec<u8>) -> Vec<u8> {
    Message::Sync(SyncMessage::SyncStep2(update)).encode_v1()
}

/// Frame: incremental update.
#[must_use]
pub fn update(update: Vec<u8>) -> Vec<u8> {
    Message::Sync(SyncMessage::Update(update)).encode_v1()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrips_sync_frames() {
        let sv = StateVector::default();
        assert_eq!(parse(&step1(sv.clone())), Ok(Incoming::SyncStep1(sv)));
        assert_eq!(parse(&update(vec![0, 0])), Ok(Incoming::Update(vec![0, 0])));
        assert_eq!(parse(&step2(vec![0, 0])), Ok(Incoming::Update(vec![0, 0])));
    }

    #[test]
    fn rejects_garbage() {
        assert_eq!(parse(&[0xff, 0xff, 0xff, 0xff]), Err(ProtocolError));
    }
}
