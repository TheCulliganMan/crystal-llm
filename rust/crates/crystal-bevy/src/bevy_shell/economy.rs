fn take_visible_money(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    apply_visible_currency_delta(runtime_shell, RuntimeCurrencyAccount::Money, 100, false)
}

fn add_visible_coins(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    apply_visible_currency_delta(runtime_shell, RuntimeCurrencyAccount::Coins, 100, true)
}

fn apply_visible_currency_delta(
    runtime_shell: &mut BevyRuntimeShell,
    account: RuntimeCurrencyAccount,
    amount: u32,
    add: bool,
) -> Result<()> {
    let mutation = if add {
        runtime_shell.shell.add_currency(account, amount)?
    } else {
        runtime_shell.shell.take_currency(account, amount)?
    };
    runtime_shell.last_audio_events.push(format!(
        "currency {} account={:?} amount={} before={} after={} cap={} checksum={:?}",
        if add { "add" } else { "take" },
        mutation.account,
        mutation.amount,
        mutation.value_before,
        mutation.value_after,
        mutation.cap,
        mutation.state_checksum
    ));
    Ok(())
}

fn record_visible_link_win(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    record_visible_link_result(runtime_shell, RuntimeLinkBattleResult::Win)
}

fn record_visible_link_loss(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    record_visible_link_result(runtime_shell, RuntimeLinkBattleResult::Loss)
}

fn record_visible_link_draw(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    record_visible_link_result(runtime_shell, RuntimeLinkBattleResult::Draw)
}

fn record_visible_link_result(
    runtime_shell: &mut BevyRuntimeShell,
    result: RuntimeLinkBattleResult,
) -> Result<()> {
    let record = runtime_shell.shell.record_link_battle_result(result)?;
    runtime_shell.last_audio_events.push(format!(
        "link result={:?} wins={} losses={} draws={} checksum={:?}",
        record.result,
        record.wins_after,
        record.losses_after,
        record.draws_after,
        record.state_checksum
    ));
    Ok(())
}

fn toggle_visible_battle_style(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let mut options = snapshot.trainer.options.clone();
    options.battle_style = match options.battle_style {
        BattleStyle::Shift => BattleStyle::Set,
        BattleStyle::Set => BattleStyle::Shift,
    };
    let result = runtime_shell.shell.set_options(options)?;
    runtime_shell.last_audio_events.push(format!(
        "options battle_style {:?}->{:?} checksum={:?}",
        result.options_before.battle_style,
        result.options_after.battle_style,
        result.state_checksum
    ));
    Ok(())
}

fn teach_selected_tmhm(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let party_index = selected_party_index(runtime_shell)?;
    teach_selected_tmhm_on(runtime_shell, party_index)
}

fn teach_selected_tmhm_on_second_slot(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let party_index = second_party_index(&snapshot)?;
    teach_selected_tmhm_on(runtime_shell, party_index)
}

