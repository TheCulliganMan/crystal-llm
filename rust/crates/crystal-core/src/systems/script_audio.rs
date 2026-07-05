use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Deserializer, Serialize, de::Error as _};

use crate::models::PokemonSpecies;
use crate::state::{GameState, ScriptAudioRuntimeEvent, ScriptAudioRuntimeKind, ScriptMusicFade};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptAudioCommand {
    #[serde(deserialize_with = "required_audio_command_token")]
    pub command: String,
    #[serde(deserialize_with = "required_nullable_audio_token")]
    pub audio_id: Option<String>,
    pub fade_frames: Option<u16>,
    #[serde(deserialize_with = "required_audio_token")]
    pub source_script: String,
    pub command_index: usize,
}

impl<'de> Deserialize<'de> for ScriptAudioCommand {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawScriptAudioCommand {
            #[serde(deserialize_with = "required_audio_command_token")]
            command: String,
            #[serde(deserialize_with = "required_nullable_audio_token")]
            audio_id: Option<String>,
            fade_frames: Option<u16>,
            #[serde(deserialize_with = "required_audio_source_token")]
            source_script: String,
            command_index: usize,
        }

        let raw = RawScriptAudioCommand::deserialize(deserializer)?;
        let command = Self {
            command: raw.command,
            audio_id: raw.audio_id,
            fade_frames: raw.fade_frames,
            source_script: raw.source_script,
            command_index: raw.command_index,
        };
        validate_script_audio_command_shape(&command).map_err(D::Error::custom)?;
        Ok(command)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ScriptAudioKind {
    Music,
    SoundEffect,
    Cry,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ScriptAudioCue {
    Play {
        command: String,
        kind: ScriptAudioKind,
        audio_id: String,
        source_script: String,
        command_index: usize,
    },
    FadeMusic {
        audio_id: String,
        fade_frames: u16,
        source_script: String,
        command_index: usize,
    },
    WaitForSoundEffect {
        source_script: String,
        command_index: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum ScriptAudioError {
    InvalidCommand { command: String },
    InvalidSourceScript { source_script: String },
    UnknownCommand { command: String },
    MissingAudioId { command: String },
    UnexpectedAudioId { command: String },
    MissingFadeFrames { command: String },
    UnexpectedFadeFrames { command: String },
    InvalidMusic { audio_id: String },
    UnknownMusic { audio_id: String },
    InvalidSoundEffect { audio_id: String },
    UnknownSoundEffect { audio_id: String },
    InvalidCrySpecies { species_id: String },
    UnknownCrySpecies { species_id: String },
    MissingCryMetadata { species_id: String },
    InvalidCryAsset { audio_id: String },
    UnknownCryAsset { audio_id: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ScriptAudioCommandIssue {
    InvalidCommand,
    MissingMusicId,
    InvalidMusicId,
    UnknownMusicId,
    MissingSoundEffectId,
    InvalidSoundEffectId,
    UnknownSoundEffectId,
    MissingCrySpecies,
    InvalidCrySpecies,
    UnknownCrySpecies,
    MissingCryMetadata,
    InvalidCryAsset,
    UnknownCryAsset,
    MissingMusicFadeFrames,
    UnexpectedAudioId,
    UnexpectedFadeFrames,
    UnknownCommand,
}

pub const SCRIPT_AUDIO_MUSIC_COMMANDS: &[&str] = &["playmusic"];
pub const SCRIPT_AUDIO_SOUND_EFFECT_COMMANDS: &[&str] = &["playsound"];
pub const SCRIPT_AUDIO_CRY_COMMANDS: &[&str] = &["cry"];
pub const SCRIPT_AUDIO_MUSIC_FADE_COMMANDS: &[&str] = &["musicfadeout"];
pub const SCRIPT_AUDIO_NO_PAYLOAD_COMMANDS: &[&str] = &["waitsfx"];

pub fn is_known_script_audio_command(command: &str) -> bool {
    SCRIPT_AUDIO_MUSIC_COMMANDS.contains(&command)
        || SCRIPT_AUDIO_SOUND_EFFECT_COMMANDS.contains(&command)
        || SCRIPT_AUDIO_CRY_COMMANDS.contains(&command)
        || SCRIPT_AUDIO_MUSIC_FADE_COMMANDS.contains(&command)
        || SCRIPT_AUDIO_NO_PAYLOAD_COMMANDS.contains(&command)
}

fn validate_script_audio_command_shape(command: &ScriptAudioCommand) -> Result<(), String> {
    if !is_known_script_audio_command(&command.command) {
        return Err(format!("unknown script audio command {}", command.command));
    }
    match command.command.as_str() {
        "playmusic" | "playsound" | "cry" => {
            if command.audio_id.is_none() {
                return Err(format!(
                    "script audio command {} requires audio_id",
                    command.command
                ));
            }
            if command.fade_frames.is_some() {
                return Err(format!(
                    "script audio command {} must not declare fade_frames",
                    command.command
                ));
            }
        }
        "musicfadeout" => {
            if command.audio_id.is_none() {
                return Err("script audio command musicfadeout requires audio_id".to_string());
            }
            if command.fade_frames.is_none() {
                return Err("script audio command musicfadeout requires fade_frames".to_string());
            }
        }
        "waitsfx" => {
            if command.audio_id.is_some() {
                return Err("script audio command waitsfx must not declare audio_id".to_string());
            }
            if command.fade_frames.is_some() {
                return Err("script audio command waitsfx must not declare fade_frames".to_string());
            }
        }
        _ => unreachable!("known script audio command was not handled"),
    }
    Ok(())
}

pub fn script_audio_command_issues(
    command: &ScriptAudioCommand,
    music_ids: &BTreeSet<String>,
    sound_effect_ids: &BTreeSet<String>,
    cry_ids: &BTreeSet<String>,
    species: &BTreeMap<String, PokemonSpecies>,
    cry_by_species: &BTreeMap<String, String>,
) -> Vec<ScriptAudioCommandIssue> {
    let mut issues = Vec::new();
    match command.command.as_str() {
        "playmusic" => {
            check_audio_id(
                command.audio_id.as_deref(),
                music_ids,
                ScriptAudioCommandIssue::MissingMusicId,
                ScriptAudioCommandIssue::InvalidMusicId,
                ScriptAudioCommandIssue::UnknownMusicId,
                &mut issues,
            );
        }
        "playsound" => {
            check_audio_id(
                command.audio_id.as_deref(),
                sound_effect_ids,
                ScriptAudioCommandIssue::MissingSoundEffectId,
                ScriptAudioCommandIssue::InvalidSoundEffectId,
                ScriptAudioCommandIssue::UnknownSoundEffectId,
                &mut issues,
            );
        }
        "cry" => match command.audio_id.as_deref() {
            Some(species_id) if !is_exact_audio_token(species_id) => {
                issues.push(ScriptAudioCommandIssue::InvalidCrySpecies);
            }
            Some(species_id) if species.contains_key(species_id) => {
                match cry_by_species.get(species_id) {
                    Some(cry_id) if !is_exact_audio_token(cry_id) => {
                        issues.push(ScriptAudioCommandIssue::InvalidCryAsset);
                    }
                    Some(cry_id) if cry_ids.contains(cry_id) => {}
                    Some(_) => issues.push(ScriptAudioCommandIssue::UnknownCryAsset),
                    None => issues.push(ScriptAudioCommandIssue::MissingCryMetadata),
                }
            }
            Some(_) => issues.push(ScriptAudioCommandIssue::UnknownCrySpecies),
            None => issues.push(ScriptAudioCommandIssue::MissingCrySpecies),
        },
        "musicfadeout" => {
            check_audio_id(
                command.audio_id.as_deref(),
                music_ids,
                ScriptAudioCommandIssue::MissingMusicId,
                ScriptAudioCommandIssue::InvalidMusicId,
                ScriptAudioCommandIssue::UnknownMusicId,
                &mut issues,
            );
            if command.fade_frames.is_none() {
                issues.push(ScriptAudioCommandIssue::MissingMusicFadeFrames);
            }
        }
        "waitsfx" => {
            if command.audio_id.is_some() {
                issues.push(ScriptAudioCommandIssue::UnexpectedAudioId);
            }
        }
        _ if !is_exact_audio_command_token(&command.command) => {
            issues.push(ScriptAudioCommandIssue::InvalidCommand)
        }
        _ => issues.push(ScriptAudioCommandIssue::UnknownCommand),
    }
    if !SCRIPT_AUDIO_MUSIC_FADE_COMMANDS.contains(&command.command.as_str())
        && command.fade_frames.is_some()
    {
        issues.push(ScriptAudioCommandIssue::UnexpectedFadeFrames);
    }
    issues
}

pub fn resolve_script_audio_command(
    command: ScriptAudioCommand,
    music_ids: &BTreeSet<String>,
    sound_effect_ids: &BTreeSet<String>,
    cry_ids: &BTreeSet<String>,
    species: &BTreeMap<String, PokemonSpecies>,
    cry_by_species: &BTreeMap<String, String>,
) -> Result<ScriptAudioCue, ScriptAudioError> {
    reject_invalid_source_script(&command)?;
    if !is_exact_audio_command_token(&command.command) {
        return Err(ScriptAudioError::InvalidCommand {
            command: command.command,
        });
    }
    match command.command.as_str() {
        "playmusic" => {
            reject_fade_frames(&command)?;
            let audio_id = require_audio_id(&command)?.to_string();
            if !is_exact_audio_token(&audio_id) {
                return Err(ScriptAudioError::InvalidMusic { audio_id });
            }
            if !music_ids.contains(&audio_id) {
                return Err(ScriptAudioError::UnknownMusic { audio_id });
            }
            Ok(play_cue(command, ScriptAudioKind::Music, audio_id))
        }
        "playsound" => {
            reject_fade_frames(&command)?;
            let audio_id = require_audio_id(&command)?.to_string();
            if !is_exact_audio_token(&audio_id) {
                return Err(ScriptAudioError::InvalidSoundEffect { audio_id });
            }
            if !sound_effect_ids.contains(&audio_id) {
                return Err(ScriptAudioError::UnknownSoundEffect { audio_id });
            }
            Ok(play_cue(command, ScriptAudioKind::SoundEffect, audio_id))
        }
        "cry" => {
            reject_fade_frames(&command)?;
            let species_id = require_audio_id(&command)?.to_string();
            if !is_exact_audio_token(&species_id) {
                return Err(ScriptAudioError::InvalidCrySpecies { species_id });
            }
            if !species.contains_key(&species_id) {
                return Err(ScriptAudioError::UnknownCrySpecies { species_id });
            }
            let audio_id = cry_by_species.get(&species_id).cloned().ok_or_else(|| {
                ScriptAudioError::MissingCryMetadata {
                    species_id: species_id.clone(),
                }
            })?;
            if !is_exact_audio_token(&audio_id) {
                return Err(ScriptAudioError::InvalidCryAsset { audio_id });
            }
            if !cry_ids.contains(&audio_id) {
                return Err(ScriptAudioError::UnknownCryAsset { audio_id });
            }
            Ok(play_cue(command, ScriptAudioKind::Cry, audio_id))
        }
        "musicfadeout" => {
            let audio_id = require_audio_id(&command)?.to_string();
            let fade_frames =
                command
                    .fade_frames
                    .ok_or_else(|| ScriptAudioError::MissingFadeFrames {
                        command: command.command.clone(),
                    })?;
            if !is_exact_audio_token(&audio_id) {
                return Err(ScriptAudioError::InvalidMusic { audio_id });
            }
            if !music_ids.contains(&audio_id) {
                return Err(ScriptAudioError::UnknownMusic { audio_id });
            }
            Ok(ScriptAudioCue::FadeMusic {
                audio_id,
                fade_frames,
                source_script: command.source_script,
                command_index: command.command_index,
            })
        }
        "waitsfx" => {
            if command.audio_id.is_some() {
                return Err(ScriptAudioError::UnexpectedAudioId {
                    command: command.command,
                });
            }
            reject_fade_frames(&command)?;
            Ok(ScriptAudioCue::WaitForSoundEffect {
                source_script: command.source_script,
                command_index: command.command_index,
            })
        }
        other => Err(ScriptAudioError::UnknownCommand {
            command: other.to_string(),
        }),
    }
}

pub fn apply_script_audio_command(
    state: &mut GameState,
    command: ScriptAudioCommand,
    music_ids: &BTreeSet<String>,
    sound_effect_ids: &BTreeSet<String>,
    cry_ids: &BTreeSet<String>,
    species: &BTreeMap<String, PokemonSpecies>,
    cry_by_species: &BTreeMap<String, String>,
) -> Result<ScriptAudioCue, ScriptAudioError> {
    let cue = resolve_script_audio_command(
        command,
        music_ids,
        sound_effect_ids,
        cry_ids,
        species,
        cry_by_species,
    )?;
    apply_audio_cue_to_state(state, &cue);
    Ok(cue)
}

pub fn apply_audio_cue_to_state(state: &mut GameState, cue: &ScriptAudioCue) {
    match cue {
        ScriptAudioCue::Play {
            command,
            kind,
            audio_id,
            source_script,
            command_index,
        } => {
            if *kind == ScriptAudioKind::Music {
                state.script_runtime.current_music = Some(audio_id.clone());
                state.script_runtime.pending_music_fade = None;
            }
            state
                .script_runtime
                .audio_events
                .push(ScriptAudioRuntimeEvent {
                    command: command.clone(),
                    kind: runtime_kind(*kind),
                    audio_id: Some(audio_id.clone()),
                    fade_frames: None,
                    source_script: source_script.clone(),
                    command_index: *command_index,
                });
        }
        ScriptAudioCue::FadeMusic {
            audio_id,
            fade_frames,
            source_script,
            command_index,
        } => {
            state.script_runtime.pending_music_fade = Some(ScriptMusicFade {
                audio_id: audio_id.clone(),
                fade_frames: *fade_frames,
                source_script: source_script.clone(),
                command_index: *command_index,
            });
            state
                .script_runtime
                .audio_events
                .push(ScriptAudioRuntimeEvent {
                    command: "musicfadeout".to_string(),
                    kind: ScriptAudioRuntimeKind::FadeMusic,
                    audio_id: Some(audio_id.clone()),
                    fade_frames: Some(*fade_frames),
                    source_script: source_script.clone(),
                    command_index: *command_index,
                });
        }
        ScriptAudioCue::WaitForSoundEffect {
            source_script,
            command_index,
        } => {
            state.script_runtime.waiting_for_sound_effect = true;
            state
                .script_runtime
                .audio_events
                .push(ScriptAudioRuntimeEvent {
                    command: "waitsfx".to_string(),
                    kind: ScriptAudioRuntimeKind::WaitForSoundEffect,
                    audio_id: None,
                    fade_frames: None,
                    source_script: source_script.clone(),
                    command_index: *command_index,
                });
        }
    }
}

fn reject_invalid_source_script(command: &ScriptAudioCommand) -> Result<(), ScriptAudioError> {
    if is_exact_audio_source_token(&command.source_script) {
        Ok(())
    } else {
        Err(ScriptAudioError::InvalidSourceScript {
            source_script: command.source_script.clone(),
        })
    }
}

fn check_audio_id(
    audio_id: Option<&str>,
    known_ids: &BTreeSet<String>,
    missing: ScriptAudioCommandIssue,
    invalid: ScriptAudioCommandIssue,
    unknown: ScriptAudioCommandIssue,
    issues: &mut Vec<ScriptAudioCommandIssue>,
) {
    match audio_id {
        Some(audio_id) if !is_exact_audio_token(audio_id) => issues.push(invalid),
        Some(audio_id) if known_ids.contains(audio_id) => {}
        Some(_) => issues.push(unknown),
        None => issues.push(missing),
    }
}

fn is_exact_audio_command_token(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && value.bytes().all(|byte| byte.is_ascii_lowercase())
        && !has_reserved_pack_prefix(value)
}

fn is_exact_audio_token(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        && !has_reserved_pack_prefix(value)
}

fn is_exact_audio_source_token(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'@'))
        && !has_reserved_pack_prefix(value)
}

fn required_audio_command_token<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    if is_exact_audio_command_token(&value) {
        Ok(value)
    } else {
        Err(serde::de::Error::custom(format!(
            "script audio command must be exact lowercase ASCII, found {value:?}"
        )))
    }
}

fn required_audio_token<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    if is_exact_audio_token(&value) {
        Ok(value)
    } else {
        Err(serde::de::Error::custom(format!(
            "script audio token must be exact ASCII alphanumeric/underscore, found {value:?}"
        )))
    }
}

fn required_audio_source_token<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    if is_exact_audio_source_token(&value) {
        Ok(value)
    } else {
        Err(serde::de::Error::custom(format!(
            "script audio source token must be exact ASM label syntax, found {value:?}"
        )))
    }
}

fn required_nullable_audio_token<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<String>::deserialize(deserializer)?;
    match value {
        Some(token) if is_exact_audio_token(&token) => Ok(Some(token)),
        Some(token) => Err(serde::de::Error::custom(format!(
            "script audio token must be exact ASCII alphanumeric/underscore, found {token:?}"
        ))),
        None => Ok(None),
    }
}

