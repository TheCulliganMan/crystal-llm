use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::models::PokemonSpecies;
use crate::state::{GameState, ScriptAudioRuntimeEvent, ScriptAudioRuntimeKind, ScriptMusicFade};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptAudioCommand {
    pub command: String,
    pub audio_id: Option<String>,
    pub fade_frames: Option<u16>,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScriptAudioKind {
    Music,
    SoundEffect,
    Cry,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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
pub enum ScriptAudioError {
    UnknownCommand { command: String },
    MissingAudioId { command: String },
    UnexpectedAudioId { command: String },
    MissingFadeFrames { command: String },
    UnexpectedFadeFrames { command: String },
    UnknownMusic { audio_id: String },
    UnknownSoundEffect { audio_id: String },
    UnknownCrySpecies { species_id: String },
    MissingCryMetadata { species_id: String },
    UnknownCryAsset { audio_id: String },
}

pub fn resolve_script_audio_command(
    command: ScriptAudioCommand,
    music_ids: &BTreeSet<String>,
    sound_effect_ids: &BTreeSet<String>,
    cry_ids: &BTreeSet<String>,
    species: &BTreeMap<String, PokemonSpecies>,
    cry_by_species: &BTreeMap<String, String>,
) -> Result<ScriptAudioCue, ScriptAudioError> {
    match command.command.as_str() {
        "playmusic" => {
            reject_fade_frames(&command)?;
            let audio_id = require_audio_id(&command)?.to_string();
            if !music_ids.contains(&audio_id) {
                return Err(ScriptAudioError::UnknownMusic { audio_id });
            }
            Ok(play_cue(command, ScriptAudioKind::Music, audio_id))
        }
        "playsound" => {
            reject_fade_frames(&command)?;
            let audio_id = require_audio_id(&command)?.to_string();
            if !sound_effect_ids.contains(&audio_id) {
                return Err(ScriptAudioError::UnknownSoundEffect { audio_id });
            }
            Ok(play_cue(command, ScriptAudioKind::SoundEffect, audio_id))
        }
        "cry" => {
            reject_fade_frames(&command)?;
            let species_id = require_audio_id(&command)?.to_string();
            if !species.contains_key(&species_id) {
                return Err(ScriptAudioError::UnknownCrySpecies { species_id });
            }
            let audio_id = cry_by_species.get(&species_id).cloned().ok_or_else(|| {
                ScriptAudioError::MissingCryMetadata {
                    species_id: species_id.clone(),
                }
            })?;
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
            command("playmusic", Some("music_route_29"), None),
            &BTreeSet::from(["MUSIC_ROUTE_29".to_string()]),
            &BTreeSet::new(),
            &BTreeSet::new(),
            &BTreeMap::new(),
            &BTreeMap::new(),
        )
        .expect_err("case-changed music is invalid");

        assert_eq!(
            error,
            ScriptAudioError::UnknownMusic {
                audio_id: "music_route_29".to_string()
            }
        );
        assert!(state.script_runtime.audio_events.is_empty());
        assert_eq!(state.script_runtime.current_music, None);
    }
}
