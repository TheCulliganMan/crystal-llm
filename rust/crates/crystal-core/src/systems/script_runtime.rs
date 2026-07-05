use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

use crate::models::Dv;
use crate::state::{
    GameState, ScriptRuntimeAsmDirective, ScriptRuntimeDecorationDescription, ScriptRuntimeDelay,
    ScriptRuntimeEarthquake, ScriptRuntimeEffect, ScriptRuntimeElevatorFloor,
    ScriptRuntimeNumericBufferWrite, ScriptRuntimeQueuedCommand, ScriptRuntimeStoneTableEntry,
    ScriptRuntimeVariableWrite,
};

pub const SCRIPT_RUNTIME_SPECIAL_PHONE_CALL_NONE: &str = "SPECIALCALL_NONE";

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
    pub rng_seed_after: Option<u32>,
    pub game_version: Option<String>,
    pub gift_original_trainer_name: Option<String>,
    pub gift_original_trainer_id: Option<u16>,
    pub gift_dvs: Option<Dv>,
    pub gift_rng_seed_after: Option<u32>,
    pub gift_nickname_accepted: Option<bool>,
    pub gift_nickname: Option<String>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StoryEventScriptConstants {
    pub global: BTreeMap<String, i64>,
    pub maps: BTreeMap<String, BTreeMap<String, i64>>,
}

impl<'de> Deserialize<'de> for StoryEventScriptConstants {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawConstants {
            global: BTreeMap<String, i64>,
            maps: BTreeMap<String, BTreeMap<String, i64>>,
        }

        let raw = RawConstants::deserialize(deserializer)?;
        for key in raw.global.keys() {
            require_runtime_token("story_event_script_constants.global key", key)
                .map_err(serde::de::Error::custom)?;
        }
        for (map_name, constants) in &raw.maps {
            require_runtime_token("story_event_script_constants.maps key", map_name)
                .map_err(serde::de::Error::custom)?;
            for key in constants.keys() {
                require_runtime_token(
                    &format!("story_event_script_constants.maps[{map_name}] key"),
                    key,
                )
                .map_err(serde::de::Error::custom)?;
            }
        }
        Ok(Self {
            global: raw.global,
            maps: raw.maps,
        })
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InitializeEventsConfig {
    pub event_flags: Vec<String>,
    pub engine_flags: Vec<String>,
    pub variable_sprites: BTreeMap<String, String>,
}

impl<'de> Deserialize<'de> for InitializeEventsConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        struct RawConfig {
            event_flags: Vec<String>,
            engine_flags: Vec<String>,
            variable_sprites: BTreeMap<String, String>,
        }

        let raw = RawConfig::deserialize(deserializer)?;
        for flag in &raw.event_flags {
            require_runtime_token("initialize_events.eventFlags", flag)
                .map_err(serde::de::Error::custom)?;
        }
        for flag in &raw.engine_flags {
            require_runtime_token("initialize_events.engineFlags", flag)
                .map_err(serde::de::Error::custom)?;
        }
        for (sprite, replacement) in &raw.variable_sprites {
            require_runtime_token("initialize_events.variableSprites key", sprite)
                .map_err(serde::de::Error::custom)?;
            require_runtime_token(
                &format!("initialize_events.variableSprites[{sprite}]"),
                replacement,
            )
            .map_err(serde::de::Error::custom)?;
        }
        Ok(Self {
            event_flags: raw.event_flags,
            engine_flags: raw.engine_flags,
            variable_sprites: raw.variable_sprites,
        })
    }
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

