use std::collections::{BTreeMap, BTreeSet, VecDeque};

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use thiserror::Error;

use crate::battle::turn::BattleAction;
use crate::input::{
    B_PAD_A, B_PAD_B, B_PAD_DOWN, B_PAD_LEFT, B_PAD_RIGHT, B_PAD_SELECT, B_PAD_START, B_PAD_UP,
};
use crate::models::{PARTY_SIZE, Party, Pokemon};
use crate::save::{SaveGameSummary, SaveModpackIdentity, validate_pack_content_hash};
use crate::state::{GameCommand, GameEvent, GameState, GameStateFrameError};
use crate::timing::Frame;
use crate::world::map::{Direction, TilePosition};

pub type PlayerId = u64;
pub type SessionId = String;
pub const LINK_PROTOCOL_VERSION: u16 = 2;
pub const LINK_PREAMBLE_BYTE: u8 = 0x00;
pub const LINK_PREAMBLE_RESPONSE: u8 = 0x61;
const LINK_MESSAGE_MAGIC: &[u8; 12] = b"CRYSTALLINK\0";
const LINK_MESSAGE_VERSION_OFFSET: usize = LINK_MESSAGE_MAGIC.len();
const LINK_MESSAGE_PAYLOAD_LENGTH_OFFSET: usize = LINK_MESSAGE_VERSION_OFFSET + 2;
const LINK_MESSAGE_PAYLOAD_HASH_OFFSET: usize = LINK_MESSAGE_PAYLOAD_LENGTH_OFFSET + 4;
const LINK_MESSAGE_HEADER_LEN: usize = LINK_MESSAGE_PAYLOAD_HASH_OFFSET + 4;
const BATTLE_MOVE_SLOTS: usize = 4;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlayerIdentity {
    id: PlayerId,
    display_name: String,
}

impl<'de> Deserialize<'de> for PlayerIdentity {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawPlayerIdentity {
            id: PlayerId,
            display_name: String,
        }

        let raw = RawPlayerIdentity::deserialize(deserializer)?;
        PlayerIdentity::new(raw.id, raw.display_name).map_err(serde::de::Error::custom)
    }
}

impl PlayerIdentity {
    pub fn new(id: PlayerId, display_name: impl Into<String>) -> Result<Self, LinkHandshakeError> {
        let player = Self {
            id,
            display_name: display_name.into(),
        };
        player.validate()?;
        Ok(player)
    }

