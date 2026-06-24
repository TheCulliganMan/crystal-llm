use std::collections::{BTreeMap, BTreeSet, VecDeque};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::battle::turn::BattleAction;
use crate::models::{PARTY_SIZE, Party, Pokemon};
use crate::save::SaveModpackIdentity;
use crate::state::GameState;
use crate::timing::Frame;
use crate::world::map::{Direction, TilePosition};

pub type PlayerId = u64;
pub type SessionId = String;
pub const LINK_PROTOCOL_VERSION: u16 = 1;
pub const LINK_PREAMBLE_BYTE: u8 = 0x00;
pub const LINK_PREAMBLE_RESPONSE: u8 = 0x61;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlayerIdentity {
    pub id: PlayerId,
    pub display_name: String,
}

impl PlayerIdentity {
    pub fn validate(&self) -> Result<(), LinkHandshakeError> {
        if self.display_name.trim().is_empty() {
            return Err(LinkHandshakeError::MissingPlayerDisplayName { player_id: self.id });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LinkSessionIdentity {
    pub protocol_version: u16,
    pub session_id: SessionId,
    pub modpack: SaveModpackIdentity,
}

impl LinkSessionIdentity {
    pub fn new(
        session_id: impl Into<String>,
        modpack: SaveModpackIdentity,
    ) -> Result<Self, LinkHandshakeError> {
        modpack
            .validate()
            .map_err(|error| LinkHandshakeError::InvalidModpackIdentity {
                message: error.to_string(),
            })?;
        Ok(Self {
            protocol_version: LINK_PROTOCOL_VERSION,
            session_id: session_id.into(),
            modpack,
        })
    }

    pub fn validate(&self) -> Result<(), LinkHandshakeError> {
        if self.session_id.trim().is_empty() {
            return Err(LinkHandshakeError::MissingSessionId);
        }
        if self.protocol_version != LINK_PROTOCOL_VERSION {
            return Err(LinkHandshakeError::ProtocolVersionMismatch {
                expected: LINK_PROTOCOL_VERSION,
                actual: self.protocol_version,
            });
        }
        self.modpack
            .validate()
            .map_err(|error| LinkHandshakeError::InvalidModpackIdentity {
                message: error.to_string(),
            })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LinkHello {
    pub session: LinkSessionIdentity,
    pub player: PlayerIdentity,
}

impl LinkHello {
    pub fn new(
        session_id: impl Into<String>,
        modpack: SaveModpackIdentity,
        player: PlayerIdentity,
    ) -> Result<Self, LinkHandshakeError> {
        let hello = Self {
            session: LinkSessionIdentity::new(session_id, modpack)?,
            player,
        };
        hello.validate()?;
        Ok(hello)
    }

    pub fn validate(&self) -> Result<(), LinkHandshakeError> {
        self.session.validate()?;
        self.player.validate()
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum LinkHandshakeError {
    #[error("link session id is required")]
    MissingSessionId,
    #[error("link protocol version {actual} does not match expected {expected}")]
    ProtocolVersionMismatch { expected: u16, actual: u16 },
    #[error("link session id {actual} does not match expected {expected}")]
    SessionMismatch { expected: String, actual: String },
    #[error("link modpack id {actual} does not match expected {expected}")]
    ModpackIdMismatch { expected: String, actual: String },
    #[error("link modpack hash {actual} does not match expected {expected}")]
    ModpackHashMismatch { expected: String, actual: String },
    #[error("link player {player_id} is not in this lobby")]
    UnknownPlayer { player_id: PlayerId },
    #[error("link player {player_id} display name is required")]
    MissingPlayerDisplayName { player_id: PlayerId },
    #[error(
        "link player {player_id} display name {actual_display_name} does not match expected {expected_display_name}"
    )]
    PlayerIdentityConflict {
        player_id: PlayerId,
        expected_display_name: String,
        actual_display_name: String,
    },
    #[error("{message}")]
    InvalidModpackIdentity { message: String },
}

pub fn validate_link_hello(
    local: &LinkSessionIdentity,
    remote: &LinkHello,
) -> Result<(), LinkHandshakeError> {
    local.validate()?;
    remote.validate()?;
    if remote.session.session_id != local.session_id {
        return Err(LinkHandshakeError::SessionMismatch {
            expected: local.session_id.clone(),
            actual: remote.session.session_id.clone(),
        });
    }
    if remote.session.modpack.id != local.modpack.id {
        return Err(LinkHandshakeError::ModpackIdMismatch {
            expected: local.modpack.id.clone(),
            actual: remote.session.modpack.id.clone(),
        });
    }
    if remote.session.modpack.hash != local.modpack.hash {
        return Err(LinkHandshakeError::ModpackHashMismatch {
            expected: local.modpack.hash.clone(),
            actual: remote.session.modpack.hash.clone(),
        });
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcceptPlayerResult {
    Added,
    Duplicate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LinkLobby {
    session: LinkSessionIdentity,
    players: BTreeMap<PlayerId, PlayerIdentity>,
}

impl LinkLobby {
    pub fn new(
        session: LinkSessionIdentity,
        local_player: PlayerIdentity,
    ) -> Result<Self, LinkHandshakeError> {
        session.validate()?;
        local_player.validate()?;
        let mut players = BTreeMap::new();
        players.insert(local_player.id, local_player);
        Ok(Self { session, players })
    }

    pub fn session(&self) -> &LinkSessionIdentity {
        &self.session
    }

    pub fn local_hello(&self, player_id: PlayerId) -> Result<LinkHello, LinkHandshakeError> {
        let player = self
            .players
            .get(&player_id)
            .cloned()
            .ok_or(LinkHandshakeError::UnknownPlayer { player_id })?;
        Ok(LinkHello {
            session: self.session.clone(),
            player,
        })
    }

    pub fn accept_hello(
        &mut self,
        hello: LinkHello,
    ) -> Result<AcceptPlayerResult, LinkHandshakeError> {
        validate_link_hello(&self.session, &hello)?;
        match self.players.get(&hello.player.id) {
            Some(existing) if existing == &hello.player => Ok(AcceptPlayerResult::Duplicate),
            Some(existing) => Err(LinkHandshakeError::PlayerIdentityConflict {
                player_id: hello.player.id,
                expected_display_name: existing.display_name.clone(),
                actual_display_name: hello.player.display_name,
            }),
            None => {
                self.players.insert(hello.player.id, hello.player);
                Ok(AcceptPlayerResult::Added)
            }
        }
    }

    pub fn players(&self) -> Vec<PlayerIdentity> {
        self.players.values().cloned().collect()
    }

    pub fn player_ids(&self) -> Vec<PlayerId> {
        self.players.keys().copied().collect()
    }

    pub fn lockstep_buffer(&self) -> LockstepBuffer {
        LockstepBuffer::new(self.player_ids())
    }

    pub fn battle_action_buffer(&self) -> BattleActionSyncBuffer {
        BattleActionSyncBuffer::from_lobby(self)
    }

    pub fn trade_buffer(
        &self,
        trade_id: impl Into<String>,
        player_a: PlayerId,
        player_b: PlayerId,
    ) -> Result<TradeSyncBuffer, TradeError> {
        TradeSyncBuffer::from_lobby(self, trade_id, player_a, player_b)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PresenceEntityType {
    Player,
    Ai,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverworldPresence {
    pub user_id: String,
    pub player_name: String,
    pub entity_type: PresenceEntityType,
    pub map_name: String,
    pub tile: TilePosition,
    pub direction: Direction,
    pub updated_at_ms: u64,
}

impl OverworldPresence {
    pub fn is_stale(&self, now_ms: u64, stale_ms: u64) -> bool {
        now_ms.saturating_sub(self.updated_at_ms) > stale_ms
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MultiplayerInteractionKind {
    Battle,
    Trade,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MultiplayerInteractionRequest {
    pub request_id: String,
    pub from_user_id: String,
    pub from_player_name: String,
    pub to_user_id: String,
    pub kind: MultiplayerInteractionKind,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MultiplayerInteractionResponse {
    pub request_id: String,
    pub from_user_id: String,
    pub to_user_id: String,
    pub kind: MultiplayerInteractionKind,
    pub accepted: bool,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BattleRngState {
    pub hardware_divider: u16,
    pub h_random_add: u8,
    pub h_random_sub: u8,
}

impl BattleRngState {
    pub fn from_seed(seed: u32) -> Self {
        let divider = ((seed ^ 0xa5a5) & 0xffff) as u16;
        Self {
            hardware_divider: if divider == 0 { 1 } else { divider },
            h_random_add: ((seed >> 8) & 0xff) as u8,
            h_random_sub: (seed & 0xff) as u8,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlayerInputFrame {
    pub player_id: PlayerId,
    pub frame: u64,
    pub joypad_mask: u8,
}

impl PlayerInputFrame {
    pub const fn new(player_id: PlayerId, frame: Frame, joypad_mask: u8) -> Self {
        Self {
            player_id,
            frame: frame.0,
            joypad_mask,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StateChecksum {
    pub frame: u64,
    pub hash: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StateChecksumFrame {
    pub player_id: PlayerId,
    pub frame: u64,
    pub hash: u32,
}

impl StateChecksumFrame {
    pub const fn new(player_id: PlayerId, frame: Frame, hash: u32) -> Self {
        Self {
            player_id,
            frame: frame.0,
            hash,
        }
    }

    pub const fn checksum(&self) -> StateChecksum {
        StateChecksum {
            frame: self.frame,
            hash: self.hash,
        }
    }

    pub fn from_game_state(
        player_id: PlayerId,
        state: &GameState,
    ) -> Result<Self, StateChecksumError> {
        let checksum = game_state_checksum(state)?;
        Ok(Self {
            player_id,
            frame: checksum.frame,
            hash: checksum.hash,
        })
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum StateChecksumError {
    #[error("failed to encode GameState for deterministic checksum: {0}")]
    Encode(String),
}

pub fn game_state_checksum(state: &GameState) -> Result<StateChecksum, StateChecksumError> {
    let bytes = bincode::serde::encode_to_vec(state, bincode::config::standard())
        .map_err(|error| StateChecksumError::Encode(error.to_string()))?;
    Ok(StateChecksum {
        frame: state.frame_counter,
        hash: fnv1a32_bytes(&bytes),
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BattleActionFrame {
    pub player_id: PlayerId,
    pub turn: u64,
    pub action: BattleAction,
    pub state_hash: Option<String>,
}

impl BattleActionFrame {
    pub fn new(player_id: PlayerId, turn: u64, action: BattleAction) -> Self {
        Self {
            player_id,
            turn,
            action,
            state_hash: None,
        }
    }

    pub fn with_state_hash(
        player_id: PlayerId,
        turn: u64,
        action: BattleAction,
        state_hash: impl Into<String>,
    ) -> Result<Self, BattleSyncError> {
        let state_hash = state_hash.into();
        if state_hash.is_empty() {
            return Err(BattleSyncError::EmptyStateHash);
        }
        Ok(Self {
            player_id,
            turn,
            action,
            state_hash: Some(state_hash),
        })
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum BattleSyncError {
    #[error("battle sync player {player_id} is not in the accepted link roster")]
    UnknownPlayer { player_id: PlayerId },
    #[error("battle sync state hash must be non-empty")]
    EmptyStateHash,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum LockstepSyncError {
    #[error("lockstep player {player_id} is not in the accepted link roster")]
    UnknownPlayer { player_id: PlayerId },
}

pub type TradeId = String;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InsertTradeFrameResult {
    Inserted,
    Duplicate,
    Conflict,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TradeParticipants {
    pub trade_id: TradeId,
    pub players: [PlayerId; 2],
}

impl TradeParticipants {
    pub fn new(
        trade_id: impl Into<String>,
        player_a: PlayerId,
        player_b: PlayerId,
    ) -> Result<Self, TradeError> {
        let trade_id = trade_id.into();
        if trade_id.is_empty() {
            return Err(TradeError::MissingTradeId);
        }
        if player_a == player_b {
            return Err(TradeError::DuplicateParticipant {
                player_id: player_a,
            });
        }
        let mut players = [player_a, player_b];
        players.sort();
        Ok(Self { trade_id, players })
    }

    pub fn contains(&self, player_id: PlayerId) -> bool {
        self.players.contains(&player_id)
    }

    pub fn other_player(&self, player_id: PlayerId) -> Option<PlayerId> {
        if self.players[0] == player_id {
            Some(self.players[1])
        } else if self.players[1] == player_id {
            Some(self.players[0])
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TradeOffer {
    pub trade_id: TradeId,
    pub player_id: PlayerId,
    pub party_slot: usize,
    pub pokemon: Pokemon,
}

impl TradeOffer {
    pub fn from_party(
        trade_id: impl Into<String>,
        player_id: PlayerId,
        party: &Party,
        party_slot: usize,
    ) -> Result<Self, TradeError> {
        let trade_id = trade_id.into();
        if trade_id.is_empty() {
            return Err(TradeError::MissingTradeId);
        }
        if party_slot >= PARTY_SIZE {
            return Err(TradeError::InvalidPartySlot { party_slot });
        }
        let pokemon = party.pokemon[party_slot]
            .clone()
            .ok_or(TradeError::EmptyPartySlot { party_slot })?;
        Ok(Self {
            trade_id,
            player_id,
            party_slot,
            pokemon,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TradeConfirmation {
    pub trade_id: TradeId,
    pub player_id: PlayerId,
    pub confirm: bool,
}

impl TradeConfirmation {
    pub fn new(trade_id: impl Into<String>, player_id: PlayerId, confirm: bool) -> Self {
        Self {
            trade_id: trade_id.into(),
            player_id,
            confirm,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TradeReplacement {
    pub party_slot: usize,
    pub received: Pokemon,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TradeOutcome {
    pub trade_id: TradeId,
    pub cancelled: bool,
    pub replacements: BTreeMap<PlayerId, TradeReplacement>,
}

impl TradeOutcome {
    pub fn apply_to_party(
        &self,
        player_id: PlayerId,
        party: &mut Party,
    ) -> Result<Option<Pokemon>, TradeError> {
        if self.cancelled {
            return Ok(None);
        }
        let replacement =
            self.replacements
                .get(&player_id)
                .ok_or(TradeError::MissingReplacement {
                    player_id,
                    trade_id: self.trade_id.clone(),
                })?;
        if replacement.party_slot >= PARTY_SIZE {
            return Err(TradeError::InvalidPartySlot {
                party_slot: replacement.party_slot,
            });
        }
        let previous = party.pokemon[replacement.party_slot]
            .replace(replacement.received.clone())
            .ok_or(TradeError::EmptyPartySlot {
                party_slot: replacement.party_slot,
            })?;
        Ok(Some(previous))
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum TradeError {
    #[error("trade id is required")]
    MissingTradeId,
    #[error("trade {actual} does not match expected {expected}")]
    TradeIdMismatch { expected: TradeId, actual: TradeId },
    #[error("trade player {player_id} is not in the accepted link roster")]
    UnknownPlayer { player_id: PlayerId },
    #[error("trade player {player_id} is not a participant in trade {trade_id}")]
    NotParticipant {
        player_id: PlayerId,
        trade_id: TradeId,
    },
    #[error("trade participant {player_id} cannot be listed twice")]
    DuplicateParticipant { player_id: PlayerId },
    #[error("party slot {party_slot} is outside the party")]
    InvalidPartySlot { party_slot: usize },
    #[error("party slot {party_slot} has no Pokemon")]
    EmptyPartySlot { party_slot: usize },
    #[error("trade {trade_id} is not ready")]
    TradeNotReady { trade_id: TradeId },
    #[error("trade {trade_id} has no replacement for player {player_id}")]
    MissingReplacement {
        player_id: PlayerId,
        trade_id: TradeId,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LinkByteFrame {
    pub player_id: PlayerId,
    pub byte: u8,
    pub clock: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LinkClockSyncFrame {
    pub player_id: PlayerId,
    pub t0: u64,
    pub t1: u64,
    pub t2: u64,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum LinkCableError {
    #[error("link cable endpoint cannot use player {player_id} as both local and remote")]
    DuplicateEndpoint { player_id: PlayerId },
    #[error("link cable player {player_id} is not in the accepted link roster")]
    UnknownPlayer { player_id: PlayerId },
    #[error("link cable frame from player {player_id} does not match remote player {expected}")]
    UnexpectedPeer {
        expected: PlayerId,
        player_id: PlayerId,
    },
    #[error("link cable clock {clock} did not advance beyond remote clock {remote_clock}")]
    ClockRegression { remote_clock: u64, clock: u64 },
    #[error("link cable expected preamble response 0x61 but got 0x{byte:02x}")]
    BadPreambleResponse { byte: u8 },
    #[error("link cable has no received bytes buffered")]
    NoBufferedByte,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkCableState {
    local_player: PlayerId,
    remote_player: PlayerId,
    local_clock: u64,
    remote_clock: u64,
    latency_samples: Vec<u64>,
    receive_buffer: VecDeque<u8>,
    established: bool,
}

impl LinkCableState {
    pub fn new(local_player: PlayerId, remote_player: PlayerId) -> Result<Self, LinkCableError> {
        if local_player == remote_player {
            return Err(LinkCableError::DuplicateEndpoint {
                player_id: local_player,
            });
        }
        Ok(Self {
            local_player,
            remote_player,
            local_clock: 0,
            remote_clock: 0,
            latency_samples: Vec::new(),
            receive_buffer: VecDeque::new(),
            established: false,
        })
    }

    pub fn from_lobby(
        lobby: &LinkLobby,
        local_player: PlayerId,
        remote_player: PlayerId,
    ) -> Result<Self, LinkCableError> {
        if !lobby.players.contains_key(&local_player) {
            return Err(LinkCableError::UnknownPlayer {
                player_id: local_player,
            });
        }
        if !lobby.players.contains_key(&remote_player) {
            return Err(LinkCableError::UnknownPlayer {
                player_id: remote_player,
            });
        }
        Self::new(local_player, remote_player)
    }

    pub const fn local_player(&self) -> PlayerId {
        self.local_player
    }

    pub const fn remote_player(&self) -> PlayerId {
        self.remote_player
    }

    pub const fn local_clock(&self) -> u64 {
        self.local_clock
    }

    pub const fn remote_clock(&self) -> u64 {
        self.remote_clock
    }

    pub const fn is_established(&self) -> bool {
        self.established
    }

    pub fn buffered_len(&self) -> usize {
        self.receive_buffer.len()
    }

    pub fn average_latency_ticks(&self) -> Option<u64> {
        if self.latency_samples.is_empty() {
            return None;
        }
        Some(self.latency_samples.iter().sum::<u64>() / self.latency_samples.len() as u64)
    }

    pub fn send_byte(&mut self, byte: u8) -> LinkByteFrame {
        self.local_clock = self.local_clock.saturating_add(1);
        LinkByteFrame {
            player_id: self.local_player,
            byte,
            clock: self.local_clock,
        }
    }

    pub fn send_bytes(&mut self, bytes: impl IntoIterator<Item = u8>) -> Vec<LinkByteFrame> {
        bytes.into_iter().map(|byte| self.send_byte(byte)).collect()
    }

    pub fn receive_byte_frame(&mut self, frame: LinkByteFrame) -> Result<(), LinkCableError> {
        self.validate_remote_frame(frame.player_id, frame.clock)?;
        self.remote_clock = frame.clock;
        self.receive_buffer.push_back(frame.byte);
        Ok(())
    }

    pub fn read_byte(&mut self) -> Result<u8, LinkCableError> {
        self.receive_buffer
            .pop_front()
            .ok_or(LinkCableError::NoBufferedByte)
    }

    pub fn host_preamble(&mut self) -> LinkByteFrame {
        self.send_byte(LINK_PREAMBLE_BYTE)
    }

    pub fn client_accept_preamble(
        &mut self,
        frame: LinkByteFrame,
    ) -> Result<Option<LinkByteFrame>, LinkCableError> {
        self.receive_byte_frame(frame)?;
        let byte = self.read_byte()?;
        if byte == LINK_PREAMBLE_BYTE {
            self.established = true;
            return Ok(Some(self.send_byte(LINK_PREAMBLE_RESPONSE)));
        }
        Ok(None)
    }

    pub fn host_accept_preamble_response(
        &mut self,
        frame: LinkByteFrame,
    ) -> Result<(), LinkCableError> {
        self.receive_byte_frame(frame)?;
        let byte = self.read_byte()?;
        if byte != LINK_PREAMBLE_RESPONSE {
            return Err(LinkCableError::BadPreambleResponse { byte });
        }
        self.established = true;
        Ok(())
    }

    pub fn sync_frame(&mut self, now_tick: u64) -> LinkClockSyncFrame {
        self.local_clock = self.local_clock.max(now_tick);
        LinkClockSyncFrame {
            player_id: self.local_player,
            t0: now_tick,
            t1: now_tick,
            t2: now_tick,
        }
    }

    pub fn receive_sync_frame(
        &mut self,
        frame: LinkClockSyncFrame,
        receive_tick: u64,
    ) -> Result<(), LinkCableError> {
        self.validate_remote_frame(frame.player_id, frame.t2)?;
        self.remote_clock = frame.t2;
        let remote_processing = frame.t2.saturating_sub(frame.t1);
        let round_trip = receive_tick
            .saturating_sub(frame.t0)
            .saturating_sub(remote_processing);
        self.latency_samples.push(round_trip);
        Ok(())
    }

    fn validate_remote_frame(&self, player_id: PlayerId, clock: u64) -> Result<(), LinkCableError> {
        if player_id != self.remote_player {
            return Err(LinkCableError::UnexpectedPeer {
                expected: self.remote_player,
                player_id,
            });
        }
        if clock <= self.remote_clock {
            return Err(LinkCableError::ClockRegression {
                remote_clock: self.remote_clock,
                clock,
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TradeSyncBuffer {
    participants: TradeParticipants,
    offers: BTreeMap<PlayerId, TradeOffer>,
    confirmations: BTreeMap<PlayerId, bool>,
}

impl TradeSyncBuffer {
    pub fn new(participants: TradeParticipants) -> Self {
        Self {
            participants,
            offers: BTreeMap::new(),
            confirmations: BTreeMap::new(),
        }
    }

    pub fn from_lobby(
        lobby: &LinkLobby,
        trade_id: impl Into<String>,
        player_a: PlayerId,
        player_b: PlayerId,
    ) -> Result<Self, TradeError> {
        let participants = TradeParticipants::new(trade_id, player_a, player_b)?;
        for player_id in participants.players {
            if !lobby.players.contains_key(&player_id) {
                return Err(TradeError::UnknownPlayer { player_id });
            }
        }
        Ok(Self::new(participants))
    }

    pub fn participants(&self) -> &TradeParticipants {
        &self.participants
    }

    pub fn insert_offer(
        &mut self,
        offer: TradeOffer,
    ) -> Result<InsertTradeFrameResult, TradeError> {
        self.validate_trade_player(&offer.trade_id, offer.player_id)?;
        if offer.party_slot >= PARTY_SIZE {
            return Err(TradeError::InvalidPartySlot {
                party_slot: offer.party_slot,
            });
        }
        match self.offers.get(&offer.player_id) {
            Some(existing) if existing == &offer => Ok(InsertTradeFrameResult::Duplicate),
            Some(_) => Ok(InsertTradeFrameResult::Conflict),
            None => {
                self.offers.insert(offer.player_id, offer);
                Ok(InsertTradeFrameResult::Inserted)
            }
        }
    }

    pub fn insert_confirmation(
        &mut self,
        confirmation: TradeConfirmation,
    ) -> Result<InsertTradeFrameResult, TradeError> {
        self.validate_trade_player(&confirmation.trade_id, confirmation.player_id)?;
        match self.confirmations.get(&confirmation.player_id) {
            Some(existing) if *existing == confirmation.confirm => {
                Ok(InsertTradeFrameResult::Duplicate)
            }
            Some(_) => Ok(InsertTradeFrameResult::Conflict),
            None => {
                self.confirmations
                    .insert(confirmation.player_id, confirmation.confirm);
                Ok(InsertTradeFrameResult::Inserted)
            }
        }
    }

    pub fn is_ready(&self) -> bool {
        self.participants
            .players
            .iter()
            .all(|player_id| self.offers.contains_key(player_id))
            && self
                .participants
                .players
                .iter()
                .all(|player_id| self.confirmations.contains_key(player_id))
    }

    pub fn outcome(&self) -> Result<TradeOutcome, TradeError> {
        if !self.is_ready() {
            return Err(TradeError::TradeNotReady {
                trade_id: self.participants.trade_id.clone(),
            });
        }
        let cancelled = self.confirmations.values().any(|confirm| !confirm);
        let mut replacements = BTreeMap::new();
        if !cancelled {
            for player_id in self.participants.players {
                let other_player = self
                    .participants
                    .other_player(player_id)
                    .expect("participant has peer");
                let local_offer = self.offers.get(&player_id).expect("ready offer");
                let remote_offer = self.offers.get(&other_player).expect("ready offer");
                replacements.insert(
                    player_id,
                    TradeReplacement {
                        party_slot: local_offer.party_slot,
                        received: remote_offer.pokemon.clone(),
                    },
                );
            }
        }
        Ok(TradeOutcome {
            trade_id: self.participants.trade_id.clone(),
            cancelled,
            replacements,
        })
    }

    fn validate_trade_player(
        &self,
        trade_id: &TradeId,
        player_id: PlayerId,
    ) -> Result<(), TradeError> {
        if trade_id != &self.participants.trade_id {
            return Err(TradeError::TradeIdMismatch {
                expected: self.participants.trade_id.clone(),
                actual: trade_id.clone(),
            });
        }
        if !self.participants.contains(player_id) {
            return Err(TradeError::NotParticipant {
                player_id,
                trade_id: self.participants.trade_id.clone(),
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InsertInputResult {
    Inserted,
    Duplicate,
    Conflict,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InsertBattleActionResult {
    Inserted,
    Duplicate,
    Conflict,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LockstepFrame {
    pub frame: u64,
    pub inputs: BTreeMap<PlayerId, u8>,
}

impl LockstepFrame {
    pub fn ordered_inputs(&self, players: &[PlayerId]) -> Option<Vec<u8>> {
        players
            .iter()
            .map(|player_id| self.inputs.get(player_id).copied())
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BattleActionTurn {
    pub turn: u64,
    pub actions: BTreeMap<PlayerId, BattleAction>,
    pub state_hashes: BTreeMap<PlayerId, String>,
}

impl BattleActionTurn {
    pub fn ordered_actions(&self, players: &[PlayerId]) -> Option<Vec<BattleAction>> {
        players
            .iter()
            .map(|player_id| self.actions.get(player_id).cloned())
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BattleActionSyncBuffer {
    players: BTreeSet<PlayerId>,
    actions: BTreeMap<u64, BTreeMap<PlayerId, BattleAction>>,
    state_hashes: BTreeMap<u64, BTreeMap<PlayerId, String>>,
}

impl BattleActionSyncBuffer {
    pub fn new(players: impl IntoIterator<Item = PlayerId>) -> Self {
        Self {
            players: players.into_iter().collect(),
            actions: BTreeMap::new(),
            state_hashes: BTreeMap::new(),
        }
    }

    pub fn from_lobby(lobby: &LinkLobby) -> Self {
        Self::new(lobby.player_ids())
    }

    pub fn players(&self) -> Vec<PlayerId> {
        self.players.iter().copied().collect()
    }

    pub fn insert_action(
        &mut self,
        action: BattleActionFrame,
    ) -> Result<InsertBattleActionResult, BattleSyncError> {
        if !self.players.contains(&action.player_id) {
            return Err(BattleSyncError::UnknownPlayer {
                player_id: action.player_id,
            });
        }
        if let Some(state_hash) = action.state_hash {
            if state_hash.is_empty() {
                return Err(BattleSyncError::EmptyStateHash);
            }

            if let Some(existing_action) = self
                .actions
                .get(&action.turn)
                .and_then(|turn_actions| turn_actions.get(&action.player_id))
            {
                if existing_action != &action.action {
                    return Ok(InsertBattleActionResult::Conflict);
                }
                if let Some(existing_hash) = self
                    .state_hashes
                    .get(&action.turn)
                    .and_then(|turn_hashes| turn_hashes.get(&action.player_id))
                {
                    if existing_hash != &state_hash {
                        return Ok(InsertBattleActionResult::Conflict);
                    }
                    return Ok(InsertBattleActionResult::Duplicate);
                }
                self.state_hashes
                    .entry(action.turn)
                    .or_default()
                    .insert(action.player_id, state_hash);
                return Ok(InsertBattleActionResult::Duplicate);
            }

            self.actions
                .entry(action.turn)
                .or_default()
                .insert(action.player_id, action.action);
            self.state_hashes
                .entry(action.turn)
                .or_default()
                .insert(action.player_id, state_hash);
            return Ok(InsertBattleActionResult::Inserted);
        }

        let turn_actions = self.actions.entry(action.turn).or_default();
        Ok(match turn_actions.get(&action.player_id) {
            Some(existing) if existing == &action.action => InsertBattleActionResult::Duplicate,
            Some(_) => InsertBattleActionResult::Conflict,
            None => {
                turn_actions.insert(action.player_id, action.action);
                InsertBattleActionResult::Inserted
            }
        })
    }

    pub fn is_turn_ready(&self, turn: u64) -> bool {
        self.actions
            .get(&turn)
            .map(|actions| {
                self.players
                    .iter()
                    .all(|player_id| actions.contains_key(player_id))
            })
            .unwrap_or(false)
    }

    pub fn turn(&self, turn: u64) -> Option<BattleActionTurn> {
        self.is_turn_ready(turn).then(|| BattleActionTurn {
            turn,
            actions: self.actions.get(&turn).cloned().unwrap_or_default(),
            state_hashes: self.state_hashes.get(&turn).cloned().unwrap_or_default(),
        })
    }

    pub fn next_ready_turn(&self, after_turn: u64) -> Option<BattleActionTurn> {
        self.actions
            .range(after_turn..)
            .find_map(|(turn, _)| self.turn(*turn))
    }

    pub fn state_hash_disagreement(&self, turn: u64) -> Option<Vec<(PlayerId, String)>> {
        let hashes = self.state_hashes.get(&turn)?;
        if !self
            .players
            .iter()
            .all(|player_id| hashes.contains_key(player_id))
        {
            return None;
        }
        let mut values = hashes.values();
        let first = values.next()?;
        let disagreement = values.any(|hash| hash != first);
        disagreement.then(|| {
            hashes
                .iter()
                .map(|(player_id, hash)| (*player_id, hash.clone()))
                .collect()
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockstepBuffer {
    players: BTreeSet<PlayerId>,
    inputs: BTreeMap<u64, BTreeMap<PlayerId, u8>>,
    checksums: BTreeMap<u64, BTreeMap<PlayerId, u32>>,
}

impl LockstepBuffer {
    pub fn new(players: impl IntoIterator<Item = PlayerId>) -> Self {
        Self {
            players: players.into_iter().collect(),
            inputs: BTreeMap::new(),
            checksums: BTreeMap::new(),
        }
    }

    pub fn players(&self) -> Vec<PlayerId> {
        self.players.iter().copied().collect()
    }

    pub fn insert_input(
        &mut self,
        input: PlayerInputFrame,
    ) -> Result<InsertInputResult, LockstepSyncError> {
        if !self.players.contains(&input.player_id) {
            return Err(LockstepSyncError::UnknownPlayer {
                player_id: input.player_id,
            });
        }
        let frame_inputs = self.inputs.entry(input.frame).or_default();
        Ok(match frame_inputs.get(&input.player_id) {
            Some(existing) if *existing == input.joypad_mask => InsertInputResult::Duplicate,
            Some(_) => InsertInputResult::Conflict,
            None => {
                frame_inputs.insert(input.player_id, input.joypad_mask);
                InsertInputResult::Inserted
            }
        })
    }

    pub fn is_frame_ready(&self, frame: u64) -> bool {
        self.inputs
            .get(&frame)
            .map(|inputs| {
                self.players
                    .iter()
                    .all(|player_id| inputs.contains_key(player_id))
            })
            .unwrap_or(false)
    }

    pub fn frame(&self, frame: u64) -> Option<LockstepFrame> {
        self.is_frame_ready(frame).then(|| LockstepFrame {
            frame,
            inputs: self.inputs.get(&frame).cloned().unwrap_or_default(),
        })
    }

    pub fn next_ready_frame(&self, after_frame: u64) -> Option<LockstepFrame> {
        self.inputs
            .range(after_frame..)
            .find_map(|(frame, _)| self.frame(*frame))
    }

    pub fn insert_checksum(
        &mut self,
        player_id: PlayerId,
        checksum: StateChecksum,
    ) -> Result<(), LockstepSyncError> {
        if !self.players.contains(&player_id) {
            return Err(LockstepSyncError::UnknownPlayer { player_id });
        }
        self.checksums
            .entry(checksum.frame)
            .or_default()
            .insert(player_id, checksum.hash);
        Ok(())
    }

    pub fn insert_checksum_frame(
        &mut self,
        checksum: StateChecksumFrame,
    ) -> Result<(), LockstepSyncError> {
        self.insert_checksum(checksum.player_id, checksum.checksum())
    }

    pub fn checksum_disagreement(&self, frame: u64) -> Option<Vec<(PlayerId, u32)>> {
        let checksums = self.checksums.get(&frame)?;
        if !self
            .players
            .iter()
            .all(|player_id| checksums.contains_key(player_id))
        {
            return None;
        }
        let mut values = checksums.iter();
        let first = values.next().map(|(_, hash)| *hash)?;
        let disagreement = checksums.values().any(|hash| *hash != first);
        disagreement.then(|| {
            checksums
                .iter()
                .map(|(player_id, hash)| (*player_id, *hash))
                .collect()
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum LinkMessage {
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

pub fn fnv1a32(input: &str) -> u32 {
    fnv1a32_bytes(input.as_bytes())
}

pub fn fnv1a32_bytes(input: &[u8]) -> u32 {
    let mut hash = 0x811c9dc5_u32;
    for byte in input {
        hash ^= *byte as u32;
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash
}

pub fn fnv1a32_hex(input: &str) -> String {
    format!("{:08x}", fnv1a32(input))
}

pub fn fnv1a32_hex_bytes(input: &[u8]) -> String {
    format!("{:08x}", fnv1a32_bytes(input))
}

pub fn latest_remote_presence<'a>(
    entries: impl IntoIterator<Item = &'a OverworldPresence>,
    local_user_id: &str,
    now_ms: u64,
    stale_ms: u64,
) -> Vec<OverworldPresence> {
    let mut by_user: BTreeMap<&str, &OverworldPresence> = BTreeMap::new();
    for entry in entries {
        if entry.user_id == local_user_id || entry.is_stale(now_ms, stale_ms) {
            continue;
        }
        match by_user.get(entry.user_id.as_str()) {
            Some(previous) if previous.updated_at_ms >= entry.updated_at_ms => {}
            _ => {
                by_user.insert(entry.user_id.as_str(), entry);
            }
        }
    }
    by_user.into_values().cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{BaseStats, Dv, PokemonSpecies};

    fn modpack(id: &str, hash: &str) -> SaveModpackIdentity {
        SaveModpackIdentity::new(id, hash).expect("modpack identity")
    }

    fn player(id: PlayerId) -> PlayerIdentity {
        PlayerIdentity {
            id,
            display_name: format!("P{id}"),
        }
    }

    fn pokemon(id: &str, item: Option<&str>) -> Pokemon {
        let mut pokemon = Pokemon::new_for_tests(
            PokemonSpecies::new_for_tests(id, BaseStats::new(45, 49, 49, 45, 65, 65)),
            12,
            Dv::from_non_hp(1, 2, 3, 4),
        );
        pokemon.item = item.map(str::to_string);
        pokemon.original_trainer_name = format!("{id}_OT");
        pokemon.original_trainer_id = id.bytes().map(u16::from).sum();
        pokemon
    }

    fn party_with(slot: usize, pokemon: Pokemon) -> Party {
        let mut party = Party::default();
        party.pokemon[slot] = Some(pokemon);
        party
    }

    #[test]
    fn link_messages_are_serializable_for_transport_neutral_netcode() {
        let message = LinkMessage::Input(PlayerInputFrame::new(2, Frame(144), 0b1001_0000));
        let json = serde_json::to_string(&message).expect("serialize link message");
        assert_eq!(
            json,
            r#"{"type":"input","player_id":2,"frame":144,"joypad_mask":144}"#
        );
        assert_eq!(
            serde_json::from_str::<LinkMessage>(&json).expect("deserialize link message"),
            message
        );
    }

    #[test]
    fn state_hash_message_carries_exact_player_and_frame_identity() {
        let message = LinkMessage::StateHash(StateChecksumFrame::new(2, Frame(144), 0xaabbccdd));
        let json = serde_json::to_string(&message).expect("serialize state hash");
        assert_eq!(
            json,
            r#"{"type":"state_hash","player_id":2,"frame":144,"hash":2864434397}"#
        );
        assert_eq!(
            serde_json::from_str::<LinkMessage>(&json).expect("deserialize state hash"),
            message
        );

        let missing_player = serde_json::from_str::<LinkMessage>(
            r#"{"type":"state_hash","frame":144,"hash":2864434397}"#,
        )
        .expect_err("state hashes must identify the reporting player")
        .to_string();
        assert!(
            missing_player.contains("missing field `player_id`"),
            "{missing_player}"
        );
    }

    #[test]
    fn game_state_checksum_uses_authoritative_serialized_state() {
        let mut state = crate::state::GameState::default();
        state.frame_counter = 144;
        state.overworld = crate::state::OverworldMemory::Active {
            map_name: "PlayersHouse2F".to_string(),
            tile: TilePosition::new(3, 3),
            facing: Direction::Down,
            mode: crate::world::movement::MovementMode::Normal,
        };
        let checksum = game_state_checksum(&state).expect("checksum");
        let frame = StateChecksumFrame::from_game_state(2, &state).expect("checksum frame");

        assert_eq!(checksum.frame, 144);
        assert_eq!(frame.player_id, 2);
        assert_eq!(frame.checksum(), checksum);

        let mut moved = state;
        moved.overworld = crate::state::OverworldMemory::Active {
            map_name: "PlayersHouse2F".to_string(),
            tile: TilePosition::new(5, 3),
            facing: Direction::Right,
            mode: crate::world::movement::MovementMode::Normal,
        };
        assert_ne!(
            game_state_checksum(&moved).expect("moved checksum").hash,
            checksum.hash
        );
    }

    #[test]
    fn hello_message_carries_protocol_and_exact_modpack_identity() {
        let hello = LinkHello::new("session-1", modpack("core-modular", "1234abcd"), player(1))
            .expect("hello");
        let message = LinkMessage::Hello(hello.clone());
        let json = serde_json::to_string(&message).expect("serialize hello");

        assert!(json.contains(r#""type":"hello""#));
        assert!(json.contains(r#""protocol_version":1"#));
        assert!(json.contains(r#""id":"core-modular""#));
        assert_eq!(
            serde_json::from_str::<LinkMessage>(&json).expect("deserialize hello"),
            message
        );
        assert_eq!(hello.session.modpack.id, "core-modular");
    }

    #[test]
    fn link_messages_reject_unknown_protocol_fields_without_transport_fallbacks() {
        let top_level_error = serde_json::from_value::<LinkMessage>(serde_json::json!({
            "type": "input",
            "player_id": 2,
            "frame": 144,
            "joypad_mask": 144,
            "rollback_window": 2
        }))
        .expect_err("input messages must not accept unversioned fields")
        .to_string();
        assert!(
            top_level_error.contains("unknown field `rollback_window`"),
            "{top_level_error}"
        );

        let nested_error = serde_json::from_value::<LinkMessage>(serde_json::json!({
            "type": "hello",
            "session": {
                "protocol_version": 1,
                "session_id": "session-1",
                "modpack": {
                    "id": "core-modular",
                    "hash": "1234abcd",
                    "normalized_id": "CORE-MODULAR"
                }
            },
            "player": {
                "id": 1,
                "display_name": "P1",
                "normalized_name": "p1"
            }
        }))
        .expect_err("hello messages must use the exact nested protocol schema")
        .to_string();
        assert!(
            nested_error.contains("unknown field `normalized_id`")
                || nested_error.contains("unknown field `normalized_name`"),
            "{nested_error}"
        );
    }

    #[test]
    fn link_handshake_requires_exact_session_protocol_and_modpack_identity() {
        let local = LinkSessionIdentity::new("session-1", modpack("core-modular", "1234abcd"))
            .expect("local");
        let matching = LinkHello::new("session-1", modpack("core-modular", "1234abcd"), player(2))
            .expect("matching");

        validate_link_hello(&local, &matching).expect("matching hello");

        let wrong_session =
            LinkHello::new("session-2", modpack("core-modular", "1234abcd"), player(2))
                .expect("wrong session");
        assert_eq!(
            validate_link_hello(&local, &wrong_session),
            Err(LinkHandshakeError::SessionMismatch {
                expected: "session-1".to_string(),
                actual: "session-2".to_string(),
            })
        );

        let wrong_hash =
            LinkHello::new("session-1", modpack("core-modular", "ffffffff"), player(2))
                .expect("wrong hash");
        assert_eq!(
            validate_link_hello(&local, &wrong_hash),
            Err(LinkHandshakeError::ModpackHashMismatch {
                expected: "1234abcd".to_string(),
                actual: "ffffffff".to_string(),
            })
        );

        let case_changed =
            LinkHello::new("session-1", modpack("CORE-MODULAR", "1234abcd"), player(2))
                .expect("case changed");
        assert_eq!(
            validate_link_hello(&local, &case_changed),
            Err(LinkHandshakeError::ModpackIdMismatch {
                expected: "core-modular".to_string(),
                actual: "CORE-MODULAR".to_string(),
            })
        );
    }

    #[test]
    fn link_handshake_rejects_empty_player_display_names_without_placeholders() {
        let empty_player = PlayerIdentity {
            id: 2,
            display_name: "  ".to_string(),
        };
        assert_eq!(
            LinkHello::new(
                "session-1",
                modpack("core-modular", "1234abcd"),
                empty_player
            ),
            Err(LinkHandshakeError::MissingPlayerDisplayName { player_id: 2 })
        );

        let local = LinkSessionIdentity::new("session-1", modpack("core-modular", "1234abcd"))
            .expect("local");
        let bypassed = LinkHello {
            session: local.clone(),
            player: PlayerIdentity {
                id: 3,
                display_name: String::new(),
            },
        };
        assert_eq!(
            validate_link_hello(&local, &bypassed),
            Err(LinkHandshakeError::MissingPlayerDisplayName { player_id: 3 })
        );

        assert_eq!(
            LinkLobby::new(
                local,
                PlayerIdentity {
                    id: 1,
                    display_name: String::new(),
                },
            ),
            Err(LinkHandshakeError::MissingPlayerDisplayName { player_id: 1 })
        );
    }

    #[test]
    fn link_handshake_rejects_protocol_drift() {
        let local = LinkSessionIdentity::new("session-1", modpack("core-modular", "1234abcd"))
            .expect("local");
        let mut remote =
            LinkHello::new("session-1", modpack("core-modular", "1234abcd"), player(2))
                .expect("remote");
        remote.session.protocol_version = LINK_PROTOCOL_VERSION + 1;

        assert_eq!(
            validate_link_hello(&local, &remote),
            Err(LinkHandshakeError::ProtocolVersionMismatch {
                expected: LINK_PROTOCOL_VERSION,
                actual: LINK_PROTOCOL_VERSION + 1,
            })
        );
    }

    #[test]
    fn link_lobby_accepts_matching_hellos_in_player_id_order() {
        let session = LinkSessionIdentity::new("session-1", modpack("core-modular", "1234abcd"))
            .expect("session");
        let mut lobby = LinkLobby::new(session.clone(), player(3)).expect("lobby");

        assert_eq!(
            lobby.accept_hello(
                LinkHello::new("session-1", modpack("core-modular", "1234abcd"), player(1))
                    .expect("player 1 hello")
            ),
            Ok(AcceptPlayerResult::Added)
        );
        assert_eq!(
            lobby.accept_hello(
                LinkHello::new("session-1", modpack("core-modular", "1234abcd"), player(2))
                    .expect("player 2 hello")
            ),
            Ok(AcceptPlayerResult::Added)
        );

        assert_eq!(lobby.session(), &session);
        assert_eq!(lobby.player_ids(), vec![1, 2, 3]);
        assert_eq!(lobby.players(), vec![player(1), player(2), player(3)]);
    }

    #[test]
    fn link_lobby_duplicate_same_player_is_idempotent() {
        let session = LinkSessionIdentity::new("session-1", modpack("core-modular", "1234abcd"))
            .expect("session");
        let mut lobby = LinkLobby::new(session, player(1)).expect("lobby");
        let hello = LinkHello::new("session-1", modpack("core-modular", "1234abcd"), player(2))
            .expect("hello");

        assert_eq!(
            lobby.accept_hello(hello.clone()),
            Ok(AcceptPlayerResult::Added)
        );
        assert_eq!(lobby.accept_hello(hello), Ok(AcceptPlayerResult::Duplicate));
        assert_eq!(lobby.player_ids(), vec![1, 2]);
    }

    #[test]
    fn link_lobby_rejects_conflicting_player_identity() {
        let session = LinkSessionIdentity::new("session-1", modpack("core-modular", "1234abcd"))
            .expect("session");
        let mut lobby = LinkLobby::new(session, player(1)).expect("lobby");
        let original = LinkHello::new("session-1", modpack("core-modular", "1234abcd"), player(2))
            .expect("original");
        let conflict = LinkHello::new(
            "session-1",
            modpack("core-modular", "1234abcd"),
            PlayerIdentity {
                id: 2,
                display_name: "P02".to_string(),
            },
        )
        .expect("conflict");

        assert_eq!(lobby.accept_hello(original), Ok(AcceptPlayerResult::Added));
        assert_eq!(
            lobby.accept_hello(conflict),
            Err(LinkHandshakeError::PlayerIdentityConflict {
                player_id: 2,
                expected_display_name: "P2".to_string(),
                actual_display_name: "P02".to_string(),
            })
        );
    }

    #[test]
    fn link_lobby_rejects_case_changed_modpack_id_before_roster_insert() {
        let session = LinkSessionIdentity::new("session-1", modpack("core-modular", "1234abcd"))
            .expect("session");
        let mut lobby = LinkLobby::new(session, player(1)).expect("lobby");
        let case_changed =
            LinkHello::new("session-1", modpack("CORE-MODULAR", "1234abcd"), player(2))
                .expect("case changed");

        assert_eq!(
            lobby.accept_hello(case_changed),
            Err(LinkHandshakeError::ModpackIdMismatch {
                expected: "core-modular".to_string(),
                actual: "CORE-MODULAR".to_string(),
            })
        );
        assert_eq!(lobby.player_ids(), vec![1]);
    }

    #[test]
    fn link_lobby_creates_lockstep_buffer_for_accepted_roster() {
        let session = LinkSessionIdentity::new("session-1", modpack("core-modular", "1234abcd"))
            .expect("session");
        let mut lobby = LinkLobby::new(session, player(4)).expect("lobby");
        lobby
            .accept_hello(
                LinkHello::new("session-1", modpack("core-modular", "1234abcd"), player(2))
                    .expect("hello"),
            )
            .expect("accept");

        let mut buffer = lobby.lockstep_buffer();
        assert_eq!(buffer.players(), vec![2, 4]);
        assert_eq!(
            buffer.insert_input(PlayerInputFrame::new(4, Frame(12), 0x10)),
            Ok(InsertInputResult::Inserted)
        );
        assert!(!buffer.is_frame_ready(12));
        assert_eq!(
            buffer.insert_input(PlayerInputFrame::new(2, Frame(12), 0x20)),
            Ok(InsertInputResult::Inserted)
        );
        assert_eq!(
            buffer
                .frame(12)
                .expect("ready")
                .ordered_inputs(&buffer.players()),
            Some(vec![0x20, 0x10])
        );
    }

    #[test]
    fn link_lobby_exports_local_hello_for_registered_player_only() {
        let session = LinkSessionIdentity::new("session-1", modpack("core-modular", "1234abcd"))
            .expect("session");
        let lobby = LinkLobby::new(session.clone(), player(1)).expect("lobby");

        assert_eq!(
            lobby.local_hello(1).expect("hello"),
            LinkHello {
                session,
                player: player(1),
            }
        );
        assert_eq!(
            lobby.local_hello(2),
            Err(LinkHandshakeError::UnknownPlayer { player_id: 2 })
        );
    }

    #[test]
    fn battle_action_sync_waits_for_roster_and_orders_exact_actions() {
        let session = LinkSessionIdentity::new("session-1", modpack("core-modular", "1234abcd"))
            .expect("session");
        let mut lobby = LinkLobby::new(session, player(4)).expect("lobby");
        lobby
            .accept_hello(
                LinkHello::new("session-1", modpack("core-modular", "1234abcd"), player(2))
                    .expect("hello"),
            )
            .expect("accept");
        let mut sync = lobby.battle_action_buffer();

        assert_eq!(
            sync.insert_action(
                BattleActionFrame::with_state_hash(
                    4,
                    1,
                    BattleAction::Item {
                        item_id: "johto_plus:EMBER_ORB".to_string(),
                    },
                    "aaaabbbb",
                )
                .expect("action")
            ),
            Ok(InsertBattleActionResult::Inserted)
        );
        assert!(!sync.is_turn_ready(1));
        assert_eq!(
            sync.insert_action(
                BattleActionFrame::with_state_hash(
                    2,
                    1,
                    BattleAction::Move { slot: 0 },
                    "aaaabbbb",
                )
                .expect("action")
            ),
            Ok(InsertBattleActionResult::Inserted)
        );

        let turn = sync.turn(1).expect("ready turn");
        assert_eq!(sync.players(), vec![2, 4]);
        assert_eq!(
            turn.ordered_actions(&sync.players()),
            Some(vec![
                BattleAction::Move { slot: 0 },
                BattleAction::Item {
                    item_id: "johto_plus:EMBER_ORB".to_string(),
                },
            ])
        );
        assert_eq!(
            turn.state_hashes.get(&2).map(String::as_str),
            Some("aaaabbbb")
        );
        assert_eq!(
            turn.state_hashes.get(&4).map(String::as_str),
            Some("aaaabbbb")
        );
        assert_eq!(sync.state_hash_disagreement(1), None);
    }

    #[test]
    fn battle_action_sync_rejects_unknown_players_and_empty_hashes() {
        let mut sync = BattleActionSyncBuffer::new([1, 2]);

        assert_eq!(
            sync.insert_action(BattleActionFrame::new(3, 1, BattleAction::Move { slot: 0 })),
            Err(BattleSyncError::UnknownPlayer { player_id: 3 })
        );
        assert_eq!(
            BattleActionFrame::with_state_hash(1, 1, BattleAction::Move { slot: 0 }, ""),
            Err(BattleSyncError::EmptyStateHash)
        );
    }

    #[test]
    fn battle_action_sync_reports_duplicates_conflicts_and_hash_disagreements() {
        let mut sync = BattleActionSyncBuffer::new([1, 2]);
        let action =
            BattleActionFrame::with_state_hash(1, 7, BattleAction::Move { slot: 0 }, "1111")
                .expect("action");

        assert_eq!(
            sync.insert_action(action.clone()),
            Ok(InsertBattleActionResult::Inserted)
        );
        assert_eq!(
            sync.insert_action(action),
            Ok(InsertBattleActionResult::Duplicate)
        );
        assert_eq!(
            sync.insert_action(
                BattleActionFrame::with_state_hash(1, 7, BattleAction::Move { slot: 0 }, "9999",)
                    .expect("hash conflict")
            ),
            Ok(InsertBattleActionResult::Conflict)
        );
        assert_eq!(
            sync.insert_action(
                BattleActionFrame::with_state_hash(1, 7, BattleAction::Run, "3333",)
                    .expect("conflict")
            ),
            Ok(InsertBattleActionResult::Conflict)
        );
        assert_eq!(
            sync.insert_action(
                BattleActionFrame::with_state_hash(
                    2,
                    7,
                    BattleAction::Switch { party_index: 3 },
                    "2222",
                )
                .expect("player 2")
            ),
            Ok(InsertBattleActionResult::Inserted)
        );

        assert_eq!(
            sync.state_hash_disagreement(7),
            Some(vec![(1, "1111".to_string()), (2, "2222".to_string())])
        );
        assert_eq!(
            sync.turn(7)
                .expect("ready")
                .ordered_actions(&sync.players()),
            Some(vec![
                BattleAction::Move { slot: 0 },
                BattleAction::Switch { party_index: 3 },
            ])
        );
    }

    #[test]
    fn battle_action_link_message_carries_exact_modpack_item_ids() {
        let message = LinkMessage::BattleAction(BattleActionFrame::new(
            2,
            9,
            BattleAction::Item {
                item_id: "johto_plus:EMBER_ORB".to_string(),
            },
        ));
        let json = serde_json::to_string(&message).expect("serialize action message");

        assert!(json.contains(r#""type":"battle_action""#));
        assert!(json.contains(r#""item_id":"johto_plus:EMBER_ORB""#));
        assert_eq!(
            serde_json::from_str::<LinkMessage>(&json).expect("deserialize action message"),
            message
        );
    }

    #[test]
    fn trade_sync_swaps_confirmed_party_slots_without_item_id_coercion() {
        let session = LinkSessionIdentity::new("session-1", modpack("core-modular", "1234abcd"))
            .expect("session");
        let mut lobby = LinkLobby::new(session, player(1)).expect("lobby");
        lobby
            .accept_hello(
                LinkHello::new("session-1", modpack("core-modular", "1234abcd"), player(2))
                    .expect("hello"),
            )
            .expect("accept");

        let pikachu = pokemon("PIKACHU", Some("johto_plus:EMBER_ORB"));
        let eevee = pokemon("EEVEE", Some("SUPER POTION"));
        let mut party_one = party_with(0, pikachu.clone());
        let mut party_two = party_with(3, eevee.clone());
        let mut trade = lobby.trade_buffer("trade-1", 2, 1).expect("trade");

        assert_eq!(trade.participants().players, [1, 2]);
        assert_eq!(
            trade.insert_offer(
                TradeOffer::from_party("trade-1", 1, &party_one, 0).expect("offer one")
            ),
            Ok(InsertTradeFrameResult::Inserted)
        );
        assert_eq!(
            trade.insert_offer(
                TradeOffer::from_party("trade-1", 2, &party_two, 3).expect("offer two")
            ),
            Ok(InsertTradeFrameResult::Inserted)
        );
        assert_eq!(
            trade.insert_confirmation(TradeConfirmation::new("trade-1", 1, true)),
            Ok(InsertTradeFrameResult::Inserted)
        );
        assert_eq!(
            trade.insert_confirmation(TradeConfirmation::new("trade-1", 2, true)),
            Ok(InsertTradeFrameResult::Inserted)
        );

        let outcome = trade.outcome().expect("outcome");
        assert!(!outcome.cancelled);
        assert_eq!(
            outcome
                .apply_to_party(1, &mut party_one)
                .expect("apply one"),
            Some(pikachu)
        );
        assert_eq!(
            outcome
                .apply_to_party(2, &mut party_two)
                .expect("apply two"),
            Some(eevee)
        );
        assert_eq!(
            party_one.pokemon[0]
                .as_ref()
                .and_then(|pokemon| pokemon.item.as_deref()),
            Some("SUPER POTION")
        );
        assert_eq!(
            party_two.pokemon[3]
                .as_ref()
                .and_then(|pokemon| pokemon.item.as_deref()),
            Some("johto_plus:EMBER_ORB")
        );
    }

    #[test]
    fn trade_sync_cancelled_trade_does_not_replace_party_slots() {
        let mut trade =
            TradeSyncBuffer::new(TradeParticipants::new("trade-1", 1, 2).expect("participants"));
        let pikachu = pokemon("PIKACHU", None);
        let eevee = pokemon("EEVEE", None);
        let mut party_one = party_with(0, pikachu.clone());

        trade
            .insert_offer(TradeOffer::from_party("trade-1", 1, &party_one, 0).expect("offer one"))
            .expect("offer one");
        trade
            .insert_offer(TradeOffer {
                trade_id: "trade-1".to_string(),
                player_id: 2,
                party_slot: 1,
                pokemon: eevee,
            })
            .expect("offer two");
        trade
            .insert_confirmation(TradeConfirmation::new("trade-1", 1, false))
            .expect("cancel");
        trade
            .insert_confirmation(TradeConfirmation::new("trade-1", 2, true))
            .expect("confirm");

        let outcome = trade.outcome().expect("outcome");
        assert!(outcome.cancelled);
        assert_eq!(outcome.replacements.len(), 0);
        assert_eq!(outcome.apply_to_party(1, &mut party_one), Ok(None));
        assert_eq!(party_one.pokemon[0], Some(pikachu));
    }

    #[test]
    fn trade_sync_rejects_unknown_players_wrong_trade_ids_and_empty_slots() {
        let session = LinkSessionIdentity::new("session-1", modpack("core-modular", "1234abcd"))
            .expect("session");
        let lobby = LinkLobby::new(session, player(1)).expect("lobby");

        assert_eq!(
            lobby.trade_buffer("trade-1", 1, 2),
            Err(TradeError::UnknownPlayer { player_id: 2 })
        );
        assert_eq!(
            TradeParticipants::new("trade-1", 1, 1),
            Err(TradeError::DuplicateParticipant { player_id: 1 })
        );
        assert_eq!(
            TradeOffer::from_party("trade-1", 1, &Party::default(), 0),
            Err(TradeError::EmptyPartySlot { party_slot: 0 })
        );

        let mut trade =
            TradeSyncBuffer::new(TradeParticipants::new("trade-1", 1, 2).expect("participants"));
        assert_eq!(
            trade.insert_confirmation(TradeConfirmation::new("trade-2", 1, true)),
            Err(TradeError::TradeIdMismatch {
                expected: "trade-1".to_string(),
                actual: "trade-2".to_string(),
            })
        );
        assert_eq!(
            trade.insert_confirmation(TradeConfirmation::new("trade-1", 3, true)),
            Err(TradeError::NotParticipant {
                player_id: 3,
                trade_id: "trade-1".to_string(),
            })
        );
    }

    #[test]
    fn trade_sync_reports_duplicate_and_conflicting_frames() {
        let pikachu = pokemon("PIKACHU", None);
        let eevee = pokemon("EEVEE", None);
        let mut trade =
            TradeSyncBuffer::new(TradeParticipants::new("trade-1", 1, 2).expect("participants"));
        let offer = TradeOffer {
            trade_id: "trade-1".to_string(),
            player_id: 1,
            party_slot: 0,
            pokemon: pikachu,
        };

        assert_eq!(
            trade.insert_offer(offer.clone()),
            Ok(InsertTradeFrameResult::Inserted)
        );
        assert_eq!(
            trade.insert_offer(offer),
            Ok(InsertTradeFrameResult::Duplicate)
        );
        assert_eq!(
            trade.insert_offer(TradeOffer {
                trade_id: "trade-1".to_string(),
                player_id: 1,
                party_slot: 1,
                pokemon: eevee,
            }),
            Ok(InsertTradeFrameResult::Conflict)
        );
        assert_eq!(
            trade.insert_confirmation(TradeConfirmation::new("trade-1", 1, true)),
            Ok(InsertTradeFrameResult::Inserted)
        );
        assert_eq!(
            trade.insert_confirmation(TradeConfirmation::new("trade-1", 1, true)),
            Ok(InsertTradeFrameResult::Duplicate)
        );
        assert_eq!(
            trade.insert_confirmation(TradeConfirmation::new("trade-1", 1, false)),
            Ok(InsertTradeFrameResult::Conflict)
        );
    }

    #[test]
    fn trade_link_messages_carry_exact_pokemon_payloads() {
        let offer = TradeOffer {
            trade_id: "trade-1".to_string(),
            player_id: 1,
            party_slot: 0,
            pokemon: pokemon("PIKACHU", Some("johto_plus:EMBER_ORB")),
        };
        let offer_message = LinkMessage::TradeOffer(offer.clone());
        let offer_json = serde_json::to_string(&offer_message).expect("serialize offer");

        assert!(offer_json.contains(r#""type":"trade_offer""#));
        assert!(offer_json.contains(r#""item":"johto_plus:EMBER_ORB""#));
        assert_eq!(
            serde_json::from_str::<LinkMessage>(&offer_json).expect("deserialize offer"),
            offer_message
        );

        let confirm_message =
            LinkMessage::TradeConfirmation(TradeConfirmation::new("trade-1", 1, true));
        let confirm_json = serde_json::to_string(&confirm_message).expect("serialize confirm");
        assert!(confirm_json.contains(r#""type":"trade_confirmation""#));
        assert_eq!(
            serde_json::from_str::<LinkMessage>(&confirm_json).expect("deserialize confirm"),
            confirm_message
        );
    }

    #[test]
    fn link_cable_preamble_establishes_exact_two_player_stream() {
        let mut host = LinkCableState::new(1, 2).expect("host");
        let mut client = LinkCableState::new(2, 1).expect("client");

        let preamble = host.host_preamble();
        assert_eq!(preamble.byte, LINK_PREAMBLE_BYTE);
        assert_eq!(preamble.clock, 1);
        let response = client
            .client_accept_preamble(preamble)
            .expect("client accept")
            .expect("response");
        assert_eq!(response.byte, LINK_PREAMBLE_RESPONSE);
        assert!(client.is_established());

        host.host_accept_preamble_response(response)
            .expect("host accept");
        assert!(host.is_established());
        assert_eq!(host.remote_clock(), 1);
        assert_eq!(client.remote_clock(), 1);
    }

    #[test]
    fn link_cable_byte_stream_buffers_and_reads_in_order() {
        let mut a = LinkCableState::new(1, 2).expect("a");
        let mut b = LinkCableState::new(2, 1).expect("b");
        let frames = a.send_bytes([0x42, 0x99, 0x7f]);

        for frame in frames {
            b.receive_byte_frame(frame).expect("receive");
        }

        assert_eq!(a.local_clock(), 3);
        assert_eq!(b.remote_clock(), 3);
        assert_eq!(b.buffered_len(), 3);
        assert_eq!(b.read_byte(), Ok(0x42));
        assert_eq!(b.read_byte(), Ok(0x99));
        assert_eq!(b.read_byte(), Ok(0x7f));
        assert_eq!(b.read_byte(), Err(LinkCableError::NoBufferedByte));
    }

    #[test]
    fn link_cable_rejects_wrong_peer_and_clock_regression() {
        let mut cable = LinkCableState::new(1, 2).expect("cable");

        assert_eq!(
            cable.receive_byte_frame(LinkByteFrame {
                player_id: 3,
                byte: 0x42,
                clock: 1,
            }),
            Err(LinkCableError::UnexpectedPeer {
                expected: 2,
                player_id: 3,
            })
        );
        cable
            .receive_byte_frame(LinkByteFrame {
                player_id: 2,
                byte: 0x42,
                clock: 1,
            })
            .expect("first");
        assert_eq!(
            cable.receive_byte_frame(LinkByteFrame {
                player_id: 2,
                byte: 0x99,
                clock: 1,
            }),
            Err(LinkCableError::ClockRegression {
                remote_clock: 1,
                clock: 1,
            })
        );
    }

    #[test]
    fn link_cable_from_lobby_requires_accepted_players() {
        let session = LinkSessionIdentity::new("session-1", modpack("core-modular", "1234abcd"))
            .expect("session");
        let lobby = LinkLobby::new(session, player(1)).expect("lobby");

        assert_eq!(
            LinkCableState::from_lobby(&lobby, 1, 2),
            Err(LinkCableError::UnknownPlayer { player_id: 2 })
        );
        assert_eq!(
            LinkCableState::new(1, 1),
            Err(LinkCableError::DuplicateEndpoint { player_id: 1 })
        );
    }

    #[test]
    fn link_cable_sync_tracks_remote_clock_and_latency_ticks() {
        let mut host = LinkCableState::new(1, 2).expect("host");
        let mut client = LinkCableState::new(2, 1).expect("client");
        let sync = host.sync_frame(100);

        client
            .receive_sync_frame(sync.clone(), 112)
            .expect("receive sync");
        assert_eq!(client.remote_clock(), 100);
        assert_eq!(client.average_latency_ticks(), Some(12));

        let message = LinkMessage::LinkClockSync(sync);
        let json = serde_json::to_string(&message).expect("serialize sync");
        assert!(json.contains(r#""type":"link_clock_sync""#));
        assert_eq!(
            serde_json::from_str::<LinkMessage>(&json).expect("deserialize sync"),
            message
        );
    }

    #[test]
    fn link_byte_messages_are_transport_neutral_json() {
        let message = LinkMessage::LinkByte(LinkByteFrame {
            player_id: 2,
            byte: LINK_PREAMBLE_RESPONSE,
            clock: 7,
        });
        let json = serde_json::to_string(&message).expect("serialize byte");

        assert_eq!(
            json,
            r#"{"type":"link_byte","player_id":2,"byte":97,"clock":7}"#
        );
        assert_eq!(
            serde_json::from_str::<LinkMessage>(&json).expect("deserialize byte"),
            message
        );
    }

    #[test]
    fn rng_state_from_seed_matches_typescript_synchronizer_formula() {
        assert_eq!(
            BattleRngState::from_seed(0x1234_5678),
            BattleRngState {
                hardware_divider: 0xf3dd,
                h_random_add: 0x56,
                h_random_sub: 0x78,
            }
        );
        assert_eq!(BattleRngState::from_seed(0xa5a5).hardware_divider, 1);
    }

    #[test]
    fn fnv_hash_matches_battle_synchronizer_helper() {
        assert_eq!(fnv1a32_hex(""), "811c9dc5");
        assert_eq!(fnv1a32_hex("battle-state"), "aa0a8273");
        assert_eq!(fnv1a32_hex_bytes(b"battle-state"), "aa0a8273");
    }

    #[test]
    fn lockstep_buffer_waits_for_all_players_and_orders_inputs() {
        let mut buffer = LockstepBuffer::new([2, 1]);
        assert_eq!(
            buffer.insert_input(PlayerInputFrame::new(2, Frame(7), 0b0001_0000)),
            Ok(InsertInputResult::Inserted)
        );
        assert!(!buffer.is_frame_ready(7));
        assert_eq!(
            buffer.insert_input(PlayerInputFrame::new(1, Frame(7), 0b1000_0000)),
            Ok(InsertInputResult::Inserted)
        );

        let frame = buffer.frame(7).expect("ready frame");
        assert_eq!(frame.frame, 7);
        assert_eq!(buffer.players(), vec![1, 2]);
        assert_eq!(
            frame.ordered_inputs(&buffer.players()),
            Some(vec![0b1000_0000, 0b0001_0000])
        );
    }

    #[test]
    fn lockstep_buffer_reports_duplicates_and_conflicts() {
        let mut buffer = LockstepBuffer::new([1, 2]);
        let input = PlayerInputFrame::new(1, Frame(3), 0x10);
        assert_eq!(
            buffer.insert_input(input.clone()),
            Ok(InsertInputResult::Inserted)
        );
        assert_eq!(buffer.insert_input(input), Ok(InsertInputResult::Duplicate));
        assert_eq!(
            buffer.insert_input(PlayerInputFrame::new(1, Frame(3), 0x20)),
            Ok(InsertInputResult::Conflict)
        );
    }

    #[test]
    fn lockstep_buffer_detects_state_hash_disagreement_after_all_players_report() {
        let mut buffer = LockstepBuffer::new([1, 2]);
        buffer
            .insert_checksum_frame(StateChecksumFrame::new(1, Frame(9), 0xaaaa))
            .expect("player 1 checksum");
        assert_eq!(buffer.checksum_disagreement(9), None);
        buffer
            .insert_checksum_frame(StateChecksumFrame::new(2, Frame(9), 0xbbbb))
            .expect("player 2 checksum");
        assert_eq!(
            buffer.checksum_disagreement(9),
            Some(vec![(1, 0xaaaa), (2, 0xbbbb)])
        );
    }

    #[test]
    fn lockstep_buffer_rejects_unknown_players_without_roster_fallback() {
        let mut buffer = LockstepBuffer::new([1, 2]);

        assert_eq!(
            buffer.insert_input(PlayerInputFrame::new(3, Frame(7), 0x10)),
            Err(LockstepSyncError::UnknownPlayer { player_id: 3 })
        );
        assert_eq!(buffer.players(), vec![1, 2]);
        assert_eq!(
            buffer.insert_checksum(
                3,
                StateChecksum {
                    frame: 7,
                    hash: 0xaaaa,
                },
            ),
            Err(LockstepSyncError::UnknownPlayer { player_id: 3 })
        );
        assert_eq!(buffer.players(), vec![1, 2]);
    }

    #[test]
    fn latest_remote_presence_filters_local_stale_and_keeps_newest_per_user() {
        let entries = vec![
            OverworldPresence {
                user_id: "local".to_string(),
                player_name: "Local".to_string(),
                entity_type: PresenceEntityType::Player,
                map_name: "NEW_BARK_TOWN".to_string(),
                tile: TilePosition::new(1, 1),
                direction: Direction::Down,
                updated_at_ms: 100,
            },
            OverworldPresence {
                user_id: "remote".to_string(),
                player_name: "Old".to_string(),
                entity_type: PresenceEntityType::Player,
                map_name: "ROUTE_29".to_string(),
                tile: TilePosition::new(2, 2),
                direction: Direction::Left,
                updated_at_ms: 50,
            },
            OverworldPresence {
                user_id: "remote".to_string(),
                player_name: "New".to_string(),
                entity_type: PresenceEntityType::Player,
                map_name: "ROUTE_29".to_string(),
                tile: TilePosition::new(3, 4),
                direction: Direction::Right,
                updated_at_ms: 150,
            },
            OverworldPresence {
                user_id: "stale".to_string(),
                player_name: "Stale".to_string(),
                entity_type: PresenceEntityType::Ai,
                map_name: "ROUTE_30".to_string(),
                tile: TilePosition::new(5, 6),
                direction: Direction::Up,
                updated_at_ms: 1,
            },
        ];

        let remote = latest_remote_presence(&entries, "local", 200, 100);
        assert_eq!(remote.len(), 1);
        assert_eq!(remote[0].user_id, "remote");
        assert_eq!(remote[0].player_name, "New");
        assert_eq!(remote[0].tile, TilePosition::new(3, 4));
    }

    #[test]
    fn presence_and_interaction_messages_are_transport_neutral_json() {
        let message = LinkMessage::Presence(OverworldPresence {
            user_id: "u1".to_string(),
            player_name: "CHRIS".to_string(),
            entity_type: PresenceEntityType::Player,
            map_name: "ROUTE_29".to_string(),
            tile: TilePosition::new(10, 12),
            direction: Direction::Up,
            updated_at_ms: 1234,
        });
        let json = serde_json::to_string(&message).expect("serialize presence");
        assert!(json.contains(r#""type":"presence""#));
        assert_eq!(
            serde_json::from_str::<LinkMessage>(&json).expect("deserialize presence"),
            message
        );
    }
}
