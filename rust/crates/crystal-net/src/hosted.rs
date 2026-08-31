use std::collections::VecDeque;

use crystal_core::multiplayer::{
    LinkHello, LinkMessage, LinkSessionIdentity, SessionSaveCheckpointFrame,
};
use ewebsock::{Options, WsEvent, WsMessage, WsReceiver, WsSender};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::{
    DEFAULT_MAX_FRAME_BYTES, EndpointError, LinkEndpoint, LinkEndpointEvent, LinkFrameCodec,
    LinkTransport, TransportError, validate_local_session_bootstrap,
};

pub const PROTOCOL_VERSION: u16 = 1;
pub const MAX_RELAY_BYTES: usize = 64 * 1024;
const SESSION_ID_BYTES: usize = 16;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModpackIdentity {
    pub id: String,
    pub content_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldIdentity {
    pub world_id: String,
    pub modpack: ModpackIdentity,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClientIdentity {
    pub user_id: String,
    pub display_name: String,
    pub world: WorldIdentity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchMode {
    Battle,
    Trade,
    TimeCapsule,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchOutcome {
    Local,
    Remote,
    Draw,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum ClientMessage {
    Hello {
        protocol_version: u16,
        identity: ClientIdentity,
    },
    Presence {
        map: String,
        tile_x: i32,
        tile_y: i32,
        direction: String,
    },
    QueueJoin {
        mode: MatchMode,
        rating: i32,
        rating_range: u32,
    },
    QueueLeave,
    Relay {
        session_id: Uuid,
        payload: serde_json::Value,
    },
    InteractionRequest {
        target_user_id: String,
        kind: MatchMode,
    },
    InteractionResponse {
        request_id: Uuid,
        target_user_id: String,
        accepted: bool,
    },
    Result {
        session_id: Uuid,
        outcome: MatchOutcome,
    },
    Ping {
        nonce: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    Welcome {
        protocol_version: u16,
        connection_id: Uuid,
    },
    Presence {
        user_id: String,
        display_name: String,
        map: String,
        tile_x: i32,
        tile_y: i32,
        direction: String,
    },
    PresenceLeft {
        user_id: String,
    },
    QueueJoined {
        mode: MatchMode,
    },
    QueueLeft,
    MatchFound {
        session_id: Uuid,
        mode: MatchMode,
        opponent_user_id: String,
        opponent_display_name: String,
        is_host: bool,
    },
    Relay {
        session_id: Uuid,
        from_user_id: String,
        payload: serde_json::Value,
    },
    InteractionRequest {
        request_id: Uuid,
        from_user_id: String,
        from_display_name: String,
        kind: MatchMode,
    },
    InteractionResponse {
        request_id: Uuid,
        from_user_id: String,
        accepted: bool,
    },
    ResultPending {
        session_id: Uuid,
    },
    ResultSettled {
        session_id: Uuid,
        winner_user_id: Option<String>,
        ranked: bool,
    },
    Pong {
        nonce: u64,
    },
    Error {
        code: String,
        message: String,
    },
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum HostedConnectionError {
    #[error("invalid hosted server URL or token: {0}")]
    InvalidUrl(String),
    #[error("open hosted WebSocket: {0}")]
    Open(String),
    #[error("hosted WebSocket closed")]
    Closed,
    #[error("hosted WebSocket error: {0}")]
    Socket(String),
    #[error("decode hosted server message: {0}")]
    Decode(String),
    #[error("hosted server rejected the request ({code}): {message}")]
    Server { code: String, message: String },
}

pub struct HostedConnection {
    sender: WsSender,
    receiver: WsReceiver,
    identity: ClientIdentity,
    opened: bool,
    pending: VecDeque<ClientMessage>,
}

impl HostedConnection {
    pub fn connect(
        server_url: impl Into<String>,
        token: Option<&str>,
        identity: ClientIdentity,
    ) -> Result<Self, HostedConnectionError> {
        let url = authenticated_url(server_url.into(), token)?;
        let options = Options {
            max_incoming_frame_size: MAX_RELAY_BYTES + 4096,
            ..Options::default()
        };
        let (sender, receiver) =
            ewebsock::connect(url, options).map_err(HostedConnectionError::Open)?;
        Ok(Self {
            sender,
            receiver,
            identity,
            opened: false,
            pending: VecDeque::new(),
        })
    }

    pub fn send(&mut self, message: ClientMessage) -> Result<(), HostedConnectionError> {
        if !self.opened {
            self.pending.push_back(message);
            return Ok(());
        }
        self.send_now(message)
    }

    pub fn join_queue(
        &mut self,
        mode: MatchMode,
        rating: i32,
        rating_range: u32,
    ) -> Result<(), HostedConnectionError> {
        self.send(ClientMessage::QueueJoin {
            mode,
            rating,
            rating_range,
        })
    }

    pub fn update_presence(
        &mut self,
        map: impl Into<String>,
        tile_x: i32,
        tile_y: i32,
        direction: impl Into<String>,
    ) -> Result<(), HostedConnectionError> {
        self.send(ClientMessage::Presence {
            map: map.into(),
            tile_x,
            tile_y,
            direction: direction.into(),
        })
    }

    pub fn poll(&mut self) -> Result<Vec<ServerMessage>, HostedConnectionError> {
        let mut messages = Vec::new();
        while let Some(event) = self.receiver.try_recv() {
            match event {
                WsEvent::Opened => {
                    self.opened = true;
                    self.send_now(ClientMessage::Hello {
                        protocol_version: PROTOCOL_VERSION,
                        identity: self.identity.clone(),
                    })?;
                    while let Some(message) = self.pending.pop_front() {
                        self.send_now(message)?;
                    }
                }
                WsEvent::Message(WsMessage::Text(text)) => {
                    let message = serde_json::from_str::<ServerMessage>(&text)
                        .map_err(|error| HostedConnectionError::Decode(error.to_string()))?;
                    if let ServerMessage::Error { code, message } = message {
                        return Err(HostedConnectionError::Server { code, message });
                    }
                    messages.push(message);
                }
                WsEvent::Message(WsMessage::Ping(payload)) => {
                    self.sender.send(WsMessage::Pong(payload));
                }
                WsEvent::Message(WsMessage::Binary(_)) => {
                    return Err(HostedConnectionError::Decode(
                        "received a link frame before a match was established".into(),
                    ));
                }
                WsEvent::Message(_) => {}
                WsEvent::Error(message) => return Err(HostedConnectionError::Socket(message)),
                WsEvent::Closed => return Err(HostedConnectionError::Closed),
            }
        }
        Ok(messages)
    }

    fn send_now(&mut self, message: ClientMessage) -> Result<(), HostedConnectionError> {
        let text = serde_json::to_string(&message)
            .map_err(|error| HostedConnectionError::Decode(error.to_string()))?;
        self.sender.send(WsMessage::Text(text));
        Ok(())
    }
}

pub struct HostedLinkTransport {
    sender: WsSender,
    receiver: WsReceiver,
    session_id: Uuid,
    codec: LinkFrameCodec,
    server_messages: VecDeque<ServerMessage>,
    connected: bool,
}

impl HostedLinkTransport {
    fn from_connection(
        connection: HostedConnection,
        session_id: Uuid,
        session: LinkSessionIdentity,
    ) -> Result<Self, TransportError> {
        Ok(Self {
            sender: connection.sender,
            receiver: connection.receiver,
            session_id,
            codec: LinkFrameCodec::for_session(DEFAULT_MAX_FRAME_BYTES, session)?,
            server_messages: VecDeque::new(),
            connected: true,
        })
    }

    fn send_protocol(&mut self, message: ClientMessage) -> Result<(), TransportError> {
        if !self.connected {
            return Err(TransportError::NotConnected);
        }
        let text = serde_json::to_string(&message)
            .map_err(|error| hosted_transport_error(error.to_string()))?;
        self.sender.send(WsMessage::Text(text));
        Ok(())
    }

    fn drain_server_messages(&mut self) -> Vec<ServerMessage> {
        self.server_messages.drain(..).collect()
    }

    pub fn disconnect(&mut self) {
        self.connected = false;
        self.sender.close();
    }
}

impl LinkTransport for HostedLinkTransport {
    fn send(&mut self, message: LinkMessage) -> Result<(), TransportError> {
        if !self.connected {
            return Err(TransportError::NotConnected);
        }
        let frame = self.codec.encode(&message)?;
        let mut envelope = Vec::with_capacity(SESSION_ID_BYTES + frame.len());
        envelope.extend_from_slice(self.session_id.as_bytes());
        envelope.extend_from_slice(&frame);
        self.sender.send(WsMessage::Binary(envelope));
        Ok(())
    }

    fn poll(&mut self) -> Result<Vec<LinkMessage>, TransportError> {
        if !self.connected {
            return Err(TransportError::NotConnected);
        }
        let mut messages = Vec::new();
        while let Some(event) = self.receiver.try_recv() {
            match event {
                WsEvent::Message(WsMessage::Binary(envelope)) => {
                    if envelope.len() <= SESSION_ID_BYTES {
                        return Err(hosted_transport_error("binary relay frame is truncated"));
                    }
                    let relayed_session = Uuid::from_slice(&envelope[..SESSION_ID_BYTES])
                        .map_err(|error| hosted_transport_error(error.to_string()))?;
                    if relayed_session != self.session_id {
                        return Err(hosted_transport_error(
                            "binary relay session does not match",
                        ));
                    }
                    messages.push(self.codec.decode(&envelope[SESSION_ID_BYTES..])?);
                }
                WsEvent::Message(WsMessage::Text(text)) => {
                    let message = serde_json::from_str::<ServerMessage>(&text)
                        .map_err(|error| hosted_transport_error(error.to_string()))?;
                    match message {
                        ServerMessage::Error { code, message } => {
                            return Err(hosted_transport_error(format!("{code}: {message}")));
                        }
                        message => self.server_messages.push_back(message),
                    }
                }
                WsEvent::Message(WsMessage::Ping(payload)) => {
                    self.sender.send(WsMessage::Pong(payload));
                }
                WsEvent::Error(message) => return Err(hosted_transport_error(message)),
                WsEvent::Closed => {
                    self.connected = false;
                    return Err(TransportError::NotConnected);
                }
                _ => {}
            }
        }
        Ok(messages)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostedLinkSessionEvent {
    Endpoint(LinkEndpointEvent),
    GameplayReady,
}

pub struct HostedLinkSession {
    endpoint: LinkEndpoint<HostedLinkTransport>,
    local_checkpoint: SessionSaveCheckpointFrame,
    checkpoint_sent: bool,
    gameplay_ready_emitted: bool,
}

impl HostedLinkSession {
    pub fn new(
        connection: HostedConnection,
        server_session_id: Uuid,
        local_hello: LinkHello,
        local_checkpoint: SessionSaveCheckpointFrame,
    ) -> Result<Self, EndpointError> {
        validate_local_session_bootstrap(&local_hello, &local_checkpoint)?;
        let transport = HostedLinkTransport::from_connection(
            connection,
            server_session_id,
            local_hello.session().clone(),
        )?;
        let mut endpoint = LinkEndpoint::new(transport, local_hello)?;
        endpoint.send_hello()?;
        Ok(Self {
            endpoint,
            local_checkpoint,
            checkpoint_sent: false,
            gameplay_ready_emitted: false,
        })
    }

    pub fn is_ready_for_gameplay(&self) -> bool {
        self.endpoint.is_ready_for_gameplay()
    }

    pub fn send(&mut self, message: LinkMessage) -> Result<(), EndpointError> {
        self.endpoint.send(message)
    }

    pub fn update_presence(
        &mut self,
        map: impl Into<String>,
        tile_x: i32,
        tile_y: i32,
        direction: impl Into<String>,
    ) -> Result<(), TransportError> {
        self.endpoint
            .transport_mut()
            .send_protocol(ClientMessage::Presence {
                map: map.into(),
                tile_x,
                tile_y,
                direction: direction.into(),
            })
    }

    pub fn drain_server_messages(&mut self) -> Vec<ServerMessage> {
        self.endpoint.transport_mut().drain_server_messages()
    }

    pub fn report_result(&mut self, outcome: MatchOutcome) -> Result<(), TransportError> {
        let session_id = self.endpoint.transport().session_id;
        self.endpoint
            .transport_mut()
            .send_protocol(ClientMessage::Result {
                session_id,
                outcome,
            })
    }

    pub fn poll(&mut self) -> Result<Vec<HostedLinkSessionEvent>, EndpointError> {
        let mut events = self
            .endpoint
            .poll()?
            .into_iter()
            .map(HostedLinkSessionEvent::Endpoint)
            .collect::<Vec<_>>();
        if self.endpoint.is_ready() && !self.checkpoint_sent {
            self.endpoint.send(LinkMessage::SessionSaveCheckpoint(
                self.local_checkpoint.clone(),
            ))?;
            self.checkpoint_sent = true;
        }
        if self.endpoint.is_ready_for_gameplay() && !self.gameplay_ready_emitted {
            self.gameplay_ready_emitted = true;
            events.push(HostedLinkSessionEvent::GameplayReady);
        }
        Ok(events)
    }

    pub fn disconnect(&mut self) {
        self.endpoint.transport_mut().disconnect();
    }
}

fn authenticated_url(
    mut url: String,
    token: Option<&str>,
) -> Result<String, HostedConnectionError> {
    if !url.starts_with("ws://") && !url.starts_with("wss://") {
        return Err(HostedConnectionError::InvalidUrl(
            "URL must start with ws:// or wss://".into(),
        ));
    }
    if let Some(token) = token.filter(|value| !value.is_empty()) {
        if !token
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~'))
        {
            return Err(HostedConnectionError::InvalidUrl(
                "token contains characters that require URL encoding".into(),
            ));
        }
        url.push(if url.contains('?') { '&' } else { '?' });
        url.push_str("token=");
        url.push_str(token);
    }
    Ok(url)
}

fn hosted_transport_error(message: impl Into<String>) -> TransportError {
    TransportError::Hosted {
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn appends_safe_browser_authentication_token() {
        assert_eq!(
            authenticated_url("ws://localhost/v1/ws".into(), Some("abc-123")).unwrap(),
            "ws://localhost/v1/ws?token=abc-123"
        );
        assert!(authenticated_url("http://localhost".into(), None).is_err());
        assert!(authenticated_url("wss://example/ws".into(), Some("bad token")).is_err());
    }

    #[test]
    fn protocol_json_is_stable() {
        let message = ClientMessage::QueueJoin {
            mode: MatchMode::Trade,
            rating: 1000,
            rating_range: 50,
        };
        assert_eq!(
            serde_json::to_value(message).unwrap(),
            serde_json::json!({
                "type": "queue_join",
                "mode": "trade",
                "rating": 1000,
                "rating_range": 50
            })
        );
    }
}
