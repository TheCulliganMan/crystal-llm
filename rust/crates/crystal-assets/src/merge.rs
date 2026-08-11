fn merge_kurt_apricorn_recipes(
    target: &mut KurtApricornRecipes,
    source: KurtApricornRecipes,
) -> Result<()> {
    for (apricorn, ball) in source {
        insert_kurt_apricorn_recipe(target, apricorn, ball)?;
    }
    Ok(())
}

fn insert_kurt_apricorn_recipe(
    target: &mut KurtApricornRecipes,
    apricorn: String,
    ball: String,
) -> Result<()> {
    if target.contains_key(&apricorn) {
        anyhow::bail!("duplicate Kurt apricorn recipe for apricorn '{apricorn}'");
    }
    validate_modpack_payload_token(&apricorn, "Kurt apricorn recipe apricorn id")?;
    validate_modpack_payload_token(&ball, "Kurt apricorn recipe ball id")?;
    target.insert(apricorn, ball);
    Ok(())
}

fn merge_dratini_move_sets(target: &mut DratiniMoveSets, source: DratiniMoveSets) -> Result<()> {
    for (mode, moves) in source {
        insert_dratini_move_set(target, mode, moves)?;
    }
    Ok(())
}

fn insert_dratini_move_set(
    target: &mut DratiniMoveSets,
    mode: u8,
    moves: Vec<String>,
) -> Result<()> {
    if target.contains_key(&mode) {
        anyhow::bail!("duplicate Dratini move set mode {mode}");
    }
    if moves.is_empty() {
        anyhow::bail!("Dratini move set mode {mode} must not be empty");
    }
    for move_id in &moves {
        validate_modpack_payload_token(move_id, "Dratini move set move id")?;
    }
    target.insert(mode, moves);
    Ok(())
}

fn merge_roaming_pokemon(
    target: &mut RoamingPokemonCatalog,
    source: RoamingPokemonCatalog,
) -> Result<()> {
    if !target.is_empty() {
        anyhow::bail!("duplicate roaming Pokemon catalog");
    }
    *target = source;
    Ok(())
}

fn insert_shuckie_gift(
    target: &mut Option<ShuckieGiftDefinition>,
    gift: ShuckieGiftDefinition,
) -> Result<()> {
    if target.is_some() {
        anyhow::bail!("duplicate Shuckie gift definition");
    }
    validate_modpack_payload_token(&gift.species, "Shuckie gift species id")?;
    if gift.level == 0 {
        anyhow::bail!("Shuckie gift level must be nonzero");
    }
    validate_modpack_payload_token(&gift.held_item, "Shuckie gift held item id")?;
    validate_exact_modpack_value(&gift.nickname, "Shuckie gift nickname")?;
    validate_exact_modpack_value(
        &gift.original_trainer_name,
        "Shuckie gift original trainer name",
    )?;
    validate_battle_table_token(
        &gift.got_today_engine_flag,
        "Shuckie gift got-today engine flag",
    )?;
    *target = Some(gift);
    Ok(())
}

fn insert_bug_contest_config(
    target: &mut Option<BugContestConfig>,
    config: BugContestConfig,
) -> Result<()> {
    if target.is_some() {
        anyhow::bail!("duplicate Bug-Catching Contest config");
    }
    if config.park_balls == 0 {
        anyhow::bail!("Bug-Catching Contest parkBalls must be nonzero");
    }
    if config.timer_seconds > 59 {
        anyhow::bail!("Bug-Catching Contest timerSeconds must be 0 through 59");
    }
    if config.selected_contestant_count == 0 {
        anyhow::bail!("Bug-Catching Contest selectedContestantCount must be nonzero");
    }
    if config.contestant_flags.len() < config.selected_contestant_count {
        anyhow::bail!(
            "Bug-Catching Contest selectedContestantCount must not exceed contestantFlags length"
        );
    }
    let mut seen_flags = BTreeSet::new();
    for (index, flag) in config.contestant_flags.iter().enumerate() {
        validate_battle_table_token(
            flag,
            &format!("Bug-Catching Contest contestant flag at index {index}"),
        )?;
        if !seen_flags.insert(flag.as_str()) {
            anyhow::bail!("duplicate Bug-Catching Contest contestant flag '{flag}'");
        }
    }
    *target = Some(config);
    Ok(())
}

fn insert_battle_tower_rules(
    target: &mut Option<BattleTowerRules>,
    rules: BattleTowerRules,
) -> Result<()> {
    if target.is_some() {
        anyhow::bail!("duplicate Battle Tower rules");
    }
    if rules.required_party_count == 0 {
        anyhow::bail!("Battle Tower requiredPartyCount must be nonzero");
    }
    if rules.challenge_streak_length == 0 {
        anyhow::bail!("Battle Tower challengeStreakLength must be nonzero");
    }
    if rules.level_group_size == 0 {
        anyhow::bail!("Battle Tower levelGroupSize must be nonzero");
    }
    if rules.minimum_level_group == 0 || rules.maximum_level_group < rules.minimum_level_group {
        anyhow::bail!("Battle Tower level group range must be nonzero and ordered");
    }
    validate_modpack_payload_token(
        &rules.party_count_failure_text,
        "Battle Tower partyCountFailureText id",
    )?;
    validate_modpack_payload_token(
        &rules.duplicate_species_failure_text,
        "Battle Tower duplicateSpeciesFailureText id",
    )?;
    validate_modpack_payload_token(
        &rules.duplicate_held_item_failure_text,
        "Battle Tower duplicateHeldItemFailureText id",
    )?;
    validate_modpack_payload_token(&rules.egg_failure_text, "Battle Tower eggFailureText id")?;
    for species_id in rules.banned_species.keys() {
        validate_modpack_payload_token(species_id, "Battle Tower bannedSpecies id")?;
    }
    *target = Some(rules);
    Ok(())
}

fn insert_happiness_data(target: &mut Option<HappinessData>, data: HappinessData) -> Result<()> {
    if target.is_some() {
        anyhow::bail!("duplicate happiness data table");
    }
    if data.changes.is_empty() {
        anyhow::bail!("happiness data requires at least one explicit change row");
    }
    let mut change_codes = BTreeSet::new();
    for (change_code, entry) in &data.changes {
        validate_battle_table_token(
            &entry.code,
            &format!("happiness data change code label for {change_code}"),
        )?;
        if !change_codes.insert(entry.code.as_str()) {
            anyhow::bail!("duplicate happiness change code '{}'", entry.code);
        }
    }
    if data.services.is_empty() {
        anyhow::bail!("happiness data requires explicit service probability tables");
    }
    for (routine, outcomes) in &data.services {
        validate_battle_table_token(routine, "happiness data service routine id")?;
        if outcomes.is_empty() {
            anyhow::bail!("happiness service '{routine}' requires at least one outcome");
        }
        for outcome in outcomes {
            if !data.changes.contains_key(&outcome.change_code) {
                anyhow::bail!(
                    "happiness service '{routine}' references missing change code {}",
                    outcome.change_code
                );
            }
        }
    }
    *target = Some(data);
    Ok(())
}

#[cfg(test)]
fn insert_exact_vec_table<T>(target: &mut Vec<T>, entries: Vec<T>, table_name: &str) -> Result<()> {
    if !target.is_empty() {
        anyhow::bail!("duplicate {table_name} table");
    }
    *target = entries;
    Ok(())
}

fn insert_token_string_vec_table(
    target: &mut Vec<String>,
    entries: Vec<String>,
    table_name: &str,
    entry_description: &str,
) -> Result<()> {
    if !target.is_empty() {
        anyhow::bail!("duplicate {table_name} table");
    }
    for entry in &entries {
        validate_battle_table_token(entry, entry_description)?;
    }
    *target = entries;
    Ok(())
}

fn insert_oak_rating_table(
    target: &mut Vec<OakRatingEntry>,
    entries: Vec<OakRatingEntry>,
) -> Result<()> {
    if !target.is_empty() {
        anyhow::bail!("duplicate Oak rating table");
    }
    let mut previous_limit = None;
    for (index, entry) in entries.iter().enumerate() {
        validate_modpack_payload_token(&entry.fanfare, "Oak rating fanfare id")?;
        validate_modpack_payload_token(&entry.text_label, "Oak rating text label id")?;
        if let Some(previous) = previous_limit
            && entry.caught_count_limit <= previous
        {
            anyhow::bail!("Oak rating entry {index} caught_count_limit must increase");
        }
        previous_limit = Some(entry.caught_count_limit);
    }
    *target = entries;
    Ok(())
}

fn insert_magikarp_length_table(
    target: &mut Vec<MagikarpLengthEntry>,
    entries: Vec<MagikarpLengthEntry>,
) -> Result<()> {
    if !target.is_empty() {
        anyhow::bail!("duplicate Magikarp length table");
    }
    let mut previous_threshold = None;
    for (index, entry) in entries.iter().enumerate() {
        if entry.divisor == 0 {
            anyhow::bail!("Magikarp length entry {index} divisor must be nonzero");
        }
        if let Some(previous) = previous_threshold
            && entry.threshold <= previous
        {
            anyhow::bail!("Magikarp length entry {index} threshold must increase");
        }
        previous_threshold = Some(entry.threshold);
    }
    *target = entries;
    Ok(())
}

fn insert_odd_egg_definitions(
    target: &mut Vec<OddEggDefinition>,
    definitions: Vec<OddEggDefinition>,
) -> Result<()> {
    if !target.is_empty() {
        anyhow::bail!("duplicate Odd Egg definitions table");
    }
    if !definitions.is_empty() {
        let total_probability = definitions
            .iter()
            .map(|definition| u32::from(definition.probability))
            .sum::<u32>();
        if total_probability != 100 {
            anyhow::bail!(
                "Odd Egg definition probabilities must total 100, got {total_probability}"
            );
        }
    }
    for (index, definition) in definitions.iter().enumerate() {
        validate_modpack_payload_token(&definition.species, "Odd Egg species id")?;
        if definition.moves.is_empty() || definition.moves.len() > 4 {
            anyhow::bail!("Odd Egg definition {index} must declare between 1 and 4 moves");
        }
        for move_id in &definition.moves {
            validate_modpack_payload_token(move_id, "Odd Egg move id")?;
        }
        if definition.probability == 0 {
            anyhow::bail!("Odd Egg definition {index} probability must be nonzero");
        }
        if definition.level == 0 || definition.level > 100 {
            anyhow::bail!("Odd Egg definition {index} level must be between 1 and 100");
        }
        validate_exact_modpack_value(&definition.nickname, "Odd Egg nickname")?;
        validate_exact_modpack_value(
            &definition.original_trainer_name,
            "Odd Egg original trainer name",
        )?;
    }
    *target = definitions;
    Ok(())
}

fn insert_exact_string_vec_table(
    target: &mut Vec<String>,
    entries: Vec<String>,
    table_name: &str,
    entry_description: &str,
) -> Result<()> {
    if !target.is_empty() {
        anyhow::bail!("duplicate {table_name} table");
    }
    for entry in &entries {
        validate_exact_modpack_value(entry, entry_description)?;
    }
    *target = entries;
    Ok(())
}

fn insert_exact_string_bundle(
    target: &mut String,
    bundle: String,
    bundle_name: &str,
    required_sections: &[&str],
) -> Result<()> {
    if !target.trim().is_empty() {
        anyhow::bail!("duplicate {bundle_name}");
    }
    validate_exact_modpack_value(&bundle, bundle_name)?;
    if let Some(issue) = runtime_bundle_issues(&bundle, required_sections)
        .into_iter()
        .next()
    {
        anyhow::bail!("{bundle_name} is invalid: {issue:?}");
    }
    *target = bundle;
    Ok(())
}

