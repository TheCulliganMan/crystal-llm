use std::path::Path;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::multiplayer::fnv1a32_hex_bytes;
use crate::state::GameState;

const SAVE_MAGIC: &[u8; 12] = b"CRYSTALSAVE\0";
pub const SAVE_EXTENSION: &str = "crystalsave";
pub const SAVE_FORMAT_VERSION: u16 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SaveModpackIdentity {
    pub id: String,
    pub hash: String,
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
        let hash = fnv1a32_hex_bytes(compiled_pack_bytes);
        Self::new(id, hash)
    }

    pub fn validate(&self) -> Result<(), SaveError> {
        if self.id.trim().is_empty() {
            return Err(SaveError::InvalidIdentity(
                "save modpack id is required".to_string(),
            ));
        }
        if self.id.trim() != self.id {
            return Err(SaveError::InvalidIdentity(
                "save modpack id must be exact and untrimmed".to_string(),
            ));
        }
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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SaveMetadata {
    pub modpack: SaveModpackIdentity,
    pub created_frame: u64,
    pub saved_frame: u64,
}

impl SaveMetadata {
    pub fn new(modpack: SaveModpackIdentity, state: &GameState) -> Self {
        Self {
            modpack,
            created_frame: state.frame_counter,
            saved_frame: state.frame_counter,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SaveGame {
    pub format_version: u16,
    pub metadata: SaveMetadata,
    pub state: GameState,
}

impl SaveGame {
    pub fn new(state: GameState, modpack: SaveModpackIdentity) -> Self {
        Self {
            format_version: SAVE_FORMAT_VERSION,
            metadata: SaveMetadata::new(modpack, &state),
            state,
        }
    }

    pub fn validate(&self) -> Result<(), SaveError> {
        if self.format_version != SAVE_FORMAT_VERSION {
            return Err(SaveError::UnsupportedVersion(self.format_version));
        }
        self.metadata.modpack.validate()?;
        if self.metadata.saved_frame != self.state.frame_counter {
            return Err(SaveError::FrameMismatch {
                metadata_frame: self.metadata.saved_frame,
                state_frame: self.state.frame_counter,
            });
        }
        if self.metadata.created_frame > self.metadata.saved_frame {
            return Err(SaveError::InvalidIdentity(format!(
                "save created_frame {} cannot exceed saved_frame {}",
                self.metadata.created_frame, self.metadata.saved_frame
            )));
        }
        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum SaveError {
    #[error("save path {path} must use .{expected}")]
    InvalidExtension {
        path: String,
        expected: &'static str,
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
    #[error("save uses unsupported format version {0}")]
    UnsupportedVersion(u16),
    #[error("save has {0} trailing bytes")]
    TrailingBytes(usize),
    #[error("failed to encode save: {0}")]
    Encode(String),
    #[error("failed to decode save: {0}")]
    Decode(String),
    #[error("{0}")]
    InvalidIdentity(String),
    #[error("save metadata frame {metadata_frame} does not match state frame {state_frame}")]
    FrameMismatch {
        metadata_frame: u64,
        state_frame: u64,
    },
    #[error("save modpack hash {actual} does not match expected {expected}")]
    ModpackHashMismatch { expected: String, actual: String },
    #[error("save modpack id {actual} does not match expected {expected}")]
    ModpackIdMismatch { expected: String, actual: String },
}

pub fn write_save_game(path: impl AsRef<Path>, save: &SaveGame) -> Result<(), SaveError> {
    let path = path.as_ref();
    validate_save_path(path)?;
    save.validate()?;
    let encoded = bincode::serde::encode_to_vec(save, bincode::config::standard())
        .map_err(|error| SaveError::Encode(error.to_string()))?;
    let mut bytes = Vec::with_capacity(SAVE_MAGIC.len() + encoded.len());
    bytes.extend_from_slice(SAVE_MAGIC);
    bytes.extend_from_slice(&encoded);
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

pub fn read_save_game(path: impl AsRef<Path>) -> Result<SaveGame, SaveError> {
    let path = path.as_ref();
    validate_save_path(path)?;
    let bytes = std::fs::read(path).map_err(|source| SaveError::Read {
        path: path.display().to_string(),
        source,
    })?;
    read_save_game_bytes(&bytes, path.display().to_string())
}

pub fn read_save_game_bytes(
    bytes: &[u8],
    source_name: impl Into<String>,
) -> Result<SaveGame, SaveError> {
    let source_name = source_name.into();
    let payload = bytes
        .strip_prefix(SAVE_MAGIC)
        .ok_or_else(|| SaveError::InvalidMagic(source_name.clone()))?;
    let (save, consumed): (SaveGame, usize) =
        bincode::serde::decode_from_slice(payload, bincode::config::standard())
            .map_err(|error| SaveError::Decode(error.to_string()))?;
    if consumed != payload.len() {
        return Err(SaveError::TrailingBytes(payload.len() - consumed));
    }
    save.validate()?;
    Ok(save)
}

pub fn assert_save_matches_modpack(
    save: &SaveGame,
    expected: &SaveModpackIdentity,
) -> Result<(), SaveError> {
    expected.validate()?;
    save.metadata.modpack.validate()?;
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
    Ok(())
}

fn validate_save_path(path: &Path) -> Result<(), SaveError> {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default();
    if extension != SAVE_EXTENSION {
        return Err(SaveError::InvalidExtension {
            path: path.display().to_string(),
            expected: SAVE_EXTENSION,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let save = SaveGame::new(state.clone(), modpack.clone());

        write_save_game(&path, &save).expect("write save");
        let loaded = read_save_game(&path).expect("read save");

        assert_eq!(loaded.state, state);
        assert_eq!(loaded.metadata.modpack, modpack);
        assert_eq!(loaded.metadata.saved_frame, 42);
        let bytes = std::fs::read(&path).expect("read raw save");
        assert!(bytes.starts_with(SAVE_MAGIC));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn save_path_rejects_json_extension() {
        let path = temp_save_path("slot.json");
        let save = SaveGame::new(
            GameState::default(),
            SaveModpackIdentity::new("core-modular", "1234abcd").expect("identity"),
        );

        let error = write_save_game(&path, &save).expect_err("json saves are not runtime saves");

        assert!(matches!(error, SaveError::InvalidExtension { .. }));
    }

    #[test]
    fn save_rejects_trailing_bytes_and_bad_magic() {
        let path = temp_save_path("slot.crystalsave");
        let save = SaveGame::new(
            GameState::default(),
            SaveModpackIdentity::new("core-modular", "1234abcd").expect("identity"),
        );
        write_save_game(&path, &save).expect("write save");
        let mut bytes = std::fs::read(&path).expect("read raw save");
        bytes.push(0xff);

        assert!(matches!(
            read_save_game_bytes(&bytes, "slot.crystalsave"),
            Err(SaveError::TrailingBytes(1))
        ));
        assert!(matches!(
            read_save_game_bytes(b"{\"sram\":{}}", "legacy.json"),
            Err(SaveError::InvalidMagic(_))
        ));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn save_validates_modpack_identity_and_frame() {
        let mut save = SaveGame::new(
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

        let mut save_json = serde_json::to_value(SaveGame::new(
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
        let save = SaveGame::new(
            GameState::default(),
            SaveModpackIdentity::new("core-modular", "1234abcd").expect("identity"),
        );
        let expected = SaveModpackIdentity::new("core-modular", "ffffffff").expect("identity");

        let error = assert_save_matches_modpack(&save, &expected)
            .expect_err("mismatched modpack hashes must not load silently");

        assert!(matches!(error, SaveError::ModpackHashMismatch { .. }));
    }

    #[test]
    fn save_modpack_id_must_match_runtime_pack() {
        let save = SaveGame::new(
            GameState::default(),
            SaveModpackIdentity::new("core-modular", "1234abcd").expect("identity"),
        );
        let expected = SaveModpackIdentity::new("other-pack", "1234abcd").expect("identity");

        let error = assert_save_matches_modpack(&save, &expected)
            .expect_err("mismatched modpack ids must not load silently");

        assert!(matches!(error, SaveError::ModpackIdMismatch { .. }));
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
    }
}
