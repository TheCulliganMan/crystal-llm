use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

use crate::random::{CrystalRandom, DividerSource};
use crate::state::{
    GameState, HALL_OF_FAME_ENTRY_LIMIT, HALL_OF_FAME_MASTER_COUNT, HALL_OF_FAME_TEAM_SIZE,
    HallOfFameEntry, HallOfFamePokemon, ScriptLocation, ScriptReturnFrame, ScriptRuntimeDelay,
    ScriptRuntimeEarthquake, ScriptRuntimeQueuedCommand, ScriptRuntimeStoneTableEntry,
};
use crate::systems::economy::{EconomyError, MoneyAccount};
use crate::systems::phone::insert_phone_number_in_first_open_slot;
use crate::timing::{wrapping_byte_counter_frames, wrapping_byte_counter_ticks};

pub const SCRIPT_RUNTIME_SPECIAL_PHONE_CALL_NONE: &str = "SPECIALCALL_NONE";
const PHONE_CALLER_MEMCALL_OPERAND: [&str; 3] =
    ["wCallerContact", "+", "PHONE_CONTACT_SCRIPT2_BANK"];
const PHONE_SCRIPT_BANK_MEMCALL_OPERAND: &str = "wPhoneScriptBank";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptRuntimeCommand {
    pub command: String,
    pub args: Vec<String>,
    pub source_script: String,
    pub command_index: usize,
}

/// Game Boy CPU condition tokens accepted by RGBDS control-flow opcodes.
///
/// CPU routines embedded beside script bytecode use these exact tokens for
/// conditional returns. Keeping them typed prevents an unknown condition from
/// being treated as either an unconditional return or a false branch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScriptRuntimeCpuCondition {
    Z,
    Nz,
    C,
    Nc,
}

impl ScriptRuntimeCpuCondition {
    pub fn from_asm_token(token: &str) -> Option<Self> {
        match token {
            "z" => Some(Self::Z),
            "nz" => Some(Self::Nz),
            "c" => Some(Self::C),
            "nc" => Some(Self::Nc),
            _ => None,
        }
    }

