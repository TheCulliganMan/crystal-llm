use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use crystal_core::multiplayer::{
    BattleActionFrame, BattleRngState, LinkByteFrame, LinkClockSyncFrame, LinkHandshakeError,
    LinkHello, LinkMessage, LinkSessionIdentity, MultiplayerInteractionRequest,
    MultiplayerInteractionResponse, OverworldPresence, PlayerId, PlayerInputFrame,
    StateChecksumFrame, TradeConfirmation, TradeOffer, fnv1a32_bytes, validate_link_hello,
};
use thiserror::Error;

const LINK_FRAME_MAGIC: &[u8; 8] = b"CRYSLINK";
pub const LINK_FRAME_VERSION: u16 = 2;
pub const DEFAULT_MAX_FRAME_BYTES: usize = 64 * 1024;
const VERSION_OFFSET: usize = LINK_FRAME_MAGIC.len();
const LENGTH_OFFSET: usize = VERSION_OFFSET + 2;
const PAYLOAD_HASH_OFFSET: usize = LENGTH_OFFSET + 4;
const HEADER_LEN: usize = PAYLOAD_HASH_OFFSET + 4;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum TransportError {
    #[error("transport is not connected")]
    NotConnected,
    #[error("message exceeds transport frame size")]
    MessageTooLarge,
    #[error("link frame max size {max_frame_bytes} is smaller than the required header")]
    FrameLimitTooSmall { max_frame_bytes: usize },
    #[error("link frame max size {max_frame_bytes} exceeds the binary frame payload length field")]
    FrameLimitTooLarge { max_frame_bytes: usize },
    #[error("link frame is shorter than the required header")]
    FrameTooShort,
    #[error("link frame magic is invalid")]
    InvalidMagic,
    #[error("link frame version {actual} does not match expected {expected}")]
    VersionMismatch { expected: u16, actual: u16 },
    #[error("link frame payload length {declared} does not match actual {actual}")]
    LengthMismatch { declared: usize, actual: usize },
    #[error("link frame payload hash {actual:#010x} does not match declared {expected:#010x}")]
    PayloadHashMismatch { expected: u32, actual: u32 },
    #[error("link frame payload must be non-empty")]
    EmptyPayload,
    #[error("link frame payload is not a valid link message: {message}")]
    InvalidPayload { message: String },
    #[error("link frame message violates protocol invariants: {message}")]
    InvalidMessage { message: String },
    #[error("link frame requires an exact session identity")]
    MissingSessionBinding,
    #[error("link frame session violates protocol invariants: {message}")]
    InvalidSession { message: String },
    #[error("link frame session does not match codec session: {message}")]
    SessionMismatch { message: String },
}

pub trait LinkTransport {
    fn send(&mut self, message: LinkMessage) -> Result<(), TransportError>;
    fn poll(&mut self) -> Result<Vec<LinkMessage>, TransportError>;
}

type SharedFrameQueue = Rc<RefCell<VecDeque<Vec<u8>>>>;

#[derive(Debug, Clone)]
pub struct MemoryLinkTransport {
    codec: LinkFrameCodec,
    inbound: SharedFrameQueue,
    outbound: SharedFrameQueue,
    connected: bool,
}

impl MemoryLinkTransport {
    pub fn pair() -> (Self, Self) {
        Self::pair_with_codec(LinkFrameCodec::default())
    }

    pub fn pair_for_session(session: LinkSessionIdentity) -> Result<(Self, Self), TransportError> {
        Ok(Self::pair_with_codec(LinkFrameCodec::for_session(
            DEFAULT_MAX_FRAME_BYTES,
            session,
        )?))
    }

    pub fn pair_with_codec(codec: LinkFrameCodec) -> (Self, Self) {
        let a_to_b = Rc::new(RefCell::new(VecDeque::new()));
        let b_to_a = Rc::new(RefCell::new(VecDeque::new()));
        (
            Self {
                codec: codec.clone(),
                inbound: Rc::clone(&b_to_a),
                outbound: Rc::clone(&a_to_b),
                connected: true,
            },
            Self {
                codec,
                inbound: a_to_b,
                outbound: b_to_a,
                connected: true,
            },
        )
    }

    pub fn disconnect(&mut self) {
        self.connected = false;
    }

    pub fn pending_inbound_frames(&self) -> usize {
        self.inbound.borrow().len()
    }

    #[cfg(test)]
    pub fn push_inbound_frame_for_tests(&mut self, frame: Vec<u8>) {
        self.inbound.borrow_mut().push_back(frame);
    }
}

impl LinkTransport for MemoryLinkTransport {
    fn send(&mut self, message: LinkMessage) -> Result<(), TransportError> {
        if !self.connected {
            return Err(TransportError::NotConnected);
        }
        let frame = self.codec.encode(&message)?;
        self.outbound.borrow_mut().push_back(frame);
        Ok(())
    }

