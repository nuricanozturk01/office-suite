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
use documents_contracts::{DocumentId, OwnerId};
use platform::http::{AuthUser, ProblemDetails};
use shared_kernel::Uuid;
use tokio::sync::broadcast::error::RecvError;

use crate::application::{CollaborationService, Frame, JoinError, Session, SessionError};

/// Largest accepted frame (a single Yjs update or awareness message).
const MAX_MESSAGE_BYTES: usize = 1024 * 1024;

const CLOSE_INVALID_PAYLOAD: u16 = 1007;
const CLOSE_POLICY: u16 = 1008;
const CLOSE_INTERNAL: u16 = 1011;
const CLOSE_TRY_AGAIN: u16 = 1013;

async fn connect(
    State(service): State<Arc<CollaborationService>>,
    user: AuthUser,
    Path(document): Path<Uuid>,
    ws: WebSocketUpgrade,
) -> Response {
    let session = match service
        .join(OwnerId::from_uuid(user.0), DocumentId::from_uuid(document))
        .await
    {
        Ok(session) => session,
        Err(JoinError::NotFound) => {
            return ProblemDetails::new(StatusCode::NOT_FOUND, "not_found", "Not found")
                .into_response();
        }
        Err(JoinError::Internal) => {
            return ProblemDetails::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal",
                "Internal error",
            )
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

async fn run(mut socket: WebSocket, session: Session) {
    // Subscribe before the first frame so nothing between hello and the loop is missed.
    let mut updates = session.subscribe();
    if socket
        .send(Message::Binary(session.hello().await.into()))
        .await
        .is_err()
    {
        return;
    }
    let mut recheck = tokio::time::interval(session.recheck_interval());
    recheck.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    recheck.tick().await; // the first tick completes immediately
    loop {
        tokio::select! {
            _ = recheck.tick() => {
                if session.access_lost().await {
                    return close(&mut socket, CLOSE_POLICY, "access changed").await;
                }
            }
            incoming = socket.recv() => match incoming {
                Some(Ok(Message::Binary(bytes))) => match session.handle(&bytes).await {
                    Ok(replies) => {
                        for reply in replies {
                            if socket.send(Message::Binary(reply.into())).await.is_err() {
                                return;
                            }
                        }
                    }
                    Err(SessionError::Forbidden) => {
                        return close(&mut socket, CLOSE_POLICY, "read-only").await;
                    }
                    Err(SessionError::Malformed) => {
                        return close(&mut socket, CLOSE_INVALID_PAYLOAD, "malformed").await;
                    }
                    Err(SessionError::Internal) => {
                        return close(&mut socket, CLOSE_INTERNAL, "internal error").await;
                    }
                },
                Some(Ok(Message::Close(_)) | Err(_)) | None => return,
                Some(Ok(_)) => {}
            },
            frame = updates.recv() => match frame {
                Ok(Frame::Data { origin, bytes }) => {
                    if origin != session.id
                        && socket.send(Message::Binary(bytes.to_vec().into())).await.is_err()
                    {
                        return;
                    }
                }
                Ok(Frame::AccessChanged { user }) => {
                    if user == session.user() {
                        return close(&mut socket, CLOSE_POLICY, "access changed").await;
                    }
                }
                Ok(Frame::Closed) | Err(RecvError::Closed) => {
                    return close(&mut socket, CLOSE_TRY_AGAIN, "resync").await;
                }
                Err(RecvError::Lagged(_)) => {
                    return close(&mut socket, CLOSE_TRY_AGAIN, "too slow").await;
                }
            },
        }
    }
}

pub(crate) fn router(service: Arc<CollaborationService>) -> Router {
    Router::new()
        .route("/{document_id}", get(connect))
        .with_state(service)
}