    pub fn is_met(self, zero: bool, carry: bool) -> bool {
        match self {
            Self::Z => zero,
            Self::Nz => !zero,
            Self::C => carry,
            Self::Nc => !carry,
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptRuntimeInputs {
    pub selected_party_index: Option<usize>,
    pub game_version: Option<String>,
    pub current_landmark_name: Option<String>,
    pub resolved_named_buffer_value: Option<String>,
    pub resolved_stone_table_entries: Option<Vec<ScriptRuntimeStoneTableEntry>>,
    pub resolved_decoration: Option<ScriptRuntimeDecorationResolution>,
    pub gift_original_trainer_name: Option<String>,
    pub gift_original_trainer_id: Option<u16>,
    pub gift_nickname_accepted: Option<bool>,
    pub gift_nickname: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptRuntimeDecorationResolution {
    pub target_script: String,
    pub string_buffer_3: Option<String>,
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
    // InitDecorations runs as part of InitializeEvents before its event flag
    // is set. These are the exact initial wDeco* bytes from decorations.asm.
    state
        .script_runtime
        .memory
        .insert("wDecoBed".to_string(), "DECO_FEATHERY_BED".to_string());
    state
        .script_runtime
        .memory
        .insert("wDecoPoster".to_string(), "DECO_TOWN_MAP".to_string());
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
    PhoneCallasmPresentation {
        effect: ScriptPhoneCallasmPresentation,
        source_script: String,
        command_index: usize,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScriptPhoneCallasmPresentation {
    RingTwice,
    HangUp,
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
    #[error("script runtime command '{command}' has invalid string buffer '{buffer}'")]
    InvalidStringBuffer { command: String, buffer: String },
    #[error("script runtime getmoney account is invalid '{account}'")]
    InvalidMoneyAccount { account: String },
    #[error("script runtime getmoney account is unknown '{account}'")]
    UnknownMoneyAccount { account: String },
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
    #[error("script runtime command 'random' requires an injected divider source")]
    RandomRequiresDivider,
    #[error("script runtime command 'getcurlandmarkname' requires the current map landmark name")]
    MissingCurrentLandmarkName,
    #[error("script runtime command '{command}' requires a resolved named-buffer value")]
    MissingResolvedNamedBufferValue { command: String },
    #[error("script runtime command 'writecmdqueue' requires a resolved stone-table queue")]
    MissingResolvedStoneTableQueue,
    #[error("script runtime command 'describedecoration' requires a resolved decoration script")]
    MissingResolvedDecoration,
    #[error("resolved decoration name is empty")]
    EmptyResolvedDecorationName,
    #[error("script random executor received non-random command '{command}'")]
    NonRandomCommandAtRandomBoundary { command: String },
    #[error("script runtime command 'random' bound {bound} does not fit GetScriptByte")]
    RandomBoundOutOfByteRange { bound: u32 },
    #[error("script runtime command 'random' divider read failed: {message}")]
    RandomDivider { message: String },
    #[error("script runtime command 'checkver' requires explicit game version input")]
    MissingGameVersion,
    #[error("script runtime command '{command}' requires source constant {constant}")]
    MissingSourceConstant { command: String, constant: String },
    #[error("script runtime command '{command}' source constant {constant}={value} is not a u16")]
    InvalidSourceConstant {
        command: String,
        constant: String,
        value: i64,
    },
    #[error("script dispatch has invalid next script '{script}'")]
    InvalidNextScript { script: String },
    #[error("script dispatch has invalid last talked object '{object_identifier}'")]
    InvalidLastTalkedObject { object_identifier: String },
    #[error("script runtime memcall has unsupported pointer operand '{operand}'")]
    UnsupportedMemcallOperand { operand: String },
    #[error("script runtime memcall requires wPhoneCallerScript")]
    MissingPhoneCallerScript,
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
    origin_map_name: &str,
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
    state.script_runtime.next_script = Some(ScriptLocation {
        origin_map_name: origin_map_name.to_string(),
        script: next_script.to_string(),
    });
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
    pub trainer_class_names: BTreeSet<String>,
    pub items: BTreeSet<String>,
    pub pokemon: BTreeSet<String>,
    pub phone_contacts: BTreeSet<String>,
    pub special_phone_calls: BTreeSet<String>,
    pub npc_trades: BTreeSet<String>,
    pub landmarks: BTreeSet<String>,
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
            trainer_class_names: BTreeSet<String>,
            items: BTreeSet<String>,
            pokemon: BTreeSet<String>,
            phone_contacts: BTreeSet<String>,
            special_phone_calls: BTreeSet<String>,
            npc_trades: BTreeSet<String>,
            landmarks: BTreeSet<String>,
            script_labels: BTreeSet<String>,
        }

        let raw = RawCatalog::deserialize(deserializer)?;
        validate_runtime_pack_id_set("script_runtime.special_routines", &raw.special_routines)
            .map_err(serde::de::Error::custom)?;
        validate_runtime_pack_id_map("script_runtime.trainer_classes", &raw.trainer_classes)
            .map_err(serde::de::Error::custom)?;
        validate_runtime_pack_id_set(
            "script_runtime.trainer_class_names",
            &raw.trainer_class_names,
        )
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
        validate_runtime_landmark_id_set("script_runtime.landmarks", &raw.landmarks)
            .map_err(serde::de::Error::custom)?;
        for label in &raw.script_labels {
            require_runtime_label("script_runtime.script_labels", label)
                .map_err(serde::de::Error::custom)?;
        }
        Ok(Self {
            special_routines: raw.special_routines,
            trainer_classes: raw.trainer_classes,
            trainer_class_names: raw.trainer_class_names,
            items: raw.items,
            pokemon: raw.pokemon,
            phone_contacts: raw.phone_contacts,
            special_phone_calls: raw.special_phone_calls,
            npc_trades: raw.npc_trades,
            landmarks: raw.landmarks,
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
    UnknownTrainerClassName {
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
    UnknownLandmark {
        landmark_id: String,
    },
    InvalidLandmark {
        landmark_id: String,
    },
    UnknownTarget {
        target_label: String,
    },
    InvalidTarget {
        target_label: String,
    },
}

pub const SCRIPT_RUNTIME_USE_SCRIPT_VAR_ID: &str = "USE_SCRIPT_VAR";

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

fn validate_runtime_landmark_id_set(field: &str, values: &BTreeSet<String>) -> Result<(), String> {
    validate_runtime_pack_id_set(field, values)?;
    for value in values {
        if !value.starts_with("LANDMARK_") {
            return Err(format!(
                "{field} values must use exact LANDMARK_* ids, found {value:?}"
            ));
        }
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
        "gettrainername" | "loadtrainer" => {
            let (trainer_class, trainer_id) = if command.command == "gettrainername" {
                (&command.args[1], &command.args[2])
            } else {
                (&command.args[0], &command.args[1])
            };
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
        "gettrainerclassname" => {
            let trainer_class = &command.args[1];
            if !is_exact_nonempty_runtime_pack_id(trainer_class) {
                issues.push(ScriptRuntimeCommandIssue::InvalidTrainerClass {
                    trainer_class: trainer_class.clone(),
                });
            } else if !catalog.trainer_class_names.contains(trainer_class) {
                issues.push(ScriptRuntimeCommandIssue::UnknownTrainerClassName {
                    trainer_class: trainer_class.clone(),
                });
            }
        }
        "winlosstext" => {
            for target_label in &command.args {
                if target_label != "0" && target_label != "-1" {
                    push_unknown_runtime_target_issue(command, target_label, catalog, &mut issues);
                }
            }
        }
        "getitemname" => {
            let item_id = &command.args[1];
            if item_id != SCRIPT_RUNTIME_USE_SCRIPT_VAR_ID {
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
        "getmonname" | "loadwildmon" => {
            let species_id = if command.command == "getmonname" {
                &command.args[1]
            } else {
                &command.args[0]
            };
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
        "getlandmarkname" => {
            let landmark_id = &command.args[1];
            if !is_exact_nonempty_runtime_pack_id(landmark_id)
                || !landmark_id.starts_with("LANDMARK_")
            {
                issues.push(ScriptRuntimeCommandIssue::InvalidLandmark {
                    landmark_id: landmark_id.clone(),
                });
            } else if !catalog.landmarks.contains(landmark_id) {
                issues.push(ScriptRuntimeCommandIssue::UnknownLandmark {
                    landmark_id: landmark_id.clone(),
                });
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
        "writecmdqueue" | "elevator" | "callasm" | "checkpokemail" | "givepokemail" => {
            let target_label = &command.args[0];
            push_unknown_runtime_target_issue(command, target_label, catalog, &mut issues);
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
    if target_label.starts_with('.') {
        let parent_script = script_label_parent(source_script);
        let local = if target_label.contains('@') {
            (script_label_parent(target_label) == parent_script)
                .then(|| target_label.to_string())?
        } else {
            format!("{target_label}@{parent_script}")
        };
        return script_labels.contains(&local).then_some(local);
    }
    if script_labels.contains(target_label) {
        return Some(target_label.to_string());
    }
    None
}

pub fn script_label_parent(source_script: &str) -> &str {
    source_script
        .rsplit_once('@')
        .map(|(_, parent)| parent)
        .unwrap_or(source_script)
}

pub fn apply_script_runtime_command_in_map(
    state: &mut GameState,
    origin_map_name: &str,
    command: ScriptRuntimeCommand,
    inputs: ScriptRuntimeInputs,
    constants: &StoryEventScriptConstants,
) -> Result<ScriptRuntimeOutcome, ScriptRuntimeCommandError> {
    validate_script_runtime_command(&command)?;
    let outcome = match command.command.as_str() {
        "addval" => {
            let left = parse_required_accumulator(state, &command)?;
            let right = parse_i32_token(&command.command, &command.args[0])?;
            let value = (i64::from(left) + i64::from(right)).rem_euclid(256) as u8;
            set_script_value(state, &command, value.to_string())
        }
        "random" => return Err(ScriptRuntimeCommandError::RandomRequiresDivider),
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
        "checkpokemail" => {
            apply_runtime_effect(state, origin_map_name, &command, constants)?;
            set_script_value(state, &command, "2".to_string())
        }
        "checkver" => {
            let value = inputs
                .game_version
                .ok_or(ScriptRuntimeCommandError::MissingGameVersion)?;
            set_script_value(state, &command, value)
        }
        "getcurlandmarkname" => {
            let landmark_name = inputs
                .current_landmark_name
                .ok_or(ScriptRuntimeCommandError::MissingCurrentLandmarkName)?;
            state
                .script_runtime
                .named_buffers
                .insert(command.args[0].clone(), landmark_name);
            ScriptRuntimeOutcome::EffectRecorded {
                command: command.command.clone(),
                source_script: command.source_script.clone(),
                command_index: command.command_index,
            }
        }
        "gettrainername"
        | "gettrainerclassname"
        | "getitemname"
        | "getmonname"
        | "getstring"
        | "getlandmarkname" => {
            let value = inputs.resolved_named_buffer_value.ok_or_else(|| {
                ScriptRuntimeCommandError::MissingResolvedNamedBufferValue {
                    command: command.command.clone(),
                }
            })?;
            state
                .script_runtime
                .named_buffers
                .insert(command.args[0].clone(), value);
            ScriptRuntimeOutcome::EffectRecorded {
                command: command.command.clone(),
                source_script: command.source_script.clone(),
                command_index: command.command_index,
            }
        }
        "writecmdqueue" => {
            let entries = inputs
                .resolved_stone_table_entries
                .ok_or(ScriptRuntimeCommandError::MissingResolvedStoneTableQueue)?;
            state.script_runtime.stone_table_entries.extend(entries);
            ScriptRuntimeOutcome::EffectRecorded {
                command: command.command.clone(),
                source_script: command.source_script.clone(),
                command_index: command.command_index,
            }
        }
        "describedecoration" => {
            let resolved = inputs
                .resolved_decoration
                .ok_or(ScriptRuntimeCommandError::MissingResolvedDecoration)?;
            if !is_exact_nonempty_runtime_token(&resolved.target_script) {
                return Err(ScriptRuntimeCommandError::InvalidNextScript {
                    script: resolved.target_script,
                });
            }
            if let Some(name) = resolved.string_buffer_3 {
                if name.is_empty() {
                    return Err(ScriptRuntimeCommandError::EmptyResolvedDecorationName);
                }
                state
                    .script_runtime
                    .named_buffers
                    .insert("STRING_BUFFER_3".to_string(), name);
            }
            state.script_runtime.next_script = Some(ScriptLocation {
                origin_map_name: origin_map_name.to_string(),
                script: resolved.target_script,
            });
            ScriptRuntimeOutcome::EffectRecorded {
                command: command.command.clone(),
                source_script: command.source_script.clone(),
                command_index: command.command_index,
            }
        }
        "checkphonecall" => set_script_value(
            state,
            &command,
            u8::from(state.script_runtime.special_phone_call.is_some()).to_string(),
        ),
        _ => {
            apply_runtime_effect(state, origin_map_name, &command, constants)?;
            ScriptRuntimeOutcome::EffectRecorded {
                command: command.command.clone(),
                source_script: command.source_script.clone(),
                command_index: command.command_index,
            }
        }
    };

    Ok(outcome)
}

pub fn apply_script_random_command_in_map<S>(
    state: &mut GameState,
    command: ScriptRuntimeCommand,
    divider: &mut S,
) -> Result<ScriptRuntimeOutcome, ScriptRuntimeCommandError>
where
    S: DividerSource + ?Sized,
    S::Error: std::fmt::Display,
{
    validate_script_runtime_command(&command)?;
    if command.command != "random" {
        return Err(
            ScriptRuntimeCommandError::NonRandomCommandAtRandomBoundary {
                command: command.command,
            },
        );
    }
    let bound = parse_u32_token(&command.command, &command.args[0])?;
    let bound = u8::try_from(bound)
        .map_err(|_| ScriptRuntimeCommandError::RandomBoundOutOfByteRange { bound })?;
    let value = if bound == 0 {
        0
    } else {
        let mut rng = CrystalRandom::new(state.random_state, divider);
        let value = rng.script_random_range(bound).map_err(|error| {
            ScriptRuntimeCommandError::RandomDivider {
                message: error.to_string(),
            }
        })?;
        state.random_state = rng.state();
        value
    };
    Ok(set_script_value(state, &command, value.to_string()))
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
    let arity_is_valid = if command.command == "memcall" {
        matches!(command.args.len(), 1 | 3)
    } else {
        command.args.len() == expected
    };
    if !arity_is_valid {
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
    if matches!(
        command.command.as_str(),
        "gettrainername"
            | "gettrainerclassname"
            | "getitemname"
            | "getmonname"
            | "getstring"
            | "getcurlandmarkname"
            | "getlandmarkname"
            | "getmoney"
            | "getnum"
    ) {
        require_script_string_buffer(&command.command, &command.args[0])?;
    }
    if command.command == "getmoney" {
        parse_script_money_account(&command.args[1])?;
    }
    if command.command == "loadwildmon" {
        parse_u8_token(&command.command, &command.args[1])?;
    }
    if command.command == "memcall" {
        let args = command.args.iter().map(String::as_str).collect::<Vec<_>>();
        if args.as_slice() != PHONE_CALLER_MEMCALL_OPERAND
            && args.as_slice() != [PHONE_SCRIPT_BANK_MEMCALL_OPERAND]
        {
            return Err(ScriptRuntimeCommandError::UnsupportedMemcallOperand {
                operand: command.args.join(" "),
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

fn require_script_string_buffer(
    command: &str,
    buffer: &str,
) -> Result<(), ScriptRuntimeCommandError> {
    if matches!(
        buffer,
        "STRING_BUFFER_3" | "STRING_BUFFER_4" | "STRING_BUFFER_5"
    ) {
        Ok(())
    } else {
        Err(ScriptRuntimeCommandError::InvalidStringBuffer {
            command: command.to_string(),
            buffer: buffer.to_string(),
        })
    }
}

fn parse_script_money_account(account: &str) -> Result<MoneyAccount, ScriptRuntimeCommandError> {
    MoneyAccount::from_script_id(account).map_err(|error| match error {
        EconomyError::InvalidMoneyAccount { account } => {
            ScriptRuntimeCommandError::InvalidMoneyAccount { account }
        }
        EconomyError::UnknownMoneyAccount { account } => {
            ScriptRuntimeCommandError::UnknownMoneyAccount { account }
        }
        _ => ScriptRuntimeCommandError::UnknownMoneyAccount {
            account: account.to_string(),
        },
    })
}

fn apply_runtime_effect(
    state: &mut GameState,
    origin_map_name: &str,
    command: &ScriptRuntimeCommand,
    constants: &StoryEventScriptConstants,
) -> Result<(), ScriptRuntimeCommandError> {
    match command.command.as_str() {
        "special" => state.script_runtime.last_special_routine = Some(command.args[0].clone()),
        "pause" | "wait" | "deactivatefacing" => {
            let parameter = parse_u8_token(&command.command, &command.args[0])?;
            let frames_per_tick = match command.command.as_str() {
                "pause" => 2,
                "wait" => 6,
                "deactivatefacing" => 1,
                _ => unreachable!("matched delay command"),
            };
            state
                .script_runtime
                .pending_delays
                .push(ScriptRuntimeDelay {
                    command: command.command.clone(),
                    parameter: u16::from(parameter),
                    frames: wrapping_byte_counter_frames(parameter, frames_per_tick),
                    release_all_objects: command.command == "deactivatefacing",
                    source_script: command.source_script.clone(),
                    command_index: command.command_index,
                });
        }
        "earthquake" => {
            let parameter = u16::from(parse_u8_token(&command.command, &command.args[0])?);
            state
                .script_runtime
                .pending_earthquakes
                .push(ScriptRuntimeEarthquake {
                    parameter,
                    shake_frames: parameter,
                    sleep_frames: wrapping_byte_counter_ticks((parameter & 0x3f) as u8),
                    source_script: command.source_script.clone(),
                    command_index: command.command_index,
                });
        }
        // The exact emote id remains in the replayable runtime effect. The
        // corresponding show/hide operations are movement opcodes.
        "loademote" => {}
        "setlasttalked" => state.script_runtime.last_talked_object = Some(command.args[0].clone()),
        "variablesprite" => {
            state
                .script_runtime
                .variable_sprites
                .insert(command.args[0].clone(), command.args[1].clone());
        }
        "getmoney" => {
            let account = parse_script_money_account(&command.args[1])?;
            let value = match account {
                MoneyAccount::YourMoney => state.money,
                MoneyAccount::MomsMoney => state.moms_money,
            };
            state
                .script_runtime
                .named_buffers
                .insert(command.args[0].clone(), value.to_string());
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
        "dontrestartmapmusic" => state.script_runtime.map_music_restart_disabled = true,
        "playmapmusic" => state.script_runtime.map_music_requested = true,
        "wildoff" => {
            state
                .flags
                .engine_flags
                .insert("STATUSFLAGS_NO_WILD_ENCOUNTERS_F".to_string(), true);
        }
        "wildon" => {
            state
                .flags
                .engine_flags
                .insert("STATUSFLAGS_NO_WILD_ENCOUNTERS_F".to_string(), false);
        }
        "lock" => state.script_runtime.player_input_locked = true,
        "release" => state.script_runtime.player_input_locked = false,
        "lockall" => state.script_runtime.all_input_locked = true,
        "releaseall" => state.script_runtime.all_input_locked = false,
        "itemnotify" => state.script_runtime.item_notify_queued = true,
        // The pack-backed item boundary resolves the variable quantity,
        // performs ReceiveItem, fills STRING_BUFFER_4 with CurItemName, and
        // queues GiveItemScript. Neither operand is itself a string buffer.
        "verbosegiveitemvar" => {}
        "addcellnum" => {
            let contact_id = &command.args[0];
            let added = !state.script_runtime.phone_numbers.contains(contact_id)
                && insert_phone_number_in_first_open_slot(
                    &mut state.script_runtime.phone_number_order,
                    contact_id,
                );
            if added {
                state
                    .script_runtime
                    .phone_numbers
                    .insert(contact_id.clone());
            }
            state.script_runtime.script_value = Some(if added {
                "0".to_string()
            } else {
                "1".to_string()
            });
        }
        "specialphonecall" => {
            state.script_runtime.special_phone_call = (command.args[0]
                != SCRIPT_RUNTIME_SPECIAL_PHONE_CALL_NONE)
                .then(|| command.args[0].clone());
            state
                .script_runtime
                .variables
                .insert("VAR_SPECIALPHONECALL".to_string(), command.args[0].clone());
        }
        "pokepic" => state.script_runtime.active_pokemon_picture = Some(command.args[0].clone()),
        "closepokepic" => state.script_runtime.active_pokemon_picture = None,
        "trade" => {}
        "winlosstext" => {
            state
                .script_runtime
                .memory
                .insert("wWinTextPointer".to_string(), command.args[0].clone());
            state
                .script_runtime
                .memory
                .insert("wLossTextPointer".to_string(), command.args[1].clone());
        }
        "loadtrainer" => {
            state
                .script_runtime
                .memory
                .insert("wBattleScriptFlags".to_string(), "129".to_string());
            state
                .script_runtime
                .memory
                .insert("wOtherTrainerClass".to_string(), command.args[0].clone());
            state
                .script_runtime
                .memory
                .insert("wOtherTrainerID".to_string(), command.args[1].clone());
        }
        "loadwildmon" => {
            let level = parse_u8_token(&command.command, &command.args[1])?;
            state
                .script_runtime
                .memory
                .insert("wBattleScriptFlags".to_string(), "128".to_string());
            state
                .script_runtime
                .memory
                .insert("wTempWildMonSpecies".to_string(), command.args[0].clone());
            state
                .script_runtime
                .memory
                .insert("wCurPartyLevel".to_string(), level.to_string());
        }
        "memcall" => {
            let target_script = state
                .script_runtime
                .memory
                .get("wPhoneCallerScript")
                .cloned()
                .ok_or(ScriptRuntimeCommandError::MissingPhoneCallerScript)?;
            if !is_exact_nonempty_runtime_label(&target_script) {
                return Err(ScriptRuntimeCommandError::InvalidNextScript {
                    script: target_script,
                });
            }
            state.script_runtime.call_stack.push(ScriptReturnFrame {
                origin_map_name: origin_map_name.to_string(),
                source_script: command.source_script.clone(),
                next_command_index: command.command_index + 1,
            });
            state.script_runtime.next_script = Some(ScriptLocation {
                origin_map_name: origin_map_name.to_string(),
                script: target_script,
            });
            state.script_runtime.script_ended = None;
        }
        "randomwildmon" => {
            state
                .script_runtime
                .memory
                .insert("wBattleScriptFlags".to_string(), "0".to_string());
        }
        // The visible runtime executes CatchTutorial synchronously at this
        // command boundary, just as Script_catchtutorial calls the routine and
        // then Script_reloadmap. There is no saved command-history byte.
        "catchtutorial" => {}
        "warpsound" => state.script_runtime.warp_sound_queued = true,
        // `Script_blackoutmod` writes only wLastSpawnMapGroup/Number. The
        // pack boundary resolves that pair to the authoritative spawn id.
        "blackoutmod" => {}
        "battletowertext" => {
            let key = match command.args[0].as_str() {
                "BATTLETOWERTEXT_INTRO" => "battle_tower_intro_text",
                "BATTLETOWERTEXT_WIN_TEXT" => "battle_tower_win_text",
                "BATTLETOWERTEXT_LOSS_TEXT" => "battle_tower_loss_text",
                token => {
                    return Err(ScriptRuntimeCommandError::UnknownNumericToken {
                        command: command.command.clone(),
                        token: token.to_string(),
                    });
                }
            };
            let text_label = state
                .script_runtime
                .variables
                .get(key)
                .cloned()
                .ok_or_else(|| ScriptRuntimeCommandError::MissingAccumulator {
                    command: command.command.clone(),
                })?;
            state.script_runtime.text_window_open = true;
            state.script_runtime.active_text_label = Some(text_label.clone());
            state.script_runtime.pending_text_label = Some(text_label);
        }
        "halloffame" => record_hall_of_fame(
            state,
            required_source_u16_constant(constants, &command, "SPAWN_LANCE")?,
        ),
        "credits" => {
            // Script_credits calls RedCredits, which stores SPAWN_RED before
            // entering the credits program. Hall of Fame credits use the
            // separate `halloffame` command and store SPAWN_LANCE there.
            state.hall_of_fame.spawn_after_champion = Some(required_source_u16_constant(
                constants,
                &command,
                "SPAWN_RED",
            )?);
            state.script_runtime.credits_requested = true;
        }
        "writevar" => {
            let value = state.script_runtime.script_value.clone().ok_or_else(|| {
                ScriptRuntimeCommandError::MissingAccumulator {
                    command: command.command.clone(),
                }
            })?;
            let target = command.args[0].clone();
            let blue_card_balance = (target == "VAR_BLUECARDBALANCE")
                .then(|| parse_u8_token(&command.command, &value))
                .transpose()?;
            state
                .script_runtime
                .variables
                .insert(target.clone(), value.clone());
            if let Some(balance) = blue_card_balance {
                state.blue_card_balance = balance;
            }
        }
        "getnum" => {
            let value = state.script_runtime.script_value.clone().ok_or_else(|| {
                ScriptRuntimeCommandError::MissingAccumulator {
                    command: command.command.clone(),
                }
            })?;
            let parsed = parse_u8_token(&command.command, &value)?;
            let rendered = parsed.to_string();
            let target_buffer = command.args[0].clone();
            state
                .script_runtime
                .named_buffers
                .insert(target_buffer.clone(), rendered.clone());
        }
        "describedecoration" => unreachable!("describedecoration is resolved before effects"),
        "elevator" | "callasm" | "checkpokemail" | "givepokemail" => {
            state
                .script_runtime
                .command_queue
                .push(ScriptRuntimeQueuedCommand {
                    origin_map_name: origin_map_name.to_string(),
                    command: command.command.clone(),
                    bank: None,
                    target: command.args[0].clone(),
                    source_script: command.source_script.clone(),
                    command_index: command.command_index,
                });
        }
        "_2dmenu" => state.script_runtime.menu_2d_requested = true,
        other => {
            return Err(ScriptRuntimeCommandError::UnknownCommand {
                command: other.to_string(),
            });
        }
    }
    Ok(())
}

/// Implements `halloffame` as a state mutation rather than a presentation-only
/// request. Crystal snapshots the current party into SRAM before starting the
/// Hall of Fame animation, saturates the win counter at 200, and records the
/// newest team at the front of a bounded history.
fn required_source_u16_constant(
    constants: &StoryEventScriptConstants,
    command: &ScriptRuntimeCommand,
    constant: &str,
) -> Result<u16, ScriptRuntimeCommandError> {
    let value = constants.global.get(constant).ok_or_else(|| {
        ScriptRuntimeCommandError::MissingSourceConstant {
            command: command.command.clone(),
            constant: constant.to_string(),
        }
    })?;
    u16::try_from(*value).map_err(|_| ScriptRuntimeCommandError::InvalidSourceConstant {
        command: command.command.clone(),
        constant: constant.to_string(),
        value: *value,
    })
}

fn record_hall_of_fame(state: &mut GameState, spawn_after_champion: u16) {
    // Script_halloffame clears GAME_TIMER_COUNTING_F before entering the
    // HallOfFame farcall. HallOfFame itself raises wGameLogicPaused only for
    // its save/recording work and clears it before the animation/credits.
    state.set_game_timer_counting(false);
    state.set_game_logic_paused(true);
    let count = state.hall_of_fame.count;
    let next_count = if count < HALL_OF_FAME_MASTER_COUNT {
        count.saturating_add(1)
    } else {
        count
    };
    let mut team: [Option<HallOfFamePokemon>; HALL_OF_FAME_TEAM_SIZE] =
        std::array::from_fn(|_| None);
    let mut slot = 0usize;
    for pokemon in state.storage.party.pokemon.iter().flatten() {
        if slot >= HALL_OF_FAME_TEAM_SIZE
            || pokemon.is_egg
            || pokemon.status.as_deref() == Some("EGG")
            || pokemon.species.id == "EGG"
        {
            continue;
        }
        let dvs = (u16::from(pokemon.dvs.attack & 0x0f) << 12)
            | (u16::from(pokemon.dvs.defense & 0x0f) << 8)
            | (u16::from(pokemon.dvs.speed & 0x0f) << 4)
            | u16::from(pokemon.dvs.special & 0x0f);
        team[slot] = Some(HallOfFamePokemon {
            species: pokemon.species.id.clone(),
            trainer_id: pokemon.original_trainer_id,
            dvs,
            level: pokemon.level,
            nickname: pokemon.nickname.chars().take(10).collect(),
        });
        slot += 1;
    }
    state.hall_of_fame.entries.insert(
        0,
        HallOfFameEntry {
            win_count: next_count,
            team,
        },
    );
    state
        .hall_of_fame
        .entries
        .truncate(HALL_OF_FAME_ENTRY_LIMIT);
    state.hall_of_fame.count = next_count;
    state.hall_of_fame.spawn_after_champion = Some(spawn_after_champion);
    state
        .flags
        .engine_flags
        .insert("STATUSFLAGS_HALL_OF_FAME_F".to_string(), true);
    state.set_game_logic_paused(false);
    state.script_runtime.hall_of_fame_requested = true;
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

fn parse_u8_token(command: &str, token: &str) -> Result<u8, ScriptRuntimeCommandError> {
    let value = parse_i32_token(command, token)?;
    u8::try_from(value).map_err(|_| ScriptRuntimeCommandError::UnknownNumericToken {
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
    if let Some(value) = script_numeric_symbol(token) {
        return Ok(value);
    }
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

fn script_numeric_symbol(token: &str) -> Option<i32> {
    match token {
        "BATTLETOWER_REWARD_QUANTITY" => Some(5),
        _ => None,
    }
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
        ("deactivatefacing", 1),
        ("earthquake", 1),
        ("loademote", 1),
        ("setlasttalked", 1),
        ("variablesprite", 2),
        ("gettrainername", 3),
        ("gettrainerclassname", 2),
        ("getitemname", 2),
        ("getmonname", 2),
        ("loadmenu", 1),
        ("verticalmenu", 0),
        ("closewindow", 0),
        ("dontrestartmapmusic", 0),
        ("playmapmusic", 0),
        ("wildoff", 0),
        ("wildon", 0),
        ("lock", 0),
        ("release", 0),
        ("lockall", 0),
        ("releaseall", 0),
        ("itemnotify", 0),
        ("addval", 1),
        ("verbosegiveitemvar", 2),
        ("getstring", 2),
        ("getcurlandmarkname", 1),
        ("getlandmarkname", 2),
        ("getmoney", 2),
        ("checkphonecall", 0),
        ("addcellnum", 1),
        ("specialphonecall", 1),
        ("checkpoke", 1),
        ("pokepic", 1),
        ("closepokepic", 0),
        ("trade", 1),
        ("winlosstext", 2),
        ("loadtrainer", 2),
        ("loadwildmon", 2),
        ("memcall", 1),
        ("randomwildmon", 0),
        ("catchtutorial", 1),
        ("warpsound", 0),
        ("blackoutmod", 1),
        ("wait", 1),
        ("random", 1),
        ("battletowertext", 1),
        ("halloffame", 0),
        ("credits", 0),
        ("describedecoration", 1),
        ("checkver", 0),
        ("writecmdqueue", 1),
        ("elevator", 1),
        ("_2dmenu", 0),
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
    use crate::models::{BaseStats, Dv, Pokemon, PokemonSpecies};
    use crate::state::PartyPokemonRef;

    fn apply_script_runtime_command(
        state: &mut GameState,
        command: ScriptRuntimeCommand,
        inputs: ScriptRuntimeInputs,
    ) -> Result<ScriptRuntimeOutcome, ScriptRuntimeCommandError> {
        let constants = StoryEventScriptConstants {
            global: BTreeMap::from([("SPAWN_LANCE".to_string(), 1), ("SPAWN_RED".to_string(), 2)]),
            maps: BTreeMap::new(),
        };
        apply_script_runtime_command_in_map(state, "TestMap", command, inputs, &constants)
    }

    fn command(name: &str, args: &[&str]) -> ScriptRuntimeCommand {
        ScriptRuntimeCommand {
            command: name.to_string(),
            args: args.iter().map(|arg| (*arg).to_string()).collect(),
            source_script: "RuntimeScript".to_string(),
            command_index: 4,
        }
    }

    #[test]
    fn wildon_and_wildoff_toggle_crystals_global_encounter_flag() {
        let mut state = GameState::default();
        apply_script_runtime_command(&mut state, command("wildoff", &[]), default_inputs())
            .expect("wildoff applies");
        assert_eq!(
            state
                .flags
                .engine_flags
                .get("STATUSFLAGS_NO_WILD_ENCOUNTERS_F"),
            Some(&true)
        );

        apply_script_runtime_command(&mut state, command("wildon", &[]), default_inputs())
            .expect("wildon applies");
        assert_eq!(
            state
                .flags
                .engine_flags
                .get("STATUSFLAGS_NO_WILD_ENCOUNTERS_F"),
            Some(&false)
        );
    }

    #[test]
    fn interaction_script_dispatch_commits_state_and_session_target() {
        let mut state = GameState::default();
        let mut last_talked_object = None;

        let outcome = commit_interaction_script_dispatch(
            &mut state,
            &mut last_talked_object,
            "Route36",
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
            state.script_runtime.next_script,
            Some(ScriptLocation {
                origin_map_name: "Route36".to_string(),
                script: "Route36SudowoodoScript".to_string(),
            })
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
            "Route36",
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
        assert_eq!(counts.get("deactivatefacing"), Some(&1));
        assert_eq!(counts.get("loademote"), Some(&1));
        assert!(!counts.contains_key("stop"));
        assert!(!counts.contains_key("push"));
        assert!(!counts.contains_key("pop"));
        assert!(!counts.contains_key("ret"));
        assert!(!counts.contains_key("ld"));
        assert!(!counts.contains_key("ldh"));
        assert!(!counts.contains_key("teleport_from"));
        assert!(!counts.contains_key("cmdqueue"));
        assert!(!counts.contains_key("stonetable"));
        assert!(!counts.contains_key("menu_coords"));
        assert!(!counts.contains_key("elevfloor"));
        assert!(!counts.contains_key("conditional_event"));
        assert!(!counts.contains_key("dw"));
        assert!(!counts.contains_key("dn"));
        assert!(!counts.contains_key("dba"));
        assert!(!counts.contains_key("dbw"));
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
        assert_eq!(
            validate_script_runtime_command(&command("stop", &[])),
            Err(ScriptRuntimeCommandError::UnknownCommand {
                command: "stop".to_string(),
            })
        );
        for cpu_instruction in ["push", "pop"] {
            assert_eq!(
                validate_script_runtime_command(&command(cpu_instruction, &["af"])),
                Err(ScriptRuntimeCommandError::UnknownCommand {
                    command: cpu_instruction.to_string(),
                })
            );
        }
    }

    #[test]
    fn battletowertext_queues_only_the_authored_print_text_boundary() {
        let mut state = GameState::default();
        state.script_runtime.variables.insert(
            "battle_tower_intro_text".to_string(),
            "BattleTowerTrainerText1".to_string(),
        );

        apply_script_runtime_command(
            &mut state,
            command("battletowertext", &["BATTLETOWERTEXT_INTRO"]),
            default_inputs(),
        )
        .expect("queue Battle Tower intro text");

        assert_eq!(
            state.script_runtime.pending_text_label.as_deref(),
            Some("BattleTowerTrainerText1")
        );
        let serialized = serde_json::to_value(&state.script_runtime)
            .expect("serialize script runtime after battletowertext");
        assert!(
            serialized.get("battle_tower_text").is_none(),
            "battletowertext must not create a second host-only runtime flag: {serialized}"
        );
    }

    #[test]
    fn trainer_battle_setup_commands_validate_and_write_exact_wram_symbols() {
        let catalog = ScriptRuntimeReferenceCatalog {
            trainer_classes: BTreeMap::from([("VANCE3".to_string(), "BIRD_KEEPER".to_string())]),
            script_labels: BTreeSet::from(["BirdKeeperVance1BeatenText".to_string()]),
            ..ScriptRuntimeReferenceCatalog::default()
        };
        let win_loss = command("winlosstext", &["BirdKeeperVance1BeatenText", "0"]);
        let load_trainer = command("loadtrainer", &["BIRD_KEEPER", "VANCE3"]);

        assert_eq!(script_runtime_command_issues(&win_loss, &catalog), []);
        assert_eq!(script_runtime_command_issues(&load_trainer, &catalog), []);

        let mut state = GameState::default();
        apply_script_runtime_command(&mut state, win_loss, default_inputs())
            .expect("winlosstext writes the canonical pointers");
        apply_script_runtime_command(&mut state, load_trainer, default_inputs())
            .expect("loadtrainer writes the canonical battle setup");

        assert_eq!(
            state
                .script_runtime
                .memory
                .get("wWinTextPointer")
                .map(String::as_str),
            Some("BirdKeeperVance1BeatenText")
        );
        assert_eq!(
            state
                .script_runtime
                .memory
                .get("wLossTextPointer")
                .map(String::as_str),
            Some("0")
        );
        assert_eq!(
            state
                .script_runtime
                .memory
                .get("wBattleScriptFlags")
                .map(String::as_str),
            Some("129")
        );
        assert_eq!(
            state
                .script_runtime
                .memory
                .get("wOtherTrainerClass")
                .map(String::as_str),
            Some("BIRD_KEEPER")
        );
        assert_eq!(
            state
                .script_runtime
                .memory
                .get("wOtherTrainerID")
                .map(String::as_str),
            Some("VANCE3")
        );
    }

    #[test]
    fn randomwildmon_clears_only_the_canonical_battle_script_flags_byte() {
        let runtime_command = command("randomwildmon", &[]);
        assert_eq!(validate_script_runtime_command(&runtime_command), Ok(()));

        let mut state = GameState::default();
        state
            .script_runtime
            .memory
            .insert("wBattleScriptFlags".to_string(), "129".to_string());
        state
            .script_runtime
            .memory
            .insert("wTempWildMonSpecies".to_string(), "GEODUDE".to_string());

        apply_script_runtime_command(&mut state, runtime_command, default_inputs())
            .expect("randomwildmon applies exact script setup");

        assert_eq!(
            state
                .script_runtime
                .memory
                .get("wBattleScriptFlags")
                .map(String::as_str),
            Some("0")
        );
        assert_eq!(
            state
                .script_runtime
                .memory
                .get("wTempWildMonSpecies")
                .map(String::as_str),
            Some("GEODUDE")
        );
    }

    #[test]
    fn loadwildmon_sets_the_exact_scripted_wild_battle_bytes() {
        let runtime_command = command("loadwildmon", &["SUDOWOODO", "20"]);
        assert_eq!(validate_script_runtime_command(&runtime_command), Ok(()));
        assert!(matches!(
            validate_script_runtime_command(&command("loadwildmon", &["SUDOWOODO", "256"])),
            Err(ScriptRuntimeCommandError::UnknownNumericToken { command, token })
                if command == "loadwildmon" && token == "256"
        ));
        let catalog = ScriptRuntimeReferenceCatalog {
            pokemon: BTreeSet::from(["SUDOWOODO".to_string()]),
            ..ScriptRuntimeReferenceCatalog::default()
        };
        assert_eq!(
            script_runtime_command_issues(&runtime_command, &catalog),
            []
        );
        assert_eq!(
            script_runtime_command_issues(&command("loadwildmon", &["MISSINGNO", "20"]), &catalog,),
            [ScriptRuntimeCommandIssue::UnknownSpecies {
                species_id: "MISSINGNO".to_string(),
            }]
        );

        let mut state = GameState::default();
        state
            .script_runtime
            .memory
            .insert("wBattleScriptFlags".to_string(), "0".to_string());
        state
            .script_runtime
            .memory
            .insert("wTempWildMonSpecies".to_string(), "GEODUDE".to_string());
        state
            .script_runtime
            .memory
            .insert("wCurPartyLevel".to_string(), "99".to_string());

        apply_script_runtime_command(&mut state, runtime_command, default_inputs())
            .expect("loadwildmon applies the exact scripted encounter setup");

        assert_eq!(
            state
                .script_runtime
                .memory
                .get("wBattleScriptFlags")
                .map(String::as_str),
            Some("128")
        );
        assert_eq!(
            state
                .script_runtime
                .memory
                .get("wTempWildMonSpecies")
                .map(String::as_str),
            Some("SUDOWOODO")
        );
        assert_eq!(
            state
                .script_runtime
                .memory
                .get("wCurPartyLevel")
                .map(String::as_str),
            Some("20")
        );
    }

    #[test]
    fn memcall_calls_the_dynamic_phone_script_and_retains_the_return_frame() {
        let runtime_command = command(
            "memcall",
            &["wCallerContact", "+", "PHONE_CONTACT_SCRIPT2_BANK"],
        );
        assert_eq!(
            validate_script_runtime_command(&command("memcall", &["wPhoneScriptBank"])),
            Ok(())
        );
        assert!(matches!(
            validate_script_runtime_command(&command("memcall", &["wUnknownPointer"])),
            Err(ScriptRuntimeCommandError::UnsupportedMemcallOperand { operand })
                if operand == "wUnknownPointer"
        ));
        let mut missing = GameState::default();
        let before_missing = missing.clone();
        assert_eq!(
            apply_script_runtime_command(&mut missing, runtime_command.clone(), default_inputs(),),
            Err(ScriptRuntimeCommandError::MissingPhoneCallerScript)
        );
        assert_eq!(missing, before_missing);

        let mut state = GameState::default();
        state.script_runtime.memory.insert(
            "wPhoneCallerScript".to_string(),
            "BillPhoneScript1".to_string(),
        );

        apply_script_runtime_command(&mut state, runtime_command, default_inputs())
            .expect("memcall enters the exact dynamically selected caller");

        assert_eq!(
            state.script_runtime.call_stack,
            [crate::state::ScriptReturnFrame {
                origin_map_name: "TestMap".to_string(),
                source_script: "RuntimeScript".to_string(),
                next_command_index: 5,
            }]
        );
        assert_eq!(
            state.script_runtime.next_script,
            Some(ScriptLocation {
                origin_map_name: "TestMap".to_string(),
                script: "BillPhoneScript1".to_string(),
            })
        );
        assert_eq!(state.script_runtime.script_ended, None);
    }

    #[test]
    fn landmark_and_money_buffer_commands_validate_typed_operands_and_write_exact_values() {
        let catalog = ScriptRuntimeReferenceCatalog {
            landmarks: BTreeSet::from(["LANDMARK_ROUTE_32".to_string()]),
            ..ScriptRuntimeReferenceCatalog::default()
        };
        let landmark = command("getlandmarkname", &["STRING_BUFFER_5", "LANDMARK_ROUTE_32"]);
        let money = command("getmoney", &["STRING_BUFFER_3", "MOMS_MONEY"]);

        assert_eq!(script_runtime_command_issues(&landmark, &catalog), []);
        assert_eq!(script_runtime_command_issues(&money, &catalog), []);
        assert_eq!(
            script_runtime_command_issues(
                &command("getlandmarkname", &["STRING_BUFFER_5", "LANDMARK_ROUTE_99"],),
                &catalog,
            ),
            vec![ScriptRuntimeCommandIssue::UnknownLandmark {
                landmark_id: "LANDMARK_ROUTE_99".to_string(),
            }]
        );
        assert_eq!(
            script_runtime_command_issues(
                &command("getlandmarkname", &["STRING_BUFFER_5", "ROUTE_32"]),
                &catalog,
            ),
            vec![ScriptRuntimeCommandIssue::InvalidLandmark {
                landmark_id: "ROUTE_32".to_string(),
            }]
        );
        assert_eq!(
            validate_script_runtime_command(&command(
                "getlandmarkname",
                &["STRING_BUFFER_2", "LANDMARK_ROUTE_32"],
            )),
            Err(ScriptRuntimeCommandError::InvalidStringBuffer {
                command: "getlandmarkname".to_string(),
                buffer: "STRING_BUFFER_2".to_string(),
            })
        );
        assert_eq!(
            validate_script_runtime_command(&command(
                "getmoney",
                &["STRING_BUFFER_3", "moms_money"],
            )),
            Err(ScriptRuntimeCommandError::UnknownMoneyAccount {
                account: "moms_money".to_string(),
            })
        );

        let mut state = GameState {
            moms_money: 654_321,
            ..GameState::default()
        };
        apply_script_runtime_command(&mut state, money, default_inputs())
            .expect("getmoney formats the exact selected account");
        assert_eq!(
            state
                .script_runtime
                .named_buffers
                .get("STRING_BUFFER_3")
                .map(String::as_str),
            Some("654321")
        );
    }

    #[test]
    fn current_landmark_name_requires_exact_map_derived_input_and_buffer() {
        let runtime_command = command("getcurlandmarkname", &["STRING_BUFFER_3"]);
        let mut state = GameState::default();
        let mut inputs = default_inputs();
        inputs.current_landmark_name = Some("VIOLET CITY".to_string());

        apply_script_runtime_command(&mut state, runtime_command.clone(), inputs)
            .expect("getcurlandmarkname writes the injected current landmark");
        assert_eq!(
            state
                .script_runtime
                .named_buffers
                .get("STRING_BUFFER_3")
                .map(String::as_str),
            Some("VIOLET CITY")
        );

        let state_before_missing = GameState::default();
        let mut missing = state_before_missing.clone();
        assert_eq!(
            apply_script_runtime_command(&mut missing, runtime_command, default_inputs()),
            Err(ScriptRuntimeCommandError::MissingCurrentLandmarkName)
        );
        assert_eq!(missing, state_before_missing);
        assert_eq!(
            validate_script_runtime_command(&command("getcurlandmarkname", &["STRING_BUFFER_2"],)),
            Err(ScriptRuntimeCommandError::InvalidStringBuffer {
                command: "getcurlandmarkname".to_string(),
                buffer: "STRING_BUFFER_2".to_string(),
            })
        );
        for invalid in [
            command(
                "gettrainername",
                &["STRING_BUFFER_2", "FALKNER", "FALKNER1"],
            ),
            command("getitemname", &["STRING_BUFFER_2", "POTION"]),
            command("getmonname", &["STRING_BUFFER_2", "PIKACHU"]),
            command("getstring", &["STRING_BUFFER_2", "SomeString"]),
        ] {
            assert_eq!(
                validate_script_runtime_command(&invalid),
                Err(ScriptRuntimeCommandError::InvalidStringBuffer {
                    command: invalid.command,
                    buffer: "STRING_BUFFER_2".to_string(),
                })
            );
        }
    }

    #[test]
    fn trainer_class_name_command_uses_exact_class_catalog_and_named_buffer() {
        let catalog = ScriptRuntimeReferenceCatalog {
            trainer_class_names: BTreeSet::from(["BEAUTY".to_string(), "COOLTRAINERM".to_string()]),
            ..ScriptRuntimeReferenceCatalog::default()
        };
        let runtime_command = command("gettrainerclassname", &["STRING_BUFFER_4", "COOLTRAINERM"]);

        assert_eq!(
            script_runtime_command_issues(&runtime_command, &catalog),
            []
        );
        assert_eq!(
            script_runtime_command_issues(
                &command("gettrainerclassname", &["STRING_BUFFER_4", "FISHER"]),
                &catalog,
            ),
            vec![ScriptRuntimeCommandIssue::UnknownTrainerClassName {
                trainer_class: "FISHER".to_string(),
            }]
        );
        assert_eq!(
            script_runtime_command_issues(
                &command("gettrainerclassname", &["STRING_BUFFER_4", "cooltrainerm"],),
                &catalog,
            ),
            vec![ScriptRuntimeCommandIssue::UnknownTrainerClassName {
                trainer_class: "cooltrainerm".to_string(),
            }]
        );
        assert_eq!(
            validate_script_runtime_command(&command(
                "gettrainerclassname",
                &["STRING_BUFFER_2", "COOLTRAINERM"],
            )),
            Err(ScriptRuntimeCommandError::InvalidStringBuffer {
                command: "gettrainerclassname".to_string(),
                buffer: "STRING_BUFFER_2".to_string(),
            })
        );

        let mut state = GameState::default();
        let mut inputs = default_inputs();
        inputs.resolved_named_buffer_value = Some("COOLTRAINER".to_string());
        apply_script_runtime_command(&mut state, runtime_command.clone(), inputs)
            .expect("gettrainerclassname records the resolved pack-owned class name");
        assert_eq!(
            state
                .script_runtime
                .named_buffers
                .get("STRING_BUFFER_4")
                .map(String::as_str),
            Some("COOLTRAINER")
        );

        let state_before_missing = GameState::default();
        let mut missing = state_before_missing.clone();
        assert_eq!(
            apply_script_runtime_command(&mut missing, runtime_command, default_inputs()),
            Err(ScriptRuntimeCommandError::MissingResolvedNamedBufferValue {
                command: "gettrainerclassname".to_string(),
            })
        );
        assert_eq!(missing, state_before_missing);
    }

    #[test]
    fn runtime_reference_catalog_requires_exact_landmark_and_trainer_class_name_sets() {
        let exact = ScriptRuntimeReferenceCatalog {
            landmarks: BTreeSet::from(["LANDMARK_ROUTE_32".to_string()]),
            trainer_class_names: BTreeSet::from(["COOLTRAINERM".to_string()]),
            ..ScriptRuntimeReferenceCatalog::default()
        };
        let value = serde_json::to_value(&exact).expect("serialize exact runtime catalog");
        let decoded = serde_json::from_value::<ScriptRuntimeReferenceCatalog>(value.clone())
            .expect("deserialize exact runtime catalog");
        assert_eq!(decoded, exact);

        let mut missing = value.clone();
        missing
            .as_object_mut()
            .expect("runtime catalog object")
            .remove("landmarks");
        assert!(
            serde_json::from_value::<ScriptRuntimeReferenceCatalog>(missing)
                .expect_err("landmark catalog field is required")
                .to_string()
                .contains("missing field `landmarks`")
        );

        let mut invalid = value;
        invalid["landmarks"] = serde_json::json!(["ROUTE_32"]);
        assert!(
            serde_json::from_value::<ScriptRuntimeReferenceCatalog>(invalid)
                .expect_err("landmark ids require canonical prefix")
                .to_string()
                .contains("exact LANDMARK_* ids")
        );

        let mut missing = serde_json::to_value(&exact).expect("serialize exact runtime catalog");
        missing
            .as_object_mut()
            .expect("runtime catalog object")
            .remove("trainer_class_names");
        assert!(
            serde_json::from_value::<ScriptRuntimeReferenceCatalog>(missing)
                .expect_err("trainer class name catalog field is required")
                .to_string()
                .contains("missing field `trainer_class_names`")
        );

        let mut invalid = serde_json::to_value(&exact).expect("serialize exact runtime catalog");
        invalid["trainer_class_names"] = serde_json::json!(["COOL TRAINER"]);
        assert!(
            serde_json::from_value::<ScriptRuntimeReferenceCatalog>(invalid)
                .expect_err("trainer class name ids require exact pack syntax")
                .to_string()
                .contains("nonempty exact pack id")
        );
    }

    #[test]
    fn cpu_return_conditions_are_typed_but_not_event_runtime_commands() {
        for (token, condition) in [
            ("z", ScriptRuntimeCpuCondition::Z),
            ("nz", ScriptRuntimeCpuCondition::Nz),
            ("c", ScriptRuntimeCpuCondition::C),
            ("nc", ScriptRuntimeCpuCondition::Nc),
        ] {
            assert_eq!(
                ScriptRuntimeCpuCondition::from_asm_token(token),
                Some(condition)
            );
        }
        assert_eq!(
            validate_script_runtime_command(&command("ret", &[])),
            Err(ScriptRuntimeCommandError::UnknownCommand {
                command: "ret".to_string(),
            })
        );
        assert_eq!(ScriptRuntimeCpuCondition::from_asm_token("NZ"), None);
    }

    #[test]
    fn conditional_event_is_background_event_data_not_an_executable_opcode() {
        let mut state = GameState::default();

        let error = apply_script_runtime_command(
            &mut state,
            command(
                "conditional_event",
                &["EVENT_OPENED_LOCKED_DOOR", ".Script"],
            ),
            default_inputs(),
        )
        .expect_err("conditional_event data cannot execute through the script interpreter");

        assert_eq!(
            error,
            ScriptRuntimeCommandError::UnknownCommand {
                command: "conditional_event".to_string(),
            }
        );
        assert!(state.script_runtime.command_queue.is_empty());
    }

    #[test]
    fn checkphonecall_reports_the_canonical_pending_special_call_byte() {
        let mut state = GameState::default();

        let none = apply_script_runtime_command(
            &mut state,
            command("checkphonecall", &[]),
            default_inputs(),
        )
        .expect("check empty special-call slot");
        assert_eq!(
            none,
            ScriptRuntimeOutcome::ScriptValueSet {
                command: "checkphonecall".to_string(),
                value: "0".to_string(),
                source_script: "RuntimeScript".to_string(),
                command_index: 4,
            }
        );

        state.script_runtime.special_phone_call = Some("SPECIALCALL_POKERUS".to_string());
        let pending = apply_script_runtime_command(
            &mut state,
            command("checkphonecall", &[]),
            default_inputs(),
        )
        .expect("check pending special-call slot");
        assert!(matches!(
            pending,
            ScriptRuntimeOutcome::ScriptValueSet { value, .. } if value == "1"
        ));
        assert_eq!(state.script_runtime.script_value.as_deref(), Some("1"));
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
                &command("getitemname", &["STRING_BUFFER_3", "legacy_POTION"]),
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
            trainer_class_names: BTreeSet::from(["FALKNER".to_string()]),
            items: BTreeSet::from(["POTION".to_string()]),
            pokemon: BTreeSet::from(["PIKACHU".to_string()]),
            phone_contacts: BTreeSet::from(["PHONE_ELM".to_string()]),
            special_phone_calls: BTreeSet::from(["SPECIALCALL_MASTERBALL".to_string()]),
            npc_trades: BTreeSet::from(["NPC_TRADE_MIKE".to_string()]),
            landmarks: BTreeSet::from(["LANDMARK_ROUTE_32".to_string()]),
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
                &command("getitemname", &["STRING_BUFFER_3", "potion"]),
                &catalog
            ),
            vec![ScriptRuntimeCommandIssue::UnknownItem {
                item_id: "potion".to_string()
            }]
        );
        assert_eq!(
            script_runtime_command_issues(
                &command("getitemname", &["STRING_BUFFER_3", "$POTION"]),
                &catalog
            ),
            vec![ScriptRuntimeCommandIssue::InvalidItem {
                item_id: "$POTION".to_string()
            }]
        );
        assert_eq!(
            script_runtime_command_issues(
                &command("getitemname", &["STRING_BUFFER_3", "ITEM_FROM_MEM"]),
                &catalog
            ),
            vec![ScriptRuntimeCommandIssue::UnknownItem {
                item_id: "ITEM_FROM_MEM".to_string()
            }]
        );
        assert_eq!(
            script_runtime_command_issues(
                &command("getmonname", &["STRING_BUFFER_3", "pikachu"]),
                &catalog
            ),
            vec![ScriptRuntimeCommandIssue::UnknownSpecies {
                species_id: "pikachu".to_string()
            }]
        );
        assert_eq!(
            script_runtime_command_issues(
                &command("getmonname", &["STRING_BUFFER_3", "$PIKACHU"]),
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
                ".Local".to_string(),
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
            script_runtime_command_issues(&command("writecmdqueue", &[".Missing"]), &catalog),
            vec![ScriptRuntimeCommandIssue::UnknownTarget {
                target_label: ".Missing".to_string()
            }]
        );
        assert_eq!(
            script_runtime_command_issues(&command("writecmdqueue", &["$Missing"]), &catalog),
            vec![ScriptRuntimeCommandIssue::InvalidTarget {
                target_label: "$Missing".to_string()
            }]
        );
    }

    #[test]
    fn commands_commit_only_their_authoritative_runtime_state() {
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
    }

    #[test]
    fn check_pokemail_sets_script_result_from_authoritative_party_mail() {
        let mut state = GameState::default();
        let species = crate::models::PokemonSpecies::new_for_tests(
            "SPEAROW",
            crate::models::BaseStats::new(40, 60, 30, 70, 31, 31),
        );
        let mut pokemon =
            crate::models::Pokemon::new_for_tests(species, 10, crate::models::Dv::default());
        pokemon.mail = Some(crate::models::pokemon::MailData {
            message: "DARK CAVE leads".to_string(),
            author: "RANDY".to_string(),
            nationality: 0,
            author_id: 0,
            species: "SPEAROW".to_string(),
            mail_type: "FLOWER_MAIL".to_string(),
        });
        pokemon.item = Some("FLOWER_MAIL".to_string());
        state.storage.party.pokemon[0] = Some(pokemon);

        apply_script_runtime_command(
            &mut state,
            command("checkpokemail", &["ReceivedSpearowMailText"]),
            default_inputs(),
        )
        .expect("check mail");

        assert_eq!(state.script_runtime.script_value.as_deref(), Some("2"));
    }

    #[test]
    fn catchtutorial_leaves_the_script_accumulator_unchanged() {
        let mut state = GameState::default();
        state.script_runtime.script_value = Some("7".to_string());

        apply_script_runtime_command(
            &mut state,
            command("catchtutorial", &["BATTLETYPE_TUTORIAL"]),
            default_inputs(),
        )
        .expect("catchtutorial");

        assert_eq!(state.script_runtime.script_value.as_deref(), Some("7"));
    }

    #[test]
    fn specialphonecall_overwrites_and_none_clears_the_single_live_slot() {
        let mut state = GameState::default();

        apply_script_runtime_command(
            &mut state,
            command("specialphonecall", &["SPECIALCALL_MASTERBALL"]),
            default_inputs(),
        )
        .expect("queue call");
        assert_eq!(
            state.script_runtime.special_phone_call.as_deref(),
            Some("SPECIALCALL_MASTERBALL")
        );
        apply_script_runtime_command(
            &mut state,
            command("specialphonecall", &["SPECIALCALL_POKERUS"]),
            default_inputs(),
        )
        .expect("overwrite call");
        assert_eq!(
            state.script_runtime.special_phone_call.as_deref(),
            Some("SPECIALCALL_POKERUS")
        );
        apply_script_runtime_command(
            &mut state,
            command(
                "specialphonecall",
                &[SCRIPT_RUNTIME_SPECIAL_PHONE_CALL_NONE],
            ),
            default_inputs(),
        )
        .expect("clear calls");

        assert!(state.script_runtime.special_phone_call.is_none());
    }

    #[test]
    fn lock_and_release_commands_mutate_exact_runtime_state() {
        let mut state = GameState::default();

        apply_script_runtime_command(&mut state, command("lock", &[]), default_inputs())
            .expect("lock");
        apply_script_runtime_command(&mut state, command("lockall", &[]), default_inputs())
            .expect("lockall");
        assert!(state.script_runtime.player_input_locked);
        assert!(state.script_runtime.all_input_locked);

        apply_script_runtime_command(&mut state, command("release", &[]), default_inputs())
            .expect("release");
        apply_script_runtime_command(&mut state, command("releaseall", &[]), default_inputs())
            .expect("releaseall");

        assert!(!state.script_runtime.player_input_locked);
        assert!(!state.script_runtime.all_input_locked);
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

        assert_eq!(
            apply_script_runtime_command(
                &mut state,
                command("random", &["10"]),
                ScriptRuntimeInputs::default(),
            ),
            Err(ScriptRuntimeCommandError::RandomRequiresDivider)
        );

        state.random_state = crate::random::CrystalRandomState::default();
        let mut divider = crate::random::ReplayDivider::new([6, 0]);
        apply_script_random_command_in_map(&mut state, command("random", &["10"]), &mut divider)
            .expect("random with exact divider source");
        assert_eq!(state.script_runtime.script_value.as_deref(), Some("7"));
        assert_eq!(divider.remaining(), 0);

        let before = state.random_state;
        let mut no_divider = crate::random::ReplayDivider::new([]);
        apply_script_random_command_in_map(&mut state, command("random", &["0"]), &mut no_divider)
            .expect("random zero stores zero without reading DIV");
        assert_eq!(state.script_runtime.script_value.as_deref(), Some("0"));
        assert_eq!(state.random_state, before);
    }

    #[test]
    fn addval_wraps_the_script_accumulator_as_an_asm_byte() {
        let mut state = GameState::default();
        state.script_runtime.script_value = Some("0".to_string());

        apply_script_runtime_command(&mut state, command("addval", &["-1"]), default_inputs())
            .expect("subtract one from zero");
        assert_eq!(state.script_runtime.script_value.as_deref(), Some("255"));

        apply_script_runtime_command(&mut state, command("addval", &["1"]), default_inputs())
            .expect("add one to 255");
        assert_eq!(state.script_runtime.script_value.as_deref(), Some("0"));
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
        assert!(!state.script_runtime.version_check_requested);
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
    }

    #[test]
    fn loademote_records_the_exact_loaded_graphic() {
        let mut state = GameState::default();

        let outcome = apply_script_runtime_command(
            &mut state,
            command("loademote", &["EMOTE_SHADOW"]),
            default_inputs(),
        )
        .expect("load exact emote graphic");

        assert!(matches!(
            outcome,
            ScriptRuntimeOutcome::EffectRecorded { ref command, .. } if command == "loademote"
        ));
    }

    #[test]
    fn pause_and_wait_expand_their_byte_counters_to_lcd_frames() {
        let mut state = GameState::default();

        apply_script_runtime_command(&mut state, command("pause", &["15"]), default_inputs())
            .expect("pause");
        apply_script_runtime_command(&mut state, command("wait", &["15"]), default_inputs())
            .expect("wait");
        apply_script_runtime_command(&mut state, command("pause", &["0"]), default_inputs())
            .expect("zero pause");

        assert_eq!(state.script_runtime.pending_delays[0].frames, 30);
        assert_eq!(state.script_runtime.pending_delays[1].frames, 90);
        assert_eq!(state.script_runtime.pending_delays[2].frames, 512);
    }

    #[test]
    fn pause_and_wait_reject_parameters_outside_the_script_byte() {
        for opcode in ["pause", "wait"] {
            let mut state = GameState::default();
            assert!(
                apply_script_runtime_command(
                    &mut state,
                    command(opcode, &["256"]),
                    default_inputs(),
                )
                .is_err()
            );
            assert!(state.script_runtime.pending_delays.is_empty());
        }
    }

    #[test]
    fn deactivatefacing_queues_exact_one_frame_wait_and_object_release() {
        let mut state = GameState::default();
        state.script_runtime.all_input_locked = true;

        apply_script_runtime_command(
            &mut state,
            command("deactivatefacing", &["3"]),
            default_inputs(),
        )
        .expect("deactivatefacing");

        let delay = &state.script_runtime.pending_delays[0];
        assert_eq!(delay.command, "deactivatefacing");
        assert_eq!(delay.frames, 3);
        assert!(delay.release_all_objects);
    }

    #[test]
    fn earthquake_zero_low_six_bit_counter_wraps_to_256_ticks() {
        let mut state = GameState::default();

        apply_script_runtime_command(&mut state, command("earthquake", &["0"]), default_inputs())
            .expect("zero earthquake byte");

        let earthquake = &state.script_runtime.pending_earthquakes[0];
        assert_eq!(earthquake.parameter, 0);
        assert_eq!(earthquake.shake_frames, 0);
        assert_eq!(earthquake.sleep_frames, 256);
    }

    #[test]
    fn earthquake_rejects_parameters_that_do_not_fit_script_byte() {
        let mut state = GameState::default();

        assert!(
            apply_script_runtime_command(
                &mut state,
                command("earthquake", &["256"]),
                default_inputs(),
            )
            .is_err()
        );
        assert!(state.script_runtime.pending_earthquakes.is_empty());
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
        assert_eq!(state.blue_card_balance, 12);
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
        state.script_runtime.script_value = Some("256".to_string());
        assert!(matches!(
            apply_script_runtime_command(
                &mut state,
                command("getnum", &["STRING_BUFFER_3"]),
                default_inputs()
            ),
            Err(ScriptRuntimeCommandError::UnknownNumericToken { .. })
        ));
        assert!(state.script_runtime.named_buffers.is_empty());
    }

    #[test]
    fn getnum_requires_a_source_string_buffer_destination() {
        assert_eq!(
            validate_script_runtime_command(&command("getnum", &["STRING_BUFFER_2"])),
            Err(ScriptRuntimeCommandError::InvalidStringBuffer {
                command: "getnum".to_string(),
                buffer: "STRING_BUFFER_2".to_string(),
            })
        );
    }

    #[test]
    fn elevator_table_directives_are_not_runtime_opcodes() {
        let mut state = GameState::default();
        assert_eq!(
            apply_script_runtime_command(
                &mut state,
                command("elevfloor", &["FLOOR_1F", "4", "CELADON_DEPT_STORE_1F"]),
                default_inputs(),
            ),
            Err(ScriptRuntimeCommandError::UnknownCommand {
                command: "elevfloor".to_string(),
            })
        );
    }

    #[test]
    fn writecmdqueue_requires_and_installs_resolved_stone_table_atomically() {
        let mut state = GameState::default();
        assert_eq!(
            apply_script_runtime_command(
                &mut state,
                command("writecmdqueue", &[".CommandQueue"]),
                default_inputs(),
            ),
            Err(ScriptRuntimeCommandError::MissingResolvedStoneTableQueue)
        );
        assert!(state.script_runtime.stone_table_entries.is_empty());

        let entries = vec![ScriptRuntimeStoneTableEntry {
            warp: 5,
            object_event: "BLACKTHORNGYM2F_BOULDER1".to_string(),
            script: ".Boulder1@BlackthornGym2FSetUpStoneTableCallback".to_string(),
            source_script: ".StoneTable@BlackthornGym2FSetUpStoneTableCallback".to_string(),
            command_index: 0,
        }];
        apply_script_runtime_command(
            &mut state,
            command("writecmdqueue", &[".CommandQueue"]),
            ScriptRuntimeInputs {
                resolved_stone_table_entries: Some(entries.clone()),
                ..default_inputs()
            },
        )
        .expect("writecmdqueue");

        assert_eq!(state.script_runtime.stone_table_entries, entries);
    }

    #[test]
    fn describedecoration_tail_jumps_to_the_resolved_asm_script() {
        let mut state = GameState::default();

        apply_script_runtime_command(
            &mut state,
            command("describedecoration", &["DECODESC_LEFT_DOLL"]),
            ScriptRuntimeInputs {
                resolved_decoration: Some(ScriptRuntimeDecorationResolution {
                    target_script: ".OrnamentConsoleScript@DecorationDesc_OrnamentOrConsole"
                        .to_string(),
                    string_buffer_3: Some("PIKACHU DOLL".to_string()),
                }),
                ..default_inputs()
            },
        )
        .expect("left doll describedecoration");

        assert_eq!(
            state.script_runtime.named_buffers.get("STRING_BUFFER_3"),
            Some(&"PIKACHU DOLL".to_string())
        );
        assert_eq!(
            state.script_runtime.next_script,
            Some(ScriptLocation {
                origin_map_name: "TestMap".to_string(),
                script: ".OrnamentConsoleScript@DecorationDesc_OrnamentOrConsole".to_string(),
            })
        );
    }

    #[test]
    fn asm_data_directives_are_not_runtime_opcodes() {
        let mut state = GameState::default();
        for (directive, args) in [
            ("dw", vec![".MenuData"]),
            ("dn", vec!["1", "2"]),
            ("dba", vec!["Target"]),
            ("dbw", vec!["1", "Target"]),
        ] {
            assert_eq!(
                apply_script_runtime_command(
                    &mut state,
                    command(directive, &args),
                    default_inputs(),
                ),
                Err(ScriptRuntimeCommandError::UnknownCommand {
                    command: directive.to_string(),
                })
            );
        }
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
    }

    #[test]
    fn menu_definition_directives_are_not_runtime_opcodes() {
        let mut state = GameState::default();
        assert_eq!(
            apply_script_runtime_command(
                &mut state,
                command(
                    "menu_coords",
                    &["SCREEN_LEFT", "2", "SCREEN_WIDTH - 1", "TEXTBOX_Y - 1"],
                ),
                default_inputs(),
            ),
            Err(ScriptRuntimeCommandError::UnknownCommand {
                command: "menu_coords".to_string(),
            })
        );
    }

    #[test]
    fn exported_script_numeric_parser_preserves_exact_asm_tokens() {
        assert_eq!(parse_script_i32_token("raw_script", "$10"), Ok(16));
        assert_eq!(parse_script_i32_token("raw_script", "%1010"), Ok(10));
        assert_eq!(parse_script_i32_token("raw_script", "-2"), Ok(-2));
        assert_eq!(parse_script_i32_token("raw_script", "+1"), Ok(1));
        assert_eq!(
            parse_script_i32_token("raw_script", "BATTLETOWER_REWARD_QUANTITY"),
            Ok(5)
        );
        assert!(matches!(
            parse_script_i32_token("raw_script", "0x10"),
            Err(ScriptRuntimeCommandError::InvalidNumericToken { .. })
        ));
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
    fn hall_of_fame_records_party_without_eggs_and_saturates_count() {
        let species =
            PokemonSpecies::new_for_tests("CHIKORITA", BaseStats::new(45, 49, 65, 45, 49, 65));
        let mut champion = Pokemon::new_for_tests(species.clone(), 42, Dv::from_non_hp(1, 2, 3, 4));
        champion.nickname = "CHAMPION".to_string();
        champion.original_trainer_id = 0x1234;
        let mut egg = Pokemon::new_for_tests(species, 5, Dv::default());
        egg.is_egg = true;
        let mut state = GameState::default();
        state.storage.party.pokemon[0] = Some(champion);
        state.storage.party.pokemon[1] = Some(egg);
        state.hall_of_fame.count = HALL_OF_FAME_MASTER_COUNT;
        state.set_game_timer_counting(true);

        let outcome =
            apply_script_runtime_command(&mut state, command("halloffame", &[]), default_inputs())
                .expect("hall of fame command");

        assert!(matches!(
            outcome,
            ScriptRuntimeOutcome::EffectRecorded { .. }
        ));
        assert_eq!(state.hall_of_fame.count, HALL_OF_FAME_MASTER_COUNT);
        assert_eq!(state.hall_of_fame.entries.len(), 1);
        let record = &state.hall_of_fame.entries[0];
        assert_eq!(
            record.team[0].as_ref().map(|mon| mon.species.as_str()),
            Some("CHIKORITA")
        );
        assert_eq!(record.team[0].as_ref().map(|mon| mon.dvs), Some(0x1234));
        assert!(record.team[1].is_none());
        assert_eq!(state.hall_of_fame.spawn_after_champion, Some(1));
        assert_eq!(
            state.flags.engine_flags.get("STATUSFLAGS_HALL_OF_FAME_F"),
            Some(&true)
        );
        assert!(state.script_runtime.hall_of_fame_requested);
        assert!(
            !state.game_timer_counting,
            "Script_halloffame clears GAME_TIMER_COUNTING_F until credits return"
        );
        assert!(
            !state.game_logic_paused,
            "HallOfFame restores wGameLogicPaused before animation and credits"
        );
    }

    #[test]
    fn credits_command_records_red_post_credits_spawn_before_presentation() {
        let mut state = GameState::default();

        let outcome =
            apply_script_runtime_command(&mut state, command("credits", &[]), default_inputs())
                .expect("credits command");

        assert!(matches!(
            outcome,
            ScriptRuntimeOutcome::EffectRecorded { .. }
        ));
        assert_eq!(state.hall_of_fame.spawn_after_champion, Some(2));
        assert!(state.script_runtime.credits_requested);
    }

    #[test]
    fn post_credits_markers_require_exported_source_constants() {
        for (name, constant) in [("halloffame", "SPAWN_LANCE"), ("credits", "SPAWN_RED")] {
            let mut state = GameState::default();
            let error = apply_script_runtime_command_in_map(
                &mut state,
                "TestMap",
                command(name, &[]),
                default_inputs(),
                &StoryEventScriptConstants::default(),
            )
            .expect_err("post-credits marker must not fall back to a numeric literal");
            assert_eq!(
                error,
                ScriptRuntimeCommandError::MissingSourceConstant {
                    command: name.to_string(),
                    constant: constant.to_string(),
                }
            );
            assert_eq!(state.hall_of_fame.spawn_after_champion, None);
        }
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
