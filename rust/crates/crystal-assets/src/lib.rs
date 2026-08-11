use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::ffi::OsStr;
use std::path::{Component, Path, PathBuf};
use std::rc::Rc;

use anyhow::{Context, Result};
use crystal_core::battle::capture::{
    CaptureAttemptContext, CaptureBallRule, CaptureBallRuleIssue, CaptureCompletion,
    CaptureOutcome, CaptureRulesIssue, CaptureWobbleProbability, CaptureWobbleProbabilityIssue,
    StoredCapture, capture_rules_issues, capture_wobble_probability_issues,
    complete_active_wild_capture_result as core_complete_active_wild_capture,
    resolve_capture_attempt as core_resolve_capture_attempt,
    throw_ball_from_bag as core_throw_ball_from_bag, validate_capture_ball_item,
};
use crystal_core::battle::damage::{
    TypeCategories, TypeCategoryIssue, TypeEffectivenessTable, TypeEffectivenessTableIssue,
    TypeEffectivenessTableKind, WeatherModifierIssue, WeatherModifiers, type_category_issues,
    type_effectiveness_table_issues, weather_modifier_issues,
};
use crystal_core::battle::start::require_active_battle_for_state_item;
use crystal_core::battle::start::{
    BattleStatDropGuardOutcome, StaticWildBattleOrigin, StaticWildBattleRequest,
    StaticWildBattleStart, TrainerBattleAdvanceOutcome, TrainerBattleCompletion,
    TrainerBattleCompletionOutcome, TrainerBattleRequest, TrainerBattleStartStatus,
    WildBattleStart, activate_static_wild_battle_start, activate_trainer_battle_start_status,
    activate_wild_battle_start,
    advance_active_trainer_battle as core_advance_active_trainer_battle,
    apply_battle_stat_drop_guard_turns, complete_trainer_battle as core_complete_trainer_battle,
    deactivate_battle_after_loss, first_available_battle_party_index,
    materialize_non_roaming_wild_battle_with_rng, materialize_roaming_wild_battle_with_rng,
    materialize_trainer_party, require_active_battle_enemy_party_index,
    require_active_battle_party_index, static_wild_battle_start, switch_active_battle_party_index,
    trainer_battle_start, wild_battle_start_from_encounter,
};
use crystal_core::battle::stats::{
    BattleStatMultiplier, BattleStatMultiplierTableIssue, BattleStatMultiplierTables,
    battle_stat_multiplier_table_issues,
};
use crystal_core::battle::turn::{
    BattleAction, BattleCombatState, BattleEscapeTrapState, BattleEvent, BattleSide,
    BattleTurnInput, BattleTurnOutcome, MovePriorityTable, MovePriorityTableIssue,
    battle_move_effect_is_supported, commit_battle_turn_outcome, commit_wild_battle_escape_attempt,
    move_priority_table_issues,
    resolve_battle_enemy_action_with_items as core_resolve_battle_enemy_action_with_items,
    resolve_battle_turn_with_items as core_resolve_battle_turn_with_items,
    resolve_wild_battle_run as core_resolve_wild_battle_run,
    resolve_wild_battle_turn_with_items as core_resolve_wild_battle_turn_with_items,
};
use crystal_core::input::{B_PAD_A, GameButton, JoypadState, direction_from_pad_mask};
use crystal_core::map::{
    BackgroundEvent, CoordEvent, MapAttributes, MapConnection, MapEventSectionCommand,
    MapEventSectionCommandIssue, MapEvents, MapScene, MapSceneTable, MapScriptSectionCommand,
    MapScriptSectionCommandIssue, ObjectEvent, WarpEvent, map_event_section_command_arg_counts,
    map_event_section_command_issues, map_script_section_command_arg_counts,
    map_script_section_command_issues,
};
pub use crystal_core::models::RuntimePokedexEntry;
#[cfg(test)]
use crystal_core::models::TrainerPartyPokemon;
use crystal_core::models::{
    Bag, BattleAnimationCatalogIssue, Dv, FrontpicAnimCatalogIssue, FrontpicAnimCommand,
    FrontpicAnimCommandIssue, FrontpicAnimProgram, ITEM_POCKET_BALL, ITEM_POCKET_ITEM,
    ITEM_POCKET_KEY_ITEM, ITEM_POCKET_TM_HM, Item, LearnedMove, MAX_BOX_MONS, MAX_PC_BOXES,
    MenuIconCatalogIssue, Move, MoveNameCatalogIssue, MovePayloadIssue, Party, PcBox,
    PcStringCatalogIssue, PokedexEntryCatalogIssue, PokedexState, PokegearLandmarkIssue,
    PokegearTownMapPaletteIssue, Pokemon, PokemonSpecies, PokemonStorage, RuntimeBundleIssue,
    SpritePaletteDefaultIssue, Trainer, TrainerCatalog, TrainerCatalogIssue,
    battle_animation_catalog_issues, calculate_stats, create_pokemon_from_known_dvs,
    frontpic_anim_catalog_issues, frontpic_anim_command_issue, menu_icon_catalog_issues,
    move_name_catalog_issues, move_payload_issues, pc_string_catalog_issues,
    pokedex_entry_catalog_issues, pokegear_landmark_issues, pokegear_town_map_palette_issues,
    pokemon_species_display_name, runtime_bundle_issues, sprite_palette_default_issues,
    trainer_catalog_issues, trainer_key, validate_saved_bag_pocket_references,
    validate_saved_pokedex_references,
};
pub use crystal_core::models::{PokegearLandmark, PokegearLandmarksPayload};
use crystal_core::multiplayer::{
    PlayerId, RuntimeCommandFrame, RuntimeCommandFrameError, RuntimeCommandPayload,
    RuntimeCommandResultFrame, StateChecksum, StateChecksumFrame, fnv1a32_bytes,
    game_state_checksum, game_state_checksum_unchecked,
};
use crystal_core::random::{CrystalRandom, DividerSource, Random, ReplayDivider};
use crystal_core::save::SaveModpackIdentity;
use crystal_core::state::{
    BattleMemory, BattleTowerState, BuenasPasswordState, BugContestState, DayCareState,
    EventFlagMemory, FishingMemory, GameState, ItemUseRuntimeEvent, LINK_MODE_COLOSSEUM,
    LinkSerialConnectionStatus, LinkSessionState, MagikarpRecordState, MysteryGiftState, Options,
    OverworldMemory, OverworldObjectMapMemory, PendingMoveLearn, RoamingPokemonState,
    SavedTrainerBattleFields, SceneMemory, ScriptAudioRuntimeEvent, ScriptAudioRuntimeKind,
    ScriptControlRuntimeEvent, ScriptControlRuntimeKind, ScriptEndState,
    ScriptGraphicsRuntimeEvent, ScriptLocation, ScriptMapLoadRequest, ScriptMapRefreshRequest,
    ScriptMapRuntimeEvent, ScriptMoneyRuntimeEvent, ScriptMusicFade, ScriptReturnFrame,
    ScriptRuntimeAsmDirective, ScriptRuntimeDecorationDescription, ScriptRuntimeDelay,
    ScriptRuntimeEarthquake, ScriptRuntimeEffect, ScriptRuntimeElevatorFloor, ScriptRuntimeEmote,
    ScriptRuntimeNumericBufferWrite, ScriptRuntimeQueuedCommand, ScriptRuntimeStoneTableEntry,
    ScriptRuntimeVariableWrite, ScriptScreenFade, ScriptShopRuntimeEvent, ScriptTextRuntimeEvent,
    ScriptTextWait, ScriptWarpRequest, ScriptYesNoPrompt, SwarmMemory,
    saved_audio_runtime_event_command_args, saved_map_runtime_event_command_args,
    saved_pending_text_wait_command_args, saved_script_end_command,
    saved_text_runtime_event_command_args,
    validate_saved_audio_reference as core_validate_saved_audio_reference,
    validate_saved_block_overrides as core_validate_saved_block_overrides,
    validate_saved_bug_contest_references as core_validate_saved_bug_contest_references,
    validate_saved_catalog_reference, validate_saved_compiled_script_command_name_reference,
    validate_saved_compiled_script_command_payload_reference,
    validate_saved_compiled_script_command_reference,
    validate_saved_compiled_script_return_reference, validate_saved_control_runtime_event_shape,
    validate_saved_day_care_references as core_validate_saved_day_care_references,
    validate_saved_engine_flag_reference as core_validate_saved_engine_flag_reference,
    validate_saved_event_flag_reference as core_validate_saved_event_flag_reference,
    validate_saved_exact_catalog_reference,
    validate_saved_flag_references as core_validate_saved_flag_references,
    validate_saved_graphics_runtime_event_shape,
    validate_saved_last_talked_object_reference as core_validate_saved_last_talked_object_reference,
    validate_saved_link_session_references as core_validate_saved_link_session_references,
    validate_saved_map_object_reference as core_validate_saved_map_object_reference,
    validate_saved_map_reference as core_validate_saved_map_reference,
    validate_saved_money_runtime_event_shape,
    validate_saved_mystery_gift_references as core_validate_saved_mystery_gift_references,
    validate_saved_object_overrides as core_validate_saved_object_overrides,
    validate_saved_optional_catalog_reference,
    validate_saved_overworld_references as core_validate_saved_overworld_references,
    validate_saved_pending_screen_fade_shape, validate_saved_player_gender,
    validate_saved_pokemon_party_references as core_validate_saved_pokemon_party_references,
    validate_saved_pokemon_reference as core_validate_saved_pokemon_reference,
    validate_saved_roaming_references as core_validate_saved_roaming_references,
    validate_saved_scene_references as core_validate_saved_scene_references,
    validate_saved_static_wild_battle_origin_reference as core_validate_saved_static_wild_battle_origin_reference,
    validate_saved_storage_references as core_validate_saved_storage_references,
    validate_saved_trainer_battle_request_field as core_validate_saved_trainer_battle_request_field,
    validate_saved_trainer_battle_request_fields, validate_saved_trainer_battle_source_reference,
    validate_saved_trainer_enemy_party_identity,
    validate_saved_warp_reference as core_validate_saved_warp_reference,
    validate_saved_wild_battle_origin_reference as core_validate_saved_wild_battle_origin_reference,
};
use crystal_core::systems::battle_escape::{
    BattleEscapeAttempt, BattleEscapeRules, BattleEscapeRulesIssue, battle_escape_rules_issues,
};
use crystal_core::systems::battle_items::{
    BattleItemOutcome, ItemPayloadIssue, ItemReferenceIssue, PartyItemOutcome,
    apply_active_battle_item_effect as core_apply_active_battle_item_effect,
    apply_battle_escape_item_use, apply_battle_pp_item_effect as core_apply_battle_pp_item_effect,
    apply_party_special_item_effect as core_apply_party_special_item_effect,
    apply_party_wide_item_effect as core_apply_party_wide_item_effect,
    clone_active_battle_party_pokemon, clone_field_party, clone_field_party_pokemon,
    item_payload_issues_with_known_field_rules, item_reference_issues,
    require_active_battle_party_pokemon_mut, require_field_party_mut,
    require_field_party_pokemon_mut, require_wild_battle_for_escape_item,
    validate_battle_escape_item, validate_battle_stat_drop_guard_item,
};
use crystal_core::systems::battle_rewards::{
    BattleRewardError, BattleRewardOutcome, BattleRewardRules, BattleRewardRulesIssue,
    PendingMoveLearnResolution, battle_reward_rules_issues,
    claim_active_trainer_battle_rewards as core_claim_active_trainer_battle_rewards,
    claim_active_wild_battle_rewards as core_claim_active_wild_battle_rewards,
    decline_pending_move_learn as core_decline_pending_move_learn, promote_next_pending_move_learn,
    rebase_pending_move_learns_for_party,
    replace_pending_move_learn as core_replace_pending_move_learn,
    sync_active_combat_player_party_from_storage,
};
use crystal_core::systems::economy::{
    CurrencyCatalog, SCRIPT_COIN_CHECK_COMMANDS, SCRIPT_COIN_MUTATION_COMMANDS,
    SCRIPT_MONEY_CHECK_COMMANDS, SCRIPT_MONEY_MUTATION_COMMANDS, ScriptEconomyCommand,
    ScriptEconomyCommandIssue, ScriptEconomyOutcome,
    apply_script_economy_command as core_apply_script_economy_command,
    script_economy_command_issues, validate_save_currency_for_runtime_pack,
};
use crystal_core::systems::evolution::{
    EvolutionContext, EvolutionEntry, EvolutionReport, EvolutionTable, EvolutionTableIssue,
    LinkMode, TRADE_ANY_ITEM, check_and_evolve, evolution_table_issues, is_known_evolution_method,
    is_known_happiness_window, is_known_stat_evolution_ratio,
};
use crystal_core::systems::experience::{GrowthRateCatalogIssue, growth_rate_catalog_issues};
use crystal_core::systems::field_items::{
    FieldItemPickupOutcome, FruitTreeCatalog, FruitTreeCatalogIssue,
    ItemfinderHiddenItem as CoreItemfinderHiddenItem, SCRIPT_FIELD_FRUIT_TREE_PICKUP_COMMANDS,
    SCRIPT_FIELD_HIDDEN_ITEM_PICKUP_COMMANDS, SCRIPT_FIELD_ITEM_PICKUP_COMMANDS,
    SCRIPT_FIELD_ITEMBALL_PICKUP_COMMANDS, ScriptFieldPickup, ScriptFieldPickupIssue,
    find_itemfinder_hidden_item as core_find_itemfinder_hidden_item, fruit_tree_catalog_issues,
    fruit_tree_collected_flag, pickup_script_field_item as core_pickup_script_field_item,
    script_field_pickup_issues,
};
use crystal_core::systems::field_moves::{
    FieldEscapeItemRule, FieldItemRule, FieldMoveBlockOutcome, FieldMoveBlockRule,
    FieldMoveCatalog, FieldMoveCatalogIssue, FieldMoveError, FieldMoveFlagOutcome,
    FieldMoveFlagRule, FieldMoveMoveRule, FieldMoveRule, FieldMoveTravelOutcome,
    FieldMoveTravelRule, FieldMoveUseOutcome, SavedDigWarpDestination,
    apply_cut_field_move as core_apply_cut_field_move, apply_dig_warp_memory_for_transition,
    apply_flash_field_move as core_apply_flash_field_move, apply_repel_item_use,
    apply_strength_field_move as core_apply_strength_field_move,
    apply_surf_field_move as core_apply_surf_field_move,
    apply_waterfall_field_move as core_apply_waterfall_field_move,
    apply_whirlpool_field_move as core_apply_whirlpool_field_move, blue_card_balance,
    field_move_catalog_issues, is_bicycle_environment, is_dig_field_move_environment,
    is_escape_rope_environment, is_fly_source_environment, is_teleport_source_environment,
    resolve_squirtbottle_target, saved_dig_warp_destination as core_saved_dig_warp_destination,
    validate_bicycle_item, validate_blue_card_item, validate_coin_case_item,
    validate_dig_field_move as core_validate_dig_field_move,
    validate_direct_field_move_actor as core_validate_direct_field_move_actor,
    validate_field_escape_item, validate_fly_field_move as core_validate_fly_field_move,
    validate_itemfinder_item, validate_pokegear_item, validate_repel_item,
    validate_saved_active_repel_item as core_validate_saved_active_repel_item,
    validate_saved_blue_card_balance, validate_squirtbottle_item,
    validate_teleport_field_move as core_validate_teleport_field_move, validate_town_map_item,
};
pub use crystal_core::systems::flee_mons::FleeMonTables;
use crystal_core::systems::flee_mons::{FleeMonCatalogIssue, flee_mon_catalog_issues};
use crystal_core::systems::gift_pokemon::{
    GiftPokemonOutcome, GiftPokemonRequest, GiftPokemonScript, GiftPokemonScriptIssue, NO_ITEM,
    gift_pokemon_script_issues, grant_gift_pokemon_to_state as core_grant_gift_pokemon_to_state,
};
use crystal_core::systems::item_use::{
    ItemUseContext, ItemUseOutcome, ItemUseRequest, use_bag_item as core_use_bag_item,
};
use crystal_core::systems::learnsets::{
    LearnsetCatalogIssue, LearnsetEntry, SpeciesLearnsets, learnset_catalog_issues,
};
use crystal_core::systems::map_context::{
    SpawnMemoryUpdate, apply_map_music_context, apply_map_scene_context,
    apply_state_block_overrides, apply_state_object_overrides, commit_overworld_snapshot,
    sync_state_object_overrides,
};
use crystal_core::systems::phone::{
    PermanentPhoneNumberRule, PhoneContactCatalog, PhoneContactCatalogIssue, PhoneContactRecord,
    SCRIPT_PHONE_CHECK_COMMANDS, SCRIPT_PHONE_REGISTRATION_COMMANDS, ScriptPhoneCommand,
    ScriptPhoneCommandIssue, ScriptPhoneError, ScriptPhoneInputs, ScriptPhoneOutcome,
    apply_script_phone_command as core_apply_script_phone_command,
    initialize_permanent_phone_numbers as core_initialize_permanent_phone_numbers,
    phone_contact_catalog_issues, script_phone_command_issues,
};
use crystal_core::systems::runtime_pack::{
    RuntimePackPresenceIssue, RuntimePackSections, runtime_pack_presence_issues,
};
use crystal_core::systems::script_audio::{
    SCRIPT_AUDIO_CRY_COMMANDS, SCRIPT_AUDIO_MUSIC_COMMANDS, SCRIPT_AUDIO_MUSIC_FADE_COMMANDS,
    SCRIPT_AUDIO_NO_PAYLOAD_COMMANDS, SCRIPT_AUDIO_SOUND_EFFECT_COMMANDS, ScriptAudioCommand,
    ScriptAudioCommandIssue, ScriptAudioCue,
    apply_script_audio_command as core_apply_script_audio_command, script_audio_command_issues,
};
use crystal_core::systems::script_blocks::{
    ScriptBlockChange, ScriptBlockChangeIssue, ScriptBlockChangeOutcome,
    apply_script_block_change as core_apply_script_block_change, script_block_change_issues,
};
use crystal_core::systems::script_control::{
    ScriptControlAction, ScriptControlCommand, ScriptControlCommandIssue,
    apply_script_control_command as core_apply_script_control_command,
    script_control_command_issues,
};
use crystal_core::systems::script_flags::{
    ScriptFlagCheckOutcome, ScriptFlagCommand, ScriptFlagCommandIssue, ScriptFlagMutationOutcome,
    apply_script_flag_mutation as core_apply_script_flag_mutation,
    check_script_flag as core_check_script_flag, is_known_script_flag_command,
    script_flag_command_issues,
};
use crystal_core::systems::script_items::{
    ScriptItemAccess, ScriptItemAccessIssue, ScriptItemCheckOutcome, ScriptItemGrant,
    ScriptItemGrantIssue, ScriptItemGrantOutcome, ScriptItemTakeOutcome,
    check_script_item as core_check_script_item, grant_script_item as core_grant_script_item,
    script_item_access_issues, script_item_grant_issues, take_script_item as core_take_script_item,
};
use crystal_core::systems::script_objects::{
    SCRIPT_MOVEMENT_DIRECTION_COMMANDS, SCRIPT_MOVEMENT_NO_ARG_COMMANDS,
    SCRIPT_MOVEMENT_REQUIRED_DURATION_COMMANDS, SCRIPT_OBJECT_COORDINATE_COMMANDS,
    SCRIPT_OBJECT_DIRECT_MOVEMENT_COMMANDS, SCRIPT_OBJECT_DIRECTION_COMMANDS,
    SCRIPT_OBJECT_EMOTE_COMMANDS, SCRIPT_OBJECT_LAST_TALKED_MOVEMENT_COMMANDS,
    SCRIPT_OBJECT_MOVEMENT_COMMANDS, SCRIPT_OBJECT_NO_PAYLOAD_COMMANDS,
    SCRIPT_OBJECT_TARGET_COMMANDS, SCRIPT_OBJECT_VISIBILITY_COMMANDS, ScriptMovement,
    ScriptMovementOutcome, ScriptMovementStep, ScriptMovementStepIssue, ScriptObjectCommand,
    ScriptObjectCommandIssue, ScriptObjectMutationOutcome,
    apply_script_movement as core_apply_script_movement,
    apply_script_object_mutation as core_apply_script_object_mutation,
    is_hideable_object_event_flag, is_known_script_movement_command, script_movement_step_issues,
    script_movement_step_runtime_stride, script_object_command_issues,
};
pub use crystal_core::systems::script_runtime::{
    InitializeEventsConfig, StoryEventScriptConstants,
};
use crystal_core::systems::script_runtime::{
    InitializeEventsIssue, SCRIPT_RUNTIME_ITEM_FROM_MEMORY_ID, SCRIPT_RUNTIME_USE_SCRIPT_VAR_ID,
    ScriptRuntimeCommand, ScriptRuntimeCommandError, ScriptRuntimeCommandIssue,
    ScriptRuntimeCpuCondition, ScriptRuntimeInputs, ScriptRuntimeOutcome,
    ScriptRuntimeReferenceCatalog, StoryEventScriptConstantIssue, apply_initialize_events,
    apply_script_runtime_command_in_map as core_apply_script_runtime_command,
    commit_interaction_script_dispatch, initialize_events_issues, parse_menu_coord_token,
    parse_script_i32_token, script_runtime_command_arg_counts, script_runtime_command_issues,
    story_event_script_constant_issues, validate_script_runtime_command,
};
use crystal_core::systems::script_scenes::{
    SCRIPT_SCENE_CHECK_COMMANDS, SCRIPT_SCENE_CURRENT_MAP_MUTATION_COMMANDS,
    SCRIPT_SCENE_TARGET_MAP_MUTATION_COMMANDS, ScriptSceneCommand, ScriptSceneCommandIssue,
    ScriptSceneOutcome, apply_script_scene_command as core_apply_script_scene_command,
    script_scene_command_issues,
};
use crystal_core::systems::script_swarms::{
    ScriptSwarmCommand, ScriptSwarmCommandIssue, ScriptSwarmOutcome,
    apply_script_swarm_command as core_apply_script_swarm_command, script_swarm_command_issues,
};
use crystal_core::systems::script_text::{
    AsmTextCatalogIssue, SCRIPT_TEXT_LABEL_COMMANDS, SCRIPT_TEXT_NO_LABEL_COMMANDS,
    ScriptMenuCommand, ScriptMenuDefinition, ScriptMenuDefinitionIssue, ScriptTextAction,
    ScriptTextBody, ScriptTextBodyCommand, ScriptTextBodyIssue, ScriptTextCommand,
    ScriptTextCommandError, apply_script_text_command as core_apply_script_text_command,
    asm_text_catalog_issues, menu_definition_command_arg_counts,
    resolve_script_text_command as core_resolve_script_text_command, script_menu_definition_issues,
    script_text_body_issues, script_text_command_issues, text_body_command_arg_counts,
};
use crystal_core::systems::script_variables::{
    ScriptVariableCommand, ScriptVariableCommandIssue, ScriptVariableOutcome,
    apply_script_variable_command as core_apply_script_variable_command,
    script_variable_command_issues,
};
use crystal_core::systems::script_warps::{
    SCRIPT_MAP_FACING_WARP_COMMANDS, SCRIPT_MAP_NEW_LOAD_COMMANDS, SCRIPT_MAP_NO_PAYLOAD_COMMANDS,
    SCRIPT_MAP_REANCHOR_COMMANDS, SCRIPT_MAP_WARP_COMMANDS, ScriptMapAction, ScriptMapCommand,
    ScriptMapCommandError, apply_script_map_command as core_apply_script_map_command,
    apply_script_warp_arrival_facing, complete_pending_script_warp, parse_script_warp_facing,
    script_map_command_issues,
};
use crystal_core::systems::shop::{
    MartCatalog, MartCatalogIssue, SCRIPT_SHOP_COMMANDS, ScriptShopCommand, ScriptShopCommandIssue,
    ScriptShopOutcome, ShopError, ShopResult,
    apply_script_shop_command as core_apply_script_shop_command,
    buy_active_shop_item as core_buy_active_shop_item, close_active_shop as core_close_active_shop,
    mart_catalog_issues, script_shop_command_issues,
    sell_active_shop_item as core_sell_active_shop_item,
};
#[cfg(test)]
use crystal_core::systems::special_routines::BattleTowerBannedSpeciesRule;
pub use crystal_core::systems::special_routines::RuntimeSpawnPointRef as RuntimeSpawnPoint;
use crystal_core::systems::special_routines::{
    BattleTowerMonDefinition, BattleTowerRules, BattleTowerRulesIssue,
    BattleTowerTrainerDefinition, BuenaPasswordCategories, BuenaPasswordCategoryDefinition,
    BuenaPasswordCategoryIssue, BuenaPrizeDefinitionIssue, BuenaPrizeDefinitions, BugContestConfig,
    BugContestConfigIssue, DratiniMoveSetIssue, DratiniMoveSets, EXECUTABLE_SPECIAL_ROUTINES,
    HappinessData, HappinessDataIssue, KurtApricornRecipeIssue, KurtApricornRecipes,
    MagikarpLengthEntry, MagikarpLengthTableIssue, OakRatingEntry, OakRatingTableIssue,
    OddEggDefinition, OddEggDefinitionIssue, RoamingMapLocation, RoamingPokemonCatalog,
    RoamingPokemonCatalogIssue, RoamingPokemonInitWrite, RoamingPokemonRoute,
    RuntimeSpawnPointCatalogIssue, ShuckieGiftDefinition, ShuckieGiftIssue,
    SpecialRoutineCatalogIssue, SpecialRoutineContext, SpecialRoutineEffect, SpecialRoutineOutcome,
    apply_random_special_routine_with_context, apply_special_routine_with_context,
    battle_tower_rules_issues, buena_password_category_issues, buena_prize_definition_issues,
    bug_contest_config_issues, checked_runtime_spawn_expected_tile, dratini_move_set_issues,
    happiness_data_issues, is_known_special_routine, kurt_apricorn_recipe_issues,
    magikarp_length_table_issues, oak_rating_table_issues, odd_egg_definition_issues,
    resolve_bug_contest_caught_mon, roaming_pokemon_catalog_issues, runtime_spawn_expected_tile,
    runtime_spawn_point_catalog_issues, runtime_spawn_subtiles_are_valid,
    saved_battle_tower_state_is_active, saved_special_battle_type_builtin_routine,
    shuckie_gift_issues, special_routine_catalog_issues,
    validate_saved_battle_tower_state as core_validate_saved_battle_tower_state,
    validate_saved_buena_password_references, validate_saved_magikarp_record_references,
    validate_saved_pending_special_battle_type,
};
use crystal_core::systems::step_events::{
    StepEventResult, StepEventRules, StepEventRulesIssue,
    process_overworld_step as core_process_overworld_step, step_event_rules_issues,
};
use crystal_core::systems::time::{ClockTime, GameDate};
use crystal_core::systems::tmhm::{
    TmHmLearnOutcome, teach_tmhm_move as core_teach_tmhm_move, validate_saved_tmhm_references,
};
use crystal_core::world::collision::{
    MetatileCollision, PlayerTraversalState, Terrain, TilesetCollision, can_enter_tile,
    describe_collision, is_permission_passable, permissions, sample_collision,
    standard_interaction_script,
};
use crystal_core::world::encounters::{
    ENCOUNTER_TIME_KEYS, EncounterMusicModifierIssue, EncounterMusicModifiers, EncounterSlotChance,
    EncounterSlotTableIssue, EncounterSlotTables, EncounterSurface, FieldEncounterCatalogIssue,
    FieldEncounterData, FieldEncounterKind, ResolvedWildEncounter, RockMonEncounterOutcome,
    TimeOfDay, WildEncounter, WildEncounterCatalogIssue, WildEncounterData, WildEncounterTable,
    apply_surf_level_variance, encounter_music_modifier_issues, encounter_slot_table_issues,
    field_encounter_catalog_issues, resolve_rock_mon_encounter as core_resolve_rock_mon_encounter,
    roll_headbutt_encounter as core_roll_headbutt_encounter, select_wild_encounter,
    table_for_surface, wild_encounter_catalog_issues,
};
use crystal_core::world::fishing::{
    FISHING_RODS, FishingCatalog, FishingCatalogIssue, FishingGroup, FishingRolledSession,
    FishingSession, RodTable, do_fishing_from_rng as core_do_fishing_from_rng,
    fishing_battle_trigger, fishing_bite, fishing_catalog_issues, fishing_rod_for_item_id,
    validate_rod as core_validate_fishing_rod,
    validate_saved_fishing_references as core_validate_saved_fishing_references,
};
pub use crystal_core::world::map::RuntimeMapMetadata;
use crystal_core::world::map::{
    Direction, METATILE_WIDTH, OverworldMapData, RuntimeMapMetadataIssue, TilePosition,
    runtime_map_metadata_issues,
};
use crystal_core::world::movement::{
    DEFAULT_RUNTIME_TILE_STRIDE, LedgeJumpOutcome, MovementMode, PlayerMovementState, StepOptions,
    StepOutcome, checked_move_by_stride,
};
use crystal_core::world::session::{
    ConnectionDestination, ConnectionTransition, ConnectionTrigger, CoordEventTrigger,
    EncounterCheckOptions, ExactEncounterContext, OverworldInteraction, OverworldInteractionTarget,
    OverworldSession, OverworldSnapshot, WarpDestination, WarpTransition, WarpTrigger,
    WildEncounterRoll, leading_usable_party_level, object_event_initial_facing,
    raw_event_tile_to_runtime_tile_checked, runtime_tile_to_raw_event_tile,
};
use crystal_core::world::session::{
    background_event_tile_position_checked, warp_tile_position_checked,
};
use flate2::{Compression, write::GzEncoder};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

