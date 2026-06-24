use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::state::{
    GameState, ScriptRuntimeAsmDirective, ScriptRuntimeDecorationDescription, ScriptRuntimeDelay,
    ScriptRuntimeEarthquake, ScriptRuntimeEffect, ScriptRuntimeElevatorFloor, ScriptRuntimeEmote,
    ScriptRuntimeNumericBufferWrite, ScriptRuntimeQueuedCommand, ScriptRuntimeStoneTableEntry,
    ScriptRuntimeVariableWrite,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptRuntimeCommand {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptRuntimeInputs {
    pub random_value: Option<u32>,
    pub game_version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScriptRuntimeOutcome {
    EffectRecorded {
        command: String,
        source_script: String,
        command_index: usize,
    },
    ScriptValueSet {
        command: String,
        value: String,
        source_script: String,
        command_index: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
pub enum ScriptRuntimeCommandError {
    #[error("unknown script runtime command '{command}'")]
    UnknownCommand { command: String },
    #[error("script runtime command '{command}' expects {expected} args but found {actual}")]
    WrongArgCount {
        command: String,
        expected: usize,
        actual: usize,
    },
    #[error("script runtime command '{command}' has an empty argument")]
    EmptyArg { command: String },
    #[error("script runtime command '{command}' has a whitespace-padded argument '{arg}'")]
    PaddedArg { command: String, arg: String },
    #[error("script runtime command '{command}' has an unknown numeric token '{token}'")]
    UnknownNumericToken { command: String, token: String },
    #[error("script runtime command '{command}' requires script accumulator")]
    MissingAccumulator { command: String },
    #[error("script runtime command 'random' requires deterministic random input")]
    MissingRandomInput,
    #[error("script runtime command 'random' received value {value} outside upper bound {bound}")]
    RandomInputOutOfRange { value: u32, bound: u32 },
    #[error("script runtime command 'checkver' requires explicit game version input")]
    MissingGameVersion,
    #[error("script runtime command 'pop' cannot pop an empty runtime stack")]
    EmptyStack,
}

pub fn apply_script_runtime_command(
    state: &mut GameState,
    command: ScriptRuntimeCommand,
    inputs: ScriptRuntimeInputs,
) -> Result<ScriptRuntimeOutcome, ScriptRuntimeCommandError> {
    validate_script_runtime_command(&command)?;

    let outcome = match command.command.as_str() {
        "addval" => {
            let left = parse_required_accumulator(state, &command)?;
            let right = parse_i32_token(&command.command, &command.args[0])?;
            set_script_value(state, &command, (left + right).to_string())
        }
        "random" => {
            let bound = parse_u32_token(&command.command, &command.args[0])?;
            let value = inputs
                .random_value
                .ok_or(ScriptRuntimeCommandError::MissingRandomInput)?;
            if bound != 0 && value >= bound {
                return Err(ScriptRuntimeCommandError::RandomInputOutOfRange { value, bound });
            }
            set_script_value(state, &command, value.to_string())
        }
        "checkpoke" => {
            let species_id = &command.args[0];
            let has_species = state
                .party
                .pokemon
                .iter()
                .flatten()
                .any(|pokemon| pokemon.species == *species_id);
            set_script_value(
                state,
                &command,
                if has_species { "TRUE" } else { "FALSE" }.to_string(),
            )
        }
        "checkver" => {
            state.script_runtime.version_check_requested = true;
            let value = inputs
                .game_version
                .ok_or(ScriptRuntimeCommandError::MissingGameVersion)?;
            set_script_value(state, &command, value)
        }
        _ => {
            apply_runtime_effect(state, &command)?;
            ScriptRuntimeOutcome::EffectRecorded {
                command: command.command.clone(),
                source_script: command.source_script.clone(),
                command_index: command.command_index,
            }
        }
    };

    state.script_runtime.effects.push(ScriptRuntimeEffect {
        command: command.command,
        args: command.args,
        source_script: command.source_script,
        command_index: command.command_index,
    });
    Ok(outcome)
}

pub fn validate_script_runtime_command(
    command: &ScriptRuntimeCommand,
) -> Result<(), ScriptRuntimeCommandError> {
    let expected = expected_arg_counts()
        .get(command.command.as_str())
        .copied()
        .ok_or_else(|| ScriptRuntimeCommandError::UnknownCommand {
            command: command.command.clone(),
        })?;
    if command.args.len() != expected {
        return Err(ScriptRuntimeCommandError::WrongArgCount {
            command: command.command.clone(),
            expected,
            actual: command.args.len(),
        });
    }
    for arg in &command.args {
        if arg.is_empty() {
            return Err(ScriptRuntimeCommandError::EmptyArg {
                command: command.command.clone(),
            });
        }
        if arg.trim() != arg {
            return Err(ScriptRuntimeCommandError::PaddedArg {
                command: command.command.clone(),
                arg: arg.clone(),
            });
        }
    }
    Ok(())
}

fn apply_runtime_effect(
    state: &mut GameState,
    command: &ScriptRuntimeCommand,
) -> Result<(), ScriptRuntimeCommandError> {
    match command.command.as_str() {
        "special" => state.script_runtime.last_special_routine = Some(command.args[0].clone()),
        "pause" | "wait" => state
            .script_runtime
            .pending_delays
            .push(ScriptRuntimeDelay {
                command: command.command.clone(),
                frames: parse_u16_token(&command.command, &command.args[0])?,
                source_script: command.source_script.clone(),
                command_index: command.command_index,
            }),
        "earthquake" => {
            let parameter = parse_u16_token(&command.command, &command.args[0])?;
            state
                .script_runtime
                .pending_earthquakes
                .push(ScriptRuntimeEarthquake {
                    parameter,
                    shake_frames: parameter,
                    sleep_frames: parameter & 0x3f,
                    source_script: command.source_script.clone(),
                    command_index: command.command_index,
                });
        }
        "showemote" => state
            .script_runtime
            .pending_emotes
            .push(ScriptRuntimeEmote {
                emote: command.args[0].clone(),
                object: command.args[1].clone(),
                duration: parse_u16_token(&command.command, &command.args[2])?,
                source_script: command.source_script.clone(),
                command_index: command.command_index,
            }),
        "setlasttalked" => state.script_runtime.last_talked_object = Some(command.args[0].clone()),
        "variablesprite" => {
            state
                .script_runtime
                .variable_sprites
                .insert(command.args[0].clone(), command.args[1].clone());
        }
        "gettrainername" | "getitemname" | "getmonname" | "getstring" => {
            state
                .script_runtime
                .named_buffers
                .insert(command.args[0].clone(), command.args[1..].join(" "));
        }
        "loadmenu" => {
            state.script_runtime.active_menu = Some(command.args[0].clone());
            state.script_runtime.window_open = true;
        }
        "verticalmenu" => state.script_runtime.window_open = true,
        "closewindow" => state.script_runtime.window_open = false,
        "menu_coords" => {
            state.script_runtime.menu_coords = Some([
                parse_i16_token(&command.command, &command.args[0])?,
                parse_i16_token(&command.command, &command.args[1])?,
                parse_i16_token(&command.command, &command.args[2])?,
                parse_i16_token(&command.command, &command.args[3])?,
            ]);
        }
        "dontrestartmapmusic" => state.script_runtime.map_music_restart_disabled = true,
        "playmapmusic" => state.script_runtime.map_music_requested = true,
        "faceplayer" | "endifjustbattled" | "jumpstd" => {}
        "itemnotify" => state.script_runtime.item_notify_queued = true,
        "verbosegiveitemvar" => {
            state
                .script_runtime
                .named_buffers
                .insert(command.args[0].clone(), command.args[1].clone());
        }
        "addcellnum" => {
            state
                .script_runtime
                .phone_numbers
                .insert(command.args[0].clone());
        }
        "specialphonecall" => state
            .script_runtime
            .special_phone_calls
            .push(command.args[0].clone()),
        "pokepic" => state.script_runtime.active_pokemon_picture = Some(command.args[0].clone()),
        "closepokepic" => state.script_runtime.active_pokemon_picture = None,
        "trade" => state
            .script_runtime
            .completed_trades
            .push(command.args[0].clone()),
        "catchtutorial" => state
            .script_runtime
            .catch_tutorials
            .push(command.args[0].clone()),
        "warpsound" => state.script_runtime.warp_sound_queued = true,
        "blackoutmod" => state.script_runtime.blackout_mod = Some(command.args[0].clone()),
        "checkscene" => {}
        "battletowertext" => state.script_runtime.battle_tower_text = Some(command.args[0].clone()),
        "halloffame" => state.script_runtime.hall_of_fame_requested = true,
        "credits" => state.script_runtime.credits_requested = true,
        "writevar" => {
            let value = state.script_runtime.script_value.clone().ok_or_else(|| {
                ScriptRuntimeCommandError::MissingAccumulator {
                    command: command.command.clone(),
                }
            })?;
            let target = command.args[0].clone();
            state
                .script_runtime
                .variables
                .insert(target.clone(), value.clone());
            state
                .script_runtime
                .variable_writes
                .push(ScriptRuntimeVariableWrite {
                    target,
                    value,
                    source_script: command.source_script.clone(),
                    command_index: command.command_index,
                });
        }
        "getnum" => {
            let value = state.script_runtime.script_value.clone().ok_or_else(|| {
                ScriptRuntimeCommandError::MissingAccumulator {
                    command: command.command.clone(),
                }
            })?;
            let parsed = parse_u16_token(&command.command, &value)?;
            let rendered = parsed.to_string();
            let target_buffer = command.args[0].clone();
            state
                .script_runtime
                .named_buffers
                .insert(target_buffer.clone(), rendered.clone());
            state
                .script_runtime
                .numeric_buffer_writes
                .push(ScriptRuntimeNumericBufferWrite {
                    target_buffer,
                    value: rendered,
                    width: 3,
                    source_script: command.source_script.clone(),
                    command_index: command.command_index,
                });
        }
        "elevfloor" => {
            state
                .script_runtime
                .elevator_floors
                .push(ScriptRuntimeElevatorFloor {
                    floor: command.args[0].clone(),
                    warp: parse_u16_token(&command.command, &command.args[1])?,
                    target_map: command.args[2].clone(),
                    source_script: command.source_script.clone(),
                    command_index: command.command_index,
                });
        }
        "stonetable" => {
            state
                .script_runtime
                .stone_table_entries
                .push(ScriptRuntimeStoneTableEntry {
                    warp: parse_u16_token(&command.command, &command.args[0])?,
                    object_event: command.args[1].clone(),
                    script: command.args[2].clone(),
                    source_script: command.source_script.clone(),
                    command_index: command.command_index,
                });
        }
        "dw" | "ldh" | "ld" | "dn" | "dba" | "dbw" => {
            state
                .script_runtime
                .asm_directives
                .push(ScriptRuntimeAsmDirective {
                    command: command.command.clone(),
                    args: command.args.clone(),
                    source_script: command.source_script.clone(),
                    command_index: command.command_index,
                })
        }
        "describedecoration" => {
            state
                .script_runtime
                .decoration_descriptions
                .push(ScriptRuntimeDecorationDescription {
                    decoration: command.args[0].clone(),
                    source_script: command.source_script.clone(),
                    command_index: command.command_index,
                })
        }
        "cmdqueue" => state
            .script_runtime
            .command_queue
            .push(ScriptRuntimeQueuedCommand {
                command: command.command.clone(),
                bank: Some(command.args[0].clone()),
                target: command.args[1].clone(),
                source_script: command.source_script.clone(),
                command_index: command.command_index,
            }),
        "writecmdqueue" | "elevator" | "callasm" | "checkpokemail" | "givepokemail" => {
            if command.command == "checkpokemail" {
                state
                    .script_runtime
                    .checked_mail_targets
                    .push(command.args[0].clone());
            }
            if command.command == "givepokemail" {
                state
                    .script_runtime
                    .given_mail_targets
                    .push(command.args[0].clone());
            }
            state
                .script_runtime
                .command_queue
                .push(ScriptRuntimeQueuedCommand {
                    command: command.command.clone(),
                    bank: None,
                    target: command.args[0].clone(),
                    source_script: command.source_script.clone(),
                    command_index: command.command_index,
                });
        }
        "conditional_event" => {
            state
                .script_runtime
                .command_queue
                .push(ScriptRuntimeQueuedCommand {
                    command: command.command.clone(),
                    bank: Some(command.args[0].clone()),
                    target: command.args[1].clone(),
                    source_script: command.source_script.clone(),
                    command_index: command.command_index,
                })
        }
        "push" => state.script_runtime.stack.push(command.args[0].clone()),
        "pop" => {
            let value = state
                .script_runtime
                .stack
                .pop()
                .ok_or(ScriptRuntimeCommandError::EmptyStack)?;
            state
                .script_runtime
                .named_buffers
                .insert(command.args[0].clone(), value);
        }
        "ret" => {}
        "teleport_from" => state.script_runtime.teleport_from_queued = true,
        "_2dmenu" => state.script_runtime.menu_2d_requested = true,
        other => {
            return Err(ScriptRuntimeCommandError::UnknownCommand {
                command: other.to_string(),
            });
        }
    }
    Ok(())
}

fn set_script_value(
    state: &mut GameState,
    command: &ScriptRuntimeCommand,
    value: String,
) -> ScriptRuntimeOutcome {
    state.script_runtime.script_value = Some(value.clone());
    ScriptRuntimeOutcome::ScriptValueSet {
        command: command.command.clone(),
        value,
        source_script: command.source_script.clone(),
        command_index: command.command_index,
    }
}

fn parse_required_accumulator(
    state: &GameState,
    command: &ScriptRuntimeCommand,
) -> Result<i32, ScriptRuntimeCommandError> {
    let value = state
        .script_runtime
        .script_value
        .as_deref()
        .ok_or_else(|| ScriptRuntimeCommandError::MissingAccumulator {
            command: command.command.clone(),
        })?;
    parse_i32_token(&command.command, value)
}

fn parse_i16_token(command: &str, token: &str) -> Result<i16, ScriptRuntimeCommandError> {
    let value = parse_i32_token(command, token)?;
    i16::try_from(value).map_err(|_| ScriptRuntimeCommandError::UnknownNumericToken {
        command: command.to_string(),
        token: token.to_string(),
    })
}

fn parse_u16_token(command: &str, token: &str) -> Result<u16, ScriptRuntimeCommandError> {
    let value = parse_i32_token(command, token)?;
    u16::try_from(value).map_err(|_| ScriptRuntimeCommandError::UnknownNumericToken {
        command: command.to_string(),
        token: token.to_string(),
    })
}

fn parse_u32_token(command: &str, token: &str) -> Result<u32, ScriptRuntimeCommandError> {
    let value = parse_i32_token(command, token)?;
    u32::try_from(value).map_err(|_| ScriptRuntimeCommandError::UnknownNumericToken {
        command: command.to_string(),
        token: token.to_string(),
    })
}

fn parse_i32_token(command: &str, token: &str) -> Result<i32, ScriptRuntimeCommandError> {
    if token.trim() != token || token.is_empty() {
        return Err(ScriptRuntimeCommandError::UnknownNumericToken {
            command: command.to_string(),
            token: token.to_string(),
        });
    }
    let (sign, raw) = match token.as_bytes()[0] {
        b'-' => (-1, &token[1..]),
        b'+' => (1, &token[1..]),
        _ => (1, token),
    };
    let (radix, digits) = if let Some(hex) = raw.strip_prefix('$') {
        (16, hex)
    } else if let Some(binary) = raw.strip_prefix('%') {
        (2, binary)
    } else {
        (10, raw)
    };
    if digits.is_empty() {
        return Err(ScriptRuntimeCommandError::UnknownNumericToken {
            command: command.to_string(),
            token: token.to_string(),
        });
    }
    i32::from_str_radix(digits, radix)
        .map(|value| value * sign)
        .map_err(|_| ScriptRuntimeCommandError::UnknownNumericToken {
            command: command.to_string(),
            token: token.to_string(),
        })
}

fn expected_arg_counts() -> BTreeMap<&'static str, usize> {
    BTreeMap::from([
        ("special", 1),
        ("pause", 1),
        ("earthquake", 1),
        ("showemote", 3),
        ("setlasttalked", 1),
        ("variablesprite", 2),
        ("gettrainername", 3),
        ("getitemname", 2),
        ("getmonname", 2),
        ("loadmenu", 1),
        ("verticalmenu", 0),
        ("closewindow", 0),
        ("menu_coords", 4),
        ("dontrestartmapmusic", 0),
        ("playmapmusic", 0),
        ("faceplayer", 0),
        ("endifjustbattled", 0),
        ("jumpstd", 1),
        ("itemnotify", 0),
        ("addval", 1),
        ("verbosegiveitemvar", 2),
        ("getstring", 2),
        ("addcellnum", 1),
        ("specialphonecall", 1),
        ("checkpoke", 1),
        ("pokepic", 1),
        ("closepokepic", 0),
        ("trade", 1),
        ("catchtutorial", 1),
        ("warpsound", 0),
        ("blackoutmod", 1),
        ("wait", 1),
        ("random", 1),
        ("checkscene", 0),
        ("battletowertext", 1),
        ("halloffame", 0),
        ("credits", 0),
        ("dw", 1),
        ("stonetable", 3),
        ("elevfloor", 3),
        ("ldh", 2),
        ("ld", 2),
        ("describedecoration", 1),
        ("cmdqueue", 2),
        ("conditional_event", 2),
        ("push", 1),
        ("pop", 1),
        ("ret", 0),
        ("checkver", 0),
        ("writecmdqueue", 1),
        ("elevator", 1),
        ("teleport_from", 0),
        ("_2dmenu", 0),
        ("dn", 2),
        ("dba", 1),
        ("dbw", 2),
        ("writevar", 1),
        ("checkpokemail", 1),
        ("getnum", 1),
        ("callasm", 1),
        ("givepokemail", 1),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::PartyPokemonRef;

    fn command(name: &str, args: &[&str]) -> ScriptRuntimeCommand {
        ScriptRuntimeCommand {
            command: name.to_string(),
            args: args.iter().map(|arg| (*arg).to_string()).collect(),
            source_script: "RuntimeScript".to_string(),
            command_index: 4,
        }
    }

    #[test]
    fn records_exact_runtime_effects_without_command_enums() {
        let mut state = GameState::default();
        apply_script_runtime_command(
            &mut state,
            command("special", &["HealParty"]),
            default_inputs(),
        )
        .expect("special");
        apply_script_runtime_command(
            &mut state,
            command("variablesprite", &["SPRITE_WEIRD_TREE", "SPRITE_SUDOWOODO"]),
            default_inputs(),
        )
        .expect("variablesprite");
        apply_script_runtime_command(
            &mut state,
            command("addcellnum", &["PHONE_YOUNGSTER_JOE"]),
            default_inputs(),
        )
        .expect("addcellnum");

        assert_eq!(
            state.script_runtime.last_special_routine.as_deref(),
            Some("HealParty")
        );
        assert_eq!(
            state.script_runtime.variable_sprites["SPRITE_WEIRD_TREE"],
            "SPRITE_SUDOWOODO"
        );
        assert!(
            state
                .script_runtime
                .phone_numbers
                .contains("PHONE_YOUNGSTER_JOE")
        );
        assert_eq!(state.script_runtime.effects.len(), 3);
        assert_eq!(state.script_runtime.effects[0].command, "special");
    }

    #[test]
    fn numeric_commands_set_script_value_from_exact_inputs() {
        let mut state = GameState::default();
        state.script_runtime.script_value = Some("$10".to_string());
        assert_eq!(
            apply_script_runtime_command(&mut state, command("addval", &["-1"]), default_inputs())
                .expect("addval"),
            ScriptRuntimeOutcome::ScriptValueSet {
                command: "addval".to_string(),
                value: "15".to_string(),
                source_script: "RuntimeScript".to_string(),
                command_index: 4,
            }
        );

        apply_script_runtime_command(
            &mut state,
            command("random", &["10"]),
            ScriptRuntimeInputs {
                random_value: Some(7),
                game_version: None,
            },
        )
        .expect("random");
        assert_eq!(state.script_runtime.script_value.as_deref(), Some("7"));

        assert!(matches!(
            apply_script_runtime_command(
                &mut state,
                command("random", &["10"]),
                ScriptRuntimeInputs {
                    random_value: Some(10),
                    game_version: None,
                },
            ),
            Err(ScriptRuntimeCommandError::RandomInputOutOfRange { .. })
        ));
    }

    #[test]
    fn checkpoke_and_checkver_are_exact_state_queries() {
        let mut state = GameState::default();
        state.party.pokemon[0] = Some(PartyPokemonRef {
            species: "CYNDAQUIL".to_string(),
            level: 5,
        });

        apply_script_runtime_command(
            &mut state,
            command("checkpoke", &["CYNDAQUIL"]),
            default_inputs(),
        )
        .expect("checkpoke");
        assert_eq!(state.script_runtime.script_value.as_deref(), Some("TRUE"));

        apply_script_runtime_command(
            &mut state,
            command("checkpoke", &["cyndaquil"]),
            default_inputs(),
        )
        .expect("case changed checkpoke");
        assert_eq!(state.script_runtime.script_value.as_deref(), Some("FALSE"));

        apply_script_runtime_command(
            &mut state,
            command("checkver", &[]),
            ScriptRuntimeInputs {
                random_value: None,
                game_version: Some("CRYSTAL".to_string()),
            },
        )
        .expect("checkver");
        assert_eq!(
            state.script_runtime.script_value.as_deref(),
            Some("CRYSTAL")
        );
        assert!(state.script_runtime.version_check_requested);
    }

    #[test]
    fn earthquake_records_exact_generated_player_movement_timing() {
        let mut state = GameState::default();

        apply_script_runtime_command(&mut state, command("earthquake", &["84"]), default_inputs())
            .expect("earthquake");

        assert_eq!(state.script_runtime.pending_delays, Vec::new());
        assert_eq!(state.script_runtime.pending_earthquakes.len(), 1);
        let earthquake = &state.script_runtime.pending_earthquakes[0];
        assert_eq!(earthquake.parameter, 84);
        assert_eq!(earthquake.shake_frames, 84);
        assert_eq!(earthquake.sleep_frames, 84 & 0x3f);
        assert_eq!(earthquake.source_script, "RuntimeScript");
        assert_eq!(earthquake.command_index, 4);
        assert_eq!(state.script_runtime.effects[0].command, "earthquake");
    }

    #[test]
    fn writevar_writes_exact_accumulator_to_target_variable() {
        let mut state = GameState::default();
        state.script_runtime.script_value = Some("12".to_string());

        apply_script_runtime_command(
            &mut state,
            command("writevar", &["VAR_BLUECARDBALANCE"]),
            default_inputs(),
        )
        .expect("writevar");

        assert_eq!(
            state
                .script_runtime
                .variables
                .get("VAR_BLUECARDBALANCE")
                .map(String::as_str),
            Some("12")
        );
        assert_eq!(state.script_runtime.variable_writes.len(), 1);
        let write = &state.script_runtime.variable_writes[0];
        assert_eq!(write.target, "VAR_BLUECARDBALANCE");
        assert_eq!(write.value, "12");
        assert_eq!(write.source_script, "RuntimeScript");
        assert_eq!(write.command_index, 4);
        assert!(!state.script_runtime.named_buffers.contains_key("writevar"));
    }

    #[test]
    fn writevar_requires_exact_script_accumulator() {
        let mut state = GameState::default();

        assert!(matches!(
            apply_script_runtime_command(
                &mut state,
                command("writevar", &["VAR_BLUECARDBALANCE"]),
                default_inputs(),
            ),
            Err(ScriptRuntimeCommandError::MissingAccumulator { .. })
        ));
        assert!(state.script_runtime.variables.is_empty());
        assert!(state.script_runtime.variable_writes.is_empty());
        assert!(state.script_runtime.effects.is_empty());
    }

    #[test]
    fn getnum_writes_exact_numeric_accumulator_to_target_buffer() {
        let mut state = GameState::default();
        state.script_runtime.script_value = Some("37".to_string());

        apply_script_runtime_command(
            &mut state,
            command("getnum", &["STRING_BUFFER_3"]),
            default_inputs(),
        )
        .expect("getnum");

        assert_eq!(
            state
                .script_runtime
                .named_buffers
                .get("STRING_BUFFER_3")
                .map(String::as_str),
            Some("37")
        );
        assert_eq!(state.script_runtime.numeric_buffer_writes.len(), 1);
        let write = &state.script_runtime.numeric_buffer_writes[0];
        assert_eq!(write.target_buffer, "STRING_BUFFER_3");
        assert_eq!(write.value, "37");
        assert_eq!(write.width, 3);
        assert_eq!(write.source_script, "RuntimeScript");
        assert_eq!(write.command_index, 4);
        assert!(!state.script_runtime.named_buffers.contains_key("getnum"));
    }

    #[test]
    fn getnum_requires_numeric_script_accumulator() {
        let mut state = GameState::default();

        assert!(matches!(
            apply_script_runtime_command(
                &mut state,
                command("getnum", &["STRING_BUFFER_3"]),
                default_inputs(),
            ),
            Err(ScriptRuntimeCommandError::MissingAccumulator { .. })
        ));
        state.script_runtime.script_value = Some("BUGS".to_string());
        assert!(matches!(
            apply_script_runtime_command(
                &mut state,
                command("getnum", &["STRING_BUFFER_3"]),
                default_inputs(),
            ),
            Err(ScriptRuntimeCommandError::UnknownNumericToken { .. })
        ));
        assert!(state.script_runtime.numeric_buffer_writes.is_empty());
        assert!(state.script_runtime.effects.is_empty());
    }

    #[test]
    fn elevfloor_records_exact_floor_warp_and_target_map() {
        let mut state = GameState::default();

        apply_script_runtime_command(
            &mut state,
            command("elevfloor", &["FLOOR_1F", "4", "CELADON_DEPT_STORE_1F"]),
            default_inputs(),
        )
        .expect("elevfloor");

        assert_eq!(state.script_runtime.elevator_floors.len(), 1);
        let floor = &state.script_runtime.elevator_floors[0];
        assert_eq!(floor.floor, "FLOOR_1F");
        assert_eq!(floor.warp, 4);
        assert_eq!(floor.target_map, "CELADON_DEPT_STORE_1F");
        assert_eq!(floor.source_script, "RuntimeScript");
        assert_eq!(floor.command_index, 4);
        assert!(!state.script_runtime.named_buffers.contains_key("elevfloor"));
        assert_eq!(state.script_runtime.effects[0].command, "elevfloor");
    }

    #[test]
    fn elevfloor_requires_exact_numeric_warp_token() {
        let mut state = GameState::default();

        assert!(matches!(
            apply_script_runtime_command(
                &mut state,
                command(
                    "elevfloor",
                    &["FLOOR_1F", "WARP_4", "CELADON_DEPT_STORE_1F"]
                ),
                default_inputs(),
            ),
            Err(ScriptRuntimeCommandError::UnknownNumericToken { .. })
        ));
        assert!(state.script_runtime.elevator_floors.is_empty());
        assert!(state.script_runtime.effects.is_empty());
    }

    #[test]
    fn stonetable_records_exact_warp_object_and_script() {
        let mut state = GameState::default();

        apply_script_runtime_command(
            &mut state,
            command(
                "stonetable",
                &["5", "BLACKTHORNGYM2F_BOULDER1", ".Boulder1"],
            ),
            default_inputs(),
        )
        .expect("stonetable");

        assert_eq!(state.script_runtime.stone_table_entries.len(), 1);
        let entry = &state.script_runtime.stone_table_entries[0];
        assert_eq!(entry.warp, 5);
        assert_eq!(entry.object_event, "BLACKTHORNGYM2F_BOULDER1");
        assert_eq!(entry.script, ".Boulder1");
        assert_eq!(entry.source_script, "RuntimeScript");
        assert_eq!(entry.command_index, 4);
        assert!(
            !state
                .script_runtime
                .named_buffers
                .contains_key("stonetable")
        );
        assert_eq!(state.script_runtime.effects[0].command, "stonetable");
    }

    #[test]
    fn stonetable_requires_exact_numeric_warp_token() {
        let mut state = GameState::default();

        assert!(matches!(
            apply_script_runtime_command(
                &mut state,
                command(
                    "stonetable",
                    &["WARP_5", "BLACKTHORNGYM2F_BOULDER1", ".Boulder1"]
                ),
                default_inputs(),
            ),
            Err(ScriptRuntimeCommandError::UnknownNumericToken { .. })
        ));
        assert!(state.script_runtime.stone_table_entries.is_empty());
        assert!(state.script_runtime.effects.is_empty());
    }

    #[test]
    fn describedecoration_records_each_exact_selector() {
        let mut state = GameState::default();

        apply_script_runtime_command(
            &mut state,
            command("describedecoration", &["DECODESC_LEFT_DOLL"]),
            default_inputs(),
        )
        .expect("left doll describedecoration");
        apply_script_runtime_command(
            &mut state,
            command("describedecoration", &["DECODESC_POSTER"]),
            default_inputs(),
        )
        .expect("poster describedecoration");

        assert_eq!(state.script_runtime.decoration_descriptions.len(), 2);
        assert_eq!(
            state.script_runtime.decoration_descriptions[0].decoration,
            "DECODESC_LEFT_DOLL"
        );
        assert_eq!(
            state.script_runtime.decoration_descriptions[1].decoration,
            "DECODESC_POSTER"
        );
        assert_eq!(
            state.script_runtime.decoration_descriptions[0].source_script,
            "RuntimeScript"
        );
        assert_eq!(
            state.script_runtime.decoration_descriptions[0].command_index,
            4
        );
        assert!(
            !state
                .script_runtime
                .named_buffers
                .contains_key("describedecoration")
        );
        assert_eq!(
            state.script_runtime.effects[1].command,
            "describedecoration"
        );
    }

    #[test]
    fn asm_directives_preserve_repeated_exact_entries_in_order() {
        let mut state = GameState::default();

        apply_script_runtime_command(&mut state, command("dw", &[".MenuData"]), default_inputs())
            .expect("first dw");
        apply_script_runtime_command(
            &mut state,
            command("dw", &[".OtherMenuData"]),
            default_inputs(),
        )
        .expect("second dw");
        apply_script_runtime_command(
            &mut state,
            command("ldh", &["a", "[rWBK]"]),
            default_inputs(),
        )
        .expect("ldh");

        assert_eq!(state.script_runtime.asm_directives.len(), 3);
        assert_eq!(state.script_runtime.asm_directives[0].command, "dw");
        assert_eq!(
            state.script_runtime.asm_directives[0].args,
            vec![".MenuData".to_string()]
        );
        assert_eq!(state.script_runtime.asm_directives[1].command, "dw");
        assert_eq!(
            state.script_runtime.asm_directives[1].args,
            vec![".OtherMenuData".to_string()]
        );
        assert_eq!(state.script_runtime.asm_directives[2].command, "ldh");
        assert_eq!(
            state.script_runtime.asm_directives[2].args,
            vec!["a".to_string(), "[rWBK]".to_string()]
        );
        assert_eq!(
            state.script_runtime.asm_directives[2].source_script,
            "RuntimeScript"
        );
        assert_eq!(state.script_runtime.asm_directives[2].command_index, 4);
        assert!(!state.script_runtime.named_buffers.contains_key("dw"));
        assert!(!state.script_runtime.named_buffers.contains_key("ldh"));
    }

    #[test]
    fn rejects_unknown_padded_or_malformed_runtime_commands() {
        let mut state = GameState::default();
        assert!(matches!(
            apply_script_runtime_command(
                &mut state,
                command("Special", &["HealParty"]),
                default_inputs()
            ),
            Err(ScriptRuntimeCommandError::UnknownCommand { .. })
        ));
        assert!(matches!(
            apply_script_runtime_command(
                &mut state,
                command("special", &[" HealParty"]),
                default_inputs()
            ),
            Err(ScriptRuntimeCommandError::PaddedArg { .. })
        ));
        assert!(matches!(
            apply_script_runtime_command(
                &mut state,
                command("pause", &["FOREVER"]),
                default_inputs()
            ),
            Err(ScriptRuntimeCommandError::UnknownNumericToken { .. })
        ));
        assert!(matches!(
            apply_script_runtime_command(
                &mut state,
                command("pop", &["wScriptVar"]),
                default_inputs()
            ),
            Err(ScriptRuntimeCommandError::EmptyStack)
        ));
    }

    fn default_inputs() -> ScriptRuntimeInputs {
        ScriptRuntimeInputs::default()
    }
}