fn insert_capture_rules(target: &mut CaptureRules, rules: CaptureRules) -> Result<()> {
    if *target != CaptureRules::default() {
        anyhow::bail!("duplicate capture rules table");
    }
    for species in &rules.fast_ball_species {
        validate_modpack_payload_token(species, "capture fast ball species id")?;
    }
    for species in rules.heavy_ball_modifiers.keys() {
        validate_modpack_payload_token(species, "capture heavy ball species id")?;
    }
    for (ball_id, rule) in &rules.ball_rules {
        validate_capture_ball_rule(ball_id, rule)?;
    }
    for ball_id in &rules.guaranteed_capture_balls {
        validate_modpack_payload_token(ball_id, "guaranteed capture ball id")?;
    }
    for status in rules.status_bonus.keys() {
        validate_battle_table_token(status, "capture status bonus id")?;
    }
    *target = rules;
    Ok(())
}

fn insert_capture_wobble_probabilities(
    target: &mut Vec<CaptureWobbleProbability>,
    probabilities: Vec<CaptureWobbleProbability>,
) -> Result<()> {
    if !target.is_empty() {
        anyhow::bail!("duplicate capture wobble probability table");
    }
    if probabilities.is_empty() {
        anyhow::bail!("capture wobble probability table must not be empty");
    }
    let mut previous = 0;
    for probability in &probabilities {
        if probability.catch_rate == 0 {
            anyhow::bail!("capture wobble probability catch_rate must be nonzero");
        }
        if probability.catch_rate < previous {
            anyhow::bail!("capture wobble probability catch_rate values must be nondecreasing");
        }
        previous = probability.catch_rate;
    }
    if previous != u8::MAX {
        anyhow::bail!("capture wobble probability table must end at catch_rate 255");
    }
    *target = probabilities;
    Ok(())
}

fn validate_capture_ball_rule(ball_id: &str, rule: &CaptureBallRule) -> Result<()> {
    validate_modpack_payload_token(ball_id, "capture ball rule item id")?;
    if !rule.battle_type.is_empty() {
        validate_battle_table_token(&rule.battle_type, "capture battle type id")?;
    }
    if rule.multiplier_denominator == 0 {
        anyhow::bail!("capture ball rule '{ball_id}' multiplier denominator must not be zero");
    }
    Ok(())
}

fn insert_battle_escape_rules(
    target: &mut BattleEscapeRules,
    rules: BattleEscapeRules,
) -> Result<()> {
    if *target != BattleEscapeRules::default() {
        anyhow::bail!("duplicate battle escape rules table");
    }
    if rules.player_speed_multiplier == 0 {
        anyhow::bail!("battle escape player_speed_multiplier must be nonzero");
    }
    if rules.enemy_speed_divisor == 0 {
        anyhow::bail!("battle escape enemy_speed_divisor must be nonzero");
    }
    if rules.rng_roll_values == 0 || rules.rng_roll_values > u16::from(u8::MAX) + 1 {
        anyhow::bail!("battle escape rng_roll_values must be between 1 and 256");
    }
    *target = rules;
    Ok(())
}

fn insert_battle_reward_rules(
    target: &mut BattleRewardRules,
    rules: BattleRewardRules,
) -> Result<()> {
    if *target != BattleRewardRules::default() {
        anyhow::bail!("duplicate battle reward rules table");
    }
    if rules.max_level == 0 {
        anyhow::bail!("battle reward max_level must be nonzero");
    }
    if rules.wild_exp_divisor <= 0 {
        anyhow::bail!("battle reward wild_exp_divisor must be positive");
    }
    if rules.trainer_exp_numerator <= 0 {
        anyhow::bail!("battle reward trainer_exp_numerator must be positive");
    }
    if rules.trainer_exp_denominator <= 0 {
        anyhow::bail!("battle reward trainer_exp_denominator must be positive");
    }
    *target = rules;
    Ok(())
}

fn insert_battle_stat_multiplier_tables(
    target: &mut BattleStatMultiplierTables,
    tables: BattleStatMultiplierTables,
) -> Result<()> {
    if *target != BattleStatMultiplierTables::default() {
        anyhow::bail!("duplicate battle stat multiplier table");
    }
    validate_battle_stat_multiplier_table("stat", &tables.stat)?;
    validate_battle_stat_multiplier_table("accuracy", &tables.accuracy)?;
    *target = tables;
    Ok(())
}

fn validate_battle_stat_multiplier_table(
    table_name: &str,
    entries: &[BattleStatMultiplier],
) -> Result<()> {
    if entries.len() != 13 {
        anyhow::bail!("battle stat multiplier {table_name} table must contain 13 entries");
    }
    for (index, entry) in entries.iter().enumerate() {
        let stage = index as i8 - 6;
        if entry.numerator <= 0 {
            anyhow::bail!(
                "battle stat multiplier {table_name} stage {stage} numerator must be positive"
            );
        }
        if entry.denominator <= 0 {
            anyhow::bail!(
                "battle stat multiplier {table_name} stage {stage} denominator must be positive"
            );
        }
    }
    Ok(())
}

fn insert_step_event_rules(target: &mut StepEventRules, rules: StepEventRules) -> Result<()> {
    if *target != StepEventRules::default() {
        anyhow::bail!("duplicate step event rules table");
    }
    if rules.poison_step_interval == 0 {
        anyhow::bail!("step event poison_step_interval must be nonzero");
    }
    validate_battle_table_token(&rules.poison_status, "step event poison status id")?;
    validate_modpack_payload_token(&rules.egg_nickname, "step event egg nickname")?;
    if rules.happiness_step_counter_target > rules.happiness_step_counter_mask {
        anyhow::bail!("step event happiness_step_counter_target must be within mask");
    }
    *target = rules;
    Ok(())
}

fn insert_encounter_slot_tables(
    target: &mut EncounterSlotTables,
    tables: EncounterSlotTables,
) -> Result<()> {
    if *target != EncounterSlotTables::default() {
        anyhow::bail!("duplicate encounter slot table");
    }
    validate_required_encounter_slot_table(&tables, EncounterSurface::Grass)?;
    validate_required_encounter_slot_table(&tables, EncounterSurface::Water)?;
    for (surface_id, table) in &tables.tables {
        if surface_id == EncounterSurface::Grass.as_key()
            || surface_id == EncounterSurface::Water.as_key()
        {
            continue;
        }
        validate_battle_table_token(surface_id, "encounter slot custom surface id")?;
        validate_encounter_slot_table(surface_id, table)?;
    }
    *target = tables;
    Ok(())
}

fn validate_required_encounter_slot_table(
    tables: &EncounterSlotTables,
    surface: EncounterSurface,
) -> Result<()> {
    let Some(table) = tables.tables.get(surface.as_key()) else {
        anyhow::bail!("encounter slot table '{}' is required", surface.as_key());
    };
    validate_encounter_slot_table(surface.as_key(), table)
}

fn validate_encounter_slot_table(surface_id: &str, table: &[EncounterSlotChance]) -> Result<()> {
    if table.is_empty() {
        anyhow::bail!("encounter slot table '{surface_id}' must not be empty");
    }
    let mut previous_threshold = 0;
    let mut slots = BTreeSet::new();
    for entry in table {
        if entry.threshold == 0 || entry.threshold > 100 {
            anyhow::bail!(
                "encounter slot table '{surface_id}' threshold must be between 1 and 100"
            );
        }
        if entry.threshold < previous_threshold {
            anyhow::bail!("encounter slot table '{surface_id}' thresholds must be nondecreasing");
        }
        previous_threshold = entry.threshold;
        if !slots.insert(entry.slot) {
            anyhow::bail!("encounter slot table '{surface_id}' slot indexes must be unique");
        }
    }
    if previous_threshold != 100 {
        anyhow::bail!("encounter slot table '{surface_id}' must end at threshold 100");
    }
    Ok(())
}

fn insert_encounter_music_modifiers(
    target: &mut EncounterMusicModifiers,
    modifiers: EncounterMusicModifiers,
) -> Result<()> {
    if *target != EncounterMusicModifiers::default() {
        anyhow::bail!("duplicate encounter music modifier table");
    }
    if modifiers.modifiers.is_empty() {
        anyhow::bail!("encounter music modifiers table must not be empty");
    }
    for (music_id, modifier) in &modifiers.modifiers {
        validate_modpack_payload_token(music_id, "encounter music modifier id")?;
        if modifier.denominator == 0 {
            anyhow::bail!("encounter music modifier '{music_id}' denominator must not be zero");
        }
    }
    *target = modifiers;
    Ok(())
}

fn insert_move_priority_table(
    target: &mut MovePriorityTable,
    table: MovePriorityTable,
) -> Result<()> {
    if *target != MovePriorityTable::default() {
        anyhow::bail!("duplicate move priority table");
    }
    if table.base_priority < 0 {
        anyhow::bail!("move priority base_priority must not be negative");
    }
    for (move_effect, priority) in &table.effect_priorities {
        validate_battle_table_token(move_effect, "move priority effect id")?;
        if *priority < 0 {
            anyhow::bail!("move priority effect '{move_effect}' priority must not be negative");
        }
    }
    for entry in &table.move_priorities {
        validate_modpack_payload_token(&entry.r#move, "move priority move id")?;
        if entry.priority < 0 {
            anyhow::bail!(
                "move priority override '{}' priority must not be negative",
                entry.r#move
            );
        }
    }
    *target = table;
    Ok(())
}

fn insert_type_categories(target: &mut TypeCategories, categories: TypeCategories) -> Result<()> {
    if *target != TypeCategories::default() {
        anyhow::bail!("duplicate type category table");
    }
    for type_id in &categories.physical {
        validate_battle_table_token(type_id, "physical type category id")?;
    }
    for type_id in &categories.special {
        validate_battle_table_token(type_id, "special type category id")?;
    }
    for type_id in &categories.physical {
        if categories.special.iter().any(|entry| entry == type_id) {
            anyhow::bail!("type category id '{type_id}' must not be both physical and special");
        }
    }
    *target = categories;
    Ok(())
}

fn insert_type_effectiveness(
    target: &mut TypeEffectivenessTable,
    table: TypeEffectivenessTable,
) -> Result<()> {
    if *target != TypeEffectivenessTable::default() {
        anyhow::bail!("duplicate type effectiveness table");
    }
    validate_type_effectiveness_table("type effectiveness matchups", &table.matchups)?;
    validate_type_effectiveness_table(
        "Foresight type effectiveness matchups",
        &table.foresight_matchups,
    )?;
    *target = table;
    Ok(())
}

fn validate_type_effectiveness_table(
    description: &str,
    table: &BTreeMap<String, BTreeMap<String, crystal_core::battle::damage::TypeMultiplier>>,
) -> Result<()> {
    for (attacker, defenders) in table {
        validate_battle_table_token(attacker, &format!("{description} attacker id"))?;
        for (defender, multiplier) in defenders {
            validate_battle_table_token(defender, &format!("{description} defender id"))?;
            if multiplier.denominator == 0 {
                anyhow::bail!(
                    "{description} '{attacker}' into '{defender}' denominator must not be zero"
                );
            }
        }
    }
    Ok(())
}

fn insert_flee_mon_tables(target: &mut FleeMonTables, tables: FleeMonTables) -> Result<()> {
    if *target != FleeMonTables::default() {
        anyhow::bail!("duplicate flee mons table");
    }
    for (bucket_id, species_ids) in &tables.buckets {
        validate_flee_mon_bucket_id(bucket_id)?;
        if species_ids.is_empty() {
            anyhow::bail!("flee mons bucket '{bucket_id}' must not be empty");
        }
        for species_id in species_ids {
            validate_modpack_payload_token(species_id, "flee mons species id")?;
        }
    }
    *target = tables;
    Ok(())
}

fn validate_flee_mon_bucket_id(bucket_id: &str) -> Result<()> {
    if bucket_id.is_empty()
        || bucket_id.trim() != bucket_id
        || !bucket_id
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte == b'_')
    {
        anyhow::bail!(
            "flee mons bucket id '{bucket_id}' must be exact lowercase ASCII or underscore"
        );
    }
    Ok(())
}

