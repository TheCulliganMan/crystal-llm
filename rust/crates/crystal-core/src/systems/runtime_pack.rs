#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimePackPresenceIssue {
    MissingPokemon,
    MissingMoves,
    MissingGrowthRates,
    MissingLearnsets,
    MissingEvolutions,
    MissingCaptureRules,
    MissingCaptureWobbleProbabilities,
    MissingBattleStatMultipliers,
    MissingMovePriorities,
    MissingTypeCategories,
    MissingTypeEffectiveness,
    MissingWeatherModifiers,
    MissingBattleRewardRules,
    MissingBattleEscapeRules,
    MissingMarts,
    MissingCurrencyConstants,
    MissingStepEventRules,
    MissingFishingCatalog,
    MissingFruitTrees,
    MissingFieldMoves,
    MissingItems,
    MissingTrainers,
    MissingAudio,
    MissingMusicAudio,
    MissingSoundEffects,
    MissingCryAudio,
    MissingPokemonCries,
    MissingTilesets,
    MissingScripts,
    MissingMapGeometry,
    MissingMapObjects,
    MissingRuntimeMapMetadata,
    MissingRuntimeSpawnPoints,
    MissingMaps,
    MissingPcStrings,
    MissingMenuIcons,
    MissingPokedexEntries,
    MissingPokemonFrontpicAnimations,
    MissingMoveNames,
    MissingAsmText,
    MissingBattleAnimations,
    MissingBattleAnimationTable,
    MissingBattleAnimBundle,
    MissingSpriteAnimBundle,
    MissingSpritePaletteDefaults,
    MissingPokegearTownMapPalettes,
    MissingPokegearLandmarks,
    MissingPhoneContacts,
    MissingPermanentPhoneNumbers,
    MissingSpecialPhoneCalls,
    MissingPhoneScripts,
    MissingFleeMons,
    MissingBuenaPasswordCategories,
    MissingRoamingPokemon,
    MissingBuenaPrizes,
    MissingKurtApricornRecipes,
    MissingShuckieGift,
    MissingDratiniMoveSets,
    MissingBugContestConfig,
    MissingBattleTowerRules,
    MissingOakRatings,
    MissingOddEggDefinitions,
    MissingMagikarpLengths,
    MissingHappinessData,
    MissingInitializeEvents,
    MissingStoryEventScriptConstants,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RuntimePackSections {
    pub has_pokemon: bool,
    pub has_moves: bool,
    pub has_growth_rates: bool,
    pub has_learnsets: bool,
    pub has_evolutions: bool,
    pub has_capture_rules: bool,
    pub has_capture_wobble_probabilities: bool,
    pub has_battle_stat_multipliers: bool,
    pub has_move_priorities: bool,
    pub has_type_categories: bool,
    pub has_type_effectiveness: bool,
    pub has_weather_modifiers: bool,
    pub has_battle_reward_rules: bool,
    pub has_battle_escape_rules: bool,
    pub has_marts: bool,
    pub has_currency_constants: bool,
    pub has_step_event_rules: bool,
    pub has_fishing_catalog: bool,
    pub has_fruit_trees: bool,
    pub has_field_moves: bool,
    pub has_items: bool,
    pub has_trainers: bool,
    pub has_audio: bool,
    pub has_music_audio: bool,
    pub has_sound_effects: bool,
    pub has_cry_audio: bool,
    pub has_pokemon_cries: bool,
    pub has_tilesets: bool,
    pub has_scripts: bool,
    pub has_map_geometry: bool,
    pub has_map_objects: bool,
    pub has_runtime_map_metadata: bool,
    pub has_runtime_spawn_points: bool,
    pub has_maps: bool,
    pub has_pc_strings: bool,
    pub has_menu_icons: bool,
    pub has_pokedex_entries: bool,
    pub has_pokemon_frontpic_animations: bool,
    pub has_move_names: bool,
    pub has_asm_text: bool,
    pub has_battle_animations: bool,
    pub has_battle_animation_table: bool,
    pub has_battle_anim_bundle: bool,
    pub has_sprite_anim_bundle: bool,
    pub has_sprite_palette_defaults: bool,
    pub has_pokegear_town_map_palettes: bool,
    pub has_pokegear_landmarks: bool,
    pub has_phone_contacts: bool,
    pub has_permanent_phone_numbers: bool,
    pub has_special_phone_calls: bool,
    pub has_phone_scripts: bool,
    pub has_flee_mons: bool,
    pub has_buena_password_categories: bool,
    pub has_roaming_pokemon: bool,
    pub has_buena_prizes: bool,
    pub has_kurt_apricorn_recipes: bool,
    pub has_shuckie_gift: bool,
    pub has_dratini_move_sets: bool,
    pub has_bug_contest_config: bool,
    pub has_battle_tower_rules: bool,
    pub has_oak_ratings: bool,
    pub has_odd_egg_definitions: bool,
    pub has_magikarp_lengths: bool,
    pub has_happiness_data: bool,
    pub has_initialize_events: bool,
    pub has_story_event_script_constants: bool,
}

