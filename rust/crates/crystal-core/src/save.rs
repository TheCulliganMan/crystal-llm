use std::{
    collections::BTreeSet,
    path::{Component, Path},
};

use serde::{Deserialize, Deserializer, Serialize};
use thiserror::Error;

use crate::multiplayer::{fnv1a32_bytes, fnv1a32_hex_bytes};
use crate::state::GameState;

const SAVE_MAGIC: &[u8; 12] = b"CRYSTALSAVE\0";
pub const SAVE_EXTENSION: &str = "crystalsave";
pub const SAVE_FORMAT_VERSION: u16 = 2;
const SAVE_VERSION_OFFSET: usize = SAVE_MAGIC.len();
const SAVE_PAYLOAD_LENGTH_OFFSET: usize = SAVE_VERSION_OFFSET + 2;
const SAVE_PAYLOAD_HASH_OFFSET: usize = SAVE_PAYLOAD_LENGTH_OFFSET + 4;
const SAVE_HEADER_LEN: usize = SAVE_PAYLOAD_HASH_OFFSET + 4;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SaveModpackIdentity {
    id: String,
    hash: String,
}

impl<'de> Deserialize<'de> for SaveModpackIdentity {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawSaveModpackIdentity {
            id: String,
            hash: String,
        }

        let raw = RawSaveModpackIdentity::deserialize(deserializer)?;
        SaveModpackIdentity::new(raw.id, raw.hash).map_err(serde::de::Error::custom)
    }
}

impl SaveModpackIdentity {
    pub fn new(id: impl Into<String>, hash: impl Into<String>) -> Result<Self, SaveError> {
        let identity = Self {
            id: id.into(),
            hash: hash.into(),
        };
        identity.validate()?;
        Ok(identity)
    }

    pub fn from_compiled_pack_bytes(
        id: impl Into<String>,
        compiled_pack_bytes: &[u8],
    ) -> Result<Self, SaveError> {
        if compiled_pack_bytes.is_empty() {
            return Err(SaveError::InvalidIdentity(
                "save modpack identity requires non-empty compiled pack bytes".to_string(),
            ));
        }
        let hash = fnv1a32_hex_bytes(compiled_pack_bytes);
        Self::new(id, hash)
    }

    pub fn validate(&self) -> Result<(), SaveError> {
        Self::validate_id(&self.id)?;
        if self.hash.len() != 8
            || !self
                .hash
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(SaveError::InvalidIdentity(format!(
                "save modpack hash '{}' must be an exact 8-character lowercase FNV hex hash",
                self.hash
            )));
        }
        Ok(())
    }

    pub fn validate_id(id: &str) -> Result<(), SaveError> {
        if id.is_empty() {
            return Err(SaveError::InvalidIdentity(
                "save modpack id is required".to_string(),
            ));
        }
        if id.trim() != id {
            return Err(SaveError::InvalidIdentity(
                "save modpack id must be exact and untrimmed".to_string(),
            ));
        }
        if !is_exact_modpack_id(id) {
            return Err(SaveError::InvalidIdentity(
                "save modpack id must be exact '+'-separated manifest ids using only ASCII letters, numbers, underscores, hyphens, or dots"
                    .to_string(),
            ));
        }
        for segment in id.split('+') {
            let lowered = segment.to_ascii_lowercase();
            if lowered.starts_with("fallback") || lowered.starts_with("legacy") {
                return Err(SaveError::InvalidIdentity(format!(
                    "save modpack id segment '{segment}' uses reserved runtime pack prefix"
                )));
            }
        }
        Ok(())
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn hash(&self) -> &str {
        &self.hash
    }
}

fn is_exact_modpack_id(value: &str) -> bool {
    if value.is_empty() || value.trim() != value {
        return false;
    }
    let mut seen = BTreeSet::new();
    for segment in value.split('+') {
        if segment.is_empty()
            || !segment
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
            || !seen.insert(segment)
        {
            return false;
        }
    }
    true
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SaveMetadata {
    modpack: SaveModpackIdentity,
    pack_content_hash: String,
    created_frame: u64,
    saved_frame: u64,
}

impl<'de> Deserialize<'de> for SaveMetadata {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawSaveMetadata {
            modpack: SaveModpackIdentity,
            pack_content_hash: String,
            created_frame: u64,
            saved_frame: u64,
        }

        let raw = RawSaveMetadata::deserialize(deserializer)?;
        let metadata = Self {
            modpack: raw.modpack,
            pack_content_hash: raw.pack_content_hash,
            created_frame: raw.created_frame,
            saved_frame: raw.saved_frame,
        };
        metadata.validate().map_err(serde::de::Error::custom)?;
        Ok(metadata)
    }
}

impl SaveMetadata {
    fn new(modpack: SaveModpackIdentity, pack_content_hash: String, state: &GameState) -> Self {
        Self {
            modpack,
            pack_content_hash,
            created_frame: state.frame_counter,
            saved_frame: state.frame_counter,
        }
    }