fn insert_fishing_catalog(target: &mut FishingCatalog, catalog: FishingCatalog) -> Result<()> {
    if *target != FishingCatalog::default() {
        anyhow::bail!("duplicate fishing table");
    }
    validate_fishing_catalog(&catalog)?;
    *target = catalog;
    Ok(())
}

fn validate_fishing_catalog(catalog: &FishingCatalog) -> Result<()> {
    for (item_id, rod) in &catalog.rod_items {
        validate_modpack_payload_token(item_id, "fishing rod item id")?;
        validate_known_fishing_rod(rod, "fishing rod item rod")?;
    }
    for (group_id, group) in &catalog.groups {
        validate_battle_table_token(group_id, "fishing group id")?;
        validate_fishing_group(group_id, group)?;
    }
    for (time_group_id, entry) in &catalog.time_groups {
        validate_battle_table_token(time_group_id, "fishing time group id")?;
        validate_modpack_payload_token(&entry.day_species, "fishing time group day species id")?;
        validate_modpack_payload_token(
            &entry.night_species,
            "fishing time group night species id",
        )?;
    }
    let mut seen_swarm_rules = BTreeSet::new();
    for (rule_id, rule) in &catalog.swarm_rules {
        validate_battle_table_token(rule_id, "fishing swarm rule id")?;
        if rule.daily_flag_bit >= u8::BITS as u8 {
            anyhow::bail!("fishing swarm rule '{rule_id}' daily flag bit must be below 8");
        }
        validate_battle_table_token(&rule.base_group, "fishing swarm base group id")?;
        if !catalog.groups.contains_key(&rule.base_group) {
            anyhow::bail!(
                "fishing swarm rule '{rule_id}' base group '{}' must exist",
                rule.base_group
            );
        }
        validate_battle_table_token(&rule.swarm_group, "fishing swarm group id")?;
        if !catalog.groups.contains_key(&rule.swarm_group) {
            anyhow::bail!(
                "fishing swarm rule '{rule_id}' swarm group '{}' must exist",
                rule.swarm_group
            );
        }
        if !seen_swarm_rules.insert((rule.daily_flag_bit, rule.swarm, rule.base_group.as_str())) {
            anyhow::bail!("duplicate fishing swarm rule for '{rule_id}'");
        }
    }
    Ok(())
}

fn validate_fishing_group(group_id: &str, group: &FishingGroup) -> Result<()> {
    for (rod, table) in &group.rod_tables {
        validate_known_fishing_rod(rod, "fishing rod table id")?;
        validate_fishing_rod_table(group_id, rod, table)?;
    }
    Ok(())
}

fn validate_fishing_rod_table(group_id: &str, rod: &str, table: &RodTable) -> Result<()> {
    if table.slots.is_empty() {
        anyhow::bail!("fishing group '{group_id}' rod '{rod}' slots must not be empty");
    }
    let mut previous_threshold = 0;
    for (slot_index, slot) in table.slots.iter().enumerate() {
        if slot.threshold == 0 {
            anyhow::bail!(
                "fishing group '{group_id}' rod '{rod}' slot {slot_index} threshold must be nonzero"
            );
        }
        if slot.threshold < previous_threshold {
            anyhow::bail!(
                "fishing group '{group_id}' rod '{rod}' thresholds must be nondecreasing"
            );
        }
        previous_threshold = slot.threshold;
        if slot.species.is_some() && slot.level == 0 {
            anyhow::bail!(
                "fishing group '{group_id}' rod '{rod}' slot {slot_index} level must be nonzero"
            );
        }
        if slot.species.is_none() && slot.time_group.is_none() {
            anyhow::bail!(
                "fishing group '{group_id}' rod '{rod}' slot {slot_index} must define species or time group"
            );
        }
        if let Some(species) = slot.species.as_deref() {
            validate_modpack_payload_token(species, "fishing slot species id")?;
        }
        if let Some(time_group) = slot.time_group.as_deref() {
            validate_battle_table_token(time_group, "fishing slot time group id")?;
        }
    }
    if previous_threshold != u8::MAX {
        anyhow::bail!("fishing group '{group_id}' rod '{rod}' must end at threshold 255");
    }
    Ok(())
}

fn validate_known_fishing_rod(rod: &str, description: &str) -> Result<()> {
    validate_battle_table_token(rod, description)?;
    if !FISHING_RODS.contains(&rod) {
        anyhow::bail!("{description} '{rod}' must be a known fishing rod");
    }
    Ok(())
}

fn insert_field_move_catalog(
    target: &mut FieldMoveCatalog,
    catalog: FieldMoveCatalog,
) -> Result<()> {
    if *target != FieldMoveCatalog::default() {
        anyhow::bail!("duplicate field moves table");
    }
    validate_field_move_block_rule("field_moves:cut", &catalog.cut, false)?;
    validate_field_move_block_rule("field_moves:whirlpool", &catalog.whirlpool, false)?;
    validate_field_move_flag_rule("field_moves:strength", &catalog.strength)?;
    validate_field_move_flag_rule("field_moves:flash", &catalog.flash)?;
    validate_field_move_travel_rule("field_moves:surf", &catalog.surf, false)?;
    validate_field_move_travel_rule("field_moves:waterfall", &catalog.waterfall, true)?;
    validate_field_move_rule("field_moves:fly", &catalog.fly)?;
    validate_field_move_move_rule("field_moves:dig", &catalog.dig, false)?;
    validate_field_move_move_rule("field_moves:teleport", &catalog.teleport, false)?;
    validate_field_move_move_rule("field_moves:headbutt", &catalog.headbutt, true)?;
    validate_field_move_move_rule("field_moves:rock_smash", &catalog.rock_smash, false)?;
    validate_field_move_move_rule("field_moves:sweet_scent", &catalog.sweet_scent, false)?;
    validate_field_escape_item_rule(&catalog.escape_rope)?;
    validate_field_item_rule("field_moves:bicycle", &catalog.bicycle)?;
    validate_field_item_rule("field_moves:itemfinder", &catalog.itemfinder)?;
    validate_field_item_rule("field_moves:squirtbottle", &catalog.squirtbottle)?;
    validate_field_item_rule("field_moves:coin_case", &catalog.coin_case)?;
    validate_field_item_rule("field_moves:blue_card", &catalog.blue_card)?;
    validate_field_item_rule("field_moves:town_map", &catalog.town_map)?;
    validate_field_item_rule("field_moves:pokegear", &catalog.pokegear)?;
    *target = catalog;
    Ok(())
}

fn insert_field_box_items(
    target: &mut BTreeMap<String, FieldBoxItemRule>,
    rules: BTreeMap<String, FieldBoxItemRule>,
) -> Result<()> {
    for (rule_id, rule) in rules {
        validate_field_box_item_rule(&rule_id, &rule)?;
        if target.insert(rule_id.clone(), rule).is_some() {
            anyhow::bail!("duplicate field box item rule {rule_id}");
        }
    }
    Ok(())
}

fn validate_field_box_item_rule(rule_id: &str, rule: &FieldBoxItemRule) -> Result<()> {
    validate_modpack_payload_token(rule_id, "field_box_items rule id")?;
    validate_modpack_payload_token(&rule.item_id, "field_box_items item id")?;
    validate_modpack_payload_token(&rule.effect, "field_box_items effect id")?;
    validate_battle_table_token(&rule.decoration_flag, "field_box_items decoration flag id")?;
    if rule_id != rule.item_id {
        anyhow::bail!(
            "field_box_items rule key {rule_id} must match item_id {}",
            rule.item_id
        );
    }
    Ok(())
}

fn insert_runtime_title_screen(
    target: &mut RuntimeTitleScreen,
    title_screen: RuntimeTitleScreen,
) -> Result<()> {
    if *target != RuntimeTitleScreen::default() {
        anyhow::bail!("duplicate runtime title screen payload");
    }
    validate_runtime_title_screen(&title_screen)?;
    *target = title_screen;
    Ok(())
}

fn validate_runtime_title_screen(title_screen: &RuntimeTitleScreen) -> Result<()> {
    if title_screen.new_game_spawn_identifier.is_none() {
        anyhow::bail!("runtime_title_screen requires new_game_spawn_identifier");
    }
    let Some(title_music) = &title_screen.title_music else {
        anyhow::bail!("runtime_title_screen requires title_music");
    };
    validate_modpack_payload_token(title_music, "runtime_title_screen title_music")?;
    Ok(())
}

fn validate_field_move_rule(subject: &str, rule: &FieldMoveRule) -> Result<()> {
    validate_modpack_payload_token(&rule.move_id, &format!("{subject} move id"))?;
    validate_field_move_badge(subject, &rule.move_id, &rule.badge)
}

fn validate_field_move_move_rule(
    subject: &str,
    rule: &FieldMoveMoveRule,
    require_target_collisions: bool,
) -> Result<()> {
    validate_modpack_payload_token(&rule.move_id, &format!("{subject} move id"))?;
    if require_target_collisions && rule.target_collisions.is_empty() {
        anyhow::bail!("{subject} target collisions must not be empty");
    }
    Ok(())
}

fn validate_field_move_block_rule(
    subject: &str,
    rule: &FieldMoveBlockRule,
    require_target_collisions: bool,
) -> Result<()> {
    validate_modpack_payload_token(&rule.move_id, &format!("{subject} move id"))?;
    validate_field_move_badge(subject, &rule.move_id, &rule.badge)?;
    if require_target_collisions && rule.target_collisions.is_empty() {
        anyhow::bail!("{subject} target collisions must not be empty");
    }
    if rule.replacements.is_empty() {
        anyhow::bail!("{subject} replacements must not be empty");
    }
    for (tileset, blocks) in &rule.replacements {
        validate_battle_table_token(tileset, &format!("{subject} replacement tileset id"))?;
        for (block_id, replacement) in blocks {
            validate_battle_table_token(
                &replacement.variant,
                &format!("{subject} replacement variant id"),
            )?;
            if replacement.replacement_block_id == *block_id {
                anyhow::bail!("{subject} replacement block {block_id} must change block id");
            }
        }
    }
    Ok(())
}

fn validate_field_move_flag_rule(subject: &str, rule: &FieldMoveFlagRule) -> Result<()> {
    validate_modpack_payload_token(&rule.move_id, &format!("{subject} move id"))?;
    validate_field_move_badge(subject, &rule.move_id, &rule.badge)?;
    validate_battle_table_token(&rule.engine_flag, &format!("{subject} engine flag id"))
}

fn validate_field_move_travel_rule(
    subject: &str,
    rule: &FieldMoveTravelRule,
    require_target_collisions: bool,
) -> Result<()> {
    validate_modpack_payload_token(&rule.move_id, &format!("{subject} move id"))?;
    validate_field_move_badge(subject, &rule.move_id, &rule.badge)?;
    if require_target_collisions && rule.target_collisions.is_empty() {
        anyhow::bail!("{subject} target collisions must not be empty");
    }
    Ok(())
}

fn validate_field_escape_item_rule(rule: &FieldEscapeItemRule) -> Result<()> {
    validate_modpack_payload_token(&rule.item_id, "field_moves:escape_rope item id")?;
    validate_battle_table_token(
        &rule.escape_rope_mode,
        "field_moves:escape_rope escape mode id",
    )
}

fn validate_field_item_rule(subject: &str, rule: &FieldItemRule) -> Result<()> {
    validate_modpack_payload_token(&rule.item_id, &format!("{subject} item id"))
}

fn validate_field_move_badge(
    subject: &str,
    move_id: &str,
    badge: &crystal_core::systems::field_moves::FieldMoveBadgeRequirement,
) -> Result<()> {
    if badge.region != "johto" {
        anyhow::bail!("{subject} badge region for '{move_id}' must be johto");
    }
    if badge.index >= 8 {
        anyhow::bail!("{subject} badge index for '{move_id}' must be below 8");
    }
    Ok(())
}