pub use crystal_core::battle::capture::CaptureRules;

pub mod modpack {
    pub use super::ScriptElevatorDefinition;
    pub use super::ScriptVerticalMenuDefinition;
    pub use crystal_core::battle::capture::{CaptureRules, CaptureWobbleProbability};
    pub use crystal_core::battle::damage::{
        TypeCategories, TypeEffectivenessTable, WeatherModifiers,
    };
    pub use crystal_core::battle::stats::{BattleStatMultiplier, BattleStatMultiplierTables};
    pub use crystal_core::battle::turn::{MovePriorityOverride, MovePriorityTable};
    pub use crystal_core::map::{MapEventSectionCommand, MapScriptSectionCommand};
    pub use crystal_core::systems::battle_rewards::BattleRewardRules;
    pub use crystal_core::systems::economy::{CurrencyCatalog, ScriptEconomyCommand};
    pub use crystal_core::systems::evolution::{EvolutionEntry, EvolutionTable};
    pub use crystal_core::systems::experience::{GrowthRateCatalog, GrowthRateCurve};
    pub use crystal_core::systems::field_items::{FruitTreeCatalog, ScriptFieldPickup};
    pub use crystal_core::systems::gift_pokemon::GiftPokemonScript;
    pub use crystal_core::systems::phone::{
        PermanentPhoneNumberRule, PhoneContactCatalog, PhoneContactRecord, ScriptPhoneCommand,
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

    #[cfg(any(test, feature = "test-fixtures"))]
    pub use super::write_compiled_game_pack_for_tests;
    pub use super::{
        COMPILED_GAME_PACK_EXTENSION, COMPILED_GAME_PACK_FORMAT_VERSION, CompiledGamePack,
        CompiledGamePackIdentity, CompiledModpack, ContentPack, ContentPackCategory,
        ContentPackFiles, ContentPackIndex, GameDataSet, LoadedCompiledGamePack, MapAccessRule,
        MapModule, ModpackAudioAsset, ModpackAudioKind, ModpackAudioLoopPolicy,
        ModpackAudioManifest, ModpackAudioManifestEntry, ModpackAudioPlaybackEntry,
        ModpackAudioPlaybackMode, ModpackAudioPlaybackPlan, ModpackAudioSource,
        ModpackCompileOptions, ModpackCompileReport, ModpackCompiler, ModpackManifest,
        ModpackMetadata, ModpackPayload, PlayabilityGraphEdge, PlayabilityRules, PlayabilityStart,
        ProgressionGrants, ProgressionRequirements, ProgressionRule,
        REQUIRED_VENDOR_RUNTIME_FILE_KEYS, VerificationError, VerificationSeverity,
        read_loaded_verified_compiled_game_pack, read_verified_compiled_game_pack,
        validate_compiled_runtime_files,
    };
    pub use crystal_core::models::{Trainer, TrainerCatalog};
    pub use crystal_core::systems::special_routines::{
        BattleTowerBannedSpeciesRule, BattleTowerRules, BuenaPasswordCategories,
        BuenaPasswordCategoryDefinition, BuenaPrizeDefinitions, BugContestConfig, DratiniMoveSets,
        HappinessChangeEntry, HappinessData, HappinessServiceOutcome, KurtApricornRecipes,
        MagikarpLengthEntry, OakRatingEntry, OddEggDefinition, RoamingMapLocation,
        RoamingPokemonCatalog, RoamingPokemonInitWrite, RoamingPokemonRoute, ShuckieGiftDefinition,
    };
    pub use crystal_core::systems::step_events::StepEventRules;
}

include!("content_pack.rs");
include!("map_modules.rs");
include!("runtime_pack.rs");
include!("verification.rs");
include!("runtime_commands.rs");
include!("game_data.rs");
include!("mutation_protocol.rs");
include!("merge.rs");
include!("script_parsing.rs");

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
