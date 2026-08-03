impl GameDataSet {
    pub fn require_current_map(&self, current_map: &str, requested_map: &str) -> Result<()> {
        if current_map != requested_map {
            anyhow::bail!(
                "script command map mismatch: session is on {current_map}, request was for {requested_map}"
            );
        }
        Ok(())
    }

    pub fn require_no_active_battle(&self, state: &GameState, context: &str) -> Result<()> {
        state.require_no_active_battle().map_err(|error| {
            anyhow::anyhow!("cannot use {context} during an active battle: {error:?}")
        })
    }

    pub fn validate_save_currency(&self, state: &GameState) -> Result<()> {
        validate_save_currency_for_runtime_pack(state, &self.currency_constants).map_err(|error| {
            anyhow::anyhow!(
                "validate Crystal runtime save against compiled pack currency constants: {error:?}"
            )
        })
    }

    pub fn process_overworld_step(&self, state: &mut GameState) -> Result<StepEventResult> {
        let result = core_process_overworld_step(state, &self.step_event_rules, &self.growth_rates)
            .context("process Day Care and party step events from compiled rules")?;
        self.normalize_day_care_egg_species(state);
        Ok(result)
    }

    fn check_trainer_sight_after_step(
        &self,
        state: &GameState,
        session: &OverworldSession,
    ) -> Result<Option<OverworldInteraction>> {
        let module = self
            .maps
            .get(&session.map.name)
            .with_context(|| format!("missing map module for {}", session.map.name))?;
        let mut eligible_scripts = BTreeSet::new();
        for (source_script, request) in &module.trainer_scripts {
            let defeated = !request.event_flag.is_empty()
                && state
                    .flags
                    .is_event_flag_set(&request.event_flag)
                    .map_err(|error| {
                        anyhow::anyhow!(
                            "check trainer event flag {} on {}: {error}",
                            request.event_flag,
                            session.map.name
                        )
                    })?;
            if !defeated {
                eligible_scripts.insert(source_script.as_str());
            }
        }
        session
            .check_trainer_sight_checked_with_filter(|object| {
                eligible_scripts.contains(object.script.as_str())
            })
            .map_err(|error| anyhow::anyhow!("check trainer sight on {}: {error}", session.map.name))
    }

    fn queue_strength_boulder_landing_script(
        &self,
        state: &mut GameState,
        session: &OverworldSession,
        object_id: &str,
    ) -> Result<bool> {
        let object_tile = session.object_runtime_tile_by_id(object_id).map_err(|error| {
            anyhow::anyhow!("resolve pushed Strength boulder {object_id} runtime tile: {error}")
        })?;
        let landing_warp = session
            .map_events
            .warps
            .iter()
            .enumerate()
            .find_map(|(index, warp)| {
                (warp_tile_position_checked(warp) == Some(object_tile))
                    .then_some((index + 1) as u16)
            });
        let Some(landing_warp) = landing_warp else {
            return Ok(false);
        };
        let Some(entry) = state
            .script_runtime
            .stone_table_entries
            .iter()
            .find(|entry| entry.warp == landing_warp && entry.object_event == object_id)
            .cloned()
        else {
            return Ok(false);
        };
        if state.script_runtime.next_script.is_some() {
            anyhow::bail!(
                "cannot queue Strength boulder landing script {} while another script is pending",
                entry.script
            );
        }
        state.script_runtime.next_script = Some(ScriptLocation {
            origin_map_name: session.map.name.clone(),
            script: entry.script,
        });
        Ok(true)
    }

    /// Crystal creates a breeding egg from the mother's pre-evolution, not
    /// from the currently evolved daycare species.  The core step hook does
    /// not own the compiled evolution/learnset catalogs, so normalize the
    /// concrete egg here at the pack boundary.
    fn normalize_day_care_egg_species(&self, state: &mut GameState) {
        let Some(existing) = state.day_care.egg.clone() else {
            return;
        };
        if !existing.is_egg {
            return;
        }
        let mut species_id = existing.species.id.clone();
        for _ in 0..8 {
            let Some(previous) = self.evolutions.0.iter().find_map(|(source, entries)| {
                entries
                    .iter()
                    .any(|entry| entry.species == species_id)
                    .then_some(source.as_str())
            }) else {
                break;
            };
            species_id = previous.to_string();
        }
        let Some(species) = self.pokemon.get(&species_id) else {
            return;
        };
        let Ok(mut egg) = create_pokemon_from_known_dvs(
            species,
            existing.level,
            existing.dvs,
            &self.learnsets,
            &self.moves,
            &self.growth_rates,
        ) else {
            return;
        };
        // FillMoves runs before InitEggMoves in Crystal.  The core step hook
        // preserves the non-maternal parent's moves as candidates; only
        // candidates present in this species' canonical egg-move table are
        // copied into the four move slots here.
        if let Some(egg_move_names) = self.egg_moves.get(&species_id).and_then(Value::as_array) {
            for move_id in egg_move_names.iter().filter_map(Value::as_str) {
                if egg.moves.len() >= 4
                    || egg.moves.iter().any(|learned| learned.name == move_id)
                    || !existing.moves.iter().any(|learned| learned.name == move_id)
                {
                    continue;
                }
                let Some(move_data) = self.moves.get(move_id) else {
                    continue;
                };
                egg.moves.push(LearnedMove {
                    name: move_id.to_string(),
                    current_pp: move_data.pp,
                    pp_ups: 0,
                });
            }
        }
        egg.nickname = existing.nickname.clone();
        egg.item = existing.item.clone();
        egg.status = existing.status.clone();
        egg.is_egg = true;
        egg.pokerus = existing.pokerus;
        egg.caught_data = existing.caught_data.clone();
        egg.mail = existing.mail.clone();
        egg.original_trainer_name = existing.original_trainer_name.clone();
        egg.original_trainer_id = existing.original_trainer_id;
        egg.happiness = existing.happiness;
        state.day_care.egg = Some(egg);
    }

    pub fn update_clock_from_datetime(
        &self,
        state: &mut GameState,
        date: GameDate,
        hour: u8,
        minute: u8,
        second: u8,
    ) {
        let previous_day_count = compute_day_count(state.time.rtc_anchor, state.time.current_date);
        let next_day_count = compute_day_count(state.time.rtc_anchor, date);
        let day_delta = next_day_count.saturating_sub(previous_day_count).min(366);
        let day_changed = state.time.current_date != date;
        state.time.update_from_datetime(date, hour, minute, second);
        if day_changed {
            for _ in 0..day_delta.max(1) {
                state.apply_daily_reset();
            }
        }
    }

    pub fn set_manual_clock_time(
        &self,
        state: &mut GameState,
        now_date: GameDate,
        now_hour: u8,
        now_minute: u8,
        now_second: u8,
        target: ClockTime,
    ) {
        // The manual clock screen still drives the same RTC day boundary as
        // the real-time source.  Without this, changing the date through the
        // UI left fishing flags, phone/day events, and Pokérus unchanged until
        // the next wall-clock update.
        let previous_day_count = compute_day_count(state.time.rtc_anchor, state.time.current_date);
        let next_day_count = compute_day_count(state.time.rtc_anchor, now_date);
        let day_delta = next_day_count.saturating_sub(previous_day_count).min(366);
        let day_changed = state.time.current_date != now_date;
        state
            .time
            .set_manual_time(now_date, now_hour, now_minute, now_second, target);
        if day_changed {
            for _ in 0..day_delta.max(1) {
                state.apply_daily_reset();
            }
        }
    }

    pub fn runtime_spawn_point(&self, spawn_identifier: u16) -> Result<&RuntimeSpawnPoint> {
        self.runtime_spawn_points
            .get(&spawn_identifier.to_string())
            .with_context(|| format!("compiled game pack missing spawn point {spawn_identifier}"))
    }

    pub fn runtime_spawn_points(&self) -> &BTreeMap<String, RuntimeSpawnPoint> {
        &self.runtime_spawn_points
    }

    pub fn runtime_spawn_point_for_map_constant(
        &self,
        map_constant: &str,
    ) -> Result<&RuntimeSpawnPoint> {
        let mut matches = self
            .runtime_spawn_points
            .values()
            .filter(|spawn| spawn.map_constant == map_constant);
        let spawn = matches.next().with_context(|| {
            format!("compiled game pack missing spawn point for {map_constant}")
        })?;
        if let Some(other) = matches.next() {
            anyhow::bail!(
                "compiled game pack has multiple spawn points for {map_constant}: {} and {}",
                spawn.identifier,
                other.identifier
            );
        }
        Ok(spawn)
    }

    pub fn map_module(&self, map_name: &str) -> Result<&MapModule> {
        self.maps
            .get(map_name)
            .with_context(|| format!("compiled game pack missing map module {map_name}"))
    }

    pub fn map_declares_script(&self, map_name: &str, script_label: &str) -> Result<bool> {
        Ok(self
            .map_module(map_name)?
            .scripts
            .contains_key(script_label))
    }

    pub fn map_script_labels(&self, map_name: &str) -> Result<BTreeSet<String>> {
        Ok(self.map_module(map_name)?.scripts.keys().cloned().collect())
    }

    pub fn map_tileset_name(&self, map_name: &str) -> Result<&str> {
        Ok(&self.map_module(map_name)?.attributes.tileset_name)
    }

    pub fn map_fishing_group(&self, map_name: &str) -> Result<Option<&str>> {
        Ok(self
            .map_module(map_name)?
            .attributes
            .fishing_group
            .as_deref())
    }

    pub fn map_scene_table(&self, map_name: &str) -> Result<&MapSceneTable> {
        Ok(&self.map_module(map_name)?.scenes)
    }

    pub fn script_text_labels_for_map(&self, map_name: &str) -> Result<BTreeSet<String>> {
        Ok(self
            .map_module(map_name)?
            .script_text_bodies
            .keys()
            .cloned()
            .collect())
    }

    pub fn saved_dig_warp_destination(
        &self,
        state: &GameState,
        context: &str,
    ) -> Result<SavedDigWarpDestination> {
        let Some(map_name) = state.dig_warp_map_name.as_deref() else {
            return core_saved_dig_warp_destination(state, context, &[])
                .map_err(|error| anyhow::anyhow!("{error}"));
        };
        let module = self.map_module(map_name)?;
        core_saved_dig_warp_destination(state, context, &module.events.warps)
            .map_err(|error| anyhow::anyhow!("{error}"))
    }

    pub fn scripted_wild_battle(
        &self,
        map_name: &str,
        source_script: &str,
        startbattle_command_index: usize,
    ) -> Result<&ScriptedWildBattle> {
        self.map_module(map_name)?
            .scripted_wild_battles
            .iter()
            .find(|battle| {
                battle.source_script == source_script
                    && battle.startbattle_command_index == startbattle_command_index
            })
            .with_context(|| {
                format!(
                    "map {map_name} has no scripted wild battle at {source_script}:{startbattle_command_index}"
                )
            })
    }

    pub fn scripted_trainer_battle(
        &self,
        map_name: &str,
        source_script: &str,
        startbattle_command_index: usize,
    ) -> Result<&ScriptedTrainerBattle> {
        self.map_module(map_name)?
            .scripted_trainer_battles
            .iter()
            .find(|battle| {
                battle.source_script == source_script
                    && battle.startbattle_command_index == startbattle_command_index
            })
            .with_context(|| {
                format!(
                    "map {map_name} has no scripted trainer battle at {source_script}:{startbattle_command_index}"
                )
            })
    }

    pub fn scripted_wild_battle_request(
        &self,
        map_name: &str,
        source_script: &str,
        startbattle_command_index: usize,
    ) -> Result<StaticWildBattleRequest> {
        Ok(self
            .scripted_wild_battle(map_name, source_script, startbattle_command_index)?
            .request
            .clone())
    }

    pub fn scripted_wild_battle_pre_flags(
        &self,
        map_name: &str,
        source_script: &str,
        startbattle_command_index: usize,
    ) -> Result<Vec<String>> {
        Ok(self
            .scripted_wild_battle(map_name, source_script, startbattle_command_index)?
            .pre_battle_event_flags
            .clone())
    }

    pub fn scripted_trainer_battle_request(
        &self,
        map_name: &str,
        source_script: &str,
        startbattle_command_index: usize,
    ) -> Result<TrainerBattleRequest> {
        Ok(self
            .scripted_trainer_battle(map_name, source_script, startbattle_command_index)?
            .request
            .clone())
    }

    pub fn scripted_wild_battle_completion_effects(
        &self,
        map_name: &str,
        source_script: &str,
        startbattle_command_index: usize,
    ) -> Result<ScriptedBattleEffects> {
        let battle =
            self.scripted_wild_battle(map_name, source_script, startbattle_command_index)?;
        Ok(ScriptedBattleEffects {
            event_flags: battle.post_battle_event_flags.clone(),
            script_flags: battle.post_battle_script_flags.clone(),
            disappear_object_ids: battle.disappear_object_ids.clone(),
        })
    }

    pub fn scripted_trainer_battle_completion_effects(
        &self,
        map_name: &str,
        source_script: &str,
        startbattle_command_index: usize,
    ) -> Result<ScriptedBattleEffects> {
        let battle =
            self.scripted_trainer_battle(map_name, source_script, startbattle_command_index)?;
        Ok(ScriptedBattleEffects {
            event_flags: battle.post_battle_event_flags.clone(),
            script_flags: battle.post_battle_script_flags.clone(),
            disappear_object_ids: Vec::new(),
        })
    }

    pub fn start_scripted_wild_battle(
        &self,
        state: &mut GameState,
        current_map: &str,
        map_name: &str,
        source_script: &str,
        startbattle_command_index: usize,
    ) -> Result<StaticWildBattleStart> {
        self.require_current_map(current_map, map_name)?;
        let mut request =
            self.scripted_wild_battle_request(map_name, source_script, startbattle_command_index)?;
        request.battle_music =
            self.wild_battle_music_for_map_time(map_name, state.time.time_of_day)?;
        let pre_battle_event_flags = self.scripted_wild_battle_pre_flags(
            map_name,
            source_script,
            startbattle_command_index,
        )?;
        let mut rng = Random::new_crystal(state.rng_seed);
        let start = self
            .static_wild_battle_start(request, &mut rng)
            .with_context(|| {
                format!(
                    "start scripted wild battle at {map_name}/{source_script}:{startbattle_command_index}"
                )
            })?;
        apply_scripted_wild_battle_start(state, &pre_battle_event_flags, &start).map_err(
            |error| {
                anyhow::anyhow!(
                    "apply scripted wild battle start at {map_name}/{source_script}:{startbattle_command_index}: {error:?}"
                )
            },
        )?;
        state.battle_active_party_index = first_available_battle_party_index(state);
        state.battle_active_enemy_party_index = Some(0);
        state.battle_rewarded_enemy_party_indices.clear();
        state.battle_escape_attempts = 0;
        state.battle_player_stat_drop_guard_turns = 0;
        state.battle_pay_day_money = 0;
        set_script_battle_result_accumulator(state);
        state.commit_rng_seed(rng.seed());
        Ok(start)
    }

    pub fn start_scripted_wild_battle_in_session(
        &self,
        state: &mut GameState,
        session: &OverworldSession,
        map_name: &str,
        source_script: &str,
        startbattle_command_index: usize,
    ) -> Result<StaticWildBattleStart> {
        self.start_scripted_wild_battle(
            state,
            &session.map.name,
            map_name,
            source_script,
            startbattle_command_index,
        )
    }

    pub fn start_scripted_trainer_battle(
        &self,
        state: &mut GameState,
        current_map: &str,
        map_name: &str,
        source_script: &str,
        startbattle_command_index: usize,
    ) -> Result<TrainerBattleStartStatus> {
        self.require_current_map(current_map, map_name)?;
        let request = self.scripted_trainer_battle_request(
            map_name,
            source_script,
            startbattle_command_index,
        )?;
        let start = self.trainer_battle_start(state, request).with_context(|| {
            format!(
                "start scripted trainer battle at {map_name}/{source_script}:{startbattle_command_index}"
            )
        })?;
        activate_trainer_battle_start_status(state, &start);
        Ok(start)
    }

    pub fn start_scripted_trainer_battle_in_session(
        &self,
        state: &mut GameState,
        session: &OverworldSession,
        map_name: &str,
        source_script: &str,
        startbattle_command_index: usize,
    ) -> Result<TrainerBattleStartStatus> {
        self.start_scripted_trainer_battle(
            state,
            &session.map.name,
            map_name,
            source_script,
            startbattle_command_index,
        )
    }

    pub fn complete_scripted_wild_battle(
        &self,
        state: &mut GameState,
        overworld: &mut OverworldSession,
        current_map: &str,
        map_name: &str,
        source_script: &str,
        startbattle_command_index: usize,
    ) -> Result<ScriptedBattleEffectsOutcome> {
        self.require_current_map(current_map, map_name)?;
        let effects = self.scripted_wild_battle_completion_effects(
            map_name,
            source_script,
            startbattle_command_index,
        )?;
        let outcome = apply_scripted_battle_effects_to_session(state, overworld, &effects)
            .map_err(|error| {
                anyhow::anyhow!(
                    "complete scripted wild battle at {map_name}/{source_script}:{startbattle_command_index}: {error:?}"
                )
            })?;
        set_script_battle_result_accumulator(state);
        let pay_day_money = self.active_battle_pay_day_payout(state);
        state.spread_pokerus_after_battle();
        deactivate_battle(state);
        self.claim_active_battle_pay_day_money(state, pay_day_money)?;
        Ok(outcome)
    }

    pub fn complete_scripted_wild_battle_in_session(
        &self,
        state: &mut GameState,
        overworld: &mut OverworldSession,
        map_name: &str,
        source_script: &str,
        startbattle_command_index: usize,
    ) -> Result<ScriptedBattleEffectsOutcome> {
        let current_map = overworld.map.name.clone();
        self.complete_scripted_wild_battle(
            state,
            overworld,
            &current_map,
            map_name,
            source_script,
            startbattle_command_index,
        )
    }

    pub fn scripted_trainer_battle_completion(
        &self,
        map_name: &str,
        source_script: &str,
        startbattle_command_index: usize,
        won: bool,
        can_lose: bool,
    ) -> Result<TrainerBattleCompletion> {
        let battle =
            self.scripted_trainer_battle(map_name, source_script, startbattle_command_index)?;
        Ok(TrainerBattleCompletion {
            trainer_id: battle.request.trainer_id.clone(),
            trainer_class: battle.request.trainer_class.clone(),
            event_flag: battle.request.event_flag.clone(),
            won,
            can_lose,
        })
    }

    pub fn complete_scripted_trainer_battle(
        &self,
        state: &mut GameState,
        current_map: &str,
        map_name: &str,
        source_script: &str,
        startbattle_command_index: usize,
        won: bool,
        can_lose: bool,
    ) -> Result<TrainerBattleCompletionOutcome> {
        self.require_current_map(current_map, map_name)?;
        let completion = self.scripted_trainer_battle_completion(
            map_name,
            source_script,
            startbattle_command_index,
            won,
            can_lose,
        )?;
        let mut outcome = core_complete_trainer_battle(state, &self.currency_constants, &completion)
            .with_context(|| {
                format!(
                    "complete scripted trainer battle at {map_name}/{source_script}:{startbattle_command_index}"
                )
            })?;
        if outcome.continued_after_battle {
            outcome.money_after = state.money;
            set_script_trainer_battle_result_accumulator(state, won);
        }
        Ok(outcome)
    }

    pub fn complete_scripted_trainer_battle_in_session(
        &self,
        state: &mut GameState,
        session: &OverworldSession,
        map_name: &str,
        source_script: &str,
        startbattle_command_index: usize,
        won: bool,
        can_lose: bool,
    ) -> Result<TrainerBattleCompletionOutcome> {
        self.complete_scripted_trainer_battle(
            state,
            &session.map.name,
            map_name,
            source_script,
            startbattle_command_index,
            won,
            can_lose,
        )
    }

    pub fn apply_scripted_trainer_battle_completion_effects(
        &self,
        state: &mut GameState,
        overworld: &mut OverworldSession,
        current_map: &str,
        map_name: &str,
        source_script: &str,
        startbattle_command_index: usize,
    ) -> Result<ScriptedBattleEffectsOutcome> {
        self.require_current_map(current_map, map_name)?;
        let effects = self.scripted_trainer_battle_completion_effects(
            map_name,
            source_script,
            startbattle_command_index,
        )?;
        apply_scripted_battle_effects_to_session(state, overworld, &effects).map_err(|error| {
            anyhow::anyhow!(
                "apply scripted trainer post-battle effects at {map_name}/{source_script}:{startbattle_command_index}: {error:?}"
            )
        })
    }

    pub fn apply_scripted_trainer_battle_completion_effects_in_session(
        &self,
        state: &mut GameState,
        overworld: &mut OverworldSession,
        map_name: &str,
        source_script: &str,
        startbattle_command_index: usize,
    ) -> Result<ScriptedBattleEffectsOutcome> {
        let current_map = overworld.map.name.clone();
        self.apply_scripted_trainer_battle_completion_effects(
            state,
            overworld,
            &current_map,
            map_name,
            source_script,
            startbattle_command_index,
        )
    }

    pub fn gift_pokemon_script(
        &self,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<&GiftPokemonScript> {
        self.map_module(map_name)?
            .gift_pokemon_scripts
            .iter()
            .find(|gift| gift.source_script == source_script && gift.command_index == command_index)
            .with_context(|| {
                format!(
                    "map {map_name} has no gift Pokemon script at {source_script}:{command_index}"
                )
            })
    }

    pub fn gift_pokemon_request(
        &self,
        map_name: &str,
        source_script: &str,
        command_index: usize,
        original_trainer_name: impl Into<String>,
        original_trainer_id: u16,
        dvs: Dv,
        nickname_accepted: bool,
        nickname: Option<String>,
    ) -> Result<GiftPokemonRequest> {
        let gift = self.gift_pokemon_script(map_name, source_script, command_index)?;
        match (
            gift.nickname_label.as_deref(),
            nickname_accepted,
            nickname.as_ref(),
        ) {
            (Some(_), true, Some(_)) => {}
            (Some(nickname_label), true, None) => {
                anyhow::bail!(
                    "gift Pokemon at {map_name}/{source_script}:{command_index} requires resolved nickname label {nickname_label}",
                );
            }
            (Some(_), false, None) => {}
            (Some(_), false, Some(_)) => {
                anyhow::bail!(
                    "gift Pokemon at {map_name}/{source_script}:{command_index} refused nickname prompt but supplied nickname"
                );
            }
            (None, false, None) => {}
            (None, true, _) => {
                anyhow::bail!(
                    "gift Pokemon at {map_name}/{source_script}:{command_index} does not declare a nickname prompt"
                );
            }
            (None, false, Some(_)) => {
                anyhow::bail!(
                    "gift Pokemon at {map_name}/{source_script}:{command_index} does not declare a nickname label"
                );
            }
        }
        if nickname.as_deref().is_some_and(str::is_empty) {
            anyhow::bail!(
                "gift Pokemon at {map_name}/{source_script}:{command_index} nickname must be nonempty when accepted"
            );
        }
        Ok(GiftPokemonRequest {
            species_id: gift.species_id.clone(),
            level: gift.level,
            held_item_id: gift.held_item_id.clone(),
            nickname,
            original_trainer_name: original_trainer_name.into(),
            original_trainer_id,
            source_script: gift.source_script.clone(),
            command_index: gift.command_index,
            egg: gift.egg,
            dvs,
        })
    }

    pub fn grant_gift_pokemon_to_state(
        &self,
        state: &mut GameState,
        request: GiftPokemonRequest,
    ) -> Result<GiftPokemonOutcome> {
        core_grant_gift_pokemon_to_state(
            state,
            &self.pokemon,
            &self.learnsets,
            &self.moves,
            &self.growth_rates,
            &self.items,
            request,
        )
        .map_err(|error| anyhow::anyhow!("grant gift Pokemon: {error:?}"))
    }

    pub fn grant_scripted_gift_pokemon(
        &self,
        state: &mut GameState,
        current_map: &str,
        map_name: &str,
        source_script: &str,
        command_index: usize,
        original_trainer_name: impl Into<String>,
        original_trainer_id: u16,
        dvs: Dv,
        nickname_accepted: bool,
        nickname: Option<String>,
    ) -> Result<GiftPokemonOutcome> {
        self.require_current_map(current_map, map_name)?;
        let request = self.gift_pokemon_request(
            map_name,
            source_script,
            command_index,
            original_trainer_name,
            original_trainer_id,
            dvs,
            nickname_accepted,
            nickname,
        )?;
        self.grant_gift_pokemon_to_state(state, request)
            .map_err(|error| {
                anyhow::anyhow!(
                    "grant gift Pokemon at {map_name}/{source_script}:{command_index}: {error:?}"
                )
            })
    }

    pub fn grant_scripted_gift_pokemon_in_session(
        &self,
        state: &mut GameState,
        session: &OverworldSession,
        map_name: &str,
        source_script: &str,
        command_index: usize,
        original_trainer_name: impl Into<String>,
        original_trainer_id: u16,
        dvs: Dv,
        nickname_accepted: bool,
        nickname: Option<String>,
    ) -> Result<GiftPokemonOutcome> {
        self.grant_scripted_gift_pokemon(
            state,
            &session.map.name,
            map_name,
            source_script,
            command_index,
            original_trainer_name,
            original_trainer_id,
            dvs,
            nickname_accepted,
            nickname,
        )
    }

    pub fn script_item_grant(
        &self,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<&ScriptItemGrant> {
        find_script_entry(
            &self.map_module(map_name)?.script_item_grants,
            map_name,
            "script item grant",
            source_script,
            command_index,
            |grant| (&grant.source_script, grant.command_index),
        )
    }

    pub fn script_item_check(
        &self,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<&ScriptItemAccess> {
        find_script_entry(
            &self.map_module(map_name)?.script_item_checks,
            map_name,
            "script item check",
            source_script,
            command_index,
            |access| (&access.source_script, access.command_index),
        )
    }

    pub fn script_item_take(
        &self,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<&ScriptItemAccess> {
        find_script_entry(
            &self.map_module(map_name)?.script_item_takes,
            map_name,
            "script item take",
            source_script,
            command_index,
            |access| (&access.source_script, access.command_index),
        )
    }

    pub fn grant_script_item(
        &self,
        state: &mut GameState,
        current_map: &str,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<ScriptItemGrantOutcome> {
        self.require_current_map(current_map, map_name)?;
        let grant = self
            .script_item_grant(map_name, source_script, command_index)?
            .clone();
        core_grant_script_item(state, &self.items, grant)
            .map_err(|error| anyhow::anyhow!("grant script item: {error:?}"))
    }

    pub fn grant_script_item_in_session(
        &self,
        state: &mut GameState,
        session: &OverworldSession,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<ScriptItemGrantOutcome> {
        self.grant_script_item(
            state,
            &session.map.name,
            map_name,
            source_script,
            command_index,
        )
    }

    pub fn check_script_item(
        &self,
        state: &GameState,
        current_map: &str,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<ScriptItemCheckOutcome> {
        self.require_current_map(current_map, map_name)?;
        let access = self
            .script_item_check(map_name, source_script, command_index)?
            .clone();
        core_check_script_item(state, &self.items, access)
            .map_err(|error| anyhow::anyhow!("check script item: {error:?}"))
    }

    pub fn check_script_item_in_session(
        &self,
        state: &GameState,
        session: &OverworldSession,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<ScriptItemCheckOutcome> {
        self.check_script_item(
            state,
            &session.map.name,
            map_name,
            source_script,
            command_index,
        )
    }

    pub fn take_script_item(
        &self,
        state: &mut GameState,
        current_map: &str,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<ScriptItemTakeOutcome> {
        self.require_current_map(current_map, map_name)?;
        let access = self
            .script_item_take(map_name, source_script, command_index)?
            .clone();
        core_take_script_item(state, &self.items, access)
            .map_err(|error| anyhow::anyhow!("take script item: {error:?}"))
    }

    pub fn take_script_item_in_session(
        &self,
        state: &mut GameState,
        session: &OverworldSession,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<ScriptItemTakeOutcome> {
        self.take_script_item(
            state,
            &session.map.name,
            map_name,
            source_script,
            command_index,
        )
    }

    pub fn script_field_pickup(
        &self,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<&ScriptFieldPickup> {
        find_script_entry(
            &self.map_module(map_name)?.script_field_pickups,
            map_name,
            "script field pickup",
            source_script,
            command_index,
            |pickup| (&pickup.source_script, pickup.command_index),
        )
    }

    pub fn pickup_script_field_item(
        &self,
        state: &mut GameState,
        session: &mut OverworldSession,
        current_map: &str,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<FieldItemPickupOutcome> {
        self.require_current_map(current_map, map_name)?;
        let pickup = self
            .script_field_pickup(map_name, source_script, command_index)?
            .clone();
        let outcome = core_pickup_script_field_item(state, &self.items, &self.fruit_trees, pickup)
            .map_err(|error| anyhow::anyhow!("pickup script field item: {error:?}"))?;
        session.sync_event_flag_memory(&state.flags);
        Ok(outcome)
    }

    pub fn pickup_script_field_item_in_session(
        &self,
        state: &mut GameState,
        session: &mut OverworldSession,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<FieldItemPickupOutcome> {
        let current_map = session.map.name.clone();
        self.pickup_script_field_item(
            state,
            session,
            &current_map,
            map_name,
            source_script,
            command_index,
        )
    }

    pub fn find_itemfinder_hidden_item(
        &self,
        state: &GameState,
        map_name: &str,
        player_tile: TilePosition,
    ) -> Result<Option<crystal_core::systems::field_items::ItemfinderHiddenItem>> {
        let module = self.map_module(map_name)?;
        self.validate_runtime_map_tile("itemfinder player", map_name, player_tile)?;
        core_find_itemfinder_hidden_item(
            state,
            map_name,
            &module.events.bg_events,
            &module.script_field_pickups,
            player_tile,
        )
        .map_err(|error| anyhow::anyhow!("find itemfinder hidden item on {map_name}: {error:?}"))
    }

    pub fn script_economy_command(
        &self,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<&ScriptEconomyCommand> {
        find_script_entry(
            &self.map_module(map_name)?.script_economy_commands,
            map_name,
            "script economy command",
            source_script,
            command_index,
            |command| (&command.source_script, command.command_index),
        )
    }

    pub fn apply_script_economy_command(
        &self,
        state: &mut GameState,
        current_map: &str,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<ScriptEconomyOutcome> {
        self.require_current_map(current_map, map_name)?;
        let command = self
            .script_economy_command(map_name, source_script, command_index)?
            .clone();
        core_apply_script_economy_command(state, command, &self.currency_constants)
            .map_err(|error| anyhow::anyhow!("apply script economy command: {error:?}"))
    }

    pub fn apply_script_economy_command_in_session(
        &self,
        state: &mut GameState,
        session: &OverworldSession,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<ScriptEconomyOutcome> {
        self.apply_script_economy_command(
            state,
            &session.map.name,
            map_name,
            source_script,
            command_index,
        )
    }

    pub fn script_phone_command(
        &self,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<&ScriptPhoneCommand> {
        find_script_entry(
            &self.map_module(map_name)?.script_phone_commands,
            map_name,
            "script phone command",
            source_script,
            command_index,
            |command| (&command.source_script, command.command_index),
        )
    }

    pub fn initialize_permanent_phone_numbers(&self, state: &mut GameState) -> Result<Vec<String>> {
        core_initialize_permanent_phone_numbers(
            state,
            &self.phone_contacts,
            &self.permanent_phone_numbers,
        )
        .map_err(|error| anyhow::anyhow!("initialize permanent phone numbers: {error:?}"))
    }

    pub fn script_swarm_command(
        &self,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<&ScriptSwarmCommand> {
        find_script_entry(
            &self.map_module(map_name)?.script_swarm_commands,
            map_name,
            "script swarm command",
            source_script,
            command_index,
            |command| (&command.source_script, command.command_index),
        )
    }

    fn runtime_map_group_table(&self) -> BTreeMap<String, (u16, u16)> {
        self.runtime_map_metadata
            .values()
            .map(|metadata| {
                (
                    metadata.constant.clone(),
                    (metadata.group_id, metadata.map_id),
                )
            })
            .collect()
    }

    pub fn apply_script_swarm_command(
        &self,
        state: &mut GameState,
        current_map: &str,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<ScriptSwarmOutcome> {
        self.require_current_map(current_map, map_name)?;
        let command = self
            .script_swarm_command(map_name, source_script, command_index)?
            .clone();
        core_apply_script_swarm_command(state, command, &self.runtime_map_group_table())
            .map_err(|error| anyhow::anyhow!("apply script swarm command: {error:?}"))
    }

    pub fn apply_script_swarm_command_in_session(
        &self,
        state: &mut GameState,
        session: &OverworldSession,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<ScriptSwarmOutcome> {
        self.apply_script_swarm_command(
            state,
            &session.map.name,
            map_name,
            source_script,
            command_index,
        )
    }

    pub fn apply_script_phone_command(
        &self,
        state: &mut GameState,
        current_map: &str,
        map_name: &str,
        source_script: &str,
        command_index: usize,
        inputs: ScriptPhoneInputs,
    ) -> Result<ScriptPhoneOutcome> {
        self.require_current_map(current_map, map_name)?;
        let command = self
            .script_phone_command(map_name, source_script, command_index)?
            .clone();
        core_apply_script_phone_command(
            state,
            command,
            &self.phone_contacts,
            &self.permanent_phone_numbers,
            inputs,
        )
        .map_err(|error| anyhow::anyhow!("apply script phone command: {error:?}"))
    }

    pub fn apply_script_phone_command_in_session(
        &self,
        state: &mut GameState,
        session: &OverworldSession,
        map_name: &str,
        source_script: &str,
        command_index: usize,
        inputs: ScriptPhoneInputs,
    ) -> Result<ScriptPhoneOutcome> {
        self.apply_script_phone_command(
            state,
            &session.map.name,
            map_name,
            source_script,
            command_index,
            inputs,
        )
    }

    pub fn script_flag_command(
        &self,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<&ScriptFlagCommand> {
        find_script_entry(
            &self.map_module(map_name)?.script_flag_commands,
            map_name,
            "script flag command",
            source_script,
            command_index,
            |command| (&command.source_script, command.command_index),
        )
    }

    pub fn apply_script_flag_mutation(
        &self,
        state: &mut GameState,
        session: &mut OverworldSession,
        current_map: &str,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<ScriptFlagMutationOutcome> {
        self.require_current_map(current_map, map_name)?;
        let command = self
            .script_flag_command(map_name, source_script, command_index)?
            .clone();
        let outcome = core_apply_script_flag_mutation(state, command)
            .map_err(|error| anyhow::anyhow!("apply script flag mutation: {error:?}"))?;
        session.sync_event_flag_memory(&state.flags);
        Ok(outcome)
    }

    pub fn apply_script_flag_mutation_in_session(
        &self,
        state: &mut GameState,
        session: &mut OverworldSession,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<ScriptFlagMutationOutcome> {
        let current_map = session.map.name.clone();
        self.apply_script_flag_mutation(
            state,
            session,
            &current_map,
            map_name,
            source_script,
            command_index,
        )
    }

    pub fn check_script_flag(
        &self,
        state: &mut GameState,
        current_map: &str,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<ScriptFlagCheckOutcome> {
        self.require_current_map(current_map, map_name)?;
        let command = self
            .script_flag_command(map_name, source_script, command_index)?
            .clone();
        let outcome = core_check_script_flag(state, command)
            .map_err(|error| anyhow::anyhow!("check script flag: {error:?}"))?;
        state.script_runtime.script_value = Some(if outcome.set { "1" } else { "0" }.to_string());
        Ok(outcome)
    }

    pub fn check_script_flag_in_session(
        &self,
        state: &mut GameState,
        session: &OverworldSession,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<ScriptFlagCheckOutcome> {
        self.check_script_flag(
            state,
            &session.map.name,
            map_name,
            source_script,
            command_index,
        )
    }

    pub fn script_scene_command(
        &self,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<&ScriptSceneCommand> {
        find_script_entry(
            &self.map_module(map_name)?.script_scene_commands,
            map_name,
            "script scene command",
            source_script,
            command_index,
            |command| (&command.source_script, command.command_index),
        )
    }

    pub fn apply_script_scene_command(
        &self,
        state: &mut GameState,
        current_map: &str,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<ScriptSceneOutcome> {
        self.require_current_map(current_map, map_name)?;
        let command = self
            .script_scene_command(map_name, source_script, command_index)?
            .clone();
        let source_scene_table = self.map_scene_table(map_name)?;
        if !source_scene_table.scenes.is_empty() {
            state
                .scenes
                .enter_map(map_name, source_scene_table)
                .map_err(|error| {
                    anyhow::anyhow!("enter scene context for {map_name}: {error:?}")
                })?;
        }
        let (target_map_name, scene_table) = if let Some(target_map_id) = command.map_id.as_deref()
        {
            let target_map_name = self.map_name_for_constant(target_map_id).with_context(|| {
                format!("script scene command references missing map id {target_map_id}")
            })?;
            let target_scene_table = self.map_scene_table(&target_map_name)?;
            (Some(target_map_name), target_scene_table)
        } else {
            (None, source_scene_table)
        };
        let outcome = core_apply_script_scene_command(
            state,
            map_name,
            target_map_name.as_deref(),
            scene_table,
            command,
        )
        .map_err(|error| anyhow::anyhow!("apply script scene command: {error:?}"))?;
        if outcome.command == "checkscene" {
            state.script_runtime.script_value = Some(outcome.scene_index.to_string());
        }
        Ok(outcome)
    }

    pub fn apply_script_scene_command_in_session(
        &self,
        state: &mut GameState,
        session: &OverworldSession,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<ScriptSceneOutcome> {
        self.apply_script_scene_command(
            state,
            &session.map.name,
            map_name,
            source_script,
            command_index,
        )
    }

    pub fn script_block_change(
        &self,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<&ScriptBlockChange> {
        find_script_entry(
            &self.map_module(map_name)?.script_block_changes,
            map_name,
            "script block change",
            source_script,
            command_index,
            |change| (&change.source_script, change.command_index),
        )
    }

    pub fn apply_script_block_change(
        &self,
        state: &mut GameState,
        session: &mut OverworldSession,
        current_map: &str,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<ScriptBlockChangeOutcome> {
        self.require_current_map(current_map, map_name)?;
        let change = self
            .script_block_change(map_name, source_script, command_index)?
            .clone();
        let outcome = core_apply_script_block_change(&mut session.map, change)
            .map_err(|error| anyhow::anyhow!("apply script block change: {error:?}"))?;
        state
            .map_block_overrides
            .entry(outcome.map_name.clone())
            .or_default()
            .insert((outcome.metatile_x, outcome.metatile_y), outcome.block_id);
        Ok(outcome)
    }

    pub fn apply_script_block_change_in_session(
        &self,
        state: &mut GameState,
        session: &mut OverworldSession,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<ScriptBlockChangeOutcome> {
        let current_map = session.map.name.clone();
        self.apply_script_block_change(
            state,
            session,
            &current_map,
            map_name,
            source_script,
            command_index,
        )
    }

    pub fn script_audio_command(
        &self,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<&ScriptAudioCommand> {
        find_script_entry(
            &self.map_module(map_name)?.script_audio_commands,
            map_name,
            "script audio command",
            source_script,
            command_index,
            |command| (&command.source_script, command.command_index),
        )
    }

    pub fn apply_script_audio_command(
        &self,
        state: &mut GameState,
        current_map: &str,
        map_name: &str,
        source_script: &str,
        command_index: usize,
        music_ids: &BTreeSet<String>,
        sound_effect_ids: &BTreeSet<String>,
        cry_ids: &BTreeSet<String>,
    ) -> Result<ScriptAudioCue> {
        self.require_current_map(current_map, map_name)?;
        let command = self
            .script_audio_command(map_name, source_script, command_index)?
            .clone();
        let cry_by_species = self.cry_by_species();
        core_apply_script_audio_command(
            state,
            command,
            music_ids,
            sound_effect_ids,
            cry_ids,
            &self.pokemon,
            &cry_by_species,
        )
        .map_err(|error| anyhow::anyhow!("apply script audio command: {error:?}"))
    }

    pub fn apply_script_audio_command_in_session(
        &self,
        state: &mut GameState,
        session: &OverworldSession,
        map_name: &str,
        source_script: &str,
        command_index: usize,
        music_ids: &BTreeSet<String>,
        sound_effect_ids: &BTreeSet<String>,
        cry_ids: &BTreeSet<String>,
    ) -> Result<ScriptAudioCue> {
        self.apply_script_audio_command(
            state,
            &session.map.name,
            map_name,
            source_script,
            command_index,
            music_ids,
            sound_effect_ids,
            cry_ids,
        )
    }

    pub fn script_map_command(
        &self,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<&ScriptMapCommand> {
        find_script_entry(
            &self.map_module(map_name)?.script_map_commands,
            map_name,
            "script map command",
            source_script,
            command_index,
            |command| (&command.source_script, command.command_index),
        )
    }

    pub fn apply_script_map_command(
        &self,
        state: &mut GameState,
        current_map: &str,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<ScriptMapAction> {
        self.require_current_map(current_map, map_name)?;
        let command = self
            .script_map_command(map_name, source_script, command_index)?
            .clone();
        core_apply_script_map_command(state, command, &self.map_ids())
            .map_err(|error| anyhow::anyhow!("apply script map command: {error:?}"))
    }

    pub fn apply_script_map_command_in_session(
        &self,
        state: &mut GameState,
        session: &OverworldSession,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<ScriptMapAction> {
        self.apply_script_map_command(
            state,
            &session.map.name,
            map_name,
            source_script,
            command_index,
        )
    }

    pub fn complete_pending_script_warp(
        &self,
        state: &mut GameState,
        session: &mut OverworldSession,
    ) -> Result<ScriptWarpRequest> {
        let request = state
            .script_runtime
            .pending_script_warp
            .clone()
            .with_context(|| "cannot execute script warp without a pending script warp")?;
        apply_script_warp_arrival_facing(&mut session.player, &request);
        complete_pending_script_warp(state, &request)
            .map_err(|error| anyhow::anyhow!("complete pending script warp: {error:?}"))
    }

    pub fn transition_pending_script_warp(
        &self,
        state: &mut GameState,
        session: &mut OverworldSession,
        music_ids: &BTreeSet<String>,
    ) -> Result<ScriptWarpRequest> {
        let request = state
            .script_runtime
            .pending_script_warp
            .clone()
            .with_context(|| "cannot execute script warp without a pending script warp")?;
        let frame = session.frame;
        let mode = MovementMode::Normal;
        *session = self.overworld_session_for_traversal(
            &request.target_map,
            request.tile,
            frame,
            mode.traversal_state(),
        )?;
        session.player.mode = mode;
        clear_transient_map_object_context(state, session);
        reset_map_bike_flags(state)?;
        // EnterMap arms wWildEncounterCooldown before running map setup.
        // CheckWildEncounterCooldown permits the fifth completed step after
        // decrementing 1 -> 0, so this state must be persisted outside the
        // transient OverworldSession.
        state.wild_encounter_cooldown = 5;
        self.complete_pending_script_warp(state, session)?;
        self.apply_saved_overworld_overrides(session, state)?;
        let mode = self.map_entry_movement_mode(state, session, mode)?;
        session.player.mode = mode;
        self.sync_current_map_music(state, &request.target_map, mode, music_ids)?;
        self.sync_current_map_scene(state, &request.target_map)?;
        self.apply_map_object_callbacks(state, session, &request.target_map)?;
        let callback_mode = self.map_entry_movement_mode(state, session, session.player.mode)?;
        if callback_mode != session.player.mode {
            session.player.mode = callback_mode;
            self.sync_current_map_music(state, &request.target_map, callback_mode, music_ids)?;
        }
        self.commit_overworld_snapshot(state, session, SpawnMemoryUpdate::Preserve);
        Ok(request)
    }

    pub fn script_text_command(
        &self,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<&ScriptTextCommand> {
        find_script_entry(
            &self.map_module(map_name)?.script_text_commands,
            map_name,
            "script text command",
            source_script,
            command_index,
            |command| (&command.source_script, command.command_index),
        )
    }

    pub fn apply_script_text_command(
        &self,
        state: &mut GameState,
        current_map: &str,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<ScriptTextAction> {
        self.require_current_map(current_map, map_name)?;
        let command = self
            .script_text_command(map_name, source_script, command_index)?
            .clone();
        let text_labels = self.script_text_labels_for_map(map_name)?;
        core_apply_script_text_command(state, command, &text_labels)
            .map_err(|error| anyhow::anyhow!("apply script text command: {error:?}"))
    }

    pub fn apply_script_text_command_in_session(
        &self,
        state: &mut GameState,
        session: &mut OverworldSession,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<ScriptTextAction> {
        self.require_current_map(&session.map.name, map_name)?;
        let command = self
            .script_text_command(map_name, source_script, command_index)?
            .clone();
        let text_labels = self.script_text_labels_for_map(map_name)?;
        let action = core_resolve_script_text_command(command.clone(), &text_labels)
            .map_err(|error| anyhow::anyhow!("resolve script text command: {error:?}"))?;
        if let ScriptTextAction::Write {
            face_player: true,
            source_script,
            command_index,
            ..
        } = &action
        {
            let face_player = ScriptObjectCommand {
                command: "faceplayer".to_string(),
                object_id: None,
                target_object_id: None,
                x: None,
                y: None,
                direction: None,
                movement: None,
                emote: None,
                duration: None,
                source_script: source_script.clone(),
                command_index: *command_index,
            };
            core_apply_script_object_mutation(state, session, &face_player).map_err(|error| {
                anyhow::anyhow!("apply jumptextfaceplayer object facing: {error:?}")
            })?;
        }
        core_apply_script_text_command(state, command, &text_labels)
            .map_err(|error| anyhow::anyhow!("apply script text command: {error:?}"))?;
        Ok(action)
    }

    pub fn script_variable_command(
        &self,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<&ScriptVariableCommand> {
        find_script_entry(
            &self.map_module(map_name)?.script_variable_commands,
            map_name,
            "script variable command",
            source_script,
            command_index,
            |command| (&command.source_script, command.command_index),
        )
    }

    pub fn apply_script_variable_command(
        &self,
        state: &mut GameState,
        current_map: &str,
        map_name: &str,
        source_script: &str,
        command_index: usize,
        time_of_day: Option<TimeOfDay>,
    ) -> Result<ScriptVariableOutcome> {
        self.require_current_map(current_map, map_name)?;
        let command = self
            .script_variable_command(map_name, source_script, command_index)?
            .clone();
        core_apply_script_variable_command(state, command, time_of_day)
            .map_err(|error| anyhow::anyhow!("apply script variable command: {error:?}"))
    }

    pub fn apply_script_variable_command_now(
        &self,
        state: &mut GameState,
        current_map: &str,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<ScriptVariableOutcome> {
        let time_of_day = state.time.time_of_day;
        self.apply_script_variable_command(
            state,
            current_map,
            map_name,
            source_script,
            command_index,
            Some(time_of_day),
        )
    }

    pub fn apply_script_variable_command_now_in_session(
        &self,
        state: &mut GameState,
        session: &OverworldSession,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<ScriptVariableOutcome> {
        let command = self.script_variable_command(map_name, source_script, command_index)?;
        if command.command == "readvar" {
            let derived = match command.target.as_deref() {
                Some("VAR_FACING") => {
                    Some(direction_script_token(session.player.facing).to_string())
                }
                Some("VAR_WEEKDAY") => Some(state.time.day_of_week.to_string()),
                Some("VAR_HOUR") => Some(state.time.game_time_hours.to_string()),
                Some("VAR_XCOORD") => Some(session.player.tile.x.to_string()),
                Some("VAR_YCOORD") => Some(session.player.tile.y.to_string()),
                Some("VAR_BLUECARDBALANCE") => Some(state.blue_card_balance.to_string()),
                Some("VAR_PARTYCOUNT") => Some(
                    state
                        .storage
                        .party
                        .pokemon
                        .iter()
                        .filter(|pokemon| pokemon.is_some())
                        .count()
                        .to_string(),
                ),
                Some("VAR_BADGES") => Some(
                    state
                        .badges
                        .johto
                        .iter()
                        .filter(|badge| **badge)
                        .count()
                        .to_string(),
                ),
                Some("VAR_UNOWNCOUNT") => Some(
                    state
                        .pokedex
                        .caught_species
                        .iter()
                        .filter(|species| species.as_str() == "UNOWN")
                        .count()
                        .to_string(),
                ),
                _ => None,
            };
            if let Some(value) = derived {
                state
                    .script_runtime
                    .variables
                    .insert(command.target.clone().unwrap_or_default(), value);
            }
        }
        self.apply_script_variable_command_now(
            state,
            &session.map.name,
            map_name,
            source_script,
            command_index,
        )
    }

    pub fn script_control_command(
        &self,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<&ScriptControlCommand> {
        find_script_entry(
            &self.map_module(map_name)?.script_control_commands,
            map_name,
            "script control command",
            source_script,
            command_index,
            |command| (&command.source_script, command.command_index),
        )
    }

    pub fn apply_script_control_command(
        &self,
        state: &mut GameState,
        current_map: &str,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<ScriptControlAction> {
        self.require_current_map(current_map, map_name)?;
        let command = self
            .script_control_command(map_name, source_script, command_index)?
            .clone();
        let constants = self.script_numeric_constants();
        core_apply_script_control_command(state, map_name, command, &constants)
            .map_err(|error| anyhow::anyhow!("apply script control command: {error:?}"))
    }

    pub fn apply_script_control_command_in_session(
        &self,
        state: &mut GameState,
        session: &OverworldSession,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<ScriptControlAction> {
        self.apply_script_control_command(
            state,
            &session.map.name,
            map_name,
            source_script,
            command_index,
        )
    }

    pub fn script_object_command(
        &self,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<&ScriptObjectCommand> {
        find_script_entry(
            &self.map_module(map_name)?.script_object_commands,
            map_name,
            "script object command",
            source_script,
            command_index,
            |command| (&command.source_script, command.command_index),
        )
    }

    pub fn apply_script_object_mutation(
        &self,
        state: &mut GameState,
        session: &mut OverworldSession,
        current_map: &str,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<ScriptObjectMutationOutcome> {
        self.require_current_map(current_map, map_name)?;
        let command = self
            .script_object_command(map_name, source_script, command_index)?
            .clone();
        let mut next_state = state.clone();
        let mut next_session = session.clone();
        let outcome =
            core_apply_script_object_mutation(&mut next_state, &mut next_session, &command)
                .map_err(|error| anyhow::anyhow!("apply script object mutation: {error:?}"))?;
        sync_state_object_overrides(&mut next_state, &next_session)
            .context("sync script object overrides")?;
        *state = next_state;
        *session = next_session;
        Ok(outcome)
    }

    pub fn apply_script_object_mutation_in_session(
        &self,
        state: &mut GameState,
        session: &mut OverworldSession,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<ScriptObjectMutationOutcome> {
        let current_map = session.map.name.clone();
        self.apply_script_object_mutation(
            state,
            session,
            &current_map,
            map_name,
            source_script,
            command_index,
        )
    }

    pub fn apply_script_movement(
        &self,
        state: &mut GameState,
        session: &mut OverworldSession,
        current_map: &str,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<ScriptMovementOutcome> {
        self.require_current_map(current_map, map_name)?;
        let command = self
            .script_object_command(map_name, source_script, command_index)?
            .clone();
        let movement_label = command.movement.as_deref().with_context(|| {
            format!(
                "script movement command at {source_script}:{command_index} has no movement label"
            )
        })?;
        let movement = self
            .script_movement(map_name, &command.source_script, movement_label)?
            .clone();
        let mut next_state = state.clone();
        let mut next_session = session.clone();
        let outcome = core_apply_script_movement(&mut next_session, &command, &movement)
            .map_err(|error| anyhow::anyhow!("apply script movement: {error:?}"))?;
        Self::apply_script_movement_effects_to_state(&mut next_state, &mut next_session, &outcome)?;
        sync_state_object_overrides(&mut next_state, &next_session)
            .context("sync script movement object overrides")?;
        *state = next_state;
        *session = next_session;
        Ok(outcome)
    }

    pub fn apply_script_movement_in_session(
        &self,
        state: &mut GameState,
        session: &mut OverworldSession,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<ScriptMovementOutcome> {
        let current_map = session.map.name.clone();
        self.apply_script_movement(
            state,
            session,
            &current_map,
            map_name,
            source_script,
            command_index,
        )
    }

    fn apply_script_movement_effects_to_state(
        state: &mut GameState,
        session: &mut OverworldSession,
        outcome: &ScriptMovementOutcome,
    ) -> Result<()> {
        for effect in &outcome.effects {
            match effect.command.as_str() {
                "teleport_from" => state.script_runtime.teleport_from_queued = true,
                "teleport_to" => state.script_runtime.teleport_from_queued = false,
                "hide_object" | "remove_object" | "step_dig" => {
                    Self::apply_script_movement_visibility_effect(state, session, outcome, true)?;
                }
                "show_object" | "return_dig" => {
                    Self::apply_script_movement_visibility_effect(state, session, outcome, false)?;
                }
                "hide_emote" => {
                    state
                        .script_runtime
                        .pending_emotes
                        .retain(|emote| emote.object != outcome.object_id);
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn apply_script_movement_visibility_effect(
        state: &mut GameState,
        session: &mut OverworldSession,
        outcome: &ScriptMovementOutcome,
        hidden: bool,
    ) -> Result<()> {
        if outcome.object_id == "PLAYER" {
            session.player_hidden = hidden;
            return Ok(());
        }
        let object = session
            .objects
            .iter()
            .find(|object| object.object_identifier.as_deref() == Some(outcome.object_id.as_str()))
            .with_context(|| {
                format!(
                    "script movement {} references unknown object {}",
                    outcome.movement, outcome.object_id
                )
            })?;
        let event_flag = object.event_flag.clone();
        if event_flag == "-1" {
            if hidden {
                session
                    .hidden_object_identifiers
                    .insert(outcome.object_id.clone());
            } else {
                session.hidden_object_identifiers.remove(&outcome.object_id);
            }
            return Ok(());
        }
        if !is_hideable_object_event_flag(&event_flag) {
            anyhow::bail!(
                "script movement {} cannot toggle object {} with event flag {}",
                outcome.movement,
                outcome.object_id,
                event_flag
            );
        }
        state
            .flags
            .set_event_flag(&event_flag, hidden)
            .with_context(|| {
                format!(
                    "script movement {} toggles object {} event flag {}",
                    outcome.movement, outcome.object_id, event_flag
                )
            })?;
        session.sync_event_flag_memory(&state.flags);
        Ok(())
    }

    pub fn script_runtime_command(
        &self,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<&ScriptRuntimeCommand> {
        find_script_entry(
            &self.map_module(map_name)?.script_runtime_commands,
            map_name,
            "script runtime command",
            source_script,
            command_index,
            |command| (&command.source_script, command.command_index),
        )
    }

    pub fn apply_script_runtime_command(
        &self,
        state: &mut GameState,
        overworld: &mut OverworldSession,
        current_map: &str,
        map_name: &str,
        source_script: &str,
        command_index: usize,
        inputs: ScriptRuntimeInputs,
    ) -> Result<(ScriptRuntimeCommand, ScriptRuntimeOutcome)> {
        self.require_current_map(current_map, map_name)?;
        let command = self
            .script_runtime_command(map_name, source_script, command_index)?
            .clone();
        if let Some(object_id) = Self::script_runtime_live_object_reference_for_command(&command)? {
            Self::require_runtime_object_reference(overworld, &object_id)?;
        }
        let trade_was_completed = command.command == "trade"
            && command
                .args
                .first()
                .is_some_and(|trade_id| state.script_runtime.completed_trades.contains(trade_id));
        let mut next_state = state.clone();
        let mut next_overworld = overworld.clone();
        let selected_party_index = inputs.selected_party_index;
        let outcome =
            core_apply_script_runtime_command(&mut next_state, map_name, command.clone(), inputs)
                .map_err(|error| anyhow::anyhow!("apply script runtime command: {error:?}"))?;
        if command.command == "trade" && !trade_was_completed {
            self.apply_npc_trade(
                &mut next_state,
                command.args.first().map(String::as_str),
                selected_party_index,
            )?;
        }
        if command.command == "givepokemail" {
            self.apply_compiled_mail_definition(&mut next_state, &command.args[0])?;
        } else if command.command == "checkpokemail" {
            self.apply_compiled_mail_check(
                &mut next_state,
                &command.args[0],
                selected_party_index,
            )?;
        }
        self.resolve_script_runtime_name_buffer(&mut next_state, &command)?;
        if command.command == "elevfloor" {
            let target_constant = command.args.get(2).with_context(|| {
                format!(
                    "elevfloor command {}:{} missing target map argument",
                    command.source_script, command.command_index
                )
            })?;
            let target_map = self.map_name_for_constant(target_constant)?;
            let floor = next_state
                .script_runtime
                .elevator_floors
                .last_mut()
                .with_context(|| {
                    format!(
                        "elevfloor command {}:{} did not enqueue an elevator floor",
                        command.source_script, command.command_index
                    )
                })?;
            floor.target_map = target_map;
        }
        if command.command == "blackoutmod" {
            let map_constant = command.args.first().with_context(|| {
                format!(
                    "blackoutmod command {}:{} missing target map argument",
                    command.source_script, command.command_index
                )
            })?;
            let spawn = self.runtime_spawn_point_for_map_constant(map_constant)?;
            next_state.last_spawn_identifier = Some(spawn.identifier);
        }
        if command.command == "setlasttalked" {
            next_overworld.last_talked_object_identifier = command.args.first().cloned();
            sync_state_object_overrides(&mut next_state, &next_overworld)
                .context("sync setlasttalked object overrides")?;
        }
        *state = next_state;
        *overworld = next_overworld;
        Ok((command, outcome))
    }

    /// Execute Crystal's local NPC trade after the script command has passed
    /// through the core runtime. The ASM removes the selected requested
    /// species and appends a freshly built traded Pokémon with the table's
    /// nickname, DVs, held item, OT, and caught-data provenance.
    fn apply_npc_trade(
        &self,
        state: &mut GameState,
        trade_id: Option<&str>,
        selected_party_index: Option<usize>,
    ) -> Result<()> {
        let Some(trade_id) = trade_id else {
            anyhow::bail!("NPC trade command is missing its trade id");
        };
        let rule = self
            .npc_trades
            .get(trade_id)
            .with_context(|| format!("NPC trade {trade_id} is missing from the compiled pack"))?;
        let Some(party_index) = selected_party_index else {
            set_npc_trade_result(state, 0);
            return Ok(());
        };
        let requested = rule.requested_species.as_str();
        let Some(requested_mon) = state
            .storage
            .party
            .pokemon
            .get(party_index)
            .and_then(Option::as_ref)
        else {
            set_npc_trade_result(state, 0);
            return Ok(());
        };
        if requested_mon.species.id != requested {
            set_npc_trade_result(state, 1);
            return Ok(());
        }
        let offered_species = self.pokemon.get(&rule.offered_species).with_context(|| {
            format!(
                "NPC trade {trade_id} references unknown offered species {}",
                rule.offered_species
            )
        })?;
        if rule.gender_requirement.ends_with("FEMALE")
            && !(requested_mon.species.gender_ratio == 254
                || (requested_mon.species.gender_ratio != 0
                    && requested_mon.dvs.attack.saturating_mul(17)
                        < requested_mon.species.gender_ratio))
        {
            set_npc_trade_result(state, 1);
            return Ok(());
        }
        let level = requested_mon.level;
        let dvs = rule.dvs.as_slice();
        if dvs.len() != 2 {
            anyhow::bail!("NPC trade {trade_id} has invalid two-byte DVs");
        }
        let traded_dvs = Dv::from_non_hp(dvs[0] >> 4, dvs[0] & 0x0f, dvs[1] >> 4, dvs[1] & 0x0f);
        let mut traded = create_pokemon_from_known_dvs(
            offered_species,
            level,
            traded_dvs,
            &self.learnsets,
            &self.moves,
            &self.growth_rates,
        )
        .map_err(|error| anyhow::anyhow!("build NPC trade {trade_id} Pokémon: {error}"))?;
        traded.nickname = rule.nickname.clone();
        traded.item = (!rule.held_item.is_empty()).then(|| rule.held_item.clone());
        traded.original_trainer_name = rule.original_trainer_name.clone();
        traded.original_trainer_id = rule.original_trainer_id;
        traded.caught_data = Some(crystal_core::models::pokemon::CaughtData {
            level,
            // SetGiftPartyMonCaughtData writes LANDMARK_GIFT ($7e); the
            // girl trade variant carries the CAUGHT_BY_GIRL bit in the
            // high bit of the caught-location byte.
            ball: 0,
            location: if rule.dialog_set.ends_with("GIRL") {
                0xfe
            } else {
                0x7e
            },
        });
        let last_party_index = state
            .storage
            .party
            .pokemon
            .iter()
            .rposition(Option::is_some)
            .context("NPC trade party unexpectedly became empty")?;
        for slot in party_index..last_party_index {
            state.storage.party.pokemon[slot] =
                state.storage.party.pokemon[slot + 1].take();
        }
        state.storage.party.pokemon[last_party_index] = Some(traded);
        state.sync_party_from_storage();
        state
            .script_runtime
            .completed_trades
            .push(trade_id.to_string());
        set_npc_trade_result(state, 2);
        Ok(())
    }

    fn apply_compiled_mail_definition(&self, state: &mut GameState, label: &str) -> Result<()> {
        let body = self
            .compiled_script_body(label)
            .with_context(|| format!("mail definition '{label}' is missing from compiled ASM"))?;
        let entries = body
            .as_array()
            .with_context(|| format!("mail definition '{label}' is not an array"))?;
        let item_id = entries
            .first()
            .and_then(|entry| entry.get("args"))
            .and_then(serde_json::Value::as_array)
            .and_then(|args| args.first())
            .and_then(serde_json::Value::as_str)
            .with_context(|| format!("mail definition '{label}' has no item"))?
            .to_string();
        self.item(&item_id).with_context(|| {
            format!("mail definition '{label}' references unknown item '{item_id}'")
        })?;
        let message = compiled_mail_message(entries)?;
        let Some(index) = state
            .storage
            .party
            .pokemon
            .iter()
            .rposition(Option::is_some)
        else {
            anyhow::bail!("cannot give mail '{label}' without a party Pokémon");
        };
        let pokemon = state.storage.party.pokemon[index]
            .as_mut()
            .expect("party index selected from occupied slots");
        pokemon.item = Some(item_id);
        pokemon.mail = Some(crystal_core::models::pokemon::MailData {
            message,
            author: pokemon.original_trainer_name.clone(),
            species: pokemon.species.id.clone(),
        });
        state.sync_party_from_storage();
        Ok(())
    }

    fn apply_compiled_mail_check(
        &self,
        state: &mut GameState,
        label: &str,
        selected_party_index: Option<usize>,
    ) -> Result<()> {
        let body = self.compiled_script_body(label).with_context(|| {
            format!("mail check definition '{label}' is missing from compiled ASM")
        })?;
        let entries = body
            .as_array()
            .with_context(|| format!("mail check definition '{label}' is not an array"))?;
        let expected = compiled_mail_message(entries)?;
        let Some(index) = selected_party_index else {
            set_compiled_mail_check_result(state, 2);
            return Ok(());
        };
        let Some(pokemon) = state
            .storage
            .party
            .pokemon
            .get(index)
            .and_then(Option::as_ref)
        else {
            set_compiled_mail_check_result(state, 2);
            return Ok(());
        };
        if !pokemon
            .item
            .as_deref()
            .is_some_and(crystal_core::models::item::is_mail_item_id)
            || pokemon.mail.is_none()
        {
            set_compiled_mail_check_result(state, 3);
            return Ok(());
        }
        if pokemon
            .mail
            .as_ref()
            .is_some_and(|mail| !strip_compiled_mail_text(&mail.message).starts_with(&expected))
        {
            set_compiled_mail_check_result(state, 0);
            return Ok(());
        }
        let party_len = state.storage.party.pokemon.len();
        let other_conscious =
            state
                .storage
                .party
                .pokemon
                .iter()
                .enumerate()
                .any(|(other_index, pokemon)| {
                    other_index != index && pokemon.as_ref().is_some_and(|pokemon| pokemon.hp > 0)
                });
        if !other_conscious {
            set_compiled_mail_check_result(state, 4);
            return Ok(());
        }
        for slot in index..(party_len - 1) {
            state.storage.party.pokemon[slot] = state.storage.party.pokemon[slot + 1].take();
        }
        state.storage.party.pokemon[party_len - 1] = None;
        state.sync_party_from_storage();
        set_compiled_mail_check_result(state, 1);
        Ok(())
    }

    fn resolve_script_runtime_name_buffer(
        &self,
        state: &mut GameState,
        command: &ScriptRuntimeCommand,
    ) -> Result<()> {
        let Some(target_buffer) = command.args.first() else {
            return Ok(());
        };
        let resolved_name = match command.command.as_str() {
            "gettrainername" => {
                let trainer_id = command.args.get(2).with_context(|| {
                    format!(
                        "gettrainername command {}:{} missing trainer id",
                        command.source_script, command.command_index
                    )
                })?;
                let trainer = self.trainers.get(trainer_id).with_context(|| {
                    format!("gettrainername references unknown trainer {trainer_id}")
                })?;
                Some(trainer.name.clone())
            }
            "getitemname" => {
                let item_arg = command.args.get(1).with_context(|| {
                    format!(
                        "getitemname command {}:{} missing item id",
                        command.source_script, command.command_index
                    )
                })?;
                let item_id = if item_arg == SCRIPT_RUNTIME_USE_SCRIPT_VAR_ID {
                    Some(state.script_runtime.script_value.as_deref().with_context(|| {
                        format!(
                            "getitemname command {}:{} requires script_value for USE_SCRIPT_VAR",
                            command.source_script, command.command_index
                        )
                    })?)
                } else if item_arg == SCRIPT_RUNTIME_ITEM_FROM_MEMORY_ID {
                    None
                } else {
                    Some(item_arg.as_str())
                };
                if let Some(item_id) = item_id {
                    let item = self.items.get(item_id).with_context(|| {
                        format!("getitemname references unknown item {item_id}")
                    })?;
                    Some(item.name.clone())
                } else {
                    None
                }
            }
            "getmonname" => {
                let species_arg = command.args.get(1).with_context(|| {
                    format!(
                        "getmonname command {}:{} missing species id",
                        command.source_script, command.command_index
                    )
                })?;
                let species_id =
                    if species_arg == SCRIPT_RUNTIME_USE_SCRIPT_VAR_ID {
                        state.script_runtime.script_value.as_deref().with_context(|| {
                        format!(
                            "getmonname command {}:{} requires script_value for USE_SCRIPT_VAR",
                            command.source_script, command.command_index
                        )
                    })?
                    } else {
                        species_arg.as_str()
                    };
                self.pokemon.get(species_id).with_context(|| {
                    format!("getmonname references unknown species {species_id}")
                })?;
                Some(pokemon_species_display_name(species_id))
            }
            _ => None,
        };
        if let Some(name) = resolved_name {
            state
                .script_runtime
                .named_buffers
                .insert(target_buffer.clone(), name);
        }
        Ok(())
    }

    pub fn apply_script_runtime_command_in_session(
        &self,
        state: &mut GameState,
        overworld: &mut OverworldSession,
        map_name: &str,
        source_script: &str,
        command_index: usize,
        inputs: ScriptRuntimeInputs,
    ) -> Result<(ScriptRuntimeCommand, ScriptRuntimeOutcome)> {
        let current_map = overworld.map.name.clone();
        self.apply_script_runtime_command(
            state,
            overworld,
            &current_map,
            map_name,
            source_script,
            command_index,
            inputs,
        )
    }

    pub fn script_runtime_live_object_reference(
        &self,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<Option<String>> {
        let command = self.script_runtime_command(map_name, source_script, command_index)?;
        Self::script_runtime_live_object_reference_for_command(command)
    }

    fn script_runtime_live_object_reference_for_command(
        command: &ScriptRuntimeCommand,
    ) -> Result<Option<String>> {
        if command.command != "setlasttalked" {
            return Ok(None);
        }
        let object_id = command
            .args
            .first()
            .with_context(|| "setlasttalked command missing object id")?;
        Ok(Some(object_id.clone()))
    }

    fn require_runtime_object_reference(
        overworld: &OverworldSession,
        object_id: &str,
    ) -> Result<()> {
        if object_id == "PLAYER" {
            return Ok(());
        }
        if overworld
            .objects
            .iter()
            .any(|object| object.object_identifier.as_deref() == Some(object_id))
        {
            return Ok(());
        }
        anyhow::bail!(
            "runtime command references missing exact object id {object_id} on {}",
            overworld.map.name
        );
    }

    pub fn script_shop_command(
        &self,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<&ScriptShopCommand> {
        find_script_entry(
            &self.map_module(map_name)?.script_shop_commands,
            map_name,
            "script shop command",
            source_script,
            command_index,
            |command| (&command.source_script, command.command_index),
        )
    }

    pub fn open_script_shop(
        &self,
        state: &mut GameState,
        current_map: &str,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<ScriptShopOutcome> {
        self.require_current_map(current_map, map_name)?;
        let command = self
            .script_shop_command(map_name, source_script, command_index)?
            .clone();
        core_apply_script_shop_command(state, &self.marts, &self.items, command)
            .map_err(|error| anyhow::anyhow!("open script shop: {error:?}"))
    }

    pub fn open_script_shop_in_session(
        &self,
        state: &mut GameState,
        session: &OverworldSession,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<ScriptShopOutcome> {
        self.open_script_shop(
            state,
            &session.map.name,
            map_name,
            source_script,
            command_index,
        )
    }

    fn select_vertical_menu_option(
        &self,
        state: &mut GameState,
        command: RuntimeVerticalMenuSelectionCommand,
    ) -> Result<RuntimeVerticalMenuSelection> {
        let active_menu = state
            .script_runtime
            .active_menu
            .as_deref()
            .context("cannot select vertical menu option because no menu is active")?;
        if active_menu != command.menu_id {
            anyhow::bail!(
                "cannot select vertical menu option for '{}' because active menu is '{}'",
                command.menu_id,
                active_menu
            );
        }
        let matches: Vec<_> = self
            .maps
            .values()
            .flat_map(|module| module.script_vertical_menus.values())
            .filter(|menu| {
                menu.header_label == command.menu_id.as_str()
                    && menu.source_script == command.source_script.as_str()
                    && menu.verticalmenu_command_index == command.verticalmenu_command_index
            })
            .collect();
        if matches.is_empty() {
            anyhow::bail!(
                "compiled pack does not declare vertical menu '{}' from {} command {}",
                command.menu_id,
                command.source_script,
                command.verticalmenu_command_index
            );
        }
        if matches.len() > 1 {
            anyhow::bail!(
                "compiled pack declares duplicate vertical menu '{}' from {} command {}",
                command.menu_id,
                command.source_script,
                command.verticalmenu_command_index
            );
        }
        let menu = matches[0];
        let Some(expected_option) = menu.options.get(command.option_index) else {
            anyhow::bail!(
                "vertical menu '{}' option index {} is out of range for {} options",
                command.menu_id,
                command.option_index,
                menu.options.len()
            );
        };
        if expected_option != &command.option {
            anyhow::bail!(
                "vertical menu '{}' option index {} is '{}', not '{}'",
                command.menu_id,
                command.option_index,
                expected_option,
                command.option
            );
        }
        let script_value = (command.option_index + 1).to_string();
        state.script_runtime.script_value = Some(script_value.clone());
        state
            .script_runtime
            .memory
            .insert("wScriptVar".to_string(), script_value.clone());
        Ok(RuntimeVerticalMenuSelection {
            menu_id: command.menu_id,
            source_script: command.source_script,
            verticalmenu_command_index: command.verticalmenu_command_index,
            option_index: command.option_index,
            option: command.option,
            script_value,
        })
    }

    fn open_vertical_menu(
        &self,
        state: &mut GameState,
        command: RuntimeVerticalMenuOpenCommand,
    ) -> Result<RuntimeVerticalMenuOpen> {
        let module = self
            .maps
            .get(&command.map_name)
            .with_context(|| format!("compiled pack does not declare map {}", command.map_name))?;
        let menu = module
            .script_vertical_menus
            .get(&command.menu_key)
            .with_context(|| {
                format!(
                    "compiled pack does not declare vertical menu {} on {}",
                    command.menu_key, command.map_name
                )
            })?;
        if menu.source_script != command.source_script {
            anyhow::bail!(
                "vertical menu {} on {} belongs to source script {}, not {}",
                command.menu_key,
                command.map_name,
                menu.source_script,
                command.source_script
            );
        }
        if menu.loadmenu_command_index != command.loadmenu_command_index {
            anyhow::bail!(
                "vertical menu {} on {} has loadmenu command {}, not {}",
                command.menu_key,
                command.map_name,
                menu.loadmenu_command_index,
                command.loadmenu_command_index
            );
        }
        if menu.verticalmenu_command_index != command.verticalmenu_command_index {
            anyhow::bail!(
                "vertical menu {} on {} has verticalmenu command {}, not {}",
                command.menu_key,
                command.map_name,
                menu.verticalmenu_command_index,
                command.verticalmenu_command_index
            );
        }
        state.script_runtime.active_menu = Some(menu.header_label.clone());
        state.script_runtime.window_open = true;
        Ok(RuntimeVerticalMenuOpen {
            map_name: command.map_name,
            menu_key: command.menu_key,
            menu_id: menu.header_label.clone(),
            source_script: command.source_script,
            loadmenu_command_index: command.loadmenu_command_index,
            verticalmenu_command_index: command.verticalmenu_command_index,
            options: menu.options.clone(),
        })
    }

    fn select_elevator_floor(
        &self,
        state: &mut GameState,
        command: RuntimeElevatorFloorSelectionCommand,
    ) -> Result<RuntimeElevatorFloorSelection> {
        let module = self
            .maps
            .get(&command.map_name)
            .with_context(|| format!("compiled pack does not declare map {}", command.map_name))?;
        let elevator_key = format!(
            "{}:{}",
            command.source_script, command.elevator_command_index
        );
        let elevator = module
            .script_elevators
            .get(&elevator_key)
            .with_context(|| {
                format!(
                    "compiled pack does not declare elevator '{}' from {} command {} on {}",
                    command.data_label,
                    command.source_script,
                    command.elevator_command_index,
                    command.map_name
                )
            })?;
        if elevator.source_script != command.source_script {
            anyhow::bail!(
                "elevator '{}' on {} belongs to source script {}, not {}",
                command.data_label,
                command.map_name,
                elevator.source_script,
                command.source_script
            );
        }
        if elevator.elevator_command_index != command.elevator_command_index {
            anyhow::bail!(
                "elevator '{}' on {} has command {}, not {}",
                command.data_label,
                command.map_name,
                elevator.elevator_command_index,
                command.elevator_command_index
            );
        }
        if elevator.data_label != command.data_label {
            anyhow::bail!(
                "elevator from {} command {} on {} uses data label {}, not {}",
                command.source_script,
                command.elevator_command_index,
                command.map_name,
                elevator.data_label,
                command.data_label
            );
        }
        let floor = elevator.floors.get(command.floor_index).with_context(|| {
            format!(
                "elevator '{}' floor index {} is out of range for {} floors",
                command.data_label,
                command.floor_index,
                elevator.floors.len()
            )
        })?;
        if floor.floor != command.floor.as_str()
            || floor.warp != command.warp
            || floor.target_map != command.target_map.as_str()
        {
            anyhow::bail!(
                "elevator '{}' floor index {} is ({}, {}, {}), not ({}, {}, {})",
                command.data_label,
                command.floor_index,
                floor.floor,
                floor.warp,
                floor.target_map,
                command.floor,
                command.warp,
                command.target_map
            );
        }
        let selected_floor = floor.floor.clone();
        let selected_warp = floor.warp;
        let selected_target_map = floor.target_map.clone();
        let destination_warp = self
            .maps
            .get(&selected_target_map)
            .with_context(|| {
                format!(
                    "elevator '{}' target map '{}' is missing from compiled pack",
                    command.data_label, selected_target_map
                )
            })?
            .events
            .warps
            .iter()
            .find(|warp| warp.index == selected_warp)
            .with_context(|| {
                format!(
                    "elevator '{}' target map '{}' does not declare warp {}",
                    command.data_label, selected_target_map, selected_warp
                )
            })?;
        let destination_tile =
            checked_runtime_map_event_tile(destination_warp.x, destination_warp.y).with_context(
                || {
                    format!(
                        "elevator '{}' target warp {} coordinate ({}, {}) overflows runtime tile coordinates",
                        command.data_label,
                        destination_warp.index,
                        destination_warp.x,
                        destination_warp.y
                    )
                },
            )?;
        let script_value = "1".to_string();
        state.script_runtime.script_value = Some(script_value.clone());
        state
            .script_runtime
            .memory
            .insert("wScriptVar".to_string(), script_value.clone());
        state.script_runtime.pending_script_warp = Some(ScriptWarpRequest {
            target_map: selected_target_map.clone(),
            tile: destination_tile,
            facing: None,
            source_script: elevator.source_script.clone(),
            command_index: elevator.elevator_command_index,
        });
        Ok(RuntimeElevatorFloorSelection {
            map_name: command.map_name,
            data_label: command.data_label,
            source_script: command.source_script,
            elevator_command_index: command.elevator_command_index,
            floor_index: command.floor_index,
            floor: selected_floor,
            warp: selected_warp,
            target_map: selected_target_map,
            destination_tile,
            script_value,
        })
    }

    pub fn apply_runtime_mutation_command(
        &self,
        state: &mut GameState,
        session: &mut OverworldSession,
        command: RuntimeMutationCommand,
        music_ids: &BTreeSet<String>,
        sound_effect_ids: &BTreeSet<String>,
        cry_ids: &BTreeSet<String>,
    ) -> Result<RuntimeMutationOutcome> {
        self.apply_runtime_mutation_command_with_checksum(
            state,
            session,
            command,
            music_ids,
            sound_effect_ids,
            cry_ids,
            true,
        )
    }

    /// Real-time overworld input path.  The Bevy presentation shell does not
    /// consume a cryptographic/deterministic checksum on every frame; it
    /// already tracks render revisions and only computes checksums at explicit
    /// save/replay boundaries.
    pub fn apply_overworld_input_fast(
        &self,
        state: &mut GameState,
        session: &mut OverworldSession,
        buttons: Vec<GameButton>,
        music_ids: &BTreeSet<String>,
        sound_effect_ids: &BTreeSet<String>,
        cry_ids: &BTreeSet<String>,
    ) -> Result<RuntimeMutationOutcome> {
        self.apply_runtime_mutation_command_with_checksum(
            state,
            session,
            RuntimeMutationCommand::ApplyOverworldInput(RuntimeOverworldInputCommand { buttons }),
            music_ids,
            sound_effect_ids,
            cry_ids,
            false,
        )
    }

    fn apply_runtime_mutation_command_with_checksum(
        &self,
        state: &mut GameState,
        session: &mut OverworldSession,
        command: RuntimeMutationCommand,
        music_ids: &BTreeSet<String>,
        sound_effect_ids: &BTreeSet<String>,
        cry_ids: &BTreeSet<String>,
        compute_checksum: bool,
    ) -> Result<RuntimeMutationOutcome> {
        session.set_time_of_day(state.time.time_of_day);
        let result = match command {
            RuntimeMutationCommand::ApplyOverworldInput(command) => {
                RuntimeMutationResult::OverworldInputApplied(self.apply_overworld_input(
                    state,
                    session,
                    command.buttons,
                    music_ids,
                )?)
            }
            RuntimeMutationCommand::GrantScriptItem(command) => {
                RuntimeMutationResult::ScriptItemGranted(self.grant_script_item_in_session(
                    state,
                    session,
                    &command.map_name,
                    &command.source_script,
                    command.command_index,
                )?)
            }
            RuntimeMutationCommand::CheckScriptItem(command) => {
                RuntimeMutationResult::ScriptItemChecked(self.check_script_item_in_session(
                    state,
                    session,
                    &command.map_name,
                    &command.source_script,
                    command.command_index,
                )?)
            }
            RuntimeMutationCommand::TakeScriptItem(command) => {
                RuntimeMutationResult::ScriptItemTaken(self.take_script_item_in_session(
                    state,
                    session,
                    &command.map_name,
                    &command.source_script,
                    command.command_index,
                )?)
            }
            RuntimeMutationCommand::PickupScriptFieldItem(command) => {
                RuntimeMutationResult::ScriptFieldItemPickedUp(
                    self.pickup_script_field_item_in_session(
                        state,
                        session,
                        &command.map_name,
                        &command.source_script,
                        command.command_index,
                    )?,
                )
            }
            RuntimeMutationCommand::ApplyScriptEconomy(command) => {
                RuntimeMutationResult::ScriptEconomyApplied(
                    self.apply_script_economy_command_in_session(
                        state,
                        session,
                        &command.map_name,
                        &command.source_script,
                        command.command_index,
                    )?,
                )
            }
            RuntimeMutationCommand::ApplyScriptPhone { command, inputs } => {
                RuntimeMutationResult::ScriptPhoneApplied(
                    self.apply_script_phone_command_in_session(
                        state,
                        session,
                        &command.map_name,
                        &command.source_script,
                        command.command_index,
                        inputs,
                    )?,
                )
            }
            RuntimeMutationCommand::ApplyScriptFlagMutation(command) => {
                RuntimeMutationResult::ScriptFlagMutated(
                    self.apply_script_flag_mutation_in_session(
                        state,
                        session,
                        &command.map_name,
                        &command.source_script,
                        command.command_index,
                    )?,
                )
            }
            RuntimeMutationCommand::CheckScriptFlag(command) => {
                RuntimeMutationResult::ScriptFlagChecked(self.check_script_flag_in_session(
                    state,
                    session,
                    &command.map_name,
                    &command.source_script,
                    command.command_index,
                )?)
            }
            RuntimeMutationCommand::ApplyScriptScene(command) => {
                RuntimeMutationResult::ScriptSceneApplied(
                    self.apply_script_scene_command_in_session(
                        state,
                        session,
                        &command.map_name,
                        &command.source_script,
                        command.command_index,
                    )?,
                )
            }
            RuntimeMutationCommand::ApplyScriptBlockChange(command) => {
                RuntimeMutationResult::ScriptBlockChanged(
                    self.apply_script_block_change_in_session(
                        state,
                        session,
                        &command.map_name,
                        &command.source_script,
                        command.command_index,
                    )?,
                )
            }
            RuntimeMutationCommand::ApplyScriptAudio(command) => {
                RuntimeMutationResult::ScriptAudioApplied(
                    self.apply_script_audio_command_in_session(
                        state,
                        session,
                        &command.map_name,
                        &command.source_script,
                        command.command_index,
                        music_ids,
                        sound_effect_ids,
                        cry_ids,
                    )?,
                )
            }
            RuntimeMutationCommand::ApplyScriptMap(command) => {
                RuntimeMutationResult::ScriptMapApplied(self.apply_script_map_command_in_session(
                    state,
                    session,
                    &command.map_name,
                    &command.source_script,
                    command.command_index,
                )?)
            }
            RuntimeMutationCommand::TransitionPendingScriptWarp => {
                RuntimeMutationResult::PendingScriptWarpTransitioned(
                    self.transition_pending_script_warp(state, session, music_ids)?,
                )
            }
            RuntimeMutationCommand::ApplyScriptText(command) => {
                RuntimeMutationResult::ScriptTextApplied(
                    self.apply_script_text_command_in_session(
                        state,
                        session,
                        &command.map_name,
                        &command.source_script,
                        command.command_index,
                    )?,
                )
            }
            RuntimeMutationCommand::ApplyScriptVariableNow(command) => {
                RuntimeMutationResult::ScriptVariableApplied(
                    self.apply_script_variable_command_now_in_session(
                        state,
                        session,
                        &command.map_name,
                        &command.source_script,
                        command.command_index,
                    )?,
                )
            }
            RuntimeMutationCommand::ApplyScriptControl(command) => {
                RuntimeMutationResult::ScriptControlApplied(
                    self.apply_script_control_command_in_session(
                        state,
                        session,
                        &command.map_name,
                        &command.source_script,
                        command.command_index,
                    )?,
                )
            }
            RuntimeMutationCommand::ApplyScriptObjectMutation(command) => {
                RuntimeMutationResult::ScriptObjectMutated(
                    self.apply_script_object_mutation_in_session(
                        state,
                        session,
                        &command.map_name,
                        &command.source_script,
                        command.command_index,
                    )?,
                )
            }
            RuntimeMutationCommand::ApplyScriptMovement(command) => {
                RuntimeMutationResult::ScriptMovementApplied(
                    self.apply_script_movement_in_session(
                        state,
                        session,
                        &command.map_name,
                        &command.source_script,
                        command.command_index,
                    )?,
                )
            }
            RuntimeMutationCommand::ApplyScriptRuntime { command, inputs } => {
                let (command, outcome) = self.apply_script_runtime_command_in_session(
                    state,
                    session,
                    &command.map_name,
                    &command.source_script,
                    command.command_index,
                    inputs,
                )?;
                RuntimeMutationResult::ScriptRuntimeApplied(command, outcome)
            }
            RuntimeMutationCommand::ApplyStandardScript {
                origin_map_name,
                script,
            } => {
                if !self.maps.contains_key(&origin_map_name) {
                    anyhow::bail!(
                        "standard script {script} origin map {origin_map_name} is missing from the pack"
                    );
                }
                let result = match script.as_str() {
                    // These stdscripts are thin wrappers around the same
                    // typed specials exposed by the runtime menu bridge.
                    // Execute the special here so a player reading a map sign
                    // or using the radio actually gets a usable menu surface.
                    "TownMapScript" => {
                        state.script_runtime.text_window_open = true;
                        state.script_runtime.text_events.push(ScriptTextRuntimeEvent {
                            command: "farwritetext".to_string(),
                            kind: ScriptTextRuntimeKind::Write,
                            text_label: Some("LookTownMapText".to_string()),
                            face_player: false,
                            closes_text: false,
                            source_script: script.clone(),
                            command_index: 0,
                        });
                        state.script_runtime.pending_text_wait = Some(ScriptTextWait {
                            command: "town_map_intro".to_string(),
                            source_script: script.clone(),
                            command_index: 0,
                        });
                        script.clone()
                    }
                    "PCScript" => {
                        self.apply_special_routine(state, "PokemonCenterPC", music_ids)?;
                        script.clone()
                    }
                    "Radio1Script" | "Radio2Script" => {
                        let station = if script == "Radio1Script" {
                            "MAPRADIO_POKEMON_CHANNEL"
                        } else {
                            "MAPRADIO_LUCKY_CHANNEL"
                        };
                        state
                            .script_runtime
                            .variables
                            .insert("_value".to_string(), station.to_string());
                        self.apply_special_routine(state, "MapRadio", music_ids)?;
                        script.clone()
                    }
                    "BugContestResultsWarpScript" => {
                        for index in 1..=10 {
                            let flag_a = format!("EVENT_BUG_CATCHING_CONTESTANT_{index}A");
                            let flag_b = format!("EVENT_BUG_CATCHING_CONTESTANT_{index}B");
                            if !state.flags.is_event_flag_set(&flag_a).map_err(|error| {
                                anyhow::anyhow!("read Bug Contest contestant flag: {error}")
                            })? {
                                state.flags.set_event_flag(&flag_b, false).map_err(|error| {
                                    anyhow::anyhow!("clear Bug Contest contestant flag: {error}")
                                })?;
                            }
                        }
                        state
                            .flags
                            .set_event_flag(
                                "EVENT_ROUTE_36_NATIONAL_PARK_GATE_OFFICER_CONTEST_DAY",
                                true,
                            )
                            .map_err(|error| anyhow::anyhow!("set contest-day flag: {error}"))?;
                        state
                            .flags
                            .set_event_flag(
                                "EVENT_ROUTE_36_NATIONAL_PARK_GATE_OFFICER_NOT_CONTEST_DAY",
                                false,
                            )
                            .map_err(|error| {
                                anyhow::anyhow!("clear non-contest-day flag: {error}")
                            })?;
                        state
                            .flags
                            .set_event_flag("EVENT_WARPED_FROM_ROUTE_35_NATIONAL_PARK_GATE", true)
                            .map_err(|error| anyhow::anyhow!("set contest warp flag: {error}"))?;
                        let target_map = "Route36NationalParkGate".to_string();
                        let tile = TilePosition::new(0, 4);
                        state.script_runtime.pending_script_warp = Some(ScriptWarpRequest {
                            target_map: target_map.clone(),
                            tile,
                            facing: None,
                            source_script: script.clone(),
                            command_index: 0,
                        });
                        state.script_runtime.map_events.push(ScriptMapRuntimeEvent {
                            command: "warp".to_string(),
                            kind: ScriptMapRuntimeKind::Warp,
                            target_map: Some(target_map),
                            tile: Some(tile),
                            facing: None,
                            map_setup: None,
                            source_script: script.clone(),
                            command_index: 0,
                        });
                        script.clone()
                    }
                    "BugContestResultsScript" => {
                        state.bug_contest.timer_active = false;
                        state
                            .flags
                            .set_engine_flag("ENGINE_BUG_CONTEST_TIMER", false)
                            .map_err(|error| anyhow::anyhow!("clear contest timer flag: {error}"))?;
                        for flag in [
                            "EVENT_WARPED_FROM_ROUTE_35_NATIONAL_PARK_GATE",
                            "EVENT_CONTEST_OFFICER_HAS_SUN_STONE",
                            "EVENT_CONTEST_OFFICER_HAS_EVERSTONE",
                            "EVENT_CONTEST_OFFICER_HAS_GOLD_BERRY",
                            "EVENT_CONTEST_OFFICER_HAS_BERRY",
                        ] {
                            state.flags.set_event_flag(flag, false).map_err(|error| {
                                anyhow::anyhow!("clear contest flag {flag}: {error}")
                            })?;
                        }
                        let rank = state
                            .script_runtime
                            .variables
                            .remove("_bug_contest_rank")
                            .or_else(|| {
                                state
                                    .script_runtime
                                    .named_buffers
                                    .get("STRING_BUFFER_3")
                                    .cloned()
                            })
                            .and_then(|value| value.parse::<u8>().ok())
                            .filter(|rank| *rank > 0)
                            .or(state.bug_contest.last_rank)
                            .context("BugContestResultsScript requires a completed contest rank")?;
                        state.bug_contest.last_rank = Some(rank);
                        let reward = match rank {
                            1 => "SUN_STONE",
                            2 => "EVERSTONE",
                            3 => "GOLD_BERRY",
                            _ => "BERRY",
                        };
                        if rank == 1 {
                            state
                                .flags
                                .set_event_flag("EVENT_TEMPORARY_UNTIL_MAP_RELOAD_1", true)
                                .map_err(|error| {
                                    anyhow::anyhow!("set first-place contest flag: {error}")
                                })?;
                        }
                        let item = self
                            .items
                            .get(reward)
                            .with_context(|| format!("contest reward {reward} is missing"))?;
                        state.bag.add_item(item, 1).map_err(|error| {
                            anyhow::anyhow!("award contest prize {reward}: {error}")
                        })?;
                        let return_party = state
                            .flags
                            .is_event_flag_set("EVENT_LEFT_MONS_WITH_CONTEST_OFFICER")
                            .map_err(|error| {
                                anyhow::anyhow!("read contest party-return flag: {error}")
                            })?;
                        if return_party {
                            self.apply_special_routine(state, "ContestReturnMons", music_ids)?;
                            state
                                .flags
                                .set_event_flag("EVENT_LEFT_MONS_WITH_CONTEST_OFFICER", false)
                                .map_err(|error| {
                                    anyhow::anyhow!("clear contest party-return flag: {error}")
                                })?;
                        }
                        for index in 1..=10 {
                            for suffix in ['A', 'B'] {
                                state
                                    .flags
                                    .set_event_flag(
                                        &format!("EVENT_BUG_CATCHING_CONTESTANT_{index}{suffix}"),
                                        true,
                                    )
                                    .map_err(|error| {
                                        anyhow::anyhow!("set contest contestant flag: {error}")
                                    })?;
                            }
                        }
                        state
                            .flags
                            .set_engine_flag("ENGINE_DAILY_BUG_CONTEST", true)
                            .map_err(|error| anyhow::anyhow!("set daily contest flag: {error}"))?;
                        state
                            .script_runtime
                            .named_buffers
                            .insert("_bug_contest_rank".to_string(), rank.to_string());
                        let mut text_labels = vec!["ContestResults_ReadyToJudgeText"];
                        if rank <= 3 {
                            text_labels.push("ContestResults_PlayerWonAPrizeText");
                        } else {
                            text_labels.push("ContestResults_ConsolationPrizeText");
                            text_labels.push("ContestResults_DidNotWinText");
                        }
                        text_labels.push("ContestResults_JoinUsNextTimeText");
                        if return_party {
                            text_labels.push("ContestResults_ReturnPartyText");
                        }
                        state.script_runtime.text_window_open = true;
                        for (command_index, text_label) in text_labels.iter().enumerate() {
                            state
                                .script_runtime
                                .text_events
                                .push(ScriptTextRuntimeEvent {
                                    command: "farwritetext".to_string(),
                                    kind: ScriptTextRuntimeKind::Write,
                                    text_label: Some((*text_label).to_string()),
                                    face_player: false,
                                    closes_text: command_index + 1 == text_labels.len(),
                                    source_script: script.to_string(),
                                    command_index,
                                });
                        }
                        state.script_runtime.pending_text_wait = Some(ScriptTextWait {
                            command: "farwritetext".to_string(),
                            source_script: script.to_string(),
                            command_index: text_labels.len() - 1,
                        });
                        script.clone()
                    }
                    "GymStatue1Script" | "GymStatue2Script" => {
                        let landmark = self.pokegear_landmark_for_map(&origin_map_name)?;
                        state.script_runtime.named_buffers.insert(
                            "STRING_BUFFER_3".to_string(),
                            landmark.name.clone(),
                        );
                        let compiled_body = self.compiled_standard_script_body(&script)?.to_vec();
                        apply_standard_script(state, &self.moves, &script, &compiled_body)?
                    }
                    "RadioTowerRocketsScript" => {
                        let compiled_body = self.compiled_standard_script_body(&script)?.to_vec();
                        let result =
                            apply_standard_script(state, &self.moves, &script, &compiled_body)?;
                        let target_map = self.map_name_for_constant("MAHOGANY_TOWN")?;
                        let scenes = self.map_module(&target_map)?.scenes.clone();
                        state
                            .scenes
                            .set_map_scene(&target_map, "SCENE_MAHOGANYTOWN_NOOP", &scenes)
                            .map_err(|error| anyhow::anyhow!("set Mahogany Town scene: {error:?}"))?;
                        result
                    }
                    _ => {
                        let compiled_body = self.compiled_standard_script_body(&script)?.to_vec();
                        apply_standard_script(state, &self.moves, &script, &compiled_body)?
                    }
                };
                RuntimeMutationResult::StandardScriptApplied(result)
            }
            RuntimeMutationCommand::TakeNextScript => {
                let Some(script) = state.script_runtime.next_script.take() else {
                    anyhow::bail!("cannot take next script because no next script is queued");
                };
                RuntimeMutationResult::NextScriptTaken(script)
            }
            RuntimeMutationCommand::DrainScriptEventQueue(command) => {
                let drained = match command.queue {
                    RuntimeScriptEventQueue::Audio => RuntimeScriptEventDrainResult::Audio(
                        std::mem::take(&mut state.script_runtime.audio_events),
                    ),
                    RuntimeScriptEventQueue::Graphics => RuntimeScriptEventDrainResult::Graphics(
                        std::mem::take(&mut state.script_runtime.graphics_events),
                    ),
                    RuntimeScriptEventQueue::Money => RuntimeScriptEventDrainResult::Money(
                        std::mem::take(&mut state.script_runtime.money_events),
                    ),
                    RuntimeScriptEventQueue::Map => RuntimeScriptEventDrainResult::Map(
                        std::mem::take(&mut state.script_runtime.map_events),
                    ),
                    RuntimeScriptEventQueue::Text => RuntimeScriptEventDrainResult::Text(
                        std::mem::take(&mut state.script_runtime.text_events),
                    ),
                    RuntimeScriptEventQueue::Control => RuntimeScriptEventDrainResult::Control(
                        std::mem::take(&mut state.script_runtime.control_events),
                    ),
                    RuntimeScriptEventQueue::Shop => RuntimeScriptEventDrainResult::Shop(
                        std::mem::take(&mut state.script_runtime.shop_events),
                    ),
                    RuntimeScriptEventQueue::ItemUse => RuntimeScriptEventDrainResult::ItemUse(
                        std::mem::take(&mut state.script_runtime.item_use_events),
                    ),
                };
                RuntimeMutationResult::ScriptEventQueueDrained(drained)
            }
            RuntimeMutationCommand::DrainScriptRuntimeQueue(command) => {
                let drained = match command.queue {
                    RuntimeScriptRuntimeQueue::PendingDelay => {
                        RuntimeScriptRuntimeQueueDrainResult::PendingDelay(std::mem::take(
                            &mut state.script_runtime.pending_delays,
                        ))
                    }
                    RuntimeScriptRuntimeQueue::PendingEarthquake => {
                        RuntimeScriptRuntimeQueueDrainResult::PendingEarthquake(std::mem::take(
                            &mut state.script_runtime.pending_earthquakes,
                        ))
                    }
                    RuntimeScriptRuntimeQueue::PendingEmote => {
                        RuntimeScriptRuntimeQueueDrainResult::PendingEmote(std::mem::take(
                            &mut state.script_runtime.pending_emotes,
                        ))
                    }
                    RuntimeScriptRuntimeQueue::Command => {
                        RuntimeScriptRuntimeQueueDrainResult::Command(std::mem::take(
                            &mut state.script_runtime.command_queue,
                        ))
                    }
                    RuntimeScriptRuntimeQueue::Stack => {
                        RuntimeScriptRuntimeQueueDrainResult::Stack(std::mem::take(
                            &mut state.script_runtime.stack,
                        ))
                    }
                    RuntimeScriptRuntimeQueue::CallStack => {
                        RuntimeScriptRuntimeQueueDrainResult::CallStack(std::mem::take(
                            &mut state.script_runtime.call_stack,
                        ))
                    }
                    RuntimeScriptRuntimeQueue::DeferredScript => {
                        RuntimeScriptRuntimeQueueDrainResult::DeferredScript(std::mem::take(
                            &mut state.script_runtime.deferred_scripts,
                        ))
                    }
                };
                RuntimeMutationResult::ScriptRuntimeQueueDrained(drained)
            }
            RuntimeMutationCommand::PopScriptCallStack => {
                let frame = state
                    .script_runtime
                    .call_stack
                    .pop()
                    .context("cannot pop script call stack because it is empty")?;
                RuntimeMutationResult::ScriptCallStackPopped(frame)
            }
            RuntimeMutationCommand::PopDeferredScript => {
                if state.script_runtime.deferred_scripts.is_empty() {
                    anyhow::bail!("cannot pop deferred script because queue is empty");
                }
                RuntimeMutationResult::DeferredScriptPopped(
                    state.script_runtime.deferred_scripts.remove(0),
                )
            }
            RuntimeMutationCommand::TakeScriptEndState => {
                let end = state
                    .script_runtime
                    .script_ended
                    .take()
                    .context("cannot take script end state because none is set")?;
                RuntimeMutationResult::ScriptEndStateTaken(end)
            }
            RuntimeMutationCommand::DrainScriptRuntimeRecordQueue(command) => {
                let drained = match command.queue {
                    RuntimeScriptRuntimeRecordQueue::VariableWrite => {
                        RuntimeScriptRuntimeRecordQueueDrainResult::VariableWrite(std::mem::take(
                            &mut state.script_runtime.variable_writes,
                        ))
                    }
                    RuntimeScriptRuntimeRecordQueue::Effect => {
                        RuntimeScriptRuntimeRecordQueueDrainResult::Effect(std::mem::take(
                            &mut state.script_runtime.effects,
                        ))
                    }
                    RuntimeScriptRuntimeRecordQueue::AsmDirective => {
                        RuntimeScriptRuntimeRecordQueueDrainResult::AsmDirective(std::mem::take(
                            &mut state.script_runtime.asm_directives,
                        ))
                    }
                    RuntimeScriptRuntimeRecordQueue::NumericBufferWrite => {
                        RuntimeScriptRuntimeRecordQueueDrainResult::NumericBufferWrite(
                            std::mem::take(&mut state.script_runtime.numeric_buffer_writes),
                        )
                    }
                    RuntimeScriptRuntimeRecordQueue::ElevatorFloor => {
                        RuntimeScriptRuntimeRecordQueueDrainResult::ElevatorFloor(std::mem::take(
                            &mut state.script_runtime.elevator_floors,
                        ))
                    }
                    RuntimeScriptRuntimeRecordQueue::StoneTableEntry => {
                        RuntimeScriptRuntimeRecordQueueDrainResult::StoneTableEntry(std::mem::take(
                            &mut state.script_runtime.stone_table_entries,
                        ))
                    }
                    RuntimeScriptRuntimeRecordQueue::DecorationDescription => {
                        RuntimeScriptRuntimeRecordQueueDrainResult::DecorationDescription(
                            std::mem::take(&mut state.script_runtime.decoration_descriptions),
                        )
                    }
                    RuntimeScriptRuntimeRecordQueue::SpecialPhoneCall => {
                        RuntimeScriptRuntimeRecordQueueDrainResult::SpecialPhoneCall(
                            std::mem::take(&mut state.script_runtime.special_phone_calls),
                        )
                    }
                    RuntimeScriptRuntimeRecordQueue::CompletedTrade => {
                        RuntimeScriptRuntimeRecordQueueDrainResult::CompletedTrade(std::mem::take(
                            &mut state.script_runtime.completed_trades,
                        ))
                    }
                    RuntimeScriptRuntimeRecordQueue::CatchTutorial => {
                        RuntimeScriptRuntimeRecordQueueDrainResult::CatchTutorial(std::mem::take(
                            &mut state.script_runtime.catch_tutorials,
                        ))
                    }
                    RuntimeScriptRuntimeRecordQueue::CheckedMailTarget => {
                        RuntimeScriptRuntimeRecordQueueDrainResult::CheckedMailTarget(
                            std::mem::take(&mut state.script_runtime.checked_mail_targets),
                        )
                    }
                    RuntimeScriptRuntimeRecordQueue::GivenMailTarget => {
                        RuntimeScriptRuntimeRecordQueueDrainResult::GivenMailTarget(std::mem::take(
                            &mut state.script_runtime.given_mail_targets,
                        ))
                    }
                };
                RuntimeMutationResult::ScriptRuntimeRecordQueueDrained(drained)
            }
            RuntimeMutationCommand::TakePendingScriptRequest(command) => {
                let request =
                    match command.kind {
                        RuntimePendingScriptRequestKind::MusicFade => {
                            RuntimePendingScriptRequest::MusicFade(
                                state.script_runtime.pending_music_fade.take().context(
                                    "cannot take pending music fade because none is pending",
                                )?,
                            )
                        }
                        RuntimePendingScriptRequestKind::ScreenFade => {
                            RuntimePendingScriptRequest::ScreenFade(
                                state.script_runtime.pending_screen_fade.take().context(
                                    "cannot take pending screen fade because none is pending",
                                )?,
                            )
                        }
                        RuntimePendingScriptRequestKind::ScriptWarp => {
                            RuntimePendingScriptRequest::ScriptWarp(
                                state.script_runtime.pending_script_warp.take().context(
                                    "cannot take pending script warp because none is pending",
                                )?,
                            )
                        }
                        RuntimePendingScriptRequestKind::MapLoad => {
                            RuntimePendingScriptRequest::MapLoad(
                                state.script_runtime.pending_map_load.take().context(
                                    "cannot take pending map load because none is pending",
                                )?,
                            )
                        }
                        RuntimePendingScriptRequestKind::MapRefresh => {
                            RuntimePendingScriptRequest::MapRefresh(
                                state.script_runtime.pending_map_refresh.take().context(
                                    "cannot take pending map refresh because none is pending",
                                )?,
                            )
                        }
                        RuntimePendingScriptRequestKind::TextLabel => {
                            RuntimePendingScriptRequest::TextLabel(
                                state.script_runtime.pending_text_label.take().context(
                                    "cannot take pending text label because none is pending",
                                )?,
                            )
                        }
                        RuntimePendingScriptRequestKind::TextWait => {
                            let wait =
                                state.script_runtime.pending_text_wait.take().context(
                                    "cannot take pending text wait because none is pending",
                                )?;
                            if wait.source_script == "PokecenterNurseScript"
                                && wait.command == "pokecenter_greeting"
                            {
                                state.script_runtime.text_events.push(ScriptTextRuntimeEvent {
                                    command: "farwritetext".to_string(),
                                    kind: ScriptTextRuntimeKind::Write,
                                    text_label: Some("NurseAskHealText".to_string()),
                                    face_player: true,
                                    closes_text: false,
                                    source_script: wait.source_script.clone(),
                                    command_index: wait.command_index.saturating_add(1),
                                });
                                state.script_runtime.pending_yes_no = Some(ScriptYesNoPrompt {
                                    source_script: wait.source_script.clone(),
                                    command_index: wait.command_index.saturating_add(1),
                                });
                            } else if wait.source_script == "PokecenterNurseScript"
                                && wait.command == "pokecenter_take"
                            {
                                let heal_indexes = (0..state.storage.party.pokemon.len())
                                    .filter(|index| {
                                        state.storage.party.pokemon[*index]
                                            .as_ref()
                                            .is_some_and(|pokemon| {
                                                !pokemon.is_egg
                                                    && pokemon.species.id != "EGG"
                                            })
                                    })
                                    .collect::<Vec<_>>();
                                for party_index in heal_indexes {
                                    full_heal_party_slot(state, &self.moves, party_index)?;
                                }
                                state.script_runtime.variables.insert(
                                    "_value".to_string(),
                                    "HEALMACHINE_POKECENTER".to_string(),
                                );
                                for (command_index, (kind, audio_id)) in [
                                    (ScriptAudioRuntimeKind::Music, "MUSIC_NONE"),
                                    (ScriptAudioRuntimeKind::SoundEffect, "SFX_HEAL_BELL"),
                                ]
                                .into_iter()
                                .enumerate()
                                {
                                    state.script_runtime.audio_events.push(
                                        ScriptAudioRuntimeEvent {
                                            command: "pokecenter_heal".to_string(),
                                            kind,
                                            audio_id: Some(audio_id.to_string()),
                                            fade_frames: None,
                                            source_script: wait.source_script.clone(),
                                            command_index,
                                        },
                                    );
                                }
                                self.apply_special_routine(state, "HealMachineAnim", music_ids)?;
                                self.apply_special_routine(state, "RestartMapMusic", music_ids)?;
                                let pokerus = self.apply_special_routine(
                                    state,
                                    "CheckPokerus",
                                    music_ids,
                                )?;
                                state.script_runtime.variables.insert(
                                    "_pokecenter_pokerus".to_string(),
                                    if matches!(
                                        pokerus.effect,
                                        SpecialRoutineEffect::CheckPokerus {
                                            newly_discovered: true,
                                            ..
                                        }
                                    ) {
                                        "1"
                                    } else {
                                        "0"
                                    }
                                    .to_string(),
                                );
                                let pokerus = state
                                    .script_runtime
                                    .variables
                                    .remove("_pokecenter_pokerus")
                                    .is_some_and(|value| value == "1");
                                let (text_label, command, closes_text) = if pokerus {
                                    ("NursePokerusText", "farwritetext", true)
                                } else {
                                    ("NurseReturnPokemonText", "pokecenter_return", false)
                                };
                                state.script_runtime.text_events.push(ScriptTextRuntimeEvent {
                                    command: "farwritetext".to_string(),
                                    kind: ScriptTextRuntimeKind::Write,
                                    text_label: Some(text_label.to_string()),
                                    face_player: true,
                                    closes_text,
                                    source_script: wait.source_script.clone(),
                                    command_index: wait.command_index.saturating_add(1),
                                });
                                state.script_runtime.pending_text_wait = Some(ScriptTextWait {
                                    command: command.to_string(),
                                    source_script: wait.source_script.clone(),
                                    command_index: wait.command_index.saturating_add(1),
                                });
                            } else if wait.source_script == "PokecenterNurseScript"
                                && wait.command == "pokecenter_return"
                            {
                                state.script_runtime.text_events.push(ScriptTextRuntimeEvent {
                                    command: "farwritetext".to_string(),
                                    kind: ScriptTextRuntimeKind::Write,
                                    text_label: Some("NurseGoodbyeText".to_string()),
                                    face_player: true,
                                    closes_text: true,
                                    source_script: wait.source_script.clone(),
                                    command_index: wait.command_index.saturating_add(1),
                                });
                                state.script_runtime.pending_text_wait = Some(ScriptTextWait {
                                    command: "farwritetext".to_string(),
                                    source_script: wait.source_script.clone(),
                                    command_index: wait.command_index.saturating_add(1),
                                });
                            } else if wait.source_script == "TownMapScript"
                                && wait.command == "town_map_intro"
                            {
                                self.apply_special_routine(
                                    state,
                                    "OverworldTownMap",
                                    music_ids,
                                )?;
                            }
                            if script_text_wait_closes_window(&wait.command) {
                                state.script_runtime.pending_text_label = None;
                                state.script_runtime.text_window_open = false;
                            }
                            RuntimePendingScriptRequest::TextWait(wait)
                        }
                        RuntimePendingScriptRequestKind::YesNo => {
                            RuntimePendingScriptRequest::YesNo(
                                state.script_runtime.pending_yes_no.take().context(
                                    "cannot take pending yes/no prompt because none is pending",
                                )?,
                            )
                        }
                        RuntimePendingScriptRequestKind::Shop => RuntimePendingScriptRequest::Shop(
                            state
                                .script_runtime
                                .pending_shop
                                .take()
                                .context("cannot take pending shop because none is pending")?,
                        ),
                    };
                RuntimeMutationResult::PendingScriptRequestTaken(request)
            }
            RuntimeMutationCommand::ResolvePendingYesNo(command) => {
                let prompt = state
                    .script_runtime
                    .pending_yes_no
                    .take()
                    .context("cannot resolve pending yes/no prompt because none is pending")?;
                let script_value = if command.accepted { "1" } else { "0" }.to_string();
                state.script_runtime.script_value = Some(script_value.clone());
                if prompt.source_script == "GameCornerCoinVendorScript" {
                    let text_label = if command.accepted {
                        resolve_coin_vendor_purchase(state)?
                    } else {
                        "CoinVendor_CancelText"
                    };
                    state.script_runtime.text_window_open = true;
                    state
                        .script_runtime
                        .text_events
                        .push(ScriptTextRuntimeEvent {
                            command: "farwritetext".to_string(),
                            kind: ScriptTextRuntimeKind::Write,
                            text_label: Some(text_label.to_string()),
                            face_player: true,
                            closes_text: true,
                            source_script: prompt.source_script.clone(),
                            command_index: prompt.command_index.saturating_add(1),
                        });
                    state.script_runtime.pending_text_wait = Some(ScriptTextWait {
                        command: "farwritetext".to_string(),
                        source_script: prompt.source_script.clone(),
                        command_index: prompt.command_index.saturating_add(1),
                    });
                } else if prompt.source_script == "PokecenterNurseScript" {
                    let text_label = if command.accepted {
                        "NurseTakePokemonText"
                    } else {
                        "NurseGoodbyeText"
                    };
                    state.script_runtime.text_window_open = true;
                    state.script_runtime.text_events.push(ScriptTextRuntimeEvent {
                        command: "farwritetext".to_string(),
                        kind: ScriptTextRuntimeKind::Write,
                        text_label: Some(text_label.to_string()),
                        face_player: true,
                        closes_text: !command.accepted,
                        source_script: prompt.source_script.clone(),
                        command_index: prompt.command_index.saturating_add(1),
                    });
                    state.script_runtime.pending_text_wait = Some(ScriptTextWait {
                        command: if command.accepted {
                            "pokecenter_take"
                        } else {
                            "farwritetext"
                        }
                        .to_string(),
                        source_script: prompt.source_script.clone(),
                        command_index: prompt.command_index.saturating_add(1),
                    });
                } else if prompt.source_script == "SmashRockScript" {
                    let rock_smash_user = command.accepted.then(|| {
                        state
                            .storage
                            .party
                            .pokemon
                            .iter()
                            .enumerate()
                            .find_map(|(party_index, pokemon)| {
                                pokemon.as_ref().and_then(|pokemon| {
                                    pokemon
                                        .moves
                                        .iter()
                                        .any(|learned| learned.name == "ROCK_SMASH")
                                        .then(|| {
                                            let actor_name = if pokemon.nickname.is_empty() {
                                                pokemon.species.id.clone()
                                            } else {
                                                pokemon.nickname.clone()
                                            };
                                            (party_index, actor_name)
                                        })
                                })
                            })
                    });
                    if let Some((party_index, actor_name)) = rock_smash_user.flatten() {
                        let mut next_state = state.clone();
                        let mut next_session = session.clone();
                        match self.use_rock_smash_field_move(
                            &mut next_state,
                            &mut next_session,
                            party_index,
                        ) {
                            Ok(_) => {
                                *state = next_state;
                                *session = next_session;
                            for buffer in 1..=4 {
                                state.script_runtime.named_buffers.insert(
                                    format!("STRING_BUFFER_{buffer}"),
                                    actor_name.clone(),
                                );
                            }
                            state.script_runtime.text_events.push(ScriptTextRuntimeEvent {
                                command: "farwritetext".to_string(),
                                kind: ScriptTextRuntimeKind::Write,
                                text_label: Some("UseRockSmashText".to_string()),
                                face_player: false,
                                closes_text: true,
                                source_script: prompt.source_script.clone(),
                                command_index: prompt.command_index.saturating_add(1),
                            });
                            state.script_runtime.pending_text_wait = Some(ScriptTextWait {
                                command: "farwritetext".to_string(),
                                source_script: prompt.source_script.clone(),
                                command_index: prompt.command_index.saturating_add(1),
                            });
                            }
                            Err(error) if error.downcast_ref::<FieldMoveError>().is_some() => {
                                state.script_runtime.text_events.push(ScriptTextRuntimeEvent {
                                    command: "farwritetext".to_string(),
                                    kind: ScriptTextRuntimeKind::Write,
                                    text_label: Some("CantUseRockSmashText".to_string()),
                                    face_player: false,
                                    closes_text: true,
                                    source_script: prompt.source_script.clone(),
                                    command_index: prompt.command_index.saturating_add(1),
                                });
                                state.script_runtime.pending_text_wait = Some(ScriptTextWait {
                                    command: "farwritetext".to_string(),
                                    source_script: prompt.source_script.clone(),
                                    command_index: prompt.command_index.saturating_add(1),
                                });
                            }
                            Err(error) => return Err(error),
                        }
                    } else {
                        if command.accepted {
                            state.script_runtime.text_events.push(ScriptTextRuntimeEvent {
                                command: "farwritetext".to_string(),
                                kind: ScriptTextRuntimeKind::Write,
                                text_label: Some("CantUseRockSmashText".to_string()),
                                face_player: false,
                                closes_text: true,
                                source_script: prompt.source_script.clone(),
                                command_index: prompt.command_index.saturating_add(1),
                            });
                            state.script_runtime.pending_text_wait = Some(ScriptTextWait {
                                command: "farwritetext".to_string(),
                                source_script: prompt.source_script.clone(),
                                command_index: prompt.command_index.saturating_add(1),
                            });
                        } else {
                            state.script_runtime.text_window_open = false;
                        }
                    }
                } else if prompt.source_script == "StrengthBoulderScript" {
                    let strength_user = command.accepted.then(|| {
                        state
                            .storage
                            .party
                            .pokemon
                            .iter()
                            .enumerate()
                            .find_map(|(party_index, pokemon)| {
                                pokemon.as_ref().and_then(|pokemon| {
                                    pokemon
                                        .moves
                                        .iter()
                                        .any(|learned| learned.name == "STRENGTH")
                                        .then(|| {
                                            let actor_name = if pokemon.nickname.is_empty() {
                                                pokemon.species.id.clone()
                                            } else {
                                                pokemon.nickname.clone()
                                            };
                                            (party_index, actor_name)
                                        })
                                })
                            })
                    });
                    if let Some((party_index, actor_name)) = strength_user.flatten() {
                        let mut next_state = state.clone();
                        match self.use_strength_field_move(&mut next_state, party_index) {
                            Ok(_) => {
                            *state = next_state;
                            for buffer in 1..=4 {
                                state.script_runtime.named_buffers.insert(
                                    format!("STRING_BUFFER_{buffer}"),
                                    actor_name.clone(),
                                );
                            }
                            state
                                .script_runtime
                                .text_events
                                .push(ScriptTextRuntimeEvent {
                                    command: "farwritetext".to_string(),
                                    kind: ScriptTextRuntimeKind::Write,
                                    text_label: Some("UseStrengthText".to_string()),
                                    face_player: false,
                                    closes_text: true,
                                    source_script: prompt.source_script.clone(),
                                    command_index: prompt.command_index.saturating_add(1),
                                });
                            state.script_runtime.pending_text_wait = Some(ScriptTextWait {
                                command: "farwritetext".to_string(),
                                source_script: prompt.source_script.clone(),
                                command_index: prompt.command_index.saturating_add(1),
                            });
                            }
                            Err(error) if error.downcast_ref::<FieldMoveError>().is_some() => {
                                state.script_runtime.text_window_open = false;
                            }
                            Err(error) => return Err(error),
                        }
                    } else {
                        state.script_runtime.text_window_open = false;
                    }
                }
                RuntimeMutationResult::PendingYesNoResolved(RuntimePendingYesNoResolution {
                    prompt,
                    accepted: command.accepted,
                    script_value,
                })
            }
            RuntimeMutationCommand::OpenVerticalMenu(command) => {
                RuntimeMutationResult::VerticalMenuOpened(self.open_vertical_menu(state, command)?)
            }
            RuntimeMutationCommand::SelectVerticalMenuOption(command) => {
                RuntimeMutationResult::VerticalMenuOptionSelected(
                    self.select_vertical_menu_option(state, command)?,
                )
            }
            RuntimeMutationCommand::SelectElevatorFloor(command) => {
                RuntimeMutationResult::ElevatorFloorSelected(
                    self.select_elevator_floor(state, command)?,
                )
            }
            RuntimeMutationCommand::ConsumeScriptRuntimeFlag(command) => {
                let consumed = match command.flag {
                    RuntimeScriptRuntimeFlag::MapMusicRestartDisabled => {
                        if !state.script_runtime.map_music_restart_disabled {
                            anyhow::bail!(
                                "cannot consume map music restart-disabled flag because it is not set"
                            );
                        }
                        state.script_runtime.map_music_restart_disabled = false;
                        RuntimeScriptRuntimeFlagValue::MapMusicRestartDisabled
                    }
                    RuntimeScriptRuntimeFlag::MapMusicRequested => {
                        if !state.script_runtime.map_music_requested {
                            anyhow::bail!(
                                "cannot consume map music requested flag because it is not set"
                            );
                        }
                        let map_music = self.checked_map_music(&session.map.name, music_ids)?;
                        state.script_runtime.map_music_requested = false;
                        apply_map_music_context(state, map_music);
                        RuntimeScriptRuntimeFlagValue::MapMusicRequested
                    }
                    RuntimeScriptRuntimeFlag::WaitingForSoundEffect => {
                        if !state.script_runtime.waiting_for_sound_effect {
                            anyhow::bail!(
                                "cannot consume waiting-for-sound-effect flag because it is not set"
                            );
                        }
                        state.script_runtime.waiting_for_sound_effect = false;
                        RuntimeScriptRuntimeFlagValue::WaitingForSoundEffect
                    }
                    RuntimeScriptRuntimeFlag::WarpCheckRequested => {
                        if !state.script_runtime.warp_check_requested {
                            anyhow::bail!(
                                "cannot consume warp-check requested flag because it is not set"
                            );
                        }
                        state.script_runtime.warp_check_requested = false;
                        RuntimeScriptRuntimeFlagValue::WarpCheckRequested
                    }
                    RuntimeScriptRuntimeFlag::ItemNotifyQueued => {
                        if !state.script_runtime.item_notify_queued {
                            anyhow::bail!(
                                "cannot consume item-notify queued flag because it is not set"
                            );
                        }
                        state.script_runtime.item_notify_queued = false;
                        RuntimeScriptRuntimeFlagValue::ItemNotifyQueued
                    }
                    RuntimeScriptRuntimeFlag::WarpSoundQueued => {
                        if !state.script_runtime.warp_sound_queued {
                            anyhow::bail!(
                                "cannot consume warp-sound queued flag because it is not set"
                            );
                        }
                        state.script_runtime.warp_sound_queued = false;
                        RuntimeScriptRuntimeFlagValue::WarpSoundQueued
                    }
                    RuntimeScriptRuntimeFlag::TeleportFromQueued => {
                        if !state.script_runtime.teleport_from_queued {
                            anyhow::bail!(
                                "cannot consume teleport-from queued flag because it is not set"
                            );
                        }
                        state.script_runtime.teleport_from_queued = false;
                        RuntimeScriptRuntimeFlagValue::TeleportFromQueued
                    }
                    RuntimeScriptRuntimeFlag::HallOfFameRequested => {
                        if !state.script_runtime.hall_of_fame_requested {
                            anyhow::bail!(
                                "cannot consume Hall of Fame requested flag because it is not set"
                            );
                        }
                        state.script_runtime.hall_of_fame_requested = false;
                        RuntimeScriptRuntimeFlagValue::HallOfFameRequested
                    }
                    RuntimeScriptRuntimeFlag::CreditsRequested => {
                        if !state.script_runtime.credits_requested {
                            anyhow::bail!(
                                "cannot consume credits requested flag because it is not set"
                            );
                        }
                        state.script_runtime.credits_requested = false;
                        RuntimeScriptRuntimeFlagValue::CreditsRequested
                    }
                    RuntimeScriptRuntimeFlag::ResetRequested => {
                        if !state.script_runtime.reset_requested {
                            anyhow::bail!(
                                "cannot consume reset requested flag because it is not set"
                            );
                        }
                        state.script_runtime.reset_requested = false;
                        RuntimeScriptRuntimeFlagValue::ResetRequested
                    }
                    RuntimeScriptRuntimeFlag::Menu2dRequested => {
                        if !state.script_runtime.menu_2d_requested {
                            anyhow::bail!(
                                "cannot consume 2D-menu requested flag because it is not set"
                            );
                        }
                        state.script_runtime.menu_2d_requested = false;
                        RuntimeScriptRuntimeFlagValue::Menu2dRequested
                    }
                    RuntimeScriptRuntimeFlag::VersionCheckRequested => {
                        if !state.script_runtime.version_check_requested {
                            anyhow::bail!(
                                "cannot consume version-check requested flag because it is not set"
                            );
                        }
                        state.script_runtime.version_check_requested = false;
                        RuntimeScriptRuntimeFlagValue::VersionCheckRequested
                    }
                    RuntimeScriptRuntimeFlag::BlackoutMod => {
                        RuntimeScriptRuntimeFlagValue::BlackoutMod(
                            state
                                .script_runtime
                                .blackout_mod
                                .take()
                                .context("cannot consume blackout mod because none is pending")?,
                        )
                    }
                    RuntimeScriptRuntimeFlag::BattleTowerText => {
                        RuntimeScriptRuntimeFlagValue::BattleTowerText(
                            state.script_runtime.battle_tower_text.take().context(
                                "cannot consume Battle Tower text because none is pending",
                            )?,
                        )
                    }
                };
                RuntimeMutationResult::ScriptRuntimeFlagConsumed(consumed)
            }
            RuntimeMutationCommand::TakeScriptRuntimeMemoryValue(command) => {
                let value =
                    match command.value {
                        RuntimeScriptRuntimeMemoryValue::ScriptValue => {
                            RuntimeScriptRuntimeMemoryValueTaken::ScriptValue(
                                state
                                    .script_runtime
                                    .script_value
                                    .take()
                                    .context("cannot take script value because none is set")?,
                            )
                        }
                        RuntimeScriptRuntimeMemoryValue::LastSpecialRoutine => {
                            RuntimeScriptRuntimeMemoryValueTaken::LastSpecialRoutine(
                                state.script_runtime.last_special_routine.take().context(
                                    "cannot take last special routine because none is set",
                                )?,
                            )
                        }
                        RuntimeScriptRuntimeMemoryValue::LastTalkedObject => {
                            RuntimeScriptRuntimeMemoryValueTaken::LastTalkedObject(
                                state.script_runtime.last_talked_object.take().context(
                                    "cannot take last talked object because none is set",
                                )?,
                            )
                        }
                    };
                RuntimeMutationResult::ScriptRuntimeMemoryValueTaken(value)
            }
            RuntimeMutationCommand::RemoveScriptRuntimeMemoryEntry(command) => {
                let removed = match command.entry {
                    RuntimeScriptRuntimeMemoryEntry::Variable => {
                        let value = state
                            .script_runtime
                            .variables
                            .remove(&command.key)
                            .with_context(|| {
                                format!(
                                    "cannot remove script variable {} because it is not set",
                                    command.key
                                )
                            })?;
                        RuntimeScriptRuntimeMemoryEntryRemoved::Variable {
                            key: command.key,
                            value,
                        }
                    }
                    RuntimeScriptRuntimeMemoryEntry::Memory => {
                        let value = state
                            .script_runtime
                            .memory
                            .remove(&command.key)
                            .with_context(|| {
                                format!(
                                    "cannot remove script memory {} because it is not set",
                                    command.key
                                )
                            })?;
                        RuntimeScriptRuntimeMemoryEntryRemoved::Memory {
                            key: command.key,
                            value,
                        }
                    }
                    RuntimeScriptRuntimeMemoryEntry::NamedBuffer => {
                        let value = state
                            .script_runtime
                            .named_buffers
                            .remove(&command.key)
                            .with_context(|| {
                                format!(
                                    "cannot remove named buffer {} because it is not set",
                                    command.key
                                )
                            })?;
                        RuntimeScriptRuntimeMemoryEntryRemoved::NamedBuffer {
                            key: command.key,
                            value,
                        }
                    }
                    RuntimeScriptRuntimeMemoryEntry::VariableSprite => {
                        let value = state
                            .script_runtime
                            .variable_sprites
                            .remove(&command.key)
                            .with_context(|| {
                                format!(
                                    "cannot remove variable sprite {} because it is not set",
                                    command.key
                                )
                            })?;
                        RuntimeScriptRuntimeMemoryEntryRemoved::VariableSprite {
                            key: command.key,
                            value,
                        }
                    }
                    RuntimeScriptRuntimeMemoryEntry::PhoneNumber => {
                        if !state.script_runtime.phone_numbers.remove(&command.key) {
                            anyhow::bail!(
                                "cannot remove phone number {} because it is not set",
                                command.key
                            );
                        }
                        RuntimeScriptRuntimeMemoryEntryRemoved::PhoneNumber { key: command.key }
                    }
                };
                RuntimeMutationResult::ScriptRuntimeMemoryEntryRemoved(removed)
            }
            RuntimeMutationCommand::OpenScriptShop(command) => {
                RuntimeMutationResult::ScriptShopOpened(self.open_script_shop_in_session(
                    state,
                    session,
                    &command.map_name,
                    &command.source_script,
                    command.command_index,
                )?)
            }
            RuntimeMutationCommand::CloseActiveMenu => {
                let Some(menu) = state.script_runtime.active_menu.take() else {
                    anyhow::bail!("cannot close active menu because no runtime menu is active");
                };
                RuntimeMutationResult::ActiveMenuClosed(menu)
            }
            RuntimeMutationCommand::CloseRuntimeWindow => {
                if !state.script_runtime.window_open {
                    anyhow::bail!("cannot close runtime window because no runtime window is open");
                }
                state.script_runtime.window_open = false;
                RuntimeMutationResult::RuntimeWindowClosed
            }
            RuntimeMutationCommand::CloseTextWindow => {
                if !state.script_runtime.text_window_open {
                    anyhow::bail!("cannot close text window because no text window is open");
                }
                state.script_runtime.text_window_open = false;
                state.script_runtime.pending_text_label = None;
                state.script_runtime.pending_text_wait = None;
                state.script_runtime.pending_yes_no = None;
                RuntimeMutationResult::TextWindowClosed
            }
            RuntimeMutationCommand::ClearMenuCoords => {
                let Some(coords) = state.script_runtime.menu_coords.take() else {
                    anyhow::bail!("cannot clear menu coordinates because none are active");
                };
                RuntimeMutationResult::MenuCoordsCleared(coords)
            }
            RuntimeMutationCommand::CloseActivePokemonPicture => {
                let Some(species) = state.script_runtime.active_pokemon_picture.take() else {
                    anyhow::bail!(
                        "cannot close active Pokemon picture because no Pokemon picture is active"
                    );
                };
                RuntimeMutationResult::ActivePokemonPictureClosed(species)
            }
            RuntimeMutationCommand::CloseScriptShop => RuntimeMutationResult::ScriptShopClosed(
                core_close_active_shop(state)
                    .map_err(|error| anyhow::anyhow!("close script shop: {error}"))?,
            ),
            RuntimeMutationCommand::BuyShopItem(command) => RuntimeMutationResult::ShopItemBought(
                self.buy_shop_item(state, &command.item_id, command.quantity)?,
            ),
            RuntimeMutationCommand::SellShopItem(command) => RuntimeMutationResult::ShopItemSold(
                self.sell_shop_item(state, &command.item_id, command.quantity)?,
            ),
            RuntimeMutationCommand::ApplySpecialRoutine {
                routine,
                rng_seed_after,
            } => {
                let mut next_state = state.clone();
                let outcome = self.apply_special_routine(&mut next_state, &routine, music_ids)?;
                if runtime_special_routine_requires_rng_boundary(&routine) {
                    let rng_seed_after = rng_seed_after.with_context(|| {
                        format!("special routine {routine} command requires rng_seed_after")
                    })?;
                    ensure_runtime_command_rng_boundary(
                        &format!("apply special routine {routine}"),
                        next_state.rng_seed,
                        rng_seed_after,
                    )?;
                } else if rng_seed_after.is_some() {
                    anyhow::bail!(
                        "special routine {routine} command must not declare rng_seed_after"
                    );
                }
                *state = next_state;
                RuntimeMutationResult::SpecialRoutineApplied(outcome)
            }
            RuntimeMutationCommand::ResolveBugContestCaughtMon { keep_new } => {
                let mut next_state = state.clone();
                let outcome =
                    resolve_bug_contest_caught_mon(&mut next_state, keep_new).map_err(|error| {
                        anyhow::anyhow!("resolve Bug Contest caught Pokemon: {error}")
                    })?;
                *state = next_state;
                RuntimeMutationResult::SpecialRoutineApplied(outcome)
            }
            RuntimeMutationCommand::RegisterKeyItem(command) => {
                self.validate_saved_item_reference("registered_key_item", &command.item_id)?;
                if !matches!(state.bag.key_items.get(&command.item_id), Some(quantity) if *quantity > 0)
                {
                    anyhow::bail!(
                        "cannot register key item {} because it is not carried",
                        command.item_id
                    );
                }
                let previous_item_id = state.registered_key_item.replace(command.item_id.clone());
                RuntimeMutationResult::KeyItemRegistered(RuntimeRegisteredKeyItemOutcome {
                    previous_item_id,
                    item_id: command.item_id,
                })
            }
            RuntimeMutationCommand::ApplyGraphicsSpecial(special) => {
                RuntimeMutationResult::GraphicsSpecialApplied(self.apply_special_routine(
                    state,
                    special.routine(),
                    music_ids,
                )?)
            }
            RuntimeMutationCommand::ApplyPartyCheckSpecial(command) => {
                let routine = command.special.routine();
                let species_id = if command.special.requires_species() {
                    Some(command.species_id.with_context(|| {
                        format!("{routine} runtime command requires species_id")
                    })?)
                } else if command.species_id.is_some() {
                    anyhow::bail!("{routine} runtime command does not accept species_id");
                } else {
                    None
                };
                let threshold =
                    if command.special.requires_threshold() {
                        Some(command.threshold.with_context(|| {
                            format!("{routine} runtime command requires threshold")
                        })?)
                    } else if command.threshold.is_some() {
                        anyhow::bail!("{routine} runtime command does not accept threshold");
                    } else {
                        None
                    };
                RuntimeMutationResult::PartyCheckSpecialApplied(
                    self.apply_special_routine_transactional(
                        state,
                        routine,
                        music_ids,
                        |next_state| {
                            if let Some(species_id) = species_id {
                                next_state
                                    .script_runtime
                                    .variables
                                    .insert("_value".to_string(), species_id);
                            }
                            if let Some(threshold) = threshold {
                                next_state
                                    .script_runtime
                                    .variables
                                    .insert("_value".to_string(), threshold.to_string());
                            }
                            Ok(())
                        },
                    )?,
                )
            }
            RuntimeMutationCommand::ApplyPhoneRandomSpecial(command) => {
                let routine = command.special.routine();
                let mut next_state = state.clone();
                next_state
                    .script_runtime
                    .variables
                    .insert("VAR_CALLERID".to_string(), command.contact_id);
                let outcome = self.apply_special_routine(&mut next_state, routine, music_ids)?;
                ensure_runtime_command_rng_boundary(
                    &format!("apply phone random special {routine}"),
                    next_state.rng_seed,
                    command.rng_seed_after,
                )?;
                *state = next_state;
                RuntimeMutationResult::PhoneRandomSpecialApplied(outcome)
            }
            RuntimeMutationCommand::CheckItemInPcOrBagSpecial(command) => {
                RuntimeMutationResult::ItemInPcOrBagChecked(
                    self.apply_special_routine_transactional(
                        state,
                        "UnusedFindItemInPCOrBag",
                        music_ids,
                        |next_state| {
                            next_state.script_runtime.script_value = Some(command.item_id.clone());
                            next_state
                                .script_runtime
                                .variables
                                .insert("_value".to_string(), command.item_id);
                            Ok(())
                        },
                    )?,
                )
            }
            RuntimeMutationCommand::CheckAnotherUsablePartyMonSpecial(command) => {
                RuntimeMutationResult::AnotherUsablePartyMonChecked(
                    self.apply_special_routine_transactional(
                        state,
                        "Function11ba38",
                        music_ids,
                        |next_state| {
                            next_state.script_runtime.variables.insert(
                                "_selected_party_index".to_string(),
                                command.party_index.to_string(),
                            );
                            Ok(())
                        },
                    )?,
                )
            }
            RuntimeMutationCommand::ActivateFishingSwarmSpecial(command) => {
                RuntimeMutationResult::FishingSwarmActivated(
                    self.apply_special_routine_transactional(
                        state,
                        "ActivateFishingSwarm",
                        music_ids,
                        |next_state| {
                            next_state.script_runtime.script_value =
                                Some(command.value.to_string());
                            next_state
                                .script_runtime
                                .variables
                                .insert("_value".to_string(), command.value.to_string());
                            Ok(())
                        },
                    )?,
                )
            }
            RuntimeMutationCommand::ApplyStoryGateSpecial(special) => {
                RuntimeMutationResult::StoryGateSpecialApplied(self.apply_special_routine(
                    state,
                    special.routine(),
                    music_ids,
                )?)
            }
            RuntimeMutationCommand::GrantScriptedGiftPokemon(command) => {
                let mut next_state = state.clone();
                let mut gift_rng = Random::new_crystal(next_state.rng_seed);
                let resolved_dvs = Dv::from_non_hp(
                    gift_rng.randrange(16) as u8,
                    gift_rng.randrange(16) as u8,
                    gift_rng.randrange(16) as u8,
                    gift_rng.randrange(16) as u8,
                );
                let resolved_rng_seed_after = gift_rng.seed();
                if command.dvs != resolved_dvs {
                    anyhow::bail!(
                        "grant scripted gift Pokemon DVs {:?} do not match resolved DVs {:?}",
                        command.dvs,
                        resolved_dvs
                    );
                }
                ensure_runtime_command_rng_boundary(
                    "grant scripted gift Pokemon",
                    resolved_rng_seed_after,
                    command.rng_seed_after,
                )?;
                next_state.rng_seed = resolved_rng_seed_after;
                let outcome = self.grant_scripted_gift_pokemon_in_session(
                    &mut next_state,
                    session,
                    &command.command.map_name,
                    &command.command.source_script,
                    command.command.command_index,
                    command.original_trainer_name,
                    command.original_trainer_id,
                    command.dvs,
                    command.nickname_accepted,
                    command.nickname,
                )?;
                *state = next_state;
                RuntimeMutationResult::ScriptedGiftPokemonGranted(outcome)
            }
            RuntimeMutationCommand::AddPartyPokemon(command) => {
                RuntimeMutationResult::PartyPokemonAdded(self.grant_gift_pokemon_to_state(
                    state,
                    GiftPokemonRequest {
                        species_id: command.species_id,
                        level: command.level,
                        held_item_id: command.held_item_id,
                        nickname: command.nickname,
                        original_trainer_name: command.original_trainer_name,
                        original_trainer_id: command.original_trainer_id,
                        source_script: "RuntimeAddPartyPokemon".to_string(),
                        command_index: 0,
                        egg: false,
                        dvs: command.dvs,
                    },
                )?)
            }
            RuntimeMutationCommand::StartScriptedWildBattle(command) => {
                RuntimeMutationResult::ScriptedWildBattleStarted(
                    self.start_scripted_wild_battle_in_session(
                        state,
                        session,
                        &command.map_name,
                        &command.source_script,
                        command.command_index,
                    )?,
                )
            }
            RuntimeMutationCommand::StartScriptedTrainerBattle(command) => {
                RuntimeMutationResult::ScriptedTrainerBattleStarted(
                    self.start_scripted_trainer_battle_in_session(
                        state,
                        session,
                        &command.map_name,
                        &command.source_script,
                        command.command_index,
                    )?,
                )
            }
            RuntimeMutationCommand::CompleteScriptedWildBattle(command) => {
                RuntimeMutationResult::ScriptedWildBattleCompleted(
                    self.complete_scripted_wild_battle_in_session(
                        state,
                        session,
                        &command.map_name,
                        &command.source_script,
                        command.command_index,
                    )?,
                )
            }
            RuntimeMutationCommand::CompleteScriptedTrainerBattle(command) => {
                RuntimeMutationResult::ScriptedTrainerBattleCompleted(
                    self.complete_scripted_trainer_battle_in_session(
                        state,
                        session,
                        &command.command.map_name,
                        &command.command.source_script,
                        command.command.command_index,
                        command.won,
                        command.can_lose,
                    )?,
                )
            }
            RuntimeMutationCommand::ApplyScriptedTrainerBattleCompletionEffects(command) => {
                RuntimeMutationResult::ScriptedTrainerBattleCompletionEffectsApplied(
                    self.apply_scripted_trainer_battle_completion_effects_in_session(
                        state,
                        session,
                        &command.map_name,
                        &command.source_script,
                        command.command_index,
                    )?,
                )
            }
            RuntimeMutationCommand::UseBagItem { item_id, context } => {
                RuntimeMutationResult::BagItemUsed(self.use_bag_item(state, &item_id, context)?)
            }
            RuntimeMutationCommand::ReplacePendingMoveLearn(command) => {
                RuntimeMutationResult::PendingMoveLearnReplaced(
                    self.replace_pending_move_learn(state, command.move_slot)?,
                )
            }
            RuntimeMutationCommand::DeclinePendingMoveLearn => {
                RuntimeMutationResult::PendingMoveLearnDeclined(
                    self.decline_pending_move_learn(state)?,
                )
            }
            RuntimeMutationCommand::UseBagRepelInField(command) => {
                RuntimeMutationResult::FieldRepelUsed(
                    self.use_bag_repel_in_field(state, &command.item_id)?,
                )
            }
            RuntimeMutationCommand::UseBagBicycleInField(command) => {
                RuntimeMutationResult::FieldBicycleUsed(self.use_bag_bicycle_in_field(
                    state,
                    session,
                    &command.item_id,
                )?)
            }
            RuntimeMutationCommand::UseBagItemfinderInField(command) => {
                RuntimeMutationResult::FieldItemfinderUsed(self.use_bag_itemfinder_in_field(
                    state,
                    session,
                    &command.item_id,
                )?)
            }
            RuntimeMutationCommand::UseBagSquirtbottleInField(command) => {
                RuntimeMutationResult::FieldSquirtbottleUsed(self.use_bag_squirtbottle_in_field(
                    state,
                    session,
                    &command.item_id,
                )?)
            }
            RuntimeMutationCommand::UseBagCoinCaseInField(command) => {
                RuntimeMutationResult::FieldCoinCaseUsed(
                    self.use_bag_coin_case_in_field(state, &command.item_id)?,
                )
            }
            RuntimeMutationCommand::UseBagBlueCardInField(command) => {
                RuntimeMutationResult::FieldBlueCardUsed(
                    self.use_bag_blue_card_in_field(state, &command.item_id)?,
                )
            }
            RuntimeMutationCommand::UseBagTownMapInField(command) => {
                RuntimeMutationResult::FieldTownMapUsed(self.use_bag_town_map_in_field(
                    state,
                    session,
                    &command.item_id,
                )?)
            }
            RuntimeMutationCommand::UseBagPokegearInField(command) => {
                RuntimeMutationResult::FieldPokegearUsed(
                    self.use_bag_pokegear_in_field(state, &command.item_id)?,
                )
            }
            RuntimeMutationCommand::UseBagBoxInField(command) => {
                RuntimeMutationResult::FieldBoxUsed(
                    self.use_bag_box_in_field(state, &command.item_id)?,
                )
            }
            RuntimeMutationCommand::UseBagEscapeRopeInField(command) => {
                RuntimeMutationResult::FieldEscapeRopeUsed(self.use_bag_escape_rope_in_session(
                    state,
                    session,
                    &command.item_id,
                    music_ids,
                )?)
            }
            RuntimeMutationCommand::UseCutFieldMove(command) => {
                RuntimeMutationResult::CutFieldMoveUsed(self.use_cut_field_move(
                    state,
                    session,
                    command.party_index,
                    command.metatile_x,
                    command.metatile_y,
                )?)
            }
            RuntimeMutationCommand::UseWhirlpoolFieldMove(command) => {
                RuntimeMutationResult::WhirlpoolFieldMoveUsed(self.use_whirlpool_field_move(
                    state,
                    session,
                    command.party_index,
                    command.metatile_x,
                    command.metatile_y,
                )?)
            }
            RuntimeMutationCommand::UseStrengthFieldMove(command) => {
                RuntimeMutationResult::StrengthFieldMoveUsed(
                    self.use_strength_field_move(state, command.party_index)?,
                )
            }
            RuntimeMutationCommand::UseFlashFieldMove(command) => {
                RuntimeMutationResult::FlashFieldMoveUsed(
                    self.use_flash_field_move(state, command.party_index)?,
                )
            }
            RuntimeMutationCommand::UseSurfFieldMove(command) => {
                RuntimeMutationResult::SurfFieldMoveUsed(self.use_surf_field_move(
                    state,
                    session,
                    command.party_index,
                )?)
            }
            RuntimeMutationCommand::UseWaterfallFieldMove(command) => {
                RuntimeMutationResult::WaterfallFieldMoveUsed(self.use_waterfall_field_move(
                    state,
                    session,
                    command.party_index,
                )?)
            }
            RuntimeMutationCommand::UseFlyFieldMove(command) => {
                RuntimeMutationResult::FlyFieldMoveUsed(self.use_fly_field_move_in_session(
                    state,
                    session,
                    command.party_index,
                    command.destination_spawn_identifier,
                    &command.flypoint_flag,
                    music_ids,
                )?)
            }
            RuntimeMutationCommand::UseDigFieldMove(command) => {
                RuntimeMutationResult::DigFieldMoveUsed(self.use_dig_field_move_in_session(
                    state,
                    session,
                    command.party_index,
                    music_ids,
                )?)
            }
            RuntimeMutationCommand::UseTeleportFieldMove(command) => {
                RuntimeMutationResult::TeleportFieldMoveUsed(
                    self.use_teleport_field_move_in_session(
                        state,
                        session,
                        command.party_index,
                        music_ids,
                    )?,
                )
            }
            RuntimeMutationCommand::UseHeadbuttFieldMove(command) => {
                reject_field_encounter_surface("HEADBUTT", command.surface)?;
                let player_id = command
                    .player_id
                    .with_context(|| "HEADBUTT field move command requires player_id")?;
                let mut next_state = state.clone();
                let next_session = session.clone();
                let outcome = self.use_headbutt_field_move(
                    &mut next_state,
                    &next_session,
                    command.party_index,
                    player_id,
                )?;
                ensure_runtime_command_rng_boundary(
                    "use HEADBUTT field move",
                    next_state.rng_seed,
                    command.rng_seed_after,
                )?;
                *state = next_state;
                *session = next_session;
                RuntimeMutationResult::HeadbuttFieldMoveUsed(outcome)
            }
            RuntimeMutationCommand::UseRockSmashFieldMove(command) => {
                let mut next_state = state.clone();
                let mut next_session = session.clone();
                let outcome = self.use_rock_smash_field_move(
                    &mut next_state,
                    &mut next_session,
                    command.party_index,
                )?;
                *state = next_state;
                *session = next_session;
                RuntimeMutationResult::RockSmashFieldMoveUsed(outcome)
            }
            RuntimeMutationCommand::UseSweetScentFieldMove(command) => {
                reject_field_encounter_player_id("SWEET_SCENT", command.player_id)?;
                let surface = command
                    .surface
                    .with_context(|| "SWEET_SCENT field move command requires surface")?;
                let mut next_state = state.clone();
                let next_session = session.clone();
                let outcome = self.use_sweet_scent_field_move(
                    &mut next_state,
                    &next_session,
                    command.party_index,
                    surface,
                )?;
                ensure_runtime_command_rng_boundary(
                    "use SWEET_SCENT field move",
                    next_state.rng_seed,
                    command.rng_seed_after,
                )?;
                *state = next_state;
                *session = next_session;
                RuntimeMutationResult::SweetScentFieldMoveUsed(outcome)
            }
            RuntimeMutationCommand::UseBagItemOnPartyPokemon(command) => {
                let (item_use, item_effect) = self.use_bag_item_on_party_pokemon_now(
                    state,
                    &command.item_id,
                    command.party_index,
                )?;
                RuntimeMutationResult::PartyPokemonItemUsed(item_use, item_effect)
            }
            RuntimeMutationCommand::UseBagItemOnWholeParty(command) => {
                let (item_use, item_effect) =
                    self.use_bag_item_on_whole_party(state, &command.item_id)?;
                RuntimeMutationResult::WholePartyItemUsed(item_use, item_effect)
            }
            RuntimeMutationCommand::UseBagItemOnPartyMove(command) => {
                let (item_use, item_effect) = self.use_bag_pp_item_on_party_pokemon(
                    state,
                    &command.item_id,
                    command.party_index,
                    command.move_slot,
                )?;
                RuntimeMutationResult::PartyMoveItemUsed(item_use, item_effect)
            }
            RuntimeMutationCommand::UseBagTmHmOnPartyPokemon(command) => {
                let (item_use, learned_move) = self.use_bag_tmhm_on_party_pokemon(
                    state,
                    &command.item_id,
                    command.party_index,
                    command.replace_slot,
                )?;
                RuntimeMutationResult::TmHmItemUsed(item_use, learned_move)
            }
            RuntimeMutationCommand::UseBagItemOnActiveBattlePokemon(command) => {
                let (item_use, battle_item) =
                    self.use_bag_item_on_active_battle_pokemon(state, &command.item_id)?;
                RuntimeMutationResult::ActiveBattlePokemonItemUsed(item_use, battle_item)
            }
            RuntimeMutationCommand::UseBagItemOnBattlePartyPokemon(command) => {
                let (item_use, battle_item) = self.use_bag_item_on_battle_party_pokemon(
                    state,
                    &command.item_id,
                    command.party_index,
                )?;
                RuntimeMutationResult::BattlePartyPokemonItemUsed(item_use, battle_item)
            }
            RuntimeMutationCommand::UseBagItemOnBattlePartyMove(command) => {
                let (item_use, battle_item) = self.use_bag_item_on_battle_party_move(
                    state,
                    &command.item_id,
                    command.party_index,
                    command.move_slot,
                )?;
                RuntimeMutationResult::BattlePartyMoveItemUsed(item_use, battle_item)
            }
            RuntimeMutationCommand::ThrowBallAtActiveBattle(command) => {
                let mut next_state = state.clone();
                let outcome =
                    self.throw_ball_at_active_battle(&mut next_state, &command.item_id)?;
                ensure_runtime_command_rng_boundary(
                    "throw ball at active battle",
                    next_state.rng_seed,
                    command.rng_seed_after,
                )?;
                *state = next_state;
                RuntimeMutationResult::BallThrown(outcome)
            }
            RuntimeMutationCommand::CompleteActiveWildCapture(command) => {
                RuntimeMutationResult::ActiveWildCaptureCompleted(
                    self.complete_active_wild_capture(
                        state,
                        &command.outcome,
                        command.nickname.as_deref(),
                    )?,
                )
            }
            RuntimeMutationCommand::SwitchActiveBattleParty(command) => {
                RuntimeMutationResult::ActiveBattlePartySwitched(
                    switch_active_battle_party_index(state, command.party_index)
                        .map_err(|error| anyhow::anyhow!("switch active battle party: {error}"))?,
                )
            }
            RuntimeMutationCommand::ResolveActiveBattleTurn(command) => {
                let mut next_state = state.clone();
                let outcome = self.resolve_active_battle_turn(
                    &mut next_state,
                    command.player_action,
                    command.enemy_action,
                )?;
                ensure_runtime_command_rng_boundary(
                    "resolve active battle turn",
                    next_state.rng_seed,
                    command.rng_seed_after,
                )?;
                *state = next_state;
                RuntimeMutationResult::ActiveBattleTurnResolved(outcome)
            }
            RuntimeMutationCommand::ResolveActiveBattleCommand(command) => {
                let mut next_state = state.clone();
                let outcome = self.resolve_active_battle_command(
                    &mut next_state,
                    command.player_action,
                    command.enemy_action,
                )?;
                ensure_runtime_command_rng_boundary(
                    "resolve active battle command",
                    next_state.rng_seed,
                    command.rng_seed_after,
                )?;
                *state = next_state;
                RuntimeMutationResult::ActiveBattleCommandResolved(outcome)
            }
            RuntimeMutationCommand::ResolveActiveBattleEnemyAction(command) => {
                let mut next_state = state.clone();
                let outcome =
                    self.resolve_active_battle_enemy_action(&mut next_state, command.enemy_action)?;
                ensure_runtime_command_rng_boundary(
                    "resolve active battle enemy action",
                    next_state.rng_seed,
                    command.rng_seed_after,
                )?;
                *state = next_state;
                RuntimeMutationResult::ActiveBattleEnemyActionResolved(outcome)
            }
            RuntimeMutationCommand::AttemptEscapeActiveWildBattle(command) => {
                let mut next_state = state.clone();
                let outcome = self.resolve_active_wild_battle_run(&mut next_state)?;
                ensure_runtime_command_rng_boundary(
                    "attempt active wild battle escape",
                    next_state.rng_seed,
                    command.rng_seed_after,
                )?;
                *state = next_state;
                RuntimeMutationResult::ActiveWildBattleEscapeAttempted(outcome)
            }
            RuntimeMutationCommand::UseBagItemToEscapeActiveWildBattle(command) => {
                RuntimeMutationResult::ActiveWildBattleEscapeItemUsed(
                    self.use_bag_item_to_escape_active_wild_battle(state, &command.item_id)?,
                )
            }
            RuntimeMutationCommand::UseBagGuardSpecInActiveBattle(command) => {
                RuntimeMutationResult::ActiveBattleGuardSpecUsed(
                    self.use_bag_guard_spec_in_active_battle(state, &command.item_id)?,
                )
            }
            RuntimeMutationCommand::AdvanceActiveTrainerBattle => {
                RuntimeMutationResult::ActiveTrainerBattleAdvanced(
                    self.advance_active_trainer_battle(state)?,
                )
            }
            RuntimeMutationCommand::ClaimActiveTrainerBattleRewardsNow => {
                RuntimeMutationResult::ActiveTrainerBattleRewardsClaimed(
                    self.claim_active_trainer_battle_rewards_now(state)?,
                )
            }
            RuntimeMutationCommand::ClaimActiveWildBattleRewardsNow => {
                RuntimeMutationResult::ActiveWildBattleRewardsClaimed(
                    self.claim_active_wild_battle_rewards_now(state)?,
                )
            }
            RuntimeMutationCommand::CastFishingRod { rod } => {
                RuntimeMutationResult::FishingRodCast(
                    self.cast_fishing_rod_in_session(state, session, &rod)?,
                )
            }
            RuntimeMutationCommand::UseBagFishingRodInField(command) => {
                RuntimeMutationResult::BagFishingRodUsed(self.use_bag_fishing_rod_in_field(
                    state,
                    session,
                    &command.item_id,
                )?)
            }
            RuntimeMutationCommand::UpdateClockFromDatetime(command) => {
                self.update_clock_from_datetime(
                    state,
                    command.date,
                    command.hour,
                    command.minute,
                    command.second,
                );
                RuntimeMutationResult::ClockUpdated
            }
            RuntimeMutationCommand::SetManualClockTime(command) => {
                self.set_manual_clock_time(
                    state,
                    command.now_date,
                    command.now_hour,
                    command.now_minute,
                    command.now_second,
                    command.target,
                );
                RuntimeMutationResult::ManualClockSet
            }
            RuntimeMutationCommand::ApplyScriptSwarm(command) => {
                RuntimeMutationResult::ScriptSwarmApplied(
                    self.apply_script_swarm_command_in_session(
                        state,
                        session,
                        &command.map_name,
                        &command.source_script,
                        command.command_index,
                    )?,
                )
            }
            RuntimeMutationCommand::ExecuteNextQueuedScriptCommand => {
                if state.script_runtime.command_queue.is_empty() {
                    anyhow::bail!(
                        "cannot execute queued script command because the queue is empty"
                    );
                }
                let queued = state.script_runtime.command_queue.remove(0);
                state.script_runtime.next_script = Some(ScriptLocation {
                    origin_map_name: queued.origin_map_name.clone(),
                    script: queued.target.clone(),
                });
                state
                    .script_runtime
                    .control_events
                    .push(ScriptControlRuntimeEvent {
                        kind: ScriptControlRuntimeKind::Jump,
                        target_script: Some(queued.target.clone()),
                        source_script: queued.source_script.clone(),
                        command_index: queued.command_index,
                    });
                RuntimeMutationResult::QueuedScriptCommandExecuted(queued)
            }
            RuntimeMutationCommand::UseDayCare(command) => {
                if matches!(command.action, RuntimeDayCareAction::CollectEgg)
                    && !matches!(command.caretaker, RuntimeDayCareCaretaker::Man)
                {
                    anyhow::bail!("Day Care egg collection is only available from the man");
                }
                let routine = match command.caretaker {
                    RuntimeDayCareCaretaker::Man => "DayCareMan",
                    RuntimeDayCareCaretaker::Lady => "DayCareLady",
                };
                let action = runtime_day_care_action_name(command.action);
                let party_slot = runtime_day_care_party_slot(&command)?;
                RuntimeMutationResult::DayCareUsed(self.apply_special_routine_transactional(
                    state,
                    routine,
                    music_ids,
                    |next_state| {
                        next_state
                            .script_runtime
                            .variables
                            .insert("_day_care_action".to_string(), action.to_string());
                        match party_slot {
                            Some(party_slot) => {
                                next_state
                                    .script_runtime
                                    .variables
                                    .insert("_party_slot".to_string(), party_slot.to_string());
                            }
                            None => {
                                next_state.script_runtime.variables.remove("_party_slot");
                            }
                        }
                        Ok(())
                    },
                )?)
            }
            RuntimeMutationCommand::CheckDayCareManOutsideSpecial => {
                RuntimeMutationResult::DayCareManOutsideChecked(self.apply_special_routine(
                    state,
                    "DayCareManOutside",
                    music_ids,
                )?)
            }
            RuntimeMutationCommand::CheckDayCareResidentSpecial(caretaker) => {
                let routine = match caretaker {
                    RuntimeDayCareCaretaker::Man => "DayCareMon1",
                    RuntimeDayCareCaretaker::Lady => "DayCareMon2",
                };
                RuntimeMutationResult::DayCareResidentChecked(
                    self.apply_special_routine(state, routine, music_ids)?,
                )
            }
            RuntimeMutationCommand::UseBugContest(command) => {
                runtime_bug_contest_rank(&command)?;
                let rng_seed_after = runtime_bug_contest_rng_seed_after(&command)?;
                let routine = match command.action {
                    RuntimeBugContestAction::GiveParkBalls => "GiveParkBalls",
                    RuntimeBugContestAction::SelectContestants => {
                        "SelectRandomBugContestContestants"
                    }
                    RuntimeBugContestAction::DropOffMons => "ContestDropOffMons",
                    RuntimeBugContestAction::ReturnMons => "ContestReturnMons",
                    RuntimeBugContestAction::CheckPartyFull => "CheckPartyFullAfterContest",
                    RuntimeBugContestAction::Judge => "BugContestJudging",
                };
                let mut next_state = state.clone();
                next_state
                    .script_runtime
                    .variables
                    .remove("_bug_contest_rank");
                let outcome = self.apply_special_routine(&mut next_state, routine, music_ids)?;
                if let Some(rng_seed_after) = rng_seed_after {
                    ensure_runtime_command_rng_boundary(
                        &format!("use Bug Contest {}", runtime_bug_contest_action_name(command.action)),
                        next_state.rng_seed,
                        rng_seed_after,
                    )?;
                }
                *state = next_state;
                RuntimeMutationResult::BugContestUsed(outcome)
            }
            RuntimeMutationCommand::UseKurtApricorn(command) => {
                RuntimeMutationResult::KurtApricornUsed(self.apply_special_routine_transactional(
                    state,
                    "SelectApricornForKurt",
                    music_ids,
                    |next_state| {
                        next_state
                            .script_runtime
                            .variables
                            .insert("_kurt_apricorn_type".to_string(), command.apricorn_id);
                        next_state.script_runtime.variables.insert(
                            "_kurt_apricorn_quantity".to_string(),
                            command.quantity.to_string(),
                        );
                        Ok(())
                    },
                )?)
            }
            RuntimeMutationCommand::UseBuenaPassword(command) => {
                let mut next_state = state.clone();
                match command.guess {
                    Some(guess) => {
                        next_state
                            .script_runtime
                            .variables
                            .insert("BUENA_PASSWORD".to_string(), guess);
                    }
                    None => {
                        next_state.script_runtime.variables.remove("BUENA_PASSWORD");
                    }
                }
                let outcome =
                    self.apply_special_routine(&mut next_state, "BuenasPassword", music_ids)?;
                ensure_runtime_command_rng_boundary(
                    "use Buena password",
                    next_state.rng_seed,
                    command.rng_seed_after,
                )?;
                *state = next_state;
                RuntimeMutationResult::BuenaPasswordUsed(outcome)
            }
            RuntimeMutationCommand::UseBuenaPrize(command) => {
                RuntimeMutationResult::BuenaPrizeUsed(self.apply_special_routine_transactional(
                    state,
                    "BuenaPrize",
                    music_ids,
                    |next_state| {
                        next_state
                            .script_runtime
                            .variables
                            .insert("_selected_prize".to_string(), command.item_id);
                        next_state.script_runtime.variables.insert(
                            "_selected_prize_quantity".to_string(),
                            command.quantity.to_string(),
                        );
                        Ok(())
                    },
                )?)
            }
            RuntimeMutationCommand::UseShuckie(command) => {
                let party_slot = runtime_shuckie_party_slot(&command)?;
                let rng_seed_after = runtime_shuckie_rng_seed_after(&command)?;
                let routine = match command.action {
                    RuntimeShuckieAction::Give => "GiveShuckle",
                    RuntimeShuckieAction::Return => "ReturnShuckie",
                };
                let mut next_state = state.clone();
                match party_slot {
                    Some(party_index) => {
                        next_state
                            .script_runtime
                            .variables
                            .insert("_selection_cancelled".to_string(), "0".to_string());
                        next_state
                            .script_runtime
                            .variables
                            .insert("_selected_party_index".to_string(), party_index.to_string());
                    }
                    None => {
                        next_state
                            .script_runtime
                            .variables
                            .insert("_selection_cancelled".to_string(), "1".to_string());
                        next_state
                            .script_runtime
                            .variables
                            .remove("_selected_party_index");
                    }
                }
                let outcome = self.apply_special_routine(&mut next_state, routine, music_ids)?;
                if let Some(rng_seed_after) = rng_seed_after {
                    ensure_runtime_command_rng_boundary(
                        "use Shuckie give",
                        next_state.rng_seed,
                        rng_seed_after,
                    )?;
                }
                *state = next_state;
                RuntimeMutationResult::ShuckieUsed(outcome)
            }
            RuntimeMutationCommand::GiveOddEgg(command) => {
                let mut next_state = state.clone();
                let outcome =
                    self.apply_special_routine(&mut next_state, "GiveOddEgg", music_ids)?;
                ensure_runtime_command_rng_boundary(
                    "give Odd Egg",
                    next_state.rng_seed,
                    command.rng_seed_after,
                )?;
                *state = next_state;
                RuntimeMutationResult::OddEggGiven(outcome)
            }
            RuntimeMutationCommand::GiveDratini(command) => {
                RuntimeMutationResult::DratiniGiven(self.apply_special_routine_transactional(
                    state,
                    "GiveDratini",
                    music_ids,
                    |next_state| {
                        next_state.script_runtime.script_value = Some(command.mode.to_string());
                        next_state
                            .script_runtime
                            .variables
                            .insert("_value".to_string(), command.mode.to_string());
                        Ok(())
                    },
                )?)
            }
            RuntimeMutationCommand::UseBillsGrandfather(command) => {
                let (party_index, species_id) = runtime_bills_grandfather_inputs(&command)?;
                RuntimeMutationResult::BillsGrandfatherUsed(
                    self.apply_special_routine_transactional(
                        state,
                        "BillsGrandfather",
                        music_ids,
                        |next_state| {
                            match party_index {
                                Some(party_index) => {
                                    next_state.script_runtime.variables.insert(
                                        "_selected_party_index".to_string(),
                                        party_index.to_string(),
                                    );
                                    next_state
                                        .script_runtime
                                        .variables
                                        .remove("_selected_species");
                                }
                                None => {
                                    next_state
                                        .script_runtime
                                        .variables
                                        .remove("_selected_party_index");
                                }
                            }
                            match species_id {
                                Some(species_id) => {
                                    next_state
                                        .script_runtime
                                        .variables
                                        .insert("_selected_species".to_string(), species_id);
                                }
                                None => {
                                    next_state
                                        .script_runtime
                                        .variables
                                        .remove("_selected_species");
                                }
                            }
                            Ok(())
                        },
                    )?,
                )
            }
            RuntimeMutationCommand::InitRoamMons => RuntimeMutationResult::RoamersInitialized(
                self.apply_special_routine(state, "InitRoamMons", music_ids)?,
            ),
            RuntimeMutationCommand::CheckMagikarpLength(command) => {
                RuntimeMutationResult::MagikarpLengthChecked(
                    self.apply_special_routine_transactional(
                        state,
                        "CheckMagikarpLength",
                        music_ids,
                        |next_state| {
                            next_state
                                .script_runtime
                                .variables
                                .insert("_selection_cancelled".to_string(), "0".to_string());
                            next_state.script_runtime.variables.insert(
                                "_selected_party_index".to_string(),
                                command.party_index.to_string(),
                            );
                            Ok(())
                        },
                    )?,
                )
            }
            RuntimeMutationCommand::ShowProfOaksPcBoot => {
                RuntimeMutationResult::ProfOaksPcBootShown(self.apply_special_routine(
                    state,
                    "ProfOaksPCBoot",
                    music_ids,
                )?)
            }
            RuntimeMutationCommand::ShowMagikarpHouseSign => {
                RuntimeMutationResult::MagikarpHouseSignShown(self.apply_special_routine(
                    state,
                    "MagikarpHouseSign",
                    music_ids,
                )?)
            }
            RuntimeMutationCommand::ApplyBattleTowerAction(command) => {
                let (level_group, selected_reward) = runtime_battle_tower_action_inputs(&command)?;
                RuntimeMutationResult::BattleTowerActionApplied(
                    self.apply_special_routine_transactional(
                        state,
                        "BattleTowerAction",
                        music_ids,
                        |next_state| {
                            next_state
                                .script_runtime
                                .variables
                                .insert("_value".to_string(), command.action);
                            match level_group {
                                Some(level_group) => {
                                    next_state.script_runtime.variables.insert(
                                        "_battle_tower_level_group".to_string(),
                                        level_group.to_string(),
                                    );
                                }
                                None => {
                                    next_state
                                        .script_runtime
                                        .variables
                                        .remove("_battle_tower_level_group");
                                }
                            }
                            match selected_reward {
                                Some(selected_reward) => {
                                    next_state
                                        .script_runtime
                                        .variables
                                        .insert("_selected_reward".to_string(), selected_reward);
                                }
                                None => {
                                    next_state
                                        .script_runtime
                                        .variables
                                        .remove("_selected_reward");
                                }
                            }
                            Ok(())
                        },
                    )?,
                )
            }
            RuntimeMutationCommand::OpenBattleTowerRoomMenuSpecial => {
                RuntimeMutationResult::BattleTowerRoomMenuOpened(self.apply_special_routine(
                    state,
                    "BattleTowerRoomMenu",
                    music_ids,
                )?)
            }
            RuntimeMutationCommand::StartBattleTowerBattleSpecial(command) => {
                RuntimeMutationResult::BattleTowerBattleStarted(
                    self.apply_special_routine_transactional(
                        state,
                        "BattleTowerBattle",
                        music_ids,
                        |next_state| {
                            next_state.script_runtime.variables.insert(
                                "_battle_result".to_string(),
                                command.battle_result.to_string(),
                            );
                            Ok(())
                        },
                    )?,
                )
            }
            RuntimeMutationCommand::LoadBattleTowerOpponentSpecial(command) => {
                RuntimeMutationResult::BattleTowerOpponentLoaded(
                    self.apply_special_routine_transactional(
                        state,
                        "LoadOpponentTrainerAndPokemonWithOTSprite",
                        music_ids,
                        |next_state| {
                            next_state
                                .script_runtime
                                .variables
                                .insert("_battle_tower_trainer_id".to_string(), command.trainer_id);
                            next_state.script_runtime.variables.insert(
                                "_battle_tower_sprite_constant".to_string(),
                                command.sprite_constant,
                            );
                            next_state.script_runtime.variables.insert(
                                "_battle_tower_target_object".to_string(),
                                command.target_object,
                            );
                            Ok(())
                        },
                    )?,
                )
            }
            RuntimeMutationCommand::ShowBattleTowerMobileErrorSpecial => {
                RuntimeMutationResult::BattleTowerMobileErrorShown(self.apply_special_routine(
                    state,
                    "BattleTowerMobileError",
                    music_ids,
                )?)
            }
            RuntimeMutationCommand::AskRememberPasswordSpecial(command) => {
                RuntimeMutationResult::RememberPasswordAsked(
                    self.apply_special_routine_transactional(
                        state,
                        "AskRememberPassword",
                        music_ids,
                        |next_state| {
                            next_state.script_runtime.variables.insert(
                                "_yes_no_result".to_string(),
                                u8::from(command.remember).to_string(),
                            );
                            Ok(())
                        },
                    )?,
                )
            }
            RuntimeMutationCommand::OpenBattleTowerLeaderboardSpecial => {
                RuntimeMutationResult::BattleTowerLeaderboardOpened(self.apply_special_routine(
                    state,
                    "Function1700ba",
                    music_ids,
                )?)
            }
            RuntimeMutationCommand::ApplyMobileHandshakeSpecial(command) => {
                RuntimeMutationResult::MobileHandshakeApplied(
                    self.apply_special_routine_transactional(
                        state,
                        "Function1011f1",
                        music_ids,
                        |next_state| {
                            next_state.script_runtime.variables.insert(
                                "_mobile_adapter_status".to_string(),
                                u8::from(command.accepted).to_string(),
                            );
                            next_state.script_runtime.variables.insert(
                                "_mobile_adapter_secondary_status".to_string(),
                                u8::from(command.accepted).to_string(),
                            );
                            next_state
                                .script_runtime
                                .variables
                                .entry("_mobile_login_password".to_string())
                                .or_insert_with(String::new);
                            next_state
                                .script_runtime
                                .variables
                                .entry("_mobile_battle_timer".to_string())
                                .or_insert_with(|| "0,0,0".to_string());
                            Ok(())
                        },
                    )?,
                )
            }
            RuntimeMutationCommand::EndMobileSessionSpecial => {
                RuntimeMutationResult::MobileSessionEnded(self.apply_special_routine(
                    state,
                    "Function101220",
                    music_ids,
                )?)
            }
            RuntimeMutationCommand::SetBattleTowerMobileFlagSpecial(flag) => {
                let routine = match flag {
                    RuntimeBattleTowerMobileFlag::Enabled => "Function103780",
                    RuntimeBattleTowerMobileFlag::Disabled => "Function1037c2",
                };
                RuntimeMutationResult::BattleTowerMobileFlagSet(
                    self.apply_special_routine(state, routine, music_ids)?,
                )
            }
            RuntimeMutationCommand::SelectThreeMobileMonsSpecial(command) => {
                RuntimeMutationResult::MobileThreeMonsSelected(
                    self.apply_special_routine_transactional(
                        state,
                        "Mobile_SelectThreeMons",
                        music_ids,
                        |next_state| {
                            let selected = command
                                .party_indexes
                                .iter()
                                .map(usize::to_string)
                                .collect::<Vec<_>>()
                                .join(",");
                            next_state
                                .script_runtime
                                .variables
                                .insert("_selected_party_indexes".to_string(), selected);
                            Ok(())
                        },
                    )?,
                )
            }
            RuntimeMutationCommand::ApplyHappinessService(command) => {
                let routine = match command.routine {
                    RuntimeHappinessServiceRoutine::OlderHaircutBrother => "OlderHaircutBrother",
                    RuntimeHappinessServiceRoutine::YoungerHaircutBrother => {
                        "YoungerHaircutBrother"
                    }
                    RuntimeHappinessServiceRoutine::DaisysGrooming => "DaisysGrooming",
                };
                let mut next_state = state.clone();
                next_state
                    .script_runtime
                    .variables
                    .insert("_party_slot".to_string(), command.party_index.to_string());
                next_state
                    .script_runtime
                    .variables
                    .insert("_rng_roll".to_string(), command.rng_roll.to_string());
                let outcome = self.apply_special_routine(&mut next_state, routine, music_ids)?;
                ensure_runtime_command_rng_boundary(
                    "apply happiness service",
                    next_state.rng_seed,
                    command.rng_seed_after,
                )?;
                *state = next_state;
                RuntimeMutationResult::HappinessServiceApplied(outcome)
            }
            RuntimeMutationCommand::UseMysteryGift(action) => {
                let routine = match action {
                    RuntimeMysteryGiftAction::Check => "CheckMysteryGift",
                    RuntimeMysteryGiftAction::ClaimItem => "GetMysteryGiftItem",
                    RuntimeMysteryGiftAction::Unlock => "UnlockMysteryGift",
                };
                RuntimeMutationResult::MysteryGiftUsed(
                    self.apply_special_routine(state, routine, music_ids)?,
                )
            }
            RuntimeMutationCommand::WarpToSpawnPoint => RuntimeMutationResult::SpawnPointWarped(
                self.apply_special_routine(state, "WarpToSpawnPoint", music_ids)?,
            ),
            RuntimeMutationCommand::HealPartySpecial => {
                RuntimeMutationResult::PartyHealedBySpecial(self.apply_special_routine(
                    state,
                    "HealParty",
                    music_ids,
                )?)
            }
            RuntimeMutationCommand::FadeOutMusicSpecial => {
                RuntimeMutationResult::MusicFadedOutBySpecial(self.apply_special_routine(
                    state,
                    "FadeOutMusic",
                    music_ids,
                )?)
            }
            RuntimeMutationCommand::WaitSfxSpecial => RuntimeMutationResult::SoundEffectWaitQueued(
                self.apply_special_routine(state, "WaitSFX", music_ids)?,
            ),
            RuntimeMutationCommand::PlayMapMusicSpecial => {
                RuntimeMutationResult::MapMusicPlayedBySpecial(self.apply_special_routine(
                    state,
                    "PlayMapMusic",
                    music_ids,
                )?)
            }
            RuntimeMutationCommand::RestartMapMusicSpecial => {
                RuntimeMutationResult::MapMusicRestartedBySpecial(self.apply_special_routine(
                    state,
                    "RestartMapMusic",
                    music_ids,
                )?)
            }
            RuntimeMutationCommand::PlayCurMonCry(command) => {
                let species_id = runtime_special_cry_species(&command)?.to_string();
                RuntimeMutationResult::CurrentMonCryPlayed(
                    self.apply_special_routine_transactional(
                        state,
                        "PlayCurMonCry",
                        music_ids,
                        |next_state| {
                            next_state
                                .script_runtime
                                .variables
                                .insert("wCurPartySpecies".to_string(), species_id);
                            Ok(())
                        },
                    )?,
                )
            }
            RuntimeMutationCommand::PlaySlowCry(command) => {
                let species_id = runtime_special_cry_species(&command)?.to_string();
                RuntimeMutationResult::SlowCryPlayed(self.apply_special_routine_transactional(
                    state,
                    "PlaySlowCry",
                    music_ids,
                    |next_state| {
                        next_state
                            .script_runtime
                            .variables
                            .insert("_value".to_string(), species_id);
                        Ok(())
                    },
                )?)
            }
            RuntimeMutationCommand::OpenPokemonCenterPcSpecial => {
                RuntimeMutationResult::PokemonCenterPcOpened(self.apply_special_routine(
                    state,
                    "PokemonCenterPC",
                    music_ids,
                )?)
            }
            RuntimeMutationCommand::OpenPlayersHousePcSpecial => {
                RuntimeMutationResult::PlayersHousePcOpened(self.apply_special_routine(
                    state,
                    "PlayersHousePC",
                    music_ids,
                )?)
            }
            RuntimeMutationCommand::OpenOverworldTownMapSpecial => {
                RuntimeMutationResult::OverworldTownMapOpened(self.apply_special_routine(
                    state,
                    "OverworldTownMap",
                    music_ids,
                )?)
            }
            RuntimeMutationCommand::OpenUnownPrinterSpecial => {
                RuntimeMutationResult::UnownPrinterOpened(self.apply_special_routine(
                    state,
                    "UnownPrinter",
                    music_ids,
                )?)
            }
            RuntimeMutationCommand::OpenMapRadioSpecial(command) => {
                let station = runtime_map_radio_station(&command)?.to_string();
                RuntimeMutationResult::MapRadioOpened(self.apply_special_routine_transactional(
                    state,
                    "MapRadio",
                    music_ids,
                    |next_state| {
                        next_state
                            .script_runtime
                            .variables
                            .insert("_value".to_string(), station);
                        Ok(())
                    },
                )?)
            }
            RuntimeMutationCommand::NameRivalSpecial(command) => {
                RuntimeMutationResult::RivalNamed(self.apply_special_routine_transactional(
                    state,
                    "NameRival",
                    music_ids,
                    |next_state| {
                        next_state
                            .script_runtime
                            .variables
                            .insert("_rival_name".to_string(), command.rival_name);
                        Ok(())
                    },
                )?)
            }
            RuntimeMutationCommand::DeletePartyMoveSpecial(command) => {
                RuntimeMutationResult::PartyMoveDeletedBySpecial(
                    self.apply_special_routine_transactional(
                        state,
                        "MoveDeletion",
                        music_ids,
                        |next_state| {
                            next_state
                                .script_runtime
                                .variables
                                .insert("_party_slot".to_string(), command.party_index.to_string());
                            next_state
                                .script_runtime
                                .variables
                                .insert("_move_slot".to_string(), command.move_index.to_string());
                            Ok(())
                        },
                    )?,
                )
            }
            RuntimeMutationCommand::CheckPokerusSpecial => RuntimeMutationResult::PokerusChecked(
                self.apply_special_routine(state, "CheckPokerus", music_ids)?,
            ),
            RuntimeMutationCommand::RatePartyNicknameSpecial(command) => {
                RuntimeMutationResult::PartyNicknameRated(
                    self.apply_special_routine_transactional(
                        state,
                        "NameRater",
                        music_ids,
                        |next_state| {
                            next_state
                                .script_runtime
                                .variables
                                .insert("_party_slot".to_string(), command.party_index.to_string());
                            next_state
                                .script_runtime
                                .variables
                                .insert("_selected_nickname".to_string(), command.nickname);
                            Ok(())
                        },
                    )?,
                )
            }
            RuntimeMutationCommand::SeePartyPokemonSpecial(command) => {
                RuntimeMutationResult::PartyPokemonSeenBySeer(
                    self.apply_special_routine_transactional(
                        state,
                        "PokeSeer",
                        music_ids,
                        |next_state| {
                            next_state
                                .script_runtime
                                .variables
                                .insert("_party_slot".to_string(), command.party_index.to_string());
                            Ok(())
                        },
                    )?,
                )
            }
            RuntimeMutationCommand::TeachPartyMoveSpecial(command) => {
                RuntimeMutationResult::PartyMoveTaughtBySpecial(
                    self.apply_special_routine_transactional(
                        state,
                        "MoveTutor",
                        music_ids,
                        |next_state| {
                            next_state
                                .script_runtime
                                .variables
                                .insert("_party_slot".to_string(), command.party_index.to_string());
                            next_state
                                .script_runtime
                                .variables
                                .insert("_move".to_string(), command.move_id);
                            Ok(())
                        },
                    )?,
                )
            }
            RuntimeMutationCommand::OpenBankOfMomSpecial => RuntimeMutationResult::BankOfMomOpened(
                self.apply_special_routine(state, "BankOfMom", music_ids)?,
            ),
            RuntimeMutationCommand::OpenGameCornerSpecial(service) => {
                let routine = match service {
                    RuntimeGameCornerService::SlotMachine => "SlotMachine",
                    RuntimeGameCornerService::CardFlip => "CardFlip",
                };
                RuntimeMutationResult::GameCornerOpened(
                    self.apply_special_routine(state, routine, music_ids)?,
                )
            }
            RuntimeMutationCommand::OpenDisplayLinkRecordSpecial => {
                RuntimeMutationResult::DisplayLinkRecordOpened(self.apply_special_routine(
                    state,
                    "DisplayLinkRecord",
                    music_ids,
                )?)
            }
            RuntimeMutationCommand::OpenTrainerHouseSpecial => {
                RuntimeMutationResult::TrainerHouseOpened(self.apply_special_routine(
                    state,
                    "TrainerHouse",
                    music_ids,
                )?)
            }
            RuntimeMutationCommand::OpenPhotoStudioSpecial(command) => {
                RuntimeMutationResult::PhotoStudioOpened(self.apply_special_routine_transactional(
                    state,
                    "PhotoStudio",
                    music_ids,
                    |next_state| {
                        next_state
                            .script_runtime
                            .variables
                            .insert("_party_slot".to_string(), command.party_index.to_string());
                        Ok(())
                    },
                )?)
            }
            RuntimeMutationCommand::CancelBattleTowerChallengeExplanationSpecial => {
                RuntimeMutationResult::BattleTowerChallengeExplanationCancelled(
                    self.apply_special_routine(
                        state,
                        "Menu_ChallengeExplanationCancel",
                        music_ids,
                    )?,
                )
            }
            RuntimeMutationCommand::SetPlayerPalette(command) => {
                RuntimeMutationResult::PlayerPaletteSet(self.apply_special_routine_transactional(
                    state,
                    "SetPlayerPalette",
                    music_ids,
                    |next_state| {
                        next_state.script_runtime.script_value =
                            Some(command.raw_value.to_string());
                        next_state
                            .script_runtime
                            .variables
                            .insert("_value".to_string(), command.raw_value.to_string());
                        Ok(())
                    },
                )?)
            }
            RuntimeMutationCommand::SetDayOfWeek => RuntimeMutationResult::DayOfWeekSet(
                self.apply_special_routine(state, "SetDayOfWeek", music_ids)?,
            ),
            RuntimeMutationCommand::UpdateTime => RuntimeMutationResult::TimeUpdated(
                self.apply_special_routine(state, "UpdateTime", music_ids)?,
            ),
            RuntimeMutationCommand::SetCableClubRequest(request) => {
                let routine = match request {
                    RuntimeCableClubRequest::Trade => "SetBitsForLinkTradeRequest",
                    RuntimeCableClubRequest::Battle => "SetBitsForBattleRequest",
                    RuntimeCableClubRequest::TimeCapsule => "SetBitsForTimeCapsuleRequest",
                };
                RuntimeMutationResult::CableClubRequestSet(
                    self.apply_special_routine(state, routine, music_ids)?,
                )
            }
            RuntimeMutationCommand::WaitForLinkedFriendSpecial(command) => {
                RuntimeMutationResult::LinkedFriendWaitedFor(
                    self.apply_special_routine_transactional(
                        state,
                        "WaitForLinkedFriend",
                        music_ids,
                        |next_state| {
                            next_state.script_runtime.variables.insert(
                                "_link_friend_ready".to_string(),
                                u8::from(command.ready).to_string(),
                            );
                            Ok(())
                        },
                    )?,
                )
            }
            RuntimeMutationCommand::CheckLinkTimeoutReceptionistSpecial(command) => {
                RuntimeMutationResult::LinkTimeoutReceptionistChecked(
                    self.apply_special_routine_transactional(
                        state,
                        "CheckLinkTimeout_Receptionist",
                        music_ids,
                        |next_state| {
                            next_state.script_runtime.variables.insert(
                                "_link_timeout".to_string(),
                                u8::from(command.timeout).to_string(),
                            );
                            next_state.script_runtime.variables.insert(
                                "_other_player_link_mode".to_string(),
                                command.other_player_link_mode.to_string(),
                            );
                            Ok(())
                        },
                    )?,
                )
            }
            RuntimeMutationCommand::CheckBothSelectedSameRoomSpecial(command) => {
                RuntimeMutationResult::BothSelectedSameRoomChecked(
                    self.apply_special_routine_transactional(
                        state,
                        "CheckBothSelectedSameRoom",
                        music_ids,
                        |next_state| {
                            next_state.script_runtime.variables.insert(
                                "_other_player_room".to_string(),
                                command.other_player_room.to_string(),
                            );
                            Ok(())
                        },
                    )?,
                )
            }
            RuntimeMutationCommand::CloseLinkSpecial => RuntimeMutationResult::LinkClosed(
                self.apply_special_routine(state, "CloseLink", music_ids)?,
            ),
            RuntimeMutationCommand::WaitForOtherPlayerToExitSpecial => {
                RuntimeMutationResult::OtherPlayerExitWaitedFor(self.apply_special_routine(
                    state,
                    "WaitForOtherPlayerToExit",
                    music_ids,
                )?)
            }
            RuntimeMutationCommand::FailedLinkToPastSpecial => {
                RuntimeMutationResult::LinkToPastFailed(self.apply_special_routine(
                    state,
                    "FailedLinkToPast",
                    music_ids,
                )?)
            }
            RuntimeMutationCommand::OpenLinkRoomSpecial(room) => {
                let routine = match room {
                    RuntimeLinkRoomSpecial::TradeCenter => "TradeCenter",
                    RuntimeLinkRoomSpecial::Colosseum => "Colosseum",
                    RuntimeLinkRoomSpecial::TimeCapsule => "EnterTimeCapsule",
                };
                RuntimeMutationResult::LinkRoomOpened(
                    self.apply_special_routine(state, routine, music_ids)?,
                )
            }
            RuntimeMutationCommand::CheckTimeCapsuleCompatibilitySpecial => {
                RuntimeMutationResult::TimeCapsuleCompatibilityChecked(self.apply_special_routine(
                    state,
                    "CheckTimeCapsuleCompatibility",
                    music_ids,
                )?)
            }
            RuntimeMutationCommand::TryQuickSaveSpecial => RuntimeMutationResult::QuickSaveTried(
                self.apply_special_routine(state, "TryQuickSave", music_ids)?,
            ),
            RuntimeMutationCommand::AskMobileOrCableSpecial => {
                RuntimeMutationResult::MobileOrCableAsked(self.apply_special_routine(
                    state,
                    "AskMobileOrCable",
                    music_ids,
                )?)
            }
            RuntimeMutationCommand::CableClubCheckWhichChrisSpecial(command) => {
                RuntimeMutationResult::CableClubChrisChecked(
                    self.apply_special_routine_transactional(
                        state,
                        "CableClubCheckWhichChris",
                        music_ids,
                        |next_state| {
                            next_state
                                .script_runtime
                                .variables
                                .insert("_player_gender".to_string(), command.gender);
                            Ok(())
                        },
                    )?,
                )
            }
            RuntimeMutationCommand::SwitchCurrentPcBox(command) => {
                if command.box_index >= MAX_PC_BOXES {
                    anyhow::bail!(
                        "PC box index {} is outside 0..{MAX_PC_BOXES}",
                        command.box_index
                    );
                }
                while state.storage.pc_boxes.len() <= command.box_index {
                    let next = state.storage.pc_boxes.len();
                    state.storage.pc_boxes.push(PcBox::new(next));
                }
                let before = state.current_pc_box;
                state.current_pc_box = command.box_index;
                RuntimeMutationResult::CurrentPcBoxSwitched(RuntimeStorageBoxSwitchOutcome {
                    box_index_before: before,
                    box_index_after: state.current_pc_box,
                })
            }
            RuntimeMutationCommand::DepositPartyPokemonToCurrentBox(command) => {
                if state.current_pc_box >= MAX_PC_BOXES {
                    anyhow::bail!(
                        "current PC box {} is outside 0..{MAX_PC_BOXES}",
                        state.current_pc_box
                    );
                }
                while state.storage.pc_boxes.len() <= state.current_pc_box {
                    let next = state.storage.pc_boxes.len();
                    state.storage.pc_boxes.push(PcBox::new(next));
                }
                let pokemon = state
                    .storage
                    .party
                    .pokemon
                    .get(command.party_index)
                    .with_context(|| {
                        format!("party index {} is outside party", command.party_index)
                    })?
                    .as_ref()
                    .with_context(|| {
                        format!("party index {} has no Pokemon", command.party_index)
                    })?;
                if state.storage.party.pokemon.iter().flatten().count() <= 1 {
                    anyhow::bail!("cannot deposit the last party Pokemon");
                }
                if pokemon
                    .item
                    .as_deref()
                    .is_some_and(crystal_core::models::item::is_mail_item_id)
                {
                    anyhow::bail!("cannot deposit a Pokemon holding mail");
                }
                let box_slot = state.storage.pc_boxes[state.current_pc_box]
                    .next_open_slot()
                    .with_context(|| format!("PC box {} is full", state.current_pc_box))?;
                let pokemon = take_party_pokemon_compact(state, command.party_index)
                    .with_context(|| format!("deposit party index {}", command.party_index))?;
                state.storage.pc_boxes[state.current_pc_box]
                    .set_slot(box_slot, Some(pokemon.clone()));
                RuntimeMutationResult::PartyPokemonDeposited(RuntimeStorageDepositOutcome {
                    party_index: command.party_index,
                    box_index: state.current_pc_box,
                    box_slot,
                    pokemon,
                })
            }
            RuntimeMutationCommand::WithdrawCurrentBoxPokemonToParty(command) => {
                let party_index = state
                    .storage
                    .party
                    .next_open_slot()
                    .context("party is full")?;
                let Some(pc_box) = state.storage.pc_boxes.get_mut(state.current_pc_box) else {
                    anyhow::bail!("current PC box {} does not exist", state.current_pc_box);
                };
                let pokemon = pc_box
                    .pokemon
                    .get(command.box_slot)
                    .with_context(|| format!("box slot {} is outside PC box", command.box_slot))?
                    .clone()
                    .with_context(|| {
                        format!("box slot {} has no Pokemon to withdraw", command.box_slot)
                })?;
                pc_box.set_slot(command.box_slot, None);
                pc_box.compact();
                state.storage.party.pokemon[party_index] = Some(pokemon.clone());
                state.sync_party_from_storage();
                RuntimeMutationResult::PcPokemonWithdrawn(RuntimeStorageWithdrawOutcome {
                    box_index: state.current_pc_box,
                    box_slot: command.box_slot,
                    party_index,
                    pokemon,
                })
            }
            RuntimeMutationCommand::ReleaseCurrentBoxPokemon(command) => {
                let Some(pc_box) = state.storage.pc_boxes.get_mut(state.current_pc_box) else {
                    anyhow::bail!("current PC box {} does not exist", state.current_pc_box);
                };
                let pokemon = pc_box
                    .pokemon
                    .get(command.box_slot)
                    .with_context(|| format!("box slot {} is outside PC box", command.box_slot))?
                    .clone()
                    .with_context(|| {
                        format!("box slot {} has no Pokemon to release", command.box_slot)
                    })?;
                if pokemon.is_egg
                    || pokemon.species.id.trim().eq_ignore_ascii_case("EGG")
                {
                    anyhow::bail!("cannot release an Egg");
                }
                if pokemon
                    .item
                    .as_deref()
                    .is_some_and(crystal_core::models::item::is_mail_item_id)
                {
                    anyhow::bail!("cannot release a Pokemon holding mail");
                }
                pc_box.set_slot(command.box_slot, None);
                pc_box.compact();
                RuntimeMutationResult::PcPokemonReleased(RuntimeStorageReleaseOutcome {
                    box_index: state.current_pc_box,
                    box_slot: command.box_slot,
                    pokemon,
                })
            }
            RuntimeMutationCommand::MovePcPokemonWithoutMail(command) => {
                for (box_index, pc_box) in state.storage.pc_boxes.iter().enumerate() {
                    if pc_box.pokemon.iter().flatten().any(|pokemon| {
                        pokemon
                            .item
                            .as_deref()
                            .is_some_and(crystal_core::models::item::is_mail_item_id)
                    }) {
                        anyhow::bail!("cannot move Pokemon while box {box_index} contains mail");
                    }
                }
                if state.storage.party.pokemon.iter().flatten().any(|pokemon| {
                    pokemon
                        .item
                        .as_deref()
                        .is_some_and(crystal_core::models::item::is_mail_item_id)
                }) {
                    anyhow::bail!("cannot move Pokemon while the party contains mail");
                }
                if command.source_box >= MAX_PC_BOXES || command.target_box >= MAX_PC_BOXES {
                    anyhow::bail!("PC move box index is outside 0..{MAX_PC_BOXES}");
                }
                while state.storage.pc_boxes.len() <= command.source_box.max(command.target_box) {
                    let next = state.storage.pc_boxes.len();
                    state.storage.pc_boxes.push(PcBox::new(next));
                }
                if command.source_slot >= MAX_BOX_MONS || command.target_slot >= MAX_BOX_MONS {
                    anyhow::bail!("PC move slot is outside 0..{MAX_BOX_MONS}");
                }
                let pokemon = state.storage.pc_boxes[command.source_box].pokemon
                    [command.source_slot]
                    .clone()
                    .context("PC move source slot is empty")?;
                let target = state.storage.pc_boxes[command.target_box].pokemon
                    [command.target_slot]
                    .clone();
                state.storage.pc_boxes[command.target_box]
                    .set_slot(command.target_slot, Some(pokemon));
                state.storage.pc_boxes[command.source_box]
                    .set_slot(command.source_slot, target.clone());
                if target.is_none() {
                    state.storage.pc_boxes[command.source_box].compact();
                }
                state.current_pc_box = command.target_box;
                RuntimeMutationResult::PcPokemonMoved(RuntimeStorageMoveOutcome {
                    source_box: command.source_box,
                    source_slot: command.source_slot,
                    target_box: command.target_box,
                    target_slot: command.target_slot,
                    swapped: target.is_some(),
                })
            }
            RuntimeMutationCommand::DepositBagItemToPc(command) => {
                let item = self
                    .items
                    .get(&command.item_id)
                    .with_context(|| format!("unknown item {}", command.item_id))?;
                if state.bag.quantity(item) < command.quantity {
                    anyhow::bail!(
                        "bag does not contain {} x{} for PC deposit",
                        command.item_id,
                        command.quantity
                    );
                }
                let added = state
                    .bag
                    .add_pc_item(item, command.quantity)
                    .map_err(|error| anyhow::anyhow!("add PC item: {error}"))?;
                if !added {
                    anyhow::bail!(
                        "PC item storage rejected {} x{}",
                        command.item_id,
                        command.quantity
                    );
                }
                state
                    .bag
                    .remove_item(item, command.quantity)
                    .map_err(|error| anyhow::anyhow!("remove bag item for PC deposit: {error}"))?;
                RuntimeMutationResult::BagItemDepositedToPc(RuntimePcItemTransferOutcome {
                    item_id: command.item_id,
                    quantity: command.quantity,
                    bag_quantity_after: state.bag.quantity(item),
                    pc_quantity_after: state.bag.pc_item_quantity(item),
                })
            }
            RuntimeMutationCommand::WithdrawPcItemToBag(command) => {
                let item = self
                    .items
                    .get(&command.item_id)
                    .with_context(|| format!("unknown item {}", command.item_id))?;
                if state.bag.pc_item_quantity(item) < command.quantity {
                    anyhow::bail!(
                        "PC item storage does not contain {} x{}",
                        command.item_id,
                        command.quantity
                    );
                }
                let added = state
                    .bag
                    .add_item(item, command.quantity)
                    .map_err(|error| anyhow::anyhow!("add bag item from PC: {error}"))?;
                if !added {
                    anyhow::bail!("bag rejected {} x{}", command.item_id, command.quantity);
                }
                state
                    .bag
                    .remove_pc_item(item, command.quantity)
                    .map_err(|error| anyhow::anyhow!("remove PC item: {error}"))?;
                RuntimeMutationResult::PcItemWithdrawnToBag(RuntimePcItemTransferOutcome {
                    item_id: command.item_id,
                    quantity: command.quantity,
                    bag_quantity_after: state.bag.quantity(item),
                    pc_quantity_after: state.bag.pc_item_quantity(item),
                })
            }
            RuntimeMutationCommand::TossPcItem(command) => {
                let item = self.items.get(&command.item_id)
                    .with_context(|| format!("unknown PC item {}", command.item_id))?;
                if item
                    .property
                    .split('|')
                    .any(|flag| flag.trim() == "CANT_TOSS")
                {
                    anyhow::bail!("PC item {} is too important to toss", command.item_id);
                }
                let removed = state.bag.remove_pc_item(item, command.quantity)
                    .map_err(|error| anyhow::anyhow!("toss PC item: {error}"))?;
                if !removed {
                    anyhow::bail!("PC does not contain {} x{}", command.item_id, command.quantity);
                }
                RuntimeMutationResult::PcItemTossed(RuntimePcItemTransferOutcome {
                    item_id: command.item_id,
                    quantity: command.quantity,
                    bag_quantity_after: state.bag.quantity(item),
                    pc_quantity_after: state.bag.pc_item_quantity(item),
                })
            }
            RuntimeMutationCommand::GiveBagItemToPartyPokemon(command) => {
                let mut staged_state = state.clone();
                let item = self
                    .items
                    .get(&command.item_id)
                    .with_context(|| format!("unknown item {}", command.item_id))?;
                if crystal_core::models::item::is_mail_item_id(&command.item_id) {
                    anyhow::bail!(
                        "Mail item {} requires the compose-Mail action",
                        command.item_id
                    );
                }
                let target = staged_state
                    .storage
                    .party
                    .pokemon
                    .get(command.party_index)
                    .with_context(|| {
                        format!("party index {} is outside party", command.party_index)
                    })?
                    .as_ref()
                    .with_context(|| {
                        format!(
                            "party index {} has no Pokemon for held item",
                            command.party_index
                        )
                    })?;
                if target.mail.is_some() {
                    anyhow::bail!(
                        "party index {} must remove MAIL before replacing its held item",
                        command.party_index
                    );
                }
                let previous_item_id = target.item.clone();
                if let Some(previous_item_id) = previous_item_id.as_deref() {
                    let previous_item = self
                        .items
                        .get(previous_item_id)
                        .with_context(|| format!("unknown held item {previous_item_id}"))?;
                    let added = staged_state
                        .bag
                        .add_item(previous_item, 1)
                        .map_err(|error| anyhow::anyhow!("return replaced held item to bag: {error}"))?;
                    if !added {
                        anyhow::bail!("bag rejected replaced held item {previous_item_id}");
                    }
                }
                let removed = staged_state
                    .bag
                    .remove_item(item, 1)
                    .map_err(|error| anyhow::anyhow!("remove held item from bag: {error}"))?;
                if !removed {
                    anyhow::bail!("bag does not contain held item {}", command.item_id);
                }
                let pokemon = staged_state.storage.party.pokemon[command.party_index]
                    .as_mut()
                    .context("validated party Pokemon disappeared during held-item transfer")?;
                pokemon.item = Some(command.item_id.clone());
                let bag_quantity_after = staged_state.bag.quantity(item);
                staged_state.sync_party_from_storage();
                *state = staged_state;
                RuntimeMutationResult::PartyPokemonHeldItemGiven(RuntimeHeldItemTransferOutcome {
                    party_index: command.party_index,
                    item_id: command.item_id,
                    bag_quantity_after,
                })
            }
            RuntimeMutationCommand::TakeHeldItemFromPartyPokemon(command) => {
                let pokemon = state
                    .storage
                    .party
                    .pokemon
                    .get_mut(command.party_index)
                    .with_context(|| {
                        format!("party index {} is outside party", command.party_index)
                    })?
                    .as_mut()
                    .with_context(|| {
                        format!(
                            "party index {} has no Pokemon for held item",
                            command.party_index
                        )
                    })?;
                if pokemon.mail.is_some() {
                    anyhow::bail!(
                        "party index {} must use the MAIL action before removing its held item",
                        command.party_index
                    );
                }
                let item_id = pokemon.item.clone().with_context(|| {
                    format!("party index {} holds no item", command.party_index)
                })?;
                let item = self
                    .items
                    .get(&item_id)
                    .with_context(|| format!("unknown held item {item_id}"))?;
                let added = state
                    .bag
                    .add_item(item, 1)
                    .map_err(|error| anyhow::anyhow!("return held item to bag: {error}"))?;
                if !added {
                    anyhow::bail!("bag rejected held item {item_id}");
                }
                pokemon.item = None;
                RuntimeMutationResult::PartyPokemonHeldItemTaken(RuntimeHeldItemTransferOutcome {
                    party_index: command.party_index,
                    item_id,
                    bag_quantity_after: state.bag.quantity(item),
                })
            }
            RuntimeMutationCommand::SendPartyMailToMailbox(command) => {
                if state.mailbox.len() >= crystal_core::state::MAILBOX_CAPACITY {
                    anyhow::bail!("mailbox is full");
                }
                let mut staged_state = state.clone();
                let pokemon = staged_state
                    .storage
                    .party
                    .pokemon
                    .get_mut(command.party_index)
                    .with_context(|| format!("party index {} is outside party", command.party_index))?
                    .as_mut()
                    .with_context(|| format!("party index {} has no Pokemon", command.party_index))?;
                let item_id = pokemon.item.take().with_context(|| {
                    format!("party index {} holds no Mail item", command.party_index)
                })?;
                let mail = pokemon.mail.take().with_context(|| {
                    format!("party index {} has no Mail message", command.party_index)
                })?;
                let mailbox_index = staged_state.mailbox.len();
                staged_state.mailbox.push(crystal_core::state::MailboxMail {
                    item_id: item_id.clone(),
                    mail: mail.clone(),
                });
                staged_state.sync_party_from_storage();
                *state = staged_state;
                RuntimeMutationResult::PartyMailSentToMailbox(RuntimeMailTransferOutcome {
                    party_index: Some(command.party_index),
                    mailbox_index: Some(mailbox_index),
                    item_id,
                    mail,
                    mailbox_count_after: state.mailbox.len(),
                    bag_quantity_after: 0,
                })
            }
            RuntimeMutationCommand::DiscardPartyMailToBag(command) => {
                let mut staged_state = state.clone();
                let pokemon = staged_state
                    .storage
                    .party
                    .pokemon
                    .get_mut(command.party_index)
                    .with_context(|| format!("party index {} is outside party", command.party_index))?
                    .as_mut()
                    .with_context(|| format!("party index {} has no Pokemon", command.party_index))?;
                let item_id = pokemon.item.clone().with_context(|| {
                    format!("party index {} holds no Mail item", command.party_index)
                })?;
                let mail = pokemon.mail.clone().with_context(|| {
                    format!("party index {} has no Mail message", command.party_index)
                })?;
                let item = self.items.get(&item_id).with_context(|| format!("unknown Mail item {item_id}"))?;
                let added = staged_state
                    .bag
                    .add_item(item, 1)
                    .map_err(|error| anyhow::anyhow!("return Mail item to bag: {error}"))?;
                if !added {
                    anyhow::bail!("bag rejected Mail item {item_id}");
                }
                let pokemon = staged_state.storage.party.pokemon[command.party_index]
                    .as_mut()
                    .context("validated party Pokemon disappeared during Mail transfer")?;
                pokemon.item = None;
                pokemon.mail = None;
                let bag_quantity_after = staged_state.bag.quantity(item);
                staged_state.sync_party_from_storage();
                *state = staged_state;
                RuntimeMutationResult::PartyMailDiscardedToBag(RuntimeMailTransferOutcome {
                    party_index: Some(command.party_index),
                    mailbox_index: None,
                    item_id,
                    mail,
                    mailbox_count_after: state.mailbox.len(),
                    bag_quantity_after,
                })
            }
            RuntimeMutationCommand::DeleteMailboxMail(command) => {
                if command.mailbox_index >= state.mailbox.len() {
                    anyhow::bail!("mailbox index {} is outside mailbox", command.mailbox_index);
                }
                let removed = state.mailbox.remove(command.mailbox_index);
                RuntimeMutationResult::MailboxMailDeleted(RuntimeMailTransferOutcome {
                    party_index: None,
                    mailbox_index: Some(command.mailbox_index),
                    item_id: removed.item_id,
                    mail: removed.mail,
                    mailbox_count_after: state.mailbox.len(),
                    bag_quantity_after: 0,
                })
            }
            RuntimeMutationCommand::MoveMailboxMailToBag(command) => {
                let mut staged_state = state.clone();
                let entry = staged_state.mailbox.get(command.mailbox_index)
                    .with_context(|| format!("mailbox index {} is outside mailbox", command.mailbox_index))?
                    .clone();
                let item = self.items.get(&entry.item_id)
                    .with_context(|| format!("unknown Mail item {}", entry.item_id))?;
                let added = staged_state.bag.add_item(item, 1)
                    .map_err(|error| anyhow::anyhow!("put Mail item in bag: {error}"))?;
                if !added {
                    anyhow::bail!("bag rejected Mail item {}", entry.item_id);
                }
                staged_state.mailbox.remove(command.mailbox_index);
                let bag_quantity_after = staged_state.bag.quantity(item);
                *state = staged_state;
                RuntimeMutationResult::MailboxMailMovedToBag(RuntimeMailTransferOutcome {
                    party_index: None,
                    mailbox_index: Some(command.mailbox_index),
                    item_id: entry.item_id,
                    mail: entry.mail,
                    mailbox_count_after: state.mailbox.len(),
                    bag_quantity_after,
                })
            }
            RuntimeMutationCommand::AttachMailboxMailToParty(command) => {
                let mut staged_state = state.clone();
                let entry = staged_state.mailbox.get(command.mailbox_index)
                    .with_context(|| format!("mailbox index {} is outside mailbox", command.mailbox_index))?
                    .clone();
                let pokemon = staged_state.storage.party.pokemon
                    .get_mut(command.party_index)
                    .with_context(|| format!("party index {} is outside party", command.party_index))?
                    .as_mut()
                    .with_context(|| format!("party index {} has no Pokemon", command.party_index))?;
                if pokemon.is_egg {
                    anyhow::bail!("Mail cannot be attached to an Egg");
                }
                if pokemon.item.is_some() {
                    anyhow::bail!("party Pokemon is already holding an item");
                }
                pokemon.item = Some(entry.item_id.clone());
                pokemon.mail = Some(entry.mail.clone());
                staged_state.mailbox.remove(command.mailbox_index);
                staged_state.sync_party_from_storage();
                *state = staged_state;
                RuntimeMutationResult::MailboxMailAttachedToParty(RuntimeMailTransferOutcome {
                    party_index: Some(command.party_index),
                    mailbox_index: Some(command.mailbox_index),
                    item_id: entry.item_id,
                    mail: entry.mail,
                    mailbox_count_after: state.mailbox.len(),
                    bag_quantity_after: 0,
                })
            }
            RuntimeMutationCommand::AwardBadge(command) => {
                let badges = match command.region {
                    RuntimeBadgeRegion::Johto => &mut state.badges.johto,
                    RuntimeBadgeRegion::Kanto => &mut state.badges.kanto,
                };
                let slot = badges
                    .get_mut(command.index)
                    .with_context(|| format!("badge index {} is outside region", command.index))?;
                let already_awarded = *slot;
                *slot = true;
                RuntimeMutationResult::BadgeAwarded(RuntimeBadgeAwardOutcome {
                    region: command.region,
                    index: command.index,
                    already_awarded,
                    awarded_count_after: badges.iter().filter(|awarded| **awarded).count(),
                })
            }
            RuntimeMutationCommand::RecordPokedexSeen(command) => {
                let species = self
                    .pokemon
                    .get(&command.species_id)
                    .with_context(|| format!("unknown Pokemon species {}", command.species_id))?;
                let already_seen = state.pokedex.has_seen(&command.species_id);
                let already_caught = state.pokedex.has_caught(&command.species_id);
                state.pokedex.record_seen(species);
                RuntimeMutationResult::PokedexSeenRecorded(RuntimePokedexRecordOutcome {
                    species_id: command.species_id,
                    already_seen,
                    already_caught,
                    seen_count_after: state.pokedex.seen_count(),
                    caught_count_after: state.pokedex.caught_count(),
                })
            }
            RuntimeMutationCommand::RecordPokedexCaught(command) => {
                let species = self
                    .pokemon
                    .get(&command.species_id)
                    .with_context(|| format!("unknown Pokemon species {}", command.species_id))?;
                let already_seen = state.pokedex.has_seen(&command.species_id);
                let already_caught = state.pokedex.has_caught(&command.species_id);
                state.pokedex.record_caught(species);
                RuntimeMutationResult::PokedexCaughtRecorded(RuntimePokedexRecordOutcome {
                    species_id: command.species_id,
                    already_seen,
                    already_caught,
                    seen_count_after: state.pokedex.seen_count(),
                    caught_count_after: state.pokedex.caught_count(),
                })
            }
            RuntimeMutationCommand::AddBagItem(command) => {
                let item = self
                    .items
                    .get(&command.item_id)
                    .with_context(|| format!("unknown item {}", command.item_id))?;
                if item.script_name != command.item_id {
                    anyhow::bail!(
                        "compiled item {} has script_name {}, expected exact id match",
                        command.item_id,
                        item.script_name
                    );
                }
                let quantity_before = state.bag.quantity(item);
                let added = state
                    .bag
                    .add_item(item, command.quantity)
                    .map_err(anyhow::Error::msg)?;
                let quantity_after = state.bag.quantity(item);
                RuntimeMutationResult::BagItemAdded(RuntimeBagItemMutationOutcome {
                    item_id: command.item_id,
                    quantity: command.quantity,
                    added,
                    quantity_before,
                    quantity_after,
                })
            }
            RuntimeMutationCommand::AddCurrency(command) => {
                let cap = runtime_currency_cap(&self.currency_constants, command.account)?;
                let before = match command.account {
                    RuntimeCurrencyAccount::Money => state.money,
                    RuntimeCurrencyAccount::Coins => u32::from(state.coins),
                };
                let after = before.saturating_add(command.amount).min(cap);
                match command.account {
                    RuntimeCurrencyAccount::Money => state.money = after,
                    RuntimeCurrencyAccount::Coins => {
                        state.coins = u16::try_from(after)
                            .context("MAX_COINS cannot fit saved coin storage")?;
                    }
                }
                RuntimeMutationResult::CurrencyAdded(RuntimeCurrencyMutationOutcome {
                    account: command.account,
                    amount: command.amount,
                    value_before: before,
                    value_after: after,
                    cap,
                })
            }
            RuntimeMutationCommand::TakeCurrency(command) => {
                let cap = runtime_currency_cap(&self.currency_constants, command.account)?;
                let before = match command.account {
                    RuntimeCurrencyAccount::Money => state.money,
                    RuntimeCurrencyAccount::Coins => u32::from(state.coins),
                };
                let after = before.saturating_sub(command.amount);
                match command.account {
                    RuntimeCurrencyAccount::Money => state.money = after,
                    RuntimeCurrencyAccount::Coins => {
                        state.coins = u16::try_from(after)
                            .context("coin value cannot fit saved coin storage")?;
                    }
                }
                RuntimeMutationResult::CurrencyTaken(RuntimeCurrencyMutationOutcome {
                    account: command.account,
                    amount: command.amount,
                    value_before: before,
                    value_after: after,
                    cap,
                })
            }
            RuntimeMutationCommand::RecordLinkBattleResult(command) => {
                match command.result {
                    RuntimeLinkBattleResult::Win => {
                        state.link_battle_stats.wins =
                            state.link_battle_stats.wins.saturating_add(1);
                    }
                    RuntimeLinkBattleResult::Loss => {
                        state.link_battle_stats.losses =
                            state.link_battle_stats.losses.saturating_add(1);
                    }
                    RuntimeLinkBattleResult::Draw => {
                        state.link_battle_stats.draws =
                            state.link_battle_stats.draws.saturating_add(1);
                    }
                }
                RuntimeMutationResult::LinkBattleResultRecorded(RuntimeLinkBattleRecordOutcome {
                    result: command.result,
                    wins_after: state.link_battle_stats.wins,
                    losses_after: state.link_battle_stats.losses,
                    draws_after: state.link_battle_stats.draws,
                })
            }
            RuntimeMutationCommand::SetOptions(command) => {
                let before = state.options.clone();
                state.options = command.options;
                RuntimeMutationResult::OptionsSet(RuntimeOptionsSetOutcome {
                    options_before: before,
                    options_after: state.options.clone(),
                })
            }
            RuntimeMutationCommand::SetTrainerIdentity(command) => {
                let before_name = state.player_name.clone();
                let before_id = state.player_id;
                state.player_name = command.player_name;
                state.player_id = command.player_id;
                RuntimeMutationResult::TrainerIdentitySet(RuntimeTrainerIdentityOutcome {
                    player_name_before: before_name,
                    player_id_before: before_id,
                    player_name_after: state.player_name.clone(),
                    player_id_after: state.player_id,
                })
            }
            RuntimeMutationCommand::SetPlayerGender(command) => {
                validate_saved_player_gender(command.player_gender).map_err(anyhow::Error::msg)?;
                let before = state.player_gender;
                state.player_gender = command.player_gender;
                RuntimeMutationResult::PlayerGenderSet(RuntimePlayerGenderOutcome {
                    player_gender_before: before,
                    player_gender_after: state.player_gender,
                })
            }
            RuntimeMutationCommand::RenamePartyPokemon(command) => {
                let pokemon = state
                    .storage
                    .party
                    .pokemon
                    .get_mut(command.party_index)
                    .with_context(|| {
                        format!("party index {} is outside party", command.party_index)
                    })?
                    .as_mut()
                    .with_context(|| {
                        format!(
                            "party index {} has no Pokemon to rename",
                            command.party_index
                        )
                    })?;
                let before = pokemon.nickname.clone();
                pokemon.nickname = command.nickname;
                let species_id = pokemon.species.id.clone();
                let nickname_after = pokemon.nickname.clone();
                state.sync_party_from_storage();
                RuntimeMutationResult::PartyPokemonRenamed(RuntimePartyNicknameOutcome {
                    party_index: command.party_index,
                    species_id,
                    nickname_before: before,
                    nickname_after,
                })
            }
            RuntimeMutationCommand::SetPartyPokemonRecoveryState(command) => {
                if let Some(status) = command.status.as_deref() {
                    self.validate_saved_pokemon_status_reference(
                        "runtime.party_recovery_setup.status",
                        status,
                    )?;
                }
                let pokemon = state
                    .storage
                    .party
                    .pokemon
                    .get_mut(command.party_index)
                    .with_context(|| {
                        format!("party index {} is outside party", command.party_index)
                    })?
                    .as_mut()
                    .with_context(|| {
                        format!(
                            "party index {} has no Pokemon to set recovery state",
                            command.party_index
                        )
                    })?;
                let hp_before = pokemon.hp;
                let status_before = pokemon.status.clone();
                let first_move = pokemon.moves.first().map(|learned| learned.name.clone());
                let first_move_pp_before = pokemon.moves.first().map(|learned| learned.current_pp);
                pokemon.hp = command.hp.min(pokemon.max_hp);
                pokemon.status = command.status;
                if let (Some(learned), Some(pp)) =
                    (pokemon.moves.first_mut(), command.first_move_pp)
                {
                    learned.current_pp = pp;
                }
                let species_id = pokemon.species.id.clone();
                let hp_after = pokemon.hp;
                let status_after = pokemon.status.clone();
                let first_move_pp_after = pokemon.moves.first().map(|learned| learned.current_pp);
                state.sync_party_from_storage();
                RuntimeMutationResult::PartyPokemonRecoveryStateSet(
                    RuntimePartyRecoverySetupOutcome {
                        party_index: command.party_index,
                        species_id,
                        hp_before,
                        hp_after,
                        status_before,
                        status_after,
                        first_move,
                        first_move_pp_before,
                        first_move_pp_after,
                    },
                )
            }
            RuntimeMutationCommand::TransferPartyPokemonHp(command) => {
                if command.source_party_index == command.target_party_index {
                    anyhow::bail!("party HP transfer source and target must differ");
                }
                let party_len = state.storage.party.pokemon.len();
                if command.source_party_index >= party_len
                    || command.target_party_index >= party_len
                {
                    anyhow::bail!(
                        "party HP transfer indexes {} and {} must be inside party length {}",
                        command.source_party_index,
                        command.target_party_index,
                        party_len
                    );
                }
                let (source, target) = if command.source_party_index < command.target_party_index {
                    let (left, right) = state
                        .storage
                        .party
                        .pokemon
                        .split_at_mut(command.target_party_index);
                    (
                        left[command.source_party_index]
                            .as_mut()
                            .context("party HP transfer source slot is empty")?,
                        right[0]
                            .as_mut()
                            .context("party HP transfer target slot is empty")?,
                    )
                } else {
                    let (left, right) = state
                        .storage
                        .party
                        .pokemon
                        .split_at_mut(command.source_party_index);
                    (
                        right[0]
                            .as_mut()
                            .context("party HP transfer source slot is empty")?,
                        left[command.target_party_index]
                            .as_mut()
                            .context("party HP transfer target slot is empty")?,
                    )
                };
                let source_is_egg = source.is_egg
                    || source.species.id == "EGG";
                let target_is_egg = target.is_egg
                    || target.species.id == "EGG";
                if source_is_egg || target_is_egg {
                    anyhow::bail!("party HP transfer cannot use an Egg");
                }
                let amount = source.max_hp / 5;
                if source.hp < amount {
                    anyhow::bail!("party HP transfer source does not have enough HP");
                }
                if target.hp == 0 || target.hp >= target.max_hp {
                    anyhow::bail!("party HP transfer target cannot receive HP");
                }
                let source_hp_before = source.hp;
                let target_hp_before = target.hp;
                source.hp -= amount;
                target.hp = target.hp.saturating_add(amount).min(target.max_hp);
                let source_hp_after = source.hp;
                let target_hp_after = target.hp;
                state.sync_party_from_storage();
                RuntimeMutationResult::PartyPokemonHpTransferred(
                    RuntimePartyHpTransferOutcome {
                        source_party_index: command.source_party_index,
                        target_party_index: command.target_party_index,
                        amount,
                        source_hp_before,
                        source_hp_after,
                        target_hp_before,
                        target_hp_after,
                    },
                )
            }
            RuntimeMutationCommand::SwapPartyPokemon(command) => {
                if command.first_party_index >= state.storage.party.pokemon.len()
                    || command.second_party_index >= state.storage.party.pokemon.len()
                {
                    anyhow::bail!(
                        "party swap indexes {} and {} must be inside party",
                        command.first_party_index,
                        command.second_party_index
                    );
                }
                if state.storage.party.pokemon[command.first_party_index].is_none()
                    || state.storage.party.pokemon[command.second_party_index].is_none()
                {
                    anyhow::bail!(
                        "party swap indexes {} and {} must both contain Pokemon",
                        command.first_party_index,
                        command.second_party_index
                    );
                }
                state
                    .storage
                    .party
                    .pokemon
                    .swap(command.first_party_index, command.second_party_index);
                state.sync_party_from_storage();
                let first_species_after = state.storage.party.pokemon[command.first_party_index]
                    .as_ref()
                    .map(|pokemon| pokemon.species.id.clone())
                    .with_context(|| {
                        format!(
                            "party swap index {} unexpectedly empty after swap",
                            command.first_party_index
                        )
                    })?;
                let second_species_after = state.storage.party.pokemon[command.second_party_index]
                    .as_ref()
                    .map(|pokemon| pokemon.species.id.clone())
                    .with_context(|| {
                        format!(
                            "party swap index {} unexpectedly empty after swap",
                            command.second_party_index
                        )
                    })?;
                RuntimeMutationResult::PartyPokemonSwapped(RuntimePartySwapOutcome {
                    first_party_index: command.first_party_index,
                    second_party_index: command.second_party_index,
                    first_species_after,
                    second_species_after,
                })
            }
            RuntimeMutationCommand::SwapPartyPokemonMoves(command) => {
                if let Some(transform) = state
                    .script_runtime
                    .active_battle_combat
                    .as_mut()
                    .filter(|combat| combat.player_party_index == command.party_index)
                    .and_then(|combat| combat.player_transform.as_mut())
                {
                    if command.first_move_index >= transform.moves.len()
                        || command.second_move_index >= transform.moves.len()
                    {
                        anyhow::bail!(
                            "move swap indexes {} and {} must be inside transformed active Pokemon moves",
                            command.first_move_index,
                            command.second_move_index
                        );
                    }
                    transform
                        .moves
                        .swap(command.first_move_index, command.second_move_index);
                    RuntimeMutationResult::PartyPokemonMovesSwapped(RuntimePartyMoveSwapOutcome {
                        party_index: command.party_index,
                        first_move_index: command.first_move_index,
                        second_move_index: command.second_move_index,
                        first_move_after: transform.moves[command.first_move_index].name.clone(),
                        second_move_after: transform.moves[command.second_move_index].name.clone(),
                    })
                } else {
                let pokemon = state
                    .storage
                    .party
                    .pokemon
                    .get_mut(command.party_index)
                    .with_context(|| {
                        format!("party index {} is outside party", command.party_index)
                    })?
                    .as_mut()
                    .with_context(|| {
                        format!("party index {} has no Pokemon", command.party_index)
                    })?;
                if command.first_move_index >= pokemon.moves.len()
                    || command.second_move_index >= pokemon.moves.len()
                {
                    anyhow::bail!(
                        "move swap indexes {} and {} must be inside party Pokemon {} moves",
                        command.first_move_index,
                        command.second_move_index,
                        command.party_index
                    );
                }
                pokemon
                    .moves
                    .swap(command.first_move_index, command.second_move_index);
                let moves_after = pokemon.moves.clone();
                let first_move_after = moves_after[command.first_move_index].name.clone();
                let second_move_after = moves_after[command.second_move_index].name.clone();
                state.sync_party_from_storage();
                if let Some(combat) = state.script_runtime.active_battle_combat.as_mut() {
                    if let Some(party_pokemon) = combat.player_party.get_mut(command.party_index) {
                        party_pokemon.moves = moves_after.clone();
                    }
                    if combat.player_party_index == command.party_index {
                        combat.player.moves = moves_after;
                    }
                }
                RuntimeMutationResult::PartyPokemonMovesSwapped(RuntimePartyMoveSwapOutcome {
                    party_index: command.party_index,
                    first_move_index: command.first_move_index,
                    second_move_index: command.second_move_index,
                    first_move_after,
                    second_move_after,
                })
                }
            }
            RuntimeMutationCommand::FullHealPartyPokemon(command) => {
                let recovered = full_heal_party_slot(state, &self.moves, command.party_index)?;
                RuntimeMutationResult::PartyPokemonFullHealed(recovered)
            }
            RuntimeMutationCommand::FullHealWholeParty => {
                let mut recovered = Vec::new();
                for party_index in 0..state.storage.party.pokemon.len() {
                    if state.storage.party.pokemon[party_index]
                        .as_ref()
                        .is_some_and(|pokemon| {
                            !pokemon.is_egg
                                && pokemon.species.id != "EGG"
                        })
                    {
                        recovered.push(full_heal_party_slot(state, &self.moves, party_index)?);
                    }
                }
                RuntimeMutationResult::WholePartyFullHealed(recovered)
            }
            RuntimeMutationCommand::ResolveBlackoutToLastSpawn => {
                let heal_indexes = (0..state.storage.party.pokemon.len())
                    .filter(|party_index| {
                        state.storage.party.pokemon[*party_index]
                            .as_ref()
                            .is_some_and(|pokemon| {
                                !pokemon.is_egg
                                    && pokemon.species.id != "EGG"
                            })
                    })
                    .collect::<Vec<_>>();
                let mut healed = Vec::new();
                for party_index in heal_indexes {
                    healed.push(full_heal_party_slot(state, &self.moves, party_index)?);
                }
                let bug_contest_active = state
                    .flags
                    .is_engine_flag_set("ENGINE_BUG_CONTEST_TIMER")
                    .map_err(|error| anyhow::anyhow!("read Bug Contest timer flag: {error}"))?;
                let (spawn_identifier, map_name, tile) = if bug_contest_active {
                    let script = "BugContestResultsWarpScript";
                    let compiled_body = self.compiled_standard_script_body(script)?.to_vec();
                    apply_standard_script(state, &self.moves, script, &compiled_body)?;
                    (None, "Route36NationalParkGate".to_string(), TilePosition::new(0, 4))
                } else {
                    let outcome =
                        self.apply_special_routine(state, "WarpToSpawnPoint", music_ids)?;
                    let SpecialRoutineEffect::WarpToSpawnPoint {
                        spawn_identifier,
                        map_name,
                        tile,
                    } = outcome.effect
                    else {
                        anyhow::bail!("WarpToSpawnPoint returned unexpected effect");
                    };
                    state.money /= 2;
                    (Some(spawn_identifier), map_name, tile)
                };
                deactivate_battle(state);
                state.script_runtime.blackout_mod = None;
                RuntimeMutationResult::BlackoutResolved(BlackoutRecoveryOutcome {
                    spawn_identifier,
                    map_name,
                    tile,
                    healed,
                })
            }
            RuntimeMutationCommand::InitializePermanentPhoneNumbers => {
                RuntimeMutationResult::PermanentPhoneNumbersInitialized(
                    self.initialize_permanent_phone_numbers(state)?,
                )
            }
        };
        session.set_time_of_day(state.time.time_of_day);
        session.sync_event_flag_memory(&state.flags);
        let result_tag = result.result_tag();
        // The loaded state is validated at runtime-shell construction and
        // mutation boundaries are typed. Avoid re-walking the complete save
        // graph on every 60 Hz frame; retain the exact serialized checksum.
        let state_checksum = if compute_checksum {
            game_state_checksum_unchecked(state)
                .with_context(|| format!("checksum runtime mutation {result_tag}"))?
        } else {
            StateChecksum::new(state.frame_counter, 0)
        };
        Ok(RuntimeMutationOutcome {
            result,
            state_checksum,
        })
    }

    pub fn buy_shop_item(
        &self,
        state: &mut GameState,
        item_id: &str,
        quantity: u16,
    ) -> Result<ShopResult> {
        core_buy_active_shop_item(state, &self.items, item_id, quantity)
            .map_err(|error| anyhow::anyhow!("buy shop item {item_id}: {error:?}"))
    }

    pub fn sell_shop_item(
        &self,
        state: &mut GameState,
        item_id: &str,
        quantity: u16,
    ) -> Result<ShopResult> {
        core_sell_active_shop_item(
            state,
            &self.items,
            &self.currency_constants,
            item_id,
            quantity,
        )
        .map_err(|error| anyhow::anyhow!("sell shop item {item_id}: {error:?}"))
    }

    pub fn script_movement(
        &self,
        map_name: &str,
        source_script: &str,
        movement_label: &str,
    ) -> Result<&ScriptMovement> {
        self.map_module(map_name)?
            .script_movements
            .iter()
            .find(|movement| {
                movement.label == movement_label
                    && movement.source_script.as_deref() == Some(source_script)
            })
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "map {map_name} has no exact movement {movement_label} for {source_script}"
                )
            })
    }

    pub fn map_ids(&self) -> BTreeSet<String> {
        self.maps.keys().cloned().collect()
    }

    pub fn map_music(&self, map_name: &str) -> Result<Option<&str>> {
        Ok(self.map_module(map_name)?.attributes.music.as_deref())
    }

    pub fn checked_map_music(
        &self,
        map_name: &str,
        music_ids: &BTreeSet<String>,
    ) -> Result<Option<String>> {
        let Some(music) = self.map_music(map_name)? else {
            return Ok(None);
        };
        if music_ids.is_empty() {
            return Ok(Some(music.to_string()));
        }
        core_validate_saved_audio_reference(
            &format!("maps.{map_name}.attributes.music"),
            music,
            ModpackAudioKind::Music.save_name(),
            music_ids
                .contains(music)
                .then_some(ModpackAudioKind::Music.save_name()),
        )
        .map_err(|error| anyhow::anyhow!("{error}"))?;
        Ok(Some(music.to_owned()))
    }

    pub fn sync_current_map_music(
        &self,
        state: &mut GameState,
        map_name: &str,
        mode: MovementMode,
        music_ids: &BTreeSet<String>,
    ) -> Result<()> {
        let music = match mode {
            MovementMode::Bike => Some("MUSIC_BICYCLE".to_string()),
            MovementMode::Surf | MovementMode::SurfPika => Some("MUSIC_SURF".to_string()),
            MovementMode::Normal | MovementMode::Skate => {
                self.checked_map_music(map_name, music_ids)?
            }
        };
        if !music_ids.is_empty() {
            if let Some(music_id) = music.as_deref() {
                core_validate_saved_audio_reference(
                    "overworld movement music",
                    music_id,
                    ModpackAudioKind::Music.save_name(),
                    music_ids
                        .contains(music_id)
                        .then_some(ModpackAudioKind::Music.save_name()),
                )
                .map_err(|error| anyhow::anyhow!("{error}"))?;
            }
        }
        apply_map_music_context(state, music);
        Ok(())
    }

    fn map_entry_movement_mode(
        &self,
        state: &GameState,
        session: &OverworldSession,
        previous: MovementMode,
    ) -> Result<MovementMode> {
        if state
            .flags
            .is_engine_flag_set("ENGINE_ALWAYS_ON_BIKE")
            .context("check ENGINE_ALWAYS_ON_BIKE during map entry")?
        {
            return Ok(MovementMode::Bike);
        }
        let sample = sample_collision(&session.map, &session.tileset, session.player.tile)
            .with_context(|| {
                format!(
                    "sample map-entry tile {},{} on {}",
                    session.player.tile.x, session.player.tile.y, session.map.name
                )
            })?;
        if describe_collision(sample.permission).terrain == Terrain::Water {
            return Ok(match previous {
                MovementMode::Surf | MovementMode::SurfPika => previous,
                MovementMode::Normal | MovementMode::Bike | MovementMode::Skate => {
                    MovementMode::Surf
                }
            });
        }
        Ok(match previous {
            MovementMode::Surf | MovementMode::SurfPika => MovementMode::Normal,
            MovementMode::Bike
                if !is_bicycle_environment(self.map_environment(&session.map.name)?) =>
            {
                MovementMode::Normal
            }
            MovementMode::Normal | MovementMode::Bike | MovementMode::Skate => previous,
        })
    }

    pub fn sync_current_map_scene(&self, state: &mut GameState, map_name: &str) -> Result<()> {
        let scenes = self.map_scene_table(map_name)?;
        apply_map_scene_context(state, map_name, scenes).map_err(|error| {
            anyhow::anyhow!("apply map scene context for {map_name}: {error:?}")
        })?;
        Ok(())
    }

    /// Run the data-driven object callbacks that Crystal executes whenever a
    /// map is entered.  These callbacks are part of the map script section,
    /// not NPC presentation data.  In particular, ElmsLab places Elm at his
    /// temporary intro position until the first scene advances.
    pub fn apply_map_object_callbacks(
        &self,
        state: &mut GameState,
        session: &mut OverworldSession,
        map_name: &str,
    ) -> Result<()> {
        self.require_current_map(&session.map.name, map_name)?;
        let module = self.map_module(map_name)?;
        if !module.scenes.scenes.is_empty() {
            state
                .scenes
                .check_scene(map_name, &module.scenes)
                .map_err(|error| {
                    anyhow::anyhow!("check map callback scene for {map_name}: {error:?}")
                })?;
        }
        let callback_names: Vec<(String, String)> = module
            .map_script_section_commands
            .iter()
            .filter(|command| {
                command.command == "callback"
                    && matches!(
                        command.args.first().map(String::as_str),
                        Some("MAPCALLBACK_NEWMAP")
                            | Some("MAPCALLBACK_TILES")
                            | Some("MAPCALLBACK_OBJECTS")
                            | Some("MAPCALLBACK_CMDQUEUE")
                    )
            })
            .filter_map(|command| {
                command
                    .args
                    .get(1)
                    .cloned()
                    .map(|name| (command.args[0].clone(), name))
            })
            .collect();
        let callback_bodies: Vec<(String, String, Vec<Value>)> = callback_names
            .into_iter()
            .filter_map(|(kind, name)| {
                module
                    .scripts
                    .get(&name)
                    .and_then(Value::as_array)
                    .map(|body| (kind, name, body.clone()))
            })
            .collect();
        for (_callback_kind, callback_name, body) in callback_bodies {
            self.execute_map_object_callback_script(
                state,
                session,
                map_name,
                &callback_name,
                &body,
            )?;
        }
        sync_state_object_overrides(state, session)
            .context("sync map object callback overrides")?;
        Ok(())
    }

    fn execute_map_object_callback_script(
        &self,
        state: &mut GameState,
        session: &mut OverworldSession,
        map_name: &str,
        callback_name: &str,
        body: &[Value],
    ) -> Result<()> {
        let mut script_name = callback_name.to_string();
        let mut command_index = 0usize;
        for _ in 0..1024 {
            let module = self.map_module(map_name)?;
            let script_body: &[Value] = if script_name == callback_name {
                body
            } else if let Some(target) = module.scripts.get(&script_name).and_then(Value::as_array)
            {
                target
            } else {
                return Ok(());
            };
            let Some(entry) = script_body.get(command_index) else {
                return Ok(());
            };
            let command = entry
                .get("command")
                .and_then(Value::as_str)
                .with_context(|| {
                    format!("callback {callback_name} command {command_index} has no command")
                })?;
            let source = script_name.as_str();
            match command {
                "checkevent" | "checkflag" | "setevent" | "clearevent" | "setflag"
                | "clearflag" => {
                    if command.starts_with("check") {
                        self.check_script_flag_in_session(
                            state,
                            session,
                            map_name,
                            source,
                            command_index,
                        )?;
                    } else {
                        self.apply_script_flag_mutation_in_session(
                            state,
                            session,
                            map_name,
                            source,
                            command_index,
                        )?;
                    }
                    command_index += 1;
                }
                "checkitem" => {
                    let outcome = self.check_script_item_in_session(
                        state,
                        session,
                        map_name,
                        source,
                        command_index,
                    )?;
                    state.script_runtime.script_value =
                        Some(if outcome.held { "1" } else { "0" }.to_string());
                    command_index += 1;
                }
                "checkscene" | "setscene" => {
                    self.apply_script_scene_command_in_session(
                        state,
                        session,
                        map_name,
                        source,
                        command_index,
                    )?;
                    command_index += 1;
                }
                "setmapscene" => {
                    self.apply_script_scene_command_in_session(
                        state,
                        session,
                        map_name,
                        source,
                        command_index,
                    )?;
                    command_index += 1;
                }
                "changeblock" => {
                    self.apply_script_block_change_in_session(
                        state,
                        session,
                        map_name,
                        source,
                        command_index,
                    )?;
                    command_index += 1;
                }
                "readvar" | "setval" | "writemem" => {
                    self.apply_script_variable_command_now_in_session(
                        state,
                        session,
                        map_name,
                        source,
                        command_index,
                    )?;
                    command_index += 1;
                }
                "checktime" => {
                    self.apply_script_variable_command_now_in_session(
                        state,
                        session,
                        map_name,
                        source,
                        command_index,
                    )?;
                    command_index += 1;
                }
                "special" => {
                    let routine = entry
                        .get("args")
                        .and_then(Value::as_array)
                        .and_then(|args| args.first())
                        .and_then(Value::as_str)
                        .with_context(|| {
                            format!("callback {callback_name} special has no routine")
                        })?;
                    self.apply_special_routine(state, routine, &BTreeSet::new())?;
                    command_index += 1;
                }
                "specialphonecall" => {
                    self.apply_script_phone_command_in_session(
                        state,
                        session,
                        map_name,
                        source,
                        command_index,
                        ScriptPhoneInputs::default(),
                    )?;
                    command_index += 1;
                }
                "cmdqueue" | "writecmdqueue" | "stonetable" => {
                    self.apply_script_runtime_command_in_session(
                        state,
                        session,
                        map_name,
                        source,
                        command_index,
                        ScriptRuntimeInputs::default(),
                    )?;
                    command_index += 1;
                }
                "jumpstd" => {
                    let target = entry
                        .get("args")
                        .and_then(Value::as_array)
                        .and_then(|args| args.first())
                        .and_then(Value::as_str)
                        .with_context(|| {
                            format!(
                                "callback {callback_name} jumpstd has no standard-script target"
                            )
                        })?;
                    if target == "InitializeEventsScript" {
                        apply_initialize_events(state, &self.initialize_events).map_err(
                            |error| {
                                anyhow::anyhow!(
                                    "initialize events from callback {callback_name}: {error}"
                                )
                            },
                        )?;
                        command_index += 1;
                    } else {
                        anyhow::bail!(
                            "callback {callback_name} uses unsupported standard script {target}"
                        );
                    }
                }
                "readmem" => {
                    self.apply_script_variable_command_now_in_session(
                        state,
                        session,
                        map_name,
                        source,
                        command_index,
                    )?;
                    command_index += 1;
                }
                "appear" | "disappear" | "moveobject" | "turnobject" | "faceobject"
                | "faceplayer" | "follow" | "stopfollow" | "showemote" => {
                    let object = self
                        .script_object_command(map_name, source, command_index)?
                        .clone();
                    core_apply_script_object_mutation(state, session, &object).map_err(
                        |error| anyhow::anyhow!("apply map object callback {map_name}: {error:?}"),
                    )?;
                    command_index += 1;
                }
                "iftrue" | "iffalse" | "ifequal" | "ifnotequal" | "ifgreater" | "ifless"
                | "sjump" | "jump" | "scall" | "sdefer" | "endcallback" | "end" => {
                    let action = self.apply_script_control_command_in_session(
                        state,
                        session,
                        map_name,
                        source,
                        command_index,
                    )?;
                    match action {
                        ScriptControlAction::Continue { .. } => command_index += 1,
                        ScriptControlAction::Jump { target_script, .. } => {
                            script_name = target_script;
                            command_index = 0;
                        }
                        ScriptControlAction::End { .. } => return Ok(()),
                    }
                }
                _ => {
                    // Callbacks may contain unrelated bookkeeping commands;
                    // leave those to the normal script runtime rather than
                    // guessing at their side effects.
                    command_index += 1;
                }
            }
        }
        anyhow::bail!("map object callback {callback_name} exceeded execution limit")
    }

    pub fn overworld_session(
        &self,
        map_name: &str,
        player_tile: TilePosition,
        frame: u64,
    ) -> Result<OverworldSession> {
        self.overworld_session_for_traversal(
            map_name,
            player_tile,
            frame,
            PlayerTraversalState::Walk,
        )
    }

    pub fn overworld_session_for_traversal(
        &self,
        map_name: &str,
        player_tile: TilePosition,
        frame: u64,
        traversal_state: PlayerTraversalState,
    ) -> Result<OverworldSession> {
        let module = self.map_module(map_name)?;
        let tileset = self.tileset_collision(&module.attributes.tileset_name)?;
        validate_runtime_overworld_map_blocks(map_name, module)?;
        let map =
            OverworldMapData::from_attributes(map_name, &module.attributes, module.blocks.clone());
        let (width, height) = map.checked_tile_bounds().with_context(|| {
            format!(
                "compiled map {map_name} runtime tile bounds overflow supported coordinate range"
            )
        })?;
        if player_tile.x < 0
            || player_tile.y < 0
            || i32::from(player_tile.x) >= i32::from(width)
            || i32::from(player_tile.y) >= i32::from(height)
        {
            anyhow::bail!(
                "runtime player tile ({}, {}) is outside compiled map {map_name} runtime tile bounds {width}x{height}",
                player_tile.x,
                player_tile.y
            );
        }
        if !Self::can_occupy_runtime_tile(&map, &tileset, player_tile, traversal_state)
            && !self.runtime_tile_is_connection_source(module, player_tile)?
        {
            anyhow::bail!(
                "runtime player tile ({}, {}) is not walkable on compiled map {map_name}",
                player_tile.x,
                player_tile.y
            );
        }
        let mut session = OverworldSession::with_events_and_objects(
            map,
            module.events.clone(),
            module.objects.clone(),
            tileset,
            player_tile,
        );
        session.frame = frame;
        Ok(session)
    }

    fn can_occupy_runtime_tile(
        map: &OverworldMapData,
        tileset: &TilesetCollision,
        player_tile: TilePosition,
        traversal_state: PlayerTraversalState,
    ) -> bool {
        [
            Direction::Down,
            Direction::Up,
            Direction::Left,
            Direction::Right,
        ]
        .into_iter()
        .any(|facing| can_enter_tile(map, tileset, player_tile, facing, traversal_state))
    }

    fn runtime_tile_is_connection_source(
        &self,
        module: &MapModule,
        player_tile: TilePosition,
    ) -> Result<bool> {
        for connection in &module.attributes.connections {
            let Some(trigger_tile) = connection_trigger_tile_from_source(player_tile, connection)
            else {
                continue;
            };
            let target_module = self.map_module(&connection.target_map)?;
            if connection_destination_tile_in_bounds(
                trigger_tile,
                &connection.direction,
                connection.offset,
                &target_module.attributes,
            )? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub fn apply_saved_overworld_overrides(
        &self,
        session: &mut OverworldSession,
        state: &GameState,
    ) -> Result<()> {
        apply_state_block_overrides(session, state)?;
        apply_state_object_overrides(session, state)?;
        Ok(())
    }

    pub fn commit_overworld_snapshot(
        &self,
        state: &mut GameState,
        session: &OverworldSession,
        spawn_update: SpawnMemoryUpdate,
    ) {
        commit_overworld_snapshot(state, &session.snapshot(), spawn_update);
    }

    pub fn commit_overworld_snapshot_data(
        &self,
        state: &mut GameState,
        snapshot: &OverworldSnapshot,
        spawn_update: SpawnMemoryUpdate,
    ) {
        commit_overworld_snapshot(state, snapshot, spawn_update);
    }

    pub fn transition_overworld_session(
        &self,
        state: &mut GameState,
        session: &mut OverworldSession,
        destination_map: &str,
        destination_tile: TilePosition,
        spawn_update: SpawnMemoryUpdate,
        music_ids: &BTreeSet<String>,
    ) -> Result<()> {
        self.transition_overworld_session_with_mode(
            state,
            session,
            destination_map,
            destination_tile,
            MovementMode::Normal,
            spawn_update,
            music_ids,
        )
    }

    fn transition_overworld_session_with_mode(
        &self,
        state: &mut GameState,
        session: &mut OverworldSession,
        destination_map: &str,
        destination_tile: TilePosition,
        mode: MovementMode,
        spawn_update: SpawnMemoryUpdate,
        music_ids: &BTreeSet<String>,
    ) -> Result<()> {
        let frame = session.frame;
        *session = self.overworld_session_for_traversal(
            destination_map,
            destination_tile,
            frame,
            mode.traversal_state(),
        )?;
        session.player.mode = mode;
        clear_transient_map_object_context(state, session);
        reset_map_bike_flags(state)?;
        state.wild_encounter_cooldown = 5;
        let destination_environment = &self
            .runtime_map_metadata_for_name(destination_map)?
            .environment;
        if destination_environment.eq_ignore_ascii_case("route")
            || destination_environment.eq_ignore_ascii_case("town")
        {
            // ResetFlashIfOutOfCave clears the transient illumination only
            // when map setup reaches an outdoor route or town.  Keeping this
            // flag forever made every later PALETTE_DARK cave render as lit.
            state
                .flags
                .set_engine_flag("STATUSFLAGS_FLASH", false)
                .map_err(|error| anyhow::anyhow!("reset FLASH on outdoor map entry: {error}"))?;
        }
        self.update_roam_mons_on_map_change(state, destination_map)?;
        self.apply_saved_overworld_overrides(session, state)?;
        let mode = self.map_entry_movement_mode(state, session, mode)?;
        session.player.mode = mode;
        self.sync_current_map_music(state, destination_map, mode, music_ids)?;
        self.sync_current_map_scene(state, destination_map)?;
        self.apply_map_object_callbacks(state, session, destination_map)?;
        let callback_mode = self.map_entry_movement_mode(state, session, session.player.mode)?;
        if callback_mode != session.player.mode {
            session.player.mode = callback_mode;
            self.sync_current_map_music(state, destination_map, callback_mode, music_ids)?;
        }
        self.commit_overworld_snapshot(state, session, spawn_update);
        Ok(())
    }

    pub fn start_overworld_session_from_spawn(
        &self,
        spawn: &RuntimeSpawnPoint,
        music_ids: &BTreeSet<String>,
    ) -> Result<(GameState, OverworldSession)> {
        let spawn_tile = runtime_spawn_expected_tile(spawn);
        let mut overworld = self.overworld_session(&spawn.map_name, spawn_tile, 0)?;
        let mut state = GameState::reset_wram_for_new_game();
        state.wild_encounter_cooldown = 5;
        state.bag.tm_hm = initial_tmhm_flags(&self.items);
        apply_initialize_events(&mut state, &self.initialize_events)
            .map_err(|error| anyhow::anyhow!("apply initialize events: {error}"))?;
        self.commit_overworld_snapshot(
            &mut state,
            &overworld,
            SpawnMemoryUpdate::Set(spawn.identifier),
        );
        let map_name = overworld.map.name.clone();
        overworld.player.mode =
            self.map_entry_movement_mode(&state, &overworld, overworld.player.mode)?;
        self.sync_current_map_music(
            &mut state,
            &map_name,
            overworld.player.mode,
            music_ids,
        )?;
        self.sync_current_map_scene(&mut state, &map_name)?;
        self.apply_map_object_callbacks(&mut state, &mut overworld, &map_name)?;
        self.commit_overworld_snapshot(
            &mut state,
            &overworld,
            SpawnMemoryUpdate::Set(spawn.identifier),
        );
        overworld.set_time_of_day(state.time.time_of_day);
        Ok((state, overworld))
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn start_overworld_session_at_runtime_tile(
        &self,
        map_name: &str,
        tile: TilePosition,
        music_ids: &BTreeSet<String>,
    ) -> Result<(GameState, OverworldSession)> {
        let mut overworld = self.overworld_session(map_name, tile, 0)?;
        let mut state = GameState::reset_wram_for_new_game();
        state.bag.tm_hm = initial_tmhm_flags(&self.items);
        apply_initialize_events(&mut state, &self.initialize_events)
            .map_err(|error| anyhow::anyhow!("apply initialize events: {error}"))?;
        self.commit_overworld_snapshot(&mut state, &overworld, SpawnMemoryUpdate::Preserve);
        let map_name = overworld.map.name.clone();
        overworld.player.mode =
            self.map_entry_movement_mode(&state, &overworld, overworld.player.mode)?;
        self.sync_current_map_music(
            &mut state,
            &map_name,
            overworld.player.mode,
            music_ids,
        )?;
        self.sync_current_map_scene(&mut state, &map_name)?;
        self.apply_map_object_callbacks(&mut state, &mut overworld, &map_name)?;
        self.commit_overworld_snapshot(&mut state, &overworld, SpawnMemoryUpdate::Preserve);
        overworld.set_time_of_day(state.time.time_of_day);
        Ok((state, overworld))
    }

    pub fn resume_overworld_session_from_state(
        &self,
        mut state: GameState,
        music_ids: &BTreeSet<String>,
    ) -> Result<(GameState, OverworldSession)> {
        let (map_name, tile, facing, mode) = state
            .overworld
            .snapshot_identity()
            .with_context(|| "cannot resume overworld session from inactive GameState")?;
        let map_name = map_name.to_string();
        let mut overworld = self.overworld_session_for_traversal(
            &map_name,
            tile,
            state.frame_counter,
            mode.traversal_state(),
        )?;
        overworld.player.facing = facing;
        overworld.player.mode = mode;
        self.apply_saved_overworld_overrides(&mut overworld, &state)?;
        let mode = self.map_entry_movement_mode(&state, &overworld, mode)?;
        overworld.player.mode = mode;
        self.sync_current_map_music(&mut state, &map_name, mode, music_ids)?;
        self.sync_current_map_scene(&mut state, &map_name)?;
        self.apply_map_object_callbacks(&mut state, &mut overworld, &map_name)?;
        self.commit_overworld_snapshot(&mut state, &overworld, SpawnMemoryUpdate::Preserve);
        overworld.set_time_of_day(state.time.time_of_day);
        if let Some(music) = state.script_runtime.current_music.as_deref() {
            core_validate_saved_audio_reference(
                "state.script_runtime.current_music",
                music,
                ModpackAudioKind::Music.save_name(),
                music_ids
                    .contains(music)
                    .then_some(ModpackAudioKind::Music.save_name()),
            )
            .map_err(|error| anyhow::anyhow!("{error}"))?;
        }
        Ok((state, overworld))
    }

    pub fn apply_overworld_input(
        &self,
        state: &mut GameState,
        session: &mut OverworldSession,
        buttons: impl IntoIterator<Item = GameButton>,
        music_ids: &BTreeSet<String>,
    ) -> Result<OverworldInputFrame> {
        let input_candidate = JoypadState::compute_mask(buttons);
        // The Bevy frame loop supplies empty input on most render ticks.  If
        // no script, battle, timer, or autonomous object needs servicing,
        // advance only the authoritative frame cursors in place instead of
        // cloning the entire GameState and OverworldSession.  This preserves
        // the exact frame/checksum contract while removing a large per-frame
        // allocation and validation cost from idle play.
        let has_autonomous_objects = session.objects.iter().any(|object| {
            matches!(
                object.spritemovedata.as_str(),
                "SPRITEMOVEDATA_WALK_LEFT_RIGHT"
                    | "SPRITEMOVEDATA_WALK_UP_DOWN"
                    | "SPRITEMOVEDATA_WANDER"
                    | "SPRITEMOVEDATA_SWIM_WANDER"
                    | "SPRITEMOVEDATA_SPINCLOCKWISE"
                    | "SPRITEMOVEDATA_SPINCOUNTERCLOCKWISE"
                    | "SPRITEMOVEDATA_SPINRANDOM_SLOW"
                    | "SPRITEMOVEDATA_SPINRANDOM_FAST"
            )
        });
        let input_locked = Self::game_state_blocks_overworld_input(state);
        let forced_tile_movement_pending =
            !input_locked && session.forced_movement_direction().is_some();
        let downhill_movement_pending = !input_locked
            && matches!(session.player.mode, MovementMode::Bike | MovementMode::Skate)
            && state
                .flags
                .is_engine_flag_set("ENGINE_DOWNHILL")
                .map_err(|error| anyhow::anyhow!("check downhill bike flag: {error}"))?;
        if input_candidate == 0
            && !state.bug_contest.timer_active
            && !forced_tile_movement_pending
            && !downhill_movement_pending
            && (input_locked || !has_autonomous_objects)
        {
            state
                .apply_joypad_mask(0)
                .map_err(|error| anyhow::anyhow!("apply idle joypad mask: {error}"))?;
            session.frame = session
                .frame
                .checked_add(1)
                .context("advance idle overworld frame")?;
            let snapshot = session.snapshot();
            self.commit_overworld_snapshot_data(state, &snapshot, SpawnMemoryUpdate::Preserve);
            return Ok(OverworldInputFrame {
                snapshot,
                input_mask: 0,
                pressed_mask: 0,
                autonomous_objects_changed: false,
                movement: None,
                ledge_jump: None,
                grass_rustle: None,
                step_events: None,
                coord_event: None,
                trainer_sight: None,
                interaction: None,
                warp: None,
                connection: None,
                wild_encounter: None,
                wild_battle: None,
            });
        }
        let mut staged_state = state.clone();
        let mut staged_session = session.clone();
        let joypad_event = staged_state
            .apply_joypad_mask(input_candidate)
            .map_err(|error| anyhow::anyhow!("apply joypad mask: {error}"))?;
        let (pressed_mask, input_mask) = match joypad_event {
            crystal_core::state::GameEvent::JoypadChanged { pressed, down } => (pressed, down),
            event => anyhow::bail!("joypad command produced unexpected event {event:?}"),
        };

        let mut bug_contest_timed_out = false;
        if staged_state.bug_contest.timer_active
            && staged_state
                .flags
                .is_engine_flag_set("ENGINE_BUG_CONTEST_TIMER")
                .map_err(|error| anyhow::anyhow!("check Bug Contest timer flag: {error}"))?
        {
            let timer =
                self.apply_internal_special_routine(&mut staged_state, "CheckBugContestTimer")?;
            bug_contest_timed_out = matches!(
                timer.effect,
                SpecialRoutineEffect::BugContestTimer { active: false, .. }
            ) && staged_session.map.name == "NationalParkBugContest";
        }

        if bug_contest_timed_out {
            for index in 1..=10 {
                let flag_a = format!("EVENT_BUG_CATCHING_CONTESTANT_{index}A");
                let flag_b = format!("EVENT_BUG_CATCHING_CONTESTANT_{index}B");
                let selected = staged_state
                    .flags
                    .is_event_flag_set(&flag_a)
                    .map_err(|error| {
                        anyhow::anyhow!("read Bug Contest contestant flag: {error}")
                    })?;
                if !selected {
                    staged_state
                        .flags
                        .set_event_flag(&flag_b, false)
                        .map_err(|error| {
                            anyhow::anyhow!("clear Bug Contest contestant flag: {error}")
                        })?;
                }
            }
            staged_state
                .flags
                .set_event_flag(
                    "EVENT_ROUTE_36_NATIONAL_PARK_GATE_OFFICER_CONTEST_DAY",
                    true,
                )
                .map_err(|error| anyhow::anyhow!("set Bug Contest result flag: {error}"))?;
            staged_state
                .flags
                .set_event_flag(
                    "EVENT_ROUTE_36_NATIONAL_PARK_GATE_OFFICER_NOT_CONTEST_DAY",
                    false,
                )
                .map_err(|error| anyhow::anyhow!("clear Bug Contest result flag: {error}"))?;
            staged_state
                .flags
                .set_event_flag("EVENT_WARPED_FROM_ROUTE_35_NATIONAL_PARK_GATE", true)
                .map_err(|error| anyhow::anyhow!("set Bug Contest warp flag: {error}"))?;
            let destination_tile = raw_event_tile_to_runtime_tile_checked(0, 4)
                .context("resolve Bug Contest results warp tile")?;
            let mode = staged_session.player.mode;
            self.transition_overworld_session_with_mode(
                &mut staged_state,
                &mut staged_session,
                "Route36NationalParkGate",
                destination_tile,
                mode,
                SpawnMemoryUpdate::Preserve,
                music_ids,
            )?;
        }

        let mut movement = None;
        let mut ledge_jump = None;
        let mut grass_rustle = None;
        let mut step_events = None;
        let mut coord_event = None;
        let mut trainer_sight = None;
        let mut warp = None;
        let mut connection = None;
        let mut interaction = None;
        let mut wild_encounter = None;
        let mut wild_battle = None;
        let mut strength_boulder_landing = false;

        let overworld_input_locked =
            bug_contest_timed_out || Self::game_state_blocks_overworld_input(&staged_state);
        let downhill = !overworld_input_locked
            && input_candidate == 0
            && matches!(
                staged_session.player.mode,
                MovementMode::Bike | MovementMode::Skate
            )
            && staged_state
                .flags
                .is_engine_flag_set("ENGINE_DOWNHILL")
                .map_err(|error| anyhow::anyhow!("check downhill bike flag: {error}"))?;
        let tile_forced_direction = if overworld_input_locked {
            None
        } else {
            staged_session.forced_movement_direction()
        };
        let tile_forced_permission = tile_forced_direction.and_then(|_| {
            sample_collision(
                &staged_session.map,
                &staged_session.tileset,
                staged_session.player.tile,
            )
            .map(|sample| sample.permission)
        });
        let forced_direction = tile_forced_direction.or(downhill.then_some(Direction::Down));
        let direction = if overworld_input_locked {
            None
        } else if forced_direction.is_some() {
            forced_direction
        } else if pressed_mask & B_PAD_A != 0 {
            None
        } else {
            direction_from_pad_mask(input_mask)
                .map_err(|error| anyhow::anyhow!("apply overworld input: {error:?}"))?
        };

        if overworld_input_locked {
            staged_session.frame += 1;
        } else if let Some(direction) = direction {
            let movement_mode_before = staged_session.player.mode;
            let direct_forced_step = tile_forced_permission.is_some_and(|permission| {
                matches!(permission & 0xf0, 0x30 | 0x40 | 0x50)
            });
            let blocked_connection_edge = if direct_forced_step {
                None
            } else {
                self.blocked_connection_edge_target(&staged_session, direction)?
            };
            let mut warp_trigger = None;
            if let Some(target) = blocked_connection_edge {
                staged_session.player.facing = direction;
                staged_session.frame += 1;
                movement = Some(StepOutcome::Blocked {
                    at: target,
                    facing: direction,
                });
            } else {
                let options = StepOptions {
                    // CheckTile-owned currents, walk tiles, doors and ice
                    // continue directly. Downhill instead enters through the
                    // joypad path and must preserve CheckTurning's four-frame
                    // facing change before its forced downward step.
                    force_step_after_turn: tile_forced_direction.is_some(),
                    ..StepOptions::default()
                };
                let strength_active = staged_state
                    .flags
                    .is_engine_flag_set("ENGINE_STRENGTH_ACTIVE")
                    .map_err(|error| anyhow::anyhow!("check active Strength flag: {error}"))?;
                let can_jump_ledge = !direct_forced_step
                    && staged_session.can_jump_ledge_checked(direction, options)?;
                let pushed_boulder = (strength_active && !direct_forced_step && !can_jump_ledge)
                    .then(|| {
                        staged_session
                            .push_strength_boulder_checked(direction, options)
                            .with_context(|| {
                                format!("push Strength boulder on {}", staged_session.map.name)
                            })
                    })
                    .transpose()?
                    .flatten();
                if let Some(object_id) = pushed_boulder {
                    staged_state
                        .script_runtime
                        .audio_events
                        .push(ScriptAudioRuntimeEvent {
                            command: "playsound".to_string(),
                            kind: ScriptAudioRuntimeKind::SoundEffect,
                            audio_id: Some("SFX_STRENGTH".to_string()),
                            fade_frames: None,
                            source_script: "MovementFunction_Strength".to_string(),
                            command_index: 0,
                        });
                    strength_boulder_landing = self.queue_strength_boulder_landing_script(
                        &mut staged_state,
                        &staged_session,
                        &object_id,
                    )?;
                }
                if direct_forced_step {
                    movement = Some(
                        staged_session
                            .forced_tile_step_checked(direction, options)
                            .with_context(|| {
                                format!(
                                    "apply direct CheckTile movement on {}",
                                    staged_session.map.name
                                )
                            })?,
                    );
                } else if can_jump_ledge {
                    let result = staged_session
                        .ledge_jump_and_check_warp_checked(direction, options)
                        .with_context(|| {
                            format!("apply overworld ledge jump on {}", staged_session.map.name)
                        })?;
                    movement = Some(match &result.outcome {
                        LedgeJumpOutcome::Jumped {
                            from,
                            to,
                            speed_multiplier,
                            ..
                        } => StepOutcome::Moved {
                            from: *from,
                            to: *to,
                            speed_multiplier: *speed_multiplier,
                        },
                        LedgeJumpOutcome::BlockedLanding { at, facing }
                        | LedgeJumpOutcome::NotLedge { at, facing } => StepOutcome::Blocked {
                            at: *at,
                            facing: *facing,
                        },
                        LedgeJumpOutcome::BlockedByObject {
                            at,
                            facing,
                            object_identifier,
                        } => StepOutcome::BlockedByObject {
                            at: *at,
                            facing: *facing,
                            object_identifier: object_identifier.clone(),
                        },
                        LedgeJumpOutcome::RuntimeTileOverflow { from, facing } => {
                            StepOutcome::RuntimeTileOverflow {
                                from: *from,
                                facing: *facing,
                            }
                        }
                    });
                    ledge_jump = Some(result.outcome);
                    warp_trigger = result.warp;
                } else {
                    let result = staged_session
                        .step_and_check_warp_checked(direction, options)
                        .with_context(|| {
                            format!("apply overworld movement on {}", staged_session.map.name)
                        })?;
                    movement = Some(result.outcome);
                    warp_trigger = result.warp;
                }
                if ledge_jump.is_none()
                    && let Some(permission) = tile_forced_permission
                    && let Some(StepOutcome::Moved {
                        speed_multiplier, ..
                    }) = movement.as_mut()
                {
                    // CheckTile uses STEP_WALK for currents, directional walk
                    // tiles, doors, staircases and caves regardless of the
                    // player's bike/skate state. Ice bypasses CheckTile and
                    // TryStep selects STEP_ICE's four-pixel slide instead.
                    let forced_speed = if matches!(
                        permission,
                        permissions::ICE | permissions::ICE_2B
                    ) {
                        2
                    } else {
                        1
                    };
                    *speed_multiplier = forced_speed;
                    // step_checked initially retains the origin from the
                    // actor mode. The source-selected step function owns that
                    // collision tile until its visible 8/4-frame landing.
                    staged_session.player_last_tile_occupied_until_frame = staged_session
                        .frame
                        .saturating_add(u64::from(8 / forced_speed) - 1);
                }
                if tile_forced_direction.is_none()
                    && staged_state
                    .flags
                    .is_engine_flag_set("ENGINE_DOWNHILL")
                    .map_err(|error| anyhow::anyhow!("check downhill bike speed flag: {error}"))?
                    && matches!(movement_mode_before, MovementMode::Bike | MovementMode::Skate)
                    && direction != Direction::Down
                    && let Some(StepOutcome::Moved {
                        speed_multiplier, ..
                    }) = movement.as_mut()
                {
                    // ASM TryStep falls back to STEP_WALK when a downhill
                    // bike/skate actor moves in any direction except down.
                    *speed_multiplier = 1;
                    // `OverworldSession::step_checked` initially retained
                    // the vacated tile using the actor mode's ordinary bike
                    // duration. Keep OBJECT_LAST_MAP_* collision ownership
                    // for the complete overridden eight-frame walk instead
                    // of releasing it halfway through the visible stride.
                    staged_session.player_last_tile_occupied_until_frame =
                        staged_session.frame.saturating_add(7);
                }
            }
            if staged_session.player.mode != movement_mode_before {
                self.sync_current_map_music(
                    &mut staged_state,
                    &staged_session.map.name,
                    staged_session.player.mode,
                    music_ids,
                )?;
            }
            let moved = matches!(movement, Some(StepOutcome::Moved { .. }));
            // Door/cave carpet warps at a map edge activate from the tile the
            // player is standing on once the required facing is established.
            // The attempted outbound step can be collision-blocked; TS/ASM
            // still run the warp check after the turn. Checking only the
            // destination of a successful step strands every such doorway.
            if warp_trigger.is_none() {
                warp_trigger = staged_session
                    .check_warp_checked()
                    .with_context(|| {
                        format!("check current warp on {}", staged_session.map.name)
                    })?;
            }
            if !moved {
                if let Some(trigger) = warp_trigger.take() {
                    let transition = self.resolve_warp_transition_with_state(
                        &mut staged_state,
                        &trigger,
                    )?;
                    self.apply_dig_warp_memory_for_transition(
                        &mut staged_state,
                        &transition,
                    )?;
                    let destination = &transition.destination;
                    let mode = staged_session.player.mode;
                    self.transition_overworld_session_with_mode(
                        &mut staged_state,
                        &mut staged_session,
                        &destination.map_name,
                        destination.tile,
                        mode,
                        SpawnMemoryUpdate::Preserve,
                        music_ids,
                    )?;
                    warp = Some(transition);
                }
            }
            if moved {
                if let Some(StepOutcome::Moved {
                    to,
                    speed_multiplier,
                    ..
                }) = movement.as_ref()
                {
                    let grass_permission = sample_collision(
                        &staged_session.map,
                        &staged_session.tileset,
                        *to,
                    )
                    .map(|sample| sample.permission);
                    if grass_permission
                        .is_some_and(|permission| matches!(permission, 0x14 | 0x18 | 0x1c))
                    {
                        grass_rustle = Some(OverworldGrassRustle {
                            tile: *to,
                            duration_frames: (8 / (*speed_multiplier).max(1)).max(1),
                        });
                    }
                }
                step_events = Some(self.process_overworld_step(&mut staged_state)?);
                if step_events
                    .as_ref()
                    .is_some_and(|events| {
                        events.repel_expired.is_none()
                            && !events.egg_hatched
                            && events.poison_result.is_none()
                    })
                    && !strength_boulder_landing
                {
                    trainer_sight =
                        self.check_trainer_sight_after_step(&staged_state, &staged_session)?;
                    if trainer_sight.is_none() {
                        let connection_trigger = staged_session
                            .check_connection_checked()
                            .with_context(|| {
                                format!("check connection on {}", staged_session.map.name)
                            })?;
                        if let Some(trigger) = connection_trigger {
                            let transition = self.resolve_connection_transition(&trigger)?;
                            let destination = &transition.destination;
                            let mode = staged_session.player.mode;
                            self.transition_overworld_session_with_mode(
                                &mut staged_state,
                                &mut staged_session,
                                &destination.map_name,
                                destination.tile,
                                mode,
                                SpawnMemoryUpdate::Preserve,
                                music_ids,
                            )?;
                            connection = Some(transition);
                        } else if let Some(trigger) = warp_trigger {
                            let transition = self.resolve_warp_transition_with_state(
                                &mut staged_state,
                                &trigger,
                            )?;
                            self.apply_dig_warp_memory_for_transition(
                                &mut staged_state,
                                &transition,
                            )?;
                            let destination = &transition.destination;
                            let mode = staged_session.player.mode;
                            self.transition_overworld_session_with_mode(
                                &mut staged_state,
                                &mut staged_session,
                                &destination.map_name,
                                destination.tile,
                                mode,
                                SpawnMemoryUpdate::Preserve,
                                music_ids,
                            )?;
                            warp = Some(transition);
                        } else {
                            coord_event = self.check_coord_event_after_step_checked(
                                &staged_state,
                                &staged_session,
                            )?;
                            if coord_event.is_none() {
                                wild_encounter = self.check_wild_encounter_after_step(
                                    &mut staged_state,
                                    &staged_session,
                                )?;
                                wild_battle = self.start_resolved_wild_encounter_after_step(
                                    &mut staged_state,
                                    &wild_encounter,
                                )?;
                            }
                        }
                    }
                }
            }
        } else {
            staged_session.frame += 1;
        }

        if !overworld_input_locked && direction.is_none() && pressed_mask & B_PAD_A != 0 {
            interaction = staged_session
                .check_interaction_checked(StepOptions::default().stride_tiles)
                .with_context(|| {
                    format!("check overworld interaction on {}", staged_session.map.name)
                })?;
        }

        if !overworld_input_locked {
            let has_autonomous_object = staged_session.objects.iter().any(|object| {
                matches!(
                    object.spritemovedata.as_str(),
                    "SPRITEMOVEDATA_WALK_LEFT_RIGHT"
                        | "SPRITEMOVEDATA_WALK_UP_DOWN"
                        | "SPRITEMOVEDATA_WANDER"
                        | "SPRITEMOVEDATA_SWIM_WANDER"
                        | "SPRITEMOVEDATA_SPINRANDOM_SLOW"
                        | "SPRITEMOVEDATA_SPINRANDOM_FAST"
                        | "SPRITEMOVEDATA_SPINCLOCKWISE"
                        | "SPRITEMOVEDATA_SPINCOUNTERCLOCKWISE"
                )
            });
            if has_autonomous_object {
                let mut autonomous_rng = Random::new_crystal(staged_state.rng_seed);
                staged_session
                    .advance_autonomous_objects_with_rng(Some(&mut autonomous_rng))
                    .map_err(|error| {
                        anyhow::anyhow!("advance autonomous overworld objects: {error}")
                    })?;
                staged_state.commit_rng_seed(autonomous_rng.seed());
            } else {
                staged_session
                    .advance_autonomous_objects()
                    .map_err(|error| {
                        anyhow::anyhow!("advance autonomous overworld objects: {error}")
                    })?;
            }
        }

        let autonomous_objects_changed = session.object_runtime_tiles
            != staged_session.object_runtime_tiles
            || session.object_facings != staged_session.object_facings;
        if warp.is_some() || connection.is_some() {
            grass_rustle = None;
        }
        let snapshot = staged_session.snapshot();
        self.commit_overworld_snapshot_data(
            &mut staged_state,
            &snapshot,
            SpawnMemoryUpdate::Preserve,
        );
        *state = staged_state;
        *session = staged_session;
        Ok(OverworldInputFrame {
            snapshot,
            input_mask,
            pressed_mask,
            autonomous_objects_changed,
            movement,
            ledge_jump,
            grass_rustle,
            step_events,
            coord_event,
            trainer_sight,
            interaction,
            warp,
            connection,
            wild_encounter,
            wild_battle,
        })
    }

    fn blocked_connection_edge_target(
        &self,
        session: &OverworldSession,
        direction: Direction,
    ) -> Result<Option<TilePosition>> {
        if session.player.facing != direction {
            return Ok(None);
        }
        let Some(target) = checked_move_by_stride(
            session.player.tile,
            direction,
            StepOptions::default().stride_tiles,
        ) else {
            return Ok(None);
        };
        let mut probe = session.clone();
        probe.player.tile = target;
        let Some(trigger) = probe
            .check_connection_checked()
            .with_context(|| format!("check connection on {}", probe.map.name))?
        else {
            return Ok(None);
        };
        if self.connection_trigger_has_destination(&trigger)? {
            Ok(None)
        } else {
            Ok(Some(target))
        }
    }

    fn connection_trigger_has_destination(&self, trigger: &ConnectionTrigger) -> Result<bool> {
        let target_attributes = self
            .map_attributes
            .get(&trigger.connection.target_map)
            .with_context(|| {
                format!(
                    "connection target '{}' missing attributes (referenced by {})",
                    trigger.connection.target_map, trigger.map_name
                )
            })?;
        connection_destination_tile_in_bounds(
            trigger.tile,
            &trigger.connection.direction,
            trigger.connection.offset,
            target_attributes,
        )
    }

    fn game_state_blocks_overworld_input(state: &GameState) -> bool {
        let runtime = &state.script_runtime;
        !matches!(state.battle, BattleMemory::Inactive)
            || runtime.player_input_locked
            || runtime.all_input_locked
            || !runtime.pending_delays.is_empty()
            || !runtime.pending_earthquakes.is_empty()
            || !runtime.pending_emotes.is_empty()
            || !runtime.command_queue.is_empty()
            || runtime.active_menu.is_some()
            || runtime.active_pokemon_picture.is_some()
            || runtime.window_open
            || runtime.text_window_open
            || !runtime.audio_events.is_empty()
            || runtime.pending_music_fade.is_some()
            || runtime.waiting_for_sound_effect
            || !runtime.graphics_events.is_empty()
            || runtime.pending_screen_fade.is_some()
            || !runtime.money_events.is_empty()
            || !runtime.map_events.is_empty()
            || runtime.pending_script_warp.is_some()
            || runtime.pending_map_load.is_some()
            || runtime.pending_map_refresh.is_some()
            || runtime.warp_check_requested
            || !runtime.text_events.is_empty()
            || runtime.pending_text_label.is_some()
            || runtime.pending_text_wait.is_some()
            || runtime.pending_yes_no.is_some()
            || !runtime.control_events.is_empty()
            || runtime.next_script.is_some()
            || !runtime.deferred_scripts.is_empty()
            || runtime.script_ended.is_some()
            || !runtime.shop_events.is_empty()
            || runtime.pending_shop.is_some()
            || !runtime.item_use_events.is_empty()
    }

    pub fn cry_by_species(&self) -> BTreeMap<String, String> {
        self.pokemon_cries
            .iter()
            .map(|(species_id, cry)| (species_id.clone(), cry.cry.clone()))
            .collect()
    }

    pub fn special_routine_context<'a>(
        &'a self,
        cry_by_species: &'a BTreeMap<String, String>,
    ) -> SpecialRoutineContext<'a> {
        SpecialRoutineContext {
            move_catalog: &self.moves,
            cry_by_species,
            species_catalog: &self.pokemon,
            learnsets: &self.learnsets,
            growth_rates: &self.growth_rates,
            item_catalog: &self.items,
            runtime_spawn_points: self.runtime_spawn_points(),
            roaming_pokemon: &self.roaming_pokemon,
            buena_password_categories: &self.buena_password_categories,
            buena_prizes: &self.buena_prizes,
            kurt_apricorn_recipes: &self.kurt_apricorn_recipes,
            shuckie_gift: self.shuckie_gift.as_ref(),
            dratini_move_sets: &self.dratini_move_sets,
            bug_contest_config: self.bug_contest_config.as_ref(),
            battle_tower_rules: self.battle_tower_rules.as_ref(),
            magikarp_lengths: &self.magikarp_lengths,
            happiness_data: self.happiness_data.as_ref(),
            trainer_catalog: &self.trainers,
            phone_contacts: &self.phone_contacts,
            wild_encounters: &self.wild_encounters,
            odd_egg_definitions: &self.odd_egg_definitions,
            oak_ratings: &self.oak_ratings,
        }
    }

    pub fn apply_special_routine(
        &self,
        state: &mut GameState,
        routine: &str,
        music_ids: &BTreeSet<String>,
    ) -> Result<SpecialRoutineOutcome> {
        self.require_special_routine(routine)?;
        if routine == "FadeOutMusic" {
            core_validate_saved_audio_reference(
                "special_routines.FadeOutMusic",
                "MUSIC_NONE",
                ModpackAudioKind::Music.save_name(),
                music_ids
                    .contains("MUSIC_NONE")
                    .then_some(ModpackAudioKind::Music.save_name()),
            )
            .map_err(|error| anyhow::anyhow!("{error}"))?;
        }
        let cry_by_species = self.cry_by_species();
        let context = self.special_routine_context(&cry_by_species);
        apply_special_routine_with_context(state, context, routine)
            .map_err(|error| anyhow::anyhow!("apply special routine {routine}: {error}"))
    }

    pub fn apply_internal_special_routine(
        &self,
        state: &mut GameState,
        routine: &str,
    ) -> Result<SpecialRoutineOutcome> {
        if !matches!(routine, "StartBugContestTimer" | "CheckBugContestTimer") {
            anyhow::bail!("unsupported internal special routine {routine}");
        }
        let cry_by_species = self.cry_by_species();
        let context = self.special_routine_context(&cry_by_species);
        apply_special_routine_with_context(state, context, routine)
            .map_err(|error| anyhow::anyhow!("apply internal special routine {routine}: {error}"))
    }

    fn apply_special_routine_transactional<F>(
        &self,
        state: &mut GameState,
        routine: &str,
        music_ids: &BTreeSet<String>,
        prepare: F,
    ) -> Result<SpecialRoutineOutcome>
    where
        F: FnOnce(&mut GameState) -> Result<()>,
    {
        let mut next_state = state.clone();
        prepare(&mut next_state)?;
        let outcome = self.apply_special_routine(&mut next_state, routine, music_ids)?;
        *state = next_state;
        Ok(outcome)
    }

    pub fn audio_ids(&self) -> BTreeSet<&str> {
        self.audio.iter().map(|asset| asset.id.as_str()).collect()
    }

    pub fn audio_manifest(
        &self,
        compiled_audio: &BTreeMap<String, Vec<u8>>,
    ) -> Result<ModpackAudioManifest> {
        ModpackAudioManifest::from_assets(&self.audio, compiled_audio)
    }

    pub fn script_text_labels(module: &MapModule) -> BTreeSet<String> {
        module.script_text_bodies.keys().cloned().collect()
    }

    pub fn script_numeric_constants(&self) -> BTreeMap<String, i32> {
        let mut constants = BTreeMap::new();
        for (constant, value) in &self.currency_constants.0 {
            if let Ok(value) = i32::try_from(*value) {
                constants.insert(constant.clone(), value);
            }
        }
        for (constant, value) in &self.story_event_script_constants.global {
            if let Ok(value) = i32::try_from(*value) {
                constants.insert(constant.clone(), value);
            }
        }
        for constants_by_map in self.story_event_script_constants.maps.values() {
            for (constant, value) in constants_by_map {
                if let Ok(value) = i32::try_from(*value) {
                    constants.insert(constant.clone(), value);
                }
            }
        }
        constants
    }

    pub fn saved_map_id(&self, map_name: &str) -> Option<&str> {
        self.maps.get(map_name).map(|module| module.id.as_str())
    }

    pub fn validate_saved_pokedex_references(&self, pokedex: &PokedexState) -> Result<()> {
        validate_saved_pokedex_references(pokedex, |species| self.saved_species_id(species))
            .map_err(|error| anyhow::anyhow!("{error}"))
    }

    pub fn validate_saved_bag_references(&self, bag: &Bag) -> Result<()> {
        validate_saved_bag_pocket_references(
            &self.items,
            "bag.items",
            &bag.items,
            ITEM_POCKET_ITEM,
        )
        .map_err(|error| anyhow::anyhow!("{error}"))?;
        validate_saved_bag_pocket_references(
            &self.items,
            "bag.pc_items",
            &bag.pc_items,
            ITEM_POCKET_ITEM,
        )
        .map_err(|error| anyhow::anyhow!("{error}"))?;
        validate_saved_bag_pocket_references(
            &self.items,
            "bag.balls",
            &bag.balls,
            ITEM_POCKET_BALL,
        )
        .map_err(|error| anyhow::anyhow!("{error}"))?;
        validate_saved_bag_pocket_references(
            &self.items,
            "bag.key_items",
            &bag.key_items,
            ITEM_POCKET_KEY_ITEM,
        )
        .map_err(|error| anyhow::anyhow!("{error}"))?;
        for (pocket_id, inventory) in &bag.custom_pockets {
            validate_saved_bag_pocket_references(
                &self.items,
                &format!("bag.custom_pockets.{pocket_id}"),
                inventory,
                pocket_id,
            )
            .map_err(|error| anyhow::anyhow!("{error}"))?;
        }
        validate_saved_tmhm_references(&self.items, &bag.tm_hm)
            .map_err(|error| anyhow::anyhow!("{error}"))
    }

    pub fn validate_saved_active_repel_item(
        &self,
        item_id: &str,
        steps_remaining: u16,
    ) -> Result<()> {
        core_validate_saved_active_repel_item(
            &self.field_moves,
            item_id,
            self.saved_item(item_id),
            steps_remaining,
        )
        .map_err(|error| anyhow::anyhow!("saved active_repel_item {item_id}: {error:?}"))
    }

    pub fn validate_saved_overworld_references(&self, overworld: &OverworldMemory) -> Result<()> {
        core_validate_saved_overworld_references(overworld, |map_name| {
            self.saved_map_tile_bounds(map_name)
        })
        .map_err(|error| anyhow::anyhow!("{error}"))
    }

    pub fn validate_saved_scene_references(&self, scenes: &SceneMemory) -> Result<()> {
        core_validate_saved_scene_references(
            scenes,
            |map_name| self.saved_map_id(map_name).is_some(),
            |map_name, scene_name| self.saved_scene_index(map_name, scene_name),
        )
        .map_err(|error| anyhow::anyhow!("{error}"))
    }

    pub fn validate_saved_block_overrides(
        &self,
        map_name: &str,
        overrides: &BTreeMap<(u16, u16), u16>,
    ) -> Result<()> {
        core_validate_saved_block_overrides(
            map_name,
            overrides,
            |map_name| self.saved_map_block_context(map_name),
            |tileset_name| self.saved_tileset_exists(tileset_name),
            |tileset_name, block_id| self.tileset_declares_metatile(tileset_name, block_id),
        )
        .map_err(|error| anyhow::anyhow!("{error}"))
    }

    pub fn validate_saved_object_overrides(
        &self,
        map_name: &str,
        memory: &OverworldObjectMapMemory,
    ) -> Result<()> {
        core_validate_saved_object_overrides(
            map_name,
            memory,
            |_| self.saved_map_tile_bounds(map_name),
            |object_id| self.map_declares_event_object(map_name, object_id),
        )
        .map_err(|error| anyhow::anyhow!("{error}"))
    }

    pub fn validate_saved_storage_references(&self, storage: &PokemonStorage) -> Result<()> {
        core_validate_saved_storage_references(storage, |path, pokemon| {
            self.validate_saved_pokemon_reference(path, pokemon)
                .map_err(|error| error.to_string())
        })
        .map_err(|error| anyhow::anyhow!("{error}"))
    }

    pub fn validate_saved_bug_contest_references(
        &self,
        bug_contest: &BugContestState,
    ) -> Result<()> {
        core_validate_saved_bug_contest_references(
            bug_contest,
            |path, pokemon| {
                self.validate_saved_pokemon_reference(path, pokemon)
                    .map_err(|error| error.to_string())
            },
            |species| self.saved_species_exists(species),
            |flag| self.saved_event_flag_exists(flag),
        )
        .map_err(|error| anyhow::anyhow!("{error}"))
    }

    pub fn validate_saved_day_care_references(&self, day_care: &DayCareState) -> Result<()> {
        core_validate_saved_day_care_references(
            day_care,
            |path, pokemon| {
                self.validate_saved_pokemon_reference(path, pokemon)
                    .map_err(|error| error.to_string())
            },
            |species| self.saved_species_exists(species),
        )
        .map_err(|error| anyhow::anyhow!("{error}"))
    }

    pub fn validate_saved_roaming_references(
        &self,
        roaming_pokemon: &[RoamingPokemonState],
    ) -> Result<()> {
        core_validate_saved_roaming_references(
            roaming_pokemon,
            |species| self.saved_species_exact_exists(species),
            |species, level| self.saved_roaming_species_level_exists(species, level),
            |map_group, map_number| self.runtime_map_group_number_exists(map_group, map_number),
        )
        .map_err(|error| anyhow::anyhow!("{error}"))
    }

    pub fn validate_saved_mystery_gift_references(
        &self,
        mystery_gift: &MysteryGiftState,
    ) -> Result<()> {
        core_validate_saved_mystery_gift_references(mystery_gift, |item_id| {
            self.saved_item_exists(item_id)
        })
        .map_err(|error| anyhow::anyhow!("{error}"))
    }

    pub fn validate_saved_magikarp_record_references(
        &self,
        record: &MagikarpRecordState,
    ) -> Result<()> {
        validate_saved_magikarp_record_references(record, !self.magikarp_lengths.is_empty())
            .map_err(|error| anyhow::anyhow!("{error}"))
    }

    pub fn validate_saved_blue_card_references(&self, state: &GameState) -> Result<()> {
        validate_saved_blue_card_balance(state, !self.buena_prizes.is_empty())
            .map_err(|error| anyhow::anyhow!("{error}"))
    }

    pub fn validate_saved_buena_password_references(
        &self,
        password: &BuenasPasswordState,
    ) -> Result<()> {
        validate_saved_buena_password_references(password, &self.buena_password_categories)
            .map_err(|error| anyhow::anyhow!("{error}"))
    }

    pub fn validate_saved_battle_tower_references(
        &self,
        tower: &BattleTowerState,
        party: &Party,
    ) -> Result<()> {
        if tower.reward_item != BattleTowerState::default().reward_item {
            self.validate_saved_item_reference("battle_tower.reward_item", &tower.reward_item)?;
        }
        core_validate_saved_battle_tower_state(tower, party, self.battle_tower_rules.as_ref())
            .map_err(|error| anyhow::anyhow!("{error}"))?;
        if saved_battle_tower_state_is_active(tower) {
            self.validate_saved_item_reference("battle_tower.reward_item", &tower.reward_item)?;
        }
        if let Some(trainer_id) = &tower.loaded_trainer_id {
            let canonical_tower_trainer = self.battle_tower_rules.as_ref().is_some_and(|rules| {
                rules
                    .trainers
                    .iter()
                    .any(|trainer| format!("BATTLE_TOWER_{}", trainer.index) == *trainer_id)
            });
            if !canonical_tower_trainer {
                let _ = self.validate_saved_trainer_reference(
                    "battle_tower.loaded_trainer_id",
                    trainer_id,
                )?;
            }
        }
        if let Some(sprite_id) = &tower.last_sprite_constant {
            self.validate_saved_sprite_reference("battle_tower.last_sprite_constant", sprite_id)?;
        }
        Ok(())
    }

    pub fn validate_saved_link_session_references(
        &self,
        link_session: &LinkSessionState,
    ) -> Result<()> {
        core_validate_saved_link_session_references(link_session, |room| {
            self.saved_special_routine_exists(room)
        })
        .map_err(|error| anyhow::anyhow!("{error}"))
    }

    pub fn validate_saved_fishing_references(&self, fishing: &FishingMemory) -> Result<()> {
        core_validate_saved_fishing_references(
            fishing,
            |rod| self.saved_fishing_rod_exists(rod),
            |bit| self.saved_fishing_daily_flag_bit_exists(bit),
            |swarm_flag| self.saved_fishing_swarm_flag_exists(swarm_flag),
        )
        .map_err(|error| anyhow::anyhow!("{error}"))
    }

    pub fn validate_saved_swarm_references(&self, swarms: &SwarmMemory) -> Result<()> {
        let map_groups = self.runtime_map_group_table();
        for (swarm_token, target) in &swarms.active {
            let Some((group_id, map_id)) = map_groups.get(&target.map_id).copied() else {
                anyhow::bail!(
                    "saved swarms.active {swarm_token} references missing runtime map {}",
                    target.map_id
                );
            };
            if target.map_group != Some(group_id) || target.map_number != Some(map_id) {
                anyhow::bail!(
                    "saved swarms.active {swarm_token} map {} has group/number {:?}/{:?}, expected {group_id}/{map_id}",
                    target.map_id,
                    target.map_group,
                    target.map_number
                );
            }
        }
        Ok(())
    }

    pub fn validate_saved_pending_special_battle_type(
        &self,
        battle_type: Option<&str>,
    ) -> Result<()> {
        validate_saved_pending_special_battle_type(
            battle_type,
            |battle_type| self.saved_pending_special_battle_type_exists(battle_type),
            |routine| self.saved_special_routine_exists(routine),
        )
        .map_err(|error| anyhow::anyhow!("{error}"))
    }

    pub fn validate_saved_flag_references(&self, flags: &EventFlagMemory) -> Result<()> {
        core_validate_saved_flag_references(
            flags,
            |flag| self.saved_event_flag_exists(flag),
            |flag| self.saved_engine_flag_exists(flag),
        )
        .map_err(|error| anyhow::anyhow!("{error}"))
    }

    pub fn validate_saved_event_flag_reference(&self, path: &str, flag: &str) -> Result<()> {
        core_validate_saved_event_flag_reference(path, flag, |flag| {
            self.saved_event_flag_exists(flag)
        })
        .map_err(|error| anyhow::anyhow!("{error}"))
    }

    pub fn validate_saved_engine_flag_reference(&self, path: &str, flag: &str) -> Result<()> {
        core_validate_saved_engine_flag_reference(path, flag, |flag| {
            self.saved_engine_flag_exists(flag)
        })
        .map_err(|error| anyhow::anyhow!("{error}"))
    }

    pub fn validate_saved_pokemon_reference(&self, path: &str, pokemon: &Pokemon) -> Result<()> {
        core_validate_saved_pokemon_reference(
            path,
            pokemon,
            |species| self.saved_species(species),
            |item_id| self.saved_item_exists(item_id),
            |status| self.saved_pokemon_status_exists(status),
            |move_name| self.saved_move_name_and_pp(move_name),
        )
        .map_err(|error| anyhow::anyhow!("{error}"))
    }

    pub fn validate_saved_item_reference(&self, path: &str, item_id: &str) -> Result<()> {
        validate_saved_exact_catalog_reference(
            path,
            item_id,
            "items",
            "item script_name",
            self.saved_item_script_name(item_id),
        )
        .map_err(|error| anyhow::anyhow!("{error}"))
    }

    pub fn validate_saved_move_reference(&self, path: &str, move_id: &str) -> Result<()> {
        validate_saved_exact_catalog_reference(
            path,
            move_id,
            "moves",
            "move id",
            self.saved_move_name_and_pp(move_id).map(|(name, _)| name),
        )
        .map_err(|error| anyhow::anyhow!("{error}"))
    }

    pub fn validate_saved_sprite_reference(&self, path: &str, sprite_id: &str) -> Result<()> {
        validate_saved_catalog_reference(path, sprite_id, "sprites", |sprite_id| {
            self.saved_sprite_exists(sprite_id)
        })
        .map_err(|error| anyhow::anyhow!("{error}"))
    }

    pub fn validate_saved_variable_sprite_reference(
        &self,
        path: &str,
        sprite_id: &str,
    ) -> Result<()> {
        validate_saved_catalog_reference(path, sprite_id, "variable sprites", |sprite_id| {
            self.saved_variable_sprite_exists(sprite_id)
        })
        .map_err(|error| anyhow::anyhow!("{error}"))
    }

    pub fn validate_saved_trainer_reference<'a>(
        &'a self,
        path: &str,
        trainer_id: &str,
    ) -> Result<&'a Trainer> {
        validate_saved_exact_catalog_reference(
            path,
            trainer_id,
            "trainers",
            "trainer id",
            self.saved_trainer_id(trainer_id),
        )
        .map_err(|error| anyhow::anyhow!("{error}"))?;
        let trainer = self.saved_trainer(trainer_id).with_context(|| {
            format!("saved {path} trainer id {trainer_id} validated but is missing")
        })?;
        Ok(trainer)
    }

    pub fn validate_saved_species_reference(&self, path: &str, species: &str) -> Result<()> {
        validate_saved_exact_catalog_reference(
            path,
            species,
            "pokemon",
            "species id",
            self.saved_species_id(species),
        )
        .map_err(|error| anyhow::anyhow!("{error}"))
    }

    pub fn validate_saved_audio_reference(
        &self,
        path: &str,
        audio_id: &str,
        expected_kind: ModpackAudioKind,
    ) -> Result<()> {
        let asset = self
            .audio
            .iter()
            .find(|asset| asset.id == audio_id)
            .with_context(|| {
                format!(
                    "save field {path} references missing {} audio id '{audio_id}'",
                    expected_kind.save_name()
                )
            })?;
        core_validate_saved_audio_reference(
            path,
            audio_id,
            expected_kind.save_name(),
            Some(asset.kind.save_name()),
        )
        .map_err(|error| anyhow::anyhow!("{error}"))
    }

    pub fn validate_saved_map_constant_reference(
        &self,
        path: &str,
        map_constant: &str,
    ) -> Result<()> {
        validate_saved_exact_catalog_reference(
            path,
            map_constant,
            "map constants",
            "map constant",
            self.saved_map_constant(map_constant),
        )
        .map_err(|error| anyhow::anyhow!("{error}"))
    }

    pub fn validate_saved_text_reference(&self, path: &str, text_label: &str) -> Result<()> {
        validate_saved_catalog_reference(path, text_label, "text", |text_label| {
            self.saved_text_exists(text_label)
        })
        .map_err(|error| anyhow::anyhow!("{error}"))
    }

    pub fn validate_saved_optional_text_reference(
        &self,
        path: &str,
        text_label: &str,
    ) -> Result<()> {
        validate_saved_optional_catalog_reference(path, text_label, "text", |text_label| {
            self.saved_text_exists(text_label)
        })
        .map_err(|error| anyhow::anyhow!("{error}"))
    }

    pub fn validate_saved_special_routine_reference(
        &self,
        path: &str,
        routine: &str,
    ) -> Result<()> {
        validate_saved_catalog_reference(path, routine, "special routines", |routine| {
            self.saved_special_routine_exists(routine)
        })
        .map_err(|error| anyhow::anyhow!("{error}"))
    }

    pub fn validate_saved_spawn_reference(&self, path: &str, spawn_identifier: u16) -> Result<()> {
        validate_saved_exact_catalog_reference(
            path,
            &spawn_identifier.to_string(),
            "runtime spawn points",
            "spawn identifier",
            self.saved_spawn_identifier(spawn_identifier),
        )
        .map_err(|error| anyhow::anyhow!("{error}"))
    }

    pub fn validate_saved_menu_reference(&self, path: &str, menu: &str) -> Result<()> {
        validate_saved_catalog_reference(path, menu, "menus", |menu| self.saved_menu_exists(menu))
            .map_err(|error| anyhow::anyhow!("{error}"))
    }

    pub fn validate_saved_phone_contact_reference(
        &self,
        path: &str,
        contact_id: &str,
    ) -> Result<()> {
        validate_saved_exact_catalog_reference(
            path,
            contact_id,
            "phone contacts",
            "phone contact id",
            self.saved_phone_contact_id(contact_id),
        )
        .map_err(|error| anyhow::anyhow!("{error}"))
    }

    pub fn validate_saved_special_phone_call_reference(
        &self,
        path: &str,
        call_id: &str,
    ) -> Result<()> {
        validate_saved_catalog_reference(path, call_id, "special phone calls", |call_id| {
            self.saved_special_phone_call_exists(call_id)
        })
        .map_err(|error| anyhow::anyhow!("{error}"))
    }

    pub fn validate_saved_npc_trade_reference(&self, path: &str, trade_id: &str) -> Result<()> {
        validate_saved_catalog_reference(path, trade_id, "NPC trades", |trade_id| {
            self.saved_npc_trade_exists(trade_id)
        })
        .map_err(|error| anyhow::anyhow!("{error}"))
    }

    pub fn validate_saved_pokemon_status_reference(&self, path: &str, status: &str) -> Result<()> {
        validate_saved_catalog_reference(path, status, "status declarations", |status| {
            self.saved_pokemon_status_exists(status)
        })
        .map_err(|error| anyhow::anyhow!("{error}"))
    }

    pub fn validate_saved_script_label_reference(
        &self,
        path: &str,
        script_label: &str,
    ) -> Result<()> {
        let _ = self.saved_compiled_script_body(path, script_label)?;
        Ok(())
    }

    pub fn saved_compiled_script_body(
        &self,
        path: &str,
        script_label: &str,
    ) -> Result<&serde_json::Value> {
        let script_body = self.compiled_script_body(script_label);
        validate_saved_catalog_reference(path, script_label, "scripts", |_| script_body.is_some())
            .map_err(|error| anyhow::anyhow!("{error}"))?;
        script_body.with_context(|| {
            format!("saved {path} script label {script_label} validated but is missing")
        })
    }

    pub fn validate_saved_script_command_reference(
        &self,
        path: &str,
        script_label: &str,
        command_index: usize,
    ) -> Result<()> {
        validate_saved_compiled_script_command_reference(
            self.saved_compiled_script_body(path, script_label)?,
            path,
            script_label,
            command_index,
        )
        .map_err(|error| anyhow::anyhow!("{error}"))
    }

    pub fn validate_saved_script_command_name_reference(
        &self,
        path: &str,
        script_label: &str,
        command_index: usize,
        saved_command: &str,
    ) -> Result<()> {
        validate_saved_compiled_script_command_name_reference(
            self.saved_compiled_script_body(path, script_label)?,
            path,
            script_label,
            command_index,
            saved_command,
        )
        .map_err(|error| anyhow::anyhow!("{error}"))
    }

    pub fn validate_saved_script_command_payload_reference(
        &self,
        path: &str,
        script_label: &str,
        command_index: usize,
        saved_command: &str,
        saved_args: &[String],
    ) -> Result<()> {
        validate_saved_compiled_script_command_payload_reference(
            self.saved_compiled_script_body(path, script_label)?,
            path,
            script_label,
            command_index,
            saved_command,
            saved_args,
        )
        .map_err(|error| anyhow::anyhow!("{error}"))
    }

    pub fn validate_saved_elevator_floor_reference(
        &self,
        path: &str,
        floor: &ScriptRuntimeElevatorFloor,
    ) -> Result<()> {
        let map_constant = self.map_constant(&floor.target_map)?;
        let args = vec![
            floor.floor.clone(),
            floor.warp.to_string(),
            map_constant.to_string(),
        ];
        self.validate_saved_script_command_payload_reference(
            path,
            &floor.source_script,
            floor.command_index,
            "elevfloor",
            &args,
        )
    }

    pub fn validate_saved_script_warp_reference(
        &self,
        path: &str,
        warp: &ScriptWarpRequest,
    ) -> Result<()> {
        let raw_tile = runtime_tile_to_raw_event_tile(warp.tile).with_context(|| {
            format!(
                "saved {path} {}:{} pending script warp tile ({}, {}) is not aligned to a raw map event coordinate",
                warp.source_script,
                warp.command_index,
                warp.tile.x,
                warp.tile.y
            )
        })?;
        let mut args = vec![
            self.map_constant(&warp.target_map)?.to_string(),
            raw_tile.x.to_string(),
            raw_tile.y.to_string(),
        ];
        let command = if let Some(facing) = warp.facing {
            args.push(direction_script_token(facing).to_string());
            "warpfacing"
        } else {
            "warp"
        };
        self.validate_saved_script_command_payload_reference(
            path,
            &warp.source_script,
            warp.command_index,
            command,
            &args,
        )
    }

    pub fn validate_saved_script_return_reference(
        &self,
        path: &str,
        script_label: &str,
        next_command_index: usize,
    ) -> Result<()> {
        validate_saved_compiled_script_return_reference(
            self.saved_compiled_script_body(path, script_label)?,
            path,
            script_label,
            next_command_index,
        )
        .map_err(|error| anyhow::anyhow!("{error}"))
    }

    pub fn validate_saved_map_reference<'a>(
        &'a self,
        path: &str,
        map_name: &str,
    ) -> Result<&'a MapModule> {
        core_validate_saved_map_reference(path, map_name, self.saved_map_id(map_name))
            .map_err(|error| anyhow::anyhow!("{error}"))?;
        let module = self
            .maps
            .get(map_name)
            .with_context(|| format!("saved {path} map {map_name} validated but is missing"))?;
        Ok(module)
    }

    pub fn validate_saved_warp_reference(
        &self,
        path: &str,
        map_name: &str,
        warp_index: u16,
    ) -> Result<()> {
        core_validate_saved_warp_reference(
            path,
            map_name,
            warp_index,
            self.saved_map_id(map_name),
            |warp_index| self.saved_warp_exists(map_name, warp_index),
        )
        .map_err(|error| anyhow::anyhow!("{error}"))
    }

    pub fn validate_saved_audio_runtime_event_command(
        &self,
        path: &str,
        event: &crystal_core::state::ScriptAudioRuntimeEvent,
    ) -> Result<()> {
        if event.source_script == "ReceiveItemScript" && event.command == "standard_receive_item" {
            if event.command_index != 0
                || event.kind != ScriptAudioRuntimeKind::SoundEffect
                || event.audio_id.as_deref() != Some("SFX_ITEM")
                || event.fade_frames.is_some()
            {
                anyhow::bail!("saved {path} ReceiveItemScript audio event has invalid shape");
            }
            return Ok(());
        }
        let Some(args) = saved_audio_runtime_event_command_args(path, event)
            .map_err(|error| anyhow::anyhow!("{error}"))?
        else {
            if event.command == "special" {
                return self.validate_saved_special_routine_reference(path, &event.source_script);
            }
            return self.validate_saved_script_command_name_reference(
                path,
                &event.source_script,
                event.command_index,
                &event.command,
            );
        };
        self.validate_saved_script_command_payload_reference(
            path,
            &event.source_script,
            event.command_index,
            &event.command,
            &args,
        )
    }

    pub fn validate_saved_graphics_runtime_event(
        &self,
        path: &str,
        event: &crystal_core::state::ScriptGraphicsRuntimeEvent,
    ) -> Result<()> {
        self.validate_saved_special_routine_reference(path, &event.source_script)?;
        validate_saved_graphics_runtime_event_shape(path, event)
            .map_err(|error| anyhow::anyhow!("{error}"))
    }

    pub fn validate_saved_screen_fade(
        &self,
        path: &str,
        fade: &crystal_core::state::ScriptScreenFade,
    ) -> Result<()> {
        self.validate_saved_special_routine_reference(path, &fade.source_script)?;
        validate_saved_pending_screen_fade_shape(path, fade)
            .map_err(|error| anyhow::anyhow!("{error}"))
    }

    pub fn validate_saved_money_runtime_event(
        &self,
        path: &str,
        event: &crystal_core::state::ScriptMoneyRuntimeEvent,
    ) -> Result<()> {
        self.validate_saved_special_routine_reference(path, &event.source_script)?;
        validate_saved_money_runtime_event_shape(path, event)
            .map_err(|error| anyhow::anyhow!("{error}"))
    }

    pub fn validate_saved_map_runtime_event_command(
        &self,
        path: &str,
        event: &crystal_core::state::ScriptMapRuntimeEvent,
    ) -> Result<()> {
        if self.saved_special_routine_exists(&event.source_script) {
            return self.validate_saved_special_routine_reference(path, &event.source_script);
        }
        let Some(args) = self.saved_map_runtime_event_command_args(path, event)? else {
            return self.validate_saved_script_command_name_reference(
                path,
                &event.source_script,
                event.command_index,
                &event.command,
            );
        };
        self.validate_saved_script_command_payload_reference(
            path,
            &event.source_script,
            event.command_index,
            &event.command,
            &args,
        )
    }

    fn saved_map_runtime_event_command_args(
        &self,
        path: &str,
        event: &crystal_core::state::ScriptMapRuntimeEvent,
    ) -> Result<Option<Vec<String>>> {
        match event.command.as_str() {
            "warp" => {
                let Some(mut args) = saved_map_runtime_event_command_args(path, event)
                    .map_err(|error| anyhow::anyhow!("{error}"))?
                else {
                    return Ok(None);
                };
                if let Some(target_map) = event.target_map.as_deref() {
                    args[0] = self.map_constant(target_map)?.to_string();
                }
                Ok(Some(args))
            }
            "warpfacing" => {
                let Some(mut args) = saved_map_runtime_event_command_args(path, event)
                    .map_err(|error| anyhow::anyhow!("{error}"))?
                else {
                    return Ok(None);
                };
                let target_map = event.target_map.as_deref().with_context(|| {
                    format!(
                        "saved {path} {}:{} warpfacing is missing target map",
                        event.source_script, event.command_index
                    )
                })?;
                args[0] = self.map_constant(target_map)?.to_string();
                Ok(Some(args))
            }
            _ => saved_map_runtime_event_command_args(path, event)
                .map_err(|error| anyhow::anyhow!("{error}")),
        }
    }

    pub fn validate_saved_text_runtime_event_command(
        &self,
        path: &str,
        event: &crystal_core::state::ScriptTextRuntimeEvent,
    ) -> Result<()> {
        if event.source_script == "ReceiveItemScript" && event.command == "standard_receive_item" {
            if event.command_index != 0
                || event.kind != ScriptTextRuntimeKind::Write
                || event.text_label.is_some()
                || event.face_player
                || !event.closes_text
            {
                anyhow::bail!("saved {path} ReceiveItemScript text event has invalid shape");
            }
            return Ok(());
        }
        let Some(args) = saved_text_runtime_event_command_args(path, event)
            .map_err(|error| anyhow::anyhow!("{error}"))?
        else {
            return self.validate_saved_script_command_name_reference(
                path,
                &event.source_script,
                event.command_index,
                &event.command,
            );
        };
        self.validate_saved_script_command_payload_reference(
            path,
            &event.source_script,
            event.command_index,
            &event.command,
            &args,
        )
    }

    pub fn validate_saved_pending_text_wait_command(
        &self,
        runtime: &crystal_core::state::ScriptRuntimeMemory,
        wait: &crystal_core::state::ScriptTextWait,
    ) -> Result<()> {
        if wait.source_script == "ReceiveItemScript" && wait.command == "standard_receive_item" {
            if wait.command_index != 0 {
                anyhow::bail!("ReceiveItemScript pending text wait must use command index 0");
            }
            if !runtime.named_buffers.contains_key("STRING_BUFFER_4") {
                anyhow::bail!("ReceiveItemScript pending text wait requires STRING_BUFFER_4");
            }
            return Ok(());
        }
        let path = "script_runtime.pending_text_wait.source_script";
        let Some(args) =
            saved_pending_text_wait_command_args(path, wait, runtime.pending_text_label.as_deref())
                .map_err(|error| anyhow::anyhow!("{error}"))?
        else {
            return self.validate_saved_script_command_name_reference(
                path,
                &wait.source_script,
                wait.command_index,
                &wait.command,
            );
        };
        self.validate_saved_script_command_payload_reference(
            path,
            &wait.source_script,
            wait.command_index,
            &wait.command,
            &args,
        )
    }

    pub fn validate_saved_script_end_command(
        &self,
        end: &crystal_core::state::ScriptEndState,
    ) -> Result<()> {
        let expected_command =
            saved_script_end_command(end).map_err(|error| anyhow::anyhow!("{error}"))?;
        self.validate_saved_script_command_payload_reference(
            "script_runtime.script_ended.source_script",
            &end.source_script,
            end.command_index,
            expected_command,
            &[],
        )
    }

    pub fn validate_saved_control_runtime_event_command(
        &self,
        path: &str,
        event: &crystal_core::state::ScriptControlRuntimeEvent,
    ) -> Result<()> {
        validate_saved_control_runtime_event_shape(path, event)
            .map_err(|error| anyhow::anyhow!("{error}"))?;
        self.validate_saved_script_command_reference(
            path,
            &event.source_script,
            event.command_index,
        )
    }

    pub fn validate_saved_last_talked_object_reference(
        &self,
        state: &GameState,
        object_id: &str,
    ) -> Result<()> {
        core_validate_saved_last_talked_object_reference(
            state,
            object_id,
            |map_name| self.saved_map_exists(map_name),
            |map_name, object_id| self.map_declares_object(map_name, object_id),
        )
        .map_err(|error| anyhow::anyhow!("{error}"))
    }

    pub fn validate_saved_map_object_reference(
        &self,
        map_name: &str,
        path: &str,
        object_id: &str,
    ) -> Result<()> {
        core_validate_saved_map_object_reference(path, map_name, object_id, |object_id| {
            self.map_declares_object(map_name, object_id)
        })
        .map_err(|error| anyhow::anyhow!("{error}"))
    }

    pub fn validate_saved_pokemon_party_references(
        &self,
        path: &str,
        party: &[Pokemon],
    ) -> Result<()> {
        core_validate_saved_pokemon_party_references(path, party, |path, pokemon| {
            self.validate_saved_pokemon_reference(path, pokemon)
                .map_err(|error| error.to_string())
        })
        .map_err(|error| anyhow::anyhow!("{error}"))
    }

    pub fn validate_saved_trainer_battle_origin_references(
        &self,
        trainer: &Trainer,
        battle_type: &str,
        trainer_class: &str,
        event_flag: &str,
        seen_text: &str,
        win_text: &str,
        loss_text: &str,
        callback: &str,
        source_script: &str,
    ) -> Result<()> {
        if let Some(scripted) =
            self.saved_scripted_trainer_battle(source_script, &trainer.trainer_id)
        {
            let request = &scripted.request;
            validate_saved_trainer_battle_request_fields(
                SavedTrainerBattleFields {
                    battle_type,
                    trainer_class,
                    event_flag,
                    seen_text,
                    win_text,
                    loss_text,
                    callback,
                },
                SavedTrainerBattleFields {
                    battle_type: &request.battle_type,
                    trainer_class: &request.trainer_class,
                    event_flag: &request.event_flag,
                    seen_text: &request.seen_text,
                    win_text: &request.win_text,
                    loss_text: &request.loss_text,
                    callback: &request.callback,
                },
                source_script,
            )
            .map_err(|error| anyhow::anyhow!("{error}"))?;
            self.validate_saved_optional_text_reference("battle.trainer.seen_text", seen_text)?;
            self.validate_saved_optional_text_reference("battle.trainer.win_text", win_text)?;
            self.validate_saved_optional_text_reference("battle.trainer.loss_text", loss_text)?;
            return Ok(());
        }

        if self.saved_special_routine_exists(source_script) {
            core_validate_saved_trainer_battle_request_field(
                "event_flag",
                event_flag,
                "",
                source_script,
            )
            .map_err(|error| anyhow::anyhow!("{error}"))?;
            core_validate_saved_trainer_battle_request_field(
                "seen_text",
                seen_text,
                "",
                source_script,
            )
            .map_err(|error| anyhow::anyhow!("{error}"))?;
            core_validate_saved_trainer_battle_request_field(
                "win_text",
                win_text,
                &trainer.win_quote,
                source_script,
            )
            .map_err(|error| anyhow::anyhow!("{error}"))?;
            core_validate_saved_trainer_battle_request_field(
                "loss_text",
                loss_text,
                &trainer.lose_quote,
                source_script,
            )
            .map_err(|error| anyhow::anyhow!("{error}"))?;
            core_validate_saved_trainer_battle_request_field(
                "callback",
                callback,
                "",
                source_script,
            )
            .map_err(|error| anyhow::anyhow!("{error}"))?;
            return Ok(());
        }

        validate_saved_trainer_battle_source_reference(source_script, |_| false)
            .map_err(|error| anyhow::anyhow!("{error}"))
    }

    pub fn validate_saved_static_wild_battle_origin_references(
        &self,
        battle_type: &str,
        species: &str,
        level: u8,
        source_script: &str,
    ) -> Result<()> {
        core_validate_saved_static_wild_battle_origin_reference(
            battle_type,
            species,
            level,
            source_script,
            |source_script, battle_type, species, level| {
                self.saved_static_wild_battle_origin_exists(
                    source_script,
                    battle_type,
                    species,
                    level,
                )
            },
        )
        .map_err(|error| anyhow::anyhow!("{error}"))
    }

    pub fn validate_saved_wild_battle_origin_references(
        &self,
        battle_type: &str,
        map_name: &str,
        enemy_pokemon: &Pokemon,
    ) -> Result<()> {
        core_validate_saved_wild_battle_origin_reference(
            battle_type,
            map_name,
            enemy_pokemon,
            |map_name, species, level| {
                self.saved_wild_encounter_exists(map_name, species, level)
                    || self
                        .roaming_pokemon
                        .get(species)
                        .is_some_and(|definition| definition.level == level)
            },
        )
        .map_err(|error| anyhow::anyhow!("{error}"))
    }

    pub fn validate_saved_trainer_enemy_party(
        &self,
        trainer: &Trainer,
        enemy_party: &[Pokemon],
        enemy_pokemon: &Pokemon,
    ) -> Result<()> {
        let expected_party = materialize_trainer_party(
            trainer,
            &self.pokemon,
            &self.learnsets,
            &self.moves,
            &self.growth_rates,
        )
        .map_err(|error| {
            anyhow::anyhow!(
                "compiled trainer {} party is invalid: {error}",
                trainer.trainer_id
            )
        })?;
        validate_saved_trainer_enemy_party_identity(
            &trainer.trainer_id,
            enemy_party,
            enemy_pokemon,
            &expected_party,
        )
        .map_err(|error| anyhow::anyhow!("{error}"))
    }

    pub fn saved_map_exists(&self, map_name: &str) -> bool {
        self.maps.contains_key(map_name)
    }

    pub fn saved_map_dimensions(&self, map_name: &str) -> Option<(u16, u16)> {
        self.maps
            .get(map_name)
            .map(|module| (module.attributes.width, module.attributes.height))
    }

    pub fn saved_map_tile_bounds(&self, map_name: &str) -> Option<(u16, u16)> {
        let width_multiplier = u16::try_from(METATILE_WIDTH).ok()?;
        self.saved_map_dimensions(map_name)
            .and_then(|(width, height)| {
                Some((
                    width.checked_mul(width_multiplier)?,
                    height.checked_mul(width_multiplier)?,
                ))
            })
    }

    pub fn saved_map_block_context(&self, map_name: &str) -> Option<(u16, u16, String)> {
        self.maps.get(map_name).map(|module| {
            (
                module.attributes.width,
                module.attributes.height,
                module.attributes.tileset_name.clone(),
            )
        })
    }

    pub fn saved_warp_exists(&self, map_name: &str, warp_index: u16) -> bool {
        self.maps.get(map_name).is_some_and(|module| {
            module
                .events
                .warps
                .iter()
                .any(|warp| warp.index == warp_index)
        })
    }

    pub fn saved_elevator_pending_warp_exists(&self, warp: &ScriptWarpRequest) -> bool {
        self.maps.values().any(|module| {
            module.script_elevators.values().any(|elevator| {
                elevator.source_script == warp.source_script.as_str()
                    && elevator.elevator_command_index == warp.command_index
                    && elevator.floors.iter().any(|floor| {
                        floor.target_map == warp.target_map.as_str()
                            && self.maps.get(&floor.target_map).is_some_and(|target| {
                                target.events.warps.iter().any(|target_warp| {
                                    target_warp.index == floor.warp
                                        && checked_runtime_map_event_tile(
                                            target_warp.x,
                                            target_warp.y,
                                        )
                                        .is_some_and(|tile| tile == warp.tile)
                                })
                            })
                    })
            })
        })
    }

    pub fn saved_scene_index(&self, map_name: &str, scene_name: &str) -> Option<usize> {
        self.maps.get(map_name).and_then(|module| {
            module
                .scenes
                .scenes
                .iter()
                .enumerate()
                .find(|(_, scene)| scene.scene_id == scene_name)
                .map(|(index, _)| index)
        })
    }

    pub fn compiled_script_body(&self, script_label: &str) -> Option<&serde_json::Value> {
        self.maps
            .values()
            .find_map(|module| module.scripts.get(script_label))
    }

    pub fn compiled_standard_script_body(&self, script_label: &str) -> Result<&[Value]> {
        validate_compiled_standard_script_catalog(self)?;
        let catalog = compiled_standard_script_catalog(self)?;
        let pointer_table = catalog
            .get("StdScripts")
            .and_then(Value::as_array)
            .context("compiled StandardScripts catalog is missing the StdScripts pointer table")?;
        let declared = pointer_table.iter().any(|entry| {
            entry.get("command").and_then(Value::as_str) == Some("add_stdscript")
                && entry
                    .get("args")
                    .and_then(Value::as_array)
                    .is_some_and(|args| args.len() == 1 && args[0].as_str() == Some(script_label))
        });
        if !declared {
            anyhow::bail!(
                "compiled standard script {script_label} is not declared by the StdScripts pointer table"
            );
        }
        catalog
            .get(script_label)
            .and_then(Value::as_array)
            .map(Vec::as_slice)
            .with_context(|| {
                format!("compiled StdScripts pointer {script_label} has no command body")
            })
    }

    pub fn map_declares_object(&self, map_name: &str, object_id: &str) -> bool {
        self.maps.get(map_name).is_some_and(|module| {
            module
                .objects
                .iter()
                .any(|object| object.object_identifier.as_deref() == Some(object_id))
        })
    }

    pub fn map_declares_event_object(&self, map_name: &str, object_id: &str) -> bool {
        self.maps.get(map_name).is_some_and(|module| {
            module
                .objects
                .iter()
                .any(|object| object.object_identifier.as_deref() == Some(object_id))
        })
    }

    pub fn tileset_declares_metatile(&self, tileset_name: &str, block_id: u16) -> bool {
        self.tilesets
            .get(tileset_name)
            .is_some_and(|tileset| tileset_declares_metatile(tileset, block_id))
    }

    pub fn saved_tileset_exists(&self, tileset_name: &str) -> bool {
        self.tilesets.contains_key(tileset_name)
    }

    pub fn item(&self, item_id: &str) -> Result<&Item> {
        self.items
            .get(item_id)
            .with_context(|| format!("compiled game pack missing item {item_id}"))
    }

    pub fn use_bag_item(
        &self,
        state: &mut GameState,
        item_id: &str,
        context: ItemUseContext,
    ) -> Result<ItemUseOutcome> {
        core_use_bag_item(
            state,
            &self.items,
            ItemUseRequest {
                item_id: item_id.to_string(),
                context,
            },
        )
        .map_err(|error| anyhow::anyhow!("use bag item {item_id}: {error:?}"))
    }

    pub fn field_repel_steps(&self, item_id: &str) -> Result<u16> {
        let item = self.item(item_id)?;
        validate_repel_item(&self.field_moves, item)
            .map_err(|error| anyhow::anyhow!("validate field repel item {item_id}: {error:?}"))
    }

    pub fn field_bicycle_item(&self, item_id: &str) -> Result<&Item> {
        let item = self.item(item_id)?;
        validate_bicycle_item(&self.field_moves, item)
            .map_err(|error| anyhow::anyhow!("validate field bicycle item {item_id}: {error:?}"))?;
        Ok(item)
    }

    pub fn field_itemfinder_item(&self, item_id: &str) -> Result<&Item> {
        let item = self.item(item_id)?;
        validate_itemfinder_item(&self.field_moves, item).map_err(|error| {
            anyhow::anyhow!("validate field itemfinder item {item_id}: {error:?}")
        })?;
        Ok(item)
    }

    pub fn field_squirtbottle_item(&self, item_id: &str) -> Result<&Item> {
        let item = self.item(item_id)?;
        validate_squirtbottle_item(&self.field_moves, item).map_err(|error| {
            anyhow::anyhow!("validate field squirtbottle item {item_id}: {error:?}")
        })?;
        Ok(item)
    }

    pub fn field_coin_case_item(&self, item_id: &str) -> Result<&Item> {
        let item = self.item(item_id)?;
        validate_coin_case_item(&self.field_moves, item).map_err(|error| {
            anyhow::anyhow!("validate field coin case item {item_id}: {error:?}")
        })?;
        Ok(item)
    }

    pub fn field_blue_card_item(&self, item_id: &str) -> Result<&Item> {
        let item = self.item(item_id)?;
        validate_blue_card_item(&self.field_moves, item).map_err(|error| {
            anyhow::anyhow!("validate field blue card item {item_id}: {error:?}")
        })?;
        Ok(item)
    }

    pub fn field_town_map_item(&self, item_id: &str) -> Result<&Item> {
        let item = self.item(item_id)?;
        validate_town_map_item(&self.field_moves, item).map_err(|error| {
            anyhow::anyhow!("validate field town map item {item_id}: {error:?}")
        })?;
        Ok(item)
    }

    pub fn field_pokegear_item(&self, item_id: &str) -> Result<&Item> {
        let item = self.item(item_id)?;
        validate_pokegear_item(&self.field_moves, item).map_err(|error| {
            anyhow::anyhow!("validate field pokegear item {item_id}: {error:?}")
        })?;
        Ok(item)
    }

    pub fn field_box_item(&self, item_id: &str) -> Result<(&Item, &FieldBoxItemRule)> {
        let rule = self
            .field_box_items
            .get(item_id)
            .with_context(|| format!("field box item {item_id} is not defined by the pack"))?;
        let item = self.item(item_id)?;
        if item.effect != rule.effect {
            anyhow::bail!(
                "field box item {item_id} effect {} does not match pack rule effect {}",
                item.effect,
                rule.effect
            );
        }
        if item.field_menu != "ITEMMENU_CURRENT" {
            anyhow::bail!(
                "field box item {item_id} has field_menu {}, expected ITEMMENU_CURRENT",
                item.field_menu
            );
        }
        Ok((item, rule))
    }

    pub fn field_escape_item(&self, item_id: &str) -> Result<&Item> {
        let item = self.item(item_id)?;
        validate_field_escape_item(&self.field_moves, item)
            .map_err(|error| anyhow::anyhow!("use field escape item {item_id}: {error:?}"))?;
        Ok(item)
    }

    fn require_field_usable_item_in_bag<'a>(
        state: &GameState,
        item_id: &str,
        item: &'a Item,
        context: &str,
    ) -> Result<&'a Item> {
        if !item.field_usable {
            anyhow::bail!("field {context} item {item_id} is not usable in the field");
        }
        if !state.bag.has_item(item) {
            anyhow::bail!("field {context} item {item_id} is not in the bag");
        }
        Ok(item)
    }

    pub fn use_bag_repel_in_field(
        &self,
        state: &mut GameState,
        item_id: &str,
    ) -> Result<FieldRepelItemUseOutcome> {
        self.require_no_active_battle(state, "field repel item")?;
        if state.repel_steps_remaining > 0 {
            anyhow::bail!("the repel used earlier is still in effect");
        }
        let steps = self.field_repel_steps(item_id)?;
        let item = self.item(item_id)?;
        Self::require_field_usable_item_in_bag(state, item_id, item, "repel")?;
        let item_use = self.use_bag_item(state, item_id, ItemUseContext::Field)?;
        let repel = apply_repel_item_use(state, item_id, steps);
        Ok(FieldRepelItemUseOutcome {
            item_use,
            repel_steps_before: repel.repel_steps_before,
            repel_steps_after: repel.repel_steps_after,
            active_repel_item_before: repel.active_repel_item_before,
            active_repel_item_after: repel.active_repel_item_after,
        })
    }

    pub fn use_bag_bicycle_in_field(
        &self,
        state: &mut GameState,
        overworld: &mut OverworldSession,
        item_id: &str,
    ) -> Result<FieldBicycleItemUseOutcome> {
        self.require_no_active_battle(state, "field bicycle item")?;
        let item = self.field_bicycle_item(item_id)?;
        Self::require_field_usable_item_in_bag(state, item_id, item, "bicycle")?;
        let map_name = overworld.map.name.clone();
        let environment = self.map_environment(&map_name)?;
        if !is_bicycle_environment(environment) {
            anyhow::bail!("cannot use field bicycle item {item_id} in environment {environment}");
        }
        let sample = sample_collision(&overworld.map, &overworld.tileset, overworld.player.tile)
            .with_context(|| {
                format!(
                    "field bicycle item {item_id} cannot sample current tile {},{}",
                    overworld.player.tile.x, overworld.player.tile.y
                )
            })?;
        if sample.permission & 0x0f != permissions::FLOOR {
            anyhow::bail!(
                "cannot use field bicycle item {item_id} on permission {:#04x}",
                sample.permission
            );
        }
        let mode_before = overworld.player.mode;
        let always_on_bike = state
            .flags
            .is_engine_flag_set("ENGINE_ALWAYS_ON_BIKE")
            .context("check ENGINE_ALWAYS_ON_BIKE")?;
        let mode_after = match mode_before {
            MovementMode::Normal => MovementMode::Bike,
            MovementMode::Bike if always_on_bike => {
                anyhow::bail!("cannot get off bicycle while ENGINE_ALWAYS_ON_BIKE is set");
            }
            MovementMode::Bike => MovementMode::Normal,
            MovementMode::Skate | MovementMode::Surf | MovementMode::SurfPika => {
                anyhow::bail!("cannot toggle bicycle from movement mode {mode_before:?}");
            }
        };
        let music = match mode_after {
            MovementMode::Bike => Some("MUSIC_BICYCLE".to_string()),
            MovementMode::Normal => self.map_music(&map_name)?.map(str::to_owned),
            MovementMode::Skate | MovementMode::Surf | MovementMode::SurfPika => {
                anyhow::bail!("bicycle toggle finished in invalid mode {mode_after:?}")
            }
        };
        let item_use = self.use_bag_item(state, item_id, ItemUseContext::Field)?;
        overworld.player.mode = mode_after;
        commit_overworld_snapshot(state, &overworld.snapshot(), SpawnMemoryUpdate::Preserve);
        apply_map_music_context(state, music);
        Ok(FieldBicycleItemUseOutcome {
            item_use,
            map_name,
            permission: sample.permission,
            mode_before,
            mode_after,
        })
    }

    pub fn use_bag_itemfinder_in_field(
        &self,
        state: &mut GameState,
        overworld: &OverworldSession,
        item_id: &str,
    ) -> Result<FieldItemfinderUseOutcome> {
        self.require_no_active_battle(state, "field itemfinder item")?;
        let item = self.field_itemfinder_item(item_id)?;
        Self::require_field_usable_item_in_bag(state, item_id, item, "itemfinder")?;
        let found =
            self.find_itemfinder_hidden_item(state, &overworld.map.name, overworld.player.tile)?;
        let itemfinder_sound_cues = if found.is_some() { 8 } else { 0 };
        let item_use = self.use_bag_item(state, item_id, ItemUseContext::Field)?;
        Ok(FieldItemfinderUseOutcome {
            item_use,
            player_tile: overworld.player.tile,
            found,
            itemfinder_sound_cues,
        })
    }

    pub fn use_bag_squirtbottle_in_field(
        &self,
        state: &mut GameState,
        overworld: &mut OverworldSession,
        item_id: &str,
    ) -> Result<FieldSquirtBottleUseOutcome> {
        self.require_no_active_battle(state, "field squirtbottle item")?;
        let item = self.field_squirtbottle_item(item_id)?;
        Self::require_field_usable_item_in_bag(state, item_id, item, "squirtbottle")?;
        let script_labels = self.map_script_labels(&overworld.map.name)?;
        let target =
            resolve_squirtbottle_target(overworld, |script| script_labels.contains(script))
                .map_err(|error| anyhow::anyhow!("{error}"))?;
        let item_use = self.use_bag_item(state, item_id, ItemUseContext::Field)?;
        if let Some(script) = target.target_script.as_ref() {
            commit_interaction_script_dispatch(
                state,
                &mut overworld.last_talked_object_identifier,
                &overworld.map.name,
                script,
                target.target_object_identifier.as_deref(),
            )
            .map_err(|error| {
                anyhow::anyhow!(
                    "dispatch field squirtbottle item {item_id} script {script}: {error:?}"
                )
            })?;
            commit_overworld_snapshot(state, &overworld.snapshot(), SpawnMemoryUpdate::Preserve);
        }
        Ok(FieldSquirtBottleUseOutcome {
            item_use,
            player_tile: overworld.player.tile,
            target_tile: target.target_tile,
            target_object_identifier: target.target_object_identifier,
            target_movement: target.target_movement,
            target_script: target.target_script,
        })
    }

    pub fn use_bag_coin_case_in_field(
        &self,
        state: &mut GameState,
        item_id: &str,
    ) -> Result<FieldKeyItemBalanceUseOutcome> {
        self.require_no_active_battle(state, "field coin case item")?;
        let item = self.field_coin_case_item(item_id)?;
        Self::require_field_usable_item_in_bag(state, item_id, item, "coin case")?;
        let balance = u32::from(state.coins);
        let item_use = self.use_bag_item(state, item_id, ItemUseContext::Field)?;
        Ok(FieldKeyItemBalanceUseOutcome {
            item_use,
            balance_label: "COIN".to_string(),
            balance,
        })
    }

    pub fn use_bag_blue_card_in_field(
        &self,
        state: &mut GameState,
        item_id: &str,
    ) -> Result<FieldKeyItemBalanceUseOutcome> {
        self.require_no_active_battle(state, "field blue card item")?;
        let item = self.field_blue_card_item(item_id)?;
        Self::require_field_usable_item_in_bag(state, item_id, item, "blue card")?;
        let balance = u32::from(
            blue_card_balance(state)
                .map_err(|error| anyhow::anyhow!("read field blue card balance: {error:?}"))?,
        );
        let item_use = self.use_bag_item(state, item_id, ItemUseContext::Field)?;
        Ok(FieldKeyItemBalanceUseOutcome {
            item_use,
            balance_label: "POINT".to_string(),
            balance,
        })
    }

    pub fn use_bag_town_map_in_field(
        &self,
        state: &mut GameState,
        overworld: &OverworldSession,
        item_id: &str,
    ) -> Result<FieldTownMapUseOutcome> {
        self.require_no_active_battle(state, "field town map item")?;
        let item = self.field_town_map_item(item_id)?;
        Self::require_field_usable_item_in_bag(state, item_id, item, "town map")?;
        let map_name = overworld.map.name.clone();
        let map_constant = self.map_constant(&map_name)?.to_string();
        let environment = self.map_environment(&map_name)?.to_string();
        let landmark = self.pokegear_landmark_for_map(&map_name)?.clone();
        let item_use = self.use_bag_item(state, item_id, ItemUseContext::Field)?;
        Ok(FieldTownMapUseOutcome {
            item_use,
            map_name,
            map_constant,
            environment,
            landmark,
        })
    }

    pub fn use_bag_pokegear_in_field(
        &self,
        state: &mut GameState,
        item_id: &str,
    ) -> Result<FieldPokegearUseOutcome> {
        self.require_no_active_battle(state, "field pokegear item")?;
        let item = self.field_pokegear_item(item_id)?;
        Self::require_field_usable_item_in_bag(state, item_id, item, "pokegear")?;
        let item_use = self.use_bag_item(state, item_id, ItemUseContext::Field)?;
        Ok(FieldPokegearUseOutcome { item_use })
    }

    pub fn use_bag_box_in_field(
        &self,
        state: &mut GameState,
        item_id: &str,
    ) -> Result<FieldBoxItemUseOutcome> {
        self.require_no_active_battle(state, "field box item")?;
        let (item, rule) = self.field_box_item(item_id)?;
        Self::require_field_usable_item_in_bag(state, item_id, item, "box")?;
        let decoration_flag = rule.decoration_flag.as_str();
        let already_owned = state
            .flags
            .is_event_flag_set(decoration_flag)
            .with_context(|| format!("check field box decoration flag {decoration_flag}"))?;
        state
            .flags
            .set_event_flag(decoration_flag, true)
            .with_context(|| format!("set field box decoration flag {decoration_flag}"))?;
        let item_use = self.use_bag_item(state, item_id, ItemUseContext::Field)?;
        Ok(FieldBoxItemUseOutcome {
            item_use,
            decoration_flag: decoration_flag.to_string(),
            already_owned,
        })
    }

    pub fn use_bag_escape_rope_in_field(
        &self,
        state: &mut GameState,
        overworld: &OverworldSession,
        item_id: &str,
    ) -> Result<FieldEscapeRopeUseOutcome> {
        self.require_no_active_battle(state, "field escape item")?;
        let item = self.field_escape_item(item_id)?;
        Self::require_field_usable_item_in_bag(state, item_id, item, "escape")?;
        let source_map = overworld.map.name.clone();
        let current_environment = self.map_environment(&source_map)?;
        if !is_escape_rope_environment(current_environment) {
            anyhow::bail!(
                "cannot use field escape item {item_id} in environment {current_environment}"
            );
        }
        let destination =
            self.saved_dig_warp_destination(state, &format!("field escape item {item_id}"))?;
        let item_use = self.use_bag_item(state, item_id, ItemUseContext::Field)?;
        Ok(FieldEscapeRopeUseOutcome {
            item_use,
            source_map,
            destination_map: destination.map_name,
            destination_warp_index: destination.warp_index,
            destination_tile: destination.tile,
        })
    }

    pub fn use_bag_escape_rope_in_session(
        &self,
        state: &mut GameState,
        overworld: &mut OverworldSession,
        item_id: &str,
        music_ids: &BTreeSet<String>,
    ) -> Result<FieldEscapeRopeUseOutcome> {
        let mut staged_state = state.clone();
        let mut staged_overworld = overworld.clone();
        let outcome =
            self.use_bag_escape_rope_in_field(&mut staged_state, &staged_overworld, item_id)?;
        self.transition_overworld_session(
            &mut staged_state,
            &mut staged_overworld,
            &outcome.destination_map,
            outcome.destination_tile,
            SpawnMemoryUpdate::Preserve,
            music_ids,
        )?;
        *state = staged_state;
        *overworld = staged_overworld;
        Ok(outcome)
    }

    pub fn apply_cut_field_move(
        &self,
        state: &mut GameState,
        storage: &PokemonStorage,
        map: &mut OverworldMapData,
        tileset: &TilesetCollision,
        party_index: usize,
        metatile_x: u16,
        metatile_y: u16,
    ) -> Result<FieldMoveBlockOutcome> {
        let tileset_name = self.map_tileset_name(&map.name)?;
        core_apply_cut_field_move(
            &self.field_moves,
            state,
            storage,
            map,
            tileset,
            tileset_name,
            party_index,
            metatile_x,
            metatile_y,
        )
        .map_err(anyhow::Error::new)
        .context("use CUT field move")
    }

    pub fn field_block_target_metatile_in_front(
        &self,
        overworld: &OverworldSession,
    ) -> Result<(u16, u16)> {
        let target = Self::checked_runtime_field_move_target(
            "BLOCK_FIELD_MOVE",
            overworld.player.tile,
            overworld.player.facing,
        )?;
        let (width, height) = overworld.map.checked_tile_bounds().with_context(|| {
            format!(
                "map {} runtime tile bounds overflow supported coordinate range",
                overworld.map.name
            )
        })?;
        if target.x < 0
            || target.y < 0
            || i32::from(target.x) >= i32::from(width)
            || i32::from(target.y) >= i32::from(height)
        {
            anyhow::bail!(
                "field block target tile ({}, {}) is outside map {} runtime tile bounds {width}x{height}",
                target.x,
                target.y,
                overworld.map.name
            );
        }
        let metatile_x = target.x.div_euclid(METATILE_WIDTH);
        let metatile_y = target.y.div_euclid(METATILE_WIDTH);
        Ok((
            u16::try_from(metatile_x).with_context(|| {
                format!("field block target metatile x {metatile_x} cannot be represented")
            })?,
            u16::try_from(metatile_y).with_context(|| {
                format!("field block target metatile y {metatile_y} cannot be represented")
            })?,
        ))
    }

    pub fn use_cut_field_move(
        &self,
        state: &mut GameState,
        overworld: &mut OverworldSession,
        party_index: usize,
        metatile_x: u16,
        metatile_y: u16,
    ) -> Result<FieldMoveBlockOutcome> {
        self.require_no_active_battle(state, "CUT field move")?;
        let storage = state.storage.clone();
        self.apply_cut_field_move(
            state,
            &storage,
            &mut overworld.map,
            &overworld.tileset,
            party_index,
            metatile_x,
            metatile_y,
        )
    }

    pub fn use_cut_field_move_in_front(
        &self,
        state: &mut GameState,
        overworld: &mut OverworldSession,
        party_index: usize,
    ) -> Result<FieldMoveBlockOutcome> {
        let (metatile_x, metatile_y) = self.field_block_target_metatile_in_front(overworld)?;
        self.use_cut_field_move(state, overworld, party_index, metatile_x, metatile_y)
    }

    pub fn apply_whirlpool_field_move(
        &self,
        state: &mut GameState,
        storage: &PokemonStorage,
        map: &mut OverworldMapData,
        tileset: &TilesetCollision,
        party_index: usize,
        metatile_x: u16,
        metatile_y: u16,
    ) -> Result<FieldMoveBlockOutcome> {
        let tileset_name = self.map_tileset_name(&map.name)?;
        core_apply_whirlpool_field_move(
            &self.field_moves,
            state,
            storage,
            map,
            tileset,
            tileset_name,
            party_index,
            metatile_x,
            metatile_y,
        )
        .map_err(anyhow::Error::new)
        .context("use WHIRLPOOL field move")
    }

    pub fn use_whirlpool_field_move(
        &self,
        state: &mut GameState,
        overworld: &mut OverworldSession,
        party_index: usize,
        metatile_x: u16,
        metatile_y: u16,
    ) -> Result<FieldMoveBlockOutcome> {
        self.require_no_active_battle(state, "WHIRLPOOL field move")?;
        let storage = state.storage.clone();
        self.apply_whirlpool_field_move(
            state,
            &storage,
            &mut overworld.map,
            &overworld.tileset,
            party_index,
            metatile_x,
            metatile_y,
        )
    }

    pub fn use_whirlpool_field_move_in_front(
        &self,
        state: &mut GameState,
        overworld: &mut OverworldSession,
        party_index: usize,
    ) -> Result<FieldMoveBlockOutcome> {
        let (metatile_x, metatile_y) = self.field_block_target_metatile_in_front(overworld)?;
        self.use_whirlpool_field_move(state, overworld, party_index, metatile_x, metatile_y)
    }

    pub fn apply_strength_field_move(
        &self,
        state: &mut GameState,
        storage: &PokemonStorage,
        party_index: usize,
    ) -> Result<FieldMoveFlagOutcome> {
        core_apply_strength_field_move(&self.field_moves, state, storage, party_index)
            .map_err(anyhow::Error::new)
            .context("use STRENGTH field move")
    }

    pub fn use_strength_field_move(
        &self,
        state: &mut GameState,
        party_index: usize,
    ) -> Result<FieldMoveFlagOutcome> {
        self.require_no_active_battle(state, "STRENGTH field move")?;
        let storage = state.storage.clone();
        self.apply_strength_field_move(state, &storage, party_index)
    }

    pub fn apply_flash_field_move(
        &self,
        state: &mut GameState,
        storage: &PokemonStorage,
        party_index: usize,
    ) -> Result<FieldMoveFlagOutcome> {
        core_apply_flash_field_move(&self.field_moves, state, storage, party_index)
            .map_err(anyhow::Error::new)
            .context("use FLASH field move")
    }

    pub fn use_flash_field_move(
        &self,
        state: &mut GameState,
        party_index: usize,
    ) -> Result<FieldMoveFlagOutcome> {
        self.require_no_active_battle(state, "FLASH field move")?;
        let storage = state.storage.clone();
        self.apply_flash_field_move(state, &storage, party_index)
    }

    pub fn apply_surf_field_move(
        &self,
        state: &GameState,
        storage: &PokemonStorage,
        map: &OverworldMapData,
        tileset: &TilesetCollision,
        player: &mut PlayerMovementState,
        party_index: usize,
    ) -> Result<FieldMoveTravelOutcome> {
        core_apply_surf_field_move(
            &self.field_moves,
            state,
            storage,
            map,
            tileset,
            player,
            party_index,
        )
        .map_err(anyhow::Error::new)
        .context("use SURF field move")
    }

    pub fn use_surf_field_move(
        &self,
        state: &mut GameState,
        overworld: &mut OverworldSession,
        party_index: usize,
    ) -> Result<FieldMoveTravelOutcome> {
        let mut staged_state = state.clone();
        let mut staged_overworld = overworld.clone();
        self.require_no_active_battle(&staged_state, "SURF field move")?;
        let target = Self::checked_runtime_field_move_target(
            "SURF",
            staged_overworld.player.tile,
            staged_overworld.player.facing,
        )?;
        if let Some((_, object)) = staged_overworld
            .visible_object_at_checked(target)
            .with_context(|| {
                format!(
                    "check SURF target occupancy on {}",
                    staged_overworld.map.name
                )
            })?
        {
            anyhow::bail!(
                "cannot use SURF field move onto occupied tile {target:?} by {:?}",
                object.object_identifier
            );
        }
        let storage = staged_state.storage.clone();
        let state_snapshot = staged_state.clone();
        let outcome = self.apply_surf_field_move(
            &state_snapshot,
            &storage,
            &staged_overworld.map,
            &staged_overworld.tileset,
            &mut staged_overworld.player,
            party_index,
        )?;
        commit_overworld_snapshot(
            &mut staged_state,
            &staged_overworld.snapshot(),
            SpawnMemoryUpdate::Preserve,
        );
        apply_map_music_context(&mut staged_state, Some("MUSIC_SURF".to_string()));
        *state = staged_state;
        *overworld = staged_overworld;
        Ok(outcome)
    }

    pub fn apply_waterfall_field_move(
        &self,
        state: &GameState,
        storage: &PokemonStorage,
        map: &OverworldMapData,
        tileset: &TilesetCollision,
        player: &mut PlayerMovementState,
        party_index: usize,
    ) -> Result<FieldMoveTravelOutcome> {
        core_apply_waterfall_field_move(
            &self.field_moves,
            state,
            storage,
            map,
            tileset,
            player,
            party_index,
        )
        .map_err(anyhow::Error::new)
        .context("use WATERFALL field move")
    }

    pub fn use_waterfall_field_move(
        &self,
        state: &mut GameState,
        overworld: &mut OverworldSession,
        party_index: usize,
    ) -> Result<FieldMoveTravelOutcome> {
        let mut staged_state = state.clone();
        let mut staged_overworld = overworld.clone();
        self.require_no_active_battle(&staged_state, "WATERFALL field move")?;
        let storage = staged_state.storage.clone();
        let state_snapshot = staged_state.clone();
        let outcome = self.apply_waterfall_field_move(
            &state_snapshot,
            &storage,
            &staged_overworld.map,
            &staged_overworld.tileset,
            &mut staged_overworld.player,
            party_index,
        )?;
        commit_overworld_snapshot(
            &mut staged_state,
            &staged_overworld.snapshot(),
            SpawnMemoryUpdate::Preserve,
        );
        *state = staged_state;
        *overworld = staged_overworld;
        Ok(outcome)
    }

    pub fn validate_fly_field_move(
        &self,
        state: &GameState,
        source_map: &str,
        party_index: usize,
    ) -> Result<FieldMoveUseOutcome> {
        let source_environment = self.map_environment(source_map)?;
        if !is_fly_source_environment(source_environment) {
            anyhow::bail!("cannot use FLY field move in environment {source_environment}");
        }
        core_validate_fly_field_move(&self.field_moves, state, &state.storage, party_index)
            .map_err(anyhow::Error::new)
            .context("use FLY field move")
    }

    pub fn use_fly_field_move(
        &self,
        state: &GameState,
        source_map: &str,
        party_index: usize,
        destination_spawn_identifier: u16,
        flypoint_flag: &str,
    ) -> Result<FlyFieldMoveOutcome> {
        self.require_no_active_battle(state, "FLY field move")?;
        let fly_rule = self.validate_fly_field_move(state, source_map, party_index)?;
        if !state
            .flags
            .is_engine_flag_set(flypoint_flag)
            .with_context(|| format!("check FLY destination flag {flypoint_flag}"))?
        {
            anyhow::bail!("FLY destination flag {flypoint_flag} is not set");
        }
        let destination_spawn = self.runtime_spawn_point(destination_spawn_identifier)?;
        Ok(FlyFieldMoveOutcome {
            actor_party_index: fly_rule.actor_party_index,
            actor_species: fly_rule.actor_species,
            flypoint_flag: flypoint_flag.to_string(),
            source_map: source_map.to_string(),
            destination_spawn_identifier,
            destination_map: destination_spawn.map_name.clone(),
            destination_tile: runtime_spawn_expected_tile(destination_spawn),
        })
    }

    pub fn use_fly_field_move_in_session(
        &self,
        state: &mut GameState,
        overworld: &mut OverworldSession,
        party_index: usize,
        destination_spawn_identifier: u16,
        flypoint_flag: &str,
        music_ids: &BTreeSet<String>,
    ) -> Result<FlyFieldMoveOutcome> {
        let mut staged_state = state.clone();
        let mut staged_overworld = overworld.clone();
        let source_map = staged_overworld.map.name.clone();
        let outcome = self.use_fly_field_move(
            &staged_state,
            &source_map,
            party_index,
            destination_spawn_identifier,
            flypoint_flag,
        )?;
        self.transition_overworld_session(
            &mut staged_state,
            &mut staged_overworld,
            &outcome.destination_map,
            outcome.destination_tile,
            SpawnMemoryUpdate::Set(destination_spawn_identifier),
            music_ids,
        )?;
        *state = staged_state;
        *overworld = staged_overworld;
        Ok(outcome)
    }

    pub fn validate_dig_field_move(
        &self,
        state: &GameState,
        source_map: &str,
        party_index: usize,
    ) -> Result<FieldMoveUseOutcome> {
        let source_environment = self.map_environment(source_map)?;
        if !is_dig_field_move_environment(source_environment) {
            anyhow::bail!("cannot use DIG field move in environment {source_environment}");
        }
        core_validate_dig_field_move(&self.field_moves, &state.storage, party_index)
            .map_err(anyhow::Error::new)
            .context("use DIG field move")
    }

    pub fn use_dig_field_move(
        &self,
        state: &GameState,
        source_map: &str,
        party_index: usize,
    ) -> Result<DigFieldMoveOutcome> {
        self.require_no_active_battle(state, "DIG field move")?;
        let dig_rule = self.validate_dig_field_move(state, source_map, party_index)?;
        let destination = self.saved_dig_warp_destination(state, "DIG field move")?;
        Ok(DigFieldMoveOutcome {
            actor_party_index: dig_rule.actor_party_index,
            actor_species: dig_rule.actor_species,
            source_map: source_map.to_string(),
            destination_map: destination.map_name,
            destination_warp_index: destination.warp_index,
            destination_tile: destination.tile,
        })
    }

    pub fn use_dig_field_move_in_session(
        &self,
        state: &mut GameState,
        overworld: &mut OverworldSession,
        party_index: usize,
        music_ids: &BTreeSet<String>,
    ) -> Result<DigFieldMoveOutcome> {
        let mut staged_state = state.clone();
        let mut staged_overworld = overworld.clone();
        let source_map = staged_overworld.map.name.clone();
        let outcome = self.use_dig_field_move(&staged_state, &source_map, party_index)?;
        self.transition_overworld_session(
            &mut staged_state,
            &mut staged_overworld,
            &outcome.destination_map,
            outcome.destination_tile,
            SpawnMemoryUpdate::Preserve,
            music_ids,
        )?;
        *state = staged_state;
        *overworld = staged_overworld;
        Ok(outcome)
    }

    pub fn validate_teleport_field_move(
        &self,
        state: &GameState,
        source_map: &str,
        party_index: usize,
    ) -> Result<FieldMoveUseOutcome> {
        let source_environment = self.map_environment(source_map)?;
        if !is_teleport_source_environment(source_environment) {
            anyhow::bail!("cannot use TELEPORT field move in environment {source_environment}");
        }
        core_validate_teleport_field_move(&self.field_moves, &state.storage, party_index)
            .map_err(anyhow::Error::new)
            .context("use TELEPORT field move")
    }

    pub fn use_teleport_field_move(
        &self,
        state: &GameState,
        source_map: &str,
        party_index: usize,
    ) -> Result<TeleportFieldMoveOutcome> {
        self.require_no_active_battle(state, "TELEPORT field move")?;
        let teleport_rule = self.validate_teleport_field_move(state, source_map, party_index)?;
        let destination_spawn_identifier = state
            .last_spawn_identifier
            .with_context(|| "TELEPORT field move has no saved spawn identifier")?;
        let destination_spawn = self.runtime_spawn_point(destination_spawn_identifier)?;
        Ok(TeleportFieldMoveOutcome {
            actor_party_index: teleport_rule.actor_party_index,
            actor_species: teleport_rule.actor_species,
            source_map: source_map.to_string(),
            destination_spawn_identifier,
            destination_map: destination_spawn.map_name.clone(),
            destination_tile: runtime_spawn_expected_tile(destination_spawn),
        })
    }

    pub fn use_teleport_field_move_in_session(
        &self,
        state: &mut GameState,
        overworld: &mut OverworldSession,
        party_index: usize,
        music_ids: &BTreeSet<String>,
    ) -> Result<TeleportFieldMoveOutcome> {
        let mut staged_state = state.clone();
        let mut staged_overworld = overworld.clone();
        let source_map = staged_overworld.map.name.clone();
        let outcome = self.use_teleport_field_move(&staged_state, &source_map, party_index)?;
        self.transition_overworld_session(
            &mut staged_state,
            &mut staged_overworld,
            &outcome.destination_map,
            outcome.destination_tile,
            SpawnMemoryUpdate::Set(outcome.destination_spawn_identifier),
            music_ids,
        )?;
        *state = staged_state;
        *overworld = staged_overworld;
        Ok(outcome)
    }

    pub fn validate_direct_field_move_actor(
        &self,
        state: &GameState,
        party_index: usize,
        move_id: &str,
    ) -> Result<FieldMoveUseOutcome> {
        core_validate_direct_field_move_actor(&state.storage, party_index, move_id)
            .map_err(anyhow::Error::new)
            .with_context(|| format!("use {move_id} field move"))
    }

    pub fn apply_active_battle_item_effect(
        &self,
        pokemon: &mut Pokemon,
        item_id: &str,
        consumed: bool,
    ) -> Result<BattleItemOutcome> {
        let item = self.item(item_id)?;
        core_apply_active_battle_item_effect(pokemon, item, consumed)
            .map_err(anyhow::Error::new)
            .with_context(|| format!("use item {item_id}"))
    }

    pub fn apply_battle_pp_item_effect(
        &self,
        pokemon: &mut Pokemon,
        item_id: &str,
        move_slot: Option<usize>,
        consumed: bool,
    ) -> Result<BattleItemOutcome> {
        let item = self.item(item_id)?;
        core_apply_battle_pp_item_effect(
            pokemon,
            item,
            &self.moves,
            move_slot,
            consumed,
        )
        .map_err(anyhow::Error::new)
        .with_context(|| format!("use PP item {item_id}"))
    }

    pub fn use_bag_item_on_active_battle_pokemon(
        &self,
        state: &mut GameState,
        item_id: &str,
    ) -> Result<(ItemUseOutcome, BattleItemOutcome)> {
        let active_index =
            require_active_battle_party_index(state).map_err(|error| anyhow::anyhow!("{error}"))?;
        self.use_bag_item_on_battle_party_pokemon(state, item_id, active_index)
    }

    pub fn use_bag_item_on_battle_party_pokemon(
        &self,
        state: &mut GameState,
        item_id: &str,
        party_index: usize,
    ) -> Result<(ItemUseOutcome, BattleItemOutcome)> {
        let mut preview = clone_active_battle_party_pokemon(state, party_index)
            .map_err(|error| anyhow::anyhow!("use battle item {item_id}: {error:?}"))?;
        self.apply_active_battle_item_effect(&mut preview, item_id, false)?;

        let item_use = self.use_bag_item(state, item_id, ItemUseContext::Battle)?;
        let pokemon = require_active_battle_party_pokemon_mut(state, party_index)
            .map_err(|error| anyhow::anyhow!("use battle item {item_id}: {error:?}"))?;
        let battle_item =
            self.apply_active_battle_item_effect(pokemon, item_id, item_use.consumed)?;
        state.sync_party_from_storage();
        Ok((item_use, battle_item))
    }

    pub fn use_bag_item_on_battle_party_move(
        &self,
        state: &mut GameState,
        item_id: &str,
        party_index: usize,
        move_slot: Option<usize>,
    ) -> Result<(ItemUseOutcome, BattleItemOutcome)> {
        let mut preview = clone_active_battle_party_pokemon(state, party_index)
            .map_err(|error| anyhow::anyhow!("use battle item {item_id}: {error:?}"))?;
        self.apply_battle_pp_item_effect(&mut preview, item_id, move_slot, false)?;

        let item_use = self.use_bag_item(state, item_id, ItemUseContext::Battle)?;
        let pokemon = require_active_battle_party_pokemon_mut(state, party_index)
            .map_err(|error| anyhow::anyhow!("use battle item {item_id}: {error:?}"))?;
        let battle_item =
            self.apply_battle_pp_item_effect(pokemon, item_id, move_slot, item_use.consumed)?;
        state.sync_party_from_storage();
        Ok((item_use, battle_item))
    }

    pub fn apply_party_wide_item_effect(
        &self,
        party: &mut Party,
        item_id: &str,
        consumed: bool,
    ) -> Result<PartyItemOutcome> {
        let item = self.item(item_id)?;
        core_apply_party_wide_item_effect(party, item, consumed)
            .map_err(anyhow::Error::new)
            .with_context(|| format!("use whole-party item {item_id}"))
    }

    pub fn apply_party_pokemon_item_effect(
        &self,
        pokemon: &mut Pokemon,
        item_id: &str,
        time_of_day: TimeOfDay,
        consumed: bool,
    ) -> Result<BattleItemOutcome> {
        let item = self.item(item_id)?;
        let mut outcome = if item.rare_candy_level_gain.is_some()
            || self.evolutions.contains_item_evolution(&item.script_name)
        {
            core_apply_party_special_item_effect(
                pokemon,
                item,
                &self.pokemon,
                &self.moves,
                &self.learnsets,
                &self.growth_rates,
                &self.battle_reward_rules,
                &self.evolutions,
                time_of_day,
                consumed,
            )
        } else {
            core_apply_active_battle_item_effect(pokemon, item, consumed)
        }
        .map_err(anyhow::Error::new)
        .with_context(|| format!("use party item {item_id}"))?;
        if item.revive_hp_percent.is_some() && outcome.hp_before == 0 && outcome.hp_after > 0 {
            pokemon.status = None;
            pokemon.sleep_turns = 0;
            outcome.status_after = None;
        }
        Ok(outcome)
    }

    pub fn teach_tmhm_move(
        &self,
        pokemon: &mut Pokemon,
        item_id: &str,
        replace_slot: Option<usize>,
        consumed: bool,
    ) -> Result<TmHmLearnOutcome> {
        let item = self.item(item_id)?;
        core_teach_tmhm_move(pokemon, item, &self.moves, replace_slot, consumed)
            .map_err(anyhow::Error::new)
            .with_context(|| format!("use TM/HM {item_id}"))
    }

    pub fn use_bag_item_on_party_pokemon(
        &self,
        state: &mut GameState,
        item_id: &str,
        party_index: usize,
        time_of_day: TimeOfDay,
    ) -> Result<(ItemUseOutcome, BattleItemOutcome)> {
        let mut preview = clone_field_party_pokemon(state, party_index)
            .map_err(|error| anyhow::anyhow!("use party item {item_id}: {error:?}"))?;
        if preview.is_egg
            || preview.species.id == "EGG"
        {
            anyhow::bail!("use party item {item_id}: Eggs can't use that");
        }
        let preview_effect =
            self.apply_party_pokemon_item_effect(&mut preview, item_id, time_of_day, false)?;
        self.require_no_existing_pending_move_learn_for_item_effect(
            state,
            party_index,
            &preview_effect,
        )?;

        let item_use = self.use_bag_item(state, item_id, ItemUseContext::Field)?;
        let pokemon = require_field_party_pokemon_mut(state, party_index)
            .map_err(|error| anyhow::anyhow!("use party item {item_id}: {error:?}"))?;
        let item_effect =
            self.apply_party_pokemon_item_effect(pokemon, item_id, time_of_day, item_use.consumed)?;
        self.queue_item_pending_move_learn(state, party_index, &item_effect)
            .with_context(|| format!("queue pending move learn for party item {item_id}"))?;
        state.sync_party_from_storage();
        Ok((item_use, item_effect))
    }

    fn require_no_existing_pending_move_learn_for_item_effect(
        &self,
        state: &GameState,
        party_index: usize,
        item_effect: &BattleItemOutcome,
    ) -> Result<()> {
        if !item_effect.pending_move_learns.is_empty() && state.pending_move_learn.is_some() {
            anyhow::bail!("pending move learn already exists for party index {party_index}");
        }
        Ok(())
    }

    pub fn replace_pending_move_learn(
        &self,
        state: &mut GameState,
        move_slot: usize,
    ) -> Result<PendingMoveLearnRuntimeResolution> {
        let pending = state
            .pending_move_learn
            .as_ref()
            .ok_or_else(|| anyhow::Error::new(BattleRewardError::MissingPendingMoveLearn))?;
        let pokemon = state
            .storage
            .party
            .pokemon
            .get(pending.party_index)
            .and_then(|pokemon| pokemon.as_ref())
            .ok_or_else(|| {
                anyhow::Error::new(BattleRewardError::PendingMoveLearnEmptyPartySlot {
                    party_index: pending.party_index,
                })
            })?;
        if let Some(learned) = pokemon.moves.get(move_slot) {
            let is_hm = self.items.values().any(|item| {
                !item.consumable && item.tmhm_move.as_deref() == Some(learned.name.as_str())
            });
            if is_hm {
                return Err(anyhow::Error::new(BattleRewardError::CannotForgetHmMove {
                    move_id: learned.name.clone(),
                }))
                .context("replace pending move learn");
            }
        }
        let resolution = core_replace_pending_move_learn(state, move_slot)
            .map_err(anyhow::Error::new)
            .context("replace pending move learn")?;
        let deferred_evolution = self.resolve_deferred_evolution_after_pending_move_learn(
            state,
            &resolution,
            "replacement",
        )?;
        promote_next_pending_move_learn(state);
        Ok(PendingMoveLearnRuntimeResolution {
            resolution,
            deferred_evolution,
        })
    }

    pub fn decline_pending_move_learn(
        &self,
        state: &mut GameState,
    ) -> Result<PendingMoveLearnRuntimeResolution> {
        let resolution = core_decline_pending_move_learn(state)
            .map_err(|error| anyhow::anyhow!("decline pending move learn: {error:?}"))?;
        let deferred_evolution = self.resolve_deferred_evolution_after_pending_move_learn(
            state,
            &resolution,
            "decline",
        )?;
        promote_next_pending_move_learn(state);
        Ok(PendingMoveLearnRuntimeResolution {
            resolution,
            deferred_evolution,
        })
    }

    fn resolve_deferred_evolution_after_pending_move_learn(
        &self,
        state: &mut GameState,
        resolution: &PendingMoveLearnResolution,
        action: &str,
    ) -> Result<Option<EvolutionReport>> {
        if !resolution.defer_level_evolution {
            return Ok(None);
        }
        let time_of_day = state.time.time_of_day;
        self.resolve_deferred_level_evolution(state, resolution.party_index, time_of_day)
            .with_context(|| {
                format!("resolve deferred level evolution after pending move learn {action}")
            })
            .map(Some)
    }

    fn queue_item_pending_move_learn(
        &self,
        state: &mut GameState,
        party_index: usize,
        item_effect: &BattleItemOutcome,
    ) -> Result<()> {
        let Some(learned_move) = item_effect.pending_move_learns.first() else {
            return Ok(());
        };
        if state.pending_move_learn.is_some() {
            anyhow::bail!("pending move learn already exists for party index {party_index}");
        }
        let pokemon = state
            .storage
            .party
            .pokemon
            .get(party_index)
            .and_then(|slot| slot.as_ref())
            .with_context(|| format!("party index {party_index} is empty"))?;
        if pokemon.moves.len() < 4 {
            anyhow::bail!(
                "pending move learn requires full move list for party index {party_index}"
            );
        }
        if pokemon
            .moves
            .iter()
            .any(|known| known.name == learned_move.name)
        {
            return Ok(());
        }
        let species_id = pokemon.species.id.clone();
        let level = pokemon.level;
        state.pending_move_learn = Some(PendingMoveLearn {
            party_index,
            species_id,
            level,
            learned_move: learned_move.clone(),
            defer_level_evolution: item_effect.deferred_level_evolution,
        });
        Ok(())
    }

    pub fn resolve_deferred_level_evolution(
        &self,
        state: &mut GameState,
        party_index: usize,
        time_of_day: TimeOfDay,
    ) -> Result<EvolutionReport> {
        if state.pending_move_learn.is_some() {
            anyhow::bail!(
                "pending move learn already exists before resolving deferred level evolution for party index {party_index}"
            );
        }
        let context = EvolutionContext {
            species: &self.pokemon,
            moves: &self.moves,
            learnsets: &self.learnsets,
            time_of_day,
            current_item: None,
            force_evolution: false,
            link_mode: LinkMode::None,
        };
        let (report, pending_move_learn) = {
            let pokemon = require_field_party_pokemon_mut(state, party_index)
                .map_err(|error| anyhow::anyhow!("resolve deferred level evolution: {error:?}"))?;
            let report = check_and_evolve(pokemon, &self.evolutions, &context, true)
                .map_err(|error| anyhow::anyhow!("resolve deferred level evolution: {error:?}"))?;
            if report.target_species.is_none() {
                anyhow::bail!(
                    "deferred level evolution did not resolve for party index {party_index}"
                );
            }
            let pending_move_learn = if let Some(learned_move) = report.pending_move_learns.first()
            {
                if pokemon.moves.len() < 4 {
                    anyhow::bail!(
                        "pending evolution move learn requires full move list for party index {party_index}"
                    );
                }
                (!pokemon
                    .moves
                    .iter()
                    .any(|known| known.name == learned_move.name))
                .then(|| PendingMoveLearn {
                    party_index,
                    species_id: pokemon.species.id.clone(),
                    level: pokemon.level,
                    learned_move: learned_move.clone(),
                    defer_level_evolution: false,
                })
            } else {
                None
            };
            (report, pending_move_learn)
        };
        if let Some(pending_move_learn) = pending_move_learn {
            state.pending_move_learn = Some(pending_move_learn);
        }
        state.sync_party_from_storage();
        rebase_pending_move_learns_for_party(state, party_index, true);
        sync_active_combat_player_party_from_storage(state);
        Ok(report)
    }

    pub fn use_bag_item_on_party_pokemon_now(
        &self,
        state: &mut GameState,
        item_id: &str,
        party_index: usize,
    ) -> Result<(ItemUseOutcome, BattleItemOutcome)> {
        let time_of_day = state.time.time_of_day;
        self.use_bag_item_on_party_pokemon(state, item_id, party_index, time_of_day)
    }

    pub fn use_bag_item_on_whole_party(
        &self,
        state: &mut GameState,
        item_id: &str,
    ) -> Result<(ItemUseOutcome, PartyItemOutcome)> {
        let mut preview = clone_field_party(state)
            .map_err(|error| anyhow::anyhow!("use whole-party item {item_id}: {error:?}"))?;
        self.apply_party_wide_item_effect(&mut preview, item_id, false)?;

        let item_use = self.use_bag_item(state, item_id, ItemUseContext::Field)?;
        let party = require_field_party_mut(state)
            .map_err(|error| anyhow::anyhow!("use whole-party item {item_id}: {error:?}"))?;
        let item_effect = self.apply_party_wide_item_effect(party, item_id, item_use.consumed)?;
        state.sync_party_from_storage();
        Ok((item_use, item_effect))
    }

    pub fn use_bag_pp_item_on_party_pokemon(
        &self,
        state: &mut GameState,
        item_id: &str,
        party_index: usize,
        move_slot: Option<usize>,
    ) -> Result<(ItemUseOutcome, BattleItemOutcome)> {
        let mut preview = clone_field_party_pokemon(state, party_index)
            .map_err(|error| anyhow::anyhow!("use party item {item_id}: {error:?}"))?;
        if preview.is_egg
            || preview.species.id == "EGG"
        {
            anyhow::bail!("use party item {item_id}: Eggs can't use that");
        }
        self.apply_battle_pp_item_effect(&mut preview, item_id, move_slot, false)?;

        let item_use = self.use_bag_item(state, item_id, ItemUseContext::Field)?;
        let pokemon = require_field_party_pokemon_mut(state, party_index)
            .map_err(|error| anyhow::anyhow!("use party item {item_id}: {error:?}"))?;
        let item_effect =
            self.apply_battle_pp_item_effect(pokemon, item_id, move_slot, item_use.consumed)?;
        state.sync_party_from_storage();
        Ok((item_use, item_effect))
    }

    pub fn use_bag_tmhm_on_party_pokemon(
        &self,
        state: &mut GameState,
        item_id: &str,
        party_index: usize,
        replace_slot: Option<usize>,
    ) -> Result<(ItemUseOutcome, TmHmLearnOutcome)> {
        let mut preview = clone_field_party_pokemon(state, party_index)
            .map_err(|error| anyhow::anyhow!("use TM/HM {item_id}: {error:?}"))?;
        if preview.is_egg
            || preview.species.id == "EGG"
        {
            anyhow::bail!("use TM/HM {item_id}: Eggs can't use that");
        }
        self.teach_tmhm_move(&mut preview, item_id, replace_slot, false)?;

        let item_use = self.use_bag_item(state, item_id, ItemUseContext::Field)?;
        let pokemon = require_field_party_pokemon_mut(state, party_index)
            .map_err(|error| anyhow::anyhow!("use TM/HM {item_id}: {error:?}"))?;
        let learned_move =
            self.teach_tmhm_move(pokemon, item_id, replace_slot, item_use.consumed)?;
        state.sync_party_from_storage();
        Ok((item_use, learned_move))
    }

    pub fn saved_item(&self, item_id: &str) -> Option<&Item> {
        self.items.get(item_id)
    }

    pub fn saved_species_exists(&self, species: &str) -> bool {
        self.pokemon.contains_key(species)
    }

    pub fn saved_species(&self, species: &str) -> Option<PokemonSpecies> {
        self.pokemon.get(species).cloned()
    }

    pub fn saved_species_exact_exists(&self, species: &str) -> bool {
        self.pokemon
            .get(species)
            .is_some_and(|compiled| compiled.id == species)
    }

    pub fn saved_species_id(&self, species: &str) -> Option<String> {
        self.pokemon
            .get(species)
            .map(|compiled| compiled.id.clone())
    }

    pub fn saved_item_exists(&self, item_id: &str) -> bool {
        self.items.contains_key(item_id)
    }

    pub fn saved_item_script_name(&self, item_id: &str) -> Option<String> {
        self.items.get(item_id).map(|item| item.script_name.clone())
    }

    pub fn saved_move_name_and_pp(&self, move_name: &str) -> Option<(String, u8)> {
        self.moves
            .get(move_name)
            .map(|move_data| (move_data.name.clone(), move_data.pp))
    }

    pub fn ball_item(&self, ball_id: &str) -> Result<&Item> {
        self.items
            .get(ball_id)
            .with_context(|| format!("compiled game pack missing ball item {ball_id}"))
    }

    pub fn capture_ball_item(&self, ball_id: &str) -> Result<&Item> {
        let ball = self.ball_item(ball_id)?;
        validate_capture_ball_item(&self.capture_rules, ball).with_context(|| {
            format!("battle capture item {ball_id} is not declared by exact capture rules")
        })?;
        Ok(ball)
    }

    pub fn throw_ball_from_bag(
        &self,
        bag: &mut Bag,
        ball_id: &str,
        player: &Pokemon,
        enemy: &Pokemon,
        context: CaptureAttemptContext,
        rng: &mut Random,
    ) -> Result<CaptureOutcome> {
        let ball = self.capture_ball_item(ball_id)?;
        if !ball.battle_usable {
            anyhow::bail!("battle capture item {ball_id} is not usable in battle");
        }
        core_throw_ball_from_bag(
            bag,
            ball,
            player,
            enemy,
            context,
            &self.capture_rules,
            &self.capture_wobble_probabilities,
            rng,
        )
        .map_err(|error| anyhow::anyhow!("throw ball {ball_id}: {error}"))?
        .with_context(|| format!("throw ball {ball_id} did not produce a capture outcome"))
    }

    pub fn throw_ball_at_active_battle(
        &self,
        state: &mut GameState,
        ball_id: &str,
    ) -> Result<CaptureOutcome> {
        let active_index =
            require_active_battle_party_index(state).map_err(|error| anyhow::anyhow!("{error}"))?;
        let player = state.storage.party.pokemon[active_index]
            .as_ref()
            .cloned()
            .with_context(|| format!("active battle party index {active_index} has no Pokemon"))?;
        let (enemy, context) = match &state.battle {
            BattleMemory::Wild {
                battle_type,
                enemy_pokemon,
                ..
            }
            | BattleMemory::StaticWild {
                battle_type,
                enemy_pokemon,
                ..
            } => {
                let mut context = CaptureAttemptContext::wild(ball_id);
                context.battle_type = battle_type.clone();
                (enemy_pokemon.clone(), context)
            }
            BattleMemory::Trainer {
                battle_type,
                enemy_pokemon,
                ..
            } => {
                let mut context = CaptureAttemptContext::wild(ball_id);
                context.battle_type = battle_type.clone();
                context.trainer_battle = true;
                (enemy_pokemon.clone(), context)
            }
            BattleMemory::Inactive => {
                anyhow::bail!("cannot throw a ball without an active battle");
            }
        };
        let mut rng = Random::new_crystal(state.rng_seed);
        let outcome = if context.battle_type == "BATTLETYPE_TUTORIAL" {
            if ball_id != "POKE_BALL" {
                anyhow::bail!("catching tutorial requires POKE_BALL");
            }
            // ASM PokeBallEffect short-circuits the Dude tutorial to a
            // successful four-shake catch. Its bag is temporary WRAM, so the
            // player's carried balls are not consumed.
            CaptureOutcome {
                caught: true,
                blocked: false,
                storage_full: false,
                wobble_count: 3,
                animation_shakes: 4,
                final_catch_rate: 255,
                rng_seed_after: rng.seed(),
                ball_id: Some("POKE_BALL".to_string()),
            }
        } else if matches!(
            context.battle_type.as_str(),
            "BATTLETYPE_CONTEST" | "BATTLETYPE_BUG_CONTEST" | "BATTLETYPE_PARK"
        ) {
            if ball_id != "PARK_BALL" {
                anyhow::bail!("Bug-Catching Contest battles require PARK_BALL");
            }
            if state.bug_contest.park_balls_remaining == 0 {
                anyhow::bail!("no PARK_BALLs remain in the Bug-Catching Contest");
            }
            let mut outcome = core_resolve_capture_attempt(
                &player,
                &enemy,
                &context,
                &self.capture_rules,
                &self.capture_wobble_probabilities,
                &mut rng,
            )
            .map_err(|error| anyhow::anyhow!("throw PARK_BALL: {error}"))?;
            state.bug_contest.park_balls_remaining -= 1;
            outcome.ball_id = Some("PARK_BALL".to_string());
            outcome
        } else if !context.trainer_battle && !state.storage.has_capture_space() {
            CaptureOutcome {
                caught: false,
                blocked: true,
                storage_full: true,
                wobble_count: 0,
                animation_shakes: 0,
                final_catch_rate: 0,
                rng_seed_after: rng.seed(),
                ball_id: Some(ball_id.to_string()),
            }
        } else {
            self.throw_ball_from_bag(
                &mut state.bag,
                ball_id,
                &player,
                &enemy,
                context,
                &mut rng,
            )?
        };
        if !outcome.storage_full {
            // BattleMenu_Pack returns to ParsePlayerAction for an actual ball
            // throw, whose non-move path clears Bide before the item effect.
            // A full Box rejects the selection before that action boundary.
            if let Some(combat) = state.script_runtime.active_battle_combat.as_mut() {
                combat.player_bide_turns = 0;
                combat.player_bide_damage = 0;
                combat.player_fury_cutter_chain = 0;
                combat.player_protect_counter = 0;
                combat.player_rage_active = false;
                combat.player_rage_counter = 0;
            }
        }
        state.commit_rng_seed(rng.seed());
        Ok(outcome)
    }

    pub fn complete_active_wild_capture(
        &self,
        state: &mut GameState,
        outcome: &CaptureOutcome,
        nickname: Option<&str>,
    ) -> Result<CaptureCompletion> {
        let mut staged_state = state.clone();
        if let Some(nickname) = nickname {
            if nickname.is_empty()
                || nickname.trim() != nickname
                || nickname.chars().count() > 10
                || nickname.chars().any(char::is_control)
            {
                anyhow::bail!("captured Pokemon nickname must be exact and at most 10 characters");
            }
        }
        let caught_map_name = match &staged_state.battle {
            crystal_core::state::BattleMemory::Wild { map_name, .. } => Some(map_name.clone()),
            crystal_core::state::BattleMemory::StaticWild { .. } => match &staged_state.overworld {
                crystal_core::state::OverworldMemory::Active { map_name, .. } => {
                    Some(map_name.clone())
                }
                crystal_core::state::OverworldMemory::Inactive => None,
            },
            _ => None,
        };
        let pay_day_money = self.active_battle_pay_day_payout(&staged_state);
        let mut completion = core_complete_active_wild_capture(&mut staged_state, outcome)
            .map_err(|error| anyhow::anyhow!("complete captured Pokemon: {error}"))?;
        if let (Some(nickname), Some(stored)) = (nickname, completion.stored.as_mut()) {
            stored.pokemon.nickname = nickname.to_string();
            match stored.location {
                crystal_core::models::CaptureStorageLocation::Party { slot } => {
                    let pokemon = staged_state.storage.party.pokemon[slot]
                        .as_mut()
                        .context("captured party destination is empty")?;
                    pokemon.nickname = nickname.to_string();
                }
                crystal_core::models::CaptureStorageLocation::Pc { box_index, slot } => {
                    let pc_box = staged_state
                        .storage
                        .pc_boxes
                        .get_mut(box_index)
                        .context("captured PC destination box is missing")?;
                    let mut pokemon = pc_box.pokemon[slot]
                        .clone()
                        .context("captured PC destination is empty")?;
                    pokemon.nickname = nickname.to_string();
                    pc_box.set_slot(slot, Some(pokemon));
                }
            }
            staged_state.sync_party_from_storage();
        }
        if let (Some(map_name), Some(stored)) = (caught_map_name, completion.stored.as_mut()) {
            if let Some(location) = self
                .saved_map_id(&map_name)
                .and_then(|map_id| map_id.parse::<u8>().ok())
            {
                if let Some(caught_data) = stored.pokemon.caught_data.as_mut() {
                    caught_data.location = location;
                }
                match stored.location {
                    crystal_core::models::CaptureStorageLocation::Party { slot } => {
                        if let Some(Some(pokemon)) =
                            staged_state.storage.party.pokemon.get_mut(slot)
                        {
                            if let Some(caught_data) = pokemon.caught_data.as_mut() {
                                caught_data.location = location;
                            }
                        }
                    }
                    crystal_core::models::CaptureStorageLocation::Pc { box_index, slot } => {
                        if let Some(Some(pokemon)) = staged_state
                            .storage
                            .pc_boxes
                            .get_mut(box_index)
                            .and_then(|pc_box| pc_box.pokemon.get_mut(slot))
                        {
                            if let Some(caught_data) = pokemon.caught_data.as_mut() {
                                caught_data.location = location;
                            }
                        }
                    }
                }
                staged_state.sync_party_from_storage();
            }
        }
        if completion.stored.is_some() || completion.contest_pokemon.is_some() {
            self.claim_active_battle_pay_day_money(&mut staged_state, pay_day_money)?;
        }
        *state = staged_state;
        Ok(completion)
    }

    pub fn battle_escape_item_mode(&self, item_id: &str) -> Result<String> {
        let item = self.item(item_id)?;
        let mode = validate_battle_escape_item(item)
            .map_err(|error| anyhow::anyhow!("validate battle escape item {item_id}: {error:?}"))?;
        Ok(mode.to_string())
    }

    pub fn require_battle_escape_item_context(
        &self,
        state: &GameState,
        item_id: &str,
    ) -> Result<String> {
        let mode = self.battle_escape_item_mode(item_id)?;
        require_wild_battle_for_escape_item(state)
            .map_err(|error| anyhow::anyhow!("use battle escape item {item_id}: {error:?}"))?;
        Ok(mode)
    }

    pub fn apply_battle_escape_item_use(
        &self,
        state: &mut GameState,
        item_id: &str,
    ) -> Result<String> {
        let mode = self.battle_escape_item_mode(item_id)?;
        apply_battle_escape_item_use(state)
            .map_err(|error| anyhow::anyhow!("use battle escape item {item_id}: {error:?}"))?;
        Ok(mode)
    }

    pub fn use_bag_item_to_escape_active_wild_battle(
        &self,
        state: &mut GameState,
        item_id: &str,
    ) -> Result<BattleEscapeItemUseOutcome> {
        let mut staged_state = state.clone();
        let pay_day_money = self.active_battle_pay_day_payout(&staged_state);
        let battle_escape_mode = self.require_battle_escape_item_context(&staged_state, item_id)?;
        let item_use = self.use_bag_item(&mut staged_state, item_id, ItemUseContext::Battle)?;
        self.apply_battle_escape_item_use(&mut staged_state, item_id)?;
        self.claim_active_battle_pay_day_money(&mut staged_state, pay_day_money)?;
        *state = staged_state;
        Ok(BattleEscapeItemUseOutcome {
            item_use,
            battle_escape_mode,
            escaped: true,
        })
    }

    pub fn require_battle_stat_drop_guard_item_context(
        &self,
        state: &GameState,
        item_id: &str,
    ) -> Result<u8> {
        let item = self.item(item_id)?;
        let turns = validate_battle_stat_drop_guard_item(item).map_err(|error| {
            anyhow::anyhow!("validate battle stat drop guard item {item_id}: {error:?}")
        })?;
        require_active_battle_for_state_item(state)
            .map_err(|error| anyhow::anyhow!("use battle state item {item_id}: {error}"))?;
        Ok(turns)
    }

    pub fn apply_battle_stat_drop_guard_item(
        &self,
        state: &mut GameState,
        item_id: &str,
    ) -> Result<BattleStatDropGuardOutcome> {
        let turns = self.require_battle_stat_drop_guard_item_context(state, item_id)?;
        apply_battle_stat_drop_guard_turns(state, turns)
            .map_err(|error| anyhow::anyhow!("use battle state item {item_id}: {error}"))
    }

    pub fn use_bag_guard_spec_in_active_battle(
        &self,
        state: &mut GameState,
        item_id: &str,
    ) -> Result<BattleStateItemUseOutcome> {
        let mut staged_state = state.clone();
        self.require_battle_stat_drop_guard_item_context(&staged_state, item_id)?;
        let item_use = self.use_bag_item(&mut staged_state, item_id, ItemUseContext::Battle)?;
        let guard = self.apply_battle_stat_drop_guard_item(&mut staged_state, item_id)?;
        *state = staged_state;
        Ok(BattleStateItemUseOutcome {
            item_use,
            stat_drop_guard_turns_before: guard.turns_before,
            stat_drop_guard_turns_after: guard.turns_after,
        })
    }

    pub fn advance_active_trainer_battle(
        &self,
        state: &mut GameState,
    ) -> Result<TrainerBattleAdvanceOutcome> {
        core_advance_active_trainer_battle(state)
            .map_err(|error| anyhow::anyhow!("advance active trainer battle: {error}"))
    }

    pub fn resolve_active_battle_turn(
        &self,
        state: &mut GameState,
        player_action: BattleAction,
        enemy_action: BattleAction,
    ) -> Result<BattleTurnOutcome> {
        let active_index =
            require_active_battle_party_index(state).map_err(|error| anyhow::anyhow!("{error}"))?;
        let player = state.storage.party.pokemon[active_index]
            .as_ref()
            .cloned()
            .with_context(|| format!("active battle party index {active_index} has no Pokemon"))?;
        let player_party = Self::active_battle_player_party(state)?;
        let active_enemy_index = require_active_battle_enemy_party_index(state)
            .map_err(|error| anyhow::anyhow!("{error}"))?;
        let (enemy, enemy_party, active_enemy_index, is_wild_battle) = match &state.battle {
            BattleMemory::Wild {
                enemy_pokemon,
                enemy_party,
                ..
            }
            | BattleMemory::StaticWild {
                enemy_pokemon,
                enemy_party,
                ..
            } => (
                enemy_pokemon.clone(),
                enemy_party.clone(),
                active_enemy_index,
                true,
            ),
            BattleMemory::Trainer {
                enemy_pokemon,
                enemy_party,
                ..
            } => (
                enemy_pokemon.clone(),
                enemy_party.clone(),
                active_enemy_index,
                false,
            ),
            BattleMemory::Inactive => {
                anyhow::bail!("cannot resolve battle turn without an active battle");
            }
        };
        let roaming_species = match &state.battle {
            BattleMemory::Wild {
                battle_type,
                enemy_pokemon,
                ..
            } if battle_type == "BATTLETYPE_ROAMING" => Some(enemy_pokemon.species.id.clone()),
            _ => None,
        };
        let mut rng = Random::new_crystal(state.rng_seed);
        Self::require_active_enemy_in_battle_party(&enemy_party, active_enemy_index)?;
        let badge_boosts_enabled = state.link_session.link_mode == 0
            && active_battle_type(state) != Some("BATTLETYPE_BATTLE_TOWER");
        let mut combat = state
            .script_runtime
            .active_battle_combat
            .clone()
            .unwrap_or_else(|| {
                BattleCombatState::new(player, enemy, state.rng_seed)
                    .with_parties(player_party, enemy_party.to_vec())
                    .with_party_indices(active_index, active_enemy_index)
                    .with_obedience(state.player_id, state.badges.johto)
                    .with_time_context(
                        state.time.time_of_day,
                        state.link_session.link_mode != 0,
                    )
                    .with_badge_boosts_enabled(badge_boosts_enabled)
            });
        combat.badge_boosts_enabled = badge_boosts_enabled;
        if matches!(player_action, BattleAction::Run)
            && active_battle_type(state).is_some_and(battle_type_blocks_escape)
            && combat.player_escape_trap.is_none()
        {
            combat.player_escape_trap = Some(BattleEscapeTrapState {
                source: BattleSide::Enemy,
                move_name: active_battle_type(state).unwrap_or_default().to_string(),
            });
        }
        let input = BattleTurnInput {
            player: player_action,
            enemy: enemy_action,
        };
        let outcome = if is_wild_battle {
            self.resolve_wild_battle_turn_with_items(
                combat,
                input,
                state.battle_escape_attempts,
                &mut rng,
            )?
        } else {
            self.resolve_battle_turn_with_items(combat, input, &mut rng)?
        };
        commit_battle_turn_outcome(state, active_index, &outcome)
            .map_err(|error| anyhow::anyhow!("commit active battle turn: {error:?}"))?;
        if let Some(species) = roaming_species.as_deref()
            && matches!(state.battle, BattleMemory::Inactive)
        {
            self.finish_roaming_battle(state, species, &outcome.state.enemy);
        }
        if is_wild_battle {
            if let Some(escape) = outcome.events.iter().find_map(|event| match event {
                BattleEvent::RunAttempt {
                    side: BattleSide::Player,
                    outcome,
                } => Some(outcome),
                _ => None,
            }) {
                commit_wild_battle_escape_attempt(state, escape);
            }
        }
        state.commit_rng_seed(rng.seed());
        Ok(outcome)
    }

    fn active_battle_player_party(state: &GameState) -> Result<Vec<Pokemon>> {
        let mut party = Vec::new();
        let mut seen_empty = false;
        for (index, slot) in state.storage.party.pokemon.iter().enumerate() {
            match slot {
                Some(pokemon) => {
                    if seen_empty {
                        anyhow::bail!(
                            "active battle party has occupied slot {index} after an empty slot"
                        );
                    }
                    party.push(pokemon.clone());
                }
                None => seen_empty = true,
            }
        }
        Ok(party)
    }

    fn require_active_enemy_in_battle_party(party: &[Pokemon], active_index: usize) -> Result<()> {
        if active_index >= party.len() {
            anyhow::bail!(
                "active battle enemy index {active_index} is outside active enemy party length {}",
                party.len()
            );
        }
        Ok(())
    }

    pub fn resolve_active_wild_battle_run(
        &self,
        state: &mut GameState,
    ) -> Result<BattleEscapeAttempt> {
        let mut staged_state = state.clone();
        let pay_day_money = self.active_battle_pay_day_payout(&staged_state);
        let battle_type = active_battle_type(&staged_state)
            .ok_or_else(|| anyhow::anyhow!("cannot escape without an active battle"))?;
        if battle_type_blocks_escape(battle_type) {
            return Ok(BattleEscapeAttempt {
                escaped: false,
                chance: 0,
                roll: None,
                attempts_before: staged_state.battle_escape_attempts,
                attempts_after: staged_state.battle_escape_attempts,
                rng_seed_after: staged_state.rng_seed,
            });
        }
        if battle_type_guarantees_escape(battle_type) || staged_state.link_session.link_mode != 0 {
            let outcome = BattleEscapeAttempt {
                escaped: true,
                chance: self.battle_escape_rules.rng_roll_values,
                roll: None,
                attempts_before: staged_state.battle_escape_attempts,
                attempts_after: staged_state.battle_escape_attempts,
                rng_seed_after: staged_state.rng_seed,
            };
            commit_wild_battle_escape_attempt(&mut staged_state, &outcome);
            self.claim_active_battle_pay_day_money(&mut staged_state, pay_day_money)?;
            *state = staged_state;
            return Ok(outcome);
        }
        let active_index = require_active_battle_party_index(&staged_state)
            .map_err(|error| anyhow::anyhow!("{error}"))?;
        let player = staged_state.storage.party.pokemon[active_index]
            .as_ref()
            .cloned()
            .with_context(|| format!("active battle party index {active_index} has no Pokemon"))?;
        let enemy = match &staged_state.battle {
            BattleMemory::Wild { enemy_pokemon, .. }
            | BattleMemory::StaticWild { enemy_pokemon, .. } => enemy_pokemon.clone(),
            BattleMemory::Trainer { trainer_id, .. } => {
                anyhow::bail!("cannot escape from trainer battle {trainer_id}");
            }
            BattleMemory::Inactive => {
                anyhow::bail!("cannot escape without an active wild battle");
            }
        };
        let mut rng = Random::new_crystal(staged_state.rng_seed);
        let combat = staged_state
            .script_runtime
            .active_battle_combat
            .clone()
            .unwrap_or_else(|| {
                BattleCombatState::new(player, enemy.clone(), staged_state.rng_seed)
                    .with_time_context(
                        staged_state.time.time_of_day,
                        staged_state.link_session.link_mode != 0,
                    )
            });
        if combat.force_switch_blocked
            || combat.player_escape_trap.is_some()
            || combat.player_trap.is_some()
        {
            return Ok(BattleEscapeAttempt {
                escaped: false,
                chance: 0,
                roll: None,
                attempts_before: staged_state.battle_escape_attempts,
                attempts_after: staged_state.battle_escape_attempts,
                rng_seed_after: staged_state.rng_seed,
            });
        }
        let outcome =
            self.resolve_wild_battle_run(&combat, staged_state.battle_escape_attempts, &mut rng)?;
        let roaming_species = match &staged_state.battle {
            BattleMemory::Wild {
                battle_type,
                enemy_pokemon,
                ..
            } if battle_type == "BATTLETYPE_ROAMING" => Some(enemy_pokemon.species.id.clone()),
            _ => None,
        };
        commit_wild_battle_escape_attempt(&mut staged_state, &outcome);
        if outcome.escaped {
            self.claim_active_battle_pay_day_money(&mut staged_state, pay_day_money)?;
            if let Some(species) = roaming_species.as_deref() {
                self.finish_roaming_battle(&mut staged_state, species, &enemy);
            }
        }
        staged_state.commit_rng_seed(rng.seed());
        *state = staged_state;
        Ok(outcome)
    }

    pub fn resolve_active_battle_command(
        &self,
        state: &mut GameState,
        player_action: BattleAction,
        enemy_action: BattleAction,
    ) -> Result<ActiveBattleCommandOutcome> {
        if matches!(player_action, BattleAction::Run) {
            if active_battle_type(state).is_some_and(battle_type_guarantees_escape)
                || state.link_session.link_mode != 0
            {
                return self
                    .resolve_active_wild_battle_run(state)
                    .map(ActiveBattleCommandOutcome::Escape);
            }
            return match &state.battle {
                BattleMemory::Wild { .. } | BattleMemory::StaticWild { .. } => {
                    if matches!(enemy_action, BattleAction::Run) {
                        self.resolve_active_wild_battle_run(state)
                            .map(ActiveBattleCommandOutcome::Escape)
                    } else {
                        self.resolve_active_battle_turn(state, BattleAction::Run, enemy_action)
                            .map(ActiveBattleCommandOutcome::Turn)
                    }
                }
                BattleMemory::Trainer { .. } => self
                    .resolve_active_battle_turn(state, BattleAction::Run, enemy_action)
                    .map(ActiveBattleCommandOutcome::Turn),
                BattleMemory::Inactive => {
                    anyhow::bail!("player run requires an active battle");
                }
            };
        }
        if matches!(enemy_action, BattleAction::Run) {
            match &state.battle {
                BattleMemory::Wild { .. }
                | BattleMemory::StaticWild { .. }
                | BattleMemory::Trainer { .. } => {}
                BattleMemory::Inactive => {
                    anyhow::bail!("enemy run requires an active battle");
                }
            }
            return self
                .resolve_active_battle_turn(state, player_action, enemy_action)
                .map(ActiveBattleCommandOutcome::Turn);
        }
        self.resolve_active_battle_turn(state, player_action, enemy_action)
            .map(ActiveBattleCommandOutcome::Turn)
    }

    pub fn resolve_active_battle_enemy_action(
        &self,
        state: &mut GameState,
        enemy_action: BattleAction,
    ) -> Result<BattleTurnOutcome> {
        let active_index =
            require_active_battle_party_index(state).map_err(|error| anyhow::anyhow!("{error}"))?;
        let player = state.storage.party.pokemon[active_index]
            .as_ref()
            .cloned()
            .with_context(|| format!("active battle party index {active_index} has no Pokemon"))?;
        let player_party = Self::active_battle_player_party(state)?;
        let active_enemy_index = require_active_battle_enemy_party_index(state)
            .map_err(|error| anyhow::anyhow!("{error}"))?;
        let force_switch_ends_battle = matches!(
            &state.battle,
            BattleMemory::Wild { .. } | BattleMemory::StaticWild { .. }
        );
        let (enemy, enemy_party, active_enemy_index) = match &state.battle {
            BattleMemory::Wild {
                enemy_pokemon,
                enemy_party,
                ..
            }
            | BattleMemory::StaticWild {
                enemy_pokemon,
                enemy_party,
                ..
            }
            | BattleMemory::Trainer {
                enemy_pokemon,
                enemy_party,
                ..
            } => (
                enemy_pokemon.clone(),
                enemy_party.clone(),
                active_enemy_index,
            ),
            BattleMemory::Inactive => {
                anyhow::bail!("cannot resolve enemy battle action without an active battle");
            }
        };
        let mut rng = Random::new_crystal(state.rng_seed);
        Self::require_active_enemy_in_battle_party(&enemy_party, active_enemy_index)?;
        let badge_boosts_enabled = state.link_session.link_mode == 0
            && active_battle_type(state) != Some("BATTLETYPE_BATTLE_TOWER");
        let mut combat = state
            .script_runtime
            .active_battle_combat
            .clone()
            .unwrap_or_else(|| {
                BattleCombatState::new(player, enemy, state.rng_seed)
                    .with_parties(player_party, enemy_party.to_vec())
                    .with_party_indices(active_index, active_enemy_index)
                    .with_obedience(state.player_id, state.badges.johto)
                    .with_time_context(
                        state.time.time_of_day,
                        state.link_session.link_mode != 0,
                    )
                    .with_badge_boosts_enabled(badge_boosts_enabled)
            });
        combat.badge_boosts_enabled = badge_boosts_enabled;
        let outcome = self.resolve_battle_enemy_action_with_items(
            combat,
            enemy_action,
            force_switch_ends_battle,
            &mut rng,
        )?;
        let pay_day_money_after_turn = self.active_battle_pay_day_money_after_turn(state, &outcome);
        commit_battle_turn_outcome(state, active_index, &outcome)
            .map_err(|error| anyhow::anyhow!("commit enemy battle action: {error:?}"))?;
        if matches!(state.battle, BattleMemory::Inactive) {
            self.claim_active_battle_pay_day_money(state, pay_day_money_after_turn)?;
        }
        state.commit_rng_seed(rng.seed());
        Ok(outcome)
    }

    pub fn resolve_battle_turn_with_items(
        &self,
        combat: BattleCombatState,
        input: BattleTurnInput,
        rng: &mut Random,
    ) -> Result<BattleTurnOutcome> {
        core_resolve_battle_turn_with_items(
            combat,
            input,
            &self.moves,
            &self.items,
            &self.move_priorities,
            &self.battle_stat_multipliers,
            &self.type_categories,
            &self.type_effectiveness,
            &self.weather_modifiers,
            rng,
        )
        .map_err(|error| anyhow::anyhow!("resolve active battle turn: {error:?}"))
    }

    pub fn resolve_wild_battle_turn_with_items(
        &self,
        combat: BattleCombatState,
        input: BattleTurnInput,
        attempts: u8,
        rng: &mut Random,
    ) -> Result<BattleTurnOutcome> {
        core_resolve_wild_battle_turn_with_items(
            combat,
            input,
            &self.moves,
            &self.items,
            &self.move_priorities,
            &self.battle_stat_multipliers,
            &self.type_categories,
            &self.type_effectiveness,
            &self.weather_modifiers,
            &self.battle_escape_rules,
            attempts,
            rng,
        )
        .map_err(|error| anyhow::anyhow!("resolve active wild battle turn: {error:?}"))
    }

    pub fn resolve_battle_enemy_action_with_items(
        &self,
        combat: BattleCombatState,
        enemy_action: BattleAction,
        force_switch_ends_battle: bool,
        rng: &mut Random,
    ) -> Result<BattleTurnOutcome> {
        core_resolve_battle_enemy_action_with_items(
            combat,
            enemy_action,
            force_switch_ends_battle,
            &self.moves,
            &self.items,
            &self.battle_stat_multipliers,
            &self.type_categories,
            &self.type_effectiveness,
            &self.weather_modifiers,
            rng,
        )
        .map_err(|error| anyhow::anyhow!("resolve active enemy battle action: {error:?}"))
    }

    pub fn resolve_wild_battle_run(
        &self,
        combat: &BattleCombatState,
        attempts: u8,
        rng: &mut Random,
    ) -> Result<BattleEscapeAttempt> {
        core_resolve_wild_battle_run(
            combat,
            &self.battle_escape_rules,
            attempts,
            &self.battle_stat_multipliers,
            rng,
        )
        .map_err(|error| anyhow::anyhow!("resolve wild battle run: {error:?}"))
    }

    pub fn claim_active_trainer_battle_rewards(
        &self,
        state: &mut GameState,
        time_of_day: TimeOfDay,
    ) -> Result<BattleRewardOutcome> {
        let mut staged_state = state.clone();
        let outcome = core_claim_active_trainer_battle_rewards(
            &mut staged_state,
            &self.battle_reward_rules,
            &self.pokemon,
            &self.moves,
            &self.learnsets,
            &self.growth_rates,
            &self.evolutions,
            time_of_day,
        )
        .map_err(|error| anyhow::anyhow!("claim trainer battle rewards: {error:?}"))?;
        *state = staged_state;
        Ok(outcome)
    }

    pub fn claim_active_trainer_battle_rewards_now(
        &self,
        state: &mut GameState,
    ) -> Result<BattleRewardOutcome> {
        self.claim_active_trainer_battle_rewards(state, state.time.time_of_day)
    }

    pub fn claim_active_wild_battle_rewards(
        &self,
        state: &mut GameState,
        time_of_day: TimeOfDay,
    ) -> Result<BattleRewardOutcome> {
        let mut staged_state = state.clone();
        let pay_day_money = self.active_battle_pay_day_payout(&staged_state);
        let outcome = core_claim_active_wild_battle_rewards(
            &mut staged_state,
            &self.battle_reward_rules,
            &self.pokemon,
            &self.moves,
            &self.learnsets,
            &self.growth_rates,
            &self.evolutions,
            time_of_day,
        )
        .map_err(|error| anyhow::anyhow!("claim wild battle rewards: {error:?}"))?;
        self.claim_active_battle_pay_day_money(&mut staged_state, pay_day_money)?;
        *state = staged_state;
        Ok(outcome)
    }

    pub fn claim_active_wild_battle_rewards_now(
        &self,
        state: &mut GameState,
    ) -> Result<BattleRewardOutcome> {
        self.claim_active_wild_battle_rewards(state, state.time.time_of_day)
    }

    fn claim_active_battle_pay_day_money(&self, state: &mut GameState, amount: u32) -> Result<()> {
        if amount == 0 {
            return Ok(());
        }
        let max_money = self
            .currency_constants
            .get("MAX_MONEY")
            .context("currency constants missing MAX_MONEY")?;
        state.money = state.money.saturating_add(amount).min(max_money);
        state.battle_pay_day_money = 0;
        Ok(())
    }

    fn active_battle_pay_day_payout(&self, state: &GameState) -> u32 {
        let amount = state.battle_pay_day_money.min(0x00ff_ffff);
        if state.battle_amulet_coin_active {
            amount.saturating_mul(2).min(0x00ff_ffff)
        } else {
            amount
        }
    }

    fn active_battle_pay_day_money_after_turn(
        &self,
        state: &GameState,
        outcome: &BattleTurnOutcome,
    ) -> u32 {
        let amount = state.battle_pay_day_money.saturating_add(
            outcome
                .events
                .iter()
                .filter_map(|event| match event {
                    BattleEvent::PayDayMoney {
                        side: BattleSide::Player,
                        amount,
                        ..
                    } => Some(*amount),
                    BattleEvent::PayDayMoney { .. } => None,
                    _ => None,
                })
                .fold(0_u32, u32::saturating_add),
        ).min(0x00ff_ffff);
        if state.battle_amulet_coin_active {
            amount.saturating_mul(2).min(0x00ff_ffff)
        } else {
            amount
        }
    }

    pub fn fishing_rod_item(&self, item_id: &str) -> Result<(&Item, &str)> {
        let item = self.item(item_id)?;
        let rod = fishing_rod_for_item_id(&self.fishing, item_id).with_context(|| {
            format!(
                "field fishing rod item {item_id} is not declared by exact fishing rod item rules"
            )
        })?;
        Ok((item, rod))
    }

    pub fn field_fishing_rod(&self, state: &GameState, item_id: &str) -> Result<String> {
        let (item, rod) = self.fishing_rod_item(item_id)?;
        if !item.field_usable {
            anyhow::bail!("field fishing rod item {item_id} is not usable in the field");
        }
        if !state.bag.has_item(item) {
            anyhow::bail!("field fishing rod item {item_id} is not in the bag");
        }
        Ok(rod.to_string())
    }

    pub fn cast_fishing_rod(
        &self,
        state: &mut GameState,
        map_name: &str,
        rod: &str,
        rng: &mut Random,
    ) -> Result<FishingRolledSession> {
        core_validate_fishing_rod(rod)
            .map_err(|error| anyhow::anyhow!("{error}"))
            .with_context(|| format!("validate fishing rod {rod} before cast"))?;
        let group = self.map_fishing_group(map_name)?;
        let time_of_day = state.time.time_of_day;
        core_do_fishing_from_rng(state, &self.fishing, group, rod, time_of_day, rng)
            .map_err(|error| anyhow::anyhow!("cast fishing rod {rod} on {map_name}: {error:?}"))
    }

    pub fn cast_fishing_rod_in_session(
        &self,
        state: &mut GameState,
        session: &OverworldSession,
        rod: &str,
    ) -> Result<FishingCastOutcome> {
        self.require_no_active_battle(state, "fishing rod")?;
        if matches!(session.player.mode, MovementMode::Surf | MovementMode::SurfPika) {
            anyhow::bail!(crystal_core::world::fishing::FishingError::CannotFishWhileSurfing);
        }
        let target = checked_move_by_stride(
            session.player.tile,
            session.player.facing,
            StepOptions::default().stride_tiles,
        )
        .ok_or_else(|| {
            anyhow::Error::new(crystal_core::world::fishing::FishingError::FacingTileOutOfBounds)
        })?;
        let sample = sample_collision(&session.map, &session.tileset, target).ok_or_else(|| {
            anyhow::Error::new(crystal_core::world::fishing::FishingError::FacingTileOutOfBounds)
        })?;
        if describe_collision(sample.permission).terrain != Terrain::Water {
            anyhow::bail!(crystal_core::world::fishing::FishingError::FacingTileIsNotWater);
        }
        let map_name = session.map.name.clone();
        let mut rng = Random::new_crystal(state.rng_seed);
        let time_of_day = state.time.time_of_day;
        let rolled = self.cast_fishing_rod(state, &map_name, rod, &mut rng)?;
        state.commit_rng_seed(rolled.rng_seed_after);
        let bite_roll = rolled.bite_roll;
        let slot_roll = rolled.slot_roll;
        let mut fishing_session = rolled.session;
        let bite_frame = fishing_session
            .start_frame
            .saturating_add(fishing_session.cast_frames)
            .saturating_add(fishing_session.bite_delay_frames);
        let bite = fishing_bite(state, &mut fishing_session, bite_frame);
        let wild_battle = if bite == Some(true) {
            fishing_battle_trigger(state);
            if let Some(encounter) = fishing_session.outcome.encounter.clone() {
                Some(self.start_fishing_battle(
                    state,
                    &map_name,
                    session.player.tile,
                    encounter,
                    time_of_day,
                    bite_roll,
                    slot_roll,
                )?)
            } else {
                None
            }
        } else {
            None
        };
        Ok(FishingCastOutcome {
            session: fishing_session,
            bite,
            wild_battle,
        })
    }

    pub fn use_bag_fishing_rod_in_field(
        &self,
        state: &mut GameState,
        session: &OverworldSession,
        item_id: &str,
    ) -> Result<FishingRodItemUseOutcome> {
        self.require_no_active_battle(state, "field fishing rod item")?;
        let rod = self.field_fishing_rod(state, item_id)?;
        let cast = self.cast_fishing_rod_in_session(state, session, &rod)?;
        let cast_state_checksum =
            game_state_checksum(state).context("checksum field fishing rod cast")?;
        let item_use = self.use_bag_item(state, item_id, ItemUseContext::Field)?;
        Ok(FishingRodItemUseOutcome {
            item_use,
            rod,
            cast,
            cast_state_checksum,
        })
    }

    pub fn saved_fishing_rod_exists(&self, rod: &str) -> bool {
        self.fishing
            .groups
            .values()
            .any(|group| group.rod_tables.contains_key(rod))
    }

    pub fn saved_fishing_daily_flag_bit_exists(&self, bit: u32) -> bool {
        self.fishing
            .swarm_rules
            .values()
            .any(|rule| u32::from(rule.daily_flag_bit) == bit)
    }

    pub fn saved_fishing_swarm_flag_exists(&self, swarm_flag: u8) -> bool {
        self.fishing
            .swarm_rules
            .values()
            .any(|rule| rule.swarm == swarm_flag)
    }

    pub fn require_special_routine(&self, routine: &str) -> Result<()> {
        if self.special_routines.contains_key(routine) {
            Ok(())
        } else {
            anyhow::bail!("compiled game pack missing exact special routine {routine}")
        }
    }

    pub fn saved_special_routine_exists(&self, routine: &str) -> bool {
        self.special_routines.contains_key(routine)
            || matches!(routine, "StartBugContestTimer" | "CheckBugContestTimer")
    }

    pub fn saved_sprite_exists(&self, sprite_id: &str) -> bool {
        self.sprite_palette_defaults.contains_key(sprite_id)
    }

    pub fn saved_variable_sprite_exists(&self, sprite_id: &str) -> bool {
        self.initialize_events
            .variable_sprites
            .contains_key(sprite_id)
    }

    pub fn saved_menu_exists(&self, menu: &str) -> bool {
        self.special_routines.contains_key(menu)
            || self
                .maps
                .values()
                .any(|module| module.script_menu_definitions.contains_key(menu))
    }

    pub fn saved_special_phone_call_exists(&self, call_id: &str) -> bool {
        self.special_phone_calls.contains_key(call_id)
    }

    pub fn saved_npc_trade_exists(&self, trade_id: &str) -> bool {
        self.npc_trades.contains_key(trade_id)
    }

    pub fn saved_catch_tutorial_battle_type_exists(&self, battle_type: &str) -> bool {
        self.maps.values().any(|module| {
            module.script_runtime_commands.iter().any(|command| {
                command.command == "catchtutorial"
                    && command
                        .args
                        .first()
                        .is_some_and(|candidate| candidate == battle_type)
            })
        })
    }

    pub fn map_name_for_constant(&self, map_constant: &str) -> Result<String> {
        self.maps
            .iter()
            .find(|(_, module)| module.attributes.map_constant.as_deref() == Some(map_constant))
            .map(|(map_name, _)| map_name.clone())
            .with_context(|| format!("compiled game pack missing map constant {map_constant}"))
    }

    pub fn saved_map_constant(&self, map_constant: &str) -> Option<String> {
        self.runtime_map_metadata
            .get(map_constant)
            .map(|metadata| metadata.constant.clone())
    }

    pub fn runtime_map_group_number_exists(&self, map_group: u16, map_number: u16) -> bool {
        self.runtime_map_metadata
            .values()
            .any(|metadata| metadata.group_id == map_group && metadata.map_id == map_number)
    }

    pub fn saved_roaming_species_level_exists(&self, species: &str, level: u8) -> bool {
        self.roaming_pokemon
            .iter()
            .any(|(candidate, definition)| candidate == species && definition.level == level)
    }

    pub fn saved_pending_special_battle_type_exists(&self, battle_type: &str) -> bool {
        self.maps.values().any(|module| {
            module
                .scripted_trainer_battles
                .iter()
                .any(|battle| battle.request.battle_type == battle_type)
                || module
                    .scripted_wild_battles
                    .iter()
                    .any(|battle| battle.request.battle_type == battle_type)
        }) || saved_special_battle_type_builtin_routine(battle_type)
            .is_some_and(|routine| self.saved_special_routine_exists(routine))
    }

    pub fn saved_static_wild_battle_origin_exists(
        &self,
        source_script: &str,
        battle_type: &str,
        species: &str,
        level: u8,
    ) -> bool {
        self.maps.values().any(|module| {
            module.scripted_wild_battles.iter().any(|battle| {
                let request = &battle.request;
                battle.source_script == source_script
                    && request.battle_type == battle_type
                    && request.species == species
                    && request.level == level
                    && request.source_script == source_script
            })
        })
    }

    pub fn saved_scripted_trainer_battle(
        &self,
        source_script: &str,
        trainer_id: &str,
    ) -> Option<&ScriptedTrainerBattle> {
        self.maps.values().find_map(|module| {
            module.scripted_trainer_battles.iter().find(|battle| {
                battle.source_script == source_script && battle.request.trainer_id == trainer_id
            })
        })
    }

    pub fn saved_spawn_identifier(&self, spawn_identifier: u16) -> Option<String> {
        self.runtime_spawn_points
            .get(&spawn_identifier.to_string())
            .map(|spawn| spawn.identifier.to_string())
    }

    pub fn saved_phone_contact_id(&self, contact_id: &str) -> Option<String> {
        self.phone_contacts
            .0
            .get(contact_id)
            .map(|contact| contact.contact_id.clone())
    }

    pub fn saved_trainer(&self, trainer_id: &str) -> Option<&Trainer> {
        self.trainers.get(trainer_id)
    }

    pub fn saved_trainer_id(&self, trainer_id: &str) -> Option<String> {
        self.trainers
            .get(trainer_id)
            .map(|trainer| trainer.trainer_id.clone())
    }

    pub fn runtime_map_metadata_for_name(&self, map_name: &str) -> Result<&RuntimeMapMetadata> {
        self.runtime_map_metadata
            .values()
            .find(|metadata| metadata.name == map_name)
            .with_context(|| {
                format!("compiled game pack missing runtime metadata for map {map_name}")
            })
    }

    pub fn map_environment(&self, map_name: &str) -> Result<&str> {
        Ok(self
            .runtime_map_metadata_for_name(map_name)?
            .environment
            .as_str())
    }

    pub fn apply_dig_warp_memory_for_transition(
        &self,
        state: &mut GameState,
        transition: &WarpTransition,
    ) -> Result<()> {
        let source_environment = self.map_environment(&transition.trigger.map_name)?;
        let destination_environment = self.map_environment(&transition.destination.map_name)?;
        apply_dig_warp_memory_for_transition(
            state,
            transition,
            source_environment,
            destination_environment,
        );
        Ok(())
    }

    pub fn map_constant(&self, map_name: &str) -> Result<&str> {
        Ok(self
            .runtime_map_metadata_for_name(map_name)?
            .constant
            .as_str())
    }

    pub fn wild_encounters_for_map(&self, map_name: &str) -> Option<&WildEncounterData> {
        self.wild_encounters.get(map_name)
    }

    pub fn require_wild_encounters_for_map(&self, map_name: &str) -> Result<&WildEncounterData> {
        self.wild_encounters
            .get(map_name)
            .with_context(|| format!("compiled game pack missing wild encounters for {map_name}"))
    }

    pub fn require_field_encounters_for_map(&self, map_name: &str) -> Result<&FieldEncounterData> {
        self.field_encounters
            .get(map_name)
            .with_context(|| format!("compiled game pack missing field encounters for {map_name}"))
    }

    pub fn roll_headbutt_encounter(
        &self,
        map_name: &str,
        target_tile: TilePosition,
        player_id: u16,
        rng: &mut Random,
    ) -> Result<crystal_core::world::encounters::FieldEncounterRoll> {
        self.validate_runtime_map_tile("HEADBUTT encounter", map_name, target_tile)?;
        let encounters = self.require_field_encounters_for_map(map_name)?;
        core_roll_headbutt_encounter(encounters, target_tile.x, target_tile.y, player_id, rng)
            .map_err(|error| anyhow::anyhow!("roll HEADBUTT encounter on {map_name}: {error:?}"))
    }

    pub fn roll_rock_smash_encounter(
        &self,
        map_name: &str,
        target_tile: TilePosition,
        rng: &mut Random,
    ) -> Result<crystal_core::world::encounters::FieldEncounterRoll> {
        self.validate_runtime_map_tile("ROCK_SMASH encounter", map_name, target_tile)?;
        let encounters = self.require_field_encounters_for_map(map_name)?;
        core_roll_rock_smash_encounter(encounters, target_tile.x, target_tile.y, rng)
            .map_err(|error| anyhow::anyhow!("roll ROCK_SMASH encounter on {map_name}: {error:?}"))
    }

    fn validate_direct_headbutt_target(
        rule: &FieldMoveMoveRule,
        map: &OverworldMapData,
        tileset: &TilesetCollision,
        target: TilePosition,
    ) -> Result<()> {
        Self::validate_runtime_field_move_tile_alignment(&rule.move_id, target)?;
        let sample = sample_collision(map, tileset, target).ok_or_else(|| {
            anyhow::anyhow!(FieldMoveError::TargetTileOutOfBounds {
                move_id: rule.move_id.clone(),
                map_name: map.name.clone(),
            })
        })?;
        if !rule.target_collisions.contains(&sample.permission) {
            anyhow::bail!(FieldMoveError::UnsupportedCollision {
                move_id: rule.move_id.clone(),
                block_id: sample.metatile_id,
            });
        }
        Ok(())
    }

    fn validate_direct_rock_smash_target(
        rule: &FieldMoveMoveRule,
        overworld: &OverworldSession,
        target: TilePosition,
    ) -> Result<(Option<String>, String)> {
        Self::validate_runtime_field_move_tile_alignment(&rule.move_id, target)?;
        let Some((_, object)) = overworld.visible_object_at_checked(target)? else {
            anyhow::bail!(FieldMoveError::MissingRockSmashTarget {
                move_id: rule.move_id.clone(),
                x: target.x,
                y: target.y,
            });
        };
        if object.spritemovedata != "SPRITEMOVEDATA_SMASHABLE_ROCK" {
            anyhow::bail!(FieldMoveError::TargetNotSmashableRock {
                move_id: rule.move_id.clone(),
                movement: object.spritemovedata.clone(),
            });
        }
        Ok((object.object_identifier.clone(), object.event_flag.clone()))
    }

    fn hide_direct_field_object(
        state: &mut GameState,
        overworld: &mut OverworldSession,
        object_identifier: Option<&str>,
        event_flag: &str,
    ) -> Result<Option<String>> {
        if event_flag == "-1" {
            let object_identifier = object_identifier.with_context(
                || "direct field object with event flag -1 has no object identifier",
            )?;
            overworld
                .hidden_object_identifiers
                .insert(object_identifier.to_string());
            return Ok(None);
        }
        if !is_hideable_object_event_flag(event_flag) {
            anyhow::bail!("direct field object cannot be hidden with event flag {event_flag}");
        }
        state
            .flags
            .set_event_flag(event_flag, true)
            .with_context(|| format!("hide direct field object event flag {event_flag}"))?;
        overworld.sync_event_flag_memory(&state.flags);
        Ok(Some(event_flag.to_string()))
    }

    fn validate_runtime_field_move_tile_alignment(move_id: &str, tile: TilePosition) -> Result<()> {
        let _ = (move_id, tile);
        Ok(())
    }

    fn checked_runtime_field_move_target(
        move_id: &str,
        tile: TilePosition,
        facing: Direction,
    ) -> Result<TilePosition> {
        Self::validate_runtime_field_move_tile_alignment(move_id, tile)?;
        checked_move_by_stride(tile, facing, StepOptions::default().stride_tiles).ok_or_else(|| {
            anyhow::anyhow!(FieldMoveError::RuntimeTileOverflow {
                move_id: move_id.to_string(),
                x: tile.x,
                y: tile.y,
            })
        })
    }

    pub fn use_headbutt_field_move(
        &self,
        state: &mut GameState,
        overworld: &OverworldSession,
        party_index: usize,
        player_id: u16,
    ) -> Result<DirectFieldEncounterMoveOutcome> {
        self.require_no_active_battle(state, "HEADBUTT field move")?;
        self.validate_direct_field_move_actor(state, party_index, "HEADBUTT")?;
        let target = Self::checked_runtime_field_move_target(
            "HEADBUTT",
            overworld.player.tile,
            overworld.player.facing,
        )?;
        Self::validate_direct_headbutt_target(
            &self.field_moves.headbutt,
            &overworld.map,
            &overworld.tileset,
            target,
        )?;
        let mut rng = Random::new_crystal(state.rng_seed);
        let field_encounter =
            self.roll_headbutt_encounter(&overworld.map.name, target, player_id, &mut rng)?;
        state.commit_rng_seed(rng.seed());
        let wild_battle = self.start_field_encounter_battle(state, &field_encounter)?;
        Ok(DirectFieldEncounterMoveOutcome {
            field_encounter,
            wild_battle,
            removed_object_identifier: None,
            removed_event_flag: None,
        })
    }

    pub fn use_rock_smash_field_move(
        &self,
        state: &mut GameState,
        overworld: &mut OverworldSession,
        party_index: usize,
    ) -> Result<DirectFieldEncounterMoveOutcome> {
        self.require_no_active_battle(state, "ROCK_SMASH field move")?;
        self.validate_direct_field_move_actor(state, party_index, "ROCK_SMASH")?;
        let target = Self::checked_runtime_field_move_target(
            "ROCK_SMASH",
            overworld.player.tile,
            overworld.player.facing,
        )?;
        let (removed_object_identifier, removed_event_flag) =
            Self::validate_direct_rock_smash_target(
                &self.field_moves.rock_smash,
                overworld,
                target,
            )?;
        let removed_event_flag = Self::hide_direct_field_object(
            state,
            overworld,
            removed_object_identifier.as_deref(),
            &removed_event_flag,
        )?;
        let mut rng = Random::new_crystal(state.rng_seed);
        let field_encounter =
            self.roll_rock_smash_encounter(&overworld.map.name, target, &mut rng)?;
        state.commit_rng_seed(rng.seed());
        let wild_battle = self.start_field_encounter_battle(state, &field_encounter)?;
        Ok(DirectFieldEncounterMoveOutcome {
            field_encounter,
            wild_battle,
            removed_object_identifier,
            removed_event_flag,
        })
    }

    pub fn roll_sweet_scent_encounter(
        &self,
        map_name: &str,
        surface: EncounterSurface,
        time: TimeOfDay,
        tile: TilePosition,
        rng: &mut Random,
    ) -> Result<WildEncounterRoll> {
        self.validate_runtime_map_tile("SWEET_SCENT encounter", map_name, tile)?;
        let encounters = self.require_wild_encounters_for_map(map_name)?;
        core_roll_sweet_scent_encounter(
            encounters,
            &self.encounter_slot_tables,
            surface,
            time,
            tile,
            rng,
        )
        .map_err(|error| anyhow::anyhow!("roll SWEET_SCENT encounter on {map_name}: {error:?}"))
    }

    pub fn use_sweet_scent_field_move(
        &self,
        state: &mut GameState,
        overworld: &OverworldSession,
        party_index: usize,
        surface: EncounterSurface,
    ) -> Result<SweetScentFieldMoveOutcome> {
        self.require_no_active_battle(state, "SWEET_SCENT field move")?;
        let actor = self.validate_direct_field_move_actor(state, party_index, "SWEET_SCENT")?;
        let mut rng = Random::new_crystal(state.rng_seed);
        let wild_encounter = self.roll_sweet_scent_encounter(
            &overworld.map.name,
            surface,
            state.time.time_of_day,
            overworld.player.tile,
            &mut rng,
        )?;
        state.commit_rng_seed(wild_encounter.rng_seed_after);
        let wild_battle = self.start_wild_battle(state, wild_encounter.clone())?;
        Ok(SweetScentFieldMoveOutcome {
            actor_party_index: party_index,
            actor_species: actor.actor_species,
            wild_encounter,
            wild_battle,
        })
    }

    pub fn check_wild_encounter(
        &self,
        session: &OverworldSession,
        rng: &mut Random,
        options: EncounterCheckOptions,
    ) -> Result<Option<WildEncounterRoll>> {
        self.validate_runtime_map_tile(
            "wild encounter check",
            &session.map.name,
            session.player.tile,
        )?;
        let Some(encounters) = self.wild_encounters_for_map(&session.map.name) else {
            return Ok(None);
        };
        session
            .check_wild_encounter(
                encounters,
                &self.encounter_slot_tables,
                &self.encounter_music_modifiers,
                rng,
                options,
            )
            .with_context(|| format!("check wild encounters on {}", session.map.name))
    }

    pub fn check_coord_event_after_step(
        &self,
        state: &GameState,
        session: &OverworldSession,
    ) -> Option<CoordEventTrigger> {
        self.check_coord_event_after_step_checked(state, session)
            .ok()
            .flatten()
    }

    pub fn check_coord_event_after_step_checked(
        &self,
        state: &GameState,
        session: &OverworldSession,
    ) -> Result<Option<CoordEventTrigger>> {
        let current_scene = state
            .scenes
            .map_scenes
            .get(&session.map.name)
            .map(String::as_str);
        session
            .check_coord_event_checked(current_scene)
            .with_context(|| format!("check coord event on {}", session.map.name))
    }

    pub fn check_wild_encounter_after_step(
        &self,
        state: &mut GameState,
        session: &OverworldSession,
    ) -> Result<Option<WildEncounterRoll>> {
        // RandomEncounter calls CheckWildEncounterCooldown before checking
        // terrain or consuming encounter RNG. Crystal allows the check which
        // decrements 1 to 0; values remaining above zero suppress this step.
        if state.wild_encounter_cooldown > 0 {
            state.wild_encounter_cooldown -= 1;
            if state.wild_encounter_cooldown > 0 {
                return Ok(None);
            }
        }
        if state
            .flags
            .is_engine_flag_set("STATUSFLAGS_NO_WILD_ENCOUNTERS_F")
            .map_err(|error| anyhow::anyhow!("check NO_WILD_ENCOUNTERS flag: {error}"))?
        {
            return Ok(None);
        }
        let mut rng = Random::new_crystal(state.rng_seed);
        let roaming_candidates = self.roaming_candidates_for_map(state, &session.map.name);
        let special_wild_encounters = if state.bug_contest.timer_active {
            vec![
                SpecialWildEncounterEntry {
                    percent: 20,
                    species: "CATERPIE".to_string(),
                    min_level: 7,
                    max_level: 18,
                },
                SpecialWildEncounterEntry {
                    percent: 20,
                    species: "WEEDLE".to_string(),
                    min_level: 7,
                    max_level: 18,
                },
                SpecialWildEncounterEntry {
                    percent: 10,
                    species: "METAPOD".to_string(),
                    min_level: 9,
                    max_level: 18,
                },
                SpecialWildEncounterEntry {
                    percent: 10,
                    species: "KAKUNA".to_string(),
                    min_level: 9,
                    max_level: 18,
                },
                SpecialWildEncounterEntry {
                    percent: 5,
                    species: "BUTTERFREE".to_string(),
                    min_level: 12,
                    max_level: 15,
                },
                SpecialWildEncounterEntry {
                    percent: 5,
                    species: "BEEDRILL".to_string(),
                    min_level: 12,
                    max_level: 15,
                },
                SpecialWildEncounterEntry {
                    percent: 10,
                    species: "VENONAT".to_string(),
                    min_level: 10,
                    max_level: 16,
                },
                SpecialWildEncounterEntry {
                    percent: 10,
                    species: "PARAS".to_string(),
                    min_level: 10,
                    max_level: 17,
                },
                SpecialWildEncounterEntry {
                    percent: 5,
                    species: "SCYTHER".to_string(),
                    min_level: 13,
                    max_level: 14,
                },
                SpecialWildEncounterEntry {
                    percent: 5,
                    species: "PINSIR".to_string(),
                    min_level: 13,
                    max_level: 14,
                },
                SpecialWildEncounterEntry {
                    percent: 255,
                    species: "VENOMOTH".to_string(),
                    min_level: 30,
                    max_level: 40,
                },
            ]
        } else {
            Vec::new()
        };
        let active_repel_item = if state.repel_steps_remaining > 0 {
            state.active_repel_item.clone()
        } else {
            None
        };
        let environment = &self
            .runtime_map_metadata_for_name(&session.map.name)?
            .environment;
        let tileset_name = self.map_tileset_name(&session.map.name)?;
        let land_encounters_on_any_land = environment.eq_ignore_ascii_case("cave")
            || tileset_name.eq_ignore_ascii_case("cave")
            || tileset_name.eq_ignore_ascii_case("dark_cave");
        let roll = self.check_wild_encounter(
            session,
            &mut rng,
            EncounterCheckOptions {
                time: state.time.time_of_day,
                music_token: state.script_runtime.current_music.clone(),
                has_cleanse_tag: Self::party_has_cleanse_tag(state),
                active_repel_item,
                lead_party_level: leading_usable_party_level(state),
                roaming_candidates,
                special_wild_encounters,
                land_encounters_on_any_land,
            },
        )?;
        if let Some(encounter) = roll.as_ref().and_then(|roll| roll.resolved.as_ref())
            && roll
                .as_ref()
                .is_some_and(|roll| roll.slot_percent_roll.is_none())
        {
            state.script_runtime.variables.insert(
                "_roaming_encounter_species".to_string(),
                encounter.encounter.species.clone(),
            );
        }
        state.commit_rng_seed(rng.seed());
        Ok(roll)
    }

    fn roaming_candidates_for_map(
        &self,
        state: &GameState,
        map_name: &str,
    ) -> Vec<Option<(String, u8)>> {
        let Some(metadata) = self
            .runtime_map_metadata
            .values()
            .find(|metadata| metadata.name == map_name)
        else {
            return Vec::new();
        };
        state
            .roaming_pokemon
            .iter()
            .take(3)
            .map(|roamer| {
                (roamer.map_group == metadata.group_id && roamer.map_number == metadata.map_id)
                    .then(|| (roamer.species.clone(), roamer.level))
            })
            .collect()
    }

    /// Crystal's `UpdateRoamMons` runs as maps are loaded. Keep the exact
    /// sixteen-entry route graph from `data/wild/roammon_maps.asm`; the
    /// previous map is excluded from normal adjacency selection and the
    /// one-in-32 random jump is handled with `RandomRange`.
    fn update_roam_mons_on_map_change(
        &self,
        state: &mut GameState,
        current_map: &str,
    ) -> Result<()> {
        if state.roaming_pokemon.is_empty() {
            return Ok(());
        }
        const ROUTE_GRAPH: &[(&str, &[&str])] = &[
            ("ROUTE_29", &["ROUTE_30", "ROUTE_46"]),
            ("ROUTE_30", &["ROUTE_29", "ROUTE_31"]),
            ("ROUTE_31", &["ROUTE_30", "ROUTE_32", "ROUTE_36"]),
            ("ROUTE_32", &["ROUTE_36", "ROUTE_31", "ROUTE_33"]),
            ("ROUTE_33", &["ROUTE_32", "ROUTE_34"]),
            ("ROUTE_34", &["ROUTE_33", "ROUTE_35"]),
            ("ROUTE_35", &["ROUTE_34", "ROUTE_36"]),
            (
                "ROUTE_36",
                &["ROUTE_35", "ROUTE_31", "ROUTE_32", "ROUTE_37"],
            ),
            ("ROUTE_37", &["ROUTE_36", "ROUTE_38", "ROUTE_42"]),
            ("ROUTE_38", &["ROUTE_37", "ROUTE_39", "ROUTE_42"]),
            ("ROUTE_39", &["ROUTE_38"]),
            (
                "ROUTE_42",
                &["ROUTE_43", "ROUTE_44", "ROUTE_37", "ROUTE_38"],
            ),
            ("ROUTE_43", &["ROUTE_42", "ROUTE_44"]),
            ("ROUTE_44", &["ROUTE_42", "ROUTE_43", "ROUTE_45"]),
            ("ROUTE_45", &["ROUTE_44", "ROUTE_46"]),
            ("ROUTE_46", &["ROUTE_45", "ROUTE_29"]),
        ];
        let map_id = |constant: &str| {
            self.runtime_map_metadata
                .values()
                .find(|metadata| metadata.constant == constant)
                .map(|metadata| (metadata.group_id, metadata.map_id))
        };
        let Some(current_ids) = self
            .runtime_map_metadata
            .values()
            .find(|metadata| metadata.name == current_map)
            .map(|metadata| (metadata.group_id, metadata.map_id))
        else {
            return Ok(());
        };
        let previous = (
            state
                .script_runtime
                .variables
                .get("_roam_last_group")
                .and_then(|value| value.parse::<u16>().ok()),
            state
                .script_runtime
                .variables
                .get("_roam_last_number")
                .and_then(|value| value.parse::<u16>().ok()),
        );
        let mut rng = Random::new_crystal(state.rng_seed);
        for roamer in state.roaming_pokemon.iter_mut() {
            let Some(origin) = self.runtime_map_metadata.values().find(|metadata| {
                metadata.group_id == roamer.map_group && metadata.map_id == roamer.map_number
            }) else {
                continue;
            };
            let Some((_, connections)) = ROUTE_GRAPH
                .iter()
                .find(|(constant, _)| *constant == origin.constant)
            else {
                continue;
            };
            let target = loop {
                let roll = rng.battle_random_byte();
                if roll & 0x1f == 0 {
                    let index = rng.randrange(16) as usize;
                    let candidate = ROUTE_GRAPH[index].0;
                    if candidate != origin.constant {
                        break map_id(candidate);
                    }
                    continue;
                }
                let index = usize::from(roll & 0x03);
                let Some(candidate) = connections.get(index) else {
                    continue;
                };
                let Some(candidate_ids) = map_id(candidate) else {
                    continue;
                };
                if previous == (Some(candidate_ids.0), Some(candidate_ids.1)) {
                    continue;
                }
                break Some(candidate_ids);
            };
            if let Some((group, number)) = target {
                roamer.map_group = group;
                roamer.map_number = number;
            }
        }
        state
            .script_runtime
            .variables
            .insert("_roam_last_group".to_string(), current_ids.0.to_string());
        state
            .script_runtime
            .variables
            .insert("_roam_last_number".to_string(), current_ids.1.to_string());
        state.commit_rng_seed(rng.seed());
        Ok(())
    }

    fn finish_roaming_battle(&self, state: &mut GameState, species: &str, enemy: &Pokemon) {
        {
            let Some(roamer) = state
                .roaming_pokemon
                .iter_mut()
                .find(|roamer| roamer.species == species)
            else {
                return;
            };
            if enemy.hp == 0 {
                // Crystal marks a defeated/fled-from roamer as unavailable
                // while retaining its species slot for save validation.
                roamer.map_group = 0;
                roamer.map_number = 0;
                roamer.hp = 0;
                return;
            }
            roamer.hp = enemy.hp.min(u16::from(u8::MAX));
        }
        if let OverworldMemory::Active { map_name, .. } = &state.overworld {
            let map_name = map_name.clone();
            let _ = self.update_roam_mons_on_map_change(state, &map_name);
        }
    }

    fn party_has_cleanse_tag(state: &GameState) -> bool {
        state
            .storage
            .party
            .pokemon
            .iter()
            .flatten()
            .any(|pokemon| pokemon.item.as_deref() == Some("CLEANSE_TAG"))
    }

    pub fn start_resolved_wild_encounter_after_step(
        &self,
        state: &mut GameState,
        roll: &Option<WildEncounterRoll>,
    ) -> Result<Option<WildBattleStart>> {
        let Some(encounter) = roll.clone().filter(|roll| roll.resolved.is_some()) else {
            return Ok(None);
        };
        self.start_wild_battle(state, encounter).map(Some)
    }

    pub fn saved_wild_encounter_exists(&self, map_name: &str, species: &str, level: u8) -> bool {
        self.wild_encounters
            .get(map_name)
            .is_some_and(|encounters| wild_encounter_data_has(encounters, species, level))
            || self
                .field_encounters
                .get(map_name)
                .is_some_and(|encounters| field_encounter_data_has(encounters, species, level))
            || self.map_fishing_encounter_has(map_name, species, level)
    }

    pub fn map_fishing_encounter_has(&self, map_name: &str, species: &str, level: u8) -> bool {
        let Some(group_name) = self
            .maps
            .get(map_name)
            .and_then(|module| module.attributes.fishing_group.as_deref())
        else {
            return false;
        };
        let Some(group) = self.fishing.groups.get(group_name) else {
            return false;
        };
        group
            .rod_tables
            .values()
            .flat_map(|table| table.slots.iter())
            .any(|slot| fishing_slot_has(&self.fishing.time_groups, slot, species, level))
    }

    pub fn pokegear_landmark_for_map(
        &self,
        map_name: &str,
    ) -> Result<&crystal_core::models::display_metadata::PokegearLandmark> {
        let landmark_constant = self
            .pokegear_landmarks
            .map_to_landmark
            .get(map_name)
            .with_context(|| {
                format!("town map missing exact landmark mapping for map {map_name}")
            })?;
        self.pokegear_landmarks
            .landmarks
            .iter()
            .find(|landmark| landmark.constant == *landmark_constant)
            .with_context(|| {
                format!(
                    "town map landmark mapping for map {map_name} points to missing landmark {landmark_constant}"
                )
            })
    }

    pub fn saved_event_flag_exists(&self, flag: &str) -> bool {
        self.initialize_events
            .event_flags
            .iter()
            .any(|known| known == flag)
            || self
                .fruit_trees
                .0
                .keys()
                .any(|tree_id| fruit_tree_collected_flag(tree_id) == flag)
            || self.saved_story_event_constant_declares_flag(flag)
            || self.bug_contest_config.as_ref().is_some_and(|config| {
                config
                    .contestant_flags
                    .iter()
                    .any(|contestant_flag| contestant_flag == flag)
            })
            || self.maps.values().any(|module| {
                module.script_flag_commands.iter().any(|command| {
                    command.flag_id == flag && !crystal_core::state::is_engine_flag_name(flag)
                }) || module
                    .objects
                    .iter()
                    .any(|object| object.event_flag == flag)
                    || module.scripted_trainer_battles.iter().any(|battle| {
                        battle
                            .post_battle_event_flags
                            .iter()
                            .any(|known| known == flag)
                    })
                    || module.scripted_wild_battles.iter().any(|battle| {
                        battle
                            .pre_battle_event_flags
                            .iter()
                            .any(|known| known == flag)
                            || battle
                                .post_battle_event_flags
                                .iter()
                                .any(|known| known == flag)
                    })
            })
    }

    pub fn saved_engine_flag_exists(&self, flag: &str) -> bool {
        self.initialize_events
            .engine_flags
            .iter()
            .any(|known| known == flag)
            || self.field_moves.strength.engine_flag == flag
            || self.field_moves.flash.engine_flag == flag
            || self.saved_story_event_constant_declares_flag(flag)
            || self.maps.values().any(|module| {
                module.script_flag_commands.iter().any(|command| {
                    command.flag_id == flag && crystal_core::state::is_engine_flag_name(flag)
                }) || module.scripted_trainer_battles.iter().any(|battle| {
                    battle
                        .post_battle_script_flags
                        .iter()
                        .any(|known| known == flag)
                }) || module.scripted_wild_battles.iter().any(|battle| {
                    battle
                        .post_battle_script_flags
                        .iter()
                        .any(|known| known == flag)
                })
            })
    }

    pub fn saved_story_event_constant_declares_flag(&self, flag: &str) -> bool {
        self.story_event_script_constants.global.contains_key(flag)
            || self
                .story_event_script_constants
                .maps
                .values()
                .any(|constants| constants.contains_key(flag))
    }

    pub fn saved_text_exists(&self, text_label: &str) -> bool {
        self.asm_text.contains_key(text_label)
            || self
                .maps
                .values()
                .any(|module| module.script_text_bodies.contains_key(text_label))
    }

    pub fn saved_pokemon_status_exists(&self, status: &str) -> bool {
        self.step_event_rules.poison_status == status
            || self.capture_rules.status_bonus.contains_key(status)
            || self
                .items
                .values()
                .any(|item| item.status_heals.iter().any(|healed| healed == status))
            || status == "POKERUS"
    }

    pub fn tileset_collision(&self, tileset_name: &str) -> Result<TilesetCollision> {
        let definition = self
            .tilesets
            .get(tileset_name)
            .with_context(|| format!("compiled game pack missing tileset '{tileset_name}'"))?;
        tileset_collision_from_definition(tileset_name, definition)
    }

    #[cfg(test)]
    pub(crate) fn load_base_json(asset_root: &AssetRoot) -> Result<Self> {
        let index = asset_root.load_content_pack_index()?;
        Self::load_from_content_pack_index(asset_root, &index)
    }

    fn load_base_json_for_compile(asset_root: &AssetRoot) -> Result<Self> {
        let index = asset_root.load_raw_content_pack_index_for_compile()?;
        Self::load_from_content_pack_index(asset_root, &index)
    }

    fn load_from_content_pack_index(
        asset_root: &AssetRoot,
        index: &ContentPackIndex,
    ) -> Result<Self> {
        let mut data = Self::default();
        data.apply_content_pack_index(asset_root, index)?;
        Ok(data)
    }

    pub(crate) fn apply_content_pack_index(
        &mut self,
        asset_root: &AssetRoot,
        index: &ContentPackIndex,
    ) -> Result<()> {
        index.validate()?;
        for pack in index.enabled_packs_sorted() {
            if let Some(compiled_path) = &pack.compiled {
                if self != &Self::default() {
                    anyhow::bail!(
                        "compiled game pack '{}' must be applied to an empty runtime dataset",
                        pack.id
                    );
                }
                let compiled_path = resolve_content_pack_compiled_game_pack_path(
                    asset_root,
                    &pack.id,
                    compiled_path,
                )?;
                let compiled = read_verified_compiled_game_pack(&compiled_path)
                    .with_context(|| format!("load compiled game pack {}", pack.id))?;
                let runtime_id = compiled.runtime_modpack_id()?;
                if runtime_id != pack.id {
                    anyhow::bail!(
                        "compiled game pack {} declared runtime modpack id {}",
                        pack.id,
                        runtime_id
                    );
                }
                *self = compiled.data().clone();
                continue;
            }

            for category in CONTENT_PACK_CATEGORIES {
                let mut seen_entries = BTreeSet::new();
                for entry in pack.files.entries(*category) {
                    if !seen_entries.insert(entry.as_str()) {
                        anyhow::bail!(
                            "content pack {} category {} includes duplicate file entry {}",
                            pack.id,
                            category.as_str(),
                            entry
                        );
                    }
                    if *category == ContentPackCategory::Audio {
                        validate_content_pack_audio_metadata_entry(&pack.id, entry)?;
                    } else {
                        validate_content_pack_json_entry(&pack.id, *category, entry)?;
                    }
                }
                for entry in pack.files.entries(*category) {
                    let path = resolve_content_pack_data_path(asset_root, &pack.id, entry)?;
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
                for (species_id, species) in parse_object_map::<PokemonSpecies>(payload)? {
                    insert_keyed_pokemon_species(&mut self.pokemon, species_id, species)?;
                }
            }
            ContentPackCategory::Moves => {
                for (move_id, move_data) in parse_object_map::<Move>(payload)? {
                    validate_manifest_move(&move_data)?;
                    insert_keyed_move_data(&mut self.moves, move_id, move_data)?;
                }
            }
            ContentPackCategory::GrowthRates => {
                for (curve_id, curve) in
                    parse_object_map::<crystal_core::systems::experience::GrowthRateCurve>(payload)?
                {
                    insert_keyed_growth_rate_curve(&mut self.growth_rates, curve_id, curve)?;
                }
            }
            ContentPackCategory::Items => {
                for (item_id, item) in parse_object_map::<Item>(payload)? {
                    validate_manifest_item(&item)?;
                    insert_keyed_item(&mut self.items, item_id, item)?;
                }
            }
            ContentPackCategory::Marts => {
                merge_mart_payload(&mut self.marts, payload)?;
            }
            ContentPackCategory::CurrencyConstants => {
                merge_currency_constants_payload(&mut self.currency_constants, payload)?;
            }
            ContentPackCategory::WildEncounters => {
                for (map_name, data) in parse_object_map::<WildEncounterData>(payload)? {
                    insert_keyed_wild_encounter_data(&mut self.wild_encounters, map_name, data)?;
                }
            }
            ContentPackCategory::FieldEncounters => {
                for (map_name, data) in parse_object_map::<FieldEncounterData>(payload)? {
                    insert_keyed_field_encounter_data(&mut self.field_encounters, map_name, data)?;
                }
            }
            ContentPackCategory::RuntimeSpawnPoints => {
                merge_runtime_spawn_points(
                    &mut self.runtime_spawn_points,
                    parse_object_map_with_description::<RuntimeSpawnPoint>(
                        payload,
                        "runtime spawn points payload",
                    )?,
                )?;
            }
            ContentPackCategory::RuntimeMapMetadata => {
                merge_runtime_map_metadata(
                    &mut self.runtime_map_metadata,
                    parse_object_map_with_description::<RuntimeMapMetadata>(
                        payload,
                        "runtime map metadata payload",
                    )?,
                )?;
            }
            ContentPackCategory::FleeMons => {
                insert_flee_mon_tables(&mut self.flee_mons, serde_json::from_value(payload)?)?;
            }
            ContentPackCategory::RoamingPokemon => {
                merge_roaming_pokemon(&mut self.roaming_pokemon, serde_json::from_value(payload)?)?;
            }
            ContentPackCategory::BuenaPasswordCategories => {
                merge_buena_password_categories(
                    &mut self.buena_password_categories,
                    serde_json::from_value(payload)?,
                )?;
            }
            ContentPackCategory::BuenaPrizes => {
                merge_buena_prizes(&mut self.buena_prizes, serde_json::from_value(payload)?)?;
            }
            ContentPackCategory::KurtApricornRecipes => {
                merge_kurt_apricorn_recipes(
                    &mut self.kurt_apricorn_recipes,
                    serde_json::from_value(payload)?,
                )?;
            }
            ContentPackCategory::ShuckieGift => {
                insert_shuckie_gift(&mut self.shuckie_gift, serde_json::from_value(payload)?)?;
            }
            ContentPackCategory::DratiniMoveSets => {
                merge_dratini_move_sets(
                    &mut self.dratini_move_sets,
                    serde_json::from_value(payload)?,
                )?;
            }
            ContentPackCategory::BugContestConfig => {
                insert_bug_contest_config(
                    &mut self.bug_contest_config,
                    serde_json::from_value(payload)?,
                )?;
            }
            ContentPackCategory::BattleTowerRules => {
                insert_battle_tower_rules(
                    &mut self.battle_tower_rules,
                    serde_json::from_value(payload)?,
                )?;
            }
            ContentPackCategory::OakRatings => {
                insert_oak_rating_table(&mut self.oak_ratings, serde_json::from_value(payload)?)?;
            }
            ContentPackCategory::OddEggDefinitions => {
                insert_odd_egg_definitions(
                    &mut self.odd_egg_definitions,
                    serde_json::from_value(payload)?,
                )?;
            }
            ContentPackCategory::MagikarpLengths => {
                insert_magikarp_length_table(
                    &mut self.magikarp_lengths,
                    serde_json::from_value(payload)?,
                )?;
            }
            ContentPackCategory::HappinessData => {
                insert_happiness_data(&mut self.happiness_data, serde_json::from_value(payload)?)?;
            }
            ContentPackCategory::EncounterSlotTables => {
                insert_encounter_slot_tables(
                    &mut self.encounter_slot_tables,
                    serde_json::from_value(payload)?,
                )?;
            }
            ContentPackCategory::EncounterMusicModifiers => {
                insert_encounter_music_modifiers(
                    &mut self.encounter_music_modifiers,
                    serde_json::from_value(payload)?,
                )?;
            }
            ContentPackCategory::BattleStatMultipliers => {
                insert_battle_stat_multiplier_tables(
                    &mut self.battle_stat_multipliers,
                    serde_json::from_value(payload)?,
                )?;
            }
            ContentPackCategory::CaptureWobbleProbabilities => {
                insert_capture_wobble_probabilities(
                    &mut self.capture_wobble_probabilities,
                    serde_json::from_value(payload)?,
                )?;
            }
            ContentPackCategory::CaptureRules => {
                insert_capture_rules(&mut self.capture_rules, serde_json::from_value(payload)?)?;
            }
            ContentPackCategory::BattleEscapeRules => {
                insert_battle_escape_rules(
                    &mut self.battle_escape_rules,
                    serde_json::from_value(payload)?,
                )?;
            }
            ContentPackCategory::MovePriorities => {
                insert_move_priority_table(
                    &mut self.move_priorities,
                    serde_json::from_value(payload)?,
                )?;
            }
            ContentPackCategory::TypeCategories => {
                insert_type_categories(
                    &mut self.type_categories,
                    serde_json::from_value(payload)?,
                )?;
            }
            ContentPackCategory::TypeEffectiveness => {
                insert_type_effectiveness(
                    &mut self.type_effectiveness,
                    serde_json::from_value(payload)?,
                )?;
            }
            ContentPackCategory::WeatherModifiers => {
                insert_weather_modifiers(
                    &mut self.weather_modifiers,
                    serde_json::from_value(payload)?,
                )?;
            }
            ContentPackCategory::BattleRewardRules => {
                insert_battle_reward_rules(
                    &mut self.battle_reward_rules,
                    serde_json::from_value(payload)?,
                )?;
            }
            ContentPackCategory::StepEventRules => {
                insert_step_event_rules(
                    &mut self.step_event_rules,
                    serde_json::from_value(payload)?,
                )?;
            }
            ContentPackCategory::Fishing => {
                insert_fishing_catalog(&mut self.fishing, serde_json::from_value(payload)?)?;
            }
            ContentPackCategory::FruitTrees => {
                merge_fruit_tree_payload(&mut self.fruit_trees, payload)?;
            }
            ContentPackCategory::FieldMoves => {
                insert_field_move_catalog(&mut self.field_moves, serde_json::from_value(payload)?)?;
            }
            ContentPackCategory::FieldBoxItems => {
                insert_field_box_items(
                    &mut self.field_box_items,
                    parse_object_map::<FieldBoxItemRule>(payload)?,
                )?;
            }
            ContentPackCategory::RuntimeTitleScreen => {
                insert_runtime_title_screen(
                    &mut self.runtime_title_screen,
                    serde_json::from_value(payload)?,
                )?;
            }
            ContentPackCategory::FlyDestinations => {
                for (flypoint_flag, destination) in parse_object_map::<FlyDestination>(payload)? {
                    insert_fly_destination(&mut self.fly_destinations, flypoint_flag, destination)?;
                }
            }
            ContentPackCategory::MapAttributes => {
                merge_map_attributes(
                    &mut self.map_attributes,
                    parse_object_map::<MapAttributes>(payload)?,
                )?;
            }
            ContentPackCategory::MapBlocks => {
                merge_map_block_payload(&mut self.map_blocks, payload)?;
            }
            ContentPackCategory::Learnsets => {
                merge_learnsets(&mut self.learnsets, parse_learnsets(payload)?)?;
            }
            ContentPackCategory::LevelUpMoves => {
                merge_level_up_moves_payload(&mut self.level_up_moves, payload)?;
            }
            ContentPackCategory::EggMoves => {
                merge_egg_moves_payload(&mut self.egg_moves, payload)?;
            }
            ContentPackCategory::Evolutions => {
                merge_evolution_payload(&mut self.evolutions, payload)?;
            }
            ContentPackCategory::Maps => {
                for (map_id, module) in parse_object_map::<MapModule>(payload)? {
                    insert_keyed_map_module(&mut self.maps, map_id, module)?;
                }
            }
            ContentPackCategory::MapScripts => {
                merge_map_script_payload(&mut self.map_scripts, payload)?;
            }
            ContentPackCategory::MapDimensions => {
                merge_map_dimensions_payload(&mut self.map_dimensions, payload)?;
            }
            ContentPackCategory::Npcs => {
                merge_npc_payload(&mut self.npcs, payload)?;
            }
            ContentPackCategory::PokegearLandmarks => {
                merge_pokegear_landmarks_payload(&mut self.pokegear_landmarks, payload)?;
            }
            ContentPackCategory::PcStrings => {
                merge_pc_strings(&mut self.pc_strings, parse_object_map::<String>(payload)?)?;
            }
            ContentPackCategory::MenuIcons => {
                merge_menu_icons(&mut self.menu_icons, parse_object_map::<String>(payload)?)?;
            }
            ContentPackCategory::Trainers => {
                for (trainer_id, trainer) in parse_object_map::<Trainer>(payload)? {
                    insert_keyed_trainer(&mut self.trainers, trainer_id, trainer)?;
                }
            }
            ContentPackCategory::Pokedex => {
                merge_pokedex_payload(&mut self.pokedex, payload)?;
            }
            ContentPackCategory::PokedexEntries => {
                for (species, entry) in parse_object_map::<RuntimePokedexEntry>(payload)? {
                    insert_keyed_pokedex_entry(&mut self.pokedex_entries, species, entry)?;
                }
            }
            ContentPackCategory::PokemonFrontpicAnim => {
                merge_frontpic_anim_programs(&mut self.pokemon_frontpic_anim, payload)?;
            }
            ContentPackCategory::InitializeEvents => {
                insert_initialize_events(
                    &mut self.initialize_events,
                    serde_json::from_value(payload)?,
                )?;
            }
            ContentPackCategory::StoryEventScriptConstants => {
                insert_story_event_script_constants(
                    &mut self.story_event_script_constants,
                    serde_json::from_value(payload)?,
                )?;
            }
            ContentPackCategory::StoryEvents => {
                merge_raw_story_event_payload(
                    &mut self.story_events,
                    payload,
                    "story event payload",
                    "story event payload key",
                )?;
            }
            ContentPackCategory::PhoneScripts => {
                merge_raw_script_payload(
                    &mut self.phone_scripts,
                    payload,
                    "phone script payload",
                    "phone script payload key",
                )?;
            }
            ContentPackCategory::PhoneContacts => {
                merge_phone_contact_payload(&mut self.phone_contacts, payload)?;
            }
            ContentPackCategory::PermanentPhoneNumbers => {
                merge_token_keyed_map(
                    &mut self.permanent_phone_numbers,
                    parse_token_keyed_rule_map(payload, "permanent phone number")?,
                    "permanent phone number",
                )?;
            }
            ContentPackCategory::SpecialPhoneCalls => {
                merge_token_keyed_map(
                    &mut self.special_phone_calls,
                    parse_token_keyed_rule_map(payload, "special phone call")?,
                    "special phone call",
                )?;
            }
            ContentPackCategory::NpcTrades => {
                merge_token_keyed_map(
                    &mut self.npc_trades,
                    parse_token_keyed_rule_map(payload, "NPC trade")?,
                    "NPC trade",
                )?;
            }
            ContentPackCategory::SpecialRoutines => {
                merge_special_routine_rules(
                    &mut self.special_routines,
                    parse_token_keyed_rule_map(payload, "special routine")?,
                )?;
            }
            ContentPackCategory::AsmText => {
                merge_asm_text(&mut self.asm_text, payload)?;
            }
            ContentPackCategory::MoveNames => {
                insert_exact_string_vec_table(
                    &mut self.move_names,
                    parse_string_vec_payload(payload, "move names")?,
                    "move names",
                    "move name",
                )?;
            }
            ContentPackCategory::BattleAnimations => {
                merge_token_keyed_string_vec_map(
                    &mut self.battle_animations,
                    parse_object_map_with_description::<Vec<String>>(
                        payload,
                        "battle animation payload",
                    )?,
                    "battle animation",
                    "battle animation command",
                )?;
            }
            ContentPackCategory::BattleAnimationTable => {
                insert_token_string_vec_table(
                    &mut self.battle_animation_table,
                    parse_string_vec_payload(payload, "battle animation table")?,
                    "battle animation",
                    "battle animation table entry",
                )?;
            }
            ContentPackCategory::BattleAnimBundle => {
                insert_exact_string_bundle(
                    &mut self.battle_anim_bundle,
                    serde_json::to_string(&payload).context("encode battle animation bundle")?,
                    "battle animation bundle",
                    &[
                        "objects",
                        "framesets",
                        "oam_sets",
                        "gfx_table",
                        "gfx_sources",
                    ],
                )?;
            }
            ContentPackCategory::SpriteAnimBundle => {
                insert_exact_string_bundle(
                    &mut self.sprite_anim_bundle,
                    serde_json::to_string(&payload).context("encode sprite animation bundle")?,
                    "sprite animation bundle",
                    &["oam_sets", "framesets", "objects"],
                )?;
            }
            ContentPackCategory::SpritePaletteDefaults => {
                merge_sprite_palette_defaults(&mut self.sprite_palette_defaults, payload)?;
            }
            ContentPackCategory::PokegearTownMapPaletteMap => {
                merge_token_keyed_token_vec_map(
                    &mut self.pokegear_town_map_palette_map,
                    parse_object_map_with_description::<Vec<String>>(
                        payload,
                        "Pokegear town map palette payload",
                    )?,
                    "Pokegear town map palette entry",
                    "Pokegear town map palette value",
                )?;
            }
            ContentPackCategory::PokemonCries => {
                merge_pokemon_cries(&mut self.pokemon_cries, payload)?;
            }
            ContentPackCategory::Audio => {
                for (audio_id, audio_asset) in parse_object_map::<ModpackAudioAsset>(payload)? {
                    insert_keyed_audio_asset(&mut self.audio, audio_id, audio_asset)?;
                }
            }
            ContentPackCategory::Tilesets => {
                for (tileset_id, tileset) in parse_object_map::<TilesetDefinition>(payload)? {
                    insert_keyed_tileset_definition(&mut self.tilesets, tileset_id, tileset)?;
                }
            }
            ContentPackCategory::Playability => {
                let playability: PlayabilityRules = serde_json::from_value(payload)?;
                merge_playability_rules(&mut self.playability, &playability)?;
            }
        }
        Ok(())
    }

    pub(crate) fn apply_modpack(&mut self, manifest: &ModpackManifest) -> Result<()> {
        if manifest.payload.pokemon.is_empty() {
            self.pokemon.clear();
        } else {
            for (species_id, species) in &manifest.payload.pokemon {
                insert_keyed_pokemon_species(
                    &mut self.pokemon,
                    species_id.clone(),
                    species.clone(),
                )?;
            }
        }
        if manifest.payload.moves.is_empty() {
            self.moves.clear();
        } else {
            for (move_id, move_data) in &manifest.payload.moves {
                validate_manifest_move(move_data)?;
                insert_keyed_move_data(&mut self.moves, move_id.clone(), move_data.clone())?;
            }
        }
        if manifest.payload.evolutions == EvolutionTable::default() {
            self.evolutions = EvolutionTable::default();
        } else {
            merge_evolution_table(&mut self.evolutions, &manifest.payload.evolutions)?;
        }
        if manifest.payload.marts == MartCatalog::default() {
            self.marts = MartCatalog::default();
        } else {
            merge_mart_catalog(&mut self.marts, &manifest.payload.marts)?;
        }
        if manifest.payload.currency_constants.0.is_empty() {
            self.currency_constants.0.clear();
        } else {
            merge_currency_constants(
                &mut self.currency_constants,
                &manifest.payload.currency_constants,
            )?;
        }
        if manifest.payload.battle_reward_rules == BattleRewardRules::default() {
            self.battle_reward_rules = BattleRewardRules::default();
        } else {
            insert_battle_reward_rules(
                &mut self.battle_reward_rules,
                manifest.payload.battle_reward_rules.clone(),
            )?;
        }
        if manifest.payload.step_event_rules == StepEventRules::default() {
            self.step_event_rules = StepEventRules::default();
        } else {
            insert_step_event_rules(
                &mut self.step_event_rules,
                manifest.payload.step_event_rules.clone(),
            )?;
        }
        if manifest.payload.maps.is_empty() {
            self.maps.clear();
        } else {
            for (map_id, map) in &manifest.payload.maps {
                insert_keyed_map_module(&mut self.maps, map_id.clone(), map.clone())?;
            }
        }
        let move_ids: BTreeSet<String> = self.moves.keys().cloned().collect();
        if manifest.payload.items.is_empty() {
            self.items.clear();
        } else {
            for (item_id, item) in &manifest.payload.items {
                validate_manifest_item(item)?;
                validate_manifest_item_references(item, &move_ids)?;
                insert_keyed_item(&mut self.items, item_id.clone(), item.clone())?;
            }
        }
        if manifest.payload.wild_encounters.is_empty() {
            self.wild_encounters.clear();
        } else {
            for (map_name, wild_encounter_data) in &manifest.payload.wild_encounters {
                insert_keyed_wild_encounter_data(
                    &mut self.wild_encounters,
                    map_name.clone(),
                    wild_encounter_data.clone(),
                )?;
            }
        }
        if manifest.payload.field_encounters.is_empty() {
            self.field_encounters.clear();
        } else {
            for (map_name, field_encounter_data) in &manifest.payload.field_encounters {
                insert_keyed_field_encounter_data(
                    &mut self.field_encounters,
                    map_name.clone(),
                    field_encounter_data.clone(),
                )?;
            }
        }
        if manifest.payload.fishing == FishingCatalog::default() {
            self.fishing = FishingCatalog::default();
        } else {
            insert_fishing_catalog(&mut self.fishing, manifest.payload.fishing.clone())?;
        }
        if manifest.payload.fruit_trees.0.is_empty() {
            self.fruit_trees.0.clear();
        } else {
            merge_fruit_tree_catalog(&mut self.fruit_trees, &manifest.payload.fruit_trees)?;
        }
        if manifest.payload.field_moves == FieldMoveCatalog::default() {
            self.field_moves = FieldMoveCatalog::default();
        } else {
            insert_field_move_catalog(&mut self.field_moves, manifest.payload.field_moves.clone())?;
        }
        if manifest.payload.field_box_items.is_empty() {
            self.field_box_items.clear();
        } else {
            insert_field_box_items(
                &mut self.field_box_items,
                manifest.payload.field_box_items.clone(),
            )?;
        }
        if manifest.payload.runtime_title_screen == RuntimeTitleScreen::default() {
            self.runtime_title_screen = RuntimeTitleScreen::default();
        } else {
            insert_runtime_title_screen(
                &mut self.runtime_title_screen,
                manifest.payload.runtime_title_screen.clone(),
            )?;
        }
        if manifest.payload.runtime_spawn_points.is_empty() {
            self.runtime_spawn_points.clear();
        } else {
            merge_runtime_spawn_points(
                &mut self.runtime_spawn_points,
                manifest
                    .payload
                    .runtime_spawn_points
                    .iter()
                    .map(|(key, value)| (key.clone(), value.clone()))
                    .collect(),
            )?;
        }
        if manifest.payload.runtime_map_metadata.is_empty() {
            self.runtime_map_metadata.clear();
        } else {
            merge_runtime_map_metadata(
                &mut self.runtime_map_metadata,
                manifest
                    .payload
                    .runtime_map_metadata
                    .iter()
                    .map(|(key, value)| (key.clone(), value.clone()))
                    .collect(),
            )?;
        }
        if manifest.payload.flee_mons == FleeMonTables::default() {
            self.flee_mons = FleeMonTables::default();
        } else {
            insert_flee_mon_tables(&mut self.flee_mons, manifest.payload.flee_mons.clone())?;
        }
        if manifest.payload.roaming_pokemon.is_empty() {
            self.roaming_pokemon.clear();
        } else {
            merge_roaming_pokemon(
                &mut self.roaming_pokemon,
                manifest.payload.roaming_pokemon.clone(),
            )?;
        }
        if manifest
            .payload
            .buena_password_categories
            .categories
            .is_empty()
            && manifest.payload.buena_password_categories.order.is_empty()
        {
            self.buena_password_categories = BuenaPasswordCategories::default();
        } else {
            merge_buena_password_categories(
                &mut self.buena_password_categories,
                manifest.payload.buena_password_categories.clone(),
            )?;
        }
        if manifest.payload.buena_prizes.is_empty() {
            self.buena_prizes.clear();
        } else {
            merge_buena_prizes(
                &mut self.buena_prizes,
                manifest.payload.buena_prizes.clone(),
            )?;
        }
        if manifest.payload.kurt_apricorn_recipes.is_empty() {
            self.kurt_apricorn_recipes.clear();
        } else {
            merge_kurt_apricorn_recipes(
                &mut self.kurt_apricorn_recipes,
                manifest.payload.kurt_apricorn_recipes.clone(),
            )?;
        }
        if let Some(shuckie_gift) = &manifest.payload.shuckie_gift {
            insert_shuckie_gift(&mut self.shuckie_gift, shuckie_gift.clone())?;
        } else {
            self.shuckie_gift = None;
        }
        if manifest.payload.dratini_move_sets.is_empty() {
            self.dratini_move_sets.clear();
        } else {
            merge_dratini_move_sets(
                &mut self.dratini_move_sets,
                manifest.payload.dratini_move_sets.clone(),
            )?;
        }
        if let Some(bug_contest_config) = &manifest.payload.bug_contest_config {
            insert_bug_contest_config(&mut self.bug_contest_config, bug_contest_config.clone())?;
        } else {
            self.bug_contest_config = None;
        }
        if let Some(battle_tower_rules) = &manifest.payload.battle_tower_rules {
            insert_battle_tower_rules(&mut self.battle_tower_rules, battle_tower_rules.clone())?;
        } else {
            self.battle_tower_rules = None;
        }
        if manifest.payload.oak_ratings.is_empty() {
            self.oak_ratings.clear();
        } else {
            insert_oak_rating_table(&mut self.oak_ratings, manifest.payload.oak_ratings.clone())?;
        }
        if manifest.payload.odd_egg_definitions.is_empty() {
            self.odd_egg_definitions.clear();
        } else {
            insert_odd_egg_definitions(
                &mut self.odd_egg_definitions,
                manifest.payload.odd_egg_definitions.clone(),
            )?;
        }
        if manifest.payload.magikarp_lengths.is_empty() {
            self.magikarp_lengths.clear();
        } else {
            insert_magikarp_length_table(
                &mut self.magikarp_lengths,
                manifest.payload.magikarp_lengths.clone(),
            )?;
        }
        if let Some(happiness_data) = &manifest.payload.happiness_data {
            insert_happiness_data(&mut self.happiness_data, happiness_data.clone())?;
        } else {
            self.happiness_data = None;
        }
        if manifest.payload.encounter_slot_tables == EncounterSlotTables::default() {
            self.encounter_slot_tables = EncounterSlotTables::default();
        } else {
            insert_encounter_slot_tables(
                &mut self.encounter_slot_tables,
                manifest.payload.encounter_slot_tables.clone(),
            )?;
        }
        if manifest.payload.encounter_music_modifiers == EncounterMusicModifiers::default() {
            self.encounter_music_modifiers = EncounterMusicModifiers::default();
        } else {
            insert_encounter_music_modifiers(
                &mut self.encounter_music_modifiers,
                manifest.payload.encounter_music_modifiers.clone(),
            )?;
        }
        if manifest.payload.battle_stat_multipliers == BattleStatMultiplierTables::default() {
            self.battle_stat_multipliers = BattleStatMultiplierTables::default();
        } else {
            insert_battle_stat_multiplier_tables(
                &mut self.battle_stat_multipliers,
                manifest.payload.battle_stat_multipliers.clone(),
            )?;
        }
        if manifest.payload.capture_wobble_probabilities.is_empty() {
            self.capture_wobble_probabilities.clear();
        } else {
            insert_capture_wobble_probabilities(
                &mut self.capture_wobble_probabilities,
                manifest.payload.capture_wobble_probabilities.clone(),
            )?;
        }
        if manifest.payload.capture_rules == CaptureRules::default() {
            self.capture_rules = CaptureRules::default();
        } else {
            insert_capture_rules(
                &mut self.capture_rules,
                manifest.payload.capture_rules.clone(),
            )?;
        }
        if manifest.payload.battle_escape_rules == BattleEscapeRules::default() {
            self.battle_escape_rules = BattleEscapeRules::default();
        } else {
            insert_battle_escape_rules(
                &mut self.battle_escape_rules,
                manifest.payload.battle_escape_rules.clone(),
            )?;
        }
        if manifest.payload.move_priorities == MovePriorityTable::default() {
            self.move_priorities = MovePriorityTable::default();
        } else {
            insert_move_priority_table(
                &mut self.move_priorities,
                manifest.payload.move_priorities.clone(),
            )?;
        }
        if manifest.payload.type_categories == TypeCategories::default() {
            self.type_categories = TypeCategories::default();
        } else {
            insert_type_categories(
                &mut self.type_categories,
                manifest.payload.type_categories.clone(),
            )?;
        }
        if manifest.payload.type_effectiveness == TypeEffectivenessTable::default() {
            self.type_effectiveness = TypeEffectivenessTable::default();
        } else {
            insert_type_effectiveness(
                &mut self.type_effectiveness,
                manifest.payload.type_effectiveness.clone(),
            )?;
        }
        if manifest.payload.weather_modifiers == WeatherModifiers::default() {
            self.weather_modifiers = WeatherModifiers::default();
        } else {
            insert_weather_modifiers(
                &mut self.weather_modifiers,
                manifest.payload.weather_modifiers.clone(),
            )?;
        }
        if manifest.payload.pc_strings.is_empty() {
            self.pc_strings.clear();
        } else {
            merge_pc_strings(
                &mut self.pc_strings,
                manifest
                    .payload
                    .pc_strings
                    .iter()
                    .map(|(key, value)| (key.clone(), value.clone()))
                    .collect(),
            )?;
        }
        if manifest.payload.menu_icons.is_empty() {
            self.menu_icons.clear();
        } else {
            merge_menu_icons(
                &mut self.menu_icons,
                manifest
                    .payload
                    .menu_icons
                    .iter()
                    .map(|(key, value)| (key.clone(), value.clone()))
                    .collect(),
            )?;
        }
        if manifest.payload.pokedex_entries.is_empty() {
            self.pokedex_entries.clear();
        } else {
            for (species, entry) in &manifest.payload.pokedex_entries {
                insert_keyed_pokedex_entry(
                    &mut self.pokedex_entries,
                    species.clone(),
                    entry.clone(),
                )?;
            }
        }
        if manifest.payload.pokemon_frontpic_anim.is_empty() {
            self.pokemon_frontpic_anim.clear();
        } else {
            merge_frontpic_anim_entries(
                &mut self.pokemon_frontpic_anim,
                manifest
                    .payload
                    .pokemon_frontpic_anim
                    .iter()
                    .map(|(key, value)| (key.clone(), value.clone()))
                    .collect(),
            )?;
        }
        if manifest.payload.initialize_events == InitializeEventsConfig::default() {
            self.initialize_events = InitializeEventsConfig::default();
        } else {
            insert_initialize_events(
                &mut self.initialize_events,
                manifest.payload.initialize_events.clone(),
            )?;
        }
        if manifest.payload.story_event_script_constants == StoryEventScriptConstants::default() {
            self.story_event_script_constants = StoryEventScriptConstants::default();
        } else {
            insert_story_event_script_constants(
                &mut self.story_event_script_constants,
                manifest.payload.story_event_script_constants.clone(),
            )?;
        }
        if manifest.payload.asm_text.is_empty() {
            self.asm_text.clear();
        } else {
            merge_asm_text_entries(
                &mut self.asm_text,
                manifest
                    .payload
                    .asm_text
                    .iter()
                    .map(|(key, value)| (key.clone(), value.clone()))
                    .collect(),
            )?;
        }
        if manifest.payload.move_names.is_empty() {
            self.move_names.clear();
        } else {
            insert_token_string_vec_table(
                &mut self.move_names,
                manifest.payload.move_names.clone(),
                "move names",
                "move name",
            )?;
        }
        if manifest.payload.battle_animations.is_empty() {
            self.battle_animations.clear();
        } else {
            merge_token_keyed_string_vec_map(
                &mut self.battle_animations,
                manifest
                    .payload
                    .battle_animations
                    .iter()
                    .map(|(key, value)| (key.clone(), value.clone()))
                    .collect(),
                "battle animation",
                "battle animation command",
            )?;
        }
        if manifest.payload.battle_animation_table.is_empty() {
            self.battle_animation_table.clear();
        } else {
            insert_token_string_vec_table(
                &mut self.battle_animation_table,
                manifest.payload.battle_animation_table.clone(),
                "battle animation",
                "battle animation table entry",
            )?;
        }
        if manifest.payload.battle_anim_bundle.is_empty() {
            self.battle_anim_bundle.clear();
        } else {
            insert_exact_string_bundle(
                &mut self.battle_anim_bundle,
                manifest.payload.battle_anim_bundle.clone(),
                "battle animation bundle",
                &[
                    "objects",
                    "framesets",
                    "oam_sets",
                    "gfx_table",
                    "gfx_sources",
                ],
            )?;
        }
        if manifest.payload.sprite_anim_bundle.is_empty() {
            self.sprite_anim_bundle.clear();
        } else {
            insert_exact_string_bundle(
                &mut self.sprite_anim_bundle,
                manifest.payload.sprite_anim_bundle.clone(),
                "sprite animation bundle",
                &["oam_sets", "framesets", "objects"],
            )?;
        }
        if manifest.payload.sprite_palette_defaults.is_empty() {
            self.sprite_palette_defaults.clear();
        } else {
            merge_sprite_palette_default_entries(
                &mut self.sprite_palette_defaults,
                manifest
                    .payload
                    .sprite_palette_defaults
                    .iter()
                    .map(|(key, value)| (key.clone(), *value))
                    .collect(),
            )?;
        }
        if manifest.payload.pokegear_town_map_palette_map.is_empty() {
            self.pokegear_town_map_palette_map.clear();
        } else {
            merge_token_keyed_token_vec_map(
                &mut self.pokegear_town_map_palette_map,
                manifest
                    .payload
                    .pokegear_town_map_palette_map
                    .iter()
                    .map(|(key, value)| (key.clone(), value.clone()))
                    .collect(),
                "Pokegear town map palette entry",
                "Pokegear town map palette value",
            )?;
        }
        if manifest.payload.pokegear_landmarks.landmarks.is_empty()
            && manifest
                .payload
                .pokegear_landmarks
                .map_to_landmark
                .is_empty()
        {
            self.pokegear_landmarks.landmarks.clear();
            self.pokegear_landmarks.map_to_landmark.clear();
        } else {
            merge_pokegear_landmarks(
                &mut self.pokegear_landmarks,
                &manifest.payload.pokegear_landmarks,
            )?;
        }
        if manifest.payload.pokemon_cries.is_empty() {
            self.pokemon_cries.clear();
        } else {
            merge_pokemon_cry_entries(
                &mut self.pokemon_cries,
                manifest
                    .payload
                    .pokemon_cries
                    .iter()
                    .map(|(key, value)| (key.clone(), value.clone()))
                    .collect(),
            )?;
        }
        if manifest.payload.trainers.trainers.is_empty() {
            self.trainers.trainers.clear();
        } else {
            for (trainer_id, trainer) in &manifest.payload.trainers.trainers {
                insert_keyed_trainer(&mut self.trainers, trainer_id.clone(), trainer.clone())?;
            }
        }
        if manifest.payload.phone_contacts.0.is_empty() {
            self.phone_contacts.0.clear();
        } else {
            merge_phone_contact_catalog(
                &mut self.phone_contacts,
                &manifest.payload.phone_contacts,
            )?;
        }
        if manifest.payload.permanent_phone_numbers.is_empty() {
            self.permanent_phone_numbers.clear();
        } else {
            merge_token_keyed_map(
                &mut self.permanent_phone_numbers,
                manifest.payload.permanent_phone_numbers.clone(),
                "permanent phone number",
            )?;
        }
        if manifest.payload.special_phone_calls.is_empty() {
            self.special_phone_calls.clear();
        } else {
            merge_token_keyed_map(
                &mut self.special_phone_calls,
                manifest.payload.special_phone_calls.clone(),
                "special phone call",
            )?;
        }
        if manifest.payload.npc_trades.is_empty() {
            self.npc_trades.clear();
        } else {
            merge_token_keyed_map(
                &mut self.npc_trades,
                manifest.payload.npc_trades.clone(),
                "NPC trade",
            )?;
        }
        if manifest.payload.special_routines.is_empty() {
            self.special_routines.clear();
        } else {
            merge_special_routine_rules(
                &mut self.special_routines,
                manifest.payload.special_routines.clone(),
            )?;
        }
        if manifest.payload.audio.is_empty() {
            self.audio.clear();
        } else {
            for (audio_id, audio_asset) in &manifest.payload.audio {
                insert_keyed_audio_asset(&mut self.audio, audio_id.clone(), audio_asset.clone())?;
            }
        }
        if manifest.payload.tilesets.is_empty() {
            self.tilesets.clear();
        } else {
            for (tileset_id, tileset) in &manifest.payload.tilesets {
                insert_keyed_tileset_definition(
                    &mut self.tilesets,
                    tileset_id.clone(),
                    tileset.clone(),
                )?;
            }
        }
        if manifest.payload.playability == PlayabilityRules::default() {
            self.playability = PlayabilityRules::default();
        } else {
            merge_playability_rules(&mut self.playability, &manifest.payload.playability)?;
        }
        Ok(())
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
        self.validate_runtime_map_tile(
            "wild battle encounter roll",
            &encounter.map_name,
            encounter.tile,
        )?;
        let species_id = &resolved.encounter.species;
        let species = self
            .pokemon
            .get(species_id)
            .with_context(|| format!("unknown wild species '{species_id}' in encounter table"))?;
        let battle_music =
            self.wild_battle_music_for_map_time(&encounter.map_name, encounter.time)?;
        Ok(wild_battle_start_from_encounter(
            encounter,
            battle_music,
            species,
            &self.learnsets,
            &self.moves,
            &self.growth_rates,
            rng,
        )?)
    }

    pub fn start_wild_battle(
        &self,
        state: &mut GameState,
        encounter: WildEncounterRoll,
    ) -> Result<WildBattleStart> {
        let roaming_species = state
            .script_runtime
            .variables
            .get("_roaming_encounter_species")
            .cloned();
        let mut rng = Random::new_crystal(state.rng_seed);
        let mut battle = self
            .wild_battle_start(encounter, &mut rng)
            .context("start wild battle from resolved encounter")?;
        if let Some(species_id) = roaming_species.as_deref() {
            let Some(roamer) = state
                .roaming_pokemon
                .iter_mut()
                .find(|roamer| roamer.species == species_id)
            else {
                anyhow::bail!("roaming encounter species {species_id} is missing from state");
            };
            let level = battle
                .encounter
                .resolved
                .as_ref()
                .map(|resolved| resolved.level)
                .context("roaming wild battle start is missing its resolved encounter")?;
            let dvs = if roamer.hp > 0 {
                Dv::from_non_hp(
                    ((roamer.dvs >> 12) & 0x0f) as u8,
                    ((roamer.dvs >> 8) & 0x0f) as u8,
                    ((roamer.dvs >> 4) & 0x0f) as u8,
                    (roamer.dvs & 0x0f) as u8,
                )
            } else {
                let generated = battle.enemy_pokemon.dvs;
                roamer.dvs = (u16::from(generated.attack & 0x0f) << 12)
                    | (u16::from(generated.defense & 0x0f) << 8)
                    | (u16::from(generated.speed & 0x0f) << 4)
                    | u16::from(generated.special & 0x0f);
                Dv::from_non_hp(
                    generated.attack,
                    generated.defense,
                    generated.speed,
                    generated.special,
                )
            };
            let mut enemy = self.create_pokemon(species_id, level, dvs)?;
            enemy.original_trainer_name = "WILD".to_string();
            enemy.original_trainer_id = 0;
            if roamer.hp > 0 {
                enemy.hp = u16::from(roamer.hp).min(enemy.max_hp);
            } else {
                roamer.hp = enemy.hp.min(u16::from(u8::MAX));
            }
            battle.enemy_pokemon = enemy.clone();
            battle.enemy_party = vec![enemy];
            battle.battle_type = "BATTLETYPE_ROAMING".to_string();
        }
        if roaming_species.is_some() {
            state
                .script_runtime
                .variables
                .remove("_roaming_encounter_species");
        }
        if state.bug_contest.timer_active {
            battle.battle_type = "BATTLETYPE_BUG_CONTEST".to_string();
        }
        state.commit_rng_seed(rng.seed());
        activate_wild_battle_start(state, &battle);
        state.battle_active_party_index = first_available_battle_party_index(state);
        state.battle_active_enemy_party_index = Some(0);
        state.battle_rewarded_enemy_party_indices.clear();
        state.battle_escape_attempts = 0;
        state.battle_player_stat_drop_guard_turns = 0;
        state.battle_pay_day_money = 0;
        Ok(battle)
    }

    pub fn start_field_encounter_battle(
        &self,
        state: &mut GameState,
        field_encounter: &crystal_core::world::encounters::FieldEncounterRoll,
    ) -> Result<Option<WildBattleStart>> {
        let Some(resolved) = field_encounter.resolved.clone() else {
            return Ok(None);
        };
        let target_tile =
            TilePosition::new(field_encounter.target_tile_x, field_encounter.target_tile_y);
        self.validate_runtime_map_tile(
            "field encounter target",
            &field_encounter.map_name,
            target_tile,
        )?;
        let surface = match field_encounter.kind {
            FieldEncounterKind::Headbutt => EncounterSurface::Grass,
            FieldEncounterKind::RockSmash => EncounterSurface::Rock,
        };
        let encounter = WildEncounterRoll {
            map_name: field_encounter.map_name.clone(),
            tile: target_tile,
            surface,
            time: state.time.time_of_day,
            threshold: 255,
            encounter_roll: field_encounter.chance_roll,
            slot_percent_roll: field_encounter.entry_roll,
            level_roll: None,
            resolved: Some(resolved),
            repelled_by: None,
            rng_seed_after: state.rng_seed,
        };
        self.start_wild_battle(state, encounter).map(Some)
    }

    pub fn start_fishing_battle(
        &self,
        state: &mut GameState,
        map_name: &str,
        tile: TilePosition,
        encounter: crystal_core::world::encounters::WildEncounter,
        time: TimeOfDay,
        bite_roll: u8,
        slot_roll: u8,
    ) -> Result<WildBattleStart> {
        self.validate_runtime_map_tile("fishing battle", map_name, tile)?;
        let roll = WildEncounterRoll {
            map_name: map_name.to_string(),
            tile,
            surface: EncounterSurface::Water,
            time,
            threshold: 0,
            encounter_roll: bite_roll,
            slot_percent_roll: Some(slot_roll),
            level_roll: None,
            resolved: Some(ResolvedWildEncounter {
                level: encounter.level,
                encounter,
                slot: 0,
            }),
            repelled_by: None,
            rng_seed_after: state.rng_seed,
        };
        self.start_wild_battle(state, roll)
    }

    fn validate_runtime_map_tile(
        &self,
        context: &str,
        map_name: &str,
        tile: TilePosition,
    ) -> Result<()> {
        let module = self.maps.get(map_name).with_context(|| {
            format!("{context} map {map_name} is missing from compiled pack maps")
        })?;
        let map =
            OverworldMapData::from_attributes(map_name, &module.attributes, module.blocks.clone());
        let (width, height) = map.checked_tile_bounds().with_context(|| {
            format!("{context} map {map_name} runtime tile bounds overflow supported coordinates")
        })?;
        if tile.x < 0
            || tile.y < 0
            || i32::from(tile.x) >= i32::from(width)
            || i32::from(tile.y) >= i32::from(height)
        {
            anyhow::bail!(
                "{context} tile ({}, {}) is outside compiled map {map_name} runtime tile bounds {width}x{height}",
                tile.x,
                tile.y
            );
        }
        Ok(())
    }

    pub fn static_wild_battle_start(
        &self,
        request: StaticWildBattleRequest,
        rng: &mut Random,
    ) -> Result<StaticWildBattleStart> {
        if request.battle_music.is_empty() {
            anyhow::bail!("static wild battle request missing exact battle_music");
        }
        Ok(static_wild_battle_start(
            &self.pokemon,
            &self.learnsets,
            &self.moves,
            &self.growth_rates,
            request,
            rng,
        )?)
    }

    pub fn wild_battle_music_for_map_time(
        &self,
        map_name: &str,
        time: TimeOfDay,
    ) -> Result<String> {
        let landmark = self.pokegear_landmark_for_map(map_name)?;
        let music_id = match landmark.region.as_str() {
            "KANTO" => "MUSIC_KANTO_WILD_BATTLE",
            "JOHTO" => match time {
                TimeOfDay::Night => "MUSIC_JOHTO_WILD_BATTLE_NIGHT",
                TimeOfDay::Morning | TimeOfDay::Day => "MUSIC_JOHTO_WILD_BATTLE",
            },
            region => anyhow::bail!(
                "town map landmark for map {map_name} has unsupported region {region:?} for wild battle music"
            ),
        };
        if !self.audio.iter().any(|asset| asset.id == music_id) {
            anyhow::bail!("wild battle music {music_id} for map {map_name} is missing from pack");
        }
        Ok(music_id.to_string())
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
        let blocks_label =
            required_map_attribute_label(map_name, "blocks_label", &attributes.blocks_label)?;
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

    /// Builds a definitive map module from the compiled split payload tables.
    ///
    /// This is not a compatibility path: every referenced label and payload must
    /// already be present in the compiled pack and must parse as the exact
    /// runtime schema.
    pub fn assemble_map_module_from_compiled_payloads(&self, map_name: &str) -> Result<MapModule> {
        if let Some(module) = self.maps.get(map_name) {
            return Ok(module.clone());
        }
        let attributes = self
            .map_attributes
            .get(map_name)
            .with_context(|| format!("missing map attributes for {map_name}"))?
            .clone();
        let map_scripts_label = required_map_attribute_label(
            map_name,
            "map_scripts_label",
            &attributes.map_scripts_label,
        )?;
        let map_events_label = required_map_attribute_label(
            map_name,
            "map_events_label",
            &attributes.map_events_label,
        )?;
        let blocks_label =
            required_map_attribute_label(map_name, "blocks_label", &attributes.blocks_label)?;

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
        let scenes = parse_map_scene_table(map_name, map_scripts)?;
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
        let map_name_by_constant = self.map_name_by_constant_from_attributes()?;
        let script_map_commands =
            parse_script_map_commands(map_name, &scripts, &map_name_by_constant)?;
        let script_text_commands = parse_script_text_commands(map_name, &scripts)?;
        let script_text_bodies = parse_script_text_bodies(map_name, &scripts)?;
        let script_menu_definitions = parse_script_menu_definitions(map_name, &scripts)?;
        let script_vertical_menus =
            parse_script_vertical_menus(map_name, &scripts, &script_menu_definitions)?;
        let script_elevators = parse_script_elevators(map_name, &scripts, &map_name_by_constant)?;
        let script_variable_commands = parse_script_variable_commands(map_name, &scripts)?;
        let script_control_commands = parse_script_control_commands(map_name, &scripts)?;
        let objects: Vec<ObjectEvent> = serde_json::from_value(objects_payload.clone())
            .with_context(|| format!("parse NPC object payload for {map_name}"))?;
        let script_field_pickups = parse_script_field_pickups(map_name, &scripts, &objects)?;
        let script_shop_commands = parse_script_shop_commands(map_name, &scripts)?;
        let script_phone_commands = parse_script_phone_commands(map_name, &scripts)?;
        let script_runtime_commands = parse_script_runtime_commands(map_name, &scripts)?;
        let script_swarm_commands = parse_script_swarm_commands(map_name, &scripts)?;
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
            script_vertical_menus,
            script_elevators,
            script_variable_commands,
            script_control_commands,
            script_field_pickups,
            script_shop_commands,
            script_phone_commands,
            script_runtime_commands,
            script_swarm_commands,
            map_script_section_commands,
            map_event_section_commands,
            scenes,
            events,
            objects,
            blocks,
        })
    }

    pub fn resolve_warp_transition(&self, trigger: &WarpTrigger) -> Result<WarpTransition> {
        if !is_exact_map_reference_token(&trigger.warp.target_map) {
            anyhow::bail!(
                "warp {} on {} has invalid target_map field {:?}",
                trigger.warp.index,
                trigger.map_name,
                trigger.warp.target_map
            );
        }
        if trigger.warp.target_map != trigger.warp.target_map_constant {
            anyhow::bail!(
                "warp {} on {} target_map {:?} does not match target_map_constant {:?}",
                trigger.warp.index,
                trigger.map_name,
                trigger.warp.target_map,
                trigger.warp.target_map_constant
            );
        }
        let destination_map = self
            .map_name_for_constant(&trigger.warp.target_map_constant)
            .with_context(|| {
                format!(
                    "unknown target map constant '{}' for warp {} on {}",
                    trigger.warp.target_map_constant, trigger.warp.index, trigger.map_name
                )
            })?;
        let destination_attributes =
            self.map_attributes.get(&destination_map).with_context(|| {
                format!(
                    "warp target '{}' missing attributes (referenced by {})",
                    destination_map, trigger.map_name
                )
            })?;
        let destination_events_label = required_map_attribute_label(
            &destination_map,
            "map_events_label",
            &destination_attributes.map_events_label,
        )
        .with_context(|| format!("resolve warp target {destination_map} map_events_label"))?;
        let destination_events_payload = self
            .map_scripts
            .get(destination_events_label)
            .with_context(|| format!("missing map events label {destination_events_label}"))?;
        let destination_events = parse_map_events(&destination_map, destination_events_payload)
            .with_context(|| format!("parse warp target events for {destination_map}"))?;
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
        let destination_warp = destination_events
            .warps
            .get(destination_index)
            .cloned()
            .with_context(|| {
                format!(
                    "warp id {} referenced by {} exceeds available warps ({}) on {}",
                    trigger.warp.target_warp_id,
                    trigger.map_name,
                    destination_events.warps.len(),
                    destination_map
                )
            })?;

        let destination_tile =
            checked_runtime_map_event_tile(destination_warp.x, destination_warp.y).with_context(
                || {
                    format!(
                        "warp id {} on {} coordinate ({}, {}) overflows runtime tile coordinates",
                        trigger.warp.target_warp_id,
                        destination_map,
                        destination_warp.x,
                        destination_warp.y
                    )
                },
            )?;

        Ok(WarpTransition {
            trigger: trigger.clone(),
            destination: WarpDestination {
                map_name: destination_map,
                tile: destination_tile,
                warp: destination_warp,
            },
        })
    }

    pub fn resolve_warp_transition_with_state(
        &self,
        state: &mut GameState,
        trigger: &WarpTrigger,
    ) -> Result<WarpTransition> {
        const LINK_ROOM_CONSTANTS: [&str; 5] = [
            "TRADE_CENTER",
            "COLOSSEUM",
            "TIME_CAPSULE",
            "MOBILE_TRADE_ROOM",
            "MOBILE_BATTLE_ROOM",
        ];

        let source_constant = self.map_constant(&trigger.map_name)?;
        let previous_is_link_room = state
            .previous_warp_map_name
            .as_deref()
            .and_then(|map_name| self.map_constant(map_name).ok())
            .is_some_and(|constant| LINK_ROOM_CONSTANTS.contains(&constant));

        let transition = if trigger.warp.target_warp_id < 1 {
            let preserve_backup = (source_constant == "POKECENTER_2F" && previous_is_link_room)
                || source_constant.ends_with("_ELEVATOR");
            if !preserve_backup {
                state.backup_warp_map_name = state.previous_warp_map_name.clone();
                state.backup_warp_index = state.previous_warp_index;
            }
            let destination_map = state
                .backup_warp_map_name
                .clone()
                .with_context(|| {
                    format!(
                        "dynamic warp {} on {} has no saved backup map",
                        trigger.warp.index, trigger.map_name
                    )
                })?;
            let destination_warp_id = Self::required_dynamic_backup_warp_index(
                state,
                trigger.warp.index,
                &trigger.map_name,
            )?;
            let destination = self.resolve_warp_destination(
                &destination_map,
                destination_warp_id,
                trigger,
            )?;
            WarpTransition {
                trigger: trigger.clone(),
                destination,
            }
        } else {
            self.resolve_warp_transition(trigger)?
        };

        let destination_constant = self.map_constant(&transition.destination.map_name)?;
        let source_is_link_room = LINK_ROOM_CONSTANTS.contains(&source_constant);
        let destination_is_link_room = LINK_ROOM_CONSTANTS.contains(&destination_constant);
        let moving_between_link_room_and_center =
            (source_constant == "POKECENTER_2F" && destination_is_link_room)
                || (source_is_link_room && destination_constant == "POKECENTER_2F");
        let leaving_dynamic_elevator =
            trigger.warp.target_warp_id < 1 && source_constant.ends_with("_ELEVATOR");
        if !moving_between_link_room_and_center && !leaving_dynamic_elevator {
            state.backup_warp_map_name = Some(trigger.map_name.clone());
        }
        if destination_constant == "POKECENTER_2F" && !source_is_link_room {
            state.backup_warp_index = Some(trigger.warp.index);
        }
        state.previous_warp_map_name = Some(trigger.map_name.clone());
        state.previous_warp_index = Some(trigger.warp.index);
        Ok(transition)
    }

    fn required_dynamic_backup_warp_index(
        state: &GameState,
        warp_index: u16,
        map_name: &str,
    ) -> Result<u16> {
        state
            .backup_warp_index
            .filter(|warp_id| *warp_id > 0)
            .with_context(|| {
                format!(
                    "dynamic warp {warp_index} on {map_name} has no saved nonzero backup warp"
                )
            })
    }

    fn resolve_warp_destination(
        &self,
        destination_map: &str,
        destination_warp_id: u16,
        trigger: &WarpTrigger,
    ) -> Result<WarpDestination> {
        let destination_attributes = self.map_attributes.get(destination_map).with_context(|| {
            format!("warp target '{destination_map}' missing attributes")
        })?;
        let events_label = required_map_attribute_label(
            destination_map,
            "map_events_label",
            &destination_attributes.map_events_label,
        )?;
        let payload = self
            .map_scripts
            .get(events_label)
            .with_context(|| format!("missing map events label {events_label}"))?;
        let events = parse_map_events(destination_map, payload)
            .with_context(|| format!("parse warp target events for {destination_map}"))?;
        let destination_warp = events
            .warps
            .get(usize::from(destination_warp_id - 1))
            .cloned()
            .with_context(|| {
                format!(
                    "backup warp id {destination_warp_id} for {destination_map} exceeds available warps ({})",
                    events.warps.len()
                )
            })?;
        let tile = checked_runtime_map_event_tile(destination_warp.x, destination_warp.y)
            .with_context(|| {
                format!(
                    "warp {} on {} resolves to overflowing destination coordinates ({}, {})",
                    trigger.warp.index, trigger.map_name, destination_warp.x, destination_warp.y
                )
            })?;
        Ok(WarpDestination {
            map_name: destination_map.to_string(),
            tile,
            warp: destination_warp,
        })
    }

    pub fn map_name_for_constant_from_metadata(&self, map_constant: &str) -> Option<String> {
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

    fn map_name_by_constant_from_attributes(&self) -> Result<BTreeMap<String, String>> {
        let mut names = BTreeMap::new();
        for (map_name, attributes) in &self.map_attributes {
            let Some(map_constant) = attributes.map_constant.as_ref() else {
                continue;
            };
            if let Some(previous) = names.insert(map_constant.clone(), map_name.clone()) {
                anyhow::bail!("duplicate map constant {map_constant} on {previous} and {map_name}");
            }
        }
        Ok(names)
    }
}

fn strip_compiled_mail_text(value: &str) -> String {
    let trimmed = value.trim();
    let unquoted = trimmed
        .strip_prefix('"')
        .and_then(|text| text.strip_suffix('"'))
        .unwrap_or(trimmed);
    unquoted.trim_end_matches('@').to_string()
}

fn set_compiled_mail_check_result(state: &mut GameState, result: u8) {
    let result = result.to_string();
    state.script_runtime.script_value = Some(result.clone());
    state
        .script_runtime
        .variables
        .insert("wScriptVar".to_string(), result);
}

fn set_npc_trade_result(state: &mut GameState, result: u8) {
    let result = result.to_string();
    state.script_runtime.script_value = Some(result.clone());
    state
        .script_runtime
        .variables
        .insert("_npc_trade_result".to_string(), result);
}

fn compiled_mail_message(entries: &[serde_json::Value]) -> Result<String> {
    if entries.is_empty() {
        anyhow::bail!("compiled mail definition has no entries");
    }
    Ok(entries
        .iter()
        .skip(1)
        .filter_map(|entry| {
            entry
                .get("args")
                .and_then(serde_json::Value::as_array)
                .and_then(|args| args.first())
                .and_then(serde_json::Value::as_str)
        })
        .map(strip_compiled_mail_text)
        .collect::<Vec<_>>()
        .join("\n"))
}

fn required_map_attribute_label<'a>(
    map_name: &str,
    field_name: &str,
    value: &'a Option<String>,
) -> Result<&'a str> {
    let Some(label) = value.as_deref() else {
        anyhow::bail!("missing {field_name} for map {map_name}");
    };
    validate_map_reference_token(label, &format!("map attributes {field_name}"))?;
    Ok(label)
}

fn connection_destination_tile(
    source_tile: crystal_core::world::map::TilePosition,
    direction: &str,
    offset: i32,
    target_attributes: &MapAttributes,
) -> Result<crystal_core::world::map::TilePosition> {
    let (target_x, target_y, max_x, max_y) =
        connection_destination_tile_components(source_tile, direction, offset, target_attributes)?;
    let min_tile = 0;
    if target_x < min_tile || target_x > max_x || target_y < min_tile || target_y > max_y {
        anyhow::bail!(
            "connection destination tile ({target_x}, {target_y}) is outside target map tile bounds {min_tile}..={max_x}, {min_tile}..={max_y}"
        );
    }
    let target_x = i16::try_from(target_x)
        .with_context(|| format!("connection destination x {target_x} overflows runtime tile"))?;
    let target_y = i16::try_from(target_y)
        .with_context(|| format!("connection destination y {target_y} overflows runtime tile"))?;
    Ok(crystal_core::world::map::TilePosition::new(
        target_x, target_y,
    ))
}

fn connection_destination_tile_in_bounds(
    source_tile: crystal_core::world::map::TilePosition,
    direction: &str,
    offset: i32,
    target_attributes: &MapAttributes,
) -> Result<bool> {
    let (target_x, target_y, max_x, max_y) =
        connection_destination_tile_components(source_tile, direction, offset, target_attributes)?;
    Ok(target_x >= 0 && target_x <= max_x && target_y >= 0 && target_y <= max_y)
}

fn connection_destination_tile_components(
    source_tile: crystal_core::world::map::TilePosition,
    direction: &str,
    offset: i32,
    target_attributes: &MapAttributes,
) -> Result<(i32, i32, i32, i32)> {
    let metatile_width = i32::from(METATILE_WIDTH);
    let offset_tiles = offset
        .checked_mul(metatile_width)
        .with_context(|| format!("connection offset {offset} overflows runtime tile space"))?;
    let width = i32::from(target_attributes.width)
        .checked_mul(metatile_width)
        .with_context(|| {
            format!(
                "connection target map {} width overflows runtime tile space",
                target_attributes
                    .map_constant
                    .as_deref()
                    .unwrap_or("<unknown>")
            )
        })?;
    let height = i32::from(target_attributes.height)
        .checked_mul(metatile_width)
        .with_context(|| {
            format!(
                "connection target map {} height overflows runtime tile space",
                target_attributes
                    .map_constant
                    .as_deref()
                    .unwrap_or("<unknown>")
            )
        })?;
    let (target_x, target_y) = match direction {
        "north" => (
            i32::from(source_tile.x)
                .checked_sub(offset_tiles)
                .context("north connection destination x overflows runtime tile space")?,
            height - metatile_width.min(height),
        ),
        "south" => (
            i32::from(source_tile.x)
                .checked_sub(offset_tiles)
                .context("south connection destination x overflows runtime tile space")?,
            0,
        ),
        "west" => (
            width - metatile_width.min(width),
            i32::from(source_tile.y)
                .checked_sub(offset_tiles)
                .context("west connection destination y overflows runtime tile space")?,
        ),
        "east" => (
            0,
            i32::from(source_tile.y)
                .checked_sub(offset_tiles)
                .context("east connection destination y overflows runtime tile space")?,
        ),
        other => anyhow::bail!("unsupported connection direction '{other}'"),
    };
    let max_x = width - 1;
    let max_y = height - 1;
    Ok((target_x, target_y, max_x, max_y))
}

fn read_json_file<T>(path: &Path) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    let bytes = std::fs::read(path).with_context(|| format!("read {}", path.display()))?;
    serde_json::from_slice(&bytes).with_context(|| format!("parse {}", path.display()))
}

fn write_compiled_game_pack(path: impl AsRef<Path>, pack: &CompiledGamePack) -> Result<()> {
    let path = path.as_ref();
    validate_compiled_game_pack_path(path)?;
    if pack.format_version != COMPILED_GAME_PACK_FORMAT_VERSION {
        anyhow::bail!(
            "compiled game pack {} has unsupported format version {}",
            path.display(),
            pack.format_version
        );
    }
    validate_compiled_game_pack_identity(pack)
        .with_context(|| format!("validate compiled game pack identity {}", path.display()))?;
    let mut serialized_pack = pack.clone();
    compress_pack_audio(&mut serialized_pack)?;
    serialized_pack.identity = derive_compiled_game_pack_identity_from_manifest(
        serialized_pack.format_version,
        &serialized_pack.data,
        &serialized_pack.audio_manifest,
        &serialized_pack.report,
    )?;
    let mut encoded = Vec::new();
    ciborium::into_writer(&serialized_pack, &mut encoded)
        .with_context(|| format!("encode compiled game pack {}", path.display()))?;
    if encoded.len() > u32::MAX as usize {
        anyhow::bail!(
            "compiled game pack {} exceeds binary payload length field",
            path.display()
        );
    }
    let mut bytes = Vec::with_capacity(COMPILED_GAME_PACK_HEADER_LEN + encoded.len());
    bytes.extend_from_slice(COMPILED_GAME_PACK_MAGIC);
    bytes.extend_from_slice(&COMPILED_GAME_PACK_FORMAT_VERSION.to_be_bytes());
    bytes.extend_from_slice(&(encoded.len() as u32).to_be_bytes());
    bytes.extend_from_slice(&fnv1a32_bytes(&encoded).to_be_bytes());
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

#[cfg(any(test, feature = "test-fixtures"))]
pub fn write_compiled_game_pack_for_tests(
    path: impl AsRef<Path>,
    pack: &CompiledGamePack,
) -> Result<()> {
    write_compiled_game_pack(path, pack)
}
