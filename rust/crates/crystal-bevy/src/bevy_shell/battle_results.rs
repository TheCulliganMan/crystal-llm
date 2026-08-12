fn finish_visible_inactive_battle_after_turn(
    runtime_shell: &mut BevyRuntimeShell,
    battle_before_turn: &crate::RuntimeBattleSnapshot,
    scripted_static_wild: Option<VisibleStaticWildOrigin>,
    plain_reason: &str,
) -> Result<()> {
    if runtime_shell.shell.snapshot()?.battle.is_some() {
        return Ok(());
    }
    let has_usable_party = runtime_shell
        .shell
        .session()
        .state()
        .storage
        .party
        .pokemon
        .iter()
        .flatten()
        .any(|pokemon| {
            !pokemon.is_egg
                && pokemon.species.id != "EGG"
                && pokemon.hp > 0
        });
    if !has_usable_party {
        if battle_before_turn.battle_type == "BATTLETYPE_CANLOSE" {
            let RuntimeBattleKind::Trainer {
                source_script,
                loss_text,
                ..
            } = &battle_before_turn.kind
            else {
                anyhow::bail!("BATTLETYPE_CANLOSE is only valid for a trainer battle");
            };
            let mut terminal_snapshot = runtime_shell.shell.snapshot()?;
            let map_name = terminal_snapshot.overworld.map_name.clone();
            terminal_snapshot.battle = Some(battle_before_turn.clone());
            queue_visible_trainer_result_text(
                runtime_shell,
                &terminal_snapshot,
                loss_text,
            )?;
            reset_visible_battle_exit_state(runtime_shell);
            return complete_visible_scripted_trainer_battle(
                runtime_shell,
                &map_name,
                source_script,
                false,
                true,
            );
        }
        reset_visible_battle_exit_state(runtime_shell);
        return resolve_visible_blackout(runtime_shell);
    }
    let player_name = runtime_shell.shell.snapshot()?.trainer.player_name;
    let base_result = runtime_shell.shell.session().state().battle_result & 0x3f;
    if base_result == 0 {
        queue_visible_pay_day_payout_for_battle(
            runtime_shell,
            battle_before_turn,
            &player_name,
        );
    }
    reset_visible_battle_exit_state(runtime_shell);
    match &battle_before_turn.kind {
        RuntimeBattleKind::StaticWild { .. } => {
            if let Some(origin) = scripted_static_wild {
                complete_visible_scripted_wild_battle(runtime_shell, &origin)
            } else {
                restore_visible_overworld_after_battle_exit(runtime_shell, plain_reason)
            }
        }
        RuntimeBattleKind::Trainer { source_script, .. } => {
            let snapshot = runtime_shell.shell.snapshot()?;
            let map_name = snapshot.overworld.map_name;
            let can_lose = battle_before_turn.battle_type == "BATTLETYPE_CANLOSE";
            complete_visible_scripted_trainer_battle(
                runtime_shell,
                &map_name,
                source_script,
                false,
                can_lose,
            )
        }
        RuntimeBattleKind::Wild { .. } => {
            restore_visible_overworld_after_battle_exit(runtime_shell, plain_reason)
        }
    }
}

fn settle_visible_resolved_battle_turn(
    runtime_shell: &mut BevyRuntimeShell,
    battle_before_turn: &crate::RuntimeBattleSnapshot,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    if snapshot.battle.is_some() {
        return settle_visible_battle_after_action(runtime_shell);
    }
    let scripted_static_wild = visible_static_wild_source(&snapshot, battle_before_turn);
    finish_visible_inactive_battle_after_turn(
        runtime_shell,
        battle_before_turn,
        scripted_static_wild,
        "battle_turn_exit",
    )
}

