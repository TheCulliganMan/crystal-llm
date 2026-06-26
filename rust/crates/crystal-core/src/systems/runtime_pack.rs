#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimePackPresenceIssue {
    MissingPokemon,
    MissingMoves,
    MissingGrowthRates,
    MissingLearnsets,
    MissingEvolutions,
    MissingBattleFormulaTables,
    MissingBattleRuleCatalogs,
    MissingEconomyCatalogs,
    MissingFieldSystemCatalogs,
    MissingItems,
    MissingTrainers,
    MissingAudio,
    MissingPokemonCries,
    MissingTilesets,
    MissingScripts,
    MissingMapGeometry,
    MissingMapObjects,
    MissingRuntimeMapMetadata,
    MissingRuntimeSpawnPoints,
    MissingMaps,
    MissingUiCatalogs,
    MissingDisplayCatalogs,
    MissingPhoneCatalogs,
    MissingSpecialSystemCatalogs,
    MissingEventBootstrapCatalogs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RuntimePackSections {
    pub has_pokemon: bool,
    pub has_moves: bool,
    pub has_growth_rates: bool,
    pub has_learnsets: bool,
    pub has_evolutions: bool,
    pub has_battle_formula_tables: bool,
    pub has_battle_rule_catalogs: bool,
    pub has_economy_catalogs: bool,
    pub has_field_system_catalogs: bool,
    pub has_items: bool,
    pub has_trainers: bool,
    pub has_audio: bool,
    pub has_pokemon_cries: bool,
    pub has_tilesets: bool,
    pub has_scripts: bool,
    pub has_map_geometry: bool,
    pub has_map_objects: bool,
    pub has_runtime_map_metadata: bool,
    pub has_runtime_spawn_points: bool,
    pub has_maps: bool,
    pub has_ui_catalogs: bool,
    pub has_display_catalogs: bool,
    pub has_phone_catalogs: bool,
    pub has_special_system_catalogs: bool,
    pub has_event_bootstrap_catalogs: bool,
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
    if !sections.has_battle_formula_tables {
        issues.push(RuntimePackPresenceIssue::MissingBattleFormulaTables);
    }
    if !sections.has_battle_rule_catalogs {
        issues.push(RuntimePackPresenceIssue::MissingBattleRuleCatalogs);
    }
    if !sections.has_economy_catalogs {
        issues.push(RuntimePackPresenceIssue::MissingEconomyCatalogs);
    }
    if !sections.has_field_system_catalogs {
        issues.push(RuntimePackPresenceIssue::MissingFieldSystemCatalogs);
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
    if !sections.has_ui_catalogs {
        issues.push(RuntimePackPresenceIssue::MissingUiCatalogs);
    }
    if !sections.has_display_catalogs {
        issues.push(RuntimePackPresenceIssue::MissingDisplayCatalogs);
    }
    if !sections.has_phone_catalogs {
        issues.push(RuntimePackPresenceIssue::MissingPhoneCatalogs);
    }
    if !sections.has_special_system_catalogs {
        issues.push(RuntimePackPresenceIssue::MissingSpecialSystemCatalogs);
    }
    if !sections.has_event_bootstrap_catalogs {
        issues.push(RuntimePackPresenceIssue::MissingEventBootstrapCatalogs);
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
                RuntimePackPresenceIssue::MissingBattleFormulaTables,
                RuntimePackPresenceIssue::MissingBattleRuleCatalogs,
                RuntimePackPresenceIssue::MissingEconomyCatalogs,
                RuntimePackPresenceIssue::MissingFieldSystemCatalogs,
                RuntimePackPresenceIssue::MissingItems,
                RuntimePackPresenceIssue::MissingTrainers,
                RuntimePackPresenceIssue::MissingAudio,
                RuntimePackPresenceIssue::MissingPokemonCries,
                RuntimePackPresenceIssue::MissingTilesets,
                RuntimePackPresenceIssue::MissingScripts,
                RuntimePackPresenceIssue::MissingMapGeometry,
                RuntimePackPresenceIssue::MissingMapObjects,
                RuntimePackPresenceIssue::MissingRuntimeMapMetadata,
                RuntimePackPresenceIssue::MissingRuntimeSpawnPoints,
                RuntimePackPresenceIssue::MissingMaps,
                RuntimePackPresenceIssue::MissingUiCatalogs,
                RuntimePackPresenceIssue::MissingDisplayCatalogs,
                RuntimePackPresenceIssue::MissingPhoneCatalogs,
                RuntimePackPresenceIssue::MissingSpecialSystemCatalogs,
                RuntimePackPresenceIssue::MissingEventBootstrapCatalogs,
            ],
        );
        assert!(runtime_pack_presence_issues(RuntimePackSections {
            has_pokemon: true,
            has_moves: true,
            has_growth_rates: true,
            has_learnsets: true,
            has_evolutions: true,
            has_battle_formula_tables: true,
            has_battle_rule_catalogs: true,
            has_economy_catalogs: true,
            has_field_system_catalogs: true,
            has_items: true,
            has_trainers: true,
            has_audio: true,
            has_pokemon_cries: true,
            has_tilesets: true,
            has_scripts: true,
            has_map_geometry: true,
            has_map_objects: true,
            has_runtime_map_metadata: true,
            has_runtime_spawn_points: true,
            has_maps: true,
            has_ui_catalogs: true,
            has_display_catalogs: true,
            has_phone_catalogs: true,
            has_special_system_catalogs: true,
            has_event_bootstrap_catalogs: true,
        })
        .is_empty());
    }
}