pub fn apply_initialize_events(
    state: &mut GameState,
    config: &InitializeEventsConfig,
) -> Result<(), String> {
    for flag in &config.event_flags {
        state
            .flags
            .set_event_flag(flag, true)
            .map_err(|error| format!("initialize_events.eventFlags[{flag}]: {error}"))?;
    }
    for flag in &config.engine_flags {
        state
            .flags
            .set_engine_flag(flag, true)
            .map_err(|error| format!("initialize_events.engineFlags[{flag}]: {error}"))?;
    }
    for (sprite, replacement) in &config.variable_sprites {
        state
            .script_runtime
            .variable_sprites
            .insert(sprite.clone(), replacement.clone());
    }
    Ok(())
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
#[serde(rename_all = "snake_case", deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
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
    #[error("script runtime source script '{source_script}' is not exact pack syntax")]
    InvalidSourceScript { source_script: String },
    #[error("script runtime command '{command}' has invalid numeric token syntax '{token}'")]
    InvalidNumericToken { command: String, token: String },
    #[error("script runtime command '{command}' has an unknown numeric token '{token}'")]
    UnknownNumericToken { command: String, token: String },
    #[error("script runtime command '{command}' requires script accumulator")]
    MissingAccumulator { command: String },
    #[error("script runtime command '{command}' requires an active menu")]
    MissingActiveMenu { command: String },
    #[error("script runtime command 'random' requires deterministic random input")]
    MissingRandomInput,
    #[error("script runtime command 'random' requires deterministic rng_seed_after input")]
    MissingRandomSeedAfter,
    #[error("script runtime command '{command}' must not declare random input fields")]
    UnexpectedRandomInput { command: String },
    #[error("script runtime command 'random' requires a positive upper bound")]
    RandomBoundZero,
    #[error("script runtime command 'random' received value {value} outside upper bound {bound}")]
    RandomInputOutOfRange { value: u32, bound: u32 },
    #[error("script runtime command 'checkver' requires explicit game version input")]
    MissingGameVersion,
    #[error("script runtime command 'pop' cannot pop an empty runtime stack")]
    EmptyStack,
    #[error("script dispatch has invalid next script '{script}'")]
    InvalidNextScript { script: String },
    #[error("script dispatch has invalid last talked object '{object_identifier}'")]
    InvalidLastTalkedObject { object_identifier: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptDispatchOutcome {
    pub next_script: String,
    pub last_talked_object: Option<String>,
}

pub fn commit_interaction_script_dispatch(
    state: &mut GameState,
    session_last_talked_object_identifier: &mut Option<String>,
    next_script: &str,
    last_talked_object: Option<&str>,
) -> Result<ScriptDispatchOutcome, ScriptRuntimeCommandError> {
    if !is_exact_nonempty_runtime_token(next_script) {
        return Err(ScriptRuntimeCommandError::InvalidNextScript {
            script: next_script.to_string(),
        });
    }
    if let Some(object_identifier) = last_talked_object
        && !is_exact_nonempty_runtime_token(object_identifier)
    {
        return Err(ScriptRuntimeCommandError::InvalidLastTalkedObject {
            object_identifier: object_identifier.to_string(),
        });
    }
    state.script_runtime.next_script = Some(next_script.to_string());
    state.script_runtime.last_talked_object = last_talked_object.map(str::to_string);
    *session_last_talked_object_identifier = last_talked_object.map(str::to_string);
    Ok(ScriptDispatchOutcome {
        next_script: next_script.to_string(),
        last_talked_object: last_talked_object.map(str::to_string),
    })
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
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

impl<'de> Deserialize<'de> for ScriptRuntimeReferenceCatalog {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawCatalog {
            special_routines: BTreeSet<String>,
            trainer_classes: BTreeMap<String, String>,
            items: BTreeSet<String>,
            pokemon: BTreeSet<String>,
            phone_contacts: BTreeSet<String>,
            special_phone_calls: BTreeSet<String>,
            npc_trades: BTreeSet<String>,
            script_labels: BTreeSet<String>,
        }

        let raw = RawCatalog::deserialize(deserializer)?;
        validate_runtime_pack_id_set("script_runtime.special_routines", &raw.special_routines)
            .map_err(serde::de::Error::custom)?;
        validate_runtime_pack_id_map("script_runtime.trainer_classes", &raw.trainer_classes)
            .map_err(serde::de::Error::custom)?;
        validate_runtime_pack_id_set("script_runtime.items", &raw.items)
            .map_err(serde::de::Error::custom)?;
        validate_runtime_pack_id_set("script_runtime.pokemon", &raw.pokemon)
            .map_err(serde::de::Error::custom)?;
        validate_runtime_pack_id_set("script_runtime.phone_contacts", &raw.phone_contacts)
            .map_err(serde::de::Error::custom)?;
        validate_runtime_pack_id_set(
            "script_runtime.special_phone_calls",
            &raw.special_phone_calls,
        )
        .map_err(serde::de::Error::custom)?;
        validate_runtime_pack_id_set("script_runtime.npc_trades", &raw.npc_trades)
            .map_err(serde::de::Error::custom)?;
        for label in &raw.script_labels {
            require_runtime_label("script_runtime.script_labels", label)
                .map_err(serde::de::Error::custom)?;
        }
        Ok(Self {
            special_routines: raw.special_routines,
            trainer_classes: raw.trainer_classes,
            items: raw.items,
            pokemon: raw.pokemon,
            phone_contacts: raw.phone_contacts,
            special_phone_calls: raw.special_phone_calls,
            npc_trades: raw.npc_trades,
            script_labels: raw.script_labels,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ScriptRuntimeCommandIssue {
    InvalidCommand {
        error: ScriptRuntimeCommandError,
    },
    UnknownSpecialRoutine {
        special_id: String,
    },
    InvalidSpecialRoutine {
        special_id: String,
    },
    UnknownTrainer {
        trainer_id: String,
    },
    InvalidTrainer {
        trainer_id: String,
    },
    InvalidTrainerClass {
        trainer_class: String,
    },
    TrainerClassMismatch {
        trainer_id: String,
        expected_class: String,
        actual_class: String,
    },
    UnknownItem {
        item_id: String,
    },
    InvalidItem {
        item_id: String,
    },
    UnknownSpecies {
        species_id: String,
    },
    InvalidSpecies {
        species_id: String,
    },
    UnknownPhoneContact {
        contact_id: String,
    },
    InvalidPhoneContact {
        contact_id: String,
    },
    UnknownSpecialPhoneCall {
        call_id: String,
    },
    InvalidSpecialPhoneCall {
        call_id: String,
    },
    UnknownNpcTrade {
        trade_id: String,
    },
    InvalidNpcTrade {
        trade_id: String,
    },
    UnknownTarget {
        target_label: String,
    },
    InvalidTarget {
        target_label: String,
    },
}

pub const SCRIPT_RUNTIME_USE_SCRIPT_VAR_ID: &str = "USE_SCRIPT_VAR";
pub const SCRIPT_RUNTIME_ITEM_FROM_MEMORY_ID: &str = "ITEM_FROM_MEM";
pub const SCRIPT_RUNTIME_CURRENT_BANK_TARGET: &str = "BANK(@)";

fn is_exact_nonempty_runtime_token(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && !has_reserved_pack_prefix(value)
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'@' | b'(' | b')')
        })
}

fn is_exact_nonempty_runtime_arg_token(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && !has_reserved_pack_prefix(value)
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric()
                || matches!(
                    byte,
                    b'_' | b'.'
                        | b'@'
                        | b'('
                        | b')'
                        | b'['
                        | b']'
                        | b'-'
                        | b'+'
                        | b'$'
                        | b'%'
                        | b' '
                )
        })
}

fn is_exact_nonempty_runtime_pack_id(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && !has_reserved_pack_prefix(value)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn is_exact_nonempty_runtime_label(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && !has_reserved_pack_prefix(value)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'@'))
}

fn require_runtime_token(field: &str, value: &str) -> Result<(), String> {
    if is_exact_nonempty_runtime_token(value) {
        Ok(())
    } else {
        Err(format!(
            "{field} must be a nonempty exact runtime token, found {value:?}"
        ))
    }
}

fn require_runtime_pack_id(field: &str, value: &str) -> Result<(), String> {
    if is_exact_nonempty_runtime_pack_id(value) {
        Ok(())
    } else {
        Err(format!(
            "{field} must be a nonempty exact pack id, found {value:?}"
        ))
    }
}

