fn open_visible_trainer_card(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let frame_phase = runtime_shell.shell.session().state().vblank_counter & 0x3f;
    runtime_shell.trainer_card_open = true;
    runtime_shell.trainer_card_page = VisibleTrainerCardPage::Info;
    runtime_shell.trainer_card_colon_visible = frame_phase >= 32;
    runtime_shell.trainer_card_colon_ticks = frame_phase % 32;
    runtime_shell.trainer_card_badge_frame = 0;
    runtime_shell.trainer_card_badge_ticks = 0;
    close_visible_party_detail_state(runtime_shell);
    runtime_shell.pokedex_menu_open = false;
    runtime_shell.pokedex_detail_open = false;
    runtime_shell.pokedex_scripted_entry = false;
    runtime_shell.pokegear_menu_open = false;
    runtime_shell.options_menu_open = false;
    runtime_shell.save_menu_open = false;
    runtime_shell.save_flow = None;
    runtime_shell.storage_cursor = None;
    runtime_shell.pc_item_cursor = None;
    close_visible_field_pack_without_log(runtime_shell);
    runtime_shell
        .last_audio_events
        .push("opened Trainer Card".to_string());
    set_shell_action_status(runtime_shell, "TRAINER CARD");
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn close_visible_trainer_card(runtime_shell: &mut BevyRuntimeShell) {
    runtime_shell.trainer_card_open = false;
    runtime_shell.trainer_card_page = VisibleTrainerCardPage::Info;
    runtime_shell.trainer_card_colon_visible = false;
    runtime_shell.trainer_card_colon_ticks = 0;
    runtime_shell.trainer_card_badge_frame = 0;
    runtime_shell.trainer_card_badge_ticks = 0;
    runtime_shell
        .last_audio_events
        .push("closed Trainer Card".to_string());
    trim_event_log(&mut runtime_shell.last_audio_events);
}

fn advance_visible_trainer_card(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    match runtime_shell.trainer_card_page {
        VisibleTrainerCardPage::Info => {
            runtime_shell.trainer_card_page = VisibleTrainerCardPage::JohtoBadges;
            runtime_shell.trainer_card_badge_frame = 0;
            runtime_shell.trainer_card_badge_ticks = runtime_shell.trainer_card_colon_ticks & 0x07;
            record_visible_runtime_action(runtime_shell, "trainer_card:johto_badges")?;
            mark_runtime_snapshot_dirty(runtime_shell);
            Ok(())
        }
        VisibleTrainerCardPage::JohtoBadges => {
            record_visible_runtime_action(runtime_shell, "trainer_card:close")?;
            close_visible_trainer_card(runtime_shell);
            continue_visible_script_after_prompt(runtime_shell)
        }
    }
}

fn return_visible_trainer_card_left(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    if runtime_shell.trainer_card_page == VisibleTrainerCardPage::JohtoBadges {
        runtime_shell.trainer_card_page = VisibleTrainerCardPage::Info;
        runtime_shell.trainer_card_badge_frame = 0;
        runtime_shell.trainer_card_badge_ticks = 0;
        record_visible_runtime_action(runtime_shell, "trainer_card:info")?;
        mark_runtime_snapshot_dirty(runtime_shell);
    }
    Ok(())
}

fn advance_visible_trainer_card_right(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    if runtime_shell.trainer_card_page == VisibleTrainerCardPage::Info {
        runtime_shell.trainer_card_page = VisibleTrainerCardPage::JohtoBadges;
        runtime_shell.trainer_card_badge_frame = 0;
        runtime_shell.trainer_card_badge_ticks = runtime_shell.trainer_card_colon_ticks & 0x07;
        record_visible_runtime_action(runtime_shell, "trainer_card:johto_badges")?;
        mark_runtime_snapshot_dirty(runtime_shell);
    }
    Ok(())
}

fn selected_visible_start_menu_option(
    runtime_shell: &mut BevyRuntimeShell,
) -> Result<StartMenuOption> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let options = visible_start_menu_options(runtime_shell, &snapshot);
    let index = strict_readonly_cursor_index(
        &runtime_shell.start_menu_cursor,
        START_MENU_SURFACE_ID,
        options.len(),
    )
    .with_context(|| {
        format!("start-menu surface {START_MENU_SURFACE_ID} is active without a valid cursor")
    })?;
    options
        .get(index)
        .copied()
        .context("start menu cursor selected no visible option")
}

fn select_visible_start_menu_option_exact(
    runtime_shell: &mut BevyRuntimeShell,
    option: StartMenuOption,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let options = visible_start_menu_options(runtime_shell, &snapshot);
    let index = options
        .iter()
        .position(|candidate| *candidate == option)
        .with_context(|| format!("Start menu option {option:?} is not visible"))?;
    runtime_shell.start_menu_cursor = Some(MenuCursor {
        surface_id: START_MENU_SURFACE_ID.to_string(),
        option_index: index,
    });
    Ok(())
}

fn visible_start_menu_options(
    _runtime_shell: &BevyRuntimeShell,
    snapshot: &RuntimeShellSnapshot,
) -> Vec<StartMenuOption> {
    let mut options = Vec::new();
    let contest_active = snapshot.bug_contest.timer_active
        || snapshot
            .progression
            .active_engine_flags
            .contains("ENGINE_BUG_CONTEST_TIMER");
    let link_mode = snapshot.link_session.link_mode != 0;
    if snapshot
        .progression
        .active_engine_flags
        .contains(ENGINE_POKEDEX_FLAG)
    {
        options.push(StartMenuOption::Pokedex);
    }
    if !snapshot.party.slots.is_empty() {
        options.push(StartMenuOption::Pokemon);
    }
    if !contest_active && !link_mode {
        options.push(StartMenuOption::Pack);
    }
    if snapshot
        .progression
        .active_engine_flags
        .contains(ENGINE_POKEGEAR_FLAG)
    {
        options.push(StartMenuOption::Pokegear);
    }
    options.push(StartMenuOption::TrainerCard);
    if contest_active {
        options.push(StartMenuOption::QuitContest);
    } else if !link_mode {
        options.push(StartMenuOption::Save);
    }
    options.extend([StartMenuOption::Options, StartMenuOption::Exit]);
    options
}

