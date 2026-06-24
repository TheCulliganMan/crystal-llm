use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use crystal_core::multiplayer::{
    BattleActionFrame, BattleRngState, LinkByteFrame, LinkClockSyncFrame, LinkHello, LinkMessage,
    MultiplayerInteractionRequest, MultiplayerInteractionResponse, OverworldPresence, PlayerId,
    PlayerInputFrame, StateChecksumFrame, TradeConfirmation, TradeOffer,
};
use thiserror::Error;

const LINK_FRAME_MAGIC: &[u8; 8] = b"CRYSLINK";
pub const LINK_FRAME_VERSION: u16 = 1;
pub const DEFAULT_MAX_FRAME_BYTES: usize = 64 * 1024;
const HEADER_LEN: usize = LINK_FRAME_MAGIC.len() + 2 + 4;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum TransportError {
    #[error("transport is not connected")]
    NotConnected,
    #[error("message exceeds transport frame size")]
    MessageTooLarge,
    #[error("link frame is shorter than the required header")]
    FrameTooShort,
    #[error("link frame magic is invalid")]
    InvalidMagic,
    #[error("link frame version {actual} does not match expected {expected}")]
    VersionMismatch { expected: u16, actual: u16 },
    #[error("link frame payload length {declared} does not match actual {actual}")]
    LengthMismatch { declared: usize, actual: usize },
    #[error("link frame payload is not a valid link message: {message}")]
    InvalidPayload { message: String },
    #[error("link frame message violates protocol invariants: {message}")]
    InvalidMessage { message: String },
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

    pub fn pair_with_codec(codec: LinkFrameCodec) -> (Self, Self) {
        let a_to_b = Rc::new(RefCell::new(VecDeque::new()));
        let b_to_a = Rc::new(RefCell::new(VecDeque::new()));
        (
            Self {
                codec,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LinkFrameCodec {
    max_frame_bytes: usize,
}

impl Default for LinkFrameCodec {
    fn default() -> Self {
        Self::new(DEFAULT_MAX_FRAME_BYTES)
    }
}

impl LinkFrameCodec {
    pub const fn new(max_frame_bytes: usize) -> Self {
        Self { max_frame_bytes }
    }

    pub const fn max_frame_bytes(&self) -> usize {
        self.max_frame_bytes
    }

    pub fn encode(&self, message: &LinkMessage) -> Result<Vec<u8>, TransportError> {
        validate_link_message(message)?;
        let wire_message = WireLinkMessage::from(message);
        let payload = bincode::serde::encode_to_vec(&wire_message, bincode::config::standard())
            .map_err(|error| TransportError::InvalidPayload {
                message: error.to_string(),
            })?;
        let frame_len = HEADER_LEN + payload.len();
        if frame_len > self.max_frame_bytes {
            return Err(TransportError::MessageTooLarge);
        }
        let mut frame = Vec::with_capacity(frame_len);
        frame.extend_from_slice(LINK_FRAME_MAGIC);
        frame.extend_from_slice(&LINK_FRAME_VERSION.to_be_bytes());
        frame.extend_from_slice(&(payload.len() as u32).to_be_bytes());
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

        let version_offset = LINK_FRAME_MAGIC.len();
        let version = u16::from_be_bytes([frame[version_offset], frame[version_offset + 1]]);
        if version != LINK_FRAME_VERSION {
            return Err(TransportError::VersionMismatch {
                expected: LINK_FRAME_VERSION,
                actual: version,
            });
        }

        let len_offset = version_offset + 2;
        let declared = u32::from_be_bytes([
            frame[len_offset],
            frame[len_offset + 1],
            frame[len_offset + 2],
            frame[len_offset + 3],
        ]) as usize;
        let actual = frame.len() - HEADER_LEN;
        if declared != actual {
            return Err(TransportError::LengthMismatch { declared, actual });
        }

        let (message, bytes_read): (WireLinkMessage, usize) =
            bincode::serde::decode_from_slice(&frame[HEADER_LEN..], bincode::config::standard())
                .map_err(|error| TransportError::InvalidPayload {
                    message: error.to_string(),
                })?;
        if bytes_read != declared {
            return Err(TransportError::LengthMismatch {
                declared,
                actual: bytes_read,
            });
        }
        let message = message.into();
        validate_link_message(&message)?;
        Ok(message)
    }
}

fn validate_link_message(message: &LinkMessage) -> Result<(), TransportError> {
    match message {
        LinkMessage::Hello(hello) => {
            hello
                .validate()
                .map_err(|error| TransportError::InvalidMessage {
                    message: error.to_string(),
                })
        }
        LinkMessage::BattleAction(action) if action.state_hash.as_deref() == Some("") => {
            Err(TransportError::InvalidMessage {
                message: "battle action state hash must be non-empty".to_string(),
            })
        }
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crystal_core::battle::turn::BattleAction;
    use crystal_core::models::{BaseStats, Dv, Pokemon, PokemonSpecies};
    use crystal_core::multiplayer::{
        BattleActionFrame, LINK_PREAMBLE_RESPONSE, LinkByteFrame, LinkClockSyncFrame, LinkHello,
        LinkSessionIdentity, PlayerIdentity, PlayerInputFrame, StateChecksumFrame,
        TradeConfirmation, TradeOffer,
    };
    use crystal_core::save::SaveModpackIdentity;
    use crystal_core::timing::Frame;

    fn modpack() -> SaveModpackIdentity {
        SaveModpackIdentity::new("core-modular", "1234abcd").expect("modpack identity")
    }

    fn hello_message() -> LinkMessage {
        LinkMessage::Hello(LinkHello {
            session: LinkSessionIdentity::new("session-1", modpack()).expect("session"),
            player: PlayerIdentity {
                id: 7,
                display_name: "P7".to_string(),
            },
        })
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
        let payload = bincode::serde::encode_to_vec(&message, bincode::config::standard())
            .expect("encode wire payload");
        let mut frame = Vec::with_capacity(HEADER_LEN + payload.len());
        frame.extend_from_slice(LINK_FRAME_MAGIC);
        frame.extend_from_slice(&LINK_FRAME_VERSION.to_be_bytes());
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
        assert_eq!(u16::from_be_bytes([frame[8], frame[9]]), LINK_FRAME_VERSION);
        assert_eq!(codec.decode(&frame).expect("decode"), message);
    }

    #[test]
    fn binary_link_frame_round_trips_input_message() {
        let codec = LinkFrameCodec::default();
        let message = LinkMessage::Input(PlayerInputFrame::new(2, Frame(144), 0b1001_0000));
        let frame = codec.encode(&message).expect("encode");

        assert_eq!(codec.decode(&frame).expect("decode"), message);
    }

    #[test]
    fn binary_link_frame_round_trips_player_bound_state_hash_message() {
        let codec = LinkFrameCodec::default();
        let message = LinkMessage::StateHash(StateChecksumFrame::new(2, Frame(144), 0xaabbccdd));
        let frame = codec.encode(&message).expect("encode");

        assert_eq!(codec.decode(&frame).expect("decode"), message);
    }

    #[test]
    fn binary_link_frame_round_trips_battle_action_message() {
        let codec = LinkFrameCodec::default();
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
        let codec = LinkFrameCodec::default();
        let offer = LinkMessage::TradeOffer(TradeOffer {
            trade_id: "trade-1".to_string(),
            player_id: 1,
            party_slot: 0,
            pokemon: pokemon("PIKACHU", Some("johto_plus:EMBER_ORB")),
        });
        let offer_frame = codec.encode(&offer).expect("encode offer");
        assert_eq!(codec.decode(&offer_frame).expect("decode offer"), offer);

        let confirmation =
            LinkMessage::TradeConfirmation(TradeConfirmation::new("trade-1", 1, true));
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
        let codec = LinkFrameCodec::default();
        let byte = LinkMessage::LinkByte(LinkByteFrame {
            player_id: 2,
            byte: LINK_PREAMBLE_RESPONSE,
            clock: 7,
        });
        let byte_frame = codec.encode(&byte).expect("encode byte");
        assert_eq!(codec.decode(&byte_frame).expect("decode byte"), byte);

        let sync = LinkMessage::LinkClockSync(LinkClockSyncFrame {
            player_id: 1,
            t0: 100,
            t1: 101,
            t2: 102,
        });
        let sync_frame = codec.encode(&sync).expect("encode sync");
        assert_eq!(codec.decode(&sync_frame).expect("decode sync"), sync);
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
        frame[9] = LINK_FRAME_VERSION as u8 + 1;

        assert_eq!(
            codec.decode(&frame),
            Err(TransportError::VersionMismatch {
                expected: LINK_FRAME_VERSION,
                actual: LINK_FRAME_VERSION + 1,
            })
        );
    }

    #[test]
    fn binary_link_codec_rejects_messages_that_bypass_protocol_constructors() {
        let codec = LinkFrameCodec::default();
        let invalid_hello = LinkMessage::Hello(LinkHello {
            session: LinkSessionIdentity {
                protocol_version: LINK_FRAME_VERSION,
                session_id: String::new(),
                modpack: modpack(),
            },
            player: PlayerIdentity {
                id: 7,
                display_name: "P7".to_string(),
            },
        });
        assert!(matches!(
            codec.encode(&invalid_hello),
            Err(TransportError::InvalidMessage { .. })
        ));

        let invalid_wire_frame = frame_from_wire_message(WireLinkMessage::Hello(LinkHello {
            session: LinkSessionIdentity {
                protocol_version: LINK_FRAME_VERSION,
                session_id: String::new(),
                modpack: modpack(),
            },
            player: PlayerIdentity {
                id: 7,
                display_name: "P7".to_string(),
            },
        }));
        assert!(matches!(
            codec.decode(&invalid_wire_frame),
            Err(TransportError::InvalidMessage { .. })
        ));

        let empty_player_frame = frame_from_wire_message(WireLinkMessage::Hello(LinkHello {
            session: LinkSessionIdentity::new("session-1", modpack()).expect("session"),
            player: PlayerIdentity {
                id: 7,
                display_name: String::new(),
            },
        }));
        assert_eq!(
            codec.decode(&empty_player_frame),
            Err(TransportError::InvalidMessage {
                message: "link player 7 display name is required".to_string()
            })
        );

        let empty_hash = LinkMessage::BattleAction(BattleActionFrame {
            player_id: 2,
            turn: 12,
            action: BattleAction::Run,
            state_hash: Some(String::new()),
        });
        assert_eq!(
            codec.encode(&empty_hash),
            Err(TransportError::InvalidMessage {
                message: "battle action state hash must be non-empty".to_string()
            })
        );
    }

    #[test]
    fn binary_link_codec_rejects_truncated_or_trailing_payloads() {
        let codec = LinkFrameCodec::default();
        let mut truncated = codec.encode(&hello_message()).expect("encode");
        truncated.pop();
        let declared =
            u32::from_be_bytes([truncated[10], truncated[11], truncated[12], truncated[13]])
                as usize;

        assert_eq!(
            codec.decode(&truncated),
            Err(TransportError::LengthMismatch {
                declared,
                actual: declared - 1,
            })
        );

        let mut trailing = codec.encode(&hello_message()).expect("encode");
        let declared =
            u32::from_be_bytes([trailing[10], trailing[11], trailing[12], trailing[13]]) as usize;
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
        let codec = LinkFrameCodec::new(HEADER_LEN);
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
        let (mut host, mut peer) = MemoryLinkTransport::pair();
        let hello = hello_message();
        let input = LinkMessage::Input(PlayerInputFrame::new(2, Frame(144), 0b1001_0000));

        host.send(hello.clone()).expect("host send");
        peer.send(input.clone()).expect("peer send");

        assert_eq!(peer.pending_inbound_frames(), 1);
        assert_eq!(host.poll().expect("host poll"), vec![input]);
        assert_eq!(peer.poll().expect("peer poll"), vec![hello]);
        assert!(host.poll().expect("host poll empty").is_empty());
    }

    #[test]
    fn memory_transport_uses_codec_limits_and_rejects_corrupt_frames() {
        let (mut host, _) = MemoryLinkTransport::pair_with_codec(LinkFrameCodec::new(HEADER_LEN));

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