fn require_runtime_label(field: &str, value: &str) -> Result<(), String> {
    if is_exact_nonempty_runtime_label(value) {
        Ok(())
    } else {
        Err(format!(
            "{field} must be a nonempty exact script label, found {value:?}"
        ))
    }
}

fn validate_runtime_pack_id_set(field: &str, values: &BTreeSet<String>) -> Result<(), String> {
    for value in values {
        require_runtime_pack_id(field, value)?;
    }
    Ok(())
}

fn validate_runtime_pack_id_map(
    field: &str,
    values: &BTreeMap<String, String>,
) -> Result<(), String> {
    for (key, value) in values {
        require_runtime_pack_id(&format!("{field} key"), key)?;
        require_runtime_pack_id(&format!("{field}[{key}]"), value)?;
    }
    Ok(())
}

fn has_reserved_pack_prefix(value: &str) -> bool {
    let value = value.to_ascii_lowercase();
    value.starts_with("fallback") || value.starts_with("legacy")
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
            if !is_exact_nonempty_runtime_pack_id(special_id) {
                issues.push(ScriptRuntimeCommandIssue::InvalidSpecialRoutine {
                    special_id: special_id.clone(),
                });
            } else if !catalog.special_routines.contains(special_id) {
                issues.push(ScriptRuntimeCommandIssue::UnknownSpecialRoutine {
                    special_id: special_id.clone(),
                });
            }
        }
        "gettrainername" => {
            let trainer_class = &command.args[1];
            let trainer_id = &command.args[2];
            let valid_trainer_class = is_exact_nonempty_runtime_pack_id(trainer_class);
            let valid_trainer_id = is_exact_nonempty_runtime_pack_id(trainer_id);
            if !valid_trainer_class {
                issues.push(ScriptRuntimeCommandIssue::InvalidTrainerClass {
                    trainer_class: trainer_class.clone(),
                });
            }
            if !valid_trainer_id {
                issues.push(ScriptRuntimeCommandIssue::InvalidTrainer {
                    trainer_id: trainer_id.clone(),
                });
            }
            if !valid_trainer_class || !valid_trainer_id {
                return issues;
            }
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
            {
                if !is_exact_nonempty_runtime_pack_id(item_id) {
                    issues.push(ScriptRuntimeCommandIssue::InvalidItem {
                        item_id: item_id.clone(),
                    });
                } else if !catalog.items.contains(item_id) {
                    issues.push(ScriptRuntimeCommandIssue::UnknownItem {
                        item_id: item_id.clone(),
                    });
                }
            }
        }
        "getmonname" => {
            let species_id = &command.args[1];
            if species_id != SCRIPT_RUNTIME_USE_SCRIPT_VAR_ID {
                if !is_exact_nonempty_runtime_pack_id(species_id) {
                    issues.push(ScriptRuntimeCommandIssue::InvalidSpecies {
                        species_id: species_id.clone(),
                    });
                } else if !catalog.pokemon.contains(species_id) {
                    issues.push(ScriptRuntimeCommandIssue::UnknownSpecies {
                        species_id: species_id.clone(),
                    });
                }
            }
        }
        "addcellnum" => {
            let contact_id = &command.args[0];
            if !is_exact_nonempty_runtime_pack_id(contact_id) {
                issues.push(ScriptRuntimeCommandIssue::InvalidPhoneContact {
                    contact_id: contact_id.clone(),
                });
            } else if !catalog.phone_contacts.contains(contact_id) {
                issues.push(ScriptRuntimeCommandIssue::UnknownPhoneContact {
                    contact_id: contact_id.clone(),
                });
            }
        }
        "specialphonecall" => {
            let call_id = &command.args[0];
            if !is_exact_nonempty_runtime_pack_id(call_id) {
                issues.push(ScriptRuntimeCommandIssue::InvalidSpecialPhoneCall {
                    call_id: call_id.clone(),
                });
            } else if call_id != SCRIPT_RUNTIME_SPECIAL_PHONE_CALL_NONE
                && !catalog.special_phone_calls.contains(call_id)
            {
                issues.push(ScriptRuntimeCommandIssue::UnknownSpecialPhoneCall {
                    call_id: call_id.clone(),
                });
            }
        }
        "checkpoke" | "pokepic" => {
            let species_id = &command.args[0];
            if !is_exact_nonempty_runtime_pack_id(species_id) {
                issues.push(ScriptRuntimeCommandIssue::InvalidSpecies {
                    species_id: species_id.clone(),
                });
            } else if !catalog.pokemon.contains(species_id) {
                issues.push(ScriptRuntimeCommandIssue::UnknownSpecies {
                    species_id: species_id.clone(),
                });
            }
        }
        "trade" => {
            let trade_id = &command.args[0];
            if !is_exact_nonempty_runtime_pack_id(trade_id) {
                issues.push(ScriptRuntimeCommandIssue::InvalidNpcTrade {
                    trade_id: trade_id.clone(),
                });
            } else if !catalog.npc_trades.contains(trade_id) {
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
    if target_label == SCRIPT_RUNTIME_CURRENT_BANK_TARGET {
        return;
    }
    if !is_exact_nonempty_runtime_label(target_label) {
        issues.push(ScriptRuntimeCommandIssue::InvalidTarget {
            target_label: target_label.to_string(),
        });
    } else if resolve_script_runtime_target_label(
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
    if command.command != "random"
        && (inputs.random_value.is_some() || inputs.rng_seed_after.is_some())
    {
        return Err(ScriptRuntimeCommandError::UnexpectedRandomInput {
            command: command.command.clone(),
        });
    }

    let outcome = match command.command.as_str() {
        "addval" => {
            let left = parse_required_accumulator(state, &command)?;
            let right = parse_i32_token(&command.command, &command.args[0])?;
            set_script_value(state, &command, (left + right).to_string())
        }
        "random" => {
            let bound = parse_u32_token(&command.command, &command.args[0])?;
            if bound == 0 {
                return Err(ScriptRuntimeCommandError::RandomBoundZero);
            }
            let value = inputs
                .random_value
                .ok_or(ScriptRuntimeCommandError::MissingRandomInput)?;
            if value >= bound {
                return Err(ScriptRuntimeCommandError::RandomInputOutOfRange { value, bound });
            }
            let rng_seed_after = inputs
                .rng_seed_after
                .ok_or(ScriptRuntimeCommandError::MissingRandomSeedAfter)?;
            state.rng_seed = rng_seed_after;
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
    if !is_exact_nonempty_runtime_token(&command.command) {
        return Err(ScriptRuntimeCommandError::PaddedCommand {
            command: command.command.clone(),
        });
    }
    if !is_exact_nonempty_runtime_label(&command.source_script) {
        return Err(ScriptRuntimeCommandError::InvalidSourceScript {
            source_script: command.source_script.clone(),
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
        if !is_exact_nonempty_runtime_arg_token(arg) {
            return Err(ScriptRuntimeCommandError::PaddedArg {
                command: command.command.clone(),
                arg: arg.clone(),
            });
        }
    }
    if command.command == "special" && !is_exact_nonempty_runtime_pack_id(&command.args[0]) {
        return Err(ScriptRuntimeCommandError::PaddedArg {
            command: command.command.clone(),
            arg: command.args[0].clone(),
        });
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
        "verticalmenu" => {
            if state.script_runtime.active_menu.is_none() {
                return Err(ScriptRuntimeCommandError::MissingActiveMenu {
                    command: command.command.clone(),
                });
            }
            state.script_runtime.window_open = true;
        }
        "closewindow" => state.script_runtime.window_open = false,
        "menu_coords" => {
            state.script_runtime.menu_coords = Some([
                parse_menu_coord_token(&command.command, &command.args[0])?,
                parse_menu_coord_token(&command.command, &command.args[1])?,
                parse_menu_coord_token(&command.command, &command.args[2])?,
                parse_menu_coord_token(&command.command, &command.args[3])?,
            ]);
        }
        "dontrestartmapmusic" => state.script_runtime.map_music_restart_disabled = true,
        "playmapmusic" => state.script_runtime.map_music_requested = true,
        "lock" => state.script_runtime.player_input_locked = true,
        "release" => state.script_runtime.player_input_locked = false,
        "lockall" => state.script_runtime.all_input_locked = true,
        "releaseall" => state.script_runtime.all_input_locked = false,
        "stop" => state.script_runtime.script_stop_requested = true,
        "itemnotify" => state.script_runtime.item_notify_queued = true,
        "verbosegiveitemvar" => {
            state
                .script_runtime
                .named_buffers
                .insert(command.args[0].clone(), command.args[1].clone());
        }
        "addcellnum" => {
            let added = state
                .script_runtime
                .phone_numbers
                .insert(command.args[0].clone());
            state.script_runtime.script_value = Some(if added {
                "0".to_string()
            } else {
                "1".to_string()
            });
        }
        "specialphonecall" => {
            if command.args[0] == SCRIPT_RUNTIME_SPECIAL_PHONE_CALL_NONE {
                state.script_runtime.special_phone_calls.clear();
            } else {
                state
                    .script_runtime
                    .special_phone_calls
                    .push(command.args[0].clone());
            }
        }
        "pokepic" => state.script_runtime.active_pokemon_picture = Some(command.args[0].clone()),
        "closepokepic" => state.script_runtime.active_pokemon_picture = None,
        "trade" => state
            .script_runtime
            .completed_trades
            .push(command.args[0].clone()),
        "catchtutorial" => {
            state
                .script_runtime
                .catch_tutorials
                .push(command.args[0].clone());
            state.script_runtime.script_value = Some("1".to_string());
        }
        "warpsound" => state.script_runtime.warp_sound_queued = true,
        "blackoutmod" => state.script_runtime.blackout_mod = Some(command.args[0].clone()),
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

pub fn parse_menu_coord_token(
    command: &str,
    token: &str,
) -> Result<i16, ScriptRuntimeCommandError> {
    if let Ok(value) = parse_i16_token(command, token) {
        return Ok(value);
    }
    if token.is_empty() || token.trim() != token || token.contains('\t') {
        return Err(ScriptRuntimeCommandError::InvalidNumericToken {
            command: command.to_string(),
            token: token.to_string(),
        });
    }
    let tokens = token.split(' ').collect::<Vec<_>>();
    if tokens.iter().any(|part| part.is_empty()) {
        return Err(ScriptRuntimeCommandError::InvalidNumericToken {
            command: command.to_string(),
            token: token.to_string(),
        });
    }
    let value = match tokens.as_slice() {
        [constant] => menu_coord_constant(command, constant)?,
        [constant, "+", amount] => {
            menu_coord_constant(command, constant)? + parse_i16_token(command, amount)?
        }
        [constant, "-", amount] => {
            menu_coord_constant(command, constant)? - parse_i16_token(command, amount)?
        }
        _ => {
            return Err(ScriptRuntimeCommandError::InvalidNumericToken {
                command: command.to_string(),
                token: token.to_string(),
            });
        }
    };
    Ok(value)
}

fn menu_coord_constant(command: &str, token: &str) -> Result<i16, ScriptRuntimeCommandError> {
    match token {
        "SCREEN_LEFT" | "SCREEN_TOP" => Ok(0),
        "SCREEN_WIDTH" => Ok(20),
        "SCREEN_HEIGHT" => Ok(18),
        "SCREEN_EDGE" | "SCREEN_RIGHT" => Ok(19),
        "SCREEN_BOTTOM" => Ok(17),
        "TEXTBOX_Y" => Ok(12),
        _ => Err(ScriptRuntimeCommandError::UnknownNumericToken {
            command: command.to_string(),
            token: token.to_string(),
        }),
    }
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

pub fn parse_script_i32_token(
    command: &str,
    token: &str,
) -> Result<i32, ScriptRuntimeCommandError> {
    parse_i32_token(command, token)
}

fn parse_i32_token(command: &str, token: &str) -> Result<i32, ScriptRuntimeCommandError> {
    if !is_potential_numeric_token(token) {
        return Err(if is_exact_numeric_symbol(token) {
            ScriptRuntimeCommandError::UnknownNumericToken {
                command: command.to_string(),
                token: token.to_string(),
            }
        } else {
            ScriptRuntimeCommandError::InvalidNumericToken {
                command: command.to_string(),
                token: token.to_string(),
            }
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
    if digits.is_empty() || !digits_are_valid(digits, radix) {
        return Err(ScriptRuntimeCommandError::InvalidNumericToken {
            command: command.to_string(),
            token: token.to_string(),
        });
    }
    i32::from_str_radix(digits, radix)
        .map(|value| value * sign)
        .map_err(|_| ScriptRuntimeCommandError::InvalidNumericToken {
            command: command.to_string(),
            token: token.to_string(),
        })
}

fn is_potential_numeric_token(token: &str) -> bool {
    if token.is_empty() || token.trim() != token {
        return false;
    }
    let raw = match token.as_bytes()[0] {
        b'-' | b'+' => &token[1..],
        _ => token,
    };
    if raw.is_empty() {
        return false;
    }
    if raw.strip_prefix('$').is_some() || raw.strip_prefix('%').is_some() {
        return true;
    }
    raw.bytes().all(|byte| byte.is_ascii_digit())
}

fn is_exact_numeric_symbol(token: &str) -> bool {
    let Some(first) = token.bytes().next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == b'_')
        && token.trim() == token
        && token
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn digits_are_valid(digits: &str, radix: u32) -> bool {
    match radix {
        2 => digits.bytes().all(|byte| matches!(byte, b'0' | b'1')),
        10 => digits.bytes().all(|byte| byte.is_ascii_digit()),
        16 => digits.bytes().all(|byte| byte.is_ascii_hexdigit()),
        _ => false,
    }
}

pub fn script_runtime_command_arg_counts() -> BTreeMap<&'static str, usize> {
    BTreeMap::from([
        ("special", 1),
        ("pause", 1),
        ("earthquake", 1),
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
        ("lock", 0),
        ("release", 0),
        ("lockall", 0),
        ("releaseall", 0),
        ("stop", 0),
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
    fn interaction_script_dispatch_commits_state_and_session_target() {
        let mut state = GameState::default();
        let mut last_talked_object = None;

        let outcome = commit_interaction_script_dispatch(
            &mut state,
            &mut last_talked_object,
            "Route36SudowoodoScript",
            Some("ROUTE36_SUDOWOODO"),
        )
        .expect("dispatch commits");

        assert_eq!(
            outcome,
            ScriptDispatchOutcome {
                next_script: "Route36SudowoodoScript".to_string(),
                last_talked_object: Some("ROUTE36_SUDOWOODO".to_string()),
            }
        );
        assert_eq!(
            state.script_runtime.next_script.as_deref(),
            Some("Route36SudowoodoScript")
        );
        assert_eq!(
            state.script_runtime.last_talked_object.as_deref(),
            Some("ROUTE36_SUDOWOODO")
        );
        assert_eq!(last_talked_object.as_deref(), Some("ROUTE36_SUDOWOODO"));
    }

    #[test]
    fn interaction_script_dispatch_rejects_invalid_tokens_without_mutation() {
        let mut state = GameState::default();
        let mut last_talked_object = Some("UNCHANGED_OBJECT".to_string());

        let error = commit_interaction_script_dispatch(
            &mut state,
            &mut last_talked_object,
            "Route36SudowoodoScript",
            Some("fallback_object"),
        )
        .expect_err("invalid object token rejected");

        assert_eq!(
            error,
            ScriptRuntimeCommandError::InvalidLastTalkedObject {
                object_identifier: "fallback_object".to_string(),
            }
        );
        assert_eq!(state.script_runtime.next_script, None);
        assert_eq!(state.script_runtime.last_talked_object, None);
        assert_eq!(last_talked_object.as_deref(), Some("UNCHANGED_OBJECT"));
    }

    #[test]
    fn exported_runtime_command_arity_table_is_validation_source() {
        let counts = script_runtime_command_arg_counts();
        assert_eq!(counts.get("special"), Some(&1));
        assert_eq!(counts.get("checkver"), Some(&0));
        assert_eq!(counts.get("givepokemail"), Some(&1));
        assert_eq!(counts.get("lock"), Some(&0));
        assert_eq!(counts.get("release"), Some(&0));
        assert_eq!(counts.get("lockall"), Some(&0));
        assert_eq!(counts.get("releaseall"), Some(&0));
        assert_eq!(counts.get("stop"), Some(&0));
        assert!(!counts.contains_key("checkscene"));
        assert!(!counts.contains_key("endifjustbattled"));
        assert!(!counts.contains_key("faceplayer"));
        assert!(!counts.contains_key("jumpstd"));
        assert!(!counts.contains_key("showemote"));
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
    fn command_issues_reject_reserved_runtime_tokens() {
        let catalog = ScriptRuntimeReferenceCatalog {
            items: BTreeSet::from(["POTION".to_string()]),
            script_labels: BTreeSet::from(["MainScript".to_string()]),
            ..ScriptRuntimeReferenceCatalog::default()
        };

        assert_eq!(
            validate_script_runtime_command(&command("fallbackspecial", &["HealParty"])),
            Err(ScriptRuntimeCommandError::PaddedCommand {
                command: "fallbackspecial".to_string(),
            })
        );
        assert_eq!(
            script_runtime_command_issues(
                &command("getitemname", &["BUFFER_1", "legacy_POTION"]),
                &catalog
            ),
            vec![ScriptRuntimeCommandIssue::InvalidCommand {
                error: ScriptRuntimeCommandError::PaddedArg {
                    command: "getitemname".to_string(),
                    arg: "legacy_POTION".to_string(),
                }
            }]
        );
        assert_eq!(
            script_runtime_command_issues(&command("callasm", &["fallbackMainScript"]), &catalog),
            vec![ScriptRuntimeCommandIssue::InvalidCommand {
                error: ScriptRuntimeCommandError::PaddedArg {
                    command: "callasm".to_string(),
                    arg: "fallbackMainScript".to_string(),
                }
            }]
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
            script_runtime_command_issues(&command("special", &["$FadeOutMusic"]), &catalog),
            vec![ScriptRuntimeCommandIssue::InvalidCommand {
                error: ScriptRuntimeCommandError::PaddedArg {
                    command: "special".to_string(),
                    arg: "$FadeOutMusic".to_string(),
                }
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
                &command(
                    "gettrainername",
                    &["STRING_BUFFER_4", "$FALKNER", "$FALKNER1"]
                ),
                &catalog
            ),
            vec![
                ScriptRuntimeCommandIssue::InvalidTrainerClass {
                    trainer_class: "$FALKNER".to_string()
                },
                ScriptRuntimeCommandIssue::InvalidTrainer {
                    trainer_id: "$FALKNER1".to_string()
                }
            ]
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
                &command("getitemname", &["BUFFER_1", "$POTION"]),
                &catalog
            ),
            vec![ScriptRuntimeCommandIssue::InvalidItem {
                item_id: "$POTION".to_string()
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
            script_runtime_command_issues(
                &command("getmonname", &["BUFFER_1", "$PIKACHU"]),
                &catalog
            ),
            vec![ScriptRuntimeCommandIssue::InvalidSpecies {
                species_id: "$PIKACHU".to_string()
            }]
        );
        assert_eq!(
            script_runtime_command_issues(&command("checkpoke", &["PIKA+CHU"]), &catalog),
            vec![ScriptRuntimeCommandIssue::InvalidSpecies {
                species_id: "PIKA+CHU".to_string()
            }]
        );
        assert_eq!(
            script_runtime_command_issues(&command("addcellnum", &["phone_elm"]), &catalog),
            vec![ScriptRuntimeCommandIssue::UnknownPhoneContact {
                contact_id: "phone_elm".to_string()
            }]
        );
        assert_eq!(
            script_runtime_command_issues(&command("addcellnum", &["PHONE ELM"]), &catalog),
            vec![ScriptRuntimeCommandIssue::InvalidPhoneContact {
                contact_id: "PHONE ELM".to_string()
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
            script_runtime_command_issues(
                &command("specialphonecall", &["SPECIAL CALL MASTERBALL"]),
                &catalog
            ),
            vec![ScriptRuntimeCommandIssue::InvalidSpecialPhoneCall {
                call_id: "SPECIAL CALL MASTERBALL".to_string()
            }]
        );
        assert_eq!(
            script_runtime_command_issues(
                &command(
                    "specialphonecall",
                    &[SCRIPT_RUNTIME_SPECIAL_PHONE_CALL_NONE]
                ),
                &catalog
            ),
            []
        );
        assert_eq!(
            script_runtime_command_issues(&command("trade", &["npc_trade_mike"]), &catalog),
            vec![ScriptRuntimeCommandIssue::UnknownNpcTrade {
                trade_id: "npc_trade_mike".to_string()
            }]
        );
        assert_eq!(
            script_runtime_command_issues(&command("trade", &["NPC TRADE MIKE"]), &catalog),
            vec![ScriptRuntimeCommandIssue::InvalidNpcTrade {
                trade_id: "NPC TRADE MIKE".to_string()
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
                    &[SCRIPT_RUNTIME_CURRENT_BANK_TARGET, "$Missing"]
                ),
                &catalog
            ),
            vec![ScriptRuntimeCommandIssue::InvalidTarget {
                target_label: "$Missing".to_string()
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
        assert_eq!(state.script_runtime.script_value.as_deref(), Some("0"));
        apply_script_runtime_command(
            &mut state,
            command("addcellnum", &["PHONE_YOUNGSTER_JOE"]),
            default_inputs(),
        )
        .expect("duplicate addcellnum");
        assert_eq!(state.script_runtime.script_value.as_deref(), Some("1"));
        assert_eq!(state.script_runtime.effects.len(), 4);
        assert_eq!(state.script_runtime.effects[0].command, "special");
        assert_eq!(state.script_runtime.effects[3].command, "addcellnum");
    }

    #[test]
    fn catchtutorial_records_battle_type_and_sets_true_script_value() {
        let mut state = GameState::default();

        apply_script_runtime_command(
            &mut state,
            command("catchtutorial", &["BATTLETYPE_TUTORIAL"]),
            default_inputs(),
        )
        .expect("catchtutorial");

        assert_eq!(
            state.script_runtime.catch_tutorials,
            vec!["BATTLETYPE_TUTORIAL".to_string()]
        );
        assert_eq!(state.script_runtime.script_value.as_deref(), Some("1"));
    }

    #[test]
    fn specialphonecall_none_clears_queued_calls_without_saving_sentinel() {
        let mut state = GameState::default();

        apply_script_runtime_command(
            &mut state,
            command("specialphonecall", &["SPECIALCALL_MASTERBALL"]),
            default_inputs(),
        )
        .expect("queue call");
        apply_script_runtime_command(
            &mut state,
            command(
                "specialphonecall",
                &[SCRIPT_RUNTIME_SPECIAL_PHONE_CALL_NONE],
            ),
            default_inputs(),
        )
        .expect("clear calls");

        assert!(state.script_runtime.special_phone_calls.is_empty());
        assert_eq!(
            state
                .script_runtime
                .effects
                .iter()
                .map(|effect| (effect.command.clone(), effect.args.clone()))
                .collect::<Vec<_>>(),
            vec![
                (
                    "specialphonecall".to_string(),
                    vec!["SPECIALCALL_MASTERBALL".to_string()]
                ),
                (
                    "specialphonecall".to_string(),
                    vec![SCRIPT_RUNTIME_SPECIAL_PHONE_CALL_NONE.to_string()]
                ),
            ]
        );
    }

    #[test]
    fn lock_release_and_stop_commands_mutate_exact_runtime_state() {
        let mut state = GameState::default();

        apply_script_runtime_command(&mut state, command("lock", &[]), default_inputs())
            .expect("lock");
        apply_script_runtime_command(&mut state, command("lockall", &[]), default_inputs())
            .expect("lockall");
        apply_script_runtime_command(&mut state, command("stop", &[]), default_inputs())
            .expect("stop");

        assert!(state.script_runtime.player_input_locked);
        assert!(state.script_runtime.all_input_locked);
        assert!(state.script_runtime.script_stop_requested);

        apply_script_runtime_command(&mut state, command("release", &[]), default_inputs())
            .expect("release");
        apply_script_runtime_command(&mut state, command("releaseall", &[]), default_inputs())
            .expect("releaseall");

        assert!(!state.script_runtime.player_input_locked);
        assert!(!state.script_runtime.all_input_locked);
        assert!(state.script_runtime.script_stop_requested);
        assert_eq!(
            state
                .script_runtime
                .effects
                .iter()
                .map(|effect| effect.command.as_str())
                .collect::<Vec<_>>(),
            vec!["lock", "lockall", "stop", "release", "releaseall"]
        );
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
                rng_seed_after: Some(1234),
                ..ScriptRuntimeInputs::default()
            },
        )
        .expect("random");
        assert_eq!(state.script_runtime.script_value.as_deref(), Some("7"));
        assert_eq!(state.rng_seed, 1234);

        assert!(matches!(
            apply_script_runtime_command(
                &mut state,
                command("random", &["10"]),
                ScriptRuntimeInputs {
                    random_value: Some(7),
                    ..ScriptRuntimeInputs::default()
                },
            ),
            Err(ScriptRuntimeCommandError::MissingRandomSeedAfter)
        ));

        assert!(matches!(
            apply_script_runtime_command(
                &mut state,
                command("random", &["10"]),
                ScriptRuntimeInputs {
                    random_value: Some(10),
                    ..ScriptRuntimeInputs::default()
                },
            ),
            Err(ScriptRuntimeCommandError::RandomInputOutOfRange { .. })
        ));
        assert_eq!(
            apply_script_runtime_command(
                &mut state,
                command("random", &["0"]),
                ScriptRuntimeInputs {
                    random_value: Some(0),
                    ..ScriptRuntimeInputs::default()
                },
            ),
            Err(ScriptRuntimeCommandError::RandomBoundZero)
        );

        assert!(matches!(
            apply_script_runtime_command(
                &mut state,
                command("addval", &["1"]),
                ScriptRuntimeInputs {
                    random_value: Some(0),
                    rng_seed_after: Some(1234),
                    ..ScriptRuntimeInputs::default()
                },
            ),
            Err(ScriptRuntimeCommandError::UnexpectedRandomInput { command })
                if command == "addval"
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
                game_version: Some("CRYSTAL".to_string()),
                ..ScriptRuntimeInputs::default()
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
        state.script_runtime.script_value = Some("12 3".to_string());
        assert!(matches!(
            apply_script_runtime_command(
                &mut state,
                command("getnum", &["STRING_BUFFER_3"]),
                default_inputs(),
            ),
            Err(ScriptRuntimeCommandError::InvalidNumericToken { .. })
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
            apply_script_runtime_command(&mut state, command("pause", &["%102"]), default_inputs()),
            Err(ScriptRuntimeCommandError::InvalidNumericToken { .. })
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
                command("spe cial", &["HealParty"]),
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
                command("special", &["Heal Party"]),
                default_inputs()
            ),
            Err(ScriptRuntimeCommandError::PaddedArg { .. })
        ));
        let mut bad_source = command("special", &["HealParty"]);
        bad_source.source_script = "fallback_script".to_string();
        assert_eq!(
            apply_script_runtime_command(&mut state, bad_source, default_inputs()),
            Err(ScriptRuntimeCommandError::InvalidSourceScript {
                source_script: "fallback_script".to_string(),
            })
        );
        assert!(matches!(
            apply_script_runtime_command(
                &mut state,
                command("pause", &["FOREVER"]),
                default_inputs()
            ),
            Err(ScriptRuntimeCommandError::UnknownNumericToken { .. })
        ));
        assert!(matches!(
            apply_script_runtime_command(&mut state, command("pause", &["$"]), default_inputs()),
            Err(ScriptRuntimeCommandError::InvalidNumericToken { .. })
        ));
        assert!(matches!(
            apply_script_runtime_command(
                &mut state,
                command("pause", &["999999999999999999999999"]),
                default_inputs()
            ),
            Err(ScriptRuntimeCommandError::InvalidNumericToken { .. })
        ));
        assert!(matches!(
            apply_script_runtime_command(
                &mut state,
                command("pop", &["wScriptVar"]),
                default_inputs()
            ),
            Err(ScriptRuntimeCommandError::EmptyStack)
        ));
        assert!(state.script_runtime.effects.is_empty());
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
    fn verticalmenu_requires_loaded_menu_identity() {
        let mut state = GameState::default();
        let error = apply_script_runtime_command(
            &mut state,
            command("verticalmenu", &[]),
            default_inputs(),
        )
        .expect_err("verticalmenu requires loadmenu state");

        assert_eq!(
            error,
            ScriptRuntimeCommandError::MissingActiveMenu {
                command: "verticalmenu".to_string(),
            }
        );
        assert!(!state.script_runtime.window_open);
        assert!(state.script_runtime.effects.is_empty());

        apply_script_runtime_command(
            &mut state,
            command("loadmenu", &["RuntimeMenu"]),
            default_inputs(),
        )
        .expect("load menu");
        apply_script_runtime_command(&mut state, command("verticalmenu", &[]), default_inputs())
            .expect("vertical menu");

        assert_eq!(
            state.script_runtime.active_menu.as_deref(),
            Some("RuntimeMenu")
        );
        assert!(state.script_runtime.window_open);
        assert_eq!(
            state
                .script_runtime
                .effects
                .iter()
                .map(|effect| effect.command.as_str())
                .collect::<Vec<_>>(),
            vec!["loadmenu", "verticalmenu"]
        );
    }

    #[test]
    fn menu_coords_accepts_verified_screen_coordinate_expressions() {
        let mut state = GameState::default();

        apply_script_runtime_command(
            &mut state,
            command(
                "menu_coords",
                &["SCREEN_LEFT", "2", "SCREEN_WIDTH - 1", "TEXTBOX_Y - 1"],
            ),
            default_inputs(),
        )
        .expect("menu coords");

        assert_eq!(state.script_runtime.menu_coords, Some([0, 2, 19, 11]));
    }

    #[test]
    fn exported_script_numeric_parser_preserves_exact_asm_tokens() {
        assert_eq!(parse_script_i32_token("raw_script", "$10"), Ok(16));
        assert_eq!(parse_script_i32_token("raw_script", "%1010"), Ok(10));
        assert_eq!(parse_script_i32_token("raw_script", "-2"), Ok(-2));
        assert_eq!(parse_script_i32_token("raw_script", "+1"), Ok(1));
        assert!(matches!(
            parse_script_i32_token("raw_script", "0x10"),
            Err(ScriptRuntimeCommandError::InvalidNumericToken { .. })
        ));
    }

    #[test]
    fn menu_coords_rejects_unknown_coordinate_symbols_without_fallback() {
        let mut state = GameState::default();

        let error = apply_script_runtime_command(
            &mut state,
            command("menu_coords", &["0", "0", "SCREEN_EDGE + LEFT", "8"]),
            default_inputs(),
        )
        .expect_err("unknown coordinate expression rejected");

        assert_eq!(
            error,
            ScriptRuntimeCommandError::UnknownNumericToken {
                command: "menu_coords".to_string(),
                token: "LEFT".to_string(),
            }
        );
        assert_eq!(state.script_runtime.menu_coords, None);
    }

    #[test]
    fn menu_coords_rejects_padded_coordinate_expressions_without_normalization() {
        let mut state = GameState::default();

        let error = apply_script_runtime_command(
            &mut state,
            command(
                "menu_coords",
                &["SCREEN_LEFT", "2", "SCREEN_WIDTH  - 1", "TEXTBOX_Y"],
            ),
            default_inputs(),
        )
        .expect_err("padded coordinate expression rejected");

        assert_eq!(
            error,
            ScriptRuntimeCommandError::InvalidNumericToken {
                command: "menu_coords".to_string(),
                token: "SCREEN_WIDTH  - 1".to_string(),
            }
        );
        assert_eq!(state.script_runtime.menu_coords, None);

        let error = apply_script_runtime_command(
            &mut state,
            command(
                "menu_coords",
                &["SCREEN_LEFT", "2", "SCREEN_WIDTH\t-\t1", "TEXTBOX_Y"],
            ),
            default_inputs(),
        )
        .expect_err("tabbed coordinate expression rejected");

        assert_eq!(
            error,
            ScriptRuntimeCommandError::PaddedArg {
                command: "menu_coords".to_string(),
                arg: "SCREEN_WIDTH\t-\t1".to_string(),
            }
        );
        assert_eq!(state.script_runtime.menu_coords, None);
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
    fn apply_initialize_events_sets_pack_defined_flags_and_variable_sprites() {
        let mut state = GameState::default();
        let config = InitializeEventsConfig {
            event_flags: vec![
                "EVENT_GOT_STARTER".to_string(),
                "EVENT_INITIALIZED_EVENTS".to_string(),
            ],
            engine_flags: vec!["ENGINE_POKEGEAR".to_string()],
            variable_sprites: [(
                "SPRITE_FUCHSIA_GYM_1".to_string(),
                "SPRITE_ROCKER".to_string(),
            )]
            .into_iter()
            .collect(),
        };

        apply_initialize_events(&mut state, &config).expect("apply initialize events");

        assert_eq!(state.flags.is_event_flag_set("EVENT_GOT_STARTER"), Ok(true));
        assert_eq!(
            state.flags.is_event_flag_set("EVENT_INITIALIZED_EVENTS"),
            Ok(true)
        );
        assert_eq!(state.flags.is_engine_flag_set("ENGINE_POKEGEAR"), Ok(true));
        assert_eq!(
            state
                .script_runtime
                .variable_sprites
                .get("SPRITE_FUCHSIA_GYM_1")
                .map(String::as_str),
            Some("SPRITE_ROCKER")
        );
        assert_eq!(state.script_runtime.next_script, None);
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

    #[test]
    fn script_runtime_serialized_variants_reject_unknown_fallback_fields() {
        let outcome_error = serde_json::from_value::<ScriptRuntimeOutcome>(serde_json::json!({
            "script_value_set": {
                "command": "random",
                "value": "3",
                "source_script": "RuntimeScript",
                "command_index": 7,
                "fallback_value": "0"
            }
        }))
        .expect_err("fallback script value must be rejected")
        .to_string();
        assert!(
            outcome_error.contains("unknown field `fallback_value`"),
            "{outcome_error}"
        );

        let command_error =
            serde_json::from_value::<ScriptRuntimeCommandError>(serde_json::json!({
                "UnknownCommand": {
                    "command": "check_version",
                    "normalized_command": "checkver"
                }
            }))
            .expect_err("normalized command must be rejected")
            .to_string();
        assert!(
            command_error.contains("unknown field `normalized_command`"),
            "{command_error}"
        );

        let issue_error = serde_json::from_value::<ScriptRuntimeCommandIssue>(serde_json::json!({
            "unknown_target": {
                "target_label": ".Missing",
                "legacy_target_label": "MissingScript"
            }
        }))
        .expect_err("legacy target label must be rejected")
        .to_string();
        assert!(
            issue_error.contains("unknown field `legacy_target_label`"),
            "{issue_error}"
        );
    }

    fn default_inputs() -> ScriptRuntimeInputs {
        ScriptRuntimeInputs::default()
    }
}