fn insert_initialize_events(
    target: &mut InitializeEventsConfig,
    config: InitializeEventsConfig,
) -> Result<()> {
    if *target != InitializeEventsConfig::default() {
        anyhow::bail!("duplicate initialize events table");
    }
    for flag in config.event_flags.iter().chain(config.engine_flags.iter()) {
        validate_battle_table_token(flag, "initialize event flag id")?;
    }
    for (sprite, replacement) in &config.variable_sprites {
        validate_modpack_payload_token(sprite, "initialize variable sprite id")?;
        validate_modpack_payload_token(replacement, "initialize variable sprite replacement id")?;
    }
    *target = config;
    Ok(())
}

fn insert_story_event_script_constants(
    target: &mut StoryEventScriptConstants,
    constants: StoryEventScriptConstants,
) -> Result<()> {
    if *target != StoryEventScriptConstants::default() {
        anyhow::bail!("duplicate story event script constants table");
    }
    for key in constants.global.keys() {
        validate_modpack_payload_token(key, "story event global constant id")?;
    }
    for (map_name, map_constants) in &constants.maps {
        validate_map_reference_token(map_name, "story event map id")?;
        for key in map_constants.keys() {
            validate_modpack_payload_token(key, "story event map constant id")?;
        }
    }
    *target = constants;
    Ok(())
}

fn insert_weather_modifiers(
    target: &mut WeatherModifiers,
    modifiers: WeatherModifiers,
) -> Result<()> {
    if *target != WeatherModifiers::default() {
        anyhow::bail!("duplicate weather modifier table");
    }
    for (weather, type_modifiers) in &modifiers.type_modifiers {
        validate_battle_table_token(weather, "weather type modifier id")?;
        for (move_type, multiplier) in type_modifiers {
            validate_battle_table_token(move_type, "weather move type id")?;
            if multiplier.denominator == 0 {
                anyhow::bail!(
                    "weather type modifier '{weather}' for '{move_type}' denominator must not be zero"
                );
            }
        }
    }
    for (weather, move_effect_modifiers) in &modifiers.move_effect_modifiers {
        validate_battle_table_token(weather, "weather move effect modifier id")?;
        for (move_effect, multiplier) in move_effect_modifiers {
            validate_battle_table_token(move_effect, "weather move effect id")?;
            if multiplier.denominator == 0 {
                anyhow::bail!(
                    "weather move effect modifier '{weather}' for '{move_effect}' denominator must not be zero"
                );
            }
        }
    }
    *target = modifiers;
    Ok(())
}

fn validate_battle_table_token(value: &str, description: &str) -> Result<()> {
    if value.is_empty()
        || value.trim() != value
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
    {
        anyhow::bail!("{description} '{value}' must be exact ASCII alphanumeric or underscore");
    }
    Ok(())
}

fn validate_modpack_payload_token(value: &str, description: &str) -> Result<()> {
    validate_battle_table_token(value, description)?;
    validate_no_reserved_payload_token(value, description)
}

fn validate_script_label_payload_token(value: &str, description: &str) -> Result<()> {
    if !is_exact_script_label_reference_token(value) {
        anyhow::bail!("{description} '{value}' must be an exact script label token");
    }
    validate_no_reserved_payload_token(value, description)
}

fn validate_no_reserved_payload_token(value: &str, description: &str) -> Result<()> {
    let lowered = value.to_ascii_lowercase();
    if lowered.starts_with("fallback") || lowered.starts_with("legacy") {
        anyhow::bail!("{description} '{value}' uses reserved modpack payload prefix");
    }
    Ok(())
}

fn merge_currency_constants_payload(target: &mut CurrencyCatalog, payload: Value) -> Result<()> {
    let constants: CurrencyCatalog = serde_json::from_value(payload)?;
    for (constant, value) in constants.0 {
        insert_currency_constant(target, constant, value)?;
    }
    Ok(())
}

fn merge_currency_constants(target: &mut CurrencyCatalog, source: &CurrencyCatalog) -> Result<()> {
    for (constant, value) in &source.0 {
        insert_currency_constant(target, constant.clone(), *value)?;
    }
    Ok(())
}

fn insert_currency_constant(
    target: &mut CurrencyCatalog,
    constant: String,
    value: u32,
) -> Result<()> {
    validate_battle_table_token(&constant, "currency constant")?;
    if target.0.contains_key(&constant) {
        anyhow::bail!("duplicate currency constant '{constant}'");
    }
    target.0.insert(constant, value);
    Ok(())
}

fn merge_pokegear_landmarks_payload(
    target: &mut PokegearLandmarksPayload,
    payload: Value,
) -> Result<()> {
    let payload: PokegearLandmarksPayload = serde_json::from_value(payload)?;
    merge_pokegear_landmarks(target, &payload)
}

fn merge_pokegear_landmarks(
    target: &mut PokegearLandmarksPayload,
    payload: &PokegearLandmarksPayload,
) -> Result<()> {
    for landmark in &payload.landmarks {
        validate_battle_table_token(&landmark.constant, "Pokegear landmark constant")?;
        if !landmark.constant.starts_with("LANDMARK_") {
            anyhow::bail!(
                "Pokegear landmark constant '{}' must use exact LANDMARK_* id",
                landmark.constant
            );
        }
        validate_battle_table_token(&landmark.label, "Pokegear landmark label")?;
        validate_exact_modpack_value(&landmark.name, "Pokegear landmark name")?;
        validate_battle_table_token(&landmark.region, "Pokegear landmark region")?;
        if target
            .landmarks
            .iter()
            .find(|existing| existing.constant == landmark.constant)
            .is_some()
        {
            anyhow::bail!(
                "duplicate Pokegear landmark constant '{}'",
                landmark.constant
            );
        }
        target.landmarks.push(landmark.clone());
    }
    for (map, landmark) in &payload.map_to_landmark {
        validate_battle_table_token(map, "Pokegear landmark map assignment")?;
        validate_battle_table_token(landmark, "Pokegear landmark map assignment target")?;
        if !landmark.starts_with("LANDMARK_") {
            anyhow::bail!(
                "Pokegear landmark map assignment target '{landmark}' must use exact LANDMARK_* id"
            );
        }
        if target.map_to_landmark.contains_key(map) {
            anyhow::bail!("duplicate Pokegear landmark map assignment for map '{map}'");
        }
        target.map_to_landmark.insert(map.clone(), landmark.clone());
    }
    Ok(())
}

fn merge_runtime_spawn_points(
    target: &mut BTreeMap<String, RuntimeSpawnPoint>,
    source: BTreeMap<String, RuntimeSpawnPoint>,
) -> Result<()> {
    for (key, spawn) in source {
        validate_exact_modpack_key(&key, "runtime spawn point")?;
        if key.parse::<u16>().ok() != Some(spawn.identifier) {
            anyhow::bail!(
                "runtime spawn point key '{key}' does not match identifier {}",
                spawn.identifier
            );
        }
        validate_battle_table_token(&spawn.map_constant, "runtime spawn point map constant")?;
        validate_battle_table_token(&spawn.map_name, "runtime spawn point map name")?;
        validate_battle_table_token(&spawn.group_name, "runtime spawn point group name")?;
        if !runtime_spawn_subtiles_are_valid(&spawn) {
            anyhow::bail!(
                "runtime spawn point '{key}' subtile ({}, {}) must be in range 0..{}",
                spawn.subtile_x,
                spawn.subtile_y,
                METATILE_WIDTH
            );
        }
        let expected_tile = checked_runtime_spawn_expected_tile(&spawn).with_context(|| {
            format!(
                "runtime spawn point '{key}' metatile/subtile coordinate ({}, {}) + ({}, {}) overflows runtime tile coordinates",
                spawn.metatile_x, spawn.metatile_y, spawn.subtile_x, spawn.subtile_y
            )
        })?;
        if spawn.tile_x != expected_tile.x || spawn.tile_y != expected_tile.y {
            anyhow::bail!(
                "runtime spawn point '{key}' tile ({}, {}) does not match metatile/subtile-derived tile ({}, {})",
                spawn.tile_x,
                spawn.tile_y,
                expected_tile.x,
                expected_tile.y
            );
        }
        if target.contains_key(&key) {
            anyhow::bail!("duplicate runtime spawn point '{key}'");
        }
        target.insert(key, spawn);
    }
    Ok(())
}

fn merge_runtime_map_metadata(
    target: &mut BTreeMap<String, RuntimeMapMetadata>,
    source: BTreeMap<String, RuntimeMapMetadata>,
) -> Result<()> {
    for (key, metadata) in source {
        validate_battle_table_token(&key, "runtime map metadata")?;
        if key != metadata.constant {
            anyhow::bail!(
                "runtime map metadata key '{key}' does not match record constant '{}'",
                metadata.constant
            );
        }
        validate_battle_table_token(&metadata.constant, "runtime map metadata constant")?;
        validate_battle_table_token(&metadata.name, "runtime map metadata name")?;
        validate_battle_table_token(&metadata.group_name, "runtime map metadata group name")?;
        validate_battle_table_token(&metadata.environment, "runtime map metadata environment")?;
        if target.contains_key(&key) {
            anyhow::bail!("duplicate runtime map metadata '{key}'");
        }
        target.insert(key, metadata);
    }
    Ok(())
}

fn merge_pc_strings(
    target: &mut BTreeMap<String, String>,
    source: BTreeMap<String, String>,
) -> Result<()> {
    for (key, value) in source {
        validate_modpack_payload_token(&key, "PC string key")?;
        validate_exact_modpack_text(&value, "PC string value")?;
        if target.contains_key(&key) {
            anyhow::bail!("duplicate PC string '{key}'");
        }
        target.insert(key, value);
    }
    Ok(())
}

fn merge_menu_icons(
    target: &mut BTreeMap<String, String>,
    source: BTreeMap<String, String>,
) -> Result<()> {
    for (key, value) in source {
        validate_modpack_payload_token(&key, "menu icon species id")?;
        validate_battle_table_token(&value, "menu icon id")?;
        if target.contains_key(&key) {
            anyhow::bail!("duplicate menu icon entry for species '{key}'");
        }
        target.insert(key, value);
    }
    Ok(())
}

fn merge_asm_text(target: &mut BTreeMap<String, String>, payload: Value) -> Result<()> {
    merge_asm_text_entries(target, parse_object_map::<String>(payload)?)
}

fn merge_asm_text_entries(
    target: &mut BTreeMap<String, String>,
    source: BTreeMap<String, String>,
) -> Result<()> {
    for (label, text) in source {
        validate_modpack_payload_token(&label, "ASM text label")?;
        validate_exact_modpack_multiline_text(
            &text,
            &format!("ASM text value for label '{label}'"),
        )?;
        if target.contains_key(&label) {
            anyhow::bail!("duplicate ASM text label '{label}'");
        }
        target.insert(label, text);
    }
    Ok(())
}

fn merge_frontpic_anim_programs(
    target: &mut BTreeMap<String, FrontpicAnimProgram>,
    payload: Value,
) -> Result<()> {
    let mut source = BTreeMap::new();
    for (species, program_payload) in parse_object_map::<Value>(payload)? {
        let program = parse_frontpic_anim_program_payload(&species, program_payload)?;
        source.insert(species, program);
    }
    merge_frontpic_anim_entries(target, source)
}

fn parse_frontpic_anim_program_payload(
    species: &str,
    payload: Value,
) -> Result<FrontpicAnimProgram> {
    let Some(program) = payload.as_object() else {
        anyhow::bail!("frontpic animation program for species '{species}' must be an object");
    };
    let allowed_program_keys = ["commands"];
    for key in program.keys() {
        if !allowed_program_keys.contains(&key.as_str()) {
            anyhow::bail!(
                "frontpic animation program for species '{species}' contains unknown field '{key}'"
            );
        }
    }
    let Some(commands_payload) = program.get("commands") else {
        anyhow::bail!("frontpic animation program for species '{species}' must declare commands");
    };
    let Some(commands_payload) = commands_payload.as_array() else {
        anyhow::bail!(
            "frontpic animation program for species '{species}' commands must be an array"
        );
    };
    let mut commands = Vec::with_capacity(commands_payload.len());
    for (index, command_payload) in commands_payload.iter().enumerate() {
        commands.push(parse_frontpic_anim_command_payload(
            species,
            index,
            command_payload,
        )?);
    }
    let program = FrontpicAnimProgram { commands };
    validate_frontpic_anim_program(species, &program)?;
    Ok(program)
}