fn open_visible_field_pack(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    close_visible_party_detail_state(runtime_shell);
    runtime_shell.pokedex_menu_open = false;
    runtime_shell.pokedex_detail_open = false;
    runtime_shell.pokedex_scripted_entry = false;
    runtime_shell.pokegear_menu_open = false;
    runtime_shell.trainer_card_open = false;
    runtime_shell.trainer_card_page = VisibleTrainerCardPage::Info;
    runtime_shell.trainer_card_colon_visible = false;
    runtime_shell.trainer_card_colon_ticks = 0;
    runtime_shell.trainer_card_badge_frame = 0;
    runtime_shell.trainer_card_badge_ticks = 0;
    runtime_shell.options_menu_open = false;
    runtime_shell.save_menu_open = false;
    runtime_shell.save_flow = None;
    runtime_shell.field_pack_action_cursor = None;
    runtime_shell.field_pack_target_mode = None;
    runtime_shell.pack_toss = None;
    runtime_shell.storage_cursor = None;
    runtime_shell.pc_item_cursor = None;
    let pockets = carried_field_pack_pockets(&snapshot);
    let pocket = if pockets.contains(&runtime_shell.last_field_pack_pocket) {
        runtime_shell.last_field_pack_pocket.clone()
    } else {
        FieldPackPocket::Items
    };
    open_visible_field_pack_pocket(runtime_shell, pocket.clone())?;
    runtime_shell.last_audio_events.push(format!(
        "opened Pack pocket {}",
        field_pack_pocket_label(&pocket)
    ));
    set_shell_action_status(
        runtime_shell,
        format!("PACK {}", field_pack_pocket_label(&pocket)),
    );
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn open_visible_pc_item_deposit_pack(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let item_count = snapshot
        .bag
        .items
        .iter()
        .filter(|item| item.quantity > 0)
        .count();
    if item_count == 0 {
        runtime_shell.pc_item_cursor = None;
        runtime_shell.pc_notice = Some("No items here!".to_string());
        record_visible_runtime_action(runtime_shell, "pc:item:deposit:empty")?;
        runtime_shell
            .last_audio_events
            .push("bag item pocket has no carried item to deposit".to_string());
        set_shell_action_status(runtime_shell, "NO ITEMS TO DEPOSIT");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    close_visible_party_detail_state(runtime_shell);
    runtime_shell.pokedex_menu_open = false;
    runtime_shell.pokedex_detail_open = false;
    runtime_shell.pokedex_scripted_entry = false;
    runtime_shell.pokegear_menu_open = false;
    runtime_shell.trainer_card_open = false;
    runtime_shell.trainer_card_page = VisibleTrainerCardPage::Info;
    runtime_shell.trainer_card_colon_visible = false;
    runtime_shell.trainer_card_colon_ticks = 0;
    runtime_shell.trainer_card_badge_frame = 0;
    runtime_shell.trainer_card_badge_ticks = 0;
    runtime_shell.options_menu_open = false;
    runtime_shell.save_menu_open = false;
    runtime_shell.save_flow = None;
    runtime_shell.field_pack_action_cursor = None;
    runtime_shell.field_pack_target_mode = None;
    runtime_shell.key_item_cursor = None;
    runtime_shell.ball_cursor = None;
    runtime_shell.tmhm_cursor = None;
    runtime_shell.custom_item_cursor = None;
    runtime_shell.field_pack_pocket = Some(FieldPackPocket::Items);
    move_visible_cursor_slot(
        &mut runtime_shell.bag_cursor,
        "bag:items".to_string(),
        item_count,
        0,
        &mut runtime_shell.last_audio_events,
    )?;
    runtime_shell
        .last_audio_events
        .push("opened PC item deposit Pack".to_string());
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn visible_field_pack_is_open(runtime_shell: &BevyRuntimeShell) -> bool {
    runtime_shell.field_pack_pocket.is_some()
        || runtime_shell.bag_cursor.is_some()
        || runtime_shell.key_item_cursor.is_some()
        || runtime_shell.ball_cursor.is_some()
        || runtime_shell.tmhm_cursor.is_some()
        || runtime_shell.custom_item_cursor.is_some()
}

fn move_visible_active_field_pack_cursor(
    runtime_shell: &mut BevyRuntimeShell,
    delta: isize,
) -> Result<()> {
    match active_visible_field_pack_pocket(runtime_shell) {
        FieldPackPocket::Items => move_visible_bag_cursor(runtime_shell, delta),
        FieldPackPocket::Balls => move_visible_ball_cursor(runtime_shell, delta),
        FieldPackPocket::KeyItems => move_visible_key_item_cursor(runtime_shell, delta),
        FieldPackPocket::TmHm => move_visible_tmhm_cursor(runtime_shell, delta),
        FieldPackPocket::Custom(pocket_id) => {
            move_visible_custom_item_cursor(runtime_shell, pocket_id.as_str(), delta)
        }
    }
}

fn shift_visible_field_pack_pocket(
    runtime_shell: &mut BevyRuntimeShell,
    delta: isize,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let pockets = carried_field_pack_pockets(&snapshot);
    if pockets.is_empty() {
        close_visible_field_pack_without_log(runtime_shell);
        record_visible_runtime_action(runtime_shell, "pack:pocket:shift:empty")?;
        runtime_shell
            .last_audio_events
            .push("Pack is empty".to_string());
        set_shell_action_status(runtime_shell, "PACK IS EMPTY");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    let current = active_visible_field_pack_pocket(runtime_shell);
    let current_index = pockets
        .iter()
        .position(|pocket| *pocket == current)
        .with_context(|| format!("active Pack pocket {current:?} has no carried pocket entry"))?;
    let next_index = if delta.is_negative() {
        current_index
            .checked_sub(delta.unsigned_abs())
            .unwrap_or(pockets.len() - 1)
    } else {
        (current_index + delta as usize) % pockets.len()
    };
    open_visible_field_pack_pocket(runtime_shell, pockets[next_index].clone())?;
    Ok(())
}

fn open_visible_field_pack_pocket(
    runtime_shell: &mut BevyRuntimeShell,
    pocket: FieldPackPocket,
) -> Result<()> {
    runtime_shell.field_pack_action_cursor = None;
    runtime_shell.bag_cursor = None;
    runtime_shell.key_item_cursor = None;
    runtime_shell.ball_cursor = None;
    runtime_shell.tmhm_cursor = None;
    runtime_shell.custom_item_cursor = None;
    runtime_shell.field_pack_pocket = Some(pocket.clone());
    runtime_shell.last_field_pack_pocket = pocket.clone();
    let label = match &pocket {
        FieldPackPocket::Items => "items".to_string(),
        FieldPackPocket::Balls => "balls".to_string(),
        FieldPackPocket::KeyItems => "key-items".to_string(),
        FieldPackPocket::TmHm => "tm-hm".to_string(),
        FieldPackPocket::Custom(pocket_id) => format!("custom:{pocket_id}"),
    };
    match &pocket {
        FieldPackPocket::Items => {
            runtime_shell.bag_cursor = Some(MenuCursor {
                surface_id: "bag:items".to_string(),
                option_index: runtime_shell.field_pack_cursor_positions[0],
            })
        }
        FieldPackPocket::Balls => {
            runtime_shell.ball_cursor = Some(MenuCursor {
                surface_id: "bag:balls".to_string(),
                option_index: runtime_shell.field_pack_cursor_positions[1],
            })
        }
        FieldPackPocket::KeyItems => {
            runtime_shell.key_item_cursor = Some(MenuCursor {
                surface_id: "bag:key-items".to_string(),
                option_index: runtime_shell.field_pack_cursor_positions[2],
            })
        }
        FieldPackPocket::TmHm => {
            runtime_shell.tmhm_cursor = Some(MenuCursor {
                surface_id: "bag:tmhm".to_string(),
                option_index: runtime_shell.field_pack_cursor_positions[3],
            })
        }
        FieldPackPocket::Custom(_) => {}
    }
    match pocket {
        FieldPackPocket::Items => move_visible_bag_cursor(runtime_shell, 0),
        FieldPackPocket::Balls => move_visible_ball_cursor(runtime_shell, 0),
        FieldPackPocket::KeyItems => move_visible_key_item_cursor(runtime_shell, 0),
        FieldPackPocket::TmHm => move_visible_tmhm_cursor(runtime_shell, 0),
        FieldPackPocket::Custom(pocket_id) => {
            move_visible_custom_item_cursor(runtime_shell, pocket_id.as_str(), 0)
        }
    }?;
    runtime_shell
        .last_audio_events
        .push(format!("opened Pack pocket {label}"));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn active_visible_field_pack_pocket(runtime_shell: &BevyRuntimeShell) -> FieldPackPocket {
    if let Some(pocket) = runtime_shell.field_pack_pocket.clone() {
        return pocket;
    }
    if let Some(pocket_id) = active_custom_pack_cursor_pocket_id(runtime_shell) {
        return FieldPackPocket::Custom(pocket_id.to_string());
    }
    if runtime_shell.key_item_cursor.is_some() {
        return FieldPackPocket::KeyItems;
    }
    if runtime_shell.ball_cursor.is_some() {
        return FieldPackPocket::Balls;
    }
    if runtime_shell.tmhm_cursor.is_some() {
        return FieldPackPocket::TmHm;
    }
    FieldPackPocket::Items
}

fn active_custom_pack_cursor_pocket_id(runtime_shell: &BevyRuntimeShell) -> Option<&str> {
    runtime_shell
        .custom_item_cursor
        .as_ref()
        .and_then(|cursor| cursor.surface_id.strip_prefix("bag:custom:"))
        .filter(|pocket_id| !pocket_id.is_empty())
}

fn carried_field_pack_pockets(snapshot: &RuntimeShellSnapshot) -> Vec<FieldPackPocket> {
    let mut pockets = FIELD_PACK_POCKETS.to_vec();
    pockets.extend(
        snapshot
            .bag
            .custom_pockets
            .iter()
            .filter(|(_, items)| carried_item_count(items) > 0)
            .map(|(pocket_id, _)| FieldPackPocket::Custom(pocket_id.clone())),
    );
    pockets
}

fn field_pack_pocket_count(snapshot: &RuntimeShellSnapshot, pocket: &FieldPackPocket) -> usize {
    match pocket {
        FieldPackPocket::Items => carried_item_count(&snapshot.bag.items),
        FieldPackPocket::Balls => carried_item_count(&snapshot.bag.balls),
        FieldPackPocket::KeyItems => carried_item_count(&snapshot.bag.key_items),
        FieldPackPocket::TmHm => snapshot.bag.tm_hm.len(),
        FieldPackPocket::Custom(pocket_id) => snapshot
            .bag
            .custom_pockets
            .get(pocket_id)
            .map(|items| carried_item_count(items))
            .unwrap_or(0),
    }
}

fn field_pack_selectable_count(carried_count: usize) -> usize {
    carried_count + 1
}

fn custom_pack_surface_id(pocket_id: &str) -> String {
    format!("bag:custom:{pocket_id}")
}

fn open_visible_pokedex_menu(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    if snapshot.pokemon.is_empty() {
        anyhow::bail!("compiled pack has no Pokemon species");
    }
    if runtime_shell.pokedex_cursor >= snapshot.pokemon.len() {
        anyhow::bail!(
            "Pokedex cursor {} is out of range for {} species",
            runtime_shell.pokedex_cursor,
            snapshot.pokemon.len()
        );
    }
    runtime_shell.pokedex_menu_open = true;
    runtime_shell.pokedex_detail_open = false;
    runtime_shell.pokedex_detail_page = 0;
    runtime_shell.pokedex_scripted_entry = false;
    runtime_shell.pokegear_menu_open = false;
    close_visible_party_detail_state(runtime_shell);
    runtime_shell.options_menu_open = false;
    runtime_shell.save_menu_open = false;
    runtime_shell.save_flow = None;
    runtime_shell.storage_cursor = None;
    runtime_shell.pc_item_cursor = None;
    close_visible_field_pack_without_log(runtime_shell);
    runtime_shell.last_audio_events.push(format!(
        "opened Pokedex selected={}",
        snapshot.pokemon[runtime_shell.pokedex_cursor].species_id
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn close_visible_pokedex_menu(runtime_shell: &mut BevyRuntimeShell) {
    runtime_shell.pokedex_menu_open = false;
    runtime_shell.pokedex_detail_open = false;
    runtime_shell.pokedex_scripted_entry = false;
    runtime_shell.pokedex_detail_page = 0;
    runtime_shell
        .last_audio_events
        .push("closed Pokedex".to_string());
    trim_event_log(&mut runtime_shell.last_audio_events);
}

fn move_visible_pokedex_cursor(runtime_shell: &mut BevyRuntimeShell, delta: isize) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    if snapshot.pokemon.is_empty() {
        anyhow::bail!("compiled pack has no Pokemon species");
    }
    anyhow::ensure!(
        runtime_shell.pokedex_cursor < snapshot.pokemon.len(),
        "Pokedex cursor {} is out of range for {} species",
        runtime_shell.pokedex_cursor,
        snapshot.pokemon.len()
    );
    let current = runtime_shell.pokedex_cursor;
    let next = if runtime_shell.pokedex_detail_open {
        let seen = &snapshot.progression.pokedex_seen_species;
        let mut candidate_index = current;
        let mut found = None;
        let step = if delta < 0 { -1 } else { 1 };
        for _ in 0..snapshot.pokemon.len() {
            let candidate = candidate_index as isize + step;
            if candidate < 0 || candidate >= snapshot.pokemon.len() as isize {
                break;
            }
            candidate_index = candidate as usize;
            if seen.contains(&snapshot.pokemon[candidate_index].species_id) {
                found = Some(candidate_index);
                break;
            }
        }
        found.unwrap_or(current)
    } else {
        (current as isize + delta).clamp(0, snapshot.pokemon.len() as isize - 1) as usize
    };
    runtime_shell.pokedex_cursor = next;
    runtime_shell.pokedex_detail_page = 0;
    runtime_shell.last_audio_events.push(format!(
        "Pokedex cursor {}->{} {}",
        current + 1,
        next + 1,
        snapshot.pokemon[next].species_id
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn page_visible_pokedex_cursor(runtime_shell: &mut BevyRuntimeShell, delta: isize) -> Result<()> {
    if runtime_shell.pokedex_detail_open {
        return Ok(());
    }
    move_visible_pokedex_cursor(runtime_shell, delta * 7)
}

fn inspect_visible_pokedex_selection(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let species = selected_pokedex_catalog_species(&snapshot, runtime_shell.pokedex_cursor)?;
    if !snapshot
        .progression
        .pokedex_seen_species
        .contains(&species.species_id)
    {
        record_visible_runtime_action(
            runtime_shell,
            format!("pokedex:unseen:{}", species.species_id),
        )?;
        set_shell_action_status(runtime_shell, "POKEDEX ENTRY NOT SEEN");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    let entry = snapshot
        .presentation
        .pokedex_entries
        .get(&species.species_id)
        .with_context(|| format!("compiled pack missing Pokedex entry {}", species.species_id))?;
    runtime_shell.pokedex_detail_open = true;
    runtime_shell.pokedex_detail_page = 0;
    runtime_shell.last_audio_events.push(format!(
        "opened Pokedex detail #{} {} class={} h={} w={} pages={}",
        species.int_id,
        species.species_id,
        entry.classification,
        entry.height_digits,
        entry.weight_digits,
        entry.pages.join(" / ")
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn close_visible_pokedex_detail(runtime_shell: &mut BevyRuntimeShell) {
    runtime_shell.pokedex_detail_open = false;
    runtime_shell.pokedex_detail_page = 0;
    runtime_shell.pokedex_scripted_entry = false;
    runtime_shell
        .last_audio_events
        .push("closed Pokedex detail".to_string());
    trim_event_log(&mut runtime_shell.last_audio_events);
}

fn selected_pokedex_catalog_species(
    snapshot: &RuntimeShellSnapshot,
    cursor: usize,
) -> Result<&crate::RuntimePokemonCatalogSnapshot> {
    if snapshot.pokemon.is_empty() {
        anyhow::bail!("compiled pack has no Pokemon species");
    }
    snapshot.pokemon.get(cursor).with_context(|| {
        format!(
            "selected Pokedex cursor {cursor} is outside compiled species catalog length {}",
            snapshot.pokemon.len()
        )
    })
}

fn open_visible_pokegear_menu(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    if snapshot
        .presentation
        .pokegear_landmarks
        .landmarks
        .is_empty()
    {
        anyhow::bail!("compiled pack has no Pokegear landmarks");
    }
    let landmark_count = snapshot.presentation.pokegear_landmarks.landmarks.len();
    if runtime_shell.pokegear_cursor >= landmark_count {
        anyhow::bail!(
            "Pokegear cursor {} is out of range for {} landmarks",
            runtime_shell.pokegear_cursor,
            landmark_count
        );
    }
    runtime_shell.pokegear_menu_open = true;
    runtime_shell.pokegear_standalone_map = false;
    runtime_shell.pokegear_phone_status = None;
    runtime_shell.pokegear_page = PokegearPage::Clock;
    runtime_shell.pokegear_radio_station = None;
    runtime_shell.pokegear_radio_segment = 0;
    runtime_shell.pokedex_menu_open = false;
    runtime_shell.pokedex_detail_open = false;
    runtime_shell.pokedex_scripted_entry = false;
    close_visible_party_detail_state(runtime_shell);
    runtime_shell.options_menu_open = false;
    runtime_shell.save_menu_open = false;
    runtime_shell.save_flow = None;
    runtime_shell.storage_cursor = None;
    runtime_shell.pc_item_cursor = None;
    close_visible_field_pack_without_log(runtime_shell);
    let landmark =
        &snapshot.presentation.pokegear_landmarks.landmarks[runtime_shell.pokegear_cursor];
    runtime_shell
        .last_audio_events
        .push(format!("opened Pokegear selected={}", landmark.constant));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn close_visible_pokegear_menu(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    exit_visible_pokegear_radio(runtime_shell)?;
    runtime_shell.pokegear_phone_call = None;
    runtime_shell.pokegear_menu_open = false;
    runtime_shell.pokegear_standalone_map = false;
    runtime_shell.pokegear_phone_status = None;
    runtime_shell.pokegear_page = PokegearPage::Clock;
    runtime_shell.pokegear_radio_station = None;
    runtime_shell.pokegear_radio_segment = 0;
    runtime_shell
        .last_audio_events
        .push("closed Pokegear".to_string());
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn move_visible_pokegear_cursor(runtime_shell: &mut BevyRuntimeShell, delta: isize) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    if runtime_shell.pokegear_page == PokegearPage::Radio {
        let before = snapshot.progression.radio_tuning_knob;
        let after = if delta.is_negative() {
            before.saturating_add(2).min(80)
        } else {
            before.saturating_sub(2)
        };
        if after == before {
            return Ok(());
        }
        let mutation = runtime_shell.shell.apply_runtime_mutation_command(
            crate::RuntimeMutationCommand::SetPokegearRadioTuning(
                crate::assets::RuntimePokegearRadioTuningCommand { tuning_knob: after },
            ),
        )?;
        let crate::RuntimeMutationResult::PokegearRadioTuningSet(outcome) = mutation.result else {
            anyhow::bail!("radio tuning mutation returned a different result");
        };
        runtime_shell.pokegear_radio_tuning_knob = outcome.tuning_knob_after;
        set_shell_action_status(
            runtime_shell,
            format!("RADIO {:.1}", visible_pokegear_radio_frequency(after)),
        );
        let snapshot = runtime_shell.shell.snapshot()?;
        sync_visible_pokegear_radio(runtime_shell, &snapshot)?;
        return Ok(());
    }
    if runtime_shell.pokegear_page == PokegearPage::Clock {
        return Ok(());
    }
    if runtime_shell.pokegear_page == PokegearPage::Phone {
        let contact_ids = visible_pokegear_phone_contact_ids(&snapshot);
        if contact_ids.is_empty() {
            return handle_visible_no_phone_contacts(runtime_shell, "move");
        }
        anyhow::ensure!(
            runtime_shell.pokegear_phone_cursor < contact_ids.len(),
            "Pokegear phone cursor {} is out of range for {} contacts",
            runtime_shell.pokegear_phone_cursor,
            contact_ids.len()
        );
        let current = runtime_shell.pokegear_phone_cursor;
        let next = wrapped_index(current, contact_ids.len(), delta);
        runtime_shell.pokegear_phone_cursor = next;
        runtime_shell.pokegear_phone_status = None;
        runtime_shell.last_audio_events.push(format!(
            "Pokegear phone cursor {}->{} {}",
            current + 1,
            next + 1,
            contact_ids[next]
        ));
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    let landmarks = &snapshot.presentation.pokegear_landmarks.landmarks;
    let region_indices = visible_pokegear_landmark_indices(&snapshot)?;
    if region_indices.is_empty() {
        anyhow::bail!("compiled pack has no Pokegear landmarks for the active region");
    }
    let current_position = region_indices
        .iter()
        .position(|index| *index == runtime_shell.pokegear_cursor)
        .with_context(|| {
            format!(
                "Pokegear landmark cursor {} is not in the active region",
                runtime_shell.pokegear_cursor
            )
        })?;
    // _TownMap increments the landmark on Up and decrements it on Down,
    // opposite the portable Pokégear list convention used by this handler.
    let delta = if runtime_shell.pokegear_standalone_map {
        -delta
    } else {
        delta
    };
    let next_position = wrapped_index(current_position, region_indices.len(), delta);
    let next = region_indices[next_position];
    runtime_shell.pokegear_cursor = next;
    runtime_shell.last_audio_events.push(format!(
        "Pokegear cursor {}->{} {}",
        current_position + 1,
        next_position + 1,
        landmarks[next].constant
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn visible_pokegear_region(snapshot: &RuntimeShellSnapshot) -> Result<&str> {
    let constant = snapshot
        .presentation
        .pokegear_landmarks
        .map_to_landmark
        .get(&snapshot.overworld.map_name)
        .with_context(|| {
            format!(
                "active map {} has no compiled Pokegear landmark mapping",
                snapshot.overworld.map_name
            )
        })?;
    let landmark = snapshot
        .presentation
        .pokegear_landmarks
        .landmarks
        .iter()
        .find(|landmark| landmark.constant == *constant)
        .with_context(|| {
            format!(
                "active map {} references missing Pokegear landmark {constant}",
                snapshot.overworld.map_name
            )
        })?;
    anyhow::ensure!(
        matches!(landmark.region.as_str(), "JOHTO" | "KANTO"),
        "Pokegear landmark {constant} has unsupported region {}",
        landmark.region
    );
    Ok(landmark.region.as_str())
}

fn visible_pokegear_landmark_indices(snapshot: &RuntimeShellSnapshot) -> Result<Vec<usize>> {
    let region = visible_pokegear_region(snapshot)?;
    let indices = snapshot
        .presentation
        .pokegear_landmarks
        .landmarks
        .iter()
        .enumerate()
        .filter_map(|(index, landmark)| (landmark.region == region).then_some(index))
        .collect::<Vec<_>>();
    anyhow::ensure!(
        !indices.is_empty(),
        "compiled pack has no Pokegear landmarks for active region {region}"
    );
    Ok(indices)
}

fn inspect_visible_pokegear_selection(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    if matches!(
        runtime_shell.pokegear_page,
        PokegearPage::Clock | PokegearPage::Radio
    ) {
        return Ok(());
    }
    if runtime_shell.pokegear_page == PokegearPage::Phone {
        return start_visible_pokegear_phone_call(runtime_shell);
    }
    Ok(())
}

fn start_visible_pokegear_phone_call(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let (contact_id, no_service) = {
        let snapshot = runtime_shell.shell.snapshot()?;
        if visible_pokegear_phone_contact_ids(&snapshot).is_empty() {
            return handle_visible_no_phone_contacts(runtime_shell, "call");
        }
        let contact_id = selected_visible_pokegear_phone_contact_id(&snapshot, runtime_shell)?;
        let no_service = snapshot
            .maps
            .iter()
            .find(|map| map.map_name == snapshot.overworld.map_name)
            .and_then(|map| map.metadata.as_ref())
            .with_context(|| {
                format!(
                    "active map {} has no compiled metadata",
                    snapshot.overworld.map_name
                )
            })?
            .phone_service
            >> 4
            != 0;
        (contact_id, no_service)
    };
    if no_service {
        record_visible_runtime_action(
            runtime_shell,
            format!("pokegear_phone_call:{contact_id}:no_service"),
        )?;
        queue_visible_shell_sound_effect(runtime_shell, "SFX_NO_SIGNAL")?;
        runtime_shell.pokegear_phone_status = Some("OUT OF SERVICE".to_string());
        runtime_shell.pokegear_phone_call = Some(VisiblePokegearPhoneCall {
            contact_id: contact_id.clone(),
            phase: VisiblePokegearPhoneCallPhase::NoServicePrompt,
        });
        runtime_shell.last_audio_events.push(format!(
            "Pokegear phone call contact={contact_id} rejected by map phone service"
        ));
        set_shell_action_status(runtime_shell, "OUT OF SERVICE");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }

    record_visible_runtime_action(
        runtime_shell,
        format!("pokegear_phone_call:{contact_id}:ring:1"),
    )?;
    queue_visible_shell_sound_effect(runtime_shell, "SFX_CALL")?;
    runtime_shell.pokegear_menu_open = false;
    runtime_shell.pokegear_phone_status = None;
    runtime_shell.pokegear_radio_station = None;
    runtime_shell.pokegear_radio_segment = 0;
    runtime_shell.pokegear_phone_call = Some(VisiblePokegearPhoneCall {
        contact_id: contact_id.clone(),
        phase: VisiblePokegearPhoneCallPhase::Ringing { rings_started: 1 },
    });
    set_shell_action_status(runtime_shell, format!("CALL {}", contact_id));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn advance_visible_pokegear_phone_call(
    runtime_shell: &mut BevyRuntimeShell,
    elapsed_input_ticks: u32,
) -> Result<bool> {
    let Some(call) = runtime_shell.pokegear_phone_call.clone() else {
        return Ok(false);
    };
    match call.phase {
        VisiblePokegearPhoneCallPhase::NoServicePrompt => Ok(false),
        VisiblePokegearPhoneCallPhase::Ringing { rings_started } => {
            let transient_pending = runtime_shell.transient_audio_playing
                || runtime_shell
                    .pending_audio
                    .iter()
                    .any(|command| !matches!(command.kind, ModpackAudioKind::Music));
            if transient_pending {
                return Ok(true);
            }
            if rings_started == 1 {
                record_visible_runtime_action(
                    runtime_shell,
                    format!("pokegear_phone_call:{}:ring:2", call.contact_id),
                )?;
                queue_visible_shell_sound_effect(runtime_shell, "SFX_CALL")?;
                runtime_shell
                    .pokegear_phone_call
                    .as_mut()
                    .expect("outgoing call remains active")
                    .phase = VisiblePokegearPhoneCallPhase::Ringing { rings_started: 2 };
                return Ok(true);
            }
            anyhow::ensure!(rings_started == 2, "invalid outgoing phone ring count");
            let outcome = runtime_shell
                .shell
                .start_pokegear_phone_call(call.contact_id.clone())?;
            if !has_visible_compiled_script_command(runtime_shell, &outcome.callback_script, 0) {
                anyhow::bail!(
                    "phone contact {} resolved missing callback script {}",
                    call.contact_id,
                    outcome.callback_script
                );
            }
            record_visible_runtime_action(
                runtime_shell,
                format!(
                    "pokegear_phone_call:{}:{}",
                    call.contact_id, outcome.callback_script
                ),
            )?;
            runtime_shell
                .pokegear_phone_call
                .as_mut()
                .expect("outgoing call remains active")
                .phase = VisiblePokegearPhoneCallPhase::Calling;
            runtime_shell.last_audio_events.push(format!(
                "Pokegear phone call contact={} callback={} callee={}",
                call.contact_id,
                outcome.callback_script,
                outcome.callee_script.as_deref().unwrap_or("out_of_area")
            ));
            trim_event_log(&mut runtime_shell.last_audio_events);
            start_visible_script_entry(runtime_shell, &outcome.callback_script)?;
            Ok(true)
        }
        VisiblePokegearPhoneCallPhase::Calling => {
            if runtime_shell.active_script_cursor.is_none()
                && !runtime_shell.shell.has_pending_script_work()
            {
                runtime_shell
                    .pokegear_phone_call
                    .as_mut()
                    .expect("outgoing call remains active")
                    .phase = VisiblePokegearPhoneCallPhase::FinishDelay {
                    frames_remaining: 10,
                };
                return Ok(true);
            }
            Ok(false)
        }
        VisiblePokegearPhoneCallPhase::FinishDelay { frames_remaining } => {
            let elapsed = u8::try_from(elapsed_input_ticks).unwrap_or(u8::MAX);
            let next = frames_remaining.saturating_sub(elapsed);
            runtime_shell
                .pokegear_phone_call
                .as_mut()
                .expect("outgoing call remains active")
                .phase = if next == 0 {
                VisiblePokegearPhoneCallPhase::AwaitHangup
            } else {
                VisiblePokegearPhoneCallPhase::FinishDelay {
                    frames_remaining: next,
                }
            };
            Ok(true)
        }
        VisiblePokegearPhoneCallPhase::AwaitHangup => Ok(false),
    }
}

fn dismiss_visible_pokegear_no_service_prompt(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    anyhow::ensure!(
        runtime_shell
            .pokegear_phone_call
            .as_ref()
            .is_some_and(|call| call.phase == VisiblePokegearPhoneCallPhase::NoServicePrompt),
        "Pokegear no-service prompt is not active"
    );
    record_visible_runtime_action(runtime_shell, "pokegear_phone_call:no_service:dismiss")?;
    runtime_shell.pokegear_phone_call = None;
    runtime_shell.pokegear_menu_open = true;
    runtime_shell.pokegear_page = PokegearPage::Phone;
    runtime_shell.pokegear_phone_status = None;
    set_shell_action_status(runtime_shell, "PHONE");
    Ok(())
}

fn finish_visible_pokegear_phone_call(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    anyhow::ensure!(
        runtime_shell
            .pokegear_phone_call
            .as_ref()
            .is_some_and(|call| call.phase == VisiblePokegearPhoneCallPhase::AwaitHangup),
        "outgoing Pokegear phone call is not waiting for hangup"
    );
    record_visible_runtime_action(runtime_shell, "pokegear_phone_call:hangup")?;
    queue_visible_shell_sound_effect(runtime_shell, "SFX_HANG_UP")?;
    runtime_shell.pokegear_phone_call = None;
    runtime_shell.pokegear_menu_open = true;
    runtime_shell.pokegear_page = PokegearPage::Phone;
    runtime_shell.pokegear_phone_status = None;
    set_shell_action_status(runtime_shell, "PHONE");
    Ok(())
}

fn begin_visible_incoming_phone_sequence(
    runtime_shell: &mut BevyRuntimeShell,
    effect: crate::core::systems::script_runtime::ScriptPhoneCallasmPresentation,
) -> Result<()> {
    anyhow::ensure!(
        runtime_shell.incoming_phone_sequence.is_none(),
        "cannot overlap incoming phone presentation sequences"
    );
    match effect {
        crate::core::systems::script_runtime::ScriptPhoneCallasmPresentation::RingTwice => {
            queue_visible_shell_sound_effect(runtime_shell, "SFX_CALL")?;
            runtime_shell.incoming_phone_sequence = Some(VisibleIncomingPhoneSequence::RingTwice {
                frames_remaining: 120,
                second_ring_started: false,
            });
        }
        crate::core::systems::script_runtime::ScriptPhoneCallasmPresentation::HangUp => {
            queue_visible_shell_sound_effect(runtime_shell, "SFX_HANG_UP")?;
            runtime_shell.incoming_phone_sequence = Some(VisibleIncomingPhoneSequence::HangUp {
                frames_remaining: 140,
            });
        }
    }
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(())
}

fn advance_visible_incoming_phone_sequence(
    runtime_shell: &mut BevyRuntimeShell,
    elapsed_input_ticks: u32,
) -> Result<bool> {
    let Some(sequence) = runtime_shell.incoming_phone_sequence else {
        return Ok(false);
    };
    let elapsed = u16::try_from(elapsed_input_ticks).unwrap_or(u16::MAX);
    match sequence {
        VisibleIncomingPhoneSequence::RingTwice {
            frames_remaining,
            second_ring_started,
        } => {
            let next = frames_remaining.saturating_sub(elapsed);
            let start_second = !second_ring_started && next <= 60;
            if start_second {
                queue_visible_shell_sound_effect(runtime_shell, "SFX_CALL")?;
            }
            if next == 0 {
                runtime_shell.incoming_phone_sequence = None;
                continue_visible_script_after_prompt(runtime_shell)?;
            } else {
                runtime_shell.incoming_phone_sequence =
                    Some(VisibleIncomingPhoneSequence::RingTwice {
                        frames_remaining: next,
                        second_ring_started: second_ring_started || start_second,
                    });
            }
        }
        VisibleIncomingPhoneSequence::HangUp { frames_remaining } => {
            let next = frames_remaining.saturating_sub(elapsed);
            if next == 0 {
                runtime_shell.incoming_phone_sequence = None;
                continue_visible_script_after_prompt(runtime_shell)?;
            } else {
                runtime_shell.incoming_phone_sequence =
                    Some(VisibleIncomingPhoneSequence::HangUp {
                        frames_remaining: next,
                    });
            }
        }
    }
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(true)
}

fn handle_visible_no_phone_contacts(
    runtime_shell: &mut BevyRuntimeShell,
    action: &str,
) -> Result<()> {
    record_visible_runtime_action(
        runtime_shell,
        format!("pokegear_phone:{action}:no_contacts"),
    )?;
    runtime_shell
        .last_audio_events
        .push("Pokegear has no registered phone contacts".to_string());
    set_shell_action_status(runtime_shell, "NO PHONE CONTACTS");
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn sync_visible_pokegear_radio(
    runtime_shell: &mut BevyRuntimeShell,
    snapshot: &RuntimeShellSnapshot,
) -> Result<()> {
    let tuning_knob = snapshot.progression.radio_tuning_knob;
    runtime_shell.pokegear_radio_tuning_knob = tuning_knob;
    let station = VISIBLE_POKEGEAR_RADIO_STATIONS
        .iter()
        .find_map(|(position, handler)| {
            (*position == tuning_knob)
                .then(|| visible_pokegear_radio_station(snapshot, handler))
                .flatten()
        });
    runtime_shell.pokegear_radio_station = station
        .as_ref()
        .map(|(constant, _)| (*constant).to_string());
    if station.is_none() {
        runtime_shell.active_pokegear_radio = None;
        set_visible_stopped_music_state(runtime_shell, Some("MUSIC_NONE"));
        runtime_shell
            .last_audio_events
            .push("Pokegear radio no signal; stopped music".to_string());
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    let music_id = station
        .as_ref()
        .expect("station was checked above")
        .1
        .clone();
    let retained_radio = station
        .as_ref()
        .map(|_| (snapshot.overworld.map_name.clone(), music_id.clone()));
    if runtime_shell.active_music.as_deref() == Some(music_id.as_str())
        || pending_music_command_is(&runtime_shell.pending_audio, &music_id)
    {
        runtime_shell.active_pokegear_radio = retained_radio;
        return Ok(());
    }
    let playback = runtime_shell
        .shell
        .runtime()
        .audio()
        .require_playback_entry(AudioKind::Music, &music_id)?;
    enqueue_bevy_audio_command(
        &mut runtime_shell.pending_audio,
        BevyAudioCommand {
            audio_id: music_id.clone(),
            kind: ModpackAudioKind::Music,
            mode: playback.mode,
            looped: matches!(
                playback.loop_policy,
                crate::assets::ModpackAudioLoopPolicy::Loop
            ),
        },
    );
    runtime_shell.pending_music_stop = true;
    runtime_shell.active_music = Some(music_id.clone());
    runtime_shell.faded_music = None;
    runtime_shell.active_pokegear_radio = retained_radio;
    let (constant, _) = station.expect("station was checked above");
    runtime_shell
        .last_audio_events
        .push(format!("Pokegear radio tuned {constant} {music_id}"));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn exit_visible_pokegear_radio(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    if runtime_shell.pokegear_page == PokegearPage::Radio
        && runtime_shell.active_music.as_deref() == Some("MUSIC_NONE")
    {
        queue_visible_current_music(runtime_shell)?;
    }
    Ok(())
}

fn visible_pokegear_radio_frequency(tuning_knob: u8) -> f32 {
    f32::from(tuning_knob + 2) / 4.0
}

fn visible_pokegear_radio_station(
    snapshot: &RuntimeShellSnapshot,
    handler: &str,
) -> Option<(&'static str, String)> {
    let landmark = snapshot
        .presentation
        .pokegear_landmarks
        .map_to_landmark
        .get(&snapshot.overworld.map_name)
        .and_then(|constant| {
            snapshot
                .presentation
                .pokegear_landmarks
                .landmarks
                .iter()
                .find(|landmark| landmark.constant == *constant)
        });
    let landmark_constant = landmark.map(|landmark| landmark.constant.as_str());
    let in_johto = landmark.is_none_or(|landmark| landmark.region != "KANTO");
    if in_johto
        && snapshot
            .progression
            .active_engine_flags
            .contains("ENGINE_ROCKETS_IN_RADIO_TOWER")
    {
        return Some(("ROCKET_RADIO", "MUSIC_ROCKET_OVERTURE".to_string()));
    }
    let flags = &snapshot.progression.active_engine_flags;
    match handler {
        "PKMNTalkAndPokedexShow" if in_johto => {
            if landmark_constant == Some("LANDMARK_FAST_SHIP")
                || matches!(
                    snapshot.progression.time.time_of_day,
                    crate::core::world::encounters::TimeOfDay::Morning
                )
            {
                Some(("POKEDEX_SHOW", "MUSIC_POKEMON_CENTER".to_string()))
            } else {
                Some(("OAKS_POKEMON_TALK", "MUSIC_POKEMON_TALK".to_string()))
            }
        }
        "PokemonMusic" if in_johto => Some((
            "POKEMON_MUSIC",
            if snapshot.progression.time.day_of_week % 2 == 0 {
                "MUSIC_POKEMON_MARCH"
            } else {
                "MUSIC_POKEMON_LULLABY"
            }
            .to_string(),
        )),
        "LuckyChannel" if in_johto => Some(("LUCKY_CHANNEL", "MUSIC_GAME_CORNER".to_string())),
        "BuenasPassword" if in_johto => {
            Some(("BUENAS_PASSWORD", "MUSIC_BUENAS_PASSWORD".to_string()))
        }
        "RuinsOfAlphRadio" if landmark_constant == Some("LANDMARK_RUINS_OF_ALPH") => {
            Some(("UNOWN_RADIO", "MUSIC_RUINS_OF_ALPH_RADIO".to_string()))
        }
        "PlacesAndPeople" if !in_johto && flags.contains("ENGINE_EXPN_CARD") => {
            Some(("PLACES_AND_PEOPLE", "MUSIC_VIRIDIAN_CITY".to_string()))
        }
        "LetsAllSing" if !in_johto && flags.contains("ENGINE_EXPN_CARD") => {
            Some(("LETS_ALL_SING", "MUSIC_BICYCLE".to_string()))
        }
        "PokeFluteRadio" if !in_johto && flags.contains("ENGINE_EXPN_CARD") => {
            Some(("POKE_FLUTE_RADIO", "MUSIC_POKE_FLUTE_CHANNEL".to_string()))
        }
        "EvolutionRadio"
            if flags.contains("ENGINE_ROCKET_SIGNAL_ON_CH20")
                && matches!(
                    landmark_constant,
                    Some("LANDMARK_MAHOGANY_TOWN" | "LANDMARK_ROUTE_43" | "LANDMARK_LAKE_OF_RAGE")
                ) =>
        {
            Some((
                "EVOLUTION_RADIO",
                "MUSIC_LAKE_OF_RAGE_ROCKET_RADIO".to_string(),
            ))
        }
        _ => None,
    }
}

fn toggle_visible_pokegear_page(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    cycle_visible_pokegear_page(runtime_shell, 1)
}

fn cycle_visible_pokegear_page(runtime_shell: &mut BevyRuntimeShell, delta: isize) -> Result<()> {
    if !runtime_shell.pokegear_menu_open {
        return handle_visible_no_active_pokegear(runtime_shell, "page_toggle");
    }
    let snapshot = runtime_shell.shell.snapshot()?;
    let mut pages = vec![PokegearPage::Clock];
    if snapshot
        .progression
        .active_engine_flags
        .contains("ENGINE_MAP_CARD")
    {
        pages.push(PokegearPage::Map);
    }
    if snapshot
        .progression
        .active_engine_flags
        .contains("ENGINE_PHONE_CARD")
    {
        pages.push(PokegearPage::Phone);
    }
    if snapshot
        .progression
        .active_engine_flags
        .contains("ENGINE_RADIO_CARD")
    {
        pages.push(PokegearPage::Radio);
    }
    let current = pages
        .iter()
        .position(|page| *page == runtime_shell.pokegear_page)
        .with_context(|| {
            format!(
                "active Pokegear page {:?} is not unlocked",
                runtime_shell.pokegear_page
            )
        })?;
    exit_visible_pokegear_radio(runtime_shell)?;
    runtime_shell.pokegear_phone_status = None;
    runtime_shell.pokegear_page = pages[wrapped_index(current, pages.len(), delta)];
    if runtime_shell.pokegear_page == PokegearPage::Map {
        let region_indices = visible_pokegear_landmark_indices(&snapshot)?;
        let current_landmark = snapshot
            .presentation
            .pokegear_landmarks
            .map_to_landmark
            .get(&snapshot.overworld.map_name)
            .with_context(|| {
                format!(
                    "active map {} has no compiled Pokegear landmark mapping",
                    snapshot.overworld.map_name
                )
            })?;
        runtime_shell.pokegear_cursor = region_indices
            .iter()
            .copied()
            .find(|index| {
                snapshot.presentation.pokegear_landmarks.landmarks[*index].constant
                    == *current_landmark
            })
            .with_context(|| {
                format!("current Pokegear landmark {current_landmark} is outside the active region")
            })?;
    }
    runtime_shell.pokegear_radio_station = None;
    runtime_shell.pokegear_radio_segment = 0;
    if runtime_shell.pokegear_page == PokegearPage::Radio {
        sync_visible_pokegear_radio(runtime_shell, &snapshot)?;
    }
    runtime_shell
        .last_audio_events
        .push(format!("Pokegear page {:?}", runtime_shell.pokegear_page));
    set_shell_action_status(
        runtime_shell,
        format!("POKEGEAR {:?}", runtime_shell.pokegear_page),
    );
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn selected_pokegear_landmark(
    snapshot: &RuntimeShellSnapshot,
    cursor: usize,
) -> Result<&crate::core::models::PokegearLandmark> {
    let landmarks = &snapshot.presentation.pokegear_landmarks.landmarks;
    if landmarks.is_empty() {
        anyhow::bail!("compiled pack has no Pokegear landmarks");
    }
    landmarks.get(cursor).with_context(|| {
        format!(
            "selected Pokegear cursor {cursor} is outside compiled landmark catalog length {}",
            landmarks.len()
        )
    })
}

fn wrapped_index(current: usize, len: usize, delta: isize) -> usize {
    if delta.is_negative() {
        current.checked_sub(delta.unsigned_abs()).unwrap_or(len - 1)
    } else {
        (current + delta as usize) % len
    }
}

fn close_visible_field_pack_without_log(runtime_shell: &mut BevyRuntimeShell) {
    if runtime_shell.field_pack_target_mode == Some(FieldPackTargetMode::PartyMove) {
        runtime_shell.party_move_cursor = None;
    }
    runtime_shell.field_pack_action_cursor = None;
    runtime_shell.bag_cursor = None;
    runtime_shell.key_item_cursor = None;
    runtime_shell.ball_cursor = None;
    runtime_shell.tmhm_cursor = None;
    runtime_shell.custom_item_cursor = None;
    runtime_shell.field_pack_pocket = None;
    runtime_shell.pack_item_switch_origin = None;
    runtime_shell.field_pack_target_mode = None;
    runtime_shell.tmhm_teach_prompt_cursor = None;
    runtime_shell.pending_tmhm_text_stage = None;
    runtime_shell.tmhm_decision_prompt_cursor = None;
    runtime_shell.tmhm_decision = None;
    runtime_shell.tmhm_forget_menu_open = false;
    runtime_shell.party_held_item_give_target = None;
    runtime_shell.held_item_swap_prompt = false;
    runtime_shell.pack_toss = None;
    runtime_shell.field_notice = None;
    runtime_shell.field_notice_queue.clear();
    runtime_shell.field_notice_scene = None;
    runtime_shell.pending_field_notice_sound = None;
    runtime_shell.pending_field_notice_cry = None;
    runtime_shell.pending_field_battle_entry = false;
    runtime_shell.pending_field_notice_effect_frames = None;
    runtime_shell.visible_cut_animation = None;
    runtime_shell.pending_whirlpool_sound_wait = false;
    runtime_shell.visible_headbutt_animation = None;
    runtime_shell.visible_flash_animation = None;
    runtime_shell.visible_fly_animation = None;
    runtime_shell.visible_waterfall_animation = None;
    runtime_shell.pending_surf_start_from = None;
    runtime_shell.hall_of_fame_pc_index = None;
    runtime_shell.visible_heal_machine = None;
    runtime_shell.visible_magnet_train = None;
    runtime_shell.visible_unown_words = None;
    runtime_shell.visible_diploma = None;
    runtime_shell.visible_battle_transition = None;
    runtime_shell.heal_music_active = false;
}

fn open_visible_tmhm_teach_prompt(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let (item_id, move_id) = selected_tmhm(runtime_shell)?;
    runtime_shell.field_pack_action_cursor = None;
    runtime_shell.field_pack_target_mode = None;
    runtime_shell.tmhm_teach_prompt_cursor = None;
    runtime_shell.pending_tmhm_text_stage = Some(VisibleTmHmTextStage::Boot);
    let text_target = if item_id.starts_with("HM") {
        "_BootedHMText"
    } else {
        "_BootedTMText"
    };
    install_visible_tmhm_text_pages(
        runtime_shell,
        visible_move_learning_text_pages(
            runtime_shell,
            text_target,
            "",
            "",
            move_id.as_deref().unwrap_or(&item_id),
        )?,
    )?;
    mark_runtime_snapshot_dirty(runtime_shell);
    record_visible_runtime_action(runtime_shell, format!("pack:tmhm:boot:{item_id}"))?;
    set_shell_action_status(runtime_shell, "BOOTED UP TM/HM");
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn install_visible_tmhm_text_pages(
    runtime_shell: &mut BevyRuntimeShell,
    pages: Vec<String>,
) -> Result<()> {
    let mut pages = VecDeque::from(pages);
    runtime_shell.field_notice = pages.pop_front();
    anyhow::ensure!(
        runtime_shell.field_notice.is_some(),
        "TM/HM source text rendered no pages"
    );
    runtime_shell.field_notice_queue = pages;
    runtime_shell.field_text_reveal = None;
    Ok(())
}

fn advance_visible_tmhm_text_stage(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let stage = runtime_shell
        .pending_tmhm_text_stage
        .context("no TM/HM source-text stage is pending")?;
    let (item_id, move_id) = selected_tmhm(runtime_shell)?;
    let move_id = move_id.as_deref().unwrap_or(&item_id);
    match stage {
        VisibleTmHmTextStage::Boot => {
            runtime_shell.pending_tmhm_text_stage = Some(VisibleTmHmTextStage::Contained);
            install_visible_tmhm_text_pages(
                runtime_shell,
                visible_move_learning_text_pages(
                    runtime_shell,
                    "_ContainedMoveText",
                    "",
                    "",
                    move_id,
                )?,
            )?;
            set_shell_action_status(runtime_shell, "TM/HM CONTAINED MOVE");
        }
        VisibleTmHmTextStage::Contained => {
            runtime_shell.pending_tmhm_text_stage = None;
            let final_page = visible_move_learning_text_pages(
                runtime_shell,
                "_ContainedMoveText",
                "",
                "",
                move_id,
            )?
            .pop()
            .context("TM/HM contained text has no final yes/no page")?;
            runtime_shell.field_notice = Some(final_page);
            runtime_shell.field_notice_queue.clear();
            runtime_shell.field_text_reveal = None;
            visible_cursor_index(
                &mut runtime_shell.tmhm_teach_prompt_cursor,
                "pack:tmhm:teach-prompt",
                2,
            );
            record_visible_runtime_action(runtime_shell, format!("pack:tmhm:prompt:{item_id}"))?;
            set_shell_action_status(runtime_shell, format!("TEACH {move_id}?"));
        }
        VisibleTmHmTextStage::Decision(decision) => {
            runtime_shell.pending_tmhm_text_stage = None;
            let snapshot = runtime_shell.shell.snapshot()?;
            let party_index = selected_party_index(runtime_shell)?;
            let nickname = snapshot
                .party
                .slots
                .iter()
                .find(|slot| slot.index == party_index)
                .map(|slot| slot.pokemon.nickname.as_str())
                .context("TM/HM decision requires the selected party Pokemon")?;
            let text_target = match decision {
                VisibleTmHmDecision::ForgetMove => "_AskForgetMoveText",
                VisibleTmHmDecision::StopLearning => "_StopLearningMoveText",
            };
            let final_page = visible_move_learning_text_pages(
                runtime_shell,
                text_target,
                nickname,
                nickname,
                move_id,
            )?
            .pop()
            .context("TM/HM decision text has no final yes/no page")?;
            runtime_shell.field_notice = Some(final_page);
            runtime_shell.field_notice_queue.clear();
            runtime_shell.field_text_reveal = None;
            visible_cursor_index(
                &mut runtime_shell.tmhm_decision_prompt_cursor,
                "pack:tmhm:decision",
                2,
            );
            set_shell_action_status(runtime_shell, "TM/HM MOVE DECISION");
        }
        VisibleTmHmTextStage::RestoreMovePrompt => {
            runtime_shell.pending_tmhm_text_stage = None;
            install_visible_tmhm_move_list_prompt(runtime_shell)?;
        }
    }
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn install_visible_tmhm_move_list_prompt(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let (item_id, move_id) = selected_tmhm(runtime_shell)?;
    let snapshot = runtime_shell.shell.snapshot()?;
    let party_index = selected_party_index(runtime_shell)?;
    let nickname = snapshot
        .party
        .slots
        .iter()
        .find(|slot| slot.index == party_index)
        .map(|slot| slot.pokemon.nickname.as_str())
        .context("TM/HM move list requires the selected party Pokemon")?;
    install_visible_tmhm_text_pages(
        runtime_shell,
        visible_move_learning_text_pages(
            runtime_shell,
            "_MoveAskForgetText",
            nickname,
            nickname,
            move_id.as_deref().unwrap_or(&item_id),
        )?,
    )
}

fn resolve_visible_tmhm_teach_prompt(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let selected = strict_readonly_cursor_index(
        &runtime_shell.tmhm_teach_prompt_cursor,
        "pack:tmhm:teach-prompt",
        2,
    )
    .context("TM/HM teach prompt requires a valid cursor")?;
    runtime_shell.tmhm_teach_prompt_cursor = None;
    runtime_shell.field_notice = None;
    runtime_shell.field_notice_queue.clear();
    runtime_shell.field_text_reveal = None;
    if selected == 0 {
        record_visible_runtime_action(runtime_shell, "pack:tmhm:teach:yes")?;
        return open_visible_field_pack_target(runtime_shell, FieldPackTargetMode::TmHmPokemon);
    }
    record_visible_runtime_action(runtime_shell, "pack:tmhm:teach:no")?;
    mark_runtime_snapshot_dirty(runtime_shell);
    set_shell_action_status(runtime_shell, "TM/HM TEACH DECLINED");
    Ok(())
}

fn open_visible_tmhm_decision_prompt(
    runtime_shell: &mut BevyRuntimeShell,
    decision: VisibleTmHmDecision,
) -> Result<()> {
    runtime_shell.tmhm_decision = Some(decision);
    runtime_shell.tmhm_decision_prompt_cursor = None;
    runtime_shell.pending_tmhm_text_stage = Some(VisibleTmHmTextStage::Decision(decision));
    let (item_id, move_id) = selected_tmhm(runtime_shell)?;
    let snapshot = runtime_shell.shell.snapshot()?;
    let party_index = selected_party_index(runtime_shell)?;
    let nickname = snapshot
        .party
        .slots
        .iter()
        .find(|slot| slot.index == party_index)
        .map(|slot| slot.pokemon.nickname.as_str())
        .context("TM/HM decision requires the selected party Pokemon")?;
    let text_target = match decision {
        VisibleTmHmDecision::ForgetMove => "_AskForgetMoveText",
        VisibleTmHmDecision::StopLearning => "_StopLearningMoveText",
    };
    let pages = visible_move_learning_text_pages(
        runtime_shell,
        text_target,
        nickname,
        nickname,
        move_id.as_deref().unwrap_or(&item_id),
    )?;
    install_visible_tmhm_text_pages(runtime_shell, pages)?;
    set_shell_action_status(
        runtime_shell,
        match decision {
            VisibleTmHmDecision::ForgetMove => "DELETE A MOVE?",
            VisibleTmHmDecision::StopLearning => "STOP LEARNING?",
        },
    );
    Ok(())
}

fn resolve_visible_tmhm_decision_prompt(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let selected = strict_readonly_cursor_index(
        &runtime_shell.tmhm_decision_prompt_cursor,
        "pack:tmhm:decision",
        2,
    )
    .context("TM/HM decision prompt requires a valid cursor")?;
    let decision = runtime_shell
        .tmhm_decision
        .context("TM/HM decision prompt is missing its decision kind")?;
    runtime_shell.tmhm_decision_prompt_cursor = None;
    runtime_shell.tmhm_decision = None;
    match (decision, selected) {
        (VisibleTmHmDecision::ForgetMove, 0) => {
            let snapshot = runtime_shell.shell.snapshot()?;
            initialize_visible_tmhm_replacement_cursor(runtime_shell, &snapshot)?;
            runtime_shell.tmhm_forget_menu_open = true;
            install_visible_tmhm_move_list_prompt(runtime_shell)?;
            set_shell_action_status(runtime_shell, "CHOOSE A MOVE TO FORGET");
        }
        (VisibleTmHmDecision::ForgetMove, _) => {
            open_visible_tmhm_decision_prompt(runtime_shell, VisibleTmHmDecision::StopLearning)?;
        }
        (VisibleTmHmDecision::StopLearning, 0) => {
            let (item_id, move_id) = selected_tmhm(runtime_shell)?;
            let snapshot = runtime_shell.shell.snapshot()?;
            let party_index = selected_party_index(runtime_shell)?;
            let nickname = snapshot
                .party
                .slots
                .iter()
                .find(|slot| slot.index == party_index)
                .map(|slot| slot.pokemon.nickname.as_str())
                .context("TM/HM result requires the selected party Pokemon")?;
            let pages = visible_move_learning_text_pages(
                runtime_shell,
                "_DidNotLearnMoveText",
                nickname,
                nickname,
                move_id.as_deref().unwrap_or(&item_id),
            )?;
            runtime_shell.party_move_cursor = None;
            runtime_shell.field_pack_target_mode = None;
            install_visible_tmhm_text_pages(runtime_shell, pages)?;
            mark_runtime_snapshot_dirty(runtime_shell);
            set_shell_action_status(runtime_shell, "DID NOT LEARN THE MOVE");
        }
        (VisibleTmHmDecision::StopLearning, _) => {
            let snapshot = runtime_shell.shell.snapshot()?;
            initialize_visible_tmhm_replacement_cursor(runtime_shell, &snapshot)?;
            runtime_shell.tmhm_forget_menu_open = true;
            install_visible_tmhm_move_list_prompt(runtime_shell)?;
            set_shell_action_status(runtime_shell, "CHOOSE A MOVE TO FORGET");
        }
    }
    Ok(())
}

fn confirm_visible_tmhm_target(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let party_index = selected_party_index(runtime_shell)?;
    let (item_id, _) = selected_tmhm(runtime_shell)?;
    let slot = snapshot
        .party
        .slots
        .iter()
        .find(|slot| slot.index == party_index)
        .with_context(|| format!("selected party index {party_index} is not in the party"))?;
    if slot.pokemon.is_egg || slot.pokemon.species.id == "EGG" {
        queue_visible_shell_sound_effect(runtime_shell, "SFX_WRONG")?;
        set_shell_action_status(runtime_shell, "EGG CANNOT LEARN TM/HM");
        return Ok(());
    }
    if !runtime_shell.tmhm_forget_menu_open {
        match runtime_shell
            .shell
            .preview_tmhm_on_party_pokemon(&item_id, party_index, None)
        {
            Ok(_) => return teach_selected_tmhm_on(runtime_shell, party_index),
            Err(error)
                if matches!(
                    error.downcast_ref::<TmHmLearnError>(),
                    Some(TmHmLearnError::MoveListFull)
                ) =>
            {
                return open_visible_tmhm_decision_prompt(
                    runtime_shell,
                    VisibleTmHmDecision::ForgetMove,
                );
            }
            Err(error) if tmhm_error_is_play_refusal(&error) => {
                let (text_target, move_id, wrong_sfx) = match error.downcast_ref::<TmHmLearnError>()
                {
                    Some(TmHmLearnError::CannotLearn { move_id, .. }) => {
                        ("_TMHMNotCompatibleText", move_id.as_str(), true)
                    }
                    Some(TmHmLearnError::AlreadyKnows { move_id }) => {
                        ("_KnowsMoveText", move_id.as_str(), false)
                    }
                    Some(TmHmLearnError::MoveListFull) => {
                        unreachable!("MoveListFull is handled before TM/HM refusal text")
                    }
                    _ => anyhow::bail!("unexpected playable TM/HM refusal: {error}"),
                };
                if wrong_sfx {
                    queue_visible_shell_sound_effect(runtime_shell, "SFX_WRONG")?;
                }
                let pages = visible_move_learning_text_pages(
                    runtime_shell,
                    text_target,
                    &slot.pokemon.nickname,
                    &slot.pokemon.nickname,
                    move_id,
                )?;
                install_visible_tmhm_text_pages(runtime_shell, pages)?;
                runtime_shell
                    .last_audio_events
                    .push(format!("TM/HM {item_id} refused: {error}"));
                mark_runtime_snapshot_dirty(runtime_shell);
                set_shell_action_status(runtime_shell, "TM/HM REFUSED");
                trim_event_log(&mut runtime_shell.last_audio_events);
                return Ok(());
            }
            Err(error) => return Err(error),
        }
    }
    if runtime_shell.tmhm_forget_menu_open {
        let selected = strict_readonly_cursor_index(
            &runtime_shell.party_move_cursor,
            &party_move_cursor_surface_id(party_index),
            slot.pokemon.moves.len() + 1,
        )
        .context("TM/HM forget menu requires a valid cursor")?;
        if selected == slot.pokemon.moves.len() {
            runtime_shell.tmhm_forget_menu_open = false;
            runtime_shell.party_move_cursor = None;
            return open_visible_tmhm_decision_prompt(
                runtime_shell,
                VisibleTmHmDecision::StopLearning,
            );
        }
        let forgotten_move = slot
            .pokemon
            .moves
            .get(selected)
            .map(|learned| learned.name.as_str())
            .context("TM/HM replacement cursor is outside the move list")?;
        let is_hm = snapshot
            .items
            .iter()
            .any(|item| !item.consumable && item.tmhm_move.as_deref() == Some(forgotten_move));
        if is_hm {
            runtime_shell.pending_tmhm_text_stage = Some(VisibleTmHmTextStage::RestoreMovePrompt);
            let pages = visible_move_learning_text_pages(
                runtime_shell,
                "_MoveCantForgetHMText",
                &slot.pokemon.nickname,
                &slot.pokemon.nickname,
                forgotten_move,
            )?;
            install_visible_tmhm_text_pages(runtime_shell, pages)?;
            set_shell_action_status(runtime_shell, "HM MOVES CAN'T BE FORGOTTEN NOW");
            return Ok(());
        }
    }
    teach_selected_tmhm_on(runtime_shell, party_index)
}

fn open_visible_field_pack_target(
    runtime_shell: &mut BevyRuntimeShell,
    mode: FieldPackTargetMode,
) -> Result<()> {
    runtime_shell.field_pack_action_cursor = None;
    let snapshot = runtime_shell.shell.snapshot()?;
    if snapshot.party.slots.is_empty() {
        runtime_shell.field_pack_target_mode = None;
        runtime_shell.party_move_cursor = None;
        record_visible_runtime_action(
            runtime_shell,
            format!(
                "pack:target:{}:empty_party",
                field_pack_target_mode_label(mode)
            ),
        )?;
        runtime_shell
            .last_audio_events
            .push("party is empty".to_string());
        set_shell_action_status(runtime_shell, "NO POKEMON");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    initialize_visible_party_cursor(runtime_shell, &snapshot);
    if mode == FieldPackTargetMode::PartyMove {
        let party_index = selected_party_index(runtime_shell)?;
        let selected = snapshot
            .party
            .slots
            .iter()
            .find(|slot| slot.index == party_index)
            .with_context(|| format!("selected party index {party_index} is not in the party"))?;
        if selected.pokemon.moves.is_empty() {
            runtime_shell.field_pack_target_mode = None;
            runtime_shell.party_move_cursor = None;
            record_visible_runtime_action(runtime_shell, "pack:target:party_move:no_moves")?;
            runtime_shell
                .last_audio_events
                .push(format!("party index {party_index} has no moves"));
            set_shell_action_status(runtime_shell, "NO MOVES");
            trim_event_log(&mut runtime_shell.last_audio_events);
            return Ok(());
        }
        visible_cursor_index(
            &mut runtime_shell.party_move_cursor,
            &party_move_cursor_surface_id(party_index),
            selected.pokemon.moves.len() + 1,
        );
    }
    let selected_species = selected_party_species_label(&snapshot, runtime_shell.party_cursor)
        .with_context(|| {
            format!(
                "field pack target cursor {} is not backed by a party slot",
                runtime_shell.party_cursor
            )
        })?;
    runtime_shell.field_pack_target_mode = Some(mode);
    runtime_shell.last_audio_events.push(format!(
        "pack target mode {} selected={}",
        field_pack_target_mode_label(mode),
        selected_species
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn close_visible_field_pack_target(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let Some(mode) = runtime_shell.field_pack_target_mode else {
        return handle_visible_no_field_pack_target(runtime_shell, "close");
    };
    record_visible_runtime_action(
        runtime_shell,
        format!("pack:target:{}:close", field_pack_target_mode_label(mode)),
    )?;
    if mode == FieldPackTargetMode::PartyMove {
        runtime_shell.party_move_cursor = None;
    }
    runtime_shell.tmhm_decision_prompt_cursor = None;
    runtime_shell.tmhm_decision = None;
    runtime_shell.tmhm_forget_menu_open = false;
    if mode == FieldPackTargetMode::TmHmPokemon {
        runtime_shell.party_move_cursor = None;
    }
    runtime_shell.field_pack_target_mode = None;
    runtime_shell
        .last_audio_events
        .push("closed pack target".to_string());
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn move_visible_field_pack_target_cursor(
    runtime_shell: &mut BevyRuntimeShell,
    delta: isize,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    if snapshot.party.slots.is_empty() {
        runtime_shell.field_pack_target_mode = None;
        runtime_shell.party_move_cursor = None;
        runtime_shell
            .last_audio_events
            .push("party is empty".to_string());
        set_shell_action_status(runtime_shell, "NO POKEMON");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    move_visible_party_cursor(runtime_shell, delta)?;
    match runtime_shell.field_pack_target_mode {
        Some(FieldPackTargetMode::PartyMove) => {
            if delta != 0 {
                runtime_shell.party_move_cursor = None;
            }
            move_visible_party_move_cursor(runtime_shell, 0)?;
        }
        Some(FieldPackTargetMode::TmHmPokemon) => {
            runtime_shell.party_move_cursor = None;
        }
        _ => {}
    }
    Ok(())
}

fn move_visible_field_pack_target_secondary_cursor(
    runtime_shell: &mut BevyRuntimeShell,
    delta: isize,
) -> Result<()> {
    match runtime_shell.field_pack_target_mode {
        Some(FieldPackTargetMode::PartyMove) => {
            let snapshot = runtime_shell.shell.snapshot()?;
            if snapshot.party.slots.is_empty() {
                runtime_shell.field_pack_target_mode = None;
                runtime_shell.party_move_cursor = None;
                runtime_shell
                    .last_audio_events
                    .push("party is empty".to_string());
                set_shell_action_status(runtime_shell, "NO POKEMON");
                trim_event_log(&mut runtime_shell.last_audio_events);
                return Ok(());
            }
            move_visible_party_move_cursor(runtime_shell, delta)
        }
        Some(FieldPackTargetMode::TmHmPokemon) => {
            let snapshot = runtime_shell.shell.snapshot()?;
            if snapshot.party.slots.is_empty() {
                runtime_shell.field_pack_target_mode = None;
                runtime_shell.party_move_cursor = None;
                runtime_shell
                    .last_audio_events
                    .push("party is empty".to_string());
                set_shell_action_status(runtime_shell, "NO POKEMON");
                trim_event_log(&mut runtime_shell.last_audio_events);
                return Ok(());
            }
            move_visible_party_cursor(runtime_shell, delta)?;
            runtime_shell.party_move_cursor = None;
            Ok(())
        }
        Some(FieldPackTargetMode::PartyPokemon | FieldPackTargetMode::HeldItem) => {
            let snapshot = runtime_shell.shell.snapshot()?;
            if snapshot.party.slots.is_empty() {
                runtime_shell.field_pack_target_mode = None;
                runtime_shell
                    .last_audio_events
                    .push("party is empty".to_string());
                set_shell_action_status(runtime_shell, "NO POKEMON");
                trim_event_log(&mut runtime_shell.last_audio_events);
                return Ok(());
            }
            move_visible_party_cursor(runtime_shell, delta)
        }
        None => {
            record_visible_runtime_action(runtime_shell, "pack:target:move:none")?;
            runtime_shell
                .last_audio_events
                .push("no active pack target mode".to_string());
            set_shell_action_status(runtime_shell, "NO PACK TARGET");
            trim_event_log(&mut runtime_shell.last_audio_events);
            Ok(())
        }
    }
}

fn confirm_visible_field_pack_target(
    runtime_shell: &mut BevyRuntimeShell,
    mode: FieldPackTargetMode,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    if snapshot.party.slots.is_empty() {
        runtime_shell.field_pack_target_mode = None;
        runtime_shell.party_move_cursor = None;
        record_visible_runtime_action(
            runtime_shell,
            format!(
                "pack:target:{}:confirm:empty_party",
                field_pack_target_mode_label(mode)
            ),
        )?;
        runtime_shell
            .last_audio_events
            .push("party is empty".to_string());
        set_shell_action_status(runtime_shell, "NO POKEMON");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    let slot = selected_party_slot_snapshot(&snapshot, runtime_shell.party_cursor)?;
    if slot.pokemon.is_egg || slot.pokemon.species.id == "EGG" {
        record_visible_runtime_action(
            runtime_shell,
            format!(
                "pack:target:{}:egg_refused",
                field_pack_target_mode_label(mode)
            ),
        )?;
        runtime_shell
            .last_audio_events
            .push("field item refused for Egg".to_string());
        runtime_shell.field_notice = Some(if mode == FieldPackTargetMode::HeldItem {
            "Eggs cannot hold or receive items.".to_string()
        } else {
            "It won't have any effect.".to_string()
        });
        mark_runtime_snapshot_dirty(runtime_shell);
        set_shell_action_status(runtime_shell, "EGGS CAN'T USE THAT");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    if mode == FieldPackTargetMode::TmHmPokemon {
        return confirm_visible_tmhm_target(runtime_shell);
    }
    if mode == FieldPackTargetMode::PartyPokemon {
        let item_id = selected_field_pack_item_id(runtime_shell)?;
        let behavior = snapshot
            .item_effect_plans
            .iter()
            .find(|plan| plan.item_id == item_id)
            .map(|plan| plan.behavior_id.as_str());
        match behavior {
            Some(ITEM_EFFECT_BEHAVIOR_RARE_CANDY) => {
                return use_selected_rare_candy(runtime_shell);
            }
            Some(ITEM_EFFECT_BEHAVIOR_EVOLUTION_STONE) => {
                return use_selected_evolution_item(runtime_shell);
            }
            _ => {}
        }
    }
    let result = match mode {
        FieldPackTargetMode::PartyPokemon => use_selected_party_item(runtime_shell),
        FieldPackTargetMode::PartyMove => use_selected_pp_item(runtime_shell),
        FieldPackTargetMode::TmHmPokemon => unreachable!("TM/HM handled above"),
        FieldPackTargetMode::HeldItem => give_selected_held_item(runtime_shell),
    };
    if result.is_ok() {
        if mode == FieldPackTargetMode::PartyMove {
            runtime_shell.party_move_cursor = None;
        }
        runtime_shell.field_pack_target_mode = None;
    }
    result
}

fn field_pack_target_mode_label(mode: FieldPackTargetMode) -> &'static str {
    match mode {
        FieldPackTargetMode::PartyPokemon => "party Pokemon",
        FieldPackTargetMode::PartyMove => "party move",
        FieldPackTargetMode::TmHmPokemon => "TM/HM Pokemon",
        FieldPackTargetMode::HeldItem => "held item Pokemon",
    }
}

fn initialize_visible_tmhm_replacement_cursor(
    runtime_shell: &mut BevyRuntimeShell,
    snapshot: &RuntimeShellSnapshot,
) -> Result<()> {
    let party_index = selected_party_index(runtime_shell)?;
    let selected = snapshot
        .party
        .slots
        .iter()
        .find(|slot| slot.index == party_index)
        .with_context(|| format!("selected party index {party_index} is not in the party"))?;
    if selected.pokemon.moves.len() >= 4 {
        visible_cursor_index(
            &mut runtime_shell.party_move_cursor,
            &party_move_cursor_surface_id(party_index),
            selected.pokemon.moves.len(),
        );
    } else {
        runtime_shell.party_move_cursor = None;
    }
    Ok(())
}

fn open_visible_save_menu(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let Some(path) = runtime_shell.quick_save_path.clone() else {
        anyhow::bail!("Start Save has no configured .crystalsave path");
    };
    runtime_shell.shell.snapshot()?;
    runtime_shell.save_menu_open = true;
    runtime_shell.save_flow = Some(VisibleSaveFlow {
        stage: VisibleSaveFlowStage::Prompt,
        origin: VisibleSaveFlowOrigin::StartMenu,
        save_exists: runtime_shell
            .shell
            .runtime()
            .load_save_summary(&path)
            .is_ok(),
        yes_no_index: 0,
    });
    close_visible_party_detail_state(runtime_shell);
    runtime_shell.pokedex_menu_open = false;
    runtime_shell.pokedex_detail_open = false;
    runtime_shell.pokedex_scripted_entry = false;
    runtime_shell.pokegear_menu_open = false;
    runtime_shell.options_menu_open = false;
    runtime_shell.storage_cursor = None;
    runtime_shell.pc_item_cursor = None;
    close_visible_field_pack_without_log(runtime_shell);
    runtime_shell
        .last_audio_events
        .push("opened Save prompt".to_string());
    set_shell_action_status(runtime_shell, "SAVE");
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn close_visible_save_menu(runtime_shell: &mut BevyRuntimeShell) {
    runtime_shell.save_menu_open = false;
    runtime_shell.save_flow = None;
    runtime_shell
        .last_audio_events
        .push("closed Save".to_string());
    trim_event_log(&mut runtime_shell.last_audio_events);
}

fn confirm_visible_save_menu(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let Some(flow) = runtime_shell.save_flow.as_ref().cloned() else {
        anyhow::bail!("Save confirmation requested without an active Save flow");
    };
    match flow.stage {
        VisibleSaveFlowStage::Prompt | VisibleSaveFlowStage::OverwritePrompt => {
            if flow.yes_no_index == 1 {
                record_visible_runtime_action(runtime_shell, "menu:save:no")?;
                return finish_visible_save_flow(runtime_shell, flow.origin, false);
            }
            if flow.stage == VisibleSaveFlowStage::Prompt && flow.save_exists {
                if let Some(active_flow) = runtime_shell.save_flow.as_mut() {
                    active_flow.stage = VisibleSaveFlowStage::OverwritePrompt;
                    active_flow.yes_no_index = 0;
                }
                record_visible_runtime_action(runtime_shell, "menu:save:overwrite_prompt")?;
                return Ok(());
            }
            record_visible_runtime_action(runtime_shell, "menu:save:yes")?;
            if let Some(active_flow) = runtime_shell.save_flow.as_mut() {
                active_flow.stage = VisibleSaveFlowStage::Saving;
            }
            let save_result = match flow.origin {
                VisibleSaveFlowOrigin::StartMenu => quick_save_from_menu(runtime_shell),
                VisibleSaveFlowOrigin::BillsPcMove => quick_save_from_bill_pc(runtime_shell),
                VisibleSaveFlowOrigin::BillsPcChangeBox { box_index } => {
                    quick_save_from_bill_pc(runtime_shell).and_then(|()| {
                        let switched = runtime_shell.shell.switch_current_pc_box(box_index)?;
                        mark_runtime_snapshot_dirty(runtime_shell);
                        set_shell_action_status(
                            runtime_shell,
                            format!("BOX {} SELECTED", switched.box_index_after + 1),
                        );
                        Ok(())
                    })
                }
            };
            match save_result {
                Ok(()) => {
                    if let Some(active_flow) = runtime_shell.save_flow.as_mut() {
                        active_flow.stage = VisibleSaveFlowStage::Saved;
                    }
                    let BevyRuntimeShell {
                        shell,
                        pending_audio,
                        last_audio_events,
                        ..
                    } = runtime_shell;
                    queue_visible_sound_effect(
                        shell.runtime().audio(),
                        pending_audio,
                        last_audio_events,
                        "SFX_SAVE",
                    )?;
                    Ok(())
                }
                Err(error) => {
                    if let Some(active_flow) = runtime_shell.save_flow.as_mut() {
                        active_flow.stage = VisibleSaveFlowStage::Error;
                    }
                    record_visible_runtime_error(runtime_shell, &error);
                    runtime_shell.last_error = Some(error.to_string());
                    Ok(())
                }
            }
        }
        VisibleSaveFlowStage::Saving => Ok(()),
        VisibleSaveFlowStage::Saved | VisibleSaveFlowStage::Error => {
            record_visible_runtime_action(runtime_shell, "menu:save:close")?;
            finish_visible_save_flow(
                runtime_shell,
                flow.origin,
                flow.stage == VisibleSaveFlowStage::Saved,
            )
        }
    }
}

fn cancel_visible_save_menu(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let Some(flow) = runtime_shell.save_flow.as_ref().cloned() else {
        anyhow::bail!("Save cancel requested without an active Save flow");
    };
    match flow.stage {
        VisibleSaveFlowStage::Prompt | VisibleSaveFlowStage::OverwritePrompt => {
            record_visible_runtime_action(runtime_shell, "menu:save:no")?;
            finish_visible_save_flow(runtime_shell, flow.origin, false)
        }
        VisibleSaveFlowStage::Saving => Ok(()),
        VisibleSaveFlowStage::Saved | VisibleSaveFlowStage::Error => {
            record_visible_runtime_action(runtime_shell, "menu:save:close")?;
            finish_visible_save_flow(
                runtime_shell,
                flow.origin,
                flow.stage == VisibleSaveFlowStage::Saved,
            )
        }
    }
}

fn finish_visible_save_flow(
    runtime_shell: &mut BevyRuntimeShell,
    origin: VisibleSaveFlowOrigin,
    saved: bool,
) -> Result<()> {
    close_visible_save_menu(runtime_shell);
    match origin {
        VisibleSaveFlowOrigin::StartMenu => continue_visible_script_after_prompt(runtime_shell),
        VisibleSaveFlowOrigin::BillsPcMove if saved => {
            open_visible_bill_pc_move_mode(runtime_shell)
        }
        VisibleSaveFlowOrigin::BillsPcMove => {
            runtime_shell.bill_pc_action_cursor = Some(MenuCursor {
                surface_id: "pc:bill-actions".to_string(),
                option_index: 3,
            });
            set_shell_action_status(runtime_shell, "BILL'S PC");
            Ok(())
        }
        VisibleSaveFlowOrigin::BillsPcChangeBox { box_index } => {
            runtime_shell.bill_pc_box_cursor = Some(MenuCursor {
                surface_id: "pc:bill-boxes".to_string(),
                option_index: box_index,
            });
            set_shell_action_status(runtime_shell, "CHOOSE A BOX");
            Ok(())
        }
    }
}

fn move_visible_save_prompt_cursor(
    runtime_shell: &mut BevyRuntimeShell,
    delta: isize,
) -> Result<()> {
    let Some(flow) = runtime_shell.save_flow.as_mut() else {
        anyhow::bail!("Save cursor requested without an active Save flow");
    };
    if !matches!(
        flow.stage,
        VisibleSaveFlowStage::Prompt | VisibleSaveFlowStage::OverwritePrompt
    ) {
        return Ok(());
    }
    flow.yes_no_index = if delta < 0 {
        flow.yes_no_index.saturating_sub(1)
    } else if delta > 0 {
        (flow.yes_no_index + 1).min(1)
    } else {
        flow.yes_no_index
    };
    Ok(())
}

fn open_visible_options_menu(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    runtime_shell.shell.snapshot()?;
    // `_Option` zeros `wJumptableIndex` before drawing and again after its
    // initial value pass. Every newly opened menu therefore starts on
    // OPT_TEXT_SPEED regardless of the row used to close the prior menu.
    runtime_shell.options_cursor = 0;
    runtime_shell.options_menu_open = true;
    close_visible_party_detail_state(runtime_shell);
    runtime_shell.pokedex_menu_open = false;
    runtime_shell.pokedex_detail_open = false;
    runtime_shell.pokedex_scripted_entry = false;
    runtime_shell.pokegear_menu_open = false;
    runtime_shell.save_menu_open = false;
    runtime_shell.save_flow = None;
    runtime_shell.storage_cursor = None;
    runtime_shell.pc_item_cursor = None;
    close_visible_field_pack_without_log(runtime_shell);
    runtime_shell
        .last_audio_events
        .push("opened Options".to_string());
    set_shell_action_status(runtime_shell, "OPTIONS");
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn close_visible_options_menu(runtime_shell: &mut BevyRuntimeShell) {
    runtime_shell.options_menu_open = false;
    runtime_shell
        .last_audio_events
        .push("closed Options".to_string());
    trim_event_log(&mut runtime_shell.last_audio_events);
}

fn close_visible_special_boundary(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let Some(boundary) = runtime_shell.special_boundary.take() else {
        record_visible_runtime_action(runtime_shell, "special:ack:none")?;
        runtime_shell
            .last_audio_events
            .push("no visible special boundary is open".to_string());
        set_shell_action_status(runtime_shell, "NO SPECIAL WINDOW");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    };
    runtime_shell.visible_special_text_pause_frames = None;
    runtime_shell.visible_internal_special_delay_frames = None;
    if boundary.label == "WaitSfx" && !visible_wait_sfx_finished(runtime_shell) {
        runtime_shell.special_boundary = Some(boundary);
        runtime_shell
            .last_audio_events
            .push("WaitSFX remains blocked until the transient channel finishes".to_string());
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    if boundary.label == "DayCareEgg"
        && let Some(sound) = runtime_shell.pending_special_sound.take()
    {
        queue_visible_shell_sound_effect(runtime_shell, &sound)?;
        runtime_shell.visible_special_text_pause_frames = Some(120);
        runtime_shell.special_boundary = Some(SpecialBoundaryDisplay {
            label: "DayCareEggDelay".to_string(),
            details: boundary.details,
        });
        set_shell_action_status(runtime_shell, "RECEIVED EGG");
        mark_runtime_snapshot_dirty(runtime_shell);
        return Ok(());
    }
    if boundary.label == "HallOfFamePC" {
        queue_visible_shell_sound_effect(runtime_shell, "SFX_MENU")?;
    }
    record_visible_runtime_action(runtime_shell, format!("special:ack:{}", boundary.label))?;
    if boundary.label == "WaitSfx"
        && runtime_shell
            .shell
            .snapshot()?
            .script_events
            .waiting_for_sound_effect
    {
        let consumed = runtime_shell
            .shell
            .consume_script_runtime_flag(RuntimeScriptRuntimeFlag::WaitingForSoundEffect)?;
        runtime_shell
            .last_audio_events
            .push(format!("consumed runtime flag {:?}", consumed));
    }
    runtime_shell
        .last_audio_events
        .push(format!("closed special boundary {}", boundary.label));
    trim_event_log(&mut runtime_shell.last_audio_events);
    if boundary.label == "PrinterError2" && runtime_shell.visible_diploma.take().is_some() {
        mark_runtime_snapshot_dirty(runtime_shell);
    }
    if matches!(
        boundary.label.as_str(),
        "DayCareMon" | "IllRaiseYourMonText" | "DayCareWithdrawText"
    ) {
        if let Some(species) = runtime_shell.pending_special_cry.take() {
            queue_visible_pokemon_cry(runtime_shell, &species, "day_care")?;
        }
    }
    if matches!(
        (
            boundary.label.as_str(),
            runtime_shell.pending_script_party_selection.as_ref(),
        ),
        (
            "WhichMonPhotoText",
            Some(PendingScriptPartySelection::PhotoStudio),
        ) | (
            "SeerSeeAllText",
            Some(PendingScriptPartySelection::PokeSeer),
        ) | (
            "NameRaterWhichMonText",
            Some(PendingScriptPartySelection::NameRater),
        ) | (
            "DeleterAskWhichMonText",
            Some(PendingScriptPartySelection::MoveDeletion { party_index: None }),
        ) | (
            "WhatShouldIRaiseText",
            Some(PendingScriptPartySelection::DayCareDeposit { .. }),
        )
    ) && runtime_shell.special_boundary_queue.is_empty()
    {
        open_visible_party_menu(runtime_shell)?;
        set_shell_action_status(runtime_shell, "CHOOSE A POKEMON");
        mark_runtime_snapshot_dirty(runtime_shell);
        return Ok(());
    }
    if boundary.label == "NPCTradeCableText" {
        let pending = runtime_shell
            .pending_npc_trade_commit
            .take()
            .context("NPC trade cable prompt has no retained trade commit")?;
        apply_visible_npc_trade_selection(
            runtime_shell,
            pending.origin_map_name,
            pending.source_script,
            pending.command_index,
            pending.trade_id.clone(),
            Some(pending.party_index),
        )?;
        let completion_text = runtime_shell
            .pc_notice
            .take()
            .context("successful NPC trade produced no completion text")?;
        let snapshot = runtime_shell.shell.snapshot()?;
        let rule = snapshot
            .special
            .npc_trades
            .get(&pending.trade_id)
            .with_context(|| {
                format!(
                    "NPC trade {} is missing after its cable prompt",
                    pending.trade_id
                )
            })?;
        let requested = visible_npc_trade_requested_name(rule);
        let offered = rule.offered_species.replace('_', " ");
        let completion_label = if rule.dialog_set.ends_with("NEWBIE") {
            "NPCTradeCompleteText4"
        } else if rule.dialog_set.ends_with("GIRL") {
            "NPCTradeCompleteText3"
        } else if rule.dialog_set.ends_with("HAPPY") {
            "NPCTradeCompleteText2"
        } else {
            "NPCTradeCompleteText1"
        };
        runtime_shell.special_boundary_queue.clear();
        runtime_shell.special_boundary = Some(SpecialBoundaryDisplay {
            label: "Text_NPCTraded".to_string(),
            details: vec![format!(
                "{} traded\n{requested} for\n{offered}.",
                snapshot.trainer.player_name
            )],
        });
        runtime_shell
            .special_boundary_queue
            .push_back(SpecialBoundaryDisplay {
                label: completion_label.to_string(),
                details: vec![completion_text],
            });
        set_shell_action_status(runtime_shell, "TRADE COMPLETE");
        mark_runtime_snapshot_dirty(runtime_shell);
        return Ok(());
    }
    if boundary.label == "NameRaterWhatNameText"
        && matches!(
            runtime_shell.pending_script_party_selection,
            Some(PendingScriptPartySelection::NameRater)
        )
    {
        let party_index = selected_party_index(runtime_shell)?;
        let snapshot = runtime_shell.shell.snapshot()?;
        let nickname = snapshot
            .party
            .slots
            .iter()
            .find(|slot| slot.index == party_index)
            .map(|slot| slot.pokemon.nickname.clone())
            .context("Name Rater selection is no longer in the party")?;
        runtime_shell.pending_script_party_selection = None;
        runtime_shell.pending_name_input = Some(PendingNameInput {
            label: "POKéMON'S NAME?".to_string(),
            value: nickname,
            max_length: 10,
            cursor_column: 0,
            cursor_row: 0,
            case: NameInputCase::Upper,
        });
        set_shell_action_status(runtime_shell, "POKEMON NAME");
        mark_runtime_snapshot_dirty(runtime_shell);
        return Ok(());
    }
    if boundary.label == "HoldStillText" {
        let party_index = runtime_shell
            .pending_photo_studio_commit
            .take()
            .context("Photo Studio hold-still text has no retained party selection")?;
        let used = runtime_shell.shell.open_photo_studio_special(party_index)?;
        runtime_shell.last_audio_events.push(format!(
            "Photo Studio print outcome={:?} checksum={:?}",
            used.outcome.effect, used.state_checksum
        ));
        runtime_shell.special_boundary_queue.clear();
        runtime_shell.special_boundary = Some(SpecialBoundaryDisplay {
            label: "PrinterError2".to_string(),
            details: vec![
                "Printer Error 2".to_string(),
                String::new(),
                "Check the Game Boy".to_string(),
                "Printer Manual.".to_string(),
                "Press B to Cancel".to_string(),
            ],
        });
        let no_photo =
            visible_exported_special_text_boundaries(runtime_shell, "NoPhotoText", "_NoPhotoText")?;
        runtime_shell.special_boundary_queue.extend(no_photo);
        set_shell_action_status(runtime_shell, "PRINTER ERROR 2");
        mark_runtime_snapshot_dirty(runtime_shell);
        return Ok(());
    }
    if boundary.label == "PrinterError2"
        && runtime_shell
            .special_boundary_queue
            .front()
            .is_some_and(|next| next.label == "NoPhotoText")
        && runtime_shell
            .shell
            .snapshot()?
            .ui
            .active_pokemon_picture
            .is_some()
    {
        let closed = runtime_shell.shell.close_active_pokemon_picture()?;
        runtime_shell.last_audio_events.push(format!(
            "Photo Studio restored map after printer cancellation species={} checksum={:?}",
            closed.species_id, closed.state_checksum
        ));
    }
    if boundary.label == "DeleterAskWhichMoveText" {
        let party_index = match runtime_shell.pending_script_party_selection.as_ref() {
            Some(PendingScriptPartySelection::MoveDeletion {
                party_index: Some(party_index),
            }) => *party_index,
            _ => anyhow::bail!("Move Deleter prompt has no retained party selection"),
        };
        open_visible_party_menu(runtime_shell)?;
        runtime_shell.party_move_cursor = Some(MenuCursor {
            surface_id: party_move_cursor_surface_id(party_index),
            option_index: 0,
        });
        set_shell_action_status(runtime_shell, "WHICH MOVE SHOULD IT FORGET?");
        mark_runtime_snapshot_dirty(runtime_shell);
        return Ok(());
    }
    if boundary.label == "Text_1_2_and_Poof" {
        anyhow::ensure!(
            runtime_shell
                .special_boundary_queue
                .front()
                .map(|next| next.label.as_str())
                == Some("MoveForgotPoofText"),
            "Move Tutor count text is not followed by MoveForgotPoofText"
        );
        queue_visible_shell_sound_effect(runtime_shell, "SFX_SWITCH_POKEMON")?;
    } else if boundary.label == "MoveForgotText"
        && runtime_shell
            .special_boundary_queue
            .front()
            .is_some_and(|next| next.label == "LearnedMoveText")
    {
        queue_visible_shell_sound_effect(runtime_shell, "SFX_DEX_FANFARE_50_79")?;
    }
    if let Some(next) = runtime_shell.special_boundary_queue.pop_front() {
        let next_label = next.label.clone();
        runtime_shell.special_boundary = Some(next);
        if next_label == "MoveForgotPoofText" {
            runtime_shell.visible_special_text_pause_frames = Some(30);
        }
        set_shell_action_status(runtime_shell, next_label);
        mark_runtime_snapshot_dirty(runtime_shell);
        return Ok(());
    }
    if matches!(
        runtime_shell.pending_script_party_selection.as_ref(),
        Some(PendingScriptPartySelection::MoveTutor { .. })
    ) && matches!(
        boundary.label.as_str(),
        "KnowsMoveText" | "TMHMNotCompatibleText" | "MoveCantForgetHMText"
    ) {
        let status = if matches!(
            runtime_shell.pending_script_party_selection.as_ref(),
            Some(PendingScriptPartySelection::MoveTutor {
                party_index: Some(_),
                ..
            })
        ) {
            "WHICH MOVE SHOULD BE FORGOTTEN?"
        } else {
            "WHICH POKéMON?"
        };
        set_shell_action_status(runtime_shell, status);
        mark_runtime_snapshot_dirty(runtime_shell);
        return Ok(());
    }
    if boundary.label == "PokecenterPCTurnOnText" {
        runtime_shell.pc_hub_cursor = Some(MenuCursor {
            surface_id: "pc:hub".to_string(),
            option_index: 0,
        });
        set_shell_action_status(runtime_shell, "ACCESS WHOSE PC?");
        mark_runtime_snapshot_dirty(runtime_shell);
        return Ok(());
    }
    if matches!(
        boundary.label.as_str(),
        "PokecenterBillsPCText" | "PokecenterPlayersPCText"
    ) {
        mark_runtime_snapshot_dirty(runtime_shell);
        return Ok(());
    }
    if boundary.label == "PokecenterPCOaksClosedText" {
        return turn_off_visible_pc_hub(runtime_shell);
    }
    if boundary.label == "PokecenterPCCantUseText" {
        let snapshot = runtime_shell.shell.snapshot()?;
        if snapshot.ui.menu.is_some() {
            let _ = runtime_shell.shell.close_active_menu()?;
        } else if snapshot.ui.window_open {
            let _ = runtime_shell.shell.close_runtime_window()?;
        }
        return continue_visible_script_after_prompt(runtime_shell);
    }
    if runtime_shell.pc_hub_session_open
        && matches!(boundary.label.as_str(), "ProfOaksPcBoot" | "HallOfFamePC")
    {
        runtime_shell.hall_of_fame_pc_index = None;
        runtime_shell.pc_hub_cursor = Some(MenuCursor {
            surface_id: "pc:hub".to_string(),
            option_index: 0,
        });
        set_shell_action_status(runtime_shell, "ACCESS WHOSE PC?");
        return Ok(());
    }
    if matches!(
        boundary.label.as_str(),
        "NameRaterHelloText" | "NameRaterBetterNameText" | "DeleterIntroText"
    ) && matches!(
        runtime_shell.pc_confirmation,
        Some(VisiblePcConfirmation::ScriptPartyIntro(_) | VisiblePcConfirmation::NameRaterRename)
    ) {
        mark_runtime_snapshot_dirty(runtime_shell);
        return Ok(());
    }
    continue_visible_script_after_prompt(runtime_shell)
}

fn visible_wait_sfx_finished(runtime_shell: &mut BevyRuntimeShell) -> bool {
    if runtime_shell
        .pending_audio
        .iter()
        .any(|command| !matches!(command.kind, ModpackAudioKind::Music))
    {
        return false;
    }
    !runtime_shell.transient_audio_playing
}

fn move_visible_options_cursor(runtime_shell: &mut BevyRuntimeShell, delta: isize) -> Result<()> {
    runtime_shell.shell.snapshot()?;
    anyhow::ensure!(
        runtime_shell.options_cursor < OPTIONS_MENU_ITEMS.len(),
        "Options cursor {} is out of range for {} rows",
        runtime_shell.options_cursor,
        OPTIONS_MENU_ITEMS.len()
    );
    let current = runtime_shell.options_cursor;
    let next = wrapped_index(current, OPTIONS_MENU_ITEMS.len(), delta);
    runtime_shell.options_cursor = next;
    #[cfg(test)]
    runtime_shell.last_audio_events.push(format!(
        "Options cursor {}->{} {}",
        current + 1,
        next + 1,
        options_menu_item_label(OPTIONS_MENU_ITEMS[next])
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn change_visible_options_selection(
    runtime_shell: &mut BevyRuntimeShell,
    delta: isize,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let selected = OPTIONS_MENU_ITEMS
        .get(runtime_shell.options_cursor)
        .copied()
        .with_context(|| {
            format!(
                "Options cursor {} is out of range for {} rows",
                runtime_shell.options_cursor,
                OPTIONS_MENU_ITEMS.len()
            )
        })?;
    if selected == OptionsMenuItem::Cancel {
        return Ok(());
    }
    let mut options = snapshot.trainer.options.clone();
    match selected {
        OptionsMenuItem::TextSpeed => {
            options.text_speed = cycle_text_speed(options.text_speed, delta);
        }
        OptionsMenuItem::BattleScene => {
            options.battle_scene = match options.battle_scene {
                BattleScene::On => BattleScene::Off,
                BattleScene::Off => BattleScene::On,
            };
        }
        OptionsMenuItem::BattleStyle => {
            options.battle_style = match options.battle_style {
                BattleStyle::Shift => BattleStyle::Set,
                BattleStyle::Set => BattleStyle::Shift,
            };
        }
        OptionsMenuItem::Sound => {
            options.sound = match options.sound {
                Sound::Mono => Sound::Stereo,
                Sound::Stereo => Sound::Mono,
            };
        }
        OptionsMenuItem::Print => {
            options.print_option = cycle_print_option(options.print_option, delta);
        }
        OptionsMenuItem::MenuAccount => {
            options.menu_account = match options.menu_account {
                MenuAccount::On => MenuAccount::Off,
                MenuAccount::Off => MenuAccount::On,
            };
        }
        OptionsMenuItem::Frame => {
            options.frame = cycle_frame_type(options.frame, delta);
        }
        OptionsMenuItem::Cancel => unreachable!("CANCEL returns before changing options"),
    }
    record_visible_runtime_action(
        runtime_shell,
        format!(
            "options:set:{}:{}",
            options_menu_item_id(selected),
            option_value_for_item(&options, selected)
        ),
    )?;
    let result = runtime_shell.shell.set_options(options)?;
    runtime_shell.last_audio_events.push(format!(
        "Options {} {:?}->{:?} checksum={:?}",
        options_menu_item_label(selected),
        option_value_for_item(&result.options_before, selected),
        option_value_for_item(&result.options_after, selected),
        result.state_checksum
    ));
    set_shell_action_status(
        runtime_shell,
        format!(
            "{} {}",
            options_menu_item_label(selected),
            option_value_for_item(&result.options_after, selected)
        ),
    );
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn confirm_visible_options_selection(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    runtime_shell.shell.snapshot()?;
    let selected = OPTIONS_MENU_ITEMS
        .get(runtime_shell.options_cursor)
        .copied()
        .with_context(|| {
            format!(
                "Options cursor {} is out of range for {} rows",
                runtime_shell.options_cursor,
                OPTIONS_MENU_ITEMS.len()
            )
        })?;
    if selected != OptionsMenuItem::Cancel {
        return Ok(());
    }
    record_visible_runtime_action(runtime_shell, "options:cancel")?;
    close_visible_options_menu(runtime_shell);
    set_shell_action_status(runtime_shell, "OPTIONS CLOSED");
    Ok(())
}

fn cycle_text_speed(current: TextSpeed, delta: isize) -> TextSpeed {
    const VALUES: &[TextSpeed] = &[TextSpeed::Fast, TextSpeed::Mid, TextSpeed::Slow];
    VALUES[cycle_value_index(VALUES, current, delta)]
}

fn cycle_print_option(current: PrintOption, delta: isize) -> PrintOption {
    const VALUES: &[PrintOption] = &[
        PrintOption::Lightest,
        PrintOption::Lighter,
        PrintOption::Normal,
        PrintOption::Darker,
        PrintOption::Darkest,
    ];
    VALUES[cycle_value_index(VALUES, current, delta)]
}

fn cycle_frame_type(current: FrameType, delta: isize) -> FrameType {
    const VALUES: &[FrameType] = &[
        FrameType::Frame1,
        FrameType::Frame2,
        FrameType::Frame3,
        FrameType::Frame4,
        FrameType::Frame5,
        FrameType::Frame6,
        FrameType::Frame7,
        FrameType::Frame8,
    ];
    VALUES[cycle_value_index(VALUES, current, delta)]
}

fn cycle_value_index<T: Copy + PartialEq>(values: &[T], current: T, delta: isize) -> usize {
    let current_index = values
        .iter()
        .position(|value| *value == current)
        .unwrap_or(0);
    wrapped_index(current_index, values.len(), delta)
}

fn options_menu_item_label(item: OptionsMenuItem) -> &'static str {
    match item {
        OptionsMenuItem::TextSpeed => "TEXT SPEED",
        OptionsMenuItem::BattleScene => "BATTLE SCENE",
        OptionsMenuItem::BattleStyle => "BATTLE STYLE",
        OptionsMenuItem::Sound => "SOUND",
        OptionsMenuItem::Print => "PRINT",
        OptionsMenuItem::MenuAccount => "MENU ACCOUNT",
        OptionsMenuItem::Frame => "FRAME",
        OptionsMenuItem::Cancel => "CANCEL",
    }
}

fn options_menu_item_id(item: OptionsMenuItem) -> &'static str {
    match item {
        OptionsMenuItem::TextSpeed => "text_speed",
        OptionsMenuItem::BattleScene => "battle_scene",
        OptionsMenuItem::BattleStyle => "battle_style",
        OptionsMenuItem::Sound => "sound",
        OptionsMenuItem::Print => "print",
        OptionsMenuItem::MenuAccount => "menu_account",
        OptionsMenuItem::Frame => "frame",
        OptionsMenuItem::Cancel => "cancel",
    }
}

fn option_value_for_item(options: &crate::core::state::Options, item: OptionsMenuItem) -> String {
    match item {
        OptionsMenuItem::TextSpeed => match options.text_speed {
            TextSpeed::Fast => "FAST",
            TextSpeed::Mid => "MID ",
            TextSpeed::Slow => "SLOW",
        }
        .to_string(),
        OptionsMenuItem::BattleScene => match options.battle_scene {
            BattleScene::On => "ON ",
            BattleScene::Off => "OFF",
        }
        .to_string(),
        OptionsMenuItem::BattleStyle => match options.battle_style {
            BattleStyle::Shift => "SHIFT",
            BattleStyle::Set => "SET  ",
        }
        .to_string(),
        OptionsMenuItem::Sound => match options.sound {
            Sound::Mono => "MONO  ",
            Sound::Stereo => "STEREO",
        }
        .to_string(),
        OptionsMenuItem::Print => match options.print_option {
            PrintOption::Lightest => "LIGHTEST",
            PrintOption::Lighter => "LIGHTER ",
            PrintOption::Normal => "NORMAL  ",
            PrintOption::Darker => "DARKER  ",
            PrintOption::Darkest => "DARKEST ",
        }
        .to_string(),
        OptionsMenuItem::MenuAccount => match options.menu_account {
            MenuAccount::On => "ON ",
            MenuAccount::Off => "OFF",
        }
        .to_string(),
        OptionsMenuItem::Frame => match options.frame {
            FrameType::Frame1 => "1",
            FrameType::Frame2 => "2",
            FrameType::Frame3 => "3",
            FrameType::Frame4 => "4",
            FrameType::Frame5 => "5",
            FrameType::Frame6 => "6",
            FrameType::Frame7 => "7",
            FrameType::Frame8 => "8",
        }
        .to_string(),
        OptionsMenuItem::Cancel => String::new(),
    }
}

fn open_visible_party_menu(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    if snapshot.party.slots.is_empty() {
        record_visible_runtime_action(runtime_shell, "party:open:empty")?;
        runtime_shell
            .last_audio_events
            .push("party is empty".to_string());
        set_shell_action_status(runtime_shell, "NO POKEMON");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    initialize_visible_party_cursor(runtime_shell, &snapshot);
    let selected_species = selected_party_species_label(&snapshot, runtime_shell.party_cursor)
        .with_context(|| {
            format!(
                "party cursor {} is not backed by a party slot after opening party menu",
                runtime_shell.party_cursor
            )
        })?;
    runtime_shell.party_menu_open = true;
    runtime_shell.party_summary_open = false;
    runtime_shell.party_move_reorder_open = false;
    runtime_shell.party_move_reorder_origin = None;
    runtime_shell.pokedex_menu_open = false;
    runtime_shell.pokedex_detail_open = false;
    runtime_shell.pokedex_scripted_entry = false;
    runtime_shell.pokegear_menu_open = false;
    runtime_shell.options_menu_open = false;
    runtime_shell.save_menu_open = false;
    runtime_shell.save_flow = None;
    runtime_shell.field_pack_pocket = None;
    runtime_shell.field_pack_action_cursor = None;
    runtime_shell.field_pack_target_mode = None;
    runtime_shell.bag_cursor = None;
    runtime_shell.key_item_cursor = None;
    runtime_shell.ball_cursor = None;
    runtime_shell.tmhm_cursor = None;
    runtime_shell.pc_item_cursor = None;
    runtime_shell.fly_cursor = None;
    runtime_shell.last_audio_events.push(format!(
        "opened Pokemon party selected={}",
        selected_species
    ));
    set_shell_action_status(runtime_shell, format!("POKEMON {}", selected_species));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn close_visible_party_menu(runtime_shell: &mut BevyRuntimeShell) {
    runtime_shell.party_menu_open = false;
    runtime_shell.party_summary_open = false;
    runtime_shell.party_move_cursor = None;
    runtime_shell.party_action_cursor = None;
    runtime_shell.party_give_take_cursor = None;
    runtime_shell.party_move_reorder_open = false;
    runtime_shell.party_move_reorder_origin = None;
    runtime_shell.party_switch_cursor = None;
    runtime_shell.party_hp_transfer_source = None;
    runtime_shell.party_hp_transfer_move = None;
    runtime_shell.fly_cursor = None;
    runtime_shell
        .last_audio_events
        .push("closed Pokemon party".to_string());
    trim_event_log(&mut runtime_shell.last_audio_events);
}

fn open_visible_party_action_menu(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    if snapshot.party.slots.is_empty() {
        runtime_shell.party_action_cursor = None;
        record_visible_runtime_action(runtime_shell, "party:actions:empty_party")?;
        runtime_shell
            .last_audio_events
            .push("party is empty".to_string());
        set_shell_action_status(runtime_shell, "NO POKEMON");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    anyhow::ensure!(
        runtime_shell.party_cursor <= snapshot.party.slots.len(),
        "party cursor {} is outside {} Pokemon/CANCEL rows",
        runtime_shell.party_cursor,
        snapshot.party.slots.len() + 1
    );
    if runtime_shell.party_cursor == snapshot.party.slots.len() {
        record_visible_runtime_action(runtime_shell, "party:cancel")?;
        close_visible_party_menu(runtime_shell);
        set_shell_action_status(runtime_shell, "POKEMON CLOSED");
        return Ok(());
    }
    let actions = visible_party_actions(&snapshot, runtime_shell)?;
    if actions.is_empty() {
        record_visible_runtime_action(runtime_shell, "party:actions:empty")?;
        runtime_shell
            .last_audio_events
            .push("selected Pokemon has no party actions".to_string());
        set_shell_action_status(runtime_shell, "NO ACTIONS");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    visible_cursor_index(
        &mut runtime_shell.party_action_cursor,
        "party:actions",
        actions.len(),
    );
    runtime_shell.last_audio_events.push(format!(
        "opened party actions {}",
        visible_party_action_labels(&actions).join("/")
    ));
    set_shell_action_status(
        runtime_shell,
        format!(
            "ACTIONS {}",
            visible_party_action_labels(&actions).join("/")
        ),
    );
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn close_visible_party_action_menu(runtime_shell: &mut BevyRuntimeShell) {
    runtime_shell.party_action_cursor = None;
    runtime_shell.party_give_take_cursor = None;
    runtime_shell.party_move_reorder_open = false;
    runtime_shell.party_move_reorder_origin = None;
    runtime_shell.party_switch_cursor = None;
    runtime_shell
        .last_audio_events
        .push("closed party actions".to_string());
    trim_event_log(&mut runtime_shell.last_audio_events);
}

fn open_visible_party_summary(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    if snapshot.party.slots.is_empty() {
        runtime_shell.party_summary_open = false;
        runtime_shell.party_action_cursor = None;
        record_visible_runtime_action(runtime_shell, "party:summary:empty_party")?;
        runtime_shell
            .last_audio_events
            .push("party is empty".to_string());
        set_shell_action_status(runtime_shell, "NO POKEMON");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    let slot = selected_party_slot_snapshot(&snapshot, runtime_shell.party_cursor)?;
    let slot_index = slot.index;
    let species_id = slot.pokemon.species.id.clone();
    let level = slot.pokemon.level;
    let hp = slot.pokemon.hp;
    let max_hp = slot.pokemon.max_hp;
    runtime_shell.party_summary_open = true;
    runtime_shell.party_summary_page = 1;
    runtime_shell.party_action_cursor = None;
    runtime_shell.party_switch_cursor = None;
    runtime_shell.fly_cursor = None;
    runtime_shell.last_audio_events.push(format!(
        "opened party summary index={} species={} level={} hp={}/{}",
        slot_index, species_id, level, hp, max_hp
    ));
    queue_visible_pokemon_cry(runtime_shell, &species_id, "party_summary")?;
    set_shell_action_status(
        runtime_shell,
        format!("{species_id} L{level} HP {hp}/{max_hp}"),
    );
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn close_visible_party_summary(runtime_shell: &mut BevyRuntimeShell) {
    runtime_shell.party_summary_open = false;
    runtime_shell.party_summary_page = 1;
    runtime_shell
        .last_audio_events
        .push("closed party summary".to_string());
    trim_event_log(&mut runtime_shell.last_audio_events);
}

fn cycle_visible_party_summary_page(
    runtime_shell: &mut BevyRuntimeShell,
    delta: isize,
) -> Result<()> {
    if !visible_wait_sfx_finished(runtime_shell) {
        return Ok(());
    }
    let snapshot = runtime_shell.shell.snapshot()?;
    let slot = selected_party_slot_snapshot(&snapshot, runtime_shell.party_cursor)?;
    if slot.pokemon.is_egg {
        return Ok(());
    }
    anyhow::ensure!(
        (1..=3).contains(&runtime_shell.party_summary_page),
        "party summary page {} is outside 1..=3",
        runtime_shell.party_summary_page
    );
    runtime_shell.party_summary_page =
        (wrapped_index(usize::from(runtime_shell.party_summary_page - 1), 3, delta) + 1) as u8;
    runtime_shell.last_audio_events.push(format!(
        "party summary page {}",
        runtime_shell.party_summary_page
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(())
}

fn move_visible_party_summary_pokemon(
    runtime_shell: &mut BevyRuntimeShell,
    delta: isize,
) -> Result<()> {
    if !visible_wait_sfx_finished(runtime_shell) {
        return Ok(());
    }
    let snapshot = runtime_shell.shell.snapshot()?;
    if snapshot.party.slots.is_empty() {
        return Ok(());
    }
    anyhow::ensure!(
        runtime_shell.party_cursor < snapshot.party.slots.len(),
        "party cursor {} is outside {} Pokemon rows",
        runtime_shell.party_cursor,
        snapshot.party.slots.len()
    );
    let current = runtime_shell.party_cursor;
    // StatsScreen_JoypadAction stops at the first/last party member; unlike
    // ordinary menu rows, summary-screen Up/Down does not wrap.
    let next = if delta.is_negative() {
        current.checked_sub(delta.unsigned_abs())
    } else {
        current
            .checked_add(delta as usize)
            .filter(|next| *next < snapshot.party.slots.len())
    };
    let Some(next) = next else {
        return Ok(());
    };
    runtime_shell.party_cursor = next;
    if runtime_shell.battle_party_summary_open {
        // The cartridge updates wPartyMenuCursor while browsing stats, so
        // leaving STATS returns to the party row that is currently displayed.
        runtime_shell.battle_switch_cursor = Some(MenuCursor {
            surface_id: "battle:switch".to_string(),
            option_index: next,
        });
    }
    runtime_shell.party_summary_page = 1;
    let slot = selected_party_slot_snapshot(&snapshot, runtime_shell.party_cursor)?;
    if !slot.pokemon.is_egg {
        queue_visible_pokemon_cry(runtime_shell, &slot.pokemon.species.id, "party_summary")?;
    }
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(())
}

fn move_visible_party_action_cursor(
    runtime_shell: &mut BevyRuntimeShell,
    delta: isize,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let actions = visible_party_actions(&snapshot, runtime_shell)?;
    move_visible_cursor_slot(
        &mut runtime_shell.party_action_cursor,
        "party:actions".to_string(),
        actions.len(),
        delta,
        &mut runtime_shell.last_audio_events,
    )
}

fn execute_visible_party_action(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let actions = visible_party_actions(&snapshot, runtime_shell)?;
    let party_index = selected_party_index(runtime_shell)?;
    let index = strict_readonly_cursor_index(
        &runtime_shell.party_action_cursor,
        "party:actions",
        actions.len(),
    )
    .context("party action surface party:actions is active without a valid cursor")?;
    record_visible_runtime_action(
        runtime_shell,
        format!(
            "party:action:{}:{}",
            party_index,
            party_action_record_id(actions[index])
        ),
    )?;
    match actions[index] {
        PartyAction::Summary => open_visible_party_summary(runtime_shell),
        PartyAction::Switch => open_visible_party_switch_target(runtime_shell),
        PartyAction::Move => open_visible_party_move_reorder(runtime_shell),
        PartyAction::Item => {
            runtime_shell.party_action_cursor = None;
            visible_cursor_index(
                &mut runtime_shell.party_give_take_cursor,
                "party:give-take",
                2,
            );
            set_shell_action_status(runtime_shell, "GIVE OR TAKE?");
            trim_event_log(&mut runtime_shell.last_audio_events);
            Ok(())
        }
        PartyAction::Cancel => {
            close_visible_party_action_menu(runtime_shell);
            set_shell_action_status(runtime_shell, "POKEMON");
            Ok(())
        }
        PartyAction::FieldMove(field_move) => {
            execute_visible_party_field_move(runtime_shell, field_move)
        }
    }
}

fn confirm_visible_party_give_take(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    if runtime_shell
        .party_give_take_cursor
        .as_ref()
        .is_some_and(|cursor| cursor.surface_id == "party:mail-actions")
    {
        return confirm_visible_party_mail_action(runtime_shell);
    }
    let selected =
        strict_readonly_cursor_index(&runtime_shell.party_give_take_cursor, "party:give-take", 2)
            .context("party GIVE/TAKE menu requires a valid cursor")?;
    runtime_shell.party_give_take_cursor = None;
    if selected == 0 {
        let party_index = selected_party_index(runtime_shell)?;
        record_visible_runtime_action(runtime_shell, "party:held_item:give")?;
        runtime_shell.party_held_item_give_target = Some(party_index);
        close_visible_party_menu(runtime_shell);
        open_visible_field_pack(runtime_shell)?;
        set_shell_action_status(runtime_shell, "CHOOSE AN ITEM TO GIVE");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    let snapshot = runtime_shell.shell.snapshot()?;
    let selected_slot = selected_party_slot_snapshot(&snapshot, runtime_shell.party_cursor)?;
    if selected_slot.pokemon.item.is_none() {
        record_visible_runtime_action(runtime_shell, "party:held_item:take:none")?;
        runtime_shell.field_notice = Some(format!(
            "{} isn't holding anything.",
            selected_slot.pokemon.nickname
        ));
        mark_runtime_snapshot_dirty(runtime_shell);
        set_shell_action_status(runtime_shell, "NOT HOLDING AN ITEM");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    if selected_slot.pokemon.mail.is_some() {
        record_visible_runtime_action(runtime_shell, "party:held_item:take:mail")?;
        runtime_shell.party_give_take_cursor = Some(MenuCursor {
            surface_id: "party:mail-actions".to_string(),
            option_index: 0,
        });
        set_shell_action_status(runtime_shell, "MAIL");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    take_selected_held_item(runtime_shell)
}

fn confirm_visible_party_mail_action(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let selected = strict_readonly_cursor_index(
        &runtime_shell.party_give_take_cursor,
        "party:mail-actions",
        3,
    )
    .context("party Mail menu requires a valid cursor")?;
    let snapshot = runtime_shell.shell.snapshot()?;
    let slot = selected_party_slot_snapshot(&snapshot, runtime_shell.party_cursor)?;
    match selected {
        0 => {
            let mail = slot
                .pokemon
                .mail
                .as_ref()
                .context("selected Pokemon has no Mail")?;
            record_visible_runtime_action(runtime_shell, "party:mail:read")?;
            runtime_shell.pending_mail_read = Some(VisibleMailRead { mail: mail.clone() });
            runtime_shell.field_notice = None;
            mark_runtime_snapshot_dirty(runtime_shell);
        }
        1 => {
            record_visible_runtime_action(runtime_shell, "party:mail:take")?;
            runtime_shell.party_give_take_cursor = None;
            runtime_shell.party_mail_take_stage = Some(1);
            runtime_shell.yes_no_cursor = Some(MenuCursor {
                surface_id: "party:mail-send-pc".to_string(),
                option_index: 0,
            });
            runtime_shell.field_notice = Some("Send removed MAIL to your PC?".to_string());
            mark_runtime_snapshot_dirty(runtime_shell);
        }
        _ => {
            record_visible_runtime_action(runtime_shell, "party:mail:quit")?;
            runtime_shell.party_give_take_cursor = None;
            set_shell_action_status(runtime_shell, "POKEMON");
        }
    }
    Ok(())
}

fn resolve_visible_party_mail_take_prompt(
    runtime_shell: &mut BevyRuntimeShell,
    accepted: bool,
) -> Result<()> {
    let stage = runtime_shell
        .party_mail_take_stage
        .context("no party Mail prompt")?;
    let party_index = selected_party_index(runtime_shell)?;
    runtime_shell.field_notice = None;
    runtime_shell.field_notice_queue.clear();
    runtime_shell.yes_no_cursor = None;
    if stage == 1 && !accepted {
        runtime_shell.party_mail_take_stage = Some(2);
        runtime_shell.yes_no_cursor = Some(MenuCursor {
            surface_id: "party:mail-lose-message".to_string(),
            option_index: 0,
        });
        runtime_shell.field_notice =
            Some("The MAIL's message will be lost. Is that OK?".to_string());
        mark_runtime_snapshot_dirty(runtime_shell);
        return Ok(());
    }
    runtime_shell.party_mail_take_stage = None;
    if !accepted {
        return Ok(());
    }
    let outcome = if stage == 1 {
        runtime_shell.shell.send_party_mail_to_mailbox(party_index)
    } else {
        runtime_shell.shell.discard_party_mail_to_bag(party_index)
    };
    match outcome {
        Ok(_) if stage == 1 => {
            runtime_shell.field_notice = Some("The MAIL was sent to your PC.".to_string())
        }
        Ok(_) => {
            runtime_shell.field_notice = Some("The MAIL was taken from the Pokemon.".to_string())
        }
        Err(error) if stage == 1 && error.to_string().contains("mailbox is full") => {
            runtime_shell.field_notice = Some("Your PC's MAILBOX is full.".to_string())
        }
        Err(error) if error.to_string().contains("bag") => {
            runtime_shell.field_notice = Some("There's no space for the removed item.".to_string())
        }
        Err(error) => return Err(error),
    }
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(())
}

fn party_move_reorder_surface_id(party_index: usize) -> String {
    format!("party:reorder-moves:{party_index}")
}

fn open_visible_party_move_reorder(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let party_index = selected_party_index(runtime_shell)?;
    let slot = selected_party_slot_snapshot(&snapshot, runtime_shell.party_cursor)?;
    runtime_shell.party_action_cursor = None;
    runtime_shell.party_move_reorder_open = true;
    runtime_shell.party_move_reorder_origin = None;
    visible_cursor_index(
        &mut runtime_shell.party_move_cursor,
        &party_move_reorder_surface_id(party_index),
        slot.pokemon.moves.len(),
    );
    set_shell_action_status(runtime_shell, "MOVE WHERE?");
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn close_visible_party_move_reorder(runtime_shell: &mut BevyRuntimeShell) {
    runtime_shell.party_move_reorder_open = false;
    runtime_shell.party_move_reorder_origin = None;
    runtime_shell.party_move_cursor = None;
}

fn confirm_visible_party_move_reorder(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let party_index = selected_party_index(runtime_shell)?;
    let slot = selected_party_slot_snapshot(&snapshot, runtime_shell.party_cursor)?;
    let selected = strict_readonly_cursor_index(
        &runtime_shell.party_move_cursor,
        &party_move_reorder_surface_id(party_index),
        slot.pokemon.moves.len(),
    )
    .context("party move reorder requires a valid cursor")?;
    let Some(origin) = runtime_shell.party_move_reorder_origin else {
        runtime_shell.party_move_reorder_origin = Some(selected);
        set_shell_action_status(runtime_shell, "MOVE WHERE?");
        return Ok(());
    };
    runtime_shell.party_move_reorder_origin = None;
    if origin == selected {
        return Ok(());
    }
    let swapped = runtime_shell
        .shell
        .swap_party_pokemon_moves(party_index, origin, selected)?;
    record_visible_runtime_action(
        runtime_shell,
        format!("party:move_swap:{party_index}:{origin}:{selected}"),
    )?;
    runtime_shell.last_audio_events.push(format!(
        "swapped party moves {} and {} -> {}, {} checksum={:?}",
        origin,
        selected,
        swapped.first_move_after,
        swapped.second_move_after,
        swapped.state_checksum
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn move_visible_party_move_reorder_cursor(
    runtime_shell: &mut BevyRuntimeShell,
    delta: isize,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let party_index = selected_party_index(runtime_shell)?;
    let slot = selected_party_slot_snapshot(&snapshot, runtime_shell.party_cursor)?;
    move_visible_cursor_slot(
        &mut runtime_shell.party_move_cursor,
        party_move_reorder_surface_id(party_index),
        slot.pokemon.moves.len(),
        delta,
        &mut runtime_shell.last_audio_events,
    )
}

fn cycle_visible_party_move_reorder_pokemon(
    runtime_shell: &mut BevyRuntimeShell,
    delta: isize,
) -> Result<()> {
    if runtime_shell.party_move_reorder_origin.is_some() {
        return Ok(());
    }
    let snapshot = runtime_shell.shell.snapshot()?;
    let party_len = snapshot.party.slots.len();
    if party_len < 2 {
        return Ok(());
    }
    anyhow::ensure!(
        runtime_shell.party_cursor < party_len,
        "party cursor {} is outside {party_len} Pokemon rows",
        runtime_shell.party_cursor
    );
    let mut next = runtime_shell.party_cursor;
    for _ in 0..party_len {
        next = wrapped_index(next, party_len, delta);
        let pokemon = &snapshot.party.slots[next].pokemon;
        if !pokemon.is_egg && pokemon.species.id != "EGG" {
            runtime_shell.party_cursor = next;
            runtime_shell.party_move_cursor = None;
            visible_cursor_index(
                &mut runtime_shell.party_move_cursor,
                &party_move_reorder_surface_id(next),
                pokemon.moves.len(),
            );
            set_shell_action_status(runtime_shell, "MOVE WHERE?");
            return Ok(());
        }
    }
    Ok(())
}

fn party_switch_cursor_surface_id(source_party_index: usize) -> String {
    format!("party:switch:{source_party_index}")
}

fn open_visible_party_switch_target(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    if snapshot.party.slots.len() < 2 {
        record_visible_runtime_action(runtime_shell, "party:switch:no_target")?;
        runtime_shell
            .last_audio_events
            .push("party has no second Pokemon to switch with".to_string());
        set_shell_action_status(runtime_shell, "NO SWITCH TARGET");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    let source_index = selected_party_index(runtime_shell)?;
    let source_slot = runtime_shell.party_cursor;
    if source_slot >= snapshot.party.slots.len() {
        anyhow::bail!(
            "party switch source cursor {} is out of range for {} slots",
            source_slot,
            snapshot.party.slots.len()
        );
    }
    let target = if source_slot == 0 { 1 } else { 0 };
    runtime_shell.party_switch_cursor = Some(MenuCursor {
        surface_id: party_switch_cursor_surface_id(source_index),
        option_index: target,
    });
    runtime_shell.last_audio_events.push(format!(
        "opened party switch source={} target={}",
        source_index, target
    ));
    set_shell_action_status(runtime_shell, format!("SWITCH PARTY #{}", source_index));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn move_visible_party_switch_cursor(
    runtime_shell: &mut BevyRuntimeShell,
    delta: isize,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    if snapshot.party.slots.len() < 2 {
        runtime_shell
            .last_audio_events
            .push("party has no second Pokemon to switch with".to_string());
        set_shell_action_status(runtime_shell, "NO SWITCH TARGET");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    let source_index = selected_party_index(runtime_shell)?;
    move_visible_cursor_slot(
        &mut runtime_shell.party_switch_cursor,
        party_switch_cursor_surface_id(source_index),
        snapshot.party.slots.len(),
        delta,
        &mut runtime_shell.last_audio_events,
    )
}

fn confirm_visible_party_switch_target(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    if snapshot.party.slots.len() < 2 {
        record_visible_runtime_action(runtime_shell, "party:switch:confirm:no_target")?;
        runtime_shell
            .last_audio_events
            .push("party has no second Pokemon to switch with".to_string());
        set_shell_action_status(runtime_shell, "NO SWITCH TARGET");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    let source_index = selected_party_index(runtime_shell)?;
    let target_index = strict_readonly_cursor_index(
        &runtime_shell.party_switch_cursor,
        &party_switch_cursor_surface_id(source_index),
        snapshot.party.slots.len(),
    )
    .context("party switch target is not selected")?;
    let target_party_index = snapshot
        .party
        .slots
        .get(target_index)
        .map(|slot| slot.index)
        .context("party switch target is not in the party")?;
    if target_party_index == source_index {
        record_visible_runtime_action(
            runtime_shell,
            format!("party:switch:confirm:same:{source_index}"),
        )?;
        runtime_shell
            .last_audio_events
            .push("party switch target is the selected Pokemon".to_string());
        set_shell_action_status(runtime_shell, "ALREADY SELECTED");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    swap_visible_party_pokemon(runtime_shell, source_index, target_party_index)
}

fn visible_party_actions(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
) -> Result<Vec<PartyAction>> {
    // engine/menus/party_menu.asm and the TypeScript wMonSubmenuItems model
    // reserve eight real entries plus the -1 terminator.
    const MAX_SUBMENU_ACTIONS: usize = 8;
    let selected = selected_party_slot_snapshot(snapshot, runtime_shell.party_cursor)?;
    let learned = selected
        .pokemon
        .moves
        .iter()
        .map(|learned| learned.name.as_str())
        .collect::<Vec<_>>();
    let is_egg = selected.pokemon.is_egg || selected.pokemon.species.id == "EGG";
    let link_mode = snapshot.link_session.link_mode != 0;
    let mut actions = Vec::new();
    if !is_egg && !link_mode {
        let field_move_rules = runtime_shell.shell.field_move_rule_keys();
        for learned_move in learned {
            let direct_hp_transfer = match learned_move {
                "SOFTBOILED" => Some(PartyFieldMove::Softboiled),
                "MILK_DRINK" => Some(PartyFieldMove::MilkDrink),
                _ => None,
            };
            if let Some(field_move) = direct_hp_transfer.or_else(|| {
                PARTY_FIELD_MOVES.iter().copied().find(|field_move| {
                    field_move_rules.iter().any(|key| {
                        key.rule_id == party_field_move_rule_id(*field_move)
                            && key.move_id.as_deref() == Some(learned_move)
                    })
                })
            }) {
                actions.push(PartyAction::FieldMove(field_move));
                if actions.len() == MAX_SUBMENU_ACTIONS {
                    return Ok(actions);
                }
            }
        }
    }
    actions.push(PartyAction::Summary);
    if actions.len() < MAX_SUBMENU_ACTIONS {
        actions.push(PartyAction::Switch);
    }
    if actions.len() < MAX_SUBMENU_ACTIONS && !is_egg {
        actions.push(PartyAction::Move);
    }
    if actions.len() < MAX_SUBMENU_ACTIONS && !is_egg && !link_mode {
        actions.push(PartyAction::Item);
    }
    if actions.len() < MAX_SUBMENU_ACTIONS {
        actions.push(PartyAction::Cancel);
    }
    Ok(actions)
}

const PARTY_FIELD_MOVES: &[PartyFieldMove] = &[
    PartyFieldMove::Surf,
    PartyFieldMove::Cut,
    PartyFieldMove::Fly,
    PartyFieldMove::Strength,
    PartyFieldMove::Flash,
    PartyFieldMove::Waterfall,
    PartyFieldMove::Dig,
    PartyFieldMove::Teleport,
    PartyFieldMove::Headbutt,
    PartyFieldMove::Whirlpool,
    PartyFieldMove::RockSmash,
    PartyFieldMove::SweetScent,
    PartyFieldMove::Softboiled,
    PartyFieldMove::MilkDrink,
];

fn execute_visible_party_field_move(
    runtime_shell: &mut BevyRuntimeShell,
    field_move: PartyFieldMove,
) -> Result<()> {
    let result = match field_move {
        PartyFieldMove::Surf => use_visible_surf(runtime_shell),
        PartyFieldMove::Cut => use_visible_cut(runtime_shell),
        PartyFieldMove::Fly => open_visible_fly_destination_menu(runtime_shell),
        PartyFieldMove::Strength => use_visible_strength(runtime_shell),
        PartyFieldMove::Flash => use_visible_flash(runtime_shell),
        PartyFieldMove::Waterfall => use_visible_waterfall(runtime_shell),
        PartyFieldMove::Dig => use_visible_dig(runtime_shell),
        PartyFieldMove::Teleport => use_visible_teleport(runtime_shell),
        PartyFieldMove::Headbutt => use_visible_headbutt(runtime_shell, true),
        PartyFieldMove::Whirlpool => use_visible_whirlpool(runtime_shell),
        PartyFieldMove::RockSmash => use_visible_rock_smash(runtime_shell),
        PartyFieldMove::SweetScent => use_visible_sweet_scent_current_surface(runtime_shell),
        PartyFieldMove::Softboiled | PartyFieldMove::MilkDrink => {
            return open_visible_party_hp_transfer_target(runtime_shell, field_move);
        }
    };
    match result {
        Ok(()) => {
            if field_move != PartyFieldMove::Fly {
                runtime_shell.party_menu_open = false;
                runtime_shell.party_summary_open = false;
                runtime_shell.party_action_cursor = None;
                runtime_shell.party_move_cursor = None;
                runtime_shell.fly_cursor = None;
            }
            Ok(())
        }
        Err(error) if party_field_move_error_is_play_refusal(&error) => {
            record_visible_runtime_action(
                runtime_shell,
                format!(
                    "party:field_move_refused:{}",
                    party_field_move_rule_id(field_move)
                ),
            )?;
            runtime_shell.last_audio_events.push(format!(
                "{} refused: {}",
                party_field_move_label(field_move),
                error
            ));
            runtime_shell.field_notice = Some(match field_move {
                PartyFieldMove::Surf => "Can't use SURF here.".to_string(),
                PartyFieldMove::Cut => "There's nothing to CUT here.".to_string(),
                PartyFieldMove::SweetScent => "Looks like there's nothing here...".to_string(),
                _ => "Can't use that here.".to_string(),
            });
            mark_runtime_snapshot_dirty(runtime_shell);
            set_shell_action_status(
                runtime_shell,
                format!("{} CAN'T BE USED HERE", party_field_move_label(field_move)),
            );
            trim_event_log(&mut runtime_shell.last_audio_events);
            Ok(())
        }
        Err(error) => Err(error),
    }
}

fn open_visible_party_hp_transfer_target(
    runtime_shell: &mut BevyRuntimeShell,
    field_move: PartyFieldMove,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let source = selected_party_slot_snapshot(&snapshot, runtime_shell.party_cursor)?;
    let amount = source.pokemon.max_hp / 5;
    runtime_shell.party_action_cursor = None;
    if source.pokemon.hp < amount {
        runtime_shell.field_notice = Some("Not enough HP!".to_string());
        mark_runtime_snapshot_dirty(runtime_shell);
        return Ok(());
    }
    runtime_shell.party_hp_transfer_source = Some(source.index);
    runtime_shell.party_hp_transfer_move = Some(field_move);
    set_shell_action_status(runtime_shell, "USE ON WHICH POKEMON?");
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(())
}

fn confirm_visible_party_hp_transfer_target(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let source_index = runtime_shell
        .party_hp_transfer_source
        .context("party HP transfer target is open without a source")?;
    let target = selected_party_slot_snapshot(&snapshot, runtime_shell.party_cursor)?;
    let source_slot = snapshot
        .party
        .slots
        .iter()
        .find(|slot| slot.index == source_index)
        .with_context(|| format!("party HP transfer source {source_index} left the party"))?;
    let target_is_egg = target.pokemon.is_egg || target.pokemon.species.id == "EGG";
    let invalid_target = target.index == source_index
        || target_is_egg
        || target.pokemon.hp == 0
        || target.pokemon.hp >= target.pokemon.max_hp;
    if invalid_target {
        runtime_shell.field_notice = Some("That can't be used\non this POKéMON.".to_string());
        mark_runtime_snapshot_dirty(runtime_shell);
        return Ok(());
    }
    let transfer = runtime_shell
        .shell
        .transfer_party_pokemon_hp(source_index, target.index)?;
    let recovered = transfer
        .outcome
        .target_hp_after
        .saturating_sub(transfer.outcome.target_hp_before);
    let target_name = target.pokemon.nickname.clone();
    let source_cursor = snapshot
        .party
        .slots
        .iter()
        .position(|slot| slot.index == source_slot.index)
        .context("party HP transfer source cursor disappeared")?;
    runtime_shell.party_cursor = source_cursor;
    runtime_shell.party_hp_transfer_source = None;
    runtime_shell.party_hp_transfer_move = None;
    runtime_shell.field_notice = Some(format!("{target_name}\nrecovered {recovered}HP!"));
    set_shell_action_status(runtime_shell, format!("RECOVERED {recovered} HP"));
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(())
}

fn cancel_visible_party_hp_transfer_target(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    if let Some(source_index) = runtime_shell.party_hp_transfer_source.take() {
        let snapshot = runtime_shell.shell.snapshot()?;
        if let Some(source_cursor) = snapshot
            .party
            .slots
            .iter()
            .position(|slot| slot.index == source_index)
        {
            runtime_shell.party_cursor = source_cursor;
        }
    }
    runtime_shell.party_hp_transfer_move = None;
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(())
}

fn party_field_move_error_is_play_refusal(error: &anyhow::Error) -> bool {
    if error.chain().any(|cause| {
        let message = cause.to_string();
        message == "Sweet Scent requires grass or surfable water under the player"
            || message.starts_with("cannot use field escape item ")
            || message.starts_with("cannot use field story key item ")
            || message.starts_with("cannot use SURF field move onto occupied tile ")
            || message.starts_with("cannot use FLY field move in environment ")
            || message.starts_with("cannot use DIG field move in environment ")
            || message.starts_with("cannot use TELEPORT field move in environment ")
    }) {
        return true;
    }
    let Some(field_move_error) = error.downcast_ref::<FieldMoveError>() else {
        return false;
    };
    matches!(
        field_move_error,
        FieldMoveError::MissingBadge { .. }
            | FieldMoveError::InvalidMovementMode { .. }
            | FieldMoveError::AlwaysOnBike { .. }
            | FieldMoveError::InvalidFacing { .. }
            | FieldMoveError::TargetTileOutOfBounds { .. }
            | FieldMoveError::BlockedTarget { .. }
            | FieldMoveError::TargetNotWater { .. }
            | FieldMoveError::TargetNotWaterfall { .. }
            | FieldMoveError::TargetOutOfBounds { .. }
            | FieldMoveError::MissingRockSmashTarget { .. }
            | FieldMoveError::TargetNotSmashableRock { .. }
            | FieldMoveError::MissingSavedDigWarpMap { .. }
            | FieldMoveError::MissingSavedDigWarpIndex { .. }
            | FieldMoveError::MissingSavedDigWarp { .. }
    )
}

fn open_visible_fly_destination_menu(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let party_index = party_index_for_field_move_rule(
        &snapshot,
        &runtime_shell.shell,
        "fly",
        runtime_shell.party_cursor,
    )?;
    let destinations = active_fly_destinations(&snapshot, &runtime_shell.shell)?;
    if destinations.is_empty() {
        anyhow::bail!("active Fly destination catalog is empty");
    }
    visible_cursor_index(
        &mut runtime_shell.fly_cursor,
        "fly:destinations",
        destinations.len(),
    );
    let first_destination = fly_destination_label(&destinations[0]);
    runtime_shell.last_audio_events.push(format!(
        "opened Fly destinations party_index={} count={}",
        party_index,
        destinations.len()
    ));
    set_shell_action_status(
        runtime_shell,
        format!(
            "FLY DESTINATIONS {} FIRST {}",
            destinations.len(),
            first_destination
        ),
    );
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn confirm_visible_fly_destination(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let result = use_visible_fly(runtime_shell);
    if result.is_ok() {
        runtime_shell.party_menu_open = false;
        runtime_shell.party_summary_open = false;
        runtime_shell.party_action_cursor = None;
        runtime_shell.party_move_cursor = None;
        runtime_shell.fly_cursor = None;
    }
    result
}

fn selected_party_slot_snapshot(
    snapshot: &RuntimeShellSnapshot,
    party_cursor: usize,
) -> Result<&crate::RuntimePartySlotSnapshot> {
    if snapshot.party.slots.is_empty() {
        anyhow::bail!("party is empty");
    }
    snapshot.party.slots.get(party_cursor).with_context(|| {
        format!(
            "selected party cursor {party_cursor} is outside party length {}",
            snapshot.party.slots.len()
        )
    })
}

fn visible_party_action_labels(actions: &[PartyAction]) -> Vec<&'static str> {
    actions
        .iter()
        .map(|action| party_action_label(*action))
        .collect()
}

fn party_action_label(action: PartyAction) -> &'static str {
    match action {
        PartyAction::Summary => "STATS",
        PartyAction::Switch => "SWITCH",
        PartyAction::Move => "MOVE",
        PartyAction::Item => "ITEM",
        PartyAction::Cancel => "CANCEL",
        PartyAction::FieldMove(field_move) => party_field_move_label(field_move),
    }
}

fn party_action_record_id(action: PartyAction) -> &'static str {
    match action {
        PartyAction::Summary => "summary",
        PartyAction::Switch => "switch",
        PartyAction::Move => "move",
        PartyAction::Item => "item",
        PartyAction::Cancel => "cancel",
        PartyAction::FieldMove(field_move) => party_field_move_rule_id(field_move),
    }
}

fn party_field_move_rule_id(field_move: PartyFieldMove) -> &'static str {
    match field_move {
        PartyFieldMove::Surf => "surf",
        PartyFieldMove::Cut => "cut",
        PartyFieldMove::Fly => "fly",
        PartyFieldMove::Strength => "strength",
        PartyFieldMove::Flash => "flash",
        PartyFieldMove::Waterfall => "waterfall",
        PartyFieldMove::Dig => "dig",
        PartyFieldMove::Teleport => "teleport",
        PartyFieldMove::Headbutt => "headbutt",
        PartyFieldMove::Whirlpool => "whirlpool",
        PartyFieldMove::RockSmash => "rock_smash",
        PartyFieldMove::SweetScent => "sweet_scent",
        PartyFieldMove::Softboiled => "softboiled",
        PartyFieldMove::MilkDrink => "milk_drink",
    }
}

fn party_field_move_label(field_move: PartyFieldMove) -> &'static str {
    match field_move {
        PartyFieldMove::Surf => "Surf",
        PartyFieldMove::Cut => "Cut",
        PartyFieldMove::Fly => "Fly",
        PartyFieldMove::Strength => "Strength",
        PartyFieldMove::Flash => "Flash",
        PartyFieldMove::Waterfall => "Waterfall",
        PartyFieldMove::Dig => "Dig",
        PartyFieldMove::Teleport => "Teleport",
        PartyFieldMove::Headbutt => "Headbutt",
        PartyFieldMove::Whirlpool => "Whirlpool",
        PartyFieldMove::RockSmash => "Rock Smash",
        PartyFieldMove::SweetScent => "Sweet Scent",
        PartyFieldMove::Softboiled => "Softboiled",
        PartyFieldMove::MilkDrink => "Milk Drink",
    }
}

fn selected_party_species_label(
    snapshot: &RuntimeShellSnapshot,
    party_cursor: usize,
) -> Option<&str> {
    snapshot
        .party
        .slots
        .get(party_cursor)
        .map(|slot| slot.pokemon.species.id.as_str())
}

fn move_visible_primary_cursor_up(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    if !runtime_shell.battle_messages.is_empty() {
        return Ok(());
    }
    if runtime_shell.visible_slot_machine.is_some() {
        return change_visible_slot_machine_bet(runtime_shell, 1);
    }
    move_visible_primary_cursor(runtime_shell, -1)
}

fn move_visible_primary_cursor_down(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    if !runtime_shell.battle_messages.is_empty() {
        return Ok(());
    }
    if runtime_shell.visible_slot_machine.is_some() {
        return change_visible_slot_machine_bet(runtime_shell, -1);
    }
    move_visible_primary_cursor(runtime_shell, 1)
}

fn move_visible_primary_cursor_left(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    if !runtime_shell.battle_messages.is_empty() {
        return Ok(());
    }
    if runtime_shell.visible_buena_password.is_some()
        || runtime_shell.visible_battle_tower_challenge_menu.is_some()
        || runtime_shell.visible_battle_tower_room_menu.is_some()
    {
        return Ok(());
    }
    if runtime_shell.hall_of_fame_pc_index.is_some() {
        return move_visible_hall_of_fame_pc(runtime_shell, -1);
    }
    if runtime_shell.visible_unown_puzzle.is_some() {
        return move_visible_unown_puzzle_cursor(runtime_shell, -1, 0);
    }
    if runtime_shell.visible_unown_printer.is_some() {
        return move_visible_unown_printer(runtime_shell, -1);
    }
    if runtime_shell.visible_card_flip.is_some() {
        return move_visible_card_flip_cursor(runtime_shell, -1, 0);
    }
    if runtime_shell.visible_slot_machine.is_some() {
        return Ok(());
    }
    if runtime_shell.visible_mom_bank.is_some() {
        move_visible_mom_bank(runtime_shell, -1, true);
        return Ok(());
    }
    if runtime_shell.pc_item_quantity.is_some() {
        return adjust_visible_pc_item_quantity(runtime_shell, -1);
    }
    if runtime_shell.kurt_apricorn_cursor.is_some() {
        if runtime_shell.kurt_apricorn_quantity.is_some() {
            return adjust_visible_kurt_apricorn_quantity(runtime_shell, -10);
        }
        return move_visible_primary_cursor(runtime_shell, -1);
    }
    if runtime_shell.options_menu_open {
        return change_visible_options_selection(runtime_shell, -1);
    }
    if runtime_shell.trainer_card_open {
        return return_visible_trainer_card_left(runtime_shell);
    }
    if runtime_shell.pending_day_of_week.is_some() {
        return Ok(());
    }
    if runtime_shell.pending_delete_save.is_some() {
        return move_visible_delete_save_cursor(runtime_shell);
    }
    if runtime_shell.pending_clock_reset.is_some() {
        return move_visible_clock_reset_cursor(runtime_shell, -1);
    }
    if runtime_shell.title_menu.is_some() {
        return move_visible_title_menu_cursor(runtime_shell, -1);
    }
    if runtime_shell.pending_time_set.is_some() {
        return move_visible_time_set_direction(runtime_shell, VisibleTimeSetDirection::Left);
    }
    if runtime_shell.pending_gender_selection.is_some() {
        return Ok(());
    }
    if runtime_shell.save_menu_open {
        return move_visible_save_prompt_cursor(runtime_shell, -1);
    }
    let snapshot = runtime_shell.shell.snapshot()?;
    if (runtime_shell.field_notice.is_some()
        && !runtime_shell.held_item_swap_prompt
        && runtime_shell.pending_contextual_field_move.is_none()
        && runtime_shell.party_mail_take_stage.is_none())
        || (runtime_shell.pc_notice.is_some() && runtime_shell.pc_confirmation.is_none())
    {
        return Ok(());
    }
    if runtime_shell.pack_toss.is_some() {
        return adjust_visible_pack_toss_quantity(runtime_shell, -10);
    }
    if runtime_shell.pc_confirmation.is_some() {
        return move_visible_cursor_slot(
            &mut runtime_shell.yes_no_cursor,
            "pc:confirmation".to_string(),
            2,
            -1,
            &mut runtime_shell.last_audio_events,
        );
    }
    if let Some(stage) = runtime_shell.party_mail_take_stage {
        return move_visible_cursor_slot(
            &mut runtime_shell.yes_no_cursor,
            if stage == 1 {
                "party:mail-send-pc"
            } else {
                "party:mail-lose-message"
            }
            .to_string(),
            2,
            -1,
            &mut runtime_shell.last_audio_events,
        );
    }
    if runtime_shell.pending_contextual_field_move.is_some() {
        return move_visible_cursor_slot(
            &mut runtime_shell.yes_no_cursor,
            "field:move-confirm".to_string(),
            2,
            -1,
            &mut runtime_shell.last_audio_events,
        );
    }
    if runtime_shell.held_item_swap_prompt {
        return move_visible_cursor_slot(
            &mut runtime_shell.yes_no_cursor,
            "party:held-item-swap".to_string(),
            2,
            -1,
            &mut runtime_shell.last_audio_events,
        );
    }
    if runtime_shell
        .party_give_take_cursor
        .as_ref()
        .is_some_and(|cursor| cursor.surface_id == "party:mail-actions")
    {
        return move_visible_cursor_slot(
            &mut runtime_shell.party_give_take_cursor,
            "party:mail-actions".to_string(),
            3,
            -1,
            &mut runtime_shell.last_audio_events,
        );
    }
    if runtime_shell.tmhm_teach_prompt_cursor.is_some() {
        return move_visible_cursor_slot(
            &mut runtime_shell.tmhm_teach_prompt_cursor,
            "pack:tmhm:teach-prompt".to_string(),
            2,
            -1,
            &mut runtime_shell.last_audio_events,
        );
    }
    if runtime_shell.tmhm_decision_prompt_cursor.is_some() {
        return move_visible_cursor_slot(
            &mut runtime_shell.tmhm_decision_prompt_cursor,
            "pack:tmhm:decision".to_string(),
            2,
            -1,
            &mut runtime_shell.last_audio_events,
        );
    }
    if runtime_shell.tmhm_forget_menu_open {
        return Ok(());
    }
    if runtime_shell.party_menu_open && runtime_shell.party_move_reorder_open {
        return cycle_visible_party_move_reorder_pokemon(runtime_shell, -1);
    }
    if runtime_shell.party_menu_open && runtime_shell.party_summary_open {
        return cycle_visible_party_summary_page(runtime_shell, -1);
    }
    if snapshot.battle.is_some() {
        if runtime_shell.battle_party_summary_open {
            return cycle_visible_party_summary_page(runtime_shell, -1);
        }
        if runtime_shell.battle_shift_prompt_cursor.is_some()
            || runtime_shell.battle_faint_prompt_cursor.is_some()
        {
            return Ok(());
        }
        if runtime_shell.field_pack_action_cursor.is_some() {
            return move_visible_field_pack_action_cursor(runtime_shell, -1);
        }
        if runtime_shell.battle_pack_target_mode.is_some() {
            return move_visible_battle_pack_target_secondary_cursor(runtime_shell, -1);
        }
        if runtime_shell.ball_cursor.is_some()
            || runtime_shell.bag_cursor.is_some()
            || runtime_shell.key_item_cursor.is_some()
            || runtime_shell.tmhm_cursor.is_some()
        {
            return shift_visible_battle_pack_pocket(runtime_shell, -1);
        }
        if runtime_shell.battle_switch_cursor.is_some() {
            return Ok(());
        }
        if runtime_shell.battle_move_cursor.is_some() {
            return Ok(());
        }
        return move_visible_battle_action_cursor_axis(
            runtime_shell,
            BattleMenuAxis::Horizontal,
            -1,
        );
    }
    if snapshot.pending_shop.is_some() {
        if !runtime_shell.shop_welcome_seen {
            return Ok(());
        }
        if runtime_shell.shop_notice.is_some() {
            return Ok(());
        }
        if runtime_shell.shop_quantity.is_some() {
            return adjust_visible_shop_quantity(runtime_shell, -10);
        }
        return Ok(());
    }
    if runtime_shell.field_notice.is_some() {
        return Ok(());
    }
    if visible_field_pack_is_open(runtime_shell) {
        if runtime_shell.pack_item_switch_origin.is_some() {
            return Ok(());
        }
        if runtime_shell.field_pack_target_mode.is_some() {
            return move_visible_field_pack_target_secondary_cursor(runtime_shell, -1);
        }
        if runtime_shell.field_pack_action_cursor.is_some() {
            return move_visible_field_pack_action_cursor(runtime_shell, -1);
        }
        return shift_visible_field_pack_pocket(runtime_shell, -1);
    }
    if runtime_shell.pokegear_menu_open {
        if runtime_shell.pokegear_standalone_map {
            return Ok(());
        }
        return cycle_visible_pokegear_page(runtime_shell, -1);
    }
    if runtime_shell.pokedex_menu_open {
        return page_visible_pokedex_cursor(runtime_shell, -1);
    }
    if runtime_shell.storage_cursor.is_some() {
        return switch_visible_pc_box_by_delta(runtime_shell, -1);
    }
    if runtime_shell.pc_item_cursor.is_some() {
        return move_visible_pc_item_cursor(runtime_shell, -1);
    }
    if runtime_shell.player_pc_action_cursor.is_some() {
        let option_count = visible_player_pc_actions(runtime_shell).len();
        return move_visible_cursor_slot(
            &mut runtime_shell.player_pc_action_cursor,
            "pc:player-actions".to_string(),
            option_count,
            -1,
            &mut runtime_shell.last_audio_events,
        );
    }
    if runtime_shell.mailbox_action_cursor.is_some() {
        return move_visible_cursor_slot(
            &mut runtime_shell.mailbox_action_cursor,
            "pc:mailbox-actions".to_string(),
            VISIBLE_MAILBOX_ACTIONS.len(),
            -1,
            &mut runtime_shell.last_audio_events,
        );
    }
    if runtime_shell.mailbox_cursor.is_some() {
        let count = snapshot.mailbox.len();
        return move_visible_cursor_slot(
            &mut runtime_shell.mailbox_cursor,
            "pc:mailbox".to_string(),
            count,
            -1,
            &mut runtime_shell.last_audio_events,
        );
    }
    if visible_menu_has_selectable_options(&snapshot) {
        return move_visible_menu_cursor_horizontal(runtime_shell, -1);
    }
    move_visible_primary_cursor(runtime_shell, -1)
}

fn move_visible_primary_cursor_right(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    if !runtime_shell.battle_messages.is_empty() {
        return Ok(());
    }
    if runtime_shell.visible_buena_password.is_some()
        || runtime_shell.visible_battle_tower_challenge_menu.is_some()
        || runtime_shell.visible_battle_tower_room_menu.is_some()
    {
        return Ok(());
    }
    if runtime_shell.hall_of_fame_pc_index.is_some() {
        return move_visible_hall_of_fame_pc(runtime_shell, 1);
    }
    if runtime_shell.visible_unown_puzzle.is_some() {
        return move_visible_unown_puzzle_cursor(runtime_shell, 1, 0);
    }
    if runtime_shell.visible_unown_printer.is_some() {
        return move_visible_unown_printer(runtime_shell, 1);
    }
    if runtime_shell.visible_card_flip.is_some() {
        return move_visible_card_flip_cursor(runtime_shell, 1, 0);
    }
    if runtime_shell.visible_slot_machine.is_some() {
        return Ok(());
    }
    if runtime_shell.visible_mom_bank.is_some() {
        move_visible_mom_bank(runtime_shell, 1, true);
        return Ok(());
    }
    if runtime_shell.pc_item_quantity.is_some() {
        return adjust_visible_pc_item_quantity(runtime_shell, 1);
    }
    if runtime_shell.kurt_apricorn_cursor.is_some() {
        if runtime_shell.kurt_apricorn_quantity.is_some() {
            return adjust_visible_kurt_apricorn_quantity(runtime_shell, 10);
        }
        return move_visible_primary_cursor(runtime_shell, 1);
    }
    if runtime_shell.options_menu_open {
        return change_visible_options_selection(runtime_shell, 1);
    }
    if runtime_shell.trainer_card_open {
        return advance_visible_trainer_card_right(runtime_shell);
    }
    if runtime_shell.pending_day_of_week.is_some() {
        return Ok(());
    }
    if runtime_shell.pending_delete_save.is_some() {
        return move_visible_delete_save_cursor(runtime_shell);
    }
    if runtime_shell.pending_clock_reset.is_some() {
        return move_visible_clock_reset_cursor(runtime_shell, 1);
    }
    if runtime_shell.title_menu.is_some() {
        return move_visible_title_menu_cursor(runtime_shell, 1);
    }
    if runtime_shell.pending_time_set.is_some() {
        return move_visible_time_set_direction(runtime_shell, VisibleTimeSetDirection::Right);
    }
    if runtime_shell.pending_gender_selection.is_some() {
        return Ok(());
    }
    if runtime_shell.save_menu_open {
        return move_visible_save_prompt_cursor(runtime_shell, 1);
    }
    let snapshot = runtime_shell.shell.snapshot()?;
    if (runtime_shell.field_notice.is_some()
        && !runtime_shell.held_item_swap_prompt
        && runtime_shell.pending_contextual_field_move.is_none()
        && runtime_shell.party_mail_take_stage.is_none())
        || (runtime_shell.pc_notice.is_some() && runtime_shell.pc_confirmation.is_none())
    {
        return Ok(());
    }
    if runtime_shell.pack_toss.is_some() {
        return adjust_visible_pack_toss_quantity(runtime_shell, 10);
    }
    if runtime_shell.pc_confirmation.is_some() {
        return move_visible_cursor_slot(
            &mut runtime_shell.yes_no_cursor,
            "pc:confirmation".to_string(),
            2,
            1,
            &mut runtime_shell.last_audio_events,
        );
    }
    if let Some(stage) = runtime_shell.party_mail_take_stage {
        return move_visible_cursor_slot(
            &mut runtime_shell.yes_no_cursor,
            if stage == 1 {
                "party:mail-send-pc"
            } else {
                "party:mail-lose-message"
            }
            .to_string(),
            2,
            1,
            &mut runtime_shell.last_audio_events,
        );
    }
    if runtime_shell.pending_contextual_field_move.is_some() {
        return move_visible_cursor_slot(
            &mut runtime_shell.yes_no_cursor,
            "field:move-confirm".to_string(),
            2,
            1,
            &mut runtime_shell.last_audio_events,
        );
    }
    if runtime_shell.held_item_swap_prompt {
        return move_visible_cursor_slot(
            &mut runtime_shell.yes_no_cursor,
            "party:held-item-swap".to_string(),
            2,
            1,
            &mut runtime_shell.last_audio_events,
        );
    }
    if runtime_shell
        .party_give_take_cursor
        .as_ref()
        .is_some_and(|cursor| cursor.surface_id == "party:mail-actions")
    {
        return move_visible_cursor_slot(
            &mut runtime_shell.party_give_take_cursor,
            "party:mail-actions".to_string(),
            3,
            1,
            &mut runtime_shell.last_audio_events,
        );
    }
    if runtime_shell.tmhm_teach_prompt_cursor.is_some() {
        return move_visible_cursor_slot(
            &mut runtime_shell.tmhm_teach_prompt_cursor,
            "pack:tmhm:teach-prompt".to_string(),
            2,
            1,
            &mut runtime_shell.last_audio_events,
        );
    }
    if runtime_shell.tmhm_decision_prompt_cursor.is_some() {
        return move_visible_cursor_slot(
            &mut runtime_shell.tmhm_decision_prompt_cursor,
            "pack:tmhm:decision".to_string(),
            2,
            1,
            &mut runtime_shell.last_audio_events,
        );
    }
    if runtime_shell.tmhm_forget_menu_open {
        return Ok(());
    }
    if runtime_shell.party_menu_open && runtime_shell.party_move_reorder_open {
        return cycle_visible_party_move_reorder_pokemon(runtime_shell, 1);
    }
    if runtime_shell.party_menu_open && runtime_shell.party_summary_open {
        return cycle_visible_party_summary_page(runtime_shell, 1);
    }
    if snapshot.battle.is_some() {
        if runtime_shell.battle_party_summary_open {
            return cycle_visible_party_summary_page(runtime_shell, 1);
        }
        if runtime_shell.battle_shift_prompt_cursor.is_some()
            || runtime_shell.battle_faint_prompt_cursor.is_some()
        {
            return Ok(());
        }
        if runtime_shell.field_pack_action_cursor.is_some() {
            return move_visible_field_pack_action_cursor(runtime_shell, 1);
        }
        if runtime_shell.battle_pack_target_mode.is_some() {
            return move_visible_battle_pack_target_secondary_cursor(runtime_shell, 1);
        }
        if runtime_shell.ball_cursor.is_some()
            || runtime_shell.bag_cursor.is_some()
            || runtime_shell.key_item_cursor.is_some()
            || runtime_shell.tmhm_cursor.is_some()
        {
            return shift_visible_battle_pack_pocket(runtime_shell, 1);
        }
        if runtime_shell.battle_switch_cursor.is_some() {
            return Ok(());
        }
        if runtime_shell.battle_move_cursor.is_some() {
            return Ok(());
        }
        return move_visible_battle_action_cursor_axis(
            runtime_shell,
            BattleMenuAxis::Horizontal,
            1,
        );
    }
    if snapshot.pending_shop.is_some() {
        if !runtime_shell.shop_welcome_seen {
            return Ok(());
        }
        if runtime_shell.shop_notice.is_some() {
            return Ok(());
        }
        if runtime_shell.shop_quantity.is_some() {
            return adjust_visible_shop_quantity(runtime_shell, 10);
        }
        return Ok(());
    }
    if runtime_shell.field_notice.is_some() {
        return Ok(());
    }
    if visible_field_pack_is_open(runtime_shell) {
        if runtime_shell.pack_item_switch_origin.is_some() {
            return Ok(());
        }
        if runtime_shell.field_pack_target_mode.is_some() {
            return move_visible_field_pack_target_secondary_cursor(runtime_shell, 1);
        }
        if runtime_shell.field_pack_action_cursor.is_some() {
            return move_visible_field_pack_action_cursor(runtime_shell, 1);
        }
        return shift_visible_field_pack_pocket(runtime_shell, 1);
    }
    if runtime_shell.pokegear_menu_open {
        if runtime_shell.pokegear_standalone_map {
            return Ok(());
        }
        return cycle_visible_pokegear_page(runtime_shell, 1);
    }
    if runtime_shell.pokedex_menu_open {
        return page_visible_pokedex_cursor(runtime_shell, 1);
    }
    if runtime_shell.storage_cursor.is_some() {
        return switch_visible_pc_box_by_delta(runtime_shell, 1);
    }
    if runtime_shell.pc_item_cursor.is_some() {
        return move_visible_pc_item_cursor(runtime_shell, 1);
    }
    if runtime_shell.player_pc_action_cursor.is_some() {
        let option_count = visible_player_pc_actions(runtime_shell).len();
        return move_visible_cursor_slot(
            &mut runtime_shell.player_pc_action_cursor,
            "pc:player-actions".to_string(),
            option_count,
            1,
            &mut runtime_shell.last_audio_events,
        );
    }
    if runtime_shell.mailbox_action_cursor.is_some() {
        return move_visible_cursor_slot(
            &mut runtime_shell.mailbox_action_cursor,
            "pc:mailbox-actions".to_string(),
            VISIBLE_MAILBOX_ACTIONS.len(),
            1,
            &mut runtime_shell.last_audio_events,
        );
    }
    if runtime_shell.mailbox_cursor.is_some() {
        let count = snapshot.mailbox.len();
        return move_visible_cursor_slot(
            &mut runtime_shell.mailbox_cursor,
            "pc:mailbox".to_string(),
            count,
            1,
            &mut runtime_shell.last_audio_events,
        );
    }
    if visible_menu_has_selectable_options(&snapshot) {
        return move_visible_menu_cursor_horizontal(runtime_shell, 1);
    }
    move_visible_primary_cursor(runtime_shell, 1)
}

fn move_visible_battle_item_list_cursor(
    runtime_shell: &mut BevyRuntimeShell,
    delta: isize,
) -> Result<()> {
    if runtime_shell.ball_cursor.is_some() {
        return move_visible_ball_cursor(runtime_shell, delta);
    }
    if runtime_shell.bag_cursor.is_some() {
        return move_visible_battle_bag_cursor(runtime_shell, delta);
    }
    if runtime_shell.key_item_cursor.is_some() {
        return move_visible_key_item_cursor(runtime_shell, delta);
    }
    if runtime_shell.tmhm_cursor.is_some() {
        return move_visible_tmhm_cursor(runtime_shell, delta);
    }
    record_visible_runtime_action(runtime_shell, "battle:item_list:move:none")?;
    runtime_shell
        .last_audio_events
        .push("no open battle item list".to_string());
    set_shell_action_status(runtime_shell, "NO BATTLE ITEMS");
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn shift_visible_battle_pack_pocket(
    runtime_shell: &mut BevyRuntimeShell,
    delta: isize,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let item_ids = carried_battle_non_ball_item_ids(&snapshot);
    let ball_ids = carried_ball_item_ids(&snapshot);
    let key_count = carried_item_count(&snapshot.bag.key_items);
    let tmhm_count = snapshot
        .bag
        .tm_hm
        .iter()
        .filter(|item| item.quantity > 0)
        .count();
    let current = if runtime_shell.ball_cursor.is_some() {
        FieldPackPocket::Balls
    } else if runtime_shell.key_item_cursor.is_some() {
        FieldPackPocket::KeyItems
    } else if runtime_shell.tmhm_cursor.is_some() {
        FieldPackPocket::TmHm
    } else {
        FieldPackPocket::Items
    };
    runtime_shell.bag_cursor = None;
    runtime_shell.ball_cursor = None;
    runtime_shell.key_item_cursor = None;
    runtime_shell.tmhm_cursor = None;
    let current_index = FIELD_PACK_POCKETS
        .iter()
        .position(|pocket| *pocket == current)
        .context("battle Pack has a nonstandard active pocket")?;
    let next_index = if delta.is_negative() {
        current_index
            .checked_sub(delta.unsigned_abs())
            .unwrap_or(FIELD_PACK_POCKETS.len() - 1)
    } else {
        (current_index + delta as usize) % FIELD_PACK_POCKETS.len()
    };
    let next = match FIELD_PACK_POCKETS[next_index].clone() {
        FieldPackPocket::Items => {
            runtime_shell.bag_cursor = Some(MenuCursor {
                surface_id: "battle:bag-items".to_string(),
                option_index: runtime_shell.field_pack_cursor_positions[0],
            });
            visible_cursor_index(
                &mut runtime_shell.bag_cursor,
                "battle:bag-items",
                field_pack_selectable_count(item_ids.len()),
            );
            runtime_shell.field_pack_cursor_positions[0] = runtime_shell
                .bag_cursor
                .as_ref()
                .map_or(0, |cursor| cursor.option_index);
            FieldPackPocket::Items
        }
        FieldPackPocket::Balls => {
            runtime_shell.ball_cursor = Some(MenuCursor {
                surface_id: "bag:balls".to_string(),
                option_index: runtime_shell.field_pack_cursor_positions[1],
            });
            visible_cursor_index(
                &mut runtime_shell.ball_cursor,
                "bag:balls",
                field_pack_selectable_count(ball_ids.len()),
            );
            runtime_shell.field_pack_cursor_positions[1] = runtime_shell
                .ball_cursor
                .as_ref()
                .map_or(0, |cursor| cursor.option_index);
            FieldPackPocket::Balls
        }
        FieldPackPocket::KeyItems => {
            runtime_shell.key_item_cursor = Some(MenuCursor {
                surface_id: "bag:key-items".to_string(),
                option_index: runtime_shell.field_pack_cursor_positions[2],
            });
            visible_cursor_index(
                &mut runtime_shell.key_item_cursor,
                "bag:key-items",
                field_pack_selectable_count(key_count),
            );
            runtime_shell.field_pack_cursor_positions[2] = runtime_shell
                .key_item_cursor
                .as_ref()
                .map_or(0, |cursor| cursor.option_index);
            FieldPackPocket::KeyItems
        }
        FieldPackPocket::TmHm => {
            runtime_shell.tmhm_cursor = Some(MenuCursor {
                surface_id: "bag:tmhm".to_string(),
                option_index: runtime_shell.field_pack_cursor_positions[3],
            });
            visible_cursor_index(
                &mut runtime_shell.tmhm_cursor,
                "bag:tmhm",
                field_pack_selectable_count(tmhm_count),
            );
            runtime_shell.field_pack_cursor_positions[3] = runtime_shell
                .tmhm_cursor
                .as_ref()
                .map_or(0, |cursor| cursor.option_index);
            FieldPackPocket::TmHm
        }
        FieldPackPocket::Custom(_) => anyhow::bail!("battle Pack cannot open a custom pocket"),
    };
    runtime_shell.field_pack_pocket = None;
    runtime_shell.last_field_pack_pocket = next.clone();
    set_shell_action_status(
        runtime_shell,
        format!("PACK {}", field_pack_pocket_label(&next)),
    );
    runtime_shell
        .last_audio_events
        .push("shifted battle Pack pocket".to_string());
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn move_visible_primary_cursor(runtime_shell: &mut BevyRuntimeShell, delta: isize) -> Result<()> {
    if runtime_shell.bill_pc_move_save.is_some()
        || runtime_shell.pc_release_sequence.is_some()
        || runtime_shell.pc_transfer_sequence.is_some()
    {
        return Ok(());
    }
    if runtime_shell.hall_of_fame_pc_index.is_some() {
        return move_visible_hall_of_fame_pc(runtime_shell, delta);
    }
    if runtime_shell.visible_card_flip.is_some() {
        return move_visible_card_flip_cursor(runtime_shell, 0, delta);
    }
    if runtime_shell.visible_unown_puzzle.is_some() {
        return move_visible_unown_puzzle_cursor(runtime_shell, 0, delta);
    }
    if runtime_shell.visible_unown_printer.is_some() {
        return Ok(());
    }
    if runtime_shell.visible_slot_machine.is_some() {
        return Ok(());
    }
    if runtime_shell.visible_mom_bank.is_some() {
        move_visible_mom_bank(runtime_shell, delta, false);
        return Ok(());
    }
    if runtime_shell.intro_screen.is_some() {
        record_visible_runtime_action(runtime_shell, format!("intro:cursor:{delta}:ignored"))?;
        runtime_shell
            .last_audio_events
            .push("intro cursor ignored".to_string());
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    if runtime_shell.pc_item_quantity.is_some() {
        return adjust_visible_pc_item_quantity(runtime_shell, if delta < 0 { 1 } else { -1 });
    }
    if runtime_shell.options_menu_open {
        return move_visible_options_cursor(runtime_shell, delta);
    }
    if runtime_shell.kurt_apricorn_cursor.is_some() {
        if runtime_shell.kurt_apricorn_quantity.is_some() {
            return adjust_visible_kurt_apricorn_quantity(
                runtime_shell,
                if delta < 0 { 1 } else { -1 },
            );
        }
        let snapshot = runtime_shell.shell.snapshot()?;
        let option_count = visible_kurt_apricorn_choices(&snapshot).len();
        return move_visible_cursor_slot(
            &mut runtime_shell.kurt_apricorn_cursor,
            "script:kurt-apricorn".to_string(),
            option_count,
            delta,
            &mut runtime_shell.last_audio_events,
        );
    }
    if runtime_shell.buena_prize_cursor.is_some()
        && runtime_shell.pc_confirmation.is_none()
        && runtime_shell.pc_notice.is_none()
    {
        let snapshot = runtime_shell.shell.snapshot()?;
        let option_count = visible_buena_prize_choices(&snapshot)?.len();
        return move_visible_cursor_slot(
            &mut runtime_shell.buena_prize_cursor,
            "script:buena-prize".to_string(),
            option_count,
            delta,
            &mut runtime_shell.last_audio_events,
        );
    }
    if let Some(menu) = runtime_shell.visible_buena_password.as_mut() {
        let mut cursor = Some(menu.cursor.clone());
        move_visible_cursor_slot(
            &mut cursor,
            "script:buena-password".to_string(),
            menu.options.len(),
            delta,
            &mut runtime_shell.last_audio_events,
        )?;
        menu.cursor = cursor.context("Buena password cursor disappeared")?;
        mark_runtime_presentation_dirty(runtime_shell);
        return Ok(());
    }
    if let Some(menu) = runtime_shell.visible_battle_tower_challenge_menu.as_mut() {
        let option_count = if menu.english { 3 } else { 4 };
        let mut cursor = Some(menu.cursor.clone());
        move_visible_cursor_slot(
            &mut cursor,
            "script:battle-tower-challenge".to_string(),
            option_count,
            delta,
            &mut runtime_shell.last_audio_events,
        )?;
        menu.cursor = cursor.context("Battle Tower challenge cursor disappeared")?;
        mark_runtime_presentation_dirty(runtime_shell);
        return Ok(());
    }
    if let Some(menu) = runtime_shell.visible_battle_tower_room_menu.as_mut() {
        match &mut menu.phase {
            VisibleBattleTowerRoomMenuPhase::PickLevel => {
                let mut cursor = Some(menu.cursor.clone());
                move_visible_cursor_slot(
                    &mut cursor,
                    "script:battle-tower-room".to_string(),
                    menu.level_groups.len() + 1,
                    -delta,
                    &mut runtime_shell.last_audio_events,
                )?;
                menu.cursor = cursor.context("Battle Tower room cursor disappeared")?;
            }
            VisibleBattleTowerRoomMenuPhase::ConfirmCancel { yes_no_index } => {
                *yes_no_index = if *yes_no_index == 0 { 1 } else { 0 };
            }
            VisibleBattleTowerRoomMenuPhase::Rejection { .. } => {}
        }
        mark_runtime_presentation_dirty(runtime_shell);
        return Ok(());
    }
    if let Some(prompt) = runtime_shell.pending_day_of_week.as_mut() {
        if prompt.confirming {
            prompt.yes_no_index = if prompt.yes_no_index == 0 { 1 } else { 0 };
        } else if delta < 0 {
            prompt.selected_day = (prompt.selected_day + 1) % 7;
        } else {
            prompt.selected_day = (prompt.selected_day + 6) % 7;
        }
        mark_runtime_snapshot_dirty(runtime_shell);
        return Ok(());
    }
    if runtime_shell.pending_delete_save.is_some() {
        return move_visible_delete_save_cursor(runtime_shell);
    }
    if runtime_shell.pending_clock_reset.is_some() {
        return move_visible_clock_reset_cursor(runtime_shell, if delta < 0 { 1 } else { -1 });
    }
    if runtime_shell.title_menu.is_some() {
        return move_visible_title_menu_cursor(runtime_shell, delta);
    }
    if runtime_shell.pending_time_set.is_some() {
        return move_visible_time_set_cursor(runtime_shell, delta);
    }
    if runtime_shell.pending_gender_selection.is_some() {
        return move_visible_gender_selection(runtime_shell, delta);
    }
    let snapshot = runtime_shell.shell.snapshot()?;
    if (runtime_shell.field_notice.is_some()
        && !runtime_shell.held_item_swap_prompt
        && runtime_shell.pending_contextual_field_move.is_none()
        && runtime_shell.party_mail_take_stage.is_none())
        || (runtime_shell.pc_notice.is_some() && runtime_shell.pc_confirmation.is_none())
    {
        return Ok(());
    }
    if runtime_shell.pack_toss.is_some() {
        return adjust_visible_pack_toss_quantity(runtime_shell, if delta < 0 { 1 } else { -1 });
    }
    if runtime_shell.pc_confirmation.is_some() {
        return move_visible_cursor_slot(
            &mut runtime_shell.yes_no_cursor,
            "pc:confirmation".to_string(),
            2,
            delta,
            &mut runtime_shell.last_audio_events,
        );
    }
    if let Some(stage) = runtime_shell.party_mail_take_stage {
        return move_visible_cursor_slot(
            &mut runtime_shell.yes_no_cursor,
            if stage == 1 {
                "party:mail-send-pc"
            } else {
                "party:mail-lose-message"
            }
            .to_string(),
            2,
            delta,
            &mut runtime_shell.last_audio_events,
        );
    }
    if runtime_shell.pending_contextual_field_move.is_some() {
        return move_visible_cursor_slot(
            &mut runtime_shell.yes_no_cursor,
            "field:move-confirm".to_string(),
            2,
            delta,
            &mut runtime_shell.last_audio_events,
        );
    }
    if runtime_shell.held_item_swap_prompt {
        return move_visible_cursor_slot(
            &mut runtime_shell.yes_no_cursor,
            "party:held-item-swap".to_string(),
            2,
            delta,
            &mut runtime_shell.last_audio_events,
        );
    }
    if runtime_shell
        .party_give_take_cursor
        .as_ref()
        .is_some_and(|cursor| cursor.surface_id == "party:mail-actions")
    {
        return move_visible_cursor_slot(
            &mut runtime_shell.party_give_take_cursor,
            "party:mail-actions".to_string(),
            3,
            delta,
            &mut runtime_shell.last_audio_events,
        );
    }
    if runtime_shell.start_menu_cursor.is_some() {
        return move_visible_start_menu_cursor(runtime_shell, delta);
    }
    if runtime_shell.save_menu_open {
        return move_visible_save_prompt_cursor(runtime_shell, delta);
    }
    if runtime_shell.pending_phone_prompt.is_some() {
        return move_visible_phone_prompt_cursor(runtime_shell, delta);
    }
    if runtime_shell.pending_remember_password.is_some() {
        if runtime_shell
            .pending_remember_password
            .as_ref()
            .is_some_and(|prompt| prompt.closing_frames.is_some())
        {
            return Ok(());
        }
        return move_visible_cursor_slot(
            &mut runtime_shell.yes_no_cursor,
            "script:remember-password".to_string(),
            2,
            delta,
            &mut runtime_shell.last_audio_events,
        );
    }
    if snapshot.ui.pending_yes_no.is_some() {
        // `yesorno` is an interpreter boundary, not permission to expose its
        // cursor before every page written ahead of it has been read.  During
        // that interval Crystal's text engine owns the joypad; moving this
        // retained cursor would let an invisible choice change underneath the
        // player's dialogue.
        if !visible_field_dialogue_is_entirely_consumed(runtime_shell, &snapshot) {
            return Ok(());
        }
        return move_visible_yes_no_cursor(runtime_shell, delta);
    }
    if snapshot.pending_move_learn.is_some() {
        return move_visible_pending_move_learn_cursor(runtime_shell, delta);
    }
    if runtime_shell.tmhm_teach_prompt_cursor.is_some() {
        return move_visible_cursor_slot(
            &mut runtime_shell.tmhm_teach_prompt_cursor,
            "pack:tmhm:teach-prompt".to_string(),
            2,
            delta,
            &mut runtime_shell.last_audio_events,
        );
    }
    if runtime_shell.tmhm_decision_prompt_cursor.is_some() {
        return move_visible_cursor_slot(
            &mut runtime_shell.tmhm_decision_prompt_cursor,
            "pack:tmhm:decision".to_string(),
            2,
            delta,
            &mut runtime_shell.last_audio_events,
        );
    }
    if runtime_shell.tmhm_forget_menu_open {
        let party_index = selected_party_index(runtime_shell)?;
        let slot = snapshot
            .party
            .slots
            .iter()
            .find(|slot| slot.index == party_index)
            .with_context(|| format!("selected party index {party_index} is not in the party"))?;
        return move_visible_cursor_slot(
            &mut runtime_shell.party_move_cursor,
            party_move_cursor_surface_id(party_index),
            slot.pokemon.moves.len() + 1,
            delta,
            &mut runtime_shell.last_audio_events,
        );
    }
    if matches!(
        runtime_shell.pending_script_party_selection.as_ref(),
        Some(PendingScriptPartySelection::MoveDeletion {
            party_index: Some(_)
        }) | Some(PendingScriptPartySelection::MoveTutor {
            party_index: Some(_),
            ..
        })
    ) {
        return move_visible_party_move_cursor(runtime_shell, delta);
    }
    if runtime_shell.party_menu_open {
        if runtime_shell.party_move_reorder_open {
            return move_visible_party_move_reorder_cursor(runtime_shell, delta);
        }
        if runtime_shell.party_summary_open {
            return move_visible_party_summary_pokemon(runtime_shell, delta);
        }
        if runtime_shell.fly_cursor.is_some() {
            return move_visible_fly_cursor(runtime_shell, delta);
        }
        if runtime_shell.party_switch_cursor.is_some() {
            return move_visible_party_switch_cursor(runtime_shell, delta);
        }
        if runtime_shell.party_give_take_cursor.is_some() {
            return move_visible_cursor_slot(
                &mut runtime_shell.party_give_take_cursor,
                "party:give-take".to_string(),
                2,
                delta,
                &mut runtime_shell.last_audio_events,
            );
        }
        if runtime_shell.party_action_cursor.is_some() {
            return move_visible_party_action_cursor(runtime_shell, delta);
        }
        return move_visible_regular_party_menu_cursor(runtime_shell, delta);
    }
    if runtime_shell.bill_pc_box_action_cursor.is_some() {
        return move_visible_cursor_slot(
            &mut runtime_shell.bill_pc_box_action_cursor,
            "pc:bill-box-actions".to_string(),
            4,
            delta,
            &mut runtime_shell.last_audio_events,
        );
    }
    if runtime_shell.bill_pc_box_cursor.is_some() {
        return move_visible_cursor_slot(
            &mut runtime_shell.bill_pc_box_cursor,
            "pc:bill-boxes".to_string(),
            crate::core::models::MAX_PC_BOXES,
            delta,
            &mut runtime_shell.last_audio_events,
        );
    }
    if runtime_shell.bill_pc_action_cursor.is_some() {
        return move_visible_cursor_slot(
            &mut runtime_shell.bill_pc_action_cursor,
            "pc:bill-actions".to_string(),
            VISIBLE_BILL_PC_ACTIONS.len(),
            delta,
            &mut runtime_shell.last_audio_events,
        );
    }
    if runtime_shell.pc_hub_cursor.is_some() {
        let option_count = visible_pc_hub_actions(&snapshot).len();
        return move_visible_cursor_slot(
            &mut runtime_shell.pc_hub_cursor,
            "pc:hub".to_string(),
            option_count,
            delta,
            &mut runtime_shell.last_audio_events,
        );
    }
    if runtime_shell.storage_cursor.is_some() {
        return move_visible_storage_cursor(runtime_shell, delta);
    }
    if runtime_shell.pc_item_cursor.is_some() {
        return move_visible_pc_item_cursor(runtime_shell, delta);
    }
    if runtime_shell.decoration_menu.is_some() {
        return move_visible_decoration_cursor(runtime_shell, delta);
    }
    if runtime_shell.player_pc_action_cursor.is_some() {
        let option_count = visible_player_pc_actions(runtime_shell).len();
        return move_visible_cursor_slot(
            &mut runtime_shell.player_pc_action_cursor,
            "pc:player-actions".to_string(),
            option_count,
            delta,
            &mut runtime_shell.last_audio_events,
        );
    }
    if runtime_shell.mailbox_action_cursor.is_some() {
        return move_visible_cursor_slot(
            &mut runtime_shell.mailbox_action_cursor,
            "pc:mailbox-actions".to_string(),
            VISIBLE_MAILBOX_ACTIONS.len(),
            delta,
            &mut runtime_shell.last_audio_events,
        );
    }
    if runtime_shell.mailbox_cursor.is_some() {
        let count = snapshot.mailbox.len();
        return move_visible_cursor_slot(
            &mut runtime_shell.mailbox_cursor,
            "pc:mailbox".to_string(),
            count,
            delta,
            &mut runtime_shell.last_audio_events,
        );
    }
    if runtime_shell.pokedex_menu_open {
        return move_visible_pokedex_cursor(runtime_shell, delta);
    }
    if runtime_shell.pokegear_menu_open {
        return move_visible_pokegear_cursor(runtime_shell, delta);
    }
    if visible_field_pack_is_open(runtime_shell) {
        if runtime_shell.field_pack_target_mode.is_some() {
            return move_visible_field_pack_target_cursor(runtime_shell, delta);
        }
        if runtime_shell.field_pack_action_cursor.is_some() {
            return move_visible_field_pack_action_cursor(runtime_shell, delta);
        }
        return move_visible_active_field_pack_cursor(runtime_shell, delta);
    }
    if let Some(battle) = &snapshot.battle {
        if runtime_shell.battle_party_summary_open {
            return move_visible_party_summary_pokemon(runtime_shell, delta);
        }
        if runtime_shell.battle_faint_prompt_cursor.is_some() {
            return move_visible_cursor_slot(
                &mut runtime_shell.battle_faint_prompt_cursor,
                "battle:faint-prompt".to_string(),
                2,
                delta,
                &mut runtime_shell.last_audio_events,
            );
        }
        if runtime_shell.battle_shift_prompt_cursor.is_some() {
            return move_visible_cursor_slot(
                &mut runtime_shell.battle_shift_prompt_cursor,
                "battle:shift-prompt".to_string(),
                2,
                delta,
                &mut runtime_shell.last_audio_events,
            );
        }
        if battle.commands.can_use_items {
            if runtime_shell.field_pack_action_cursor.is_some() {
                return move_visible_field_pack_action_cursor(runtime_shell, delta);
            }
            if runtime_shell.ball_cursor.is_some() {
                return move_visible_ball_cursor(runtime_shell, delta);
            }
            if runtime_shell.bag_cursor.is_some() {
                if runtime_shell.battle_pack_target_mode.is_some() {
                    return move_visible_battle_pack_target_cursor(runtime_shell, delta);
                }
                return move_visible_battle_bag_cursor(runtime_shell, delta);
            }
            if runtime_shell.key_item_cursor.is_some() {
                return move_visible_key_item_cursor(runtime_shell, delta);
            }
            if runtime_shell.tmhm_cursor.is_some() {
                return move_visible_tmhm_cursor(runtime_shell, delta);
            }
        }
        if runtime_shell.battle_move_cursor.is_some() {
            return move_visible_battle_move_cursor(runtime_shell, delta);
        }
        if runtime_shell.battle_party_action_cursor.is_some() {
            let current = strict_readonly_cursor_index(
                &runtime_shell.battle_party_action_cursor,
                "battle:party-actions",
                3,
            )
            .context(
                "battle party action surface battle:party-actions is active without a valid cursor",
            )?;
            let next = if delta.is_negative() {
                current.saturating_sub(1)
            } else {
                (current + 1).min(2)
            };
            if next != current {
                runtime_shell.battle_party_action_cursor = Some(MenuCursor {
                    surface_id: "battle:party-actions".to_string(),
                    option_index: next,
                });
                runtime_shell.last_audio_events.push(format!(
                    "battle party action cursor {}->{}",
                    current + 1,
                    next + 1
                ));
                trim_event_log(&mut runtime_shell.last_audio_events);
            }
            return Ok(());
        }
        if runtime_shell.battle_switch_cursor.is_some() {
            return move_visible_battle_switch_cursor(runtime_shell, delta);
        }
        return move_visible_battle_action_cursor_axis(
            runtime_shell,
            BattleMenuAxis::Vertical,
            delta,
        );
    }
    if runtime_shell.elevator_cursor.is_some() {
        return move_visible_elevator_cursor(runtime_shell, delta);
    }
    if runtime_shell.pending_pc_release.is_some() {
        return move_visible_cursor_slot(
            &mut runtime_shell.yes_no_cursor,
            "pc:release-confirm".to_string(),
            2,
            delta,
            &mut runtime_shell.last_audio_events,
        );
    }
    if runtime_shell.pc_notice.is_some() {
        return Ok(());
    }
    if runtime_shell.bill_pc_box_summary.is_some() {
        return Ok(());
    }
    if runtime_shell.bill_pc_pokemon_action_cursor.is_some() {
        return move_visible_cursor_slot(
            &mut runtime_shell.bill_pc_pokemon_action_cursor,
            "pc:pokemon-actions".to_string(),
            4,
            delta,
            &mut runtime_shell.last_audio_events,
        );
    }
    if runtime_shell.field_notice.is_some() {
        return Ok(());
    }
    if snapshot.pending_shop.is_some() {
        if !runtime_shell.shop_welcome_seen {
            return Ok(());
        }
        if runtime_shell.shop_notice.is_some() {
            return Ok(());
        }
        if runtime_shell.shop_quantity.is_some() {
            return adjust_visible_shop_quantity(runtime_shell, if delta < 0 { 1 } else { -1 });
        }
        if runtime_shell.shop_top_cursor.is_some() {
            return move_visible_mart_cursor_slot(
                &mut runtime_shell.shop_top_cursor,
                "shop:top".to_string(),
                3,
                delta,
                &mut runtime_shell.last_audio_events,
            );
        }
        if runtime_shell.sell_cursor.is_some() {
            return move_visible_sell_cursor(runtime_shell, delta);
        }
        return move_visible_shop_buy_cursor(runtime_shell, delta);
    }
    if visible_menu_has_selectable_options(&snapshot) {
        return move_visible_menu_cursor(runtime_shell, delta);
    }
    Ok(())
}

fn move_visible_decoration_cursor(
    runtime_shell: &mut BevyRuntimeShell,
    delta: isize,
) -> Result<()> {
    let menu = runtime_shell
        .decoration_menu
        .as_mut()
        .context("decoration cursor requires an active menu")?;
    let (cursor, surface_id, option_count) = match &mut menu.phase {
        VisibleDecorationMenuPhase::Categories { categories, cursor } => {
            (cursor, "pc:decorations:categories", categories.len() + 1)
        }
        VisibleDecorationMenuPhase::Decorations {
            decorations,
            cursor,
            ..
        } => (cursor, "pc:decorations:items", decorations.len() + 2),
        VisibleDecorationMenuPhase::Side { cursor, .. } => (cursor, "pc:decorations:side", 3),
    };
    let mut slot = Some(cursor.clone());
    move_visible_cursor_slot(
        &mut slot,
        surface_id.to_string(),
        option_count,
        delta,
        &mut runtime_shell.last_audio_events,
    )?;
    *cursor = slot.context("decoration cursor move removed its cursor")?;
    Ok(())
}

fn adjust_visible_kurt_apricorn_quantity(
    runtime_shell: &mut BevyRuntimeShell,
    delta: i16,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let choices = visible_kurt_apricorn_choices(&snapshot);
    let selected = strict_readonly_cursor_index(
        &runtime_shell.kurt_apricorn_cursor,
        "script:kurt-apricorn",
        choices.len(),
    )
    .context("Kurt quantity selection has no valid cursor")?;
    let maximum = choices
        .get(selected)
        .map(|(_, quantity)| *quantity)
        .context("Kurt quantity selection has no Apricorn type")?;
    let current = runtime_shell.kurt_apricorn_quantity.unwrap_or(1);
    runtime_shell.kurt_apricorn_quantity =
        Some((i32::from(current) + i32::from(delta)).clamp(1, i32::from(maximum)) as u16);
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(())
}