    pub fn validate(&self) -> Result<(), SaveError> {
        self.modpack.validate()?;
        validate_pack_content_hash(&self.pack_content_hash)?;
        if self.created_frame > self.saved_frame {
            return Err(SaveError::InvalidIdentity(format!(
                "save created_frame {} cannot exceed saved_frame {}",
                self.created_frame, self.saved_frame
            )));
        }
        Ok(())
    }

    pub fn modpack(&self) -> &SaveModpackIdentity {
        &self.modpack
    }

    pub fn pack_content_hash(&self) -> &str {
        &self.pack_content_hash
    }

    pub fn created_frame(&self) -> u64 {
        self.created_frame
    }

    pub fn saved_frame(&self) -> u64 {
        self.saved_frame
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SaveGame {
    format_version: u16,
    metadata: SaveMetadata,
    state: GameState,
}

impl SaveGame {
    fn new(state: GameState, modpack: SaveModpackIdentity, pack_content_hash: String) -> Self {
        Self {
            format_version: SAVE_FORMAT_VERSION,
            metadata: SaveMetadata::new(modpack, pack_content_hash, &state),
            state,
        }
    }

    pub fn validate(&self) -> Result<(), SaveError> {
        if self.format_version != SAVE_FORMAT_VERSION {
            return Err(SaveError::UnsupportedVersion(self.format_version));
        }
        self.metadata.validate()?;
        if self.metadata.saved_frame != self.state.frame_counter {
            return Err(SaveError::FrameMismatch {
                metadata_frame: self.metadata.saved_frame,
                state_frame: self.state.frame_counter,
            });
        }
        self.state
            .validate_saved_state()
            .map_err(SaveError::InvalidState)?;
        Ok(())
    }

    pub fn format_version(&self) -> u16 {
        self.format_version
    }

    pub fn metadata(&self) -> &SaveMetadata {
        &self.metadata
    }

    pub fn state(&self) -> &GameState {
        &self.state
    }

    pub fn into_state(self) -> GameState {
        self.state
    }
}

#[derive(Debug, Error)]
pub enum SaveError {
    #[error("save path {path} must use .{expected}")]
    InvalidExtension {
        path: String,
        expected: &'static str,
    },
    #[error("save path {path} must not include {component} components")]
    InvalidPathComponent {
        path: String,
        component: &'static str,
    },
    #[error("failed to read save {path}: {source}")]
    Read {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to write save {path}: {source}")]
    Write {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to create save directory {path}: {source}")]
    CreateDirectory {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("save {0} is not a Crystal Rust save")]
    InvalidMagic(String),
    #[error("save frame is shorter than the required header")]
    FrameTooShort,
    #[error("save uses unsupported format version {0}")]
    UnsupportedVersion(u16),
    #[error("save payload length {declared} does not match actual {actual}")]
    PayloadLengthMismatch { declared: usize, actual: usize },
    #[error("save payload hash {actual:#010x} does not match declared {expected:#010x}")]
    PayloadHashMismatch { expected: u32, actual: u32 },
    #[error("save payload must be non-empty")]
    EmptyPayload,
    #[error("save has {0} trailing bytes")]
    TrailingBytes(usize),
    #[error("failed to encode save: {0}")]
    Encode(String),
    #[error("failed to decode save: {0}")]
    Decode(String),
    #[error("{0}")]
    InvalidIdentity(String),
    #[error("invalid save state: {0}")]
    InvalidState(String),
    #[error("save metadata frame {metadata_frame} does not match state frame {state_frame}")]
    FrameMismatch {
        metadata_frame: u64,
        state_frame: u64,
    },
    #[error("save modpack hash {actual} does not match expected {expected}")]
    ModpackHashMismatch { expected: String, actual: String },
    #[error("save modpack id {actual} does not match expected {expected}")]
    ModpackIdMismatch { expected: String, actual: String },
    #[error("save pack content hash {actual} does not match expected {expected}")]
    PackContentHashMismatch { expected: String, actual: String },
}

fn write_save_game(path: impl AsRef<Path>, save: &SaveGame) -> Result<(), SaveError> {
    let path = path.as_ref();
    validate_save_path(path)?;
    save.validate()?;
    let bytes = encode_save_game_bytes(save)?;
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent).map_err(|source| SaveError::CreateDirectory {
            path: parent.display().to_string(),
            source,
        })?;
    }
    std::fs::write(path, bytes).map_err(|source| SaveError::Write {
        path: path.display().to_string(),
        source,
    })
}

fn encode_save_game_bytes(save: &SaveGame) -> Result<Vec<u8>, SaveError> {
    let encoded = bincode::serde::encode_to_vec(save, save_binary_config())
        .map_err(|error| SaveError::Encode(error.to_string()))?;
    if encoded.len() > u32::MAX as usize {
        return Err(SaveError::Encode(
            "encoded save exceeds binary payload length field".to_string(),
        ));
    }
    let mut bytes = Vec::with_capacity(SAVE_HEADER_LEN + encoded.len());
    bytes.extend_from_slice(SAVE_MAGIC);
    bytes.extend_from_slice(&SAVE_FORMAT_VERSION.to_be_bytes());
    bytes.extend_from_slice(&(encoded.len() as u32).to_be_bytes());
    bytes.extend_from_slice(&fnv1a32_bytes(&encoded).to_be_bytes());
    bytes.extend_from_slice(&encoded);
    Ok(bytes)
}

pub fn write_save_game_for_modpack(
    path: impl AsRef<Path>,
    state: GameState,
    modpack: &SaveModpackIdentity,
    pack_content_hash: &str,
) -> Result<(), SaveError> {
    modpack.validate()?;
    validate_pack_content_hash(pack_content_hash)?;
    let save = SaveGame::new(state, modpack.clone(), pack_content_hash.to_string());
    write_save_game(path, &save)
}

fn read_save_game(path: impl AsRef<Path>) -> Result<SaveGame, SaveError> {
    let path = path.as_ref();
    validate_save_path(path)?;
    let bytes = std::fs::read(path).map_err(|source| SaveError::Read {
        path: path.display().to_string(),
        source,
    })?;
    read_save_game_bytes(&bytes, path.display().to_string())
}

pub fn read_save_game_for_modpack(
    path: impl AsRef<Path>,
    expected: &SaveModpackIdentity,
    expected_pack_content_hash: &str,
) -> Result<SaveGame, SaveError> {
    let save = read_save_game(path)?;
    assert_save_matches_modpack(&save, expected, expected_pack_content_hash)?;
    Ok(save)
}

fn read_save_game_bytes(
    bytes: &[u8],
    source_name: impl Into<String>,
) -> Result<SaveGame, SaveError> {
    let source_name = source_name.into();
    if !bytes.starts_with(SAVE_MAGIC) {
        return Err(SaveError::InvalidMagic(source_name.clone()));
    }
    if bytes.len() < SAVE_HEADER_LEN {
        return Err(SaveError::FrameTooShort);
    }
    let version = u16::from_be_bytes([bytes[SAVE_VERSION_OFFSET], bytes[SAVE_VERSION_OFFSET + 1]]);
    if version != SAVE_FORMAT_VERSION {
        return Err(SaveError::UnsupportedVersion(version));
    }
    let declared = u32::from_be_bytes([
        bytes[SAVE_PAYLOAD_LENGTH_OFFSET],
        bytes[SAVE_PAYLOAD_LENGTH_OFFSET + 1],
        bytes[SAVE_PAYLOAD_LENGTH_OFFSET + 2],
        bytes[SAVE_PAYLOAD_LENGTH_OFFSET + 3],
    ]) as usize;
    let actual = bytes.len() - SAVE_HEADER_LEN;
    if declared != actual {
        return Err(SaveError::PayloadLengthMismatch { declared, actual });
    }
    if declared == 0 {
        return Err(SaveError::EmptyPayload);
    }
    let expected_hash = u32::from_be_bytes([
        bytes[SAVE_PAYLOAD_HASH_OFFSET],
        bytes[SAVE_PAYLOAD_HASH_OFFSET + 1],
        bytes[SAVE_PAYLOAD_HASH_OFFSET + 2],
        bytes[SAVE_PAYLOAD_HASH_OFFSET + 3],
    ]);
    let payload = &bytes[SAVE_HEADER_LEN..];
    let actual_hash = fnv1a32_bytes(payload);
    if actual_hash != expected_hash {
        return Err(SaveError::PayloadHashMismatch {
            expected: expected_hash,
            actual: actual_hash,
        });
    }
    let (save, consumed): (SaveGame, usize) =
        bincode::serde::decode_from_slice(payload, save_binary_config())
            .map_err(|error| SaveError::Decode(error.to_string()))?;
    if consumed != payload.len() {
        return Err(SaveError::TrailingBytes(payload.len() - consumed));
    }
    save.validate()?;
    Ok(save)
}

pub fn read_save_game_bytes_for_modpack(
    bytes: &[u8],
    source_name: impl Into<String>,
    expected: &SaveModpackIdentity,
    expected_pack_content_hash: &str,
) -> Result<SaveGame, SaveError> {
    let source_name = source_name.into();
    validate_save_path(Path::new(&source_name))?;
    let save = read_save_game_bytes(bytes, source_name)?;
    assert_save_matches_modpack(&save, expected, expected_pack_content_hash)?;
    Ok(save)
}

pub fn assert_save_matches_modpack(
    save: &SaveGame,
    expected: &SaveModpackIdentity,
    expected_pack_content_hash: &str,
) -> Result<(), SaveError> {
    expected.validate()?;
    validate_pack_content_hash(expected_pack_content_hash)?;
    save.validate()?;
    if save.metadata.modpack.id != expected.id {
        return Err(SaveError::ModpackIdMismatch {
            expected: expected.id.clone(),
            actual: save.metadata.modpack.id.clone(),
        });
    }
    if save.metadata.modpack.hash != expected.hash {
        return Err(SaveError::ModpackHashMismatch {
            expected: expected.hash.clone(),
            actual: save.metadata.modpack.hash.clone(),
        });
    }
    if save.metadata.pack_content_hash != expected_pack_content_hash {
        return Err(SaveError::PackContentHashMismatch {
            expected: expected_pack_content_hash.to_string(),
            actual: save.metadata.pack_content_hash.clone(),
        });
    }
    Ok(())
}

pub fn validate_pack_content_hash(hash: &str) -> Result<(), SaveError> {
    if hash.len() != 8
        || !hash
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(SaveError::InvalidIdentity(format!(
            "save pack content hash '{hash}' must be an exact 8-character lowercase FNV hex hash"
        )));
    }
    Ok(())
}

fn validate_save_path(path: &Path) -> Result<(), SaveError> {
    if path
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(SaveError::InvalidPathComponent {
            path: path.display().to_string(),
            component: "parent-directory",
        });
    }
    if path
        .components()
        .any(|component| matches!(component, Component::CurDir))
    {
        return Err(SaveError::InvalidPathComponent {
            path: path.display().to_string(),
            component: "current-directory",
        });
    }
    match path.extension().and_then(|extension| extension.to_str()) {
        Some(extension) if extension == SAVE_EXTENSION => Ok(()),
        _ => Err(SaveError::InvalidExtension {
            path: path.display().to_string(),
            expected: SAVE_EXTENSION,
        }),
    }
}