pub fn runtime_pack_presence_issues(
    sections: RuntimePackSections,
) -> Vec<RuntimePackPresenceIssue> {
    let mut issues = Vec::new();

    if !sections.has_pokemon {
        issues.push(RuntimePackPresenceIssue::MissingPokemon);
    }
    if !sections.has_moves {
        issues.push(RuntimePackPresenceIssue::MissingMoves);
    }
    if !sections.has_growth_rates {
        issues.push(RuntimePackPresenceIssue::MissingGrowthRates);
    }
    if !sections.has_learnsets {
        issues.push(RuntimePackPresenceIssue::MissingLearnsets);
    }
    if !sections.has_evolutions {
        issues.push(RuntimePackPresenceIssue::MissingEvolutions);
    }
    if !sections.has_capture_rules {
        issues.push(RuntimePackPresenceIssue::MissingCaptureRules);
    }
    if !sections.has_capture_wobble_probabilities {
        issues.push(RuntimePackPresenceIssue::MissingCaptureWobbleProbabilities);
    }
    if !sections.has_battle_stat_multipliers {
        issues.push(RuntimePackPresenceIssue::MissingBattleStatMultipliers);
    }
    if !sections.has_move_priorities {
        issues.push(RuntimePackPresenceIssue::MissingMovePriorities);
    }
    if !sections.has_type_categories {
        issues.push(RuntimePackPresenceIssue::MissingTypeCategories);
    }
    if !sections.has_type_effectiveness {
        issues.push(RuntimePackPresenceIssue::MissingTypeEffectiveness);
    }
    if !sections.has_weather_modifiers {
        issues.push(RuntimePackPresenceIssue::MissingWeatherModifiers);
    }
    if !sections.has_battle_reward_rules {
        issues.push(RuntimePackPresenceIssue::MissingBattleRewardRules);
    }
    if !sections.has_battle_escape_rules {
        issues.push(RuntimePackPresenceIssue::MissingBattleEscapeRules);
    }
    if !sections.has_marts {
        issues.push(RuntimePackPresenceIssue::MissingMarts);
    }
    if !sections.has_currency_constants {
        issues.push(RuntimePackPresenceIssue::MissingCurrencyConstants);
    }
    if !sections.has_step_event_rules {
        issues.push(RuntimePackPresenceIssue::MissingStepEventRules);
    }
    if !sections.has_fishing_catalog {
        issues.push(RuntimePackPresenceIssue::MissingFishingCatalog);
    }
    if !sections.has_fruit_trees {
        issues.push(RuntimePackPresenceIssue::MissingFruitTrees);
    }
    if !sections.has_field_moves {
        issues.push(RuntimePackPresenceIssue::MissingFieldMoves);
    }
    if !sections.has_items {
        issues.push(RuntimePackPresenceIssue::MissingItems);
    }
    if !sections.has_trainers {
        issues.push(RuntimePackPresenceIssue::MissingTrainers);
    }
    if !sections.has_audio {
        issues.push(RuntimePackPresenceIssue::MissingAudio);
    }
    if !sections.has_music_audio {
        issues.push(RuntimePackPresenceIssue::MissingMusicAudio);
    }
    if !sections.has_sound_effects {
        issues.push(RuntimePackPresenceIssue::MissingSoundEffects);
    }
    if !sections.has_cry_audio {
        issues.push(RuntimePackPresenceIssue::MissingCryAudio);
    }
    if !sections.has_pokemon_cries {
        issues.push(RuntimePackPresenceIssue::MissingPokemonCries);
    }
    if !sections.has_tilesets {
        issues.push(RuntimePackPresenceIssue::MissingTilesets);
    }
    if !sections.has_scripts {
        issues.push(RuntimePackPresenceIssue::MissingScripts);
    }
    if !sections.has_map_geometry {
        issues.push(RuntimePackPresenceIssue::MissingMapGeometry);
    }
    if !sections.has_map_objects {
        issues.push(RuntimePackPresenceIssue::MissingMapObjects);
    }
    if !sections.has_runtime_map_metadata {
        issues.push(RuntimePackPresenceIssue::MissingRuntimeMapMetadata);
    }
    if !sections.has_runtime_spawn_points {
        issues.push(RuntimePackPresenceIssue::MissingRuntimeSpawnPoints);
    }
    if !sections.has_maps {
        issues.push(RuntimePackPresenceIssue::MissingMaps);
    }
    if !sections.has_pc_strings {
        issues.push(RuntimePackPresenceIssue::MissingPcStrings);
    }
    if !sections.has_menu_icons {
        issues.push(RuntimePackPresenceIssue::MissingMenuIcons);
    }
    if !sections.has_pokedex_entries {
        issues.push(RuntimePackPresenceIssue::MissingPokedexEntries);
    }
    if !sections.has_pokemon_frontpic_animations {
        issues.push(RuntimePackPresenceIssue::MissingPokemonFrontpicAnimations);
    }
    if !sections.has_move_names {
        issues.push(RuntimePackPresenceIssue::MissingMoveNames);
    }
    if !sections.has_asm_text {
        issues.push(RuntimePackPresenceIssue::MissingAsmText);
    }
    if !sections.has_battle_animations {
        issues.push(RuntimePackPresenceIssue::MissingBattleAnimations);
    }
    if !sections.has_battle_animation_table {
        issues.push(RuntimePackPresenceIssue::MissingBattleAnimationTable);
    }
    if !sections.has_battle_anim_bundle {
        issues.push(RuntimePackPresenceIssue::MissingBattleAnimBundle);
    }
    if !sections.has_sprite_anim_bundle {
        issues.push(RuntimePackPresenceIssue::MissingSpriteAnimBundle);
    }
    if !sections.has_sprite_palette_defaults {
        issues.push(RuntimePackPresenceIssue::MissingSpritePaletteDefaults);
    }
    if !sections.has_pokegear_town_map_palettes {
        issues.push(RuntimePackPresenceIssue::MissingPokegearTownMapPalettes);
    }
    if !sections.has_pokegear_landmarks {
        issues.push(RuntimePackPresenceIssue::MissingPokegearLandmarks);
    }
    if !sections.has_phone_contacts {
        issues.push(RuntimePackPresenceIssue::MissingPhoneContacts);
    }
    if !sections.has_permanent_phone_numbers {
        issues.push(RuntimePackPresenceIssue::MissingPermanentPhoneNumbers);
    }
    if !sections.has_special_phone_calls {
        issues.push(RuntimePackPresenceIssue::MissingSpecialPhoneCalls);
    }
    if !sections.has_phone_scripts {
        issues.push(RuntimePackPresenceIssue::MissingPhoneScripts);
    }
    if !sections.has_flee_mons {
        issues.push(RuntimePackPresenceIssue::MissingFleeMons);
    }
    if !sections.has_buena_password_categories {
        issues.push(RuntimePackPresenceIssue::MissingBuenaPasswordCategories);
    }
    if !sections.has_roaming_pokemon {
        issues.push(RuntimePackPresenceIssue::MissingRoamingPokemon);
    }
    if !sections.has_buena_prizes {
        issues.push(RuntimePackPresenceIssue::MissingBuenaPrizes);
    }
    if !sections.has_kurt_apricorn_recipes {
        issues.push(RuntimePackPresenceIssue::MissingKurtApricornRecipes);
    }
    if !sections.has_shuckie_gift {
        issues.push(RuntimePackPresenceIssue::MissingShuckieGift);
    }
    if !sections.has_dratini_move_sets {
        issues.push(RuntimePackPresenceIssue::MissingDratiniMoveSets);
    }
    if !sections.has_bug_contest_config {
        issues.push(RuntimePackPresenceIssue::MissingBugContestConfig);
    }
    if !sections.has_battle_tower_rules {
        issues.push(RuntimePackPresenceIssue::MissingBattleTowerRules);
    }
    if !sections.has_oak_ratings {
        issues.push(RuntimePackPresenceIssue::MissingOakRatings);
    }
    if !sections.has_odd_egg_definitions {
        issues.push(RuntimePackPresenceIssue::MissingOddEggDefinitions);
    }
    if !sections.has_magikarp_lengths {
        issues.push(RuntimePackPresenceIssue::MissingMagikarpLengths);
    }
    if !sections.has_happiness_data {
        issues.push(RuntimePackPresenceIssue::MissingHappinessData);
    }
    if !sections.has_initialize_events {
        issues.push(RuntimePackPresenceIssue::MissingInitializeEvents);
    }
    if !sections.has_story_event_script_constants {
        issues.push(RuntimePackPresenceIssue::MissingStoryEventScriptConstants);
    }

    issues
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_pack_presence_issues_require_core_game_sections() {
        assert_eq!(
            runtime_pack_presence_issues(RuntimePackSections::default()),
            vec![
                RuntimePackPresenceIssue::MissingPokemon,
                RuntimePackPresenceIssue::MissingMoves,
                RuntimePackPresenceIssue::MissingGrowthRates,
                RuntimePackPresenceIssue::MissingLearnsets,
                RuntimePackPresenceIssue::MissingEvolutions,
                RuntimePackPresenceIssue::MissingCaptureRules,
                RuntimePackPresenceIssue::MissingCaptureWobbleProbabilities,
                RuntimePackPresenceIssue::MissingBattleStatMultipliers,
                RuntimePackPresenceIssue::MissingMovePriorities,
                RuntimePackPresenceIssue::MissingTypeCategories,
                RuntimePackPresenceIssue::MissingTypeEffectiveness,
                RuntimePackPresenceIssue::MissingWeatherModifiers,
                RuntimePackPresenceIssue::MissingBattleRewardRules,
                RuntimePackPresenceIssue::MissingBattleEscapeRules,
                RuntimePackPresenceIssue::MissingMarts,
                RuntimePackPresenceIssue::MissingCurrencyConstants,
                RuntimePackPresenceIssue::MissingStepEventRules,
                RuntimePackPresenceIssue::MissingFishingCatalog,
                RuntimePackPresenceIssue::MissingFruitTrees,
                RuntimePackPresenceIssue::MissingFieldMoves,
                RuntimePackPresenceIssue::MissingItems,
                RuntimePackPresenceIssue::MissingTrainers,
                RuntimePackPresenceIssue::MissingAudio,
                RuntimePackPresenceIssue::MissingMusicAudio,
                RuntimePackPresenceIssue::MissingSoundEffects,
                RuntimePackPresenceIssue::MissingCryAudio,
                RuntimePackPresenceIssue::MissingPokemonCries,
                RuntimePackPresenceIssue::MissingTilesets,
                RuntimePackPresenceIssue::MissingScripts,
                RuntimePackPresenceIssue::MissingMapGeometry,
                RuntimePackPresenceIssue::MissingMapObjects,
                RuntimePackPresenceIssue::MissingRuntimeMapMetadata,
                RuntimePackPresenceIssue::MissingRuntimeSpawnPoints,
                RuntimePackPresenceIssue::MissingMaps,
                RuntimePackPresenceIssue::MissingPcStrings,
                RuntimePackPresenceIssue::MissingMenuIcons,
                RuntimePackPresenceIssue::MissingPokedexEntries,
                RuntimePackPresenceIssue::MissingPokemonFrontpicAnimations,
                RuntimePackPresenceIssue::MissingMoveNames,
                RuntimePackPresenceIssue::MissingAsmText,
                RuntimePackPresenceIssue::MissingBattleAnimations,
                RuntimePackPresenceIssue::MissingBattleAnimationTable,
                RuntimePackPresenceIssue::MissingBattleAnimBundle,
                RuntimePackPresenceIssue::MissingSpriteAnimBundle,
                RuntimePackPresenceIssue::MissingSpritePaletteDefaults,
                RuntimePackPresenceIssue::MissingPokegearTownMapPalettes,
                RuntimePackPresenceIssue::MissingPokegearLandmarks,
                RuntimePackPresenceIssue::MissingPhoneContacts,
                RuntimePackPresenceIssue::MissingPermanentPhoneNumbers,
                RuntimePackPresenceIssue::MissingSpecialPhoneCalls,
                RuntimePackPresenceIssue::MissingPhoneScripts,
                RuntimePackPresenceIssue::MissingFleeMons,
                RuntimePackPresenceIssue::MissingBuenaPasswordCategories,
                RuntimePackPresenceIssue::MissingRoamingPokemon,
                RuntimePackPresenceIssue::MissingBuenaPrizes,
                RuntimePackPresenceIssue::MissingKurtApricornRecipes,
                RuntimePackPresenceIssue::MissingShuckieGift,
                RuntimePackPresenceIssue::MissingDratiniMoveSets,
                RuntimePackPresenceIssue::MissingBugContestConfig,
                RuntimePackPresenceIssue::MissingBattleTowerRules,
                RuntimePackPresenceIssue::MissingOakRatings,
                RuntimePackPresenceIssue::MissingOddEggDefinitions,
                RuntimePackPresenceIssue::MissingMagikarpLengths,
                RuntimePackPresenceIssue::MissingHappinessData,
                RuntimePackPresenceIssue::MissingInitializeEvents,
                RuntimePackPresenceIssue::MissingStoryEventScriptConstants,
            ],
        );
        assert!(
            runtime_pack_presence_issues(RuntimePackSections {
                has_pokemon: true,
                has_moves: true,
                has_growth_rates: true,
                has_learnsets: true,
                has_evolutions: true,
                has_capture_rules: true,
                has_capture_wobble_probabilities: true,
                has_battle_stat_multipliers: true,
                has_move_priorities: true,
                has_type_categories: true,
                has_type_effectiveness: true,
                has_weather_modifiers: true,
                has_battle_reward_rules: true,
                has_battle_escape_rules: true,
                has_marts: true,
                has_currency_constants: true,
                has_step_event_rules: true,
                has_fishing_catalog: true,
                has_fruit_trees: true,
                has_field_moves: true,
                has_items: true,
                has_trainers: true,
                has_audio: true,
                has_music_audio: true,
                has_sound_effects: true,
                has_cry_audio: true,
                has_pokemon_cries: true,
                has_tilesets: true,
                has_scripts: true,
                has_map_geometry: true,
                has_map_objects: true,
                has_runtime_map_metadata: true,
                has_runtime_spawn_points: true,
                has_maps: true,
                has_pc_strings: true,
                has_menu_icons: true,
                has_pokedex_entries: true,
                has_pokemon_frontpic_animations: true,
                has_move_names: true,
                has_asm_text: true,
                has_battle_animations: true,
                has_battle_animation_table: true,
                has_battle_anim_bundle: true,
                has_sprite_anim_bundle: true,
                has_sprite_palette_defaults: true,
                has_pokegear_town_map_palettes: true,
                has_pokegear_landmarks: true,
                has_phone_contacts: true,
                has_permanent_phone_numbers: true,
                has_special_phone_calls: true,
                has_phone_scripts: true,
                has_flee_mons: true,
                has_buena_password_categories: true,
                has_roaming_pokemon: true,
                has_buena_prizes: true,
                has_kurt_apricorn_recipes: true,
                has_shuckie_gift: true,
                has_dratini_move_sets: true,
                has_bug_contest_config: true,
                has_battle_tower_rules: true,
                has_oak_ratings: true,
                has_odd_egg_definitions: true,
                has_magikarp_lengths: true,
                has_happiness_data: true,
                has_initialize_events: true,
                has_story_event_script_constants: true,
            })
            .is_empty()
        );
    }

    #[test]
    fn runtime_pack_presence_requires_each_audio_kind_not_just_any_audio() {
        let sections = RuntimePackSections {
            has_pokemon: true,
            has_moves: true,
            has_growth_rates: true,
            has_learnsets: true,
            has_evolutions: true,
            has_capture_rules: true,
            has_capture_wobble_probabilities: true,
            has_battle_stat_multipliers: true,
            has_move_priorities: true,
            has_type_categories: true,
            has_type_effectiveness: true,
            has_weather_modifiers: true,
            has_battle_reward_rules: true,
            has_battle_escape_rules: true,
            has_marts: true,
            has_currency_constants: true,
            has_step_event_rules: true,
            has_fishing_catalog: true,
            has_fruit_trees: true,
            has_field_moves: true,
            has_items: true,
            has_trainers: true,
            has_audio: true,
            has_music_audio: true,
            has_sound_effects: false,
            has_cry_audio: false,
            has_pokemon_cries: true,
            has_tilesets: true,
            has_scripts: true,
            has_map_geometry: true,
            has_map_objects: true,
            has_runtime_map_metadata: true,
            has_runtime_spawn_points: true,
            has_maps: true,
            has_pc_strings: true,
            has_menu_icons: true,
            has_pokedex_entries: true,
            has_pokemon_frontpic_animations: true,
            has_move_names: true,
            has_asm_text: true,
            has_battle_animations: true,
            has_battle_animation_table: true,
            has_battle_anim_bundle: true,
            has_sprite_anim_bundle: true,
            has_sprite_palette_defaults: true,
            has_pokegear_town_map_palettes: true,
            has_pokegear_landmarks: true,
            has_phone_contacts: true,
            has_permanent_phone_numbers: true,
            has_special_phone_calls: true,
            has_phone_scripts: true,
            has_flee_mons: true,
            has_buena_password_categories: true,
            has_roaming_pokemon: true,
            has_buena_prizes: true,
            has_kurt_apricorn_recipes: true,
            has_shuckie_gift: true,
            has_dratini_move_sets: true,
            has_bug_contest_config: true,
            has_battle_tower_rules: true,
            has_oak_ratings: true,
            has_odd_egg_definitions: true,
            has_magikarp_lengths: true,
            has_happiness_data: true,
            has_initialize_events: true,
            has_story_event_script_constants: true,
        };

        assert_eq!(
            runtime_pack_presence_issues(sections),
            vec![
                RuntimePackPresenceIssue::MissingSoundEffects,
                RuntimePackPresenceIssue::MissingCryAudio,
            ]
        );
    }
}
