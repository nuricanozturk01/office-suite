//! Cross-instance bus implementations.

use std::sync::Arc;

use async_trait::async_trait;
use documents_contracts::DocumentId;
use platform::docbus::{DocBus, RecvError, Subscription};
use shared_kernel::Uuid;

use crate::application::{BusKind, BusMessage, BusStream, StoreError, UpdateBus};

const HEADER_LEN: usize = 16 + 1 + 8;
const KIND_UPDATE: u8 = 0;
const KIND_AWARENESS: u8 = 1;

/// Single-instance setups and tests: nothing is published, nothing arrives.
#[derive(Debug, Clone, Copy)]
pub(crate) struct NoBus;

struct Silent;

#[async_trait]
impl BusStream for Silent {
    async fn next(&mut self) -> Option<BusMessage> {
        std::future::pending().await
    }
}

#[async_trait]
impl UpdateBus for NoBus {
    async fn publish(&self, _document: DocumentId, _message: BusMessage) {}

    async fn subscribe(&self, _document: DocumentId) -> Result<Box<dyn BusStream>, StoreError> {
        Ok(Box::new(Silent))
    }
}

/// Redis pub/sub bus, channel `collab.<document_id>`.
pub(crate) struct RedisBus {
    bus: Arc<DocBus>,
}

impl RedisBus {
    pub(crate) const fn new(bus: Arc<DocBus>) -> Self {
        Self { bus }
    }
}

fn channel(document: DocumentId) -> String {
    format!("collab.{document}")
}

fn encode(message: &BusMessage) -> Vec<u8> {
    let (kind, seq) = match message.kind {
        BusKind::Update { seq } => (KIND_UPDATE, seq),
        BusKind::Awareness => (KIND_AWARENESS, 0),
    };
    let mut out = Vec::with_capacity(HEADER_LEN + message.bytes.len());
    out.extend_from_slice(message.instance.as_bytes());
    out.push(kind);
    out.extend_from_slice(&seq.to_be_bytes());
    out.extend_from_slice(&message.bytes);
    out
}

fn decode(raw: &[u8]) -> Option<BusMessage> {
    let header = raw.get(..HEADER_LEN)?;
    let instance = Uuid::from_slice(header.get(..16)?).ok()?;
    let seq = i64::from_be_bytes(header.get(17..25)?.try_into().ok()?);
    let kind = match *header.get(16)? {
        KIND_UPDATE => BusKind::Update { seq },
        KIND_AWARENESS => BusKind::Awareness,
        _ => return None,
    };
    Some(BusMessage {
        instance,
        kind,
        bytes: Arc::from(raw.get(HEADER_LEN..)?),
    })
}

struct RedisStream(Subscription);

#[async_trait]
impl BusStream for RedisStream {
    async fn next(&mut self) -> Option<BusMessage> {
        loop {
            match self.0.recv().await {
                Ok(raw) => {
                    if let Some(message) = decode(&raw) {
                        return Some(message);
                    }
                    tracing::warn!("ignoring malformed collaboration bus message");
                }
                // Missed messages are healed by the room's database catch-up.
                Err(RecvError::Lagged(_)) => {}
                Err(RecvError::Closed) => return None,
            }
        }
    }
}

#[async_trait]
impl UpdateBus for RedisBus {
    async fn publish(&self, document: DocumentId, message: BusMessage) {
        if let Err(err) = self.bus.publish(&channel(document), encode(&message)).await {
            tracing::warn!(%document, error = %err, "collaboration bus publish failed");
        }
    }

    async fn subscribe(&self, document: DocumentId) -> Result<Box<dyn BusStream>, StoreError> {
        let sub = self
            .bus
            .subscribe(&channel(document))
            .await
            .map_err(|err| {
                tracing::error!(%document, error = %err, "collaboration bus subscribe failed");
                StoreError
            })?;
        Ok(Box::new(RedisStream(sub)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages_roundtrip() {
        let update = BusMessage {
            instance: Uuid::from_u128(7),
            kind: BusKind::Update { seq: 42 },
            bytes: Arc::from(&b"abc"[..]),
        };
        let back = decode(&encode(&update)).unwrap();
        assert_eq!(
            (back.instance, back.kind, &*back.bytes),
            (update.instance, update.kind, &b"abc"[..])
        );
        let awareness = BusMessage {
            kind: BusKind::Awareness,
            ..update
        };
        assert_eq!(
            decode(&encode(&awareness)).unwrap().kind,
            BusKind::Awareness
        );
    }

    #[test]
    fn malformed_messages_are_rejected() {
        assert!(decode(&[1, 2, 3]).is_none());
        let mut raw = encode(&BusMessage {
            instance: Uuid::nil(),
            kind: BusKind::Awareness,
            bytes: Arc::from(&b""[..]),
        });
        raw[16] = 9;
        assert!(decode(&raw).is_none());
    }
}