fn teach_selected_tmhm_on(runtime_shell: &mut BevyRuntimeShell, party_index: usize) -> Result<()> {
    let (item_id, move_id) = selected_tmhm(runtime_shell)?;
    let snapshot = runtime_shell.shell.snapshot()?;
    let pokemon = snapshot
        .party
        .slots
        .iter()
        .find(|slot| slot.index == party_index)
        .with_context(|| format!("selected party index {party_index} is not in the party"))?;
    let move_count = pokemon.pokemon.moves.len();
    let replace_slot = if move_count >= 4 {
        Some(selected_party_move_slot(runtime_shell, party_index)?)
    } else {
        None
    };
    record_visible_runtime_action(
        runtime_shell,
        format!("party:tmhm:{item_id}:pokemon:{party_index}:replace:{replace_slot:?}"),
    )?;
    let taught =
        match runtime_shell
            .shell
            .use_bag_tmhm_on_party_pokemon(&item_id, party_index, replace_slot)
        {
            Ok(taught) => taught,
            Err(error) if tmhm_error_is_play_refusal(&error) => {
                return handle_visible_field_item_refusal(runtime_shell, &item_id, error);
            }
            Err(error) => return Err(error),
        };
    runtime_shell.last_audio_events.push(format!(
        "tmhm item={} move={:?} party_index={} replace_slot={:?} item_use={:?} checksum={:?}",
        item_id, move_id, party_index, replace_slot, taught.item_use, taught.state_checksum
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    set_shell_action_status(
        runtime_shell,
        format!(
            "TAUGHT {} TO PARTY #{party_index}",
            move_id
                .as_deref()
                .map(|move_id| battle_move_display_name(&snapshot, move_id))
                .unwrap_or_else(|| item_display_name(&snapshot, &item_id))
        ),
    );
    runtime_shell.party_move_cursor = None;
    close_visible_field_pack_without_log(runtime_shell);
    let learned_move = move_id
        .as_deref()
        .map(|move_id| battle_move_display_name(&snapshot, move_id))
        .unwrap_or_else(|| item_display_name(&snapshot, &item_id));
    runtime_shell.field_notice = Some(format!(
        "{} learned {}!",
        pokemon.pokemon.nickname, learned_move
    ));
    mark_runtime_snapshot_dirty(runtime_shell);
    continue_visible_script_after_prompt(runtime_shell)?;
    Ok(())
}

fn use_selected_active_battle_item(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let item_id = selected_carried_battle_item_matching(
        runtime_shell,
        |item| {
            item.battle_usable
                && (item.battle_stat_boost_stat.is_some()
                    || item.battle_focus_energy == Some(true)
                    || item.battle_stat_drop_guard == Some(true)
                    || item.revive_hp_percent.is_some()
                    || !item.status_heals.is_empty()
                    || item.confusion_heal == Some(true))
        },
        "bag has no carried active battle item",
    )?;
    use_active_battle_item_by_id(runtime_shell, &item_id)
}

fn use_active_battle_item_by_id(runtime_shell: &mut BevyRuntimeShell, item_id: &str) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    if snapshot.battle.is_none() {
        return handle_visible_no_active_battle(runtime_shell, "active_item");
    }
    record_visible_runtime_action(runtime_shell, format!("battle:item:{item_id}:active"))?;
    let used = match runtime_shell
        .shell
        .use_bag_item_on_active_battle_pokemon(item_id)
    {
        Ok(used) => used,
        Err(error) if battle_item_error_is_play_refusal(&error) => {
            return handle_visible_battle_item_refusal(runtime_shell, item_id, error);
        }
        Err(error) => return Err(error),
    };
    record_visible_battle_item_action_frame(runtime_shell, item_id)?;
    reset_visible_battle_action_cursors(runtime_shell);
    runtime_shell.last_audio_events.push(format!(
        "active battle item item={} item_use={:?} battle_item={:?} checksum={:?}",
        item_id, used.item_use, used.battle_item, used.state_checksum
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    set_shell_action_status(runtime_shell, format!("USED {item_id} IN BATTLE"));
    let item = snapshot
        .items
        .iter()
        .find(|item| item.item_id == item_id)
        .with_context(|| format!("compiled item catalog missing {item_id}"))?;
    let uses_item_text = item.battle_stat_boost_stat.is_some()
        || item.battle_focus_energy == Some(true)
        || (item.confusion_heal == Some(true) && item.status_heals.is_empty());
    if uses_item_text {
        stage_visible_battle_item_use(runtime_shell, item_id)?;
    }
    stage_visible_battle_item_effect(runtime_shell, &snapshot, &used.battle_item, None)?;
    resolve_visible_battle_enemy_response_after_player_item(runtime_shell, item_id)?;
    Ok(())
}

fn use_selected_visible_battle_bag_item(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    if snapshot.battle.is_none() {
        return handle_visible_no_active_battle(runtime_shell, "bag_item");
    }
    if carried_battle_non_ball_item_ids(&snapshot).is_empty() {
        runtime_shell.bag_cursor = None;
        runtime_shell.battle_pack_target_mode = None;
        record_visible_runtime_action(runtime_shell, "battle:item:use:no_items")?;
        runtime_shell
            .last_audio_events
            .push("bag item pocket has no carried item".to_string());
        set_shell_action_status(runtime_shell, "NO ITEMS");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    let item_id = selected_battle_bag_item_id(runtime_shell)?;
    if let Some(ball_index) = carried_ball_item_ids(&snapshot)
        .iter()
        .position(|ball_id| ball_id == &item_id)
    {
        return throw_visible_battle_ball_id(runtime_shell, ball_index, item_id);
    }
    let item = snapshot
        .items
        .iter()
        .find(|item| item.item_id == item_id)
        .with_context(|| format!("selected bag item {item_id} is missing from item catalog"))?;
    let battle_usable = item.battle_usable;
    let battle_escape_mode = item.battle_escape_mode.clone();
    let battle_stat_drop_guard = item.battle_stat_drop_guard;
    if !battle_usable {
        record_visible_runtime_action(runtime_shell, format!("battle:item:unusable:{item_id}"))?;
        runtime_shell.battle_pack_target_mode = None;
        runtime_shell.party_move_cursor = None;
        runtime_shell.field_pack_action_cursor = None;
        runtime_shell
            .battle_messages
            .push_back("It won't have any effect.".to_string());
        runtime_shell.battle_message_scene = Some(Box::new(snapshot.clone()));
        mark_runtime_snapshot_dirty(runtime_shell);
        set_shell_action_status(runtime_shell, "ITEM CAN'T BE USED");
        return Ok(());
    }
    if battle_escape_mode.is_some() {
        return use_battle_escape_item_by_id(runtime_shell, &item_id);
    }
    if battle_stat_drop_guard == Some(true) {
        return use_guard_spec_by_id(runtime_shell, &item_id);
    }
    let targets_move = item_targets_party_move_fields(
        item.pp_restore_scope.as_deref(),
        item.pp_restore_points,
        item.pp_up_stages,
    );
    if targets_move {
        return open_visible_battle_pack_target(runtime_shell, BattlePackTargetMode::PartyMove);
    }
    if item_targets_party_pokemon_fields(item) {
        return open_visible_battle_pack_target(runtime_shell, BattlePackTargetMode::PartyPokemon);
    }
    if item_targets_active_battle_pokemon_fields(item) {
        return use_active_battle_item_by_id(runtime_shell, &item_id);
    }
    anyhow::bail!("battle item {item_id} has no declared battle payload")
}

fn selected_visible_battle_pack_action_item(runtime_shell: &mut BevyRuntimeShell) -> Result<String> {
    let snapshot = runtime_shell.shell.snapshot()?;
    if runtime_shell.ball_cursor.is_some() {
        return selected_battle_ball_id(runtime_shell).map(|(_, item_id)| item_id);
    }
    if runtime_shell.key_item_cursor.is_some() {
        return selected_key_item_id(runtime_shell);
    }
    if runtime_shell.tmhm_cursor.is_some() {
        return selected_tmhm(runtime_shell).map(|(item_id, _)| item_id);
    }
    let item_ids = carried_battle_non_ball_item_ids(&snapshot);
    let index = visible_cursor_index(
        &mut runtime_shell.bag_cursor,
        "battle:bag-items",
        item_ids.len(),
    );
    item_ids.get(index).cloned().context("battle Pack item cursor selected no item")
}

fn open_visible_battle_pack_action_menu(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let pocket = active_visible_field_pack_pocket(runtime_shell);
    let actions = visible_selected_pack_item_actions(&snapshot, runtime_shell, &pocket, true)?;
    let item_id = selected_visible_battle_pack_action_item(runtime_shell)?;
    visible_cursor_index(
        &mut runtime_shell.field_pack_action_cursor,
        "pack:actions",
        actions.len(),
    );
    record_visible_runtime_action(runtime_shell, format!("battle:pack:actions:open:{item_id}"))?;
    Ok(())
}

fn execute_visible_battle_pack_action(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let pocket = active_visible_field_pack_pocket(runtime_shell);
    let actions = visible_selected_pack_item_actions(&snapshot, runtime_shell, &pocket, true)?;
    let index = visible_cursor_index(
        &mut runtime_shell.field_pack_action_cursor,
        "pack:actions",
        actions.len(),
    );
    let action = actions[index];
    let item_id = selected_visible_battle_pack_action_item(runtime_shell)?;
    record_visible_runtime_action(
        runtime_shell,
        format!("battle:pack:action:{}:{}", item_id, visible_field_pack_action_record_id(action)),
    )?;
    close_visible_field_pack_action_menu(runtime_shell);
    match action {
        FieldPackAction::Use if runtime_shell.ball_cursor.is_some() => {
            throw_visible_battle_ball(runtime_shell)
        }
        FieldPackAction::Use if runtime_shell.bag_cursor.is_some() => {
            use_selected_visible_battle_bag_item(runtime_shell)
        }
        FieldPackAction::Use => {
            let snapshot = runtime_shell.shell.snapshot()?;
            runtime_shell.battle_pack_target_mode = None;
            runtime_shell.party_move_cursor = None;
            runtime_shell
                .battle_messages
                .push_back("It won't have any\neffect.".to_string());
            runtime_shell.battle_message_scene = Some(Box::new(snapshot));
            mark_runtime_snapshot_dirty(runtime_shell);
            Ok(())
        }
        FieldPackAction::Give | FieldPackAction::Toss | FieldPackAction::Select => {
            anyhow::bail!(
                "battle Pack generated field-only action {} for {item_id}",
                visible_field_pack_action_record_id(action)
            )
        }
        FieldPackAction::Quit => Ok(()),
    }
}

fn close_visible_field_pack_from_cancel(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    record_visible_runtime_action(runtime_shell, "pack:cancel")?;
    close_visible_field_pack_without_log(runtime_shell);
    set_shell_action_status(runtime_shell, "PACK CLOSED");
    continue_visible_script_after_prompt(runtime_shell)
}

fn open_visible_field_pack_action_menu(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let pocket = active_visible_field_pack_pocket(runtime_shell);
    if selected_field_pack_cancel_row(&snapshot, runtime_shell, &pocket) {
        return close_visible_field_pack_from_cancel(runtime_shell);
    }
    if runtime_shell.party_held_item_give_target.is_some() {
        if !visible_selected_pack_item_actions(&snapshot, runtime_shell, &pocket, false)?
            .contains(&FieldPackAction::Give)
        {
            record_visible_runtime_action(runtime_shell, "party:held_item:give:unholdable")?;
            runtime_shell.field_notice = Some("That item can't be held.".to_string());
            mark_runtime_snapshot_dirty(runtime_shell);
            set_shell_action_status(runtime_shell, "THAT ITEM CAN'T BE HELD");
            trim_event_log(&mut runtime_shell.last_audio_events);
            return Ok(());
        }
        return give_selected_held_item(runtime_shell);
    }
    let item_id = selected_field_pack_item_id(runtime_shell)?;
    let actions = visible_selected_pack_item_actions(&snapshot, runtime_shell, &pocket, false)?;
    visible_cursor_index(
        &mut runtime_shell.field_pack_action_cursor,
        "pack:actions",
        actions.len(),
    );
    record_visible_runtime_action(runtime_shell, format!("pack:actions:open:{item_id}"))?;
    runtime_shell.last_audio_events.push(format!(
        "opened Pack actions item={} {}",
        item_id,
        visible_field_pack_action_labels(&actions).join("/")
    ));
    set_shell_action_status(
        runtime_shell,
        format!(
            "ACTIONS {}",
            visible_field_pack_action_labels(&actions).join("/")
        ),
    );
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn close_visible_field_pack_action_menu(runtime_shell: &mut BevyRuntimeShell) {
    runtime_shell.field_pack_action_cursor = None;
    runtime_shell
        .last_audio_events
        .push("closed Pack actions".to_string());
    trim_event_log(&mut runtime_shell.last_audio_events);
}

fn move_visible_field_pack_action_cursor(
    runtime_shell: &mut BevyRuntimeShell,
    delta: isize,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let pocket = active_visible_field_pack_pocket(runtime_shell);
    let in_battle = snapshot.battle.is_some();
    let actions = visible_selected_pack_item_actions(
        &snapshot,
        runtime_shell,
        &pocket,
        in_battle,
    )?;
    move_visible_cursor_slot(
        &mut runtime_shell.field_pack_action_cursor,
        "pack:actions".to_string(),
        actions.len(),
        delta,
        &mut runtime_shell.last_audio_events,
    )
}

fn execute_visible_field_pack_action(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let pocket = active_visible_field_pack_pocket(runtime_shell);
    let actions = visible_selected_pack_item_actions(&snapshot, runtime_shell, &pocket, false)?;
    let index = visible_cursor_index(
        &mut runtime_shell.field_pack_action_cursor,
        "pack:actions",
        actions.len(),
    );
    let action = actions[index];
    let item_id = selected_field_pack_item_id(runtime_shell)?;
    record_visible_runtime_action(
        runtime_shell,
        format!(
            "pack:action:{}:{}",
            item_id,
            visible_field_pack_action_record_id(action)
        ),
    )?;
    match action {
        FieldPackAction::Use => {
            close_visible_field_pack_action_menu(runtime_shell);
            use_visible_field_bag_item_by_id(runtime_shell, item_id)
        }
        FieldPackAction::Select => {
            close_visible_field_pack_action_menu(runtime_shell);
            register_selected_visible_key_item(runtime_shell)
        }
        FieldPackAction::Quit => {
            close_visible_field_pack_action_menu(runtime_shell);
            Ok(())
        }
        FieldPackAction::Give => {
            close_visible_field_pack_action_menu(runtime_shell);
            open_visible_field_pack_target(runtime_shell, FieldPackTargetMode::HeldItem)
        }
        FieldPackAction::Toss => {
            close_visible_field_pack_action_menu(runtime_shell);
            begin_visible_pack_toss(runtime_shell, item_id)
        }
    }
}

fn begin_visible_pack_toss(
    runtime_shell: &mut BevyRuntimeShell,
    item_id: String,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let max_quantity = carried_item_quantity(&snapshot, &item_id)
        .with_context(|| format!("selected toss item {item_id} is not carried"))?;
    runtime_shell.pack_toss = Some(VisiblePackToss {
        item_id,
        quantity: 1,
        max_quantity,
        confirming: false,
    });
    runtime_shell.yes_no_cursor = None;
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(())
}

fn adjust_visible_pack_toss_quantity(
    runtime_shell: &mut BevyRuntimeShell,
    delta: i16,
) -> Result<()> {
    let Some(toss) = runtime_shell.pack_toss.as_mut() else {
        return Ok(());
    };
    if toss.confirming {
        return move_visible_cursor_slot(
            &mut runtime_shell.yes_no_cursor,
            "pack:toss-confirm".to_string(),
            2,
            delta.signum() as isize,
            &mut runtime_shell.last_audio_events,
        );
    }
    toss.quantity = match delta {
        1 if toss.quantity >= toss.max_quantity => 1,
        1 => toss.quantity + 1,
        -1 if toss.quantity <= 1 => toss.max_quantity,
        -1 => toss.quantity - 1,
        _ => (i32::from(toss.quantity) + i32::from(delta))
            .clamp(1, i32::from(toss.max_quantity)) as u16,
    };
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(())
}

fn confirm_visible_pack_toss(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let Some(toss) = runtime_shell.pack_toss.as_mut() else {
        return Ok(());
    };
    if !toss.confirming {
        toss.confirming = true;
        runtime_shell.yes_no_cursor = Some(MenuCursor {
            surface_id: "pack:toss-confirm".to_string(),
            option_index: 0,
        });
        mark_runtime_snapshot_dirty(runtime_shell);
        return Ok(());
    }
    let accepted = strict_readonly_cursor_index(
        &runtime_shell.yes_no_cursor,
        "pack:toss-confirm",
        2,
    )
    .context("Pack toss confirmation requires a valid cursor")?
        == 0;
    if !accepted {
        runtime_shell.pack_toss = None;
        runtime_shell.yes_no_cursor = None;
        mark_runtime_snapshot_dirty(runtime_shell);
        return Ok(());
    }
    let toss = runtime_shell
        .pack_toss
        .take()
        .context("Pack toss disappeared before confirmation")?;
    runtime_shell.yes_no_cursor = None;
    toss_visible_field_pack_item(runtime_shell, toss.item_id, toss.quantity)
}

fn cancel_visible_pack_toss(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    runtime_shell.pack_toss = None;
    runtime_shell.yes_no_cursor = None;
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(())
}

fn toss_visible_field_pack_item(
    runtime_shell: &mut BevyRuntimeShell,
    item_id: String,
    quantity: u16,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let display_name = item_display_name(&snapshot, &item_id);
    let removed = runtime_shell.shell.remove_bag_item(&item_id, quantity)?;
    match active_visible_field_pack_pocket(runtime_shell) {
        FieldPackPocket::Items => move_visible_bag_cursor(runtime_shell, 0)?,
        FieldPackPocket::Balls => move_visible_ball_cursor(runtime_shell, 0)?,
        FieldPackPocket::KeyItems => move_visible_key_item_cursor(runtime_shell, 0)?,
        FieldPackPocket::TmHm => move_visible_tmhm_cursor(runtime_shell, 0)?,
        FieldPackPocket::Custom(pocket_id) => {
            move_visible_custom_item_cursor(runtime_shell, &pocket_id, 0)?
        }
    }
    runtime_shell.last_audio_events.push(format!(
        "tossed Pack item {} quantity={} before={} after={} checksum={:?}",
        removed.item_id,
        removed.quantity,
        removed.quantity_before,
        removed.quantity_after,
        removed.state_checksum
    ));
    set_shell_action_status(
        runtime_shell,
        format!("TOSSED {} x{}", display_name, removed.quantity),
    );
    let notice = format!("Threw away {}(S).", display_name);
    runtime_shell.field_notice = Some(notice);
    mark_runtime_snapshot_dirty(runtime_shell);
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn register_selected_visible_key_item(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let item_id = selected_field_pack_item_id(runtime_shell)?;
    let carried_key_item = snapshot
        .bag
        .key_items
        .iter()
        .any(|item| item.item_id == item_id && item.quantity > 0);
    if !carried_key_item {
        record_visible_runtime_action(
            runtime_shell,
            format!("pack:key_item:register:{item_id}:invalid"),
        )?;
        runtime_shell
            .last_audio_events
            .push(format!("selected Pack item {item_id} cannot be registered"));
        let notice = "You can't register\nthat item.".to_string();
        runtime_shell.field_notice = Some(notice);
        set_shell_action_status(runtime_shell, "ITEM CAN'T BE REGISTERED");
        mark_runtime_snapshot_dirty(runtime_shell);
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    record_visible_runtime_action(runtime_shell, format!("pack:key_item:register:{item_id}"))?;
    let registered = runtime_shell.shell.register_key_item(&item_id)?;
    let display_name = item_display_name(&snapshot, &registered.outcome.item_id);
    runtime_shell.last_audio_events.push(format!(
        "registered key item {} previous={:?} checksum={:?}",
        registered.outcome.item_id, registered.outcome.previous_item_id, registered.state_checksum
    ));
    set_shell_action_status(
        runtime_shell,
        format!("REGISTERED {display_name}"),
    );
    let notice = format!("Registered the\n{display_name}.");
    runtime_shell.field_notice = Some(notice);
    mark_runtime_snapshot_dirty(runtime_shell);
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn visible_selected_pack_item_actions(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    pocket: &FieldPackPocket,
    in_battle: bool,
) -> Result<Vec<FieldPackAction>> {
    let item_id = if in_battle && *pocket == FieldPackPocket::Items {
        let item_ids = carried_battle_non_ball_item_ids(snapshot);
        strict_readonly_cursor_index(
            &runtime_shell.bag_cursor,
            "battle:bag-items",
            field_pack_selectable_count(item_ids.len()),
        )
        .filter(|index| *index < item_ids.len())
        .and_then(|index| item_ids.get(index).cloned())
    } else {
        selected_field_pack_item_id_from_snapshot(snapshot, runtime_shell, pocket)
    }
    .context("Pack action menu has no selected item")?;
    let item = snapshot
        .items
        .iter()
        .find(|item| item.item_id == item_id)
        .with_context(|| format!("selected Pack item {item_id} is missing from the catalog"))?;
    if in_battle {
        return Ok(if item.battle_usable {
            vec![FieldPackAction::Use, FieldPackAction::Quit]
        } else {
            vec![FieldPackAction::Quit]
        });
    }
    let cant_toss = item.property.split('|').any(|flag| flag.trim() == "CANT_TOSS");
    let cant_select = item
        .property
        .split('|')
        .any(|flag| flag.trim() == "CANT_SELECT");
    let usable = item.field_usable;
    let mut actions = Vec::new();
    if cant_toss || usable {
        actions.push(FieldPackAction::Use);
    }
    if !cant_toss {
        actions.push(FieldPackAction::Give);
    }
    if !cant_toss {
        actions.push(FieldPackAction::Toss);
    }
    if !cant_select {
        actions.push(FieldPackAction::Select);
    }
    actions.push(FieldPackAction::Quit);
    Ok(actions)
}

fn visible_field_pack_action_labels(actions: &[FieldPackAction]) -> Vec<&'static str> {
    actions
        .iter()
        .copied()
        .map(visible_field_pack_action_label)
        .collect()
}

fn visible_field_pack_action_label(action: FieldPackAction) -> &'static str {
    match action {
        FieldPackAction::Use => "USE",
        FieldPackAction::Give => "GIVE",
        FieldPackAction::Toss => "TOSS",
        FieldPackAction::Select => "SEL",
        FieldPackAction::Quit => "QUIT",
    }
}

fn visible_field_pack_action_record_id(action: FieldPackAction) -> &'static str {
    match action {
        FieldPackAction::Use => "use",
        FieldPackAction::Give => "give",
        FieldPackAction::Toss => "toss",
        FieldPackAction::Select => "sel",
        FieldPackAction::Quit => "quit",
    }
}

fn use_visible_field_bag_item_by_id(
    runtime_shell: &mut BevyRuntimeShell,
    item_id: String,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let item = snapshot
        .items
        .iter()
        .find(|item| item.item_id == item_id)
        .with_context(|| format!("selected bag item {item_id} is missing from item catalog"))?
        .clone();
    let selected_is_tmhm = snapshot
        .bag
        .tm_hm
        .iter()
        .any(|tmhm| tmhm.item_id == item_id);
    if !item.field_usable && !selected_is_tmhm {
        record_visible_runtime_action(runtime_shell, format!("field:item:{item_id}:no_action"))?;
        runtime_shell
            .last_audio_events
            .push(format!("selected bag item {item_id} is not field usable"));
        runtime_shell.field_notice = Some("It won't have any effect.".to_string());
        mark_runtime_snapshot_dirty(runtime_shell);
        set_shell_action_status(runtime_shell, format!("{item_id} CANNOT BE USED HERE"));
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    record_visible_runtime_action(runtime_shell, format!("field:item:{item_id}"))?;
    if selected_is_tmhm {
        return open_visible_tmhm_teach_prompt(runtime_shell);
    }
    if item.repel_steps.is_some() {
        if snapshot.progression.repel_steps_remaining > 0 {
            record_visible_runtime_action(runtime_shell, format!("field:item:{item_id}:repel_active"))?;
            runtime_shell.field_notice = Some(visible_asm_text(
                &snapshot,
                "RepelUsedEarlierIsStillInEffectText",
            )?);
            mark_runtime_snapshot_dirty(runtime_shell);
            set_shell_action_status(runtime_shell, "REPEL STILL IN EFFECT");
            return Ok(());
        }
        let item_use = match runtime_shell.shell.use_bag_repel_in_field(&item_id) {
            Ok(item_use) => item_use,
            Err(error) if party_field_move_error_is_play_refusal(&error) => {
                return handle_visible_field_action_refusal(
                    runtime_shell,
                    &item_id,
                    format!("{item_id} CAN'T BE USED HERE"),
                    error,
                );
            }
            Err(error) => return Err(error),
        };
        runtime_shell.last_audio_events.push(format!(
            "field repel item={} steps={} consumed={} checksum={:?}",
            item_id,
            item_use.repel_steps_after,
            item_use.item_use.consumed,
            item_use.state_checksum
        ));
        set_shell_action_status(
            runtime_shell,
            format!("REPEL ACTIVE {} STEPS", item_use.repel_steps_after),
        );
        close_visible_field_pack_without_log(runtime_shell);
        runtime_shell.field_notice = Some(visible_asm_text(&snapshot, "RepelUseText")?);
        mark_runtime_snapshot_dirty(runtime_shell);
        continue_visible_script_after_prompt(runtime_shell)?;
        return Ok(());
    }
    if field_rule_item_matches(&runtime_shell.shell, "escape_rope", &item_id) {
        let item_use = match runtime_shell.shell.use_bag_escape_rope_in_field(&item_id) {
            Ok(item_use) => item_use,
            Err(error) if party_field_move_error_is_play_refusal(&error) => {
                return handle_visible_field_action_refusal(
                    runtime_shell,
                    &item_id,
                    format!("{item_id} CAN'T BE USED HERE"),
                    error,
                );
            }
            Err(error) => return Err(error),
        };
        runtime_shell.last_audio_events.push(format!(
            "field escape_rope item={} destination={} warp={} checksum={:?}",
            item_id,
            item_use.destination_map,
            item_use.destination_warp_index,
            item_use.state_checksum
        ));
        set_shell_action_status(
            runtime_shell,
            format!(
                "ESCAPE ROPE TO {} WARP {}",
                item_use.destination_map, item_use.destination_warp_index
            ),
        );
        close_visible_field_pack_without_log(runtime_shell);
        runtime_shell.field_notice = Some(visible_asm_text(&snapshot, "UseEscapeRopeText")?);
        retain_visible_field_notice_scene(runtime_shell, &snapshot);
        runtime_shell.pending_field_travel_arrival = true;
        runtime_shell.pending_field_travel_delay_frames = None;
        runtime_shell.visible_field_travel_animation = Some(VisibleFieldTravelAnimation::DigOut);
        return Ok(());
    }
    if runtime_shell.shell.fishing_rod_ids().contains(&item_id) {
        let scene = snapshot.clone();
        let item_use = match runtime_shell.shell.use_bag_fishing_rod_in_field(&item_id) {
            Ok(item_use) => item_use,
            Err(error) if fishing_error_is_cant_fish_here(&error) => {
                close_visible_field_pack_without_log(runtime_shell);
                retain_visible_field_notice_scene(runtime_shell, &scene);
                runtime_shell.field_notice = Some(visible_fishing_cant_cast_text());
                mark_runtime_snapshot_dirty(runtime_shell);
                return Ok(());
            }
            Err(error) if party_field_move_error_is_play_refusal(&error) => {
                return handle_visible_field_action_refusal(
                    runtime_shell,
                    &item_id,
                    "NOT EVEN A NIBBLE",
                    error,
                );
            }
            Err(error) => return Err(error),
        };
        runtime_shell.last_audio_events.push(format!(
            "field fishing item={} rod={} group={:?} bite={:?} battle={:?} checksum={:?}",
            item_id,
            item_use.rod,
            item_use.cast.session.group,
            item_use.cast.bite,
            item_use.cast.wild_battle,
            item_use.state_checksum
        ));
        set_shell_action_status(
            runtime_shell,
            format!(
                "FISHING {:?} {:?}",
                item_use.cast.bite, item_use.cast.wild_battle
            ),
        );
        close_visible_field_pack_without_log(runtime_shell);
        present_visible_fishing_cast(
            runtime_shell,
            &scene,
            item_use.cast.bite,
            item_use.cast.wild_battle.is_some(),
        )?;
        return Ok(());
    }
    if field_rule_item_matches(&runtime_shell.shell, "bicycle", &item_id) {
        if snapshot.overworld.mode == MovementMode::Bike
            && snapshot
                .progression
                .active_engine_flags
                .contains("ENGINE_ALWAYS_ON_BIKE")
        {
            runtime_shell.field_notice = Some(visible_asm_text(&snapshot, "CantGetOffBikeText")?);
            mark_runtime_snapshot_dirty(runtime_shell);
            set_shell_action_status(runtime_shell, "CAN'T GET OFF HERE");
            return Ok(());
        }
        let item_use = match runtime_shell.shell.use_bag_bicycle_in_field(&item_id) {
            Ok(item_use) => item_use,
            Err(error) if party_field_move_error_is_play_refusal(&error) => {
                return handle_visible_field_action_refusal(
                    runtime_shell,
                    &item_id,
                    format!("{item_id} CAN'T BE USED HERE"),
                    error,
                );
            }
            Err(error) => return Err(error),
        };
        runtime_shell.last_audio_events.push(format!(
            "field bicycle item={} mode={:?}->{:?} checksum={:?}",
            item_id, item_use.mode_before, item_use.mode_after, item_use.state_checksum
        ));
        set_shell_action_status(
            runtime_shell,
            format!("BICYCLE MODE {:?}", item_use.mode_after),
        );
        if item_use.mode_after == MovementMode::Bike {
            // BikeFunction starts MUSIC_BICYCLE before Script_GetOnBike owns
            // the acknowledgement textbox. Field text otherwise suspends the
            // current-music synchronizer until the player closes that text.
            queue_visible_current_music(runtime_shell)?;
        }
        close_visible_field_pack_without_log(runtime_shell);
        // VAR_MOVEMENT changes before the source textbox, but the visible
        // player sprite changes only at UpdatePlayerSprite after it closes.
        retain_visible_field_notice_scene(runtime_shell, &snapshot);
        let display_name = item_display_name(&snapshot, &item_id);
        runtime_shell.field_notice = Some(match item_use.mode_after {
            MovementMode::Bike => format!(
                "{} got on the\n{}.",
                snapshot.trainer.player_name, display_name
            ),
            MovementMode::Normal => format!(
                "{} got off\nthe {}.",
                snapshot.trainer.player_name, display_name
            ),
        MovementMode::Skate | MovementMode::Surf | MovementMode::SurfPika => {
            anyhow::bail!("bicycle use ended in invalid mode {:?}", item_use.mode_after)
        }
        });
        mark_runtime_snapshot_dirty(runtime_shell);
        continue_visible_script_after_prompt(runtime_shell)?;
        return Ok(());
    }
    if field_rule_item_matches(&runtime_shell.shell, "itemfinder", &item_id) {
        let item_use = match runtime_shell.shell.use_bag_itemfinder_in_field(&item_id) {
            Ok(item_use) => item_use,
            Err(error) if party_field_move_error_is_play_refusal(&error) => {
                return handle_visible_field_action_refusal(
                    runtime_shell,
                    &item_id,
                    format!("{item_id} CAN'T BE USED HERE"),
                    error,
                );
            }
            Err(error) => return Err(error),
        };
        runtime_shell.last_audio_events.push(format!(
            "field itemfinder item={} found={:?} cues={} consumed={} checksum={:?}",
            item_id,
            item_use.found,
            item_use.itemfinder_sound_cues,
            item_use.item_use.consumed,
            item_use.state_checksum
        ));
        set_shell_action_status(runtime_shell, format!("ITEMFINDER {:?}", item_use.found));
        close_visible_field_pack_without_log(runtime_shell);
        runtime_shell.field_notice = Some(visible_asm_text(
            &snapshot,
            if item_use.found.is_some() {
                "ItemfinderItemNearbyText"
            } else {
                "ItemfinderNopeText"
            },
        )?);
        mark_runtime_snapshot_dirty(runtime_shell);
        continue_visible_script_after_prompt(runtime_shell)?;
        return Ok(());
    }
    if field_rule_item_matches(&runtime_shell.shell, "squirtbottle", &item_id) {
        let item_use = match runtime_shell.shell.use_bag_squirtbottle_in_field(&item_id) {
            Ok(item_use) => item_use,
            Err(error) if party_field_move_error_is_play_refusal(&error) => {
                return handle_visible_field_action_refusal(
                    runtime_shell,
                    &item_id,
                    format!("{item_id} CAN'T BE USED HERE"),
                    error,
                );
            }
            Err(error) => return Err(error),
        };
        runtime_shell.last_audio_events.push(format!(
            "field squirtbottle item={} target={:?} movement={} script={:?} checksum={:?}",
            item_id,
            item_use.target_object_identifier,
            item_use.target_movement,
            item_use.target_script,
            item_use.state_checksum
        ));
        set_shell_action_status(
            runtime_shell,
            format!(
                "SQUIRTBOTTLE TARGET {:?}",
                item_use.target_object_identifier
            ),
        );
        close_visible_field_pack_without_log(runtime_shell);
        consume_visible_dispatched_field_script(runtime_shell)?;
        return Ok(());
    }
    if field_rule_item_matches(&runtime_shell.shell, "coin_case", &item_id) {
        let item_use = runtime_shell.shell.use_bag_coin_case_in_field(&item_id)?;
        runtime_shell.last_audio_events.push(format!(
            "field coin_case item={} {}={} checksum={:?}",
            item_id, item_use.balance_label, item_use.balance, item_use.state_checksum
        ));
        set_shell_action_status(
            runtime_shell,
            format!("{} {}", item_use.balance_label, item_use.balance),
        );
        close_visible_field_pack_without_log(runtime_shell);
        open_visible_field_balance_boundary(
            runtime_shell,
            "FieldCoinCase",
            &item_id,
            &item_use.balance_label,
            item_use.balance,
        );
        return Ok(());
    }
    if field_rule_item_matches(&runtime_shell.shell, "blue_card", &item_id) {
        let item_use = runtime_shell.shell.use_bag_blue_card_in_field(&item_id)?;
        runtime_shell.last_audio_events.push(format!(
            "field blue_card item={} {}={} checksum={:?}",
            item_id, item_use.balance_label, item_use.balance, item_use.state_checksum
        ));
        set_shell_action_status(
            runtime_shell,
            format!("{} {}", item_use.balance_label, item_use.balance),
        );
        close_visible_field_pack_without_log(runtime_shell);
        open_visible_field_balance_boundary(
            runtime_shell,
            "FieldBlueCard",
            &item_id,
            &item_use.balance_label,
            item_use.balance,
        );
        return Ok(());
    }
    if field_rule_item_matches(&runtime_shell.shell, "town_map", &item_id) {
        let item_use = runtime_shell.shell.use_bag_town_map_in_field(&item_id)?;
        close_visible_field_pack_without_log(runtime_shell);
        open_visible_pokegear_menu(runtime_shell)?;
        runtime_shell.last_audio_events.push(format!(
            "field town_map item={} landmark={:?} checksum={:?}",
            item_id, item_use.landmark, item_use.state_checksum
        ));
        set_shell_action_status(runtime_shell, format!("TOWN MAP {:?}", item_use.landmark));
        return Ok(());
    }
    if runtime_shell.shell.is_bag_box_item(&item_id) {
        let item_use = runtime_shell.shell.use_bag_box_in_field(&item_id)?;
        runtime_shell.last_audio_events.push(format!(
            "field box item={} flag={} already_owned={} consumed={} checksum={:?}",
            item_id,
            item_use.decoration_flag,
            item_use.already_owned,
            item_use.item_use.consumed,
            item_use.state_checksum
        ));
        set_shell_action_status(
            runtime_shell,
            format!(
                "DECORATION {} OWNED={}",
                item_use.decoration_flag, item_use.already_owned
            ),
        );
        close_visible_field_pack_without_log(runtime_shell);
        runtime_shell.field_notice = Some(visible_asm_text(&snapshot, "SentTrophyHomeText")?);
        mark_runtime_snapshot_dirty(runtime_shell);
        return Ok(());
    }
    if item.party_revive_hp_percent.is_some() {
        let used = match runtime_shell.shell.use_bag_item_on_whole_party(&item_id) {
            Ok(used) => used,
            Err(error) if field_item_error_is_play_refusal(&error) => {
                return handle_visible_field_item_refusal(runtime_shell, &item_id, error);
            }
            Err(error) => return Err(error),
        };
        runtime_shell.last_audio_events.push(format!(
            "field whole-party item={} item_use={:?} effect={:?} checksum={:?}",
            item_id, used.item_use, used.item_effect, used.state_checksum
        ));
        trim_event_log(&mut runtime_shell.last_audio_events);
        set_shell_action_status(runtime_shell, format!("USED {item_id} ON PARTY"));
        close_visible_field_pack_without_log(runtime_shell);
        runtime_shell.field_notice = Some(format!(
            "{}'s <PKMN>\nwere all healed!",
            snapshot.trainer.player_name
        ));
        mark_runtime_snapshot_dirty(runtime_shell);
        continue_visible_script_after_prompt(runtime_shell)?;
        return Ok(());
    }
    let targets_move = item_targets_party_move_fields(
        item.pp_restore_scope.as_deref(),
        item.pp_restore_points,
        item.pp_up_stages,
    );
    if targets_move {
        return open_visible_field_pack_target(runtime_shell, FieldPackTargetMode::PartyMove);
    }
    if item_targets_party_pokemon_fields(&item) {
        return open_visible_field_pack_target(runtime_shell, FieldPackTargetMode::PartyPokemon);
    }
    if runtime_shell.shell.is_bag_pokegear_item(&item_id) {
        let item_use = runtime_shell.shell.use_bag_pokegear_in_field(&item_id)?;
        close_visible_field_pack_without_log(runtime_shell);
        open_visible_pokegear_menu(runtime_shell)?;
        runtime_shell.last_audio_events.push(format!(
            "field pokegear item={} consumed={} checksum={:?}",
            item_id, item_use.item_use.consumed, item_use.state_checksum
        ));
        trim_event_log(&mut runtime_shell.last_audio_events);
        set_shell_action_status(runtime_shell, format!("OPENED POKEGEAR WITH {item_id}"));
        return Ok(());
    }
    anyhow::bail!("field item {item_id} has no declared field payload")
}

fn field_item_error_is_play_refusal(error: &anyhow::Error) -> bool {
    let Some(item_error) = error.downcast_ref::<BattleItemError>() else {
        return false;
    };
    matches!(
        item_error,
        BattleItemError::TargetFainted { .. } | BattleItemError::NoTargetChange { .. }
    )
}

fn tmhm_error_is_play_refusal(error: &anyhow::Error) -> bool {
    let Some(tmhm_error) = error.downcast_ref::<TmHmLearnError>() else {
        return false;
    };
    matches!(
        tmhm_error,
        TmHmLearnError::CannotLearn { .. }
            | TmHmLearnError::AlreadyKnows { .. }
            | TmHmLearnError::MoveListFull
    )
}

fn handle_visible_field_item_refusal(
    runtime_shell: &mut BevyRuntimeShell,
    item_id: &str,
    error: anyhow::Error,
) -> Result<()> {
    runtime_shell
        .last_audio_events
        .push(format!("field item {item_id} refused: {error}"));
    runtime_shell.field_notice = Some("It won't have any effect.".to_string());
    mark_runtime_snapshot_dirty(runtime_shell);
    set_shell_action_status(runtime_shell, format!("{item_id} WON'T HAVE ANY EFFECT"));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn handle_visible_field_action_refusal(
    runtime_shell: &mut BevyRuntimeShell,
    action_id: &str,
    status: impl Into<String>,
    error: anyhow::Error,
) -> Result<()> {
    let status = status.into();
    runtime_shell
        .last_audio_events
        .push(format!("field action {action_id} refused: {error}"));
    runtime_shell.field_notice = Some(if status == "NOT EVEN A NIBBLE" {
        "Not even a nibble!".to_string()
    } else {
        "Can't use that here.".to_string()
    });
    mark_runtime_snapshot_dirty(runtime_shell);
    set_shell_action_status(runtime_shell, status);
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn item_targets_party_move_fields(
    pp_restore_scope: Option<&str>,
    pp_restore_points: Option<u8>,
    pp_up_stages: Option<u8>,
) -> bool {
    pp_up_stages.is_some() || (pp_restore_scope == Some("MOVE") && pp_restore_points.is_some())
}

fn item_targets_party_pokemon_fields(item: &crate::RuntimeItemCatalogSnapshot) -> bool {
    item.revive_hp_percent.is_some()
        || !item.status_heals.is_empty()
        || item.confusion_heal == Some(true)
        || (item.pp_restore_scope.as_deref() == Some("ALL")
            && item.pp_restore_points.is_some())
        || item.vitamin_stat.is_some()
        || item.rare_candy_level_gain.is_some()
        || item.party_special_effect
}

fn item_targets_active_battle_pokemon_fields(item: &crate::RuntimeItemCatalogSnapshot) -> bool {
    item.battle_stat_boost_stat.is_some() || item.battle_focus_energy == Some(true)
}

fn open_visible_battle_pack_target(
    runtime_shell: &mut BevyRuntimeShell,
    mode: BattlePackTargetMode,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    if snapshot.battle.is_none() {
        return handle_visible_no_active_battle(runtime_shell, "party_move_item_target_open");
    }
    if snapshot.party.slots.is_empty() {
        runtime_shell.battle_pack_target_mode = None;
        runtime_shell.party_move_cursor = None;
        record_visible_runtime_action(
            runtime_shell,
            format!(
                "battle:pack:target:{}:empty_party",
                battle_pack_target_mode_label(mode)
            ),
        )?;
        runtime_shell
            .last_audio_events
            .push("party is empty".to_string());
        set_shell_action_status(runtime_shell, "NO POKEMON");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    normalize_visible_party_cursor(runtime_shell, &snapshot);
    if mode == BattlePackTargetMode::PartyMove {
        runtime_shell.party_move_cursor = None;
    }
    runtime_shell.battle_pack_target_mode = Some(mode);
    runtime_shell.last_audio_events.push(format!(
        "opened battle pack target {}",
        battle_pack_target_mode_label(mode)
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn open_selected_battle_party_item_target(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    if snapshot.battle.is_none() {
        return handle_visible_no_active_battle(runtime_shell, "party_item_target_open");
    }
    if carried_battle_usable_item_ids(&snapshot).is_empty() {
        runtime_shell.bag_cursor = None;
        runtime_shell.battle_pack_target_mode = None;
        record_visible_runtime_action(runtime_shell, "battle:item:party_target:no_items")?;
        runtime_shell
            .last_audio_events
            .push("bag has no carried item or ball".to_string());
        set_shell_action_status(runtime_shell, "NO BATTLE ITEMS");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    let item_id = selected_battle_bag_item_id(runtime_shell)?;
    let item = snapshot
        .items
        .iter()
        .find(|item| item.item_id == item_id)
        .with_context(|| format!("selected battle item {item_id} is missing from item catalog"))?;
    if !item.battle_usable || !item_targets_party_pokemon_fields(item) {
        record_visible_runtime_action(
            runtime_shell,
            format!("battle:item:{item_id}:party_target:no_action"),
        )?;
        runtime_shell.last_audio_events.push(format!(
            "selected battle item {item_id} does not target party Pokemon"
        ));
        set_shell_action_status(runtime_shell, format!("{item_id} CANNOT TARGET POKEMON"));
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    if item_targets_party_move_fields(
        item.pp_restore_scope.as_deref(),
        item.pp_restore_points,
        item.pp_up_stages,
    ) {
        record_visible_runtime_action(
            runtime_shell,
            format!("battle:item:{item_id}:pokemon_target:no_action"),
        )?;
        runtime_shell.last_audio_events.push(format!(
            "selected battle item {item_id} targets a party move"
        ));
        set_shell_action_status(runtime_shell, format!("{item_id} TARGETS A MOVE"));
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    open_visible_battle_pack_target(runtime_shell, BattlePackTargetMode::PartyPokemon)
}

fn open_selected_battle_party_move_item_target(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    if snapshot.battle.is_none() {
        return handle_visible_no_active_battle(runtime_shell, "party_move_item_target_open");
    }
    if carried_battle_usable_item_ids(&snapshot).is_empty() {
        runtime_shell.bag_cursor = None;
        runtime_shell.battle_pack_target_mode = None;
        record_visible_runtime_action(runtime_shell, "battle:item:move_target:no_items")?;
        runtime_shell
            .last_audio_events
            .push("bag has no carried battle-usable item".to_string());
        set_shell_action_status(runtime_shell, "NO BATTLE ITEMS");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    let item_id = selected_battle_bag_item_id(runtime_shell)?;
    let item = snapshot
        .items
        .iter()
        .find(|item| item.item_id == item_id)
        .with_context(|| format!("selected battle item {item_id} is missing from item catalog"))?;
    if !item.battle_usable {
        record_visible_runtime_action(
            runtime_shell,
            format!("battle:item:{item_id}:move_target:no_action"),
        )?;
        runtime_shell.last_audio_events.push(format!(
            "selected battle item {item_id} does not target party moves"
        ));
        set_shell_action_status(runtime_shell, format!("{item_id} CANNOT TARGET MOVES"));
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    if !item_targets_party_move_fields(
        item.pp_restore_scope.as_deref(),
        item.pp_restore_points,
        item.pp_up_stages,
    ) {
        record_visible_runtime_action(
            runtime_shell,
            format!("battle:item:{item_id}:move_target:no_action"),
        )?;
        runtime_shell.last_audio_events.push(format!(
            "selected battle item {item_id} targets party Pokemon, not moves"
        ));
        set_shell_action_status(runtime_shell, format!("{item_id} TARGETS POKEMON"));
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    open_visible_battle_pack_target(runtime_shell, BattlePackTargetMode::PartyMove)
}

fn close_visible_battle_pack_target(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    if runtime_shell.battle_pack_target_mode == Some(BattlePackTargetMode::PartyMove)
        && runtime_shell.party_move_cursor.take().is_some()
    {
        record_visible_runtime_action(runtime_shell, "battle:pack_target:party_move:back_to_party")?;
        set_shell_action_status(runtime_shell, "USE ON WHICH POKEMON");
        mark_runtime_snapshot_dirty(runtime_shell);
        return Ok(());
    }
    let Some(mode) = runtime_shell.battle_pack_target_mode.take() else {
        record_visible_runtime_action(runtime_shell, "battle:pack_target:close:none")?;
        runtime_shell
            .last_audio_events
            .push("no battle pack target mode is open".to_string());
        set_shell_action_status(runtime_shell, "NO BATTLE ITEM TARGET");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    };
    record_visible_runtime_action(
        runtime_shell,
        format!(
            "battle:pack_target:{}:close",
            battle_pack_target_mode_label(mode)
        ),
    )?;
    if mode == BattlePackTargetMode::PartyMove {
        runtime_shell.party_move_cursor = None;
    }
    runtime_shell.last_audio_events.push(format!(
        "closed battle pack target {}",
        battle_pack_target_mode_label(mode)
    ));
    set_shell_action_status(runtime_shell, "BATTLE ITEM TARGET CLOSED");
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn move_visible_battle_pack_target_cursor(
    runtime_shell: &mut BevyRuntimeShell,
    delta: isize,
) -> Result<()> {
    match runtime_shell.battle_pack_target_mode {
        Some(BattlePackTargetMode::PartyMove) => {
            let snapshot = runtime_shell.shell.snapshot()?;
            if snapshot.party.slots.is_empty() {
                runtime_shell.battle_pack_target_mode = None;
                runtime_shell.party_move_cursor = None;
                runtime_shell
                    .last_audio_events
                    .push("party is empty".to_string());
                set_shell_action_status(runtime_shell, "NO POKEMON");
                trim_event_log(&mut runtime_shell.last_audio_events);
                return Ok(());
            }
            if runtime_shell.party_move_cursor.is_some() {
                return move_visible_party_move_cursor(runtime_shell, delta);
            }
            let row_count = snapshot.party.slots.len() + 1;
            runtime_shell.party_cursor = wrapped_index(
                runtime_shell.party_cursor.min(row_count - 1),
                row_count,
                delta,
            );
            mark_runtime_snapshot_dirty(runtime_shell);
            Ok(())
        }
        Some(BattlePackTargetMode::PartyPokemon) => {
            let snapshot = runtime_shell.shell.snapshot()?;
            if snapshot.party.slots.is_empty() {
                runtime_shell.battle_pack_target_mode = None;
                runtime_shell
                    .last_audio_events
                    .push("party is empty".to_string());
                set_shell_action_status(runtime_shell, "NO POKEMON");
                trim_event_log(&mut runtime_shell.last_audio_events);
                return Ok(());
            }
            let row_count = snapshot.party.slots.len() + 1;
            runtime_shell.party_cursor = wrapped_index(
                runtime_shell.party_cursor.min(row_count - 1),
                row_count,
                delta,
            );
            mark_runtime_snapshot_dirty(runtime_shell);
            Ok(())
        }
        None => {
            record_visible_runtime_action(runtime_shell, "battle:pack_target:move:none")?;
            runtime_shell
                .last_audio_events
                .push("no battle pack target mode is open".to_string());
            set_shell_action_status(runtime_shell, "NO BATTLE ITEM TARGET");
            trim_event_log(&mut runtime_shell.last_audio_events);
            Ok(())
        }
    }
}

fn move_visible_battle_pack_target_secondary_cursor(
    runtime_shell: &mut BevyRuntimeShell,
    delta: isize,
) -> Result<()> {
    match runtime_shell.battle_pack_target_mode {
        Some(BattlePackTargetMode::PartyMove) => {
            let snapshot = runtime_shell.shell.snapshot()?;
            if snapshot.party.slots.is_empty() {
                runtime_shell.battle_pack_target_mode = None;
                runtime_shell.party_move_cursor = None;
                runtime_shell
                    .last_audio_events
                    .push("party is empty".to_string());
                set_shell_action_status(runtime_shell, "NO POKEMON");
                trim_event_log(&mut runtime_shell.last_audio_events);
                return Ok(());
            }
            if runtime_shell.party_move_cursor.is_some() {
                move_visible_party_move_cursor(runtime_shell, delta)
            } else {
                Ok(())
            }
        }
        Some(BattlePackTargetMode::PartyPokemon) => {
            let snapshot = runtime_shell.shell.snapshot()?;
            if snapshot.party.slots.is_empty() {
                runtime_shell.battle_pack_target_mode = None;
                runtime_shell
                    .last_audio_events
                    .push("party is empty".to_string());
                set_shell_action_status(runtime_shell, "NO POKEMON");
                trim_event_log(&mut runtime_shell.last_audio_events);
                return Ok(());
            }
            let row_count = snapshot.party.slots.len() + 1;
            runtime_shell.party_cursor = wrapped_index(
                runtime_shell.party_cursor.min(row_count - 1),
                row_count,
                delta,
            );
            mark_runtime_snapshot_dirty(runtime_shell);
            Ok(())
        }
        None => {
            record_visible_runtime_action(runtime_shell, "battle:pack_target:move_secondary:none")?;
            runtime_shell
                .last_audio_events
                .push("no battle pack target mode is open".to_string());
            set_shell_action_status(runtime_shell, "NO BATTLE ITEM TARGET");
            trim_event_log(&mut runtime_shell.last_audio_events);
            Ok(())
        }
    }
}

fn use_selected_battle_pack_target(
    runtime_shell: &mut BevyRuntimeShell,
    mode: BattlePackTargetMode,
) -> Result<()> {
    match mode {
        BattlePackTargetMode::PartyPokemon => use_selected_battle_party_item(runtime_shell),
        BattlePackTargetMode::PartyMove => {
            if runtime_shell.party_move_cursor.is_some() {
                return use_selected_battle_party_move_item(runtime_shell);
            }
            let snapshot = runtime_shell.shell.snapshot()?;
            if runtime_shell.party_cursor >= snapshot.party.slots.len() {
                return close_visible_battle_pack_target(runtime_shell);
            }
            let party_index = selected_party_index(runtime_shell)?;
            let selected = snapshot
                .party
                .slots
                .iter()
                .find(|slot| slot.index == party_index)
                .with_context(|| format!("selected party index {party_index} is not in the party"))?;
            if selected.pokemon.is_egg || selected.pokemon.species.id == "EGG" {
                record_visible_runtime_action(
                    runtime_shell,
                    format!("battle:item:party_move:{party_index}:egg_refused"),
                )?;
                runtime_shell
                    .battle_messages
                    .push_back("That can't be used\non an EGG.".to_string());
                runtime_shell.battle_message_scene = Some(Box::new(snapshot));
                mark_runtime_snapshot_dirty(runtime_shell);
                return Ok(());
            }
            if selected.pokemon.moves.is_empty() {
                runtime_shell
                    .battle_messages
                    .push_back("It won't have any effect.".to_string());
                runtime_shell.battle_message_scene = Some(Box::new(snapshot));
                mark_runtime_snapshot_dirty(runtime_shell);
                return Ok(());
            }
            visible_cursor_index(
                &mut runtime_shell.party_move_cursor,
                &party_move_cursor_surface_id(party_index),
                selected.pokemon.moves.len(),
            );
            mark_runtime_snapshot_dirty(runtime_shell);
            Ok(())
        }
    }
}

fn battle_pack_target_mode_label(mode: BattlePackTargetMode) -> &'static str {
    match mode {
        BattlePackTargetMode::PartyPokemon => "party Pokemon",
        BattlePackTargetMode::PartyMove => "party move",
    }
}

fn use_selected_battle_party_item(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    if snapshot.battle.is_none() {
        return handle_visible_no_active_battle(runtime_shell, "party_item");
    }
    if snapshot.party.slots.is_empty() {
        runtime_shell.battle_pack_target_mode = None;
        record_visible_runtime_action(runtime_shell, "battle:item:party:empty_party")?;
        runtime_shell
            .last_audio_events
            .push("party is empty".to_string());
        set_shell_action_status(runtime_shell, "NO POKEMON");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    if runtime_shell.party_cursor >= snapshot.party.slots.len() {
        return close_visible_battle_pack_target(runtime_shell);
    }
    let party_index = selected_party_index(runtime_shell)?;
    use_selected_battle_party_item_on(runtime_shell, party_index)
}

fn use_selected_battle_party_item_on(
    runtime_shell: &mut BevyRuntimeShell,
    party_index: usize,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let target = snapshot
        .party
        .slots
        .iter()
        .find(|slot| slot.index == party_index)
        .with_context(|| format!("battle item target party index {party_index} is missing"))?;
    if target.pokemon.is_egg || target.pokemon.species.id == "EGG" {
        record_visible_runtime_action(
            runtime_shell,
            format!("battle:item:party:{party_index}:egg_refused"),
        )?;
        runtime_shell
            .battle_messages
            .push_back("That can't be used\non an EGG.".to_string());
        runtime_shell.battle_message_scene = Some(Box::new(snapshot));
        mark_runtime_snapshot_dirty(runtime_shell);
        set_shell_action_status(runtime_shell, "THAT CAN'T BE USED ON AN EGG");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    if carried_battle_usable_item_ids(&snapshot).is_empty() {
        runtime_shell.bag_cursor = None;
        runtime_shell.battle_pack_target_mode = None;
        record_visible_runtime_action(runtime_shell, "battle:item:party:no_items")?;
        runtime_shell
            .last_audio_events
            .push("bag has no carried battle-usable item".to_string());
        set_shell_action_status(runtime_shell, "NO BATTLE ITEMS");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    let item_id = selected_battle_bag_item_id(runtime_shell)?;
    let item = snapshot
        .items
        .iter()
        .find(|item| item.item_id == item_id)
        .with_context(|| format!("selected battle item {item_id} is missing from item catalog"))?;
    record_visible_runtime_action(
        runtime_shell,
        format!("battle:item:{item_id}:party:{party_index}"),
    )?;
    let use_result = if item.pp_restore_scope.as_deref() == Some("ALL")
        && item.pp_restore_points.is_some()
    {
        runtime_shell
            .shell
            .use_bag_item_on_battle_party_move(&item_id, party_index, None)
    } else {
        runtime_shell
            .shell
            .use_bag_item_on_battle_party_pokemon(&item_id, party_index)
    };
    let used = match use_result {
        Ok(used) => used,
        Err(error) if battle_item_error_is_play_refusal(&error) => {
            return handle_visible_battle_item_refusal(runtime_shell, &item_id, error);
        }
        Err(error) => return Err(error),
    };
    record_visible_battle_item_action_frame(runtime_shell, &item_id)?;
    reset_visible_battle_action_cursors(runtime_shell);
    runtime_shell.last_audio_events.push(format!(
        "battle party item item={} party_index={} item_use={:?} battle_item={:?} checksum={:?}",
        item_id, party_index, used.item_use, used.battle_item, used.state_checksum
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    set_shell_action_status(
        runtime_shell,
        format!("USED {item_id} ON PARTY #{party_index}"),
    );
    stage_visible_battle_item_effect(
        runtime_shell,
        &snapshot,
        &used.battle_item,
        Some(party_index),
    )?;
    resolve_visible_battle_enemy_response_after_player_item(runtime_shell, &item_id)?;
    Ok(())
}

fn use_selected_battle_party_move_item(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    if snapshot.battle.is_none() {
        return handle_visible_no_active_battle(runtime_shell, "party_move_item");
    }
    if snapshot.party.slots.is_empty() {
        runtime_shell.battle_pack_target_mode = None;
        runtime_shell.party_move_cursor = None;
        record_visible_runtime_action(runtime_shell, "battle:item:party_move:empty_party")?;
        runtime_shell
            .last_audio_events
            .push("party is empty".to_string());
        set_shell_action_status(runtime_shell, "NO POKEMON");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    let party_index = selected_party_index(runtime_shell)?;
    use_selected_battle_party_move_item_on(runtime_shell, party_index)
}

fn use_selected_battle_party_move_item_on(
    runtime_shell: &mut BevyRuntimeShell,
    party_index: usize,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    if carried_battle_usable_item_ids(&snapshot).is_empty() {
        runtime_shell.bag_cursor = None;
        runtime_shell.battle_pack_target_mode = None;
        runtime_shell.party_move_cursor = None;
        record_visible_runtime_action(runtime_shell, "battle:item:party_move:no_items")?;
        runtime_shell
            .last_audio_events
            .push("bag has no carried battle-usable item".to_string());
        set_shell_action_status(runtime_shell, "NO BATTLE ITEMS");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    let move_slot = selected_party_move_slot(runtime_shell, party_index)?;
    let item_id = selected_battle_bag_item_id(runtime_shell)?;
    record_visible_runtime_action(
        runtime_shell,
        format!("battle:item:{item_id}:party:{party_index}:move:{move_slot}"),
    )?;
    let used = match runtime_shell.shell.use_bag_item_on_battle_party_move(
        &item_id,
        party_index,
        Some(move_slot),
    ) {
        Ok(used) => used,
        Err(error) if battle_item_error_is_play_refusal(&error) => {
            return handle_visible_battle_item_refusal(runtime_shell, &item_id, error);
        }
        Err(error) => return Err(error),
    };
    record_visible_battle_item_action_frame(runtime_shell, &item_id)?;
    reset_visible_battle_action_cursors(runtime_shell);
    runtime_shell.last_audio_events.push(format!(
        "battle party move item item={} party_index={} move_slot={} item_use={:?} battle_item={:?} checksum={:?}",
        item_id, party_index, move_slot, used.item_use, used.battle_item, used.state_checksum
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    set_shell_action_status(
        runtime_shell,
        format!(
            "USED {item_id} ON PARTY #{party_index} MOVE {}",
            move_slot + 1
        ),
    );
    stage_visible_battle_item_effect(
        runtime_shell,
        &snapshot,
        &used.battle_item,
        Some(party_index),
    )?;
    resolve_visible_battle_enemy_response_after_player_item(runtime_shell, &item_id)?;
    Ok(())
}

fn use_selected_battle_escape_item(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let item_id = selected_carried_battle_item_matching(
        runtime_shell,
        |item| item.battle_escape_mode.is_some(),
        "bag has no carried battle escape item",
    )?;
    use_battle_escape_item_by_id(runtime_shell, &item_id)
}

fn use_battle_escape_item_by_id(runtime_shell: &mut BevyRuntimeShell, item_id: &str) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let Some(battle) = snapshot.battle.as_ref() else {
        return handle_visible_no_active_battle(runtime_shell, "escape_item");
    };
    let scripted_static_wild = visible_static_wild_source(&snapshot, battle);
    record_visible_runtime_action(runtime_shell, format!("battle:item:{item_id}:escape"))?;
    let used = match runtime_shell
        .shell
        .use_bag_item_to_escape_active_wild_battle(item_id)
    {
        Ok(used) => used,
        Err(error) if battle_item_error_is_play_refusal(&error) => {
            return handle_visible_battle_item_refusal(runtime_shell, item_id, error);
        }
        Err(error) => return Err(error),
    };
    record_visible_battle_item_action_frame(runtime_shell, item_id)?;
    runtime_shell.battle_messages.push_back(format!(
        "{} used the {}.",
        snapshot.trainer.player_name,
        item_display_name(&snapshot, item_id)
    ));
    runtime_shell.battle_message_scene = Some(Box::new(snapshot.clone()));
    mark_runtime_snapshot_dirty(runtime_shell);
    if used.escaped {
        queue_visible_pay_day_payout(runtime_shell, &snapshot);
        finish_visible_wild_battle_exit(runtime_shell, scripted_static_wild, "battle_escape")?;
    } else {
        reset_visible_battle_action_cursors(runtime_shell);
        resolve_visible_battle_enemy_response_after_player_item(runtime_shell, item_id)?;
    }
    runtime_shell.last_audio_events.push(format!(
        "battle escape item item={} item_use={:?} mode={:?} escaped={} checksum={:?}",
        item_id, used.item_use, used.battle_escape_mode, used.escaped, used.state_checksum
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    set_shell_action_status(runtime_shell, format!("{item_id} ESCAPED={}", used.escaped));
    Ok(())
}

fn use_selected_guard_spec(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let item_id = selected_carried_battle_item_matching(
        runtime_shell,
        |item| item.battle_stat_drop_guard == Some(true),
        "bag has no carried Guard Spec item",
    )?;
    use_guard_spec_by_id(runtime_shell, &item_id)
}

fn use_guard_spec_by_id(runtime_shell: &mut BevyRuntimeShell, item_id: &str) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    if snapshot.battle.is_none() {
        return handle_visible_no_active_battle(runtime_shell, "guard_spec");
    }
    record_visible_runtime_action(runtime_shell, format!("battle:item:{item_id}:guard_spec"))?;
    let used = match runtime_shell
        .shell
        .use_bag_guard_spec_in_active_battle(item_id)
    {
        Ok(used) => used,
        Err(error) if battle_item_error_is_play_refusal(&error) => {
            return handle_visible_battle_item_refusal(runtime_shell, item_id, error);
        }
        Err(error) => return Err(error),
    };
    record_visible_battle_item_action_frame(runtime_shell, item_id)?;
    reset_visible_battle_action_cursors(runtime_shell);
    runtime_shell.last_audio_events.push(format!(
        "guard spec item={} item_use={:?} guard_turns {}->{} checksum={:?}",
        item_id,
        used.item_use,
        used.stat_drop_guard_turns_before,
        used.stat_drop_guard_turns_after,
        used.state_checksum
    ));
    set_shell_action_status(
        runtime_shell,
        format!("GUARD SPEC {} TURNS", used.stat_drop_guard_turns_after),
    );
    stage_visible_battle_item_use(runtime_shell, item_id)?;
    resolve_visible_battle_enemy_response_after_player_item(runtime_shell, item_id)?;
    Ok(())
}

fn start_or_complete_visible_scripted_wild_battle(
    runtime_shell: &mut BevyRuntimeShell,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    if let Some(battle) = snapshot.battle {
        let RuntimeBattleKind::StaticWild { source_script, .. } = battle.kind else {
            anyhow::bail!("active battle is not a scripted wild battle");
        };
        complete_visible_scripted_wild_battle(
            runtime_shell,
            &snapshot.overworld.map_name,
            &source_script,
        )?;
        return Ok(());
    }
    let key = runtime_shell
        .shell
        .scripted_wild_battle_keys()
        .into_iter()
        .find(|key| key.map_name == snapshot.overworld.map_name)
        .with_context(|| {
            format!(
                "map {} has no compiled scripted wild battle",
                snapshot.overworld.map_name
            )
        })?;
    record_visible_runtime_action(
        runtime_shell,
        format!(
            "battle:start_static_wild:{}:{}:{}",
            key.map_name.as_str(),
            key.source_script.as_str(),
            key.startbattle_command_index
        ),
    )?;
    let start = runtime_shell.shell.start_scripted_wild_battle(
        &key.map_name,
        &key.source_script,
        key.startbattle_command_index,
    )?;
    prepare_visible_battle_entry(runtime_shell)?;
    runtime_shell.last_audio_events.push(format!(
        "scripted wild start source={} species={} level={} start={:?}",
        key.source_script, key.species, key.level, start
    ));
    Ok(())
}

fn start_or_complete_visible_scripted_trainer_battle(
    runtime_shell: &mut BevyRuntimeShell,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    if let Some(battle) = snapshot.battle {
        let RuntimeBattleKind::Trainer { source_script, .. } = battle.kind else {
            anyhow::bail!("active battle is not a scripted trainer battle");
        };
        complete_visible_scripted_trainer_battle(
            runtime_shell,
            &snapshot.overworld.map_name,
            &source_script,
            true,
            false,
        )?;
        return Ok(());
    }
    let key = runtime_shell
        .shell
        .scripted_trainer_battle_keys()
        .into_iter()
        .find(|key| key.map_name == snapshot.overworld.map_name)
        .with_context(|| {
            format!(
                "map {} has no compiled scripted trainer battle",
                snapshot.overworld.map_name
            )
        })?;
    record_visible_runtime_action(
        runtime_shell,
        format!(
            "battle:start_trainer:{}:{}:{}",
            key.map_name.as_str(),
            key.source_script.as_str(),
            key.startbattle_command_index
        ),
    )?;
    let start = runtime_shell.shell.start_scripted_trainer_battle(
        &key.map_name,
        &key.source_script,
        key.startbattle_command_index,
    )?;
    prepare_visible_battle_entry(runtime_shell)?;
    runtime_shell.last_audio_events.push(format!(
        "scripted trainer start source={} trainer={}:{} start={:?}",
        key.source_script, key.trainer_class, key.trainer_id, start
    ));
    Ok(())
}

fn complete_visible_scripted_wild_battle(
    runtime_shell: &mut BevyRuntimeShell,
    map_name: &str,
    source_script: &str,
) -> Result<()> {
    let key = runtime_shell
        .shell
        .scripted_wild_battle_keys()
        .into_iter()
        .find(|key| key.map_name == map_name && key.source_script == source_script)
        .with_context(|| {
            format!(
                "compiled scripted wild battle key missing for {} on {}",
                source_script, map_name
            )
        })?;
    record_visible_runtime_action(
        runtime_shell,
        format!(
            "battle:complete_static_wild:{}:{}:{}",
            key.map_name.as_str(),
            key.source_script.as_str(),
            key.startbattle_command_index
        ),
    )?;
    let next_cursor = Some(RuntimeCompiledScriptCursor {
        origin_map_name: key.map_name.clone(),
        source_script: key.source_script.clone(),
        command_index: key.startbattle_command_index + 1,
    });
    let completed = runtime_shell
        .shell
        .complete_scripted_wild_battle_and_run_compiled_script(
            &key.map_name,
            &key.source_script,
            key.startbattle_command_index,
            next_cursor,
            256,
            ScriptRuntimeInputs::default(),
            ScriptPhoneInputs::default(),
        )?;
    let completion = completed.completion;
    runtime_shell.last_audio_events.push(format!(
        "scripted wild complete source={} reload={} effects={:?} resumed_steps={} checksum={:?}",
        key.source_script,
        key.reload_map_after_battle,
        completion.effects,
        completed.run.steps.len(),
        completion.state_checksum
    ));
    if key.reload_map_after_battle {
        reset_visible_map_reload_after_battle(runtime_shell, "scripted_wild_battle_reload");
    }
    let reached_boundary =
        integrate_visible_compiled_script_run(runtime_shell, &completed.run.steps)?;
    arm_visible_active_script_cursor_from_run(runtime_shell, completed.run.next_cursor);
    if reached_boundary {
        return Ok(());
    }
    continue_visible_script_after_prompt(runtime_shell)?;
    queue_visible_current_music(runtime_shell)?;
    Ok(())
}

fn complete_visible_scripted_trainer_battle(
    runtime_shell: &mut BevyRuntimeShell,
    map_name: &str,
    source_script: &str,
    won: bool,
    can_lose: bool,
) -> Result<()> {
    let battle_before_completion = runtime_shell.shell.snapshot()?;
    let player_name = battle_before_completion.trainer.player_name.clone();
    let key = runtime_shell
        .shell
        .scripted_trainer_battle_keys()
        .into_iter()
        .find(|key| key.map_name == map_name && key.source_script == source_script)
        .with_context(|| {
            format!(
                "compiled scripted trainer battle key missing for {} on {}",
                source_script, map_name
            )
        })?;
    record_visible_runtime_action(
        runtime_shell,
        format!(
            "battle:complete_trainer:{}:{}:{}:{}:{}",
            key.map_name.as_str(),
            key.source_script.as_str(),
            key.startbattle_command_index,
            won,
            can_lose
        ),
    )?;
    let next_cursor = Some(RuntimeCompiledScriptCursor {
        origin_map_name: key.map_name.clone(),
        source_script: key.source_script.clone(),
        command_index: key.startbattle_command_index + 1,
    });
    let completed = runtime_shell
        .shell
        .complete_scripted_trainer_battle_and_run_compiled_script(
            &key.map_name,
            &key.source_script,
            key.startbattle_command_index,
            won,
            can_lose,
            next_cursor,
            256,
            ScriptRuntimeInputs::default(),
            ScriptPhoneInputs::default(),
        )?;
    let completion = completed.completion;
    if won && let Some(prize_money) = completion.trainer_prize_money {
        runtime_shell.battle_messages.push_back(format!(
            "{player_name} got ¥{prize_money}\nfor winning!"
        ));
        if runtime_shell.shell.snapshot()?.trainer.moms_money
            > battle_before_completion.trainer.moms_money
        {
            runtime_shell
                .battle_messages
                .push_back("Sent some to MOM!".to_string());
        }
        mark_runtime_snapshot_dirty(runtime_shell);
    }
    queue_visible_pay_day_payout(runtime_shell, &battle_before_completion);
    runtime_shell.last_audio_events.push(format!(
        "scripted trainer complete source={} reload={} effects={:?} resumed_steps={} checksum={:?}",
        key.source_script,
        key.reload_map_after_battle,
        completion.effects,
        completed.run.steps.len(),
        completion.state_checksum
    ));
    if completion.continued_after_battle {
        if key.reload_map_after_battle {
            reset_visible_map_reload_after_battle(runtime_shell, "scripted_trainer_battle_reload");
        }
        let reached_boundary =
            integrate_visible_compiled_script_run(runtime_shell, &completed.run.steps)?;
        arm_visible_active_script_cursor_from_run(runtime_shell, completed.run.next_cursor);
        if reached_boundary {
            return Ok(());
        }
        continue_visible_script_after_prompt(runtime_shell)?;
        queue_visible_current_music(runtime_shell)?;
    } else {
        queue_visible_current_music(runtime_shell)?;
    }
    Ok(())
}

fn resolve_visible_battle_move(runtime_shell: &mut BevyRuntimeShell, slot: usize) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let Some(ref battle) = snapshot.battle else {
        return handle_visible_no_active_battle(runtime_shell, "move_slot");
    };
    let battle_before_turn = battle.clone();
    if visible_active_battle_player_fainted(&snapshot) {
        record_visible_runtime_action(runtime_shell, "battle:move:fainted_replacement_required")?;
        runtime_shell
            .last_audio_events
            .push("active battle Pokemon is fainted; choose a replacement".to_string());
        runtime_shell
            .battle_messages
            .push_back("Choose a POKéMON to continue!".to_string());
        runtime_shell.battle_message_scene = Some(Box::new(snapshot.clone()));
        mark_runtime_snapshot_dirty(runtime_shell);
        set_shell_action_status(runtime_shell, "CHOOSE NEXT POKEMON");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    let move_selection_bypassed = battle.commands.player_forced_struggle
        || battle.commands.player_turn_automatic
        || battle.commands.player_fight_automatic;
    if !move_selection_bypassed
        && !battle.commands.player_move_slots.contains(&slot)
    {
        record_visible_runtime_action(runtime_shell, format!("battle:move:{slot}:unavailable"))?;
        runtime_shell
            .last_audio_events
            .push(format!("player move slot {slot} is not available"));
        set_shell_action_status(runtime_shell, "MOVE UNAVAILABLE");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    if !move_selection_bypassed {
        let selected_move = battle
            .player_moves
            .get(slot)
            .with_context(|| format!("selected move slot {slot} is missing"))?;
        if battle.player_disabled_move.as_deref() == Some(selected_move.name.as_str()) {
            record_visible_runtime_action(runtime_shell, format!("battle:move:{slot}:disabled"))?;
            runtime_shell
                .battle_messages
                .push_back("The move is DISABLED!".to_string());
            runtime_shell.battle_message_scene = Some(Box::new(snapshot.clone()));
            mark_runtime_snapshot_dirty(runtime_shell);
            set_shell_action_status(runtime_shell, "THE MOVE IS DISABLED");
            trim_event_log(&mut runtime_shell.last_audio_events);
            return Ok(());
        }
        if selected_move.current_pp == 0
            && battle.player_moves.iter().any(|learned| {
                learned.current_pp > 0
                    && battle.player_disabled_move.as_deref() != Some(learned.name.as_str())
            })
        {
            record_visible_runtime_action(runtime_shell, format!("battle:move:{slot}:no_pp"))?;
            runtime_shell
                .battle_messages
                .push_back("There's no PP left for this move!".to_string());
            runtime_shell.battle_message_scene = Some(Box::new(snapshot.clone()));
            mark_runtime_snapshot_dirty(runtime_shell);
            set_shell_action_status(runtime_shell, "THERE'S NO PP LEFT FOR THIS MOVE");
            trim_event_log(&mut runtime_shell.last_audio_events);
            return Ok(());
        }
    }
    let (enemy_action, enemy_rng_seed_after) =
        selected_enemy_battle_action(&snapshot, battle, &mut runtime_shell.trainer_items_used)?;
    persist_selected_enemy_trainer_item(runtime_shell, battle, &enemy_action)?;
    let enemy_slot = match &enemy_action {
        BattleAction::Move { slot } => Some(*slot),
        BattleAction::Switch { .. } => None,
        _ => None,
    };
    record_visible_runtime_action(
        runtime_shell,
        format!("battle:move:{slot}:enemy:{enemy_action:?}"),
    )?;
    record_visible_battle_action_frame(runtime_shell, BattleAction::Move { slot })?;
    let turn = resolve_active_battle_turn_with_enemy_rng(
        runtime_shell,
        enemy_rng_seed_after,
        BattleAction::Move { slot },
        enemy_action,
    )?;
    reset_visible_battle_action_cursors(runtime_shell);
    stage_visible_battle_messages(runtime_shell, &snapshot, &turn.outcome.events);
    let events = format_battle_turn_events(&turn.outcome.events);
    runtime_shell.last_audio_events.push(format!(
        "battle move player_slot={} enemy_slot={} {} events={} checksum={:?}",
        slot,
        enemy_slot.map_or_else(|| "switch".to_string(), |slot| slot.to_string()),
        format_battle_turn_summary(&turn.outcome),
        events,
        turn.state_checksum
    ));
    set_shell_action_status(
        runtime_shell,
        format!(
            "BATTLE MOVE {} {}",
            slot + 1,
            format_battle_turn_summary(&turn.outcome)
        ),
    );
    settle_visible_resolved_battle_turn(runtime_shell, &battle_before_turn)
}

fn visible_battle_action_ids(
    snapshot: &RuntimeShellSnapshot,
    battle: &crate::RuntimeBattleSnapshot,
) -> Vec<VisibleBattleAction> {
    if visible_active_battle_player_fainted(snapshot) {
        let mut actions = Vec::new();
        if !battle.commands.switch_party_indices.is_empty() {
            actions.push(VisibleBattleAction::Pokemon);
        }
        return actions;
    }
    // Crystal's main battle menu is structurally fixed. Availability is
    // checked after selection (no switch target, link-item refusal, trainer
    // escape refusal); hiding entries changes both cursor geometry and input.
    vec![
        VisibleBattleAction::Fight,
        VisibleBattleAction::Pokemon,
        VisibleBattleAction::Pack,
        VisibleBattleAction::Run,
    ]
}

fn sync_visible_battle_action_cursor(runtime_shell: &mut BevyRuntimeShell) {
    let snapshot = match runtime_shell.shell.snapshot() {
        Ok(snapshot) => snapshot,
        Err(error) => {
            runtime_shell.last_error = Some(error.to_string());
            return;
        }
    };
    let Some(battle) = snapshot.battle.as_ref() else {
        runtime_shell.battle_action_cursor = None;
        runtime_shell.battle_move_cursor = None;
        runtime_shell.battle_move_swap_origin = None;
        runtime_shell.battle_switch_cursor = None;
        runtime_shell.battle_party_action_cursor = None;
        runtime_shell.battle_party_summary_open = false;
        runtime_shell.pending_battle_move_switch_slot = None;
        runtime_shell.party_move_cursor = None;
        runtime_shell.battle_pack_target_mode = None;
        if runtime_shell
            .bag_cursor
            .as_ref()
            .is_some_and(|cursor| cursor.surface_id == "battle:bag-items")
        {
            runtime_shell.bag_cursor = None;
        }
        if runtime_shell.ball_cursor.is_some() && runtime_shell.field_pack_pocket.is_none() {
            runtime_shell.ball_cursor = None;
        }
        return;
    };
    if runtime_shell.field_pack_pocket.is_some() {
        runtime_shell.field_pack_pocket = None;
        runtime_shell.field_pack_action_cursor = None;
        runtime_shell.field_pack_target_mode = None;
        runtime_shell.battle_pack_target_mode = None;
        runtime_shell.bag_cursor = None;
        runtime_shell.key_item_cursor = None;
        runtime_shell.ball_cursor = None;
        runtime_shell.tmhm_cursor = None;
        runtime_shell.custom_item_cursor = None;
    }
    if visible_active_battle_player_fainted(&snapshot) {
        reset_visible_battle_item_cursors(runtime_shell);
        runtime_shell.battle_move_cursor = None;
        runtime_shell.battle_move_swap_origin = None;
        runtime_shell.party_move_cursor = None;
        if battle.commands.switch_party_indices.is_empty() {
            runtime_shell.battle_action_cursor = None;
            runtime_shell.battle_switch_cursor = None;
            runtime_shell.pending_battle_move_switch_slot = None;
            return;
        }
        if runtime_shell.battle_faint_prompt_cursor.is_some() {
            runtime_shell.battle_action_cursor = None;
            runtime_shell.battle_switch_cursor = None;
            return;
        }
        runtime_shell.battle_action_cursor = Some(MenuCursor {
            surface_id: "battle:actions".to_string(),
            option_index: 0,
        });
        visible_cursor_index(
            &mut runtime_shell.battle_switch_cursor,
            "battle:switch",
            battle_switch_option_count(&snapshot),
        );
        return;
    }
    let actions = visible_battle_action_ids(&snapshot, battle);
    if actions.is_empty() {
        runtime_shell.battle_action_cursor = None;
        return;
    }
    visible_cursor_index(
        &mut runtime_shell.battle_action_cursor,
        "battle:actions",
        actions.len(),
    );
}

fn selected_visible_battle_action(
    runtime_shell: &mut BevyRuntimeShell,
    snapshot: &RuntimeShellSnapshot,
    battle: &crate::RuntimeBattleSnapshot,
) -> Result<VisibleBattleAction> {
    let actions = visible_battle_action_ids(snapshot, battle);
    if actions.is_empty() {
        anyhow::bail!("active battle has no available player action");
    }
    let index = visible_cursor_index(
        &mut runtime_shell.battle_action_cursor,
        "battle:actions",
        actions.len(),
    );
    Ok(actions[index])
}

fn selected_visible_battle_action_readonly(
    runtime_shell: &BevyRuntimeShell,
    snapshot: &RuntimeShellSnapshot,
    battle: &crate::RuntimeBattleSnapshot,
) -> Result<VisibleBattleAction> {
    let actions = visible_battle_action_ids(snapshot, battle);
    if actions.is_empty() {
        anyhow::bail!("active battle has no available player action");
    }
    let index = strict_readonly_cursor_index(
        &runtime_shell.battle_action_cursor,
        "battle:actions",
        actions.len(),
    )
    .context("battle action surface is active without a valid cursor")?;
    Ok(actions[index])
}

fn select_visible_battle_action(
    runtime_shell: &mut BevyRuntimeShell,
    action: VisibleBattleAction,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let Some(battle) = snapshot.battle.as_ref() else {
        return handle_visible_no_active_battle(runtime_shell, "press_a");
    };
    let actions = visible_battle_action_ids(&snapshot, battle);
    let index = actions
        .iter()
        .position(|candidate| *candidate == action)
        .with_context(|| format!("visible battle action {:?} is not available", action))?;
    runtime_shell.battle_action_cursor = Some(MenuCursor {
        surface_id: "battle:actions".to_string(),
        option_index: index,
    });
    Ok(())
}

fn finish_visible_wild_battle_with_first_move(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    for _ in 0..32 {
        let snapshot = runtime_shell.shell.snapshot()?;
        let Some(battle) = snapshot.battle.as_ref() else {
            reset_visible_battle_exit_state(runtime_shell);
            return Ok(());
        };
        if battle.enemy_pokemon.hp == 0 {
            press_visible_battle_a_button(runtime_shell)?;
            continue;
        }
        if visible_active_battle_player_fainted(&snapshot) {
            anyhow::bail!("visible wild battle smoke player Pokemon fainted before battle ended");
        }
        select_visible_battle_action(runtime_shell, VisibleBattleAction::Fight)?;
        if runtime_shell.battle_move_cursor.is_none() {
            press_visible_battle_a_button(runtime_shell)?;
        }
        press_visible_battle_a_button(runtime_shell)?;
    }
    anyhow::bail!("visible wild battle smoke did not finish within 32 turns")
}

fn finish_visible_overworld_random_battle(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    finish_visible_wild_battle_with_first_move(runtime_shell)
}

fn visible_battle_action_label(action: VisibleBattleAction) -> &'static str {
    match action {
        VisibleBattleAction::Fight => "Fight",
        VisibleBattleAction::Pokemon => "Pokemon",
        VisibleBattleAction::Pack => "Pack",
        VisibleBattleAction::Run => "Run",
    }
}

fn open_visible_battle_pack(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let Some(battle) = snapshot.battle.as_ref() else {
        return handle_visible_no_active_battle(runtime_shell, "pack_open");
    };
    if visible_active_battle_player_fainted(&snapshot) {
        record_visible_runtime_action(runtime_shell, "battle:pack:fainted_replacement_required")?;
        runtime_shell
            .last_audio_events
            .push("active battle Pokemon is fainted; choose a replacement".to_string());
        runtime_shell
            .battle_messages
            .push_back("Choose a POKéMON to continue!".to_string());
        runtime_shell.battle_message_scene = Some(Box::new(snapshot.clone()));
        mark_runtime_snapshot_dirty(runtime_shell);
        set_shell_action_status(runtime_shell, "CHOOSE NEXT POKEMON");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    if !battle.commands.can_use_items {
        record_visible_runtime_action(runtime_shell, "battle:pack:items_unavailable")?;
        runtime_shell
            .last_audio_events
            .push("active battle does not allow item use".to_string());
        runtime_shell
            .battle_messages
            .push_back("Items can't be used here.".to_string());
        runtime_shell.battle_message_scene = Some(Box::new(snapshot.clone()));
        mark_runtime_snapshot_dirty(runtime_shell);
        set_shell_action_status(runtime_shell, "ITEMS UNAVAILABLE");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    let initial_pocket = if FIELD_PACK_POCKETS.contains(&runtime_shell.last_field_pack_pocket) {
        runtime_shell.last_field_pack_pocket.clone()
    } else {
        FieldPackPocket::Items
    };
    let item_ids = carried_battle_non_ball_item_ids(&snapshot);
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
    runtime_shell.ball_cursor = None;
    runtime_shell.key_item_cursor = None;
    runtime_shell.tmhm_cursor = None;
    runtime_shell.custom_item_cursor = None;
    runtime_shell.field_pack_pocket = None;
    runtime_shell.field_pack_action_cursor = None;
    runtime_shell.field_pack_target_mode = None;
    runtime_shell.battle_pack_target_mode = None;
    runtime_shell.party_move_cursor = None;
    if initial_pocket != FieldPackPocket::Items {
        let pocket_index = FIELD_PACK_POCKETS
            .iter()
            .position(|pocket| *pocket == initial_pocket)
            .context("retained battle Pack pocket is not standard")?;
        shift_visible_battle_pack_pocket(runtime_shell, pocket_index as isize)?;
    }
    runtime_shell.last_audio_events.push(format!(
        "opened battle Pack pocket {}",
        field_pack_pocket_label(&active_visible_field_pack_pocket(runtime_shell))
    ));
    set_shell_action_status(
        runtime_shell,
        format!(
            "BATTLE PACK {}",
            field_pack_pocket_label(&active_visible_field_pack_pocket(runtime_shell))
        ),
    );
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn press_visible_battle_a_button(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    if runtime_shell.battle_party_summary_open {
        if !visible_wait_sfx_finished(runtime_shell) {
            return Ok(());
        }
        let snapshot = runtime_shell.shell.snapshot()?;
        let slot = selected_party_slot_snapshot(&snapshot, runtime_shell.party_cursor)?;
        if slot.pokemon.is_egg || runtime_shell.party_summary_page >= 3 {
            runtime_shell.battle_party_summary_open = false;
            runtime_shell.party_summary_open = false;
            runtime_shell.party_summary_page = 1;
            runtime_shell.battle_party_action_cursor = None;
            mark_runtime_snapshot_dirty(runtime_shell);
            return Ok(());
        }
        return cycle_visible_party_summary_page(runtime_shell, 1);
    }
    if runtime_shell.battle_party_action_cursor.is_some() {
        return execute_visible_battle_party_action(runtime_shell);
    }
    if runtime_shell.battle_faint_prompt_cursor.is_some() {
        return confirm_visible_wild_faint_prompt(runtime_shell);
    }
    if runtime_shell.battle_shift_prompt_cursor.is_some() {
        return confirm_visible_trainer_shift_prompt(runtime_shell);
    }
    let snapshot = runtime_shell.shell.snapshot()?;
    let Some(battle) = snapshot.battle.as_ref() else {
        return handle_visible_no_active_battle(runtime_shell, "press_a");
    };
    if visible_active_battle_player_fainted(&snapshot) {
        reset_visible_battle_item_cursors(runtime_shell);
        if runtime_shell.battle_switch_cursor.is_some() {
            return switch_visible_battle_pokemon(runtime_shell);
        }
        return handle_visible_player_fainted_battle_boundary(runtime_shell, &snapshot, battle);
    }
    if battle.commands.can_use_items {
        if runtime_shell.field_pack_action_cursor.is_some() {
            return execute_visible_battle_pack_action(runtime_shell);
        }
        if runtime_shell.ball_cursor.is_some() {
            let ball_ids = carried_ball_item_ids(&snapshot);
            if strict_readonly_cursor_index(
                &runtime_shell.ball_cursor,
                "bag:balls",
                field_pack_selectable_count(ball_ids.len()),
            ) == Some(ball_ids.len()) {
                record_visible_runtime_action(runtime_shell, "battle:pack:cancel")?;
                reset_visible_battle_item_cursors(runtime_shell);
                set_shell_action_status(runtime_shell, "BATTLE");
                return Ok(());
            }
            return open_visible_battle_pack_action_menu(runtime_shell);
        }
        if runtime_shell.bag_cursor.is_some() {
            let item_ids = carried_battle_non_ball_item_ids(&snapshot);
            if strict_readonly_cursor_index(
                &runtime_shell.bag_cursor,
                "battle:bag-items",
                field_pack_selectable_count(item_ids.len()),
            ) == Some(item_ids.len()) {
                record_visible_runtime_action(runtime_shell, "battle:pack:cancel")?;
                reset_visible_battle_item_cursors(runtime_shell);
                set_shell_action_status(runtime_shell, "BATTLE");
                return Ok(());
            }
            if let Some(mode) = runtime_shell.battle_pack_target_mode {
                return use_selected_battle_pack_target(runtime_shell, mode);
            }
            return open_visible_battle_pack_action_menu(runtime_shell);
        }
        if runtime_shell.key_item_cursor.is_some() || runtime_shell.tmhm_cursor.is_some() {
            let (cursor, surface_id, item_ids) = if runtime_shell.key_item_cursor.is_some() {
                (
                    &runtime_shell.key_item_cursor,
                    "bag:key-items",
                    snapshot.bag.key_items.iter().filter(|item| item.quantity > 0)
                        .map(|item| item.item_id.clone()).collect::<Vec<_>>(),
                )
            } else {
                (
                    &runtime_shell.tmhm_cursor,
                    "bag:tmhm",
                    snapshot.bag.tm_hm.iter().filter(|item| item.quantity > 0)
                        .map(|item| item.item_id.clone()).collect::<Vec<_>>(),
                )
            };
            let selected = strict_readonly_cursor_index(
                cursor, surface_id, field_pack_selectable_count(item_ids.len()),
            );
            if selected == Some(item_ids.len()) {
                record_visible_runtime_action(runtime_shell, "battle:pack:cancel")?;
                reset_visible_battle_item_cursors(runtime_shell);
                set_shell_action_status(runtime_shell, "BATTLE");
                return Ok(());
            }
            return open_visible_battle_pack_action_menu(runtime_shell);
        }
    }
    if battle.enemy_pokemon.hp == 0 {
        return match battle.kind {
            RuntimeBattleKind::Trainer { .. } => {
                let Some(enemy_index) = battle.active_enemy_party_index else {
                    anyhow::bail!("active trainer battle has no active enemy party index");
                };
                if battle.rewarded_enemy_party_indices.contains(&enemy_index) {
                    if runtime_shell.battle_switch_cursor.is_some()
                        && trainer_shift_switch_pending(&snapshot, battle)
                    {
                        return switch_visible_battle_pokemon(runtime_shell);
                    }
                    advance_visible_trainer_battle(runtime_shell)
                } else {
                    claim_visible_battle_rewards(runtime_shell)
                }
            }
            RuntimeBattleKind::Wild { .. } | RuntimeBattleKind::StaticWild { .. } => {
                claim_visible_battle_rewards(runtime_shell)
            }
        };
    }
    let actions = visible_battle_action_ids(&snapshot, &battle);
    if actions.is_empty() {
        runtime_shell.battle_action_cursor = None;
        record_visible_runtime_action(runtime_shell, "battle:a:no_actions")?;
        runtime_shell
            .last_audio_events
            .push("active battle has no available player action".to_string());
        set_shell_action_status(runtime_shell, "NO BATTLE ACTION");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    let selected_action = selected_visible_battle_action(runtime_shell, &snapshot, &battle)?;
    if runtime_shell.battle_move_cursor.is_none()
        && runtime_shell.battle_switch_cursor.is_none()
    {
        // The main 2D battle menu owns its confirmation click. Submenus own
        // their later confirmations independently; in particular the FIGHT
        // row must not double the post-MoveSelectionScreen click.
        queue_visible_shell_sound_effect(runtime_shell, "SFX_READ_TEXT_2")?;
    }
    match selected_action {
        VisibleBattleAction::Fight => {
            // BattleMenu_Fight clears wNumFleeAttempts before returning to
            // ParsePlayerAction. This happens on entering FIGHT, even when
            // MoveSelectionScreen is subsequently canceled.
            runtime_shell
                .shell
                .session_mut()
                .state
                .battle_escape_attempts = 0;
            if battle.commands.player_forced_struggle
                || battle.commands.player_turn_automatic
                || battle.commands.player_fight_automatic
            {
                resolve_visible_battle_move(runtime_shell, 0)
            } else if runtime_shell.battle_move_cursor.is_some() {
                resolve_visible_selected_battle_move(runtime_shell)
            } else {
                open_visible_battle_move_target(runtime_shell)
            }
        }
        VisibleBattleAction::Pokemon => {
            if runtime_shell.battle_switch_cursor.is_some() {
                open_visible_battle_party_action_menu(runtime_shell)
            } else {
                open_visible_battle_switch_target(runtime_shell)
            }
        }
        VisibleBattleAction::Pack if battle.battle_type == "BATTLETYPE_TUTORIAL" => {
            // TutorialPack swaps to a temporary BALL pocket and the ASM
            // Right+A stream selects its sole POKE BALL.
            throw_visible_battle_ball_id(runtime_shell, 0, "POKE_BALL".to_string())
        }
        VisibleBattleAction::Pack
            if matches!(
                battle.battle_type.as_str(),
                "BATTLETYPE_CONTEST" | "BATTLETYPE_BUG_CONTEST" | "BATTLETYPE_PARK"
            ) =>
        {
            if snapshot.bug_contest.park_balls_remaining == 0 {
                record_visible_runtime_action(runtime_shell, "battle:park_ball:none")?;
                runtime_shell
                    .last_audio_events
                    .push("You're out of PARK BALLs!".to_string());
                runtime_shell
                    .battle_messages
                    .push_back("You're out of PARK BALLs!".to_string());
                runtime_shell.battle_message_scene = Some(Box::new(snapshot.clone()));
                mark_runtime_snapshot_dirty(runtime_shell);
                set_shell_action_status(runtime_shell, "OUT OF PARK BALLS");
                trim_event_log(&mut runtime_shell.last_audio_events);
                return Ok(());
            }
            throw_visible_battle_ball_id(runtime_shell, 0, "PARK_BALL".to_string())
        }
        VisibleBattleAction::Pack => open_visible_battle_pack(runtime_shell),
        VisibleBattleAction::Run => attempt_visible_battle_run(runtime_shell),
    }
}

fn press_visible_battle_b_button(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    if runtime_shell.battle_party_summary_open {
        if !visible_wait_sfx_finished(runtime_shell) {
            return Ok(());
        }
        runtime_shell.battle_party_summary_open = false;
        runtime_shell.party_summary_open = false;
        runtime_shell.party_summary_page = 1;
        runtime_shell.battle_party_action_cursor = None;
        mark_runtime_snapshot_dirty(runtime_shell);
        return Ok(());
    }
    if runtime_shell.battle_party_action_cursor.is_some() {
        queue_visible_shell_sound_effect(runtime_shell, "SFX_READ_TEXT_2")?;
        runtime_shell.battle_party_action_cursor = None;
        mark_runtime_snapshot_dirty(runtime_shell);
        return Ok(());
    }
    if runtime_shell.battle_faint_prompt_cursor.is_some() {
        let selected = strict_readonly_cursor_index(
            &runtime_shell.battle_faint_prompt_cursor,
            "battle:faint-prompt",
            2,
        )
        .context("wild faint prompt requires a valid YES/NO cursor")?;
        if selected == 0 {
            record_visible_runtime_action(runtime_shell, "battle:faint_prompt:b_on_yes_ignored")?;
            return Ok(());
        }
        return resolve_visible_wild_faint_prompt(runtime_shell, false);
    }
    if runtime_shell.battle_shift_prompt_cursor.is_some() {
        return resolve_visible_trainer_shift_prompt(runtime_shell, false);
    }
    let snapshot = runtime_shell.shell.snapshot()?;
    let Some(battle) = snapshot.battle.as_ref() else {
        return handle_visible_no_active_battle(runtime_shell, "press_b");
    };
    if visible_active_battle_player_fainted(&snapshot) {
        record_visible_runtime_action(runtime_shell, "battle:replacement_required:cancel_ignored")?;
        reset_visible_battle_item_cursors(runtime_shell);
        if runtime_shell.battle_switch_cursor.is_none() {
            handle_visible_player_fainted_battle_boundary(runtime_shell, &snapshot, battle)?;
        }
        runtime_shell
            .last_audio_events
            .push("replacement required; battle cancel ignored".to_string());
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    if runtime_shell.battle_pack_target_mode.is_some() {
        return close_visible_battle_pack_target(runtime_shell);
    }
    if runtime_shell.field_pack_action_cursor.is_some() {
        record_visible_runtime_action(runtime_shell, "battle:pack:actions:close")?;
        close_visible_field_pack_action_menu(runtime_shell);
        return Ok(());
    }
    if runtime_shell.bag_cursor.is_some()
        || runtime_shell.ball_cursor.is_some()
        || runtime_shell.key_item_cursor.is_some()
        || runtime_shell.tmhm_cursor.is_some()
    {
        record_visible_runtime_action(runtime_shell, "battle:item_menu:close")?;
        reset_visible_battle_item_cursors(runtime_shell);
        runtime_shell
            .last_audio_events
            .push("closed battle item cursor".to_string());
        set_shell_action_status(runtime_shell, "BATTLE ITEM CLOSED");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    if runtime_shell.battle_move_cursor.is_some() || runtime_shell.battle_switch_cursor.is_some() {
        queue_visible_shell_sound_effect(runtime_shell, "SFX_READ_TEXT_2")?;
        let kept_current_pokemon = runtime_shell.battle_switch_cursor.is_some()
            && trainer_shift_switch_pending(&snapshot, battle);
        runtime_shell.battle_move_cursor = None;
        runtime_shell.battle_move_swap_origin = None;
        runtime_shell.battle_switch_cursor = None;
        runtime_shell.pending_battle_move_switch_slot = None;
        runtime_shell.last_audio_events.push(
            if kept_current_pokemon {
                "trainer shift kept current Pokemon"
            } else {
                "closed battle submenu"
            }
            .to_string(),
        );
        if kept_current_pokemon {
            record_visible_runtime_action(runtime_shell, "battle:shift_keep_current")?;
            set_shell_action_status(runtime_shell, "SHIFT: KEEP CURRENT");
            trim_event_log(&mut runtime_shell.last_audio_events);
            return advance_visible_trainer_battle(runtime_shell);
        }
        record_visible_runtime_action(runtime_shell, "battle:submenu:close")?;
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    record_visible_runtime_action(runtime_shell, "battle:action_menu:cancel_ignored")?;
    runtime_shell
        .last_audio_events
        .push("battle cancel at action menu".to_string());
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn open_visible_battle_party_action_menu(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let selected = strict_readonly_cursor_index(
        &runtime_shell.battle_switch_cursor,
        "battle:switch",
        battle_switch_option_count(&snapshot),
    )
    .context("battle party list requires a valid cursor")?;
    if selected >= snapshot.party.slots.len() {
        return press_visible_battle_b_button(runtime_shell);
    }
    runtime_shell.battle_party_action_cursor = Some(MenuCursor {
        surface_id: "battle:party-actions".to_string(),
        option_index: 0,
    });
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(())
}

fn execute_visible_battle_party_action(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let selected = strict_readonly_cursor_index(
        &runtime_shell.battle_party_action_cursor,
        "battle:party-actions",
        3,
    )
    .context("battle party action menu requires a valid cursor")?;
    queue_visible_shell_sound_effect(runtime_shell, "SFX_READ_TEXT_2")?;
    match selected {
        0 => {
            runtime_shell.battle_party_action_cursor = None;
            switch_visible_battle_pokemon(runtime_shell)
        }
        1 => {
            let snapshot = runtime_shell.shell.snapshot()?;
            let selected_party_slot = strict_readonly_cursor_index(
                &runtime_shell.battle_switch_cursor,
                "battle:switch",
                battle_switch_option_count(&snapshot),
            )
            .context("battle party stats requires a selected Pokemon")?;
            let slot = snapshot
                .party
                .slots
                .get(selected_party_slot)
                .context("battle party stats selected CANCEL")?;
            runtime_shell.party_cursor = selected_party_slot;
            runtime_shell.party_summary_page = 1;
            runtime_shell.party_summary_open = true;
            runtime_shell.battle_party_summary_open = true;
            queue_visible_pokemon_cry(
                runtime_shell,
                &slot.pokemon.species.id,
                "battle_party_summary",
            )?;
            mark_runtime_snapshot_dirty(runtime_shell);
            Ok(())
        }
        2 => {
            runtime_shell.battle_party_action_cursor = None;
            mark_runtime_snapshot_dirty(runtime_shell);
            Ok(())
        }
        _ => unreachable!("battle party action cursor is bounded to three entries"),
    }
}

fn attempt_visible_battle_run(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let Some(battle) = snapshot.battle.as_ref() else {
        return handle_visible_no_active_battle(runtime_shell, "run");
    };
    if matches!(battle.kind, RuntimeBattleKind::Trainer { .. }) {
        // BattleMenu_Run refuses trainer battles immediately. It does not
        // submit a turn or allow the opponent to attack.
        record_visible_runtime_action(runtime_shell, "battle:run:trainer_refused")?;
        let refusal = "No! There's no\nrunning from a\ntrainer battle!".to_string();
        runtime_shell.last_audio_events.push(refusal.clone());
        runtime_shell.battle_messages.push_back(refusal);
        runtime_shell.battle_message_scene = Some(Box::new(snapshot.clone()));
        mark_runtime_snapshot_dirty(runtime_shell);
        set_shell_action_status(runtime_shell, "NO RUNNING FROM TRAINER BATTLE");
        // BattleMenu_Run loops back through BattleMenu without clearing
        // wBattleMenuCursorPosition, so RUN remains selected after refusal.
        runtime_shell.battle_move_cursor = None;
        reset_visible_battle_item_cursors(runtime_shell);
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    let battle_before_run = battle.clone();
    let scripted_static_wild = visible_static_wild_source(&snapshot, battle);
    let (enemy_action, enemy_rng_seed_after) =
        selected_enemy_battle_action(&snapshot, battle, &mut runtime_shell.trainer_items_used)?;
    persist_selected_enemy_trainer_item(runtime_shell, battle, &enemy_action)?;
    let enemy_action_label = format!("enemy:{enemy_action:?}");
    record_visible_runtime_action(runtime_shell, "battle:run")?;
    record_visible_battle_action_frame(runtime_shell, BattleAction::Run)?;
    let turn = resolve_active_battle_turn_with_enemy_rng(
        runtime_shell,
        enemy_rng_seed_after,
        BattleAction::Run,
        enemy_action,
    )?;
    reset_visible_battle_action_cursors(runtime_shell);
    stage_visible_battle_messages(runtime_shell, &snapshot, &turn.outcome.events);
    let events = format_battle_turn_events(&turn.outcome.events);
    runtime_shell.last_audio_events.push(format!(
        "battle run {enemy_action_label} {} events={} checksum={:?}",
        format_battle_turn_summary(&turn.outcome),
        events,
        turn.state_checksum
    ));
    set_shell_action_status(
        runtime_shell,
        format!("RUN {}", format_battle_turn_summary(&turn.outcome)),
    );
    if runtime_shell.shell.snapshot()?.battle.is_some() {
        settle_visible_battle_after_action(runtime_shell)?;
        let settled = runtime_shell.shell.snapshot()?;
        if settled.battle.as_ref().is_some_and(|battle| {
            battle.enemy_pokemon.hp > 0
                && battle.active_player_party_index.is_some_and(|active| {
                    settled
                        .party
                        .slots
                        .iter()
                        .find(|slot| slot.index == active)
                        .is_some_and(|slot| slot.pokemon.hp > 0)
                })
        }) {
            // A failed wild escape returns to BattleMenu with its retained RUN
            // position. Do this after settlement, which rebuilds action cursors.
            // A resulting faint instead owns the forced replacement boundary.
            runtime_shell.battle_action_cursor = Some(MenuCursor {
                surface_id: "battle:actions".to_string(),
                option_index: 3,
            });
        }
    }
    finish_visible_inactive_battle_after_turn(
        runtime_shell,
        &battle_before_run,
        scripted_static_wild,
        "battle_run_turn_exit",
    )?;
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn resolve_visible_selected_battle_move(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let Some(battle) = snapshot.battle.as_ref() else {
        return handle_visible_no_active_battle(runtime_shell, "selected_move");
    };
    // ParsePlayerAction calls PlayClickSFX immediately after
    // MoveSelectionScreen returns, before it branches on cancellation or
    // resolves the selected move. Keep this separate from cursor movement
    // and from HP-palette changes, both of which are silent here.
    queue_visible_shell_sound_effect(runtime_shell, "SFX_READ_TEXT_2")?;
    if selected_battle_move_cursor_is_cancel(runtime_shell, &snapshot, battle)? {
        // The selected CANCEL row returns from MoveSelectionScreen with A;
        // ParsePlayerAction owns the click queued above. Close directly so
        // the physical-B path's own MenuClickSound is not played a second time.
        runtime_shell.battle_move_cursor = None;
        runtime_shell.battle_move_swap_origin = None;
        runtime_shell.pending_battle_move_switch_slot = None;
        record_visible_runtime_action(runtime_shell, "battle:move_menu:cancel_row")?;
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    let slot = selected_battle_move_slot(runtime_shell)?;
    if selected_battle_move_effect(&snapshot, slot)? == "BATON_PASS" {
        return open_visible_battle_move_switch_target(runtime_shell, slot);
    }
    resolve_visible_battle_move(runtime_shell, slot)
}

fn selected_battle_move_effect(snapshot: &RuntimeShellSnapshot, slot: usize) -> Result<&str> {
    let Some(battle) = snapshot.battle.as_ref() else {
        anyhow::bail!("no active battle");
    };
    let learned = battle
        .player_moves
        .get(slot)
        .with_context(|| format!("active battle move slot {slot} is missing"))?;
    snapshot
        .moves
        .iter()
        .find(|catalog| catalog.move_id == learned.name)
        .map(|catalog| catalog.effect.as_str())
        .with_context(|| format!("compiled move catalog missing {}", learned.name))
}

fn selected_battle_move_slot(runtime_shell: &mut BevyRuntimeShell) -> Result<usize> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let Some(battle) = snapshot.battle else {
        anyhow::bail!("no active battle");
    };
    if battle.commands.player_move_slots.is_empty() {
        anyhow::bail!("active battle has no available player moves");
    }
    let cursor_index = visible_cursor_index(
        &mut runtime_shell.battle_move_cursor,
        "battle:moves",
        battle.commands.player_move_slots.len(),
    );
    Ok(battle.commands.player_move_slots[cursor_index])
}

fn selected_battle_move_cursor_is_cancel(
    runtime_shell: &mut BevyRuntimeShell,
    snapshot: &RuntimeShellSnapshot,
    battle: &crate::RuntimeBattleSnapshot,
) -> Result<bool> {
    let total = battle_move_menu_option_count(snapshot, battle)
        .with_context(|| "active battle has no move menu option count")?;
    let cursor_index =
        visible_cursor_index(&mut runtime_shell.battle_move_cursor, "battle:moves", total);
    Ok(cursor_index >= battle.commands.player_move_slots.len())
}

fn selected_enemy_battle_move_slot_with_rng(
    snapshot: &RuntimeShellSnapshot,
    battle: &crate::RuntimeBattleSnapshot,
    rng: &mut Random,
) -> Result<usize> {
    let commands = &battle.commands;
    if commands.enemy_move_slots.is_empty() {
        // ParseEnemyAction selects STRUGGLE before wild slot sampling or
        // trainer AI scoring when every move is absent, exhausted, or
        // disabled. Slot zero is only the synthetic action carrier; core
        // turn resolution does not index it for automatic STRUGGLE.
        return Ok(0);
    }
    let Some(ai_flags) = (match &battle.kind {
        RuntimeBattleKind::Trainer { ai_move_flags, .. } => Some(*ai_move_flags),
        RuntimeBattleKind::Wild { .. } | RuntimeBattleKind::StaticWild { .. } => None,
    }) else {
        let usable_slots = commands
            .enemy_move_slots
            .iter()
            .copied()
            .filter(|slot| {
                battle
                    .enemy_moves
                    .get(*slot)
                    .is_some_and(|learned| learned.current_pp > 0)
            })
            .collect::<BTreeSet<_>>();
        loop {
            let slot = usize::from(rng.battle_random_byte() & 3);
            if usable_slots.contains(&slot) {
                return Ok(slot);
            }
        }
    };

    // Crystal initializes every move at score 20, applies the enabled trainer
    // scoring layers, then chooses uniformly among the lowest scores.  This
    // is intentionally expressed as a score table so additional ASM layers
    // can be added without returning to uniform random selection.
    let player_defender_types: [Option<&str>; 2] = battle
        .active_player_party_index
        .and_then(|active_index| {
            snapshot
                .party
                .slots
                .iter()
                .find(|slot| slot.index == active_index)
        })
        .map(|slot| {
            [
                Some(slot.pokemon.species.type1.as_str()),
                Some(slot.pokemon.species.type2.as_str()),
            ]
        })
        .unwrap_or([Some("NONE"), Some("NONE")]);
    let usable_slots = commands
        .enemy_move_slots
        .iter()
        .copied()
        .filter(|slot| {
            battle
                .enemy_moves
                .get(*slot)
                .is_some_and(|learned| learned.current_pp > 0)
        })
        .collect::<BTreeSet<_>>();
    let mut scored = Vec::with_capacity(battle.enemy_moves.len().min(4));
    for slot in 0..battle.enemy_moves.len().min(4) {
        let move_data = battle.enemy_moves.get(slot).and_then(|known| {
            snapshot
                .moves
                .iter()
                .find(|candidate| candidate.name == known.name || candidate.move_id == known.name)
        });
        let move_is_usable = usable_slots.contains(&slot);
        let mut score = if move_is_usable { 20i16 } else { 80i16 };
        if let Some(move_data) = move_data.filter(|_| move_is_usable) {
            let is_status = move_data.power == 0;
            // ASM: AI_BASIC dismisses sleep/toxic/poison/paralyze when the
            // player already has any status condition.  These effects are
            // represented by their exact compiled move-effect tokens; do not
            // treat every zero-power move as status-only (screens, weather,
            // and setup moves remain eligible for later AI layers).
            if ai_flags & (1 << 0) != 0
                && is_status_only_ai_effect(move_data.effect.as_str())
                && player_has_status(snapshot, battle)
            {
                score += 10;
            }
            if !is_status {
                let mut effectiveness_num = 1u32;
                let mut effectiveness_den = 1u32;
                for defender_type in player_defender_types
                    .iter()
                    .copied()
                    .flatten()
                    .filter(|defender_type| *defender_type != "NONE")
                {
                    if let Some(multiplier) = snapshot
                        .battle_rules
                        .type_effectiveness
                        .matchups
                        .get(&move_data.move_type)
                        .and_then(|defenders| defenders.get(defender_type))
                    {
                        effectiveness_num =
                            effectiveness_num.saturating_mul(u32::from(multiplier.numerator));
                        effectiveness_den = effectiveness_den
                            .saturating_mul(u32::from(multiplier.denominator.max(1)));
                    }
                }
                if effectiveness_num == 0 {
                    score += 40;
                } else if effectiveness_num > effectiveness_den {
                    score -= 5;
                } else if effectiveness_num < effectiveness_den {
                    score += 5;
                }
            }
            if ai_flags & (1 << 3) != 0 {
                score -= (move_data.power / 10).min(12) as i16;
            }
            if ai_flags & (1 << 6) != 0 {
                score -= (move_data.power / 8).min(16) as i16;
            }
            if ai_flags & (1 << 8) != 0 && is_status {
                score -= 6;
            }
            if ai_flags & (1 << 4) != 0 && move_data.accuracy >= 90 {
                score -= 2;
            }
            if ai_flags & (1 << 9) != 0 && move_data.accuracy < 80 {
                score += 4;
            }
        } else if move_is_usable {
            score += 60;
        }
        scored.push((slot, score));
    }
    let best_score = scored.iter().map(|(_, score)| *score).min().unwrap_or(20);
    let best = scored
        .iter()
        .filter_map(|(slot, score)| (*score == best_score).then_some(*slot))
        .collect::<BTreeSet<_>>();
    loop {
        let slot = usize::from(rng.battle_random_byte() & 3);
        if best.contains(&slot) {
            return Ok(slot);
        }
    }
}

fn is_status_only_ai_effect(effect: &str) -> bool {
    matches!(
        effect,
        "SLEEP"
            | "TOXIC"
            | "POISON"
            | "PARALYZE"
            | "EFFECT_SLEEP"
            | "EFFECT_TOXIC"
            | "EFFECT_POISON"
            | "EFFECT_PARALYZE"
    )
}

fn player_has_status(
    snapshot: &RuntimeShellSnapshot,
    battle: &crate::RuntimeBattleSnapshot,
) -> bool {
    battle
        .active_player_party_index
        .and_then(|active| {
            snapshot
                .party
                .slots
                .iter()
                .find(|slot| slot.index == active)
        })
        .and_then(|slot| slot.pokemon.status.as_deref())
        .is_some_and(|status| !status.is_empty() && status != "NONE")
}

fn selected_enemy_battle_action(
    snapshot: &RuntimeShellSnapshot,
    battle: &crate::RuntimeBattleSnapshot,
    trainer_items_used: &mut BTreeSet<String>,
) -> Result<(BattleAction, u32)> {
    sync_persisted_trainer_item_usage(battle, trainer_items_used);
    let RuntimeBattleKind::Trainer {
        trainer_id,
        ai_item_switch_flags,
        ..
    } = &battle.kind
    else {
        let mut rng = Random::new_crystal(snapshot.progression.rng_seed);
        if wild_enemy_should_flee(battle, &mut rng) {
            return Ok((BattleAction::Run, rng.seed()));
        }
        let slot = selected_enemy_battle_move_slot_with_rng(snapshot, battle, &mut rng)?;
        return Ok((BattleAction::Move { slot }, rng.seed()));
    };
    let switch_flags = if battle.battle_type == "BATTLETYPE_BATTLE_TOWER" {
        snapshot
            .trainers
            .iter()
            .find(|trainer| trainer.trainer_class == "FALKNER")
            .context("Battle Tower AI requires trainer class 1 FALKNER attributes")?
            .ai_item_switch_flags
    } else {
        *ai_item_switch_flags
    };

    // CheckEnemyLockedIn returns from AI_SwitchOrTryItem entirely. Unlike
    // the later DontSwitch branches, it suppresses trainer items as well as
    // switching and proceeds directly to the retained move.
    if battle.enemy_switch_locked {
        let slot = battle
            .commands
            .enemy_move_slots
            .first()
            .copied()
            .unwrap_or(0);
        return Ok((BattleAction::Move { slot }, snapshot.progression.rng_seed));
    }

    // Player CANT_RUN and enemy Wrap take the ASM DontSwitch path. Trainer
    // items remain eligible there; only the switch branch is suppressed.
    let switch_blocked = battle.player_cannot_escape || battle.enemy_wrapped;

    let Some((party_index, switch_tier)) = (!switch_blocked)
        .then(|| trainer_switch_candidate(snapshot, battle))
        .transpose()?
        .flatten()
    else {
        let (item, item_seed) = selected_enemy_trainer_item(
            snapshot, battle, trainer_id, &switch_flags, trainer_items_used,
            snapshot.progression.rng_seed,
        )?;
        if let Some(item_id) = item {
            return Ok((BattleAction::Item { item_id }, item_seed));
        }
        let mut rng = Random::new_crystal(item_seed);
        let slot = selected_enemy_battle_move_slot_with_rng(snapshot, battle, &mut rng)?;
        return Ok((BattleAction::Move { slot }, rng.seed()));
    };

    // TrainerClassAttributes encodes SWITCH_OFTEN, SWITCH_RARELY, and
    // SWITCH_SOMETIMES in bits 0..2. Crystal only evaluates this path for a
    // low-HP active mon; use the same 8-bit BattleRandom stream so a switch
    // consumes the boundary sample before the turn is resolved.
    let switch_mask = switch_flags & 0x07;
    if switch_mask == 0 {
        let (item, item_seed) = selected_enemy_trainer_item(
            snapshot, battle, trainer_id, &switch_flags, trainer_items_used,
            snapshot.progression.rng_seed,
        )?;
        if let Some(item_id) = item {
            return Ok((BattleAction::Item { item_id }, item_seed));
        }
        let mut rng = Random::new_crystal(item_seed);
        let slot = selected_enemy_battle_move_slot_with_rng(snapshot, battle, &mut rng)?;
        return Ok((BattleAction::Move { slot }, rng.seed()));
    }
    let mut rng = Random::new_crystal(snapshot.progression.rng_seed);
    // Trainer switch thresholds use BattleRandom (hRandomSub), not
    // RandomRange's hRandomAdd register.  Using randrange(256) here shifts
    // the AI's decision stream and changes both switch odds and every later
    // battle RNG call.
    let roll = u32::from(rng.battle_random_byte());
    let should_switch = if switch_mask & 0x01 != 0 {
        match switch_tier { 0x10 => roll < 128, 0x20 => roll < 200, _ => roll >= 10 }
    } else if switch_mask & 0x02 != 0 {
        match switch_tier { 0x10 => roll < 20, 0x20 => roll < 30, _ => roll >= 200 }
    } else {
        match switch_tier { 0x10 => roll < 50, 0x20 => roll < 128, _ => roll >= 50 }
    };
    if should_switch {
        return Ok((BattleAction::Switch { party_index }, rng.seed()));
    }
    let (item, item_seed) = selected_enemy_trainer_item(
        snapshot, battle, trainer_id, &switch_flags, trainer_items_used, rng.seed(),
    )?;
    if let Some(item_id) = item {
        return Ok((BattleAction::Item { item_id }, item_seed));
    }
    let mut move_rng = Random::new_crystal(item_seed);
    let slot = selected_enemy_battle_move_slot_with_rng(snapshot, battle, &mut move_rng)?;
    Ok((BattleAction::Move { slot }, move_rng.seed()))
}

fn wild_enemy_should_flee(
    battle: &crate::RuntimeBattleSnapshot,
    rng: &mut Random,
) -> bool {
    const ALWAYS_FLEE: [&str; 2] = ["RAIKOU", "ENTEI"];
    const OFTEN_FLEE: [&str; 8] = [
        "CUBONE",
        "ARTICUNO",
        "ZAPDOS",
        "MOLTRES",
        "QUAGSIRE",
        "DELIBIRD",
        "PHANPY",
        "TEDDIURSA",
    ];
    const SOMETIMES_FLEE: [&str; 13] = [
        "MAGNEMITE",
        "GRIMER",
        "TANGELA",
        "MR_MIME",
        "EEVEE",
        "PORYGON",
        "DRATINI",
        "DRAGONAIR",
        "TOGETIC",
        "UMBREON",
        "UNOWN",
        "SNUBBULL",
        "HERACROSS",
    ];

    if battle.player_cannot_escape
        || battle.enemy_wrapped
        || matches!(battle.enemy_pokemon.status.as_deref(), Some("SLEEP" | "FREEZE"))
    {
        return false;
    }
    let species = battle.enemy_pokemon.species.id.as_str();
    if ALWAYS_FLEE.contains(&species) {
        return true;
    }
    let roll = rng.battle_random_byte();
    if roll >= 128 {
        return false;
    }
    if OFTEN_FLEE.contains(&species) {
        return true;
    }
    roll < 26 && SOMETIMES_FLEE.contains(&species)
}

fn sync_persisted_trainer_item_usage(
    battle: &crate::RuntimeBattleSnapshot,
    trainer_items_used: &mut BTreeSet<String>,
) {
    trainer_items_used.extend(battle.trainer_items_used.iter().cloned());
}

fn trainer_switch_candidate(
    snapshot: &RuntimeShellSnapshot,
    battle: &crate::RuntimeBattleSnapshot,
) -> Result<Option<(usize, u8)>> {
    let active_enemy_index = battle
        .active_enemy_party_index
        .context("trainer switch scoring is missing the active enemy party index")?;
    let active_enemy = battle
        .enemy_party
        .get(active_enemy_index)
        .context("trainer active enemy index is outside its party")?;
    let player = battle
        .active_player_party_index
        .and_then(|index| snapshot.party.slots.iter().find(|slot| slot.index == index))
        .map(|slot| &slot.pokemon)
        .context("trainer switch scoring is missing the active player Pokemon")?;
    let alive = battle
        .enemy_party
        .iter()
        .enumerate()
        .filter(|(index, pokemon)| {
            *index != active_enemy_index
                && pokemon.hp > 0
                && !pokemon.is_egg
                && pokemon.species.id != "EGG"
        })
        .map(|(index, pokemon)| (index, pokemon))
        .collect::<Vec<_>>();
    if alive.is_empty() {
        return Ok(None);
    }

    let last_player_move = battle
        .player_last_move
        .as_deref()
        .and_then(|move_id| snapshot.moves.iter().find(|data| {
            data.move_id == move_id || data.name == move_id
        }));
    let candidate_is_healthy = |pokemon: &crate::core::models::Pokemon| {
        pokemon.max_hp > 0 && u32::from(pokemon.hp) * 4 >= u32::from(pokemon.max_hp)
    };
    let resists_player = |pokemon: &crate::core::models::Pokemon| -> Result<bool> {
        if let Some(move_data) = last_player_move.filter(|data| data.power > 0) {
            return Ok(battle_ai_type_matchup(snapshot, &move_data.move_type, pokemon)? <= 0);
        }
        Ok(
            battle_ai_type_matchup(snapshot, &player.species.type1, pokemon)? <= 0
                && battle_ai_type_matchup(snapshot, &player.species.type2, pokemon)? <= 0,
        )
    };
    let has_super_effective_move = |pokemon: &crate::core::models::Pokemon| -> Result<bool> {
        for learned in &pokemon.moves {
            let move_data = snapshot
                .moves
                .iter()
                .find(|data| data.move_id == learned.name || data.name == learned.name)
                .with_context(|| format!("trainer switch move {} is missing", learned.name))?;
            if move_data.power > 0
                && battle_ai_type_matchup(snapshot, &move_data.move_type, player)? > 0
            {
                return Ok(true);
            }
        }
        Ok(false)
    };

    if active_enemy.perish_song_turns == 1 {
        for (index, pokemon) in &alive {
            if candidate_is_healthy(pokemon)
                && resists_player(pokemon)?
                && has_super_effective_move(pokemon)?
            {
                return Ok(Some((*index, 0x30)));
            }
        }
        return Ok(Some((alive[0].0, 0x30)));
    }

    let switch_score = |enemy: &crate::core::models::Pokemon,
                        enemy_moves: &[crate::core::models::LearnedMove]|
     -> Result<i8> {
        let mut score = 10_i8;
        if battle.player_used_moves.is_empty() {
            for player_type in [&player.species.type1, &player.species.type2] {
                if battle_ai_type_matchup(snapshot, player_type, enemy)? > 0 {
                    score -= 1;
                }
                if player.species.type1 == player.species.type2 {
                    break;
                }
            }
        } else {
            let mut best = 0_i8;
            for move_id in &battle.player_used_moves {
                let move_data = snapshot
                    .moves
                    .iter()
                    .find(|data| data.move_id == *move_id || data.name == *move_id)
                    .with_context(|| format!("used player move {move_id} is missing"))?;
                if move_data.power == 0 {
                    continue;
                }
                let matchup = battle_ai_type_matchup(snapshot, &move_data.move_type, enemy)?;
                if matchup > 0 {
                    score -= 1;
                    best = 2;
                    break;
                }
                if matchup == 0 {
                    best = 2;
                } else if matchup == -1 && best == 0 {
                    best = 1;
                }
            }
            if best != 2 {
                score += 1;
                if best == 0 {
                    score += 1;
                }
            }
        }

        // The ASM accumulates one point per resisted damaging move, five per
        // neutral move, and 100 for any super-effective move. This matters for
        // sets with several resisted attacks; reducing it to a best-matchup
        // comparison changes the switch threshold.
        let mut enemy_matchup_score = 0_u16;
        for learned in enemy_moves {
            let move_data = snapshot
                .moves
                .iter()
                .find(|data| data.move_id == learned.name || data.name == learned.name)
                .with_context(|| format!("trainer switch move {} is missing", learned.name))?;
            if move_data.power == 0 {
                continue;
            }
            match battle_ai_type_matchup(snapshot, &move_data.move_type, player)? {
                -2 => {}
                -1 => enemy_matchup_score = enemy_matchup_score.saturating_add(1),
                0 => enemy_matchup_score = enemy_matchup_score.saturating_add(5),
                _ => enemy_matchup_score = 100,
            }
        }
        if enemy_matchup_score == 0 {
            score -= 2;
        } else if enemy_matchup_score < 5 {
            score -= 1;
        } else if enemy_matchup_score >= 100 {
            score += 1;
        }
        Ok(score)
    };

    // CheckAbleToSwitch returns immediately only at 11 or higher. A score of
    // exactly 10 continues into candidate evaluation and is not itself enough
    // to trigger a switch.
    if switch_score(active_enemy, &battle.enemy_moves)? >= 11 {
        return Ok(None);
    }

    // wLastPlayerCounterMove takes precedence: Crystal first narrows the
    // party to Pokemon immune to that damaging move, then prefers one with a
    // super-effective attack (falling back to a neutral attacker). The
    // two-Pokemon case receives the stronger $20 tier when that candidate's
    // own matchup score is favorable.
    if let Some(last_move) = last_player_move.filter(|data| data.power > 0) {
        let mut neutral_immune = None;
        let mut super_immune = None;
        for (index, pokemon) in &alive {
            if battle_ai_type_matchup(snapshot, &last_move.move_type, pokemon)? != -2 {
                continue;
            }
            let mut has_neutral = false;
            for learned in &pokemon.moves {
                let move_data = snapshot
                    .moves
                    .iter()
                    .find(|data| data.move_id == learned.name || data.name == learned.name)
                    .with_context(|| format!("trainer switch move {} is missing", learned.name))?;
                if move_data.power == 0 {
                    continue;
                }
                match battle_ai_type_matchup(snapshot, &move_data.move_type, player)? {
                    matchup if matchup > 0 => {
                        super_immune.get_or_insert((*index, pokemon));
                        break;
                    }
                    matchup if matchup == 0 => has_neutral = true,
                    _ => {}
                }
            }
            if has_neutral {
                neutral_immune.get_or_insert((*index, pokemon));
            }
        }
        if let Some((index, pokemon)) = super_immune.or(neutral_immune) {
            let candidate_score = switch_score(pokemon, &pokemon.moves)?;
            if battle.enemy_party.len() == 2 {
                return Ok(Some((index, if candidate_score < 10 { 0x20 } else { 0x10 })));
            }
            if candidate_score < 10 {
                return Ok(Some((index, 0x10)));
            }
            return Ok(None);
        }
    }

    for (index, pokemon) in alive {
        if candidate_is_healthy(pokemon)
            && resists_player(pokemon)?
            && has_super_effective_move(pokemon)?
            && switch_score(pokemon, &pokemon.moves)? < 10
        {
            return Ok(Some((index, 0x10)));
        }
    }
    Ok(None)
}

fn battle_ai_type_matchup(
    snapshot: &RuntimeShellSnapshot,
    move_type: &str,
    defender: &crate::core::models::Pokemon,
) -> Result<i8> {
    let defender_types = if defender.species.type1 == defender.species.type2 {
        vec![defender.species.type1.clone()]
    } else {
        vec![defender.species.type1.clone(), defender.species.type2.clone()]
    };
    let multiplier = crate::core::battle::damage::calculate_type_effectiveness_multiplier(
        &snapshot.battle_rules.type_effectiveness,
        move_type,
        &defender_types,
    )
    .map_err(|error| anyhow::anyhow!("trainer switch type matchup: {error:?}"))?;
    if multiplier.numerator == 0 {
        return Ok(-2);
    }
    Ok(match multiplier.numerator.cmp(&multiplier.denominator) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    })
}

fn persist_selected_enemy_trainer_item(
    runtime_shell: &mut BevyRuntimeShell,
    battle: &crate::RuntimeBattleSnapshot,
    action: &BattleAction,
) -> Result<()> {
    let BattleAction::Item { item_id } = action else {
        return Ok(());
    };
    let RuntimeBattleKind::Trainer { trainer_id, .. } = &battle.kind else {
        return Ok(());
    };
    let prefix = format!("{trainer_id}:{item_id}:");
    let usage_keys = runtime_shell
        .trainer_items_used
        .iter()
        .filter(|key| key.starts_with(&prefix))
        .cloned()
        .collect::<Vec<_>>();
    // Item AI can run before the first damage turn has materialized the
    // combat record. Materialize it here from the authoritative active battle
    // rather than dropping the use or storing renderer-only state.
    let initial_combat = active_battle_combat_state(runtime_shell.shell.session().state())?;
    let state = runtime_shell.shell.session_mut().state_mut();
    let combat = state
        .script_runtime
        .active_battle_combat
        .get_or_insert(initial_combat);
    combat.trainer_items_used.extend(usage_keys);
    Ok(())
}

fn selected_enemy_trainer_item(
    snapshot: &RuntimeShellSnapshot,
    battle: &crate::RuntimeBattleSnapshot,
    trainer_id: &str,
    ai_item_switch_flags: &u32,
    trainer_items_used: &mut BTreeSet<String>,
    rng_seed: u32,
) -> Result<(Option<String>, u32)> {
    if battle.battle_type == "BATTLETYPE_BATTLE_TOWER" {
        return Ok((None, rng_seed));
    }
    let Some(trainer) = snapshot
        .trainers
        .iter()
        .find(|trainer| trainer.trainer_id == trainer_id)
    else {
        return Ok((None, rng_seed));
    };
    let active_index = battle
        .active_enemy_party_index
        .context("trainer item AI is missing the active enemy party index")?;
    let active = battle
        .enemy_party
        .get(active_index)
        .context("trainer item AI active enemy index is outside its party")?;
    if active.max_hp == 0 || active.hp == 0 {
        return Ok((None, rng_seed));
    }
    // AI_TryItem is gated by IsHighestLevel in the original engine.  A
    // trainer's consumables are reserved for its highest-level party member.
    if battle
        .enemy_party
        .iter()
        .any(|pokemon| pokemon.level > active.level)
    {
        return Ok((None, rng_seed));
    }

    // The ASM item table is ordered from strongest to weakest healing/status
    // item.  Only offer an item when it can change the active monster, so a
    // failed item attempt never consumes the trainer slot.
    let has_status = active
        .status
        .as_deref()
        .is_some_and(|status| !status.is_empty() && status != "NONE");
    let below_half = active.hp.saturating_mul(2) <= active.max_hp;
    let below_quarter = active.hp.saturating_mul(4) <= active.max_hp;
    let context_use = ai_item_switch_flags & (1 << 6) != 0;
    let always_use = ai_item_switch_flags & (1 << 4) != 0;
    let unknown_use = ai_item_switch_flags & (1 << 5) != 0;
    let mut rng = Random::new_crystal(rng_seed);
    let heal_item_usable = |rng: &mut Random| {
        if context_use {
            return below_quarter || (below_half && rng.battle_random_byte() < 50);
        }
        if unknown_use {
            return below_quarter && rng.battle_random_byte() >= 50;
        }
        below_quarter || (below_half && rng.battle_random_byte() < 128)
    };
    const TRAINER_AI_ITEM_ORDER: [&str; 13] = [
        "FULL_RESTORE", "MAX_POTION", "HYPER_POTION", "SUPER_POTION", "POTION",
        "X_ACCURACY", "FULL_HEAL", "GUARD_SPEC", "DIRE_HIT", "X_ATTACK", "X_DEFEND",
        "X_SPEED", "X_SPECIAL",
    ];
    for item_id in TRAINER_AI_ITEM_ORDER {
        let Some(item_slot) = trainer.items.iter().enumerate().find_map(|(slot, owned)| {
            (owned.as_deref() == Some(item_id)
                && !trainer_items_used.contains(&format!("{trainer_id}:{item_id}:{slot}")))
                .then_some(slot)
        }) else {
            continue;
        };
        let item_key = format!("{trainer_id}:{item_id}:{item_slot}");
        let usable = match item_id {
            "FULL_RESTORE" => heal_item_usable(&mut rng)
                || (context_use
                    && (matches!(active.status.as_deref(), Some("FREEZE" | "SLEEP"))
                        || (active.status.as_deref() == Some("BAD_POISON")
                            && battle.enemy_toxic_turns >= 4
                            && rng.battle_random_byte() < 128))),
            "MAX_POTION" | "HYPER_POTION" | "SUPER_POTION" | "POTION" => {
                heal_item_usable(&mut rng)
            }
            "FULL_HEAL" if context_use => {
                matches!(active.status.as_deref(), Some("FREEZE" | "SLEEP"))
                    || (active.status.as_deref() == Some("BAD_POISON")
                        && battle.enemy_toxic_turns >= 4
                        && rng.battle_random_byte() < 128)
            }
            "FULL_HEAL" if always_use => has_status,
            "FULL_HEAL" => has_status && rng.battle_random_byte() < 50,
            "X_ACCURACY" | "GUARD_SPEC" | "DIRE_HIT" | "X_ATTACK" | "X_DEFEND"
            | "X_SPEED" | "X_SPECIAL" => {
                if battle.enemy_turns_taken == 0 {
                    always_use
                        || (rng.battle_random_byte() >= 128
                            && (context_use || rng.battle_random_byte() >= 128))
                } else {
                    always_use && rng.battle_random_byte() < 50
                }
            }
            _ => false,
        };
        if usable {
            // The exact item effect is resolved by the core battle pipeline;
            // eligibility has already consumed the same AI RNG samples.
            trainer_items_used.insert(item_key);
            return Ok((Some(item_id.to_string()), rng.seed()));
        }
    }
    Ok((None, rng.seed()))
}

fn resolve_active_battle_turn_with_enemy_rng(
    runtime_shell: &mut BevyRuntimeShell,
    enemy_rng_seed_after: u32,
    player_action: BattleAction,
    enemy_action: BattleAction,
) -> Result<crate::RuntimeBattleTurn> {
    let mut preview = runtime_shell.shell.session.state.clone();
    // Enemy AI consumes Crystal RNG before the core turn begins.  Seed the
    // turn preview after that AI consumption, then commit the same starting
    // seed to the authoritative session before applying the turn.  Previously
    // this argument was ignored, so AI RNG calls vanished from battle state.
    preview.rng_seed = enemy_rng_seed_after;
    runtime_shell.runtime.data.resolve_active_battle_turn(
        &mut preview,
        player_action.clone(),
        enemy_action.clone(),
    )?;
    runtime_shell.shell.session_mut().state_mut().rng_seed = enemy_rng_seed_after;
    runtime_shell
        .shell
        .resolve_active_battle_turn(player_action, enemy_action, preview.rng_seed)
}

fn resolve_active_battle_enemy_action_with_rng(
    runtime_shell: &mut BevyRuntimeShell,
    enemy_rng_seed_after: u32,
    enemy_action: BattleAction,
) -> Result<crate::RuntimeBattleTurn> {
    let mut preview = runtime_shell.shell.session.state.clone();
    preview.rng_seed = enemy_rng_seed_after;
    runtime_shell
        .runtime
        .data
        .resolve_active_battle_enemy_action(&mut preview, enemy_action.clone())?;
    runtime_shell.shell.session_mut().state_mut().rng_seed = enemy_rng_seed_after;
    runtime_shell
        .shell
        .resolve_active_battle_enemy_action(enemy_action, preview.rng_seed)
}

fn battle_item_error_is_play_refusal(error: &anyhow::Error) -> bool {
    let Some(item_error) = error.downcast_ref::<BattleItemError>() else {
        return false;
    };
    matches!(
        item_error,
        BattleItemError::TargetFainted { .. }
            | BattleItemError::ActiveTrainerBattle { .. }
            | BattleItemError::NoTargetChange { .. }
    )
}

fn handle_visible_battle_item_refusal(
    runtime_shell: &mut BevyRuntimeShell,
    item_id: &str,
    error: anyhow::Error,
) -> Result<()> {
    runtime_shell.battle_pack_target_mode = None;
    runtime_shell.party_move_cursor = None;
    runtime_shell.field_pack_action_cursor = None;
    let snapshot = runtime_shell.shell.snapshot()?;
    runtime_shell
        .battle_messages
        .push_back("It won't have any effect.".to_string());
    if snapshot.battle.is_some() {
        runtime_shell.battle_message_scene = Some(Box::new(snapshot));
    }
    mark_runtime_snapshot_dirty(runtime_shell);
    runtime_shell
        .last_audio_events
        .push(format!("battle item {item_id} refused: {error}"));
    set_shell_action_status(runtime_shell, format!("{item_id} WON'T HAVE ANY EFFECT"));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn handle_visible_no_active_battle(
    runtime_shell: &mut BevyRuntimeShell,
    action: &str,
) -> Result<()> {
    reset_visible_battle_action_cursors(runtime_shell);
    reset_visible_battle_item_cursors(runtime_shell);
    record_visible_runtime_action(runtime_shell, format!("battle:{action}:no_active_battle"))?;
    runtime_shell
        .last_audio_events
        .push("no active battle".to_string());
    set_shell_action_status(runtime_shell, "NO ACTIVE BATTLE");
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn handle_visible_no_active_shop(runtime_shell: &mut BevyRuntimeShell, action: &str) -> Result<()> {
    runtime_shell.menu_cursor = None;
    runtime_shell.sell_cursor = None;
    record_visible_runtime_action(runtime_shell, format!("shop:{action}:no_active_shop"))?;
    runtime_shell
        .last_audio_events
        .push("no active shop".to_string());
    set_shell_action_status(runtime_shell, "NO ACTIVE SHOP");
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn handle_visible_no_active_menu(runtime_shell: &mut BevyRuntimeShell, action: &str) -> Result<()> {
    runtime_shell.menu_cursor = None;
    record_visible_runtime_action(runtime_shell, format!("ui:menu:{action}:no_active_menu"))?;
    runtime_shell
        .last_audio_events
        .push("no active menu".to_string());
    set_shell_action_status(runtime_shell, "NO ACTIVE MENU");
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn handle_visible_no_active_title_menu(
    runtime_shell: &mut BevyRuntimeShell,
    action: &str,
) -> Result<()> {
    record_visible_runtime_action(runtime_shell, format!("title:{action}:not_open"))?;
    runtime_shell
        .last_audio_events
        .push("title menu is not open".to_string());
    set_shell_action_status(runtime_shell, "TITLE CLOSED");
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn handle_visible_no_credits_screen(
    runtime_shell: &mut BevyRuntimeShell,
    action: &str,
) -> Result<()> {
    record_visible_runtime_action(runtime_shell, format!("credits:{action}:not_open"))?;
    runtime_shell
        .last_audio_events
        .push("credits screen is not open".to_string());
    set_shell_action_status(runtime_shell, "CREDITS CLOSED");
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn handle_visible_no_player_name_input(
    runtime_shell: &mut BevyRuntimeShell,
    action: &str,
) -> Result<()> {
    record_visible_runtime_action(runtime_shell, format!("player_name:{action}:not_open"))?;
    runtime_shell
        .last_audio_events
        .push("no player name input is open".to_string());
    set_shell_action_status(runtime_shell, "NAME INPUT CLOSED");
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn handle_visible_no_active_pokegear(
    runtime_shell: &mut BevyRuntimeShell,
    action: &str,
) -> Result<()> {
    record_visible_runtime_action(runtime_shell, format!("pokegear:{action}:not_open"))?;
    runtime_shell
        .last_audio_events
        .push("Pokegear is not open".to_string());
    set_shell_action_status(runtime_shell, "POKEGEAR CLOSED");
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn handle_visible_no_field_pack_target(
    runtime_shell: &mut BevyRuntimeShell,
    action: &str,
) -> Result<()> {
    runtime_shell.party_move_cursor = None;
    record_visible_runtime_action(runtime_shell, format!("pack:target:{action}:none"))?;
    runtime_shell
        .last_audio_events
        .push("no active pack target mode".to_string());
    set_shell_action_status(runtime_shell, "NO PACK TARGET");
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn handle_visible_no_elevator_prompt(
    runtime_shell: &mut BevyRuntimeShell,
    action: &str,
) -> Result<()> {
    runtime_shell.elevator_cursor = None;
    record_visible_runtime_action(runtime_shell, format!("ui:elevator:{action}:none_visible"))?;
    runtime_shell
        .last_audio_events
        .push("no compiled elevator prompt is visible".to_string());
    set_shell_action_status(runtime_shell, "NO ELEVATOR");
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn handle_visible_no_runtime_flag(
    runtime_shell: &mut BevyRuntimeShell,
    action: &str,
) -> Result<()> {
    record_visible_runtime_action(runtime_shell, format!("script:flag:{action}:none"))?;
    runtime_shell
        .last_audio_events
        .push("no active script runtime flag".to_string());
    set_shell_action_status(runtime_shell, "NO RUNTIME FLAG");
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn handle_visible_no_active_script_cursor(
    runtime_shell: &mut BevyRuntimeShell,
    action: &str,
) -> Result<()> {
    record_visible_runtime_action(runtime_shell, format!("script:{action}:no_active_cursor"))?;
    runtime_shell
        .last_audio_events
        .push("no active script cursor".to_string());
    set_shell_action_status(runtime_shell, "NO ACTIVE SCRIPT");
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn stage_visible_battle_item_use(
    runtime_shell: &mut BevyRuntimeShell,
    item_id: &str,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let player_name = snapshot.trainer.player_name.as_str();
    let display_name = item_display_name(&snapshot, item_id);
    runtime_shell.battle_messages.push_back(format!(
        "{player_name} used the {display_name}."
    ));
    runtime_shell.battle_message_scene = Some(Box::new(snapshot));
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(())
}

fn stage_visible_battle_item_effect(
    runtime_shell: &mut BevyRuntimeShell,
    snapshot: &RuntimeShellSnapshot,
    outcome: &BattleItemOutcome,
    party_index: Option<usize>,
) -> Result<()> {
    let target_index = party_index.or_else(|| {
        snapshot
            .battle
            .as_ref()
            .and_then(|battle| battle.active_player_party_index)
    });
    let target_name = target_index
        .and_then(|index| snapshot.party.slots.iter().find(|slot| slot.index == index))
        .map(|slot| slot.pokemon.nickname.as_str())
        .context("battle item result requires its target party Pokemon")?;
    let notice = if outcome.hp_before == 0 && outcome.hp_after > 0 {
        Some(format!("{target_name}\nis revitalized."))
    } else if outcome.hp_after > outcome.hp_before {
        Some(format!(
            "{target_name}\nrecovered {}HP!",
            outcome.hp_after - outcome.hp_before
        ))
    } else if outcome.status_before != outcome.status_after {
        match outcome.status_before.as_deref() {
            Some("POISON") => Some(format!("{target_name}'s\ncured of poison.")),
            Some("PARALYSIS") => Some(format!("{target_name}'s\nrid of paralysis.")),
            Some("BURN") => Some(format!("{target_name}'s\nburn was healed.")),
            Some("FREEZE") => Some(format!("{target_name}\nwas defrosted.")),
            Some("SLEEP") => Some(format!("{target_name}\nwoke up.")),
            _ => None,
        }
    } else if outcome.confusion_turns_after < outcome.confusion_turns_before {
        let pure_confusion_heal = snapshot
            .items
            .iter()
            .find(|item| item.item_id == outcome.item_id)
            .is_some_and(|item| {
                item.confusion_heal == Some(true) && item.status_heals.is_empty()
            });
        Some(if pure_confusion_heal {
            format!("{target_name}'s\nconfused no more!")
        } else {
            format!("{target_name} came\nto its senses.")
        })
    } else if let Some(change) = outcome
        .pp_changes
        .iter()
        .find(|change| change.pp_after > change.pp_before)
    {
        let raises_pp = snapshot
            .items
            .iter()
            .find(|item| item.item_id == outcome.item_id)
            .is_some_and(|item| item.pp_up_stages.is_some());
        Some(if raises_pp {
            format!(
                "{}'s PP\nincreased.",
                battle_move_display_name(snapshot, &change.move_id)
            )
        } else {
            "PP was restored.".to_string()
        })
    } else if let Some(change) = outcome.battle_stat_stage_changes.first() {
        Some(format!(
            "{target_name}'s\n{} rose!",
            battle_stat_display_name(&change.stat)
        ))
    } else {
        None
    };
    if let Some(notice) = notice {
        runtime_shell.battle_messages.push_back(notice);
        mark_runtime_snapshot_dirty(runtime_shell);
    }
    Ok(())
}

fn resolve_visible_battle_enemy_response_after_player_item(
    runtime_shell: &mut BevyRuntimeShell,
    item_id: &str,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let Some(ref battle) = snapshot.battle else {
        return Ok(());
    };
    if visible_active_battle_player_fainted(&snapshot) || battle.enemy_pokemon.hp == 0 {
        return settle_visible_battle_after_action(runtime_shell);
    }
    let battle_before_turn = battle.clone();
    let (enemy_action, enemy_rng_seed_after) =
        selected_enemy_battle_action(&snapshot, battle, &mut runtime_shell.trainer_items_used)?;
    persist_selected_enemy_trainer_item(runtime_shell, battle, &enemy_action)?;
    let enemy_slot = match &enemy_action {
        BattleAction::Move { slot } => Some(*slot),
        _ => None,
    };
    record_visible_runtime_action(
        runtime_shell,
        format!("battle:enemy_response:{item_id}:{enemy_action:?}"),
    )?;
    let turn = resolve_active_battle_enemy_action_with_rng(
        runtime_shell,
        enemy_rng_seed_after,
        enemy_action,
    )?;
    stage_visible_battle_messages(runtime_shell, &snapshot, &turn.outcome.events);
    let events = format_battle_turn_events(&turn.outcome.events);
    runtime_shell.last_audio_events.push(format!(
        "battle item response item={} enemy_slot={} {} events={} checksum={:?}",
        item_id,
        enemy_slot.map_or_else(|| "switch".to_string(), |slot| slot.to_string()),
        format_battle_turn_summary(&turn.outcome),
        events,
        turn.state_checksum
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    set_shell_action_status(
        runtime_shell,
        format!(
            "ENEMY RESPONSE {} {}",
            item_id,
            format_battle_turn_summary(&turn.outcome)
        ),
    );
    settle_visible_resolved_battle_turn(runtime_shell, &battle_before_turn)
}

fn format_battle_turn_events(events: &[crate::core::battle::turn::BattleEvent]) -> String {
    if events.is_empty() {
        return "none".to_string();
    }
    events
        .iter()
        .map(|event| match event {
            crate::core::battle::turn::BattleEvent::AutomaticStruggle { side } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_automatic_struggle",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_automatic_struggle",
            },
            crate::core::battle::turn::BattleEvent::MoveSelected { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_move_selected",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_move_selected",
            },
            crate::core::battle::turn::BattleEvent::Disobeyed { side }
            | crate::core::battle::turn::BattleEvent::DisobedienceIdle { side, .. }
            | crate::core::battle::turn::BattleEvent::DisobedienceIgnoredSleeping { side } => {
                match side {
                    crate::core::battle::turn::BattleSide::Player => "player_disobeyed",
                    crate::core::battle::turn::BattleSide::Enemy => "enemy_disobeyed",
                }
            }
            crate::core::battle::turn::BattleEvent::NoPp { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_no_pp",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_no_pp",
            },
            crate::core::battle::turn::BattleEvent::MoveUsed { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_move_used",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_move_used",
            },
            crate::core::battle::turn::BattleEvent::Missed { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_missed",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_missed",
            },
            crate::core::battle::turn::BattleEvent::NoEffect { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_no_effect",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_no_effect",
            },
            crate::core::battle::turn::BattleEvent::Damage { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_damage",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_damage",
            },
            crate::core::battle::turn::BattleEvent::MagnitudePower { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_magnitude_power",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_magnitude_power",
            },
            crate::core::battle::turn::BattleEvent::HiddenPowerResolved { side, .. } => {
                match side {
                    crate::core::battle::turn::BattleSide::Player => "player_hidden_power",
                    crate::core::battle::turn::BattleSide::Enemy => "enemy_hidden_power",
                }
            }
            crate::core::battle::turn::BattleEvent::PresentPower { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_present_power",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_present_power",
            },
            crate::core::battle::turn::BattleEvent::PresentHeal { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_present_heal",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_present_heal",
            },
            crate::core::battle::turn::BattleEvent::PresentFailed { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_present_failed",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_present_failed",
            },
            crate::core::battle::turn::BattleEvent::JumpKickCrash { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_jump_kick_crash",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_jump_kick_crash",
            },
            crate::core::battle::turn::BattleEvent::RampageStarted { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_rampage_started",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_rampage_started",
            },
            crate::core::battle::turn::BattleEvent::RampageForcedMove { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_rampage_forced",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_rampage_forced",
            },
            crate::core::battle::turn::BattleEvent::RampageEnded { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_rampage_ended",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_rampage_ended",
            },
            crate::core::battle::turn::BattleEvent::AirborneStarted { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_airborne_started",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_airborne_started",
            },
            crate::core::battle::turn::BattleEvent::AirborneForcedMove { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_airborne_forced",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_airborne_forced",
            },
            crate::core::battle::turn::BattleEvent::AirborneAvoided { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_airborne_avoided",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_airborne_avoided",
            },
            crate::core::battle::turn::BattleEvent::AirborneEnded { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_airborne_ended",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_airborne_ended",
            },
            crate::core::battle::turn::BattleEvent::ChargeStarted { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_charge_started",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_charge_started",
            },
            crate::core::battle::turn::BattleEvent::ChargeForcedMove { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_charge_forced",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_charge_forced",
            },
            crate::core::battle::turn::BattleEvent::ChargeEnded { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_charge_ended",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_charge_ended",
            },
            crate::core::battle::turn::BattleEvent::MultiHitCount { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_multi_hit",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_multi_hit",
            },
            crate::core::battle::turn::BattleEvent::PayDayMoney { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_pay_day_money",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_pay_day_money",
            },
            crate::core::battle::turn::BattleEvent::PainSplitApplied { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_pain_split",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_pain_split",
            },
            crate::core::battle::turn::BattleEvent::OhkoFailed { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_ohko_failed",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_ohko_failed",
            },
            crate::core::battle::turn::BattleEvent::MistApplied { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_mist",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_mist",
            },
            crate::core::battle::turn::BattleEvent::MistFailed { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_mist_failed",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_mist_failed",
            },
            crate::core::battle::turn::BattleEvent::MistProtected { target, .. } => match target {
                crate::core::battle::turn::BattleSide::Player => "player_mist_protected",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_mist_protected",
            },
            crate::core::battle::turn::BattleEvent::SafeguardApplied { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_safeguard",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_safeguard",
            },
            crate::core::battle::turn::BattleEvent::SafeguardFailed { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_safeguard_failed",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_safeguard_failed",
            },
            crate::core::battle::turn::BattleEvent::SafeguardProtected { target, .. } => {
                match target {
                    crate::core::battle::turn::BattleSide::Player => "player_safeguard_protected",
                    crate::core::battle::turn::BattleSide::Enemy => "enemy_safeguard_protected",
                }
            }
            crate::core::battle::turn::BattleEvent::SafeguardCount { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_safeguard_count",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_safeguard_count",
            },
            crate::core::battle::turn::BattleEvent::ReflectApplied { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_reflect",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_reflect",
            },
            crate::core::battle::turn::BattleEvent::ReflectFailed { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_reflect_failed",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_reflect_failed",
            },
            crate::core::battle::turn::BattleEvent::ReflectCount { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_reflect_count",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_reflect_count",
            },
            crate::core::battle::turn::BattleEvent::LightScreenApplied { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_light_screen",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_light_screen",
            },
            crate::core::battle::turn::BattleEvent::LightScreenFailed { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_light_screen_failed",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_light_screen_failed",
            },
            crate::core::battle::turn::BattleEvent::LightScreenCount { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_light_screen_count",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_light_screen_count",
            },
            crate::core::battle::turn::BattleEvent::DestinyBondApplied { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_destiny_bond",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_destiny_bond",
            },
            crate::core::battle::turn::BattleEvent::DestinyBondActivated { side, .. } => match side
            {
                crate::core::battle::turn::BattleSide::Player => "player_destiny_bond_activated",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_destiny_bond_activated",
            },
            crate::core::battle::turn::BattleEvent::SleepTalkSelected { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_sleep_talk_selected",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_sleep_talk_selected",
            },
            crate::core::battle::turn::BattleEvent::SleepTalkFailed { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_sleep_talk_failed",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_sleep_talk_failed",
            },
            crate::core::battle::turn::BattleEvent::EncoreApplied { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_encore",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_encore",
            },
            crate::core::battle::turn::BattleEvent::EncoreFailed { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_encore_failed",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_encore_failed",
            },
            crate::core::battle::turn::BattleEvent::EncoreForcedMove { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_encore_forced",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_encore_forced",
            },
            crate::core::battle::turn::BattleEvent::EncoreEnded { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_encore_ended",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_encore_ended",
            },
            crate::core::battle::turn::BattleEvent::LeechSeedApplied { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_leech_seed",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_leech_seed",
            },
            crate::core::battle::turn::BattleEvent::LeechSeedFailed { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_leech_seed_failed",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_leech_seed_failed",
            },
            crate::core::battle::turn::BattleEvent::LeechSeedImmune { target, .. } => {
                match target {
                    crate::core::battle::turn::BattleSide::Player => "player_leech_seed_immune",
                    crate::core::battle::turn::BattleSide::Enemy => "enemy_leech_seed_immune",
                }
            }
            crate::core::battle::turn::BattleEvent::LeechSeedDamage { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_leech_seed_damage",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_leech_seed_damage",
            },
            crate::core::battle::turn::BattleEvent::LeechSeedDrain { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_leech_seed_drain",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_leech_seed_drain",
            },
            crate::core::battle::turn::BattleEvent::CurseApplied { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_curse_applied",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_curse_applied",
            },
            crate::core::battle::turn::BattleEvent::CurseFailed { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_curse_failed",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_curse_failed",
            },
            crate::core::battle::turn::BattleEvent::CurseDamage { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_curse_damage",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_curse_damage",
            },
            crate::core::battle::turn::BattleEvent::CurseEnded { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_curse_ended",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_curse_ended",
            },
            crate::core::battle::turn::BattleEvent::NightmareApplied { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_nightmare_applied",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_nightmare_applied",
            },
            crate::core::battle::turn::BattleEvent::NightmareFailed { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_nightmare_failed",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_nightmare_failed",
            },
            crate::core::battle::turn::BattleEvent::NightmareDamage { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_nightmare_damage",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_nightmare_damage",
            },
            crate::core::battle::turn::BattleEvent::NightmareEnded { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_nightmare_ended",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_nightmare_ended",
            },
            crate::core::battle::turn::BattleEvent::TrapApplied { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_trap_applied",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_trap_applied",
            },
            crate::core::battle::turn::BattleEvent::TrapFailed { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_trap_failed",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_trap_failed",
            },
            crate::core::battle::turn::BattleEvent::TrapDamage { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_trap_damage",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_trap_damage",
            },
            crate::core::battle::turn::BattleEvent::TrapEnded { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_trap_ended",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_trap_ended",
            },
            crate::core::battle::turn::BattleEvent::EscapeTrapApplied { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_escape_trap_applied",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_escape_trap_applied",
            },
            crate::core::battle::turn::BattleEvent::EscapeTrapEnded { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_escape_trap_ended",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_escape_trap_ended",
            },
            crate::core::battle::turn::BattleEvent::LockOnApplied { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_lock_on_applied",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_lock_on_applied",
            },
            crate::core::battle::turn::BattleEvent::LockOnConsumed { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_lock_on_consumed",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_lock_on_consumed",
            },
            crate::core::battle::turn::BattleEvent::AttractApplied { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_attract_applied",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_attract_applied",
            },
            crate::core::battle::turn::BattleEvent::AttractFailed { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_attract_failed",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_attract_failed",
            },
            crate::core::battle::turn::BattleEvent::InfatuatedTurn { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_infatuated_turn",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_infatuated_turn",
            },
            crate::core::battle::turn::BattleEvent::InfatuatedImmobilized { side, .. } => {
                match side {
                    crate::core::battle::turn::BattleSide::Player => {
                        "player_infatuated_immobilized"
                    }
                    crate::core::battle::turn::BattleSide::Enemy => "enemy_infatuated_immobilized",
                }
            }
            crate::core::battle::turn::BattleEvent::RechargeTurn { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_recharge_turn",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_recharge_turn",
            },
            crate::core::battle::turn::BattleEvent::RechargeStarted { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_recharge_started",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_recharge_started",
            },
            crate::core::battle::turn::BattleEvent::ForceSwitchApplied { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_force_switch",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_force_switch",
            },
            crate::core::battle::turn::BattleEvent::ForceSwitchFailed { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_force_switch_failed",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_force_switch_failed",
            },
            crate::core::battle::turn::BattleEvent::SpikesApplied { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_spikes_applied",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_spikes_applied",
            },
            crate::core::battle::turn::BattleEvent::SpikesFailed { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_spikes_failed",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_spikes_failed",
            },
            crate::core::battle::turn::BattleEvent::SpikesDamage { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_spikes_damage",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_spikes_damage",
            },
            crate::core::battle::turn::BattleEvent::SpikesImmune { side } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_spikes_immune",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_spikes_immune",
            },
            crate::core::battle::turn::BattleEvent::RapidSpinCleared { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_rapid_spin_cleared",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_rapid_spin_cleared",
            },
            crate::core::battle::turn::BattleEvent::CounterDamage { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_counter_damage",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_counter_damage",
            },
            crate::core::battle::turn::BattleEvent::ForesightApplied { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_foresight_applied",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_foresight_applied",
            },
            crate::core::battle::turn::BattleEvent::ForesightFailed { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_foresight_failed",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_foresight_failed",
            },
            crate::core::battle::turn::BattleEvent::DisableApplied { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_disable_applied",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_disable_applied",
            },
            crate::core::battle::turn::BattleEvent::DisableFailed { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_disable_failed",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_disable_failed",
            },
            crate::core::battle::turn::BattleEvent::DisabledMove { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_disabled_move",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_disabled_move",
            },
            crate::core::battle::turn::BattleEvent::DisableCount { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_disable_count",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_disable_count",
            },
            crate::core::battle::turn::BattleEvent::DisableEnded { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_disable_ended",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_disable_ended",
            },
            crate::core::battle::turn::BattleEvent::ProtectApplied { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_protect_applied",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_protect_applied",
            },
            crate::core::battle::turn::BattleEvent::ProtectFailed { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_protect_failed",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_protect_failed",
            },
            crate::core::battle::turn::BattleEvent::MoveProtected { target, .. } => match target {
                crate::core::battle::turn::BattleSide::Player => "player_move_protected",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_move_protected",
            },
            crate::core::battle::turn::BattleEvent::EndureApplied { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_endure_applied",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_endure_applied",
            },
            crate::core::battle::turn::BattleEvent::EndureFailed { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_endure_failed",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_endure_failed",
            },
            crate::core::battle::turn::BattleEvent::EnduredHit { target, .. } => match target {
                crate::core::battle::turn::BattleSide::Player => "player_endured_hit",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_endured_hit",
            },
            crate::core::battle::turn::BattleEvent::SpiteApplied { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_spite_applied",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_spite_applied",
            },
            crate::core::battle::turn::BattleEvent::SpiteFailed { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_spite_failed",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_spite_failed",
            },
            crate::core::battle::turn::BattleEvent::StatsReset { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_stats_reset",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_stats_reset",
            },
            crate::core::battle::turn::BattleEvent::PsychUpApplied { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_psych_up",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_psych_up",
            },
            crate::core::battle::turn::BattleEvent::WeatherApplied { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_weather_applied",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_weather_applied",
            },
            crate::core::battle::turn::BattleEvent::WeatherContinues { .. } => "weather_continues",
            crate::core::battle::turn::BattleEvent::SandstormDamage { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_sandstorm_damage",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_sandstorm_damage",
            },
            crate::core::battle::turn::BattleEvent::WeatherEnded { .. } => "weather_ended",
            crate::core::battle::turn::BattleEvent::Fainted { side } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_fainted",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_fainted",
            },
            crate::core::battle::turn::BattleEvent::Switched { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_switched",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_switched",
            },
            crate::core::battle::turn::BattleEvent::SwitchBlocked { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_switch_blocked",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_switch_blocked",
            },
            crate::core::battle::turn::BattleEvent::ItemUsed { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_item_used",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_item_used",
            },
            crate::core::battle::turn::BattleEvent::BattleItemEffect { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_item_effect",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_item_effect",
            },
            crate::core::battle::turn::BattleEvent::RunAttempt { side, outcome } => match side {
                crate::core::battle::turn::BattleSide::Player if outcome.escaped => {
                    "player_run_escaped"
                }
                crate::core::battle::turn::BattleSide::Player => "player_run_failed",
                crate::core::battle::turn::BattleSide::Enemy if outcome.escaped => {
                    "enemy_run_escaped"
                }
                crate::core::battle::turn::BattleSide::Enemy => "enemy_run_failed",
            },
            crate::core::battle::turn::BattleEvent::RunBlocked { side, .. } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_run_blocked",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_run_blocked",
            },
            crate::core::battle::turn::BattleEvent::RunPrevented { side } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_run_prevented",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_run_prevented",
            },
            crate::core::battle::turn::BattleEvent::Fled { side } => match side {
                crate::core::battle::turn::BattleSide::Player => "player_fled",
                crate::core::battle::turn::BattleSide::Enemy => "enemy_fled",
            },
            _ => "battle_event",
        })
        .collect::<Vec<_>>()
        .join(",")
}