fn has_reserved_pack_prefix(value: &str) -> bool {
    let value = value.to_ascii_lowercase();
    value.starts_with("fallback") || value.starts_with("legacy")
}

fn runtime_kind(kind: ScriptAudioKind) -> ScriptAudioRuntimeKind {
    match kind {
        ScriptAudioKind::Music => ScriptAudioRuntimeKind::Music,
        ScriptAudioKind::SoundEffect => ScriptAudioRuntimeKind::SoundEffect,
        ScriptAudioKind::Cry => ScriptAudioRuntimeKind::Cry,
    }
}

fn require_audio_id(command: &ScriptAudioCommand) -> Result<&str, ScriptAudioError> {
    command
        .audio_id
        .as_deref()
        .ok_or_else(|| ScriptAudioError::MissingAudioId {
            command: command.command.clone(),
        })
}

fn reject_fade_frames(command: &ScriptAudioCommand) -> Result<(), ScriptAudioError> {
    if command.fade_frames.is_some() {
        Err(ScriptAudioError::UnexpectedFadeFrames {
            command: command.command.clone(),
        })
    } else {
        Ok(())
    }
}

fn play_cue(
    command: ScriptAudioCommand,
    kind: ScriptAudioKind,
    audio_id: String,
) -> ScriptAudioCue {
    ScriptAudioCue::Play {
        command: command.command,
        kind,
        audio_id,
        source_script: command.source_script,
        command_index: command.command_index,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{BaseStats, PokemonSpecies};

    fn command(name: &str, audio_id: Option<&str>, fade_frames: Option<u16>) -> ScriptAudioCommand {
        ScriptAudioCommand {
            command: name.to_string(),
            audio_id: audio_id.map(str::to_string),
            fade_frames,
            source_script: "AudioScript".to_string(),
            command_index: 7,
        }
    }

    fn species(id: &str) -> PokemonSpecies {
        PokemonSpecies::new_for_tests(id, BaseStats::new(45, 49, 65, 45, 49, 65))
    }

    #[test]
    fn exported_audio_command_sets_are_exact() {
        assert!(SCRIPT_AUDIO_MUSIC_COMMANDS.contains(&"playmusic"));
        assert!(SCRIPT_AUDIO_SOUND_EFFECT_COMMANDS.contains(&"playsound"));
        assert!(SCRIPT_AUDIO_CRY_COMMANDS.contains(&"cry"));
        assert!(SCRIPT_AUDIO_MUSIC_FADE_COMMANDS.contains(&"musicfadeout"));
        assert!(SCRIPT_AUDIO_NO_PAYLOAD_COMMANDS.contains(&"waitsfx"));
        assert!(is_known_script_audio_command("playsound"));
        assert!(!is_known_script_audio_command("PlaySound"));
        assert!(!is_known_script_audio_command("fadeaudio"));
    }

    #[test]
    fn script_audio_serialized_variants_reject_unknown_fallback_fields() {
        let cue_error = serde_json::from_value::<ScriptAudioCue>(serde_json::json!({
            "play": {
                "command": "cry",
                "kind": "cry",
                "audio_id": "CRY_LUGIA",
                "source_script": "AudioScript",
                "command_index": 7,
                "fallback_audio_id": "CRY_DEFAULT"
            }
        }))
        .expect_err("audio cues must not accept fallback audio ids");
        assert!(
            cue_error
                .to_string()
                .contains("unknown field `fallback_audio_id`"),
            "{cue_error}"
        );

        let error_error = serde_json::from_value::<ScriptAudioError>(serde_json::json!({
            "UnknownCryAsset": {
                "audio_id": "CRY_LUGIA",
                "legacy_audio_id": "LUGIA"
            }
        }))
        .expect_err("audio errors must not accept legacy audio ids");
        assert!(
            error_error
                .to_string()
                .contains("unknown field `legacy_audio_id`"),
            "{error_error}"
        );
    }

    #[test]
    fn script_audio_resolves_commands_from_local_script_labels() {
        let music = BTreeSet::from(["MUSIC_RIVAL_AFTER".to_string()]);
        let sfx = BTreeSet::new();
        let cries = BTreeSet::new();
        let species = BTreeMap::new();
        let cry_by_species = BTreeMap::new();
        let mut command = command("playmusic", Some("MUSIC_RIVAL_AFTER"), None);
        command.source_script = ".AfterVictorious@CherrygroveRivalSceneNorth".to_string();

        let cue =
            resolve_script_audio_command(command, &music, &sfx, &cries, &species, &cry_by_species)
                .expect("local-label script audio command resolves");

        assert_eq!(
            cue,
            ScriptAudioCue::Play {
                command: "playmusic".to_string(),
                kind: ScriptAudioKind::Music,
                audio_id: "MUSIC_RIVAL_AFTER".to_string(),
                source_script: ".AfterVictorious@CherrygroveRivalSceneNorth".to_string(),
                command_index: 7,
            }
        );
    }

    #[test]
    fn script_audio_issue_collector_reports_exact_pack_shape_errors() {
        let music = BTreeSet::from(["MUSIC_ROUTE_29".to_string()]);
        let sfx = BTreeSet::from(["SFX_GET_BADGE".to_string()]);
        let cries = BTreeSet::from(["CRY_LUGIA".to_string()]);
        let species = BTreeMap::from([("LUGIA".to_string(), species("LUGIA"))]);
        let cry_by_species = BTreeMap::from([("LUGIA".to_string(), "CRY_HO_OH".to_string())]);

        assert_eq!(
            script_audio_command_issues(
                &command("playmusic", None, Some(4)),
                &music,
                &sfx,
                &cries,
                &species,
                &cry_by_species,
            ),
            vec![
                ScriptAudioCommandIssue::MissingMusicId,
                ScriptAudioCommandIssue::UnexpectedFadeFrames,
            ]
        );
        assert_eq!(
            script_audio_command_issues(
                &command("playmusic", Some("MUSIC ROUTE 29"), None),
                &music,
                &sfx,
                &cries,
                &species,
                &cry_by_species,
            ),
            vec![ScriptAudioCommandIssue::InvalidMusicId]
        );
        assert_eq!(
            script_audio_command_issues(
                &command("playsound", Some("sfx_get_badge"), None),
                &music,
                &sfx,
                &cries,
                &species,
                &cry_by_species,
            ),
            vec![ScriptAudioCommandIssue::UnknownSoundEffectId]
        );
        assert_eq!(
            script_audio_command_issues(
                &command("playsound", Some("SFX GET BADGE"), None),
                &music,
                &sfx,
                &cries,
                &species,
                &cry_by_species,
            ),
            vec![ScriptAudioCommandIssue::InvalidSoundEffectId]
        );
        assert_eq!(
            script_audio_command_issues(
                &command("cry", Some("LU GIA"), None),
                &music,
                &sfx,
                &cries,
                &species,
                &cry_by_species,
            ),
            vec![ScriptAudioCommandIssue::InvalidCrySpecies]
        );
        let invalid_cry_by_species =
            BTreeMap::from([("LUGIA".to_string(), "CRY LUGIA".to_string())]);
        assert_eq!(
            script_audio_command_issues(
                &command("cry", Some("LUGIA"), None),
                &music,
                &sfx,
                &cries,
                &species,
                &invalid_cry_by_species,
            ),
            vec![ScriptAudioCommandIssue::InvalidCryAsset]
        );
        assert_eq!(
            script_audio_command_issues(
                &command("cry", Some("LUGIA"), None),
                &music,
                &sfx,
                &cries,
                &species,
                &cry_by_species,
            ),
            vec![ScriptAudioCommandIssue::UnknownCryAsset]
        );
        assert_eq!(
            script_audio_command_issues(
                &command("musicfadeout", Some("MUSIC_ROUTE_29"), None),
                &music,
                &sfx,
                &cries,
                &species,
                &cry_by_species,
            ),
            vec![ScriptAudioCommandIssue::MissingMusicFadeFrames]
        );
        assert_eq!(
            script_audio_command_issues(
                &command("waitsfx", Some("SFX_GET_BADGE"), Some(1)),
                &music,
                &sfx,
                &cries,
                &species,
                &cry_by_species,
            ),
            vec![
                ScriptAudioCommandIssue::UnexpectedAudioId,
                ScriptAudioCommandIssue::UnexpectedFadeFrames,
            ]
        );
        assert_eq!(
            script_audio_command_issues(
                &command("PlaySound", Some("SFX_GET_BADGE"), None),
                &music,
                &sfx,
                &cries,
                &species,
                &cry_by_species,
            ),
            vec![ScriptAudioCommandIssue::InvalidCommand]
        );
        assert_eq!(
            script_audio_command_issues(
                &command("fadeaudio", Some("MUSIC_ROUTE_29"), None),
                &music,
                &sfx,
                &cries,
                &species,
                &cry_by_species,
            ),
            vec![ScriptAudioCommandIssue::UnknownCommand]
        );
    }

    #[test]
    fn script_audio_commands_reject_reserved_pack_prefixes() {
        let music = BTreeSet::from(["MUSIC_ROUTE_29".to_string()]);
        let sfx = BTreeSet::from(["SFX_GET_BADGE".to_string()]);
        let cries = BTreeSet::from(["CRY_LUGIA".to_string()]);
        let species = BTreeMap::from([("LUGIA".to_string(), species("LUGIA"))]);
        let cry_by_species = BTreeMap::from([("LUGIA".to_string(), "legacy_cry".to_string())]);

        assert_eq!(
            script_audio_command_issues(
                &command("fallbacksound", Some("SFX_GET_BADGE"), None),
                &music,
                &sfx,
                &cries,
                &species,
                &cry_by_species,
            ),
            vec![ScriptAudioCommandIssue::InvalidCommand]
        );
        assert_eq!(
            script_audio_command_issues(
                &command("playmusic", Some("fallback_music"), None),
                &music,
                &sfx,
                &cries,
                &species,
                &cry_by_species,
            ),
            vec![ScriptAudioCommandIssue::InvalidMusicId]
        );
        assert_eq!(
            script_audio_command_issues(
                &command("cry", Some("LUGIA"), None),
                &music,
                &sfx,
                &cries,
                &species,
                &cry_by_species,
            ),
            vec![ScriptAudioCommandIssue::InvalidCryAsset]
        );

        for (field, value) in [
            ("command", serde_json::json!("fallbacksound")),
            ("audio_id", serde_json::json!("legacy_audio")),
            ("source_script", serde_json::json!("fallback_script")),
        ] {
            let mut payload = serde_json::json!({
                "command": "playmusic",
                "audio_id": "MUSIC_ROUTE_29",
                "fade_frames": null,
                "source_script": "AudioScript",
                "command_index": 7
            });
            payload[field] = value;

            let error = serde_json::from_value::<ScriptAudioCommand>(payload)
                .expect_err("reserved script audio command tokens must fail during JSON load")
                .to_string();

            assert!(
                error.contains("script audio"),
                "{field} produced unexpected error: {error}"
            );
        }
    }

    #[test]
    fn resolves_exact_music_sound_and_wait_cues() {
        let music = BTreeSet::from(["MUSIC_ROUTE_29".to_string()]);
        let sfx = BTreeSet::from(["SFX_GET_BADGE".to_string()]);
        let cries = BTreeSet::new();
        let species = BTreeMap::new();

        assert_eq!(
            resolve_script_audio_command(
                command("playmusic", Some("MUSIC_ROUTE_29"), None),
                &music,
                &sfx,
                &cries,
                &species,
                &BTreeMap::new(),
            )
            .expect("play music"),
            ScriptAudioCue::Play {
                command: "playmusic".to_string(),
                kind: ScriptAudioKind::Music,
                audio_id: "MUSIC_ROUTE_29".to_string(),
                source_script: "AudioScript".to_string(),
                command_index: 7,
            }
        );
        assert!(matches!(
            resolve_script_audio_command(
                command("playsound", Some("SFX_GET_BADGE"), None),
                &music,
                &sfx,
                &cries,
                &species,
                &BTreeMap::new(),
            ),
            Ok(ScriptAudioCue::Play {
                kind: ScriptAudioKind::SoundEffect,
                ..
            })
        ));
        assert_eq!(
            resolve_script_audio_command(
                command("waitsfx", None, None),
                &music,
                &sfx,
                &cries,
                &species,
                &BTreeMap::new(),
            ),
            Ok(ScriptAudioCue::WaitForSoundEffect {
                source_script: "AudioScript".to_string(),
                command_index: 7,
            })
        );
    }

    #[test]
    fn cry_requires_exact_species_and_exact_cry_asset() {
        let species = BTreeMap::from([("LUGIA".to_string(), species("LUGIA"))]);
        let sfx = BTreeSet::from(["SFX_GET_BADGE".to_string()]);
        let cries = BTreeSet::from(["CRY_LUGIA".to_string()]);
        let cry_by_species = BTreeMap::from([("LUGIA".to_string(), "CRY_LUGIA".to_string())]);
        let cue = resolve_script_audio_command(
            command("cry", Some("LUGIA"), None),
            &BTreeSet::new(),
            &BTreeSet::new(),
            &cries,
            &species,
            &cry_by_species,
        )
        .expect("resolve cry");

        assert_eq!(
            cue,
            ScriptAudioCue::Play {
                command: "cry".to_string(),
                kind: ScriptAudioKind::Cry,
                audio_id: "CRY_LUGIA".to_string(),
                source_script: "AudioScript".to_string(),
                command_index: 7,
            }
        );
        assert_eq!(
            resolve_script_audio_command(
                command("cry", Some("LU GIA"), None),
                &BTreeSet::new(),
                &BTreeSet::new(),
                &cries,
                &species,
                &cry_by_species,
            ),
            Err(ScriptAudioError::InvalidCrySpecies {
                species_id: "LU GIA".to_string(),
            })
        );
        assert_eq!(
            resolve_script_audio_command(
                command("PlaySound", Some("SFX_GET_BADGE"), None),
                &BTreeSet::new(),
                &sfx,
                &cries,
                &species,
                &cry_by_species,
            ),
            Err(ScriptAudioError::InvalidCommand {
                command: "PlaySound".to_string(),
            })
        );
        assert_eq!(
            resolve_script_audio_command(
                command("cry", Some("lugia"), None),
                &BTreeSet::new(),
                &BTreeSet::new(),
                &cries,
                &species,
                &cry_by_species,
            ),
            Err(ScriptAudioError::UnknownCrySpecies {
                species_id: "lugia".to_string(),
            })
        );
    }

    #[test]
    fn fade_music_requires_exact_music_and_frame_count() {
        let music = BTreeSet::from(["MUSIC_NEW_BARK_TOWN".to_string()]);
        let cue = resolve_script_audio_command(
            command("musicfadeout", Some("MUSIC_NEW_BARK_TOWN"), Some(16)),
            &music,
            &BTreeSet::new(),
            &BTreeSet::new(),
            &BTreeMap::new(),
            &BTreeMap::new(),
        )
        .expect("fade music");

        assert_eq!(
            cue,
            ScriptAudioCue::FadeMusic {
                audio_id: "MUSIC_NEW_BARK_TOWN".to_string(),
                fade_frames: 16,
                source_script: "AudioScript".to_string(),
                command_index: 7,
            }
        );
    }

    #[test]
    fn applies_audio_cues_to_runtime_state_after_exact_validation() {
        let music = BTreeSet::from(["MUSIC_ROUTE_29".to_string()]);
        let sfx = BTreeSet::from(["SFX_GET_BADGE".to_string()]);
        let species = BTreeMap::from([("LUGIA".to_string(), species("LUGIA"))]);
        let cries = BTreeSet::from(["CRY_LUGIA".to_string()]);
        let cry_by_species = BTreeMap::from([("LUGIA".to_string(), "CRY_LUGIA".to_string())]);
        let mut state = GameState::default();

        apply_script_audio_command(
            &mut state,
            command("playmusic", Some("MUSIC_ROUTE_29"), None),
            &music,
            &sfx,
            &cries,
            &species,
            &cry_by_species,
        )
        .expect("play music");
        apply_script_audio_command(
            &mut state,
            command("playsound", Some("SFX_GET_BADGE"), None),
            &music,
            &sfx,
            &cries,
            &species,
            &cry_by_species,
        )
        .expect("play sfx");
        apply_script_audio_command(
            &mut state,
            command("cry", Some("LUGIA"), None),
            &music,
            &sfx,
            &cries,
            &species,
            &cry_by_species,
        )
        .expect("play cry");

        assert_eq!(
            state.script_runtime.current_music.as_deref(),
            Some("MUSIC_ROUTE_29")
        );
        assert_eq!(state.script_runtime.audio_events.len(), 3);
        assert_eq!(
            state.script_runtime.audio_events[0].kind,
            ScriptAudioRuntimeKind::Music
        );
        assert_eq!(
            state.script_runtime.audio_events[1].kind,
            ScriptAudioRuntimeKind::SoundEffect
        );
        assert_eq!(
            state.script_runtime.audio_events[2].kind,
            ScriptAudioRuntimeKind::Cry
        );
    }

    #[test]
    fn applies_music_fade_and_waitsfx_without_synthesizing_audio_ids() {
        let music = BTreeSet::from(["MUSIC_NEW_BARK_TOWN".to_string()]);
        let mut state = GameState::default();

        apply_script_audio_command(
            &mut state,
            command("musicfadeout", Some("MUSIC_NEW_BARK_TOWN"), Some(16)),
            &music,
            &BTreeSet::new(),
            &BTreeSet::new(),
            &BTreeMap::new(),
            &BTreeMap::new(),
        )
        .expect("fade");
        apply_script_audio_command(
            &mut state,
            command("waitsfx", None, None),
            &music,
            &BTreeSet::new(),
            &BTreeSet::new(),
            &BTreeMap::new(),
            &BTreeMap::new(),
        )
        .expect("wait");

        assert_eq!(
            state.script_runtime.pending_music_fade,
            Some(ScriptMusicFade {
                audio_id: "MUSIC_NEW_BARK_TOWN".to_string(),
                fade_frames: 16,
                source_script: "AudioScript".to_string(),
                command_index: 7,
            })
        );
        assert!(state.script_runtime.waiting_for_sound_effect);
        assert_eq!(state.script_runtime.audio_events[1].audio_id, None);
    }

    #[test]
    fn invalid_audio_command_does_not_mutate_runtime_state() {
        let mut state = GameState::default();
        let error = apply_script_audio_command(
            &mut state,
            command("playmusic", Some("MUSIC ROUTE 29"), None),
            &BTreeSet::from(["MUSIC_ROUTE_29".to_string()]),
            &BTreeSet::new(),
            &BTreeSet::new(),
            &BTreeMap::new(),
            &BTreeMap::new(),
        )
        .expect_err("malformed music id is invalid");

        assert_eq!(
            error,
            ScriptAudioError::InvalidMusic {
                audio_id: "MUSIC ROUTE 29".to_string()
            }
        );
        assert!(state.script_runtime.audio_events.is_empty());
        assert_eq!(state.script_runtime.current_music, None);
    }

    #[test]
    fn invalid_audio_source_script_does_not_mutate_runtime_state() {
        let mut state = GameState::default();
        state.script_runtime.current_music = Some("MUSIC_NEW_BARK_TOWN".to_string());
        let mut bad_source = command("playmusic", Some("MUSIC_ROUTE_29"), None);
        bad_source.source_script = "legacy_audio_script".to_string();

        let error = apply_script_audio_command(
            &mut state,
            bad_source,
            &BTreeSet::from(["MUSIC_ROUTE_29".to_string()]),
            &BTreeSet::new(),
            &BTreeSet::new(),
            &BTreeMap::new(),
            &BTreeMap::new(),
        )
        .expect_err("malformed source script is invalid");

        assert_eq!(
            error,
            ScriptAudioError::InvalidSourceScript {
                source_script: "legacy_audio_script".to_string()
            }
        );
        assert!(state.script_runtime.audio_events.is_empty());
        assert_eq!(
            state.script_runtime.current_music.as_deref(),
            Some("MUSIC_NEW_BARK_TOWN")
        );
        assert_eq!(state.script_runtime.pending_music_fade, None);
    }

    #[test]
    fn script_audio_kind_json_rejects_legacy_alias_payloads() {
        let error =
            serde_json::from_str::<ScriptAudioKind>(r#"{"cry":{"fallback_kind":"sound_effect"}}"#)
                .expect_err("script audio kinds must not accept fallback aliases")
                .to_string();
        assert!(
            error.contains("invalid type") || error.contains("unknown field `fallback_kind`"),
            "{error}"
        );
    }
}