fn parse_frontpic_anim_command_payload(
    species: &str,
    index: usize,
    payload: &Value,
) -> Result<FrontpicAnimCommand> {
    let Some(command) = payload.as_object() else {
        anyhow::bail!(
            "frontpic animation program for species '{species}' command {index} must be an object"
        );
    };
    let allowed_command_keys = ["kind", "frame", "duration", "count", "target"];
    for key in command.keys() {
        if !allowed_command_keys.contains(&key.as_str()) {
            anyhow::bail!(
                "frontpic animation program for species '{species}' command {index} contains unknown field '{key}'"
            );
        }
    }
    let kind = match command.get("kind") {
        Some(Value::String(kind)) => kind.clone(),
        Some(_) => anyhow::bail!(
            "frontpic animation program for species '{species}' command {index} kind must be a string"
        ),
        None => anyhow::bail!(
            "frontpic animation program for species '{species}' command {index} must declare kind"
        ),
    };
    let command = FrontpicAnimCommand {
        kind,
        frame: frontpic_anim_u16_field(species, index, command, "frame")?,
        duration: frontpic_anim_u16_field(species, index, command, "duration")?,
        count: frontpic_anim_u16_field(species, index, command, "count")?,
        target: frontpic_anim_u16_field(species, index, command, "target")?,
    };
    if let Some(issue) = frontpic_anim_command_issue(&command) {
        anyhow::bail!(
            "frontpic animation program for species '{species}' command {index} '{}' is invalid: {issue:?}",
            command.kind
        );
    }
    Ok(command)
}

fn frontpic_anim_u16_field(
    species: &str,
    index: usize,
    command: &serde_json::Map<String, Value>,
    field: &str,
) -> Result<Option<u16>> {
    let Some(value) = command.get(field) else {
        return Ok(None);
    };
    let Some(value) = value.as_u64() else {
        anyhow::bail!(
            "frontpic animation program for species '{species}' command {index} field '{field}' must be an unsigned integer"
        );
    };
    let value = u16::try_from(value).with_context(|| {
        format!(
            "frontpic animation program for species '{species}' command {index} field '{field}' is outside u16 range"
        )
    })?;
    Ok(Some(value))
}

fn merge_frontpic_anim_entries(
    target: &mut BTreeMap<String, FrontpicAnimProgram>,
    source: BTreeMap<String, FrontpicAnimProgram>,
) -> Result<()> {
    for (species, program) in source {
        validate_modpack_payload_token(&species, "frontpic animation program species id")?;
        validate_frontpic_anim_program(&species, &program)?;
        if target.contains_key(&species) {
            anyhow::bail!("duplicate frontpic animation program for species '{species}'");
        }
        target.insert(species, program);
    }
    Ok(())
}

fn validate_frontpic_anim_program(species: &str, program: &FrontpicAnimProgram) -> Result<()> {
    if program.commands.is_empty() {
        anyhow::bail!("frontpic animation program for species '{species}' must not be empty");
    }
    for (index, command) in program.commands.iter().enumerate() {
        if let Some(issue) = frontpic_anim_command_issue(command) {
            anyhow::bail!(
                "frontpic animation program for species '{species}' command {index} '{}' is invalid: {issue:?}",
                command.kind
            );
        }
        if command.kind == "dorepeat"
            && command
                .target
                .is_some_and(|target| usize::from(target) >= program.commands.len())
        {
            let target = command
                .target
                .context("dorepeat command target checked above but is missing")?;
            anyhow::bail!(
                "frontpic animation program for species '{species}' command {index} 'dorepeat' targets missing command {}",
                target
            );
        }
    }
    Ok(())
}

fn merge_sprite_palette_defaults(target: &mut BTreeMap<String, i64>, payload: Value) -> Result<()> {
    merge_sprite_palette_default_entries(target, parse_object_map::<i64>(payload)?)
}

fn merge_sprite_palette_default_entries(
    target: &mut BTreeMap<String, i64>,
    source: BTreeMap<String, i64>,
) -> Result<()> {
    for (sprite_id, palette) in source {
        validate_modpack_payload_token(&sprite_id, "sprite palette default sprite id")?;
        if palette < 0 {
            anyhow::bail!(
                "sprite palette default for sprite '{sprite_id}' must be nonnegative, found {palette}"
            );
        }
        if target.contains_key(&sprite_id) {
            anyhow::bail!("duplicate sprite palette default '{sprite_id}'");
        }
        target.insert(sprite_id, palette);
    }
    Ok(())
}

fn merge_pokemon_cries(
    target: &mut BTreeMap<String, PokemonCryMetadata>,
    payload: Value,
) -> Result<()> {
    let mut source = BTreeMap::new();
    for (species, metadata_payload) in parse_object_map::<Value>(payload)? {
        let metadata = parse_pokemon_cry_metadata_payload(&species, metadata_payload)?;
        source.insert(species, metadata);
    }
    merge_pokemon_cry_entries(target, source)
}

fn parse_pokemon_cry_metadata_payload(species: &str, payload: Value) -> Result<PokemonCryMetadata> {
    let Some(metadata) = payload.as_object() else {
        anyhow::bail!("Pokemon cry metadata for species '{species}' must be an object");
    };
    let allowed_keys = ["cry", "pitch", "length"];
    for key in metadata.keys() {
        if !allowed_keys.contains(&key.as_str()) {
            anyhow::bail!(
                "Pokemon cry metadata for species '{species}' contains unknown field '{key}'"
            );
        }
    }
    let cry = match metadata.get("cry") {
        Some(Value::String(cry)) => cry.clone(),
        Some(_) => {
            anyhow::bail!("Pokemon cry metadata for species '{species}' cry must be a string")
        }
        None => anyhow::bail!("Pokemon cry metadata for species '{species}' must declare cry"),
    };
    validate_modpack_payload_token(&cry, "Pokemon cry metadata audio id")?;
    Ok(PokemonCryMetadata {
        cry,
        pitch: pokemon_cry_word_field(species, metadata, "pitch")?,
        length: pokemon_cry_word_field(species, metadata, "length")?,
    })
}

fn pokemon_cry_word_field(
    species: &str,
    metadata: &serde_json::Map<String, Value>,
    field: &str,
) -> Result<i16> {
    let Some(value) = metadata.get(field) else {
        anyhow::bail!("Pokemon cry metadata for species '{species}' must declare {field}");
    };
    let Some(value) = value.as_i64() else {
        anyhow::bail!("Pokemon cry metadata for species '{species}' {field} must be an integer");
    };
    if !(i16::MIN as i64..=i16::MAX as i64).contains(&value) {
        anyhow::bail!(
            "Pokemon cry metadata for species '{species}' {field} {value} must fit an exact Crystal word"
        );
    }
    Ok(value as i16)
}

fn merge_pokemon_cry_entries(
    target: &mut BTreeMap<String, PokemonCryMetadata>,
    source: BTreeMap<String, PokemonCryMetadata>,
) -> Result<()> {
    for (species, metadata) in source {
        validate_modpack_payload_token(&species, "Pokemon cry metadata species id")?;
        validate_pokemon_cry_metadata(&species, &metadata)?;
        if target.contains_key(&species) {
            anyhow::bail!("duplicate Pokemon cry metadata for species '{species}'");
        }
        target.insert(species, metadata);
    }
    Ok(())
}

fn validate_pokemon_cry_metadata(_species: &str, metadata: &PokemonCryMetadata) -> Result<()> {
    validate_modpack_payload_token(&metadata.cry, "Pokemon cry metadata audio id")?;
    Ok(())
}

#[cfg(test)]
fn merge_exact_keyed_map<T>(
    target: &mut BTreeMap<String, T>,
    source: BTreeMap<String, T>,
    description: &str,
) -> Result<()> {
    for (key, value) in source {
        validate_exact_modpack_key(&key, description)?;
        if target.contains_key(&key) {
            anyhow::bail!("duplicate {description} '{key}'");
        }
        target.insert(key, value);
    }
    Ok(())
}

fn merge_token_keyed_map<T>(
    target: &mut BTreeMap<String, T>,
    source: BTreeMap<String, T>,
    description: &str,
) -> Result<()> {
    for (key, value) in source {
        validate_modpack_payload_token(&key, description)?;
        if target.contains_key(&key) {
            anyhow::bail!("duplicate {description} '{key}'");
        }
        target.insert(key, value);
    }
    Ok(())
}

fn parse_token_keyed_rule_map<T>(payload: Value, description: &str) -> Result<BTreeMap<String, T>>
where
    T: DeserializeOwned,
{
    let source: BTreeMap<String, T> =
        serde_json::from_value(payload).with_context(|| format!("parse {description} payload"))?;
    for key in source.keys() {
        validate_modpack_payload_token(key, description)?;
    }
    Ok(source)
}

fn merge_special_routine_rules(
    target: &mut BTreeMap<String, SpecialRoutineRule>,
    source: BTreeMap<String, SpecialRoutineRule>,
) -> Result<()> {
    for (routine, rule) in source {
        validate_modpack_payload_token(&routine, "special routine")?;
        if !is_known_special_routine(&routine) {
            anyhow::bail!("special routine '{routine}' is not implemented by the Rust runtime");
        }
        if target.contains_key(&routine) {
            anyhow::bail!("duplicate special routine '{routine}'");
        }
        target.insert(routine, rule);
    }
    Ok(())
}

#[cfg(test)]
fn merge_exact_keyed_string_vec_map(
    target: &mut BTreeMap<String, Vec<String>>,
    source: BTreeMap<String, Vec<String>>,
    key_description: &str,
    value_description: &str,
) -> Result<()> {
    for (key, values) in source {
        validate_exact_modpack_key(&key, key_description)?;
        if values.is_empty() {
            anyhow::bail!(
                "{key_description} '{key}' must declare at least one {value_description}"
            );
        }
        for value in &values {
            validate_exact_modpack_value(value, value_description)?;
        }
        if target.contains_key(&key) {
            anyhow::bail!("duplicate {key_description} '{key}'");
        }
        target.insert(key, values);
    }
    Ok(())
}

fn merge_token_keyed_string_vec_map(
    target: &mut BTreeMap<String, Vec<String>>,
    source: BTreeMap<String, Vec<String>>,
    key_description: &str,
    value_description: &str,
) -> Result<()> {
    for (key, values) in source {
        validate_modpack_payload_token(&key, key_description)?;
        if values.is_empty() {
            anyhow::bail!(
                "{key_description} '{key}' must declare at least one {value_description}"
            );
        }
        for value in &values {
            validate_exact_modpack_value(value, value_description)?;
        }
        if target.contains_key(&key) {
            anyhow::bail!("duplicate {key_description} '{key}'");
        }
        target.insert(key, values);
    }
    Ok(())
}

fn merge_token_keyed_token_vec_map(
    target: &mut BTreeMap<String, Vec<String>>,
    source: BTreeMap<String, Vec<String>>,
    key_description: &str,
    value_description: &str,
) -> Result<()> {
    for (key, values) in source {
        validate_modpack_payload_token(&key, key_description)?;
        if values.is_empty() {
            anyhow::bail!(
                "{key_description} '{key}' must declare at least one {value_description}"
            );
        }
        for value in &values {
            validate_battle_table_token(value, value_description)?;
        }
        if target.contains_key(&key) {
            anyhow::bail!("duplicate {key_description} '{key}'");
        }
        target.insert(key, values);
    }
    Ok(())
}