fn execute_next_visible_queued_script_command(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    if runtime_shell
        .shell
        .snapshot()?
        .script_events
        .command_queue
        .is_empty()
    {
        record_visible_runtime_action(runtime_shell, "script:queued:none")?;
        runtime_shell
            .last_audio_events
            .push("no queued script command is pending".to_string());
        set_shell_action_status(runtime_shell, "NO QUEUED SCRIPT");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    record_visible_runtime_action(runtime_shell, "script:queued:execute_next")?;
    let executed = runtime_shell.shell.run_next_queued_script_until_boundary(
        256,
        ScriptRuntimeInputs::default(),
        ScriptPhoneInputs::default(),
    )?;
    let target = executed.queued.queued.target.clone();
    runtime_shell.last_audio_events.push(format!(
        "script queued command={} target={} bank={:?} source={}:{} resumed_steps={} checksum={:?}",
        executed.queued.queued.command,
        target,
        executed.queued.queued.bank,
        executed.queued.queued.source_script,
        executed.queued.queued.command_index,
        executed.run.steps.len(),
        executed.queued.state_checksum
    ));
    let reached_boundary =
        integrate_visible_compiled_script_run(runtime_shell, &executed.run.steps)?;
    arm_visible_active_script_cursor_from_run(runtime_shell, executed.run.next_cursor);
    if reached_boundary {
        return Ok(());
    }
    continue_visible_script_after_prompt(runtime_shell)?;
    Ok(())
}

fn take_visible_next_script(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    if runtime_shell
        .shell
        .snapshot()?
        .script_events
        .next_script
        .is_none()
    {
        record_visible_runtime_action(runtime_shell, "script:next:none")?;
        runtime_shell
            .last_audio_events
            .push("no next script is pending".to_string());
        set_shell_action_status(runtime_shell, "NO NEXT SCRIPT");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    record_visible_runtime_action(runtime_shell, "script:next:take")?;
    let next = runtime_shell.shell.run_pending_next_script_until_boundary(
        256,
        ScriptRuntimeInputs::default(),
        ScriptPhoneInputs::default(),
    )?;
    reset_visible_selection_cursors(runtime_shell);
    runtime_shell.last_audio_events.push(format!(
        "script next={} resumed_steps={} checksum={:?}",
        next.next_script.script,
        next.run.steps.len(),
        next.next_script.state_checksum
    ));
    // A compiled-script boundary only says that the core has paused.  Its
    // mutations still need to be presented before returning: special routines
    // such as PlayersHousePC create the menu boundary and the visible PC
    // surface in the same step.  Short-circuiting here armed the following
    // `iftrue` command without ever opening that surface, so the next frame
    // closed the menu and the interaction appeared to do nothing.
    let effects_reached_boundary =
        integrate_visible_compiled_script_run(runtime_shell, &next.run.steps)?;
    let reached_boundary = next.run.boundary.is_some() || effects_reached_boundary;
    arm_visible_active_script_cursor_from_run(runtime_shell, next.run.next_cursor);
    if reached_boundary {
        return Ok(());
    }
    continue_visible_script_after_prompt(runtime_shell)?;
    Ok(())
}

fn take_visible_deferred_script(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    if runtime_shell
        .shell
        .snapshot()?
        .script_events
        .deferred_scripts
        .is_empty()
    {
        record_visible_runtime_action(runtime_shell, "script:deferred:none")?;
        runtime_shell
            .last_audio_events
            .push("no deferred script is pending".to_string());
        set_shell_action_status(runtime_shell, "NO DEFERRED SCRIPT");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    record_visible_runtime_action(runtime_shell, "script:deferred:take")?;
    let deferred = runtime_shell
        .shell
        .run_next_deferred_script_until_boundary(
            256,
            ScriptRuntimeInputs::default(),
            ScriptPhoneInputs::default(),
        )?;
    reset_visible_selection_cursors(runtime_shell);
    runtime_shell.last_audio_events.push(format!(
        "script deferred={} resumed_steps={} checksum={:?}",
        deferred.deferred_script.script,
        deferred.run.steps.len(),
        deferred.deferred_script.state_checksum
    ));
    let reached_boundary =
        integrate_visible_compiled_script_run(runtime_shell, &deferred.run.steps)?;
    arm_visible_active_script_cursor_from_run(runtime_shell, deferred.run.next_cursor);
    if reached_boundary {
        return Ok(());
    }
    continue_visible_script_after_prompt(runtime_shell)?;
    Ok(())
}

fn take_visible_script_end_state(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    consume_visible_script_end_state(runtime_shell)?;
    if visible_script_call_stack_has_return(runtime_shell) {
        resume_visible_script_return(runtime_shell)?;
    }
    Ok(())
}

fn consume_visible_script_end_state(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    if runtime_shell
        .shell
        .snapshot()?
        .script_events
        .script_ended
        .is_none()
    {
        record_visible_runtime_action(runtime_shell, "script:end_state:none")?;
        runtime_shell
            .last_audio_events
            .push("no script end state is pending".to_string());
        set_shell_action_status(runtime_shell, "NO SCRIPT END");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    record_visible_runtime_action(runtime_shell, "script:end_state:take")?;
    let end = runtime_shell.shell.take_script_end_state()?;
    let is_callback = end.end.callback;
    runtime_shell.last_audio_events.push(format!(
        "script end state source={}:{} callback={} just_battled={} checksum={:?}",
        end.end.source_script,
        end.end.command_index,
        end.end.callback,
        end.end.just_battled_guard,
        end.state_checksum
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    if is_callback {
        take_next_visible_map_callback(runtime_shell)?;
    }
    Ok(())
}

fn apply_visible_script_entry_command(
    runtime_shell: &mut BevyRuntimeShell,
    script: &str,
) -> Result<()> {
    if open_visible_elevator_for_script_command(runtime_shell, script, 0)? {
        arm_visible_active_script_cursor(runtime_shell, script, 1);
        return Ok(());
    }
    if open_visible_gift_pokemon_for_script_command(runtime_shell, script, 0)? {
        arm_visible_active_script_cursor(runtime_shell, script, 1);
        return Ok(());
    }
    if open_visible_phone_prompt_for_script_command(runtime_shell, script, 0)? {
        arm_visible_active_script_cursor(runtime_shell, script, 1);
        return Ok(());
    }
    if open_visible_day_care_for_script_command(runtime_shell, script, 0)? {
        arm_visible_active_script_cursor(runtime_shell, script, 1);
        return Ok(());
    }
    if open_visible_script_party_selection_for_command(runtime_shell, script, 0)? {
        arm_visible_active_script_cursor(runtime_shell, script, 1);
        return Ok(());
    }
    if open_visible_buena_prize_for_script_command(runtime_shell, script, 0)? {
        arm_visible_active_script_cursor(runtime_shell, script, 1);
        return Ok(());
    }
    if open_visible_slot_machine_for_script_command(runtime_shell, script, 0)? {
        arm_visible_active_script_cursor(runtime_shell, script, 1);
        return Ok(());
    }
    if open_visible_card_flip_for_script_command(runtime_shell, script, 0)? {
        arm_visible_active_script_cursor(runtime_shell, script, 1);
        return Ok(());
    }
    if open_visible_kurt_apricorn_for_script_command(runtime_shell, script, 0)? {
        arm_visible_active_script_cursor(runtime_shell, script, 1);
        return Ok(());
    }
    if apply_visible_name_rival_for_script_command(runtime_shell, script, 0)? {
        arm_visible_active_script_cursor(runtime_shell, script, 1);
        return Ok(());
    }
    if open_visible_day_of_week_for_script_command(runtime_shell, script, 0)? {
        arm_visible_active_script_cursor(runtime_shell, script, 0);
        return Ok(());
    }
    record_visible_runtime_action(runtime_shell, format!("script:step:{script}:0"))?;
    let origin_map_name = runtime_shell.shell.session.overworld.map.name.clone();
    let stepped = runtime_shell.shell.step_compiled_script_command(
        &origin_map_name,
        script,
        0,
        explicit_compiled_script_runtime_inputs(runtime_shell, script, 0)?,
        explicit_compiled_script_phone_inputs(runtime_shell, script, 0),
    )?;
    runtime_shell.last_audio_events.push(format!(
        "script step={} command=0 result={} checksum={:?}",
        script,
        stepped.mutation.result.result_tag(),
        stepped.mutation.state_checksum
    ));
    integrate_visible_script_mutation_outcome(runtime_shell, &stepped.mutation)?;
    trim_event_log(&mut runtime_shell.last_audio_events);
    if activate_visible_script_boundary_after_outcome(runtime_shell, &stepped.mutation)? {
        arm_visible_active_script_cursor_from_run(runtime_shell, stepped.next_cursor.clone());
        return Ok(());
    }
    if open_visible_vertical_menu_for_script_command(runtime_shell, script, 0)? {
        arm_visible_active_script_cursor_from_run(runtime_shell, stepped.next_cursor.clone());
        return Ok(());
    }
    arm_visible_script_cursor_after_step(runtime_shell, &stepped);
    Ok(())
}

fn start_visible_script_entry(runtime_shell: &mut BevyRuntimeShell, script: &str) -> Result<()> {
    if has_visible_compiled_script_command(runtime_shell, script, 0) {
        apply_visible_script_entry_command(runtime_shell, script)
    } else {
        runtime_shell.active_script_cursor = None;
        runtime_shell
            .last_audio_events
            .push(format!("script complete={script}"));
        trim_event_log(&mut runtime_shell.last_audio_events);
        Ok(())
    }
}

fn arm_visible_active_script_cursor(
    runtime_shell: &mut BevyRuntimeShell,
    script: &str,
    next_command_index: usize,
) {
    let origin_map_name = runtime_shell.shell.session.overworld.map.name.clone();
    arm_visible_active_script_cursor_with_origin(
        runtime_shell,
        &origin_map_name,
        script,
        next_command_index,
    );
}

fn arm_visible_active_script_cursor_with_origin(
    runtime_shell: &mut BevyRuntimeShell,
    origin_map_name: &str,
    script: &str,
    next_command_index: usize,
) {
    if has_visible_compiled_script_command(runtime_shell, script, next_command_index) {
        runtime_shell.active_script_cursor = Some(ActiveScriptCursor {
            origin_map_name: origin_map_name.to_string(),
            source_script: script.to_string(),
            next_command_index,
        });
    } else {
        runtime_shell.active_script_cursor = None;
        runtime_shell
            .last_audio_events
            .push(format!("script complete={script}"));
        trim_event_log(&mut runtime_shell.last_audio_events);
    }
}

fn visible_active_compiled_script_cursor(
    runtime_shell: &BevyRuntimeShell,
) -> Option<RuntimeCompiledScriptCursor> {
    runtime_shell
        .active_script_cursor
        .as_ref()
        .map(|cursor| RuntimeCompiledScriptCursor {
            origin_map_name: cursor.origin_map_name.clone(),
            source_script: cursor.source_script.clone(),
            command_index: cursor.next_command_index,
        })
}

fn arm_visible_active_script_cursor_from_run(
    runtime_shell: &mut BevyRuntimeShell,
    cursor: Option<RuntimeCompiledScriptCursor>,
) {
    if let Some(cursor) = cursor {
        let opens_day_of_week = compiled_special_routine_at(
            runtime_shell,
            &cursor.source_script,
            cursor.command_index,
        )
        .is_ok_and(|routine| routine.as_deref() == Some("SetDayOfWeek"));
        if opens_day_of_week && runtime_shell.pending_day_of_week.is_none() {
            runtime_shell.pending_day_of_week = Some(PendingDayOfWeekPrompt {
                origin_map_name: cursor.origin_map_name.clone(),
                source_script: cursor.source_script.clone(),
                command_index: cursor.command_index,
                selected_day: 0,
                confirming: false,
                yes_no_index: 0,
            });
            set_shell_action_status(runtime_shell, "WHAT DAY IS IT?");
            mark_runtime_snapshot_dirty(runtime_shell);
        }
        runtime_shell.active_script_cursor = Some(ActiveScriptCursor {
            origin_map_name: cursor.origin_map_name,
            source_script: cursor.source_script,
            next_command_index: cursor.command_index,
        });
    } else {
        runtime_shell.active_script_cursor = None;
    }
}

fn arm_visible_script_cursor_after_step(
    runtime_shell: &mut BevyRuntimeShell,
    step: &crate::RuntimeCompiledScriptStep,
) {
    if matches!(
        &step.mutation.result,
        RuntimeMutationResult::ScriptControlApplied(_)
    ) {
        arm_visible_script_cursor_after_outcome(
            runtime_shell,
            &step.source_script,
            step.next_cursor
                .as_ref()
                .map(|cursor| cursor.command_index)
                .unwrap_or(step.command_index + 1),
            &step.mutation,
        );
    } else {
        arm_visible_active_script_cursor_from_run(runtime_shell, step.next_cursor.clone());
    }
}

fn arm_visible_script_cursor_after_outcome(
    runtime_shell: &mut BevyRuntimeShell,
    script: &str,
    next_command_index: usize,
    outcome: &RuntimeMutationOutcome,
) {
    match &outcome.result {
        RuntimeMutationResult::ScriptControlApplied(ScriptControlAction::End {
            callback, ..
        }) => {
            runtime_shell.active_script_cursor = None;
            if let Err(error) = consume_visible_script_end_state(runtime_shell) {
                record_visible_runtime_system_error(
                    runtime_shell,
                    anyhow::anyhow!("failed to consume script end state for {script}: {error:#}"),
                );
                return;
            }
            if *callback {
                return;
            }
            if visible_script_call_stack_has_return(runtime_shell) {
                match resume_visible_script_return(runtime_shell) {
                    Ok(()) => {}
                    Err(error) => {
                        runtime_shell.active_script_cursor = None;
                        record_visible_runtime_system_error(
                            runtime_shell,
                            anyhow::anyhow!(
                                "failed to resume script return for {script}: {error:#}"
                            ),
                        );
                        return;
                    }
                }
            } else {
                runtime_shell.active_script_cursor = None;
                runtime_shell
                    .last_audio_events
                    .push(format!("script end={script}"));
                trim_event_log(&mut runtime_shell.last_audio_events);
            }
        }
        RuntimeMutationResult::ScriptControlApplied(ScriptControlAction::Jump {
            target_script,
            call,
            deferred,
            standard,
            ..
        }) => {
            runtime_shell.last_audio_events.push(format!(
                "script branch={} target={} call={} deferred={} standard={}",
                script, target_script, call, deferred, standard
            ));
            trim_event_log(&mut runtime_shell.last_audio_events);
            if *deferred {
                arm_visible_active_script_cursor(runtime_shell, script, next_command_index);
            } else if let Err(error) = take_visible_next_script(runtime_shell) {
                runtime_shell.active_script_cursor = None;
                record_visible_runtime_system_error(
                    runtime_shell,
                    anyhow::anyhow!(
                        "failed to take next script after branch from {script}: {error:#}"
                    ),
                );
            }
        }
        RuntimeMutationResult::ScriptControlApplied(ScriptControlAction::Continue { .. }) => {
            arm_visible_active_script_cursor(runtime_shell, script, next_command_index);
        }
        _ => arm_visible_active_script_cursor(runtime_shell, script, next_command_index),
    }
}

fn visible_script_call_stack_has_return(runtime_shell: &BevyRuntimeShell) -> bool {
    runtime_shell
        .shell
        .snapshot()
        .map(|snapshot| !snapshot.script_events.call_stack.is_empty())
        .unwrap_or(false)
}

fn resume_visible_script_return(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    record_visible_runtime_action(runtime_shell, "script:return:pop")?;
    let returned = runtime_shell.shell.pop_script_call_stack()?;
    runtime_shell.last_audio_events.push(format!(
        "script return={} command={} checksum={:?}",
        returned.frame.source_script, returned.frame.next_command_index, returned.state_checksum
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    arm_visible_active_script_cursor(
        runtime_shell,
        &returned.frame.source_script,
        returned.frame.next_command_index,
    );
    Ok(())
}

fn execute_visible_active_script_step(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let Some(cursor) = runtime_shell.active_script_cursor.clone() else {
        return handle_visible_no_active_script_cursor(runtime_shell, "step");
    };
    if !has_visible_compiled_script_command(
        runtime_shell,
        &cursor.source_script,
        cursor.next_command_index,
    ) {
        runtime_shell.active_script_cursor = None;
        runtime_shell
            .last_audio_events
            .push(format!("script complete={}", cursor.source_script));
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    if open_visible_elevator_for_script_command(
        runtime_shell,
        &cursor.source_script,
        cursor.next_command_index,
    )? {
        arm_visible_active_script_cursor_with_origin(
            runtime_shell,
            &cursor.origin_map_name,
            &cursor.source_script,
            cursor.next_command_index + 1,
        );
        return Ok(());
    }
    if open_visible_gift_pokemon_for_script_command(
        runtime_shell,
        &cursor.source_script,
        cursor.next_command_index,
    )? {
        arm_visible_active_script_cursor_with_origin(
            runtime_shell,
            &cursor.origin_map_name,
            &cursor.source_script,
            cursor.next_command_index + 1,
        );
        return Ok(());
    }
    if open_visible_phone_prompt_for_script_command(
        runtime_shell,
        &cursor.source_script,
        cursor.next_command_index,
    )? {
        arm_visible_active_script_cursor_with_origin(
            runtime_shell,
            &cursor.origin_map_name,
            &cursor.source_script,
            cursor.next_command_index + 1,
        );
        return Ok(());
    }
    if open_visible_day_care_for_script_command(
        runtime_shell,
        &cursor.source_script,
        cursor.next_command_index,
    )? {
        arm_visible_active_script_cursor_with_origin(
            runtime_shell,
            &cursor.origin_map_name,
            &cursor.source_script,
            cursor.next_command_index + 1,
        );
        return Ok(());
    }
    if open_visible_script_party_selection_for_command(
        runtime_shell,
        &cursor.source_script,
        cursor.next_command_index,
    )? {
        arm_visible_active_script_cursor_with_origin(
            runtime_shell,
            &cursor.origin_map_name,
            &cursor.source_script,
            cursor.next_command_index + 1,
        );
        return Ok(());
    }
    if open_visible_buena_prize_for_script_command(
        runtime_shell,
        &cursor.source_script,
        cursor.next_command_index,
    )? {
        arm_visible_active_script_cursor_with_origin(
            runtime_shell,
            &cursor.origin_map_name,
            &cursor.source_script,
            cursor.next_command_index + 1,
        );
        return Ok(());
    }
    if open_visible_slot_machine_for_script_command(
        runtime_shell,
        &cursor.source_script,
        cursor.next_command_index,
    )? {
        arm_visible_active_script_cursor_with_origin(
            runtime_shell,
            &cursor.origin_map_name,
            &cursor.source_script,
            cursor.next_command_index + 1,
        );
        return Ok(());
    }
    if open_visible_card_flip_for_script_command(
        runtime_shell,
        &cursor.source_script,
        cursor.next_command_index,
    )? {
        arm_visible_active_script_cursor_with_origin(
            runtime_shell,
            &cursor.origin_map_name,
            &cursor.source_script,
            cursor.next_command_index + 1,
        );
        return Ok(());
    }
    if open_visible_kurt_apricorn_for_script_command(
        runtime_shell,
        &cursor.source_script,
        cursor.next_command_index,
    )? {
        arm_visible_active_script_cursor_with_origin(
            runtime_shell,
            &cursor.origin_map_name,
            &cursor.source_script,
            cursor.next_command_index + 1,
        );
        return Ok(());
    }
    if apply_visible_name_rival_for_script_command(
        runtime_shell,
        &cursor.source_script,
        cursor.next_command_index,
    )? {
        arm_visible_active_script_cursor_with_origin(
            runtime_shell,
            &cursor.origin_map_name,
            &cursor.source_script,
            cursor.next_command_index + 1,
        );
        return Ok(());
    }
    if open_visible_day_of_week_for_script_command(
        runtime_shell,
        &cursor.source_script,
        cursor.next_command_index,
    )? {
        return Ok(());
    }
    record_visible_runtime_action(
        runtime_shell,
        format!(
            "script:step:{}:{}",
            cursor.source_script, cursor.next_command_index
        ),
    )?;
    let stepped = runtime_shell
        .shell
        .step_compiled_script_command(
            &cursor.origin_map_name,
            &cursor.source_script,
            cursor.next_command_index,
            explicit_compiled_script_runtime_inputs(
                runtime_shell,
                &cursor.source_script,
                cursor.next_command_index,
            )?,
            explicit_compiled_script_phone_inputs(
                runtime_shell,
                &cursor.source_script,
                cursor.next_command_index,
            ),
        )
        .with_context(|| {
            format!(
                "visible script step {}:{}",
                cursor.source_script, cursor.next_command_index
            )
        })?;
    runtime_shell.last_audio_events.push(format!(
        "script step={} command={} result={} checksum={:?}",
        cursor.source_script,
        cursor.next_command_index,
        stepped.mutation.result.result_tag(),
        stepped.mutation.state_checksum
    ));
    integrate_visible_script_mutation_outcome(runtime_shell, &stepped.mutation)?;
    trim_event_log(&mut runtime_shell.last_audio_events);
    if activate_visible_script_boundary_after_outcome(runtime_shell, &stepped.mutation)? {
        arm_visible_active_script_cursor_from_run(runtime_shell, stepped.next_cursor.clone());
        return Ok(());
    }
    if open_visible_vertical_menu_for_script_command(
        runtime_shell,
        &cursor.source_script,
        cursor.next_command_index,
    )? {
        arm_visible_active_script_cursor_from_run(runtime_shell, stepped.next_cursor.clone());
        return Ok(());
    }
    arm_visible_script_cursor_after_step(runtime_shell, &stepped);
    Ok(())
}

fn activate_visible_script_boundary_after_outcome(
    runtime_shell: &mut BevyRuntimeShell,
    outcome: &RuntimeMutationOutcome,
) -> Result<bool> {
    match &outcome.result {
        RuntimeMutationResult::ScriptShopOpened(_) => Ok(true),
        RuntimeMutationResult::ScriptTextApplied(
            crate::core::systems::script_text::ScriptTextAction::WaitButton { .. }
            | crate::core::systems::script_text::ScriptTextAction::YesNo { .. },
        ) => Ok(true),
        RuntimeMutationResult::ScriptedWildBattleStarted(_)
        | RuntimeMutationResult::ScriptedTrainerBattleStarted(_) => {
            prepare_visible_battle_entry(runtime_shell)?;
            Ok(true)
        }
        RuntimeMutationResult::SpecialRoutineApplied(special) => {
            activate_visible_special_routine_boundary(runtime_shell, &special.effect)
        }
        _ => Ok(false),
    }
}

fn integrate_visible_compiled_script_run(
    runtime_shell: &mut BevyRuntimeShell,
    steps: &[crate::RuntimeCompiledScriptStep],
) -> Result<bool> {
    let mut reached_boundary = false;
    let mut blocking_movement_precedes_visible_text = false;
    for (index, step) in steps.iter().enumerate() {
        runtime_shell.last_audio_events.push(format!(
            "script resume step={} command={} result={} checksum={:?}",
            step.source_script,
            step.command_index,
            step.mutation.result.result_tag(),
            step.mutation.state_checksum
        ));
        integrate_visible_script_mutation_outcome(runtime_shell, &step.mutation)?;
        if matches!(
            &step.mutation.result,
            RuntimeMutationResult::ScriptMovementApplied(_)
        ) && steps[index + 1..].iter().any(|later| {
            matches!(
                &later.mutation.result,
                RuntimeMutationResult::ScriptTextApplied(
                    crate::core::systems::script_text::ScriptTextAction::Open { .. }
                        | crate::core::systems::script_text::ScriptTextAction::Write { .. }
                )
            )
        }) {
            blocking_movement_precedes_visible_text = true;
        }
        if activate_visible_script_boundary_after_outcome(runtime_shell, &step.mutation)? {
            reached_boundary = true;
        }
    }
    if blocking_movement_precedes_visible_text {
        // ASM `applymovement` is blocking. Core can evaluate the following
        // `opentext`/`writetext` in the same compiled transaction, but the
        // retained movement scene must represent the earlier LCD state.
        // Reveal the terminal text only after the movement animation ends.
        if let Some(scene) = runtime_shell.visible_script_movement_scene.as_mut() {
            let scene = Arc::make_mut(scene);
            scene.ui.text = None;
            scene.ui.window_open = false;
            scene.ui.text_window_open = false;
            scene.ui.coords = None;
            scene.ui.pending_yes_no = None;
            scene.ui.pending_text_wait = None;
            scene.script_events.text_window_open = false;
            scene.script_events.pending_text_label = None;
            scene.script_events.pending_text_wait = None;
            scene.script_events.pending_yes_no = None;
        }
    }
    // A compiled run mutates the authoritative runtime before Bevy presents
    // the result. Invalidate once per run so presentation-only boundaries
    // such as `pokepic` cannot remain hidden behind a cached pre-command
    // snapshot. Per-step invalidation would add redundant render work.
    if !steps.is_empty() {
        mark_runtime_snapshot_dirty(runtime_shell);
    }
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(reached_boundary)
}

fn integrate_visible_script_mutation_outcome(
    runtime_shell: &mut BevyRuntimeShell,
    outcome: &RuntimeMutationOutcome,
) -> Result<()> {
    match &outcome.result {
        RuntimeMutationResult::ScriptItemGranted(item) => {
            runtime_shell
                .last_audio_events
                .push(format!("script item grant {item:?}"));
            match item {
                crate::core::systems::script_items::ScriptItemGrantOutcome::Granted {
                    item_id,
                    ..
                } => {
                    runtime_shell.pending_item_notification = Some(item_id.clone());
                }
                crate::core::systems::script_items::ScriptItemGrantOutcome::BagFull {
                    ..
                } => {
                    runtime_shell.pending_item_notification = None;
                }
                _ => {}
            }
            set_shell_action_status(runtime_shell, "ITEM GRANT");
        }
        RuntimeMutationResult::SpecialRoutineApplied(special) => {
            if let SpecialRoutineEffect::GetMysteryGiftItem {
                item_id: Some(item_id),
                received: true,
            } = &special.effect
            {
                runtime_shell.pending_item_notification = Some(item_id.clone());
            }
        }
        RuntimeMutationResult::ScriptItemChecked(item) => {
            runtime_shell.last_audio_events.push(format!(
                "script item check {} held={}",
                item.item_id, item.held
            ));
            set_shell_action_status(runtime_shell, format!("ITEM CHECK {}", item.item_id));
        }
        RuntimeMutationResult::ScriptItemTaken(item) => {
            runtime_shell.last_audio_events.push(format!(
                "script item take {} removed={}",
                item.item_id, item.removed
            ));
            set_shell_action_status(runtime_shell, format!("ITEM TAKE {}", item.item_id));
        }
        RuntimeMutationResult::ScriptFieldItemPickedUp(item) => match item {
            FieldItemPickupOutcome::Collected {
                item_id,
                quantity,
                source,
                ..
            } => {
                runtime_shell.last_audio_events.push(format!(
                    "field item collected {} x{} source={source:?}",
                    item_id, quantity
                ));
                set_shell_action_status(runtime_shell, format!("GOT {item_id}"));
            }
            FieldItemPickupOutcome::AlreadyCollected { source, .. } => {
                runtime_shell
                    .last_audio_events
                    .push(format!("field item already collected source={source:?}"));
                set_shell_action_status(runtime_shell, "ALREADY COLLECTED");
            }
            FieldItemPickupOutcome::BagFull {
                item_id,
                quantity,
                source,
                ..
            } => {
                runtime_shell.last_audio_events.push(format!(
                    "field item bag full {} x{} source={source:?}",
                    item_id, quantity
                ));
                set_shell_action_status(runtime_shell, "PACK FULL");
            }
        },
        RuntimeMutationResult::ScriptEconomyApplied(economy) => {
            runtime_shell
                .last_audio_events
                .push(format!("script economy {economy:?}"));
            set_shell_action_status(runtime_shell, "ECONOMY");
        }
        RuntimeMutationResult::ScriptPhoneApplied(phone) => {
            runtime_shell
                .last_audio_events
                .push(format!("script phone {phone:?}"));
            set_shell_action_status(runtime_shell, "PHONE");
        }
        RuntimeMutationResult::ScriptFlagMutated(flag) => {
            runtime_shell.last_audio_events.push(format!(
                "script flag {}={} engine={}",
                flag.flag_id, flag.value, flag.engine_flag
            ));
            set_shell_action_status(runtime_shell, format!("FLAG {}", flag.flag_id));
        }
        RuntimeMutationResult::ScriptFlagChecked(flag) => {
            runtime_shell.last_audio_events.push(format!(
                "script flag check {} set={} engine={}",
                flag.flag_id, flag.set, flag.engine_flag
            ));
            set_shell_action_status(runtime_shell, format!("FLAG CHECK {}", flag.flag_id));
        }
        RuntimeMutationResult::ScriptSceneApplied(scene) => {
            runtime_shell.last_audio_events.push(format!(
                "script scene {} {} index={} script={:?}",
                scene.command, scene.scene_id, scene.scene_index, scene.script_name
            ));
            set_shell_action_status(runtime_shell, format!("SCENE {}", scene.scene_id));
        }
        RuntimeMutationResult::ScriptAudioApplied(_) => {
            drain_visible_audio_events(runtime_shell)?;
        }
        RuntimeMutationResult::ScriptMapApplied(action) => {
            runtime_shell
                .last_audio_events
                .push(format!("script map action {action:?}"));
            match action {
                crate::core::systems::script_warps::ScriptMapAction::Warp {
                    target_map, ..
                } => {
                    set_shell_action_status(runtime_shell, format!("SCRIPT WARP {target_map}"));
                }
                crate::core::systems::script_warps::ScriptMapAction::LoadMap { .. }
                | crate::core::systems::script_warps::ScriptMapAction::RefreshMap { .. } => {
                    set_shell_action_status(runtime_shell, "SCRIPT MAP");
                }
                crate::core::systems::script_warps::ScriptMapAction::NoWarp { .. }
                | crate::core::systems::script_warps::ScriptMapAction::WarpCheck { .. } => {}
            }
        }
        RuntimeMutationResult::ScriptObjectMutated(object) => {
            runtime_shell.last_audio_events.push(format!(
                "script object {} {} ({:?},{:?})->({:?},{:?})",
                object.command,
                object.object_id,
                object.previous_x,
                object.previous_y,
                object.x,
                object.y
            ));
            set_shell_action_status(runtime_shell, format!("OBJECT {}", object.command));
        }
        RuntimeMutationResult::ScriptMovementApplied(movement) => {
            runtime_shell.last_audio_events.push(format!(
                "script movement {} {} ({},{})->({},{}) steps={} facing={:?} program={:?}",
                movement.object_id,
                movement.movement,
                movement.previous_tile.x,
                movement.previous_tile.y,
                movement.tile.x,
                movement.tile.y,
                movement.steps_applied,
                movement.facing,
                movement.executed_steps
            ));
            let mut reported_visible_effect = false;
            for effect in &movement.effects {
                runtime_shell.last_audio_events.push(format!(
                    "script movement effect {} index={}",
                    effect.command, effect.index
                ));
                if let Some(frames) = visible_script_movement_effect_frames(
                    movement,
                    effect.command.as_str(),
                    effect.index,
                )? {
                    runtime_shell.last_audio_events.push(format!(
                        "visible movement effect {} frames={}",
                        effect.command, frames
                    ));
                    set_shell_action_status(
                        runtime_shell,
                        format!("{} {}", effect.command.to_ascii_uppercase(), frames),
                    );
                    reported_visible_effect = true;
                }
            }
            if !reported_visible_effect {
                set_shell_action_status(runtime_shell, format!("MOVE {}", movement.object_id));
            }
            begin_visible_script_movement(runtime_shell, movement)?;
        }
        RuntimeMutationResult::ScriptBlockChanged(block) => {
            runtime_shell.last_audio_events.push(format!(
                "script block changed {} ({},{})",
                block.map_name, block.x, block.y
            ));
            set_shell_action_status(runtime_shell, "BLOCK CHANGED");
        }
        RuntimeMutationResult::ScriptTextApplied(text) => {
            runtime_shell
                .last_audio_events
                .push(format!("script text {text:?}"));
            match text {
                crate::core::systems::script_text::ScriptTextAction::YesNo { .. } => {
                    runtime_shell.yes_no_cursor = Some(MenuCursor {
                        surface_id: "ui:yes-no".to_string(),
                        option_index: 0,
                    });
                    mark_runtime_snapshot_dirty(runtime_shell);
                }
                crate::core::systems::script_text::ScriptTextAction::Close { .. } => {
                    runtime_shell.yes_no_cursor = None;
                }
                _ => {}
            }
            set_shell_action_status(runtime_shell, "TEXT");
        }
        RuntimeMutationResult::ScriptVariableApplied(variable) => {
            runtime_shell
                .last_audio_events
                .push(format!("script variable {variable:?}"));
            set_shell_action_status(runtime_shell, "VARIABLE");
        }
        RuntimeMutationResult::ScriptRuntimeApplied(command, runtime) => {
            runtime_shell.last_audio_events.push(format!(
                "script runtime {} args={:?} outcome={:?}",
                command.command, command.args, runtime
            ));
            if !open_visible_script_runtime_boundary_if_needed(runtime_shell, command)? {
                set_shell_action_status(runtime_shell, format!("RUNTIME {}", command.command));
            }
            if command.command == "closewindow" {
                runtime_shell.visible_balance_overlay = None;
                mark_runtime_snapshot_dirty(runtime_shell);
            }
        }
        RuntimeMutationResult::ScriptSwarmApplied(swarm) => {
            runtime_shell.last_audio_events.push(format!(
                "script swarm {} map={}",
                swarm.swarm_token, swarm.map_id
            ));
            set_shell_action_status(runtime_shell, format!("SWARM {}", swarm.swarm_token));
        }
        _ => {}
    }
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn visible_script_movement_effect_frames(
    movement: &crate::core::systems::script_objects::ScriptMovementOutcome,
    command: &str,
    effect_index: usize,
) -> Result<Option<u16>> {
    match command {
        "step_shake" | "rock_smash" => movement
            .executed_steps
            .iter()
            .find(|step| step.index == effect_index)
            .and_then(|step| step.duration)
            .map(Some)
            .with_context(|| {
                format!(
                    "script movement {} effect {} index={} has no exact duration",
                    movement.movement, command, effect_index
                )
            }),
        "tree_shake" => Ok(Some(24)),
        "skyfall" => Ok(Some(32)),
        "skyfall_top" => Ok(Some(16)),
        "fish_cast_rod" | "fish_got_bite" => Ok(Some(0)),
        _ => Ok(None),
    }
}

fn begin_visible_script_movement(
    runtime_shell: &mut BevyRuntimeShell,
    movement: &crate::core::systems::script_objects::ScriptMovementOutcome,
) -> Result<()> {
    let mut tile = movement.previous_tile;
    let mut fixed_facing = false;
    let mut phases = VecDeque::new();
    for step in &movement.executed_steps {
        let command = step.command.as_str();
        match command {
            "fix_facing" => fixed_facing = true,
            "remove_fixed_facing" => fixed_facing = false,
            "step" | "turn_step" | "slide_step" | "slow_step" | "slow_slide_step"
            | "fast_step" | "big_step" | "fast_slide_step" | "jump_step"
            | "slow_jump_step" | "fast_jump_step" => {
                let direction = visible_script_movement_direction(step)?;
                let jump = command.contains("jump_step");
                let stride = if jump { 2 } else { 1 };
                let to = crate::core::world::movement::checked_move_by_stride(
                    tile,
                    direction,
                    stride,
                )
                .with_context(|| {
                    format!(
                        "visible script movement {} {} overflows from ({},{})",
                        movement.movement, command, tile.x, tile.y
                    )
                })?;
                let base = if command.starts_with("slow_") {
                    WALK_FRAME_HOLD_TICKS.saturating_mul(2)
                } else if command.starts_with("fast_") || command == "big_step" {
                    WALK_FRAME_HOLD_TICKS / 2
                } else {
                    WALK_FRAME_HOLD_TICKS
                };
                phases.push_back(VisibleScriptMovementPhase::Move {
                    from: tile,
                    to,
                    direction,
                    duration: if jump { base.saturating_mul(2) } else { base }.max(1),
                    jump,
                    update_facing: !fixed_facing,
                    standing_frame: jump || command.contains("slide_step"),
                });
                tile = to;
            }
            "turn_head" | "turn_in" | "turn_waterfall" | "step_bump" | "turn_away" => {
                let mut direction = visible_script_movement_direction(step)?;
                if command == "turn_away" {
                    direction = visible_opposite_direction(direction);
                }
                let duration = match command {
                    "turn_away" => WALK_FRAME_HOLD_TICKS.saturating_mul(2),
                    "turn_in" => WALK_FRAME_HOLD_TICKS,
                    "turn_waterfall" => WALK_FRAME_HOLD_TICKS / 2,
                    _ => 1,
                };
                phases.push_back(VisibleScriptMovementPhase::Turn {
                    direction,
                    duration,
                });
            }
            "step_sleep" => phases.push_back(VisibleScriptMovementPhase::Hold {
                duration: step.duration.unwrap_or(u16::from(WALK_FRAME_HOLD_TICKS)),
            }),
            command if command.starts_with("step_sleep_") => {
                let duration = command
                    .strip_prefix("step_sleep_")
                    .and_then(|value| value.parse::<u16>().ok())
                    .with_context(|| format!("invalid exact movement sleep {command}"))?;
                phases.push_back(VisibleScriptMovementPhase::Hold { duration });
            }
            "teleport_from" => {
                phases.push_back(VisibleScriptMovementPhase::Stationary {
                    duration: 16,
                    effect: VisibleStationaryMovementEffect::TeleportSpin,
                });
                phases.push_back(VisibleScriptMovementPhase::Stationary {
                    duration: 16,
                    effect: VisibleStationaryMovementEffect::TeleportRise,
                });
            }
            "teleport_to" => {
                phases.push_back(VisibleScriptMovementPhase::Stationary {
                    duration: 17,
                    effect: VisibleStationaryMovementEffect::TeleportWait,
                });
                phases.push_back(VisibleScriptMovementPhase::Stationary {
                    duration: 16,
                    effect: VisibleStationaryMovementEffect::TeleportDescent,
                });
                phases.push_back(VisibleScriptMovementPhase::Stationary {
                    duration: 16,
                    effect: VisibleStationaryMovementEffect::TeleportSpin,
                });
            }
            "skyfall_top" => phases.push_back(VisibleScriptMovementPhase::Stationary {
                duration: 16,
                effect: VisibleStationaryMovementEffect::SkyfallTop,
            }),
            "skyfall" => {
                phases.push_back(VisibleScriptMovementPhase::Stationary {
                    duration: 16,
                    effect: VisibleStationaryMovementEffect::SkyfallWait,
                });
                phases.push_back(VisibleScriptMovementPhase::Stationary {
                    duration: 16,
                    effect: VisibleStationaryMovementEffect::SkyfallFall,
                });
            }
            "tree_shake" => phases.push_back(VisibleScriptMovementPhase::TreeShake { duration: 24 }),
            "remove_object" | "hide_object" => {
                phases.push_back(VisibleScriptMovementPhase::Visibility { hidden: true });
            }
            "show_object" => {
                phases.push_back(VisibleScriptMovementPhase::Visibility { hidden: false });
            }
            "step_dig" => {
                phases.push_back(VisibleScriptMovementPhase::Stationary {
                    duration: step.duration.context("movement effect missing duration")?,
                    effect: VisibleStationaryMovementEffect::DigSpin,
                });
                phases.push_back(VisibleScriptMovementPhase::Visibility { hidden: true });
            }
            "return_dig" => {
                phases.push_back(VisibleScriptMovementPhase::Visibility { hidden: false });
                phases.push_back(VisibleScriptMovementPhase::Stationary {
                    duration: step.duration.context("movement effect missing duration")?,
                    effect: VisibleStationaryMovementEffect::DigSpin,
                });
            }
            "step_shake" => phases.push_back(VisibleScriptMovementPhase::ScreenShake {
                parameter: step.duration.context("screen shake movement missing parameter")?,
            }),
            "rock_smash" => phases.push_back(VisibleScriptMovementPhase::Stationary {
                duration: step.duration.context("Rock Smash movement missing duration")?,
                effect: VisibleStationaryMovementEffect::RockSmash,
            }),
            "step_wait_end" => {
                phases.push_back(VisibleScriptMovementPhase::Hold {
                    duration: step.duration.context("movement effect missing duration")?,
                });
            }
            _ => {}
        }
    }
    if phases.is_empty() {
        return Ok(());
    }
    if tile != movement.tile {
        anyhow::bail!(
            "visible script movement {} reconstructed ({},{}) but authority ended at ({},{})",
            movement.movement,
            tile.x,
            tile.y,
            movement.tile.x,
            movement.tile.y
        );
    }
    if runtime_shell.visible_script_movement.is_some() {
        let current_object = runtime_shell
            .visible_script_movement
            .as_ref()
            .map(|active| active.object_id.as_str());
        if current_object != Some(movement.object_id.as_str()) {
            let revealed_object = if !movement.previous_hidden && movement.object_id != "PLAYER" {
                runtime_shell
                    .shell
                    .session()
                    .overworld
                    .objects
                    .iter()
                    .enumerate()
                    .find(|(_, object)| {
                        object.object_identifier.as_deref() == Some(movement.object_id.as_str())
                    })
                    .map(|(slot, object)| (slot, object.clone()))
            } else {
                None
            };
            let scene = Arc::make_mut(
                runtime_shell
                    .visible_script_movement_scene
                    .as_mut()
                    .context("queued visible script movement has no retained scene")?,
            );
            if movement.object_id == "PLAYER" {
                scene.overworld.tile = movement.previous_tile;
                scene.overworld.facing = movement.previous_facing;
                scene.overworld_player_hidden = movement.previous_hidden;
            } else {
                scene
                    .visible_object_runtime_tiles
                    .insert(movement.object_id.clone(), movement.previous_tile);
                scene
                    .visible_object_facings
                    .insert(movement.object_id.clone(), movement.previous_facing);
                if movement.previous_hidden {
                    if let Some(index) = scene.visible_objects.iter().position(|object| {
                        object.object_identifier.as_deref() == Some(movement.object_id.as_str())
                    }) {
                        scene.visible_objects.remove(index);
                        scene.visible_object_slots.remove(index);
                    }
                } else if !scene.visible_objects.iter().any(|object| {
                    object.object_identifier.as_deref() == Some(movement.object_id.as_str())
                }) {
                    let (slot, object) = revealed_object.with_context(|| {
                        format!("queued movement cannot restore unknown object {}", movement.object_id)
                    })?;
                    scene.visible_objects.push(object);
                    scene.visible_object_slots.push(slot);
                }
            }
        }
        runtime_shell
            .visible_script_movement
            .as_mut()
            .context("visible script movement disappeared while queueing program")?
            .pending_programs
            .push_back(VisibleScriptMovementProgram {
                object_id: movement.object_id.clone(),
                previous_tile: movement.previous_tile,
                previous_facing: movement.previous_facing,
                previous_hidden: movement.previous_hidden,
                phases,
                follower_object_id: movement
                    .previous_follower
                    .as_ref()
                    .map(|follower| follower.object_id.clone()),
                follower_queued_step: initial_visible_follower_step(movement),
            });
        mark_runtime_snapshot_dirty(runtime_shell);
        return Ok(());
    }
    let mut scene = runtime_shell.shell.snapshot()?;
    if movement.object_id == "PLAYER" {
        scene.overworld.tile = movement.previous_tile;
        scene.overworld.facing = movement.previous_facing;
        scene.overworld_player_hidden = movement.previous_hidden;
    } else {
        scene
            .visible_object_runtime_tiles
            .insert(movement.object_id.clone(), movement.previous_tile);
        scene
            .visible_object_facings
            .insert(movement.object_id.clone(), movement.previous_facing);
        if movement.previous_hidden {
            if let Some(index) = scene.visible_objects.iter().position(|object| {
                object.object_identifier.as_deref() == Some(movement.object_id.as_str())
            }) {
                scene.visible_objects.remove(index);
                scene.visible_object_slots.remove(index);
            }
        } else if !scene.visible_objects.iter().any(|object| {
            object.object_identifier.as_deref() == Some(movement.object_id.as_str())
        }) {
            if let Some((slot, object)) = runtime_shell
                .shell
                .session()
                .overworld
                .objects
                .iter()
                .enumerate()
                .find(|(_, object)| {
                    object.object_identifier.as_deref() == Some(movement.object_id.as_str())
                })
            {
                scene.visible_objects.push(object.clone());
                scene.visible_object_slots.push(slot);
            }
        }
    }
    if movement.object_id == "PLAYER" {
        runtime_shell.player_walk_stride = false;
        runtime_shell.player_walk_mirror_stride = false;
    }
    restore_visible_follower_origin(&mut scene, movement)?;
    runtime_shell.visible_script_movement_scene = Some(Arc::new(scene));
    runtime_shell.visible_script_movement = Some(VisibleScriptMovement {
        object_id: movement.object_id.clone(),
        phases,
        pending_programs: VecDeque::new(),
        hold_frames_remaining: 0,
        active_jump_duration: None,
        active_uses_standing_frame: false,
        active_tree_shake_duration: None,
        active_stationary_effect: None,
        active_stationary_duration: 0,
        stationary_y_offset: 0,
        stationary_initial_facing: movement.previous_facing,
        follower_object_id: movement
            .previous_follower
            .as_ref()
            .map(|follower| follower.object_id.clone()),
        follower_queued_step: initial_visible_follower_step(movement),
        follower_active_jump_duration: None,
        follower_active_uses_standing_frame: false,
    });
    start_next_visible_script_movement_phase(runtime_shell)?;
    Ok(())
}

fn restore_visible_follower_origin(
    scene: &mut crate::RuntimeShellSnapshot,
    movement: &crate::core::systems::script_objects::ScriptMovementOutcome,
) -> Result<()> {
    let Some(follower) = movement.previous_follower.as_ref() else {
        return Ok(());
    };
    if follower.object_id == "PLAYER" {
        scene.overworld.tile = follower.tile;
        scene.overworld.facing = follower.facing;
    } else {
        scene
            .visible_object_runtime_tiles
            .insert(follower.object_id.clone(), follower.tile);
        scene
            .visible_object_facings
            .insert(follower.object_id.clone(), follower.facing);
    }
    Ok(())
}

fn initial_visible_follower_step(
    movement: &crate::core::systems::script_objects::ScriptMovementOutcome,
) -> Option<VisibleFollowerStep> {
    let follower = movement.previous_follower.as_ref()?;
    if let Some(queued) = follower.queued_step {
        return Some(VisibleFollowerStep {
            direction: queued.direction,
            stride: u8::try_from(queued.stride).ok()?,
            duration: queued.duration,
            jump: queued.jump,
            standing_frame: queued.standing_frame,
        });
    }
    let direction = if movement.previous_tile.x > follower.tile.x {
        Direction::Right
    } else if movement.previous_tile.x < follower.tile.x {
        Direction::Left
    } else if movement.previous_tile.y > follower.tile.y {
        Direction::Down
    } else if movement.previous_tile.y < follower.tile.y {
        Direction::Up
    } else {
        return None;
    };
    Some(VisibleFollowerStep {
        direction,
        stride: 1,
        duration: WALK_FRAME_HOLD_TICKS,
        jump: false,
        standing_frame: false,
    })
}

fn begin_visible_follower_step(
    runtime_shell: &mut BevyRuntimeShell,
    leader_direction: Direction,
    leader_stride: u8,
    leader_duration: u8,
    leader_jump: bool,
    leader_standing_frame: bool,
) -> Result<()> {
    let (follower_object_id, queued_step) = {
        let movement = runtime_shell
            .visible_script_movement
            .as_mut()
            .context("visible follower step lost its movement program")?;
        let follower_object_id = movement.follower_object_id.clone();
        let queued_step = movement.follower_queued_step.take();
        if follower_object_id.is_some() {
            movement.follower_queued_step = Some(VisibleFollowerStep {
                direction: leader_direction,
                stride: leader_stride,
                duration: leader_duration,
                jump: leader_jump,
                standing_frame: leader_standing_frame,
            });
        }
        movement.follower_active_jump_duration = queued_step
            .as_ref()
            .and_then(|step| step.jump.then_some(step.duration));
        movement.follower_active_uses_standing_frame = queued_step
            .as_ref()
            .is_some_and(|step| step.standing_frame);
        (follower_object_id, queued_step)
    };
    let (Some(follower_object_id), Some(queued_step)) = (follower_object_id, queued_step) else {
        return Ok(());
    };
    let (follower_from, direction) = {
        let scene = Arc::make_mut(
            runtime_shell
                .visible_script_movement_scene
                .as_mut()
                .context("visible follower step has no retained scene")?,
        );
        let follower_from = if follower_object_id == "PLAYER" {
            scene.overworld.tile
        } else {
            scene
                .visible_object_runtime_tiles
                .get(&follower_object_id)
                .copied()
                .with_context(|| format!("visible follower {follower_object_id} has no retained tile"))?
        };
        let direction = queued_step.direction;
        let follower_to = crate::core::world::movement::checked_move_by_stride(
            follower_from,
            direction,
            i16::from(queued_step.stride),
        )
        .context("visible queued follower step overflows runtime coordinates")?;
        if follower_object_id == "PLAYER" {
            scene.overworld.tile = follower_to;
            scene.overworld.facing = direction;
        } else {
            scene
                .visible_object_runtime_tiles
                .insert(follower_object_id.clone(), follower_to);
            scene
                .visible_object_facings
                .insert(follower_object_id.clone(), direction);
        }
        (follower_from, direction)
    };
    if follower_object_id == "PLAYER" {
        runtime_shell.player_walk_from = Some(follower_from);
        runtime_shell.player_walk_total_ticks = queued_step.duration;
        runtime_shell.player_walk_frame_ticks = queued_step.duration;
        runtime_shell.player_walk_stride = !runtime_shell.player_walk_stride;
    } else {
        advance_object_walk_phase(runtime_shell, &follower_object_id, direction);
        runtime_shell
            .object_walk_from
            .insert(follower_object_id.clone(), follower_from);
        runtime_shell
            .object_walk_total_ticks_by_id
            .insert(follower_object_id.clone(), queued_step.duration);
        runtime_shell
            .object_walk_frame_ticks_by_id
            .insert(follower_object_id, queued_step.duration);
    }
    Ok(())
}

fn visible_script_movement_direction(
    step: &crate::core::systems::script_objects::ScriptMovementStep,
) -> Result<Direction> {
    match step.direction.as_deref().map(str::to_ascii_lowercase).as_deref() {
        Some("up") => Ok(Direction::Up),
        Some("down") => Ok(Direction::Down),
        Some("left") => Ok(Direction::Left),
        Some("right") => Ok(Direction::Right),
        _ => anyhow::bail!(
            "script movement {} index={} has no exact direction",
            step.command,
            step.index
        ),
    }
}

const fn visible_opposite_direction(direction: Direction) -> Direction {
    match direction {
        Direction::Up => Direction::Down,
        Direction::Down => Direction::Up,
        Direction::Left => Direction::Right,
        Direction::Right => Direction::Left,
    }
}

fn clear_visible_non_pc_surfaces(runtime_shell: &mut BevyRuntimeShell) {
    runtime_shell.start_menu_cursor = None;
    runtime_shell.menu_cursor = None;
    runtime_shell.kurt_apricorn_cursor = None;
    runtime_shell.kurt_apricorn_quantity = None;
    runtime_shell.buena_prize_cursor = None;
    runtime_shell.visible_unown_puzzle = None;
    runtime_shell.visible_slot_machine = None;
    runtime_shell.visible_card_flip = None;
    close_visible_party_detail_state(runtime_shell);
    runtime_shell.pokedex_menu_open = false;
    runtime_shell.pokedex_detail_open = false;
    runtime_shell.pokedex_scripted_entry = false;
    runtime_shell.pokegear_menu_open = false;
    runtime_shell.pokegear_phone_status = None;
    runtime_shell.trainer_card_open = false;
    runtime_shell.trainer_card_page = VisibleTrainerCardPage::Info;
    runtime_shell.trainer_card_colon_visible = false;
    runtime_shell.trainer_card_colon_ticks = 0;
    runtime_shell.trainer_card_badge_frame = 0;
    runtime_shell.trainer_card_badge_ticks = 0;
    runtime_shell.options_menu_open = false;
    runtime_shell.save_menu_open = false;
    runtime_shell.save_flow = None;
    runtime_shell.special_boundary = None;
    runtime_shell.special_boundary_queue.clear();
    runtime_shell.pending_special_cry = None;
    runtime_shell.pending_special_sound = None;
    close_visible_field_pack_without_log(runtime_shell);
    reset_visible_battle_action_cursors(runtime_shell);
}

fn activate_visible_special_routine_boundary(
    runtime_shell: &mut BevyRuntimeShell,
    effect: &SpecialRoutineEffect,
) -> Result<bool> {
    match effect {
        SpecialRoutineEffect::PokemonCenterPc { .. } => {
            let snapshot = runtime_shell.shell.snapshot()?;
            clear_visible_non_pc_surfaces(runtime_shell);
            if snapshot.storage.party_count == 0 {
                runtime_shell.pc_hub_session_open = false;
                runtime_shell.pc_hub_cursor = None;
                runtime_shell.bill_pc_session_open = false;
                runtime_shell.bill_pc_action_cursor = None;
                runtime_shell.bill_pc_box_cursor = None;
                runtime_shell.bill_pc_move_open = false;
                runtime_shell.bill_pc_move_party_open = false;
                runtime_shell.bill_pc_move_source = None;
                runtime_shell.storage_cursor = None;
                runtime_shell.pc_item_cursor = None;
                runtime_shell.special_boundary = Some(SpecialBoundaryDisplay {
                    label: "PokecenterPCCantUseText".to_string(),
                    details: vec!["BZZZT! YOU MUST HAVE A POKEMON TO USE THIS!".to_string()],
                });
                set_shell_action_status(runtime_shell, "CAN'T USE THE PC");
                return Ok(true);
            }
            runtime_shell.pc_hub_session_open = true;
            runtime_shell.bill_pc_session_open = false;
            runtime_shell.bill_pc_action_cursor = None;
            runtime_shell.bill_pc_box_cursor = None;
            runtime_shell.bill_pc_move_open = false;
            runtime_shell.bill_pc_move_party_open = false;
            runtime_shell.bill_pc_move_source = None;
            runtime_shell.pc_hub_cursor = Some(MenuCursor {
                surface_id: "pc:hub".to_string(),
                option_index: 0,
            });
            runtime_shell.storage_cursor = None;
            runtime_shell.pc_item_cursor = None;
            runtime_shell.last_audio_events.push(format!(
                "opened Pokemon Center PC hub current_box={}",
                snapshot.storage.current_pc_box
            ));
            set_shell_action_status(runtime_shell, "ACCESS WHOSE PC?");
            trim_event_log(&mut runtime_shell.last_audio_events);
            Ok(true)
        }
        SpecialRoutineEffect::PlayersHousePc { .. } => {
            let player_name = runtime_shell.shell.snapshot()?.trainer.player_name;
            clear_visible_non_pc_surfaces(runtime_shell);
            runtime_shell.pc_hub_session_open = false;
            runtime_shell.pc_hub_cursor = None;
            runtime_shell.bill_pc_session_open = false;
            runtime_shell.bill_pc_action_cursor = None;
            runtime_shell.bill_pc_box_cursor = None;
            runtime_shell.bill_pc_move_open = false;
            runtime_shell.bill_pc_move_party_open = false;
            runtime_shell.bill_pc_move_source = None;
            runtime_shell.player_pc_action_cursor = Some(MenuCursor {
                surface_id: "pc:player-actions".to_string(),
                option_index: 0,
            });
            runtime_shell.pc_item_cursor = None;
            runtime_shell.pc_item_action = None;
            runtime_shell.pc_item_quantity = None;
            runtime_shell.storage_cursor = None;
            runtime_shell.field_notice = Some(format!("{player_name} turned on\nthe PC."));
            runtime_shell
                .last_audio_events
                .push("opened player's house PC".to_string());
            set_shell_action_status(runtime_shell, "PLAYER'S PC");
            trim_event_log(&mut runtime_shell.last_audio_events);
            Ok(true)
        }
        SpecialRoutineEffect::OverworldTownMap { .. } => {
            open_visible_pokegear_menu(runtime_shell)?;
            runtime_shell.pokegear_page = PokegearPage::Map;
            let snapshot = runtime_shell.shell.snapshot()?;
            let region_indices = visible_pokegear_landmark_indices(&snapshot);
            let current_landmark = snapshot
                .presentation
                .pokegear_landmarks
                .map_to_landmark
                .get(&snapshot.overworld.map_name);
            runtime_shell.pokegear_cursor = current_landmark
                .and_then(|constant| {
                    region_indices.iter().copied().find(|index| {
                        snapshot.presentation.pokegear_landmarks.landmarks[*index].constant
                            == *constant
                    })
                })
                .or_else(|| region_indices.first().copied())
                .unwrap_or(0);
            set_shell_action_status(runtime_shell, "TOWN MAP");
            Ok(true)
        }
        SpecialRoutineEffect::MapRadio { station } => {
            open_visible_pokegear_menu(runtime_shell)?;
            runtime_shell.pokegear_page = PokegearPage::Radio;
            runtime_shell.pokegear_radio_station = Some(station.clone());
            runtime_shell.pokegear_radio_segment = 0;
            runtime_shell.last_audio_events.push(format!(
                "opened map radio station={station}"
            ));
            set_shell_action_status(runtime_shell, format!("RADIO {station}"));
            trim_event_log(&mut runtime_shell.last_audio_events);
            Ok(true)
        }
        SpecialRoutineEffect::WarpToSpawnPoint {
            spawn_identifier,
            map_name,
            tile,
        } => {
            runtime_shell.last_audio_events.push(format!(
                "spawn warp special spawn={spawn_identifier} map={map_name} tile=({}, {})",
                tile.x, tile.y
            ));
            trim_event_log(&mut runtime_shell.last_audio_events);
            if runtime_shell
                .shell
                .snapshot()?
                .script_events
                .pending_script_warp
                .is_some()
            {
                execute_visible_pending_script_warp(runtime_shell)?;
            } else {
                settle_visible_overworld_arrival(runtime_shell, "spawn_warp")?;
            }
            Ok(true)
        }
        SpecialRoutineEffect::HealParty { healed_slots } => {
            runtime_shell.last_audio_events.push(format!(
                "healed party slots={}",
                healed_slots
                    .iter()
                    .map(|slot| slot.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            ));
            trim_event_log(&mut runtime_shell.last_audio_events);
            Ok(false)
        }
        SpecialRoutineEffect::FadeOutMusic { .. } => {
            if runtime_shell
                .shell
                .snapshot()?
                .script_events
                .pending_music_fade
                .is_some()
            {
                take_visible_pending_music_fade(runtime_shell)?;
                Ok(true)
            } else {
                drain_visible_audio_events(runtime_shell)?;
                Ok(false)
            }
        }
        SpecialRoutineEffect::WaitSfx => {
            drain_visible_audio_events(runtime_shell)?;
            if runtime_shell
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
                trim_event_log(&mut runtime_shell.last_audio_events);
            }
            Ok(false)
        }
        SpecialRoutineEffect::PlayMapMusic | SpecialRoutineEffect::RestartMapMusic => {
            runtime_shell.heal_music_active = false;
            drain_visible_audio_events(runtime_shell)?;
            queue_visible_current_music(runtime_shell)?;
            Ok(true)
        }
        SpecialRoutineEffect::PlayCurMonCry { species, .. }
        | SpecialRoutineEffect::PlaySlowCry { species, .. } => {
            queue_visible_pokemon_cry(runtime_shell, species, "special")?;
            drain_visible_audio_events(runtime_shell)?;
            Ok(true)
        }
        SpecialRoutineEffect::GraphicsCommand {
            kind: crate::core::state::ScriptGraphicsRuntimeKind::HealMachineAnim,
        } => {
            let snapshot = runtime_shell.shell.snapshot()?;
            let script_value = snapshot
                .script_events
                .script_value
                .as_deref()
                .context("HealMachineAnim requires wScriptVar")?;
            let kind = match script_value {
                "HEALMACHINE_POKECENTER" | "0" => 0,
                "HEALMACHINE_ELMS_LAB" | "1" => 1,
                "HEALMACHINE_HALL_OF_FAME" | "2" => 2,
                value => anyhow::bail!(
                    "HealMachineAnim wScriptVar must name one of the three source animation types or contain 0, 1, or 2; got {value}"
                ),
            };
            let party_count = u8::try_from(snapshot.party.slots.len())
                .context("HealMachineAnim party count exceeds u8")?;
            if party_count == 0 {
                drain_visible_non_audio_script_events_without_record(runtime_shell)?;
                return Ok(false);
            }
            runtime_shell.visible_heal_machine = Some(VisibleHealMachine {
                kind,
                party_count,
                frame: 0,
            });
            drain_visible_non_audio_script_events_without_record(runtime_shell)?;
            mark_runtime_snapshot_dirty(runtime_shell);
            Ok(true)
        }
        SpecialRoutineEffect::RuntimeVisualCommand {
            kind: crate::core::state::ScriptGraphicsRuntimeKind::MagnetTrain,
        } => {
            let snapshot = runtime_shell.shell.snapshot()?;
            let script_value = snapshot
                .script_events
                .script_value
                .as_deref()
                .context("MagnetTrain requires wScriptVar")?;
            let direction_to_goldenrod = match script_value {
                "TRUE" | "1" => true,
                "FALSE" | "0" => false,
                value => anyhow::bail!(
                    "MagnetTrain wScriptVar must be TRUE/FALSE or 1/0, got {value}"
                ),
            };
            let (direction, position, hold_position, final_position) =
                if direction_to_goldenrod {
                    (-1, -96, -64, 96)
                } else {
                    (1, 96, 64, -96)
                };
            runtime_shell.visible_magnet_train = Some(VisibleMagnetTrain {
                direction,
                hold_position,
                final_position,
                position,
                offset: position,
                wait_counter: 0,
                phase: 0,
                arrival_sfx_played: false,
            });
            queue_visible_magnet_train_music(runtime_shell)?;
            drain_visible_non_audio_script_events_without_record(runtime_shell)?;
            mark_runtime_snapshot_dirty(runtime_shell);
            Ok(true)
        }
        SpecialRoutineEffect::RuntimeVisualCommand {
            kind: crate::core::state::ScriptGraphicsRuntimeKind::DisplayUnownWords,
        } => {
            let snapshot = runtime_shell.shell.snapshot()?;
            let value = snapshot
                .script_events
                .script_value
                .as_deref()
                .context("DisplayUnownWords requires wScriptVar")?;
            let word = match value {
                "UNOWNWORDS_ESCAPE" | "0" => "ESCAPE",
                "UNOWNWORDS_LIGHT" | "1" => "LIGHT",
                "UNOWNWORDS_WATER" | "2" => "WATER",
                "UNOWNWORDS_HO_OH" | "3" => "HO-OH",
                value => anyhow::bail!("unknown Unown word constant {value}"),
            };
            runtime_shell.visible_unown_words = Some(word.to_string());
            drain_visible_non_audio_script_events_without_record(runtime_shell)?;
            mark_runtime_snapshot_dirty(runtime_shell);
            Ok(true)
        }
        SpecialRoutineEffect::RuntimeVisualCommand {
            kind: crate::core::state::ScriptGraphicsRuntimeKind::Diploma,
        } => {
            runtime_shell.visible_diploma = Some(0);
            drain_visible_non_audio_script_events_without_record(runtime_shell)?;
            mark_runtime_snapshot_dirty(runtime_shell);
            Ok(true)
        }
        SpecialRoutineEffect::GraphicsCommand { .. }
        | SpecialRoutineEffect::RuntimeVisualCommand { .. } => {
            drain_visible_non_audio_script_events_without_record(runtime_shell)?;
            Ok(false)
        }
        SpecialRoutineEffect::ScreenFade { .. } => {
            if runtime_shell
                .shell
                .snapshot()?
                .script_events
                .pending_screen_fade
                .is_some()
            {
                take_visible_pending_screen_fade(runtime_shell)?;
                Ok(true)
            } else {
                drain_visible_non_audio_script_events_without_record(runtime_shell)?;
                Ok(false)
            }
        }
        SpecialRoutineEffect::QuickSave { requested } => {
            if *requested {
                quick_save_from_script(runtime_shell)?;
                continue_visible_script_after_prompt(runtime_shell)?;
                Ok(true)
            } else {
                runtime_shell
                    .last_audio_events
                    .push("special quick save not requested".to_string());
                trim_event_log(&mut runtime_shell.last_audio_events);
                Ok(false)
            }
        }
        SpecialRoutineEffect::BugContestJudging { placements, .. } => {
            runtime_shell.special_boundary_queue.clear();
            for placement in placements.iter().rev() {
                let announcement = match placement.place {
                    1 => format!(
                        "This Bug-Catching Contest winner is… {}, who caught a {}!",
                        placement.trainer_name,
                        canonical_species_display_name(&placement.species)
                    ),
                    2 => format!(
                        "Placing second was {}, who caught a {}!",
                        placement.trainer_name,
                        canonical_species_display_name(&placement.species)
                    ),
                    _ => format!(
                        "Placing third was {}, who caught a {}!",
                        placement.trainer_name,
                        canonical_species_display_name(&placement.species)
                    ),
                };
                runtime_shell
                    .special_boundary_queue
                    .push_back(SpecialBoundaryDisplay {
                        label: "BugContestJudging".to_string(),
                        details: vec![announcement],
                    });
                runtime_shell
                    .special_boundary_queue
                    .push_back(SpecialBoundaryDisplay {
                        label: "BugContestJudging".to_string(),
                        details: vec![format!(
                            "{} was {} points!",
                            if placement.place == 1 {
                                "The winning score"
                            } else {
                                "The score"
                            },
                            placement.score
                        )],
                    });
            }
            runtime_shell.special_boundary = runtime_shell.special_boundary_queue.pop_front();
            set_shell_action_status(runtime_shell, "CONTEST RESULTS");
            Ok(runtime_shell.special_boundary.is_some())
        }
        SpecialRoutineEffect::GameCornerPrizeMonCheckDex {
            species,
            recorded_caught,
            ..
        } => {
            if !recorded_caught {
                return Ok(false);
            }
            let snapshot = runtime_shell.shell.snapshot()?;
            let species_index = snapshot
                .pokemon
                .iter()
                .position(|entry| entry.species_id == *species)
                .with_context(|| format!("new Pokedex entry species {species} is missing"))?;
            runtime_shell.pokedex_cursor = species_index;
            open_visible_pokedex_menu(runtime_shell)?;
            inspect_visible_pokedex_selection(runtime_shell)?;
            runtime_shell.pokedex_scripted_entry = true;
            set_shell_action_status(runtime_shell, format!("NEW POKEDEX ENTRY {species}"));
            mark_runtime_snapshot_dirty(runtime_shell);
            Ok(true)
        }
        SpecialRoutineEffect::PlaceMoneyTopRight { money, .. } => {
            runtime_shell.visible_balance_overlay =
                Some(VisibleBalanceOverlay::MoneyTopRight { money: *money });
            mark_runtime_snapshot_dirty(runtime_shell);
            Ok(false)
        }
        SpecialRoutineEffect::DisplayMoneyAndCoinBalance { money, coins, .. } => {
            runtime_shell.visible_balance_overlay = Some(VisibleBalanceOverlay::MoneyAndCoins {
                money: *money,
                coins: *coins,
            });
            mark_runtime_snapshot_dirty(runtime_shell);
            Ok(false)
        }
        SpecialRoutineEffect::DisplayCoinCaseBalance { coins, .. } => {
            runtime_shell.visible_balance_overlay =
                Some(VisibleBalanceOverlay::CoinsTopRight { coins: *coins });
            mark_runtime_snapshot_dirty(runtime_shell);
            Ok(false)
        }
        SpecialRoutineEffect::BankOfMom { initialized, .. } => {
            runtime_shell.visible_mom_bank = Some(VisibleMomBank {
                phase: if *initialized {
                    VisibleMomBankPhase::AccessQuestion
                } else {
                    VisibleMomBankPhase::InitializeQuestion
                },
                menu_index: 0,
                yes_no_index: 0,
                amount: 0,
                digit: 5,
                messages: if *initialized {
                    [
                        "Hi! Welcome home!\nYou're trying very hard, I see.",
                        "I've kept your room tidy.",
                    ]
                    .into_iter()
                    .map(str::to_string)
                    .collect()
                } else {
                    [
                        "Wow, that's a cute POKéMON.\nWhere did you get it?",
                        "…",
                        "So, you're leaving on an adventure…",
                        "OK!\nI'll help too.",
                        "But what can I do for you?",
                        "I know! I'll save money for you.",
                        "On a long journey, money's important.",
                    ]
                    .into_iter()
                    .map(str::to_string)
                    .collect()
                },
                close_after_messages: false,
            });
            set_shell_action_status(runtime_shell, "MOM'S BANK");
            mark_runtime_snapshot_dirty(runtime_shell);
            Ok(true)
        }
        SpecialRoutineEffect::DayCareInteraction {
            action,
            success,
            ..
        } if action == "collect_egg" => {
            let snapshot = runtime_shell.shell.snapshot()?;
            runtime_shell.special_boundary_queue.clear();
            let messages = if *success {
                vec![
                    format!("{} received\nthe EGG!", snapshot.trainer.player_name),
                    "Take good care of\nit.".to_string(),
                ]
            } else if snapshot.day_care.egg_present {
                vec!["You have no room\nin your party.\nCome back later.".to_string()]
            } else {
                vec!["Not yet…".to_string()]
            };
            let mut messages = messages.into_iter();
            let first = messages.next().context("day-care result has no visible text")?;
            runtime_shell.special_boundary = Some(SpecialBoundaryDisplay {
                label: "DayCareEgg".to_string(),
                details: vec![first],
            });
            runtime_shell.special_boundary_queue.extend(messages.map(|message| {
                SpecialBoundaryDisplay {
                    label: "DayCareEgg".to_string(),
                    details: vec![message],
                }
            }));
            runtime_shell.pending_special_sound =
                (*success).then(|| "SFX_GET_EGG".to_string());
            set_shell_action_status(
                runtime_shell,
                if *success { "RECEIVED EGG" } else { "DAY-CARE EGG" },
            );
            mark_runtime_snapshot_dirty(runtime_shell);
            Ok(true)
        }
        SpecialRoutineEffect::DayCareMon {
            caretaker,
            occupied,
            ..
        } => {
            if !occupied {
                return Ok(false);
            }
            let snapshot = runtime_shell.shell.snapshot()?;
            let (resident, partner, owner) = match caretaker.as_str() {
                "man" => (&snapshot.day_care.man, &snapshot.day_care.lady, "MAN"),
                "lady" => (&snapshot.day_care.lady, &snapshot.day_care.man, "LADY"),
                exact => anyhow::bail!("unknown day-care caretaker {exact}"),
            };
            let pokemon = resident
                .pokemon
                .as_ref()
                .with_context(|| format!("occupied day-care {caretaker} has no Pokemon"))?;
            let nickname = pokemon.nickname.trim();
            let nickname = if nickname.is_empty() {
                canonical_species_display_name(&pokemon.species.id)
            } else {
                nickname.to_string()
            };
            runtime_shell.special_boundary_queue.clear();
            runtime_shell.special_boundary = Some(SpecialBoundaryDisplay {
                label: "DayCareMon".to_string(),
                details: vec![format!(
                    "It's {nickname}\nthat was left with\nthe DAY-CARE {owner}."
                )],
            });
            if let Some(partner) = partner.pokemon.as_ref() {
                let partner_name = if partner.nickname.trim().is_empty() {
                    canonical_species_display_name(&partner.species.id)
                } else {
                    partner.nickname.trim().to_string()
                };
                let compatibility = match snapshot.day_care.compatibility_score {
                    255 => "It's brimming with\nenergy.".to_string(),
                    0 => format!("It has no interest\nin {partner_name}."),
                    230..=254 => format!("It appears to care\nfor {partner_name}."),
                    70..=229 => format!("It's friendly with\n{partner_name}."),
                    _ => format!("It shows interest\nin {partner_name}."),
                };
                runtime_shell
                    .special_boundary_queue
                    .push_back(SpecialBoundaryDisplay {
                        label: "DayCareCompatibility".to_string(),
                        details: vec![compatibility],
                    });
            }
            runtime_shell.pending_special_cry = Some(pokemon.species.id.clone());
            set_shell_action_status(runtime_shell, format!("DAY-CARE {nickname}"));
            mark_runtime_snapshot_dirty(runtime_shell);
            Ok(true)
        }
        SpecialRoutineEffect::ProfOaksPcBoot {
            seen_count,
            caught_count,
            rating_label,
        } => {
            open_visible_prof_oak_rating(
                runtime_shell,
                *seen_count,
                *caught_count,
                rating_label,
            )?;
            Ok(true)
        }
        SpecialRoutineEffect::CheckForLuckyNumberWinners {
            tier,
            text_label,
            ..
        } => {
            if *tier == 0 {
                return Ok(false);
            }
            let label = text_label
                .as_deref()
                .context("winning lucky-number result has no text label")?;
            let snapshot = runtime_shell.shell.snapshot()?;
            let text = snapshot
                .presentation
                .asm_text
                .get(label)
                .with_context(|| format!("lucky-number result text {label} is missing"))?;
            let text = text.clone();
            runtime_shell.special_boundary = Some(SpecialBoundaryDisplay {
                label: label.to_string(),
                details: text.lines().map(str::to_string).collect(),
            });
            set_shell_action_status(runtime_shell, format!("LUCKY NUMBER TIER {tier}"));
            mark_runtime_snapshot_dirty(runtime_shell);
            Ok(true)
        }
        SpecialRoutineEffect::MagikarpHouseSign { formatted, .. } => {
            let snapshot = runtime_shell.shell.snapshot()?;
            runtime_shell.special_boundary = Some(SpecialBoundaryDisplay {
                label: "MagikarpHouseSign".to_string(),
                details: vec![
                    "CURRENT RECORD".to_string(),
                    String::new(),
                    format!(
                        "{formatted} caught by\n{}",
                        snapshot.magikarp_record.best_owner_name
                    ),
                ],
            });
            set_shell_action_status(runtime_shell, "CURRENT RECORD");
            mark_runtime_snapshot_dirty(runtime_shell);
            Ok(true)
        }
        SpecialRoutineEffect::UnownPuzzle { .. } => {
            update_visible_unown_puzzle_from_effect(runtime_shell, effect)?;
            Ok(true)
        }
        SpecialRoutineEffect::SlotMachine { bet, payout, coins, windows, .. } => {
            runtime_shell.visible_slot_machine = Some(VisibleSlotMachine {
                bet: *bet,
                coins: *coins,
                payout: *payout,
                windows: windows.clone(),
                message: if *payout > 0 { format!("WIN {payout}") } else { "DARN".to_string() },
            });
            mark_runtime_snapshot_dirty(runtime_shell);
            Ok(true)
        }
        SpecialRoutineEffect::CardFlip { card_index, card_name, card_level, payout, deck, revealed, coins, .. } => {
            runtime_shell.visible_card_flip = Some(VisibleCardFlip {
                phase: VisibleCardFlipPhase::PlayAgain,
                yes_no_index: 0,
                which_card: card_index % 2,
                bet_x: 2,
                bet_y: 2,
                round: card_index / 2,
                face_card: Some((card_name.clone(), *card_level)),
                coins: *coins,
                payout: *payout,
                deck: deck.clone(),
                revealed: revealed.clone(),
                message: format!("{}  WIN {}", canonical_species_display_name(card_name), payout),
            });
            mark_runtime_snapshot_dirty(runtime_shell);
            Ok(true)
        }
        SpecialRoutineEffect::SetDayOfWeek { .. }
        | SpecialRoutineEffect::NameRival { .. }
        | SpecialRoutineEffect::MoveTutor { .. }
        | SpecialRoutineEffect::BuenasPassword { .. }
        | SpecialRoutineEffect::BuenaPrize { .. }
        | SpecialRoutineEffect::UnownPrinter { .. }
        | SpecialRoutineEffect::UnusedMemoryGame { .. }
        | SpecialRoutineEffect::LinkAction { .. }
        | SpecialRoutineEffect::LinkResult { .. }
        | SpecialRoutineEffect::LinkRoom { .. }
        | SpecialRoutineEffect::TimeCapsuleCompatibility { .. }
        | SpecialRoutineEffect::AskMobileOrCable { .. }
        | SpecialRoutineEffect::CableClubCheckWhichChris { .. }
        | SpecialRoutineEffect::DisplayLinkRecord { .. }
        | SpecialRoutineEffect::BattleTowerAction { .. }
        | SpecialRoutineEffect::CheckForBattleTowerRules { .. }
        | SpecialRoutineEffect::BattleTowerChallengeExplanationCancel
        | SpecialRoutineEffect::BattleTowerRoomMenu { .. }
        | SpecialRoutineEffect::BattleTowerBattle { .. }
        | SpecialRoutineEffect::BattleTowerLeaderboard { .. }
        | SpecialRoutineEffect::BattleTowerMobileError
        | SpecialRoutineEffect::LoadOpponentTrainerAndPokemonWithOtSprite { .. }
        | SpecialRoutineEffect::AskRememberPassword { .. }
        | SpecialRoutineEffect::MobileHandshake { .. }
        | SpecialRoutineEffect::MobileSessionEnded
        | SpecialRoutineEffect::BattleTowerMobileFlag { .. }
        | SpecialRoutineEffect::MobileSelectThreeMons { .. }
        | SpecialRoutineEffect::UnusedFindItemInPcOrBag { .. }
        | SpecialRoutineEffect::Function11ba38 { .. }
        | SpecialRoutineEffect::GameCornerGameUnavailable { .. } => {
            open_visible_special_boundary(runtime_shell, effect);
            Ok(true)
        }
        // These routines only write state or wScriptVar. Their callers branch
        // or continue immediately; none owns a source UI surface.
        SpecialRoutineEffect::GameboyCheck { .. }
        | SpecialRoutineEffect::FirstPokemonHappiness { .. }
        | SpecialRoutineEffect::CheckFirstMonIsEgg { .. }
        | SpecialRoutineEffect::FindPartyMonThatSpecies { .. }
        | SpecialRoutineEffect::FindPartyMonThatSpeciesYourTrainerId { .. }
        | SpecialRoutineEffect::FindPartyMonAboveLevel { .. }
        | SpecialRoutineEffect::FindPartyMonAtLeastThatHappy { .. }
        | SpecialRoutineEffect::MonCheck { .. }
        | SpecialRoutineEffect::BeastsCheck { .. }
        | SpecialRoutineEffect::CheckCaughtCelebi { .. }
        | SpecialRoutineEffect::CheckPokerus { .. }
        | SpecialRoutineEffect::UnusedSetSeenMon { .. }
        | SpecialRoutineEffect::InitRoamMons { .. }
        | SpecialRoutineEffect::CheckMagikarpLength { .. }
        | SpecialRoutineEffect::PhotoStudio { .. }
        | SpecialRoutineEffect::PokeSeer { .. }
        | SpecialRoutineEffect::NameRater { .. }
        | SpecialRoutineEffect::HappinessService { .. }
        | SpecialRoutineEffect::MoveDeletion { .. }
        | SpecialRoutineEffect::CelebiShrineEvent { .. }
        | SpecialRoutineEffect::TrainerHouse { .. }
        | SpecialRoutineEffect::HoOhChamber { .. }
        | SpecialRoutineEffect::UnownChamber { .. }
        | SpecialRoutineEffect::Reset { .. }
        | SpecialRoutineEffect::MobileAdapterStatus { .. }
        | SpecialRoutineEffect::GsHealings { .. }
        | SpecialRoutineEffect::TrainerRankingsHealings { .. }
        | SpecialRoutineEffect::RandomUnseenWildMon { .. }
        | SpecialRoutineEffect::RandomPhoneWildMon { .. }
        | SpecialRoutineEffect::RandomPhoneMon { .. }
        | SpecialRoutineEffect::ActivateFishingSwarm { .. }
        | SpecialRoutineEffect::SetPlayerPalette { .. }
        | SpecialRoutineEffect::SnorlaxAwake { .. }
        | SpecialRoutineEffect::InitialSetDstFlag
        | SpecialRoutineEffect::InitialClearDstFlag
        | SpecialRoutineEffect::UpdateTime { .. }
        | SpecialRoutineEffect::UnusedCheckUnusedTwoDayTimer { .. }
        | SpecialRoutineEffect::SampleKenjiBreakCountdown { .. }
        | SpecialRoutineEffect::CheckLuckyNumberShowFlag { .. }
        | SpecialRoutineEffect::ResetLuckyNumberShowFlag { .. }
        | SpecialRoutineEffect::PrintTodaysLuckyNumber { .. }
        | SpecialRoutineEffect::GiveShuckle { .. }
        | SpecialRoutineEffect::ReturnShuckie { .. }
        | SpecialRoutineEffect::GiveDratini { .. }
        | SpecialRoutineEffect::BillsGrandfather { .. }
        | SpecialRoutineEffect::SelectApricornForKurt { .. }
        | SpecialRoutineEffect::CheckMysteryGift { .. }
        | SpecialRoutineEffect::GetMysteryGiftItem { .. }
        | SpecialRoutineEffect::UnlockMysteryGift { .. }
        | SpecialRoutineEffect::GiveOddEgg { .. }
        | SpecialRoutineEffect::GiveParkBalls { .. }
        | SpecialRoutineEffect::BugContestTimer { .. }
        | SpecialRoutineEffect::SelectRandomBugContestContestants { .. }
        | SpecialRoutineEffect::ContestDropOffMons { .. }
        | SpecialRoutineEffect::ContestReturnMons { .. }
        | SpecialRoutineEffect::CheckPartyFullAfterContest { .. } => Ok(false),
        _ => Ok(false),
    }
}

fn activate_visible_special_boundary_if_needed(
    runtime_shell: &mut BevyRuntimeShell,
    effect: &SpecialRoutineEffect,
) -> Result<()> {
    activate_visible_special_routine_boundary(runtime_shell, effect)?;
    Ok(())
}

fn open_visible_special_boundary(
    runtime_shell: &mut BevyRuntimeShell,
    effect: &SpecialRoutineEffect,
) {
    let boundary = special_boundary_display(effect);
    runtime_shell.last_audio_events.push(format!(
        "opened special boundary {} details={} effect={effect:?}",
        boundary.label,
        boundary.details.join(" | ")
    ));
    runtime_shell.special_boundary = Some(boundary);
    trim_event_log(&mut runtime_shell.last_audio_events);
}

fn open_visible_script_runtime_boundary_if_needed(
    runtime_shell: &mut BevyRuntimeShell,
    command: &crate::core::systems::script_runtime::ScriptRuntimeCommand,
) -> Result<bool> {
    match command.command.as_str() {
        "catchtutorial" => {
            start_visible_catch_tutorial(
                runtime_shell,
                &command.source_script,
                command.command_index,
            )?;
            Ok(true)
        }
        "trade" => {
            let trade_id = command
                .args
                .first()
                .with_context(|| "trade runtime command has no trade id")?
                .clone();
            let snapshot = runtime_shell.shell.snapshot()?;
            let rule = snapshot
                .special
                .npc_trades
                .get(&trade_id)
                .with_context(|| format!("NPC trade {trade_id} is missing from the runtime snapshot"))?;
            runtime_shell.pc_notice = Some(visible_completed_npc_trade_text(rule));
            set_shell_action_status(runtime_shell, "TRADE COMPLETE");
            Ok(true)
        }
        // Decoration selection is resolved by the compiled collision/event
        // transaction before script dispatch. Do not re-route a descriptor
        // through a Rust label switch: the standard collision script and the
        // exported decoration data are the authoritative ASM control flow.
        "describedecoration" => Ok(false),
        _ => Ok(false),
    }
}

fn start_visible_catch_tutorial(
    runtime_shell: &mut BevyRuntimeShell,
    source_script: &str,
    command_index: usize,
) -> Result<()> {
    let map_name = runtime_shell.shell.snapshot()?.overworld.map_name;
    let start = runtime_shell.shell.start_scripted_wild_battle(
        &map_name,
        source_script,
        command_index,
    )?;
    prepare_visible_battle_entry(runtime_shell)?;
    runtime_shell.last_audio_events.push(format!(
        "catch tutorial start source={source_script} start={start:?}"
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(())
}

fn special_boundary_display(effect: &SpecialRoutineEffect) -> SpecialBoundaryDisplay {
    match effect {
        SpecialRoutineEffect::FadeOutMusic {
            audio_id,
            fade_frames,
        } => SpecialBoundaryDisplay {
            label: "FadeOutMusic".to_string(),
            details: vec![format!("audio={audio_id}"), format!("frames={fade_frames}")],
        },
        SpecialRoutineEffect::WaitSfx => SpecialBoundaryDisplay {
            label: "WaitSfx".to_string(),
            details: vec!["script is waiting for sound effect completion".to_string()],
        },
        SpecialRoutineEffect::PlayMapMusic => SpecialBoundaryDisplay {
            label: "PlayMapMusic".to_string(),
            details: Vec::new(),
        },
        SpecialRoutineEffect::RestartMapMusic => SpecialBoundaryDisplay {
            label: "RestartMapMusic".to_string(),
            details: Vec::new(),
        },
        SpecialRoutineEffect::PlayCurMonCry { species, audio_id } => SpecialBoundaryDisplay {
            label: "PlayCurMonCry".to_string(),
            details: vec![format!("species={species}"), format!("audio={audio_id}")],
        },
        SpecialRoutineEffect::PlaySlowCry { species, audio_id } => SpecialBoundaryDisplay {
            label: "PlaySlowCry".to_string(),
            details: vec![format!("species={species}"), format!("audio={audio_id}")],
        },
        SpecialRoutineEffect::GraphicsCommand { kind } => SpecialBoundaryDisplay {
            label: "GraphicsCommand".to_string(),
            details: vec![format!("kind={kind:?}")],
        },
        SpecialRoutineEffect::ScreenFade {
            color,
            direction,
            frames,
        } => SpecialBoundaryDisplay {
            label: "ScreenFade".to_string(),
            details: vec![
                format!("color={color:?}"),
                format!("direction={direction:?}"),
                format!("frames={frames}"),
            ],
        },
        SpecialRoutineEffect::RuntimeVisualCommand { kind } => SpecialBoundaryDisplay {
            label: "RuntimeVisualCommand".to_string(),
            details: vec![format!("kind={kind:?}")],
        },
        SpecialRoutineEffect::GameboyCheck { token } => SpecialBoundaryDisplay {
            label: "GameboyCheck".to_string(),
            details: vec![format!("token={token}")],
        },
        SpecialRoutineEffect::MobileAdapterStatus { value } => SpecialBoundaryDisplay {
            label: "MobileAdapterStatus".to_string(),
            details: vec![format!("value={value}")],
        },
        SpecialRoutineEffect::FirstPokemonHappiness {
            party_slot,
            species,
            nickname,
            happiness,
        } => SpecialBoundaryDisplay {
            label: "FirstPokemonHappiness".to_string(),
            details: vec![
                format!("party_slot={party_slot}"),
                format!("species={species}"),
                format!("nickname={nickname}"),
                format!("happiness={happiness}"),
            ],
        },
        SpecialRoutineEffect::CheckFirstMonIsEgg {
            species,
            nickname,
            is_egg,
        } => SpecialBoundaryDisplay {
            label: "CheckFirstMonIsEgg".to_string(),
            details: vec![
                format!("species={species}"),
                format!("nickname={nickname}"),
                format!("is_egg={is_egg}"),
            ],
        },
        SpecialRoutineEffect::FindPartyMonThatSpecies { species, found } => {
            SpecialBoundaryDisplay {
                label: "FindPartyMonThatSpecies".to_string(),
                details: vec![format!("species={species}"), format!("found={found}")],
            }
        }
        SpecialRoutineEffect::FindPartyMonThatSpeciesYourTrainerId {
            species,
            player_name,
            player_id,
            found,
        } => SpecialBoundaryDisplay {
            label: "FindPartyMonThatSpeciesYourTrainerId".to_string(),
            details: vec![
                format!("species={species}"),
                format!("trainer={player_name}#{player_id}"),
                format!("found={found}"),
            ],
        },
        SpecialRoutineEffect::FindPartyMonAboveLevel {
            level,
            found,
            species,
        } => SpecialBoundaryDisplay {
            label: "FindPartyMonAboveLevel".to_string(),
            details: vec![
                format!("level={level}"),
                format!("found={found}"),
                format!("species={}", species.as_deref().unwrap_or("-")),
            ],
        },
        SpecialRoutineEffect::FindPartyMonAtLeastThatHappy {
            happiness,
            found,
            species,
        } => SpecialBoundaryDisplay {
            label: "FindPartyMonAtLeastThatHappy".to_string(),
            details: vec![
                format!("happiness={happiness}"),
                format!("found={found}"),
                format!("species={}", species.as_deref().unwrap_or("-")),
            ],
        },
        SpecialRoutineEffect::MonCheck {
            species,
            player_name,
            player_id,
            owned,
        } => SpecialBoundaryDisplay {
            label: "MonCheck".to_string(),
            details: vec![
                format!("species={species}"),
                format!("trainer={player_name}#{player_id}"),
                format!("owned={owned}"),
            ],
        },
        SpecialRoutineEffect::BeastsCheck {
            player_name,
            player_id,
            missing_species,
            owned_all,
        } => SpecialBoundaryDisplay {
            label: "BeastsCheck".to_string(),
            details: vec![
                format!("trainer={player_name}#{player_id}"),
                format!("missing={}", missing_species.as_deref().unwrap_or("-")),
                format!("owned_all={owned_all}"),
            ],
        },
        SpecialRoutineEffect::GameCornerPrizeMonCheckDex {
            species,
            species_int_id,
            already_caught,
            recorded_caught,
        } => SpecialBoundaryDisplay {
            label: "GameCornerPrizeMonCheckDex".to_string(),
            details: vec![
                format!("species={species}"),
                format!("species_int_id={species_int_id}"),
                format!("already_caught={already_caught}"),
                format!("recorded_caught={recorded_caught}"),
            ],
        },
        SpecialRoutineEffect::UnusedSetSeenMon {
            species,
            species_int_id,
            newly_seen,
        } => SpecialBoundaryDisplay {
            label: "UnusedSetSeenMon".to_string(),
            details: vec![
                format!("species={species}"),
                format!("species_int_id={species_int_id}"),
                format!("newly_seen={newly_seen}"),
            ],
        },
        SpecialRoutineEffect::RandomUnseenWildMon {
            contact_id,
            map_name,
            species,
            already_seen,
            script_value,
            random_state_after,
        } => SpecialBoundaryDisplay {
            label: "RandomUnseenWildMon".to_string(),
            details: vec![
                format!("contact={contact_id}"),
                format!("map={map_name}"),
                format!("species={}", species.as_deref().unwrap_or("-")),
                format!("already_seen={already_seen}"),
                format!("value={script_value}"),
                format!("random={random_state_after:?}"),
            ],
        },
        SpecialRoutineEffect::RandomPhoneWildMon {
            contact_id,
            map_name,
            time_of_day,
            species,
            random_state_after,
        } => SpecialBoundaryDisplay {
            label: "RandomPhoneWildMon".to_string(),
            details: vec![
                format!("contact={contact_id}"),
                format!("map={map_name}"),
                format!("time={time_of_day:?}"),
                format!("species={species}"),
                format!("random={random_state_after:?}"),
            ],
        },
        SpecialRoutineEffect::RandomPhoneMon {
            contact_id,
            trainer_id,
            species,
            party_index,
            random_state_after,
        } => SpecialBoundaryDisplay {
            label: "RandomPhoneMon".to_string(),
            details: vec![
                format!("contact={contact_id}"),
                format!("trainer={trainer_id}"),
                format!("species={species}"),
                format!("party_index={party_index}"),
                format!("random={random_state_after:?}"),
            ],
        },
        SpecialRoutineEffect::ActivateFishingSwarm { value } => SpecialBoundaryDisplay {
            label: "ActivateFishingSwarm".to_string(),
            details: vec![format!("value={value}")],
        },
        SpecialRoutineEffect::CheckCaughtCelebi { caught } => SpecialBoundaryDisplay {
            label: "CheckCaughtCelebi".to_string(),
            details: vec![format!("caught={caught}")],
        },
        SpecialRoutineEffect::SetPlayerPalette {
            raw_value,
            palette_id,
            changed,
        } => SpecialBoundaryDisplay {
            label: "SetPlayerPalette".to_string(),
            details: vec![
                format!("raw_value={raw_value}"),
                format!("palette={palette_id}"),
                format!("changed={changed}"),
            ],
        },
        SpecialRoutineEffect::SnorlaxAwake { music, tile, awake } => SpecialBoundaryDisplay {
            label: "SnorlaxAwake".to_string(),
            details: vec![
                format!("music={}", music.as_deref().unwrap_or("-")),
                format!("tile={}", optional_tile_label(*tile)),
                format!("awake={awake}"),
            ],
        },
        SpecialRoutineEffect::SetDayOfWeek { day } => SpecialBoundaryDisplay {
            label: "SetDayOfWeek".to_string(),
            details: vec![format!("day={day}")],
        },
        SpecialRoutineEffect::InitialSetDstFlag => SpecialBoundaryDisplay {
            label: "InitialSetDstFlag".to_string(),
            details: Vec::new(),
        },
        SpecialRoutineEffect::InitialClearDstFlag => SpecialBoundaryDisplay {
            label: "InitialClearDstFlag".to_string(),
            details: Vec::new(),
        },
        SpecialRoutineEffect::UpdateTime {
            hour,
            minute,
            second,
            day_of_week,
            time_of_day,
        } => SpecialBoundaryDisplay {
            label: "UpdateTime".to_string(),
            details: vec![
                format!("time={hour:02}:{minute:02}:{second:02}"),
                format!("day={day_of_week}"),
                format!("time_of_day={time_of_day:?}"),
            ],
        },
        SpecialRoutineEffect::UnusedCheckUnusedTwoDayTimer {
            start_day,
            current_day,
            elapsed_days,
            remaining_days,
        } => SpecialBoundaryDisplay {
            label: "UnusedCheckUnusedTwoDayTimer".to_string(),
            details: vec![
                format!("start_day={start_day}"),
                format!("current_day={current_day}"),
                format!("elapsed={elapsed_days}"),
                format!("remaining={remaining_days}"),
            ],
        },
        SpecialRoutineEffect::SampleKenjiBreakCountdown {
            value,
            random_state_after,
        } => SpecialBoundaryDisplay {
            label: "SampleKenjiBreakCountdown".to_string(),
            details: vec![
                format!("value={value}"),
                format!("random={random_state_after:?}"),
            ],
        },
        SpecialRoutineEffect::CheckLuckyNumberShowFlag { flag } => SpecialBoundaryDisplay {
            label: "CheckLuckyNumberShowFlag".to_string(),
            details: vec![format!("flag={flag}")],
        },
        SpecialRoutineEffect::ResetLuckyNumberShowFlag {
            lucky_number,
            lucky_number_day,
            random_state_after,
        } => SpecialBoundaryDisplay {
            label: "ResetLuckyNumberShowFlag".to_string(),
            details: vec![
                format!("number={lucky_number:05}"),
                format!("day={lucky_number_day}"),
                format!("random={random_state_after:?}"),
            ],
        },
        SpecialRoutineEffect::GsHealings { healings } => SpecialBoundaryDisplay {
            label: "GsHealings".to_string(),
            details: vec![format!("healings={healings}")],
        },
        SpecialRoutineEffect::TrainerRankingsHealings { healings } => SpecialBoundaryDisplay {
            label: "TrainerRankingsHealings".to_string(),
            details: vec![format!("healings={healings}")],
        },
        SpecialRoutineEffect::Reset { value } => SpecialBoundaryDisplay {
            label: "Reset".to_string(),
            details: vec![format!("value={value}")],
        },
        SpecialRoutineEffect::HoOhChamber {
            has_ho_oh,
            suicune_unleashed,
            raikou_unleashed,
            entei_unleashed,
            open,
        } => SpecialBoundaryDisplay {
            label: "HoOhChamber".to_string(),
            details: vec![
                format!("has_ho_oh={has_ho_oh}"),
                format!("suicune={suicune_unleashed}"),
                format!("raikou={raikou_unleashed}"),
                format!("entei={entei_unleashed}"),
                format!("open={open}"),
            ],
        },
        SpecialRoutineEffect::UnownChamber { chamber, open } => SpecialBoundaryDisplay {
            label: format!("{chamber}Chamber"),
            details: vec![format!("open={open}")],
        },
        SpecialRoutineEffect::CheckPokerus {
            found,
            newly_discovered,
        } => SpecialBoundaryDisplay {
            label: "CheckPokerus".to_string(),
            details: vec![
                format!("found={found}"),
                format!("newly_discovered={newly_discovered}"),
            ],
        },
        SpecialRoutineEffect::InitRoamMons { roamers } => SpecialBoundaryDisplay {
            label: "InitRoamMons".to_string(),
            details: vec![format!("roamers={}", roamers.len())],
        },
        SpecialRoutineEffect::CelebiShrineEvent { battle_type } => SpecialBoundaryDisplay {
            label: "CelebiShrineEvent".to_string(),
            details: vec![format!("battle_type={battle_type}")],
        },
        SpecialRoutineEffect::ProfOaksPcBoot {
            seen_count,
            caught_count,
            rating_label,
        } => SpecialBoundaryDisplay {
            label: "ProfOaksPcBoot".to_string(),
            details: vec![
                format!("seen={seen_count}"),
                format!("caught={caught_count}"),
                format!("rating={rating_label}"),
            ],
        },
        SpecialRoutineEffect::CheckForLuckyNumberWinners {
            lucky_number,
            tier,
            source,
            species,
            text_label,
        } => SpecialBoundaryDisplay {
            label: "CheckForLuckyNumberWinners".to_string(),
            details: vec![
                format!("number={lucky_number:05}"),
                format!("tier={tier}"),
                format!("source={source:?}"),
                format!("species={}", species.as_deref().unwrap_or("-")),
                format!("text={}", text_label.as_deref().unwrap_or("-")),
            ],
        },
        SpecialRoutineEffect::PlaceMoneyTopRight { money, formatted } => SpecialBoundaryDisplay {
            label: "PlaceMoneyTopRight".to_string(),
            details: vec![format!("money={money}"), format!("formatted={formatted}")],
        },
        SpecialRoutineEffect::DisplayMoneyAndCoinBalance {
            money,
            coins,
            formatted_money,
            formatted_coins,
        } => SpecialBoundaryDisplay {
            label: "DisplayMoneyAndCoinBalance".to_string(),
            details: vec![
                format!("money={money}"),
                format!("coins={coins}"),
                format!("formatted_money={formatted_money}"),
                format!("formatted_coins={formatted_coins}"),
            ],
        },
        SpecialRoutineEffect::DisplayCoinCaseBalance {
            coins,
            formatted_coins,
        } => SpecialBoundaryDisplay {
            label: "DisplayCoinCaseBalance".to_string(),
            details: vec![
                format!("coins={coins}"),
                format!("formatted_coins={formatted_coins}"),
            ],
        },
        SpecialRoutineEffect::PrintTodaysLuckyNumber {
            lucky_number,
            formatted,
        } => SpecialBoundaryDisplay {
            label: "PrintTodaysLuckyNumber".to_string(),
            details: vec![
                format!("number={lucky_number:05}"),
                format!("formatted={formatted}"),
            ],
        },
        SpecialRoutineEffect::NameRival { rival_name } => SpecialBoundaryDisplay {
            label: "NameRival".to_string(),
            details: vec![format!("rival={rival_name}")],
        },
        SpecialRoutineEffect::MoveDeletion {
            party_slot,
            species,
            deleted_move,
            remaining_moves,
        } => SpecialBoundaryDisplay {
            label: "MoveDeletion".to_string(),
            details: vec![
                format!("party_slot={party_slot}"),
                format!("species={species}"),
                format!("deleted={deleted_move}"),
                format!("remaining_moves={remaining_moves}"),
            ],
        },
        SpecialRoutineEffect::HappinessService {
            party_slot,
            species,
            old_happiness,
            new_happiness,
            script_value,
            change_code,
            rng_seed_after,
        } => SpecialBoundaryDisplay {
            label: "HappinessService".to_string(),
            details: vec![
                format!("party_slot={party_slot}"),
                format!("species={species}"),
                format!("happiness={old_happiness}->{new_happiness}"),
                format!("value={script_value} change={change_code}"),
                format!("rng={rng_seed_after}"),
            ],
        },
        SpecialRoutineEffect::NameRater {
            party_slot,
            species,
            old_nickname,
            new_nickname,
        } => SpecialBoundaryDisplay {
            label: "NameRater".to_string(),
            details: vec![
                format!("party_slot={party_slot}"),
                format!("species={species}"),
                format!("nickname={old_nickname}->{new_nickname}"),
            ],
        },
        SpecialRoutineEffect::PokeSeer {
            party_slot,
            species,
            nickname,
            original_trainer_name,
            original_trainer_id,
        } => SpecialBoundaryDisplay {
            label: "PokeSeer".to_string(),
            details: vec![
                format!("party_slot={party_slot}"),
                format!("species={species}"),
                format!("nickname={nickname}"),
                format!("ot={original_trainer_name}#{original_trainer_id}"),
            ],
        },
        SpecialRoutineEffect::MoveTutor {
            party_slot,
            species,
            move_name,
            learned,
        } => SpecialBoundaryDisplay {
            label: "MoveTutor".to_string(),
            details: vec![
                format!("party_slot={party_slot}"),
                format!("species={species}"),
                format!("move={move_name}"),
                format!("learned={learned}"),
            ],
        },
        SpecialRoutineEffect::GiveShuckle {
            stored,
            random_state_after,
        } => SpecialBoundaryDisplay {
            label: "GiveShuckle".to_string(),
            details: vec![
                format!("stored={stored}"),
                format!("rng={random_state_after:?}"),
            ],
        },
        SpecialRoutineEffect::ReturnShuckie { party_slot, result } => SpecialBoundaryDisplay {
            label: "ReturnShuckie".to_string(),
            details: vec![
                format!("party_slot={}", optional_usize_label(*party_slot)),
                format!("result={result}"),
            ],
        },
        SpecialRoutineEffect::GiveDratini {
            party_slot,
            mode,
            move_names,
            learned,
        } => SpecialBoundaryDisplay {
            label: "GiveDratini".to_string(),
            details: vec![
                format!("party_slot={}", optional_usize_label(*party_slot)),
                format!("mode={mode}"),
                format!("moves={}", move_names.join(",")),
                format!("learned={learned}"),
            ],
        },
        SpecialRoutineEffect::BillsGrandfather {
            party_slot,
            species,
        } => SpecialBoundaryDisplay {
            label: "BillsGrandfather".to_string(),
            details: vec![
                format!("party_slot={}", optional_usize_label(*party_slot)),
                format!("species={}", species.as_deref().unwrap_or("-")),
            ],
        },
        SpecialRoutineEffect::SelectApricornForKurt { apricorn, quantity } => {
            SpecialBoundaryDisplay {
                label: "SelectApricornForKurt".to_string(),
                details: vec![
                    format!("apricorn={}", apricorn.as_deref().unwrap_or("-")),
                    format!("quantity={quantity}"),
                ],
            }
        }
        SpecialRoutineEffect::DayCareInteraction {
            caretaker,
            action,
            success,
            pokemon,
        } => SpecialBoundaryDisplay {
            label: "DayCareInteraction".to_string(),
            details: vec![
                format!("caretaker={caretaker}"),
                format!("action={action}"),
                format!("success={success}"),
                format!("pokemon={}", pokemon.as_deref().unwrap_or("-")),
            ],
        },
        SpecialRoutineEffect::DayCareMon {
            caretaker,
            occupied,
            pokemon,
            level,
        } => SpecialBoundaryDisplay {
            label: "DayCareMon".to_string(),
            details: vec![
                format!("caretaker={caretaker}"),
                format!("occupied={occupied}"),
                format!("pokemon={}", pokemon.as_deref().unwrap_or("-")),
                format!("level={}", optional_u8_label(*level)),
            ],
        },
        SpecialRoutineEffect::GiveParkBalls { balls } => SpecialBoundaryDisplay {
            label: "GiveParkBalls".to_string(),
            details: vec![format!("balls={balls}")],
        },
        SpecialRoutineEffect::BugContestTimer {
            active,
            minutes_remaining,
            seconds_remaining,
        } => SpecialBoundaryDisplay {
            label: "BugContestTimer".to_string(),
            details: vec![
                format!("active={active}"),
                format!("minutes={minutes_remaining}"),
                format!("seconds={seconds_remaining}"),
            ],
        },
        SpecialRoutineEffect::SelectRandomBugContestContestants {
            flags,
            random_state_after,
        } => SpecialBoundaryDisplay {
            label: "SelectRandomBugContestContestants".to_string(),
            details: vec![
                format!("flags={}", flags.join(",")),
                format!("random_state={random_state_after:?}"),
            ],
        },
        SpecialRoutineEffect::ContestDropOffMons {
            result,
            backup_count,
            second_party_species,
        } => SpecialBoundaryDisplay {
            label: "ContestDropOffMons".to_string(),
            details: vec![
                format!("result={result}"),
                format!("backup_count={backup_count}"),
                format!("second={}", second_party_species.as_deref().unwrap_or("-")),
            ],
        },
        SpecialRoutineEffect::ContestReturnMons { restored_count } => SpecialBoundaryDisplay {
            label: "ContestReturnMons".to_string(),
            details: vec![format!("restored_count={restored_count}")],
        },
        SpecialRoutineEffect::CheckPartyFullAfterContest { result, species } => {
            SpecialBoundaryDisplay {
                label: "CheckPartyFullAfterContest".to_string(),
                details: vec![
                    format!("result={result}"),
                    format!("species={}", species.as_deref().unwrap_or("-")),
                ],
            }
        }
        SpecialRoutineEffect::BugContestJudging {
            rank,
            placements,
            random_state_after,
        } => SpecialBoundaryDisplay {
            label: "BugContestJudging".to_string(),
            details: placements
                .iter()
                .rev()
                .map(|placement| {
                    format!(
                        "{} PLACE: {} caught {}! {} points!",
                        match placement.place {
                            1 => "FIRST",
                            2 => "SECOND",
                            _ => "THIRD",
                        },
                        placement.trainer_name,
                        canonical_species_display_name(&placement.species),
                        placement.score
                    )
                })
                .chain(std::iter::once(format!("PLAYER RANK: {rank}")))
                .chain(std::iter::once(format!(
                    "random_state={random_state_after:?}"
                )))
                .collect(),
        },
        SpecialRoutineEffect::CheckMysteryGift { has_pending_item } => SpecialBoundaryDisplay {
            label: "CheckMysteryGift".to_string(),
            details: vec![format!("has_pending_item={has_pending_item}")],
        },
        SpecialRoutineEffect::GetMysteryGiftItem { item_id, received } => SpecialBoundaryDisplay {
            label: "GetMysteryGiftItem".to_string(),
            details: vec![
                format!("item={}", item_id.as_deref().unwrap_or("-")),
                format!("received={received}"),
            ],
        },
        SpecialRoutineEffect::UnlockMysteryGift { newly_unlocked } => SpecialBoundaryDisplay {
            label: "UnlockMysteryGift".to_string(),
            details: vec![format!("newly_unlocked={newly_unlocked}")],
        },
        SpecialRoutineEffect::BuenasPassword {
            category,
            category_type,
            correct,
            guess,
            matched,
            random_state_after,
        } => SpecialBoundaryDisplay {
            label: "BuenasPassword".to_string(),
            details: vec![
                format!("category={category}"),
                format!("type={category_type}"),
                format!("correct={correct}"),
                format!("guess={}", guess.as_deref().unwrap_or("-")),
                format!("matched={matched}"),
                format!("rng={random_state_after:?}"),
            ],
        },
        SpecialRoutineEffect::BuenaPrize {
            item_id,
            quantity,
            points_spent,
            balance,
        } => SpecialBoundaryDisplay {
            label: "BuenaPrize".to_string(),
            details: vec![
                format!("item={item_id}"),
                format!("quantity={quantity}"),
                format!("points_spent={points_spent}"),
                format!("balance={balance}"),
            ],
        },
        SpecialRoutineEffect::GiveOddEgg {
            table_index,
            species,
            party_slot,
            shiny,
            random_state_after,
        } => SpecialBoundaryDisplay {
            label: "GiveOddEgg".to_string(),
            details: vec![
                format!("table_index={table_index}"),
                format!("species={species}"),
                format!("party_slot={party_slot}"),
                format!("shiny={shiny}"),
                format!("rng={random_state_after:?}"),
            ],
        },
        SpecialRoutineEffect::UnownPrinter { unlocked } => SpecialBoundaryDisplay {
            label: "UnownPrinter".to_string(),
            details: vec![format!("unlocked={unlocked}")],
        },
        SpecialRoutineEffect::UnownPuzzle {
            puzzle_id,
            solved,
            moves,
            holding_piece,
            random_state_after,
            ..
        } => SpecialBoundaryDisplay {
            label: "UnownPuzzle".to_string(),
            details: vec![
                format!("puzzle={puzzle_id}"),
                format!("solved={solved}"),
                format!("moves={moves}"),
                format!("holding={holding_piece:?}"),
                format!("random_state={random_state_after:?}"),
            ],
        },
        SpecialRoutineEffect::BankOfMom { money, moms_money, .. } => SpecialBoundaryDisplay {
            label: "BankOfMom".to_string(),
            details: vec![format!("money={money}"), format!("moms_money={moms_money}")],
        },
        SpecialRoutineEffect::SlotMachine { coins, .. }
        | SpecialRoutineEffect::CardFlip { coins, .. }
        | SpecialRoutineEffect::UnusedMemoryGame { coins, .. } => SpecialBoundaryDisplay {
            label: match effect {
                SpecialRoutineEffect::SlotMachine { .. } => "SlotMachine",
                SpecialRoutineEffect::CardFlip { .. } => "CardFlip",
                _ => "UnusedMemoryGame",
            }
            .to_string(),
            details: vec![format!("coins={coins}")],
        },
        SpecialRoutineEffect::CheckMagikarpLength {
            party_slot,
            species,
            feet,
            inches,
            result,
        } => SpecialBoundaryDisplay {
            label: "CheckMagikarpLength".to_string(),
            details: vec![
                format!("party_slot={party_slot}"),
                format!("species={species}"),
                format!("length={}ft {}in", feet, inches),
                format!("result={result}"),
            ],
        },
        SpecialRoutineEffect::MagikarpHouseSign {
            feet,
            inches,
            formatted,
        } => SpecialBoundaryDisplay {
            label: "MagikarpHouseSign".to_string(),
            details: vec![
                format!("length={}ft {}in", feet, inches),
                format!("formatted={formatted}"),
            ],
        },
        SpecialRoutineEffect::DisplayLinkRecord {
            wins,
            losses,
            draws,
        } => SpecialBoundaryDisplay {
            label: "DisplayLinkRecord".to_string(),
            details: vec![
                format!("wins={wins}"),
                format!("losses={losses}"),
                format!("draws={draws}"),
            ],
        },
        SpecialRoutineEffect::TrainerHouse { enabled } => SpecialBoundaryDisplay {
            label: "TrainerHouse".to_string(),
            details: vec![format!("enabled={enabled}")],
        },
        SpecialRoutineEffect::PhotoStudio {
            party_slot,
            species,
        } => SpecialBoundaryDisplay {
            label: "PhotoStudio".to_string(),
            details: vec![
                format!("party_slot={}", optional_usize_label(*party_slot)),
                format!("species={}", species.as_deref().unwrap_or("-")),
            ],
        },
        SpecialRoutineEffect::CheckForBattleTowerRules { failure } => SpecialBoundaryDisplay {
            label: "CheckForBattleTowerRules".to_string(),
            details: vec![format!("failure={}", failure.as_deref().unwrap_or("-"))],
        },
        SpecialRoutineEffect::BattleTowerChallengeExplanationCancel => SpecialBoundaryDisplay {
            label: "BattleTowerChallengeExplanationCancel".to_string(),
            details: Vec::new(),
        },
        SpecialRoutineEffect::BattleTowerRoomMenu { records } => SpecialBoundaryDisplay {
            label: "BattleTowerRoomMenu".to_string(),
            details: vec![format!("records={}", records.len())],
        },
        SpecialRoutineEffect::BattleTowerLeaderboard {
            records,
            acknowledged,
        } => SpecialBoundaryDisplay {
            label: "BattleTowerLeaderboard".to_string(),
            details: vec![
                format!("records={}", records.len()),
                format!("acknowledged={acknowledged}"),
            ],
        },
        SpecialRoutineEffect::BattleTowerMobileError => SpecialBoundaryDisplay {
            label: "BattleTowerMobileError".to_string(),
            details: Vec::new(),
        },
        SpecialRoutineEffect::GameCornerGameUnavailable { game, reason } => {
            SpecialBoundaryDisplay {
                label: "GameCornerGameUnavailable".to_string(),
                details: vec![format!("game={game}"), format!("reason={reason:?}")],
            }
        }
        _ => SpecialBoundaryDisplay {
            label: "SpecialRoutine".to_string(),
            details: Vec::new(),
        },
    }
}

fn optional_usize_label(value: Option<usize>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "-".to_string())
}

fn optional_u8_label(value: Option<u8>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "-".to_string())
}

fn optional_tile_label(value: Option<(i16, i16)>) -> String {
    value
        .map(|(x, y)| format!("({x},{y})"))
        .unwrap_or_else(|| "-".to_string())
}

fn open_visible_vertical_menu_for_script_command(
    runtime_shell: &mut BevyRuntimeShell,
    source_script: &str,
    command_index: usize,
) -> Result<bool> {
    let Some(key) = runtime_shell
        .shell
        .script_vertical_menu_keys()
        .into_iter()
        .find(|key| {
            key.source_script == source_script && key.verticalmenu_command_index == command_index
        })
    else {
        return Ok(false);
    };
    let map_name = key.map_name.clone();
    let menu_key = key.menu_key.clone();
    let key_source_script = key.source_script.clone();
    let loadmenu_command_index = key.loadmenu_command_index;
    let verticalmenu_command_index = key.verticalmenu_command_index;
    record_visible_runtime_action(
        runtime_shell,
        format!(
            "ui:open_vertical_menu:{}:{}:{}:{}:{}",
            map_name.as_str(),
            menu_key.as_str(),
            key_source_script.as_str(),
            loadmenu_command_index,
            verticalmenu_command_index
        ),
    )?;
    let opened = runtime_shell.shell.open_vertical_menu(
        map_name,
        menu_key,
        key_source_script.clone(),
        loadmenu_command_index,
        verticalmenu_command_index,
    )?;
    close_visible_field_pack_without_log(runtime_shell);
    runtime_shell.battle_pack_target_mode = None;
    runtime_shell.pack_toss = None;
    runtime_shell.party_move_cursor = None;
    runtime_shell.menu_cursor = Some(MenuCursor {
        surface_id: format!(
            "{}:{}:{}",
            opened.menu_id, key_source_script, verticalmenu_command_index
        ),
        option_index: 0,
    });
    runtime_shell.last_audio_events.push(format!(
        "opened vertical menu {} options={} checksum={:?}",
        opened.menu_id,
        opened.options.len(),
        opened.state_checksum
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(true)
}

fn open_visible_elevator_for_script_command(
    runtime_shell: &mut BevyRuntimeShell,
    source_script: &str,
    command_index: usize,
) -> Result<bool> {
    let map_name = runtime_shell.shell.current_map_name().to_string();
    if !runtime_shell
        .shell
        .has_script_elevator_command_at(&map_name, source_script, command_index)
    {
        return Ok(false);
    }
    let snapshot = runtime_shell.shell.snapshot()?;
    let elevators: Vec<&RuntimeElevatorSnapshot> = snapshot
        .ui
        .elevators
        .iter()
        .filter(|elevator| {
            elevator.map_name == snapshot.overworld.map_name
                && elevator.source_script == source_script
                && elevator.elevator_command_index == command_index
        })
        .collect();
    if elevators.is_empty() {
        return Ok(false);
    }
    let option_count: usize = elevators.iter().map(|elevator| elevator.floors.len()).sum();
    if option_count == 0 {
        anyhow::bail!(
            "elevator command {source_script}:{command_index} exists but has no floors on {}",
            snapshot.overworld.map_name
        );
    }
    let surface_id = elevator_surface_id(source_script, command_index);
    record_visible_runtime_action(
        runtime_shell,
        format!(
            "ui:open_elevator:{}:{}:{}:{}",
            snapshot.overworld.map_name, source_script, command_index, option_count
        ),
    )?;
    close_visible_field_pack_without_log(runtime_shell);
    runtime_shell.battle_pack_target_mode = None;
    runtime_shell.party_move_cursor = None;
    runtime_shell.elevator_cursor = Some(MenuCursor {
        surface_id,
        option_index: 0,
    });
    runtime_shell.last_audio_events.push(format!(
        "opened elevator prompt {}:{} floors={}",
        source_script, command_index, option_count
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(true)
}

fn open_visible_gift_pokemon_for_script_command(
    runtime_shell: &mut BevyRuntimeShell,
    source_script: &str,
    command_index: usize,
) -> Result<bool> {
    let map_name = runtime_shell.shell.current_map_name().to_string();
    if !runtime_shell
        .shell
        .has_gift_pokemon_command_at(&map_name, source_script, command_index)
    {
        return Ok(false);
    }
    let snapshot = runtime_shell.shell.snapshot()?;
    let surface_id = gift_pokemon_surface_id(source_script, command_index);
    let option_count = snapshot
        .ui
        .gift_pokemon
        .iter()
        .filter(|gift| {
            gift.map_name == snapshot.overworld.map_name
                && gift.source_script == source_script
                && gift.command_index == command_index
        })
        .count();
    if option_count == 0 {
        anyhow::bail!(
            "gift Pokemon command {source_script}:{command_index} exists but has no prompt option on {}",
            snapshot.overworld.map_name
        );
    }
    record_visible_runtime_action(
        runtime_shell,
        format!(
            "ui:open_gift_pokemon:{}:{}:{}:{}",
            snapshot.overworld.map_name, source_script, command_index, option_count
        ),
    )?;
    close_visible_field_pack_without_log(runtime_shell);
    runtime_shell.battle_pack_target_mode = None;
    runtime_shell.party_move_cursor = None;
    runtime_shell.gift_pokemon_cursor = Some(MenuCursor {
        surface_id,
        option_index: 0,
    });
    runtime_shell.last_audio_events.push(format!(
        "opened gift Pokemon prompt {}:{} options={}",
        source_script, command_index, option_count
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(true)
}

fn open_visible_phone_prompt_for_script_command(
    runtime_shell: &mut BevyRuntimeShell,
    source_script: &str,
    command_index: usize,
) -> Result<bool> {
    let map_name = runtime_shell.shell.current_map_name().to_string();
    if !runtime_shell.shell.has_script_phone_prompt_command_at(
        &map_name,
        source_script,
        command_index,
    ) {
        return Ok(false);
    }
    let snapshot = runtime_shell.shell.snapshot()?;
    let Some(command) = runtime_shell
        .shell
        .script_phone_command_keys()
        .into_iter()
        .find(|command| {
            command.map_name == snapshot.overworld.map_name
                && command.source_script == source_script
                && command.command_index == command_index
        })
    else {
        return Ok(false);
    };
    if command.command != "askforphonenumber" {
        return Ok(false);
    }
    record_visible_runtime_action(
        runtime_shell,
        format!(
            "ui:open_phone_number:{}:{}:{}:{}",
            snapshot.overworld.map_name, source_script, command_index, command.contact_id
        ),
    )?;
    close_visible_field_pack_without_log(runtime_shell);
    runtime_shell.battle_pack_target_mode = None;
    runtime_shell.party_move_cursor = None;
    runtime_shell.pending_phone_prompt = Some(PendingPhonePrompt {
        source_script: source_script.to_string(),
        command_index,
        contact_id: command.contact_id.clone(),
    });
    runtime_shell.yes_no_cursor = Some(MenuCursor {
        surface_id: "ui:phone-number".to_string(),
        option_index: 0,
    });
    runtime_shell.last_audio_events.push(format!(
        "opened phone number prompt {}:{} contact={}",
        source_script, command_index, command.contact_id
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(true)
}

fn open_visible_day_of_week_for_script_command(
    runtime_shell: &mut BevyRuntimeShell,
    source_script: &str,
    command_index: usize,
) -> Result<bool> {
    if compiled_special_routine_at(runtime_shell, source_script, command_index)?.as_deref()
        != Some("SetDayOfWeek")
    {
        return Ok(false);
    }
    let origin_map_name = runtime_shell.shell.current_map_name().to_string();
    record_visible_runtime_action(
        runtime_shell,
        format!("ui:open_day_of_week:{source_script}:{command_index}"),
    )?;
    runtime_shell.pending_day_of_week = Some(PendingDayOfWeekPrompt {
        origin_map_name,
        source_script: source_script.to_string(),
        command_index,
        selected_day: 0,
        confirming: false,
        yes_no_index: 0,
    });
    set_shell_action_status(runtime_shell, "WHAT DAY IS IT?");
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(true)
}

fn has_visible_compiled_script_command(
    runtime_shell: &BevyRuntimeShell,
    script: &str,
    command_index: usize,
) -> bool {
    runtime_shell
        .shell
        .runtime()
        .compiled_script_command_name(script, command_index)
        .is_ok()
}

fn execute_last_interaction_script(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let Some(interaction) = runtime_shell
        .shell
        .last_frame()
        .and_then(|frame| frame.interaction.clone())
    else {
        record_visible_runtime_action(runtime_shell, "overworld:interaction:none_recorded")?;
        runtime_shell
            .last_audio_events
            .push("no object or background interaction has been recorded".to_string());
        set_shell_action_status(runtime_shell, "NOTHING THERE");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    };
    dispatch_visible_overworld_interaction(runtime_shell, interaction, "interaction")
}

fn execute_current_overworld_interaction_script(
    runtime_shell: &mut BevyRuntimeShell,
) -> Result<()> {
    let Some(interaction) = runtime_shell
        .shell
        .current_overworld_interaction_checked()?
    else {
        record_visible_runtime_action(runtime_shell, "overworld:current_interaction:none")?;
        runtime_shell
            .last_audio_events
            .push("no current object or background interaction is visible".to_string());
        set_shell_action_status(runtime_shell, "NOTHING THERE");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    };
    dispatch_visible_overworld_interaction(runtime_shell, interaction, "current_interaction")
}

fn dispatch_visible_overworld_interaction(
    runtime_shell: &mut BevyRuntimeShell,
    interaction: crate::core::world::session::OverworldInteraction,
    action_kind: &'static str,
) -> Result<()> {
    // Every successful A-button interaction owns one acknowledgement cue:
    // object and background events call PlayTalkObject, while the successful
    // tile-collision path converges on PlayClickSFX before script dispatch.
    queue_visible_shell_sound_effect(runtime_shell, "SFX_READ_TEXT_2")?;
    record_visible_runtime_action(
        runtime_shell,
        format!(
            "overworld:{action_kind}:{}:{:?}",
            interaction.script, interaction.target
        ),
    )?;
    let dispatch = runtime_shell
        .shell
        .dispatch_interaction_script(&interaction)?;
    runtime_shell.last_audio_events.push(format!(
        "interaction script={} target={:?} last_talked={:?} checksum={:?}",
        interaction.script,
        interaction.target,
        dispatch.last_talked_object,
        dispatch.state_checksum
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    take_visible_next_script(runtime_shell)?;
    // `take_visible_next_script` already runs the compiled entry through its
    // first authored boundary and integrates that boundary's presentation.
    // Advancing again here races the renderer that creates the typewriter
    // state and consumes `waitbutton`/`closetext` in the same A press.
    Ok(())
}

fn execute_last_trainer_sight_script(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let Some(interaction) = runtime_shell
        .shell
        .last_frame()
        .and_then(|frame| frame.trainer_sight.clone())
    else {
        record_visible_runtime_action(runtime_shell, "overworld:trainer_sight:none_recorded")?;
        runtime_shell
            .last_audio_events
            .push("no trainer sight interaction has been recorded".to_string());
        set_shell_action_status(runtime_shell, "NO TRAINER SIGHT");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    };
    record_visible_runtime_action(
        runtime_shell,
        format!(
            "overworld:trainer_sight:{}:{:?}",
            interaction.script, interaction.target
        ),
    )?;
    let crate::core::world::session::OverworldInteractionTarget::Object {
        object_identifier: Some(object_id),
        ..
    } = &interaction.target
    else {
        anyhow::bail!(
            "trainer sight interaction {} has no identified object target",
            interaction.script
        );
    };
    let object_id = object_id.clone();
    let delta_x = interaction
        .player_tile
        .x
        .checked_sub(interaction.target_tile.x)
        .context("trainer sight horizontal distance overflow")?;
    let delta_y = interaction
        .player_tile
        .y
        .checked_sub(interaction.target_tile.y)
        .context("trainer sight vertical distance overflow")?;
    let (direction, distance) = match (delta_x, delta_y) {
        (0, delta) if delta > 0 => (Direction::Down, delta.unsigned_abs()),
        (0, delta) if delta < 0 => (Direction::Up, delta.unsigned_abs()),
        (delta, 0) if delta > 0 => (Direction::Right, delta.unsigned_abs()),
        (delta, 0) if delta < 0 => (Direction::Left, delta.unsigned_abs()),
        _ => anyhow::bail!(
            "trainer sight interaction {} is not axis aligned: trainer={:?} player={:?}",
            interaction.script,
            interaction.target_tile,
            interaction.player_tile
        ),
    };
    let distance_tiles = distance
        / u16::try_from(crate::core::world::movement::DEFAULT_RUNTIME_TILE_STRIDE)
            .context("runtime tile stride is not positive")?;
    let steps_remaining = distance_tiles.saturating_sub(1);
    runtime_shell.visible_overworld_emote = Some(VisibleOverworldEmote {
        emote: "EMOTE_SHOCK".to_string(),
        object: object_id.clone(),
        frames_remaining: 30,
    });
    runtime_shell.pending_trainer_sight = Some(PendingTrainerSight {
        interaction,
        object_id,
        direction,
        steps_remaining,
        // SeenByTrainerScript inserts `step_sleep 1` before the path.
        frames_until_step: 1,
    });
    mark_runtime_snapshot_dirty(runtime_shell);
    set_shell_action_status(runtime_shell, "TRAINER SPOTTED");
    Ok(())
}

fn finish_visible_trainer_sight_script(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let pending = runtime_shell
        .pending_trainer_sight
        .take()
        .context("trainer sight cutscene completed without retained interaction")?;
    let opposite = match pending.direction {
        Direction::Up => Direction::Down,
        Direction::Down => Direction::Up,
        Direction::Left => Direction::Right,
        Direction::Right => Direction::Left,
    };
    runtime_shell.trainer_walk_from = None;
    runtime_shell.object_walk_from.clear();
    runtime_shell.object_walk_frame_ticks_by_id.clear();
    runtime_shell.object_walk_total_ticks_by_id.clear();
    runtime_shell.pending_overworld_step_boundary = None;
    runtime_shell.pending_overworld_warp_scene = None;
    runtime_shell.visible_script_movement = None;
    runtime_shell.visible_script_movement_scene = None;
    runtime_shell.object_walk_frame_ticks = 0;
    runtime_shell.object_walk_total_ticks = WALK_FRAME_HOLD_TICKS;
    runtime_shell.object_walk_stride = false;
    {
        let overworld = &mut runtime_shell.shell.session_mut().overworld;
        overworld.set_object_runtime_facing(&pending.object_id, pending.direction)?;
        overworld.set_player_facing(opposite);
    }
    let dispatch = runtime_shell
        .shell
        .dispatch_interaction_script(&pending.interaction)?;
    runtime_shell.last_audio_events.push(format!(
        "trainer sight script={} target={:?} last_talked={:?} checksum={:?}",
        pending.interaction.script,
        pending.interaction.target,
        dispatch.last_talked_object,
        dispatch.state_checksum
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    take_visible_next_script(runtime_shell)?;
    advance_visible_script_until_player_boundary(runtime_shell)
}

fn execute_last_coord_event_script(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let Some(coord_event) = runtime_shell
        .shell
        .last_frame()
        .and_then(|frame| frame.coord_event.clone())
    else {
        record_visible_runtime_action(runtime_shell, "overworld:coord_event:none_recorded")?;
        runtime_shell
            .last_audio_events
            .push("no coord event has been recorded".to_string());
        set_shell_action_status(runtime_shell, "NO COORD EVENT");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    };
    record_visible_runtime_action(
        runtime_shell,
        format!(
            "overworld:coord_event:{}:{}:{}:{}",
            coord_event.script_name, coord_event.tile.x, coord_event.tile.y, coord_event.scene_id
        ),
    )?;
    let dispatch = runtime_shell
        .shell
        .dispatch_coord_event_script(&coord_event)?;
    runtime_shell.last_audio_events.push(format!(
        "coord event script={} tile=({}, {}) scene={} checksum={:?}",
        coord_event.script_name,
        coord_event.tile.x,
        coord_event.tile.y,
        coord_event.scene_id,
        dispatch.state_checksum
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    take_visible_next_script(runtime_shell)?;
    advance_visible_script_until_player_boundary(runtime_shell)
}

fn execute_visible_pending_script_warp(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    record_visible_runtime_action(runtime_shell, "script:pending:warp")?;
    let next_cursor = visible_active_compiled_script_cursor(runtime_shell);
    let transitioned = if let Some(cursor) = next_cursor {
        runtime_shell
            .shell
            .transition_script_warp_and_run_compiled_script(
                Some(cursor),
                256,
                ScriptRuntimeInputs::default(),
                ScriptPhoneInputs::default(),
            )?
    } else {
        let warp = runtime_shell.shell.execute_pending_script_warp()?;
        crate::RuntimeScriptWarpCompiledScriptRun {
            warp,
            run: crate::RuntimeCompiledScriptRun {
                steps: Vec::new(),
                next_cursor: None,
                boundary: None,
                ended: false,
            },
        }
    };
    let warp = transitioned.warp;
    runtime_shell.last_audio_events.push(format!(
        "script warp target={} tile=({}, {}) facing={:?} resumed_steps={} checksum={:?}",
        warp.target_map,
        warp.tile.x,
        warp.tile.y,
        warp.facing,
        transitioned.run.steps.len(),
        warp.state_checksum
    ));
    let reached_boundary =
        integrate_visible_compiled_script_run(runtime_shell, &transitioned.run.steps)?;
    arm_visible_active_script_cursor_from_run(runtime_shell, transitioned.run.next_cursor);
    if reached_boundary {
        return Ok(());
    }
    // Script_warp enters through MAPSETUP_WARP: the LCD is blanked during
    // the map replacement and the destination fades in from white. It does
    // not use MAPSETUP_DOOR's gradual source-map fade-out.
    reset_visible_navigation_state(runtime_shell);
    suppress_visible_map_name_sign_for_current_map(runtime_shell)?;
    queue_visible_current_music(runtime_shell)?;
    runtime_shell.visible_walk_warp_phase = Some(VisibleWalkWarpPhase::ScriptFadeIn);
    runtime_shell.screen_fade = Some(VisibleScreenFade::new(
        ScriptFadeColor::White,
        ScriptFadeDirection::In,
        8,
    ));
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(())
}

fn settle_visible_overworld_frame_arrival(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    // Arrival is a single-consumer boundary. The real keyboard schedule takes
    // it before calling here, but direct smoke/replay drivers enter this same
    // function after an authoritative tick. Consume both retained boundary
    // records here as well so a later script boundary cannot replay the old
    // warp and replace the currently running destination script.
    runtime_shell.pending_overworld_step_boundary = None;
    runtime_shell.pending_overworld_warp_scene = None;
    let Some(frame) = runtime_shell.shell.last_frame().cloned() else {
        record_visible_runtime_action(runtime_shell, "overworld:arrival:no_frame")?;
        runtime_shell
            .last_audio_events
            .push("no overworld frame has been recorded".to_string());
        set_shell_action_status(runtime_shell, "NO OVERWORLD FRAME");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    };
    let reason = if let Some(warp) = frame.warp.as_ref() {
        if runtime_shell.visible_walk_warp_phase.is_none() {
            record_visible_runtime_action(runtime_shell, "overworld:walk_warp:fade_out")?;
            let permission = warp.trigger.permission;
            let sfx_id = match permission {
                // PIT/PIT_68 dispatch FallIntoMapScript, which uses
                // newloadmap MAPSETUP_FALL without the `warpsound` command.
                crate::core::world::collision::permissions::PIT
                | crate::core::world::collision::permissions::PIT_68 => None,
                // GetWarpSFX compares the complete collision byte against
                // COLL_DOOR only. Alternate doors, stairs, caves, ladders,
                // and carpets all fall through to SFX_EXIT_BUILDING.
                crate::core::world::collision::permissions::DOOR => Some("SFX_ENTER_DOOR"),
                crate::core::world::collision::permissions::WARP_PANEL => Some("SFX_WARP_TO"),
                _ => Some("SFX_EXIT_BUILDING"),
            };
            if let Some(sfx_id) = sfx_id {
                queue_visible_shell_sound_effect(runtime_shell, sfx_id)?;
            }
            runtime_shell.visible_walk_warp_phase = Some(VisibleWalkWarpPhase::FadeOut);
            runtime_shell.screen_fade = Some(VisibleScreenFade::new(
                ScriptFadeColor::White,
                ScriptFadeDirection::Out,
                8,
            ));
            mark_runtime_snapshot_dirty(runtime_shell);
            return Ok(());
        }
        "walk_warp"
    } else if frame.connection.is_some() {
        "map_connection"
    } else {
        record_visible_runtime_action(runtime_shell, "overworld:arrival:no_transition")?;
        runtime_shell
            .last_audio_events
            .push("last overworld frame has no warp or connection transition".to_string());
        set_shell_action_status(runtime_shell, "NO MAP TRANSITION");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    };
    settle_visible_overworld_arrival(runtime_shell, reason)
}

fn settle_visible_overworld_travel(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    settle_visible_overworld_arrival(runtime_shell, "field_travel")
}

fn suppress_visible_map_name_sign_for_current_map(
    runtime_shell: &mut BevyRuntimeShell,
) -> Result<()> {
    let map_name = runtime_shell.shell.current_map_name();
    runtime_shell.previous_map_sign_landmark = if matches!(
        map_name,
        "Route35NationalParkGate" | "Route36NationalParkGate"
    ) {
        Some("__MAP_NAME_SIGN_SENTINEL__".to_string())
    } else {
        runtime_shell
            .shell
            .runtime()
            .data()
            .pokegear_landmarks
            .map_to_landmark
            .get(map_name)
            .cloned()
    };
    runtime_shell.visible_map_name_sign = None;
    Ok(())
}

fn settle_visible_overworld_arrival(
    runtime_shell: &mut BevyRuntimeShell,
    reason: &str,
) -> Result<()> {
    if reason == "new_game" {
        runtime_shell.shell.set_game_timer_counting(true)?;
    }
    let connection_continuation = (reason == "map_connection").then(|| {
        (
            runtime_shell.overworld_held_direction,
            runtime_shell.overworld_held_directions.clone(),
            runtime_shell.overworld_buffered_direction,
            runtime_shell.player_walk_stride,
            runtime_shell.player_walk_mirror_stride,
        )
    });
    reset_visible_navigation_state(runtime_shell);
    if let Some((held_direction, held_directions, buffered_direction, stride, mirror_stride)) =
        connection_continuation
    {
        // MAPSETUP_CONNECTION is seamless. Keep the live D-pad arbitration
        // and the current foot phase across the map boundary; door/warp/fly
        // arrivals retain the full input reset above.
        runtime_shell.overworld_held_direction = held_direction;
        runtime_shell.overworld_held_directions = held_directions;
        runtime_shell.overworld_buffered_direction = buffered_direction;
        runtime_shell.overworld_direction_repeat_ticks = 0;
        runtime_shell.player_walk_stride = stride;
        runtime_shell.player_walk_mirror_stride = mirror_stride;
    }
    if reason != "map_connection" {
        suppress_visible_map_name_sign_for_current_map(runtime_shell)?;
    }
    let script_events = runtime_shell.shell.script_events_snapshot();
    if script_events.pending_map_load.is_some() {
        return take_visible_pending_map_load(runtime_shell);
    }
    if script_events.pending_map_refresh.is_some() {
        return take_visible_pending_map_refresh(runtime_shell);
    }
    arm_visible_current_scene_script(runtime_shell, reason)?;
    arm_visible_current_map_callbacks(runtime_shell, reason)?;
    // A callback can terminate at its explicit End/EndCallback boundary after
    // publishing passive control/graphics records. Those records are already
    // reflected in authoritative state and must not remain as "script work"
    // that captures the first overworld direction or A press after arrival.
    // A drain may itself let the callback reach its final opcode, so repeat
    // until continuation publishes no more passive records. Genuine text,
    // menu, and yes/no boundaries remain untouched.
    for _ in 0..64 {
        continue_visible_script_after_prompt(runtime_shell)?;
        let snapshot = runtime_shell.shell.snapshot()?;
        let has_audio = !snapshot.script_events.audio_events.is_empty();
        let has_non_audio = has_visible_pending_non_audio_script_events(&snapshot);
        if !has_audio && !has_non_audio {
            break;
        }
        if has_audio {
            drain_visible_audio_events(runtime_shell)?;
        }
        if has_non_audio {
            drain_visible_non_audio_script_events(runtime_shell)?;
        }
    }
    close_visible_noninteractive_runtime_surfaces_until_idle(runtime_shell)?;
    queue_visible_current_music(runtime_shell)?;
    Ok(())
}

fn restore_visible_loaded_runtime_state(
    runtime_shell: &mut BevyRuntimeShell,
    reason: &str,
) -> Result<()> {
    reset_visible_navigation_state(runtime_shell);
    reset_visible_deterministic_session_history(runtime_shell)?;
    reset_visible_music_state(runtime_shell);
    queue_visible_current_music(runtime_shell)?;
    let snapshot = runtime_shell.shell.snapshot()?;
    runtime_shell.previous_map_sign_landmark = if matches!(
        snapshot.overworld.map_name.as_str(),
        "Route35NationalParkGate" | "Route36NationalParkGate"
    ) {
        Some("__MAP_NAME_SIGN_SENTINEL__".to_string())
    } else {
        snapshot
            .presentation
            .pokegear_landmarks
            .map_to_landmark
            .get(&snapshot.overworld.map_name)
            .cloned()
    };
    runtime_shell.visible_map_name_sign = None;
    record_visible_runtime_action(
        runtime_shell,
        format!(
            "save:restore:{}:{}:{}:{}",
            reason,
            snapshot.overworld.map_name,
            snapshot.overworld.tile.x,
            snapshot.overworld.tile.y
        ),
    )?;
    set_shell_action_status(
        runtime_shell,
        format!(
            "RESTORED {} ({},{})",
            snapshot.overworld.map_name, snapshot.overworld.tile.x, snapshot.overworld.tile.y
        ),
    );
    Ok(())
}

fn reset_script_cursors(runtime_shell: &mut BevyRuntimeShell) {
    runtime_shell.active_script_cursor = None;
    runtime_shell
        .last_audio_events
        .push("reset script cursors".to_string());
    trim_event_log(&mut runtime_shell.last_audio_events);
}

fn quick_save(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    quick_save_with_policy(runtime_shell, false, false)
}

fn quick_save_from_script(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    quick_save_with_policy(runtime_shell, true, false)
}

fn quick_save_from_menu(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    quick_save_with_policy(runtime_shell, false, true)
}

fn quick_save_with_policy(
    runtime_shell: &mut BevyRuntimeShell,
    allow_active_script_cursor: bool,
    allow_save_menu: bool,
) -> Result<()> {
    let Some(path) = runtime_shell.quick_save_path.clone() else {
        anyhow::bail!("quick-save has no configured .crystalsave path");
    };
    let snapshot_before_save = runtime_shell.shell.snapshot()?;
    if runtime_shell.pending_time_set.is_some()
        || runtime_shell.pending_oak_intro.is_some()
        || runtime_shell.pending_gender_selection.is_some()
        || runtime_shell.pending_name_choice.is_some()
        || runtime_shell.pending_name_input.is_some()
        || snapshot_before_save.trainer.player_name.is_empty()
    {
        record_visible_runtime_action(runtime_shell, "save:write:player_name_pending")?;
        runtime_shell
            .last_audio_events
            .push("cannot save before the player name is confirmed".to_string());
        set_shell_action_status(runtime_shell, "CAN'T SAVE YET");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    let blockers = visible_quick_save_blockers(
        runtime_shell,
        &snapshot_before_save,
        allow_active_script_cursor,
        allow_save_menu,
    );
    if !blockers.is_empty() {
        record_visible_runtime_action(
            runtime_shell,
            format!("save:write:blocked:{}", blockers.join("|")),
        )?;
        runtime_shell.last_audio_events.push(format!(
            "cannot save while runtime surfaces are active: {}",
            blockers.join(", ")
        ));
        set_shell_action_status(runtime_shell, "CAN'T SAVE NOW");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    let modpack_id = snapshot_before_save.boot.modpack_id.clone();
    let modpack_hash = runtime_shell.shell.runtime().modpack().hash().to_string();
    let pack_content_hash = snapshot_before_save.boot.pack_content_hash.clone();
    record_visible_runtime_action(
        runtime_shell,
        format!(
            "save:write:{}:{}:{}:{}",
            path.display(),
            modpack_id,
            modpack_hash,
            pack_content_hash
        ),
    )?;
    runtime_shell.shell.save(&path)?;
    let summary = runtime_shell.shell.runtime().load_save_summary(&path)?;
    runtime_shell.last_audio_events.push(format!(
        "saved {} frame={} pack_hash={} state_hash={:#010x}",
        path.display(),
        summary.saved_frame(),
        summary.pack_content_hash(),
        summary.state_hash()
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    set_shell_action_status(
        runtime_shell,
        if runtime_debug_overlays_enabled() {
            format!(
                "SAVED FRAME {} PACK {} HASH {:#010x}",
                summary.saved_frame(),
                summary.modpack().id(),
                summary.state_hash()
            )
        } else {
            format!("SAVED FRAME {}", summary.saved_frame())
        },
    );
    Ok(())
}

fn quick_load(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let Some(path) = runtime_shell.quick_save_path.clone() else {
        anyhow::bail!("quick-load has no configured .crystalsave path");
    };
    let summary = runtime_shell.shell.runtime().load_save_summary(&path)?;
    let snapshot_before_load = runtime_shell.shell.snapshot()?;
    let blockers = visible_quick_load_blockers(runtime_shell, &snapshot_before_load);
    if !blockers.is_empty() {
        record_visible_runtime_action(
            runtime_shell,
            format!("save:load:blocked:{}", blockers.join("|")),
        )?;
        runtime_shell.last_audio_events.push(format!(
            "cannot load while runtime surfaces are active: {}",
            blockers.join(", ")
        ));
        set_shell_action_status(runtime_shell, "CAN'T LOAD NOW");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    record_visible_runtime_action(
        runtime_shell,
        format!(
            "save:load:{}:{}:{}:{}",
            path.display(),
            summary.modpack().id(),
            summary.modpack().hash(),
            summary.pack_content_hash()
        ),
    )?;
    load_visible_runtime_save(runtime_shell, &path, "quick_load")?;
    runtime_shell.last_audio_events.push(format!(
        "loaded {} frame={} pack_hash={} state_hash={:#010x}",
        path.display(),
        summary.saved_frame(),
        summary.pack_content_hash(),
        summary.state_hash()
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    set_shell_action_status(
        runtime_shell,
        if runtime_debug_overlays_enabled() {
            format!(
                "LOADED FRAME {} PACK {} HASH {:#010x}",
                summary.saved_frame(),
                summary.modpack().id(),
                summary.state_hash()
            )
        } else {
            format!("LOADED FRAME {}", summary.saved_frame())
        },
    );
    Ok(())
}

fn load_visible_runtime_save(
    runtime_shell: &mut BevyRuntimeShell,
    path: &PathBuf,
    arrival_reason: &str,
) -> Result<()> {
    runtime_shell.shell.load(path)?;
    let post_credits_spawn = runtime_shell
        .shell
        .session()
        .state()
        .hall_of_fame
        .spawn_after_champion;
    let mut post_credits_warped = false;
    if let Some(marker) = post_credits_spawn {
        // Match intro_menu.asm::FinishContinueFunction: the marker is a
        // one-shot continuation destination. Use the normal spawn-point warp
        // path so map setup, music, objects, and coordinates stay authoritative.
        let spawn_identifier = match marker {
            1 => POST_CREDITS_NEW_BARK_SPAWN_IDENTIFIER,
            2 => POST_CREDITS_MT_SILVER_SPAWN_IDENTIFIER,
            _ => anyhow::bail!("unsupported post-credits spawn marker {marker}"),
        };
        let spawn = runtime_shell
            .shell
            .snapshot()?
            .spawn_points
            .iter()
            .find(|spawn| spawn.identifier == spawn_identifier)
            .cloned()
            .with_context(|| {
                format!("compiled pack is missing post-credits spawn {spawn_identifier}")
            })?;
        let state = runtime_shell.shell.session_mut().state_mut();
        state.last_spawn_identifier = Some(spawn_identifier);
        state.hall_of_fame.spawn_after_champion = None;
        state
            .script_runtime
            .variables
            .insert("wLastSpawnMapGroup".to_string(), spawn.group_id.to_string());
        state
            .script_runtime
            .variables
            .insert("wLastSpawnMapNumber".to_string(), spawn.map_id.to_string());
        let warped = runtime_shell.shell.warp_to_spawn_point()?;
        runtime_shell.shell.execute_pending_script_warp()?;
        post_credits_warped = true;
        runtime_shell.last_audio_events.push(format!(
            "post-credits continue spawn={} outcome={:?} checksum={:?}",
            spawn_identifier, warped.outcome.effect, warped.state_checksum
        ));
    }
    runtime_shell.title_menu = None;
    runtime_shell.credits_screen = None;
    runtime_shell.pending_time_set = None;
    runtime_shell.pending_oak_intro = None;
    runtime_shell.pending_gender_selection = None;
    if post_credits_warped {
        settle_visible_overworld_arrival(runtime_shell, "post_credits")
    } else {
        restore_visible_loaded_runtime_state(runtime_shell, arrival_reason)
    }
}

fn reset_visible_navigation_state(runtime_shell: &mut BevyRuntimeShell) {
    reset_visible_script_navigation_state(runtime_shell);
    reset_visible_selection_cursors(runtime_shell);
    runtime_shell.pending_name_input = None;
    runtime_shell.pending_name_choice = None;
    runtime_shell.pending_delete_save = None;
    runtime_shell.pending_clock_reset = None;
    runtime_shell.pending_time_set = None;
    runtime_shell.pending_oak_intro = None;
    runtime_shell.pending_gender_selection = None;
    runtime_shell.visible_blackout_phase = None;
    runtime_shell.visible_walk_warp_phase = None;
    runtime_shell.credits_screen = None;
    runtime_shell.last_battle_cry_key = None;
    runtime_shell.pending_battle_cries_after_messages.clear();
    runtime_shell.battle_enemy_send_out_pending = false;
    runtime_shell.battle_player_send_out_pending = false;
    runtime_shell.battle_enemy_hp_at_player_send_out = None;
    runtime_shell.pending_battle_scenes_after_message.clear();
    runtime_shell.pending_enemy_response_after_capture = None;
    runtime_shell.visible_capture_animation = None;
    runtime_shell.visible_move_animations.clear();
    runtime_shell.visible_send_out_animation = None;
    runtime_shell.visible_trainer_exit_animation = None;
    runtime_shell.visible_frontpic_animation = None;
    runtime_shell.visible_fishing_animation = None;
    runtime_shell.last_overworld_input = None;
    runtime_shell.overworld_direction_repeat_ticks = 0;
    runtime_shell.overworld_held_direction = None;
    runtime_shell.overworld_held_directions.clear();
    runtime_shell.overworld_buffered_direction = None;
    runtime_shell.ui_held_direction = None;
    runtime_shell.ui_direction_repeat_ticks = 0;
    runtime_shell.player_walk_frame_ticks = 0;
    runtime_shell.player_walk_total_ticks = WALK_FRAME_HOLD_TICKS;
    runtime_shell.player_walk_stride = false;
    runtime_shell.player_walk_mirror_stride = false;
    runtime_shell.object_walk_frame_ticks = 0;
    runtime_shell.object_walk_total_ticks = WALK_FRAME_HOLD_TICKS;
    runtime_shell.object_walk_stride = false;
    runtime_shell.object_walk_from.clear();
    runtime_shell.object_walk_frame_ticks_by_id.clear();
    runtime_shell.object_walk_total_ticks_by_id.clear();
    runtime_shell.object_walk_phases.clear();
    runtime_shell.object_walk_directions.clear();
    runtime_shell.pending_overworld_step_boundary = None;
    runtime_shell.pending_overworld_warp_scene = None;
    runtime_shell.visible_script_movement = None;
    runtime_shell.visible_script_movement_scene = None;
    runtime_shell.recent_overworld_inputs.clear();
}

fn reset_visible_map_reload_after_battle(runtime_shell: &mut BevyRuntimeShell, reason: &str) {
    reset_visible_script_navigation_state(runtime_shell);
    reset_visible_selection_cursors(runtime_shell);
    runtime_shell.last_overworld_input = None;
    runtime_shell.overworld_direction_repeat_ticks = 0;
    runtime_shell.overworld_held_direction = None;
    runtime_shell.overworld_held_directions.clear();
    runtime_shell.overworld_buffered_direction = None;
    runtime_shell.ui_held_direction = None;
    runtime_shell.ui_direction_repeat_ticks = 0;
    runtime_shell.player_walk_frame_ticks = 0;
    runtime_shell.player_walk_total_ticks = WALK_FRAME_HOLD_TICKS;
    runtime_shell.player_walk_stride = false;
    runtime_shell.player_walk_mirror_stride = false;
    runtime_shell.object_walk_frame_ticks = 0;
    runtime_shell.object_walk_total_ticks = WALK_FRAME_HOLD_TICKS;
    runtime_shell.object_walk_stride = false;
    runtime_shell.object_walk_from.clear();
    runtime_shell.object_walk_frame_ticks_by_id.clear();
    runtime_shell.object_walk_total_ticks_by_id.clear();
    runtime_shell.object_walk_phases.clear();
    runtime_shell.object_walk_directions.clear();
    runtime_shell.pending_overworld_step_boundary = None;
    runtime_shell.pending_overworld_warp_scene = None;
    runtime_shell.visible_script_movement = None;
    runtime_shell.visible_script_movement_scene = None;
    runtime_shell.recent_overworld_inputs.clear();
    reset_visible_music_state(runtime_shell);
    runtime_shell
        .last_audio_events
        .push(format!("map reload after battle reason={reason}"));
    trim_event_log(&mut runtime_shell.last_audio_events);
}

fn reset_visible_deterministic_session_history(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell
        .shell
        .snapshot()
        .context("deterministic session reset requires a valid runtime snapshot")?;
    let checksum = snapshot.state_checksum;
    runtime_shell.deterministic_session_checkpoint = if snapshot.trainer.player_name.is_empty() {
        None
    } else {
        Some(
            visible_deterministic_session_checkpoint(&runtime_shell.shell, checksum.clone())
                .context("deterministic session reset requires a valid runtime checkpoint")?,
        )
    };
    runtime_shell.deterministic_session_start = checksum;
    runtime_shell.deterministic_input_frames.clear();
    runtime_shell.deterministic_battle_actions.clear();
    runtime_shell.deterministic_menu_results.clear();
    runtime_shell.shell.clear_retained_runtime_commands();
    Ok(())
}

fn reset_visible_script_navigation_state(runtime_shell: &mut BevyRuntimeShell) {
    runtime_shell.active_script_cursor = None;
    runtime_shell.pending_map_callbacks.clear();
    runtime_shell.map_callback_return_cursor = None;
    runtime_shell.map_reload_return_cursor = None;
    runtime_shell.pending_scene_script = None;
    runtime_shell.script_command_cursor = 0;
}

fn reset_visible_selection_cursors(runtime_shell: &mut BevyRuntimeShell) {
    runtime_shell.start_menu_cursor = None;
    runtime_shell.menu_cursor = None;
    runtime_shell.sell_cursor = None;
    runtime_shell.shop_top_cursor = Some(MenuCursor {
        surface_id: "shop:top".to_string(),
        option_index: 0,
    });
    runtime_shell.shop_quantity = None;
    runtime_shell.shop_notice = None;
    runtime_shell.shop_welcome_seen = false;
    runtime_shell.shop_return_to_top_after_notice = false;
    runtime_shell.shop_close_after_notice = false;
    runtime_shell.pending_pc_release = None;
    runtime_shell.bill_pc_pokemon_action_cursor = None;
    runtime_shell.bill_pc_box_summary = None;
    runtime_shell.pc_notice = None;
    runtime_shell.field_notice = None;
    runtime_shell.pending_tmhm_teach_prompt_after_boot = false;
    runtime_shell.field_notice_queue.clear();
    runtime_shell.pending_sweet_scent_nothing_notice = false;
    runtime_shell.field_notice_scene = None;
    runtime_shell.pending_field_travel_arrival = false;
    runtime_shell.pending_field_travel_delay_frames = None;
    runtime_shell.pending_field_notice_sound = None;
    runtime_shell.pending_field_notice_cry = None;
    runtime_shell.visible_strength_notice_phase = None;
    runtime_shell.pending_field_battle_entry = false;
    runtime_shell.pending_field_notice_effect_frames = None;
    runtime_shell.visible_sweet_scent_delay = false;
    runtime_shell.visible_cut_animation = None;
    runtime_shell.visible_whirlpool_animation = None;
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
    runtime_shell.elevator_cursor = None;
    runtime_shell.gift_pokemon_cursor = None;
    runtime_shell.yes_no_cursor = None;
    runtime_shell.pending_phone_prompt = None;
    runtime_shell.pending_day_of_week = None;
    runtime_shell.pending_trainer_sight = None;
    close_visible_party_detail_state(runtime_shell);
    runtime_shell.party_cursor = 0;
    runtime_shell.party_held_item_give_target = None;
    runtime_shell.held_item_swap_prompt = false;
    runtime_shell.pending_contextual_field_move = None;
    runtime_shell.pokedex_menu_open = false;
    runtime_shell.pokedex_detail_open = false;
    runtime_shell.pokedex_scripted_entry = false;
    runtime_shell.pokedex_cursor = 0;
    runtime_shell.pokegear_menu_open = false;
    runtime_shell.pokegear_cursor = 0;
    runtime_shell.pokegear_phone_cursor = 0;
    runtime_shell.pokegear_phone_status = None;
    runtime_shell.pokegear_page = PokegearPage::Clock;
    runtime_shell.pokegear_radio_station = None;
    runtime_shell.pokegear_radio_segment = 0;
    runtime_shell.trainer_card_open = false;
    runtime_shell.trainer_card_page = VisibleTrainerCardPage::Info;
    runtime_shell.trainer_card_colon_visible = false;
    runtime_shell.trainer_card_colon_ticks = 0;
    runtime_shell.trainer_card_badge_frame = 0;
    runtime_shell.trainer_card_badge_ticks = 0;
    runtime_shell.options_menu_open = false;
    runtime_shell.options_cursor = 0;
    runtime_shell.save_menu_open = false;
    runtime_shell.save_flow = None;
    runtime_shell.special_boundary = None;
    runtime_shell.special_boundary_queue.clear();
    runtime_shell.pending_special_cry = None;
    runtime_shell.pending_special_sound = None;
    runtime_shell.field_pack_pocket = None;
    runtime_shell.pack_item_switch_origin = None;
    runtime_shell.field_pack_action_cursor = None;
    runtime_shell.field_pack_target_mode = None;
    runtime_shell.battle_pack_target_mode = None;
    runtime_shell.bag_cursor = None;
    runtime_shell.key_item_cursor = None;
    runtime_shell.ball_cursor = None;
    runtime_shell.tmhm_cursor = None;
    runtime_shell.custom_item_cursor = None;
    runtime_shell.storage_cursor = None;
    runtime_shell.pc_item_cursor = None;
    runtime_shell.pc_hub_session_open = false;
    runtime_shell.pc_hub_cursor = None;
    runtime_shell.bill_pc_session_open = false;
    runtime_shell.bill_pc_action_cursor = None;
    runtime_shell.bill_pc_box_cursor = None;
    runtime_shell.bill_pc_move_open = false;
    runtime_shell.bill_pc_move_party_open = false;
    runtime_shell.bill_pc_move_source = None;
    runtime_shell.fly_cursor = None;
    reset_visible_battle_action_cursors(runtime_shell);
}

fn reset_visible_battle_action_cursors(runtime_shell: &mut BevyRuntimeShell) {
    reset_visible_battle_item_cursors(runtime_shell);
    runtime_shell.battle_action_cursor = None;
    runtime_shell.battle_move_cursor = None;
    runtime_shell.battle_move_swap_origin = None;
    runtime_shell.battle_shift_prompt_cursor = None;
    runtime_shell.battle_faint_prompt_cursor = None;
    runtime_shell.battle_switch_cursor = None;
    runtime_shell.battle_party_action_cursor = None;
    runtime_shell.battle_party_summary_open = false;
    runtime_shell.pending_battle_move_switch_slot = None;
}

fn reset_visible_battle_exit_state(runtime_shell: &mut BevyRuntimeShell) {
    reset_visible_battle_action_cursors(runtime_shell);
    runtime_shell.party_move_cursor = None;
    runtime_shell.last_battle_cry_key = None;
    runtime_shell.pending_battle_cries_after_messages.clear();
    runtime_shell.battle_enemy_send_out_pending = false;
    runtime_shell.battle_player_send_out_pending = false;
    runtime_shell.battle_enemy_hp_at_player_send_out = None;
    runtime_shell.pending_battle_scenes_after_message.clear();
    runtime_shell.pending_enemy_response_after_capture = None;
    runtime_shell.visible_move_animations.clear();
    runtime_shell.visible_send_out_animation = None;
    runtime_shell.visible_trainer_exit_animation = None;
    runtime_shell.visible_frontpic_animation = None;
    if runtime_shell.battle_messages.is_empty() {
        runtime_shell.battle_text_reveal = None;
        reset_visible_music_state(runtime_shell);
    }
}
