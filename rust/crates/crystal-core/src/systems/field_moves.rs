use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::map::{ObjectEvent, WarpEvent};
use crate::models::{Item, PokemonStorage};
use crate::state::{EventFlagError, GameState};
use crate::world::collision::{
    MetatileCollision, Terrain, TilesetCollision, describe_collision, is_direction_blocked,
    sample_collision,
};
use crate::world::map::{Direction, OverworldMapData, TilePosition};
use crate::world::movement::{MovementMode, PlayerMovementState, StepOptions, move_by_stride};
use crate::world::session::{OverworldSession, WarpTransition};

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldMoveCatalog {
    pub cut: FieldMoveBlockRule,
    pub whirlpool: FieldMoveBlockRule,
    pub strength: FieldMoveFlagRule,
    pub flash: FieldMoveFlagRule,
    pub surf: FieldMoveTravelRule,
    pub waterfall: FieldMoveTravelRule,
    pub fly: FieldMoveRule,
    pub dig: FieldMoveMoveRule,
    pub teleport: FieldMoveMoveRule,
    pub escape_rope: FieldEscapeItemRule,
    pub repel: FieldRepelItemRule,
    pub bicycle: FieldItemRule,
    pub itemfinder: FieldItemRule,
    pub squirtbottle: FieldItemRule,
    pub coin_case: FieldItemRule,
    pub blue_card: FieldItemRule,
    pub town_map: FieldItemRule,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldMoveBadgeRequirement {
    pub region: String,
    pub index: usize,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldMoveRule {
    pub move_id: String,
    pub badge: FieldMoveBadgeRequirement,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldMoveMoveRule {
    pub move_id: String,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldEscapeItemRule {
    pub item_id: String,
    pub escape_rope_mode: String,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldRepelItemRule {}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldItemRule {
    pub item_id: String,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldMoveBlockRule {
    pub move_id: String,
    pub badge: FieldMoveBadgeRequirement,
    pub target_collisions: Vec<u8>,
    pub replacements: BTreeMap<String, BTreeMap<u16, FieldMoveReplacement>>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldMoveReplacement {
    pub replacement_block_id: u16,
    pub variant: String,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldMoveFlagRule {
    pub move_id: String,
    pub badge: FieldMoveBadgeRequirement,
    pub engine_flag: String,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldMoveTravelRule {
    pub move_id: String,
    pub badge: FieldMoveBadgeRequirement,
    pub blocked_collisions: Vec<u8>,
    pub target_collisions: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldMoveCatalogIssue {
    InvalidMoveId {
        subject: String,
    },
    UnknownMoveId {
        subject: String,
        move_id: String,
    },
    InvalidBadgeRegion {
        subject: String,
        move_id: String,
        region: String,
    },
    InvalidBadgeIndex {
        subject: String,
        move_id: String,
        index: usize,
    },
    MissingTargetCollisions {
        subject: String,
        move_id: String,
    },
    MissingReplacements {
        subject: String,
        move_id: String,
    },
    InvalidReplacementTileset {
        subject: String,
    },
    InvalidReplacementVariant {
        subject: String,
    },
    InvalidReplacementBlock {
        subject: String,
        block_id: u16,
    },
    InvalidEngineFlag {
        subject: String,
        move_id: String,
    },
    InvalidEscapeItemId,
    InvalidEscapeItemMode,
    UnknownEscapeItemRule {
        item_id: String,
        escape_rope_mode: String,
    },
    UnusableEscapeItem {
        item_id: String,
    },
    MissingRepelItemPayload,
    MissingUsableRepelItemPayload,
    InvalidFieldItemId {
        subject: String,
    },
    UnknownFieldItemId {
        subject: String,
        item_id: String,
    },
    UnusableFieldItem {
        subject: String,
        item_id: String,
    },
}

pub fn field_move_catalog_issues(
    catalog: &FieldMoveCatalog,
    moves: &BTreeSet<String>,
    items: &BTreeMap<String, Item>,
) -> Vec<FieldMoveCatalogIssue> {
    if catalog == &FieldMoveCatalog::default() {
        return Vec::new();
    }

    let mut issues = Vec::new();
    collect_block_rule_issues("field_moves:cut", &catalog.cut, moves, false, &mut issues);
    collect_block_rule_issues(
        "field_moves:whirlpool",
        &catalog.whirlpool,
        moves,
        false,
        &mut issues,
    );
    collect_flag_rule_issues(
        "field_moves:strength",
        &catalog.strength,
        moves,
        &mut issues,
    );
    collect_flag_rule_issues("field_moves:flash", &catalog.flash, moves, &mut issues);
    collect_travel_rule_issues("field_moves:surf", &catalog.surf, moves, false, &mut issues);
    collect_travel_rule_issues(
        "field_moves:waterfall",
        &catalog.waterfall,
        moves,
        true,
        &mut issues,
    );
    collect_move_rule_issues("field_moves:fly", &catalog.fly, moves, &mut issues);
    collect_move_only_rule_issues("field_moves:dig", &catalog.dig, moves, &mut issues);
    collect_move_only_rule_issues(
        "field_moves:teleport",
        &catalog.teleport,
        moves,
        &mut issues,
    );
    collect_escape_item_rule_issues(&catalog.escape_rope, items, &mut issues);
    collect_repel_rule_issues(items, &mut issues);
    collect_field_item_rule_issues("field_moves:bicycle", &catalog.bicycle, items, &mut issues);
    collect_field_item_rule_issues(
        "field_moves:itemfinder",
        &catalog.itemfinder,
        items,
        &mut issues,
    );
    collect_field_item_rule_issues(
        "field_moves:squirtbottle",
        &catalog.squirtbottle,
        items,
        &mut issues,
    );
    collect_field_item_rule_issues(
        "field_moves:coin_case",
        &catalog.coin_case,
        items,
        &mut issues,
    );
    collect_field_item_rule_issues(
        "field_moves:blue_card",
        &catalog.blue_card,
        items,
        &mut issues,
    );
    collect_field_item_rule_issues(
        "field_moves:town_map",
        &catalog.town_map,
        items,
        &mut issues,
    );
    issues
}

fn collect_move_rule_issues(
    subject: &str,
    rule: &FieldMoveRule,
    moves: &BTreeSet<String>,
    issues: &mut Vec<FieldMoveCatalogIssue>,
) {
    collect_move_id_issues(subject, &rule.move_id, moves, issues);
    collect_badge_issues(subject, &rule.move_id, &rule.badge, issues);
}

fn collect_move_only_rule_issues(
    subject: &str,
    rule: &FieldMoveMoveRule,
    moves: &BTreeSet<String>,
    issues: &mut Vec<FieldMoveCatalogIssue>,
) {
    collect_move_id_issues(subject, &rule.move_id, moves, issues);
}

fn collect_block_rule_issues(
    subject: &str,
    rule: &FieldMoveBlockRule,
    moves: &BTreeSet<String>,
    require_target_collisions: bool,
    issues: &mut Vec<FieldMoveCatalogIssue>,
) {
    collect_move_id_issues(subject, &rule.move_id, moves, issues);
    collect_badge_issues(subject, &rule.move_id, &rule.badge, issues);
    if require_target_collisions || rule.target_collisions.is_empty() {
        if rule.target_collisions.is_empty() {
            issues.push(FieldMoveCatalogIssue::MissingTargetCollisions {
                subject: subject.to_string(),
                move_id: rule.move_id.clone(),
            });
        }
    }
    if rule.replacements.is_empty() {
        issues.push(FieldMoveCatalogIssue::MissingReplacements {
            subject: subject.to_string(),
            move_id: rule.move_id.clone(),
        });
    }
    for (tileset, blocks) in &rule.replacements {
        let tileset_subject = format!("{subject}:replacements:{tileset}");
        let exact_tileset = is_exact_field_move_token(tileset);
        if !exact_tileset {
            issues.push(FieldMoveCatalogIssue::InvalidReplacementTileset {
                subject: tileset_subject,
            });
        }
        for (block_id, replacement) in blocks {
            let replacement_subject = format!("{subject}:replacements:{tileset}:{block_id}");
            let exact_variant = is_exact_field_move_token(&replacement.variant);
            if !exact_variant {
                issues.push(FieldMoveCatalogIssue::InvalidReplacementVariant {
                    subject: replacement_subject.clone(),
                });
            }
            if replacement.replacement_block_id == *block_id {
                issues.push(FieldMoveCatalogIssue::InvalidReplacementBlock {
                    subject: replacement_subject,
                    block_id: *block_id,
                });
            }
        }
    }
}

fn collect_flag_rule_issues(
    subject: &str,
    rule: &FieldMoveFlagRule,
    moves: &BTreeSet<String>,
    issues: &mut Vec<FieldMoveCatalogIssue>,
) {
    collect_move_id_issues(subject, &rule.move_id, moves, issues);
    collect_badge_issues(subject, &rule.move_id, &rule.badge, issues);
    if !is_exact_field_move_token(&rule.engine_flag) {
        issues.push(FieldMoveCatalogIssue::InvalidEngineFlag {
            subject: subject.to_string(),
            move_id: rule.move_id.clone(),
        });
    }
}

fn collect_travel_rule_issues(
    subject: &str,
    rule: &FieldMoveTravelRule,
    moves: &BTreeSet<String>,
    require_target_collisions: bool,
    issues: &mut Vec<FieldMoveCatalogIssue>,
) {
    collect_move_id_issues(subject, &rule.move_id, moves, issues);
    collect_badge_issues(subject, &rule.move_id, &rule.badge, issues);
    if require_target_collisions && rule.target_collisions.is_empty() {
        issues.push(FieldMoveCatalogIssue::MissingTargetCollisions {
            subject: subject.to_string(),
            move_id: rule.move_id.clone(),
        });
    }
}

fn collect_escape_item_rule_issues(
    rule: &FieldEscapeItemRule,
    items: &BTreeMap<String, Item>,
    issues: &mut Vec<FieldMoveCatalogIssue>,
) {
    let invalid_item_id = !is_exact_field_move_token(&rule.item_id);
    let invalid_escape_rope_mode = !is_exact_field_move_token(&rule.escape_rope_mode);
    if invalid_item_id {
        issues.push(FieldMoveCatalogIssue::InvalidEscapeItemId);
    }
    if invalid_escape_rope_mode {
        issues.push(FieldMoveCatalogIssue::InvalidEscapeItemMode);
    }
    if items.is_empty() || invalid_item_id || invalid_escape_rope_mode {
        return;
    }
    match items.get(&rule.item_id) {
        Some(item) if item.escape_rope_mode.as_deref() == Some(rule.escape_rope_mode.as_str()) => {
            if !item.field_usable {
                issues.push(FieldMoveCatalogIssue::UnusableEscapeItem {
                    item_id: rule.item_id.clone(),
                });
            }
        }
        _ => issues.push(FieldMoveCatalogIssue::UnknownEscapeItemRule {
            item_id: rule.item_id.clone(),
            escape_rope_mode: rule.escape_rope_mode.clone(),
        }),
    }
}

fn collect_repel_rule_issues(
    items: &BTreeMap<String, Item>,
    issues: &mut Vec<FieldMoveCatalogIssue>,
) {
    if items.is_empty() {
        return;
    }
    let repel_items = items
        .values()
        .filter(|item| item.repel_steps.is_some())
        .collect::<Vec<_>>();
    if repel_items.is_empty() {
        issues.push(FieldMoveCatalogIssue::MissingRepelItemPayload);
    } else if !repel_items.iter().any(|item| item.field_usable) {
        issues.push(FieldMoveCatalogIssue::MissingUsableRepelItemPayload);
    }
}

fn collect_field_item_rule_issues(
    subject: &str,
    rule: &FieldItemRule,
    items: &BTreeMap<String, Item>,
    issues: &mut Vec<FieldMoveCatalogIssue>,
) {
    if !is_exact_field_move_token(&rule.item_id) {
        issues.push(FieldMoveCatalogIssue::InvalidFieldItemId {
            subject: subject.to_string(),
        });
        return;
    }
    match items.get(&rule.item_id) {
        Some(item) if !item.field_usable => issues.push(FieldMoveCatalogIssue::UnusableFieldItem {
            subject: subject.to_string(),
            item_id: rule.item_id.clone(),
        }),
        Some(_) => {}
        None => issues.push(FieldMoveCatalogIssue::UnknownFieldItemId {
            subject: subject.to_string(),
            item_id: rule.item_id.clone(),
        }),
    }
}

fn collect_move_id_issues(
    subject: &str,
    move_id: &str,
    moves: &BTreeSet<String>,
    issues: &mut Vec<FieldMoveCatalogIssue>,
) {
    if !is_exact_field_move_token(move_id) {
        issues.push(FieldMoveCatalogIssue::InvalidMoveId {
            subject: subject.to_string(),
        });
    } else if !moves.contains(move_id) {
        issues.push(FieldMoveCatalogIssue::UnknownMoveId {
            subject: subject.to_string(),
            move_id: move_id.to_string(),
        });
    }
}

fn is_exact_field_move_token(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && !has_reserved_pack_prefix(value)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn has_reserved_pack_prefix(value: &str) -> bool {
    let value = value.to_ascii_lowercase();
    value.starts_with("fallback") || value.starts_with("legacy")
}

fn collect_badge_issues(
    subject: &str,
    move_id: &str,
    badge: &FieldMoveBadgeRequirement,
    issues: &mut Vec<FieldMoveCatalogIssue>,
) {
    if badge.region != "johto" {
        issues.push(FieldMoveCatalogIssue::InvalidBadgeRegion {
            subject: subject.to_string(),
            move_id: move_id.to_string(),
            region: badge.region.clone(),
        });
    }
    if badge.index >= 8 {
        issues.push(FieldMoveCatalogIssue::InvalidBadgeIndex {
            subject: subject.to_string(),
            move_id: move_id.to_string(),
            index: badge.index,
        });
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldMoveBlockOutcome {
    pub move_id: String,
    pub actor_party_index: usize,
    pub actor_species: String,
    pub map_name: String,
    pub tileset_name: String,
    pub metatile_x: u16,
    pub metatile_y: u16,
    pub previous_block_id: u16,
    pub replacement_block_id: u16,
    pub variant: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldMoveFlagOutcome {
    pub move_id: String,
    pub actor_party_index: usize,
    pub actor_species: String,
    pub engine_flag: String,
    pub was_set: bool,
    pub is_set: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldMoveTravelOutcome {
    pub move_id: String,
    pub actor_party_index: usize,
    pub actor_species: String,
    pub map_name: String,
    pub from_tile: TilePosition,
    pub to_tile: TilePosition,
    pub steps: u16,
    pub mode: MovementMode,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepelItemUseOutcome {
    pub item_id: String,
    pub repel_steps_before: u16,
    pub repel_steps_after: u16,
    pub active_repel_item_before: Option<String>,
    pub active_repel_item_after: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldMoveUseOutcome {
    pub move_id: String,
    pub actor_party_index: usize,
    pub actor_species: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DigWarpMemoryOutcome {
    pub before_map_name: Option<String>,
    pub before_index: Option<u16>,
    pub after_map_name: Option<String>,
    pub after_index: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedDigWarpDestination {
    pub map_name: String,
    pub warp_index: u16,
    pub tile: TilePosition,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SquirtBottleTarget {
    pub target_tile: TilePosition,
    pub target_object_identifier: Option<String>,
    pub target_movement: String,
    pub target_script: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(deny_unknown_fields)]
pub enum FieldMoveError {
    #[error("field move {move_id} is missing required modpack rule field {field}")]
    MissingRuleField { move_id: String, field: String },
    #[error("field move rule field {field} has invalid value {value}")]
    InvalidRuleField { field: String, value: String },
    #[error("field move {move_id} uses unsupported badge region {region}")]
    UnsupportedBadgeRegion { move_id: String, region: String },
    #[error("field move {move_id} uses invalid badge index {badge_index}")]
    InvalidBadgeIndex { move_id: String, badge_index: usize },
    #[error("field move party index {party_index} is outside the party")]
    PartyIndexOutOfRange { party_index: usize },
    #[error("field move party index {party_index} has no Pokemon")]
    EmptyPartySlot { party_index: usize },
    #[error("party Pokemon at index {party_index} does not know {move_id}")]
    PokemonDoesNotKnowMove { party_index: usize, move_id: String },
    #[error("field move {move_id} requires Johto badge index {badge_index}")]
    MissingBadge { move_id: String, badge_index: usize },
    #[error("field escape item {item_id} expected configured item id {expected_item_id}")]
    InvalidEscapeItemId {
        item_id: String,
        expected_item_id: String,
    },
    #[error(
        "field escape item {item_id} has escape_rope_mode {mode:?}, expected configured mode {expected_mode}"
    )]
    InvalidEscapeItemMode {
        item_id: String,
        mode: Option<String>,
        expected_mode: String,
    },
    #[error("field repel item {item_id} is missing repel_steps")]
    MissingRepelItemSteps { item_id: String },
    #[error("field repel item {item_id} has invalid repel_steps 0")]
    InvalidRepelItemSteps { item_id: String },
    #[error("saved active_repel_item {item_id} is missing from compiled pack items")]
    MissingSavedActiveRepelItem { item_id: String },
    #[error(
        "saved repel_steps_remaining {steps_remaining} exceeds compiled active_repel_item {item_id} duration {compiled_steps}"
    )]
    SavedRepelStepsExceedCompiledDuration {
        item_id: String,
        steps_remaining: u16,
        compiled_steps: u16,
    },
    #[error("field item rule {rule_id} has no configured item id")]
    MissingFieldItemId { rule_id: String },
    #[error(
        "field item {item_id} for rule {rule_id} expected configured item id {expected_item_id}"
    )]
    InvalidFieldItemId {
        rule_id: String,
        item_id: String,
        expected_item_id: String,
    },
    #[error("blue card balance VAR_BLUECARDBALANCE has invalid exact integer {value}")]
    InvalidBlueCardBalance { value: String },
    #[error("blue card balance VAR_BLUECARDBALANCE is outside 0..=30: {balance}")]
    BlueCardBalanceOutOfRange { balance: u16 },
    #[error("saved blue_card_balance {balance} requires compiled Buena prize definitions")]
    MissingBuenaPrizesForSavedBlueCardBalance { balance: u8 },
    #[error("{context} has no saved dig warp map")]
    MissingSavedDigWarpMap { context: String },
    #[error("{context} has no saved dig warp index")]
    MissingSavedDigWarpIndex { context: String },
    #[error("{context} saved dig warp index {warp_index} missing on {map_name}")]
    MissingSavedDigWarp {
        context: String,
        map_name: String,
        warp_index: u16,
    },
    #[error(
        "field squirtbottle target {object_identifier:?} references missing exact script {script}"
    )]
    MissingSquirtBottleTargetScript {
        object_identifier: Option<String>,
        script: String,
    },
    #[error("field move {move_id} cannot be used while player movement mode is {mode:?}")]
    InvalidMovementMode { move_id: String, mode: MovementMode },
    #[error("field move {move_id} requires facing {required:?}, got {actual:?}")]
    InvalidFacing {
        move_id: String,
        required: Direction,
        actual: Direction,
    },
    #[error("field move {move_id} target tile is outside map {map_name}")]
    TargetTileOutOfBounds { move_id: String, map_name: String },
    #[error("field move {move_id} target tile is blocked")]
    BlockedTarget { move_id: String },
    #[error("field move {move_id} target tile is not water")]
    TargetNotWater { move_id: String },
    #[error("field move {move_id} target tile is not a waterfall")]
    TargetNotWaterfall { move_id: String },
    #[error("field move target ({metatile_x}, {metatile_y}) is outside map {map_name}")]
    TargetOutOfBounds {
        map_name: String,
        metatile_x: u16,
        metatile_y: u16,
    },
    #[error("field move target block {block_id:#04x} is missing tileset collision data")]
    MissingMetatileCollision { block_id: u16 },
    #[error(
        "field move {move_id} target block {block_id:#04x} does not contain a supported collision"
    )]
    UnsupportedCollision { move_id: String, block_id: u16 },
    #[error(
        "field move {move_id} has no exact replacement for tileset '{tileset_name}' block {block_id:#04x}"
    )]
    UnsupportedReplacement {
        move_id: String,
        tileset_name: String,
        block_id: u16,
    },
    #[error("field move flag error: {0}")]
    Flag(#[from] EventFlagError),
}

pub fn resolve_squirtbottle_target<F>(
    overworld: &OverworldSession,
    target_script_exists: F,
) -> Result<SquirtBottleTarget, FieldMoveError>
where
    F: FnOnce(&str) -> bool,
{
    let target_tile = move_by_stride(
        overworld.player.tile,
        overworld.player.facing,
        StepOptions::default().stride_tiles,
    );
    let Some((_, object)) = overworld.visible_object_at(target_tile) else {
        return Ok(SquirtBottleTarget {
            target_tile,
            target_object_identifier: None,
            target_movement: String::new(),
            target_script: None,
        });
    };
    squirtbottle_target_for_object(target_tile, object, target_script_exists)
}

fn squirtbottle_target_for_object<F>(
    target_tile: TilePosition,
    object: &ObjectEvent,
    target_script_exists: F,
) -> Result<SquirtBottleTarget, FieldMoveError>
where
    F: FnOnce(&str) -> bool,
{
    const SUDOWOODO_MOVEMENT: &str = "SPRITEMOVEDATA_SUDOWOODO";
    if object.spritemovedata != SUDOWOODO_MOVEMENT {
        return Ok(SquirtBottleTarget {
            target_tile,
            target_object_identifier: object.object_identifier.clone(),
            target_movement: object.spritemovedata.clone(),
            target_script: None,
        });
    }
    if !target_script_exists(&object.script) {
        return Err(FieldMoveError::MissingSquirtBottleTargetScript {
            object_identifier: object.object_identifier.clone(),
            script: object.script.clone(),
        });
    }
    Ok(SquirtBottleTarget {
        target_tile,
        target_object_identifier: object.object_identifier.clone(),
        target_movement: object.spritemovedata.clone(),
        target_script: Some(object.script.clone()),
    })
}

pub fn apply_cut_field_move(
    catalog: &FieldMoveCatalog,
    state: &mut GameState,
    storage: &PokemonStorage,
    map: &mut OverworldMapData,
    tileset: &TilesetCollision,
    tileset_name: &str,
    party_index: usize,
    metatile_x: u16,
    metatile_y: u16,
) -> Result<FieldMoveBlockOutcome, FieldMoveError> {
    apply_block_field_move(
        &catalog.cut,
        state,
        storage,
        map,
        tileset,
        tileset_name,
        party_index,
        metatile_x,
        metatile_y,
    )
}

pub fn apply_whirlpool_field_move(
    catalog: &FieldMoveCatalog,
    state: &mut GameState,
    storage: &PokemonStorage,
    map: &mut OverworldMapData,
    tileset: &TilesetCollision,
    tileset_name: &str,
    party_index: usize,
    metatile_x: u16,
    metatile_y: u16,
) -> Result<FieldMoveBlockOutcome, FieldMoveError> {
    apply_block_field_move(
        &catalog.whirlpool,
        state,
        storage,
        map,
        tileset,
        tileset_name,
        party_index,
        metatile_x,
        metatile_y,
    )
}

fn apply_block_field_move(
    rule: &FieldMoveBlockRule,
    state: &mut GameState,
    storage: &PokemonStorage,
    map: &mut OverworldMapData,
    tileset: &TilesetCollision,
    tileset_name: &str,
    party_index: usize,
    metatile_x: u16,
    metatile_y: u16,
) -> Result<FieldMoveBlockOutcome, FieldMoveError> {
    require_rule_field(&rule.move_id, "move_id")?;
    let actor = require_party_move(storage, party_index, &rule.move_id)?;
    require_badge(state, &rule.move_id, &rule.badge)?;
    let (index, previous_block_id, collisions) =
        target_metatile(map, tileset, metatile_x, metatile_y)?;
    if !contains_any_collision(collisions, &rule.target_collisions) {
        return Err(FieldMoveError::UnsupportedCollision {
            move_id: rule.move_id.clone(),
            block_id: previous_block_id,
        });
    }
    let replacement =
        block_replacement(rule, tileset_name, previous_block_id).ok_or_else(|| {
            FieldMoveError::UnsupportedReplacement {
                move_id: rule.move_id.clone(),
                tileset_name: tileset_name.to_string(),
                block_id: previous_block_id,
            }
        })?;
    let replacement_block_id = replacement.replacement_block_id;
    map.metatile_ids[index] = replacement_block_id;
    record_block_override(
        state,
        &map.name,
        metatile_x,
        metatile_y,
        replacement_block_id,
    );
    Ok(FieldMoveBlockOutcome {
        move_id: rule.move_id.clone(),
        actor_party_index: party_index,
        actor_species: actor.species.id.clone(),
        map_name: map.name.clone(),
        tileset_name: tileset_name.to_string(),
        metatile_x,
        metatile_y,
        previous_block_id,
        replacement_block_id,
        variant: replacement.variant.clone(),
    })
}

pub fn apply_strength_field_move(
    catalog: &FieldMoveCatalog,
    state: &mut GameState,
    storage: &PokemonStorage,
    party_index: usize,
) -> Result<FieldMoveFlagOutcome, FieldMoveError> {
    apply_flag_field_move(&catalog.strength, state, storage, party_index)
}

pub fn apply_flash_field_move(
    catalog: &FieldMoveCatalog,
    state: &mut GameState,
    storage: &PokemonStorage,
    party_index: usize,
) -> Result<FieldMoveFlagOutcome, FieldMoveError> {
    apply_flag_field_move(&catalog.flash, state, storage, party_index)
}

fn apply_flag_field_move(
    rule: &FieldMoveFlagRule,
    state: &mut GameState,
    storage: &PokemonStorage,
    party_index: usize,
) -> Result<FieldMoveFlagOutcome, FieldMoveError> {
    require_rule_field(&rule.move_id, "move_id")?;
    require_rule_field(&rule.engine_flag, "engine_flag")?;
    let actor = require_party_move(storage, party_index, &rule.move_id)?;
    require_badge(state, &rule.move_id, &rule.badge)?;
    set_field_move_engine_flag(
        state,
        &rule.move_id,
        party_index,
        &actor.species.id,
        &rule.engine_flag,
    )
}

pub fn apply_surf_field_move(
    catalog: &FieldMoveCatalog,
    state: &GameState,
    storage: &PokemonStorage,
    map: &OverworldMapData,
    tileset: &TilesetCollision,
    player: &mut PlayerMovementState,
    party_index: usize,
) -> Result<FieldMoveTravelOutcome, FieldMoveError> {
    let rule = &catalog.surf;
    require_rule_field(&rule.move_id, "move_id")?;
    let actor = require_party_move(storage, party_index, &rule.move_id)?;
    require_badge(state, &rule.move_id, &rule.badge)?;
    if player.mode == MovementMode::Surf {
        return Err(FieldMoveError::InvalidMovementMode {
            move_id: rule.move_id.clone(),
            mode: player.mode,
        });
    }
    let target = move_by_stride(player.tile, player.facing, 2);
    let sample = sample_collision(map, tileset, target).ok_or_else(|| {
        FieldMoveError::TargetTileOutOfBounds {
            move_id: rule.move_id.clone(),
            map_name: map.name.clone(),
        }
    })?;
    if describe_collision(sample.permission).terrain != Terrain::Water {
        return Err(FieldMoveError::TargetNotWater {
            move_id: rule.move_id.clone(),
        });
    }
    if is_direction_blocked(sample.permission, player.facing)
        || rule.blocked_collisions.contains(&sample.permission)
    {
        return Err(FieldMoveError::BlockedTarget {
            move_id: rule.move_id.clone(),
        });
    }
    let from_tile = player.tile;
    player.mode = MovementMode::Surf;
    player.tile = target;
    Ok(FieldMoveTravelOutcome {
        move_id: rule.move_id.clone(),
        actor_party_index: party_index,
        actor_species: actor.species.id.clone(),
        map_name: map.name.clone(),
        from_tile,
        to_tile: target,
        steps: 1,
        mode: player.mode,
    })
}

pub fn apply_waterfall_field_move(
    catalog: &FieldMoveCatalog,
    state: &GameState,
    storage: &PokemonStorage,
    map: &OverworldMapData,
    tileset: &TilesetCollision,
    player: &mut PlayerMovementState,
    party_index: usize,
) -> Result<FieldMoveTravelOutcome, FieldMoveError> {
    let rule = &catalog.waterfall;
    require_rule_field(&rule.move_id, "move_id")?;
    let actor = require_party_move(storage, party_index, &rule.move_id)?;
    require_badge(state, &rule.move_id, &rule.badge)?;
    if player.mode != MovementMode::Surf {
        return Err(FieldMoveError::InvalidMovementMode {
            move_id: rule.move_id.clone(),
            mode: player.mode,
        });
    }
    if player.facing != Direction::Up {
        return Err(FieldMoveError::InvalidFacing {
            move_id: rule.move_id.clone(),
            required: Direction::Up,
            actual: player.facing,
        });
    }
    let first_target = move_by_stride(player.tile, player.facing, 2);
    let first_sample = sample_collision(map, tileset, first_target).ok_or_else(|| {
        FieldMoveError::TargetTileOutOfBounds {
            move_id: rule.move_id.clone(),
            map_name: map.name.clone(),
        }
    })?;
    if !rule.target_collisions.contains(&first_sample.permission) {
        return Err(FieldMoveError::TargetNotWaterfall {
            move_id: rule.move_id.clone(),
        });
    }

    let from_tile = player.tile;
    let mut steps = 0_u16;
    loop {
        let target = move_by_stride(player.tile, player.facing, 2);
        let Some(sample) = sample_collision(map, tileset, target) else {
            break;
        };
        if describe_collision(sample.permission).terrain != Terrain::Water {
            break;
        }
        if is_direction_blocked(sample.permission, player.facing) {
            break;
        }
        player.tile = target;
        steps += 1;
        if !rule.target_collisions.contains(&sample.permission) {
            break;
        }
    }
    if steps == 0 {
        return Err(FieldMoveError::BlockedTarget {
            move_id: rule.move_id.clone(),
        });
    }
    Ok(FieldMoveTravelOutcome {
        move_id: rule.move_id.clone(),
        actor_party_index: party_index,
        actor_species: actor.species.id.clone(),
        map_name: map.name.clone(),
        from_tile,
        to_tile: player.tile,
        steps,
        mode: player.mode,
    })
}

pub fn validate_fly_field_move(
    catalog: &FieldMoveCatalog,
    state: &GameState,
    storage: &PokemonStorage,
    party_index: usize,
) -> Result<FieldMoveUseOutcome, FieldMoveError> {
    let rule = &catalog.fly;
    require_rule_field(&rule.move_id, "move_id")?;
    let actor = require_party_move(storage, party_index, &rule.move_id)?;
    require_badge(state, &rule.move_id, &rule.badge)?;
    Ok(FieldMoveUseOutcome {
        move_id: rule.move_id.clone(),
        actor_party_index: party_index,
        actor_species: actor.species.id.clone(),
    })
}

pub fn validate_dig_field_move(
    catalog: &FieldMoveCatalog,
    storage: &PokemonStorage,
    party_index: usize,
) -> Result<FieldMoveUseOutcome, FieldMoveError> {
    validate_move_only_field_move(&catalog.dig, storage, party_index)
}

pub fn validate_teleport_field_move(
    catalog: &FieldMoveCatalog,
    storage: &PokemonStorage,
    party_index: usize,
) -> Result<FieldMoveUseOutcome, FieldMoveError> {
    validate_move_only_field_move(&catalog.teleport, storage, party_index)
}

pub fn validate_field_escape_item(
    catalog: &FieldMoveCatalog,
    item: &Item,
) -> Result<(), FieldMoveError> {
    let rule = &catalog.escape_rope;
    require_rule_field(&rule.item_id, "item_id")?;
    require_rule_field(&rule.escape_rope_mode, "escape_rope_mode")?;
    if item.script_name != rule.item_id {
        return Err(FieldMoveError::InvalidEscapeItemId {
            item_id: item.script_name.clone(),
            expected_item_id: rule.item_id.clone(),
        });
    }
    if item.escape_rope_mode.as_deref() != Some(rule.escape_rope_mode.as_str()) {
        return Err(FieldMoveError::InvalidEscapeItemMode {
            item_id: item.script_name.clone(),
            mode: item.escape_rope_mode.clone(),
            expected_mode: rule.escape_rope_mode.clone(),
        });
    }
    Ok(())
}

pub fn validate_repel_item(catalog: &FieldMoveCatalog, item: &Item) -> Result<u16, FieldMoveError> {
    let _rule = &catalog.repel;
    let steps = item
        .repel_steps
        .ok_or_else(|| FieldMoveError::MissingRepelItemSteps {
            item_id: item.script_name.clone(),
        })?;
    if steps == 0 {
        return Err(FieldMoveError::InvalidRepelItemSteps {
            item_id: item.script_name.clone(),
        });
    }
    Ok(steps)
}

pub fn validate_saved_active_repel_item(
    catalog: &FieldMoveCatalog,
    item_id: &str,
    item: Option<&Item>,
    steps_remaining: u16,
) -> Result<(), FieldMoveError> {
    let item = item.ok_or_else(|| FieldMoveError::MissingSavedActiveRepelItem {
        item_id: item_id.to_string(),
    })?;
    let compiled_steps = validate_repel_item(catalog, item)?;
    if steps_remaining > compiled_steps {
        return Err(FieldMoveError::SavedRepelStepsExceedCompiledDuration {
            item_id: item.script_name.clone(),
            steps_remaining,
            compiled_steps,
        });
    }
    Ok(())
}

pub fn apply_repel_item_use(
    state: &mut GameState,
    item_id: impl Into<String>,
    steps: u16,
) -> RepelItemUseOutcome {
    let item_id = item_id.into();
    let repel_steps_before = state.repel_steps_remaining;
    let active_repel_item_before = state.active_repel_item.clone();
    state.repel_steps_remaining = steps;
    state.active_repel_item = Some(item_id.clone());
    RepelItemUseOutcome {
        item_id,
        repel_steps_before,
        repel_steps_after: state.repel_steps_remaining,
        active_repel_item_before,
        active_repel_item_after: state.active_repel_item.clone(),
    }
}

pub fn validate_bicycle_item(
    catalog: &FieldMoveCatalog,
    item: &Item,
) -> Result<(), FieldMoveError> {
    validate_field_item_id("bicycle", &catalog.bicycle, item)
}

pub fn validate_itemfinder_item(
    catalog: &FieldMoveCatalog,
    item: &Item,
) -> Result<(), FieldMoveError> {
    validate_field_item_id("itemfinder", &catalog.itemfinder, item)
}

pub fn validate_squirtbottle_item(
    catalog: &FieldMoveCatalog,
    item: &Item,
) -> Result<(), FieldMoveError> {
    validate_field_item_id("squirtbottle", &catalog.squirtbottle, item)
}

pub fn validate_coin_case_item(
    catalog: &FieldMoveCatalog,
    item: &Item,
) -> Result<(), FieldMoveError> {
    validate_field_item_id("coin_case", &catalog.coin_case, item)
}

pub fn validate_blue_card_item(
    catalog: &FieldMoveCatalog,
    item: &Item,
) -> Result<(), FieldMoveError> {
    validate_field_item_id("blue_card", &catalog.blue_card, item)
}

pub fn validate_town_map_item(
    catalog: &FieldMoveCatalog,
    item: &Item,
) -> Result<(), FieldMoveError> {
    validate_field_item_id("town_map", &catalog.town_map, item)
}

pub fn blue_card_balance(state: &GameState) -> Result<u8, FieldMoveError> {
    let Some(value) = state.script_runtime.variables.get("VAR_BLUECARDBALANCE") else {
        return Ok(0);
    };
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(FieldMoveError::InvalidBlueCardBalance {
            value: value.clone(),
        });
    }
    let parsed = value
        .parse::<u16>()
        .map_err(|_| FieldMoveError::InvalidBlueCardBalance {
            value: value.clone(),
        })?;
    if parsed > 30 {
        return Err(FieldMoveError::BlueCardBalanceOutOfRange { balance: parsed });
    }
    Ok(parsed as u8)
}

pub fn validate_saved_blue_card_balance(
    state: &GameState,
    has_buena_prizes: bool,
) -> Result<(), FieldMoveError> {
    if state.blue_card_balance > 0 && !has_buena_prizes {
        return Err(FieldMoveError::MissingBuenaPrizesForSavedBlueCardBalance {
            balance: state.blue_card_balance,
        });
    }
    Ok(())
}

pub fn is_dig_warp_source_environment(environment: &str) -> bool {
    matches!(environment, "ROUTE" | "TOWN")
}

pub fn is_dig_warp_destination_environment(environment: &str) -> bool {
    matches!(environment, "INDOOR" | "CAVE" | "DUNGEON" | "GATE")
}

pub fn is_escape_rope_environment(environment: &str) -> bool {
    matches!(environment, "CAVE" | "DUNGEON")
}

pub fn is_bicycle_environment(environment: &str) -> bool {
    matches!(environment, "ROUTE" | "TOWN" | "CAVE" | "GATE")
}

pub fn is_dig_field_move_environment(environment: &str) -> bool {
    matches!(environment, "CAVE" | "DUNGEON")
}

pub fn is_fly_source_environment(environment: &str) -> bool {
    matches!(environment, "ROUTE" | "TOWN")
}

pub fn is_teleport_source_environment(environment: &str) -> bool {
    matches!(environment, "ROUTE" | "TOWN")
}

pub fn apply_dig_warp_memory_for_transition(
    state: &mut GameState,
    transition: &WarpTransition,
    source_environment: &str,
    destination_environment: &str,
) -> DigWarpMemoryOutcome {
    let before_map_name = state.dig_warp_map_name.clone();
    let before_index = state.dig_warp_index;
    let saved = is_dig_warp_source_environment(source_environment)
        && is_dig_warp_destination_environment(destination_environment)
        && !is_dig_previous_map_blacklisted(&transition.trigger.map_name);
    if saved {
        state.dig_warp_map_name = Some(transition.trigger.map_name.clone());
        state.dig_warp_index = Some(transition.trigger.warp.index);
    } else {
        state.dig_warp_map_name = None;
        state.dig_warp_index = None;
    }
    DigWarpMemoryOutcome {
        before_map_name,
        before_index,
        after_map_name: state.dig_warp_map_name.clone(),
        after_index: state.dig_warp_index,
    }
}

pub fn saved_dig_warp_destination(
    state: &GameState,
    context: &str,
    warps: &[WarpEvent],
) -> Result<SavedDigWarpDestination, FieldMoveError> {
    let map_name =
        state
            .dig_warp_map_name
            .clone()
            .ok_or_else(|| FieldMoveError::MissingSavedDigWarpMap {
                context: context.to_string(),
            })?;
    let warp_index =
        state
            .dig_warp_index
            .ok_or_else(|| FieldMoveError::MissingSavedDigWarpIndex {
                context: context.to_string(),
            })?;
    let warp = warps
        .iter()
        .find(|warp| warp.index == warp_index)
        .ok_or_else(|| FieldMoveError::MissingSavedDigWarp {
            context: context.to_string(),
            map_name: map_name.clone(),
            warp_index,
        })?;
    Ok(SavedDigWarpDestination {
        map_name,
        warp_index,
        tile: TilePosition::new(warp.x as i16, warp.y as i16),
    })
}

pub fn is_dig_previous_map_blacklisted(map_name: &str) -> bool {
    matches!(map_name, "MountMoonSquare" | "TinTowerRoof")
}

fn validate_field_item_id(
    rule_id: &str,
    rule: &FieldItemRule,
    item: &Item,
) -> Result<(), FieldMoveError> {
    require_field_item_rule_id(rule_id, &rule.item_id)?;
    if item.script_name != rule.item_id {
        return Err(FieldMoveError::InvalidFieldItemId {
            rule_id: rule_id.to_string(),
            item_id: item.script_name.clone(),
            expected_item_id: rule.item_id.clone(),
        });
    }
    Ok(())
}

fn require_field_item_rule_id(rule_id: &str, item_id: &str) -> Result<(), FieldMoveError> {
    if item_id.is_empty() {
        return Err(FieldMoveError::MissingFieldItemId {
            rule_id: rule_id.to_string(),
        });
    }
    require_rule_field(item_id, "item_id")
}

fn validate_move_only_field_move(
    rule: &FieldMoveMoveRule,
    storage: &PokemonStorage,
    party_index: usize,
) -> Result<FieldMoveUseOutcome, FieldMoveError> {
    require_rule_field(&rule.move_id, "move_id")?;
    let actor = require_party_move(storage, party_index, &rule.move_id)?;
    Ok(FieldMoveUseOutcome {
        move_id: rule.move_id.clone(),
        actor_party_index: party_index,
        actor_species: actor.species.id.clone(),
    })
}

pub fn validate_direct_field_move_actor(
    storage: &PokemonStorage,
    party_index: usize,
    move_id: &str,
) -> Result<FieldMoveUseOutcome, FieldMoveError> {
    require_rule_field(move_id, "move_id")?;
    let actor = require_party_move(storage, party_index, move_id)?;
    Ok(FieldMoveUseOutcome {
        move_id: move_id.to_string(),
        actor_party_index: party_index,
        actor_species: actor.species.id.clone(),
    })
}

fn require_party_move<'a>(
    storage: &'a PokemonStorage,
    party_index: usize,
    move_id: &str,
) -> Result<&'a crate::models::Pokemon, FieldMoveError> {
    let Some(slot) = storage.party.pokemon.get(party_index) else {
        return Err(FieldMoveError::PartyIndexOutOfRange { party_index });
    };
    let Some(pokemon) = slot.as_ref() else {
        return Err(FieldMoveError::EmptyPartySlot { party_index });
    };
    if !pokemon.moves.iter().any(|known| known.name == move_id) {
        return Err(FieldMoveError::PokemonDoesNotKnowMove {
            party_index,
            move_id: move_id.to_string(),
        });
    }
    Ok(pokemon)
}

fn require_rule_field(value: &str, field: &str) -> Result<(), FieldMoveError> {
    if value.is_empty() {
        return Err(FieldMoveError::MissingRuleField {
            move_id: "<unconfigured>".to_string(),
            field: field.to_string(),
        });
    }
    if !is_exact_field_move_token(value) {
        return Err(FieldMoveError::InvalidRuleField {
            field: field.to_string(),
            value: value.to_string(),
        });
    }
    Ok(())
}

fn require_badge(
    state: &GameState,
    move_id: &str,
    badge: &FieldMoveBadgeRequirement,
) -> Result<(), FieldMoveError> {
    if badge.region != "johto" {
        return Err(FieldMoveError::UnsupportedBadgeRegion {
            move_id: move_id.to_string(),
            region: badge.region.clone(),
        });
    }
    let Some(has_badge) = state.badges.johto.get(badge.index).copied() else {
        return Err(FieldMoveError::InvalidBadgeIndex {
            move_id: move_id.to_string(),
            badge_index: badge.index,
        });
    };
    if has_badge {
        return Ok(());
    }
    Err(FieldMoveError::MissingBadge {
        move_id: move_id.to_string(),
        badge_index: badge.index,
    })
}

fn target_metatile<'a>(
    map: &OverworldMapData,
    tileset: &'a TilesetCollision,
    metatile_x: u16,
    metatile_y: u16,
) -> Result<(usize, u16, &'a MetatileCollision), FieldMoveError> {
    let index = map
        .metatile_index(metatile_x as i16, metatile_y as i16)
        .ok_or_else(|| FieldMoveError::TargetOutOfBounds {
            map_name: map.name.clone(),
            metatile_x,
            metatile_y,
        })?;
    let previous_block_id = map.metatile_ids[index];
    let collisions = tileset.metatiles.get(previous_block_id as usize).ok_or(
        FieldMoveError::MissingMetatileCollision {
            block_id: previous_block_id,
        },
    )?;
    Ok((index, previous_block_id, collisions))
}

fn contains_any_collision(metatile: &MetatileCollision, collisions: &[u8]) -> bool {
    metatile
        .collision
        .iter()
        .any(|permission| collisions.contains(permission))
}

fn record_block_override(
    state: &mut GameState,
    map_name: &str,
    metatile_x: u16,
    metatile_y: u16,
    replacement_block_id: u16,
) {
    state
        .map_block_overrides
        .entry(map_name.to_string())
        .or_default()
        .insert((metatile_x, metatile_y), replacement_block_id);
}

fn set_field_move_engine_flag(
    state: &mut GameState,
    move_id: &str,
    party_index: usize,
    actor_species: &str,
    engine_flag: &str,
) -> Result<FieldMoveFlagOutcome, FieldMoveError> {
    let was_set = state.flags.is_engine_flag_set(engine_flag)?;
    state.flags.set_engine_flag(engine_flag, true)?;
    Ok(FieldMoveFlagOutcome {
        move_id: move_id.to_string(),
        actor_party_index: party_index,
        actor_species: actor_species.to_string(),
        engine_flag: engine_flag.to_string(),
        was_set,
        is_set: true,
    })
}

fn block_replacement<'a>(
    rule: &'a FieldMoveBlockRule,
    tileset_name: &str,
    block_id: u16,
) -> Option<&'a FieldMoveReplacement> {
    rule.replacements
        .get(tileset_name)
        .and_then(|blocks| blocks.get(&block_id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::{MapAttributes, WarpEvent};
    use crate::models::{BaseStats, Dv, Item, LearnedMove, Pokemon, PokemonSpecies, item_pocket};
    use crate::world::collision::permissions;

    const MOVE_CUT: &str = "CUT";
    const MOVE_STRENGTH: &str = "STRENGTH";
    const MOVE_FLASH: &str = "FLASH";
    const MOVE_WHIRLPOOL: &str = "WHIRLPOOL";
    const MOVE_SURF: &str = "SURF";
    const MOVE_WATERFALL: &str = "WATERFALL";
    const MOVE_FLY: &str = "FLY";
    const MOVE_DIG: &str = "DIG";
    const MOVE_TELEPORT: &str = "TELEPORT";
    const BADGE_ZEPHYR: usize = 0;
    const BADGE_HIVE: usize = 1;
    const BADGE_PLAIN: usize = 2;
    const BADGE_FOG: usize = 3;
    const BADGE_STORM: usize = 5;
    const BADGE_GLACIER: usize = 6;
    const BADGE_RISING: usize = 7;
    const COLL_CUT_TREE: u8 = 0x12;
    const COLL_CUT_TREE_ALT: u8 = 0x1a;
    const COLL_TALL_GRASS: u8 = permissions::TALL_GRASS;
    const COLL_LONG_GRASS: u8 = 0x14;
    const COLL_LONG_GRASS_ALT: u8 = 0x1c;
    const COLL_WHIRLPOOL: u8 = permissions::WHIRLPOOL;
    const COLL_WHIRLPOOL_ALT: u8 = permissions::WHIRLPOOL_2C;

    fn badge(index: usize) -> FieldMoveBadgeRequirement {
        FieldMoveBadgeRequirement {
            region: "johto".to_string(),
            index,
        }
    }

    fn replacement(replacement_block_id: u16, variant: &str) -> FieldMoveReplacement {
        FieldMoveReplacement {
            replacement_block_id,
            variant: variant.to_string(),
        }
    }

    fn replacements(
        entries: Vec<(&str, u16, u16, &str)>,
    ) -> BTreeMap<String, BTreeMap<u16, FieldMoveReplacement>> {
        let mut replacements = BTreeMap::new();
        for (tileset, block_id, replacement_block_id, variant) in entries {
            replacements
                .entry(tileset.to_string())
                .or_insert_with(BTreeMap::new)
                .insert(block_id, replacement(replacement_block_id, variant));
        }
        replacements
    }

    fn catalog() -> FieldMoveCatalog {
        FieldMoveCatalog {
            cut: FieldMoveBlockRule {
                move_id: MOVE_CUT.to_string(),
                badge: badge(BADGE_HIVE),
                target_collisions: vec![
                    COLL_CUT_TREE,
                    COLL_CUT_TREE_ALT,
                    COLL_TALL_GRASS,
                    COLL_LONG_GRASS,
                    COLL_LONG_GRASS_ALT,
                ],
                replacements: replacements(vec![
                    ("johto", 0x03, 0x02, "grass"),
                    ("johto", 0x5b, 0x3c, "tree"),
                ]),
            },
            whirlpool: FieldMoveBlockRule {
                move_id: MOVE_WHIRLPOOL.to_string(),
                badge: badge(BADGE_GLACIER),
                target_collisions: vec![COLL_WHIRLPOOL, COLL_WHIRLPOOL_ALT],
                replacements: replacements(vec![("johto", 0x07, 0x36, "whirlpool")]),
            },
            strength: FieldMoveFlagRule {
                move_id: MOVE_STRENGTH.to_string(),
                badge: badge(BADGE_PLAIN),
                engine_flag: "ENGINE_STRENGTH_ACTIVE".to_string(),
            },
            flash: FieldMoveFlagRule {
                move_id: MOVE_FLASH.to_string(),
                badge: badge(BADGE_ZEPHYR),
                engine_flag: "STATUSFLAGS_FLASH".to_string(),
            },
            surf: FieldMoveTravelRule {
                move_id: MOVE_SURF.to_string(),
                badge: badge(BADGE_FOG),
                blocked_collisions: vec![
                    permissions::WHIRLPOOL,
                    permissions::WHIRLPOOL_2C,
                    permissions::WATERFALL,
                    permissions::WATERFALL_RIGHT,
                    permissions::WATERFALL_LEFT,
                    permissions::WATERFALL_UP,
                ],
                target_collisions: Vec::new(),
            },
            waterfall: FieldMoveTravelRule {
                move_id: MOVE_WATERFALL.to_string(),
                badge: badge(BADGE_RISING),
                blocked_collisions: Vec::new(),
                target_collisions: vec![
                    permissions::WATERFALL,
                    permissions::WATERFALL_RIGHT,
                    permissions::WATERFALL_LEFT,
                    permissions::WATERFALL_UP,
                    permissions::CURRENT_DOWN,
                ],
            },
            fly: FieldMoveRule {
                move_id: MOVE_FLY.to_string(),
                badge: badge(BADGE_STORM),
            },
            dig: FieldMoveMoveRule {
                move_id: MOVE_DIG.to_string(),
            },
            teleport: FieldMoveMoveRule {
                move_id: MOVE_TELEPORT.to_string(),
            },
            escape_rope: FieldEscapeItemRule {
                item_id: "ESCAPE_ROPE".to_string(),
                escape_rope_mode: "DIG_WARP".to_string(),
            },
            repel: FieldRepelItemRule {},
            bicycle: FieldItemRule {
                item_id: "BICYCLE".to_string(),
            },
            itemfinder: FieldItemRule {
                item_id: "ITEMFINDER".to_string(),
            },
            squirtbottle: FieldItemRule {
                item_id: "SQUIRTBOTTLE".to_string(),
            },
            coin_case: FieldItemRule {
                item_id: "COIN_CASE".to_string(),
            },
            blue_card: FieldItemRule {
                item_id: "BLUE_CARD".to_string(),
            },
            town_map: FieldItemRule {
                item_id: "TOWN_MAP".to_string(),
            },
        }
    }

    fn attributes(tileset_name: &str) -> MapAttributes {
        attributes_with_size(tileset_name, 2, 1)
    }

    fn attributes_with_size(tileset_name: &str, width: u16, height: u16) -> MapAttributes {
        MapAttributes {
            tileset_name: tileset_name.to_string(),
            border_block: 0,
            width,
            height,
            connections: Vec::new(),
            time_of_day: None,
            phone_service: 0,
            phone_flag: false,
            environment: None,
            location: None,
            music: None,
            palette: None,
            fishing_group: None,
            map_constant: None,
            map_group_constant: None,
            blocks_label: None,
            map_scripts_label: None,
            map_events_label: None,
            connection_flags: None,
        }
    }

    fn map(blocks: Vec<u16>) -> OverworldMapData {
        OverworldMapData::from_attributes("Route29", &attributes("johto"), blocks)
    }

    fn map_with_size(width: u16, height: u16, blocks: Vec<u16>) -> OverworldMapData {
        OverworldMapData::from_attributes(
            "Route29",
            &attributes_with_size("johto", width, height),
            blocks,
        )
    }

    fn tileset() -> TilesetCollision {
        let mut metatiles = vec![
            MetatileCollision {
                collision: [permissions::FLOOR; 4],
            };
            0x68
        ];
        metatiles[0x03] = MetatileCollision {
            collision: [COLL_TALL_GRASS; 4],
        };
        metatiles[0x07] = MetatileCollision {
            collision: [COLL_WHIRLPOOL; 4],
        };
        metatiles[0x08] = MetatileCollision {
            collision: [permissions::WATER; 4],
        };
        metatiles[0x09] = MetatileCollision {
            collision: [permissions::WATERFALL; 4],
        };
        metatiles[0x5b] = MetatileCollision {
            collision: [COLL_CUT_TREE; 4],
        };
        TilesetCollision { metatiles }
    }

    fn pokemon_with_move(move_id: &str) -> Pokemon {
        let species =
            PokemonSpecies::new_for_tests("CHIKORITA", BaseStats::new(45, 49, 65, 45, 49, 65));
        let mut pokemon = Pokemon::new_for_tests(species, 8, Dv::default());
        pokemon.moves = vec![LearnedMove {
            name: move_id.to_string(),
            current_pp: 15,
            pp_ups: 0,
        }];
        pokemon
    }

    fn storage_with(move_id: &str) -> PokemonStorage {
        let mut storage = PokemonStorage::default();
        storage
            .register_capture(pokemon_with_move(move_id))
            .expect("register test pokemon");
        storage
    }

    fn escape_item(effect: &str, mode: Option<&str>) -> Item {
        Item {
            name: "Escape Rope".to_string(),
            description: String::new(),
            effect: effect.to_string(),
            status_heals: Vec::new(),
            revive_hp_percent: None,
            party_revive_hp_percent: None,
            pp_restore_scope: None,
            pp_restore_points: None,
            pp_up_stages: None,
            vitamin_stat: None,
            vitamin_stat_exp: None,
            vitamin_max_stat_exp: None,
            rare_candy_level_gain: None,
            battle_stat_boost_stat: None,
            battle_stat_boost_stages: None,
            battle_escape_mode: None,
            battle_focus_energy: None,
            battle_stat_drop_guard: None,
            battle_stat_drop_guard_turns: None,
            confusion_heal: None,
            repel_steps: None,
            escape_rope_mode: mode.map(str::to_string),
            price: 550,
            held_effect: "HELD_NONE".to_string(),
            parameter: 0,
            property: "CANT_SELECT".to_string(),
            pocket: item_pocket("ITEM"),
            field_menu: "ITEMMENU_CLOSE".to_string(),
            field_usable: true,
            battle_menu: "ITEMMENU_NOUSE".to_string(),
            battle_usable: false,
            script_name: "ESCAPE_ROPE".to_string(),
            consumable: true,
            tmhm_index: None,
            tmhm_move: None,
        }
    }

    fn repel_item(effect: &str, steps: Option<u16>) -> Item {
        Item {
            name: "Repel".to_string(),
            description: String::new(),
            effect: effect.to_string(),
            status_heals: Vec::new(),
            revive_hp_percent: None,
            party_revive_hp_percent: None,
            pp_restore_scope: None,
            pp_restore_points: None,
            pp_up_stages: None,
            vitamin_stat: None,
            vitamin_stat_exp: None,
            vitamin_max_stat_exp: None,
            rare_candy_level_gain: None,
            battle_stat_boost_stat: None,
            battle_stat_boost_stages: None,
            battle_escape_mode: None,
            battle_focus_energy: None,
            battle_stat_drop_guard: None,
            battle_stat_drop_guard_turns: None,
            confusion_heal: None,
            repel_steps: steps,
            escape_rope_mode: None,
            price: 350,
            held_effect: "HELD_NONE".to_string(),
            parameter: 0,
            property: "CANT_SELECT".to_string(),
            pocket: item_pocket("ITEM"),
            field_menu: "ITEMMENU_CLOSE".to_string(),
            field_usable: true,
            battle_menu: "ITEMMENU_NOUSE".to_string(),
            battle_usable: false,
            script_name: "MOD_REPEL".to_string(),
            consumable: true,
            tmhm_index: None,
            tmhm_move: None,
        }
    }

    fn field_effect_item(script_name: &str, effect: &str) -> Item {
        let mut item = repel_item(effect, None);
        item.name = script_name.to_string();
        item.script_name = script_name.to_string();
        item
    }

    #[test]
    fn cut_replaces_exact_tree_block_and_records_save_override() {
        let mut state = GameState::default();
        state.badges.johto[BADGE_HIVE] = true;
        let storage = storage_with(MOVE_CUT);
        let mut map = map(vec![0x5b, 0x00]);

        let outcome = apply_cut_field_move(
            &catalog(),
            &mut state,
            &storage,
            &mut map,
            &tileset(),
            "johto",
            0,
            0,
            0,
        )
        .expect("cut tree");

        assert_eq!(outcome.previous_block_id, 0x5b);
        assert_eq!(outcome.replacement_block_id, 0x3c);
        assert_eq!(outcome.variant, "tree");
        assert_eq!(map.metatile_at(0, 0), Some(0x3c));
        assert_eq!(
            state
                .map_block_overrides
                .get("Route29")
                .and_then(|overrides| overrides.get(&(0, 0)))
                .copied(),
            Some(0x3c)
        );
    }

    #[test]
    fn cut_rejects_missing_badge_without_mutating_map() {
        let mut state = GameState::default();
        let storage = storage_with(MOVE_CUT);
        let mut map = map(vec![0x5b, 0x00]);

        let error = apply_cut_field_move(
            &catalog(),
            &mut state,
            &storage,
            &mut map,
            &tileset(),
            "johto",
            0,
            0,
            0,
        )
        .expect_err("missing badge");

        assert_eq!(
            error,
            FieldMoveError::MissingBadge {
                move_id: MOVE_CUT.to_string(),
                badge_index: BADGE_HIVE,
            }
        );
        assert_eq!(map.metatile_at(0, 0), Some(0x5b));
        assert!(state.map_block_overrides.is_empty());
    }

    #[test]
    fn whirlpool_replaces_exact_block() {
        let mut state = GameState::default();
        state.badges.johto[BADGE_GLACIER] = true;
        let storage = storage_with(MOVE_WHIRLPOOL);
        let mut map = map(vec![0x07, 0x00]);

        let outcome = apply_whirlpool_field_move(
            &catalog(),
            &mut state,
            &storage,
            &mut map,
            &tileset(),
            "johto",
            0,
            0,
            0,
        )
        .expect("clear whirlpool");

        assert_eq!(outcome.replacement_block_id, 0x36);
        assert_eq!(outcome.variant, "whirlpool");
        assert_eq!(map.metatile_at(0, 0), Some(0x36));
    }

    #[test]
    fn strength_sets_exact_engine_flag() {
        let mut state = GameState::default();
        state.badges.johto[BADGE_PLAIN] = true;
        let storage = storage_with(MOVE_STRENGTH);

        let outcome =
            apply_strength_field_move(&catalog(), &mut state, &storage, 0).expect("use strength");

        assert_eq!(outcome.engine_flag, "ENGINE_STRENGTH_ACTIVE");
        assert!(!outcome.was_set);
        assert_eq!(
            state.flags.is_engine_flag_set("ENGINE_STRENGTH_ACTIVE"),
            Ok(true)
        );
    }

    #[test]
    fn flash_sets_exact_engine_flag() {
        let mut state = GameState::default();
        state.badges.johto[BADGE_ZEPHYR] = true;
        let storage = storage_with(MOVE_FLASH);

        let outcome =
            apply_flash_field_move(&catalog(), &mut state, &storage, 0).expect("use flash");

        assert_eq!(outcome.engine_flag, "STATUSFLAGS_FLASH");
        assert_eq!(
            state.flags.is_engine_flag_set("STATUSFLAGS_FLASH"),
            Ok(true)
        );
    }

    #[test]
    fn surf_enters_facing_water_tile_and_sets_surf_mode() {
        let mut state = GameState::default();
        state.badges.johto[BADGE_FOG] = true;
        let storage = storage_with(MOVE_SURF);
        let map = map(vec![0x00, 0x08]);
        let mut player = PlayerMovementState::new(TilePosition::new(0, 0));
        player.facing = Direction::Right;

        let outcome = apply_surf_field_move(
            &catalog(),
            &state,
            &storage,
            &map,
            &tileset(),
            &mut player,
            0,
        )
        .expect("surf");

        assert_eq!(outcome.from_tile, TilePosition::new(0, 0));
        assert_eq!(outcome.to_tile, TilePosition::new(2, 0));
        assert_eq!(outcome.steps, 1);
        assert_eq!(player.mode, MovementMode::Surf);
        assert_eq!(player.tile, TilePosition::new(2, 0));
    }

    #[test]
    fn surf_rejects_non_water_target_without_moving() {
        let mut state = GameState::default();
        state.badges.johto[BADGE_FOG] = true;
        let storage = storage_with(MOVE_SURF);
        let map = map(vec![0x00, 0x00]);
        let mut player = PlayerMovementState::new(TilePosition::new(0, 0));
        player.facing = Direction::Right;

        let error = apply_surf_field_move(
            &catalog(),
            &state,
            &storage,
            &map,
            &tileset(),
            &mut player,
            0,
        )
        .expect_err("not water");

        assert_eq!(
            error,
            FieldMoveError::TargetNotWater {
                move_id: MOVE_SURF.to_string()
            }
        );
        assert_eq!(player.mode, MovementMode::Normal);
        assert_eq!(player.tile, TilePosition::new(0, 0));
    }

    #[test]
    fn waterfall_climbs_until_leaving_waterfall_collision() {
        let mut state = GameState::default();
        state.badges.johto[BADGE_RISING] = true;
        let storage = storage_with(MOVE_WATERFALL);
        let map = map_with_size(1, 4, vec![0x08, 0x09, 0x09, 0x08]);
        let mut player = PlayerMovementState::new(TilePosition::new(1, 7));
        player.facing = Direction::Up;
        player.mode = MovementMode::Surf;

        let outcome = apply_waterfall_field_move(
            &catalog(),
            &state,
            &storage,
            &map,
            &tileset(),
            &mut player,
            0,
        )
        .expect("waterfall");

        assert_eq!(outcome.from_tile, TilePosition::new(1, 7));
        assert_eq!(outcome.to_tile, TilePosition::new(1, 1));
        assert_eq!(outcome.steps, 3);
        assert_eq!(player.mode, MovementMode::Surf);
        assert_eq!(player.tile, TilePosition::new(1, 1));
    }

    #[test]
    fn waterfall_requires_rising_badge_without_moving() {
        let state = GameState::default();
        let storage = storage_with(MOVE_WATERFALL);
        let map = map_with_size(1, 2, vec![0x09, 0x08]);
        let mut player = PlayerMovementState::new(TilePosition::new(1, 3));
        player.facing = Direction::Up;
        player.mode = MovementMode::Surf;

        let error = apply_waterfall_field_move(
            &catalog(),
            &state,
            &storage,
            &map,
            &tileset(),
            &mut player,
            0,
        )
        .expect_err("missing badge");

        assert_eq!(
            error,
            FieldMoveError::MissingBadge {
                move_id: MOVE_WATERFALL.to_string(),
                badge_index: BADGE_RISING,
            }
        );
        assert_eq!(player.tile, TilePosition::new(1, 3));
    }

    #[test]
    fn fly_uses_catalog_move_and_badge_requirement_without_hardcoded_badge() {
        let mut catalog = catalog();
        catalog.fly.badge = badge(BADGE_ZEPHYR);
        let mut state = GameState::default();
        let storage = storage_with(MOVE_FLY);

        let missing = validate_fly_field_move(&catalog, &state, &storage, 0)
            .expect_err("catalog badge is required");
        assert_eq!(
            missing,
            FieldMoveError::MissingBadge {
                move_id: MOVE_FLY.to_string(),
                badge_index: BADGE_ZEPHYR,
            }
        );

        state.badges.johto[BADGE_ZEPHYR] = true;
        let outcome =
            validate_fly_field_move(&catalog, &state, &storage, 0).expect("catalog badge set");

        assert_eq!(outcome.move_id, MOVE_FLY);
        assert_eq!(outcome.actor_party_index, 0);
        assert_eq!(outcome.actor_species, "CHIKORITA");
    }

    #[test]
    fn field_move_badge_index_outside_region_is_invalid_rule_data() {
        let mut catalog = catalog();
        catalog.fly.badge = badge(8);
        let state = GameState::default();
        let storage = storage_with(MOVE_FLY);

        assert_eq!(
            validate_fly_field_move(&catalog, &state, &storage, 0),
            Err(FieldMoveError::InvalidBadgeIndex {
                move_id: MOVE_FLY.to_string(),
                badge_index: 8,
            })
        );
    }

    #[test]
    fn dig_and_teleport_use_catalog_move_ids_without_hardcoded_move_names() {
        let mut catalog = catalog();
        catalog.dig.move_id = MOVE_TELEPORT.to_string();
        catalog.teleport.move_id = MOVE_DIG.to_string();
        let storage = storage_with(MOVE_TELEPORT);

        let dig = validate_dig_field_move(&catalog, &storage, 0)
            .expect("dig validator uses catalog move id");
        assert_eq!(dig.move_id, MOVE_TELEPORT);
        assert_eq!(dig.actor_species, "CHIKORITA");

        let teleport = validate_teleport_field_move(&catalog, &storage, 0)
            .expect_err("teleport validator requires its own catalog move id");
        assert_eq!(
            teleport,
            FieldMoveError::PokemonDoesNotKnowMove {
                party_index: 0,
                move_id: MOVE_DIG.to_string(),
            }
        );
    }

    #[test]
    fn direct_field_move_actor_validation_uses_exact_move_id() {
        let storage = storage_with("HEADBUTT");

        assert_eq!(
            validate_direct_field_move_actor(&storage, 0, "HEADBUTT"),
            Ok(FieldMoveUseOutcome {
                move_id: "HEADBUTT".to_string(),
                actor_party_index: 0,
                actor_species: "CHIKORITA".to_string(),
            })
        );
        assert_eq!(
            validate_direct_field_move_actor(&storage, 0, "headbutt"),
            Err(FieldMoveError::PokemonDoesNotKnowMove {
                party_index: 0,
                move_id: "headbutt".to_string(),
            })
        );
        assert_eq!(
            validate_direct_field_move_actor(&storage, 0, "SWEET_SCENT"),
            Err(FieldMoveError::PokemonDoesNotKnowMove {
                party_index: 0,
                move_id: "SWEET_SCENT".to_string(),
            })
        );
    }

    #[test]
    fn blue_card_balance_reads_exact_script_variable_without_fallback() {
        let mut state = GameState::default();
        assert_eq!(blue_card_balance(&state), Ok(0));

        state
            .script_runtime
            .variables
            .insert("VAR_BLUECARDBALANCE".to_string(), "30".to_string());
        assert_eq!(blue_card_balance(&state), Ok(30));

        state
            .script_runtime
            .variables
            .insert("VAR_BLUECARDBALANCE".to_string(), " 3".to_string());
        assert_eq!(
            blue_card_balance(&state),
            Err(FieldMoveError::InvalidBlueCardBalance {
                value: " 3".to_string(),
            })
        );

        state
            .script_runtime
            .variables
            .insert("VAR_BLUECARDBALANCE".to_string(), "31".to_string());
        assert_eq!(
            blue_card_balance(&state),
            Err(FieldMoveError::BlueCardBalanceOutOfRange { balance: 31 })
        );

        state.script_runtime.variables.insert(
            "VAR_BLUECARDBALANCE".to_string(),
            "999999999999".to_string(),
        );
        assert_eq!(
            blue_card_balance(&state),
            Err(FieldMoveError::InvalidBlueCardBalance {
                value: "999999999999".to_string(),
            })
        );
    }

    #[test]
    fn field_move_environment_predicates_match_crystal_categories_exactly() {
        for environment in ["ROUTE", "TOWN"] {
            assert!(is_dig_warp_source_environment(environment));
            assert!(is_fly_source_environment(environment));
            assert!(is_teleport_source_environment(environment));
        }
        for environment in ["INDOOR", "CAVE", "DUNGEON", "GATE"] {
            assert!(is_dig_warp_destination_environment(environment));
        }
        for environment in ["CAVE", "DUNGEON"] {
            assert!(is_escape_rope_environment(environment));
            assert!(is_dig_field_move_environment(environment));
        }
        for environment in ["ROUTE", "TOWN", "CAVE", "GATE"] {
            assert!(is_bicycle_environment(environment));
        }
        for environment in ["route", "INDOORS", "FOREST", ""] {
            assert!(!is_dig_warp_source_environment(environment));
            assert!(!is_dig_warp_destination_environment(environment));
            assert!(!is_escape_rope_environment(environment));
            assert!(!is_bicycle_environment(environment));
            assert!(!is_dig_field_move_environment(environment));
            assert!(!is_fly_source_environment(environment));
            assert!(!is_teleport_source_environment(environment));
        }
    }

    fn warp_transition_from(map_name: &str, index: u16) -> WarpTransition {
        let warp = WarpEvent {
            index,
            x: 3,
            y: 4,
            target_map_constant: "DESTINATION".to_string(),
            target_map: "DESTINATION".to_string(),
            target_warp_id: 1,
        };
        WarpTransition {
            trigger: crate::world::session::WarpTrigger {
                map_name: map_name.to_string(),
                tile: TilePosition::new(3, 4),
                warp: warp.clone(),
            },
            destination: crate::world::session::WarpDestination {
                map_name: "DESTINATION".to_string(),
                tile: TilePosition::new(1, 2),
                warp,
            },
        }
    }

    #[test]
    fn dig_warp_memory_saves_only_valid_source_destination_transitions() {
        let mut state = GameState {
            dig_warp_map_name: Some("OLD_MAP".to_string()),
            dig_warp_index: Some(9),
            ..GameState::default()
        };
        let transition = warp_transition_from("ROUTE_29", 3);

        assert_eq!(
            apply_dig_warp_memory_for_transition(&mut state, &transition, "ROUTE", "CAVE"),
            DigWarpMemoryOutcome {
                before_map_name: Some("OLD_MAP".to_string()),
                before_index: Some(9),
                after_map_name: Some("ROUTE_29".to_string()),
                after_index: Some(3),
            }
        );
        assert_eq!(state.dig_warp_map_name.as_deref(), Some("ROUTE_29"));
        assert_eq!(state.dig_warp_index, Some(3));

        assert_eq!(
            apply_dig_warp_memory_for_transition(&mut state, &transition, "CAVE", "ROUTE"),
            DigWarpMemoryOutcome {
                before_map_name: Some("ROUTE_29".to_string()),
                before_index: Some(3),
                after_map_name: None,
                after_index: None,
            }
        );
        assert_eq!(state.dig_warp_map_name, None);
        assert_eq!(state.dig_warp_index, None);
    }

    #[test]
    fn dig_warp_memory_rejects_blacklisted_previous_maps() {
        let mut state = GameState::default();
        let transition = warp_transition_from("TinTowerRoof", 2);

        assert_eq!(
            apply_dig_warp_memory_for_transition(&mut state, &transition, "TOWN", "INDOOR"),
            DigWarpMemoryOutcome {
                before_map_name: None,
                before_index: None,
                after_map_name: None,
                after_index: None,
            }
        );
        assert!(is_dig_previous_map_blacklisted("TinTowerRoof"));
    }

    #[test]
    fn saved_dig_warp_destination_requires_exact_saved_map_and_warp_index() {
        let warps = vec![
            WarpEvent {
                index: 1,
                x: 5,
                y: 7,
                target_map_constant: "DESTINATION".to_string(),
                target_map: "DESTINATION".to_string(),
                target_warp_id: 1,
            },
            WarpEvent {
                index: 3,
                x: 9,
                y: 11,
                target_map_constant: "DESTINATION".to_string(),
                target_map: "DESTINATION".to_string(),
                target_warp_id: 2,
            },
        ];
        let mut state = GameState::default();

        assert_eq!(
            saved_dig_warp_destination(&state, "DIG field move", &warps),
            Err(FieldMoveError::MissingSavedDigWarpMap {
                context: "DIG field move".to_string(),
            })
        );

        state.dig_warp_map_name = Some("ROUTE_29".to_string());
        assert_eq!(
            saved_dig_warp_destination(&state, "DIG field move", &warps),
            Err(FieldMoveError::MissingSavedDigWarpIndex {
                context: "DIG field move".to_string(),
            })
        );

        state.dig_warp_index = Some(2);
        assert_eq!(
            saved_dig_warp_destination(&state, "DIG field move", &warps),
            Err(FieldMoveError::MissingSavedDigWarp {
                context: "DIG field move".to_string(),
                map_name: "ROUTE_29".to_string(),
                warp_index: 2,
            })
        );

        state.dig_warp_index = Some(3);
        assert_eq!(
            saved_dig_warp_destination(&state, "DIG field move", &warps),
            Ok(SavedDigWarpDestination {
                map_name: "ROUTE_29".to_string(),
                warp_index: 3,
                tile: TilePosition::new(9, 11),
            })
        );
    }

    #[test]
    fn move_only_field_move_rejects_malformed_catalog_move_id_before_party_check() {
        let mut catalog = catalog();
        catalog.dig.move_id = "DI G".to_string();
        let storage = storage_with(MOVE_DIG);

        assert_eq!(
            validate_dig_field_move(&catalog, &storage, 0),
            Err(FieldMoveError::InvalidRuleField {
                field: "move_id".to_string(),
                value: "DI G".to_string(),
            })
        );
    }

    #[test]
    fn escape_rope_item_uses_catalog_item_id_and_mode_without_effect_join() {
        let mut catalog = catalog();
        catalog.escape_rope.item_id = "MOD_ESCAPE_ROPE".to_string();
        catalog.escape_rope.escape_rope_mode = "MOD_WARP".to_string();

        let wrong_item_id =
            validate_field_escape_item(&catalog, &escape_item("ESCAPE_ROPE", Some("MOD_WARP")))
                .expect_err("item id comes from catalog");
        assert_eq!(
            wrong_item_id,
            FieldMoveError::InvalidEscapeItemId {
                item_id: "ESCAPE_ROPE".to_string(),
                expected_item_id: "MOD_ESCAPE_ROPE".to_string(),
            }
        );

        let mut mod_escape_rope = escape_item("UNRELATED_EFFECT", Some("DIG_WARP"));
        mod_escape_rope.script_name = "MOD_ESCAPE_ROPE".to_string();
        let wrong_mode = validate_field_escape_item(&catalog, &mod_escape_rope)
            .expect_err("mode comes from catalog");
        assert_eq!(
            wrong_mode,
            FieldMoveError::InvalidEscapeItemMode {
                item_id: "MOD_ESCAPE_ROPE".to_string(),
                mode: Some("DIG_WARP".to_string()),
                expected_mode: "MOD_WARP".to_string(),
            }
        );

        mod_escape_rope.escape_rope_mode = Some("MOD_WARP".to_string());
        validate_field_escape_item(&catalog, &mod_escape_rope)
            .expect("catalog item id and mode accepted");
    }

    #[test]
    fn escape_rope_rejects_malformed_catalog_rule_fields_before_item_check() {
        let mut catalog = catalog();
        catalog.escape_rope.item_id = "ESCAPE ROPE".to_string();

        assert_eq!(
            validate_field_escape_item(&catalog, &escape_item("ESCAPE_ROPE", Some("DIG_WARP"))),
            Err(FieldMoveError::InvalidRuleField {
                field: "item_id".to_string(),
                value: "ESCAPE ROPE".to_string(),
            })
        );

        catalog.escape_rope.item_id = "ESCAPE_ROPE".to_string();
        catalog.escape_rope.escape_rope_mode = "DIG WARP".to_string();
        assert_eq!(
            validate_field_escape_item(&catalog, &escape_item("ESCAPE_ROPE", Some("DIG_WARP"))),
            Err(FieldMoveError::InvalidRuleField {
                field: "escape_rope_mode".to_string(),
                value: "DIG WARP".to_string(),
            })
        );
    }

    #[test]
    fn repel_item_uses_repel_steps_payload_without_effect_join() {
        let catalog = catalog();
        let missing_steps = validate_repel_item(&catalog, &repel_item("MOD_REPEL", None))
            .expect_err("repel steps are required");
        assert_eq!(
            missing_steps,
            FieldMoveError::MissingRepelItemSteps {
                item_id: "MOD_REPEL".to_string(),
            }
        );

        assert_eq!(
            validate_repel_item(&catalog, &repel_item("ANY_PACK_EFFECT", Some(75)))
                .expect("repel payload accepted"),
            75
        );
        assert_eq!(
            validate_saved_active_repel_item(&catalog, "MOD_REPEL", None, 10),
            Err(FieldMoveError::MissingSavedActiveRepelItem {
                item_id: "MOD_REPEL".to_string(),
            })
        );
        assert_eq!(
            validate_saved_active_repel_item(
                &catalog,
                "MOD_REPEL",
                Some(&repel_item("MOD_REPEL", Some(75))),
                76,
            ),
            Err(FieldMoveError::SavedRepelStepsExceedCompiledDuration {
                item_id: "MOD_REPEL".to_string(),
                steps_remaining: 76,
                compiled_steps: 75,
            })
        );
    }

    #[test]
    fn repel_item_use_commits_exact_item_and_replaces_existing_repel_state() {
        let mut state = GameState::default();
        state.repel_steps_remaining = 3;
        state.active_repel_item = Some("OLD_REPEL".to_string());

        let outcome = apply_repel_item_use(&mut state, "MOD_REPEL", 100);

        assert_eq!(outcome.item_id, "MOD_REPEL");
        assert_eq!(outcome.repel_steps_before, 3);
        assert_eq!(outcome.repel_steps_after, 100);
        assert_eq!(
            outcome.active_repel_item_before,
            Some("OLD_REPEL".to_string())
        );
        assert_eq!(
            outcome.active_repel_item_after,
            Some("MOD_REPEL".to_string())
        );
        assert_eq!(state.repel_steps_remaining, 100);
        assert_eq!(state.active_repel_item, Some("MOD_REPEL".to_string()));
    }

    #[test]
    fn bicycle_item_uses_catalog_item_id_without_effect_join() {
        let mut catalog = catalog();
        catalog.bicycle.item_id = "MOD_BICYCLE".to_string();

        let wrong_item_id = validate_bicycle_item(
            &catalog,
            &field_effect_item("BICYCLE", "MOD_BICYCLE_EFFECT"),
        )
        .expect_err("bicycle item id comes from catalog");
        assert_eq!(
            wrong_item_id,
            FieldMoveError::InvalidFieldItemId {
                rule_id: "bicycle".to_string(),
                item_id: "BICYCLE".to_string(),
                expected_item_id: "MOD_BICYCLE".to_string(),
            }
        );

        validate_bicycle_item(
            &catalog,
            &field_effect_item("MOD_BICYCLE", "UNRELATED_EFFECT"),
        )
        .expect("catalog bicycle item id accepted");
    }

    #[test]
    fn bicycle_item_rejects_malformed_catalog_item_id_before_item_check() {
        let mut catalog = catalog();
        catalog.bicycle.item_id = "MOD BICYCLE".to_string();

        assert_eq!(
            validate_bicycle_item(
                &catalog,
                &field_effect_item("MOD_BICYCLE", "BICYCLE_EFFECT")
            ),
            Err(FieldMoveError::InvalidRuleField {
                field: "item_id".to_string(),
                value: "MOD BICYCLE".to_string(),
            })
        );
    }

    #[test]
    fn field_key_items_use_catalog_item_ids_without_effect_joins() {
        let mut catalog = catalog();
        let cases: [(
            &str,
            fn(&mut FieldMoveCatalog) -> &mut FieldItemRule,
            fn(&FieldMoveCatalog, &Item) -> Result<(), FieldMoveError>,
            &str,
            &str,
        ); 5] = [
            (
                "itemfinder",
                |catalog| &mut catalog.itemfinder,
                validate_itemfinder_item,
                "ITEMFINDER",
                "MOD_ITEMFINDER_ITEM",
            ),
            (
                "squirtbottle",
                |catalog| &mut catalog.squirtbottle,
                validate_squirtbottle_item,
                "SQUIRTBOTTLE",
                "MOD_SQUIRTBOTTLE_ITEM",
            ),
            (
                "coin_case",
                |catalog| &mut catalog.coin_case,
                validate_coin_case_item,
                "COIN_CASE",
                "MOD_COIN_CASE_ITEM",
            ),
            (
                "blue_card",
                |catalog| &mut catalog.blue_card,
                validate_blue_card_item,
                "BLUE_CARD",
                "MOD_BLUE_CARD_ITEM",
            ),
            (
                "town_map",
                |catalog| &mut catalog.town_map,
                validate_town_map_item,
                "TOWN_MAP",
                "MOD_TOWN_MAP_ITEM",
            ),
        ];

        for (rule_id, rule, validate, default_item_id, mod_item_id) in cases {
            rule(&mut catalog).item_id = mod_item_id.to_string();
            let wrong_item_id =
                validate(&catalog, &field_effect_item(default_item_id, mod_item_id))
                    .expect_err("default item id is not accepted after catalog override");
            assert_eq!(
                wrong_item_id,
                FieldMoveError::InvalidFieldItemId {
                    rule_id: rule_id.to_string(),
                    item_id: default_item_id.to_string(),
                    expected_item_id: mod_item_id.to_string(),
                }
            );
            validate(
                &catalog,
                &field_effect_item(mod_item_id, "UNRELATED_EFFECT"),
            )
            .expect("catalog item id accepted");
        }
    }

    #[test]
    fn field_move_catalog_issues_validate_exact_move_rules() {
        let mut catalog = FieldMoveCatalog::default();
        catalog.cut = FieldMoveBlockRule {
            move_id: "CUT".to_string(),
            badge: FieldMoveBadgeRequirement {
                region: "kanto".to_string(),
                index: 8,
            },
            target_collisions: Vec::new(),
            replacements: replacements(vec![
                ("", 1, 2, "tree"),
                ("johto cave", 3, 4, "tall grass"),
                ("JOHTO_CAVE", 3, 6, "TREE"),
                ("johto", 7, 7, "tree"),
            ]),
        };
        catalog.fly = FieldMoveRule {
            move_id: "FL Y".to_string(),
            badge: badge(BADGE_STORM),
        };
        catalog.strength = FieldMoveFlagRule {
            move_id: "STRENGTH".to_string(),
            badge: badge(BADGE_PLAIN),
            engine_flag: "ENGINE STRENGTH".to_string(),
        };
        catalog.waterfall = FieldMoveTravelRule {
            move_id: "WATERFALL".to_string(),
            badge: badge(BADGE_RISING),
            blocked_collisions: Vec::new(),
            target_collisions: Vec::new(),
        };

        let moves = BTreeSet::from([
            "STRENGTH".to_string(),
            "WATERFALL".to_string(),
            "FLY".to_string(),
        ]);
        let issues = field_move_catalog_issues(&catalog, &moves, &BTreeMap::new());

        assert!(issues.contains(&FieldMoveCatalogIssue::UnknownMoveId {
            subject: "field_moves:cut".to_string(),
            move_id: "CUT".to_string(),
        }));
        assert!(issues.contains(&FieldMoveCatalogIssue::InvalidBadgeRegion {
            subject: "field_moves:cut".to_string(),
            move_id: "CUT".to_string(),
            region: "kanto".to_string(),
        }));
        assert!(issues.contains(&FieldMoveCatalogIssue::InvalidBadgeIndex {
            subject: "field_moves:cut".to_string(),
            move_id: "CUT".to_string(),
            index: 8,
        }));
        assert!(
            issues.contains(&FieldMoveCatalogIssue::MissingTargetCollisions {
                subject: "field_moves:cut".to_string(),
                move_id: "CUT".to_string(),
            })
        );
        assert!(
            issues.contains(&FieldMoveCatalogIssue::InvalidReplacementTileset {
                subject: "field_moves:cut:replacements:".to_string(),
            })
        );
        assert!(
            issues.contains(&FieldMoveCatalogIssue::InvalidReplacementVariant {
                subject: "field_moves:cut:replacements:johto cave:3".to_string(),
            })
        );
        assert!(
            issues.contains(&FieldMoveCatalogIssue::InvalidReplacementBlock {
                subject: "field_moves:cut:replacements:johto:7".to_string(),
                block_id: 7,
            })
        );
        assert!(issues.contains(&FieldMoveCatalogIssue::InvalidMoveId {
            subject: "field_moves:fly".to_string(),
        }));
        assert!(issues.contains(&FieldMoveCatalogIssue::InvalidEngineFlag {
            subject: "field_moves:strength".to_string(),
            move_id: "STRENGTH".to_string(),
        }));
        assert!(
            issues.contains(&FieldMoveCatalogIssue::MissingTargetCollisions {
                subject: "field_moves:waterfall".to_string(),
                move_id: "WATERFALL".to_string(),
            })
        );
    }

    #[test]
    fn field_move_catalog_issues_validate_exact_item_payloads() {
        let mut catalog = FieldMoveCatalog::default();
        catalog.escape_rope = FieldEscapeItemRule {
            item_id: "MOD_ESCAPE_ROPE".to_string(),
            escape_rope_mode: "MOD_WARP".to_string(),
        };
        catalog.repel = FieldRepelItemRule {};
        catalog.bicycle = FieldItemRule {
            item_id: "MOD_BICYCLE".to_string(),
        };
        catalog.itemfinder = FieldItemRule {
            item_id: " ITEMFINDER".to_string(),
        };
        catalog.coin_case = FieldItemRule {
            item_id: "COIN CASE".to_string(),
        };

        let mut escape_rope = escape_item("ESCAPE_ROPE", Some("DIG_WARP"));
        escape_rope.escape_rope_mode = Some("DIG_WARP".to_string());
        let bicycle = field_effect_item("BICYCLE", "BICYCLE");
        let items = BTreeMap::from([
            ("ESCAPE_ROPE".to_string(), escape_rope),
            ("BICYCLE".to_string(), bicycle),
        ]);

        let issues = field_move_catalog_issues(&catalog, &BTreeSet::new(), &items);

        assert!(
            issues.contains(&FieldMoveCatalogIssue::UnknownEscapeItemRule {
                item_id: "MOD_ESCAPE_ROPE".to_string(),
                escape_rope_mode: "MOD_WARP".to_string(),
            })
        );
        assert!(issues.contains(&FieldMoveCatalogIssue::MissingRepelItemPayload));
        assert!(issues.contains(&FieldMoveCatalogIssue::UnknownFieldItemId {
            subject: "field_moves:bicycle".to_string(),
            item_id: "MOD_BICYCLE".to_string(),
        }));
        assert!(issues.contains(&FieldMoveCatalogIssue::InvalidFieldItemId {
            subject: "field_moves:itemfinder".to_string(),
        }));
        assert!(issues.contains(&FieldMoveCatalogIssue::InvalidFieldItemId {
            subject: "field_moves:coin_case".to_string(),
        }));
    }

    #[test]
    fn field_move_catalog_issues_reject_declared_items_that_are_not_field_usable() {
        let mut catalog = FieldMoveCatalog::default();
        catalog.escape_rope = FieldEscapeItemRule {
            item_id: "ESCAPE_ROPE".to_string(),
            escape_rope_mode: "DIG_WARP".to_string(),
        };
        catalog.bicycle = FieldItemRule {
            item_id: "BICYCLE".to_string(),
        };

        let mut escape_rope = escape_item("ESCAPE_ROPE", Some("DIG_WARP"));
        escape_rope.field_usable = false;
        let mut bicycle = field_effect_item("BICYCLE", "BICYCLE");
        bicycle.field_usable = false;
        let items = BTreeMap::from([
            ("ESCAPE_ROPE".to_string(), escape_rope),
            ("BICYCLE".to_string(), bicycle),
        ]);

        let issues = field_move_catalog_issues(&catalog, &BTreeSet::new(), &items);

        assert!(issues.contains(&FieldMoveCatalogIssue::UnusableEscapeItem {
            item_id: "ESCAPE_ROPE".to_string(),
        }));
        assert!(issues.contains(&FieldMoveCatalogIssue::UnusableFieldItem {
            subject: "field_moves:bicycle".to_string(),
            item_id: "BICYCLE".to_string(),
        }));
    }

    #[test]
    fn field_move_catalog_issues_reject_repel_payloads_that_are_not_field_usable() {
        let mut catalog = FieldMoveCatalog::default();
        catalog.repel = FieldRepelItemRule {};

        let mut repel = repel_item("MOD_REPEL", Some(100));
        repel.field_usable = false;
        let items = BTreeMap::from([("MOD_REPEL".to_string(), repel)]);

        let issues = field_move_catalog_issues(&catalog, &BTreeSet::new(), &items);

        assert!(issues.contains(&FieldMoveCatalogIssue::MissingUsableRepelItemPayload));
    }

    #[test]
    fn field_move_catalog_issues_reject_invalid_escape_rule_without_unknown_fallback() {
        let mut catalog = FieldMoveCatalog::default();
        catalog.escape_rope = FieldEscapeItemRule {
            item_id: "ESCAPE ROPE".to_string(),
            escape_rope_mode: "DIG WARP".to_string(),
        };

        let escape_rope = escape_item("ESCAPE_ROPE", Some("DIG_WARP"));
        let items = BTreeMap::from([("ESCAPE_ROPE".to_string(), escape_rope)]);

        let issues = field_move_catalog_issues(&catalog, &BTreeSet::new(), &items);

        assert!(issues.contains(&FieldMoveCatalogIssue::InvalidEscapeItemId));
        assert!(issues.contains(&FieldMoveCatalogIssue::InvalidEscapeItemMode));
        assert!(
            !issues
                .iter()
                .any(|issue| matches!(issue, FieldMoveCatalogIssue::UnknownEscapeItemRule { .. }))
        );
    }

    #[test]
    fn field_move_tokens_reject_reserved_pack_prefixes() {
        let mut catalog = FieldMoveCatalog::default();
        catalog.fly = FieldMoveRule {
            move_id: "fallback_fly".to_string(),
            badge: badge(BADGE_STORM),
        };
        catalog.strength = FieldMoveFlagRule {
            move_id: "STRENGTH".to_string(),
            badge: badge(BADGE_PLAIN),
            engine_flag: "legacy_strength_flag".to_string(),
        };
        catalog.escape_rope = FieldEscapeItemRule {
            item_id: "fallback_escape_rope".to_string(),
            escape_rope_mode: "legacy_dig_warp".to_string(),
        };

        let issues = field_move_catalog_issues(&catalog, &BTreeSet::new(), &BTreeMap::new());

        assert!(issues.contains(&FieldMoveCatalogIssue::InvalidMoveId {
            subject: "field_moves:fly".to_string(),
        }));
        assert!(issues.contains(&FieldMoveCatalogIssue::InvalidEngineFlag {
            subject: "field_moves:strength".to_string(),
            move_id: "STRENGTH".to_string(),
        }));
        assert!(issues.contains(&FieldMoveCatalogIssue::InvalidEscapeItemId));
        assert!(issues.contains(&FieldMoveCatalogIssue::InvalidEscapeItemMode));

        assert_eq!(
            require_rule_field("fallback_move", "move_id"),
            Err(FieldMoveError::InvalidRuleField {
                field: "move_id".to_string(),
                value: "fallback_move".to_string(),
            })
        );
    }

    #[test]
    fn field_move_error_json_rejects_unknown_fallback_fields() {
        let replacement_error = serde_json::from_value::<FieldMoveError>(serde_json::json!({
            "UnsupportedReplacement": {
                "move_id": "CUT",
                "tileset_name": "JohtoOverworld",
                "block_id": 7,
                "fallback_block_id": 1
            }
        }))
        .expect_err("fallback block id must be rejected")
        .to_string();
        assert!(
            replacement_error.contains("unknown field `fallback_block_id`"),
            "{replacement_error}"
        );

        let item_error = serde_json::from_value::<FieldMoveError>(serde_json::json!({
            "InvalidFieldItemId": {
                "rule_id": "bicycle",
                "item_id": "MOD_BICYCLE",
                "expected_item_id": "BICYCLE",
                "legacy_item_id": "BICYCLE"
            }
        }))
        .expect_err("legacy item id must be rejected")
        .to_string();
        assert!(
            item_error.contains("unknown field `legacy_item_id`"),
            "{item_error}"
        );
    }
}