fn save_binary_config() -> impl bincode::config::Config {
    bincode::config::standard()
        .with_little_endian()
        .with_fixed_int_encoding()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pack_content_hash() -> &'static str {
        "01020304"
    }

    fn test_save(state: GameState, modpack: SaveModpackIdentity) -> SaveGame {
        SaveGame::new(state, modpack, pack_content_hash().to_string())
    }

    fn temp_save_path(name: &str) -> std::path::PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "crystal-core-save-{}-{unique}-{name}",
            std::process::id()
        ))
    }

    #[test]
    fn save_game_round_trips_as_binary_artifact() {
        let path = temp_save_path("slot.crystalsave");
        let mut state = GameState::default();
        state.frame_counter = 42;
        state.pokedex.seen_species.insert("CHIKORITA".to_string());
        let modpack = SaveModpackIdentity::new("core-modular", "1234abcd").expect("identity");
        let save = test_save(state.clone(), modpack.clone());

        write_save_game(&path, &save).expect("write save");
        let loaded = read_save_game(&path).expect("read save");
        let loaded_for_pack = read_save_game_for_modpack(&path, &modpack, pack_content_hash())
            .expect("read save for exact pack");

        assert_eq!(loaded.state, state);
        assert_eq!(loaded_for_pack, loaded);
        assert_eq!(loaded.metadata.modpack, modpack);
        assert_eq!(loaded.metadata.saved_frame, 42);
        let bytes = std::fs::read(&path).expect("read raw save");
        assert!(bytes.starts_with(SAVE_MAGIC));
        assert_eq!(
            u16::from_be_bytes([bytes[SAVE_VERSION_OFFSET], bytes[SAVE_VERSION_OFFSET + 1]]),
            SAVE_FORMAT_VERSION
        );
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn save_path_rejects_json_extension() {
        let path = temp_save_path("slot.json");
        let save = test_save(
            GameState::default(),
            SaveModpackIdentity::new("core-modular", "1234abcd").expect("identity"),
        );

        let error = write_save_game(&path, &save).expect_err("json saves are not runtime saves");

        assert!(matches!(error, SaveError::InvalidExtension { .. }));
    }

    #[test]
    fn save_path_requires_explicit_crystalsave_extension() {
        let path = temp_save_path("slot");
        let save = test_save(
            GameState::default(),
            SaveModpackIdentity::new("core-modular", "1234abcd").expect("identity"),
        );

        let error = write_save_game(&path, &save).expect_err("missing extension is invalid");

        assert!(matches!(error, SaveError::InvalidExtension { .. }));
    }

    #[test]
    fn save_paths_reject_current_and_parent_directory_aliases() {
        let save = test_save(
            GameState::default(),
            SaveModpackIdentity::new("core-modular", "1234abcd").expect("identity"),
        );
        let bytes = encode_save_game_bytes(&save).expect("encode save");

        let current_dir_error = write_save_game("saves/./slot.crystalsave", &save)
            .expect_err("save writes must reject current-directory aliases");
        assert!(matches!(
            current_dir_error,
            SaveError::InvalidPathComponent {
                component: "current-directory",
                ..
            }
        ));

        let parent_dir_error = read_save_game_bytes_for_modpack(
            &bytes,
            "saves/../slot.crystalsave",
            save.metadata().modpack(),
            save.metadata().pack_content_hash(),
        )
        .expect_err("save reads must reject parent-directory aliases");
        assert!(matches!(
            parent_dir_error,
            SaveError::InvalidPathComponent {
                component: "parent-directory",
                ..
            }
        ));
    }

    #[test]
    fn save_rejects_trailing_bytes_and_bad_magic() {
        let path = temp_save_path("slot.crystalsave");
        let save = test_save(
            GameState::default(),
            SaveModpackIdentity::new("core-modular", "1234abcd").expect("identity"),
        );
        write_save_game(&path, &save).expect("write save");
        let mut bytes = std::fs::read(&path).expect("read raw save");
        bytes.push(0xff);

        assert!(matches!(
            read_save_game_bytes(&bytes, "slot.crystalsave"),
            Err(SaveError::PayloadLengthMismatch { .. })
        ));
        assert!(matches!(
            read_save_game_bytes(SAVE_MAGIC, "slot.crystalsave"),
            Err(SaveError::FrameTooShort)
        ));
        assert!(matches!(
            read_save_game_bytes(b"{\"sram\":{}}", "legacy.json"),
            Err(SaveError::InvalidMagic(_))
        ));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn save_rejects_empty_payload_hash_mismatch_and_legacy_unframed_payloads() {
        let save = test_save(
            GameState::default(),
            SaveModpackIdentity::new("core-modular", "1234abcd").expect("identity"),
        );

        let mut empty = Vec::with_capacity(SAVE_HEADER_LEN);
        empty.extend_from_slice(SAVE_MAGIC);
        empty.extend_from_slice(&SAVE_FORMAT_VERSION.to_be_bytes());
        empty.extend_from_slice(&0_u32.to_be_bytes());
        empty.extend_from_slice(&fnv1a32_bytes(&[]).to_be_bytes());
        assert!(matches!(
            read_save_game_bytes(&empty, "slot.crystalsave"),
            Err(SaveError::EmptyPayload)
        ));

        let mut corrupt = encode_save_game_bytes(&save).expect("encode framed save");
        let expected = u32::from_be_bytes([
            corrupt[SAVE_PAYLOAD_HASH_OFFSET],
            corrupt[SAVE_PAYLOAD_HASH_OFFSET + 1],
            corrupt[SAVE_PAYLOAD_HASH_OFFSET + 2],
            corrupt[SAVE_PAYLOAD_HASH_OFFSET + 3],
        ]);
        let last = corrupt.last_mut().expect("payload byte");
        *last ^= 0x01;
        let actual = fnv1a32_bytes(&corrupt[SAVE_HEADER_LEN..]);
        assert!(matches!(
            read_save_game_bytes(&corrupt, "slot.crystalsave"),
            Err(SaveError::PayloadHashMismatch { expected: err_expected, actual: err_actual })
                if err_expected == expected && err_actual == actual
        ));

        let encoded =
            bincode::serde::encode_to_vec(&save, save_binary_config()).expect("encode legacy save");
        let mut legacy = SAVE_MAGIC.to_vec();
        legacy.extend_from_slice(&encoded);
        assert!(matches!(
            read_save_game_bytes(&legacy, "slot.crystalsave"),
            Err(SaveError::UnsupportedVersion(_))
        ));
    }

    #[test]
    fn save_validates_modpack_identity_and_frame() {
        let mut save = test_save(
            GameState::default(),
            SaveModpackIdentity::new("core-modular", "1234abcd").expect("identity"),
        );
        save.metadata.saved_frame = 9;

        assert!(matches!(
            save.validate(),
            Err(SaveError::FrameMismatch {
                metadata_frame: 9,
                state_frame: 0
            })
        ));
        assert!(SaveModpackIdentity::new("core-modular", "not-a-hash").is_err());
        let uppercase_hash = SaveModpackIdentity::new("core-modular", "ABCDEF12")
            .expect_err("hash is exact lowercase");
        assert!(
            uppercase_hash
                .to_string()
                .contains("exact 8-character lowercase FNV hex hash"),
            "{uppercase_hash}"
        );
        let padded_id =
            SaveModpackIdentity::new(" core-modular ", "1234abcd").expect_err("id is untrimmed");
        assert!(
            padded_id
                .to_string()
                .contains("id must be exact and untrimmed"),
            "{padded_id}"
        );
        let whitespace_id =
            SaveModpackIdentity::new(" ", "1234abcd").expect_err("id whitespace is untrimmed");
        assert!(
            whitespace_id
                .to_string()
                .contains("id must be exact and untrimmed"),
            "{whitespace_id}"
        );
        let spaced_id =
            SaveModpackIdentity::new("core modular", "1234abcd").expect_err("id is a token");
        assert!(
            spaced_id.to_string().contains("separated manifest ids"),
            "{spaced_id}"
        );
        let joined_identity = SaveModpackIdentity::new("core-modular+johto.plus", "1234abcd")
            .expect("joined manifest ids are the canonical runtime pack id");
        assert_eq!(joined_identity.id(), "core-modular+johto.plus");
        for malformed_id in [
            "+core-modular",
            "core-modular+",
            "core-modular++johto",
            "core-modular+core-modular",
            "core-modular+johto/plus",
            "core-modular+johto plus",
        ] {
            let error = SaveModpackIdentity::new(malformed_id, "1234abcd")
                .expect_err("malformed joined manifest ids are invalid");
            assert!(
                error.to_string().contains("separated manifest ids"),
                "{malformed_id}: {error}"
            );
        }
        for reserved_id in [
            "fallback-core",
            "legacy.core",
            "core-modular+fallback-johto",
            "core-modular+legacy_johto",
        ] {
            let error = SaveModpackIdentity::new(reserved_id, "1234abcd")
                .expect_err("reserved runtime pack identities are invalid");
            assert!(
                error
                    .to_string()
                    .contains("uses reserved runtime pack prefix"),
                "{reserved_id}: {error}"
            );
        }
    }

    #[test]
    fn save_json_payloads_reject_unknown_fields_without_identity_fallbacks() {
        let identity_error = serde_json::from_value::<SaveModpackIdentity>(serde_json::json!({
            "id": "core-modular",
            "hash": "1234abcd",
            "normalized_id": "CORE-MODULAR"
        }))
        .expect_err("modpack identity must not accept alternate ids")
        .to_string();
        assert!(
            identity_error.contains("unknown field `normalized_id`"),
            "{identity_error}"
        );

        let hash_error = serde_json::from_value::<SaveModpackIdentity>(serde_json::json!({
            "id": "core-modular",
            "hash": "ABCDEF12"
        }))
        .expect_err("modpack identity hashes must validate during JSON load")
        .to_string();
        assert!(
            hash_error.contains("exact 8-character lowercase FNV hex hash"),
            "{hash_error}"
        );

        let reserved_error = serde_json::from_value::<SaveModpackIdentity>(serde_json::json!({
            "id": "core-modular+fallback-save",
            "hash": "1234abcd"
        }))
        .expect_err("modpack identity ids must validate during JSON load")
        .to_string();
        assert!(
            reserved_error.contains("uses reserved runtime pack prefix"),
            "{reserved_error}"
        );

        let metadata_frame_error = serde_json::from_value::<SaveMetadata>(serde_json::json!({
            "modpack": {
                "id": "core-modular",
                "hash": "1234abcd"
            },
            "pack_content_hash": "01020304",
            "created_frame": 9,
            "saved_frame": 8
        }))
        .expect_err("save metadata frame order must validate during JSON load")
        .to_string();
        assert!(
            metadata_frame_error.contains("created_frame 9 cannot exceed saved_frame 8"),
            "{metadata_frame_error}"
        );

        let metadata_identity_error = serde_json::from_value::<SaveMetadata>(serde_json::json!({
            "modpack": {
                "id": "legacy-pack",
                "hash": "1234abcd"
            },
            "pack_content_hash": "01020304",
            "created_frame": 8,
            "saved_frame": 8
        }))
        .expect_err("save metadata modpack must validate during JSON load")
        .to_string();
        assert!(
            metadata_identity_error.contains("uses reserved runtime pack prefix"),
            "{metadata_identity_error}"
        );

        let mut save_json = serde_json::to_value(test_save(
            GameState::default(),
            SaveModpackIdentity::new("core-modular", "1234abcd").expect("identity"),
        ))
        .expect("save json");
        save_json
            .as_object_mut()
            .expect("save object")
            .insert("legacy_sram".to_string(), serde_json::json!({}));

        let save_error = serde_json::from_value::<SaveGame>(save_json)
            .expect_err("save games must use the exact binary payload schema")
            .to_string();
        assert!(
            save_error.contains("unknown field `legacy_sram`"),
            "{save_error}"
        );
    }

    #[test]
    fn save_modpack_hash_must_match_runtime_pack() {
        let save = test_save(
            GameState::default(),
            SaveModpackIdentity::new("core-modular", "1234abcd").expect("identity"),
        );
        let expected = SaveModpackIdentity::new("core-modular", "ffffffff").expect("identity");

        let error = assert_save_matches_modpack(&save, &expected, pack_content_hash())
            .expect_err("mismatched modpack hashes must not load silently");

        assert!(matches!(error, SaveError::ModpackHashMismatch { .. }));

        let bytes = { encode_save_game_bytes(&save).expect("encode framed save") };
        assert!(matches!(
            read_save_game_bytes_for_modpack(&bytes, "slot.crystalsave", &expected, pack_content_hash()),
            Err(SaveError::ModpackHashMismatch { .. })
        ));
        assert!(matches!(
            read_save_game_bytes_for_modpack(&bytes, "slot.json", &expected, pack_content_hash()),
            Err(SaveError::InvalidExtension { .. })
        ));
    }

    #[test]
    fn save_modpack_match_requires_valid_save_payload() {
        let expected = SaveModpackIdentity::new("core-modular", "1234abcd").expect("identity");
        let mut save = test_save(GameState::default(), expected.clone());
        save.metadata.saved_frame = 9;

        assert!(matches!(
            assert_save_matches_modpack(&save, &expected, pack_content_hash()),
            Err(SaveError::FrameMismatch {
                metadata_frame: 9,
                state_frame: 0
            })
        ));
    }

    #[test]
    fn save_modpack_id_must_match_runtime_pack() {
        let save = test_save(
            GameState::default(),
            SaveModpackIdentity::new("core-modular", "1234abcd").expect("identity"),
        );
        let expected = SaveModpackIdentity::new("other-pack", "1234abcd").expect("identity");

        let error = assert_save_matches_modpack(&save, &expected, pack_content_hash())
            .expect_err("mismatched modpack ids must not load silently");

        assert!(matches!(error, SaveError::ModpackIdMismatch { .. }));
    }

    #[test]
    fn save_pack_content_hash_must_match_runtime_pack() {
        let expected = SaveModpackIdentity::new("core-modular", "1234abcd").expect("identity");
        let save = test_save(GameState::default(), expected.clone());

        let error = assert_save_matches_modpack(&save, &expected, "ffffffff")
            .expect_err("mismatched pack content hashes must not load silently");

        assert!(matches!(error, SaveError::PackContentHashMismatch { .. }));

        let bytes = encode_save_game_bytes(&save).expect("encode framed save");
        assert!(matches!(
            read_save_game_bytes_for_modpack(&bytes, "slot.crystalsave", &expected, "ffffffff"),
            Err(SaveError::PackContentHashMismatch { .. })
        ));
    }

    #[test]
    fn save_validation_rejects_invalid_saved_state_identifiers() {
        let expected = SaveModpackIdentity::new("core-modular", "1234abcd").expect("identity");
        let mut save_json =
            serde_json::to_value(test_save(GameState::default(), expected.clone()))
                .expect("save json");
        save_json["state"]["flags"]["event_flags"]["EVENT_BAD FLAG"] = serde_json::json!(true);
        let save: SaveGame = serde_json::from_value(save_json).expect("decode exact save shape");

        let error = save.validate().expect_err("saved flag ids must be exact");
        assert!(
            matches!(error, SaveError::InvalidState(message) if message.contains("EVENT_BAD FLAG"))
        );

        let bytes = { encode_save_game_bytes(&save).expect("encode framed save") };
        let error = read_save_game_bytes_for_modpack(&bytes, "slot.crystalsave", &expected, pack_content_hash())
            .expect_err("binary save load must validate decoded state");
        assert!(
            matches!(error, SaveError::InvalidState(message) if message.contains("EVENT_BAD FLAG"))
        );

        let mut scene_save_json =
            serde_json::to_value(test_save(GameState::default(), expected.clone()))
                .expect("save json");
        scene_save_json["state"]["scenes"]["map_scenes"]["ElmsLab"] =
            serde_json::json!("SCENE ELMSLAB NOOP");
        scene_save_json["state"]["scenes"]["map_scene_indices"]["ElmsLab"] = serde_json::json!(1);
        let scene_save: SaveGame =
            serde_json::from_value(scene_save_json).expect("decode exact save shape");
        let error = scene_save
            .validate()
            .expect_err("saved scene ids must be exact");
        assert!(
            matches!(error, SaveError::InvalidState(message) if message.contains("SCENE ELMSLAB NOOP"))
        );

        let mut runtime_save_json =
            serde_json::to_value(test_save(GameState::default(), expected.clone()))
                .expect("save json");
        runtime_save_json["state"]["script_runtime"]["next_script"] =
            serde_json::json!(" .Done@Script");
        let runtime_save: SaveGame =
            serde_json::from_value(runtime_save_json).expect("decode exact save shape");
        let error = runtime_save
            .validate()
            .expect_err("saved script labels must be exact");
        assert!(
            matches!(error, SaveError::InvalidState(message) if message.contains(" .Done@Script"))
        );

        let mut runtime_event_save_json =
            serde_json::to_value(test_save(GameState::default(), expected.clone()))
                .expect("save json");
        runtime_event_save_json["state"]["script_runtime"]["current_music"] =
            serde_json::json!("MUSIC ROUTE 29");
        let runtime_event_save: SaveGame =
            serde_json::from_value(runtime_event_save_json).expect("decode exact save shape");
        let error = runtime_event_save
            .validate()
            .expect_err("saved runtime event ids must be exact");
        assert!(
            matches!(error, SaveError::InvalidState(message) if message.contains("MUSIC ROUTE 29"))
        );

        let mut runtime_queue_save_json =
            serde_json::to_value(test_save(GameState::default(), expected.clone()))
                .expect("save json");
        runtime_queue_save_json["state"]["script_runtime"]["command_queue"] = serde_json::json!([{
            "command": "callasm",
            "target": "Queued Target",
            "bank": "BANK1",
            "source_script": "QueueScript",
            "command_index": 6
        }]);
        let runtime_queue_save: SaveGame =
            serde_json::from_value(runtime_queue_save_json).expect("decode exact save shape");
        let error = runtime_queue_save
            .validate()
            .expect_err("saved runtime queues must be exact");
        assert!(
            matches!(error, SaveError::InvalidState(message) if message.contains("Queued Target"))
        );

        let mut state_identity_save_json =
            serde_json::to_value(test_save(GameState::default(), expected.clone()))
                .expect("save json");
        state_identity_save_json["state"]["active_repel_item"] = serde_json::json!("SUPER REPEL");
        let state_identity_save: SaveGame =
            serde_json::from_value(state_identity_save_json).expect("decode exact save shape");
        let error = state_identity_save
            .validate()
            .expect_err("saved state identifiers must be exact");
        assert!(
            matches!(error, SaveError::InvalidState(message) if message.contains("SUPER REPEL"))
        );

        let mut overworld_save_json =
            serde_json::to_value(test_save(GameState::default(), expected.clone()))
                .expect("save json");
        overworld_save_json["state"]["overworld"] = serde_json::json!({
            "active": {
                "map_name": "Route 29",
                "tile": { "x": 1, "y": 2 },
                "facing": "down",
                "mode": "normal"
            }
        });
        let overworld_save: SaveGame =
            serde_json::from_value(overworld_save_json).expect("decode exact save shape");
        let error = overworld_save
            .validate()
            .expect_err("saved overworld identifiers must be exact");
        assert!(matches!(error, SaveError::InvalidState(message) if message.contains("Route 29")));

        let mut bag_save_json =
            serde_json::to_value(test_save(GameState::default(), expected.clone()))
                .expect("save json");
        bag_save_json["state"]["bag"]["items"]["POTION"] = serde_json::json!(100);
        let bag_save: SaveGame =
            serde_json::from_value(bag_save_json).expect("decode exact save shape");
        let error = bag_save
            .validate()
            .expect_err("saved bag metadata must be exact");
        assert!(
            matches!(error, SaveError::InvalidState(message) if message.contains("invalid saved bag"))
        );

        let mut pc_bag_save_json =
            serde_json::to_value(test_save(GameState::default(), expected.clone()))
                .expect("save json");
        pc_bag_save_json["state"]["bag"]["pc_items"]["POTION"] = serde_json::json!(100);
        let pc_bag_save: SaveGame =
            serde_json::from_value(pc_bag_save_json).expect("decode exact save shape");
        let error = pc_bag_save
            .validate()
            .expect_err("saved PC item metadata must be exact");
        assert!(matches!(error, SaveError::InvalidState(message) if message.contains("pc_items")));

        let mut storage_save_json =
            serde_json::to_value(test_save(GameState::default(), expected.clone()))
                .expect("save json");
        let mut pc_box_json =
            serde_json::to_value(crate::models::PcBox::new(0)).expect("pc box json");
        pc_box_json["count"] = serde_json::json!(1);
        storage_save_json["state"]["storage"]["pc_boxes"] = serde_json::json!([pc_box_json]);
        let storage_save: SaveGame =
            serde_json::from_value(storage_save_json).expect("decode exact save shape");
        let error = storage_save
            .validate()
            .expect_err("saved storage metadata must be exact");
        assert!(
            matches!(error, SaveError::InvalidState(message) if message.contains("invalid saved storage"))
        );

        let mut party_projection_save_json =
            serde_json::to_value(test_save(GameState::default(), expected.clone()))
                .expect("save json");
        party_projection_save_json["state"]["party"]["pokemon"][0] = serde_json::json!({
            "species": "CHIKORITA",
            "level": 6
        });
        let party_projection_save: SaveGame =
            serde_json::from_value(party_projection_save_json).expect("decode exact save shape");
        let error = party_projection_save
            .validate()
            .expect_err("saved party projection must match storage");
        assert!(
            matches!(error, SaveError::InvalidState(message) if message.contains("party projection"))
        );

        let mut battle_cursor_save_json =
            serde_json::to_value(test_save(GameState::default(), expected.clone()))
                .expect("save json");
        battle_cursor_save_json["state"]["battle_active_enemy_party_index"] = serde_json::json!(0);
        let battle_cursor_save: SaveGame =
            serde_json::from_value(battle_cursor_save_json).expect("decode exact save shape");
        let error = battle_cursor_save
            .validate()
            .expect_err("saved battle cursors must match active battle");
        assert!(
            matches!(error, SaveError::InvalidState(message) if message.contains("battle_active_enemy_party_index"))
        );
    }

    #[test]
    fn modpack_identity_hashes_compiled_pack_bytes() {
        let identity =
            SaveModpackIdentity::from_compiled_pack_bytes("core-modular", b"compiled-pack")
                .expect("identity");

        assert_eq!(identity.id, "core-modular");
        assert_eq!(identity.hash.len(), 8);
        assert!(
            identity
                .hash
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        );
        let empty = SaveModpackIdentity::from_compiled_pack_bytes("core-modular", b"")
            .expect_err("empty compiled pack bytes are not a runtime pack identity");
        assert!(
            empty
                .to_string()
                .contains("requires non-empty compiled pack bytes"),
            "{empty}"
        );
    }
}
