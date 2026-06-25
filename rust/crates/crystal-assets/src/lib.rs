use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::{Component, Path, PathBuf};
use std::rc::Rc;

use anyhow::{Context, Result};
use crystal_core::battle::capture::{
    CaptureBallRuleIssue, CaptureWobbleProbability, capture_ball_rule_issues,
};
use crystal_core::battle::damage::{TypeCategories, TypeEffectivenessTable, WeatherModifiers};
use crystal_core::battle::start::{
    StaticWildBattleRequest, StaticWildBattleStart, TrainerBattleRequest, TrainerBattleStartStatus,
    WildBattleStart, static_wild_battle_start, trainer_battle_start,
    wild_battle_start_from_encounter,
};
use crystal_core::battle::stats::BattleStatMultiplierTables;
use crystal_core::battle::turn::MovePriorityTable;
use crystal_core::map::{
    BackgroundEvent, CoordEvent, MapAttributes, MapConnection, MapEventSectionCommand, MapEvents,
    MapScene, MapSceneTable, MapScriptSectionCommand, ObjectEvent, WarpEvent,
    map_event_section_command_arg_counts, map_script_section_command_arg_counts,
};
#[cfg(test)]
use crystal_core::models::FrontpicAnimCommand;
use crystal_core::models::{
    Dv, FrontpicAnimCommandIssue, FrontpicAnimProgram, ITEM_POCKET_BALL, ITEM_POCKET_TM_HM, Item,
    Move, Pokemon, PokemonSpecies, Trainer, TrainerCatalog, create_pokemon_from_known_dvs,
    frontpic_anim_command_issue,
};
use crystal_core::random::Random;
use crystal_core::systems::battle_escape::BattleEscapeRules;
use crystal_core::systems::battle_rewards::BattleRewardRules;
use crystal_core::systems::economy::{
    CurrencyCatalog, MoneyAccount, SCRIPT_COIN_CHECK_COMMANDS, SCRIPT_COIN_MUTATION_COMMANDS,
    SCRIPT_MONEY_CHECK_COMMANDS, SCRIPT_MONEY_MUTATION_COMMANDS, ScriptEconomyCommand,
    is_known_script_economy_command, resolve_amount,
};
use crystal_core::systems::evolution::{
    EvolutionEntry, EvolutionTable, METHOD_HAPPINESS, METHOD_ITEM, METHOD_LEVEL, METHOD_STAT,
    METHOD_TRADE, TRADE_ANY_ITEM, is_known_happiness_window, is_known_stat_evolution_ratio,
};
use crystal_core::systems::field_items::{
    FruitTreeCatalog, SCRIPT_FIELD_FRUIT_TREE_PICKUP_COMMANDS,
    SCRIPT_FIELD_HIDDEN_ITEM_PICKUP_COMMANDS, SCRIPT_FIELD_ITEM_PICKUP_COMMANDS,
    SCRIPT_FIELD_ITEMBALL_PICKUP_COMMANDS, ScriptFieldPickup, ScriptFieldPickupIssue,
    script_field_pickup_issues,
};
use crystal_core::systems::field_moves::{
    FieldItemRule, FieldMoveBlockRule, FieldMoveCatalog, FieldMoveFlagRule, FieldMoveMoveRule,
    FieldMoveRule, FieldMoveTravelRule,
};
use crystal_core::systems::gift_pokemon::{GiftPokemonScript, NO_ITEM};
use crystal_core::systems::learnsets::{LearnsetEntry, SpeciesLearnsets};
use crystal_core::systems::phone::{
    PhoneContactCatalog, SCRIPT_PHONE_CHECK_COMMANDS, SCRIPT_PHONE_REGISTRATION_COMMANDS,
    ScriptPhoneCommand, ScriptPhoneError, validate_script_phone_command,
};
use crystal_core::systems::script_audio::{
    ScriptAudioCommand, ScriptAudioCommandIssue, script_audio_command_issues,
};
use crystal_core::systems::script_blocks::{CHANGE_BLOCK_COORD_STRIDE, ScriptBlockChange};
use crystal_core::systems::script_control::{
    ScriptControlCommand, validate_script_control_command,
};
use crystal_core::systems::script_flags::{
    ScriptFlagCommand, ScriptFlagCommandIssue, is_known_script_flag_command,
    script_flag_command_issues,
};
use crystal_core::systems::script_items::{ScriptItemAccess, ScriptItemGrant};
use crystal_core::systems::script_objects::{
    SCRIPT_MOVEMENT_DIRECTION_COMMANDS, SCRIPT_MOVEMENT_NO_ARG_COMMANDS,
    SCRIPT_MOVEMENT_OPTIONAL_DURATION_COMMANDS, SCRIPT_OBJECT_COORDINATE_COMMANDS,
    SCRIPT_OBJECT_DIRECT_MOVEMENT_COMMANDS, SCRIPT_OBJECT_DIRECTION_COMMANDS,
    SCRIPT_OBJECT_EMOTE_COMMANDS, SCRIPT_OBJECT_LAST_TALKED_MOVEMENT_COMMANDS,
    SCRIPT_OBJECT_MOVEMENT_COMMANDS, SCRIPT_OBJECT_NO_PAYLOAD_COMMANDS,
    SCRIPT_OBJECT_TARGET_COMMANDS, SCRIPT_OBJECT_VISIBILITY_COMMANDS, ScriptMovement,
    ScriptMovementStep, ScriptObjectCommand, is_hideable_object_event_flag,
    is_known_script_movement_command, is_known_script_object_command, parse_script_direction,
};
use crystal_core::systems::script_runtime::{
    ScriptRuntimeCommand, ScriptRuntimeCommandError, script_runtime_command_arg_counts,
    validate_script_runtime_command,
};
use crystal_core::systems::script_scenes::{
    SCRIPT_SCENE_CHECK_COMMANDS, SCRIPT_SCENE_CURRENT_MAP_MUTATION_COMMANDS,
    SCRIPT_SCENE_TARGET_MAP_MUTATION_COMMANDS, ScriptSceneCommand, is_known_script_scene_command,
};
use crystal_core::systems::script_text::{
    SCRIPT_TEXT_LABEL_COMMANDS, SCRIPT_TEXT_NO_LABEL_COMMANDS, ScriptMenuCommand,
    ScriptMenuDefinition, ScriptTextBody, ScriptTextBodyCommand, ScriptTextCommand,
    is_known_script_text_command, menu_definition_command_arg_counts, text_body_command_arg_counts,
};
use crystal_core::systems::script_variables::{
    ScriptVariableCommand, validate_script_variable_command,
};
use crystal_core::systems::script_warps::{
    SCRIPT_MAP_FACING_WARP_COMMANDS, SCRIPT_MAP_NEW_LOAD_COMMANDS, SCRIPT_MAP_NO_PAYLOAD_COMMANDS,
    SCRIPT_MAP_REANCHOR_COMMANDS, SCRIPT_MAP_WARP_COMMANDS, ScriptMapCommand,
    is_known_script_map_command, parse_script_warp_facing,
};
use crystal_core::systems::shop::{
    MartCatalog, SCRIPT_SHOP_COMMANDS, ScriptShopCommand, ShopError, validate_script_shop_command,
};
use crystal_core::systems::special_routines::{
    BUENA_PASSWORD_CATEGORY_ITEM, BUENA_PASSWORD_CATEGORY_MON, BUENA_PASSWORD_CATEGORY_MOVE,
    BattleTowerRules, BuenaPasswordCategoryDefinition, BuenaPrizeDefinition, BugContestConfig,
    DratiniMoveSetDefinition, HappinessData, KurtApricornRecipe, MagikarpLengthEntry,
    OakRatingEntry, OddEggDefinition, RoamingPokemonDefinition, ShuckieGiftDefinition,
    is_known_buena_password_category_type, is_known_special_routine,
};
use crystal_core::systems::step_events::StepEventRules;
use crystal_core::world::collision::{
    MetatileCollision, PlayerTraversalState, TilesetCollision, can_enter_tile,
    is_permission_passable, permissions, sample_collision,
};
use crystal_core::world::encounters::{
    ENCOUNTER_TIME_KEYS, EncounterMusicModifiers, EncounterSlotTables, FieldEncounterData,
    FieldEncounterEntry, WildEncounterData, resolve_encounter_time_key,
};
use crystal_core::world::fishing::{FishingCatalog, is_known_fishing_rod};
use crystal_core::world::map::{Direction, OverworldMapData, TilePosition};
use crystal_core::world::session::{
    ConnectionDestination, ConnectionTransition, ConnectionTrigger, WarpDestination,
    WarpTransition, WarpTrigger, WildEncounterRoll, object_event_initial_facing,
    warp_tile_position,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub use crystal_core::battle::capture::CaptureRules;

pub mod modpack {
    pub use crystal_core::battle::capture::{CaptureRules, CaptureWobbleProbability};
    pub use crystal_core::battle::damage::{
        TypeCategories, TypeEffectivenessEntry, TypeEffectivenessTable, WeatherModifiers,
        WeatherMoveEffectModifier, WeatherTypeModifier,
    };
    pub use crystal_core::battle::stats::{BattleStatMultiplier, BattleStatMultiplierTables};
    pub use crystal_core::battle::turn::{
        MoveEffectPriority, MovePriorityOverride, MovePriorityTable,
    };
    pub use crystal_core::map::{MapEventSectionCommand, MapScriptSectionCommand};
    pub use crystal_core::systems::battle_rewards::BattleRewardRules;
    pub use crystal_core::systems::economy::{CurrencyCatalog, ScriptEconomyCommand};
    pub use crystal_core::systems::evolution::{EvolutionEntry, EvolutionTable};
    pub use crystal_core::systems::experience::{GrowthRateCatalog, GrowthRateCurve};
    pub use crystal_core::systems::field_items::{FruitTreeCatalog, ScriptFieldPickup};
    pub use crystal_core::systems::gift_pokemon::GiftPokemonScript;
    pub use crystal_core::systems::phone::{
        PhoneContactCatalog, PhoneContactRecord, ScriptPhoneCommand,
    };
    pub use crystal_core::systems::script_audio::ScriptAudioCommand;
    pub use crystal_core::systems::script_blocks::ScriptBlockChange;
    pub use crystal_core::systems::script_control::ScriptControlCommand;
    pub use crystal_core::systems::script_flags::ScriptFlagCommand;
    pub use crystal_core::systems::script_items::{ScriptItemAccess, ScriptItemGrant};
    pub use crystal_core::systems::script_objects::{
        ScriptMovement, ScriptMovementStep, ScriptObjectCommand,
    };
    pub use crystal_core::systems::script_runtime::ScriptRuntimeCommand;
    pub use crystal_core::systems::script_scenes::ScriptSceneCommand;
    pub use crystal_core::systems::script_text::{
        ScriptMenuCommand, ScriptMenuDefinition, ScriptTextBody, ScriptTextBodyCommand,
        ScriptTextCommand,
    };
    pub use crystal_core::systems::script_variables::ScriptVariableCommand;
    pub use crystal_core::systems::script_warps::ScriptMapCommand;
    pub use crystal_core::systems::shop::{MartCatalog, ScriptShopCommand};
    pub use crystal_core::world::encounters::{
        EncounterMusicModifier, EncounterMusicModifiers, EncounterSlotChance, EncounterSlotTables,
    };
    pub use crystal_core::world::fishing::FishingCatalog;

    pub use super::{
        COMPILED_GAME_PACK_EXTENSION, COMPILED_GAME_PACK_FORMAT_VERSION, CompiledContentPack,
        CompiledGamePack, CompiledModpack, ContentPack, ContentPackCategory, ContentPackFiles,
        ContentPackIndex, GameDataSet, LoadedCompiledGamePack, MapAccessRule, MapModule,
        ModpackAudioAsset, ModpackAudioKind, ModpackCompileOptions, ModpackCompileReport,
        ModpackCompiler, ModpackManifest, ModpackMetadata, ModpackPayload, PlayabilityGraphEdge,
        PlayabilityRules, PlayabilityStart, ProgressionGrants, ProgressionRequirements,
        ProgressionRule, VerificationError, VerificationSeverity, read_compiled_game_pack,
        read_loaded_compiled_game_pack, write_compiled_game_pack,
    };
    pub use crystal_core::models::{Trainer, TrainerCatalog};
    pub use crystal_core::systems::special_routines::{
        BattleTowerRules, BuenaPasswordCategoryDefinition, BuenaPrizeDefinition, BugContestConfig,
        DratiniMoveSetDefinition, HappinessChangeEntry, HappinessData, HappinessServiceOutcome,
        HappinessServiceTable, KurtApricornRecipe, MagikarpLengthEntry, OakRatingEntry,
        OddEggDefinition, RoamingPokemonDefinition, ShuckieGiftDefinition,
    };
    pub use crystal_core::systems::step_events::StepEventRules;
}

const COMPILED_GAME_PACK_MAGIC: &[u8; 12] = b"CRYSTALPACK\0";
pub const COMPILED_GAME_PACK_EXTENSION: &str = "crystalpack";
pub const COMPILED_GAME_PACK_FORMAT_VERSION: u16 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssetRoot {
    pub repository_root: PathBuf,
}

impl AssetRoot {
    pub fn new(repository_root: impl Into<PathBuf>) -> Self {
        Self {
            repository_root: repository_root.into(),
        }
    }

    pub fn vendor_pokecrystal(&self) -> PathBuf {
        self.repository_root.join("vendor/pokecrystal")
    }

    pub fn typescript_assets(&self) -> PathBuf {
        self.repository_root.join("packages/assets/src")
    }

    pub fn runtime_assets(&self) -> PathBuf {
        self.repository_root.join("apps/web/assets")
    }

    pub fn resolve_vendor(&self, relative_path: impl AsRef<Path>) -> PathBuf {
        self.vendor_pokecrystal().join(relative_path)
    }

    pub fn resolve_data_path(&self, relative_path: impl AsRef<Path>) -> Result<PathBuf> {
        let relative_path = relative_path.as_ref();
        if relative_path.is_absolute() {
            anyhow::bail!(
                "runtime data path '{}' must be relative to assets/data",
                relative_path.display()
            );
        }
        let relative_text = relative_path.to_string_lossy();
        if relative_text.starts_with("assets/data/") {
            anyhow::bail!(
                "runtime data path '{relative_text}' must not include the assets/data prefix"
            );
        }
        if relative_path
            .components()
            .any(|component| matches!(component, Component::ParentDir))
        {
            anyhow::bail!(
                "runtime data path '{}' must not traverse parent directories",
                relative_path.display()
            );
        }
        Ok(self.runtime_assets().join("data").join(relative_path))
    }

    pub fn load_content_pack_index(&self) -> Result<ContentPackIndex> {
        let mut index: ContentPackIndex =
            read_json_file(&self.runtime_assets().join("data/content-packs/index.json"))?;
        index.sort_packs();
        Ok(index)
    }

    pub fn load_modpack_manifest(
        &self,
        relative_path: impl AsRef<Path>,
    ) -> Result<ModpackManifest> {
        read_json_file(&self.repository_root.join(relative_path))
    }

    pub fn load_base_game_data(&self) -> Result<GameDataSet> {
        GameDataSet::load_base_json(self)
    }

    pub fn compile_modpacks(
        &self,
        manifests: &[ModpackManifest],
        options: ModpackCompileOptions,
    ) -> Result<CompiledModpack> {
        ModpackCompiler::new(self).compile(manifests, options)
    }

    pub fn load_compiled_game_pack(
        &self,
        relative_path: impl AsRef<Path>,
    ) -> Result<CompiledGamePack> {
        read_compiled_game_pack(resolve_compiled_game_pack_data_path(
            self,
            relative_path.as_ref(),
        )?)
    }

    pub fn load_loaded_compiled_game_pack(
        &self,
        relative_path: impl AsRef<Path>,
    ) -> Result<LoadedCompiledGamePack> {
        read_loaded_compiled_game_pack(resolve_compiled_game_pack_data_path(
            self,
            relative_path.as_ref(),
        )?)
    }

    pub fn load_compiled_game_data(&self, relative_path: impl AsRef<Path>) -> Result<GameDataSet> {
        Ok(self.load_compiled_game_pack(relative_path)?.data)
    }

    pub fn load_tileset_collision(&self, tileset_name: &str) -> Result<TilesetCollision> {
        let path = self
            .runtime_assets()
            .join("data/tilesets")
            .join(format!("{tileset_name}.json"));
        let raw: BTreeMap<String, Vec<Value>> = read_json_file(&path)?;
        let max_id = raw
            .keys()
            .map(|key| {
                parse_metatile_id(key)
                    .with_context(|| format!("parse metatile id '{key}' in {}", path.display()))
            })
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .max()
            .with_context(|| format!("tileset collision file {} is empty", path.display()))?;
        let mut metatiles = vec![
            MetatileCollision {
                collision: [permissions::FLOOR; 4],
            };
            max_id + 1
        ];
        for (id, quadrants) in raw {
            if quadrants.len() != 4 {
                anyhow::bail!(
                    "tileset {tileset_name} metatile {id} has {} collision quadrants",
                    quadrants.len()
                );
            }
            let index = parse_metatile_id(&id)?;
            let mut collision = [0_u8; 4];
            for (quadrant, value) in quadrants.into_iter().enumerate() {
                collision[quadrant] = match value {
                    Value::Number(number) => number
                        .as_u64()
                        .and_then(|value| u8::try_from(value).ok())
                        .with_context(|| {
                            format!("invalid collision value for {tileset_name}:{id}")
                        })?,
                    Value::String(token) => resolve_collision_token(&token).with_context(|| {
                        format!("unknown collision token {token} in {tileset_name}:{id}")
                    })?,
                    _ => anyhow::bail!("invalid collision entry in {tileset_name}:{id}"),
                };
            }
            metatiles[index] = MetatileCollision { collision };
        }
        Ok(TilesetCollision { metatiles })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentPackCategory {
    Pokemon,
    Moves,
    GrowthRates,
    Learnsets,
    LevelUpMoves,
    EggMoves,
    Evolutions,
    Maps,
    MapBlocks,
    MapAttributes,
    MapDimensions,
    WildEncounters,
    FieldEncounters,
    RuntimeSpawnPoints,
    RuntimeMapMetadata,
    FleeMons,
    RoamingPokemon,
    BuenaPasswordCategories,
    BuenaPrizes,
    KurtApricornRecipes,
    ShuckieGift,
    DratiniMoveSets,
    BugContestConfig,
    BattleTowerRules,
    OakRatings,
    OddEggDefinitions,
    MagikarpLengths,
    HappinessData,
    EncounterSlotTables,
    EncounterMusicModifiers,
    BattleStatMultipliers,
    CaptureWobbleProbabilities,
    CaptureRules,
    BattleEscapeRules,
    MovePriorities,
    TypeCategories,
    TypeEffectiveness,
    WeatherModifiers,
    BattleRewardRules,
    StepEventRules,
    Fishing,
    FruitTrees,
    FieldMoves,
    Npcs,
    PokegearLandmarks,
    PcStrings,
    MenuIcons,
    Items,
    Marts,
    CurrencyConstants,
    Trainers,
    Pokedex,
    PokedexEntries,
    PokemonFrontpicAnim,
    InitializeEvents,
    StoryEventScriptConstants,
    StoryEvents,
    PhoneScripts,
    PhoneContacts,
    PermanentPhoneNumbers,
    SpecialPhoneCalls,
    NpcTrades,
    SpecialRoutines,
    AsmText,
    MoveNames,
    BattleAnimations,
    BattleAnimationTable,
    BattleAnimBundle,
    SpriteAnimBundle,
    SpritePaletteDefaults,
    PokegearTownMapPaletteMap,
    PokemonCries,
    Audio,
    Tilesets,
    Playability,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContentPackFiles {
    pub pokemon: Vec<String>,
    pub moves: Vec<String>,
    pub growth_rates: Vec<String>,
    pub learnsets: Vec<String>,
    pub level_up_moves: Vec<String>,
    pub egg_moves: Vec<String>,
    pub evolutions: Vec<String>,
    pub maps: Vec<String>,
    pub map_blocks: Vec<String>,
    pub map_attributes: Vec<String>,
    pub map_dimensions: Vec<String>,
    pub wild_encounters: Vec<String>,
    pub field_encounters: Vec<String>,
    pub runtime_spawn_points: Vec<String>,
    pub runtime_map_metadata: Vec<String>,
    pub flee_mons: Vec<String>,
    pub roaming_pokemon: Vec<String>,
    pub buena_password_categories: Vec<String>,
    pub buena_prizes: Vec<String>,
    pub kurt_apricorn_recipes: Vec<String>,
    pub shuckie_gift: Vec<String>,
    pub dratini_move_sets: Vec<String>,
    pub bug_contest_config: Vec<String>,
    pub battle_tower_rules: Vec<String>,
    pub oak_ratings: Vec<String>,
    pub odd_egg_definitions: Vec<String>,
    pub magikarp_lengths: Vec<String>,
    pub happiness_data: Vec<String>,
    pub encounter_slot_tables: Vec<String>,
    pub encounter_music_modifiers: Vec<String>,
    pub battle_stat_multipliers: Vec<String>,
    pub capture_wobble_probabilities: Vec<String>,
    pub capture_rules: Vec<String>,
    pub battle_escape_rules: Vec<String>,
    pub move_priorities: Vec<String>,
    pub type_categories: Vec<String>,
    pub type_effectiveness: Vec<String>,
    pub weather_modifiers: Vec<String>,
    pub battle_reward_rules: Vec<String>,
    pub step_event_rules: Vec<String>,
    pub fishing: Vec<String>,
    pub fruit_trees: Vec<String>,
    pub field_moves: Vec<String>,
    pub npcs: Vec<String>,
    pub pokegear_landmarks: Vec<String>,
    pub pc_strings: Vec<String>,
    pub menu_icons: Vec<String>,
    pub items: Vec<String>,
    pub marts: Vec<String>,
    pub currency_constants: Vec<String>,
    pub trainers: Vec<String>,
    pub pokedex: Vec<String>,
    pub pokedex_entries: Vec<String>,
    pub pokemon_frontpic_anim: Vec<String>,
    pub initialize_events: Vec<String>,
    pub story_event_script_constants: Vec<String>,
    pub story_events: Vec<String>,
    pub phone_scripts: Vec<String>,
    pub phone_contacts: Vec<String>,
    pub permanent_phone_numbers: Vec<String>,
    pub special_phone_calls: Vec<String>,
    pub npc_trades: Vec<String>,
    pub special_routines: Vec<String>,
    pub asm_text: Vec<String>,
    pub move_names: Vec<String>,
    pub battle_animations: Vec<String>,
    pub battle_animation_table: Vec<String>,
    pub battle_anim_bundle: Vec<String>,
    pub sprite_anim_bundle: Vec<String>,
    pub sprite_palette_defaults: Vec<String>,
    pub pokegear_town_map_palette_map: Vec<String>,
    pub pokemon_cries: Vec<String>,
    pub audio: Vec<String>,
    pub tilesets: Vec<String>,
    pub playability: Vec<String>,
}

impl ContentPackFiles {
    pub fn entries(&self, category: ContentPackCategory) -> &[String] {
        match category {
            ContentPackCategory::Pokemon => &self.pokemon,
            ContentPackCategory::Moves => &self.moves,
            ContentPackCategory::GrowthRates => &self.growth_rates,
            ContentPackCategory::Learnsets => &self.learnsets,
            ContentPackCategory::LevelUpMoves => &self.level_up_moves,
            ContentPackCategory::EggMoves => &self.egg_moves,
            ContentPackCategory::Evolutions => &self.evolutions,
            ContentPackCategory::Maps => &self.maps,
            ContentPackCategory::MapBlocks => &self.map_blocks,
            ContentPackCategory::MapAttributes => &self.map_attributes,
            ContentPackCategory::MapDimensions => &self.map_dimensions,
            ContentPackCategory::WildEncounters => &self.wild_encounters,
            ContentPackCategory::FieldEncounters => &self.field_encounters,
            ContentPackCategory::RuntimeSpawnPoints => &self.runtime_spawn_points,
            ContentPackCategory::RuntimeMapMetadata => &self.runtime_map_metadata,
            ContentPackCategory::FleeMons => &self.flee_mons,
            ContentPackCategory::RoamingPokemon => &self.roaming_pokemon,
            ContentPackCategory::BuenaPasswordCategories => &self.buena_password_categories,
            ContentPackCategory::BuenaPrizes => &self.buena_prizes,
            ContentPackCategory::KurtApricornRecipes => &self.kurt_apricorn_recipes,
            ContentPackCategory::ShuckieGift => &self.shuckie_gift,
            ContentPackCategory::DratiniMoveSets => &self.dratini_move_sets,
            ContentPackCategory::BugContestConfig => &self.bug_contest_config,
            ContentPackCategory::BattleTowerRules => &self.battle_tower_rules,
            ContentPackCategory::OakRatings => &self.oak_ratings,
            ContentPackCategory::OddEggDefinitions => &self.odd_egg_definitions,
            ContentPackCategory::MagikarpLengths => &self.magikarp_lengths,
            ContentPackCategory::HappinessData => &self.happiness_data,
            ContentPackCategory::EncounterSlotTables => &self.encounter_slot_tables,
            ContentPackCategory::EncounterMusicModifiers => &self.encounter_music_modifiers,
            ContentPackCategory::BattleStatMultipliers => &self.battle_stat_multipliers,
            ContentPackCategory::CaptureWobbleProbabilities => &self.capture_wobble_probabilities,
            ContentPackCategory::CaptureRules => &self.capture_rules,
            ContentPackCategory::BattleEscapeRules => &self.battle_escape_rules,
            ContentPackCategory::MovePriorities => &self.move_priorities,
            ContentPackCategory::TypeCategories => &self.type_categories,
            ContentPackCategory::TypeEffectiveness => &self.type_effectiveness,
            ContentPackCategory::WeatherModifiers => &self.weather_modifiers,
            ContentPackCategory::BattleRewardRules => &self.battle_reward_rules,
            ContentPackCategory::StepEventRules => &self.step_event_rules,
            ContentPackCategory::Fishing => &self.fishing,
            ContentPackCategory::FruitTrees => &self.fruit_trees,
            ContentPackCategory::FieldMoves => &self.field_moves,
            ContentPackCategory::Npcs => &self.npcs,
            ContentPackCategory::PokegearLandmarks => &self.pokegear_landmarks,
            ContentPackCategory::PcStrings => &self.pc_strings,
            ContentPackCategory::MenuIcons => &self.menu_icons,
            ContentPackCategory::Items => &self.items,
            ContentPackCategory::Marts => &self.marts,
            ContentPackCategory::CurrencyConstants => &self.currency_constants,
            ContentPackCategory::Trainers => &self.trainers,
            ContentPackCategory::Pokedex => &self.pokedex,
            ContentPackCategory::PokedexEntries => &self.pokedex_entries,
            ContentPackCategory::PokemonFrontpicAnim => &self.pokemon_frontpic_anim,
            ContentPackCategory::InitializeEvents => &self.initialize_events,
            ContentPackCategory::StoryEventScriptConstants => &self.story_event_script_constants,
            ContentPackCategory::StoryEvents => &self.story_events,
            ContentPackCategory::PhoneScripts => &self.phone_scripts,
            ContentPackCategory::PhoneContacts => &self.phone_contacts,
            ContentPackCategory::PermanentPhoneNumbers => &self.permanent_phone_numbers,
            ContentPackCategory::SpecialPhoneCalls => &self.special_phone_calls,
            ContentPackCategory::NpcTrades => &self.npc_trades,
            ContentPackCategory::SpecialRoutines => &self.special_routines,
            ContentPackCategory::AsmText => &self.asm_text,
            ContentPackCategory::MoveNames => &self.move_names,
            ContentPackCategory::BattleAnimations => &self.battle_animations,
            ContentPackCategory::BattleAnimationTable => &self.battle_animation_table,
            ContentPackCategory::BattleAnimBundle => &self.battle_anim_bundle,
            ContentPackCategory::SpriteAnimBundle => &self.sprite_anim_bundle,
            ContentPackCategory::SpritePaletteDefaults => &self.sprite_palette_defaults,
            ContentPackCategory::PokegearTownMapPaletteMap => &self.pokegear_town_map_palette_map,
            ContentPackCategory::PokemonCries => &self.pokemon_cries,
            ContentPackCategory::Audio => &self.audio,
            ContentPackCategory::Tilesets => &self.tilesets,
            ContentPackCategory::Playability => &self.playability,
        }
    }
}

const CONTENT_PACK_CATEGORIES: &[ContentPackCategory] = &[
    ContentPackCategory::Pokemon,
    ContentPackCategory::Moves,
    ContentPackCategory::GrowthRates,
    ContentPackCategory::Learnsets,
    ContentPackCategory::LevelUpMoves,
    ContentPackCategory::EggMoves,
    ContentPackCategory::Evolutions,
    ContentPackCategory::Maps,
    ContentPackCategory::MapBlocks,
    ContentPackCategory::MapAttributes,
    ContentPackCategory::MapDimensions,
    ContentPackCategory::WildEncounters,
    ContentPackCategory::FieldEncounters,
    ContentPackCategory::RuntimeSpawnPoints,
    ContentPackCategory::RuntimeMapMetadata,
    ContentPackCategory::FleeMons,
    ContentPackCategory::RoamingPokemon,
    ContentPackCategory::BuenaPasswordCategories,
    ContentPackCategory::BuenaPrizes,
    ContentPackCategory::KurtApricornRecipes,
    ContentPackCategory::ShuckieGift,
    ContentPackCategory::DratiniMoveSets,
    ContentPackCategory::BugContestConfig,
    ContentPackCategory::BattleTowerRules,
    ContentPackCategory::OakRatings,
    ContentPackCategory::OddEggDefinitions,
    ContentPackCategory::MagikarpLengths,
    ContentPackCategory::HappinessData,
    ContentPackCategory::EncounterSlotTables,
    ContentPackCategory::EncounterMusicModifiers,
    ContentPackCategory::BattleStatMultipliers,
    ContentPackCategory::CaptureWobbleProbabilities,
    ContentPackCategory::CaptureRules,
    ContentPackCategory::BattleEscapeRules,
    ContentPackCategory::MovePriorities,
    ContentPackCategory::TypeCategories,
    ContentPackCategory::TypeEffectiveness,
    ContentPackCategory::WeatherModifiers,
    ContentPackCategory::BattleRewardRules,
    ContentPackCategory::StepEventRules,
    ContentPackCategory::Fishing,
    ContentPackCategory::FruitTrees,
    ContentPackCategory::FieldMoves,
    ContentPackCategory::Npcs,
    ContentPackCategory::PokegearLandmarks,
    ContentPackCategory::PcStrings,
    ContentPackCategory::MenuIcons,
    ContentPackCategory::Items,
    ContentPackCategory::Marts,
    ContentPackCategory::CurrencyConstants,
    ContentPackCategory::Trainers,
    ContentPackCategory::Pokedex,
    ContentPackCategory::PokedexEntries,
    ContentPackCategory::PokemonFrontpicAnim,
    ContentPackCategory::InitializeEvents,
    ContentPackCategory::StoryEventScriptConstants,
    ContentPackCategory::StoryEvents,
    ContentPackCategory::PhoneScripts,
    ContentPackCategory::PhoneContacts,
    ContentPackCategory::PermanentPhoneNumbers,
    ContentPackCategory::SpecialPhoneCalls,
    ContentPackCategory::NpcTrades,
    ContentPackCategory::SpecialRoutines,
    ContentPackCategory::AsmText,
    ContentPackCategory::MoveNames,
    ContentPackCategory::BattleAnimations,
    ContentPackCategory::BattleAnimationTable,
    ContentPackCategory::BattleAnimBundle,
    ContentPackCategory::SpriteAnimBundle,
    ContentPackCategory::SpritePaletteDefaults,
    ContentPackCategory::PokegearTownMapPaletteMap,
    ContentPackCategory::PokemonCries,
    ContentPackCategory::Audio,
    ContentPackCategory::Tilesets,
    ContentPackCategory::Playability,
];

impl ContentPackCategory {
    pub fn as_str(self) -> &'static str {
        match self {
            ContentPackCategory::Pokemon => "pokemon",
            ContentPackCategory::Moves => "moves",
            ContentPackCategory::GrowthRates => "growth_rates",
            ContentPackCategory::Learnsets => "learnsets",
            ContentPackCategory::LevelUpMoves => "level_up_moves",
            ContentPackCategory::EggMoves => "egg_moves",
            ContentPackCategory::Evolutions => "evolutions",
            ContentPackCategory::Maps => "maps",
            ContentPackCategory::MapBlocks => "map_blocks",
            ContentPackCategory::MapAttributes => "map_attributes",
            ContentPackCategory::MapDimensions => "map_dimensions",
            ContentPackCategory::WildEncounters => "wild_encounters",
            ContentPackCategory::FieldEncounters => "field_encounters",
            ContentPackCategory::RuntimeSpawnPoints => "runtime_spawn_points",
            ContentPackCategory::RuntimeMapMetadata => "runtime_map_metadata",
            ContentPackCategory::FleeMons => "flee_mons",
            ContentPackCategory::RoamingPokemon => "roaming_pokemon",
            ContentPackCategory::BuenaPasswordCategories => "buena_password_categories",
            ContentPackCategory::BuenaPrizes => "buena_prizes",
            ContentPackCategory::KurtApricornRecipes => "kurt_apricorn_recipes",
            ContentPackCategory::ShuckieGift => "shuckie_gift",
            ContentPackCategory::DratiniMoveSets => "dratini_move_sets",
            ContentPackCategory::BugContestConfig => "bug_contest_config",
            ContentPackCategory::BattleTowerRules => "battle_tower_rules",
            ContentPackCategory::OakRatings => "oak_ratings",
            ContentPackCategory::OddEggDefinitions => "odd_egg_definitions",
            ContentPackCategory::MagikarpLengths => "magikarp_lengths",
            ContentPackCategory::HappinessData => "happiness_data",
            ContentPackCategory::EncounterSlotTables => "encounter_slot_tables",
            ContentPackCategory::EncounterMusicModifiers => "encounter_music_modifiers",
            ContentPackCategory::BattleStatMultipliers => "battle_stat_multipliers",
            ContentPackCategory::CaptureWobbleProbabilities => "capture_wobble_probabilities",
            ContentPackCategory::CaptureRules => "capture_rules",
            ContentPackCategory::BattleEscapeRules => "battle_escape_rules",
            ContentPackCategory::MovePriorities => "move_priorities",
            ContentPackCategory::TypeCategories => "type_categories",
            ContentPackCategory::TypeEffectiveness => "type_effectiveness",
            ContentPackCategory::WeatherModifiers => "weather_modifiers",
            ContentPackCategory::BattleRewardRules => "battle_reward_rules",
            ContentPackCategory::StepEventRules => "step_event_rules",
            ContentPackCategory::Fishing => "fishing",
            ContentPackCategory::FruitTrees => "fruit_trees",
            ContentPackCategory::FieldMoves => "field_moves",
            ContentPackCategory::Npcs => "npcs",
            ContentPackCategory::PokegearLandmarks => "pokegear_landmarks",
            ContentPackCategory::PcStrings => "pc_strings",
            ContentPackCategory::MenuIcons => "menu_icons",
            ContentPackCategory::Items => "items",
            ContentPackCategory::Marts => "marts",
            ContentPackCategory::CurrencyConstants => "currency_constants",
            ContentPackCategory::Trainers => "trainers",
            ContentPackCategory::Pokedex => "pokedex",
            ContentPackCategory::PokedexEntries => "pokedex_entries",
            ContentPackCategory::PokemonFrontpicAnim => "pokemon_frontpic_anim",
            ContentPackCategory::InitializeEvents => "initialize_events",
            ContentPackCategory::StoryEventScriptConstants => "story_event_script_constants",
            ContentPackCategory::StoryEvents => "story_events",
            ContentPackCategory::PhoneScripts => "phone_scripts",
            ContentPackCategory::PhoneContacts => "phone_contacts",
            ContentPackCategory::PermanentPhoneNumbers => "permanent_phone_numbers",
            ContentPackCategory::SpecialPhoneCalls => "special_phone_calls",
            ContentPackCategory::NpcTrades => "npc_trades",
            ContentPackCategory::SpecialRoutines => "special_routines",
            ContentPackCategory::AsmText => "asm_text",
            ContentPackCategory::MoveNames => "move_names",
            ContentPackCategory::BattleAnimations => "battle_animations",
            ContentPackCategory::BattleAnimationTable => "battle_animation_table",
            ContentPackCategory::BattleAnimBundle => "battle_anim_bundle",
            ContentPackCategory::SpriteAnimBundle => "sprite_anim_bundle",
            ContentPackCategory::SpritePaletteDefaults => "sprite_palette_defaults",
            ContentPackCategory::PokegearTownMapPaletteMap => "pokegear_town_map_palette_map",
            ContentPackCategory::PokemonCries => "pokemon_cries",
            ContentPackCategory::Audio => "audio",
            ContentPackCategory::Tilesets => "tilesets",
            ContentPackCategory::Playability => "playability",
        }
    }
}

fn required_nullable_string<'de, D>(
    deserializer: D,
) -> std::result::Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::<String>::deserialize(deserializer)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContentPack {
    pub id: String,
    pub enabled: bool,
    pub priority: i32,
    pub path: String,
    #[serde(deserialize_with = "required_nullable_string")]
    pub compiled: Option<String>,
    pub files: ContentPackFiles,
}

impl Default for ContentPack {
    fn default() -> Self {
        Self {
            id: String::new(),
            enabled: true,
            priority: 0,
            path: String::new(),
            compiled: None,
            files: ContentPackFiles::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContentPackIndex {
    pub version: u16,
    pub packs: Vec<ContentPack>,
}

impl Default for ContentPackIndex {
    fn default() -> Self {
        Self {
            version: 1,
            packs: Vec::new(),
        }
    }
}

impl ContentPackIndex {
    pub fn sort_packs(&mut self) {
        self.packs
            .sort_by(|a, b| a.priority.cmp(&b.priority).then_with(|| a.id.cmp(&b.id)));
    }

    pub fn enabled_packs_sorted(&self) -> Vec<&ContentPack> {
        let mut packs: Vec<&ContentPack> = self.packs.iter().filter(|pack| pack.enabled).collect();
        packs.sort_by(|a, b| a.priority.cmp(&b.priority).then_with(|| a.id.cmp(&b.id)));
        packs
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompiledContentPack {
    pub version: u16,
    #[serde(rename = "packId")]
    pub pack_id: String,
    pub categories: CompiledContentPackCategories,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompiledContentPackCategories {
    pub pokemon: Vec<Value>,
    pub moves: Vec<Value>,
    pub growth_rates: Vec<Value>,
    pub learnsets: Vec<Value>,
    pub level_up_moves: Vec<Value>,
    pub egg_moves: Vec<Value>,
    pub evolutions: Vec<Value>,
    pub maps: Vec<Value>,
    pub map_blocks: Vec<Value>,
    pub map_attributes: Vec<Value>,
    pub map_dimensions: Vec<Value>,
    pub wild_encounters: Vec<Value>,
    pub field_encounters: Vec<Value>,
    pub runtime_spawn_points: Vec<Value>,
    pub runtime_map_metadata: Vec<Value>,
    pub flee_mons: Vec<Value>,
    pub roaming_pokemon: Vec<Value>,
    pub buena_password_categories: Vec<Value>,
    pub buena_prizes: Vec<Value>,
    pub kurt_apricorn_recipes: Vec<Value>,
    pub shuckie_gift: Vec<Value>,
    pub dratini_move_sets: Vec<Value>,
    pub bug_contest_config: Vec<Value>,
    pub battle_tower_rules: Vec<Value>,
    pub oak_ratings: Vec<Value>,
    pub odd_egg_definitions: Vec<Value>,
    pub magikarp_lengths: Vec<Value>,
    pub happiness_data: Vec<Value>,
    pub encounter_slot_tables: Vec<Value>,
    pub encounter_music_modifiers: Vec<Value>,
    pub battle_stat_multipliers: Vec<Value>,
    pub capture_wobble_probabilities: Vec<Value>,
    pub capture_rules: Vec<Value>,
    pub battle_escape_rules: Vec<Value>,
    pub move_priorities: Vec<Value>,
    pub type_categories: Vec<Value>,
    pub type_effectiveness: Vec<Value>,
    pub weather_modifiers: Vec<Value>,
    pub battle_reward_rules: Vec<Value>,
    pub step_event_rules: Vec<Value>,
    pub fishing: Vec<Value>,
    pub fruit_trees: Vec<Value>,
    pub field_moves: Vec<Value>,
    pub npcs: Vec<Value>,
    pub pokegear_landmarks: Vec<Value>,
    pub pc_strings: Vec<Value>,
    pub menu_icons: Vec<Value>,
    pub items: Vec<Value>,
    pub marts: Vec<Value>,
    pub currency_constants: Vec<Value>,
    pub trainers: Vec<Value>,
    pub pokedex: Vec<Value>,
    pub pokedex_entries: Vec<Value>,
    pub pokemon_frontpic_anim: Vec<Value>,
    pub initialize_events: Vec<Value>,
    pub story_event_script_constants: Vec<Value>,
    pub story_events: Vec<Value>,
    pub phone_scripts: Vec<Value>,
    pub phone_contacts: Vec<Value>,
    pub permanent_phone_numbers: Vec<Value>,
    pub special_phone_calls: Vec<Value>,
    pub npc_trades: Vec<Value>,
    pub special_routines: Vec<Value>,
    pub asm_text: Vec<Value>,
    pub move_names: Vec<Value>,
    pub battle_animations: Vec<Value>,
    pub battle_animation_table: Vec<Value>,
    pub battle_anim_bundle: Vec<Value>,
    pub sprite_anim_bundle: Vec<Value>,
    pub sprite_palette_defaults: Vec<Value>,
    pub pokegear_town_map_palette_map: Vec<Value>,
    pub pokemon_cries: Vec<Value>,
    pub audio: Vec<Value>,
    pub tilesets: Vec<Value>,
    pub playability: Vec<Value>,
}

impl CompiledContentPackCategories {
    pub fn entries(&self, category: ContentPackCategory) -> &[Value] {
        match category {
            ContentPackCategory::Pokemon => &self.pokemon,
            ContentPackCategory::Moves => &self.moves,
            ContentPackCategory::GrowthRates => &self.growth_rates,
            ContentPackCategory::Learnsets => &self.learnsets,
            ContentPackCategory::LevelUpMoves => &self.level_up_moves,
            ContentPackCategory::EggMoves => &self.egg_moves,
            ContentPackCategory::Evolutions => &self.evolutions,
            ContentPackCategory::Maps => &self.maps,
            ContentPackCategory::MapBlocks => &self.map_blocks,
            ContentPackCategory::MapAttributes => &self.map_attributes,
            ContentPackCategory::MapDimensions => &self.map_dimensions,
            ContentPackCategory::WildEncounters => &self.wild_encounters,
            ContentPackCategory::FieldEncounters => &self.field_encounters,
            ContentPackCategory::RuntimeSpawnPoints => &self.runtime_spawn_points,
            ContentPackCategory::RuntimeMapMetadata => &self.runtime_map_metadata,
            ContentPackCategory::FleeMons => &self.flee_mons,
            ContentPackCategory::RoamingPokemon => &self.roaming_pokemon,
            ContentPackCategory::BuenaPasswordCategories => &self.buena_password_categories,
            ContentPackCategory::BuenaPrizes => &self.buena_prizes,
            ContentPackCategory::KurtApricornRecipes => &self.kurt_apricorn_recipes,
            ContentPackCategory::ShuckieGift => &self.shuckie_gift,
            ContentPackCategory::DratiniMoveSets => &self.dratini_move_sets,
            ContentPackCategory::BugContestConfig => &self.bug_contest_config,
            ContentPackCategory::BattleTowerRules => &self.battle_tower_rules,
            ContentPackCategory::OakRatings => &self.oak_ratings,
            ContentPackCategory::OddEggDefinitions => &self.odd_egg_definitions,
            ContentPackCategory::MagikarpLengths => &self.magikarp_lengths,
            ContentPackCategory::HappinessData => &self.happiness_data,
            ContentPackCategory::EncounterSlotTables => &self.encounter_slot_tables,
            ContentPackCategory::EncounterMusicModifiers => &self.encounter_music_modifiers,
            ContentPackCategory::BattleStatMultipliers => &self.battle_stat_multipliers,
            ContentPackCategory::CaptureWobbleProbabilities => &self.capture_wobble_probabilities,
            ContentPackCategory::CaptureRules => &self.capture_rules,
            ContentPackCategory::BattleEscapeRules => &self.battle_escape_rules,
            ContentPackCategory::MovePriorities => &self.move_priorities,
            ContentPackCategory::TypeCategories => &self.type_categories,
            ContentPackCategory::TypeEffectiveness => &self.type_effectiveness,
            ContentPackCategory::WeatherModifiers => &self.weather_modifiers,
            ContentPackCategory::BattleRewardRules => &self.battle_reward_rules,
            ContentPackCategory::StepEventRules => &self.step_event_rules,
            ContentPackCategory::Fishing => &self.fishing,
            ContentPackCategory::FruitTrees => &self.fruit_trees,
            ContentPackCategory::FieldMoves => &self.field_moves,
            ContentPackCategory::Npcs => &self.npcs,
            ContentPackCategory::PokegearLandmarks => &self.pokegear_landmarks,
            ContentPackCategory::PcStrings => &self.pc_strings,
            ContentPackCategory::MenuIcons => &self.menu_icons,
            ContentPackCategory::Items => &self.items,
            ContentPackCategory::Marts => &self.marts,
            ContentPackCategory::CurrencyConstants => &self.currency_constants,
            ContentPackCategory::Trainers => &self.trainers,
            ContentPackCategory::Pokedex => &self.pokedex,
            ContentPackCategory::PokedexEntries => &self.pokedex_entries,
            ContentPackCategory::PokemonFrontpicAnim => &self.pokemon_frontpic_anim,
            ContentPackCategory::InitializeEvents => &self.initialize_events,
            ContentPackCategory::StoryEventScriptConstants => &self.story_event_script_constants,
            ContentPackCategory::StoryEvents => &self.story_events,
            ContentPackCategory::PhoneScripts => &self.phone_scripts,
            ContentPackCategory::PhoneContacts => &self.phone_contacts,
            ContentPackCategory::PermanentPhoneNumbers => &self.permanent_phone_numbers,
            ContentPackCategory::SpecialPhoneCalls => &self.special_phone_calls,
            ContentPackCategory::NpcTrades => &self.npc_trades,
            ContentPackCategory::SpecialRoutines => &self.special_routines,
            ContentPackCategory::AsmText => &self.asm_text,
            ContentPackCategory::MoveNames => &self.move_names,
            ContentPackCategory::BattleAnimations => &self.battle_animations,
            ContentPackCategory::BattleAnimationTable => &self.battle_animation_table,
            ContentPackCategory::BattleAnimBundle => &self.battle_anim_bundle,
            ContentPackCategory::SpriteAnimBundle => &self.sprite_anim_bundle,
            ContentPackCategory::SpritePaletteDefaults => &self.sprite_palette_defaults,
            ContentPackCategory::PokegearTownMapPaletteMap => &self.pokegear_town_map_palette_map,
            ContentPackCategory::PokemonCries => &self.pokemon_cries,
            ContentPackCategory::Audio => &self.audio,
            ContentPackCategory::Tilesets => &self.tilesets,
            ContentPackCategory::Playability => &self.playability,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModpackManifest {
    pub schema_version: u16,
    pub metadata: ModpackMetadata,
    pub priority: i32,
    pub dependencies: Vec<String>,
    pub payload: ModpackPayload,
}

impl Default for ModpackManifest {
    fn default() -> Self {
        Self {
            schema_version: 1,
            metadata: ModpackMetadata::default(),
            priority: 0,
            dependencies: Vec::new(),
            payload: ModpackPayload::default(),
        }
    }
}

impl ModpackManifest {
    pub fn id(&self) -> &str {
        &self.metadata.id
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModpackMetadata {
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(deserialize_with = "required_nullable_string")]
    pub author: Option<String>,
    #[serde(deserialize_with = "required_nullable_string")]
    pub description: Option<String>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModpackPayload {
    pub pokemon: Vec<PokemonSpecies>,
    pub maps: Vec<MapModule>,
    pub items: Vec<Item>,
    pub moves: Vec<Move>,
    pub evolutions: EvolutionTable,
    pub marts: MartCatalog,
    pub currency_constants: CurrencyCatalog,
    pub battle_reward_rules: BattleRewardRules,
    pub battle_escape_rules: BattleEscapeRules,
    pub step_event_rules: StepEventRules,
    pub fishing: FishingCatalog,
    pub fruit_trees: FruitTreeCatalog,
    pub field_moves: FieldMoveCatalog,
    pub runtime_spawn_points: BTreeMap<String, RuntimeSpawnPoint>,
    pub runtime_map_metadata: BTreeMap<String, RuntimeMapMetadata>,
    pub flee_mons: FleeMonTables,
    pub buena_password_categories: Vec<BuenaPasswordCategoryDefinition>,
    pub roaming_pokemon: Vec<RoamingPokemonDefinition>,
    pub buena_prizes: Vec<BuenaPrizeDefinition>,
    pub kurt_apricorn_recipes: Vec<KurtApricornRecipe>,
    pub shuckie_gift: Option<ShuckieGiftDefinition>,
    pub dratini_move_sets: Vec<DratiniMoveSetDefinition>,
    pub bug_contest_config: Option<BugContestConfig>,
    pub battle_tower_rules: Option<BattleTowerRules>,
    pub oak_ratings: Vec<OakRatingEntry>,
    pub odd_egg_definitions: Vec<OddEggDefinition>,
    pub magikarp_lengths: Vec<MagikarpLengthEntry>,
    pub happiness_data: Option<HappinessData>,
    pub encounter_slot_tables: EncounterSlotTables,
    pub encounter_music_modifiers: EncounterMusicModifiers,
    pub battle_stat_multipliers: BattleStatMultiplierTables,
    pub capture_wobble_probabilities: Vec<CaptureWobbleProbability>,
    pub move_priorities: MovePriorityTable,
    pub type_categories: TypeCategories,
    pub type_effectiveness: TypeEffectivenessTable,
    pub weather_modifiers: WeatherModifiers,
    pub pc_strings: BTreeMap<String, String>,
    pub menu_icons: BTreeMap<String, String>,
    pub pokedex_entries: Vec<RuntimePokedexEntry>,
    pub pokemon_frontpic_anim: BTreeMap<String, FrontpicAnimProgram>,
    pub initialize_events: InitializeEventsConfig,
    pub story_event_script_constants: StoryEventScriptConstants,
    pub asm_text: BTreeMap<String, String>,
    pub move_names: Vec<String>,
    pub battle_animations: BTreeMap<String, Vec<String>>,
    pub battle_animation_table: Vec<String>,
    pub battle_anim_bundle: String,
    pub sprite_anim_bundle: String,
    pub sprite_palette_defaults: BTreeMap<String, i64>,
    pub pokegear_town_map_palette_map: BTreeMap<String, Vec<String>>,
    pub pokegear_landmarks: PokegearLandmarksPayload,
    pub pokemon_cries: BTreeMap<String, PokemonCryMetadata>,
    pub wild_encounters: Vec<WildEncounterData>,
    pub field_encounters: Vec<FieldEncounterData>,
    pub trainers: TrainerCatalog,
    pub phone_contacts: PhoneContactCatalog,
    pub permanent_phone_numbers: Vec<String>,
    pub special_phone_calls: BTreeSet<String>,
    pub npc_trades: BTreeSet<String>,
    pub special_routines: BTreeSet<String>,
    pub audio: Vec<ModpackAudioAsset>,
    pub capture_rules: CaptureRules,
    pub tilesets: Vec<Value>,
    pub playability: PlayabilityRules,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FleeMonTables {
    pub always: Vec<String>,
    pub often: Vec<String>,
    pub sometimes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeSpawnPoint {
    pub identifier: u16,
    pub map_constant: String,
    pub map_name: String,
    pub group_id: i16,
    pub map_id: i16,
    pub tile_x: i16,
    pub tile_y: i16,
    pub group_name: String,
    pub metatile_x: i16,
    pub metatile_y: i16,
    pub subtile_x: i16,
    pub subtile_y: i16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeMapMetadata {
    pub constant: String,
    pub name: String,
    pub group_name: String,
    pub group_id: u16,
    pub map_id: u16,
    pub width: u16,
    pub height: u16,
    pub environment: String,
    pub phone_service: u8,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PokegearLandmarksPayload {
    pub landmarks: Vec<PokegearLandmark>,
    pub map_to_landmark: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PokegearLandmark {
    pub id: u16,
    pub constant: String,
    pub label: String,
    pub name: String,
    pub x: i16,
    pub y: i16,
    pub region: String,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InitializeEventsConfig {
    pub event_flags: Vec<String>,
    pub engine_flags: Vec<String>,
    pub variable_sprites: BTreeMap<String, String>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StoryEventScriptConstants {
    pub global: BTreeMap<String, i64>,
    pub maps: BTreeMap<String, BTreeMap<String, i64>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PokemonCryMetadata {
    pub cry: String,
    pub pitch: i16,
    pub length: i16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimePokedexEntry {
    pub species: String,
    pub classification: String,
    pub height_digits: u16,
    pub weight_digits: u16,
    pub pages: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModpackAudioKind {
    Music,
    SoundEffect,
    Cry,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModpackAudioAsset {
    pub id: String,
    pub path: String,
    pub kind: ModpackAudioKind,
}

impl ModpackAudioAsset {
    pub fn music(id: impl Into<String>, path: impl Into<String>) -> Result<Self> {
        let asset = Self {
            id: id.into(),
            path: path.into(),
            kind: ModpackAudioKind::Music,
        };
        asset.validate()?;
        Ok(asset)
    }

    pub fn cry(id: impl Into<String>, path: impl Into<String>) -> Result<Self> {
        let asset = Self {
            id: id.into(),
            path: path.into(),
            kind: ModpackAudioKind::Cry,
        };
        asset.validate()?;
        Ok(asset)
    }

    pub fn sound_effect(id: impl Into<String>, path: impl Into<String>) -> Result<Self> {
        let asset = Self {
            id: id.into(),
            path: path.into(),
            kind: ModpackAudioKind::SoundEffect,
        };
        asset.validate()?;
        Ok(asset)
    }

    pub fn from_content_pack_path(path: impl Into<String>) -> Result<Self> {
        let path = path.into();
        let asset_path = Path::new(&path);
        let stem = asset_path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .filter(|stem| !stem.trim().is_empty())
            .with_context(|| format!("audio file path '{path}' must have a file stem"))?;
        let parent = asset_path
            .parent()
            .and_then(Path::file_name)
            .and_then(|name| name.to_str())
            .with_context(|| {
                format!("audio file path '{path}' must live under music, sfx, or cries")
            })?;
        match parent {
            "music" => Self::music(stem.to_string(), path),
            "sfx" => Self::sound_effect(stem.to_string(), path),
            "cries" => Self::cry(stem.to_string(), path),
            _ => anyhow::bail!("audio file path '{path}' must live under music, sfx, or cries"),
        }
    }

    pub fn validate(&self) -> Result<()> {
        if self.id.trim().is_empty() {
            anyhow::bail!("audio asset id is required");
        }
        let valid_id = match self.kind {
            ModpackAudioKind::Music => self.id.starts_with("MUSIC_"),
            ModpackAudioKind::SoundEffect => self.id.starts_with("SFX_"),
            ModpackAudioKind::Cry => self.id.starts_with("CRY_"),
        } && self
            .id
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_');
        if !valid_id {
            anyhow::bail!(
                "audio asset '{}' must use an exact {:?} id",
                self.id,
                self.kind
            );
        }
        let path = Path::new(&self.path);
        let stem = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .ok_or_else(|| {
                anyhow::anyhow!("audio asset '{}' path must have a file stem", self.id)
            })?;
        if stem != self.id {
            anyhow::bail!(
                "audio asset '{}' path stem '{}' must match the exact audio id",
                self.id,
                stem
            );
        }
        let extension = path
            .extension()
            .and_then(|extension| extension.to_str())
            .ok_or_else(|| {
                anyhow::anyhow!("audio asset '{}' path must have a file extension", self.id)
            })?;
        match self.kind {
            ModpackAudioKind::Music | ModpackAudioKind::SoundEffect | ModpackAudioKind::Cry
                if extension == "mid" =>
            {
                Ok(())
            }
            ModpackAudioKind::Music | ModpackAudioKind::SoundEffect | ModpackAudioKind::Cry => {
                anyhow::bail!("audio asset '{}' must use a .mid file", self.id)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MapModule {
    pub id: String,
    pub attributes: MapAttributes,
    pub scripts: BTreeMap<String, Value>,
    pub trainer_scripts: BTreeMap<String, TrainerBattleRequest>,
    pub scripted_trainer_battles: Vec<ScriptedTrainerBattle>,
    pub scripted_wild_battles: Vec<ScriptedWildBattle>,
    pub script_item_grants: Vec<ScriptItemGrant>,
    pub script_item_checks: Vec<ScriptItemAccess>,
    pub script_item_takes: Vec<ScriptItemAccess>,
    pub script_economy_commands: Vec<ScriptEconomyCommand>,
    pub gift_pokemon_scripts: Vec<GiftPokemonScript>,
    pub script_flag_commands: Vec<ScriptFlagCommand>,
    pub script_scene_commands: Vec<ScriptSceneCommand>,
    pub script_audio_commands: Vec<ScriptAudioCommand>,
    pub script_block_changes: Vec<ScriptBlockChange>,
    pub script_object_commands: Vec<ScriptObjectCommand>,
    pub script_movements: Vec<ScriptMovement>,
    pub script_map_commands: Vec<ScriptMapCommand>,
    pub script_text_commands: Vec<ScriptTextCommand>,
    pub script_text_bodies: BTreeMap<String, ScriptTextBody>,
    pub script_menu_definitions: BTreeMap<String, ScriptMenuDefinition>,
    pub script_variable_commands: Vec<ScriptVariableCommand>,
    pub script_control_commands: Vec<ScriptControlCommand>,
    pub script_field_pickups: Vec<ScriptFieldPickup>,
    pub script_shop_commands: Vec<ScriptShopCommand>,
    pub script_phone_commands: Vec<ScriptPhoneCommand>,
    pub script_runtime_commands: Vec<ScriptRuntimeCommand>,
    pub map_script_section_commands: Vec<MapScriptSectionCommand>,
    pub map_event_section_commands: Vec<MapEventSectionCommand>,
    pub scenes: MapSceneTable,
    pub events: MapEvents,
    pub objects: Vec<ObjectEvent>,
    pub blocks: Vec<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptedWildBattle {
    pub source_script: String,
    pub loadwildmon_command_index: usize,
    pub startbattle_command_index: usize,
    pub request: StaticWildBattleRequest,
    pub reload_map_after_battle: bool,
    pub pre_battle_event_flags: Vec<String>,
    pub post_battle_event_flags: Vec<String>,
    pub post_battle_script_flags: Vec<String>,
    pub disappear_object_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptedTrainerBattle {
    pub source_script: String,
    pub loadtrainer_command_index: usize,
    pub startbattle_command_index: usize,
    pub request: TrainerBattleRequest,
    pub reload_map_after_battle: bool,
    pub post_battle_event_flags: Vec<String>,
    pub post_battle_script_flags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModpackCompileOptions {
    pub verify: bool,
    pub playability: PlayabilityRules,
}

impl Default for ModpackCompileOptions {
    fn default() -> Self {
        Self {
            verify: true,
            playability: PlayabilityRules::default(),
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlayabilityRules {
    pub start_maps: Vec<String>,
    pub start_tiles: Vec<PlayabilityStart>,
    pub initial_events: Vec<String>,
    pub initial_items: Vec<String>,
    pub goal_maps: Vec<String>,
    pub goal_events: Vec<String>,
    pub goal_items: Vec<String>,
    pub progression_rules: Vec<ProgressionRule>,
    pub map_access: Vec<MapAccessRule>,
    pub require_all_maps_reachable: bool,
    pub require_walkable_maps: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlayabilityStart {
    pub map: String,
    pub tile: TilePosition,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProgressionRequirements {
    pub events: Vec<String>,
    pub items: Vec<String>,
    pub maps: Vec<String>,
}

impl ProgressionRequirements {
    fn is_empty(&self) -> bool {
        self.events.is_empty() && self.items.is_empty() && self.maps.is_empty()
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProgressionGrants {
    pub events: Vec<String>,
    pub items: Vec<String>,
    pub maps: Vec<String>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProgressionRule {
    pub id: String,
    pub requires: ProgressionRequirements,
    pub grants: ProgressionGrants,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MapAccessRule {
    pub map: String,
    pub requires: ProgressionRequirements,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompiledModpack {
    pub data: GameDataSet,
    pub report: ModpackCompileReport,
}

impl CompiledModpack {
    pub fn into_game_pack(self) -> CompiledGamePack {
        CompiledGamePack {
            format_version: COMPILED_GAME_PACK_FORMAT_VERSION,
            data: self.data,
            report: self.report,
        }
    }

    pub fn write_game_pack(&self, path: impl AsRef<Path>) -> Result<()> {
        write_compiled_game_pack(
            path,
            &CompiledGamePack {
                format_version: COMPILED_GAME_PACK_FORMAT_VERSION,
                data: self.data.clone(),
                report: self.report.clone(),
            },
        )
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompiledGamePack {
    pub format_version: u16,
    pub data: GameDataSet,
    pub report: ModpackCompileReport,
}

impl CompiledGamePack {
    pub fn new(data: GameDataSet, report: ModpackCompileReport) -> Self {
        Self {
            format_version: COMPILED_GAME_PACK_FORMAT_VERSION,
            data,
            report,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LoadedCompiledGamePack {
    pub path: PathBuf,
    pub bytes: Vec<u8>,
    pub pack: CompiledGamePack,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModpackCompileReport {
    pub manifests: Vec<String>,
    pub maps: usize,
    pub pokemon: usize,
    pub moves: usize,
    pub items: usize,
    pub graph_edges: Vec<PlayabilityGraphEdge>,
    pub reachable_maps: Vec<String>,
    pub solvable_maps: Vec<String>,
    pub solvable_events: Vec<String>,
    pub solvable_items: Vec<String>,
    pub diagnostics: Vec<VerificationError>,
}

impl ModpackCompileReport {
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == VerificationSeverity::Error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlayabilityGraphEdge {
    pub from: String,
    pub to: String,
    pub kind: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationSeverity {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerificationError {
    pub severity: VerificationSeverity,
    pub code: String,
    pub subject: String,
    pub message: String,
}

impl VerificationError {
    fn error(
        code: impl Into<String>,
        subject: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity: VerificationSeverity::Error,
            code: code.into(),
            subject: subject.into(),
            message: message.into(),
        }
    }

    fn warning(
        code: impl Into<String>,
        subject: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity: VerificationSeverity::Warning,
            code: code.into(),
            subject: subject.into(),
            message: message.into(),
        }
    }
}

pub struct ModpackCompiler<'a> {
    asset_root: &'a AssetRoot,
}

impl<'a> ModpackCompiler<'a> {
    pub fn new(asset_root: &'a AssetRoot) -> Self {
        Self { asset_root }
    }

    pub fn compile(
        &self,
        manifests: &[ModpackManifest],
        options: ModpackCompileOptions,
    ) -> Result<CompiledModpack> {
        let mut data = self.asset_root.load_base_game_data()?;
        let mut seen_manifest_ids = BTreeSet::new();
        for manifest in manifests {
            validate_manifest_shape(manifest)?;
            if !seen_manifest_ids.insert(manifest.id().to_string()) {
                anyhow::bail!("duplicate modpack manifest id '{}'", manifest.id());
            }
        }
        for manifest in manifests {
            for dependency in &manifest.dependencies {
                if !seen_manifest_ids.contains(dependency) {
                    anyhow::bail!(
                        "modpack '{}' depends on missing modpack '{}'",
                        manifest.id(),
                        dependency
                    );
                }
            }
        }
        let mut manifests_sorted: Vec<&ModpackManifest> = manifests.iter().collect();
        manifests_sorted
            .sort_by(|a, b| a.priority.cmp(&b.priority).then_with(|| a.id().cmp(b.id())));
        for manifest in manifests_sorted {
            data.apply_modpack(manifest)?;
        }
        materialize_runtime_map_modules(&mut data)?;

        let playability = merged_playability_rules(&data.playability, &options.playability);
        let mut report = verify_game_data(self.asset_root, &data, &playability);
        report.manifests = manifests
            .iter()
            .map(|manifest| manifest.id().to_string())
            .collect();
        report.maps = all_map_names(&data).len();
        report.pokemon = data.pokemon.len();
        report.moves = data.moves.len();
        report.items = data.items.len();

        if options.verify && report.has_errors() {
            let summary = report
                .diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.severity == VerificationSeverity::Error)
                .take(8)
                .map(|diagnostic| {
                    format!(
                        "{} [{}]: {}",
                        diagnostic.subject, diagnostic.code, diagnostic.message
                    )
                })
                .collect::<Vec<_>>()
                .join("; ");
            anyhow::bail!("modpack verification failed: {summary}");
        }

        Ok(CompiledModpack { data, report })
    }
}

fn validate_manifest_shape(manifest: &ModpackManifest) -> Result<()> {
    if manifest.schema_version != 1 {
        anyhow::bail!(
            "unsupported modpack schema_version {} for '{}'",
            manifest.schema_version,
            manifest.id()
        );
    }
    if manifest.id().trim().is_empty() {
        anyhow::bail!("modpack metadata.id is required");
    }
    if manifest.metadata.name.trim().is_empty() {
        anyhow::bail!("modpack '{}' metadata.name is required", manifest.id());
    }
    if manifest.metadata.version.trim().is_empty() {
        anyhow::bail!("modpack '{}' metadata.version is required", manifest.id());
    }
    Ok(())
}

fn verify_game_data(
    asset_root: &AssetRoot,
    data: &GameDataSet,
    rules: &PlayabilityRules,
) -> ModpackCompileReport {
    let mut diagnostics = Vec::new();
    let map_names = all_map_names(data);

    verify_species_and_moves(data, &mut diagnostics);
    verify_items(data, &mut diagnostics);
    verify_evolutions(data, &mut diagnostics);
    verify_encounters(data, &map_names, &mut diagnostics);
    verify_audio_assets(asset_root, data, &mut diagnostics);
    verify_map_music(data, &mut diagnostics);
    verify_trainer_encounter_music(data, &mut diagnostics);
    verify_capture_rules(data, &mut diagnostics);
    verify_capture_wobble_probabilities(data, &mut diagnostics);
    verify_battle_escape_rules(data, &mut diagnostics);
    verify_move_priorities(data, &mut diagnostics);
    verify_type_categories(data, &mut diagnostics);
    verify_type_effectiveness(data, &mut diagnostics);
    verify_weather_modifiers(data, &mut diagnostics);
    verify_battle_reward_rules(data, &mut diagnostics);
    verify_step_event_rules(data, &mut diagnostics);
    verify_runtime_pack_data(data, &mut diagnostics);
    verify_encounter_slot_tables(data, &mut diagnostics);
    verify_encounter_music_modifiers(data, &mut diagnostics);
    verify_battle_stat_multipliers(data, &mut diagnostics);
    verify_phone_contacts(data, &mut diagnostics);
    verify_special_routines(data, &mut diagnostics);
    verify_marts(data, &mut diagnostics);
    verify_fruit_trees(data, &mut diagnostics);
    verify_script_item_grants(data, &mut diagnostics);
    verify_script_economy_commands(data, &mut diagnostics);
    verify_gift_pokemon_scripts(data, &mut diagnostics);
    verify_script_flag_commands(data, &mut diagnostics);
    verify_script_scene_commands(data, &mut diagnostics);
    verify_script_audio_commands(data, &mut diagnostics);
    verify_script_block_changes(data, &mut diagnostics);
    verify_script_movements(data, &mut diagnostics);
    verify_script_object_commands(data, &mut diagnostics);
    verify_script_map_commands(data, &mut diagnostics);
    verify_script_text_commands(data, &mut diagnostics);
    verify_script_text_bodies(data, &mut diagnostics);
    verify_script_menu_definitions(data, &mut diagnostics);
    verify_script_variable_commands(data, &mut diagnostics);
    verify_script_control_commands(data, &mut diagnostics);
    verify_map_section_commands(data, &mut diagnostics);
    verify_script_field_pickups(data, &mut diagnostics);
    verify_script_shop_commands(data, &mut diagnostics);
    verify_script_phone_commands(data, &mut diagnostics);
    verify_script_runtime_commands(data, &mut diagnostics);
    verify_fishing(data, &mut diagnostics);
    verify_field_moves(data, &mut diagnostics);
    let graph = verify_maps(asset_root, data, &map_names, rules, &mut diagnostics);

    let reachable_maps = reachable_maps(&map_names, &graph, rules);
    verify_progression_rules(data, &map_names, rules, &mut diagnostics);
    let progression = solve_progression(&reachable_maps, &map_names, rules);
    let loaded_maps: Vec<String> = map_names.iter().cloned().collect();
    let loaded_progression = solve_progression(&loaded_maps, &map_names, rules);
    verify_solubility(
        &map_names,
        &reachable_maps,
        &progression,
        &loaded_progression,
        rules,
        &mut diagnostics,
    );

    ModpackCompileReport {
        graph_edges: graph
            .edges
            .iter()
            .map(|edge| PlayabilityGraphEdge {
                from: edge.from_map.clone(),
                to: edge.to_map.clone(),
                kind: edge.kind.clone(),
            })
            .collect(),
        reachable_maps,
        solvable_maps: progression.maps.iter().cloned().collect(),
        solvable_events: loaded_progression.events.iter().cloned().collect(),
        solvable_items: loaded_progression.items.iter().cloned().collect(),
        diagnostics,
        ..ModpackCompileReport::default()
    }
}

fn merged_playability_rules(
    base: &PlayabilityRules,
    overlay: &PlayabilityRules,
) -> PlayabilityRules {
    let mut merged = base.clone();
    merge_playability_rules(&mut merged, overlay);
    merged
}

fn merge_playability_rules(target: &mut PlayabilityRules, source: &PlayabilityRules) {
    target.start_maps.extend(source.start_maps.iter().cloned());
    target
        .start_tiles
        .extend(source.start_tiles.iter().cloned());
    target
        .initial_events
        .extend(source.initial_events.iter().cloned());
    target
        .initial_items
        .extend(source.initial_items.iter().cloned());
    target.goal_maps.extend(source.goal_maps.iter().cloned());
    target
        .goal_events
        .extend(source.goal_events.iter().cloned());
    target.goal_items.extend(source.goal_items.iter().cloned());
    target
        .progression_rules
        .extend(source.progression_rules.iter().cloned());
    target.map_access.extend(source.map_access.iter().cloned());
    target.require_all_maps_reachable |= source.require_all_maps_reachable;
    target.require_walkable_maps |= source.require_walkable_maps;
}

fn materialize_runtime_map_modules(data: &mut GameDataSet) -> Result<()> {
    let map_names: Vec<String> = data.map_attributes.keys().cloned().collect();
    for map_name in map_names {
        if data.maps.contains_key(&map_name) {
            continue;
        }
        let module = data
            .map_module(&map_name)
            .with_context(|| format!("materialize runtime map module {map_name}"))?;
        data.maps.insert(map_name, module);
    }
    Ok(())
}

fn verify_species_and_moves(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    for (species_id, species) in &data.pokemon {
        if !data.learnsets.contains_key(species_id) {
            diagnostics.push(VerificationError::error(
                "missing_species_learnset",
                species_id,
                "Pokemon species is missing an explicit level-up learnset",
            ));
        }
        for item_id in [species.item1.as_deref(), species.item2.as_deref()]
            .into_iter()
            .flatten()
        {
            if !data.items.contains_key(item_id) {
                diagnostics.push(VerificationError::error(
                    "unknown_species_held_item",
                    species_id,
                    format!("Pokemon species references missing held item '{item_id}'"),
                ));
            }
        }
        for move_name in &species.tmhm_learnset {
            if !data.moves.contains_key(move_name) {
                diagnostics.push(VerificationError::error(
                    "unknown_tmhm_move",
                    species_id,
                    format!("TM/HM learnset references missing move '{move_name}'"),
                ));
            }
        }
    }
    for (species, learnset) in &data.learnsets {
        if !data.pokemon.contains_key(species) {
            diagnostics.push(VerificationError::error(
                "unknown_learnset_species",
                species,
                "learnset references a species that is not loaded",
            ));
        }
        for entry in learnset {
            let move_name = &entry.1;
            if !data.moves.contains_key(move_name) {
                diagnostics.push(VerificationError::error(
                    "unknown_level_move",
                    species,
                    format!("level-up learnset references missing move '{move_name}'"),
                ));
            }
        }
    }
}

fn verify_items(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    for (item_id, item) in &data.items {
        if item.pocket == ITEM_POCKET_BALL
            && !data
                .capture_rules
                .ball_rules
                .contains_key(&item.script_name)
            && !data
                .capture_rules
                .guaranteed_capture_balls
                .contains(&item.script_name)
        {
            diagnostics.push(VerificationError::error(
                "unknown_capture_ball_item",
                item_id,
                format!(
                    "BALL pocket item '{}' uses unsupported capture ball id '{}'",
                    item_id, item.script_name
                ),
            ));
        }
        verify_item_payload_fields(item_id, item, diagnostics);
    }
}

fn verify_item_payload_fields(
    item_id: &str,
    item: &Item,
    diagnostics: &mut Vec<VerificationError>,
) {
    if item.parameter != 0 {
        verify_item_heal_amount(item_id, item, diagnostics);
    }
    if item.revive_hp_percent.is_some() {
        verify_item_percent(
            item_id,
            item.revive_hp_percent,
            "revive_hp_percent",
            "invalid_item_revive_hp_percent",
            diagnostics,
        );
    }
    if item.party_revive_hp_percent.is_some() {
        verify_item_percent(
            item_id,
            item.party_revive_hp_percent,
            "party_revive_hp_percent",
            "invalid_item_party_revive_hp_percent",
            diagnostics,
        );
    }
    if item.pp_restore_scope.is_some() || item.pp_restore_points.is_some() {
        verify_item_pp_restore_payload(item_id, item, diagnostics);
    }
    if let Some(stages) = item.pp_up_stages {
        if !(1..=3).contains(&stages) {
            diagnostics.push(VerificationError::error(
                "invalid_item_pp_up_stages",
                item_id,
                format!("pp_up_stages must be from 1 to 3, found {stages}"),
            ));
        }
    }
    if item.vitamin_stat.is_some()
        || item.vitamin_stat_exp.is_some()
        || item.vitamin_max_stat_exp.is_some()
    {
        verify_item_vitamin_payload(item_id, item, diagnostics);
    }
    if let Some(level_gain) = item.rare_candy_level_gain {
        if level_gain == 0 {
            diagnostics.push(VerificationError::error(
                "invalid_item_rare_candy_level_gain",
                item_id,
                format!("rare_candy_level_gain must be positive, found {level_gain}"),
            ));
        }
    }
    if item.battle_stat_boost_stat.is_some() || item.battle_stat_boost_stages.is_some() {
        verify_item_battle_stat_boost_payload(item_id, item, diagnostics);
    }
    if let Some(guard) = item.battle_stat_drop_guard {
        if !guard {
            diagnostics.push(VerificationError::error(
                "invalid_item_battle_stat_drop_guard",
                item_id,
                "battle_stat_drop_guard must be true when declared",
            ));
        }
    }
    if let Some(mode) = item.battle_escape_mode.as_deref() {
        if mode != "WILD_BATTLE" {
            diagnostics.push(VerificationError::error(
                "invalid_item_battle_escape_mode",
                item_id,
                format!("battle_escape_mode must be 'WILD_BATTLE' when declared, found '{mode}'"),
            ));
        }
    }
    if let Some(steps) = item.repel_steps {
        if steps == 0 {
            diagnostics.push(VerificationError::error(
                "invalid_item_repel_steps",
                item_id,
                format!("repel_steps must be positive when declared, found {steps}"),
            ));
        }
    }
    if let Some(focus) = item.battle_focus_energy {
        if !focus {
            diagnostics.push(VerificationError::error(
                "invalid_item_battle_focus_energy",
                item_id,
                "battle_focus_energy must be true when declared",
            ));
        }
    }
    if let Some(confusion_heal) = item.confusion_heal {
        if !confusion_heal {
            diagnostics.push(VerificationError::error(
                "invalid_item_confusion_heal",
                item_id,
                "confusion_heal must be true when declared",
            ));
        }
    }
}

fn verify_item_heal_amount(item_id: &str, item: &Item, diagnostics: &mut Vec<VerificationError>) {
    if item.parameter == -1 || item.parameter > 0 {
        return;
    }
    diagnostics.push(VerificationError::error(
        "invalid_item_heal_amount",
        item_id,
        format!(
            "{} requires parameter -1 or a positive HP amount, found {}",
            item.effect, item.parameter
        ),
    ));
}

fn verify_item_percent(
    item_id: &str,
    percent: Option<u8>,
    field_name: &str,
    invalid_code: &str,
    diagnostics: &mut Vec<VerificationError>,
) {
    match percent {
        Some(percent) if (1..=100).contains(&percent) => {}
        Some(percent) => diagnostics.push(VerificationError::error(
            invalid_code,
            item_id,
            format!("{field_name} must be from 1 to 100, found {percent}"),
        )),
        None => {}
    }
}

fn verify_item_pp_restore_payload(
    item_id: &str,
    item: &Item,
    diagnostics: &mut Vec<VerificationError>,
) {
    match item.pp_restore_scope.as_deref() {
        Some("MOVE" | "POKEMON") => {}
        Some(scope) => diagnostics.push(VerificationError::error(
            "invalid_item_pp_restore_scope",
            item_id,
            format!("RESTORE_PP requires pp_restore_scope 'MOVE' or 'POKEMON', found '{scope}'"),
        )),
        None => diagnostics.push(VerificationError::error(
            "missing_item_pp_restore_scope",
            item_id,
            "RESTORE_PP requires explicit pp_restore_scope",
        )),
    }
    if let Some(0) = item.pp_restore_points {
        diagnostics.push(VerificationError::error(
            "invalid_item_pp_restore_points",
            item_id,
            "RESTORE_PP pp_restore_points must be positive when present",
        ));
    }
}

fn verify_item_vitamin_payload(
    item_id: &str,
    item: &Item,
    diagnostics: &mut Vec<VerificationError>,
) {
    match item.vitamin_stat.as_deref() {
        Some("HP" | "ATTACK" | "DEFENSE" | "SPEED" | "SPECIAL") => {}
        Some(stat) => diagnostics.push(VerificationError::error(
            "invalid_item_vitamin_stat",
            item_id,
            format!("VITAMIN uses unknown vitamin_stat '{stat}'"),
        )),
        None => diagnostics.push(VerificationError::error(
            "missing_item_vitamin_stat",
            item_id,
            "VITAMIN requires explicit vitamin_stat",
        )),
    }
    match item.vitamin_stat_exp {
        Some(amount) if amount > 0 => {}
        Some(amount) => diagnostics.push(VerificationError::error(
            "invalid_item_vitamin_stat_exp",
            item_id,
            format!("VITAMIN requires positive vitamin_stat_exp, found {amount}"),
        )),
        None => diagnostics.push(VerificationError::error(
            "missing_item_vitamin_stat_exp",
            item_id,
            "VITAMIN requires explicit vitamin_stat_exp",
        )),
    }
    match (item.vitamin_max_stat_exp, item.vitamin_stat_exp) {
        (Some(max), Some(amount)) if max >= amount && max > 0 => {}
        (Some(max), _) => diagnostics.push(VerificationError::error(
            "invalid_item_vitamin_max_stat_exp",
            item_id,
            format!("VITAMIN requires vitamin_max_stat_exp >= vitamin_stat_exp and positive, found {max}"),
        )),
        (None, _) => diagnostics.push(VerificationError::error(
            "missing_item_vitamin_max_stat_exp",
            item_id,
            "VITAMIN requires explicit vitamin_max_stat_exp",
        )),
    }
}

fn verify_item_battle_stat_boost_payload(
    item_id: &str,
    item: &Item,
    diagnostics: &mut Vec<VerificationError>,
) {
    match item.battle_stat_boost_stat.as_deref() {
        Some("ATTACK" | "DEFENSE" | "SPEED" | "SPECIAL_ATTACK" | "ACCURACY") => {}
        Some(stat) => diagnostics.push(VerificationError::error(
            "invalid_item_battle_stat_boost_stat",
            item_id,
            format!(
                "{} uses unknown battle_stat_boost_stat '{stat}'",
                item.effect
            ),
        )),
        None => diagnostics.push(VerificationError::error(
            "missing_item_battle_stat_boost_stat",
            item_id,
            format!("{} requires explicit battle_stat_boost_stat", item.effect),
        )),
    }
    match item.battle_stat_boost_stages {
        Some(stages) if (1..=6).contains(&stages) => {}
        Some(stages) => diagnostics.push(VerificationError::error(
            "invalid_item_battle_stat_boost_stages",
            item_id,
            format!(
                "{} requires battle_stat_boost_stages from 1 to 6, found {stages}",
                item.effect
            ),
        )),
        None => diagnostics.push(VerificationError::error(
            "missing_item_battle_stat_boost_stages",
            item_id,
            format!("{} requires explicit battle_stat_boost_stages", item.effect),
        )),
    }
}

fn verify_evolutions(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    for species_id in data.pokemon.keys() {
        if !data.evolutions.0.contains_key(species_id) {
            diagnostics.push(VerificationError::error(
                "missing_species_evolutions",
                species_id,
                "Pokemon species is missing an explicit evolution table entry",
            ));
        }
    }
    for (species_id, entries) in &data.evolutions.0 {
        if !data.pokemon.contains_key(species_id) {
            diagnostics.push(VerificationError::error(
                "unknown_evolution_source_species",
                species_id,
                "evolution table references a source species that is not loaded",
            ));
        }
        for entry in entries {
            if !data.pokemon.contains_key(&entry.species) {
                diagnostics.push(VerificationError::error(
                    "unknown_evolution_target_species",
                    species_id,
                    format!("evolution target species '{}' is not loaded", entry.species),
                ));
            }
            match entry.method.as_str() {
                METHOD_LEVEL => {
                    if entry.level.is_none() {
                        diagnostics.push(VerificationError::error(
                            "missing_evolution_level",
                            species_id,
                            "LEVEL evolution requires an exact level",
                        ));
                    }
                }
                METHOD_ITEM => match entry.item.as_deref() {
                    Some(item_id) if data.items.contains_key(item_id) => {}
                    Some(item_id) => diagnostics.push(VerificationError::error(
                        "unknown_evolution_item",
                        species_id,
                        format!("ITEM evolution references missing item '{item_id}'"),
                    )),
                    None => diagnostics.push(VerificationError::error(
                        "missing_evolution_item",
                        species_id,
                        "ITEM evolution requires an exact item id",
                    )),
                },
                METHOD_HAPPINESS => match entry.happiness.as_deref() {
                    Some(window) if is_known_happiness_window(window) => {}
                    Some(window) => diagnostics.push(VerificationError::error(
                        "unknown_evolution_happiness_window",
                        species_id,
                        format!("HAPPINESS evolution uses unknown window '{window}'"),
                    )),
                    None => diagnostics.push(VerificationError::error(
                        "missing_evolution_happiness_window",
                        species_id,
                        "HAPPINESS evolution requires an exact time window",
                    )),
                },
                METHOD_TRADE => {
                    if let Some(item_id) = entry.held_item.as_deref() {
                        if item_id != TRADE_ANY_ITEM && !data.items.contains_key(item_id) {
                            diagnostics.push(VerificationError::error(
                                "unknown_trade_evolution_item",
                                species_id,
                                format!("TRADE evolution references missing held item '{item_id}'"),
                            ));
                        }
                    }
                }
                METHOD_STAT => {
                    if entry.level.is_none() {
                        diagnostics.push(VerificationError::error(
                            "missing_stat_evolution_level",
                            species_id,
                            "STAT evolution requires an exact level",
                        ));
                    }
                    match entry.stat_ratio.as_deref() {
                        Some(ratio) if is_known_stat_evolution_ratio(ratio) => {}
                        Some(ratio) => diagnostics.push(VerificationError::error(
                            "unknown_evolution_stat_ratio",
                            species_id,
                            format!("STAT evolution uses unknown ratio '{ratio}'"),
                        )),
                        None => diagnostics.push(VerificationError::error(
                            "missing_evolution_stat_ratio",
                            species_id,
                            "STAT evolution requires an exact stat ratio",
                        )),
                    }
                }
                method => diagnostics.push(VerificationError::error(
                    "unknown_evolution_method",
                    species_id,
                    format!("evolution uses unknown method '{method}'"),
                )),
            }
        }
    }
}

fn verify_encounters(
    data: &GameDataSet,
    map_names: &BTreeSet<String>,
    diagnostics: &mut Vec<VerificationError>,
) {
    for (map_name, encounters) in &data.wild_encounters {
        if !map_names.contains(map_name) {
            diagnostics.push(VerificationError::error(
                "unknown_encounter_map",
                map_name,
                "wild encounters reference a map that is not loaded",
            ));
        }
        for species in encounter_species(encounters) {
            if !data.pokemon.contains_key(&species) {
                diagnostics.push(VerificationError::error(
                    "unknown_encounter_species",
                    map_name,
                    format!("wild encounters reference missing species '{species}'"),
                ));
            }
        }
        verify_wild_encounter_rates_and_tables(map_name, encounters, diagnostics);
    }
    for (map_name, encounters) in &data.field_encounters {
        if !map_names.contains(map_name) {
            diagnostics.push(VerificationError::error(
                "unknown_field_encounter_map",
                map_name,
                "field encounters reference a map that is not loaded",
            ));
        }
        for species in field_encounter_species(encounters) {
            if !data.pokemon.contains_key(&species) {
                diagnostics.push(VerificationError::error(
                    "unknown_field_encounter_species",
                    map_name,
                    format!("field encounters reference missing species '{species}'"),
                ));
            }
        }
        verify_field_encounter_tables(map_name, encounters, diagnostics);
    }
}

fn verify_field_encounter_tables(
    map_name: &str,
    encounters: &FieldEncounterData,
    diagnostics: &mut Vec<VerificationError>,
) {
    if let Some(headbutt) = encounters.headbutt.as_ref() {
        verify_field_encounter_bucket(
            map_name,
            "headbutt",
            "common",
            &headbutt.common,
            diagnostics,
        );
        verify_field_encounter_bucket(map_name, "headbutt", "rare", &headbutt.rare, diagnostics);
    }
    if let Some(rock_smash) = encounters.rock_smash.as_ref() {
        verify_field_encounter_bucket(
            map_name,
            "rock_smash",
            "common",
            &rock_smash.common,
            diagnostics,
        );
    }
}

fn verify_field_encounter_bucket(
    map_name: &str,
    kind: &'static str,
    bucket: &'static str,
    entries: &[FieldEncounterEntry],
    diagnostics: &mut Vec<VerificationError>,
) {
    let subject = format!("{map_name}:{kind}:{bucket}");
    if entries.is_empty() {
        diagnostics.push(VerificationError::error(
            "empty_field_encounter_bucket",
            &subject,
            format!("{kind} field encounters require a non-empty {bucket} bucket"),
        ));
        return;
    }

    for (index, entry) in entries.iter().enumerate() {
        if entry.weight == 0 {
            diagnostics.push(VerificationError::error(
                "zero_weight_field_encounter",
                &format!("{subject}:{index}"),
                format!(
                    "{kind} field encounter {bucket} entry for '{}' has zero weight",
                    entry.species
                ),
            ));
        }
    }

    let total_weight: u16 = entries.iter().map(|entry| u16::from(entry.weight)).sum();
    if total_weight != 100 {
        diagnostics.push(VerificationError::error(
            "invalid_field_encounter_weight_total",
            &subject,
            format!("{kind} field encounter {bucket} weights total {total_weight}, expected 100"),
        ));
    }
}

fn verify_wild_encounter_rates_and_tables(
    map_name: &str,
    encounters: &WildEncounterData,
    diagnostics: &mut Vec<VerificationError>,
) {
    if let Some(rates) = encounters.grass_rates.as_ref() {
        for time in rates.keys() {
            if !ENCOUNTER_TIME_KEYS.contains(&time.as_str()) {
                diagnostics.push(VerificationError::error(
                    "unknown_grass_encounter_rate_time",
                    map_name,
                    format!("grass encounter rate uses unknown exact time key '{time}'"),
                ));
            }
        }
    }

    if let Some(grass) = encounters.grass.as_ref() {
        for time in ENCOUNTER_TIME_KEYS {
            let time_of_day =
                resolve_encounter_time_key(time).expect("core encounter time key must resolve");
            let slots = grass.slots(time_of_day);
            let rate = encounters
                .grass_rates
                .as_ref()
                .and_then(|rates| rates.get(*time))
                .copied();

            if rate.is_none() {
                diagnostics.push(VerificationError::error(
                    "missing_grass_encounter_rate",
                    map_name,
                    format!("grass encounters for '{time}' require an exact grass rate"),
                ));
            }
            if let Some(rate) = rate {
                if rate > 0 && slots.is_empty() {
                    diagnostics.push(VerificationError::error(
                        "empty_grass_encounter_slots",
                        map_name,
                        format!("grass encounter rate for '{time}' has no slots"),
                    ));
                }
            }
        }
    } else if encounters
        .grass_rates
        .as_ref()
        .is_some_and(|rates| rates.values().any(|rate| *rate > 0))
    {
        diagnostics.push(VerificationError::error(
            "missing_grass_encounter_table",
            map_name,
            "positive grass encounter rates require a grass encounter table",
        ));
    }

    if let Some(water) = encounters.water.as_ref() {
        if encounters.water_rate.is_none() {
            diagnostics.push(VerificationError::error(
                "missing_water_encounter_rate",
                map_name,
                "water encounters require an exact water rate",
            ));
        }
        if let Some(water_rate) = encounters.water_rate {
            if water_rate > 0 {
                for time in ENCOUNTER_TIME_KEYS {
                    let time_of_day = resolve_encounter_time_key(time)
                        .expect("core encounter time key must resolve");
                    let slots = water.slots(time_of_day);
                    if slots.is_empty() {
                        diagnostics.push(VerificationError::error(
                            "empty_water_encounter_slots",
                            map_name,
                            format!("water encounter rate has no slots for '{time}'"),
                        ));
                    }
                }
            }
        }
    } else if let Some(water_rate) = encounters.water_rate {
        if water_rate > 0 {
            diagnostics.push(VerificationError::error(
                "missing_water_encounter_table",
                map_name,
                "positive water encounter rate requires a water encounter table",
            ));
        }
    }
}

fn verify_audio_assets(
    asset_root: &AssetRoot,
    data: &GameDataSet,
    diagnostics: &mut Vec<VerificationError>,
) {
    for audio_asset in &data.audio {
        if let Err(error) = audio_asset.validate() {
            diagnostics.push(VerificationError::error(
                "invalid_audio_asset",
                &audio_asset.id,
                error.to_string(),
            ));
            continue;
        }
        let path = match asset_root.resolve_data_path(&audio_asset.path) {
            Ok(path) => path,
            Err(error) => {
                diagnostics.push(VerificationError::error(
                    "invalid_audio_path",
                    &audio_asset.id,
                    error.to_string(),
                ));
                continue;
            }
        };
        if !path.exists() {
            diagnostics.push(VerificationError::error(
                "missing_audio_file",
                &audio_asset.id,
                format!("audio file '{}' is missing", audio_asset.path),
            ));
            continue;
        }
        match std::fs::read(&path) {
            Ok(bytes) if bytes.starts_with(b"MThd") => {}
            Ok(_) => diagnostics.push(VerificationError::error(
                "invalid_midi_file",
                &audio_asset.id,
                format!("audio file '{}' is not a MIDI file", audio_asset.path),
            )),
            Err(error) => diagnostics.push(VerificationError::error(
                "unreadable_audio_file",
                &audio_asset.id,
                format!(
                    "audio file '{}' could not be read: {error}",
                    audio_asset.path
                ),
            )),
        }
    }
}

fn verify_map_music(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    let (music, _, _) = script_audio_catalog_ids(data);
    for (map_name, module) in &data.maps {
        let Some(music_id) = module.attributes.music.as_deref() else {
            continue;
        };
        if !music.contains(music_id) {
            diagnostics.push(VerificationError::error(
                "unknown_map_music_id",
                map_name,
                format!("map music references missing music audio id '{music_id}'"),
            ));
        }
    }
}

fn verify_trainer_encounter_music(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    if data.trainers.is_empty() {
        return;
    }
    let (music, _, _) = script_audio_catalog_ids(data);
    for (trainer_id, trainer) in &data.trainers.trainers {
        if trainer.encounter_music.trim().is_empty() {
            diagnostics.push(VerificationError::error(
                "missing_trainer_encounter_music",
                trainer_id,
                "trainer is missing explicit encounter music",
            ));
        } else if !music.contains(&trainer.encounter_music) {
            diagnostics.push(VerificationError::error(
                "unknown_trainer_encounter_music",
                trainer_id,
                format!(
                    "trainer references missing encounter music '{}'",
                    trainer.encounter_music
                ),
            ));
        }
    }
}

fn verify_capture_rules(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    if data
        .items
        .values()
        .any(|item| item.pocket == ITEM_POCKET_BALL)
        && data.capture_rules.ball_rules.is_empty()
    {
        diagnostics.push(VerificationError::error(
            "missing_capture_ball_rules",
            "capture_rules:ball_rules",
            "capture ball rules must be declared when BALL pocket items exist",
        ));
    }
    for species in &data.capture_rules.fast_ball_species {
        if !data.pokemon.contains_key(species) {
            diagnostics.push(VerificationError::error(
                "unknown_fast_ball_species",
                species,
                "Fast Ball rule references a species that is not loaded",
            ));
        }
    }
    for species in data.capture_rules.heavy_ball_modifiers.keys() {
        if !data.pokemon.contains_key(species) {
            diagnostics.push(VerificationError::error(
                "unknown_heavy_ball_species",
                species,
                "Heavy Ball rule references a species that is not loaded",
            ));
        }
    }
    for (ball_id, rule) in &data.capture_rules.ball_rules {
        let subject = format!("capture_rules:ball_rules:{ball_id}");
        for issue in capture_ball_rule_issues(ball_id, rule) {
            match issue {
                CaptureBallRuleIssue::InvalidBallId => diagnostics.push(VerificationError::error(
                    "invalid_capture_ball_id",
                    &subject,
                    "capture ball id must be an exact nonempty id",
                )),
                CaptureBallRuleIssue::InvalidBattleType => {
                    diagnostics.push(VerificationError::error(
                        "invalid_capture_ball_battle_type",
                        &subject,
                        "capture ball battle type must be exact when present",
                    ));
                }
                CaptureBallRuleIssue::InvalidMultiplierDenominator => {
                    diagnostics.push(VerificationError::error(
                        "invalid_capture_ball_multiplier",
                        &subject,
                        "capture ball multiplier denominator must be nonzero",
                    ));
                }
            }
        }
    }
}

fn verify_capture_wobble_probabilities(
    data: &GameDataSet,
    diagnostics: &mut Vec<VerificationError>,
) {
    if !data
        .items
        .values()
        .any(|item| item.pocket == ITEM_POCKET_BALL)
    {
        return;
    }
    if data.capture_wobble_probabilities.is_empty() {
        diagnostics.push(VerificationError::error(
            "missing_capture_wobble_probabilities",
            "capture_wobble_probabilities",
            "capture wobble probabilities must be declared when capture balls exist",
        ));
        return;
    }
    let mut previous = 0;
    for entry in &data.capture_wobble_probabilities {
        if entry.catch_rate == 0 {
            diagnostics.push(VerificationError::error(
                "invalid_capture_wobble_catch_rate",
                "capture_wobble_probabilities",
                "capture wobble catch rates must be in 1..=255",
            ));
        }
        if entry.catch_rate < previous {
            diagnostics.push(VerificationError::error(
                "unordered_capture_wobble_probability",
                "capture_wobble_probabilities",
                format!(
                    "capture wobble catch rate {} appears after {}",
                    entry.catch_rate, previous
                ),
            ));
        }
        previous = entry.catch_rate;
    }
    if previous != u8::MAX {
        diagnostics.push(VerificationError::error(
            "incomplete_capture_wobble_probabilities",
            "capture_wobble_probabilities",
            "capture wobble probabilities must end at catch rate 255",
        ));
    }
}

fn verify_marts(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    for (mart_id, item_ids) in &data.marts.0 {
        if mart_id.trim().is_empty() {
            diagnostics.push(VerificationError::error(
                "empty_mart_id",
                mart_id,
                "mart id is required",
            ));
        }
        for item_id in item_ids {
            if !data.items.contains_key(item_id) {
                diagnostics.push(VerificationError::error(
                    "unknown_mart_item",
                    mart_id,
                    format!("mart references missing item '{item_id}'"),
                ));
            }
        }
    }
}

fn verify_fruit_trees(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    for (fruit_tree_id, item_id) in &data.fruit_trees.0 {
        let subject = format!("fruit_trees:{fruit_tree_id}");
        if fruit_tree_id.trim().is_empty() {
            diagnostics.push(VerificationError::error(
                "empty_fruit_tree_id",
                &subject,
                "fruit tree id must not be empty",
            ));
        }
        if !data.items.contains_key(item_id) {
            diagnostics.push(VerificationError::error(
                "unknown_fruit_tree_item",
                &subject,
                format!("fruit tree references missing item '{item_id}'"),
            ));
        }
    }
}

fn verify_script_item_grants(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    for (map_name, module) in &data.maps {
        for grant in &module.script_item_grants {
            if grant.item_id != "ITEM_FROM_MEM" && !data.items.contains_key(&grant.item_id) {
                diagnostics.push(VerificationError::error(
                    "unknown_script_item_grant_item",
                    format!("{map_name}:{}:{}", grant.source_script, grant.command_index),
                    format!(
                        "script item grant references missing item '{}'",
                        grant.item_id
                    ),
                ));
            }
        }
        for access in &module.script_item_checks {
            verify_script_item_access(data, diagnostics, map_name, access, "checkitem");
        }
        for access in &module.script_item_takes {
            verify_script_item_access(data, diagnostics, map_name, access, "takeitem");
        }
    }
}

fn verify_script_item_access(
    data: &GameDataSet,
    diagnostics: &mut Vec<VerificationError>,
    map_name: &str,
    access: &ScriptItemAccess,
    command: &str,
) {
    if !data.items.contains_key(&access.item_id) {
        diagnostics.push(VerificationError::error(
            "unknown_script_item_access_item",
            format!(
                "{map_name}:{}:{}",
                access.source_script, access.command_index
            ),
            format!("{command} references missing item '{}'", access.item_id),
        ));
    }
}

fn verify_script_economy_commands(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    let constants = economy_constants(data);
    for (map_name, module) in &data.maps {
        for command in &module.script_economy_commands {
            let subject = format!(
                "{map_name}:{}:{}",
                command.source_script, command.command_index
            );
            if SCRIPT_MONEY_CHECK_COMMANDS.contains(&command.command.as_str())
                || SCRIPT_MONEY_MUTATION_COMMANDS.contains(&command.command.as_str())
            {
                let Some(account) = command.account.as_deref() else {
                    diagnostics.push(VerificationError::error(
                        "missing_script_money_account",
                        &subject,
                        "money command is missing account id",
                    ));
                    continue;
                };
                if MoneyAccount::from_script_id(account).is_err() {
                    diagnostics.push(VerificationError::error(
                        "unknown_script_money_account",
                        &subject,
                        format!("money command references unknown account '{account}'"),
                    ));
                }
                if SCRIPT_MONEY_MUTATION_COMMANDS.contains(&command.command.as_str())
                    && constants.get("MAX_MONEY").is_none()
                {
                    diagnostics.push(VerificationError::error(
                        "missing_script_money_cap",
                        &subject,
                        "money mutation requires MAX_MONEY in pack currency constants",
                    ));
                }
            } else if SCRIPT_COIN_CHECK_COMMANDS.contains(&command.command.as_str())
                || SCRIPT_COIN_MUTATION_COMMANDS.contains(&command.command.as_str())
            {
                if command.account.is_some() {
                    diagnostics.push(VerificationError::error(
                        "unexpected_script_coin_account",
                        &subject,
                        "coin command must not carry a money account id",
                    ));
                }
                if SCRIPT_COIN_MUTATION_COMMANDS.contains(&command.command.as_str())
                    && constants.get("MAX_COINS").is_none()
                {
                    diagnostics.push(VerificationError::error(
                        "missing_script_coin_cap",
                        &subject,
                        "coin mutation requires MAX_COINS in pack currency constants",
                    ));
                }
            } else if !is_known_script_economy_command(&command.command) {
                diagnostics.push(VerificationError::error(
                    "unknown_script_economy_command",
                    &subject,
                    format!("unknown economy command '{}'", command.command),
                ));
                continue;
            }
            if let Err(error) = resolve_amount(&command.amount_tokens, &constants) {
                diagnostics.push(VerificationError::error(
                    "unresolved_script_currency_amount",
                    &subject,
                    format!("currency amount does not resolve from pack constants: {error:?}"),
                ));
            }
        }
    }
}

fn economy_constants(data: &GameDataSet) -> CurrencyCatalog {
    let mut constants = data.currency_constants.clone();
    for (constant, value) in &data.story_event_script_constants.global {
        if let Ok(value) = u32::try_from(*value) {
            constants.0.insert(constant.clone(), value);
        }
    }
    for constants_by_map in data.story_event_script_constants.maps.values() {
        for (constant, value) in constants_by_map {
            if let Ok(value) = u32::try_from(*value) {
                constants.0.insert(constant.clone(), value);
            }
        }
    }
    constants
}

fn verify_gift_pokemon_scripts(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    for (map_name, module) in &data.maps {
        for gift in &module.gift_pokemon_scripts {
            let subject = format!("{map_name}:{}:{}", gift.source_script, gift.command_index);
            if !data.pokemon.contains_key(&gift.species_id) {
                diagnostics.push(VerificationError::error(
                    "unknown_gift_pokemon_species",
                    &subject,
                    format!("gift references missing species '{}'", gift.species_id),
                ));
            }
            if let Some(item_id) = gift.held_item_id.as_deref()
                && !data.items.contains_key(item_id)
            {
                diagnostics.push(VerificationError::error(
                    "unknown_gift_pokemon_item",
                    &subject,
                    format!("gift references missing held item '{item_id}'"),
                ));
            }
            for (field, label) in [
                ("nickname", gift.nickname_label.as_deref()),
                ("original trainer", gift.ot_label.as_deref()),
            ] {
                let Some(label) = label else {
                    continue;
                };
                if label.trim().is_empty() {
                    diagnostics.push(VerificationError::error(
                        "empty_gift_pokemon_label",
                        &subject,
                        format!("gift {field} label must be non-empty"),
                    ));
                } else if !module.scripts.contains_key(label) {
                    diagnostics.push(VerificationError::error(
                        "unknown_gift_pokemon_label",
                        &subject,
                        format!("gift {field} label '{label}' is not loaded in map scripts"),
                    ));
                }
            }
        }
    }
}

fn verify_script_flag_commands(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    for (map_name, module) in &data.maps {
        for command in &module.script_flag_commands {
            let subject = format!(
                "{map_name}:{}:{}",
                command.source_script, command.command_index
            );
            for issue in script_flag_command_issues(command) {
                match issue {
                    ScriptFlagCommandIssue::UnknownCommand => {
                        diagnostics.push(VerificationError::error(
                            "unknown_script_flag_command",
                            &subject,
                            format!("unknown script flag command '{}'", command.command),
                        ));
                    }
                    ScriptFlagCommandIssue::EmptyFlagId => {
                        diagnostics.push(VerificationError::error(
                            "empty_script_flag_id",
                            &subject,
                            "script flag command references an empty flag id",
                        ));
                    }
                }
            }
        }
    }
}

fn verify_script_scene_commands(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    for (map_name, module) in &data.maps {
        for command in &module.script_scene_commands {
            let subject = format!(
                "{map_name}:{}:{}",
                command.source_script, command.command_index
            );
            if SCRIPT_SCENE_CHECK_COMMANDS.contains(&command.command.as_str()) {
                if command.map_id.is_some() {
                    diagnostics.push(VerificationError::error(
                        "unexpected_script_scene_map",
                        &subject,
                        "checkscene must not carry a target map id",
                    ));
                }
                if command.scene_id.is_some() {
                    diagnostics.push(VerificationError::error(
                        "unexpected_script_scene_id",
                        &subject,
                        "checkscene must not carry a scene id",
                    ));
                }
                if module.scenes.scenes.is_empty() {
                    diagnostics.push(VerificationError::error(
                        "missing_script_scene_table",
                        &subject,
                        "checkscene requires the current map to declare scenes",
                    ));
                }
            } else if SCRIPT_SCENE_CURRENT_MAP_MUTATION_COMMANDS.contains(&command.command.as_str())
            {
                if command.map_id.is_some() {
                    diagnostics.push(VerificationError::error(
                        "unexpected_script_scene_map",
                        &subject,
                        "setscene must not carry a target map id",
                    ));
                }
                verify_scene_token(
                    diagnostics,
                    &subject,
                    map_name,
                    command.scene_id.as_deref(),
                    &module.scenes,
                    scene_slot_count(module),
                );
            } else if SCRIPT_SCENE_TARGET_MAP_MUTATION_COMMANDS.contains(&command.command.as_str())
            {
                let Some(map_id) = command.map_id.as_deref() else {
                    diagnostics.push(VerificationError::error(
                        "missing_script_scene_map",
                        &subject,
                        "setmapscene requires a target map id",
                    ));
                    continue;
                };
                let Some((target_map_name, target_module)) = scene_table_for_map_id(data, map_id)
                else {
                    diagnostics.push(VerificationError::error(
                        "unknown_script_scene_map",
                        &subject,
                        format!("setmapscene references missing map id '{map_id}'"),
                    ));
                    continue;
                };
                verify_scene_token(
                    diagnostics,
                    &subject,
                    &target_map_name,
                    command.scene_id.as_deref(),
                    &target_module.scenes,
                    scene_slot_count(target_module),
                );
            } else if !is_known_script_scene_command(&command.command) {
                diagnostics.push(VerificationError::error(
                    "unknown_script_scene_command",
                    &subject,
                    format!("unknown scene command '{}'", command.command),
                ));
            }
        }
    }
}

fn verify_script_audio_commands(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    let (music, sound_effects, cries) = script_audio_catalog_ids(data);
    let cry_by_species: BTreeMap<String, String> = data
        .pokemon_cries
        .iter()
        .map(|(species_id, metadata)| (species_id.clone(), metadata.cry.clone()))
        .collect();

    for (map_name, module) in &data.maps {
        for command in &module.script_audio_commands {
            let subject = format!(
                "{map_name}:{}:{}",
                command.source_script, command.command_index
            );
            for issue in script_audio_command_issues(
                command,
                &music,
                &sound_effects,
                &cries,
                &data.pokemon,
                &cry_by_species,
            ) {
                match issue {
                    ScriptAudioCommandIssue::MissingMusicId => {
                        diagnostics.push(VerificationError::error(
                            "missing_script_music_id",
                            &subject,
                            "audio command is missing a music id",
                        ))
                    }
                    ScriptAudioCommandIssue::UnknownMusicId => {
                        let audio_id = command.audio_id.as_deref().unwrap_or_default();
                        diagnostics.push(VerificationError::error(
                            "unknown_script_music_id",
                            &subject,
                            format!("audio command references missing music '{audio_id}'"),
                        ));
                    }
                    ScriptAudioCommandIssue::MissingSoundEffectId => {
                        diagnostics.push(VerificationError::error(
                            "missing_script_sfx_id",
                            &subject,
                            "playsound command is missing a sound effect id",
                        ))
                    }
                    ScriptAudioCommandIssue::UnknownSoundEffectId => {
                        let audio_id = command.audio_id.as_deref().unwrap_or_default();
                        diagnostics.push(VerificationError::error(
                            "unknown_script_sfx_id",
                            &subject,
                            format!("playsound command references missing sfx '{audio_id}'"),
                        ));
                    }
                    ScriptAudioCommandIssue::MissingCrySpecies => {
                        diagnostics.push(VerificationError::error(
                            "missing_script_cry_id",
                            &subject,
                            "cry command is missing a species id",
                        ));
                    }
                    ScriptAudioCommandIssue::UnknownCrySpecies => {
                        let species_id = command.audio_id.as_deref().unwrap_or_default();
                        diagnostics.push(VerificationError::error(
                            "unknown_script_cry_species",
                            &subject,
                            format!("cry command references missing species '{species_id}'"),
                        ));
                    }
                    ScriptAudioCommandIssue::MissingCryMetadata => {
                        let species_id = command.audio_id.as_deref().unwrap_or_default();
                        diagnostics.push(VerificationError::error(
                            "missing_script_cry_metadata",
                            &subject,
                            format!(
                                "cry command references species '{species_id}' without cry metadata"
                            ),
                        ));
                    }
                    ScriptAudioCommandIssue::UnknownCryAsset => {
                        let species_id = command.audio_id.as_deref().unwrap_or_default();
                        let cry_id = cry_by_species.get(species_id).map_or("", String::as_str);
                        diagnostics.push(VerificationError::error(
                            "unknown_script_cry_audio",
                            &subject,
                            format!(
                                "cry command references missing cry audio '{cry_id}' for species '{species_id}'"
                            ),
                        ));
                    }
                    ScriptAudioCommandIssue::MissingMusicFadeFrames => {
                        diagnostics.push(VerificationError::error(
                            "missing_script_music_fade_frames",
                            &subject,
                            "musicfadeout command is missing fade frames",
                        ));
                    }
                    ScriptAudioCommandIssue::UnexpectedAudioId => {
                        diagnostics.push(VerificationError::error(
                            "unexpected_script_audio_id",
                            &subject,
                            "waitsfx command must not carry an audio id",
                        ));
                    }
                    ScriptAudioCommandIssue::UnexpectedFadeFrames => {
                        diagnostics.push(VerificationError::error(
                            "unexpected_script_audio_fade_frames",
                            &subject,
                            format!("{} command must not carry fade frames", command.command),
                        ));
                    }
                    ScriptAudioCommandIssue::UnknownCommand => {
                        diagnostics.push(VerificationError::error(
                            "unknown_script_audio_command",
                            &subject,
                            format!("unknown audio command '{}'", command.command),
                        ));
                    }
                }
            }
        }
    }
}

fn verify_script_block_changes(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    for (map_name, module) in &data.maps {
        for change in &module.script_block_changes {
            let subject = format!(
                "{map_name}:{}:{}",
                change.source_script, change.command_index
            );
            let metatile_x = change.x / CHANGE_BLOCK_COORD_STRIDE;
            let metatile_y = change.y / CHANGE_BLOCK_COORD_STRIDE;
            if metatile_x >= module.attributes.width || metatile_y >= module.attributes.height {
                diagnostics.push(VerificationError::error(
                    "script_block_change_out_of_bounds",
                    &subject,
                    format!(
                        "changeblock targets ({}, {}) outside {} dimensions {}x{}",
                        change.x,
                        change.y,
                        map_name,
                        module.attributes.width,
                        module.attributes.height
                    ),
                ));
            }
            let expected_blocks =
                module.attributes.width as usize * module.attributes.height as usize;
            if !module.blocks.is_empty() && module.blocks.len() != expected_blocks {
                diagnostics.push(VerificationError::error(
                    "script_block_map_size_mismatch",
                    &subject,
                    format!(
                        "{} has {} blocks but attributes require {}",
                        map_name,
                        module.blocks.len(),
                        expected_blocks
                    ),
                ));
            }
        }
    }
}

fn verify_script_object_commands(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    for (map_name, module) in &data.maps {
        for object in &module.objects {
            if object_event_initial_facing(&object.spritemovedata).is_none() {
                let subject = format!(
                    "{map_name}:{}",
                    object
                        .object_identifier
                        .as_deref()
                        .unwrap_or("<unidentified>")
                );
                diagnostics.push(VerificationError::error(
                    "unknown_object_movement_data",
                    &subject,
                    format!(
                        "object event uses unknown spritemovedata '{}'",
                        object.spritemovedata
                    ),
                ));
            }
        }
        let object_ids: BTreeSet<&str> = module
            .objects
            .iter()
            .filter_map(|object| object.object_identifier.as_deref())
            .collect();
        let movements: BTreeSet<(&str, Option<&str>)> = module
            .script_movements
            .iter()
            .map(|movement| (movement.label.as_str(), movement.source_script.as_deref()))
            .collect();
        for command in &module.script_object_commands {
            let subject = format!(
                "{map_name}:{}:{}",
                command.source_script, command.command_index
            );
            if SCRIPT_OBJECT_NO_PAYLOAD_COMMANDS.contains(&command.command.as_str()) {
            } else if SCRIPT_OBJECT_VISIBILITY_COMMANDS.contains(&command.command.as_str()) {
                let Some(object_id) = command.object_id.as_deref() else {
                    diagnostics.push(VerificationError::error(
                        "script_object_missing_id",
                        &subject,
                        format!("{} command is missing an object id", command.command),
                    ));
                    continue;
                };
                if object_id == "LAST_TALKED" || object_id == "PLAYER" {
                    continue;
                }
                let Some(object) = module
                    .objects
                    .iter()
                    .find(|object| object.object_identifier.as_deref() == Some(object_id))
                else {
                    diagnostics.push(VerificationError::error(
                        "unknown_script_object_id",
                        &subject,
                        format!(
                            "{} references missing object id '{object_id}'",
                            command.command
                        ),
                    ));
                    continue;
                };
                if object.event_flag != "-1" && !is_hideable_object_event_flag(&object.event_flag) {
                    diagnostics.push(VerificationError::error(
                        "script_object_unhideable",
                        &subject,
                        format!(
                            "{} references object '{}' with unhideable event flag '{}'",
                            command.command, object_id, object.event_flag
                        ),
                    ));
                }
            } else if SCRIPT_OBJECT_COORDINATE_COMMANDS.contains(&command.command.as_str()) {
                verify_required_object_id(diagnostics, &subject, command, &object_ids, false);
                if command.x.is_none() || command.y.is_none() {
                    diagnostics.push(VerificationError::error(
                        "script_object_missing_coordinates",
                        &subject,
                        "moveobject command is missing x/y coordinates",
                    ));
                }
            } else if SCRIPT_OBJECT_DIRECTION_COMMANDS.contains(&command.command.as_str())
                || SCRIPT_OBJECT_TARGET_COMMANDS.contains(&command.command.as_str())
            {
                verify_required_object_id(diagnostics, &subject, command, &object_ids, true);
                if SCRIPT_OBJECT_DIRECTION_COMMANDS.contains(&command.command.as_str()) {
                    verify_script_direction(diagnostics, &subject, command.direction.as_deref());
                }
                if SCRIPT_OBJECT_TARGET_COMMANDS.contains(&command.command.as_str()) {
                    verify_required_target_object_id(
                        diagnostics,
                        &subject,
                        command,
                        &object_ids,
                        true,
                    );
                }
            } else if SCRIPT_OBJECT_MOVEMENT_COMMANDS.contains(&command.command.as_str()) {
                if SCRIPT_OBJECT_DIRECT_MOVEMENT_COMMANDS.contains(&command.command.as_str()) {
                    verify_required_object_id(diagnostics, &subject, command, &object_ids, true);
                }
                let Some(movement) = command.movement.as_deref() else {
                    diagnostics.push(VerificationError::error(
                        "script_object_missing_movement",
                        &subject,
                        format!("{} command is missing a movement label", command.command),
                    ));
                    continue;
                };
                let movement_source = script_label_parent(&command.source_script);
                if !movements.contains(&(movement, None))
                    && !movements.contains(&(movement, Some(movement_source)))
                {
                    diagnostics.push(VerificationError::error(
                        "unknown_script_movement",
                        &subject,
                        format!(
                            "{} references missing movement '{movement}'",
                            command.command
                        ),
                    ));
                }
            } else if SCRIPT_OBJECT_EMOTE_COMMANDS.contains(&command.command.as_str()) {
                verify_required_object_id(diagnostics, &subject, command, &object_ids, true);
                if command.emote.is_none() || command.duration.is_none() {
                    diagnostics.push(VerificationError::error(
                        "script_object_missing_emote",
                        &subject,
                        "showemote command is missing emote/duration fields",
                    ));
                }
            } else if !is_known_script_object_command(&command.command) {
                diagnostics.push(VerificationError::error(
                    "unknown_script_object_command",
                    &subject,
                    format!("unknown object command '{}'", command.command),
                ));
            }
        }
    }
}

fn verify_script_movements(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    for (map_name, module) in &data.maps {
        for movement in &module.script_movements {
            for step in &movement.steps {
                let subject = format!("{map_name}:{}:{}", movement.label, step.index);
                let command = step.command.as_str();
                match command {
                    "step_end" => {
                        if step.direction.is_some() {
                            diagnostics.push(VerificationError::error(
                                "script_movement_unexpected_direction",
                                &subject,
                                "step_end must not carry a direction",
                            ));
                        }
                    }
                    command
                        if SCRIPT_MOVEMENT_NO_ARG_COMMANDS.contains(&command)
                            || SCRIPT_MOVEMENT_OPTIONAL_DURATION_COMMANDS.contains(&command) =>
                    {
                        if step.direction.is_some() {
                            diagnostics.push(VerificationError::error(
                                "script_movement_unexpected_direction",
                                &subject,
                                format!("{} must not carry a direction", step.command),
                            ));
                        }
                    }
                    command if SCRIPT_MOVEMENT_DIRECTION_COMMANDS.contains(&command) => {
                        verify_script_direction(diagnostics, &subject, step.direction.as_deref());
                    }
                    command => diagnostics.push(VerificationError::error(
                        "unsupported_script_movement_command",
                        &subject,
                        format!("unsupported movement command '{command}'"),
                    )),
                }
            }
        }
    }
}

fn verify_script_map_commands(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    let script_warp_targets = script_warp_target_constants(data);
    for (map_name, module) in &data.maps {
        for command in &module.script_map_commands {
            let subject = format!(
                "{map_name}:{}:{}",
                command.source_script, command.command_index
            );
            if SCRIPT_MAP_WARP_COMMANDS.contains(&command.command.as_str()) {
                verify_script_warp_destination(
                    diagnostics,
                    &subject,
                    command,
                    &script_warp_targets,
                );
                if command.facing.is_some() {
                    diagnostics.push(VerificationError::error(
                        "unexpected_script_warp_facing",
                        &subject,
                        "warp command must not carry a facing direction",
                    ));
                }
                if command.map_setup.is_some() {
                    diagnostics.push(VerificationError::error(
                        "unexpected_script_map_setup",
                        &subject,
                        "warp command must not carry a map setup",
                    ));
                }
            } else if SCRIPT_MAP_FACING_WARP_COMMANDS.contains(&command.command.as_str()) {
                verify_script_warp_destination(
                    diagnostics,
                    &subject,
                    command,
                    &script_warp_targets,
                );
                match command.facing.as_deref() {
                    Some(facing) => {
                        if parse_script_warp_facing(facing).is_err() {
                            diagnostics.push(VerificationError::error(
                                "unknown_script_warp_facing",
                                &subject,
                                format!("warpfacing references unknown direction '{facing}'"),
                            ));
                        }
                    }
                    None => diagnostics.push(VerificationError::error(
                        "missing_script_warp_facing",
                        &subject,
                        "warpfacing command is missing a facing direction",
                    )),
                }
                if command.map_setup.is_some() {
                    diagnostics.push(VerificationError::error(
                        "unexpected_script_map_setup",
                        &subject,
                        "warpfacing command must not carry a map setup",
                    ));
                }
            } else if SCRIPT_MAP_NO_PAYLOAD_COMMANDS.contains(&command.command.as_str()) {
                verify_no_script_warp_destination(diagnostics, &subject, command);
                if command.facing.is_some() {
                    diagnostics.push(VerificationError::error(
                        "unexpected_script_warp_facing",
                        &subject,
                        format!(
                            "{} command must not carry a facing direction",
                            command.command
                        ),
                    ));
                }
                if command.map_setup.is_some() {
                    diagnostics.push(VerificationError::error(
                        "unexpected_script_map_setup",
                        &subject,
                        format!("{} command must not carry a map setup", command.command),
                    ));
                }
            } else if SCRIPT_MAP_REANCHOR_COMMANDS.contains(&command.command.as_str()) {
                verify_no_script_warp_destination(diagnostics, &subject, command);
                if command.facing.is_some() {
                    diagnostics.push(VerificationError::error(
                        "unexpected_script_warp_facing",
                        &subject,
                        "reanchormap command must not carry a facing direction",
                    ));
                }
            } else if SCRIPT_MAP_NEW_LOAD_COMMANDS.contains(&command.command.as_str()) {
                verify_no_script_warp_destination(diagnostics, &subject, command);
                if command.facing.is_some() {
                    diagnostics.push(VerificationError::error(
                        "unexpected_script_warp_facing",
                        &subject,
                        "newloadmap command must not carry a facing direction",
                    ));
                }
                if command.map_setup.is_none() {
                    diagnostics.push(VerificationError::error(
                        "missing_script_map_setup",
                        &subject,
                        "newloadmap command is missing a map setup",
                    ));
                }
            } else if !is_known_script_map_command(&command.command) {
                diagnostics.push(VerificationError::error(
                    "unknown_script_map_command",
                    &subject,
                    format!("unknown script map command '{}'", command.command),
                ));
            }
        }
    }
}

fn script_warp_target_constants(data: &GameDataSet) -> BTreeSet<String> {
    data.maps
        .iter()
        .flat_map(|(map_name, module)| {
            [
                Some(map_name.clone()),
                module.attributes.map_constant.clone(),
            ]
            .into_iter()
            .flatten()
        })
        .collect()
}

fn verify_script_warp_destination(
    diagnostics: &mut Vec<VerificationError>,
    subject: &str,
    command: &ScriptMapCommand,
    map_names: &BTreeSet<String>,
) {
    match command.target_map.as_deref() {
        Some("NONE")
            if command.command == "warp" && command.x == Some(0) && command.y == Some(0) => {}
        Some("NONE") => diagnostics.push(VerificationError::error(
            "malformed_script_no_warp_sentinel",
            subject,
            "NONE is only valid for the exact warp NONE, 0, 0 sentinel",
        )),
        Some(target_map) if map_names.contains(target_map) => {}
        Some(target_map) => diagnostics.push(VerificationError::error(
            "unknown_script_warp_map",
            subject,
            format!(
                "{} command references missing map '{target_map}'",
                command.command
            ),
        )),
        None => diagnostics.push(VerificationError::error(
            "missing_script_warp_map",
            subject,
            format!("{} command is missing a target map", command.command),
        )),
    }
    if command.x.is_none() || command.y.is_none() {
        diagnostics.push(VerificationError::error(
            "missing_script_warp_coordinates",
            subject,
            format!("{} command is missing x/y coordinates", command.command),
        ));
    }
}

fn verify_no_script_warp_destination(
    diagnostics: &mut Vec<VerificationError>,
    subject: &str,
    command: &ScriptMapCommand,
) {
    if command.target_map.is_some() || command.x.is_some() || command.y.is_some() {
        diagnostics.push(VerificationError::error(
            "unexpected_script_warp_destination",
            subject,
            format!(
                "{} command must not carry target map or coordinates",
                command.command
            ),
        ));
    }
}

fn verify_script_text_commands(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    for (map_name, module) in &data.maps {
        let text_labels: BTreeSet<&str> = module
            .scripts
            .iter()
            .filter_map(|(label, payload)| is_text_script(payload).then_some(label.as_str()))
            .collect();
        for command in &module.script_text_commands {
            let subject = format!(
                "{map_name}:{}:{}",
                command.source_script, command.command_index
            );
            if SCRIPT_TEXT_NO_LABEL_COMMANDS.contains(&command.command.as_str()) {
                if command.text_label.is_some() {
                    diagnostics.push(VerificationError::error(
                        "unexpected_script_text_label",
                        &subject,
                        format!("{} command must not carry a text label", command.command),
                    ));
                }
            } else if SCRIPT_TEXT_LABEL_COMMANDS.contains(&command.command.as_str()) {
                match command.text_label.as_deref() {
                    Some(label) if text_labels.contains(label) => {}
                    Some(label)
                        if resolve_script_target_label(
                            &module.scripts,
                            &command.source_script,
                            label,
                        )
                        .is_some_and(|resolved| text_labels.contains(resolved.as_str())) => {}
                    Some(label) => diagnostics.push(VerificationError::error(
                        "unknown_script_text_label",
                        &subject,
                        format!(
                            "{} command references missing text label '{label}'",
                            command.command
                        ),
                    )),
                    None => diagnostics.push(VerificationError::error(
                        "missing_script_text_label",
                        &subject,
                        format!("{} command is missing a text label", command.command),
                    )),
                }
            } else if !is_known_script_text_command(&command.command) {
                diagnostics.push(VerificationError::error(
                    "unknown_script_text_command",
                    &subject,
                    format!("unknown text command '{}'", command.command),
                ));
            }
        }
    }
}

fn verify_script_text_bodies(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    let expected_arg_counts = text_body_command_arg_counts();
    for (map_name, module) in &data.maps {
        for (label, body) in &module.script_text_bodies {
            if body.label != *label {
                diagnostics.push(VerificationError::error(
                    "script_text_body_label_mismatch",
                    &format!("{map_name}:{label}"),
                    format!(
                        "text body key '{}' does not match record label '{}'",
                        label, body.label
                    ),
                ));
            }
            for command in &body.commands {
                let subject = format!("{map_name}:{label}:{}", command.command_index);
                let Some(expected) = expected_arg_counts.get(command.command.as_str()) else {
                    diagnostics.push(VerificationError::error(
                        "unknown_script_text_body_command",
                        &subject,
                        format!("unknown text body command '{}'", command.command),
                    ));
                    continue;
                };
                if command.args.len() != *expected {
                    diagnostics.push(VerificationError::error(
                        "malformed_script_text_body_command",
                        &subject,
                        format!(
                            "{} expects {} args but found {}",
                            command.command,
                            expected,
                            command.args.len()
                        ),
                    ));
                }
            }
        }
    }
}

fn verify_script_menu_definitions(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    let expected_arg_counts = menu_definition_command_arg_counts();
    for (map_name, module) in &data.maps {
        for (label, menu) in &module.script_menu_definitions {
            if menu.label != *label {
                diagnostics.push(VerificationError::error(
                    "script_menu_label_mismatch",
                    &format!("{map_name}:{label}"),
                    format!(
                        "menu definition key '{}' does not match record label '{}'",
                        label, menu.label
                    ),
                ));
            }
            for command in &menu.commands {
                let subject = format!("{map_name}:{label}:{}", command.command_index);
                let Some(expected) = expected_arg_counts.get(command.command.as_str()) else {
                    diagnostics.push(VerificationError::error(
                        "unknown_script_menu_command",
                        &subject,
                        format!("unknown menu definition command '{}'", command.command),
                    ));
                    continue;
                };
                if !expected.contains(&command.args.len()) {
                    diagnostics.push(VerificationError::error(
                        "malformed_script_menu_command",
                        &subject,
                        format!(
                            "{} expects one of {:?} args but found {}",
                            command.command,
                            expected,
                            command.args.len()
                        ),
                    ));
                }
            }
        }
    }
}

fn verify_script_variable_commands(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    for (map_name, module) in &data.maps {
        for command in &module.script_variable_commands {
            let subject = format!(
                "{map_name}:{}:{}",
                command.source_script, command.command_index
            );
            if let Err(error) = validate_script_variable_command(command) {
                diagnostics.push(VerificationError::error(
                    "invalid_script_variable_command",
                    &subject,
                    error.to_string(),
                ));
            }
        }
    }
}

fn verify_script_control_commands(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    for (map_name, module) in &data.maps {
        let script_labels: BTreeSet<&str> = module.scripts.keys().map(String::as_str).collect();
        for command in &module.script_control_commands {
            let subject = format!(
                "{map_name}:{}:{}",
                command.source_script, command.command_index
            );
            if let Err(error) = validate_script_control_command(command) {
                diagnostics.push(VerificationError::error(
                    "invalid_script_control_command",
                    &subject,
                    error.to_string(),
                ));
                continue;
            }
            if command.command != "jumpstd" {
                let Some(target) = command.resolved_target_script.as_deref() else {
                    continue;
                };
                if !script_labels.contains(target) {
                    diagnostics.push(VerificationError::error(
                        "unknown_script_control_target",
                        &subject,
                        format!(
                            "{} command resolves to missing script label '{target}'",
                            command.command
                        ),
                    ));
                }
            }
        }
    }
}

fn verify_script_field_pickups(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    for (map_name, module) in &data.maps {
        for pickup in &module.script_field_pickups {
            let subject = format!(
                "{map_name}:{}:{}",
                pickup.source_script, pickup.command_index
            );
            for issue in script_field_pickup_issues(pickup, &data.items, &data.fruit_trees) {
                match issue {
                    ScriptFieldPickupIssue::MissingItem => {
                        diagnostics.push(VerificationError::error(
                            "script_field_pickup_missing_item",
                            &subject,
                            format!("{} pickup is missing item_id", pickup.command),
                        ));
                    }
                    ScriptFieldPickupIssue::UnknownItem => {
                        let item_id = pickup.item_id.as_deref().unwrap_or_default();
                        diagnostics.push(VerificationError::error(
                            "unknown_script_field_pickup_item",
                            &subject,
                            format!(
                                "{} pickup references missing item '{item_id}'",
                                pickup.command
                            ),
                        ));
                    }
                    ScriptFieldPickupIssue::InvalidQuantity => {
                        diagnostics.push(VerificationError::error(
                            "script_field_pickup_invalid_quantity",
                            &subject,
                            format!("{} pickup has zero quantity", pickup.command),
                        ));
                    }
                    ScriptFieldPickupIssue::MissingEvent => {
                        diagnostics.push(VerificationError::error(
                            "script_field_pickup_missing_event",
                            &subject,
                            format!("{} pickup is missing event_flag", pickup.command),
                        ));
                    }
                    ScriptFieldPickupIssue::InvalidCollectibleFlag => {
                        let event_flag = pickup.event_flag.as_deref().unwrap_or_default();
                        diagnostics.push(VerificationError::error(
                            "script_field_pickup_uncollectible_event",
                            &subject,
                            format!(
                                "{} pickup uses uncollectible event flag '{event_flag}'",
                                pickup.command
                            ),
                        ));
                    }
                    ScriptFieldPickupIssue::MissingFruitTree => {
                        diagnostics.push(VerificationError::error(
                            "script_field_pickup_missing_fruit_tree",
                            &subject,
                            "fruittree pickup is missing fruit_tree_id",
                        ));
                    }
                    ScriptFieldPickupIssue::EmptyFruitTree => {
                        diagnostics.push(VerificationError::error(
                            "script_field_pickup_empty_fruit_tree",
                            &subject,
                            "fruittree pickup has an empty fruit_tree_id",
                        ));
                    }
                    ScriptFieldPickupIssue::UnknownFruitTree => {
                        let fruit_tree_id = pickup.fruit_tree_id.as_deref().unwrap_or_default();
                        diagnostics.push(VerificationError::error(
                            "unknown_script_field_fruit_tree",
                            &subject,
                            format!("fruittree references missing tree '{fruit_tree_id}'"),
                        ));
                    }
                    ScriptFieldPickupIssue::MalformedFruitTree => {
                        diagnostics.push(VerificationError::error(
                            "script_field_pickup_malformed_fruit_tree",
                            &subject,
                            "fruittree pickup must not inline item_id or event_flag",
                        ));
                    }
                    ScriptFieldPickupIssue::UnknownCommand => {
                        diagnostics.push(VerificationError::error(
                            "unknown_script_field_pickup_command",
                            &subject,
                            format!("unknown field pickup command '{}'", pickup.command),
                        ));
                    }
                }
            }
        }
    }
}

fn verify_phone_contacts(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    let map_constants = map_constants(data);
    for (contact_id, record) in &data.phone_contacts.0 {
        let subject = format!("phone_contacts:{contact_id}");
        if contact_id.trim().is_empty() {
            diagnostics.push(VerificationError::error(
                "empty_phone_contact_id",
                &subject,
                "phone contact catalog keys must be nonempty exact ids",
            ));
        }
        if &record.contact_id != contact_id {
            diagnostics.push(VerificationError::error(
                "phone_contact_id_mismatch",
                &subject,
                format!(
                    "phone contact key '{}' does not match record contactId '{}'",
                    contact_id, record.contact_id
                ),
            ));
        }
        if record.primary_label.trim().is_empty() {
            diagnostics.push(VerificationError::error(
                "empty_phone_contact_primary_label",
                &subject,
                "phone contact primaryLabel must be nonempty",
            ));
        }
        if record.lines.is_empty() || record.lines.iter().any(|line| line.trim().is_empty()) {
            diagnostics.push(VerificationError::error(
                "invalid_phone_contact_lines",
                &subject,
                "phone contact display lines must be nonempty",
            ));
        } else if let Some(first_line) = record.lines.first() {
            let expected_primary = first_line.trim_end_matches(':').trim();
            if expected_primary != record.primary_label {
                diagnostics.push(VerificationError::error(
                    "phone_contact_primary_label_mismatch",
                    &subject,
                    format!(
                        "phone contact primaryLabel '{}' does not match first display line '{}'",
                        record.primary_label, first_line
                    ),
                ));
            }
        }
        if let Some(map_constant) = record.map_constant.as_deref() {
            if map_constant.trim().is_empty() {
                diagnostics.push(VerificationError::error(
                    "empty_phone_contact_map",
                    &subject,
                    "phone contact mapConstant must be nonempty when present",
                ));
            } else if !map_constants.contains_key(map_constant) {
                diagnostics.push(VerificationError::error(
                    "unknown_phone_contact_map",
                    &subject,
                    format!("phone contact references missing map constant '{map_constant}'"),
                ));
            }
        }
    }
    for contact_id in &data.permanent_phone_numbers {
        if !data.phone_contacts.0.contains_key(contact_id) {
            diagnostics.push(VerificationError::error(
                "unknown_permanent_phone_contact",
                contact_id,
                format!("permanent phone number references unknown contact '{contact_id}'"),
            ));
        }
    }
}

fn verify_required_object_sections(
    subject: &str,
    value: &str,
    sections: &[&str],
    diagnostics: &mut Vec<VerificationError>,
) {
    if value.trim().is_empty() {
        return;
    }
    let value: Value = match serde_json::from_str(value) {
        Ok(value) => value,
        Err(error) => {
            diagnostics.push(VerificationError::error(
                "invalid_runtime_bundle",
                subject,
                format!("runtime bundle payload is not valid JSON: {error}"),
            ));
            return;
        }
    };
    let Some(object) = value.as_object() else {
        diagnostics.push(VerificationError::error(
            "invalid_runtime_bundle",
            subject,
            "runtime bundle payload must be an object",
        ));
        return;
    };
    for section in sections {
        match object.get(*section).and_then(Value::as_object) {
            Some(section_object) if !section_object.is_empty() => {}
            _ => diagnostics.push(VerificationError::error(
                "missing_runtime_bundle_section",
                subject,
                format!("runtime bundle is missing non-empty section '{section}'"),
            )),
        }
    }
}

fn verify_runtime_pack_data(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    if data.pokemon.is_empty() {
        diagnostics.push(VerificationError::error(
            "missing_runtime_pokemon",
            "pokemon",
            "runtime pack must include Pokemon species data",
        ));
    }
    if data.moves.is_empty() {
        diagnostics.push(VerificationError::error(
            "missing_runtime_moves",
            "moves",
            "runtime pack must include move data",
        ));
    }
    if data.maps.is_empty() {
        diagnostics.push(VerificationError::error(
            "missing_runtime_maps",
            "maps",
            "runtime pack must include map modules",
        ));
    }

    let map_constants = map_constants(data);
    for (key, metadata) in &data.runtime_map_metadata {
        if key != &metadata.constant {
            diagnostics.push(VerificationError::error(
                "runtime_map_metadata_constant_mismatch",
                key,
                format!(
                    "runtime map metadata key '{}' does not match record constant '{}'",
                    key, metadata.constant
                ),
            ));
        }
        match map_constants.get(&metadata.constant) {
            Some(map_name) if map_name == &metadata.name => {}
            Some(map_name) => diagnostics.push(VerificationError::error(
                "runtime_map_metadata_name_mismatch",
                key,
                format!(
                    "runtime map metadata '{}' names '{}' but map attributes use '{}'",
                    metadata.constant, metadata.name, map_name
                ),
            )),
            None => diagnostics.push(VerificationError::error(
                "unknown_runtime_map_metadata_constant",
                key,
                format!(
                    "runtime map metadata references missing map constant '{}'",
                    metadata.constant
                ),
            )),
        }
        if metadata.group_name.trim().is_empty() || metadata.environment.trim().is_empty() {
            diagnostics.push(VerificationError::error(
                "invalid_runtime_map_metadata",
                key,
                "runtime map metadata groupName and environment must be non-empty",
            ));
        }
    }

    for (key, spawn) in &data.runtime_spawn_points {
        if key.parse::<u16>().ok() != Some(spawn.identifier) {
            diagnostics.push(VerificationError::error(
                "runtime_spawn_point_identifier_mismatch",
                key,
                format!(
                    "runtime spawn point key '{}' does not match identifier {}",
                    key, spawn.identifier
                ),
            ));
        }
        if spawn.map_constant != "N_A" {
            match data.runtime_map_metadata.get(&spawn.map_constant) {
                Some(metadata) if metadata.name == spawn.map_name => {}
                Some(metadata) => diagnostics.push(VerificationError::error(
                    "runtime_spawn_point_map_mismatch",
                    key,
                    format!(
                        "runtime spawn point targets '{}' but metadata names '{}'",
                        spawn.map_name, metadata.name
                    ),
                )),
                None => diagnostics.push(VerificationError::error(
                    "unknown_runtime_spawn_point_map",
                    key,
                    format!(
                        "runtime spawn point references missing map constant '{}'",
                        spawn.map_constant
                    ),
                )),
            }
        }
        if spawn.group_name.trim().is_empty() {
            diagnostics.push(VerificationError::error(
                "invalid_runtime_spawn_point",
                key,
                "runtime spawn point groupName must be non-empty",
            ));
        }
    }

    for flag in data
        .initialize_events
        .event_flags
        .iter()
        .chain(data.initialize_events.engine_flags.iter())
    {
        if flag.trim().is_empty() {
            diagnostics.push(VerificationError::error(
                "invalid_initialize_event_flag",
                flag,
                "initialize event and engine flags must be non-empty",
            ));
        }
    }
    for (sprite, replacement) in &data.initialize_events.variable_sprites {
        if sprite.trim().is_empty() || replacement.trim().is_empty() {
            diagnostics.push(VerificationError::error(
                "invalid_initialize_event_sprite",
                sprite,
                "initialize event variable sprite keys and values must be non-empty",
            ));
        }
    }

    for key in data.story_event_script_constants.global.keys() {
        if key.trim().is_empty() {
            diagnostics.push(VerificationError::error(
                "invalid_story_event_script_constant",
                key,
                "global story event script constant keys must be non-empty",
            ));
        }
    }
    for (map_name, constants) in &data.story_event_script_constants.maps {
        if map_name.trim().is_empty() {
            diagnostics.push(VerificationError::error(
                "invalid_story_event_script_constant_map",
                map_name,
                "story event script constant map keys must be non-empty",
            ));
        }
        for key in constants.keys() {
            if key.trim().is_empty() {
                diagnostics.push(VerificationError::error(
                    "invalid_story_event_script_constant",
                    &format!("{map_name}:{key}"),
                    "map story event script constant keys must be non-empty",
                ));
            }
        }
    }

    for (label, text) in &data.asm_text {
        if label.trim().is_empty() || text.trim().is_empty() {
            diagnostics.push(VerificationError::error(
                "invalid_asm_text",
                label,
                "ASM text keys and values must be non-empty",
            ));
        }
    }
    if !data.move_names.is_empty() && data.move_names.len() != data.moves.len() {
        diagnostics.push(VerificationError::error(
            "move_names_count_mismatch",
            "move_names",
            format!(
                "move_names contains {} entries but moves contains {}",
                data.move_names.len(),
                data.moves.len()
            ),
        ));
    }
    for (index, move_name) in data.move_names.iter().enumerate() {
        if move_name.trim().is_empty() {
            diagnostics.push(VerificationError::error(
                "invalid_move_name",
                &index.to_string(),
                "move name must be non-empty",
            ));
        }
    }
    for (label, commands) in &data.battle_animations {
        if label.trim().is_empty() || commands.is_empty() {
            diagnostics.push(VerificationError::error(
                "invalid_battle_animation",
                label,
                "battle animation labels and command lists must be non-empty",
            ));
        }
    }
    for (index, label) in data.battle_animation_table.iter().enumerate() {
        if label.trim().is_empty() {
            diagnostics.push(VerificationError::error(
                "invalid_battle_animation_table_entry",
                &index.to_string(),
                "battle animation table labels must be non-empty",
            ));
        }
    }
    if !data.battle_animation_table.is_empty()
        && data.battle_animation_table.len() != data.moves.len() + 1
    {
        diagnostics.push(VerificationError::error(
            "battle_animation_table_count_mismatch",
            "battle_animation_table",
            format!(
                "battle animation table contains {} entries but moves plus dummy contains {}",
                data.battle_animation_table.len(),
                data.moves.len() + 1
            ),
        ));
    }
    verify_required_object_sections(
        "battle_anim_bundle",
        &data.battle_anim_bundle,
        &[
            "objects",
            "framesets",
            "oam_sets",
            "gfx_table",
            "gfx_sources",
        ],
        diagnostics,
    );
    verify_required_object_sections(
        "sprite_anim_bundle",
        &data.sprite_anim_bundle,
        &["oam_sets", "framesets", "objects"],
        diagnostics,
    );
    for (sprite, palette) in &data.sprite_palette_defaults {
        if sprite.trim().is_empty() || *palette < 0 {
            diagnostics.push(VerificationError::error(
                "invalid_sprite_palette_default",
                sprite,
                "sprite palette defaults require non-empty sprite ids and non-negative palettes",
            ));
        }
    }
    for (name, palettes) in &data.pokegear_town_map_palette_map {
        if name.trim().is_empty()
            || palettes.is_empty()
            || palettes.iter().any(|entry| entry.trim().is_empty())
        {
            diagnostics.push(VerificationError::error(
                "invalid_pokegear_palette_map",
                name,
                "Pokegear palette map entries must be non-empty",
            ));
        }
    }
    let landmark_constants: BTreeSet<&str> = data
        .pokegear_landmarks
        .landmarks
        .iter()
        .map(|landmark| landmark.constant.as_str())
        .collect();
    for landmark in &data.pokegear_landmarks.landmarks {
        if landmark.constant.trim().is_empty()
            || landmark.label.trim().is_empty()
            || landmark.name.trim().is_empty()
            || landmark.region.trim().is_empty()
        {
            diagnostics.push(VerificationError::error(
                "invalid_pokegear_landmark",
                &landmark.constant,
                "Pokegear landmarks require non-empty constant, label, name, and region fields",
            ));
        }
        if !landmark.constant.starts_with("LANDMARK_") {
            diagnostics.push(VerificationError::error(
                "invalid_pokegear_landmark_constant",
                &landmark.constant,
                "Pokegear landmark constants must use exact LANDMARK_* ids",
            ));
        }
    }
    for (map_name, landmark_constant) in &data.pokegear_landmarks.map_to_landmark {
        if !data.maps.contains_key(map_name) {
            diagnostics.push(VerificationError::error(
                "unknown_pokegear_landmark_map",
                map_name,
                "Pokegear map-to-landmark entry references a map that is not loaded",
            ));
        }
        if !landmark_constants.contains(landmark_constant.as_str()) {
            diagnostics.push(VerificationError::error(
                "unknown_pokegear_landmark_constant",
                map_name,
                format!(
                    "Pokegear map-to-landmark entry references missing landmark constant '{landmark_constant}'"
                ),
            ));
        }
    }
    let (_, _, cry_audio) = script_audio_catalog_ids(data);
    for (species_id, cry) in &data.pokemon_cries {
        if species_id.trim().is_empty() || cry.cry.trim().is_empty() {
            diagnostics.push(VerificationError::error(
                "invalid_pokemon_cry_metadata",
                species_id,
                "Pokemon cry metadata requires non-empty exact keys and non-empty cry ids",
            ));
        } else if species_id.parse::<u16>().is_err() && !data.pokemon.contains_key(species_id) {
            diagnostics.push(VerificationError::error(
                "unknown_pokemon_cry_species",
                species_id,
                "Pokemon cry metadata species keys must match loaded Pokemon ids exactly",
            ));
        }
    }
    for species_id in data.pokemon.keys() {
        let Some(cry) = data.pokemon_cries.get(species_id) else {
            diagnostics.push(VerificationError::error(
                "missing_species_cry_metadata",
                species_id,
                "Pokemon species is missing explicit cry metadata",
            ));
            continue;
        };
        if !cry_audio.contains(&cry.cry) {
            diagnostics.push(VerificationError::error(
                "unknown_species_cry_audio",
                species_id,
                format!(
                    "Pokemon species references missing cry audio '{}' through cry metadata",
                    cry.cry
                ),
            ));
        }
    }

    for species_id in data
        .flee_mons
        .always
        .iter()
        .chain(data.flee_mons.often.iter())
        .chain(data.flee_mons.sometimes.iter())
    {
        if !data.pokemon.contains_key(species_id) {
            diagnostics.push(VerificationError::error(
                "unknown_flee_mon_species",
                species_id,
                format!("flee mon table references missing species '{species_id}'"),
            ));
        }
    }

    for (key, value) in &data.pc_strings {
        if key.trim().is_empty() || value.trim().is_empty() {
            diagnostics.push(VerificationError::error(
                "invalid_pc_string",
                key,
                "PC string keys and values must be non-empty",
            ));
        }
    }

    for (species_id, icon) in &data.menu_icons {
        if species_id != "EGG" && !data.pokemon.contains_key(species_id) {
            diagnostics.push(VerificationError::error(
                "unknown_menu_icon_species",
                species_id,
                format!("menu icon references missing species '{species_id}'"),
            ));
        }
        if icon.trim().is_empty() {
            diagnostics.push(VerificationError::error(
                "invalid_menu_icon",
                species_id,
                "menu icon id must be non-empty",
            ));
        }
    }
    for species_id in data.pokemon.keys() {
        if !data.menu_icons.contains_key(species_id) {
            diagnostics.push(VerificationError::error(
                "missing_species_menu_icon",
                species_id,
                "Pokemon species is missing an explicit menu icon entry",
            ));
        }
    }

    for (species_id, entry) in &data.pokedex_entries {
        if species_id != &entry.species {
            diagnostics.push(VerificationError::error(
                "pokedex_entry_species_mismatch",
                species_id,
                format!(
                    "pokedex entry key '{}' does not match record species '{}'",
                    species_id, entry.species
                ),
            ));
        }
        if !data.pokemon.contains_key(species_id) {
            diagnostics.push(VerificationError::error(
                "unknown_pokedex_entry_species",
                species_id,
                format!("pokedex entry references missing species '{species_id}'"),
            ));
        }
        if entry.classification.trim().is_empty() || entry.pages.is_empty() {
            diagnostics.push(VerificationError::error(
                "invalid_pokedex_entry",
                species_id,
                "pokedex entry classification and pages must be non-empty",
            ));
        }
    }
    for species_id in data.pokemon.keys() {
        if !data.pokedex_entries.contains_key(species_id) {
            diagnostics.push(VerificationError::error(
                "missing_species_pokedex_entry",
                species_id,
                "Pokemon species is missing an explicit Pokedex entry",
            ));
        }
    }

    for (species_id, program) in &data.pokemon_frontpic_anim {
        if !is_frontpic_animation_asset_key(species_id, data) {
            diagnostics.push(VerificationError::error(
                "unknown_frontpic_anim_species",
                species_id,
                format!("frontpic animation references missing species '{species_id}'"),
            ));
        }
        if program.commands.is_empty() {
            diagnostics.push(VerificationError::error(
                "empty_frontpic_anim",
                species_id,
                "frontpic animation program must contain at least one command",
            ));
        }
        for (index, command) in program.commands.iter().enumerate() {
            let subject = format!("{species_id}:{index}");
            if let Some(issue) = frontpic_anim_command_issue(command) {
                match issue {
                    FrontpicAnimCommandIssue::MissingFrame => {
                        diagnostics.push(VerificationError::error(
                            "malformed_frontpic_anim_command",
                            &subject,
                            "frame command requires frame and duration",
                        ));
                    }
                    FrontpicAnimCommandIssue::MissingSetRepeatCount => {
                        diagnostics.push(VerificationError::error(
                            "malformed_frontpic_anim_command",
                            &subject,
                            "setrepeat command requires count",
                        ));
                    }
                    FrontpicAnimCommandIssue::MissingDoRepeatTarget => {
                        diagnostics.push(VerificationError::error(
                            "malformed_frontpic_anim_command",
                            &subject,
                            "dorepeat command requires target",
                        ));
                    }
                    FrontpicAnimCommandIssue::UnknownCommand => {
                        diagnostics.push(VerificationError::error(
                            "unknown_frontpic_anim_command",
                            &subject,
                            format!("unknown frontpic animation command '{}'", command.kind),
                        ));
                    }
                }
            }
        }
    }
    for species_id in data.pokemon.keys() {
        if !data.pokemon_frontpic_anim.contains_key(species_id) {
            diagnostics.push(VerificationError::error(
                "missing_species_frontpic_anim",
                species_id,
                "Pokemon species is missing an explicit frontpic animation program",
            ));
        }
    }
}

fn is_frontpic_animation_asset_key(species_id: &str, data: &GameDataSet) -> bool {
    data.pokemon.contains_key(species_id)
        || species_id == "EGG"
        || species_id
            .strip_prefix("UNOWN_")
            .and_then(|suffix| {
                suffix
                    .as_bytes()
                    .first()
                    .copied()
                    .filter(|_| suffix.len() == 1)
            })
            .is_some_and(|byte| byte.is_ascii_uppercase())
}

fn verify_script_shop_commands(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    for (map_name, module) in &data.maps {
        for command in &module.script_shop_commands {
            let subject = format!(
                "{map_name}:{}:{}",
                command.source_script, command.command_index
            );
            if let Err(error) = validate_script_shop_command(&data.marts, command) {
                match error {
                    ShopError::UnknownMartType { mart_type } => {
                        diagnostics.push(VerificationError::error(
                            "unknown_script_shop_mart_type",
                            &subject,
                            format!("pokemart uses unknown mart type '{mart_type}'"),
                        ));
                    }
                    ShopError::InvalidZeroMart { mart_type } => {
                        diagnostics.push(VerificationError::error(
                            "script_shop_invalid_zero_mart",
                            &subject,
                            format!("pokemart type '{mart_type}' cannot use explicit mart id 0"),
                        ));
                    }
                    ShopError::UnknownMart { mart_id } => {
                        diagnostics.push(VerificationError::error(
                            "unknown_script_shop_mart",
                            &subject,
                            format!("pokemart references missing mart '{mart_id}'"),
                        ));
                    }
                    _ => {}
                }
            }
        }
    }
}

fn verify_script_phone_commands(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    for (map_name, module) in &data.maps {
        for command in &module.script_phone_commands {
            let subject = format!(
                "{map_name}:{}:{}",
                command.source_script, command.command_index
            );
            if let Err(error) = validate_script_phone_command(command, &data.phone_contacts) {
                match error {
                    ScriptPhoneError::UnknownCommand { command } => {
                        diagnostics.push(VerificationError::error(
                            "unknown_script_phone_command",
                            &subject,
                            format!("unknown phone command '{command}'"),
                        ));
                    }
                    ScriptPhoneError::UnknownContact {
                        command: phone_command,
                        contact_id,
                    } => {
                        diagnostics.push(VerificationError::error(
                            "unknown_script_phone_contact",
                            &subject,
                            format!(
                                "phone command '{phone_command}' references unknown contact '{contact_id}'"
                            ),
                        ));
                    }
                    ScriptPhoneError::EmptyContact {
                        command: phone_command,
                    } => {
                        diagnostics.push(VerificationError::error(
                            "unknown_script_phone_contact",
                            &subject,
                            format!(
                                "phone command '{phone_command}' references unknown contact '{}'",
                                command.contact_id
                            ),
                        ));
                    }
                    ScriptPhoneError::PaddedContact {
                        command: phone_command,
                        contact_id,
                    } => {
                        diagnostics.push(VerificationError::error(
                            "unknown_script_phone_contact",
                            &subject,
                            format!(
                                "phone command '{phone_command}' references unknown contact '{contact_id}'"
                            ),
                        ));
                    }
                    _ => {}
                }
            }
        }
    }
}

fn verify_happiness_data(data: &HappinessData, diagnostics: &mut Vec<VerificationError>) {
    if data.changes.is_empty() {
        diagnostics.push(VerificationError::error(
            "empty_happiness_changes",
            "happiness_data:changes",
            "happiness data requires at least one explicit change row",
        ));
    }
    let mut change_codes = BTreeSet::new();
    let mut code_names = BTreeSet::new();
    for entry in &data.changes {
        let subject = format!("happiness_data:changes:{}", entry.change_code);
        if entry.code.trim().is_empty() {
            diagnostics.push(VerificationError::error(
                "empty_happiness_change_code",
                &subject,
                "happiness change entries require exact nonempty code labels",
            ));
        }
        if !code_names.insert(entry.code.clone()) {
            diagnostics.push(VerificationError::error(
                "duplicate_happiness_change_code",
                &subject,
                format!("duplicate happiness change code '{}'", entry.code),
            ));
        }
        if !change_codes.insert(entry.change_code) {
            diagnostics.push(VerificationError::error(
                "duplicate_happiness_change_index",
                &subject,
                format!("duplicate happiness change index {}", entry.change_code),
            ));
        }
    }
    if data.services.is_empty() {
        diagnostics.push(VerificationError::error(
            "empty_happiness_services",
            "happiness_data:services",
            "happiness data requires explicit service probability tables",
        ));
    }
    let mut service_names = BTreeSet::new();
    for service in &data.services {
        let subject = format!("happiness_data:services:{}", service.routine);
        if service.routine.trim().is_empty() {
            diagnostics.push(VerificationError::error(
                "empty_happiness_service_routine",
                &subject,
                "happiness service routine ids must be exact nonempty labels",
            ));
        }
        if !service_names.insert(service.routine.clone()) {
            diagnostics.push(VerificationError::error(
                "duplicate_happiness_service",
                &subject,
                format!("duplicate happiness service table '{}'", service.routine),
            ));
        }
        if service.outcomes.is_empty() {
            diagnostics.push(VerificationError::error(
                "empty_happiness_service_outcomes",
                &subject,
                "happiness service tables require at least one outcome",
            ));
        }
        for outcome in &service.outcomes {
            if !change_codes.contains(&outcome.change_code) {
                diagnostics.push(VerificationError::error(
                    "unknown_happiness_service_change",
                    &subject,
                    format!(
                        "happiness service outcome references missing change code {}",
                        outcome.change_code
                    ),
                ));
            }
        }
    }
}

fn verify_encounter_slot_tables(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    if data.wild_encounters.is_empty() {
        return;
    }
    verify_encounter_slot_table(
        "grass",
        &data.encounter_slot_tables.grass,
        "encounter_slot_tables:grass",
        diagnostics,
    );
    verify_encounter_slot_table(
        "water",
        &data.encounter_slot_tables.water,
        "encounter_slot_tables:water",
        diagnostics,
    );
}

fn verify_encounter_slot_table(
    surface: &str,
    table: &[crystal_core::world::encounters::EncounterSlotChance],
    subject: &str,
    diagnostics: &mut Vec<VerificationError>,
) {
    if table.is_empty() {
        diagnostics.push(VerificationError::error(
            "missing_encounter_slot_table",
            subject,
            format!("encounter slot table for {surface} must be declared by the modpack"),
        ));
        return;
    }
    let mut previous_threshold = 0;
    let mut slots = BTreeSet::new();
    for entry in table {
        if entry.threshold == 0 || entry.threshold > 100 {
            diagnostics.push(VerificationError::error(
                "invalid_encounter_slot_threshold",
                subject,
                format!(
                    "encounter slot table for {surface} has threshold {} outside 1..=100",
                    entry.threshold
                ),
            ));
        }
        if entry.threshold < previous_threshold {
            diagnostics.push(VerificationError::error(
                "unordered_encounter_slot_threshold",
                subject,
                format!(
                    "encounter slot table for {surface} has threshold {} after {}",
                    entry.threshold, previous_threshold
                ),
            ));
        }
        previous_threshold = entry.threshold;
        if !slots.insert(entry.slot) {
            diagnostics.push(VerificationError::error(
                "duplicate_encounter_slot_index",
                subject,
                format!(
                    "encounter slot table for {surface} repeats slot {}",
                    entry.slot
                ),
            ));
        }
    }
    if previous_threshold != 100 {
        diagnostics.push(VerificationError::error(
            "incomplete_encounter_slot_table",
            subject,
            format!("encounter slot table for {surface} must end at threshold 100"),
        ));
    }
}

fn verify_encounter_music_modifiers(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    if data.wild_encounters.is_empty() {
        return;
    }
    let (music, _, _) = script_audio_catalog_ids(data);
    if data.encounter_music_modifiers.modifiers.is_empty() {
        diagnostics.push(VerificationError::error(
            "missing_encounter_music_modifiers",
            "encounter_music_modifiers",
            "encounter music modifiers must be declared by the modpack",
        ));
        return;
    }
    let mut seen = BTreeSet::new();
    for modifier in &data.encounter_music_modifiers.modifiers {
        let subject = format!("encounter_music_modifiers:{}", modifier.music_id);
        if modifier.music_id.is_empty() {
            diagnostics.push(VerificationError::error(
                "missing_encounter_music_modifier_id",
                &subject,
                "encounter music modifier is missing music_id",
            ));
        } else if !music.contains(&modifier.music_id) {
            diagnostics.push(VerificationError::error(
                "unknown_encounter_music_modifier_id",
                &subject,
                format!(
                    "encounter music modifier references missing music audio id '{}'",
                    modifier.music_id
                ),
            ));
        }
        if !seen.insert(modifier.music_id.as_str()) {
            diagnostics.push(VerificationError::error(
                "duplicate_encounter_music_modifier_id",
                &subject,
                format!(
                    "encounter music modifiers repeat music id '{}'",
                    modifier.music_id
                ),
            ));
        }
        if modifier.denominator == 0 {
            diagnostics.push(VerificationError::error(
                "invalid_encounter_music_modifier_ratio",
                &subject,
                "encounter music modifier denominator must be greater than zero",
            ));
        }
    }
}

fn verify_battle_stat_multipliers(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    if data.moves.is_empty() {
        return;
    }
    verify_battle_stat_multiplier_table(
        "stat",
        &data.battle_stat_multipliers.stat,
        "battle_stat_multipliers:stat",
        diagnostics,
    );
    verify_battle_stat_multiplier_table(
        "accuracy",
        &data.battle_stat_multipliers.accuracy,
        "battle_stat_multipliers:accuracy",
        diagnostics,
    );
}

fn verify_battle_stat_multiplier_table(
    table_name: &str,
    table: &[crystal_core::battle::stats::BattleStatMultiplier],
    subject: &str,
    diagnostics: &mut Vec<VerificationError>,
) {
    if table.len() != 13 {
        diagnostics.push(VerificationError::error(
            "invalid_battle_stat_multiplier_table_length",
            subject,
            format!(
                "battle stat multiplier table {table_name} must declare exactly 13 rows for stages -6..=6, found {}",
                table.len()
            ),
        ));
    }
    for (index, entry) in table.iter().enumerate() {
        let stage = index as i8 - 6;
        if entry.numerator <= 0 {
            diagnostics.push(VerificationError::error(
                "invalid_battle_stat_multiplier_numerator",
                subject,
                format!(
                    "battle stat multiplier table {table_name} stage {stage} has nonpositive numerator {}",
                    entry.numerator
                ),
            ));
        }
        if entry.denominator <= 0 {
            diagnostics.push(VerificationError::error(
                "invalid_battle_stat_multiplier_denominator",
                subject,
                format!(
                    "battle stat multiplier table {table_name} stage {stage} has nonpositive denominator {}",
                    entry.denominator
                ),
            ));
        }
    }
}

fn verify_weather_modifiers(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    if data.moves.is_empty() {
        return;
    }
    if data.weather_modifiers.type_modifiers.is_empty() {
        diagnostics.push(VerificationError::error(
            "missing_weather_type_modifiers",
            "weather_modifiers:type_modifiers",
            "weather type modifiers must be declared when moves exist",
        ));
    }
    if data.weather_modifiers.move_effect_modifiers.is_empty() {
        diagnostics.push(VerificationError::error(
            "missing_weather_move_effect_modifiers",
            "weather_modifiers:move_effect_modifiers",
            "weather move-effect modifiers must be declared when moves exist",
        ));
    }
    for entry in &data.weather_modifiers.type_modifiers {
        if entry.weather.trim().is_empty() || entry.weather.trim() != entry.weather {
            diagnostics.push(VerificationError::error(
                "invalid_weather_modifier_weather",
                "weather_modifiers:type_modifiers",
                format!(
                    "weather modifier has invalid weather id {:?}",
                    entry.weather
                ),
            ));
        }
        verify_type_multiplier(
            entry.multiplier,
            "weather_modifiers:type_modifiers",
            diagnostics,
        );
    }
    for entry in &data.weather_modifiers.move_effect_modifiers {
        if entry.weather.trim().is_empty() || entry.weather.trim() != entry.weather {
            diagnostics.push(VerificationError::error(
                "invalid_weather_modifier_weather",
                "weather_modifiers:move_effect_modifiers",
                format!(
                    "weather modifier has invalid weather id {:?}",
                    entry.weather
                ),
            ));
        }
        if entry.move_effect.trim().is_empty() || entry.move_effect.trim() != entry.move_effect {
            diagnostics.push(VerificationError::error(
                "invalid_weather_modifier_move_effect",
                "weather_modifiers:move_effect_modifiers",
                format!(
                    "weather move-effect modifier has invalid move effect {:?}",
                    entry.move_effect
                ),
            ));
        }
        verify_type_multiplier(
            entry.multiplier,
            "weather_modifiers:move_effect_modifiers",
            diagnostics,
        );
    }
}

fn verify_type_effectiveness(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    if data.moves.is_empty() {
        return;
    }
    if data.type_effectiveness.matchups.is_empty() {
        diagnostics.push(VerificationError::error(
            "missing_type_effectiveness_matchups",
            "type_effectiveness:matchups",
            "type effectiveness matchups must be declared when moves exist",
        ));
    }
    if data.type_effectiveness.foresight_matchups.is_empty() {
        diagnostics.push(VerificationError::error(
            "missing_type_effectiveness_foresight_matchups",
            "type_effectiveness:foresight_matchups",
            "Foresight type effectiveness matchups must be declared when moves exist",
        ));
    }
    let declared_types: BTreeSet<&str> = data
        .type_categories
        .physical
        .iter()
        .chain(data.type_categories.special.iter())
        .map(String::as_str)
        .collect();
    let mut matchup_pairs = BTreeSet::new();
    for entry in &data.type_effectiveness.matchups {
        verify_type_multiplier(entry.multiplier, "type_effectiveness:matchups", diagnostics);
        if !declared_types.is_empty() {
            if !declared_types.contains(entry.attacker.as_str()) {
                diagnostics.push(VerificationError::error(
                    "unknown_type_effectiveness_attacker",
                    "type_effectiveness:matchups",
                    format!(
                        "type effectiveness attacker {:?} is not declared in type categories",
                        entry.attacker
                    ),
                ));
            }
            if !declared_types.contains(entry.defender.as_str()) {
                diagnostics.push(VerificationError::error(
                    "unknown_type_effectiveness_defender",
                    "type_effectiveness:matchups",
                    format!(
                        "type effectiveness defender {:?} is not declared in type categories",
                        entry.defender
                    ),
                ));
            }
        }
        if !matchup_pairs.insert((entry.attacker.as_str(), entry.defender.as_str())) {
            diagnostics.push(VerificationError::error(
                "duplicate_type_effectiveness_matchup",
                "type_effectiveness:matchups",
                format!(
                    "type effectiveness matchup {:?} -> {:?} is declared more than once",
                    entry.attacker, entry.defender
                ),
            ));
        }
    }
    for attacker in &declared_types {
        for defender in &declared_types {
            if !matchup_pairs.contains(&(*attacker, *defender)) {
                diagnostics.push(VerificationError::error(
                    "missing_type_effectiveness_matchup",
                    "type_effectiveness:matchups",
                    format!(
                        "type effectiveness matchup {:?} -> {:?} must be declared explicitly",
                        attacker, defender
                    ),
                ));
            }
        }
    }
    let mut foresight_pairs = BTreeSet::new();
    for entry in &data.type_effectiveness.foresight_matchups {
        verify_type_multiplier(
            entry.multiplier,
            "type_effectiveness:foresight_matchups",
            diagnostics,
        );
        if !declared_types.is_empty() {
            if !declared_types.contains(entry.attacker.as_str()) {
                diagnostics.push(VerificationError::error(
                    "unknown_foresight_type_effectiveness_attacker",
                    "type_effectiveness:foresight_matchups",
                    format!(
                        "Foresight type effectiveness attacker {:?} is not declared in type categories",
                        entry.attacker
                    ),
                ));
            }
            if !declared_types.contains(entry.defender.as_str()) {
                diagnostics.push(VerificationError::error(
                    "unknown_foresight_type_effectiveness_defender",
                    "type_effectiveness:foresight_matchups",
                    format!(
                        "Foresight type effectiveness defender {:?} is not declared in type categories",
                        entry.defender
                    ),
                ));
            }
        }
        if !foresight_pairs.insert((entry.attacker.as_str(), entry.defender.as_str())) {
            diagnostics.push(VerificationError::error(
                "duplicate_foresight_type_effectiveness_matchup",
                "type_effectiveness:foresight_matchups",
                format!(
                    "Foresight type effectiveness matchup {:?} -> {:?} is declared more than once",
                    entry.attacker, entry.defender
                ),
            ));
        }
    }
}

fn verify_type_categories(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    if data.moves.is_empty() {
        return;
    }
    if data.type_categories.physical.is_empty() {
        diagnostics.push(VerificationError::error(
            "missing_physical_type_categories",
            "type_categories:physical",
            "physical type categories must be declared when moves exist",
        ));
    }
    if data.type_categories.special.is_empty() {
        diagnostics.push(VerificationError::error(
            "missing_special_type_categories",
            "type_categories:special",
            "special type categories must be declared when moves exist",
        ));
    }
    for type_id in &data.type_categories.physical {
        verify_exact_type_category_token(type_id, "type_categories:physical", diagnostics);
    }
    for type_id in &data.type_categories.special {
        verify_exact_type_category_token(type_id, "type_categories:special", diagnostics);
    }
    for type_id in &data.type_categories.physical {
        if data
            .type_categories
            .special
            .iter()
            .any(|entry| entry == type_id)
        {
            diagnostics.push(VerificationError::error(
                "overlapping_type_category",
                "type_categories",
                format!("type category '{type_id}' is declared as both physical and special"),
            ));
        }
    }
}

fn verify_move_priorities(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    if data.moves.is_empty() {
        return;
    }
    if data.move_priorities.base_priority < 0 {
        diagnostics.push(VerificationError::error(
            "invalid_base_move_priority",
            "move_priorities:base_priority",
            format!(
                "base move priority must be nonnegative, found {}",
                data.move_priorities.base_priority
            ),
        ));
    }
    if data.move_priorities.effect_priorities.is_empty() {
        diagnostics.push(VerificationError::error(
            "missing_move_effect_priorities",
            "move_priorities:effect_priorities",
            "move effect priorities must be declared when moves exist",
        ));
    }
    let mut effect_priorities = BTreeSet::new();
    for entry in &data.move_priorities.effect_priorities {
        if entry.move_effect.trim().is_empty() || entry.move_effect.trim() != entry.move_effect {
            diagnostics.push(VerificationError::error(
                "invalid_move_effect_priority_id",
                "move_priorities:effect_priorities",
                format!(
                    "move effect priority id must be exact and untrimmed, found {:?}",
                    entry.move_effect
                ),
            ));
        }
        if !effect_priorities.insert(entry.move_effect.as_str()) {
            diagnostics.push(VerificationError::error(
                "duplicate_move_effect_priority",
                "move_priorities:effect_priorities",
                format!(
                    "move effect priority '{}' is declared more than once",
                    entry.move_effect
                ),
            ));
        }
        if entry.priority < 0 {
            diagnostics.push(VerificationError::error(
                "invalid_move_effect_priority",
                "move_priorities:effect_priorities",
                format!(
                    "move effect priority must be nonnegative, found {}",
                    entry.priority
                ),
            ));
        }
    }
    for move_data in data.moves.values() {
        if !effect_priorities.contains(move_data.effect.as_str()) {
            diagnostics.push(VerificationError::error(
                "missing_move_effect_priority",
                "move_priorities:effect_priorities",
                format!(
                    "move '{}' effect '{}' must have an explicit priority row",
                    move_data.name, move_data.effect
                ),
            ));
        }
    }
    for entry in &data.move_priorities.move_priorities {
        if entry.r#move.trim().is_empty() || entry.r#move.trim() != entry.r#move {
            diagnostics.push(VerificationError::error(
                "invalid_move_priority_id",
                "move_priorities:move_priorities",
                format!(
                    "move priority override id must be exact and untrimmed, found {:?}",
                    entry.r#move
                ),
            ));
        }
        if entry.priority < 0 {
            diagnostics.push(VerificationError::error(
                "invalid_move_priority",
                "move_priorities:move_priorities",
                format!(
                    "move priority override must be nonnegative, found {}",
                    entry.priority
                ),
            ));
        }
    }
}

fn verify_exact_type_category_token(
    type_id: &str,
    subject: &str,
    diagnostics: &mut Vec<VerificationError>,
) {
    if type_id.trim().is_empty() || type_id.trim() != type_id {
        diagnostics.push(VerificationError::error(
            "invalid_type_category_token",
            subject,
            format!("type category token must be exact and untrimmed, found {type_id:?}"),
        ));
    }
}

fn verify_type_multiplier(
    multiplier: crystal_core::battle::damage::TypeMultiplier,
    subject: &str,
    diagnostics: &mut Vec<VerificationError>,
) {
    if multiplier.denominator == 0 {
        diagnostics.push(VerificationError::error(
            "invalid_type_multiplier_denominator",
            subject,
            "type multiplier denominator must be nonzero",
        ));
    }
}

fn verify_special_routines(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    let (music, _, _) = script_audio_catalog_ids(data);
    for routine in &data.special_routines {
        if routine.trim().is_empty() {
            diagnostics.push(VerificationError::error(
                "empty_special_routine",
                "special_routines",
                "special routine ids must be nonempty exact labels",
            ));
        }
        if !is_known_special_routine(routine) {
            diagnostics.push(VerificationError::error(
                "unknown_declared_special_routine",
                &format!("special_routines:{routine}"),
                format!("special routine '{routine}' is not implemented by the Rust runtime"),
            ));
        }
        if routine == "FadeOutMusic" && !music.contains("MUSIC_NONE") {
            diagnostics.push(VerificationError::error(
                "missing_special_routine_music_id",
                "special_routines:FadeOutMusic",
                "FadeOutMusic requires the modpack to declare exact music id 'MUSIC_NONE'",
            ));
        }
    }
    if data.special_routines.contains("InitRoamMons") && data.roaming_pokemon.is_empty() {
        diagnostics.push(VerificationError::error(
            "missing_roaming_pokemon_definitions",
            "special_routines:InitRoamMons",
            "InitRoamMons requires explicit roaming Pokemon definitions in the modpack",
        ));
    }
    if data.special_routines.contains("BuenaPrize") && data.buena_prizes.is_empty() {
        diagnostics.push(VerificationError::error(
            "missing_buena_prize_definitions",
            "special_routines:BuenaPrize",
            "BuenaPrize requires explicit Buena prize definitions in the modpack",
        ));
    }
    if data.special_routines.contains("BuenasPassword") && data.buena_password_categories.is_empty()
    {
        diagnostics.push(VerificationError::error(
            "missing_buena_password_categories",
            "special_routines:BuenasPassword",
            "BuenasPassword requires explicit Buena password categories in the modpack",
        ));
    }
    if data.special_routines.contains("SelectApricornForKurt")
        && data.kurt_apricorn_recipes.is_empty()
    {
        diagnostics.push(VerificationError::error(
            "missing_kurt_apricorn_recipes",
            "special_routines:SelectApricornForKurt",
            "SelectApricornForKurt requires explicit Kurt apricorn recipes in the modpack",
        ));
    }
    if (data.special_routines.contains("GiveShuckle")
        || data.special_routines.contains("ReturnShuckie"))
        && data.shuckie_gift.is_none()
    {
        diagnostics.push(VerificationError::error(
            "missing_shuckie_gift",
            "special_routines:Shuckie",
            "GiveShuckle and ReturnShuckie require explicit Shuckie gift data in the modpack",
        ));
    }
    if data.special_routines.contains("GiveDratini") && data.dratini_move_sets.is_empty() {
        diagnostics.push(VerificationError::error(
            "missing_dratini_move_sets",
            "special_routines:GiveDratini",
            "GiveDratini requires explicit Dratini move sets in the modpack",
        ));
    }
    if data.special_routines.contains("GiveOddEgg") && data.odd_egg_definitions.is_empty() {
        diagnostics.push(VerificationError::error(
            "missing_odd_egg_definitions",
            "special_routines:GiveOddEgg",
            "GiveOddEgg requires explicit Odd Egg definitions in the modpack",
        ));
    }
    if (data.special_routines.contains("GiveParkBalls")
        || data
            .special_routines
            .contains("SelectRandomBugContestContestants"))
        && data.bug_contest_config.is_none()
    {
        diagnostics.push(VerificationError::error(
            "missing_bug_contest_config",
            "special_routines:BugContest",
            "GiveParkBalls and SelectRandomBugContestContestants require explicit Bug-Catching Contest config in the modpack",
        ));
    }
    if (data.special_routines.contains("BattleTowerAction")
        || data.special_routines.contains("CheckForBattleTowerRules"))
        && data.battle_tower_rules.is_none()
    {
        diagnostics.push(VerificationError::error(
            "missing_battle_tower_rules",
            "special_routines:BattleTowerRules",
            "Battle Tower special routines require explicit Battle Tower rules in the modpack",
        ));
    }
    if data.special_routines.contains("ProfOaksPCBoot") && data.oak_ratings.is_empty() {
        diagnostics.push(VerificationError::error(
            "missing_oak_rating_table",
            "special_routines:ProfOaksPCBoot",
            "ProfOaksPCBoot requires explicit Oak rating entries in the modpack",
        ));
    }
    if data.special_routines.contains("CheckMagikarpLength") && data.magikarp_lengths.is_empty() {
        diagnostics.push(VerificationError::error(
            "missing_magikarp_length_table",
            "special_routines:CheckMagikarpLength",
            "CheckMagikarpLength requires explicit Magikarp length table in the modpack",
        ));
    }
    let happiness_service_required = [
        "OlderHaircutBrother",
        "YoungerHaircutBrother",
        "DaisysGrooming",
    ]
    .iter()
    .any(|routine| data.special_routines.contains(*routine));
    if happiness_service_required && data.happiness_data.is_none() {
        diagnostics.push(VerificationError::error(
            "missing_happiness_data",
            "special_routines:HappinessService",
            "happiness service routines require explicit happiness data in the modpack",
        ));
    }
    if let Some(happiness_data) = &data.happiness_data {
        verify_happiness_data(happiness_data, diagnostics);
    }
    for (index, definition) in data.roaming_pokemon.iter().enumerate() {
        let subject = format!("roaming_pokemon:{index}");
        if definition.species.trim().is_empty() {
            diagnostics.push(VerificationError::error(
                "empty_roaming_pokemon_species",
                &subject,
                "roaming Pokemon species id must be an exact nonempty id",
            ));
        } else if !data.pokemon.contains_key(&definition.species) {
            diagnostics.push(VerificationError::error(
                "unknown_roaming_pokemon_species",
                &subject,
                format!(
                    "roaming Pokemon references missing species '{}'",
                    definition.species
                ),
            ));
        }
        if definition.level == 0 {
            diagnostics.push(VerificationError::error(
                "invalid_roaming_pokemon_level",
                &subject,
                "roaming Pokemon level must be nonzero",
            ));
        }
    }
    for (index, prize) in data.buena_prizes.iter().enumerate() {
        let subject = format!("buena_prizes:{index}");
        if prize.item_id.trim().is_empty() {
            diagnostics.push(VerificationError::error(
                "empty_buena_prize_item",
                &subject,
                "Buena prize item id must be an exact nonempty id",
            ));
        } else if !data.items.contains_key(&prize.item_id) {
            diagnostics.push(VerificationError::error(
                "unknown_buena_prize_item",
                &subject,
                format!("Buena prize references missing item '{}'", prize.item_id),
            ));
        }
        if prize.cost == 0 {
            diagnostics.push(VerificationError::error(
                "invalid_buena_prize_cost",
                &subject,
                "Buena prize cost must be nonzero",
            ));
        }
    }
    for (index, category) in data.buena_password_categories.iter().enumerate() {
        let subject = format!("buena_password_categories:{index}");
        if category.id.trim().is_empty() {
            diagnostics.push(VerificationError::error(
                "empty_buena_password_category_id",
                &subject,
                "Buena password category id must be an exact nonempty id",
            ));
        }
        if !is_known_buena_password_category_type(&category.category_type) {
            diagnostics.push(VerificationError::error(
                "unknown_buena_password_category_type",
                &subject,
                format!(
                    "Buena password category '{}' has unknown type '{}'",
                    category.id, category.category_type
                ),
            ));
        }
        if category.points == 0 {
            diagnostics.push(VerificationError::error(
                "invalid_buena_password_points",
                &subject,
                "Buena password category points must be nonzero",
            ));
        }
        if category.options.is_empty() {
            diagnostics.push(VerificationError::error(
                "empty_buena_password_options",
                &subject,
                "Buena password category must declare at least one option",
            ));
        }
        for (option_index, option) in category.options.iter().enumerate() {
            let option_subject = format!("{subject}:option:{option_index}");
            if option.trim().is_empty() {
                diagnostics.push(VerificationError::error(
                    "empty_buena_password_option",
                    &option_subject,
                    "Buena password option must be an exact nonempty id or string",
                ));
                continue;
            }
            match category.category_type.as_str() {
                BUENA_PASSWORD_CATEGORY_MON if !data.pokemon.contains_key(option) => {
                    diagnostics.push(VerificationError::error(
                        "unknown_buena_password_species",
                        &option_subject,
                        format!("Buena password option references missing species '{option}'"),
                    ));
                }
                BUENA_PASSWORD_CATEGORY_ITEM if !data.items.contains_key(option) => {
                    diagnostics.push(VerificationError::error(
                        "unknown_buena_password_item",
                        &option_subject,
                        format!("Buena password option references missing item '{option}'"),
                    ));
                }
                BUENA_PASSWORD_CATEGORY_MOVE if !data.moves.contains_key(option) => {
                    diagnostics.push(VerificationError::error(
                        "unknown_buena_password_move",
                        &option_subject,
                        format!("Buena password option references missing move '{option}'"),
                    ));
                }
                _ => {}
            }
        }
    }
    for (index, recipe) in data.kurt_apricorn_recipes.iter().enumerate() {
        let subject = format!("kurt_apricorn_recipes:{index}");
        if recipe.apricorn.trim().is_empty() {
            diagnostics.push(VerificationError::error(
                "empty_kurt_apricorn_recipe_apricorn",
                &subject,
                "Kurt apricorn recipe apricorn id must be an exact nonempty id",
            ));
        } else if !data.items.contains_key(&recipe.apricorn) {
            diagnostics.push(VerificationError::error(
                "unknown_kurt_apricorn_recipe_apricorn",
                &subject,
                format!(
                    "Kurt apricorn recipe references missing apricorn item '{}'",
                    recipe.apricorn
                ),
            ));
        }
        if recipe.ball.trim().is_empty() {
            diagnostics.push(VerificationError::error(
                "empty_kurt_apricorn_recipe_ball",
                &subject,
                "Kurt apricorn recipe ball id must be an exact nonempty id",
            ));
        } else if !data.items.contains_key(&recipe.ball) {
            diagnostics.push(VerificationError::error(
                "unknown_kurt_apricorn_recipe_ball",
                &subject,
                format!(
                    "Kurt apricorn recipe references missing ball item '{}'",
                    recipe.ball
                ),
            ));
        }
    }
    if let Some(gift) = data.shuckie_gift.as_ref() {
        if gift.species.trim().is_empty() {
            diagnostics.push(VerificationError::error(
                "empty_shuckie_gift_species",
                "shuckie_gift",
                "Shuckie gift species id must be an exact nonempty id",
            ));
        } else if !data.pokemon.contains_key(&gift.species) {
            diagnostics.push(VerificationError::error(
                "unknown_shuckie_gift_species",
                "shuckie_gift",
                format!("Shuckie gift references missing species '{}'", gift.species),
            ));
        }
        if gift.level == 0 {
            diagnostics.push(VerificationError::error(
                "invalid_shuckie_gift_level",
                "shuckie_gift",
                "Shuckie gift level must be nonzero",
            ));
        }
        if gift.held_item.trim().is_empty() {
            diagnostics.push(VerificationError::error(
                "empty_shuckie_gift_item",
                "shuckie_gift",
                "Shuckie gift held item id must be an exact nonempty id",
            ));
        } else if !data.items.contains_key(&gift.held_item) {
            diagnostics.push(VerificationError::error(
                "unknown_shuckie_gift_item",
                "shuckie_gift",
                format!(
                    "Shuckie gift references missing held item '{}'",
                    gift.held_item
                ),
            ));
        }
        if gift.nickname.trim().is_empty() || gift.original_trainer_name.trim().is_empty() {
            diagnostics.push(VerificationError::error(
                "empty_shuckie_gift_name",
                "shuckie_gift",
                "Shuckie gift nickname and original trainer name must be nonempty",
            ));
        }
        let engine_flags = script_engine_flag_ids(data);
        if gift.got_today_engine_flag.trim().is_empty() {
            diagnostics.push(VerificationError::error(
                "empty_shuckie_gift_engine_flag",
                "shuckie_gift",
                "Shuckie gift engine flag must be an exact nonempty id",
            ));
        } else if !engine_flags.contains(&gift.got_today_engine_flag) {
            diagnostics.push(VerificationError::error(
                "unknown_shuckie_gift_engine_flag",
                "shuckie_gift",
                format!(
                    "Shuckie gift references missing engine flag '{}'",
                    gift.got_today_engine_flag
                ),
            ));
        }
    }
    for (index, move_set) in data.dratini_move_sets.iter().enumerate() {
        let subject = format!("dratini_move_sets:{index}");
        if move_set.moves.is_empty() {
            diagnostics.push(VerificationError::error(
                "empty_dratini_move_set",
                &subject,
                "Dratini move set must contain at least one move",
            ));
        }
        for (move_index, move_id) in move_set.moves.iter().enumerate() {
            let move_subject = format!("{subject}:move:{move_index}");
            if move_id.trim().is_empty() {
                diagnostics.push(VerificationError::error(
                    "empty_dratini_move",
                    &move_subject,
                    "Dratini move id must be an exact nonempty id",
                ));
            } else if !data.moves.contains_key(move_id) {
                diagnostics.push(VerificationError::error(
                    "unknown_dratini_move",
                    &move_subject,
                    format!("Dratini move set references missing move '{move_id}'"),
                ));
            }
        }
    }
    if !data.odd_egg_definitions.is_empty() {
        let total_probability = data
            .odd_egg_definitions
            .iter()
            .map(|definition| u32::from(definition.probability))
            .sum::<u32>();
        if total_probability != 100 {
            diagnostics.push(VerificationError::error(
                "invalid_odd_egg_probability_total",
                "odd_egg_definitions",
                format!("Odd Egg probabilities must sum to 100, found {total_probability}"),
            ));
        }
    }
    for (index, definition) in data.odd_egg_definitions.iter().enumerate() {
        let subject = format!("odd_egg_definitions:{index}");
        if definition.species.trim().is_empty() || definition.species.trim() != definition.species {
            diagnostics.push(VerificationError::error(
                "invalid_odd_egg_species",
                &subject,
                "Odd Egg species must be an exact nonempty species id",
            ));
        } else if !data.pokemon.contains_key(&definition.species) {
            diagnostics.push(VerificationError::error(
                "unknown_odd_egg_species",
                &subject,
                format!(
                    "Odd Egg references missing species '{}'",
                    definition.species
                ),
            ));
        }
        if definition.moves.is_empty() || definition.moves.len() > 4 {
            diagnostics.push(VerificationError::error(
                "invalid_odd_egg_move_count",
                &subject,
                "Odd Egg move list must contain 1..=4 exact move ids",
            ));
        }
        for (move_index, move_id) in definition.moves.iter().enumerate() {
            let move_subject = format!("{subject}:move:{move_index}");
            if move_id.trim().is_empty() || move_id.trim() != move_id {
                diagnostics.push(VerificationError::error(
                    "invalid_odd_egg_move",
                    &move_subject,
                    "Odd Egg move id must be an exact nonempty id",
                ));
            } else if !data.moves.contains_key(move_id) {
                diagnostics.push(VerificationError::error(
                    "unknown_odd_egg_move",
                    &move_subject,
                    format!("Odd Egg references missing move '{move_id}'"),
                ));
            }
        }
        if definition.probability == 0 {
            diagnostics.push(VerificationError::error(
                "invalid_odd_egg_probability",
                &subject,
                "Odd Egg probability must be positive",
            ));
        }
        if definition.level == 0 || definition.level > 100 {
            diagnostics.push(VerificationError::error(
                "invalid_odd_egg_level",
                &subject,
                format!("Odd Egg level must be 1..=100, found {}", definition.level),
            ));
        }
        if definition.nickname.trim().is_empty()
            || definition.nickname.trim() != definition.nickname
        {
            diagnostics.push(VerificationError::error(
                "invalid_odd_egg_nickname",
                &subject,
                "Odd Egg nickname must be exact nonempty pack data",
            ));
        }
        if definition.original_trainer_name.trim().is_empty()
            || definition.original_trainer_name.trim() != definition.original_trainer_name
        {
            diagnostics.push(VerificationError::error(
                "invalid_odd_egg_original_trainer_name",
                &subject,
                "Odd Egg original trainer name must be exact nonempty pack data",
            ));
        }
    }
    if let Some(config) = data.bug_contest_config.as_ref() {
        if config.park_balls == 0 {
            diagnostics.push(VerificationError::error(
                "invalid_bug_contest_park_balls",
                "bug_contest_config",
                "Bug-Catching Contest park_balls must be positive",
            ));
        }
        if config.timer_seconds > 59 {
            diagnostics.push(VerificationError::error(
                "invalid_bug_contest_timer_seconds",
                "bug_contest_config",
                format!(
                    "Bug-Catching Contest timer_seconds must be 0..=59, found {}",
                    config.timer_seconds
                ),
            ));
        }
        if config.selected_contestant_count == 0 {
            diagnostics.push(VerificationError::error(
                "invalid_bug_contest_selected_count",
                "bug_contest_config",
                "Bug-Catching Contest selected_contestant_count must be positive",
            ));
        }
        if config.contestant_flags.len() < config.selected_contestant_count {
            diagnostics.push(VerificationError::error(
                "invalid_bug_contest_selected_count",
                "bug_contest_config",
                format!(
                    "Bug-Catching Contest selected_contestant_count {} exceeds {} contestant flags",
                    config.selected_contestant_count,
                    config.contestant_flags.len()
                ),
            ));
        }
        let event_flags = script_event_flag_ids(data);
        let mut seen = BTreeSet::new();
        for (index, flag) in config.contestant_flags.iter().enumerate() {
            let subject = format!("bug_contest_config:contestant_flags:{index}");
            if flag.trim().is_empty() || flag.trim() != flag {
                diagnostics.push(VerificationError::error(
                    "empty_bug_contest_contestant_flag",
                    &subject,
                    "Bug-Catching Contest contestant flag must be an exact nonempty id",
                ));
                continue;
            }
            if !seen.insert(flag) {
                diagnostics.push(VerificationError::error(
                    "duplicate_bug_contest_contestant_flag",
                    &subject,
                    format!("Bug-Catching Contest contestant flag '{flag}' is duplicated"),
                ));
            }
            if !event_flags.contains(flag) {
                diagnostics.push(VerificationError::error(
                    "unknown_bug_contest_contestant_flag",
                    &subject,
                    format!("Bug-Catching Contest contestant flag '{flag}' is not loaded"),
                ));
            }
        }
    }
    if let Some(rules) = data.battle_tower_rules.as_ref() {
        if rules.required_party_count == 0 {
            diagnostics.push(VerificationError::error(
                "invalid_battle_tower_required_party_count",
                "battle_tower_rules:required_party_count",
                "Battle Tower requiredPartyCount must be nonzero",
            ));
        }
        if rules.challenge_streak_length == 0 {
            diagnostics.push(VerificationError::error(
                "invalid_battle_tower_challenge_streak_length",
                "battle_tower_rules:challengeStreakLength",
                "Battle Tower challengeStreakLength must be nonzero",
            ));
        }
        if rules.level_group_size == 0 {
            diagnostics.push(VerificationError::error(
                "invalid_battle_tower_level_group_size",
                "battle_tower_rules:levelGroupSize",
                "Battle Tower levelGroupSize must be nonzero",
            ));
        }
        if rules.minimum_level_group == 0 || rules.maximum_level_group < rules.minimum_level_group {
            diagnostics.push(VerificationError::error(
                "invalid_battle_tower_level_group_range",
                "battle_tower_rules:levelGroupRange",
                "Battle Tower level group range must be nonzero and ordered",
            ));
        }
        for (field, value) in [
            (
                "partyCountFailureText",
                rules.party_count_failure_text.as_str(),
            ),
            (
                "duplicateSpeciesFailureText",
                rules.duplicate_species_failure_text.as_str(),
            ),
            (
                "duplicateHeldItemFailureText",
                rules.duplicate_held_item_failure_text.as_str(),
            ),
            ("eggFailureText", rules.egg_failure_text.as_str()),
        ] {
            if value.trim().is_empty() || value.trim() != value {
                diagnostics.push(VerificationError::error(
                    "invalid_battle_tower_failure_text",
                    format!("battle_tower_rules:{field}"),
                    "Battle Tower failure text ids must be exact nonempty ids",
                ));
            }
        }
        let mut seen = BTreeSet::new();
        for (index, species_id) in rules.banned_species.iter().enumerate() {
            let subject = format!("battle_tower_rules:banned_species:{index}");
            if species_id.trim().is_empty() || species_id.trim() != species_id {
                diagnostics.push(VerificationError::error(
                    "invalid_battle_tower_banned_species",
                    &subject,
                    "Battle Tower bannedSpecies entries must be exact nonempty species ids",
                ));
                continue;
            }
            if !seen.insert(species_id) {
                diagnostics.push(VerificationError::error(
                    "duplicate_battle_tower_banned_species",
                    &subject,
                    format!("Battle Tower bannedSpecies repeats '{species_id}'"),
                ));
            }
            if !data.pokemon.contains_key(species_id) {
                diagnostics.push(VerificationError::error(
                    "unknown_battle_tower_banned_species",
                    &subject,
                    format!("Battle Tower bannedSpecies references missing species '{species_id}'"),
                ));
            }
        }
    }
    if !data.oak_ratings.is_empty() {
        let mut previous_limit = None;
        for (index, entry) in data.oak_ratings.iter().enumerate() {
            let subject = format!("oak_ratings:{index}");
            if entry.fanfare.trim().is_empty() || entry.fanfare.trim() != entry.fanfare {
                diagnostics.push(VerificationError::error(
                    "invalid_oak_rating_fanfare",
                    &subject,
                    "Oak rating fanfare must be an exact nonempty id",
                ));
            }
            if entry.text_label.trim().is_empty() || entry.text_label.trim() != entry.text_label {
                diagnostics.push(VerificationError::error(
                    "invalid_oak_rating_text_label",
                    &subject,
                    "Oak rating textLabel must be an exact nonempty id",
                ));
            }
            if previous_limit.is_some_and(|limit| entry.caught_count_limit <= limit) {
                diagnostics.push(VerificationError::error(
                    "invalid_oak_rating_order",
                    &subject,
                    "Oak rating caughtCountLimit values must be strictly increasing",
                ));
            }
            previous_limit = Some(entry.caught_count_limit);
        }
        if let Some(last) = data.oak_ratings.last() {
            let pokemon_count = data.pokemon.len();
            if pokemon_count > 0 && last.caught_count_limit < pokemon_count {
                diagnostics.push(VerificationError::error(
                    "incomplete_oak_rating_coverage",
                    "oak_ratings",
                    format!(
                        "Oak rating table only covers {} caught Pokemon, but {} Pokemon are loaded",
                        last.caught_count_limit, pokemon_count
                    ),
                ));
            }
        }
    }
    let mut previous_magikarp_threshold = None;
    for (index, entry) in data.magikarp_lengths.iter().enumerate() {
        let subject = format!("magikarp_lengths:{index}");
        if entry.divisor == 0 {
            diagnostics.push(VerificationError::error(
                "invalid_magikarp_length_divisor",
                &subject,
                format!(
                    "Magikarp length threshold {} has zero divisor",
                    entry.threshold
                ),
            ));
        }
        if previous_magikarp_threshold.is_some_and(|previous| entry.threshold <= previous) {
            diagnostics.push(VerificationError::error(
                "invalid_magikarp_length_threshold_order",
                &subject,
                "Magikarp length thresholds must be strictly increasing",
            ));
        }
        previous_magikarp_threshold = Some(entry.threshold);
    }
}

fn script_event_flag_ids(data: &GameDataSet) -> BTreeSet<String> {
    let mut flags = data
        .initialize_events
        .event_flags
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    for module in data.maps.values() {
        for command in &module.script_flag_commands {
            if command.flag_id.starts_with("EVENT_") {
                flags.insert(command.flag_id.clone());
            }
        }
    }
    flags
}

fn script_engine_flag_ids(data: &GameDataSet) -> BTreeSet<String> {
    let mut flags = data
        .initialize_events
        .engine_flags
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    for module in data.maps.values() {
        for command in &module.script_flag_commands {
            if command.flag_id.starts_with("ENGINE_") {
                flags.insert(command.flag_id.clone());
            }
        }
    }
    flags
}

fn verify_script_runtime_commands(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    for (map_name, module) in &data.maps {
        for command in &module.script_runtime_commands {
            let subject = format!(
                "{map_name}:{}:{}",
                command.source_script, command.command_index
            );
            if let Err(error) = validate_script_runtime_command(command) {
                match error {
                    ScriptRuntimeCommandError::UnknownCommand { command } => {
                        diagnostics.push(VerificationError::error(
                            "unknown_script_runtime_command",
                            &subject,
                            format!("unknown runtime command '{command}'"),
                        ));
                    }
                    ScriptRuntimeCommandError::WrongArgCount {
                        command,
                        expected,
                        actual,
                    } => {
                        diagnostics.push(VerificationError::error(
                            "malformed_script_runtime_command",
                            &subject,
                            format!("{command} expects {expected} args but found {actual}"),
                        ));
                    }
                    ScriptRuntimeCommandError::EmptyArg { command }
                    | ScriptRuntimeCommandError::PaddedArg { command, .. } => {
                        diagnostics.push(VerificationError::error(
                            "malformed_script_runtime_command",
                            &subject,
                            format!("{command} requires exact nonempty args"),
                        ));
                    }
                    _ => {}
                }
                continue;
            };
            match command.command.as_str() {
                "special" => {
                    let special_id = &command.args[0];
                    if !data.special_routines.contains(special_id) {
                        diagnostics.push(VerificationError::error(
                            "unknown_script_special_routine",
                            &subject,
                            format!("special references unknown routine '{special_id}'"),
                        ));
                    }
                }
                "gettrainername" => {
                    let trainer_class = &command.args[1];
                    let trainer_id = &command.args[2];
                    match data.trainers.get(trainer_id) {
                        Some(trainer) if trainer.trainer_class == *trainer_class => {}
                        Some(trainer) => diagnostics.push(VerificationError::error(
                            "script_trainer_name_class_mismatch",
                            &subject,
                            format!(
                                "gettrainername expected trainer '{}' to have class '{}' but pack declares '{}'",
                                trainer_id, trainer_class, trainer.trainer_class
                            ),
                        )),
                        None => diagnostics.push(VerificationError::error(
                            "unknown_script_trainer_name",
                            &subject,
                            format!("gettrainername references unknown trainer '{trainer_id}'"),
                        )),
                    }
                }
                "getitemname" => {
                    let item_id = &command.args[1];
                    if item_id != "USE_SCRIPT_VAR"
                        && item_id != "ITEM_FROM_MEM"
                        && !data.items.contains_key(item_id)
                    {
                        diagnostics.push(VerificationError::error(
                            "unknown_script_item_name",
                            &subject,
                            format!("getitemname references unknown item '{item_id}'"),
                        ));
                    }
                }
                "getmonname" => {
                    let species_id = &command.args[1];
                    if species_id != "USE_SCRIPT_VAR" && !data.pokemon.contains_key(species_id) {
                        diagnostics.push(VerificationError::error(
                            "unknown_script_mon_name",
                            &subject,
                            format!("getmonname references unknown species '{species_id}'"),
                        ));
                    }
                }
                "addcellnum" => {
                    let contact_id = &command.args[0];
                    if !data.phone_contacts.0.contains_key(contact_id) {
                        diagnostics.push(VerificationError::error(
                            "unknown_script_addcellnum_contact",
                            &subject,
                            format!("addcellnum references unknown contact '{contact_id}'"),
                        ));
                    }
                }
                "specialphonecall" => {
                    let call_id = &command.args[0];
                    if !data.special_phone_calls.contains(call_id) {
                        diagnostics.push(VerificationError::error(
                            "unknown_script_special_phone_call",
                            &subject,
                            format!("specialphonecall references unknown call '{call_id}'"),
                        ));
                    }
                }
                "checkpoke" | "pokepic" => {
                    let species_id = &command.args[0];
                    if !data.pokemon.contains_key(species_id) {
                        diagnostics.push(VerificationError::error(
                            "unknown_script_species_runtime_command",
                            &subject,
                            format!(
                                "{} references unknown species '{}'",
                                command.command, species_id
                            ),
                        ));
                    }
                }
                "trade" => {
                    let trade_id = &command.args[0];
                    if !data.npc_trades.contains(trade_id) {
                        diagnostics.push(VerificationError::error(
                            "unknown_script_npc_trade",
                            &subject,
                            format!("trade references unknown NPC trade '{trade_id}'"),
                        ));
                    }
                }
                "cmdqueue" | "writecmdqueue" | "elevator" | "callasm" | "dba" | "dw"
                | "checkpokemail" | "givepokemail" => {
                    let target_label = match command.command.as_str() {
                        "cmdqueue" => &command.args[1],
                        _ => &command.args[0],
                    };
                    if target_label != "BANK(@)"
                        && resolve_script_target_label(
                            &module.scripts,
                            &command.source_script,
                            target_label,
                        )
                        .is_none()
                    {
                        diagnostics.push(VerificationError::error(
                            "unknown_script_runtime_target",
                            &subject,
                            format!(
                                "{} references unknown target '{}'",
                                command.command, target_label
                            ),
                        ));
                    }
                }
                "stonetable" => {
                    let target_label = &command.args[2];
                    if resolve_script_target_label(
                        &module.scripts,
                        &command.source_script,
                        target_label,
                    )
                    .is_none()
                    {
                        diagnostics.push(VerificationError::error(
                            "unknown_script_runtime_target",
                            &subject,
                            format!("stonetable references unknown target '{target_label}'"),
                        ));
                    }
                }
                "conditional_event" => {
                    let target_label = &command.args[1];
                    if resolve_script_target_label(
                        &module.scripts,
                        &command.source_script,
                        target_label,
                    )
                    .is_none()
                    {
                        diagnostics.push(VerificationError::error(
                            "unknown_script_runtime_target",
                            &subject,
                            format!("conditional_event references unknown target '{target_label}'"),
                        ));
                    }
                }
                _ => {}
            }
        }
    }
}

fn verify_map_section_commands(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    let script_counts = map_script_section_command_arg_counts();
    let event_counts = map_event_section_command_arg_counts();
    for (map_name, module) in &data.maps {
        for command in &module.map_script_section_commands {
            let subject = format!("{map_name}:map_scripts:{}", command.command_index);
            let Some(expected) = script_counts.get(command.command.as_str()) else {
                diagnostics.push(VerificationError::error(
                    "unknown_map_script_section_command",
                    &subject,
                    format!("unknown map script section command '{}'", command.command),
                ));
                continue;
            };
            if !expected.contains(&command.args.len()) {
                diagnostics.push(VerificationError::error(
                    "malformed_map_script_section_command",
                    &subject,
                    format!(
                        "{} expects one of {:?} args but found {}",
                        command.command,
                        expected,
                        command.args.len()
                    ),
                ));
                continue;
            }
            match command.command.as_str() {
                "scene_script" => {
                    let script = &command.args[0];
                    if !module.scripts.contains_key(script) {
                        diagnostics.push(VerificationError::error(
                            "unknown_map_scene_script",
                            &subject,
                            format!("scene_script references unknown script '{script}'"),
                        ));
                    }
                }
                "callback" => {
                    let script = &command.args[1];
                    if !module.scripts.contains_key(script) {
                        diagnostics.push(VerificationError::error(
                            "unknown_map_callback_script",
                            &subject,
                            format!("callback references unknown script '{script}'"),
                        ));
                    }
                }
                _ => {}
            }
        }
        for command in &module.map_event_section_commands {
            let subject = format!("{map_name}:map_events:{}", command.command_index);
            let Some(expected) = event_counts.get(command.command.as_str()) else {
                diagnostics.push(VerificationError::error(
                    "unknown_map_event_section_command",
                    &subject,
                    format!("unknown map event section command '{}'", command.command),
                ));
                continue;
            };
            if !expected.contains(&command.args.len()) {
                diagnostics.push(VerificationError::error(
                    "malformed_map_event_section_command",
                    &subject,
                    format!(
                        "{} expects one of {:?} args but found {}",
                        command.command,
                        expected,
                        command.args.len()
                    ),
                ));
                continue;
            }
            match command.command.as_str() {
                "coord_event" | "bg_event" => {
                    let script = &command.args[3];
                    if !module.scripts.contains_key(script) {
                        diagnostics.push(VerificationError::error(
                            "unknown_map_event_script",
                            &subject,
                            format!("{} references unknown script '{script}'", command.command),
                        ));
                    }
                }
                "object_event" => {
                    let script = &command.args[11];
                    if script != "-1"
                        && script != "ObjectEvent"
                        && !module.scripts.contains_key(script)
                    {
                        diagnostics.push(VerificationError::error(
                            "unknown_map_object_event_script",
                            &subject,
                            format!("object_event references unknown script '{script}'"),
                        ));
                    }
                }
                _ => {}
            }
        }
    }
}

fn is_text_script(payload: &Value) -> bool {
    let Some(entries) = payload.as_array() else {
        return false;
    };
    entries.iter().any(|entry| {
        matches!(
            entry.get("command").and_then(Value::as_str),
            Some(
                "text"
                    | "line"
                    | "para"
                    | "cont"
                    | "done"
                    | "prompt"
                    | "text_ram"
                    | "text_decimal"
                    | "text_far"
                    | "sound_dex_fanfare_50_79"
                    | "sound_dex_fanfare_80_109"
                    | "sound_dex_fanfare_140_169"
                    | "sound_dex_fanfare_170_199"
                    | "sound_dex_fanfare_200_229"
                    | "sound_dex_fanfare_230_plus"
            )
        )
    })
}

fn verify_script_direction(
    diagnostics: &mut Vec<VerificationError>,
    subject: &str,
    direction: Option<&str>,
) {
    let Some(direction) = direction else {
        diagnostics.push(VerificationError::error(
            "missing_script_direction",
            subject,
            "movement command is missing a direction",
        ));
        return;
    };
    if let Err(error) = parse_script_direction(direction) {
        diagnostics.push(VerificationError::error(
            "unknown_script_direction",
            subject,
            error.to_string(),
        ));
    }
}

fn verify_required_object_id(
    diagnostics: &mut Vec<VerificationError>,
    subject: &str,
    command: &ScriptObjectCommand,
    object_ids: &BTreeSet<&str>,
    allow_player: bool,
) {
    let Some(object_id) = command.object_id.as_deref() else {
        diagnostics.push(VerificationError::error(
            "script_object_missing_id",
            subject,
            format!("{} command is missing an object id", command.command),
        ));
        return;
    };
    if allow_player && object_id == "PLAYER" {
        return;
    }
    if object_id == "LAST_TALKED" {
        return;
    }
    if !object_ids.contains(object_id) {
        diagnostics.push(VerificationError::error(
            "unknown_script_object_id",
            subject,
            format!(
                "{} references missing object id '{object_id}'",
                command.command
            ),
        ));
    }
}

fn verify_required_target_object_id(
    diagnostics: &mut Vec<VerificationError>,
    subject: &str,
    command: &ScriptObjectCommand,
    object_ids: &BTreeSet<&str>,
    allow_player: bool,
) {
    let Some(object_id) = command.target_object_id.as_deref() else {
        diagnostics.push(VerificationError::error(
            "script_object_missing_target_id",
            subject,
            format!("{} command is missing a target object id", command.command),
        ));
        return;
    };
    if allow_player && object_id == "PLAYER" {
        return;
    }
    if object_id == "LAST_TALKED" {
        return;
    }
    if !object_ids.contains(object_id) {
        diagnostics.push(VerificationError::error(
            "unknown_script_object_id",
            subject,
            format!(
                "{} references missing target object id '{object_id}'",
                command.command
            ),
        ));
    }
}

fn script_audio_catalog_ids(
    data: &GameDataSet,
) -> (BTreeSet<String>, BTreeSet<String>, BTreeSet<String>) {
    let mut music = BTreeSet::new();
    let mut sound_effects = BTreeSet::new();
    let mut cries = BTreeSet::new();
    for asset in &data.audio {
        match asset.kind {
            ModpackAudioKind::Music => {
                insert_audio_id(&mut music, asset);
            }
            ModpackAudioKind::SoundEffect => {
                insert_audio_id(&mut sound_effects, asset);
            }
            ModpackAudioKind::Cry => {
                cries.insert(asset.id.clone());
            }
        }
    }
    (music, sound_effects, cries)
}

fn insert_audio_id(catalog: &mut BTreeSet<String>, asset: &ModpackAudioAsset) {
    catalog.insert(asset.id.clone());
}

fn scene_table_for_map_id<'a>(
    data: &'a GameDataSet,
    map_id: &str,
) -> Option<(String, &'a MapModule)> {
    data.maps
        .iter()
        .find(|(_, module)| module.attributes.map_constant.as_deref() == Some(map_id))
        .map(|(map_name, module)| (map_name.clone(), module))
}

fn scene_slot_count(module: &MapModule) -> usize {
    module
        .map_script_section_commands
        .iter()
        .filter(|command| command.command == "scene_script" || command.command == "scene_const")
        .count()
}

fn verify_scene_token(
    diagnostics: &mut Vec<VerificationError>,
    subject: &str,
    map_name: &str,
    scene_id: Option<&str>,
    table: &MapSceneTable,
    scene_slot_count: usize,
) {
    let Some(scene_id) = scene_id else {
        diagnostics.push(VerificationError::error(
            "missing_script_scene_id",
            subject,
            "scene command is missing a scene id",
        ));
        return;
    };
    if table.scenes.iter().any(|scene| scene.scene_id == scene_id) {
        return;
    }
    if let Ok(index) = scene_id.parse::<usize>() {
        if index < scene_slot_count {
            return;
        }
    }
    diagnostics.push(VerificationError::error(
        "unknown_script_scene_id",
        subject,
        format!("scene command references missing scene '{scene_id}' on {map_name}"),
    ));
}

fn verify_battle_reward_rules(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    if data.battle_reward_rules == BattleRewardRules::default() {
        return;
    }
    if data.battle_reward_rules.max_level == 0 {
        diagnostics.push(VerificationError::error(
            "invalid_battle_reward_rule",
            "battle_reward_rules:max_level",
            "battle reward rules maxLevel must be nonzero",
        ));
    }
    if data.battle_reward_rules.wild_exp_divisor <= 0 {
        diagnostics.push(VerificationError::error(
            "invalid_battle_reward_rule",
            "battle_reward_rules:wild_exp_divisor",
            "battle reward rules wildExpDivisor must be positive",
        ));
    }
    if data.battle_reward_rules.trainer_exp_numerator <= 0 {
        diagnostics.push(VerificationError::error(
            "invalid_battle_reward_rule",
            "battle_reward_rules:trainer_exp_numerator",
            "battle reward rules trainerExpNumerator must be positive",
        ));
    }
    if data.battle_reward_rules.trainer_exp_denominator <= 0 {
        diagnostics.push(VerificationError::error(
            "invalid_battle_reward_rule",
            "battle_reward_rules:trainer_exp_denominator",
            "battle reward rules trainerExpDenominator must be positive",
        ));
    }
}

fn verify_battle_escape_rules(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    if data.pokemon.is_empty() {
        return;
    }
    if data.battle_escape_rules == BattleEscapeRules::default() {
        diagnostics.push(VerificationError::error(
            "missing_battle_escape_rules",
            "battle_escape_rules",
            "battle escape rules must be declared when Pokemon exist",
        ));
        return;
    }
    if data.battle_escape_rules.player_speed_multiplier == 0 {
        diagnostics.push(VerificationError::error(
            "invalid_battle_escape_rule",
            "battle_escape_rules:player_speed_multiplier",
            "battle escape player speed multiplier must be nonzero",
        ));
    }
    if data.battle_escape_rules.enemy_speed_divisor == 0 {
        diagnostics.push(VerificationError::error(
            "invalid_battle_escape_rule",
            "battle_escape_rules:enemy_speed_divisor",
            "battle escape enemy speed divisor must be nonzero",
        ));
    }
    if data.battle_escape_rules.rng_roll_values == 0
        || data.battle_escape_rules.rng_roll_values > u16::from(u8::MAX) + 1
    {
        diagnostics.push(VerificationError::error(
            "invalid_battle_escape_rule",
            "battle_escape_rules:rng_roll_values",
            "battle escape rng roll values must be in 1..=256",
        ));
    }
}

fn verify_step_event_rules(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    if data.step_event_rules == StepEventRules::default() {
        return;
    }
    if data.step_event_rules.poison_step_interval == 0 {
        diagnostics.push(VerificationError::error(
            "invalid_step_event_rule",
            "step_event_rules:poison_step_interval",
            "step event rules poisonStepInterval must be nonzero",
        ));
    }
    if data.step_event_rules.poison_status.trim().is_empty()
        || data.step_event_rules.poison_status.trim() != data.step_event_rules.poison_status
    {
        diagnostics.push(VerificationError::error(
            "invalid_step_event_rule",
            "step_event_rules:poison_status",
            "step event rules poisonStatus must be an exact nonempty status id",
        ));
    }
    if data.step_event_rules.egg_nickname.trim().is_empty()
        || data.step_event_rules.egg_nickname.trim() != data.step_event_rules.egg_nickname
    {
        diagnostics.push(VerificationError::error(
            "invalid_step_event_rule",
            "step_event_rules:egg_nickname",
            "step event rules eggNickname must be an exact nonempty nickname token",
        ));
    }
    if data.step_event_rules.happiness_step_counter_target
        > data.step_event_rules.happiness_step_counter_mask
    {
        diagnostics.push(VerificationError::error(
            "invalid_step_event_rule",
            "step_event_rules:happiness_step_counter_target",
            "step event rules happinessStepCounterTarget must fit inside happinessStepCounterMask",
        ));
    }
}

fn verify_fishing(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    let referenced_groups: Vec<(&String, &str)> = data
        .map_attributes
        .iter()
        .chain(
            data.maps
                .iter()
                .map(|(map_name, module)| (map_name, &module.attributes)),
        )
        .filter_map(|(map_name, attributes)| {
            attributes
                .fishing_group
                .as_deref()
                .filter(|group| *group != crystal_core::world::fishing::FISHGROUP_NONE)
                .map(|group| (map_name, group))
        })
        .collect();
    if data.fishing.groups.is_empty() {
        for (map_name, group) in referenced_groups {
            diagnostics.push(VerificationError::error(
                "missing_fishing_catalog",
                map_name,
                format!("map references fishing group '{group}' but no fishing catalog is loaded"),
            ));
        }
        return;
    }
    if data.fishing.rod_items.is_empty() {
        diagnostics.push(VerificationError::error(
            "missing_fishing_rod_items",
            "fishing",
            "fishing catalog must declare exact item id to rod rules",
        ));
    }
    let item_ids: BTreeSet<&str> = data.items.keys().map(String::as_str).collect();
    let mut rod_item_ids = BTreeSet::new();
    for rule in &data.fishing.rod_items {
        let subject = format!("fishing:rod_items:{}", rule.item_id);
        if !rod_item_ids.insert(rule.item_id.as_str()) {
            diagnostics.push(VerificationError::error(
                "duplicate_fishing_rod_item_id",
                &subject,
                format!("fishing rod item rules repeat item id '{}'", rule.item_id),
            ));
        }
        if !is_known_fishing_rod(&rule.rod) {
            diagnostics.push(VerificationError::error(
                "unknown_fishing_rod_item_rod",
                &subject,
                format!(
                    "fishing rod item rule references unknown rod '{}'",
                    rule.rod
                ),
            ));
        }
        if !item_ids.contains(rule.item_id.as_str()) {
            diagnostics.push(VerificationError::error(
                "unknown_fishing_rod_item_id",
                &subject,
                format!(
                    "fishing rod item rule references missing item id '{}'",
                    rule.item_id
                ),
            ));
        }
    }
    for (map_name, group) in referenced_groups {
        if !data.fishing.groups.contains_key(group) {
            diagnostics.push(VerificationError::error(
                "unknown_map_fishing_group",
                map_name,
                format!("map references missing fishing group '{group}'"),
            ));
        }
    }
    for (group_id, group) in &data.fishing.groups {
        for (rod, table) in &group.rod_tables {
            if !is_known_fishing_rod(rod) {
                diagnostics.push(VerificationError::error(
                    "unknown_fishing_rod",
                    group_id,
                    format!("fishing group references unknown rod '{rod}'"),
                ));
            }
            for slot in &table.slots {
                if let Some(species) = slot.species.as_deref()
                    && !data.pokemon.contains_key(species)
                {
                    diagnostics.push(VerificationError::error(
                        "unknown_fishing_species",
                        group_id,
                        format!("fishing slot references missing species '{species}'"),
                    ));
                }
                if let Some(time_group) = slot.time_group {
                    let Some(entry) = data.fishing.time_groups.get(time_group) else {
                        diagnostics.push(VerificationError::error(
                            "unknown_fishing_time_group",
                            group_id,
                            format!("fishing slot references missing time group {time_group}"),
                        ));
                        continue;
                    };
                    for species in [&entry.day_species, &entry.night_species] {
                        if !data.pokemon.contains_key(species) {
                            diagnostics.push(VerificationError::error(
                                "unknown_fishing_species",
                                group_id,
                                format!(
                                    "fishing time group references missing species '{species}'"
                                ),
                            ));
                        }
                    }
                }
            }
        }
    }
    let mut seen_swarm_rules = BTreeSet::new();
    for (index, rule) in data.fishing.swarm_rules.iter().enumerate() {
        let subject = format!("fishing:swarm_rules:{index}");
        if rule.daily_flag_bit >= u8::BITS as u8 {
            diagnostics.push(VerificationError::error(
                "invalid_fishing_swarm_flag_bit",
                &subject,
                format!(
                    "fishing swarm rule dailyFlagBit must be 0..=7, found {}",
                    rule.daily_flag_bit
                ),
            ));
        }
        if rule.base_group.trim().is_empty() || rule.base_group.trim() != rule.base_group {
            diagnostics.push(VerificationError::error(
                "invalid_fishing_swarm_base_group",
                &subject,
                "fishing swarm baseGroup must be an exact nonempty fish group id",
            ));
        } else if !data.fishing.groups.contains_key(&rule.base_group) {
            diagnostics.push(VerificationError::error(
                "unknown_fishing_swarm_base_group",
                &subject,
                format!(
                    "fishing swarm rule references missing base group '{}'",
                    rule.base_group
                ),
            ));
        }
        if rule.swarm_group.trim().is_empty() || rule.swarm_group.trim() != rule.swarm_group {
            diagnostics.push(VerificationError::error(
                "invalid_fishing_swarm_group",
                &subject,
                "fishing swarm swarmGroup must be an exact nonempty fish group id",
            ));
        } else if !data.fishing.groups.contains_key(&rule.swarm_group) {
            diagnostics.push(VerificationError::error(
                "unknown_fishing_swarm_group",
                &subject,
                format!(
                    "fishing swarm rule references missing swarm group '{}'",
                    rule.swarm_group
                ),
            ));
        }
        if !seen_swarm_rules.insert((rule.daily_flag_bit, rule.swarm, rule.base_group.as_str())) {
            diagnostics.push(VerificationError::error(
                "duplicate_fishing_swarm_rule",
                &subject,
                "fishing swarm rules must not repeat the same dailyFlagBit, swarm, and baseGroup",
            ));
        }
    }
}

fn verify_field_moves(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    if data.field_moves == FieldMoveCatalog::default() {
        return;
    }
    verify_field_move_block_rule(data, "field_moves:cut", &data.field_moves.cut, diagnostics);
    verify_field_move_block_rule(
        data,
        "field_moves:whirlpool",
        &data.field_moves.whirlpool,
        diagnostics,
    );
    verify_field_move_flag_rule(
        data,
        "field_moves:strength",
        &data.field_moves.strength,
        diagnostics,
    );
    verify_field_move_flag_rule(
        data,
        "field_moves:flash",
        &data.field_moves.flash,
        diagnostics,
    );
    verify_field_move_travel_rule(
        data,
        "field_moves:surf",
        &data.field_moves.surf,
        diagnostics,
    );
    verify_field_move_travel_rule(
        data,
        "field_moves:waterfall",
        &data.field_moves.waterfall,
        diagnostics,
    );
    verify_field_move_rule(data, "field_moves:fly", &data.field_moves.fly, diagnostics);
    verify_field_move_move_rule(data, "field_moves:dig", &data.field_moves.dig, diagnostics);
    verify_field_move_move_rule(
        data,
        "field_moves:teleport",
        &data.field_moves.teleport,
        diagnostics,
    );
    verify_field_escape_item_rule(data, diagnostics);
    verify_field_repel_item_rule(data, diagnostics);
    verify_field_item_rule(
        data,
        "field_moves:bicycle",
        &data.field_moves.bicycle,
        diagnostics,
    );
    verify_field_item_rule(
        data,
        "field_moves:itemfinder",
        &data.field_moves.itemfinder,
        diagnostics,
    );
    verify_field_item_rule(
        data,
        "field_moves:squirtbottle",
        &data.field_moves.squirtbottle,
        diagnostics,
    );
    verify_field_item_rule(
        data,
        "field_moves:coin_case",
        &data.field_moves.coin_case,
        diagnostics,
    );
    verify_field_item_rule(
        data,
        "field_moves:blue_card",
        &data.field_moves.blue_card,
        diagnostics,
    );
    verify_field_item_rule(
        data,
        "field_moves:town_map",
        &data.field_moves.town_map,
        diagnostics,
    );
}

fn verify_field_move_rule(
    data: &GameDataSet,
    subject: &str,
    rule: &FieldMoveRule,
    diagnostics: &mut Vec<VerificationError>,
) {
    verify_field_move_id(data, subject, &rule.move_id, diagnostics);
    verify_field_move_badge(subject, &rule.move_id, &rule.badge, diagnostics);
}

fn verify_field_move_move_rule(
    data: &GameDataSet,
    subject: &str,
    rule: &FieldMoveMoveRule,
    diagnostics: &mut Vec<VerificationError>,
) {
    verify_field_move_id(data, subject, &rule.move_id, diagnostics);
}

fn verify_field_escape_item_rule(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    let rule = &data.field_moves.escape_rope;
    if rule.item_id.trim().is_empty() || rule.item_id.trim() != rule.item_id {
        diagnostics.push(VerificationError::error(
            "invalid_field_escape_item_id",
            "field_moves:escape_rope",
            "field escape item id must be exact and nonempty",
        ));
    }
    if rule.escape_rope_mode.trim().is_empty()
        || rule.escape_rope_mode.trim() != rule.escape_rope_mode
    {
        diagnostics.push(VerificationError::error(
            "invalid_field_escape_item_mode",
            "field_moves:escape_rope",
            "field escape item mode must be exact and nonempty",
        ));
    }
    if data.items.is_empty() || rule.item_id.is_empty() || rule.escape_rope_mode.is_empty() {
        return;
    }
    match data.items.get(&rule.item_id) {
        Some(item) if item.escape_rope_mode.as_deref() == Some(rule.escape_rope_mode.as_str()) => {}
        _ => {
            diagnostics.push(VerificationError::error(
                "unknown_field_escape_item_rule",
                "field_moves:escape_rope",
                format!(
                    "field escape item rule references item '{}' with mode '{}' not implemented by the item payload",
                    rule.item_id, rule.escape_rope_mode
                ),
            ));
        }
    }
}

fn verify_field_repel_item_rule(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    if data.items.is_empty() {
        return;
    }
    if !data.items.values().any(|item| item.repel_steps.is_some()) {
        diagnostics.push(VerificationError::error(
            "missing_field_repel_item_payload",
            "field_moves:repel",
            "field repel behavior requires at least one item with repel_steps",
        ));
    }
}

fn verify_field_item_rule(
    data: &GameDataSet,
    subject: &str,
    rule: &FieldItemRule,
    diagnostics: &mut Vec<VerificationError>,
) {
    if rule.item_id.trim().is_empty() || rule.item_id.trim() != rule.item_id {
        diagnostics.push(VerificationError::error(
            "invalid_field_item_id",
            subject,
            "field item id must be exact and nonempty",
        ));
        return;
    }
    if !data.items.contains_key(&rule.item_id) {
        diagnostics.push(VerificationError::error(
            "unknown_field_item_id",
            subject,
            format!("field item rule references unknown item '{}'", rule.item_id),
        ));
    }
}

fn verify_field_move_id(
    data: &GameDataSet,
    subject: &str,
    move_id: &str,
    diagnostics: &mut Vec<VerificationError>,
) {
    if move_id.trim().is_empty() || move_id.trim() != move_id {
        diagnostics.push(VerificationError::error(
            "invalid_field_move_id",
            subject,
            "field move id must be an exact nonempty move id",
        ));
    } else if !data.moves.contains_key(move_id) {
        diagnostics.push(VerificationError::error(
            "unknown_field_move_id",
            subject,
            format!("field move references missing move '{move_id}'"),
        ));
    }
}

fn verify_field_move_badge(
    subject: &str,
    move_id: &str,
    badge: &crystal_core::systems::field_moves::FieldMoveBadgeRequirement,
    diagnostics: &mut Vec<VerificationError>,
) {
    if badge.region != "johto" {
        diagnostics.push(VerificationError::error(
            "invalid_field_move_badge_region",
            subject,
            format!(
                "field move '{move_id}' badge region must be exact 'johto', found '{}'",
                badge.region
            ),
        ));
    }
    if badge.index >= 8 {
        diagnostics.push(VerificationError::error(
            "invalid_field_move_badge_index",
            subject,
            format!(
                "field move '{move_id}' Johto badge index must be 0..=7, found {}",
                badge.index
            ),
        ));
    }
}

fn verify_field_move_block_rule(
    data: &GameDataSet,
    subject: &str,
    rule: &FieldMoveBlockRule,
    diagnostics: &mut Vec<VerificationError>,
) {
    verify_field_move_id(data, subject, &rule.move_id, diagnostics);
    verify_field_move_badge(subject, &rule.move_id, &rule.badge, diagnostics);
    if rule.target_collisions.is_empty() {
        diagnostics.push(VerificationError::error(
            "missing_field_move_target_collisions",
            subject,
            format!(
                "field move '{}' requires exact target collisions",
                rule.move_id
            ),
        ));
    }
    if rule.replacements.is_empty() {
        diagnostics.push(VerificationError::error(
            "missing_field_move_replacements",
            subject,
            format!(
                "field move '{}' requires exact replacement rows",
                rule.move_id
            ),
        ));
    }
    let mut seen = BTreeSet::new();
    for (index, replacement) in rule.replacements.iter().enumerate() {
        let replacement_subject = format!("{subject}:replacement:{index}");
        if replacement.tileset.trim().is_empty()
            || replacement.tileset.trim() != replacement.tileset
        {
            diagnostics.push(VerificationError::error(
                "invalid_field_move_replacement_tileset",
                &replacement_subject,
                "field move replacement tileset must be exact and nonempty",
            ));
        }
        if replacement.variant.trim().is_empty()
            || replacement.variant.trim() != replacement.variant
        {
            diagnostics.push(VerificationError::error(
                "invalid_field_move_replacement_variant",
                &replacement_subject,
                "field move replacement variant must be exact and nonempty",
            ));
        }
        if !seen.insert((replacement.tileset.as_str(), replacement.block_id)) {
            diagnostics.push(VerificationError::error(
                "duplicate_field_move_replacement",
                &replacement_subject,
                "field move replacements must not repeat the same tileset and block id",
            ));
        }
    }
}

fn verify_field_move_flag_rule(
    data: &GameDataSet,
    subject: &str,
    rule: &FieldMoveFlagRule,
    diagnostics: &mut Vec<VerificationError>,
) {
    verify_field_move_id(data, subject, &rule.move_id, diagnostics);
    verify_field_move_badge(subject, &rule.move_id, &rule.badge, diagnostics);
    if rule.engine_flag.trim().is_empty() || rule.engine_flag.trim() != rule.engine_flag {
        diagnostics.push(VerificationError::error(
            "invalid_field_move_engine_flag",
            subject,
            format!(
                "field move '{}' requires an exact engine flag",
                rule.move_id
            ),
        ));
    }
}

fn verify_field_move_travel_rule(
    data: &GameDataSet,
    subject: &str,
    rule: &FieldMoveTravelRule,
    diagnostics: &mut Vec<VerificationError>,
) {
    verify_field_move_id(data, subject, &rule.move_id, diagnostics);
    verify_field_move_badge(subject, &rule.move_id, &rule.badge, diagnostics);
    if subject.ends_with(":waterfall") && rule.target_collisions.is_empty() {
        diagnostics.push(VerificationError::error(
            "missing_field_move_target_collisions",
            subject,
            "waterfall requires exact target collisions",
        ));
    }
}

fn verify_progression_rules(
    data: &GameDataSet,
    map_names: &BTreeSet<String>,
    rules: &PlayabilityRules,
    diagnostics: &mut Vec<VerificationError>,
) {
    let item_ids: BTreeSet<&str> = data.items.keys().map(String::as_str).collect();
    let mut rule_ids = BTreeSet::new();
    for item in rules
        .initial_items
        .iter()
        .chain(rules.goal_items.iter())
        .chain(
            rules
                .progression_rules
                .iter()
                .flat_map(|rule| rule.requires.items.iter().chain(rule.grants.items.iter())),
        )
        .chain(
            rules
                .map_access
                .iter()
                .flat_map(|rule| rule.requires.items.iter()),
        )
    {
        if !item_ids.contains(item.as_str()) {
            diagnostics.push(VerificationError::error(
                "unknown_progression_item",
                item,
                "progression rule references an item that is not loaded",
            ));
        }
    }
    for map in rules
        .goal_maps
        .iter()
        .chain(
            rules
                .progression_rules
                .iter()
                .flat_map(|rule| rule.requires.maps.iter().chain(rule.grants.maps.iter())),
        )
        .chain(rules.map_access.iter().map(|rule| &rule.map))
        .chain(
            rules
                .map_access
                .iter()
                .flat_map(|rule| rule.requires.maps.iter()),
        )
    {
        if !map_names.contains(map) {
            diagnostics.push(VerificationError::error(
                "unknown_progression_map",
                map,
                "progression rule references a map that is not loaded",
            ));
        }
    }
    for rule in &rules.progression_rules {
        if rule.id.trim().is_empty() {
            diagnostics.push(VerificationError::error(
                "missing_progression_rule_id",
                "playability",
                "progression rules require explicit ids",
            ));
        } else if !rule_ids.insert(rule.id.as_str()) {
            diagnostics.push(VerificationError::error(
                "duplicate_progression_rule_id",
                &rule.id,
                "progression rule ids must be unique",
            ));
        }
        if rule.requires.is_empty()
            && rule.grants.events.is_empty()
            && rule.grants.items.is_empty()
            && rule.grants.maps.is_empty()
        {
            diagnostics.push(VerificationError::error(
                "empty_progression_rule",
                &rule.id,
                "progression rule must require or grant at least one fact",
            ));
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct PlayabilityGraph {
    components: BTreeMap<String, usize>,
    start_states: Vec<(String, usize)>,
    edges: Vec<ComponentGraphEdge>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ComponentGraphEdge {
    from_map: String,
    from_component: usize,
    to_map: String,
    to_component: usize,
    kind: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ValidationMapModule {
    id: String,
    attributes: MapAttributes,
    events: MapEvents,
    blocks: Vec<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MapPlayabilityContext {
    map: OverworldMapData,
    component_by_tile: BTreeMap<(i16, i16), usize>,
    component_count: usize,
}

impl MapPlayabilityContext {
    fn component_at(&self, tile: TilePosition) -> Option<usize> {
        self.component_by_tile.get(&(tile.x, tile.y)).copied()
    }
}

fn map_playability_context(
    asset_root: &AssetRoot,
    module: &MapModule,
    rules: &PlayabilityRules,
    diagnostics: &mut Vec<VerificationError>,
) -> Option<MapPlayabilityContext> {
    map_playability_context_from_parts(
        asset_root,
        &module.id,
        &module.attributes,
        module.blocks.clone(),
        rules,
        diagnostics,
    )
}

fn map_playability_context_from_parts(
    asset_root: &AssetRoot,
    map_name: &str,
    attributes: &MapAttributes,
    blocks: Vec<u16>,
    rules: &PlayabilityRules,
    diagnostics: &mut Vec<VerificationError>,
) -> Option<MapPlayabilityContext> {
    let map = OverworldMapData::from_attributes(map_name, attributes, blocks);
    let tileset = match asset_root.load_tileset_collision(&attributes.tileset_name) {
        Ok(tileset) => tileset,
        Err(error) => {
            diagnostics.push(map_validation_diagnostic(
                rules,
                "missing_tileset_collision",
                map_name,
                error.to_string(),
            ));
            return None;
        }
    };
    let (width, height) = map.tile_bounds();
    let mut component_by_tile = BTreeMap::new();
    let mut component_count = 0_usize;
    for y in 0..height as i16 {
        for x in 0..width as i16 {
            let start = TilePosition::new(x, y);
            if component_by_tile.contains_key(&(x, y))
                || !is_walkable_validation_tile(&map, &tileset, start)
            {
                continue;
            }
            let component = component_count;
            component_count += 1;
            let mut queue = VecDeque::from([start]);
            component_by_tile.insert((start.x, start.y), component);
            while let Some(tile) = queue.pop_front() {
                for direction in [
                    Direction::Down,
                    Direction::Up,
                    Direction::Left,
                    Direction::Right,
                ] {
                    let next = tile.moved(direction);
                    if component_by_tile.contains_key(&(next.x, next.y))
                        || !can_enter_tile(
                            &map,
                            &tileset,
                            next,
                            direction,
                            PlayerTraversalState::Walk,
                        )
                    {
                        continue;
                    }
                    component_by_tile.insert((next.x, next.y), component);
                    queue.push_back(next);
                }
            }
        }
    }
    Some(MapPlayabilityContext {
        map,
        component_by_tile,
        component_count,
    })
}

fn is_walkable_validation_tile(
    map: &OverworldMapData,
    tileset: &TilesetCollision,
    tile: TilePosition,
) -> bool {
    sample_collision(map, tileset, tile)
        .map(|sample| {
            is_permission_passable(
                sample.permission,
                Direction::Down,
                PlayerTraversalState::Walk,
            )
        })
        .unwrap_or(false)
}

fn connection_source_tile(
    context: &MapPlayabilityContext,
    connection: &MapConnection,
) -> Option<TilePosition> {
    let (width, height) = context.map.tile_bounds();
    match connection.direction.as_str() {
        "north" => (0..width as i16)
            .map(|x| TilePosition::new(x, 0))
            .find(|tile| context.component_at(*tile).is_some()),
        "south" => (0..width as i16)
            .map(|x| TilePosition::new(x, height as i16 - 1))
            .find(|tile| context.component_at(*tile).is_some()),
        "west" => (0..height as i16)
            .map(|y| TilePosition::new(0, y))
            .find(|tile| context.component_at(*tile).is_some()),
        "east" => (0..height as i16)
            .map(|y| TilePosition::new(width as i16 - 1, y))
            .find(|tile| context.component_at(*tile).is_some()),
        _ => None,
    }
}

fn connection_source_component(
    context: &MapPlayabilityContext,
    connection: &MapConnection,
) -> Option<usize> {
    connection_source_tile(context, connection).and_then(|tile| context.component_at(tile))
}

fn connection_trigger_tile(
    context: &MapPlayabilityContext,
    connection: &MapConnection,
) -> TilePosition {
    let source = connection_source_tile(context, connection)
        .expect("connection source component was checked before trigger tile resolution");
    match connection.direction.as_str() {
        "north" => TilePosition::new(source.x, -1),
        "south" => TilePosition::new(source.x, context.map.tile_bounds().1 as i16),
        "west" => TilePosition::new(-1, source.y),
        "east" => TilePosition::new(context.map.tile_bounds().0 as i16, source.y),
        _ => source,
    }
}

fn verify_maps(
    asset_root: &AssetRoot,
    data: &GameDataSet,
    map_names: &BTreeSet<String>,
    rules: &PlayabilityRules,
    diagnostics: &mut Vec<VerificationError>,
) -> PlayabilityGraph {
    let constants = map_constants(data);
    let mut context_cache: BTreeMap<String, Option<Rc<MapPlayabilityContext>>> = BTreeMap::new();
    let mut graph = PlayabilityGraph::default();
    for map_name in map_names {
        let Some(module) = validation_map_for_playability(data, map_name, rules, diagnostics)
        else {
            continue;
        };
        let context = cached_map_playability_context_for_parts(
            &mut context_cache,
            asset_root,
            &module.id,
            &module.attributes,
            module.blocks.clone(),
            rules,
            diagnostics,
        );
        if let Some(context) = &context {
            graph
                .components
                .insert(map_name.clone(), context.component_count);
            for start in rules
                .start_tiles
                .iter()
                .filter(|start| start.map == *map_name)
            {
                if let Some(component) = context.component_at(start.tile) {
                    graph.start_states.push((map_name.clone(), component));
                } else {
                    diagnostics.push(VerificationError::error(
                        "invalid_start_tile",
                        map_name,
                        format!(
                            "start tile ({}, {}) is not walkable under map collision",
                            start.tile.x, start.tile.y
                        ),
                    ));
                }
            }
        }
        let expected_blocks = module.attributes.width as usize * module.attributes.height as usize;
        if expected_blocks == 0 {
            diagnostics.push(VerificationError::error(
                "empty_map_dimensions",
                map_name,
                "map width and height must both be greater than zero",
            ));
        }
        if module.blocks.len() != expected_blocks && !module.blocks.is_empty() {
            diagnostics.push(VerificationError::error(
                "wrong_map_block_count",
                map_name,
                format!(
                    "map has {} blocks but dimensions require {expected_blocks}",
                    module.blocks.len()
                ),
            ));
        }
        if rules.require_walkable_maps {
            verify_walkable_map(
                &module.id,
                context.as_ref().map(|context| context.as_ref()),
                rules,
                diagnostics,
            );
        }
        for connection in &module.attributes.connections {
            if !map_names.contains(&connection.target_map) {
                diagnostics.push(VerificationError::error(
                    "unknown_connection_target",
                    map_name,
                    format!(
                        "connection references missing map '{}'",
                        connection.target_map
                    ),
                ));
                continue;
            }
            let Some(source_context) = context.as_ref() else {
                continue;
            };
            let Some(source_component) = connection_source_component(source_context, connection)
            else {
                diagnostics.push(transition_diagnostic(
                    rules,
                    "unreachable_connection",
                    map_name,
                    format!(
                        "connection to '{}' has no reachable walkable border tile",
                        connection.target_map
                    ),
                ));
                continue;
            };
            let Some(target_attributes) =
                map_attributes_for_validation(data, &connection.target_map, rules, diagnostics)
            else {
                continue;
            };
            let destination_tile = match connection_destination_tile(
                connection_trigger_tile(source_context, connection),
                &connection.direction,
                connection.offset,
                target_attributes,
            ) {
                Ok(tile) => tile,
                Err(error) => {
                    diagnostics.push(transition_diagnostic(
                        rules,
                        "invalid_connection_transition",
                        map_name,
                        error.to_string(),
                    ));
                    continue;
                }
            };
            let target_context = cached_map_playability_context_for_map(
                &mut context_cache,
                asset_root,
                data,
                &connection.target_map,
                rules,
                diagnostics,
            );
            let Some(target_component) = target_context
                .as_ref()
                .and_then(|context| context.component_at(destination_tile))
            else {
                diagnostics.push(transition_diagnostic(
                    rules,
                    "unreachable_connection_destination",
                    map_name,
                    format!(
                        "connection to '{}' lands on an unwalkable tile",
                        connection.target_map
                    ),
                ));
                continue;
            };
            graph.edges.push(ComponentGraphEdge {
                from_map: map_name.clone(),
                from_component: source_component,
                to_map: connection.target_map.clone(),
                to_component: target_component,
                kind: "connection".to_string(),
            });
        }
        for warp in &module.events.warps {
            let Some(target_map) = constants.get(&warp.target_map_constant) else {
                diagnostics.push(VerificationError::error(
                    "unknown_warp_target",
                    map_name,
                    format!(
                        "warp {} references unknown map constant '{}'",
                        warp.index, warp.target_map_constant
                    ),
                ));
                continue;
            };
            if warp.target_warp_id < 1 {
                continue;
            }
            let Some(target_warp) =
                target_warp(data, target_map, warp.target_warp_id, rules, diagnostics)
            else {
                diagnostics.push(transition_diagnostic(
                    rules,
                    "unknown_warp_index",
                    map_name,
                    format!(
                        "warp {} targets missing warp id {} on {}",
                        warp.index, warp.target_warp_id, target_map
                    ),
                ));
                continue;
            };
            let source_tile = warp_tile_position(warp);
            let Some(source_component) = context
                .as_ref()
                .and_then(|context| context.component_at(source_tile))
            else {
                diagnostics.push(transition_diagnostic(
                    rules,
                    "unreachable_warp",
                    map_name,
                    format!("warp {} is not on a reachable walkable tile", warp.index),
                ));
                continue;
            };
            let target_context = cached_map_playability_context_for_map(
                &mut context_cache,
                asset_root,
                data,
                target_map,
                rules,
                diagnostics,
            );
            let target_tile = warp_tile_position(&target_warp);
            let Some(target_component) = target_context
                .as_ref()
                .and_then(|context| context.component_at(target_tile))
            else {
                diagnostics.push(transition_diagnostic(
                    rules,
                    "unreachable_warp_destination",
                    map_name,
                    format!(
                        "warp {} lands on an unwalkable tile on {}",
                        warp.index, target_map
                    ),
                ));
                continue;
            };
            graph.edges.push(ComponentGraphEdge {
                from_map: map_name.clone(),
                from_component: source_component,
                to_map: target_map.clone(),
                to_component: target_component,
                kind: "warp".to_string(),
            });
        }
    }
    graph
}

fn transition_diagnostic(
    rules: &PlayabilityRules,
    code: impl Into<String>,
    map_name: &str,
    message: impl Into<String>,
) -> VerificationError {
    if rules.require_all_maps_reachable || playability_mentions_map(rules, map_name) {
        VerificationError::error(code, map_name, message)
    } else {
        VerificationError::warning(code, map_name, message)
    }
}

fn map_validation_diagnostic(
    rules: &PlayabilityRules,
    code: impl Into<String>,
    map_name: &str,
    message: impl Into<String>,
) -> VerificationError {
    if rules.require_all_maps_reachable || playability_mentions_map(rules, map_name) {
        VerificationError::error(code, map_name, message)
    } else {
        VerificationError::warning(code, map_name, message)
    }
}

fn playability_mentions_map(rules: &PlayabilityRules, map_name: &str) -> bool {
    rules.start_maps.iter().any(|map| map == map_name)
        || rules.start_tiles.iter().any(|start| start.map == map_name)
        || rules.goal_maps.iter().any(|map| map == map_name)
        || rules.map_access.iter().any(|access| access.map == map_name)
}

fn verify_walkable_map(
    map_name: &str,
    context: Option<&MapPlayabilityContext>,
    rules: &PlayabilityRules,
    diagnostics: &mut Vec<VerificationError>,
) {
    if context
        .map(|context| context.component_count == 0)
        .unwrap_or(true)
    {
        diagnostics.push(map_validation_diagnostic(
            rules,
            "unwalkable_map",
            map_name,
            "map has no walkable tile under its tileset collision",
        ));
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct ProgressionState {
    maps: BTreeSet<String>,
    events: BTreeSet<String>,
    items: BTreeSet<String>,
}

fn solve_progression(
    reachable_maps: &[String],
    loaded_maps: &BTreeSet<String>,
    rules: &PlayabilityRules,
) -> ProgressionState {
    let physical_maps: BTreeSet<String> = reachable_maps.iter().cloned().collect();
    let mut state = ProgressionState {
        events: rules.initial_events.iter().cloned().collect(),
        items: rules.initial_items.iter().cloned().collect(),
        ..ProgressionState::default()
    };
    let mut applied_rules = BTreeSet::new();
    loop {
        let mut changed = false;
        for map in &physical_maps {
            if !state.maps.contains(map) && map_accessible(map, &state, rules) {
                state.maps.insert(map.clone());
                changed = true;
            }
        }
        for rule in &rules.progression_rules {
            if applied_rules.contains(&rule.id) || !requirements_met(&rule.requires, &state) {
                continue;
            }
            applied_rules.insert(rule.id.clone());
            for event in &rule.grants.events {
                changed |= state.events.insert(event.clone());
            }
            for item in &rule.grants.items {
                changed |= state.items.insert(item.clone());
            }
            for map in &rule.grants.maps {
                if loaded_maps.contains(map) {
                    changed |= state.maps.insert(map.clone());
                }
            }
        }
        if !changed {
            break;
        }
    }
    state
}

fn map_accessible(map: &str, state: &ProgressionState, rules: &PlayabilityRules) -> bool {
    rules
        .map_access
        .iter()
        .filter(|rule| rule.map == map)
        .all(|rule| requirements_met(&rule.requires, state))
}

fn requirements_met(requirements: &ProgressionRequirements, state: &ProgressionState) -> bool {
    requirements
        .events
        .iter()
        .all(|event| state.events.contains(event))
        && requirements
            .items
            .iter()
            .all(|item| state.items.contains(item))
        && requirements.maps.iter().all(|map| state.maps.contains(map))
}

fn verify_solubility(
    map_names: &BTreeSet<String>,
    reachable_maps: &[String],
    progression: &ProgressionState,
    loaded_progression: &ProgressionState,
    rules: &PlayabilityRules,
    diagnostics: &mut Vec<VerificationError>,
) {
    if rules.start_maps.is_empty()
        && rules.start_tiles.is_empty()
        && (!rules.goal_maps.is_empty()
            || !rules.goal_events.is_empty()
            || !rules.goal_items.is_empty()
            || rules.require_all_maps_reachable)
    {
        diagnostics.push(VerificationError::error(
            "missing_start_map",
            "playability",
            "playability rules require at least one explicit start map",
        ));
    }
    for start in &rules.start_maps {
        if !map_names.contains(start) {
            diagnostics.push(VerificationError::error(
                "unknown_start_map",
                start,
                "playability start map is not loaded",
            ));
        }
    }
    for start in &rules.start_tiles {
        if !map_names.contains(&start.map) {
            diagnostics.push(VerificationError::error(
                "unknown_start_map",
                &start.map,
                "playability start tile map is not loaded",
            ));
        }
    }
    let reachable: BTreeSet<&str> = reachable_maps.iter().map(String::as_str).collect();
    for goal in &rules.goal_maps {
        if !map_names.contains(goal) {
            diagnostics.push(VerificationError::error(
                "unknown_goal_map",
                goal,
                "playability goal map is not loaded",
            ));
        } else if !reachable.contains(goal.as_str()) {
            diagnostics.push(VerificationError::error(
                "unreachable_goal_map",
                goal,
                "playability goal map cannot be reached from the configured starts",
            ));
        } else if !progression.maps.contains(goal) {
            diagnostics.push(VerificationError::error(
                "unsolved_goal_map",
                goal,
                "playability goal map is physically reachable but blocked by progression rules",
            ));
        }
    }
    for goal in &rules.goal_events {
        if !loaded_progression.events.contains(goal) {
            diagnostics.push(VerificationError::error(
                "unsolved_goal_event",
                goal,
                "playability goal event cannot be produced by progression rules",
            ));
        }
    }
    for goal in &rules.goal_items {
        if !loaded_progression.items.contains(goal) {
            diagnostics.push(VerificationError::error(
                "unsolved_goal_item",
                goal,
                "playability goal item cannot be produced by progression rules",
            ));
        }
    }
    if rules.require_all_maps_reachable {
        for map_name in map_names {
            if !reachable.contains(map_name.as_str()) {
                diagnostics.push(VerificationError::error(
                    "unreachable_map",
                    map_name,
                    "map cannot be reached from the configured starts",
                ));
            } else if !progression.maps.contains(map_name) {
                diagnostics.push(VerificationError::error(
                    "unsolved_map",
                    map_name,
                    "map is physically reachable but blocked by progression rules",
                ));
            }
        }
    }
}

fn runtime_module_script_subset<'a>(
    all_scripts: &BTreeMap<String, Value>,
    seeds: impl IntoIterator<Item = &'a str>,
) -> BTreeMap<String, Value> {
    let mut scripts = BTreeMap::new();
    let mut pending: Vec<String> = seeds.into_iter().map(str::to_string).collect();
    while let Some(label) = pending.pop() {
        if scripts.contains_key(&label) {
            continue;
        }
        let Some(payload) = all_scripts.get(&label) else {
            continue;
        };
        scripts.insert(label.clone(), payload.clone());
        for reference in script_payload_references(&label, payload, all_scripts) {
            if !scripts.contains_key(&reference) {
                pending.push(reference);
            }
        }
    }
    scripts
}

fn script_payload_references(
    current_label: &str,
    payload: &Value,
    all_scripts: &BTreeMap<String, Value>,
) -> Vec<String> {
    let Some(commands) = payload.as_array() else {
        return Vec::new();
    };
    let mut references = Vec::new();
    for command in commands {
        let Some(args) = command.get("args").and_then(Value::as_array) else {
            continue;
        };
        for arg in args.iter().filter_map(Value::as_str) {
            if all_scripts.contains_key(arg) {
                references.push(arg.to_string());
                continue;
            }
            if arg.starts_with('.') {
                let parent_label = script_label_parent(current_label);
                let scoped = format!("{arg}@{parent_label}");
                if all_scripts.contains_key(&scoped) {
                    references.push(scoped);
                }
            }
        }
    }
    references
}

fn all_map_names(data: &GameDataSet) -> BTreeSet<String> {
    data.map_attributes
        .keys()
        .chain(data.maps.keys())
        .cloned()
        .collect()
}

fn map_constants(data: &GameDataSet) -> BTreeMap<String, String> {
    data.map_attributes
        .iter()
        .filter_map(|(map_name, attributes)| {
            attributes
                .map_constant
                .as_ref()
                .map(|constant| (constant.clone(), map_name.clone()))
        })
        .chain(data.maps.iter().filter_map(|(map_name, module)| {
            module
                .attributes
                .map_constant
                .as_ref()
                .map(|constant| (constant.clone(), map_name.clone()))
        }))
        .collect()
}

fn validation_map_for_playability(
    data: &GameDataSet,
    map_name: &str,
    rules: &PlayabilityRules,
    diagnostics: &mut Vec<VerificationError>,
) -> Option<ValidationMapModule> {
    if let Some(module) = data.maps.get(map_name) {
        return Some(ValidationMapModule {
            id: module.id.clone(),
            attributes: module.attributes.clone(),
            events: module.events.clone(),
            blocks: module.blocks.clone(),
        });
    }
    let attributes = map_attributes_for_validation(data, map_name, rules, diagnostics)?.clone();
    let events = map_events_for_validation(data, map_name, &attributes, rules, diagnostics)?;
    let blocks = map_blocks_for_validation(data, map_name, &attributes, rules, diagnostics)?;
    Some(ValidationMapModule {
        id: map_name.to_string(),
        attributes,
        events,
        blocks,
    })
}

fn cached_map_playability_context_for_parts(
    cache: &mut BTreeMap<String, Option<Rc<MapPlayabilityContext>>>,
    asset_root: &AssetRoot,
    map_name: &str,
    attributes: &MapAttributes,
    blocks: Vec<u16>,
    rules: &PlayabilityRules,
    diagnostics: &mut Vec<VerificationError>,
) -> Option<Rc<MapPlayabilityContext>> {
    if let Some(context) = cache.get(map_name) {
        return context.clone();
    }
    let context = map_playability_context_from_parts(
        asset_root,
        map_name,
        attributes,
        blocks,
        rules,
        diagnostics,
    )
    .map(Rc::new);
    cache.insert(map_name.to_string(), context.clone());
    context
}

fn cached_map_playability_context_for_map(
    cache: &mut BTreeMap<String, Option<Rc<MapPlayabilityContext>>>,
    asset_root: &AssetRoot,
    data: &GameDataSet,
    map_name: &str,
    rules: &PlayabilityRules,
    diagnostics: &mut Vec<VerificationError>,
) -> Option<Rc<MapPlayabilityContext>> {
    if let Some(context) = cache.get(map_name) {
        return context.clone();
    }
    let context = if let Some(module) = data.maps.get(map_name) {
        map_playability_context(asset_root, module, rules, diagnostics)
    } else {
        let attributes = map_attributes_for_validation(data, map_name, rules, diagnostics)?;
        let blocks = map_blocks_for_validation(data, map_name, attributes, rules, diagnostics)?;
        map_playability_context_from_parts(
            asset_root,
            map_name,
            attributes,
            blocks,
            rules,
            diagnostics,
        )
    }
    .map(Rc::new);
    cache.insert(map_name.to_string(), context.clone());
    context
}

fn map_events_for_validation(
    data: &GameDataSet,
    map_name: &str,
    attributes: &MapAttributes,
    rules: &PlayabilityRules,
    diagnostics: &mut Vec<VerificationError>,
) -> Option<MapEvents> {
    let Some(events_label) = attributes
        .map_events_label
        .as_deref()
        .filter(|label| !label.trim().is_empty())
    else {
        diagnostics.push(map_validation_diagnostic(
            rules,
            "unassemblable_map",
            map_name,
            format!("missing map_events_label for map {map_name}"),
        ));
        return None;
    };
    let Some(events_payload) = data.map_scripts.get(events_label) else {
        diagnostics.push(map_validation_diagnostic(
            rules,
            "unassemblable_map",
            map_name,
            format!("missing map events label {events_label}"),
        ));
        return None;
    };
    match parse_map_events(map_name, events_payload) {
        Ok(events) => Some(events),
        Err(error) => {
            diagnostics.push(map_validation_diagnostic(
                rules,
                "unassemblable_map",
                map_name,
                error.to_string(),
            ));
            None
        }
    }
}

fn map_attributes_for_validation<'a>(
    data: &'a GameDataSet,
    map_name: &str,
    rules: &PlayabilityRules,
    diagnostics: &mut Vec<VerificationError>,
) -> Option<&'a MapAttributes> {
    if let Some(module) = data.maps.get(map_name) {
        return Some(&module.attributes);
    }
    let attributes = data.map_attributes.get(map_name);
    if attributes.is_none() {
        diagnostics.push(map_validation_diagnostic(
            rules,
            "unassemblable_map",
            map_name,
            format!("missing map attributes for {map_name}"),
        ));
    }
    attributes
}

fn map_blocks_for_validation(
    data: &GameDataSet,
    map_name: &str,
    attributes: &MapAttributes,
    rules: &PlayabilityRules,
    diagnostics: &mut Vec<VerificationError>,
) -> Option<Vec<u16>> {
    let Some(blocks_label) = attributes
        .blocks_label
        .as_deref()
        .filter(|label| !label.trim().is_empty())
    else {
        diagnostics.push(map_validation_diagnostic(
            rules,
            "unassemblable_map",
            map_name,
            format!("missing blocks_label for map {map_name}"),
        ));
        return None;
    };
    let Some(encoded_blocks) = data.map_blocks.get(blocks_label) else {
        diagnostics.push(map_validation_diagnostic(
            rules,
            "unassemblable_map",
            map_name,
            format!("missing map block payload {blocks_label}"),
        ));
        return None;
    };
    match decode_base64_bytes(encoded_blocks) {
        Ok(blocks) => Some(blocks.into_iter().map(u16::from).collect()),
        Err(error) => {
            diagnostics.push(map_validation_diagnostic(
                rules,
                "unassemblable_map",
                map_name,
                format!("decode map block payload {blocks_label}: {error}"),
            ));
            None
        }
    }
}

fn target_warp(
    data: &GameDataSet,
    target_map: &str,
    target_warp_id: i16,
    rules: &PlayabilityRules,
    diagnostics: &mut Vec<VerificationError>,
) -> Option<WarpEvent> {
    if target_warp_id < 1 {
        return None;
    }
    if let Some(module) = data.maps.get(target_map) {
        return module
            .events
            .warps
            .get(target_warp_id as usize - 1)
            .cloned();
    }
    let Some(attributes) = data.map_attributes.get(target_map) else {
        diagnostics.push(map_validation_diagnostic(
            rules,
            "unassemblable_map",
            target_map,
            format!("missing map attributes for {target_map}"),
        ));
        return None;
    };
    let Some(events_label) = attributes
        .map_events_label
        .as_deref()
        .filter(|label| !label.trim().is_empty())
    else {
        diagnostics.push(map_validation_diagnostic(
            rules,
            "unassemblable_map",
            target_map,
            format!("missing map_events_label for map {target_map}"),
        ));
        return None;
    };
    let Some(events_payload) = data.map_scripts.get(events_label) else {
        diagnostics.push(map_validation_diagnostic(
            rules,
            "unassemblable_map",
            target_map,
            format!("missing map events label {events_label}"),
        ));
        return None;
    };
    match parse_map_events(target_map, events_payload) {
        Ok(events) => events.warps.get(target_warp_id as usize - 1).cloned(),
        Err(error) => {
            diagnostics.push(map_validation_diagnostic(
                rules,
                "unassemblable_map",
                target_map,
                error.to_string(),
            ));
            None
        }
    }
}

fn reachable_maps(
    map_names: &BTreeSet<String>,
    graph: &PlayabilityGraph,
    rules: &PlayabilityRules,
) -> Vec<String> {
    let mut adjacency: BTreeMap<(&str, usize), Vec<(&str, usize)>> = BTreeMap::new();
    for edge in &graph.edges {
        adjacency
            .entry((edge.from_map.as_str(), edge.from_component))
            .or_default()
            .push((edge.to_map.as_str(), edge.to_component));
    }

    let mut seen_states = BTreeSet::new();
    let mut queue = VecDeque::new();
    if rules.start_tiles.is_empty() {
        let start_maps: Vec<String> = rules
            .start_maps
            .iter()
            .filter(|map_name| map_names.contains(*map_name))
            .cloned()
            .collect();
        for map_name in start_maps {
            let component_count = graph.components.get(&map_name).copied().unwrap_or(0);
            for component in 0..component_count {
                queue.push_back((map_name.clone(), component));
            }
        }
    } else {
        for (map_name, component) in &graph.start_states {
            queue.push_back((map_name.clone(), *component));
        }
    }

    while let Some((map_name, component)) = queue.pop_front() {
        if !seen_states.insert((map_name.clone(), component)) {
            continue;
        }
        for (next_map, next_component) in adjacency
            .get(&(map_name.as_str(), component))
            .into_iter()
            .flatten()
        {
            let next = ((*next_map).to_string(), *next_component);
            if !seen_states.contains(&next) {
                queue.push_back(next);
            }
        }
    }

    seen_states
        .into_iter()
        .map(|(map_name, _)| map_name)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn encounter_species(data: &WildEncounterData) -> BTreeSet<String> {
    let mut species = BTreeSet::new();
    for table in [data.grass.as_ref(), data.water.as_ref()]
        .into_iter()
        .flatten()
    {
        for encounter in table
            .morning
            .iter()
            .chain(table.day.iter())
            .chain(table.night.iter())
        {
            species.insert(encounter.species.clone());
        }
    }
    species
}

fn field_encounter_species(data: &FieldEncounterData) -> BTreeSet<String> {
    let mut species = BTreeSet::new();
    for table in [data.headbutt.as_ref(), data.rock_smash.as_ref()]
        .into_iter()
        .flatten()
    {
        for encounter in table.common.iter().chain(table.rare.iter()) {
            species.insert(encounter.species.clone());
        }
    }
    species
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GameDataSet {
    pub pokemon: BTreeMap<String, PokemonSpecies>,
    pub moves: BTreeMap<String, Move>,
    pub growth_rates: crystal_core::systems::experience::GrowthRateCatalog,
    pub learnsets: SpeciesLearnsets,
    pub level_up_moves: BTreeMap<String, Value>,
    pub egg_moves: BTreeMap<String, Value>,
    pub evolutions: EvolutionTable,
    pub maps: BTreeMap<String, MapModule>,
    pub map_scripts: BTreeMap<String, Value>,
    pub map_attributes: BTreeMap<String, MapAttributes>,
    pub map_dimensions: BTreeMap<String, Value>,
    pub map_blocks: BTreeMap<String, String>,
    pub items: BTreeMap<String, Item>,
    pub marts: MartCatalog,
    pub currency_constants: CurrencyCatalog,
    pub battle_reward_rules: BattleRewardRules,
    pub battle_escape_rules: BattleEscapeRules,
    pub step_event_rules: StepEventRules,
    pub fishing: FishingCatalog,
    pub fruit_trees: FruitTreeCatalog,
    pub field_moves: FieldMoveCatalog,
    pub runtime_spawn_points: BTreeMap<String, RuntimeSpawnPoint>,
    pub runtime_map_metadata: BTreeMap<String, RuntimeMapMetadata>,
    pub flee_mons: FleeMonTables,
    pub buena_password_categories: Vec<BuenaPasswordCategoryDefinition>,
    pub roaming_pokemon: Vec<RoamingPokemonDefinition>,
    pub buena_prizes: Vec<BuenaPrizeDefinition>,
    pub kurt_apricorn_recipes: Vec<KurtApricornRecipe>,
    pub shuckie_gift: Option<ShuckieGiftDefinition>,
    pub dratini_move_sets: Vec<DratiniMoveSetDefinition>,
    pub bug_contest_config: Option<BugContestConfig>,
    pub battle_tower_rules: Option<BattleTowerRules>,
    pub oak_ratings: Vec<OakRatingEntry>,
    pub odd_egg_definitions: Vec<OddEggDefinition>,
    pub magikarp_lengths: Vec<MagikarpLengthEntry>,
    pub happiness_data: Option<HappinessData>,
    pub encounter_slot_tables: EncounterSlotTables,
    pub encounter_music_modifiers: EncounterMusicModifiers,
    pub battle_stat_multipliers: BattleStatMultiplierTables,
    pub capture_wobble_probabilities: Vec<CaptureWobbleProbability>,
    pub move_priorities: MovePriorityTable,
    pub type_categories: TypeCategories,
    pub type_effectiveness: TypeEffectivenessTable,
    pub weather_modifiers: WeatherModifiers,
    pub pc_strings: BTreeMap<String, String>,
    pub menu_icons: BTreeMap<String, String>,
    pub pokedex_entries: BTreeMap<String, RuntimePokedexEntry>,
    pub pokemon_frontpic_anim: BTreeMap<String, FrontpicAnimProgram>,
    pub initialize_events: InitializeEventsConfig,
    pub story_event_script_constants: StoryEventScriptConstants,
    pub asm_text: BTreeMap<String, String>,
    pub move_names: Vec<String>,
    pub battle_animations: BTreeMap<String, Vec<String>>,
    pub battle_animation_table: Vec<String>,
    pub battle_anim_bundle: String,
    pub sprite_anim_bundle: String,
    pub sprite_palette_defaults: BTreeMap<String, i64>,
    pub pokegear_town_map_palette_map: BTreeMap<String, Vec<String>>,
    pub pokemon_cries: BTreeMap<String, PokemonCryMetadata>,
    pub wild_encounters: BTreeMap<String, WildEncounterData>,
    pub field_encounters: BTreeMap<String, FieldEncounterData>,
    pub npcs: BTreeMap<String, Value>,
    pub pokegear_landmarks: PokegearLandmarksPayload,
    pub trainers: TrainerCatalog,
    pub pokedex: Vec<Value>,
    pub story_events: Vec<Value>,
    pub phone_scripts: Vec<Value>,
    pub phone_contacts: PhoneContactCatalog,
    pub permanent_phone_numbers: Vec<String>,
    pub special_phone_calls: BTreeSet<String>,
    pub npc_trades: BTreeSet<String>,
    pub special_routines: BTreeSet<String>,
    pub audio: Vec<ModpackAudioAsset>,
    pub capture_rules: CaptureRules,
    pub tilesets: Vec<Value>,
    pub playability: PlayabilityRules,
}

impl GameDataSet {
    pub fn load_base_json(asset_root: &AssetRoot) -> Result<Self> {
        let index = asset_root.load_content_pack_index()?;
        let mut data = Self::default();
        data.apply_content_pack_index(asset_root, &index)?;
        Ok(data)
    }

    pub fn with_content_packs(mut self, asset_root: &AssetRoot) -> Result<Self> {
        self.apply_content_packs(asset_root)?;
        Ok(self)
    }

    pub fn apply_content_packs(&mut self, asset_root: &AssetRoot) -> Result<()> {
        let index = asset_root.load_content_pack_index()?;
        self.apply_content_pack_index(asset_root, &index)
    }

    pub fn apply_content_pack_index(
        &mut self,
        asset_root: &AssetRoot,
        index: &ContentPackIndex,
    ) -> Result<()> {
        for pack in index.enabled_packs_sorted() {
            if let Some(compiled_path) = &pack.compiled {
                let compiled_path =
                    resolve_content_pack_data_path(asset_root, &pack.id, compiled_path)?;
                let compiled: CompiledContentPack = read_json_file(&compiled_path)
                    .with_context(|| format!("load compiled content pack {}", pack.id))?;
                if compiled.pack_id != pack.id {
                    anyhow::bail!(
                        "compiled content pack {} declared packId {}",
                        pack.id,
                        compiled.pack_id
                    );
                }
                for category in CONTENT_PACK_CATEGORIES {
                    for payload in compiled.categories.entries(*category) {
                        self.apply_content_pack_payload(*category, payload.clone())
                            .with_context(|| {
                                format!(
                                    "apply compiled content pack {} category {}",
                                    pack.id,
                                    category.as_str()
                                )
                            })?;
                    }
                }
                continue;
            }

            for category in CONTENT_PACK_CATEGORIES {
                for entry in pack.files.entries(*category) {
                    let path = resolve_content_pack_data_path(asset_root, &pack.id, entry)?;
                    if *category == ContentPackCategory::Audio {
                        let audio_asset = ModpackAudioAsset::from_content_pack_path(entry)
                            .with_context(|| {
                                format!("apply content pack {} audio file {}", pack.id, entry)
                            })?;
                        if !path.exists() {
                            anyhow::bail!(
                                "content pack {} audio file {} is missing",
                                pack.id,
                                entry
                            );
                        }
                        self.audio.push(audio_asset);
                        continue;
                    }
                    let payload: Value = read_json_file(&path).with_context(|| {
                        format!(
                            "load content pack {} category {} file {}",
                            pack.id,
                            category.as_str(),
                            entry
                        )
                    })?;
                    self.apply_content_pack_payload(*category, payload)
                        .with_context(|| {
                            format!(
                                "apply content pack {} category {} file {}",
                                pack.id,
                                category.as_str(),
                                entry
                            )
                        })?;
                }
            }
        }
        Ok(())
    }

    fn apply_content_pack_payload(
        &mut self,
        category: ContentPackCategory,
        payload: Value,
    ) -> Result<()> {
        match category {
            ContentPackCategory::Pokemon => {
                for species in parse_one_or_many::<PokemonSpecies>(payload)? {
                    self.pokemon.insert(species.id.clone(), species);
                }
            }
            ContentPackCategory::Moves => {
                for move_data in parse_one_or_many::<Move>(payload)? {
                    self.moves.insert(move_data.name.clone(), move_data);
                }
            }
            ContentPackCategory::GrowthRates => {
                for curve in parse_one_or_many::<crystal_core::systems::experience::GrowthRateCurve>(
                    payload,
                )? {
                    self.growth_rates.insert(curve.id.clone(), curve);
                }
            }
            ContentPackCategory::Items => {
                for item in parse_one_or_many::<Item>(payload)? {
                    self.items.insert(item_key(&item)?, item);
                }
            }
            ContentPackCategory::Marts => {
                merge_mart_payload(&mut self.marts, payload)?;
            }
            ContentPackCategory::CurrencyConstants => {
                merge_currency_constants_payload(&mut self.currency_constants, payload)?;
            }
            ContentPackCategory::WildEncounters => {
                for data in parse_one_or_many::<WildEncounterData>(payload)? {
                    self.wild_encounters.insert(data.map_name.clone(), data);
                }
            }
            ContentPackCategory::FieldEncounters => {
                for data in parse_one_or_many::<FieldEncounterData>(payload)? {
                    self.field_encounters.insert(data.map_name.clone(), data);
                }
            }
            ContentPackCategory::RuntimeSpawnPoints => {
                self.runtime_spawn_points
                    .extend(parse_object_map::<RuntimeSpawnPoint>(payload)?);
            }
            ContentPackCategory::RuntimeMapMetadata => {
                self.runtime_map_metadata
                    .extend(parse_object_map::<RuntimeMapMetadata>(payload)?);
            }
            ContentPackCategory::FleeMons => {
                self.flee_mons =
                    serde_json::from_value(payload).context("parse flee mons payload")?;
            }
            ContentPackCategory::RoamingPokemon => {
                self.roaming_pokemon =
                    serde_json::from_value(payload).context("parse roaming Pokemon payload")?;
            }
            ContentPackCategory::BuenaPasswordCategories => {
                self.buena_password_categories = serde_json::from_value(payload)
                    .context("parse Buena password categories payload")?;
            }
            ContentPackCategory::BuenaPrizes => {
                self.buena_prizes =
                    serde_json::from_value(payload).context("parse Buena prizes payload")?;
            }
            ContentPackCategory::KurtApricornRecipes => {
                self.kurt_apricorn_recipes = serde_json::from_value(payload)
                    .context("parse Kurt apricorn recipes payload")?;
            }
            ContentPackCategory::ShuckieGift => {
                self.shuckie_gift =
                    Some(serde_json::from_value(payload).context("parse Shuckie gift payload")?);
            }
            ContentPackCategory::DratiniMoveSets => {
                self.dratini_move_sets =
                    serde_json::from_value(payload).context("parse Dratini move sets payload")?;
            }
            ContentPackCategory::BugContestConfig => {
                self.bug_contest_config = Some(
                    serde_json::from_value(payload)
                        .context("parse Bug-Catching Contest config payload")?,
                );
            }
            ContentPackCategory::BattleTowerRules => {
                self.battle_tower_rules = Some(
                    serde_json::from_value(payload).context("parse Battle Tower rules payload")?,
                );
            }
            ContentPackCategory::OakRatings => {
                self.oak_ratings =
                    serde_json::from_value(payload).context("parse Oak rating table payload")?;
            }
            ContentPackCategory::OddEggDefinitions => {
                self.odd_egg_definitions =
                    serde_json::from_value(payload).context("parse Odd Egg definitions payload")?;
            }
            ContentPackCategory::MagikarpLengths => {
                self.magikarp_lengths = serde_json::from_value(payload)
                    .context("parse Magikarp length table payload")?;
            }
            ContentPackCategory::HappinessData => {
                self.happiness_data =
                    Some(serde_json::from_value(payload).context("parse happiness data payload")?);
            }
            ContentPackCategory::EncounterSlotTables => {
                self.encounter_slot_tables = serde_json::from_value(payload)
                    .context("parse encounter slot tables payload")?;
            }
            ContentPackCategory::EncounterMusicModifiers => {
                self.encounter_music_modifiers = serde_json::from_value(payload)
                    .context("parse encounter music modifiers payload")?;
            }
            ContentPackCategory::BattleStatMultipliers => {
                self.battle_stat_multipliers = serde_json::from_value(payload)
                    .context("parse battle stat multipliers payload")?;
            }
            ContentPackCategory::CaptureWobbleProbabilities => {
                self.capture_wobble_probabilities = serde_json::from_value(payload)
                    .context("parse capture wobble probabilities payload")?;
            }
            ContentPackCategory::CaptureRules => {
                self.capture_rules =
                    serde_json::from_value(payload).context("parse capture rules payload")?;
            }
            ContentPackCategory::BattleEscapeRules => {
                self.battle_escape_rules =
                    serde_json::from_value(payload).context("parse battle escape rules payload")?;
            }
            ContentPackCategory::MovePriorities => {
                self.move_priorities =
                    serde_json::from_value(payload).context("parse move priorities payload")?;
            }
            ContentPackCategory::TypeCategories => {
                self.type_categories =
                    serde_json::from_value(payload).context("parse type categories payload")?;
            }
            ContentPackCategory::TypeEffectiveness => {
                self.type_effectiveness =
                    serde_json::from_value(payload).context("parse type effectiveness payload")?;
            }
            ContentPackCategory::WeatherModifiers => {
                self.weather_modifiers =
                    serde_json::from_value(payload).context("parse weather modifiers payload")?;
            }
            ContentPackCategory::BattleRewardRules => {
                self.battle_reward_rules =
                    serde_json::from_value(payload).context("parse battle reward rules payload")?;
            }
            ContentPackCategory::StepEventRules => {
                self.step_event_rules =
                    serde_json::from_value(payload).context("parse step event rules payload")?;
            }
            ContentPackCategory::Fishing => {
                self.fishing = serde_json::from_value(payload).context("parse fishing payload")?;
            }
            ContentPackCategory::FruitTrees => {
                merge_fruit_tree_payload(&mut self.fruit_trees, payload)?;
            }
            ContentPackCategory::FieldMoves => {
                self.field_moves =
                    serde_json::from_value(payload).context("parse field moves payload")?;
            }
            ContentPackCategory::MapAttributes => {
                for (map_name, attributes) in parse_object_map::<MapAttributes>(payload)? {
                    self.map_attributes.insert(map_name, attributes);
                }
            }
            ContentPackCategory::MapBlocks => {
                for (label, encoded) in parse_object_map::<String>(payload)? {
                    self.map_blocks.insert(label, encoded);
                }
            }
            ContentPackCategory::Learnsets => {
                for (species, learnset) in parse_learnsets(payload)? {
                    self.learnsets.insert(species, learnset);
                }
            }
            ContentPackCategory::LevelUpMoves => {
                merge_species_value_payload(&mut self.level_up_moves, payload, "moves")?;
            }
            ContentPackCategory::EggMoves => {
                merge_species_value_payload(&mut self.egg_moves, payload, "moves")?;
            }
            ContentPackCategory::Evolutions => {
                merge_evolution_payload(&mut self.evolutions, payload)?;
            }
            ContentPackCategory::Maps => {
                merge_object_payload(&mut self.map_scripts, payload)?;
            }
            ContentPackCategory::MapDimensions => {
                merge_object_payload(&mut self.map_dimensions, payload)?;
            }
            ContentPackCategory::Npcs => {
                merge_object_payload(&mut self.npcs, payload)?;
            }
            ContentPackCategory::PokegearLandmarks => {
                merge_pokegear_landmarks_payload(&mut self.pokegear_landmarks, payload)?;
            }
            ContentPackCategory::PcStrings => {
                self.pc_strings.extend(parse_object_map::<String>(payload)?);
            }
            ContentPackCategory::MenuIcons => {
                self.menu_icons.extend(parse_object_map::<String>(payload)?);
            }
            ContentPackCategory::Trainers => {
                for trainer in parse_one_or_many::<Trainer>(payload)? {
                    self.trainers.insert(trainer)?;
                }
            }
            ContentPackCategory::Pokedex => {
                push_flattened(&mut self.pokedex, payload);
            }
            ContentPackCategory::PokedexEntries => {
                for entry in parse_one_or_many::<RuntimePokedexEntry>(payload)? {
                    self.pokedex_entries.insert(entry.species.clone(), entry);
                }
            }
            ContentPackCategory::PokemonFrontpicAnim => {
                self.pokemon_frontpic_anim
                    .extend(parse_object_map::<FrontpicAnimProgram>(payload)?);
            }
            ContentPackCategory::InitializeEvents => {
                self.initialize_events =
                    serde_json::from_value(payload).context("parse initialize events payload")?;
            }
            ContentPackCategory::StoryEventScriptConstants => {
                self.story_event_script_constants = serde_json::from_value(payload)
                    .context("parse story event script constants payload")?;
            }
            ContentPackCategory::StoryEvents => {
                push_flattened(&mut self.story_events, payload);
            }
            ContentPackCategory::PhoneScripts => {
                push_flattened(&mut self.phone_scripts, payload);
            }
            ContentPackCategory::PhoneContacts => {
                merge_phone_contact_payload(&mut self.phone_contacts, payload)?;
            }
            ContentPackCategory::PermanentPhoneNumbers => {
                self.permanent_phone_numbers
                    .extend(serde_json::from_value::<Vec<String>>(payload)?);
            }
            ContentPackCategory::SpecialPhoneCalls => {
                self.special_phone_calls
                    .extend(serde_json::from_value::<Vec<String>>(payload)?);
            }
            ContentPackCategory::NpcTrades => {
                self.npc_trades
                    .extend(serde_json::from_value::<Vec<String>>(payload)?);
            }
            ContentPackCategory::SpecialRoutines => {
                self.special_routines
                    .extend(serde_json::from_value::<Vec<String>>(payload)?);
            }
            ContentPackCategory::AsmText => {
                self.asm_text.extend(parse_object_map::<String>(payload)?);
            }
            ContentPackCategory::MoveNames => {
                self.move_names
                    .extend(serde_json::from_value::<Vec<String>>(payload)?);
            }
            ContentPackCategory::BattleAnimations => {
                self.battle_animations
                    .extend(parse_object_map::<Vec<String>>(payload)?);
            }
            ContentPackCategory::BattleAnimationTable => {
                self.battle_animation_table
                    .extend(serde_json::from_value::<Vec<String>>(payload)?);
            }
            ContentPackCategory::BattleAnimBundle => {
                self.battle_anim_bundle =
                    serde_json::to_string(&payload).context("encode battle animation bundle")?;
            }
            ContentPackCategory::SpriteAnimBundle => {
                self.sprite_anim_bundle =
                    serde_json::to_string(&payload).context("encode sprite animation bundle")?;
            }
            ContentPackCategory::SpritePaletteDefaults => {
                self.sprite_palette_defaults
                    .extend(parse_object_map::<i64>(payload)?);
            }
            ContentPackCategory::PokegearTownMapPaletteMap => {
                self.pokegear_town_map_palette_map
                    .extend(parse_object_map::<Vec<String>>(payload)?);
            }
            ContentPackCategory::PokemonCries => {
                self.pokemon_cries
                    .extend(parse_object_map::<PokemonCryMetadata>(payload)?);
            }
            ContentPackCategory::Audio => {
                for audio_asset in parse_one_or_many::<ModpackAudioAsset>(payload)? {
                    audio_asset.validate()?;
                    self.audio.push(audio_asset);
                }
            }
            ContentPackCategory::Tilesets => {
                push_flattened(&mut self.tilesets, payload);
            }
            ContentPackCategory::Playability => {
                let playability: PlayabilityRules =
                    serde_json::from_value(payload).context("parse playability payload")?;
                merge_playability_rules(&mut self.playability, &playability);
            }
        }
        Ok(())
    }

    pub fn apply_modpack(&mut self, manifest: &ModpackManifest) -> Result<()> {
        for species in &manifest.payload.pokemon {
            self.pokemon.insert(species.id.clone(), species.clone());
        }
        for move_data in &manifest.payload.moves {
            self.moves.insert(move_data.name.clone(), move_data.clone());
        }
        merge_evolution_table(&mut self.evolutions, &manifest.payload.evolutions);
        merge_mart_catalog(&mut self.marts, &manifest.payload.marts);
        merge_currency_constants(
            &mut self.currency_constants,
            &manifest.payload.currency_constants,
        );
        self.battle_reward_rules = manifest.payload.battle_reward_rules.clone();
        self.step_event_rules = manifest.payload.step_event_rules.clone();
        for map in &manifest.payload.maps {
            self.maps.insert(map.id.clone(), map.clone());
        }
        for item in &manifest.payload.items {
            validate_manifest_item(item)?;
            self.items.insert(item_key(item)?, item.clone());
        }
        for wild_encounter_data in &manifest.payload.wild_encounters {
            self.wild_encounters.insert(
                wild_encounter_data.map_name.clone(),
                wild_encounter_data.clone(),
            );
        }
        for field_encounter_data in &manifest.payload.field_encounters {
            self.field_encounters.insert(
                field_encounter_data.map_name.clone(),
                field_encounter_data.clone(),
            );
        }
        self.fishing = manifest.payload.fishing.clone();
        merge_fruit_tree_catalog(&mut self.fruit_trees, &manifest.payload.fruit_trees);
        self.field_moves = manifest.payload.field_moves.clone();
        self.runtime_spawn_points.extend(
            manifest
                .payload
                .runtime_spawn_points
                .iter()
                .map(|(key, value)| (key.clone(), value.clone())),
        );
        self.runtime_map_metadata.extend(
            manifest
                .payload
                .runtime_map_metadata
                .iter()
                .map(|(key, value)| (key.clone(), value.clone())),
        );
        self.flee_mons = manifest.payload.flee_mons.clone();
        self.roaming_pokemon = manifest.payload.roaming_pokemon.clone();
        self.buena_password_categories = manifest.payload.buena_password_categories.clone();
        self.buena_prizes = manifest.payload.buena_prizes.clone();
        self.kurt_apricorn_recipes = manifest.payload.kurt_apricorn_recipes.clone();
        self.shuckie_gift = manifest.payload.shuckie_gift.clone();
        self.dratini_move_sets = manifest.payload.dratini_move_sets.clone();
        self.bug_contest_config = manifest.payload.bug_contest_config.clone();
        self.battle_tower_rules = manifest.payload.battle_tower_rules.clone();
        self.oak_ratings = manifest.payload.oak_ratings.clone();
        self.odd_egg_definitions = manifest.payload.odd_egg_definitions.clone();
        self.magikarp_lengths = manifest.payload.magikarp_lengths.clone();
        self.happiness_data = manifest.payload.happiness_data.clone();
        self.encounter_slot_tables = manifest.payload.encounter_slot_tables.clone();
        self.encounter_music_modifiers = manifest.payload.encounter_music_modifiers.clone();
        self.battle_stat_multipliers = manifest.payload.battle_stat_multipliers.clone();
        self.capture_wobble_probabilities = manifest.payload.capture_wobble_probabilities.clone();
        self.capture_rules = manifest.payload.capture_rules.clone();
        self.battle_escape_rules = manifest.payload.battle_escape_rules.clone();
        self.move_priorities = manifest.payload.move_priorities.clone();
        self.type_categories = manifest.payload.type_categories.clone();
        self.type_effectiveness = manifest.payload.type_effectiveness.clone();
        self.weather_modifiers = manifest.payload.weather_modifiers.clone();
        self.pc_strings.extend(
            manifest
                .payload
                .pc_strings
                .iter()
                .map(|(key, value)| (key.clone(), value.clone())),
        );
        self.menu_icons.extend(
            manifest
                .payload
                .menu_icons
                .iter()
                .map(|(key, value)| (key.clone(), value.clone())),
        );
        for entry in &manifest.payload.pokedex_entries {
            self.pokedex_entries
                .insert(entry.species.clone(), entry.clone());
        }
        self.pokemon_frontpic_anim.extend(
            manifest
                .payload
                .pokemon_frontpic_anim
                .iter()
                .map(|(key, value)| (key.clone(), value.clone())),
        );
        self.initialize_events = manifest.payload.initialize_events.clone();
        self.story_event_script_constants = manifest.payload.story_event_script_constants.clone();
        self.asm_text.extend(
            manifest
                .payload
                .asm_text
                .iter()
                .map(|(key, value)| (key.clone(), value.clone())),
        );
        self.move_names = manifest.payload.move_names.clone();
        self.battle_animations.extend(
            manifest
                .payload
                .battle_animations
                .iter()
                .map(|(key, value)| (key.clone(), value.clone())),
        );
        self.battle_animation_table = manifest.payload.battle_animation_table.clone();
        self.battle_anim_bundle = manifest.payload.battle_anim_bundle.clone();
        self.sprite_anim_bundle = manifest.payload.sprite_anim_bundle.clone();
        self.sprite_palette_defaults.extend(
            manifest
                .payload
                .sprite_palette_defaults
                .iter()
                .map(|(key, value)| (key.clone(), *value)),
        );
        self.pokegear_town_map_palette_map.extend(
            manifest
                .payload
                .pokegear_town_map_palette_map
                .iter()
                .map(|(key, value)| (key.clone(), value.clone())),
        );
        merge_pokegear_landmarks(
            &mut self.pokegear_landmarks,
            &manifest.payload.pokegear_landmarks,
        );
        self.pokemon_cries.extend(
            manifest
                .payload
                .pokemon_cries
                .iter()
                .map(|(key, value)| (key.clone(), value.clone())),
        );
        for trainer in manifest.payload.trainers.trainers.values() {
            self.trainers.insert(trainer.clone())?;
        }
        merge_phone_contact_catalog(&mut self.phone_contacts, &manifest.payload.phone_contacts);
        self.permanent_phone_numbers
            .extend(manifest.payload.permanent_phone_numbers.iter().cloned());
        self.special_phone_calls
            .extend(manifest.payload.special_phone_calls.iter().cloned());
        self.npc_trades
            .extend(manifest.payload.npc_trades.iter().cloned());
        self.special_routines
            .extend(manifest.payload.special_routines.iter().cloned());
        self.audio.extend(manifest.payload.audio.iter().cloned());
        self.capture_rules.fast_ball_species.extend(
            manifest
                .payload
                .capture_rules
                .fast_ball_species
                .iter()
                .cloned(),
        );
        self.capture_rules.heavy_ball_modifiers.extend(
            manifest
                .payload
                .capture_rules
                .heavy_ball_modifiers
                .iter()
                .map(|(species, modifier)| (species.clone(), *modifier)),
        );
        self.capture_rules.ball_rules.extend(
            manifest
                .payload
                .capture_rules
                .ball_rules
                .iter()
                .map(|(ball, rule)| (ball.clone(), rule.clone())),
        );
        self.capture_rules.guaranteed_capture_balls.extend(
            manifest
                .payload
                .capture_rules
                .guaranteed_capture_balls
                .iter()
                .cloned(),
        );
        self.capture_rules.status_bonus.extend(
            manifest
                .payload
                .capture_rules
                .status_bonus
                .iter()
                .map(|(status, bonus)| (status.clone(), *bonus)),
        );
        self.tilesets
            .extend(manifest.payload.tilesets.iter().cloned());
        merge_playability_rules(&mut self.playability, &manifest.payload.playability);
        Ok(())
    }

    pub fn with_modpack(mut self, manifest: &ModpackManifest) -> Result<Self> {
        self.apply_modpack(manifest)?;
        Ok(self)
    }

    pub fn create_pokemon(&self, species_id: &str, level: u8, dvs: Dv) -> Result<Pokemon> {
        let species = self
            .pokemon
            .get(species_id)
            .with_context(|| format!("unknown Pokemon species '{species_id}'"))?;
        Ok(create_pokemon_from_known_dvs(
            species,
            level,
            dvs,
            &self.learnsets,
            &self.moves,
            &self.growth_rates,
        )?)
    }

    pub fn wild_battle_start(
        &self,
        encounter: WildEncounterRoll,
        rng: &mut Random,
    ) -> Result<WildBattleStart> {
        let resolved = encounter
            .resolved
            .as_ref()
            .with_context(|| "cannot start wild battle from a non-triggered encounter roll")?;
        let species_id = &resolved.encounter.species;
        let species = self
            .pokemon
            .get(species_id)
            .with_context(|| format!("unknown wild species '{species_id}' in encounter table"))?;
        Ok(wild_battle_start_from_encounter(
            encounter,
            species,
            &self.learnsets,
            &self.moves,
            &self.growth_rates,
            rng,
        )?)
    }

    pub fn static_wild_battle_start(
        &self,
        request: StaticWildBattleRequest,
        rng: &mut Random,
    ) -> Result<StaticWildBattleStart> {
        Ok(static_wild_battle_start(
            &self.pokemon,
            &self.learnsets,
            &self.moves,
            &self.growth_rates,
            request,
            rng,
        )?)
    }

    pub fn trainer_battle_start(
        &self,
        state: &crystal_core::state::GameState,
        request: TrainerBattleRequest,
    ) -> Result<TrainerBattleStartStatus> {
        Ok(trainer_battle_start(
            state,
            &self.trainers,
            &self.pokemon,
            &self.learnsets,
            &self.moves,
            &self.growth_rates,
            request,
        )?)
    }

    pub fn overworld_map(&self, map_name: &str) -> Result<OverworldMapData> {
        let attributes = self
            .map_attributes
            .get(map_name)
            .with_context(|| format!("missing map attributes for {map_name}"))?;
        let blocks_label = attributes
            .blocks_label
            .as_deref()
            .filter(|label| !label.trim().is_empty())
            .with_context(|| format!("missing blocks_label for map {map_name}"))?;
        let encoded_blocks = self
            .map_blocks
            .get(blocks_label)
            .with_context(|| format!("missing map block payload {blocks_label}"))?;
        let metatile_ids = decode_base64_bytes(encoded_blocks)
            .with_context(|| format!("decode map block payload {blocks_label}"))?
            .into_iter()
            .map(u16::from)
            .collect();
        Ok(OverworldMapData::from_attributes(
            map_name,
            attributes,
            metatile_ids,
        ))
    }

    pub fn map_module(&self, map_name: &str) -> Result<MapModule> {
        if let Some(module) = self.maps.get(map_name) {
            return Ok(module.clone());
        }
        let attributes = self
            .map_attributes
            .get(map_name)
            .with_context(|| format!("missing map attributes for {map_name}"))?
            .clone();
        let map_scripts_label = attributes
            .map_scripts_label
            .as_deref()
            .filter(|label| !label.trim().is_empty())
            .with_context(|| format!("missing map_scripts_label for map {map_name}"))?;
        let map_events_label = attributes
            .map_events_label
            .as_deref()
            .filter(|label| !label.trim().is_empty())
            .with_context(|| format!("missing map_events_label for map {map_name}"))?;
        let blocks_label = attributes
            .blocks_label
            .as_deref()
            .filter(|label| !label.trim().is_empty())
            .with_context(|| format!("missing blocks_label for map {map_name}"))?;

        if !self.map_scripts.contains_key(map_scripts_label) {
            anyhow::bail!("missing map scripts label {map_scripts_label}");
        }
        let events_script = self
            .map_scripts
            .get(map_events_label)
            .with_context(|| format!("missing map events label {map_events_label}"))?;
        let objects_payload = self
            .npcs
            .get(map_name)
            .with_context(|| format!("missing NPC object payload for {map_name}"))?;
        let encoded_blocks = self
            .map_blocks
            .get(blocks_label)
            .with_context(|| format!("missing map block payload {blocks_label}"))?;

        let map_scripts = self
            .map_scripts
            .get(map_scripts_label)
            .with_context(|| format!("missing map scripts label {map_scripts_label}"))?;
        let scripts =
            runtime_module_script_subset(&self.map_scripts, [map_scripts_label, map_events_label]);
        let mut scenes = parse_map_scene_table(map_name, map_scripts)?;
        add_referenced_numeric_scene_ids(&mut scenes, self, map_name, &attributes, &scripts);
        let map_script_section_commands =
            parse_map_script_section_commands(map_name, map_scripts_label, map_scripts)?;
        let map_event_section_commands =
            parse_map_event_section_commands(map_name, map_events_label, events_script)?;
        let events = parse_map_events(map_name, events_script)?;
        let trainer_scripts = parse_trainer_scripts(map_name, &scripts)?;
        let scripted_trainer_battles = parse_scripted_trainer_battles(map_name, &scripts)?;
        let scripted_wild_battles = parse_scripted_wild_battles(map_name, &scripts)?;
        let script_item_grants = parse_script_item_grants(map_name, &scripts)?;
        let (script_item_checks, script_item_takes) =
            parse_script_item_accesses(map_name, &scripts)?;
        let script_economy_commands = parse_script_economy_commands(map_name, &scripts)?;
        let gift_pokemon_scripts =
            parse_gift_pokemon_scripts(map_name, &scripts, &self.story_event_script_constants)?;
        let script_flag_commands = parse_script_flag_commands(map_name, &scripts)?;
        let script_scene_commands = parse_script_scene_commands(map_name, &scripts)?;
        let script_audio_commands = parse_script_audio_commands(map_name, &scripts)?;
        let script_block_changes = parse_script_block_changes(map_name, &scripts)?;
        let script_object_commands = parse_script_object_commands(map_name, &scripts)?;
        let script_movements = parse_script_movements(map_name, &scripts, &script_object_commands)?;
        let script_map_commands = parse_script_map_commands(map_name, &scripts)?;
        let script_text_commands = parse_script_text_commands(map_name, &scripts)?;
        let script_text_bodies = parse_script_text_bodies(map_name, &scripts)?;
        let script_menu_definitions = parse_script_menu_definitions(map_name, &scripts)?;
        let script_variable_commands = parse_script_variable_commands(map_name, &scripts)?;
        let script_control_commands = parse_script_control_commands(map_name, &scripts)?;
        let objects: Vec<ObjectEvent> = serde_json::from_value(objects_payload.clone())
            .with_context(|| format!("parse NPC object payload for {map_name}"))?;
        let script_field_pickups = parse_script_field_pickups(map_name, &scripts, &objects)?;
        let script_shop_commands = parse_script_shop_commands(map_name, &scripts)?;
        let script_phone_commands = parse_script_phone_commands(map_name, &scripts)?;
        let script_runtime_commands = parse_script_runtime_commands(map_name, &scripts)?;
        let blocks = decode_base64_bytes(encoded_blocks)
            .with_context(|| format!("decode map block payload {blocks_label}"))?
            .into_iter()
            .map(u16::from)
            .collect();

        Ok(MapModule {
            id: map_name.to_string(),
            attributes,
            scripts,
            trainer_scripts,
            scripted_trainer_battles,
            scripted_wild_battles,
            script_item_grants,
            script_item_checks,
            script_item_takes,
            script_economy_commands,
            gift_pokemon_scripts,
            script_flag_commands,
            script_scene_commands,
            script_audio_commands,
            script_block_changes,
            script_object_commands,
            script_movements,
            script_map_commands,
            script_text_commands,
            script_text_bodies,
            script_menu_definitions,
            script_variable_commands,
            script_control_commands,
            script_field_pickups,
            script_shop_commands,
            script_phone_commands,
            script_runtime_commands,
            map_script_section_commands,
            map_event_section_commands,
            scenes,
            events,
            objects,
            blocks,
        })
    }

    pub fn resolve_warp_transition(&self, trigger: &WarpTrigger) -> Result<WarpTransition> {
        let destination_map = self
            .map_name_for_constant(&trigger.warp.target_map_constant)
            .with_context(|| {
                format!(
                    "unknown target map constant '{}' for warp {} on {}",
                    trigger.warp.target_map_constant, trigger.warp.index, trigger.map_name
                )
            })?;
        let destination_module = self
            .map_module(&destination_map)
            .with_context(|| format!("load destination map module {destination_map}"))?;
        if trigger.warp.target_warp_id < 1 {
            anyhow::bail!(
                "warp {} on {} has dynamic target warp id {}",
                trigger.warp.index,
                trigger.map_name,
                trigger.warp.target_warp_id
            );
        }
        let destination_index = trigger
            .warp
            .target_warp_id
            .checked_sub(1)
            .with_context(|| {
                format!(
                    "warp {} on {} has invalid target warp id 0",
                    trigger.warp.index, trigger.map_name
                )
            })? as usize;
        let destination_warp = destination_module
            .events
            .warps
            .get(destination_index)
            .cloned()
            .with_context(|| {
                format!(
                    "warp id {} referenced by {} exceeds available warps ({}) on {}",
                    trigger.warp.target_warp_id,
                    trigger.map_name,
                    destination_module.events.warps.len(),
                    destination_map
                )
            })?;

        Ok(WarpTransition {
            trigger: trigger.clone(),
            destination: WarpDestination {
                map_name: destination_map,
                tile: warp_tile_position(&destination_warp),
                warp: destination_warp,
            },
        })
    }

    pub fn map_name_for_constant(&self, map_constant: &str) -> Option<String> {
        map_constants(self).get(map_constant).cloned()
    }

    pub fn resolve_connection_transition(
        &self,
        trigger: &ConnectionTrigger,
    ) -> Result<ConnectionTransition> {
        let target_map = trigger.connection.target_map.clone();
        let target_attributes = self.map_attributes.get(&target_map).with_context(|| {
            format!(
                "connection target '{}' missing attributes (referenced by {})",
                target_map, trigger.map_name
            )
        })?;
        let target_tile = connection_destination_tile(
            trigger.tile,
            &trigger.connection.direction,
            trigger.connection.offset,
            target_attributes,
        )?;

        Ok(ConnectionTransition {
            trigger: trigger.clone(),
            destination: ConnectionDestination {
                map_name: target_map,
                tile: target_tile,
            },
        })
    }
}

fn connection_destination_tile(
    source_tile: crystal_core::world::map::TilePosition,
    direction: &str,
    offset: i32,
    target_attributes: &MapAttributes,
) -> Result<crystal_core::world::map::TilePosition> {
    let offset_tiles = offset * 2;
    let width = target_attributes.width as i32 * 2;
    let height = target_attributes.height as i32 * 2;
    let (target_x, target_y) = match direction {
        "north" => (source_tile.x as i32 - offset_tiles, height - 1),
        "south" => (source_tile.x as i32 - offset_tiles, 1),
        "west" => (width - 1, source_tile.y as i32 - offset_tiles),
        "east" => (1, source_tile.y as i32 - offset_tiles),
        other => anyhow::bail!("unsupported connection direction '{other}'"),
    };
    let min_tile = 1;
    let max_x = (width - 1).max(min_tile);
    let max_y = (height - 1).max(min_tile);
    if target_x < min_tile || target_x > max_x || target_y < min_tile || target_y > max_y {
        anyhow::bail!(
            "connection destination tile ({target_x}, {target_y}) is outside target map tile bounds {min_tile}..={max_x}, {min_tile}..={max_y}"
        );
    }
    Ok(crystal_core::world::map::TilePosition::new(
        target_x as i16,
        target_y as i16,
    ))
}

fn read_json_file<T>(path: &Path) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    let bytes = std::fs::read(path).with_context(|| format!("read {}", path.display()))?;
    serde_json::from_slice(&bytes).with_context(|| format!("parse {}", path.display()))
}

pub fn write_compiled_game_pack(path: impl AsRef<Path>, pack: &CompiledGamePack) -> Result<()> {
    let path = path.as_ref();
    validate_compiled_game_pack_path(path)?;
    if pack.format_version != COMPILED_GAME_PACK_FORMAT_VERSION {
        anyhow::bail!(
            "compiled game pack {} has unsupported format version {}",
            path.display(),
            pack.format_version
        );
    }
    let mut encoded = Vec::new();
    ciborium::into_writer(pack, &mut encoded)
        .with_context(|| format!("encode compiled game pack {}", path.display()))?;
    let mut bytes = Vec::with_capacity(COMPILED_GAME_PACK_MAGIC.len() + encoded.len());
    bytes.extend_from_slice(COMPILED_GAME_PACK_MAGIC);
    bytes.extend_from_slice(&encoded);
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("create compiled game pack directory {}", parent.display()))?;
    }
    std::fs::write(path, bytes)
        .with_context(|| format!("write compiled game pack {}", path.display()))
}

pub fn read_compiled_game_pack(path: impl AsRef<Path>) -> Result<CompiledGamePack> {
    Ok(read_loaded_compiled_game_pack(path)?.pack)
}

pub fn read_loaded_compiled_game_pack(path: impl AsRef<Path>) -> Result<LoadedCompiledGamePack> {
    let path = path.as_ref();
    validate_compiled_game_pack_path(path)?;
    let bytes = std::fs::read(path).with_context(|| format!("read {}", path.display()))?;
    let pack = decode_compiled_game_pack(&bytes, path)?;
    Ok(LoadedCompiledGamePack {
        path: path.to_path_buf(),
        bytes,
        pack,
    })
}

fn decode_compiled_game_pack(bytes: &[u8], path: &Path) -> Result<CompiledGamePack> {
    let payload = bytes
        .strip_prefix(COMPILED_GAME_PACK_MAGIC)
        .with_context(|| format!("{} is not a compiled Crystal game pack", path.display()))?;
    let mut cursor = std::io::Cursor::new(payload);
    let pack: CompiledGamePack = ciborium::from_reader(&mut cursor)
        .with_context(|| format!("decode compiled game pack {}", path.display()))?;
    if cursor.position() as usize != payload.len() {
        anyhow::bail!(
            "compiled game pack {} has {} trailing bytes",
            path.display(),
            payload.len() - cursor.position() as usize
        );
    }
    if pack.format_version != COMPILED_GAME_PACK_FORMAT_VERSION {
        anyhow::bail!(
            "compiled game pack {} uses unsupported format version {}",
            path.display(),
            pack.format_version
        );
    }
    Ok(pack)
}

fn resolve_content_pack_data_path(
    asset_root: &AssetRoot,
    pack_id: &str,
    entry: &str,
) -> Result<PathBuf> {
    let path = Path::new(entry);
    if path.is_absolute() {
        anyhow::bail!("content pack {pack_id} path '{entry}' must be relative to assets/data");
    }
    if entry.starts_with("assets/data/") {
        anyhow::bail!(
            "content pack {pack_id} path '{entry}' must not include the assets/data prefix"
        );
    }
    if path
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        anyhow::bail!("content pack {pack_id} path '{entry}' must not traverse parent directories");
    }
    asset_root.resolve_data_path(path)
}

fn resolve_compiled_game_pack_data_path(asset_root: &AssetRoot, entry: &Path) -> Result<PathBuf> {
    if entry.is_absolute() {
        anyhow::bail!(
            "compiled game pack path '{}' must be relative to assets/data",
            entry.display()
        );
    }
    let entry_text = entry.to_string_lossy();
    if entry_text.starts_with("assets/data/") {
        anyhow::bail!(
            "compiled game pack path '{entry_text}' must not include the assets/data prefix"
        );
    }
    if entry
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        anyhow::bail!(
            "compiled game pack path '{}' must not traverse parent directories",
            entry.display()
        );
    }
    validate_compiled_game_pack_path(entry)?;
    asset_root.resolve_data_path(entry)
}

fn validate_compiled_game_pack_path(path: &Path) -> Result<()> {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "compiled game pack {} must have a file extension",
                path.display()
            )
        })?;
    if extension != COMPILED_GAME_PACK_EXTENSION {
        anyhow::bail!(
            "compiled game pack {} must use .{}",
            path.display(),
            COMPILED_GAME_PACK_EXTENSION
        );
    }
    Ok(())
}

fn add_referenced_numeric_scene_ids(
    scenes: &mut MapSceneTable,
    data: &GameDataSet,
    _map_name: &str,
    attributes: &MapAttributes,
    scripts: &BTreeMap<String, Value>,
) {
    if !scenes.scenes.is_empty() {
        return;
    }

    let mut numeric_ids = BTreeSet::new();
    for payload in scripts.values() {
        collect_numeric_setscene_tokens(payload, &mut numeric_ids);
    }
    if let Some(map_constant) = attributes.map_constant.as_deref() {
        for payload in data.map_scripts.values() {
            collect_numeric_setmapscene_tokens(payload, map_constant, &mut numeric_ids);
        }
    }

    scenes
        .scenes
        .extend(numeric_ids.into_iter().map(|scene_id| MapScene {
            scene_id: scene_id.to_string(),
            script_name: None,
        }));
}

fn collect_numeric_setscene_tokens(payload: &Value, numeric_ids: &mut BTreeSet<usize>) {
    let Some(entries) = payload.as_array() else {
        return;
    };
    for entry in entries {
        let Some("setscene") = entry.get("command").and_then(Value::as_str) else {
            continue;
        };
        if let Some(scene_id) = entry
            .get("args")
            .and_then(Value::as_array)
            .and_then(|args| args.first())
            .and_then(Value::as_str)
            .and_then(|token| token.parse::<usize>().ok())
        {
            numeric_ids.insert(scene_id);
        }
    }
}

fn collect_numeric_setmapscene_tokens(
    payload: &Value,
    map_constant: &str,
    numeric_ids: &mut BTreeSet<usize>,
) {
    let Some(entries) = payload.as_array() else {
        return;
    };
    for entry in entries {
        let Some("setmapscene") = entry.get("command").and_then(Value::as_str) else {
            continue;
        };
        let Some(args) = entry.get("args").and_then(Value::as_array) else {
            continue;
        };
        if args.first().and_then(Value::as_str) != Some(map_constant) {
            continue;
        }
        if let Some(scene_id) = args
            .get(1)
            .and_then(Value::as_str)
            .and_then(|token| token.parse::<usize>().ok())
        {
            numeric_ids.insert(scene_id);
        }
    }
}

fn parse_one_or_many<T>(payload: Value) -> Result<Vec<T>>
where
    T: DeserializeOwned,
{
    if payload.is_array() {
        return serde_json::from_value(payload).context("parse array payload");
    }
    serde_json::from_value(payload)
        .map(|entry| vec![entry])
        .context("parse single payload entry")
}

fn parse_object_map<T>(payload: Value) -> Result<BTreeMap<String, T>>
where
    T: DeserializeOwned,
{
    serde_json::from_value(payload).context("parse object-map payload")
}

fn parse_learnsets(payload: Value) -> Result<SpeciesLearnsets> {
    let mut learnsets = SpeciesLearnsets::new();
    if let Some(array) = payload.as_array() {
        for entry in array {
            merge_learnset_entry(&mut learnsets, entry.clone())?;
        }
        return Ok(learnsets);
    }
    if let Some(object) = payload.as_object() {
        if object.contains_key("species") && object.contains_key("learnset") {
            merge_learnset_entry(&mut learnsets, Value::Object(object.clone()))?;
            return Ok(learnsets);
        }
        for (species, _value) in object {
            anyhow::bail!(
                "learnset payload for species '{species}' must use an explicit species/learnset entry"
            );
        }
        anyhow::bail!("learnset payload object must declare species and learnset");
    }
    anyhow::bail!("learnset payload must be an entry object or an array of entry objects")
}

fn merge_learnset_entry(learnsets: &mut SpeciesLearnsets, payload: Value) -> Result<()> {
    #[derive(Deserialize)]
    struct Entry {
        species: String,
        learnset: Vec<LearnsetEntry>,
    }

    let entry: Entry = serde_json::from_value(payload).context("parse learnset entry")?;
    learnsets.insert(entry.species, entry.learnset);
    Ok(())
}

fn merge_evolution_payload(target: &mut EvolutionTable, payload: Value) -> Result<()> {
    if let Some(array) = payload.as_array() {
        for entry in array {
            merge_evolution_payload(target, entry.clone())?;
        }
        return Ok(());
    }
    let Some(object) = payload.as_object() else {
        anyhow::bail!("evolution payload must be an entry object or an array of entry objects");
    };
    if object.contains_key("species") && object.contains_key("evolutions") {
        merge_evolution_entry(target, Value::Object(object.clone()))?;
        return Ok(());
    }
    for (species, _value) in object {
        anyhow::bail!(
            "evolution payload for species '{species}' must use an explicit species/evolutions entry"
        );
    }
    anyhow::bail!("evolution payload object must declare species and evolutions")
}

fn merge_evolution_entry(target: &mut EvolutionTable, payload: Value) -> Result<()> {
    #[derive(Deserialize)]
    struct Entry {
        species: String,
        evolutions: Vec<EvolutionEntry>,
    }

    let entry: Entry = serde_json::from_value(payload).context("parse evolution entry")?;
    target.0.insert(entry.species, entry.evolutions);
    Ok(())
}

fn merge_evolution_table(target: &mut EvolutionTable, source: &EvolutionTable) {
    for (species, entries) in &source.0 {
        target.0.insert(species.clone(), entries.clone());
    }
}

fn merge_mart_payload(target: &mut MartCatalog, payload: Value) -> Result<()> {
    let marts: BTreeMap<String, Vec<String>> =
        serde_json::from_value(payload).context("parse mart catalog payload")?;
    target.0.extend(marts);
    Ok(())
}

fn merge_mart_catalog(target: &mut MartCatalog, source: &MartCatalog) {
    for (mart_id, item_ids) in &source.0 {
        target.0.insert(mart_id.clone(), item_ids.clone());
    }
}

fn merge_fruit_tree_payload(target: &mut FruitTreeCatalog, payload: Value) -> Result<()> {
    let fruit_trees: BTreeMap<String, String> =
        serde_json::from_value(payload).context("parse fruit tree catalog payload")?;
    target.0.extend(fruit_trees);
    Ok(())
}

fn merge_fruit_tree_catalog(target: &mut FruitTreeCatalog, source: &FruitTreeCatalog) {
    for (tree_id, item_id) in &source.0 {
        target.0.insert(tree_id.clone(), item_id.clone());
    }
}

fn merge_phone_contact_payload(target: &mut PhoneContactCatalog, payload: Value) -> Result<()> {
    let contacts: PhoneContactCatalog =
        serde_json::from_value(payload).context("parse phone contact catalog payload")?;
    merge_phone_contact_catalog(target, &contacts);
    Ok(())
}

fn merge_phone_contact_catalog(target: &mut PhoneContactCatalog, source: &PhoneContactCatalog) {
    for (contact_id, record) in &source.0 {
        target.0.insert(contact_id.clone(), record.clone());
    }
}

fn merge_currency_constants_payload(target: &mut CurrencyCatalog, payload: Value) -> Result<()> {
    let constants: BTreeMap<String, u32> =
        serde_json::from_value(payload).context("parse currency constants payload")?;
    target.0.extend(constants);
    Ok(())
}

fn merge_currency_constants(target: &mut CurrencyCatalog, source: &CurrencyCatalog) {
    for (constant, value) in &source.0 {
        target.0.insert(constant.clone(), *value);
    }
}

fn merge_pokegear_landmarks_payload(
    target: &mut PokegearLandmarksPayload,
    payload: Value,
) -> Result<()> {
    let payload: PokegearLandmarksPayload =
        serde_json::from_value(payload).context("parse pokegear landmarks payload")?;
    merge_pokegear_landmarks(target, &payload);
    Ok(())
}

fn merge_pokegear_landmarks(
    target: &mut PokegearLandmarksPayload,
    payload: &PokegearLandmarksPayload,
) {
    for landmark in &payload.landmarks {
        if let Some(existing) = target
            .landmarks
            .iter_mut()
            .find(|existing| existing.constant == landmark.constant)
        {
            *existing = landmark.clone();
        } else {
            target.landmarks.push(landmark.clone());
        }
    }
    target.map_to_landmark.extend(
        payload
            .map_to_landmark
            .iter()
            .map(|(map, landmark)| (map.clone(), landmark.clone())),
    );
}

fn merge_species_value_payload(
    target: &mut BTreeMap<String, Value>,
    payload: Value,
    field_name: &str,
) -> Result<()> {
    if let Some(array) = payload.as_array() {
        for entry in array {
            merge_species_value_payload(target, entry.clone(), field_name)?;
        }
        return Ok(());
    }
    let Some(object) = payload.as_object() else {
        anyhow::bail!("species value payload must be an entry object or an array of entry objects");
    };
    if let Some(species) = object.get("species").and_then(Value::as_str) {
        let value = object.get(field_name).cloned().ok_or_else(|| {
            anyhow::anyhow!("species value payload for '{species}' must declare {field_name}")
        })?;
        target.insert(species.to_string(), value);
        return Ok(());
    }
    for (species, _value) in object {
        anyhow::bail!(
            "species value payload for species '{species}' must use an explicit species/{field_name} entry"
        );
    }
    anyhow::bail!("species value payload object must declare species and {field_name}")
}

fn merge_object_payload(target: &mut BTreeMap<String, Value>, payload: Value) -> Result<()> {
    if let Some(array) = payload.as_array() {
        for entry in array {
            merge_object_payload(target, entry.clone())?;
        }
        return Ok(());
    }
    let Some(object) = payload.as_object() else {
        anyhow::bail!("object payload must be an object or an array of objects");
    };
    for (key, value) in object {
        target.insert(key.clone(), value.clone());
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
struct ScriptCommand {
    command: String,
    args: Vec<String>,
}

fn parse_map_events(map_name: &str, payload: &Value) -> Result<MapEvents> {
    let commands: Vec<ScriptCommand> =
        serde_json::from_value(payload.clone()).context("parse map event command list")?;
    let mut section: Option<&str> = None;
    let mut next_warp_index = 1_u16;
    let mut events = MapEvents::default();

    for command in commands {
        match command.command.as_str() {
            "def_warp_events" => {
                section = Some("warps");
                next_warp_index = 1;
            }
            "def_coord_events" => {
                section = Some("coord_events");
            }
            "def_bg_events" => {
                section = Some("bg_events");
            }
            "def_object_events" => {
                section = Some("object_events");
            }
            "warp_event" if section == Some("warps") => {
                if command.args.len() != 4 {
                    anyhow::bail!(
                        "Malformed warp_event in {map_name}: expected 4 args, found {}.",
                        command.args.len()
                    );
                }
                let target_map_constant = command.args[2].trim_end_matches(',').to_string();
                events.warps.push(WarpEvent {
                    index: next_warp_index,
                    x: parse_script_u16(&command.args[0])?,
                    y: parse_script_u16(&command.args[1])?,
                    target_map: target_map_constant.clone(),
                    target_map_constant,
                    target_warp_id: i16::try_from(parse_script_i32(&command.args[3])?)
                        .with_context(|| {
                            format!(
                                "warp_event target warp id '{}' in {map_name} is outside i16 range",
                                command.args[3]
                            )
                        })?,
                });
                next_warp_index += 1;
            }
            "coord_event" if section == Some("coord_events") => {
                if command.args.len() != 4 {
                    anyhow::bail!(
                        "Malformed coord_event in {map_name}: expected 4 args, found {}.",
                        command.args.len()
                    );
                }
                events.coord_events.push(CoordEvent {
                    x: parse_script_u16(&command.args[0])?,
                    y: parse_script_u16(&command.args[1])?,
                    scene_id: command.args[2].clone(),
                    script_name: command.args[3].clone(),
                });
            }
            "bg_event" if section == Some("bg_events") => {
                if command.args.len() != 4 {
                    anyhow::bail!(
                        "Malformed bg_event in {map_name}: expected 4 args, found {}.",
                        command.args.len()
                    );
                }
                events.bg_events.push(BackgroundEvent {
                    x: parse_script_u16(&command.args[0])?,
                    y: parse_script_u16(&command.args[1])?,
                    event_type: command.args[2].clone(),
                    script: command.args[3].clone(),
                });
            }
            _ => {}
        }
    }

    Ok(events)
}

fn parse_map_script_section_commands(
    map_name: &str,
    script_label: &str,
    payload: &Value,
) -> Result<Vec<MapScriptSectionCommand>> {
    let expected_arg_counts = map_script_section_command_arg_counts();
    let commands: Vec<ScriptCommand> = serde_json::from_value(payload.clone())
        .with_context(|| format!("parse map script section commands for {map_name}"))?;
    let mut parsed = Vec::new();
    for (index, command) in commands.into_iter().enumerate() {
        let Some(expected) = expected_arg_counts.get(command.command.as_str()) else {
            continue;
        };
        if !expected.contains(&command.args.len()) {
            anyhow::bail!(
                "Malformed {} command in {script_label} for {map_name}: expected one of {:?} args, found {}.",
                command.command,
                expected,
                command.args.len()
            );
        }
        parsed.push(MapScriptSectionCommand {
            command: command.command,
            args: command.args,
            command_index: index,
        });
    }
    Ok(parsed)
}

fn parse_map_event_section_commands(
    map_name: &str,
    script_label: &str,
    payload: &Value,
) -> Result<Vec<MapEventSectionCommand>> {
    let expected_arg_counts = map_event_section_command_arg_counts();
    let commands: Vec<ScriptCommand> = serde_json::from_value(payload.clone())
        .with_context(|| format!("parse map event section commands for {map_name}"))?;
    let mut parsed = Vec::new();
    for (index, command) in commands.into_iter().enumerate() {
        let Some(expected) = expected_arg_counts.get(command.command.as_str()) else {
            continue;
        };
        if !expected.contains(&command.args.len()) {
            anyhow::bail!(
                "Malformed {} command in {script_label} for {map_name}: expected one of {:?} args, found {}.",
                command.command,
                expected,
                command.args.len()
            );
        }
        parsed.push(MapEventSectionCommand {
            command: command.command,
            args: command.args,
            command_index: index,
        });
    }
    Ok(parsed)
}

fn parse_trainer_scripts(
    map_name: &str,
    scripts: &BTreeMap<String, Value>,
) -> Result<BTreeMap<String, TrainerBattleRequest>> {
    let mut trainer_scripts = BTreeMap::new();
    for (script_name, payload) in scripts {
        let Some(entries) = payload.as_array() else {
            continue;
        };
        for entry in entries {
            let Some(command_name) = entry.get("command").and_then(Value::as_str) else {
                continue;
            };
            if command_name != "trainer" {
                continue;
            }
            let args = entry
                .get("args")
                .and_then(Value::as_array)
                .with_context(|| {
                    format!("Malformed trainer command in {script_name} for {map_name}: args must be an array.")
                })?;
            if args.len() != 7 {
                anyhow::bail!(
                    "Malformed trainer command in {script_name} for {map_name}: expected 7 args, found {}.",
                    args.len()
                );
            }
            let arg = |index: usize| -> Result<&str> {
                args[index].as_str().with_context(|| {
                    format!(
                        "Malformed trainer command in {script_name} for {map_name}: arg {index} must be a string."
                    )
                })
            };
            let mut request = TrainerBattleRequest::new(arg(0)?, arg(1)?, "");
            request.event_flag = trainer_command_optional_arg(arg(2)?);
            request.seen_text = trainer_command_optional_arg(arg(3)?);
            request.win_text = trainer_command_optional_arg(arg(4)?);
            request.loss_text = trainer_command_optional_arg(arg(5)?);
            request.callback = trainer_command_optional_arg(arg(6)?);
            request.source_script = script_name.clone();
            trainer_scripts.insert(script_name.clone(), request);
        }
    }
    Ok(trainer_scripts)
}

fn parse_script_item_grants(
    map_name: &str,
    scripts: &BTreeMap<String, Value>,
) -> Result<Vec<ScriptItemGrant>> {
    let mut grants = Vec::new();
    for (script_name, payload) in scripts {
        let Some(entries) = payload.as_array() else {
            continue;
        };
        for (index, entry) in entries.iter().enumerate() {
            let Some(command_name) = entry.get("command").and_then(Value::as_str) else {
                continue;
            };
            let verbose = match command_name {
                "giveitem" => false,
                "verbosegiveitem" => true,
                _ => continue,
            };
            let args = script_command_args(map_name, script_name, command_name, entry)?;
            if args.len() != 1 && args.len() != 2 {
                anyhow::bail!(
                    "Malformed {command_name} command in {script_name} for {map_name}: expected 1 or 2 args, found {}.",
                    args.len()
                );
            }
            let quantity = if let Some(quantity) = args.get(1) {
                parse_script_u16(quantity)?
            } else {
                1
            };
            grants.push(ScriptItemGrant {
                item_id: args[0].to_string(),
                quantity,
                source_script: script_name.clone(),
                command_index: index,
                verbose,
            });
        }
    }
    Ok(grants)
}

fn parse_script_item_accesses(
    map_name: &str,
    scripts: &BTreeMap<String, Value>,
) -> Result<(Vec<ScriptItemAccess>, Vec<ScriptItemAccess>)> {
    let mut checks = Vec::new();
    let mut takes = Vec::new();
    for (script_name, payload) in scripts {
        let Some(entries) = payload.as_array() else {
            continue;
        };
        for (index, entry) in entries.iter().enumerate() {
            let Some(command_name) = entry.get("command").and_then(Value::as_str) else {
                continue;
            };
            if command_name != "checkitem" && command_name != "takeitem" {
                continue;
            }
            let args = script_command_args(map_name, script_name, command_name, entry)?;
            if args.len() != 1 {
                anyhow::bail!(
                    "Malformed {command_name} command in {script_name} for {map_name}: expected 1 arg, found {}.",
                    args.len()
                );
            }
            let access = ScriptItemAccess {
                item_id: args[0].to_string(),
                source_script: script_name.clone(),
                command_index: index,
            };
            if command_name == "checkitem" {
                checks.push(access);
            } else {
                takes.push(access);
            }
        }
    }
    Ok((checks, takes))
}

fn parse_script_field_pickups(
    map_name: &str,
    scripts: &BTreeMap<String, Value>,
    objects: &[ObjectEvent],
) -> Result<Vec<ScriptFieldPickup>> {
    let mut pickups = Vec::new();
    for (script_name, payload) in scripts {
        let Some(entries) = payload.as_array() else {
            continue;
        };
        for (index, entry) in entries.iter().enumerate() {
            let Some(command_name) = entry.get("command").and_then(Value::as_str) else {
                continue;
            };
            match command_name {
                command if SCRIPT_FIELD_ITEM_PICKUP_COMMANDS.contains(&command) => {
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if SCRIPT_FIELD_ITEMBALL_PICKUP_COMMANDS.contains(&command)
                        && args.len() != 1
                        && args.len() != 2
                    {
                        anyhow::bail!(
                            "Malformed {command_name} command in {script_name} for {map_name}: expected 1 or 2 args, found {}.",
                            args.len()
                        );
                    }
                    if SCRIPT_FIELD_HIDDEN_ITEM_PICKUP_COMMANDS.contains(&command)
                        && args.len() != 2
                    {
                        anyhow::bail!(
                            "Malformed {command_name} command in {script_name} for {map_name}: expected 2 args, found {}.",
                            args.len()
                        );
                    }
                    let (quantity, event_flag) =
                        if SCRIPT_FIELD_ITEMBALL_PICKUP_COMMANDS.contains(&command) {
                            let event_flag = objects
                                .iter()
                                .find(|object| object.script == *script_name)
                                .map(|object| object.event_flag.clone());
                            let quantity = if let Some(quantity) = args.get(1) {
                                parse_script_u16(quantity)?
                            } else {
                                1
                            };
                            (quantity, event_flag)
                        } else {
                            (1, Some(args[1].to_string()))
                        };
                    pickups.push(ScriptFieldPickup {
                        command: command_name.to_string(),
                        item_id: Some(args[0].to_string()),
                        quantity,
                        event_flag,
                        fruit_tree_id: None,
                        source_script: script_name.clone(),
                        command_index: index,
                    });
                }
                command if SCRIPT_FIELD_FRUIT_TREE_PICKUP_COMMANDS.contains(&command) => {
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if args.len() != 1 {
                        anyhow::bail!(
                            "Malformed {command_name} command in {script_name} for {map_name}: expected 1 arg, found {}.",
                            args.len()
                        );
                    }
                    pickups.push(ScriptFieldPickup {
                        command: command_name.to_string(),
                        item_id: None,
                        quantity: 1,
                        event_flag: None,
                        fruit_tree_id: Some(args[0].to_string()),
                        source_script: script_name.clone(),
                        command_index: index,
                    });
                }
                _ => {}
            }
        }
    }
    Ok(pickups)
}

fn parse_script_shop_commands(
    map_name: &str,
    scripts: &BTreeMap<String, Value>,
) -> Result<Vec<ScriptShopCommand>> {
    let mut commands = Vec::new();
    for (script_name, payload) in scripts {
        let Some(entries) = payload.as_array() else {
            continue;
        };
        for (index, entry) in entries.iter().enumerate() {
            let Some(command_name) = entry.get("command").and_then(Value::as_str) else {
                continue;
            };
            if !SCRIPT_SHOP_COMMANDS.contains(&command_name) {
                continue;
            }
            let args = script_command_args(map_name, script_name, command_name, entry)?;
            if args.len() != 2 {
                anyhow::bail!(
                    "Malformed pokemart command in {script_name} for {map_name}: expected 2 args, found {}.",
                    args.len()
                );
            }
            commands.push(ScriptShopCommand {
                mart_type: args[0].to_string(),
                mart_id: args[1].to_string(),
                source_script: script_name.clone(),
                command_index: index,
            });
        }
    }
    Ok(commands)
}

fn parse_script_phone_commands(
    map_name: &str,
    scripts: &BTreeMap<String, Value>,
) -> Result<Vec<ScriptPhoneCommand>> {
    let mut commands = Vec::new();
    for (script_name, payload) in scripts {
        let Some(entries) = payload.as_array() else {
            continue;
        };
        for (index, entry) in entries.iter().enumerate() {
            let Some(command_name) = entry.get("command").and_then(Value::as_str) else {
                continue;
            };
            if !SCRIPT_PHONE_CHECK_COMMANDS.contains(&command_name)
                && !SCRIPT_PHONE_REGISTRATION_COMMANDS.contains(&command_name)
            {
                continue;
            }
            let args = script_command_args(map_name, script_name, command_name, entry)?;
            if args.len() != 1 {
                anyhow::bail!(
                    "Malformed {command_name} command in {script_name} for {map_name}: expected 1 arg, found {}.",
                    args.len()
                );
            }
            commands.push(ScriptPhoneCommand {
                command: command_name.to_string(),
                contact_id: args[0].to_string(),
                source_script: script_name.clone(),
                command_index: index,
            });
        }
    }
    Ok(commands)
}

fn parse_script_runtime_commands(
    map_name: &str,
    scripts: &BTreeMap<String, Value>,
) -> Result<Vec<ScriptRuntimeCommand>> {
    let expected_arg_counts = script_runtime_command_arg_counts();
    let mut commands = Vec::new();
    for (script_name, payload) in scripts {
        let Some(entries) = payload.as_array() else {
            continue;
        };
        for (index, entry) in entries.iter().enumerate() {
            let Some(command_name) = entry.get("command").and_then(Value::as_str) else {
                continue;
            };
            let Some(expected) = expected_arg_counts.get(command_name) else {
                continue;
            };
            let args = script_command_args(map_name, script_name, command_name, entry)?;
            if args.len() != *expected {
                anyhow::bail!(
                    "Malformed {command_name} command in {script_name} for {map_name}: expected {expected} args, found {}.",
                    args.len()
                );
            }
            commands.push(ScriptRuntimeCommand {
                command: command_name.to_string(),
                args: args.iter().map(|arg| (*arg).to_string()).collect(),
                source_script: script_name.clone(),
                command_index: index,
            });
        }
    }
    Ok(commands)
}

fn parse_script_economy_commands(
    map_name: &str,
    scripts: &BTreeMap<String, Value>,
) -> Result<Vec<ScriptEconomyCommand>> {
    let mut commands = Vec::new();
    for (script_name, payload) in scripts {
        let Some(entries) = payload.as_array() else {
            continue;
        };
        for (index, entry) in entries.iter().enumerate() {
            let Some(command_name) = entry.get("command").and_then(Value::as_str) else {
                continue;
            };
            match command_name {
                command
                    if SCRIPT_MONEY_CHECK_COMMANDS.contains(&command)
                        || SCRIPT_MONEY_MUTATION_COMMANDS.contains(&command) =>
                {
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if args.len() < 2 {
                        anyhow::bail!(
                            "Malformed {command_name} command in {script_name} for {map_name}: expected account and amount args, found {}.",
                            args.len()
                        );
                    }
                    commands.push(ScriptEconomyCommand {
                        command: command_name.to_string(),
                        account: Some(args[0].to_string()),
                        amount_tokens: args[1..].iter().map(|arg| (*arg).to_string()).collect(),
                        source_script: script_name.clone(),
                        command_index: index,
                    });
                }
                command
                    if SCRIPT_COIN_CHECK_COMMANDS.contains(&command)
                        || SCRIPT_COIN_MUTATION_COMMANDS.contains(&command) =>
                {
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if args.is_empty() {
                        anyhow::bail!(
                            "Malformed {command_name} command in {script_name} for {map_name}: expected amount args, found 0."
                        );
                    }
                    commands.push(ScriptEconomyCommand {
                        command: command_name.to_string(),
                        account: None,
                        amount_tokens: args.iter().map(|arg| (*arg).to_string()).collect(),
                        source_script: script_name.clone(),
                        command_index: index,
                    });
                }
                _ => {}
            }
        }
    }
    Ok(commands)
}

fn parse_gift_pokemon_scripts(
    map_name: &str,
    scripts: &BTreeMap<String, Value>,
    constants: &StoryEventScriptConstants,
) -> Result<Vec<GiftPokemonScript>> {
    let mut gifts = Vec::new();
    for (script_name, payload) in scripts {
        let Some(entries) = payload.as_array() else {
            continue;
        };
        for (index, entry) in entries.iter().enumerate() {
            let Some(command_name) = entry.get("command").and_then(Value::as_str) else {
                continue;
            };
            match command_name {
                "givepoke" => {
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if args.len() != 2 && args.len() != 3 && args.len() != 5 {
                        anyhow::bail!(
                            "Malformed givepoke command in {script_name} for {map_name}: expected 2, 3, or 5 args, found {}.",
                            args.len()
                        );
                    }
                    let level = resolve_gift_level_token(map_name, args[1], constants)?;
                    gifts.push(GiftPokemonScript {
                        species_id: args[0].to_string(),
                        level_token: args[1].to_string(),
                        level,
                        held_item_id: args.get(2).and_then(|item| {
                            if *item == NO_ITEM {
                                None
                            } else {
                                Some((*item).to_string())
                            }
                        }),
                        nickname_label: args.get(3).map(|value| (*value).to_string()),
                        ot_label: args.get(4).map(|value| (*value).to_string()),
                        source_script: script_name.clone(),
                        command_index: index,
                        egg: false,
                    });
                }
                "giveegg" => {
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if args.len() != 2 {
                        anyhow::bail!(
                            "Malformed giveegg command in {script_name} for {map_name}: expected 2 args, found {}.",
                            args.len()
                        );
                    }
                    let level = resolve_gift_level_token(map_name, args[1], constants)?;
                    gifts.push(GiftPokemonScript {
                        species_id: args[0].to_string(),
                        level_token: args[1].to_string(),
                        level,
                        held_item_id: None,
                        nickname_label: None,
                        ot_label: None,
                        source_script: script_name.clone(),
                        command_index: index,
                        egg: true,
                    });
                }
                _ => {}
            }
        }
    }
    Ok(gifts)
}

fn parse_script_flag_commands(
    map_name: &str,
    scripts: &BTreeMap<String, Value>,
) -> Result<Vec<ScriptFlagCommand>> {
    let mut commands = Vec::new();
    for (script_name, payload) in scripts {
        let Some(entries) = payload.as_array() else {
            continue;
        };
        for (index, entry) in entries.iter().enumerate() {
            let Some(command_name) = entry.get("command").and_then(Value::as_str) else {
                continue;
            };
            match command_name {
                "setevent" | "clearevent" | "checkevent" | "setflag" | "clearflag"
                | "checkflag" => {
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if args.len() != 1 {
                        anyhow::bail!(
                            "Malformed {command_name} command in {script_name} for {map_name}: expected 1 arg, found {}.",
                            args.len()
                        );
                    }
                    commands.push(ScriptFlagCommand {
                        command: command_name.to_string(),
                        flag_id: args[0].to_string(),
                        source_script: script_name.clone(),
                        command_index: index,
                    });
                }
                _ => {}
            }
        }
    }
    Ok(commands)
}

fn parse_script_scene_commands(
    map_name: &str,
    scripts: &BTreeMap<String, Value>,
) -> Result<Vec<ScriptSceneCommand>> {
    let mut commands = Vec::new();
    for (script_name, payload) in scripts {
        let Some(entries) = payload.as_array() else {
            continue;
        };
        for (index, entry) in entries.iter().enumerate() {
            let Some(command_name) = entry.get("command").and_then(Value::as_str) else {
                continue;
            };
            match command_name {
                command if SCRIPT_SCENE_CHECK_COMMANDS.contains(&command) => {
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if !args.is_empty() {
                        anyhow::bail!(
                            "Malformed {command_name} command in {script_name} for {map_name}: expected 0 args, found {}.",
                            args.len()
                        );
                    }
                    commands.push(ScriptSceneCommand {
                        command: command_name.to_string(),
                        map_id: None,
                        scene_id: None,
                        source_script: script_name.clone(),
                        command_index: index,
                    });
                }
                command if SCRIPT_SCENE_CURRENT_MAP_MUTATION_COMMANDS.contains(&command) => {
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if args.len() != 1 {
                        anyhow::bail!(
                            "Malformed {command_name} command in {script_name} for {map_name}: expected 1 arg, found {}.",
                            args.len()
                        );
                    }
                    commands.push(ScriptSceneCommand {
                        command: command_name.to_string(),
                        map_id: None,
                        scene_id: Some(args[0].to_string()),
                        source_script: script_name.clone(),
                        command_index: index,
                    });
                }
                command if SCRIPT_SCENE_TARGET_MAP_MUTATION_COMMANDS.contains(&command) => {
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if args.len() != 2 {
                        anyhow::bail!(
                            "Malformed {command_name} command in {script_name} for {map_name}: expected 2 args, found {}.",
                            args.len()
                        );
                    }
                    commands.push(ScriptSceneCommand {
                        command: command_name.to_string(),
                        map_id: Some(args[0].to_string()),
                        scene_id: Some(args[1].to_string()),
                        source_script: script_name.clone(),
                        command_index: index,
                    });
                }
                _ => {}
            }
        }
    }
    Ok(commands)
}

fn parse_script_audio_commands(
    map_name: &str,
    scripts: &BTreeMap<String, Value>,
) -> Result<Vec<ScriptAudioCommand>> {
    let mut commands = Vec::new();
    for (script_name, payload) in scripts {
        let Some(entries) = payload.as_array() else {
            continue;
        };
        for (index, entry) in entries.iter().enumerate() {
            let Some(command_name) = entry.get("command").and_then(Value::as_str) else {
                continue;
            };
            match command_name {
                "playmusic" | "playsound" | "cry" => {
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if args.len() != 1 {
                        anyhow::bail!(
                            "Malformed {command_name} command in {script_name} for {map_name}: expected 1 arg, found {}.",
                            args.len()
                        );
                    }
                    commands.push(ScriptAudioCommand {
                        command: command_name.to_string(),
                        audio_id: Some(args[0].to_string()),
                        fade_frames: None,
                        source_script: script_name.clone(),
                        command_index: index,
                    });
                }
                "musicfadeout" => {
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if args.len() != 2 {
                        anyhow::bail!(
                            "Malformed musicfadeout command in {script_name} for {map_name}: expected 2 args, found {}.",
                            args.len()
                        );
                    }
                    commands.push(ScriptAudioCommand {
                        command: command_name.to_string(),
                        audio_id: Some(args[0].to_string()),
                        fade_frames: Some(parse_script_u16(args[1])?),
                        source_script: script_name.clone(),
                        command_index: index,
                    });
                }
                "waitsfx" => {
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if !args.is_empty() {
                        anyhow::bail!(
                            "Malformed waitsfx command in {script_name} for {map_name}: expected 0 args, found {}.",
                            args.len()
                        );
                    }
                    commands.push(ScriptAudioCommand {
                        command: command_name.to_string(),
                        audio_id: None,
                        fade_frames: None,
                        source_script: script_name.clone(),
                        command_index: index,
                    });
                }
                _ => {}
            }
        }
    }
    Ok(commands)
}

fn parse_script_block_changes(
    map_name: &str,
    scripts: &BTreeMap<String, Value>,
) -> Result<Vec<ScriptBlockChange>> {
    let mut changes = Vec::new();
    for (script_name, payload) in scripts {
        let Some(entries) = payload.as_array() else {
            continue;
        };
        for (index, entry) in entries.iter().enumerate() {
            let Some(command_name) = entry.get("command").and_then(Value::as_str) else {
                continue;
            };
            if command_name != "changeblock" {
                continue;
            }
            let args = script_command_args(map_name, script_name, command_name, entry)?;
            if args.len() != 3 {
                anyhow::bail!(
                    "Malformed changeblock command in {script_name} for {map_name}: expected 3 args, found {}.",
                    args.len()
                );
            }
            changes.push(ScriptBlockChange {
                x: parse_script_u16(args[0])?,
                y: parse_script_u16(args[1])?,
                block_id: parse_script_u16(args[2])?,
                source_script: script_name.clone(),
                command_index: index,
            });
        }
    }
    Ok(changes)
}

fn parse_script_object_commands(
    map_name: &str,
    scripts: &BTreeMap<String, Value>,
) -> Result<Vec<ScriptObjectCommand>> {
    let mut commands = Vec::new();
    for (script_name, payload) in scripts {
        let Some(entries) = payload.as_array() else {
            continue;
        };
        for (index, entry) in entries.iter().enumerate() {
            let Some(command_name) = entry.get("command").and_then(Value::as_str) else {
                continue;
            };
            match command_name {
                command if SCRIPT_OBJECT_NO_PAYLOAD_COMMANDS.contains(&command) => {
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if !args.is_empty() {
                        anyhow::bail!(
                            "Malformed {command_name} command in {script_name} for {map_name}: expected 0 args, found {}.",
                            args.len()
                        );
                    }
                    commands.push(ScriptObjectCommand {
                        command: command_name.to_string(),
                        object_id: None,
                        target_object_id: None,
                        x: None,
                        y: None,
                        direction: None,
                        movement: None,
                        emote: None,
                        duration: None,
                        source_script: script_name.clone(),
                        command_index: index,
                    });
                }
                command if SCRIPT_OBJECT_VISIBILITY_COMMANDS.contains(&command) => {
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if args.len() != 1 {
                        anyhow::bail!(
                            "Malformed {command_name} command in {script_name} for {map_name}: expected 1 arg, found {}.",
                            args.len()
                        );
                    }
                    commands.push(ScriptObjectCommand {
                        command: command_name.to_string(),
                        object_id: Some(args[0].to_string()),
                        target_object_id: None,
                        x: None,
                        y: None,
                        direction: None,
                        movement: None,
                        emote: None,
                        duration: None,
                        source_script: script_name.clone(),
                        command_index: index,
                    });
                }
                command if SCRIPT_OBJECT_COORDINATE_COMMANDS.contains(&command) => {
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if args.len() != 3 {
                        anyhow::bail!(
                            "Malformed {command_name} command in {script_name} for {map_name}: expected 3 args, found {}.",
                            args.len()
                        );
                    }
                    commands.push(ScriptObjectCommand {
                        command: command_name.to_string(),
                        object_id: Some(args[0].to_string()),
                        target_object_id: None,
                        x: Some(parse_script_u16(args[1])?),
                        y: Some(parse_script_u16(args[2])?),
                        direction: None,
                        movement: None,
                        emote: None,
                        duration: None,
                        source_script: script_name.clone(),
                        command_index: index,
                    });
                }
                command if SCRIPT_OBJECT_DIRECTION_COMMANDS.contains(&command) => {
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if args.len() != 2 {
                        anyhow::bail!(
                            "Malformed {command_name} command in {script_name} for {map_name}: expected 2 args, found {}.",
                            args.len()
                        );
                    }
                    commands.push(ScriptObjectCommand {
                        command: command_name.to_string(),
                        object_id: Some(args[0].to_string()),
                        target_object_id: None,
                        x: None,
                        y: None,
                        direction: Some(args[1].to_string()),
                        movement: None,
                        emote: None,
                        duration: None,
                        source_script: script_name.clone(),
                        command_index: index,
                    });
                }
                command if SCRIPT_OBJECT_DIRECT_MOVEMENT_COMMANDS.contains(&command) => {
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if args.len() != 2 {
                        anyhow::bail!(
                            "Malformed {command_name} command in {script_name} for {map_name}: expected 2 args, found {}.",
                            args.len()
                        );
                    }
                    commands.push(ScriptObjectCommand {
                        command: command_name.to_string(),
                        object_id: Some(args[0].to_string()),
                        target_object_id: None,
                        x: None,
                        y: None,
                        direction: None,
                        movement: Some(args[1].to_string()),
                        emote: None,
                        duration: None,
                        source_script: script_name.clone(),
                        command_index: index,
                    });
                }
                command if SCRIPT_OBJECT_LAST_TALKED_MOVEMENT_COMMANDS.contains(&command) => {
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if args.len() != 1 {
                        anyhow::bail!(
                            "Malformed {command_name} command in {script_name} for {map_name}: expected 1 arg, found {}.",
                            args.len()
                        );
                    }
                    commands.push(ScriptObjectCommand {
                        command: command_name.to_string(),
                        object_id: None,
                        target_object_id: None,
                        x: None,
                        y: None,
                        direction: None,
                        movement: Some(args[0].to_string()),
                        emote: None,
                        duration: None,
                        source_script: script_name.clone(),
                        command_index: index,
                    });
                }
                command if SCRIPT_OBJECT_TARGET_COMMANDS.contains(&command) => {
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if args.len() != 2 {
                        anyhow::bail!(
                            "Malformed {command_name} command in {script_name} for {map_name}: expected 2 args, found {}.",
                            args.len()
                        );
                    }
                    commands.push(ScriptObjectCommand {
                        command: command_name.to_string(),
                        object_id: Some(args[0].to_string()),
                        target_object_id: Some(args[1].to_string()),
                        x: None,
                        y: None,
                        direction: None,
                        movement: None,
                        emote: None,
                        duration: None,
                        source_script: script_name.clone(),
                        command_index: index,
                    });
                }
                command if SCRIPT_OBJECT_EMOTE_COMMANDS.contains(&command) => {
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if args.len() != 3 {
                        anyhow::bail!(
                            "Malformed {command_name} command in {script_name} for {map_name}: expected 3 args, found {}.",
                            args.len()
                        );
                    }
                    commands.push(ScriptObjectCommand {
                        command: command_name.to_string(),
                        object_id: Some(args[1].to_string()),
                        target_object_id: None,
                        x: None,
                        y: None,
                        direction: None,
                        movement: None,
                        emote: Some(args[0].to_string()),
                        duration: Some(parse_script_u16(args[2])?),
                        source_script: script_name.clone(),
                        command_index: index,
                    });
                }
                _ => {}
            }
        }
    }
    Ok(commands)
}

fn parse_script_map_commands(
    map_name: &str,
    scripts: &BTreeMap<String, Value>,
) -> Result<Vec<ScriptMapCommand>> {
    let mut commands = Vec::new();
    for (script_name, payload) in scripts {
        let Some(entries) = payload.as_array() else {
            continue;
        };
        for (index, entry) in entries.iter().enumerate() {
            let Some(command_name) = entry.get("command").and_then(Value::as_str) else {
                continue;
            };
            match command_name {
                command if SCRIPT_MAP_WARP_COMMANDS.contains(&command) => {
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if args.len() != 3 {
                        anyhow::bail!(
                            "Malformed {command_name} command in {script_name} for {map_name}: expected 3 args, found {}.",
                            args.len()
                        );
                    }
                    commands.push(ScriptMapCommand {
                        command: command_name.to_string(),
                        target_map: Some(script_warp_target_map(args[0])),
                        x: Some(parse_script_u16(args[1])?),
                        y: Some(parse_script_u16(args[2])?),
                        facing: None,
                        map_setup: None,
                        source_script: script_name.clone(),
                        command_index: index,
                    });
                }
                command if SCRIPT_MAP_FACING_WARP_COMMANDS.contains(&command) => {
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if args.len() != 4 {
                        anyhow::bail!(
                            "Malformed {command_name} command in {script_name} for {map_name}: expected 4 args, found {}.",
                            args.len()
                        );
                    }
                    commands.push(ScriptMapCommand {
                        command: command_name.to_string(),
                        target_map: Some(script_warp_target_map(args[1])),
                        x: Some(parse_script_u16(args[2])?),
                        y: Some(parse_script_u16(args[3])?),
                        facing: Some(args[0].to_string()),
                        map_setup: None,
                        source_script: script_name.clone(),
                        command_index: index,
                    });
                }
                command if SCRIPT_MAP_NEW_LOAD_COMMANDS.contains(&command) => {
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if args.len() != 1 {
                        anyhow::bail!(
                            "Malformed {command_name} command in {script_name} for {map_name}: expected 1 arg, found {}.",
                            args.len()
                        );
                    }
                    commands.push(ScriptMapCommand {
                        command: command_name.to_string(),
                        target_map: None,
                        x: None,
                        y: None,
                        facing: None,
                        map_setup: Some(args[0].to_string()),
                        source_script: script_name.clone(),
                        command_index: index,
                    });
                }
                command if SCRIPT_MAP_NO_PAYLOAD_COMMANDS.contains(&command) => {
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if !args.is_empty() {
                        anyhow::bail!(
                            "Malformed {command_name} command in {script_name} for {map_name}: expected 0 args, found {}.",
                            args.len()
                        );
                    }
                    commands.push(ScriptMapCommand {
                        command: command_name.to_string(),
                        target_map: None,
                        x: None,
                        y: None,
                        facing: None,
                        map_setup: None,
                        source_script: script_name.clone(),
                        command_index: index,
                    });
                }
                command if SCRIPT_MAP_REANCHOR_COMMANDS.contains(&command) => {
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if args.len() > 1 {
                        anyhow::bail!(
                            "Malformed {command_name} command in {script_name} for {map_name}: expected 0 or 1 args, found {}.",
                            args.len()
                        );
                    }
                    commands.push(ScriptMapCommand {
                        command: command_name.to_string(),
                        target_map: None,
                        x: None,
                        y: None,
                        facing: None,
                        map_setup: args.first().map(|setup| (*setup).to_string()),
                        source_script: script_name.clone(),
                        command_index: index,
                    });
                }
                _ => {}
            }
        }
    }
    Ok(commands)
}

fn parse_script_text_commands(
    map_name: &str,
    scripts: &BTreeMap<String, Value>,
) -> Result<Vec<ScriptTextCommand>> {
    let mut commands = Vec::new();
    for (script_name, payload) in scripts {
        let Some(entries) = payload.as_array() else {
            continue;
        };
        for (index, entry) in entries.iter().enumerate() {
            let Some(command_name) = entry.get("command").and_then(Value::as_str) else {
                continue;
            };
            match command_name {
                "opentext" | "closetext" | "promptbutton" | "waitbutton" | "yesorno" => {
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if !args.is_empty() {
                        anyhow::bail!(
                            "Malformed {command_name} command in {script_name} for {map_name}: expected 0 args, found {}.",
                            args.len()
                        );
                    }
                    commands.push(ScriptTextCommand {
                        command: command_name.to_string(),
                        text_label: None,
                        source_script: script_name.clone(),
                        command_index: index,
                    });
                }
                "writetext" | "jumptext" | "jumptextfaceplayer" => {
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if args.len() != 1 {
                        anyhow::bail!(
                            "Malformed {command_name} command in {script_name} for {map_name}: expected 1 arg, found {}.",
                            args.len()
                        );
                    }
                    commands.push(ScriptTextCommand {
                        command: command_name.to_string(),
                        text_label: Some(args[0].to_string()),
                        source_script: script_name.clone(),
                        command_index: index,
                    });
                }
                _ => {}
            }
        }
    }
    Ok(commands)
}

fn parse_script_text_bodies(
    map_name: &str,
    scripts: &BTreeMap<String, Value>,
) -> Result<BTreeMap<String, ScriptTextBody>> {
    let expected_arg_counts = text_body_command_arg_counts();
    let mut bodies = BTreeMap::new();
    for (script_name, payload) in scripts {
        if !is_text_script(payload) {
            continue;
        }
        let Some(entries) = payload.as_array() else {
            continue;
        };
        let mut commands = Vec::new();
        for (index, entry) in entries.iter().enumerate() {
            let Some(command_name) = entry.get("command").and_then(Value::as_str) else {
                continue;
            };
            let Some(expected) = expected_arg_counts.get(command_name) else {
                continue;
            };
            let args = text_body_command_args(map_name, script_name, command_name, entry)?;
            if args.len() != *expected {
                anyhow::bail!(
                    "Malformed {command_name} command in {script_name} for {map_name}: expected {expected} args, found {}.",
                    args.len()
                );
            }
            commands.push(ScriptTextBodyCommand {
                command: command_name.to_string(),
                args,
                command_index: index,
            });
        }
        if !commands.is_empty() {
            bodies.insert(
                script_name.clone(),
                ScriptTextBody {
                    label: script_name.clone(),
                    commands,
                },
            );
        }
    }
    Ok(bodies)
}

fn parse_script_menu_definitions(
    map_name: &str,
    scripts: &BTreeMap<String, Value>,
) -> Result<BTreeMap<String, ScriptMenuDefinition>> {
    let expected_arg_counts = menu_definition_command_arg_counts();
    let mut menus = BTreeMap::new();
    for (script_name, payload) in scripts {
        let Some(entries) = payload.as_array() else {
            continue;
        };
        let command_names: Vec<&str> = entries
            .iter()
            .filter_map(|entry| entry.get("command").and_then(Value::as_str))
            .collect();
        if command_names.is_empty()
            || !command_names
                .iter()
                .all(|command| expected_arg_counts.contains_key(command))
            || (!command_names.contains(&"menu_coords") && !script_name.contains("Menu"))
        {
            continue;
        }
        let mut commands = Vec::new();
        for (index, entry) in entries.iter().enumerate() {
            let command_name = entry
                .get("command")
                .and_then(Value::as_str)
                .with_context(|| {
                    format!("Malformed menu definition command in {script_name} for {map_name}: command must be a string.")
                })?;
            let expected = &expected_arg_counts[command_name];
            let args = script_command_args(map_name, script_name, command_name, entry)?;
            if !expected.contains(&args.len()) {
                anyhow::bail!(
                    "Malformed {command_name} menu command in {script_name} for {map_name}: expected one of {:?} args, found {}.",
                    expected,
                    args.len()
                );
            }
            commands.push(ScriptMenuCommand {
                command: command_name.to_string(),
                args: args.iter().map(|arg| (*arg).to_string()).collect(),
                command_index: index,
            });
        }
        menus.insert(
            script_name.clone(),
            ScriptMenuDefinition {
                label: script_name.clone(),
                commands,
            },
        );
    }
    Ok(menus)
}

fn text_body_command_args(
    map_name: &str,
    script_name: &str,
    command_name: &str,
    entry: &Value,
) -> Result<Vec<String>> {
    let args = entry.get("args").with_context(|| {
        format!("Malformed {command_name} command in {script_name} for {map_name}: missing args.")
    })?;
    if let Some(text) = args.as_str() {
        if text.is_empty() {
            return Ok(Vec::new());
        }
        return Ok(vec![text.to_string()]);
    }
    let Some(array) = args.as_array() else {
        anyhow::bail!(
            "Malformed {command_name} command in {script_name} for {map_name}: args must be a string or an array."
        );
    };
    array
        .iter()
        .enumerate()
        .map(|(index, value)| {
            value.as_str().map(str::to_string).with_context(|| {
                format!(
                    "Malformed {command_name} command in {script_name} for {map_name}: arg {index} must be a string."
                )
            })
        })
        .collect()
}

fn parse_script_variable_commands(
    map_name: &str,
    scripts: &BTreeMap<String, Value>,
) -> Result<Vec<ScriptVariableCommand>> {
    let mut commands = Vec::new();
    for (script_name, payload) in scripts {
        let Some(entries) = payload.as_array() else {
            continue;
        };
        for (index, entry) in entries.iter().enumerate() {
            let Some(command_name) = entry.get("command").and_then(Value::as_str) else {
                continue;
            };
            match command_name {
                "setval" => {
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if args.is_empty() {
                        anyhow::bail!(
                            "Malformed setval command in {script_name} for {map_name}: expected at least 1 arg, found 0."
                        );
                    }
                    commands.push(ScriptVariableCommand {
                        command: command_name.to_string(),
                        target: None,
                        value_tokens: args.iter().map(|arg| (*arg).to_string()).collect(),
                        source_script: script_name.clone(),
                        command_index: index,
                    });
                }
                "readvar" | "readmem" | "writemem" => {
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if args.len() != 1 {
                        anyhow::bail!(
                            "Malformed {command_name} command in {script_name} for {map_name}: expected 1 arg, found {}.",
                            args.len()
                        );
                    }
                    commands.push(ScriptVariableCommand {
                        command: command_name.to_string(),
                        target: Some(args[0].to_string()),
                        value_tokens: Vec::new(),
                        source_script: script_name.clone(),
                        command_index: index,
                    });
                }
                "loadvar" | "loadmem" => {
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if args.len() < 2 {
                        anyhow::bail!(
                            "Malformed {command_name} command in {script_name} for {map_name}: expected at least 2 args, found {}.",
                            args.len()
                        );
                    }
                    commands.push(ScriptVariableCommand {
                        command: command_name.to_string(),
                        target: Some(args[0].to_string()),
                        value_tokens: args[1..].iter().map(|arg| (*arg).to_string()).collect(),
                        source_script: script_name.clone(),
                        command_index: index,
                    });
                }
                "checktime" => {
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if args.len() != 1 {
                        anyhow::bail!(
                            "Malformed checktime command in {script_name} for {map_name}: expected 1 arg, found {}.",
                            args.len()
                        );
                    }
                    commands.push(ScriptVariableCommand {
                        command: command_name.to_string(),
                        target: None,
                        value_tokens: vec![args[0].to_string()],
                        source_script: script_name.clone(),
                        command_index: index,
                    });
                }
                _ => {}
            }
        }
    }
    Ok(commands)
}

fn parse_script_control_commands(
    map_name: &str,
    scripts: &BTreeMap<String, Value>,
) -> Result<Vec<ScriptControlCommand>> {
    let mut commands = Vec::new();
    for (script_name, payload) in scripts {
        let Some(entries) = payload.as_array() else {
            continue;
        };
        for (index, entry) in entries.iter().enumerate() {
            let Some(command_name) = entry.get("command").and_then(Value::as_str) else {
                continue;
            };
            match command_name {
                "ifequal" | "ifnotequal" | "ifgreater" | "ifless" => {
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if args.len() != 2 {
                        anyhow::bail!(
                            "Malformed {command_name} command in {script_name} for {map_name}: expected 2 args, found {}.",
                            args.len()
                        );
                    }
                    let target_label = args[1].to_string();
                    commands.push(ScriptControlCommand {
                        command: command_name.to_string(),
                        compare_value: Some(args[0].to_string()),
                        resolved_target_script: resolve_script_target_label(
                            scripts,
                            script_name,
                            args[1],
                        ),
                        target_label: Some(target_label),
                        source_script: script_name.clone(),
                        command_index: index,
                    });
                }
                "iftrue" | "iffalse" | "sjump" | "jump" | "scall" | "sdefer" => {
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if args.len() != 1 {
                        anyhow::bail!(
                            "Malformed {command_name} command in {script_name} for {map_name}: expected 1 arg, found {}.",
                            args.len()
                        );
                    }
                    let target_label = args[0].to_string();
                    commands.push(ScriptControlCommand {
                        command: command_name.to_string(),
                        compare_value: None,
                        resolved_target_script: resolve_script_target_label(
                            scripts,
                            script_name,
                            args[0],
                        ),
                        target_label: Some(target_label),
                        source_script: script_name.clone(),
                        command_index: index,
                    });
                }
                "jumpstd" => {
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if args.len() != 1 {
                        anyhow::bail!(
                            "Malformed jumpstd command in {script_name} for {map_name}: expected 1 arg, found {}.",
                            args.len()
                        );
                    }
                    commands.push(ScriptControlCommand {
                        command: command_name.to_string(),
                        compare_value: None,
                        target_label: Some(args[0].to_string()),
                        resolved_target_script: None,
                        source_script: script_name.clone(),
                        command_index: index,
                    });
                }
                "end" | "endcallback" | "endifjustbattled" => {
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if !args.is_empty() {
                        anyhow::bail!(
                            "Malformed {command_name} command in {script_name} for {map_name}: expected 0 args, found {}.",
                            args.len()
                        );
                    }
                    commands.push(ScriptControlCommand {
                        command: command_name.to_string(),
                        compare_value: None,
                        target_label: None,
                        resolved_target_script: None,
                        source_script: script_name.clone(),
                        command_index: index,
                    });
                }
                _ => {}
            }
        }
    }
    Ok(commands)
}

fn resolve_script_target_label(
    scripts: &BTreeMap<String, Value>,
    source_script: &str,
    target_label: &str,
) -> Option<String> {
    if scripts.contains_key(target_label) {
        return Some(target_label.to_string());
    }
    if target_label.starts_with('.') {
        let parent_script = script_label_parent(source_script);
        let local = format!("{target_label}@{parent_script}");
        if scripts.contains_key(&local) {
            return Some(local);
        }
    }
    None
}

fn script_label_parent(source_script: &str) -> &str {
    source_script
        .rsplit_once('@')
        .map(|(_, parent)| parent)
        .unwrap_or(source_script)
}

fn parse_script_movements(
    map_name: &str,
    scripts: &BTreeMap<String, Value>,
    object_commands: &[ScriptObjectCommand],
) -> Result<Vec<ScriptMovement>> {
    let mut movements = Vec::new();
    let movement_refs: BTreeSet<(&str, &str)> = object_commands
        .iter()
        .filter(|command| SCRIPT_OBJECT_MOVEMENT_COMMANDS.contains(&command.command.as_str()))
        .filter_map(|command| {
            command
                .movement
                .as_deref()
                .map(|movement| (movement, command.source_script.as_str()))
        })
        .collect();
    for (movement_label, source_script) in movement_refs {
        let parent_script = script_label_parent(source_script);
        let local_label = format!("{movement_label}@{parent_script}");
        let (script_key, source_script) = if scripts.contains_key(movement_label) {
            (movement_label, None)
        } else if scripts.contains_key(&local_label) {
            (local_label.as_str(), Some(parent_script.to_string()))
        } else {
            continue;
        };
        let Some(payload) = scripts.get(script_key) else {
            continue;
        };
        let Some(entries) = payload.as_array() else {
            continue;
        };
        let mut steps = Vec::new();
        for (index, entry) in entries.iter().enumerate() {
            let Some(command_name) = entry.get("command").and_then(Value::as_str) else {
                continue;
            };
            if !is_known_script_movement_command(command_name) {
                anyhow::bail!(
                    "Malformed movement script {script_key} for {map_name}: non-movement command '{command_name}' at index {index}."
                );
            }
            let args = script_command_args(map_name, script_key, command_name, entry)?;
            let (direction, duration) = match command_name {
                command if SCRIPT_MOVEMENT_DIRECTION_COMMANDS.contains(&command) => {
                    if args.len() != 1 {
                        anyhow::bail!(
                            "Malformed {command_name} movement in {script_key} for {map_name}: expected 1 arg, found {}.",
                            args.len()
                        );
                    }
                    (Some(args[0].to_string()), None)
                }
                "step_sleep" => {
                    if args.len() > 1 {
                        anyhow::bail!(
                            "Malformed step_sleep movement in {script_key} for {map_name}: expected 0 or 1 args, found {}.",
                            args.len()
                        );
                    }
                    let duration = if let Some(duration) = args.first() {
                        Some(parse_script_u16(duration)?)
                    } else {
                        None
                    };
                    (None, duration)
                }
                command if SCRIPT_MOVEMENT_NO_ARG_COMMANDS.contains(&command) => {
                    if !args.is_empty() {
                        anyhow::bail!(
                            "Malformed {command_name} movement in {script_key} for {map_name}: expected 0 args, found {}.",
                            args.len()
                        );
                    }
                    (None, None)
                }
                _ => unreachable!("movement command checked above"),
            };
            steps.push(ScriptMovementStep {
                command: command_name.to_string(),
                direction,
                duration,
                index,
            });
        }
        movements.push(ScriptMovement {
            label: movement_label.to_string(),
            source_script,
            steps,
        });
    }
    Ok(movements)
}

fn parse_scripted_trainer_battles(
    map_name: &str,
    scripts: &BTreeMap<String, Value>,
) -> Result<Vec<ScriptedTrainerBattle>> {
    let mut scripted_battles = Vec::new();
    for (script_name, payload) in scripts {
        let Some(entries) = payload.as_array() else {
            continue;
        };
        let mut last_win_text = String::new();
        let mut last_loss_text = String::new();
        let mut battle_type = "BATTLETYPE_TRAINER".to_string();
        let mut pending: Option<PendingScriptedTrainerBattle> = None;

        for (index, entry) in entries.iter().enumerate() {
            let Some(command_name) = entry.get("command").and_then(Value::as_str) else {
                continue;
            };
            match command_name {
                "winlosstext" => {
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if args.len() != 2 {
                        anyhow::bail!(
                            "Malformed winlosstext command in {script_name} for {map_name}: expected 2 args, found {}.",
                            args.len()
                        );
                    }
                    last_win_text = trainer_command_optional_arg(args[0]);
                    last_loss_text = trainer_command_optional_arg(args[1]);
                }
                "loadvar" => {
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if args.len() == 2 && args[0] == "VAR_BATTLETYPE" {
                        battle_type = args[1].to_string();
                    }
                }
                "loadtrainer" => {
                    if let Some(done) = pending.take() {
                        scripted_battles.push(done.into_battle(map_name)?);
                    }
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if args.len() != 2 {
                        anyhow::bail!(
                            "Malformed loadtrainer command in {script_name} for {map_name}: expected 2 args, found {}.",
                            args.len()
                        );
                    }
                    let mut request = TrainerBattleRequest::new(args[0], args[1], "");
                    request.battle_type = battle_type.clone();
                    request.win_text = last_win_text.clone();
                    request.loss_text = last_loss_text.clone();
                    request.source_script = script_name.clone();
                    pending = Some(PendingScriptedTrainerBattle {
                        source_script: script_name.clone(),
                        loadtrainer_command_index: index,
                        startbattle_command_index: None,
                        request,
                        reload_map_after_battle: false,
                        post_battle_event_flags: Vec::new(),
                        post_battle_script_flags: Vec::new(),
                    });
                }
                "startbattle" => {
                    if let Some(pending) = pending.as_mut() {
                        pending.startbattle_command_index = Some(index);
                    }
                }
                "reloadmapafterbattle" => {
                    if let Some(pending) = pending.as_mut() {
                        pending.reload_map_after_battle = true;
                    }
                }
                "setevent" => {
                    if let Some(pending) = pending.as_mut() {
                        let args = script_command_args(map_name, script_name, command_name, entry)?;
                        if args.len() != 1 {
                            anyhow::bail!(
                                "Malformed setevent command in {script_name} for {map_name}: expected 1 arg, found {}.",
                                args.len()
                            );
                        }
                        pending.post_battle_event_flags.push(args[0].to_string());
                    }
                }
                "setflag" => {
                    if let Some(pending) = pending.as_mut() {
                        let args = script_command_args(map_name, script_name, command_name, entry)?;
                        if args.len() != 1 {
                            anyhow::bail!(
                                "Malformed setflag command in {script_name} for {map_name}: expected 1 arg, found {}.",
                                args.len()
                            );
                        }
                        pending.post_battle_script_flags.push(args[0].to_string());
                    }
                }
                _ => {}
            }
        }

        if let Some(done) = pending {
            scripted_battles.push(done.into_battle(map_name)?);
        }
    }

    Ok(scripted_battles)
}

struct PendingScriptedTrainerBattle {
    source_script: String,
    loadtrainer_command_index: usize,
    startbattle_command_index: Option<usize>,
    request: TrainerBattleRequest,
    reload_map_after_battle: bool,
    post_battle_event_flags: Vec<String>,
    post_battle_script_flags: Vec<String>,
}

impl PendingScriptedTrainerBattle {
    fn into_battle(self, map_name: &str) -> Result<ScriptedTrainerBattle> {
        let startbattle_command_index = self.startbattle_command_index.with_context(|| {
            format!(
                "loadtrainer command in {} for {map_name} is not followed by startbattle",
                self.source_script
            )
        })?;
        Ok(ScriptedTrainerBattle {
            source_script: self.source_script,
            loadtrainer_command_index: self.loadtrainer_command_index,
            startbattle_command_index,
            request: self.request,
            reload_map_after_battle: self.reload_map_after_battle,
            post_battle_event_flags: self.post_battle_event_flags,
            post_battle_script_flags: self.post_battle_script_flags,
        })
    }
}

fn parse_scripted_wild_battles(
    map_name: &str,
    scripts: &BTreeMap<String, Value>,
) -> Result<Vec<ScriptedWildBattle>> {
    let mut scripted_battles = Vec::new();
    for (script_name, payload) in scripts {
        let Some(entries) = payload.as_array() else {
            continue;
        };
        let mut battle_type = "BATTLETYPE_NORMAL".to_string();
        let mut event_flags_since_last_battle = Vec::new();
        let mut pending: Option<PendingScriptedWildBattle> = None;

        for (index, entry) in entries.iter().enumerate() {
            let Some(command_name) = entry.get("command").and_then(Value::as_str) else {
                continue;
            };
            match command_name {
                "loadvar" => {
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if args.len() == 2 && args[0] == "VAR_BATTLETYPE" {
                        battle_type = args[1].to_string();
                        if let Some(pending) = pending.as_mut() {
                            pending.request.battle_type = battle_type.clone();
                        }
                    }
                }
                "loadwildmon" => {
                    if let Some(done) = pending.take() {
                        if let Some(battle) = done.into_battle() {
                            scripted_battles.push(battle);
                        }
                    }
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if args.len() != 2 {
                        anyhow::bail!(
                            "Malformed loadwildmon command in {script_name} for {map_name}: expected 2 args, found {}.",
                            args.len()
                        );
                    }
                    let level = parse_script_u16(args[1])?;
                    let level = u8::try_from(level).with_context(|| {
                        format!(
                            "loadwildmon level '{}' in {script_name} for {map_name} is outside u8 range",
                            args[1]
                        )
                    })?;
                    let mut request = StaticWildBattleRequest::new(args[0], level);
                    request.battle_type = battle_type.clone();
                    request.source_script = script_name.clone();
                    pending = Some(PendingScriptedWildBattle {
                        source_script: script_name.clone(),
                        loadwildmon_command_index: index,
                        startbattle_command_index: None,
                        request,
                        reload_map_after_battle: false,
                        pre_battle_event_flags: event_flags_since_last_battle.clone(),
                        post_battle_event_flags: Vec::new(),
                        post_battle_script_flags: Vec::new(),
                        disappear_object_ids: Vec::new(),
                    });
                }
                "startbattle" => {
                    if let Some(pending) = pending.as_mut() {
                        pending.startbattle_command_index = Some(index);
                    }
                }
                "reloadmapafterbattle" => {
                    if let Some(pending) = pending.as_mut() {
                        pending.reload_map_after_battle = true;
                    }
                }
                "setevent" => {
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if args.len() != 1 {
                        anyhow::bail!(
                            "Malformed setevent command in {script_name} for {map_name}: expected 1 arg, found {}.",
                            args.len()
                        );
                    }
                    if let Some(pending) = pending.as_mut() {
                        pending.post_battle_event_flags.push(args[0].to_string());
                    } else {
                        event_flags_since_last_battle.push(args[0].to_string());
                    }
                }
                "setflag" => {
                    if let Some(pending) = pending.as_mut() {
                        let args = script_command_args(map_name, script_name, command_name, entry)?;
                        if args.len() != 1 {
                            anyhow::bail!(
                                "Malformed setflag command in {script_name} for {map_name}: expected 1 arg, found {}.",
                                args.len()
                            );
                        }
                        pending.post_battle_script_flags.push(args[0].to_string());
                    }
                }
                "disappear" => {
                    if let Some(pending) = pending.as_mut() {
                        let args = script_command_args(map_name, script_name, command_name, entry)?;
                        if args.len() != 1 {
                            anyhow::bail!(
                                "Malformed disappear command in {script_name} for {map_name}: expected 1 arg, found {}.",
                                args.len()
                            );
                        }
                        pending.disappear_object_ids.push(args[0].to_string());
                    }
                }
                _ => {}
            }
        }

        if let Some(done) = pending {
            if let Some(battle) = done.into_battle() {
                scripted_battles.push(battle);
            }
        }
    }
    Ok(scripted_battles)
}

struct PendingScriptedWildBattle {
    source_script: String,
    loadwildmon_command_index: usize,
    startbattle_command_index: Option<usize>,
    request: StaticWildBattleRequest,
    reload_map_after_battle: bool,
    pre_battle_event_flags: Vec<String>,
    post_battle_event_flags: Vec<String>,
    post_battle_script_flags: Vec<String>,
    disappear_object_ids: Vec<String>,
}

impl PendingScriptedWildBattle {
    fn into_battle(self) -> Option<ScriptedWildBattle> {
        let startbattle_command_index = self.startbattle_command_index?;
        Some(ScriptedWildBattle {
            source_script: self.source_script,
            loadwildmon_command_index: self.loadwildmon_command_index,
            startbattle_command_index,
            request: self.request,
            reload_map_after_battle: self.reload_map_after_battle,
            pre_battle_event_flags: self.pre_battle_event_flags,
            post_battle_event_flags: self.post_battle_event_flags,
            post_battle_script_flags: self.post_battle_script_flags,
            disappear_object_ids: self.disappear_object_ids,
        })
    }
}

fn script_command_args<'a>(
    map_name: &str,
    script_name: &str,
    command_name: &str,
    entry: &'a Value,
) -> Result<Vec<&'a str>> {
    let args = entry
        .get("args")
        .and_then(Value::as_array)
        .with_context(|| {
            format!(
                "Malformed {command_name} command in {script_name} for {map_name}: args must be an array."
            )
        })?;
    args.iter()
        .enumerate()
        .map(|(index, value)| {
            value.as_str().with_context(|| {
                format!(
                    "Malformed {command_name} command in {script_name} for {map_name}: arg {index} must be a string."
                )
            })
        })
        .collect()
}

fn trainer_command_optional_arg(value: &str) -> String {
    if value == "0" {
        String::new()
    } else {
        value.to_string()
    }
}

fn parse_map_scene_table(map_name: &str, payload: &Value) -> Result<MapSceneTable> {
    let commands: Vec<ScriptCommand> =
        serde_json::from_value(payload.clone()).context("parse map scene command list")?;
    let mut in_scene_section = false;
    let mut scenes = Vec::new();

    for command in commands {
        match command.command.as_str() {
            "def_scene_scripts" => {
                in_scene_section = true;
            }
            "scene_script" if in_scene_section => {
                if command.args.len() != 1 && command.args.len() != 2 {
                    anyhow::bail!(
                        "Malformed scene_script in {map_name}: expected 1 or 2 args, found {}.",
                        command.args.len()
                    );
                }
                if command.args.len() == 1 {
                    continue;
                }
                scenes.push(MapScene {
                    script_name: Some(command.args[0].clone()),
                    scene_id: command.args[1].clone(),
                });
            }
            "scene_const" if in_scene_section => {
                if command.args.len() != 1 {
                    anyhow::bail!(
                        "Malformed scene_const in {map_name}: expected 1 arg, found {}.",
                        command.args.len()
                    );
                }
                scenes.push(MapScene {
                    scene_id: command.args[0].clone(),
                    script_name: None,
                });
            }
            "def_callbacks" | "def_warp_events" | "def_coord_events" | "def_bg_events"
            | "def_object_events" => {
                in_scene_section = false;
            }
            _ => {}
        }
    }

    Ok(MapSceneTable { scenes })
}

fn parse_script_i32(token: &str) -> Result<i32> {
    let trimmed = token.trim();
    if trimmed.is_empty() {
        anyhow::bail!("cannot parse an empty numeric token");
    }
    let (sign, raw) = match trimmed.as_bytes()[0] {
        b'-' => (-1, &trimmed[1..]),
        b'+' => (1, &trimmed[1..]),
        _ => (1, trimmed),
    };
    let (radix, digits) = if let Some(hex) = raw.strip_prefix('$') {
        (16, hex)
    } else if let Some(hex) = raw.strip_prefix("0x").or_else(|| raw.strip_prefix("0X")) {
        (16, hex)
    } else if let Some(binary) = raw.strip_prefix('%') {
        (2, binary)
    } else {
        (10, raw)
    };
    if digits.is_empty() {
        anyhow::bail!("numeric token '{token}' does not contain digits");
    }
    let parsed = i32::from_str_radix(digits, radix)
        .with_context(|| format!("parse numeric token '{token}'"))?;
    Ok(sign * parsed)
}

fn parse_script_u16(token: &str) -> Result<u16> {
    let value = parse_script_i32(token)?;
    u16::try_from(value).with_context(|| format!("numeric token '{token}' is outside u16 range"))
}

fn resolve_gift_level_token(
    map_name: &str,
    token: &str,
    constants: &StoryEventScriptConstants,
) -> Result<u8> {
    let value = if let Ok(value) = parse_script_i32(token) {
        i64::from(value)
    } else if let Some(value) = constants.maps.get(map_name).and_then(|map| map.get(token)) {
        *value
    } else if let Some(value) = constants.global.get(token) {
        *value
    } else {
        anyhow::bail!("gift level token '{token}' does not resolve from pack constants");
    };
    let level = u8::try_from(value)
        .with_context(|| format!("gift level token '{token}' is outside u8 range"))?;
    if level == 0 {
        anyhow::bail!("gift level token '{token}' resolves to zero");
    }
    Ok(level)
}

fn script_warp_target_map(constant: &str) -> String {
    constant.to_string()
}

fn push_flattened(target: &mut Vec<Value>, payload: Value) {
    if let Some(array) = payload.as_array() {
        target.extend(array.iter().cloned());
    } else {
        target.push(payload);
    }
}

fn item_key(item: &Item) -> Result<String> {
    if item.script_name.trim().is_empty() {
        anyhow::bail!("item '{}' is missing explicit script_name", item.name);
    } else {
        Ok(item.script_name.clone())
    }
}

fn validate_manifest_item(item: &Item) -> Result<()> {
    if item.pocket == ITEM_POCKET_TM_HM && item.tmhm_index.is_none() {
        anyhow::bail!(
            "TM/HM item '{}' must declare explicit tmhm_index",
            item.script_name
        );
    }
    Ok(())
}

fn resolve_collision_token(token: &str) -> Result<u8> {
    let trimmed = token.trim();
    if !trimmed.is_empty() && trimmed.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return u8::from_str_radix(trimmed, 16)
            .with_context(|| format!("invalid hexadecimal collision token {trimmed}"));
    }
    Ok(match token.trim() {
        "FLOOR" => permissions::FLOOR,
        "04" => 0x04,
        "WALL" => permissions::WALL,
        "TALL_GRASS" => permissions::TALL_GRASS,
        "WATER_21" => 0x21,
        "WATER" => permissions::WATER,
        "ICE" => 0x23,
        "ICE_2B" => 0x2b,
        "WHIRLPOOL" => permissions::WHIRLPOOL,
        "WATERFALL_RIGHT" => permissions::WATERFALL_RIGHT,
        "WATERFALL_LEFT" => permissions::WATERFALL_LEFT,
        "WATERFALL_UP" => permissions::WATERFALL_UP,
        "WATERFALL" => permissions::WATERFALL,
        "UP_WALL" => permissions::UP_WALL,
        "DOWN_WALL" => permissions::DOWN_WALL,
        "LEFT_WALL" => permissions::LEFT_WALL,
        "RIGHT_WALL" => permissions::RIGHT_WALL,
        "DOOR" => permissions::DOOR,
        "DOOR_75" => permissions::DOOR_75,
        "DOOR_79" => permissions::DOOR_79,
        "DOOR_7D" => permissions::DOOR_7D,
        "LADDER" => permissions::LADDER,
        "STAIRCASE" => permissions::STAIRCASE,
        "STAIRCASE_73" => permissions::STAIRCASE_73,
        "CAVE" => permissions::CAVE,
        "CAVE_74" => permissions::CAVE_74,
        "COUNTER" => permissions::COUNTER,
        "COUNTER_98" => permissions::COUNTER_98,
        "BOOKSHELF" => permissions::BOOKSHELF,
        "PC" => permissions::PC,
        "RADIO" => permissions::RADIO,
        "TOWN_MAP" => permissions::TOWN_MAP,
        "MART_SHELF" => permissions::MART_SHELF,
        "TV" => permissions::TV,
        "WINDOW" => permissions::WINDOW,
        "INCENSE_BURNER" => permissions::INCENSE_BURNER,
        "BUOY" => 0x27,
        "VIRTUAL_BOY" => 0x61,
        "CUT_TREE" => 0x12,
        "HEADBUTT_TREE" => 0x15,
        "HOP_RIGHT" => permissions::HOP_RIGHT,
        "HOP_LEFT" => permissions::HOP_LEFT,
        "HOP_UP" => permissions::HOP_UP,
        "HOP_DOWN" => permissions::HOP_DOWN,
        "HOP_DOWN_RIGHT" => permissions::HOP_DOWN_RIGHT,
        "HOP_DOWN_LEFT" => permissions::HOP_DOWN_LEFT,
        "HOP_UP_RIGHT" => permissions::HOP_UP_RIGHT,
        "HOP_UP_LEFT" => permissions::HOP_UP_LEFT,
        "BRAKE" => 0x40,
        "BRAKE_45" => 0x45,
        "BRAKE_46" => 0x46,
        "BRAKE_47" => 0x47,
        "BRAKE_ALT" => 0x54,
        "BRAKE_55" => 0x55,
        "BRAKE_56" => 0x56,
        "BRAKE_57" => 0x57,
        "5B" => 0x5b,
        "PIT" => permissions::PIT,
        "PIT_68" => permissions::PIT_68,
        "WARP_CARPET_DOWN" => permissions::WARP_CARPET_DOWN,
        "WARP_CARPET_UP" => permissions::WARP_CARPET_UP,
        "WARP_CARPET_LEFT" => permissions::WARP_CARPET_LEFT,
        "WARP_CARPET_RIGHT" => permissions::WARP_CARPET_RIGHT,
        "WARP_PANEL" => permissions::WARP_PANEL,
        "WARP_77" => permissions::WARP_77,
        "WARP_7F" => permissions::WARP_7F,
        "01" => 0x01,
        other => anyhow::bail!("unknown collision token {other}"),
    })
}

fn parse_metatile_id(id: &str) -> Result<usize> {
    usize::from_str_radix(id.trim(), 16).with_context(|| format!("parse hex metatile id '{id}'"))
}

fn decode_base64_bytes(input: &str) -> Result<Vec<u8>> {
    let mut out = Vec::with_capacity(input.len() * 3 / 4);
    let mut chunk = [0_u8; 4];
    let mut chunk_len = 0;
    let mut padding = 0;

    for byte in input.bytes().filter(|byte| !byte.is_ascii_whitespace()) {
        let value = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b'=' => {
                padding += 1;
                0
            }
            _ => anyhow::bail!("invalid base64 byte 0x{byte:02x}"),
        };
        chunk[chunk_len] = value;
        chunk_len += 1;
        if chunk_len != 4 {
            continue;
        }
        if padding > 2 {
            anyhow::bail!("invalid base64 padding");
        }
        out.push((chunk[0] << 2) | (chunk[1] >> 4));
        if padding < 2 {
            out.push((chunk[1] << 4) | (chunk[2] >> 2));
        }
        if padding == 0 {
            out.push((chunk[2] << 6) | chunk[3]);
        }
        chunk = [0; 4];
        chunk_len = 0;
        padding = 0;
    }

    if chunk_len != 0 {
        anyhow::bail!("base64 length is not a multiple of 4");
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crystal_core::map::MapConnection;
    use crystal_core::models::{
        BaseStats, Item, ability, egg_group, growth_rate, item_pocket, pokemon_type,
    };
    use crystal_core::random::Random;
    use crystal_core::state::GameState;
    use crystal_core::systems::economy::{
        AmountComparison, MoneyAccount, check_coins, check_money, take_money,
    };
    use crystal_core::systems::field_items::{
        FieldItemPickup, FieldItemPickupOutcome, FieldItemSource, pickup_field_item,
        pickup_script_field_item,
    };
    use crystal_core::systems::gift_pokemon::{GiftPokemonRequest, give_gift_pokemon};
    use crystal_core::systems::phone::PhoneContactRecord;
    use crystal_core::systems::script_blocks::apply_script_block_change;
    use crystal_core::systems::script_flags::{apply_script_flag_mutation, check_script_flag};
    use crystal_core::systems::script_items::{
        ScriptItemGrantOutcome, check_script_item, grant_script_item, take_script_item,
    };
    use crystal_core::systems::script_objects::{
        apply_script_movement, apply_script_object_mutation,
    };
    use crystal_core::systems::script_scenes::apply_script_scene_command;
    use crystal_core::systems::scripted_battles::{
        ScriptedBattleEffects, apply_scripted_battle_effects_to_session,
    };
    use crystal_core::world::collision::{
        MetatileCollision, PlayerTraversalState, TilesetCollision, can_enter_tile, permissions,
        sample_collision,
    };
    use crystal_core::world::encounters::EncounterMusicModifier;
    use crystal_core::world::encounters::{
        EncounterSurface, FieldEncounterData, FieldEncounterEntry, FieldEncounterTable, TimeOfDay,
        WildEncounter, WildEncounterTable, table_for_surface,
    };
    use crystal_core::world::map::{Direction, TilePosition};
    use crystal_core::world::movement::{StepOptions, StepOutcome};
    use crystal_core::world::session::{
        EncounterCheckOptions, OverworldSession, warp_tile_position,
    };

    fn test_item(id: &str) -> Item {
        Item {
            name: id.to_string(),
            description: String::new(),
            effect: "NONE".to_string(),
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
            escape_rope_mode: None,
            price: 0,
            held_effect: "HELD_NONE".to_string(),
            parameter: 0,
            property: String::new(),
            pocket: item_pocket("ITEM"),
            field_menu: "ITEMMENU_NOUSE".to_string(),
            field_usable: false,
            battle_menu: "ITEMMENU_NOUSE".to_string(),
            battle_usable: false,
            script_name: id.to_string(),
            consumable: false,
            tmhm_index: None,
            tmhm_move: None,
        }
    }

    fn species() -> PokemonSpecies {
        PokemonSpecies {
            id: "NEW_MON".to_string(),
            int_id: 252,
            base_stats: BaseStats::new(40, 50, 40, 60, 70, 50),
            type1: pokemon_type("ELECTRIC"),
            type2: pokemon_type("ELECTRIC"),
            catch_rate: 45,
            base_exp: 80,
            item1: None,
            item2: None,
            gender_ratio: 127,
            unknown1: 0,
            step_cycles_to_hatch: 20,
            unknown2: 0,
            growth_rate: growth_rate("GROWTH_MEDIUM_FAST"),
            egg_group1: egg_group("EGG_GROUND"),
            egg_group2: egg_group("EGG_GROUND"),
            tmhm_learnset: vec!["THUNDERBOLT".to_string()],
            ability: ability("NONE"),
            pic_size: 0,
            front_pic: 0,
            back_pic: 0,
            weight: 120,
        }
    }

    fn test_move(name: &str) -> Move {
        Move {
            name: name.to_string(),
            move_type: pokemon_type("NORMAL"),
            power: 40,
            accuracy: 100,
            pp: 35,
            effect: "NORMAL_HIT".to_string(),
            effect_chance: 0,
            stat: None,
            amount: None,
        }
    }

    fn test_battle_stat_multipliers() -> BattleStatMultiplierTables {
        let identity = crystal_core::battle::stats::BattleStatMultiplier {
            numerator: 1,
            denominator: 1,
        };
        BattleStatMultiplierTables {
            stat: vec![identity; 13],
            accuracy: vec![identity; 13],
        }
    }

    fn test_weather_modifiers() -> WeatherModifiers {
        WeatherModifiers {
            type_modifiers: vec![crystal_core::battle::damage::WeatherTypeModifier {
                weather: "WEATHER_RAIN".to_string(),
                move_type: pokemon_type("WATER"),
                multiplier: crystal_core::battle::damage::TypeMultiplier {
                    numerator: 3,
                    denominator: 2,
                },
            }],
            move_effect_modifiers: vec![crystal_core::battle::damage::WeatherMoveEffectModifier {
                weather: "WEATHER_RAIN".to_string(),
                move_effect: "EFFECT_SOLARBEAM".to_string(),
                multiplier: crystal_core::battle::damage::TypeMultiplier {
                    numerator: 1,
                    denominator: 2,
                },
            }],
        }
    }

    fn test_type_effectiveness() -> TypeEffectivenessTable {
        let types = ["NORMAL", "FIGHTING", "FIRE", "WATER"];
        let matchups = types
            .iter()
            .flat_map(|attacker| {
                types.iter().map(move |defender| {
                    crystal_core::battle::damage::TypeEffectivenessEntry {
                        attacker: pokemon_type(attacker),
                        defender: pokemon_type(defender),
                        multiplier: crystal_core::battle::damage::TypeMultiplier::one(),
                    }
                })
            })
            .collect();
        TypeEffectivenessTable {
            matchups,
            foresight_matchups: vec![crystal_core::battle::damage::TypeEffectivenessEntry {
                attacker: pokemon_type("NORMAL"),
                defender: pokemon_type("FIGHTING"),
                multiplier: crystal_core::battle::damage::TypeMultiplier::zero(),
            }],
        }
    }

    fn test_type_categories() -> TypeCategories {
        TypeCategories {
            physical: vec!["NORMAL".to_string(), "FIGHTING".to_string()],
            special: vec!["FIRE".to_string(), "WATER".to_string()],
        }
    }

    fn test_move_priorities() -> MovePriorityTable {
        MovePriorityTable {
            base_priority: 1,
            effect_priorities: vec![
                crystal_core::battle::turn::MoveEffectPriority {
                    move_effect: "PRIORITY_HIT".to_string(),
                    priority: 2,
                },
                crystal_core::battle::turn::MoveEffectPriority {
                    move_effect: "NORMAL_HIT".to_string(),
                    priority: 1,
                },
            ],
            move_priorities: vec![crystal_core::battle::turn::MovePriorityOverride {
                r#move: "VITAL_THROW".to_string(),
                priority: 0,
            }],
        }
    }

    fn test_battle_escape_rules() -> BattleEscapeRules {
        BattleEscapeRules {
            player_speed_multiplier: 32,
            enemy_speed_divisor: 4,
            failed_attempt_bonus: 30,
            rng_roll_values: 256,
        }
    }

    fn add_runtime_species_and_move(data: &mut GameDataSet) {
        let mut known_species = species();
        known_species.tmhm_learnset.clear();
        let species_id = known_species.id.clone();
        data.pokemon.insert(species_id.clone(), known_species);
        data.moves.insert("TACKLE".to_string(), test_move("TACKLE"));
        data.battle_stat_multipliers = test_battle_stat_multipliers();
        data.battle_escape_rules = test_battle_escape_rules();
        data.move_priorities = test_move_priorities();
        data.type_categories = test_type_categories();
        data.type_effectiveness = test_type_effectiveness();
        data.weather_modifiers = test_weather_modifiers();
        data.learnsets.insert(species_id.clone(), Vec::new());
        data.evolutions.0.insert(species_id.clone(), Vec::new());
        data.menu_icons
            .insert(species_id.clone(), "ICON_PIKACHU".to_string());
        data.pokedex_entries.insert(
            species_id.clone(),
            RuntimePokedexEntry {
                species: species_id.clone(),
                classification: "SPARK".to_string(),
                height_digits: 4,
                weight_digits: 60,
                pages: vec!["Stores static in its fur.".to_string()],
            },
        );
        data.pokemon_frontpic_anim.insert(
            species_id.clone(),
            FrontpicAnimProgram {
                commands: vec![FrontpicAnimCommand {
                    kind: "endanim".to_string(),
                    ..FrontpicAnimCommand::default()
                }],
            },
        );
        data.pokemon_cries.insert(
            species_id,
            PokemonCryMetadata {
                cry: "CRY_CHIKORITA".to_string(),
                pitch: 0,
                length: 0,
            },
        );
        data.audio.push(ModpackAudioAsset {
            id: "CRY_CHIKORITA".to_string(),
            path: "content-packs/core-modular/cries/CRY_CHIKORITA.mid".to_string(),
            kind: ModpackAudioKind::Cry,
        });
    }

    fn add_wild_encounter_marker(data: &mut GameDataSet) {
        data.wild_encounters.insert(
            "Route29".to_string(),
            WildEncounterData {
                map_name: "Route29".to_string(),
                grass_rates: Some([("day".to_string(), 30)].into_iter().collect()),
                water_rate: None,
                grass: None,
                water: None,
            },
        );
    }

    fn add_test_trainer(data: &mut GameDataSet, encounter_music: &str) {
        data.trainers.trainers.insert(
            "YOUNGSTER_JOEY".to_string(),
            Trainer {
                name: "Joey".to_string(),
                trainer_id: "YOUNGSTER_JOEY".to_string(),
                trainer_class: "YOUNGSTER".to_string(),
                party: Vec::new(),
                win_quote: "I won!".to_string(),
                lose_quote: "I lost!".to_string(),
                items: Vec::new(),
                base_reward: 4,
                ai_move_flags: 0,
                ai_item_switch_flags: 0,
                encounter_music: encounter_music.to_string(),
                ai_layers: Vec::new(),
            },
        );
    }

    #[test]
    fn verifier_rejects_missing_battle_escape_rules_without_formula_fallback() {
        let mut data = GameDataSet::default();
        add_runtime_species_and_move(&mut data);
        data.battle_escape_rules = BattleEscapeRules::default();

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.has_errors());
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "missing_battle_escape_rules"
                && diagnostic.subject == "battle_escape_rules"
        }));
    }

    #[test]
    fn verifier_requires_trainer_encounter_music_declared_by_pack() {
        let mut data = GameDataSet::default();
        add_test_trainer(&mut data, "");

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "missing_trainer_encounter_music"
                && diagnostic.subject == "YOUNGSTER_JOEY"
        }));
    }

    #[test]
    fn verifier_requires_trainer_encounter_music_reference_exact_music_asset() {
        let mut data = GameDataSet::default();
        add_test_trainer(&mut data, "MUSIC_YOUNGSTER_ENCOUNTER");
        data.audio.push(ModpackAudioAsset {
            id: "SFX_TACKLE".to_string(),
            path: "content-packs/test/sfx/SFX_TACKLE.mid".to_string(),
            kind: ModpackAudioKind::SoundEffect,
        });

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_trainer_encounter_music"
                && diagnostic.subject == "YOUNGSTER_JOEY"
        }));
    }

    #[test]
    fn verifier_requires_fly_field_move_from_exact_modpack_rule() {
        let mut data = GameDataSet::default();
        add_runtime_species_and_move(&mut data);
        data.field_moves.fly = FieldMoveRule {
            move_id: "fly".to_string(),
            badge: crystal_core::systems::field_moves::FieldMoveBadgeRequirement {
                region: "johto".to_string(),
                index: 5,
            },
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_field_move_id" && diagnostic.subject == "field_moves:fly"
        }));
    }

    #[test]
    fn verifier_requires_escape_rope_rule_match_exact_item_payload() {
        let mut data = GameDataSet::default();
        let mut item = test_item("ESCAPE_ROPE");
        item.effect = "ESCAPE_ROPE".to_string();
        item.escape_rope_mode = Some("DIG_WARP".to_string());
        data.items.insert("ESCAPE_ROPE".to_string(), item);
        data.field_moves.escape_rope = crystal_core::systems::field_moves::FieldEscapeItemRule {
            item_id: "MOD_ESCAPE_ROPE".to_string(),
            escape_rope_mode: "MOD_WARP".to_string(),
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_field_escape_item_rule"
                && diagnostic.subject == "field_moves:escape_rope"
        }));
    }

    #[test]
    fn verifier_requires_repel_rule_match_exact_item_payload() {
        let mut data = GameDataSet::default();
        let mut item = test_item("REPEL");
        item.effect = "REPEL".to_string();
        data.items.insert("REPEL".to_string(), item);
        data.field_moves.repel = crystal_core::systems::field_moves::FieldRepelItemRule {};

        let mut diagnostics = Vec::new();
        verify_field_repel_item_rule(&data, &mut diagnostics);

        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "missing_field_repel_item_payload"
                && diagnostic.subject == "field_moves:repel"
        }));
    }

    #[test]
    fn verifier_requires_bicycle_rule_match_exact_field_item_payload() {
        let mut data = GameDataSet::default();
        let mut item = test_item("BICYCLE");
        item.effect = "BICYCLE".to_string();
        item.field_menu = "ITEMMENU_CLOSE".to_string();
        data.items.insert("BICYCLE".to_string(), item);
        data.field_moves.bicycle = FieldItemRule {
            item_id: "MOD_BICYCLE".to_string(),
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_field_item_id"
                && diagnostic.subject == "field_moves:bicycle"
        }));
    }

    #[test]
    fn verifier_requires_field_key_item_rules_match_exact_item_payloads() {
        let mut data = GameDataSet::default();
        for (item_id, effect) in [
            ("ITEMFINDER", "ITEMFINDER"),
            ("SQUIRTBOTTLE", "SQUIRTBOTTLE"),
            ("COIN_CASE", "COIN_CASE"),
            ("BLUE_CARD", "BLUE_CARD"),
            ("TOWN_MAP", "TOWN_MAP"),
        ] {
            let mut item = test_item(item_id);
            item.effect = effect.to_string();
            item.field_menu = "ITEMMENU_CLOSE".to_string();
            data.items.insert(item_id.to_string(), item);
        }
        data.field_moves.itemfinder = FieldItemRule {
            item_id: "MOD_ITEMFINDER".to_string(),
        };
        data.field_moves.squirtbottle = FieldItemRule {
            item_id: "MOD_SQUIRTBOTTLE".to_string(),
        };
        data.field_moves.coin_case = FieldItemRule {
            item_id: "MOD_COIN_CASE".to_string(),
        };
        data.field_moves.blue_card = FieldItemRule {
            item_id: "MOD_BLUE_CARD".to_string(),
        };
        data.field_moves.town_map = FieldItemRule {
            item_id: "MOD_TOWN_MAP".to_string(),
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        for subject in [
            "field_moves:itemfinder",
            "field_moves:squirtbottle",
            "field_moves:coin_case",
            "field_moves:blue_card",
            "field_moves:town_map",
        ] {
            assert!(report.diagnostics.iter().any(|diagnostic| {
                diagnostic.code == "unknown_field_item_id" && diagnostic.subject == subject
            }));
        }
    }

    #[test]
    fn verifier_requires_encounter_music_modifiers_declared_by_pack() {
        let mut data = GameDataSet::default();
        add_wild_encounter_marker(&mut data);

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "missing_encounter_music_modifiers"
                && diagnostic.subject == "encounter_music_modifiers"
        }));
    }

    #[test]
    fn verifier_requires_encounter_music_modifiers_reference_exact_music_assets() {
        let mut data = GameDataSet::default();
        add_wild_encounter_marker(&mut data);
        data.audio.push(ModpackAudioAsset {
            id: "MUSIC_POKEMON_MARCH".to_string(),
            path: "content-packs/test/music/MUSIC_POKEMON_MARCH.mid".to_string(),
            kind: ModpackAudioKind::Music,
        });
        data.encounter_music_modifiers = EncounterMusicModifiers {
            modifiers: vec![
                EncounterMusicModifier {
                    music_id: "MUSIC_POKEMON_MARCH".to_string(),
                    numerator: 2,
                    denominator: 1,
                },
                EncounterMusicModifier {
                    music_id: "MUSIC_POKEMON_MARCH".to_string(),
                    numerator: 2,
                    denominator: 1,
                },
                EncounterMusicModifier {
                    music_id: "SFX_TACKLE".to_string(),
                    numerator: 1,
                    denominator: 0,
                },
            ],
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "duplicate_encounter_music_modifier_id"
                && diagnostic.subject == "encounter_music_modifiers:MUSIC_POKEMON_MARCH"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_encounter_music_modifier_id"
                && diagnostic.subject == "encounter_music_modifiers:SFX_TACKLE"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_encounter_music_modifier_ratio"
                && diagnostic.subject == "encounter_music_modifiers:SFX_TACKLE"
        }));
    }

    #[test]
    fn verifier_rejects_invalid_battle_escape_rules_from_pack() {
        let mut data = GameDataSet::default();
        add_runtime_species_and_move(&mut data);
        data.battle_escape_rules.player_speed_multiplier = 0;
        data.battle_escape_rules.enemy_speed_divisor = 0;
        data.battle_escape_rules.rng_roll_values = u16::from(u8::MAX) + 2;

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        for subject in [
            "battle_escape_rules:player_speed_multiplier",
            "battle_escape_rules:enemy_speed_divisor",
            "battle_escape_rules:rng_roll_values",
        ] {
            assert!(
                report.diagnostics.iter().any(|diagnostic| {
                    diagnostic.code == "invalid_battle_escape_rule" && diagnostic.subject == subject
                }),
                "missing invalid battle escape diagnostic for {subject}"
            );
        }
    }

    fn test_map_module(id: &str, map_constant: &str, connection_target: Option<&str>) -> MapModule {
        MapModule {
            id: id.to_string(),
            attributes: MapAttributes {
                tileset_name: "johto".to_string(),
                border_block: 0,
                width: 1,
                height: 1,
                connections: connection_target
                    .map(|target| {
                        vec![MapConnection {
                            direction: "east".to_string(),
                            target_map: target.to_string(),
                            offset: 0,
                        }]
                    })
                    .unwrap_or_default(),
                time_of_day: None,
                phone_service: 0,
                phone_flag: false,
                environment: Some("route".to_string()),
                location: Some("johto".to_string()),
                music: None,
                palette: None,
                fishing_group: None,
                map_constant: Some(map_constant.to_string()),
                map_group_constant: None,
                blocks_label: None,
                map_scripts_label: None,
                map_events_label: None,
                connection_flags: None,
            },
            scripts: BTreeMap::new(),
            trainer_scripts: BTreeMap::new(),
            scripted_trainer_battles: Vec::new(),
            scripted_wild_battles: Vec::new(),
            script_item_grants: Vec::new(),
            script_item_checks: Vec::new(),
            script_item_takes: Vec::new(),
            script_economy_commands: Vec::new(),
            gift_pokemon_scripts: Vec::new(),
            script_flag_commands: Vec::new(),
            script_scene_commands: Vec::new(),
            script_audio_commands: Vec::new(),
            script_block_changes: Vec::new(),
            script_object_commands: Vec::new(),
            script_movements: Vec::new(),
            script_map_commands: Vec::new(),
            script_text_commands: Vec::new(),
            script_text_bodies: BTreeMap::new(),
            script_menu_definitions: BTreeMap::new(),
            script_variable_commands: Vec::new(),
            script_control_commands: Vec::new(),
            script_field_pickups: Vec::new(),
            script_shop_commands: Vec::new(),
            script_phone_commands: Vec::new(),
            script_runtime_commands: Vec::new(),
            map_script_section_commands: Vec::new(),
            map_event_section_commands: Vec::new(),
            scenes: MapSceneTable::default(),
            events: MapEvents::default(),
            objects: Vec::new(),
            blocks: vec![1],
        }
    }

    fn assert_map_module_requires_field(field: &'static str) {
        let module = test_map_module("NewRoute", "NEW_ROUTE", None);
        let mut json = serde_json::to_value(module).expect("serialize full map module");
        json.as_object_mut()
            .expect("map module json object")
            .remove(field)
            .unwrap_or_else(|| panic!("fixture must include {field}"));

        let error = serde_json::from_value::<MapModule>(json)
            .expect_err("map module fields must be explicit, even when empty")
            .to_string();
        let expected = format!("missing field `{field}`");
        assert!(error.contains(&expected), "{error}");
    }

    #[test]
    fn map_module_json_requires_explicit_script_sections() {
        assert_map_module_requires_field("scripts");
        assert_map_module_requires_field("trainer_scripts");
        assert_map_module_requires_field("scripted_trainer_battles");
        assert_map_module_requires_field("script_phone_commands");
        assert_map_module_requires_field("script_runtime_commands");
        assert_map_module_requires_field("map_script_section_commands");
        assert_map_module_requires_field("map_event_section_commands");
    }

    #[test]
    fn map_module_json_rejects_unknown_nested_script_command_fields() {
        let mut module = test_map_module("NewRoute", "NEW_ROUTE", None);
        module.script_audio_commands = vec![ScriptAudioCommand {
            command: "playmusic".to_string(),
            audio_id: Some("MUSIC_ROUTE_29".to_string()),
            fade_frames: None,
            source_script: "NewRouteScript".to_string(),
            command_index: 0,
        }];
        let mut json = serde_json::to_value(module).expect("serialize full map module");
        let command = json["script_audio_commands"]
            .as_array_mut()
            .expect("audio commands")
            .first_mut()
            .expect("first audio command")
            .as_object_mut()
            .expect("audio command object");
        command.insert(
            "mp3".to_string(),
            Value::String("music/route29.mp3".to_string()),
        );

        let error = serde_json::from_value::<MapModule>(json)
            .expect_err("nested script command fields must be definitive")
            .to_string();
        assert!(error.contains("unknown field `mp3`"), "{error}");
    }

    fn test_object(object_id: &str, event_flag: &str, x: u16, y: u16) -> ObjectEvent {
        ObjectEvent {
            sprite: "SPRITE_MON".to_string(),
            x,
            y,
            spritemovedata: "SPRITEMOVEDATA_STANDING_DOWN".to_string(),
            move_range_x: 0,
            move_range_y: 0,
            hram_x: -1,
            hram_y: -1,
            pal: 0,
            object_type: "OBJECTTYPE_SCRIPT".to_string(),
            radius: 0,
            script: "ObjectScript".to_string(),
            label: None,
            event_flag: event_flag.to_string(),
            object_identifier: Some(object_id.to_string()),
            sightline_direction_override: None,
        }
    }

    fn temp_test_path(name: &str) -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "crystal-assets-{}-{unique}-{name}",
            std::process::id()
        ))
    }

    #[test]
    fn verifier_rejects_unknown_object_movement_data_without_direction_fallback() {
        let mut module = test_map_module("Start", "START_MAP", None);
        module.scripts = BTreeMap::from([("ObjectScript".to_string(), Value::Array(Vec::new()))]);
        let mut object = test_object("START_OBJECT", "EVENT_START_OBJECT", 0, 0);
        object.spritemovedata = "spritemovedata_standing_down".to_string();
        module.objects = vec![object];
        let mut middle = test_map_module("Middle", "MIDDLE_MAP", None);
        middle.attributes.width = 2;
        middle.blocks = vec![1, 1];
        let data = GameDataSet {
            maps: [("Start".to_string(), module)].into_iter().collect(),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_object_movement_data"
                && diagnostic.subject == "Start:START_OBJECT"
                && diagnostic.message.contains("spritemovedata_standing_down")
        }));
    }

    fn empty_content_pack_files_json() -> serde_json::Map<String, Value> {
        let mut json = serde_json::Map::new();
        for category in CONTENT_PACK_CATEGORIES {
            json.insert(category.as_str().to_string(), Value::Array(Vec::new()));
        }
        json
    }

    fn content_pack_json(id: &str, enabled: bool, priority: i32) -> Value {
        serde_json::json!({
            "id": id,
            "enabled": enabled,
            "priority": priority,
            "path": format!("content-packs/{id}"),
            "compiled": null,
            "files": Value::Object(empty_content_pack_files_json()),
        })
    }

    #[test]
    fn content_pack_index_requires_explicit_pack_metadata_and_sorts_enabled_packs() {
        let index: ContentPackIndex = serde_json::from_value(serde_json::json!({
          "version": 1,
          "packs": [
            content_pack_json("late", true, 10),
            content_pack_json("disabled", false, -100),
            content_pack_json("early", true, -10)
          ]
        }))
        .expect("parse content pack index");

        assert_eq!(index.version, 1);
        let ids: Vec<&str> = index
            .enabled_packs_sorted()
            .into_iter()
            .map(|pack| pack.id.as_str())
            .collect();
        assert_eq!(ids, vec!["early", "late"]);

        let missing_version = serde_json::from_value::<ContentPackIndex>(serde_json::json!({
            "packs": []
        }))
        .expect_err("content pack index version must be explicit")
        .to_string();
        assert!(
            missing_version.contains("missing field `version`"),
            "{missing_version}"
        );

        let mut missing_compiled = content_pack_json("missing-compiled", true, 0);
        missing_compiled
            .as_object_mut()
            .expect("pack object")
            .remove("compiled");
        let missing_compiled = serde_json::from_value::<ContentPack>(missing_compiled)
            .expect_err("content pack compiled field must be explicit, even when null")
            .to_string();
        assert!(
            missing_compiled.contains("missing field `compiled`"),
            "{missing_compiled}"
        );
    }

    #[test]
    fn content_pack_files_keep_existing_json_categories_and_add_game_asset_categories() {
        let mut json = empty_content_pack_files_json();
        json.insert(
            "pokemon".to_string(),
            serde_json::json!(["mods/new/pokemon.json"]),
        );
        json.insert(
            "map_attributes".to_string(),
            serde_json::json!(["mods/new/map_attributes.json"]),
        );
        json.insert(
            "audio".to_string(),
            serde_json::json!([
                "mods/new/music/route29.mid",
                "mods/new/sfx/tackle.mid",
                "mods/new/cries/nidoran_m.mid"
            ]),
        );
        json.insert(
            "tilesets".to_string(),
            serde_json::json!(["mods/new/tilesets.json"]),
        );
        json.insert(
            "playability".to_string(),
            serde_json::json!(["mods/new/playability/main.json"]),
        );

        let files: ContentPackFiles =
            serde_json::from_value(Value::Object(json.clone())).expect("parse files");

        assert_eq!(
            files.entries(ContentPackCategory::Pokemon),
            &["mods/new/pokemon.json".to_string()]
        );
        assert_eq!(
            files.entries(ContentPackCategory::Audio),
            &[
                "mods/new/music/route29.mid".to_string(),
                "mods/new/sfx/tackle.mid".to_string(),
                "mods/new/cries/nidoran_m.mid".to_string(),
            ]
        );
        assert_eq!(
            files.entries(ContentPackCategory::Playability),
            &["mods/new/playability/main.json".to_string()]
        );
        assert!(files.entries(ContentPackCategory::Moves).is_empty());

        json.remove("moves");
        let error = serde_json::from_value::<ContentPackFiles>(Value::Object(json))
            .expect_err("content pack file categories must be explicit, even when empty")
            .to_string();
        assert!(error.contains("missing field `moves`"), "{error}");
    }

    fn empty_compiled_content_pack_json() -> Value {
        let mut categories = serde_json::Map::new();
        for category in CONTENT_PACK_CATEGORIES {
            categories.insert(category.as_str().to_string(), Value::Array(Vec::new()));
        }
        serde_json::json!({
            "version": 1,
            "packId": "strict-pack",
            "categories": Value::Object(categories),
        })
    }

    #[test]
    fn compiled_content_pack_requires_every_declared_category() {
        let mut json = empty_compiled_content_pack_json();
        json.pointer_mut("/categories")
            .and_then(Value::as_object_mut)
            .expect("categories object")
            .remove("fishing");

        let error = serde_json::from_value::<CompiledContentPack>(json)
            .expect_err("compiled packs must not infer missing categories")
            .to_string();

        assert!(error.contains("missing field `fishing`"), "{error}");
    }

    #[test]
    fn compiled_content_pack_rejects_unknown_categories() {
        let mut json = empty_compiled_content_pack_json();
        json.pointer_mut("/categories")
            .and_then(Value::as_object_mut)
            .expect("categories object")
            .insert("legacy_json".to_string(), Value::Array(Vec::new()));

        let error = serde_json::from_value::<CompiledContentPack>(json)
            .expect_err("compiled packs must not accept undeclared categories")
            .to_string();

        assert!(error.contains("unknown field `legacy_json`"), "{error}");
    }

    #[test]
    fn compiled_content_pack_rejects_legacy_pack_id_alias() {
        let mut json = empty_compiled_content_pack_json();
        let object = json.as_object_mut().expect("compiled pack object");
        let pack_id = object.remove("packId").expect("packId");
        object.insert("pack_id".to_string(), pack_id);

        let error = serde_json::from_value::<CompiledContentPack>(json)
            .expect_err("compiled packs must use the exported packId field exactly")
            .to_string();

        assert!(error.contains("unknown field `pack_id`"), "{error}");
    }

    #[test]
    fn compile_options_json_requires_explicit_verification_and_playability_rules() {
        let missing_verify = serde_json::from_value::<ModpackCompileOptions>(serde_json::json!({
            "playability": PlayabilityRules::default()
        }))
        .expect_err("compile options must not default verification")
        .to_string();
        assert!(
            missing_verify.contains("missing field `verify`"),
            "{missing_verify}"
        );

        let missing_playability =
            serde_json::from_value::<ModpackCompileOptions>(serde_json::json!({
                "verify": true
            }))
            .expect_err("compile options must not default playability")
            .to_string();
        assert!(
            missing_playability.contains("missing field `playability`"),
            "{missing_playability}"
        );

        let unknown_fallback = serde_json::from_value::<ModpackCompileOptions>(serde_json::json!({
            "verify": true,
            "playability": PlayabilityRules::default(),
            "fallback_playability": true
        }))
        .expect_err("compile options must not accept fallback metadata")
        .to_string();
        assert!(
            unknown_fallback.contains("unknown field `fallback_playability`"),
            "{unknown_fallback}"
        );
    }

    #[test]
    fn content_pack_payloads_merge_playability_rules_as_modpack_data() {
        let mut data = GameDataSet::default();

        data.apply_content_pack_payload(
            ContentPackCategory::Playability,
            serde_json::json!({
                "start_maps": ["Start"],
                "start_tiles": [],
                "initial_events": [],
                "initial_items": [],
                "goal_maps": [],
                "goal_events": ["EVENT_DONE"],
                "goal_items": [],
                "progression_rules": [{
                    "id": "finish",
                    "requires": { "events": [], "items": [], "maps": ["Start"] },
                    "grants": { "events": ["EVENT_DONE"], "items": [], "maps": [] }
                }],
                "map_access": [],
                "require_all_maps_reachable": false,
                "require_walkable_maps": true
            }),
        )
        .expect("apply playability payload");

        assert_eq!(data.playability.start_maps, vec!["Start".to_string()]);
        assert_eq!(data.playability.goal_events, vec!["EVENT_DONE".to_string()]);
        assert_eq!(data.playability.progression_rules[0].id, "finish");
    }

    #[test]
    fn modpack_payloads_merge_playability_rules_as_modpack_data() {
        let manifest = ModpackManifest {
            payload: ModpackPayload {
                playability: PlayabilityRules {
                    start_maps: vec!["Start".to_string()],
                    goal_items: vec!["PASS".to_string()],
                    ..PlayabilityRules::default()
                },
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };
        let mut data = GameDataSet::default();

        data.apply_modpack(&manifest)
            .expect("apply playability manifest");

        assert_eq!(data.playability.start_maps, vec!["Start".to_string()]);
        assert_eq!(data.playability.goal_items, vec!["PASS".to_string()]);
    }

    #[test]
    fn compiled_game_pack_round_trips_as_runtime_artifact() {
        let path = temp_test_path("runtime.crystalpack");
        let mut data = GameDataSet::default();
        data.pokemon.insert("NEW_MON".to_string(), species());
        data.moves.insert("TACKLE".to_string(), test_move("TACKLE"));
        let report = ModpackCompileReport {
            manifests: vec!["base-game".to_string()],
            pokemon: 1,
            moves: 1,
            ..ModpackCompileReport::default()
        };
        let pack = CompiledGamePack::new(data, report);

        write_compiled_game_pack(&path, &pack).expect("write compiled pack");
        let loaded = read_compiled_game_pack(&path).expect("read compiled pack");
        let loaded_artifact =
            read_loaded_compiled_game_pack(&path).expect("read loaded compiled pack");

        assert_eq!(loaded, pack);
        assert_eq!(loaded_artifact.pack, pack);
        assert_eq!(
            loaded_artifact.bytes,
            std::fs::read(&path).expect("read raw pack")
        );
        assert!(loaded.data.pokemon.contains_key("NEW_MON"));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn verification_rejects_empty_runtime_game_sections() {
        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &GameDataSet::default(),
            &PlayabilityRules::default(),
        );

        let codes: BTreeSet<&str> = report
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code.as_str())
            .collect();

        assert!(codes.contains("missing_runtime_pokemon"));
        assert!(codes.contains("missing_runtime_moves"));
        assert!(codes.contains("missing_runtime_maps"));
    }

    #[test]
    fn runtime_compiled_game_pack_rejects_json_extension() {
        let path = temp_test_path("runtime.json");
        let pack = CompiledGamePack::new(GameDataSet::default(), ModpackCompileReport::default());

        let error = write_compiled_game_pack(&path, &pack)
            .expect_err("runtime compiled packs must not be JSON files")
            .to_string();

        assert!(error.contains("must use .crystalpack"));

        let extensionless = temp_test_path("runtime");
        let extensionless_error = write_compiled_game_pack(&extensionless, &pack)
            .expect_err("runtime compiled packs must declare an exact extension")
            .to_string();

        assert!(
            extensionless_error.contains("must have a file extension"),
            "{extensionless_error}"
        );
    }

    #[test]
    fn asset_root_compiled_game_pack_paths_reject_aliases_and_load_relative_pack() {
        let root = temp_test_path("compiled-pack-root");
        let _ = std::fs::remove_dir_all(&root);
        let data_root = root.join("apps/web/assets/data");
        std::fs::create_dir_all(data_root.join("content-packs")).expect("create data root");

        let pack = CompiledGamePack::new(GameDataSet::default(), ModpackCompileReport::default());
        write_compiled_game_pack(data_root.join("content-packs/core.crystalpack"), &pack)
            .expect("write compiled pack");
        let asset_root = AssetRoot::new(&root);

        let loaded = asset_root
            .load_loaded_compiled_game_pack("content-packs/core.crystalpack")
            .expect("load relative compiled pack");
        assert!(loaded.bytes.starts_with(COMPILED_GAME_PACK_MAGIC));

        let legacy = asset_root
            .load_compiled_game_pack("assets/data/content-packs/core.crystalpack")
            .expect_err("compiled pack paths must not accept assets/data aliases")
            .to_string();
        assert!(
            legacy.contains("must not include the assets/data prefix"),
            "{legacy}"
        );

        let traversal = asset_root
            .load_compiled_game_pack("content-packs/../core.crystalpack")
            .expect_err("compiled pack paths must not traverse")
            .to_string();
        assert!(
            traversal.contains("must not traverse parent directories"),
            "{traversal}"
        );

        let absolute = asset_root
            .load_compiled_game_pack(data_root.join("content-packs/core.crystalpack"))
            .expect_err("compiled pack paths must not be absolute")
            .to_string();
        assert!(
            absolute.contains("must be relative to assets/data"),
            "{absolute}"
        );

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn compiled_report_requires_complete_exported_shape() {
        let mut report = serde_json::to_value(ModpackCompileReport::default())
            .expect("serialize compile report");
        report
            .as_object_mut()
            .expect("report object")
            .remove("reachable_maps");

        let error = serde_json::from_value::<ModpackCompileReport>(report)
            .expect_err("compiled reports must not default missing fields")
            .to_string();

        assert!(error.contains("missing field `reachable_maps`"), "{error}");
    }

    #[test]
    fn compiled_report_rejects_unknown_fields() {
        let mut report = serde_json::to_value(ModpackCompileReport::default())
            .expect("serialize compile report");
        report
            .as_object_mut()
            .expect("report object")
            .insert("legacy_summary".to_string(), serde_json::json!({}));

        let error = serde_json::from_value::<ModpackCompileReport>(report)
            .expect_err("compiled reports must use the exported report schema exactly")
            .to_string();

        assert!(error.contains("unknown field `legacy_summary`"), "{error}");
    }

    #[test]
    fn music_modpack_assets_must_be_midi_files_not_json_or_asm() {
        let midi = ModpackAudioAsset::from_content_pack_path("mods/new/music/MUSIC_ROUTE_29.mid")
            .expect("valid MIDI music asset");
        assert_eq!(midi.id, "MUSIC_ROUTE_29");
        assert_eq!(midi.kind, ModpackAudioKind::Music);
        let sfx = ModpackAudioAsset::from_content_pack_path("mods/new/sfx/SFX_TACKLE.mid")
            .expect("valid MIDI sfx asset");
        assert_eq!(sfx.kind, ModpackAudioKind::SoundEffect);

        let json_error = ModpackAudioAsset::from_content_pack_path("mods/new/audio.json")
            .expect_err("ambiguous audio JSON is not accepted");
        assert!(
            json_error
                .to_string()
                .contains("must live under music, sfx, or cries")
        );

        let root_level_error = ModpackAudioAsset::from_content_pack_path("MUSIC_ROUTE_29.mid")
            .expect_err("root-level audio is not accepted");
        assert!(
            root_level_error
                .to_string()
                .contains("must live under music, sfx, or cries"),
            "{root_level_error}"
        );

        let lowercase_error =
            ModpackAudioAsset::from_content_pack_path("mods/new/music/route29.mid")
                .expect_err("lowercase path-derived music id is not accepted");
        assert!(lowercase_error.to_string().contains("must use an exact"));

        let padded_id_error =
            ModpackAudioAsset::music(" MUSIC_ROUTE_29", "mods/new/music/MUSIC_ROUTE_29.mid")
                .expect_err("padded audio ids are not accepted");
        assert!(
            padded_id_error.to_string().contains("must use an exact"),
            "{padded_id_error}"
        );

        let mismatched_path =
            ModpackAudioAsset::music("MUSIC_ROUTE_29", "mods/new/music/MUSIC_ROUTE_30.mid")
                .expect_err("explicit audio ids must match their file stems");
        assert!(
            mismatched_path
                .to_string()
                .contains("must match the exact audio id"),
            "{mismatched_path}"
        );

        let asm_error =
            ModpackAudioAsset::from_content_pack_path("mods/new/music/MUSIC_ROUTE_29.asm")
                .expect_err("music ASM is not accepted");
        assert!(asm_error.to_string().contains("must use a .mid file"));

        let extensionless_error =
            ModpackAudioAsset::from_content_pack_path("mods/new/music/MUSIC_ROUTE_29")
                .expect_err("extensionless music is not accepted");
        assert!(
            extensionless_error
                .to_string()
                .contains("path must have a file extension"),
            "{extensionless_error}"
        );

        let mp3_error =
            ModpackAudioAsset::from_content_pack_path("mods/new/music/MUSIC_ROUTE_29.mp3")
                .expect_err("MP3 music is not accepted");
        assert!(mp3_error.to_string().contains("must use a .mid file"));

        let midi_error =
            ModpackAudioAsset::from_content_pack_path("mods/new/music/MUSIC_ROUTE_29.midi")
                .expect_err(".midi music is not accepted");
        assert!(midi_error.to_string().contains("must use a .mid file"));

        let uppercase_mid_error =
            ModpackAudioAsset::from_content_pack_path("mods/new/music/MUSIC_ROUTE_29.MID")
                .expect_err("case-changed MIDI extensions are not accepted");
        assert!(
            uppercase_mid_error
                .to_string()
                .contains("must use a .mid file"),
            "{uppercase_mid_error}"
        );

        let cry = ModpackAudioAsset::cry("CRY_NIDORAN_M", "mods/new/cries/CRY_NIDORAN_M.mid")
            .expect("valid MIDI cry asset");
        assert_eq!(cry.kind, ModpackAudioKind::Cry);

        let singular_cry_dir =
            ModpackAudioAsset::from_content_pack_path("mods/new/cry/CRY_NIDORAN_M.mid")
                .expect_err("singular cry directory is not a modpack audio category")
                .to_string();
        assert!(
            singular_cry_dir.contains("must live under music, sfx, or cries"),
            "{singular_cry_dir}"
        );

        let cry_mp3_error =
            ModpackAudioAsset::cry("CRY_NIDORAN_M", "mods/new/cries/CRY_NIDORAN_M.mp3")
                .expect_err("MP3 cries are not accepted");
        assert!(cry_mp3_error.to_string().contains("must use a .mid file"));
    }

    #[test]
    fn definitive_runtime_payloads_require_explicit_pack_fields() {
        let missing_flee_bucket =
            serde_json::from_str::<FleeMonTables>(r#"{"always":[],"often":[]}"#)
                .expect_err("flee mon buckets must all be explicit")
                .to_string();
        assert!(
            missing_flee_bucket.contains("missing field `sometimes`"),
            "{missing_flee_bucket}"
        );

        let missing_initialize_bucket =
            serde_json::from_str::<InitializeEventsConfig>(r#"{"eventFlags":[],"engineFlags":[]}"#)
                .expect_err("initialize event buckets must all be explicit")
                .to_string();
        assert!(
            missing_initialize_bucket.contains("missing field `variableSprites`"),
            "{missing_initialize_bucket}"
        );

        let missing_story_maps =
            serde_json::from_str::<StoryEventScriptConstants>(r#"{"global":{}}"#)
                .expect_err("story event constants must declare map constants explicitly")
                .to_string();
        assert!(
            missing_story_maps.contains("missing field `maps`"),
            "{missing_story_maps}"
        );
    }

    #[test]
    fn definitive_runtime_payloads_reject_unknown_pack_fields() {
        for (label, result) in [
            (
                "flee mons",
                serde_json::from_str::<FleeMonTables>(
                    r#"{"always":[],"often":[],"sometimes":[],"fallback":[]}"#,
                )
                .map(|_| ()),
            ),
            (
                "initialize events",
                serde_json::from_str::<InitializeEventsConfig>(
                    r#"{"eventFlags":[],"engineFlags":[],"variableSprites":{},"legacy":true}"#,
                )
                .map(|_| ()),
            ),
            (
                "story event constants",
                serde_json::from_str::<StoryEventScriptConstants>(
                    r#"{"global":{},"maps":{},"legacy":{}}"#,
                )
                .map(|_| ()),
            ),
            (
                "pokemon cry metadata",
                serde_json::from_str::<PokemonCryMetadata>(
                    r#"{"cry":"CRY_NIDORAN_M","pitch":256,"length":64,"mp3":"nidoran.mp3"}"#,
                )
                .map(|_| ()),
            ),
            (
                "audio asset",
                serde_json::from_str::<ModpackAudioAsset>(
                    r#"{"id":"route29","path":"music/route29.mid","kind":"music","mp3":"route29.mp3"}"#,
                )
                .map(|_| ()),
            ),
            (
                "runtime pokedex entry",
                serde_json::from_str::<RuntimePokedexEntry>(
                    r#"{"species":"NIDORAN_M","classification":"POISON PIN","heightDigits":4,"weightDigits":70,"pages":[],"legacySpecies":"nidoran-m"}"#,
                )
                .map(|_| ()),
            ),
            (
                "runtime spawn point",
                serde_json::from_str::<RuntimeSpawnPoint>(
                    r#"{"identifier":1,"mapConstant":"ROUTE_29","mapName":"Route29","groupId":1,"mapId":1,"tileX":8,"tileY":8,"groupName":"GROUP","metatileX":4,"metatileY":4,"subtileX":0,"subtileY":0,"fallbackMap":"NewBarkTown"}"#,
                )
                .map(|_| ()),
            ),
            (
                "runtime map metadata",
                serde_json::from_str::<RuntimeMapMetadata>(
                    r#"{"constant":"ROUTE_29","name":"Route29","groupName":"GROUP","groupId":1,"mapId":1,"width":20,"height":18,"environment":"route","phoneService":1,"legacyWidth":10}"#,
                )
                .map(|_| ()),
            ),
        ] {
            let error = result.expect_err(label).to_string();
            assert!(error.contains("unknown field"), "{label}: {error}");
        }
    }

    #[test]
    fn frontpic_animation_json_requires_explicit_program_and_command_kind() {
        let missing_commands = serde_json::from_str::<FrontpicAnimProgram>(r#"{}"#)
            .expect_err("frontpic animation programs must declare command lists")
            .to_string();
        assert!(
            missing_commands.contains("missing field `commands`"),
            "{missing_commands}"
        );

        let missing_kind =
            serde_json::from_str::<FrontpicAnimProgram>(r#"{"commands":[{"frame":0}]}"#)
                .expect_err("frontpic animation commands must declare their opcode kind")
                .to_string();
        assert!(
            missing_kind.contains("missing field `kind`"),
            "{missing_kind}"
        );

        let explicit_command =
            serde_json::from_str::<FrontpicAnimProgram>(r#"{"commands":[{"kind":"endanim"}]}"#)
                .expect(
                    "optional command operands may be absent when the opcode does not use them",
                );
        assert_eq!(explicit_command.commands[0].kind, "endanim");

        let unknown_program_field = serde_json::from_str::<FrontpicAnimProgram>(
            r#"{"commands":[{"kind":"endanim"}],"fallback":[]}"#,
        )
        .expect_err("frontpic animation programs must not accept unknown fields")
        .to_string();
        assert!(
            unknown_program_field.contains("unknown field `fallback`"),
            "{unknown_program_field}"
        );

        let unknown_command_field = serde_json::from_str::<FrontpicAnimProgram>(
            r#"{"commands":[{"kind":"endanim","legacyOpcode":"end"}]}"#,
        )
        .expect_err("frontpic animation commands must not accept unknown fields")
        .to_string();
        assert!(
            unknown_command_field.contains("unknown field `legacyOpcode`"),
            "{unknown_command_field}"
        );
    }

    #[test]
    fn playability_json_requires_explicit_rule_fields() {
        let complete_rules = r#"{
          "start_maps":[],
          "start_tiles":[],
          "initial_events":[],
          "initial_items":[],
          "goal_maps":[],
          "goal_events":[],
          "goal_items":[],
          "progression_rules":[],
          "map_access":[],
          "require_all_maps_reachable":false,
          "require_walkable_maps":true
        }"#;
        serde_json::from_str::<PlayabilityRules>(complete_rules)
            .expect("complete playability payload should parse");

        let missing_goal_items = complete_rules.replace(r#"          "goal_items":[],"#, "");
        let missing_goal_items = serde_json::from_str::<PlayabilityRules>(&missing_goal_items)
            .expect_err("goal item rules must be explicit, even when empty")
            .to_string();
        assert!(
            missing_goal_items.contains("missing field `goal_items`"),
            "{missing_goal_items}"
        );

        let missing_requirement_buckets =
            serde_json::from_str::<ProgressionRequirements>(r#"{"maps":["Route29"]}"#)
                .expect_err("progression requirements must declare every bucket")
                .to_string();
        assert!(
            missing_requirement_buckets.contains("missing field `events`"),
            "{missing_requirement_buckets}"
        );

        let missing_grant_buckets =
            serde_json::from_str::<ProgressionGrants>(r#"{"events":["EVENT_DONE"]}"#)
                .expect_err("progression grants must declare every bucket")
                .to_string();
        assert!(
            missing_grant_buckets.contains("missing field `items`"),
            "{missing_grant_buckets}"
        );

        let missing_progression_grants = serde_json::from_str::<ProgressionRule>(
            r#"{"id":"script:Route29:Test","requires":{"events":[],"items":[],"maps":["Route29"]}}"#,
        )
        .expect_err("progression rules must declare grants explicitly")
        .to_string();
        assert!(
            missing_progression_grants.contains("missing field `grants`"),
            "{missing_progression_grants}"
        );

        let missing_map_access_requires =
            serde_json::from_str::<MapAccessRule>(r#"{"map":"Route29"}"#)
                .expect_err("map access rules must declare requirements explicitly")
                .to_string();
        assert!(
            missing_map_access_requires.contains("missing field `requires`"),
            "{missing_map_access_requires}"
        );

        let unknown_rule_field = complete_rules.replace(
            r#"          "require_walkable_maps":true"#,
            r#"          "require_walkable_maps":true,
          "fallback_maps":[]"#,
        );
        let unknown_rule_field = serde_json::from_str::<PlayabilityRules>(&unknown_rule_field)
            .expect_err("playability rules must not accept unknown fields")
            .to_string();
        assert!(
            unknown_rule_field.contains("unknown field `fallback_maps`"),
            "{unknown_rule_field}"
        );

        let unknown_start_field = serde_json::from_str::<PlayabilityStart>(
            r#"{"map":"Route29","tile":{"x":1,"y":2},"legacySpawn":"home"}"#,
        )
        .expect_err("playability starts must not accept unknown fields")
        .to_string();
        assert!(
            unknown_start_field.contains("unknown field `legacySpawn`"),
            "{unknown_start_field}"
        );

        let unknown_requirement_field = serde_json::from_str::<ProgressionRequirements>(
            r#"{"events":[],"items":[],"maps":[],"badges":[]}"#,
        )
        .expect_err("progression requirements must not accept unknown fields")
        .to_string();
        assert!(
            unknown_requirement_field.contains("unknown field `badges`"),
            "{unknown_requirement_field}"
        );
    }

    #[test]
    fn modpack_manifest_supports_typed_pokemon_and_map_additions() {
        let manifest = ModpackManifest {
            metadata: ModpackMetadata {
                id: "johto-plus".to_string(),
                name: "Johto Plus".to_string(),
                version: "0.1.0".to_string(),
                author: Some("Tester".to_string()),
                description: None,
            },
            payload: ModpackPayload {
                pokemon: vec![species()],
                maps: vec![MapModule {
                    id: "NEW_ROUTE".to_string(),
                    attributes: MapAttributes {
                        tileset_name: "johto".to_string(),
                        border_block: 1,
                        width: 10,
                        height: 9,
                        connections: vec![MapConnection {
                            direction: "north".to_string(),
                            target_map: "CHERRYGROVE_CITY".to_string(),
                            offset: 0,
                        }],
                        time_of_day: None,
                        phone_service: 0,
                        phone_flag: false,
                        environment: Some("route".to_string()),
                        location: Some("johto".to_string()),
                        music: Some("MUSIC_ROUTE_29".to_string()),
                        palette: None,
                        fishing_group: None,
                        map_constant: Some("NEW_ROUTE".to_string()),
                        map_group_constant: None,
                        blocks_label: None,
                        map_scripts_label: None,
                        map_events_label: None,
                        connection_flags: None,
                    },
                    scripts: BTreeMap::new(),
                    trainer_scripts: BTreeMap::new(),
                    scripted_trainer_battles: Vec::new(),
                    scripted_wild_battles: Vec::new(),
                    script_item_grants: Vec::new(),
                    script_item_checks: Vec::new(),
                    script_item_takes: Vec::new(),
                    script_economy_commands: Vec::new(),
                    gift_pokemon_scripts: Vec::new(),
                    script_flag_commands: Vec::new(),
                    script_scene_commands: Vec::new(),
                    script_audio_commands: Vec::new(),
                    script_block_changes: Vec::new(),
                    script_object_commands: Vec::new(),
                    script_movements: Vec::new(),
                    script_map_commands: Vec::new(),
                    script_text_commands: Vec::new(),
                    script_text_bodies: BTreeMap::new(),
                    script_menu_definitions: BTreeMap::new(),
                    script_variable_commands: Vec::new(),
                    script_control_commands: Vec::new(),
                    script_field_pickups: Vec::new(),
                    script_shop_commands: Vec::new(),
                    script_phone_commands: Vec::new(),
                    script_runtime_commands: Vec::new(),
                    map_script_section_commands: Vec::new(),
                    map_event_section_commands: Vec::new(),
                    scenes: MapSceneTable::default(),
                    events: MapEvents::default(),
                    objects: Vec::new(),
                    blocks: vec![0; 90],
                }],
                items: vec![Item {
                    name: "Spark Charm".to_string(),
                    description: "A charged charm.".to_string(),
                    effect: "NONE".to_string(),
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
                    escape_rope_mode: None,
                    price: 100,
                    held_effect: "HELD_NONE".to_string(),
                    parameter: 0,
                    property: String::new(),
                    pocket: item_pocket("ITEM"),
                    field_menu: "ITEMMENU_NOUSE".to_string(),
                    field_usable: false,
                    battle_menu: "ITEMMENU_NOUSE".to_string(),
                    battle_usable: false,
                    script_name: "SPARK_CHARM".to_string(),
                    consumable: false,
                    tmhm_index: None,
                    tmhm_move: None,
                }],
                moves: vec![Move {
                    name: "SPARK".to_string(),
                    move_type: pokemon_type("ELECTRIC"),
                    power: 40,
                    accuracy: 100,
                    pp: 30,
                    effect: "NORMAL_HIT".to_string(),
                    effect_chance: 0,
                    stat: None,
                    amount: None,
                }],
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };

        let json = serde_json::to_string(&manifest).expect("serialize modpack");
        let parsed: ModpackManifest = serde_json::from_str(&json).expect("parse modpack");
        assert_eq!(parsed.id(), "johto-plus");
        assert_eq!(parsed.payload.pokemon[0].id, "NEW_MON");
        assert_eq!(parsed.payload.moves[0].name, "SPARK");
        assert_eq!(parsed.payload.maps[0].blocks.len(), 90);
    }

    #[test]
    fn modpack_overlay_merges_capture_rules_as_definitive_pack_data() {
        let mut data = GameDataSet::default();
        let manifest = ModpackManifest {
            payload: ModpackPayload {
                capture_rules: CaptureRules {
                    fast_ball_species: ["MAGNEMITE".to_string()].into_iter().collect(),
                    heavy_ball_modifiers: [("KADABRA".to_string(), 40)].into_iter().collect(),
                    ball_rules: BTreeMap::new(),
                    guaranteed_capture_balls: BTreeSet::new(),
                    status_bonus: BTreeMap::new(),
                },
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };

        data.apply_modpack(&manifest).expect("apply capture rules");

        assert!(data.capture_rules.fast_ball_species.contains("MAGNEMITE"));
        assert_eq!(
            data.capture_rules.heavy_ball_modifiers.get("KADABRA"),
            Some(&40)
        );
    }

    #[test]
    fn verifier_rejects_runtime_pack_species_case_and_malformed_frontpic_commands() {
        let data = GameDataSet {
            pokemon: [(species().id.clone(), species())].into_iter().collect(),
            runtime_spawn_points: [(
                "1".to_string(),
                RuntimeSpawnPoint {
                    identifier: 0,
                    map_constant: "MISSING_MAP".to_string(),
                    map_name: "MissingMap".to_string(),
                    group_id: 1,
                    map_id: 1,
                    tile_x: 0,
                    tile_y: 0,
                    group_name: String::new(),
                    metatile_x: 0,
                    metatile_y: 0,
                    subtile_x: 0,
                    subtile_y: 0,
                },
            )]
            .into_iter()
            .collect(),
            flee_mons: FleeMonTables {
                always: vec!["new_mon".to_string()],
                ..FleeMonTables::default()
            },
            menu_icons: [("New_Mon".to_string(), "ICON_NEW_MON".to_string())]
                .into_iter()
                .collect(),
            pokedex_entries: [(
                "NEW_MON".to_string(),
                RuntimePokedexEntry {
                    species: "new_mon".to_string(),
                    classification: String::new(),
                    height_digits: 1,
                    weight_digits: 1,
                    pages: Vec::new(),
                },
            )]
            .into_iter()
            .collect(),
            pokemon_frontpic_anim: [(
                "NEW_MON".to_string(),
                FrontpicAnimProgram {
                    commands: vec![FrontpicAnimCommand {
                        kind: "frame".to_string(),
                        frame: Some(0),
                        duration: None,
                        ..FrontpicAnimCommand::default()
                    }],
                },
            )]
            .into_iter()
            .collect(),
            initialize_events: InitializeEventsConfig {
                event_flags: vec![String::new()],
                ..InitializeEventsConfig::default()
            },
            story_event_script_constants: StoryEventScriptConstants {
                global: [(String::new(), 1)].into_iter().collect(),
                ..StoryEventScriptConstants::default()
            },
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );
        let codes: BTreeSet<&str> = report
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code.as_str())
            .collect();

        assert!(codes.contains("runtime_spawn_point_identifier_mismatch"));
        assert!(codes.contains("unknown_runtime_spawn_point_map"));
        assert!(codes.contains("invalid_runtime_spawn_point"));
        assert!(codes.contains("invalid_initialize_event_flag"));
        assert!(codes.contains("invalid_story_event_script_constant"));
        assert!(codes.contains("unknown_flee_mon_species"));
        assert!(codes.contains("unknown_menu_icon_species"));
        assert!(codes.contains("pokedex_entry_species_mismatch"));
        assert!(codes.contains("invalid_pokedex_entry"));
        assert!(codes.contains("malformed_frontpic_anim_command"));
    }

    #[test]
    fn verifier_requires_species_display_records_from_pack() {
        let species_id = species().id;
        let data = GameDataSet {
            pokemon: [(species_id.clone(), species())].into_iter().collect(),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        for expected in [
            "missing_species_menu_icon",
            "missing_species_pokedex_entry",
            "missing_species_frontpic_anim",
        ] {
            assert!(
                report.diagnostics.iter().any(|diagnostic| {
                    diagnostic.code == expected && diagnostic.subject == species_id
                }),
                "missing diagnostic {expected}: {:?}",
                report.diagnostics
            );
        }
    }

    #[test]
    fn verifier_rejects_pokegear_landmarks_for_unknown_maps_or_constants() {
        let data = GameDataSet {
            maps: [(
                "Route29".to_string(),
                test_map_module("Route29", "ROUTE_29", None),
            )]
            .into_iter()
            .collect(),
            pokegear_landmarks: PokegearLandmarksPayload {
                landmarks: vec![
                    PokegearLandmark {
                        id: 1,
                        constant: "LANDMARK_ROUTE_29".to_string(),
                        label: "ROUTE_29".to_string(),
                        name: "Route 29".to_string(),
                        x: 2,
                        y: 3,
                        region: "johto".to_string(),
                    },
                    PokegearLandmark {
                        id: 2,
                        constant: "route_30".to_string(),
                        label: String::new(),
                        name: "Route 30".to_string(),
                        x: 4,
                        y: 5,
                        region: "johto".to_string(),
                    },
                ],
                map_to_landmark: [
                    ("Route29".to_string(), "LANDMARK_ROUTE_30".to_string()),
                    ("MissingRoute".to_string(), "LANDMARK_ROUTE_29".to_string()),
                ]
                .into_iter()
                .collect(),
            },
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_pokegear_landmark" && diagnostic.subject == "route_30"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_pokegear_landmark_constant"
                && diagnostic.subject == "route_30"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_pokegear_landmark_constant"
                && diagnostic.subject == "Route29"
                && diagnostic.message.contains("LANDMARK_ROUTE_30")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_pokegear_landmark_map"
                && diagnostic.subject == "MissingRoute"
        }));
    }

    #[test]
    fn verifier_accepts_exact_frontpic_animation_asset_keys_only() {
        let valid_program = FrontpicAnimProgram {
            commands: vec![FrontpicAnimCommand {
                kind: "endanim".to_string(),
                ..FrontpicAnimCommand::default()
            }],
        };
        let data = GameDataSet {
            pokemon: [(species().id.clone(), species())].into_iter().collect(),
            pokemon_frontpic_anim: [
                ("EGG".to_string(), valid_program.clone()),
                ("UNOWN_A".to_string(), valid_program.clone()),
                ("unown_a".to_string(), valid_program.clone()),
                ("UNOWN_AA".to_string(), valid_program.clone()),
                ("UNOWN_1".to_string(), valid_program),
            ]
            .into_iter()
            .collect(),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );
        let invalid_frontpic_subjects: BTreeSet<&str> = report
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == "unknown_frontpic_anim_species")
            .map(|diagnostic| diagnostic.subject.as_str())
            .collect();

        assert_eq!(
            invalid_frontpic_subjects,
            BTreeSet::from(["UNOWN_1", "UNOWN_AA", "unown_a"])
        );
    }

    #[test]
    fn entity_content_payloads_reject_object_map_fallback_shape() {
        let mut single = species();
        single.id = "SINGLE_MON".to_string();
        let single_payload = serde_json::to_value(&single).expect("serialize single species");
        let mut data = GameDataSet::default();
        data.apply_content_pack_payload(ContentPackCategory::Pokemon, single_payload)
            .expect("single exported Pokemon file is canonical");
        assert!(data.pokemon.contains_key("SINGLE_MON"));

        let mut array_entry = species();
        array_entry.id = "ARRAY_MON".to_string();
        let mut data = GameDataSet::default();
        data.apply_content_pack_payload(
            ContentPackCategory::Pokemon,
            serde_json::json!([array_entry]),
        )
        .expect("compiled Pokemon category entries are arrays");
        assert!(data.pokemon.contains_key("ARRAY_MON"));

        let mut mapped = species();
        mapped.id = "MAPPED_MON".to_string();
        let mut data = GameDataSet::default();
        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::Pokemon,
                serde_json::json!({ "MAPPED_MON": mapped }),
            )
            .expect_err("Pokemon category files must not use object-map compatibility shape")
            .to_string();

        assert!(error.contains("parse single payload entry"), "{error}");
    }

    #[test]
    fn content_pack_payloads_merge_evolutions_as_typed_definitive_data() {
        let mut data = GameDataSet::default();

        data.apply_content_pack_payload(
            ContentPackCategory::Evolutions,
            serde_json::json!([
                {
                    "species": "PIKACHU",
                    "evolutions": [{
                        "method": "ITEM",
                        "species": "RAICHU",
                        "level": null,
                        "item": "THUNDERSTONE",
                        "held_item": null,
                        "happiness": null,
                        "stat_ratio": null
                    }]
                },
                {
                    "species": "EEVEE",
                    "evolutions": [{
                        "method": "HAPPINESS",
                        "species": "ESPEON",
                        "level": null,
                        "item": null,
                        "held_item": null,
                        "happiness": "TR_MORNDAY",
                        "stat_ratio": null
                    }]
                }
            ]),
        )
        .expect("apply evolution payload");

        assert_eq!(
            data.evolutions
                .entries_for("PIKACHU")
                .expect("PIKACHU evolutions")[0]
                .item
                .as_deref(),
            Some("THUNDERSTONE")
        );
        assert_eq!(
            data.evolutions
                .entries_for("EEVEE")
                .expect("EEVEE evolutions")[0]
                .species,
            "ESPEON"
        );
    }

    #[test]
    fn learnset_and_evolution_payloads_reject_object_map_fallback_shape() {
        let mut data = GameDataSet::default();
        data.apply_content_pack_payload(
            ContentPackCategory::Learnsets,
            serde_json::json!({
                "species": "NEW_MON",
                "learnset": []
            }),
        )
        .expect("single exported learnset entry is canonical");
        assert_eq!(data.learnsets["NEW_MON"], Vec::<LearnsetEntry>::new());

        let mut data = GameDataSet::default();
        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::Learnsets,
                serde_json::json!({
                    "NEW_MON": []
                }),
            )
            .expect_err("learnsets must not use object-map compatibility shape")
            .to_string();
        assert!(
            error.contains("must use an explicit species/learnset entry"),
            "{error}"
        );

        let mut data = GameDataSet::default();
        data.apply_content_pack_payload(
            ContentPackCategory::Evolutions,
            serde_json::json!({
                "species": "NEW_MON",
                "evolutions": []
            }),
        )
        .expect("single exported evolution entry is canonical");
        assert!(
            data.evolutions
                .entries_for("NEW_MON")
                .expect("NEW_MON evolutions")
                .is_empty()
        );

        let mut data = GameDataSet::default();
        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::Evolutions,
                serde_json::json!({
                    "NEW_MON": []
                }),
            )
            .expect_err("evolutions must not use object-map compatibility shape")
            .to_string();
        assert!(
            error.contains("must use an explicit species/evolutions entry"),
            "{error}"
        );
    }

    #[test]
    fn species_move_payloads_require_explicit_species_and_moves_fields() {
        let mut data = GameDataSet::default();
        data.apply_content_pack_payload(
            ContentPackCategory::LevelUpMoves,
            serde_json::json!({
                "species": "NEW_MON",
                "moves": ["TACKLE"]
            }),
        )
        .expect("single exported level-up move entry is canonical");
        assert_eq!(
            data.level_up_moves["NEW_MON"],
            serde_json::json!(["TACKLE"])
        );

        let mut data = GameDataSet::default();
        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::EggMoves,
                serde_json::json!({
                    "NEW_MON": ["CHARM"]
                }),
            )
            .expect_err("egg moves must not use object-map compatibility shape")
            .to_string();
        assert!(
            error.contains("must use an explicit species/moves entry"),
            "{error}"
        );

        let mut data = GameDataSet::default();
        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::LevelUpMoves,
                serde_json::json!({
                    "species": "NEW_MON"
                }),
            )
            .expect_err("species move payloads must declare moves explicitly")
            .to_string();
        assert!(error.contains("must declare moves"), "{error}");
    }

    #[test]
    fn map_like_payloads_reject_non_object_noop_shape() {
        let mut data = GameDataSet::default();
        data.apply_content_pack_payload(
            ContentPackCategory::MapDimensions,
            serde_json::json!({
                "Route29": {
                    "width": 10,
                    "height": 9
                }
            }),
        )
        .expect("single exported object-map payload is canonical");
        assert!(data.map_dimensions.contains_key("Route29"));

        let mut data = GameDataSet::default();
        data.apply_content_pack_payload(
            ContentPackCategory::MapDimensions,
            serde_json::json!([
                {
                    "Route30": {
                        "width": 12,
                        "height": 10
                    }
                }
            ]),
        )
        .expect("compiled object-map payload arrays are canonical");
        assert!(data.map_dimensions.contains_key("Route30"));

        let mut data = GameDataSet::default();
        let error = data
            .apply_content_pack_payload(ContentPackCategory::MapDimensions, serde_json::json!(null))
            .expect_err("map-like payloads must not ignore malformed scalar payloads")
            .to_string();
        assert!(
            error.contains("object payload must be an object or an array of objects"),
            "{error}"
        );
    }

    #[test]
    fn modpack_overlay_replaces_evolutions_by_exact_species_id() {
        let mut data = GameDataSet::default();
        data.evolutions.0.insert(
            "NEW_MON".to_string(),
            vec![EvolutionEntry::level("OLD_FORM", 20)],
        );
        let manifest = ModpackManifest {
            payload: ModpackPayload {
                evolutions: EvolutionTable(
                    [(
                        "NEW_MON".to_string(),
                        vec![EvolutionEntry::level("NEW_FORM", 30)],
                    )]
                    .into_iter()
                    .collect(),
                ),
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };

        data.apply_modpack(&manifest)
            .expect("apply evolution manifest");

        assert_eq!(
            data.evolutions
                .entries_for("NEW_MON")
                .expect("NEW_MON evolutions"),
            &[EvolutionEntry::level("NEW_FORM", 30)]
        );
    }

    #[test]
    fn content_pack_payloads_merge_marts_as_exact_pack_data() {
        let mut data = GameDataSet::default();

        data.apply_content_pack_payload(
            ContentPackCategory::Marts,
            serde_json::json!({
                "MartCherrygroveDex": ["POKE_BALL", "POTION"]
            }),
        )
        .expect("apply mart payload");

        assert_eq!(
            data.marts
                .inventory_ids("MartCherrygroveDex")
                .expect("mart"),
            &["POKE_BALL".to_string(), "POTION".to_string()]
        );
        assert!(data.marts.inventory_ids("MART_CHERRYGROVE_DEX").is_err());
    }

    #[test]
    fn content_pack_payloads_merge_currency_constants_as_exact_pack_data() {
        let mut data = GameDataSet::default();

        data.apply_content_pack_payload(
            ContentPackCategory::CurrencyConstants,
            serde_json::json!({
                "ROUTE43GATE_TOLL": 1000,
                "GOLDENRODGAMECORNER_TM25_COINS": 5500
            }),
        )
        .expect("apply currency constants");

        assert_eq!(data.currency_constants.get("ROUTE43GATE_TOLL"), Some(1000));
        assert_eq!(data.currency_constants.get("route43gate_toll"), None);
    }

    #[test]
    fn content_pack_payloads_merge_roaming_pokemon_as_exact_pack_data() {
        let mut data = GameDataSet::default();

        data.apply_content_pack_payload(
            ContentPackCategory::RoamingPokemon,
            serde_json::json!([
                { "species": "RAIKOU", "level": 40, "mapGroup": 2, "mapNumber": 5 }
            ]),
        )
        .expect("apply roaming Pokemon payload");

        assert_eq!(
            data.roaming_pokemon,
            vec![RoamingPokemonDefinition {
                species: "RAIKOU".to_string(),
                level: 40,
                map_group: 2,
                map_number: 5,
            }]
        );
    }

    #[test]
    fn content_pack_payloads_merge_buena_prizes_as_exact_pack_data() {
        let mut data = GameDataSet::default();

        data.apply_content_pack_payload(
            ContentPackCategory::BuenaPrizes,
            serde_json::json!([
                { "itemId": "RARE_CANDY", "cost": 3 }
            ]),
        )
        .expect("apply Buena prizes payload");

        assert_eq!(
            data.buena_prizes,
            vec![BuenaPrizeDefinition {
                item_id: "RARE_CANDY".to_string(),
                cost: 3,
            }]
        );
    }

    #[test]
    fn content_pack_payloads_merge_buena_password_categories_as_exact_pack_data() {
        let mut data = GameDataSet::default();

        data.apply_content_pack_payload(
            ContentPackCategory::BuenaPasswordCategories,
            serde_json::json!([
                {
                    "id": "HealingItems",
                    "categoryType": "BUENA_ITEM",
                    "points": 12,
                    "options": ["POTION", "ANTIDOTE", "PARLYZ_HEAL"]
                }
            ]),
        )
        .expect("apply Buena password category payload");

        assert_eq!(
            data.buena_password_categories,
            vec![BuenaPasswordCategoryDefinition {
                id: "HealingItems".to_string(),
                category_type: "BUENA_ITEM".to_string(),
                points: 12,
                options: vec![
                    "POTION".to_string(),
                    "ANTIDOTE".to_string(),
                    "PARLYZ_HEAL".to_string()
                ],
            }]
        );
    }

    #[test]
    fn content_pack_payloads_merge_kurt_apricorn_recipes_as_exact_pack_data() {
        let mut data = GameDataSet::default();

        data.apply_content_pack_payload(
            ContentPackCategory::KurtApricornRecipes,
            serde_json::json!([
                { "apricorn": "RED_APRICORN", "ball": "LEVEL_BALL" }
            ]),
        )
        .expect("apply Kurt apricorn recipe payload");

        assert_eq!(
            data.kurt_apricorn_recipes,
            vec![KurtApricornRecipe {
                apricorn: "RED_APRICORN".to_string(),
                ball: "LEVEL_BALL".to_string(),
            }]
        );
    }

    #[test]
    fn content_pack_payloads_merge_shuckie_gift_as_exact_pack_data() {
        let mut data = GameDataSet::default();

        data.apply_content_pack_payload(
            ContentPackCategory::ShuckieGift,
            serde_json::json!({
                "species": "SHUCKLE",
                "level": 15,
                "heldItem": "BERRY",
                "nickname": "SHUCKIE",
                "originalTrainerName": "MANIA",
                "originalTrainerId": 518,
                "gotTodayEngineFlag": "ENGINE_GOT_SHUCKIE_TODAY"
            }),
        )
        .expect("apply Shuckie gift payload");

        assert_eq!(
            data.shuckie_gift,
            Some(ShuckieGiftDefinition {
                species: "SHUCKLE".to_string(),
                level: 15,
                held_item: "BERRY".to_string(),
                nickname: "SHUCKIE".to_string(),
                original_trainer_name: "MANIA".to_string(),
                original_trainer_id: 518,
                got_today_engine_flag: "ENGINE_GOT_SHUCKIE_TODAY".to_string(),
            })
        );
    }

    #[test]
    fn content_pack_payloads_merge_dratini_move_sets_as_exact_pack_data() {
        let mut data = GameDataSet::default();

        data.apply_content_pack_payload(
            ContentPackCategory::DratiniMoveSets,
            serde_json::json!([
                { "mode": 0, "moves": ["WRAP", "THUNDER_WAVE", "TWISTER", "EXTREMESPEED"] },
                { "mode": 1, "moves": ["WRAP", "LEER", "THUNDER_WAVE", "TWISTER"] }
            ]),
        )
        .expect("apply Dratini move sets payload");

        assert_eq!(
            data.dratini_move_sets,
            vec![
                DratiniMoveSetDefinition {
                    mode: 0,
                    moves: vec![
                        "WRAP".to_string(),
                        "THUNDER_WAVE".to_string(),
                        "TWISTER".to_string(),
                        "EXTREMESPEED".to_string()
                    ],
                },
                DratiniMoveSetDefinition {
                    mode: 1,
                    moves: vec![
                        "WRAP".to_string(),
                        "LEER".to_string(),
                        "THUNDER_WAVE".to_string(),
                        "TWISTER".to_string()
                    ],
                },
            ]
        );
    }

    #[test]
    fn content_pack_payloads_merge_bug_contest_config_as_exact_pack_data() {
        let mut data = GameDataSet::default();

        data.apply_content_pack_payload(
            ContentPackCategory::BugContestConfig,
            serde_json::json!({
                "parkBalls": 20,
                "timerMinutes": 20,
                "timerSeconds": 0,
                "selectedContestantCount": 5,
                "contestantFlags": [
                    "EVENT_BUG_CATCHING_CONTESTANT_1A",
                    "EVENT_BUG_CATCHING_CONTESTANT_2A"
                ]
            }),
        )
        .expect("apply Bug-Catching Contest config payload");

        assert_eq!(
            data.bug_contest_config,
            Some(BugContestConfig {
                park_balls: 20,
                timer_minutes: 20,
                timer_seconds: 0,
                selected_contestant_count: 5,
                contestant_flags: vec![
                    "EVENT_BUG_CATCHING_CONTESTANT_1A".to_string(),
                    "EVENT_BUG_CATCHING_CONTESTANT_2A".to_string()
                ],
            })
        );
    }

    #[test]
    fn content_pack_payloads_merge_battle_tower_rules_as_exact_pack_data() {
        let mut data = GameDataSet::default();

        data.apply_content_pack_payload(
            ContentPackCategory::BattleTowerRules,
            serde_json::json!({
                "bannedSpecies": ["MEWTWO", "MEW", "LUGIA", "HO_OH", "CELEBI"],
                "requiredPartyCount": 3,
                "challengeStreakLength": 7,
                "minimumLevelGroup": 1,
                "maximumLevelGroup": 10,
                "levelGroupSize": 10,
                "partyCountFailureText": "OnlyThreeMonMayBeEnteredText",
                "duplicateSpeciesFailureText": "TheMonMustAllBeDifferentKindsText",
                "duplicateHeldItemFailureText": "TheMonMustNotHoldTheSameItemsText",
                "eggFailureText": "YouCantTakeAnEggText"
            }),
        )
        .expect("apply Battle Tower rules payload");

        assert_eq!(
            data.battle_tower_rules,
            Some(BattleTowerRules {
                banned_species: vec![
                    "MEWTWO".to_string(),
                    "MEW".to_string(),
                    "LUGIA".to_string(),
                    "HO_OH".to_string(),
                    "CELEBI".to_string(),
                ],
                required_party_count: 3,
                challenge_streak_length: 7,
                minimum_level_group: 1,
                maximum_level_group: 10,
                level_group_size: 10,
                party_count_failure_text: "OnlyThreeMonMayBeEnteredText".to_string(),
                duplicate_species_failure_text: "TheMonMustAllBeDifferentKindsText".to_string(),
                duplicate_held_item_failure_text: "TheMonMustNotHoldTheSameItemsText".to_string(),
                egg_failure_text: "YouCantTakeAnEggText".to_string(),
            })
        );
    }

    #[test]
    fn content_pack_payloads_merge_oak_ratings_as_exact_pack_data() {
        let mut data = GameDataSet::default();

        data.apply_content_pack_payload(
            ContentPackCategory::OakRatings,
            serde_json::json!([
                {
                    "caughtCountLimit": 9,
                    "fanfare": "SFX_DEX_FANFARE_LESS_THAN_20",
                    "textLabel": "OakRating01"
                }
            ]),
        )
        .expect("apply Oak ratings payload");

        assert_eq!(
            data.oak_ratings,
            vec![OakRatingEntry {
                caught_count_limit: 9,
                fanfare: "SFX_DEX_FANFARE_LESS_THAN_20".to_string(),
                text_label: "OakRating01".to_string(),
            }]
        );
    }

    #[test]
    fn content_pack_payloads_merge_odd_egg_definitions_as_exact_pack_data() {
        let mut data = GameDataSet::default();

        data.apply_content_pack_payload(
            ContentPackCategory::OddEggDefinitions,
            serde_json::json!([
                {
                    "species": "CLEFFA",
                    "moves": ["POUND", "CHARM", "DIZZY_PUNCH"],
                    "originalTrainerId": 768,
                    "dvs": [2, 10, 10, 10],
                    "probability": 100,
                    "level": 5,
                    "experience": 125,
                    "hatchCycles": 20,
                    "nickname": "EGG",
                    "originalTrainerName": "ODD"
                }
            ]),
        )
        .expect("apply Odd Egg definitions payload");

        assert_eq!(
            data.odd_egg_definitions,
            vec![OddEggDefinition {
                species: "CLEFFA".to_string(),
                moves: vec![
                    "POUND".to_string(),
                    "CHARM".to_string(),
                    "DIZZY_PUNCH".to_string()
                ],
                original_trainer_id: 768,
                dvs: [2, 10, 10, 10],
                probability: 100,
                level: 5,
                experience: 125,
                hatch_cycles: 20,
                nickname: "EGG".to_string(),
                original_trainer_name: "ODD".to_string(),
            }]
        );
    }

    #[test]
    fn content_pack_payloads_merge_magikarp_lengths_as_exact_pack_data() {
        let mut data = GameDataSet::default();

        data.apply_content_pack_payload(
            ContentPackCategory::MagikarpLengths,
            serde_json::json!([
                { "threshold": 110, "divisor": 1 },
                { "threshold": 310, "divisor": 2 }
            ]),
        )
        .expect("apply Magikarp length table payload");

        assert_eq!(
            data.magikarp_lengths,
            vec![
                MagikarpLengthEntry {
                    threshold: 110,
                    divisor: 1,
                },
                MagikarpLengthEntry {
                    threshold: 310,
                    divisor: 2,
                },
            ]
        );
    }

    #[test]
    fn content_pack_payloads_merge_happiness_data_as_exact_pack_data() {
        let mut data = GameDataSet::default();

        data.apply_content_pack_payload(
            ContentPackCategory::HappinessData,
            serde_json::json!({
                "changes": [
                    { "code": "HAPPINESS_GROOMING", "changeCode": 18, "low": 3, "mid": 3, "high": 1 }
                ],
                "services": [
                    {
                        "routine": "DaisysGrooming",
                        "outcomes": [
                            { "rollWeight": 255, "scriptValue": 2, "changeCode": 18 }
                        ]
                    }
                ]
            }),
        )
        .expect("apply happiness data payload");

        assert_eq!(
            data.happiness_data,
            Some(HappinessData {
                changes: vec![
                    crystal_core::systems::special_routines::HappinessChangeEntry {
                        code: "HAPPINESS_GROOMING".to_string(),
                        change_code: 18,
                        low: 3,
                        mid: 3,
                        high: 1,
                    }
                ],
                services: vec![
                    crystal_core::systems::special_routines::HappinessServiceTable {
                        routine: "DaisysGrooming".to_string(),
                        outcomes: vec![
                            crystal_core::systems::special_routines::HappinessServiceOutcome {
                                roll_weight: 255,
                                script_value: 2,
                                change_code: 18,
                            },
                        ],
                    }
                ],
            })
        );
    }

    #[test]
    fn content_pack_payloads_merge_encounter_slot_tables_as_exact_pack_data() {
        let mut data = GameDataSet::default();

        data.apply_content_pack_payload(
            ContentPackCategory::EncounterSlotTables,
            serde_json::json!({
                "grass": [
                    { "threshold": 30, "slot": 0 },
                    { "threshold": 100, "slot": 1 }
                ],
                "water": [
                    { "threshold": 100, "slot": 0 }
                ]
            }),
        )
        .expect("apply encounter slot tables payload");

        assert_eq!(
            data.encounter_slot_tables,
            EncounterSlotTables {
                grass: vec![
                    crystal_core::world::encounters::EncounterSlotChance {
                        threshold: 30,
                        slot: 0,
                    },
                    crystal_core::world::encounters::EncounterSlotChance {
                        threshold: 100,
                        slot: 1,
                    },
                ],
                water: vec![crystal_core::world::encounters::EncounterSlotChance {
                    threshold: 100,
                    slot: 0,
                }],
            }
        );
    }

    #[test]
    fn content_pack_payloads_merge_encounter_music_modifiers_as_exact_pack_data() {
        let mut data = GameDataSet::default();

        data.apply_content_pack_payload(
            ContentPackCategory::EncounterMusicModifiers,
            serde_json::json!({
                "modifiers": [
                    { "music_id": "MUSIC_POKEMON_MARCH", "numerator": 2, "denominator": 1 },
                    { "music_id": "MUSIC_POKEMON_LULLABY", "numerator": 1, "denominator": 2 }
                ]
            }),
        )
        .expect("apply encounter music modifiers payload");

        assert_eq!(
            data.encounter_music_modifiers,
            EncounterMusicModifiers {
                modifiers: vec![
                    EncounterMusicModifier {
                        music_id: "MUSIC_POKEMON_MARCH".to_string(),
                        numerator: 2,
                        denominator: 1,
                    },
                    EncounterMusicModifier {
                        music_id: "MUSIC_POKEMON_LULLABY".to_string(),
                        numerator: 1,
                        denominator: 2,
                    },
                ],
            }
        );
    }

    #[test]
    fn content_pack_payloads_merge_battle_stat_multipliers_as_exact_pack_data() {
        let mut data = GameDataSet::default();

        data.apply_content_pack_payload(
            ContentPackCategory::BattleStatMultipliers,
            serde_json::json!({
                "stat": [
                    { "numerator": 25, "denominator": 100 },
                    { "numerator": 28, "denominator": 100 },
                    { "numerator": 33, "denominator": 100 },
                    { "numerator": 40, "denominator": 100 },
                    { "numerator": 50, "denominator": 100 },
                    { "numerator": 66, "denominator": 100 },
                    { "numerator": 1, "denominator": 1 },
                    { "numerator": 15, "denominator": 10 },
                    { "numerator": 2, "denominator": 1 },
                    { "numerator": 25, "denominator": 10 },
                    { "numerator": 3, "denominator": 1 },
                    { "numerator": 35, "denominator": 10 },
                    { "numerator": 4, "denominator": 1 }
                ],
                "accuracy": [
                    { "numerator": 33, "denominator": 100 },
                    { "numerator": 36, "denominator": 100 },
                    { "numerator": 43, "denominator": 100 },
                    { "numerator": 50, "denominator": 100 },
                    { "numerator": 60, "denominator": 100 },
                    { "numerator": 75, "denominator": 100 },
                    { "numerator": 1, "denominator": 1 },
                    { "numerator": 133, "denominator": 100 },
                    { "numerator": 166, "denominator": 100 },
                    { "numerator": 2, "denominator": 1 },
                    { "numerator": 233, "denominator": 100 },
                    { "numerator": 133, "denominator": 50 },
                    { "numerator": 3, "denominator": 1 }
                ]
            }),
        )
        .expect("apply battle stat multipliers payload");

        assert_eq!(data.battle_stat_multipliers.stat.len(), 13);
        assert_eq!(data.battle_stat_multipliers.accuracy.len(), 13);
        assert_eq!(data.battle_stat_multipliers.stat[0].numerator, 25);
        assert_eq!(data.battle_stat_multipliers.accuracy[8].numerator, 166);
        assert_eq!(data.battle_stat_multipliers.accuracy[11].denominator, 50);
    }

    #[test]
    fn content_pack_payloads_merge_capture_wobble_probabilities_as_exact_pack_data() {
        let mut data = GameDataSet::default();

        data.apply_content_pack_payload(
            ContentPackCategory::CaptureWobbleProbabilities,
            serde_json::json!([
                { "catch_rate": 1, "chance": 63 },
                { "catch_rate": 255, "chance": 255 }
            ]),
        )
        .expect("apply capture wobble probabilities payload");

        assert_eq!(
            data.capture_wobble_probabilities,
            vec![
                CaptureWobbleProbability {
                    catch_rate: 1,
                    chance: 63,
                },
                CaptureWobbleProbability {
                    catch_rate: 255,
                    chance: 255,
                },
            ]
        );
    }

    #[test]
    fn content_pack_payloads_merge_type_effectiveness_as_exact_pack_data() {
        let mut data = GameDataSet::default();

        data.apply_content_pack_payload(
            ContentPackCategory::TypeEffectiveness,
            serde_json::json!({
                "matchups": [
                    {
                        "attacker": "FIRE",
                        "defender": "GRASS",
                        "multiplier": { "numerator": 2, "denominator": 1 }
                    },
                    {
                        "attacker": "ELECTRIC",
                        "defender": "GROUND",
                        "multiplier": { "numerator": 0, "denominator": 1 }
                    }
                ],
                "foresight_matchups": [
                    {
                        "attacker": "NORMAL",
                        "defender": "GHOST",
                        "multiplier": { "numerator": 0, "denominator": 1 }
                    }
                ]
            }),
        )
        .expect("apply type effectiveness payload");

        assert_eq!(
            data.type_effectiveness,
            TypeEffectivenessTable {
                matchups: vec![
                    crystal_core::battle::damage::TypeEffectivenessEntry {
                        attacker: pokemon_type("FIRE"),
                        defender: pokemon_type("GRASS"),
                        multiplier: crystal_core::battle::damage::TypeMultiplier {
                            numerator: 2,
                            denominator: 1,
                        },
                    },
                    crystal_core::battle::damage::TypeEffectivenessEntry {
                        attacker: pokemon_type("ELECTRIC"),
                        defender: pokemon_type("GROUND"),
                        multiplier: crystal_core::battle::damage::TypeMultiplier::zero(),
                    },
                ],
                foresight_matchups: vec![crystal_core::battle::damage::TypeEffectivenessEntry {
                    attacker: pokemon_type("NORMAL"),
                    defender: pokemon_type("GHOST"),
                    multiplier: crystal_core::battle::damage::TypeMultiplier::zero(),
                }],
            }
        );
    }

    #[test]
    fn content_pack_payloads_merge_type_categories_as_exact_pack_data() {
        let mut data = GameDataSet::default();

        data.apply_content_pack_payload(
            ContentPackCategory::TypeCategories,
            serde_json::json!({
                "physical": ["NORMAL", "FIGHTING", "BIRD"],
                "special": ["FIRE", "WATER", "PSYCHIC_TYPE"]
            }),
        )
        .expect("apply type categories payload");

        assert_eq!(
            data.type_categories,
            TypeCategories {
                physical: vec![
                    "NORMAL".to_string(),
                    "FIGHTING".to_string(),
                    "BIRD".to_string(),
                ],
                special: vec![
                    "FIRE".to_string(),
                    "WATER".to_string(),
                    "PSYCHIC_TYPE".to_string(),
                ],
            }
        );
    }

    #[test]
    fn content_pack_payloads_merge_move_priorities_as_exact_pack_data() {
        let mut data = GameDataSet::default();

        data.apply_content_pack_payload(
            ContentPackCategory::MovePriorities,
            serde_json::json!({
                "base_priority": 1,
                "effect_priorities": [
                    { "move_effect": "PROTECT", "priority": 3 },
                    { "move_effect": "PRIORITY_HIT", "priority": 2 }
                ],
                "move_priorities": [
                    { "move": "VITAL_THROW", "priority": 0 }
                ]
            }),
        )
        .expect("apply move priorities payload");

        assert_eq!(
            data.move_priorities,
            MovePriorityTable {
                base_priority: 1,
                effect_priorities: vec![
                    crystal_core::battle::turn::MoveEffectPriority {
                        move_effect: "PROTECT".to_string(),
                        priority: 3,
                    },
                    crystal_core::battle::turn::MoveEffectPriority {
                        move_effect: "PRIORITY_HIT".to_string(),
                        priority: 2,
                    },
                ],
                move_priorities: vec![crystal_core::battle::turn::MovePriorityOverride {
                    r#move: "VITAL_THROW".to_string(),
                    priority: 0,
                }],
            }
        );
    }

    #[test]
    fn content_pack_payloads_merge_weather_modifiers_as_exact_pack_data() {
        let mut data = GameDataSet::default();

        data.apply_content_pack_payload(
            ContentPackCategory::WeatherModifiers,
            serde_json::json!({
                "type_modifiers": [
                    {
                        "weather": "WEATHER_RAIN",
                        "move_type": "WATER",
                        "multiplier": { "numerator": 3, "denominator": 2 }
                    },
                    {
                        "weather": "WEATHER_SUN",
                        "move_type": "FIRE",
                        "multiplier": { "numerator": 3, "denominator": 2 }
                    }
                ],
                "move_effect_modifiers": [
                    {
                        "weather": "WEATHER_RAIN",
                        "move_effect": "EFFECT_SOLARBEAM",
                        "multiplier": { "numerator": 1, "denominator": 2 }
                    }
                ]
            }),
        )
        .expect("apply weather modifiers payload");

        assert_eq!(
            data.weather_modifiers,
            WeatherModifiers {
                type_modifiers: vec![
                    crystal_core::battle::damage::WeatherTypeModifier {
                        weather: "WEATHER_RAIN".to_string(),
                        move_type: pokemon_type("WATER"),
                        multiplier: crystal_core::battle::damage::TypeMultiplier {
                            numerator: 3,
                            denominator: 2,
                        },
                    },
                    crystal_core::battle::damage::WeatherTypeModifier {
                        weather: "WEATHER_SUN".to_string(),
                        move_type: pokemon_type("FIRE"),
                        multiplier: crystal_core::battle::damage::TypeMultiplier {
                            numerator: 3,
                            denominator: 2,
                        },
                    },
                ],
                move_effect_modifiers: vec![
                    crystal_core::battle::damage::WeatherMoveEffectModifier {
                        weather: "WEATHER_RAIN".to_string(),
                        move_effect: "EFFECT_SOLARBEAM".to_string(),
                        multiplier: crystal_core::battle::damage::TypeMultiplier {
                            numerator: 1,
                            denominator: 2,
                        },
                    },
                ],
            }
        );
    }

    #[test]
    fn modpack_overlay_replaces_marts_by_exact_id() {
        let mut data = GameDataSet {
            marts: MartCatalog(
                [("MartNew".to_string(), vec!["POTION".to_string()])]
                    .into_iter()
                    .collect(),
            ),
            ..GameDataSet::default()
        };
        let manifest = ModpackManifest {
            payload: ModpackPayload {
                marts: MartCatalog(
                    [("MartNew".to_string(), vec!["POKE_BALL".to_string()])]
                        .into_iter()
                        .collect(),
                ),
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };

        data.apply_modpack(&manifest).expect("apply mart manifest");

        assert_eq!(
            data.marts.inventory_ids("MartNew").expect("mart"),
            &["POKE_BALL".to_string()]
        );
    }

    #[test]
    fn base_game_data_is_loaded_from_the_core_modular_pack() {
        let root = repository_root_for_tests();
        let data = AssetRoot::new(root)
            .load_base_game_data()
            .expect("load base game data");

        assert_eq!(data.pokemon.len(), 251);
        assert_eq!(data.pokemon["BULBASAUR"].base_stats.hp, 45);
        assert_eq!(data.moves.len(), 251);
        assert_eq!(data.moves["POUND"].pp, 35);
        assert_eq!(data.learnsets.len(), 251);
        assert_eq!(data.learnsets["BULBASAUR"][0].1, "TACKLE");
        assert_eq!(
            data.evolutions
                .entries_for("BULBASAUR")
                .expect("BULBASAUR evolutions")[0]
                .species,
            "IVYSAUR"
        );
        assert_eq!(data.items.len(), 256);
        assert_eq!(
            data.map_attributes["Route29"].map_constant.as_deref(),
            Some("ROUTE_29")
        );
        assert!(data.map_scripts.contains_key("Route29_MapScripts"));
        assert!(data.map_blocks.contains_key("Route29_Blocks"));
        assert!(data.npcs.contains_key("Route29"));
        assert!(!data.phone_scripts.is_empty());
        assert_eq!(
            data.runtime_spawn_points
                .get("0")
                .map(|spawn| spawn.map_name.as_str()),
            Some("PlayersHouse2F")
        );
        assert_eq!(
            data.runtime_map_metadata
                .get("ROUTE_29")
                .map(|metadata| metadata.name.as_str()),
            Some("Route29")
        );
        assert!(
            data.initialize_events
                .event_flags
                .contains(&"EVENT_RIVAL_CHERRYGROVE_CITY".to_string())
        );
        assert_eq!(
            data.story_event_script_constants.global.get("TRUE"),
            Some(&1)
        );
        assert_eq!(
            data.asm_text
                .get("WildPokemonAppearedText")
                .map(String::as_str),
            Some("Wild @\n\n\nappeared!")
        );
        assert_eq!(data.move_names.first().map(String::as_str), Some("POUND"));
        assert!(data.battle_animations.contains_key("BattleAnim_Pound"));
        assert_eq!(
            data.battle_animation_table.get(1).map(String::as_str),
            Some("BattleAnim_Pound")
        );
        let battle_anim_bundle: Value =
            serde_json::from_str(&data.battle_anim_bundle).expect("battle anim bundle json");
        let sprite_anim_bundle: Value =
            serde_json::from_str(&data.sprite_anim_bundle).expect("sprite anim bundle json");
        assert!(battle_anim_bundle.get("objects").is_some());
        assert!(sprite_anim_bundle.get("oam_sets").is_some());
        assert_eq!(data.sprite_palette_defaults.get("SPRITE_CHRIS"), Some(&0));
        assert!(
            data.pokegear_town_map_palette_map
                .get("town_map")
                .is_some_and(|entries| !entries.is_empty())
        );
        assert_eq!(
            data.pokemon_cries.get("CHIKORITA").map(|cry| (
                cry.cry.as_str(),
                cry.pitch,
                cry.length
            )),
            Some(("CRY_CHIKORITA", -16, 176))
        );
        assert_eq!(
            data.pokemon_cries
                .get("AMPHAROS")
                .map(|cry| (cry.cry.as_str(), cry.pitch, cry.length)),
            Some(("CRY_AMPHAROS", -124, 232))
        );
        assert!(!data.pokemon_cries.contains_key("252"));
        assert!(data.flee_mons.always.contains(&"RAIKOU".to_string()));
        assert_eq!(
            data.pc_strings
                .get("PCString_ChooseaPKMN")
                .map(String::as_str),
            Some("Choose a <PK><MN>.")
        );
        assert_eq!(
            data.menu_icons.get("CHIKORITA").map(String::as_str),
            Some("ICON_ODDISH")
        );
        assert_eq!(
            data.pokedex_entries
                .get("CHIKORITA")
                .map(|entry| entry.classification.as_str()),
            Some("LEAF")
        );
        assert!(data.pokemon_frontpic_anim.contains_key("CHIKORITA"));
    }

    #[test]
    fn base_core_pack_compiles_with_exported_playability_rules() {
        let root = repository_root_for_tests();
        let compiled = AssetRoot::new(root)
            .compile_modpacks(&[], ModpackCompileOptions::default())
            .expect("compile base core pack");

        assert!(
            !compiled.report.has_errors(),
            "{:?}",
            compiled.report.diagnostics
        );
        assert!(
            compiled
                .report
                .solvable_events
                .contains(&"EVENT_HALL_OF_FAME".to_string())
        );
    }

    #[test]
    fn modpack_payload_empty_sections_are_authoritative() {
        let mut data = GameDataSet {
            fishing: FishingCatalog {
                groups: [(
                    "FISHGROUP_LAKE".to_string(),
                    crystal_core::world::fishing::FishingGroup {
                        bite_threshold: crystal_core::world::fishing::threshold(50, true),
                        rod_tables: BTreeMap::new(),
                    },
                )]
                .into_iter()
                .collect(),
                time_groups: Vec::new(),
                swarm_rules: Vec::new(),
                rod_items: Vec::new(),
            },
            flee_mons: FleeMonTables {
                always: vec!["RAIKOU".to_string()],
                often: vec!["ENTEI".to_string()],
                sometimes: vec!["SUICUNE".to_string()],
            },
            initialize_events: InitializeEventsConfig {
                event_flags: vec!["EVENT_GOT_A_POKEMON_FROM_ELM".to_string()],
                engine_flags: vec!["ENGINE_POKEGEAR".to_string()],
                variable_sprites: [(
                    "SPRITE_WEIRD_TREE".to_string(),
                    "SPRITE_SUDOWOODO".to_string(),
                )]
                .into_iter()
                .collect(),
            },
            story_event_script_constants: StoryEventScriptConstants {
                global: [("TRUE".to_string(), 1)].into_iter().collect(),
                maps: BTreeMap::new(),
            },
            move_names: vec!["POUND".to_string()],
            battle_animation_table: vec![
                "BattleAnim_0".to_string(),
                "BattleAnim_Pound".to_string(),
            ],
            battle_anim_bundle: "{\"objects\":[]}".to_string(),
            sprite_anim_bundle: "{\"oam_sets\":[]}".to_string(),
            ..GameDataSet::default()
        };
        let manifest = ModpackManifest {
            metadata: ModpackMetadata {
                id: "empty-authoritative".to_string(),
                name: "Empty Authoritative".to_string(),
                version: "1.0.0".to_string(),
                author: None,
                description: None,
            },
            payload: ModpackPayload {
                fishing: FishingCatalog::default(),
                flee_mons: FleeMonTables::default(),
                initialize_events: InitializeEventsConfig::default(),
                story_event_script_constants: StoryEventScriptConstants::default(),
                move_names: Vec::new(),
                battle_animation_table: Vec::new(),
                battle_anim_bundle: String::new(),
                sprite_anim_bundle: String::new(),
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };

        data.apply_modpack(&manifest)
            .expect("apply explicit empty authoritative sections");

        assert!(data.fishing.groups.is_empty());
        assert!(data.flee_mons.always.is_empty());
        assert!(data.flee_mons.often.is_empty());
        assert!(data.flee_mons.sometimes.is_empty());
        assert!(data.initialize_events.event_flags.is_empty());
        assert!(data.initialize_events.engine_flags.is_empty());
        assert!(data.initialize_events.variable_sprites.is_empty());
        assert!(data.story_event_script_constants.global.is_empty());
        assert!(data.story_event_script_constants.maps.is_empty());
        assert!(data.move_names.is_empty());
        assert!(data.battle_animation_table.is_empty());
        assert!(data.battle_anim_bundle.is_empty());
        assert!(data.sprite_anim_bundle.is_empty());
    }

    #[test]
    fn modpack_overlay_adds_and_replaces_by_stable_ids() {
        let mut data = GameDataSet {
            pokemon: [(species().id.clone(), species())].into_iter().collect(),
            moves: [(
                "SPARK".to_string(),
                Move {
                    name: "SPARK".to_string(),
                    move_type: pokemon_type("ELECTRIC"),
                    power: 40,
                    accuracy: 100,
                    pp: 30,
                    effect: "NORMAL_HIT".to_string(),
                    effect_chance: 0,
                    stat: None,
                    amount: None,
                },
            )]
            .into_iter()
            .collect(),
            ..GameDataSet::default()
        };
        let replacement = PokemonSpecies {
            base_stats: BaseStats::new(99, 50, 40, 60, 70, 50),
            ..species()
        };
        let manifest = ModpackManifest {
            metadata: ModpackMetadata {
                id: "overlay".to_string(),
                name: "Overlay".to_string(),
                version: "1.0.0".to_string(),
                author: None,
                description: None,
            },
            payload: ModpackPayload {
                pokemon: vec![replacement],
                moves: vec![Move {
                    name: "NEW_MOVE".to_string(),
                    move_type: pokemon_type("NORMAL"),
                    power: 1,
                    accuracy: 100,
                    pp: 40,
                    effect: "NORMAL_HIT".to_string(),
                    effect_chance: 0,
                    stat: None,
                    amount: None,
                }],
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };

        data.apply_modpack(&manifest)
            .expect("manifest should apply with exact exported ids");

        assert_eq!(data.pokemon["NEW_MON"].base_stats.hp, 99);
        assert!(data.moves.contains_key("SPARK"));
        assert_eq!(data.moves["NEW_MOVE"].pp, 40);
    }

    #[test]
    fn verifier_rejects_missing_encounter_species_before_pack_is_playable() {
        let mut known_species = species();
        known_species.tmhm_learnset.clear();
        let data = GameDataSet {
            pokemon: [(known_species.id.clone(), known_species)]
                .into_iter()
                .collect(),
            moves: [("TACKLE".to_string(), test_move("TACKLE"))]
                .into_iter()
                .collect(),
            maps: [(
                "Start".to_string(),
                test_map_module("Start", "START_MAP", None),
            )]
            .into_iter()
            .collect(),
            wild_encounters: [(
                "Start".to_string(),
                WildEncounterData {
                    map_name: "Start".to_string(),
                    grass: Some(WildEncounterTable {
                        morning: vec![WildEncounter {
                            level: 3,
                            species: "MISSING_MON".to_string(),
                        }],
                        ..WildEncounterTable::default()
                    }),
                    ..WildEncounterData::default()
                },
            )]
            .into_iter()
            .collect(),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.has_errors());
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_encounter_species"
                && diagnostic.message.contains("MISSING_MON")
        }));
    }

    #[test]
    fn verifier_rejects_wild_encounter_rate_table_mismatches_without_defaults() {
        let data = GameDataSet {
            maps: [(
                "Start".to_string(),
                test_map_module("Start", "START_MAP", None),
            )]
            .into_iter()
            .collect(),
            wild_encounters: [(
                "Start".to_string(),
                WildEncounterData {
                    map_name: "Start".to_string(),
                    grass_rates: Some(
                        [("DAY".to_string(), 20), ("night".to_string(), 10)]
                            .into_iter()
                            .collect(),
                    ),
                    water_rate: Some(15),
                    grass: Some(WildEncounterTable {
                        morning: vec![WildEncounter {
                            level: 3,
                            species: "NEW_MON".to_string(),
                        }],
                        ..WildEncounterTable::default()
                    }),
                    water: Some(WildEncounterTable {
                        morning: vec![WildEncounter {
                            level: 10,
                            species: "NEW_MON".to_string(),
                        }],
                        ..WildEncounterTable::default()
                    }),
                },
            )]
            .into_iter()
            .collect(),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.has_errors());
        for code in [
            "unknown_grass_encounter_rate_time",
            "missing_grass_encounter_rate",
            "empty_grass_encounter_slots",
            "empty_water_encounter_slots",
        ] {
            assert!(
                report
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.code == code),
                "missing diagnostic {code}: {:?}",
                report.diagnostics
            );
        }
    }

    #[test]
    fn verifier_rejects_present_wild_encounter_tables_without_exact_rates() {
        let data = GameDataSet {
            maps: [(
                "Start".to_string(),
                test_map_module("Start", "START_MAP", None),
            )]
            .into_iter()
            .collect(),
            wild_encounters: [(
                "Start".to_string(),
                WildEncounterData {
                    map_name: "Start".to_string(),
                    grass_rates: None,
                    water_rate: None,
                    grass: Some(WildEncounterTable::default()),
                    water: Some(WildEncounterTable::default()),
                },
            )]
            .into_iter()
            .collect(),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.has_errors());
        for time in ["morning", "day", "night"] {
            assert!(
                report.diagnostics.iter().any(|diagnostic| {
                    diagnostic.code == "missing_grass_encounter_rate"
                        && diagnostic.message.contains(time)
                }),
                "missing grass rate diagnostic for {time}: {:?}",
                report.diagnostics
            );
        }
        assert!(
            report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "missing_water_encounter_rate"),
            "missing water rate diagnostic: {:?}",
            report.diagnostics
        );
    }

    #[test]
    fn verifier_rejects_positive_wild_encounter_rates_without_tables() {
        let data = GameDataSet {
            maps: [(
                "Start".to_string(),
                test_map_module("Start", "START_MAP", None),
            )]
            .into_iter()
            .collect(),
            wild_encounters: [(
                "Start".to_string(),
                WildEncounterData {
                    map_name: "Start".to_string(),
                    grass_rates: Some([("day".to_string(), 20)].into_iter().collect()),
                    water_rate: Some(15),
                    ..WildEncounterData::default()
                },
            )]
            .into_iter()
            .collect(),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.has_errors());
        assert!(
            report
                .diagnostics
                .iter()
                .any(|diagnostic| { diagnostic.code == "missing_grass_encounter_table" })
        );
        assert!(
            report
                .diagnostics
                .iter()
                .any(|diagnostic| { diagnostic.code == "missing_water_encounter_table" })
        );
    }

    #[test]
    fn verifier_rejects_present_field_encounter_tables_with_unusable_buckets() {
        let data = GameDataSet {
            pokemon: [("NEW_MON".to_string(), species())].into_iter().collect(),
            maps: [(
                "Start".to_string(),
                test_map_module("Start", "START_MAP", None),
            )]
            .into_iter()
            .collect(),
            field_encounters: [(
                "Start".to_string(),
                FieldEncounterData {
                    map_name: "Start".to_string(),
                    headbutt: Some(FieldEncounterTable {
                        common: vec![FieldEncounterEntry {
                            weight: 90,
                            species: "NEW_MON".to_string(),
                            level: 3,
                        }],
                        rare: Vec::new(),
                    }),
                    rock_smash: Some(FieldEncounterTable {
                        common: vec![FieldEncounterEntry {
                            weight: 0,
                            species: "NEW_MON".to_string(),
                            level: 8,
                        }],
                        rare: Vec::new(),
                    }),
                },
            )]
            .into_iter()
            .collect(),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.has_errors());
        for code in [
            "invalid_field_encounter_weight_total",
            "empty_field_encounter_bucket",
            "zero_weight_field_encounter",
        ] {
            assert!(
                report
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.code == code),
                "missing diagnostic {code}: {:?}",
                report.diagnostics
            );
        }
    }

    #[test]
    fn verifier_rejects_missing_midi_asset_files() {
        let data = GameDataSet {
            audio: vec![
                ModpackAudioAsset::music(
                    "MUSIC_MISSING_THEME",
                    "content-packs/test/music/MUSIC_MISSING_THEME.mid",
                )
                .expect("valid MIDI asset shape"),
            ],
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "missing_audio_file" && diagnostic.subject == "MUSIC_MISSING_THEME"
        }));
    }

    #[test]
    fn verifier_rejects_invalid_midi_asset_bytes() {
        let root = temp_test_path("invalid-midi-root");
        let midi_path = root.join("apps/web/assets/data/content-packs/test/music/MUSIC_BAD.mid");
        std::fs::create_dir_all(midi_path.parent().expect("midi parent")).expect("create midi dir");
        std::fs::write(&midi_path, b"not midi").expect("write invalid midi");
        let data = GameDataSet {
            audio: vec![
                ModpackAudioAsset::music("MUSIC_BAD", "content-packs/test/music/MUSIC_BAD.mid")
                    .expect("valid MIDI asset shape"),
            ],
            ..GameDataSet::default()
        };

        let report = verify_game_data(&AssetRoot::new(&root), &data, &PlayabilityRules::default());

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_midi_file" && diagnostic.subject == "MUSIC_BAD"
        }));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn verifier_rejects_capture_rules_for_missing_species() {
        let data = GameDataSet {
            capture_rules: CaptureRules {
                fast_ball_species: ["MISSING_FAST".to_string()].into_iter().collect(),
                heavy_ball_modifiers: [("MISSING_HEAVY".to_string(), 40)].into_iter().collect(),
                ball_rules: BTreeMap::new(),
                guaranteed_capture_balls: BTreeSet::new(),
                status_bonus: BTreeMap::new(),
            },
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_fast_ball_species" && diagnostic.subject == "MISSING_FAST"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_heavy_ball_species" && diagnostic.subject == "MISSING_HEAVY"
        }));
    }

    #[test]
    fn verifier_rejects_unknown_ball_pocket_items_without_poke_ball_fallback() {
        let mut mod_ball = test_item("MOD_BALL");
        mod_ball.pocket = item_pocket("BALL");
        let mut poke_ball = test_item("POKE_BALL");
        poke_ball.pocket = item_pocket("BALL");
        let data = GameDataSet {
            items: [
                ("MOD_BALL".to_string(), mod_ball),
                ("POKE_BALL".to_string(), poke_ball),
            ]
            .into_iter()
            .collect(),
            capture_rules: CaptureRules {
                fast_ball_species: BTreeSet::new(),
                heavy_ball_modifiers: BTreeMap::new(),
                ball_rules: [(
                    "POKE_BALL".to_string(),
                    crystal_core::battle::capture::CaptureBallRule {
                        multiplier_numerator: 1,
                        multiplier_denominator: 1,
                        battle_type: String::new(),
                        skip_hp_calc: false,
                        use_heavy_ball_weight_modifier: false,
                        use_level_ball_multiplier: false,
                        require_same_species: false,
                        require_same_gender: false,
                        require_fast_species: false,
                    },
                )]
                .into_iter()
                .collect(),
                guaranteed_capture_balls: BTreeSet::new(),
                status_bonus: BTreeMap::new(),
            },
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_capture_ball_item" && diagnostic.subject == "MOD_BALL"
        }));
        assert!(!report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_capture_ball_item" && diagnostic.subject == "POKE_BALL"
        }));
    }

    #[test]
    fn verifier_accepts_modpack_item_menu_ids_as_definitive_data() {
        let mut mod_menu = test_item("MOD_MENU_ITEM");
        mod_menu.field_menu = "ITEMMENU_MODDED".to_string();
        mod_menu.battle_menu = "ITEMMENU_NOUSE".to_string();
        let mut exact_menu = test_item("EXACT_MENU_ITEM");
        exact_menu.field_menu = "ITEMMENU_CURRENT".to_string();
        exact_menu.battle_menu = "ITEMMENU_PARTY".to_string();
        let data = GameDataSet {
            items: [
                ("MOD_MENU_ITEM".to_string(), mod_menu),
                ("EXACT_MENU_ITEM".to_string(), exact_menu),
            ]
            .into_iter()
            .collect(),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(!report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_item_menu"
                && (diagnostic.subject == "MOD_MENU_ITEM"
                    || diagnostic.subject == "EXACT_MENU_ITEM")
        }));
    }

    #[test]
    fn verifier_rejects_invalid_utility_item_payloads_without_effect_inference() {
        let mut bad_poke_doll = test_item("BAD_POKE_DOLL");
        bad_poke_doll.effect = "MOD_ESCAPE_ITEM".to_string();
        bad_poke_doll.battle_escape_mode = Some("ANY_BATTLE".to_string());
        let mut bad_repel = test_item("BAD_REPEL");
        bad_repel.effect = "MOD_REPEL_ITEM".to_string();
        bad_repel.repel_steps = Some(0);
        let mut bad_rope = test_item("BAD_ESCAPE_ROPE");
        bad_rope.effect = "ESCAPE_ROPE".to_string();
        let mut exact_poke_doll = test_item("POKE_DOLL");
        exact_poke_doll.effect = "MOD_ESCAPE_ITEM".to_string();
        exact_poke_doll.battle_escape_mode = Some("WILD_BATTLE".to_string());
        let mut exact_repel = test_item("REPEL");
        exact_repel.effect = "MOD_REPEL_ITEM".to_string();
        exact_repel.repel_steps = Some(100);
        let mut exact_rope = test_item("ESCAPE_ROPE");
        exact_rope.effect = "MOD_ESCAPE_ROPE".to_string();
        exact_rope.escape_rope_mode = Some("MOD_WARP".to_string());
        let data = GameDataSet {
            items: [
                ("BAD_POKE_DOLL".to_string(), bad_poke_doll),
                ("BAD_REPEL".to_string(), bad_repel),
                ("BAD_ESCAPE_ROPE".to_string(), bad_rope),
                ("POKE_DOLL".to_string(), exact_poke_doll),
                ("REPEL".to_string(), exact_repel),
                ("ESCAPE_ROPE".to_string(), exact_rope),
            ]
            .into_iter()
            .collect(),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        for (code, subject) in [
            ("invalid_item_battle_escape_mode", "BAD_POKE_DOLL"),
            ("invalid_item_repel_steps", "BAD_REPEL"),
        ] {
            assert!(
                report
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.code == code && diagnostic.subject == subject),
                "missing diagnostic {code} for {subject}: {:?}",
                report.diagnostics
            );
        }
        for subject in ["POKE_DOLL", "REPEL", "ESCAPE_ROPE"] {
            assert!(
                !report
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.subject == subject)
            );
        }
    }

    #[test]
    fn verifier_rejects_invalid_battle_item_payloads_without_effect_inference() {
        let mut bad_restore_hp = test_item("BAD_RESTORE_HP");
        bad_restore_hp.effect = "MOD_HEAL".to_string();
        bad_restore_hp.parameter = -2;
        let mut bad_status_heal = test_item("BAD_STATUS_HEAL");
        bad_status_heal.effect = "STATUS_HEAL".to_string();
        let mut bad_revive = test_item("BAD_REVIVE");
        bad_revive.effect = "MOD_REVIVE".to_string();
        bad_revive.revive_hp_percent = Some(0);
        let mut bad_sacred_ash = test_item("BAD_SACRED_ASH");
        bad_sacred_ash.effect = "MOD_PARTY_REVIVE".to_string();
        bad_sacred_ash.party_revive_hp_percent = Some(0);
        let mut bad_restore_pp = test_item("BAD_RESTORE_PP");
        bad_restore_pp.effect = "MOD_RESTORE_PP".to_string();
        bad_restore_pp.pp_restore_scope = Some("PARTY".to_string());
        bad_restore_pp.pp_restore_points = Some(0);
        let mut bad_pp_up = test_item("BAD_PP_UP");
        bad_pp_up.effect = "MOD_PP_UP".to_string();
        bad_pp_up.pp_up_stages = Some(4);
        let mut bad_vitamin = test_item("BAD_VITAMIN");
        bad_vitamin.effect = "MOD_VITAMIN".to_string();
        bad_vitamin.vitamin_stat = Some("LUCK".to_string());
        bad_vitamin.vitamin_stat_exp = Some(0);
        bad_vitamin.vitamin_max_stat_exp = Some(0);
        let mut bad_rare_candy = test_item("BAD_RARE_CANDY");
        bad_rare_candy.effect = "MOD_CANDY".to_string();
        bad_rare_candy.rare_candy_level_gain = Some(0);
        let mut bad_x_item = test_item("BAD_X_ITEM");
        bad_x_item.effect = "MOD_BATTLE_BOOST".to_string();
        bad_x_item.battle_stat_boost_stat = Some("LUCK".to_string());
        bad_x_item.battle_stat_boost_stages = Some(7);
        let mut bad_guard_spec = test_item("BAD_GUARD_SPEC");
        bad_guard_spec.effect = "MOD_GUARD".to_string();
        bad_guard_spec.battle_stat_drop_guard = Some(false);
        let mut bad_dire_hit = test_item("BAD_DIRE_HIT");
        bad_dire_hit.effect = "MOD_FOCUS".to_string();
        bad_dire_hit.battle_focus_energy = Some(false);
        let mut bad_bitter_berry = test_item("BAD_BITTER_BERRY");
        bad_bitter_berry.effect = "MOD_CONFUSION_HEAL".to_string();
        bad_bitter_berry.confusion_heal = Some(false);

        let mut exact_restore_hp = test_item("EXACT_RESTORE_HP");
        exact_restore_hp.effect = "MOD_HEAL".to_string();
        exact_restore_hp.parameter = 20;
        let mut exact_status_heal = test_item("EXACT_STATUS_HEAL");
        exact_status_heal.effect = "MOD_STATUS_HEAL".to_string();
        exact_status_heal.status_heals = vec!["POISON".to_string()];
        let mut exact_revive = test_item("EXACT_REVIVE");
        exact_revive.effect = "MOD_REVIVE".to_string();
        exact_revive.revive_hp_percent = Some(50);
        let mut exact_sacred_ash = test_item("EXACT_SACRED_ASH");
        exact_sacred_ash.effect = "MOD_PARTY_REVIVE".to_string();
        exact_sacred_ash.party_revive_hp_percent = Some(100);
        let mut exact_restore_pp = test_item("EXACT_RESTORE_PP");
        exact_restore_pp.effect = "MOD_RESTORE_PP".to_string();
        exact_restore_pp.pp_restore_scope = Some("MOVE".to_string());
        exact_restore_pp.pp_restore_points = Some(10);
        let mut exact_pp_up = test_item("EXACT_PP_UP");
        exact_pp_up.effect = "MOD_PP_UP".to_string();
        exact_pp_up.pp_up_stages = Some(1);
        let mut exact_vitamin = test_item("EXACT_VITAMIN");
        exact_vitamin.effect = "MOD_VITAMIN".to_string();
        exact_vitamin.vitamin_stat = Some("SPECIAL".to_string());
        exact_vitamin.vitamin_stat_exp = Some(2560);
        exact_vitamin.vitamin_max_stat_exp = Some(25600);
        let mut exact_rare_candy = test_item("EXACT_RARE_CANDY");
        exact_rare_candy.effect = "MOD_CANDY".to_string();
        exact_rare_candy.rare_candy_level_gain = Some(1);
        let mut exact_x_item = test_item("EXACT_X_ITEM");
        exact_x_item.effect = "MOD_BATTLE_BOOST".to_string();
        exact_x_item.battle_stat_boost_stat = Some("SPECIAL_ATTACK".to_string());
        exact_x_item.battle_stat_boost_stages = Some(1);
        let mut exact_guard_spec = test_item("EXACT_GUARD_SPEC");
        exact_guard_spec.effect = "MOD_GUARD".to_string();
        exact_guard_spec.battle_stat_drop_guard = Some(true);
        let mut exact_dire_hit = test_item("EXACT_DIRE_HIT");
        exact_dire_hit.effect = "MOD_FOCUS".to_string();
        exact_dire_hit.battle_focus_energy = Some(true);
        let mut exact_bitter_berry = test_item("EXACT_BITTER_BERRY");
        exact_bitter_berry.effect = "MOD_CONFUSION_HEAL".to_string();
        exact_bitter_berry.confusion_heal = Some(true);

        let data = GameDataSet {
            items: [
                ("BAD_RESTORE_HP".to_string(), bad_restore_hp),
                ("BAD_STATUS_HEAL".to_string(), bad_status_heal),
                ("BAD_REVIVE".to_string(), bad_revive),
                ("BAD_SACRED_ASH".to_string(), bad_sacred_ash),
                ("BAD_RESTORE_PP".to_string(), bad_restore_pp),
                ("BAD_PP_UP".to_string(), bad_pp_up),
                ("BAD_VITAMIN".to_string(), bad_vitamin),
                ("BAD_RARE_CANDY".to_string(), bad_rare_candy),
                ("BAD_X_ITEM".to_string(), bad_x_item),
                ("BAD_GUARD_SPEC".to_string(), bad_guard_spec),
                ("BAD_DIRE_HIT".to_string(), bad_dire_hit),
                ("BAD_BITTER_BERRY".to_string(), bad_bitter_berry),
                ("EXACT_RESTORE_HP".to_string(), exact_restore_hp),
                ("EXACT_STATUS_HEAL".to_string(), exact_status_heal),
                ("EXACT_REVIVE".to_string(), exact_revive),
                ("EXACT_SACRED_ASH".to_string(), exact_sacred_ash),
                ("EXACT_RESTORE_PP".to_string(), exact_restore_pp),
                ("EXACT_PP_UP".to_string(), exact_pp_up),
                ("EXACT_VITAMIN".to_string(), exact_vitamin),
                ("EXACT_RARE_CANDY".to_string(), exact_rare_candy),
                ("EXACT_X_ITEM".to_string(), exact_x_item),
                ("EXACT_GUARD_SPEC".to_string(), exact_guard_spec),
                ("EXACT_DIRE_HIT".to_string(), exact_dire_hit),
                ("EXACT_BITTER_BERRY".to_string(), exact_bitter_berry),
            ]
            .into_iter()
            .collect(),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        for (code, subject) in [
            ("invalid_item_heal_amount", "BAD_RESTORE_HP"),
            ("invalid_item_revive_hp_percent", "BAD_REVIVE"),
            ("invalid_item_party_revive_hp_percent", "BAD_SACRED_ASH"),
            ("invalid_item_pp_restore_scope", "BAD_RESTORE_PP"),
            ("invalid_item_pp_restore_points", "BAD_RESTORE_PP"),
            ("invalid_item_pp_up_stages", "BAD_PP_UP"),
            ("invalid_item_vitamin_stat", "BAD_VITAMIN"),
            ("invalid_item_vitamin_stat_exp", "BAD_VITAMIN"),
            ("invalid_item_vitamin_max_stat_exp", "BAD_VITAMIN"),
            ("invalid_item_rare_candy_level_gain", "BAD_RARE_CANDY"),
            ("invalid_item_battle_stat_boost_stat", "BAD_X_ITEM"),
            ("invalid_item_battle_stat_boost_stages", "BAD_X_ITEM"),
            ("invalid_item_battle_stat_drop_guard", "BAD_GUARD_SPEC"),
            ("invalid_item_battle_focus_energy", "BAD_DIRE_HIT"),
            ("invalid_item_confusion_heal", "BAD_BITTER_BERRY"),
        ] {
            assert!(
                report
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.code == code && diagnostic.subject == subject),
                "missing diagnostic {code} for {subject}: {:?}",
                report.diagnostics
            );
        }
        for subject in [
            "EXACT_RESTORE_HP",
            "EXACT_STATUS_HEAL",
            "EXACT_REVIVE",
            "EXACT_SACRED_ASH",
            "EXACT_RESTORE_PP",
            "EXACT_PP_UP",
            "EXACT_VITAMIN",
            "EXACT_RARE_CANDY",
            "EXACT_X_ITEM",
            "EXACT_GUARD_SPEC",
            "EXACT_DIRE_HIT",
            "EXACT_BITTER_BERRY",
        ] {
            assert!(
                !report
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.subject == subject),
                "unexpected diagnostic for {subject}: {:?}",
                report.diagnostics
            );
        }
    }

    #[test]
    fn verifier_rejects_unknown_evolution_facts_without_case_coercion() {
        let mut source = species();
        source.id = "NEW_MON".to_string();
        source.tmhm_learnset.clear();
        let mut target = species();
        target.id = "NEW_FORM".to_string();
        target.tmhm_learnset.clear();
        let data = GameDataSet {
            pokemon: [
                ("NEW_MON".to_string(), source),
                ("NEW_FORM".to_string(), target),
            ]
            .into_iter()
            .collect(),
            items: [("THUNDERSTONE".to_string(), test_item("THUNDERSTONE"))]
                .into_iter()
                .collect(),
            evolutions: EvolutionTable(
                [
                    (
                        "new_mon".to_string(),
                        vec![EvolutionEntry::level("NEW_FORM", 20)],
                    ),
                    (
                        "NEW_MON".to_string(),
                        vec![
                            EvolutionEntry::item("new_form", "thunderstone"),
                            EvolutionEntry::happiness("NEW_FORM", "MORNINGISH"),
                            EvolutionEntry::stat("NEW_FORM", 20, "ATTACKIER"),
                            EvolutionEntry {
                                method: "MOON_PHASE".to_string(),
                                species: "NEW_FORM".to_string(),
                                level: None,
                                item: None,
                                held_item: None,
                                happiness: None,
                                stat_ratio: None,
                            },
                        ],
                    ),
                ]
                .into_iter()
                .collect(),
            ),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        for expected in [
            "unknown_evolution_source_species",
            "unknown_evolution_target_species",
            "unknown_evolution_item",
            "unknown_evolution_happiness_window",
            "unknown_evolution_stat_ratio",
            "unknown_evolution_method",
        ] {
            assert!(
                report
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.code == expected),
                "missing diagnostic {expected}: {:?}",
                report.diagnostics
            );
        }
    }

    #[test]
    fn verifier_requires_explicit_empty_learnsets_and_evolutions() {
        let mut known_species = species();
        known_species.id = "FINAL_MON".to_string();
        known_species.tmhm_learnset.clear();
        let missing = GameDataSet {
            pokemon: [("FINAL_MON".to_string(), known_species.clone())]
                .into_iter()
                .collect(),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &missing,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "missing_species_learnset" && diagnostic.subject == "FINAL_MON"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "missing_species_evolutions" && diagnostic.subject == "FINAL_MON"
        }));

        let explicit_empty = GameDataSet {
            pokemon: [("FINAL_MON".to_string(), known_species)]
                .into_iter()
                .collect(),
            learnsets: [("FINAL_MON".to_string(), Vec::new())]
                .into_iter()
                .collect(),
            evolutions: EvolutionTable(
                [("FINAL_MON".to_string(), Vec::new())]
                    .into_iter()
                    .collect(),
            ),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &explicit_empty,
            &PlayabilityRules::default(),
        );

        assert!(!report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "missing_species_learnset" && diagnostic.subject == "FINAL_MON"
        }));
        assert!(!report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "missing_species_evolutions" && diagnostic.subject == "FINAL_MON"
        }));
    }

    #[test]
    fn verifier_rejects_unknown_species_held_items_without_case_coercion() {
        let mut known_species = species();
        known_species.tmhm_learnset.clear();
        known_species.item1 = Some("potion".to_string());
        known_species.item2 = Some("RARE_CANDY".to_string());
        let species_id = known_species.id.clone();
        let data = GameDataSet {
            pokemon: [(species_id.clone(), known_species)].into_iter().collect(),
            learnsets: [(species_id.clone(), Vec::new())].into_iter().collect(),
            evolutions: EvolutionTable([(species_id.clone(), Vec::new())].into_iter().collect()),
            items: [
                ("POTION".to_string(), test_item("POTION")),
                ("RARE_CANDY".to_string(), test_item("RARE_CANDY")),
            ]
            .into_iter()
            .collect(),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_species_held_item"
                && diagnostic.subject == species_id
                && diagnostic.message.contains("potion")
        }));
        assert!(!report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_species_held_item"
                && diagnostic.message.contains("RARE_CANDY")
        }));
    }

    #[test]
    fn verifier_rejects_unknown_mart_items_without_case_coercion() {
        let data = GameDataSet {
            items: [("POTION".to_string(), test_item("POTION"))]
                .into_iter()
                .collect(),
            marts: MartCatalog(
                [("MartNew".to_string(), vec!["potion".to_string()])]
                    .into_iter()
                    .collect(),
            ),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_mart_item"
                && diagnostic.subject == "MartNew"
                && diagnostic.message.contains("potion")
        }));
    }

    #[test]
    fn verifier_rejects_malformed_script_shop_commands_without_coercion() {
        let mut module = test_map_module("Start", "START_MAP", None);
        module.script_shop_commands = vec![
            ScriptShopCommand {
                mart_type: "MARTTYPE_STANDARD".to_string(),
                mart_id: "mart_cherrygrove".to_string(),
                source_script: "ClerkScript".to_string(),
                command_index: 1,
            },
            ScriptShopCommand {
                mart_type: "MARTTYPE_STANDARD".to_string(),
                mart_id: "0".to_string(),
                source_script: "ZeroScript".to_string(),
                command_index: 2,
            },
            ScriptShopCommand {
                mart_type: "marttype_standard".to_string(),
                mart_id: "MART_CHERRYGROVE".to_string(),
                source_script: "LowerTypeScript".to_string(),
                command_index: 3,
            },
        ];
        let data = GameDataSet {
            maps: [("Start".to_string(), module)].into_iter().collect(),
            marts: MartCatalog(
                [("MART_CHERRYGROVE".to_string(), vec!["POTION".to_string()])]
                    .into_iter()
                    .collect(),
            ),
            items: [("POTION".to_string(), test_item("POTION"))]
                .into_iter()
                .collect(),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_script_shop_mart"
                && diagnostic.subject == "Start:ClerkScript:1"
                && diagnostic.message.contains("mart_cherrygrove")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "script_shop_invalid_zero_mart"
                && diagnostic.subject == "Start:ZeroScript:2"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_script_shop_mart_type"
                && diagnostic.subject == "Start:LowerTypeScript:3"
                && diagnostic.message.contains("marttype_standard")
        }));
    }

    #[test]
    fn verifier_rejects_unknown_script_item_grants_without_case_coercion() {
        let mut module = test_map_module("Start", "START_MAP", None);
        module.script_item_grants = vec![ScriptItemGrant {
            item_id: "potion".to_string(),
            quantity: 1,
            source_script: "GiftScript".to_string(),
            command_index: 4,
            verbose: true,
        }];
        let data = GameDataSet {
            maps: [("Start".to_string(), module)].into_iter().collect(),
            items: [("POTION".to_string(), test_item("POTION"))]
                .into_iter()
                .collect(),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_script_item_grant_item"
                && diagnostic.subject == "Start:GiftScript:4"
                && diagnostic.message.contains("potion")
        }));
    }

    #[test]
    fn verifier_rejects_unknown_script_item_access_without_case_coercion() {
        let mut module = test_map_module("Start", "START_MAP", None);
        module.script_item_checks = vec![ScriptItemAccess {
            item_id: "pass".to_string(),
            source_script: "GateScript".to_string(),
            command_index: 3,
        }];
        module.script_item_takes = vec![ScriptItemAccess {
            item_id: "lost_item".to_string(),
            source_script: "CopycatScript".to_string(),
            command_index: 8,
        }];
        let data = GameDataSet {
            maps: [("Start".to_string(), module)].into_iter().collect(),
            items: [
                ("PASS".to_string(), test_item("PASS")),
                ("LOST_ITEM".to_string(), test_item("LOST_ITEM")),
            ]
            .into_iter()
            .collect(),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_script_item_access_item"
                && diagnostic.subject == "Start:GateScript:3"
                && diagnostic.message.contains("pass")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_script_item_access_item"
                && diagnostic.subject == "Start:CopycatScript:8"
                && diagnostic.message.contains("lost_item")
        }));
    }

    #[test]
    fn verifier_rejects_malformed_script_field_pickups_without_coercion() {
        let mut module = test_map_module("Start", "START_MAP", None);
        module.script_field_pickups = vec![
            ScriptFieldPickup {
                command: "itemball".to_string(),
                item_id: Some("potion".to_string()),
                quantity: 1,
                event_flag: Some("EVENT_ROUTE_29_POTION".to_string()),
                fruit_tree_id: None,
                source_script: "Route29Potion".to_string(),
                command_index: 0,
            },
            ScriptFieldPickup {
                command: "hiddenitem".to_string(),
                item_id: Some("POTION".to_string()),
                quantity: 0,
                event_flag: Some("-1".to_string()),
                fruit_tree_id: None,
                source_script: "HiddenPotion".to_string(),
                command_index: 0,
            },
            ScriptFieldPickup {
                command: "fruittree".to_string(),
                item_id: None,
                quantity: 1,
                event_flag: None,
                fruit_tree_id: Some(String::new()),
                source_script: "FruitTree".to_string(),
                command_index: 0,
            },
            ScriptFieldPickup {
                command: "ITEMBALL".to_string(),
                item_id: Some("POTION".to_string()),
                quantity: 1,
                event_flag: Some("EVENT_ROUTE_29_POTION".to_string()),
                fruit_tree_id: None,
                source_script: "UppercasePickup".to_string(),
                command_index: 0,
            },
        ];
        let data = GameDataSet {
            maps: [("Start".to_string(), module)].into_iter().collect(),
            items: [("POTION".to_string(), test_item("POTION"))]
                .into_iter()
                .collect(),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        for expected in [
            "unknown_script_field_pickup_item",
            "script_field_pickup_invalid_quantity",
            "script_field_pickup_uncollectible_event",
            "script_field_pickup_empty_fruit_tree",
            "unknown_script_field_fruit_tree",
            "unknown_script_field_pickup_command",
        ] {
            assert!(
                report
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.code == expected),
                "missing diagnostic {expected}: {:?}",
                report.diagnostics
            );
        }
    }

    #[test]
    fn verifier_rejects_unknown_fruit_tree_catalog_items_without_case_coercion() {
        let data = GameDataSet {
            fruit_trees: FruitTreeCatalog(
                [("FRUITTREE_ROUTE_29".to_string(), "berry".to_string())]
                    .into_iter()
                    .collect(),
            ),
            items: [("BERRY".to_string(), test_item("BERRY"))]
                .into_iter()
                .collect(),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_fruit_tree_item"
                && diagnostic.subject == "fruit_trees:FRUITTREE_ROUTE_29"
                && diagnostic.message.contains("berry")
        }));
    }

    #[test]
    fn verifier_rejects_referenced_fruit_tree_without_catalog() {
        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.script_field_pickups = vec![ScriptFieldPickup {
            command: "fruittree".to_string(),
            item_id: None,
            quantity: 1,
            event_flag: None,
            fruit_tree_id: Some("FRUITTREE_ROUTE_29".to_string()),
            source_script: "Route29FruitTree".to_string(),
            command_index: 0,
        }];
        let data = GameDataSet {
            maps: [("Route29".to_string(), module)].into_iter().collect(),
            items: [("BERRY".to_string(), test_item("BERRY"))]
                .into_iter()
                .collect(),
            fruit_trees: FruitTreeCatalog::default(),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_script_field_fruit_tree"
                && diagnostic.subject == "Route29:Route29FruitTree:0"
                && diagnostic.message.contains("FRUITTREE_ROUTE_29")
        }));
    }

    #[test]
    fn verifier_rejects_unresolved_script_economy_constants_without_case_coercion() {
        let mut module = test_map_module("Start", "START_MAP", None);
        module.script_economy_commands = vec![ScriptEconomyCommand {
            command: "checkmoney".to_string(),
            account: Some("YOUR_MONEY".to_string()),
            amount_tokens: vec!["route43gate_toll".to_string()],
            source_script: "TollScript".to_string(),
            command_index: 2,
        }];
        let data = GameDataSet {
            maps: [("Start".to_string(), module)].into_iter().collect(),
            currency_constants: CurrencyCatalog(
                [("ROUTE43GATE_TOLL".to_string(), 1_000)]
                    .into_iter()
                    .collect(),
            ),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unresolved_script_currency_amount"
                && diagnostic.subject == "Start:TollScript:2"
                && diagnostic.message.contains("route43gate_toll")
        }));
    }

    #[test]
    fn verifier_rejects_money_mutation_without_pack_max_money() {
        let mut module = test_map_module("Start", "START_MAP", None);
        module.script_economy_commands = vec![ScriptEconomyCommand {
            command: "takemoney".to_string(),
            account: Some("YOUR_MONEY".to_string()),
            amount_tokens: vec!["PRICE".to_string()],
            source_script: "BuyScript".to_string(),
            command_index: 4,
        }];
        let data = GameDataSet {
            maps: [("Start".to_string(), module)].into_iter().collect(),
            currency_constants: CurrencyCatalog([("PRICE".to_string(), 500)].into_iter().collect()),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "missing_script_money_cap"
                && diagnostic.subject == "Start:BuyScript:4"
                && diagnostic.message.contains("MAX_MONEY")
        }));
    }

    #[test]
    fn verifier_rejects_coin_mutation_without_pack_max_coins() {
        let mut module = test_map_module("Start", "START_MAP", None);
        module.script_economy_commands = vec![ScriptEconomyCommand {
            command: "givecoins".to_string(),
            account: None,
            amount_tokens: vec!["PRICE".to_string()],
            source_script: "PrizeScript".to_string(),
            command_index: 5,
        }];
        let data = GameDataSet {
            maps: [("Start".to_string(), module)].into_iter().collect(),
            currency_constants: CurrencyCatalog([("PRICE".to_string(), 500)].into_iter().collect()),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "missing_script_coin_cap"
                && diagnostic.subject == "Start:PrizeScript:5"
                && diagnostic.message.contains("MAX_COINS")
        }));
    }

    #[test]
    fn verifier_rejects_unknown_gift_pokemon_facts_without_case_coercion() {
        let mut module = test_map_module("Start", "START_MAP", None);
        module.gift_pokemon_scripts = vec![GiftPokemonScript {
            species_id: "cyndaquil".to_string(),
            level_token: "5".to_string(),
            level: 5,
            held_item_id: Some("berry".to_string()),
            nickname_label: Some("giftstartername".to_string()),
            ot_label: Some(String::new()),
            source_script: "StarterScript".to_string(),
            command_index: 2,
            egg: false,
        }];
        module
            .scripts
            .insert("GiftStarterName".to_string(), Value::Array(Vec::new()));
        let data = GameDataSet {
            maps: [("Start".to_string(), module)].into_iter().collect(),
            pokemon: [("CYNDAQUIL".to_string(), species())].into_iter().collect(),
            items: [("BERRY".to_string(), test_item("BERRY"))]
                .into_iter()
                .collect(),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_gift_pokemon_species"
                && diagnostic.subject == "Start:StarterScript:2"
                && diagnostic.message.contains("cyndaquil")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_gift_pokemon_item"
                && diagnostic.subject == "Start:StarterScript:2"
                && diagnostic.message.contains("berry")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_gift_pokemon_label"
                && diagnostic.subject == "Start:StarterScript:2"
                && diagnostic.message.contains("giftstartername")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "empty_gift_pokemon_label"
                && diagnostic.subject == "Start:StarterScript:2"
        }));
    }

    #[test]
    fn verifier_rejects_malformed_script_flag_commands_without_normalization() {
        let mut module = test_map_module("Start", "START_MAP", None);
        module.script_flag_commands = vec![
            ScriptFlagCommand {
                command: "SET_EVENT".to_string(),
                flag_id: "EVENT_ROUTE_29_POTION".to_string(),
                source_script: "RouteScript".to_string(),
                command_index: 4,
            },
            ScriptFlagCommand {
                command: "setevent".to_string(),
                flag_id: String::new(),
                source_script: "RouteScript".to_string(),
                command_index: 5,
            },
        ];
        let data = GameDataSet {
            maps: [("Start".to_string(), module)].into_iter().collect(),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_script_flag_command"
                && diagnostic.subject == "Start:RouteScript:4"
                && diagnostic.message.contains("SET_EVENT")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "empty_script_flag_id" && diagnostic.subject == "Start:RouteScript:5"
        }));
    }

    #[test]
    fn verifier_rejects_unknown_script_warp_targets_without_normalization() {
        let mut start = test_map_module("Start", "START_MAP", None);
        start.script_map_commands = vec![
            ScriptMapCommand {
                command: "warp".to_string(),
                target_map: Some("destination".to_string()),
                x: Some(4),
                y: Some(5),
                facing: None,
                map_setup: None,
                source_script: "WarpScript".to_string(),
                command_index: 2,
            },
            ScriptMapCommand {
                command: "warpfacing".to_string(),
                target_map: Some("Destination".to_string()),
                x: Some(4),
                y: Some(5),
                facing: Some("up".to_string()),
                map_setup: None,
                source_script: "WarpScript".to_string(),
                command_index: 3,
            },
            ScriptMapCommand {
                command: "warp".to_string(),
                target_map: Some("NONE".to_string()),
                x: Some(1),
                y: Some(0),
                facing: None,
                map_setup: None,
                source_script: "WarpScript".to_string(),
                command_index: 4,
            },
        ];
        let destination = test_map_module("Destination", "DESTINATION", None);
        let data = GameDataSet {
            maps: [
                ("Start".to_string(), start),
                ("Destination".to_string(), destination),
            ]
            .into_iter()
            .collect(),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_script_warp_map"
                && diagnostic.subject == "Start:WarpScript:2"
                && diagnostic.message.contains("destination")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_script_warp_facing"
                && diagnostic.subject == "Start:WarpScript:3"
                && diagnostic.message.contains("up")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "malformed_script_no_warp_sentinel"
                && diagnostic.subject == "Start:WarpScript:4"
        }));
    }

    #[test]
    fn verifier_rejects_unknown_script_text_labels_without_normalization() {
        let mut module = test_map_module("Start", "START_MAP", None);
        module.scripts.insert(
            "GreetingText".to_string(),
            serde_json::json!([
                {"command": "text", "args": "\"Hello.\""},
                {"command": "done", "args": []}
            ]),
        );
        module.script_text_commands = vec![
            ScriptTextCommand {
                command: "writetext".to_string(),
                text_label: Some("greetingtext".to_string()),
                source_script: "GreetingScript".to_string(),
                command_index: 2,
            },
            ScriptTextCommand {
                command: "waitbutton".to_string(),
                text_label: Some("GreetingText".to_string()),
                source_script: "GreetingScript".to_string(),
                command_index: 3,
            },
            ScriptTextCommand {
                command: "jumptext".to_string(),
                text_label: None,
                source_script: "GreetingScript".to_string(),
                command_index: 4,
            },
        ];
        let data = GameDataSet {
            maps: [("Start".to_string(), module)].into_iter().collect(),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_script_text_label"
                && diagnostic.subject == "Start:GreetingScript:2"
                && diagnostic.message.contains("greetingtext")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unexpected_script_text_label"
                && diagnostic.subject == "Start:GreetingScript:3"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "missing_script_text_label"
                && diagnostic.subject == "Start:GreetingScript:4"
        }));
    }

    #[test]
    fn verifier_rejects_malformed_script_variable_commands_without_normalization() {
        let mut module = test_map_module("Start", "START_MAP", None);
        module.script_variable_commands = vec![
            ScriptVariableCommand {
                command: "checktime".to_string(),
                target: None,
                value_tokens: vec!["night".to_string()],
                source_script: "VarScript".to_string(),
                command_index: 1,
            },
            ScriptVariableCommand {
                command: "readvar".to_string(),
                target: Some(String::new()),
                value_tokens: Vec::new(),
                source_script: "VarScript".to_string(),
                command_index: 2,
            },
            ScriptVariableCommand {
                command: "setval".to_string(),
                target: Some("VAR_BADGES".to_string()),
                value_tokens: vec!["7".to_string()],
                source_script: "VarScript".to_string(),
                command_index: 3,
            },
        ];
        let data = GameDataSet {
            maps: [("Start".to_string(), module)].into_iter().collect(),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        for index in [1, 2, 3] {
            let subject = format!("Start:VarScript:{index}");
            assert!(
                report.diagnostics.iter().any(|diagnostic| {
                    diagnostic.code == "invalid_script_variable_command"
                        && diagnostic.subject == subject
                }),
                "missing diagnostic for {subject}: {:?}",
                report.diagnostics
            );
        }
    }

    #[test]
    fn verifier_rejects_malformed_script_control_commands_without_target_fallbacks() {
        let mut module = test_map_module("Start", "START_MAP", None);
        module.scripts.insert(
            "MainScript".to_string(),
            serde_json::json!([
                {"command": "iftrue", "args": [".Done"]},
                {"command": "end", "args": []}
            ]),
        );
        module.script_control_commands = vec![
            ScriptControlCommand {
                command: "iftrue".to_string(),
                compare_value: Some("TRUE".to_string()),
                target_label: Some(".Done".to_string()),
                resolved_target_script: Some(".Done@MainScript".to_string()),
                source_script: "MainScript".to_string(),
                command_index: 0,
            },
            ScriptControlCommand {
                command: "ifequal".to_string(),
                compare_value: Some("TRUE".to_string()),
                target_label: Some(".missing".to_string()),
                resolved_target_script: Some(".missing@MainScript".to_string()),
                source_script: "MainScript".to_string(),
                command_index: 1,
            },
            ScriptControlCommand {
                command: "sjump".to_string(),
                compare_value: None,
                target_label: Some(".Done".to_string()),
                resolved_target_script: None,
                source_script: "MainScript".to_string(),
                command_index: 2,
            },
        ];
        let data = GameDataSet {
            maps: [("Start".to_string(), module)].into_iter().collect(),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_script_control_command"
                && diagnostic.subject == "Start:MainScript:0"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_script_control_target"
                && diagnostic.subject == "Start:MainScript:1"
                && diagnostic.message.contains(".missing@MainScript")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_script_control_command"
                && diagnostic.subject == "Start:MainScript:2"
        }));
    }

    #[test]
    fn verifier_rejects_unknown_script_scene_targets_without_normalization() {
        let mut module = test_map_module("Start", "START_MAP", None);
        module.scenes = MapSceneTable {
            scenes: vec![MapScene {
                scene_id: "SCENE_START_OPEN".to_string(),
                script_name: None,
            }],
        };
        module.script_scene_commands = vec![
            ScriptSceneCommand {
                command: "setscene".to_string(),
                map_id: None,
                scene_id: Some("scene_start_open".to_string()),
                source_script: "StartScript".to_string(),
                command_index: 2,
            },
            ScriptSceneCommand {
                command: "setmapscene".to_string(),
                map_id: Some("route_43".to_string()),
                scene_id: Some("0".to_string()),
                source_script: "StartScript".to_string(),
                command_index: 3,
            },
            ScriptSceneCommand {
                command: "setmapscene".to_string(),
                map_id: Some("Route43Gate".to_string()),
                scene_id: Some("0".to_string()),
                source_script: "StartScript".to_string(),
                command_index: 4,
            },
            ScriptSceneCommand {
                command: "setscene".to_string(),
                map_id: None,
                scene_id: Some("0".to_string()),
                source_script: "StartScript".to_string(),
                command_index: 5,
            },
        ];
        let mut target = test_map_module("Route43Gate", "ROUTE_43_GATE", None);
        target.scenes = MapSceneTable {
            scenes: vec![MapScene {
                scene_id: "SCENE_ROUTE43GATE_ROCKET_SHAKEDOWN".to_string(),
                script_name: None,
            }],
        };
        let data = GameDataSet {
            maps: [
                ("Start".to_string(), module),
                ("Route43Gate".to_string(), target),
            ]
            .into_iter()
            .collect(),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_script_scene_id"
                && diagnostic.subject == "Start:StartScript:2"
                && diagnostic.message.contains("scene_start_open")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_script_scene_map"
                && diagnostic.subject == "Start:StartScript:3"
                && diagnostic.message.contains("route_43")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_script_scene_map"
                && diagnostic.subject == "Start:StartScript:4"
                && diagnostic.message.contains("Route43Gate")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_script_scene_id"
                && diagnostic.subject == "Start:StartScript:5"
                && diagnostic.message.contains("0")
        }));
    }

    #[test]
    fn verifier_rejects_unknown_script_audio_ids_without_normalization() {
        let mut module = test_map_module("Start", "START_MAP", None);
        module.script_audio_commands = vec![
            ScriptAudioCommand {
                command: "playmusic".to_string(),
                audio_id: Some("music_route_29".to_string()),
                fade_frames: None,
                source_script: "AudioScript".to_string(),
                command_index: 1,
            },
            ScriptAudioCommand {
                command: "cry".to_string(),
                audio_id: Some("lugia".to_string()),
                fade_frames: None,
                source_script: "AudioScript".to_string(),
                command_index: 2,
            },
            ScriptAudioCommand {
                command: "cry".to_string(),
                audio_id: Some("LUGIA".to_string()),
                fade_frames: None,
                source_script: "AudioScript".to_string(),
                command_index: 3,
            },
            ScriptAudioCommand {
                command: "cry".to_string(),
                audio_id: Some("CHIKORITA".to_string()),
                fade_frames: None,
                source_script: "AudioScript".to_string(),
                command_index: 4,
            },
        ];
        let data = GameDataSet {
            maps: [("Start".to_string(), module)].into_iter().collect(),
            pokemon: [
                ("LUGIA".to_string(), species()),
                ("CHIKORITA".to_string(), species()),
            ]
            .into_iter()
            .collect(),
            pokemon_cries: [(
                "LUGIA".to_string(),
                PokemonCryMetadata {
                    cry: "CRY_LUGIA".to_string(),
                    pitch: 0,
                    length: 0,
                },
            )]
            .into_iter()
            .collect(),
            audio: vec![
                ModpackAudioAsset {
                    id: "MUSIC_ROUTE_29".to_string(),
                    path: "content-packs/test/music/MUSIC_ROUTE_29.mid".to_string(),
                    kind: ModpackAudioKind::Music,
                },
                ModpackAudioAsset {
                    id: "CRY_HO_OH".to_string(),
                    path: "content-packs/test/cries/CRY_HO_OH.mid".to_string(),
                    kind: ModpackAudioKind::Cry,
                },
            ],
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_script_music_id"
                && diagnostic.subject == "Start:AudioScript:1"
                && diagnostic.message.contains("music_route_29")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_script_cry_species"
                && diagnostic.subject == "Start:AudioScript:2"
                && diagnostic.message.contains("lugia")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_script_cry_audio"
                && diagnostic.subject == "Start:AudioScript:3"
                && diagnostic.message.contains("CRY_LUGIA")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "missing_script_cry_metadata"
                && diagnostic.subject == "Start:AudioScript:4"
                && diagnostic.message.contains("CHIKORITA")
        }));
    }

    #[test]
    fn verifier_requires_every_species_cry_metadata_and_declared_cry_audio() {
        let mut lugia = species();
        lugia.id = "LUGIA".to_string();
        lugia.tmhm_learnset.clear();
        let mut chikorita = species();
        chikorita.id = "CHIKORITA".to_string();
        chikorita.tmhm_learnset.clear();
        let data = GameDataSet {
            pokemon: [
                ("LUGIA".to_string(), lugia),
                ("CHIKORITA".to_string(), chikorita),
            ]
            .into_iter()
            .collect(),
            learnsets: [
                ("LUGIA".to_string(), Vec::new()),
                ("CHIKORITA".to_string(), Vec::new()),
            ]
            .into_iter()
            .collect(),
            evolutions: EvolutionTable(
                [
                    ("LUGIA".to_string(), Vec::new()),
                    ("CHIKORITA".to_string(), Vec::new()),
                ]
                .into_iter()
                .collect(),
            ),
            pokemon_cries: [(
                "LUGIA".to_string(),
                PokemonCryMetadata {
                    cry: "CRY_LUGIA".to_string(),
                    pitch: 0,
                    length: 0,
                },
            )]
            .into_iter()
            .collect(),
            audio: vec![ModpackAudioAsset {
                id: "CRY_HO_OH".to_string(),
                path: "content-packs/test/cries/CRY_HO_OH.mid".to_string(),
                kind: ModpackAudioKind::Cry,
            }],
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_species_cry_audio"
                && diagnostic.subject == "LUGIA"
                && diagnostic.message.contains("CRY_LUGIA")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "missing_species_cry_metadata" && diagnostic.subject == "CHIKORITA"
        }));
    }

    #[test]
    fn verifier_requires_script_audio_ids_declared_by_pack_not_path_aliases() {
        let mut module = test_map_module("Start", "START_MAP", None);
        module.script_audio_commands = vec![ScriptAudioCommand {
            command: "playmusic".to_string(),
            audio_id: Some("MUSIC_ROUTE_29".to_string()),
            fade_frames: None,
            source_script: "AudioScript".to_string(),
            command_index: 1,
        }];
        let data = GameDataSet {
            maps: [("Start".to_string(), module)].into_iter().collect(),
            audio: vec![ModpackAudioAsset {
                id: "MUSIC_CUSTOM_ROUTE29".to_string(),
                path: "content-packs/test/music/MUSIC_CUSTOM_ROUTE29.mid".to_string(),
                kind: ModpackAudioKind::Music,
            }],
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_script_music_id"
                && diagnostic.subject == "Start:AudioScript:1"
                && diagnostic.message.contains("MUSIC_ROUTE_29")
        }));
    }

    #[test]
    fn verifier_requires_map_music_declared_as_exact_music_asset() {
        let mut module = test_map_module("Start", "START_MAP", None);
        module.attributes.music = Some("MUSIC_ROUTE_29".to_string());
        let wrong_kind_module = {
            let mut module = test_map_module("WrongKind", "WRONG_KIND", None);
            module.attributes.music = Some("SFX_TACKLE".to_string());
            module
        };
        let data = GameDataSet {
            maps: [
                ("Start".to_string(), module),
                ("WrongKind".to_string(), wrong_kind_module),
            ]
            .into_iter()
            .collect(),
            audio: vec![ModpackAudioAsset {
                id: "SFX_TACKLE".to_string(),
                path: "content-packs/test/sfx/SFX_TACKLE.mid".to_string(),
                kind: ModpackAudioKind::SoundEffect,
            }],
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_map_music_id"
                && diagnostic.subject == "Start"
                && diagnostic.message.contains("MUSIC_ROUTE_29")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_map_music_id"
                && diagnostic.subject == "WrongKind"
                && diagnostic.message.contains("SFX_TACKLE")
        }));
    }

    #[test]
    fn verifier_requires_music_none_to_be_declared_by_pack() {
        let mut module = test_map_module("Start", "START_MAP", None);
        module.script_audio_commands = vec![ScriptAudioCommand {
            command: "musicfadeout".to_string(),
            audio_id: Some("MUSIC_NONE".to_string()),
            fade_frames: Some(2),
            source_script: "FadeScript".to_string(),
            command_index: 1,
        }];
        let data = GameDataSet {
            maps: [("Start".to_string(), module)].into_iter().collect(),
            special_routines: ["FadeOutMusic".to_string()].into_iter().collect(),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_script_music_id"
                && diagnostic.subject == "Start:FadeScript:1"
                && diagnostic.message.contains("MUSIC_NONE")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "missing_special_routine_music_id"
                && diagnostic.subject == "special_routines:FadeOutMusic"
                && diagnostic.message.contains("MUSIC_NONE")
        }));
    }

    #[test]
    fn verifier_rejects_init_roam_mons_without_roaming_pokemon_pack_data() {
        let data = GameDataSet {
            special_routines: BTreeSet::from(["InitRoamMons".to_string()]),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "missing_roaming_pokemon_definitions"
                && diagnostic.subject == "special_routines:InitRoamMons"
                && diagnostic.message.contains("roaming Pokemon")
        }));
    }

    #[test]
    fn verifier_rejects_declared_special_routine_unknown_to_rust_runtime() {
        let data = GameDataSet {
            special_routines: BTreeSet::from(["ModpackOnlyRoutine".to_string()]),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_declared_special_routine"
                && diagnostic.subject == "special_routines:ModpackOnlyRoutine"
                && diagnostic
                    .message
                    .contains("is not implemented by the Rust runtime")
        }));
    }

    #[test]
    fn verifier_rejects_buena_prize_without_buena_prize_pack_data() {
        let data = GameDataSet {
            special_routines: BTreeSet::from(["BuenaPrize".to_string()]),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "missing_buena_prize_definitions"
                && diagnostic.subject == "special_routines:BuenaPrize"
                && diagnostic.message.contains("Buena prize")
        }));
    }

    #[test]
    fn verifier_rejects_buenas_password_without_buena_password_category_pack_data() {
        let data = GameDataSet {
            special_routines: BTreeSet::from(["BuenasPassword".to_string()]),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "missing_buena_password_categories"
                && diagnostic.subject == "special_routines:BuenasPassword"
                && diagnostic.message.contains("Buena password")
        }));
    }

    #[test]
    fn verifier_rejects_select_apricorn_without_kurt_apricorn_recipe_pack_data() {
        let data = GameDataSet {
            special_routines: BTreeSet::from(["SelectApricornForKurt".to_string()]),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "missing_kurt_apricorn_recipes"
                && diagnostic.subject == "special_routines:SelectApricornForKurt"
                && diagnostic.message.contains("Kurt apricorn")
        }));
    }

    #[test]
    fn verifier_rejects_shuckie_routines_without_shuckie_gift_pack_data() {
        let data = GameDataSet {
            special_routines: BTreeSet::from([
                "GiveShuckle".to_string(),
                "ReturnShuckie".to_string(),
            ]),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "missing_shuckie_gift"
                && diagnostic.subject == "special_routines:Shuckie"
                && diagnostic.message.contains("Shuckie gift")
        }));
    }

    #[test]
    fn verifier_rejects_give_dratini_without_dratini_move_sets_pack_data() {
        let data = GameDataSet {
            special_routines: BTreeSet::from(["GiveDratini".to_string()]),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "missing_dratini_move_sets"
                && diagnostic.subject == "special_routines:GiveDratini"
                && diagnostic.message.contains("Dratini move sets")
        }));
    }

    #[test]
    fn verifier_rejects_battle_tower_action_without_battle_tower_rules_pack_data() {
        let data = GameDataSet {
            special_routines: BTreeSet::from(["BattleTowerAction".to_string()]),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "missing_battle_tower_rules"
                && diagnostic.subject == "special_routines:BattleTowerRules"
                && diagnostic.message.contains("Battle Tower rules")
        }));
    }

    #[test]
    fn verifier_rejects_battle_tower_rule_check_without_battle_tower_rules_pack_data() {
        let data = GameDataSet {
            special_routines: BTreeSet::from(["CheckForBattleTowerRules".to_string()]),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "missing_battle_tower_rules"
                && diagnostic.subject == "special_routines:BattleTowerRules"
                && diagnostic.message.contains("Battle Tower rules")
        }));
    }

    #[test]
    fn verifier_rejects_prof_oaks_pc_without_oak_rating_pack_data() {
        let data = GameDataSet {
            special_routines: BTreeSet::from(["ProfOaksPCBoot".to_string()]),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "missing_oak_rating_table"
                && diagnostic.subject == "special_routines:ProfOaksPCBoot"
                && diagnostic.message.contains("Oak rating")
        }));
    }

    #[test]
    fn verifier_rejects_oak_ratings_without_case_or_order_coercion() {
        let mut chikorita = species();
        chikorita.id = "CHIKORITA".to_string();
        let mut cyndaquil = species();
        cyndaquil.id = "CYNDAQUIL".to_string();
        let data = GameDataSet {
            pokemon: [
                ("CHIKORITA".to_string(), chikorita),
                ("CYNDAQUIL".to_string(), cyndaquil),
            ]
            .into_iter()
            .collect(),
            oak_ratings: vec![
                OakRatingEntry {
                    caught_count_limit: 1,
                    fanfare: " SFX_DEX_FANFARE_LESS_THAN_20".to_string(),
                    text_label: "OakRating01".to_string(),
                },
                OakRatingEntry {
                    caught_count_limit: 1,
                    fanfare: "SFX_DEX_FANFARE_LESS_THAN_20".to_string(),
                    text_label: "".to_string(),
                },
            ],
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_oak_rating_fanfare" && diagnostic.subject == "oak_ratings:0"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_oak_rating_text_label"
                && diagnostic.subject == "oak_ratings:1"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_oak_rating_order" && diagnostic.subject == "oak_ratings:1"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "incomplete_oak_rating_coverage"
                && diagnostic.subject == "oak_ratings"
        }));
    }

    #[test]
    fn verifier_rejects_battle_tower_banned_species_without_case_coercion() {
        let mut mewtwo = species();
        mewtwo.id = "MEWTWO".to_string();
        let data = GameDataSet {
            pokemon: [("MEWTWO".to_string(), mewtwo)].into_iter().collect(),
            battle_tower_rules: Some(BattleTowerRules {
                banned_species: vec!["mewtwo".to_string(), " MEWTWO".to_string()],
                required_party_count: 0,
                challenge_streak_length: 0,
                minimum_level_group: 2,
                maximum_level_group: 1,
                level_group_size: 0,
                party_count_failure_text: " OnlyThreeMonMayBeEnteredText".to_string(),
                duplicate_species_failure_text: "TheMonMustAllBeDifferentKindsText".to_string(),
                duplicate_held_item_failure_text: "TheMonMustNotHoldTheSameItemsText".to_string(),
                egg_failure_text: "".to_string(),
            }),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_battle_tower_banned_species"
                && diagnostic.subject == "battle_tower_rules:banned_species:0"
                && diagnostic.message.contains("mewtwo")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_battle_tower_banned_species"
                && diagnostic.subject == "battle_tower_rules:banned_species:1"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_battle_tower_required_party_count"
                && diagnostic.subject == "battle_tower_rules:required_party_count"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_battle_tower_challenge_streak_length"
                && diagnostic.subject == "battle_tower_rules:challengeStreakLength"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_battle_tower_level_group_size"
                && diagnostic.subject == "battle_tower_rules:levelGroupSize"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_battle_tower_level_group_range"
                && diagnostic.subject == "battle_tower_rules:levelGroupRange"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_battle_tower_failure_text"
                && diagnostic.subject == "battle_tower_rules:partyCountFailureText"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_battle_tower_failure_text"
                && diagnostic.subject == "battle_tower_rules:eggFailureText"
        }));
    }

    #[test]
    fn verifier_rejects_give_odd_egg_without_odd_egg_pack_data() {
        let data = GameDataSet {
            special_routines: BTreeSet::from(["GiveOddEgg".to_string()]),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "missing_odd_egg_definitions"
                && diagnostic.subject == "special_routines:GiveOddEgg"
                && diagnostic.message.contains("Odd Egg definitions")
        }));
    }

    #[test]
    fn verifier_rejects_odd_egg_species_and_moves_without_case_coercion() {
        let mut cleffa = species();
        cleffa.id = "CLEFFA".to_string();
        let data = GameDataSet {
            pokemon: [("CLEFFA".to_string(), cleffa)].into_iter().collect(),
            moves: [("POUND".to_string(), test_move("POUND"))]
                .into_iter()
                .collect(),
            odd_egg_definitions: vec![OddEggDefinition {
                species: "cleffa".to_string(),
                moves: vec!["pound".to_string()],
                original_trainer_id: 768,
                dvs: [2, 10, 10, 10],
                probability: 100,
                level: 5,
                experience: 125,
                hatch_cycles: 20,
                nickname: "EGG".to_string(),
                original_trainer_name: "ODD".to_string(),
            }],
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_odd_egg_species"
                && diagnostic.subject == "odd_egg_definitions:0"
                && diagnostic.message.contains("cleffa")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_odd_egg_move"
                && diagnostic.subject == "odd_egg_definitions:0:move:0"
                && diagnostic.message.contains("pound")
        }));
    }

    #[test]
    fn verifier_rejects_out_of_bounds_script_block_changes_without_resizing() {
        let mut module = test_map_module("Start", "START_MAP", None);
        module.attributes.width = 2;
        module.attributes.height = 2;
        module.blocks = vec![1, 2, 3, 4];
        module.script_block_changes = vec![ScriptBlockChange {
            x: 4,
            y: 1,
            block_id: 0x2e,
            source_script: "DoorScript".to_string(),
            command_index: 6,
        }];
        let data = GameDataSet {
            maps: [("Start".to_string(), module)].into_iter().collect(),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "script_block_change_out_of_bounds"
                && diagnostic.subject == "Start:DoorScript:6"
                && diagnostic.message.contains("(4, 1)")
        }));
    }

    #[test]
    fn verifier_rejects_unknown_runtime_special_without_coercion() {
        let mut module = test_map_module("Start", "START_MAP", None);
        module.script_runtime_commands = vec![ScriptRuntimeCommand {
            command: "special".to_string(),
            args: vec!["fadeoutmusic".to_string()],
            source_script: "StartScript".to_string(),
            command_index: 0,
        }];
        let data = GameDataSet {
            maps: [("Start".to_string(), module)].into_iter().collect(),
            special_routines: BTreeSet::from(["FadeOutMusic".to_string()]),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.has_errors());
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_script_special_routine"
                && diagnostic.subject == "Start:StartScript:0"
                && diagnostic.message.contains("fadeoutmusic")
        }));
    }

    #[test]
    fn verifier_rejects_malformed_text_bodies_without_coercion() {
        let mut module = test_map_module("Start", "START_MAP", None);
        module.script_text_bodies.insert(
            "GreetingText".to_string(),
            ScriptTextBody {
                label: "greetingtext".to_string(),
                commands: vec![
                    ScriptTextBodyCommand {
                        command: "Text".to_string(),
                        args: vec!["\"Hi!\"".to_string()],
                        command_index: 0,
                    },
                    ScriptTextBodyCommand {
                        command: "done".to_string(),
                        args: vec!["\"extra\"".to_string()],
                        command_index: 1,
                    },
                ],
            },
        );
        let data = GameDataSet {
            maps: [("Start".to_string(), module)].into_iter().collect(),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.has_errors());
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "script_text_body_label_mismatch"
                && diagnostic.subject == "Start:GreetingText"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_script_text_body_command"
                && diagnostic.subject == "Start:GreetingText:0"
                && diagnostic.message.contains("Text")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "malformed_script_text_body_command"
                && diagnostic.subject == "Start:GreetingText:1"
                && diagnostic.message.contains("done expects 0 args")
        }));
    }

    #[test]
    fn verifier_rejects_malformed_map_section_commands_without_coercion() {
        let mut module = test_map_module("Start", "START_MAP", None);
        module.scripts = BTreeMap::from([("KnownScript".to_string(), Value::Array(Vec::new()))]);
        module.map_script_section_commands = vec![
            MapScriptSectionCommand {
                command: "scene_script".to_string(),
                args: vec!["missing_script".to_string()],
                command_index: 1,
            },
            MapScriptSectionCommand {
                command: "callback".to_string(),
                args: vec![
                    "MAPCALLBACK_OBJECTS".to_string(),
                    "MissingCallback".to_string(),
                ],
                command_index: 2,
            },
        ];
        module.map_event_section_commands = vec![
            MapEventSectionCommand {
                command: "warp_event".to_string(),
                args: vec!["1".to_string(), "2".to_string()],
                command_index: 3,
            },
            MapEventSectionCommand {
                command: "bg_event".to_string(),
                args: vec![
                    "1".to_string(),
                    "2".to_string(),
                    "BGEVENT_READ".to_string(),
                    "MissingSign".to_string(),
                ],
                command_index: 4,
            },
            MapEventSectionCommand {
                command: "object_event".to_string(),
                args: vec![
                    "0".to_string(),
                    "0".to_string(),
                    "SPRITE_MON".to_string(),
                    "SPRITEMOVEDATA_STANDING_DOWN".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "-1".to_string(),
                    "-1".to_string(),
                    "PAL_NPC_RED".to_string(),
                    "OBJECTTYPE_SCRIPT".to_string(),
                    "0".to_string(),
                    "MissingObjectScript".to_string(),
                    "-1".to_string(),
                ],
                command_index: 5,
            },
        ];
        let data = GameDataSet {
            maps: [("Start".to_string(), module)].into_iter().collect(),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        for expected in [
            "unknown_map_scene_script",
            "unknown_map_callback_script",
            "malformed_map_event_section_command",
            "unknown_map_event_script",
            "unknown_map_object_event_script",
        ] {
            assert!(
                report
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.code == expected),
                "missing diagnostic {expected}: {:?}",
                report.diagnostics
            );
        }
    }

    #[test]
    fn verifier_rejects_unknown_small_runtime_references_without_coercion() {
        let mut module = test_map_module("Start", "START_MAP", None);
        module.script_runtime_commands = vec![
            ScriptRuntimeCommand {
                command: "addcellnum".to_string(),
                args: vec!["phone_elm".to_string()],
                source_script: "PhoneScript".to_string(),
                command_index: 0,
            },
            ScriptRuntimeCommand {
                command: "specialphonecall".to_string(),
                args: vec!["specialcall_masterball".to_string()],
                source_script: "PhoneScript".to_string(),
                command_index: 1,
            },
            ScriptRuntimeCommand {
                command: "checkpoke".to_string(),
                args: vec!["pikachu".to_string()],
                source_script: "SpeciesScript".to_string(),
                command_index: 2,
            },
            ScriptRuntimeCommand {
                command: "trade".to_string(),
                args: vec!["npc_trade_mike".to_string()],
                source_script: "TradeScript".to_string(),
                command_index: 3,
            },
            ScriptRuntimeCommand {
                command: "callasm".to_string(),
                args: vec![".missing".to_string()],
                source_script: "AsmScript".to_string(),
                command_index: 4,
            },
        ];
        module.scripts = BTreeMap::from([("AsmScript".to_string(), Value::Array(Vec::new()))]);
        let data = GameDataSet {
            maps: [("Start".to_string(), module)].into_iter().collect(),
            phone_contacts: PhoneContactCatalog(BTreeMap::from([(
                "PHONE_ELM".to_string(),
                PhoneContactRecord {
                    contact_id: "PHONE_ELM".to_string(),
                    trainer_class: Some("TRAINER_NONE".to_string()),
                    trainer_label: Some("PHONECONTACT_ELM".to_string()),
                    lines: vec!["ELM:".to_string()],
                    primary_label: "ELM".to_string(),
                    map_constant: Some("ELMS_LAB".to_string()),
                    callee_time_mask: 7,
                    callee_script: Some("ElmPhoneCalleeScript".to_string()),
                    caller_time_mask: 0,
                    caller_script: None,
                },
            )])),
            special_phone_calls: BTreeSet::from(["SPECIALCALL_MASTERBALL".to_string()]),
            npc_trades: BTreeSet::from(["NPC_TRADE_MIKE".to_string()]),
            pokemon: [("PIKACHU".to_string(), species())].into_iter().collect(),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        for expected in [
            "unknown_script_addcellnum_contact",
            "unknown_script_special_phone_call",
            "unknown_script_species_runtime_command",
            "unknown_script_npc_trade",
            "unknown_script_runtime_target",
        ] {
            assert!(
                report
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.code == expected),
                "missing diagnostic {expected}: {:?}",
                report.diagnostics
            );
        }
    }

    #[test]
    fn verifier_rejects_unknown_phone_contact_maps_without_case_coercion() {
        fn phone_contact(contact_id: &str, map_constant: Option<&str>) -> PhoneContactRecord {
            PhoneContactRecord {
                contact_id: contact_id.to_string(),
                trainer_class: Some("TRAINER_NONE".to_string()),
                trainer_label: Some(format!("PHONECONTACT_{contact_id}")),
                lines: vec![format!("{contact_id}:")],
                primary_label: contact_id.to_string(),
                map_constant: map_constant.map(str::to_string),
                callee_time_mask: 7,
                callee_script: None,
                caller_time_mask: 0,
                caller_script: None,
            }
        }

        let mut empty_lines = phone_contact("PHONE_LINES", None);
        empty_lines.lines = vec![String::new()];
        let mut mismatch = phone_contact("PHONE_MISMATCH", None);
        mismatch.primary_label = "OTHER_LABEL".to_string();
        let data = GameDataSet {
            maps: [(
                "ElmsLab".to_string(),
                test_map_module("ElmsLab", "ELMS_LAB", None),
            )]
            .into_iter()
            .collect(),
            phone_contacts: PhoneContactCatalog(BTreeMap::from([
                (
                    "PHONE_ELM".to_string(),
                    phone_contact("PHONE_ELM", Some("ELMS_LAB")),
                ),
                (
                    "PHONE_CASE".to_string(),
                    phone_contact("PHONE_CASE", Some("elms_lab")),
                ),
                (
                    "PHONE_EMPTY".to_string(),
                    phone_contact("PHONE_EMPTY", Some("")),
                ),
                ("PHONE_LINES".to_string(), empty_lines),
                ("PHONE_MISMATCH".to_string(), mismatch),
            ])),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_phone_contact_map"
                && diagnostic.subject == "phone_contacts:PHONE_CASE"
                && diagnostic.message.contains("elms_lab")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "empty_phone_contact_map"
                && diagnostic.subject == "phone_contacts:PHONE_EMPTY"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_phone_contact_lines"
                && diagnostic.subject == "phone_contacts:PHONE_LINES"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "phone_contact_primary_label_mismatch"
                && diagnostic.subject == "phone_contacts:PHONE_MISMATCH"
        }));
        assert!(!report.diagnostics.iter().any(|diagnostic| {
            (diagnostic.code == "unknown_phone_contact_map"
                || diagnostic.code == "empty_phone_contact_map"
                || diagnostic.code == "invalid_phone_contact_lines"
                || diagnostic.code == "phone_contact_primary_label_mismatch")
                && diagnostic.subject == "phone_contacts:PHONE_ELM"
        }));
    }

    #[test]
    fn verifier_rejects_invalid_script_object_commands_without_coercion() {
        let mut module = test_map_module("Start", "START_MAP", None);
        module.objects = vec![
            test_object("START_RIVAL", "EVENT_START_RIVAL", 1, 1),
            test_object("START_ALWAYS_VISIBLE", "0", 2, 1),
        ];
        module.script_object_commands = vec![
            ScriptObjectCommand {
                command: "disappear".to_string(),
                object_id: Some("start_rival".to_string()),
                target_object_id: None,
                x: None,
                y: None,
                direction: None,
                movement: None,
                emote: None,
                duration: None,
                source_script: "LowercaseScript".to_string(),
                command_index: 4,
            },
            ScriptObjectCommand {
                command: "appear".to_string(),
                object_id: Some("START_ALWAYS_VISIBLE".to_string()),
                target_object_id: None,
                x: None,
                y: None,
                direction: None,
                movement: None,
                emote: None,
                duration: None,
                source_script: "UnhideableScript".to_string(),
                command_index: 7,
            },
            ScriptObjectCommand {
                command: "applymovement".to_string(),
                object_id: Some("START_RIVAL".to_string()),
                target_object_id: None,
                x: None,
                y: None,
                direction: None,
                movement: Some("MissingMovement".to_string()),
                emote: None,
                duration: None,
                source_script: "MovementScript".to_string(),
                command_index: 9,
            },
            ScriptObjectCommand {
                command: "follow".to_string(),
                object_id: Some("START_RIVAL".to_string()),
                target_object_id: Some("start_player".to_string()),
                x: None,
                y: None,
                direction: None,
                movement: None,
                emote: None,
                duration: None,
                source_script: "FollowScript".to_string(),
                command_index: 11,
            },
        ];
        let data = GameDataSet {
            maps: [("Start".to_string(), module)].into_iter().collect(),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_script_object_id"
                && diagnostic.subject == "Start:LowercaseScript:4"
                && diagnostic.message.contains("start_rival")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "script_object_unhideable"
                && diagnostic.subject == "Start:UnhideableScript:7"
                && diagnostic.message.contains("START_ALWAYS_VISIBLE")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_script_movement"
                && diagnostic.subject == "Start:MovementScript:9"
                && diagnostic.message.contains("MissingMovement")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_script_object_id"
                && diagnostic.subject == "Start:FollowScript:11"
                && diagnostic.message.contains("start_player")
        }));
    }

    #[test]
    fn verifier_accepts_temporary_script_objects_and_last_talked_operand() {
        let mut module = test_map_module("CeladonGameCorner", "CELADON_GAME_CORNER", None);
        module.objects = vec![test_object("CELADONGAMECORNER_FISHER", "-1", 1, 1)];
        module.script_object_commands = vec![
            ScriptObjectCommand {
                command: "disappear".to_string(),
                object_id: Some("CELADONGAMECORNER_FISHER".to_string()),
                target_object_id: None,
                x: None,
                y: None,
                direction: None,
                movement: None,
                emote: None,
                duration: None,
                source_script: "TemporaryScript".to_string(),
                command_index: 1,
            },
            ScriptObjectCommand {
                command: "turnobject".to_string(),
                object_id: Some("LAST_TALKED".to_string()),
                target_object_id: None,
                x: None,
                y: None,
                direction: Some("LEFT".to_string()),
                movement: None,
                emote: None,
                duration: None,
                source_script: "LastTalkedScript".to_string(),
                command_index: 2,
            },
        ];
        let data = GameDataSet {
            maps: [("CeladonGameCorner".to_string(), module)]
                .into_iter()
                .collect(),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(
            !report.diagnostics.iter().any(|diagnostic| {
                diagnostic.code == "script_object_unhideable"
                    || diagnostic.code == "unknown_script_object_id"
            }),
            "{:?}",
            report.diagnostics
        );
    }

    #[test]
    fn verifier_accepts_script_economy_commands_with_exact_pack_constants() {
        let mut module = test_map_module("Start", "START_MAP", None);
        module.script_economy_commands = vec![
            ScriptEconomyCommand {
                command: "checkmoney".to_string(),
                account: Some("YOUR_MONEY".to_string()),
                amount_tokens: vec!["ROUTE43GATE_TOLL - 1".to_string()],
                source_script: "TollScript".to_string(),
                command_index: 2,
            },
            ScriptEconomyCommand {
                command: "takecoins".to_string(),
                account: None,
                amount_tokens: vec!["MAX_COINS".to_string(), "-".to_string(), "1".to_string()],
                source_script: "PrizeScript".to_string(),
                command_index: 8,
            },
        ];
        let data = GameDataSet {
            maps: [("Start".to_string(), module)].into_iter().collect(),
            currency_constants: CurrencyCatalog(
                [
                    ("ROUTE43GATE_TOLL".to_string(), 1_000),
                    ("MAX_COINS".to_string(), 9_999),
                ]
                .into_iter()
                .collect(),
            ),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(
            !report.diagnostics.iter().any(|diagnostic| {
                diagnostic.code == "unresolved_script_currency_amount"
                    || diagnostic.code == "unknown_script_money_account"
            }),
            "{:?}",
            report.diagnostics
        );
    }

    #[test]
    fn modpack_tmhm_items_require_explicit_index_data() {
        let mut tm = test_item("TM_MUD_SLAP");
        tm.pocket = item_pocket("TM_HM");
        let manifest = ModpackManifest {
            payload: ModpackPayload {
                items: vec![tm],
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };
        let mut data = GameDataSet::default();

        let error = data
            .apply_modpack(&manifest)
            .expect_err("missing tmhm index rejected");

        assert!(
            error
                .to_string()
                .contains("TM/HM item 'TM_MUD_SLAP' must declare explicit tmhm_index"),
            "{error}"
        );
    }

    #[test]
    fn modpack_symbolic_tm_grants_validate_against_exact_item_data() {
        let mut tm = test_item("TM_MUD_SLAP");
        tm.pocket = item_pocket("TM_HM");
        tm.tmhm_index = Some(30);
        let mut module = test_map_module("VioletGym", "VIOLET_GYM", None);
        module.script_item_grants = vec![ScriptItemGrant {
            item_id: "TM_MUD_SLAP".to_string(),
            quantity: 1,
            source_script: "VioletGymFalknerScript".to_string(),
            command_index: 27,
            verbose: true,
        }];
        let data = GameDataSet {
            maps: [("VioletGym".to_string(), module)].into_iter().collect(),
            items: [("TM_MUD_SLAP".to_string(), tm)].into_iter().collect(),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(
            !report.diagnostics.iter().any(|diagnostic| {
                diagnostic.code == "unknown_script_item_grant_item"
                    || diagnostic.code == "unindexed_tmhm_item"
            }),
            "{:?}",
            report.diagnostics
        );
    }

    #[test]
    fn modpack_overlay_replaces_currency_constants_by_exact_id() {
        let mut data = GameDataSet {
            currency_constants: CurrencyCatalog(
                [("ROUTE43GATE_TOLL".to_string(), 500)]
                    .into_iter()
                    .collect(),
            ),
            ..GameDataSet::default()
        };
        let manifest = ModpackManifest {
            payload: ModpackPayload {
                currency_constants: CurrencyCatalog(
                    [
                        ("ROUTE43GATE_TOLL".to_string(), 1_000),
                        ("route43gate_toll".to_string(), 1),
                    ]
                    .into_iter()
                    .collect(),
                ),
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };

        data.apply_modpack(&manifest)
            .expect("apply currency constants");

        assert_eq!(data.currency_constants.get("ROUTE43GATE_TOLL"), Some(1_000));
        assert_eq!(data.currency_constants.get("route43gate_toll"), Some(1));
    }

    #[test]
    fn modpack_payload_replaces_fishing_catalog_as_definitive_data() {
        let catalog = FishingCatalog {
            groups: [(
                "FISHGROUP_NEW".to_string(),
                crystal_core::world::fishing::FishingGroup {
                    bite_threshold: crystal_core::world::fishing::threshold(50, true),
                    rod_tables: BTreeMap::new(),
                },
            )]
            .into_iter()
            .collect(),
            time_groups: Vec::new(),
            swarm_rules: Vec::new(),
            rod_items: Vec::new(),
        };
        let manifest = ModpackManifest {
            payload: ModpackPayload {
                fishing: catalog.clone(),
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };
        let mut data = GameDataSet::default();

        data.apply_modpack(&manifest)
            .expect("apply fishing catalog");

        assert_eq!(data.fishing, catalog);
    }

    #[test]
    fn verifier_rejects_unknown_fishing_facts_without_case_coercion() {
        let mut known_species = species();
        known_species.id = "MAGIKARP".to_string();
        known_species.tmhm_learnset.clear();
        let data = GameDataSet {
            pokemon: [("MAGIKARP".to_string(), known_species)]
                .into_iter()
                .collect(),
            map_attributes: [(
                "Lake".to_string(),
                MapAttributes {
                    tileset_name: "johto".to_string(),
                    border_block: 0,
                    width: 1,
                    height: 1,
                    connections: Vec::new(),
                    time_of_day: None,
                    phone_service: 0,
                    phone_flag: false,
                    environment: None,
                    location: None,
                    music: None,
                    palette: None,
                    fishing_group: Some("fishgroup_lake".to_string()),
                    map_constant: Some("LAKE".to_string()),
                    map_group_constant: None,
                    blocks_label: None,
                    map_scripts_label: None,
                    map_events_label: None,
                    connection_flags: None,
                },
            )]
            .into_iter()
            .collect(),
            fishing: FishingCatalog {
                groups: [(
                    "FISHGROUP_LAKE".to_string(),
                    crystal_core::world::fishing::FishingGroup {
                        bite_threshold: 128,
                        rod_tables: [(
                            "good_rod".to_string(),
                            crystal_core::world::fishing::RodTable {
                                slots: vec![
                                    crystal_core::world::fishing::FishingSlot {
                                        threshold: 255,
                                        species: Some("magikarp".to_string()),
                                        level: 10,
                                        time_group: None,
                                    },
                                    crystal_core::world::fishing::FishingSlot {
                                        threshold: 255,
                                        species: None,
                                        level: 0,
                                        time_group: Some(0),
                                    },
                                ],
                            },
                        )]
                        .into_iter()
                        .collect(),
                    },
                )]
                .into_iter()
                .collect(),
                time_groups: vec![crystal_core::world::fishing::TimeFishEntry {
                    day_species: "MAGIKARP".to_string(),
                    day_level: 10,
                    night_species: "staryu".to_string(),
                    night_level: 10,
                }],
                swarm_rules: vec![crystal_core::world::fishing::FishingSwarmRule {
                    daily_flag_bit: 8,
                    swarm: 1,
                    base_group: "fishgroup_lake".to_string(),
                    swarm_group: "FISHGROUP_MISSING".to_string(),
                }],
                rod_items: vec![
                    crystal_core::world::fishing::FishingRodItemRule {
                        item_id: "GOOD_ROD".to_string(),
                        rod: "good_rod".to_string(),
                    },
                    crystal_core::world::fishing::FishingRodItemRule {
                        item_id: "GOOD_ROD".to_string(),
                        rod: crystal_core::world::fishing::ROD_GOOD.to_string(),
                    },
                ],
            },
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        for expected in [
            "unknown_map_fishing_group",
            "unknown_fishing_rod",
            "unknown_fishing_species",
            "invalid_fishing_swarm_flag_bit",
            "unknown_fishing_swarm_base_group",
            "unknown_fishing_swarm_group",
            "duplicate_fishing_rod_item_id",
            "unknown_fishing_rod_item_rod",
            "unknown_fishing_rod_item_id",
        ] {
            assert!(
                report
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.code == expected),
                "missing diagnostic {expected}: {:?}",
                report.diagnostics
            );
        }
    }

    #[test]
    fn verifier_rejects_referenced_fishing_group_without_catalog() {
        let data = GameDataSet {
            map_attributes: [(
                "Lake".to_string(),
                MapAttributes {
                    tileset_name: "johto".to_string(),
                    border_block: 0,
                    width: 1,
                    height: 1,
                    connections: Vec::new(),
                    time_of_day: None,
                    phone_service: 0,
                    phone_flag: false,
                    environment: None,
                    location: None,
                    music: None,
                    palette: None,
                    fishing_group: Some("FISHGROUP_LAKE".to_string()),
                    map_constant: Some("LAKE".to_string()),
                    map_group_constant: None,
                    blocks_label: None,
                    map_scripts_label: None,
                    map_events_label: None,
                    connection_flags: None,
                },
            )]
            .into_iter()
            .collect(),
            fishing: FishingCatalog::default(),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "missing_fishing_catalog" && diagnostic.subject == "Lake"
        }));
    }

    #[test]
    fn verifier_builds_reachability_graph_and_rejects_unsolved_goals() {
        let mut known_species = species();
        known_species.tmhm_learnset.clear();
        let mut start = test_map_module("Start", "START_MAP", Some("Middle"));
        start.attributes.height = 2;
        start.blocks = vec![5, 1];
        let mut middle = test_map_module("Middle", "MIDDLE_MAP", None);
        middle.attributes.height = 2;
        middle.blocks = vec![1, 1];
        let data = GameDataSet {
            pokemon: [(known_species.id.clone(), known_species)]
                .into_iter()
                .collect(),
            moves: [("TACKLE".to_string(), test_move("TACKLE"))]
                .into_iter()
                .collect(),
            maps: [
                ("Start".to_string(), start),
                ("Middle".to_string(), middle),
                (
                    "Goal".to_string(),
                    test_map_module("Goal", "GOAL_MAP", None),
                ),
            ]
            .into_iter()
            .collect(),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules {
                start_maps: vec!["Start".to_string()],
                start_tiles: Vec::new(),
                goal_maps: vec!["Goal".to_string()],
                require_all_maps_reachable: true,
                require_walkable_maps: false,
                ..PlayabilityRules::default()
            },
        );

        assert_eq!(
            report.graph_edges,
            vec![PlayabilityGraphEdge {
                from: "Start".to_string(),
                to: "Middle".to_string(),
                kind: "connection".to_string(),
            }]
        );
        assert_eq!(
            report.reachable_maps,
            vec!["Middle".to_string(), "Start".to_string()]
        );
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unreachable_goal_map" && diagnostic.subject == "Goal"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unreachable_map" && diagnostic.subject == "Goal"
        }));
    }

    #[test]
    fn verifier_rejects_connection_that_exists_only_on_blocked_collision() {
        let mut known_species = species();
        known_species.tmhm_learnset.clear();
        let mut blocked_start = test_map_module("Start", "START_MAP", Some("Goal"));
        blocked_start.blocks = vec![5];
        let data = GameDataSet {
            pokemon: [(known_species.id.clone(), known_species)]
                .into_iter()
                .collect(),
            moves: [("TACKLE".to_string(), test_move("TACKLE"))]
                .into_iter()
                .collect(),
            maps: [
                ("Start".to_string(), blocked_start),
                (
                    "Goal".to_string(),
                    test_map_module("Goal", "GOAL_MAP", None),
                ),
            ]
            .into_iter()
            .collect(),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules {
                start_maps: vec!["Start".to_string()],
                start_tiles: Vec::new(),
                goal_maps: vec!["Goal".to_string()],
                require_all_maps_reachable: false,
                require_walkable_maps: false,
                ..PlayabilityRules::default()
            },
        );

        assert!(report.graph_edges.is_empty());
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unreachable_connection" && diagnostic.subject == "Start"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unreachable_goal_map" && diagnostic.subject == "Goal"
        }));
    }

    #[test]
    fn verifier_uses_explicit_start_tiles_instead_of_whole_start_map() {
        let mut known_species = species();
        known_species.tmhm_learnset.clear();
        let mut start = test_map_module("Start", "START_MAP", Some("Goal"));
        start.attributes.width = 3;
        start.attributes.height = 2;
        start.blocks = vec![1, 5, 5, 5, 5, 1];
        let mut goal = test_map_module("Goal", "GOAL_MAP", None);
        goal.attributes.height = 2;
        goal.blocks = vec![1, 1];
        let data = GameDataSet {
            pokemon: [(known_species.id.clone(), known_species)]
                .into_iter()
                .collect(),
            moves: [("TACKLE".to_string(), test_move("TACKLE"))]
                .into_iter()
                .collect(),
            maps: [("Start".to_string(), start), ("Goal".to_string(), goal)]
                .into_iter()
                .collect(),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules {
                start_maps: Vec::new(),
                start_tiles: vec![PlayabilityStart {
                    map: "Start".to_string(),
                    tile: TilePosition::new(0, 0),
                }],
                goal_maps: vec!["Goal".to_string()],
                require_all_maps_reachable: false,
                require_walkable_maps: false,
                ..PlayabilityRules::default()
            },
        );

        assert_eq!(report.reachable_maps, vec!["Start".to_string()]);
        assert!(report.graph_edges.iter().any(|edge| {
            edge.from == "Start" && edge.to == "Goal" && edge.kind == "connection"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unreachable_goal_map" && diagnostic.subject == "Goal"
        }));
    }

    #[test]
    fn verifier_rejects_unwalkable_explicit_start_tiles() {
        let mut known_species = species();
        known_species.tmhm_learnset.clear();
        let mut start = test_map_module("Start", "START_MAP", None);
        start.blocks = vec![5];
        let mut goal = test_map_module("Goal", "GOAL_MAP", None);
        goal.attributes.width = 2;
        goal.blocks = vec![1, 1];
        let data = GameDataSet {
            pokemon: [(known_species.id.clone(), known_species)]
                .into_iter()
                .collect(),
            moves: [("TACKLE".to_string(), test_move("TACKLE"))]
                .into_iter()
                .collect(),
            maps: [("Start".to_string(), start)].into_iter().collect(),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules {
                start_maps: Vec::new(),
                start_tiles: vec![PlayabilityStart {
                    map: "Start".to_string(),
                    tile: TilePosition::new(0, 0),
                }],
                goal_maps: vec!["Start".to_string()],
                require_all_maps_reachable: false,
                require_walkable_maps: false,
                ..PlayabilityRules::default()
            },
        );

        assert!(report.reachable_maps.is_empty());
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_start_tile" && diagnostic.subject == "Start"
        }));
    }

    #[test]
    fn verifier_solves_explicit_progression_event_and_item_goals() {
        let mut data = GameDataSet {
            maps: [(
                "Start".to_string(),
                test_map_module("Start", "START_MAP", None),
            )]
            .into_iter()
            .collect(),
            items: [(
                "KEY_CARD".to_string(),
                Item {
                    name: "Key Card".to_string(),
                    description: "Opens a required gate.".to_string(),
                    effect: "NONE".to_string(),
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
                    escape_rope_mode: None,
                    price: 0,
                    held_effect: "HELD_NONE".to_string(),
                    parameter: 0,
                    property: String::new(),
                    pocket: item_pocket("KEY_ITEM"),
                    field_menu: "ITEMMENU_NOUSE".to_string(),
                    field_usable: false,
                    battle_menu: "ITEMMENU_NOUSE".to_string(),
                    battle_usable: false,
                    script_name: "KEY_CARD".to_string(),
                    consumable: false,
                    tmhm_index: None,
                    tmhm_move: None,
                },
            )]
            .into_iter()
            .collect(),
            ..GameDataSet::default()
        };
        add_runtime_species_and_move(&mut data);

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules {
                start_maps: vec!["Start".to_string()],
                goal_events: vec!["EVENT_CHAMPION_DEFEATED".to_string()],
                goal_items: vec!["KEY_CARD".to_string()],
                progression_rules: vec![ProgressionRule {
                    id: "beat_champion".to_string(),
                    requires: ProgressionRequirements {
                        maps: vec!["Start".to_string()],
                        ..ProgressionRequirements::default()
                    },
                    grants: ProgressionGrants {
                        events: vec!["EVENT_CHAMPION_DEFEATED".to_string()],
                        items: vec!["KEY_CARD".to_string()],
                        ..ProgressionGrants::default()
                    },
                }],
                ..PlayabilityRules::default()
            },
        );

        assert!(!report.has_errors(), "{:?}", report.diagnostics);
        assert_eq!(report.solvable_maps, vec!["Start".to_string()]);
        assert_eq!(
            report.solvable_events,
            vec!["EVENT_CHAMPION_DEFEATED".to_string()]
        );
        assert_eq!(report.solvable_items, vec!["KEY_CARD".to_string()]);
    }

    #[test]
    fn verifier_solves_events_from_script_granted_loaded_maps() {
        let mut data = GameDataSet {
            maps: [
                (
                    "Start".to_string(),
                    test_map_module("Start", "START_MAP", None),
                ),
                (
                    "ScriptedGoal".to_string(),
                    test_map_module("ScriptedGoal", "SCRIPTED_GOAL", None),
                ),
            ]
            .into_iter()
            .collect(),
            ..GameDataSet::default()
        };
        add_runtime_species_and_move(&mut data);

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules {
                start_maps: vec!["Start".to_string()],
                goal_events: vec!["EVENT_SCRIPTED_ENDING".to_string()],
                progression_rules: vec![
                    ProgressionRule {
                        id: "scripted_warp".to_string(),
                        requires: ProgressionRequirements {
                            maps: vec!["Start".to_string()],
                            ..ProgressionRequirements::default()
                        },
                        grants: ProgressionGrants {
                            maps: vec!["ScriptedGoal".to_string()],
                            ..ProgressionGrants::default()
                        },
                    },
                    ProgressionRule {
                        id: "scripted_goal_event".to_string(),
                        requires: ProgressionRequirements {
                            maps: vec!["ScriptedGoal".to_string()],
                            ..ProgressionRequirements::default()
                        },
                        grants: ProgressionGrants {
                            events: vec!["EVENT_SCRIPTED_ENDING".to_string()],
                            ..ProgressionGrants::default()
                        },
                    },
                ],
                ..PlayabilityRules::default()
            },
        );

        assert!(!report.has_errors(), "{:?}", report.diagnostics);
        assert_eq!(
            report.solvable_maps,
            vec!["ScriptedGoal".to_string(), "Start".to_string()]
        );
        assert_eq!(
            report.solvable_events,
            vec!["EVENT_SCRIPTED_ENDING".to_string()]
        );
    }

    #[test]
    fn verifier_rejects_unsolved_progression_event_goals() {
        let data = GameDataSet {
            maps: [(
                "Start".to_string(),
                test_map_module("Start", "START_MAP", None),
            )]
            .into_iter()
            .collect(),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules {
                start_maps: vec!["Start".to_string()],
                goal_events: vec!["EVENT_CHAMPION_DEFEATED".to_string()],
                progression_rules: vec![ProgressionRule {
                    id: "blocked_champion".to_string(),
                    requires: ProgressionRequirements {
                        events: vec!["EVENT_NEVER_GRANTED".to_string()],
                        ..ProgressionRequirements::default()
                    },
                    grants: ProgressionGrants {
                        events: vec!["EVENT_CHAMPION_DEFEATED".to_string()],
                        ..ProgressionGrants::default()
                    },
                }],
                ..PlayabilityRules::default()
            },
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unsolved_goal_event"
                && diagnostic.subject == "EVENT_CHAMPION_DEFEATED"
        }));
    }

    #[test]
    fn verifier_applies_map_access_requirements_to_reachable_maps() {
        let key_card = Item {
            name: "Key Card".to_string(),
            description: "Opens a required gate.".to_string(),
            effect: "NONE".to_string(),
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
            escape_rope_mode: None,
            price: 0,
            held_effect: "HELD_NONE".to_string(),
            parameter: 0,
            property: String::new(),
            pocket: item_pocket("KEY_ITEM"),
            field_menu: "ITEMMENU_NOUSE".to_string(),
            field_usable: false,
            battle_menu: "ITEMMENU_NOUSE".to_string(),
            battle_usable: false,
            script_name: "KEY_CARD".to_string(),
            consumable: false,
            tmhm_index: None,
            tmhm_move: None,
        };
        let mut start = test_map_module("Start", "START_MAP", Some("Goal"));
        start.attributes.height = 2;
        start.blocks = vec![5, 1];
        let mut goal = test_map_module("Goal", "GOAL_MAP", None);
        goal.attributes.height = 2;
        goal.blocks = vec![1, 1];
        let mut data = GameDataSet {
            maps: [("Start".to_string(), start), ("Goal".to_string(), goal)]
                .into_iter()
                .collect(),
            items: [("KEY_CARD".to_string(), key_card)].into_iter().collect(),
            ..GameDataSet::default()
        };
        add_runtime_species_and_move(&mut data);

        let blocked = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules {
                start_maps: vec!["Start".to_string()],
                goal_maps: vec!["Goal".to_string()],
                map_access: vec![MapAccessRule {
                    map: "Goal".to_string(),
                    requires: ProgressionRequirements {
                        items: vec!["KEY_CARD".to_string()],
                        ..ProgressionRequirements::default()
                    },
                }],
                ..PlayabilityRules::default()
            },
        );

        assert_eq!(
            blocked.reachable_maps,
            vec!["Goal".to_string(), "Start".to_string()]
        );
        assert_eq!(blocked.solvable_maps, vec!["Start".to_string()]);
        assert!(blocked.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unsolved_goal_map" && diagnostic.subject == "Goal"
        }));

        let solved = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules {
                start_maps: vec!["Start".to_string()],
                goal_maps: vec!["Goal".to_string()],
                progression_rules: vec![ProgressionRule {
                    id: "get_key_card".to_string(),
                    requires: ProgressionRequirements {
                        maps: vec!["Start".to_string()],
                        ..ProgressionRequirements::default()
                    },
                    grants: ProgressionGrants {
                        items: vec!["KEY_CARD".to_string()],
                        ..ProgressionGrants::default()
                    },
                }],
                map_access: vec![MapAccessRule {
                    map: "Goal".to_string(),
                    requires: ProgressionRequirements {
                        items: vec!["KEY_CARD".to_string()],
                        ..ProgressionRequirements::default()
                    },
                }],
                ..PlayabilityRules::default()
            },
        );

        assert!(!solved.has_errors(), "{:?}", solved.diagnostics);
        assert_eq!(
            solved.solvable_maps,
            vec!["Goal".to_string(), "Start".to_string()]
        );
    }

    #[test]
    fn verifier_requires_explicit_start_maps_for_solvability_rules() {
        let mut known_species = species();
        known_species.tmhm_learnset.clear();
        let data = GameDataSet {
            pokemon: [(known_species.id.clone(), known_species)]
                .into_iter()
                .collect(),
            moves: [("TACKLE".to_string(), test_move("TACKLE"))]
                .into_iter()
                .collect(),
            maps: [(
                "Goal".to_string(),
                test_map_module("Goal", "GOAL_MAP", None),
            )]
            .into_iter()
            .collect(),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules {
                start_maps: Vec::new(),
                start_tiles: Vec::new(),
                goal_maps: vec!["Goal".to_string()],
                require_all_maps_reachable: false,
                require_walkable_maps: false,
                ..PlayabilityRules::default()
            },
        );

        assert!(report.reachable_maps.is_empty());
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "missing_start_map" && diagnostic.subject == "playability"
        }));
    }

    #[test]
    fn compiler_rejects_missing_manifest_dependencies() {
        let manifest = ModpackManifest {
            metadata: ModpackMetadata {
                id: "dependent".to_string(),
                name: "Dependent".to_string(),
                version: "1.0.0".to_string(),
                author: None,
                description: None,
            },
            dependencies: vec!["missing-base".to_string()],
            ..ModpackManifest::default()
        };

        let error = AssetRoot::new(repository_root_for_tests())
            .compile_modpacks(&[manifest], ModpackCompileOptions::default())
            .expect_err("missing dependency should fail compilation");

        assert!(
            error
                .to_string()
                .contains("depends on missing modpack 'missing-base'")
        );
    }

    #[test]
    fn base_game_data_loads_existing_exported_wild_encounter_json() {
        let root = repository_root_for_tests();
        let data = AssetRoot::new(root)
            .load_base_game_data()
            .expect("load base game data");

        let route29 = data
            .wild_encounters
            .get("Route29")
            .expect("load Route 29 wild encounters");
        let slots = table_for_surface(route29, EncounterSurface::Grass, TimeOfDay::Day)
            .expect("Route 29 day grass table");
        assert_eq!(data.wild_encounters.len(), 114);
        assert_eq!(route29.grass_rates.as_ref().unwrap()["day"], 10);
        assert_eq!(slots.len(), 7);
        assert_eq!(slots[0].species, "PIDGEY");
    }

    #[test]
    fn base_game_data_loads_trainers_into_exact_catalog() {
        let root = repository_root_for_tests();
        let data = AssetRoot::new(root)
            .load_base_game_data()
            .expect("load base game data");

        let trainer = data
            .trainers
            .get("FALKNER1")
            .expect("FALKNER1 trainer data");
        assert_eq!(trainer.trainer_id, "FALKNER1");
        assert_eq!(trainer.trainer_class, "FALKNER");
        assert_eq!(trainer.party.len(), 2);
        assert_eq!(trainer.party[0].species, "PIDGEY");
        assert_eq!(trainer.party[1].species, "PIDGEOTTO");

        let start = data
            .trainer_battle_start(
                &crystal_core::state::GameState::default(),
                TrainerBattleRequest::new("FALKNER", "FALKNER1", "EVENT_BEAT_FALKNER"),
            )
            .expect("trainer battle start resolves from pack catalog");

        let TrainerBattleStartStatus::Started(start) = start else {
            panic!("FALKNER1 should not be defeated in default state");
        };
        assert_eq!(start.trainer_class, "FALKNER");
        assert_eq!(start.trainer_id, "FALKNER1");
        assert_eq!(start.enemy_party.len(), 2);
        assert_eq!(start.enemy_pokemon.species.id, "PIDGEY");
        assert_eq!(start.enemy_pokemon.moves[0].name, "TACKLE");
    }

    #[test]
    fn route29_overworld_map_is_assembled_from_core_modular_pack() {
        let root = repository_root_for_tests();
        let data = AssetRoot::new(root)
            .load_base_game_data()
            .expect("load base game data");

        let map = data.overworld_map("Route29").expect("assemble Route29");

        assert_eq!(map.name, "Route29");
        assert_eq!((map.width, map.height), (30, 9));
        assert_eq!(map.border_block, 5);
        assert_eq!(map.metatile_ids.len(), 270);
        assert_eq!(map.metatile_ids[0], 5);
        assert_eq!(map.tile_bounds(), (60, 18));
    }

    #[test]
    fn route29_map_module_is_assembled_from_core_modular_pack() {
        let root = repository_root_for_tests();
        let data = AssetRoot::new(root)
            .load_base_game_data()
            .expect("load base game data");

        let module = data.map_module("Route29").expect("assemble Route29 module");

        assert_eq!(module.id, "Route29");
        assert_eq!(module.attributes.map_constant.as_deref(), Some("ROUTE_29"));
        assert_eq!(module.blocks.len(), 270);
        assert_eq!(module.objects.len(), 8);
        assert_eq!(module.objects[0].hram_x, -1);
        assert_eq!(
            module.objects[0].object_identifier.as_deref(),
            Some("ROUTE29_COOLTRAINER_M1")
        );
        assert_eq!(module.events.warps.len(), 1);
        assert_eq!(
            module.events.warps[0].target_map_constant,
            "ROUTE_29_ROUTE_46_GATE"
        );
        assert_eq!(module.events.warps[0].target_map, "ROUTE_29_ROUTE_46_GATE");
        assert_eq!(module.events.coord_events.len(), 2);
        assert_eq!(module.events.bg_events.len(), 2);
        assert_eq!(module.events.bg_events[0].event_type, "BGEVENT_READ");
        assert_eq!(module.events.bg_events[0].script, "Route29Sign1");
        assert!(
            module
                .map_event_section_commands
                .iter()
                .any(|command| command.command == "def_warp_events" && command.command_index == 1)
        );
        assert!(module.map_event_section_commands.iter().any(|command| {
            command.command == "warp_event"
                && command.args == vec!["27", "1", "ROUTE_29_ROUTE_46_GATE", "3"]
        }));
        assert!(module.map_event_section_commands.iter().any(|command| {
            command.command == "coord_event"
                && command.args
                    == vec![
                        "53",
                        "8",
                        "SCENE_ROUTE29_CATCH_TUTORIAL",
                        "Route29Tutorial1",
                    ]
        }));
        assert!(module.map_event_section_commands.iter().any(|command| {
            command.command == "object_event"
                && command.args[2] == "SPRITE_COOLTRAINER_M"
                && command.args[11] == "Route29CooltrainerMScript"
        }));
        assert!(module.scripts.contains_key("Route29_MapScripts"));
        assert!(module.scripts.contains_key("Route29YoungsterScript"));
    }

    #[test]
    fn map_module_extracts_trainer_battle_requests_from_exact_script_args() {
        let root = repository_root_for_tests();
        let data = AssetRoot::new(root)
            .load_base_game_data()
            .expect("load base game data");

        let module = data
            .map_module("BlackthornGym2F")
            .expect("assemble BlackthornGym2F module");
        let request = module
            .trainer_scripts
            .get("TrainerCooltrainermCody")
            .expect("Cody trainer script");

        assert_eq!(request.trainer_class, "COOLTRAINERM");
        assert_eq!(request.trainer_id, "CODY");
        assert_eq!(request.event_flag, "EVENT_BEAT_COOLTRAINERM_CODY");
        assert_eq!(request.seen_text, "CooltrainermCodySeenText");
        assert_eq!(request.win_text, "CooltrainermCodyBeatenText");
        assert_eq!(request.loss_text, "");
        assert_eq!(request.callback, ".Script");
        assert_eq!(request.source_script, "TrainerCooltrainermCody");

        let start = data
            .trainer_battle_start(&crystal_core::state::GameState::default(), request.clone())
            .expect("trainer battle start resolves from extracted map script");
        let TrainerBattleStartStatus::Started(start) = start else {
            panic!("Cody should not be defeated in default state");
        };
        assert_eq!(start.trainer_class, "COOLTRAINERM");
        assert_eq!(start.trainer_id, "CODY");
        assert_eq!(start.event_flag, "EVENT_BEAT_COOLTRAINERM_CODY");
        assert!(!start.enemy_party.is_empty());
    }

    #[test]
    fn map_module_extracts_scripted_loadtrainer_battles_with_post_flags() {
        let root = repository_root_for_tests();
        let data = AssetRoot::new(root)
            .load_base_game_data()
            .expect("load base game data");

        let module = data
            .map_module("VermilionGym")
            .expect("assemble VermilionGym module");
        let battle = module
            .scripted_trainer_battles
            .iter()
            .find(|battle| battle.source_script == "VermilionGymSurgeScript")
            .expect("Surge scripted trainer battle");

        assert_eq!(battle.request.battle_type, "BATTLETYPE_TRAINER");
        assert_eq!(battle.request.trainer_class, "LT_SURGE");
        assert_eq!(battle.request.trainer_id, "LT_SURGE1");
        assert_eq!(battle.request.event_flag, "");
        assert_eq!(battle.request.win_text, "LtSurgeWinLossText");
        assert_eq!(battle.request.loss_text, "");
        assert!(battle.reload_map_after_battle);
        assert_eq!(
            battle.post_battle_event_flags,
            vec![
                "EVENT_BEAT_LTSURGE".to_string(),
                "EVENT_BEAT_GENTLEMAN_GREGORY".to_string(),
                "EVENT_BEAT_GUITARIST_VINCENT".to_string(),
                "EVENT_BEAT_JUGGLER_HORTON".to_string(),
            ]
        );
        assert_eq!(
            battle.post_battle_script_flags,
            vec!["ENGINE_THUNDERBADGE".to_string()]
        );

        let start = data
            .trainer_battle_start(
                &crystal_core::state::GameState::default(),
                battle.request.clone(),
            )
            .expect("scripted trainer battle starts from pack data");
        let TrainerBattleStartStatus::Started(start) = start else {
            panic!("Surge should not be defeated by request event flag");
        };
        assert_eq!(start.trainer_class, "LT_SURGE");
        assert_eq!(start.trainer_id, "LT_SURGE1");
        assert_eq!(start.win_text, "LtSurgeWinLossText");
        assert_eq!(start.enemy_party.len(), 5);
    }

    #[test]
    fn scripted_trainer_battle_effects_apply_post_battle_badge_flags() {
        let root = repository_root_for_tests();
        let data = AssetRoot::new(root)
            .load_base_game_data()
            .expect("load base game data");
        let module = data
            .map_module("VermilionGym")
            .expect("assemble VermilionGym module");
        let battle = module
            .scripted_trainer_battles
            .iter()
            .find(|battle| battle.source_script == "VermilionGymSurgeScript")
            .expect("Surge scripted trainer battle");
        let mut state = GameState::default();
        let effects = ScriptedBattleEffects {
            event_flags: battle.post_battle_event_flags.clone(),
            script_flags: battle.post_battle_script_flags.clone(),
            disappear_object_ids: Vec::new(),
        };

        let outcome = crystal_core::systems::scripted_battles::apply_scripted_battle_effects(
            &mut state,
            &module.objects,
            &effects,
        )
        .expect("Surge post battle effects apply");

        assert_eq!(
            outcome.event_flags_set,
            vec![
                "EVENT_BEAT_LTSURGE".to_string(),
                "EVENT_BEAT_GENTLEMAN_GREGORY".to_string(),
                "EVENT_BEAT_GUITARIST_VINCENT".to_string(),
                "EVENT_BEAT_JUGGLER_HORTON".to_string(),
            ]
        );
        assert_eq!(
            outcome.script_flags_set,
            vec!["ENGINE_THUNDERBADGE".to_string()]
        );
        assert_eq!(
            state.flags.is_event_flag_set("EVENT_BEAT_LTSURGE"),
            Ok(true)
        );
        assert_eq!(
            state.flags.is_engine_flag_set("ENGINE_THUNDERBADGE"),
            Ok(true)
        );
        assert_eq!(
            state.flags.is_event_flag_set("ENGINE_THUNDERBADGE"),
            Ok(false)
        );
    }

    #[test]
    fn map_module_extracts_scripted_rival_battle_win_loss_text() {
        let root = repository_root_for_tests();
        let data = AssetRoot::new(root)
            .load_base_game_data()
            .expect("load base game data");

        let module = data
            .map_module("AzaleaTown")
            .expect("assemble AzaleaTown module");
        let battle = module
            .scripted_trainer_battles
            .iter()
            .find(|battle| battle.source_script == "AzaleaTownRivalBattleScript")
            .expect("Azalea rival scripted trainer battle");

        assert_eq!(battle.request.trainer_class, "RIVAL1");
        assert_eq!(battle.request.trainer_id, "RIVAL1_2_TOTODILE");
        assert_eq!(battle.request.win_text, "AzaleaTownRivalWinText");
        assert_eq!(battle.request.loss_text, "AzaleaTownRivalLossText");
        assert!(battle.reload_map_after_battle);
    }

    #[test]
    fn map_module_extracts_static_lugia_battle_with_forceitem_metadata() {
        let root = repository_root_for_tests();
        let data = AssetRoot::new(root)
            .load_base_game_data()
            .expect("load base game data");

        let module = data
            .map_module("WhirlIslandLugiaChamber")
            .expect("assemble Lugia chamber module");
        let battle = module
            .scripted_wild_battles
            .iter()
            .find(|battle| battle.source_script == "Lugia")
            .expect("Lugia scripted wild battle");

        assert_eq!(battle.request.battle_type, "BATTLETYPE_FORCEITEM");
        assert_eq!(battle.request.species, "LUGIA");
        assert_eq!(battle.request.level, 60);
        assert_eq!(battle.pre_battle_event_flags, vec!["EVENT_FOUGHT_LUGIA"]);
        assert_eq!(
            battle.disappear_object_ids,
            vec!["WHIRLISLANDLUGIACHAMBER_LUGIA"]
        );
        assert!(battle.reload_map_after_battle);

        let mut rng = Random::new(1);
        let start = data
            .static_wild_battle_start(battle.request.clone(), &mut rng)
            .expect("Lugia battle starts from pack data");
        assert_eq!(start.battle_type, "BATTLETYPE_FORCEITEM");
        assert_eq!(start.enemy_pokemon.species.id, "LUGIA");
        assert_eq!(start.enemy_pokemon.level, 60);
        assert_eq!(start.enemy_pokemon.original_trainer_name, "WILD");
        let lugia = data.pokemon.get("LUGIA").expect("LUGIA species");
        assert_eq!(
            start.enemy_pokemon.item,
            lugia.item1.clone().or_else(|| lugia.item2.clone())
        );
    }

    #[test]
    fn map_module_extracts_static_red_gyarados_forceshiny_battle() {
        let root = repository_root_for_tests();
        let data = AssetRoot::new(root)
            .load_base_game_data()
            .expect("load base game data");

        let module = data
            .map_module("LakeOfRage")
            .expect("assemble LakeOfRage module");
        let battle = module
            .scripted_wild_battles
            .iter()
            .find(|battle| battle.source_script == "RedGyarados")
            .expect("Red Gyarados scripted wild battle");

        assert_eq!(battle.request.battle_type, "BATTLETYPE_FORCESHINY");
        assert_eq!(battle.request.species, "GYARADOS");
        assert_eq!(battle.request.level, 30);
        assert_eq!(battle.disappear_object_ids, vec!["LAKEOFRAGE_GYARADOS"]);
        assert!(battle.reload_map_after_battle);

        let mut rng = Random::new(1);
        let start = data
            .static_wild_battle_start(battle.request.clone(), &mut rng)
            .expect("Red Gyarados battle starts from pack data");
        assert_eq!(start.enemy_pokemon.dvs, Dv::from_non_hp(14, 10, 10, 10));
    }

    #[test]
    fn map_module_extracts_static_snorlax_post_battle_event() {
        let root = repository_root_for_tests();
        let data = AssetRoot::new(root)
            .load_base_game_data()
            .expect("load base game data");

        let module = data
            .map_module("VermilionCity")
            .expect("assemble VermilionCity module");
        let battle = module
            .scripted_wild_battles
            .iter()
            .find(|battle| battle.source_script == "VermilionSnorlax")
            .expect("Snorlax scripted wild battle");

        assert_eq!(battle.request.battle_type, "BATTLETYPE_FORCEITEM");
        assert_eq!(battle.request.species, "SNORLAX");
        assert_eq!(battle.request.level, 50);
        assert_eq!(
            battle.disappear_object_ids,
            vec!["VERMILIONCITY_BIG_SNORLAX"]
        );
        assert_eq!(battle.post_battle_event_flags, vec!["EVENT_FOUGHT_SNORLAX"]);
        assert!(battle.reload_map_after_battle);
    }

    #[test]
    fn scripted_static_battle_effects_hide_pack_object_by_exact_disappear_id() {
        let root = repository_root_for_tests();
        let asset_root = AssetRoot::new(&root);
        let data = asset_root
            .load_base_game_data()
            .expect("load base game data");
        let module = data
            .map_module("VermilionCity")
            .expect("assemble VermilionCity module");
        let map = data
            .overworld_map("VermilionCity")
            .expect("assemble VermilionCity map");
        let tileset = asset_root
            .load_tileset_collision("johto")
            .expect("load johto collision");
        let mut session = OverworldSession::with_events_and_objects(
            map,
            module.events.clone(),
            module.objects.clone(),
            tileset,
            TilePosition::new(1, 1),
        );
        let snorlax = module
            .scripted_wild_battles
            .iter()
            .find(|battle| battle.source_script == "VermilionSnorlax")
            .expect("Snorlax battle");
        assert!(
            session
                .objects
                .iter()
                .find(|object| object.object_identifier.as_deref()
                    == Some("VERMILIONCITY_BIG_SNORLAX"))
                .map(|object| session.is_object_visible(object))
                .unwrap_or(false)
        );

        let mut state = GameState::default();
        let effects = ScriptedBattleEffects {
            event_flags: snorlax.post_battle_event_flags.clone(),
            script_flags: snorlax.post_battle_script_flags.clone(),
            disappear_object_ids: snorlax.disappear_object_ids.clone(),
        };
        let outcome = apply_scripted_battle_effects_to_session(&mut state, &mut session, &effects)
            .expect("Snorlax effects apply");

        assert_eq!(outcome.event_flags_set, vec!["EVENT_FOUGHT_SNORLAX"]);
        assert_eq!(
            outcome.disappeared_objects[0].event_flag,
            "EVENT_VERMILION_CITY_SNORLAX"
        );
        assert_eq!(
            state.flags.is_event_flag_set("EVENT_FOUGHT_SNORLAX"),
            Ok(true)
        );
        assert_eq!(
            state
                .flags
                .is_event_flag_set("EVENT_VERMILION_CITY_SNORLAX"),
            Ok(true)
        );
        let snorlax_object = session
            .objects
            .iter()
            .find(|object| object.object_identifier.as_deref() == Some("VERMILIONCITY_BIG_SNORLAX"))
            .expect("Snorlax object");
        assert!(!session.is_object_visible(snorlax_object));
    }

    #[test]
    fn scripted_static_battle_pre_flags_and_disappear_flags_are_distinct() {
        let root = repository_root_for_tests();
        let asset_root = AssetRoot::new(&root);
        let data = asset_root
            .load_base_game_data()
            .expect("load base game data");
        let module = data
            .map_module("WhirlIslandLugiaChamber")
            .expect("assemble Lugia chamber module");
        let map = data
            .overworld_map("WhirlIslandLugiaChamber")
            .expect("assemble Lugia chamber map");
        let tileset = asset_root
            .load_tileset_collision("johto")
            .expect("load johto collision");
        let mut session = OverworldSession::with_events_and_objects(
            map,
            module.events.clone(),
            module.objects.clone(),
            tileset,
            TilePosition::new(1, 1),
        );
        let lugia = module
            .scripted_wild_battles
            .iter()
            .find(|battle| battle.source_script == "Lugia")
            .expect("Lugia battle");

        let mut state = GameState::default();
        let effects = ScriptedBattleEffects {
            event_flags: lugia.pre_battle_event_flags.clone(),
            script_flags: lugia.post_battle_script_flags.clone(),
            disappear_object_ids: lugia.disappear_object_ids.clone(),
        };
        let outcome = apply_scripted_battle_effects_to_session(&mut state, &mut session, &effects)
            .expect("Lugia effects apply");

        assert_eq!(outcome.event_flags_set, vec!["EVENT_FOUGHT_LUGIA"]);
        assert_eq!(
            outcome.disappeared_objects[0].event_flag,
            "EVENT_WHIRL_ISLAND_LUGIA_CHAMBER_LUGIA"
        );
        assert_eq!(
            state.flags.is_event_flag_set("EVENT_FOUGHT_LUGIA"),
            Ok(true)
        );
        assert_eq!(
            state
                .flags
                .is_event_flag_set("EVENT_WHIRL_ISLAND_LUGIA_CHAMBER_LUGIA"),
            Ok(true)
        );
    }

    #[test]
    fn map_module_extracts_scene_table_from_generated_map_scripts() {
        let root = repository_root_for_tests();
        let data = AssetRoot::new(root)
            .load_base_game_data()
            .expect("load base game data");

        let module = data.map_module("ElmsLab").expect("assemble ElmsLab module");

        assert_eq!(module.scenes.scenes.len(), 7);
        assert_eq!(module.scenes.scenes[0].scene_id, "SCENE_ELMSLAB_MEET_ELM");
        assert_eq!(
            module.scenes.scenes[0].script_name.as_deref(),
            Some("ElmsLabMeetElmScene")
        );
        assert_eq!(
            module.scenes.scenes[6].scene_id,
            "SCENE_ELMSLAB_AIDE_GIVES_POKE_BALLS"
        );
        assert_eq!(module.scenes.scenes[6].script_name, None);
        assert!(module
            .map_script_section_commands
            .iter()
            .any(|command| command.command == "def_scene_scripts" && command.command_index == 0));
        assert!(module.map_script_section_commands.iter().any(|command| {
            command.command == "scene_script"
                && command.args == vec!["ElmsLabMeetElmScene", "SCENE_ELMSLAB_MEET_ELM"]
        }));
        assert!(module.map_script_section_commands.iter().any(|command| {
            command.command == "scene_const"
                && command.args == vec!["SCENE_ELMSLAB_AIDE_GIVES_POKE_BALLS"]
        }));
        assert!(module.map_script_section_commands.iter().any(|command| {
            command.command == "callback"
                && command.args == vec!["MAPCALLBACK_OBJECTS", "ElmsLabMoveElmCallback"]
        }));
    }

    #[test]
    fn one_arg_scene_scripts_do_not_synthesize_empty_scene_ids() {
        let root = repository_root_for_tests();
        let data = AssetRoot::new(root)
            .load_base_game_data()
            .expect("load base game data");

        let module = data
            .map_module("AzaleaPokecenter1F")
            .expect("assemble AzaleaPokecenter1F module");

        assert!(module.map_script_section_commands.iter().any(|command| {
            command.command == "scene_script" && command.args == vec!["AzaleaPokecenter1FNoopScene"]
        }));
        assert!(module.scenes.scenes.is_empty());
    }

    #[test]
    fn map_module_extracts_verbose_script_item_grants_with_exact_ids() {
        let root = repository_root_for_tests();
        let data = AssetRoot::new(root)
            .load_base_game_data()
            .expect("load base game data");
        let module = data
            .map_module("VioletGym")
            .expect("assemble VioletGym module");

        let grant = module
            .script_item_grants
            .iter()
            .find(|grant| {
                grant.source_script == "VioletGymFalknerScript" && grant.item_id == "TM_MUD_SLAP"
            })
            .expect("Falkner TM grant");

        assert_eq!(grant.quantity, 1);
        assert_eq!(grant.command_index, 27);
        assert!(grant.verbose);

        assert!(!data.items.contains_key("tm_mud_slap"));
    }

    #[test]
    fn map_module_extracts_quantity_script_item_grants() {
        let root = repository_root_for_tests();
        let data = AssetRoot::new(root)
            .load_base_game_data()
            .expect("load base game data");
        let module = data.map_module("ElmsLab").expect("assemble ElmsLab module");

        let grant = module
            .script_item_grants
            .iter()
            .find(|grant| grant.source_script == "AideScript_GiveYouBalls")
            .expect("aide Poke Ball grant");

        assert_eq!(grant.item_id, "POKE_BALL");
        assert_eq!(grant.quantity, 5);
        assert_eq!(grant.command_index, 5);
        assert!(!grant.verbose);

        let mut state = GameState::default();
        let outcome =
            grant_script_item(&mut state, &data.items, grant.clone()).expect("grant exact balls");

        assert_eq!(
            outcome,
            ScriptItemGrantOutcome::Granted {
                item_id: "POKE_BALL".to_string(),
                quantity: 5,
                source_script: "AideScript_GiveYouBalls".to_string(),
                command_index: 5,
                verbose: false,
            }
        );
        assert_eq!(state.bag.quantity(&data.items["POKE_BALL"]), 5);
    }

    #[test]
    fn map_module_extracts_checkitem_commands_with_exact_ids() {
        let root = repository_root_for_tests();
        let data = AssetRoot::new(root)
            .load_base_game_data()
            .expect("load base game data");
        let module = data
            .map_module("GoldenrodMagnetTrainStation")
            .expect("assemble GoldenrodMagnetTrainStation module");

        let access = module
            .script_item_checks
            .iter()
            .find(|access| {
                access.source_script == "GoldenrodMagnetTrainStationOfficerScript"
                    && access.item_id == "PASS"
            })
            .expect("Magnet Train pass check");

        assert_eq!(access.command_index, 11);

        let mut state = GameState::default();
        let missing = check_script_item(&state, &data.items, access.clone()).expect("check pass");
        assert!(!missing.held);
        state
            .bag
            .add_item(&data.items["PASS"], 1)
            .expect("add pass");
        let held = check_script_item(&state, &data.items, access.clone()).expect("check pass");
        assert!(held.held);
    }

    #[test]
    fn map_module_extracts_takeitem_commands_with_exact_ids() {
        let root = repository_root_for_tests();
        let data = AssetRoot::new(root)
            .load_base_game_data()
            .expect("load base game data");
        let module = data
            .map_module("CopycatsHouse2F")
            .expect("assemble CopycatsHouse2F module");

        let access = module
            .script_item_takes
            .iter()
            .find(|access| access.source_script == "Copycat" && access.item_id == "LOST_ITEM")
            .expect("Copycat lost item take");

        assert_eq!(access.command_index, 65);

        let mut state = GameState::default();
        state
            .bag
            .add_item(&data.items["LOST_ITEM"], 1)
            .expect("add lost item");
        let outcome =
            take_script_item(&mut state, &data.items, access.clone()).expect("take lost item");

        assert!(outcome.removed);
        assert_eq!(state.bag.quantity(&data.items["LOST_ITEM"]), 0);
    }

    #[test]
    fn map_module_extracts_givepoke_commands_with_exact_metadata() {
        let root = repository_root_for_tests();
        let data = AssetRoot::new(root)
            .load_base_game_data()
            .expect("load base game data");
        let module = data.map_module("ElmsLab").expect("assemble ElmsLab module");

        let gift = module
            .gift_pokemon_scripts
            .iter()
            .find(|gift| gift.source_script == "CyndaquilPokeBallScript")
            .expect("Cyndaquil starter gift");

        assert_eq!(gift.species_id, "CYNDAQUIL");
        assert_eq!(gift.level_token, "5");
        assert_eq!(gift.level, 5);
        assert_eq!(gift.held_item_id.as_deref(), Some("BERRY"));
        assert_eq!(gift.command_index, 22);
        assert!(!gift.egg);
    }

    #[test]
    fn map_module_extracts_custom_gift_metadata_labels() {
        let root = repository_root_for_tests();
        let data = AssetRoot::new(root)
            .load_base_game_data()
            .expect("load base game data");
        let module = data
            .map_module("Route35GoldenrodGate")
            .expect("assemble Route35GoldenrodGate module");

        let gift = module
            .gift_pokemon_scripts
            .iter()
            .find(|gift| gift.source_script == "RandyScript")
            .expect("Randy Spearow gift");

        assert_eq!(gift.species_id, "SPEAROW");
        assert_eq!(gift.level_token, "10");
        assert_eq!(gift.level, 10);
        assert_eq!(gift.held_item_id, None);
        assert_eq!(gift.nickname_label.as_deref(), Some("GiftSpearowName"));
        assert_eq!(gift.ot_label.as_deref(), Some("GiftSpearowOTName"));
    }

    #[test]
    fn map_module_extracts_giveegg_with_resolved_pack_level() {
        let root = repository_root_for_tests();
        let data = AssetRoot::new(root)
            .load_base_game_data()
            .expect("load base game data");
        let module = data
            .map_module("VioletPokecenter1F")
            .expect("assemble VioletPokecenter1F module");

        let egg = module
            .gift_pokemon_scripts
            .iter()
            .find(|gift| gift.source_script == "VioletPokecenter1F_ElmsAideScript")
            .expect("Togepi egg gift");

        assert_eq!(egg.species_id, "TOGEPI");
        assert_eq!(egg.level_token, "EGG_LEVEL");
        assert_eq!(egg.level, 5);
        assert!(egg.egg);
    }

    #[test]
    fn gift_level_tokens_resolve_only_from_exact_pack_constants() {
        let mut constants = StoryEventScriptConstants::default();
        assert!(resolve_gift_level_token("Start", "EGG_LEVEL", &constants).is_err());

        constants.global.insert("EGG_LEVEL".to_string(), 5);
        assert_eq!(
            resolve_gift_level_token("Start", "EGG_LEVEL", &constants).expect("global constant"),
            5
        );
        assert!(resolve_gift_level_token("Start", "egg_level", &constants).is_err());

        constants.maps.insert(
            "Start".to_string(),
            [("EGG_LEVEL".to_string(), 6)].into_iter().collect(),
        );
        assert_eq!(
            resolve_gift_level_token("Start", "EGG_LEVEL", &constants).expect("map constant"),
            6
        );
        assert!(resolve_gift_level_token("Start", "0", &constants).is_err());
    }

    #[test]
    fn map_module_extracts_script_flag_commands_and_applies_exact_storage() {
        let root = repository_root_for_tests();
        let data = AssetRoot::new(root)
            .load_base_game_data()
            .expect("load base game data");
        let module = data
            .map_module("RuinsOfAlphKabutoChamber")
            .expect("assemble RuinsOfAlphKabutoChamber module");

        let solved = module
            .script_flag_commands
            .iter()
            .find(|command| {
                command.source_script == "RuinsOfAlphKabutoChamberPuzzle"
                    && command.command == "setevent"
                    && command.flag_id == "EVENT_SOLVED_KABUTO_PUZZLE"
            })
            .expect("exact Kabuto puzzle setevent")
            .clone();
        let unlocked = module
            .script_flag_commands
            .iter()
            .find(|command| {
                command.source_script == "RuinsOfAlphKabutoChamberPuzzle"
                    && command.command == "setflag"
                    && command.flag_id == "ENGINE_UNLOCKED_UNOWNS_A_TO_K"
            })
            .expect("exact Kabuto puzzle setflag")
            .clone();

        assert_eq!(solved.command_index, 7);
        assert_eq!(unlocked.command_index, 8);

        let mut state = GameState::default();
        let solved_outcome =
            apply_script_flag_mutation(&mut state, solved).expect("apply exact event mutation");
        let unlocked_outcome =
            apply_script_flag_mutation(&mut state, unlocked).expect("apply exact engine mutation");

        assert!(!solved_outcome.engine_flag);
        assert!(unlocked_outcome.engine_flag);
        assert_eq!(
            check_script_flag(
                &state,
                ScriptFlagCommand {
                    command: "checkevent".to_string(),
                    flag_id: "EVENT_SOLVED_KABUTO_PUZZLE".to_string(),
                    source_script: "RuinsOfAlphKabutoChamberHiddenDoorsCallback".to_string(),
                    command_index: 3,
                },
            )
            .expect("check exact event flag")
            .set,
            true
        );
        assert_eq!(
            check_script_flag(
                &state,
                ScriptFlagCommand {
                    command: "checkevent".to_string(),
                    flag_id: "event_solved_kabuto_puzzle".to_string(),
                    source_script: "RuinsOfAlphKabutoChamberHiddenDoorsCallback".to_string(),
                    command_index: 3,
                },
            )
            .expect("case-changed flag remains distinct")
            .set,
            false
        );
    }

    #[test]
    fn map_module_extracts_scene_commands_and_applies_exact_scene_tables() {
        let root = repository_root_for_tests();
        let data = AssetRoot::new(root)
            .load_base_game_data()
            .expect("load base game data");
        let route43 = data.map_module("Route43").expect("assemble Route43 module");
        let gate = data
            .map_module("Route43Gate")
            .expect("assemble Route43Gate module");

        let command = route43
            .script_scene_commands
            .iter()
            .find(|command| {
                command.source_script == "Route43CheckIfRocketsScene"
                    && command.command == "setmapscene"
                    && command.map_id.as_deref() == Some("ROUTE_43_GATE")
                    && command.scene_id.as_deref() == Some("SCENE_ROUTE43GATE_ROCKET_SHAKEDOWN")
            })
            .expect("Route43 setmapscene to Route43Gate")
            .clone();
        assert_eq!(command.command_index, 2);

        let target_map = data
            .map_name_for_constant(command.map_id.as_deref().expect("target map id"))
            .expect("resolve exact target map constant");
        let mut state = GameState::default();
        let outcome = apply_script_scene_command(
            &mut state,
            "Route43",
            Some(&target_map),
            &gate.scenes,
            command,
        )
        .expect("apply setmapscene");

        assert_eq!(target_map, "Route43Gate");
        assert_eq!(outcome.scene_id, "SCENE_ROUTE43GATE_ROCKET_SHAKEDOWN");
        assert_eq!(outcome.scene_index, 0);
        assert_eq!(
            state.scenes.map_scenes["Route43Gate"],
            "SCENE_ROUTE43GATE_ROCKET_SHAKEDOWN"
        );

        let gate_setscene = gate
            .script_scene_commands
            .iter()
            .find(|command| {
                command.source_script == "Route43GateRocketTakeoverScript"
                    && command.command == "setscene"
                    && command.scene_id.as_deref() == Some("SCENE_ROUTE43GATE_NOOP")
            })
            .expect("Route43Gate setscene noop")
            .clone();
        assert_eq!(gate_setscene.command_index, 4);
        state
            .scenes
            .enter_map("Route43Gate", &gate.scenes)
            .expect("enter gate map");
        let outcome = apply_script_scene_command(
            &mut state,
            "Route43Gate",
            None,
            &gate.scenes,
            gate_setscene,
        )
        .expect("apply setscene");
        assert_eq!(outcome.scene_id, "SCENE_ROUTE43GATE_NOOP");
        assert_eq!(state.scenes.scene_name, "SCENE_ROUTE43GATE_NOOP");

        assert_eq!(
            route43
                .scenes
                .scenes
                .iter()
                .map(|scene| scene.scene_id.as_str())
                .collect::<Vec<_>>(),
            vec!["0", "1"]
        );
        let route_setscene = gate
            .script_scene_commands
            .iter()
            .find(|command| {
                command.source_script == "Route43GateCheckIfRocketsCallback"
                    && command.command == "setmapscene"
                    && command.map_id.as_deref() == Some("ROUTE_43")
                    && command.scene_id.as_deref() == Some("1")
            })
            .expect("Route43Gate setmapscene back to Route43")
            .clone();
        let outcome = apply_script_scene_command(
            &mut state,
            "Route43Gate",
            Some("Route43"),
            &route43.scenes,
            route_setscene,
        )
        .expect("apply numeric Route43 setmapscene");
        assert_eq!(outcome.map_name, "Route43");
        assert_eq!(outcome.scene_id, "1");
        assert_eq!(outcome.scene_index, 1);
        assert_eq!(state.scenes.map_scenes["Route43"], "1");
    }

    #[test]
    fn map_module_extracts_script_audio_commands_with_exact_tokens() {
        let root = repository_root_for_tests();
        let data = AssetRoot::new(root)
            .load_base_game_data()
            .expect("load base game data");

        let gate = data
            .map_module("Route43Gate")
            .expect("assemble Route43Gate module");
        let music = gate
            .script_audio_commands
            .iter()
            .find(|command| command.source_script == "Route43GateRocketTakeoverScript")
            .expect("Route43Gate takeover music");
        assert_eq!(music.command, "playmusic");
        assert_eq!(music.audio_id.as_deref(), Some("MUSIC_ROCKET_ENCOUNTER"));
        assert_eq!(music.command_index, 0);

        let gym = data
            .map_module("MahoganyGym")
            .expect("assemble MahoganyGym module");
        let badge = gym
            .script_audio_commands
            .iter()
            .find(|command| command.audio_id.as_deref() == Some("SFX_GET_BADGE"))
            .expect("Mahogany badge sound");
        assert_eq!(badge.command, "playsound");
        assert!(
            gym.script_audio_commands
                .iter()
                .any(|command| command.command == "waitsfx" && command.audio_id.is_none())
        );

        let lugia = data
            .map_module("WhirlIslandLugiaChamber")
            .expect("assemble WhirlIslandLugiaChamber module");
        let cry = lugia
            .script_audio_commands
            .iter()
            .find(|command| command.command == "cry" && command.source_script == "Lugia")
            .expect("Lugia cry");
        assert_eq!(cry.audio_id.as_deref(), Some("LUGIA"));
        assert_eq!(cry.fade_frames, None);
    }

    #[test]
    fn map_module_extracts_changeblock_commands_and_applies_exact_map_blocks() {
        let root = repository_root_for_tests();
        let data = AssetRoot::new(root)
            .load_base_game_data()
            .expect("load base game data");
        let module = data
            .map_module("RuinsOfAlphKabutoChamber")
            .expect("assemble RuinsOfAlphKabutoChamber module");
        let change = module
            .script_block_changes
            .iter()
            .find(|change| {
                change.source_script == "RuinsOfAlphKabutoChamberHiddenDoorsCallback"
                    && change.x == 4
                    && change.y == 0
            })
            .expect("Kabuto chamber wall-open changeblock")
            .clone();

        assert_eq!(change.block_id, 0x2e);
        assert_eq!(change.command_index, 2);

        let mut map = data
            .overworld_map("RuinsOfAlphKabutoChamber")
            .expect("load Kabuto chamber map");
        let previous = map.metatile_at(2, 0).expect("block before change");
        let outcome =
            apply_script_block_change(&mut map, change).expect("apply exact block change");

        assert_eq!((outcome.metatile_x, outcome.metatile_y), (2, 0));
        assert_eq!(outcome.previous_block_id, previous);
        assert_eq!(outcome.block_id, 0x2e);
        assert_eq!(map.metatile_at(2, 0), Some(0x2e));
    }

    #[test]
    fn map_module_extracts_script_map_commands_with_exact_destinations() {
        let root = repository_root_for_tests();
        let data = AssetRoot::new(root)
            .load_base_game_data()
            .expect("load base game data");

        let gym = data
            .map_module("EcruteakGym")
            .expect("assemble EcruteakGym module");
        let warp = gym
            .script_map_commands
            .iter()
            .find(|command| command.source_script == "EcruteakGymClosed")
            .expect("Ecruteak gym closed warp");
        assert_eq!(warp.command, "warp");
        assert_eq!(warp.target_map.as_deref(), Some("ECRUTEAK_CITY"));
        assert_eq!((warp.x, warp.y), (Some(6), Some(27)));
        assert_eq!(warp.command_index, 12);

        let train = data
            .map_module("SaffronMagnetTrainStation")
            .expect("assemble SaffronMagnetTrainStation module");
        assert!(train.script_map_commands.iter().any(|command| {
            command.command == "newloadmap"
                && command.map_setup.as_deref() == Some("MAPSETUP_TRAIN")
        }));

        let bedroom = data
            .map_module("PlayersHouse2F")
            .expect("assemble PlayersHouse2F module");
        assert!(bedroom.script_map_commands.iter().any(|command| {
            command.command == "warp"
                && command.target_map.as_deref() == Some("NONE")
                && command.x == Some(0)
                && command.y == Some(0)
        }));
    }

    #[test]
    fn map_module_extracts_script_text_commands_with_exact_labels() {
        let root = repository_root_for_tests();
        let data = AssetRoot::new(root)
            .load_base_game_data()
            .expect("load base game data");
        let module = data
            .map_module("BlackthornCity")
            .expect("assemble BlackthornCity module");

        let write = module
            .script_text_commands
            .iter()
            .find(|command| {
                command.source_script == "BlackthornSuperNerdScript"
                    && command.command == "writetext"
                    && command.text_label.as_deref() == Some("Text_ClairIsOut")
            })
            .expect("Blackthorn super nerd text");
        assert_eq!(write.command_index, 6);

        let sign = module
            .script_text_commands
            .iter()
            .find(|command| {
                command.source_script == "BlackthornCitySign"
                    && command.command == "jumptext"
                    && command.text_label.as_deref() == Some("BlackthornCitySignText")
            })
            .expect("Blackthorn city sign text");
        assert_eq!(sign.command_index, 0);

        let gramps = module
            .script_text_commands
            .iter()
            .find(|command| {
                command.source_script == "BlackthornGramps1Script"
                    && command.command == "jumptextfaceplayer"
                    && command.text_label.as_deref() == Some("BlackthornGrampsRefusesEntryText")
            })
            .expect("Blackthorn gramps face text");
        assert_eq!(gramps.command_index, 0);

        assert!(module.script_text_commands.iter().any(|command| {
            command.source_script == "BlackthornSuperNerdScript"
                && command.command == "opentext"
                && command.text_label.is_none()
        }));
        assert!(module.script_text_commands.iter().any(|command| {
            command.source_script == "BlackthornSuperNerdScript"
                && command.command == "waitbutton"
                && command.text_label.is_none()
        }));
        assert!(module.script_text_commands.iter().any(|command| {
            command.source_script == "BlackthornSuperNerdScript"
                && command.command == "closetext"
                && command.text_label.is_none()
        }));

        let clair = module
            .script_text_bodies
            .get("Text_ClairIsOut")
            .expect("typed Clair text body");
        assert_eq!(clair.label, "Text_ClairIsOut");
        assert_eq!(clair.commands[0].command, "text");
        assert_eq!(clair.commands[0].args, vec!["\"I am sorry.\""]);
        assert_eq!(clair.commands[1].command, "para");
        assert_eq!(clair.commands[1].args, vec!["\"CLAIR, our GYM\""]);
        assert!(
            clair
                .commands
                .iter()
                .any(|command| command.command == "done" && command.args.is_empty())
        );

        let vending = data
            .map_module("CeladonDeptStore6F")
            .expect("assemble CeladonDeptStore6F module");
        let menu_header = vending
            .script_menu_definitions
            .get(".MenuHeader@CeladonDeptStore6FVendingMachine")
            .expect("typed vending menu header");
        assert!(menu_header.commands.iter().any(|command| {
            command.command == "menu_coords"
                && command.args == vec!["0", "2", "SCREEN_WIDTH - 1", "TEXTBOX_Y - 1"]
        }));
        let menu_data = vending
            .script_menu_definitions
            .get(".MenuData@CeladonDeptStore6FVendingMachine")
            .expect("typed vending menu data");
        assert!(menu_data.commands.iter().any(|command| {
            command.command == "db"
                && command.args
                    == vec!["\"FRESH WATER  ¥{d:CELADONDEPTSTORE6F_FRESH_WATER_PRICE}@\""]
        }));
    }

    #[test]
    fn map_module_extracts_script_variable_commands_with_exact_tokens() {
        let root = repository_root_for_tests();
        let data = AssetRoot::new(root)
            .load_base_game_data()
            .expect("load base game data");

        let route44 = data.map_module("Route44").expect("assemble Route44 module");
        let caller = route44
            .script_variable_commands
            .iter()
            .find(|command| {
                command.source_script == "TrainerBirdKeeperVance1"
                    && command.command == "loadvar"
                    && command.target.as_deref() == Some("VAR_CALLERID")
            })
            .expect("Vance caller variable");
        assert_eq!(caller.command_index, 1);
        assert_eq!(caller.value_tokens, vec!["PHONE_BIRDKEEPER_VANCE"]);

        let rematch_read = route44
            .script_variable_commands
            .iter()
            .find(|command| {
                command.source_script == ".WantsBattle@TrainerBirdKeeperVance1"
                    && command.command == "readmem"
                    && command.target.as_deref() == Some("wVanceFightCount")
            })
            .expect("Vance fight count read");
        assert_eq!(rematch_read.command_index, 2);
        assert!(rematch_read.value_tokens.is_empty());

        let rematch_load = route44
            .script_variable_commands
            .iter()
            .find(|command| {
                command.source_script == ".LoadFight1@TrainerBirdKeeperVance1"
                    && command.command == "loadmem"
                    && command.target.as_deref() == Some("wVanceFightCount")
            })
            .expect("Vance fight count load");
        assert_eq!(rematch_load.command_index, 3);
        assert_eq!(rematch_load.value_tokens, vec!["2"]);

        let route29 = data.map_module("Route29").expect("assemble Route29 module");
        let weekday = route29
            .script_variable_commands
            .iter()
            .find(|command| {
                command.source_script == "Route29TuscanyCallback"
                    && command.command == "readvar"
                    && command.target.as_deref() == Some("VAR_WEEKDAY")
            })
            .expect("Tuscany weekday read");
        assert_eq!(weekday.command_index, 4);
        let day_check = route29
            .script_variable_commands
            .iter()
            .find(|command| {
                command.source_script == "Route29CooltrainerMScript"
                    && command.command == "checktime"
                    && command.value_tokens == vec!["DAY"]
            })
            .expect("cooltrainer day check");
        assert_eq!(day_check.command_index, 2);

        let switches = data
            .map_module("GoldenrodUndergroundSwitchRoomEntrances")
            .expect("assemble underground switches");
        let setval = switches
            .script_variable_commands
            .iter()
            .find(|command| {
                command.source_script == "EmergencySwitchScript"
                    && command.command == "setval"
                    && command.value_tokens == vec!["7"]
            })
            .expect("emergency switch setval");
        assert_eq!(setval.command_index, 8);
        let write = switches
            .script_variable_commands
            .iter()
            .find(|command| {
                command.source_script == "EmergencySwitchScript"
                    && command.command == "writemem"
                    && command.target.as_deref() == Some("wUndergroundSwitchPositions")
            })
            .expect("emergency switch writemem");
        assert_eq!(write.command_index, 9);
    }

    #[test]
    fn map_module_extracts_script_control_commands_with_exact_targets() {
        let root = repository_root_for_tests();
        let data = AssetRoot::new(root)
            .load_base_game_data()
            .expect("load base game data");

        let blackthorn = data
            .map_module("BlackthornCity")
            .expect("assemble BlackthornCity module");
        let santos = blackthorn
            .script_control_commands
            .iter()
            .find(|command| {
                command.source_script == "BlackthornCitySantosCallback"
                    && command.command == "ifequal"
                    && command.compare_value.as_deref() == Some("SATURDAY")
                    && command.target_label.as_deref() == Some(".SantosAppears")
            })
            .expect("Santos local branch");
        assert_eq!(santos.command_index, 1);
        assert_eq!(
            santos.resolved_target_script.as_deref(),
            Some(".SantosAppears@BlackthornCitySantosCallback")
        );

        let route44 = data.map_module("Route44").expect("assemble Route44 module");
        let nested = route44
            .script_control_commands
            .iter()
            .find(|command| {
                command.source_script == ".WantsBattle@TrainerBirdKeeperVance1"
                    && command.command == "ifequal"
                    && command.compare_value.as_deref() == Some("2")
                    && command.target_label.as_deref() == Some(".Fight2")
            })
            .expect("nested local branch resolves to parent script");
        assert_eq!(
            nested.resolved_target_script.as_deref(),
            Some(".Fight2@TrainerBirdKeeperVance1")
        );

        let call = route44
            .script_control_commands
            .iter()
            .find(|command| {
                command.source_script == "TrainerBirdKeeperVance1"
                    && command.command == "scall"
                    && command.target_label.as_deref() == Some("Route44AskNumber1M")
            })
            .expect("Route44 scall");
        assert_eq!(
            call.resolved_target_script.as_deref(),
            Some("Route44AskNumber1M")
        );

        let gym = data
            .map_module("EcruteakGym")
            .expect("assemble EcruteakGym module");
        let standard = gym
            .script_control_commands
            .iter()
            .find(|command| {
                command.source_script == "EcruteakGymStatue"
                    && command.command == "jumpstd"
                    && command.target_label.as_deref() == Some("GymStatue1Script")
            })
            .expect("gym statue jumpstd");
        assert_eq!(standard.resolved_target_script, None);

        assert!(route44.script_control_commands.iter().any(|command| {
            command.source_script == "TrainerBirdKeeperVance1"
                && command.command == "endifjustbattled"
                && command.target_label.is_none()
                && command.compare_value.is_none()
        }));
    }

    #[test]
    fn map_module_extracts_object_commands_and_applies_exact_mutations() {
        let root = repository_root_for_tests();
        let data = AssetRoot::new(root)
            .load_base_game_data()
            .expect("load base game data");
        let module = data
            .map_module("IndigoPlateauPokecenter1F")
            .expect("assemble IndigoPlateauPokecenter1F module");
        let moveobject = module
            .script_object_commands
            .iter()
            .find(|command| {
                command.command == "moveobject"
                    && command.object_id.as_deref() == Some("INDIGOPLATEAUPOKECENTER1F_RIVAL")
            })
            .expect("rival moveobject command")
            .clone();
        let appear = module
            .script_object_commands
            .iter()
            .find(|command| {
                command.command == "appear"
                    && command.object_id.as_deref() == Some("INDIGOPLATEAUPOKECENTER1F_RIVAL")
            })
            .expect("rival appear command")
            .clone();
        let disappear = module
            .script_object_commands
            .iter()
            .find(|command| {
                command.command == "disappear"
                    && command.object_id.as_deref() == Some("INDIGOPLATEAUPOKECENTER1F_RIVAL")
            })
            .expect("rival disappear command")
            .clone();
        let applymovement = module
            .script_object_commands
            .iter()
            .find(|command| {
                command.command == "applymovement"
                    && command.object_id.as_deref() == Some("INDIGOPLATEAUPOKECENTER1F_RIVAL")
                    && command.movement.as_deref() == Some("PlateauRivalMovement1")
            })
            .expect("rival applymovement command")
            .clone();
        let rival_movement = module
            .script_movements
            .iter()
            .find(|movement| movement.label == "PlateauRivalMovement1")
            .expect("rival movement script")
            .clone();
        let turn_player = module
            .script_object_commands
            .iter()
            .find(|command| {
                command.command == "turnobject"
                    && command.object_id.as_deref() == Some("PLAYER")
                    && command.source_script == "PlateauRivalBattle1"
            })
            .expect("player turnobject command");
        let emote_player = module
            .script_object_commands
            .iter()
            .find(|command| {
                command.command == "showemote"
                    && command.object_id.as_deref() == Some("PLAYER")
                    && command.source_script == "PlateauRivalBattle1"
            })
            .expect("player showemote command");

        assert_eq!((moveobject.x, moveobject.y), (Some(17), Some(9)));
        assert_eq!(rival_movement.steps.len(), 7);
        assert_eq!(rival_movement.steps[0].command, "step");
        assert_eq!(rival_movement.steps[0].direction.as_deref(), Some("UP"));
        assert_eq!(rival_movement.steps[5].command, "turn_head");
        assert_eq!(rival_movement.steps[5].direction.as_deref(), Some("LEFT"));
        assert_eq!(turn_player.direction.as_deref(), Some("DOWN"));
        assert_eq!(emote_player.emote.as_deref(), Some("EMOTE_SHOCK"));
        assert_eq!(emote_player.duration, Some(15));

        let mut session = OverworldSession::with_events_and_objects(
            data.overworld_map("IndigoPlateauPokecenter1F")
                .expect("load IndigoPlateauPokecenter1F map"),
            module.events.clone(),
            module.objects.clone(),
            TilesetCollision {
                metatiles: vec![MetatileCollision {
                    collision: [permissions::FLOOR; 4],
                }],
            },
            TilePosition::new(0, 0),
        );
        let mut state = GameState::default();

        let moved = apply_script_object_mutation(&mut state, &mut session, &moveobject)
            .expect("moveobject applies");
        assert_eq!((moved.x, moved.y), (Some(17), Some(9)));
        let rival = session
            .objects
            .iter()
            .find(|object| {
                object.object_identifier.as_deref() == Some("INDIGOPLATEAUPOKECENTER1F_RIVAL")
            })
            .expect("rival object after move");
        assert_eq!((rival.x, rival.y), (17, 9));

        let moved_by_script = apply_script_movement(&mut session, &applymovement, &rival_movement)
            .expect("applymovement moves rival");
        assert_eq!(moved_by_script.previous_tile, TilePosition::new(17, 9));
        assert_eq!(moved_by_script.tile, TilePosition::new(17, 4));
        let rival = session
            .objects
            .iter()
            .find(|object| {
                object.object_identifier.as_deref() == Some("INDIGOPLATEAUPOKECENTER1F_RIVAL")
            })
            .expect("rival object after movement");
        assert_eq!((rival.x, rival.y), (17, 4));
        assert_eq!(
            session
                .object_facings
                .get("INDIGOPLATEAUPOKECENTER1F_RIVAL"),
            Some(&Direction::Left)
        );

        apply_script_object_mutation(&mut state, &mut session, &disappear)
            .expect("disappear applies");
        let rival = session
            .objects
            .iter()
            .find(|object| {
                object.object_identifier.as_deref() == Some("INDIGOPLATEAUPOKECENTER1F_RIVAL")
            })
            .expect("rival object after disappear");
        assert!(!session.is_object_visible(rival));

        apply_script_object_mutation(&mut state, &mut session, &appear).expect("appear applies");
        let rival = session
            .objects
            .iter()
            .find(|object| {
                object.object_identifier.as_deref() == Some("INDIGOPLATEAUPOKECENTER1F_RIVAL")
            })
            .expect("rival object after appear");
        assert!(session.is_object_visible(rival));
    }

    #[test]
    fn map_module_extracts_fixed_facing_movement_without_turning_player() {
        let root = repository_root_for_tests();
        let data = AssetRoot::new(root)
            .load_base_game_data()
            .expect("load base game data");
        let module = data
            .map_module("EcruteakGym")
            .expect("assemble EcruteakGym module");
        let command = module
            .script_object_commands
            .iter()
            .find(|command| {
                command.command == "applymovement"
                    && command.object_id.as_deref() == Some("PLAYER")
                    && command.movement.as_deref() == Some("EcruteakGymPlayerSlowStepDownMovement")
            })
            .expect("player fixed-facing applymovement")
            .clone();
        let movement = module
            .script_movements
            .iter()
            .find(|movement| movement.label == "EcruteakGymPlayerSlowStepDownMovement")
            .expect("player fixed-facing movement")
            .clone();

        assert_eq!(movement.steps[0].command, "fix_facing");
        assert_eq!(movement.steps[1].command, "slow_step");
        assert_eq!(movement.steps[1].direction.as_deref(), Some("DOWN"));
        assert_eq!(movement.steps[2].command, "remove_fixed_facing");

        let mut session = OverworldSession::with_events_and_objects(
            data.overworld_map("EcruteakGym")
                .expect("load EcruteakGym map"),
            module.events.clone(),
            module.objects.clone(),
            TilesetCollision {
                metatiles: vec![MetatileCollision {
                    collision: [permissions::FLOOR; 4],
                }],
            },
            TilePosition::new(10, 10),
        );
        session.player.facing = Direction::Left;

        let outcome =
            apply_script_movement(&mut session, &command, &movement).expect("movement applies");

        assert_eq!(outcome.previous_tile, TilePosition::new(10, 10));
        assert_eq!(outcome.tile, TilePosition::new(10, 11));
        assert_eq!(session.player.tile, TilePosition::new(10, 11));
        assert_eq!(session.player.facing, Direction::Left);
        assert_eq!(outcome.facing, Direction::Left);
        assert_eq!(
            outcome.effects,
            vec![
                crystal_core::systems::script_objects::ScriptMovementEffect {
                    command: "fix_facing".to_string(),
                    index: 0,
                },
                crystal_core::systems::script_objects::ScriptMovementEffect {
                    command: "remove_fixed_facing".to_string(),
                    index: 2,
                },
            ]
        );
    }

    #[test]
    fn map_module_extracts_follow_and_last_talked_object_commands() {
        let root = repository_root_for_tests();
        let data = AssetRoot::new(root)
            .load_base_game_data()
            .expect("load base game data");
        let tower = data
            .map_module("BattleTower1F")
            .expect("assemble BattleTower1F module");
        let follow = tower
            .script_object_commands
            .iter()
            .find(|command| {
                command.command == "follow"
                    && command.source_script == "Script_WalkToBattleTowerElevator"
            })
            .expect("BattleTower follow command")
            .clone();
        let stopfollow = tower
            .script_object_commands
            .iter()
            .find(|command| {
                command.command == "stopfollow"
                    && command.source_script == "Script_WalkToBattleTowerElevator"
            })
            .expect("BattleTower stopfollow command")
            .clone();

        assert_eq!(
            follow.object_id.as_deref(),
            Some("BATTLETOWER1F_RECEPTIONIST")
        );
        assert_eq!(follow.target_object_id.as_deref(), Some("PLAYER"));

        let mut session = OverworldSession::with_events_and_objects(
            data.overworld_map("BattleTower1F")
                .expect("load BattleTower1F map"),
            tower.events.clone(),
            tower.objects.clone(),
            TilesetCollision {
                metatiles: vec![MetatileCollision {
                    collision: [permissions::FLOOR; 4],
                }],
            },
            TilePosition::new(0, 0),
        );
        let mut state = GameState::default();

        apply_script_object_mutation(&mut state, &mut session, &follow).expect("follow applies");
        assert_eq!(
            session.following,
            Some(crystal_core::world::session::OverworldFollowState {
                leader_object_id: "BATTLETOWER1F_RECEPTIONIST".to_string(),
                follower_object_id: "PLAYER".to_string(),
            })
        );
        apply_script_object_mutation(&mut state, &mut session, &stopfollow)
            .expect("stopfollow applies");
        assert_eq!(session.following, None);

        let pokecenter = data
            .map_module("Pokecenter2F")
            .expect("assemble Pokecenter2F module");
        let last_talked = pokecenter
            .script_object_commands
            .iter()
            .find(|command| {
                command.command == "applymovementlasttalked"
                    && command.source_script == "BattleTradeMobile_WalkIn"
            })
            .expect("applymovementlasttalked command");
        assert_eq!(
            last_talked.movement.as_deref(),
            Some("Pokecenter2FMobileMobileMovementData_ReceptionistWalksUpAndLeft_LookDown")
        );
        assert!(pokecenter.script_movements.iter().any(|movement| {
            movement.label
                == "Pokecenter2FMobileMobileMovementData_ReceptionistWalksUpAndLeft_LookDown"
        }));
    }

    #[test]
    fn map_module_extracts_runtime_commands_with_exact_tokens() {
        let root = repository_root_for_tests();
        let data = AssetRoot::new(root)
            .load_base_game_data()
            .expect("load base game data");
        let azalea = data
            .map_module("AzaleaTown")
            .expect("assemble AzaleaTown module");
        let special = azalea
            .script_runtime_commands
            .iter()
            .find(|command| {
                command.command == "special"
                    && command.source_script == "AzaleaTownRivalBattleScene1"
            })
            .expect("rival scene special command");
        assert_eq!(special.args, vec!["FadeOutMusic"]);
        let pause = azalea
            .script_runtime_commands
            .iter()
            .find(|command| {
                command.command == "pause" && command.source_script == "AzaleaTownRivalBattleScene1"
            })
            .expect("rival scene pause command");
        assert_eq!(pause.args, vec!["15"]);

        let gym = data
            .map_module("AzaleaGym")
            .expect("assemble AzaleaGym module");
        let trainer_name = gym
            .script_runtime_commands
            .iter()
            .find(|command| {
                command.command == "gettrainername" && command.source_script == "AzaleaGymStatue"
            })
            .expect("gym statue trainer name command");
        assert_eq!(
            trainer_name.args,
            vec!["STRING_BUFFER_4", "BUGSY", "BUGSY1"]
        );

        let vending = data
            .map_module("CeladonDeptStore6F")
            .expect("assemble CeladonDeptStore6F module");
        assert!(vending.script_runtime_commands.iter().any(|command| {
            command.command == "loadmenu"
                && command.args == vec![".MenuHeader"]
                && command.source_script == "CeladonDeptStore6FVendingMachine"
        }));
        assert!(vending.script_runtime_commands.iter().any(|command| {
            command.command == "verticalmenu"
                && command.args.is_empty()
                && command.source_script == "CeladonDeptStore6FVendingMachine"
        }));
        assert!(vending.script_runtime_commands.iter().any(|command| {
            command.command == "menu_coords"
                && command.args == vec!["0", "2", "SCREEN_WIDTH - 1", "TEXTBOX_Y - 1"]
        }));

        let bills_family = data
            .map_module("BillsFamilysHouse")
            .expect("assemble Bill family house");
        assert!(bills_family.script_runtime_commands.iter().any(|command| {
            command.command == "addcellnum"
                && command.args == vec!["PHONE_BILL"]
                && command.source_script == "BillsYoungerSisterScript"
        }));

        let dragon_shrine = data
            .map_module("DragonShrine")
            .expect("assemble Dragon Shrine");
        assert!(dragon_shrine.script_runtime_commands.iter().any(|command| {
            command.command == "specialphonecall"
                && command.args == vec!["SPECIALCALL_MASTERBALL"]
                && command.source_script == "DragonShrineTakeTestScript"
        }));

        let route39 = data.map_module("Route39").expect("assemble Route39");
        assert!(route39.script_runtime_commands.iter().any(|command| {
            command.command == "checkpoke"
                && command.args == vec!["PIKACHU"]
                && command.source_script == "TrainerPokefanmDerek"
        }));

        let elms_lab = data.map_module("ElmsLab").expect("assemble Elm's Lab");
        assert!(elms_lab.script_runtime_commands.iter().any(|command| {
            command.command == "pokepic"
                && command.args == vec!["CYNDAQUIL"]
                && command.source_script == "CyndaquilPokeBallScript"
        }));
        assert!(elms_lab.script_runtime_commands.iter().any(|command| {
            command.command == "closepokepic"
                && command.args.is_empty()
                && command.source_script == "CyndaquilPokeBallScript"
        }));

        let emy = data
            .map_module("BlackthornEmysHouse")
            .expect("assemble Emy trade house");
        assert!(emy.script_runtime_commands.iter().any(|command| {
            command.command == "trade"
                && command.args == vec!["NPC_TRADE_EMY"]
                && command.source_script == "Emy"
        }));

        let blackthorn_gym = data
            .map_module("BlackthornGym2F")
            .expect("assemble Blackthorn Gym 2F");
        assert!(
            blackthorn_gym
                .script_runtime_commands
                .iter()
                .any(|command| {
                    command.command == "writecmdqueue"
                        && command.args == vec![".CommandQueue"]
                        && command.source_script == "BlackthornGym2FSetUpStoneTableCallback"
                })
        );
        assert!(
            blackthorn_gym
                .script_runtime_commands
                .iter()
                .any(|command| {
                    command.command == "cmdqueue"
                        && command.args == vec!["CMDQUEUE_STONETABLE", ".StoneTable"]
                        && command.source_script == "BlackthornGym2FSetUpStoneTableCallback"
                })
        );
        assert!(
            blackthorn_gym
                .script_runtime_commands
                .iter()
                .any(|command| {
                    command.command == "stonetable"
                        && command.args == vec!["5", "BLACKTHORNGYM2F_BOULDER1", ".Boulder1"]
                })
        );

        let elevator = data
            .map_module("CeladonDeptStoreElevator")
            .expect("assemble Celadon elevator");
        assert!(elevator.script_runtime_commands.iter().any(|command| {
            command.command == "elevator"
                && command.args == vec!["CeladonDeptStoreElevatorData"]
                && command.source_script == "CeladonDeptStoreElevatorScript"
        }));
        assert!(elevator.script_runtime_commands.iter().any(|command| {
            command.command == "elevfloor"
                && command.args == vec!["FLOOR_1F", "4", "CELADON_DEPT_STORE_1F"]
        }));

        let bedroom = data
            .map_module("PlayersHouse2F")
            .expect("assemble player's bedroom");
        assert!(bedroom.script_runtime_commands.iter().any(|command| {
            command.command == "describedecoration"
                && command.args == vec!["DECODESC_LEFT_DOLL"]
                && command.source_script == "PlayersHouseDoll1Script"
        }));
        assert!(bedroom.script_runtime_commands.iter().any(|command| {
            command.command == "conditional_event"
                && command.args == vec!["EVENT_PLAYERS_ROOM_POSTER", ".Script"]
                && command.source_script == "PlayersHousePosterScript"
        }));

        let route31 = data.map_module("Route31").expect("assemble Route31");
        assert!(route31.script_runtime_commands.iter().any(|command| {
            command.command == "checkpokemail"
                && command.args == vec!["ReceivedSpearowMailText"]
                && command.source_script == "Route31MailRecipientScript"
        }));
        let route35_gate = data
            .map_module("Route35GoldenrodGate")
            .expect("assemble Route35 Goldenrod gate");
        assert!(route35_gate.script_runtime_commands.iter().any(|command| {
            command.command == "givepokemail"
                && command.args == vec!["GiftSpearowMail"]
                && command.source_script == "RandyScript"
        }));

        let hallway = data
            .map_module("BattleTowerHallway")
            .expect("assemble Battle Tower hallway");
        assert!(hallway.script_runtime_commands.iter().any(|command| {
            command.command == "callasm"
                && command.args == vec![".asm_load_battle_room"]
                && command.source_script == "BattleTowerHallwayChooseBattleRoomScript"
        }));
        assert!(hallway.script_runtime_commands.iter().any(|command| {
            command.command == "ldh"
                && command.args == vec!["a", "[rWBK]"]
                && command.source_script == "BattleTowerHallwayChooseBattleRoomScript"
        }));
        assert!(hallway.script_runtime_commands.iter().any(|command| {
            command.command == "ret"
                && command.args.is_empty()
                && command.source_script == "BattleTowerHallwayChooseBattleRoomScript"
        }));

        let academy = data
            .map_module("EarlsPokemonAcademy")
            .expect("assemble Earl's academy");
        assert!(academy.script_runtime_commands.iter().any(|command| {
            command.command == "_2dmenu"
                && command.args.is_empty()
                && command.source_script == "AcademyBlackboard"
        }));
        assert!(academy.script_runtime_commands.iter().any(|command| {
            command.command == "dba"
                && command.args == vec![".Text"]
                && command.source_script == "AcademyBlackboard"
        }));

        let radio_tower = data
            .map_module("RadioTower2F")
            .expect("assemble Radio Tower 2F");
        assert!(radio_tower.script_runtime_commands.iter().any(|command| {
            command.command == "writevar"
                && command.args == vec!["VAR_BLUECARDBALANCE"]
                && command.source_script == "Buena"
        }));

        let route35_gate = data
            .map_module("Route35NationalParkGate")
            .expect("assemble Route35 National Park gate");
        assert!(route35_gate.script_runtime_commands.iter().any(|command| {
            command.command == "getnum"
                && command.args == vec!["STRING_BUFFER_3"]
                && command.source_script == "Route35NationalParkGateLeavingContestEarlyScript"
        }));
    }

    #[test]
    fn extracted_numeric_givepoke_materializes_exact_party_gift() {
        let root = repository_root_for_tests();
        let data = AssetRoot::new(root)
            .load_base_game_data()
            .expect("load base game data");
        let module = data.map_module("ElmsLab").expect("assemble ElmsLab module");
        let gift = module
            .gift_pokemon_scripts
            .iter()
            .find(|gift| gift.source_script == "CyndaquilPokeBallScript")
            .expect("Cyndaquil starter gift");
        let mut storage = crystal_core::models::PokemonStorage::default();

        let outcome = give_gift_pokemon(
            &mut storage,
            &data.pokemon,
            &data.learnsets,
            &data.moves,
            &data.growth_rates,
            &data.items,
            GiftPokemonRequest {
                species_id: gift.species_id.clone(),
                level: gift.level,
                held_item_id: gift.held_item_id.clone(),
                nickname: None,
                original_trainer_name: "PLAYER".to_string(),
                original_trainer_id: 1234,
                source_script: gift.source_script.clone(),
                command_index: gift.command_index,
                egg: gift.egg,
                dvs: Dv::from_non_hp(10, 10, 10, 10),
            },
        )
        .expect("materialize starter gift");

        assert_eq!(outcome.pokemon.species.id, "CYNDAQUIL");
        assert_eq!(outcome.pokemon.level, 5);
        assert_eq!(outcome.pokemon.item.as_deref(), Some("BERRY"));
        assert_eq!(storage.party.filled_slots(), 1);
    }

    #[test]
    fn map_module_extracts_money_script_commands_with_exact_tokens() {
        let root = repository_root_for_tests();
        let data = AssetRoot::new(root)
            .load_base_game_data()
            .expect("load base game data");
        let module = data
            .map_module("Route43Gate")
            .expect("assemble Route43Gate module");

        let check = module
            .script_economy_commands
            .iter()
            .find(|command| {
                command.source_script == "RocketScript_Southbound"
                    && command.command == "checkmoney"
            })
            .expect("Route43 toll checkmoney");

        assert_eq!(check.account.as_deref(), Some("YOUR_MONEY"));
        assert_eq!(check.amount_tokens, vec!["ROUTE43GATE_TOLL - 1"]);
        assert_eq!(check.command_index, 9);

        let mut state = GameState {
            money: 1_000,
            ..GameState::default()
        };
        assert_eq!(CurrencyCatalog::default().get("ROUTE43GATE_TOLL"), None);
        let constants = economy_constants(&data);
        assert_eq!(constants.get("ROUTE43GATE_TOLL"), Some(1_000));
        assert_eq!(constants.get("route43gate_toll"), None);
        let account = MoneyAccount::from_script_id(check.account.as_deref().expect("account"))
            .expect("exact account");
        let outcome = check_money(&state, account, &check.amount_tokens, &constants)
            .expect("check exact toll");
        assert_eq!(outcome.comparison, AmountComparison::HaveMore);
        assert_eq!(
            take_money(
                &mut state,
                account,
                &vec!["ROUTE43GATE_TOLL".to_string()],
                &constants
            ),
            Ok(0)
        );
    }

    #[test]
    fn parser_extracts_givemoney_from_exact_money_command_class() {
        let scripts: BTreeMap<String, Value> = [(
            "PrizeScript".to_string(),
            serde_json::json!([
                {
                    "command": "givemoney",
                    "args": ["MOMS_MONEY", "MAX_MONEY", "-", "1"]
                },
                {
                    "command": "GiveMoney",
                    "args": ["YOUR_MONEY", "1"]
                }
            ]),
        )]
        .into_iter()
        .collect();

        let commands =
            parse_script_economy_commands("PrizeMap", &scripts).expect("parse economy commands");

        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].command, "givemoney");
        assert_eq!(commands[0].account.as_deref(), Some("MOMS_MONEY"));
        assert_eq!(commands[0].amount_tokens, vec!["MAX_MONEY", "-", "1"]);
        assert_eq!(commands[0].source_script, "PrizeScript");
        assert_eq!(commands[0].command_index, 0);
    }

    #[test]
    fn parser_extracts_phone_commands_from_exact_command_classes() {
        let scripts: BTreeMap<String, Value> = [(
            "PhoneScript".to_string(),
            serde_json::json!([
                {
                    "command": "checkcellnum",
                    "args": ["PHONE_MOM"]
                },
                {
                    "command": "askforphonenumber",
                    "args": ["PHONE_JOEY"]
                },
                {
                    "command": "CheckCellNum",
                    "args": ["PHONE_ELM"]
                }
            ]),
        )]
        .into_iter()
        .collect();

        let commands =
            parse_script_phone_commands("PhoneMap", &scripts).expect("parse phone commands");

        assert_eq!(commands.len(), 2);
        assert_eq!(commands[0].command, "checkcellnum");
        assert_eq!(commands[0].contact_id, "PHONE_MOM");
        assert_eq!(commands[0].command_index, 0);
        assert_eq!(commands[1].command, "askforphonenumber");
        assert_eq!(commands[1].contact_id, "PHONE_JOEY");
        assert_eq!(commands[1].command_index, 1);
    }

    #[test]
    fn map_module_extracts_coin_script_commands_with_exact_tokens() {
        let root = repository_root_for_tests();
        let data = AssetRoot::new(root)
            .load_base_game_data()
            .expect("load base game data");
        let module = data
            .map_module("CeladonGameCornerPrizeRoom")
            .expect("assemble CeladonGameCornerPrizeRoom module");

        let check = module
            .script_economy_commands
            .iter()
            .find(|command| {
                command.source_script == ".Pikachu@CeladonGameCornerPrizeRoomPokemonVendor"
                    && command.command == "checkcoins"
            })
            .expect("Celadon Pokemon prize checkcoins");

        assert_eq!(check.account, None);
        assert_eq!(
            check.amount_tokens,
            vec!["CELADONGAMECORNERPRIZEROOM_PIKACHU_COINS"]
        );
        assert_eq!(check.command_index, 0);

        let state = GameState {
            coins: 2_222,
            ..GameState::default()
        };
        let constants = CurrencyCatalog(
            [(
                "CELADONGAMECORNERPRIZEROOM_PIKACHU_COINS".to_string(),
                2_222,
            )]
            .into_iter()
            .collect(),
        );
        let outcome =
            check_coins(&state, &check.amount_tokens, &constants).expect("check exact coins");
        assert_eq!(outcome.comparison, AmountComparison::HaveAmount);
    }

    #[test]
    fn map_module_extracts_shop_script_commands_with_exact_mart_ids() {
        let root = repository_root_for_tests();
        let data = AssetRoot::new(root)
            .load_base_game_data()
            .expect("load base game data");
        let cherrygrove = data
            .map_module("CherrygroveMart")
            .expect("assemble CherrygroveMart module");

        let pre_dex = cherrygrove
            .script_shop_commands
            .iter()
            .find(|command| {
                command.source_script == "CherrygroveMartClerkScript"
                    && command.mart_id == "MART_CHERRYGROVE"
            })
            .expect("Cherrygrove pre-dex mart");
        assert_eq!(pre_dex.mart_type, "MARTTYPE_STANDARD");
        assert_eq!(pre_dex.command_index, 3);

        let dex = cherrygrove
            .script_shop_commands
            .iter()
            .find(|command| command.source_script == ".PokeBallsInStock@CherrygroveMartClerkScript")
            .expect("Cherrygrove dex mart branch");
        assert_eq!(dex.mart_type, "MARTTYPE_STANDARD");
        assert_eq!(dex.mart_id, "MART_CHERRYGROVE_DEX");
        assert!(data.marts.0.contains_key(&dex.mart_id));

        let roof = data
            .map_module("GoldenrodDeptStoreRoof")
            .expect("assemble GoldenrodDeptStoreRoof module");
        let rooftop = roof
            .script_shop_commands
            .iter()
            .find(|command| command.source_script == "GoldenrodDeptStoreRoofClerkScript")
            .expect("Goldenrod rooftop mart");
        assert_eq!(rooftop.mart_type, "MARTTYPE_ROOFTOP");
        assert_eq!(rooftop.mart_id, "0");
    }

    #[test]
    fn map_module_extracts_script_field_pickups_with_exact_ids() {
        let root = repository_root_for_tests();
        let data = AssetRoot::new(root)
            .load_base_game_data()
            .expect("load base game data");
        let route29 = data.map_module("Route29").expect("assemble Route29 module");
        let potion = route29
            .script_field_pickups
            .iter()
            .find(|pickup| pickup.source_script == "Route29Potion")
            .expect("Route29 itemball pickup");

        assert_eq!(potion.command, "itemball");
        assert_eq!(potion.item_id.as_deref(), Some("POTION"));
        assert_eq!(potion.quantity, 1);
        assert_eq!(potion.event_flag.as_deref(), Some("EVENT_ROUTE_29_POTION"));
        assert_eq!(
            potion.to_field_item_pickup().expect("executable pickup"),
            FieldItemPickup {
                item_id: "POTION".to_string(),
                quantity: 1,
                event_flag: "EVENT_ROUTE_29_POTION".to_string(),
                source: FieldItemSource::ItemBall,
            }
        );

        let fruit = route29
            .script_field_pickups
            .iter()
            .find(|pickup| pickup.source_script == "Route29FruitTree")
            .expect("Route29 fruit tree");
        assert_eq!(fruit.command, "fruittree");
        assert_eq!(fruit.item_id, None);
        assert_eq!(fruit.event_flag, None);
        assert_eq!(fruit.fruit_tree_id.as_deref(), Some("FRUITTREE_ROUTE_29"));
        assert_eq!(
            fruit
                .to_fruit_tree_pickup(&data.fruit_trees)
                .expect("executable fruit tree"),
            FieldItemPickup {
                item_id: "BERRY".to_string(),
                quantity: 1,
                event_flag: "FRUITTREE_ROUTE_29_COLLECTED".to_string(),
                source: FieldItemSource::FruitTree,
            }
        );

        let route13 = data.map_module("Route13").expect("assemble Route13 module");
        let hidden = route13
            .script_field_pickups
            .iter()
            .find(|pickup| pickup.source_script == "Route13HiddenCalcium")
            .expect("Route13 hidden item");
        assert_eq!(hidden.command, "hiddenitem");
        assert_eq!(hidden.item_id.as_deref(), Some("CALCIUM"));
        assert_eq!(
            hidden.event_flag.as_deref(),
            Some("EVENT_ROUTE_13_HIDDEN_CALCIUM")
        );
    }

    #[test]
    fn route29_itemball_pickup_uses_exact_pack_item_and_object_flag() {
        let root = repository_root_for_tests();
        let data = AssetRoot::new(root)
            .load_base_game_data()
            .expect("load base game data");
        let module = data.map_module("Route29").expect("assemble Route29 module");
        let object = module
            .objects
            .iter()
            .find(|object| object.object_identifier.as_deref() == Some("ROUTE29_POKE_BALL"))
            .expect("Route29 itemball object");
        let mut state = GameState::default();

        let outcome = pickup_field_item(
            &mut state,
            &data.items,
            FieldItemPickup {
                item_id: "POTION".to_string(),
                quantity: 1,
                event_flag: object.event_flag.clone(),
                source: FieldItemSource::ItemBall,
            },
        )
        .expect("pick up potion");

        assert_eq!(
            outcome,
            FieldItemPickupOutcome::Collected {
                item_id: "POTION".to_string(),
                quantity: 1,
                event_flag: "EVENT_ROUTE_29_POTION".to_string(),
                source: FieldItemSource::ItemBall,
            }
        );
        assert_eq!(state.bag.items["POTION"], 1);
        assert_eq!(
            state.flags.is_event_flag_set("EVENT_ROUTE_29_POTION"),
            Ok(true)
        );
    }

    #[test]
    fn route29_fruit_tree_pickup_uses_exact_pack_catalog_without_default_item() {
        let root = repository_root_for_tests();
        let data = AssetRoot::new(root)
            .load_base_game_data()
            .expect("load base game data");
        let route29 = data.map_module("Route29").expect("assemble Route29 module");
        let pickup = route29
            .script_field_pickups
            .iter()
            .find(|pickup| pickup.source_script == "Route29FruitTree")
            .expect("Route29 fruit tree")
            .clone();
        let mut state = GameState::default();

        let outcome = pickup_script_field_item(&mut state, &data.items, &data.fruit_trees, pickup)
            .expect("fruit tree pickup");

        assert_eq!(
            outcome,
            FieldItemPickupOutcome::Collected {
                item_id: "BERRY".to_string(),
                quantity: 1,
                event_flag: "FRUITTREE_ROUTE_29_COLLECTED".to_string(),
                source: FieldItemSource::FruitTree,
            }
        );
        assert_eq!(state.bag.items["BERRY"], 1);
    }

    #[test]
    fn johto_tileset_collision_loads_from_controlled_runtime_assets() {
        let root = repository_root_for_tests();
        let tileset = AssetRoot::new(root)
            .load_tileset_collision("johto")
            .expect("load johto collision");

        assert!(tileset.metatiles.len() > 100);
        assert_eq!(
            tileset.metatiles[17].collision,
            [
                permissions::WALL,
                permissions::WALL,
                permissions::WALL,
                permissions::WALL
            ]
        );
        assert_eq!(resolve_collision_token("ICE").expect("resolve ice"), 0x23);
        assert_eq!(
            resolve_collision_token("ICE_2B").expect("resolve alternate ice"),
            0x2b
        );
        assert_eq!(
            resolve_collision_token("WATERFALL").expect("resolve waterfall"),
            permissions::WATERFALL
        );
        assert_eq!(
            resolve_collision_token("HOP_UP_LEFT").expect("resolve ledge"),
            permissions::HOP_UP_LEFT
        );
        assert_eq!(
            resolve_collision_token("5B").expect("resolve garbage collision"),
            0x5b
        );
        assert_eq!(
            resolve_collision_token("unknown")
                .expect_err("unknown token")
                .to_string(),
            "unknown collision token unknown"
        );
    }

    #[test]
    fn route29_overworld_session_steps_with_real_map_and_tileset_data() {
        let root = repository_root_for_tests();
        let asset_root = AssetRoot::new(root);
        let data = asset_root
            .load_base_game_data()
            .expect("load base game data");
        let map = data.overworld_map("Route29").expect("assemble route map");
        let tileset = asset_root
            .load_tileset_collision("johto")
            .expect("load johto collision");
        let start = (0..map.height as i16 * 2)
            .flat_map(|y| (0..map.width as i16 * 2 - 2).map(move |x| TilePosition::new(x, y)))
            .find(|tile| {
                can_enter_tile(
                    &map,
                    &tileset,
                    TilePosition::new(tile.x + 2, tile.y),
                    Direction::Right,
                    PlayerTraversalState::Walk,
                )
            })
            .expect("walkable Route29 rightward step");
        let mut session = OverworldSession::new(map, tileset, start);

        let outcome = session.step(
            Direction::Right,
            StepOptions {
                force_step_after_turn: true,
                ..StepOptions::default()
            },
        );

        assert_eq!(
            outcome,
            StepOutcome::Moved {
                from: start,
                to: TilePosition::new(start.x + 2, start.y),
                speed_multiplier: 1,
            }
        );
        assert_eq!(session.snapshot().map_name, "Route29");
        assert_eq!(session.snapshot().frame, 1);
        assert_ne!(session.state_hash(), 0);
    }

    #[test]
    fn route29_overworld_session_rolls_pack_backed_wild_encounter() {
        let root = repository_root_for_tests();
        let asset_root = AssetRoot::new(root);
        let data = asset_root
            .load_base_game_data()
            .expect("load base game data");
        let map = data.overworld_map("Route29").expect("assemble route map");
        let tileset = asset_root
            .load_tileset_collision("johto")
            .expect("load johto collision");
        let grass_tile = (0..map.height as i16 * 2)
            .flat_map(|y| (0..map.width as i16 * 2).map(move |x| TilePosition::new(x, y)))
            .find(|tile| {
                sample_collision(&map, &tileset, *tile)
                    .map(|sample| sample.permission == permissions::TALL_GRASS)
                    .unwrap_or(false)
            })
            .expect("Route29 tall grass tile");
        let encounters = data
            .wild_encounters
            .get("Route29")
            .expect("Route29 wild encounters");
        let session = OverworldSession::new(map, tileset, grass_tile);
        let mut selected_seed = None;
        let mut selected_roll = None;
        for seed in 1..10_000 {
            let mut rng = Random::new(seed);
            let roll = session
                .check_wild_encounter(
                    encounters,
                    &data.encounter_slot_tables,
                    &data.encounter_music_modifiers,
                    &mut rng,
                    EncounterCheckOptions {
                        time: TimeOfDay::Day,
                        music_token: None,
                        has_cleanse_tag: false,
                    },
                )
                .expect("Route29 encounter roll")
                .expect("Route29 grass roll");
            if roll.resolved.is_some() {
                selected_seed = Some(seed);
                selected_roll = Some(roll);
                break;
            }
        }
        let roll = selected_roll.expect("deterministic Route29 encounter seed");

        assert_eq!(selected_seed, Some(20));
        assert_eq!(roll.map_name, "Route29");
        assert_eq!(roll.tile, grass_tile);
        assert_eq!(roll.surface, EncounterSurface::Grass);
        assert_eq!(roll.time, TimeOfDay::Day);
        assert_eq!(roll.threshold, 25);
        let mut battle_rng = Random::new(42);
        let battle = data
            .wild_battle_start(roll.clone(), &mut battle_rng)
            .expect("Route29 wild battle start");

        let resolved = roll.resolved.expect("resolved Route29 encounter");
        assert_eq!(resolved.encounter.species, "PIDGEY");
        assert_eq!(resolved.level, 4);
        assert_eq!(battle.battle_type, "BATTLETYPE_NORMAL");
        assert_eq!(battle.enemy_pokemon.species.id, "PIDGEY");
        assert_eq!(battle.enemy_pokemon.level, 4);
        assert_eq!(battle.enemy_pokemon.original_trainer_name, "WILD");
        assert_eq!(battle.enemy_party, vec![battle.enemy_pokemon.clone()]);
        assert_eq!(battle.enemy_pokemon.moves[0].name, "TACKLE");
        assert_eq!(battle.rng_seed_after, battle_rng.seed());
    }

    #[test]
    fn route29_overworld_session_reports_pack_backed_warp_event() {
        let root = repository_root_for_tests();
        let asset_root = AssetRoot::new(root);
        let data = asset_root
            .load_base_game_data()
            .expect("load base game data");
        let module = data.map_module("Route29").expect("assemble route module");
        let map = data.overworld_map("Route29").expect("assemble route map");
        let tileset = asset_root
            .load_tileset_collision("johto")
            .expect("load johto collision");
        let warp = module.events.warps[0].clone();
        let warp_tile = warp_tile_position(&warp);
        let mut session =
            OverworldSession::with_events(map, module.events.clone(), tileset, warp_tile);

        let trigger = session.check_warp().expect("Route29 warp trigger");
        assert_eq!(trigger.map_name, "Route29");
        assert_eq!(trigger.tile, warp_tile);
        assert_eq!(trigger.warp.index, warp.index);
        assert_eq!(trigger.warp.target_map_constant, "ROUTE_29_ROUTE_46_GATE");
        assert_eq!(trigger.warp.target_map, "ROUTE_29_ROUTE_46_GATE");
        assert_eq!(trigger.warp.target_warp_id, 3);

        let result = session.step_and_check_warp(
            Direction::Left,
            StepOptions {
                force_step_after_turn: true,
                ..StepOptions::default()
            },
        );
        assert_eq!(
            result.outcome,
            StepOutcome::Blocked {
                at: TilePosition::new(warp_tile.x - 2, warp_tile.y),
                facing: Direction::Left,
            }
        );
        assert_eq!(result.warp.expect("warp remains active").tile, warp_tile);
    }

    #[test]
    fn route29_warp_transition_resolves_destination_from_pack_constants() {
        let root = repository_root_for_tests();
        let asset_root = AssetRoot::new(root);
        let data = asset_root
            .load_base_game_data()
            .expect("load base game data");
        let module = data.map_module("Route29").expect("assemble route module");
        let map = data.overworld_map("Route29").expect("assemble route map");
        let tileset = asset_root
            .load_tileset_collision("johto")
            .expect("load johto collision");
        let warp = module.events.warps[0].clone();
        let start = warp_tile_position(&warp);
        let session = OverworldSession::with_events(map, module.events.clone(), tileset, start);
        let trigger = session.check_warp().expect("Route29 warp trigger");

        let transition = data
            .resolve_warp_transition(&trigger)
            .expect("resolve Route29 warp destination");

        assert_eq!(transition.trigger, trigger);
        assert_eq!(transition.destination.map_name, "Route29Route46Gate");
        assert_eq!(transition.destination.warp.index, 3);
        assert_eq!(transition.destination.warp.target_map_constant, "ROUTE_29");
        assert_eq!(
            transition.destination.tile,
            warp_tile_position(&transition.destination.warp)
        );

        let destination_module = data
            .map_module(&transition.destination.map_name)
            .expect("load destination module");
        let destination_map = data
            .overworld_map(&transition.destination.map_name)
            .expect("load destination map");
        let destination_tileset = asset_root
            .load_tileset_collision(&destination_module.attributes.tileset_name)
            .expect("load destination tileset");
        let destination_session = transition.apply_to(
            destination_map,
            destination_module.events,
            destination_module.objects,
            destination_tileset,
            session.frame + 1,
        );

        assert_eq!(
            destination_session.snapshot().map_name,
            "Route29Route46Gate"
        );
        assert_eq!(
            destination_session.snapshot().tile,
            transition.destination.tile
        );
        assert_eq!(destination_session.snapshot().frame, 1);
    }

    #[test]
    fn warp_transition_requires_declared_target_map_constant() {
        let root = repository_root_for_tests();
        let data = AssetRoot::new(root)
            .load_base_game_data()
            .expect("load base game data");
        let trigger = WarpTrigger {
            map_name: "Route29".to_string(),
            tile: TilePosition::new(55, 3),
            warp: WarpEvent {
                index: 1,
                x: 27,
                y: 1,
                target_map_constant: "MISSING_TARGET_MAP".to_string(),
                target_map: "MissingTargetMap".to_string(),
                target_warp_id: 1,
            },
        };

        let error = data
            .resolve_warp_transition(&trigger)
            .expect_err("missing target map constant");

        assert!(
            error
                .to_string()
                .contains("unknown target map constant 'MISSING_TARGET_MAP'")
        );
    }

    #[test]
    fn route29_east_connection_resolves_destination_from_pack_attributes() {
        let root = repository_root_for_tests();
        let asset_root = AssetRoot::new(root);
        let data = asset_root
            .load_base_game_data()
            .expect("load base game data");
        let module = data.map_module("Route29").expect("assemble route module");
        let map = data.overworld_map("Route29").expect("assemble route map");
        let tileset = asset_root
            .load_tileset_collision("johto")
            .expect("load johto collision");
        let mut session =
            OverworldSession::with_events(map, module.events, tileset, TilePosition::new(58, 5));

        let outcome = session.step(
            Direction::Right,
            StepOptions {
                force_step_after_turn: true,
                ..StepOptions::default()
            },
        );
        assert_eq!(
            outcome,
            StepOutcome::Moved {
                from: TilePosition::new(58, 5),
                to: TilePosition::new(60, 5),
                speed_multiplier: 1,
            }
        );

        let trigger = session.check_connection().expect("Route29 east connection");
        assert_eq!(trigger.connection.direction, "east");
        assert_eq!(trigger.connection.target_map, "NewBarkTown");

        let transition = data
            .resolve_connection_transition(&trigger)
            .expect("resolve Route29 east connection");

        assert_eq!(transition.trigger, trigger);
        assert_eq!(transition.destination.map_name, "NewBarkTown");
        assert_eq!(transition.destination.tile, TilePosition::new(1, 5));

        let destination_module = data
            .map_module(&transition.destination.map_name)
            .expect("load destination module");
        let destination_map = data
            .overworld_map(&transition.destination.map_name)
            .expect("load destination map");
        let destination_tileset = asset_root
            .load_tileset_collision(&destination_module.attributes.tileset_name)
            .expect("load destination tileset");
        session = transition.apply_to(
            destination_map,
            destination_module.events,
            destination_module.objects,
            destination_tileset,
            session.frame,
        );

        assert_eq!(session.snapshot().map_name, "NewBarkTown");
        assert_eq!(session.snapshot().tile, TilePosition::new(1, 5));
        assert_eq!(session.snapshot().frame, 1);
    }

    #[test]
    fn connection_transition_requires_declared_target_map_attributes() {
        let root = repository_root_for_tests();
        let data = AssetRoot::new(root)
            .load_base_game_data()
            .expect("load base game data");
        let trigger = ConnectionTrigger {
            map_name: "Route29".to_string(),
            tile: TilePosition::new(60, 5),
            connection: MapConnection {
                direction: "east".to_string(),
                target_map: "MissingTarget".to_string(),
                offset: 0,
            },
        };

        let error = data
            .resolve_connection_transition(&trigger)
            .expect_err("missing connection target");

        assert!(
            error
                .to_string()
                .contains("connection target 'MissingTarget' missing attributes")
        );
    }

    #[test]
    fn connection_transition_does_not_fallback_to_target_map_module_attributes() {
        let mut data = GameDataSet::default();
        data.maps.insert(
            "Target".to_string(),
            test_map_module("Target", "TARGET", None),
        );
        let trigger = ConnectionTrigger {
            map_name: "Source".to_string(),
            tile: TilePosition::new(1, 1),
            connection: MapConnection {
                direction: "east".to_string(),
                target_map: "Target".to_string(),
                offset: 0,
            },
        };

        let error = data
            .resolve_connection_transition(&trigger)
            .expect_err("connection transition requires the map_attributes section");

        assert!(
            error
                .to_string()
                .contains("connection target 'Target' missing attributes"),
            "{error}"
        );
    }

    #[test]
    fn connection_transition_rejects_out_of_bounds_destination_without_clamping() {
        let mut data = GameDataSet::default();
        data.map_attributes.insert(
            "Target".to_string(),
            MapAttributes {
                tileset_name: "johto".to_string(),
                border_block: 5,
                width: 2,
                height: 2,
                connections: Vec::new(),
                time_of_day: None,
                phone_service: 0,
                phone_flag: false,
                environment: None,
                location: None,
                music: None,
                palette: None,
                fishing_group: None,
                map_constant: Some("TARGET".to_string()),
                map_group_constant: None,
                blocks_label: None,
                map_scripts_label: None,
                map_events_label: None,
                connection_flags: None,
            },
        );
        let trigger = ConnectionTrigger {
            map_name: "Source".to_string(),
            tile: TilePosition::new(60, 99),
            connection: MapConnection {
                direction: "east".to_string(),
                target_map: "Target".to_string(),
                offset: 0,
            },
        };

        let error = data
            .resolve_connection_transition(&trigger)
            .expect_err("out-of-bounds destination must be rejected");

        assert!(
            error
                .to_string()
                .contains("connection destination tile (1, 99) is outside target map")
        );
    }

    #[test]
    fn overworld_map_requires_explicit_blocks_label_and_payload() {
        let mut data = GameDataSet::default();
        data.map_attributes.insert(
            "MissingBlocks".to_string(),
            MapAttributes {
                tileset_name: "johto".to_string(),
                border_block: 5,
                width: 1,
                height: 1,
                connections: Vec::new(),
                time_of_day: None,
                phone_service: 0,
                phone_flag: false,
                environment: None,
                location: None,
                music: None,
                palette: None,
                fishing_group: None,
                map_constant: Some("MISSING_BLOCKS".to_string()),
                map_group_constant: None,
                blocks_label: None,
                map_scripts_label: None,
                map_events_label: None,
                connection_flags: None,
            },
        );

        let error = data
            .overworld_map("MissingBlocks")
            .expect_err("blocks label is required");

        assert!(error.to_string().contains("missing blocks_label"));
    }

    #[test]
    fn map_block_base64_decoder_rejects_invalid_payloads() {
        assert_eq!(decode_base64_bytes("AQID").expect("decode"), vec![1, 2, 3]);
        assert!(decode_base64_bytes("AQI").is_err());
        assert!(decode_base64_bytes("AQ@D").is_err());
    }

    #[test]
    fn modpack_overlay_adds_and_replaces_wild_encounters_by_map_name() {
        let mut data = GameDataSet::default();
        let route = WildEncounterData {
            map_name: "NEW_ROUTE".to_string(),
            grass_rates: Some([("day".to_string(), 20)].into_iter().collect()),
            grass: Some(WildEncounterTable {
                day: vec![WildEncounter {
                    level: 3,
                    species: "NEW_MON".to_string(),
                }],
                ..WildEncounterTable::default()
            }),
            ..WildEncounterData::default()
        };
        let replacement = WildEncounterData {
            grass: Some(WildEncounterTable {
                day: vec![WildEncounter {
                    level: 5,
                    species: "BULBASAUR".to_string(),
                }],
                ..WildEncounterTable::default()
            }),
            ..route.clone()
        };
        let manifest = ModpackManifest {
            payload: ModpackPayload {
                wild_encounters: vec![route, replacement],
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };

        data.apply_modpack(&manifest)
            .expect("manifest should apply with exact exported ids");

        let encounters = data
            .wild_encounters
            .get("NEW_ROUTE")
            .expect("wild encounter table");
        let slots = table_for_surface(encounters, EncounterSurface::Grass, TimeOfDay::Day)
            .expect("overlay day grass table");
        assert_eq!(data.wild_encounters.len(), 1);
        assert_eq!(slots[0].species, "BULBASAUR");
        assert_eq!(slots[0].level, 5);
    }

    #[test]
    fn modpack_items_require_explicit_script_name() {
        let item = Item {
            name: "Flash Step Charm".to_string(),
            description: "A modded item.".to_string(),
            effect: "NONE".to_string(),
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
            escape_rope_mode: None,
            price: 100,
            held_effect: "HELD_NONE".to_string(),
            parameter: 0,
            property: String::new(),
            pocket: item_pocket("ITEM"),
            field_menu: String::new(),
            field_usable: true,
            battle_menu: String::new(),
            battle_usable: true,
            script_name: String::new(),
            consumable: false,
            tmhm_index: None,
            tmhm_move: None,
        };
        let manifest = ModpackManifest {
            payload: ModpackPayload {
                items: vec![item],
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };
        let mut data = GameDataSet::default();

        let error = data
            .apply_modpack(&manifest)
            .expect_err("missing item ids must not be derived from display names");

        assert!(
            error
                .to_string()
                .contains("item 'Flash Step Charm' is missing explicit script_name")
        );
    }

    fn explicit_empty_manifest_json() -> Value {
        serde_json::to_value(ModpackManifest {
            metadata: ModpackMetadata {
                id: "empty-pack".to_string(),
                name: "Empty Pack".to_string(),
                version: "1.0.0".to_string(),
                author: None,
                description: None,
            },
            ..ModpackManifest::default()
        })
        .expect("serialize complete empty manifest")
    }

    #[test]
    fn modpack_manifest_json_requires_explicit_top_level_metadata() {
        let mut missing_payload = explicit_empty_manifest_json();
        missing_payload
            .as_object_mut()
            .expect("manifest object")
            .remove("payload");
        let error = serde_json::from_value::<ModpackManifest>(missing_payload)
            .expect_err("missing payload must not default to an empty pack")
            .to_string();
        assert!(error.contains("missing field `payload`"), "{error}");

        let mut missing_author = explicit_empty_manifest_json();
        missing_author
            .get_mut("metadata")
            .expect("metadata")
            .as_object_mut()
            .expect("metadata object")
            .remove("author");
        let error = serde_json::from_value::<ModpackManifest>(missing_author)
            .expect_err("nullable author must be explicit")
            .to_string();
        assert!(error.contains("missing field `author`"), "{error}");
    }

    #[test]
    fn modpack_manifest_json_requires_explicit_payload_categories() {
        let mut missing_audio = explicit_empty_manifest_json();
        missing_audio
            .get_mut("payload")
            .expect("payload")
            .as_object_mut()
            .expect("payload object")
            .remove("audio");
        let error = serde_json::from_value::<ModpackManifest>(missing_audio)
            .expect_err("missing payload category must not default to empty")
            .to_string();
        assert!(error.contains("missing field `audio`"), "{error}");
    }

    #[test]
    fn modpack_manifest_json_rejects_unknown_fields() {
        let mut manifest = explicit_empty_manifest_json();
        manifest["metadata"]["displayName"] = Value::String("Coerced Name".to_string());
        let error = serde_json::from_value::<ModpackManifest>(manifest)
            .expect_err("unknown metadata fields must not be ignored")
            .to_string();
        assert!(error.contains("unknown field `displayName`"), "{error}");

        let mut manifest = explicit_empty_manifest_json();
        manifest["payload"]["wildPokemon"] = serde_json::json!([]);
        let error = serde_json::from_value::<ModpackManifest>(manifest)
            .expect_err("unknown payload fields must not be ignored")
            .to_string();
        assert!(error.contains("unknown field `wildPokemon`"), "{error}");

        let mut manifest = explicit_empty_manifest_json();
        manifest["fallback"] = Value::Bool(true);
        let error = serde_json::from_value::<ModpackManifest>(manifest)
            .expect_err("unknown manifest fields must not be ignored")
            .to_string();
        assert!(error.contains("unknown field `fallback`"), "{error}");
    }

    #[test]
    fn content_pack_index_json_rejects_unknown_fields() {
        let index = serde_json::json!({
            "version": 1,
            "packs": [
                {
                    "id": "bad-pack",
                    "enabled": true,
                    "priority": 0,
                    "path": "content-packs/bad-pack",
                    "compiled": null,
                    "fallback": true,
                    "files": ContentPackFiles::default()
                }
            ]
        });

        let error = serde_json::from_value::<ContentPackIndex>(index)
            .expect_err("unknown content pack fields must not be ignored")
            .to_string();
        assert!(error.contains("unknown field `fallback`"), "{error}");
    }

    #[test]
    fn compiler_report_json_rejects_unknown_fields() {
        let edge_error = serde_json::from_value::<PlayabilityGraphEdge>(serde_json::json!({
            "from": "NewBarkTown",
            "to": "Route29",
            "kind": "walk",
            "fallback": true
        }))
        .expect_err("playability graph edges must not accept fallback metadata")
        .to_string();
        assert!(
            edge_error.contains("unknown field `fallback`"),
            "{edge_error}"
        );

        let diagnostic_error = serde_json::from_value::<VerificationError>(serde_json::json!({
            "severity": "error",
            "code": "unknown_map",
            "subject": "Route29",
            "message": "missing map",
            "normalizedSubject": "route29"
        }))
        .expect_err("verification diagnostics must not accept normalized aliases")
        .to_string();
        assert!(
            diagnostic_error.contains("unknown field `normalizedSubject`"),
            "{diagnostic_error}"
        );
    }

    #[test]
    fn content_pack_paths_reject_legacy_prefix_absolute_and_traversal() {
        let asset_root = AssetRoot::new(repository_root_for_tests());

        let legacy = resolve_content_pack_data_path(
            &asset_root,
            "bad-pack",
            "assets/data/content-packs/bad/pokemon/a.json",
        )
        .expect_err("content pack paths must not accept assets/data-prefixed aliases")
        .to_string();
        assert!(
            legacy.contains("must not include the assets/data prefix"),
            "{legacy}"
        );

        let absolute = resolve_content_pack_data_path(
            &asset_root,
            "bad-pack",
            "/tmp/content-packs/bad/pokemon/a.json",
        )
        .expect_err("content pack paths must not be absolute")
        .to_string();
        assert!(
            absolute.contains("must be relative to assets/data"),
            "{absolute}"
        );

        let traversal =
            resolve_content_pack_data_path(&asset_root, "bad-pack", "content-packs/../bad.json")
                .expect_err("content pack paths must not traverse parent directories")
                .to_string();
        assert!(
            traversal.contains("must not traverse parent directories"),
            "{traversal}"
        );

        let canonical =
            resolve_content_pack_data_path(&asset_root, "good-pack", "content-packs/good/a.json")
                .expect("canonical content pack path");
        assert!(canonical.ends_with("apps/web/assets/data/content-packs/good/a.json"));
    }

    #[test]
    fn runtime_data_paths_reject_aliases_absolute_and_traversal() {
        let asset_root = AssetRoot::new(repository_root_for_tests());

        let legacy = asset_root
            .resolve_data_path("assets/data/content-packs/core-modular/music/MUSIC_ROUTE_29.mid")
            .expect_err("runtime paths must not accept assets/data aliases")
            .to_string();
        assert!(
            legacy.contains("must not include the assets/data prefix"),
            "{legacy}"
        );

        let absolute = asset_root
            .resolve_data_path("/tmp/content-packs/core-modular/music/MUSIC_ROUTE_29.mid")
            .expect_err("runtime paths must not be absolute")
            .to_string();
        assert!(
            absolute.contains("must be relative to assets/data"),
            "{absolute}"
        );

        let traversal = asset_root
            .resolve_data_path("content-packs/core-modular/../bad.mid")
            .expect_err("runtime paths must not traverse")
            .to_string();
        assert!(
            traversal.contains("must not traverse parent directories"),
            "{traversal}"
        );

        let canonical = asset_root
            .resolve_data_path("content-packs/core-modular/music/MUSIC_ROUTE_29.mid")
            .expect("canonical runtime data path");
        assert!(
            canonical.ends_with(
                "apps/web/assets/data/content-packs/core-modular/music/MUSIC_ROUTE_29.mid"
            )
        );
    }

    #[test]
    fn game_data_set_json_requires_explicit_sections() {
        let mut data = serde_json::to_value(GameDataSet::default())
            .expect("serialize complete empty game data");
        data.as_object_mut()
            .expect("game data object")
            .remove("audio");

        let error = serde_json::from_value::<GameDataSet>(data)
            .expect_err("missing game data sections must not default to empty")
            .to_string();
        assert!(error.contains("missing field `audio`"), "{error}");
    }

    #[test]
    fn pokedex_entry_json_requires_explicit_pages() {
        let mut entry = serde_json::json!({
            "species":"BULBASAUR",
            "classification":"SEED",
            "heightDigits":204,
            "weightDigits":150,
            "pages":["A strange seed was planted on its back."]
        });
        entry
            .as_object_mut()
            .expect("pokedex entry object")
            .remove("pages");

        let error = serde_json::from_value::<RuntimePokedexEntry>(entry)
            .expect_err("missing pages must not default to an empty entry")
            .to_string();
        assert!(error.contains("missing field `pages`"), "{error}");
    }

    #[test]
    fn raw_script_command_json_requires_explicit_args() {
        let error = serde_json::from_value::<Vec<ScriptCommand>>(serde_json::json!([{
            "command":"end"
        }]))
        .expect_err("raw script commands must export args explicitly")
        .to_string();

        assert!(error.contains("missing field `args`"), "{error}");
    }

    #[test]
    fn modpack_item_json_requires_explicit_effect_without_defaulting_to_none() {
        let mut manifest = explicit_empty_manifest_json();
        manifest["metadata"]["id"] = Value::String("bad-items".to_string());
        manifest["metadata"]["name"] = Value::String("Bad Items".to_string());
        manifest["payload"]["items"] = serde_json::json!([{
            "name":"Flash Step Charm",
            "description":"A malformed modded item.",
            "price":100,
            "held_effect":"HELD_NONE",
            "parameter":0,
            "property":"",
            "pocket":"ITEM",
            "field_menu":"",
            "battle_menu":"",
            "script_name":"FLASH_STEP_CHARM",
            "tmhm_index":null
        }]);

        let error = serde_json::from_value::<ModpackManifest>(manifest)
            .expect_err("missing item effect must not default to NONE")
            .to_string();

        assert!(error.contains("missing field `effect`"), "{error}");
    }

    #[test]
    fn missing_core_pack_index_is_an_error_not_a_fallback() {
        let root = std::env::temp_dir().join(format!(
            "crystal-assets-missing-pack-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("apps/web/assets/data/content-packs"))
            .expect("create temp asset root");

        let error = AssetRoot::new(&root)
            .load_base_game_data()
            .expect_err("missing core pack index should fail");

        assert!(error.to_string().contains("content-packs/index.json"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn creates_pokemon_from_loaded_existing_json() {
        let root = repository_root_for_tests();
        let data = AssetRoot::new(root)
            .load_base_game_data()
            .expect("load base game data");

        let pokemon = data
            .create_pokemon("BULBASAUR", 10, Dv::from_non_hp(10, 10, 10, 10))
            .expect("create bulbasaur");

        assert_eq!(pokemon.nickname, "BULBASAUR");
        assert_eq!(pokemon.level, 10);
        assert_eq!(
            pokemon
                .moves
                .iter()
                .map(|learned| learned.name.as_str())
                .collect::<Vec<_>>(),
            vec!["TACKLE", "GROWL", "LEECH_SEED", "VINE_WHIP"]
        );
        assert_eq!(pokemon.moves[0].current_pp, data.moves["TACKLE"].pp);
        assert_eq!(pokemon.experience, 560);
        assert_eq!(pokemon.happiness, 70);
    }

    fn repository_root_for_tests() -> PathBuf {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        manifest_dir
            .ancestors()
            .nth(3)
            .expect("workspace is nested under rust/crates/crystal-assets")
            .to_path_buf()
    }
}