    fn poll(&mut self) -> Result<Vec<LinkMessage>, TransportError> {
        if !self.connected {
            return Err(TransportError::NotConnected);
        }
        let frames = self.inbound.borrow_mut().drain(..).collect::<Vec<_>>();
        frames
            .into_iter()
            .map(|frame| self.codec.decode(&frame))
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
enum WireLinkMessage {
    Hello(LinkHello),
    RngInit { state: BattleRngState },
    BattleAction(BattleActionFrame),
    TradeOffer(TradeOffer),
    TradeConfirmation(TradeConfirmation),
    LinkByte(LinkByteFrame),
    LinkClockSync(LinkClockSyncFrame),
    Input(PlayerInputFrame),
    StateHash(StateChecksumFrame),
    Presence(OverworldPresence),
    InteractionRequest(MultiplayerInteractionRequest),
    InteractionResponse(MultiplayerInteractionResponse),
    Disconnect { player_id: PlayerId, reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct WireLinkFrame {
    session: LinkSessionIdentity,
    message: WireLinkMessage,
}

impl From<&LinkMessage> for WireLinkMessage {
    fn from(message: &LinkMessage) -> Self {
        match message {
            LinkMessage::Hello(hello) => Self::Hello(hello.clone()),
            LinkMessage::RngInit { state } => Self::RngInit { state: *state },
            LinkMessage::BattleAction(action) => Self::BattleAction(action.clone()),
            LinkMessage::TradeOffer(offer) => Self::TradeOffer(offer.clone()),
            LinkMessage::TradeConfirmation(confirmation) => {
                Self::TradeConfirmation(confirmation.clone())
            }
            LinkMessage::LinkByte(frame) => Self::LinkByte(frame.clone()),
            LinkMessage::LinkClockSync(frame) => Self::LinkClockSync(frame.clone()),
            LinkMessage::Input(input) => Self::Input(input.clone()),
            LinkMessage::StateHash(checksum) => Self::StateHash(checksum.clone()),
            LinkMessage::Presence(presence) => Self::Presence(presence.clone()),
            LinkMessage::InteractionRequest(request) => Self::InteractionRequest(request.clone()),
            LinkMessage::InteractionResponse(response) => {
                Self::InteractionResponse(response.clone())
            }
            LinkMessage::Disconnect { player_id, reason } => Self::Disconnect {
                player_id: *player_id,
                reason: reason.clone(),
            },
        }
    }
}

impl From<WireLinkMessage> for LinkMessage {
    fn from(message: WireLinkMessage) -> Self {
        match message {
            WireLinkMessage::Hello(hello) => Self::Hello(hello),
            WireLinkMessage::RngInit { state } => Self::RngInit { state },
            WireLinkMessage::BattleAction(action) => Self::BattleAction(action),
            WireLinkMessage::TradeOffer(offer) => Self::TradeOffer(offer),
            WireLinkMessage::TradeConfirmation(confirmation) => {
                Self::TradeConfirmation(confirmation)
            }
            WireLinkMessage::LinkByte(frame) => Self::LinkByte(frame),
            WireLinkMessage::LinkClockSync(frame) => Self::LinkClockSync(frame),
            WireLinkMessage::Input(input) => Self::Input(input),
            WireLinkMessage::StateHash(checksum) => Self::StateHash(checksum),
            WireLinkMessage::Presence(presence) => Self::Presence(presence),
            WireLinkMessage::InteractionRequest(request) => Self::InteractionRequest(request),
            WireLinkMessage::InteractionResponse(response) => Self::InteractionResponse(response),
            WireLinkMessage::Disconnect { player_id, reason } => {
                Self::Disconnect { player_id, reason }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkFrameCodec {
    max_frame_bytes: usize,
    session: Option<LinkSessionIdentity>,
}

impl Default for LinkFrameCodec {
    fn default() -> Self {
        Self::new(DEFAULT_MAX_FRAME_BYTES).expect("default frame limit fits link header")
    }
}

impl LinkFrameCodec {
    pub fn new(max_frame_bytes: usize) -> Result<Self, TransportError> {
        validate_frame_limit(max_frame_bytes)?;
        Ok(Self {
            max_frame_bytes,
            session: None,
        })
    }

    pub fn for_session(
        max_frame_bytes: usize,
        session: LinkSessionIdentity,
    ) -> Result<Self, TransportError> {
        validate_frame_limit(max_frame_bytes)?;
        session
            .validate()
            .map_err(|error| TransportError::InvalidSession {
                message: error.to_string(),
            })?;
        Ok(Self {
            max_frame_bytes,
            session: Some(session),
        })
    }

    pub const fn max_frame_bytes(&self) -> usize {
        self.max_frame_bytes
    }

    pub fn encode(&self, message: &LinkMessage) -> Result<Vec<u8>, TransportError> {
        let session = message_session(message, self.session.as_ref())?;
        validate_link_message(message)?;
        validate_frame_session(&session, message)?;
        let wire_frame = WireLinkFrame {
            session,
            message: WireLinkMessage::from(message),
        };
        let payload = bincode::serde::encode_to_vec(&wire_frame, link_frame_binary_config())
            .map_err(|error| TransportError::InvalidPayload {
                message: error.to_string(),
            })?;
        if payload.len() > u32::MAX as usize {
            return Err(TransportError::MessageTooLarge);
        }
        let frame_len = HEADER_LEN + payload.len();
        if frame_len > self.max_frame_bytes {
            return Err(TransportError::MessageTooLarge);
        }
        let mut frame = Vec::with_capacity(frame_len);
        frame.extend_from_slice(LINK_FRAME_MAGIC);
        frame.extend_from_slice(&LINK_FRAME_VERSION.to_be_bytes());
        frame.extend_from_slice(&(payload.len() as u32).to_be_bytes());
        frame.extend_from_slice(&fnv1a32_bytes(&payload).to_be_bytes());
        frame.extend_from_slice(&payload);
        Ok(frame)
    }

    pub fn decode(&self, frame: &[u8]) -> Result<LinkMessage, TransportError> {
        if frame.len() > self.max_frame_bytes {
            return Err(TransportError::MessageTooLarge);
        }
        if frame.len() < HEADER_LEN {
            return Err(TransportError::FrameTooShort);
        }
        if &frame[..LINK_FRAME_MAGIC.len()] != LINK_FRAME_MAGIC {
            return Err(TransportError::InvalidMagic);
        }

        let version = u16::from_be_bytes([frame[VERSION_OFFSET], frame[VERSION_OFFSET + 1]]);
        if version != LINK_FRAME_VERSION {
            return Err(TransportError::VersionMismatch {
                expected: LINK_FRAME_VERSION,
                actual: version,
            });
        }

        let declared = u32::from_be_bytes([
            frame[LENGTH_OFFSET],
            frame[LENGTH_OFFSET + 1],
            frame[LENGTH_OFFSET + 2],
            frame[LENGTH_OFFSET + 3],
        ]) as usize;
        let actual = frame.len() - HEADER_LEN;
        if declared != actual {
            return Err(TransportError::LengthMismatch { declared, actual });
        }
        if declared == 0 {
            return Err(TransportError::EmptyPayload);
        }
        let expected_hash = u32::from_be_bytes([
            frame[PAYLOAD_HASH_OFFSET],
            frame[PAYLOAD_HASH_OFFSET + 1],
            frame[PAYLOAD_HASH_OFFSET + 2],
            frame[PAYLOAD_HASH_OFFSET + 3],
        ]);
        let payload = &frame[HEADER_LEN..];
        let actual_hash = fnv1a32_bytes(payload);
        if actual_hash != expected_hash {
            return Err(TransportError::PayloadHashMismatch {
                expected: expected_hash,
                actual: actual_hash,
            });
        }

        let (wire_frame, bytes_read): (WireLinkFrame, usize) =
            bincode::serde::decode_from_slice(payload, link_frame_binary_config()).map_err(
                |error| TransportError::InvalidPayload {
                    message: error.to_string(),
                },
            )?;
        if bytes_read != declared {
            return Err(TransportError::LengthMismatch {
                declared,
                actual: bytes_read,
            });
        }
        wire_frame
            .session
            .validate()
            .map_err(|error| TransportError::InvalidSession {
                message: error.to_string(),
            })?;
        if let Some(expected_session) = &self.session {
            compare_sessions(expected_session, &wire_frame.session)
                .map_err(session_mismatch_error)?;
        }
        let message = wire_frame.message.into();
        if self.session.is_none() && !matches!(message, LinkMessage::Hello(_)) {
            return Err(TransportError::MissingSessionBinding);
        }
        validate_frame_session(&wire_frame.session, &message)?;
        validate_link_message(&message)?;
        Ok(message)
    }
}

fn validate_frame_limit(max_frame_bytes: usize) -> Result<(), TransportError> {
    if max_frame_bytes < HEADER_LEN {
        return Err(TransportError::FrameLimitTooSmall { max_frame_bytes });
    }
    if max_frame_bytes - HEADER_LEN > u32::MAX as usize {
        return Err(TransportError::FrameLimitTooLarge { max_frame_bytes });
    }
    Ok(())
}

fn link_frame_binary_config() -> impl bincode::config::Config {
    bincode::config::standard()
        .with_little_endian()
        .with_fixed_int_encoding()
}

fn message_session(
    message: &LinkMessage,
    codec_session: Option<&LinkSessionIdentity>,
) -> Result<LinkSessionIdentity, TransportError> {
    if let Some(session) = codec_session {
        session
            .validate()
            .map_err(|error| TransportError::InvalidSession {
                message: error.to_string(),
            })?;
        return Ok(session.clone());
    }
    match message {
        LinkMessage::Hello(hello) => Ok(hello.session().clone()),
        _ => Err(TransportError::MissingSessionBinding),
    }
}

fn compare_sessions(
    expected: &LinkSessionIdentity,
    actual: &LinkSessionIdentity,
) -> Result<(), LinkHandshakeError> {
    expected.validate()?;
    actual.validate()?;
    if actual.session_id() != expected.session_id() {
        return Err(LinkHandshakeError::SessionMismatch {
            expected: expected.session_id().to_string(),
            actual: actual.session_id().to_string(),
        });
    }
    if actual.modpack().id() != expected.modpack().id() {
        return Err(LinkHandshakeError::ModpackIdMismatch {
            expected: expected.modpack().id().to_string(),
            actual: actual.modpack().id().to_string(),
        });
    }
    if actual.modpack().hash() != expected.modpack().hash() {
        return Err(LinkHandshakeError::ModpackHashMismatch {
            expected: expected.modpack().hash().to_string(),
            actual: actual.modpack().hash().to_string(),
        });
    }
    if actual.protocol_version() != expected.protocol_version() {
        return Err(LinkHandshakeError::ProtocolVersionMismatch {
            expected: expected.protocol_version(),
            actual: actual.protocol_version(),
        });
    }
    Ok(())
}

fn validate_frame_session(
    session: &LinkSessionIdentity,
    message: &LinkMessage,
) -> Result<(), TransportError> {
    if let LinkMessage::Hello(hello) = message {
        validate_link_hello(session, hello).map_err(session_mismatch_error)?;
    }
    Ok(())
}

fn session_mismatch_error(error: LinkHandshakeError) -> TransportError {
    match error {
        LinkHandshakeError::SessionMismatch { .. }
        | LinkHandshakeError::ModpackIdMismatch { .. }
        | LinkHandshakeError::ModpackHashMismatch { .. }
        | LinkHandshakeError::ProtocolVersionMismatch { .. } => TransportError::SessionMismatch {
            message: error.to_string(),
        },
        _ => TransportError::InvalidMessage {
            message: error.to_string(),
        },
    }
}

fn validate_link_message(message: &LinkMessage) -> Result<(), TransportError> {
    message
        .validate()
        .map_err(|error| TransportError::InvalidMessage {
            message: error.to_string(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crystal_core::battle::turn::BattleAction;
    use crystal_core::models::{BaseStats, Dv, Pokemon, PokemonSpecies};
    use crystal_core::multiplayer::{
        BattleActionFrame, LINK_PREAMBLE_RESPONSE, LinkByteFrame, LinkClockSyncFrame, LinkHello,
        LinkSessionIdentity, MultiplayerInteractionKind, PlayerIdentity, PlayerInputFrame,
        StateChecksumFrame, TradeConfirmation, TradeOffer,
    };
    use crystal_core::save::SaveModpackIdentity;
    use crystal_core::timing::Frame;

    fn modpack() -> SaveModpackIdentity {
        SaveModpackIdentity::new("core-modular", "1234abcd").expect("modpack identity")
    }

    fn session() -> LinkSessionIdentity {
        LinkSessionIdentity::new("session-1", modpack()).expect("session")
    }

    fn player(id: PlayerId, display_name: &str) -> PlayerIdentity {
        PlayerIdentity::new(id, display_name).expect("player")
    }

    fn session_codec() -> LinkFrameCodec {
        LinkFrameCodec::for_session(DEFAULT_MAX_FRAME_BYTES, session()).expect("session codec")
    }

    fn hello_message() -> LinkMessage {
        LinkMessage::Hello(LinkHello::from_session(session(), player(7, "P7")).expect("hello"))
    }

    fn pokemon(id: &str, item: Option<&str>) -> Pokemon {
        let mut pokemon = Pokemon::new_for_tests(
            PokemonSpecies::new_for_tests(id, BaseStats::new(45, 49, 49, 45, 65, 65)),
            12,
            Dv::from_non_hp(1, 2, 3, 4),
        );
        pokemon.item = item.map(str::to_string);
        pokemon
    }

    fn frame_from_wire_message(message: WireLinkMessage) -> Vec<u8> {
        frame_from_wire_frame(WireLinkFrame {
            session: session(),
            message,
        })
    }

    fn frame_from_wire_frame(frame: WireLinkFrame) -> Vec<u8> {
        let payload = bincode::serde::encode_to_vec(&frame, link_frame_binary_config())
            .expect("encode wire payload");
        let mut frame = Vec::with_capacity(HEADER_LEN + payload.len());
        frame.extend_from_slice(LINK_FRAME_MAGIC);
        frame.extend_from_slice(&LINK_FRAME_VERSION.to_be_bytes());
        frame.extend_from_slice(&(payload.len() as u32).to_be_bytes());
        frame.extend_from_slice(&fnv1a32_bytes(&payload).to_be_bytes());
        frame.extend_from_slice(&payload);
        frame
    }

    fn legacy_v1_frame_from_wire_frame(frame: WireLinkFrame) -> Vec<u8> {
        let payload = bincode::serde::encode_to_vec(&frame, link_frame_binary_config())
            .expect("encode legacy wire payload");
        let mut frame = Vec::with_capacity(LINK_FRAME_MAGIC.len() + 2 + 4 + payload.len());
        frame.extend_from_slice(LINK_FRAME_MAGIC);
        frame.extend_from_slice(&1_u16.to_be_bytes());
        frame.extend_from_slice(&(payload.len() as u32).to_be_bytes());
        frame.extend_from_slice(&payload);
        frame
    }

    #[test]
    fn binary_link_frame_round_trips_hello_with_exact_modpack_identity() {
        let codec = LinkFrameCodec::default();
        let message = hello_message();
        let frame = codec.encode(&message).expect("encode");

        assert_eq!(&frame[..8], b"CRYSLINK");
        assert_eq!(
            u16::from_be_bytes([frame[VERSION_OFFSET], frame[VERSION_OFFSET + 1]]),
            LINK_FRAME_VERSION
        );
        assert_eq!(codec.decode(&frame).expect("decode"), message);
    }

    #[test]
    fn binary_link_frame_round_trips_input_message() {
        let codec = session_codec();
        let message =
            LinkMessage::Input(PlayerInputFrame::new(2, Frame(144), 0b1001_0000).expect("input"));
        let frame = codec.encode(&message).expect("encode");

        assert_eq!(codec.decode(&frame).expect("decode"), message);
    }

    #[test]
    fn binary_link_frame_round_trips_player_bound_state_hash_message() {
        let codec = session_codec();
        let message = LinkMessage::StateHash(StateChecksumFrame::new(2, Frame(144), 0xaabbccdd));
        let frame = codec.encode(&message).expect("encode");

        assert_eq!(codec.decode(&frame).expect("decode"), message);
    }

    #[test]
    fn binary_link_frame_round_trips_battle_action_message() {
        let codec = session_codec();
        let message = LinkMessage::BattleAction(
            BattleActionFrame::with_state_hash(
                2,
                12,
                BattleAction::Item {
                    item_id: "johto_plus:EMBER_ORB".to_string(),
                },
                "aaaabbbb",
            )
            .expect("action"),
        );
        let frame = codec.encode(&message).expect("encode");

        assert_eq!(codec.decode(&frame).expect("decode"), message);
    }

    #[test]
    fn binary_link_frame_round_trips_trade_messages() {
        let codec = session_codec();
        let offer = LinkMessage::TradeOffer(
            TradeOffer::new(
                "trade-1",
                1,
                0,
                pokemon("PIKACHU", Some("johto_plus:EMBER_ORB")),
            )
            .expect("offer"),
        );
        let offer_frame = codec.encode(&offer).expect("encode offer");
        assert_eq!(codec.decode(&offer_frame).expect("decode offer"), offer);

        let confirmation = LinkMessage::TradeConfirmation(
            TradeConfirmation::new("trade-1", 1, true).expect("confirmation"),
        );
        let confirmation_frame = codec.encode(&confirmation).expect("encode confirmation");
        assert_eq!(
            codec
                .decode(&confirmation_frame)
                .expect("decode confirmation"),
            confirmation
        );
    }

    #[test]
    fn binary_link_frame_round_trips_link_cable_messages() {
        let codec = session_codec();
        let byte = LinkMessage::LinkByte(
            LinkByteFrame::new(2, LINK_PREAMBLE_RESPONSE, 7).expect("byte frame"),
        );
        let byte_frame = codec.encode(&byte).expect("encode byte");
        assert_eq!(codec.decode(&byte_frame).expect("decode byte"), byte);

        let sync = LinkMessage::LinkClockSync(
            LinkClockSyncFrame::new(1, 100, 101, 102).expect("sync frame"),
        );
        let sync_frame = codec.encode(&sync).expect("encode sync");
        assert_eq!(codec.decode(&sync_frame).expect("decode sync"), sync);
    }

    #[test]
    fn binary_link_codec_rejects_zero_clock_link_byte_frames() {
        let codec = session_codec();
        let frame = frame_from_wire_message(WireLinkMessage::LinkByte(
            LinkByteFrame::new_unchecked_for_tests(2, LINK_PREAMBLE_RESPONSE, 0),
        ));

        assert_eq!(
            codec.decode(&frame),
            Err(TransportError::InvalidMessage {
                message: "link cable clock 0 must be nonzero".to_string(),
            })
        );
    }

    #[test]
    fn binary_link_codec_rejects_impossible_clock_sync_ordering() {
        let codec = session_codec();
        let frame = frame_from_wire_message(WireLinkMessage::LinkClockSync(
            LinkClockSyncFrame::new_unchecked_for_tests(1, 12, 11, 13),
        ));

        assert_eq!(
            codec.decode(&frame),
            Err(TransportError::InvalidMessage {
                message:
                    "link cable clock sync requires t0 <= t1 <= t2 but got t0=12, t1=11, t2=13"
                        .to_string(),
            })
        );
    }

    #[test]
    fn binary_link_codec_rejects_json_and_bad_magic() {
        let codec = LinkFrameCodec::default();
        assert_eq!(
            codec.decode(br#"{"type":"input","player_id":2}"#),
            Err(TransportError::InvalidMagic)
        );
    }

    #[test]
    fn binary_link_codec_rejects_protocol_version_drift() {
        let codec = LinkFrameCodec::default();
        let mut frame = codec.encode(&hello_message()).expect("encode");
        frame[VERSION_OFFSET + 1] = LINK_FRAME_VERSION as u8 + 1;

        assert_eq!(
            codec.decode(&frame),
            Err(TransportError::VersionMismatch {
                expected: LINK_FRAME_VERSION,
                actual: LINK_FRAME_VERSION + 1,
            })
        );
    }

    #[test]
    fn binary_link_codec_rejects_legacy_unchecksummed_v1_frames() {
        let codec = LinkFrameCodec::default();
        let frame = legacy_v1_frame_from_wire_frame(WireLinkFrame {
            session: session(),
            message: WireLinkMessage::Hello(
                LinkHello::from_session(session(), player(7, "P7")).expect("hello"),
            ),
        });

        assert_eq!(
            codec.decode(&frame),
            Err(TransportError::VersionMismatch {
                expected: LINK_FRAME_VERSION,
                actual: 1,
            })
        );
    }

    #[test]
    fn binary_link_codec_rejects_payload_hash_mismatch() {
        let codec = LinkFrameCodec::default();
        let mut frame = codec.encode(&hello_message()).expect("encode");
        let expected = u32::from_be_bytes([
            frame[PAYLOAD_HASH_OFFSET],
            frame[PAYLOAD_HASH_OFFSET + 1],
            frame[PAYLOAD_HASH_OFFSET + 2],
            frame[PAYLOAD_HASH_OFFSET + 3],
        ]);
        let last = frame.last_mut().expect("payload byte");
        *last ^= 0x01;
        let actual = fnv1a32_bytes(&frame[HEADER_LEN..]);

        assert_eq!(
            codec.decode(&frame),
            Err(TransportError::PayloadHashMismatch { expected, actual })
        );
    }

    #[test]
    fn binary_link_codec_rejects_messages_that_bypass_protocol_constructors() {
        let codec = LinkFrameCodec::default();
        let invalid_hello = LinkMessage::Hello(
            serde_json::from_value(serde_json::json!({
                "session": {
                    "protocol_version": LINK_FRAME_VERSION,
                    "session_id": "",
                    "modpack": modpack(),
                },
                "player": {
                    "id": 7,
                    "display_name": "P7",
                },
            }))
            .expect("deserialize invalid hello"),
        );
        assert!(matches!(
            codec.encode(&invalid_hello),
            Err(TransportError::InvalidMessage { .. })
        ));

        let invalid_wire_frame = frame_from_wire_message(WireLinkMessage::Hello(
            serde_json::from_value(serde_json::json!({
                "session": {
                    "protocol_version": LINK_FRAME_VERSION,
                    "session_id": "",
                    "modpack": modpack(),
                },
                "player": {
                    "id": 7,
                    "display_name": "P7",
                },
            }))
            .expect("deserialize invalid wire hello"),
        ));
        assert!(matches!(
            codec.decode(&invalid_wire_frame),
            Err(TransportError::InvalidMessage { .. })
        ));

        let empty_player_frame = frame_from_wire_message(WireLinkMessage::Hello(
            serde_json::from_value(serde_json::json!({
                "session": session(),
                "player": {
                    "id": 7,
                    "display_name": "",
                },
            }))
            .expect("deserialize empty-player hello"),
        ));
        assert_eq!(
            codec.decode(&empty_player_frame),
            Err(TransportError::InvalidMessage {
                message: "link player 7 display name is required".to_string()
            })
        );

        let empty_hash = LinkMessage::BattleAction(BattleActionFrame::new_unchecked_for_tests(
            2,
            12,
            BattleAction::Run,
            String::new(),
        ));
        assert_eq!(
            codec.encode(&empty_hash),
            Err(TransportError::MissingSessionBinding)
        );

        let codec = session_codec();
        assert_eq!(
            codec.encode(&empty_hash),
            Err(TransportError::InvalidMessage {
                message: "battle sync state hash must be non-empty".to_string()
            })
        );

        let padded_hash_frame = frame_from_wire_message(WireLinkMessage::BattleAction(
            BattleActionFrame::new_unchecked_for_tests(
                2,
                12,
                BattleAction::Run,
                " 2222".to_string(),
            ),
        ));
        assert_eq!(
            codec.decode(&padded_hash_frame),
            Err(TransportError::InvalidMessage {
                message: "battle sync state hash  2222 must be exact and untrimmed".to_string()
            })
        );

        let empty_trade_frame = frame_from_wire_message(WireLinkMessage::TradeConfirmation(
            TradeConfirmation::new_unchecked_for_tests("", 1, true),
        ));
        assert_eq!(
            codec.decode(&empty_trade_frame),
            Err(TransportError::InvalidMessage {
                message: "trade id is required".to_string()
            })
        );

        let invalid_offer_frame = frame_from_wire_message(WireLinkMessage::TradeOffer(
            TradeOffer::new_unchecked_for_tests(
                "trade-1",
                1,
                crystal_core::models::PARTY_SIZE,
                pokemon("PIKACHU", None),
            ),
        ));
        assert_eq!(
            codec.decode(&invalid_offer_frame),
            Err(TransportError::InvalidMessage {
                message: format!(
                    "party slot {} is outside the party",
                    crystal_core::models::PARTY_SIZE
                ),
            })
        );

        let empty_interaction_frame = frame_from_wire_message(WireLinkMessage::InteractionRequest(
            MultiplayerInteractionRequest::new_unchecked_for_tests(
                "",
                "user-a",
                "Player A",
                "user-b",
                MultiplayerInteractionKind::Trade,
                123,
            ),
        ));
        assert_eq!(
            codec.decode(&empty_interaction_frame),
            Err(TransportError::InvalidMessage {
                message: "interaction request id must be non-empty".to_string()
            })
        );

        let padded_disconnect_frame = frame_from_wire_message(WireLinkMessage::Disconnect {
            player_id: 1,
            reason: " done".to_string(),
        });
        assert_eq!(
            codec.decode(&padded_disconnect_frame),
            Err(TransportError::InvalidMessage {
                message: "disconnect reason must be exact and untrimmed".to_string()
            })
        );
    }

    #[test]
    fn binary_link_codec_requires_session_binding_for_gameplay_frames() {
        let codec = LinkFrameCodec::default();
        let input = PlayerInputFrame::new(2, Frame(144), 0b1001_0000).expect("input");

        assert_eq!(
            codec.encode(&LinkMessage::Input(input.clone())),
            Err(TransportError::MissingSessionBinding)
        );

        let frame = frame_from_wire_message(WireLinkMessage::Input(input));
        assert_eq!(
            codec.decode(&frame),
            Err(TransportError::MissingSessionBinding)
        );
    }

    #[test]
    fn binary_link_codec_rejects_session_bound_hello_mismatch_on_encode() {
        let codec = session_codec();
        let mut hello = hello_message();
        if let LinkMessage::Hello(hello) = &mut hello {
            *hello = LinkHello::from_session(
                LinkSessionIdentity::new(
                    "session-1",
                    SaveModpackIdentity::new("other-pack", "1234abcd").expect("other pack"),
                )
                .expect("other session"),
                player(7, "P7"),
            )
            .expect("other hello");
        }

        assert!(matches!(
            codec.encode(&hello),
            Err(TransportError::SessionMismatch { .. })
        ));
    }

    #[test]
    fn binary_link_codec_rejects_conflicting_input_direction_masks() {
        let codec = session_codec();
        let frame = frame_from_wire_message(WireLinkMessage::Input(
            PlayerInputFrame::new_unchecked_for_tests(2, 144, 0b0000_0011),
        ));

        assert_eq!(
            codec.decode(&frame),
            Err(TransportError::InvalidMessage {
                message: "lockstep input mask 0b00000011 has conflicting direction buttons"
                    .to_string(),
            })
        );
    }

    #[test]
    fn binary_link_codec_rejects_cross_session_frames() {
        let codec = session_codec();
        let other_session = LinkSessionIdentity::new(
            "session-1",
            SaveModpackIdentity::new("other-pack", "1234abcd").expect("other pack"),
        )
        .expect("other session");
        let frame = frame_from_wire_frame(WireLinkFrame {
            session: other_session,
            message: WireLinkMessage::Input(
                PlayerInputFrame::new(2, Frame(144), 0b1001_0000).expect("input"),
            ),
        });

        assert!(matches!(
            codec.decode(&frame),
            Err(TransportError::SessionMismatch { .. })
        ));
    }

    #[test]
    fn binary_link_codec_rejects_truncated_or_trailing_payloads() {
        let codec = LinkFrameCodec::default();
        let mut empty = Vec::with_capacity(HEADER_LEN);
        empty.extend_from_slice(LINK_FRAME_MAGIC);
        empty.extend_from_slice(&LINK_FRAME_VERSION.to_be_bytes());
        empty.extend_from_slice(&0_u32.to_be_bytes());
        empty.extend_from_slice(&fnv1a32_bytes(&[]).to_be_bytes());
        assert_eq!(codec.decode(&empty), Err(TransportError::EmptyPayload));

        let mut truncated = codec.encode(&hello_message()).expect("encode");
        truncated.pop();
        let declared = u32::from_be_bytes([
            truncated[LENGTH_OFFSET],
            truncated[LENGTH_OFFSET + 1],
            truncated[LENGTH_OFFSET + 2],
            truncated[LENGTH_OFFSET + 3],
        ]) as usize;

        assert_eq!(
            codec.decode(&truncated),
            Err(TransportError::LengthMismatch {
                declared,
                actual: declared - 1,
            })
        );

        let mut trailing = codec.encode(&hello_message()).expect("encode");
        let declared = u32::from_be_bytes([
            trailing[LENGTH_OFFSET],
            trailing[LENGTH_OFFSET + 1],
            trailing[LENGTH_OFFSET + 2],
            trailing[LENGTH_OFFSET + 3],
        ]) as usize;
        trailing.push(0);
        assert_eq!(
            codec.decode(&trailing),
            Err(TransportError::LengthMismatch {
                declared,
                actual: declared + 1,
            })
        );
    }

    #[test]
    fn binary_link_codec_enforces_max_frame_size() {
        assert_eq!(
            LinkFrameCodec::new(HEADER_LEN - 1),
            Err(TransportError::FrameLimitTooSmall {
                max_frame_bytes: HEADER_LEN - 1,
            })
        );
        #[cfg(target_pointer_width = "64")]
        {
            let too_large = HEADER_LEN + u32::MAX as usize + 1;
            assert_eq!(
                LinkFrameCodec::new(too_large),
                Err(TransportError::FrameLimitTooLarge {
                    max_frame_bytes: too_large,
                })
            );
        }
        let codec = LinkFrameCodec::new(HEADER_LEN).expect("codec");
        assert_eq!(
            codec.encode(&hello_message()),
            Err(TransportError::MessageTooLarge)
        );

        let oversized = vec![0; HEADER_LEN + 1];
        assert_eq!(
            codec.decode(&oversized),
            Err(TransportError::MessageTooLarge)
        );
    }

    #[test]
    fn memory_transport_delivers_binary_framed_messages_bidirectionally() {
        let (mut host, mut peer) = MemoryLinkTransport::pair_with_codec(session_codec());
        let hello = hello_message();
        let input =
            LinkMessage::Input(PlayerInputFrame::new(2, Frame(144), 0b1001_0000).expect("input"));

        host.send(hello.clone()).expect("host send");
        peer.send(input.clone()).expect("peer send");

        assert_eq!(peer.pending_inbound_frames(), 1);
        assert_eq!(host.poll().expect("host poll"), vec![input]);
        assert_eq!(peer.poll().expect("peer poll"), vec![hello]);
        assert!(host.poll().expect("host poll empty").is_empty());
    }

    #[test]
    fn memory_transport_pair_for_session_binds_gameplay_frames_to_exact_pack_session() {
        let (mut host, mut peer) =
            MemoryLinkTransport::pair_for_session(session()).expect("session transport");
        let input =
            LinkMessage::Input(PlayerInputFrame::new(2, Frame(144), 0b1001_0000).expect("input"));

        host.send(input.clone()).expect("session-bound send");

        assert_eq!(peer.poll().expect("peer poll"), vec![input]);
    }

    #[test]
    fn memory_transport_uses_codec_limits_and_rejects_corrupt_frames() {
        let (mut host, _) =
            MemoryLinkTransport::pair_with_codec(LinkFrameCodec::new(HEADER_LEN).expect("codec"));

        assert_eq!(
            host.send(hello_message()),
            Err(TransportError::MessageTooLarge)
        );

        let (_, mut peer) = MemoryLinkTransport::pair();
        peer.push_inbound_frame_for_tests(br#"{"type":"hello"}"#.to_vec());
        assert_eq!(peer.poll(), Err(TransportError::InvalidMagic));
    }

    #[test]
    fn memory_transport_disconnect_is_a_hard_error() {
        let (mut host, mut peer) = MemoryLinkTransport::pair();
        host.disconnect();
        peer.disconnect();

        assert_eq!(
            host.send(hello_message()),
            Err(TransportError::NotConnected)
        );
        assert_eq!(peer.poll(), Err(TransportError::NotConnected));
    }
}
