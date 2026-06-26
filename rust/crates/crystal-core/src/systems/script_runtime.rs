use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

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

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StoryEventScriptConstants {
    pub global: BTreeMap<String, i64>,
    pub maps: BTreeMap<String, BTreeMap<String, i64>>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InitializeEventsConfig {
    pub event_flags: Vec<String>,
    pub engine_flags: Vec<String>,
    pub variable_sprites: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InitializeEventsIssue {
    InvalidFlag { flag: String },
    InvalidVariableSprite { sprite: String },
}

pub fn initialize_events_issues(config: &InitializeEventsConfig) -> Vec<InitializeEventsIssue> {
    let mut issues = Vec::new();

    for flag in config.event_flags.iter().chain(config.engine_flags.iter()) {
        if !is_exact_nonempty_runtime_token(flag) {
            issues.push(InitializeEventsIssue::InvalidFlag { flag: flag.clone() });
        }
    }
    for (sprite, replacement) in &config.variable_sprites {
        if !is_exact_nonempty_runtime_token(sprite) || !is_exact_nonempty_runtime_token(replacement)
        {
            issues.push(InitializeEventsIssue::InvalidVariableSprite {
                sprite: sprite.clone(),
            });
        }
    }

    issues
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoryEventScriptConstantIssue {
    InvalidGlobalConstant { key: String },
    InvalidMap { map_name: String },
    InvalidMapConstant { map_name: String, key: String },
}

pub fn story_event_script_constant_issues(
    constants: &StoryEventScriptConstants,
) -> Vec<StoryEventScriptConstantIssue> {
    let mut issues = Vec::new();

    for key in constants.global.keys() {
        if !is_exact_nonempty_runtime_token(key) {
            issues.push(StoryEventScriptConstantIssue::InvalidGlobalConstant { key: key.clone() });
        }
    }
    for (map_name, constants) in &constants.maps {
        if !is_exact_nonempty_runtime_token(map_name) {
            issues.push(StoryEventScriptConstantIssue::InvalidMap {
                map_name: map_name.clone(),
            });
        }
        for key in constants.keys() {
            if !is_exact_nonempty_runtime_token(key) {
                issues.push(StoryEventScriptConstantIssue::InvalidMapConstant {
                    map_name: map_name.clone(),
                    key: key.clone(),
                });
            }
        }
    }

    issues
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
    #[error("script runtime command name is empty")]
    EmptyCommand,
    #[error("script runtime command name is whitespace-padded '{command}'")]
    PaddedCommand { command: String },
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

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptRuntimeReferenceCatalog {
    pub special_routines: BTreeSet<String>,
    pub trainer_classes: BTreeMap<String, String>,
    pub items: BTreeSet<String>,
    pub pokemon: BTreeSet<String>,
    pub phone_contacts: BTreeSet<String>,
    pub special_phone_calls: BTreeSet<String>,
    pub npc_trades: BTreeSet<String>,
    pub script_labels: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScriptRuntimeCommandIssue {
    InvalidCommand {
        error: ScriptRuntimeCommandError,
    },
    UnknownSpecialRoutine {
        special_id: String,
    },
    UnknownTrainer {
        trainer_id: String,
    },
    TrainerClassMismatch {
        trainer_id: String,
        expected_class: String,
        actual_class: String,
    },
    UnknownItem {
        item_id: String,
    },
    UnknownSpecies {
        species_id: String,
    },
    UnknownPhoneContact {
        contact_id: String,
    },
    UnknownSpecialPhoneCall {
        call_id: String,
    },
    UnknownNpcTrade {
        trade_id: String,
    },
    UnknownTarget {
        target_label: String,
    },
}

pub const SCRIPT_RUNTIME_USE_SCRIPT_VAR_ID: &str = "USE_SCRIPT_VAR";
pub const SCRIPT_RUNTIME_ITEM_FROM_MEMORY_ID: &str = "ITEM_FROM_MEM";
pub const SCRIPT_RUNTIME_CURRENT_BANK_TARGET: &str = "BANK(@)";

fn is_exact_nonempty_runtime_token(value: &str) -> bool {
    !value.is_empty() && value.trim() == value
}

pub fn script_runtime_command_issues(
    command: &ScriptRuntimeCommand,
    catalog: &ScriptRuntimeReferenceCatalog,
) -> Vec<ScriptRuntimeCommandIssue> {
    let mut issues = Vec::new();
    if let Err(error) = validate_script_runtime_command(command) {
        issues.push(ScriptRuntimeCommandIssue::InvalidCommand { error });
        return issues;
    }
    match command.command.as_str() {
        "special" => {
            let special_id = &command.args[0];
            if !catalog.special_routines.contains(special_id) {
                issues.push(ScriptRuntimeCommandIssue::UnknownSpecialRoutine {
                    special_id: special_id.clone(),
                });
            }
        }
        "gettrainername" => {
            let trainer_class = &command.args[1];
            let trainer_id = &command.args[2];
            match catalog.trainer_classes.get(trainer_id) {
                Some(actual_class) if actual_class == trainer_class => {}
                Some(actual_class) => {
                    issues.push(ScriptRuntimeCommandIssue::TrainerClassMismatch {
                        trainer_id: trainer_id.clone(),
                        expected_class: trainer_class.clone(),
                        actual_class: actual_class.clone(),
                    })
                }
                None => issues.push(ScriptRuntimeCommandIssue::UnknownTrainer {
                    trainer_id: trainer_id.clone(),
                }),
            }
        }
        "getitemname" => {
            let item_id = &command.args[1];
            if item_id != SCRIPT_RUNTIME_USE_SCRIPT_VAR_ID
                && item_id != SCRIPT_RUNTIME_ITEM_FROM_MEMORY_ID
                && !catalog.items.contains(item_id)
            {
                issues.push(ScriptRuntimeCommandIssue::UnknownItem {
                    item_id: item_id.clone(),
                });
            }
        }
        "getmonname" => {
            let species_id = &command.args[1];
            if species_id != SCRIPT_RUNTIME_USE_SCRIPT_VAR_ID
                && !catalog.pokemon.contains(species_id)
            {
                issues.push(ScriptRuntimeCommandIssue::UnknownSpecies {
                    species_id: species_id.clone(),
                });
            }
        }
        "addcellnum" => {
            let contact_id = &command.args[0];
            if !catalog.phone_contacts.contains(contact_id) {
                issues.push(ScriptRuntimeCommandIssue::UnknownPhoneContact {
                    contact_id: contact_id.clone(),
                });
            }
        }
        "specialphonecall" => {
            let call_id = &command.args[0];
            if !catalog.special_phone_calls.contains(call_id) {
                issues.push(ScriptRuntimeCommandIssue::UnknownSpecialPhoneCall {
                    call_id: call_id.clone(),
                });
            }
        }
        "checkpoke" | "pokepic" => {
            let species_id = &command.args[0];
            if !catalog.pokemon.contains(species_id) {
                issues.push(ScriptRuntimeCommandIssue::UnknownSpecies {
                    species_id: species_id.clone(),
                });
            }
        }
        "trade" => {
            let trade_id = &command.args[0];
            if !catalog.npc_trades.contains(trade_id) {
                issues.push(ScriptRuntimeCommandIssue::UnknownNpcTrade {
                    trade_id: trade_id.clone(),
                });
            }
        }
        "cmdqueue" | "writecmdqueue" | "elevator" | "callasm" | "dba" | "dw" | "checkpokemail"
        | "givepokemail" => {
            let target_label = if command.command == "cmdqueue" {
                &command.args[1]
            } else {
                &command.args[0]
            };
            push_unknown_runtime_target_issue(command, target_label, catalog, &mut issues);
        }
        "stonetable" => {
            push_unknown_runtime_target_issue(command, &command.args[2], catalog, &mut issues);
        }
        "conditional_event" => {
            push_unknown_runtime_target_issue(command, &command.args[1], catalog, &mut issues);
        }
        _ => {}
    }
    issues
}

fn push_unknown_runtime_target_issue(
    command: &ScriptRuntimeCommand,
    target_label: &str,
    catalog: &ScriptRuntimeReferenceCatalog,
    issues: &mut Vec<ScriptRuntimeCommandIssue>,
) {
    if target_label != SCRIPT_RUNTIME_CURRENT_BANK_TARGET
        && resolve_script_runtime_target_label(
            &catalog.script_labels,
            &command.source_script,
            target_label,
        )
        .is_none()
    {
        issues.push(ScriptRuntimeCommandIssue::UnknownTarget {
            target_label: target_label.to_string(),
        });
    }
}

pub fn resolve_script_runtime_target_label(
    script_labels: &BTreeSet<String>,
    source_script: &str,
    target_label: &str,
) -> Option<String> {
    if script_labels.contains(target_label) {
        return Some(target_label.to_string());
    }
    if target_label.starts_with('.') {
        let parent_script = script_label_parent(source_script);
        let local = format!("{target_label}@{parent_script}");
        if script_labels.contains(&local) {
            return Some(local);
        }
    }
    None
}

pub fn script_label_parent(source_script: &str) -> &str {
    source_script
        .rsplit_once('@')
        .map(|(_, parent)| parent)
        .unwrap_or(source_script)
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
    if command.command.is_empty() {
        return Err(ScriptRuntimeCommandError::EmptyCommand);
    }
    if command.command.trim() != command.command {
        return Err(ScriptRuntimeCommandError::PaddedCommand {
            command: command.command.clone(),
        });
    }
    let expected = script_runtime_command_arg_counts()
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

pub fn script_runtime_command_arg_counts() -> BTreeMap<&'static str, usize> {
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
    fn exported_runtime_command_arity_table_is_validation_source() {
        let counts = script_runtime_command_arg_counts();
        assert_eq!(counts.get("special"), Some(&1));
        assert_eq!(counts.get("checkver"), Some(&0));
        assert_eq!(counts.get("givepokemail"), Some(&1));
        assert!(!counts.contains_key("SPECIAL"));

        assert_eq!(
            validate_script_runtime_command(&command("checkver", &[])),
            Ok(())
        );
        assert_eq!(
            validate_script_runtime_command(&command("checkver", &["EXTRA"])),
            Err(ScriptRuntimeCommandError::WrongArgCount {
                command: "checkver".to_string(),
                expected: 0,
                actual: 1,
            })
        );
    }

    #[test]
    fn command_issues_validate_exact_runtime_references() {
        let catalog = ScriptRuntimeReferenceCatalog {
            special_routines: BTreeSet::from(["FadeOutMusic".to_string()]),
            trainer_classes: BTreeMap::from([("FALKNER1".to_string(), "FALKNER".to_string())]),
            items: BTreeSet::from(["POTION".to_string()]),
            pokemon: BTreeSet::from(["PIKACHU".to_string()]),
            phone_contacts: BTreeSet::from(["PHONE_ELM".to_string()]),
            special_phone_calls: BTreeSet::from(["SPECIALCALL_MASTERBALL".to_string()]),
            npc_trades: BTreeSet::from(["NPC_TRADE_MIKE".to_string()]),
            script_labels: BTreeSet::from([
                "MainScript".to_string(),
                ".Done@MainScript".to_string(),
            ]),
        };

        assert_eq!(
            script_runtime_command_issues(&command("special", &["fadeoutmusic"]), &catalog),
            vec![ScriptRuntimeCommandIssue::UnknownSpecialRoutine {
                special_id: "fadeoutmusic".to_string()
            }]
        );
        assert_eq!(
            script_runtime_command_issues(
                &command(
                    "gettrainername",
                    &["STRING_BUFFER_4", "BUG_CATCHER", "FALKNER1"]
                ),
                &catalog
            ),
            vec![ScriptRuntimeCommandIssue::TrainerClassMismatch {
                trainer_id: "FALKNER1".to_string(),
                expected_class: "BUG_CATCHER".to_string(),
                actual_class: "FALKNER".to_string()
            }]
        );
        assert_eq!(
            script_runtime_command_issues(
                &command(
                    "gettrainername",
                    &["STRING_BUFFER_4", "FALKNER", "falkner1"]
                ),
                &catalog
            ),
            vec![ScriptRuntimeCommandIssue::UnknownTrainer {
                trainer_id: "falkner1".to_string()
            }]
        );
        assert_eq!(
            script_runtime_command_issues(
                &command("getitemname", &["BUFFER_1", "potion"]),
                &catalog
            ),
            vec![ScriptRuntimeCommandIssue::UnknownItem {
                item_id: "potion".to_string()
            }]
        );
        assert_eq!(
            script_runtime_command_issues(
                &command(
                    "getitemname",
                    &["BUFFER_1", SCRIPT_RUNTIME_ITEM_FROM_MEMORY_ID]
                ),
                &catalog
            ),
            []
        );
        assert_eq!(
            script_runtime_command_issues(
                &command("getmonname", &["BUFFER_1", "pikachu"]),
                &catalog
            ),
            vec![ScriptRuntimeCommandIssue::UnknownSpecies {
                species_id: "pikachu".to_string()
            }]
        );
        assert_eq!(
            script_runtime_command_issues(&command("addcellnum", &["phone_elm"]), &catalog),
            vec![ScriptRuntimeCommandIssue::UnknownPhoneContact {
                contact_id: "phone_elm".to_string()
            }]
        );
        assert_eq!(
            script_runtime_command_issues(
                &command("specialphonecall", &["specialcall_masterball"]),
                &catalog
            ),
            vec![ScriptRuntimeCommandIssue::UnknownSpecialPhoneCall {
                call_id: "specialcall_masterball".to_string()
            }]
        );
        assert_eq!(
            script_runtime_command_issues(&command("trade", &["npc_trade_mike"]), &catalog),
            vec![ScriptRuntimeCommandIssue::UnknownNpcTrade {
                trade_id: "npc_trade_mike".to_string()
            }]
        );
    }

    #[test]
    fn command_issues_resolve_exact_script_targets_without_fallbacks() {
        let catalog = ScriptRuntimeReferenceCatalog {
            script_labels: BTreeSet::from([
                "AsmScript".to_string(),
                ".Local@AsmScript".to_string(),
                "GlobalTarget".to_string(),
            ]),
            ..ScriptRuntimeReferenceCatalog::default()
        };

        assert_eq!(
            resolve_script_runtime_target_label(
                &catalog.script_labels,
                ".Nested@AsmScript",
                ".Local"
            ),
            Some(".Local@AsmScript".to_string())
        );
        let mut local_call = command("callasm", &[".Local"]);
        local_call.source_script = ".Nested@AsmScript".to_string();
        assert_eq!(script_runtime_command_issues(&local_call, &catalog), []);
        assert_eq!(
            script_runtime_command_issues(&command("callasm", &["GlobalTarget"]), &catalog),
            []
        );
        assert_eq!(
            script_runtime_command_issues(
                &command(
                    "cmdqueue",
                    &[SCRIPT_RUNTIME_CURRENT_BANK_TARGET, ".Missing"]
                ),
                &catalog
            ),
            vec![ScriptRuntimeCommandIssue::UnknownTarget {
                target_label: ".Missing".to_string()
            }]
        );
        assert_eq!(
            script_runtime_command_issues(
                &command(
                    "cmdqueue",
                    &[
                        SCRIPT_RUNTIME_CURRENT_BANK_TARGET,
                        SCRIPT_RUNTIME_CURRENT_BANK_TARGET
                    ],
                ),
                &catalog
            ),
            []
        );
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
            apply_script_runtime_command(&mut state, command("", &["HealParty"]), default_inputs()),
            Err(ScriptRuntimeCommandError::EmptyCommand)
        ));
        assert!(matches!(
            apply_script_runtime_command(
                &mut state,
                command(" special", &["HealParty"]),
                default_inputs()
            ),
            Err(ScriptRuntimeCommandError::PaddedCommand { .. })
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

    #[test]
    fn runtime_command_schema_requires_explicit_args_even_when_empty() {
        let missing_args = serde_json::from_str::<ScriptRuntimeCommand>(
            r#"{
              "command":"verticalmenu",
              "source_script":"RuntimeScript",
              "command_index":4
            }"#,
        )
        .expect_err("missing args must not default to empty")
        .to_string();

        assert!(
            missing_args.contains("missing field `args`"),
            "{missing_args}"
        );

        let explicit_empty_args = serde_json::from_str::<ScriptRuntimeCommand>(
            r#"{
              "command":"verticalmenu",
              "args":[],
              "source_script":"RuntimeScript",
              "command_index":4
            }"#,
        )
        .expect("zero-arg commands must declare an explicit empty args array");

        assert_eq!(explicit_empty_args.command, "verticalmenu");
        assert!(explicit_empty_args.args.is_empty());
        assert_eq!(explicit_empty_args.source_script, "RuntimeScript");
        assert_eq!(explicit_empty_args.command_index, 4);
    }

    #[test]
    fn story_event_script_constants_require_explicit_maps_field() {
        let missing_maps = serde_json::from_str::<StoryEventScriptConstants>(r#"{"global":{}}"#)
            .expect_err("story event constants must declare map constants explicitly")
            .to_string();

        assert!(missing_maps.contains("missing field `maps`"));
    }

    #[test]
    fn initialize_events_require_explicit_variable_sprites_field() {
        let missing_variable_sprites =
            serde_json::from_str::<InitializeEventsConfig>(r#"{"eventFlags":[],"engineFlags":[]}"#)
                .expect_err("initialize event buckets must all be explicit")
                .to_string();

        assert!(missing_variable_sprites.contains("missing field `variableSprites`"));
    }

    #[test]
    fn story_event_script_constants_reject_unknown_pack_fields() {
        let unknown_field = serde_json::from_str::<StoryEventScriptConstants>(
            r#"{"global":{},"maps":{},"legacy":{}}"#,
        )
        .expect_err("story event constants reject unknown pack fields")
        .to_string();

        assert!(unknown_field.contains("unknown field"));
    }

    #[test]
    fn initialize_events_reject_unknown_pack_fields() {
        let unknown_field = serde_json::from_str::<InitializeEventsConfig>(
            r#"{"eventFlags":[],"engineFlags":[],"variableSprites":{},"legacy":true}"#,
        )
        .expect_err("initialize events reject unknown pack fields")
        .to_string();

        assert!(unknown_field.contains("unknown field"));
    }

    #[test]
    fn initialize_events_issues_require_nonempty_flags_and_variable_sprites() {
        let config = InitializeEventsConfig {
            event_flags: vec![
                "EVENT_GOT_STARTER".to_string(),
                String::new(),
                " EVENT_PADDED".to_string(),
            ],
            engine_flags: vec![" ".to_string(), "ENGINE_POKEGEAR".to_string()],
            variable_sprites: [
                (" PADDED_SPRITE".to_string(), "SPRITE_ELM".to_string()),
                (
                    "PADDED_REPLACEMENT".to_string(),
                    " SPRITE_SILVER".to_string(),
                ),
                ("SPRITE_ELM".to_string(), String::new()),
                (String::new(), "SPRITE_SILVER".to_string()),
            ]
            .into_iter()
            .collect(),
        };

        assert_eq!(
            initialize_events_issues(&config),
            vec![
                InitializeEventsIssue::InvalidFlag {
                    flag: String::new(),
                },
                InitializeEventsIssue::InvalidFlag {
                    flag: " EVENT_PADDED".to_string(),
                },
                InitializeEventsIssue::InvalidFlag {
                    flag: " ".to_string(),
                },
                InitializeEventsIssue::InvalidVariableSprite {
                    sprite: String::new(),
                },
                InitializeEventsIssue::InvalidVariableSprite {
                    sprite: " PADDED_SPRITE".to_string(),
                },
                InitializeEventsIssue::InvalidVariableSprite {
                    sprite: "PADDED_REPLACEMENT".to_string(),
                },
                InitializeEventsIssue::InvalidVariableSprite {
                    sprite: "SPRITE_ELM".to_string(),
                },
            ],
        );
    }

    #[test]
    fn story_event_script_constant_issues_require_nonempty_keys() {
        let constants = StoryEventScriptConstants {
            global: [
                ("".to_string(), 1),
                (" TRUE".to_string(), 1),
                ("TRUE".to_string(), 1),
            ]
            .into_iter()
            .collect(),
            maps: [
                (
                    "".to_string(),
                    BTreeMap::from([("EVENT_ONE".to_string(), 1)]),
                ),
                (
                    " ROUTE_30".to_string(),
                    BTreeMap::from([("EVENT_THREE".to_string(), 4)]),
                ),
                (
                    "ROUTE_29".to_string(),
                    BTreeMap::from([
                        ("".to_string(), 2),
                        (" EVENT_PADDED".to_string(), 4),
                        ("EVENT_TWO".to_string(), 3),
                    ]),
                ),
            ]
            .into_iter()
            .collect(),
        };

        assert_eq!(
            story_event_script_constant_issues(&constants),
            vec![
                StoryEventScriptConstantIssue::InvalidGlobalConstant { key: String::new() },
                StoryEventScriptConstantIssue::InvalidGlobalConstant {
                    key: " TRUE".to_string(),
                },
                StoryEventScriptConstantIssue::InvalidMap {
                    map_name: String::new(),
                },
                StoryEventScriptConstantIssue::InvalidMap {
                    map_name: " ROUTE_30".to_string(),
                },
                StoryEventScriptConstantIssue::InvalidMapConstant {
                    map_name: "ROUTE_29".to_string(),
                    key: String::new(),
                },
                StoryEventScriptConstantIssue::InvalidMapConstant {
                    map_name: "ROUTE_29".to_string(),
                    key: " EVENT_PADDED".to_string(),
                },
            ],
        );
    }

    fn default_inputs() -> ScriptRuntimeInputs {
        ScriptRuntimeInputs::default()
    }
}