fn validate_exact_modpack_key(key: &str, description: &str) -> Result<()> {
    if key.is_empty() || key.trim() != key || key.chars().any(char::is_control) {
        anyhow::bail!("{description} key '{key}' must be exact, non-empty, and untrimmed");
    }
    Ok(())
}

fn validate_exact_modpack_value(value: &str, description: &str) -> Result<()> {
    if value.is_empty() || value.trim() != value || value.chars().any(char::is_control) {
        anyhow::bail!("{description} '{value}' must be exact, non-empty, and untrimmed");
    }
    Ok(())
}

fn validate_exact_modpack_text(value: &str, description: &str) -> Result<()> {
    if value.is_empty() || value.chars().any(char::is_control) {
        anyhow::bail!("{description} must be exact non-empty text");
    }
    Ok(())
}

fn validate_exact_modpack_multiline_text(value: &str, description: &str) -> Result<()> {
    if value.is_empty()
        || value.trim() != value
        || value
            .chars()
            .any(|char| char.is_control() && char != '\n' && char != '\r')
    {
        anyhow::bail!("{description} '{value}' must be exact, non-empty, and untrimmed");
    }
    Ok(())
}

#[cfg(test)]
fn merge_exact_string_vec(
    target: &mut Vec<String>,
    source: Vec<String>,
    description: &str,
) -> Result<()> {
    for value in source {
        validate_exact_modpack_value(&value, description)?;
        if target.contains(&value) {
            anyhow::bail!("duplicate {description} '{value}'");
        }
        target.push(value);
    }
    Ok(())
}

fn merge_exact_vec_by<T, F>(
    target: &mut Vec<T>,
    source: Vec<T>,
    description: &str,
    mut identity: F,
) -> Result<()>
where
    F: FnMut(&T) -> String,
{
    let mut seen = BTreeSet::new();
    for value in target.iter() {
        let key = identity(value);
        if !seen.insert(key.clone()) {
            anyhow::bail!("duplicate {description} '{key}'");
        }
    }
    for value in source {
        let key = identity(&value);
        if !seen.insert(key.clone()) {
            anyhow::bail!("duplicate {description} '{key}'");
        }
        target.push(value);
    }
    Ok(())
}

#[cfg(test)]
fn merge_exact_string_set(
    target: &mut BTreeSet<String>,
    source: Vec<String>,
    description: &str,
) -> Result<()> {
    for value in source {
        validate_exact_modpack_value(&value, description)?;
        if !target.insert(value.clone()) {
            anyhow::bail!("duplicate {description} '{value}'");
        }
    }
    Ok(())
}

fn insert_audio_asset(target: &mut Vec<ModpackAudioAsset>, asset: ModpackAudioAsset) -> Result<()> {
    asset.validate()?;
    if target.iter().any(|existing| existing.id == asset.id) {
        anyhow::bail!("duplicate audio asset id '{}'", asset.id);
    }
    if let Some(existing) = target.iter().find(|existing| existing.path == asset.path) {
        anyhow::bail!(
            "audio asset path '{}' is already declared by audio id '{}'",
            asset.path,
            existing.id
        );
    }
    target.push(asset);
    Ok(())
}

fn insert_keyed_audio_asset(
    target: &mut Vec<ModpackAudioAsset>,
    audio_id: String,
    asset: ModpackAudioAsset,
) -> Result<()> {
    validate_audio_asset_key(&audio_id)?;
    if audio_id != asset.id {
        anyhow::bail!(
            "audio asset key '{audio_id}' does not match record id '{}'",
            asset.id
        );
    }
    insert_audio_asset(target, asset)
}

fn validate_audio_asset_key(audio_id: &str) -> Result<()> {
    let has_known_prefix = audio_id.starts_with("MUSIC_")
        || audio_id.starts_with("SFX_")
        || audio_id.starts_with("CRY_");
    let exact_token = !audio_id.is_empty()
        && audio_id.trim() == audio_id
        && audio_id
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_');
    if !has_known_prefix || !exact_token {
        anyhow::bail!(
            "audio asset key '{audio_id}' must be an exact MUSIC_, SFX_, or CRY_ audio id"
        );
    }
    Ok(())
}

fn insert_keyed_tileset_definition(
    target: &mut BTreeMap<String, TilesetDefinition>,
    tileset_id: String,
    tileset: TilesetDefinition,
) -> Result<()> {
    if !is_exact_tileset_id(&tileset_id) {
        anyhow::bail!("tileset id '{tileset_id}' must be an exact asset id");
    }
    if tileset.collision.is_empty() {
        anyhow::bail!("tileset '{tileset_id}' must declare collision data");
    }
    if tileset.palette_map.is_empty() {
        anyhow::bail!("tileset '{tileset_id}' must declare palette_map data");
    }
    let mut metatile_ids = BTreeSet::new();
    for (metatile_id, quadrants) in &tileset.collision {
        validate_exact_modpack_key(metatile_id, "tileset metatile id")?;
        let parsed_metatile_id = parse_metatile_id(metatile_id).with_context(|| {
            format!("parse metatile id '{metatile_id}' in tileset '{tileset_id}'")
        })?;
        metatile_ids.insert(parsed_metatile_id);
        if quadrants.len() != 4 {
            anyhow::bail!(
                "tileset '{tileset_id}' metatile '{metatile_id}' must declare 4 collision quadrants"
            );
        }
        for token in quadrants {
            validate_exact_modpack_value(token, "tileset collision token")?;
            resolve_collision_token(token).with_context(|| {
                format!("resolve collision token '{token}' in tileset '{tileset_id}:{metatile_id}'")
            })?;
        }
    }
    require_dense_metatile_ids(
        &metatile_ids,
        &format!("tileset '{tileset_id}' collision map"),
    )?;
    if target.insert(tileset_id.clone(), tileset).is_some() {
        anyhow::bail!("duplicate tileset id '{tileset_id}'");
    }
    Ok(())
}

fn require_dense_metatile_ids(ids: &BTreeSet<usize>, subject: &str) -> Result<()> {
    let Some(max_id) = ids.iter().next_back().copied() else {
        anyhow::bail!("{subject} must declare collision data");
    };
    for expected in 0..=max_id {
        if !ids.contains(&expected) {
            anyhow::bail!("{subject} must explicitly declare metatile id {expected}");
        }
    }
    Ok(())
}

fn is_exact_tileset_id(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}

fn insert_keyed_trainer(
    target: &mut TrainerCatalog,
    trainer_id: String,
    trainer: Trainer,
) -> Result<()> {
    let record_id = trainer_key(&trainer).context("parse trainer id")?;
    validate_modpack_payload_token(&trainer_id, "trainer key")?;
    validate_modpack_payload_token(&record_id, "trainer id")?;
    if trainer_id != record_id {
        anyhow::bail!("trainer key '{trainer_id}' does not match record trainer_id '{record_id}'");
    }
    validate_trainer_pack_record(&trainer)?;
    target.insert(trainer).context("insert trainer payload")
}

fn merge_trainer_class_names(
    target: &mut BTreeMap<String, String>,
    source: BTreeMap<String, String>,
) -> Result<()> {
    for (trainer_class, display_name) in source {
        validate_modpack_payload_token(&trainer_class, "trainer class name id")?;
        validate_exact_modpack_value(&display_name, "trainer class display name")?;
        if target.contains_key(&trainer_class) {
            anyhow::bail!("duplicate trainer class display name '{trainer_class}'");
        }
        target.insert(trainer_class, display_name);
    }
    Ok(())
}

fn validate_trainer_pack_record(trainer: &Trainer) -> Result<()> {
    validate_battle_table_token(
        &trainer.trainer_class,
        &format!("trainer '{}' trainer_class", trainer.trainer_id),
    )?;
    validate_modpack_payload_token(
        &trainer.trainer_id,
        &format!("trainer '{}' trainer_id", trainer.trainer_id),
    )?;
    validate_modpack_payload_token(
        &trainer.encounter_music,
        &format!("trainer '{}' encounter_music", trainer.trainer_id),
    )?;
    if trainer.party.is_empty() {
        anyhow::bail!(
            "trainer '{}' must declare an explicit nonempty party",
            trainer.trainer_id
        );
    }
    for (slot, party_mon) in trainer.party.iter().enumerate() {
        validate_modpack_payload_token(
            &party_mon.species,
            &format!(
                "trainer '{}' party species at slot {slot}",
                trainer.trainer_id
            ),
        )?;
        if let Some(item_id) = party_mon.item.as_deref() {
            validate_modpack_payload_token(
                item_id,
                &format!("trainer '{}' party item at slot {slot}", trainer.trainer_id),
            )?;
        }
        for learned_move in &party_mon.moves {
            validate_modpack_payload_token(
                &learned_move.name,
                &format!("trainer '{}' party move at slot {slot}", trainer.trainer_id),
            )?;
        }
    }
    for (slot, item_id) in trainer.items.iter().enumerate() {
        if let Some(item_id) = item_id.as_deref() {
            validate_modpack_payload_token(
                item_id,
                &format!(
                    "trainer '{}' battle item at slot {slot}",
                    trainer.trainer_id
                ),
            )?;
        }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct LevelUpMoveValue {
    level: u8,
    #[serde(rename = "move")]
    move_id: String,
}

fn merge_level_up_moves_payload(
    target: &mut BTreeMap<String, Value>,
    payload: Value,
) -> Result<()> {
    let Some(object) = payload.as_object() else {
        anyhow::bail!("species value payload must be a species-keyed object");
    };
    if object.is_empty() {
        anyhow::bail!("species value payload must contain at least one entry");
    }
    for (species, entry) in object {
        validate_modpack_payload_token(species, "species value payload species key")?;
        merge_keyed_level_up_moves_entry(target, species, entry.clone())?;
    }
    Ok(())
}

fn merge_egg_moves_payload(target: &mut BTreeMap<String, Value>, payload: Value) -> Result<()> {
    let Some(object) = payload.as_object() else {
        anyhow::bail!("species value payload must be a species-keyed object");
    };
    if object.is_empty() {
        anyhow::bail!("species value payload must contain at least one entry");
    }
    for (species, entry) in object {
        validate_modpack_payload_token(species, "species value payload species key")?;
        merge_keyed_egg_moves_entry(target, species, entry.clone())?;
    }
    Ok(())
}

fn merge_keyed_level_up_moves_entry(
    target: &mut BTreeMap<String, Value>,
    key: &str,
    payload: Value,
) -> Result<()> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Entry {
        species: String,
        moves: Vec<LevelUpMoveValue>,
    }

    let entry: Entry = serde_json::from_value(payload)
        .with_context(|| format!("parse species value payload for '{key}'"))?;
    if entry.species != key {
        anyhow::bail!(
            "species value key '{key}' does not match record species '{}'",
            entry.species
        );
    }
    validate_modpack_payload_token(&entry.species, "species value payload species")?;
    for learned_move in &entry.moves {
        validate_modpack_payload_token(
            &learned_move.move_id,
            &format!("species value payload moves for '{}'", entry.species),
        )?;
    }
    if target.contains_key(&entry.species) {
        anyhow::bail!(
            "duplicate species value payload for species '{}'",
            entry.species
        );
    }
    target.insert(entry.species, serde_json::to_value(entry.moves)?);
    Ok(())
}

fn merge_keyed_egg_moves_entry(
    target: &mut BTreeMap<String, Value>,
    key: &str,
    payload: Value,
) -> Result<()> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Entry {
        species: String,
        moves: Vec<String>,
    }

    let entry: Entry = serde_json::from_value(payload)
        .with_context(|| format!("parse species value payload for '{key}'"))?;
    if entry.species != key {
        anyhow::bail!(
            "species value key '{key}' does not match record species '{}'",
            entry.species
        );
    }
    validate_modpack_payload_token(&entry.species, "species value payload species")?;
    for move_id in &entry.moves {
        validate_modpack_payload_token(
            move_id,
            &format!("species value payload moves for '{}'", entry.species),
        )?;
    }
    if target.contains_key(&entry.species) {
        anyhow::bail!(
            "duplicate species value payload for species '{}'",
            entry.species
        );
    }
    target.insert(entry.species, serde_json::to_value(entry.moves)?);
    Ok(())
}

