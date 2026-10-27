use std::sync::Arc;

use axum::{
    Router,
    extract::{
        Path, State,
        ws::{CloseFrame, Message, WebSocket, WebSocketUpgrade},
    },
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use identity_contracts::UserId;
use livesync_contracts::{ClientMessage, ServerMessage};
use platform::http::{AuthUser, ProblemDetails};
use shared_kernel::Uuid;
use tokio::sync::broadcast::error::RecvError;
use workspace_contracts::ResourceId;

use crate::application::{Event, JoinError, LiveSyncService, Session};

/// Clients send tiny activity messages; anything bigger is refused.
const MAX_MESSAGE_BYTES: usize = 4 * 1024;

const CLOSE_POLICY: u16 = 1008;
const CLOSE_INTERNAL: u16 = 1011;
const CLOSE_TRY_AGAIN: u16 = 1013;

async fn connect(
    State(service): State<Arc<LiveSyncService>>,
    user: AuthUser,
    Path(resource): Path<Uuid>,
    ws: WebSocketUpgrade,
) -> Response {
    let session = match service
        .join(UserId::from_uuid(user.0), ResourceId::from_uuid(resource))
        .await
    {
        Ok(session) => session,
        Err(JoinError::NotFound) => {
            return ProblemDetails::new(StatusCode::NOT_FOUND, "not_found", "Not found")
                .into_response();
        }
    };
    ws.protocols(["bearer"])
        .max_message_size(MAX_MESSAGE_BYTES)
        .on_upgrade(move |socket| run(socket, session))
}

async fn close(socket: &mut WebSocket, code: u16, reason: &'static str) {
    let _ = socket
        .send(Message::Close(Some(CloseFrame {
            code,
            reason: reason.into(),
        })))
        .await;
}

async fn send(socket: &mut WebSocket, message: &ServerMessage) -> bool {
    match serde_json::to_string(message) {
        Ok(text) => socket.send(Message::Text(text.into())).await.is_ok(),
        Err(_) => false,
    }
}

async fn run(mut socket: WebSocket, mut session: Session) {
    serve(&mut socket, &mut session).await;
    session.leave().await;
}

async fn serve(socket: &mut WebSocket, session: &mut Session) {
    if !send(socket, &session.welcome()).await {
        return;
    }
    let mut recheck = tokio::time::interval(session.recheck_interval());
    recheck.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    recheck.tick().await; // the first tick completes immediately
    loop {
        tokio::select! {
            _ = recheck.tick() => {
                if session.access_lost().await {
                    return close(socket, CLOSE_POLICY, "access changed").await;
                }
            }
            incoming = socket.recv() => match incoming {
                Some(Ok(Message::Close(_)) | Err(_)) | None => return,
                Some(Ok(Message::Text(text))) => {
                    // Anything else a client sends is ignored.
                    if let Ok(ClientMessage::Activity { editing }) = serde_json::from_str(&text) {
                        session.set_editing(editing).await;
                    }
                }
                Some(Ok(_)) => {}
            },
            event = session.next_event() => match event {
                Ok(Event::Message(message)) => {
                    if !is_for(&message, session.id()) {
                        continue;
                    }
                    if !send(socket, &message).await {
                        return;
                    }
                }
                Ok(Event::AccessChanged { user }) => {
                    if user == session.user() && session.access_lost().await {
                        return close(socket, CLOSE_POLICY, "access changed").await;
                    }
                }
                Err(RecvError::Lagged(_)) => {
                    return close(socket, CLOSE_TRY_AGAIN, "too slow").await;
                }
                Err(RecvError::Closed) => {
                    return close(socket, CLOSE_INTERNAL, "room closed").await;
                }
            },
        }
    }
}

/// A connection does not hear about itself.
fn is_for(message: &ServerMessage, me: Uuid) -> bool {
    match message {
        ServerMessage::Joined { participant } => participant.session != me,
        ServerMessage::Left { session } | ServerMessage::Activity { session, .. } => *session != me,
        ServerMessage::Welcome { .. } | ServerMessage::Saved { .. } => true,
    }
}

pub(crate) fn router(service: Arc<LiveSyncService>) -> Router {
    Router::new()
        .route("/{resource_id}", get(connect))
        .with_state(service)
}