    pub fn validate(&self) -> Result<(), LinkHandshakeError> {
        if self.id == 0 {
            return Err(LinkHandshakeError::InvalidPlayerIdentity { player_id: self.id });
        }
        if self.display_name.is_empty() {
            return Err(LinkHandshakeError::MissingPlayerDisplayName { player_id: self.id });
        }
        if !is_exact_multiplayer_text(&self.display_name) {
            return Err(LinkHandshakeError::InvalidPlayerDisplayName {
                player_id: self.id,
                display_name: self.display_name.clone(),
            });
        }
        Ok(())
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn new_unchecked_for_tests(id: PlayerId, display_name: impl Into<String>) -> Self {
        Self {
            id,
            display_name: display_name.into(),
        }
    }

    pub fn id(&self) -> PlayerId {
        self.id
    }

    pub fn display_name(&self) -> &str {
        &self.display_name
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LinkSessionIdentity {
    protocol_version: u16,
    session_id: SessionId,
    modpack: SaveModpackIdentity,
    pack_content_hash: String,
}

impl<'de> Deserialize<'de> for LinkSessionIdentity {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawLinkSessionIdentity {
            protocol_version: u16,
            session_id: SessionId,
            modpack: SaveModpackIdentity,
            pack_content_hash: String,
        }

        let raw = RawLinkSessionIdentity::deserialize(deserializer)?;
        let session = Self {
            protocol_version: raw.protocol_version,
            session_id: raw.session_id,
            modpack: raw.modpack,
            pack_content_hash: raw.pack_content_hash,
        };
        session.validate().map_err(serde::de::Error::custom)?;
        Ok(session)
    }
}

impl LinkSessionIdentity {
    pub fn new(
        session_id: impl Into<String>,
        modpack: SaveModpackIdentity,
        pack_content_hash: impl Into<String>,
    ) -> Result<Self, LinkHandshakeError> {
        modpack
            .validate()
            .map_err(|error| LinkHandshakeError::InvalidModpackIdentity {
                message: error.to_string(),
            })?;
        let pack_content_hash = pack_content_hash.into();
        validate_pack_content_hash(&pack_content_hash).map_err(|error| {
            LinkHandshakeError::InvalidModpackIdentity {
                message: error.to_string(),
            }
        })?;
        let session = Self {
            protocol_version: LINK_PROTOCOL_VERSION,
            session_id: session_id.into(),
            modpack,
            pack_content_hash,
        };
        session.validate()?;
        Ok(session)
    }

    pub fn validate(&self) -> Result<(), LinkHandshakeError> {
        if self.session_id.is_empty() {
            return Err(LinkHandshakeError::MissingSessionId);
        }
        if self.session_id.trim() != self.session_id {
            return Err(LinkHandshakeError::InvalidSessionId {
                session_id: self.session_id.clone(),
            });
        }
        if !is_exact_session_id(&self.session_id) {
            return Err(LinkHandshakeError::InvalidSessionId {
                session_id: self.session_id.clone(),
            });
        }
        if has_reserved_session_id_prefix(&self.session_id) {
            return Err(LinkHandshakeError::ReservedSessionId {
                session_id: self.session_id.clone(),
            });
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
            })?;
        validate_pack_content_hash(&self.pack_content_hash).map_err(|error| {
            LinkHandshakeError::InvalidModpackIdentity {
                message: error.to_string(),
            }
        })
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn new_unchecked_for_tests(
        protocol_version: u16,
        session_id: impl Into<String>,
        modpack: SaveModpackIdentity,
        pack_content_hash: impl Into<String>,
    ) -> Self {
        Self {
            protocol_version,
            session_id: session_id.into(),
            modpack,
            pack_content_hash: pack_content_hash.into(),
        }
    }

    pub fn protocol_version(&self) -> u16 {
        self.protocol_version
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub fn modpack(&self) -> &SaveModpackIdentity {
        &self.modpack
    }

    pub fn pack_content_hash(&self) -> &str {
        &self.pack_content_hash
    }
}

fn is_exact_session_id(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
}

fn has_reserved_session_id_prefix(value: &str) -> bool {
    let lowered = value.to_ascii_lowercase();
    lowered.starts_with("fallback") || lowered.starts_with("legacy")
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LinkHello {
    session: LinkSessionIdentity,
    player: PlayerIdentity,
}

impl<'de> Deserialize<'de> for LinkHello {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawLinkHello {
            session: LinkSessionIdentity,
            player: PlayerIdentity,
        }

        let raw = RawLinkHello::deserialize(deserializer)?;
        LinkHello::from_session(raw.session, raw.player).map_err(serde::de::Error::custom)
    }
}

impl LinkHello {
    pub fn new(
        session_id: impl Into<String>,
        modpack: SaveModpackIdentity,
        pack_content_hash: impl Into<String>,
        player: PlayerIdentity,
    ) -> Result<Self, LinkHandshakeError> {
        let hello = Self {
            session: LinkSessionIdentity::new(session_id, modpack, pack_content_hash)?,
            player,
        };
        hello.validate()?;
        Ok(hello)
    }

    pub fn from_session(
        session: LinkSessionIdentity,
        player: PlayerIdentity,
    ) -> Result<Self, LinkHandshakeError> {
        let hello = Self { session, player };
        hello.validate()?;
        Ok(hello)
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn new_unchecked_for_tests(session: LinkSessionIdentity, player: PlayerIdentity) -> Self {
        Self { session, player }
    }

    pub fn validate(&self) -> Result<(), LinkHandshakeError> {
        self.session.validate()?;
        self.player.validate()
    }

    pub fn session(&self) -> &LinkSessionIdentity {
        &self.session
    }

    pub fn player(&self) -> &PlayerIdentity {
        &self.player
    }

    pub fn into_player(self) -> PlayerIdentity {
        self.player
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum LinkHandshakeError {
    #[error("link session id is required")]
    MissingSessionId,
    #[error("link session id {session_id} must be an exact non-empty value")]
    InvalidSessionId { session_id: String },
    #[error("link session id {session_id} uses reserved runtime session prefix")]
    ReservedSessionId { session_id: String },
    #[error("link protocol version {actual} does not match expected {expected}")]
    ProtocolVersionMismatch { expected: u16, actual: u16 },
    #[error("link session id {actual} does not match expected {expected}")]
    SessionMismatch { expected: String, actual: String },
    #[error("link modpack id {actual} does not match expected {expected}")]
    ModpackIdMismatch { expected: String, actual: String },
    #[error("link modpack hash {actual} does not match expected {expected}")]
    ModpackHashMismatch { expected: String, actual: String },
    #[error("link pack content hash {actual} does not match expected {expected}")]
    PackContentHashMismatch { expected: String, actual: String },
    #[error("link player {player_id} is not in this lobby")]
    UnknownPlayer { player_id: PlayerId },
    #[error("link player id {player_id} is not a valid link identity")]
    InvalidPlayerIdentity { player_id: PlayerId },
    #[error("link player {player_id} display name is required")]
    MissingPlayerDisplayName { player_id: PlayerId },
    #[error("link player {player_id} display name {display_name} must be an exact non-empty value")]
    InvalidPlayerDisplayName {
        player_id: PlayerId,
        display_name: String,
    },
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
    remote.validate()?;
    validate_link_session_identity(local, remote.session())
}

pub fn validate_link_session_identity(
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
    if actual.pack_content_hash() != expected.pack_content_hash() {
        return Err(LinkHandshakeError::PackContentHashMismatch {
            expected: expected.pack_content_hash().to_string(),
            actual: actual.pack_content_hash().to_string(),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcceptPlayerResult {
    Added,
    Duplicate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LinkLobby {
    session: LinkSessionIdentity,
    players: BTreeMap<PlayerId, PlayerIdentity>,
}

impl<'de> Deserialize<'de> for LinkLobby {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawLinkLobby {
            session: LinkSessionIdentity,
            players: BTreeMap<PlayerId, PlayerIdentity>,
        }

        let raw = RawLinkLobby::deserialize(deserializer)?;
        raw.session.validate().map_err(serde::de::Error::custom)?;
        if raw.players.is_empty() {
            return Err(serde::de::Error::custom(
                LinkHandshakeError::UnknownPlayer { player_id: 0 },
            ));
        }
        for (player_id, player) in &raw.players {
            if *player_id != player.id() {
                return Err(serde::de::Error::custom(
                    LinkHandshakeError::InvalidPlayerIdentity {
                        player_id: *player_id,
                    },
                ));
            }
            player.validate().map_err(serde::de::Error::custom)?;
        }
        Ok(Self {
            session: raw.session,
            players: raw.players,
        })
    }
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
        let player_id = hello.player().id();
        match self.players.get(&player_id) {
            Some(existing) if existing == hello.player() => Ok(AcceptPlayerResult::Duplicate),
            Some(existing) => Err(LinkHandshakeError::PlayerIdentityConflict {
                player_id,
                expected_display_name: existing.display_name.clone(),
                actual_display_name: hello.player().display_name().to_string(),
            }),
            None => {
                self.players.insert(player_id, hello.into_player());
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

    pub fn lockstep_buffer(&self) -> Result<LockstepBuffer, LockstepSyncError> {
        LockstepBuffer::new(self.player_ids())
    }

    pub fn validate_save_checkpoint(
        &self,
        checkpoint: &SaveCheckpointFrame,
    ) -> Result<(), SaveCheckpointFrameError> {
        checkpoint.validate_for_players(self.player_ids())
    }

    pub fn battle_action_buffer(&self) -> Result<BattleActionSyncBuffer, BattleSyncError> {
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
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum PresenceEntityType {
    Player,
    Ai,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OverworldPresence {
    user_id: String,
    player_name: String,
    entity_type: PresenceEntityType,
    map_name: String,
    tile: TilePosition,
    direction: Direction,
    updated_at_ms: u64,
}

impl<'de> Deserialize<'de> for OverworldPresence {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawOverworldPresence {
            user_id: String,
            player_name: String,
            entity_type: PresenceEntityType,
            map_name: String,
            tile: TilePosition,
            direction: Direction,
            updated_at_ms: u64,
        }

        let raw = RawOverworldPresence::deserialize(deserializer)?;
        OverworldPresence::new(
            raw.user_id,
            raw.player_name,
            raw.entity_type,
            raw.map_name,
            raw.tile,
            raw.direction,
            raw.updated_at_ms,
        )
        .map_err(serde::de::Error::custom)
    }
}

impl OverworldPresence {
    pub fn new(
        user_id: impl Into<String>,
        player_name: impl Into<String>,
        entity_type: PresenceEntityType,
        map_name: impl Into<String>,
        tile: TilePosition,
        direction: Direction,
        updated_at_ms: u64,
    ) -> Result<Self, MultiplayerMessageError> {
        let presence = Self {
            user_id: user_id.into(),
            player_name: player_name.into(),
            entity_type,
            map_name: map_name.into(),
            tile,
            direction,
            updated_at_ms,
        };
        presence.validate()?;
        Ok(presence)
    }

    pub fn validate(&self) -> Result<(), MultiplayerMessageError> {
        validate_multiplayer_user_id("presence user id", &self.user_id)?;
        validate_multiplayer_text("presence player name", &self.player_name)?;
        validate_multiplayer_token("presence map name", &self.map_name)?;
        validate_presence_tile(self.tile)
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn new_unchecked_for_tests(
        user_id: impl Into<String>,
        player_name: impl Into<String>,
        entity_type: PresenceEntityType,
        map_name: impl Into<String>,
        tile: TilePosition,
        direction: Direction,
        updated_at_ms: u64,
    ) -> Self {
        Self {
            user_id: user_id.into(),
            player_name: player_name.into(),
            entity_type,
            map_name: map_name.into(),
            tile,
            direction,
            updated_at_ms,
        }
    }

    pub fn is_stale(&self, now_ms: u64, stale_ms: u64) -> bool {
        now_ms.saturating_sub(self.updated_at_ms) > stale_ms
    }

    pub fn user_id(&self) -> &str {
        &self.user_id
    }

    pub fn player_name(&self) -> &str {
        &self.player_name
    }

    pub const fn entity_type(&self) -> PresenceEntityType {
        self.entity_type
    }

    pub fn map_name(&self) -> &str {
        &self.map_name
    }

    pub const fn tile(&self) -> TilePosition {
        self.tile
    }

    pub const fn direction(&self) -> Direction {
        self.direction
    }

    pub const fn updated_at_ms(&self) -> u64 {
        self.updated_at_ms
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SessionOverworldPresence {
    session: LinkSessionIdentity,
    presence: OverworldPresence,
}

impl<'de> Deserialize<'de> for SessionOverworldPresence {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawSessionOverworldPresence {
            session: LinkSessionIdentity,
            presence: OverworldPresence,
        }

        let raw = RawSessionOverworldPresence::deserialize(deserializer)?;
        SessionOverworldPresence::new(raw.session, raw.presence).map_err(serde::de::Error::custom)
    }
}

impl SessionOverworldPresence {
    pub fn new(session: LinkSessionIdentity, presence: OverworldPresence) -> Result<Self, String> {
        let frame = Self { session, presence };
        frame.validate()?;
        Ok(frame)
    }

    pub fn validate(&self) -> Result<(), String> {
        self.session.validate().map_err(|error| error.to_string())?;
        self.presence.validate().map_err(|error| error.to_string())
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn new_unchecked_for_tests(
        session: LinkSessionIdentity,
        presence: OverworldPresence,
    ) -> Self {
        Self { session, presence }
    }

    pub fn session(&self) -> &LinkSessionIdentity {
        &self.session
    }

    pub fn presence(&self) -> &OverworldPresence {
        &self.presence
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum MultiplayerInteractionKind {
    Battle,
    Trade,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MultiplayerInteractionRequest {
    request_id: String,
    from_user_id: String,
    from_player_name: String,
    to_user_id: String,
    kind: MultiplayerInteractionKind,
    timestamp_ms: u64,
}

impl<'de> Deserialize<'de> for MultiplayerInteractionRequest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawMultiplayerInteractionRequest {
            request_id: String,
            from_user_id: String,
            from_player_name: String,
            to_user_id: String,
            kind: MultiplayerInteractionKind,
            timestamp_ms: u64,
        }

        let raw = RawMultiplayerInteractionRequest::deserialize(deserializer)?;
        MultiplayerInteractionRequest::new(
            raw.request_id,
            raw.from_user_id,
            raw.from_player_name,
            raw.to_user_id,
            raw.kind,
            raw.timestamp_ms,
        )
        .map_err(serde::de::Error::custom)
    }
}

impl MultiplayerInteractionRequest {
    pub fn new(
        request_id: impl Into<String>,
        from_user_id: impl Into<String>,
        from_player_name: impl Into<String>,
        to_user_id: impl Into<String>,
        kind: MultiplayerInteractionKind,
        timestamp_ms: u64,
    ) -> Result<Self, MultiplayerMessageError> {
        let request = Self {
            request_id: request_id.into(),
            from_user_id: from_user_id.into(),
            from_player_name: from_player_name.into(),
            to_user_id: to_user_id.into(),
            kind,
            timestamp_ms,
        };
        request.validate()?;
        Ok(request)
    }

    pub fn validate(&self) -> Result<(), MultiplayerMessageError> {
        validate_multiplayer_token("interaction request id", &self.request_id)?;
        validate_multiplayer_user_id("interaction request source user id", &self.from_user_id)?;
        validate_multiplayer_text(
            "interaction request source player name",
            &self.from_player_name,
        )?;
        validate_multiplayer_user_id("interaction request target user id", &self.to_user_id)?;
        validate_distinct_interaction_users(
            "interaction request",
            &self.from_user_id,
            &self.to_user_id,
        )
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn new_unchecked_for_tests(
        request_id: impl Into<String>,
        from_user_id: impl Into<String>,
        from_player_name: impl Into<String>,
        to_user_id: impl Into<String>,
        kind: MultiplayerInteractionKind,
        timestamp_ms: u64,
    ) -> Self {
        Self {
            request_id: request_id.into(),
            from_user_id: from_user_id.into(),
            from_player_name: from_player_name.into(),
            to_user_id: to_user_id.into(),
            kind,
            timestamp_ms,
        }
    }

    pub fn request_id(&self) -> &str {
        &self.request_id
    }

    pub fn from_user_id(&self) -> &str {
        &self.from_user_id
    }

    pub fn from_player_name(&self) -> &str {
        &self.from_player_name
    }

    pub fn to_user_id(&self) -> &str {
        &self.to_user_id
    }

    pub const fn kind(&self) -> MultiplayerInteractionKind {
        self.kind
    }

    pub const fn timestamp_ms(&self) -> u64 {
        self.timestamp_ms
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SessionMultiplayerInteractionRequest {
    session: LinkSessionIdentity,
    request: MultiplayerInteractionRequest,
}

impl<'de> Deserialize<'de> for SessionMultiplayerInteractionRequest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawSessionMultiplayerInteractionRequest {
            session: LinkSessionIdentity,
            request: MultiplayerInteractionRequest,
        }

        let raw = RawSessionMultiplayerInteractionRequest::deserialize(deserializer)?;
        SessionMultiplayerInteractionRequest::new(raw.session, raw.request)
            .map_err(serde::de::Error::custom)
    }
}

impl SessionMultiplayerInteractionRequest {
    pub fn new(
        session: LinkSessionIdentity,
        request: MultiplayerInteractionRequest,
    ) -> Result<Self, String> {
        let frame = Self { session, request };
        frame.validate()?;
        Ok(frame)
    }

    pub fn validate(&self) -> Result<(), String> {
        self.session.validate().map_err(|error| error.to_string())?;
        self.request.validate().map_err(|error| error.to_string())
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn new_unchecked_for_tests(
        session: LinkSessionIdentity,
        request: MultiplayerInteractionRequest,
    ) -> Self {
        Self { session, request }
    }

    pub fn session(&self) -> &LinkSessionIdentity {
        &self.session
    }

    pub fn request(&self) -> &MultiplayerInteractionRequest {
        &self.request
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MultiplayerInteractionResponse {
    request_id: String,
    from_user_id: String,
    to_user_id: String,
    kind: MultiplayerInteractionKind,
    accepted: bool,
    timestamp_ms: u64,
}

impl<'de> Deserialize<'de> for MultiplayerInteractionResponse {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawMultiplayerInteractionResponse {
            request_id: String,
            from_user_id: String,
            to_user_id: String,
            kind: MultiplayerInteractionKind,
            accepted: bool,
            timestamp_ms: u64,
        }

        let raw = RawMultiplayerInteractionResponse::deserialize(deserializer)?;
        MultiplayerInteractionResponse::new(
            raw.request_id,
            raw.from_user_id,
            raw.to_user_id,
            raw.kind,
            raw.accepted,
            raw.timestamp_ms,
        )
        .map_err(serde::de::Error::custom)
    }
}

impl MultiplayerInteractionResponse {
    pub fn new(
        request_id: impl Into<String>,
        from_user_id: impl Into<String>,
        to_user_id: impl Into<String>,
        kind: MultiplayerInteractionKind,
        accepted: bool,
        timestamp_ms: u64,
    ) -> Result<Self, MultiplayerMessageError> {
        let response = Self {
            request_id: request_id.into(),
            from_user_id: from_user_id.into(),
            to_user_id: to_user_id.into(),
            kind,
            accepted,
            timestamp_ms,
        };
        response.validate()?;
        Ok(response)
    }

    pub fn validate(&self) -> Result<(), MultiplayerMessageError> {
        validate_multiplayer_token("interaction response id", &self.request_id)?;
        validate_multiplayer_user_id("interaction response source user id", &self.from_user_id)?;
        validate_multiplayer_user_id("interaction response target user id", &self.to_user_id)?;
        validate_distinct_interaction_users(
            "interaction response",
            &self.from_user_id,
            &self.to_user_id,
        )
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn new_unchecked_for_tests(
        request_id: impl Into<String>,
        from_user_id: impl Into<String>,
        to_user_id: impl Into<String>,
        kind: MultiplayerInteractionKind,
        accepted: bool,
        timestamp_ms: u64,
    ) -> Self {
        Self {
            request_id: request_id.into(),
            from_user_id: from_user_id.into(),
            to_user_id: to_user_id.into(),
            kind,
            accepted,
            timestamp_ms,
        }
    }

    pub fn request_id(&self) -> &str {
        &self.request_id
    }

    pub fn from_user_id(&self) -> &str {
        &self.from_user_id
    }

    pub fn to_user_id(&self) -> &str {
        &self.to_user_id
    }

    pub const fn kind(&self) -> MultiplayerInteractionKind {
        self.kind
    }

    pub const fn accepted(&self) -> bool {
        self.accepted
    }

    pub const fn timestamp_ms(&self) -> u64 {
        self.timestamp_ms
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SessionMultiplayerInteractionResponse {
    session: LinkSessionIdentity,
    response: MultiplayerInteractionResponse,
}

impl<'de> Deserialize<'de> for SessionMultiplayerInteractionResponse {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawSessionMultiplayerInteractionResponse {
            session: LinkSessionIdentity,
            response: MultiplayerInteractionResponse,
        }

        let raw = RawSessionMultiplayerInteractionResponse::deserialize(deserializer)?;
        SessionMultiplayerInteractionResponse::new(raw.session, raw.response)
            .map_err(serde::de::Error::custom)
    }
}

impl SessionMultiplayerInteractionResponse {
    pub fn new(
        session: LinkSessionIdentity,
        response: MultiplayerInteractionResponse,
    ) -> Result<Self, String> {
        let frame = Self { session, response };
        frame.validate()?;
        Ok(frame)
    }

    pub fn validate(&self) -> Result<(), String> {
        self.session.validate().map_err(|error| error.to_string())?;
        self.response.validate().map_err(|error| error.to_string())
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn new_unchecked_for_tests(
        session: LinkSessionIdentity,
        response: MultiplayerInteractionResponse,
    ) -> Self {
        Self { session, response }
    }

    pub fn session(&self) -> &LinkSessionIdentity {
        &self.session
    }

    pub fn response(&self) -> &MultiplayerInteractionResponse {
        &self.response
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SessionDisconnectFrame {
    session: LinkSessionIdentity,
    player_id: PlayerId,
    reason: String,
}

impl<'de> Deserialize<'de> for SessionDisconnectFrame {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawSessionDisconnectFrame {
            session: LinkSessionIdentity,
            player_id: PlayerId,
            reason: String,
        }

        let raw = RawSessionDisconnectFrame::deserialize(deserializer)?;
        SessionDisconnectFrame::new(raw.session, raw.player_id, raw.reason)
            .map_err(serde::de::Error::custom)
    }
}

impl SessionDisconnectFrame {
    pub fn new(
        session: LinkSessionIdentity,
        player_id: PlayerId,
        reason: impl Into<String>,
    ) -> Result<Self, String> {
        let frame = Self {
            session,
            player_id,
            reason: reason.into(),
        };
        frame.validate()?;
        Ok(frame)
    }

    pub fn validate(&self) -> Result<(), String> {
        self.session.validate().map_err(|error| error.to_string())?;
        validate_disconnect_payload(self.player_id, &self.reason).map_err(|error| error.to_string())
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn new_unchecked_for_tests(
        session: LinkSessionIdentity,
        player_id: PlayerId,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            session,
            player_id,
            reason: reason.into(),
        }
    }

    pub fn session(&self) -> &LinkSessionIdentity {
        &self.session
    }

    pub fn player_id(&self) -> PlayerId {
        self.player_id
    }

    pub fn reason(&self) -> &str {
        &self.reason
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum MultiplayerMessageError {
    #[error("{field} must be non-empty")]
    EmptyText { field: &'static str },
    #[error("{field} must be exact and untrimmed")]
    InvalidText { field: &'static str },
    #[error("multiplayer player id {player_id} is not a valid link identity")]
    InvalidPlayerIdentity { player_id: PlayerId },
    #[error("{field} tile coordinates must be non-negative but got x={x}, y={y}")]
    InvalidTile { field: &'static str, x: i16, y: i16 },
    #[error("{field} source and target user ids must be different")]
    SameInteractionUser { field: &'static str },
    #[error("{field} frame must be positive but got {frame}")]
    InvalidFrame { field: &'static str, frame: u64 },
    #[error("{message_type} link message is not bound to an exact session identity")]
    MissingSessionIdentity { message_type: &'static str },
    #[error("{message_type} link message session does not match expected session: {message}")]
    SessionIdentityMismatch {
        message_type: &'static str,
        message: String,
    },
    #[error("{message}")]
    InvalidLinkHandshake { message: String },
    #[error("{message}")]
    InvalidBattleAction { message: String },
    #[error("{message}")]
    InvalidBattleRng { message: String },
    #[error("{message}")]
    InvalidTradeFrame { message: String },
    #[error("{message}")]
    InvalidLinkCableFrame { message: String },
    #[error("{message}")]
    InvalidLockstepFrame { message: String },
    #[error("{message}")]
    InvalidCommandChecksumEvent { message: String },
    #[error("{message}")]
    InvalidRuntimeCommand { message: String },
    #[error("link message payload is empty")]
    EmptyBinaryPayload,
    #[error("link message payload has invalid magic header")]
    InvalidBinaryMagic,
    #[error("link message binary format version {actual} does not match expected {expected}")]
    BinaryVersionMismatch { expected: u16, actual: u16 },
    #[error("link message payload length {actual} does not match header length {expected}")]
    BinaryLengthMismatch { expected: usize, actual: usize },
    #[error("link message payload hash {actual:08x} does not match expected {expected:08x}")]
    BinaryHashMismatch { expected: u32, actual: u32 },
    #[error("failed to encode link message: {0}")]
    BinaryEncode(String),
    #[error("failed to decode link message: {0}")]
    BinaryDecode(String),
}

fn validate_multiplayer_text(
    field: &'static str,
    value: &str,
) -> Result<(), MultiplayerMessageError> {
    if value.is_empty() {
        return Err(MultiplayerMessageError::EmptyText { field });
    }
    if !is_exact_multiplayer_text(value) {
        return Err(MultiplayerMessageError::InvalidText { field });
    }
    Ok(())
}

fn is_exact_multiplayer_text(value: &str) -> bool {
    value.trim() == value && !value.chars().any(char::is_control)
}

fn validate_multiplayer_token(
    field: &'static str,
    value: &str,
) -> Result<(), MultiplayerMessageError> {
    if value.is_empty() {
        return Err(MultiplayerMessageError::EmptyText { field });
    }
    if !is_exact_multiplayer_token(value) {
        return Err(MultiplayerMessageError::InvalidText { field });
    }
    Ok(())
}

fn validate_multiplayer_user_id(
    field: &'static str,
    value: &str,
) -> Result<(), MultiplayerMessageError> {
    if value.is_empty() {
        return Err(MultiplayerMessageError::EmptyText { field });
    }
    if !is_exact_multiplayer_user_id(value) {
        return Err(MultiplayerMessageError::InvalidText { field });
    }
    Ok(())
}

fn validate_distinct_interaction_users(
    field: &'static str,
    from_user_id: &str,
    to_user_id: &str,
) -> Result<(), MultiplayerMessageError> {
    if from_user_id == to_user_id {
        return Err(MultiplayerMessageError::SameInteractionUser { field });
    }
    Ok(())
}

fn validate_presence_tile(tile: TilePosition) -> Result<(), MultiplayerMessageError> {
    if tile.x < 0 || tile.y < 0 {
        return Err(MultiplayerMessageError::InvalidTile {
            field: "presence",
            x: tile.x,
            y: tile.y,
        });
    }
    Ok(())
}

fn validate_disconnect_payload(
    player_id: PlayerId,
    reason: &str,
) -> Result<(), MultiplayerMessageError> {
    if player_id == 0 {
        return Err(MultiplayerMessageError::InvalidPlayerIdentity { player_id });
    }
    validate_multiplayer_text("disconnect reason", reason)
}

fn validate_command_checksum_events(events: &[GameEvent]) -> Result<(), MultiplayerMessageError> {
    for (index, event) in events.iter().enumerate() {
        match event {
            GameEvent::FrameAdvanced { frame } if *frame == 0 => {
                return Err(MultiplayerMessageError::InvalidCommandChecksumEvent {
                    message: format!("command checksum event {index} advances to frame 0"),
                });
            }
            GameEvent::FrameAdvanced { .. } => {}
            GameEvent::JoypadChanged { pressed, down } => {
                validate_command_event_joypad_mask(index, "pressed", *pressed)?;
                validate_command_event_joypad_mask(index, "down", *down)?;
                if pressed & !down != 0 {
                    return Err(MultiplayerMessageError::InvalidCommandChecksumEvent {
                        message: format!(
                            "command checksum event {index} has pressed bits {pressed:#010b} outside down mask {down:#010b}"
                        ),
                    });
                }
            }
        }
    }
    Ok(())
}

fn validate_command_event_joypad_mask(
    index: usize,
    field: &str,
    mask: u8,
) -> Result<(), MultiplayerMessageError> {
    const VALID_JOYPAD_MASK: u8 = B_PAD_A
        | B_PAD_B
        | B_PAD_SELECT
        | B_PAD_START
        | B_PAD_RIGHT
        | B_PAD_LEFT
        | B_PAD_UP
        | B_PAD_DOWN;
    if mask & !VALID_JOYPAD_MASK != 0 {
        return Err(MultiplayerMessageError::InvalidCommandChecksumEvent {
            message: format!(
                "command checksum event {index} {field} mask {mask:#010b} contains invalid button bits"
            ),
        });
    }
    validate_lockstep_joypad_mask(mask).map_err(|error| {
        MultiplayerMessageError::InvalidCommandChecksumEvent {
            message: format!("command checksum event {index} {field}: {error}"),
        }
    })
}

fn is_exact_multiplayer_token(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && !has_reserved_pack_prefix(value)
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-' || byte == b':'
        })
}

fn is_exact_multiplayer_user_id(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && !has_reserved_pack_prefix(value)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BattleRngState {
    hardware_divider: u16,
    h_random_add: u8,
    h_random_sub: u8,
}

impl<'de> Deserialize<'de> for BattleRngState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawBattleRngState {
            hardware_divider: u16,
            h_random_add: u8,
            h_random_sub: u8,
        }

        let raw = RawBattleRngState::deserialize(deserializer)?;
        BattleRngState::new(raw.hardware_divider, raw.h_random_add, raw.h_random_sub)
            .map_err(serde::de::Error::custom)
    }
}

impl BattleRngState {
    pub fn new(
        hardware_divider: u16,
        h_random_add: u8,
        h_random_sub: u8,
    ) -> Result<Self, BattleRngError> {
        let state = Self {
            hardware_divider,
            h_random_add,
            h_random_sub,
        };
        state.validate()?;
        Ok(state)
    }

    pub fn from_seed(seed: u32) -> Self {
        let divider = ((seed ^ 0xa5a5) & 0xffff) as u16;
        Self {
            hardware_divider: if divider == 0 { 1 } else { divider },
            h_random_add: ((seed >> 8) & 0xff) as u8,
            h_random_sub: (seed & 0xff) as u8,
        }
    }

    pub fn validate(&self) -> Result<(), BattleRngError> {
        if self.hardware_divider == 0 {
            return Err(BattleRngError::InvalidHardwareDivider);
        }
        Ok(())
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub const fn new_unchecked_for_tests(
        hardware_divider: u16,
        h_random_add: u8,
        h_random_sub: u8,
    ) -> Self {
        Self {
            hardware_divider,
            h_random_add,
            h_random_sub,
        }
    }

    pub const fn hardware_divider(&self) -> u16 {
        self.hardware_divider
    }

    pub const fn h_random_add(&self) -> u8 {
        self.h_random_add
    }

    pub const fn h_random_sub(&self) -> u8 {
        self.h_random_sub
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum BattleRngError {
    #[error("battle rng hardware divider must be nonzero")]
    InvalidHardwareDivider,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SessionBattleRngInitFrame {
    session: LinkSessionIdentity,
    state: BattleRngState,
}

impl<'de> Deserialize<'de> for SessionBattleRngInitFrame {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawSessionBattleRngInitFrame {
            session: LinkSessionIdentity,
            state: BattleRngState,
        }

        let raw = RawSessionBattleRngInitFrame::deserialize(deserializer)?;
        SessionBattleRngInitFrame::new(raw.session, raw.state).map_err(serde::de::Error::custom)
    }
}

impl SessionBattleRngInitFrame {
    pub fn new(session: LinkSessionIdentity, state: BattleRngState) -> Result<Self, String> {
        let frame = Self { session, state };
        frame.validate()?;
        Ok(frame)
    }

    pub fn validate(&self) -> Result<(), String> {
        self.session.validate().map_err(|error| error.to_string())?;
        self.state.validate().map_err(|error| error.to_string())
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub const fn new_unchecked_for_tests(
        session: LinkSessionIdentity,
        state: BattleRngState,
    ) -> Self {
        Self { session, state }
    }

    pub const fn session(&self) -> &LinkSessionIdentity {
        &self.session
    }

    pub const fn state(&self) -> BattleRngState {
        self.state
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlayerInputFrame {
    player_id: PlayerId,
    frame: u64,
    joypad_mask: u8,
}

impl<'de> Deserialize<'de> for PlayerInputFrame {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawPlayerInputFrame {
            player_id: PlayerId,
            frame: u64,
            joypad_mask: u8,
        }

        let raw = RawPlayerInputFrame::deserialize(deserializer)?;
        let input = Self {
            player_id: raw.player_id,
            frame: raw.frame,
            joypad_mask: raw.joypad_mask,
        };
        input.validate().map_err(serde::de::Error::custom)?;
        Ok(input)
    }
}

impl PlayerInputFrame {
    pub fn new(
        player_id: PlayerId,
        frame: Frame,
        joypad_mask: u8,
    ) -> Result<Self, LockstepSyncError> {
        let input = Self {
            player_id,
            frame: frame.0,
            joypad_mask,
        };
        input.validate()?;
        Ok(input)
    }

    pub fn validate(&self) -> Result<(), LockstepSyncError> {
        if self.player_id == 0 {
            return Err(LockstepSyncError::InvalidPlayerIdentity {
                player_id: self.player_id,
            });
        }
        validate_lockstep_joypad_mask(self.joypad_mask)
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub const fn new_unchecked_for_tests(player_id: PlayerId, frame: u64, joypad_mask: u8) -> Self {
        Self {
            player_id,
            frame,
            joypad_mask,
        }
    }

    pub const fn player_id(&self) -> PlayerId {
        self.player_id
    }

    pub const fn frame(&self) -> u64 {
        self.frame
    }

    pub const fn joypad_mask(&self) -> u8 {
        self.joypad_mask
    }

    pub const fn into_parts(self) -> (PlayerId, u64, u8) {
        (self.player_id, self.frame, self.joypad_mask)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SessionPlayerInputFrame {
    session: LinkSessionIdentity,
    input: PlayerInputFrame,
}

impl<'de> Deserialize<'de> for SessionPlayerInputFrame {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawSessionPlayerInputFrame {
            session: LinkSessionIdentity,
            input: PlayerInputFrame,
        }

        let raw = RawSessionPlayerInputFrame::deserialize(deserializer)?;
        SessionPlayerInputFrame::new(raw.session, raw.input).map_err(serde::de::Error::custom)
    }
}

impl SessionPlayerInputFrame {
    pub fn new(session: LinkSessionIdentity, input: PlayerInputFrame) -> Result<Self, String> {
        let frame = Self { session, input };
        frame.validate()?;
        Ok(frame)
    }

    pub fn validate(&self) -> Result<(), String> {
        self.session.validate().map_err(|error| error.to_string())?;
        self.input.validate().map_err(|error| error.to_string())
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub const fn new_unchecked_for_tests(
        session: LinkSessionIdentity,
        input: PlayerInputFrame,
    ) -> Self {
        Self { session, input }
    }

    pub const fn session(&self) -> &LinkSessionIdentity {
        &self.session
    }

    pub const fn input(&self) -> &PlayerInputFrame {
        &self.input
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MenuChoiceFrame {
    player_id: PlayerId,
    frame: u64,
    menu_id: String,
    option_index: usize,
    verticalmenu_command_index: usize,
}

impl<'de> Deserialize<'de> for MenuChoiceFrame {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawMenuChoiceFrame {
            player_id: PlayerId,
            frame: u64,
            menu_id: String,
            option_index: usize,
            verticalmenu_command_index: usize,
        }

        let raw = RawMenuChoiceFrame::deserialize(deserializer)?;
        let choice = Self {
            player_id: raw.player_id,
            frame: raw.frame,
            menu_id: raw.menu_id,
            option_index: raw.option_index,
            verticalmenu_command_index: raw.verticalmenu_command_index,
        };
        choice.validate().map_err(serde::de::Error::custom)?;
        Ok(choice)
    }
}

impl MenuChoiceFrame {
    pub fn new(
        player_id: PlayerId,
        frame: Frame,
        menu_id: impl Into<String>,
        option_index: usize,
        verticalmenu_command_index: usize,
    ) -> Result<Self, MultiplayerMessageError> {
        let choice = Self {
            player_id,
            frame: frame.0,
            menu_id: menu_id.into(),
            option_index,
            verticalmenu_command_index,
        };
        choice.validate()?;
        Ok(choice)
    }

    pub fn validate(&self) -> Result<(), MultiplayerMessageError> {
        if self.player_id == 0 {
            return Err(MultiplayerMessageError::InvalidPlayerIdentity {
                player_id: self.player_id,
            });
        }
        if self.frame == 0 {
            return Err(MultiplayerMessageError::InvalidFrame {
                field: "menu_choice.frame",
                frame: self.frame,
            });
        }
        validate_multiplayer_token("menu_choice.menu_id", &self.menu_id)?;
        Ok(())
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn new_unchecked_for_tests(
        player_id: PlayerId,
        frame: u64,
        menu_id: impl Into<String>,
        option_index: usize,
        verticalmenu_command_index: usize,
    ) -> Self {
        Self {
            player_id,
            frame,
            menu_id: menu_id.into(),
            option_index,
            verticalmenu_command_index,
        }
    }

    pub const fn player_id(&self) -> PlayerId {
        self.player_id
    }

    pub const fn frame(&self) -> u64 {
        self.frame
    }

    pub fn menu_id(&self) -> &str {
        &self.menu_id
    }

    pub const fn option_index(&self) -> usize {
        self.option_index
    }

    pub const fn verticalmenu_command_index(&self) -> usize {
        self.verticalmenu_command_index
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MenuChoiceResultFrame {
    choice: MenuChoiceFrame,
    checksum: StateChecksumFrame,
    script_value: String,
}

impl<'de> Deserialize<'de> for MenuChoiceResultFrame {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawMenuChoiceResultFrame {
            choice: MenuChoiceFrame,
            checksum: StateChecksumFrame,
            script_value: String,
        }

        let raw = RawMenuChoiceResultFrame::deserialize(deserializer)?;
        MenuChoiceResultFrame::new(raw.choice, raw.checksum, raw.script_value)
            .map_err(serde::de::Error::custom)
    }
}

impl MenuChoiceResultFrame {
    pub fn new(
        choice: MenuChoiceFrame,
        checksum: StateChecksumFrame,
        script_value: impl Into<String>,
    ) -> Result<Self, MenuChoiceResultFrameError> {
        let result = Self {
            choice,
            checksum,
            script_value: script_value.into(),
        };
        result.validate()?;
        Ok(result)
    }

    pub fn validate(&self) -> Result<(), MenuChoiceResultFrameError> {
        self.choice
            .validate()
            .map_err(|error| MenuChoiceResultFrameError::InvalidChoice {
                message: error.to_string(),
            })?;
        self.checksum
            .validate()
            .map_err(|error| MenuChoiceResultFrameError::InvalidChecksum {
                message: error.to_string(),
            })?;
        if self.choice.player_id() != self.checksum.player_id() {
            return Err(MenuChoiceResultFrameError::PlayerChecksumMismatch {
                choice_player_id: self.choice.player_id(),
                checksum_player_id: self.checksum.player_id(),
            });
        }
        if self.checksum.frame() < self.choice.frame() {
            return Err(MenuChoiceResultFrameError::ResultBeforeChoice {
                choice_frame: self.choice.frame(),
                checksum_frame: self.checksum.frame(),
            });
        }
        validate_multiplayer_text("menu_choice_result.script_value", &self.script_value).map_err(
            |error| MenuChoiceResultFrameError::InvalidScriptValue {
                message: error.to_string(),
            },
        )?;
        Ok(())
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn new_unchecked_for_tests(
        choice: MenuChoiceFrame,
        checksum: StateChecksumFrame,
        script_value: impl Into<String>,
    ) -> Self {
        Self {
            choice,
            checksum,
            script_value: script_value.into(),
        }
    }

    pub const fn choice(&self) -> &MenuChoiceFrame {
        &self.choice
    }

    pub const fn checksum(&self) -> &StateChecksumFrame {
        &self.checksum
    }

    pub fn script_value(&self) -> &str {
        &self.script_value
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SessionMenuChoiceFrame {
    session: LinkSessionIdentity,
    choice: MenuChoiceFrame,
}

impl<'de> Deserialize<'de> for SessionMenuChoiceFrame {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawSessionMenuChoiceFrame {
            session: LinkSessionIdentity,
            choice: MenuChoiceFrame,
        }

        let raw = RawSessionMenuChoiceFrame::deserialize(deserializer)?;
        SessionMenuChoiceFrame::new(raw.session, raw.choice).map_err(serde::de::Error::custom)
    }
}

impl SessionMenuChoiceFrame {
    pub fn new(session: LinkSessionIdentity, choice: MenuChoiceFrame) -> Result<Self, String> {
        let frame = Self { session, choice };
        frame.validate()?;
        Ok(frame)
    }

    pub fn validate(&self) -> Result<(), String> {
        self.session.validate().map_err(|error| error.to_string())?;
        self.choice.validate().map_err(|error| error.to_string())
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn new_unchecked_for_tests(session: LinkSessionIdentity, choice: MenuChoiceFrame) -> Self {
        Self { session, choice }
    }

    pub const fn session(&self) -> &LinkSessionIdentity {
        &self.session
    }

    pub const fn choice(&self) -> &MenuChoiceFrame {
        &self.choice
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SessionMenuChoiceResultFrame {
    session: LinkSessionIdentity,
    result: MenuChoiceResultFrame,
}

impl<'de> Deserialize<'de> for SessionMenuChoiceResultFrame {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawSessionMenuChoiceResultFrame {
            session: LinkSessionIdentity,
            result: MenuChoiceResultFrame,
        }

        let raw = RawSessionMenuChoiceResultFrame::deserialize(deserializer)?;
        SessionMenuChoiceResultFrame::new(raw.session, raw.result).map_err(serde::de::Error::custom)
    }
}

impl SessionMenuChoiceResultFrame {
    pub fn new(
        session: LinkSessionIdentity,
        result: MenuChoiceResultFrame,
    ) -> Result<Self, String> {
        let frame = Self { session, result };
        frame.validate()?;
        Ok(frame)
    }

    pub fn validate(&self) -> Result<(), String> {
        self.session.validate().map_err(|error| error.to_string())?;
        self.result.validate().map_err(|error| error.to_string())
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn new_unchecked_for_tests(
        session: LinkSessionIdentity,
        result: MenuChoiceResultFrame,
    ) -> Self {
        Self { session, result }
    }

    pub const fn session(&self) -> &LinkSessionIdentity {
        &self.session
    }

    pub const fn result(&self) -> &MenuChoiceResultFrame {
        &self.result
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum MenuChoiceResultFrameError {
    #[error("menu choice result choice is invalid: {message}")]
    InvalidChoice { message: String },
    #[error("menu choice result checksum is invalid: {message}")]
    InvalidChecksum { message: String },
    #[error(
        "menu choice result player {choice_player_id} does not match checksum player {checksum_player_id}"
    )]
    PlayerChecksumMismatch {
        choice_player_id: PlayerId,
        checksum_player_id: PlayerId,
    },
    #[error(
        "menu choice result checksum frame {checksum_frame} is before choice frame {choice_frame}"
    )]
    ResultBeforeChoice {
        choice_frame: u64,
        checksum_frame: u64,
    },
    #[error("menu choice result script value is invalid: {message}")]
    InvalidScriptValue { message: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StateChecksum {
    frame: u64,
    hash: u32,
}

impl StateChecksum {
    pub const fn new(frame: u64, hash: u32) -> Self {
        Self { frame, hash }
    }

    pub const fn frame(&self) -> u64 {
        self.frame
    }

    pub const fn hash(&self) -> u32 {
        self.hash
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StateChecksumFrame {
    player_id: PlayerId,
    frame: u64,
    hash: u32,
}

impl<'de> Deserialize<'de> for StateChecksumFrame {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawStateChecksumFrame {
            player_id: PlayerId,
            frame: u64,
            hash: u32,
        }

        let raw = RawStateChecksumFrame::deserialize(deserializer)?;
        let frame = Self {
            player_id: raw.player_id,
            frame: raw.frame,
            hash: raw.hash,
        };
        frame.validate().map_err(serde::de::Error::custom)?;
        Ok(frame)
    }
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
        StateChecksum::new(self.frame, self.hash)
    }

    pub fn validate(&self) -> Result<(), LockstepSyncError> {
        if self.player_id == 0 {
            return Err(LockstepSyncError::InvalidPlayerIdentity {
                player_id: self.player_id,
            });
        }
        Ok(())
    }

    pub fn from_game_state(
        player_id: PlayerId,
        state: &GameState,
    ) -> Result<Self, StateChecksumError> {
        if player_id == 0 {
            return Err(StateChecksumError::InvalidPlayerIdentity { player_id });
        }
        let checksum = game_state_checksum(state)?;
        Ok(Self {
            player_id,
            frame: checksum.frame(),
            hash: checksum.hash(),
        })
    }

    pub const fn player_id(&self) -> PlayerId {
        self.player_id
    }

    pub const fn frame(&self) -> u64 {
        self.frame
    }

    pub const fn hash(&self) -> u32 {
        self.hash
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SessionStateChecksumFrame {
    session: LinkSessionIdentity,
    checksum: StateChecksumFrame,
}

impl<'de> Deserialize<'de> for SessionStateChecksumFrame {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawSessionStateChecksumFrame {
            session: LinkSessionIdentity,
            checksum: StateChecksumFrame,
        }

        let raw = RawSessionStateChecksumFrame::deserialize(deserializer)?;
        SessionStateChecksumFrame::new(raw.session, raw.checksum).map_err(serde::de::Error::custom)
    }
}

impl SessionStateChecksumFrame {
    pub fn new(session: LinkSessionIdentity, checksum: StateChecksumFrame) -> Result<Self, String> {
        let frame = Self { session, checksum };
        frame.validate()?;
        Ok(frame)
    }

    pub fn validate(&self) -> Result<(), String> {
        self.session.validate().map_err(|error| error.to_string())?;
        self.checksum.validate().map_err(|error| error.to_string())
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn new_unchecked_for_tests(
        session: LinkSessionIdentity,
        checksum: StateChecksumFrame,
    ) -> Self {
        Self { session, checksum }
    }

    pub const fn session(&self) -> &LinkSessionIdentity {
        &self.session
    }

    pub const fn checksum(&self) -> &StateChecksumFrame {
        &self.checksum
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum StateChecksumError {
    #[error("state checksum player id {player_id} is not a valid link identity")]
    InvalidPlayerIdentity { player_id: PlayerId },
    #[error("state checksum requires valid saved state: {0}")]
    InvalidState(String),
    #[error("failed to encode GameState for deterministic checksum: {0}")]
    Encode(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CommandChecksumResult {
    pub events: Vec<GameEvent>,
    pub checksum: StateChecksumFrame,
}

impl<'de> Deserialize<'de> for CommandChecksumResult {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawCommandChecksumResult {
            events: Vec<GameEvent>,
            checksum: StateChecksumFrame,
        }

        let raw = RawCommandChecksumResult::deserialize(deserializer)?;
        validate_command_checksum_events(&raw.events).map_err(serde::de::Error::custom)?;
        raw.checksum.validate().map_err(serde::de::Error::custom)?;
        Ok(Self {
            events: raw.events,
            checksum: raw.checksum,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeCommandPayload {
    schema: String,
    bytes: Vec<u8>,
    hash: u32,
}

impl<'de> Deserialize<'de> for RuntimeCommandPayload {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawRuntimeCommandPayload {
            schema: String,
            bytes: Vec<u8>,
            hash: u32,
        }

        let raw = RawRuntimeCommandPayload::deserialize(deserializer)?;
        let payload = Self {
            schema: raw.schema,
            bytes: raw.bytes,
            hash: raw.hash,
        };
        payload.validate().map_err(serde::de::Error::custom)?;
        Ok(payload)
    }
}

impl RuntimeCommandPayload {
    pub fn new(
        schema: impl Into<String>,
        bytes: Vec<u8>,
    ) -> Result<Self, RuntimeCommandFrameError> {
        let payload = Self {
            schema: schema.into(),
            hash: fnv1a32_bytes(&bytes),
            bytes,
        };
        payload.validate()?;
        Ok(payload)
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn new_unchecked_for_tests(schema: impl Into<String>, bytes: Vec<u8>, hash: u32) -> Self {
        Self {
            schema: schema.into(),
            bytes,
            hash,
        }
    }

    pub fn validate(&self) -> Result<(), RuntimeCommandFrameError> {
        validate_runtime_command_token("runtime command payload schema", &self.schema)?;
        if self.bytes.is_empty() {
            return Err(RuntimeCommandFrameError::EmptyPayload);
        }
        let actual = fnv1a32_bytes(&self.bytes);
        if self.hash != actual {
            return Err(RuntimeCommandFrameError::PayloadHashMismatch {
                expected: self.hash,
                actual,
            });
        }
        Ok(())
    }

    pub fn schema(&self) -> &str {
        &self.schema
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub const fn hash(&self) -> u32 {
        self.hash
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeCommandFrame {
    player_id: PlayerId,
    sequence: u64,
    payload: RuntimeCommandPayload,
    expected_state: StateChecksum,
}

impl<'de> Deserialize<'de> for RuntimeCommandFrame {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawRuntimeCommandFrame {
            player_id: PlayerId,
            sequence: u64,
            payload: RuntimeCommandPayload,
            expected_state: StateChecksum,
        }

        let raw = RawRuntimeCommandFrame::deserialize(deserializer)?;
        RuntimeCommandFrame::new(raw.player_id, raw.sequence, raw.payload, raw.expected_state)
            .map_err(serde::de::Error::custom)
    }
}

impl RuntimeCommandFrame {
    pub fn new(
        player_id: PlayerId,
        sequence: u64,
        payload: RuntimeCommandPayload,
        expected_state: StateChecksum,
    ) -> Result<Self, RuntimeCommandFrameError> {
        let frame = Self {
            player_id,
            sequence,
            payload,
            expected_state,
        };
        frame.validate()?;
        Ok(frame)
    }

    pub fn validate(&self) -> Result<(), RuntimeCommandFrameError> {
        if self.player_id == 0 {
            return Err(RuntimeCommandFrameError::InvalidPlayerIdentity {
                player_id: self.player_id,
            });
        }
        if self.sequence == 0 {
            return Err(RuntimeCommandFrameError::InvalidSequence {
                sequence: self.sequence,
            });
        }
        self.payload.validate()?;
        Ok(())
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn new_unchecked_for_tests(
        player_id: PlayerId,
        sequence: u64,
        payload: RuntimeCommandPayload,
        expected_state: StateChecksum,
    ) -> Self {
        Self {
            player_id,
            sequence,
            payload,
            expected_state,
        }
    }

    pub const fn player_id(&self) -> PlayerId {
        self.player_id
    }

    pub const fn sequence(&self) -> u64 {
        self.sequence
    }

    pub const fn payload(&self) -> &RuntimeCommandPayload {
        &self.payload
    }

    pub const fn expected_state(&self) -> &StateChecksum {
        &self.expected_state
    }

    pub fn require_expected_state(
        &self,
        actual: &StateChecksum,
    ) -> Result<(), RuntimeCommandFrameError> {
        if &self.expected_state != actual {
            return Err(RuntimeCommandFrameError::ExpectedStateMismatch {
                expected_frame: self.expected_state.frame(),
                expected_hash: self.expected_state.hash(),
                actual_frame: actual.frame(),
                actual_hash: actual.hash(),
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeCommandResultFrame {
    request: RuntimeCommandFrame,
    checksum: StateChecksumFrame,
    result_tag: String,
}

impl<'de> Deserialize<'de> for RuntimeCommandResultFrame {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawRuntimeCommandResultFrame {
            request: RuntimeCommandFrame,
            checksum: StateChecksumFrame,
            result_tag: String,
        }

        let raw = RawRuntimeCommandResultFrame::deserialize(deserializer)?;
        RuntimeCommandResultFrame::new(raw.request, raw.checksum, raw.result_tag)
            .map_err(serde::de::Error::custom)
    }
}

impl RuntimeCommandResultFrame {
    pub fn new(
        request: RuntimeCommandFrame,
        checksum: StateChecksumFrame,
        result_tag: impl Into<String>,
    ) -> Result<Self, RuntimeCommandFrameError> {
        let frame = Self {
            request,
            checksum,
            result_tag: result_tag.into(),
        };
        frame.validate()?;
        Ok(frame)
    }

    pub fn validate(&self) -> Result<(), RuntimeCommandFrameError> {
        self.request.validate()?;
        self.checksum
            .validate()
            .map_err(|error| RuntimeCommandFrameError::InvalidChecksum {
                message: error.to_string(),
            })?;
        if self.request.player_id() != self.checksum.player_id() {
            return Err(RuntimeCommandFrameError::PlayerChecksumMismatch {
                request_player_id: self.request.player_id(),
                checksum_player_id: self.checksum.player_id(),
            });
        }
        if self.checksum.frame() < self.request.expected_state().frame() {
            return Err(RuntimeCommandFrameError::ResultBeforeExpectedState {
                expected_frame: self.request.expected_state().frame(),
                result_frame: self.checksum.frame(),
            });
        }
        validate_runtime_command_token("runtime command result tag", &self.result_tag)?;
        Ok(())
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn new_unchecked_for_tests(
        request: RuntimeCommandFrame,
        checksum: StateChecksumFrame,
        result_tag: impl Into<String>,
    ) -> Self {
        Self {
            request,
            checksum,
            result_tag: result_tag.into(),
        }
    }

    pub const fn request(&self) -> &RuntimeCommandFrame {
        &self.request
    }

    pub const fn checksum(&self) -> &StateChecksumFrame {
        &self.checksum
    }

    pub fn result_tag(&self) -> &str {
        &self.result_tag
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SessionRuntimeCommandFrame {
    session: LinkSessionIdentity,
    command: RuntimeCommandFrame,
}

impl<'de> Deserialize<'de> for SessionRuntimeCommandFrame {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawSessionRuntimeCommandFrame {
            session: LinkSessionIdentity,
            command: RuntimeCommandFrame,
        }

        let raw = RawSessionRuntimeCommandFrame::deserialize(deserializer)?;
        SessionRuntimeCommandFrame::new(raw.session, raw.command).map_err(serde::de::Error::custom)
    }
}

impl SessionRuntimeCommandFrame {
    pub fn new(
        session: LinkSessionIdentity,
        command: RuntimeCommandFrame,
    ) -> Result<Self, RuntimeCommandFrameError> {
        let frame = Self { session, command };
        frame.validate()?;
        Ok(frame)
    }

    pub fn validate(&self) -> Result<(), RuntimeCommandFrameError> {
        self.session
            .validate()
            .map_err(|error| RuntimeCommandFrameError::InvalidSession {
                message: error.to_string(),
            })?;
        self.command.validate()
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn new_unchecked_for_tests(
        session: LinkSessionIdentity,
        command: RuntimeCommandFrame,
    ) -> Self {
        Self { session, command }
    }

    pub const fn session(&self) -> &LinkSessionIdentity {
        &self.session
    }

    pub const fn command(&self) -> &RuntimeCommandFrame {
        &self.command
    }

    pub fn into_command(self) -> RuntimeCommandFrame {
        self.command
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SessionRuntimeCommandResultFrame {
    session: LinkSessionIdentity,
    result: RuntimeCommandResultFrame,
}

impl<'de> Deserialize<'de> for SessionRuntimeCommandResultFrame {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawSessionRuntimeCommandResultFrame {
            session: LinkSessionIdentity,
            result: RuntimeCommandResultFrame,
        }

        let raw = RawSessionRuntimeCommandResultFrame::deserialize(deserializer)?;
        SessionRuntimeCommandResultFrame::new(raw.session, raw.result)
            .map_err(serde::de::Error::custom)
    }
}

impl SessionRuntimeCommandResultFrame {
    pub fn new(
        session: LinkSessionIdentity,
        result: RuntimeCommandResultFrame,
    ) -> Result<Self, RuntimeCommandFrameError> {
        let frame = Self { session, result };
        frame.validate()?;
        Ok(frame)
    }

    pub fn validate(&self) -> Result<(), RuntimeCommandFrameError> {
        self.session
            .validate()
            .map_err(|error| RuntimeCommandFrameError::InvalidSession {
                message: error.to_string(),
            })?;
        self.result.validate()
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn new_unchecked_for_tests(
        session: LinkSessionIdentity,
        result: RuntimeCommandResultFrame,
    ) -> Self {
        Self { session, result }
    }

    pub const fn session(&self) -> &LinkSessionIdentity {
        &self.session
    }

    pub const fn result(&self) -> &RuntimeCommandResultFrame {
        &self.result
    }

    pub fn into_result(self) -> RuntimeCommandResultFrame {
        self.result
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum RuntimeCommandFrameError {
    #[error("runtime command player id {player_id} is not a valid link identity")]
    InvalidPlayerIdentity { player_id: PlayerId },
    #[error("runtime command sequence {sequence} must be positive")]
    InvalidSequence { sequence: u64 },
    #[error("runtime command session is invalid: {message}")]
    InvalidSession { message: String },
    #[error("{field} must be exact and non-empty")]
    InvalidToken { field: &'static str },
    #[error("runtime command payload must be non-empty")]
    EmptyPayload,
    #[error("runtime command payload hash {actual:#010x} does not match declared {expected:#010x}")]
    PayloadHashMismatch { expected: u32, actual: u32 },
    #[error("runtime command checksum is invalid: {message}")]
    InvalidChecksum { message: String },
    #[error(
        "runtime command result player {request_player_id} does not match checksum player {checksum_player_id}"
    )]
    PlayerChecksumMismatch {
        request_player_id: PlayerId,
        checksum_player_id: PlayerId,
    },
    #[error(
        "runtime command expected state frame {expected_frame} hash {expected_hash:#010x} does not match actual frame {actual_frame} hash {actual_hash:#010x}"
    )]
    ExpectedStateMismatch {
        expected_frame: u64,
        expected_hash: u32,
        actual_frame: u64,
        actual_hash: u32,
    },
    #[error(
        "runtime command result frame {result_frame} is before expected state frame {expected_frame}"
    )]
    ResultBeforeExpectedState {
        expected_frame: u64,
        result_frame: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SessionSaveSummaryFrame {
    session: LinkSessionIdentity,
    summary: SaveGameSummary,
}

impl<'de> Deserialize<'de> for SessionSaveSummaryFrame {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawSessionSaveSummaryFrame {
            session: LinkSessionIdentity,
            summary: SaveGameSummary,
        }

        let raw = RawSessionSaveSummaryFrame::deserialize(deserializer)?;
        SessionSaveSummaryFrame::new(raw.session, raw.summary).map_err(serde::de::Error::custom)
    }
}

impl SessionSaveSummaryFrame {
    pub fn new(
        session: LinkSessionIdentity,
        summary: SaveGameSummary,
    ) -> Result<Self, SessionSaveSummaryFrameError> {
        let frame = Self { session, summary };
        frame.validate()?;
        Ok(frame)
    }

    pub fn validate(&self) -> Result<(), SessionSaveSummaryFrameError> {
        self.session
            .validate()
            .map_err(|error| SessionSaveSummaryFrameError::InvalidSession {
                message: error.to_string(),
            })?;
        self.summary
            .validate()
            .map_err(|error| SessionSaveSummaryFrameError::InvalidSummary {
                message: error.to_string(),
            })?;
        if self.summary.modpack().id() != self.session.modpack().id() {
            return Err(SessionSaveSummaryFrameError::ModpackIdMismatch {
                expected: self.session.modpack().id().to_string(),
                actual: self.summary.modpack().id().to_string(),
            });
        }
        if self.summary.modpack().hash() != self.session.modpack().hash() {
            return Err(SessionSaveSummaryFrameError::ModpackHashMismatch {
                expected: self.session.modpack().hash().to_string(),
                actual: self.summary.modpack().hash().to_string(),
            });
        }
        if self.summary.pack_content_hash() != self.session.pack_content_hash() {
            return Err(SessionSaveSummaryFrameError::PackContentHashMismatch {
                expected: self.session.pack_content_hash().to_string(),
                actual: self.summary.pack_content_hash().to_string(),
            });
        }
        Ok(())
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn new_unchecked_for_tests(session: LinkSessionIdentity, summary: SaveGameSummary) -> Self {
        Self { session, summary }
    }

    pub const fn session(&self) -> &LinkSessionIdentity {
        &self.session
    }

    pub const fn summary(&self) -> &SaveGameSummary {
        &self.summary
    }

    pub fn into_summary(self) -> SaveGameSummary {
        self.summary
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum SessionSaveSummaryFrameError {
    #[error("save summary session is invalid: {message}")]
    InvalidSession { message: String },
    #[error("save summary is invalid: {message}")]
    InvalidSummary { message: String },
    #[error("save summary modpack id {actual} does not match session modpack id {expected}")]
    ModpackIdMismatch { expected: String, actual: String },
    #[error("save summary modpack hash {actual} does not match session modpack hash {expected}")]
    ModpackHashMismatch { expected: String, actual: String },
    #[error(
        "save summary pack content hash {actual} does not match session pack content hash {expected}"
    )]
    PackContentHashMismatch { expected: String, actual: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SaveCheckpointFrame {
    summary: SaveGameSummary,
    checksum: StateChecksumFrame,
}

impl<'de> Deserialize<'de> for SaveCheckpointFrame {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawSaveCheckpointFrame {
            summary: SaveGameSummary,
            checksum: StateChecksumFrame,
        }

        let raw = RawSaveCheckpointFrame::deserialize(deserializer)?;
        SaveCheckpointFrame::new(raw.summary, raw.checksum).map_err(serde::de::Error::custom)
    }
}

impl SaveCheckpointFrame {
    pub fn new(
        summary: SaveGameSummary,
        checksum: StateChecksumFrame,
    ) -> Result<Self, SaveCheckpointFrameError> {
        let frame = Self { summary, checksum };
        frame.validate()?;
        Ok(frame)
    }

    pub fn validate(&self) -> Result<(), SaveCheckpointFrameError> {
        self.summary
            .validate()
            .map_err(|error| SaveCheckpointFrameError::InvalidSummary {
                message: error.to_string(),
            })?;
        self.checksum
            .validate()
            .map_err(|error| SaveCheckpointFrameError::InvalidChecksum {
                message: error.to_string(),
            })?;
        if self.summary.state_frame() != self.checksum.frame() {
            return Err(SaveCheckpointFrameError::FrameMismatch {
                summary_frame: self.summary.state_frame(),
                checksum_frame: self.checksum.frame(),
            });
        }
        if self.summary.state_hash() != self.checksum.hash() {
            return Err(SaveCheckpointFrameError::HashMismatch {
                summary_hash: self.summary.state_hash(),
                checksum_hash: self.checksum.hash(),
            });
        }
        Ok(())
    }

    pub fn validate_for_players(
        &self,
        players: impl IntoIterator<Item = PlayerId>,
    ) -> Result<(), SaveCheckpointFrameError> {
        self.validate()?;
        let player_id = self.checksum.player_id();
        if !players.into_iter().any(|candidate| candidate == player_id) {
            return Err(SaveCheckpointFrameError::UnknownPlayer { player_id });
        }
        Ok(())
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn new_unchecked_for_tests(summary: SaveGameSummary, checksum: StateChecksumFrame) -> Self {
        Self { summary, checksum }
    }

    pub const fn summary(&self) -> &SaveGameSummary {
        &self.summary
    }

    pub const fn checksum(&self) -> &StateChecksumFrame {
        &self.checksum
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SessionSaveCheckpointFrame {
    session: LinkSessionIdentity,
    checkpoint: SaveCheckpointFrame,
}

impl<'de> Deserialize<'de> for SessionSaveCheckpointFrame {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawSessionSaveCheckpointFrame {
            session: LinkSessionIdentity,
            checkpoint: SaveCheckpointFrame,
        }

        let raw = RawSessionSaveCheckpointFrame::deserialize(deserializer)?;
        SessionSaveCheckpointFrame::new(raw.session, raw.checkpoint)
            .map_err(serde::de::Error::custom)
    }
}

impl SessionSaveCheckpointFrame {
    pub fn new(
        session: LinkSessionIdentity,
        checkpoint: SaveCheckpointFrame,
    ) -> Result<Self, SaveCheckpointFrameError> {
        let frame = Self {
            session,
            checkpoint,
        };
        frame.validate()?;
        Ok(frame)
    }

    pub fn validate(&self) -> Result<(), SaveCheckpointFrameError> {
        let summary =
            SessionSaveSummaryFrame::new(self.session.clone(), self.checkpoint.summary().clone())
                .map_err(|error| SaveCheckpointFrameError::InvalidSessionSummary {
                message: error.to_string(),
            })?;
        summary
            .validate()
            .map_err(|error| SaveCheckpointFrameError::InvalidSessionSummary {
                message: error.to_string(),
            })?;
        self.checkpoint.validate()
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn new_unchecked_for_tests(
        session: LinkSessionIdentity,
        checkpoint: SaveCheckpointFrame,
    ) -> Self {
        Self {
            session,
            checkpoint,
        }
    }

    pub const fn session(&self) -> &LinkSessionIdentity {
        &self.session
    }

    pub const fn checkpoint(&self) -> &SaveCheckpointFrame {
        &self.checkpoint
    }

    pub fn into_checkpoint(self) -> SaveCheckpointFrame {
        self.checkpoint
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum SaveCheckpointFrameError {
    #[error("save checkpoint summary is invalid: {message}")]
    InvalidSummary { message: String },
    #[error("save checkpoint checksum is invalid: {message}")]
    InvalidChecksum { message: String },
    #[error("save checkpoint session summary is invalid: {message}")]
    InvalidSessionSummary { message: String },
    #[error(
        "save checkpoint summary frame {summary_frame} does not match checksum frame {checksum_frame}"
    )]
    FrameMismatch {
        summary_frame: u64,
        checksum_frame: u64,
    },
    #[error(
        "save checkpoint summary hash {summary_hash:#010x} does not match checksum hash {checksum_hash:#010x}"
    )]
    HashMismatch {
        summary_hash: u32,
        checksum_hash: u32,
    },
    #[error("save checkpoint player {player_id} is not in the declared link roster")]
    UnknownPlayer { player_id: PlayerId },
}

fn validate_runtime_command_token(
    field: &'static str,
    value: &str,
) -> Result<(), RuntimeCommandFrameError> {
    if !is_exact_multiplayer_token(value) {
        return Err(RuntimeCommandFrameError::InvalidToken { field });
    }
    Ok(())
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CommandChecksumError {
    #[error(transparent)]
    Frame(#[from] GameStateFrameError),
    #[error(transparent)]
    Checksum(#[from] StateChecksumError),
}

pub fn game_state_checksum(state: &GameState) -> Result<StateChecksum, StateChecksumError> {
    state
        .validate_saved_state()
        .map_err(StateChecksumError::InvalidState)?;
    game_state_checksum_unchecked(state)
}

/// Compute the authoritative state checksum without re-validating the whole
/// save graph.
///
/// Runtime mutation code has already validated the loaded state and applies
/// only typed mutations that preserve those invariants. Keeping this split
/// lets the fixed-rate frame loop retain its checksum contract without walking
/// every party, box, map override, script queue, and catalog reference on
/// every frame.
pub fn game_state_checksum_unchecked(
    state: &GameState,
) -> Result<StateChecksum, StateChecksumError> {
    let bytes = bincode::serde::encode_to_vec(state, state_checksum_binary_config())
        .map_err(|error| StateChecksumError::Encode(error.to_string()))?;
    Ok(StateChecksum::new(
        state.frame_counter,
        fnv1a32_bytes(&bytes),
    ))
}

pub fn apply_command_with_checksum(
    state: &mut GameState,
    player_id: PlayerId,
    command: GameCommand,
) -> Result<CommandChecksumResult, CommandChecksumError> {
    if player_id == 0 {
        return Err(CommandChecksumError::Checksum(
            StateChecksumError::InvalidPlayerIdentity { player_id },
        ));
    }
    let events = state.apply_command(command)?;
    let checksum = StateChecksumFrame::from_game_state(player_id, state)?;
    Ok(CommandChecksumResult { events, checksum })
}

fn state_checksum_binary_config() -> impl bincode::config::Config {
    bincode::config::standard()
        .with_little_endian()
        .with_fixed_int_encoding()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BattleActionFrame {
    player_id: PlayerId,
    turn: u64,
    action: BattleAction,
    state_hash: String,
}

impl<'de> Deserialize<'de> for BattleActionFrame {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawBattleActionFrame {
            player_id: PlayerId,
            turn: u64,
            action: BattleAction,
            state_hash: String,
        }

        let raw = RawBattleActionFrame::deserialize(deserializer)?;
        BattleActionFrame::new(raw.player_id, raw.turn, raw.action, raw.state_hash)
            .map_err(serde::de::Error::custom)
    }
}

impl BattleActionFrame {
    pub fn new(
        player_id: PlayerId,
        turn: u64,
        action: BattleAction,
        state_hash: impl Into<String>,
    ) -> Result<Self, BattleSyncError> {
        let state_hash = state_hash.into();
        let frame = Self {
            player_id,
            turn,
            action,
            state_hash,
        };
        frame.validate()?;
        Ok(frame)
    }

    pub fn with_state_hash(
        player_id: PlayerId,
        turn: u64,
        action: BattleAction,
        state_hash: impl Into<String>,
    ) -> Result<Self, BattleSyncError> {
        Self::new(player_id, turn, action, state_hash)
    }

    pub fn validate(&self) -> Result<(), BattleSyncError> {
        if self.player_id == 0 {
            return Err(BattleSyncError::InvalidPlayerIdentity {
                player_id: self.player_id,
            });
        }
        validate_battle_action(&self.action)?;
        if self.state_hash.is_empty() {
            return Err(BattleSyncError::EmptyStateHash);
        }
        if !is_exact_state_hash(&self.state_hash) {
            return Err(BattleSyncError::InvalidStateHash {
                state_hash: self.state_hash.clone(),
            });
        }
        Ok(())
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn new_unchecked_for_tests(
        player_id: PlayerId,
        turn: u64,
        action: BattleAction,
        state_hash: String,
    ) -> Self {
        Self {
            player_id,
            turn,
            action,
            state_hash,
        }
    }

    pub const fn player_id(&self) -> PlayerId {
        self.player_id
    }

    pub const fn turn(&self) -> u64 {
        self.turn
    }

    pub const fn action(&self) -> &BattleAction {
        &self.action
    }

    pub fn state_hash(&self) -> &str {
        self.state_hash.as_str()
    }

    pub fn into_parts(self) -> (PlayerId, u64, BattleAction, String) {
        (self.player_id, self.turn, self.action, self.state_hash)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SessionBattleActionFrame {
    session: LinkSessionIdentity,
    action: BattleActionFrame,
}

impl<'de> Deserialize<'de> for SessionBattleActionFrame {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawSessionBattleActionFrame {
            session: LinkSessionIdentity,
            action: BattleActionFrame,
        }

        let raw = RawSessionBattleActionFrame::deserialize(deserializer)?;
        SessionBattleActionFrame::new(raw.session, raw.action).map_err(serde::de::Error::custom)
    }
}

impl SessionBattleActionFrame {
    pub fn new(session: LinkSessionIdentity, action: BattleActionFrame) -> Result<Self, String> {
        let frame = Self { session, action };
        frame.validate()?;
        Ok(frame)
    }

    pub fn validate(&self) -> Result<(), String> {
        self.session.validate().map_err(|error| error.to_string())?;
        self.action.validate().map_err(|error| error.to_string())
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn new_unchecked_for_tests(
        session: LinkSessionIdentity,
        action: BattleActionFrame,
    ) -> Self {
        Self { session, action }
    }

    pub const fn session(&self) -> &LinkSessionIdentity {
        &self.session
    }

    pub const fn action(&self) -> &BattleActionFrame {
        &self.action
    }
}

fn validate_battle_action(action: &BattleAction) -> Result<(), BattleSyncError> {
    match action {
        BattleAction::Move { slot } => {
            if *slot >= BATTLE_MOVE_SLOTS {
                return Err(BattleSyncError::InvalidMoveSlot { slot: *slot });
            }
        }
        BattleAction::MoveSwitch { slot, party_index } => {
            if *slot >= BATTLE_MOVE_SLOTS {
                return Err(BattleSyncError::InvalidMoveSlot { slot: *slot });
            }
            if *party_index >= PARTY_SIZE {
                return Err(BattleSyncError::InvalidSwitchPartyIndex {
                    party_index: *party_index,
                });
            }
        }
        BattleAction::Switch { party_index } => {
            if *party_index >= PARTY_SIZE {
                return Err(BattleSyncError::InvalidSwitchPartyIndex {
                    party_index: *party_index,
                });
            }
        }
        BattleAction::Item { item_id } => {
            if item_id.is_empty() {
                return Err(BattleSyncError::EmptyItemId);
            }
            if !is_exact_multiplayer_item_id(item_id) {
                return Err(BattleSyncError::InvalidItemId {
                    item_id: item_id.clone(),
                });
            }
        }
        BattleAction::Run => {}
    }
    Ok(())
}

fn is_exact_state_hash(value: &str) -> bool {
    value.len() == 8
        && value.trim() == value
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn is_exact_multiplayer_item_id(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && !has_reserved_pack_prefix(value)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b':')
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum BattleSyncError {
    #[error("battle sync player {player_id} is not in the accepted link roster")]
    UnknownPlayer { player_id: PlayerId },
    #[error("battle sync player id {player_id} is not a valid link identity")]
    InvalidPlayerIdentity { player_id: PlayerId },
    #[error("battle sync roster must contain at least one player")]
    EmptyRoster,
    #[error("battle sync state hash must be non-empty")]
    EmptyStateHash,
    #[error(
        "battle sync state hash {state_hash} must be an exact 8-character lowercase FNV hex hash"
    )]
    InvalidStateHash { state_hash: String },
    #[error("battle sync missing state hash for player {player_id}")]
    MissingStateHash { player_id: PlayerId },
    #[error("battle sync has state hash for player {player_id} without an action")]
    UnexpectedStateHash { player_id: PlayerId },
    #[error("battle sync item id must be non-empty")]
    EmptyItemId,
    #[error("battle sync item id {item_id} must be exact and untrimmed")]
    InvalidItemId { item_id: String },
    #[error("battle sync move slot {slot} is outside move range 0..4")]
    InvalidMoveSlot { slot: usize },
    #[error("battle sync switch party index {party_index} is outside party range 0..6")]
    InvalidSwitchPartyIndex { party_index: usize },
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum LockstepSyncError {
    #[error("lockstep player {player_id} is not in the accepted link roster")]
    UnknownPlayer { player_id: PlayerId },
    #[error("lockstep player id {player_id} is not a valid link identity")]
    InvalidPlayerIdentity { player_id: PlayerId },
    #[error("lockstep roster must contain at least one player")]
    EmptyRoster,
    #[error("lockstep input mask {mask:#010b} has conflicting direction buttons")]
    ConflictingJoypadDirections { mask: u8 },
    #[error("lockstep frame {actual} does not match expected frame {expected}")]
    FrameOutOfOrder { expected: u64, actual: u64 },
    #[error("lockstep frame {frame} is missing input for player {player_id}")]
    MissingPlayerInput { frame: u64, player_id: PlayerId },
    #[error("lockstep frame {frame} includes non-roster player {player_id}")]
    NonRosterPlayerInput { frame: u64, player_id: PlayerId },
    #[error("lockstep frame cursor overflowed at frame {frame}")]
    FrameCursorOverflow { frame: u64 },
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum InputJournalError {
    #[error("input journal session violates protocol invariants: {message}")]
    InvalidSession { message: String },
    #[error("input journal start checksum is invalid: {message}")]
    InvalidStartChecksum { message: String },
    #[error("input journal terminal checksum is invalid: {message}")]
    InvalidTerminalChecksum { message: String },
    #[error(
        "input journal terminal checksum frame {actual} does not match expected frame {expected}"
    )]
    TerminalChecksumFrameMismatch { expected: u64, actual: u64 },
    #[error(
        "input journal terminal checksum hash {actual:#010x} does not match expected hash {expected:#010x}"
    )]
    TerminalChecksumHashMismatch { expected: u32, actual: u32 },
    #[error("input journal terminal checksum player {player_id} is not in the declared roster")]
    TerminalChecksumUnknownPlayer { player_id: PlayerId },
    #[error(
        "input journal terminal checksum player {actual} does not match expected player {expected}"
    )]
    TerminalChecksumPlayerMismatch {
        expected: PlayerId,
        actual: PlayerId,
    },
    #[error("input journal frame is invalid: {message}")]
    InvalidFrame { message: String },
    #[error("input journal player {player_id} is not in the declared roster")]
    UnknownPlayer { player_id: PlayerId },
    #[error("input journal frame {actual} does not match expected frame {expected}")]
    FrameOutOfOrder { expected: u64, actual: u64 },
    #[error("input journal frame {frame} is missing player {player_id}")]
    MissingPlayerInput { frame: u64, player_id: PlayerId },
    #[error("input journal frame {frame} includes non-roster player {player_id}")]
    NonRosterPlayerInput { frame: u64, player_id: PlayerId },
    #[error("input journal frame cursor overflowed at frame {frame}")]
    FrameCursorOverflow { frame: u64 },
    #[error("failed to encode input journal for deterministic fingerprint: {message}")]
    Encode { message: String },
    #[error(
        "input journal fingerprint {actual} does not match deterministic fingerprint {expected}"
    )]
    FingerprintMismatch { expected: String, actual: String },
    #[error("deterministic replay command {sequence} is invalid: {message}")]
    InvalidRuntimeCommand { sequence: u64, message: String },
    #[error("deterministic replay command result {sequence} is invalid: {message}")]
    InvalidRuntimeCommandResult { sequence: u64, message: String },
    #[error(
        "deterministic replay menu choice result {menu_id}:{option_index} is invalid: {message}"
    )]
    InvalidMenuChoiceResult {
        menu_id: String,
        option_index: usize,
        message: String,
    },
    #[error("deterministic replay command {sequence} session does not match input journal session")]
    RuntimeCommandSessionMismatch { sequence: u64 },
    #[error(
        "deterministic replay command result {sequence} session does not match input journal session"
    )]
    RuntimeCommandResultSessionMismatch { sequence: u64 },
    #[error(
        "deterministic replay command sequence {actual} does not immediately follow {previous}"
    )]
    RuntimeCommandSequenceNotContiguous { previous: u64, actual: u64 },
    #[error(
        "deterministic replay has {results} command results for {commands} commands"
    )]
    RuntimeCommandResultCountMismatch { commands: usize, results: usize },
    #[error(
        "deterministic replay result at index {index} is for sequence {result_sequence}, expected command sequence {command_sequence}"
    )]
    RuntimeCommandResultRequestMismatch {
        index: usize,
        command_sequence: u64,
        result_sequence: u64,
    },
    #[error(
        "deterministic replay command {sequence} expected frame {frame} is outside journal frames {start}..={terminal}"
    )]
    RuntimeCommandFrameOutsideJournal {
        sequence: u64,
        frame: u64,
        start: u64,
        terminal: u64,
    },
    #[error(
        "deterministic replay command result {sequence} frame {frame} is outside journal frames {start}..={terminal}"
    )]
    RuntimeCommandResultFrameOutsideJournal {
        sequence: u64,
        frame: u64,
        start: u64,
        terminal: u64,
    },
    #[error(
        "deterministic replay menu choice {menu_id}:{option_index} frame {frame} is outside journal frames {start}..={terminal}"
    )]
    MenuChoiceFrameOutsideJournal {
        menu_id: String,
        option_index: usize,
        frame: u64,
        start: u64,
        terminal: u64,
    },
    #[error("save resume replay checkpoint is invalid: {message}")]
    InvalidSaveCheckpoint { message: String },
    #[error("save resume replay session does not match input journal session")]
    SaveReplaySessionMismatch,
    #[error(
        "save resume replay checkpoint frame {checkpoint_frame} does not match journal start frame {journal_frame}"
    )]
    SaveReplayStartFrameMismatch {
        checkpoint_frame: u64,
        journal_frame: u64,
    },
    #[error(
        "save resume replay checkpoint player {checkpoint_player_id} does not match journal start player {journal_player_id}"
    )]
    SaveReplayStartPlayerMismatch {
        checkpoint_player_id: PlayerId,
        journal_player_id: PlayerId,
    },
    #[error(
        "save resume replay checkpoint hash {checkpoint_hash:#010x} does not match journal start hash {journal_hash:#010x}"
    )]
    SaveReplayStartHashMismatch {
        checkpoint_hash: u32,
        journal_hash: u32,
    },
}

pub type TradeId = String;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InsertTradeFrameResult {
    Inserted,
    Duplicate,
    Conflict,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TradeParticipants {
    trade_id: TradeId,
    players: [PlayerId; 2],
}

impl<'de> Deserialize<'de> for TradeParticipants {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawTradeParticipants {
            trade_id: TradeId,
            players: [PlayerId; 2],
        }

        let raw = RawTradeParticipants::deserialize(deserializer)?;
        TradeParticipants::new(raw.trade_id, raw.players[0], raw.players[1])
            .map_err(serde::de::Error::custom)
    }
}

impl TradeParticipants {
    pub fn new(
        trade_id: impl Into<String>,
        player_a: PlayerId,
        player_b: PlayerId,
    ) -> Result<Self, TradeError> {
        let trade_id = trade_id.into();
        validate_trade_id(&trade_id)?;
        validate_trade_player_identity(player_a)?;
        validate_trade_player_identity(player_b)?;
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

    pub fn trade_id(&self) -> &str {
        &self.trade_id
    }

    pub fn players(&self) -> [PlayerId; 2] {
        self.players
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TradeOffer {
    trade_id: TradeId,
    player_id: PlayerId,
    party_slot: usize,
    pokemon: Pokemon,
}

impl<'de> Deserialize<'de> for TradeOffer {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawTradeOffer {
            trade_id: TradeId,
            player_id: PlayerId,
            party_slot: usize,
            pokemon: Pokemon,
        }

        let raw = RawTradeOffer::deserialize(deserializer)?;
        TradeOffer::new(raw.trade_id, raw.player_id, raw.party_slot, raw.pokemon)
            .map_err(serde::de::Error::custom)
    }
}

impl TradeOffer {
    pub fn new(
        trade_id: impl Into<String>,
        player_id: PlayerId,
        party_slot: usize,
        pokemon: Pokemon,
    ) -> Result<Self, TradeError> {
        let offer = Self {
            trade_id: trade_id.into(),
            player_id,
            party_slot,
            pokemon,
        };
        offer.validate()?;
        Ok(offer)
    }

    pub fn from_party(
        trade_id: impl Into<String>,
        player_id: PlayerId,
        party: &Party,
        party_slot: usize,
    ) -> Result<Self, TradeError> {
        let trade_id = trade_id.into();
        validate_trade_id(&trade_id)?;
        validate_trade_player_identity(player_id)?;
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

    pub fn validate(&self) -> Result<(), TradeError> {
        validate_trade_id(&self.trade_id)?;
        validate_trade_player_identity(self.player_id)?;
        if self.party_slot >= PARTY_SIZE {
            return Err(TradeError::InvalidPartySlot {
                party_slot: self.party_slot,
            });
        }
        self.pokemon
            .validate_saved_state()
            .map_err(|error| TradeError::InvalidPokemon {
                trade_id: self.trade_id.clone(),
                message: error,
            })?;
        Ok(())
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn new_unchecked_for_tests(
        trade_id: impl Into<String>,
        player_id: PlayerId,
        party_slot: usize,
        pokemon: Pokemon,
    ) -> Self {
        Self {
            trade_id: trade_id.into(),
            player_id,
            party_slot,
            pokemon,
        }
    }

    pub fn trade_id(&self) -> &str {
        &self.trade_id
    }

    pub fn player_id(&self) -> PlayerId {
        self.player_id
    }

    pub fn party_slot(&self) -> usize {
        self.party_slot
    }

    pub fn pokemon(&self) -> &Pokemon {
        &self.pokemon
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TradeConfirmation {
    trade_id: TradeId,
    player_id: PlayerId,
    confirm: bool,
}

impl<'de> Deserialize<'de> for TradeConfirmation {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawTradeConfirmation {
            trade_id: TradeId,
            player_id: PlayerId,
            confirm: bool,
        }

        let raw = RawTradeConfirmation::deserialize(deserializer)?;
        TradeConfirmation::new(raw.trade_id, raw.player_id, raw.confirm)
            .map_err(serde::de::Error::custom)
    }
}

impl TradeConfirmation {
    pub fn new(
        trade_id: impl Into<String>,
        player_id: PlayerId,
        confirm: bool,
    ) -> Result<Self, TradeError> {
        let confirmation = Self {
            trade_id: trade_id.into(),
            player_id,
            confirm,
        };
        confirmation.validate()?;
        Ok(confirmation)
    }

    pub fn validate(&self) -> Result<(), TradeError> {
        validate_trade_id(&self.trade_id)?;
        validate_trade_player_identity(self.player_id)
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn new_unchecked_for_tests(
        trade_id: impl Into<String>,
        player_id: PlayerId,
        confirm: bool,
    ) -> Self {
        Self {
            trade_id: trade_id.into(),
            player_id,
            confirm,
        }
    }

    pub fn trade_id(&self) -> &str {
        &self.trade_id
    }

    pub fn player_id(&self) -> PlayerId {
        self.player_id
    }

    pub fn confirm(&self) -> bool {
        self.confirm
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SessionTradeOffer {
    session: LinkSessionIdentity,
    offer: TradeOffer,
}

impl<'de> Deserialize<'de> for SessionTradeOffer {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawSessionTradeOffer {
            session: LinkSessionIdentity,
            offer: TradeOffer,
        }

        let raw = RawSessionTradeOffer::deserialize(deserializer)?;
        SessionTradeOffer::new(raw.session, raw.offer).map_err(serde::de::Error::custom)
    }
}

impl SessionTradeOffer {
    pub fn new(session: LinkSessionIdentity, offer: TradeOffer) -> Result<Self, String> {
        let frame = Self { session, offer };
        frame.validate()?;
        Ok(frame)
    }

    pub fn validate(&self) -> Result<(), String> {
        self.session.validate().map_err(|error| error.to_string())?;
        self.offer.validate().map_err(|error| error.to_string())
    }

    pub fn session(&self) -> &LinkSessionIdentity {
        &self.session
    }

    pub fn offer(&self) -> &TradeOffer {
        &self.offer
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SessionTradeConfirmation {
    session: LinkSessionIdentity,
    confirmation: TradeConfirmation,
}

impl<'de> Deserialize<'de> for SessionTradeConfirmation {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawSessionTradeConfirmation {
            session: LinkSessionIdentity,
            confirmation: TradeConfirmation,
        }

        let raw = RawSessionTradeConfirmation::deserialize(deserializer)?;
        SessionTradeConfirmation::new(raw.session, raw.confirmation)
            .map_err(serde::de::Error::custom)
    }
}

impl SessionTradeConfirmation {
    pub fn new(
        session: LinkSessionIdentity,
        confirmation: TradeConfirmation,
    ) -> Result<Self, String> {
        let frame = Self {
            session,
            confirmation,
        };
        frame.validate()?;
        Ok(frame)
    }

    pub fn validate(&self) -> Result<(), String> {
        self.session.validate().map_err(|error| error.to_string())?;
        self.confirmation
            .validate()
            .map_err(|error| error.to_string())
    }

    pub fn session(&self) -> &LinkSessionIdentity {
        &self.session
    }

    pub fn confirmation(&self) -> &TradeConfirmation {
        &self.confirmation
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TradeReplacement {
    party_slot: usize,
    received: Pokemon,
}

impl<'de> Deserialize<'de> for TradeReplacement {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawTradeReplacement {
            party_slot: usize,
            received: Pokemon,
        }

        let raw = RawTradeReplacement::deserialize(deserializer)?;
        TradeReplacement::new(raw.party_slot, raw.received).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TradeOutcome {
    trade_id: TradeId,
    cancelled: bool,
    replacements: BTreeMap<PlayerId, TradeReplacement>,
}

impl<'de> Deserialize<'de> for TradeOutcome {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawTradeOutcome {
            trade_id: TradeId,
            cancelled: bool,
            replacements: BTreeMap<PlayerId, TradeReplacement>,
        }

        let raw = RawTradeOutcome::deserialize(deserializer)?;
        TradeOutcome::new(raw.trade_id, raw.cancelled, raw.replacements)
            .map_err(serde::de::Error::custom)
    }
}

impl TradeOutcome {
    pub fn new(
        trade_id: impl Into<String>,
        cancelled: bool,
        replacements: BTreeMap<PlayerId, TradeReplacement>,
    ) -> Result<Self, TradeError> {
        let outcome = Self {
            trade_id: trade_id.into(),
            cancelled,
            replacements,
        };
        outcome.validate()?;
        Ok(outcome)
    }

    pub fn trade_id(&self) -> &str {
        &self.trade_id
    }

    pub fn cancelled(&self) -> bool {
        self.cancelled
    }

    pub fn replacements(&self) -> &BTreeMap<PlayerId, TradeReplacement> {
        &self.replacements
    }

    pub fn validate(&self) -> Result<(), TradeError> {
        validate_trade_id(&self.trade_id)?;
        if self.cancelled {
            if !self.replacements.is_empty() {
                return Err(TradeError::InvalidReplacementCount {
                    trade_id: self.trade_id.clone(),
                    expected: 0,
                    actual: self.replacements.len(),
                });
            }
            return Ok(());
        }
        if self.replacements.len() != 2 {
            return Err(TradeError::InvalidReplacementCount {
                trade_id: self.trade_id.clone(),
                expected: 2,
                actual: self.replacements.len(),
            });
        }
        for replacement in self.replacements.values() {
            replacement.validate()?;
        }
        Ok(())
    }

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
        replacement.validate()?;
        let previous = party.pokemon[replacement.party_slot()]
            .replace(replacement.received().clone())
            .ok_or(TradeError::EmptyPartySlot {
                party_slot: replacement.party_slot(),
            })?;
        Ok(Some(previous))
    }
}

impl TradeReplacement {
    pub fn new(party_slot: usize, received: Pokemon) -> Result<Self, TradeError> {
        let replacement = Self {
            party_slot,
            received,
        };
        replacement.validate()?;
        Ok(replacement)
    }

    pub fn validate(&self) -> Result<(), TradeError> {
        if self.party_slot >= PARTY_SIZE {
            return Err(TradeError::InvalidPartySlot {
                party_slot: self.party_slot,
            });
        }
        self.received
            .validate_saved_state()
            .map_err(|error| TradeError::InvalidPokemon {
                trade_id: "trade replacement".to_string(),
                message: error,
            })?;
        Ok(())
    }

    pub fn party_slot(&self) -> usize {
        self.party_slot
    }

    pub fn received(&self) -> &Pokemon {
        &self.received
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum TradeError {
    #[error("trade id is required")]
    MissingTradeId,
    #[error("trade id {trade_id} must be exact and untrimmed")]
    InvalidTradeId { trade_id: TradeId },
    #[error("trade {actual} does not match expected {expected}")]
    TradeIdMismatch { expected: TradeId, actual: TradeId },
    #[error("trade player {player_id} is not in the accepted link roster")]
    UnknownPlayer { player_id: PlayerId },
    #[error("trade player id {player_id} is not a valid link identity")]
    InvalidPlayerIdentity { player_id: PlayerId },
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
    #[error("trade {trade_id} has {actual} replacements but expected {expected}")]
    InvalidReplacementCount {
        trade_id: TradeId,
        expected: usize,
        actual: usize,
    },
    #[error("trade {trade_id} carries invalid Pokemon: {message}")]
    InvalidPokemon { trade_id: TradeId, message: String },
}

fn validate_trade_id(trade_id: &str) -> Result<(), TradeError> {
    if trade_id.is_empty() {
        return Err(TradeError::MissingTradeId);
    }
    if !is_exact_trade_id(trade_id) {
        return Err(TradeError::InvalidTradeId {
            trade_id: trade_id.to_string(),
        });
    }
    Ok(())
}

fn validate_trade_player_identity(player_id: PlayerId) -> Result<(), TradeError> {
    if player_id == 0 {
        return Err(TradeError::InvalidPlayerIdentity { player_id });
    }
    Ok(())
}

fn is_exact_trade_id(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && !has_reserved_pack_prefix(value)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
}

fn has_reserved_pack_prefix(value: &str) -> bool {
    let value = value.to_ascii_lowercase();
    value.starts_with("fallback") || value.starts_with("legacy")
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LinkByteFrame {
    player_id: PlayerId,
    byte: u8,
    clock: u64,
}

impl<'de> Deserialize<'de> for LinkByteFrame {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawLinkByteFrame {
            player_id: PlayerId,
            byte: u8,
            clock: u64,
        }

        let raw = RawLinkByteFrame::deserialize(deserializer)?;
        LinkByteFrame::new(raw.player_id, raw.byte, raw.clock).map_err(serde::de::Error::custom)
    }
}

impl LinkByteFrame {
    pub fn new(player_id: PlayerId, byte: u8, clock: u64) -> Result<Self, LinkCableError> {
        let frame = Self {
            player_id,
            byte,
            clock,
        };
        frame.validate()?;
        Ok(frame)
    }

    pub fn validate(&self) -> Result<(), LinkCableError> {
        if self.player_id == 0 {
            return Err(LinkCableError::InvalidPlayerIdentity {
                player_id: self.player_id,
            });
        }
        if self.clock == 0 {
            return Err(LinkCableError::InvalidClock { clock: self.clock });
        }
        Ok(())
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub const fn new_unchecked_for_tests(player_id: PlayerId, byte: u8, clock: u64) -> Self {
        Self {
            player_id,
            byte,
            clock,
        }
    }

    pub const fn player_id(&self) -> PlayerId {
        self.player_id
    }

    pub const fn byte(&self) -> u8 {
        self.byte
    }

    pub const fn clock(&self) -> u64 {
        self.clock
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SessionLinkByteFrame {
    session: LinkSessionIdentity,
    frame: LinkByteFrame,
}

impl<'de> Deserialize<'de> for SessionLinkByteFrame {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawSessionLinkByteFrame {
            session: LinkSessionIdentity,
            frame: LinkByteFrame,
        }

        let raw = RawSessionLinkByteFrame::deserialize(deserializer)?;
        SessionLinkByteFrame::new(raw.session, raw.frame).map_err(serde::de::Error::custom)
    }
}

impl SessionLinkByteFrame {
    pub fn new(session: LinkSessionIdentity, frame: LinkByteFrame) -> Result<Self, String> {
        let bound = Self { session, frame };
        bound.validate()?;
        Ok(bound)
    }

    pub fn validate(&self) -> Result<(), String> {
        self.session.validate().map_err(|error| error.to_string())?;
        self.frame.validate().map_err(|error| error.to_string())
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub const fn new_unchecked_for_tests(
        session: LinkSessionIdentity,
        frame: LinkByteFrame,
    ) -> Self {
        Self { session, frame }
    }

    pub const fn session(&self) -> &LinkSessionIdentity {
        &self.session
    }

    pub const fn frame(&self) -> &LinkByteFrame {
        &self.frame
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LinkClockSyncFrame {
    player_id: PlayerId,
    t0: u64,
    t1: u64,
    t2: u64,
}

impl<'de> Deserialize<'de> for LinkClockSyncFrame {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawLinkClockSyncFrame {
            player_id: PlayerId,
            t0: u64,
            t1: u64,
            t2: u64,
        }

        let raw = RawLinkClockSyncFrame::deserialize(deserializer)?;
        LinkClockSyncFrame::new(raw.player_id, raw.t0, raw.t1, raw.t2)
            .map_err(serde::de::Error::custom)
    }
}

impl LinkClockSyncFrame {
    pub fn new(player_id: PlayerId, t0: u64, t1: u64, t2: u64) -> Result<Self, LinkCableError> {
        let frame = Self {
            player_id,
            t0,
            t1,
            t2,
        };
        frame.validate()?;
        Ok(frame)
    }

    pub fn validate(&self) -> Result<(), LinkCableError> {
        if self.player_id == 0 {
            return Err(LinkCableError::InvalidPlayerIdentity {
                player_id: self.player_id,
            });
        }
        if self.t2 == 0 {
            return Err(LinkCableError::InvalidClock { clock: self.t2 });
        }
        if self.t0 > self.t1 || self.t1 > self.t2 {
            return Err(LinkCableError::InvalidClockSync {
                t0: self.t0,
                t1: self.t1,
                t2: self.t2,
            });
        }
        Ok(())
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub const fn new_unchecked_for_tests(player_id: PlayerId, t0: u64, t1: u64, t2: u64) -> Self {
        Self {
            player_id,
            t0,
            t1,
            t2,
        }
    }

    pub const fn player_id(&self) -> PlayerId {
        self.player_id
    }

    pub const fn t0(&self) -> u64 {
        self.t0
    }

    pub const fn t1(&self) -> u64 {
        self.t1
    }

    pub const fn t2(&self) -> u64 {
        self.t2
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SessionLinkClockSyncFrame {
    session: LinkSessionIdentity,
    frame: LinkClockSyncFrame,
}

impl<'de> Deserialize<'de> for SessionLinkClockSyncFrame {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawSessionLinkClockSyncFrame {
            session: LinkSessionIdentity,
            frame: LinkClockSyncFrame,
        }

        let raw = RawSessionLinkClockSyncFrame::deserialize(deserializer)?;
        SessionLinkClockSyncFrame::new(raw.session, raw.frame).map_err(serde::de::Error::custom)
    }
}

impl SessionLinkClockSyncFrame {
    pub fn new(session: LinkSessionIdentity, frame: LinkClockSyncFrame) -> Result<Self, String> {
        let bound = Self { session, frame };
        bound.validate()?;
        Ok(bound)
    }

    pub fn validate(&self) -> Result<(), String> {
        self.session.validate().map_err(|error| error.to_string())?;
        self.frame.validate().map_err(|error| error.to_string())
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub const fn new_unchecked_for_tests(
        session: LinkSessionIdentity,
        frame: LinkClockSyncFrame,
    ) -> Self {
        Self { session, frame }
    }

    pub const fn session(&self) -> &LinkSessionIdentity {
        &self.session
    }

    pub const fn frame(&self) -> &LinkClockSyncFrame {
        &self.frame
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum LinkCableError {
    #[error("link cable endpoint cannot use player {player_id} as both local and remote")]
    DuplicateEndpoint { player_id: PlayerId },
    #[error("link cable player {player_id} is not in the accepted link roster")]
    UnknownPlayer { player_id: PlayerId },
    #[error("link cable player id {player_id} is not a valid link identity")]
    InvalidPlayerIdentity { player_id: PlayerId },
    #[error("link cable frame from player {player_id} does not match remote player {expected}")]
    UnexpectedPeer {
        expected: PlayerId,
        player_id: PlayerId,
    },
    #[error("link cable clock {clock} must be nonzero")]
    InvalidClock { clock: u64 },
    #[error("link cable clock {clock} did not advance beyond remote clock {remote_clock}")]
    ClockRegression { remote_clock: u64, clock: u64 },
    #[error("link cable clock sync requires t0 <= t1 <= t2 but got t0={t0}, t1={t1}, t2={t2}")]
    InvalidClockSync { t0: u64, t1: u64, t2: u64 },
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
        if local_player == 0 {
            return Err(LinkCableError::InvalidPlayerIdentity {
                player_id: local_player,
            });
        }
        if remote_player == 0 {
            return Err(LinkCableError::InvalidPlayerIdentity {
                player_id: remote_player,
            });
        }
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
        frame.validate()?;
        self.validate_remote_frame(frame.player_id(), frame.clock())?;
        self.remote_clock = frame.clock();
        self.receive_buffer.push_back(frame.byte());
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
        self.local_clock = self.local_clock.saturating_add(1).max(now_tick);
        LinkClockSyncFrame {
            player_id: self.local_player,
            t0: self.local_clock,
            t1: self.local_clock,
            t2: self.local_clock,
        }
    }

    pub fn receive_sync_frame(
        &mut self,
        frame: LinkClockSyncFrame,
        receive_tick: u64,
    ) -> Result<(), LinkCableError> {
        frame.validate()?;
        self.validate_remote_frame(frame.player_id(), frame.t2())?;
        self.remote_clock = frame.t2();
        let remote_processing = frame.t2().saturating_sub(frame.t1());
        let round_trip = receive_tick
            .saturating_sub(frame.t0())
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
        for player_id in participants.players() {
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
        offer.validate()?;
        self.validate_trade_player(offer.trade_id(), offer.player_id())?;
        match self.offers.get(&offer.player_id()) {
            Some(existing) if existing == &offer => Ok(InsertTradeFrameResult::Duplicate),
            Some(_) => Ok(InsertTradeFrameResult::Conflict),
            None => {
                self.offers.insert(offer.player_id(), offer);
                Ok(InsertTradeFrameResult::Inserted)
            }
        }
    }

    pub fn insert_confirmation(
        &mut self,
        confirmation: TradeConfirmation,
    ) -> Result<InsertTradeFrameResult, TradeError> {
        confirmation.validate()?;
        self.validate_trade_player(confirmation.trade_id(), confirmation.player_id())?;
        match self.confirmations.get(&confirmation.player_id()) {
            Some(existing) if *existing == confirmation.confirm() => {
                Ok(InsertTradeFrameResult::Duplicate)
            }
            Some(_) => Ok(InsertTradeFrameResult::Conflict),
            None => {
                self.confirmations
                    .insert(confirmation.player_id(), confirmation.confirm());
                Ok(InsertTradeFrameResult::Inserted)
            }
        }
    }

    pub fn is_ready(&self) -> bool {
        self.participants
            .players()
            .iter()
            .all(|player_id| self.offers.contains_key(player_id))
            && self
                .participants
                .players()
                .iter()
                .all(|player_id| self.confirmations.contains_key(player_id))
    }

    pub fn outcome(&self) -> Result<TradeOutcome, TradeError> {
        if !self.is_ready() {
            return Err(TradeError::TradeNotReady {
                trade_id: self.participants.trade_id().to_string(),
            });
        }
        let cancelled = self.confirmations.values().any(|confirm| !confirm);
        let mut replacements = BTreeMap::new();
        if !cancelled {
            for player_id in self.participants.players() {
                let Some(other_player) = self.participants.other_player(player_id) else {
                    return Err(TradeError::TradeNotReady {
                        trade_id: self.participants.trade_id().to_string(),
                    });
                };
                let Some(local_offer) = self.offers.get(&player_id) else {
                    return Err(TradeError::TradeNotReady {
                        trade_id: self.participants.trade_id().to_string(),
                    });
                };
                let Some(remote_offer) = self.offers.get(&other_player) else {
                    return Err(TradeError::TradeNotReady {
                        trade_id: self.participants.trade_id().to_string(),
                    });
                };
                replacements.insert(
                    player_id,
                    TradeReplacement::new(
                        local_offer.party_slot(),
                        remote_offer.pokemon().clone(),
                    )?,
                );
            }
        }
        TradeOutcome::new(
            self.participants.trade_id().to_string(),
            cancelled,
            replacements,
        )
    }

    fn validate_trade_player(&self, trade_id: &str, player_id: PlayerId) -> Result<(), TradeError> {
        validate_trade_id(trade_id)?;
        if trade_id != self.participants.trade_id() {
            return Err(TradeError::TradeIdMismatch {
                expected: self.participants.trade_id().to_string(),
                actual: trade_id.to_string(),
            });
        }
        if !self.participants.contains(player_id) {
            return Err(TradeError::NotParticipant {
                player_id,
                trade_id: self.participants.trade_id().to_string(),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InsertChecksumResult {
    Inserted,
    Duplicate,
    Conflict,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockstepFrame {
    frame: u64,
    inputs: BTreeMap<PlayerId, u8>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct LockstepInputEntry {
    player_id: PlayerId,
    joypad_mask: u8,
}

impl Serialize for LockstepFrame {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        #[derive(Serialize)]
        #[serde(deny_unknown_fields)]
        struct RawLockstepFrame {
            frame: u64,
            inputs: Vec<LockstepInputEntry>,
        }

        let inputs = self
            .inputs
            .iter()
            .map(|(player_id, joypad_mask)| LockstepInputEntry {
                player_id: *player_id,
                joypad_mask: *joypad_mask,
            })
            .collect();
        RawLockstepFrame {
            frame: self.frame,
            inputs,
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for LockstepFrame {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawLockstepFrame {
            frame: u64,
            inputs: Vec<LockstepInputEntry>,
        }

        let raw = RawLockstepFrame::deserialize(deserializer)?;
        let mut inputs = BTreeMap::new();
        for entry in raw.inputs {
            if inputs.insert(entry.player_id, entry.joypad_mask).is_some() {
                return Err(serde::de::Error::custom(format!(
                    "lockstep frame {} contains duplicate input for player {}",
                    raw.frame, entry.player_id
                )));
            }
        }
        LockstepFrame::new(raw.frame, inputs).map_err(serde::de::Error::custom)
    }
}

impl LockstepFrame {
    pub fn new(frame: u64, inputs: BTreeMap<PlayerId, u8>) -> Result<Self, LockstepSyncError> {
        let lockstep_frame = Self { frame, inputs };
        lockstep_frame.validate_inputs()?;
        Ok(lockstep_frame)
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn new_unchecked_for_tests(frame: u64, inputs: BTreeMap<PlayerId, u8>) -> Self {
        Self { frame, inputs }
    }

    fn validate_inputs(&self) -> Result<(), LockstepSyncError> {
        if self.inputs.is_empty() {
            return Err(LockstepSyncError::EmptyRoster);
        }
        for (player_id, mask) in &self.inputs {
            if *player_id == 0 {
                return Err(LockstepSyncError::InvalidPlayerIdentity {
                    player_id: *player_id,
                });
            }
            validate_lockstep_joypad_mask(*mask)?;
        }
        Ok(())
    }

    pub const fn frame(&self) -> u64 {
        self.frame
    }

    pub fn inputs(&self) -> &BTreeMap<PlayerId, u8> {
        &self.inputs
    }

    pub fn joypad_mask_for(&self, player_id: PlayerId) -> Option<u8> {
        self.inputs.get(&player_id).copied()
    }

    pub fn ordered_inputs(&self, players: &[PlayerId]) -> Option<Vec<u8>> {
        players
            .iter()
            .map(|player_id| self.inputs.get(player_id).copied())
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DeterministicInputJournal {
    session: LinkSessionIdentity,
    players: BTreeSet<PlayerId>,
    start_checksum: StateChecksumFrame,
    terminal_checksum: StateChecksumFrame,
    frames: Vec<LockstepFrame>,
}

impl<'de> Deserialize<'de> for DeterministicInputJournal {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawDeterministicInputJournal {
            session: LinkSessionIdentity,
            players: BTreeSet<PlayerId>,
            start_checksum: StateChecksumFrame,
            terminal_checksum: StateChecksumFrame,
            frames: Vec<LockstepFrame>,
        }

        let raw = RawDeterministicInputJournal::deserialize(deserializer)?;
        DeterministicInputJournal::new(
            raw.session,
            raw.players,
            raw.start_checksum,
            raw.terminal_checksum,
            raw.frames,
        )
        .map_err(serde::de::Error::custom)
    }
}

impl DeterministicInputJournal {
    pub fn new(
        session: LinkSessionIdentity,
        players: impl IntoIterator<Item = PlayerId>,
        start_checksum: StateChecksumFrame,
        terminal_checksum: StateChecksumFrame,
        frames: Vec<LockstepFrame>,
    ) -> Result<Self, InputJournalError> {
        let journal = Self {
            session,
            players: players.into_iter().collect(),
            start_checksum,
            terminal_checksum,
            frames,
        };
        journal.validate()?;
        Ok(journal)
    }

    pub fn validate(&self) -> Result<(), InputJournalError> {
        self.session
            .validate()
            .map_err(|error| InputJournalError::InvalidSession {
                message: error.to_string(),
            })?;
        self.start_checksum.validate().map_err(|error| {
            InputJournalError::InvalidStartChecksum {
                message: error.to_string(),
            }
        })?;
        self.terminal_checksum.validate().map_err(|error| {
            InputJournalError::InvalidTerminalChecksum {
                message: error.to_string(),
            }
        })?;
        if self.players.is_empty() {
            return Err(InputJournalError::InvalidFrame {
                message: LockstepSyncError::EmptyRoster.to_string(),
            });
        }
        for player_id in &self.players {
            if *player_id == 0 {
                return Err(InputJournalError::InvalidFrame {
                    message: LockstepSyncError::InvalidPlayerIdentity {
                        player_id: *player_id,
                    }
                    .to_string(),
                });
            }
        }
        if !self.players.contains(&self.start_checksum.player_id()) {
            return Err(InputJournalError::UnknownPlayer {
                player_id: self.start_checksum.player_id(),
            });
        }
        if !self.players.contains(&self.terminal_checksum.player_id()) {
            return Err(InputJournalError::TerminalChecksumUnknownPlayer {
                player_id: self.terminal_checksum.player_id(),
            });
        }
        let mut expected_frame = self.start_checksum.frame();
        for frame in &self.frames {
            frame
                .validate_inputs()
                .map_err(|error| InputJournalError::InvalidFrame {
                    message: error.to_string(),
                })?;
            if frame.frame() != expected_frame {
                return Err(InputJournalError::FrameOutOfOrder {
                    expected: expected_frame,
                    actual: frame.frame(),
                });
            }
            for player_id in &self.players {
                if !frame.inputs().contains_key(player_id) {
                    return Err(InputJournalError::MissingPlayerInput {
                        frame: frame.frame(),
                        player_id: *player_id,
                    });
                }
            }
            for player_id in frame.inputs().keys() {
                if !self.players.contains(player_id) {
                    return Err(InputJournalError::NonRosterPlayerInput {
                        frame: frame.frame(),
                        player_id: *player_id,
                    });
                }
            }
            expected_frame =
                expected_frame
                    .checked_add(1)
                    .ok_or(InputJournalError::FrameCursorOverflow {
                        frame: frame.frame(),
                    })?;
        }
        if self.terminal_checksum.frame() != expected_frame {
            return Err(InputJournalError::TerminalChecksumFrameMismatch {
                expected: expected_frame,
                actual: self.terminal_checksum.frame(),
            });
        }
        Ok(())
    }

    pub fn session(&self) -> &LinkSessionIdentity {
        &self.session
    }

    pub fn players(&self) -> &BTreeSet<PlayerId> {
        &self.players
    }

    pub fn start_checksum(&self) -> &StateChecksumFrame {
        &self.start_checksum
    }

    pub fn terminal_checksum(&self) -> &StateChecksumFrame {
        &self.terminal_checksum
    }

    pub fn frames(&self) -> &[LockstepFrame] {
        &self.frames
    }

    pub fn canonical_bytes(&self) -> Result<Vec<u8>, InputJournalError> {
        self.validate()?;
        bincode::serde::encode_to_vec(self, link_message_binary_config()).map_err(|error| {
            InputJournalError::Encode {
                message: error.to_string(),
            }
        })
    }

    pub fn fingerprint(&self) -> Result<u32, InputJournalError> {
        Ok(fnv1a32_bytes(&self.canonical_bytes()?))
    }

    pub fn fingerprint_hex(&self) -> Result<String, InputJournalError> {
        Ok(format!("{:08x}", self.fingerprint()?))
    }

    pub fn into_frames(self) -> Vec<LockstepFrame> {
        self.frames
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DeterministicInputJournalFrame {
    fingerprint: String,
    journal: DeterministicInputJournal,
}

impl<'de> Deserialize<'de> for DeterministicInputJournalFrame {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawDeterministicInputJournalFrame {
            fingerprint: String,
            journal: DeterministicInputJournal,
        }

        let raw = RawDeterministicInputJournalFrame::deserialize(deserializer)?;
        let frame = Self {
            fingerprint: raw.fingerprint,
            journal: raw.journal,
        };
        frame.validate().map_err(serde::de::Error::custom)?;
        Ok(frame)
    }
}

impl DeterministicInputJournalFrame {
    pub fn new(journal: DeterministicInputJournal) -> Result<Self, InputJournalError> {
        let fingerprint = journal.fingerprint_hex()?;
        Ok(Self {
            fingerprint,
            journal,
        })
    }

    pub fn validate(&self) -> Result<(), InputJournalError> {
        self.journal.validate()?;
        let expected = self.journal.fingerprint_hex()?;
        if self.fingerprint != expected {
            return Err(InputJournalError::FingerprintMismatch {
                expected,
                actual: self.fingerprint.clone(),
            });
        }
        Ok(())
    }

    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }

    pub fn journal(&self) -> &DeterministicInputJournal {
        &self.journal
    }

    pub fn into_journal(self) -> DeterministicInputJournal {
        self.journal
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DeterministicReplayBundle {
    input_journal: DeterministicInputJournalFrame,
    runtime_commands: Vec<SessionRuntimeCommandFrame>,
    runtime_results: Vec<SessionRuntimeCommandResultFrame>,
    menu_results: Vec<MenuChoiceResultFrame>,
    terminal_checksum: StateChecksumFrame,
}

impl<'de> Deserialize<'de> for DeterministicReplayBundle {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawDeterministicReplayBundle {
            input_journal: DeterministicInputJournalFrame,
            runtime_commands: Vec<SessionRuntimeCommandFrame>,
            runtime_results: Vec<SessionRuntimeCommandResultFrame>,
            menu_results: Vec<MenuChoiceResultFrame>,
            terminal_checksum: StateChecksumFrame,
        }

        let raw = RawDeterministicReplayBundle::deserialize(deserializer)?;
        DeterministicReplayBundle::new(
            raw.input_journal,
            raw.runtime_commands,
            raw.runtime_results,
            raw.menu_results,
            raw.terminal_checksum,
        )
        .map_err(serde::de::Error::custom)
    }
}

impl DeterministicReplayBundle {
    pub fn new(
        input_journal: DeterministicInputJournalFrame,
        runtime_commands: Vec<SessionRuntimeCommandFrame>,
        runtime_results: Vec<SessionRuntimeCommandResultFrame>,
        menu_results: Vec<MenuChoiceResultFrame>,
        terminal_checksum: StateChecksumFrame,
    ) -> Result<Self, InputJournalError> {
        let bundle = Self {
            input_journal,
            runtime_commands,
            runtime_results,
            menu_results,
            terminal_checksum,
        };
        bundle.validate()?;
        Ok(bundle)
    }

    pub fn validate(&self) -> Result<(), InputJournalError> {
        self.input_journal.validate()?;
        self.terminal_checksum.validate().map_err(|error| {
            InputJournalError::InvalidTerminalChecksum {
                message: error.to_string(),
            }
        })?;
        let journal = self.input_journal.journal();
        if self.terminal_checksum.frame() != journal.terminal_checksum().frame() {
            return Err(InputJournalError::TerminalChecksumFrameMismatch {
                expected: journal.terminal_checksum().frame(),
                actual: self.terminal_checksum.frame(),
            });
        }
        if self.terminal_checksum.player_id() != journal.terminal_checksum().player_id() {
            return Err(InputJournalError::TerminalChecksumPlayerMismatch {
                expected: journal.terminal_checksum().player_id(),
                actual: self.terminal_checksum.player_id(),
            });
        }
        if self.terminal_checksum.hash() != journal.terminal_checksum().hash() {
            return Err(InputJournalError::TerminalChecksumHashMismatch {
                expected: journal.terminal_checksum().hash(),
                actual: self.terminal_checksum.hash(),
            });
        }
        let start = journal.start_checksum().frame();
        let terminal = journal.terminal_checksum().frame();
        for command in &self.runtime_commands {
            command
                .validate()
                .map_err(|error| InputJournalError::InvalidRuntimeCommand {
                    sequence: command.command().sequence(),
                    message: error.to_string(),
                })?;
            if !link_session_identity_matches(command.session(), journal.session()) {
                return Err(InputJournalError::RuntimeCommandSessionMismatch {
                    sequence: command.command().sequence(),
                });
            }
            let frame = command.command().expected_state().frame();
            if frame < start || frame > terminal {
                return Err(InputJournalError::RuntimeCommandFrameOutsideJournal {
                    sequence: command.command().sequence(),
                    frame,
                    start,
                    terminal,
                });
            }
        }
        for result in &self.runtime_results {
            result
                .validate()
                .map_err(|error| InputJournalError::InvalidRuntimeCommandResult {
                    sequence: result.result().request().sequence(),
                    message: error.to_string(),
                })?;
            if !link_session_identity_matches(result.session(), journal.session()) {
                return Err(InputJournalError::RuntimeCommandResultSessionMismatch {
                    sequence: result.result().request().sequence(),
                });
            }
            let request_frame = result.result().request().expected_state().frame();
            if request_frame < start || request_frame > terminal {
                return Err(InputJournalError::RuntimeCommandResultFrameOutsideJournal {
                    sequence: result.result().request().sequence(),
                    frame: request_frame,
                    start,
                    terminal,
                });
            }
            let frame = result.result().checksum().frame();
            if frame < start || frame > terminal {
                return Err(InputJournalError::RuntimeCommandResultFrameOutsideJournal {
                    sequence: result.result().request().sequence(),
                    frame,
                    start,
                    terminal,
                });
            }
        }
        if let Some(first) = self.runtime_commands.first() {
            let actual = first.command().sequence();
            if actual != 1 {
                return Err(InputJournalError::RuntimeCommandSequenceNotContiguous {
                    previous: 0,
                    actual,
                });
            }
        }
        for commands in self.runtime_commands.windows(2) {
            let previous = commands[0].command().sequence();
            let actual = commands[1].command().sequence();
            if previous.checked_add(1) != Some(actual) {
                return Err(InputJournalError::RuntimeCommandSequenceNotContiguous {
                    previous,
                    actual,
                });
            }
        }
        if self.runtime_commands.len() != self.runtime_results.len() {
            return Err(InputJournalError::RuntimeCommandResultCountMismatch {
                commands: self.runtime_commands.len(),
                results: self.runtime_results.len(),
            });
        }
        for (index, (command, result)) in self
            .runtime_commands
            .iter()
            .zip(&self.runtime_results)
            .enumerate()
        {
            if result.result().request() != command.command() {
                return Err(InputJournalError::RuntimeCommandResultRequestMismatch {
                    index,
                    command_sequence: command.command().sequence(),
                    result_sequence: result.result().request().sequence(),
                });
            }
        }
        for result in &self.menu_results {
            result
                .validate()
                .map_err(|error| InputJournalError::InvalidMenuChoiceResult {
                    menu_id: result.choice().menu_id().to_string(),
                    option_index: result.choice().option_index(),
                    message: error.to_string(),
                })?;
            let choice_frame = result.choice().frame();
            if choice_frame < start || choice_frame > terminal {
                return Err(InputJournalError::MenuChoiceFrameOutsideJournal {
                    menu_id: result.choice().menu_id().to_string(),
                    option_index: result.choice().option_index(),
                    frame: choice_frame,
                    start,
                    terminal,
                });
            }
            let checksum_frame = result.checksum().frame();
            if checksum_frame < start || checksum_frame > terminal {
                return Err(InputJournalError::MenuChoiceFrameOutsideJournal {
                    menu_id: result.choice().menu_id().to_string(),
                    option_index: result.choice().option_index(),
                    frame: checksum_frame,
                    start,
                    terminal,
                });
            }
        }
        Ok(())
    }

    pub fn input_journal(&self) -> &DeterministicInputJournalFrame {
        &self.input_journal
    }

    pub fn runtime_commands(&self) -> &[SessionRuntimeCommandFrame] {
        &self.runtime_commands
    }

    pub fn runtime_results(&self) -> &[SessionRuntimeCommandResultFrame] {
        &self.runtime_results
    }

    pub fn menu_results(&self) -> &[MenuChoiceResultFrame] {
        &self.menu_results
    }

    pub fn terminal_checksum(&self) -> &StateChecksumFrame {
        &self.terminal_checksum
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SaveResumeReplayBundle {
    checkpoint: SessionSaveCheckpointFrame,
    replay: DeterministicReplayBundle,
}

impl<'de> Deserialize<'de> for SaveResumeReplayBundle {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawSaveResumeReplayBundle {
            checkpoint: SessionSaveCheckpointFrame,
            replay: DeterministicReplayBundle,
        }

        let raw = RawSaveResumeReplayBundle::deserialize(deserializer)?;
        SaveResumeReplayBundle::new(raw.checkpoint, raw.replay).map_err(serde::de::Error::custom)
    }
}

impl SaveResumeReplayBundle {
    pub fn new(
        checkpoint: SessionSaveCheckpointFrame,
        replay: DeterministicReplayBundle,
    ) -> Result<Self, InputJournalError> {
        let bundle = Self { checkpoint, replay };
        bundle.validate()?;
        Ok(bundle)
    }

    pub fn validate(&self) -> Result<(), InputJournalError> {
        self.checkpoint
            .validate()
            .map_err(|error| InputJournalError::InvalidSaveCheckpoint {
                message: error.to_string(),
            })?;
        self.replay.validate()?;
        let journal = self.replay.input_journal().journal();
        if !link_session_identity_matches(self.checkpoint.session(), journal.session()) {
            return Err(InputJournalError::SaveReplaySessionMismatch);
        }
        let checkpoint = self.checkpoint.checkpoint().checksum();
        let start = journal.start_checksum();
        if checkpoint.frame() != start.frame() {
            return Err(InputJournalError::SaveReplayStartFrameMismatch {
                checkpoint_frame: checkpoint.frame(),
                journal_frame: start.frame(),
            });
        }
        if checkpoint.player_id() != start.player_id() {
            return Err(InputJournalError::SaveReplayStartPlayerMismatch {
                checkpoint_player_id: checkpoint.player_id(),
                journal_player_id: start.player_id(),
            });
        }
        if checkpoint.hash() != start.hash() {
            return Err(InputJournalError::SaveReplayStartHashMismatch {
                checkpoint_hash: checkpoint.hash(),
                journal_hash: start.hash(),
            });
        }
        Ok(())
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn new_unchecked_for_tests(
        checkpoint: SessionSaveCheckpointFrame,
        replay: DeterministicReplayBundle,
    ) -> Self {
        Self { checkpoint, replay }
    }

    pub const fn checkpoint(&self) -> &SessionSaveCheckpointFrame {
        &self.checkpoint
    }

    pub const fn replay(&self) -> &DeterministicReplayBundle {
        &self.replay
    }
}

fn link_session_identity_matches(
    expected: &LinkSessionIdentity,
    actual: &LinkSessionIdentity,
) -> bool {
    expected.protocol_version() == actual.protocol_version()
        && expected.session_id() == actual.session_id()
        && expected.modpack().id() == actual.modpack().id()
        && expected.modpack().hash() == actual.modpack().hash()
        && expected.pack_content_hash() == actual.pack_content_hash()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BattleActionTurn {
    turn: u64,
    actions: BTreeMap<PlayerId, BattleAction>,
    state_hashes: BTreeMap<PlayerId, String>,
}

impl<'de> Deserialize<'de> for BattleActionTurn {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawBattleActionTurn {
            turn: u64,
            actions: BTreeMap<PlayerId, BattleAction>,
            state_hashes: BTreeMap<PlayerId, String>,
        }

        let raw = RawBattleActionTurn::deserialize(deserializer)?;
        BattleActionTurn::new(raw.turn, raw.actions, raw.state_hashes)
            .map_err(serde::de::Error::custom)
    }
}

impl BattleActionTurn {
    pub fn new(
        turn: u64,
        actions: BTreeMap<PlayerId, BattleAction>,
        state_hashes: BTreeMap<PlayerId, String>,
    ) -> Result<Self, BattleSyncError> {
        let action_turn = Self {
            turn,
            actions,
            state_hashes,
        };
        action_turn.validate()?;
        Ok(action_turn)
    }

    pub fn validate(&self) -> Result<(), BattleSyncError> {
        if self.actions.is_empty() || self.state_hashes.is_empty() {
            return Err(BattleSyncError::EmptyRoster);
        }
        for player_id in self.actions.keys().chain(self.state_hashes.keys()) {
            if *player_id == 0 {
                return Err(BattleSyncError::InvalidPlayerIdentity {
                    player_id: *player_id,
                });
            }
        }
        for action in self.actions.values() {
            validate_battle_action(action)?;
        }
        for player_id in self.actions.keys() {
            if !self.state_hashes.contains_key(player_id) {
                return Err(BattleSyncError::MissingStateHash {
                    player_id: *player_id,
                });
            }
        }
        for player_id in self.state_hashes.keys() {
            if !self.actions.contains_key(player_id) {
                return Err(BattleSyncError::UnexpectedStateHash {
                    player_id: *player_id,
                });
            }
        }
        for state_hash in self.state_hashes.values() {
            if state_hash.is_empty() {
                return Err(BattleSyncError::EmptyStateHash);
            }
            if !is_exact_state_hash(state_hash) {
                return Err(BattleSyncError::InvalidStateHash {
                    state_hash: state_hash.clone(),
                });
            }
        }
        Ok(())
    }

    pub const fn turn(&self) -> u64 {
        self.turn
    }

    pub fn actions(&self) -> &BTreeMap<PlayerId, BattleAction> {
        &self.actions
    }

    pub fn state_hashes(&self) -> &BTreeMap<PlayerId, String> {
        &self.state_hashes
    }

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
    pub fn new(players: impl IntoIterator<Item = PlayerId>) -> Result<Self, BattleSyncError> {
        let players: BTreeSet<PlayerId> = players.into_iter().collect();
        if players.is_empty() {
            return Err(BattleSyncError::EmptyRoster);
        }
        validate_battle_sync_roster_identities(&players)?;
        Ok(Self {
            players,
            actions: BTreeMap::new(),
            state_hashes: BTreeMap::new(),
        })
    }

    pub fn from_lobby(lobby: &LinkLobby) -> Result<Self, BattleSyncError> {
        Self::new(lobby.player_ids())
    }

    pub fn players(&self) -> Vec<PlayerId> {
        self.players.iter().copied().collect()
    }

    pub fn insert_action(
        &mut self,
        action: BattleActionFrame,
    ) -> Result<InsertBattleActionResult, BattleSyncError> {
        action.validate()?;
        let (player_id, turn, battle_action, state_hash) = action.into_parts();
        if !self.players.contains(&player_id) {
            return Err(BattleSyncError::UnknownPlayer { player_id });
        }
        if let Some(existing_action) = self
            .actions
            .get(&turn)
            .and_then(|turn_actions| turn_actions.get(&player_id))
        {
            if existing_action != &battle_action {
                return Ok(InsertBattleActionResult::Conflict);
            }
            if let Some(existing_hash) = self
                .state_hashes
                .get(&turn)
                .and_then(|turn_hashes| turn_hashes.get(&player_id))
            {
                if existing_hash != &state_hash {
                    return Ok(InsertBattleActionResult::Conflict);
                }
                return Ok(InsertBattleActionResult::Duplicate);
            }
            self.state_hashes
                .entry(turn)
                .or_default()
                .insert(player_id, state_hash);
            return Ok(InsertBattleActionResult::Duplicate);
        }

        self.actions
            .entry(turn)
            .or_default()
            .insert(player_id, battle_action);
        self.state_hashes
            .entry(turn)
            .or_default()
            .insert(player_id, state_hash);
        Ok(InsertBattleActionResult::Inserted)
    }

    pub fn is_turn_ready(&self, turn: u64) -> bool {
        let Some(actions) = self.actions.get(&turn) else {
            return false;
        };
        let Some(state_hashes) = self.state_hashes.get(&turn) else {
            return false;
        };
        actions.len() == self.players.len()
            && state_hashes.len() == self.players.len()
            && self.players.iter().all(|player_id| {
                actions.contains_key(player_id) && state_hashes.contains_key(player_id)
            })
    }

    pub fn turn(&self, turn: u64) -> Option<BattleActionTurn> {
        if !self.is_turn_ready(turn) {
            return None;
        }
        let state_hashes = self.state_hashes.get(&turn)?.clone();
        BattleActionTurn::new(turn, self.actions.get(&turn)?.clone(), state_hashes).ok()
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

fn validate_battle_sync_roster_identities(
    players: &BTreeSet<PlayerId>,
) -> Result<(), BattleSyncError> {
    if players.contains(&0) {
        return Err(BattleSyncError::InvalidPlayerIdentity { player_id: 0 });
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockstepBuffer {
    players: BTreeSet<PlayerId>,
    inputs: BTreeMap<u64, BTreeMap<PlayerId, u8>>,
    checksums: BTreeMap<u64, BTreeMap<PlayerId, u32>>,
}

impl LockstepBuffer {
    pub fn new(players: impl IntoIterator<Item = PlayerId>) -> Result<Self, LockstepSyncError> {
        let players: BTreeSet<PlayerId> = players.into_iter().collect();
        if players.is_empty() {
            return Err(LockstepSyncError::EmptyRoster);
        }
        validate_lockstep_roster_identities(&players)?;
        Ok(Self {
            players,
            inputs: BTreeMap::new(),
            checksums: BTreeMap::new(),
        })
    }

    pub fn players(&self) -> Vec<PlayerId> {
        self.players.iter().copied().collect()
    }

    pub fn insert_input(
        &mut self,
        input: PlayerInputFrame,
    ) -> Result<InsertInputResult, LockstepSyncError> {
        input.validate()?;
        let (player_id, frame, joypad_mask) = input.into_parts();
        if !self.players.contains(&player_id) {
            return Err(LockstepSyncError::UnknownPlayer { player_id });
        }
        let frame_inputs = self.inputs.entry(frame).or_default();
        Ok(match frame_inputs.get(&player_id) {
            Some(existing) if *existing == joypad_mask => InsertInputResult::Duplicate,
            Some(_) => InsertInputResult::Conflict,
            None => {
                frame_inputs.insert(player_id, joypad_mask);
                InsertInputResult::Inserted
            }
        })
    }

    pub fn is_frame_ready(&self, frame: u64) -> bool {
        self.inputs
            .get(&frame)
            .map(|inputs| {
                inputs.len() == self.players.len()
                    && self
                        .players
                        .iter()
                        .all(|player_id| inputs.contains_key(player_id))
            })
            .unwrap_or(false)
    }

    pub fn frame(&self, frame: u64) -> Option<LockstepFrame> {
        if !self.is_frame_ready(frame) {
            return None;
        }
        LockstepFrame::new(frame, self.inputs.get(&frame)?.clone()).ok()
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
    ) -> Result<InsertChecksumResult, LockstepSyncError> {
        if player_id == 0 {
            return Err(LockstepSyncError::InvalidPlayerIdentity { player_id });
        }
        if !self.players.contains(&player_id) {
            return Err(LockstepSyncError::UnknownPlayer { player_id });
        }
        let frame_checksums = self.checksums.entry(checksum.frame()).or_default();
        Ok(match frame_checksums.get(&player_id) {
            Some(existing) if *existing == checksum.hash() => InsertChecksumResult::Duplicate,
            Some(_) => InsertChecksumResult::Conflict,
            None => {
                frame_checksums.insert(player_id, checksum.hash());
                InsertChecksumResult::Inserted
            }
        })
    }

    pub fn insert_checksum_frame(
        &mut self,
        checksum: StateChecksumFrame,
    ) -> Result<InsertChecksumResult, LockstepSyncError> {
        checksum.validate()?;
        self.insert_checksum(checksum.player_id(), checksum.checksum())
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

fn validate_lockstep_joypad_mask(mask: u8) -> Result<(), LockstepSyncError> {
    let directions = [B_PAD_RIGHT, B_PAD_LEFT, B_PAD_UP, B_PAD_DOWN]
        .into_iter()
        .filter(|direction| mask & *direction != 0)
        .count();
    if directions > 1 {
        return Err(LockstepSyncError::ConflictingJoypadDirections { mask });
    }
    Ok(())
}

fn validate_lockstep_roster_identities(
    players: &BTreeSet<PlayerId>,
) -> Result<(), LockstepSyncError> {
    if players.contains(&0) {
        return Err(LockstepSyncError::InvalidPlayerIdentity { player_id: 0 });
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DeterministicLockstep {
    local_player_id: PlayerId,
    players: BTreeSet<PlayerId>,
    next_frame: u64,
    previous_local_joypad_mask: u8,
}

impl<'de> Deserialize<'de> for DeterministicLockstep {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawDeterministicLockstep {
            local_player_id: PlayerId,
            players: BTreeSet<PlayerId>,
            next_frame: u64,
            previous_local_joypad_mask: u8,
        }

        let raw = RawDeterministicLockstep::deserialize(deserializer)?;
        let lockstep = Self {
            local_player_id: raw.local_player_id,
            players: raw.players,
            next_frame: raw.next_frame,
            previous_local_joypad_mask: raw.previous_local_joypad_mask,
        };
        lockstep.validate().map_err(serde::de::Error::custom)?;
        Ok(lockstep)
    }
}

impl DeterministicLockstep {
    pub fn new(
        players: impl IntoIterator<Item = PlayerId>,
        local_player_id: PlayerId,
    ) -> Result<Self, LockstepSyncError> {
        let lockstep = Self {
            local_player_id,
            players: players.into_iter().collect(),
            next_frame: 0,
            previous_local_joypad_mask: 0,
        };
        lockstep.validate()?;
        Ok(lockstep)
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn new_unchecked_for_tests(
        local_player_id: PlayerId,
        players: BTreeSet<PlayerId>,
        next_frame: u64,
        previous_local_joypad_mask: u8,
    ) -> Self {
        Self {
            local_player_id,
            players,
            next_frame,
            previous_local_joypad_mask,
        }
    }

    pub const fn local_player_id(&self) -> PlayerId {
        self.local_player_id
    }

    pub const fn next_frame(&self) -> u64 {
        self.next_frame
    }

    pub const fn previous_local_joypad_mask(&self) -> u8 {
        self.previous_local_joypad_mask
    }

    pub fn validate(&self) -> Result<(), LockstepSyncError> {
        if self.players.is_empty() {
            return Err(LockstepSyncError::EmptyRoster);
        }
        validate_lockstep_roster_identities(&self.players)?;
        if self.local_player_id == 0 {
            return Err(LockstepSyncError::InvalidPlayerIdentity {
                player_id: self.local_player_id,
            });
        }
        if !self.players.contains(&self.local_player_id) {
            return Err(LockstepSyncError::UnknownPlayer {
                player_id: self.local_player_id,
            });
        }
        validate_lockstep_joypad_mask(self.previous_local_joypad_mask)
    }

    pub fn from_lobby(
        lobby: &LinkLobby,
        local_player_id: PlayerId,
    ) -> Result<Self, LockstepSyncError> {
        Self::new(lobby.player_ids(), local_player_id)
    }

    pub fn players(&self) -> Vec<PlayerId> {
        self.players.iter().copied().collect()
    }

    pub fn apply_frame(
        &mut self,
        frame: LockstepFrame,
    ) -> Result<AppliedLockstepFrame, LockstepSyncError> {
        self.validate()?;
        self.validate_frame(&frame)?;
        let local_joypad_mask = frame.joypad_mask_for(self.local_player_id).ok_or(
            LockstepSyncError::MissingPlayerInput {
                frame: frame.frame(),
                player_id: self.local_player_id,
            },
        )?;
        let local_pressed_mask =
            (local_joypad_mask ^ self.previous_local_joypad_mask) & local_joypad_mask;
        let mut ordered_inputs = Vec::with_capacity(self.players.len());
        for player_id in &self.players {
            ordered_inputs.push(frame.joypad_mask_for(*player_id).ok_or(
                LockstepSyncError::MissingPlayerInput {
                    frame: frame.frame(),
                    player_id: *player_id,
                },
            )?);
        }
        self.previous_local_joypad_mask = local_joypad_mask;
        self.next_frame =
            self.next_frame
                .checked_add(1)
                .ok_or(LockstepSyncError::FrameCursorOverflow {
                    frame: self.next_frame,
                })?;
        Ok(AppliedLockstepFrame::new(
            frame.frame(),
            self.local_player_id,
            local_joypad_mask,
            local_pressed_mask,
            ordered_inputs,
        ))
    }

    fn validate_frame(&self, frame: &LockstepFrame) -> Result<(), LockstepSyncError> {
        if frame.frame() != self.next_frame {
            return Err(LockstepSyncError::FrameOutOfOrder {
                expected: self.next_frame,
                actual: frame.frame(),
            });
        }
        for player_id in frame.inputs().keys() {
            if !self.players.contains(player_id) {
                return Err(LockstepSyncError::NonRosterPlayerInput {
                    frame: frame.frame(),
                    player_id: *player_id,
                });
            }
        }
        frame.validate_inputs()?;
        for player_id in &self.players {
            if !frame.inputs().contains_key(player_id) {
                return Err(LockstepSyncError::MissingPlayerInput {
                    frame: frame.frame(),
                    player_id: *player_id,
                });
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AppliedLockstepFrame {
    frame: u64,
    local_player_id: PlayerId,
    local_joypad_mask: u8,
    local_pressed_mask: u8,
    ordered_inputs: Vec<u8>,
}

impl AppliedLockstepFrame {
    pub fn new(
        frame: u64,
        local_player_id: PlayerId,
        local_joypad_mask: u8,
        local_pressed_mask: u8,
        ordered_inputs: Vec<u8>,
    ) -> Self {
        Self {
            frame,
            local_player_id,
            local_joypad_mask,
            local_pressed_mask,
            ordered_inputs,
        }
    }

    pub const fn frame(&self) -> u64 {
        self.frame
    }

    pub const fn local_player_id(&self) -> PlayerId {
        self.local_player_id
    }

    pub const fn local_joypad_mask(&self) -> u8 {
        self.local_joypad_mask
    }

    pub const fn local_pressed_mask(&self) -> u8 {
        self.local_pressed_mask
    }

    pub fn ordered_inputs(&self) -> &[u8] {
        &self.ordered_inputs
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum LinkMessage {
    Hello(LinkHello),
    RngInit { state: BattleRngState },
    SessionRngInit(SessionBattleRngInitFrame),
    BattleAction(BattleActionFrame),
    SessionBattleAction(SessionBattleActionFrame),
    TradeOffer(TradeOffer),
    SessionTradeOffer(SessionTradeOffer),
    TradeConfirmation(TradeConfirmation),
    SessionTradeConfirmation(SessionTradeConfirmation),
    LinkByte(LinkByteFrame),
    SessionLinkByte(SessionLinkByteFrame),
    LinkClockSync(LinkClockSyncFrame),
    SessionLinkClockSync(SessionLinkClockSyncFrame),
    Input(PlayerInputFrame),
    SessionInput(SessionPlayerInputFrame),
    MenuChoice(MenuChoiceFrame),
    SessionMenuChoice(SessionMenuChoiceFrame),
    MenuChoiceResult(MenuChoiceResultFrame),
    SessionMenuChoiceResult(SessionMenuChoiceResultFrame),
    InputJournal(DeterministicInputJournalFrame),
    DeterministicReplay(DeterministicReplayBundle),
    SaveResumeReplay(SaveResumeReplayBundle),
    SaveSummary(SaveGameSummary),
    SessionSaveSummary(SessionSaveSummaryFrame),
    SaveCheckpoint(SaveCheckpointFrame),
    SessionSaveCheckpoint(SessionSaveCheckpointFrame),
    StateHash(StateChecksumFrame),
    SessionStateHash(SessionStateChecksumFrame),
    CommandChecksum(CommandChecksumResult),
    RuntimeCommand(RuntimeCommandFrame),
    SessionRuntimeCommand(SessionRuntimeCommandFrame),
    RuntimeCommandResult(RuntimeCommandResultFrame),
    SessionRuntimeCommandResult(SessionRuntimeCommandResultFrame),
    Presence(OverworldPresence),
    SessionPresence(SessionOverworldPresence),
    InteractionRequest(MultiplayerInteractionRequest),
    SessionInteractionRequest(SessionMultiplayerInteractionRequest),
    InteractionResponse(MultiplayerInteractionResponse),
    SessionInteractionResponse(SessionMultiplayerInteractionResponse),
    Disconnect { player_id: PlayerId, reason: String },
    SessionDisconnect(SessionDisconnectFrame),
}

impl<'de> Deserialize<'de> for LinkMessage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
        enum RawLinkMessage {
            Hello(LinkHello),
            RngInit { state: BattleRngState },
            SessionRngInit(SessionBattleRngInitFrame),
            BattleAction(BattleActionFrame),
            SessionBattleAction(SessionBattleActionFrame),
            TradeOffer(TradeOffer),
            SessionTradeOffer(SessionTradeOffer),
            TradeConfirmation(TradeConfirmation),
            SessionTradeConfirmation(SessionTradeConfirmation),
            LinkByte(LinkByteFrame),
            SessionLinkByte(SessionLinkByteFrame),
            LinkClockSync(LinkClockSyncFrame),
            SessionLinkClockSync(SessionLinkClockSyncFrame),
            Input(PlayerInputFrame),
            SessionInput(SessionPlayerInputFrame),
            MenuChoice(MenuChoiceFrame),
            SessionMenuChoice(SessionMenuChoiceFrame),
            MenuChoiceResult(MenuChoiceResultFrame),
            SessionMenuChoiceResult(SessionMenuChoiceResultFrame),
            InputJournal(DeterministicInputJournalFrame),
            DeterministicReplay(DeterministicReplayBundle),
            SaveResumeReplay(SaveResumeReplayBundle),
            SaveSummary(SaveGameSummary),
            SessionSaveSummary(SessionSaveSummaryFrame),
            SaveCheckpoint(SaveCheckpointFrame),
            SessionSaveCheckpoint(SessionSaveCheckpointFrame),
            StateHash(StateChecksumFrame),
            SessionStateHash(SessionStateChecksumFrame),
            CommandChecksum(CommandChecksumResult),
            RuntimeCommand(RuntimeCommandFrame),
            SessionRuntimeCommand(SessionRuntimeCommandFrame),
            RuntimeCommandResult(RuntimeCommandResultFrame),
            SessionRuntimeCommandResult(SessionRuntimeCommandResultFrame),
            Presence(OverworldPresence),
            SessionPresence(SessionOverworldPresence),
            InteractionRequest(MultiplayerInteractionRequest),
            SessionInteractionRequest(SessionMultiplayerInteractionRequest),
            InteractionResponse(MultiplayerInteractionResponse),
            SessionInteractionResponse(SessionMultiplayerInteractionResponse),
            Disconnect { player_id: PlayerId, reason: String },
            SessionDisconnect(SessionDisconnectFrame),
        }

        let raw = RawLinkMessage::deserialize(deserializer)?;
        let message = match raw {
            RawLinkMessage::Hello(hello) => Self::Hello(hello),
            RawLinkMessage::RngInit { state } => Self::RngInit { state },
            RawLinkMessage::SessionRngInit(frame) => Self::SessionRngInit(frame),
            RawLinkMessage::BattleAction(action) => Self::BattleAction(action),
            RawLinkMessage::SessionBattleAction(action) => Self::SessionBattleAction(action),
            RawLinkMessage::TradeOffer(offer) => Self::TradeOffer(offer),
            RawLinkMessage::SessionTradeOffer(offer) => Self::SessionTradeOffer(offer),
            RawLinkMessage::TradeConfirmation(confirmation) => {
                Self::TradeConfirmation(confirmation)
            }
            RawLinkMessage::SessionTradeConfirmation(confirmation) => {
                Self::SessionTradeConfirmation(confirmation)
            }
            RawLinkMessage::LinkByte(frame) => Self::LinkByte(frame),
            RawLinkMessage::SessionLinkByte(frame) => Self::SessionLinkByte(frame),
            RawLinkMessage::LinkClockSync(frame) => Self::LinkClockSync(frame),
            RawLinkMessage::SessionLinkClockSync(frame) => Self::SessionLinkClockSync(frame),
            RawLinkMessage::Input(input) => Self::Input(input),
            RawLinkMessage::SessionInput(input) => Self::SessionInput(input),
            RawLinkMessage::MenuChoice(choice) => Self::MenuChoice(choice),
            RawLinkMessage::SessionMenuChoice(choice) => Self::SessionMenuChoice(choice),
            RawLinkMessage::MenuChoiceResult(result) => Self::MenuChoiceResult(result),
            RawLinkMessage::SessionMenuChoiceResult(result) => {
                Self::SessionMenuChoiceResult(result)
            }
            RawLinkMessage::InputJournal(journal) => Self::InputJournal(journal),
            RawLinkMessage::DeterministicReplay(bundle) => Self::DeterministicReplay(bundle),
            RawLinkMessage::SaveResumeReplay(bundle) => Self::SaveResumeReplay(bundle),
            RawLinkMessage::SaveSummary(summary) => Self::SaveSummary(summary),
            RawLinkMessage::SessionSaveSummary(summary) => Self::SessionSaveSummary(summary),
            RawLinkMessage::SaveCheckpoint(checkpoint) => Self::SaveCheckpoint(checkpoint),
            RawLinkMessage::SessionSaveCheckpoint(checkpoint) => {
                Self::SessionSaveCheckpoint(checkpoint)
            }
            RawLinkMessage::StateHash(frame) => Self::StateHash(frame),
            RawLinkMessage::SessionStateHash(frame) => Self::SessionStateHash(frame),
            RawLinkMessage::CommandChecksum(result) => Self::CommandChecksum(result),
            RawLinkMessage::RuntimeCommand(command) => Self::RuntimeCommand(command),
            RawLinkMessage::SessionRuntimeCommand(command) => Self::SessionRuntimeCommand(command),
            RawLinkMessage::RuntimeCommandResult(result) => Self::RuntimeCommandResult(result),
            RawLinkMessage::SessionRuntimeCommandResult(result) => {
                Self::SessionRuntimeCommandResult(result)
            }
            RawLinkMessage::Presence(presence) => Self::Presence(presence),
            RawLinkMessage::SessionPresence(presence) => Self::SessionPresence(presence),
            RawLinkMessage::InteractionRequest(request) => Self::InteractionRequest(request),
            RawLinkMessage::SessionInteractionRequest(request) => {
                Self::SessionInteractionRequest(request)
            }
            RawLinkMessage::InteractionResponse(response) => Self::InteractionResponse(response),
            RawLinkMessage::SessionInteractionResponse(response) => {
                Self::SessionInteractionResponse(response)
            }
            RawLinkMessage::Disconnect { player_id, reason } => {
                Self::Disconnect { player_id, reason }
            }
            RawLinkMessage::SessionDisconnect(frame) => Self::SessionDisconnect(frame),
        };
        message.validate().map_err(serde::de::Error::custom)?;
        Ok(message)
    }
}

impl LinkMessage {
    pub fn message_type(&self) -> &'static str {
        match self {
            Self::Hello(_) => "hello",
            Self::RngInit { .. } => "rng_init",
            Self::SessionRngInit(_) => "session_rng_init",
            Self::BattleAction(_) => "battle_action",
            Self::SessionBattleAction(_) => "session_battle_action",
            Self::TradeOffer(_) => "trade_offer",
            Self::SessionTradeOffer(_) => "session_trade_offer",
            Self::TradeConfirmation(_) => "trade_confirmation",
            Self::SessionTradeConfirmation(_) => "session_trade_confirmation",
            Self::LinkByte(_) => "link_byte",
            Self::SessionLinkByte(_) => "session_link_byte",
            Self::LinkClockSync(_) => "link_clock_sync",
            Self::SessionLinkClockSync(_) => "session_link_clock_sync",
            Self::Input(_) => "input",
            Self::SessionInput(_) => "session_input",
            Self::MenuChoice(_) => "menu_choice",
            Self::SessionMenuChoice(_) => "session_menu_choice",
            Self::MenuChoiceResult(_) => "menu_choice_result",
            Self::SessionMenuChoiceResult(_) => "session_menu_choice_result",
            Self::InputJournal(_) => "input_journal",
            Self::DeterministicReplay(_) => "deterministic_replay",
            Self::SaveResumeReplay(_) => "save_resume_replay",
            Self::SaveSummary(_) => "save_summary",
            Self::SessionSaveSummary(_) => "session_save_summary",
            Self::SaveCheckpoint(_) => "save_checkpoint",
            Self::SessionSaveCheckpoint(_) => "session_save_checkpoint",
            Self::StateHash(_) => "state_hash",
            Self::SessionStateHash(_) => "session_state_hash",
            Self::CommandChecksum(_) => "command_checksum",
            Self::RuntimeCommand(_) => "runtime_command",
            Self::SessionRuntimeCommand(_) => "session_runtime_command",
            Self::RuntimeCommandResult(_) => "runtime_command_result",
            Self::SessionRuntimeCommandResult(_) => "session_runtime_command_result",
            Self::Presence(_) => "presence",
            Self::SessionPresence(_) => "session_presence",
            Self::InteractionRequest(_) => "interaction_request",
            Self::SessionInteractionRequest(_) => "session_interaction_request",
            Self::InteractionResponse(_) => "interaction_response",
            Self::SessionInteractionResponse(_) => "session_interaction_response",
            Self::Disconnect { .. } => "disconnect",
            Self::SessionDisconnect(_) => "session_disconnect",
        }
    }

    pub fn session(&self) -> Option<&LinkSessionIdentity> {
        match self {
            Self::Hello(hello) => Some(hello.session()),
            Self::SessionRngInit(frame) => Some(frame.session()),
            Self::SessionBattleAction(frame) => Some(frame.session()),
            Self::SessionTradeOffer(frame) => Some(frame.session()),
            Self::SessionTradeConfirmation(frame) => Some(frame.session()),
            Self::SessionLinkByte(frame) => Some(frame.session()),
            Self::SessionLinkClockSync(frame) => Some(frame.session()),
            Self::SessionInput(frame) => Some(frame.session()),
            Self::SessionMenuChoice(frame) => Some(frame.session()),
            Self::SessionMenuChoiceResult(frame) => Some(frame.session()),
            Self::InputJournal(frame) => Some(frame.journal().session()),
            Self::DeterministicReplay(bundle) => Some(bundle.input_journal().journal().session()),
            Self::SaveResumeReplay(bundle) => Some(bundle.checkpoint().session()),
            Self::SessionSaveSummary(frame) => Some(frame.session()),
            Self::SessionSaveCheckpoint(frame) => Some(frame.session()),
            Self::SessionStateHash(frame) => Some(frame.session()),
            Self::SessionRuntimeCommand(frame) => Some(frame.session()),
            Self::SessionRuntimeCommandResult(frame) => Some(frame.session()),
            Self::SessionPresence(frame) => Some(frame.session()),
            Self::SessionInteractionRequest(frame) => Some(frame.session()),
            Self::SessionInteractionResponse(frame) => Some(frame.session()),
            Self::SessionDisconnect(frame) => Some(frame.session()),
            Self::RngInit { .. }
            | Self::BattleAction(_)
            | Self::TradeOffer(_)
            | Self::TradeConfirmation(_)
            | Self::LinkByte(_)
            | Self::LinkClockSync(_)
            | Self::Input(_)
            | Self::MenuChoice(_)
            | Self::MenuChoiceResult(_)
            | Self::SaveSummary(_)
            | Self::SaveCheckpoint(_)
            | Self::StateHash(_)
            | Self::CommandChecksum(_)
            | Self::RuntimeCommand(_)
            | Self::RuntimeCommandResult(_)
            | Self::Presence(_)
            | Self::InteractionRequest(_)
            | Self::InteractionResponse(_)
            | Self::Disconnect { .. } => None,
        }
    }

    pub fn is_session_bound(&self) -> bool {
        self.session().is_some()
    }

    pub fn require_session(&self) -> Result<&LinkSessionIdentity, MultiplayerMessageError> {
        self.session()
            .ok_or(MultiplayerMessageError::MissingSessionIdentity {
                message_type: self.message_type(),
            })
    }

    pub fn validate_session_identity(
        &self,
        expected: &LinkSessionIdentity,
    ) -> Result<&LinkSessionIdentity, MultiplayerMessageError> {
        let actual = self.require_session()?;
        validate_link_session_identity(expected, actual).map_err(|error| {
            MultiplayerMessageError::SessionIdentityMismatch {
                message_type: self.message_type(),
                message: error.to_string(),
            }
        })?;
        Ok(actual)
    }

    pub fn validate(&self) -> Result<(), MultiplayerMessageError> {
        match self {
            Self::Hello(hello) => {
                hello
                    .validate()
                    .map_err(|error| MultiplayerMessageError::InvalidLinkHandshake {
                        message: error.to_string(),
                    })
            }
            Self::SessionRngInit(frame) => frame
                .validate()
                .map_err(|message| MultiplayerMessageError::InvalidBattleRng { message }),
            Self::BattleAction(action) => {
                action
                    .validate()
                    .map_err(|error| MultiplayerMessageError::InvalidBattleAction {
                        message: error.to_string(),
                    })
            }
            Self::SessionBattleAction(action) => action
                .validate()
                .map_err(|message| MultiplayerMessageError::InvalidBattleAction { message }),
            Self::TradeOffer(offer) => {
                offer
                    .validate()
                    .map_err(|error| MultiplayerMessageError::InvalidTradeFrame {
                        message: error.to_string(),
                    })
            }
            Self::SessionTradeOffer(offer) => offer
                .validate()
                .map_err(|message| MultiplayerMessageError::InvalidTradeFrame { message }),
            Self::TradeConfirmation(confirmation) => confirmation.validate().map_err(|error| {
                MultiplayerMessageError::InvalidTradeFrame {
                    message: error.to_string(),
                }
            }),
            Self::SessionTradeConfirmation(confirmation) => confirmation
                .validate()
                .map_err(|message| MultiplayerMessageError::InvalidTradeFrame { message }),
            Self::LinkByte(frame) => {
                frame
                    .validate()
                    .map_err(|error| MultiplayerMessageError::InvalidLinkCableFrame {
                        message: error.to_string(),
                    })
            }
            Self::SessionLinkByte(frame) => frame
                .validate()
                .map_err(|message| MultiplayerMessageError::InvalidLinkCableFrame { message }),
            Self::LinkClockSync(sync) => {
                sync.validate()
                    .map_err(|error| MultiplayerMessageError::InvalidLinkCableFrame {
                        message: error.to_string(),
                    })
            }
            Self::SessionLinkClockSync(sync) => sync
                .validate()
                .map_err(|message| MultiplayerMessageError::InvalidLinkCableFrame { message }),
            Self::Input(input) => {
                input
                    .validate()
                    .map_err(|error| MultiplayerMessageError::InvalidLockstepFrame {
                        message: error.to_string(),
                    })
            }
            Self::SessionInput(input) => input
                .validate()
                .map_err(|message| MultiplayerMessageError::InvalidLockstepFrame { message }),
            Self::MenuChoice(choice) => choice.validate(),
            Self::SessionMenuChoice(choice) => choice
                .validate()
                .map_err(|message| MultiplayerMessageError::InvalidLockstepFrame { message }),
            Self::MenuChoiceResult(result) => {
                result
                    .validate()
                    .map_err(|error| MultiplayerMessageError::InvalidRuntimeCommand {
                        message: error.to_string(),
                    })
            }
            Self::SessionMenuChoiceResult(result) => result
                .validate()
                .map_err(|message| MultiplayerMessageError::InvalidRuntimeCommand { message }),
            Self::InputJournal(journal_frame) => journal_frame.validate().map_err(|error| {
                MultiplayerMessageError::InvalidLockstepFrame {
                    message: error.to_string(),
                }
            }),
            Self::DeterministicReplay(bundle) => {
                bundle
                    .validate()
                    .map_err(|error| MultiplayerMessageError::InvalidLockstepFrame {
                        message: error.to_string(),
                    })
            }
            Self::SaveResumeReplay(bundle) => {
                bundle
                    .validate()
                    .map_err(|error| MultiplayerMessageError::InvalidLockstepFrame {
                        message: error.to_string(),
                    })
            }
            Self::SaveSummary(summary) => {
                summary
                    .validate()
                    .map_err(|error| MultiplayerMessageError::InvalidLinkHandshake {
                        message: error.to_string(),
                    })
            }
            Self::SessionSaveSummary(summary) => {
                summary
                    .validate()
                    .map_err(|error| MultiplayerMessageError::InvalidLinkHandshake {
                        message: error.to_string(),
                    })
            }
            Self::SaveCheckpoint(checkpoint) => checkpoint.validate().map_err(|error| {
                MultiplayerMessageError::InvalidLockstepFrame {
                    message: error.to_string(),
                }
            }),
            Self::SessionSaveCheckpoint(checkpoint) => checkpoint.validate().map_err(|error| {
                MultiplayerMessageError::InvalidLockstepFrame {
                    message: error.to_string(),
                }
            }),
            Self::Presence(presence) => presence.validate(),
            Self::SessionPresence(presence) => presence
                .validate()
                .map_err(|message| MultiplayerMessageError::InvalidLinkHandshake { message }),
            Self::InteractionRequest(request) => request.validate(),
            Self::SessionInteractionRequest(request) => request
                .validate()
                .map_err(|message| MultiplayerMessageError::InvalidLinkHandshake { message }),
            Self::InteractionResponse(response) => response.validate(),
            Self::SessionInteractionResponse(response) => response
                .validate()
                .map_err(|message| MultiplayerMessageError::InvalidLinkHandshake { message }),
            Self::Disconnect { player_id, reason } => {
                validate_disconnect_payload(*player_id, reason)
            }
            Self::SessionDisconnect(frame) => frame
                .validate()
                .map_err(|message| MultiplayerMessageError::InvalidLinkHandshake { message }),
            Self::RngInit { state } => {
                state
                    .validate()
                    .map_err(|error| MultiplayerMessageError::InvalidBattleRng {
                        message: error.to_string(),
                    })
            }
            Self::StateHash(frame) => {
                frame
                    .validate()
                    .map_err(|error| MultiplayerMessageError::InvalidLockstepFrame {
                        message: error.to_string(),
                    })
            }
            Self::SessionStateHash(frame) => frame
                .validate()
                .map_err(|message| MultiplayerMessageError::InvalidLockstepFrame { message }),
            Self::CommandChecksum(result) => {
                validate_command_checksum_events(&result.events)?;
                result.checksum.validate().map_err(|error| {
                    MultiplayerMessageError::InvalidLockstepFrame {
                        message: error.to_string(),
                    }
                })
            }
            Self::RuntimeCommand(command) => {
                command
                    .validate()
                    .map_err(|error| MultiplayerMessageError::InvalidRuntimeCommand {
                        message: error.to_string(),
                    })
            }
            Self::SessionRuntimeCommand(command) => {
                command
                    .validate()
                    .map_err(|error| MultiplayerMessageError::InvalidRuntimeCommand {
                        message: error.to_string(),
                    })
            }
            Self::RuntimeCommandResult(result) => {
                result
                    .validate()
                    .map_err(|error| MultiplayerMessageError::InvalidRuntimeCommand {
                        message: error.to_string(),
                    })
            }
            Self::SessionRuntimeCommandResult(result) => {
                result
                    .validate()
                    .map_err(|error| MultiplayerMessageError::InvalidRuntimeCommand {
                        message: error.to_string(),
                    })
            }
        }
    }
}

pub fn encode_link_message_bytes(
    message: &LinkMessage,
) -> Result<Vec<u8>, MultiplayerMessageError> {
    message.validate()?;
    let encoded = serde_json::to_vec(message)
        .map_err(|error| MultiplayerMessageError::BinaryEncode(error.to_string()))?;
    if encoded.len() > u32::MAX as usize {
        return Err(MultiplayerMessageError::BinaryEncode(
            "encoded link message exceeds binary payload length field".to_string(),
        ));
    }
    let mut bytes = Vec::with_capacity(LINK_MESSAGE_HEADER_LEN + encoded.len());
    bytes.extend_from_slice(LINK_MESSAGE_MAGIC);
    bytes.extend_from_slice(&LINK_PROTOCOL_VERSION.to_be_bytes());
    bytes.extend_from_slice(&(encoded.len() as u32).to_be_bytes());
    bytes.extend_from_slice(&fnv1a32_bytes(&encoded).to_be_bytes());
    bytes.extend_from_slice(&encoded);
    Ok(bytes)
}

pub fn decode_link_message_bytes(bytes: &[u8]) -> Result<LinkMessage, MultiplayerMessageError> {
    if bytes.is_empty() {
        return Err(MultiplayerMessageError::EmptyBinaryPayload);
    }
    if bytes.len() < LINK_MESSAGE_HEADER_LEN
        || &bytes[..LINK_MESSAGE_MAGIC.len()] != LINK_MESSAGE_MAGIC
    {
        return Err(MultiplayerMessageError::InvalidBinaryMagic);
    }
    let version = u16::from_be_bytes([
        bytes[LINK_MESSAGE_VERSION_OFFSET],
        bytes[LINK_MESSAGE_VERSION_OFFSET + 1],
    ]);
    if version != LINK_PROTOCOL_VERSION {
        return Err(MultiplayerMessageError::BinaryVersionMismatch {
            expected: LINK_PROTOCOL_VERSION,
            actual: version,
        });
    }
    let expected_len = u32::from_be_bytes([
        bytes[LINK_MESSAGE_PAYLOAD_LENGTH_OFFSET],
        bytes[LINK_MESSAGE_PAYLOAD_LENGTH_OFFSET + 1],
        bytes[LINK_MESSAGE_PAYLOAD_LENGTH_OFFSET + 2],
        bytes[LINK_MESSAGE_PAYLOAD_LENGTH_OFFSET + 3],
    ]) as usize;
    let expected_hash = u32::from_be_bytes([
        bytes[LINK_MESSAGE_PAYLOAD_HASH_OFFSET],
        bytes[LINK_MESSAGE_PAYLOAD_HASH_OFFSET + 1],
        bytes[LINK_MESSAGE_PAYLOAD_HASH_OFFSET + 2],
        bytes[LINK_MESSAGE_PAYLOAD_HASH_OFFSET + 3],
    ]);
    let payload = &bytes[LINK_MESSAGE_HEADER_LEN..];
    if payload.len() != expected_len {
        return Err(MultiplayerMessageError::BinaryLengthMismatch {
            expected: expected_len,
            actual: payload.len(),
        });
    }
    let actual_hash = fnv1a32_bytes(payload);
    if actual_hash != expected_hash {
        return Err(MultiplayerMessageError::BinaryHashMismatch {
            expected: expected_hash,
            actual: actual_hash,
        });
    }
    let message: LinkMessage = serde_json::from_slice(payload)
        .map_err(|error| MultiplayerMessageError::BinaryDecode(error.to_string()))?;
    message.validate()?;
    Ok(message)
}

pub fn decode_link_message_bytes_for_session(
    bytes: &[u8],
    expected_session: &LinkSessionIdentity,
) -> Result<LinkMessage, MultiplayerMessageError> {
    let message = decode_link_message_bytes(bytes)?;
    message.validate_session_identity(expected_session)?;
    Ok(message)
}

fn link_message_binary_config() -> impl bincode::config::Config {
    bincode::config::standard()
        .with_little_endian()
        .with_fixed_int_encoding()
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
        if entry.user_id() == local_user_id || entry.is_stale(now_ms, stale_ms) {
            continue;
        }
        match by_user.get(entry.user_id()) {
            Some(previous) if previous.updated_at_ms() >= entry.updated_at_ms() => {}
            _ => {
                by_user.insert(entry.user_id(), entry);
            }
        }
    }
    by_user.into_values().cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{BaseStats, Dv, PokemonSpecies};

    fn test_modpack(id: &str, hash: &str) -> SaveModpackIdentity {
        SaveModpackIdentity::new(id, hash).expect("modpack identity")
    }

    fn pack_content_hash() -> &'static str {
        "0102030401020304010203040102030401020304010203040102030401020304"
    }

    fn test_session(
        id: &str,
        modpack: SaveModpackIdentity,
    ) -> Result<LinkSessionIdentity, LinkHandshakeError> {
        LinkSessionIdentity::new(id, modpack, pack_content_hash())
    }

    fn test_hello(
        id: &str,
        modpack: SaveModpackIdentity,
        player: PlayerIdentity,
    ) -> Result<LinkHello, LinkHandshakeError> {
        LinkHello::new(id, modpack, pack_content_hash(), player)
    }

    fn test_player(id: PlayerId) -> PlayerIdentity {
        PlayerIdentity::new(id, format!("P{id}")).expect("player")
    }

    fn save_summary(
        modpack: SaveModpackIdentity,
        pack_content_hash: &str,
        frame: u64,
    ) -> SaveGameSummary {
        save_summary_with_hash(modpack, pack_content_hash, frame, 0xaabb_ccdd)
    }

    fn save_summary_with_hash(
        modpack: SaveModpackIdentity,
        pack_content_hash: &str,
        frame: u64,
        state_hash: u32,
    ) -> SaveGameSummary {
        serde_json::from_value(serde_json::json!({
            "format_version": crate::save::SAVE_FORMAT_VERSION,
            "modpack": {
                "id": modpack.id(),
                "hash": modpack.hash()
            },
            "pack_content_hash": pack_content_hash,
            "created_frame": frame,
            "saved_frame": frame,
            "state_frame": frame,
            "state_hash": state_hash
        }))
        .expect("save summary")
    }

    fn confirmation(trade_id: &str, player_id: PlayerId, confirm: bool) -> TradeConfirmation {
        TradeConfirmation::new(trade_id, player_id, confirm).expect("trade confirmation")
    }

    fn pokemon(id: &str, item: Option<&str>) -> Pokemon {
        let int_id = id.bytes().map(u16::from).sum();
        let mut species = PokemonSpecies::new_for_tests(id, BaseStats::new(45, 49, 49, 45, 65, 65));
        species.int_id = int_id;
        let mut pokemon = Pokemon::new_for_tests(species, 12, Dv::from_non_hp(1, 2, 3, 4));
        pokemon.item = item.map(str::to_string);
        pokemon.original_trainer_name = format!("{id}_OT");
        pokemon.original_trainer_id = int_id;
        pokemon
    }

    fn party_with(slot: usize, pokemon: Pokemon) -> Party {
        let mut party = Party::default();
        party.pokemon[slot] = Some(pokemon);
        party
    }

    #[test]
    fn link_messages_are_serializable_for_transport_neutral_netcode() {
        let message =
            LinkMessage::Input(PlayerInputFrame::new(2, Frame(144), 0b1001_0000).expect("input"));
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
    fn link_message_reports_exact_session_identity_for_bound_transport_messages() {
        let session = test_session("session-1", test_modpack("core-modular", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd"))
            .expect("session identity");
        let hello = LinkMessage::Hello(
            LinkHello::from_session(session.clone(), test_player(1)).expect("hello"),
        );
        let input = LinkMessage::SessionInput(
            SessionPlayerInputFrame::new(
                session.clone(),
                PlayerInputFrame::new(2, Frame(144), 0b1001_0000).expect("input"),
            )
            .expect("bound input"),
        );
        let state_hash = LinkMessage::SessionStateHash(
            SessionStateChecksumFrame::new(
                session.clone(),
                StateChecksumFrame::new(2, Frame(144), 0xaabb_ccdd),
            )
            .expect("bound state hash"),
        );
        let disconnect = LinkMessage::SessionDisconnect(
            SessionDisconnectFrame::new(session.clone(), 2, "closed").expect("disconnect"),
        );

        for message in [hello, input, state_hash, disconnect] {
            assert!(message.is_session_bound(), "{message:?}");
            assert_eq!(message.session(), Some(&session), "{message:?}");
            assert_eq!(message.require_session(), Ok(&session), "{message:?}");
            assert_eq!(
                message.validate_session_identity(&session),
                Ok(&session),
                "{message:?}"
            );
        }
    }

    #[test]
    fn link_message_reports_raw_transport_messages_as_unbound() {
        let messages = [
            LinkMessage::Input(PlayerInputFrame::new(2, Frame(144), 0b1001_0000).expect("input")),
            LinkMessage::StateHash(StateChecksumFrame::new(2, Frame(144), 0xaabb_ccdd)),
            LinkMessage::Disconnect {
                player_id: 2,
                reason: "closed".to_string(),
            },
        ];

        for message in messages {
            assert!(!message.is_session_bound(), "{message:?}");
            assert_eq!(message.session(), None, "{message:?}");
            assert_eq!(
                message.require_session(),
                Err(MultiplayerMessageError::MissingSessionIdentity {
                    message_type: message.message_type(),
                }),
                "{message:?}"
            );
            assert_eq!(
                message.validate_session_identity(
                    &test_session("session-1", test_modpack("core-modular", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd"))
                        .expect("session")
                ),
                Err(MultiplayerMessageError::MissingSessionIdentity {
                    message_type: message.message_type(),
                }),
                "{message:?}"
            );
        }
    }

    #[test]
    fn link_message_rejects_session_identity_mismatches_without_pack_fallback() {
        let expected = test_session("session-1", test_modpack("core-modular", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd"))
            .expect("expected session");
        let other_pack = test_session("session-1", test_modpack("core-modular", "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"))
            .expect("other pack session");
        let message = LinkMessage::SessionInput(
            SessionPlayerInputFrame::new(
                other_pack,
                PlayerInputFrame::new(2, Frame(144), 0b1001_0000).expect("input"),
            )
            .expect("bound input"),
        );

        assert!(matches!(
            message.validate_session_identity(&expected),
            Err(MultiplayerMessageError::SessionIdentityMismatch {
                message_type: "session_input",
                message,
            }) if message.contains("modpack hash")
        ));
    }

    #[test]
    fn link_message_reports_stable_transport_message_type_labels() {
        assert_eq!(
            LinkMessage::Input(PlayerInputFrame::new(2, Frame(144), 0b1001_0000).expect("input"))
                .message_type(),
            "input"
        );
        assert_eq!(
            LinkMessage::SessionDisconnect(
                SessionDisconnectFrame::new(
                    test_session("session-1", test_modpack("core-modular", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd"))
                        .expect("session"),
                    2,
                    "closed",
                )
                .expect("disconnect")
            )
            .message_type(),
            "session_disconnect"
        );
    }

    #[test]
    fn link_messages_can_bind_inputs_to_exact_pack_session_identity() {
        let session = test_session("session-1", test_modpack("core-modular", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd"))
            .expect("session identity");
        let input = PlayerInputFrame::new(2, Frame(144), 0b1001_0000).expect("input");
        let bound =
            SessionPlayerInputFrame::new(session.clone(), input.clone()).expect("bound input");
        let message = LinkMessage::SessionInput(bound.clone());
        let json = serde_json::to_string(&message).expect("serialize bound input");

        assert!(json.contains(r#""type":"session_input""#), "{json}");
        assert!(json.contains(r#""session_id":"session-1""#), "{json}");
        assert!(json.contains(r#""id":"core-modular""#), "{json}");
        assert!(json.contains(r#""hash":"1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd""#), "{json}");
        assert!(
            json.contains(&format!(r#""pack_content_hash":"{}""#, pack_content_hash())),
            "{json}"
        );
        assert!(json.contains(r#""player_id":2"#), "{json}");
        assert!(json.contains(r#""frame":144"#), "{json}");
        assert!(json.contains(r#""joypad_mask":144"#), "{json}");
        assert_eq!(
            serde_json::from_str::<LinkMessage>(&json).expect("deserialize bound input"),
            message
        );
        assert_eq!(bound.session(), &session);
        assert_eq!(bound.input(), &input);
    }

    #[test]
    fn session_input_link_message_rejects_invalid_session_identity() {
        let input = PlayerInputFrame::new(2, Frame(144), 0b1001_0000).expect("input");
        let invalid_session = LinkSessionIdentity::new_unchecked_for_tests(
            LINK_PROTOCOL_VERSION,
            " session-1",
            test_modpack("core-modular", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd"),
            pack_content_hash(),
        );
        let invalid_bound =
            SessionPlayerInputFrame::new_unchecked_for_tests(invalid_session, input);

        assert!(matches!(
            LinkMessage::SessionInput(invalid_bound).validate(),
            Err(MultiplayerMessageError::InvalidLockstepFrame { .. })
        ));

        let invalid_json = serde_json::json!({
            "type": "session_input",
            "session": {
                "protocol_version": LINK_PROTOCOL_VERSION,
                "session_id": " session-1",
                "modpack": {
                    "id": "core-modular",
                    "hash": "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd"
                },
                "pack_content_hash": pack_content_hash()
            },
            "input": {
                "player_id": 2,
                "frame": 144,
                "joypad_mask": 144
            }
        });

        assert!(
            serde_json::from_value::<LinkMessage>(invalid_json)
                .expect_err("invalid session input rejected")
                .to_string()
                .contains("session-1")
        );
    }

    #[test]
    fn link_messages_serialize_exact_menu_choices_as_transport_neutral_payloads() {
        let choice = MenuChoiceFrame::new(2, Frame(144), "RuntimeMenu", 1, 4).expect("menu choice");
        let message = LinkMessage::MenuChoice(choice.clone());
        let json = serde_json::to_string(&message).expect("serialize menu choice");

        assert_eq!(
            json,
            r#"{"type":"menu_choice","player_id":2,"frame":144,"menu_id":"RuntimeMenu","option_index":1,"verticalmenu_command_index":4}"#
        );
        assert_eq!(
            serde_json::from_str::<LinkMessage>(&json).expect("deserialize menu choice"),
            message
        );
        assert_eq!(choice.player_id(), 2);
        assert_eq!(choice.frame(), 144);
        assert_eq!(choice.menu_id(), "RuntimeMenu");
        assert_eq!(choice.option_index(), 1);
        assert_eq!(choice.verticalmenu_command_index(), 4);
        assert_eq!(message.validate(), Ok(()));
        assert_eq!(
            MenuChoiceFrame::new(0, Frame(144), "RuntimeMenu", 1, 4),
            Err(MultiplayerMessageError::InvalidPlayerIdentity { player_id: 0 })
        );
        assert_eq!(
            MenuChoiceFrame::new(2, Frame(0), "RuntimeMenu", 1, 4),
            Err(MultiplayerMessageError::InvalidFrame {
                field: "menu_choice.frame",
                frame: 0
            })
        );
        assert_eq!(
            MenuChoiceFrame::new(2, Frame(144), "Runtime Menu", 1, 4),
            Err(MultiplayerMessageError::InvalidText {
                field: "menu_choice.menu_id"
            })
        );
    }

    #[test]
    fn link_messages_can_bind_menu_choices_to_exact_pack_session_identity() {
        let session = test_session("session-1", test_modpack("core-modular", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd"))
            .expect("session identity");
        let choice = MenuChoiceFrame::new(2, Frame(144), "RuntimeMenu", 1, 4).expect("menu choice");
        let bound =
            SessionMenuChoiceFrame::new(session.clone(), choice.clone()).expect("bound choice");
        let message = LinkMessage::SessionMenuChoice(bound.clone());
        let json = serde_json::to_string(&message).expect("serialize bound menu choice");

        assert!(json.contains(r#""type":"session_menu_choice""#), "{json}");
        assert!(json.contains(r#""session_id":"session-1""#), "{json}");
        assert!(json.contains(r#""id":"core-modular""#), "{json}");
        assert!(json.contains(r#""hash":"1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd""#), "{json}");
        assert!(
            json.contains(&format!(r#""pack_content_hash":"{}""#, pack_content_hash())),
            "{json}"
        );
        assert!(json.contains(r#""menu_id":"RuntimeMenu""#), "{json}");
        assert_eq!(
            serde_json::from_str::<LinkMessage>(&json).expect("deserialize bound menu choice"),
            message
        );
        assert_eq!(bound.session(), &session);
        assert_eq!(bound.choice(), &choice);

        let invalid_session = LinkSessionIdentity::new_unchecked_for_tests(
            LINK_PROTOCOL_VERSION,
            " session-1",
            test_modpack("core-modular", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd"),
            pack_content_hash(),
        );
        let invalid_bound =
            SessionMenuChoiceFrame::new_unchecked_for_tests(invalid_session, choice);
        assert!(matches!(
            LinkMessage::SessionMenuChoice(invalid_bound).validate(),
            Err(MultiplayerMessageError::InvalidLockstepFrame { .. })
        ));
    }

    #[test]
    fn link_messages_serialize_menu_choice_results_with_state_checksum() {
        let choice = MenuChoiceFrame::new(2, Frame(144), "RuntimeMenu", 1, 4).expect("menu choice");
        let result = MenuChoiceResultFrame::new(
            choice.clone(),
            StateChecksumFrame::new(2, Frame(145), 0xaabb_ccdd),
            "2",
        )
        .expect("menu choice result");
        let message = LinkMessage::MenuChoiceResult(result.clone());
        let json = serde_json::to_string(&message).expect("serialize menu choice result");

        assert!(json.contains(r#""type":"menu_choice_result""#));
        assert_eq!(
            serde_json::from_str::<LinkMessage>(&json).expect("deserialize menu choice result"),
            message
        );
        assert_eq!(result.choice(), &choice);
        assert_eq!(result.checksum().player_id(), 2);
        assert_eq!(result.checksum().frame(), 145);
        assert_eq!(result.script_value(), "2");
        assert_eq!(message.validate(), Ok(()));
        assert_eq!(
            MenuChoiceResultFrame::new(
                choice.clone(),
                StateChecksumFrame::new(3, Frame(145), 0xaabb_ccdd),
                "2",
            ),
            Err(MenuChoiceResultFrameError::PlayerChecksumMismatch {
                choice_player_id: 2,
                checksum_player_id: 3
            })
        );
        assert_eq!(
            MenuChoiceResultFrame::new(
                choice,
                StateChecksumFrame::new(2, Frame(143), 0xaabb_ccdd),
                "2",
            ),
            Err(MenuChoiceResultFrameError::ResultBeforeChoice {
                choice_frame: 144,
                checksum_frame: 143
            })
        );
    }

    #[test]
    fn link_messages_can_bind_menu_choice_results_to_exact_pack_session_identity() {
        let session = test_session("session-1", test_modpack("core-modular", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd"))
            .expect("session identity");
        let choice = MenuChoiceFrame::new(2, Frame(144), "RuntimeMenu", 1, 4).expect("menu choice");
        let result = MenuChoiceResultFrame::new(
            choice,
            StateChecksumFrame::new(2, Frame(145), 0xaabb_ccdd),
            "2",
        )
        .expect("menu choice result");
        let bound = SessionMenuChoiceResultFrame::new(session.clone(), result.clone())
            .expect("bound menu result");
        let message = LinkMessage::SessionMenuChoiceResult(bound.clone());
        let json = serde_json::to_string(&message).expect("serialize bound menu result");

        assert!(
            json.contains(r#""type":"session_menu_choice_result""#),
            "{json}"
        );
        assert!(json.contains(r#""session_id":"session-1""#), "{json}");
        assert!(
            json.contains(&format!(r#""pack_content_hash":"{}""#, pack_content_hash())),
            "{json}"
        );
        assert!(json.contains(r#""script_value":"2""#), "{json}");
        assert_eq!(
            serde_json::from_str::<LinkMessage>(&json).expect("deserialize bound menu result"),
            message
        );
        assert_eq!(bound.session(), &session);
        assert_eq!(bound.result(), &result);

        let invalid_session = LinkSessionIdentity::new_unchecked_for_tests(
            LINK_PROTOCOL_VERSION,
            " session-1",
            test_modpack("core-modular", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd"),
            pack_content_hash(),
        );
        let invalid_bound =
            SessionMenuChoiceResultFrame::new_unchecked_for_tests(invalid_session, result);
        assert!(matches!(
            LinkMessage::SessionMenuChoiceResult(invalid_bound).validate(),
            Err(MultiplayerMessageError::InvalidRuntimeCommand { .. })
        ));
    }

    #[test]
    fn link_messages_serialize_input_journals_as_transport_neutral_payloads() {
        let session =
            test_session("session-1", test_modpack("core-modular", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd")).expect("session");
        let journal = DeterministicInputJournal::new(
            session,
            [1, 2],
            StateChecksumFrame::new(1, Frame(4), 0xaabb_ccdd),
            StateChecksumFrame::new(1, Frame(5), 0xbbcc_ddee),
            vec![
                LockstepFrame::new(4, BTreeMap::from([(1, 0x10), (2, 0x20)]))
                    .expect("lockstep frame"),
            ],
        )
        .expect("journal");
        let journal_frame = DeterministicInputJournalFrame::new(journal).expect("journal frame");
        let message = LinkMessage::InputJournal(journal_frame.clone());

        let json = serde_json::to_string(&message).expect("serialize journal message");

        assert!(json.contains(r#""type":"input_journal""#));
        assert!(json.contains(r#""fingerprint":""#));
        assert!(json.contains(r#""session_id":"session-1""#));
        assert_eq!(
            serde_json::from_str::<LinkMessage>(&json).expect("deserialize journal message"),
            message
        );
    }

    #[test]
    fn link_messages_round_trip_as_framed_binary_payloads() {
        let message =
            LinkMessage::Input(PlayerInputFrame::new(2, Frame(144), 0b1001_0000).expect("input"));
        let bytes = encode_link_message_bytes(&message).expect("encode binary link message");

        assert!(bytes.starts_with(LINK_MESSAGE_MAGIC));
        assert_eq!(
            decode_link_message_bytes(&bytes).expect("decode binary link message"),
            message
        );

        let json = serde_json::to_vec(&message).expect("serialize json link message");
        assert_eq!(
            decode_link_message_bytes(&json),
            Err(MultiplayerMessageError::InvalidBinaryMagic)
        );
    }

    #[test]
    fn binary_link_messages_can_decode_with_exact_session_gate() {
        let session = test_session("session-1", test_modpack("core-modular", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd"))
            .expect("session identity");
        let message = LinkMessage::SessionInput(
            SessionPlayerInputFrame::new(
                session.clone(),
                PlayerInputFrame::new(2, Frame(144), 0b1001_0000).expect("input"),
            )
            .expect("bound input"),
        );
        let bytes = encode_link_message_bytes(&message).expect("encode binary link message");

        assert_eq!(
            decode_link_message_bytes_for_session(&bytes, &session)
                .expect("decode exact-session binary link message"),
            message
        );
    }

    #[test]
    fn binary_link_messages_reject_raw_or_wrong_session_at_exact_session_gate() {
        let session = test_session("session-1", test_modpack("core-modular", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd"))
            .expect("session identity");
        let raw_message =
            LinkMessage::Input(PlayerInputFrame::new(2, Frame(144), 0b1001_0000).expect("input"));
        let raw_bytes =
            encode_link_message_bytes(&raw_message).expect("encode raw binary link message");

        assert_eq!(
            decode_link_message_bytes_for_session(&raw_bytes, &session),
            Err(MultiplayerMessageError::MissingSessionIdentity {
                message_type: "input",
            })
        );

        let wrong_session = test_session("session-1", test_modpack("core-modular", "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"))
            .expect("wrong session identity");
        let wrong_message = LinkMessage::SessionInput(
            SessionPlayerInputFrame::new(
                wrong_session,
                PlayerInputFrame::new(2, Frame(144), 0b1001_0000).expect("input"),
            )
            .expect("bound input"),
        );
        let wrong_bytes =
            encode_link_message_bytes(&wrong_message).expect("encode wrong binary link message");

        assert!(matches!(
            decode_link_message_bytes_for_session(&wrong_bytes, &session),
            Err(MultiplayerMessageError::SessionIdentityMismatch {
                message_type: "session_input",
                message,
            }) if message.contains("modpack hash")
        ));
    }

    #[test]
    fn binary_link_messages_reject_tampered_headers_and_payloads() {
        let message = LinkMessage::StateHash(StateChecksumFrame::new(2, Frame(144), 0xaabbccdd));
        let mut bytes = encode_link_message_bytes(&message).expect("encode binary link message");

        let mut wrong_version = bytes.clone();
        wrong_version[LINK_MESSAGE_VERSION_OFFSET..LINK_MESSAGE_PAYLOAD_LENGTH_OFFSET]
            .copy_from_slice(&(LINK_PROTOCOL_VERSION + 1).to_be_bytes());
        assert_eq!(
            decode_link_message_bytes(&wrong_version),
            Err(MultiplayerMessageError::BinaryVersionMismatch {
                expected: LINK_PROTOCOL_VERSION,
                actual: LINK_PROTOCOL_VERSION + 1,
            })
        );

        let mut truncated = bytes.clone();
        truncated.pop();
        assert!(matches!(
            decode_link_message_bytes(&truncated),
            Err(MultiplayerMessageError::BinaryLengthMismatch { .. })
        ));

        let last = bytes.len() - 1;
        bytes[last] ^= 0x01;
        assert!(matches!(
            decode_link_message_bytes(&bytes),
            Err(MultiplayerMessageError::BinaryHashMismatch { .. })
        ));
    }

    #[test]
    fn binary_link_messages_validate_decoded_payloads() {
        let invalid_message = LinkMessage::CommandChecksum(CommandChecksumResult {
            events: vec![GameEvent::JoypadChanged {
                pressed: B_PAD_A,
                down: 0,
            }],
            checksum: StateChecksumFrame::new(2, Frame(7), 0x1111_1111),
        });
        let encoded = serde_json::to_vec(&invalid_message)
            .expect("encode invalid link message for decode test");
        let mut bytes = Vec::with_capacity(LINK_MESSAGE_HEADER_LEN + encoded.len());
        bytes.extend_from_slice(LINK_MESSAGE_MAGIC);
        bytes.extend_from_slice(&LINK_PROTOCOL_VERSION.to_be_bytes());
        bytes.extend_from_slice(&(encoded.len() as u32).to_be_bytes());
        bytes.extend_from_slice(&fnv1a32_bytes(&encoded).to_be_bytes());
        bytes.extend_from_slice(&encoded);

        assert_eq!(
            decode_link_message_bytes(&bytes),
            Err(MultiplayerMessageError::BinaryDecode(
                "command checksum event 0 has pressed bits 0b00010000 outside down mask 0b00000000"
                    .to_string(),
            ))
        );
        assert_eq!(
            encode_link_message_bytes(&invalid_message),
            Err(MultiplayerMessageError::InvalidCommandChecksumEvent {
                message:
                    "command checksum event 0 has pressed bits 0b00010000 outside down mask 0b00000000"
                        .to_string(),
            })
        );

        let invalid_hello = LinkMessage::Hello(LinkHello::new_unchecked_for_tests(
            LinkSessionIdentity::new_unchecked_for_tests(
                LINK_PROTOCOL_VERSION,
                " session-1",
                test_modpack("core-modular", "1234abce1234abce1234abce1234abce1234abce1234abce1234abce1234abce"),
                pack_content_hash(),
            ),
            test_player(2),
        ));
        let encoded_hello =
            serde_json::to_vec(&invalid_hello).expect("encode invalid hello for decode test");
        let mut hello_bytes = Vec::with_capacity(LINK_MESSAGE_HEADER_LEN + encoded_hello.len());
        hello_bytes.extend_from_slice(LINK_MESSAGE_MAGIC);
        hello_bytes.extend_from_slice(&LINK_PROTOCOL_VERSION.to_be_bytes());
        hello_bytes.extend_from_slice(&(encoded_hello.len() as u32).to_be_bytes());
        hello_bytes.extend_from_slice(&fnv1a32_bytes(&encoded_hello).to_be_bytes());
        hello_bytes.extend_from_slice(&encoded_hello);

        assert!(decode_link_message_bytes(&hello_bytes).is_err());
        assert!(matches!(
            encode_link_message_bytes(&invalid_hello),
            Err(MultiplayerMessageError::InvalidLinkHandshake { .. })
        ));
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
    fn session_state_hash_message_carries_exact_pack_bound_session_identity() {
        let session = test_session("session-1", test_modpack("core-modular", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd"))
            .expect("session identity");
        let checksum = StateChecksumFrame::new(2, Frame(144), 0xaabbccdd);
        let frame = SessionStateChecksumFrame::new(session.clone(), checksum.clone())
            .expect("session checksum");
        let message = LinkMessage::SessionStateHash(frame.clone());

        let json = serde_json::to_string(&message).expect("serialize session state hash");
        assert!(json.contains(r#""type":"session_state_hash""#), "{json}");
        assert!(json.contains(r#""session_id":"session-1""#), "{json}");
        assert!(json.contains(r#""id":"core-modular""#), "{json}");
        assert!(json.contains(r#""hash":"1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd""#), "{json}");
        assert!(
            json.contains(&format!(r#""pack_content_hash":"{}""#, pack_content_hash())),
            "{json}"
        );
        assert!(json.contains(r#""player_id":2"#), "{json}");
        assert!(json.contains(r#""frame":144"#), "{json}");
        assert_eq!(
            serde_json::from_str::<LinkMessage>(&json).expect("deserialize session state hash"),
            message
        );
        assert_eq!(frame.session(), &session);
        assert_eq!(frame.checksum(), &checksum);

        let invalid_session = LinkSessionIdentity::new_unchecked_for_tests(
            LINK_PROTOCOL_VERSION,
            " session-1",
            test_modpack("core-modular", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd"),
            pack_content_hash(),
        );
        let invalid_frame =
            SessionStateChecksumFrame::new_unchecked_for_tests(invalid_session, checksum);
        assert!(matches!(
            LinkMessage::SessionStateHash(invalid_frame).validate(),
            Err(MultiplayerMessageError::InvalidLockstepFrame { .. })
        ));
    }

    #[test]
    fn command_checksum_message_carries_events_and_exact_checksum_identity() {
        let message = LinkMessage::CommandChecksum(CommandChecksumResult {
            events: vec![crate::state::GameEvent::JoypadChanged {
                pressed: 0b0001_0000,
                down: 0b0001_0000,
            }],
            checksum: StateChecksumFrame::new(2, Frame(144), 0xaabbccdd),
        });
        let json = serde_json::to_string(&message).expect("serialize command checksum");
        assert_eq!(
            json,
            r#"{"type":"command_checksum","events":[{"type":"joypad_changed","pressed":16,"down":16}],"checksum":{"player_id":2,"frame":144,"hash":2864434397}}"#
        );
        assert_eq!(
            serde_json::from_str::<LinkMessage>(&json).expect("deserialize command checksum"),
            message
        );
    }

    #[test]
    fn command_checksum_message_rejects_invalid_checksum_identity() {
        let message = LinkMessage::CommandChecksum(CommandChecksumResult {
            events: Vec::new(),
            checksum: StateChecksumFrame::new(0, Frame(7), 0x1111_1111),
        });

        assert_eq!(
            message.validate(),
            Err(MultiplayerMessageError::InvalidLockstepFrame {
                message: "lockstep player id 0 is not a valid link identity".to_string(),
            })
        );
    }

    #[test]
    fn command_checksum_message_rejects_invalid_event_payloads() {
        let invalid_pressed = LinkMessage::CommandChecksum(CommandChecksumResult {
            events: vec![GameEvent::JoypadChanged {
                pressed: B_PAD_A,
                down: 0,
            }],
            checksum: StateChecksumFrame::new(2, Frame(7), 0x1111_1111),
        });
        assert_eq!(
            invalid_pressed.validate(),
            Err(MultiplayerMessageError::InvalidCommandChecksumEvent {
                message:
                    "command checksum event 0 has pressed bits 0b00010000 outside down mask 0b00000000"
                        .to_string(),
            })
        );

        let conflicting_directions = LinkMessage::CommandChecksum(CommandChecksumResult {
            events: vec![GameEvent::JoypadChanged {
                pressed: 0,
                down: B_PAD_LEFT | B_PAD_RIGHT,
            }],
            checksum: StateChecksumFrame::new(2, Frame(7), 0x1111_1111),
        });
        assert_eq!(
            conflicting_directions.validate(),
            Err(MultiplayerMessageError::InvalidCommandChecksumEvent {
                message:
                    "command checksum event 0 down: lockstep input mask 0b00000011 has conflicting direction buttons"
                        .to_string(),
            })
        );

        let frame_zero = LinkMessage::CommandChecksum(CommandChecksumResult {
            events: vec![GameEvent::FrameAdvanced { frame: 0 }],
            checksum: StateChecksumFrame::new(2, Frame(7), 0x1111_1111),
        });
        assert_eq!(
            frame_zero.validate(),
            Err(MultiplayerMessageError::InvalidCommandChecksumEvent {
                message: "command checksum event 0 advances to frame 0".to_string(),
            })
        );
    }

    #[test]
    fn runtime_command_frames_can_be_bound_to_exact_link_session_identity() {
        let session = test_session("session-1", test_modpack("core-modular", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd"))
            .expect("session identity");
        let payload = RuntimeCommandPayload::new("script-command", vec![0x10, 0x20])
            .expect("runtime payload");
        assert_eq!(
            RuntimeCommandFrame::new(2, 0, payload.clone(), StateChecksum::new(144, 0xaabb_ccdd)),
            Err(RuntimeCommandFrameError::InvalidSequence { sequence: 0 })
        );
        let command = RuntimeCommandFrame::new(2, 7, payload, StateChecksum::new(144, 0xaabb_ccdd))
            .expect("runtime command");
        let bound_command = SessionRuntimeCommandFrame::new(session.clone(), command.clone())
            .expect("bound command");

        assert_eq!(bound_command.session(), &session);
        assert_eq!(bound_command.command(), &command);
        let json = serde_json::to_string(&bound_command).expect("serialize bound command");
        assert!(json.contains(r#""session_id":"session-1""#));
        assert!(json.contains(&format!(r#""pack_content_hash":"{}""#, pack_content_hash())));
        assert_eq!(
            serde_json::from_str::<SessionRuntimeCommandFrame>(&json)
                .expect("deserialize bound command"),
            bound_command
        );

        let invalid_session = LinkSessionIdentity::new_unchecked_for_tests(
            LINK_PROTOCOL_VERSION,
            " session-1",
            test_modpack("core-modular", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd"),
            pack_content_hash(),
        );
        let invalid_bound =
            SessionRuntimeCommandFrame::new_unchecked_for_tests(invalid_session, command);
        assert!(matches!(
            invalid_bound.validate(),
            Err(RuntimeCommandFrameError::InvalidSession { .. })
        ));
    }

    #[test]
    fn runtime_command_result_frames_can_be_bound_to_exact_link_session_identity() {
        let session = test_session("session-1", test_modpack("core-modular", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd"))
            .expect("session identity");
        let payload = RuntimeCommandPayload::new("script-command", vec![0x10, 0x20])
            .expect("runtime payload");
        let command = RuntimeCommandFrame::new(2, 7, payload, StateChecksum::new(144, 0xaabb_ccdd))
            .expect("runtime command");
        let result = RuntimeCommandResultFrame::new(
            command.clone(),
            StateChecksumFrame::new(2, Frame(145), 0xbbcc_ddee),
            "ok",
        )
        .expect("runtime command result");
        let bound_result = SessionRuntimeCommandResultFrame::new(session.clone(), result.clone())
            .expect("bound result");

        assert_eq!(bound_result.session(), &session);
        assert_eq!(bound_result.result(), &result);
        let json = serde_json::to_string(&bound_result).expect("serialize bound result");
        assert!(json.contains(r#""session_id":"session-1""#));
        assert!(json.contains(r#""result_tag":"ok""#));
        assert_eq!(
            serde_json::from_str::<SessionRuntimeCommandResultFrame>(&json)
                .expect("deserialize bound result"),
            bound_result
        );

        let invalid_result = RuntimeCommandResultFrame::new_unchecked_for_tests(
            command,
            StateChecksumFrame::new(3, Frame(145), 0xbbcc_ddee),
            "ok",
        );
        let invalid_bound =
            SessionRuntimeCommandResultFrame::new_unchecked_for_tests(session, invalid_result);
        assert!(matches!(
            invalid_bound.validate(),
            Err(RuntimeCommandFrameError::PlayerChecksumMismatch { .. })
        ));
    }

    #[test]
    fn save_summaries_can_be_bound_to_exact_link_session_identity() {
        let modpack = test_modpack("core-modular", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd");
        let session = test_session("session-1", modpack.clone()).expect("session identity");
        let summary = save_summary(modpack, pack_content_hash(), 144);
        let bound =
            SessionSaveSummaryFrame::new(session.clone(), summary.clone()).expect("bound summary");

        assert_eq!(bound.session(), &session);
        assert_eq!(bound.summary(), &summary);
        let json = serde_json::to_string(&bound).expect("serialize bound summary");
        assert!(json.contains(r#""session_id":"session-1""#));
        assert!(json.contains(r#""saved_frame":144"#));
        assert_eq!(
            serde_json::from_str::<SessionSaveSummaryFrame>(&json)
                .expect("deserialize bound summary"),
            bound
        );
        assert_eq!(LinkMessage::SaveSummary(summary).validate(), Ok(()));

        let other_summary = save_summary(
            SaveModpackIdentity::new("core-modular", "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff").expect("other identity"),
            pack_content_hash(),
            144,
        );
        let invalid = SessionSaveSummaryFrame::new_unchecked_for_tests(session, other_summary);
        assert!(matches!(
            invalid.validate(),
            Err(SessionSaveSummaryFrameError::ModpackHashMismatch { .. })
        ));
    }

    #[test]
    fn save_checkpoints_bind_summary_to_state_checksum_and_session() {
        let modpack = test_modpack("core-modular", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd");
        let session = test_session("session-1", modpack.clone()).expect("session identity");
        let summary = save_summary(modpack, pack_content_hash(), 144);
        let checksum = StateChecksumFrame::new(2, Frame(144), 0xaabb_ccdd);
        let checkpoint =
            SaveCheckpointFrame::new(summary.clone(), checksum.clone()).expect("checkpoint");
        let bound = SessionSaveCheckpointFrame::new(session.clone(), checkpoint.clone())
            .expect("bound checkpoint");
        let wrong_pack_session =
            test_session("session-1", test_modpack("other-pack", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd")).expect("session");
        assert!(matches!(
            SessionSaveCheckpointFrame::new(wrong_pack_session, checkpoint.clone()),
            Err(SaveCheckpointFrameError::InvalidSessionSummary { .. })
        ));

        assert_eq!(checkpoint.summary(), &summary);
        assert_eq!(checkpoint.checksum(), &checksum);
        assert_eq!(bound.session(), &session);
        assert_eq!(bound.checkpoint(), &checkpoint);
        assert_eq!(
            LinkMessage::SaveCheckpoint(checkpoint.clone()).validate(),
            Ok(())
        );
        assert_eq!(checkpoint.validate_for_players([1, 2]), Ok(()));
        let mut lobby = LinkLobby::new(session.clone(), test_player(1)).expect("lobby");
        lobby
            .accept_hello(LinkHello::from_session(session.clone(), test_player(2)).expect("hello"))
            .expect("accept checkpoint player");
        assert_eq!(lobby.validate_save_checkpoint(&checkpoint), Ok(()));
        assert_eq!(
            checkpoint.validate_for_players([1]),
            Err(SaveCheckpointFrameError::UnknownPlayer { player_id: 2 })
        );

        let wrong_frame = SaveCheckpointFrame::new_unchecked_for_tests(
            summary.clone(),
            StateChecksumFrame::new(2, Frame(145), 0xaabb_ccdd),
        );
        assert_eq!(
            wrong_frame.validate(),
            Err(SaveCheckpointFrameError::FrameMismatch {
                summary_frame: 144,
                checksum_frame: 145,
            })
        );
        let wrong_hash = SaveCheckpointFrame::new_unchecked_for_tests(
            summary.clone(),
            StateChecksumFrame::new(2, Frame(144), 0xdddd_ccbb),
        );
        assert_eq!(
            wrong_hash.validate(),
            Err(SaveCheckpointFrameError::HashMismatch {
                summary_hash: 0xaabb_ccdd,
                checksum_hash: 0xdddd_ccbb,
            })
        );
    }

    #[test]
    fn link_messages_can_carry_session_bound_save_and_runtime_frames() {
        let modpack = test_modpack("core-modular", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd");
        let session = test_session("session-1", modpack.clone()).expect("session identity");
        let summary = save_summary(modpack.clone(), pack_content_hash(), 144);
        let checkpoint = SaveCheckpointFrame::new(
            summary.clone(),
            StateChecksumFrame::new(2, Frame(144), 0xaabb_ccdd),
        )
        .expect("checkpoint");
        let payload = RuntimeCommandPayload::new("script-command", vec![0x10, 0x20])
            .expect("runtime payload");
        let command = RuntimeCommandFrame::new(2, 7, payload, StateChecksum::new(144, 0xaabb_ccdd))
            .expect("runtime command");
        let result = RuntimeCommandResultFrame::new(
            command.clone(),
            StateChecksumFrame::new(2, Frame(145), 0xbbcc_ddee),
            "ok",
        )
        .expect("runtime result");

        for message in [
            LinkMessage::SessionSaveSummary(
                SessionSaveSummaryFrame::new(session.clone(), summary.clone())
                    .expect("bound summary"),
            ),
            LinkMessage::SessionSaveCheckpoint(
                SessionSaveCheckpointFrame::new(session.clone(), checkpoint)
                    .expect("bound checkpoint"),
            ),
            LinkMessage::SessionRuntimeCommand(
                SessionRuntimeCommandFrame::new(session.clone(), command.clone())
                    .expect("bound command"),
            ),
            LinkMessage::SessionRuntimeCommandResult(
                SessionRuntimeCommandResultFrame::new(session.clone(), result)
                    .expect("bound result"),
            ),
        ] {
            assert_eq!(message.validate(), Ok(()));
            let json = serde_json::to_string(&message).expect("serialize session-bound message");
            assert!(json.contains(r#""session_id":"session-1""#));
            assert!(json.contains(r#""pack_content_hash":"0102030401020304010203040102030401020304010203040102030401020304""#));
            assert_eq!(
                serde_json::from_str::<LinkMessage>(&json).expect("deserialize bound message"),
                message
            );
            encode_link_message_bytes(&message).expect("encode bound message");
        }

        let mismatched_summary = save_summary(
            SaveModpackIdentity::new("core-modular", "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff").expect("other identity"),
            pack_content_hash(),
            144,
        );
        assert!(matches!(
            LinkMessage::SessionSaveSummary(SessionSaveSummaryFrame::new_unchecked_for_tests(
                session,
                mismatched_summary,
            ))
            .validate(),
            Err(MultiplayerMessageError::InvalidLinkHandshake { .. })
        ));
    }

    #[test]
    fn deterministic_replay_bundle_binds_journal_commands_results_and_terminal_checksum() {
        let session = test_session("session-1", test_modpack("core-modular", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd"))
            .expect("session identity");
        let journal = DeterministicInputJournal::new(
            session.clone(),
            [1, 2],
            StateChecksumFrame::new(1, Frame(4), 0xaabb_ccdd),
            StateChecksumFrame::new(1, Frame(6), 0xbbcc_ddee),
            vec![
                LockstepFrame::new(4, BTreeMap::from([(1, 0x10), (2, 0x20)])).expect("frame 4"),
                LockstepFrame::new(5, BTreeMap::from([(1, 0x00), (2, 0x80)])).expect("frame 5"),
            ],
        )
        .expect("journal");
        let journal_frame = DeterministicInputJournalFrame::new(journal).expect("journal frame");
        let payload = RuntimeCommandPayload::new("script-command", vec![0x10, 0x20])
            .expect("runtime payload");
        let command = RuntimeCommandFrame::new(2, 7, payload, StateChecksum::new(5, 0xaabb_ccdd))
            .expect("runtime command");
        let bound_command = SessionRuntimeCommandFrame::new(session.clone(), command.clone())
            .expect("bound command");
        let result = RuntimeCommandResultFrame::new(
            command,
            StateChecksumFrame::new(2, Frame(6), 0xbbcc_ddee),
            "ok",
        )
        .expect("runtime result");
        let bound_result =
            SessionRuntimeCommandResultFrame::new(session, result).expect("bound result");
        let menu_result = MenuChoiceResultFrame::new(
            MenuChoiceFrame::new(1, Frame(5), "RuntimeMenu", 1, 4).expect("menu choice"),
            StateChecksumFrame::new(1, Frame(6), 0xbbcc_ddee),
            "2",
        )
        .expect("menu result");
        let bundle = DeterministicReplayBundle::new(
            journal_frame.clone(),
            vec![bound_command.clone()],
            vec![bound_result.clone()],
            vec![menu_result.clone()],
            journal_frame.journal().terminal_checksum().clone(),
        )
        .expect("replay bundle");

        assert_eq!(bundle.input_journal(), &journal_frame);
        assert_eq!(bundle.runtime_commands(), &[bound_command]);
        assert_eq!(bundle.runtime_results(), &[bound_result]);
        assert_eq!(bundle.menu_results(), &[menu_result]);
        assert_eq!(
            bundle.terminal_checksum(),
            journal_frame.journal().terminal_checksum()
        );
        let json = serde_json::to_string(&bundle).expect("serialize bundle");
        assert!(json.contains(r#""input_journal""#));
        assert!(json.contains(r#""runtime_commands""#));
        assert_eq!(
            serde_json::from_str::<DeterministicReplayBundle>(&json).expect("deserialize bundle"),
            bundle
        );
    }

    #[test]
    fn save_resume_replay_binds_checkpoint_to_journal_start_state() {
        let modpack = test_modpack("core-modular", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd");
        let session = test_session("session-1", modpack.clone()).expect("session identity");
        let checkpoint = SessionSaveCheckpointFrame::new(
            session.clone(),
            SaveCheckpointFrame::new(
                save_summary(modpack, pack_content_hash(), 4),
                StateChecksumFrame::new(1, Frame(4), 0xaabb_ccdd),
            )
            .expect("checkpoint"),
        )
        .expect("session checkpoint");
        let journal = DeterministicInputJournal::new(
            session.clone(),
            [1, 2],
            StateChecksumFrame::new(1, Frame(4), 0xaabb_ccdd),
            StateChecksumFrame::new(1, Frame(6), 0xbbcc_ddee),
            vec![
                LockstepFrame::new(4, BTreeMap::from([(1, 0x10), (2, 0x20)])).expect("frame 4"),
                LockstepFrame::new(5, BTreeMap::from([(1, 0x00), (2, 0x80)])).expect("frame 5"),
            ],
        )
        .expect("journal");
        let journal_frame = DeterministicInputJournalFrame::new(journal).expect("journal frame");
        let replay = DeterministicReplayBundle::new(
            journal_frame.clone(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            journal_frame.journal().terminal_checksum().clone(),
        )
        .expect("replay");
        let resume =
            SaveResumeReplayBundle::new(checkpoint.clone(), replay.clone()).expect("resume replay");

        assert_eq!(resume.checkpoint(), &checkpoint);
        assert_eq!(resume.replay(), &replay);
        assert_eq!(LinkMessage::SaveResumeReplay(resume).validate(), Ok(()));
    }

    #[test]
    fn save_resume_replay_rejects_session_and_start_checksum_mismatches() {
        let modpack = test_modpack("core-modular", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd");
        let resume_session = test_session("session-1", modpack.clone()).expect("session identity");
        let other_session = test_session("session-2", modpack.clone()).expect("other session");
        let checkpoint = SessionSaveCheckpointFrame::new(
            resume_session.clone(),
            SaveCheckpointFrame::new(
                save_summary(modpack.clone(), pack_content_hash(), 4),
                StateChecksumFrame::new(1, Frame(4), 0xaabb_ccdd),
            )
            .expect("checkpoint"),
        )
        .expect("session checkpoint");
        let journal = DeterministicInputJournal::new(
            resume_session.clone(),
            [1, 2],
            StateChecksumFrame::new(1, Frame(4), 0xaabb_ccdd),
            StateChecksumFrame::new(1, Frame(5), 0xbbcc_ddee),
            vec![LockstepFrame::new(4, BTreeMap::from([(1, 0x10), (2, 0x20)])).expect("frame 4")],
        )
        .expect("journal");
        let journal_frame = DeterministicInputJournalFrame::new(journal).expect("journal frame");
        let replay = DeterministicReplayBundle::new(
            journal_frame.clone(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            journal_frame.journal().terminal_checksum().clone(),
        )
        .expect("replay");

        let wrong_session_checkpoint =
            SessionSaveCheckpointFrame::new(other_session, checkpoint.checkpoint().clone())
                .expect("wrong session checkpoint");
        assert_eq!(
            SaveResumeReplayBundle::new(wrong_session_checkpoint, replay.clone()),
            Err(InputJournalError::SaveReplaySessionMismatch)
        );

        let wrong_hash_checkpoint = SessionSaveCheckpointFrame::new(
            resume_session.clone(),
            SaveCheckpointFrame::new(
                save_summary_with_hash(modpack.clone(), pack_content_hash(), 4, 0xdddd_ccbb),
                StateChecksumFrame::new(1, Frame(4), 0xdddd_ccbb),
            )
            .expect("wrong hash checkpoint"),
        )
        .expect("wrong hash session checkpoint");
        assert_eq!(
            SaveResumeReplayBundle::new(wrong_hash_checkpoint, replay.clone()),
            Err(InputJournalError::SaveReplayStartHashMismatch {
                checkpoint_hash: 0xdddd_ccbb,
                journal_hash: 0xaabb_ccdd,
            })
        );

        let wrong_player_checkpoint = SessionSaveCheckpointFrame::new(
            resume_session,
            SaveCheckpointFrame::new(
                save_summary(modpack, pack_content_hash(), 4),
                StateChecksumFrame::new(2, Frame(4), 0xaabb_ccdd),
            )
            .expect("wrong player checkpoint"),
        )
        .expect("wrong player session checkpoint");
        assert_eq!(
            SaveResumeReplayBundle::new(wrong_player_checkpoint, replay),
            Err(InputJournalError::SaveReplayStartPlayerMismatch {
                checkpoint_player_id: 2,
                journal_player_id: 1,
            })
        );
    }

    #[test]
    fn deterministic_replay_bundle_rejects_wrong_session_and_out_of_range_commands() {
        let replay_session = test_session("session-1", test_modpack("core-modular", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd"))
            .expect("session identity");
        let other_session = test_session("session-2", test_modpack("core-modular", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd"))
            .expect("other session");
        let journal = DeterministicInputJournal::new(
            replay_session.clone(),
            [1, 2],
            StateChecksumFrame::new(1, Frame(4), 0xaabb_ccdd),
            StateChecksumFrame::new(1, Frame(6), 0xbbcc_ddee),
            vec![
                LockstepFrame::new(4, BTreeMap::from([(1, 0x10), (2, 0x20)])).expect("frame 4"),
                LockstepFrame::new(5, BTreeMap::from([(1, 0x00), (2, 0x80)])).expect("frame 5"),
            ],
        )
        .expect("journal");
        let journal_frame = DeterministicInputJournalFrame::new(journal).expect("journal frame");
        let payload = RuntimeCommandPayload::new("script-command", vec![0x10, 0x20])
            .expect("runtime payload");
        let command_before =
            RuntimeCommandFrame::new(2, 7, payload.clone(), StateChecksum::new(3, 0xaabb_ccdd))
                .expect("runtime command");
        let wrong_session_command =
            SessionRuntimeCommandFrame::new(other_session, command_before.clone())
                .expect("bound command");
        let wrong_session_bundle = DeterministicReplayBundle::new(
            journal_frame.clone(),
            vec![wrong_session_command],
            Vec::new(),
            Vec::new(),
            journal_frame.journal().terminal_checksum().clone(),
        );

        assert_eq!(
            wrong_session_bundle,
            Err(InputJournalError::RuntimeCommandSessionMismatch { sequence: 7 })
        );

        let out_of_range_command =
            SessionRuntimeCommandFrame::new(replay_session.clone(), command_before.clone())
                .expect("bound command");
        let out_of_range_bundle = DeterministicReplayBundle::new(
            journal_frame.clone(),
            vec![out_of_range_command],
            Vec::new(),
            Vec::new(),
            journal_frame.journal().terminal_checksum().clone(),
        );

        assert_eq!(
            out_of_range_bundle,
            Err(InputJournalError::RuntimeCommandFrameOutsideJournal {
                sequence: 7,
                frame: 3,
                start: 4,
                terminal: 6,
            })
        );

        let out_of_range_result = RuntimeCommandResultFrame::new(
            command_before,
            StateChecksumFrame::new(2, Frame(4), 0xaabb_ccdd),
            "ok",
        )
        .expect("runtime result with in-range checksum");
        let out_of_range_bound_result =
            SessionRuntimeCommandResultFrame::new(replay_session, out_of_range_result)
                .expect("bound result");
        let out_of_range_result_bundle = DeterministicReplayBundle::new(
            journal_frame.clone(),
            Vec::new(),
            vec![out_of_range_bound_result],
            Vec::new(),
            journal_frame.journal().terminal_checksum().clone(),
        );

        assert_eq!(
            out_of_range_result_bundle,
            Err(InputJournalError::RuntimeCommandResultFrameOutsideJournal {
                sequence: 7,
                frame: 3,
                start: 4,
                terminal: 6,
            })
        );

        let out_of_range_menu = MenuChoiceResultFrame::new(
            MenuChoiceFrame::new(1, Frame(3), "RuntimeMenu", 1, 4).expect("menu choice"),
            StateChecksumFrame::new(1, Frame(4), 0xaabb_ccdd),
            "2",
        )
        .expect("menu result");
        let out_of_range_menu_bundle = DeterministicReplayBundle::new(
            journal_frame.clone(),
            Vec::new(),
            Vec::new(),
            vec![out_of_range_menu],
            journal_frame.journal().terminal_checksum().clone(),
        );

        assert_eq!(
            out_of_range_menu_bundle,
            Err(InputJournalError::MenuChoiceFrameOutsideJournal {
                menu_id: "RuntimeMenu".to_string(),
                option_index: 1,
                frame: 3,
                start: 4,
                terminal: 6,
            })
        );
    }

    #[test]
    fn deterministic_replay_bundle_rejects_terminal_checksum_hash_mismatch() {
        let replay_session = test_session("session-1", test_modpack("core-modular", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd"))
            .expect("session identity");
        let journal = DeterministicInputJournal::new(
            replay_session,
            [1, 2],
            StateChecksumFrame::new(1, Frame(4), 0xaabb_ccdd),
            StateChecksumFrame::new(1, Frame(6), 0xbbcc_ddee),
            vec![
                LockstepFrame::new(4, BTreeMap::from([(1, 0x10), (2, 0x20)])).expect("frame 4"),
                LockstepFrame::new(5, BTreeMap::from([(1, 0x00), (2, 0x80)])).expect("frame 5"),
            ],
        )
        .expect("journal");
        let journal_frame = DeterministicInputJournalFrame::new(journal).expect("journal frame");
        let bundle = DeterministicReplayBundle::new(
            journal_frame,
            Vec::new(),
            Vec::new(),
            Vec::new(),
            StateChecksumFrame::new(1, Frame(6), 0xcccc_dddd),
        );

        assert_eq!(
            bundle,
            Err(InputJournalError::TerminalChecksumHashMismatch {
                expected: 0xbbcc_ddee,
                actual: 0xcccc_dddd,
            })
        );

        let journal = DeterministicInputJournal::new(
            test_session("session-1", test_modpack("core-modular", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd"))
                .expect("session identity"),
            [1, 2],
            StateChecksumFrame::new(1, Frame(4), 0xaabb_ccdd),
            StateChecksumFrame::new(1, Frame(6), 0xbbcc_ddee),
            vec![
                LockstepFrame::new(4, BTreeMap::from([(1, 0x10), (2, 0x20)])).expect("frame 4"),
                LockstepFrame::new(5, BTreeMap::from([(1, 0x00), (2, 0x80)])).expect("frame 5"),
            ],
        )
        .expect("journal");
        let journal_frame = DeterministicInputJournalFrame::new(journal).expect("journal frame");
        let bundle = DeterministicReplayBundle::new(
            journal_frame,
            Vec::new(),
            Vec::new(),
            Vec::new(),
            StateChecksumFrame::new(2, Frame(6), 0xbbcc_ddee),
        );

        assert_eq!(
            bundle,
            Err(InputJournalError::TerminalChecksumPlayerMismatch {
                expected: 1,
                actual: 2,
            })
        );
    }

    #[test]
    fn game_state_checksum_uses_authoritative_serialized_state() {
        let mut state = crate::state::GameState::default();
        state.frame_counter = 144;
        state.overworld = crate::state::OverworldMemory::Active {
            map_name: "PlayersHouse2F".to_string(),
            tile: TilePosition::new(4, 4),
            facing: Direction::Down,
            mode: crate::world::movement::MovementMode::Normal,
        };
        let checksum = game_state_checksum(&state).expect("checksum");
        assert_eq!(
            checksum,
            game_state_checksum_unchecked(&state).expect("unchecked checksum")
        );
        let frame = StateChecksumFrame::from_game_state(2, &state).expect("checksum frame");

        assert_eq!(checksum.frame(), 144);
        assert_eq!(frame.player_id(), 2);
        assert_eq!(frame.checksum(), checksum);
        assert_eq!(
            StateChecksumFrame::from_game_state(0, &state),
            Err(StateChecksumError::InvalidPlayerIdentity { player_id: 0 })
        );

        let mut moved = state;
        moved.overworld = crate::state::OverworldMemory::Active {
            map_name: "PlayersHouse2F".to_string(),
            tile: TilePosition::new(6, 4),
            facing: Direction::Right,
            mode: crate::world::movement::MovementMode::Normal,
        };
        assert_ne!(
            game_state_checksum(&moved).expect("moved checksum").hash(),
            checksum.hash()
        );
    }

    #[test]
    fn game_state_checksum_rejects_malformed_saved_state_without_hashing() {
        let state = crate::state::GameState {
            active_repel_item: Some("SUPER REPEL".to_string()),
            ..crate::state::GameState::default()
        };

        assert_eq!(
            game_state_checksum(&state),
            Err(StateChecksumError::InvalidState(
                "active_repel_item has invalid token 'SUPER REPEL'".to_string()
            ))
        );
        assert_eq!(
            StateChecksumFrame::from_game_state(2, &state),
            Err(StateChecksumError::InvalidState(
                "active_repel_item has invalid token 'SUPER REPEL'".to_string()
            ))
        );
    }

    #[test]
    fn command_checksum_applies_command_and_reports_authoritative_state_hash() {
        let mut state = crate::state::GameState::default();

        let result = apply_command_with_checksum(
            &mut state,
            2,
            crate::state::GameCommand::Joypad { mask: 0b0001_0000 },
        )
        .expect("command checksum");

        assert_eq!(
            result.events,
            vec![crate::state::GameEvent::JoypadChanged {
                pressed: 0b0001_0000,
                down: 0b0001_0000,
            }]
        );
        assert_eq!(result.checksum.player_id(), 2);
        assert_eq!(result.checksum.frame(), state.frame_counter);
        assert_eq!(
            result.checksum.checksum(),
            game_state_checksum(&state).expect("checksum")
        );
    }

    #[test]
    fn command_checksum_rejects_invalid_player_before_command_mutation() {
        let mut state = crate::state::GameState::default();

        let error = apply_command_with_checksum(
            &mut state,
            0,
            crate::state::GameCommand::Joypad { mask: 0b0001_0000 },
        )
        .expect_err("invalid player rejected");

        assert_eq!(
            error,
            CommandChecksumError::Checksum(StateChecksumError::InvalidPlayerIdentity {
                player_id: 0
            })
        );
        assert_eq!(state.joypad.h_joypad_down, 0);
    }

    #[test]
    fn hello_message_carries_protocol_and_exact_modpack_identity() {
        let hello = test_hello(
            "session-1",
            test_modpack("core-modular", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd"),
            test_player(1),
        )
        .expect("hello");
        let message = LinkMessage::Hello(hello.clone());
        let json = serde_json::to_string(&message).expect("serialize hello");

        assert!(json.contains(r#""type":"hello""#));
        assert!(json.contains(r#""protocol_version":2"#));
        assert!(json.contains(r#""id":"core-modular""#));
        assert_eq!(
            serde_json::from_str::<LinkMessage>(&json).expect("deserialize hello"),
            message
        );
        assert_eq!(hello.session().modpack().id(), "core-modular");
    }

    #[test]
    fn hello_json_deserialization_validates_exact_handshake_identity() {
        let prior_protocol = serde_json::from_value::<LinkMessage>(serde_json::json!({
            "type": "hello",
            "session": {
                "protocol_version": 1,
                "session_id": "session-1",
                "modpack": {
                    "id": "core-modular",
                    "hash": "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd"
                },
                "pack_content_hash": "0102030401020304010203040102030401020304010203040102030401020304"
            },
            "player": {
                "id": 1,
                "display_name": "P1"
            }
        }))
        .expect_err("protocol v1 has no compatibility path under protocol v2")
        .to_string();
        assert!(prior_protocol.contains("protocol version 1"), "{prior_protocol}");

        let invalid_player = serde_json::from_value::<LinkMessage>(serde_json::json!({
            "type": "hello",
            "session": {
                "protocol_version": 2,
                "session_id": "session-1",
                "modpack": {
                    "id": "core-modular",
                    "hash": "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd"
                },
                "pack_content_hash": "0102030401020304010203040102030401020304010203040102030401020304"
            },
            "player": {
                "id": 0,
                "display_name": "P1"
            }
        }))
        .expect_err("hello JSON must validate player identity during decode")
        .to_string();
        assert!(
            invalid_player.contains("not a valid link identity"),
            "{invalid_player}"
        );

        let invalid_session = serde_json::from_value::<LinkMessage>(serde_json::json!({
            "type": "hello",
            "session": {
                "protocol_version": 2,
                "session_id": " legacy-session",
                "modpack": {
                    "id": "core-modular",
                    "hash": "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd"
                },
                "pack_content_hash": "0102030401020304010203040102030401020304010203040102030401020304"
            },
            "player": {
                "id": 1,
                "display_name": "P1"
            }
        }))
        .expect_err("hello JSON must validate session identity during decode")
        .to_string();
        assert!(
            invalid_session.contains("link session id"),
            "{invalid_session}"
        );
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
                "protocol_version": 2,
                "session_id": "session-1",
                "modpack": {
                    "id": "core-modular",
                    "hash": "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd",
                    "normalized_id": "CORE-MODULAR"
                },
                "pack_content_hash": "0102030401020304010203040102030401020304010203040102030401020304"
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

        let missing_hash_error = serde_json::from_value::<LinkMessage>(serde_json::json!({
            "type": "battle_action",
            "player_id": 1,
            "turn": 7,
            "action": "run"
        }))
        .expect_err("battle action messages must include deterministic state hash")
        .to_string();
        assert!(
            missing_hash_error.contains("missing field `state_hash`"),
            "{missing_hash_error}"
        );

        let presence_type_error = serde_json::from_value::<PresenceEntityType>(serde_json::json!({
            "player": {
                "legacy_entity_type": "human"
            }
        }))
        .expect_err("presence entity types must not accept legacy aliases")
        .to_string();
        assert!(
            presence_type_error.contains("invalid type")
                || presence_type_error.contains("unknown variant"),
            "{presence_type_error}"
        );

        let interaction_kind_error =
            serde_json::from_value::<MultiplayerInteractionKind>(serde_json::json!({
                "battle": {
                    "fallback_kind": "trade"
                }
            }))
            .expect_err("interaction kinds must not accept fallback aliases")
            .to_string();
        assert!(
            interaction_kind_error.contains("invalid type")
                || interaction_kind_error.contains("unknown variant"),
            "{interaction_kind_error}"
        );
    }

    #[test]
    fn link_handshake_requires_exact_session_protocol_and_modpack_identity() {
        let local =
            test_session("session-1", test_modpack("core-modular", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd")).expect("local");
        let matching = test_hello(
            "session-1",
            test_modpack("core-modular", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd"),
            test_player(2),
        )
        .expect("matching");

        validate_link_hello(&local, &matching).expect("matching hello");

        let wrong_session = test_hello(
            "session-2",
            test_modpack("core-modular", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd"),
            test_player(2),
        )
        .expect("wrong session");
        assert_eq!(
            validate_link_hello(&local, &wrong_session),
            Err(LinkHandshakeError::SessionMismatch {
                expected: "session-1".to_string(),
                actual: "session-2".to_string(),
            })
        );

        let wrong_hash = test_hello(
            "session-1",
            test_modpack("core-modular", "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"),
            test_player(2),
        )
        .expect("wrong hash");
        assert_eq!(
            validate_link_hello(&local, &wrong_hash),
            Err(LinkHandshakeError::ModpackHashMismatch {
                expected: "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd".to_string(),
                actual: "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff".to_string(),
            })
        );

        let wrong_content_hash = LinkHello::from_session(
            LinkSessionIdentity::new(
                "session-1",
                test_modpack("core-modular", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd"),
                "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
            )
            .expect("wrong content hash session"),
            test_player(2),
        )
        .expect("wrong content hash");
        assert_eq!(
            validate_link_hello(&local, &wrong_content_hash),
            Err(LinkHandshakeError::PackContentHashMismatch {
                expected: pack_content_hash().to_string(),
                actual: "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff".to_string(),
            })
        );

        let case_changed = test_hello(
            "session-1",
            test_modpack("CORE-MODULAR", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd"),
            test_player(2),
        )
        .expect("case changed");
        assert_eq!(
            validate_link_hello(&local, &case_changed),
            Err(LinkHandshakeError::ModpackIdMismatch {
                expected: "core-modular".to_string(),
                actual: "CORE-MODULAR".to_string(),
            })
        );

        let reserved_pack = SaveModpackIdentity::new("core-modular+fallback-link", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd")
            .expect_err("link modpack identities must reject reserved pack id segments");
        assert!(
            reserved_pack
                .to_string()
                .contains("uses reserved runtime pack prefix"),
            "{reserved_pack}"
        );
    }

    #[test]
    fn link_session_identity_validation_owns_protocol_and_modpack_comparison() {
        let local =
            test_session("session-1", test_modpack("core-modular", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd")).expect("local");
        let matching =
            test_session("session-1", test_modpack("core-modular", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd")).expect("matching");
        validate_link_session_identity(&local, &matching).expect("matching session");

        let protocol_drift = LinkSessionIdentity::new_unchecked_for_tests(
            LINK_PROTOCOL_VERSION + 1,
            "session-1",
            test_modpack("core-modular", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd"),
            pack_content_hash(),
        );
        assert_eq!(
            validate_link_session_identity(&local, &protocol_drift),
            Err(LinkHandshakeError::ProtocolVersionMismatch {
                expected: LINK_PROTOCOL_VERSION,
                actual: LINK_PROTOCOL_VERSION + 1,
            })
        );

        let other_pack =
            test_session("session-1", test_modpack("other-pack", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd")).expect("other pack");
        assert_eq!(
            validate_link_session_identity(&local, &other_pack),
            Err(LinkHandshakeError::ModpackIdMismatch {
                expected: "core-modular".to_string(),
                actual: "other-pack".to_string(),
            })
        );
    }

    #[test]
    fn link_handshake_rejects_empty_player_display_names_without_placeholders() {
        let zero_id_player = PlayerIdentity::new_unchecked_for_tests(0, "P0");
        assert_eq!(
            test_hello(
                "session-1",
                test_modpack("core-modular", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd"),
                zero_id_player,
            ),
            Err(LinkHandshakeError::InvalidPlayerIdentity { player_id: 0 })
        );

        let empty_player = PlayerIdentity::new_unchecked_for_tests(2, "");
        assert_eq!(
            test_hello(
                "session-1",
                test_modpack("core-modular", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd"),
                empty_player
            ),
            Err(LinkHandshakeError::MissingPlayerDisplayName { player_id: 2 })
        );

        let padded_player = PlayerIdentity::new_unchecked_for_tests(2, " P2");
        assert_eq!(
            test_hello(
                "session-1",
                test_modpack("core-modular", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd"),
                padded_player
            ),
            Err(LinkHandshakeError::InvalidPlayerDisplayName {
                player_id: 2,
                display_name: " P2".to_string(),
            })
        );

        let control_player = PlayerIdentity::new_unchecked_for_tests(2, "P\n2");
        assert_eq!(
            test_hello(
                "session-1",
                test_modpack("core-modular", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd"),
                control_player
            ),
            Err(LinkHandshakeError::InvalidPlayerDisplayName {
                player_id: 2,
                display_name: "P\n2".to_string(),
            })
        );

        let local =
            test_session("session-1", test_modpack("core-modular", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd")).expect("local");
        let bypassed = LinkHello::new_unchecked_for_tests(
            local.clone(),
            PlayerIdentity::new_unchecked_for_tests(3, ""),
        );
        assert_eq!(
            validate_link_hello(&local, &bypassed),
            Err(LinkHandshakeError::MissingPlayerDisplayName { player_id: 3 })
        );

        let padded_bypassed = LinkHello::new_unchecked_for_tests(
            local.clone(),
            PlayerIdentity::new_unchecked_for_tests(3, "P3 "),
        );
        assert_eq!(
            validate_link_hello(&local, &padded_bypassed),
            Err(LinkHandshakeError::InvalidPlayerDisplayName {
                player_id: 3,
                display_name: "P3 ".to_string(),
            })
        );

        let control_bypassed = LinkHello::new_unchecked_for_tests(
            local.clone(),
            PlayerIdentity::new_unchecked_for_tests(3, "P3\0"),
        );
        assert_eq!(
            validate_link_hello(&local, &control_bypassed),
            Err(LinkHandshakeError::InvalidPlayerDisplayName {
                player_id: 3,
                display_name: "P3\0".to_string(),
            })
        );

        assert_eq!(
            LinkLobby::new(local, PlayerIdentity::new_unchecked_for_tests(1, ""),),
            Err(LinkHandshakeError::MissingPlayerDisplayName { player_id: 1 })
        );
    }

    #[test]
    fn link_handshake_rejects_malformed_session_ids_without_trimming() {
        let pack_identity = test_modpack("core-modular", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd");
        assert_eq!(
            test_session("", pack_identity.clone()).and_then(|session| session.validate()),
            Err(LinkHandshakeError::MissingSessionId)
        );
        assert_eq!(
            test_session(" session-1", pack_identity.clone())
                .and_then(|session| session.validate()),
            Err(LinkHandshakeError::InvalidSessionId {
                session_id: " session-1".to_string(),
            })
        );
        assert_eq!(
            test_session("session 1", pack_identity.clone()).and_then(|session| session.validate()),
            Err(LinkHandshakeError::InvalidSessionId {
                session_id: "session 1".to_string(),
            })
        );
        assert_eq!(
            test_session("fallback-session", pack_identity.clone())
                .and_then(|session| session.validate()),
            Err(LinkHandshakeError::ReservedSessionId {
                session_id: "fallback-session".to_string(),
            })
        );

        let local = test_session("session-1", pack_identity.clone()).expect("local");
        let remote = LinkHello::new_unchecked_for_tests(
            LinkSessionIdentity::new_unchecked_for_tests(
                LINK_PROTOCOL_VERSION,
                "session-1 ",
                pack_identity,
                pack_content_hash(),
            ),
            test_player(2),
        );
        assert_eq!(
            validate_link_hello(&local, &remote),
            Err(LinkHandshakeError::InvalidSessionId {
                session_id: "session-1 ".to_string(),
            })
        );

        let reserved_remote = LinkHello::new_unchecked_for_tests(
            LinkSessionIdentity::new_unchecked_for_tests(
                LINK_PROTOCOL_VERSION,
                "legacy-session",
                test_modpack("core-modular", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd"),
                pack_content_hash(),
            ),
            test_player(2),
        );
        assert_eq!(
            validate_link_hello(&local, &reserved_remote),
            Err(LinkHandshakeError::ReservedSessionId {
                session_id: "legacy-session".to_string(),
            })
        );
    }

    #[test]
    fn link_handshake_rejects_protocol_drift() {
        let local =
            test_session("session-1", test_modpack("core-modular", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd")).expect("local");
        let remote = LinkHello::new_unchecked_for_tests(
            LinkSessionIdentity::new_unchecked_for_tests(
                LINK_PROTOCOL_VERSION + 1,
                "session-1",
                test_modpack("core-modular", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd"),
                pack_content_hash(),
            ),
            test_player(2),
        );

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
        let session =
            test_session("session-1", test_modpack("core-modular", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd")).expect("session");
        let mut lobby = LinkLobby::new(session.clone(), test_player(3)).expect("lobby");

        assert_eq!(
            lobby.accept_hello(
                test_hello(
                    "session-1",
                    test_modpack("core-modular", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd"),
                    test_player(1)
                )
                .expect("player 1 hello")
            ),
            Ok(AcceptPlayerResult::Added)
        );
        assert_eq!(
            lobby.accept_hello(
                test_hello(
                    "session-1",
                    test_modpack("core-modular", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd"),
                    test_player(2)
                )
                .expect("player 2 hello")
            ),
            Ok(AcceptPlayerResult::Added)
        );

        assert_eq!(lobby.session(), &session);
        assert_eq!(lobby.player_ids(), vec![1, 2, 3]);
        assert_eq!(
            lobby.players(),
            vec![test_player(1), test_player(2), test_player(3)]
        );
    }

    #[test]
    fn link_lobby_duplicate_same_player_is_idempotent() {
        let session =
            test_session("session-1", test_modpack("core-modular", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd")).expect("session");
        let mut lobby = LinkLobby::new(session, test_player(1)).expect("lobby");
        let hello = test_hello(
            "session-1",
            test_modpack("core-modular", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd"),
            test_player(2),
        )
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
        let session =
            test_session("session-1", test_modpack("core-modular", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd")).expect("session");
        let mut lobby = LinkLobby::new(session, test_player(1)).expect("lobby");
        let original = test_hello(
            "session-1",
            test_modpack("core-modular", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd"),
            test_player(2),
        )
        .expect("original");
        let conflict = test_hello(
            "session-1",
            test_modpack("core-modular", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd"),
            PlayerIdentity::new(2, "P02").expect("player"),
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
        let session =
            test_session("session-1", test_modpack("core-modular", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd")).expect("session");
        let mut lobby = LinkLobby::new(session, test_player(1)).expect("lobby");
        let case_changed = test_hello(
            "session-1",
            test_modpack("CORE-MODULAR", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd"),
            test_player(2),
        )
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
        let session =
            test_session("session-1", test_modpack("core-modular", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd")).expect("session");
        let mut lobby = LinkLobby::new(session, test_player(4)).expect("lobby");
        lobby
            .accept_hello(
                test_hello(
                    "session-1",
                    test_modpack("core-modular", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd"),
                    test_player(2),
                )
                .expect("hello"),
            )
            .expect("accept");

        let mut buffer = lobby.lockstep_buffer().expect("lockstep buffer");
        assert_eq!(buffer.players(), vec![2, 4]);
        assert_eq!(
            buffer.insert_input(PlayerInputFrame::new(4, Frame(12), 0x10).expect("input")),
            Ok(InsertInputResult::Inserted)
        );
        assert!(!buffer.is_frame_ready(12));
        assert_eq!(
            buffer.insert_input(PlayerInputFrame::new(2, Frame(12), 0x20).expect("input")),
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
        let session =
            test_session("session-1", test_modpack("core-modular", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd")).expect("session");
        let lobby = LinkLobby::new(session.clone(), test_player(1)).expect("lobby");

        assert_eq!(
            lobby.local_hello(1).expect("hello"),
            LinkHello::from_session(session, test_player(1)).expect("hello")
        );
        assert_eq!(
            lobby.local_hello(2),
            Err(LinkHandshakeError::UnknownPlayer { player_id: 2 })
        );
    }

    #[test]
    fn battle_action_sync_waits_for_roster_and_orders_exact_actions() {
        let session =
            test_session("session-1", test_modpack("core-modular", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd")).expect("session");
        let mut lobby = LinkLobby::new(session, test_player(4)).expect("lobby");
        lobby
            .accept_hello(
                test_hello(
                    "session-1",
                    test_modpack("core-modular", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd"),
                    test_player(2),
                )
                .expect("hello"),
            )
            .expect("accept");
        let mut sync = lobby.battle_action_buffer().expect("battle action buffer");

        assert_eq!(
            sync.insert_action(
                BattleActionFrame::with_state_hash(
                    4,
                    1,
                    BattleAction::Item {
                        item_id: "EMBER_ORB".to_string(),
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
                    item_id: "EMBER_ORB".to_string(),
                },
            ])
        );
        assert_eq!(
            turn.state_hashes().get(&2).map(String::as_str),
            Some("aaaabbbb")
        );
        assert_eq!(
            turn.state_hashes().get(&4).map(String::as_str),
            Some("aaaabbbb")
        );
        assert_eq!(sync.state_hash_disagreement(1), None);
    }

    #[test]
    fn battle_action_sync_rejects_unknown_players_and_empty_hashes() {
        let mut sync = BattleActionSyncBuffer::new([1, 2]).expect("battle action buffer");

        assert_eq!(
            BattleActionFrame::new(0, 1, BattleAction::Move { slot: 0 }, "11111111"),
            Err(BattleSyncError::InvalidPlayerIdentity { player_id: 0 })
        );
        assert_eq!(
            sync.insert_action(
                BattleActionFrame::new(3, 1, BattleAction::Move { slot: 0 }, "11111111")
                    .expect("action"),
            ),
            Err(BattleSyncError::UnknownPlayer { player_id: 3 })
        );
        assert_eq!(
            BattleActionFrame::with_state_hash(1, 1, BattleAction::Move { slot: 0 }, ""),
            Err(BattleSyncError::EmptyStateHash)
        );
        assert_eq!(
            BattleActionFrame::with_state_hash(1, 1, BattleAction::Move { slot: 0 }, " 1111"),
            Err(BattleSyncError::InvalidStateHash {
                state_hash: " 1111".to_string(),
            })
        );
        assert_eq!(
            BattleActionFrame::with_state_hash(1, 1, BattleAction::Move { slot: 0 }, "AAAABBBB"),
            Err(BattleSyncError::InvalidStateHash {
                state_hash: "AAAABBBB".to_string(),
            })
        );
        assert_eq!(
            BattleActionFrame::with_state_hash(1, 1, BattleAction::Move { slot: 0 }, "1111"),
            Err(BattleSyncError::InvalidStateHash {
                state_hash: "1111".to_string(),
            })
        );
        assert_eq!(
            sync.insert_action(BattleActionFrame::new_unchecked_for_tests(
                1,
                1,
                BattleAction::Move { slot: 0 },
                "2222 ".to_string(),
            )),
            Err(BattleSyncError::InvalidStateHash {
                state_hash: "2222 ".to_string(),
            })
        );
        assert_eq!(
            BattleActionFrame::new(
                1,
                1,
                BattleAction::Move {
                    slot: BATTLE_MOVE_SLOTS,
                },
                "11111111",
            ),
            Err(BattleSyncError::InvalidMoveSlot {
                slot: BATTLE_MOVE_SLOTS,
            })
        );
        assert_eq!(
            BattleActionFrame::new(
                1,
                1,
                BattleAction::MoveSwitch {
                    slot: 0,
                    party_index: 1,
                },
                "11111111",
            )
            .map(|frame| frame.action().clone()),
            Ok(BattleAction::MoveSwitch {
                slot: 0,
                party_index: 1,
            })
        );
        assert_eq!(
            BattleActionFrame::new(
                1,
                1,
                BattleAction::MoveSwitch {
                    slot: BATTLE_MOVE_SLOTS,
                    party_index: 1,
                },
                "11111111",
            ),
            Err(BattleSyncError::InvalidMoveSlot {
                slot: BATTLE_MOVE_SLOTS,
            })
        );
        assert_eq!(
            BattleActionFrame::new(
                1,
                1,
                BattleAction::MoveSwitch {
                    slot: 0,
                    party_index: PARTY_SIZE,
                },
                "11111111",
            ),
            Err(BattleSyncError::InvalidSwitchPartyIndex {
                party_index: PARTY_SIZE,
            })
        );
        assert_eq!(
            BattleActionFrame::new(
                1,
                1,
                BattleAction::Switch {
                    party_index: PARTY_SIZE,
                },
                "11111111",
            ),
            Err(BattleSyncError::InvalidSwitchPartyIndex {
                party_index: PARTY_SIZE,
            })
        );
        assert_eq!(
            BattleActionFrame::new(
                1,
                1,
                BattleAction::Item {
                    item_id: "EMBER ORB".to_string(),
                },
                "11111111",
            ),
            Err(BattleSyncError::InvalidItemId {
                item_id: "EMBER ORB".to_string()
            })
        );
        assert_eq!(
            BattleActionFrame::new(
                1,
                1,
                BattleAction::Item {
                    item_id: " POTION".to_string(),
                },
                "11111111",
            ),
            Err(BattleSyncError::InvalidItemId {
                item_id: " POTION".to_string(),
            })
        );
        assert!(
            BattleActionFrame::new(
                1,
                1,
                BattleAction::Item {
                    item_id: "EMBER_ORB".to_string(),
                },
                "11111111",
            )
            .is_ok()
        );
    }

    #[test]
    fn battle_action_turn_rejects_empty_or_malformed_aggregate_payloads() {
        assert_eq!(
            BattleActionTurn::new(3, BTreeMap::new(), BTreeMap::new()),
            Err(BattleSyncError::EmptyRoster)
        );

        assert_eq!(
            BattleActionTurn::new(
                3,
                BTreeMap::from([(0, BattleAction::Move { slot: 0 })]),
                BTreeMap::from([(0, "11111111".to_string())]),
            ),
            Err(BattleSyncError::InvalidPlayerIdentity { player_id: 0 })
        );

        assert_eq!(
            BattleActionTurn::new(
                3,
                BTreeMap::from([(
                    1,
                    BattleAction::Item {
                        item_id: "EMBER ORB".to_string(),
                    },
                )]),
                BTreeMap::from([(1, "11111111".to_string())]),
            ),
            Err(BattleSyncError::InvalidItemId {
                item_id: "EMBER ORB".to_string(),
            })
        );
        assert_eq!(
            BattleActionTurn::new(
                3,
                BTreeMap::from([(
                    1,
                    BattleAction::Move {
                        slot: BATTLE_MOVE_SLOTS,
                    },
                )]),
                BTreeMap::from([(1, "11111111".to_string())]),
            ),
            Err(BattleSyncError::InvalidMoveSlot {
                slot: BATTLE_MOVE_SLOTS,
            })
        );
        assert_eq!(
            BattleActionTurn::new(
                3,
                BTreeMap::from([(
                    1,
                    BattleAction::Switch {
                        party_index: PARTY_SIZE,
                    },
                )]),
                BTreeMap::from([(1, "11111111".to_string())]),
            ),
            Err(BattleSyncError::InvalidSwitchPartyIndex {
                party_index: PARTY_SIZE,
            })
        );

        assert_eq!(
            BattleActionTurn::new(
                3,
                BTreeMap::from([(1, BattleAction::Move { slot: 0 })]),
                BTreeMap::new(),
            ),
            Err(BattleSyncError::EmptyRoster)
        );

        assert_eq!(
            BattleActionTurn::new(
                3,
                BTreeMap::from([(1, BattleAction::Move { slot: 0 })]),
                BTreeMap::from([(2, "11111111".to_string())]),
            ),
            Err(BattleSyncError::MissingStateHash { player_id: 1 })
        );

        assert_eq!(
            BattleActionTurn::new(
                3,
                BTreeMap::from([(1, BattleAction::Move { slot: 0 })]),
                BTreeMap::from([(1, "11111111".to_string()), (2, "11111111".to_string())]),
            ),
            Err(BattleSyncError::UnexpectedStateHash { player_id: 2 })
        );
        assert_eq!(
            BattleActionTurn::new(
                3,
                BTreeMap::from([(1, BattleAction::Move { slot: 0 })]),
                BTreeMap::from([(1, "1111".to_string())]),
            ),
            Err(BattleSyncError::InvalidStateHash {
                state_hash: "1111".to_string(),
            })
        );
        assert_eq!(
            BattleActionTurn::new(
                3,
                BTreeMap::from([(1, BattleAction::Move { slot: 0 })]),
                BTreeMap::from([(1, "AAAABBBB".to_string())]),
            ),
            Err(BattleSyncError::InvalidStateHash {
                state_hash: "AAAABBBB".to_string(),
            })
        );
    }

    #[test]
    fn battle_action_sync_reports_duplicates_conflicts_and_hash_disagreements() {
        let mut sync = BattleActionSyncBuffer::new([1, 2]).expect("battle action buffer");
        let action =
            BattleActionFrame::with_state_hash(1, 7, BattleAction::Move { slot: 0 }, "11111111")
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
                BattleActionFrame::with_state_hash(
                    1,
                    7,
                    BattleAction::Move { slot: 0 },
                    "99999999",
                )
                .expect("hash conflict")
            ),
            Ok(InsertBattleActionResult::Conflict)
        );
        assert_eq!(
            sync.insert_action(
                BattleActionFrame::with_state_hash(1, 7, BattleAction::Run, "33333333",)
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
                    "22222222",
                )
                .expect("player 2")
            ),
            Ok(InsertBattleActionResult::Inserted)
        );

        assert_eq!(
            sync.state_hash_disagreement(7),
            Some(vec![
                (1, "11111111".to_string()),
                (2, "22222222".to_string())
            ])
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
    fn battle_action_sync_requires_exact_roster_cardinality_for_ready_turns() {
        let mut sync = BattleActionSyncBuffer::new([1, 2]).expect("battle action buffer");
        sync.actions.insert(
            4,
            BTreeMap::from([
                (1, BattleAction::Move { slot: 0 }),
                (2, BattleAction::Move { slot: 1 }),
                (3, BattleAction::Run),
            ]),
        );

        assert!(!sync.is_turn_ready(4));
        assert_eq!(sync.turn(4), None);
        assert_eq!(sync.next_ready_turn(4), None);
    }

    #[test]
    fn battle_action_sync_requires_exact_hash_roster_for_ready_turns() {
        let mut sync = BattleActionSyncBuffer::new([1, 2]).expect("battle action buffer");
        sync.actions.insert(
            4,
            BTreeMap::from([
                (1, BattleAction::Move { slot: 0 }),
                (2, BattleAction::Move { slot: 1 }),
            ]),
        );
        sync.state_hashes
            .insert(4, BTreeMap::from([(1, "11111111".to_string())]));

        assert!(!sync.is_turn_ready(4));
        assert_eq!(sync.turn(4), None);

        sync.state_hashes.insert(
            4,
            BTreeMap::from([
                (1, "11111111".to_string()),
                (2, "22222222".to_string()),
                (3, "33333333".to_string()),
            ]),
        );

        assert!(!sync.is_turn_ready(4));
        assert_eq!(sync.turn(4), None);
    }

    #[test]
    fn battle_action_sync_rejects_empty_rosters() {
        assert_eq!(
            BattleActionSyncBuffer::new(std::iter::empty::<PlayerId>()),
            Err(BattleSyncError::EmptyRoster)
        );
        assert_eq!(
            BattleActionSyncBuffer::new([0, 1]),
            Err(BattleSyncError::InvalidPlayerIdentity { player_id: 0 })
        );
    }

    #[test]
    fn battle_action_link_message_carries_exact_modpack_item_ids() {
        let message = LinkMessage::BattleAction(
            BattleActionFrame::new(
                2,
                9,
                BattleAction::Item {
                    item_id: "EMBER_ORB".to_string(),
                },
                "11111111",
            )
            .expect("action"),
        );
        let json = serde_json::to_string(&message).expect("serialize action message");

        assert!(json.contains(r#""type":"battle_action""#));
        assert!(json.contains(r#""item_id":"EMBER_ORB""#));
        assert!(json.contains(r#""state_hash":"11111111""#));
        assert_eq!(
            serde_json::from_str::<LinkMessage>(&json).expect("deserialize action message"),
            message
        );
    }

    #[test]
    fn session_battle_action_link_message_carries_exact_pack_bound_session_identity() {
        let session = test_session("session-1", test_modpack("core-modular", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd"))
            .expect("session identity");
        let action = BattleActionFrame::new(
            2,
            9,
            BattleAction::Item {
                item_id: "EMBER_ORB".to_string(),
            },
            "11111111",
        )
        .expect("action");
        let bound =
            SessionBattleActionFrame::new(session.clone(), action.clone()).expect("bound action");
        let message = LinkMessage::SessionBattleAction(bound.clone());
        let json = serde_json::to_string(&message).expect("serialize bound action message");

        assert!(json.contains(r#""type":"session_battle_action""#), "{json}");
        assert!(json.contains(r#""session_id":"session-1""#), "{json}");
        assert!(json.contains(r#""id":"core-modular""#), "{json}");
        assert!(json.contains(r#""hash":"1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd""#), "{json}");
        assert!(
            json.contains(&format!(r#""pack_content_hash":"{}""#, pack_content_hash())),
            "{json}"
        );
        assert!(json.contains(r#""item_id":"EMBER_ORB""#), "{json}");
        assert!(json.contains(r#""state_hash":"11111111""#), "{json}");
        assert_eq!(
            serde_json::from_str::<LinkMessage>(&json).expect("deserialize bound action message"),
            message
        );
        assert_eq!(bound.session(), &session);
        assert_eq!(bound.action(), &action);

        let invalid_session = LinkSessionIdentity::new_unchecked_for_tests(
            LINK_PROTOCOL_VERSION,
            " session-1",
            test_modpack("core-modular", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd"),
            pack_content_hash(),
        );
        let invalid_bound =
            SessionBattleActionFrame::new_unchecked_for_tests(invalid_session, action);
        assert!(matches!(
            LinkMessage::SessionBattleAction(invalid_bound).validate(),
            Err(MultiplayerMessageError::InvalidBattleAction { .. })
        ));
    }

    #[test]
    fn trade_sync_swaps_confirmed_party_slots_without_item_id_coercion() {
        let session =
            test_session("session-1", test_modpack("core-modular", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd")).expect("session");
        let mut lobby = LinkLobby::new(session, test_player(1)).expect("lobby");
        lobby
            .accept_hello(
                test_hello(
                    "session-1",
                    test_modpack("core-modular", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd"),
                    test_player(2),
                )
                .expect("hello"),
            )
            .expect("accept");

        let pikachu = pokemon("PIKACHU", Some("EMBER_ORB"));
        let eevee = pokemon("EEVEE", Some("SUPER_POTION"));
        let mut party_one = party_with(0, pikachu.clone());
        let mut party_two = party_with(3, eevee.clone());
        let mut trade = lobby.trade_buffer("trade-1", 2, 1).expect("trade");

        assert_eq!(trade.participants().players(), [1, 2]);
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
            trade.insert_confirmation(confirmation("trade-1", 1, true)),
            Ok(InsertTradeFrameResult::Inserted)
        );
        assert_eq!(
            trade.insert_confirmation(confirmation("trade-1", 2, true)),
            Ok(InsertTradeFrameResult::Inserted)
        );

        let outcome = trade.outcome().expect("outcome");
        assert!(!outcome.cancelled());
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
            Some("SUPER_POTION")
        );
        assert_eq!(
            party_two.pokemon[3]
                .as_ref()
                .and_then(|pokemon| pokemon.item.as_deref()),
            Some("EMBER_ORB")
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
            .insert_offer(TradeOffer::new("trade-1", 2, 1, eevee).expect("offer two"))
            .expect("offer two");
        trade
            .insert_confirmation(confirmation("trade-1", 1, false))
            .expect("cancel");
        trade
            .insert_confirmation(confirmation("trade-1", 2, true))
            .expect("confirm");

        let outcome = trade.outcome().expect("outcome");
        assert!(outcome.cancelled());
        assert_eq!(outcome.replacements().len(), 0);
        assert_eq!(outcome.apply_to_party(1, &mut party_one), Ok(None));
        assert_eq!(party_one.pokemon[0], Some(pikachu));
    }

    #[test]
    fn trade_outcome_requires_exact_replacement_shape_without_apply_fallbacks() {
        assert_eq!(
            TradeOutcome::new("trade-1", false, BTreeMap::new()),
            Err(TradeError::InvalidReplacementCount {
                trade_id: "trade-1".to_string(),
                expected: 2,
                actual: 0,
            })
        );

        assert_eq!(
            TradeOutcome::new(
                "trade-1",
                false,
                BTreeMap::from([(
                    1,
                    TradeReplacement::new(0, pokemon("PIKACHU", None)).expect("replacement"),
                )]),
            ),
            Err(TradeError::InvalidReplacementCount {
                trade_id: "trade-1".to_string(),
                expected: 2,
                actual: 1,
            })
        );

        assert_eq!(
            TradeOutcome::new(
                "trade-1",
                true,
                BTreeMap::from([(
                    1,
                    TradeReplacement::new(0, pokemon("PIKACHU", None)).expect("replacement"),
                )]),
            ),
            Err(TradeError::InvalidReplacementCount {
                trade_id: "trade-1".to_string(),
                expected: 0,
                actual: 1,
            })
        );
    }

    #[test]
    fn trade_sync_rejects_unknown_players_wrong_trade_ids_and_empty_slots() {
        let session =
            test_session("session-1", test_modpack("core-modular", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd")).expect("session");
        let lobby = LinkLobby::new(session, test_player(1)).expect("lobby");

        assert_eq!(
            lobby.trade_buffer("trade-1", 1, 2),
            Err(TradeError::UnknownPlayer { player_id: 2 })
        );
        assert_eq!(
            TradeParticipants::new("trade-1", 1, 1),
            Err(TradeError::DuplicateParticipant { player_id: 1 })
        );
        assert_eq!(
            TradeParticipants::new("trade-1", 0, 1),
            Err(TradeError::InvalidPlayerIdentity { player_id: 0 })
        );
        assert_eq!(
            TradeParticipants::new(" trade-1", 1, 2),
            Err(TradeError::InvalidTradeId {
                trade_id: " trade-1".to_string(),
            })
        );
        assert_eq!(
            TradeOffer::from_party("trade-1 ", 1, &Party::default(), 0),
            Err(TradeError::InvalidTradeId {
                trade_id: "trade-1 ".to_string(),
            })
        );
        assert_eq!(
            TradeOffer::from_party("trade-1", 1, &Party::default(), 0),
            Err(TradeError::EmptyPartySlot { party_slot: 0 })
        );
        assert_eq!(
            TradeOffer::new("trade-1", 0, 0, pokemon("PIKACHU", None)),
            Err(TradeError::InvalidPlayerIdentity { player_id: 0 })
        );
        assert_eq!(
            TradeOffer::new("trade-1", 1, PARTY_SIZE, pokemon("PIKACHU", None)),
            Err(TradeError::InvalidPartySlot {
                party_slot: PARTY_SIZE,
            })
        );
        let mut impossible_pokemon = pokemon("PIKACHU", None);
        impossible_pokemon.level = 0;
        assert_eq!(
            TradeOffer::new("trade-1", 1, 0, impossible_pokemon),
            Err(TradeError::InvalidPokemon {
                trade_id: "trade-1".to_string(),
                message: "pokemon.level 0 is outside range 1..100".to_string(),
            })
        );

        assert_eq!(
            TradeConfirmation::new(" trade-1", 1, true),
            Err(TradeError::InvalidTradeId {
                trade_id: " trade-1".to_string(),
            })
        );
        assert_eq!(
            TradeConfirmation::new("trade 1", 1, true),
            Err(TradeError::InvalidTradeId {
                trade_id: "trade 1".to_string(),
            })
        );
        assert_eq!(
            TradeConfirmation::new("trade-1", 0, true),
            Err(TradeError::InvalidPlayerIdentity { player_id: 0 })
        );
        let mut trade =
            TradeSyncBuffer::new(TradeParticipants::new("trade-1", 1, 2).expect("participants"));
        assert_eq!(
            trade.insert_confirmation(confirmation("trade-2", 1, true)),
            Err(TradeError::TradeIdMismatch {
                expected: "trade-1".to_string(),
                actual: "trade-2".to_string(),
            })
        );
        assert_eq!(
            trade.insert_confirmation(confirmation("trade-1", 3, true)),
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
        let offer = TradeOffer::new("trade-1", 1, 0, pikachu).expect("offer");

        assert_eq!(
            trade.insert_offer(offer.clone()),
            Ok(InsertTradeFrameResult::Inserted)
        );
        assert_eq!(
            trade.insert_offer(offer),
            Ok(InsertTradeFrameResult::Duplicate)
        );
        assert_eq!(
            trade.insert_offer(TradeOffer::new("trade-1", 1, 1, eevee).expect("conflict offer")),
            Ok(InsertTradeFrameResult::Conflict)
        );
        assert_eq!(
            trade.insert_confirmation(confirmation("trade-1", 1, true)),
            Ok(InsertTradeFrameResult::Inserted)
        );
        assert_eq!(
            trade.insert_confirmation(confirmation("trade-1", 1, true)),
            Ok(InsertTradeFrameResult::Duplicate)
        );
        assert_eq!(
            trade.insert_confirmation(confirmation("trade-1", 1, false)),
            Ok(InsertTradeFrameResult::Conflict)
        );
    }

    #[test]
    fn trade_link_messages_carry_exact_pokemon_payloads() {
        let offer =
            TradeOffer::new("trade-1", 1, 0, pokemon("PIKACHU", Some("EMBER_ORB"))).expect("offer");
        let offer_message = LinkMessage::TradeOffer(offer.clone());
        let offer_json = serde_json::to_string(&offer_message).expect("serialize offer");

        assert!(offer_json.contains(r#""type":"trade_offer""#));
        assert!(offer_json.contains(r#""item":"EMBER_ORB""#));
        assert_eq!(
            serde_json::from_str::<LinkMessage>(&offer_json).expect("deserialize offer"),
            offer_message
        );

        let confirm_message = LinkMessage::TradeConfirmation(confirmation("trade-1", 1, true));
        let confirm_json = serde_json::to_string(&confirm_message).expect("serialize confirm");
        assert!(confirm_json.contains(r#""type":"trade_confirmation""#));
        assert_eq!(
            serde_json::from_str::<LinkMessage>(&confirm_json).expect("deserialize confirm"),
            confirm_message
        );
    }

    #[test]
    fn session_trade_offer_link_message_carries_exact_pack_bound_session_identity() {
        let session = test_session("session-trade-1", test_modpack("core-crystal", "1234abc11234abc11234abc11234abc11234abc11234abc11234abc11234abc1"))
            .expect("session");
        let offer =
            TradeOffer::new("trade-1", 1, 0, pokemon("PIKACHU", Some("EMBER_ORB"))).expect("offer");
        let message = LinkMessage::SessionTradeOffer(
            SessionTradeOffer::new(session.clone(), offer.clone()).expect("session offer"),
        );
        let json = serde_json::to_string(&message).expect("serialize session trade offer");

        assert!(json.contains(r#""type":"session_trade_offer""#));
        assert!(json.contains(r#""session_id":"session-trade-1""#));
        assert!(json.contains(r#""id":"core-crystal""#));
        assert!(json.contains(r#""hash":"1234abc11234abc11234abc11234abc11234abc11234abc11234abc11234abc1""#));
        assert!(json.contains(r#""pack_content_hash":"0102030401020304010203040102030401020304010203040102030401020304""#));
        assert!(json.contains(r#""trade_id":"trade-1""#));
        assert!(json.contains(r#""item":"EMBER_ORB""#));
        assert_eq!(
            serde_json::from_str::<LinkMessage>(&json).expect("deserialize session trade offer"),
            message
        );

        let decoded =
            serde_json::from_str::<LinkMessage>(&json).expect("decode session trade offer");
        match decoded {
            LinkMessage::SessionTradeOffer(frame) => {
                assert_eq!(frame.session(), &session);
                assert_eq!(frame.offer(), &offer);
            }
            other => panic!("unexpected message: {other:?}"),
        }
    }

    #[test]
    fn session_trade_confirmation_link_message_carries_exact_pack_bound_session_identity() {
        let session = test_session("session-trade-2", test_modpack("core-crystal", "1234abc21234abc21234abc21234abc21234abc21234abc21234abc21234abc2"))
            .expect("session");
        let confirmation = confirmation("trade-1", 2, true);
        let message = LinkMessage::SessionTradeConfirmation(
            SessionTradeConfirmation::new(session.clone(), confirmation.clone())
                .expect("session confirmation"),
        );
        let json = serde_json::to_string(&message).expect("serialize session trade confirmation");

        assert!(json.contains(r#""type":"session_trade_confirmation""#));
        assert!(json.contains(r#""session_id":"session-trade-2""#));
        assert!(json.contains(r#""id":"core-crystal""#));
        assert!(json.contains(r#""hash":"1234abc21234abc21234abc21234abc21234abc21234abc21234abc21234abc2""#));
        assert!(json.contains(r#""pack_content_hash":"0102030401020304010203040102030401020304010203040102030401020304""#));
        assert!(json.contains(r#""trade_id":"trade-1""#));
        assert_eq!(
            serde_json::from_str::<LinkMessage>(&json)
                .expect("deserialize session trade confirmation"),
            message
        );

        let decoded =
            serde_json::from_str::<LinkMessage>(&json).expect("decode session trade confirmation");
        match decoded {
            LinkMessage::SessionTradeConfirmation(frame) => {
                assert_eq!(frame.session(), &session);
                assert_eq!(frame.confirmation(), &confirmation);
            }
            other => panic!("unexpected message: {other:?}"),
        }
    }

    #[test]
    fn session_trade_messages_reject_invalid_session_identity() {
        let session = LinkSessionIdentity::new_unchecked_for_tests(
            LINK_PROTOCOL_VERSION,
            " bad-session",
            test_modpack("core-crystal", "1234abc11234abc11234abc11234abc11234abc11234abc11234abc11234abc1"),
            pack_content_hash(),
        );
        let offer = TradeOffer::new("trade-1", 1, 0, pokemon("PIKACHU", None)).expect("offer");
        let confirmation = confirmation("trade-1", 1, true);

        assert!(SessionTradeOffer::new(session.clone(), offer).is_err());
        assert!(SessionTradeConfirmation::new(session, confirmation).is_err());

        let invalid_offer_json = serde_json::json!({
            "type": "session_trade_offer",
            "session": {
                "protocol_version": LINK_PROTOCOL_VERSION,
                "session_id": " bad-session",
                "modpack": {
                    "id": "core-crystal",
                    "hash": "1234abc11234abc11234abc11234abc11234abc11234abc11234abc11234abc1"
                },
                "pack_content_hash": pack_content_hash()
            },
            "offer": {
                "trade_id": "trade-1",
                "player_id": 1,
                "party_slot": 0,
                "pokemon": pokemon("PIKACHU", None)
            }
        });
        assert!(
            serde_json::from_value::<LinkMessage>(invalid_offer_json)
                .expect_err("invalid offer session rejected")
                .to_string()
                .contains("bad-session")
        );

        let invalid_confirmation_json = serde_json::json!({
            "type": "session_trade_confirmation",
            "session": {
                "protocol_version": LINK_PROTOCOL_VERSION,
                "session_id": " bad-session",
                "modpack": {
                    "id": "core-crystal",
                    "hash": "1234abc11234abc11234abc11234abc11234abc11234abc11234abc11234abc1"
                },
                "pack_content_hash": pack_content_hash()
            },
            "confirmation": {
                "trade_id": "trade-1",
                "player_id": 1,
                "confirm": true
            }
        });
        assert!(
            serde_json::from_value::<LinkMessage>(invalid_confirmation_json)
                .expect_err("invalid confirmation session rejected")
                .to_string()
                .contains("bad-session")
        );
    }

    #[test]
    fn link_cable_preamble_establishes_exact_two_player_stream() {
        let mut host = LinkCableState::new(1, 2).expect("host");
        let mut client = LinkCableState::new(2, 1).expect("client");

        let preamble = host.host_preamble();
        assert_eq!(preamble.byte(), LINK_PREAMBLE_BYTE);
        assert_eq!(preamble.clock(), 1);
        let response = client
            .client_accept_preamble(preamble)
            .expect("client accept")
            .expect("response");
        assert_eq!(response.byte(), LINK_PREAMBLE_RESPONSE);
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
    fn session_link_byte_message_carries_exact_pack_bound_session_identity() {
        let session = test_session("session-1", test_modpack("core-modular", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd"))
            .expect("session identity");
        let frame = LinkByteFrame::new(2, LINK_PREAMBLE_RESPONSE, 7).expect("frame");
        let bound = SessionLinkByteFrame::new(session.clone(), frame.clone()).expect("bound byte");
        let message = LinkMessage::SessionLinkByte(bound.clone());
        let json = serde_json::to_string(&message).expect("serialize bound byte");

        assert!(json.contains(r#""type":"session_link_byte""#), "{json}");
        assert!(json.contains(r#""session_id":"session-1""#), "{json}");
        assert!(json.contains(r#""id":"core-modular""#), "{json}");
        assert!(json.contains(r#""hash":"1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd""#), "{json}");
        assert!(
            json.contains(&format!(r#""pack_content_hash":"{}""#, pack_content_hash())),
            "{json}"
        );
        assert!(json.contains(r#""player_id":2"#), "{json}");
        assert!(json.contains(r#""byte":97"#), "{json}");
        assert!(json.contains(r#""clock":7"#), "{json}");
        assert_eq!(
            serde_json::from_str::<LinkMessage>(&json).expect("deserialize bound byte"),
            message
        );
        assert_eq!(bound.session(), &session);
        assert_eq!(bound.frame(), &frame);
    }

    #[test]
    fn link_cable_rejects_wrong_peer_and_clock_regression() {
        let mut cable = LinkCableState::new(1, 2).expect("cable");

        assert_eq!(
            LinkByteFrame::new(0, 0x42, 1),
            Err(LinkCableError::InvalidPlayerIdentity { player_id: 0 })
        );
        assert_eq!(
            LinkClockSyncFrame::new(0, 1, 2, 3),
            Err(LinkCableError::InvalidPlayerIdentity { player_id: 0 })
        );
        assert_eq!(
            cable.receive_byte_frame(LinkByteFrame::new_unchecked_for_tests(2, 0x42, 0)),
            Err(LinkCableError::InvalidClock { clock: 0 })
        );
        assert_eq!(
            cable.receive_byte_frame(LinkByteFrame::new(3, 0x42, 1).expect("frame")),
            Err(LinkCableError::UnexpectedPeer {
                expected: 2,
                player_id: 3,
            })
        );
        cable
            .receive_byte_frame(LinkByteFrame::new(2, 0x42, 1).expect("frame"))
            .expect("first");
        assert_eq!(
            cable.receive_byte_frame(LinkByteFrame::new(2, 0x99, 1).expect("frame")),
            Err(LinkCableError::ClockRegression {
                remote_clock: 1,
                clock: 1,
            })
        );
    }

    #[test]
    fn link_cable_from_lobby_requires_accepted_players() {
        let session =
            test_session("session-1", test_modpack("core-modular", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd")).expect("session");
        let lobby = LinkLobby::new(session, test_player(1)).expect("lobby");

        assert_eq!(
            LinkCableState::from_lobby(&lobby, 1, 2),
            Err(LinkCableError::UnknownPlayer { player_id: 2 })
        );
        assert_eq!(
            LinkCableState::new(1, 1),
            Err(LinkCableError::DuplicateEndpoint { player_id: 1 })
        );
        assert_eq!(
            LinkCableState::new(0, 1),
            Err(LinkCableError::InvalidPlayerIdentity { player_id: 0 })
        );
        assert_eq!(
            LinkCableState::new(1, 0),
            Err(LinkCableError::InvalidPlayerIdentity { player_id: 0 })
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
    fn session_link_clock_sync_message_carries_exact_pack_bound_session_identity() {
        let session = test_session("session-1", test_modpack("core-modular", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd"))
            .expect("session identity");
        let frame = LinkClockSyncFrame::new(2, 10, 11, 12).expect("sync");
        let bound =
            SessionLinkClockSyncFrame::new(session.clone(), frame.clone()).expect("bound sync");
        let message = LinkMessage::SessionLinkClockSync(bound.clone());
        let json = serde_json::to_string(&message).expect("serialize bound sync");

        assert!(
            json.contains(r#""type":"session_link_clock_sync""#),
            "{json}"
        );
        assert!(json.contains(r#""session_id":"session-1""#), "{json}");
        assert!(json.contains(r#""id":"core-modular""#), "{json}");
        assert!(json.contains(r#""hash":"1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd""#), "{json}");
        assert!(
            json.contains(&format!(r#""pack_content_hash":"{}""#, pack_content_hash())),
            "{json}"
        );
        assert!(json.contains(r#""player_id":2"#), "{json}");
        assert!(json.contains(r#""t0":10"#), "{json}");
        assert!(json.contains(r#""t1":11"#), "{json}");
        assert!(json.contains(r#""t2":12"#), "{json}");
        assert_eq!(
            serde_json::from_str::<LinkMessage>(&json).expect("deserialize bound sync"),
            message
        );
        assert_eq!(bound.session(), &session);
        assert_eq!(bound.frame(), &frame);
    }

    #[test]
    fn session_link_cable_messages_reject_invalid_session_identity() {
        let invalid_session = LinkSessionIdentity::new_unchecked_for_tests(
            LINK_PROTOCOL_VERSION,
            " session-1",
            test_modpack("core-modular", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd"),
            pack_content_hash(),
        );
        let byte = LinkByteFrame::new(2, LINK_PREAMBLE_RESPONSE, 7).expect("byte");
        let sync = LinkClockSyncFrame::new(2, 10, 11, 12).expect("sync");

        assert!(matches!(
            LinkMessage::SessionLinkByte(SessionLinkByteFrame::new_unchecked_for_tests(
                invalid_session.clone(),
                byte
            ))
            .validate(),
            Err(MultiplayerMessageError::InvalidLinkCableFrame { .. })
        ));
        assert!(matches!(
            LinkMessage::SessionLinkClockSync(SessionLinkClockSyncFrame::new_unchecked_for_tests(
                invalid_session,
                sync
            ))
            .validate(),
            Err(MultiplayerMessageError::InvalidLinkCableFrame { .. })
        ));
    }

    #[test]
    fn link_cable_sync_rejects_impossible_clock_ordering() {
        let mut client = LinkCableState::new(2, 1).expect("client");
        assert_eq!(
            LinkClockSyncFrame::new(1, 0, 0, 0),
            Err(LinkCableError::InvalidClock { clock: 0 })
        );

        let invalid = LinkClockSyncFrame::new_unchecked_for_tests(1, 12, 11, 13);

        assert_eq!(
            client.receive_sync_frame(invalid, 20),
            Err(LinkCableError::InvalidClockSync {
                t0: 12,
                t1: 11,
                t2: 13,
            })
        );
        assert_eq!(client.remote_clock(), 0);
        assert_eq!(client.average_latency_ticks(), None);
    }

    #[test]
    fn link_cable_sync_frames_are_monotonic_when_caller_tick_stalls() {
        let mut host = LinkCableState::new(1, 2).expect("host");
        let mut client = LinkCableState::new(2, 1).expect("client");

        let first = host.sync_frame(7);
        let second = host.sync_frame(7);
        let third = host.sync_frame(6);

        assert_eq!(first.t2(), 7);
        assert_eq!(second.t2(), 8);
        assert_eq!(third.t2(), 9);
        client
            .receive_sync_frame(first, 12)
            .expect("first sync accepted");
        client
            .receive_sync_frame(second, 13)
            .expect("stalled tick sync accepted");
        client
            .receive_sync_frame(third, 14)
            .expect("regressed tick sync accepted");
        assert_eq!(client.remote_clock(), 9);
    }

    #[test]
    fn link_byte_messages_are_transport_neutral_json() {
        let message =
            LinkMessage::LinkByte(LinkByteFrame::new(2, LINK_PREAMBLE_RESPONSE, 7).expect("frame"));
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
            BattleRngState::new(0xf3dd, 0x56, 0x78).expect("rng state")
        );
        assert_eq!(BattleRngState::from_seed(0xa5a5).hardware_divider(), 1);
        assert_eq!(
            LinkMessage::RngInit {
                state: BattleRngState::new_unchecked_for_tests(0, 0x56, 0x78),
            }
            .validate(),
            Err(MultiplayerMessageError::InvalidBattleRng {
                message: "battle rng hardware divider must be nonzero".to_string(),
            })
        );
    }

    #[test]
    fn session_rng_init_link_message_carries_exact_pack_bound_session_identity() {
        let session = test_session("session-1", test_modpack("core-modular", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd"))
            .expect("session identity");
        let state = BattleRngState::new(0xf3dd, 0x56, 0x78).expect("rng state");
        let bound = SessionBattleRngInitFrame::new(session.clone(), state).expect("bound rng init");
        let message = LinkMessage::SessionRngInit(bound.clone());
        let json = serde_json::to_string(&message).expect("serialize bound rng init");

        assert!(json.contains(r#""type":"session_rng_init""#), "{json}");
        assert!(json.contains(r#""session_id":"session-1""#), "{json}");
        assert!(json.contains(r#""id":"core-modular""#), "{json}");
        assert!(json.contains(r#""hash":"1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd""#), "{json}");
        assert!(
            json.contains(&format!(r#""pack_content_hash":"{}""#, pack_content_hash())),
            "{json}"
        );
        assert!(json.contains(r#""hardware_divider":62429"#), "{json}");
        assert!(json.contains(r#""h_random_add":86"#), "{json}");
        assert!(json.contains(r#""h_random_sub":120"#), "{json}");
        assert_eq!(
            serde_json::from_str::<LinkMessage>(&json).expect("deserialize bound rng init"),
            message
        );
        assert_eq!(bound.session(), &session);
        assert_eq!(bound.state(), state);
    }

    #[test]
    fn session_rng_init_link_message_rejects_invalid_session_identity() {
        let state = BattleRngState::new(0xf3dd, 0x56, 0x78).expect("rng state");
        let invalid_session = LinkSessionIdentity::new_unchecked_for_tests(
            LINK_PROTOCOL_VERSION,
            " session-1",
            test_modpack("core-modular", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd"),
            pack_content_hash(),
        );
        let invalid_bound =
            SessionBattleRngInitFrame::new_unchecked_for_tests(invalid_session, state);

        assert!(matches!(
            LinkMessage::SessionRngInit(invalid_bound).validate(),
            Err(MultiplayerMessageError::InvalidBattleRng { .. })
        ));

        let invalid_json = serde_json::json!({
            "type": "session_rng_init",
            "session": {
                "protocol_version": LINK_PROTOCOL_VERSION,
                "session_id": " session-1",
                "modpack": {
                    "id": "core-modular",
                    "hash": "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd"
                },
                "pack_content_hash": pack_content_hash()
            },
            "state": {
                "hardware_divider": 62429,
                "h_random_add": 86,
                "h_random_sub": 120
            }
        });

        assert!(
            serde_json::from_value::<LinkMessage>(invalid_json)
                .expect_err("invalid session rng init rejected")
                .to_string()
                .contains("session-1")
        );
    }

    #[test]
    fn fnv_hash_matches_battle_synchronizer_reference_values() {
        assert_eq!(fnv1a32_hex(""), "811c9dc5");
        assert_eq!(fnv1a32_hex("battle-state"), "aa0a8273");
        assert_eq!(fnv1a32_hex_bytes(b"battle-state"), "aa0a8273");
    }

    #[test]
    fn lockstep_buffer_waits_for_all_players_and_orders_inputs() {
        let mut buffer = LockstepBuffer::new([2, 1]).expect("lockstep buffer");
        assert_eq!(
            buffer.insert_input(PlayerInputFrame::new(2, Frame(7), 0b0001_0000).expect("input")),
            Ok(InsertInputResult::Inserted)
        );
        assert!(!buffer.is_frame_ready(7));
        assert_eq!(
            buffer.insert_input(PlayerInputFrame::new(1, Frame(7), 0b1000_0000).expect("input")),
            Ok(InsertInputResult::Inserted)
        );

        let frame = buffer.frame(7).expect("ready frame");
        assert_eq!(frame.frame(), 7);
        assert_eq!(buffer.players(), vec![1, 2]);
        assert_eq!(
            frame.ordered_inputs(&buffer.players()),
            Some(vec![0b1000_0000, 0b0001_0000])
        );
    }

    #[test]
    fn deterministic_lockstep_applies_ready_frames_in_roster_order() {
        let mut lockstep = DeterministicLockstep::new([4, 2], 4).expect("lockstep");
        assert_eq!(lockstep.players(), vec![2, 4]);

        let first = lockstep
            .apply_frame(
                LockstepFrame::new(0, BTreeMap::from([(4, 0b0001_0001), (2, 0b1000_0000)]))
                    .expect("frame"),
            )
            .expect("apply first");

        assert_eq!(
            first,
            AppliedLockstepFrame::new(
                0,
                4,
                0b0001_0001,
                0b0001_0001,
                vec![0b1000_0000, 0b0001_0001],
            )
        );
        assert_eq!(lockstep.next_frame(), 1);
        assert_eq!(lockstep.previous_local_joypad_mask(), 0b0001_0001);

        let held_right = lockstep
            .apply_frame(
                LockstepFrame::new(1, BTreeMap::from([(2, 0), (4, 0b0000_0001)])).expect("frame"),
            )
            .expect("apply second");

        assert_eq!(held_right.local_joypad_mask(), 0b0000_0001);
        assert_eq!(held_right.local_pressed_mask(), 0);
        assert_eq!(held_right.ordered_inputs(), &[0, 0b0000_0001]);
        assert_eq!(lockstep.next_frame(), 2);
    }

    #[test]
    fn deterministic_lockstep_serializes_exact_saveable_cursor() {
        let mut lockstep = DeterministicLockstep::new([1, 2], 1).expect("lockstep");
        lockstep
            .apply_frame(
                LockstepFrame::new(0, BTreeMap::from([(1, 0x10), (2, 0x20)])).expect("frame"),
            )
            .expect("apply");

        let json = serde_json::to_string(&lockstep).expect("serialize lockstep");

        assert_eq!(
            json,
            r#"{"local_player_id":1,"players":[1,2],"next_frame":1,"previous_local_joypad_mask":16}"#
        );
        assert_eq!(
            serde_json::from_str::<DeterministicLockstep>(&json).expect("deserialize lockstep"),
            lockstep
        );
    }

    #[test]
    fn deterministic_input_journal_records_pack_bound_contiguous_lockstep_frames() {
        let session =
            test_session("session-1", test_modpack("core-modular", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd")).expect("session");
        let start_checksum = StateChecksumFrame::new(1, Frame(4), 0xaabb_ccdd);
        let frames = vec![
            LockstepFrame::new(4, BTreeMap::from([(1, 0x10), (2, 0x20)])).expect("frame 4"),
            LockstepFrame::new(5, BTreeMap::from([(1, 0x00), (2, 0x80)])).expect("frame 5"),
        ];

        let terminal_checksum = StateChecksumFrame::new(1, Frame(6), 0xbbcc_ddee);
        let journal = DeterministicInputJournal::new(
            session.clone(),
            [1, 2],
            start_checksum.clone(),
            terminal_checksum.clone(),
            frames,
        )
        .expect("journal");

        assert_eq!(journal.session(), &session);
        assert_eq!(journal.players(), &BTreeSet::from([1, 2]));
        assert_eq!(journal.start_checksum(), &start_checksum);
        assert_eq!(journal.terminal_checksum(), &terminal_checksum);
        assert_eq!(journal.frames().len(), 2);
        let canonical_bytes = journal.canonical_bytes().expect("journal bytes");
        assert_eq!(journal.fingerprint(), Ok(fnv1a32_bytes(&canonical_bytes)));
        assert_eq!(
            journal.fingerprint_hex(),
            Ok(format!("{:08x}", fnv1a32_bytes(&canonical_bytes)))
        );
        let json = serde_json::to_string(&journal).expect("serialize journal");
        assert!(json.contains(r#""session_id":"session-1""#));
        assert_eq!(
            serde_json::from_str::<DeterministicInputJournal>(&json)
                .expect("deserialize journal")
                .validate(),
            Ok(())
        );
        let journal_frame = DeterministicInputJournalFrame::new(journal.clone()).expect("frame");
        assert_eq!(
            journal_frame.fingerprint(),
            journal.fingerprint_hex().expect("journal fingerprint")
        );
        assert_eq!(journal_frame.journal(), &journal);
        assert_eq!(journal_frame.validate(), Ok(()));
    }

    #[test]
    fn deterministic_input_journal_rejects_missing_players_and_frame_gaps() {
        let session =
            test_session("session-1", test_modpack("core-modular", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd")).expect("session");
        let start_checksum = StateChecksumFrame::new(1, Frame(4), 0xaabb_ccdd);

        assert_eq!(
            DeterministicInputJournal::new(
                session.clone(),
                [1, 2],
                start_checksum.clone(),
                StateChecksumFrame::new(1, Frame(4), 0xbbcc_ddee),
                vec![LockstepFrame::new(5, BTreeMap::from([(1, 0), (2, 0)])).expect("frame")]
            ),
            Err(InputJournalError::FrameOutOfOrder {
                expected: 4,
                actual: 5,
            })
        );

        assert_eq!(
            DeterministicInputJournal::new(
                session,
                [1, 2],
                start_checksum,
                StateChecksumFrame::new(1, Frame(5), 0xbbcc_ddee),
                vec![LockstepFrame::new(4, BTreeMap::from([(1, 0)])).expect("frame")]
            ),
            Err(InputJournalError::MissingPlayerInput {
                frame: 4,
                player_id: 2,
            })
        );
    }

    #[test]
    fn deterministic_input_journal_requires_explicit_terminal_checksum_frame() {
        let session =
            test_session("session-1", test_modpack("core-modular", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd")).expect("session");

        assert_eq!(
            DeterministicInputJournal::new(
                session,
                [1, 2],
                StateChecksumFrame::new(1, Frame(4), 0xaabb_ccdd),
                StateChecksumFrame::new(1, Frame(4), 0xbbcc_ddee),
                vec![LockstepFrame::new(4, BTreeMap::from([(1, 0), (2, 0)])).expect("frame")]
            ),
            Err(InputJournalError::TerminalChecksumFrameMismatch {
                expected: 5,
                actual: 4,
            })
        );
    }

    #[test]
    fn deterministic_lockstep_validates_deserialized_cursor_without_fallbacks() {
        let empty_roster_error = serde_json::from_str::<DeterministicLockstep>(
            r#"{"local_player_id":1,"players":[],"next_frame":0,"previous_local_joypad_mask":0}"#,
        )
        .expect_err("lockstep deserialization must reject an empty roster")
        .to_string();
        assert!(
            empty_roster_error.contains("lockstep roster must contain at least one player"),
            "{empty_roster_error}"
        );

        let missing_local_error = serde_json::from_str::<DeterministicLockstep>(
            r#"{"local_player_id":3,"players":[1,2],"next_frame":0,"previous_local_joypad_mask":0}"#,
        )
        .expect_err("lockstep deserialization must reject a missing local player")
        .to_string();
        assert!(
            missing_local_error.contains("lockstep player 3 is not in the accepted link roster"),
            "{missing_local_error}"
        );

        let zero_local_error = serde_json::from_str::<DeterministicLockstep>(
            r#"{"local_player_id":0,"players":[0,1],"next_frame":0,"previous_local_joypad_mask":0}"#,
        )
        .expect_err("lockstep deserialization must reject zero player ids")
        .to_string();
        assert!(
            zero_local_error.contains("lockstep player id 0 is not a valid link identity"),
            "{zero_local_error}"
        );

        let conflicting_mask_error = serde_json::from_str::<DeterministicLockstep>(
            r#"{"local_player_id":1,"players":[1,2],"next_frame":0,"previous_local_joypad_mask":3}"#,
        )
        .expect_err("lockstep deserialization must reject conflicting joypad directions")
        .to_string();
        assert!(
            conflicting_mask_error
                .contains("lockstep input mask 0b00000011 has conflicting direction buttons"),
            "{conflicting_mask_error}"
        );
    }

    #[test]
    fn deterministic_lockstep_rejects_drift_without_roster_or_frame_fallbacks() {
        assert_eq!(
            DeterministicLockstep::new([1, 2], 3),
            Err(LockstepSyncError::UnknownPlayer { player_id: 3 })
        );

        let mut lockstep = DeterministicLockstep::new([1, 2], 1).expect("lockstep");
        assert_eq!(
            lockstep.apply_frame(
                LockstepFrame::new(1, BTreeMap::from([(1, 0), (2, 0)])).expect("frame"),
            ),
            Err(LockstepSyncError::FrameOutOfOrder {
                expected: 0,
                actual: 1,
            })
        );
        assert_eq!(lockstep.next_frame(), 0);

        assert_eq!(
            lockstep.apply_frame(LockstepFrame::new(0, BTreeMap::from([(1, 0)])).expect("frame"),),
            Err(LockstepSyncError::MissingPlayerInput {
                frame: 0,
                player_id: 2,
            })
        );
        assert_eq!(lockstep.next_frame(), 0);

        assert_eq!(
            lockstep.apply_frame(
                LockstepFrame::new(0, BTreeMap::from([(1, 0), (2, 0), (3, 0)])).expect("frame"),
            ),
            Err(LockstepSyncError::NonRosterPlayerInput {
                frame: 0,
                player_id: 3,
            })
        );
        assert_eq!(lockstep.next_frame(), 0);

        assert_eq!(
            lockstep.apply_frame(LockstepFrame::new_unchecked_for_tests(
                0,
                BTreeMap::from([(1, B_PAD_LEFT | B_PAD_RIGHT), (2, 0)]),
            )),
            Err(LockstepSyncError::ConflictingJoypadDirections {
                mask: B_PAD_LEFT | B_PAD_RIGHT,
            })
        );
        assert_eq!(lockstep.next_frame(), 0);

        let mut lockstep =
            DeterministicLockstep::new_unchecked_for_tests(1, BTreeSet::from([1, 2]), u64::MAX, 0);
        assert_eq!(
            lockstep.apply_frame(
                LockstepFrame::new(u64::MAX, BTreeMap::from([(1, 0), (2, 0)])).expect("frame"),
            ),
            Err(LockstepSyncError::FrameCursorOverflow { frame: u64::MAX })
        );
        assert_eq!(lockstep.next_frame(), u64::MAX);
    }

    #[test]
    fn lockstep_buffer_reports_duplicates_and_conflicts() {
        let mut buffer = LockstepBuffer::new([1, 2]).expect("lockstep buffer");
        let input = PlayerInputFrame::new(1, Frame(3), 0x10).expect("input");
        assert_eq!(
            buffer.insert_input(input.clone()),
            Ok(InsertInputResult::Inserted)
        );
        assert_eq!(buffer.insert_input(input), Ok(InsertInputResult::Duplicate));
        assert_eq!(
            buffer.insert_input(PlayerInputFrame::new(1, Frame(3), 0x20).expect("input")),
            Ok(InsertInputResult::Conflict)
        );
    }

    #[test]
    fn lockstep_buffer_rejects_conflicting_direction_masks() {
        let mut buffer = LockstepBuffer::new([1, 2]).expect("lockstep buffer");
        assert_eq!(
            buffer.insert_input(PlayerInputFrame::new_unchecked_for_tests(
                1,
                3,
                B_PAD_LEFT | B_PAD_RIGHT,
            )),
            Err(LockstepSyncError::ConflictingJoypadDirections {
                mask: B_PAD_LEFT | B_PAD_RIGHT,
            })
        );
        assert_eq!(buffer.frame(3), None);
    }

    #[test]
    fn lockstep_input_rejects_invalid_player_identity_without_roster_fallback() {
        assert_eq!(
            PlayerInputFrame::new(0, Frame(3), 0x10),
            Err(LockstepSyncError::InvalidPlayerIdentity { player_id: 0 })
        );

        let mut buffer = LockstepBuffer::new([1, 2]).expect("lockstep buffer");
        assert_eq!(
            buffer.insert_input(PlayerInputFrame::new_unchecked_for_tests(0, 3, 0x10)),
            Err(LockstepSyncError::InvalidPlayerIdentity { player_id: 0 })
        );
    }

    #[test]
    fn lockstep_frame_rejects_empty_input_sets_without_idle_fallback() {
        assert_eq!(
            LockstepFrame::new(4, BTreeMap::new()),
            Err(LockstepSyncError::EmptyRoster)
        );
        assert_eq!(
            LockstepFrame::new(4, BTreeMap::from([(0, 0x10)])),
            Err(LockstepSyncError::InvalidPlayerIdentity { player_id: 0 })
        );
    }

    #[test]
    fn lockstep_buffer_reports_checksum_duplicates_and_conflicts_without_overwrite() {
        let mut buffer = LockstepBuffer::new([1, 2]).expect("lockstep buffer");
        let checksum = StateChecksumFrame::new(1, Frame(3), 0xaaaa);
        assert_eq!(
            buffer.insert_checksum_frame(checksum.clone()),
            Ok(InsertChecksumResult::Inserted)
        );
        assert_eq!(
            buffer.insert_checksum_frame(checksum),
            Ok(InsertChecksumResult::Duplicate)
        );
        assert_eq!(
            buffer.insert_checksum_frame(StateChecksumFrame::new(1, Frame(3), 0xbbbb)),
            Ok(InsertChecksumResult::Conflict)
        );
        buffer
            .insert_checksum_frame(StateChecksumFrame::new(2, Frame(3), 0xaaaa))
            .expect("player 2 checksum");

        assert_eq!(buffer.checksum_disagreement(3), None);
    }

    #[test]
    fn lockstep_buffer_detects_state_hash_disagreement_after_all_players_report() {
        let mut buffer = LockstepBuffer::new([1, 2]).expect("lockstep buffer");
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
        let mut buffer = LockstepBuffer::new([1, 2]).expect("lockstep buffer");

        assert_eq!(
            buffer.insert_input(PlayerInputFrame::new(3, Frame(7), 0x10).expect("input")),
            Err(LockstepSyncError::UnknownPlayer { player_id: 3 })
        );
        assert_eq!(buffer.players(), vec![1, 2]);
        assert_eq!(
            buffer.insert_checksum(3, StateChecksum::new(7, 0xaaaa),),
            Err(LockstepSyncError::UnknownPlayer { player_id: 3 })
        );
        assert_eq!(buffer.players(), vec![1, 2]);
        assert_eq!(
            buffer.insert_checksum(0, StateChecksum::new(7, 0xaaaa)),
            Err(LockstepSyncError::InvalidPlayerIdentity { player_id: 0 })
        );
        assert_eq!(
            buffer.insert_checksum_frame(StateChecksumFrame::new(0, Frame(7), 0xaaaa)),
            Err(LockstepSyncError::InvalidPlayerIdentity { player_id: 0 })
        );
    }

    #[test]
    fn lockstep_buffer_requires_exact_roster_cardinality_for_ready_frames() {
        let mut buffer = LockstepBuffer::new([1, 2]).expect("lockstep buffer");
        buffer
            .inputs
            .insert(8, BTreeMap::from([(1, 0x10), (2, 0x20), (3, 0x30)]));

        assert!(!buffer.is_frame_ready(8));
        assert_eq!(buffer.frame(8), None);
        assert_eq!(buffer.next_ready_frame(8), None);
    }

    #[test]
    fn lockstep_buffer_rejects_empty_rosters() {
        assert_eq!(
            LockstepBuffer::new(std::iter::empty::<PlayerId>()),
            Err(LockstepSyncError::EmptyRoster)
        );
        assert_eq!(
            LockstepBuffer::new([0, 1]),
            Err(LockstepSyncError::InvalidPlayerIdentity { player_id: 0 })
        );
    }

    #[test]
    fn latest_remote_presence_filters_local_stale_and_keeps_newest_per_user() {
        let entries = vec![
            OverworldPresence::new(
                "local",
                "Local",
                PresenceEntityType::Player,
                "NEW_BARK_TOWN",
                TilePosition::new(1, 1),
                Direction::Down,
                100,
            )
            .expect("presence"),
            OverworldPresence::new(
                "remote",
                "Old",
                PresenceEntityType::Player,
                "ROUTE_29",
                TilePosition::new(2, 2),
                Direction::Left,
                50,
            )
            .expect("presence"),
            OverworldPresence::new(
                "remote",
                "New",
                PresenceEntityType::Player,
                "ROUTE_29",
                TilePosition::new(3, 4),
                Direction::Right,
                150,
            )
            .expect("presence"),
            OverworldPresence::new(
                "stale",
                "Stale",
                PresenceEntityType::Ai,
                "ROUTE_30",
                TilePosition::new(5, 6),
                Direction::Up,
                1,
            )
            .expect("presence"),
        ];

        let remote = latest_remote_presence(&entries, "local", 200, 100);
        assert_eq!(remote.len(), 1);
        assert_eq!(remote[0].user_id(), "remote");
        assert_eq!(remote[0].player_name(), "New");
        assert_eq!(remote[0].tile(), TilePosition::new(3, 4));
    }

    #[test]
    fn presence_and_interaction_messages_are_transport_neutral_json() {
        let message = LinkMessage::Presence(
            OverworldPresence::new(
                "u1",
                "CHRIS",
                PresenceEntityType::Player,
                "ROUTE_29",
                TilePosition::new(10, 12),
                Direction::Up,
                1234,
            )
            .expect("presence"),
        );
        let json = serde_json::to_string(&message).expect("serialize presence");
        assert!(json.contains(r#""type":"presence""#));
        assert_eq!(
            serde_json::from_str::<LinkMessage>(&json).expect("deserialize presence"),
            message
        );
    }

    #[test]
    fn session_presence_and_interactions_carry_exact_pack_bound_session_identity() {
        let session = test_session("session-1", test_modpack("core-modular", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd"))
            .expect("session identity");
        let presence = OverworldPresence::new(
            "u1",
            "CHRIS",
            PresenceEntityType::Player,
            "ROUTE_29",
            TilePosition::new(10, 12),
            Direction::Up,
            1234,
        )
        .expect("presence");
        let presence_message = LinkMessage::SessionPresence(
            SessionOverworldPresence::new(session.clone(), presence.clone())
                .expect("bound presence"),
        );
        let presence_json =
            serde_json::to_string(&presence_message).expect("serialize bound presence");

        assert!(
            presence_json.contains(r#""type":"session_presence""#),
            "{presence_json}"
        );
        assert!(
            presence_json.contains(r#""session_id":"session-1""#),
            "{presence_json}"
        );
        assert!(
            presence_json.contains(r#""id":"core-modular""#),
            "{presence_json}"
        );
        assert!(
            presence_json.contains(r#""hash":"1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd""#),
            "{presence_json}"
        );
        assert!(
            presence_json.contains(&format!(r#""pack_content_hash":"{}""#, pack_content_hash())),
            "{presence_json}"
        );
        assert!(presence_json.contains(r#""map_name":"ROUTE_29""#));
        assert_eq!(
            serde_json::from_str::<LinkMessage>(&presence_json)
                .expect("deserialize bound presence"),
            presence_message
        );

        let request = MultiplayerInteractionRequest::new(
            "request-1",
            "u1",
            "CHRIS",
            "u2",
            MultiplayerInteractionKind::Battle,
            1235,
        )
        .expect("request");
        let request_message = LinkMessage::SessionInteractionRequest(
            SessionMultiplayerInteractionRequest::new(session.clone(), request.clone())
                .expect("bound request"),
        );
        let request_json =
            serde_json::to_string(&request_message).expect("serialize bound request");

        assert!(
            request_json.contains(r#""type":"session_interaction_request""#),
            "{request_json}"
        );
        assert!(
            request_json.contains(r#""session_id":"session-1""#),
            "{request_json}"
        );
        assert!(request_json.contains(r#""request_id":"request-1""#));
        assert_eq!(
            serde_json::from_str::<LinkMessage>(&request_json).expect("deserialize bound request"),
            request_message
        );

        let response = MultiplayerInteractionResponse::new(
            "request-1",
            "u2",
            "u1",
            MultiplayerInteractionKind::Battle,
            true,
            1236,
        )
        .expect("response");
        let response_message = LinkMessage::SessionInteractionResponse(
            SessionMultiplayerInteractionResponse::new(session.clone(), response.clone())
                .expect("bound response"),
        );
        let response_json =
            serde_json::to_string(&response_message).expect("serialize bound response");

        assert!(
            response_json.contains(r#""type":"session_interaction_response""#),
            "{response_json}"
        );
        assert!(
            response_json.contains(r#""session_id":"session-1""#),
            "{response_json}"
        );
        assert!(response_json.contains(r#""accepted":true"#));
        assert_eq!(
            serde_json::from_str::<LinkMessage>(&response_json)
                .expect("deserialize bound response"),
            response_message
        );
    }

    #[test]
    fn session_presence_and_interactions_reject_invalid_session_identity() {
        let invalid_session = LinkSessionIdentity::new_unchecked_for_tests(
            LINK_PROTOCOL_VERSION,
            " session-1",
            test_modpack("core-modular", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd"),
            pack_content_hash(),
        );
        let presence = OverworldPresence::new(
            "u1",
            "CHRIS",
            PresenceEntityType::Player,
            "ROUTE_29",
            TilePosition::new(10, 12),
            Direction::Up,
            1234,
        )
        .expect("presence");
        let request = MultiplayerInteractionRequest::new(
            "request-1",
            "u1",
            "CHRIS",
            "u2",
            MultiplayerInteractionKind::Trade,
            1235,
        )
        .expect("request");
        let response = MultiplayerInteractionResponse::new(
            "request-1",
            "u2",
            "u1",
            MultiplayerInteractionKind::Trade,
            false,
            1236,
        )
        .expect("response");

        assert!(matches!(
            LinkMessage::SessionPresence(SessionOverworldPresence::new_unchecked_for_tests(
                invalid_session.clone(),
                presence
            ))
            .validate(),
            Err(MultiplayerMessageError::InvalidLinkHandshake { .. })
        ));
        assert!(matches!(
            LinkMessage::SessionInteractionRequest(
                SessionMultiplayerInteractionRequest::new_unchecked_for_tests(
                    invalid_session.clone(),
                    request,
                )
            )
            .validate(),
            Err(MultiplayerMessageError::InvalidLinkHandshake { .. })
        ));
        assert!(matches!(
            LinkMessage::SessionInteractionResponse(
                SessionMultiplayerInteractionResponse::new_unchecked_for_tests(
                    invalid_session,
                    response,
                )
            )
            .validate(),
            Err(MultiplayerMessageError::InvalidLinkHandshake { .. })
        ));
    }

    #[test]
    fn presence_and_interaction_validate_exact_identity_fields() {
        let presence = OverworldPresence::new_unchecked_for_tests(
            " u1",
            "CHRIS",
            PresenceEntityType::Player,
            "ROUTE_29",
            TilePosition::new(10, 12),
            Direction::Up,
            1234,
        );
        assert_eq!(
            presence.validate(),
            Err(MultiplayerMessageError::InvalidText {
                field: "presence user id",
            })
        );
        let malformed_map = OverworldPresence::new_unchecked_for_tests(
            "u1",
            "CHRIS",
            PresenceEntityType::Player,
            "ROUTE 29",
            TilePosition::new(10, 12),
            Direction::Up,
            1234,
        );
        assert_eq!(
            malformed_map.validate(),
            Err(MultiplayerMessageError::InvalidText {
                field: "presence map name",
            })
        );
        let malformed_name = OverworldPresence::new_unchecked_for_tests(
            "u1",
            "CHRIS\nRED",
            PresenceEntityType::Player,
            "ROUTE_29",
            TilePosition::new(10, 12),
            Direction::Up,
            1234,
        );
        assert_eq!(
            malformed_name.validate(),
            Err(MultiplayerMessageError::InvalidText {
                field: "presence player name",
            })
        );
        let negative_tile = OverworldPresence::new_unchecked_for_tests(
            "u1",
            "CHRIS",
            PresenceEntityType::Player,
            "ROUTE_29",
            TilePosition::new(-1, 12),
            Direction::Up,
            1234,
        );
        assert_eq!(
            negative_tile.validate(),
            Err(MultiplayerMessageError::InvalidTile {
                field: "presence",
                x: -1,
                y: 12,
            })
        );
        let namespaced_user = OverworldPresence::new_unchecked_for_tests(
            "link:u1",
            "CHRIS",
            PresenceEntityType::Player,
            "ROUTE_29",
            TilePosition::new(10, 12),
            Direction::Up,
            1234,
        );
        assert_eq!(
            namespaced_user.validate(),
            Err(MultiplayerMessageError::InvalidText {
                field: "presence user id",
            })
        );
        let namespaced_user_decode = serde_json::from_value::<LinkMessage>(serde_json::json!({
            "type": "presence",
            "user_id": "link:u1",
            "player_name": "CHRIS",
            "entity_type": "player",
            "map_name": "ROUTE_29",
            "tile": { "x": 10, "y": 12 },
            "direction": "up",
            "updated_at_ms": 1234
        }))
        .expect_err("presence JSON must validate user id during decode")
        .to_string();
        assert!(
            namespaced_user_decode.contains("presence user id"),
            "{namespaced_user_decode}"
        );
        let reserved_user = OverworldPresence::new_unchecked_for_tests(
            "fallback-user",
            "CHRIS",
            PresenceEntityType::Player,
            "ROUTE_29",
            TilePosition::new(10, 12),
            Direction::Up,
            1234,
        );
        assert_eq!(
            reserved_user.validate(),
            Err(MultiplayerMessageError::InvalidText {
                field: "presence user id",
            })
        );

        let request = MultiplayerInteractionRequest::new_unchecked_for_tests(
            "",
            "u1",
            "CHRIS",
            "u2",
            MultiplayerInteractionKind::Trade,
            1234,
        );
        assert_eq!(
            request.validate(),
            Err(MultiplayerMessageError::EmptyText {
                field: "interaction request id",
            })
        );
        let malformed_request = MultiplayerInteractionRequest::new_unchecked_for_tests(
            "request 1",
            "u1",
            "CHRIS",
            "u2",
            MultiplayerInteractionKind::Trade,
            1234,
        );
        assert_eq!(
            malformed_request.validate(),
            Err(MultiplayerMessageError::InvalidText {
                field: "interaction request id",
            })
        );
        let namespaced_target = MultiplayerInteractionRequest::new_unchecked_for_tests(
            "request-1",
            "u1",
            "CHRIS",
            "link:u2",
            MultiplayerInteractionKind::Trade,
            1234,
        );
        assert_eq!(
            namespaced_target.validate(),
            Err(MultiplayerMessageError::InvalidText {
                field: "interaction request target user id",
            })
        );
        let namespaced_target_decode = serde_json::from_value::<LinkMessage>(serde_json::json!({
            "type": "interaction_request",
            "request_id": "request-1",
            "from_user_id": "u1",
            "from_player_name": "CHRIS",
            "to_user_id": "link:u2",
            "kind": "trade",
            "timestamp_ms": 1234
        }))
        .expect_err("interaction request JSON must validate target user id during decode")
        .to_string();
        assert!(
            namespaced_target_decode.contains("interaction request target user id"),
            "{namespaced_target_decode}"
        );
        let self_request = MultiplayerInteractionRequest::new_unchecked_for_tests(
            "request-1",
            "u1",
            "CHRIS",
            "u1",
            MultiplayerInteractionKind::Trade,
            1234,
        );
        assert_eq!(
            self_request.validate(),
            Err(MultiplayerMessageError::SameInteractionUser {
                field: "interaction request",
            })
        );

        let response = MultiplayerInteractionResponse::new_unchecked_for_tests(
            "request-1",
            "u2",
            " u1",
            MultiplayerInteractionKind::Trade,
            true,
            1235,
        );
        assert_eq!(
            response.validate(),
            Err(MultiplayerMessageError::InvalidText {
                field: "interaction response target user id",
            })
        );
        let malformed_response_decode = serde_json::from_value::<LinkMessage>(serde_json::json!({
            "type": "interaction_response",
            "request_id": "request-1",
            "from_user_id": "u2",
            "to_user_id": " u1",
            "kind": "trade",
            "accepted": true,
            "timestamp_ms": 1235
        }))
        .expect_err("interaction response JSON must validate target user id during decode")
        .to_string();
        assert!(
            malformed_response_decode.contains("interaction response target user id"),
            "{malformed_response_decode}"
        );
        let self_response = MultiplayerInteractionResponse::new_unchecked_for_tests(
            "request-1",
            "u2",
            "u2",
            MultiplayerInteractionKind::Trade,
            true,
            1235,
        );
        assert_eq!(
            self_response.validate(),
            Err(MultiplayerMessageError::SameInteractionUser {
                field: "interaction response",
            })
        );
        assert!(
            MultiplayerInteractionResponse::new(
                "battle:request-1",
                "user-2",
                "user_1",
                MultiplayerInteractionKind::Battle,
                false,
                1236,
            )
            .is_ok()
        );
    }

    #[test]
    fn link_message_validate_owns_protocol_payload_rules() {
        assert_eq!(
            LinkMessage::Disconnect {
                player_id: 0,
                reason: "done".to_string(),
            }
            .validate(),
            Err(MultiplayerMessageError::InvalidPlayerIdentity { player_id: 0 })
        );

        assert_eq!(
            LinkMessage::Disconnect {
                player_id: 1,
                reason: " done".to_string(),
            }
            .validate(),
            Err(MultiplayerMessageError::InvalidText {
                field: "disconnect reason",
            })
        );

        assert_eq!(
            LinkMessage::Disconnect {
                player_id: 1,
                reason: "done\0now".to_string(),
            }
            .validate(),
            Err(MultiplayerMessageError::InvalidText {
                field: "disconnect reason",
            })
        );

        assert_eq!(
            LinkMessage::BattleAction(BattleActionFrame::new_unchecked_for_tests(
                1,
                7,
                BattleAction::Run,
                String::new(),
            ))
            .validate(),
            Err(MultiplayerMessageError::InvalidBattleAction {
                message: "battle sync state hash must be non-empty".to_string(),
            })
        );

        assert_eq!(
            LinkMessage::StateHash(StateChecksumFrame::new(0, Frame(7), 0x1111_1111)).validate(),
            Err(MultiplayerMessageError::InvalidLockstepFrame {
                message: "lockstep player id 0 is not a valid link identity".to_string(),
            })
        );
    }

    #[test]
    fn session_disconnect_message_carries_exact_pack_bound_session_identity() {
        let session = test_session("session-1", test_modpack("core-modular", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd"))
            .expect("session identity");
        let frame =
            SessionDisconnectFrame::new(session.clone(), 2, "closed").expect("disconnect frame");
        let message = LinkMessage::SessionDisconnect(frame.clone());
        let json = serde_json::to_string(&message).expect("serialize session disconnect");

        assert!(json.contains(r#""type":"session_disconnect""#), "{json}");
        assert!(json.contains(r#""session_id":"session-1""#), "{json}");
        assert!(json.contains(r#""id":"core-modular""#), "{json}");
        assert!(json.contains(r#""hash":"1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd""#), "{json}");
        assert!(
            json.contains(&format!(r#""pack_content_hash":"{}""#, pack_content_hash())),
            "{json}"
        );
        assert!(json.contains(r#""player_id":2"#), "{json}");
        assert!(json.contains(r#""reason":"closed""#), "{json}");
        assert_eq!(
            serde_json::from_str::<LinkMessage>(&json).expect("deserialize session disconnect"),
            message
        );
        assert_eq!(frame.session(), &session);
        assert_eq!(frame.player_id(), 2);
        assert_eq!(frame.reason(), "closed");
    }

    #[test]
    fn session_disconnect_rejects_invalid_session_and_payload() {
        let invalid_session = LinkSessionIdentity::new_unchecked_for_tests(
            LINK_PROTOCOL_VERSION,
            " session-1",
            test_modpack("core-modular", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd"),
            pack_content_hash(),
        );
        assert!(matches!(
            LinkMessage::SessionDisconnect(SessionDisconnectFrame::new_unchecked_for_tests(
                invalid_session,
                2,
                "closed",
            ))
            .validate(),
            Err(MultiplayerMessageError::InvalidLinkHandshake { .. })
        ));

        let session = test_session("session-1", test_modpack("core-modular", "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd"))
            .expect("session identity");
        assert_eq!(
            SessionDisconnectFrame::new(session.clone(), 0, "closed"),
            Err("multiplayer player id 0 is not a valid link identity".to_string())
        );
        assert_eq!(
            SessionDisconnectFrame::new(session, 2, " closed"),
            Err("disconnect reason must be exact and untrimmed".to_string())
        );
    }
}
