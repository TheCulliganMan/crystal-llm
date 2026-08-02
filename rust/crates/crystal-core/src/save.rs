use std::{
    collections::BTreeSet,
    path::{Component, Path, PathBuf},
};

use serde::{Deserialize, Deserializer, Serialize};
use thiserror::Error;

use sha2::{Digest, Sha256};

use crate::multiplayer::{fnv1a32_bytes, game_state_checksum};
use crate::state::GameState;

const SAVE_MAGIC: &[u8; 12] = b"CRYSTALSAVE\0";
pub const SAVE_EXTENSION: &str = "crystalsave";
pub const SAVE_FORMAT_VERSION: u16 = 5;
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
        let hash = sha256_hex(compiled_pack_bytes);
        Self::new(id, hash)
    }

    pub fn validate(&self) -> Result<(), SaveError> {
        Self::validate_id(&self.id)?;
        if self.hash.len() != 64
            || !self
                .hash
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(SaveError::InvalidIdentity(format!(
                "save modpack hash '{}' must be an exact 64-character lowercase SHA-256 hex digest",
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
    saved_state_hash: u32,
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
            saved_state_hash: u32,
        }

        let raw = RawSaveMetadata::deserialize(deserializer)?;
        let metadata = Self {
            modpack: raw.modpack,
            pack_content_hash: raw.pack_content_hash,
            created_frame: raw.created_frame,
            saved_frame: raw.saved_frame,
            saved_state_hash: raw.saved_state_hash,
        };
        metadata.validate().map_err(serde::de::Error::custom)?;
        Ok(metadata)
    }
}

impl SaveMetadata {
    fn new(
        modpack: SaveModpackIdentity,
        pack_content_hash: String,
        state: &GameState,
    ) -> Result<Self, SaveError> {
        let checksum = game_state_checksum(state).map_err(|error| {
            SaveError::InvalidState(format!(
                "failed to checksum save state for metadata: {error}"
            ))
        })?;
        let metadata = Self {
            modpack,
            pack_content_hash,
            created_frame: state.frame_counter,
            saved_frame: state.frame_counter,
            saved_state_hash: checksum.hash(),
        };
        metadata.validate()?;
        Ok(metadata)
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

    pub fn saved_state_hash(&self) -> u32 {
        self.saved_state_hash
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SaveGame {
    format_version: u16,
    metadata: SaveMetadata,
    state: GameState,
}

impl<'de> Deserialize<'de> for SaveGame {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawSaveGame {
            format_version: u16,
            metadata: SaveMetadata,
            state: GameState,
        }

        let raw = RawSaveGame::deserialize(deserializer)?;
        let save = Self {
            format_version: raw.format_version,
            metadata: raw.metadata,
            state: raw.state,
        };
        save.validate().map_err(serde::de::Error::custom)?;
        Ok(save)
    }
}

impl SaveGame {
    fn new(
        state: GameState,
        modpack: SaveModpackIdentity,
        pack_content_hash: String,
    ) -> Result<Self, SaveError> {
        let metadata = SaveMetadata::new(modpack, pack_content_hash, &state)?;
        let save = Self {
            format_version: SAVE_FORMAT_VERSION,
            metadata,
            state,
        };
        save.validate()?;
        Ok(save)
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
        let checksum = game_state_checksum(&self.state).map_err(|error| {
            SaveError::InvalidState(format!("failed to checksum save state: {error}"))
        })?;
        if self.metadata.saved_state_hash != checksum.hash() {
            return Err(SaveError::StateHashMismatch {
                expected: self.metadata.saved_state_hash,
                actual: checksum.hash(),
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SaveGameSummary {
    format_version: u16,
    modpack: SaveModpackIdentity,
    pack_content_hash: String,
    created_frame: u64,
    saved_frame: u64,
    state_frame: u64,
    state_hash: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveSlotSummary {
    slot_id: String,
    path: PathBuf,
    summary: SaveGameSummary,
}

impl SaveSlotSummary {
    fn new(path: PathBuf, summary: SaveGameSummary) -> Result<Self, SaveError> {
        validate_save_path(&path)?;
        let file_stem = path
            .file_stem()
            .and_then(|file_stem| file_stem.to_str())
            .ok_or_else(|| {
                SaveError::InvalidIdentity(format!(
                    "save slot path {} must have a UTF-8 slot id",
                    path.display()
                ))
            })?;
        validate_save_slot_id(file_stem)?;
        Ok(Self {
            slot_id: file_stem.to_string(),
            path,
            summary,
        })
    }

    pub fn slot_id(&self) -> &str {
        &self.slot_id
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn summary(&self) -> &SaveGameSummary {
        &self.summary
    }
}

impl<'de> Deserialize<'de> for SaveGameSummary {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawSaveGameSummary {
            format_version: u16,
            modpack: SaveModpackIdentity,
            pack_content_hash: String,
            created_frame: u64,
            saved_frame: u64,
            state_frame: u64,
            state_hash: u32,
        }

        let raw = RawSaveGameSummary::deserialize(deserializer)?;
        let summary = Self {
            format_version: raw.format_version,
            modpack: raw.modpack,
            pack_content_hash: raw.pack_content_hash,
            created_frame: raw.created_frame,
            saved_frame: raw.saved_frame,
            state_frame: raw.state_frame,
            state_hash: raw.state_hash,
        };
        summary.validate().map_err(serde::de::Error::custom)?;
        Ok(summary)
    }
}

impl SaveGameSummary {
    pub fn new(
        modpack: SaveModpackIdentity,
        pack_content_hash: String,
        state: &GameState,
    ) -> Result<Self, SaveError> {
        state
            .validate_saved_state()
            .map_err(SaveError::InvalidState)?;
        let checksum = game_state_checksum(state).map_err(|error| {
            SaveError::InvalidState(format!("failed to checksum save summary state: {error}"))
        })?;
        let summary = Self {
            format_version: SAVE_FORMAT_VERSION,
            modpack,
            pack_content_hash,
            created_frame: state.frame_counter,
            saved_frame: state.frame_counter,
            state_frame: state.frame_counter,
            state_hash: checksum.hash(),
        };
        summary.validate()?;
        Ok(summary)
    }

    pub fn from_save(save: &SaveGame) -> Result<Self, SaveError> {
        save.validate()?;
        Ok(Self {
            format_version: save.format_version,
            modpack: save.metadata.modpack.clone(),
            pack_content_hash: save.metadata.pack_content_hash.clone(),
            created_frame: save.metadata.created_frame,
            saved_frame: save.metadata.saved_frame,
            state_frame: save.state.frame_counter,
            state_hash: save.metadata.saved_state_hash,
        })
    }

    pub fn validate(&self) -> Result<(), SaveError> {
        if self.format_version != SAVE_FORMAT_VERSION {
            return Err(SaveError::UnsupportedVersion(self.format_version));
        }
        self.modpack.validate()?;
        validate_pack_content_hash(&self.pack_content_hash)?;
        if self.created_frame > self.saved_frame {
            return Err(SaveError::InvalidIdentity(format!(
                "save summary created_frame {} cannot exceed saved_frame {}",
                self.created_frame, self.saved_frame
            )));
        }
        if self.saved_frame != self.state_frame {
            return Err(SaveError::FrameMismatch {
                metadata_frame: self.saved_frame,
                state_frame: self.state_frame,
            });
        }
        Ok(())
    }

    pub fn format_version(&self) -> u16 {
        self.format_version
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

    pub fn state_frame(&self) -> u64 {
        self.state_frame
    }

    pub fn state_hash(&self) -> u32 {
        self.state_hash
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
    #[error("failed to read save directory {path}: {source}")]
    ReadDirectory {
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
    #[error("save state hash {actual:#010x} does not match metadata {expected:#010x}")]
    StateHashMismatch { expected: u32, actual: u32 },
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
    let backup = save_backup_path(path);
    if path.exists() {
        if backup.exists() {
            std::fs::remove_file(&backup).map_err(|source| SaveError::Write {
                path: backup.display().to_string(),
                source,
            })?;
        }
        std::fs::rename(path, &backup).map_err(|source| SaveError::Write {
            path: backup.display().to_string(),
            source,
        })?;
    }
    if let Err(error) = write_primary_save_bytes(path, &bytes) {
        if !path.exists() && backup.exists() {
            let _ = std::fs::rename(&backup, path);
        }
        return Err(error);
    }
    Ok(())
}

fn save_backup_path(path: &Path) -> PathBuf {
    PathBuf::from(format!("{}.bak", path.display()))
}

fn write_primary_save_bytes(path: &Path, bytes: &[u8]) -> Result<(), SaveError> {
    let temporary = PathBuf::from(format!("{}.tmp-{}", path.display(), std::process::id()));
    std::fs::write(&temporary, bytes).map_err(|source| SaveError::Write {
        path: temporary.display().to_string(),
        source,
    })?;
    if let Err(source) = std::fs::rename(&temporary, path) {
        let _ = std::fs::remove_file(&temporary);
        return Err(SaveError::Write {
            path: path.display().to_string(),
            source,
        });
    }
    Ok(())
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
    let save = SaveGame::new(state, modpack.clone(), pack_content_hash.to_string())?;
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
    let path = path.as_ref();
    let save = match read_save_game(path) {
        Ok(save) => save,
        Err(primary_error) => {
            let backup = save_backup_path(path);
            if !backup.exists() {
                return Err(primary_error);
            }
            let backup_bytes = match std::fs::read(&backup) {
                Ok(bytes) => bytes,
                Err(_) => return Err(primary_error),
            };
            let save = match read_save_game_bytes(&backup_bytes, backup.display().to_string()) {
                Ok(save) => save,
                Err(_) => return Err(primary_error),
            };
            assert_save_matches_modpack(&save, expected, expected_pack_content_hash)?;
            let bytes = encode_save_game_bytes(&save)?;
            write_primary_save_bytes(path, &bytes)?;
            save
        }
    };
    assert_save_matches_modpack(&save, expected, expected_pack_content_hash)?;
    Ok(save)
}

fn read_save_game_summary(path: impl AsRef<Path>) -> Result<SaveGameSummary, SaveError> {
    let save = read_save_game(path)?;
    SaveGameSummary::from_save(&save)
}

pub fn read_save_game_summary_for_modpack(
    path: impl AsRef<Path>,
    expected: &SaveModpackIdentity,
    expected_pack_content_hash: &str,
) -> Result<SaveGameSummary, SaveError> {
    let save = read_save_game_for_modpack(path, expected, expected_pack_content_hash)?;
    SaveGameSummary::from_save(&save)
}

pub fn list_save_game_summaries_for_modpack(
    directory: impl AsRef<Path>,
    expected: &SaveModpackIdentity,
    expected_pack_content_hash: &str,
) -> Result<Vec<SaveSlotSummary>, SaveError> {
    let directory = directory.as_ref();
    validate_save_directory_path(directory)?;
    expected.validate()?;
    validate_pack_content_hash(expected_pack_content_hash)?;
    let mut paths = Vec::new();
    for entry in std::fs::read_dir(directory).map_err(|source| SaveError::ReadDirectory {
        path: directory.display().to_string(),
        source,
    })? {
        let entry = entry.map_err(|source| SaveError::ReadDirectory {
            path: directory.display().to_string(),
            source,
        })?;
        paths.push(entry.path());
    }
    paths.sort();

    let mut summaries = Vec::with_capacity(paths.len());
    for path in paths {
        if path.extension().and_then(|extension| extension.to_str()) != Some(SAVE_EXTENSION) {
            continue;
        }
        validate_save_path(&path)?;
        let summary =
            match read_save_game_summary_for_modpack(&path, expected, expected_pack_content_hash) {
                Ok(summary) => summary,
                Err(
                    SaveError::ModpackIdMismatch { .. }
                    | SaveError::ModpackHashMismatch { .. }
                    | SaveError::PackContentHashMismatch { .. },
                ) => continue,
                Err(error) => return Err(error),
            };
        summaries.push(SaveSlotSummary::new(path, summary)?);
    }
    Ok(summaries)
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
    let save = read_save_game_bytes(bytes, source_name)?;
    assert_save_matches_modpack(&save, expected, expected_pack_content_hash)?;
    Ok(save)
}

pub fn read_save_game_summary_bytes_for_modpack(
    bytes: &[u8],
    source_name: impl Into<String>,
    expected: &SaveModpackIdentity,
    expected_pack_content_hash: &str,
) -> Result<SaveGameSummary, SaveError> {
    let save =
        read_save_game_bytes_for_modpack(bytes, source_name, expected, expected_pack_content_hash)?;
    SaveGameSummary::from_save(&save)
}

fn assert_save_matches_modpack(
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
    if hash.len() != 64
        || !hash
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(SaveError::InvalidIdentity(format!(
            "save pack content hash '{hash}' must be an exact 64-character lowercase SHA-256 hex digest"
        )));
    }
    Ok(())
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn validate_save_path(path: &Path) -> Result<(), SaveError> {
    let path_text = path.as_os_str().to_string_lossy();
    if path_text == "." || path_text.starts_with("./") || path_text.contains("/./") {
        return Err(SaveError::InvalidPathComponent {
            path: path.display().to_string(),
            component: "current-directory",
        });
    }
    if path_text == ".." || path_text.starts_with("../") || path_text.contains("/../") {
        return Err(SaveError::InvalidPathComponent {
            path: path.display().to_string(),
            component: "parent-directory",
        });
    }
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
        Some(extension) if extension == SAVE_EXTENSION => {
            let file_stem = path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .ok_or_else(|| {
                    SaveError::InvalidIdentity(format!(
                        "save path {} must have a UTF-8 slot id",
                        path.display()
                    ))
                })?;
            validate_save_slot_id(file_stem)
        }
        _ => Err(SaveError::InvalidExtension {
            path: path.display().to_string(),
            expected: SAVE_EXTENSION,
        }),
    }
}

fn validate_save_directory_path(path: &Path) -> Result<(), SaveError> {
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
    Ok(())
}

fn validate_save_slot_id(slot_id: &str) -> Result<(), SaveError> {
    if slot_id.is_empty()
        || slot_id.trim() != slot_id
        || !slot_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
    {
        return Err(SaveError::InvalidIdentity(format!(
            "save slot id '{slot_id}' must be exact ASCII using only letters, numbers, underscores, hyphens, or dots"
        )));
    }
    Ok(())
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
        "0102030401020304010203040102030401020304010203040102030401020304"
    }

    fn test_save(state: GameState, modpack: SaveModpackIdentity) -> SaveGame {
        SaveGame::new(state, modpack, pack_content_hash().to_string()).expect("test save")
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

    fn temp_save_dir(name: &str) -> std::path::PathBuf {
        let path = temp_save_path(name);
        std::fs::create_dir_all(&path).expect("create temp save dir");
        path
    }

    #[test]
    fn save_game_round_trips_as_binary_artifact() {
        let path = temp_save_path("slot.crystalsave");
        let mut state = GameState::default();
        state.frame_counter = 42;
        state.pokedex.seen_species.insert("CHIKORITA".to_string());
        let modpack = SaveModpackIdentity::new(
            "core-modular",
            "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd",
        )
        .expect("identity");
        let save = test_save(state.clone(), modpack.clone());

        write_save_game(&path, &save).expect("write save");
        let loaded = read_save_game(&path).expect("read save");
        let loaded_for_pack = read_save_game_for_modpack(&path, &modpack, pack_content_hash())
            .expect("read save for exact pack");

        assert_eq!(loaded.state, state);
        assert_eq!(loaded_for_pack, loaded);
        assert_eq!(loaded.metadata.modpack, modpack);
        assert_eq!(loaded.metadata.saved_frame, 42);
        let expected_state_hash = game_state_checksum(&state).expect("state checksum").hash();
        assert_eq!(loaded.metadata.saved_state_hash(), expected_state_hash);
        let summary = read_save_game_summary_for_modpack(&path, &modpack, pack_content_hash())
            .expect("read exact save summary");
        let direct_summary =
            SaveGameSummary::new(modpack.clone(), pack_content_hash().to_string(), &state)
                .expect("direct save summary");
        assert_eq!(summary.format_version(), SAVE_FORMAT_VERSION);
        assert_eq!(direct_summary, summary);
        assert_eq!(summary.modpack(), &modpack);
        assert_eq!(summary.pack_content_hash(), pack_content_hash());
        assert_eq!(summary.created_frame(), 42);
        assert_eq!(summary.saved_frame(), 42);
        assert_eq!(summary.state_frame(), 42);
        assert_eq!(summary.state_hash(), expected_state_hash);
        let bytes = std::fs::read(&path).expect("read raw save");
        let summary_from_bytes = read_save_game_summary_bytes_for_modpack(
            &bytes,
            "slot.crystalsave",
            &modpack,
            pack_content_hash(),
        )
        .expect("read exact save summary bytes");
        assert_eq!(summary_from_bytes, summary);
        assert!(bytes.starts_with(SAVE_MAGIC));
        assert_eq!(
            u16::from_be_bytes([bytes[SAVE_VERSION_OFFSET], bytes[SAVE_VERSION_OFFSET + 1]]),
            SAVE_FORMAT_VERSION
        );
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn save_slot_index_lists_only_exact_pack_bound_crystalsaves() {
        let directory = temp_save_dir("slot-index");
        let modpack = SaveModpackIdentity::new(
            "core-modular",
            "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd",
        )
        .expect("identity");
        let mut first_state = GameState::default();
        first_state.frame_counter = 12;
        let mut second_state = GameState::default();
        second_state.frame_counter = 18;

        write_save_game_for_modpack(
            directory.join("b-slot.crystalsave"),
            second_state.clone(),
            &modpack,
            pack_content_hash(),
        )
        .expect("write second save");
        write_save_game_for_modpack(
            directory.join("a-slot.crystalsave"),
            first_state.clone(),
            &modpack,
            pack_content_hash(),
        )
        .expect("write first save");

        let slots = list_save_game_summaries_for_modpack(&directory, &modpack, pack_content_hash())
            .expect("list exact save slots");

        assert_eq!(slots.len(), 2);
        assert_eq!(slots[0].slot_id(), "a-slot");
        assert_eq!(slots[0].path(), directory.join("a-slot.crystalsave"));
        assert_eq!(slots[0].summary().state_frame(), 12);
        assert_eq!(slots[1].slot_id(), "b-slot");
        assert_eq!(slots[1].path(), directory.join("b-slot.crystalsave"));
        assert_eq!(slots[1].summary().state_frame(), 18);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn save_slot_index_ignores_non_save_entries_and_skips_incompatible_saves() {
        let directory = temp_save_dir("slot-index-invalid");
        let modpack = SaveModpackIdentity::new(
            "core-modular",
            "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd",
        )
        .expect("identity");
        let other_modpack = SaveModpackIdentity::new(
            "other-pack",
            "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd",
        )
        .expect("identity");

        write_save_game_for_modpack(
            directory.join("slot.crystalsave"),
            GameState::default(),
            &modpack,
            pack_content_hash(),
        )
        .expect("write save");
        std::fs::write(directory.join("notes.txt"), b"not a save").expect("write non-save entry");

        let slots = list_save_game_summaries_for_modpack(&directory, &modpack, pack_content_hash())
            .expect("list ignores non-save entries");
        assert_eq!(slots.len(), 1);
        assert_eq!(slots[0].slot_id(), "slot");
        std::fs::remove_file(directory.join("notes.txt")).expect("remove non-save entry");

        write_save_game_for_modpack(
            directory.join("other.crystalsave"),
            GameState::default(),
            &other_modpack,
            pack_content_hash(),
        )
        .expect("write incompatible save");
        let slots = list_save_game_summaries_for_modpack(&directory, &modpack, pack_content_hash())
            .expect("list skips incompatible save");
        assert_eq!(slots.len(), 1);
        assert_eq!(slots[0].slot_id(), "slot");
        std::fs::remove_file(directory.join("other.crystalsave"))
            .expect("remove incompatible save");

        write_save_game_for_modpack(
            directory.join("other-hash.crystalsave"),
            GameState::default(),
            &SaveModpackIdentity::new(
                "core-modular",
                "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
            )
            .expect("identity"),
            pack_content_hash(),
        )
        .expect("write incompatible hash save");
        let slots = list_save_game_summaries_for_modpack(&directory, &modpack, pack_content_hash())
            .expect("list skips incompatible save hash");
        assert_eq!(slots.len(), 1);
        assert_eq!(slots[0].slot_id(), "slot");
        std::fs::remove_file(directory.join("other-hash.crystalsave"))
            .expect("remove incompatible hash save");

        write_save_game_for_modpack(
            directory.join("other-content.crystalsave"),
            GameState::default(),
            &modpack,
            "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
        )
        .expect("write incompatible content save");
        let slots = list_save_game_summaries_for_modpack(&directory, &modpack, pack_content_hash())
            .expect("list skips incompatible content save");
        assert_eq!(slots.len(), 1);
        assert_eq!(slots[0].slot_id(), "slot");
        std::fs::remove_file(directory.join("other-content.crystalsave"))
            .expect("remove incompatible content save");

        let bad_slot_error = write_save_game_for_modpack(
            directory.join("bad slot.crystalsave"),
            GameState::default(),
            &modpack,
            pack_content_hash(),
        )
        .expect_err("write must reject invalid slot name");
        assert!(matches!(
            bad_slot_error,
            SaveError::InvalidIdentity(message) if message.contains("save slot id")
        ));
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn save_slot_index_rejects_corrupt_crystalsaves_instead_of_hiding_them() {
        let directory = temp_save_dir("slot-index-corrupt");
        let modpack = SaveModpackIdentity::new(
            "core-modular",
            "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd",
        )
        .expect("identity");
        write_save_game_for_modpack(
            directory.join("slot.crystalsave"),
            GameState::default(),
            &modpack,
            pack_content_hash(),
        )
        .expect("write save");
        std::fs::write(directory.join("corrupt.crystalsave"), b"not a save")
            .expect("write corrupt save");

        let error = list_save_game_summaries_for_modpack(&directory, &modpack, pack_content_hash())
            .expect_err("corrupt runtime save must not be hidden by slot listing");

        assert!(
            matches!(error, SaveError::InvalidMagic(path) if path.ends_with("corrupt.crystalsave"))
        );
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn save_path_rejects_json_extension() {
        let path = temp_save_path("slot.json");
        let save = test_save(
            GameState::default(),
            SaveModpackIdentity::new(
                "core-modular",
                "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd",
            )
            .expect("identity"),
        );

        let error = write_save_game(&path, &save).expect_err("json saves are not runtime saves");

        assert!(matches!(error, SaveError::InvalidExtension { .. }));
    }

    #[test]
    fn save_path_requires_explicit_crystalsave_extension() {
        let path = temp_save_path("slot");
        let save = test_save(
            GameState::default(),
            SaveModpackIdentity::new(
                "core-modular",
                "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd",
            )
            .expect("identity"),
        );

        let error = write_save_game(&path, &save).expect_err("missing extension is invalid");

        assert!(matches!(error, SaveError::InvalidExtension { .. }));
    }

    #[test]
    fn save_path_rejects_malformed_slot_id_before_write() {
        let path = temp_save_path("bad slot.crystalsave");
        let save = test_save(
            GameState::default(),
            SaveModpackIdentity::new(
                "core-modular",
                "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd",
            )
            .expect("identity"),
        );

        let error = write_save_game(&path, &save).expect_err("malformed slot id is invalid");

        assert!(matches!(
            error,
            SaveError::InvalidIdentity(message) if message.contains("save slot id")
        ));
    }

    #[test]
    fn save_paths_reject_current_and_parent_directory_aliases() {
        let save = test_save(
            GameState::default(),
            SaveModpackIdentity::new(
                "core-modular",
                "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd",
            )
            .expect("identity"),
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

        let parent_dir_error = read_save_game("saves/../slot.crystalsave")
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
            SaveModpackIdentity::new(
                "core-modular",
                "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd",
            )
            .expect("identity"),
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
    fn framed_save_bytes_accept_transport_source_labels_without_path_coercion() {
        let modpack = SaveModpackIdentity::new(
            "core-modular",
            "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd",
        )
        .expect("identity");
        let save = test_save(GameState::default(), modpack.clone());
        let bytes = encode_save_game_bytes(&save).expect("encode framed save");

        let loaded = read_save_game_bytes_for_modpack(
            &bytes,
            "peer/session-1/checkpoint",
            &modpack,
            pack_content_hash(),
        )
        .expect("transport-labelled save bytes load by frame and pack identity");

        assert_eq!(loaded, save);
    }

    #[test]
    fn save_rejects_empty_payload_hash_mismatch_and_legacy_unframed_payloads() {
        let save = test_save(
            GameState::default(),
            SaveModpackIdentity::new(
                "core-modular",
                "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd",
            )
            .expect("identity"),
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

        let mut payload_with_trailing =
            bincode::serde::encode_to_vec(&save, save_binary_config()).expect("encode save");
        payload_with_trailing.push(0xff);
        let mut framed_trailing = Vec::with_capacity(SAVE_HEADER_LEN + payload_with_trailing.len());
        framed_trailing.extend_from_slice(SAVE_MAGIC);
        framed_trailing.extend_from_slice(&SAVE_FORMAT_VERSION.to_be_bytes());
        framed_trailing.extend_from_slice(&(payload_with_trailing.len() as u32).to_be_bytes());
        framed_trailing.extend_from_slice(&fnv1a32_bytes(&payload_with_trailing).to_be_bytes());
        framed_trailing.extend_from_slice(&payload_with_trailing);
        assert!(matches!(
            read_save_game_bytes(&framed_trailing, "slot.crystalsave"),
            Err(SaveError::TrailingBytes(1))
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
            SaveModpackIdentity::new(
                "core-modular",
                "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd",
            )
            .expect("identity"),
        );
        save.metadata.saved_frame = 9;

        assert!(matches!(
            save.validate(),
            Err(SaveError::FrameMismatch {
                metadata_frame: 9,
                state_frame: 0
            })
        ));
        let mut hash_mismatch = test_save(
            GameState::default(),
            SaveModpackIdentity::new(
                "core-modular",
                "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd",
            )
            .expect("identity"),
        );
        let expected_hash = hash_mismatch.metadata.saved_state_hash;
        hash_mismatch.metadata.saved_state_hash ^= 1;
        assert!(matches!(
            hash_mismatch.validate(),
            Err(SaveError::StateHashMismatch { expected, actual })
                if expected == expected_hash ^ 1 && actual == expected_hash
        ));
        assert!(SaveModpackIdentity::new("core-modular", "not-a-hash").is_err());
        let uppercase_hash = SaveModpackIdentity::new(
            "core-modular",
            "ABCDEF12ABCDEF12ABCDEF12ABCDEF12ABCDEF12ABCDEF12ABCDEF12ABCDEF12",
        )
        .expect_err("hash is exact lowercase");
        assert!(
            uppercase_hash
                .to_string()
                .contains("exact 64-character lowercase SHA-256 hex digest"),
            "{uppercase_hash}"
        );
        let padded_id = SaveModpackIdentity::new(
            " core-modular ",
            "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd",
        )
        .expect_err("id is untrimmed");
        assert!(
            padded_id
                .to_string()
                .contains("id must be exact and untrimmed"),
            "{padded_id}"
        );
        let whitespace_id = SaveModpackIdentity::new(
            " ",
            "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd",
        )
        .expect_err("id whitespace is untrimmed");
        assert!(
            whitespace_id
                .to_string()
                .contains("id must be exact and untrimmed"),
            "{whitespace_id}"
        );
        let spaced_id = SaveModpackIdentity::new(
            "core modular",
            "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd",
        )
        .expect_err("id is a token");
        assert!(
            spaced_id.to_string().contains("separated manifest ids"),
            "{spaced_id}"
        );
        let joined_identity = SaveModpackIdentity::new(
            "core-modular+johto.plus",
            "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd",
        )
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
            let error = SaveModpackIdentity::new(
                malformed_id,
                "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd",
            )
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
            let error = SaveModpackIdentity::new(
                reserved_id,
                "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd",
            )
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
    fn save_game_deserialize_rejects_invalid_frame_without_later_fixup() {
        let mut save = test_save(
            GameState::default(),
            SaveModpackIdentity::new(
                "core-modular",
                "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd",
            )
            .expect("identity"),
        );
        save.metadata.saved_frame = 7;
        let payload =
            bincode::serde::encode_to_vec(&save, save_binary_config()).expect("encode save");

        let error = match bincode::serde::decode_from_slice::<SaveGame, _>(
            &payload,
            save_binary_config(),
        ) {
            Ok(_) => panic!("invalid save frame must fail during SaveGame deserialize"),
            Err(error) => error.to_string(),
        };
        assert!(
            error.contains("save metadata frame 7 does not match state frame 0"),
            "{error}"
        );
    }

    #[test]
    fn corrupted_primary_save_recovers_from_backup_and_repairs_primary() {
        let path = temp_save_path("backup-recovery.crystalsave");
        let backup = save_backup_path(&path);
        let modpack = SaveModpackIdentity::new(
            "core-modular",
            "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd",
        )
        .expect("identity");

        let mut first_state = GameState::default();
        first_state.frame_counter = 11;
        write_save_game(&path, &test_save(first_state.clone(), modpack.clone()))
            .expect("write first save");

        let mut second_state = first_state.clone();
        second_state.frame_counter = 12;
        write_save_game(&path, &test_save(second_state, modpack.clone()))
            .expect("write second save");
        assert!(backup.exists(), "second write should preserve a backup");

        std::fs::write(&path, b"truncated primary").expect("corrupt primary save");
        let recovered = read_save_game_for_modpack(&path, &modpack, pack_content_hash())
            .expect("recover valid backup");

        assert_eq!(recovered.state.frame_counter, first_state.frame_counter);
        assert_eq!(
            read_save_game(&path).expect("repaired primary").state,
            first_state
        );
        let backup_save = read_save_game_bytes(
            &std::fs::read(&backup).expect("read preserved backup"),
            backup.display().to_string(),
        )
        .expect("preserved backup");
        assert_eq!(backup_save.state, first_state);

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(&backup);
    }

    #[test]
    fn save_json_payloads_reject_unknown_fields_without_identity_fallbacks() {
        let identity_error = serde_json::from_value::<SaveModpackIdentity>(serde_json::json!({
            "id": "core-modular",
            "hash": "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd",
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
            "hash": "ABCDEF12ABCDEF12ABCDEF12ABCDEF12ABCDEF12ABCDEF12ABCDEF12ABCDEF12"
        }))
        .expect_err("modpack identity hashes must validate during JSON load")
        .to_string();
        assert!(
            hash_error.contains("exact 64-character lowercase SHA-256 hex digest"),
            "{hash_error}"
        );

        let reserved_error = serde_json::from_value::<SaveModpackIdentity>(serde_json::json!({
            "id": "core-modular+fallback-save",
            "hash": "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd"
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
                "hash": "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd"
            },
            "pack_content_hash": "0102030401020304010203040102030401020304010203040102030401020304",
            "created_frame": 9,
            "saved_frame": 8,
            "saved_state_hash": 0
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
                "hash": "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd"
            },
            "pack_content_hash": "0102030401020304010203040102030401020304010203040102030401020304",
            "created_frame": 8,
            "saved_frame": 8,
            "saved_state_hash": 0
        }))
        .expect_err("save metadata modpack must validate during JSON load")
        .to_string();
        assert!(
            metadata_identity_error.contains("uses reserved runtime pack prefix"),
            "{metadata_identity_error}"
        );

        let mut save_json = serde_json::to_value(test_save(
            GameState::default(),
            SaveModpackIdentity::new(
                "core-modular",
                "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd",
            )
            .expect("identity"),
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
            SaveModpackIdentity::new(
                "core-modular",
                "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd",
            )
            .expect("identity"),
        );
        let expected = SaveModpackIdentity::new(
            "core-modular",
            "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
        )
        .expect("identity");

        let error = assert_save_matches_modpack(&save, &expected, pack_content_hash())
            .expect_err("mismatched modpack hashes must not load silently");

        assert!(matches!(error, SaveError::ModpackHashMismatch { .. }));

        let bytes = { encode_save_game_bytes(&save).expect("encode framed save") };
        assert!(matches!(
            read_save_game_bytes_for_modpack(
                &bytes,
                "slot.crystalsave",
                &expected,
                pack_content_hash()
            ),
            Err(SaveError::ModpackHashMismatch { .. })
        ));
        assert!(matches!(
            read_save_game_summary_bytes_for_modpack(
                &bytes,
                "slot.crystalsave",
                &expected,
                pack_content_hash()
            ),
            Err(SaveError::ModpackHashMismatch { .. })
        ));
        assert!(matches!(
            read_save_game_bytes_for_modpack(&bytes, "slot.json", &expected, pack_content_hash()),
            Err(SaveError::ModpackHashMismatch { .. })
        ));
    }

    #[test]
    fn save_modpack_match_requires_valid_save_payload() {
        let expected = SaveModpackIdentity::new(
            "core-modular",
            "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd",
        )
        .expect("identity");
        let mut save = test_save(GameState::default(), expected.clone());
        save.metadata.saved_frame = 9;

        assert!(matches!(
            assert_save_matches_modpack(&save, &expected, pack_content_hash()),
            Err(SaveError::FrameMismatch {
                metadata_frame: 9,
                state_frame: 0
            })
        ));
        assert!(matches!(
            SaveGameSummary::from_save(&save),
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
            SaveModpackIdentity::new(
                "core-modular",
                "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd",
            )
            .expect("identity"),
        );
        let expected = SaveModpackIdentity::new(
            "other-pack",
            "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd",
        )
        .expect("identity");

        let error = assert_save_matches_modpack(&save, &expected, pack_content_hash())
            .expect_err("mismatched modpack ids must not load silently");

        assert!(matches!(error, SaveError::ModpackIdMismatch { .. }));
    }

    #[test]
    fn save_pack_content_hash_must_match_runtime_pack() {
        let expected = SaveModpackIdentity::new(
            "core-modular",
            "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd",
        )
        .expect("identity");
        let save = test_save(GameState::default(), expected.clone());

        let error = assert_save_matches_modpack(
            &save,
            &expected,
            "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
        )
        .expect_err("mismatched pack content hashes must not load silently");

        assert!(matches!(error, SaveError::PackContentHashMismatch { .. }));

        let bytes = encode_save_game_bytes(&save).expect("encode framed save");
        assert!(matches!(
            read_save_game_bytes_for_modpack(
                &bytes,
                "slot.crystalsave",
                &expected,
                "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"
            ),
            Err(SaveError::PackContentHashMismatch { .. })
        ));
    }

    #[test]
    fn save_validation_rejects_invalid_saved_state_identifiers() {
        let expected = SaveModpackIdentity::new(
            "core-modular",
            "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd",
        )
        .expect("identity");
        let mut save = test_save(GameState::default(), expected.clone());
        save.state
            .flags
            .event_flags
            .insert("EVENT_BAD FLAG".to_string(), true);

        let error = save
            .state
            .validate_saved_state()
            .expect_err("saved flag ids must be exact");
        assert!(error.contains("EVENT_BAD FLAG"));
        let error = SaveGameSummary::new(
            expected.clone(),
            pack_content_hash().to_string(),
            save.state(),
        )
        .expect_err("save summaries must reject unsaveable state");
        assert!(
            matches!(error, SaveError::InvalidState(message) if message.contains("EVENT_BAD FLAG"))
        );

        let bytes = { encode_save_game_bytes(&save).expect("encode framed save") };
        let error = read_save_game_bytes_for_modpack(
            &bytes,
            "slot.crystalsave",
            &expected,
            pack_content_hash(),
        )
        .expect_err("binary save load must validate decoded state");
        assert!(matches!(
            error,
            SaveError::Decode(_)
                | SaveError::InvalidState(_)
                | SaveError::StateHashMismatch { .. }
                | SaveError::FrameMismatch { .. }
        ));

        let mut scene_save_json =
            serde_json::to_value(test_save(GameState::default(), expected.clone()))
                .expect("save json");
        scene_save_json["state"]["scenes"]["map_scenes"]["ElmsLab"] =
            serde_json::json!("SCENE ELMSLAB NOOP");
        scene_save_json["state"]["scenes"]["map_scene_indices"]["ElmsLab"] = serde_json::json!(1);
        let error = serde_json::from_value::<SaveGame>(scene_save_json)
            .expect_err("saved scene ids must be exact")
            .to_string();
        assert!(error.contains("SCENE ELMSLAB NOOP"));

        let mut runtime_save_json =
            serde_json::to_value(test_save(GameState::default(), expected.clone()))
                .expect("save json");
        runtime_save_json["state"]["script_runtime"]["next_script"] =
            serde_json::json!(" .Done@Script");
        let error = serde_json::from_value::<SaveGame>(runtime_save_json)
            .expect_err("saved script labels must be exact")
            .to_string();
        assert!(error.contains(" .Done@Script"));

        let mut runtime_event_save_json =
            serde_json::to_value(test_save(GameState::default(), expected.clone()))
                .expect("save json");
        runtime_event_save_json["state"]["script_runtime"]["current_music"] =
            serde_json::json!("MUSIC ROUTE 29");
        let error = serde_json::from_value::<SaveGame>(runtime_event_save_json)
            .expect_err("saved runtime event ids must be exact")
            .to_string();
        assert!(error.contains("MUSIC ROUTE 29"));

        let mut runtime_queue_save_json =
            serde_json::to_value(test_save(GameState::default(), expected.clone()))
                .expect("save json");
        runtime_queue_save_json["state"]["script_runtime"]["command_queue"] = serde_json::json!([{
            "origin_map_name": "TestMap",
            "command": "callasm",
            "target": "Queued Target",
            "bank": "BANK1",
            "source_script": "QueueScript",
            "command_index": 6
        }]);
        let error = serde_json::from_value::<SaveGame>(runtime_queue_save_json)
            .expect_err("saved runtime queues must be exact")
            .to_string();
        assert!(error.contains("Queued Target"));

        let mut state_identity_save_json =
            serde_json::to_value(test_save(GameState::default(), expected.clone()))
                .expect("save json");
        state_identity_save_json["state"]["active_repel_item"] = serde_json::json!("SUPER REPEL");
        let error = serde_json::from_value::<SaveGame>(state_identity_save_json)
            .expect_err("saved state identifiers must be exact")
            .to_string();
        assert!(error.contains("SUPER REPEL"));

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
        let error = serde_json::from_value::<SaveGame>(overworld_save_json)
            .expect_err("saved overworld identifiers must be exact")
            .to_string();
        assert!(error.contains("Route 29"));

        let mut bag_save_json =
            serde_json::to_value(test_save(GameState::default(), expected.clone()))
                .expect("save json");
        bag_save_json["state"]["bag"]["items"]["POTION"] = serde_json::json!(100);
        let error = serde_json::from_value::<SaveGame>(bag_save_json)
            .expect_err("saved bag metadata must be exact")
            .to_string();
        assert!(error.contains("items.POTION quantity 100 exceeds stack limit 99"));

        let mut pc_bag_save_json =
            serde_json::to_value(test_save(GameState::default(), expected.clone()))
                .expect("save json");
        pc_bag_save_json["state"]["bag"]["pc_items"]["POTION"] = serde_json::json!(100);
        let error = serde_json::from_value::<SaveGame>(pc_bag_save_json)
            .expect_err("saved PC item metadata must be exact")
            .to_string();
        assert!(error.contains("pc_items.POTION quantity 100 exceeds stack limit 99"));

        let mut storage_save_json =
            serde_json::to_value(test_save(GameState::default(), expected.clone()))
                .expect("save json");
        let mut pc_box_json =
            serde_json::to_value(crate::models::PcBox::new(0)).expect("pc box json");
        pc_box_json["count"] = serde_json::json!(1);
        storage_save_json["state"]["storage"]["pc_boxes"] = serde_json::json!([pc_box_json]);
        let error = serde_json::from_value::<SaveGame>(storage_save_json)
            .expect_err("saved storage metadata must be exact")
            .to_string();
        assert!(error.contains("box count 1 must match filled pokemon slots 0"));

        let mut party_projection_save_json =
            serde_json::to_value(test_save(GameState::default(), expected.clone()))
                .expect("save json");
        party_projection_save_json["state"]["party"]["pokemon"][0] = serde_json::json!({
            "species": "CHIKORITA",
            "level": 6
        });
        let error = serde_json::from_value::<SaveGame>(party_projection_save_json)
            .expect_err("saved party projection must match storage")
            .to_string();
        assert!(error.contains("party projection") || error.contains("missing field"));

        let mut battle_cursor_save_json =
            serde_json::to_value(test_save(GameState::default(), expected.clone()))
                .expect("save json");
        battle_cursor_save_json["state"]["battle_active_enemy_party_index"] = serde_json::json!(0);
        let error = serde_json::from_value::<SaveGame>(battle_cursor_save_json)
            .expect_err("saved battle cursors must match active battle")
            .to_string();
        assert!(error.contains("battle_active_enemy_party_index"));
    }

    #[test]
    fn modpack_identity_hashes_compiled_pack_bytes() {
        let identity =
            SaveModpackIdentity::from_compiled_pack_bytes("core-modular", b"compiled-pack")
                .expect("identity");

        assert_eq!(identity.id, "core-modular");
        assert_eq!(
            identity.hash,
            "7563c469171e9467f67116b9ed207de1ec7b9180c4aebe7da709e6f991c1aa7f"
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