#[cfg(test)]
fn merge_object_payload(
    target: &mut BTreeMap<String, Value>,
    payload: Value,
    key_description: &str,
) -> Result<()> {
    let Some(object) = payload.as_object() else {
        anyhow::bail!("object payload must be an object");
    };
    if object.is_empty() {
        anyhow::bail!("object payload must contain at least one entry");
    }
    let key_validation_description = key_description
        .strip_suffix(" key")
        .unwrap_or(key_description);
    for (key, value) in object {
        validate_battle_table_token(key, key_validation_description)?;
        if target.contains_key(key) {
            anyhow::bail!("duplicate object payload key '{key}'");
        }
        target.insert(key.clone(), value.clone());
    }
    Ok(())
}

fn merge_map_script_payload(target: &mut BTreeMap<String, Value>, payload: Value) -> Result<()> {
    let Some(object) = payload.as_object() else {
        anyhow::bail!("map script payload must be an object");
    };
    if object.is_empty() {
        anyhow::bail!("map script payload must contain at least one entry");
    }
    for (script_key, value) in object {
        validate_script_label_payload_token(script_key, "map script payload")?;
        if target.contains_key(script_key) {
            anyhow::bail!("duplicate object payload key '{script_key}'");
        }
        if let Some(text) = value.as_str() {
            validate_exact_modpack_value(text, &format!("map script payload '{script_key}' text"))?;
            target.insert(script_key.clone(), Value::String(text.to_string()));
        } else {
            let commands = parse_raw_script_command_list("map script payload", script_key, value)?;
            target.insert(
                script_key.clone(),
                serde_json::to_value(commands).with_context(|| {
                    format!("encode canonical map script payload for {script_key}")
                })?,
            );
        }
    }
    Ok(())
}

fn merge_map_block_payload(target: &mut BTreeMap<String, String>, payload: Value) -> Result<()> {
    let Some(object) = payload.as_object() else {
        anyhow::bail!("map block payload must be an object");
    };
    if object.is_empty() {
        anyhow::bail!("map block payload must contain at least one entry");
    }
    for (block_label, value) in object {
        validate_modpack_payload_token(block_label, "map block data for label")?;
        if target.contains_key(block_label) {
            anyhow::bail!("duplicate map block data for label '{block_label}'");
        }
        let encoded_blocks = value
            .as_str()
            .with_context(|| format!("map block payload '{block_label}' must be a string"))?;
        validate_exact_modpack_value(
            encoded_blocks,
            &format!("map block data for label '{block_label}'"),
        )?;
        let decoded_blocks = decode_base64_bytes(encoded_blocks)
            .with_context(|| format!("decode map block payload '{block_label}'"))?;
        if decoded_blocks.is_empty() {
            anyhow::bail!("map block payload '{block_label}' must decode to at least one block");
        }
        target.insert(block_label.clone(), encoded_blocks.to_string());
    }
    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct MapDimensionsPayload {
    width: u16,
    height: u16,
}

fn merge_map_dimensions_payload(
    target: &mut BTreeMap<String, Value>,
    payload: Value,
) -> Result<()> {
    let Some(object) = payload.as_object() else {
        anyhow::bail!("map dimensions payload must be an object");
    };
    if object.is_empty() {
        anyhow::bail!("map dimensions payload must contain at least one entry");
    }
    for (map_name, value) in object {
        validate_map_reference_token(map_name, "map dimensions payload")?;
        if target.contains_key(map_name) {
            anyhow::bail!("duplicate object payload key '{map_name}'");
        }
        let dimensions: MapDimensionsPayload = serde_json::from_value(value.clone())
            .with_context(|| format!("parse map dimensions payload for {map_name}"))?;
        if dimensions.width == 0 || dimensions.height == 0 {
            anyhow::bail!(
                "map dimensions payload for {map_name} must declare positive width and height"
            );
        }
        target.insert(
            map_name.clone(),
            serde_json::to_value(dimensions).with_context(|| {
                format!("encode canonical map dimensions payload for {map_name}")
            })?,
        );
    }
    Ok(())
}

fn merge_npc_payload(target: &mut BTreeMap<String, Value>, payload: Value) -> Result<()> {
    let Some(object) = payload.as_object() else {
        anyhow::bail!("NPC payload must be an object");
    };
    if object.is_empty() {
        anyhow::bail!("NPC payload must contain at least one entry");
    }
    for (map_name, value) in object {
        validate_map_reference_token(map_name, "NPC payload")?;
        if target.contains_key(map_name) {
            anyhow::bail!("duplicate object payload key '{map_name}'");
        }
        let objects: Vec<ObjectEvent> = serde_json::from_value(value.clone())
            .with_context(|| format!("parse NPC object payload for {map_name}"))?;
        validate_npc_object_events(map_name, &objects)?;
        let canonical_payload = serde_json::to_value(&objects)
            .with_context(|| format!("encode canonical NPC object payload for {map_name}"))?;
        target.insert(map_name.clone(), canonical_payload);
    }
    Ok(())
}

fn validate_npc_object_events(map_name: &str, objects: &[ObjectEvent]) -> Result<()> {
    let mut object_identifiers = BTreeSet::new();
    for (index, object) in objects.iter().enumerate() {
        validate_exact_modpack_value(
            &object.sprite,
            &format!("NPC object {index} on {map_name} sprite"),
        )?;
        validate_object_event_reference(map_name, index, "spritemovedata", &object.spritemovedata)?;
        if object_event_initial_facing(&object.spritemovedata).is_none() {
            anyhow::bail!(
                "NPC object {index} on {map_name} uses unknown spritemovedata '{}'",
                object.spritemovedata
            );
        }
        validate_object_event_reference(map_name, index, "object_type", &object.object_type)?;
        if object.script != "-1" && object.script != "ObjectEvent" {
            validate_object_event_reference(map_name, index, "script", &object.script)?;
        }
        if let Some(label) = object.label.as_deref() {
            validate_object_event_reference(map_name, index, "label", label)?;
        }
        if object.event_flag != "-1" {
            validate_object_event_reference(map_name, index, "event_flag", &object.event_flag)?;
        }
        if let Some(object_id) = object.object_identifier.as_deref() {
            validate_object_event_reference(map_name, index, "object_identifier", object_id)?;
            if !object_identifiers.insert(object_id.to_string()) {
                anyhow::bail!(
                    "NPC object identifier '{object_id}' is duplicated on map {map_name}"
                );
            }
        }
        if let Some(direction) = object.sightline_direction_override.as_deref() {
            validate_object_event_reference(
                map_name,
                index,
                "sightline_direction_override",
                direction,
            )?;
        }
    }
    Ok(())
}

fn validate_object_event_reference(
    map_name: &str,
    index: usize,
    field: &str,
    value: &str,
) -> Result<()> {
    if !is_exact_object_event_reference_token(value) {
        anyhow::bail!("NPC object {index} on {map_name} {field} '{value}' must be an exact token");
    }
    Ok(())
}

fn merge_pokedex_payload(target: &mut Vec<Value>, payload: Value) -> Result<()> {
    let Some(object) = payload.as_object() else {
        anyhow::bail!("pokedex payload must be a species-keyed object");
    };
    if object.is_empty() {
        anyhow::bail!("pokedex payload must contain at least one entry");
    }
    for (species, entry) in object {
        validate_modpack_payload_token(species, "pokedex species key")?;
        merge_keyed_pokedex_payload(target, species, entry.clone())?;
    }
    Ok(())
}

fn merge_keyed_pokedex_payload(target: &mut Vec<Value>, key: &str, payload: Value) -> Result<()> {
    #[derive(Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct RawPokedexEntry {
        species: String,
        classification: String,
        height: f64,
        weight: f64,
        text: String,
    }

    let entry: RawPokedexEntry = serde_json::from_value(payload.clone())
        .with_context(|| format!("parse pokedex entry payload for '{key}'"))?;
    if entry.species != key {
        anyhow::bail!(
            "pokedex key '{key}' does not match record species '{}'",
            entry.species
        );
    }
    validate_modpack_payload_token(&entry.species, "pokedex species")?;
    validate_exact_modpack_value(&entry.classification, "pokedex classification")?;
    validate_exact_modpack_value(&entry.text, "pokedex text")?;
    if !entry.height.is_finite() || entry.height <= 0.0 {
        anyhow::bail!(
            "pokedex entry for species '{}' must declare positive height",
            entry.species
        );
    }
    if !entry.weight.is_finite() || entry.weight <= 0.0 {
        anyhow::bail!(
            "pokedex entry for species '{}' must declare positive weight",
            entry.species
        );
    }
    if target
        .iter()
        .any(|stored| stored.get("species").and_then(Value::as_str) == Some(entry.species.as_str()))
    {
        anyhow::bail!("duplicate pokedex payload for species '{}'", entry.species);
    }
    target.push(
        serde_json::to_value(entry)
            .with_context(|| format!("encode canonical pokedex payload for '{key}'"))?,
    );
    Ok(())
}

#[cfg(test)]
fn merge_raw_object_payload(
    target: &mut Vec<Value>,
    payload: Value,
    payload_description: &str,
    key_description: &str,
) -> Result<()> {
    let Some(object) = payload.as_object() else {
        anyhow::bail!("{payload_description} must be an object");
    };
    if object.is_empty() {
        anyhow::bail!("{payload_description} must contain at least one entry");
    }
    let key_validation_description = key_description
        .strip_suffix(" key")
        .unwrap_or(key_description);
    for key in object.keys() {
        validate_modpack_payload_token(key, key_validation_description)?;
        if target.iter().any(|entry| {
            entry
                .as_object()
                .is_some_and(|existing| existing.contains_key(key))
        }) {
            anyhow::bail!("duplicate {key_description} '{key}'");
        }
    }
    target.push(payload);
    Ok(())
}

fn merge_raw_script_payload(
    target: &mut Vec<Value>,
    payload: Value,
    payload_description: &str,
    key_description: &str,
) -> Result<()> {
    let Some(object) = payload.as_object() else {
        anyhow::bail!("{payload_description} must be an object");
    };
    if object.is_empty() {
        anyhow::bail!("{payload_description} must contain at least one entry");
    }
    let key_validation_description = key_description
        .strip_suffix(" key")
        .unwrap_or(key_description);
    let mut canonical_payload = serde_json::Map::new();
    for (key, value) in object {
        validate_script_label_payload_token(key, key_validation_description)?;
        if target.iter().any(|entry| {
            entry
                .as_object()
                .is_some_and(|existing| existing.contains_key(key))
        }) {
            anyhow::bail!("duplicate {key_description} '{key}'");
        }
        let commands = parse_raw_script_command_list(payload_description, key, value)?;
        canonical_payload.insert(
            key.clone(),
            serde_json::to_value(commands)
                .with_context(|| format!("encode canonical {payload_description} for {key}"))?,
        );
    }
    target.push(Value::Object(canonical_payload));
    Ok(())
}

