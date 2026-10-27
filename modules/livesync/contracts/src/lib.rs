//! Public surface of the `livesync` module: the wire protocol of its live channel.
//!
//! A client opens a WebSocket for one resource (a document, spreadsheet, presentation or PDF) and
//! receives [`ServerMessage`]s as JSON text frames: who else has the resource open, and when
//! somebody saved a new version of it. The protocol says nothing about the editor behind the
//! resource, so any editor (a vendored one or one of ours) can use the channel.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use identity_contracts::UserId;
use serde::{Deserialize, Serialize};
use shared_kernel::Uuid;
use time::OffsetDateTime;
use workspace_contracts::Role;

/// One open connection to a resource.
///
/// A user with two tabs open is two participants.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Participant {
    /// Identifies this connection; unique per tab.
    pub session: Uuid,
    /// The signed-in user.
    pub user_id: UserId,
    /// Name to show for the user.
    pub display_name: String,
    /// What the user may do with the resource.
    pub role: Role,
    /// The user is changing the file right now (unsaved edits).
    #[serde(default)]
    pub editing: bool,
}

/// What a client tells the server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    /// Whether this connection is changing the file right now.
    Activity {
        /// `true` while there are unsaved edits.
        editing: bool,
    },
}

/// What the server tells a connected client.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    /// First message: this connection's id and everybody else who is connected.
    Welcome {
        /// This connection.
        session: Uuid,
        /// The other participants.
        participants: Vec<Participant>,
    },
    /// Somebody connected.
    Joined {
        /// The newcomer.
        participant: Participant,
    },
    /// Somebody disconnected.
    Left {
        /// The connection that closed.
        session: Uuid,
    },
    /// A participant started or stopped changing the file.
    Activity {
        /// The connection.
        session: Uuid,
        /// Whether it has unsaved edits.
        editing: bool,
    },
    /// A new version of the file was saved (any user, any tab, including this one).
    Saved {
        /// Package version after the save.
        version: i64,
        /// Who saved it; `None` when unknown.
        by: Option<UserId>,
        /// When it was saved (UTC).
        #[serde(with = "time::serde::rfc3339")]
        at: OffsetDateTime,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages_carry_a_type_tag_in_snake_case() {
        let message = ServerMessage::Left {
            session: Uuid::nil(),
        };
        let json = serde_json::to_value(&message).unwrap_or_default();
        assert_eq!(json["type"], "left");
        assert_eq!(
            serde_json::from_value::<ServerMessage>(json).ok(),
            Some(message)
        );
    }

    #[test]
    fn clients_announce_activity() {
        let json = serde_json::json!({"type": "activity", "editing": true});
        assert_eq!(
            serde_json::from_value::<ClientMessage>(json).ok(),
            Some(ClientMessage::Activity { editing: true })
        );
    }

    #[test]
    fn participants_without_the_editing_flag_are_not_editing() {
        let old = serde_json::json!({
            "session": Uuid::nil(),
            "user_id": Uuid::nil(),
            "display_name": "A",
            "role": "viewer"
        });
        let participant = serde_json::from_value::<Participant>(old).ok();
        assert_eq!(participant.map(|p| p.editing), Some(false));
    }
}
