fn verify_game_data(
    asset_root: &AssetRoot,
    data: &GameDataSet,
    rules: &PlayabilityRules,
) -> ModpackCompileReport {
    let mut diagnostics = Vec::new();
    let map_names = data.maps.keys().cloned().collect::<BTreeSet<_>>();

    verify_species_and_moves(data, &mut diagnostics);
    verify_items(data, &mut diagnostics);
    verify_evolutions(data, &mut diagnostics);
    verify_encounters(data, &map_names, &mut diagnostics);
    verify_trainers(data, &mut diagnostics);
    verify_trainer_class_names(data, &mut diagnostics);
    verify_audio_assets(asset_root, data, &mut diagnostics);
    verify_map_music(data, &mut diagnostics);
    verify_runtime_title_screen(data, &mut diagnostics);
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
    verify_map_command_source_scripts(data, &mut diagnostics);
    verify_script_economy_commands(data, &mut diagnostics);
    verify_gift_pokemon_scripts(data, &mut diagnostics);
    verify_script_flag_commands(data, &mut diagnostics);
    verify_script_scene_commands(data, &mut diagnostics);
    verify_script_audio_commands(data, &mut diagnostics);
    verify_audio_asset_usage(data, &mut diagnostics);
    verify_script_block_changes(data, &mut diagnostics);
    verify_script_movements(data, &mut diagnostics);
    verify_script_object_commands(data, &mut diagnostics);
    verify_script_map_commands(data, &mut diagnostics);
    verify_script_text_commands(data, &mut diagnostics);
    verify_script_text_bodies(data, &mut diagnostics);
    verify_script_menu_definitions(data, &mut diagnostics);
    verify_script_variable_commands(data, &mut diagnostics);
    verify_script_control_commands(data, &mut diagnostics);
    verify_standard_script_catalog(data, &mut diagnostics);
    verify_overworld_event_catalog(data, &mut diagnostics);
    verify_map_section_commands(data, &mut diagnostics);
    verify_script_field_pickups(data, &mut diagnostics);
    verify_script_shop_commands(data, &mut diagnostics);
    verify_script_phone_commands(data, &mut diagnostics);
    verify_script_runtime_commands(data, &mut diagnostics);
    verify_script_swarm_commands(data, &mut diagnostics);
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

fn verify_standard_script_catalog(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    if let Err(error) = validate_compiled_standard_script_catalog(data) {
        diagnostics.push(VerificationError::error(
            "invalid_runtime_standard_scripts",
            "story_events:StandardScripts",
            error.to_string(),
        ));
    }
}

fn verify_overworld_event_catalog(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    if let Err(error) = validate_compiled_overworld_event_catalog(data) {
        diagnostics.push(VerificationError::error(
            "invalid_runtime_overworld_events",
            "story_events:OverworldEvents",
            error.to_string(),
        ));
    }
}

fn merged_playability_rules(
    base: &PlayabilityRules,
    overlay: &PlayabilityRules,
) -> Result<PlayabilityRules> {
    let mut merged = base.clone();
    merge_playability_rules(&mut merged, overlay)?;
    Ok(merged)
}

fn merge_playability_rules(target: &mut PlayabilityRules, source: &PlayabilityRules) -> Result<()> {
    merge_token_string_vec(
        &mut target.start_maps,
        source.start_maps.clone(),
        "playability start map",
    )?;
    for start in &source.start_tiles {
        validate_modpack_payload_token(&start.map, "playability start tile map")?;
    }
    merge_exact_vec_by(
        &mut target.start_tiles,
        source.start_tiles.clone(),
        "playability start tile",
        |start| format!("{}@{},{}", start.map, start.tile.x, start.tile.y),
    )?;
    merge_token_string_vec(
        &mut target.initial_events,
        source.initial_events.clone(),
        "playability initial event",
    )?;
    merge_token_string_vec(
        &mut target.initial_items,
        source.initial_items.clone(),
        "playability initial item",
    )?;
    merge_token_string_vec(
        &mut target.goal_maps,
        source.goal_maps.clone(),
        "playability goal map",
    )?;
    merge_token_string_vec(
        &mut target.goal_events,
        source.goal_events.clone(),
        "playability goal event",
    )?;
    merge_token_string_vec(
        &mut target.goal_items,
        source.goal_items.clone(),
        "playability goal item",
    )?;
    for rule in &source.progression_rules {
        validate_progression_rule(rule)?;
    }
    merge_exact_vec_by(
        &mut target.progression_rules,
        source.progression_rules.clone(),
        "playability progression rule",
        |rule| rule.id.clone(),
    )?;
    for rule in &source.map_access {
        validate_modpack_payload_token(&rule.map, "playability map access map")?;
        validate_progression_requirements(&rule.requires, "playability map access requirement")?;
    }
    merge_exact_vec_by(
        &mut target.map_access,
        source.map_access.clone(),
        "playability map access rule",
        |rule| rule.map.clone(),
    )?;
    target.require_all_maps_reachable |= source.require_all_maps_reachable;
    target.require_walkable_maps |= source.require_walkable_maps;
    Ok(())
}

fn validate_progression_rule(rule: &ProgressionRule) -> Result<()> {
    if !is_exact_progression_rule_id(&rule.id) {
        anyhow::bail!(
            "playability progression rule id '{}' must be exact ASCII label token",
            rule.id
        );
    }
    validate_no_reserved_payload_token(&rule.id, "playability progression rule id")?;
    validate_progression_requirements(&rule.requires, "playability progression requirement")?;
    validate_progression_grants(&rule.grants, "playability progression grant")?;
    Ok(())
}

fn validate_progression_requirements(
    requirements: &ProgressionRequirements,
    description: &str,
) -> Result<()> {
    validate_exact_string_slice(&requirements.events, &format!("{description} event"))?;
    validate_exact_string_slice(&requirements.items, &format!("{description} item"))?;
    validate_exact_string_slice(&requirements.maps, &format!("{description} map"))?;
    Ok(())
}

fn validate_progression_grants(grants: &ProgressionGrants, description: &str) -> Result<()> {
    validate_exact_string_slice(&grants.events, &format!("{description} event"))?;
    validate_exact_string_slice(&grants.items, &format!("{description} item"))?;
    validate_exact_string_slice(&grants.maps, &format!("{description} map"))?;
    Ok(())
}

fn validate_exact_string_slice(values: &[String], description: &str) -> Result<()> {
    for value in values {
        validate_modpack_payload_token(value, description)?;
    }
    Ok(())
}

fn merge_token_string_vec(
    target: &mut Vec<String>,
    source: Vec<String>,
    description: &str,
) -> Result<()> {
    for value in source {
        validate_modpack_payload_token(&value, description)?;
        if target.contains(&value) {
            anyhow::bail!("duplicate {description} '{value}'");
        }
        target.push(value);
    }
    Ok(())
}

fn materialize_runtime_map_modules(data: &mut GameDataSet) -> Result<()> {
    let map_names: Vec<String> = data.map_attributes.keys().cloned().collect();
    for map_name in map_names {
        if data.maps.contains_key(&map_name) {
            continue;
        }
        let module = data
            .assemble_map_module_from_compiled_payloads(&map_name)
            .with_context(|| format!("assemble definitive compiled map module for {map_name}"))?;
        insert_map_module(&mut data.maps, module)?;
    }
    data.materialize_global_scripts()
        .context("assemble definitive compiled global scripts")?;
    Ok(())
}

fn verify_species_and_moves(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    let item_ids: BTreeSet<String> = data.items.keys().cloned().collect();
    let move_ids: BTreeSet<String> = data.moves.keys().cloned().collect();
    for (move_id, move_data) in &data.moves {
        for issue in move_payload_issues(move_data) {
            diagnostics.push(move_payload_issue_diagnostic(move_id, issue));
        }
        if !battle_move_effect_is_supported(&move_data.effect) {
            diagnostics.push(VerificationError::error(
                "unsupported_battle_move_effect",
                move_id,
                format!(
                    "move '{move_id}' declares battle effect '{}' that has no Rust runtime mutation",
                    move_data.effect
                ),
            ));
        }
    }
    diagnostics.extend(
        move_source_index_catalog_issues(&data.moves)
            .into_iter()
            .map(move_source_index_catalog_issue_diagnostic),
    );
    diagnostics.extend(
        growth_rate_catalog_issues(&data.growth_rates)
            .into_iter()
            .map(growth_rate_catalog_issue_diagnostic),
    );
    diagnostics.extend(
        learnset_catalog_issues(&data.pokemon, &data.learnsets, &item_ids, &move_ids)
            .into_iter()
            .map(learnset_catalog_issue_diagnostic),
    );
}

fn move_source_index_catalog_issue_diagnostic(
    issue: MoveSourceIndexCatalogIssue,
) -> VerificationError {
    match issue {
        MoveSourceIndexCatalogIssue::DuplicateSourceIndex {
            source_index,
            first_move,
            second_move,
        } => VerificationError::error(
            "duplicate_move_source_index",
            source_index.to_string(),
            format!(
                "moves '{first_move}' and '{second_move}' both declare ASM move index {source_index}"
            ),
        ),
    }
}

fn growth_rate_catalog_issue_diagnostic(issue: GrowthRateCatalogIssue) -> VerificationError {
    match issue {
        GrowthRateCatalogIssue::InvalidCatalogId { growth_rate } => VerificationError::error(
            "invalid_growth_rate_id",
            &growth_rate,
            format!(
                "growth-rate catalog ids must be exact non-empty tokens, found {growth_rate:?}"
            ),
        ),
        GrowthRateCatalogIssue::MismatchedCurveId {
            growth_rate,
            declared_id,
        } => VerificationError::error(
            "growth_rate_id_mismatch",
            &growth_rate,
            format!(
                "growth-rate catalog key '{growth_rate}' does not match curve id '{declared_id}'"
            ),
        ),
        GrowthRateCatalogIssue::ZeroDenominator { growth_rate } => VerificationError::error(
            "invalid_growth_rate_denominator",
            &growth_rate,
            "growth-rate curves must declare a nonzero denominator",
        ),
    }
}

fn move_payload_issue_diagnostic(move_id: &str, issue: MovePayloadIssue) -> VerificationError {
    match issue {
        MovePayloadIssue::MissingName => VerificationError::error(
            "missing_move_name",
            move_id,
            "moves must declare explicit nonempty name ids",
        ),
        MovePayloadIssue::InvalidName { name } => VerificationError::error(
            "invalid_move_name",
            move_id,
            format!("moves must declare exact name ids, found '{name}'"),
        ),
        MovePayloadIssue::MissingType => VerificationError::error(
            "missing_move_type",
            move_id,
            "moves must declare explicit nonempty type ids",
        ),
        MovePayloadIssue::InvalidType { move_type } => VerificationError::error(
            "invalid_move_type",
            move_id,
            format!("moves must declare exact type ids, found '{move_type}'"),
        ),
        MovePayloadIssue::MissingEffect => VerificationError::error(
            "missing_move_effect",
            move_id,
            "moves must declare explicit nonempty effect ids",
        ),
        MovePayloadIssue::InvalidEffect { effect } => VerificationError::error(
            "invalid_move_effect",
            move_id,
            format!("moves must declare exact effect ids, found '{effect}'"),
        ),
    }
}

fn learnset_catalog_issue_diagnostic(issue: LearnsetCatalogIssue) -> VerificationError {
    match issue {
        LearnsetCatalogIssue::MissingSpeciesLearnset { species_id } => VerificationError::error(
            "missing_species_learnset",
            species_id,
            "Pokemon species is missing an explicit level-up learnset",
        ),
        LearnsetCatalogIssue::InvalidSpeciesHeldItem {
            species_id,
            item_id,
        } => VerificationError::error(
            "invalid_species_held_item",
            species_id,
            format!("Pokemon species held item '{item_id}' must be an exact nonempty item id"),
        ),
        LearnsetCatalogIssue::UnknownSpeciesHeldItem {
            species_id,
            item_id,
        } => VerificationError::error(
            "unknown_species_held_item",
            species_id,
            format!("Pokemon species references missing held item '{item_id}'"),
        ),
        LearnsetCatalogIssue::InvalidTmHmMove {
            species_id,
            move_id,
        } => VerificationError::error(
            "invalid_tmhm_move",
            species_id,
            format!("TM/HM learnset move '{move_id}' must be an exact nonempty move id"),
        ),
        LearnsetCatalogIssue::UnknownTmHmMove {
            species_id,
            move_id,
        } => VerificationError::error(
            "unknown_tmhm_move",
            species_id,
            format!("TM/HM learnset references missing move '{move_id}'"),
        ),
        LearnsetCatalogIssue::InvalidLearnsetSpecies { species_id } => VerificationError::error(
            "invalid_learnset_species",
            species_id,
            "learnset species id must be an exact nonempty species id",
        ),
        LearnsetCatalogIssue::UnknownLearnsetSpecies { species_id } => VerificationError::error(
            "unknown_learnset_species",
            species_id,
            "learnset references a species that is not loaded",
        ),
        LearnsetCatalogIssue::InvalidLevelMove {
            species_id,
            move_id,
        } => VerificationError::error(
            "invalid_level_move",
            species_id,
            format!("level-up learnset move '{move_id}' must be an exact nonempty move id"),
        ),
        LearnsetCatalogIssue::UnknownLevelMove {
            species_id,
            move_id,
        } => VerificationError::error(
            "unknown_level_move",
            species_id,
            format!("level-up learnset references missing move '{move_id}'"),
        ),
    }
}

fn verify_items(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    let move_ids: BTreeSet<String> = data.moves.keys().cloned().collect();
    let field_rule_item_ids = field_rule_item_ids(data);
    for (item_id, item) in &data.items {
        let payload_issues =
            item_payload_issues_with_known_field_rules(item, field_rule_item_ids.contains(item_id));
        let has_invalid_capture_ball_id = payload_issues.iter().any(|issue| {
            matches!(
                issue,
                ItemPayloadIssue::MissingScriptName | ItemPayloadIssue::InvalidScriptName { .. }
            )
        });
        if item.pocket == ITEM_POCKET_BALL
            && !has_invalid_capture_ball_id
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
        for issue in payload_issues {
            diagnostics.push(item_payload_issue_diagnostic(item_id, item, issue));
        }
        for issue in item_reference_issues(item, &move_ids) {
            diagnostics.push(item_reference_issue_diagnostic(item_id, issue));
        }
    }
}

fn field_rule_item_ids(data: &GameDataSet) -> BTreeSet<String> {
    [
        &data.field_moves.bicycle,
        &data.field_moves.itemfinder,
        &data.field_moves.squirtbottle,
        &data.field_moves.coin_case,
        &data.field_moves.blue_card,
        &data.field_moves.town_map,
        &data.field_moves.pokegear,
    ]
    .into_iter()
    .filter_map(|rule| (!rule.item_id.is_empty()).then(|| rule.item_id.clone()))
    .chain(
        [&data.field_moves.card_key, &data.field_moves.basement_key]
            .into_iter()
            .filter_map(|rule| (!rule.item_id.is_empty()).then(|| rule.item_id.clone())),
    )
    .chain(
        (!data.field_moves.escape_rope.item_id.is_empty())
            .then(|| data.field_moves.escape_rope.item_id.clone()),
    )
    .chain(
        data.fishing
            .rod_items
            .keys()
            .filter(|item_id| !item_id.is_empty())
            .cloned(),
    )
    .chain(data.field_box_items.keys().cloned())
    .chain(
        data.field_box_items
            .values()
            .filter_map(|rule| (!rule.item_id.is_empty()).then(|| rule.item_id.clone())),
    )
    .collect()
}

fn item_payload_issue_diagnostic(
    item_id: &str,
    item: &Item,
    issue: ItemPayloadIssue,
) -> VerificationError {
    match issue {
        ItemPayloadIssue::MissingName => VerificationError::error(
            "missing_item_name",
            item_id,
            "items must declare explicit nonempty display names",
        ),
        ItemPayloadIssue::InvalidName { name } => VerificationError::error(
            "invalid_item_name",
            item_id,
            format!("items must declare exact display names, found '{name}'"),
        ),
        ItemPayloadIssue::MissingDescription => VerificationError::error(
            "missing_item_description",
            item_id,
            "items must declare explicit nonempty descriptions",
        ),
        ItemPayloadIssue::InvalidDescription { description } => VerificationError::error(
            "invalid_item_description",
            item_id,
            format!("items must declare exact descriptions, found '{description}'"),
        ),
        ItemPayloadIssue::MissingScriptName => VerificationError::error(
            "missing_item_script_name",
            item_id,
            "items must declare explicit nonempty script_name ids",
        ),
        ItemPayloadIssue::InvalidScriptName { script_name } => VerificationError::error(
            "invalid_item_script_name",
            item_id,
            format!("items must declare exact script_name ids, found '{script_name}'"),
        ),
        ItemPayloadIssue::MissingPocket => VerificationError::error(
            "missing_item_pocket",
            item_id,
            "items must declare explicit nonempty pocket ids",
        ),
        ItemPayloadIssue::InvalidPocket { pocket } => VerificationError::error(
            "invalid_item_pocket",
            item_id,
            format!("items must declare exact pocket ids, found '{pocket}'"),
        ),
        ItemPayloadIssue::MissingEffect => VerificationError::error(
            "missing_item_effect",
            item_id,
            "items must declare explicit nonempty effect ids",
        ),
        ItemPayloadIssue::InvalidEffect { effect } => VerificationError::error(
            "invalid_item_effect",
            item_id,
            format!("items must declare exact effect ids, found '{effect}'"),
        ),
        ItemPayloadIssue::MissingHeldEffect => VerificationError::error(
            "missing_item_held_effect",
            item_id,
            "items must declare explicit nonempty held_effect ids",
        ),
        ItemPayloadIssue::InvalidHeldEffect { held_effect } => VerificationError::error(
            "invalid_item_held_effect",
            item_id,
            format!("items must declare exact held_effect ids, found '{held_effect}'"),
        ),
        ItemPayloadIssue::InvalidProperty { property } => VerificationError::error(
            "invalid_item_property",
            item_id,
            format!("items must declare exact nonempty property ids, found '{property}'"),
        ),
        ItemPayloadIssue::MissingFieldMenu => VerificationError::error(
            "missing_item_field_menu",
            item_id,
            "items must declare explicit nonempty field_menu ids",
        ),
        ItemPayloadIssue::InvalidFieldMenu { menu } => VerificationError::error(
            "invalid_item_field_menu",
            item_id,
            format!("items must declare exact field_menu ids, found '{menu}'"),
        ),
        ItemPayloadIssue::MissingBattleMenu => VerificationError::error(
            "missing_item_battle_menu",
            item_id,
            "items must declare explicit nonempty battle_menu ids",
        ),
        ItemPayloadIssue::InvalidBattleMenu { menu } => VerificationError::error(
            "invalid_item_battle_menu",
            item_id,
            format!("items must declare exact battle_menu ids, found '{menu}'"),
        ),
        ItemPayloadIssue::InvalidStatusHeal { index, status } => VerificationError::error(
            "invalid_item_status_heal",
            item_id,
            format!("status_heals:{index} must be an exact nonempty status id, found '{status}'"),
        ),
        ItemPayloadIssue::InvalidHealAmount { amount } => VerificationError::error(
            "invalid_item_heal_amount",
            item_id,
            format!(
                "{} requires parameter -1 or a positive HP amount, found {amount}",
                item.effect
            ),
        ),
        ItemPayloadIssue::InvalidReviveHpPercent { percent } => VerificationError::error(
            "invalid_item_revive_hp_percent",
            item_id,
            format!("revive_hp_percent must be from 1 to 100, found {percent}"),
        ),
        ItemPayloadIssue::InvalidPartyReviveHpPercent { percent } => VerificationError::error(
            "invalid_item_party_revive_hp_percent",
            item_id,
            format!("party_revive_hp_percent must be from 1 to 100, found {percent}"),
        ),
        ItemPayloadIssue::MissingPpRestoreScope => VerificationError::error(
            "missing_item_pp_restore_scope",
            item_id,
            "RESTORE_PP requires explicit pp_restore_scope",
        ),
        ItemPayloadIssue::InvalidPpRestoreScope { scope } => VerificationError::error(
            "invalid_item_pp_restore_scope",
            item_id,
            format!("RESTORE_PP requires pp_restore_scope 'MOVE' or 'POKEMON', found '{scope}'"),
        ),
        ItemPayloadIssue::InvalidPpRestorePoints { .. } => VerificationError::error(
            "invalid_item_pp_restore_points",
            item_id,
            "RESTORE_PP pp_restore_points must be positive when present",
        ),
        ItemPayloadIssue::InvalidPpUpStages { stages } => VerificationError::error(
            "invalid_item_pp_up_stages",
            item_id,
            format!("pp_up_stages must be from 1 to 3, found {stages}"),
        ),
        ItemPayloadIssue::MissingVitaminStat => VerificationError::error(
            "missing_item_vitamin_stat",
            item_id,
            "VITAMIN requires explicit vitamin_stat",
        ),
        ItemPayloadIssue::InvalidVitaminStat { stat } => VerificationError::error(
            "invalid_item_vitamin_stat",
            item_id,
            format!("VITAMIN uses unknown vitamin_stat '{stat}'"),
        ),
        ItemPayloadIssue::MissingVitaminStatExp => VerificationError::error(
            "missing_item_vitamin_stat_exp",
            item_id,
            "VITAMIN requires explicit vitamin_stat_exp",
        ),
        ItemPayloadIssue::InvalidVitaminStatExp { amount } => VerificationError::error(
            "invalid_item_vitamin_stat_exp",
            item_id,
            format!("VITAMIN requires positive vitamin_stat_exp, found {amount}"),
        ),
        ItemPayloadIssue::MissingVitaminMaxStatExp => VerificationError::error(
            "missing_item_vitamin_max_stat_exp",
            item_id,
            "VITAMIN requires explicit vitamin_max_stat_exp",
        ),
        ItemPayloadIssue::InvalidVitaminMaxStatExp { max } => VerificationError::error(
            "invalid_item_vitamin_max_stat_exp",
            item_id,
            format!(
                "VITAMIN requires vitamin_max_stat_exp >= vitamin_stat_exp and positive, found {max}"
            ),
        ),
        ItemPayloadIssue::InvalidRareCandyLevelGain { level_gain } => VerificationError::error(
            "invalid_item_rare_candy_level_gain",
            item_id,
            format!("rare_candy_level_gain must be positive, found {level_gain}"),
        ),
        ItemPayloadIssue::MissingBattleStatBoostStat => VerificationError::error(
            "missing_item_battle_stat_boost_stat",
            item_id,
            format!("{} requires explicit battle_stat_boost_stat", item.effect),
        ),
        ItemPayloadIssue::InvalidBattleStatBoostStat { stat } => VerificationError::error(
            "invalid_item_battle_stat_boost_stat",
            item_id,
            format!(
                "{} uses unknown battle_stat_boost_stat '{stat}'",
                item.effect
            ),
        ),
        ItemPayloadIssue::MissingBattleStatBoostStages => VerificationError::error(
            "missing_item_battle_stat_boost_stages",
            item_id,
            format!("{} requires explicit battle_stat_boost_stages", item.effect),
        ),
        ItemPayloadIssue::InvalidBattleStatBoostStages { stages } => VerificationError::error(
            "invalid_item_battle_stat_boost_stages",
            item_id,
            format!(
                "{} requires battle_stat_boost_stages from 1 to 6, found {stages}",
                item.effect
            ),
        ),
        ItemPayloadIssue::MissingBattleStatDropGuard => VerificationError::error(
            "missing_item_battle_stat_drop_guard",
            item_id,
            "battle_stat_drop_guard_turns requires explicit battle_stat_drop_guard",
        ),
        ItemPayloadIssue::InvalidBattleStatDropGuard => VerificationError::error(
            "invalid_item_battle_stat_drop_guard",
            item_id,
            "battle_stat_drop_guard must be true when declared",
        ),
        ItemPayloadIssue::MissingBattleStatDropGuardTurns => VerificationError::error(
            "missing_item_battle_stat_drop_guard_turns",
            item_id,
            "battle_stat_drop_guard requires explicit battle_stat_drop_guard_turns",
        ),
        ItemPayloadIssue::InvalidBattleStatDropGuardTurns { turns } => VerificationError::error(
            "invalid_item_battle_stat_drop_guard_turns",
            item_id,
            format!("battle_stat_drop_guard_turns must be positive, found {turns}"),
        ),
        ItemPayloadIssue::InvalidBattleEscapeMode { mode } => VerificationError::error(
            "invalid_item_battle_escape_mode",
            item_id,
            format!("battle_escape_mode must be 'WILD_BATTLE' when declared, found '{mode}'"),
        ),
        ItemPayloadIssue::InvalidBattleCaptureBall => VerificationError::error(
            "invalid_item_battle_capture_ball",
            item_id,
            "battle_capture_ball must be true when declared",
        ),
        ItemPayloadIssue::InvalidRepelSteps { steps } => VerificationError::error(
            "invalid_item_repel_steps",
            item_id,
            format!("repel_steps must be positive when declared, found {steps}"),
        ),
        ItemPayloadIssue::InvalidBattleFocusEnergy => VerificationError::error(
            "invalid_item_battle_focus_energy",
            item_id,
            "battle_focus_energy must be true when declared",
        ),
        ItemPayloadIssue::InvalidConfusionHeal => VerificationError::error(
            "invalid_item_confusion_heal",
            item_id,
            "confusion_heal must be true when declared",
        ),
        ItemPayloadIssue::MissingTmhmIndex => VerificationError::error(
            "missing_item_tmhm_index",
            item_id,
            "TM/HM items must declare explicit tmhm_index",
        ),
        ItemPayloadIssue::InvalidTmhmIndex { index } => VerificationError::error(
            "invalid_item_tmhm_index",
            item_id,
            format!("TM/HM item tmhm_index must be positive, found {index}"),
        ),
        ItemPayloadIssue::MissingTmhmMove => VerificationError::error(
            "missing_item_tmhm_move",
            item_id,
            "TM/HM items must declare explicit tmhm_move",
        ),
        ItemPayloadIssue::InvalidTmhmMove { move_id } => VerificationError::error(
            "invalid_item_tmhm_move",
            item_id,
            format!("TM/HM item tmhm_move must be an exact move id, found '{move_id}'"),
        ),
        ItemPayloadIssue::InvalidFieldUsableMenu { menu, usable } => VerificationError::error(
            "invalid_item_field_usable_menu",
            item_id,
            format!("field_usable {usable} contradicts field_menu '{menu}'"),
        ),
        ItemPayloadIssue::InvalidBattleUsableMenu { menu, usable } => VerificationError::error(
            "invalid_item_battle_usable_menu",
            item_id,
            format!("battle_usable {usable} contradicts battle_menu '{menu}'"),
        ),
        ItemPayloadIssue::MissingFieldItemPayload => VerificationError::error(
            "missing_item_field_payload",
            item_id,
            "field_usable items must declare an exact field item payload or field item rule",
        ),
        ItemPayloadIssue::MissingBattleItemPayload => VerificationError::error(
            "missing_item_battle_payload",
            item_id,
            "battle_usable items must declare an exact battle item payload",
        ),
    }
}

fn item_reference_issue_diagnostic(item_id: &str, issue: ItemReferenceIssue) -> VerificationError {
    match issue {
        ItemReferenceIssue::UnknownTmhmMove { move_id } => VerificationError::error(
            "unknown_item_tmhm_move",
            item_id,
            format!("TM/HM item references missing move '{move_id}'"),
        ),
    }
}

fn verify_evolutions(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    let species_ids: BTreeSet<String> = data.pokemon.keys().cloned().collect();
    let item_ids: BTreeSet<String> = data.items.keys().cloned().collect();
    diagnostics.extend(
        evolution_table_issues(&data.evolutions, &species_ids, &item_ids)
            .into_iter()
            .map(evolution_table_issue_diagnostic),
    );
}

fn evolution_table_issue_diagnostic(issue: EvolutionTableIssue) -> VerificationError {
    match issue {
        EvolutionTableIssue::MissingSpeciesEvolutions { species_id } => VerificationError::error(
            "missing_species_evolutions",
            species_id,
            "Pokemon species is missing an explicit evolution table entry",
        ),
        EvolutionTableIssue::InvalidSourceSpecies { species_id } => VerificationError::error(
            "invalid_evolution_source_species",
            species_id,
            "evolution table source species must be an exact nonempty species id",
        ),
        EvolutionTableIssue::UnknownSourceSpecies { species_id } => VerificationError::error(
            "unknown_evolution_source_species",
            species_id,
            "evolution table references a source species that is not loaded",
        ),
        EvolutionTableIssue::InvalidTargetSpecies {
            source_species_id,
            target_species_id,
        } => VerificationError::error(
            "invalid_evolution_target_species",
            source_species_id,
            format!(
                "evolution target species '{target_species_id}' must be an exact nonempty species id"
            ),
        ),
        EvolutionTableIssue::UnknownTargetSpecies {
            source_species_id,
            target_species_id,
        } => VerificationError::error(
            "unknown_evolution_target_species",
            source_species_id,
            format!("evolution target species '{target_species_id}' is not loaded"),
        ),
        EvolutionTableIssue::MissingLevel { source_species_id } => VerificationError::error(
            "missing_evolution_level",
            source_species_id,
            "LEVEL evolution requires an exact level",
        ),
        EvolutionTableIssue::MissingItem { source_species_id } => VerificationError::error(
            "missing_evolution_item",
            source_species_id,
            "ITEM evolution requires an exact item id",
        ),
        EvolutionTableIssue::InvalidItem {
            source_species_id,
            item_id,
        } => VerificationError::error(
            "invalid_evolution_item",
            source_species_id,
            format!("ITEM evolution item '{item_id}' must be an exact nonempty item id"),
        ),
        EvolutionTableIssue::UnknownItem {
            source_species_id,
            item_id,
        } => VerificationError::error(
            "unknown_evolution_item",
            source_species_id,
            format!("ITEM evolution references missing item '{item_id}'"),
        ),
        EvolutionTableIssue::MissingHappinessWindow { source_species_id } => {
            VerificationError::error(
                "missing_evolution_happiness_window",
                source_species_id,
                "HAPPINESS evolution requires an exact time window",
            )
        }
        EvolutionTableIssue::InvalidHappinessWindow {
            source_species_id,
            window,
        } => VerificationError::error(
            "invalid_evolution_happiness_window",
            source_species_id,
            format!("HAPPINESS evolution window '{window}' must be an exact nonempty time window"),
        ),
        EvolutionTableIssue::UnknownHappinessWindow {
            source_species_id,
            window,
        } => VerificationError::error(
            "unknown_evolution_happiness_window",
            source_species_id,
            format!("HAPPINESS evolution uses unknown window '{window}'"),
        ),
        EvolutionTableIssue::UnknownTradeItem {
            source_species_id,
            item_id,
        } => VerificationError::error(
            "unknown_trade_evolution_item",
            source_species_id,
            format!("TRADE evolution references missing held item '{item_id}'"),
        ),
        EvolutionTableIssue::InvalidTradeItem {
            source_species_id,
            item_id,
        } => VerificationError::error(
            "invalid_trade_evolution_item",
            source_species_id,
            format!("TRADE evolution held item '{item_id}' must be an exact nonempty item id"),
        ),
        EvolutionTableIssue::MissingStatLevel { source_species_id } => VerificationError::error(
            "missing_stat_evolution_level",
            source_species_id,
            "STAT evolution requires an exact level",
        ),
        EvolutionTableIssue::MissingStatRatio { source_species_id } => VerificationError::error(
            "missing_evolution_stat_ratio",
            source_species_id,
            "STAT evolution requires an exact stat ratio",
        ),
        EvolutionTableIssue::InvalidStatRatio {
            source_species_id,
            ratio,
        } => VerificationError::error(
            "invalid_evolution_stat_ratio",
            source_species_id,
            format!("STAT evolution ratio '{ratio}' must be an exact nonempty stat ratio"),
        ),
        EvolutionTableIssue::UnknownStatRatio {
            source_species_id,
            ratio,
        } => VerificationError::error(
            "unknown_evolution_stat_ratio",
            source_species_id,
            format!("STAT evolution uses unknown ratio '{ratio}'"),
        ),
        EvolutionTableIssue::InvalidMethod {
            source_species_id,
            method,
        } => VerificationError::error(
            "invalid_evolution_method",
            source_species_id,
            format!("evolution method '{method}' must be an exact nonempty method id"),
        ),
        EvolutionTableIssue::UnknownMethod {
            source_species_id,
            method,
        } => VerificationError::error(
            "unknown_evolution_method",
            source_species_id,
            format!("evolution uses unknown method '{method}'"),
        ),
    }
}

fn verify_encounters(
    data: &GameDataSet,
    map_names: &BTreeSet<String>,
    diagnostics: &mut Vec<VerificationError>,
) {
    let species_ids: BTreeSet<String> = data.pokemon.keys().cloned().collect();
    diagnostics.extend(
        wild_encounter_catalog_issues(&data.wild_encounters, map_names, &species_ids)
            .into_iter()
            .map(wild_encounter_catalog_issue_diagnostic),
    );
    diagnostics.extend(
        field_encounter_catalog_issues(&data.field_encounters, map_names, &species_ids)
            .into_iter()
            .map(field_encounter_catalog_issue_diagnostic),
    );
}

fn wild_encounter_catalog_issue_diagnostic(issue: WildEncounterCatalogIssue) -> VerificationError {
    match issue {
        WildEncounterCatalogIssue::InvalidMap { map_name } => VerificationError::error(
            "invalid_encounter_map",
            map_name,
            "wild encounter map id must be an exact nonempty map id",
        ),
        WildEncounterCatalogIssue::UnknownMap { map_name } => VerificationError::error(
            "unknown_encounter_map",
            map_name,
            "wild encounters reference a map that is not loaded",
        ),
        WildEncounterCatalogIssue::InvalidSpecies {
            map_name,
            species_id,
        } => VerificationError::error(
            "invalid_encounter_species",
            map_name,
            format!("wild encounter species '{species_id}' must be an exact nonempty species id"),
        ),
        WildEncounterCatalogIssue::UnknownSpecies {
            map_name,
            species_id,
        } => VerificationError::error(
            "unknown_encounter_species",
            map_name,
            format!("wild encounters reference missing species '{species_id}'"),
        ),
        WildEncounterCatalogIssue::InvalidGrassRateTime { map_name, time_key } => {
            VerificationError::error(
                "invalid_grass_encounter_rate_time",
                map_name,
                format!("grass encounter rate time key '{time_key}' must be exact"),
            )
        }
        WildEncounterCatalogIssue::UnknownGrassRateTime { map_name, time_key } => {
            VerificationError::error(
                "unknown_grass_encounter_rate_time",
                map_name,
                format!("grass encounter rate uses unknown exact time key '{time_key}'"),
            )
        }
        WildEncounterCatalogIssue::MissingGrassRate { map_name, time_key } => {
            VerificationError::error(
                "missing_grass_encounter_rate",
                map_name,
                format!("grass encounters for '{time_key}' require an exact grass rate"),
            )
        }
        WildEncounterCatalogIssue::EmptyGrassSlots { map_name, time_key } => {
            VerificationError::error(
                "empty_grass_encounter_slots",
                map_name,
                format!("grass encounter rate for '{time_key}' has no slots"),
            )
        }
        WildEncounterCatalogIssue::MissingGrassTable { map_name } => VerificationError::error(
            "missing_grass_encounter_table",
            map_name,
            "positive grass encounter rates require a grass encounter table",
        ),
        WildEncounterCatalogIssue::MissingWaterRate { map_name } => VerificationError::error(
            "missing_water_encounter_rate",
            map_name,
            "water encounters require an exact water rate",
        ),
        WildEncounterCatalogIssue::EmptyWaterSlots { map_name, time_key } => {
            VerificationError::error(
                "empty_water_encounter_slots",
                map_name,
                format!("water encounter rate has no slots for '{time_key}'"),
            )
        }
        WildEncounterCatalogIssue::MissingWaterTable { map_name } => VerificationError::error(
            "missing_water_encounter_table",
            map_name,
            "positive water encounter rate requires a water encounter table",
        ),
    }
}

fn field_encounter_catalog_issue_diagnostic(
    issue: FieldEncounterCatalogIssue,
) -> VerificationError {
    match issue {
        FieldEncounterCatalogIssue::InvalidMap { map_name } => VerificationError::error(
            "invalid_field_encounter_map",
            map_name,
            "field encounter map id must be an exact nonempty map id",
        ),
        FieldEncounterCatalogIssue::UnknownMap { map_name } => VerificationError::error(
            "unknown_field_encounter_map",
            map_name,
            "field encounters reference a map that is not loaded",
        ),
        FieldEncounterCatalogIssue::InvalidSpecies {
            map_name,
            species_id,
        } => VerificationError::error(
            "invalid_field_encounter_species",
            map_name,
            format!("field encounter species '{species_id}' must be an exact nonempty species id"),
        ),
        FieldEncounterCatalogIssue::UnknownSpecies {
            map_name,
            species_id,
        } => VerificationError::error(
            "unknown_field_encounter_species",
            map_name,
            format!("field encounters reference missing species '{species_id}'"),
        ),
        FieldEncounterCatalogIssue::InvalidKind { map_name, kind } => VerificationError::error(
            "invalid_field_encounter_kind",
            format!("{map_name}:{kind}"),
            "field encounter table id must be an exact supported table id",
        ),
        FieldEncounterCatalogIssue::EmptyBucket {
            map_name,
            kind,
            bucket,
        } => VerificationError::error(
            "empty_field_encounter_bucket",
            format!("{map_name}:{kind}:{bucket}"),
            format!("{kind} field encounters require a non-empty {bucket} bucket"),
        ),
        FieldEncounterCatalogIssue::ZeroWeight {
            map_name,
            kind,
            bucket,
            entry_index,
            species_id,
        } => VerificationError::error(
            "zero_weight_field_encounter",
            format!("{map_name}:{kind}:{bucket}:{entry_index}"),
            format!("{kind} field encounter {bucket} entry for '{species_id}' has zero weight"),
        ),
        FieldEncounterCatalogIssue::InvalidSleepTurns {
            map_name,
            kind,
            bucket,
            entry_index,
            species_id,
            time,
            sleep_turns,
        } => VerificationError::error(
            "invalid_field_encounter_sleep_turns",
            format!("{map_name}:{kind}:{bucket}:{entry_index}"),
            format!(
                "{kind} field encounter {bucket} entry for '{species_id}' has {:?} sleep counter {sleep_turns}, expected 1..=7",
                time
            ),
        ),
        FieldEncounterCatalogIssue::UnexpectedSleepRule {
            map_name,
            kind,
            bucket,
            entry_index,
            species_id,
            time,
        } => VerificationError::error(
            "unexpected_field_encounter_sleep_rule",
            format!("{map_name}:{kind}:{bucket}:{entry_index}"),
            format!(
                "{kind} field encounter {bucket} entry for '{species_id}' has a {:?} tree-sleep rule",
                time
            ),
        ),
        FieldEncounterCatalogIssue::InvalidWeightTotal {
            map_name,
            kind,
            bucket,
            total_weight,
        } => VerificationError::error(
            "invalid_field_encounter_weight_total",
            format!("{map_name}:{kind}:{bucket}"),
            format!("{kind} field encounter {bucket} weights total {total_weight}, expected 100"),
        ),
    }
}

fn verify_audio_assets(
    asset_root: &AssetRoot,
    data: &GameDataSet,
    diagnostics: &mut Vec<VerificationError>,
) {
    let mut seen_audio_ids = BTreeSet::new();
    let mut seen_audio_paths = BTreeMap::new();
    for audio_asset in &data.audio {
        if audio_asset.id == crystal_core::systems::script_audio::MUSIC_NONE_ID {
            diagnostics.push(VerificationError::error(
                "reserved_silent_music_asset",
                &audio_asset.id,
                "MUSIC_NONE is the PlayMusic/_InitSound control sentinel and must not store an audio payload",
            ));
            continue;
        }
        if let Err(error) = audio_asset.validate() {
            diagnostics.push(VerificationError::error(
                "invalid_audio_asset",
                &audio_asset.id,
                error.to_string(),
            ));
            continue;
        }
        if !seen_audio_ids.insert(audio_asset.id.clone()) {
            diagnostics.push(VerificationError::error(
                "duplicate_audio_asset",
                &audio_asset.id,
                "audio asset ids must be unique across music, sound effects, and cries",
            ));
            continue;
        }
        if let Some(first_id) =
            seen_audio_paths.insert(audio_asset.path.clone(), audio_asset.id.clone())
        {
            diagnostics.push(VerificationError::error(
                "duplicate_audio_path",
                &audio_asset.id,
                format!(
                    "audio asset path '{}' is already declared by audio id '{}'",
                    audio_asset.path, first_id
                ),
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
            Ok(bytes) => match audio_asset.source {
                ModpackAudioSource::Pcm if !bytes.is_empty() => {
                    let Some(format) = &audio_asset.pcm_format else {
                        diagnostics.push(VerificationError::error(
                            "invalid_pcm_file",
                            &audio_asset.id,
                            format!("audio file '{}' is missing pcm_format", audio_asset.path),
                        ));
                        continue;
                    };
                    let frame_size = match format.frame_size_bytes(&audio_asset.id) {
                        Ok(frame_size) => frame_size,
                        Err(error) => {
                            diagnostics.push(VerificationError::error(
                                "invalid_pcm_file",
                                &audio_asset.id,
                                error.to_string(),
                            ));
                            continue;
                        }
                    };
                    if bytes.len() % frame_size != 0 {
                        diagnostics.push(VerificationError::error(
                            "invalid_pcm_file",
                            &audio_asset.id,
                            format!(
                                "audio file '{}' has {} bytes, not a whole number of {}-byte PCM frames",
                                audio_asset.path,
                                bytes.len(),
                                frame_size
                            ),
                        ));
                    }
                }
                ModpackAudioSource::Pcm => diagnostics.push(VerificationError::error(
                    "invalid_pcm_file",
                    &audio_asset.id,
                    format!("audio file '{}' is empty", audio_asset.path),
                )),
                ModpackAudioSource::Midi
                    if bytes.len() >= 22
                        && bytes.starts_with(b"MThd")
                        && &bytes[14..18] == b"MTrk" => {}
                ModpackAudioSource::Midi => diagnostics.push(VerificationError::error(
                    "invalid_midi_file",
                    &audio_asset.id,
                    format!("audio file '{}' is not a valid MIDI file", audio_asset.path),
                )),
            },
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
        if !is_exact_audio_reference_token(music_id) {
            diagnostics.push(VerificationError::error(
                "invalid_map_music_id",
                map_name,
                format!("map music id must be an exact pack token, found {music_id:?}"),
            ));
        } else if music_id != crystal_core::systems::script_audio::MUSIC_NONE_ID
            && !music.contains(music_id)
        {
            diagnostics.push(VerificationError::error(
                "unknown_map_music_id",
                map_name,
                format!("map music references missing music audio id '{music_id}'"),
            ));
        }
    }
}

fn verify_audio_asset_usage(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    let mut used_music = BTreeSet::new();
    let mut used_sound_effects = BTreeSet::new();
    let mut used_cries = BTreeSet::new();

    for module in data.maps.values() {
        if let Some(music_id) = module.attributes.music.as_deref() {
            if is_exact_audio_reference_token(music_id) {
                used_music.insert(music_id.to_string());
            }
        }
        for command in &module.script_audio_commands {
            match command.command.as_str() {
                command_name if SCRIPT_AUDIO_MUSIC_COMMANDS.contains(&command_name) => {
                    insert_exact_optional_audio_id(&mut used_music, command.audio_id.as_deref());
                }
                command_name if SCRIPT_AUDIO_MUSIC_FADE_COMMANDS.contains(&command_name) => {
                    insert_exact_optional_audio_id(&mut used_music, command.audio_id.as_deref());
                }
                command_name if SCRIPT_AUDIO_SOUND_EFFECT_COMMANDS.contains(&command_name) => {
                    insert_exact_optional_audio_id(
                        &mut used_sound_effects,
                        command.audio_id.as_deref(),
                    );
                }
                command_name if SCRIPT_AUDIO_CRY_COMMANDS.contains(&command_name) => {
                    if let Some(species_id) = command.audio_id.as_deref() {
                        if let Some(cry) = data.pokemon_cries.get(species_id) {
                            if is_exact_audio_reference_token(&cry.cry) {
                                used_cries.insert(cry.cry.clone());
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    for trainer in data.trainers.trainers.values() {
        if is_exact_audio_reference_token(&trainer.encounter_music) {
            used_music.insert(trainer.encounter_music.clone());
        }
    }
    for music_id in data.encounter_music_modifiers.modifiers.keys() {
        if is_exact_audio_reference_token(music_id) {
            used_music.insert(music_id.clone());
        }
    }
    if data.special_routines.contains_key("SnorlaxAwake") {
        used_music.insert("MUSIC_POKE_FLUTE_CHANNEL".to_string());
    }
    if data.special_routines.contains_key("GetMysteryGiftItem") {
        used_sound_effects.insert("SFX_ITEM".to_string());
    }
    for cry in data.pokemon_cries.values() {
        if is_exact_audio_reference_token(&cry.cry) {
            used_cries.insert(cry.cry.clone());
        }
    }
    for entry in &data.oak_ratings {
        if is_exact_audio_reference_token(&entry.fanfare) {
            used_sound_effects.insert(entry.fanfare.clone());
        }
    }

    for asset in &data.audio {
        if asset.validate().is_err() {
            continue;
        }
        let used = match asset.kind {
            ModpackAudioKind::Music => used_music.contains(&asset.id),
            ModpackAudioKind::SoundEffect => used_sound_effects.contains(&asset.id),
            ModpackAudioKind::Cry => used_cries.contains(&asset.id),
        };
        if !used {
            diagnostics.push(VerificationError::warning(
                "unused_audio_asset",
                &asset.id,
                format!(
                    "audio asset '{}' is declared but not referenced by the definitive modpack",
                    asset.id
                ),
            ));
        }
    }
}

fn insert_exact_optional_audio_id(target: &mut BTreeSet<String>, audio_id: Option<&str>) {
    if let Some(audio_id) = audio_id {
        if is_exact_audio_reference_token(audio_id) {
            target.insert(audio_id.to_string());
        }
    }
}

fn is_exact_audio_reference_token(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn verify_trainers(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    let species_ids: BTreeSet<String> = data.pokemon.keys().cloned().collect();
    let move_ids: BTreeSet<String> = data.moves.keys().cloned().collect();
    diagnostics.extend(
        trainer_catalog_issues(&data.trainers, &species_ids, &data.items, &move_ids)
            .into_iter()
            .map(trainer_catalog_issue_diagnostic),
    );
    verify_scripted_battle_requests(data, diagnostics);
}

fn verify_trainer_class_names(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    for (trainer_class, display_name) in &data.trainer_class_names {
        if !is_exact_scripted_battle_reference_token(trainer_class) {
            diagnostics.push(VerificationError::error(
                "invalid_trainer_class_name_id",
                format!("trainer_class_names:{trainer_class}"),
                format!(
                    "trainer class display-name id must be exact and nonempty, found {trainer_class:?}"
                ),
            ));
        }
        if display_name.is_empty()
            || display_name.trim() != display_name
            || display_name.chars().any(char::is_control)
        {
            diagnostics.push(VerificationError::error(
                "invalid_trainer_class_display_name",
                format!("trainer_class_names:{trainer_class}"),
                format!(
                    "trainer class display name must be exact nonempty text, found {display_name:?}"
                ),
            ));
        }
    }
    for trainer in data.trainers.trainers.values() {
        if !data
            .trainer_class_names
            .contains_key(&trainer.trainer_class)
        {
            diagnostics.push(VerificationError::error(
                "missing_trainer_class_display_name",
                format!("trainer_class_names:{}", trainer.trainer_class),
                format!(
                    "trainer '{}' references class '{}' without an authoritative display name",
                    trainer.trainer_id, trainer.trainer_class
                ),
            ));
        }
    }
}

fn verify_scripted_battle_requests(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    for (map_name, module) in &data.maps {
        verify_trainer_object_scripts(map_name, module, diagnostics);
        verify_unique_script_command_positions(
            map_name,
            "scripted_trainer_battle_start",
            module.scripted_trainer_battles.iter().map(|battle| {
                (
                    battle.source_script.as_str(),
                    battle.startbattle_command_index,
                )
            }),
            diagnostics,
        );
        verify_unique_script_command_positions(
            map_name,
            "scripted_wild_battle_start",
            module.scripted_wild_battles.iter().map(|battle| {
                (
                    battle.source_script.as_str(),
                    battle.startbattle_command_index,
                )
            }),
            diagnostics,
        );
        for (source_script, request) in &module.trainer_scripts {
            verify_trainer_battle_request(
                map_name,
                source_script,
                None,
                request,
                &data.trainers,
                diagnostics,
            );
        }
        for battle in &module.scripted_trainer_battles {
            verify_trainer_battle_request(
                map_name,
                &battle.source_script,
                Some(battle.loadtrainer_command_index),
                &battle.request,
                &data.trainers,
                diagnostics,
            );
        }
        for battle in &module.scripted_wild_battles {
            verify_static_wild_battle_request(
                map_name,
                &battle.source_script,
                battle.loadwildmon_command_index,
                &battle.request,
                &data.pokemon,
                diagnostics,
            );
        }
    }
}

fn verify_trainer_object_scripts(
    map_name: &str,
    module: &MapModule,
    diagnostics: &mut Vec<VerificationError>,
) {
    let trainer_objects: BTreeMap<&str, Vec<&ObjectEvent>> = module
        .objects
        .iter()
        .filter(|object| object.object_type == "OBJECTTYPE_TRAINER")
        .fold(BTreeMap::new(), |mut objects_by_script, object| {
            objects_by_script
                .entry(object.script.as_str())
                .or_default()
                .push(object);
            objects_by_script
        });

    for (script, objects) in &trainer_objects {
        if objects.len() > 1 {
            diagnostics.push(VerificationError::error(
                "trainer_duplicate_object_script",
                format!("{map_name}:{script}"),
                format!(
                    "trainer object script '{script}' resolves to {} OBJECTTYPE_TRAINER objects",
                    objects.len()
                ),
            ));
        }
        if !module.trainer_scripts.contains_key(*script) {
            for object in objects {
                if object.script == "ObjectEvent"
                    && scripted_trainer_object_is_backed_by_exact_script(module, object)
                {
                    continue;
                }
                diagnostics.push(VerificationError::error(
                    "trainer_object_missing_battle_request",
                    format!(
                        "{map_name}:{}",
                        object
                            .object_identifier
                            .as_deref()
                            .unwrap_or(object.script.as_str())
                    ),
                    format!(
                        "trainer object script '{}' has no exact trainer battle request",
                        object.script
                    ),
                ));
            }
        } else if let Some(request) = module.trainer_scripts.get(*script) {
            for object in objects {
                if object.event_flag != request.event_flag {
                    diagnostics.push(VerificationError::error(
                        "trainer_object_event_flag_mismatch",
                        format!(
                            "{map_name}:{}",
                            object
                                .object_identifier
                                .as_deref()
                                .unwrap_or(object.script.as_str())
                        ),
                        format!(
                            "trainer object script '{}' event flag '{}' does not match trainer battle request event flag '{}'",
                            object.script, object.event_flag, request.event_flag
                        ),
                    ));
                }
            }
        }
    }

    for source_script in module.trainer_scripts.keys() {
        if !trainer_objects.contains_key(source_script.as_str()) {
            diagnostics.push(VerificationError::error(
                "trainer_battle_request_missing_object",
                format!("{map_name}:trainer_script:{source_script}"),
                format!(
                    "trainer battle request '{source_script}' has no exact OBJECTTYPE_TRAINER object"
                ),
            ));
        }
    }
}

fn scripted_trainer_object_is_backed_by_exact_script(
    module: &MapModule,
    object: &ObjectEvent,
) -> bool {
    let Some(object_id) = object.object_identifier.as_deref() else {
        return false;
    };
    module.scripted_trainer_battles.iter().any(|battle| {
        script_references_exact_arg(&module.scripts, &battle.source_script, object_id)
            && scripted_trainer_continuation_reaches_event_flag(module, battle, &object.event_flag)
    })
}

fn scripted_trainer_continuation_reaches_event_flag(
    module: &MapModule,
    battle: &ScriptedTrainerBattle,
    expected_event_flag: &str,
) -> bool {
    let mut pending = VecDeque::from([(
        battle.source_script.clone(),
        battle.startbattle_command_index + 1,
    )]);
    let mut visited = BTreeSet::new();

    while let Some((source_script, command_index)) = pending.pop_front() {
        if !visited.insert((source_script.clone(), command_index)) {
            continue;
        }
        let Some(entries) = module.scripts.get(&source_script).and_then(Value::as_array) else {
            continue;
        };
        let Some(entry) = entries.get(command_index) else {
            continue;
        };
        let Some(command) = entry.get("command").and_then(Value::as_str) else {
            continue;
        };
        let args = entry
            .get("args")
            .and_then(Value::as_array)
            .and_then(|args| args.iter().map(Value::as_str).collect::<Option<Vec<_>>>());

        if command == "setevent"
            && args
                .as_deref()
                .is_some_and(|args| args == [expected_event_flag])
        {
            return true;
        }

        let next = (source_script.clone(), command_index + 1);
        match command {
            "sjump" | "farsjump" | "stopandsjump" => {
                if let Some(target) =
                    args.as_deref()
                        .filter(|args| args.len() == 1)
                        .and_then(|args| {
                            resolve_script_target_label(&module.scripts, &source_script, args[0])
                        })
                {
                    pending.push_back((target, 0));
                }
            }
            "loadtrainer" | "loadwildmon" | "catchtutorial" | "startbattle" | "end" | "endall"
            | "endcallback" | "reloadend" | "return" | "jumpstd" | "halloffame" | "credits"
            | "iftrue" | "iffalse" | "ifequal" | "ifnotequal" | "ifgreater" | "ifless"
            | "scall" | "farscall" => {}
            _ => pending.push_back(next),
        }
    }

    false
}

fn script_references_exact_arg(
    scripts: &BTreeMap<String, Value>,
    source_script: &str,
    expected_arg: &str,
) -> bool {
    let Some(entries) = scripts.get(source_script).and_then(Value::as_array) else {
        return false;
    };
    entries.iter().any(|entry| {
        entry
            .get("args")
            .and_then(Value::as_array)
            .is_some_and(|args| args.iter().any(|arg| arg.as_str() == Some(expected_arg)))
    })
}

fn verify_trainer_battle_request(
    map_name: &str,
    source_script: &str,
    command_index: Option<usize>,
    request: &TrainerBattleRequest,
    trainers: &TrainerCatalog,
    diagnostics: &mut Vec<VerificationError>,
) {
    let subject = battle_request_subject(map_name, source_script, command_index);
    if !is_exact_scripted_battle_reference_token(&request.trainer_id) {
        diagnostics.push(VerificationError::error(
            "invalid_scripted_trainer_id",
            &subject,
            format!(
                "trainer battle request trainer_id must be exact and nonempty, found {:?}",
                request.trainer_id
            ),
        ));
        return;
    }
    if !is_exact_scripted_battle_reference_token(&request.trainer_class) {
        diagnostics.push(VerificationError::error(
            "invalid_scripted_trainer_class",
            &subject,
            format!(
                "trainer battle request trainer_class must be exact and nonempty, found {:?}",
                request.trainer_class
            ),
        ));
        return;
    }
    let Some(trainer) = trainers.get(&request.trainer_id) else {
        diagnostics.push(VerificationError::error(
            "unknown_scripted_trainer",
            &subject,
            format!(
                "trainer battle request references missing trainer '{}'",
                request.trainer_id
            ),
        ));
        return;
    };
    if trainer.trainer_class != request.trainer_class {
        diagnostics.push(VerificationError::error(
            "scripted_trainer_class_mismatch",
            &subject,
            format!(
                "trainer battle request class '{}' does not match trainer '{}' class '{}'",
                request.trainer_class, request.trainer_id, trainer.trainer_class
            ),
        ));
    }
}

fn verify_static_wild_battle_request(
    map_name: &str,
    source_script: &str,
    command_index: usize,
    request: &StaticWildBattleRequest,
    species: &BTreeMap<String, PokemonSpecies>,
    diagnostics: &mut Vec<VerificationError>,
) {
    let subject = battle_request_subject(map_name, source_script, Some(command_index));
    if !is_exact_scripted_battle_reference_token(&request.species) {
        diagnostics.push(VerificationError::error(
            "invalid_scripted_wild_species",
            &subject,
            format!(
                "static wild battle request species must be exact and nonempty, found {:?}",
                request.species
            ),
        ));
    } else if !species.contains_key(&request.species) {
        diagnostics.push(VerificationError::error(
            "unknown_scripted_wild_species",
            &subject,
            format!(
                "static wild battle request references missing species '{}'",
                request.species
            ),
        ));
    }
    if request.level == 0 {
        diagnostics.push(VerificationError::error(
            "invalid_scripted_wild_level",
            &subject,
            format!(
                "static wild battle request for species '{}' has zero level",
                request.species
            ),
        ));
    }
}

fn is_exact_scripted_battle_reference_token(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn battle_request_subject(
    map_name: &str,
    source_script: &str,
    command_index: Option<usize>,
) -> String {
    match command_index {
        Some(command_index) => format!("{map_name}:{source_script}:{command_index}"),
        None => format!("{map_name}:{source_script}"),
    }
}

fn trainer_catalog_issue_diagnostic(issue: TrainerCatalogIssue) -> VerificationError {
    match issue {
        TrainerCatalogIssue::KeyMismatch { key, trainer_id } => VerificationError::error(
            "trainer_catalog_key_mismatch",
            key,
            format!("trainer catalog key does not match trainer_id '{trainer_id}'"),
        ),
        TrainerCatalogIssue::InvalidTrainerId { trainer_id } => VerificationError::error(
            "invalid_trainer_id",
            trainer_id,
            "trainer_id must be an exact nonempty id",
        ),
        TrainerCatalogIssue::MissingTrainerClass { trainer_id } => VerificationError::error(
            "missing_trainer_class",
            trainer_id,
            "trainer must declare an exact trainer class",
        ),
        TrainerCatalogIssue::InvalidTrainerClass {
            trainer_id,
            trainer_class,
        } => VerificationError::error(
            "invalid_trainer_class",
            trainer_id,
            format!("trainer class '{trainer_class}' must be an exact nonempty id"),
        ),
        TrainerCatalogIssue::EmptyParty { trainer_id } => VerificationError::error(
            "empty_trainer_party",
            trainer_id,
            "trainer must declare an explicit nonempty party",
        ),
        TrainerCatalogIssue::InvalidPartySpecies {
            trainer_id,
            slot,
            species,
        } => VerificationError::error(
            "invalid_trainer_party_species",
            format!("{trainer_id}:party:{slot}"),
            format!("trainer party species '{species}' must be an exact nonempty species id"),
        ),
        TrainerCatalogIssue::UnknownPartySpecies {
            trainer_id,
            slot,
            species,
        } => VerificationError::error(
            "unknown_trainer_party_species",
            format!("{trainer_id}:party:{slot}"),
            format!("trainer party references missing species '{species}'"),
        ),
        TrainerCatalogIssue::InvalidPartyItem {
            trainer_id,
            slot,
            item_id,
        } => VerificationError::error(
            "invalid_trainer_party_item",
            format!("{trainer_id}:party:{slot}"),
            format!("trainer party item '{item_id}' must be an exact nonempty item id"),
        ),
        TrainerCatalogIssue::UnknownPartyItem {
            trainer_id,
            slot,
            item_id,
        } => VerificationError::error(
            "unknown_trainer_party_item",
            format!("{trainer_id}:party:{slot}"),
            format!("trainer party references missing item '{item_id}'"),
        ),
        TrainerCatalogIssue::InvalidBattleItem {
            trainer_id,
            slot,
            item_id,
        } => VerificationError::error(
            "invalid_trainer_battle_item",
            format!("{trainer_id}:item:{slot}"),
            format!("trainer battle item '{item_id}' must be an exact nonempty item id"),
        ),
        TrainerCatalogIssue::UnknownBattleItem {
            trainer_id,
            slot,
            item_id,
        } => VerificationError::error(
            "unknown_trainer_battle_item",
            format!("{trainer_id}:item:{slot}"),
            format!("trainer battle item references missing item '{item_id}'"),
        ),
        TrainerCatalogIssue::UnusableBattleItem {
            trainer_id,
            slot,
            item_id,
        } => VerificationError::error(
            "unusable_trainer_battle_item",
            format!("{trainer_id}:item:{slot}"),
            format!("trainer battle item references item '{item_id}' that is not battle usable"),
        ),
        TrainerCatalogIssue::InvalidPartyMove {
            trainer_id,
            slot,
            move_id,
        } => VerificationError::error(
            "invalid_trainer_party_move",
            format!("{trainer_id}:party:{slot}"),
            format!("trainer party move '{move_id}' must be an exact nonempty move id"),
        ),
        TrainerCatalogIssue::UnknownPartyMove {
            trainer_id,
            slot,
            move_id,
        } => VerificationError::error(
            "unknown_trainer_party_move",
            format!("{trainer_id}:party:{slot}"),
            format!("trainer party references missing move '{move_id}'"),
        ),
    }
}

fn verify_runtime_title_screen(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    let presentation = &data.runtime_title_screen.program;
    if presentation.schema_version != 1 {
        diagnostics.push(VerificationError::error(
            "invalid_runtime_title_program_schema",
            "runtime_title_screen",
            format!(
                "runtime title presentation requires schema_version 1, found {}",
                presentation.schema_version
            ),
        ));
    } else {
        for entrypoint in [
            "boot",
            "intro",
            "title",
            "main_menu",
            "continue",
            "new_game",
            "delete_save",
            "reset_clock",
        ] {
            match presentation.entrypoints.get(entrypoint) {
                Some(target) if presentation.blocks.contains_key(target) => {}
                Some(target) => diagnostics.push(VerificationError::error(
                    "unknown_runtime_title_entrypoint_block",
                    format!("runtime_title_screen:{entrypoint}"),
                    format!("entrypoint targets missing block '{target}'"),
                )),
                None => diagnostics.push(VerificationError::error(
                    "missing_runtime_title_entrypoint",
                    format!("runtime_title_screen:{entrypoint}"),
                    "runtime title presentation entrypoint is missing",
                )),
            }
        }
    }
    match data.story_event_script_constants.global.get("SPAWN_HOME") {
        Some(value) => match u16::try_from(*value) {
            Ok(spawn_identifier)
                if data
                    .runtime_spawn_points
                    .contains_key(&spawn_identifier.to_string()) => {}
            Ok(spawn_identifier) => diagnostics.push(VerificationError::error(
                "unknown_runtime_title_spawn_identifier",
                "SPAWN_HOME",
                format!("source constant SPAWN_HOME={spawn_identifier} has no runtime spawn point"),
            )),
            Err(_) => diagnostics.push(VerificationError::error(
                "invalid_runtime_title_spawn_identifier",
                "SPAWN_HOME",
                format!("source constant SPAWN_HOME={value} is not a u16"),
            )),
        },
        None => diagnostics.push(VerificationError::error(
            "missing_runtime_title_spawn_identifier",
            "SPAWN_HOME",
            "new-game startup requires exported source constant SPAWN_HOME",
        )),
    }

    let Some(title_music) = data.runtime_title_screen.title_music.as_deref() else {
        diagnostics.push(VerificationError::error(
            "missing_runtime_title_music_id",
            "runtime_title_screen",
            "runtime title screen requires explicit title music",
        ));
        return;
    };
    let (music, _, _) = script_audio_catalog_ids(data);
    if !is_exact_audio_reference_token(title_music) {
        diagnostics.push(VerificationError::error(
            "invalid_runtime_title_music_id",
            "runtime_title_screen",
            format!("runtime title music id must be an exact pack token, found {title_music:?}"),
        ));
    } else if !music.contains(title_music) {
        diagnostics.push(VerificationError::error(
            "unknown_runtime_title_music_id",
            "runtime_title_screen",
            format!("runtime title screen references missing music audio id '{title_music}'"),
        ));
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
        } else if !is_exact_audio_reference_token(&trainer.encounter_music) {
            diagnostics.push(VerificationError::error(
                "invalid_trainer_encounter_music",
                trainer_id,
                format!(
                    "trainer encounter music id must be an exact pack token, found {:?}",
                    trainer.encounter_music
                ),
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
    let ball_items: BTreeMap<String, Item> = data
        .items
        .values()
        .filter(|item| item.pocket == ITEM_POCKET_BALL)
        .map(|item| (item.script_name.clone(), item.clone()))
        .collect();
    let has_ball_pocket_items = data
        .items
        .values()
        .any(|item| item.pocket == ITEM_POCKET_BALL);
    let species_ids: BTreeSet<String> = data.pokemon.keys().cloned().collect();
    diagnostics.extend(
        capture_rules_issues(
            &data.capture_rules,
            &species_ids,
            &ball_items,
            has_ball_pocket_items,
        )
        .into_iter()
        .map(capture_rules_issue_diagnostic),
    );
}

fn capture_rules_issue_diagnostic(issue: CaptureRulesIssue) -> VerificationError {
    match issue {
        CaptureRulesIssue::MissingBallRules => VerificationError::error(
            "missing_capture_ball_rules",
            "capture_rules:ball_rules",
            "capture ball rules must be declared when BALL pocket items exist",
        ),
        CaptureRulesIssue::InvalidFastBallSpecies { species } => VerificationError::error(
            "invalid_fast_ball_species",
            &species,
            format!("Fast Ball species id must be exact and nonempty, found {species:?}"),
        ),
        CaptureRulesIssue::UnknownFastBallSpecies { species } => VerificationError::error(
            "unknown_fast_ball_species",
            &species,
            "Fast Ball rule references a species that is not loaded",
        ),
        CaptureRulesIssue::InvalidHeavyBallSpecies { species } => VerificationError::error(
            "invalid_heavy_ball_species",
            &species,
            format!("Heavy Ball species id must be exact and nonempty, found {species:?}"),
        ),
        CaptureRulesIssue::UnknownHeavyBallSpecies { species } => VerificationError::error(
            "unknown_heavy_ball_species",
            &species,
            "Heavy Ball rule references a species that is not loaded",
        ),
        CaptureRulesIssue::InvalidBallRuleItem { ball_id } => VerificationError::error(
            "invalid_capture_ball_rule_item",
            format!("capture_rules:ball_rules:{ball_id}"),
            format!("capture ball rule item id must be exact and nonempty, found {ball_id:?}"),
        ),
        CaptureRulesIssue::UnknownBallRuleItem { ball_id } => VerificationError::error(
            "unknown_capture_ball_rule_item",
            format!("capture_rules:ball_rules:{ball_id}"),
            format!("capture ball rule references missing BALL pocket item '{ball_id}'"),
        ),
        CaptureRulesIssue::UnusableBallRuleItem { ball_id } => VerificationError::error(
            "unusable_capture_ball_rule_item",
            format!("capture_rules:ball_rules:{ball_id}"),
            format!("capture ball rule references item '{ball_id}' that is not battle usable"),
        ),
        CaptureRulesIssue::InvalidGuaranteedCaptureBall { ball_id } => VerificationError::error(
            "invalid_guaranteed_capture_ball",
            "capture_rules:guaranteed_capture_balls",
            format!("guaranteed capture ball id must be exact and nonempty, found {ball_id:?}"),
        ),
        CaptureRulesIssue::UnknownGuaranteedCaptureBall { ball_id } => VerificationError::error(
            "unknown_guaranteed_capture_ball",
            "capture_rules:guaranteed_capture_balls",
            format!("guaranteed capture ball references missing BALL pocket item '{ball_id}'"),
        ),
        CaptureRulesIssue::UnusableGuaranteedCaptureBall { ball_id } => VerificationError::error(
            "unusable_guaranteed_capture_ball",
            "capture_rules:guaranteed_capture_balls",
            format!(
                "guaranteed capture ball references item '{ball_id}' that is not battle usable"
            ),
        ),
        CaptureRulesIssue::InvalidBallRule { ball_id, issue } => {
            capture_ball_rule_issue_diagnostic(&ball_id, issue)
        }
    }
}

fn capture_ball_rule_issue_diagnostic(
    ball_id: &str,
    issue: CaptureBallRuleIssue,
) -> VerificationError {
    let subject = format!("capture_rules:ball_rules:{ball_id}");
    match issue {
        CaptureBallRuleIssue::InvalidBallId => VerificationError::error(
            "invalid_capture_ball_id",
            &subject,
            "capture ball id must be an exact nonempty id",
        ),
        CaptureBallRuleIssue::InvalidBattleType => VerificationError::error(
            "invalid_capture_ball_battle_type",
            &subject,
            "capture ball battle type must be exact when present",
        ),
        CaptureBallRuleIssue::InvalidMultiplierDenominator => VerificationError::error(
            "invalid_capture_ball_multiplier",
            &subject,
            "capture ball multiplier denominator must be nonzero",
        ),
    }
}

fn verify_capture_wobble_probabilities(
    data: &GameDataSet,
    diagnostics: &mut Vec<VerificationError>,
) {
    let has_ball_pocket_items = data
        .items
        .values()
        .any(|item| item.pocket == ITEM_POCKET_BALL);
    diagnostics.extend(
        capture_wobble_probability_issues(
            &data.capture_wobble_probabilities,
            has_ball_pocket_items,
        )
        .into_iter()
        .map(capture_wobble_probability_issue_diagnostic),
    );
}

fn capture_wobble_probability_issue_diagnostic(
    issue: CaptureWobbleProbabilityIssue,
) -> VerificationError {
    match issue {
        CaptureWobbleProbabilityIssue::MissingTable => VerificationError::error(
            "missing_capture_wobble_probabilities",
            "capture_wobble_probabilities",
            "capture wobble probabilities must be declared when capture balls exist",
        ),
        CaptureWobbleProbabilityIssue::InvalidCatchRate => VerificationError::error(
            "invalid_capture_wobble_catch_rate",
            "capture_wobble_probabilities",
            "capture wobble catch rates must be in 1..=255",
        ),
        CaptureWobbleProbabilityIssue::UnorderedCatchRate {
            catch_rate,
            previous,
        } => VerificationError::error(
            "unordered_capture_wobble_probability",
            "capture_wobble_probabilities",
            format!("capture wobble catch rate {catch_rate} appears after {previous}"),
        ),
        CaptureWobbleProbabilityIssue::IncompleteTable => VerificationError::error(
            "incomplete_capture_wobble_probabilities",
            "capture_wobble_probabilities",
            "capture wobble probabilities must end at catch rate 255",
        ),
    }
}

fn verify_marts(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    diagnostics.extend(
        mart_catalog_issues(&data.marts, &data.items)
            .into_iter()
            .map(mart_catalog_issue_diagnostic),
    );
}

fn mart_catalog_issue_diagnostic(issue: MartCatalogIssue) -> VerificationError {
    match issue {
        MartCatalogIssue::EmptyMartId { mart_id } => {
            VerificationError::error("empty_mart_id", mart_id, "mart id is required")
        }
        MartCatalogIssue::InvalidMartId { mart_id } => VerificationError::error(
            "invalid_mart_id",
            &mart_id,
            format!("mart id must be exact and untrimmed, found {mart_id:?}"),
        ),
        MartCatalogIssue::InvalidItem { mart_id, item_id } => VerificationError::error(
            "invalid_mart_item",
            mart_id,
            format!("mart item id must be exact and untrimmed, found {item_id:?}"),
        ),
        MartCatalogIssue::UnknownItem { mart_id, item_id } => VerificationError::error(
            "unknown_mart_item",
            mart_id,
            format!("mart references missing item '{item_id}'"),
        ),
    }
}

fn verify_fruit_trees(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    diagnostics.extend(
        fruit_tree_catalog_issues(&data.fruit_trees, &data.items)
            .into_iter()
            .map(fruit_tree_catalog_issue_diagnostic),
    );
}

fn fruit_tree_catalog_issue_diagnostic(issue: FruitTreeCatalogIssue) -> VerificationError {
    match issue {
        FruitTreeCatalogIssue::EmptyFruitTreeId { fruit_tree_id } => VerificationError::error(
            "empty_fruit_tree_id",
            format!("fruit_trees:{fruit_tree_id}"),
            "fruit tree id must not be empty",
        ),
        FruitTreeCatalogIssue::InvalidFruitTreeId { fruit_tree_id } => VerificationError::error(
            "invalid_fruit_tree_id",
            format!("fruit_trees:{fruit_tree_id}"),
            format!("fruit tree id must be exact and untrimmed, found {fruit_tree_id:?}"),
        ),
        FruitTreeCatalogIssue::UnknownItem {
            fruit_tree_id,
            item_id,
        } => VerificationError::error(
            "unknown_fruit_tree_item",
            format!("fruit_trees:{fruit_tree_id}"),
            format!("fruit tree references missing item '{item_id}'"),
        ),
        FruitTreeCatalogIssue::InvalidItem {
            fruit_tree_id,
            item_id,
        } => VerificationError::error(
            "invalid_fruit_tree_item",
            format!("fruit_trees:{fruit_tree_id}"),
            format!("fruit tree item id must be exact and untrimmed, found {item_id:?}"),
        ),
    }
}

fn verify_script_item_grants(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    for (map_name, module) in &data.maps {
        verify_unique_script_command_positions(
            map_name,
            "script_item_grants",
            module
                .script_item_grants
                .iter()
                .map(|grant| (grant.source_script.as_str(), grant.command_index)),
            diagnostics,
        );
        verify_unique_script_command_positions(
            map_name,
            "script_item_checks",
            module
                .script_item_checks
                .iter()
                .map(|access| (access.source_script.as_str(), access.command_index)),
            diagnostics,
        );
        verify_unique_script_command_positions(
            map_name,
            "script_item_takes",
            module
                .script_item_takes
                .iter()
                .map(|access| (access.source_script.as_str(), access.command_index)),
            diagnostics,
        );
        for grant in &module.script_item_grants {
            let subject = format!("{map_name}:{}:{}", grant.source_script, grant.command_index);
            diagnostics.extend(
                script_item_grant_issues(grant, &data.items)
                    .into_iter()
                    .map(|issue| script_item_grant_issue_diagnostic(&subject, issue)),
            );
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
    let subject = format!(
        "{map_name}:{}:{}",
        access.source_script, access.command_index
    );
    diagnostics.extend(
        script_item_access_issues(access, &data.items)
            .into_iter()
            .map(|issue| script_item_access_issue_diagnostic(&subject, command, issue)),
    );
}

fn script_item_grant_issue_diagnostic(
    subject: &str,
    issue: ScriptItemGrantIssue,
) -> VerificationError {
    match issue {
        ScriptItemGrantIssue::InvalidCommand { command } => VerificationError::error(
            "invalid_script_item_grant_command",
            subject,
            format!("script item grant command must be exact Crystal syntax, found {command:?}"),
        ),
        ScriptItemGrantIssue::InvalidItem { item_id } => VerificationError::error(
            "invalid_script_item_grant_item",
            subject,
            format!("script item grant item id must be exact and nonempty, found {item_id:?}"),
        ),
        ScriptItemGrantIssue::UnknownItem { item_id } => VerificationError::error(
            "unknown_script_item_grant_item",
            subject,
            format!("script item grant references missing item '{item_id}'"),
        ),
        ScriptItemGrantIssue::InvalidQuantity => VerificationError::error(
            "invalid_script_item_grant_quantity",
            subject,
            "script item grant quantity must be greater than zero",
        ),
    }
}

fn script_item_access_issue_diagnostic(
    subject: &str,
    command: &str,
    issue: ScriptItemAccessIssue,
) -> VerificationError {
    match issue {
        ScriptItemAccessIssue::InvalidCommand { command } => VerificationError::error(
            "invalid_script_item_access_command",
            subject,
            format!("{command} command must be exact Crystal syntax"),
        ),
        ScriptItemAccessIssue::InvalidItem { item_id } => VerificationError::error(
            "invalid_script_item_access_item",
            subject,
            format!("{command} item id must be exact and nonempty, found {item_id:?}"),
        ),
        ScriptItemAccessIssue::UnknownItem { item_id } => VerificationError::error(
            "unknown_script_item_access_item",
            subject,
            format!("{command} references missing item '{item_id}'"),
        ),
    }
}

fn verify_map_command_source_scripts(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    for (map_name, module) in &data.maps {
        for source_script in module.trainer_scripts.keys() {
            verify_script_source_reference(
                map_name,
                module,
                "trainer_script",
                source_script,
                None,
                diagnostics,
            );
        }
        for battle in &module.scripted_trainer_battles {
            verify_script_source_reference(
                map_name,
                module,
                "scripted_trainer_battle",
                &battle.source_script,
                Some(battle.loadtrainer_command_index),
                diagnostics,
            );
        }
        for battle in &module.scripted_wild_battles {
            verify_script_source_reference(
                map_name,
                module,
                "scripted_wild_battle",
                &battle.source_script,
                Some(battle.loadwildmon_command_index),
                diagnostics,
            );
        }
        for grant in &module.script_item_grants {
            verify_script_source_reference(
                map_name,
                module,
                "script_item_grant",
                &grant.source_script,
                Some(grant.command_index),
                diagnostics,
            );
        }
        for access in &module.script_item_checks {
            verify_script_source_reference(
                map_name,
                module,
                "script_item_check",
                &access.source_script,
                Some(access.command_index),
                diagnostics,
            );
        }
        for access in &module.script_item_takes {
            verify_script_source_reference(
                map_name,
                module,
                "script_item_take",
                &access.source_script,
                Some(access.command_index),
                diagnostics,
            );
        }
        for command in &module.script_economy_commands {
            verify_script_source_reference(
                map_name,
                module,
                "script_economy",
                &command.source_script,
                Some(command.command_index),
                diagnostics,
            );
        }
        for gift in &module.gift_pokemon_scripts {
            verify_script_source_reference(
                map_name,
                module,
                "gift_pokemon",
                &gift.source_script,
                Some(gift.command_index),
                diagnostics,
            );
        }
        for command in &module.script_flag_commands {
            verify_script_source_reference(
                map_name,
                module,
                "script_flag",
                &command.source_script,
                Some(command.command_index),
                diagnostics,
            );
        }
        for command in &module.script_scene_commands {
            verify_script_source_reference(
                map_name,
                module,
                "script_scene",
                &command.source_script,
                Some(command.command_index),
                diagnostics,
            );
        }
        for command in &module.script_audio_commands {
            verify_script_source_reference(
                map_name,
                module,
                "script_audio",
                &command.source_script,
                Some(command.command_index),
                diagnostics,
            );
        }
        for change in &module.script_block_changes {
            verify_script_source_reference(
                map_name,
                module,
                "script_block",
                &change.source_script,
                Some(change.command_index),
                diagnostics,
            );
        }
        for command in &module.script_object_commands {
            verify_script_source_reference(
                map_name,
                module,
                "script_object",
                &command.source_script,
                Some(command.command_index),
                diagnostics,
            );
        }
        for movement in &module.script_movements {
            if let Some(source_script) = movement.source_script.as_deref() {
                verify_script_source_reference(
                    map_name,
                    module,
                    "script_movement",
                    source_script,
                    None,
                    diagnostics,
                );
            }
        }
        for command in &module.script_map_commands {
            verify_script_source_reference(
                map_name,
                module,
                "script_map",
                &command.source_script,
                Some(command.command_index),
                diagnostics,
            );
        }
        for command in &module.script_text_commands {
            verify_script_source_reference(
                map_name,
                module,
                "script_text",
                &command.source_script,
                Some(command.command_index),
                diagnostics,
            );
        }
        for command in &module.script_variable_commands {
            verify_script_source_reference(
                map_name,
                module,
                "script_variable",
                &command.source_script,
                Some(command.command_index),
                diagnostics,
            );
        }
        for command in &module.script_control_commands {
            verify_script_source_reference(
                map_name,
                module,
                "script_control",
                &command.source_script,
                Some(command.command_index),
                diagnostics,
            );
        }
        for pickup in &module.script_field_pickups {
            verify_script_source_reference(
                map_name,
                module,
                "script_field_pickup",
                &pickup.source_script,
                Some(pickup.command_index),
                diagnostics,
            );
        }
        for command in &module.script_shop_commands {
            verify_script_source_reference(
                map_name,
                module,
                "script_shop",
                &command.source_script,
                Some(command.command_index),
                diagnostics,
            );
        }
        for command in &module.script_phone_commands {
            verify_script_source_reference(
                map_name,
                module,
                "script_phone",
                &command.source_script,
                Some(command.command_index),
                diagnostics,
            );
        }
        for command in &module.script_runtime_commands {
            verify_script_source_reference(
                map_name,
                module,
                "script_runtime",
                &command.source_script,
                Some(command.command_index),
                diagnostics,
            );
        }
        for command in &module.script_swarm_commands {
            verify_script_source_reference(
                map_name,
                module,
                "script_swarm",
                &command.source_script,
                Some(command.command_index),
                diagnostics,
            );
        }
    }
}

fn verify_script_source_reference(
    map_name: &str,
    module: &MapModule,
    category: &str,
    source_script: &str,
    command_index: Option<usize>,
    diagnostics: &mut Vec<VerificationError>,
) {
    if module.scripts.contains_key(source_script) {
        return;
    }
    let subject = match command_index {
        Some(command_index) => format!("{map_name}:{category}:{source_script}:{command_index}"),
        None => format!("{map_name}:{category}:{source_script}"),
    };
    if !is_exact_script_label_reference_token(source_script) {
        diagnostics.push(VerificationError::error(
            "invalid_command_source_script",
            subject,
            format!("{category} source script label must be exact, found {source_script:?}"),
        ));
        return;
    }
    diagnostics.push(VerificationError::error(
        "unknown_command_source_script",
        subject,
        format!("{category} references missing exact source script '{source_script}'"),
    ));
}

fn verify_script_economy_commands(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    let constants = economy_constants(data);
    for (map_name, module) in &data.maps {
        verify_unique_script_command_positions(
            map_name,
            "script_economy_commands",
            module
                .script_economy_commands
                .iter()
                .map(|command| (command.source_script.as_str(), command.command_index)),
            diagnostics,
        );
        for command in &module.script_economy_commands {
            let subject = format!(
                "{map_name}:{}:{}",
                command.source_script, command.command_index
            );
            diagnostics.extend(
                script_economy_command_issues(command, &constants)
                    .into_iter()
                    .map(|issue| script_economy_command_issue_diagnostic(&subject, command, issue)),
            );
        }
    }
}

fn script_economy_command_issue_diagnostic(
    subject: &str,
    command: &ScriptEconomyCommand,
    issue: ScriptEconomyCommandIssue,
) -> VerificationError {
    match issue {
        ScriptEconomyCommandIssue::InvalidCommand => VerificationError::error(
            "invalid_script_economy_command",
            subject,
            format!(
                "economy command must be exact lowercase with no surrounding whitespace, found {:?}",
                command.command
            ),
        ),
        ScriptEconomyCommandIssue::UnknownCommand => VerificationError::error(
            "unknown_script_economy_command",
            subject,
            format!("unknown economy command '{}'", command.command),
        ),
        ScriptEconomyCommandIssue::MissingMoneyAccount => VerificationError::error(
            "missing_script_money_account",
            subject,
            "money command is missing account id",
        ),
        ScriptEconomyCommandIssue::InvalidMoneyAccount => {
            let account = optional_id_for_diagnostic(command.account.as_deref());
            VerificationError::error(
                "invalid_script_money_account",
                subject,
                format!("money command account id must be exact and nonempty, found {account}"),
            )
        }
        ScriptEconomyCommandIssue::UnknownMoneyAccount => {
            let account = optional_id_for_diagnostic(command.account.as_deref());
            VerificationError::error(
                "unknown_script_money_account",
                subject,
                format!("money command references unknown account {account}"),
            )
        }
        ScriptEconomyCommandIssue::UnexpectedCoinAccount => VerificationError::error(
            "unexpected_script_coin_account",
            subject,
            "coin command must not carry a money account id",
        ),
        ScriptEconomyCommandIssue::MissingMoneyCap => VerificationError::error(
            "missing_script_money_cap",
            subject,
            "money mutation requires MAX_MONEY in pack currency constants",
        ),
        ScriptEconomyCommandIssue::MissingCoinCap => VerificationError::error(
            "missing_script_coin_cap",
            subject,
            "coin mutation requires MAX_COINS in pack currency constants",
        ),
        ScriptEconomyCommandIssue::UnresolvedAmount { error } => VerificationError::error(
            "unresolved_script_currency_amount",
            subject,
            format!("currency amount does not resolve from pack constants: {error:?}"),
        ),
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
        let script_labels: BTreeSet<String> = module.scripts.keys().cloned().collect();
        verify_unique_script_command_positions(
            map_name,
            "gift_pokemon_scripts",
            module
                .gift_pokemon_scripts
                .iter()
                .map(|gift| (gift.source_script.as_str(), gift.command_index)),
            diagnostics,
        );
        for gift in &module.gift_pokemon_scripts {
            let subject = format!("{map_name}:{}:{}", gift.source_script, gift.command_index);
            for issue in
                gift_pokemon_script_issues(gift, &data.pokemon, &data.items, &script_labels)
            {
                diagnostics.push(gift_pokemon_script_issue_diagnostic(&subject, issue));
            }
        }
    }
}

fn gift_pokemon_script_issue_diagnostic(
    subject: &str,
    issue: GiftPokemonScriptIssue,
) -> VerificationError {
    match issue {
        GiftPokemonScriptIssue::InvalidSourceScript { source_script } => VerificationError::error(
            "invalid_gift_pokemon_source_script",
            subject,
            format!("gift source script must be exact pack syntax, found {source_script:?}"),
        ),
        GiftPokemonScriptIssue::InvalidSpeciesId { species_id } => VerificationError::error(
            "invalid_gift_pokemon_species",
            subject,
            format!("gift species id must be an exact pack token, found {species_id:?}"),
        ),
        GiftPokemonScriptIssue::UnknownSpecies { species_id } => VerificationError::error(
            "unknown_gift_pokemon_species",
            subject,
            format!("gift references missing species '{species_id}'"),
        ),
        GiftPokemonScriptIssue::InvalidHeldItemId { item_id } => VerificationError::error(
            "invalid_gift_pokemon_item",
            subject,
            format!("gift held item id must be an exact pack token, found {item_id:?}"),
        ),
        GiftPokemonScriptIssue::UnknownHeldItem { item_id } => VerificationError::error(
            "unknown_gift_pokemon_item",
            subject,
            format!("gift references missing held item '{item_id}'"),
        ),
        GiftPokemonScriptIssue::EmptyLabel { field } => VerificationError::error(
            "empty_gift_pokemon_label",
            subject,
            format!("gift {field} label must be non-empty"),
        ),
        GiftPokemonScriptIssue::InvalidLabel { field, label } => VerificationError::error(
            "invalid_gift_pokemon_label",
            subject,
            format!("gift {field} label '{label}' must be an exact non-empty value"),
        ),
        GiftPokemonScriptIssue::UnknownLabel { field, label } => VerificationError::error(
            "unknown_gift_pokemon_label",
            subject,
            format!("gift {field} label '{label}' is not loaded in map scripts"),
        ),
    }
}

fn verify_script_flag_commands(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    for (map_name, module) in &data.maps {
        verify_unique_script_command_positions(
            map_name,
            "script_flag_commands",
            module
                .script_flag_commands
                .iter()
                .map(|command| (command.source_script.as_str(), command.command_index)),
            diagnostics,
        );
        for command in &module.script_flag_commands {
            let subject = format!(
                "{map_name}:{}:{}",
                command.source_script, command.command_index
            );
            diagnostics.extend(
                script_flag_command_issues(command)
                    .into_iter()
                    .map(|issue| script_flag_command_issue_diagnostic(&subject, command, issue)),
            );
        }
    }
}

fn script_flag_command_issue_diagnostic(
    subject: &str,
    command: &ScriptFlagCommand,
    issue: ScriptFlagCommandIssue,
) -> VerificationError {
    match issue {
        ScriptFlagCommandIssue::InvalidCommand => VerificationError::error(
            "invalid_script_flag_command",
            subject,
            format!(
                "script flag command must be an exact lowercase pack token, found {:?}",
                command.command
            ),
        ),
        ScriptFlagCommandIssue::UnknownCommand => VerificationError::error(
            "unknown_script_flag_command",
            subject,
            format!("unknown script flag command '{}'", command.command),
        ),
        ScriptFlagCommandIssue::EmptyFlagId => VerificationError::error(
            "empty_script_flag_id",
            subject,
            "script flag command references an empty flag id",
        ),
        ScriptFlagCommandIssue::InvalidFlagId => VerificationError::error(
            "invalid_script_flag_id",
            subject,
            format!(
                "script flag command flag id must be exact and untrimmed, found {:?}",
                command.flag_id
            ),
        ),
        ScriptFlagCommandIssue::FlagKindMismatch => VerificationError::error(
            "script_flag_kind_mismatch",
            subject,
            format!(
                "script flag command '{}' cannot address flag '{}' from the other flag table",
                command.command, command.flag_id
            ),
        ),
    }
}

fn verify_script_scene_commands(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    for (map_name, module) in &data.maps {
        verify_scene_table_script_references(map_name, module, diagnostics);
        verify_unique_script_command_positions(
            map_name,
            "script_scene_commands",
            module
                .script_scene_commands
                .iter()
                .map(|command| (command.source_script.as_str(), command.command_index)),
            diagnostics,
        );
        for command in &module.script_scene_commands {
            let subject = format!(
                "{map_name}:{}:{}",
                command.source_script, command.command_index
            );
            diagnostics.extend(
                script_scene_command_issues(command)
                    .into_iter()
                    .map(|issue| script_scene_command_issue_diagnostic(&subject, command, issue)),
            );
            if SCRIPT_SCENE_CHECK_COMMANDS.contains(&command.command.as_str()) {
                // The ASM writes $ff when the current map has no scene-script table.
            } else if SCRIPT_SCENE_TARGET_MAP_CHECK_COMMANDS.contains(&command.command.as_str()) {
                let Some(map_id) = command.map_id.as_deref() else {
                    continue;
                };
                if !is_exact_script_scene_reference_token(map_id) {
                    continue;
                }
                let Some((_, _target_module)) = scene_table_for_map_id(data, map_id) else {
                    diagnostics.push(VerificationError::error(
                        "unknown_script_scene_map",
                        &subject,
                        format!("checkmapscene references missing map id '{map_id}'"),
                    ));
                    continue;
                };
            } else if SCRIPT_SCENE_CURRENT_MAP_MUTATION_COMMANDS.contains(&command.command.as_str())
            {
                if command
                    .scene_id
                    .as_deref()
                    .is_some_and(is_exact_script_scene_reference_token)
                {
                    verify_scene_token(
                        diagnostics,
                        &subject,
                        map_name,
                        command.scene_id.as_deref(),
                        &module.scenes,
                    );
                }
            } else if SCRIPT_SCENE_TARGET_MAP_MUTATION_COMMANDS.contains(&command.command.as_str())
            {
                let Some(map_id) = command.map_id.as_deref() else {
                    continue;
                };
                if !is_exact_script_scene_reference_token(map_id) {
                    continue;
                }
                let Some((target_map_name, target_module)) = scene_table_for_map_id(data, map_id)
                else {
                    diagnostics.push(VerificationError::error(
                        "unknown_script_scene_map",
                        &subject,
                        format!("setmapscene references missing map id '{map_id}'"),
                    ));
                    continue;
                };
                if command
                    .scene_id
                    .as_deref()
                    .is_some_and(is_exact_script_scene_reference_token)
                {
                    verify_scene_token(
                        diagnostics,
                        &subject,
                        &target_map_name,
                        command.scene_id.as_deref(),
                        &target_module.scenes,
                    );
                }
            }
        }
    }
}

fn verify_scene_table_script_references(
    map_name: &str,
    module: &MapModule,
    diagnostics: &mut Vec<VerificationError>,
) {
    let mut scene_ids = BTreeSet::new();
    for scene in &module.scenes.scenes {
        if !scene_ids.insert(scene.scene_id.as_str()) {
            diagnostics.push(VerificationError::error(
                "duplicate_scene_id",
                format!("{map_name}:{}", scene.scene_id),
                "map scene ids must be unique for runtime lookup",
            ));
        }
        let Some(script_name) = scene.script_name.as_deref() else {
            continue;
        };
        if !is_exact_script_label_reference_token(script_name) {
            diagnostics.push(VerificationError::error(
                "invalid_scene_script",
                format!("{map_name}:{}", scene.scene_id),
                format!("scene script label must be exact, found {script_name:?}"),
            ));
        } else if !module.scripts.contains_key(script_name) {
            diagnostics.push(VerificationError::error(
                "unknown_scene_script",
                format!("{map_name}:{}", scene.scene_id),
                format!("scene references missing exact script '{script_name}'"),
            ));
        }
    }
}

fn is_exact_script_scene_reference_token(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn script_scene_command_issue_diagnostic(
    subject: &str,
    command: &ScriptSceneCommand,
    issue: ScriptSceneCommandIssue,
) -> VerificationError {
    match issue {
        ScriptSceneCommandIssue::InvalidSourceScript => VerificationError::error(
            "invalid_script_scene_source_script",
            subject,
            "scene command source script must be exact pack syntax",
        ),
        ScriptSceneCommandIssue::InvalidCommand => VerificationError::error(
            "invalid_script_scene_command",
            subject,
            format!(
                "scene command must be an exact lowercase pack token, found {:?}",
                command.command
            ),
        ),
        ScriptSceneCommandIssue::UnknownCommand => VerificationError::error(
            "unknown_script_scene_command",
            subject,
            format!("unknown scene command '{}'", command.command),
        ),
        ScriptSceneCommandIssue::MissingTargetMap => VerificationError::error(
            "missing_script_scene_map",
            subject,
            format!("{} requires a target map id", command.command),
        ),
        ScriptSceneCommandIssue::InvalidTargetMap => VerificationError::error(
            "invalid_script_scene_map",
            subject,
            format!(
                "{} target map id must be exact and nonempty, found {}",
                command.command,
                optional_id_for_diagnostic(command.map_id.as_deref())
            ),
        ),
        ScriptSceneCommandIssue::UnexpectedTargetMap => VerificationError::error(
            "unexpected_script_scene_map",
            subject,
            format!("{} must not carry a target map id", command.command),
        ),
        ScriptSceneCommandIssue::MissingSceneId => VerificationError::error(
            "missing_script_scene_id",
            subject,
            format!("{} command is missing a scene id", command.command),
        ),
        ScriptSceneCommandIssue::InvalidSceneId => VerificationError::error(
            "invalid_script_scene_id",
            subject,
            format!(
                "{} scene id must be exact and nonempty, found {}",
                command.command,
                optional_id_for_diagnostic(command.scene_id.as_deref())
            ),
        ),
        ScriptSceneCommandIssue::UnexpectedSceneId => VerificationError::error(
            "unexpected_script_scene_id",
            subject,
            format!("{} must not carry a scene id", command.command),
        ),
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
        verify_unique_script_command_positions(
            map_name,
            "script_audio_commands",
            module
                .script_audio_commands
                .iter()
                .map(|command| (command.source_script.as_str(), command.command_index)),
            diagnostics,
        );
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
                    ScriptAudioCommandIssue::InvalidCommand => {
                        diagnostics.push(VerificationError::error(
                            "invalid_script_audio_command",
                            &subject,
                            format!(
                                "audio command must be an exact lowercase pack token, found {:?}",
                                command.command
                            ),
                        ));
                    }
                    ScriptAudioCommandIssue::MissingMusicId => {
                        diagnostics.push(VerificationError::error(
                            "missing_script_music_id",
                            &subject,
                            "audio command is missing a music id",
                        ))
                    }
                    ScriptAudioCommandIssue::InvalidMusicId => {
                        let audio_id = optional_id_for_diagnostic(command.audio_id.as_deref());
                        diagnostics.push(VerificationError::error(
                            "invalid_script_music_id",
                            &subject,
                            format!("music id must be an exact pack token, found {audio_id}"),
                        ));
                    }
                    ScriptAudioCommandIssue::UnknownMusicId => {
                        let audio_id = optional_id_for_diagnostic(command.audio_id.as_deref());
                        diagnostics.push(VerificationError::error(
                            "unknown_script_music_id",
                            &subject,
                            format!("audio command references missing music {audio_id}"),
                        ));
                    }
                    ScriptAudioCommandIssue::MissingSoundEffectId => {
                        diagnostics.push(VerificationError::error(
                            "missing_script_sfx_id",
                            &subject,
                            "playsound command is missing a sound effect id",
                        ))
                    }
                    ScriptAudioCommandIssue::InvalidSoundEffectId => {
                        let audio_id = optional_id_for_diagnostic(command.audio_id.as_deref());
                        diagnostics.push(VerificationError::error(
                            "invalid_script_sfx_id",
                            &subject,
                            format!(
                                "sound effect id must be an exact pack token, found {audio_id}"
                            ),
                        ));
                    }
                    ScriptAudioCommandIssue::UnknownSoundEffectId => {
                        let audio_id = optional_id_for_diagnostic(command.audio_id.as_deref());
                        diagnostics.push(VerificationError::error(
                            "unknown_script_sfx_id",
                            &subject,
                            format!("playsound command references missing sfx {audio_id}"),
                        ));
                    }
                    ScriptAudioCommandIssue::MissingCrySpecies => {
                        diagnostics.push(VerificationError::error(
                            "missing_script_cry_id",
                            &subject,
                            "cry command is missing a species id",
                        ));
                    }
                    ScriptAudioCommandIssue::InvalidCrySpecies => {
                        let species_id = optional_id_for_diagnostic(command.audio_id.as_deref());
                        diagnostics.push(VerificationError::error(
                            "invalid_script_cry_species",
                            &subject,
                            format!(
                                "cry species id must be an exact pack token, found {species_id}"
                            ),
                        ));
                    }
                    ScriptAudioCommandIssue::UnknownCrySpecies => {
                        let species_id = optional_id_for_diagnostic(command.audio_id.as_deref());
                        diagnostics.push(VerificationError::error(
                            "unknown_script_cry_species",
                            &subject,
                            format!("cry command references missing species {species_id}"),
                        ));
                    }
                    ScriptAudioCommandIssue::MissingCryMetadata => {
                        let species_id = optional_id_for_diagnostic(command.audio_id.as_deref());
                        diagnostics.push(VerificationError::error(
                            "missing_script_cry_metadata",
                            &subject,
                            format!(
                                "cry command references species {species_id} without cry metadata"
                            ),
                        ));
                    }
                    ScriptAudioCommandIssue::InvalidCryAsset => {
                        let species_id = optional_id_for_diagnostic(command.audio_id.as_deref());
                        let cry_id = command
                            .audio_id
                            .as_deref()
                            .and_then(|species_id| cry_by_species.get(species_id))
                            .map_or_else(
                                || "<missing>".to_string(),
                                |cry_id| format!("{cry_id:?}"),
                            );
                        diagnostics.push(VerificationError::error(
                            "invalid_script_cry_audio",
                            &subject,
                            format!(
                                "cry audio id must be an exact pack token, found {cry_id} for species {species_id}"
                            ),
                        ));
                    }
                    ScriptAudioCommandIssue::UnknownCryAsset => {
                        let species_id = optional_id_for_diagnostic(command.audio_id.as_deref());
                        let cry_id = command
                            .audio_id
                            .as_deref()
                            .and_then(|species_id| cry_by_species.get(species_id))
                            .map_or_else(
                                || "<missing>".to_string(),
                                |cry_id| format!("{cry_id:?}"),
                            );
                        diagnostics.push(VerificationError::error(
                            "unknown_script_cry_audio",
                            &subject,
                            format!(
                                "cry command references missing cry audio {cry_id} for species {species_id}"
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
        verify_unique_script_command_positions(
            map_name,
            "script_block_changes",
            module
                .script_block_changes
                .iter()
                .map(|change| (change.source_script.as_str(), change.command_index)),
            diagnostics,
        );
        diagnostics.extend(
            script_block_change_issues(
                &module.script_block_changes,
                module.attributes.width,
                module.attributes.height,
                module.blocks.len(),
            )
            .into_iter()
            .map(|issue| script_block_change_issue_diagnostic(map_name, issue)),
        );
    }
}

fn script_block_change_issue_diagnostic(
    map_name: &str,
    issue: ScriptBlockChangeIssue,
) -> VerificationError {
    match issue {
        ScriptBlockChangeIssue::InvalidSourceScript {
            source_script,
            command_index,
        } => VerificationError::error(
            "invalid_script_block_change_source_script",
            format!("{map_name}:{source_script}:{command_index}"),
            format!("changeblock source script must be exact pack syntax, found {source_script:?}"),
        ),
        ScriptBlockChangeIssue::UnalignedCoordinates {
            source_script,
            command_index,
            x,
            y,
        } => VerificationError::error(
            "script_block_change_unaligned",
            format!("{map_name}:{source_script}:{command_index}"),
            format!("changeblock coordinates ({x}, {y}) must be aligned to block coordinates"),
        ),
        ScriptBlockChangeIssue::OutOfBounds {
            source_script,
            command_index,
            x,
            y,
            width,
            height,
        } => VerificationError::error(
            "script_block_change_out_of_bounds",
            format!("{map_name}:{source_script}:{command_index}"),
            format!(
                "changeblock targets ({x}, {y}) outside {map_name} dimensions {width}x{height}"
            ),
        ),
        ScriptBlockChangeIssue::MapSizeMismatch {
            source_script,
            command_index,
            actual_blocks,
            expected_blocks,
        } => VerificationError::error(
            "script_block_map_size_mismatch",
            format!("{map_name}:{source_script}:{command_index}"),
            format!(
                "{map_name} has {actual_blocks} blocks but attributes require {expected_blocks}"
            ),
        ),
    }
}

fn verify_script_object_commands(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    for (map_name, module) in &data.maps {
        if module.objects.len() > 15 {
            diagnostics.push(VerificationError::error(
                "too_many_map_objects",
                map_name,
                format!(
                    "map declares {} non-player object records, but wMapObjects has only indexes 1..=15",
                    module.objects.len()
                ),
            ));
        }
        let mut object_identifiers = BTreeSet::new();
        for object in &module.objects {
            if object.x > u16::from(u8::MAX - 4) || object.y > u16::from(u8::MAX - 4) {
                let subject = format!(
                    "{map_name}:{}",
                    object
                        .object_identifier
                        .as_deref()
                        .unwrap_or("<unidentified>")
                );
                diagnostics.push(VerificationError::error(
                    "object_event_coordinate_storage_overflow",
                    &subject,
                    format!(
                        "object_event coordinate ({}, {}) cannot fit MAPOBJECT_X/Y after the ASM macro adds 4",
                        object.x, object.y
                    ),
                ));
            }
            if let Some(object_id) = &object.object_identifier {
                let subject = format!("{map_name}:{object_id}");
                if !is_exact_object_event_reference_token(object_id) {
                    diagnostics.push(VerificationError::error(
                        "invalid_object_identifier",
                        &subject,
                        format!(
                            "object identifier must be an exact pack token, found {object_id:?}"
                        ),
                    ));
                } else if !object_identifiers.insert(object_id.clone()) {
                    diagnostics.push(VerificationError::error(
                        "duplicate_object_identifier",
                        &subject,
                        format!("object identifier '{object_id}' is duplicated in map {map_name}"),
                    ));
                }
            }
            if object.script != "-1" && object.script != "ObjectEvent" {
                let subject = format!(
                    "{map_name}:{}",
                    object
                        .object_identifier
                        .as_deref()
                        .unwrap_or("<unidentified>")
                );
                if !is_exact_object_event_reference_token(&object.script) {
                    diagnostics.push(VerificationError::error(
                        "invalid_object_event_script",
                        &subject,
                        format!(
                            "object event script must be an exact pack token, found {:?}",
                            object.script
                        ),
                    ));
                } else if !module.scripts.contains_key(&object.script) {
                    diagnostics.push(VerificationError::error(
                        "unknown_object_event_script",
                        &subject,
                        format!(
                            "object event references missing exact script '{}'",
                            object.script
                        ),
                    ));
                }
            }
            if !is_exact_object_event_reference_token(&object.spritemovedata) {
                let subject = format!(
                    "{map_name}:{}",
                    object
                        .object_identifier
                        .as_deref()
                        .unwrap_or("<unidentified>")
                );
                diagnostics.push(VerificationError::error(
                    "invalid_object_movement_data",
                    &subject,
                    format!(
                        "object event spritemovedata must be an exact pack token, found {:?}",
                        object.spritemovedata
                    ),
                ));
            } else if object_event_initial_facing(&object.spritemovedata).is_none() {
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
            if object.move_range_x > 0xf || object.move_range_y > 0xf {
                let subject = format!(
                    "{map_name}:{}",
                    object
                        .object_identifier
                        .as_deref()
                        .unwrap_or("<unidentified>")
                );
                diagnostics.push(VerificationError::error(
                    "invalid_object_movement_radius",
                    &subject,
                    format!(
                        "object event movement radius ({}, {}) exceeds the two four-bit fields in MAPOBJECT_RADIUS",
                        object.move_range_x, object.move_range_y
                    ),
                ));
            }
            let valid_schedule = if object.hram_x == -1 {
                object.hram_y == -1 || (0..=0b111).contains(&object.hram_y)
            } else {
                (0..24).contains(&object.hram_x) && (0..24).contains(&object.hram_y)
            };
            if !valid_schedule {
                let subject = format!(
                    "{map_name}:{}",
                    object
                        .object_identifier
                        .as_deref()
                        .unwrap_or("<unidentified>")
                );
                diagnostics.push(VerificationError::error(
                    "invalid_object_schedule",
                    &subject,
                    format!(
                        "object event schedule ({}, {}) must be -1 plus a three-bit time-of-day mask/-1, or two hours in 0..24",
                        object.hram_x, object.hram_y
                    ),
                ));
            }
            if !is_exact_object_event_reference_token(&object.object_type) {
                let subject = format!(
                    "{map_name}:{}",
                    object
                        .object_identifier
                        .as_deref()
                        .unwrap_or("<unidentified>")
                );
                diagnostics.push(VerificationError::error(
                    "invalid_object_type",
                    &subject,
                    format!(
                        "object event object_type must be an exact pack token, found {:?}",
                        object.object_type
                    ),
                ));
            } else if !is_runtime_supported_object_type(&object.object_type) {
                let subject = format!(
                    "{map_name}:{}",
                    object
                        .object_identifier
                        .as_deref()
                        .unwrap_or("<unidentified>")
                );
                diagnostics.push(VerificationError::error(
                    "unsupported_object_type",
                    &subject,
                    format!(
                        "object event uses object_type '{}' that is not implemented by the Rust runtime",
                        object.object_type
                    ),
                ));
            }
        }
        let object_event_flags: BTreeMap<String, String> = module
            .objects
            .iter()
            .filter_map(|object| {
                object
                    .object_identifier
                    .as_ref()
                    .map(|object_id| (object_id.clone(), object.event_flag.clone()))
            })
            .collect();
        let hideable_event_flags: BTreeSet<String> = module
            .objects
            .iter()
            .filter(|object| is_hideable_object_event_flag(&object.event_flag))
            .map(|object| object.event_flag.clone())
            .collect();
        let movements: BTreeSet<(String, Option<String>)> = module
            .script_movements
            .iter()
            .map(|movement| (movement.label.clone(), movement.source_script.clone()))
            .collect();
        verify_unique_script_command_positions(
            map_name,
            "script_object_commands",
            module
                .script_object_commands
                .iter()
                .map(|command| (command.source_script.as_str(), command.command_index)),
            diagnostics,
        );
        for command in &module.script_object_commands {
            diagnostics.extend(
                script_object_command_issues(
                    command,
                    &object_event_flags,
                    &hideable_event_flags,
                    &movements,
                )
                .into_iter()
                .map(|issue| script_object_command_issue_diagnostic(map_name, issue)),
            );
            verify_script_moveobject_destination(map_name, module, command, diagnostics);
            verify_script_movement_runtime_match(map_name, module, command, diagnostics);
            verify_script_applymovement_endpoint(map_name, module, command, diagnostics);
        }
    }
}

fn verify_script_moveobject_destination(
    map_name: &str,
    module: &MapModule,
    command: &ScriptObjectCommand,
    diagnostics: &mut Vec<VerificationError>,
) {
    if !SCRIPT_OBJECT_COORDINATE_COMMANDS.contains(&command.command.as_str()) {
        return;
    }
    let (Some(x), Some(y)) = (command.x, command.y) else {
        return;
    };
    let Some(tile) = checked_runtime_map_event_tile(x, y) else {
        return;
    };
    let Some((width, height)) = runtime_map_tile_bounds(module) else {
        diagnostics.push(VerificationError::error(
            "script_moveobject_runtime_bounds_overflow",
            format!(
                "{map_name}:{}:{}",
                command.source_script, command.command_index
            ),
            "map runtime tile bounds overflow the supported coordinate range",
        ));
        return;
    };
    if tile.x < 0 || tile.y < 0 || tile.x >= width || tile.y >= height {
        diagnostics.push(VerificationError::error(
            "script_moveobject_destination_out_of_bounds",
            format!("{map_name}:{}:{}", command.source_script, command.command_index),
            format!(
                "moveobject raw coordinate ({x}, {y}) resolves to runtime tile ({}, {}) outside map bounds {}x{}",
                tile.x, tile.y, width, height
            ),
        ));
    }
}

fn is_runtime_supported_object_type(object_type: &str) -> bool {
    matches!(
        object_type,
        "OBJECTTYPE_SCRIPT" | "OBJECTTYPE_TRAINER" | "OBJECTTYPE_ITEMBALL"
    )
}

fn verify_script_movement_runtime_match(
    map_name: &str,
    module: &MapModule,
    command: &ScriptObjectCommand,
    diagnostics: &mut Vec<VerificationError>,
) {
    let Some(movement_label) = command.movement.as_deref() else {
        return;
    };
    if !is_exact_object_event_reference_token(movement_label) {
        return;
    }
    let matches = module
        .script_movements
        .iter()
        .filter(|movement| {
            movement.label == movement_label
                && movement.source_script.as_deref() == Some(command.source_script.as_str())
        })
        .count();
    if matches > 1 {
        diagnostics.push(VerificationError::error(
            "ambiguous_script_movement",
            format!("{map_name}:{}:{}", command.source_script, command.command_index),
            format!(
                "movement '{movement_label}' resolves to {matches} runtime matches for source script '{}'",
                command.source_script
            ),
        ));
        return;
    }
    if matches == 1 {
        return;
    }
}

fn verify_script_applymovement_endpoint(
    map_name: &str,
    module: &MapModule,
    command: &ScriptObjectCommand,
    diagnostics: &mut Vec<VerificationError>,
) {
    if !SCRIPT_OBJECT_DIRECT_MOVEMENT_COMMANDS.contains(&command.command.as_str()) {
        return;
    }
    let Some(object_id) = command.object_id.as_deref() else {
        return;
    };
    if object_id == "PLAYER" || object_id == "LAST_TALKED" {
        return;
    }
    let Some(movement_label) = command.movement.as_deref() else {
        return;
    };
    let mut matching_movements = module.script_movements.iter().filter(|movement| {
        movement.label == movement_label
            && movement.source_script.as_deref() == Some(command.source_script.as_str())
    });
    let Some(movement) = matching_movements.next() else {
        return;
    };
    if matching_movements.next().is_some() {
        return;
    }
    let Some(object) = module
        .objects
        .iter()
        .find(|object| object.object_identifier.as_deref() == Some(object_id))
    else {
        return;
    };
    let Some(mut tile) = checked_runtime_map_event_tile(object.x, object.y) else {
        return;
    };
    for step in &movement.steps {
        if script_movement_step_ends_sequence_for_verifier(&step.command) {
            break;
        }
        let Some(stride) = script_movement_step_stride_for_verifier(&step.command) else {
            continue;
        };
        let Some(direction) = step
            .direction
            .as_deref()
            .and_then(script_movement_direction_for_verifier)
        else {
            return;
        };
        let Some(next_tile) = checked_move_by_stride(tile, direction, stride) else {
            diagnostics.push(VerificationError::error(
                "script_applymovement_endpoint_overflow",
                format!(
                    "{map_name}:{}:{}",
                    command.source_script, command.command_index
                ),
                format!(
                    "applymovement for object '{object_id}' overflows supported runtime coordinates from ({}, {})",
                    tile.x, tile.y
                ),
            ));
            return;
        };
        tile = next_tile;
    }
    let _ = (map_name, module, object_id, tile);
}

fn script_movement_step_ends_sequence_for_verifier(command: &str) -> bool {
    matches!(command, "step_end" | "step_stop" | "step_loop")
}

fn script_movement_step_stride_for_verifier(command: &str) -> Option<i16> {
    script_movement_step_runtime_stride(command)
}

fn script_movement_direction_for_verifier(direction: &str) -> Option<Direction> {
    match direction {
        "DOWN" => Some(Direction::Down),
        "UP" => Some(Direction::Up),
        "LEFT" => Some(Direction::Left),
        "RIGHT" => Some(Direction::Right),
        _ => None,
    }
}

fn direction_script_token(direction: Direction) -> &'static str {
    match direction {
        Direction::Down => "DOWN",
        Direction::Up => "UP",
        Direction::Left => "LEFT",
        Direction::Right => "RIGHT",
    }
}

fn verify_unique_script_command_positions<'a, I>(
    map_name: &str,
    category: &str,
    positions: I,
    diagnostics: &mut Vec<VerificationError>,
) where
    I: IntoIterator<Item = (&'a str, usize)>,
{
    let mut seen = BTreeSet::new();
    for (source_script, command_index) in positions {
        if !seen.insert((source_script.to_string(), command_index)) {
            diagnostics.push(VerificationError::error(
                "duplicate_script_command_position",
                format!("{map_name}:{category}:{source_script}:{command_index}"),
                format!(
                    "{category} entries must be unique for each exact source script and command index"
                ),
            ));
        }
    }
}

fn is_exact_object_event_reference_token(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn script_object_command_issue_diagnostic(
    map_name: &str,
    issue: ScriptObjectCommandIssue,
) -> VerificationError {
    match issue {
        ScriptObjectCommandIssue::InvalidSourceScript {
            source_script,
            command_index,
        } => VerificationError::error(
            "invalid_script_object_source_script",
            format!("{map_name}:{source_script}:{command_index}"),
            format!(
                "object command source script must be exact pack syntax, found {source_script:?}"
            ),
        ),
        ScriptObjectCommandIssue::InvalidCommand {
            source_script,
            command_index,
            command,
        } => VerificationError::error(
            "invalid_script_object_command",
            format!("{map_name}:{source_script}:{command_index}"),
            format!(
                "script object command must be an exact lowercase pack token, found {command:?}"
            ),
        ),
        ScriptObjectCommandIssue::MissingObjectId {
            source_script,
            command_index,
            command,
        } => VerificationError::error(
            "script_object_missing_id",
            format!("{map_name}:{source_script}:{command_index}"),
            format!("{command} command is missing an object id"),
        ),
        ScriptObjectCommandIssue::UnknownObjectId {
            source_script,
            command_index,
            command,
            object_id,
        } => VerificationError::error(
            "unknown_script_object_id",
            format!("{map_name}:{source_script}:{command_index}"),
            format!("{command} references missing object id '{object_id}'"),
        ),
        ScriptObjectCommandIssue::InvalidObjectId {
            source_script,
            command_index,
            command,
            object_id,
        } => VerificationError::error(
            "invalid_script_object_id",
            format!("{map_name}:{source_script}:{command_index}"),
            format!("{command} object id must be an exact pack token, found {object_id:?}"),
        ),
        ScriptObjectCommandIssue::UnhideableObject {
            source_script,
            command_index,
            command,
            object_id,
            event_flag,
        } => VerificationError::error(
            "script_object_unhideable",
            format!("{map_name}:{source_script}:{command_index}"),
            format!(
                "{command} references object '{object_id}' with unhideable event flag '{event_flag}'"
            ),
        ),
        ScriptObjectCommandIssue::MissingCoordinates {
            source_script,
            command_index,
        } => VerificationError::error(
            "script_object_missing_coordinates",
            format!("{map_name}:{source_script}:{command_index}"),
            "moveobject command is missing x/y coordinates",
        ),
        ScriptObjectCommandIssue::MoveCoordinatesOutOfRange {
            source_script,
            command_index,
            x,
            y,
        } => VerificationError::error(
            "script_object_move_coordinates_out_of_range",
            format!("{map_name}:{source_script}:{command_index}"),
            format!("moveobject command has out-of-range raw event coordinate ({x}, {y})"),
        ),
        ScriptObjectCommandIssue::MissingDirection {
            source_script,
            command_index,
        } => VerificationError::error(
            "missing_script_direction",
            format!("{map_name}:{source_script}:{command_index}"),
            "movement command is missing a direction",
        ),
        ScriptObjectCommandIssue::UnknownDirection {
            source_script,
            command_index,
            direction,
        } => VerificationError::error(
            "unknown_script_direction",
            format!("{map_name}:{source_script}:{command_index}"),
            format!("unknown script direction '{direction}'"),
        ),
        ScriptObjectCommandIssue::MissingTargetObjectId {
            source_script,
            command_index,
            command,
        } => VerificationError::error(
            "script_object_missing_target_id",
            format!("{map_name}:{source_script}:{command_index}"),
            format!("{command} command is missing a target object id"),
        ),
        ScriptObjectCommandIssue::UnknownTargetObjectId {
            source_script,
            command_index,
            command,
            object_id,
        } => VerificationError::error(
            "unknown_script_object_id",
            format!("{map_name}:{source_script}:{command_index}"),
            format!("{command} references missing target object id '{object_id}'"),
        ),
        ScriptObjectCommandIssue::InvalidTargetObjectId {
            source_script,
            command_index,
            command,
            object_id,
        } => VerificationError::error(
            "invalid_script_target_object_id",
            format!("{map_name}:{source_script}:{command_index}"),
            format!("{command} target object id must be an exact pack token, found {object_id:?}"),
        ),
        ScriptObjectCommandIssue::MissingMovement {
            source_script,
            command_index,
            command,
        } => VerificationError::error(
            "script_object_missing_movement",
            format!("{map_name}:{source_script}:{command_index}"),
            format!("{command} command is missing a movement label"),
        ),
        ScriptObjectCommandIssue::UnknownMovement {
            source_script,
            command_index,
            command,
            movement,
        } => VerificationError::error(
            "unknown_script_movement",
            format!("{map_name}:{source_script}:{command_index}"),
            format!("{command} references missing movement '{movement}'"),
        ),
        ScriptObjectCommandIssue::InvalidMovement {
            source_script,
            command_index,
            command,
            movement,
        } => VerificationError::error(
            "invalid_script_movement",
            format!("{map_name}:{source_script}:{command_index}"),
            format!("{command} movement label must be an exact pack token, found {movement:?}"),
        ),
        ScriptObjectCommandIssue::MissingEmote {
            source_script,
            command_index,
        } => VerificationError::error(
            "script_object_missing_emote",
            format!("{map_name}:{source_script}:{command_index}"),
            "showemote command is missing emote/duration fields",
        ),
        ScriptObjectCommandIssue::EmoteDurationOutOfByteRange {
            source_script,
            command_index,
            duration,
        } => VerificationError::error(
            "script_emote_duration_out_of_byte_range",
            format!("{map_name}:{source_script}:{command_index}"),
            format!("showemote duration {duration} does not fit a byte"),
        ),
        ScriptObjectCommandIssue::UnknownCommand {
            source_script,
            command_index,
            command,
        } => VerificationError::error(
            "unknown_script_object_command",
            format!("{map_name}:{source_script}:{command_index}"),
            format!("unknown object command '{command}'"),
        ),
    }
}

fn verify_script_movements(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    for (map_name, module) in &data.maps {
        let mut seen_movements = BTreeSet::new();
        for movement in &module.script_movements {
            let movement_key = (movement.label.clone(), movement.source_script.clone());
            if !seen_movements.insert(movement_key) {
                let subject = match movement.source_script.as_deref() {
                    Some(source_script) => {
                        format!("{map_name}:{}:{source_script}", movement.label)
                    }
                    None => format!("{map_name}:{}", movement.label),
                };
                diagnostics.push(VerificationError::error(
                    "duplicate_script_movement",
                    subject,
                    "script movement labels must be unique for their exact source script",
                ));
            }
            if !script_movement_has_terminator(movement) {
                let subject = match movement.source_script.as_deref() {
                    Some(source_script) => {
                        format!("{map_name}:{}:{source_script}", movement.label)
                    }
                    None => format!("{map_name}:{}", movement.label),
                };
                diagnostics.push(VerificationError::error(
                    "unterminated_script_movement",
                    subject,
                    "script movement must end with a terminating opcode",
                ));
            }
            for step in &movement.steps {
                let subject = format!("{map_name}:{}:{}", movement.label, step.index);
                diagnostics.extend(
                    script_movement_step_issues(step)
                        .into_iter()
                        .map(|issue| script_movement_step_issue_diagnostic(&subject, step, issue)),
                );
            }
        }
    }
}

fn script_movement_has_terminator(movement: &ScriptMovement) -> bool {
    movement
        .steps
        .last()
        .is_some_and(|step| is_script_movement_terminator(&step.command))
}

fn script_movement_step_issue_diagnostic(
    subject: &str,
    step: &ScriptMovementStep,
    issue: ScriptMovementStepIssue,
) -> VerificationError {
    match issue {
        ScriptMovementStepIssue::UnexpectedDirection => VerificationError::error(
            "script_movement_unexpected_direction",
            subject,
            format!("{} must not carry a direction", step.command),
        ),
        ScriptMovementStepIssue::UnexpectedDuration => VerificationError::error(
            "script_movement_unexpected_duration",
            subject,
            format!("{} must not carry a duration", step.command),
        ),
        ScriptMovementStepIssue::MissingDirection => VerificationError::error(
            "missing_script_direction",
            subject,
            "script object command is missing a direction",
        ),
        ScriptMovementStepIssue::MissingDuration => VerificationError::error(
            "missing_script_movement_duration",
            subject,
            "script movement command is missing a duration",
        ),
        ScriptMovementStepIssue::DurationOutOfByteRange { duration } => VerificationError::error(
            "script_movement_duration_out_of_byte_range",
            subject,
            format!("script movement duration {duration} does not fit a byte"),
        ),
        ScriptMovementStepIssue::ZeroSleepDuration => VerificationError::error(
            "script_movement_zero_sleep_duration",
            subject,
            "step_sleep 0 cannot be encoded by the source macro",
        ),
        ScriptMovementStepIssue::UnknownDirection { direction } => VerificationError::error(
            "unknown_script_direction",
            subject,
            format!("unknown script direction '{direction}'"),
        ),
        ScriptMovementStepIssue::UnsupportedCommand => VerificationError::error(
            "unsupported_script_movement_command",
            subject,
            format!("unsupported movement command '{}'", step.command),
        ),
    }
}

fn verify_script_map_commands(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    let script_warp_targets = script_warp_target_constants(data);
    let mut playability_cache = BTreeMap::new();
    for (map_name, module) in &data.maps {
        verify_unique_script_command_positions(
            map_name,
            "script_map_commands",
            module
                .script_map_commands
                .iter()
                .map(|command| (command.source_script.as_str(), command.command_index)),
            diagnostics,
        );
        for command in &module.script_map_commands {
            let subject = format!(
                "{map_name}:{}:{}",
                command.source_script, command.command_index
            );
            diagnostics.extend(
                script_map_command_issues(command, &script_warp_targets)
                    .into_iter()
                    .map(|issue| script_map_command_issue_diagnostic(&subject, command, issue)),
            );
            verify_script_warp_command_destination(
                data,
                &mut playability_cache,
                command,
                &subject,
                diagnostics,
            );
        }
    }
}

fn verify_script_warp_command_destination(
    data: &GameDataSet,
    playability_cache: &mut BTreeMap<String, Option<Rc<MapPlayabilityContext>>>,
    command: &ScriptMapCommand,
    subject: &str,
    diagnostics: &mut Vec<VerificationError>,
) {
    if !SCRIPT_MAP_WARP_COMMANDS.contains(&command.command.as_str())
        && !SCRIPT_MAP_FACING_WARP_COMMANDS.contains(&command.command.as_str())
    {
        return;
    }
    let Some(target_map) = command.target_map.as_deref() else {
        return;
    };
    if target_map == "NONE" {
        return;
    }
    if !data.maps.contains_key(target_map) {
        return;
    }
    let (Some(x), Some(y)) = (command.x, command.y) else {
        return;
    };
    let Some(tile) = checked_runtime_map_event_tile(x, y) else {
        return;
    };
    let Some(context) = cached_map_playability_context_for_map(
        playability_cache,
        data,
        target_map,
        &PlayabilityRules::default(),
        diagnostics,
    ) else {
        return;
    };
    let Some((width, height)) = context.map.checked_tile_bounds() else {
        diagnostics.push(VerificationError::error(
            "script_warp_runtime_bounds_overflow",
            subject.to_string(),
            format!(
                "{} command target map {target_map} runtime tile bounds overflow supported coordinate range",
                command.command
            ),
        ));
        return;
    };
    if tile.x < 0
        || tile.y < 0
        || i32::from(tile.x) >= i32::from(width)
        || i32::from(tile.y) >= i32::from(height)
    {
        diagnostics.push(VerificationError::error(
            "script_warp_destination_out_of_bounds",
            subject.to_string(),
            format!(
                "{} command target {target_map} raw coordinate ({x}, {y}) resolves to runtime tile ({}, {}) outside map bounds {}x{}",
                command.command, tile.x, tile.y, width, height
            ),
        ));
        return;
    }
    if context.component_at(tile).is_none() {
        diagnostics.push(VerificationError::error(
            "script_warp_destination_not_walkable",
            subject.to_string(),
            format!(
                "{} command target {target_map} runtime tile ({}, {}) is not walkable",
                command.command, tile.x, tile.y
            ),
        ));
    }
}

fn script_map_command_issue_diagnostic(
    subject: &str,
    command: &ScriptMapCommand,
    issue: ScriptMapCommandError,
) -> VerificationError {
    match issue {
        ScriptMapCommandError::InvalidSourceScript { source_script } => VerificationError::error(
            "invalid_script_map_source_script",
            subject,
            format!("script map source script must be exact pack syntax, found {source_script:?}"),
        ),
        ScriptMapCommandError::InvalidCommand { .. } => VerificationError::error(
            "invalid_script_map_command",
            subject,
            format!(
                "script map command must be an exact lowercase pack token, found {:?}",
                command.command
            ),
        ),
        ScriptMapCommandError::UnknownCommand { .. } => VerificationError::error(
            "unknown_script_map_command",
            subject,
            format!("unknown script map command '{}'", command.command),
        ),
        ScriptMapCommandError::MissingTargetMap { .. } => VerificationError::error(
            "missing_script_warp_map",
            subject,
            format!("{} command is missing a target map", command.command),
        ),
        ScriptMapCommandError::InvalidTargetMap { target_map, .. } => VerificationError::error(
            "invalid_script_warp_map",
            subject,
            format!("{} command has invalid map '{target_map}'", command.command),
        ),
        ScriptMapCommandError::UnknownTargetMap { target_map, .. } => VerificationError::error(
            "unknown_script_warp_map",
            subject,
            format!(
                "{} command references missing map '{target_map}'",
                command.command
            ),
        ),
        ScriptMapCommandError::MalformedBadWarpSentinel { .. } => VerificationError::error(
            "malformed_script_bad_warp_sentinel",
            subject,
            "NONE is only valid for the exact warp NONE, 0, 0 sentinel",
        ),
        ScriptMapCommandError::MissingCoordinates { .. } => VerificationError::error(
            "missing_script_warp_coordinates",
            subject,
            format!("{} command is missing x/y coordinates", command.command),
        ),
        ScriptMapCommandError::CoordinatesOutOfRange { .. } => VerificationError::error(
            "script_warp_coordinates_out_of_range",
            subject,
            format!(
                "{} command has coordinates outside runtime tile range",
                command.command
            ),
        ),
        ScriptMapCommandError::UnexpectedWarpDestination { .. } => VerificationError::error(
            "unexpected_script_warp_destination",
            subject,
            format!(
                "{} command must not carry target map or coordinates",
                command.command
            ),
        ),
        ScriptMapCommandError::MissingFacing { .. } => VerificationError::error(
            "missing_script_warp_facing",
            subject,
            "warpfacing command is missing a facing direction",
        ),
        ScriptMapCommandError::UnexpectedFacing { .. } => VerificationError::error(
            "unexpected_script_warp_facing",
            subject,
            format!(
                "{} command must not carry a facing direction",
                command.command
            ),
        ),
        ScriptMapCommandError::InvalidFacing { facing } => VerificationError::error(
            "invalid_script_warp_facing",
            subject,
            format!("warpfacing has invalid direction '{facing}'"),
        ),
        ScriptMapCommandError::UnknownFacing { facing } => VerificationError::error(
            "unknown_script_warp_facing",
            subject,
            format!("warpfacing references unknown direction '{facing}'"),
        ),
        ScriptMapCommandError::MissingMapSetup { .. } => VerificationError::error(
            "missing_script_map_setup",
            subject,
            "newloadmap command is missing a map setup",
        ),
        ScriptMapCommandError::InvalidMapSetup { map_setup, .. } => VerificationError::error(
            "invalid_script_map_setup",
            subject,
            format!(
                "{} command has invalid map setup '{map_setup}'",
                command.command
            ),
        ),
        ScriptMapCommandError::UnexpectedMapSetup { .. } => VerificationError::error(
            "unexpected_script_map_setup",
            subject,
            format!("{} command must not carry a map setup", command.command),
        ),
        ScriptMapCommandError::MissingPendingScriptWarp => VerificationError::error(
            "missing_pending_script_warp",
            subject,
            format!(
                "{} command requires a pending script warp request",
                command.command
            ),
        ),
        ScriptMapCommandError::PendingScriptWarpMismatch => VerificationError::error(
            "pending_script_warp_mismatch",
            subject,
            format!(
                "{} command does not match the pending script warp request",
                command.command
            ),
        ),
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

fn script_text_wait_closes_window(command: &str) -> bool {
    matches!(command, "jumptext" | "jumptextfaceplayer" | "farjumptext")
}

fn verify_script_text_commands(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    let mut global_text_labels = data.asm_text.keys().cloned().collect::<BTreeSet<_>>();
    if let Some(global) = &data.global_scripts {
        global_text_labels.extend(global.script_text_bodies.keys().cloned());
    }
    for (map_name, module) in &data.maps {
        let mut text_labels: BTreeSet<String> = module
            .scripts
            .iter()
            .filter_map(|(label, payload)| is_text_script(payload).then_some(label.clone()))
            .collect();
        text_labels.extend(global_text_labels.iter().cloned());
        verify_unique_script_command_positions(
            map_name,
            "script_text_commands",
            module
                .script_text_commands
                .iter()
                .map(|command| (command.source_script.as_str(), command.command_index)),
            diagnostics,
        );
        for command in &module.script_text_commands {
            let subject = format!(
                "{map_name}:{}:{}",
                command.source_script, command.command_index
            );
            diagnostics.extend(
                script_text_command_issues(command, &text_labels)
                    .into_iter()
                    .map(|issue| script_text_command_issue_diagnostic(&subject, command, issue)),
            );
        }
    }
    if let Some(module) = &data.global_scripts {
        verify_unique_script_command_positions(
            "GlobalScripts",
            "script_text_commands",
            module
                .script_text_commands
                .iter()
                .map(|command| (command.source_script.as_str(), command.command_index)),
            diagnostics,
        );
        for command in &module.script_text_commands {
            let subject = format!(
                "GlobalScripts:{}:{}",
                command.source_script, command.command_index
            );
            diagnostics.extend(
                script_text_command_issues(command, &global_text_labels)
                    .into_iter()
                    .map(|issue| script_text_command_issue_diagnostic(&subject, command, issue)),
            );
        }
    }
}

fn script_text_command_issue_diagnostic(
    subject: &str,
    command: &ScriptTextCommand,
    issue: ScriptTextCommandError,
) -> VerificationError {
    match issue {
        ScriptTextCommandError::InvalidSourceScript { source_script } => VerificationError::error(
            "invalid_script_text_source_script",
            subject,
            format!("script text source script must be exact pack syntax, found {source_script:?}"),
        ),
        ScriptTextCommandError::InvalidCommand { .. } => VerificationError::error(
            "invalid_script_text_command",
            subject,
            format!(
                "script text command must be an exact lowercase pack token, found {:?}",
                command.command
            ),
        ),
        ScriptTextCommandError::UnknownCommand { .. } => VerificationError::error(
            "unknown_script_text_command",
            subject,
            format!("unknown text command '{}'", command.command),
        ),
        ScriptTextCommandError::MissingTextLabel { .. } => VerificationError::error(
            "missing_script_text_label",
            subject,
            format!("{} command is missing a text label", command.command),
        ),
        ScriptTextCommandError::InvalidTextLabel { text_label, .. } => VerificationError::error(
            "invalid_script_text_label",
            subject,
            format!(
                "{} command has invalid text label '{text_label}'",
                command.command
            ),
        ),
        ScriptTextCommandError::UnknownTextLabel { .. } => {
            let label = optional_id_for_diagnostic(command.text_label.as_deref());
            VerificationError::error(
                "unknown_script_text_label",
                subject,
                format!(
                    "{} command references missing text label {label}",
                    command.command
                ),
            )
        }
        ScriptTextCommandError::UnexpectedTextLabel { .. } => VerificationError::error(
            "unexpected_script_text_label",
            subject,
            format!("{} command must not carry a text label", command.command),
        ),
    }
}

fn verify_script_text_bodies(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    for (map_name, module) in &data.maps {
        for (label, body) in &module.script_text_bodies {
            diagnostics.extend(
                script_text_body_issues(label, body)
                    .into_iter()
                    .map(|issue| script_text_body_issue_diagnostic(map_name, label, issue)),
            );
        }
    }
}

fn verify_script_menu_definitions(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    for (map_name, module) in &data.maps {
        for (label, menu) in &module.script_menu_definitions {
            diagnostics.extend(
                script_menu_definition_issues(label, menu)
                    .into_iter()
                    .map(|issue| script_menu_definition_issue_diagnostic(map_name, label, issue)),
            );
            for command in menu
                .commands
                .iter()
                .filter(|command| command.command == "menu_coords")
            {
                if let Err(error) = validate_menu_coord_args(&command.args) {
                    diagnostics.push(VerificationError::error(
                        "invalid_script_menu_coordinates",
                        format!("{map_name}:{label}:{}", command.command_index),
                        format!("invalid menu_coords command: {error:#}"),
                    ));
                }
            }
        }
        for (label, menu) in &module.script_vertical_menus {
            if menu.options.is_empty() {
                diagnostics.push(VerificationError::error(
                    "empty_script_vertical_menu",
                    format!("{map_name}:{label}"),
                    "vertical menu must declare at least one option",
                ));
            }
            if !module
                .script_menu_definitions
                .contains_key(&menu.header_label)
            {
                diagnostics.push(VerificationError::error(
                    "unknown_script_vertical_menu_header",
                    format!("{map_name}:{label}"),
                    format!(
                        "vertical menu references missing header '{}'",
                        menu.header_label
                    ),
                ));
            }
            if let Some(data_label) = &menu.data_label {
                if !module.script_menu_definitions.contains_key(data_label) {
                    diagnostics.push(VerificationError::error(
                        "unknown_script_vertical_menu_data",
                        format!("{map_name}:{label}"),
                        format!("vertical menu references missing data label '{data_label}'"),
                    ));
                }
            }
        }
        for (label, elevator) in &module.script_elevators {
            if elevator.floors.is_empty() {
                diagnostics.push(VerificationError::error(
                    "empty_script_elevator",
                    format!("{map_name}:{label}"),
                    "elevator must declare at least one floor",
                ));
            }
            if !module.scripts.contains_key(&elevator.data_label) {
                diagnostics.push(VerificationError::error(
                    "unknown_script_elevator_data",
                    format!("{map_name}:{label}"),
                    format!(
                        "elevator references missing data label '{}'",
                        elevator.data_label
                    ),
                ));
            }
            for (index, floor) in elevator.floors.iter().enumerate() {
                if !data.saved_warp_exists(&floor.target_map, floor.warp) {
                    diagnostics.push(VerificationError::error(
                        "unknown_script_elevator_warp",
                        format!("{map_name}:{label}:{index}"),
                        format!(
                            "elevator floor '{}' references missing warp {} on {}",
                            floor.floor, floor.warp, floor.target_map
                        ),
                    ));
                }
            }
        }
    }
}

fn script_text_body_issue_diagnostic(
    map_name: &str,
    label: &str,
    issue: ScriptTextBodyIssue,
) -> VerificationError {
    match issue {
        ScriptTextBodyIssue::InvalidKey { key } => VerificationError::error(
            "invalid_script_text_body_key",
            format!("{map_name}:{key}"),
            format!("text body key '{key}' is not an exact nonempty label"),
        ),
        ScriptTextBodyIssue::InvalidLabel { label } => VerificationError::error(
            "invalid_script_text_body_label",
            format!("{map_name}:{label}"),
            format!("text body record label '{label}' is not an exact nonempty label"),
        ),
        ScriptTextBodyIssue::LabelMismatch { key, label } => VerificationError::error(
            "script_text_body_label_mismatch",
            format!("{map_name}:{key}"),
            format!("text body key '{key}' does not match record label '{label}'"),
        ),
        ScriptTextBodyIssue::UnknownCommand {
            command_index,
            command,
        } => VerificationError::error(
            "unknown_script_text_body_command",
            format!("{map_name}:{label}:{command_index}"),
            format!("unknown text body command '{command}'"),
        ),
        ScriptTextBodyIssue::MalformedCommand {
            command_index,
            command,
            expected,
            actual,
        } => VerificationError::error(
            "malformed_script_text_body_command",
            format!("{map_name}:{label}:{command_index}"),
            format!("{command} expects {expected} args but found {actual}"),
        ),
    }
}

fn script_menu_definition_issue_diagnostic(
    map_name: &str,
    label: &str,
    issue: ScriptMenuDefinitionIssue,
) -> VerificationError {
    match issue {
        ScriptMenuDefinitionIssue::InvalidKey { key } => VerificationError::error(
            "invalid_script_menu_key",
            format!("{map_name}:{key}"),
            format!("menu definition key '{key}' is not an exact nonempty label"),
        ),
        ScriptMenuDefinitionIssue::InvalidLabel { label } => VerificationError::error(
            "invalid_script_menu_label",
            format!("{map_name}:{label}"),
            format!("menu definition label '{label}' is not an exact nonempty label"),
        ),
        ScriptMenuDefinitionIssue::LabelMismatch { key, label } => VerificationError::error(
            "script_menu_label_mismatch",
            format!("{map_name}:{key}"),
            format!("menu definition key '{key}' does not match record label '{label}'"),
        ),
        ScriptMenuDefinitionIssue::UnknownCommand {
            command_index,
            command,
        } => VerificationError::error(
            "unknown_script_menu_command",
            format!("{map_name}:{label}:{command_index}"),
            format!("unknown menu definition command '{command}'"),
        ),
        ScriptMenuDefinitionIssue::MalformedCommand {
            command_index,
            command,
            expected,
            actual,
        } => VerificationError::error(
            "malformed_script_menu_command",
            format!("{map_name}:{label}:{command_index}"),
            format!("{command} expects one of {expected:?} args but found {actual}"),
        ),
    }
}

fn verify_script_variable_commands(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    for (map_name, module) in &data.maps {
        verify_unique_script_command_positions(
            map_name,
            "script_variable_commands",
            module
                .script_variable_commands
                .iter()
                .map(|command| (command.source_script.as_str(), command.command_index)),
            diagnostics,
        );
        diagnostics.extend(
            script_variable_command_issues(&module.script_variable_commands)
                .into_iter()
                .map(|issue| script_variable_command_issue_diagnostic(map_name, issue)),
        );
    }
}

fn script_variable_command_issue_diagnostic(
    map_name: &str,
    issue: ScriptVariableCommandIssue,
) -> VerificationError {
    VerificationError::error(
        "invalid_script_variable_command",
        format!("{map_name}:{}:{}", issue.source_script, issue.command_index),
        issue.error.to_string(),
    )
}

fn verify_script_control_commands(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    for (map_name, module) in &data.maps {
        let script_labels: BTreeSet<String> = module.scripts.keys().cloned().collect();
        let executable_positions = map_runtime_executable_script_command_positions(module);
        verify_unique_script_command_positions(
            map_name,
            "script_control_commands",
            module
                .script_control_commands
                .iter()
                .map(|command| (command.source_script.as_str(), command.command_index)),
            diagnostics,
        );
        for command in &module.script_control_commands {
            let subject = format!(
                "{map_name}:{}:{}",
                command.source_script, command.command_index
            );
            diagnostics.extend(
                script_control_command_issues(command, &script_labels)
                    .into_iter()
                    .map(|issue| script_control_command_issue_diagnostic(&subject, command, issue)),
            );
            if let Some((target_script, target_command)) =
                non_executable_control_target(&module.scripts, &executable_positions, command)
            {
                diagnostics.push(non_executable_script_control_target_diagnostic(
                    &subject,
                    command,
                    &target_script,
                    &target_command,
                ));
            }
        }
        verify_executable_script_bodies(
            map_name,
            &module.scripts,
            &executable_positions,
            diagnostics,
        );
    }
    if let Some(module) = data.global_scripts.as_ref() {
        let map_name = "GlobalScripts";
        let script_labels: BTreeSet<String> = module.scripts.keys().cloned().collect();
        let executable_positions = global_runtime_executable_script_command_positions(module);
        verify_unique_script_command_positions(
            map_name,
            "script_control_commands",
            module
                .script_control_commands
                .iter()
                .map(|command| (command.source_script.as_str(), command.command_index)),
            diagnostics,
        );
        for command in &module.script_control_commands {
            let subject = format!(
                "{map_name}:{}:{}",
                command.source_script, command.command_index
            );
            diagnostics.extend(
                script_control_command_issues(command, &script_labels)
                    .into_iter()
                    .map(|issue| script_control_command_issue_diagnostic(&subject, command, issue)),
            );
            if let Some((target_script, target_command)) =
                non_executable_control_target(&module.scripts, &executable_positions, command)
            {
                diagnostics.push(non_executable_script_control_target_diagnostic(
                    &subject,
                    command,
                    &target_script,
                    &target_command,
                ));
            }
        }
        verify_executable_script_bodies(
            map_name,
            &module.scripts,
            &executable_positions,
            diagnostics,
        );
    }
}

fn verify_executable_script_bodies(
    map_name: &str,
    scripts: &BTreeMap<String, Value>,
    executable_positions: &BTreeSet<(String, usize)>,
    diagnostics: &mut Vec<VerificationError>,
) {
    for (source_script, body) in scripts {
        if !executable_positions.contains(&(source_script.clone(), 0)) {
            continue;
        }
        let Some(commands) = body.as_array() else {
            continue;
        };
        for (command_index, command) in commands.iter().enumerate() {
            let command_name = command
                .get("command")
                .and_then(Value::as_str)
                .unwrap_or("<missing command>");
            if !executable_positions.contains(&(source_script.clone(), command_index)) {
                diagnostics.push(VerificationError::error(
                    "non_executable_script_command",
                    format!("{map_name}:{source_script}:{command_index}"),
                    format!(
                        "executable compiled script body reaches '{command_name}', which has no Rust runtime mutation"
                    ),
                ));
                break;
            }
            if script_command_ends_linear_execution(command_name) {
                break;
            }
        }
    }
}

fn script_command_ends_linear_execution(command: &str) -> bool {
    matches!(
        command,
        "end"
            | "endcallback"
            | "sjump"
            | "farsjump"
            | "jumpstd"
            | "jumptext"
            | "jumptextfaceplayer"
            | "trainer"
    )
}

fn map_runtime_executable_script_command_positions(
    module: &MapModule,
) -> BTreeSet<(String, usize)> {
    let mut positions = BTreeSet::new();
    macro_rules! insert_positions {
        ($commands:expr) => {
            positions.extend(
                $commands
                    .iter()
                    .map(|command| (command.source_script.clone(), command.command_index)),
            );
        };
    }
    insert_positions!(module.script_item_grants);
    insert_positions!(module.script_item_checks);
    insert_positions!(module.script_item_takes);
    insert_positions!(module.script_economy_commands);
    insert_positions!(module.gift_pokemon_scripts);
    insert_positions!(module.script_flag_commands);
    insert_positions!(module.script_scene_commands);
    insert_positions!(module.script_audio_commands);
    insert_positions!(module.script_block_changes);
    insert_positions!(module.script_object_commands);
    insert_positions!(module.script_map_commands);
    insert_positions!(module.script_text_commands);
    insert_positions!(module.script_variable_commands);
    insert_positions!(module.script_control_commands);
    insert_positions!(module.script_field_pickups);
    insert_positions!(module.script_shop_commands);
    insert_positions!(module.script_phone_commands);
    insert_positions!(module.script_swarm_commands);
    positions.extend(
        module
            .script_runtime_commands
            .iter()
            .filter(|command| command.command != "conditional_event")
            .map(|command| (command.source_script.clone(), command.command_index)),
    );
    positions.extend(module.scripted_wild_battles.iter().map(|battle| {
        (
            battle.source_script.clone(),
            battle.startbattle_command_index,
        )
    }));
    positions.extend(module.scripted_trainer_battles.iter().map(|battle| {
        (
            battle.source_script.clone(),
            battle.startbattle_command_index,
        )
    }));
    for source_script in module.trainer_scripts.keys() {
        if let Some(commands) = module.scripts.get(source_script).and_then(Value::as_array) {
            positions.extend(commands.iter().enumerate().filter_map(|(index, command)| {
                (command.get("command").and_then(Value::as_str) == Some("trainer"))
                    .then(|| (source_script.clone(), index))
            }));
        }
    }
    positions
}

fn global_runtime_executable_script_command_positions(
    module: &GlobalScriptModule,
) -> BTreeSet<(String, usize)> {
    let mut positions = BTreeSet::new();
    macro_rules! insert_positions {
        ($commands:expr) => {
            positions.extend(
                $commands
                    .iter()
                    .map(|command| (command.source_script.clone(), command.command_index)),
            );
        };
    }
    insert_positions!(module.script_item_grants);
    insert_positions!(module.script_item_checks);
    insert_positions!(module.script_item_takes);
    insert_positions!(module.script_economy_commands);
    insert_positions!(module.script_flag_commands);
    insert_positions!(module.script_scene_commands);
    insert_positions!(module.script_audio_commands);
    insert_positions!(module.script_block_changes);
    insert_positions!(module.script_object_commands);
    insert_positions!(module.script_map_commands);
    insert_positions!(module.script_text_commands);
    insert_positions!(module.script_variable_commands);
    insert_positions!(module.script_control_commands);
    insert_positions!(module.script_shop_commands);
    insert_positions!(module.script_phone_commands);
    insert_positions!(module.script_swarm_commands);
    positions.extend(
        module
            .script_runtime_commands
            .iter()
            .filter(|command| command.command != "conditional_event")
            .map(|command| (command.source_script.clone(), command.command_index)),
    );
    // These five source-certified global paths enter battle through typed
    // field/contest boundaries rather than a map-local scripted battle row.
    for (source_script, command_index) in [
        (".FightTheHookedPokemon@Script_GotABite", 6),
        (".SweetScent@SweetScentFromMenu", 10),
        ("BugCatchingContestBattleScript", 2),
        ("HeadbuttScript", 8),
        ("RockSmashScript", 12),
    ] {
        if module
            .scripts
            .get(source_script)
            .and_then(Value::as_array)
            .and_then(|commands| commands.get(command_index))
            .and_then(|command| command.get("command"))
            .and_then(Value::as_str)
            == Some("startbattle")
        {
            positions.insert((source_script.to_string(), command_index));
        }
    }
    positions
}

fn non_executable_control_target(
    scripts: &BTreeMap<String, Value>,
    executable_positions: &BTreeSet<(String, usize)>,
    command: &ScriptControlCommand,
) -> Option<(String, String)> {
    if matches!(command.command.as_str(), "jumpstd" | "callstd") {
        return None;
    }
    let target_script = command.resolved_target_script.as_ref()?;
    let target_body = scripts.get(target_script)?.as_array()?;
    let target_command = target_body
        .first()
        .and_then(|command| command.get("command"))
        .and_then(Value::as_str)
        .unwrap_or("<missing command>");
    (!executable_positions.contains(&(target_script.clone(), 0)))
        .then(|| (target_script.clone(), target_command.to_string()))
}

fn non_executable_script_control_target_diagnostic(
    subject: &str,
    command: &ScriptControlCommand,
    target_script: &str,
    target_command: &str,
) -> VerificationError {
    VerificationError::error(
        "non_executable_script_control_target",
        subject,
        format!(
            "{} resolves to '{target_script}', whose first command '{target_command}' has no Rust runtime mutation",
            command.command
        ),
    )
}

fn script_control_command_issue_diagnostic(
    subject: &str,
    command: &ScriptControlCommand,
    issue: ScriptControlCommandIssue,
) -> VerificationError {
    match issue {
        ScriptControlCommandIssue::InvalidCommand { error } => {
            VerificationError::error("invalid_script_control_command", subject, error.to_string())
        }
        ScriptControlCommandIssue::InvalidTargetScript { target_script } => {
            VerificationError::error(
                "invalid_script_control_target",
                subject,
                format!(
                    "{} command resolves to invalid script label '{target_script}'",
                    command.command
                ),
            )
        }
        ScriptControlCommandIssue::UnknownTargetScript { target_script } => {
            VerificationError::error(
                "unknown_script_control_target",
                subject,
                format!(
                    "{} command resolves to missing script label '{target_script}'",
                    command.command
                ),
            )
        }
    }
}

fn verify_script_field_pickups(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    for (map_name, module) in &data.maps {
        verify_map_event_script_references(map_name, module, diagnostics);
        verify_hidden_item_bg_events(map_name, module, diagnostics);
        verify_itemball_object_pickups(map_name, module, diagnostics);
        verify_unique_script_command_positions(
            map_name,
            "script_field_pickups",
            module
                .script_field_pickups
                .iter()
                .map(|pickup| (pickup.source_script.as_str(), pickup.command_index)),
            diagnostics,
        );
        for pickup in &module.script_field_pickups {
            let subject = format!(
                "{map_name}:{}:{}",
                pickup.source_script, pickup.command_index
            );
            diagnostics.extend(
                script_field_pickup_issues(pickup, &data.items, &data.fruit_trees)
                    .into_iter()
                    .map(|issue| script_field_pickup_issue_diagnostic(&subject, pickup, issue)),
            );
        }
    }
}

fn verify_itemball_object_pickups(
    map_name: &str,
    module: &MapModule,
    diagnostics: &mut Vec<VerificationError>,
) {
    let itemball_objects: BTreeMap<&str, Vec<&ObjectEvent>> = module
        .objects
        .iter()
        .filter(|object| object.object_type == "OBJECTTYPE_ITEMBALL")
        .fold(BTreeMap::new(), |mut objects_by_script, object| {
            objects_by_script
                .entry(object.script.as_str())
                .or_default()
                .push(object);
            objects_by_script
        });
    let itemball_pickups: BTreeMap<&str, Vec<&ScriptFieldPickup>> = module
        .script_field_pickups
        .iter()
        .filter(|pickup| pickup.command == "itemball")
        .fold(BTreeMap::new(), |mut pickups_by_script, pickup| {
            pickups_by_script
                .entry(pickup.source_script.as_str())
                .or_default()
                .push(pickup);
            pickups_by_script
        });

    for (script, objects) in &itemball_objects {
        if objects.len() > 1 {
            diagnostics.push(VerificationError::error(
                "itemball_duplicate_object_script",
                format!("{map_name}:{script}"),
                format!(
                    "item ball object script '{script}' resolves to {} OBJECTTYPE_ITEMBALL objects",
                    objects.len()
                ),
            ));
        }
        let pickup_count = itemball_pickups.get(script).map(Vec::len).unwrap_or(0);
        if pickup_count == 0 {
            for object in objects {
                diagnostics.push(VerificationError::error(
                    "itemball_object_missing_pickup",
                    format!(
                        "{map_name}:{}",
                        object
                            .object_identifier
                            .as_deref()
                            .unwrap_or(object.script.as_str())
                    ),
                    format!(
                        "item ball object script '{}' has no exact itemball field pickup",
                        object.script
                    ),
                ));
            }
        } else if pickup_count > 1 {
            diagnostics.push(VerificationError::error(
                "itemball_object_duplicate_pickup",
                format!("{map_name}:{script}"),
                format!(
                    "item ball object script '{script}' resolves to {pickup_count} itemball field pickups"
                ),
            ));
        }
    }

    for (script, pickups) in &itemball_pickups {
        if !itemball_objects.contains_key(script) {
            for pickup in pickups {
                diagnostics.push(VerificationError::error(
                    "itemball_pickup_missing_object",
                    format!(
                        "{map_name}:script_field_pickup:{}:{}",
                        pickup.source_script, pickup.command_index
                    ),
                    format!(
                        "itemball field pickup '{}' has no exact OBJECTTYPE_ITEMBALL object",
                        pickup.source_script
                    ),
                ));
            }
        }
    }
}

fn verify_map_event_script_references(
    map_name: &str,
    module: &MapModule,
    diagnostics: &mut Vec<VerificationError>,
) {
    verify_map_event_runtime_positions(map_name, module, diagnostics);
    for event in &module.events.coord_events {
        if !is_exact_script_label_reference_token(&event.script_name) {
            diagnostics.push(VerificationError::error(
                "invalid_coord_event_script",
                format!(
                    "{map_name}:{}:{}:{},{}",
                    event.scene_id, event.script_name, event.x, event.y
                ),
                format!(
                    "coord event script label must be exact, found {:?}",
                    event.script_name
                ),
            ));
        } else if !module.scripts.contains_key(&event.script_name) {
            diagnostics.push(VerificationError::error(
                "unknown_coord_event_script",
                format!(
                    "{map_name}:{}:{}:{},{}",
                    event.scene_id, event.script_name, event.x, event.y
                ),
                format!(
                    "coord event references missing exact script '{}'",
                    event.script_name
                ),
            ));
        }
    }
    for event in &module.events.bg_events {
        if !is_exact_script_label_reference_token(&event.script) {
            diagnostics.push(VerificationError::error(
                "invalid_bg_event_script",
                format!(
                    "{map_name}:{}:{}:{},{}",
                    event.event_type, event.script, event.x, event.y
                ),
                format!(
                    "{} background event script label must be exact, found {:?}",
                    event.event_type, event.script
                ),
            ));
        } else if !module.scripts.contains_key(&event.script) {
            diagnostics.push(VerificationError::error(
                "unknown_bg_event_script",
                format!(
                    "{map_name}:{}:{}:{},{}",
                    event.event_type, event.script, event.x, event.y
                ),
                format!(
                    "{} background event references missing exact script '{}'",
                    event.event_type, event.script
                ),
            ));
        }
    }
}

fn verify_map_event_runtime_positions(
    map_name: &str,
    module: &MapModule,
    diagnostics: &mut Vec<VerificationError>,
) {
    let Some(bounds) = runtime_map_tile_bounds(module) else {
        diagnostics.push(VerificationError::error(
            "map_event_runtime_bounds_overflow",
            map_name.to_string(),
            "map runtime tile bounds overflow the supported coordinate range",
        ));
        return;
    };
    for warp in &module.events.warps {
        verify_raw_event_runtime_position(
            map_name,
            "warp_event",
            &format!("warp:{}:{},{}", warp.index, warp.x, warp.y),
            warp.x,
            warp.y,
            bounds,
            diagnostics,
        );
    }
    for event in &module.events.coord_events {
        verify_raw_event_runtime_position(
            map_name,
            "coord_event",
            &format!(
                "coord:{}:{}:{},{}",
                event.scene_id, event.script_name, event.x, event.y
            ),
            event.x,
            event.y,
            bounds,
            diagnostics,
        );
    }
    for event in &module.events.bg_events {
        verify_raw_event_runtime_position(
            map_name,
            "bg_event",
            &format!(
                "bg:{}:{}:{},{}",
                event.event_type, event.script, event.x, event.y
            ),
            event.x,
            event.y,
            bounds,
            diagnostics,
        );
    }
    let coord_events = &module.events.coord_events;
    for (index, event) in coord_events.iter().enumerate() {
        for other in coord_events.iter().skip(index + 1) {
            let Some(event_tile) = checked_runtime_map_event_tile(event.x, event.y) else {
                continue;
            };
            let Some(other_tile) = checked_runtime_map_event_tile(other.x, other.y) else {
                continue;
            };
            if event_tile == other_tile
                && coord_event_scenes_overlap(&event.scene_id, &other.scene_id)
            {
                diagnostics.push(VerificationError::error(
                    "duplicate_coord_event_position",
                    format!("{map_name}:{},{}", event_tile.x, event_tile.y),
                    format!(
                        "coord events for scenes {:?} and {:?} resolve to the same runtime tile",
                        event.scene_id, other.scene_id
                    ),
                ));
            }
        }
    }

    let mut bg_event_tiles = BTreeSet::new();
    for event in &module.events.bg_events {
        let Some(event_tile) = checked_runtime_map_event_tile(event.x, event.y) else {
            continue;
        };
        if !bg_event_tiles.insert((event_tile.x, event_tile.y)) {
            diagnostics.push(VerificationError::error(
                "duplicate_bg_event_position",
                format!("{map_name}:{},{}", event_tile.x, event_tile.y),
                "background events must be unique for each runtime tile",
            ));
        }
    }
}

fn runtime_map_tile_bounds(module: &MapModule) -> Option<(i16, i16)> {
    let width = i32::from(module.attributes.width).checked_mul(i32::from(METATILE_WIDTH))?;
    let height = i32::from(module.attributes.height).checked_mul(i32::from(METATILE_WIDTH))?;
    if width > i32::from(i16::MAX) || height > i32::from(i16::MAX) {
        return None;
    }
    Some((i16::try_from(width).ok()?, i16::try_from(height).ok()?))
}

fn verify_raw_event_runtime_position(
    map_name: &str,
    event_kind: &str,
    subject: &str,
    x: u16,
    y: u16,
    bounds: (i16, i16),
    diagnostics: &mut Vec<VerificationError>,
) {
    if x > u16::from(u8::MAX) || y > u16::from(u8::MAX) {
        diagnostics.push(VerificationError::error(
            "map_event_coordinate_storage_overflow",
            format!("{map_name}:{event_kind}:{subject}"),
            format!(
                "{event_kind} coordinate ({x}, {y}) cannot fit its byte-sized ASM event record"
            ),
        ));
    }
    let Some(tile) = checked_runtime_map_event_tile(x, y) else {
        diagnostics.push(VerificationError::error(
            "map_event_runtime_position_overflow",
            format!("{map_name}:{subject}"),
            format!("{event_kind} coordinate ({x}, {y}) overflows runtime tile coordinates"),
        ));
        return;
    };
    if tile.x < 0
        || tile.y < 0
        || i32::from(tile.x) >= i32::from(bounds.0)
        || i32::from(tile.y) >= i32::from(bounds.1)
    {
        diagnostics.push(VerificationError::error(
            "map_event_runtime_position_out_of_bounds",
            format!("{map_name}:{subject}"),
            format!(
                "{event_kind} raw coordinate ({x}, {y}) resolves to runtime tile ({}, {}) outside map bounds {}x{}",
                tile.x, tile.y, bounds.0, bounds.1
            ),
        ));
    }
}

fn checked_runtime_map_event_tile(x: u16, y: u16) -> Option<TilePosition> {
    raw_event_tile_to_runtime_tile_checked(x, y)
}

fn coord_event_scenes_overlap(left: &str, right: &str) -> bool {
    left.is_empty() || right.is_empty() || left == right
}

fn is_exact_script_label_reference_token(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'@'))
}

fn verify_hidden_item_bg_events(
    map_name: &str,
    module: &MapModule,
    diagnostics: &mut Vec<VerificationError>,
) {
    for event in &module.events.bg_events {
        if event.event_type != "BGEVENT_ITEM" {
            continue;
        }
        let subject = format!("{map_name}:{}:{},{}", event.script, event.x, event.y);
        let matches = module
            .script_field_pickups
            .iter()
            .filter(|pickup| pickup.command == "hiddenitem" && pickup.source_script == event.script)
            .count();
        match matches {
            0 => diagnostics.push(VerificationError::error(
                "hidden_item_bg_event_missing_pickup",
                &subject,
                format!(
                    "BGEVENT_ITEM script '{}' has no exact hiddenitem pickup",
                    event.script
                ),
            )),
            1 => {}
            _ => diagnostics.push(VerificationError::error(
                "hidden_item_bg_event_duplicate_pickup",
                &subject,
                format!(
                    "BGEVENT_ITEM script '{}' resolves to {matches} hiddenitem pickups",
                    event.script
                ),
            )),
        }
    }
}

fn script_field_pickup_issue_diagnostic(
    subject: &str,
    pickup: &ScriptFieldPickup,
    issue: ScriptFieldPickupIssue,
) -> VerificationError {
    match issue {
        ScriptFieldPickupIssue::InvalidCommand => VerificationError::error(
            "invalid_script_field_pickup_command",
            subject,
            format!(
                "field pickup command must be an exact lowercase pack token, found {:?}",
                pickup.command
            ),
        ),
        ScriptFieldPickupIssue::MissingItem => VerificationError::error(
            "script_field_pickup_missing_item",
            subject,
            format!("{} pickup is missing item_id", pickup.command),
        ),
        ScriptFieldPickupIssue::InvalidItem => {
            let item_id = optional_id_for_diagnostic(pickup.item_id.as_deref());
            VerificationError::error(
                "invalid_script_field_pickup_item",
                subject,
                format!("{} pickup has invalid item id {item_id}", pickup.command),
            )
        }
        ScriptFieldPickupIssue::UnknownItem => {
            let item_id = optional_id_for_diagnostic(pickup.item_id.as_deref());
            VerificationError::error(
                "unknown_script_field_pickup_item",
                subject,
                format!(
                    "{} pickup references missing item {item_id}",
                    pickup.command
                ),
            )
        }
        ScriptFieldPickupIssue::InvalidQuantity => VerificationError::error(
            "script_field_pickup_invalid_quantity",
            subject,
            format!("{} pickup has zero quantity", pickup.command),
        ),
        ScriptFieldPickupIssue::MissingEvent => VerificationError::error(
            "script_field_pickup_missing_event",
            subject,
            format!("{} pickup is missing event_flag", pickup.command),
        ),
        ScriptFieldPickupIssue::InvalidCollectibleFlag => {
            let event_flag = optional_id_for_diagnostic(pickup.event_flag.as_deref());
            VerificationError::error(
                "script_field_pickup_uncollectible_event",
                subject,
                format!(
                    "{} pickup uses uncollectible event flag {event_flag}",
                    pickup.command
                ),
            )
        }
        ScriptFieldPickupIssue::MissingFruitTree => VerificationError::error(
            "script_field_pickup_missing_fruit_tree",
            subject,
            "fruittree pickup is missing fruit_tree_id",
        ),
        ScriptFieldPickupIssue::EmptyFruitTree => VerificationError::error(
            "script_field_pickup_empty_fruit_tree",
            subject,
            "fruittree pickup has an empty fruit_tree_id",
        ),
        ScriptFieldPickupIssue::InvalidFruitTree => {
            let fruit_tree_id = optional_id_for_diagnostic(pickup.fruit_tree_id.as_deref());
            VerificationError::error(
                "script_field_pickup_invalid_fruit_tree",
                subject,
                format!("fruittree pickup has invalid fruit_tree_id {fruit_tree_id}"),
            )
        }
        ScriptFieldPickupIssue::UnknownFruitTree => {
            let fruit_tree_id = optional_id_for_diagnostic(pickup.fruit_tree_id.as_deref());
            VerificationError::error(
                "unknown_script_field_fruit_tree",
                subject,
                format!("fruittree references missing tree {fruit_tree_id}"),
            )
        }
        ScriptFieldPickupIssue::MalformedFruitTree => VerificationError::error(
            "script_field_pickup_malformed_fruit_tree",
            subject,
            "fruittree pickup must not inline item_id or event_flag",
        ),
        ScriptFieldPickupIssue::UnknownCommand => VerificationError::error(
            "unknown_script_field_pickup_command",
            subject,
            format!("unknown field pickup command '{}'", pickup.command),
        ),
    }
}

fn verify_phone_contacts(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    verify_unique_runtime_map_bindings(data, diagnostics);
    let map_constants = map_constants(data);
    diagnostics.extend(
        phone_contact_catalog_issues(
            &data.phone_contacts,
            &data.permanent_phone_numbers,
            &map_constants,
        )
        .into_iter()
        .map(phone_contact_catalog_issue_diagnostic),
    );

    let global_module = data.global_scripts.as_ref();
    let executable_positions = global_module
        .map(global_runtime_executable_script_command_positions)
        .unwrap_or_default();
    if !data.phone_contacts.0.is_empty() {
        for (script, diagnostic_name) in [
            ("LoadPhoneScriptBank", "pokegear_phone_callback_script"),
            ("LoadOutOfAreaScript", "pokegear_phone_callback_script"),
            (
                "PhoneScript_JustTalkToThem",
                "pokegear_phone_same_map_script",
            ),
            ("PhoneOutOfAreaScript", "pokegear_phone_out_of_area_script"),
        ] {
            verify_dynamic_phone_script_root(
                "phone_contacts",
                diagnostic_name,
                script,
                global_module,
                &executable_positions,
                diagnostics,
            );
        }
    }
    for (contact_id, contact) in &data.phone_contacts.0 {
        for (field, script) in [
            ("calleeScript", contact.callee_script.as_deref()),
            ("callerScript", contact.caller_script.as_deref()),
        ] {
            let Some(script) = script else {
                continue;
            };
            verify_dynamic_phone_script_root(
                &format!("phone_contacts:{contact_id}:{field}"),
                if field == "calleeScript" {
                    "phone_contact_callee_script"
                } else {
                    "phone_contact_caller_script"
                },
                script,
                global_module,
                &executable_positions,
                diagnostics,
            );
        }
    }

    let mut values = BTreeMap::<u8, &str>::new();
    for (call_id, rule) in &data.special_phone_calls {
        let subject = format!("special_phone_calls:{call_id}");
        if !is_exact_script_label_reference_token(call_id) {
            diagnostics.push(VerificationError::error(
                "invalid_special_phone_call_id",
                &subject,
                format!("special phone call id '{call_id}' is not an exact token"),
            ));
        }
        if rule.value == 0 {
            diagnostics.push(VerificationError::error(
                "invalid_special_phone_call_value",
                &subject,
                "special phone call value zero is reserved for SPECIALCALL_NONE",
            ));
        } else if let Some(previous) = values.insert(rule.value, call_id) {
            diagnostics.push(VerificationError::error(
                "duplicate_special_phone_call_value",
                &subject,
                format!(
                    "special phone call value {} is already owned by {previous}",
                    rule.value
                ),
            ));
        }
        if !matches!(
            rule.condition.as_str(),
            "SpecialCallOnlyWhenOutside" | "SpecialCallWhereverYouAre"
        ) {
            diagnostics.push(VerificationError::error(
                "invalid_special_phone_call_condition",
                &subject,
                format!(
                    "special phone call condition '{}' has no Rust runtime predicate",
                    rule.condition
                ),
            ));
        }
        if !data.phone_contacts.0.contains_key(&rule.contact_id) {
            diagnostics.push(VerificationError::error(
                "unknown_special_phone_call_contact",
                &subject,
                format!(
                    "special phone call references missing contact '{}'",
                    rule.contact_id
                ),
            ));
        }
        if !is_exact_script_label_reference_token(&rule.caller_script) {
            diagnostics.push(VerificationError::error(
                "invalid_special_phone_call_caller_script",
                &subject,
                format!(
                    "special phone call callerScript '{}' is not an exact script label",
                    rule.caller_script
                ),
            ));
        } else {
            verify_dynamic_phone_script_root(
                &subject,
                "special_phone_call_caller_script",
                &rule.caller_script,
                global_module,
                &executable_positions,
                diagnostics,
            );
        }
    }
}

fn verify_dynamic_phone_script_root(
    subject: &str,
    diagnostic_name: &str,
    script: &str,
    global_module: Option<&GlobalScriptModule>,
    executable_positions: &BTreeSet<(String, usize)>,
    diagnostics: &mut Vec<VerificationError>,
) {
    let Some(module) = global_module else {
        diagnostics.push(VerificationError::error(
            format!("unknown_{diagnostic_name}"),
            subject,
            format!("dynamic phone script '{script}' is absent from global scripts"),
        ));
        return;
    };
    let Some(body) = module.scripts.get(script) else {
        diagnostics.push(VerificationError::error(
            format!("unknown_{diagnostic_name}"),
            subject,
            format!("dynamic phone script '{script}' is absent from global scripts"),
        ));
        return;
    };
    let first_command = body
        .as_array()
        .and_then(|commands| commands.first())
        .and_then(|command| command.get("command"))
        .and_then(Value::as_str)
        .unwrap_or("<missing command>");
    if !executable_positions.contains(&(script.to_string(), 0)) {
        diagnostics.push(VerificationError::error(
            format!("non_executable_{diagnostic_name}"),
            subject,
            format!(
                "dynamic phone script '{script}' begins with '{first_command}', which has no Rust runtime mutation"
            ),
        ));
    }
}

fn phone_contact_catalog_issue_diagnostic(issue: PhoneContactCatalogIssue) -> VerificationError {
    match issue {
        PhoneContactCatalogIssue::EmptyContactId { contact_id } => VerificationError::error(
            "empty_phone_contact_id",
            format!("phone_contacts:{contact_id}"),
            "phone contact catalog keys must be nonempty exact ids",
        ),
        PhoneContactCatalogIssue::InvalidContactId { contact_id } => VerificationError::error(
            "invalid_phone_contact_id",
            format!("phone_contacts:{contact_id}"),
            format!("phone contact catalog key must be exact and untrimmed, found {contact_id:?}"),
        ),
        PhoneContactCatalogIssue::ContactIdMismatch {
            contact_id,
            record_contact_id,
        } => VerificationError::error(
            "phone_contact_id_mismatch",
            format!("phone_contacts:{contact_id}"),
            format!(
                "phone contact key '{contact_id}' does not match record contactId '{record_contact_id}'"
            ),
        ),
        PhoneContactCatalogIssue::InvalidTrainerClass {
            contact_id,
            trainer_class,
        } => VerificationError::error(
            "invalid_phone_contact_trainer_class",
            format!("phone_contacts:{contact_id}"),
            format!(
                "phone contact trainerClass must be an exact pack token, found {trainer_class:?}"
            ),
        ),
        PhoneContactCatalogIssue::InvalidTrainerLabel {
            contact_id,
            trainer_label,
        } => VerificationError::error(
            "invalid_phone_contact_trainer_label",
            format!("phone_contacts:{contact_id}"),
            format!(
                "phone contact trainerLabel must be an exact pack token, found {trainer_label:?}"
            ),
        ),
        PhoneContactCatalogIssue::EmptyPrimaryLabel { contact_id } => VerificationError::error(
            "empty_phone_contact_primary_label",
            format!("phone_contacts:{contact_id}"),
            "phone contact primaryLabel must be nonempty",
        ),
        PhoneContactCatalogIssue::InvalidLines { contact_id } => VerificationError::error(
            "invalid_phone_contact_lines",
            format!("phone_contacts:{contact_id}"),
            "phone contact display lines must be nonempty",
        ),
        PhoneContactCatalogIssue::PrimaryLabelMismatch {
            contact_id,
            primary_label,
            first_line,
        } => VerificationError::error(
            "phone_contact_primary_label_mismatch",
            format!("phone_contacts:{contact_id}"),
            format!(
                "phone contact primaryLabel '{primary_label}' does not match first display line '{first_line}'"
            ),
        ),
        PhoneContactCatalogIssue::EmptyMapConstant { contact_id } => VerificationError::error(
            "empty_phone_contact_map",
            format!("phone_contacts:{contact_id}"),
            "phone contact mapConstant must be nonempty when present",
        ),
        PhoneContactCatalogIssue::InvalidMapConstant {
            contact_id,
            map_constant,
        } => VerificationError::error(
            "invalid_phone_contact_map",
            format!("phone_contacts:{contact_id}"),
            format!(
                "phone contact mapConstant must be an exact pack token, found {map_constant:?}"
            ),
        ),
        PhoneContactCatalogIssue::UnknownMapConstant {
            contact_id,
            map_constant,
        } => VerificationError::error(
            "unknown_phone_contact_map",
            format!("phone_contacts:{contact_id}"),
            format!("phone contact references missing map constant '{map_constant}'"),
        ),
        PhoneContactCatalogIssue::InvalidCalleeScript {
            contact_id,
            callee_script,
        } => VerificationError::error(
            "invalid_phone_contact_callee_script",
            format!("phone_contacts:{contact_id}"),
            format!(
                "phone contact calleeScript must be an exact pack token, found {callee_script:?}"
            ),
        ),
        PhoneContactCatalogIssue::InvalidCallerScript {
            contact_id,
            caller_script,
        } => VerificationError::error(
            "invalid_phone_contact_caller_script",
            format!("phone_contacts:{contact_id}"),
            format!(
                "phone contact callerScript must be an exact pack token, found {caller_script:?}"
            ),
        ),
        PhoneContactCatalogIssue::UnknownPermanentContact { contact_id } => {
            VerificationError::error(
                "unknown_permanent_phone_contact",
                &contact_id,
                format!("permanent phone number references unknown contact '{contact_id}'"),
            )
        }
        PhoneContactCatalogIssue::InvalidPermanentContact { contact_id } => {
            VerificationError::error(
                "invalid_permanent_phone_contact",
                &contact_id,
                format!(
                    "permanent phone number id must be an exact pack token, found {contact_id:?}"
                ),
            )
        }
    }
}

fn verify_required_object_sections(
    subject: &str,
    value: &str,
    sections: &[&str],
    diagnostics: &mut Vec<VerificationError>,
) {
    diagnostics.extend(
        runtime_bundle_issues(value, sections)
            .into_iter()
            .map(|issue| runtime_bundle_issue_diagnostic(subject, issue)),
    );
}

fn verify_runtime_pack_data(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    diagnostics.extend(
        runtime_pack_presence_issues(runtime_pack_sections(data))
            .into_iter()
            .map(runtime_pack_presence_issue_diagnostic),
    );

    let map_constants = map_constants(data);
    diagnostics.extend(
        runtime_map_metadata_issues(&data.runtime_map_metadata, &map_constants)
            .into_iter()
            .map(runtime_map_metadata_issue_diagnostic),
    );

    let runtime_map_names: BTreeMap<String, String> = data
        .runtime_map_metadata
        .iter()
        .map(|(constant, metadata)| (constant.clone(), metadata.name.clone()))
        .collect();
    diagnostics.extend(
        runtime_spawn_point_catalog_issues(&data.runtime_spawn_points, &runtime_map_names)
            .into_iter()
            .map(runtime_spawn_point_catalog_issue_diagnostic),
    );
    verify_runtime_spawn_points_within_declared_maps(data, diagnostics);

    diagnostics.extend(
        initialize_events_issues(&data.initialize_events)
            .into_iter()
            .map(initialize_events_issue_diagnostic),
    );

    diagnostics.extend(
        story_event_script_constant_issues(&data.story_event_script_constants)
            .into_iter()
            .map(story_event_script_constant_issue_diagnostic),
    );

    diagnostics.extend(
        asm_text_catalog_issues(&data.asm_text)
            .into_iter()
            .map(asm_text_catalog_issue_diagnostic),
    );
    diagnostics.extend(
        move_name_catalog_issues(&data.move_names, data.moves.len())
            .into_iter()
            .map(move_name_catalog_issue_diagnostic),
    );
    diagnostics.extend(
        battle_animation_catalog_issues(
            &data.battle_animations,
            &data.battle_animation_table,
            data.moves.len(),
        )
        .into_iter()
        .map(battle_animation_catalog_issue_diagnostic),
    );
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
    diagnostics.extend(
        sprite_palette_default_issues(&data.sprite_palette_defaults)
            .into_iter()
            .map(sprite_palette_default_issue_diagnostic),
    );
    diagnostics.extend(
        pokegear_town_map_palette_issues(&data.pokegear_town_map_palette_map)
            .into_iter()
            .map(pokegear_town_map_palette_issue_diagnostic),
    );
    verify_definitive_map_modules(data, diagnostics);
    verify_tileset_references(data, diagnostics);
    let map_names: BTreeSet<String> = data.maps.keys().cloned().collect();
    diagnostics.extend(
        pokegear_landmark_issues(&data.pokegear_landmarks, &map_names)
            .into_iter()
            .map(pokegear_landmark_issue_diagnostic),
    );
    verify_unique_pokegear_landmark_constants(&data.pokegear_landmarks, diagnostics);
    let (_, _, cry_audio) = script_audio_catalog_ids(data);
    for (species_id, cry) in &data.pokemon_cries {
        if species_id.trim().is_empty() || cry.cry.trim().is_empty() {
            diagnostics.push(VerificationError::error(
                "invalid_pokemon_cry_metadata",
                species_id,
                "Pokemon cry metadata requires non-empty exact keys and non-empty cry ids",
            ));
        } else if !is_exact_audio_reference_token(species_id) {
            diagnostics.push(VerificationError::error(
                "invalid_pokemon_cry_species",
                species_id,
                format!(
                    "Pokemon cry metadata species id must be an exact pack token, found {species_id:?}"
                ),
            ));
        } else if !data.pokemon.contains_key(species_id) {
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
        if !is_exact_audio_reference_token(&cry.cry) {
            diagnostics.push(VerificationError::error(
                "invalid_species_cry_audio",
                species_id,
                format!(
                    "Pokemon species cry audio id must be an exact pack token, found {:?}",
                    cry.cry
                ),
            ));
        } else if !cry_audio.contains(&cry.cry) {
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

    diagnostics.extend(
        pc_string_catalog_issues(&data.pc_strings)
            .into_iter()
            .map(pc_string_catalog_issue_diagnostic),
    );

    let species_ids: BTreeSet<String> = data.pokemon.keys().cloned().collect();
    diagnostics.extend(
        flee_mon_catalog_issues(&data.flee_mons, &species_ids)
            .into_iter()
            .map(flee_mon_catalog_issue_diagnostic),
    );
    diagnostics.extend(
        menu_icon_catalog_issues(&data.menu_icons, &species_ids)
            .into_iter()
            .map(menu_icon_catalog_issue_diagnostic),
    );
    diagnostics.extend(
        pokedex_entry_catalog_issues(&data.pokedex_entries, &species_ids)
            .into_iter()
            .map(pokedex_entry_catalog_issue_diagnostic),
    );
    diagnostics.extend(
        frontpic_anim_catalog_issues(&data.pokemon_frontpic_anim, &species_ids)
            .into_iter()
            .map(frontpic_anim_catalog_issue_diagnostic),
    );
}

fn verify_definitive_map_modules(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    for map_name in data.map_attributes.keys() {
        if !data.maps.contains_key(map_name) {
            diagnostics.push(VerificationError::error(
                "missing_compiled_map_module",
                map_name,
                "runtime map attributes must have a definitive compiled map module",
            ));
        }
    }
    for map_name in data.maps.keys() {
        if !data.map_attributes.contains_key(map_name) {
            diagnostics.push(VerificationError::error(
                "missing_compiled_map_attributes",
                map_name,
                "compiled map module must have matching runtime map attributes",
            ));
        }
    }
}

fn verify_tileset_references(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    for (map_name, attributes) in &data.map_attributes {
        verify_map_tileset_reference(map_name, &attributes.tileset_name, data, diagnostics);
    }
    for (map_name, module) in &data.maps {
        verify_map_tileset_reference(map_name, &module.attributes.tileset_name, data, diagnostics);
    }
    for tileset_id in data.tilesets.keys() {
        if !is_exact_tileset_id(tileset_id) {
            diagnostics.push(VerificationError::error(
                "invalid_tileset_id",
                tileset_id,
                format!("tileset id must be an exact asset id, found {tileset_id:?}"),
            ));
        }
    }
}

fn verify_map_tileset_reference(
    map_name: &str,
    tileset_id: &str,
    data: &GameDataSet,
    diagnostics: &mut Vec<VerificationError>,
) {
    if !is_exact_tileset_id(tileset_id) {
        diagnostics.push(VerificationError::error(
            "invalid_map_tileset",
            map_name,
            format!("map tileset_name must be an exact tileset id, found {tileset_id:?}"),
        ));
    } else if !data.tilesets.contains_key(tileset_id) {
        diagnostics.push(VerificationError::error(
            "unknown_map_tileset",
            map_name,
            format!("map references missing tileset '{tileset_id}'"),
        ));
    }
}

fn runtime_pack_sections(data: &GameDataSet) -> RuntimePackSections {
    let has_music_audio = data
        .audio
        .iter()
        .any(|asset| asset.kind == ModpackAudioKind::Music && asset.validate().is_ok());
    let has_sound_effects = data
        .audio
        .iter()
        .any(|asset| asset.kind == ModpackAudioKind::SoundEffect && asset.validate().is_ok());
    let has_cry_audio = data
        .audio
        .iter()
        .any(|asset| asset.kind == ModpackAudioKind::Cry && asset.validate().is_ok());
    RuntimePackSections {
        has_pokemon: !data.pokemon.is_empty(),
        has_moves: !data.moves.is_empty(),
        has_growth_rates: !data.growth_rates.is_empty(),
        has_learnsets: !data.learnsets.is_empty(),
        has_evolutions: !data.evolutions.0.is_empty(),
        has_capture_rules: has_capture_rules(data),
        has_capture_wobble_probabilities: has_capture_wobble_probabilities(data),
        has_battle_stat_multipliers: has_battle_stat_multipliers(data),
        has_move_priorities: has_move_priorities(data),
        has_type_categories: has_type_categories(data),
        has_type_effectiveness: has_type_effectiveness(data),
        has_weather_modifiers: has_weather_modifiers(data),
        has_battle_reward_rules: has_battle_reward_rules(data),
        has_battle_escape_rules: has_battle_escape_rules(data),
        has_marts: has_marts(data),
        has_currency_constants: has_currency_constants(data),
        has_step_event_rules: has_step_event_rules(data),
        has_fishing_catalog: has_fishing_catalog(data),
        has_fruit_trees: has_fruit_trees(data),
        has_field_moves: has_field_moves(data),
        has_runtime_title_screen: has_runtime_title_screen(data),
        has_items: !data.items.is_empty(),
        has_ball_items: data
            .items
            .values()
            .any(|item| item.pocket == ITEM_POCKET_BALL),
        has_tmhm_items: data
            .items
            .values()
            .any(|item| item.pocket == ITEM_POCKET_TM_HM),
        has_trainers: !data.trainers.is_empty(),
        has_audio: data.audio.iter().any(|asset| asset.validate().is_ok()),
        has_music_audio,
        has_sound_effects,
        has_cry_audio,
        has_pokemon_cries: !data.pokemon_cries.is_empty(),
        has_tilesets: !data.tilesets.is_empty(),
        has_scripts: has_runtime_script_data(data),
        has_map_geometry: has_runtime_map_geometry(data),
        has_map_objects: has_runtime_map_objects(data),
        has_runtime_map_metadata: !data.runtime_map_metadata.is_empty(),
        has_runtime_spawn_points: !data.runtime_spawn_points.is_empty(),
        has_maps: !data.maps.is_empty(),
        has_pc_strings: has_pc_strings(data),
        has_menu_icons: has_menu_icons(data),
        has_pokedex_entries: has_pokedex_entries(data),
        has_pokemon_frontpic_animations: has_pokemon_frontpic_animations(data),
        has_move_names: has_move_names(data),
        has_asm_text: has_asm_text(data),
        has_battle_animations: has_battle_animations(data),
        has_battle_animation_table: has_battle_animation_table(data),
        has_battle_anim_bundle: has_battle_anim_bundle(data),
        has_sprite_anim_bundle: has_sprite_anim_bundle(data),
        has_sprite_palette_defaults: has_sprite_palette_defaults(data),
        has_pokegear_town_map_palettes: has_pokegear_town_map_palettes(data),
        has_pokegear_landmarks: has_pokegear_landmarks(data),
        has_phone_contacts: has_phone_contacts(data),
        has_permanent_phone_numbers: has_permanent_phone_numbers(data),
        has_special_phone_calls: has_special_phone_calls(data),
        has_phone_scripts: has_phone_scripts(data),
        has_flee_mons: has_flee_mons(data),
        has_buena_password_categories: has_buena_password_categories(data),
        has_roaming_pokemon: has_roaming_pokemon(data),
        has_buena_prizes: has_buena_prizes(data),
        has_kurt_apricorn_recipes: has_kurt_apricorn_recipes(data),
        has_shuckie_gift: has_shuckie_gift(data),
        has_dratini_move_sets: has_dratini_move_sets(data),
        has_bug_contest_config: has_bug_contest_config(data),
        has_battle_tower_rules: has_battle_tower_rules(data),
        has_oak_ratings: has_oak_ratings(data),
        has_odd_egg_definitions: has_odd_egg_definitions(data),
        has_magikarp_lengths: has_magikarp_lengths(data),
        has_happiness_data: has_happiness_data(data),
        has_initialize_events: has_initialize_events(data),
        has_story_event_script_constants: has_story_event_script_constants(data),
    }
}

fn sprite_palette_default_issue_diagnostic(issue: SpritePaletteDefaultIssue) -> VerificationError {
    match issue {
        SpritePaletteDefaultIssue::InvalidDefault { sprite_id } => VerificationError::error(
            "invalid_sprite_palette_default",
            &sprite_id,
            "sprite palette defaults require exact non-empty sprite ids and non-negative palettes",
        ),
    }
}

fn battle_animation_catalog_issue_diagnostic(
    issue: BattleAnimationCatalogIssue,
) -> VerificationError {
    match issue {
        BattleAnimationCatalogIssue::InvalidAnimation { label } => VerificationError::error(
            "invalid_battle_animation",
            &label,
            "battle animation labels must be exact non-empty values and command lists must be non-empty",
        ),
        BattleAnimationCatalogIssue::InvalidCommand {
            label,
            index,
            command,
        } => VerificationError::error(
            "invalid_battle_animation_command",
            format!("{label}:{index}"),
            format!("battle animation command is not canonical ASM: {command:?}"),
        ),
        BattleAnimationCatalogIssue::UnknownCommandTarget {
            label,
            index,
            target,
        } => VerificationError::error(
            "unknown_battle_animation_command_target",
            format!("{label}:{index}"),
            format!("battle animation command references missing ASM target {target:?}"),
        ),
        BattleAnimationCatalogIssue::InvalidTableEntry { index } => VerificationError::error(
            "invalid_battle_animation_table_entry",
            index.to_string(),
            "battle animation table labels must be exact non-empty values",
        ),
        BattleAnimationCatalogIssue::UnknownTableAnimation { index, label } => {
            VerificationError::error(
                "unknown_battle_animation_table_entry",
                index.to_string(),
                format!("battle animation table references missing animation '{label}'"),
            )
        }
        BattleAnimationCatalogIssue::TableCountMismatch {
            actual_count,
            expected_count,
        } => VerificationError::error(
            "battle_animation_table_count_mismatch",
            "battle_animation_table",
            format!(
                "battle animation table contains {actual_count} entries but moves plus dummy contains {expected_count}"
            ),
        ),
    }
}

fn move_name_catalog_issue_diagnostic(issue: MoveNameCatalogIssue) -> VerificationError {
    match issue {
        MoveNameCatalogIssue::CountMismatch {
            actual_count,
            expected_count,
        } => VerificationError::error(
            "move_names_count_mismatch",
            "move_names",
            format!(
                "move_names contains {actual_count} entries but moves contains {expected_count}"
            ),
        ),
        MoveNameCatalogIssue::InvalidName { index } => VerificationError::error(
            "invalid_move_name",
            index.to_string(),
            "move name must be an exact non-empty value",
        ),
    }
}

fn story_event_script_constant_issue_diagnostic(
    issue: StoryEventScriptConstantIssue,
) -> VerificationError {
    match issue {
        StoryEventScriptConstantIssue::InvalidGlobalConstant { key } => VerificationError::error(
            "invalid_story_event_script_constant",
            &key,
            "global story event script constant keys must be non-empty",
        ),
        StoryEventScriptConstantIssue::InvalidMap { map_name } => VerificationError::error(
            "invalid_story_event_script_constant_map",
            &map_name,
            "story event script constant map keys must be non-empty",
        ),
        StoryEventScriptConstantIssue::InvalidMapConstant { map_name, key } => {
            let subject = format!("{map_name}:{key}");
            VerificationError::error(
                "invalid_story_event_script_constant",
                subject,
                "map story event script constant keys must be non-empty",
            )
        }
    }
}

fn verify_unique_runtime_map_bindings(
    data: &GameDataSet,
    diagnostics: &mut Vec<VerificationError>,
) {
    let mut map_constants: BTreeMap<&str, &str> = BTreeMap::new();
    for (map_name, module) in &data.maps {
        let Some(map_constant) = module.attributes.map_constant.as_deref() else {
            continue;
        };
        if let Some(previous_map) = map_constants.insert(map_constant, map_name.as_str()) {
            diagnostics.push(VerificationError::error(
                "duplicate_runtime_map_constant",
                map_constant,
                format!(
                    "maps '{}' and '{}' both declare map constant '{}'",
                    previous_map, map_name, map_constant
                ),
            ));
        }
    }

    let mut metadata_names: BTreeMap<&str, &str> = BTreeMap::new();
    for (map_constant, metadata) in &data.runtime_map_metadata {
        if let Some(previous_constant) = metadata_names.insert(&metadata.name, map_constant) {
            diagnostics.push(VerificationError::error(
                "duplicate_runtime_map_metadata_name",
                &metadata.name,
                format!(
                    "runtime map metadata '{}' and '{}' both name map '{}'",
                    previous_constant, map_constant, metadata.name
                ),
            ));
        }
    }
}

fn initialize_events_issue_diagnostic(issue: InitializeEventsIssue) -> VerificationError {
    match issue {
        InitializeEventsIssue::InvalidFlag { flag } => VerificationError::error(
            "invalid_initialize_event_flag",
            &flag,
            "initialize event and engine flags must be non-empty",
        ),
        InitializeEventsIssue::InvalidVariableSprite { sprite } => VerificationError::error(
            "invalid_initialize_event_sprite",
            &sprite,
            "initialize event variable sprite keys and values must be non-empty",
        ),
    }
}

fn runtime_spawn_point_catalog_issue_diagnostic(
    issue: RuntimeSpawnPointCatalogIssue,
) -> VerificationError {
    match issue {
        RuntimeSpawnPointCatalogIssue::IdentifierMismatch { key, identifier } => {
            VerificationError::error(
                "runtime_spawn_point_identifier_mismatch",
                &key,
                format!(
                    "runtime spawn point key '{}' does not match identifier {}",
                    key, identifier
                ),
            )
        }
        RuntimeSpawnPointCatalogIssue::MapMismatch {
            key,
            map_name,
            metadata_name,
        } => VerificationError::error(
            "runtime_spawn_point_map_mismatch",
            &key,
            format!(
                "runtime spawn point targets '{map_name}' but metadata names '{metadata_name}'"
            ),
        ),
        RuntimeSpawnPointCatalogIssue::UnknownMap { key, map_constant } => {
            VerificationError::error(
                "unknown_runtime_spawn_point_map",
                &key,
                format!("runtime spawn point references missing map constant '{map_constant}'"),
            )
        }
        RuntimeSpawnPointCatalogIssue::InvalidSpawnPoint { key } => VerificationError::error(
            "invalid_runtime_spawn_point",
            &key,
            "runtime spawn point id, map constant, map name, and groupName must be exact non-empty values",
        ),
        RuntimeSpawnPointCatalogIssue::CoordinateMismatch {
            key,
            tile_x,
            tile_y,
            expected_tile_x,
            expected_tile_y,
        } => VerificationError::error(
            "runtime_spawn_point_coordinate_mismatch",
            &key,
            format!(
                "runtime spawn point tile ({tile_x}, {tile_y}) does not match metatile/subtile-derived tile ({expected_tile_x}, {expected_tile_y})"
            ),
        ),
        RuntimeSpawnPointCatalogIssue::CoordinateOverflow {
            key,
            metatile_x,
            metatile_y,
            subtile_x,
            subtile_y,
        } => VerificationError::error(
            "runtime_spawn_point_coordinate_overflow",
            &key,
            format!(
                "runtime spawn point metatile/subtile coordinate ({metatile_x}, {metatile_y}) + ({subtile_x}, {subtile_y}) overflows runtime tile coordinates"
            ),
        ),
        RuntimeSpawnPointCatalogIssue::InvalidSubtile {
            key,
            subtile_x,
            subtile_y,
            metatile_width,
        } => VerificationError::error(
            "invalid_runtime_spawn_point_subtile",
            &key,
            format!(
                "runtime spawn point subtile ({subtile_x}, {subtile_y}) must be in range 0..{metatile_width}"
            ),
        ),
        RuntimeSpawnPointCatalogIssue::DuplicateMapBinding {
            key,
            existing_key,
            group_id,
            map_id,
        } => VerificationError::error(
            "duplicate_runtime_spawn_point_map_binding",
            &key,
            format!(
                "runtime spawn point duplicates group {group_id} map {map_id} already bound by spawn point {existing_key}"
            ),
        ),
    }
}

fn verify_runtime_spawn_points_within_declared_maps(
    data: &GameDataSet,
    diagnostics: &mut Vec<VerificationError>,
) {
    for (key, spawn) in &data.runtime_spawn_points {
        let Some(metadata) = data.runtime_map_metadata.get(&spawn.map_constant) else {
            continue;
        };
        if !runtime_spawn_subtiles_are_valid(spawn) {
            continue;
        }
        let Some(tile) = checked_runtime_spawn_expected_tile(spawn) else {
            diagnostics.push(VerificationError::error(
                "runtime_spawn_point_coordinate_overflow",
                key,
                format!(
                    "runtime spawn point metatile/subtile coordinate ({}, {}) + ({}, {}) overflows runtime tile coordinates",
                    spawn.metatile_x, spawn.metatile_y, spawn.subtile_x, spawn.subtile_y
                ),
            ));
            continue;
        };
        let max_x = i32::from(metadata.width) * i32::from(METATILE_WIDTH);
        let max_y = i32::from(metadata.height) * i32::from(METATILE_WIDTH);
        let out_of_bounds = i32::from(tile.x) < 0
            || i32::from(tile.y) < 0
            || i32::from(tile.x) >= max_x
            || i32::from(tile.y) >= max_y;
        if out_of_bounds {
            diagnostics.push(VerificationError::error(
                "runtime_spawn_point_out_of_bounds",
                key,
                format!(
                    "runtime spawn point resolves to tile ({}, {}) outside map {} runtime bounds {}x{}",
                    tile.x, tile.y, metadata.name, max_x, max_y
                ),
            ));
            continue;
        }
        let Some(module) = data.maps.get(&metadata.name) else {
            continue;
        };
        let mut collision_diagnostics = Vec::new();
        let context = map_playability_context(
            data,
            module,
            &PlayabilityRules::default(),
            &mut collision_diagnostics,
        );
        diagnostics.extend(collision_diagnostics);
        if let Some(context) = context
            && context.component_at(tile).is_none()
        {
            diagnostics.push(VerificationError::error(
                "unwalkable_runtime_spawn_point",
                key,
                format!(
                    "runtime spawn point resolves to non-walkable tile ({}, {}) on {}",
                    tile.x, tile.y, metadata.name
                ),
            ));
        }
    }
}

fn runtime_map_metadata_issue_diagnostic(issue: RuntimeMapMetadataIssue) -> VerificationError {
    match issue {
        RuntimeMapMetadataIssue::ConstantMismatch {
            key,
            record_constant,
        } => VerificationError::error(
            "runtime_map_metadata_constant_mismatch",
            &key,
            format!(
                "runtime map metadata key '{}' does not match record constant '{}'",
                key, record_constant
            ),
        ),
        RuntimeMapMetadataIssue::NameMismatch {
            key,
            constant,
            metadata_name,
            map_name,
        } => VerificationError::error(
            "runtime_map_metadata_name_mismatch",
            &key,
            format!(
                "runtime map metadata '{constant}' names '{metadata_name}' but map attributes use '{map_name}'"
            ),
        ),
        RuntimeMapMetadataIssue::UnknownMapConstant { key, constant } => VerificationError::error(
            "unknown_runtime_map_metadata_constant",
            &key,
            format!("runtime map metadata references missing map constant '{constant}'"),
        ),
        RuntimeMapMetadataIssue::InvalidMetadata { key } => VerificationError::error(
            "invalid_runtime_map_metadata",
            &key,
            "runtime map metadata groupName and environment must be non-empty",
        ),
    }
}

fn runtime_bundle_issue_diagnostic(subject: &str, issue: RuntimeBundleIssue) -> VerificationError {
    match issue {
        RuntimeBundleIssue::InvalidJson { error } => VerificationError::error(
            "invalid_runtime_bundle",
            subject,
            format!("runtime bundle payload is not valid JSON: {error}"),
        ),
        RuntimeBundleIssue::NotObject => VerificationError::error(
            "invalid_runtime_bundle",
            subject,
            "runtime bundle payload must be an object",
        ),
        RuntimeBundleIssue::MissingSection { section } => VerificationError::error(
            "missing_runtime_bundle_section",
            subject,
            format!("runtime bundle is missing non-empty section '{section}'"),
        ),
        RuntimeBundleIssue::UnknownSection { section } => VerificationError::error(
            "unknown_runtime_bundle_section",
            subject,
            format!("runtime bundle contains unknown section '{section}'"),
        ),
    }
}

fn runtime_pack_presence_issue_diagnostic(issue: RuntimePackPresenceIssue) -> VerificationError {
    match issue {
        RuntimePackPresenceIssue::MissingPokemon => VerificationError::error(
            "missing_runtime_pokemon",
            "pokemon",
            "runtime pack must include Pokemon species data",
        ),
        RuntimePackPresenceIssue::MissingMoves => VerificationError::error(
            "missing_runtime_moves",
            "moves",
            "runtime pack must include move data",
        ),
        RuntimePackPresenceIssue::MissingGrowthRates => VerificationError::error(
            "missing_runtime_growth_rates",
            "growth_rates",
            "runtime pack must include growth-rate data",
        ),
        RuntimePackPresenceIssue::MissingLearnsets => VerificationError::error(
            "missing_runtime_learnsets",
            "learnsets",
            "runtime pack must include Pokemon learnset data",
        ),
        RuntimePackPresenceIssue::MissingEvolutions => VerificationError::error(
            "missing_runtime_evolutions",
            "evolutions",
            "runtime pack must include Pokemon evolution data",
        ),
        RuntimePackPresenceIssue::MissingCaptureRules => VerificationError::error(
            "missing_runtime_capture_rules",
            "capture_rules",
            "runtime pack must include capture rules",
        ),
        RuntimePackPresenceIssue::MissingCaptureWobbleProbabilities => VerificationError::error(
            "missing_runtime_capture_wobble_probabilities",
            "capture_wobble_probabilities",
            "runtime pack must include capture wobble probability data",
        ),
        RuntimePackPresenceIssue::MissingBattleStatMultipliers => VerificationError::error(
            "missing_runtime_battle_stat_multipliers",
            "battle_stat_multipliers",
            "runtime pack must include battle stat multiplier tables",
        ),
        RuntimePackPresenceIssue::MissingMovePriorities => VerificationError::error(
            "missing_runtime_move_priorities",
            "move_priorities",
            "runtime pack must include move priority data",
        ),
        RuntimePackPresenceIssue::MissingTypeCategories => VerificationError::error(
            "missing_runtime_type_categories",
            "type_categories",
            "runtime pack must include type category data",
        ),
        RuntimePackPresenceIssue::MissingTypeEffectiveness => VerificationError::error(
            "missing_runtime_type_effectiveness",
            "type_effectiveness",
            "runtime pack must include type effectiveness tables",
        ),
        RuntimePackPresenceIssue::MissingWeatherModifiers => VerificationError::error(
            "missing_runtime_weather_modifiers",
            "weather_modifiers",
            "runtime pack must include weather modifier tables",
        ),
        RuntimePackPresenceIssue::MissingBattleRewardRules => VerificationError::error(
            "missing_runtime_battle_reward_rules",
            "battle_reward_rules",
            "runtime pack must include battle reward rules",
        ),
        RuntimePackPresenceIssue::MissingBattleEscapeRules => VerificationError::error(
            "missing_runtime_battle_escape_rules",
            "battle_escape_rules",
            "runtime pack must include battle escape rules",
        ),
        RuntimePackPresenceIssue::MissingMarts => VerificationError::error(
            "missing_runtime_marts",
            "marts",
            "runtime pack must include mart catalogs",
        ),
        RuntimePackPresenceIssue::MissingCurrencyConstants => VerificationError::error(
            "missing_runtime_currency_constants",
            "currency_constants",
            "runtime pack must include currency constants",
        ),
        RuntimePackPresenceIssue::MissingStepEventRules => VerificationError::error(
            "missing_runtime_step_event_rules",
            "step_event_rules",
            "runtime pack must include step event rules",
        ),
        RuntimePackPresenceIssue::MissingFishingCatalog => VerificationError::error(
            "missing_runtime_fishing_catalog",
            "fishing",
            "runtime pack must include fishing catalogs",
        ),
        RuntimePackPresenceIssue::MissingFruitTrees => VerificationError::error(
            "missing_runtime_fruit_trees",
            "fruit_trees",
            "runtime pack must include fruit tree data",
        ),
        RuntimePackPresenceIssue::MissingFieldMoves => VerificationError::error(
            "missing_runtime_field_moves",
            "field_moves",
            "runtime pack must include field move rules",
        ),
        RuntimePackPresenceIssue::MissingRuntimeTitleScreen => VerificationError::error(
            "missing_runtime_title_screen",
            "runtime_title_screen",
            "runtime pack must include title screen data",
        ),
        RuntimePackPresenceIssue::MissingItems => VerificationError::error(
            "missing_runtime_items",
            "items",
            "runtime pack must include item data",
        ),
        RuntimePackPresenceIssue::MissingBallItems => VerificationError::error(
            "missing_runtime_ball_items",
            "items",
            "runtime pack must include explicit ball item data",
        ),
        RuntimePackPresenceIssue::MissingTmHmItems => VerificationError::error(
            "missing_runtime_tmhm_items",
            "items",
            "runtime pack must include explicit TM/HM item data",
        ),
        RuntimePackPresenceIssue::MissingTrainers => VerificationError::error(
            "missing_runtime_trainers",
            "trainers",
            "runtime pack must include trainer data",
        ),
        RuntimePackPresenceIssue::MissingAudio => VerificationError::error(
            "missing_runtime_audio",
            "audio",
            "runtime pack must include audio asset data",
        ),
        RuntimePackPresenceIssue::MissingMusicAudio => VerificationError::error(
            "missing_runtime_music_audio",
            "audio",
            "runtime pack must include music audio assets",
        ),
        RuntimePackPresenceIssue::MissingSoundEffects => VerificationError::error(
            "missing_runtime_sound_effects",
            "audio",
            "runtime pack must include sound effect audio assets",
        ),
        RuntimePackPresenceIssue::MissingCryAudio => VerificationError::error(
            "missing_runtime_cry_audio",
            "audio",
            "runtime pack must include cry audio assets",
        ),
        RuntimePackPresenceIssue::MissingPokemonCries => VerificationError::error(
            "missing_runtime_pokemon_cries",
            "pokemon_cries",
            "runtime pack must include Pokemon cry metadata",
        ),
        RuntimePackPresenceIssue::MissingTilesets => VerificationError::error(
            "missing_runtime_tilesets",
            "tilesets",
            "runtime pack must include tileset data",
        ),
        RuntimePackPresenceIssue::MissingScripts => VerificationError::error(
            "missing_runtime_scripts",
            "scripts",
            "runtime pack must include script data",
        ),
        RuntimePackPresenceIssue::MissingMapGeometry => VerificationError::error(
            "missing_runtime_map_geometry",
            "map_geometry",
            "runtime pack must include explicit map geometry data",
        ),
        RuntimePackPresenceIssue::MissingMapObjects => VerificationError::error(
            "missing_runtime_map_objects",
            "map_objects",
            "runtime pack must include explicit map object data",
        ),
        RuntimePackPresenceIssue::MissingRuntimeMapMetadata => VerificationError::error(
            "missing_runtime_map_metadata",
            "runtime_map_metadata",
            "runtime pack must include runtime map metadata",
        ),
        RuntimePackPresenceIssue::MissingRuntimeSpawnPoints => VerificationError::error(
            "missing_runtime_spawn_points",
            "runtime_spawn_points",
            "runtime pack must include runtime spawn points",
        ),
        RuntimePackPresenceIssue::MissingMaps => VerificationError::error(
            "missing_runtime_maps",
            "maps",
            "runtime pack must include map modules",
        ),
        RuntimePackPresenceIssue::MissingPcStrings => VerificationError::error(
            "missing_runtime_pc_strings",
            "pc_strings",
            "runtime pack must include PC string data",
        ),
        RuntimePackPresenceIssue::MissingMenuIcons => VerificationError::error(
            "missing_runtime_menu_icons",
            "menu_icons",
            "runtime pack must include menu icon data",
        ),
        RuntimePackPresenceIssue::MissingPokedexEntries => VerificationError::error(
            "missing_runtime_pokedex_entries",
            "pokedex_entries",
            "runtime pack must include Pokedex entry data",
        ),
        RuntimePackPresenceIssue::MissingPokemonFrontpicAnimations => VerificationError::error(
            "missing_runtime_pokemon_frontpic_animations",
            "pokemon_frontpic_anim",
            "runtime pack must include Pokemon frontpic animation data",
        ),
        RuntimePackPresenceIssue::MissingMoveNames => VerificationError::error(
            "missing_runtime_move_names",
            "move_names",
            "runtime pack must include move names",
        ),
        RuntimePackPresenceIssue::MissingAsmText => VerificationError::error(
            "missing_runtime_asm_text",
            "asm_text",
            "runtime pack must include ASM text data",
        ),
        RuntimePackPresenceIssue::MissingBattleAnimations => VerificationError::error(
            "missing_runtime_battle_animations",
            "battle_animations",
            "runtime pack must include battle animation metadata",
        ),
        RuntimePackPresenceIssue::MissingBattleAnimationTable => VerificationError::error(
            "missing_runtime_battle_animation_table",
            "battle_animation_table",
            "runtime pack must include battle animation table data",
        ),
        RuntimePackPresenceIssue::MissingBattleAnimBundle => VerificationError::error(
            "missing_runtime_battle_anim_bundle",
            "battle_anim_bundle",
            "runtime pack must include battle animation runtime bundle",
        ),
        RuntimePackPresenceIssue::MissingSpriteAnimBundle => VerificationError::error(
            "missing_runtime_sprite_anim_bundle",
            "sprite_anim_bundle",
            "runtime pack must include sprite animation runtime bundle",
        ),
        RuntimePackPresenceIssue::MissingSpritePaletteDefaults => VerificationError::error(
            "missing_runtime_sprite_palette_defaults",
            "sprite_palette_defaults",
            "runtime pack must include sprite palette defaults",
        ),
        RuntimePackPresenceIssue::MissingPokegearTownMapPalettes => VerificationError::error(
            "missing_runtime_pokegear_town_map_palettes",
            "pokegear_town_map_palette_map",
            "runtime pack must include Pokegear town map palette data",
        ),
        RuntimePackPresenceIssue::MissingPokegearLandmarks => VerificationError::error(
            "missing_runtime_pokegear_landmarks",
            "pokegear_landmarks",
            "runtime pack must include Pokegear landmark data",
        ),
        RuntimePackPresenceIssue::MissingPhoneContacts => VerificationError::error(
            "missing_runtime_phone_contacts",
            "phone_contacts",
            "runtime pack must include phone contacts",
        ),
        RuntimePackPresenceIssue::MissingPermanentPhoneNumbers => VerificationError::error(
            "missing_runtime_permanent_phone_numbers",
            "permanent_phone_numbers",
            "runtime pack must include permanent phone number data",
        ),
        RuntimePackPresenceIssue::MissingSpecialPhoneCalls => VerificationError::error(
            "missing_runtime_special_phone_calls",
            "special_phone_calls",
            "runtime pack must include special phone call data",
        ),
        RuntimePackPresenceIssue::MissingPhoneScripts => VerificationError::error(
            "missing_runtime_phone_scripts",
            "phone_scripts",
            "runtime pack must include phone script data",
        ),
        RuntimePackPresenceIssue::MissingFleeMons => VerificationError::error(
            "missing_runtime_flee_mons",
            "flee_mons",
            "runtime pack must include flee-mon tables",
        ),
        RuntimePackPresenceIssue::MissingBuenaPasswordCategories => VerificationError::error(
            "missing_runtime_buena_password_categories",
            "buena_password_categories",
            "runtime pack must include Buena password categories",
        ),
        RuntimePackPresenceIssue::MissingRoamingPokemon => VerificationError::error(
            "missing_runtime_roaming_pokemon",
            "roaming_pokemon",
            "runtime pack must include roaming Pokemon definitions",
        ),
        RuntimePackPresenceIssue::MissingBuenaPrizes => VerificationError::error(
            "missing_runtime_buena_prizes",
            "buena_prizes",
            "runtime pack must include Buena prize definitions",
        ),
        RuntimePackPresenceIssue::MissingKurtApricornRecipes => VerificationError::error(
            "missing_runtime_kurt_apricorn_recipes",
            "kurt_apricorn_recipes",
            "runtime pack must include Kurt apricorn recipes",
        ),
        RuntimePackPresenceIssue::MissingShuckieGift => VerificationError::error(
            "missing_runtime_shuckie_gift",
            "shuckie_gift",
            "runtime pack must include Shuckie gift data",
        ),
        RuntimePackPresenceIssue::MissingDratiniMoveSets => VerificationError::error(
            "missing_runtime_dratini_move_sets",
            "dratini_move_sets",
            "runtime pack must include Dratini move-set data",
        ),
        RuntimePackPresenceIssue::MissingBugContestConfig => VerificationError::error(
            "missing_runtime_bug_contest_config",
            "bug_contest_config",
            "runtime pack must include Bug-Catching Contest config",
        ),
        RuntimePackPresenceIssue::MissingBattleTowerRules => VerificationError::error(
            "missing_runtime_battle_tower_rules",
            "battle_tower_rules",
            "runtime pack must include Battle Tower rules",
        ),
        RuntimePackPresenceIssue::MissingOakRatings => VerificationError::error(
            "missing_runtime_oak_ratings",
            "oak_ratings",
            "runtime pack must include Oak rating data",
        ),
        RuntimePackPresenceIssue::MissingOddEggDefinitions => VerificationError::error(
            "missing_runtime_odd_egg_definitions",
            "odd_egg_definitions",
            "runtime pack must include Odd Egg definitions",
        ),
        RuntimePackPresenceIssue::MissingMagikarpLengths => VerificationError::error(
            "missing_runtime_magikarp_lengths",
            "magikarp_lengths",
            "runtime pack must include Magikarp length table data",
        ),
        RuntimePackPresenceIssue::MissingHappinessData => VerificationError::error(
            "missing_runtime_happiness_data",
            "happiness_data",
            "runtime pack must include happiness data",
        ),
        RuntimePackPresenceIssue::MissingInitializeEvents => VerificationError::error(
            "missing_runtime_initialize_events",
            "initialize_events",
            "runtime pack must include initialize-events data",
        ),
        RuntimePackPresenceIssue::MissingStoryEventScriptConstants => VerificationError::error(
            "missing_runtime_story_event_script_constants",
            "story_event_script_constants",
            "runtime pack must include story event script constants",
        ),
    }
}

fn has_runtime_script_data(data: &GameDataSet) -> bool {
    !data.map_scripts.is_empty()
        || data.maps.values().any(|module| {
            !module.scripts.is_empty()
                || !module.trainer_scripts.is_empty()
                || !module.scripted_trainer_battles.is_empty()
                || !module.scripted_wild_battles.is_empty()
                || !module.script_item_grants.is_empty()
                || !module.script_item_checks.is_empty()
                || !module.script_item_takes.is_empty()
                || !module.script_economy_commands.is_empty()
                || !module.gift_pokemon_scripts.is_empty()
                || !module.script_flag_commands.is_empty()
                || !module.script_scene_commands.is_empty()
                || !module.script_audio_commands.is_empty()
                || !module.script_block_changes.is_empty()
                || !module.script_object_commands.is_empty()
                || !module.script_movements.is_empty()
                || !module.script_map_commands.is_empty()
                || !module.script_text_commands.is_empty()
                || !module.script_text_bodies.is_empty()
                || !module.script_menu_definitions.is_empty()
                || !module.script_variable_commands.is_empty()
                || !module.script_control_commands.is_empty()
                || !module.script_field_pickups.is_empty()
                || !module.script_shop_commands.is_empty()
                || !module.script_phone_commands.is_empty()
                || !module.script_runtime_commands.is_empty()
                || !module.map_script_section_commands.is_empty()
                || !module.map_event_section_commands.is_empty()
        })
}

fn has_runtime_map_geometry(data: &GameDataSet) -> bool {
    data.maps.values().any(|module| {
        module.attributes.width > 0 && module.attributes.height > 0 && !module.blocks.is_empty()
    })
}

fn has_runtime_map_objects(data: &GameDataSet) -> bool {
    data.maps.values().any(|module| !module.objects.is_empty())
}

fn has_capture_rules(data: &GameDataSet) -> bool {
    !data.capture_rules.ball_rules.is_empty()
}

fn has_capture_wobble_probabilities(data: &GameDataSet) -> bool {
    !data.capture_wobble_probabilities.is_empty()
}

fn has_battle_stat_multipliers(data: &GameDataSet) -> bool {
    !data.battle_stat_multipliers.stat.is_empty()
        && !data.battle_stat_multipliers.accuracy.is_empty()
}

fn has_move_priorities(data: &GameDataSet) -> bool {
    !data.move_priorities.effect_priorities.is_empty()
}

fn has_type_categories(data: &GameDataSet) -> bool {
    !data.type_categories.physical.is_empty() && !data.type_categories.special.is_empty()
}

fn has_type_effectiveness(data: &GameDataSet) -> bool {
    !data.type_effectiveness.matchups.is_empty()
        && !data.type_effectiveness.foresight_matchups.is_empty()
}

fn has_weather_modifiers(data: &GameDataSet) -> bool {
    !data.weather_modifiers.type_modifiers.is_empty()
        && !data.weather_modifiers.move_effect_modifiers.is_empty()
}

fn has_battle_reward_rules(data: &GameDataSet) -> bool {
    data.battle_reward_rules != BattleRewardRules::default()
}

fn has_battle_escape_rules(data: &GameDataSet) -> bool {
    data.battle_escape_rules != BattleEscapeRules::default()
}

fn has_marts(data: &GameDataSet) -> bool {
    !data.marts.0.is_empty()
}

fn has_currency_constants(data: &GameDataSet) -> bool {
    !data.currency_constants.0.is_empty()
}

fn has_step_event_rules(data: &GameDataSet) -> bool {
    data.step_event_rules != StepEventRules::default()
}

fn has_fishing_catalog(data: &GameDataSet) -> bool {
    !data.fishing.groups.is_empty() && !data.fishing.rod_items.is_empty()
}

fn has_fruit_trees(data: &GameDataSet) -> bool {
    !data.fruit_trees.0.is_empty()
}

fn has_field_moves(data: &GameDataSet) -> bool {
    data.field_moves != FieldMoveCatalog::default()
}

fn has_runtime_title_screen(data: &GameDataSet) -> bool {
    data.runtime_title_screen != RuntimeTitleScreen::default()
}

fn has_pc_strings(data: &GameDataSet) -> bool {
    !data.pc_strings.is_empty()
}

fn has_menu_icons(data: &GameDataSet) -> bool {
    !data.menu_icons.is_empty()
}

fn has_pokedex_entries(data: &GameDataSet) -> bool {
    !data.pokedex_entries.is_empty()
}

fn has_pokemon_frontpic_animations(data: &GameDataSet) -> bool {
    !data.pokemon_frontpic_anim.is_empty()
}

fn has_move_names(data: &GameDataSet) -> bool {
    !data.move_names.is_empty()
}

fn has_asm_text(data: &GameDataSet) -> bool {
    !data.asm_text.is_empty()
}

fn has_battle_animations(data: &GameDataSet) -> bool {
    !data.battle_animations.is_empty()
}

fn has_battle_animation_table(data: &GameDataSet) -> bool {
    !data.battle_animation_table.is_empty()
}

fn has_battle_anim_bundle(data: &GameDataSet) -> bool {
    !data.battle_anim_bundle.trim().is_empty()
}

fn has_sprite_anim_bundle(data: &GameDataSet) -> bool {
    !data.sprite_anim_bundle.trim().is_empty()
}

fn has_sprite_palette_defaults(data: &GameDataSet) -> bool {
    !data.sprite_palette_defaults.is_empty()
}

fn has_pokegear_town_map_palettes(data: &GameDataSet) -> bool {
    !data.pokegear_town_map_palette_map.is_empty()
}

fn has_pokegear_landmarks(data: &GameDataSet) -> bool {
    !data.pokegear_landmarks.landmarks.is_empty()
        && !data.pokegear_landmarks.map_to_landmark.is_empty()
}

fn has_phone_contacts(data: &GameDataSet) -> bool {
    !data.phone_contacts.0.is_empty()
}

fn has_permanent_phone_numbers(data: &GameDataSet) -> bool {
    !data.permanent_phone_numbers.is_empty()
}

fn has_special_phone_calls(data: &GameDataSet) -> bool {
    !data.special_phone_calls.is_empty()
}

fn has_phone_scripts(data: &GameDataSet) -> bool {
    !data.phone_scripts.is_empty()
}

fn has_flee_mons(data: &GameDataSet) -> bool {
    !data.flee_mons.is_empty()
}

fn has_buena_password_categories(data: &GameDataSet) -> bool {
    !data.buena_password_categories.categories.is_empty()
        && !data.buena_password_categories.order.is_empty()
}

fn has_roaming_pokemon(data: &GameDataSet) -> bool {
    !data.roaming_pokemon.is_empty()
}

fn has_buena_prizes(data: &GameDataSet) -> bool {
    !data.buena_prizes.is_empty()
}

fn has_kurt_apricorn_recipes(data: &GameDataSet) -> bool {
    !data.kurt_apricorn_recipes.is_empty()
}

fn has_shuckie_gift(data: &GameDataSet) -> bool {
    data.shuckie_gift.is_some()
}

fn has_dratini_move_sets(data: &GameDataSet) -> bool {
    !data.dratini_move_sets.is_empty()
}

fn has_bug_contest_config(data: &GameDataSet) -> bool {
    data.bug_contest_config.is_some()
}

fn has_battle_tower_rules(data: &GameDataSet) -> bool {
    data.battle_tower_rules.is_some()
}

fn has_oak_ratings(data: &GameDataSet) -> bool {
    !data.oak_ratings.is_empty()
}

fn has_odd_egg_definitions(data: &GameDataSet) -> bool {
    !data.odd_egg_definitions.is_empty()
}

fn has_magikarp_lengths(data: &GameDataSet) -> bool {
    !data.magikarp_lengths.is_empty()
}

fn has_happiness_data(data: &GameDataSet) -> bool {
    data.happiness_data.is_some()
}

fn has_initialize_events(data: &GameDataSet) -> bool {
    data.initialize_events != InitializeEventsConfig::default()
}

fn has_story_event_script_constants(data: &GameDataSet) -> bool {
    data.story_event_script_constants != StoryEventScriptConstants::default()
}

fn asm_text_catalog_issue_diagnostic(issue: AsmTextCatalogIssue) -> VerificationError {
    match issue {
        AsmTextCatalogIssue::InvalidText { label } => VerificationError::error(
            "invalid_asm_text",
            &label,
            "ASM text keys and values must be non-empty",
        ),
    }
}

fn pokegear_town_map_palette_issue_diagnostic(
    issue: PokegearTownMapPaletteIssue,
) -> VerificationError {
    match issue {
        PokegearTownMapPaletteIssue::InvalidEntry { map_name } => VerificationError::error(
            "invalid_pokegear_palette_map",
            &map_name,
            "Pokegear palette map entries must be exact non-empty values",
        ),
    }
}

fn pokegear_landmark_issue_diagnostic(issue: PokegearLandmarkIssue) -> VerificationError {
    match issue {
        PokegearLandmarkIssue::InvalidLandmark { constant } => VerificationError::error(
            "invalid_pokegear_landmark",
            &constant,
            "Pokegear landmarks require non-empty constant, label, name, and region fields",
        ),
        PokegearLandmarkIssue::InvalidConstant { constant } => VerificationError::error(
            "invalid_pokegear_landmark_constant",
            &constant,
            "Pokegear landmark constants must use exact LANDMARK_* ids",
        ),
        PokegearLandmarkIssue::InvalidMapEntry { map_name } => VerificationError::error(
            "invalid_pokegear_landmark_map",
            &map_name,
            "Pokegear map-to-landmark map keys must be exact non-empty values",
        ),
        PokegearLandmarkIssue::InvalidLandmarkReference {
            map_name,
            landmark_constant,
        } => VerificationError::error(
            "invalid_pokegear_landmark_reference",
            &map_name,
            format!(
                "Pokegear map-to-landmark entry has invalid landmark constant '{landmark_constant}'"
            ),
        ),
        PokegearLandmarkIssue::UnknownMap { map_name } => VerificationError::error(
            "unknown_pokegear_landmark_map",
            &map_name,
            "Pokegear map-to-landmark entry references a map that is not loaded",
        ),
        PokegearLandmarkIssue::UnknownLandmarkConstant {
            map_name,
            landmark_constant,
        } => VerificationError::error(
            "unknown_pokegear_landmark_constant",
            &map_name,
            format!(
                "Pokegear map-to-landmark entry references missing landmark constant '{landmark_constant}'"
            ),
        ),
    }
}

fn verify_unique_pokegear_landmark_constants(
    payload: &PokegearLandmarksPayload,
    diagnostics: &mut Vec<VerificationError>,
) {
    let mut constants = BTreeSet::new();
    for landmark in &payload.landmarks {
        if is_valid_pokegear_landmark_constant(&landmark.constant)
            && !constants.insert(landmark.constant.as_str())
        {
            diagnostics.push(VerificationError::error(
                "duplicate_pokegear_landmark_constant",
                &landmark.constant,
                "Pokegear landmark constants must be unique for runtime lookup",
            ));
        }
    }
}

fn is_valid_pokegear_landmark_constant(value: &str) -> bool {
    is_exact_pack_token(value) && value.starts_with("LANDMARK_")
}

fn pc_string_catalog_issue_diagnostic(issue: PcStringCatalogIssue) -> VerificationError {
    match issue {
        PcStringCatalogIssue::InvalidString { key } => VerificationError::error(
            "invalid_pc_string",
            &key,
            "PC string keys and values must be exact non-empty values",
        ),
    }
}

fn flee_mon_catalog_issue_diagnostic(issue: FleeMonCatalogIssue) -> VerificationError {
    match issue {
        FleeMonCatalogIssue::InvalidBucketId { bucket_id } => VerificationError::error(
            "invalid_flee_mon_bucket",
            &bucket_id,
            "flee mon bucket ids must be exact lowercase modpack tokens",
        ),
        FleeMonCatalogIssue::EmptyBucket { bucket_id } => VerificationError::error(
            "empty_flee_mon_bucket",
            &bucket_id,
            "flee mon buckets must declare at least one species",
        ),
        FleeMonCatalogIssue::InvalidSpeciesId { species_id } => VerificationError::error(
            "invalid_flee_mon_species",
            &species_id,
            "flee mon table species ids must be exact non-empty values",
        ),
        FleeMonCatalogIssue::UnknownSpecies { species_id } => VerificationError::error(
            "unknown_flee_mon_species",
            &species_id,
            format!("flee mon table references missing species '{species_id}'"),
        ),
    }
}

fn menu_icon_catalog_issue_diagnostic(issue: MenuIconCatalogIssue) -> VerificationError {
    match issue {
        MenuIconCatalogIssue::InvalidSpeciesId { species_id } => VerificationError::error(
            "invalid_menu_icon_species",
            &species_id,
            "menu icon species ids must be exact non-empty values",
        ),
        MenuIconCatalogIssue::UnknownSpecies { species_id } => VerificationError::error(
            "unknown_menu_icon_species",
            &species_id,
            format!("menu icon references missing species '{species_id}'"),
        ),
        MenuIconCatalogIssue::InvalidIcon { species_id } => VerificationError::error(
            "invalid_menu_icon",
            &species_id,
            "menu icon id must be exact non-empty value",
        ),
        MenuIconCatalogIssue::MissingSpeciesIcon { species_id } => VerificationError::error(
            "missing_species_menu_icon",
            &species_id,
            "Pokemon species is missing an explicit menu icon entry",
        ),
    }
}

fn pokedex_entry_catalog_issue_diagnostic(issue: PokedexEntryCatalogIssue) -> VerificationError {
    match issue {
        PokedexEntryCatalogIssue::InvalidSpeciesId { species_id } => VerificationError::error(
            "invalid_pokedex_entry_species",
            &species_id,
            "pokedex entry species ids must be exact non-empty values",
        ),
        PokedexEntryCatalogIssue::SpeciesMismatch {
            species_id,
            record_species,
        } => VerificationError::error(
            "pokedex_entry_species_mismatch",
            &species_id,
            format!(
                "pokedex entry key '{}' does not match record species '{}'",
                species_id, record_species
            ),
        ),
        PokedexEntryCatalogIssue::UnknownSpecies { species_id } => VerificationError::error(
            "unknown_pokedex_entry_species",
            &species_id,
            format!("pokedex entry references missing species '{species_id}'"),
        ),
        PokedexEntryCatalogIssue::InvalidEntry { species_id } => VerificationError::error(
            "invalid_pokedex_entry",
            &species_id,
            "pokedex entry classification and pages must be non-empty",
        ),
        PokedexEntryCatalogIssue::MissingSpeciesEntry { species_id } => VerificationError::error(
            "missing_species_pokedex_entry",
            &species_id,
            "Pokemon species is missing an explicit Pokedex entry",
        ),
    }
}

fn frontpic_anim_catalog_issue_diagnostic(issue: FrontpicAnimCatalogIssue) -> VerificationError {
    match issue {
        FrontpicAnimCatalogIssue::InvalidSpeciesId { species_id } => VerificationError::error(
            "invalid_frontpic_anim_species",
            &species_id,
            "frontpic animation species ids must be exact non-empty values",
        ),
        FrontpicAnimCatalogIssue::UnknownSpecies { species_id } => VerificationError::error(
            "unknown_frontpic_anim_species",
            &species_id,
            format!("frontpic animation references missing species '{species_id}'"),
        ),
        FrontpicAnimCatalogIssue::EmptyProgram { species_id } => VerificationError::error(
            "empty_frontpic_anim",
            &species_id,
            "frontpic animation program must contain at least one command",
        ),
        FrontpicAnimCatalogIssue::Command {
            species_id,
            index,
            command,
            issue,
        } => {
            let subject = format!("{species_id}:{index}");
            match issue {
                FrontpicAnimCommandIssue::MissingFrame => VerificationError::error(
                    "malformed_frontpic_anim_command",
                    subject,
                    "frame command requires frame and duration",
                ),
                FrontpicAnimCommandIssue::MissingSetRepeatCount => VerificationError::error(
                    "malformed_frontpic_anim_command",
                    subject,
                    "setrepeat command requires count",
                ),
                FrontpicAnimCommandIssue::MissingDoRepeatTarget => VerificationError::error(
                    "malformed_frontpic_anim_command",
                    subject,
                    "dorepeat command requires target",
                ),
                FrontpicAnimCommandIssue::InvalidDoRepeatTarget => VerificationError::error(
                    "malformed_frontpic_anim_command",
                    subject,
                    "dorepeat command target must point to a previous command",
                ),
                FrontpicAnimCommandIssue::UnknownCommand => VerificationError::error(
                    "unknown_frontpic_anim_command",
                    subject,
                    format!("unknown frontpic animation command '{command}'"),
                ),
            }
        }
        FrontpicAnimCatalogIssue::MissingSpeciesProgram { species_id } => VerificationError::error(
            "missing_species_frontpic_anim",
            &species_id,
            "Pokemon species is missing an explicit frontpic animation program",
        ),
    }
}

fn verify_script_shop_commands(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    for (map_name, module) in &data.maps {
        verify_unique_script_command_positions(
            map_name,
            "script_shop_commands",
            module
                .script_shop_commands
                .iter()
                .map(|command| (command.source_script.as_str(), command.command_index)),
            diagnostics,
        );
        diagnostics.extend(
            script_shop_command_issues(&data.marts, &module.script_shop_commands)
                .into_iter()
                .map(|issue| script_shop_command_issue_diagnostic(map_name, issue)),
        );
    }
}

fn script_shop_command_issue_diagnostic(
    map_name: &str,
    issue: ScriptShopCommandIssue,
) -> VerificationError {
    let subject = format!("{map_name}:{}:{}", issue.source_script, issue.command_index);
    match issue.error {
        ShopError::UnknownMartType { mart_type } => VerificationError::error(
            "unknown_script_shop_mart_type",
            subject,
            format!("pokemart uses unknown mart type '{mart_type}'"),
        ),
        ShopError::InvalidCommand { command } => VerificationError::error(
            "invalid_script_shop_command",
            subject,
            format!("script shop command must be an exact lowercase pack token, found {command:?}"),
        ),
        ShopError::UnknownCommand { command } => VerificationError::error(
            "unknown_script_shop_command",
            subject,
            format!("script shop command '{command}' is not supported"),
        ),
        ShopError::InvalidMartType { mart_type } => VerificationError::error(
            "invalid_script_shop_mart_type",
            subject,
            format!("pokemart mart type must be exact and nonempty, found {mart_type:?}"),
        ),
        ShopError::InvalidZeroMart { mart_type } => VerificationError::error(
            "script_shop_invalid_zero_mart",
            subject,
            format!("pokemart type '{mart_type}' cannot use explicit mart id 0"),
        ),
        ShopError::InvalidMartId { mart_id } => VerificationError::error(
            "invalid_script_shop_mart",
            subject,
            format!("pokemart mart id must be exact and nonempty, found {mart_id:?}"),
        ),
        ShopError::UnknownMart { mart_id } => VerificationError::error(
            "unknown_script_shop_mart",
            subject,
            format!("pokemart references missing mart '{mart_id}'"),
        ),
        _ => unreachable!("script_shop_command_issues only returns verifier shop errors"),
    }
}

fn verify_script_phone_commands(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    for (map_name, module) in &data.maps {
        verify_unique_script_command_positions(
            map_name,
            "script_phone_commands",
            module
                .script_phone_commands
                .iter()
                .map(|command| (command.source_script.as_str(), command.command_index)),
            diagnostics,
        );
        diagnostics.extend(
            script_phone_command_issues(&module.script_phone_commands, &data.phone_contacts)
                .into_iter()
                .map(|issue| script_phone_command_issue_diagnostic(map_name, issue)),
        );
    }
}

fn script_phone_command_issue_diagnostic(
    map_name: &str,
    issue: ScriptPhoneCommandIssue,
) -> VerificationError {
    let subject = format!("{map_name}:{}:{}", issue.source_script, issue.command_index);
    match issue.error {
        ScriptPhoneError::InvalidCommand { command } => VerificationError::error(
            "invalid_script_phone_command",
            subject,
            format!("phone command must be an exact lowercase pack token, found {command:?}"),
        ),
        ScriptPhoneError::UnknownCommand { command } => VerificationError::error(
            "unknown_script_phone_command",
            subject,
            format!("unknown phone command '{command}'"),
        ),
        ScriptPhoneError::UnknownContact {
            command,
            contact_id,
        } => VerificationError::error(
            "unknown_script_phone_contact",
            subject,
            format!("phone command '{command}' references unknown contact '{contact_id}'"),
        ),
        ScriptPhoneError::EmptyContact { command } => VerificationError::error(
            "invalid_script_phone_contact",
            subject,
            format!(
                "phone command '{command}' contact id must be an exact non-empty pack token, found {:?}",
                issue.contact_id
            ),
        ),
        ScriptPhoneError::PaddedContact {
            command,
            contact_id,
        } => VerificationError::error(
            "invalid_script_phone_contact",
            subject,
            format!(
                "phone command '{command}' contact id must be exact and untrimmed, found {contact_id:?}"
            ),
        ),
        _ => unreachable!("script_phone_command_issues only returns verifier phone errors"),
    }
}

fn verify_happiness_data(data: &HappinessData, diagnostics: &mut Vec<VerificationError>) {
    diagnostics.extend(
        happiness_data_issues(data)
            .into_iter()
            .map(happiness_data_issue_diagnostic),
    );
}

fn happiness_data_issue_diagnostic(issue: HappinessDataIssue) -> VerificationError {
    match issue {
        HappinessDataIssue::EmptyChanges => VerificationError::error(
            "empty_happiness_changes",
            "happiness_data:changes",
            "happiness data requires at least one explicit change row",
        ),
        HappinessDataIssue::EmptyChangeCode { change_code } => VerificationError::error(
            "empty_happiness_change_code",
            format!("happiness_data:changes:{change_code}"),
            "happiness change entries require exact nonempty code labels",
        ),
        HappinessDataIssue::InvalidChangeCode { code, change_code } => VerificationError::error(
            "invalid_happiness_change_code",
            format!("happiness_data:changes:{change_code}"),
            format!("happiness change code '{code}' must be an exact pack token"),
        ),
        HappinessDataIssue::DuplicateChangeCode { code, change_code } => VerificationError::error(
            "duplicate_happiness_change_code",
            format!("happiness_data:changes:{change_code}"),
            format!("duplicate happiness change code '{code}'"),
        ),
        HappinessDataIssue::EmptyServices => VerificationError::error(
            "empty_happiness_services",
            "happiness_data:services",
            "happiness data requires explicit service probability tables",
        ),
        HappinessDataIssue::EmptyServiceRoutine { routine } => VerificationError::error(
            "empty_happiness_service_routine",
            format!("happiness_data:services:{routine}"),
            "happiness service routine ids must be exact nonempty labels",
        ),
        HappinessDataIssue::InvalidServiceRoutine { routine } => VerificationError::error(
            "invalid_happiness_service_routine",
            format!("happiness_data:services:{routine}"),
            format!("happiness service routine '{routine}' must be an exact pack token"),
        ),
        HappinessDataIssue::EmptyServiceOutcomes { routine } => VerificationError::error(
            "empty_happiness_service_outcomes",
            format!("happiness_data:services:{routine}"),
            "happiness service tables require at least one outcome",
        ),
        HappinessDataIssue::UnknownServiceChange {
            routine,
            change_code,
        } => VerificationError::error(
            "unknown_happiness_service_change",
            format!("happiness_data:services:{routine}"),
            format!("happiness service outcome references missing change code {change_code}"),
        ),
    }
}

fn verify_encounter_slot_tables(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    diagnostics.extend(
        encounter_slot_table_issues(
            &data.encounter_slot_tables,
            !data.wild_encounters.is_empty(),
        )
        .into_iter()
        .map(encounter_slot_table_issue_diagnostic),
    );
    verify_wild_encounter_slot_coverage(data, diagnostics);
}

fn verify_wild_encounter_slot_coverage(
    data: &GameDataSet,
    diagnostics: &mut Vec<VerificationError>,
) {
    let max_grass_slot = max_encounter_slot(&data.encounter_slot_tables, EncounterSurface::Grass);
    let max_water_slot = max_encounter_slot(&data.encounter_slot_tables, EncounterSurface::Water);
    for (map_name, encounters) in &data.wild_encounters {
        if let (Some(max_slot), Some(grass), Some(rates)) = (
            max_grass_slot,
            encounters.grass.as_ref(),
            encounters.grass_rates.as_ref(),
        ) {
            for (time_key, slots) in [
                ("morning", grass.morning.as_slice()),
                ("day", grass.day.as_slice()),
                ("night", grass.night.as_slice()),
            ] {
                if rates.get(time_key).is_some_and(|rate| *rate > 0) && max_slot >= slots.len() {
                    diagnostics.push(unresolved_encounter_slot_index_diagnostic(
                        map_name,
                        "grass",
                        time_key,
                        max_slot,
                        slots.len(),
                    ));
                }
            }
        }
        if let (Some(max_slot), Some(water), Some(rate)) = (
            max_water_slot,
            encounters.water.as_ref(),
            encounters.water_rate,
        ) {
            if rate > 0 {
                for (time_key, slots) in [
                    ("morning", water.morning.as_slice()),
                    ("day", water.day.as_slice()),
                    ("night", water.night.as_slice()),
                ] {
                    if max_slot >= slots.len() {
                        diagnostics.push(unresolved_encounter_slot_index_diagnostic(
                            map_name,
                            "water",
                            time_key,
                            max_slot,
                            slots.len(),
                        ));
                    }
                }
            }
        }
    }
}

fn max_encounter_slot(tables: &EncounterSlotTables, surface: EncounterSurface) -> Option<usize> {
    tables
        .tables
        .get(surface.as_key())
        .and_then(|table| table.iter().map(|entry| entry.slot).max())
}

fn unresolved_encounter_slot_index_diagnostic(
    map_name: &str,
    surface: &str,
    time_key: &str,
    max_slot: usize,
    slot_count: usize,
) -> VerificationError {
    VerificationError::error(
        "unresolved_encounter_slot_index",
        format!("{map_name}:{surface}:{time_key}"),
        format!(
            "{surface} encounter slot table references slot {max_slot}, but {map_name} has {slot_count} {surface} slots for {time_key}"
        ),
    )
}

fn encounter_slot_table_issue_diagnostic(issue: EncounterSlotTableIssue) -> VerificationError {
    match issue {
        EncounterSlotTableIssue::InvalidSurfaceId { surface_id } => VerificationError::error(
            "invalid_encounter_slot_surface",
            format!("encounter_slot_tables:{surface_id}"),
            format!("encounter slot surface id {surface_id} must be an exact modpack token"),
        ),
        EncounterSlotTableIssue::MissingTable { surface } => {
            let surface = encounter_surface_label(surface);
            VerificationError::error(
                "missing_encounter_slot_table",
                format!("encounter_slot_tables:{surface}"),
                format!("encounter slot table for {surface} must be declared by the modpack"),
            )
        }
        EncounterSlotTableIssue::InvalidThreshold { surface, threshold } => {
            let surface = encounter_surface_label(surface);
            VerificationError::error(
                "invalid_encounter_slot_threshold",
                format!("encounter_slot_tables:{surface}"),
                format!(
                    "encounter slot table for {surface} has threshold {threshold} outside 1..=100"
                ),
            )
        }
        EncounterSlotTableIssue::UnorderedThreshold {
            surface,
            threshold,
            previous,
        } => {
            let surface = encounter_surface_label(surface);
            VerificationError::error(
                "unordered_encounter_slot_threshold",
                format!("encounter_slot_tables:{surface}"),
                format!(
                    "encounter slot table for {surface} has threshold {threshold} after {previous}"
                ),
            )
        }
        EncounterSlotTableIssue::DuplicateSlotIndex { surface, slot } => {
            let surface = encounter_surface_label(surface);
            VerificationError::error(
                "duplicate_encounter_slot_index",
                format!("encounter_slot_tables:{surface}"),
                format!("encounter slot table for {surface} repeats slot {slot}"),
            )
        }
        EncounterSlotTableIssue::IncompleteTable { surface } => {
            let surface = encounter_surface_label(surface);
            VerificationError::error(
                "incomplete_encounter_slot_table",
                format!("encounter_slot_tables:{surface}"),
                format!("encounter slot table for {surface} must end at threshold 100"),
            )
        }
        EncounterSlotTableIssue::InvalidCustomThreshold {
            surface_id,
            threshold,
        } => VerificationError::error(
            "invalid_encounter_slot_threshold",
            format!("encounter_slot_tables:{surface_id}"),
            format!(
                "encounter slot table for {surface_id} has threshold {threshold} outside 1..=100"
            ),
        ),
        EncounterSlotTableIssue::UnorderedCustomThreshold {
            surface_id,
            threshold,
            previous,
        } => VerificationError::error(
            "unordered_encounter_slot_threshold",
            format!("encounter_slot_tables:{surface_id}"),
            format!(
                "encounter slot table for {surface_id} has threshold {threshold} after {previous}"
            ),
        ),
        EncounterSlotTableIssue::DuplicateCustomSlotIndex { surface_id, slot } => {
            VerificationError::error(
                "duplicate_encounter_slot_index",
                format!("encounter_slot_tables:{surface_id}"),
                format!("encounter slot table for {surface_id} repeats slot {slot}"),
            )
        }
        EncounterSlotTableIssue::EmptyCustomTable { surface_id } => VerificationError::error(
            "missing_encounter_slot_table",
            format!("encounter_slot_tables:{surface_id}"),
            format!("encounter slot table for {surface_id} must not be empty"),
        ),
        EncounterSlotTableIssue::IncompleteCustomTable { surface_id } => VerificationError::error(
            "incomplete_encounter_slot_table",
            format!("encounter_slot_tables:{surface_id}"),
            format!("encounter slot table for {surface_id} must end at threshold 100"),
        ),
    }
}

fn encounter_surface_label(surface: EncounterSurface) -> &'static str {
    match surface {
        EncounterSurface::Grass => "grass",
        EncounterSurface::Water => "water",
        EncounterSurface::Rock => "rock",
    }
}

fn verify_encounter_music_modifiers(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    let (music, _, _) = script_audio_catalog_ids(data);
    diagnostics.extend(
        encounter_music_modifier_issues(
            &data.encounter_music_modifiers,
            &music,
            !data.wild_encounters.is_empty(),
        )
        .into_iter()
        .map(encounter_music_modifier_issue_diagnostic),
    );
}

fn encounter_music_modifier_issue_diagnostic(
    issue: EncounterMusicModifierIssue,
) -> VerificationError {
    match issue {
        EncounterMusicModifierIssue::MissingTable => VerificationError::error(
            "missing_encounter_music_modifiers",
            "encounter_music_modifiers",
            "encounter music modifiers must be declared by the modpack",
        ),
        EncounterMusicModifierIssue::MissingMusicId { music_id } => VerificationError::error(
            "missing_encounter_music_modifier_id",
            format!("encounter_music_modifiers:{music_id}"),
            "encounter music modifier is missing music_id",
        ),
        EncounterMusicModifierIssue::InvalidMusicId { music_id } => VerificationError::error(
            "invalid_encounter_music_modifier_id",
            format!("encounter_music_modifiers:{music_id}"),
            format!("encounter music modifier id must be an exact pack token, found {music_id:?}"),
        ),
        EncounterMusicModifierIssue::UnknownMusicId { music_id } => VerificationError::error(
            "unknown_encounter_music_modifier_id",
            format!("encounter_music_modifiers:{music_id}"),
            format!("encounter music modifier references missing music audio id '{music_id}'"),
        ),
        EncounterMusicModifierIssue::InvalidRatio { music_id } => VerificationError::error(
            "invalid_encounter_music_modifier_ratio",
            format!("encounter_music_modifiers:{music_id}"),
            "encounter music modifier denominator must be greater than zero",
        ),
    }
}

fn verify_battle_stat_multipliers(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    diagnostics.extend(
        battle_stat_multiplier_table_issues(&data.battle_stat_multipliers, !data.moves.is_empty())
            .into_iter()
            .map(battle_stat_multiplier_table_issue_diagnostic),
    );
}

fn battle_stat_multiplier_table_issue_diagnostic(
    issue: BattleStatMultiplierTableIssue,
) -> VerificationError {
    let table_name = issue.table().as_str();
    let subject = format!("battle_stat_multipliers:{table_name}");
    match issue {
        BattleStatMultiplierTableIssue::InvalidLength { actual, .. } => VerificationError::error(
            "invalid_battle_stat_multiplier_table_length",
            subject,
            format!(
                "battle stat multiplier table {table_name} must declare exactly 13 rows for stages -6..=6, found {actual}"
            ),
        ),
        BattleStatMultiplierTableIssue::InvalidNumerator {
            stage, numerator, ..
        } => VerificationError::error(
            "invalid_battle_stat_multiplier_numerator",
            subject,
            format!(
                "battle stat multiplier table {table_name} stage {stage} has nonpositive numerator {numerator}"
            ),
        ),
        BattleStatMultiplierTableIssue::InvalidDenominator {
            stage, denominator, ..
        } => VerificationError::error(
            "invalid_battle_stat_multiplier_denominator",
            subject,
            format!(
                "battle stat multiplier table {table_name} stage {stage} has nonpositive denominator {denominator}"
            ),
        ),
    }
}

fn verify_weather_modifiers(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    diagnostics.extend(
        weather_modifier_issues(&data.weather_modifiers, &data.moves, !data.moves.is_empty())
            .into_iter()
            .map(weather_modifier_issue_diagnostic),
    );
}

fn weather_modifier_issue_diagnostic(issue: WeatherModifierIssue) -> VerificationError {
    match issue {
        WeatherModifierIssue::MissingTypeModifiers => VerificationError::error(
            "missing_weather_type_modifiers",
            "weather_modifiers:type_modifiers",
            "weather type modifiers must be declared when moves exist",
        ),
        WeatherModifierIssue::MissingMoveEffectModifiers => VerificationError::error(
            "missing_weather_move_effect_modifiers",
            "weather_modifiers:move_effect_modifiers",
            "weather move-effect modifiers must be declared when moves exist",
        ),
        WeatherModifierIssue::InvalidWeather { table, weather } => VerificationError::error(
            "invalid_weather_modifier_weather",
            table.subject(),
            format!("weather modifier has invalid weather id {weather:?}"),
        ),
        WeatherModifierIssue::InvalidMoveType { move_type } => VerificationError::error(
            "invalid_weather_modifier_move_type",
            "weather_modifiers:type_modifiers",
            format!("weather type modifier has invalid move type {move_type:?}"),
        ),
        WeatherModifierIssue::InvalidMoveEffect { move_effect } => VerificationError::error(
            "invalid_weather_modifier_move_effect",
            "weather_modifiers:move_effect_modifiers",
            format!("weather move-effect modifier has invalid move effect {move_effect:?}"),
        ),
        WeatherModifierIssue::UnknownMoveEffect { move_effect } => VerificationError::error(
            "unknown_weather_modifier_move_effect",
            "weather_modifiers:move_effect_modifiers",
            format!("weather move-effect modifier references missing move effect '{move_effect}'"),
        ),
        WeatherModifierIssue::InvalidMultiplierDenominator { table } => VerificationError::error(
            "invalid_type_multiplier_denominator",
            table.subject(),
            "type multiplier denominator must be nonzero",
        ),
    }
}

fn verify_type_effectiveness(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    diagnostics.extend(
        type_effectiveness_table_issues(
            &data.type_effectiveness,
            &data.type_categories,
            !data.moves.is_empty(),
        )
        .into_iter()
        .map(type_effectiveness_table_issue_diagnostic),
    );
}

fn type_effectiveness_table_issue_diagnostic(
    issue: TypeEffectivenessTableIssue,
) -> VerificationError {
    match issue {
        TypeEffectivenessTableIssue::MissingMatchups => VerificationError::error(
            "missing_type_effectiveness_matchups",
            "type_effectiveness:matchups",
            "type effectiveness matchups must be declared when moves exist",
        ),
        TypeEffectivenessTableIssue::MissingForesightMatchups => VerificationError::error(
            "missing_type_effectiveness_foresight_matchups",
            "type_effectiveness:foresight_matchups",
            "Foresight type effectiveness matchups must be declared when moves exist",
        ),
        TypeEffectivenessTableIssue::InvalidMultiplierDenominator { table } => {
            VerificationError::error(
                "invalid_type_multiplier_denominator",
                table.subject(),
                "type multiplier denominator must be nonzero",
            )
        }
        TypeEffectivenessTableIssue::InvalidAttacker { table, attacker } => {
            let (code, prefix) = type_effectiveness_invalid_parts(table, true);
            VerificationError::error(
                code,
                table.subject(),
                format!("{prefix} attacker {attacker:?} must be an exact pack token"),
            )
        }
        TypeEffectivenessTableIssue::InvalidDefender { table, defender } => {
            let (code, prefix) = type_effectiveness_invalid_parts(table, false);
            VerificationError::error(
                code,
                table.subject(),
                format!("{prefix} defender {defender:?} must be an exact pack token"),
            )
        }
        TypeEffectivenessTableIssue::UnknownAttacker { table, attacker } => {
            let (code, prefix) = type_effectiveness_unknown_parts(table, true);
            VerificationError::error(
                code,
                table.subject(),
                format!("{prefix} attacker {attacker:?} is not declared in type categories"),
            )
        }
        TypeEffectivenessTableIssue::UnknownDefender { table, defender } => {
            let (code, prefix) = type_effectiveness_unknown_parts(table, false);
            VerificationError::error(
                code,
                table.subject(),
                format!("{prefix} defender {defender:?} is not declared in type categories"),
            )
        }
        TypeEffectivenessTableIssue::MissingMatchup { attacker, defender } => {
            VerificationError::error(
                "missing_type_effectiveness_matchup",
                "type_effectiveness:matchups",
                format!(
                    "type effectiveness matchup {attacker:?} -> {defender:?} must be declared explicitly"
                ),
            )
        }
    }
}

fn type_effectiveness_invalid_parts(
    table: TypeEffectivenessTableKind,
    attacker: bool,
) -> (&'static str, &'static str) {
    match (table, attacker) {
        (TypeEffectivenessTableKind::Matchups, true) => {
            ("invalid_type_effectiveness_attacker", "type effectiveness")
        }
        (TypeEffectivenessTableKind::Matchups, false) => {
            ("invalid_type_effectiveness_defender", "type effectiveness")
        }
        (TypeEffectivenessTableKind::ForesightMatchups, true) => (
            "invalid_foresight_type_effectiveness_attacker",
            "Foresight type effectiveness",
        ),
        (TypeEffectivenessTableKind::ForesightMatchups, false) => (
            "invalid_foresight_type_effectiveness_defender",
            "Foresight type effectiveness",
        ),
    }
}

fn type_effectiveness_unknown_parts(
    table: TypeEffectivenessTableKind,
    attacker: bool,
) -> (&'static str, &'static str) {
    match (table, attacker) {
        (TypeEffectivenessTableKind::Matchups, true) => {
            ("unknown_type_effectiveness_attacker", "type effectiveness")
        }
        (TypeEffectivenessTableKind::Matchups, false) => {
            ("unknown_type_effectiveness_defender", "type effectiveness")
        }
        (TypeEffectivenessTableKind::ForesightMatchups, true) => (
            "unknown_foresight_type_effectiveness_attacker",
            "Foresight type effectiveness",
        ),
        (TypeEffectivenessTableKind::ForesightMatchups, false) => (
            "unknown_foresight_type_effectiveness_defender",
            "Foresight type effectiveness",
        ),
    }
}

fn verify_type_categories(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    diagnostics.extend(
        type_category_issues(&data.type_categories, !data.moves.is_empty())
            .into_iter()
            .map(type_category_issue_diagnostic),
    );
}

fn verify_move_priorities(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    diagnostics.extend(
        move_priority_table_issues(&data.move_priorities, &data.moves, !data.moves.is_empty())
            .into_iter()
            .map(move_priority_table_issue_diagnostic),
    );
}

fn move_priority_table_issue_diagnostic(issue: MovePriorityTableIssue) -> VerificationError {
    match issue {
        MovePriorityTableIssue::InvalidBasePriority { priority } => VerificationError::error(
            "invalid_base_move_priority",
            "move_priorities:base_priority",
            format!("base move priority must be nonnegative, found {priority}"),
        ),
        MovePriorityTableIssue::MissingEffectPriorities => VerificationError::error(
            "missing_move_effect_priorities",
            "move_priorities:effect_priorities",
            "move effect priorities must be declared when moves exist",
        ),
        MovePriorityTableIssue::InvalidMoveEffectPriorityId { move_effect } => {
            VerificationError::error(
                "invalid_move_effect_priority_id",
                "move_priorities:effect_priorities",
                format!(
                    "move effect priority id must be exact and untrimmed, found {move_effect:?}"
                ),
            )
        }
        MovePriorityTableIssue::InvalidMoveEffectPriority { priority, .. } => {
            VerificationError::error(
                "invalid_move_effect_priority",
                "move_priorities:effect_priorities",
                format!("move effect priority must be nonnegative, found {priority}"),
            )
        }
        MovePriorityTableIssue::MissingMoveEffectPriority {
            move_name,
            move_effect,
        } => VerificationError::error(
            "missing_move_effect_priority",
            "move_priorities:effect_priorities",
            format!("move '{move_name}' effect '{move_effect}' must have an explicit priority row"),
        ),
        MovePriorityTableIssue::InvalidMovePriorityId { move_name } => VerificationError::error(
            "invalid_move_priority_id",
            "move_priorities:move_priorities",
            format!("move priority override id must be exact and untrimmed, found {move_name:?}"),
        ),
        MovePriorityTableIssue::UnknownMovePriority { move_name } => VerificationError::error(
            "unknown_move_priority",
            "move_priorities:move_priorities",
            format!("move priority override references missing move '{move_name}'"),
        ),
        MovePriorityTableIssue::DuplicateMovePriority { move_name } => VerificationError::error(
            "duplicate_move_priority",
            "move_priorities:move_priorities",
            format!("move priority override for move '{move_name}' is duplicated"),
        ),
        MovePriorityTableIssue::InvalidMovePriority { priority, .. } => VerificationError::error(
            "invalid_move_priority",
            "move_priorities:move_priorities",
            format!("move priority override must be nonnegative, found {priority}"),
        ),
    }
}

fn type_category_issue_diagnostic(issue: TypeCategoryIssue) -> VerificationError {
    match issue {
        TypeCategoryIssue::MissingPhysical => VerificationError::error(
            "missing_physical_type_categories",
            "type_categories:physical",
            "physical type categories must be declared when moves exist",
        ),
        TypeCategoryIssue::MissingSpecial => VerificationError::error(
            "missing_special_type_categories",
            "type_categories:special",
            "special type categories must be declared when moves exist",
        ),
        TypeCategoryIssue::InvalidToken { table, type_id } => VerificationError::error(
            "invalid_type_category_token",
            table.subject(),
            format!("type category token must be exact and untrimmed, found {type_id:?}"),
        ),
        TypeCategoryIssue::Overlap { type_id } => VerificationError::error(
            "overlapping_type_category",
            "type_categories",
            format!("type category '{type_id}' is declared as both physical and special"),
        ),
    }
}

fn special_routine_catalog_issue_diagnostic(
    issue: SpecialRoutineCatalogIssue,
) -> VerificationError {
    match issue {
        SpecialRoutineCatalogIssue::EmptyRoutine { .. } => VerificationError::error(
            "empty_special_routine",
            "special_routines",
            "special routine ids must be nonempty exact labels",
        ),
        SpecialRoutineCatalogIssue::InvalidRoutine { routine } => VerificationError::error(
            "invalid_special_routine",
            format!("special_routines:{routine}"),
            format!("special routine id '{routine}' must be an exact pack token"),
        ),
        SpecialRoutineCatalogIssue::UnknownRoutine { routine } => VerificationError::error(
            "unknown_declared_special_routine",
            format!("special_routines:{routine}"),
            format!("special routine '{routine}' is not implemented by the Rust runtime"),
        ),
    }
}

fn roaming_pokemon_catalog_issue_diagnostic(
    issue: RoamingPokemonCatalogIssue,
) -> VerificationError {
    VerificationError::error(
        "invalid_roaming_pokemon_catalog",
        "roaming_pokemon",
        issue.to_string(),
    )
}

fn buena_prize_definition_issue_diagnostic(issue: BuenaPrizeDefinitionIssue) -> VerificationError {
    match issue {
        BuenaPrizeDefinitionIssue::EmptyItem { item_id } => VerificationError::error(
            "empty_buena_prize_item",
            format!("buena_prizes:{item_id}"),
            "Buena prize item id must be an exact nonempty id",
        ),
        BuenaPrizeDefinitionIssue::InvalidItem { item_id } => VerificationError::error(
            "invalid_buena_prize_item",
            format!("buena_prizes:{item_id}"),
            format!("Buena prize item id '{item_id}' must be an exact nonempty id"),
        ),
        BuenaPrizeDefinitionIssue::UnknownItem { item_id } => VerificationError::error(
            "unknown_buena_prize_item",
            format!("buena_prizes:{item_id}"),
            format!("Buena prize references missing item '{item_id}'"),
        ),
        BuenaPrizeDefinitionIssue::InvalidCost { item_id } => VerificationError::error(
            "invalid_buena_prize_cost",
            format!("buena_prizes:{item_id}"),
            "Buena prize cost must be nonzero",
        ),
    }
}

fn buena_password_category_issue_diagnostic(
    issue: BuenaPasswordCategoryIssue,
) -> VerificationError {
    match issue {
        BuenaPasswordCategoryIssue::EmptyId { id } => VerificationError::error(
            "empty_buena_password_category_id",
            format!("buena_password_categories:{id}"),
            "Buena password category id must be an exact nonempty id",
        ),
        BuenaPasswordCategoryIssue::InvalidId { id } => VerificationError::error(
            "invalid_buena_password_category_id",
            format!("buena_password_categories:{id}"),
            format!("Buena password category id '{id}' must be an exact nonempty id"),
        ),
        BuenaPasswordCategoryIssue::UnknownOrderedId { id } => VerificationError::error(
            "unknown_buena_password_category_order_id",
            format!("buena_password_categories:{id}"),
            format!("Buena password category order references missing id '{id}'"),
        ),
        BuenaPasswordCategoryIssue::DuplicateOrderedId { id } => VerificationError::error(
            "duplicate_buena_password_category_order_id",
            format!("buena_password_categories:{id}"),
            format!("Buena password category order repeats id '{id}'"),
        ),
        BuenaPasswordCategoryIssue::InvalidCategoryType { id, category_type } => {
            VerificationError::error(
                "invalid_buena_password_category_type",
                format!("buena_password_categories:{id}"),
                format!(
                    "Buena password category '{id}' type must be an exact pack token, found {category_type:?}"
                ),
            )
        }
        BuenaPasswordCategoryIssue::UnknownCategoryType { id, category_type } => {
            VerificationError::error(
                "unknown_buena_password_category_type",
                format!("buena_password_categories:{id}"),
                format!("Buena password category '{id}' has unknown type '{category_type}'"),
            )
        }
        BuenaPasswordCategoryIssue::InvalidPoints { id } => VerificationError::error(
            "invalid_buena_password_points",
            format!("buena_password_categories:{id}"),
            "Buena password category points must be nonzero",
        ),
        BuenaPasswordCategoryIssue::EmptyOptions { id } => VerificationError::error(
            "empty_buena_password_options",
            format!("buena_password_categories:{id}"),
            "Buena password category must declare at least one option",
        ),
        BuenaPasswordCategoryIssue::EmptyOption { id, option_index } => VerificationError::error(
            "empty_buena_password_option",
            format!("buena_password_categories:{id}:option:{option_index}"),
            "Buena password option must be an exact nonempty id or string",
        ),
        BuenaPasswordCategoryIssue::InvalidOption {
            id,
            option_index,
            option,
        } => VerificationError::error(
            "invalid_buena_password_option",
            format!("buena_password_categories:{id}:option:{option_index}"),
            format!("Buena password option '{option}' must be an exact nonempty id or string"),
        ),
        BuenaPasswordCategoryIssue::UnknownSpecies {
            id,
            option_index,
            species,
        } => VerificationError::error(
            "unknown_buena_password_species",
            format!("buena_password_categories:{id}:option:{option_index}"),
            format!("Buena password option references missing species '{species}'"),
        ),
        BuenaPasswordCategoryIssue::UnknownItem {
            id,
            option_index,
            item_id,
        } => VerificationError::error(
            "unknown_buena_password_item",
            format!("buena_password_categories:{id}:option:{option_index}"),
            format!("Buena password option references missing item '{item_id}'"),
        ),
        BuenaPasswordCategoryIssue::UnknownMove {
            id,
            option_index,
            move_id,
        } => VerificationError::error(
            "unknown_buena_password_move",
            format!("buena_password_categories:{id}:option:{option_index}"),
            format!("Buena password option references missing move '{move_id}'"),
        ),
    }
}

fn kurt_apricorn_recipe_issue_diagnostic(issue: KurtApricornRecipeIssue) -> VerificationError {
    match issue {
        KurtApricornRecipeIssue::EmptyApricorn { apricorn } => VerificationError::error(
            "empty_kurt_apricorn_recipe_apricorn",
            format!("kurt_apricorn_recipes:{apricorn}"),
            "Kurt apricorn recipe apricorn id must be an exact nonempty id",
        ),
        KurtApricornRecipeIssue::InvalidApricorn { apricorn } => VerificationError::error(
            "invalid_kurt_apricorn_recipe_apricorn",
            format!("kurt_apricorn_recipes:{apricorn}"),
            format!("Kurt apricorn recipe apricorn id '{apricorn}' must be an exact nonempty id"),
        ),
        KurtApricornRecipeIssue::UnknownApricorn { apricorn } => VerificationError::error(
            "unknown_kurt_apricorn_recipe_apricorn",
            format!("kurt_apricorn_recipes:{apricorn}"),
            format!("Kurt apricorn recipe references missing apricorn item '{apricorn}'"),
        ),
        KurtApricornRecipeIssue::EmptyBall { apricorn } => VerificationError::error(
            "empty_kurt_apricorn_recipe_ball",
            format!("kurt_apricorn_recipes:{apricorn}"),
            "Kurt apricorn recipe ball id must be an exact nonempty id",
        ),
        KurtApricornRecipeIssue::InvalidBall { apricorn, ball } => VerificationError::error(
            "invalid_kurt_apricorn_recipe_ball",
            format!("kurt_apricorn_recipes:{apricorn}"),
            format!("Kurt apricorn recipe ball id '{ball}' must be an exact nonempty id"),
        ),
        KurtApricornRecipeIssue::UnknownBall { apricorn, ball } => VerificationError::error(
            "unknown_kurt_apricorn_recipe_ball",
            format!("kurt_apricorn_recipes:{apricorn}"),
            format!("Kurt apricorn recipe references missing ball item '{ball}'"),
        ),
    }
}

fn shuckie_gift_issue_diagnostic(issue: ShuckieGiftIssue) -> VerificationError {
    match issue {
        ShuckieGiftIssue::EmptySpecies => VerificationError::error(
            "empty_shuckie_gift_species",
            "shuckie_gift",
            "Shuckie gift species id must be an exact nonempty id",
        ),
        ShuckieGiftIssue::InvalidSpecies { species } => VerificationError::error(
            "invalid_shuckie_gift_species",
            "shuckie_gift",
            format!("Shuckie gift species id '{species}' must be an exact pack token"),
        ),
        ShuckieGiftIssue::UnknownSpecies { species } => VerificationError::error(
            "unknown_shuckie_gift_species",
            "shuckie_gift",
            format!("Shuckie gift references missing species '{species}'"),
        ),
        ShuckieGiftIssue::InvalidLevel => VerificationError::error(
            "invalid_shuckie_gift_level",
            "shuckie_gift",
            "Shuckie gift level must be nonzero",
        ),
        ShuckieGiftIssue::EmptyHeldItem => VerificationError::error(
            "empty_shuckie_gift_item",
            "shuckie_gift",
            "Shuckie gift held item id must be an exact nonempty id",
        ),
        ShuckieGiftIssue::InvalidHeldItem { held_item } => VerificationError::error(
            "invalid_shuckie_gift_item",
            "shuckie_gift",
            format!("Shuckie gift held item id '{held_item}' must be an exact pack token"),
        ),
        ShuckieGiftIssue::UnknownHeldItem { held_item } => VerificationError::error(
            "unknown_shuckie_gift_item",
            "shuckie_gift",
            format!("Shuckie gift references missing held item '{held_item}'"),
        ),
        ShuckieGiftIssue::EmptyName => VerificationError::error(
            "empty_shuckie_gift_name",
            "shuckie_gift",
            "Shuckie gift nickname and original trainer name must be nonempty",
        ),
        ShuckieGiftIssue::EmptyEngineFlag => VerificationError::error(
            "empty_shuckie_gift_engine_flag",
            "shuckie_gift",
            "Shuckie gift engine flag must be an exact nonempty id",
        ),
        ShuckieGiftIssue::InvalidEngineFlag { engine_flag } => VerificationError::error(
            "invalid_shuckie_gift_engine_flag",
            "shuckie_gift",
            format!("Shuckie gift engine flag '{engine_flag}' must be an exact pack token"),
        ),
        ShuckieGiftIssue::UnknownEngineFlag { engine_flag } => VerificationError::error(
            "unknown_shuckie_gift_engine_flag",
            "shuckie_gift",
            format!("Shuckie gift references missing engine flag '{engine_flag}'"),
        ),
    }
}

fn verify_special_routines(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    diagnostics.extend(
        special_routine_catalog_issues(&data.special_routines.keys().cloned().collect())
            .into_iter()
            .map(special_routine_catalog_issue_diagnostic),
    );
    if data.special_routines.contains_key("InitRoamMons") && data.roaming_pokemon.is_empty() {
        diagnostics.push(VerificationError::error(
            "missing_roaming_pokemon_definitions",
            "special_routines:InitRoamMons",
            "InitRoamMons requires explicit roaming Pokemon definitions in the modpack",
        ));
    }
    if data.special_routines.contains_key("BuenaPrize") && data.buena_prizes.is_empty() {
        diagnostics.push(VerificationError::error(
            "missing_buena_prize_definitions",
            "special_routines:BuenaPrize",
            "BuenaPrize requires explicit Buena prize definitions in the modpack",
        ));
    }
    if data.special_routines.contains_key("BuenasPassword")
        && data.buena_password_categories.categories.is_empty()
        && data.buena_password_categories.order.is_empty()
    {
        diagnostics.push(VerificationError::error(
            "missing_buena_password_categories",
            "special_routines:BuenasPassword",
            "BuenasPassword requires explicit Buena password categories in the modpack",
        ));
    }
    if data.special_routines.contains_key("SelectApricornForKurt")
        && data.kurt_apricorn_recipes.is_empty()
    {
        diagnostics.push(VerificationError::error(
            "missing_kurt_apricorn_recipes",
            "special_routines:SelectApricornForKurt",
            "SelectApricornForKurt requires explicit Kurt apricorn recipes in the modpack",
        ));
    }
    if (data.special_routines.contains_key("GiveShuckle")
        || data.special_routines.contains_key("ReturnShuckie"))
        && data.shuckie_gift.is_none()
    {
        diagnostics.push(VerificationError::error(
            "missing_shuckie_gift",
            "special_routines:Shuckie",
            "GiveShuckle and ReturnShuckie require explicit Shuckie gift data in the modpack",
        ));
    }
    if data.special_routines.contains_key("GiveDratini") && data.dratini_move_sets.is_empty() {
        diagnostics.push(VerificationError::error(
            "missing_dratini_move_sets",
            "special_routines:GiveDratini",
            "GiveDratini requires explicit Dratini move sets in the modpack",
        ));
    }
    if data.special_routines.contains_key("GiveOddEgg") && data.odd_egg_definitions.is_empty() {
        diagnostics.push(VerificationError::error(
            "missing_odd_egg_definitions",
            "special_routines:GiveOddEgg",
            "GiveOddEgg requires explicit Odd Egg definitions in the modpack",
        ));
    }
    if (data.special_routines.contains_key("GiveParkBalls")
        || data
            .special_routines
            .contains_key("SelectRandomBugContestContestants"))
        && data.bug_contest_config.is_none()
    {
        diagnostics.push(VerificationError::error(
            "missing_bug_contest_config",
            "special_routines:BugContest",
            "GiveParkBalls and SelectRandomBugContestContestants require explicit Bug-Catching Contest config in the modpack",
        ));
    }
    if (data.special_routines.contains_key("BattleTowerAction")
        || data
            .special_routines
            .contains_key("CheckForBattleTowerRules"))
        && data.battle_tower_rules.is_none()
    {
        diagnostics.push(VerificationError::error(
            "missing_battle_tower_rules",
            "special_routines:BattleTowerRules",
            "Battle Tower special routines require explicit Battle Tower rules in the modpack",
        ));
    }
    if data.special_routines.contains_key("ProfOaksPCBoot") && data.oak_ratings.is_empty() {
        diagnostics.push(VerificationError::error(
            "missing_oak_rating_table",
            "special_routines:ProfOaksPCBoot",
            "ProfOaksPCBoot requires explicit Oak rating entries in the modpack",
        ));
    }
    if data.special_routines.contains_key("CheckMagikarpLength") && data.magikarp_lengths.is_empty()
    {
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
    .any(|routine| data.special_routines.contains_key(*routine));
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
    let species_ids: BTreeSet<String> = data.pokemon.keys().cloned().collect();
    diagnostics.extend(
        roaming_pokemon_catalog_issues(&data.roaming_pokemon, &species_ids)
            .into_iter()
            .map(roaming_pokemon_catalog_issue_diagnostic),
    );
    if data
        .runtime_map_metadata
        .values()
        .any(|metadata| metadata.group_id == u16::from(data.roaming_pokemon.inactive_map.map_group))
    {
        diagnostics.push(VerificationError::error(
            "roaming_inactive_group_collides_with_runtime_map",
            "roaming_pokemon:inactiveMap",
            format!(
                "inactiveMap group {} must not match any runtime map group",
                data.roaming_pokemon.inactive_map.map_group
            ),
        ));
    }
    let roaming_map_exists = |map_group: u8, map_number: u8| {
        data.runtime_map_metadata.values().any(|metadata| {
            metadata.group_id == u16::from(map_group) && metadata.map_id == u16::from(map_number)
        })
    };
    for write in &data.roaming_pokemon.init_writes {
        if !roaming_map_exists(write.map_group, write.map_number) {
            diagnostics.push(VerificationError::error(
                "roaming_init_map_missing_from_runtime_metadata",
                format!("roaming_pokemon:initWrites[{}]", write.slot),
                format!(
                    "roaming init slot {} map {}/{} is missing from runtime map metadata",
                    write.slot, write.map_group, write.map_number
                ),
            ));
        }
    }
    for (index, route) in data.roaming_pokemon.routes.iter().enumerate() {
        if !roaming_map_exists(route.map_group, route.map_number) {
            diagnostics.push(VerificationError::error(
                "roaming_route_map_missing_from_runtime_metadata",
                format!("roaming_pokemon:routes[{index}]"),
                format!(
                    "roaming route {index} map {}/{} is missing from runtime map metadata",
                    route.map_group, route.map_number
                ),
            ));
        }
        for (connection_index, connection) in route.connections.iter().enumerate() {
            if !roaming_map_exists(connection.map_group, connection.map_number) {
                diagnostics.push(VerificationError::error(
                    "roaming_connection_map_missing_from_runtime_metadata",
                    format!(
                        "roaming_pokemon:routes[{index}].connections[{connection_index}]"
                    ),
                    format!(
                        "roaming route {index} connection {connection_index} map {}/{} is missing from runtime map metadata",
                        connection.map_group, connection.map_number
                    ),
                ));
            }
        }
    }
    let item_ids: BTreeSet<String> = data.items.keys().cloned().collect();
    diagnostics.extend(
        buena_prize_definition_issues(&data.buena_prizes, &item_ids)
            .into_iter()
            .map(buena_prize_definition_issue_diagnostic),
    );
    let move_ids: BTreeSet<String> = data.moves.keys().cloned().collect();
    diagnostics.extend(
        buena_password_category_issues(
            &data.buena_password_categories,
            &species_ids,
            &item_ids,
            &move_ids,
        )
        .into_iter()
        .map(buena_password_category_issue_diagnostic),
    );
    diagnostics.extend(
        kurt_apricorn_recipe_issues(&data.kurt_apricorn_recipes, &item_ids)
            .into_iter()
            .map(kurt_apricorn_recipe_issue_diagnostic),
    );
    if let Some(gift) = data.shuckie_gift.as_ref() {
        let engine_flags = script_engine_flag_ids(data);
        diagnostics.extend(
            shuckie_gift_issues(gift, &species_ids, &item_ids, &engine_flags)
                .into_iter()
                .map(shuckie_gift_issue_diagnostic),
        );
    }
    let move_ids: BTreeSet<String> = data.moves.keys().cloned().collect();
    diagnostics.extend(
        dratini_move_set_issues(&data.dratini_move_sets, &move_ids)
            .into_iter()
            .map(dratini_move_set_issue_diagnostic),
    );
    let species_ids: BTreeSet<String> = data.pokemon.keys().cloned().collect();
    diagnostics.extend(
        odd_egg_definition_issues(&data.odd_egg_definitions, &species_ids, &move_ids)
            .into_iter()
            .map(odd_egg_definition_issue_diagnostic),
    );
    if let Some(config) = data.bug_contest_config.as_ref() {
        let event_flags = script_event_flag_ids(data);
        diagnostics.extend(
            bug_contest_config_issues(config, &event_flags)
                .into_iter()
                .map(bug_contest_config_issue_diagnostic),
        );
        for (index, encounter) in config.encounters.iter().enumerate() {
            if !species_ids.contains(&encounter.species) {
                diagnostics.push(VerificationError::error(
                    "unknown_bug_contest_encounter_species",
                    format!("bug_contest_config:encounters:{index}:species"),
                    format!(
                        "Bug-Catching Contest encounter references missing species '{}'",
                        encounter.species
                    ),
                ));
            }
        }
    }
    if let Some(rules) = data.battle_tower_rules.as_ref() {
        let species_ids: BTreeSet<String> = data.pokemon.keys().cloned().collect();
        diagnostics.extend(
            battle_tower_rules_issues(rules, &species_ids)
                .into_iter()
                .map(battle_tower_rules_issue_diagnostic),
        );
    }
    if !data.oak_ratings.is_empty() {
        diagnostics.extend(
            oak_rating_table_issues(&data.oak_ratings, data.pokemon.len())
                .into_iter()
                .map(oak_rating_table_issue_diagnostic),
        );
    }
    diagnostics.extend(
        magikarp_length_table_issues(&data.magikarp_lengths)
            .into_iter()
            .map(magikarp_length_table_issue_diagnostic),
    );
}

fn oak_rating_table_issue_diagnostic(issue: OakRatingTableIssue) -> VerificationError {
    match issue {
        OakRatingTableIssue::InvalidFanfare { index, .. } => VerificationError::error(
            "invalid_oak_rating_fanfare",
            format!("oak_ratings:{index}"),
            "Oak rating fanfare must be an exact nonempty id",
        ),
        OakRatingTableIssue::InvalidTextLabel { index, .. } => VerificationError::error(
            "invalid_oak_rating_text_label",
            format!("oak_ratings:{index}"),
            "Oak rating textLabel must be an exact nonempty id",
        ),
        OakRatingTableIssue::InvalidOrder { index, .. } => VerificationError::error(
            "invalid_oak_rating_order",
            format!("oak_ratings:{index}"),
            "Oak rating caughtCountLimit values must be strictly increasing",
        ),
        OakRatingTableIssue::IncompleteCoverage {
            pokemon_count,
            last_caught_count_limit,
        } => VerificationError::error(
            "incomplete_oak_rating_coverage",
            "oak_ratings",
            format!(
                "Oak rating table only covers {last_caught_count_limit} caught Pokemon, but {pokemon_count} Pokemon are loaded"
            ),
        ),
    }
}

fn battle_tower_rules_issue_diagnostic(issue: BattleTowerRulesIssue) -> VerificationError {
    match issue {
        BattleTowerRulesIssue::MissingRequiredPartyCount => VerificationError::error(
            "invalid_battle_tower_required_party_count",
            "battle_tower_rules:required_party_count",
            "Battle Tower requiredPartyCount must be nonzero",
        ),
        BattleTowerRulesIssue::MissingChallengeStreakLength => VerificationError::error(
            "invalid_battle_tower_challenge_streak_length",
            "battle_tower_rules:challengeStreakLength",
            "Battle Tower challengeStreakLength must be nonzero",
        ),
        BattleTowerRulesIssue::MissingLevelGroupSize => VerificationError::error(
            "invalid_battle_tower_level_group_size",
            "battle_tower_rules:levelGroupSize",
            "Battle Tower levelGroupSize must be nonzero",
        ),
        BattleTowerRulesIssue::InvalidLevelGroupRange => VerificationError::error(
            "invalid_battle_tower_level_group_range",
            "battle_tower_rules:levelGroupRange",
            "Battle Tower level group range must be nonzero and ordered",
        ),
        BattleTowerRulesIssue::MissingTrainerRoster => VerificationError::error(
            "missing_battle_tower_trainer_roster",
            "battle_tower_rules:trainers",
            "Battle Tower trainer roster must be compiled and nonempty",
        ),
        BattleTowerRulesIssue::MissingMonGroups => VerificationError::error(
            "missing_battle_tower_mon_groups",
            "battle_tower_rules:monGroups",
            "Battle Tower Pokemon groups must be compiled and nonempty",
        ),
        BattleTowerRulesIssue::InvalidFailureText { field, .. } => VerificationError::error(
            "invalid_battle_tower_failure_text",
            field.subject(),
            "Battle Tower failure text ids must be exact nonempty ids",
        ),
        BattleTowerRulesIssue::InvalidBannedSpecies { species_id } => VerificationError::error(
            "invalid_battle_tower_banned_species",
            format!("battle_tower_rules:bannedSpecies:{species_id}"),
            "Battle Tower bannedSpecies entries must be exact nonempty species ids",
        ),
        BattleTowerRulesIssue::UnknownBannedSpecies { species_id } => VerificationError::error(
            "unknown_battle_tower_banned_species",
            format!("battle_tower_rules:bannedSpecies:{species_id}"),
            format!("Battle Tower bannedSpecies references missing species '{species_id}'"),
        ),
    }
}

fn odd_egg_definition_issue_diagnostic(issue: OddEggDefinitionIssue) -> VerificationError {
    match issue {
        OddEggDefinitionIssue::InvalidProbabilityTotal { total_probability } => {
            VerificationError::error(
                "invalid_odd_egg_probability_total",
                "odd_egg_definitions",
                format!("Odd Egg probabilities must sum to 100, found {total_probability}"),
            )
        }
        OddEggDefinitionIssue::InvalidSpecies { index, .. } => VerificationError::error(
            "invalid_odd_egg_species",
            format!("odd_egg_definitions:{index}"),
            "Odd Egg species must be an exact nonempty species id",
        ),
        OddEggDefinitionIssue::UnknownSpecies { index, species_id } => VerificationError::error(
            "unknown_odd_egg_species",
            format!("odd_egg_definitions:{index}"),
            format!("Odd Egg references missing species '{species_id}'"),
        ),
        OddEggDefinitionIssue::InvalidMoveCount { index, .. } => VerificationError::error(
            "invalid_odd_egg_move_count",
            format!("odd_egg_definitions:{index}"),
            "Odd Egg move list must contain 1..=4 exact move ids",
        ),
        OddEggDefinitionIssue::InvalidMove {
            index, move_index, ..
        } => VerificationError::error(
            "invalid_odd_egg_move",
            format!("odd_egg_definitions:{index}:move:{move_index}"),
            "Odd Egg move id must be an exact nonempty id",
        ),
        OddEggDefinitionIssue::UnknownMove {
            index,
            move_index,
            move_id,
        } => VerificationError::error(
            "unknown_odd_egg_move",
            format!("odd_egg_definitions:{index}:move:{move_index}"),
            format!("Odd Egg references missing move '{move_id}'"),
        ),
        OddEggDefinitionIssue::InvalidProbability { index } => VerificationError::error(
            "invalid_odd_egg_probability",
            format!("odd_egg_definitions:{index}"),
            "Odd Egg probability must be positive",
        ),
        OddEggDefinitionIssue::InvalidLevel { index, level } => VerificationError::error(
            "invalid_odd_egg_level",
            format!("odd_egg_definitions:{index}"),
            format!("Odd Egg level must be 1..=100, found {level}"),
        ),
        OddEggDefinitionIssue::InvalidNickname { index, .. } => VerificationError::error(
            "invalid_odd_egg_nickname",
            format!("odd_egg_definitions:{index}"),
            "Odd Egg nickname must be exact nonempty pack data",
        ),
        OddEggDefinitionIssue::InvalidOriginalTrainerName { index, .. } => {
            VerificationError::error(
                "invalid_odd_egg_original_trainer_name",
                format!("odd_egg_definitions:{index}"),
                "Odd Egg original trainer name must be exact nonempty pack data",
            )
        }
    }
}

fn dratini_move_set_issue_diagnostic(issue: DratiniMoveSetIssue) -> VerificationError {
    match issue {
        DratiniMoveSetIssue::EmptyMoveSet { mode } => VerificationError::error(
            "empty_dratini_move_set",
            format!("dratini_move_sets:{mode}"),
            "Dratini move set must contain at least one move",
        ),
        DratiniMoveSetIssue::InvalidMove {
            mode, move_index, ..
        } => VerificationError::error(
            "invalid_dratini_move",
            format!("dratini_move_sets:{mode}:move:{move_index}"),
            "Dratini move id must be an exact nonempty id",
        ),
        DratiniMoveSetIssue::UnknownMove {
            mode,
            move_index,
            move_id,
        } => VerificationError::error(
            "unknown_dratini_move",
            format!("dratini_move_sets:{mode}:move:{move_index}"),
            format!("Dratini move set references missing move '{move_id}'"),
        ),
    }
}

fn bug_contest_config_issue_diagnostic(issue: BugContestConfigIssue) -> VerificationError {
    match issue {
        BugContestConfigIssue::MissingParkBalls => VerificationError::error(
            "invalid_bug_contest_park_balls",
            "bug_contest_config",
            "Bug-Catching Contest park_balls must be positive",
        ),
        BugContestConfigIssue::InvalidTimerSeconds { timer_seconds } => VerificationError::error(
            "invalid_bug_contest_timer_seconds",
            "bug_contest_config",
            format!("Bug-Catching Contest timer_seconds must be 0..=59, found {timer_seconds}"),
        ),
        BugContestConfigIssue::MissingSelectedContestantCount => VerificationError::error(
            "invalid_bug_contest_selected_count",
            "bug_contest_config",
            "Bug-Catching Contest selected_contestant_count must be positive",
        ),
        BugContestConfigIssue::SelectedContestantCountExceedsFlags {
            selected_contestant_count,
            contestant_flag_count,
        } => VerificationError::error(
            "invalid_bug_contest_selected_count",
            "bug_contest_config",
            format!(
                "Bug-Catching Contest selected_contestant_count {selected_contestant_count} exceeds {contestant_flag_count} contestant flags"
            ),
        ),
        BugContestConfigIssue::InvalidContestantFlag { index, .. } => VerificationError::error(
            "invalid_bug_contest_contestant_flag",
            format!("bug_contest_config:contestant_flags:{index}"),
            "Bug-Catching Contest contestant flag must be an exact nonempty id",
        ),
        BugContestConfigIssue::DuplicateContestantFlag { index, flag } => VerificationError::error(
            "duplicate_bug_contest_contestant_flag",
            format!("bug_contest_config:contestant_flags:{index}"),
            format!("Bug-Catching Contest contestant flag '{flag}' is duplicated"),
        ),
        BugContestConfigIssue::UnknownContestantFlag { index, flag } => VerificationError::error(
            "unknown_bug_contest_contestant_flag",
            format!("bug_contest_config:contestant_flags:{index}"),
            format!("Bug-Catching Contest contestant flag '{flag}' is not loaded"),
        ),
        BugContestConfigIssue::InvalidEncounterTable { message } => VerificationError::error(
            "invalid_bug_contest_encounter_table",
            "bug_contest_config:encounters",
            message,
        ),
        BugContestConfigIssue::InvalidEncounterSpecies { index, species } => {
            VerificationError::error(
                "invalid_bug_contest_encounter_species",
                format!("bug_contest_config:encounters:{index}:species"),
                format!("Bug-Catching Contest encounter species '{species}' is invalid"),
            )
        }
        BugContestConfigIssue::UnsupportedEncounterSpecies { index, species } => {
            VerificationError::error(
                "unsupported_bug_contest_encounter_species",
                format!("bug_contest_config:encounters:{index}:species"),
                format!(
                    "Bug-Catching Contest encounter species '{species}' is not supported by the source chooser"
                ),
            )
        }
        BugContestConfigIssue::InvalidEncounterLevelRange {
            index,
            min_level,
            max_level,
        } => VerificationError::error(
            "invalid_bug_contest_encounter_level_range",
            format!("bug_contest_config:encounters:{index}"),
            format!(
                "Bug-Catching Contest encounter level range {min_level}..={max_level} is invalid"
            ),
        ),
    }
}

fn magikarp_length_table_issue_diagnostic(issue: MagikarpLengthTableIssue) -> VerificationError {
    match issue {
        MagikarpLengthTableIssue::InvalidEntryCount { actual } => VerificationError::error(
            "invalid_magikarp_length_entry_count",
            "magikarp_lengths",
            format!("Magikarp length table must contain exactly 14 source rows, found {actual}"),
        ),
        MagikarpLengthTableIssue::InvalidDivisor { index, threshold } => VerificationError::error(
            "invalid_magikarp_length_divisor",
            format!("magikarp_lengths:{index}"),
            format!("Magikarp length threshold {threshold} has zero divisor"),
        ),
        MagikarpLengthTableIssue::InvalidThresholdOrder { index, .. } => VerificationError::error(
            "invalid_magikarp_length_threshold_order",
            format!("magikarp_lengths:{index}"),
            "Magikarp length thresholds must be strictly increasing",
        ),
    }
}

fn script_event_flag_ids(data: &GameDataSet) -> BTreeSet<String> {
    let mut flags = data
        .initialize_events
        .event_flags
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    flags.extend(
        data.fruit_trees
            .0
            .keys()
            .map(|tree_id| fruit_tree_collected_flag(tree_id)),
    );
    flags.extend(
        data.decorations
            .decorations
            .iter()
            .map(|decoration| decoration.event_flag.clone()),
    );
    for module in data.maps.values() {
        for command in &module.script_flag_commands {
            if command.flag_id.starts_with("EVENT_") {
                flags.insert(command.flag_id.clone());
            }
        }
        for body in module.scripts.values().filter_map(Value::as_array) {
            for command in body {
                if command.get("command").and_then(Value::as_str) == Some("conditional_event")
                    && let Some(flag) = command
                        .get("args")
                        .and_then(Value::as_array)
                        .and_then(|args| args.first())
                        .and_then(Value::as_str)
                {
                    flags.insert(flag.to_string());
                }
            }
        }
    }
    if let Some(module) = &data.global_scripts {
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
    if let Some(module) = &data.global_scripts {
        for command in &module.script_flag_commands {
            if command.flag_id.starts_with("ENGINE_") {
                flags.insert(command.flag_id.clone());
            }
        }
    }
    if !data.field_moves.strength.engine_flag.is_empty() {
        flags.insert(data.field_moves.strength.engine_flag.clone());
    }
    if !data.field_moves.flash.engine_flag.is_empty() {
        flags.insert(data.field_moves.flash.engine_flag.clone());
    }
    flags
}

fn verify_script_runtime_commands(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    for (map_name, module) in &data.maps {
        verify_script_runtime_module_commands(
            data,
            map_name,
            &module.scripts,
            &module.scripts,
            &module.script_runtime_commands,
            diagnostics,
        );
    }
    if let Some(module) = data.global_scripts.as_ref() {
        verify_script_runtime_module_commands(
            data,
            "GlobalScripts",
            &module.scripts,
            &module.definitions,
            &module.script_runtime_commands,
            diagnostics,
        );
    }
}

fn verify_script_runtime_module_commands(
    data: &GameDataSet,
    map_name: &str,
    _scripts: &BTreeMap<String, Value>,
    callasm_definitions: &BTreeMap<String, Value>,
    commands: &[ScriptRuntimeCommand],
    diagnostics: &mut Vec<VerificationError>,
) {
    verify_unique_script_command_positions(
        map_name,
        "script_runtime_commands",
        commands
            .iter()
            .map(|command| (command.source_script.as_str(), command.command_index)),
        diagnostics,
    );
    let catalog = ScriptRuntimeReferenceCatalog {
        special_routines: data.special_routines.keys().cloned().collect(),
        trainer_classes: data
            .trainers
            .trainers
            .iter()
            .map(|(trainer_id, trainer)| (trainer_id.clone(), trainer.trainer_class.clone()))
            .collect(),
        trainer_class_names: data.trainer_class_names.keys().cloned().collect(),
        items: data.items.keys().cloned().collect(),
        pokemon: data.pokemon.keys().cloned().collect(),
        phone_contacts: data.phone_contacts.0.keys().cloned().collect(),
        special_phone_calls: data.special_phone_calls.keys().cloned().collect(),
        npc_trades: data.npc_trades.keys().cloned().collect(),
        landmarks: data
            .pokegear_landmarks
            .landmarks
            .iter()
            .map(|landmark| landmark.constant.clone())
            .collect(),
        script_labels: callasm_definitions.keys().cloned().collect(),
    };
    for command in commands {
        let subject = format!(
            "{map_name}:{}:{}",
            command.source_script, command.command_index
        );
        if command.command == "writecmdqueue"
            && let Err(error) = data.resolve_script_runtime_stone_table_queue(map_name, command)
        {
            diagnostics.push(VerificationError::error(
                "invalid_script_command_queue",
                subject.clone(),
                error.to_string(),
            ));
        }
        if map_name == "GlobalScripts"
            && command.source_script == "TreeMons"
            && command.command == "dw"
        {
            continue;
        }
        diagnostics.extend(
            script_runtime_command_issues(command, &catalog)
                .into_iter()
                .map(|issue| script_runtime_command_issue_diagnostic(&subject, command, issue)),
        );
        if command.command == "callasm"
            && let [target] = command.args.as_slice()
        {
            let Some(resolved_target) =
                resolve_script_target_label(callasm_definitions, &command.source_script, target)
            else {
                diagnostics.push(VerificationError::error(
                    "non_executable_callasm_target",
                    subject.clone(),
                    format!("callasm target '{target}' has no exact resolvable CPU routine body"),
                ));
                continue;
            };
            let certificate = if command.source_script == "RockSmashScript"
                && command.command_index == 8
                && target == "RockMonEncounter"
            {
                certify_rock_mon_encounter_callasm_target(callasm_definitions, &resolved_target)
            } else if command.source_script == "HeadbuttScript"
                && command.command_index == 4
                && target == "TreeMonEncounter"
            {
                certify_tree_mon_encounter_callasm_target(callasm_definitions, &resolved_target)
            } else {
                certify_synchronous_script_callasm_target(callasm_definitions, &resolved_target)
            };
            if let Err(failure) = certificate {
                diagnostics.push(VerificationError::error(
                    "non_executable_callasm_target",
                    subject.clone(),
                    format!(
                        "callasm target '{target}' resolves to '{}', but command {} '{}' is not synchronously executable: {}",
                        failure.target_script,
                        failure.command_index,
                        failure.command,
                        failure.reason,
                    ),
                ));
            }
        }
        if command.command == "special"
            && let Some(routine) = command.args.first()
            && data.special_routines.contains_key(routine)
            && !EXECUTABLE_SPECIAL_ROUTINES.contains(&routine.as_str())
        {
            diagnostics.push(VerificationError::error(
                "inactive_script_special_routine",
                subject,
                format!("special references inactive declared routine '{routine}'"),
            ));
        }
    }
}

fn verify_script_swarm_commands(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    let map_ids = data
        .runtime_map_metadata
        .values()
        .map(|metadata| metadata.constant.clone())
        .collect::<BTreeSet<_>>();
    for (map_name, module) in &data.maps {
        verify_unique_script_command_positions(
            map_name,
            "script_swarm_commands",
            module
                .script_swarm_commands
                .iter()
                .map(|command| (command.source_script.as_str(), command.command_index)),
            diagnostics,
        );
        for command in &module.script_swarm_commands {
            let subject = format!(
                "{map_name}:{}:{}",
                command.source_script, command.command_index
            );
            diagnostics.extend(
                script_swarm_command_issues(command, &map_ids)
                    .into_iter()
                    .map(|issue| script_swarm_command_issue_diagnostic(&subject, command, issue)),
            );
        }
    }
}

fn script_swarm_command_issue_diagnostic(
    subject: &str,
    command: &ScriptSwarmCommand,
    issue: ScriptSwarmCommandIssue,
) -> VerificationError {
    match issue {
        ScriptSwarmCommandIssue::InvalidCommand => VerificationError::error(
            "invalid_script_swarm_command",
            subject,
            format!(
                "script swarm command must be exact 'swarm', found {:?}",
                command.command
            ),
        ),
        ScriptSwarmCommandIssue::UnknownCommand => VerificationError::error(
            "unknown_script_swarm_command",
            subject,
            format!("unknown script swarm command '{}'", command.command),
        ),
        ScriptSwarmCommandIssue::InvalidSwarmToken => VerificationError::error(
            "invalid_script_swarm_token",
            subject,
            format!(
                "script swarm token must be exact, found {:?}",
                command.swarm_token
            ),
        ),
        ScriptSwarmCommandIssue::InvalidMapId => VerificationError::error(
            "invalid_script_swarm_map_id",
            subject,
            format!(
                "script swarm map id must be exact, found {:?}",
                command.map_id
            ),
        ),
        ScriptSwarmCommandIssue::UnknownMap => VerificationError::error(
            "unknown_script_swarm_map_id",
            subject,
            format!(
                "script swarm references missing runtime map '{}'",
                command.map_id
            ),
        ),
    }
}

fn script_runtime_command_issue_diagnostic(
    subject: &str,
    command: &ScriptRuntimeCommand,
    issue: ScriptRuntimeCommandIssue,
) -> VerificationError {
    match issue {
        ScriptRuntimeCommandIssue::InvalidCommand { error } => {
            script_runtime_invalid_command_diagnostic(subject, error)
        }
        ScriptRuntimeCommandIssue::UnknownSpecialRoutine { special_id } => {
            VerificationError::error(
                "unknown_script_special_routine",
                subject,
                format!("special references unknown routine '{special_id}'"),
            )
        }
        ScriptRuntimeCommandIssue::InvalidSpecialRoutine { special_id } => {
            VerificationError::error(
                "invalid_script_special_routine",
                subject,
                format!("special routine id must be an exact pack token, found {special_id:?}"),
            )
        }
        ScriptRuntimeCommandIssue::UnknownTrainer { trainer_id } => VerificationError::error(
            "unknown_script_trainer_name",
            subject,
            format!("gettrainername references unknown trainer '{trainer_id}'"),
        ),
        ScriptRuntimeCommandIssue::InvalidTrainer { trainer_id } => VerificationError::error(
            "invalid_script_trainer_name",
            subject,
            format!("gettrainername trainer id must be an exact pack token, found {trainer_id:?}"),
        ),
        ScriptRuntimeCommandIssue::InvalidTrainerClass { trainer_class } => {
            VerificationError::error(
                "invalid_script_trainer_class",
                subject,
                format!(
                    "gettrainername trainer class must be an exact pack token, found {trainer_class:?}"
                ),
            )
        }
        ScriptRuntimeCommandIssue::UnknownTrainerClassName { trainer_class } => {
            VerificationError::error(
                "unknown_script_trainer_class_name",
                subject,
                format!(
                    "gettrainerclassname references class '{trainer_class}' without an authoritative display name"
                ),
            )
        }
        ScriptRuntimeCommandIssue::TrainerClassMismatch {
            trainer_id,
            expected_class,
            actual_class,
        } => VerificationError::error(
            "script_trainer_name_class_mismatch",
            subject,
            format!(
                "gettrainername expected trainer '{}' to have class '{}' but pack declares '{}'",
                trainer_id, expected_class, actual_class
            ),
        ),
        ScriptRuntimeCommandIssue::UnknownItem { item_id } => VerificationError::error(
            "unknown_script_item_name",
            subject,
            format!("getitemname references unknown item '{item_id}'"),
        ),
        ScriptRuntimeCommandIssue::InvalidItem { item_id } => VerificationError::error(
            "invalid_script_item_name",
            subject,
            format!("getitemname item id must be an exact pack token, found {item_id:?}"),
        ),
        ScriptRuntimeCommandIssue::UnknownSpecies { species_id } => {
            let code = if command.command == "getmonname" {
                "unknown_script_mon_name"
            } else {
                "unknown_script_species_runtime_command"
            };
            let message = if command.command == "getmonname" {
                format!("getmonname references unknown species '{species_id}'")
            } else {
                format!(
                    "{} references unknown species '{}'",
                    command.command, species_id
                )
            };
            VerificationError::error(code, subject, message)
        }
        ScriptRuntimeCommandIssue::InvalidSpecies { species_id } => {
            let code = if command.command == "getmonname" {
                "invalid_script_mon_name"
            } else {
                "invalid_script_species_runtime_command"
            };
            let message = if command.command == "getmonname" {
                format!("getmonname species id must be an exact pack token, found {species_id:?}")
            } else {
                format!(
                    "{} species id must be an exact pack token, found {:?}",
                    command.command, species_id
                )
            };
            VerificationError::error(code, subject, message)
        }
        ScriptRuntimeCommandIssue::UnknownPhoneContact { contact_id } => VerificationError::error(
            "unknown_script_addcellnum_contact",
            subject,
            format!("addcellnum references unknown contact '{contact_id}'"),
        ),
        ScriptRuntimeCommandIssue::InvalidPhoneContact { contact_id } => VerificationError::error(
            "invalid_script_addcellnum_contact",
            subject,
            format!("addcellnum contact id must be an exact pack token, found {contact_id:?}"),
        ),
        ScriptRuntimeCommandIssue::UnknownSpecialPhoneCall { call_id } => VerificationError::error(
            "unknown_script_special_phone_call",
            subject,
            format!("specialphonecall references unknown call '{call_id}'"),
        ),
        ScriptRuntimeCommandIssue::InvalidSpecialPhoneCall { call_id } => VerificationError::error(
            "invalid_script_special_phone_call",
            subject,
            format!("specialphonecall id must be an exact pack token, found {call_id:?}"),
        ),
        ScriptRuntimeCommandIssue::UnknownNpcTrade { trade_id } => VerificationError::error(
            "unknown_script_npc_trade",
            subject,
            format!("trade references unknown NPC trade '{trade_id}'"),
        ),
        ScriptRuntimeCommandIssue::InvalidNpcTrade { trade_id } => VerificationError::error(
            "invalid_script_npc_trade",
            subject,
            format!("trade id must be an exact pack token, found {trade_id:?}"),
        ),
        ScriptRuntimeCommandIssue::UnknownLandmark { landmark_id } => VerificationError::error(
            "unknown_script_landmark_name",
            subject,
            format!("getlandmarkname references unknown landmark '{landmark_id}'"),
        ),
        ScriptRuntimeCommandIssue::InvalidLandmark { landmark_id } => VerificationError::error(
            "invalid_script_landmark_name",
            subject,
            format!(
                "getlandmarkname landmark id must use exact LANDMARK_* syntax, found {landmark_id:?}"
            ),
        ),
        ScriptRuntimeCommandIssue::UnknownTarget { target_label } => VerificationError::error(
            "unknown_script_runtime_target",
            subject,
            format!(
                "{} references unknown target '{}'",
                command.command, target_label
            ),
        ),
        ScriptRuntimeCommandIssue::InvalidTarget { target_label } => VerificationError::error(
            "invalid_script_runtime_target",
            subject,
            format!(
                "{} target label must be exact, found {target_label:?}",
                command.command
            ),
        ),
    }
}

fn script_runtime_invalid_command_diagnostic(
    subject: &str,
    error: ScriptRuntimeCommandError,
) -> VerificationError {
    match error {
        ScriptRuntimeCommandError::UnknownCommand { command } => VerificationError::error(
            "unknown_script_runtime_command",
            subject,
            format!("unknown runtime command '{command}'"),
        ),
        ScriptRuntimeCommandError::WrongArgCount {
            command,
            expected,
            actual,
        } => VerificationError::error(
            "malformed_script_runtime_command",
            subject,
            format!("{command} expects {expected} args but found {actual}"),
        ),
        ScriptRuntimeCommandError::EmptyArg { command }
        | ScriptRuntimeCommandError::PaddedArg { command, .. } => VerificationError::error(
            "malformed_script_runtime_command",
            subject,
            format!("{command} requires exact nonempty args"),
        ),
        error => VerificationError::error(
            "malformed_script_runtime_command",
            subject,
            error.to_string(),
        ),
    }
}

fn verify_map_section_commands(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    for (map_name, module) in &data.maps {
        let script_labels: BTreeSet<String> = module.scripts.keys().cloned().collect();
        for command in &module.map_script_section_commands {
            let subject = format!("{map_name}:map_scripts:{}", command.command_index);
            diagnostics.extend(
                map_script_section_command_issues(command, &script_labels)
                    .into_iter()
                    .map(|issue| {
                        map_script_section_command_issue_diagnostic(&subject, command, issue)
                    }),
            );
        }
        for command in &module.map_event_section_commands {
            let subject = format!("{map_name}:map_events:{}", command.command_index);
            diagnostics.extend(
                map_event_section_command_issues(command, &script_labels)
                    .into_iter()
                    .map(|issue| {
                        map_event_section_command_issue_diagnostic(&subject, command, issue)
                    }),
            );
        }
    }
}

fn map_script_section_command_issue_diagnostic(
    subject: &str,
    command: &MapScriptSectionCommand,
    issue: MapScriptSectionCommandIssue,
) -> VerificationError {
    match issue {
        MapScriptSectionCommandIssue::UnknownCommand => VerificationError::error(
            "unknown_map_script_section_command",
            subject,
            format!("unknown map script section command '{}'", command.command),
        ),
        MapScriptSectionCommandIssue::WrongArgCount { expected, actual } => {
            VerificationError::error(
                "malformed_map_script_section_command",
                subject,
                format!(
                    "{} expects one of {:?} args but found {}",
                    command.command, expected, actual
                ),
            )
        }
        MapScriptSectionCommandIssue::InvalidArg { arg } => VerificationError::error(
            "malformed_map_script_section_command",
            subject,
            format!("{} has invalid exact operand '{arg}'", command.command),
        ),
        MapScriptSectionCommandIssue::UnknownSceneScript { script } => VerificationError::error(
            "unknown_map_scene_script",
            subject,
            format!("scene_script references unknown script '{script}'"),
        ),
        MapScriptSectionCommandIssue::UnknownCallbackScript { script } => VerificationError::error(
            "unknown_map_callback_script",
            subject,
            format!("callback references unknown script '{script}'"),
        ),
    }
}

fn map_event_section_command_issue_diagnostic(
    subject: &str,
    command: &MapEventSectionCommand,
    issue: MapEventSectionCommandIssue,
) -> VerificationError {
    match issue {
        MapEventSectionCommandIssue::UnknownCommand => VerificationError::error(
            "unknown_map_event_section_command",
            subject,
            format!("unknown map event section command '{}'", command.command),
        ),
        MapEventSectionCommandIssue::WrongArgCount { expected, actual } => {
            VerificationError::error(
                "malformed_map_event_section_command",
                subject,
                format!(
                    "{} expects one of {:?} args but found {}",
                    command.command, expected, actual
                ),
            )
        }
        MapEventSectionCommandIssue::InvalidArg { arg } => VerificationError::error(
            "malformed_map_event_section_command",
            subject,
            format!("{} has invalid exact operand '{arg}'", command.command),
        ),
        MapEventSectionCommandIssue::UnknownEventScript { script } => VerificationError::error(
            "unknown_map_event_script",
            subject,
            format!("{} references unknown script '{script}'", command.command),
        ),
        MapEventSectionCommandIssue::UnknownObjectEventScript { script } => {
            VerificationError::error(
                "unknown_map_object_event_script",
                subject,
                format!("object_event references unknown script '{script}'"),
            )
        }
    }
}

fn is_text_script(payload: &Value) -> bool {
    let Some(entries) = payload.as_array() else {
        return false;
    };
    let text_commands = text_body_command_arg_counts();
    // Text bodies begin with a TX_* macro. Checking every later command would
    // misclassify raw data such as mail strings, where `db` is followed by the
    // charmap's `next` control token.
    entries
        .first()
        .and_then(|entry| entry.get("command"))
        .and_then(Value::as_str)
        .is_some_and(|command| text_commands.contains_key(command))
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

fn verify_scene_token(
    diagnostics: &mut Vec<VerificationError>,
    subject: &str,
    map_name: &str,
    scene_id: Option<&str>,
    table: &MapSceneTable,
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
        if table.scenes.is_empty() || index < table.scenes.len() {
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
    diagnostics.extend(
        battle_reward_rules_issues(&data.battle_reward_rules)
            .into_iter()
            .map(battle_reward_rules_issue_diagnostic),
    );
    match data.currency_constants.0.get("START_MONEY") {
        Some(_) => {}
        None => diagnostics.push(VerificationError::error(
            "missing_start_money_constant",
            "currency_constants:START_MONEY",
            "new-game initialization requires the exact START_MONEY constant",
        )),
    }
    match data.currency_constants.0.get("MOM_MONEY") {
        Some(value) if *value == data.battle_reward_rules.mom_money_increment => {}
        Some(value) => diagnostics.push(VerificationError::error(
            "mismatched_mom_money_constant",
            "currency_constants:MOM_MONEY",
            format!(
                "MOM_MONEY is {value}, but the exported Mom purchase increment is {}",
                data.battle_reward_rules.mom_money_increment
            ),
        )),
        None => diagnostics.push(VerificationError::error(
            "missing_mom_money_constant",
            "currency_constants:MOM_MONEY",
            "new-game initialization requires the exact MOM_MONEY constant",
        )),
    }
    for (set_name, rules) in [
        ("random", &data.battle_reward_rules.mom_random_items),
        (
            "progression",
            &data.battle_reward_rules.mom_progression_items,
        ),
    ] {
        for (index, rule) in rules.iter().enumerate() {
            let subject = format!("battle_reward_rules:mom_{set_name}_items:{index}");
            match rule.kind {
                MomPurchaseKind::Item => {
                    if !data.items.contains_key(&rule.target) {
                        diagnostics.push(VerificationError::error(
                            "unknown_mom_purchase_item",
                            subject,
                            format!(
                                "Mom purchase rule references unknown item '{}'",
                                rule.target
                            ),
                        ));
                    }
                }
                MomPurchaseKind::Doll => {}
            }
        }
    }
}

fn battle_reward_rules_issue_diagnostic(issue: BattleRewardRulesIssue) -> VerificationError {
    let message = match issue {
        BattleRewardRulesIssue::MissingMaxLevel => {
            "battle reward rules maxLevel must be nonzero".to_string()
        }
        BattleRewardRulesIssue::InvalidWildExpDivisor { .. } => {
            "battle reward rules wildExpDivisor must be positive".to_string()
        }
        BattleRewardRulesIssue::InvalidTrainerExpNumerator { .. } => {
            "battle reward rules trainerExpNumerator must be positive".to_string()
        }
        BattleRewardRulesIssue::InvalidTrainerExpDenominator { .. } => {
            "battle reward rules trainerExpDenominator must be positive".to_string()
        }
        BattleRewardRulesIssue::InvalidMomPurchaseRules { ref reason } => {
            format!("battle reward rules Mom purchase data is invalid: {reason}")
        }
    };
    VerificationError::error(
        "invalid_battle_reward_rule",
        issue.field().subject(),
        message,
    )
}

fn verify_battle_escape_rules(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    diagnostics.extend(
        battle_escape_rules_issues(&data.battle_escape_rules, !data.pokemon.is_empty())
            .into_iter()
            .map(battle_escape_rules_issue_diagnostic),
    );
}

fn battle_escape_rules_issue_diagnostic(issue: BattleEscapeRulesIssue) -> VerificationError {
    match issue {
        BattleEscapeRulesIssue::Missing => VerificationError::error(
            "missing_battle_escape_rules",
            "battle_escape_rules",
            "battle escape rules must be declared when Pokemon exist",
        ),
        BattleEscapeRulesIssue::MissingPlayerSpeedMultiplier => VerificationError::error(
            "invalid_battle_escape_rule",
            "battle_escape_rules:player_speed_multiplier",
            "battle escape player speed multiplier must be nonzero",
        ),
        BattleEscapeRulesIssue::MissingEnemySpeedDivisor => VerificationError::error(
            "invalid_battle_escape_rule",
            "battle_escape_rules:enemy_speed_divisor",
            "battle escape enemy speed divisor must be nonzero",
        ),
        BattleEscapeRulesIssue::InvalidRngRollValues { .. } => VerificationError::error(
            "invalid_battle_escape_rule",
            "battle_escape_rules:rng_roll_values",
            "battle escape rng roll values must be in 1..=256",
        ),
    }
}

fn verify_step_event_rules(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    diagnostics.extend(
        step_event_rules_issues(&data.step_event_rules)
            .into_iter()
            .map(step_event_rules_issue_diagnostic),
    );
}

fn step_event_rules_issue_diagnostic(issue: StepEventRulesIssue) -> VerificationError {
    match issue {
        StepEventRulesIssue::MissingPoisonStepInterval => VerificationError::error(
            "invalid_step_event_rule",
            "step_event_rules:poison_step_interval",
            "step event rules poisonStepInterval must be nonzero",
        ),
        StepEventRulesIssue::InvalidPoisonStatus { .. } => VerificationError::error(
            "invalid_step_event_rule",
            "step_event_rules:poison_status",
            "step event rules poisonStatus must be an exact nonempty status id",
        ),
        StepEventRulesIssue::InvalidEggNickname { .. } => VerificationError::error(
            "invalid_step_event_rule",
            "step_event_rules:egg_nickname",
            "step event rules eggNickname must be an exact nonempty nickname token",
        ),
        StepEventRulesIssue::HappinessTargetOutsideMask { .. } => VerificationError::error(
            "invalid_step_event_rule",
            "step_event_rules:happiness_step_counter_target",
            "step event rules happinessStepCounterTarget must fit inside happinessStepCounterMask",
        ),
    }
}

fn verify_fishing(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    let referenced_groups: Vec<(String, String)> = data
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
                .map(|group| (map_name.clone(), group.to_string()))
        })
        .collect();
    let species_ids: BTreeSet<String> = data.pokemon.keys().cloned().collect();
    diagnostics.extend(
        fishing_catalog_issues(&data.fishing, &referenced_groups, &data.items, &species_ids)
            .into_iter()
            .map(fishing_catalog_issue_diagnostic),
    );
}

fn fishing_catalog_issue_diagnostic(issue: FishingCatalogIssue) -> VerificationError {
    match issue {
        FishingCatalogIssue::MissingCatalog { map_name, group } => VerificationError::error(
            "missing_fishing_catalog",
            map_name,
            format!("map references fishing group '{group}' but no fishing catalog is loaded"),
        ),
        FishingCatalogIssue::MissingRodItems => VerificationError::error(
            "missing_fishing_rod_items",
            "fishing",
            "fishing catalog must declare exact item id to rod rules",
        ),
        FishingCatalogIssue::InvalidRodItemId { item_id } => VerificationError::error(
            "invalid_fishing_rod_item_id",
            format!("fishing:rod_items:{item_id}"),
            "fishing rod item rule item id must be an exact nonempty item id",
        ),
        FishingCatalogIssue::InvalidRodItemRod { item_id, rod } => VerificationError::error(
            "invalid_fishing_rod_item_rod",
            format!("fishing:rod_items:{item_id}"),
            format!("fishing rod item rule rod id must be exact and nonempty, found {rod:?}"),
        ),
        FishingCatalogIssue::UnknownRodItemRod { item_id, rod } => VerificationError::error(
            "unknown_fishing_rod_item_rod",
            format!("fishing:rod_items:{item_id}"),
            format!("fishing rod item rule references unknown rod '{rod}'"),
        ),
        FishingCatalogIssue::UnknownRodItemId { item_id } => VerificationError::error(
            "unknown_fishing_rod_item_id",
            format!("fishing:rod_items:{item_id}"),
            format!("fishing rod item rule references missing item id '{item_id}'"),
        ),
        FishingCatalogIssue::UnusableRodItem { item_id } => VerificationError::error(
            "unusable_fishing_rod_item",
            format!("fishing:rod_items:{item_id}"),
            format!("fishing rod item rule references item '{item_id}' that is not field usable"),
        ),
        FishingCatalogIssue::InvalidMapFishingGroup { map_name, group } => {
            VerificationError::error(
                "invalid_map_fishing_group",
                map_name,
                format!("map fishing group '{group}' must be an exact nonempty fish group id"),
            )
        }
        FishingCatalogIssue::UnknownMapFishingGroup { map_name, group } => {
            VerificationError::error(
                "unknown_map_fishing_group",
                map_name,
                format!("map references missing fishing group '{group}'"),
            )
        }
        FishingCatalogIssue::InvalidFishingGroupId { group_id } => VerificationError::error(
            "invalid_fishing_group_id",
            group_id,
            "fishing group id must be exact and nonempty",
        ),
        FishingCatalogIssue::InvalidFishingGroupSourceIndex { group_id } => {
            VerificationError::error(
                "invalid_fishing_group_source_index",
                group_id,
                "fishing group source index must be nonzero",
            )
        }
        FishingCatalogIssue::DuplicateFishingGroupSourceIndex {
            group_id,
            source_index,
        } => VerificationError::error(
            "duplicate_fishing_group_source_index",
            group_id,
            format!("fishing group source index {source_index} is duplicated"),
        ),
        FishingCatalogIssue::InvalidFishingRod { group_id, rod } => VerificationError::error(
            "invalid_fishing_rod",
            group_id,
            format!("fishing group rod id must be exact and nonempty, found {rod:?}"),
        ),
        FishingCatalogIssue::UnknownFishingRod { group_id, rod } => VerificationError::error(
            "unknown_fishing_rod",
            group_id,
            format!("fishing group references unknown rod '{rod}'"),
        ),
        FishingCatalogIssue::EmptyFishingRodTable { group_id, rod } => VerificationError::error(
            "empty_fishing_rod_table",
            group_id,
            format!("fishing group rod '{rod}' must declare slots"),
        ),
        FishingCatalogIssue::InvalidFishingSlotThreshold {
            group_id,
            rod,
            slot_index,
            threshold,
        } => VerificationError::error(
            "invalid_fishing_slot_threshold",
            group_id,
            format!("fishing {rod} slot {slot_index} has invalid threshold {threshold}"),
        ),
        FishingCatalogIssue::UnorderedFishingSlotThreshold {
            group_id,
            rod,
            slot_index,
            threshold,
            previous,
        } => VerificationError::error(
            "unordered_fishing_slot_threshold",
            group_id,
            format!(
                "fishing {rod} slot {slot_index} threshold {threshold} is below previous {previous}"
            ),
        ),
        FishingCatalogIssue::IncompleteFishingRodTable {
            group_id,
            rod,
            last_threshold,
        } => VerificationError::error(
            "incomplete_fishing_rod_table",
            group_id,
            format!("fishing {rod} table must end at threshold 255, found {last_threshold}"),
        ),
        FishingCatalogIssue::InvalidFishingSlotLevel {
            group_id,
            rod,
            slot_index,
            level,
        } => VerificationError::error(
            "invalid_fishing_slot_level",
            group_id,
            format!("fishing {rod} slot {slot_index} has invalid direct species level {level}"),
        ),
        FishingCatalogIssue::MissingFishingSlotSpecies {
            group_id,
            rod,
            slot_index,
        } => VerificationError::error(
            "missing_fishing_slot_species",
            group_id,
            format!("fishing {rod} slot {slot_index} must declare species or time group"),
        ),
        FishingCatalogIssue::InvalidFishingSpecies { group_id, species } => {
            VerificationError::error(
                "invalid_fishing_species",
                group_id,
                format!("fishing slot species '{species}' must be an exact nonempty species id"),
            )
        }
        FishingCatalogIssue::UnknownFishingSpecies { group_id, species } => {
            VerificationError::error(
                "unknown_fishing_species",
                group_id,
                format!("fishing slot references missing species '{species}'"),
            )
        }
        FishingCatalogIssue::UnknownFishingTimeGroup {
            group_id,
            time_group,
        } => VerificationError::error(
            "unknown_fishing_time_group",
            group_id,
            format!("fishing slot references missing time group {time_group}"),
        ),
        FishingCatalogIssue::InvalidFishingTimeGroupSpecies {
            time_group,
            species,
        } => VerificationError::error(
            "invalid_fishing_time_group_species",
            format!("fishing:time_groups:{time_group}"),
            format!("fishing time group species '{species}' must be an exact nonempty species id"),
        ),
        FishingCatalogIssue::UnknownFishingTimeGroupSpecies {
            time_group,
            species,
        } => VerificationError::error(
            "unknown_fishing_time_group_species",
            format!("fishing:time_groups:{time_group}"),
            format!("fishing time group references missing species '{species}'"),
        ),
        FishingCatalogIssue::InvalidSwarmFlagBit {
            rule_id,
            daily_flag_bit,
        } => VerificationError::error(
            "invalid_fishing_swarm_flag_bit",
            format!("fishing:swarm_rules:{rule_id}"),
            format!("fishing swarm rule dailyFlagBit must be 0..=7, found {daily_flag_bit}"),
        ),
        FishingCatalogIssue::InvalidSwarmBaseGroup { rule_id } => VerificationError::error(
            "invalid_fishing_swarm_base_group",
            format!("fishing:swarm_rules:{rule_id}"),
            "fishing swarm baseGroup must be an exact nonempty fish group id",
        ),
        FishingCatalogIssue::UnknownSwarmBaseGroup {
            rule_id,
            base_group,
        } => VerificationError::error(
            "unknown_fishing_swarm_base_group",
            format!("fishing:swarm_rules:{rule_id}"),
            format!("fishing swarm rule references missing base group '{base_group}'"),
        ),
        FishingCatalogIssue::InvalidSwarmGroup { rule_id } => VerificationError::error(
            "invalid_fishing_swarm_group",
            format!("fishing:swarm_rules:{rule_id}"),
            "fishing swarm swarmGroup must be an exact nonempty fish group id",
        ),
        FishingCatalogIssue::UnknownSwarmGroup {
            rule_id,
            swarm_group,
        } => VerificationError::error(
            "unknown_fishing_swarm_group",
            format!("fishing:swarm_rules:{rule_id}"),
            format!("fishing swarm rule references missing swarm group '{swarm_group}'"),
        ),
        FishingCatalogIssue::DuplicateSwarmRule { rule_id } => VerificationError::error(
            "duplicate_fishing_swarm_rule",
            format!("fishing:swarm_rules:{rule_id}"),
            "fishing swarm rules must not repeat the same dailyFlagBit, swarm, and baseGroup",
        ),
        FishingCatalogIssue::InvalidFishingTimeGroupId { time_group } => VerificationError::error(
            "invalid_fishing_time_group_id",
            format!("fishing:time_groups:{time_group}"),
            "fishing time group id must be exact and nonempty",
        ),
        FishingCatalogIssue::InvalidSwarmRuleId { rule_id } => VerificationError::error(
            "invalid_fishing_swarm_rule_id",
            format!("fishing:swarm_rules:{rule_id}"),
            "fishing swarm rule id must be exact and nonempty",
        ),
    }
}

fn verify_field_moves(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    let move_ids: BTreeSet<String> = data.moves.keys().cloned().collect();
    diagnostics.extend(
        field_move_catalog_issues(&data.field_moves, &move_ids, &data.items)
            .into_iter()
            .map(field_move_catalog_issue_diagnostic),
    );
    verify_field_move_block_replacements(data, diagnostics);
    verify_story_key_rules(data, diagnostics);
    verify_field_box_items(data, diagnostics);
}

fn verify_story_key_rules(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    for (rule_id, expected_effect, rule) in [
        ("card_key", "CARD_KEY", &data.field_moves.card_key),
        (
            "basement_key",
            "BASEMENT_KEY",
            &data.field_moves.basement_key,
        ),
    ] {
        let subject = format!("field_moves:{rule_id}");
        if let Some(item) = data.items.get(&rule.item_id)
            && item.effect != expected_effect
        {
            diagnostics.push(VerificationError::error(
                "mismatched_story_key_effect",
                &subject,
                format!(
                    "story key item {} has effect {}, expected {expected_effect}",
                    rule.item_id, item.effect
                ),
            ));
        }
        let Some(module) = data.maps.get(&rule.map_name) else {
            diagnostics.push(VerificationError::error(
                "missing_story_key_map",
                &subject,
                format!("story key map {} is missing", rule.map_name),
            ));
            continue;
        };
        if !module.scripts.contains_key(&rule.target_script) {
            diagnostics.push(VerificationError::error(
                "missing_story_key_script",
                &subject,
                format!(
                    "story key target script {} is missing from {}",
                    rule.target_script, rule.map_name
                ),
            ));
        }
        match runtime_map_tile_bounds(module) {
            Some((width, height))
                if rule.target_tile.x >= 0
                    && rule.target_tile.y >= 0
                    && rule.target_tile.x < width
                    && rule.target_tile.y < height => {}
            Some((width, height)) => diagnostics.push(VerificationError::error(
                "story_key_target_out_of_bounds",
                &subject,
                format!(
                    "story key target ({}, {}) is outside {} bounds {width}x{height}",
                    rule.target_tile.x, rule.target_tile.y, rule.map_name
                ),
            )),
            None => diagnostics.push(VerificationError::error(
                "story_key_map_bounds_overflow",
                &subject,
                format!("story key map {} bounds overflow", rule.map_name),
            )),
        }
    }
}

fn verify_field_box_items(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    verify_decorations(data, diagnostics);
    for (rule_id, rule) in &data.field_box_items {
        if rule_id != &rule.item_id {
            diagnostics.push(VerificationError::error(
                "mismatched_field_box_item_rule_id",
                format!("field_box_items:{rule_id}"),
                format!(
                    "field box item rule key must match item_id '{}', found key '{rule_id}'",
                    rule.item_id
                ),
            ));
        }
        if !is_exact_pack_token(rule_id) {
            diagnostics.push(VerificationError::error(
                "invalid_field_box_item_rule_id",
                format!("field_box_items:{rule_id}"),
                "field box item rule id must be exact and nonempty",
            ));
        }
        if !is_exact_pack_token(&rule.item_id) {
            diagnostics.push(VerificationError::error(
                "invalid_field_box_item_id",
                format!("field_box_items:{rule_id}"),
                "field box item id must be exact and nonempty",
            ));
        }
        if !is_exact_pack_token(&rule.effect) {
            diagnostics.push(VerificationError::error(
                "invalid_field_box_item_effect",
                format!("field_box_items:{rule_id}"),
                "field box item effect must be exact and nonempty",
            ));
        }
        if !is_exact_pack_token(&rule.decoration_flag) {
            diagnostics.push(VerificationError::error(
                "invalid_field_box_decoration_flag",
                format!("field_box_items:{rule_id}"),
                "field box decoration flag must be exact and nonempty",
            ));
        }
        let Some(item) = data.items.get(&rule.item_id) else {
            diagnostics.push(VerificationError::error(
                "unknown_field_box_item",
                format!("field_box_items:{rule_id}"),
                format!(
                    "field box item rule references unknown item '{}'",
                    rule.item_id
                ),
            ));
            continue;
        };
        if item.effect != rule.effect {
            diagnostics.push(VerificationError::error(
                "mismatched_field_box_item_effect",
                format!("field_box_items:{rule_id}"),
                format!(
                    "field box item '{}' effect '{}' must match rule effect '{}'",
                    rule.item_id, item.effect, rule.effect
                ),
            ));
        }
        if !item.field_usable {
            diagnostics.push(VerificationError::error(
                "unusable_field_box_item",
                format!("field_box_items:{rule_id}"),
                format!("field box item '{}' must be field usable", rule.item_id),
            ));
        }
        if item.field_menu != "ITEMMENU_CURRENT" {
            diagnostics.push(VerificationError::error(
                "invalid_field_box_item_menu",
                format!("field_box_items:{rule_id}"),
                format!(
                    "field box item '{}' field_menu '{}' must be ITEMMENU_CURRENT",
                    rule.item_id, item.field_menu
                ),
            ));
        }
    }
}

fn verify_decorations(data: &GameDataSet, diagnostics: &mut Vec<VerificationError>) {
    const CATEGORY_ORDER: &[DecorationCategory] = &[
        DecorationCategory::Bed,
        DecorationCategory::Carpet,
        DecorationCategory::Plant,
        DecorationCategory::Poster,
        DecorationCategory::GameConsole,
        DecorationCategory::Ornament,
        DecorationCategory::BigDoll,
    ];
    if data.decorations.category_order != CATEGORY_ORDER {
        diagnostics.push(VerificationError::error(
            "invalid_decoration_category_order",
            "decorations:category_order",
            "decoration categories must preserve the seven-entry ASM menu order",
        ));
    }
    if data.decorations.decorations.len() != 45 {
        diagnostics.push(VerificationError::error(
            "invalid_decoration_count",
            "decorations:decorations",
            format!(
                "decoration catalog must contain all 45 ASM decorations, found {}",
                data.decorations.decorations.len()
            ),
        ));
    }

    let mut indices = BTreeSet::new();
    let mut ids = BTreeSet::new();
    let mut flags = BTreeSet::new();
    for (position, decoration) in data.decorations.decorations.iter().enumerate() {
        let subject = format!("decorations:decorations:{position}");
        if !indices.insert(decoration.index) {
            diagnostics.push(VerificationError::error(
                "duplicate_decoration_index",
                &subject,
                format!("duplicate decoration index {}", decoration.index),
            ));
        }
        if !is_exact_pack_token(&decoration.id) || !ids.insert(decoration.id.as_str()) {
            diagnostics.push(VerificationError::error(
                "invalid_decoration_id",
                &subject,
                format!("decoration id '{}' must be exact and unique", decoration.id),
            ));
        }
        if decoration.display_name.is_empty()
            || decoration.display_name.trim() != decoration.display_name
            || decoration.display_name.chars().any(char::is_control)
        {
            diagnostics.push(VerificationError::error(
                "invalid_decoration_display_name",
                &subject,
                "decoration display_name must be nonempty, unpadded, and printable",
            ));
        }
        let expected_action = match decoration.category {
            DecorationCategory::Bed => "SET_UP_BED",
            DecorationCategory::Carpet => "SET_UP_CARPET",
            DecorationCategory::Plant => "SET_UP_PLANT",
            DecorationCategory::Poster => "SET_UP_POSTER",
            DecorationCategory::GameConsole => "SET_UP_CONSOLE",
            DecorationCategory::Ornament => "SET_UP_DOLL",
            DecorationCategory::BigDoll => "SET_UP_BIG_DOLL",
        };
        if decoration.action != expected_action {
            diagnostics.push(VerificationError::error(
                "invalid_decoration_action",
                &subject,
                format!(
                    "decoration {} action '{}' must be {expected_action} for its category",
                    decoration.id, decoration.action
                ),
            ));
        }
        if !decoration.event_flag.starts_with("EVENT_DECO_")
            || !is_exact_pack_token(&decoration.event_flag)
            || !flags.insert(decoration.event_flag.as_str())
        {
            diagnostics.push(VerificationError::error(
                "invalid_decoration_event_flag",
                &subject,
                format!(
                    "decoration {} ownership flag '{}' must be an exact unique EVENT_DECO_ token",
                    decoration.id, decoration.event_flag
                ),
            ));
        }
        if decoration.sprite.is_empty()
            || decoration.sprite.trim() != decoration.sprite
            || decoration.sprite.chars().any(char::is_control)
        {
            diagnostics.push(VerificationError::error(
                "invalid_decoration_sprite",
                &subject,
                "decoration sprite must be exact, nonempty, and unpadded",
            ));
        }
    }
    for category in CATEGORY_ORDER {
        if !data
            .decorations
            .decorations
            .iter()
            .any(|decoration| decoration.category == *category)
        {
            diagnostics.push(VerificationError::error(
                "empty_decoration_category",
                "decorations:decorations",
                format!("decoration category {category:?} has no entries"),
            ));
        }
    }
}

fn verify_field_move_block_replacements(
    data: &GameDataSet,
    diagnostics: &mut Vec<VerificationError>,
) {
    verify_field_move_block_rule_replacements("cut", &data.field_moves.cut, data, diagnostics);
    verify_field_move_block_rule_replacements(
        "whirlpool",
        &data.field_moves.whirlpool,
        data,
        diagnostics,
    );
}

fn verify_field_move_block_rule_replacements(
    rule_name: &str,
    rule: &FieldMoveBlockRule,
    data: &GameDataSet,
    diagnostics: &mut Vec<VerificationError>,
) {
    if rule.move_id.is_empty() || rule.target_collisions.is_empty() || rule.replacements.is_empty()
    {
        return;
    }
    for (tileset_name, replacements) in &rule.replacements {
        let Some(tileset) = data.tilesets.get(tileset_name) else {
            diagnostics.push(VerificationError::error(
                "unknown_field_move_replacement_tileset",
                format!("field_moves:{rule_name}:replacements:{tileset_name}"),
                format!(
                    "field move '{}' replacement table references missing tileset {tileset_name}",
                    rule.move_id
                ),
            ));
            continue;
        };
        let Ok(collision) = tileset_collision_from_definition(tileset_name, tileset) else {
            continue;
        };
        for (block_id, replacement) in replacements {
            if collision.metatiles.get(*block_id as usize).is_none() {
                diagnostics.push(VerificationError::error(
                    "unknown_field_move_replacement_block",
                    format!("field_moves:{rule_name}:replacements:{tileset_name}:{block_id}"),
                    format!(
                        "field move '{}' replacement table references missing block {block_id:#04x} in tileset {tileset_name}",
                        rule.move_id
                    ),
                ));
            }
            let replacement_block_id = replacement.replacement_block_id;
            if collision
                .metatiles
                .get(replacement_block_id as usize)
                .is_none()
            {
                diagnostics.push(VerificationError::error(
                    "unknown_field_move_replacement_target_block",
                    format!("field_moves:{rule_name}:replacements:{tileset_name}:{block_id}"),
                    format!(
                        "field move '{}' replacement table writes missing block {replacement_block_id:#04x} in tileset {tileset_name}",
                        rule.move_id
                    ),
                ));
            }
        }
    }
}

fn field_move_catalog_issue_diagnostic(issue: FieldMoveCatalogIssue) -> VerificationError {
    match issue {
        FieldMoveCatalogIssue::InvalidMoveId { subject } => VerificationError::error(
            "invalid_field_move_id",
            subject,
            "field move id must be an exact nonempty move id",
        ),
        FieldMoveCatalogIssue::UnknownMoveId { subject, move_id } => VerificationError::error(
            "unknown_field_move_id",
            subject,
            format!("field move references missing move '{move_id}'"),
        ),
        FieldMoveCatalogIssue::InvalidBadgeRegion {
            subject,
            move_id,
            region,
        } => VerificationError::error(
            "invalid_field_move_badge_region",
            subject,
            format!("field move '{move_id}' badge region must be exact 'johto', found '{region}'"),
        ),
        FieldMoveCatalogIssue::InvalidBadgeIndex {
            subject,
            move_id,
            index,
        } => VerificationError::error(
            "invalid_field_move_badge_index",
            subject,
            format!("field move '{move_id}' Johto badge index must be 0..=7, found {index}"),
        ),
        FieldMoveCatalogIssue::MissingTargetCollisions { subject, move_id } => {
            VerificationError::error(
                "missing_field_move_target_collisions",
                subject,
                format!("field move '{move_id}' requires exact target collisions"),
            )
        }
        FieldMoveCatalogIssue::MissingReplacements { subject, move_id } => {
            VerificationError::error(
                "missing_field_move_replacements",
                subject,
                format!("field move '{move_id}' requires exact replacement rows"),
            )
        }
        FieldMoveCatalogIssue::InvalidReplacementTileset { subject } => VerificationError::error(
            "invalid_field_move_replacement_tileset",
            subject,
            "field move replacement tileset must be exact and nonempty",
        ),
        FieldMoveCatalogIssue::InvalidReplacementVariant { subject } => VerificationError::error(
            "invalid_field_move_replacement_variant",
            subject,
            "field move replacement variant must be exact and nonempty",
        ),
        FieldMoveCatalogIssue::InvalidReplacementBlock { subject, block_id } => {
            VerificationError::error(
                "invalid_field_move_replacement_block",
                subject,
                format!(
                    "field move replacement for block {block_id:#04x} must change the target block"
                ),
            )
        }
        FieldMoveCatalogIssue::InvalidEngineFlag { subject, move_id } => VerificationError::error(
            "invalid_field_move_engine_flag",
            subject,
            format!("field move '{move_id}' requires an exact engine flag"),
        ),
        FieldMoveCatalogIssue::InvalidEscapeItemId => VerificationError::error(
            "invalid_field_escape_item_id",
            "field_moves:escape_rope",
            "field escape item id must be exact and nonempty",
        ),
        FieldMoveCatalogIssue::InvalidEscapeItemMode => VerificationError::error(
            "invalid_field_escape_item_mode",
            "field_moves:escape_rope",
            "field escape item mode must be exact and nonempty",
        ),
        FieldMoveCatalogIssue::UnknownEscapeItemRule {
            item_id,
            escape_rope_mode,
        } => VerificationError::error(
            "unknown_field_escape_item_rule",
            "field_moves:escape_rope",
            format!(
                "field escape item rule references item '{item_id}' with mode '{escape_rope_mode}' not implemented by the item payload"
            ),
        ),
        FieldMoveCatalogIssue::UnusableEscapeItem { item_id } => VerificationError::error(
            "unusable_field_escape_item",
            "field_moves:escape_rope",
            format!("field escape item rule references item '{item_id}' that is not field usable"),
        ),
        FieldMoveCatalogIssue::MissingRepelItemPayload => VerificationError::error(
            "missing_field_repel_item_payload",
            "field_moves:repel",
            "field repel behavior requires at least one item with repel_steps",
        ),
        FieldMoveCatalogIssue::MissingUsableRepelItemPayload => VerificationError::error(
            "missing_usable_field_repel_item_payload",
            "field_moves:repel",
            "field repel behavior requires at least one field-usable item with repel_steps",
        ),
        FieldMoveCatalogIssue::InvalidFieldItemId { subject } => VerificationError::error(
            "invalid_field_item_id",
            subject,
            "field item id must be exact and nonempty",
        ),
        FieldMoveCatalogIssue::UnknownFieldItemId { subject, item_id } => VerificationError::error(
            "unknown_field_item_id",
            subject,
            format!("field item rule references unknown item '{item_id}'"),
        ),
        FieldMoveCatalogIssue::UnusableFieldItem { subject, item_id } => VerificationError::error(
            "unusable_field_item",
            subject,
            format!("field item rule references item '{item_id}' that is not field usable"),
        ),
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
    objects: Vec<ObjectEvent>,
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

const PLAYABILITY_RUNTIME_TILE_STRIDE: i16 = DEFAULT_RUNTIME_TILE_STRIDE;

fn map_playability_context(
    data: &GameDataSet,
    module: &MapModule,
    rules: &PlayabilityRules,
    diagnostics: &mut Vec<VerificationError>,
) -> Option<MapPlayabilityContext> {
    map_playability_context_from_parts(
        data,
        &module.id,
        &module.attributes,
        module.blocks.clone(),
        rules,
        diagnostics,
    )
}

fn map_playability_context_from_parts(
    data: &GameDataSet,
    map_name: &str,
    attributes: &MapAttributes,
    blocks: Vec<u16>,
    rules: &PlayabilityRules,
    diagnostics: &mut Vec<VerificationError>,
) -> Option<MapPlayabilityContext> {
    let map = OverworldMapData::from_attributes(map_name, attributes, blocks);
    let tileset = match data
        .tilesets
        .get(&attributes.tileset_name)
        .with_context(|| format!("missing tileset '{}'", attributes.tileset_name))
        .and_then(|tileset| tileset_collision_from_definition(&attributes.tileset_name, tileset))
    {
        Ok(tileset) => tileset,
        Err(error) => {
            diagnostics.push(map_validation_diagnostic(
                rules,
                "invalid_tileset_collision",
                map_name,
                error.to_string(),
            ));
            return None;
        }
    };
    let Some((width, height)) = map.checked_tile_bounds() else {
        diagnostics.push(map_validation_diagnostic(
            rules,
            "map_runtime_tile_bounds_overflow",
            map_name,
            "map runtime tile bounds overflow the supported coordinate range".to_string(),
        ));
        return None;
    };
    let Some(width) = i16::try_from(width).ok() else {
        diagnostics.push(map_validation_diagnostic(
            rules,
            "map_runtime_tile_bounds_overflow",
            map_name,
            "map runtime tile width overflows the supported coordinate range".to_string(),
        ));
        return None;
    };
    let Some(height) = i16::try_from(height).ok() else {
        diagnostics.push(map_validation_diagnostic(
            rules,
            "map_runtime_tile_bounds_overflow",
            map_name,
            "map runtime tile height overflows the supported coordinate range".to_string(),
        ));
        return None;
    };
    let mut component_by_tile = BTreeMap::new();
    let mut component_count = 0_usize;
    let aligned_tile_step =
        usize::try_from(PLAYABILITY_RUNTIME_TILE_STRIDE).expect("runtime tile stride is positive");
    for y in (0..height).step_by(aligned_tile_step) {
        for x in (0..width).step_by(aligned_tile_step) {
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
                    let Some(next) =
                        checked_move_by_stride(tile, direction, PLAYABILITY_RUNTIME_TILE_STRIDE)
                    else {
                        continue;
                    };
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

pub fn tileset_collision_from_definition(
    tileset_id: &str,
    definition: &TilesetDefinition,
) -> Result<TilesetCollision> {
    let ids = definition
        .collision
        .keys()
        .map(|key| parse_metatile_id(key).with_context(|| format!("parse metatile id '{key}'")))
        .collect::<Result<BTreeSet<_>>>()?;
    require_dense_metatile_ids(&ids, &format!("tileset '{tileset_id}' collision map"))?;
    let max_id = ids
        .iter()
        .copied()
        .max()
        .with_context(|| format!("tileset '{tileset_id}' collision map is empty"))?;
    let mut metatiles = vec![None; max_id + 1];
    for (id, quadrants) in &definition.collision {
        if quadrants.len() != 4 {
            anyhow::bail!(
                "tileset '{tileset_id}' metatile '{id}' has {} collision quadrants",
                quadrants.len()
            );
        }
        let index = parse_metatile_id(id)?;
        let mut collision = [0_u8; 4];
        for (quadrant, token) in quadrants.iter().enumerate() {
            collision[quadrant] = resolve_collision_token(token).with_context(|| {
                format!("unknown collision token {token} in tileset '{tileset_id}:{id}'")
            })?;
        }
        metatiles[index] = Some(MetatileCollision { collision });
    }
    Ok(TilesetCollision {
        metatiles: metatiles
            .into_iter()
            .collect::<Option<Vec<_>>>()
            .with_context(|| {
                format!("tileset '{tileset_id}' collision map has missing metatile ids")
            })?,
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
        "north" => connection_source_boundary_tiles(
            width,
            height,
            PLAYABILITY_RUNTIME_TILE_STRIDE,
            "north",
        )
        .into_iter()
        .find(|tile| context.component_at(*tile).is_some()),
        "south" => connection_source_boundary_tiles(
            width,
            height,
            PLAYABILITY_RUNTIME_TILE_STRIDE,
            "south",
        )
        .into_iter()
        .find(|tile| context.component_at(*tile).is_some()),
        "west" => {
            connection_source_boundary_tiles(width, height, PLAYABILITY_RUNTIME_TILE_STRIDE, "west")
                .into_iter()
                .find(|tile| context.component_at(*tile).is_some())
        }
        "east" => {
            connection_source_boundary_tiles(width, height, PLAYABILITY_RUNTIME_TILE_STRIDE, "east")
                .into_iter()
                .find(|tile| context.component_at(*tile).is_some())
        }
        _ => None,
    }
}

fn connection_source_boundary_tiles(
    width: u16,
    height: u16,
    stride_tiles: i16,
    direction: &str,
) -> Vec<TilePosition> {
    if stride_tiles <= 0 {
        return Vec::new();
    }
    let Some(width) = i16::try_from(width).ok() else {
        return Vec::new();
    };
    let Some(height) = i16::try_from(height).ok() else {
        return Vec::new();
    };
    let stride = stride_tiles;
    if width < stride || height < stride {
        return Vec::new();
    }
    let aligned_axis = |limit: i16| {
        (0..limit)
            .step_by(usize::try_from(stride).expect("positive runtime stride fits usize"))
            .collect::<Vec<_>>()
    };
    match direction {
        "north" => aligned_axis(width)
            .into_iter()
            .map(|x| TilePosition::new(x, 0))
            .collect(),
        "south" => {
            let y = height - stride;
            aligned_axis(width)
                .into_iter()
                .map(|x| TilePosition::new(x, y))
                .collect()
        }
        "west" => aligned_axis(height)
            .into_iter()
            .map(|y| TilePosition::new(0, y))
            .collect(),
        "east" => {
            let x = width - stride;
            aligned_axis(height)
                .into_iter()
                .map(|y| TilePosition::new(x, y))
                .collect()
        }
        _ => Vec::new(),
    }
}

fn connection_source_tile_for_target(
    context: &MapPlayabilityContext,
    connection: &MapConnection,
    target_attributes: &MapAttributes,
) -> Option<TilePosition> {
    let (width, height) = context.map.tile_bounds();
    connection_source_boundary_tiles(
        width,
        height,
        PLAYABILITY_RUNTIME_TILE_STRIDE,
        &connection.direction,
    )
    .into_iter()
    .find(|tile| {
        if context.component_at(*tile).is_none() {
            return false;
        }
        let Some(trigger_tile) = connection_trigger_tile_from_source(*tile, connection) else {
            return false;
        };
        connection_destination_tile_in_bounds(
            trigger_tile,
            &connection.direction,
            connection.offset,
            target_attributes,
        )
        .unwrap_or(false)
    })
}

fn connection_source_tile_and_component_for_target(
    context: &MapPlayabilityContext,
    connection: &MapConnection,
    target_attributes: &MapAttributes,
) -> Option<(TilePosition, usize)> {
    let tile = connection_source_tile_for_target(context, connection, target_attributes)?;
    let component = context.component_at(tile)?;
    Some((tile, component))
}

fn connection_trigger_tile_from_source(
    source: TilePosition,
    connection: &MapConnection,
) -> Option<TilePosition> {
    match connection.direction.as_str() {
        "north" => checked_move_by_stride(source, Direction::Up, PLAYABILITY_RUNTIME_TILE_STRIDE),
        "south" => checked_move_by_stride(source, Direction::Down, PLAYABILITY_RUNTIME_TILE_STRIDE),
        "west" => checked_move_by_stride(source, Direction::Left, PLAYABILITY_RUNTIME_TILE_STRIDE),
        "east" => checked_move_by_stride(source, Direction::Right, PLAYABILITY_RUNTIME_TILE_STRIDE),
        _ => Some(source),
    }
}