fn merge_raw_story_event_payload(
    target: &mut Vec<Value>,
    payload: Value,
    payload_description: &str,
    key_description: &str,
) -> Result<()> {
    let Some(object) = payload.as_object() else {
        anyhow::bail!("{payload_description} must be an object");
    };
    if object.is_empty() {
        anyhow::bail!("{payload_description} must contain at least one entry");
    }
    let key_validation_description = key_description
        .strip_suffix(" key")
        .unwrap_or(key_description);
    let mut canonical_payload = serde_json::Map::new();
    for (map_key, value) in object {
        validate_map_reference_token(map_key, key_validation_description)?;
        if target.iter().any(|entry| {
            entry
                .as_object()
                .is_some_and(|existing| existing.contains_key(map_key))
        }) {
            anyhow::bail!("duplicate {key_description} '{map_key}'");
        }
        let Some(scripts) = value.as_object() else {
            anyhow::bail!("{payload_description} '{map_key}' must be a script object");
        };
        if scripts.is_empty() {
            anyhow::bail!("{payload_description} '{map_key}' must contain at least one script");
        }
        let mut canonical_scripts = serde_json::Map::new();
        for (script_key, script_payload) in scripts {
            validate_script_label_payload_token(script_key, "story event script")?;
            // The standard-script catalog contains a metadata entry alongside
            // executable script bodies. GlobalScriptRoots is a list of labels,
            // not a list of `{ command, args }` script commands.
            if map_key == "StandardScripts" && script_key == "GlobalScriptRoots" {
                let roots = script_payload.as_array().with_context(|| {
                    format!("{payload_description} '{map_key}' GlobalScriptRoots must be an array")
                })?;
                for (index, root) in roots.iter().enumerate() {
                    let root = root.as_str().with_context(|| {
                        format!(
                            "{payload_description} '{map_key}' GlobalScriptRoots entry {index} must be a script label"
                        )
                    })?;
                    validate_script_label_payload_token(root, "global script root")?;
                }
                canonical_scripts.insert(script_key.clone(), Value::Array(roots.clone()));
                continue;
            }
            let commands =
                parse_raw_script_command_list(payload_description, script_key, script_payload)?;
            canonical_scripts.insert(
                script_key.clone(),
                serde_json::to_value(commands).with_context(|| {
                    format!(
                        "encode canonical {payload_description} script '{script_key}' for {map_key}"
                    )
                })?,
            );
        }
        canonical_payload.insert(map_key.clone(), Value::Object(canonical_scripts));
    }
    target.push(Value::Object(canonical_payload));
    Ok(())
}

#[cfg(test)]
mod story_event_payload_tests {
    use super::*;

    #[test]
    fn standard_scripts_accept_global_script_root_metadata() {
        let mut target = Vec::new();
        let payload = serde_json::json!({
            "StandardScripts": {
                "GlobalScriptRoots": ["BugCatchingContestOverScript"],
                "StdScripts": [{"command": "add_stdscript", "args": ["ExampleScript"]}],
                "ExampleScript": [{"command": "end", "args": ""}]
            }
        });

        merge_raw_story_event_payload(
            &mut target,
            payload,
            "story event payload",
            "story event payload key",
        )
        .expect("standard-script metadata should be accepted");
        assert_eq!(
            target[0]["StandardScripts"]["GlobalScriptRoots"],
            serde_json::json!(["BugCatchingContestOverScript"])
        );
    }
}

fn validate_raw_script_command_list(
    payload_description: &str,
    script_key: &str,
    value: &Value,
) -> Result<()> {
    parse_raw_script_command_list(payload_description, script_key, value).map(|_| ())
}

fn parse_raw_script_command_list(
    payload_description: &str,
    script_key: &str,
    value: &Value,
) -> Result<Vec<Value>> {
    let commands: Vec<RawScriptCommand> = serde_json::from_value(value.clone())
        .with_context(|| format!("parse {payload_description} script '{script_key}'"))?;
    let mut canonical_commands = Vec::with_capacity(commands.len());
    for (index, command) in commands.iter().enumerate() {
        validate_exact_modpack_value(
            &command.command,
            &format!("{payload_description} script '{script_key}' command {index} name"),
        )?;
        let canonical_args =
            canonical_raw_script_args(payload_description, script_key, index, &command.args)?;
        validate_canonical_raw_script_command_args(
            payload_description,
            script_key,
            index,
            &command.command,
            &canonical_args,
        )?;
        canonical_commands.push(serde_json::json!({
            "command": command.command,
            "args": canonical_args,
        }));
    }
    Ok(canonical_commands)
}

fn canonical_raw_script_args(
    payload_description: &str,
    script_key: &str,
    command_index: usize,
    args: &Value,
) -> Result<Value> {
    if let Some(arg) = args.as_str() {
        if !arg.is_empty() {
            validate_exact_modpack_value(
                arg,
                &format!("{payload_description} script '{script_key}' command {command_index} arg"),
            )?;
        }
        return Ok(Value::String(arg.to_string()));
    }
    let Some(values) = args.as_array() else {
        anyhow::bail!(
            "{payload_description} script '{script_key}' command {command_index} args must be a string or array"
        );
    };
    for (arg_index, arg) in values.iter().enumerate() {
        let Some(arg) = arg.as_str() else {
            anyhow::bail!(
                "{payload_description} script '{script_key}' command {command_index} arg {arg_index} must be a string"
            );
        };
        validate_exact_modpack_value(
            arg,
            &format!(
                "{payload_description} script '{script_key}' command {command_index} arg {arg_index}"
            ),
        )?;
    }
    Ok(Value::Array(values.clone()))
}

fn validate_canonical_raw_script_command_args(
    payload_description: &str,
    script_key: &str,
    command_index: usize,
    command: &str,
    args: &Value,
) -> Result<()> {
    if command != "warpfacing" {
        return Ok(());
    }
    let values = args.as_array().with_context(|| {
        format!(
            "{payload_description} script '{script_key}' command {command_index} warpfacing args must be an array"
        )
    })?;
    if values.len() != 4 {
        anyhow::bail!(
            "{payload_description} script '{script_key}' command {command_index} warpfacing args must have 4 entries"
        );
    }
    let facing = values[3].as_str().with_context(|| {
        format!(
            "{payload_description} script '{script_key}' command {command_index} warpfacing facing arg must be a string"
        )
    })?;
    parse_script_warp_facing(facing).map_err(|error| anyhow::anyhow!("{error}"))?;
    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawScriptCommand {
    command: String,
    args: Value,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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
            "db" => {
                if command.args.len() != 2 {
                    anyhow::bail!(
                        "Malformed db in {map_name}: expected 2 args, found {}.",
                        command.args.len()
                    );
                }
                validate_map_section_args(&command.args, &format!("db in {map_name}"))?;
                section = None;
            }
            "def_warp_events" => {
                if !command.args.is_empty() {
                    anyhow::bail!(
                        "Malformed def_warp_events in {map_name}: expected 0 args, found {}.",
                        command.args.len()
                    );
                }
                section = Some("warps");
                next_warp_index = 1;
            }
            "def_coord_events" => {
                if !command.args.is_empty() {
                    anyhow::bail!(
                        "Malformed def_coord_events in {map_name}: expected 0 args, found {}.",
                        command.args.len()
                    );
                }
                section = Some("coord_events");
            }
            "def_bg_events" => {
                if !command.args.is_empty() {
                    anyhow::bail!(
                        "Malformed def_bg_events in {map_name}: expected 0 args, found {}.",
                        command.args.len()
                    );
                }
                section = Some("bg_events");
            }
            "def_object_events" => {
                if !command.args.is_empty() {
                    anyhow::bail!(
                        "Malformed def_object_events in {map_name}: expected 0 args, found {}.",
                        command.args.len()
                    );
                }
                section = Some("object_events");
            }
            "warp_event" if section == Some("warps") => {
                if command.args.len() != 4 {
                    anyhow::bail!(
                        "Malformed warp_event in {map_name}: expected 4 args, found {}.",
                        command.args.len()
                    );
                }
                validate_map_section_args(&command.args, &format!("warp_event in {map_name}"))?;
                validate_map_reference_token(&command.args[2], "warp_event target map")?;
                let (x, y) = parse_map_event_runtime_coords(
                    map_name,
                    "warp_event",
                    &command.args[0],
                    &command.args[1],
                )?;
                let target_map_constant = command.args[2].to_string();
                events.warps.push(WarpEvent {
                    index: next_warp_index,
                    x,
                    y,
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
                validate_map_section_args(&command.args, &format!("coord_event in {map_name}"))?;
                let (x, y) = parse_map_event_runtime_coords(
                    map_name,
                    "coord_event",
                    &command.args[0],
                    &command.args[1],
                )?;
                events.coord_events.push(CoordEvent {
                    x,
                    y,
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
                validate_map_section_args(&command.args, &format!("bg_event in {map_name}"))?;
                let (x, y) = parse_map_event_runtime_coords(
                    map_name,
                    "bg_event",
                    &command.args[0],
                    &command.args[1],
                )?;
                events.bg_events.push(BackgroundEvent {
                    x,
                    y,
                    event_type: command.args[2].clone(),
                    script: command.args[3].clone(),
                });
            }
            "object_event" if section == Some("object_events") => {
                validate_map_section_args(&command.args, &format!("object_event in {map_name}"))?;
            }
            "warp_event" | "coord_event" | "bg_event" | "object_event" => {
                anyhow::bail!(
                    "Malformed {} in {map_name}: command appears outside its declared section.",
                    command.command
                );
            }
            _ => {
                anyhow::bail!(
                    "Malformed map events in {map_name}: unknown command '{}'.",
                    command.command
                );
            }
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
            anyhow::bail!(
                "Malformed command in {script_label} for {map_name}: unknown map script section command '{}'.",
                command.command
            );
        };
        if !expected.contains(&command.args.len()) {
            anyhow::bail!(
                "Malformed {} command in {script_label} for {map_name}: expected one of {:?} args, found {}.",
                command.command,
                expected,
                command.args.len()
            );
        }
        let parsed_command = MapScriptSectionCommand {
            command: command.command,
            args: command.args,
            command_index: index,
        };
        validate_map_script_section_command_shape(map_name, &parsed_command)?;
        parsed.push(parsed_command);
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
            anyhow::bail!(
                "Malformed command in {script_label} for {map_name}: unknown map event section command '{}'.",
                command.command
            );
        };
        if !expected.contains(&command.args.len()) {
            anyhow::bail!(
                "Malformed {} command in {script_label} for {map_name}: expected one of {:?} args, found {}.",
                command.command,
                expected,
                command.args.len()
            );
        }
        let parsed_command = MapEventSectionCommand {
            command: command.command,
            args: command.args,
            command_index: index,
        };
        validate_map_event_section_command_shape(map_name, &parsed_command)?;
        parsed.push(parsed_command);
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
            request.seen_text = resolve_trainer_command_reference(
                scripts,
                script_name,
                arg(3)?,
                "seen text",
            )?;
            request.win_text = resolve_trainer_command_reference(
                scripts,
                script_name,
                arg(4)?,
                "win text",
            )?;
            request.loss_text = resolve_trainer_command_reference(
                scripts,
                script_name,
                arg(5)?,
                "loss text",
            )?;
            request.callback = resolve_trainer_command_reference(
                scripts,
                script_name,
                arg(6)?,
                "callback",
            )?;
            request.source_script = script_name.clone();
            trainer_scripts.insert(script_name.clone(), request);
        }
    }
    Ok(trainer_scripts)
}

fn resolve_trainer_command_reference(
    scripts: &BTreeMap<String, Value>,
    source_script: &str,
    raw_label: &str,
    field: &str,
) -> Result<String> {
    let label = trainer_command_optional_arg(raw_label);
    if label.is_empty() || !label.starts_with('.') {
        return Ok(label);
    }
    let parent = source_script
        .rsplit_once('@')
        .map(|(_, parent)| parent)
        .unwrap_or(source_script);
    let scoped = format!("{label}@{parent}");
    match (scripts.contains_key(&label), scripts.contains_key(&scoped)) {
        (false, true) => Ok(scoped),
        (true, false) => Ok(label),
        (false, false) => anyhow::bail!(
            "trainer command {field} {label} from {source_script} resolves to neither {label} nor {scoped}"
        ),
        (true, true) => anyhow::bail!(
            "trainer command {field} {label} from {source_script} is ambiguous between {label} and {scoped}"
        ),
    }
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
                command: command_name.to_string(),
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
                command: command_name.to_string(),
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
                command: command_name.to_string(),
                mart_type: args[0].to_string(),
                mart_id: args[1].to_string(),
                source_script: script_name.clone(),
                command_index: index,
            });
        }
    }
    Ok(commands)
}
