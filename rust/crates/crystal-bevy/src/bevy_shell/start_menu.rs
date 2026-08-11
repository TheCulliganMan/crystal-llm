fn visible_start_menu_entries(runtime_shell: &BevyRuntimeShell) -> Result<Vec<String>> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let options = visible_start_menu_options(runtime_shell, &snapshot);
    let selected = strict_readonly_cursor_index(
        &runtime_shell.start_menu_cursor,
        START_MENU_SURFACE_ID,
        options.len(),
    )
    .context("start menu is open without a valid cursor")?;
    Ok(options
        .iter()
        .enumerate()
        .map(|(index, option)| {
            let marker = if index == selected { ">" } else { " " };
            format!(
                "{marker}{}",
                start_menu_option_display_label(*option, &snapshot)
            )
        })
        .collect())
}

fn visible_field_pack_entries(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
) -> Vec<String> {
    if let Some(cursor) = &runtime_shell.tmhm_decision_prompt_cursor {
        let Some(selected) = strict_readonly_cursor_index(
            &Some(cursor.clone()),
            "pack:tmhm:decision",
            2,
        ) else {
            return vec![compact_scene_label("INVALID CURSOR pack:tmhm:decision", 30)];
        };
        let Some(party) = snapshot
            .party
            .slots
            .get(runtime_shell.party_cursor)
            .map(|slot| slot.pokemon.nickname.as_str())
        else {
            return vec![compact_scene_label(
                "INVALID TM/HM PARTY SELECTION",
                30,
            )];
        };
        let prompt = match runtime_shell.tmhm_decision {
            Some(VisibleTmHmDecision::ForgetMove) => {
                format!("DELETE A MOVE FOR {party}?")
            }
            Some(VisibleTmHmDecision::StopLearning) => "STOP LEARNING THIS MOVE?".to_string(),
            None => "INVALID TM/HM DECISION".to_string(),
        };
        return vec![
            compact_scene_label(&prompt, 30),
            format!("{}YES", if selected == 0 { ">" } else { " " }),
            format!("{}NO", if selected == 1 { ">" } else { " " }),
        ];
    }
    if let Some(cursor) = &runtime_shell.tmhm_teach_prompt_cursor {
        let Some(selected) = strict_readonly_cursor_index(
            &Some(cursor.clone()),
            "pack:tmhm:teach-prompt",
            2,
        ) else {
            return vec![compact_scene_label("INVALID CURSOR pack:tmhm:teach", 30)];
        };
        let active = FieldPackPocket::TmHm;
        let item = selected_field_pack_item_label(snapshot, runtime_shell, &active)
            .unwrap_or_else(|| "TM/HM".to_string());
        let move_id = runtime_shell
            .tmhm_cursor
            .as_ref()
            .and_then(|cursor| {
                strict_readonly_cursor_index(
                    &Some(cursor.clone()),
                    "bag:tmhm",
                    field_pack_selectable_count(snapshot.bag.tm_hm.len()),
                )
            })
            .and_then(|index| snapshot.bag.tm_hm.get(index))
            .and_then(|tmhm| tmhm.move_id.as_deref())
            .unwrap_or(&item);
        let move_name = battle_move_display_name(snapshot, move_id);
        return vec![
            compact_scene_label(&format!("TEACH {move_name}?"), 30),
            format!("{}YES", if selected == 0 { ">" } else { " " }),
            format!("{}NO", if selected == 1 { ">" } else { " " }),
        ];
    }
    if let Some(mode) = runtime_shell.field_pack_target_mode {
        return visible_field_pack_target_entries(snapshot, runtime_shell, mode);
    }
    let active = active_visible_field_pack_pocket(runtime_shell);
    if runtime_shell.field_pack_action_cursor.is_some() {
        return visible_field_pack_action_entries(snapshot, runtime_shell, &active)
            .into_iter()
            .take(SCENE_MENU_VISIBLE_ROWS)
            .collect();
    }
    let mut entries = vec![format!(
        "POCKET: {}",
        field_pack_pocket_label(&active).to_uppercase()
    )];
    entries.extend(match active {
        FieldPackPocket::Items => selected_pack_entries(
            snapshot,
            &snapshot.bag.items,
            &runtime_shell.bag_cursor,
            "bag:items",
        ),
        FieldPackPocket::Balls => selected_pack_entries(
            snapshot,
            &snapshot.bag.balls,
            &runtime_shell.ball_cursor,
            "bag:balls",
        ),
        FieldPackPocket::KeyItems => selected_pack_entries(
            snapshot,
            &snapshot.bag.key_items,
            &runtime_shell.key_item_cursor,
            "bag:key-items",
        ),
        FieldPackPocket::TmHm => selected_tmhm_pack_entries(snapshot, runtime_shell),
        FieldPackPocket::Custom(pocket_id) => {
            let Some(items) = snapshot.bag.custom_pockets.get(&pocket_id) else {
                return vec![compact_scene_label(
                    &format!("INVALID POCKET {pocket_id}"),
                    30,
                )];
            };
            selected_pack_entries(
                snapshot,
                items,
                &runtime_shell.custom_item_cursor,
                &custom_pack_surface_id(&pocket_id),
            )
        }
    });
    entries.into_iter().take(SCENE_MENU_VISIBLE_ROWS).collect()
}

fn visible_field_pack_action_entries(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    pocket: &FieldPackPocket,
) -> Vec<String> {
    let actions = match visible_selected_pack_item_actions(
        snapshot,
        runtime_shell,
        pocket,
        snapshot.battle.is_some(),
    ) {
        Ok(actions) => actions,
        Err(error) => return vec![compact_scene_label(&format!("INVALID ACTIONS {error}"), 30)],
    };
    let Some(selected) = strict_readonly_cursor_index(
        &runtime_shell.field_pack_action_cursor,
        "pack:actions",
        actions.len(),
    ) else {
        return vec![compact_scene_label("INVALID CURSOR pack:actions", 30)];
    };
    let mut entries = selected_field_pack_item_label(snapshot, runtime_shell, pocket)
        .map(|label| vec![compact_scene_label(&format!("ACTION {label}"), 30)])
        .unwrap_or_else(|| vec![compact_scene_label("INVALID CURSOR field:item", 30)]);
    entries.extend(actions.iter().enumerate().map(|(index, action)| {
        let marker = if index == selected { ">" } else { " " };
        format!("{marker}{}", visible_field_pack_action_label(*action))
    }));
    entries
}

fn visible_field_pack_target_entries(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    mode: FieldPackTargetMode,
) -> Vec<String> {
    let selected_party = runtime_shell
        .party_cursor
        .min(snapshot.party.slots.len().saturating_sub(1));
    let mut entries = selected_field_pack_item_label(
        snapshot,
        runtime_shell,
        &active_visible_field_pack_pocket(runtime_shell),
    )
    .map(|label| vec![compact_scene_label(&format!("ITEM {label}"), 30)])
    .unwrap_or_else(|| vec![compact_scene_label("INVALID CURSOR field:item", 30)]);
    if mode == FieldPackTargetMode::PartyMove {
        let Some(slot) = snapshot.party.slots.get(selected_party) else {
            return vec![compact_scene_label(
                &format!("INVALID PARTY SLOT {selected_party}"),
                30,
            )];
        };
        entries.push(party_slot_entry(snapshot, slot, true));
        let selected_move = strict_readonly_cursor_index(
            &runtime_shell.party_move_cursor,
            &party_move_cursor_surface_id(slot.index),
            slot.pokemon.moves.len(),
        );
        let Some(selected_move) = selected_move else {
            entries.push(compact_scene_label(
                &format!("INVALID CURSOR party:{}:moves", slot.index),
                30,
            ));
            return entries.into_iter().take(SCENE_MENU_VISIBLE_ROWS).collect();
        };
        entries.extend(windowed_move_entries(
            snapshot,
            &slot.pokemon.moves,
            selected_move,
        ));
        return entries.into_iter().take(SCENE_MENU_VISIBLE_ROWS).collect();
    }
    if mode == FieldPackTargetMode::TmHmPokemon && runtime_shell.tmhm_forget_menu_open {
        let Some(slot) = snapshot.party.slots.get(selected_party) else {
            return vec![compact_scene_label(
                &format!("INVALID PARTY SLOT {selected_party}"),
                30,
            )];
        };
        let row_count = slot.pokemon.moves.len();
        let Some(selected_move) = strict_readonly_cursor_index(
            &runtime_shell.party_move_cursor,
            &party_move_cursor_surface_id(slot.index),
            row_count,
        ) else {
            entries.push(compact_scene_label(
                &format!("INVALID CURSOR party:{}:moves", slot.index),
                30,
            ));
            return entries.into_iter().take(SCENE_MENU_VISIBLE_ROWS).collect();
        };
        entries.push(compact_scene_label("CHOOSE A MOVE TO FORGET", 30));
        entries.extend(windowed_index_range(selected_move, row_count).map(|index| {
            if index == slot.pokemon.moves.len() {
                format!("{}CANCEL", if index == selected_move { ">" } else { " " })
            } else {
                let marker = if index == selected_move { ">" } else { " " };
                move_menu_entry(snapshot, &slot.pokemon.moves[index], marker)
            }
        }));
        return entries.into_iter().take(SCENE_MENU_VISIBLE_ROWS).collect();
    }
    if mode == FieldPackTargetMode::TmHmPokemon {
        let item_id = runtime_shell
            .tmhm_cursor
            .as_ref()
            .and_then(|cursor| {
                strict_readonly_cursor_index(
                    &Some(cursor.clone()),
                    "bag:tmhm",
                    field_pack_selectable_count(snapshot.bag.tm_hm.len()),
                )
            })
            .and_then(|index| snapshot.bag.tm_hm.get(index))
            .map(|tmhm| tmhm.item_id.as_str());
        entries.extend(windowed_index_range(selected_party, snapshot.party.slots.len()).map(
            |index| {
                let slot = &snapshot.party.slots[index];
                let is_egg = slot.pokemon.is_egg
                    || slot.pokemon.species.id == "EGG";
                let able = !is_egg
                    && item_id.is_some_and(|item_id| {
                        match runtime_shell.shell.preview_tmhm_on_party_pokemon(
                            item_id,
                            slot.index,
                            None,
                        ) {
                            Ok(_) => true,
                            Err(error) => matches!(
                                error.downcast_ref::<TmHmLearnError>(),
                                Some(
                                    TmHmLearnError::MoveListFull
                                        | TmHmLearnError::AlreadyKnows { .. }
                                )
                            ),
                        }
                    });
                compact_scene_label(
                    &format!(
                        "{} {}",
                        party_slot_entry(snapshot, slot, index == selected_party),
                        if able { "ABLE" } else { "NOT ABLE" }
                    ),
                    30,
                )
            },
        ));
        return entries.into_iter().take(SCENE_MENU_VISIBLE_ROWS).collect();
    }
    if mode == FieldPackTargetMode::PartyPokemon {
        let item_id = selected_field_pack_item_id_from_snapshot(
            snapshot,
            runtime_shell,
            &active_visible_field_pack_pocket(runtime_shell),
        );
        let is_evolution_item = item_id.as_deref().is_some_and(|item_id| {
            snapshot.item_effect_plans.iter().any(|plan| {
                plan.item_id == item_id
                    && plan.behavior_id == ITEM_EFFECT_BEHAVIOR_EVOLUTION_STONE
            })
        });
        if is_evolution_item {
            entries.extend(windowed_index_range(selected_party, snapshot.party.slots.len()).map(
                |index| {
                    let slot = &snapshot.party.slots[index];
                    let able = item_id.as_deref().is_some_and(|item_id| {
                        runtime_shell
                            .shell
                            .preview_party_item_on_pokemon(item_id, slot.index)
                            .is_ok_and(|outcome| outcome.evolution_target.is_some())
                    });
                    compact_scene_label(
                        &format!(
                            "{} {}",
                            party_slot_entry(snapshot, slot, index == selected_party),
                            if able { "ABLE" } else { "NOT ABLE" }
                        ),
                        30,
                    )
                },
            ));
            return entries.into_iter().take(SCENE_MENU_VISIBLE_ROWS).collect();
        }
    }
    entries.extend(windowed_party_slot_entries(snapshot, selected_party));
    entries.into_iter().take(SCENE_MENU_VISIBLE_ROWS).collect()
}

fn visible_party_menu_entries(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
) -> Vec<String> {
    let selected_party_slot = runtime_shell
        .party_cursor
        .min(snapshot.party.slots.len().saturating_sub(1));
    if runtime_shell.party_move_reorder_open {
        let Some(slot) = snapshot.party.slots.get(selected_party_slot) else {
            return vec![compact_scene_label("INVALID PARTY", 30)];
        };
        let row_count = slot.pokemon.moves.len() + 1;
        let Some(selected) = strict_readonly_cursor_index(
            &runtime_shell.party_move_cursor,
            &party_move_reorder_surface_id(slot.index),
            row_count,
        ) else {
            return vec![compact_scene_label("INVALID CURSOR party:move-reorder", 30)];
        };
        let mut entries = vec![compact_scene_label(
            &format!(
                "{} \u{e10a}{} MOVE",
                slot.pokemon.nickname, slot.pokemon.level
            ),
            30,
        )];
        entries.extend(windowed_index_range(selected, row_count).map(|index| {
            let marker = if runtime_shell.party_move_reorder_origin == Some(index) {
                "\u{25b7}"
            } else if index == selected {
                ">"
            } else {
                " "
            };
            move_menu_entry(snapshot, &slot.pokemon.moves[index], marker)
        }));
        return entries.into_iter().take(SCENE_MENU_VISIBLE_ROWS).collect();
    }
    if let Some(give_take_cursor) = &runtime_shell.party_give_take_cursor {
        if give_take_cursor.surface_id == "party:mail-actions" {
            let Some(selected) = strict_readonly_cursor_index(
                &Some(give_take_cursor.clone()),
                "party:mail-actions",
                3,
            ) else {
                return vec![compact_scene_label("INVALID CURSOR party:mail-actions", 30)];
            };
            let mut entries = vec![party_slot_entry(snapshot, &snapshot.party.slots[selected_party_slot], true)];
            entries.extend(["READ", "TAKE", "QUIT"].iter().enumerate().map(|(index, label)| {
                format!("{}{label}", if selected == index { ">" } else { " " })
            }));
            return entries;
        }
        let Some(selected) = strict_readonly_cursor_index(
            &Some(give_take_cursor.clone()),
            "party:give-take",
            2,
        ) else {
            return vec![compact_scene_label("INVALID CURSOR party:give-take", 30)];
        };
        let Some(slot) = snapshot.party.slots.get(selected_party_slot) else {
            return vec![compact_scene_label("INVALID PARTY", 30)];
        };
        return vec![
            party_slot_entry(snapshot, slot, true),
            format!("{}GIVE", if selected == 0 { ">" } else { " " }),
            format!("{}TAKE", if selected == 1 { ">" } else { " " }),
        ];
    }
    if let Some(switch_cursor) = &runtime_shell.party_switch_cursor {
        let Some(source_slot) = snapshot.party.slots.get(selected_party_slot) else {
            return vec![compact_scene_label(
                &format!("INVALID PARTY SLOT {selected_party_slot}"),
                30,
            )];
        };
        let selected_target = strict_readonly_cursor_index(
            &Some(switch_cursor.clone()),
            &party_switch_cursor_surface_id(source_slot.index),
            snapshot.party.slots.len(),
        );
        let Some(selected_target) = selected_target else {
            return vec![compact_scene_label("INVALID CURSOR party:switch", 30)];
        };
        return windowed_index_range(selected_target, snapshot.party.slots.len())
            .map(|index| {
                let slot = &snapshot.party.slots[index];
                party_switch_slot_entry(
                    snapshot,
                    slot,
                    index == selected_target,
                    index == selected_party_slot,
                )
            })
            .collect();
    }
    if runtime_shell.party_summary_open {
        return visible_party_summary_entries(snapshot, runtime_shell);
    }
    if let Some(action_cursor) = &runtime_shell.party_action_cursor {
        let actions = match visible_party_actions(snapshot, runtime_shell) {
            Ok(actions) => actions,
            Err(error) => {
                return vec![compact_scene_label(
                    &format!("INVALID PARTY ACTIONS {error:#}"),
                    30,
                )];
            }
        };
        let selected_action = strict_readonly_cursor_index(
            &Some(action_cursor.clone()),
            "party:actions",
            actions.len(),
        );
        let Some(selected_action) = selected_action else {
            return vec![compact_scene_label("INVALID CURSOR party:actions", 30)];
        };
        let row_count = normal_visible_party_menu_row_count(snapshot);
        let selected_row = runtime_shell.party_cursor.min(row_count.saturating_sub(1));
        let mut entries = windowed_index_range(selected_row, row_count)
            .map(|index| {
                if index >= snapshot.party.slots.len() {
                    party_cancel_entry(index == selected_row)
                } else {
                    let slot = &snapshot.party.slots[index];
                    party_slot_entry(snapshot, slot, index == selected_row)
                }
            })
            .collect::<Vec<_>>();
        entries.push("SUBMENU:".to_string());
        let visible_action_rows = SCENE_MENU_VISIBLE_ROWS.saturating_sub(entries.len());
        let action_window_start = visible_window_start(
            selected_action,
            actions.len(),
            visible_action_rows,
        );
        let action_window_end = (action_window_start + visible_action_rows).min(actions.len());
        entries.extend(
            (action_window_start..action_window_end).map(|index| {
                let action = actions[index];
                let marker = if index == selected_action { ">" } else { " " };
                party_submenu_action_entry(action, marker)
            }),
        );
        return entries.into_iter().take(SCENE_MENU_VISIBLE_ROWS).collect();
    }
    if runtime_shell.fly_cursor.is_some() {
        return visible_fly_destination_entries(snapshot, runtime_shell);
    }
    let row_count = normal_visible_party_menu_row_count(snapshot);
    let selected_row = runtime_shell.party_cursor.min(row_count.saturating_sub(1));
    windowed_index_range(selected_row, row_count)
        .map(|index| {
            if index >= snapshot.party.slots.len() {
                party_cancel_entry(index == selected_row)
            } else {
                let slot = &snapshot.party.slots[index];
                party_slot_entry(snapshot, slot, index == selected_row)
            }
        })
        .collect()
}

fn visible_party_summary_entries(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
) -> Vec<String> {
    let selected = runtime_shell
        .party_cursor
        .min(snapshot.party.slots.len().saturating_sub(1));
    let Some(slot) = snapshot.party.slots.get(selected) else {
        return Vec::new();
    };
    let pokemon = &slot.pokemon;
    let status = party_status_token(pokemon);
    let held = pokemon
        .item
        .as_deref()
        .map(|item| item_display_name(snapshot, item))
        .unwrap_or_else(|| "-".to_string());
    let mut entries = vec![
        compact_scene_label(
            &format!(
                "SUMMARY {}/{} {}",
                selected + 1,
                snapshot.party.slots.len(),
                pokemon.nickname
            ),
            30,
        ),
        compact_scene_label(
            &format!(
                "{} \u{e10a}{} HP {}/{} {status}",
                crate::core::models::pokemon_species_display_name(&pokemon.species.id),
                pokemon.level,
                pokemon.hp,
                pokemon.max_hp
            ),
            30,
        ),
        compact_scene_label(
            &format!(
                "ATK {} DEF {} SPD {}",
                pokemon.attack, pokemon.defense, pokemon.speed
            ),
            30,
        ),
        compact_scene_label(
            &format!(
                "SAT {} SDF {} ITEM {held}",
                pokemon.special_attack, pokemon.special_defense
            ),
            30,
        ),
        compact_scene_label(
            &format!(
                "EXP {} HAP {} OT {}#{}",
                pokemon.experience,
                pokemon.happiness,
                pokemon.original_trainer_name,
                pokemon.original_trainer_id
            ),
            30,
        ),
    ];
    entries.push(format!("PAGE {}/3", runtime_shell.party_summary_page.clamp(1, 3)));
    if pokemon.moves.is_empty() {
        entries.push("NO MOVES".to_string());
    } else {
        entries.extend(
            pokemon
                .moves
                .iter()
                .take(SCENE_MENU_VISIBLE_ROWS.saturating_sub(entries.len()))
                .map(|learned| move_menu_entry(snapshot, learned, " ")),
        );
    }
    entries.into_iter().take(SCENE_MENU_VISIBLE_ROWS).collect()
}

fn visible_fly_destination_entries(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
) -> Vec<String> {
    let destinations = active_fly_destinations(snapshot, &runtime_shell.shell);
    let selected = strict_readonly_cursor_index(
        &runtime_shell.fly_cursor,
        "fly:destinations",
        destinations.len(),
    );
    let Some(selected) = selected else {
        return vec![compact_scene_label("INVALID CURSOR fly:destinations", 30)];
    };
    destinations
        .iter()
        .enumerate()
        .skip(visible_window_start(
            selected,
            destinations.len(),
            SCENE_MENU_VISIBLE_ROWS,
        ))
        .take(SCENE_MENU_VISIBLE_ROWS)
        .map(|(index, destination)| {
            let marker = if index == selected { ">" } else { " " };
            let label = fly_destination_label(destination);
            compact_scene_label(&format!("{marker}{label}"), SCENE_DIALOG_TEXT_CHARS)
        })
        .collect()
}

fn visible_pokedex_menu_entries(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
) -> Vec<String> {
    if snapshot.pokemon.is_empty() {
        return Vec::new();
    }
    if runtime_shell.pokedex_detail_open {
        return visible_pokedex_detail_entries(snapshot, runtime_shell);
    }
    let selected = runtime_shell
        .pokedex_cursor
        .min(snapshot.pokemon.len().saturating_sub(1));
    windowed_index_range(selected, snapshot.pokemon.len())
        .map(|index| {
            let species = &snapshot.pokemon[index];
            let marker = if index == selected { ">" } else { " " };
            pokedex_entry_row(snapshot, species, marker)
        })
        .collect()
}

fn visible_pokedex_detail_entries(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
) -> Vec<String> {
    let Ok(species) = selected_pokedex_catalog_species(snapshot, runtime_shell.pokedex_cursor)
    else {
        return vec![compact_scene_label("INVALID POKEDEX", 30)];
    };
    let Some(entry) = snapshot
        .presentation
        .pokedex_entries
        .get(&species.species_id)
    else {
        return vec![compact_scene_label(
            &format!("INVALID DEX {}", species.species_id),
            30,
        )];
    };
    let mut entries = vec![
        compact_scene_label(
            &format!(
                "#{:03} {} {}",
                species.int_id, species.species_id, entry.classification
            ),
            30,
        ),
        compact_scene_label(
            &format!(
                "HT {} WT {} TYPE {}/{}",
                entry.height_digits, entry.weight_digits, species.type1, species.type2
            ),
            30,
        ),
        compact_scene_label(
            &format!(
                "CATCH {} EXP {} GROW {}",
                species.catch_rate, species.base_exp, species.growth_rate
            ),
            30,
        ),
        compact_scene_label(
            &format!(
                "EGG {}/{} AB {}",
                species.egg_group1, species.egg_group2, species.ability
            ),
            30,
        ),
    ];
    entries.extend(
        entry
            .pages
            .iter()
            .flat_map(|page| wrap_scene_dialog_line(page, SCENE_DIALOG_TEXT_CHARS))
            .take(SCENE_MENU_VISIBLE_ROWS.saturating_sub(entries.len())),
    );
    entries.into_iter().take(SCENE_MENU_VISIBLE_ROWS).collect()
}

fn visible_pokegear_menu_entries(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
) -> Vec<String> {
    if runtime_shell.pokegear_page == PokegearPage::Clock {
        let time = &snapshot.progression.time;
        const DAY_NAMES: [&str; 7] = ["SUN", "MON", "TUES", "WED", "THURS", "FRI", "SAT"];
        let day = DAY_NAMES[usize::from(time.day_of_week % 7)];
        let time_period = match time.time_of_day {
            crate::core::world::encounters::TimeOfDay::Morning => "MORN",
            crate::core::world::encounters::TimeOfDay::Day => "DAY",
            crate::core::world::encounters::TimeOfDay::Night => "NITE",
        };
        let hour_24 = time.registers.hours;
        let hour_12 = match hour_24 % 12 {
            0 => 12,
            hour => hour,
        };
        let meridiem = if hour_24 < 12 { "AM" } else { "PM" };
        return vec![
            format!("{day} {time_period}"),
            format!("{hour_12:>2}:{:02}{meridiem}", time.registers.minutes),
        ];
    }
    if runtime_shell.pokegear_page == PokegearPage::Radio {
        let Some(station) = runtime_shell.pokegear_radio_station.as_deref() else {
            let index = runtime_shell
                .pokegear_radio_index
                .min(VISIBLE_POKEGEAR_RADIO_FREQUENCIES.len() - 1);
            let (frequency, handler) = VISIBLE_POKEGEAR_RADIO_FREQUENCIES[index];
            return vec![
                format!("RADIO  {:.1}", frequency),
                visible_pokegear_radio_handler_name(snapshot, handler).to_string(),
                "UP/DOWN TUNE".to_string(),
                "LEFT/RIGHT CARD".to_string(),
            ];
        };
        let transcript = visible_map_radio_transcript(station);
        let segment = runtime_shell
            .pokegear_radio_segment
            .min(transcript.len().saturating_sub(1));
        let mut entries = vec![compact_scene_label(
            &format!("RADIO  {}", visible_map_radio_station_name(station)),
            30,
        )];
        if let Some(label) = transcript.get(segment) {
            if let Some(text) = snapshot.presentation.asm_text.get(*label) {
                entries.extend(
                    normalize_visible_script_text_with_context(
                        text,
                        &snapshot.trainer.player_name,
                        visible_rival_name(snapshot),
                        snapshot.progression.time.day_of_week,
                    )
                        .lines()
                        .flat_map(|line| wrap_scene_dialog_line(line, SCENE_DIALOG_TEXT_CHARS)),
                );
            } else {
                entries.push(compact_scene_label(label, 30));
            }
        }
        return entries.into_iter().take(SCENE_MENU_VISIBLE_ROWS).collect();
    }
    if runtime_shell.pokegear_page == PokegearPage::Phone {
        return visible_pokegear_phone_entries(snapshot, runtime_shell);
    }
    let region_indices = visible_pokegear_landmark_indices(snapshot);
    if region_indices.is_empty() {
        return Vec::new();
    }
    Vec::new()
}

fn visible_pokegear_radio_handler_name(
    snapshot: &RuntimeShellSnapshot,
    handler: &str,
) -> &'static str {
    match visible_pokegear_radio_station(snapshot, handler).map(|(constant, _)| constant) {
        Some("OAKS_POKEMON_TALK") => "OAK'S POKEMON TALK",
        Some("POKEDEX_SHOW") => "POKEDEX SHOW",
        Some("POKEMON_MUSIC") => "POKEMON MUSIC",
        Some("LUCKY_CHANNEL") => "LUCKY CHANNEL",
        Some("BUENAS_PASSWORD") => "BUENA'S PASSWORD",
        Some("UNOWN_RADIO") => "?????",
        Some("PLACES_AND_PEOPLE") => "PLACES & PEOPLE",
        Some("LETS_ALL_SING") => "LET'S ALL SING",
        Some("POKE_FLUTE_RADIO") => "POKE FLUTE",
        Some("EVOLUTION_RADIO") => "EVOLUTION RADIO",
        Some("ROCKET_RADIO") => "ROCKET RADIO",
        _ => "NO SIGNAL",
    }
}

fn visible_map_radio_transcript(station: &str) -> &'static [&'static str] {
    const POKEMON_CHANNEL: &[&str] = &[
        "PlayersRadioText1",
        "PlayersRadioText2",
        "PlayersRadioText3",
        "PlayersRadioText4",
    ];
    const LUCKY_CHANNEL: &[&str] = &[
        "LC_Text1", "LC_Text2", "LC_Text3", "LC_Text4", "LC_Text5", "LC_Text6",
        "LC_Text7", "LC_Text8", "LC_Text9", "LC_Text7", "LC_Text8", "LC_Text10",
        "LC_Text11",
    ];
    match station {
        "MAPRADIO_LUCKY_CHANNEL" | "LUCKY_CHANNEL" => LUCKY_CHANNEL,
        _ => POKEMON_CHANNEL,
    }
}

fn visible_map_radio_station_name(station: &str) -> String {
    match station {
        "MAPRADIO_POKEMON_CHANNEL" | "OAKS_POKEMON_TALK" | "POKEDEX_SHOW" => {
            "OAK'S POKEMON TALK".to_string()
        }
        "MAPRADIO_LUCKY_CHANNEL" | "LUCKY_CHANNEL" => "LUCKY CHANNEL".to_string(),
        "MAPRADIO_POKEMON_MUSIC" | "POKEMON_MUSIC" => "POKEMON MUSIC".to_string(),
        "MAPRADIO_PLACES_PEOPLE" | "PLACES_AND_PEOPLE" => "PLACES & PEOPLE".to_string(),
        "MAPRADIO_LETS_ALL_SING" | "LETS_ALL_SING" => "LET'S ALL SING".to_string(),
        "MAPRADIO_ROCKET" | "ROCKET_RADIO" => "ROCKET RADIO".to_string(),
        "MAPRADIO_UNOWN" => "MYSTERIOUS BROADCAST".to_string(),
        other => other.replace('_', " "),
    }
}

fn visible_pokegear_phone_entries(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
) -> Vec<String> {
    let contact_ids = visible_pokegear_phone_contact_ids(snapshot);
    if contact_ids.is_empty() {
        return vec!["PHONE EMPTY".to_string()];
    }
    let selected = runtime_shell
        .pokegear_phone_cursor
        .min(contact_ids.len().saturating_sub(1));
    let mut entries = Vec::new();
    let visible_contacts = 3;
    let start = visible_window_start(selected, contact_ids.len(), visible_contacts);
    for (index, contact_id) in contact_ids
        .iter()
        .enumerate()
        .skip(start)
        .take(visible_contacts)
    {
        let marker = if index == selected { ">" } else { " " };
        let lines = snapshot
            .special
            .phone_contacts
            .0
            .get(contact_id)
            .map(|contact| contact.lines.as_slice())
            .unwrap_or(&[]);
        let primary = lines.first().map(String::as_str).unwrap_or(contact_id);
        entries.push(compact_scene_label(
            &format!("{marker}{primary}"),
            SCENE_DIALOG_TEXT_CHARS,
        ));
        if let Some(class) = lines.get(1) {
            entries.push(compact_scene_label(
                &format!("    {class}"),
                SCENE_DIALOG_TEXT_CHARS,
            ));
        }
    }
    let map_has_no_service = snapshot
        .maps
        .iter()
        .find(|map| map.map_name == snapshot.overworld.map_name)
        .and_then(|map| map.metadata.as_ref())
        .is_some_and(|metadata| ((metadata.phone_service & 0xf0) >> 4) != 0);
    if let Some(status) = runtime_shell
        .pokegear_phone_status
        .as_deref()
        .or(map_has_no_service.then_some("NO SERVICE"))
    {
        entries.push(compact_scene_label(status, SCENE_DIALOG_TEXT_CHARS));
    }
    entries.into_iter().take(10).collect()
}

fn visible_pokegear_phone_contact_ids(snapshot: &RuntimeShellSnapshot) -> Vec<String> {
    snapshot
        .script_events
        .phone_numbers
        .iter()
        .cloned()
        .collect()
}

fn selected_visible_pokegear_phone_contact_id(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
) -> Result<String> {
    let contact_ids = visible_pokegear_phone_contact_ids(snapshot);
    if contact_ids.is_empty() {
        anyhow::bail!("Pokegear has no registered phone contacts");
    }
    contact_ids
        .get(runtime_shell.pokegear_phone_cursor)
        .cloned()
        .with_context(|| {
            format!(
                "Pokegear phone cursor {} is out of range for {} contacts",
                runtime_shell.pokegear_phone_cursor,
                contact_ids.len()
            )
        })
}

fn visible_options_menu_entries(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
) -> Vec<String> {
    let selected = runtime_shell
        .options_cursor
        .min(OPTIONS_MENU_ITEMS.len().saturating_sub(1));
    windowed_index_range(selected, OPTIONS_MENU_ITEMS.len())
        .map(|index| {
            let item = OPTIONS_MENU_ITEMS[index];
            let marker = if index == selected { ">" } else { " " };
            let value = option_value_for_item(&snapshot.trainer.options, item);
            if value.is_empty() {
                format!("{marker}{}", options_menu_item_label(item))
            } else {
                format!("{marker}{}: {value}", options_menu_item_label(item))
            }
        })
        .collect()
}

fn visible_trainer_card_entries(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
) -> Vec<String> {
    if runtime_shell.trainer_card_page == VisibleTrainerCardPage::JohtoBadges {
        const JOHTO_BADGE_NAMES: [&str; 8] = [
            "ZEPHYR", "HIVE", "PLAIN", "FOG", "MINERAL", "STORM", "GLACIER", "RISING",
        ];
        let mut entries = vec!["BADGES".to_string()];
        entries.extend(
            JOHTO_BADGE_NAMES
                .iter()
                .zip(snapshot.progression.badges.johto.iter())
                .filter(|(_, owned)| **owned)
                .map(|(name, _)| (*name).to_string()),
        );
        return entries;
    }
    let mut entries = vec![
        compact_scene_label(&format!("NAME/ {}", snapshot.trainer.player_name), 30),
        format!("ID {:05}", snapshot.trainer.player_id),
        format!(
            "MONEY {}",
            format_trainer_card_money(snapshot.trainer.money)
        ),
    ];
    if snapshot
        .progression
        .active_engine_flags
        .contains(ENGINE_POKEDEX_FLAG)
    {
        entries.push(format!(
            "#DEX {:>3}",
            snapshot.progression.pokedex_owned.min(999)
        ));
    }
    entries.extend([
        compact_scene_label(&format!("TIME {:?}", snapshot.progression.time), 30),
        format!(
            "BADGES {}",
            visible_badge_count(&snapshot.progression.badges)
        ),
    ]);
    entries
}

fn visible_save_slot_preview_entries_for_path(
    runtime_shell: &BevyRuntimeShell,
    path: &std::path::Path,
) -> Vec<String> {
    match runtime_shell.shell.runtime().load_save_summary(path) {
        Ok(summary) => vec![
            compact_scene_label(
                &format!(
                    "EXISTS F{} {} {}",
                    summary.saved_frame(),
                    summary.modpack().id(),
                    summary.pack_content_hash()
                ),
                30,
            ),
            compact_scene_label(&format!("SAVE VERSION {}", summary.format_version()), 30),
            compact_scene_label(&format!("SAVE PACK {}", summary.modpack().id()), 30),
            compact_scene_label(
                &format!(
                    "SAVE HASH {} {}",
                    summary.modpack().hash(),
                    summary.pack_content_hash()
                ),
                30,
            ),
        ],
        Err(error) => vec![compact_scene_label(&format!("INVALID SAVE {error}"), 30)],
    }
}

fn visible_special_boundary_entries(runtime_shell: &BevyRuntimeShell) -> Vec<String> {
    let Some(boundary) = &runtime_shell.special_boundary else {
        return Vec::new();
    };
    visible_special_boundary_display_entries(boundary)
}

fn visible_special_boundary_display_entries(boundary: &SpecialBoundaryDisplay) -> Vec<String> {
    if boundary.label == "HallOfFamePC" {
        return boundary.details.clone();
    }
    if matches!(
        boundary.label.as_str(),
        "PokecenterPCCantUseText" | "ProfOaksPcBoot" | "BugContestJudging"
    ) {
        return boundary
            .details
            .iter()
            .flat_map(|line| wrap_scene_dialog_line(line, SCENE_DIALOG_TEXT_CHARS))
            .take(SCENE_MENU_VISIBLE_ROWS)
            .collect();
    }
    vec![format!(">{}", boundary.label)]
}

fn visible_move_battler_offsets(
    animation: Option<&VisibleMoveAnimation>,
) -> (Vec3, Vec3) {
    let Some(animation) = animation.filter(|animation| animation.started) else {
        return (Vec3::ZERO, Vec3::ZERO);
    };
    let mut player = Vec3::ZERO;
    let mut enemy = Vec3::ZERO;
    for effect in animation
        .bg_events
        .iter()
        .filter(|effect| !effect.incremented && effect.frame <= animation.frame)
    {
        if animation.bg_events.iter().any(|candidate| {
            !candidate.incremented
                && candidate.effect_id == effect.effect_id
                && candidate.frame > effect.frame
                && candidate.frame <= animation.frame
        }) {
            continue;
        }
        let user_is_player = animation.player_move;
        let target_is_player = match effect.target.as_str() {
            "BG_EFFECT_USER" => user_is_player,
            "BG_EFFECT_TARGET" => !user_is_player,
            _ => true,
        };
        let reset_frame = animation
            .bg_events
            .iter()
            .filter(|candidate| {
                candidate.incremented
                    && candidate.effect_id == effect.effect_id
                    && candidate.frame >= effect.frame
                    && candidate.frame <= animation.frame
            })
            .map(|candidate| candidate.frame)
            .max()
            .unwrap_or(effect.frame);
        let age = animation.frame.saturating_sub(reset_frame);
        let phase_age = animation.frame.saturating_sub(effect.frame);
        let source_scale = TILE_SIZE / SOURCE_TILE_SIZE as f32;
        let (offset_x, offset_y, force_player) = match effect.effect_id.as_str() {
            "BATTLE_BG_EFFECT_TACKLE"
            | "BATTLE_BG_EFFECT_BODY_SLAM"
            | "BATTLE_BG_EFFECT_BETA_PURSUIT"
            | "BATTLE_BG_EFFECT_ROLLOUT"
            | "BATTLE_BG_EFFECT_VITAL_THROW" => {
                let distance = match age {
                    0 => 2,
                    1 => 4,
                    2 => 6,
                    3 => 8,
                    4 => 6,
                    5 => 4,
                    6 => 2,
                    7 => 0,
                    _ => continue,
                };
                (if target_is_player { distance } else { -distance }, 0, false)
            }
            "BATTLE_BG_EFFECT_WOBBLE_MON" | "BATTLE_BG_EFFECT_WAVE_DEFORM_MON" => {
                let amplitude = if effect.param == 0 { 4 } else { effect.param };
                let frequency = if effect.duration == 0 { 3 } else { effect.duration };
                let lifetime = if effect.duration == 0 { frequency.saturating_mul(2) } else { effect.duration };
                if age >= lifetime.max(1) { continue; }
                let value = (f64::from(amplitude)
                    * (f64::from(phase_age) * std::f64::consts::PI / f64::from(frequency.max(1))).sin())
                    .round() as i32;
                (0, value, false)
            }
            "BATTLE_BG_EFFECT_WOBBLE_PLAYER" => {
                let amplitude = if effect.param == 0 { 3 } else { effect.param };
                let frequency = if effect.duration == 0 { 4 } else { effect.duration };
                let lifetime = if effect.duration == 0 { frequency.saturating_mul(2) } else { effect.duration };
                if age >= lifetime.max(1) { continue; }
                let value = (f64::from(amplitude)
                    * (f64::from(phase_age) * std::f64::consts::PI / f64::from(frequency.max(1))).sin())
                    .round() as i32;
                (value, 0, true)
            }
            "BATTLE_BG_EFFECT_VIBRATE_MON" => {
                let amplitude = if effect.param == 0 { 2 } else { effect.param };
                let lifetime = effect.duration.max(2);
                if age >= lifetime { continue; }
                let value = (f64::from(amplitude)
                    * (f64::from(phase_age) * std::f64::consts::PI).sin())
                    .round() as i32;
                (value, 0, false)
            }
            "BATTLE_BG_EFFECT_DIG" | "BATTLE_BG_EFFECT_FLAIL" | "BATTLE_BG_EFFECT_DOUBLE_TEAM" => {
                let amplitude = if effect.param == 0 { 5 } else { effect.param };
                let frequency = if effect.param == 0 { 3 } else { u16::from(effect.param) };
                let lifetime = if effect.duration == 0 { frequency.saturating_mul(2) } else { effect.duration };
                if age >= lifetime.max(1) { continue; }
                let value = (f64::from(amplitude)
                    * (f64::from(phase_age) * std::f64::consts::PI / f64::from(frequency.max(1))).sin())
                    .round() as i32;
                (0, value, false)
            }
            "BATTLE_BG_EFFECT_BOUNCE_DOWN" => {
                let Some(value) = visible_bounce_down_offset(animation, effect) else {
                    continue;
                };
                (0, value, false)
            }
            "BATTLE_BG_EFFECT_REMOVE_MON" => {
                let duration = if effect.duration == 0 { 6 } else { effect.duration };
                if age >= duration.saturating_sub(1) {
                    continue;
                }
                let start = if effect.param == 0 { 80 } else { i32::from(effect.param) };
                let value = (f64::from(start) * (f64::from(age) / f64::from(duration.max(1))))
                    .round() as i32;
                (if target_is_player { value } else { -value }, 0, false)
            }
            "BATTLE_BG_EFFECT_BETA_SEND_OUT_MON1" | "BATTLE_BG_EFFECT_BETA_SEND_OUT_MON2" => {
                let duration = if effect.duration == 0 { 6 } else { effect.duration };
                if age >= duration {
                    continue;
                }
                let start = if effect.param == 0 { 80 } else { i32::from(effect.param) };
                let numerator = duration.saturating_sub(age).saturating_sub(1);
                let value = (f64::from(start)
                    * (f64::from(numerator) / f64::from(duration.max(1))))
                    .round() as i32;
                (if target_is_player { value } else { -value }, 0, false)
            }
            "BATTLE_BG_EFFECT_FAINT_MON" => {
                let duration = if effect.duration == 0 { 14 } else { effect.duration };
                if age >= duration {
                    continue;
                }
                let step = i32::from(if effect.param == 0 { 4 } else { effect.param });
                let max_steps = i32::from((duration.saturating_add(1) / 2).max(1));
                // The first host update is the source effect's progress-zero
                // frame. Advance once per two updates after that initial hold.
                let step_index = i32::from((age / 2).min(duration));
                (0, (step_index * step).min(max_steps * step), false)
            }
            _ => continue,
        };
        let offset = Vec3::new(
            offset_x as f32 * source_scale,
            -(offset_y as f32) * source_scale,
            0.0,
        );
        let target_is_player = force_player || target_is_player;
        if target_is_player {
            player += offset;
        } else {
            enemy += offset;
        }
    }
    (player, enemy)
}

fn visible_move_screen_offset(animation: Option<&VisibleMoveAnimation>) -> Vec3 {
    let Some(animation) = animation.filter(|animation| animation.started) else {
        return Vec3::ZERO;
    };
    let mut result = Vec3::ZERO;
    for effect in animation.bg_events.iter().filter(|effect| {
        !effect.incremented
            && effect.frame <= animation.frame
            && matches!(
                effect.effect_id.as_str(),
                "BATTLE_BG_EFFECT_SHAKE_SCREEN_X"
                    | "BATTLE_BG_EFFECT_SHAKE_SCREEN_Y"
                    | "BATTLE_BG_EFFECT_WOBBLE_SCREEN"
            )
    }) {
        if animation.bg_events.iter().any(|candidate| {
            !candidate.incremented
                && candidate.effect_id == effect.effect_id
                && candidate.frame > effect.frame
                && candidate.frame <= animation.frame
        }) {
            continue;
        }
        let reset_frame = animation
            .bg_events
            .iter()
            .filter(|candidate| {
                candidate.incremented
                    && candidate.effect_id == effect.effect_id
                    && candidate.frame >= effect.frame
                    && candidate.frame <= animation.frame
            })
            .map(|candidate| candidate.frame)
            .max()
            .unwrap_or(effect.frame);
        let age = animation.frame.saturating_sub(reset_frame);
        let phase_age = animation.frame.saturating_sub(effect.frame);
        let (screen_x, screen_y) = if effect.effect_id == "BATTLE_BG_EFFECT_WOBBLE_SCREEN" {
            let amplitude = if effect.param == 0 { 3 } else { effect.param };
            let frequency = if effect.duration == 0 { 4 } else { effect.duration };
            let lifetime = if effect.duration == 0 {
                frequency.saturating_mul(2)
            } else {
                effect.duration
            };
            if age >= lifetime.max(1) {
                continue;
            }
            let value = (f64::from(amplitude)
                * (f64::from(phase_age) * std::f64::consts::PI
                    / f64::from(frequency.max(1)))
                .sin())
                .round() as f32;
            (value, 0.0)
        } else {
            if age >= effect.duration.max(1) {
                continue;
            }
            let amplitude = if let Some(value) =
                parse_visible_battle_animation_int(&effect.target).filter(|value| *value > 0)
            {
                value as f32
            } else if effect.param >> 4 == 0 {
                4.0
            } else {
                f32::from(effect.param >> 4)
            };
            let frequency = u16::from(if effect.param & 0x0f == 0 {
                2
            } else {
                effect.param & 0x0f
            });
            let value = if (age / frequency) & 1 == 0 {
                amplitude
            } else {
                -amplitude
            };
            if effect.effect_id == "BATTLE_BG_EFFECT_SHAKE_SCREEN_X" {
                (value, 0.0)
            } else {
                (0.0, value)
            }
        };
        let source_scale = TILE_SIZE / SOURCE_TILE_SIZE as f32;
        result += Vec3::new(
            screen_x * source_scale,
            -screen_y * source_scale,
            0.0,
        );
    }
    result
}

fn apply_visible_battle_screen_offset(
    runtime_shell: Res<BevyRuntimeShell>,
    mut battle_commands: Query<
        (Entity, &mut Transform),
        (
            Or<(With<BattleCommandMarker>, With<BattleBattlerMarker>)>,
            Without<FixedBattleCanvasMarker>,
        ),
    >,
    mut applied_offsets: Local<HashMap<Entity, Vec3>>,
) {
    let current = visible_move_screen_offset(runtime_shell.visible_move_animations.front());
    let mut live = HashSet::new();
    for (entity, mut transform) in &mut battle_commands {
        live.insert(entity);
        let previous = applied_offsets.get(&entity).copied().unwrap_or(Vec3::ZERO);
        transform.translation += current - previous;
        applied_offsets.insert(entity, current);
    }
    applied_offsets.retain(|entity, _| live.contains(entity));
}

fn visible_bounce_down_offset(
    animation: &VisibleMoveAnimation,
    effect: &VisibleMoveBgEvent,
) -> Option<i32> {
    let mut distance = 0_i32;
    let mut returning = false;
    let mut end_requested = false;
    for frame in effect.frame..=animation.frame {
        if animation.bg_events.iter().any(|candidate| {
            candidate.incremented
                && candidate.effect_id == effect.effect_id
                && candidate.frame == frame
        }) {
            end_requested = true;
        }
        if !returning {
            distance += 2;
            if distance >= 8 {
                distance = 8;
                returning = true;
            }
        } else {
            distance = (distance - 2).max(0);
            if distance == 0 && !end_requested {
                returning = false;
            }
        }
        if end_requested && returning && distance == 0 {
            return (frame == animation.frame).then_some(0);
        }
    }
    Some(distance)
}

fn visible_move_battler_visibility(
    animation: Option<&VisibleMoveAnimation>,
) -> (bool, bool) {
    let Some(animation) = animation.filter(|animation| animation.started) else {
        return (true, true);
    };
    let mut player_visible = true;
    let mut enemy_visible = true;
    for effect in animation
        .bg_events
        .iter()
        .filter(|effect| !effect.incremented && effect.frame <= animation.frame)
    {
        let reset_frame = animation
            .bg_events
            .iter()
            .filter(|candidate| {
                candidate.incremented
                    && candidate.effect_id == effect.effect_id
                    && candidate.frame >= effect.frame
                    && candidate.frame <= animation.frame
            })
            .map(|candidate| candidate.frame)
            .max()
            .unwrap_or(effect.frame);
        let age = animation.frame.saturating_sub(reset_frame);
        let visible = match effect.effect_id.as_str() {
            "BATTLE_BG_EFFECT_HIDE_MON" | "BATTLE_BG_EFFECT_WITHDRAW" => false,
            "BATTLE_BG_EFFECT_SHOW_MON" => true,
            "BATTLE_BG_EFFECT_REMOVE_MON" => {
                let duration = if effect.duration == 0 { 6 } else { effect.duration };
                age < duration.saturating_sub(1)
            }
            "BATTLE_BG_EFFECT_FAINT_MON" => {
                let duration = if effect.duration == 0 { 14 } else { effect.duration };
                age < duration
            }
            "BATTLE_BG_EFFECT_ENTER_MON" => true,
            "BATTLE_BG_EFFECT_RETURN_MON" => {
                let step_delay = effect.duration.max(1);
                age < step_delay.saturating_mul(3)
            }
            _ => continue,
        };
        let target_player = match effect.target.as_str() {
            "BG_EFFECT_USER" => animation.player_move,
            "BG_EFFECT_TARGET" => !animation.player_move,
            _ => matches!(
                effect.effect_id.as_str(),
                "BATTLE_BG_EFFECT_REMOVE_MON" | "BATTLE_BG_EFFECT_FAINT_MON"
            ),
        };
        if target_player {
            player_visible = visible;
        } else {
            enemy_visible = visible;
        }
    }
    (player_visible, enemy_visible)
}

fn visible_move_controls_battler_visibility(
    animation: Option<&VisibleMoveAnimation>,
    player: bool,
) -> bool {
    let Some(animation) = animation.filter(|animation| animation.started) else {
        return false;
    };
    animation.bg_events.iter().any(|effect| {
        if !matches!(
            effect.effect_id.as_str(),
            "BATTLE_BG_EFFECT_HIDE_MON"
                | "BATTLE_BG_EFFECT_SHOW_MON"
                | "BATTLE_BG_EFFECT_WITHDRAW"
                | "BATTLE_BG_EFFECT_REMOVE_MON"
                | "BATTLE_BG_EFFECT_ENTER_MON"
                | "BATTLE_BG_EFFECT_RETURN_MON"
        ) {
            return false;
        }
        let targets_player = match effect.target.as_str() {
            "BG_EFFECT_USER" => animation.player_move,
            "BG_EFFECT_TARGET" => !animation.player_move,
            _ => false,
        };
        targets_player == player
    })
}

fn visible_move_battler_clip_tiles(
    animation: Option<&VisibleMoveAnimation>,
) -> (Option<u8>, Option<u8>) {
    let Some(animation) = animation.filter(|animation| animation.started) else {
        return (None, None);
    };
    let mut player = None;
    let mut enemy = None;
    for effect in animation
        .bg_events
        .iter()
        .filter(|effect| !effect.incremented && effect.frame <= animation.frame)
    {
        if !matches!(
            effect.effect_id.as_str(),
            "BATTLE_BG_EFFECT_ENTER_MON"
                | "BATTLE_BG_EFFECT_RETURN_MON"
                | "BATTLE_BG_EFFECT_BATTLEROBJ_1ROW"
                | "BATTLE_BG_EFFECT_BATTLEROBJ_2ROW"
        ) {
            continue;
        }
        let age = animation.frame.saturating_sub(effect.frame);
        let step = if matches!(
            effect.effect_id.as_str(),
            "BATTLE_BG_EFFECT_BATTLEROBJ_1ROW" | "BATTLE_BG_EFFECT_BATTLEROBJ_2ROW"
        ) {
            age
        } else {
            age / effect.duration.max(1)
        };
        let target_player = match effect.target.as_str() {
            "BG_EFFECT_USER" => animation.player_move,
            "BG_EFFECT_TARGET" => !animation.player_move,
            _ => true,
        };
        let clip_tiles = match (effect.effect_id.as_str(), target_player, step) {
            ("BATTLE_BG_EFFECT_ENTER_MON", true, 0) => Some(2),
            ("BATTLE_BG_EFFECT_ENTER_MON", true, 1) => Some(4),
            ("BATTLE_BG_EFFECT_ENTER_MON", false, 0) => Some(3),
            ("BATTLE_BG_EFFECT_ENTER_MON", false, 1) => Some(5),
            ("BATTLE_BG_EFFECT_RETURN_MON", true, 0) => Some(6),
            ("BATTLE_BG_EFFECT_RETURN_MON", true, 1) => Some(4),
            ("BATTLE_BG_EFFECT_RETURN_MON", true, _) => Some(2),
            ("BATTLE_BG_EFFECT_RETURN_MON", false, 0) => Some(7),
            ("BATTLE_BG_EFFECT_RETURN_MON", false, 1) => Some(5),
            ("BATTLE_BG_EFFECT_RETURN_MON", false, _) => Some(3),
            ("BATTLE_BG_EFFECT_BATTLEROBJ_1ROW", true, value)
            | ("BATTLE_BG_EFFECT_BATTLEROBJ_2ROW", true, value)
                if value < 6 => Some((value + 1) as u8),
            ("BATTLE_BG_EFFECT_BATTLEROBJ_1ROW", false, value)
            | ("BATTLE_BG_EFFECT_BATTLEROBJ_2ROW", false, value)
                if value < 6 => Some((value + 1) as u8),
            _ => None,
        };
        if target_player {
            player = clip_tiles;
        } else {
            enemy = clip_tiles;
        }
    }
    (player, enemy)
}

fn visible_move_battler_overlays(
    animation: Option<&VisibleMoveAnimation>,
) -> (Option<([u8; 3], u8)>, Option<([u8; 3], u8)>) {
    let Some(animation) = animation.filter(|animation| animation.started) else {
        return (None, None);
    };
    let mut player = None;
    let mut enemy = None;
    for effect in animation
        .bg_events
        .iter()
        .filter(|effect| !effect.incremented && effect.frame <= animation.frame)
    {
        let target_player = match effect.target.as_str() {
            "BG_EFFECT_USER" => animation.player_move,
            "BG_EFFECT_TARGET" => !animation.player_move,
            _ => false,
        };
        let age = animation.frame.saturating_sub(effect.frame);
        let stopped = animation.bg_events.iter().any(|candidate| {
            candidate.incremented
                && candidate.effect_id == effect.effect_id
                && candidate.frame >= effect.frame
                && candidate.frame <= animation.frame
        });
        let palette_step = u16::from((effect.param >> 4).max(1));
        let overlay = match effect.effect_id.as_str() {
            "BATTLE_BG_EFFECT_FADE_MON_TO_LIGHT" => {
                let alpha = match age / palette_step {
                    1 => 128,
                    2 => 213,
                    _ => 0,
                };
                (alpha != 0).then_some(([255, 255, 255], alpha))
            }
            "BATTLE_BG_EFFECT_FADE_MON_TO_LIGHT_REPEATING" if !stopped => {
                let alpha = [0, 128, 213, 128][usize::from((age / palette_step) as u8 & 3)];
                (alpha != 0).then_some(([255, 255, 255], alpha))
            }
            "BATTLE_BG_EFFECT_FADE_MON_TO_BLACK" => {
                let alpha = match age / palette_step {
                    1 => 128,
                    2 => 213,
                    _ => 0,
                };
                (alpha != 0).then_some(([0, 0, 0], alpha))
            }
            "BATTLE_BG_EFFECT_FADE_MON_TO_BLACK_REPEATING"
            | "BATTLE_BG_EFFECT_FADE_MONS_TO_BLACK_REPEATING" if !stopped => {
                let alpha = [0, 128, 213, 128][usize::from((age / palette_step) as u8 & 3)];
                (alpha != 0).then_some(([0, 0, 0], alpha))
            }
            "BATTLE_BG_EFFECT_FADE_MON_TO_WHITE_WAIT_FADE_BACK" => {
                if stopped {
                    None
                } else {
                    let step_delay = u16::from((effect.param >> 4).max(1));
                    let initial_delay = u16::from(if effect.param & 0x0f == 0 {
                        (effect.param >> 4).max(1)
                    } else {
                        effect.param & 0x0f
                    });
                    let index = if age < initial_delay {
                        0
                    } else {
                        1 + (age - initial_delay) / step_delay
                    };
                    let alpha = match index {
                        0 | 16 => 0,
                        1 | 15 => 128,
                        2 | 14 => 213,
                        3..=13 => 255,
                        _ => 0,
                    };
                    (alpha != 0).then_some(([255, 255, 255], alpha))
                }
            }
            "BATTLE_BG_EFFECT_FADE_MON_FROM_WHITE" => {
                let index = usize::from((age / 2).min(3));
                let alpha = [255, 213, 128, 0][index];
                (alpha != 0).then_some(([255, 255, 255], alpha))
            }
            _ => None,
        };
        if let Some(overlay) = overlay {
            if effect.effect_id == "BATTLE_BG_EFFECT_FADE_MONS_TO_BLACK_REPEATING" {
                player = Some(overlay);
                enemy = Some(overlay);
            } else if target_player {
                player = Some(overlay);
            } else {
                enemy = Some(overlay);
            }
        }
    }
    (player, enemy)
}

fn visible_move_battler_art_overrides(
    animation: Option<&VisibleMoveAnimation>,
) -> (VisibleBattlerArtOverride, VisibleBattlerArtOverride) {
    let Some(animation) = animation.filter(|animation| {
        animation.started || animation.waiting_for_hp
    }) else {
        return (
            VisibleBattlerArtOverride::Unchanged,
            VisibleBattlerArtOverride::Unchanged,
        );
    };
    let mut player = VisibleBattlerArtOverride::Unchanged;
    let mut enemy = VisibleBattlerArtOverride::Unchanged;
    for event in animation
        .bg_events
        .iter()
        .filter(|event| event.frame <= animation.frame && event.effect_id.starts_with("BATTLE_ACTOR_"))
    {
        let mut target_player = animation.player_move;
        let state = match event.effect_id.as_str() {
            "BATTLE_ACTOR_TRANSFORM" => VisibleBattlerArtOverride::Transform,
            "BATTLE_ACTOR_RAISESUB" => VisibleBattlerArtOverride::Substitute,
            "BATTLE_ACTOR_DROPSUB" | "BATTLE_ACTOR_UPDATEACTORPIC" => {
                VisibleBattlerArtOverride::Pokemon
            }
            "BATTLE_ACTOR_BEATUP" => VisibleBattlerArtOverride::Pokemon,
            "BATTLE_ACTOR_MINIMIZE" => VisibleBattlerArtOverride::Minimize,
            "BATTLE_ACTOR_MINIMIZEOPP" => {
                target_player = !target_player;
                VisibleBattlerArtOverride::Minimize
            }
            _ => continue,
        };
        if target_player {
            player = state;
        } else {
            enemy = state;
        }
    }
    (player, enemy)
}

fn visible_move_battler_species_overrides(
    animation: Option<&VisibleMoveAnimation>,
) -> (Option<&str>, Option<&str>) {
    let Some(animation) = animation.filter(|animation| animation.started) else {
        return (None, None);
    };
    let beat_up_active = animation.bg_events.iter().any(|event| {
        event.frame <= animation.frame && event.effect_id == "BATTLE_ACTOR_BEATUP"
    });
    if !beat_up_active {
        return (None, None);
    }
    let species = animation.actor_species_override.as_deref();
    if animation.player_move {
        (species, None)
    } else {
        (None, species)
    }
}

fn visible_move_battler_shiny_overrides(
    animation: Option<&VisibleMoveAnimation>,
) -> (Option<bool>, Option<bool>) {
    let Some(animation) = animation.filter(|animation| animation.started) else {
        return (None, None);
    };
    let beat_up_active = animation.bg_events.iter().any(|event| {
        event.frame <= animation.frame && event.effect_id == "BATTLE_ACTOR_BEATUP"
    });
    if !beat_up_active {
        return (None, None);
    }
    if animation.player_move {
        (animation.actor_shiny_override, None)
    } else {
        (None, animation.actor_shiny_override)
    }
}

fn visible_surf_line_offsets(
    animation: Option<&VisibleMoveAnimation>,
) -> Option<[i8; 0x5f]> {
    let animation = animation.filter(|animation| animation.started)?;
    let surf_effect = animation.bg_events.iter().find(|effect| {
        !effect.incremented
            && effect.effect_id == "BATTLE_BG_EFFECT_SURF"
            && effect.frame <= animation.frame
    })?;
    let (spawn_index, spawn) = animation.object_events.iter().enumerate().find(|(_, event)| {
        event.frame <= animation.frame
            && matches!(
                &event.command,
                VisibleMoveObjectCommand::Spawn { object_id, .. }
                    if object_id == "BATTLE_ANIM_OBJ_SURF"
            )
    })?;
    let VisibleMoveObjectCommand::Spawn { x, y, param, .. } = &spawn.command else {
        return None;
    };
    let mut state = 0_u8;
    let mut state_frame = spawn.frame;
    for event in animation.object_events.iter().skip(spawn_index + 1) {
        if event.frame > animation.frame {
            break;
        }
        match &event.command {
            VisibleMoveObjectCommand::Increment { slot: 1 } => {
                state = state.wrapping_add(1);
                state_frame = event.frame;
            }
            VisibleMoveObjectCommand::Set { slot: 1, value } => {
                state = *value;
                state_frame = event.frame;
            }
            VisibleMoveObjectCommand::Clear => return None,
            _ => {}
        }
    }
    let age = animation.frame.saturating_sub(spawn.frame);
    let state_age = animation.frame.saturating_sub(state_frame);
    let (_, animated_y) = visible_battle_anim_object_position(
        "BATTLE_ANIM_FUNC_SURF",
        i32::from(*x),
        i32::from(*y),
        *param,
        age,
        state,
        state_age,
        animation.player_move,
    )?;
    let rise_updates = (i32::from(*y) - i32::from(*param) + 1).max(0) as u16;
    let start_y = if state != 0 {
        i32::from(*param).saturating_sub(1)
            + i32::from(state_age.saturating_add(1)).saturating_mul(2)
    } else if age >= rise_updates {
        0
    } else {
        animated_y.saturating_sub(16)
    };
    let start = start_y.clamp(0, 0x5e) as usize;
    let rotation = usize::from(animation.frame.saturating_sub(surf_effect.frame)) + 1;
    let mut offsets = [0_i8; 0x5f];
    for line in start.saturating_add(1)..=0x5e {
        offsets[line] = visible_battle_anim_sine(((line + rotation) as u8).wrapping_mul(2), 2)
            as i8;
    }
    Some(offsets)
}

fn spawn_battle_battler_markers(
    commands: &mut Commands,
    snapshot: &RuntimeShellSnapshot,
    battle: &crate::RuntimeBattleSnapshot,
    entry_messages_remaining: usize,
    enemy_send_out_pending: bool,
    player_send_out_pending: bool,
    capture_enemy_hidden: bool,
    capture_enemy_clip_tiles: Option<u8>,
    capture_throw_active: bool,
    send_out_animation: Option<&VisibleSendOutAnimation>,
    trainer_exit_animation: Option<&VisibleTrainerExitAnimation>,
    frontpic_animation: Option<&VisibleFrontpicAnimation>,
    move_animation: Option<&VisibleMoveAnimation>,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) -> Result<()> {
    let (player_move_offset, enemy_move_offset) =
        visible_move_battler_offsets(move_animation);
    let (move_player_visible, move_enemy_visible) =
        visible_move_battler_visibility(move_animation);
    let move_player_visible = move_player_visible
        && (!battle.player_semi_invulnerable
            || visible_move_controls_battler_visibility(move_animation, true));
    let move_enemy_visible = move_enemy_visible
        && (!battle.enemy_semi_invulnerable
            || visible_move_controls_battler_visibility(move_animation, false));
    let (move_player_clip_tiles, move_enemy_clip_tiles) =
        visible_move_battler_clip_tiles(move_animation);
    let (move_player_overlay, move_enemy_overlay) =
        visible_move_battler_overlays(move_animation);
    let (move_player_art, move_enemy_art) =
        visible_move_battler_art_overrides(move_animation);
    let (move_player_species, move_enemy_species) =
        visible_move_battler_species_overrides(move_animation);
    let (move_player_shiny, move_enemy_shiny) =
        visible_move_battler_shiny_overrides(move_animation);
    let surf_line_offsets = visible_surf_line_offsets(move_animation);
    let render_hp = |side, hp| visible_faint_animation_render_hp(move_animation, side, hp);
    let active_player_species = battle.player_transformed_species.as_deref().or_else(|| {
        battle
            .active_player_party_index
            .and_then(|active_index| snapshot.party.slots.iter().find(|slot| slot.index == active_index))
            .map(|slot| slot.pokemon.species.id.as_str())
    });
    let enemy_default_species = battle
        .enemy_transformed_species
        .as_deref()
        .unwrap_or(&battle.enemy_pokemon.species.id);
    let enemy_render_species = if let Some(species) = move_enemy_species {
        species
    } else if move_enemy_art == VisibleBattlerArtOverride::Transform {
        active_player_species.context(
            "enemy Transform animation requires a concrete active player species",
        )?
    } else {
        enemy_default_species
    };
    let player_transform_species = if let Some(species) = move_player_species {
        Some(species)
    } else if move_player_art == VisibleBattlerArtOverride::Transform {
        Some(enemy_default_species)
    } else {
        None
    };
    let send_out_scale = |side| {
        send_out_animation
            .filter(|animation| animation.side == side)
            .map(VisibleSendOutAnimation::battler_scale)
            .unwrap_or(1.0)
    };
    let send_out_clip_tiles = |side| {
        send_out_animation
            .filter(|animation| animation.side == side)
            .and_then(VisibleSendOutAnimation::battler_clip_tiles)
    };
    let enemy_animation_frame = frontpic_animation
        .filter(|animation| animation.species_id == battle.enemy_pokemon.species.id)
        .map(|animation| animation.frame)
        .unwrap_or(0);
    if let Some(exit) = trainer_exit_animation
        && exit.side == crate::core::battle::turn::BattleSide::Player
    {
        spawn_battler_marker(
            commands,
            rendered_art,
            asset_root,
            images,
            enemy_render_species,
            PokemonSpriteSide::Front,
            render_hp(crate::core::battle::turn::BattleSide::Enemy, battle.enemy_pokemon.hp),
            battle.enemy_pokemon.max_hp,
            battle.enemy_substitute_hp > 0,
            move_enemy_shiny.unwrap_or_else(|| visible_pokemon_is_shiny(&battle.enemy_pokemon)),
            enemy_animation_frame,
            enemy_move_offset,
            1.0,
            move_enemy_overlay,
            move_enemy_art,
            move_enemy_clip_tiles,
            surf_line_offsets.as_ref(),
        )?;
        let player_stem = if snapshot.trainer.player_gender == PLAYER_GENDER_FEMALE {
            "kris_back"
        } else {
            "chris_back"
        };
        spawn_battle_trainer_marker(
            commands,
            rendered_art,
            asset_root,
            images,
            &format!("battle-player:{player_stem}"),
            Vec3::new(
                PLAYFIELD_LEFT + TILE_SIZE * 2.0 + exit.x_offset(),
                PLAYFIELD_TOP - TILE_SIZE * 6.0,
                3.0,
            ),
        )?;
        return Ok(());
    }
    if enemy_send_out_pending {
        let active_index = battle.active_player_party_index.context(
            "enemy send-out presentation requires an active player party slot",
        )?;
        let slot = snapshot
            .party
            .slots
            .iter()
            .find(|slot| slot.index == active_index)
            .with_context(|| {
                format!("active player party slot {active_index} is absent during enemy send-out")
            })?;
        spawn_battler_marker(
            commands,
            rendered_art,
            asset_root,
            images,
            player_transform_species.unwrap_or_else(|| {
                battle
                    .player_transformed_species
                    .as_deref()
                    .unwrap_or(&slot.pokemon.species.id)
            }),
            PokemonSpriteSide::Back,
            render_hp(crate::core::battle::turn::BattleSide::Player, slot.pokemon.hp),
            slot.pokemon.max_hp,
            battle.player_substitute_hp > 0,
            move_player_shiny.unwrap_or_else(|| visible_pokemon_is_shiny(&slot.pokemon)),
            0,
            player_move_offset,
            1.0,
            move_player_overlay,
            move_player_art,
            move_player_clip_tiles,
            surf_line_offsets.as_ref(),
        )?;
        return Ok(());
    }
    if player_send_out_pending {
        spawn_battler_marker(
            commands,
            rendered_art,
            asset_root,
            images,
            enemy_render_species,
            PokemonSpriteSide::Front,
            render_hp(crate::core::battle::turn::BattleSide::Enemy, battle.enemy_pokemon.hp),
            battle.enemy_pokemon.max_hp,
            battle.enemy_substitute_hp > 0,
            move_enemy_shiny.unwrap_or_else(|| visible_pokemon_is_shiny(&battle.enemy_pokemon)),
            enemy_animation_frame,
            enemy_move_offset,
            1.0,
            move_enemy_overlay,
            move_enemy_art,
            move_enemy_clip_tiles,
            surf_line_offsets.as_ref(),
        )?;
        return Ok(());
    }
    if let RuntimeBattleKind::Trainer { trainer_class, .. } = &battle.kind {
        if entry_messages_remaining >= 3 {
            spawn_battle_trainer_marker(
                commands,
                rendered_art,
                asset_root,
                images,
                &format!(
                    "battle-trainer:{}",
                    normalize_battle_trainer_sprite_id(trainer_class)
                ),
                Vec3::new(
                    PLAYFIELD_LEFT + TILE_SIZE * 12.0,
                    PLAYFIELD_TOP,
                    3.0,
                ),
            )?;
            let player_stem = if snapshot.trainer.player_gender == PLAYER_GENDER_FEMALE {
                "kris_back"
            } else {
                "chris_back"
            };
            spawn_battle_trainer_marker(
                commands,
                rendered_art,
                asset_root,
                images,
                &format!("battle-player:{player_stem}"),
                Vec3::new(
                    PLAYFIELD_LEFT + TILE_SIZE * 2.0,
                    PLAYFIELD_TOP - TILE_SIZE * 6.0,
                    3.0,
                ),
            )?;
            return Ok(());
        }
        if entry_messages_remaining == 2 {
            // ShowBattleTextEnemySentOut runs after the enemy trainer has
            // slid away but before ShowSetEnemyMonAndSendOutAnimation. The
            // player's backpic remains; the enemy Pokémon is not visible yet.
            let player_stem = if snapshot.trainer.player_gender == PLAYER_GENDER_FEMALE {
                "kris_back"
            } else {
                "chris_back"
            };
            spawn_battle_trainer_marker(
                commands,
                rendered_art,
                asset_root,
                images,
                &format!("battle-player:{player_stem}"),
                Vec3::new(
                    PLAYFIELD_LEFT + TILE_SIZE * 2.0,
                    PLAYFIELD_TOP - TILE_SIZE * 6.0,
                    3.0,
                ),
            )?;
            if let Some(exit) = trainer_exit_animation
                && exit.side == crate::core::battle::turn::BattleSide::Enemy
            {
                spawn_battle_trainer_marker(
                    commands,
                    rendered_art,
                    asset_root,
                    images,
                    &format!(
                        "battle-trainer:{}",
                        normalize_battle_trainer_sprite_id(trainer_class)
                    ),
                    Vec3::new(
                        PLAYFIELD_LEFT + TILE_SIZE * 12.0 + exit.x_offset(),
                        PLAYFIELD_TOP,
                        3.0,
                    ),
                )?;
            }
            return Ok(());
        }
    }
    let enemy_scale = send_out_scale(crate::core::battle::turn::BattleSide::Enemy);
    if !capture_enemy_hidden && move_enemy_visible && enemy_scale > 0.0 {
        spawn_battler_marker(
            commands,
            rendered_art,
            asset_root,
            images,
            enemy_render_species,
            PokemonSpriteSide::Front,
            render_hp(crate::core::battle::turn::BattleSide::Enemy, battle.enemy_pokemon.hp),
            battle.enemy_pokemon.max_hp,
            battle.enemy_substitute_hp > 0,
            move_enemy_shiny.unwrap_or_else(|| visible_pokemon_is_shiny(&battle.enemy_pokemon)),
            enemy_animation_frame,
            enemy_move_offset,
            enemy_scale,
            move_enemy_overlay,
            move_enemy_art,
            capture_enemy_clip_tiles
                .or_else(|| send_out_clip_tiles(crate::core::battle::turn::BattleSide::Enemy))
                .or(move_enemy_clip_tiles),
            surf_line_offsets.as_ref(),
        )?;
    }
    if battle.battle_type == "BATTLETYPE_TUTORIAL" {
        if !capture_throw_active {
            spawn_battle_trainer_marker(
                commands,
                rendered_art,
                asset_root,
                images,
                "battle-player:dude",
                Vec3::new(
                    PLAYFIELD_LEFT + TILE_SIZE * 2.0,
                    PLAYFIELD_TOP - TILE_SIZE * 6.0,
                    3.0,
                ),
            )?;
        }
        return Ok(());
    }
    if entry_messages_remaining == 1 {
        let player_stem = if snapshot.trainer.player_gender == PLAYER_GENDER_FEMALE {
            "kris_back"
        } else {
            "chris_back"
        };
        spawn_battle_trainer_marker(
            commands,
            rendered_art,
            asset_root,
            images,
            &format!("battle-player:{player_stem}"),
            Vec3::new(
                PLAYFIELD_LEFT + TILE_SIZE * 2.0,
                PLAYFIELD_TOP - TILE_SIZE * 6.0,
                3.0,
            ),
        )?;
        return Ok(());
    }
    let player_pokemon_visible = entry_messages_remaining == 0 && move_player_visible;
    if player_pokemon_visible {
        let player_scale = send_out_scale(crate::core::battle::turn::BattleSide::Player);
        if player_scale <= 0.0 {
            return Ok(());
        }
        let active_index = battle
            .active_player_party_index
            .context("visible player battler requires an active party slot")?;
        let slot = snapshot
            .party
            .slots
            .iter()
            .find(|slot| slot.index == active_index)
            .with_context(|| format!("active player party slot {active_index} is absent"))?;
        spawn_battler_marker(
            commands,
            rendered_art,
            asset_root,
            images,
            player_transform_species.unwrap_or_else(|| {
                battle
                    .player_transformed_species
                    .as_deref()
                    .unwrap_or(&slot.pokemon.species.id)
            }),
            PokemonSpriteSide::Back,
            render_hp(crate::core::battle::turn::BattleSide::Player, slot.pokemon.hp),
            slot.pokemon.max_hp,
            battle.player_substitute_hp > 0,
            move_player_shiny.unwrap_or_else(|| visible_pokemon_is_shiny(&slot.pokemon)),
            0,
            player_move_offset,
            player_scale,
            move_player_overlay,
            move_player_art,
            send_out_clip_tiles(crate::core::battle::turn::BattleSide::Player)
                .or(move_player_clip_tiles),
            surf_line_offsets.as_ref(),
        )?;
    }
    Ok(())
}

fn visible_faint_animation_render_hp(
    move_animation: Option<&VisibleMoveAnimation>,
    side: crate::core::battle::turn::BattleSide,
    hp: u16,
) -> u16 {
    if hp == 0
        && move_animation.is_some_and(|animation| {
            animation.started
                && animation.animation_label == "BattleAnim_FaintMon"
                && animation.player_move
                    == (side == crate::core::battle::turn::BattleSide::Player)
        })
    {
        1
    } else {
        hp
    }
}

fn normalize_battle_trainer_sprite_id(trainer_class: &str) -> String {
    let normalized = trainer_class.trim().to_ascii_lowercase();
    if normalized == "pokemon_prof" {
        return "oak".to_string();
    }
    if normalized == "medium" {
        return normalized;
    }
    if normalized.ends_with('m') && !normalized.ends_with("_m") {
        format!("{}_m", &normalized[..normalized.len() - 1])
    } else if normalized.ends_with('f') && !normalized.ends_with("_f") {
        format!("{}_f", &normalized[..normalized.len() - 1])
    } else {
        normalized
    }
}

fn spawn_battle_trainer_marker(
    commands: &mut Commands,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
    asset_id: &str,
    top_left: Vec3,
) -> Result<()> {
    let key = IntroArtKey {
        asset_id: asset_id.to_string(),
    };
    if !rendered_art.intro_cache.contains_key(&key) {
        let frame = load_oak_intro_frame(asset_root, asset_id, images)
            .with_context(|| format!("load required battle trainer art {asset_id}"))?;
        rendered_art.intro_cache.insert(key.clone(), frame);
    }
    let frame = rendered_art
        .intro_cache
        .get(&key)
        .cloned()
        .context("cached battle trainer art disappeared")?;
    let source_scale = TILE_SIZE / SOURCE_TILE_SIZE as f32;
    let display_size = frame.size * source_scale;
    let position = Vec3::new(
        top_left.x + display_size.x * 0.5,
        top_left.y - display_size.y * 0.5,
        top_left.z,
    );
    commands.spawn((
        SpriteBundle {
            texture: frame.handle.clone(),
            sprite: Sprite {
                color: Color::WHITE,
                custom_size: Some(display_size),
                ..default()
            },
            transform: Transform::from_translation(position),
            ..default()
        },
        BattleBattlerMarker,
    ));
    Ok(())
}

fn spawn_battler_marker(
    commands: &mut Commands,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
    species_id: &str,
    side: PokemonSpriteSide,
    hp: u16,
    _max_hp: u16,
    substitute: bool,
    shiny: bool,
    animation_frame: u16,
    position_offset: Vec3,
    battler_scale: f32,
    overlay: Option<([u8; 3], u8)>,
    art_override: VisibleBattlerArtOverride,
    clip_tiles: Option<u8>,
    line_offsets: Option<&[i8; 0x5f]>,
) -> Result<()> {
    if hp == 0 {
        return Ok(());
    }
    let substitute = match art_override {
        VisibleBattlerArtOverride::Substitute => true,
        VisibleBattlerArtOverride::Pokemon => false,
        _ => substitute,
    };
    let minimize = art_override == VisibleBattlerArtOverride::Minimize;
    let frame = if minimize {
        battle_minimize_frame(rendered_art, asset_root, images)?.clone()
    } else if substitute {
        battle_substitute_frames(rendered_art, asset_root, images)?[match side {
            PokemonSpriteSide::Front => 0,
            PokemonSpriteSide::Back => 1,
        }]
        .clone()
    } else {
        pokemon_animation_frame_for_art(
            rendered_art,
            asset_root,
            species_id,
            side,
            shiny,
            animation_frame,
            images,
        )
            .with_context(|| {
                format!(
                    "required battle Pokemon art {} {:?} could not be rendered: {}",
                    species_id,
                    side,
                    pokemon_art_error(rendered_art, species_id, side, shiny)
                )
            })?
    };
    let source_scale = TILE_SIZE / SOURCE_TILE_SIZE as f32;
    let native_size = if minimize || substitute {
        Vec2::splat(TILE_SIZE * 2.0)
    } else {
        frame.size * source_scale
    };
    let display_size = native_size * battler_scale;
    let (anchor_x, anchor_y) = match side {
        PokemonSpriteSide::Front => (
            PLAYFIELD_LEFT + TILE_SIZE * 12.0,
            PLAYFIELD_TOP,
        ),
        PokemonSpriteSide::Back => (
            PLAYFIELD_LEFT + TILE_SIZE * 2.0,
            PLAYFIELD_TOP - TILE_SIZE * 6.0,
        ),
    };
    // True send-out scaling and square pic-resize clipping both retain the
    // full native frame's center.
    let position = Vec3::new(
        anchor_x + native_size.x * 0.5,
        anchor_y - native_size.y * 0.5,
        3.0,
    ) + position_offset;
    spawn_battle_battler_texture(
        commands,
        &frame,
        display_size,
        position,
        Color::WHITE,
        clip_tiles,
        line_offsets,
    );
    if let Some((colour, alpha)) = overlay {
        let overlay_frame = battle_battler_overlay_frame(
            rendered_art,
            images,
            &frame,
            colour,
        )?;
        spawn_battle_battler_texture(
            commands,
            &overlay_frame,
            display_size,
            position + Vec3::new(0.0, 0.0, 0.01),
            Color::rgba_u8(255, 255, 255, alpha),
            clip_tiles,
            line_offsets,
        );
    }
    Ok(())
}

fn spawn_battle_battler_texture(
    commands: &mut Commands,
    frame: &SpriteFrame,
    display_size: Vec2,
    position: Vec3,
    colour: Color,
    clip_tiles: Option<u8>,
    line_offsets: Option<&[i8; 0x5f]>,
) {
    let Some(line_offsets) = line_offsets else {
        let (rect, visible_size) = if let Some(clip_tiles) = clip_tiles {
            let clip_source_pixels = f32::from(clip_tiles) * SOURCE_TILE_SIZE as f32;
            let visible_width = frame.size.x.min(clip_source_pixels);
            let visible_height = frame.size.y.min(clip_source_pixels);
            let source_x = (frame.size.x - visible_width) * 0.5;
            let source_y = (frame.size.y - visible_height) * 0.5;
            (
                Some(Rect::new(
                    source_x,
                    source_y,
                    source_x + visible_width,
                    source_y + visible_height,
                )),
                Vec2::new(
                    display_size.x * visible_width / frame.size.x,
                    display_size.y * visible_height / frame.size.y,
                ),
            )
        } else {
            (None, display_size)
        };
        commands.spawn((
            SpriteBundle {
                texture: frame.handle.clone(),
                sprite: Sprite {
                    color: colour,
                    rect,
                    custom_size: Some(visible_size),
                    ..default()
                },
                transform: Transform::from_translation(position),
                ..default()
            },
            BattleBattlerMarker,
        ));
        return;
    };
    let scale = TILE_SIZE / SOURCE_TILE_SIZE as f32;
    let display_height = display_size.y / scale;
    let center_line = (PLAYFIELD_TOP - position.y) / scale;
    let top_line = center_line - display_height / 2.0;
    let first_line = top_line.floor() as i32;
    let last_line = (top_line + display_height).ceil() as i32;
    for line in first_line..last_line {
        if !(0..0x5f).contains(&line) {
            continue;
        }
        let sampled_line = line as f32 + f32::from(line_offsets[line as usize]);
        let source_top_ratio = (sampled_line - top_line) / display_height;
        let source_bottom_ratio = (sampled_line + 1.0 - top_line) / display_height;
        if source_bottom_ratio <= 0.0 || source_top_ratio >= 1.0 {
            continue;
        }
        let source_top = source_top_ratio.clamp(0.0, 1.0) * frame.size.y;
        let source_bottom = source_bottom_ratio.clamp(0.0, 1.0) * frame.size.y;
        if source_bottom <= source_top {
            continue;
        }
        commands.spawn((
            SpriteBundle {
                texture: frame.handle.clone(),
                sprite: Sprite {
                    color: colour,
                    rect: Some(Rect::new(0.0, source_top, frame.size.x, source_bottom)),
                    custom_size: Some(Vec2::new(display_size.x, scale)),
                    ..default()
                },
                transform: Transform::from_xyz(
                    position.x,
                    PLAYFIELD_TOP - (line as f32 + 0.5) * scale,
                    position.z,
                ),
                ..default()
            },
            BattleBattlerMarker,
        ));
    }
}

fn battle_minimize_frame<'a>(
    rendered_art: &'a mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) -> Result<&'a SpriteFrame> {
    if rendered_art.battle_minimize_cache.is_none()
        && rendered_art.battle_minimize_error.is_none()
    {
        let loaded = (|| -> Result<SpriteFrame> {
            let path = asset_root.runtime_assets().join("gfx/battle/minimize.png");
            let source = image::open(&path)
                .with_context(|| format!("decode Minimize battle PNG {}", path.display()))?
                .to_rgba8();
            let (width, height) = source.dimensions();
            if width == 0 || height == 0 {
                anyhow::bail!("Minimize battle PNG {} is empty", path.display());
            }
            let mut image = Image::new(
                Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                source.into_raw(),
                TextureFormat::Rgba8UnormSrgb,
                RenderAssetUsages::default(),
            );
            image.sampler = ImageSampler::nearest();
            Ok(SpriteFrame {
                handle: images.add(image),
                size: Vec2::new(width as f32, height as f32),
            })
        })();
        match loaded {
            Ok(frame) => rendered_art.battle_minimize_cache = Some(frame),
            Err(error) => rendered_art.battle_minimize_error = Some(error.to_string()),
        }
    }
    rendered_art.battle_minimize_cache.as_ref().with_context(|| {
        rendered_art
            .battle_minimize_error
            .clone()
            .unwrap_or_else(|| "Minimize battle art is unavailable".to_string())
    })
}

fn battle_battler_overlay_frame(
    rendered_art: &mut RenderedTilesetArt,
    images: &mut Assets<Image>,
    source: &SpriteFrame,
    colour: [u8; 3],
) -> Result<SpriteFrame> {
    let key = (source.handle.id(), colour);
    if let Some(frame) = rendered_art.battle_battler_overlay_cache.get(&key) {
        return Ok(frame.clone());
    }
    let mut image = images
        .get(&source.handle)
        .with_context(|| "battle battler overlay source image is unavailable")?
        .clone();
    for pixel in image.data.chunks_exact_mut(4) {
        if pixel[3] == 0 {
            continue;
        }
        pixel[0] = colour[0];
        pixel[1] = colour[1];
        pixel[2] = colour[2];
    }
    image.sampler = ImageSampler::nearest();
    let frame = SpriteFrame {
        handle: images.add(image),
        size: source.size,
    };
    rendered_art
        .battle_battler_overlay_cache
        .insert(key, frame.clone());
    Ok(frame)
}

fn visible_pokemon_is_shiny(pokemon: &crate::core::models::pokemon::Pokemon) -> bool {
    pokemon.dvs.defense == 10
        && pokemon.dvs.speed == 10
        && pokemon.dvs.special == 10
        && matches!(pokemon.dvs.attack, 2 | 3 | 6 | 7 | 10 | 11 | 14 | 15)
}

fn battle_substitute_frames<'a>(
    rendered_art: &'a mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) -> Result<&'a [SpriteFrame; 2]> {
    if rendered_art.battle_substitute_cache.is_none()
        && rendered_art.battle_substitute_error.is_none()
    {
        let loaded = (|| -> Result<[SpriteFrame; 2]> {
            let path = asset_root.runtime_assets().join("gfx/sprites/monster.png");
            let source = image::open(&path)
                .with_context(|| format!("decode substitute source PNG {}", path.display()))?
                .to_rgba8();
            if source.width() != 16 || source.height() < 32 {
                anyhow::bail!(
                    "substitute source PNG {} must contain two 16x16 frames, found {}x{}",
                    path.display(),
                    source.width(),
                    source.height()
                );
            }
            let mut load_frame = |frame_index: u32| {
                let mut pixels = vec![0; 16 * 16 * 4];
                for y in 0..16u32 {
                    for x in 0..16u32 {
                        let pixel = source.get_pixel(x, frame_index * 16 + y);
                        let output = ((y * 16 + x) * 4) as usize;
                        if pixel[0] < 240 || pixel[1] < 240 || pixel[2] < 240 {
                            pixels[output] = pixel[0];
                            pixels[output + 1] = pixel[1];
                            pixels[output + 2] = pixel[2];
                            pixels[output + 3] = 255;
                        }
                    }
                }
                let mut image = Image::new(
                    Extent3d {
                        width: 16,
                        height: 16,
                        depth_or_array_layers: 1,
                    },
                    TextureDimension::D2,
                    pixels,
                    TextureFormat::Rgba8UnormSrgb,
                    RenderAssetUsages::default(),
                );
                image.sampler = ImageSampler::nearest();
                SpriteFrame {
                    handle: images.add(image),
                    size: Vec2::splat(16.0),
                }
            };
            Ok([load_frame(0), load_frame(1)])
        })();
        match loaded {
            Ok(frames) => rendered_art.battle_substitute_cache = Some(frames),
            Err(error) => rendered_art.battle_substitute_error = Some(error.to_string()),
        }
    }
    rendered_art
        .battle_substitute_cache
        .as_ref()
        .with_context(|| {
            rendered_art
                .battle_substitute_error
                .clone()
                .unwrap_or_else(|| "battle substitute art is unavailable".to_string())
        })
}

fn spawn_battle_hud(
    commands: &mut Commands,
    snapshot: &RuntimeShellSnapshot,
    battle: &crate::RuntimeBattleSnapshot,
    entry_messages_remaining: usize,
    enemy_send_out_pending: bool,
    player_send_out_pending: bool,
    trainer_exit_active: bool,
    hp_tween: Option<&VisibleBattleHpTween>,
    exp_tween: Option<&VisibleBattleExpTween>,
    growth_rates: &crate::core::systems::experience::GrowthRateCatalog,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) -> Result<()> {
    require_bitmap_font_art(rendered_art, asset_root, images)?;
    if enemy_send_out_pending {
        spawn_battle_hud_borders(
            commands,
            rendered_art,
            asset_root,
            images,
            false,
            true,
            false,
        )?;
        let active_index = battle.active_player_party_index.context(
            "enemy send-out HUD requires an active player party slot",
        )?;
        let slot = snapshot
            .party
            .slots
            .iter()
            .find(|slot| slot.index == active_index)
            .with_context(|| {
                format!("active player party slot {active_index} is absent from the send-out HUD")
            })?;
        spawn_battle_hud_side(
            commands,
            rendered_art,
            asset_root,
            images,
            &slot.pokemon,
            BattleHpSide::Player,
            hp_tween.map(|tween| tween.player_pixels),
            hp_tween.map(|tween| tween.player_hp),
        )?;
        return Ok(());
    }
    if player_send_out_pending {
        spawn_battle_hud_borders(
            commands,
            rendered_art,
            asset_root,
            images,
            true,
            false,
            false,
        )?;
        spawn_battle_hud_side(
            commands,
            rendered_art,
            asset_root,
            images,
            &battle.enemy_pokemon,
            BattleHpSide::Enemy,
            hp_tween.map(|tween| tween.enemy_pixels),
            None,
        )?;
        return Ok(());
    }
    let trainer_enemy_pending = matches!(battle.kind, RuntimeBattleKind::Trainer { .. })
        && entry_messages_remaining >= 2;
    let player_pokemon_visible = battle.battle_type != "BATTLETYPE_TUTORIAL"
        && entry_messages_remaining == 0;
    let party_hud_visible = match &battle.kind {
        RuntimeBattleKind::Trainer { .. } => entry_messages_remaining >= 3 || trainer_exit_active,
        RuntimeBattleKind::Wild { .. } | RuntimeBattleKind::StaticWild { .. } => {
            if battle.battle_type == "BATTLETYPE_TUTORIAL" {
                entry_messages_remaining > 0
            } else {
                entry_messages_remaining >= 2
            }
        }
    };
    if party_hud_visible {
        spawn_battle_hud_borders(
            commands,
            rendered_art,
            asset_root,
            images,
            true,
            true,
            true,
        )?;
        spawn_battle_party_balls(
            commands,
            snapshot,
            battle,
            rendered_art,
            asset_root,
            images,
        )?;
    } else if !trainer_enemy_pending {
        spawn_battle_hud_borders(
            commands,
            rendered_art,
            asset_root,
            images,
            true,
            player_pokemon_visible,
            false,
        )?;
    }
    if trainer_enemy_pending {
        return Ok(());
    }
    if !matches!(battle.kind, RuntimeBattleKind::Trainer { .. })
        && snapshot
            .progression
            .pokedex_caught_species
            .contains(&battle.enemy_pokemon.species.id)
    {
        spawn_battle_caught_icon(commands, rendered_art, asset_root, images);
    }
    spawn_battle_hud_side(
        commands,
        rendered_art,
        asset_root,
        images,
        &battle.enemy_pokemon,
        BattleHpSide::Enemy,
        hp_tween.map(|tween| tween.enemy_pixels),
        None,
    )?;
    if !player_pokemon_visible {
        return Ok(());
    }
    let active_index = battle
        .active_player_party_index
        .context("visible player battle HUD requires an active party slot")?;
    let slot = snapshot
        .party
        .slots
        .iter()
        .find(|slot| slot.index == active_index)
        .with_context(|| format!("active player party slot {active_index} is absent from the HUD"))?;
    let mut display_pokemon = slot.pokemon.clone();
    if let Some(tween) = exp_tween {
        display_pokemon.level = tween.level;
    }
    spawn_battle_hud_side(
        commands,
        rendered_art,
        asset_root,
        images,
        &display_pokemon,
        BattleHpSide::Player,
        hp_tween.map(|tween| tween.player_pixels),
        hp_tween.map(|tween| tween.player_hp),
    )?;
    spawn_battle_exp_bar(
        commands,
        &display_pokemon,
        growth_rates,
        rendered_art,
        asset_root,
        images,
        exp_tween.map(|tween| tween.pixels),
    )?;
    Ok(())
}

fn spawn_battle_party_balls(
    commands: &mut Commands,
    snapshot: &RuntimeShellSnapshot,
    battle: &crate::RuntimeBattleSnapshot,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) -> Result<()> {
    let frames = battle_party_ball_frames(rendered_art, asset_root, images)?;
    let tile_for = |pokemon: Option<&crate::core::models::Pokemon>| match pokemon {
        None => 3,
        Some(pokemon) if pokemon.hp == 0 => 2,
        Some(pokemon) if battle_status_token(pokemon.status.as_deref()).is_some() => 1,
        Some(_) => 0,
    };
    let mut spawn = |frame_index: usize, tile_x: f32, tile_y: f32| {
        let frame = &frames[frame_index];
        let (x, y) = battle_hud_tile_origin(tile_x, tile_y);
        commands.spawn((
            SpriteBundle {
                texture: frame.handle.clone(),
                sprite: Sprite { custom_size: Some(frame.size), ..default() },
                transform: Transform::from_xyz(x, y, 3.75),
                ..default()
            },
            BattleHudMarker,
            BattleCommandMarker,
        ));
    };
    for index in 0..6 {
        spawn(
            tile_for(snapshot.party.slots.get(index).map(|slot| &slot.pokemon)),
            11.0 + index as f32,
            10.0,
        );
    }
    if matches!(battle.kind, RuntimeBattleKind::Trainer { .. }) {
        for index in 0..6 {
            spawn(
                tile_for(battle.enemy_party.get(index)),
                8.0 - index as f32,
                2.0,
            );
        }
    }
    Ok(())
}

fn battle_party_ball_frames<'a>(
    rendered_art: &'a mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) -> Result<&'a [SpriteFrame; 4]> {
    if rendered_art.battle_party_ball_cache.is_none()
        && rendered_art.battle_party_ball_error.is_none()
    {
        let loaded = (|| -> Result<[SpriteFrame; 4]> {
            let path = asset_root.runtime_assets().join("gfx/battle/balls.2bpp");
            let data = std::fs::read(&path)
                .with_context(|| format!("read battle party-ball graphics {}", path.display()))?;
            if data.len() != 4 * 16 {
                anyhow::bail!(
                    "battle party-ball graphics {} must contain four tiles, found {} bytes",
                    path.display(),
                    data.len()
                );
            }
            let palette = load_battle_anim_palette(asset_root, "yellow")?;
            let mut load = |tile_index: usize| {
                let tile = &data[tile_index * 16..tile_index * 16 + 16];
                let mut pixels = vec![0_u8; SOURCE_TILE_SIZE * SOURCE_TILE_SIZE * 4];
                for row in 0..SOURCE_TILE_SIZE {
                    let lo = tile[row * 2];
                    let hi = tile[row * 2 + 1];
                    for col in 0..SOURCE_TILE_SIZE {
                        let bit = 1 << (7 - col);
                        let level = ((hi & bit != 0) as u8) << 1 | (lo & bit != 0) as u8;
                        if level == 0 {
                            continue;
                        }
                        let offset = (row * SOURCE_TILE_SIZE + col) * 4;
                        pixels[offset..offset + 4]
                            .copy_from_slice(&palette[usize::from(level)]);
                    }
                }
                let mut image = Image::new(
                    Extent3d {
                        width: SOURCE_TILE_SIZE as u32,
                        height: SOURCE_TILE_SIZE as u32,
                        depth_or_array_layers: 1,
                    },
                    TextureDimension::D2,
                    pixels,
                    TextureFormat::Rgba8UnormSrgb,
                    RenderAssetUsages::default(),
                );
                image.sampler = ImageSampler::nearest();
                SpriteFrame { handle: images.add(image), size: Vec2::splat(TILE_SIZE) }
            };
            Ok([load(0), load(1), load(2), load(3)])
        })();
        match loaded {
            Ok(frames) => rendered_art.battle_party_ball_cache = Some(frames),
            Err(error) => rendered_art.battle_party_ball_error = Some(error.to_string()),
        }
    }
    rendered_art.battle_party_ball_cache.as_ref().with_context(|| {
        rendered_art
            .battle_party_ball_error
            .clone()
            .unwrap_or_else(|| "battle party-ball art is unavailable".to_string())
    })
}

fn spawn_battle_hud_borders(
    commands: &mut Commands,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
    enemy_visible: bool,
    player_visible: bool,
    player_party_icons: bool,
) -> Result<()> {
    let player_party_corner = if player_party_icons {
        Some(
            battle_exp_bar_tiles(rendered_art, asset_root, images)?
                .get(&0x5c)
                .context("battle party HUD corner tile $5c was not loaded")?
                .clone(),
        )
    } else {
        None
    };
    let mut tiles = battle_hud_border_tiles(rendered_art, asset_root, images)?.clone();
    if let Some(frame) = player_party_corner {
        tiles.insert(0x5c, frame);
    }
    let mut place = |tile_id: u8, tile_x: f32, tile_y: f32| -> Result<()> {
        let frame = tiles
            .get(&tile_id)
            .with_context(|| format!("battle HUD border tile ${tile_id:02x} was not loaded"))?;
        let (x, y) = battle_hud_tile_origin(tile_x, tile_y);
        commands.spawn((
            SpriteBundle {
                texture: frame.handle.clone(),
                sprite: Sprite {
                    custom_size: Some(frame.size),
                    ..default()
                },
                transform: Transform::from_xyz(x, y, 3.5),
                ..default()
            },
            BattleHudMarker,
            BattleCommandMarker,
        ));
        Ok(())
    };

    if enemy_visible {
        // DrawEnemyHUD: left elbow, lower-left corner, horizontal run, lower-right corner.
        place(0x6d, 1.0, 2.0)?;
        place(0x74, 1.0, 3.0)?;
        for x in 2..10 {
            place(0x76, x as f32, 3.0)?;
        }
        place(0x78, 10.0, 3.0)?;
    }

    if player_visible {
        // DrawPlayerHUD starts its border at (18,10); the HP bar contributes
        // its own distinct end tile at (18,9).
        place(0x73, 18.0, 10.0)?;
        place(if player_party_icons { 0x5c } else { 0x77 }, 18.0, 11.0)?;
        for x in 10..18 {
            place(0x76, x as f32, 11.0)?;
        }
        place(0x6f, 9.0, 11.0)?;
    }
    Ok(())
}

fn battle_hud_border_tiles<'a>(
    rendered_art: &'a mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) -> Result<&'a HashMap<u8, SpriteFrame>> {
    if rendered_art.battle_hud_border_cache.is_none()
        && rendered_art.battle_hud_border_error.is_none()
    {
        let loaded = (|| -> Result<HashMap<u8, SpriteFrame>> {
            let battle_root = asset_root.runtime_assets().join("gfx/battle");
            let sources = [
                ("enemy_hp_bar_border.png", 0x6cu8, 4u32),
                ("hp_exp_bar_border.png", 0x73u8, 6u32),
            ];
            let mut result = HashMap::new();
            for (name, start_tile, tile_count) in sources {
                let path = battle_root.join(name);
                let source = image::open(&path)
                    .with_context(|| format!("decode battle HUD PNG {}", path.display()))?
                    .to_rgba8();
                let (width, height) = source.dimensions();
                if width != tile_count * SOURCE_TILE_SIZE as u32
                    || height != SOURCE_TILE_SIZE as u32
                {
                    anyhow::bail!(
                        "battle HUD PNG {} must be {}x{}, found {}x{}",
                        path.display(),
                        tile_count * SOURCE_TILE_SIZE as u32,
                        SOURCE_TILE_SIZE,
                        width,
                        height
                    );
                }
                for tile_offset in 0..tile_count {
                    let mut pixels = vec![0; (SOURCE_TILE_SIZE * SOURCE_TILE_SIZE * 4) as usize];
                    for y in 0..SOURCE_TILE_SIZE as u32 {
                        for x in 0..SOURCE_TILE_SIZE as u32 {
                            let pixel = source.get_pixel(tile_offset * SOURCE_TILE_SIZE as u32 + x, y);
                            let output = ((y * SOURCE_TILE_SIZE as u32 + x) * 4) as usize;
                            let transparent = pixel[0] > 240 && pixel[1] > 240 && pixel[2] > 240;
                            if !transparent {
                                pixels[output] = pixel[0];
                                pixels[output + 1] = pixel[1];
                                pixels[output + 2] = pixel[2];
                                pixels[output + 3] = 255;
                            }
                        }
                    }
                    let mut image = Image::new(
                        Extent3d {
                            width: SOURCE_TILE_SIZE as u32,
                            height: SOURCE_TILE_SIZE as u32,
                            depth_or_array_layers: 1,
                        },
                        TextureDimension::D2,
                        pixels,
                        TextureFormat::Rgba8UnormSrgb,
                        RenderAssetUsages::default(),
                    );
                    image.sampler = ImageSampler::nearest();
                    result.insert(
                        start_tile + tile_offset as u8,
                        SpriteFrame {
                            handle: images.add(image),
                            size: Vec2::splat(TILE_SIZE),
                        },
                    );
                }
            }
            Ok(result)
        })();
        match loaded {
            Ok(tiles) => rendered_art.battle_hud_border_cache = Some(tiles),
            Err(error) => rendered_art.battle_hud_border_error = Some(error.to_string()),
        }
    }
    rendered_art
        .battle_hud_border_cache
        .as_ref()
        .with_context(|| rendered_art.battle_hud_border_error.clone().unwrap_or_else(|| "battle HUD border art is unavailable".to_string()))
}

fn spawn_battle_caught_icon(
    commands: &mut Commands,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) {
    // The source caught marker is literal font tile $5d.  The bitmap-font map
    // exposes that tile through the canonical <TRAINER> private-use glyph.
    spawn_battle_hud_bitmap_text(
        commands,
        rendered_art,
        asset_root,
        images,
        "\u{e103}",
        1.0,
        1.0,
        3.7,
    );
}

fn spawn_battle_exp_bar(
    commands: &mut Commands,
    pokemon: &crate::core::models::pokemon::Pokemon,
    growth_rates: &crate::core::systems::experience::GrowthRateCatalog,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
    fill_override: Option<u16>,
) -> Result<()> {
    const EXP_BAR_PIXELS: i32 = 64;
    let fill_pixels = if let Some(fill_pixels) = fill_override {
        i32::from(fill_pixels)
    } else if pokemon.level >= 100 {
        0
    } else {
        let level = pokemon.level.clamp(1, 99);
        let current_level_exp = crate::core::systems::experience::calculate_experience(
            growth_rates,
            &pokemon.species.growth_rate,
            level,
        )?;
        let next_level_exp = crate::core::systems::experience::calculate_experience(
            growth_rates,
            &pokemon.species.growth_rate,
            level + 1,
        )?;
        let span = (next_level_exp - current_level_exp).max(1);
        let capped_exp = pokemon.experience.clamp(current_level_exp, next_level_exp);
        let remaining = (next_level_exp - capped_exp).max(0);
        EXP_BAR_PIXELS - ((remaining * EXP_BAR_PIXELS) / span).clamp(0, EXP_BAR_PIXELS)
    };
    let tiles = battle_exp_bar_tiles(rendered_art, asset_root, images)?;
    let full_tiles = fill_pixels / SOURCE_TILE_SIZE as i32;
    let remainder = fill_pixels % SOURCE_TILE_SIZE as i32;
    for index in 0..8 {
        let tile_id = if index < full_tiles {
            0x6a
        } else if index == full_tiles && remainder > 0 {
            0x54 + remainder as u8
        } else {
            0x62
        };
        let frame = tiles
            .get(&tile_id)
            .with_context(|| format!("battle EXP tile ${tile_id:02x} was not loaded"))?;
        let tile_x = 17.0 - index as f32;
        let (x, y) = battle_hud_tile_origin(tile_x, 11.0);
        commands.spawn((
            SpriteBundle {
                texture: frame.handle.clone(),
                sprite: Sprite { custom_size: Some(frame.size), ..default() },
                transform: Transform::from_xyz(x, y, 3.62),
                ..default()
            },
            BattleHudMarker,
            BattleCommandMarker,
        ));
    }
    Ok(())
}

fn battle_exp_bar_tiles<'a>(
    rendered_art: &'a mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) -> Result<&'a HashMap<u8, SpriteFrame>> {
    if rendered_art.battle_exp_bar_cache.is_none()
        && rendered_art.battle_exp_bar_error.is_none()
    {
        let loaded = (|| -> Result<HashMap<u8, SpriteFrame>> {
            let assets = asset_root.runtime_assets();
            let exp = std::fs::read(assets.join("gfx/battle/expbar.2bpp"))
                .context("read battle EXP partial-tile graphics")?;
            let battle_font = std::fs::read(assets.join("gfx/font/font_battle_extra.2bpp"))
                .context("read battle HP/EXP template graphics")?;
            let mut result = HashMap::new();
            for (tile_id, data, source_index) in [
                (0x62_u8, battle_font.as_slice(), 2_usize),
                (0x6a_u8, battle_font.as_slice(), 10_usize),
            ]
            .into_iter()
            .chain((0..8).map(|index| (0x55 + index as u8, exp.as_slice(), index)))
            {
                let offset = source_index * 16;
                let tile = data
                    .get(offset..offset + 16)
                    .with_context(|| format!("battle EXP source tile {source_index} is missing"))?;
                let mut pixels = vec![0_u8; SOURCE_TILE_SIZE * SOURCE_TILE_SIZE * 4];
                for row in 0..SOURCE_TILE_SIZE {
                    let lo = tile[row * 2];
                    let hi = tile[row * 2 + 1];
                    for col in 0..SOURCE_TILE_SIZE {
                        let bit = 1 << (7 - col);
                        let level = ((hi & bit != 0) as u8) << 1 | (lo & bit != 0) as u8;
                        if level == 0 {
                            continue;
                        }
                        let color = match level {
                            1 => [247, 214, 123],
                            2 => [33, 140, 255],
                            _ => [0, 0, 0],
                        };
                        let target = (row * SOURCE_TILE_SIZE + col) * 4;
                        pixels[target..target + 3].copy_from_slice(&color);
                        pixels[target + 3] = 255;
                    }
                }
                let mut image = Image::new(
                    Extent3d {
                        width: SOURCE_TILE_SIZE as u32,
                        height: SOURCE_TILE_SIZE as u32,
                        depth_or_array_layers: 1,
                    },
                    TextureDimension::D2,
                    pixels,
                    TextureFormat::Rgba8UnormSrgb,
                    RenderAssetUsages::default(),
                );
                image.sampler = ImageSampler::nearest();
                result.insert(
                    tile_id,
                    SpriteFrame {
                        handle: images.add(image),
                        size: Vec2::splat(TILE_SIZE),
                    },
                );
            }
            Ok(result)
        })();
        match loaded {
            Ok(tiles) => rendered_art.battle_exp_bar_cache = Some(tiles),
            Err(error) => rendered_art.battle_exp_bar_error = Some(error.to_string()),
        }
    }
    rendered_art.battle_exp_bar_cache.as_ref().with_context(|| {
        rendered_art
            .battle_exp_bar_error
            .clone()
            .unwrap_or_else(|| "battle EXP bar art is unavailable".to_string())
    })
}

fn spawn_battle_hud_side(
    commands: &mut Commands,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
    pokemon: &crate::core::models::pokemon::Pokemon,
    side: BattleHpSide,
    hp_pixel_override: Option<u16>,
    hp_value_override: Option<u16>,
) -> Result<()> {
    let (
        name_tile_x,
        name_tile_y,
        level_tile_x,
        level_tile_y,
        status_tile_x,
        status_tile_y,
        hp_tile_x,
        hp_tile_y,
    ) = match side {
        BattleHpSide::Enemy => (1.0, 0.0, 6.0, 1.0, 6.0, 1.0, 2.0, 2.0),
        BattleHpSide::Player => (10.0, 7.0, 14.0, 8.0, 14.0, 8.0, 10.0, 9.0),
    };
    spawn_battle_hud_bitmap_text(
        commands,
        rendered_art,
        asset_root,
        images,
        &compact_scene_label(&pokemon.nickname, 10),
        name_tile_x,
        name_tile_y,
        3.7,
    );
    let status = battle_status_token(pokemon.status.as_deref());
    if let Some(raw_status) = pokemon.status.as_deref() {
        anyhow::ensure!(
            status.is_some()
                || ["", "OK", "NONE", "HEALTHY", "CONFUSION", "CNF"]
                    .iter()
                    .any(|token| raw_status.eq_ignore_ascii_case(token)),
            "battle HUD has unknown status {raw_status} for {}",
            pokemon.nickname
        );
    }
    let gender = crate::core::battle::turn::battle_pokemon_gender(pokemon);
    let level_or_status = status
        .map(str::to_string)
        .unwrap_or_else(|| format!("\u{e10a}{}", pokemon.level));
    // PrintPlayerHUD/DrawEnemyHUD decrement the level destination for a
    // genderless battler. Status text keeps the fixed three-tile position.
    let (level_or_status_tile_x, level_or_status_tile_y) = if status.is_some() {
        (status_tile_x, status_tile_y)
    } else if gender.is_none() {
        (level_tile_x - 1.0, level_tile_y)
    } else {
        (level_tile_x, level_tile_y)
    };
    spawn_battle_hud_bitmap_text(
        commands,
        rendered_art,
        asset_root,
        images,
        &level_or_status,
        level_or_status_tile_x,
        level_or_status_tile_y,
        3.7,
    );
    if let Some(gender) = gender {
        let (gender_tile_x, gender_tile_y) = match side {
            BattleHpSide::Enemy => (9.0, 1.0),
            BattleHpSide::Player => (17.0, 8.0),
        };
        let glyph = match gender {
            crate::core::battle::turn::BattlePokemonGender::Male => "♂",
            crate::core::battle::turn::BattlePokemonGender::Female => "♀",
        };
        spawn_battle_hud_bitmap_text(
            commands,
            rendered_art,
            asset_root,
            images,
            glyph,
            gender_tile_x,
            gender_tile_y,
            3.7,
        );
    }
    spawn_battle_hud_hp_bar(
        commands,
        rendered_art,
        asset_root,
        images,
        hp_tile_x,
        hp_tile_y,
        pokemon.hp,
        pokemon.max_hp,
        side,
        hp_pixel_override,
    )?;
    if side == BattleHpSide::Player {
        let displayed_hp = hp_value_override.unwrap_or(pokemon.hp);
        spawn_battle_hud_bitmap_text(
            commands,
            rendered_art,
            asset_root,
            images,
            &format!("{:>3}/{:>3}", displayed_hp.min(999), pokemon.max_hp.min(999)),
            hp_tile_x + 1.0,
            hp_tile_y + 1.0,
            3.7,
        );
    }
    Ok(())
}

fn spawn_battle_hud_bitmap_text(
    commands: &mut Commands,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
    text: &str,
    tile_x: f32,
    tile_y: f32,
    z: f32,
) {
    let (x, y) = battle_hud_tile_origin(tile_x, tile_y);
    for (index, frame) in bitmap_text_frames(rendered_art, asset_root, images, text)
        .into_iter()
        .enumerate()
    {
        commands.spawn((
            SpriteBundle {
                texture: frame.handle,
                sprite: Sprite {
                    custom_size: Some(frame.size),
                    ..default()
                },
                transform: Transform::from_xyz(x + index as f32 * BITMAP_FONT_ADVANCE, y, z),
                ..default()
            },
            BattleHudMarker,
            BattleCommandMarker,
        ));
    }
}

fn spawn_battle_hud_hp_bar(
    commands: &mut Commands,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
    tile_x: f32,
    tile_y: f32,
    hp: u16,
    max_hp: u16,
    side: BattleHpSide,
    hp_pixel_override: Option<u16>,
) -> Result<()> {
    let tiles = battle_hp_bar_tiles(rendered_art, asset_root, images)?;
    let fill_pixels = hp_pixel_override
        .unwrap_or_else(|| battle_hud_hp_pixels(hp, max_hp))
        .min(BATTLE_HUD_HP_BAR_LENGTH_PX);
    // HPBarAnim_PaletteUpdate passes the currently drawn pixel length to
    // SetHPPal.  The authoritative HP value is already committed while the
    // bar drains, so deriving the palette from `hp` changes color too early.
    let zone = visible_hp_zone(fill_pixels);
    for (offset, tile_id) in [0x60_u8, 0x61].into_iter().enumerate() {
        let frame = tiles
            .get(&(tile_id, zone))
            .with_context(|| format!("battle HP label tile ${tile_id:02x} was not loaded"))?;
        let (x, y) = battle_hud_tile_origin(tile_x + offset as f32, tile_y);
        commands.spawn((
            SpriteBundle {
                texture: frame.handle.clone(),
                sprite: Sprite { custom_size: Some(frame.size), ..default() },
                transform: Transform::from_xyz(x, y, 3.7),
                ..default()
            },
            BattleHudMarker,
            BattleCommandMarker,
        ));
    }
    let full_tiles = fill_pixels / SOURCE_TILE_SIZE as u16;
    let remainder = fill_pixels % SOURCE_TILE_SIZE as u16;
    for index in 0..BATTLE_HUD_HP_BAR_LENGTH_TILES as u16 {
        let tile_id = if index < full_tiles {
            0x6a
        } else if index == full_tiles && remainder > 0 {
            0x62 + remainder as u8
        } else {
            0x62
        };
        let frame = tiles.get(&(tile_id, zone)).with_context(|| {
            format!(
                "battle HP fill tile ${tile_id:02x} for palette zone {zone} was not loaded"
            )
        })?;
        let (x, y) = battle_hud_tile_origin(tile_x + 2.0 + index as f32, tile_y);
        commands.spawn((
            SpriteBundle {
                texture: frame.handle.clone(),
                sprite: Sprite { custom_size: Some(frame.size), ..default() },
                transform: Transform::from_xyz(x, y, 3.62),
                ..default()
            },
            BattleHudMarker,
            BattleCommandMarker,
        ));
    }
    let end_tile = if side == BattleHpSide::Player { 0x6c } else { 0x6b };
    let frame = tiles.get(&(end_tile, zone)).with_context(|| {
        format!(
            "battle HP end tile ${end_tile:02x} for palette zone {zone} was not loaded"
        )
    })?;
    let (x, y) = battle_hud_tile_origin(
        tile_x + 2.0 + BATTLE_HUD_HP_BAR_LENGTH_TILES,
        tile_y,
    );
    commands.spawn((
        SpriteBundle {
            texture: frame.handle.clone(),
            sprite: Sprite { custom_size: Some(frame.size), ..default() },
            transform: Transform::from_xyz(x, y, 3.62),
            ..default()
        },
        BattleHudMarker,
        BattleCommandMarker,
    ));
    Ok(())
}

fn battle_hud_tile_origin(tile_x: f32, tile_y: f32) -> (f32, f32) {
    (
        PLAYFIELD_LEFT + tile_x * TILE_SIZE + (SOURCE_TILE_SIZE as f32 * BATTLE_HUD_SCALE * 0.5),
        PLAYFIELD_TOP - tile_y * TILE_SIZE - (SOURCE_TILE_SIZE as f32 * BATTLE_HUD_SCALE * 0.5),
    )
}

fn battle_hp_bar_tiles<'a>(
    rendered_art: &'a mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) -> Result<&'a HashMap<(u8, u8), SpriteFrame>> {
    if rendered_art.battle_hp_bar_cache.is_none()
        && rendered_art.battle_hp_bar_error.is_none()
    {
        let loaded = (|| -> Result<HashMap<(u8, u8), SpriteFrame>> {
            let assets = asset_root.runtime_assets();
            let battle_font = std::fs::read(assets.join("gfx/font/font_battle_extra.2bpp"))
                .context("read battle HP tile graphics")?;
            let player_end = std::fs::read(assets.join("gfx/battle/enemy_hp_bar_border.1bpp"))
                .context("read player HP end tile graphics")?;
            let mut result = HashMap::new();
            for zone in 0..=2_u8 {
                let fill = match zone {
                    2 => [0, 189, 0],
                    1 => [255, 173, 0],
                    _ => [255, 0, 0],
                };
                let tile_ids = 0x60_u8..=0x6c;
                for tile_id in tile_ids {
                    let (tile, one_bpp) = if tile_id == 0x6c {
                        (player_end.get(0..8).context("player HP end tile is missing")?, true)
                    } else {
                        let source_index = usize::from(tile_id - 0x60);
                        let offset = source_index * 16;
                        (
                            battle_font
                                .get(offset..offset + 16)
                                .with_context(|| format!("battle HP tile ${tile_id:02x} is missing"))?,
                            false,
                        )
                    };
                    let mut pixels = vec![0_u8; SOURCE_TILE_SIZE * SOURCE_TILE_SIZE * 4];
                    for row in 0..SOURCE_TILE_SIZE {
                        let lo = tile[if one_bpp { row } else { row * 2 }];
                        let hi = if one_bpp { 0 } else { tile[row * 2 + 1] };
                        for col in 0..SOURCE_TILE_SIZE {
                            let bit = 1 << (7 - col);
                            let level = ((hi & bit != 0) as u8) << 1 | (lo & bit != 0) as u8;
                            if level == 0 {
                                continue;
                            }
                            let color = match level {
                                1 => [247, 214, 123],
                                2 => fill,
                                _ => [0, 0, 0],
                            };
                            let target = (row * SOURCE_TILE_SIZE + col) * 4;
                            pixels[target..target + 3].copy_from_slice(&color);
                            pixels[target + 3] = 255;
                        }
                    }
                    let mut image = Image::new(
                        Extent3d {
                            width: SOURCE_TILE_SIZE as u32,
                            height: SOURCE_TILE_SIZE as u32,
                            depth_or_array_layers: 1,
                        },
                        TextureDimension::D2,
                        pixels,
                        TextureFormat::Rgba8UnormSrgb,
                        RenderAssetUsages::default(),
                    );
                    image.sampler = ImageSampler::nearest();
                    result.insert(
                        (tile_id, zone),
                        SpriteFrame {
                            handle: images.add(image),
                            size: Vec2::splat(TILE_SIZE),
                        },
                    );
                }
            }
            Ok(result)
        })();
        match loaded {
            Ok(tiles) => rendered_art.battle_hp_bar_cache = Some(tiles),
            Err(error) => rendered_art.battle_hp_bar_error = Some(error.to_string()),
        }
    }
    rendered_art.battle_hp_bar_cache.as_ref().with_context(|| {
        rendered_art
            .battle_hp_bar_error
            .clone()
            .unwrap_or_else(|| "battle HP bar art is unavailable".to_string())
    })
}

fn battle_hud_hp_pixels(hp: u16, max_hp: u16) -> u16 {
    if max_hp == 0 || hp == 0 {
        return 0;
    }
    let clamped_hp = hp.min(max_hp);
    let mut product = u32::from(clamped_hp) * u32::from(BATTLE_HUD_HP_BAR_LENGTH_PX);
    let mut divisor = u32::from(max_hp);
    // ComputeHPBarPixels has only an eight-bit hardware divisor. When the
    // maximum HP has a high byte, the cartridge truncates both operands by
    // two bits before dividing.
    if max_hp > u16::from(u8::MAX) {
        product >>= 2;
        divisor >>= 2;
    }
    let pixels = (product / divisor) as u16;
    pixels.max(1).min(BATTLE_HUD_HP_BAR_LENGTH_PX)
}

fn advance_visible_hp_pixels(current: &mut u16, target: u16, frames_until_step: &mut u8) -> bool {
    if *current == target {
        return false;
    }
    if *frames_until_step > 0 {
        *frames_until_step -= 1;
        return false;
    }
    // HPBarAnim_BGMapUpdate holds an ordinary player/enemy redraw for two
    // LCD frames total. This countdown is consumed on subsequent updates, so
    // one deferred update produces the source two-frame pixel spacing; using
    // 2 here made every pixel last three frames.
    *frames_until_step = 1;
    if *current < target {
        *current += 1;
    } else {
        *current -= 1;
    }
    true
}

fn advance_visible_player_hp_number(tween: &mut VisibleBattleHpTween) {
    if tween.player_pixels == tween.player_target_pixels {
        tween.player_hp = tween.player_target_hp;
        return;
    }
    if tween.player_hp < tween.player_target_hp {
        while tween.player_hp < tween.player_target_hp {
            tween.player_hp += 1;
            if battle_hud_hp_pixels(tween.player_hp, tween.player_max_hp) >= tween.player_pixels {
                break;
            }
        }
    } else {
        while tween.player_hp > tween.player_target_hp {
            tween.player_hp -= 1;
            if battle_hud_hp_pixels(tween.player_hp, tween.player_max_hp) <= tween.player_pixels {
                break;
            }
        }
    }
}

fn visible_battle_hp_tween_active(tween: &VisibleBattleHpTween) -> bool {
    tween.player_pixels != tween.player_target_pixels
        || tween.enemy_pixels != tween.enemy_target_pixels
}

fn visible_hp_zone(pixels: u16) -> u8 {
    let red_threshold =
        ((u32::from(BATTLE_HUD_HP_BAR_LENGTH_PX) * 21) / 100) as u16;
    let yellow_threshold = BATTLE_HUD_HP_BAR_LENGTH_PX / 2;
    if pixels < red_threshold {
        0
    } else if pixels < yellow_threshold {
        1
    } else {
        2
    }
}

fn battle_hud_hp_color(fill_pixels: u16) -> Color {
    match visible_hp_zone(fill_pixels) {
        2 => Color::rgba(0.0, 0.74, 0.0, 0.96),
        1 => Color::rgba(0.98, 0.72, 0.0, 0.96),
        _ => Color::rgba(0.96, 0.0, 0.0, 0.96),
    }
}

fn battle_status_token(status: Option<&str>) -> Option<&'static str> {
    let status = status?;
    if ["POISON", "BAD_POISON", "BAD POISON", "PSN"]
        .iter()
        .any(|candidate| status.eq_ignore_ascii_case(candidate))
    {
        Some("PSN")
    } else if ["SLEEP", "SLP"]
        .iter()
        .any(|candidate| status.eq_ignore_ascii_case(candidate))
    {
        Some("SLP")
    } else if ["PARALYSIS", "PAR"]
        .iter()
        .any(|candidate| status.eq_ignore_ascii_case(candidate))
    {
        Some("PAR")
    } else if ["BURN", "BRN"]
        .iter()
        .any(|candidate| status.eq_ignore_ascii_case(candidate))
    {
        Some("BRN")
    } else if ["FREEZE", "FRZ"]
        .iter()
        .any(|candidate| status.eq_ignore_ascii_case(candidate))
    {
        Some("FRZ")
    } else {
        None
    }
}

fn party_status_token(pokemon: &crate::core::models::pokemon::Pokemon) -> &'static str {
    if pokemon.hp == 0 {
        return "FNT";
    }
    let Some(status) = pokemon.status.as_deref() else {
        return "OK";
    };
    battle_status_token(Some(status)).unwrap_or_else(|| {
        if ["CONFUSION", "CNF"]
            .iter()
            .any(|candidate| status.eq_ignore_ascii_case(candidate))
        {
            "CNF"
        } else {
            "OK"
        }
    })
}

fn spawn_visible_move_animation_overlay(
    commands: &mut Commands,
    runtime_shell: &BevyRuntimeShell,
) {
    if runtime_shell.visible_move_animations.front().is_none()
        && let Some(send_out) = runtime_shell.visible_send_out_animation.as_ref()
        && send_out.shiny
        && send_out.frame >= VisibleSendOutAnimation::NORMAL_FRAMES
    {
        let age = send_out.frame - VisibleSendOutAnimation::NORMAL_FRAMES;
        if age < 3 {
            commands.spawn((
                SpriteBundle {
                    sprite: Sprite {
                        color: Color::rgba_u8(255, 255, 255, 96),
                        custom_size: Some(Vec2::new(PLAYFIELD_WIDTH, PLAYFIELD_HEIGHT)),
                        ..default()
                    },
                    transform: Transform::from_xyz(0.0, 0.0, 3.48),
                    ..default()
                },
                BattleCommandMarker,
            ));
        }
        return;
    }
    let Some(animation) = runtime_shell
        .visible_move_animations
        .front()
        .filter(|animation| animation.started)
    else {
        return;
    };
    let mut overlay = None;
    for effect in animation
        .bg_events
        .iter()
        .filter(|effect| !effect.incremented && effect.frame <= animation.frame)
    {
        if animation.bg_events.iter().any(|candidate| {
            !candidate.incremented
                && candidate.effect_id == effect.effect_id
                && candidate.frame > effect.frame
                && candidate.frame <= animation.frame
        }) {
            continue;
        }
        let reset_frame = animation
            .bg_events
            .iter()
            .filter(|candidate| {
                candidate.incremented
                    && candidate.effect_id == effect.effect_id
                    && candidate.frame >= effect.frame
                    && candidate.frame <= animation.frame
            })
            .map(|candidate| candidate.frame)
            .max()
            .unwrap_or(effect.frame);
        let active_age = animation.frame.saturating_sub(reset_frame);
        let phase_age = animation.frame.saturating_sub(effect.frame);
        let resolved = match effect.effect_id.as_str() {
            "BATTLE_BG_EFFECT_FLASH_INVERTED" | "BATTLE_BG_EFFECT_FLASH_WHITE" => {
                let duration = if effect.duration == 0 { 4 } else { effect.duration };
                let frequency = u16::from(if effect.param == 0 { 2 } else { effect.param });
                if active_age >= duration || (phase_age / frequency) % 2 != 0 {
                    None
                } else {
                    Some((
                        [255_u8, 255, 255],
                        if effect.effect_id == "BATTLE_BG_EFFECT_FLASH_WHITE" { 128 } else { 96 },
                    ))
                }
            }
            "BATTLE_BG_EFFECT_WHITE_HUES" => (active_age < effect.duration.max(1))
                .then_some(([248, 248, 248], 128)),
            "BATTLE_BG_EFFECT_BLACK_HUES" => (active_age < effect.duration.max(1))
                .then_some(([0, 0, 0], 128)),
            "BATTLE_BG_EFFECT_ALTERNATE_HUES" => {
                let duration = if effect.duration == 0 { 4 } else { effect.duration };
                (active_age < duration).then(|| {
                    let colour = if (phase_age / 2) % 2 == 0 {
                        [255, 255, 255]
                    } else {
                        [8, 8, 8]
                    };
                    (colour, 112)
                })
            }
            "BATTLE_BG_EFFECT_CYCLE_OBPALS_GRAY_AND_YELLOW"
            | "BATTLE_BG_EFFECT_CYCLE_MID_OBPALS_GRAY_AND_YELLOW" => {
                let duration = if effect.duration == 0 { 6 } else { effect.duration };
                (active_age < duration).then(|| {
                    let colour = if (phase_age / 3) % 2 == 0 {
                        [192, 192, 192]
                    } else {
                        [255, 232, 120]
                    };
                    (colour, 96)
                })
            }
            "BATTLE_BG_EFFECT_CYCLE_BGPALS_INVERTED" => {
                let duration = if effect.duration == 0 { 4 } else { effect.duration };
                (active_age < duration).then(|| {
                    let colour = if (phase_age / 2) % 2 == 0 {
                        [255, 255, 255]
                    } else {
                        [16, 16, 8]
                    };
                    (colour, 104)
                })
            }
            "BATTLE_BG_EFFECT_CYCLE_MON_LIGHT_DARK_REPEATING" => {
                let duration = if effect.duration == 0 { 6 } else { effect.duration };
                (active_age < duration).then(|| {
                    let colour = if (phase_age / 3) % 2 == 0 {
                        [224, 224, 224]
                    } else {
                        [16, 16, 16]
                    };
                    (colour, 96)
                })
            }
            "BATTLE_BG_EFFECT_ACID_ARMOR" => {
                let duration = if effect.duration == 0 { 4 } else { effect.duration };
                (active_age < duration).then(|| {
                    let colour = if (phase_age / 2) % 2 == 0 {
                        [148, 200, 148]
                    } else {
                        [48, 112, 64]
                    };
                    (colour, 100)
                })
            }
            "BATTLE_BG_EFFECT_START_WATER"
            | "BATTLE_BG_EFFECT_WATER"
            | "BATTLE_BG_EFFECT_END_WATER"
            | "BATTLE_BG_EFFECT_WHIRLPOOL" => {
                let duration = if effect.duration == 0 { 6 } else { effect.duration };
                (active_age < duration).then_some(([28, 84, 160], 112))
            }
            "BATTLE_BG_EFFECT_NIGHT_SHADE" => {
                (active_age < effect.duration.max(1)).then_some(([4, 4, 12], 128))
            }
            "BATTLE_BG_EFFECT_PSYCHIC" => {
                let duration = if effect.duration == 0 { 4 } else { effect.duration };
                (active_age < duration).then(|| {
                    let colour = if (phase_age / 2) % 2 == 0 {
                        [152, 80, 192]
                    } else {
                        [64, 32, 128]
                    };
                    (colour, 120)
                })
            }
            "BATTLE_BG_EFFECT_TELEPORT" => {
                (active_age < effect.duration.max(1)).then_some(([120, 120, 220], 110))
            }
            "BATTLE_BG_EFFECT_RAPID_FLASH" if !animation.bg_events.iter().any(|candidate| {
                candidate.incremented
                    && candidate.effect_id == effect.effect_id
                    && candidate.frame >= effect.frame
                    && candidate.frame <= animation.frame
            }) => {
                ((phase_age / 2) & 1 == 1).then_some(([0, 0, 0], 160))
            }
            "BATTLE_BG_EFFECT_FLASH_MON_REPEATING" if !animation.bg_events.iter().any(|candidate| {
                candidate.incremented
                    && candidate.effect_id == effect.effect_id
                    && candidate.frame >= effect.frame
                    && candidate.frame <= animation.frame
            }) => match (phase_age / 2) & 3 {
                1 => Some(([0, 0, 0], 213)),
                3 => Some(([255, 255, 255], 255)),
                _ => None,
            },
            _ => None,
        };
        if resolved.is_some() {
            overlay = resolved;
        }
    }
    let Some((colour, alpha)) = overlay else {
        return;
    };
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::rgba_u8(colour[0], colour[1], colour[2], alpha),
                custom_size: Some(Vec2::new(PLAYFIELD_WIDTH, PLAYFIELD_HEIGHT)),
                ..default()
            },
            transform: Transform::from_xyz(0.0, 0.0, 3.48),
            ..default()
        },
        BattleCommandMarker,
    ));
}

fn visible_battle_anim_sine(angle: u8, amplitude: u8) -> i32 {
    const WAVE: [u16; 32] = [
        0x0000, 0x0019, 0x0032, 0x004a, 0x0062, 0x0079, 0x008e, 0x00a2,
        0x00b5, 0x00c6, 0x00d5, 0x00e2, 0x00ed, 0x00f5, 0x00fb, 0x00ff,
        0x0100, 0x00ff, 0x00fb, 0x00f5, 0x00ed, 0x00e2, 0x00d5, 0x00c6,
        0x00b5, 0x00a2, 0x008e, 0x0079, 0x0062, 0x004a, 0x0032, 0x0019,
    ];
    let normalized = angle & 0x3f;
    let mut product = 0_u16;
    let mut multiplier = amplitude;
    let mut value = WAVE[usize::from(normalized & 0x1f)];
    while multiplier != 0 {
        if multiplier & 1 != 0 {
            product = product.wrapping_add(value);
        }
        multiplier >>= 1;
        value = value.wrapping_shl(1);
    }
    let magnitude = ((product >> 8) & 0xff) as u8;
    if normalized & 0x20 != 0 {
        i32::from(((!magnitude).wrapping_add(1)) as i8)
    } else {
        i32::from(magnitude as i8)
    }
}

fn visible_battle_anim_frameset<'a>(
    function: &str,
    base: &'a str,
    param: u8,
    age: u16,
    x: i32,
    state: u8,
    state_age: u16,
    y: i32,
    player_move: bool,
) -> (&'a str, u16) {
    match function {
        "BATTLE_ANIM_FUNC_EMBER" if (param >> 4) == 3 => {
            ("BATTLE_ANIM_FRAMESET_FLAMETHROWER", age)
        }
        "BATTLE_ANIM_FUNC_BUBBLE" if age >= 12 => {
            ("BATTLE_ANIM_FRAMESET_PULSING_BUBBLE", age - 12)
        }
        "BATTLE_ANIM_FUNC_ROCK_SMASH" if param & 0x40 != 0 => {
            ("BATTLE_ANIM_FRAMESET_SMALL_ROCK", age)
        }
        "BATTLE_ANIM_FUNC_ROCK_SMASH" => ("BATTLE_ANIM_FRAMESET_BIG_ROCK", age),
        "BATTLE_ANIM_FUNC_SING" => match param {
            1 => ("BATTLE_ANIM_FRAMESET_MUSIC_NOTE_2", age),
            2 => ("BATTLE_ANIM_FRAMESET_MUSIC_NOTE_3", age),
            _ => ("BATTLE_ANIM_FRAMESET_MUSIC_NOTE_1", age),
        },
        "BATTLE_ANIM_FUNC_DIZZY" => {
            let mut runtime_param = param;
            let mut toggles = 0_u16;
            let mut last_toggle_age = 0_u16;
            for tick in 0..=age {
                runtime_param = runtime_param.wrapping_add(2);
                if runtime_param & 0x1f == 0 {
                    toggles = toggles.wrapping_add(1);
                    last_toggle_age = tick;
                }
            }
            let frameset_age = age.saturating_sub(last_toggle_age);
            if base == "BATTLE_ANIM_FRAMESET_IMP" {
                if toggles & 1 == 0 {
                    ("BATTLE_ANIM_FRAMESET_IMP", frameset_age)
                } else {
                    ("BATTLE_ANIM_FRAMESET_IMP_FLIPPED", frameset_age)
                }
            } else if toggles & 1 == 0 {
                ("BATTLE_ANIM_FRAMESET_CHICK_1", frameset_age)
            } else {
                ("BATTLE_ANIM_FRAMESET_CHICK_2", frameset_age)
            }
        }
        "BATTLE_ANIM_FUNC_BITE" => {
            let initial_angle = if param & 0x80 != 0 { 0x30_u8 } else { 0x10 };
            let amplitude = match param & 0x7f {
                0 => 0x10,
                value => value,
            };
            let angle = initial_angle.wrapping_add((age as u8).wrapping_mul(2));
            if visible_battle_anim_sine(angle, amplitude) >= 0 {
                ("BATTLE_ANIM_FRAMESET_BITE_2", 0)
            } else {
                ("BATTLE_ANIM_FRAMESET_BITE_1", 0)
            }
        }
        "BATTLE_ANIM_FUNC_FIRE_BLAST" if param == 7 => {
            let travel_frames = u16::try_from(((0x88 - x).max(0) + 1) / 2).unwrap_or(0);
            let transition_age = travel_frames.saturating_add(1);
            if age < transition_age {
                (base, age)
            } else {
                (
                    "BATTLE_ANIM_FRAMESET_EMBER",
                    age.saturating_sub(transition_age),
                )
            }
        }
        "BATTLE_ANIM_FUNC_FIRE_BLAST" => ("BATTLE_ANIM_FRAMESET_BURNED", age),
        "BATTLE_ANIM_FUNC_RAZOR_LEAF" if state != 0 => {
            ("BATTLE_ANIM_FRAMESET_RAZOR_LEAF_1", state_age)
        }
        "BATTLE_ANIM_FUNC_RAZOR_LEAF" if age >= 17 => {
            let offset = if param & 0x40 != 0 { 32 } else { 0 };
            (
                "BATTLE_ANIM_FRAMESET_RAZOR_LEAF_2",
                age.saturating_sub(17).saturating_add(offset),
            )
        }
        "BATTLE_ANIM_FUNC_LEECH_SEED" if age >= 99 => {
            ("BATTLE_ANIM_FRAMESET_LEECH_SEED_3", age - 99)
        }
        "BATTLE_ANIM_FUNC_LEECH_SEED" if age >= 34 => {
            ("BATTLE_ANIM_FRAMESET_LEECH_SEED_2", age - 34)
        }
        "BATTLE_ANIM_FUNC_PARALYZED" if param & 0x80 != 0 => {
            ("BATTLE_ANIM_FRAMESET_PARALYZED_FLIPPED", age)
        }
        "BATTLE_ANIM_FUNC_AMNESIA" => {
            let phase = (age / 8) % 3;
            let frameset = match phase {
                0 => "BATTLE_ANIM_FRAMESET_AMNESIA_1",
                1 => "BATTLE_ANIM_FRAMESET_AMNESIA_2",
                _ => "BATTLE_ANIM_FRAMESET_AMNESIA_3",
            };
            (frameset, age % 8)
        }
        "BATTLE_ANIM_FUNC_HEAL_BELL_NOTES" => match param {
            1 => ("BATTLE_ANIM_FRAMESET_MUSIC_NOTE_2", age),
            2 => ("BATTLE_ANIM_FRAMESET_MUSIC_NOTE_3", age),
            _ => ("BATTLE_ANIM_FRAMESET_MUSIC_NOTE_1", age),
        },
        "BATTLE_ANIM_FUNC_LOCK_ON_MIND_READER" => {
            let toggles = age.min(40) / 4;
            let frameset = match toggles % 4 {
                0 => "BATTLE_ANIM_FRAMESET_LOCK_ON_1",
                1 => "BATTLE_ANIM_FRAMESET_LOCK_ON_2",
                2 => "BATTLE_ANIM_FRAMESET_LOCK_ON_3",
                _ => "BATTLE_ANIM_FRAMESET_LOCK_ON_4",
            };
            (frameset, age.saturating_sub(toggles.saturating_mul(4)))
        }
        "BATTLE_ANIM_FUNC_SLUDGE" if age >= 13 => {
            ("BATTLE_ANIM_FRAMESET_SLUDGE_BUBBLE_BURST", age - 13)
        }
        "BATTLE_ANIM_FUNC_STRING" => match param {
            0 => ("BATTLE_ANIM_FRAMESET_STRING_SHOT_1", age),
            1 => ("BATTLE_ANIM_FRAMESET_STRING_SHOT_2", age),
            2 => ("BATTLE_ANIM_FRAMESET_STRING_SHOT_3", age),
            _ => (base, age),
        },
        "BATTLE_ANIM_FUNC_WRAP" if state != 0 => match base {
            "BATTLE_ANIM_FRAMESET_BIND_1" => ("BATTLE_ANIM_FRAMESET_BIND_2", state_age),
            "BATTLE_ANIM_FRAMESET_BIND_3" => ("BATTLE_ANIM_FRAMESET_BIND_4", state_age),
            _ => (base, state_age),
        },
        "BATTLE_ANIM_FUNC_WATER_GUN" => {
            let transition_age = u16::try_from((y - 0x2f).max(0)).unwrap_or(0);
            if age < transition_age {
                (base, age)
            } else if age < transition_age.saturating_add(24) {
                (
                    "BATTLE_ANIM_FRAMESET_WATER_GUN_2",
                    age.saturating_sub(transition_age),
                )
            } else {
                (
                    "BATTLE_ANIM_FRAMESET_WATER_GUN_3",
                    age.saturating_sub(transition_age.saturating_add(24)),
                )
            }
        }
        "BATTLE_ANIM_FUNC_SOUND" => {
            let effective_param = if player_move {
                param
            } else {
                (!param).wrapping_add(3)
            };
            let frameset = match effective_param & 3 {
                0 => "BATTLE_ANIM_FRAMESET_SOUND_1",
                1 => "BATTLE_ANIM_FRAMESET_SOUND_2",
                2 => "BATTLE_ANIM_FRAMESET_SOUND_3",
                _ => "BATTLE_ANIM_FRAMESET_SOUND_4",
            };
            (frameset, age)
        }
        "BATTLE_ANIM_FUNC_EGG" if param == 6 && state != 0 => {
            ("BATTLE_ANIM_FRAMESET_EGG_CRACKED_BOTTOM", state_age)
        }
        "BATTLE_ANIM_FUNC_EGG" if param == 6 && age >= 57 => {
            ("BATTLE_ANIM_FRAMESET_EGG_WOBBLE", age - 57)
        }
        "BATTLE_ANIM_FUNC_EGG" if param == 11 && age >= 1 => {
            ("BATTLE_ANIM_FRAMESET_EGG_CRACKED_TOP", age - 1)
        }
        _ => (base, age),
    }
}

fn visible_battle_anim_object_position(
    function: &str,
    x: i32,
    y: i32,
    param: u8,
    age: u16,
    state: u8,
    state_age: u16,
    player_move: bool,
) -> Option<(i32, i32)> {
    let updates = u32::from(age) + 1;
    match function {
        "BATTLE_ANIM_FUNC_NULL" => Some((x & 0xff, y & 0xff)),
        "BATTLE_ANIM_FUNC_USER_TO_TARGET"
        | "BATTLE_ANIM_FUNC_USER_TO_TARGET_DISAPPEAR"
        | "BATTLE_ANIM_FUNC_THROW_TO_TARGET_DISAPPEAR" => {
            let speed = if function == "BATTLE_ANIM_FUNC_THROW_TO_TARGET_DISAPPEAR" {
                6
            } else {
                i32::from(if param == 0 { 2 } else { param })
            };
            let current_x = (x + speed.saturating_mul(updates as i32)) & 0xff;
            if current_x >= 0x84 {
                return None;
            }
            let y_step = (speed / 2).max(1);
            Some((current_x, (y - y_step.saturating_mul(updates as i32)) & 0xff))
        }
        "BATTLE_ANIM_FUNC_WAVE_TO_TARGET" => {
            let speed = i32::from(if param == 0 { 2 } else { param });
            let current_x = (x + speed.saturating_mul(updates as i32)) & 0xff;
            if current_x >= 0x84 {
                return None;
            }
            let base_y = (y - (speed / 2).max(1).saturating_mul(updates as i32)) & 0xff;
            let angle = (updates as u8).wrapping_mul(4);
            Some((current_x, (base_y + visible_battle_anim_sine(angle, 6)) & 0xff))
        }
        "BATTLE_ANIM_FUNC_USER_TO_TARGET_SPIN" => {
            let mut state = 0_u8;
            let mut runtime_param = param;
            let mut angle = 0_u8;
            let mut base_x = x & 0xff;
            let mut x_offset = 0_i32;
            let mut y_offset = 0_i32;
            for _ in 0..updates {
                if state == 0 {
                    state = 1;
                }
                if state == 1 {
                    state = 2;
                    angle = 0;
                }
                if state == 2 {
                    if angle >= 0x40 {
                        let high = runtime_param & 0xf0;
                        if high != 0 {
                            runtime_param = high.wrapping_sub(0x10) | (runtime_param & 0x0f);
                            state = 1;
                            continue;
                        }
                        state = 3;
                        continue;
                    }
                    let step = match runtime_param & 0x0f {
                        0 => 4,
                        value => value,
                    };
                    x_offset = (visible_battle_anim_sine(angle.wrapping_add(0x10), 0x18)
                        - 0x18)
                        >> 1;
                    y_offset = visible_battle_anim_sine(angle, 0x18);
                    angle = angle.wrapping_add(step);
                    continue;
                }
                base_x = (base_x + 4) & 0xff;
                if base_x >= 0xb0 {
                    return None;
                }
            }
            Some(((base_x + x_offset) & 0xff, (y + y_offset) & 0xff))
        }
        "BATTLE_ANIM_FUNC_THROW_TO_TARGET" => {
            let moved = updates.min(u32::try_from(((0x88 - x).max(0) + 1) / 2).unwrap_or(0));
            let base_x = (x + 2 * moved as i32).min(0x88) & 0xff;
            let base_y = (y - moved as i32) & 0xff;
            let angle = 0_u8.wrapping_sub((moved.saturating_sub(1)) as u8);
            Some((base_x, (base_y + visible_battle_anim_sine(angle, param)) & 0xff))
        }
        "BATTLE_ANIM_FUNC_MOVE_IN_CIRCLE" => {
            let start = if param & 0x80 != 0 { 0x20 } else { 0 };
            let amplitude = param & 0x7f;
            let angle = (start as u8).wrapping_add(age as u8);
            Some((
                (x + visible_battle_anim_sine(angle.wrapping_add(0x10), amplitude)) & 0xff,
                (y + visible_battle_anim_sine(angle, amplitude)) & 0xff,
            ))
        }
        "BATTLE_ANIM_FUNC_SHAKE" => {
            if age >= 31 {
                return None;
            }
            let encoded = param & 0x0f;
            let amplitude = i32::from(if encoded == 0 { 2 } else { encoded });
            let offset = if age % 2 == 0 { amplitude } else { -amplitude };
            Some(((x + offset) & 0xff, y & 0xff))
        }
        "BATTLE_ANIM_FUNC_DROP" => {
            let mut angle = 0x30_u8;
            let mut amplitude = 0x48_u8;
            let decrement = param;
            for step in 0..updates {
                let offset = visible_battle_anim_sine(angle, amplitude);
                angle = angle.wrapping_add(1);
                if angle & 0x3f == 0 {
                    angle = 0x20;
                    amplitude = amplitude.saturating_sub(decrement);
                    if amplitude == 0 {
                        return None;
                    }
                }
                if step == updates - 1 {
                    return Some((x & 0xff, (y + offset) & 0xff));
                }
            }
            None
        }
        "BATTLE_ANIM_FUNC_MOVE_UP" => {
            let speed = param;
            let mut offset = 0_u8;
            for step in 0..updates {
                if offset != 0 && offset < 0xd8 {
                    return None;
                }
                offset = offset.wrapping_sub(speed);
                if step == updates - 1 {
                    return Some((x & 0xff, (y + i32::from(offset)) & 0xff));
                }
            }
            None
        }
        "BATTLE_ANIM_FUNC_RAPID_SPIN" => {
            let offset = 0_u8.wrapping_sub((updates as u8).wrapping_mul(4));
            if offset == 0xd0 {
                None
            } else {
                Some((x & 0xff, (y + i32::from(offset)) & 0xff))
            }
        }
        "BATTLE_ANIM_FUNC_ABSORB" => {
            let encoded = param & 0x0f;
            let speed = i32::from(if encoded == 0 { 2 } else { encoded });
            let current_x = (x - speed.saturating_mul(updates as i32)) & 0xff;
            if current_x < 0x30 {
                return None;
            }
            let y_step = (speed / 2).max(1);
            Some((current_x, (y + y_step.saturating_mul(updates as i32)) & 0xff))
        }
        "BATTLE_ANIM_FUNC_ABSORB_CIRCLE" => {
            let mut runtime_param = param;
            let mut radius = 0x40_u8;
            let mut base_x = x & 0xff;
            let mut base_y = y & 0xff;
            let mut x_offset = 0_i32;
            let mut y_offset = 0_i32;
            for _ in 0..updates {
                let angle = runtime_param;
                x_offset = visible_battle_anim_sine(angle.wrapping_add(0x10), radius);
                y_offset = visible_battle_anim_sine(angle, radius);
                runtime_param = runtime_param.wrapping_add(1);
                if runtime_param & 1 == 0 {
                    base_x = (base_x - 1) & 0xff;
                }
                if runtime_param & 3 == 0 {
                    base_y = (base_y + 1) & 0xff;
                }
                radius = if base_x >= 0x5a {
                    radius.saturating_add(1).min(0x60)
                } else {
                    radius.saturating_sub(1)
                };
                if radius == 0 {
                    return None;
                }
            }
            Some(((base_x + x_offset) & 0xff, (base_y + y_offset) & 0xff))
        }
        "BATTLE_ANIM_FUNC_COTTON" => {
            let angle = ((updates as u8) >> 1).wrapping_add(param);
            Some((
                (x + visible_battle_anim_sine(angle.wrapping_add(0x10), 0x18)) & 0xff,
                (y + (visible_battle_anim_sine(angle, 0x18) >> 2)) & 0xff,
            ))
        }
        "BATTLE_ANIM_FUNC_POWDER" => {
            if age >= 112 {
                return None;
            }
            let y_offset = updates / 2;
            let x_offset = if updates % 2 == 1 { 0x10 } else { 0 };
            Some(((x + x_offset) & 0xff, (y + y_offset as i32) & 0xff))
        }
        "BATTLE_ANIM_FUNC_ANCIENT_POWER" => {
            if age >= 32 {
                return None;
            }
            let angle = (age as u8).wrapping_add(1);
            Some((
                x & 0xff,
                (y - visible_battle_anim_sine(angle, param)) & 0xff,
            ))
        }
        "BATTLE_ANIM_FUNC_SPEED_LINE" => {
            let travel = i32::from(age);
            let offset = if param & 0x80 != 0 { -travel } else { travel };
            Some(((x + offset) & 0xff, y & 0xff))
        }
        "BATTLE_ANIM_FUNC_FLOAT_UP" => {
            let angle = updates as u8;
            Some((
                (x + visible_battle_anim_sine(angle, 4)) & 0xff,
                (y - updates as i32) & 0xff,
            ))
        }
        "BATTLE_ANIM_FUNC_RECOVER" => {
            let initial_amplitude = param & 0xf0;
            let prior_decrements = age / 2;
            let amplitude = initial_amplitude.saturating_sub(prior_decrements as u8);
            if amplitude == 0 {
                return None;
            }
            let angle = ((param & 0x0f) << 3).wrapping_add(age as u8);
            Some((
                (x + visible_battle_anim_sine(angle.wrapping_add(0x10), amplitude)) & 0xff,
                (y + visible_battle_anim_sine(angle, amplitude)) & 0xff,
            ))
        }
        "BATTLE_ANIM_FUNC_RAZOR_WIND" => {
            let initial_angle = if param & 0x80 != 0 { 0x20_u8 } else { 0 };
            let angle = initial_angle.wrapping_add((age as u8).wrapping_mul(0x10));
            let amplitude = param & 0x7f;
            Some((
                (x + visible_battle_anim_sine(angle.wrapping_add(0x10), amplitude)) & 0xff,
                (y + visible_battle_anim_sine(angle, amplitude)) & 0xff,
            ))
        }
        "BATTLE_ANIM_FUNC_SPIRAL_DESCENT" => {
            // BattleAnimFunc_SpiralDescent removes on the 328th update,
            // immediately before applying that frame's offsets.
            if age >= 327 {
                return None;
            }
            let angle = age as u8;
            let descent = i32::from(age / 8);
            Some((
                (x + visible_battle_anim_sine(angle.wrapping_add(0x10), 0x18)) & 0xff,
                (y + (visible_battle_anim_sine(angle, 0x18) >> 3) + descent) & 0xff,
            ))
        }
        "BATTLE_ANIM_FUNC_ROCK_SMASH" => {
            if age == 0 {
                return Some((x & 0xff, y & 0xff));
            }
            let angle = 0x40_u8.wrapping_sub((age - 1) as u8);
            if angle < 0x30 {
                return None;
            }
            let next_angle = angle.wrapping_sub(1);
            Some((
                (x + visible_battle_anim_sine(next_angle.wrapping_mul(13), 4)) & 0xff,
                (y + visible_battle_anim_sine(angle, param & 0x3f)) & 0xff,
            ))
        }
        "BATTLE_ANIM_FUNC_SING" => {
            if age == 0 {
                return Some((x & 0xff, y & 0xff));
            }
            let prior_x = (x + 2 * i32::from(age - 1)) & 0xff;
            if prior_x >= 0xb8 {
                return None;
            }
            let angle = 0_u8.wrapping_sub(age as u8);
            Some((
                (prior_x + 2) & 0xff,
                (y - i32::from(age) + visible_battle_anim_sine(angle, 8)) & 0xff,
            ))
        }
        "BATTLE_ANIM_FUNC_PRESENT_SMOKESCREEN" => {
            let mut sprite_x = x & 0xff;
            let mut sprite_y = y & 0xff;
            let mut base_x = None;
            let mut base_y = None;
            let mut angle = 0x34_u8;
            let mut amplitude = 0x10_u8;
            for _ in 0..updates {
                if sprite_x < 0x6c {
                    return None;
                }
                sprite_x = (sprite_x + 2) & 0xff;
                sprite_y = (sprite_y - 1) & 0xff;
                let y_offset = visible_battle_anim_sine(angle, amplitude).abs();
                angle = angle.wrapping_sub(4);
                if angle & 0x1f == 0 {
                    amplitude /= 2;
                }
                let stable_x = *base_x.get_or_insert(sprite_x);
                let stable_y = *base_y.get_or_insert(sprite_y);
                sprite_x = stable_x;
                sprite_y = (stable_y + y_offset) & 0xff;
            }
            Some((sprite_x, sprite_y))
        }
        "BATTLE_ANIM_FUNC_CLAMP_ENCORE" => {
            if age == 0 {
                return Some((x & 0xff, y & 0xff));
            }
            let angle = ((age - 1) as u8).wrapping_mul(4);
            let amplitude = param & 0x7f;
            Some((
                (x + visible_battle_anim_sine(angle, amplitude)) & 0xff,
                (y + visible_battle_anim_sine(
                    angle.wrapping_add(0x40),
                    amplitude / 2,
                )) & 0xff,
            ))
        }
        "BATTLE_ANIM_FUNC_DIZZY" => {
            let angle = param.wrapping_add((age as u8).wrapping_mul(2));
            Some((
                (x + visible_battle_anim_sine(angle.wrapping_add(0x10), 0x10)) & 0xff,
                (y + (visible_battle_anim_sine(angle, 0x10) >> 2)) & 0xff,
            ))
        }
        "BATTLE_ANIM_FUNC_BITE" => {
            let initial_angle = if param & 0x80 != 0 { 0x30_u8 } else { 0x10 };
            let amplitude = match param & 0x7f {
                0 => 0x10,
                value => value,
            };
            let mut boundary_count = 0_u8;
            for tick in 1..=updates {
                if initial_angle.wrapping_add((tick as u8).wrapping_mul(2)) & 0x1f == 0 {
                    boundary_count = boundary_count.saturating_add(1);
                }
            }
            if boundary_count >= 5 {
                return None;
            }
            let angle = initial_angle.wrapping_add((age as u8).wrapping_mul(2));
            Some((
                x & 0xff,
                (y + visible_battle_anim_sine(angle, amplitude)) & 0xff,
            ))
        }
        "BATTLE_ANIM_FUNC_SHINY" => Some((
            (x + visible_battle_anim_sine(param.wrapping_add(0x10), 0x10)) & 0xff,
            (y + visible_battle_anim_sine(param, 0x10)) & 0xff,
        )),
        "BATTLE_ANIM_FUNC_FIRE_BLAST" => {
            if age == 0 {
                return Some((x & 0xff, y & 0xff));
            }
            match param {
                1 => Some((x & 0xff, (y - i32::from(age)) & 0xff)),
                2 => Some(((x - i32::from(age)) & 0xff, y & 0xff)),
                3 => Some(((x + i32::from(age)) & 0xff, y & 0xff)),
                4 => Some((
                    (x - i32::from(age)) & 0xff,
                    (y + i32::from(age)) & 0xff,
                )),
                5 => Some((
                    (x + i32::from(age)) & 0xff,
                    (y + i32::from(age)) & 0xff,
                )),
                7 => {
                    let travel_frames =
                        u16::try_from(((0x88 - x).max(0) + 1) / 2).unwrap_or(0);
                    if age <= travel_frames {
                        return Some((
                            (x + 2 * i32::from(age)) & 0xff,
                            (y - i32::from(age)) & 0xff,
                        ));
                    }
                    let base_x = (x + 2 * i32::from(travel_frames)) & 0xff;
                    let base_y = (y - i32::from(travel_frames)) & 0xff;
                    let angle = age.saturating_sub(travel_frames.saturating_add(1)) as u8;
                    Some((
                        (base_x + visible_battle_anim_sine(angle.wrapping_add(0x10), 0x10))
                            & 0xff,
                        (base_y + visible_battle_anim_sine(angle, 0x10)) & 0xff,
                    ))
                }
                8 => {
                    let angle = (age - 1) as u8;
                    Some((
                        (x + visible_battle_anim_sine(angle.wrapping_add(0x10), 0x10)) & 0xff,
                        (y + visible_battle_anim_sine(angle, 0x10)) & 0xff,
                    ))
                }
                9 => None,
                _ => Some((x & 0xff, y & 0xff)),
            }
        }
        "BATTLE_ANIM_FUNC_RAZOR_LEAF" => {
            let trigger_age = (state != 0).then(|| age.saturating_sub(state_age));
            let mut jump_index = 0_u8;
            let mut var1 = 0_u8;
            let mut var2 = 0_u8;
            let mut base_x = x & 0xff;
            let mut base_y = y & 0xff;
            let mut x_offset = 0_i32;
            let mut y_offset = 0_i32;
            for tick in 0..=age {
                if trigger_age == Some(tick) {
                    jump_index = jump_index.wrapping_add(state);
                }
                if jump_index == 0 {
                    jump_index = 1;
                    var1 = 0x40;
                }
                if jump_index == 1 {
                    let angle = var1;
                    if angle < 0x30 {
                        jump_index = 2;
                        var1 = 0;
                        var2 = 0;
                        continue;
                    }
                    let radius = param & 0x3f;
                    var1 = angle.wrapping_sub(1);
                    y_offset = visible_battle_anim_sine(angle, radius);
                    let magnitude = match radius {
                        0x20.. => 0x100_i32,
                        0x18..=0x1f => 0x180,
                        _ => 0x200,
                    };
                    let delta = if param & 0x80 == 0 { magnitude } else { -magnitude };
                    let position = (((base_x as u16) << 8) | u16::from(var2))
                        .wrapping_add(delta as u16);
                    base_x = i32::from(position >> 8);
                    var2 = position as u8;
                    continue;
                }
                if jump_index == 2 {
                    if (y_offset & 0xff) == 0x20 {
                        return None;
                    }
                    let angle = var1;
                    x_offset = visible_battle_anim_sine(angle, 0x10);
                    var1 = if param & 0x40 != 0 {
                        angle.wrapping_sub(1)
                    } else {
                        angle.wrapping_add(1)
                    };
                    let position = (((y_offset & 0xff) as u16) << 8 | u16::from(var2))
                        .wrapping_add(0x80);
                    y_offset = i32::from(position >> 8);
                    var2 = position as u8;
                    continue;
                }
                if jump_index == 3 {
                    jump_index = 4;
                    continue;
                }
                if (4..=7).contains(&jump_index) {
                    jump_index = jump_index.wrapping_add(1);
                    continue;
                }
                if jump_index == 8 && base_x < 0xc0 {
                    base_x = (base_x + 8) & 0xff;
                    base_y = (base_y - 4) & 0xff;
                }
            }
            Some(((base_x + x_offset) & 0xff, (base_y + y_offset) & 0xff))
        }
        "BATTLE_ANIM_FUNC_LEECH_SEED" => {
            if age == 0 {
                return Some((x & 0xff, y & 0xff));
            }
            let movement_steps = age.min(33);
            let mut base_x = x & 0xff;
            let mut base_y = y & 0xff;
            let mut fraction = 0_u8;
            let mut countdown = 0x40_u8;
            let mut y_offset = 0_i32;
            let delta = u16::from((param & 0xf0) >> 4) << 8
                | u16::from(param & 0x0f) << 4;
            for _ in 0..movement_steps {
                let original_countdown = countdown;
                countdown = countdown.wrapping_sub(1);
                y_offset = visible_battle_anim_sine(original_countdown, 0x20);
                let position = (((base_x as u16) << 8) | u16::from(fraction))
                    .wrapping_add(delta);
                base_x = i32::from(position >> 8);
                fraction = position as u8;
                if countdown & 1 == 0 {
                    base_y = (base_y - 1) & 0xff;
                }
            }
            Some((base_x & 0xff, (base_y + y_offset) & 0xff))
        }
        "BATTLE_ANIM_FUNC_PARALYZED" => {
            let delay = u16::from((param & 0x70) >> 4);
            let interval = delay.saturating_add(1).max(1);
            let toggles = if age == 0 {
                0
            } else {
                1_u16.saturating_add((age - 1) / interval)
            };
            let magnitude = i32::from(param & 0x0f);
            let initial_offset = if param & 0x80 != 0 {
                -magnitude
            } else {
                magnitude
            };
            let offset = if toggles & 1 == 0 {
                initial_offset
            } else {
                -initial_offset
            };
            Some(((x + offset) & 0xff, y & 0xff))
        }
        "BATTLE_ANIM_FUNC_METRONOME_HAND" => {
            let angle = (age as u8).wrapping_mul(2);
            Some((
                (x + visible_battle_anim_sine(angle.wrapping_add(0x10), 8)) & 0xff,
                (y + visible_battle_anim_sine(angle, 2)) & 0xff,
            ))
        }
        "BATTLE_ANIM_FUNC_AMNESIA" => {
            if age >= 32 {
                None
            } else {
                Some((x & 0xff, y & 0xff))
            }
        }
        "BATTLE_ANIM_FUNC_AGILITY" => {
            Some(((x + i32::from(param)) & 0xff, y & 0xff))
        }
        "BATTLE_ANIM_FUNC_GROWTH_SWORDS_DANCE" => {
            let angle = param.wrapping_add(age as u8);
            Some((
                (x + visible_battle_anim_sine(angle.wrapping_add(0x10), 0x18)) & 0xff,
                (y + (visible_battle_anim_sine(angle, 0x18) >> 3)
                    - 2 * i32::from(age))
                    & 0xff,
            ))
        }
        "BATTLE_ANIM_FUNC_SAFEGUARD_PROTECT" => {
            let angle = param.wrapping_add(age as u8);
            Some((
                (x + (visible_battle_anim_sine(angle.wrapping_add(0x10), 0x18) >> 1))
                    & 0xff,
                (y + visible_battle_anim_sine(angle, 0x18)) & 0xff,
            ))
        }
        "BATTLE_ANIM_FUNC_SMOKE_FLAME_WHEEL" => {
            let mut runtime_param = param;
            let mut vertical_drift = 0_u8;
            let mut x_offset = 0_i32;
            let mut y_offset = 0_i32;
            for _ in 0..updates {
                let angle = runtime_param;
                x_offset = visible_battle_anim_sine(angle.wrapping_add(0x10), 0x18);
                y_offset = (visible_battle_anim_sine(angle, 0x18) >> 3)
                    + i32::from(vertical_drift);
                runtime_param = runtime_param.wrapping_add(2);
                if runtime_param & 7 == 0 {
                    if vertical_drift as i8 == -24 {
                        return None;
                    }
                    vertical_drift = vertical_drift.wrapping_sub(1);
                }
            }
            Some(((x + x_offset) & 0xff, (y + y_offset) & 0xff))
        }
        "BATTLE_ANIM_FUNC_RAIN_SANDSTORM" => {
            if age == 0 {
                return Some((x & 0xff, y & 0xff));
            }
            let speed = match param {
                0 => 2_i32,
                1 => 8,
                2 => 4,
                _ => return None,
            };
            let vertical = i32::from((age.saturating_mul(4) % 0x70) as u8);
            Some(((x + speed * i32::from(age)) & 0xff, (y + vertical) & 0xff))
        }
        "BATTLE_ANIM_FUNC_HEAL_BELL_NOTES" => {
            if age == 0 {
                return Some((x & 0xff, y & 0xff));
            }
            if age > 0x38 {
                return None;
            }
            let base_x = if y & 1 == 0 {
                x - i32::from(age)
            } else {
                x
            };
            Some((
                (base_x
                    + visible_battle_anim_sine((age as u8).wrapping_add(0x10), 0x18))
                    & 0xff,
                (y + i32::from(age)) & 0xff,
            ))
        }
        "BATTLE_ANIM_FUNC_BATON_PASS" => {
            let mut amplitude = param;
            let mut angle = 0_u8;
            let mut y_offset = 0_i32;
            for _ in 0..updates {
                if amplitude == 0 {
                    break;
                }
                angle = angle.wrapping_add(1);
                y_offset = visible_battle_anim_sine(angle, amplitude).abs();
                if angle & 0x1f == 0 {
                    amplitude >>= 1;
                }
            }
            Some((x & 0xff, (y + y_offset) & 0xff))
        }
        "BATTLE_ANIM_FUNC_LOCK_ON_MIND_READER" => {
            if age >= 58 {
                None
            } else {
                Some((x & 0xff, y & 0xff))
            }
        }
        "BATTLE_ANIM_FUNC_PERISH_SONG" => {
            let angle = param.wrapping_add((age as u8).wrapping_mul(2));
            Some((
                (x + visible_battle_anim_sine(angle.wrapping_add(0x10), 0x50)) & 0xff,
                (y + (visible_battle_anim_sine(angle, 0x50) >> 2)
                    + i32::from(age as u8))
                    & 0xff,
            ))
        }
        "BATTLE_ANIM_FUNC_SACRED_FIRE" => {
            let mut runtime_param = param;
            let mut vertical_drift = 0_u8;
            let mut x_offset = 0_i32;
            let mut y_offset = 0_i32;
            for _ in 0..updates {
                let angle = runtime_param;
                x_offset = visible_battle_anim_sine(angle.wrapping_add(0x10), 0x18);
                y_offset = (visible_battle_anim_sine(angle, 0x18) >> 3)
                    + i32::from(vertical_drift);
                runtime_param = runtime_param.wrapping_add(2);
                if runtime_param & 3 == 0 {
                    if vertical_drift as i8 == -48 {
                        return None;
                    }
                    vertical_drift = vertical_drift.wrapping_sub(2);
                }
            }
            Some(((x + x_offset) & 0xff, (y + y_offset) & 0xff))
        }
        "BATTLE_ANIM_FUNC_SLUDGE" => {
            let rise = age.saturating_sub(13);
            Some((x & 0xff, (y - i32::from(rise)) & 0xff))
        }
        "BATTLE_ANIM_FUNC_SOLAR_BEAM" => {
            if age == 0 {
                return Some((x & 0xff, y & 0xff));
            }
            let fixed_radius = 0x2800_i32 - 0x80 * i32::from(age - 1);
            let radius = u8::try_from((fixed_radius.max(0) >> 8) & 0xff).ok()?;
            if radius == 0 {
                return None;
            }
            Some((
                (x + visible_battle_anim_sine(param.wrapping_add(0x10), radius)) & 0xff,
                (y + visible_battle_anim_sine(param, radius)) & 0xff,
            ))
        }
        "BATTLE_ANIM_FUNC_DIG" => {
            if age >= 23 {
                return None;
            }
            let offset = if age <= 11 {
                2 * i32::from(age + 1)
            } else {
                24 - 2 * i32::from(age - 11)
            };
            Some((x & 0xff, (y + offset) & 0xff))
        }
        "BATTLE_ANIM_FUNC_STRING" => Some((x & 0xff, y & 0xff)),
        "BATTLE_ANIM_FUNC_WRAP" => Some((x & 0xff, y & 0xff)),
        "BATTLE_ANIM_FUNC_METRONOME_SPARKLE_SKETCH" => {
            let mut runtime_param = param;
            let mut y_offset = 0_u8;
            let mut x_offset = 0_i32;
            for _ in 0..updates {
                if y_offset >= 0x20 {
                    return None;
                }
                x_offset = visible_battle_anim_sine(runtime_param.wrapping_add(0x10), 8);
                runtime_param = runtime_param.wrapping_add(2);
                if runtime_param & 7 == 0 {
                    y_offset = y_offset.saturating_add(1);
                }
            }
            Some(((x + x_offset) & 0xff, (y + i32::from(y_offset)) & 0xff))
        }
        "BATTLE_ANIM_FUNC_ENCORE_BELLY_DRUM" => {
            let progress = age.saturating_mul(2);
            if progress >= 0x10 {
                return None;
            }
            let amplitude = progress as u8;
            Some((
                (x + visible_battle_anim_sine(param.wrapping_add(0x10), amplitude)) & 0xff,
                (y + visible_battle_anim_sine(param, amplitude)) & 0xff,
            ))
        }
        "BATTLE_ANIM_FUNC_SWAGGER_MORNING_SUN" => {
            let angle = param & 0x3f;
            let speed = (param >> 6) & 0x03;
            let amplitude = speed.wrapping_mul((age as u8).wrapping_add(1));
            Some((
                (x + visible_battle_anim_sine(angle.wrapping_add(0x10), amplitude)) & 0xff,
                (y + visible_battle_anim_sine(angle, amplitude)) & 0xff,
            ))
        }
        "BATTLE_ANIM_FUNC_PETAL_DANCE" => {
            if age >= 327 {
                return None;
            }
            let angle = age as u8;
            let descent = i32::from(age / 8);
            Some((
                (x + visible_battle_anim_sine(angle.wrapping_add(0x10), 0x18)) & 0xff,
                (y + (visible_battle_anim_sine(angle, 0x18) >> 3) + descent) & 0xff,
            ))
        }
        "BATTLE_ANIM_FUNC_BONEMERANG" => {
            if age == 0 {
                return Some((x & 0xff, y & 0xff));
            }
            let angle = param.wrapping_add((age - 1) as u8);
            Some((
                (x + visible_battle_anim_sine(angle.wrapping_add(8 + 0x10), 0x30)) & 0xff,
                (y + visible_battle_anim_sine(angle, 0x30)) & 0xff,
            ))
        }
        "BATTLE_ANIM_FUNC_PSYCH_UP" => {
            let angle = param.wrapping_add((age as u8).wrapping_add(1));
            Some((
                (x + visible_battle_anim_sine(angle.wrapping_add(0x10), 0x18)) & 0xff,
                (y + (visible_battle_anim_sine(angle, 0x18) >> 2)) & 0xff,
            ))
        }
        "BATTLE_ANIM_FUNC_CONVERSION" => {
            let mut angle = param;
            let mut radius = 0_u8;
            let mut progress = 0_u8;
            let mut x_offset = 0_i32;
            let mut y_offset = 0_i32;
            for _ in 0..updates {
                x_offset = visible_battle_anim_sine(angle.wrapping_add(0x10), radius);
                y_offset = visible_battle_anim_sine(angle, radius);
                angle = angle.wrapping_add(1);
                progress = progress.wrapping_add(1);
                if progress < 0x40 {
                    radius = radius.wrapping_add(1);
                } else {
                    radius = radius.wrapping_sub(1);
                    if radius == 0 {
                        return None;
                    }
                }
            }
            Some(((x + x_offset) & 0xff, (y + y_offset) & 0xff))
        }
        "BATTLE_ANIM_FUNC_BATTLE_ANIM_OBJ_B0" => {
            let high = u32::from((param >> 4) & 0x0f);
            let low = u32::from(param & 0x0f);
            let delta = ((high * 0x11) << 8) | (low << 4);
            let position = ((x as u32 & 0xff) << 8)
                .wrapping_add(delta.wrapping_mul(updates));
            Some((((position >> 8) as i32) & 0xff, y & 0xff))
        }
        "BATTLE_ANIM_FUNC_WATER_GUN" => {
            let transition_age = u16::try_from((y - 0x2f).max(0)).unwrap_or(0);
            if age < transition_age {
                let steps = age.saturating_add(1);
                let angle = param.wrapping_sub(steps as u8);
                return Some((
                    (x + 2 * i32::from(steps)) & 0xff,
                    (y - i32::from(steps) + visible_battle_anim_sine(angle, 8)) & 0xff,
                ));
            }
            let base_x = (x + 2 * i32::from(transition_age)) & 0xff;
            let splash_offset = age.saturating_sub(transition_age).min(24);
            Some((base_x, (0x30 + i32::from(splash_offset)) & 0xff))
        }
        "BATTLE_ANIM_FUNC_SPIKES" => {
            if age == 0 {
                return Some((x & 0xff, y & 0xff));
            }
            let movement_steps = age.min(33);
            let mut base_x = x & 0xff;
            let mut base_y = y & 0xff;
            let mut fraction = 0_u8;
            let mut countdown = 0x40_u8;
            let mut y_offset = 0_i32;
            let delta = u16::from((param & 0xf0) >> 4) << 8
                | u16::from(param & 0x0f) << 4;
            for _ in 0..movement_steps {
                let original_countdown = countdown;
                countdown = countdown.wrapping_sub(1);
                y_offset = visible_battle_anim_sine(original_countdown, 0x20);
                let position = (((base_x as u16) << 8) | u16::from(fraction))
                    .wrapping_add(delta);
                base_x = i32::from(position >> 8);
                fraction = position as u8;
                if countdown & 1 == 0 {
                    base_y = (base_y - 1) & 0xff;
                }
            }
            Some((base_x & 0xff, (base_y + y_offset) & 0xff))
        }
        "BATTLE_ANIM_FUNC_HIDDEN_POWER" => {
            if state == 0 {
                let angle = param.wrapping_add(age as u8);
                return Some((
                    (x + visible_battle_anim_sine(angle.wrapping_add(0x10), 0x18)) & 0xff,
                    (y + (visible_battle_anim_sine(angle, 0x18) >> 2)) & 0xff,
                ));
            }
            let trigger_age = age.saturating_sub(state_age);
            if state_age == 0 {
                let angle = param.wrapping_add(trigger_age.saturating_sub(1) as u8);
                return Some((
                    (x + visible_battle_anim_sine(angle.wrapping_add(0x10), 0x18)) & 0xff,
                    (y + (visible_battle_anim_sine(angle, 0x18) >> 2)) & 0xff,
                ));
            }
            let radius = 0x18_u16.saturating_add(state_age.saturating_sub(1).saturating_mul(8));
            if radius >= 0x80 {
                return None;
            }
            let angle = param.wrapping_add(trigger_age as u8);
            let radius = radius as u8;
            Some((
                (x + visible_battle_anim_sine(angle.wrapping_add(0x10), radius)) & 0xff,
                (y + (visible_battle_anim_sine(angle, radius) >> 2)) & 0xff,
            ))
        }
        "BATTLE_ANIM_FUNC_SKY_ATTACK" => match state {
            0 => Some((x & 0xff, y & 0xff)),
            1 => {
                let prior_x = x + 4 * i32::from(state_age);
                if prior_x >= 0x84 {
                    Some((prior_x & 0xff, y & 0xff))
                } else {
                    Some(((prior_x + 4) & 0xff, y & 0xff))
                }
            }
            _ => {
                let prior_x = 0x84 + 4 * i32::from(state_age);
                if prior_x >= 0xd0 {
                    None
                } else {
                    Some(((prior_x + 4) & 0xff, y & 0xff))
                }
            }
        },
        "BATTLE_ANIM_FUNC_HORN" => {
            if age == 0 {
                return Some((x & 0xff, y & 0xff));
            }
            let travel_frames = u16::try_from(((0x58 - x).max(0) + 1) / 2).unwrap_or(0);
            if age <= travel_frames {
                return Some(((x + 2 * i32::from(age)) & 0xff, y & 0xff));
            }
            if age == travel_frames.saturating_add(1) {
                return Some(((x + 2 * i32::from(travel_frames)) & 0xff, y & 0xff));
            }
            let impact_step = age.saturating_sub(travel_frames.saturating_add(2));
            let amplitude = impact_step.saturating_mul(8);
            if amplitude >= 0x20 {
                return None;
            }
            let x_offset = visible_battle_anim_sine(amplitude as u8, 8);
            Some((
                (x + 2 * i32::from(travel_frames) + x_offset) & 0xff,
                (y - (x_offset >> 1)) & 0xff,
            ))
        }
        "BATTLE_ANIM_FUNC_SOUND" => {
            if age == 0 {
                return Some((x & 0xff, y & 0xff));
            }
            if age >= 9 {
                return None;
            }
            let effective_param = if player_move {
                param
            } else {
                (!param).wrapping_add(3)
            };
            let angle = ((age - 1) as u8).wrapping_mul(2);
            let x_offset = visible_battle_anim_sine(angle, 0x10);
            let y_offset = match effective_param & 3 {
                0 => -x_offset,
                1 => 0,
                _ => x_offset,
            };
            Some(((x + x_offset) & 0xff, (y + y_offset) & 0xff))
        }
        "BATTLE_ANIM_FUNC_CONFUSE_RAY" => {
            if age == 0 {
                return Some((x & 0xff, y & 0xff));
            }
            let amplitude = (param >> 4) | ((param & 0x0f) << 4);
            let mut base_x = x & 0xff;
            let mut base_y = y & 0xff;
            let mut current_x = base_x;
            let mut x_offset = 0_i32;
            let mut y_offset = 0_i32;
            for step in 1..=age {
                let angle = (param & 0x3f).wrapping_add(step as u8);
                y_offset = visible_battle_anim_sine(angle, amplitude);
                x_offset = visible_battle_anim_sine(angle.wrapping_add(0x10), amplitude);
                if current_x < 0x80 {
                    if angle & 3 == 0 {
                        base_y = (base_y - 1) & 0xff;
                    }
                    if angle & 1 == 0 {
                        base_x = (base_x + 1) & 0xff;
                    }
                }
                current_x = (base_x + x_offset) & 0xff;
            }
            Some((current_x, (base_y + y_offset) & 0xff))
        }
        "BATTLE_ANIM_FUNC_CURSE" => {
            let prior_x = x - 2 * i32::from(age);
            if prior_x < 0x30 {
                None
            } else {
                Some(((prior_x - 2) & 0xff, (y + 2 * i32::from(age + 1)) & 0xff))
            }
        }
        "BATTLE_ANIM_FUNC_BETA_PURSUIT" => {
            if age == 0 {
                return Some((x & 0xff, y & 0xff));
            }
            if param == 0 {
                if age >= 7 {
                    return None;
                }
                let offset = -20 + 4 * i32::from(age);
                Some((x & 0xff, (y + offset) & 0xff))
            } else {
                let offset = -(4 * i32::from(age.min(10)));
                Some((x & 0xff, (y + offset) & 0xff))
            }
        }
        "BATTLE_ANIM_FUNC_THIEF_PAYDAY" => {
            if age == 0 {
                return Some((x & 0xff, y & 0xff));
            }
            let mut angle = 0x28_u8;
            let mut amplitude = (y - 0x28) as u8;
            let mut base_x = x & 0xff;
            let mut y_offset = 0_i32;
            let mask = if param == 0 { 0xff } else { param };
            for _ in 0..age {
                y_offset = visible_battle_anim_sine(angle, amplitude);
                if angle & mask == 0 {
                    base_x = (base_x - 1) & 0xff;
                }
                angle = angle.wrapping_add(1);
                if angle & 0x3f == 0 {
                    angle = 0x20;
                    amplitude /= 2;
                }
            }
            Some((base_x, (y + y_offset) & 0xff))
        }
        "BATTLE_ANIM_FUNC_GUST" => {
            const RADII: [u8; 9] = [8, 6, 5, 4, 5, 6, 8, 12, 16];
            let mut runtime_param = 0_u8;
            let mut angle = 0_u8;
            let mut radius_index = 0_usize;
            let mut base_x = x & 0xff;
            let mut base_y = y & 0xff;
            let mut x_offset = 0_i32;
            let mut y_offset = 0_i32;
            let trigger_age = (state != 0).then(|| age.saturating_sub(state_age));
            for tick in 0..=age {
                let radius = RADII[radius_index];
                y_offset = (visible_battle_anim_sine(angle, radius) >> 4)
                    + i32::from(runtime_param);
                x_offset = visible_battle_anim_sine(angle.wrapping_add(0x10), radius);
                angle = angle.wrapping_sub(8);
                if runtime_param >= 0xc2 {
                    runtime_param = 0;
                    radius_index = 0;
                    x_offset = 0;
                    y_offset = 0;
                } else {
                    runtime_param = runtime_param.wrapping_sub(1);
                    if runtime_param & 7 == 0 {
                        radius_index = (radius_index + 1) % RADII.len();
                    }
                }
                if trigger_age.is_some_and(|trigger| tick >= trigger) && base_x < 0x88 {
                    base_x = (base_x + 1) & 0xff;
                    if base_x & 1 == 0 {
                        base_y = (base_y - 1) & 0xff;
                    }
                }
            }
            Some(((base_x + x_offset) & 0xff, (base_y + y_offset) & 0xff))
        }
        "BATTLE_ANIM_FUNC_STRENGTH_SEISMIC_TOSS" => {
            let trigger_age = (state != 0).then(|| age.saturating_sub(state_age));
            let mut jump_index = 0_u8;
            let mut var1 = 0_u8;
            let mut var2 = 0_u8;
            let mut base_x = x & 0xff;
            let mut base_y = y & 0xff;
            let mut y_offset = 0_u8;
            for tick in 0..=age {
                if trigger_age == Some(tick) {
                    jump_index = jump_index.wrapping_add(state);
                }
                if jump_index == 0 {
                    if y_offset == 0xe0 {
                        var1 = 2;
                        jump_index = 1;
                        continue;
                    }
                    let accumulator = ((u16::from(y_offset) << 8) | u16::from(var1))
                        .wrapping_sub(0x80);
                    y_offset = (accumulator >> 8) as u8;
                    var1 = accumulator as u8;
                    continue;
                }
                if jump_index == 1 {
                    if var2 != 0 {
                        var2 = var2.wrapping_sub(1);
                        continue;
                    }
                    var2 = 4;
                    var1 = (!var1).wrapping_add(1);
                    y_offset = y_offset.wrapping_add(var1);
                    continue;
                }
                base_y = (base_y - 2) & 0xff;
                base_x = (base_x + 4) & 0xff;
                if base_x >= 0x84 {
                    return None;
                }
            }
            Some((base_x, (base_y + i32::from(y_offset)) & 0xff))
        }
        "BATTLE_ANIM_FUNC_SURF" => {
            let trigger_age = (state != 0).then(|| age.saturating_sub(state_age));
            let mut runtime_state = 0_u8;
            let base_x = x & 0xff;
            let mut base_y = y & 0xff;
            let mut x_offset = 0_i32;
            let mut y_offset = 0_i32;
            let mut angle = 0_u8;
            for tick in 0..=age {
                if trigger_age == Some(tick) {
                    runtime_state = runtime_state.wrapping_add(state);
                }
                if runtime_state == 0 {
                    runtime_state = 1;
                    continue;
                }
                if runtime_state == 1 {
                    if base_y < i32::from(param) {
                        runtime_state = 2;
                        continue;
                    }
                    base_y = (base_y - 1) & 0xff;
                    y_offset = visible_battle_anim_sine(angle, 0x10);
                    x_offset = (x_offset + 1) & 7;
                    angle = angle.wrapping_add(2);
                    continue;
                }
                if runtime_state == 2 {
                    continue;
                }
                if runtime_state == 3 {
                    if base_y >= 0x70 {
                        return None;
                    }
                    base_y = (base_y + 2) & 0xff;
                    continue;
                }
                return None;
            }
            Some(((base_x + x_offset) & 0xff, (base_y + y_offset) & 0xff))
        }
        "BATTLE_ANIM_FUNC_POISON_GAS" => {
            let mut runtime_state = 0_u8;
            let mut base_x = x & 0xff;
            let mut base_y = y & 0xff;
            let mut angle = 0_u8;
            let mut descent = 0_u8;
            let mut x_offset = 0_i32;
            let mut y_offset = 0_i32;
            for _ in 0..=age {
                if runtime_state == 0 {
                    if base_x >= 0x84 {
                        runtime_state = 1;
                        continue;
                    }
                    base_x = (base_x + 1) & 0xff;
                    angle = angle.wrapping_add(1);
                    x_offset = visible_battle_anim_sine(angle.wrapping_add(0x10), 0x18);
                    if base_x & 1 == 0 {
                        base_y = (base_y - 1) & 0xff;
                    }
                    continue;
                }
                y_offset = (visible_battle_anim_sine(angle, 0x18) >> 3)
                    + i32::from(descent);
                x_offset = visible_battle_anim_sine(angle.wrapping_add(0x10), 0x18);
                angle = angle.wrapping_add(1);
                if angle & 7 == 0 {
                    if descent >= 0x28 {
                        return None;
                    }
                    descent = descent.saturating_add(1);
                }
            }
            Some(((base_x + x_offset) & 0xff, (base_y + y_offset) & 0xff))
        }
        "BATTLE_ANIM_FUNC_KICK" => {
            if state == 0 {
                let movements = i32::from(age).min(((0x98 - x).max(0) + 1) / 2);
                Some(((x + movements * 2) & 0xff, (y - movements) & 0xff))
            } else {
                let movements = i32::from(state_age).min(((0x98 - x).max(0) + 1) / 2);
                let angle = 0x2c_u8.wrapping_add(state_age.min(u16::from(u8::MAX)) as u8);
                Some((
                    (x + movements * 2) & 0xff,
                    (y + visible_battle_anim_sine(angle, 8)) & 0xff,
                ))
            }
        }
        "BATTLE_ANIM_FUNC_EGG" if param == 1 && state != 0 => None,
        "BATTLE_ANIM_FUNC_EGG" if param == 1 && age <= 56 => {
            let movements = i32::from(age).min((0x40 - x).max(0));
            let (angle, amplitude) = visible_egg_wave_at_age(age);
            Some((
                (x + movements) & 0xff,
                (y + visible_battle_anim_sine(angle, amplitude)) & 0xff,
            ))
        }
        "BATTLE_ANIM_FUNC_EGG" if param == 1 => {
            let mut base_x = (x + (0x40 - x).max(0)) & 0xff;
            let mut fixed_y = (y & 0xff) << 8;
            let mut pause = 0_u8;
            let mut resume_after_pause = false;
            for _ in 0..age.saturating_sub(56) {
                if base_x >= 0x88 {
                    return Some((base_x, (fixed_y >> 8) & 0xff));
                }
                if pause != 0 {
                    pause = pause.saturating_sub(1);
                    if pause == 0 {
                        resume_after_pause = true;
                    }
                    continue;
                }
                if !resume_after_pause && base_x & 0xf == 0 && base_x != (x & 0xff) {
                    pause = 0x10;
                    continue;
                }
                resume_after_pause = false;
                base_x = (base_x + 1) & 0xff;
                fixed_y = (fixed_y - 0x80) & 0xffff;
            }
            Some((base_x, (fixed_y >> 8) & 0xff))
        }
        "BATTLE_ANIM_FUNC_EGG" if param == 6 && age <= 56 => {
            let movements = i32::from(age).min((0x4b - x).max(0));
            let (angle, amplitude) = visible_egg_wave_at_age(age);
            Some((
                (x + movements) & 0xff,
                (y + visible_battle_anim_sine(angle, amplitude)) & 0xff,
            ))
        }
        "BATTLE_ANIM_FUNC_EGG" if param == 6 && state != 0 => {
            Some(((0x4b) & 0xff, (y + 4) & 0xff))
        }
        "BATTLE_ANIM_FUNC_EGG" if param == 6 => {
            if age == 57 {
                let (angle, amplitude) = visible_egg_wave_at_age(56);
                Some((0x4b, (y + visible_battle_anim_sine(angle, amplitude)) & 0xff))
            } else {
                let angle = 2_u8.wrapping_mul(age.saturating_sub(58) as u8);
                Some(((0x4b + visible_battle_anim_sine(angle, 2)) & 0xff, y & 0xff))
            }
        }
        "BATTLE_ANIM_FUNC_EGG" if param == 11 => {
            Some((x & 0xff, y & 0xff))
        }
        "BATTLE_ANIM_FUNC_NEEDLE" => {
            if age == 0 {
                return Some((x & 0xff, y & 0xff));
            }
            let speed = i32::from(param & 0x0f);
            let movements = i32::from(age);
            let prior_x = (x + speed.saturating_mul(movements.saturating_sub(1))) & 0xff;
            if prior_x >= 0x84 {
                return None;
            }
            let y_speed = speed / 2;
            let y_offset = if (param >> 4) & 0x0f == 2 {
                let angle = 0_u8.wrapping_sub((age.saturating_sub(1) as u8).wrapping_mul(4));
                visible_battle_anim_sine(angle, 0x10).min(0)
            } else {
                0
            };
            Some((
                (prior_x + speed) & 0xff,
                (y - y_speed.saturating_mul(movements) + y_offset) & 0xff,
            ))
        }
        "BATTLE_ANIM_FUNC_EMBER" => {
            match param >> 4 {
                1 => {
                    let encoded = param & 0x0f;
                    let step = i32::from(if encoded == 0 { 1 } else { encoded });
                    Some((
                        (x + step.saturating_mul(updates as i32)) & 0xff,
                        (y - (step >> 1).saturating_mul(updates as i32)) & 0xff,
                    ))
                }
                2 => None,
                _ => Some((x & 0xff, y & 0xff)),
            }
        }
        "BATTLE_ANIM_FUNC_BUBBLE" => {
            let initial_steps = updates.min(12);
            let step = i32::from(param & 0x0f);
            let mut current_x = (x + step.saturating_mul(initial_steps as i32)) & 0xff;
            let mut current_y = (y - (step >> 1).saturating_mul(initial_steps as i32)) & 0xff;
            if updates <= 12 {
                return Some((current_x, current_y));
            }
            let mut x_fraction = 0_u8;
            let mut y_fraction = 0_u8;
            for _ in 0..updates - 12 {
                if current_x < 0x98 {
                    let position = (((current_x as u16) << 8) | u16::from(x_fraction))
                        .wrapping_add(0x60);
                    current_x = i32::from(position >> 8);
                    x_fraction = position as u8;
                }
                if current_y >= 0x20 {
                    let delta = 0xff00_u16 | u16::from(param & 0xf0);
                    let position = (((current_y as u16) << 8) | u16::from(y_fraction))
                        .wrapping_add(delta);
                    current_y = i32::from(position >> 8);
                    y_fraction = position as u8;
                }
            }
            Some((current_x & 0xff, current_y & 0xff))
        }
        "BATTLE_ANIM_FUNC_THUNDER_WAVE" => {
            let current_x = (x + 2_i32.saturating_mul(updates as i32)) & 0xff;
            if current_x >= 0x84 {
                return None;
            }
            let base_y = (y - updates as i32) & 0xff;
            let angle = (updates as u8).wrapping_mul(6);
            Some((
                current_x,
                (base_y + visible_battle_anim_sine(angle, 6)) & 0xff,
            ))
        }
        _ => None,
    }
}

fn visible_egg_wave_at_age(age: u16) -> (u8, u8) {
    if age == 0 {
        return (0, 0);
    }
    if age <= 24 {
        return (0x28_u8.wrapping_add((age - 1) as u8), 0x10);
    }
    (0x20_u8.wrapping_add((age - 25) as u8), 0x08)
}

fn spawn_visible_move_animation_objects(
    commands: &mut Commands,
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) -> Result<()> {
    let synthetic_shiny;
    let animation = if let Some(animation) = runtime_shell
        .visible_move_animations
        .front()
        .filter(|animation| animation.started)
    {
        animation
    } else if let Some(send_out) = runtime_shell.visible_send_out_animation.as_ref().filter(|animation| {
        animation.shiny && animation.frame >= VisibleSendOutAnimation::NORMAL_FRAMES
    }) {
        let sparkle_frame = u16::from(send_out.frame - VisibleSendOutAnimation::NORMAL_FRAMES);
        synthetic_shiny = VisibleMoveAnimation {
            trigger_message: String::new(),
            move_id: "SEND_OUT_SHINY".to_string(),
            animation_label: "BattleAnim_SendOutMon.Shiny".to_string(),
            player_move: send_out.side == crate::core::battle::turn::BattleSide::Player,
            started: true,
            waiting_for_hp: false,
            frame: sparkle_frame,
            total_frames: u16::from(VisibleSendOutAnimation::SHINY_FRAMES),
            sound_events: Vec::new(),
            next_sound_event: 0,
            cry_events: Vec::new(),
            next_cry_event: 0,
            object_events: (0_u8..8)
                .map(|index| VisibleMoveObjectEvent {
                    frame: u16::from(index) * 4,
                    command: VisibleMoveObjectCommand::Spawn {
                        object_id: "BATTLE_ANIM_OBJ_SHINY".to_string(),
                        x: 48,
                        y: 96,
                        param: index * 8,
                    },
                })
                .collect(),
            bg_events: Vec::new(),
            actor_species_override: None,
            actor_shiny_override: None,
        };
        &synthetic_shiny
    } else if let Some(capture) = runtime_shell
        .visible_capture_animation
        .as_ref()
        .filter(|animation| animation.retained_objects_visible())
    {
        let mut object_events = Vec::new();
        if capture.blocked {
            object_events.push(VisibleMoveObjectEvent {
                frame: 20,
                command: VisibleMoveObjectCommand::Spawn {
                    object_id: "BATTLE_ANIM_OBJ_HIT_YFIX".to_string(),
                    x: 112,
                    y: 40,
                    param: 0,
                },
            });
        } else {
            object_events.push(VisibleMoveObjectEvent {
                frame: 52,
                command: VisibleMoveObjectCommand::Spawn {
                    object_id: "BATTLE_ANIM_OBJ_BALL_POOF".to_string(),
                    x: 136,
                    y: 64,
                    param: 0x10,
                },
            });
        }
        if !capture.blocked && capture.ball_id.eq_ignore_ascii_case("MASTER_BALL") {
            object_events.extend((0_u8..8).map(|index| VisibleMoveObjectEvent {
                frame: 76,
                command: VisibleMoveObjectCommand::Spawn {
                    object_id: "BATTLE_ANIM_OBJ_MASTER_BALL_SPARKLE".to_string(),
                    x: 136,
                    y: 56,
                    param: 0x30 + index,
                },
            }));
        }
        if !capture.blocked && !capture.caught {
            object_events.push(VisibleMoveObjectEvent {
                frame: capture.total_frames().saturating_sub(34),
                command: VisibleMoveObjectCommand::Spawn {
                    object_id: "BATTLE_ANIM_OBJ_BALL_POOF".to_string(),
                    x: 136,
                    y: 64,
                    param: 0x10,
                },
            });
        }
        synthetic_shiny = VisibleMoveAnimation {
            trigger_message: String::new(),
            move_id: format!("THROW_{}", capture.ball_id),
            animation_label: "BattleAnim_ThrowPokeBall".to_string(),
            player_move: true,
            started: true,
            waiting_for_hp: false,
            frame: capture.frame,
            total_frames: capture.total_frames(),
            sound_events: Vec::new(),
            next_sound_event: 0,
            cry_events: Vec::new(),
            next_cry_event: 0,
            object_events,
            bg_events: Vec::new(),
            actor_species_override: None,
            actor_shiny_override: None,
        };
        &synthetic_shiny
    } else {
        return Ok(());
    };
    let bundle = battle_anim_render_bundle(rendered_art, snapshot)?;
    // Crystal owns ten animation-object structs. Spawns occupy the first free
    // slot, while anim_incobj/anim_setobj address those slots one-based.
    let mut slots = [None::<(&VisibleMoveObjectEvent, u16, u8, u16)>; 10];
    for event in animation
        .object_events
        .iter()
        .filter(|event| event.frame <= animation.frame)
    {
        for slot in &mut slots {
            let Some((spawn, spawn_frame, state, state_frame)) = slot.as_ref() else {
                continue;
            };
            let VisibleMoveObjectCommand::Spawn { object_id, x, y, param } = &spawn.command else {
                continue;
            };
            let object = bundle
                .get("objects")
                .and_then(|objects| objects.get(object_id))
                .with_context(|| format!("battle animation object {object_id} is missing from the runtime bundle"))?;
            let function = battle_anim_object_function(object_id, object)?;
            let age = event.frame.saturating_sub(*spawn_frame);
            let state_age = event.frame.saturating_sub(*state_frame);
            let deleted = if function == "BATTLE_ANIM_FUNC_NULL" {
                object
                    .get("frameset")
                    .and_then(serde_json::Value::as_str)
                    .and_then(|frameset| visible_null_battle_animation_object_lifetime(&bundle, frameset))
                    .is_some_and(|lifetime| age >= lifetime)
            } else {
                visible_battle_anim_object_position(
                    function,
                    i32::from(*x),
                    i32::from(*y),
                    *param,
                    age,
                    *state,
                    state_age,
                    animation.player_move,
                )
                .is_none()
            };
            if deleted {
                *slot = None;
            }
        }
        match &event.command {
            VisibleMoveObjectCommand::Spawn { .. } => {
                if let Some(slot) = slots.iter_mut().find(|slot| slot.is_none()) {
                    *slot = Some((event, event.frame, 0, event.frame));
                }
            }
            VisibleMoveObjectCommand::Clear => slots.fill(None),
            VisibleMoveObjectCommand::Increment { slot } => {
                if let Some(entry) = slot.checked_sub(1).and_then(|slot| slots.get_mut(usize::from(slot)))
                    && let Some((_, _, state, state_frame)) = entry.as_mut()
                {
                    *state = state.wrapping_add(1);
                    *state_frame = event.frame;
                }
            }
            VisibleMoveObjectCommand::Set { slot, value } => {
                if let Some(entry) = slot.checked_sub(1).and_then(|slot| slots.get_mut(usize::from(slot)))
                    && let Some((_, _, state, state_frame)) = entry.as_mut()
                {
                    *state = *value;
                    *state_frame = event.frame;
                }
            }
        }
    }
    // Replaying object commands reconstructs slot ownership, but an object
    // may expire on a frame with no subsequent command. Crystal updates all
    // ten animation structs every frame, so apply the same lifetime/function
    // deletion check at the actual rendered age before collecting sprites.
    for slot in &mut slots {
        let Some((spawn, spawn_frame, state, state_frame)) = slot.as_ref() else {
            continue;
        };
        let VisibleMoveObjectCommand::Spawn { object_id, x, y, param } = &spawn.command else {
            continue;
        };
        let object = bundle
            .get("objects")
            .and_then(|objects| objects.get(object_id))
            .with_context(|| {
                format!("battle animation object {object_id} is missing from the runtime bundle")
            })?;
        let function = battle_anim_object_function(object_id, object)?;
        let age = animation.frame.saturating_sub(*spawn_frame);
        let state_age = animation.frame.saturating_sub(*state_frame);
        let deleted = if function == "BATTLE_ANIM_FUNC_NULL" {
            object
                .get("frameset")
                .and_then(serde_json::Value::as_str)
                .and_then(|frameset| {
                    visible_null_battle_animation_object_lifetime(&bundle, frameset)
                })
                .is_some_and(|lifetime| age >= lifetime)
        } else {
            visible_battle_anim_object_position(
                function,
                i32::from(*x),
                i32::from(*y),
                *param,
                age,
                *state,
                state_age,
                animation.player_move,
            )
            .is_none()
        };
        if deleted {
            *slot = None;
        }
    }
    let active = slots
        .into_iter()
        .flatten()
        .map(|(event, spawn_frame, state, state_frame)| {
            (
                event,
                animation.frame.saturating_sub(spawn_frame),
                state,
                animation.frame.saturating_sub(state_frame),
            )
        })
        .collect::<Vec<_>>();
    if active.is_empty() {
        return Ok(());
    }
    for (event, age, state, state_age) in active {
        let VisibleMoveObjectCommand::Spawn { object_id, x, y, .. } = &event.command else {
            continue;
        };
        let Some(object) = bundle
            .get("objects")
            .and_then(|objects| objects.get(object_id))
        else {
            anyhow::bail!("battle animation object {object_id} is missing from the runtime bundle");
        };
        let function = battle_anim_object_function(object_id, object)?;
        let VisibleMoveObjectCommand::Spawn { param, .. } = &event.command else {
            continue;
        };
        if function == "BATTLE_ANIM_FUNC_RAIN_SANDSTORM" && *param > 2 {
            anyhow::bail!(
                "battle animation object {object_id} has invalid Rain/Sandstorm variant {param}"
            );
        }
        if function == "BATTLE_ANIM_FUNC_STRING" && *param > 2 {
            anyhow::bail!(
                "battle animation object {object_id} has invalid String Shot variant {param}"
            );
        }
        let Some((animated_x, animated_y)) = visible_battle_anim_object_position(
            function,
            i32::from(*x),
            i32::from(*y),
            *param,
            age,
            state,
            state_age,
            animation.player_move,
        ) else {
            continue;
        };
        let base_frameset_name = object
            .get("frameset")
            .and_then(serde_json::Value::as_str)
            .with_context(|| format!("battle animation object {object_id} has no frameset"))?;
        let (frameset_name, frameset_age) = visible_battle_anim_frameset(
            function,
            base_frameset_name,
            *param,
            age,
            i32::from(*x),
            state,
            state_age,
            i32::from(*y),
            animation.player_move,
        );
        let Some((frame_index, frame)) = battle_anim_frame_at_age(&bundle, frameset_name, frameset_age)? else {
            continue;
        };
        let palette_override = if function == "BATTLE_ANIM_FUNC_SKY_ATTACK" {
            let palette_age = if state == 0 {
                age
            } else {
                age.saturating_sub(state_age).saturating_sub(1)
            };
            if palette_age == 0 {
                Some("PAL_BATTLE_OB_GRAY")
            } else {
                let runtime_param = param.wrapping_add(palette_age as u8);
                Some(match (runtime_param >> 2) % 4 {
                    0 => "PAL_BATTLE_OB_GRAY",
                    1 => "PAL_BATTLE_OB_YELLOW",
                    2 => "PAL_BATTLE_OB_RED",
                    _ => "PAL_BATTLE_OB_BLUE",
                })
            }
        } else {
            None
        };
        let rendered = battle_anim_rendered_frame(
            rendered_art,
            &bundle,
            asset_root,
            object_id,
            object,
            frameset_name,
            frame_index,
            frame,
            !animation.player_move,
            function == "BATTLE_ANIM_FUNC_STRING" && *param == 0,
            function == "BATTLE_ANIM_FUNC_WATER_GUN"
                && age >= u16::try_from((i32::from(*y) - 0x2f).max(0)).unwrap_or(0),
            palette_override,
            images,
        )?;
        let flags = object
            .get("flags")
            .and_then(serde_json::Value::as_i64)
            .with_context(|| format!("battle animation object {object_id} has no numeric flags"))?;
        let relative = flags & 1 != 0;
        let fix_y = object
            .get("fix_y")
            .and_then(serde_json::Value::as_i64)
            .with_context(|| format!("battle animation object {object_id} has no numeric fix_y"))?;
        let dynamic_fix_y = if matches!(
            function,
            "BATTLE_ANIM_FUNC_LEECH_SEED" | "BATTLE_ANIM_FUNC_SPIKES"
        ) {
            i32::from(age.min(33)) * 2
        } else {
            0
        };
        let (source_x, source_y) = if relative && !animation.player_move {
            // InitBattleAnimation mirrors the object's base coordinate for an
            // enemy move; the animation function's offsets are applied after
            // that mirror. Mirroring `animated_x` reverses every X offset.
            let x_offset = (animated_x - i32::from(*x)) & 0xff;
            let mirrored_x = (0xb4_i32 - i32::from(*x) + x_offset) & 0xff;
            let adjusted_y = if fix_y == 0xff {
                (animated_y + 40) & 0xff
            } else {
                let y_offset = (animated_y - i32::from(*y)) & 0xff;
                let enemy_fix_y_adjust = i32::from(matches!(
                    animation.animation_label.as_str(),
                    "BattleAnim_Kinesis"
                        | "BattleAnim_Recover"
                        | "BattleAnim_Softboiled"
                        | "BattleAnim_MilkDrink"
                )) * SOURCE_TILE_SIZE as i32;
                (fix_y as i32 + dynamic_fix_y - i32::from(*y) + y_offset
                    - enemy_fix_y_adjust)
                    & 0xff
            };
            (mirrored_x, adjusted_y)
        } else {
            (animated_x & 0xff, animated_y & 0xff)
        };
        let scale = TILE_SIZE / SOURCE_TILE_SIZE as f32;
        let destination_x = source_x - 8 + i32::from(rendered.offset_x);
        let destination_y = source_y - 16 + i32::from(rendered.offset_y);
        commands.spawn((
            SpriteBundle {
                texture: rendered.sprite.handle.clone(),
                sprite: Sprite { custom_size: Some(rendered.sprite.size), ..default() },
                transform: Transform::from_xyz(
                    PLAYFIELD_LEFT + (destination_x as f32 + rendered.sprite.size.x / scale / 2.0) * scale,
                    PLAYFIELD_TOP - (destination_y as f32 + rendered.sprite.size.y / scale / 2.0) * scale,
                    3.45,
                ),
                ..default()
            },
            BattleCommandMarker,
        ));
    }
    Ok(())
}

fn battle_anim_object_function<'a>(
    object_id: &str,
    object: &'a serde_json::Value,
) -> Result<&'a str> {
    let function = object
        .get("function")
        .with_context(|| format!("battle animation object {object_id} has no function field"))?;
    if function.is_null() {
        return Ok("BATTLE_ANIM_FUNC_NULL");
    }
    function
        .as_str()
        .with_context(|| format!("battle animation object {object_id} has a non-string function"))
}

fn battle_anim_render_bundle(
    rendered_art: &mut RenderedTilesetArt,
    snapshot: &RuntimeShellSnapshot,
) -> Result<serde_json::Value> {
    if rendered_art.battle_anim_bundle_cache.is_none()
        && rendered_art.battle_anim_bundle_error.is_none()
    {
        match serde_json::from_str::<serde_json::Value>(&snapshot.presentation.battle_anim_bundle)
            .context("decode runtime battle animation bundle")
        {
            Ok(bundle) => rendered_art.battle_anim_bundle_cache = Some(bundle),
            Err(error) => rendered_art.battle_anim_bundle_error = Some(error.to_string()),
        }
    }
    rendered_art
        .battle_anim_bundle_cache
        .clone()
        .with_context(|| {
            rendered_art
                .battle_anim_bundle_error
                .clone()
                .unwrap_or_else(|| "battle animation bundle is unavailable".to_string())
        })
}

fn battle_anim_frame_at_age<'a>(
    bundle: &'a serde_json::Value,
    frameset_name: &str,
    age: u16,
) -> Result<Option<(usize, &'a serde_json::Value)>> {
    let frames = bundle
        .get("framesets")
        .and_then(|framesets| framesets.get(frameset_name))
        .and_then(serde_json::Value::as_array)
        .with_context(|| format!("battle animation frameset {frameset_name} is missing"))?;
    let mut remaining = u32::from(age);
    let mut index = 0_usize;
    let mut control_guard = 0_usize;
    let mut last_frame = None;
    loop {
        let frame = frames
            .get(index)
            .with_context(|| format!("battle animation frameset {frameset_name} overran"))?;
        match frame.get("command").and_then(serde_json::Value::as_str) {
            Some("frame") => {
                let duration = frame
                    .get("duration")
                    .and_then(serde_json::Value::as_u64)
                    .with_context(|| format!("battle animation frame {frameset_name}[{index}] has no duration"))?
                    .max(1) as u32;
                if remaining < duration {
                    return Ok(Some((index, frame)));
                }
                remaining -= duration;
                last_frame = Some((index, frame));
                index += 1;
                control_guard = 0;
            }
            Some("wait") => {
                let duration = frame
                    .get("duration")
                    .and_then(serde_json::Value::as_u64)
                    .with_context(|| format!("battle animation wait {frameset_name}[{index}] has no duration"))?
                    .max(1) as u32;
                if remaining < duration {
                    return Ok(last_frame);
                }
                remaining -= duration;
                index += 1;
                control_guard = 0;
            }
            Some("delete") => return Ok(None),
            Some("restart") => {
                index = 0;
                control_guard = control_guard.saturating_add(1);
            }
            Some("end") => {
                index = index.saturating_sub(1);
                control_guard = control_guard.saturating_add(1);
            }
            other => anyhow::bail!("unknown {frameset_name} frame command {other:?}"),
        }
        if control_guard > frames.len().saturating_add(2) {
            anyhow::bail!("battle animation frameset {frameset_name} has a control-command cycle");
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn battle_anim_rendered_frame(
    rendered_art: &mut RenderedTilesetArt,
    bundle: &serde_json::Value,
    asset_root: &AssetRoot,
    object_id: &str,
    object: &serde_json::Value,
    frameset_name: &str,
    frame_index: usize,
    frame: &serde_json::Value,
    enemy_move: bool,
    extra_yflip: bool,
    suppress_enemy_flips: bool,
    palette_override: Option<&str>,
    images: &mut Assets<Image>,
) -> Result<BattleAnimRenderedFrame> {
    let flags = object
        .get("flags")
        .and_then(serde_json::Value::as_i64)
        .with_context(|| format!("battle animation object {object_id} has no numeric flags"))?;
    let frame_xflip = frame
        .get("xflip")
        .and_then(serde_json::Value::as_bool)
        .with_context(|| format!("battle animation frame {frameset_name}[{frame_index}] has no xflip"))?
        ^ (enemy_move && !suppress_enemy_flips && flags & 0x20 != 0);
    let frame_yflip = frame
        .get("yflip")
        .and_then(serde_json::Value::as_bool)
        .with_context(|| format!("battle animation frame {frameset_name}[{frame_index}] has no yflip"))?
        ^ (enemy_move && !suppress_enemy_flips && flags & 0x40 != 0)
        ^ extra_yflip;
    let cache_key = format!(
        "{object_id}:{frameset_name}:{frame_index}:{frame_xflip}:{frame_yflip}:{}",
        palette_override.unwrap_or("default")
    );
    if let Some(rendered) = rendered_art.battle_anim_object_cache.get(&cache_key) {
        return Ok(rendered.clone());
    }
    if let Some(error) = rendered_art.battle_anim_object_errors.get(&cache_key) {
        anyhow::bail!(error.clone());
    }
    let loaded = (|| -> Result<BattleAnimRenderedFrame> {
        let oam_name = frame
            .get("oam_set")
            .and_then(serde_json::Value::as_str)
            .with_context(|| format!("battle animation frame {frameset_name}[{frame_index}] has no OAM set"))?;
        let oam = bundle
            .get("oam_sets")
            .and_then(|sets| sets.get(oam_name))
            .with_context(|| format!("battle animation OAM set {oam_name} is missing"))?;
        let entries = oam
            .get("entries")
            .and_then(serde_json::Value::as_array)
            .with_context(|| format!("battle animation OAM set {oam_name} has no entries"))?;
        let tile_offset = oam
            .get("tile_offset")
            .and_then(serde_json::Value::as_i64)
            .with_context(|| format!("battle animation OAM set {oam_name} has no tile offset"))?;
        let base_frameset_name = object
            .get("frameset")
            .and_then(serde_json::Value::as_str)
            .with_context(|| format!("battle animation object {object_id} has no base frameset"))?;
        let base_frames = bundle
            .get("framesets")
            .and_then(|sets| sets.get(base_frameset_name))
            .and_then(serde_json::Value::as_array)
            .with_context(|| format!("battle animation frameset {base_frameset_name} is missing"))?;
        // OAM tile offsets remain relative to the graphics block loaded for
        // the object's declared frameset. A runtime frameset override changes
        // OAM selection, not the base VRAM address.
        let mut base_offset = None;
        for (base_index, base_frame) in base_frames.iter().enumerate() {
            let Some(base_oam_name) = base_frame
                .get("oam_set")
                .and_then(serde_json::Value::as_str)
            else {
                continue;
            };
            let offset = bundle
                .get("oam_sets")
                .and_then(|sets| sets.get(base_oam_name))
                .with_context(|| {
                    format!(
                        "battle animation base frame {base_frameset_name}[{base_index}] references missing OAM set {base_oam_name}"
                    )
                })?
                .get("tile_offset")
                .and_then(serde_json::Value::as_i64)
                .with_context(|| {
                    format!(
                        "battle animation base OAM set {base_oam_name} has no tile offset"
                    )
                })?;
            base_offset = Some(base_offset.map_or(offset, |current: i64| current.min(offset)));
        }
        let base_offset = base_offset.with_context(|| {
            format!("battle animation base frameset {base_frameset_name} has no OAM tile offset")
        })?;
        let gfx_id = object
            .get("gfx_id")
            .and_then(serde_json::Value::as_str)
            .with_context(|| format!("battle animation object {object_id} has no gfx id"))?;
        let gfx_entry = bundle
            .get("gfx_table")
            .and_then(|table| table.get(gfx_id))
            .and_then(serde_json::Value::as_array)
            .with_context(|| format!("battle animation gfx table entry {gfx_id} is missing"))?;
        let gfx_label = gfx_entry
            .get(1)
            .and_then(serde_json::Value::as_str)
            .with_context(|| format!("battle animation gfx table entry {gfx_id} has no source label"))?;
        let relative_path = bundle
            .get("gfx_sources")
            .and_then(|sources| sources.get(gfx_label))
            .and_then(serde_json::Value::as_str)
            .with_context(|| format!("battle animation gfx source {gfx_label} is missing"))?;
        let compressed_path = asset_root.runtime_assets().join(relative_path);
        let raw_path = if compressed_path.extension().is_some_and(|extension| extension == "lz") {
            compressed_path.with_extension("")
        } else {
            compressed_path
        };
        let tile_data = std::fs::read(&raw_path)
            .with_context(|| format!("read battle animation graphics {}", raw_path.display()))?;
        if tile_data.len() % 16 != 0 {
            anyhow::bail!("battle animation graphics {} are not 2bpp tile aligned", raw_path.display());
        }
        let declared_palette = object
            .get("palette")
            .and_then(serde_json::Value::as_str)
            .with_context(|| format!("battle animation object {object_id} has no palette"))?;
        let palette_name = match palette_override.unwrap_or(declared_palette) {
            "PAL_BATTLE_OB_GRAY" | "PAL_BATTLE_OB_ENEMY" => "gray",
            "PAL_BATTLE_OB_YELLOW" => "yellow",
            "PAL_BATTLE_OB_RED" => "red",
            "PAL_BATTLE_OB_GREEN" => "green",
            "PAL_BATTLE_OB_BLUE" => "blue",
            "PAL_BATTLE_OB_BROWN" | "PAL_BATTLE_OB_PLAYER" => "brown",
            other => anyhow::bail!("unknown battle animation palette {other}"),
        };
        let palette = load_battle_anim_palette(asset_root, palette_name)?;
        let mut pieces = Vec::<(i32, i32, [u8; 8 * 8 * 4])>::new();
        let mut min_x = 0_i32;
        let mut min_y = 0_i32;
        let mut max_x = 0_i32;
        let mut max_y = 0_i32;
        for entry in entries {
            let entry_x = entry
                .get("x")
                .and_then(serde_json::Value::as_i64)
                .with_context(|| format!("battle animation OAM set {oam_name} has an entry without x"))? as i32;
            let entry_y = entry
                .get("y")
                .and_then(serde_json::Value::as_i64)
                .with_context(|| format!("battle animation OAM set {oam_name} has an entry without y"))? as i32;
            let x = if frame_xflip { -(entry_x + 8) } else { entry_x };
            let y = if frame_yflip { -(entry_y + 8) } else { entry_y };
            let entry_xflip = entry
                .get("xflip")
                .and_then(serde_json::Value::as_bool)
                .with_context(|| format!("battle animation OAM set {oam_name} has an entry without xflip"))?;
            let entry_yflip = entry
                .get("yflip")
                .and_then(serde_json::Value::as_bool)
                .with_context(|| format!("battle animation OAM set {oam_name} has an entry without yflip"))?;
            let tile_id = entry
                .get("tile_id")
                .and_then(serde_json::Value::as_i64)
                .with_context(|| format!("battle animation OAM set {oam_name} has an entry without tile_id"))?;
            let tile_index = tile_offset + tile_id - base_offset;
            let tile_start = usize::try_from(tile_index)
                .ok()
                .and_then(|index| index.checked_mul(16))
                .context("battle animation tile index overflow")?;
            let tile = tile_data
                .get(tile_start..tile_start + 16)
                .with_context(|| format!("battle animation tile {tile_index} is missing from {gfx_label}"))?;
            let mut pixels = [0_u8; 8 * 8 * 4];
            for output_y in 0..8_usize {
                for output_x in 0..8_usize {
                    let source_x = if entry_xflip ^ frame_xflip { 7 - output_x } else { output_x };
                    let source_y = if entry_yflip ^ frame_yflip { 7 - output_y } else { output_y };
                    let bit = 1 << (7 - source_x);
                    let colour = ((tile[source_y * 2] & bit != 0) as usize)
                        | (((tile[source_y * 2 + 1] & bit != 0) as usize) << 1);
                    if colour == 0 {
                        continue;
                    }
                    let target = (output_y * 8 + output_x) * 4;
                    pixels[target..target + 4].copy_from_slice(&palette[colour]);
                }
            }
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x + 8);
            max_y = max_y.max(y + 8);
            pieces.push((x, y, pixels));
        }
        if pieces.is_empty() {
            anyhow::bail!("battle animation OAM set {oam_name} rendered no pieces");
        }
        let width = usize::try_from((max_x - min_x).max(1)).context("battle animation width overflow")?;
        let height = usize::try_from((max_y - min_y).max(1)).context("battle animation height overflow")?;
        let mut composite = vec![0_u8; width * height * 4];
        for (x, y, pixels) in pieces {
            for tile_y in 0..8_usize {
                for tile_x in 0..8_usize {
                    let source = (tile_y * 8 + tile_x) * 4;
                    if pixels[source + 3] == 0 {
                        continue;
                    }
                    let target_x = usize::try_from(x - min_x).unwrap() + tile_x;
                    let target_y = usize::try_from(y - min_y).unwrap() + tile_y;
                    let target = (target_y * width + target_x) * 4;
                    composite[target..target + 4].copy_from_slice(&pixels[source..source + 4]);
                }
            }
        }
        let mut image = Image::new(
            Extent3d { width: width as u32, height: height as u32, depth_or_array_layers: 1 },
            TextureDimension::D2,
            composite,
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        );
        image.sampler = ImageSampler::nearest();
        let scale = TILE_SIZE / SOURCE_TILE_SIZE as f32;
        Ok(BattleAnimRenderedFrame {
            sprite: SpriteFrame {
                handle: images.add(image),
                size: Vec2::new(width as f32 * scale, height as f32 * scale),
            },
            offset_x: i16::try_from(min_x).context("battle animation X offset overflow")?,
            offset_y: i16::try_from(min_y).context("battle animation Y offset overflow")?,
        })
    })();
    match loaded {
        Ok(rendered) => {
            rendered_art
                .battle_anim_object_cache
                .insert(cache_key.clone(), rendered.clone());
            Ok(rendered)
        }
        Err(error) => {
            rendered_art
                .battle_anim_object_errors
                .insert(cache_key, error.to_string());
            Err(error)
        }
    }
}

fn load_battle_anim_palette(asset_root: &AssetRoot, requested: &str) -> Result<[[u8; 4]; 4]> {
    let path = asset_root.runtime_assets().join("gfx/battle_anims/battle_anims.pal");
    let source = std::fs::read_to_string(&path)
        .with_context(|| format!("read battle animation palettes {}", path.display()))?;
    let mut section = "";
    let mut colours = Vec::new();
    for line in source.lines() {
        let line = line.trim();
        if let Some(name) = line.strip_prefix(';') {
            section = name.trim();
            continue;
        }
        if section != requested || !line.starts_with("RGB") {
            continue;
        }
        let channels = line[3..]
            .split(',')
            .map(|channel| channel.trim().parse::<u8>())
            .collect::<std::result::Result<Vec<_>, _>>()
            .with_context(|| format!("parse battle animation palette {requested}"))?;
        if channels.len() != 3 {
            anyhow::bail!("battle animation palette {requested} has malformed RGB data");
        }
        colours.push([
            ((u16::from(channels[0]) * 255 + 15) / 31) as u8,
            ((u16::from(channels[1]) * 255 + 15) / 31) as u8,
            ((u16::from(channels[2]) * 255 + 15) / 31) as u8,
            if colours.is_empty() { 0 } else { 255 },
        ]);
    }
    colours
        .try_into()
        .map_err(|_| anyhow::anyhow!("battle animation palette {requested} must contain four colours"))
}

fn spawn_battle_command_menu(
    commands: &mut Commands,
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    battle: &crate::RuntimeBattleSnapshot,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) -> Result<()> {
    require_bitmap_font_art(rendered_art, asset_root, images)?;
    if runtime_shell
        .visible_capture_animation
        .as_ref()
        .is_some_and(|animation| animation.started)
        || runtime_shell.visible_frontpic_animation.is_some()
        || runtime_shell
            .visible_move_animations
            .front()
            .is_some_and(|animation| animation.started)
        || runtime_shell.visible_send_out_animation.is_some()
        || runtime_shell.visible_trainer_exit_animation.is_some()
        || runtime_shell
        .battle_hp_tween
        .as_ref()
        .is_some_and(visible_battle_hp_tween_active)
        || runtime_shell
            .battle_exp_tween
            .as_ref()
            .is_some_and(|tween| tween.started)
    {
        return Ok(());
    }
    if battle_window_frame_art(rendered_art, asset_root, images).is_none() {
        let frame_id = rendered_art.selected_window_frame_id.clamp(1, 8);
        anyhow::bail!(
            "{}",
            rendered_art
                .window_frame_errors
                .get(&frame_id)
                .cloned()
                .unwrap_or_else(|| "battle window frame art is unavailable".to_string())
        );
    }
    if let Some(stats) = runtime_shell
        .battle_level_stats
        .front()
        .filter(|stats| stats.active)
    {
        spawn_battle_window(
            commands, rendered_art, asset_root, images, 9.0, 0.0, 11.0, 12.0, 4.0,
        );
        for (index, (label, value)) in [
            ("ATTACK", stats.attack),
            ("DEFENSE", stats.defense),
            ("SPCL.ATK", stats.special_attack),
            ("SPCL.DEF", stats.special_defense),
            ("SPEED", stats.speed),
        ]
        .iter()
        .enumerate()
        {
            let label_row = 1.0 + index as f32 * 2.0;
            let (x, y) = battle_hud_tile_origin(11.0, label_row);
            spawn_battle_command_bitmap_text(
                commands, rendered_art, asset_root, images, label, x, y, 4.2,
            );
            let (x, y) = battle_hud_tile_origin(15.0, label_row + 1.0);
            spawn_battle_command_bitmap_text(
                commands, rendered_art, asset_root, images, &format!("{:>3}", value), x, y, 4.2,
            );
        }
        return Ok(());
    }
    if let Some(message) = runtime_shell.battle_messages.front() {
        spawn_battle_window(
            commands,
            rendered_art,
            asset_root,
            images,
            BATTLE_TEXT_BOX_LEFT_TILE,
            BATTLE_TEXT_BOX_TOP_TILE,
            BATTLE_TEXT_BOX_WIDTH_TILES,
            BATTLE_TEXT_BOX_HEIGHT_TILES,
            3.5,
        );
        for (line_index, line) in visible_battle_message_lines(runtime_shell, message)
            .iter()
            .enumerate()
        {
            // SpeechTextbox prints at TEXTBOX_INNERY and INNERY + 2. Once a
            // third line begins, TextScroll moves the previous baseline up.
            let (x, y) = battle_hud_tile_origin(1.0, 14.0 + line_index as f32 * 2.0);
            spawn_battle_command_bitmap_text(
                commands,
                rendered_art,
                asset_root,
                images,
                line,
                x,
                y,
                3.8,
            );
        }
        if visible_battle_message_is_complete(runtime_shell, message)
            && runtime_shell.lcd_animation_frame & (1 << 4) != 0
        {
            let (x, y) = battle_hud_tile_origin(18.0, 16.0);
            spawn_battle_command_bitmap_text(
                commands,
                rendered_art,
                asset_root,
                images,
                "▼",
                x,
                y,
                3.85,
            );
        }
        return Ok(());
    }
    if snapshot.pending_move_learn.is_some() {
        spawn_battle_pending_move_learn_screen(
            commands, snapshot, runtime_shell, rendered_art, asset_root, images,
        )?;
        return Ok(());
    }
    if runtime_shell.battle_pack_target_mode == Some(BattlePackTargetMode::PartyPokemon)
        || (runtime_shell.battle_pack_target_mode == Some(BattlePackTargetMode::PartyMove)
            && runtime_shell.party_move_cursor.is_none())
    {
        spawn_battle_party_menu(
            commands, snapshot, runtime_shell, rendered_art, asset_root, images,
        )
        ?;
        return Ok(());
    }
    if runtime_shell.battle_pack_target_mode == Some(BattlePackTargetMode::PartyMove)
        && runtime_shell.party_move_cursor.is_some()
    {
        spawn_battle_pack_move_target_screen(
            commands, snapshot, runtime_shell, rendered_art, asset_root, images,
        )?;
        return Ok(());
    }
    if runtime_shell.battle_pack_target_mode.is_none()
        && (runtime_shell.bag_cursor.is_some()
            || runtime_shell.ball_cursor.is_some()
            || runtime_shell.key_item_cursor.is_some()
            || runtime_shell.tmhm_cursor.is_some())
    {
        spawn_battle_pack_screen(
            commands, snapshot, runtime_shell, rendered_art, asset_root, images,
        )?;
        return Ok(());
    }
    if runtime_shell.battle_move_cursor.is_some() {
        spawn_battle_move_menu(
            commands,
            snapshot,
            runtime_shell,
            battle,
            rendered_art,
            asset_root,
            images,
        )?;
        return Ok(());
    }
    if runtime_shell.battle_party_summary_open {
        spawn_field_party_summary_screen(
            commands,
            snapshot,
            runtime_shell,
            rendered_art,
            asset_root,
            images,
        )?;
        return Ok(());
    }
    if runtime_shell.battle_switch_cursor.is_some() {
        spawn_battle_party_menu(
            commands,
            snapshot,
            runtime_shell,
            rendered_art,
            asset_root,
            images,
        )
        ?;
        return Ok(());
    }
    let entries = visible_battle_command_menu_entries(snapshot, runtime_shell, battle)?;
    if entries.is_empty() {
        return Ok(());
    }
    if runtime_shell.battle_faint_prompt_cursor.is_some()
        || runtime_shell.battle_shift_prompt_cursor.is_some()
    {
        spawn_battle_yes_no_prompt(
            commands,
            runtime_shell,
            rendered_art,
            asset_root,
            images,
            &entries,
        );
        return Ok(());
    }
    if battle_command_entries_are_main_menu(&entries) {
        spawn_battle_main_command_menu(
            commands,
            snapshot,
            runtime_shell,
            battle,
            rendered_art,
            asset_root,
            images,
            &entries,
        )?;
        return Ok(());
    }
    spawn_battle_window(
        commands,
        rendered_art,
        asset_root,
        images,
        BATTLE_TEXT_BOX_LEFT_TILE,
        BATTLE_TEXT_BOX_TOP_TILE,
        BATTLE_TEXT_BOX_WIDTH_TILES,
        BATTLE_TEXT_BOX_HEIGHT_TILES,
        3.5,
    );
    let two_columns = scene_menu_uses_two_columns(&entries);
    for (index, entry) in entries.iter().enumerate() {
        let (tile_x, tile_y) = battle_submenu_entry_tile(index, two_columns);
        let (x, y) = battle_hud_tile_origin(tile_x, tile_y);
        let display_entry = compact_scene_label(
            &animated_battle_cursor_entry(runtime_shell, entry),
            if two_columns { 9 } else { 18 },
        );
        spawn_battle_command_bitmap_text(
            commands,
            rendered_art,
            asset_root,
            images,
            &display_entry,
            x,
            y,
            3.8,
        );
    }
    Ok(())
}

fn spawn_battle_pending_move_learn_screen(
    commands: &mut Commands,
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) -> Result<()> {
    let pending = snapshot
        .pending_move_learn
        .as_ref()
        .context("battle move-learning screen has no pending move")?;
    let pending_move = snapshot
        .moves
        .iter()
        .find(|move_data| move_data.move_id == pending.learned_move.name)
        .with_context(|| {
            format!(
                "battle move-learning metadata {} is missing",
                pending.learned_move.name
            )
        })?;
    let move_name = pending_move.name.replace('_', " ");
    let slot = snapshot
        .party
        .slots
        .iter()
        .find(|slot| slot.index == pending.party_index)
        .with_context(|| {
            format!(
                "battle move-learning party slot {} is missing",
                pending.party_index
            )
        })?;
    for learned in &slot.pokemon.moves {
        snapshot
            .moves
            .iter()
            .find(|move_data| move_data.move_id == learned.name)
            .with_context(|| {
                format!("battle move-forget metadata {} is missing", learned.name)
            })?;
    }

    if runtime_shell.move_learn_forget_menu_open {
        let option_count = slot.pokemon.moves.len() + 1;
        let selected = strict_readonly_cursor_index(
            &runtime_shell.party_move_cursor,
            &party_move_cursor_surface_id(pending.party_index),
            option_count,
        )
        .context("battle move-forget screen has no valid cursor")?;
        commands.spawn((
            SpriteBundle {
                sprite: Sprite { color: Color::rgb(1.0, 1.0, 1.0), custom_size: Some(Vec2::new(PLAYFIELD_WIDTH, PLAYFIELD_HEIGHT)), ..default() },
                transform: Transform::from_xyz(0.0, 0.0, 3.4),
                ..default()
            },
            BattleCommandMarker,
        ));
        for (line_index, line) in ["Which move should", "be forgotten?"].iter().enumerate() {
            let (x, y) = battle_hud_tile_origin(1.0, 1.0 + line_index as f32);
            spawn_battle_command_bitmap_text(
                commands, rendered_art, asset_root, images, line, x, y, 3.8,
            );
        }
        for (index, learned) in slot.pokemon.moves.iter().enumerate().take(4) {
            let row = 3.0 + index as f32 * 2.0;
            let (x, y) = battle_hud_tile_origin(1.0, row);
            spawn_battle_command_bitmap_text(
                commands, rendered_art, asset_root, images,
                &format!(
                    "{}{}",
                    if selected == index { battle_cursor_glyph(runtime_shell) } else { " " },
                    battle_move_display_name(snapshot, &learned.name),
                ), x, y, 3.8,
            );
            let (x, y) = battle_hud_tile_origin(11.0, row + 1.0);
            spawn_battle_command_bitmap_text(commands, rendered_art, asset_root, images, &visible_move_pp_text(snapshot, learned), x, y, 3.8);
        }
        let (x, y) = battle_hud_tile_origin(1.0, 12.0);
        spawn_battle_command_bitmap_text(
            commands, rendered_art, asset_root, images,
            &format!(
                "{}CANCEL",
                if selected == slot.pokemon.moves.len() { battle_cursor_glyph(runtime_shell) } else { " " },
            ), x, y, 3.8,
        );
        let (x, y) = battle_hud_tile_origin(1.0, 15.0);
        spawn_battle_command_bitmap_text(commands, rendered_art, asset_root, images, &format!("Trying to learn {move_name}."), x, y, 3.8);
        return Ok(());
    }

    spawn_battle_window(
        commands, rendered_art, asset_root, images,
        BATTLE_TEXT_BOX_LEFT_TILE, BATTLE_TEXT_BOX_TOP_TILE,
        BATTLE_TEXT_BOX_WIDTH_TILES, BATTLE_TEXT_BOX_HEIGHT_TILES, 3.5,
    );
    let prompt = match runtime_shell.move_learn_decision {
        Some(VisibleTmHmDecision::ForgetMove) => format!("Delete a move to make room\nfor {move_name}?"),
        Some(VisibleTmHmDecision::StopLearning) => format!("Stop learning {move_name}?"),
        None => format!(
            "But {} can't learn more\nthan four moves.\nDelete an older move to\nmake room for {move_name}?",
            slot.pokemon.nickname
        ),
    };
    for (line_index, line) in wrap_boot_text_for_box(&prompt, 18, 4).iter().enumerate() {
        let (x, y) = battle_hud_tile_origin(1.0, 13.0 + line_index as f32);
        spawn_battle_command_bitmap_text(commands, rendered_art, asset_root, images, line, x, y, 3.8);
    }
    if runtime_shell.move_learn_decision_cursor.is_some() {
        let selected = strict_readonly_cursor_index(
            &runtime_shell.move_learn_decision_cursor,
            "move-learn:decision",
            2,
        )
        .context("battle move-learning decision cursor is invalid")?;
        spawn_battle_window(
            commands,
            rendered_art,
            asset_root,
            images,
            FIELD_YES_NO_LEFT_TILE,
            FIELD_YES_NO_TOP_TILE,
            FIELD_YES_NO_WIDTH_TILES,
            FIELD_YES_NO_HEIGHT_TILES,
            4.0,
        );
        for (index, label) in ["YES", "NO"].iter().enumerate() {
            let (x, y) = battle_hud_tile_origin(
                FIELD_YES_NO_LEFT_TILE,
                FIELD_YES_NO_TOP_TILE + 1.0 + index as f32,
            );
            spawn_battle_command_bitmap_text(
                commands, rendered_art, asset_root, images,
                &format!(
                    "{}{label}",
                    if selected == index { battle_cursor_glyph(runtime_shell) } else { " " },
                ), x, y, 4.2,
            );
        }
    }
    Ok(())
}

fn spawn_visible_capture_animation(
    commands: &mut Commands,
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) -> Result<()> {
    let Some(animation) = runtime_shell
        .visible_capture_animation
        .as_ref()
        .filter(|animation| animation.ball_visible())
    else {
        return Ok(());
    };
    if !animation.blocked {
        let master_ball = animation.ball_id.eq_ignore_ascii_case("MASTER_BALL");
        let _drop_start = if master_ball { 164 } else { 92 };
        // BreakFree sets the retained ball directly to stage 11 before its
        // poof and ENTER_MON wait, which deinitializes it immediately.
        if !animation.caught
            && animation.frame >= animation.total_frames().saturating_sub(34)
        {
            return Ok(());
        }
    }
    let (screen_x, screen_y) = if animation.blocked {
        let Some((x, y)) = visible_capture_object_position(
            64,
            92,
            0x20,
            0x70,
            animation.frame,
            true,
        ) else {
            return Ok(());
        };
        (x as f32, y as f32)
    } else if animation.frame < 36 {
        let master_ball = animation.ball_id.eq_ignore_ascii_case("MASTER_BALL");
        let (x, y) = visible_capture_object_position(
            if master_ball { 64 } else { 68 },
            92,
            if master_ball { 0x20 } else { 0x40 },
            0x88,
            animation.frame,
            false,
        )
        .context("ordinary Poké Ball throw deinitialized during its flight")?;
        (x as f32, y as f32)
    } else if animation.frame < 68 {
        // The second object is forced into Poké Ball stage 7. Stage 8 uses a
        // radius-$20 sine for 32 updates before deinitializing.
        let age = animation.frame.saturating_sub(36);
        let angle = 0_u8.wrapping_sub(age as u8);
        (136.0, 65.0 + visible_battle_anim_sine(angle, 0x20) as f32)
    } else {
        let master_ball = animation.ball_id.eq_ignore_ascii_case("MASTER_BALL");
        let (_, landed_y) = visible_capture_object_position(
            if master_ball { 64 } else { 68 },
            92,
            if master_ball { 0x20 } else { 0x40 },
            0x88,
            36,
            false,
        )
        .context("capture throw did not reach its retained landing object")?;
        let drop_start = if master_ball { 164 } else { 92 };
        let drop_age = animation
            .frame
            .saturating_sub(drop_start)
            .min(127);
        let amplitude = 0x10_u8.saturating_sub(((drop_age / 32) as u8) * 4);
        let angle = 0_u8.wrapping_sub(drop_age as u8);
        let y_offset = if animation.frame >= drop_start
            && animation.frame < animation.shake_setup_frame()
        {
            visible_battle_anim_sine(angle, amplitude)
        } else {
            0
        };
        (136.0, landed_y as f32 + y_offset as f32)
    };
    let bundle = battle_anim_render_bundle(rendered_art, snapshot)?;
    let object_id = if animation.blocked {
        "BATTLE_ANIM_OBJ_POKE_BALL_BLOCKED"
    } else {
        "BATTLE_ANIM_OBJ_POKE_BALL"
    };
    let object = bundle
        .get("objects")
        .and_then(|objects| objects.get(object_id))
        .with_context(|| format!("battle animation object {object_id} is missing"))?;
    let drop_start = if animation.ball_id.eq_ignore_ascii_case("MASTER_BALL") { 164 } else { 92 };
    let (frameset, frameset_age) = if animation.blocked || animation.frame < 36 {
        ("BATTLE_ANIM_FRAMESET_POKE_BALL_1", animation.frame)
    } else if animation.frame < 68 {
        ("BATTLE_ANIM_FRAMESET_POKE_BALL_2", 0)
    } else if animation.frame < drop_start {
        // The retained throw object switches to the flattened OAM set while
        // RETURN_MON collapses the target; it remains visible beneath the
        // separate opening object rather than disappearing for 24 frames.
        ("BATTLE_ANIM_FRAMESET_POKE_BALL_3", 0)
    } else {
        let bounce_age = animation.frame.saturating_sub(drop_start);
        if bounce_age < 128 {
            ("BATTLE_ANIM_FRAMESET_POKE_BALL_1", bounce_age)
        } else {
            let first_check = animation.first_shake_check_frame();
            let completed_check = animation.frame.saturating_sub(first_check) / 48 + 1;
            let wobble_start = first_check.saturating_add(48 * completed_check.saturating_sub(1));
            let successful_wobble = animation.frame >= first_check
                && if animation.caught {
                    completed_check < u16::from(animation.animation_shakes)
                } else {
                    completed_check <= u16::from(animation.animation_shakes)
                };
            if successful_wobble {
                (
                    "BATTLE_ANIM_FRAMESET_POKE_BALL_5",
                    animation.frame.saturating_sub(wobble_start),
                )
            } else {
                ("BATTLE_ANIM_FRAMESET_POKE_BALL_4", 0)
            }
        }
    };
    let Some((frame_index, frame)) = battle_anim_frame_at_age(&bundle, frameset, frameset_age)? else {
        return Ok(());
    };
    let rendered = battle_anim_rendered_frame(
        rendered_art,
        &bundle,
        asset_root,
        object_id,
        object,
        frameset,
        frame_index,
        frame,
        false,
        false,
        true,
        Some(match animation.ball_id.as_str() {
            "MASTER_BALL" => "PAL_BATTLE_OB_GREEN",
            "ULTRA_BALL" | "FRIEND_BALL" => "PAL_BATTLE_OB_YELLOW",
            "GREAT_BALL" | "LURE_BALL" | "FAST_BALL" => "PAL_BATTLE_OB_BLUE",
            "HEAVY_BALL" | "MOON_BALL" => "PAL_BATTLE_OB_GRAY",
            "LEVEL_BALL" => "PAL_BATTLE_OB_BROWN",
            _ => "PAL_BATTLE_OB_RED",
        }),
        images,
    )?;
    let source_scale = TILE_SIZE / SOURCE_TILE_SIZE as f32;
    let destination_x = screen_x - 8.0 + f32::from(rendered.offset_x);
    let destination_y = screen_y - 16.0 + f32::from(rendered.offset_y);
    let x = PLAYFIELD_LEFT
        + (destination_x + rendered.sprite.size.x / source_scale / 2.0) * source_scale;
    let y = PLAYFIELD_TOP
        - (destination_y + rendered.sprite.size.y / source_scale / 2.0) * source_scale;
    commands.spawn((
        SpriteBundle {
            texture: rendered.sprite.handle.clone(),
            sprite: Sprite {
                custom_size: Some(rendered.sprite.size),
                ..default()
            },
            transform: Transform::from_xyz(x, y, 4.1),
            ..default()
        },
        BattleCommandMarker,
    ));
    Ok(())
}

fn visible_capture_object_position(
    start_x: i32,
    start_y: i32,
    amplitude: u8,
    target_x: i32,
    frame: u16,
    blocked: bool,
) -> Option<(i32, i32)> {
    let mut x = start_x;
    let mut y = start_y;
    let mut y_offset = 0;
    let mut angle = 0_u8;
    let mut stage = 0_u8;
    for _ in 0..frame {
        if stage == 0 {
            stage = 1;
            continue;
        }
        if stage == 1 {
            if x < target_x {
                x = (x + 2) & 0xff;
                y = (y - 1) & 0xff;
                y_offset = visible_battle_anim_sine(angle, amplitude);
                angle = angle.wrapping_sub(1);
                continue;
            }
            if !blocked {
                y = (y + y_offset) & 0xff;
                y_offset = 0;
                stage = 2;
                continue;
            }
            stage = 2;
        }
        if blocked && stage == 2 {
            if y >= 0x80 {
                return None;
            }
            y = (y + 4) & 0xff;
            x = (x - 2) & 0xff;
        }
    }
    Some((x, (y + y_offset) & 0xff))
}

fn spawn_visible_send_out_poof(
    commands: &mut Commands,
    runtime_shell: &BevyRuntimeShell,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) -> Result<()> {
    let Some(animation) = runtime_shell.visible_send_out_animation.as_ref() else {
        return Ok(());
    };
    let frame_index = usize::from(animation.frame / 3);
    if frame_index >= 4 {
        return Ok(());
    }
    let frames = battle_send_out_poof_frames(rendered_art, asset_root, images)?;
    let frame = &frames[frame_index];
    let (screen_x, screen_y) = match animation.side {
        crate::core::battle::turn::BattleSide::Player => (40.0, 100.0),
        crate::core::battle::turn::BattleSide::Enemy => (120.0, 36.0),
    };
    let source_scale = TILE_SIZE / SOURCE_TILE_SIZE as f32;
    // The 40x40 smoke cache is already centered about the animation object;
    // only the Game Boy OAM hardware origin offsets remain.
    let x = PLAYFIELD_LEFT + (screen_x - 8.0) * source_scale;
    let y = PLAYFIELD_TOP - (screen_y - 16.0) * source_scale;
    commands.spawn((
        SpriteBundle {
            texture: frame.handle.clone(),
            sprite: Sprite {
                custom_size: Some(frame.size * source_scale),
                ..default()
            },
            transform: Transform::from_xyz(x, y, 4.15),
            ..default()
        },
        BattleCommandMarker,
    ));
    Ok(())
}

fn spawn_visible_fishing_animation(
    commands: &mut Commands,
    runtime_shell: &BevyRuntimeShell,
    facing: Direction,
    player_x: f32,
    player_y: f32,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) -> Result<()> {
    let Some(animation) = runtime_shell.visible_fishing_animation else {
        return Ok(());
    };
    let rod = fishing_rod_frame(rendered_art, asset_root, facing, images)?;
    // FacingFish* OAM positions, measured from the 16x16 player's top-left.
    let (dx, dy) = match facing {
        Direction::Down => (-TILE_SIZE * 0.5, -TILE_SIZE * 1.5),
        Direction::Up => (-TILE_SIZE * 0.5, TILE_SIZE * 1.5),
        Direction::Left => (-TILE_SIZE * 1.5, -TILE_SIZE * 0.125),
        Direction::Right => (TILE_SIZE * 1.5, -TILE_SIZE * 0.125),
    };
    commands.spawn((
        SpriteBundle {
            texture: rod.handle.clone(),
            sprite: Sprite { custom_size: Some(rod.size), ..default() },
            transform: Transform::from_xyz(player_x + dx, player_y + dy, 2.8),
            ..default()
        },
        PlayerFacingMarker,
    ));
    if animation.phase == VisibleFishingPhase::Pause {
        let shock = emote_frame_for_art(rendered_art, asset_root, "EMOTE_SHOCK", images)
            .context("required fishing bite emote could not be rendered")?;
        commands.spawn((
            SpriteBundle {
                texture: shock.handle,
                sprite: Sprite { custom_size: Some(shock.size), ..default() },
                transform: Transform::from_xyz(
                    player_x,
                    player_y + TILE_SIZE * 1.35,
                    2.9,
                ),
                ..default()
            },
            PlayerFacingMarker,
        ));
    }
    Ok(())
}

fn fishing_rod_frame<'a>(
    rendered_art: &'a mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    facing: Direction,
    images: &mut Assets<Image>,
) -> Result<&'a SpriteFrame> {
    if rendered_art.fishing_rod_cache.is_none() && rendered_art.fishing_rod_error.is_none() {
        let loaded = (|| -> Result<[SpriteFrame; 3]> {
            let path = asset_root
                .runtime_assets()
                .join("gfx/overworld/fishing_rod.png");
            let source = image::open(&path)
                .with_context(|| format!("decode fishing rod PNG {}", path.display()))?
                .to_rgba8();
            if source.width() != 8 || source.height() != 16 {
                anyhow::bail!("fishing rod sheet must contain exactly tiles $fc and $fd");
            }
            let make_tile = |tile: u32, mirror: bool, images: &mut Assets<Image>| {
                let mut pixels = vec![0_u8; 8 * 8 * 4];
                for y in 0..8_u32 {
                    for x in 0..8_u32 {
                        let source_x = if mirror { 7 - x } else { x };
                        let pixel = source.get_pixel(source_x, tile * 8 + y);
                        let target = ((y * 8 + x) * 4) as usize;
                        if pixel[0] <= 240 || pixel[1] <= 240 || pixel[2] <= 240 {
                            pixels[target..target + 4].copy_from_slice(&[32, 32, 32, 255]);
                        }
                    }
                }
                let mut image = Image::new(
                    Extent3d { width: 8, height: 8, depth_or_array_layers: 1 },
                    TextureDimension::D2,
                    pixels,
                    TextureFormat::Rgba8UnormSrgb,
                    RenderAssetUsages::default(),
                );
                image.sampler = ImageSampler::nearest();
                SpriteFrame { handle: images.add(image), size: Vec2::splat(TILE_SIZE) }
            };
            Ok([
                make_tile(0, false, images),
                make_tile(1, false, images),
                make_tile(1, true, images),
            ])
        })();
        match loaded {
            Ok(frame) => rendered_art.fishing_rod_cache = Some(frame),
            Err(error) => rendered_art.fishing_rod_error = Some(error.to_string()),
        }
    }
    let frames = rendered_art.fishing_rod_cache.as_ref().with_context(|| {
        rendered_art
            .fishing_rod_error
            .clone()
            .unwrap_or_else(|| "fishing rod art is unavailable".to_string())
    })?;
    Ok(match facing {
        Direction::Down | Direction::Up => &frames[0],
        Direction::Right => &frames[1],
        Direction::Left => &frames[2],
    })
}

fn fishing_player_frame(
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    female: bool,
    facing: Direction,
    palette_id: u8,
    time_of_day: &str,
    images: &mut Assets<Image>,
) -> Result<SpriteFrame> {
    let time = normalize_tileset_time_of_day(time_of_day);
    let key = format!("{}:{facing:?}:{}:{time}", if female { "kris" } else { "chris" }, palette_id & 7);
    if !rendered_art.fishing_player_cache.contains_key(&key)
        && !rendered_art.fishing_player_errors.contains_key(&key)
    {
        let loaded = (|| -> Result<SpriteFrame> {
            let assets = asset_root.runtime_assets();
            let player_name = if female { "kris" } else { "chris" };
            let normal_path = assets.join("gfx/sprites").join(format!("{player_name}.png"));
            let fish_path = assets
                .join("gfx/overworld")
                .join(format!("{player_name}_fish.png"));
            let normal = image::open(&normal_path)
                .with_context(|| format!("decode player sprite PNG {}", normal_path.display()))?
                .to_rgba8();
            let fish = image::open(&fish_path)
                .with_context(|| format!("decode fishing sprite PNG {}", fish_path.display()))?
                .to_rgba8();
            if normal.width() != 16 || normal.height() < 48 || fish.width() != 16 || fish.height() < 24 {
                anyhow::bail!("fishing sprite sources must contain the three 16-pixel facings");
            }
            let (normal_frame, fish_row, mirror) = match facing {
                Direction::Down => (0_u32, 0_u32, false),
                Direction::Up => (1, 1, false),
                Direction::Left => (2, 2, false),
                Direction::Right => (2, 2, true),
            };
            let mut combined = image::RgbaImage::new(16, 16);
            for y in 0..16_u32 {
                for x in 0..16_u32 {
                    combined.put_pixel(x, y, *normal.get_pixel(x, normal_frame * 16 + y));
                }
            }
            // LoadFishingGFX replaces vTiles $02/$03, $06/$07, or $0a/$0b:
            // the lower 8-pixel pair of the direction's ordinary 16x16 body.
            for y in 0..8_u32 {
                for x in 0..16_u32 {
                    combined.put_pixel(x, y + 8, *fish.get_pixel(x, fish_row * 8 + y));
                }
            }
            let palette_bank = load_npc_sprite_palette_bank(asset_root, &time)?;
            let palette = palette_bank
                .get(usize::from(palette_id & 7))
                .with_context(|| {
                    format!(
                        "fishing player palette {} is missing from the {time} NPC palette bank",
                        palette_id & 7
                    )
                })?;
            Ok(create_sprite_frame(&combined, 16, 0, palette, mirror, images))
        })();
        match loaded {
            Ok(frame) => {
                rendered_art.fishing_player_cache.insert(key.clone(), frame);
            }
            Err(error) => {
                rendered_art.fishing_player_errors.insert(key.clone(), error.to_string());
            }
        }
    }
    rendered_art
        .fishing_player_cache
        .get(&key)
        .cloned()
        .with_context(|| {
            rendered_art
                .fishing_player_errors
                .get(&key)
                .cloned()
                .unwrap_or_else(|| "fishing player art is unavailable".to_string())
        })
}

fn battle_send_out_poof_frames<'a>(
    rendered_art: &'a mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) -> Result<&'a [SpriteFrame; 4]> {
    if rendered_art.battle_send_out_poof_cache.is_none()
        && rendered_art.battle_send_out_poof_error.is_none()
    {
        let loaded = (|| -> Result<[SpriteFrame; 4]> {
            let data = std::fs::read(
                asset_root.runtime_assets().join("gfx/battle_anims/smoke.2bpp"),
            )
            .context("read battle send-out smoke graphics")?;
            let compose = |tile_offset: usize, expanded: bool, images: &mut Assets<Image>| -> Result<SpriteFrame> {
                let mut pixels = vec![0_u8; 40 * 40 * 4];
                let base_positions = [-16_i16, -8, 0, 8];
                let expanded_positions = [-20_i16, -12, 4, 12];
                let positions = if expanded { expanded_positions } else { base_positions };
                let tile_ids = [0_usize, 1, 1, 0, 2, 3, 3, 2, 2, 3, 3, 2, 0, 1, 1, 0];
                for row in 0..4 {
                    for col in 0..4 {
                        let tile_index = tile_offset + tile_ids[row * 4 + col];
                        let offset = tile_index * 16;
                        let tile = data
                            .get(offset..offset + 16)
                            .with_context(|| format!("battle smoke tile {tile_index} is missing"))?;
                        for source_y in 0..8_usize {
                            for source_x in 0..8_usize {
                                let flip_x = col >= 2;
                                let flip_y = row >= 2;
                                let sample_x = if flip_x { 7 - source_x } else { source_x };
                                let sample_y = if flip_y { 7 - source_y } else { source_y };
                                let sample_lo = tile[sample_y * 2];
                                let sample_hi = tile[sample_y * 2 + 1];
                                let bit = 1 << (7 - sample_x);
                                let level = ((sample_hi & bit != 0) as u8) << 1
                                    | (sample_lo & bit != 0) as u8;
                                if level == 0 {
                                    continue;
                                }
                                let target_x = i32::from(positions[col]) + source_x as i32 + 20;
                                let target_y = i32::from(positions[row]) + source_y as i32 + 20;
                                if !(0..40).contains(&target_x) || !(0..40).contains(&target_y) {
                                    continue;
                                }
                                let output = ((target_y * 40 + target_x) * 4) as usize;
                                let shade = match level { 1 => 224, 2 => 128, _ => 40 };
                                pixels[output] = shade;
                                pixels[output + 1] = shade;
                                pixels[output + 2] = shade;
                                pixels[output + 3] = 255;
                            }
                        }
                    }
                }
                let mut image = Image::new(
                    Extent3d { width: 40, height: 40, depth_or_array_layers: 1 },
                    TextureDimension::D2,
                    pixels,
                    TextureFormat::Rgba8UnormSrgb,
                    RenderAssetUsages::default(),
                );
                image.sampler = ImageSampler::nearest();
                Ok(SpriteFrame { handle: images.add(image), size: Vec2::splat(40.0) })
            };
            Ok([
                compose(0, false, images)?,
                compose(4, false, images)?,
                compose(8, false, images)?,
                compose(8, true, images)?,
            ])
        })();
        match loaded {
            Ok(frames) => rendered_art.battle_send_out_poof_cache = Some(frames),
            Err(error) => rendered_art.battle_send_out_poof_error = Some(error.to_string()),
        }
    }
    rendered_art.battle_send_out_poof_cache.as_ref().with_context(|| {
        rendered_art
            .battle_send_out_poof_error
            .clone()
            .unwrap_or_else(|| "battle send-out poof art is unavailable".to_string())
    })
}

fn spawn_battle_pack_screen(
    commands: &mut Commands,
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) -> Result<()> {
    let (item_ids, cursor, surface_id, pocket_label, show_quantity) =
        if runtime_shell.ball_cursor.is_some() {
            (carried_ball_item_ids(snapshot), &runtime_shell.ball_cursor, "bag:balls", "BALL", true)
        } else if runtime_shell.key_item_cursor.is_some() {
            (
                snapshot.bag.key_items.iter().filter(|item| item.quantity > 0)
                    .map(|item| item.item_id.clone()).collect(),
                &runtime_shell.key_item_cursor,
                "bag:key-items",
                "KEY",
                false,
            )
        } else if runtime_shell.tmhm_cursor.is_some() {
            (
                snapshot.bag.tm_hm.iter().filter(|item| item.quantity > 0)
                    .map(|item| item.item_id.clone()).collect(),
                &runtime_shell.tmhm_cursor,
                "bag:tmhm",
                "TM/HM",
                true,
            )
        } else {
            (carried_battle_non_ball_item_ids(snapshot), &runtime_shell.bag_cursor, "battle:bag-items", "ITEMS", true)
        };
    let row_count = field_pack_selectable_count(item_ids.len());
    let selected = strict_readonly_cursor_index(cursor, surface_id, row_count)
        .with_context(|| format!("battle pack surface {surface_id} has no valid cursor"))?;
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                // gfx/pack/pack.pal color zero: RGB 31,31,31.
                color: Color::WHITE,
                custom_size: Some(Vec2::new(PLAYFIELD_WIDTH, PLAYFIELD_HEIGHT)),
                ..default()
            },
            transform: Transform::from_xyz(0.0, 0.0, 3.4),
            ..default()
        },
        BattleCommandMarker,
    ));
    for (row, text) in [
        (1.0, "< PACK >"),
        (4.0, "[BAG]"),
        (7.0, "<     >"),
        (8.0, pocket_label),
    ] {
        let (x, y) = battle_hud_tile_origin(0.0, row);
        spawn_battle_command_bitmap_text(
            commands, rendered_art, asset_root, images, text, x, y, 3.8,
        );
    }
    let list_start = visible_window_start(selected, row_count, 7);
    for visible_index in 0..7 {
        let index = list_start + visible_index;
        if index >= row_count {
            break;
        }
        let row = 2.0 + visible_index as f32;
        let (x, y) = battle_hud_tile_origin(7.0, row);
        if index >= item_ids.len() {
            spawn_battle_command_bitmap_text(
                commands, rendered_art, asset_root, images,
                &format!("{}CANCEL", if index == selected { ">" } else { " " }),
                x, y, 3.8,
            );
            continue;
        }
        let item_id = &item_ids[index];
        let item = snapshot
            .items
            .iter()
            .find(|item| item.item_id == *item_id)
            .with_context(|| format!("battle PACK item {item_id} is missing"))?;
        spawn_battle_command_bitmap_text(
            commands, rendered_art, asset_root, images,
            &format!(
                "{}{}",
                if index == selected { ">" } else { " " },
                compact_scene_label(&item.name.replace('_', " "), 8)
            ),
            x, y, 3.8,
        );
        if show_quantity {
            let quantity = carried_item_quantity(snapshot, item_id)
                .or_else(|| {
                    snapshot
                        .bag
                        .tm_hm
                        .iter()
                        .find(|item| item.item_id == *item_id)
                        .map(|item| item.quantity)
                })
                .with_context(|| format!("battle pack item {item_id} has no carried quantity"))?;
            let (x, y) = battle_hud_tile_origin(16.0, row);
            spawn_battle_command_bitmap_text(
                commands, rendered_art, asset_root, images,
                &format!("×{:02}", quantity.min(99)), x, y, 3.8,
            );
        }
    }
    let description = if selected >= item_ids.len() {
        "Close the PACK."
    } else {
        item_ids
            .get(selected)
            .and_then(|item_id| snapshot.items.iter().find(|item| item.item_id == *item_id))
            .map(|item| item.description.as_str())
            .with_context(|| {
                format!("battle pack selection {selected} has no item description")
            })?
    };
    for (index, line) in wrap_boot_text_for_box(description, 18, 4).iter().enumerate() {
        let (x, y) = battle_hud_tile_origin(1.0, 13.0 + index as f32);
        spawn_battle_command_bitmap_text(
            commands, rendered_art, asset_root, images, line, x, y, 3.8,
        );
    }
    if let Some(action_cursor) = &runtime_shell.field_pack_action_cursor {
        let pocket = active_visible_field_pack_pocket(runtime_shell);
        let actions = visible_selected_pack_item_actions(
            snapshot,
            runtime_shell,
            &pocket,
            true,
        )?;
        let action_selected = strict_readonly_cursor_index(
            &Some(action_cursor.clone()), "pack:actions", actions.len(),
        )
        .context("battle pack action menu has no valid cursor")?;
        let top = match actions.len() { 4 => 3.0, 3 => 5.0, 2 => 7.0, _ => 9.0 };
        spawn_battle_window(
            commands,
            rendered_art,
            asset_root,
            images,
            13.0,
            top,
            7.0,
            actions.len() as f32 * 2.0 + 1.0,
            4.1,
        );
        for (index, action) in actions.iter().enumerate() {
            let (x, y) = battle_hud_tile_origin(14.0, top + 1.0 + index as f32 * 2.0);
            spawn_battle_command_bitmap_text(
                commands, rendered_art, asset_root, images,
                &format!("{}{}", if index == action_selected { ">" } else { " " }, visible_field_pack_action_label(*action)),
                x, y, 4.3,
            );
        }
    }
    Ok(())
}

fn spawn_battle_party_menu(
    commands: &mut Commands,
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) -> Result<()> {
    let item_target = runtime_shell.battle_pack_target_mode.is_some();
    let option_count = if item_target {
        snapshot.party.slots.len() + 1
    } else {
        battle_switch_option_count(snapshot)
    };
    let selected = if item_target {
        anyhow::ensure!(
            runtime_shell.party_cursor < option_count,
            "battle item-target party cursor {} is outside {option_count} rows",
            runtime_shell.party_cursor
        );
        runtime_shell.party_cursor
    } else {
        strict_readonly_cursor_index(
            &runtime_shell.battle_switch_cursor,
            "battle:switch",
            option_count,
        )
        .context("battle party menu requires a valid cursor")?
    };
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                // gfx/stats/party_menu_bg.pal color 0: RGB 31,31,31.
                color: Color::rgb(1.0, 1.0, 1.0),
                custom_size: Some(Vec2::new(PLAYFIELD_WIDTH, PLAYFIELD_HEIGHT)),
                ..default()
            },
            transform: Transform::from_xyz(0.0, 0.0, 3.4),
            ..default()
        },
        BattleCommandMarker,
    ));
    for (row_index, slot) in snapshot.party.slots.iter().enumerate().take(6) {
        let name_row = 1.0 + row_index as f32 * 2.0;
        let status_row = name_row + 1.0;
        let marker = if selected == row_index { ">" } else { " " };
        spawn_battle_party_icon(
            commands,
            snapshot,
            slot,
            row_index,
            selected == row_index,
            false,
            rendered_art,
            asset_root,
            images,
        )?;
        let (x, y) = battle_hud_tile_origin(0.0, name_row);
        spawn_battle_command_bitmap_text(
            commands,
            rendered_art,
            asset_root,
            images,
            marker,
            x,
            y,
            3.8,
        );
        let (x, y) = battle_hud_tile_origin(3.0, name_row);
        spawn_battle_command_bitmap_text(
            commands,
            rendered_art,
            asset_root,
            images,
            &compact_scene_label(&slot.pokemon.nickname, 10),
            x,
            y,
            3.8,
        );
        if slot.pokemon.is_egg {
            continue;
        }
        let (x, y) = battle_hud_tile_origin(13.0, name_row);
        spawn_battle_command_bitmap_text(
            commands,
            rendered_art,
            asset_root,
            images,
            &format!("{:>3}/{:>3}", slot.pokemon.hp.min(999), slot.pokemon.max_hp.min(999)),
            x,
            y,
            3.8,
        );
        let (x, y) = battle_hud_tile_origin(5.0, status_row);
        spawn_battle_command_bitmap_text(
            commands,
            rendered_art,
            asset_root,
            images,
            party_status_token(&slot.pokemon),
            x,
            y,
            3.8,
        );
        let (x, y) = battle_hud_tile_origin(8.0, status_row);
        spawn_battle_command_bitmap_text(
            commands,
            rendered_art,
            asset_root,
            images,
            &format!("\u{e10a}{:>2}", slot.pokemon.level.min(100)),
            x,
            y,
            3.8,
        );
        spawn_battle_hud_hp_bar(
            commands,
            rendered_art,
            asset_root,
            images,
            11.0,
            status_row,
            slot.pokemon.hp,
            slot.pokemon.max_hp,
            BattleHpSide::Player,
            None,
        )?;
    }
    let cancel_row = 1.0 + snapshot.party.slots.len().min(6) as f32 * 2.0;
    let cancel_marker = if selected >= snapshot.party.slots.len() { ">" } else { " " };
    let (x, y) = battle_hud_tile_origin(1.0, cancel_row);
    spawn_battle_command_bitmap_text(
        commands, rendered_art, asset_root, images,
        &format!("{cancel_marker}CANCEL"), x, y, 3.8,
    );
    spawn_battle_window(
        commands,
        rendered_art,
        asset_root,
        images,
        0.0,
        14.0,
        20.0,
        4.0,
        3.9,
    );
    let (x, y) = battle_hud_tile_origin(1.0, 15.0);
    spawn_battle_command_bitmap_text(
        commands,
        rendered_art,
        asset_root,
        images,
        if item_target {
            "Use on which <PKMN>?"
        } else if visible_active_battle_player_fainted(snapshot) {
            "Which <PKMN>?"
        } else {
            "Choose a <PKMN>."
        },
        x,
        y,
        4.1,
    );
    if !item_target {
        if runtime_shell.battle_party_action_cursor.is_some() {
            let action_selected = strict_readonly_cursor_index(
                &runtime_shell.battle_party_action_cursor,
                "battle:party-actions",
                3,
            )
            .context("battle party action cursor is invalid")?;
            spawn_battle_window(
                commands,
                rendered_art,
                asset_root,
                images,
                11.0,
                11.0,
                9.0,
                7.0,
                4.2,
            );
            for (index, label) in ["SWITCH", "STATS", "CANCEL"].iter().enumerate() {
                let (x, y) = battle_hud_tile_origin(12.0, 12.0 + index as f32 * 2.0);
                spawn_battle_command_bitmap_text(
                    commands,
                    rendered_art,
                    asset_root,
                    images,
                    &format!("{}{label}", if index == action_selected { ">" } else { " " }),
                    x,
                    y,
                    4.4,
                );
            }
        }
    }
    Ok(())
}

fn spawn_battle_pack_move_target_screen(
    commands: &mut Commands,
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) -> Result<()> {
    let slot = snapshot
        .party
        .slots
        .get(runtime_shell.party_cursor)
        .with_context(|| {
            format!(
                "battle PP-item target party cursor {} is outside {} slots",
                runtime_shell.party_cursor,
                snapshot.party.slots.len()
            )
        })?;
    let selected = strict_readonly_cursor_index(
        &runtime_shell.party_move_cursor,
        &party_move_cursor_surface_id(slot.index),
        slot.pokemon.moves.len(),
    )
    .with_context(|| {
        format!(
            "battle PP-item target move cursor is invalid for party slot {} with {} moves",
            slot.index,
            slot.pokemon.moves.len()
        )
    })?;
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::rgb(1.0, 1.0, 1.0),
                custom_size: Some(Vec2::new(PLAYFIELD_WIDTH, PLAYFIELD_HEIGHT)),
                ..default()
            },
            transform: Transform::from_xyz(0.0, 0.0, 3.4),
            ..default()
        },
        BattleCommandMarker,
    ));
    let (x, y) = battle_hud_tile_origin(1.0, 1.0);
    spawn_battle_command_bitmap_text(
        commands, rendered_art, asset_root, images,
        &format!("{} \u{e10a}{:>2}", compact_scene_label(&slot.pokemon.nickname, 10), slot.pokemon.level),
        x, y, 3.8,
    );
    for (index, learned) in slot.pokemon.moves.iter().enumerate().take(4) {
        let row = 3.0 + index as f32 * 2.0;
        let (x, y) = battle_hud_tile_origin(1.0, row);
        spawn_battle_command_bitmap_text(
            commands, rendered_art, asset_root, images,
            &format!("{}{}", if index == selected { ">" } else { " " }, battle_move_display_name(snapshot, &learned.name)),
            x, y, 3.8,
        );
        let (x, y) = battle_hud_tile_origin(10.0, row + 1.0);
        spawn_battle_command_bitmap_text(
            commands, rendered_art, asset_root, images,
            &visible_move_pp_text(snapshot, learned),
            x,
            y,
            3.8,
        );
    }
    let item_ids = carried_battle_usable_item_ids(snapshot);
    let item_index = strict_readonly_cursor_index(
        &runtime_shell.bag_cursor,
        "battle:bag-items",
        item_ids.len(),
    )
    .with_context(|| {
        format!(
            "battle PP-item target bag cursor is invalid for {} usable items",
            item_ids.len()
        )
    })?;
    let item_id = item_ids
        .get(item_index)
        .context("battle PP-item target selection is missing from the usable item list")?;
    let item = snapshot
        .items
        .iter()
        .find(|item| item.item_id == *item_id)
        .with_context(|| format!("battle PP-item target item {item_id} is missing"))?;
    let raises_pp = item.pp_up_stages.is_some();
    let prompt = if raises_pp {
        ["Raise the PP of", "which move?"]
    } else {
        ["Restore the PP of", "which move?"]
    };
    for (line_index, line) in prompt.iter().enumerate() {
        let (x, y) = battle_hud_tile_origin(1.0, 13.0 + line_index as f32);
        spawn_battle_command_bitmap_text(
            commands, rendered_art, asset_root, images, line, x, y, 3.8,
        );
    }
    Ok(())
}

fn spawn_battle_party_icon(
    commands: &mut Commands,
    snapshot: &RuntimeShellSnapshot,
    slot: &crate::RuntimePartySlotSnapshot,
    row_index: usize,
    selected: bool,
    field_surface: bool,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) -> Result<()> {
    let species_id = if slot.pokemon.is_egg {
        "EGG"
    } else {
        slot.pokemon.species.id.as_str()
    };
    let icon_id = snapshot
        .presentation
        .menu_icons
        .get(species_id)
        .with_context(|| format!("party icon mapping missing species {species_id}"))?
        .clone();
    if !rendered_art.party_icon_cache.contains_key(&icon_id)
        && !rendered_art.party_icon_errors.contains_key(&icon_id)
    {
        match load_party_icon_frame(asset_root, &icon_id, images) {
            Ok(frames) => {
                rendered_art.party_icon_cache.insert(icon_id.clone(), frames);
            }
            Err(error) => {
                rendered_art
                    .party_icon_errors
                    .insert(icon_id.clone(), error.to_string());
            }
        }
    }
    let frames = rendered_art
        .party_icon_cache
        .get(&icon_id)
        .with_context(|| {
            rendered_art
                .party_icon_errors
                .get(&icon_id)
                .cloned()
                .unwrap_or_else(|| format!("party icon {icon_id} is unavailable"))
        })?;
    let max_hp = slot.pokemon.max_hp.max(1);
    let hp = slot.pokemon.hp.min(max_hp);
    let (duration, bob_amplitude) = if u32::from(hp) * 2 >= u32::from(max_hp) {
        (8_u64, 2.0_f32)
    } else if u32::from(hp) * 100 >= u32::from(max_hp) * 21 {
        (72_u64, 1.0_f32)
    } else {
        (136_u64, 0.0_f32)
    };
    let frame_counter = snapshot.state_checksum.frame();
    let frame = &frames[((frame_counter / duration) & 1) as usize];
    let bob_pixels = if selected
        && bob_amplitude > 0.0
        && (frame_counter / 16) & 1 != 0
    {
        bob_amplitude
    } else {
        0.0
    };
    // ASM OAM starts the first 16x16 icon at LCD pixel (8,4), then advances
    // exactly 16 pixels per party slot.
    let (x, y) = battle_hud_tile_origin(1.5, 1.0 + row_index as f32 * 2.0);
    let mut icon = commands.spawn(SpriteBundle {
            texture: frame.handle.clone(),
            sprite: Sprite { custom_size: Some(frame.size), ..default() },
            transform: Transform::from_xyz(
                x,
                y + bob_pixels * (TILE_SIZE / SOURCE_TILE_SIZE as f32),
                3.85,
            ),
            ..default()
        });
    if field_surface {
        icon.insert(FieldCommandMarker);
    } else {
        icon.insert(BattleCommandMarker);
    }
    if let Some(item_id) = slot.pokemon.item.as_deref() {
        let overlay_id = if item_id.ends_with("_MAIL") { "mail" } else { "item" };
        spawn_battle_party_icon_overlay(
            commands,
            overlay_id,
            row_index,
            bob_pixels,
            field_surface,
            rendered_art,
            asset_root,
            images,
        )?;
    }
    Ok(())
}

fn spawn_battle_party_icon_overlay(
    commands: &mut Commands,
    overlay_id: &str,
    row_index: usize,
    bob_pixels: f32,
    field_surface: bool,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) -> Result<()> {
    if !rendered_art.party_icon_overlay_cache.contains_key(overlay_id)
        && !rendered_art.party_icon_overlay_errors.contains_key(overlay_id)
    {
        match load_party_icon_overlay(asset_root, overlay_id, images) {
            Ok(frame) => {
                rendered_art
                    .party_icon_overlay_cache
                    .insert(overlay_id.to_string(), frame);
            }
            Err(error) => {
                rendered_art
                    .party_icon_overlay_errors
                    .insert(overlay_id.to_string(), error.to_string());
            }
        }
    }
    let frame = rendered_art
        .party_icon_overlay_cache
        .get(overlay_id)
        .with_context(|| {
            rendered_art
                .party_icon_overlay_errors
                .get(overlay_id)
                .cloned()
                .unwrap_or_else(|| format!("party icon overlay {overlay_id} is unavailable"))
        })?;
    let (x, y) = battle_hud_tile_origin(1.0, 1.5 + row_index as f32 * 2.0);
    let mut overlay = commands.spawn(SpriteBundle {
            texture: frame.handle.clone(),
            sprite: Sprite { custom_size: Some(frame.size), ..default() },
            transform: Transform::from_xyz(
                x,
                y + bob_pixels * (TILE_SIZE / SOURCE_TILE_SIZE as f32),
                3.9,
            ),
            ..default()
        });
    if field_surface {
        overlay.insert(FieldCommandMarker);
    } else {
        overlay.insert(BattleCommandMarker);
    }
    Ok(())
}

fn load_party_icon_overlay(
    asset_root: &AssetRoot,
    overlay_id: &str,
    images: &mut Assets<Image>,
) -> Result<SpriteFrame> {
    let assets = asset_root.runtime_assets();
    let palette_path = assets.join("gfx/stats/party_menu_ob.pal");
    let palette_content = std::fs::read_to_string(&palette_path)
        .with_context(|| format!("read party icon palette {}", palette_path.display()))?;
    let palette = parse_palette_file(&palette_content, None)?
        .into_iter()
        .next()
        .context("party icon palette must define one four-color palette")?;
    let path = assets
        .join("gfx/stats")
        .join(format!("{overlay_id}.2bpp"));
    let data = std::fs::read(&path)
        .with_context(|| format!("read party icon overlay {}", path.display()))?;
    if data.len() != 16 {
        anyhow::bail!(
            "party icon overlay {} must contain exactly one 2bpp tile, found {} bytes",
            path.display(),
            data.len()
        );
    }
    let mut rgba = vec![0_u8; 8 * 8 * 4];
    for row in 0..8 {
        let lo = data[row * 2];
        let hi = data[row * 2 + 1];
        for col in 0..8 {
            let bit = 1 << (7 - col);
            let palette_index = (((hi & bit != 0) as usize) << 1)
                | (lo & bit != 0) as usize;
            let offset = (row * 8 + col) * 4;
            rgba[offset..offset + 3].copy_from_slice(&palette[palette_index]);
            rgba[offset + 3] = if palette_index == 0 { 0 } else { 255 };
        }
    }
    let mut image = Image::new(
        Extent3d { width: 8, height: 8, depth_or_array_layers: 1 },
        TextureDimension::D2,
        rgba,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.sampler = ImageSampler::nearest();
    Ok(SpriteFrame { handle: images.add(image), size: Vec2::splat(TILE_SIZE) })
}

fn load_party_icon_frame(
    asset_root: &AssetRoot,
    icon_id: &str,
    images: &mut Assets<Image>,
) -> Result<[SpriteFrame; 2]> {
    let assets = asset_root.runtime_assets();
    let palette_path = assets.join("gfx/stats/party_menu_ob.pal");
    let palette_content = std::fs::read_to_string(&palette_path)
        .with_context(|| format!("read party icon palette {}", palette_path.display()))?;
    let palette = parse_palette_file(&palette_content, None)?
        .into_iter()
        .next()
        .context("party icon palette must define one four-color palette")?;
    let stem = icon_id
        .strip_prefix("ICON_")
        .unwrap_or(icon_id)
        .to_ascii_lowercase();
    let icon_path = assets.join("gfx/icons").join(format!("{stem}.2bpp"));
    let data = std::fs::read(&icon_path)
        .with_context(|| format!("read party icon {}", icon_path.display()))?;
    if data.len() < 8 * 16 || data.len() % 16 != 0 {
        anyhow::bail!(
            "party icon {} must contain two 16x16 frames (at least 128 aligned bytes), found {}",
            icon_path.display(),
            data.len()
        );
    }
    let mut decoded = Vec::with_capacity(2);
    for frame_index in 0..2 {
        let mut rgba = vec![0_u8; 16 * 16 * 4];
        for tile_row in 0..2 {
            for tile_col in 0..2 {
                let tile_index = frame_index * 4 + tile_row * 2 + tile_col;
                let tile = &data[tile_index * 16..tile_index * 16 + 16];
                for row in 0..8 {
                    let lo = tile[row * 2];
                    let hi = tile[row * 2 + 1];
                    for col in 0..8 {
                        let bit = 1 << (7 - col);
                        let palette_index = (((hi & bit != 0) as usize) << 1)
                            | (lo & bit != 0) as usize;
                        let target_x = tile_col * 8 + col;
                        let target_y = tile_row * 8 + row;
                        let offset = (target_y * 16 + target_x) * 4;
                        rgba[offset..offset + 3].copy_from_slice(&palette[palette_index]);
                        rgba[offset + 3] = if palette_index == 0 { 0 } else { 255 };
                    }
                }
            }
        }
        let mut image = Image::new(
            Extent3d {
                width: 16,
                height: 16,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            rgba,
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        );
        image.sampler = ImageSampler::nearest();
        decoded.push(SpriteFrame {
            handle: images.add(image),
            size: Vec2::splat(TILE_SIZE * 2.0),
        });
    }
    decoded
        .try_into()
        .map_err(|_| anyhow::anyhow!("party icon decoder did not produce two frames"))
}

fn spawn_battle_yes_no_prompt(
    commands: &mut Commands,
    runtime_shell: &BevyRuntimeShell,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
    entries: &[String],
) {
    spawn_battle_window(
        commands,
        rendered_art,
        asset_root,
        images,
        BATTLE_TEXT_BOX_LEFT_TILE,
        BATTLE_TEXT_BOX_TOP_TILE,
        BATTLE_TEXT_BOX_WIDTH_TILES,
        BATTLE_TEXT_BOX_HEIGHT_TILES,
        3.5,
    );
    spawn_battle_window(
        commands,
        rendered_art,
        asset_root,
        images,
        FIELD_YES_NO_LEFT_TILE,
        FIELD_YES_NO_TOP_TILE,
        FIELD_YES_NO_WIDTH_TILES,
        FIELD_YES_NO_HEIGHT_TILES,
        3.9,
    );
    let prompt_count = entries.len().saturating_sub(2);
    let mut prompt_lines = Vec::new();
    for entry in entries.iter().take(prompt_count) {
        prompt_lines.extend(wrap_boot_text_for_box(entry, 18, 4));
    }
    for (line_index, line) in prompt_lines.into_iter().take(4).enumerate() {
        let (x, y) = battle_hud_tile_origin(1.0, 13.0 + line_index as f32);
        spawn_battle_command_bitmap_text(
            commands,
            rendered_art,
            asset_root,
            images,
            &line,
            x,
            y,
            3.8,
        );
    }
    for (index, entry) in entries.iter().skip(prompt_count).enumerate() {
        let (x, y) = battle_hud_tile_origin(
            // `entry` already contains the cursor/blank prefix. Crystal's
            // menu header places that prefix on the window's left tile and
            // the YES/NO label one tile into the window.
            FIELD_YES_NO_LEFT_TILE,
            FIELD_YES_NO_TOP_TILE + 1.0 + index as f32,
        );
        spawn_battle_command_bitmap_text(
            commands,
            rendered_art,
            asset_root,
            images,
            &animated_battle_cursor_entry(runtime_shell, entry),
            x,
            y,
            4.1,
        );
    }
}

fn battle_submenu_entry_tile(index: usize, two_columns: bool) -> (f32, f32) {
    if two_columns {
        let row = index / 2;
        let col = index % 2;
        (
            BATTLE_SUBMENU_ORIGIN_TILE_X + col as f32 * BATTLE_SUBMENU_COLUMN_SPACING_TILES,
            BATTLE_SUBMENU_ORIGIN_TILE_Y + row as f32 * BATTLE_SUBMENU_ROW_SPACING_TILES,
        )
    } else {
        (
            BATTLE_SUBMENU_ORIGIN_TILE_X,
            BATTLE_SUBMENU_ORIGIN_TILE_Y + index as f32 * BATTLE_SUBMENU_ROW_SPACING_TILES,
        )
    }
}

fn spawn_battle_main_command_menu(
    commands: &mut Commands,
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    battle: &crate::RuntimeBattleSnapshot,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
    entries: &[String],
) -> Result<()> {
    let contest_menu = entries.get(2).is_some_and(|entry| {
        entry
            .trim_start_matches(|ch| ch == '>' || ch == ' ')
            .starts_with("PARKBALL×")
    });
    let (left, width) = if contest_menu {
        (2.0, 18.0)
    } else {
        (BATTLE_MAIN_MENU_LEFT_TILE, BATTLE_MAIN_MENU_WIDTH_TILES)
    };
    spawn_battle_window(
        commands,
        rendered_art,
        asset_root,
        images,
        BATTLE_TEXT_BOX_LEFT_TILE,
        BATTLE_TEXT_BOX_TOP_TILE,
        BATTLE_TEXT_BOX_WIDTH_TILES,
        BATTLE_TEXT_BOX_HEIGHT_TILES,
        3.45,
    );
    spawn_battle_window(
        commands,
        rendered_art,
        asset_root,
        images,
        left,
        BATTLE_MAIN_MENU_TOP_TILE,
        width,
        BATTLE_MAIN_MENU_HEIGHT_TILES,
        3.5,
    );
    if !contest_menu {
        let prompt_name = if battle.battle_type == "BATTLETYPE_TUTORIAL" {
            "DUDE".to_string()
        } else {
            let active = battle
                .active_player_party_index
                .context("battle command menu requires an active party slot")?;
            snapshot
                .party
                .slots
                .iter()
                .find(|slot| slot.index == active)
                .with_context(|| {
                    format!("battle command menu active party slot {active} is missing")
                })?
                .pokemon
                .nickname
                .clone()
        };
        for (line_index, line) in wrap_boot_text_for_box(
            &format!("What will {prompt_name} do?"),
            7,
            4,
        )
        .iter()
        .enumerate()
        {
            let (x, y) = battle_hud_tile_origin(1.0, 13.0 + line_index as f32);
            spawn_battle_command_bitmap_text(
                commands,
                rendered_art,
                asset_root,
                images,
                line,
                x,
                y,
                3.8,
            );
        }
    }
    for (index, entry) in entries.iter().enumerate() {
        let (tile_x, tile_y) = if contest_menu {
            let row = index / 2;
            let col = index % 2;
            (3.0 + col as f32 * 12.0, 13.0 + row as f32 * 2.0)
        } else {
            battle_main_menu_entry_tile(index)
        };
        let (x, y) = battle_hud_tile_origin(tile_x, tile_y);
        spawn_battle_command_bitmap_text(
            commands,
            rendered_art,
            asset_root,
            images,
            &compact_scene_label(
                &animated_battle_cursor_entry(runtime_shell, entry),
                if contest_menu { 12 } else { 7 },
            ),
            x,
            y,
            3.8,
        );
    }
    Ok(())
}

fn battle_command_entries_are_main_menu(entries: &[String]) -> bool {
    if entries.len() != BATTLE_MAIN_MENU_LABELS.len() {
        return false;
    }
    if entries.get(2).is_some_and(|entry| {
        entry
            .trim_start_matches(|ch| ch == '>' || ch == ' ')
            .starts_with("PARKBALL×")
    }) {
        return entries
            .iter()
            .enumerate()
            .all(|(index, entry)| match index {
                0 => entry.trim_start_matches(|ch| ch == '>' || ch == ' ').trim() == "FIGHT",
                1 => entry.trim_start_matches(|ch| ch == '>' || ch == ' ').trim() == "<PKMN>",
                2 => true,
                3 => entry.trim_start_matches(|ch| ch == '>' || ch == ' ').trim() == "RUN",
                _ => false,
            });
    }
    entries
        .iter()
        .zip(BATTLE_MAIN_MENU_LABELS)
        .all(|(entry, label)| {
            let trimmed = entry.trim_start_matches(|ch| ch == '>' || ch == ' ').trim();
            trimmed == label
        })
}

#[cfg(test)]
fn battle_main_menu_panel_center() -> (f32, f32) {
    (
        PLAYFIELD_LEFT
            + (BATTLE_MAIN_MENU_LEFT_TILE + BATTLE_MAIN_MENU_WIDTH_TILES * 0.5) * TILE_SIZE,
        PLAYFIELD_TOP
            - (BATTLE_MAIN_MENU_TOP_TILE + BATTLE_MAIN_MENU_HEIGHT_TILES * 0.5) * TILE_SIZE,
    )
}

fn battle_main_menu_entry_tile(index: usize) -> (f32, f32) {
    let row = index / 2;
    let col = index % 2;
    (
        // The menu rectangle begins at x=8. Its cursor occupies the first
        // inner tile at x=9, and the prefixed label therefore starts at x=10.
        BATTLE_MAIN_MENU_ORIGIN_TILE_X
            + col as f32 * BATTLE_MAIN_MENU_COLUMN_SPACING_TILES,
        BATTLE_MAIN_MENU_ORIGIN_TILE_Y + row as f32 * BATTLE_MAIN_MENU_ROW_SPACING_TILES,
    )
}

fn spawn_battle_move_menu(
    commands: &mut Commands,
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    battle: &crate::RuntimeBattleSnapshot,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) -> Result<()> {
    let (_slot, cursor_index, visible_rows, start) =
        battle_move_menu_state(snapshot, runtime_shell, battle)?;
    spawn_battle_window(
        commands,
        rendered_art,
        asset_root,
        images,
        BATTLE_MOVE_SELECTION_LEFT_TILE,
        BATTLE_MOVE_SELECTION_TOP_TILE,
        BATTLE_MOVE_SELECTION_WIDTH_TILES,
        BATTLE_MOVE_SELECTION_HEIGHT_TILES,
        3.5,
    );
    let total = battle.player_moves.len() + 1;
    for visible_index in 0..visible_rows {
        let index = start + visible_index;
        if index >= total {
            break;
        }
        let marker = if runtime_shell.battle_move_swap_origin == Some(index) {
            "▷"
        } else if index == cursor_index {
            battle_cursor_glyph(runtime_shell)
        } else {
            " "
        };
        let label = if index < battle.player_moves.len() {
            battle_move_display_name(snapshot, &battle.player_moves[index].name)
        } else {
            "CANCEL".to_string()
        };
        let (tile_x, tile_y) = battle_move_menu_entry_tile(visible_index);
        let (x, y) = battle_hud_tile_origin(tile_x, tile_y);
        spawn_battle_command_bitmap_text(
            commands,
            rendered_art,
            asset_root,
            images,
            &compact_scene_label(&format!("{marker}{label}"), 12),
            x,
            y,
            3.8,
        );
    }
    spawn_battle_move_info_window(
        commands,
        snapshot,
        battle,
        &battle.player_moves,
        cursor_index,
        rendered_art,
        asset_root,
        images,
    )?;
    Ok(())
}

fn spawn_battle_window(
    commands: &mut Commands,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
    tile_x: f32,
    tile_y: f32,
    width_tiles: f32,
    height_tiles: f32,
    z: f32,
) {
    let (center_x, center_y) = battle_window_center(tile_x, tile_y, width_tiles, height_tiles);
    if width_tiles > 2.0 && height_tiles > 2.0 {
        commands.spawn((
            SpriteBundle {
                sprite: Sprite {
                    color: Color::WHITE,
                    custom_size: Some(Vec2::new(
                        TILE_SIZE * (width_tiles - 2.0),
                        TILE_SIZE * (height_tiles - 2.0),
                    )),
                    ..default()
                },
                transform: Transform::from_xyz(center_x, center_y, z),
                ..default()
            },
            BattleCommandMarker,
        ));
    }
    if let Some(frame) = battle_window_frame_art(rendered_art, asset_root, images) {
        let width = width_tiles.round().max(0.0) as usize;
        let height = height_tiles.round().max(0.0) as usize;
        if width >= 2 && height >= 2 {
            spawn_battle_window_frame_tiles(
                commands,
                frame,
                tile_x,
                tile_y,
                width,
                height,
                z + 0.05,
            );
            return;
        }
    }
}

fn spawn_battle_move_info_window(
    commands: &mut Commands,
    snapshot: &RuntimeShellSnapshot,
    battle: &crate::RuntimeBattleSnapshot,
    moves: &[crate::core::models::LearnedMove],
    cursor_index: usize,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) -> Result<()> {
    spawn_battle_window(
        commands,
        rendered_art,
        asset_root,
        images,
        BATTLE_MOVE_INFO_LEFT_TILE,
        BATTLE_MOVE_INFO_TOP_TILE,
        BATTLE_MOVE_INFO_WIDTH_TILES,
        BATTLE_MOVE_INFO_HEIGHT_TILES,
        3.5,
    );
    let selected = moves.get(cursor_index);
    if let Some(selected) = selected {
        if battle.player_disabled_move.as_deref() == Some(selected.name.as_str()) {
            spawn_battle_move_info_text(
                commands,
                rendered_art,
                asset_root,
                images,
                "Disabled!",
                1.0,
                10.0,
            );
            return Ok(());
        }
        let move_data = snapshot
            .moves
            .iter()
            .find(|move_data| move_data.move_id == selected.name)
            .with_context(|| {
                format!(
                    "battle move info is missing move metadata for {}",
                    selected.name
                )
            })?;
        let move_type = move_data.move_type.as_str();
        let max_pp = crate::core::models::max_move_pp(move_data.pp, selected.pp_ups);
        spawn_battle_move_info_text(
            commands,
            rendered_art,
            asset_root,
            images,
            "TYPE/",
            1.0,
            9.0,
        );
        spawn_battle_move_info_text(
            commands,
            rendered_art,
            asset_root,
            images,
            &compact_scene_label(&battle_type_display_name(move_type), 10),
            2.0,
            10.0,
        );
        spawn_battle_move_info_text(
            commands,
            rendered_art,
            asset_root,
            images,
            "PP",
            1.0,
            11.0,
        );
        spawn_battle_move_info_text(
            commands,
            rendered_art,
            asset_root,
            images,
            &format!(
                "{:>2}/{:>2}",
                selected.current_pp.min(99),
                max_pp.min(99)
            ),
            5.0,
            11.0,
        );
    } else {
        spawn_battle_move_info_text(
            commands,
            rendered_art,
            asset_root,
            images,
            "TYPE/",
            1.0,
            9.0,
        );
        spawn_battle_move_info_text(
            commands,
            rendered_art,
            asset_root,
            images,
            "----",
            2.0,
            10.0,
        );
        spawn_battle_move_info_text(
            commands,
            rendered_art,
            asset_root,
            images,
            "PP --/--",
            1.0,
            11.0,
        );
    }
    Ok(())
}

fn spawn_battle_move_info_text(
    commands: &mut Commands,
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
    text: &str,
    tile_x: f32,
    tile_y: f32,
) {
    let (x, y) = battle_hud_tile_origin(tile_x, tile_y);
    spawn_battle_command_bitmap_text(commands, rendered_art, asset_root, images, text, x, y, 3.8);
}

fn battle_window_frame_art<'a>(
    rendered_art: &'a mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) -> Option<&'a WindowFrameArt> {
    let frame_id = rendered_art.selected_window_frame_id.clamp(1, 8);
    window_frame_art(rendered_art, asset_root, images, frame_id)
}

fn window_frame_art<'a>(
    rendered_art: &'a mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
    frame_id: u8,
) -> Option<&'a WindowFrameArt> {
    if !rendered_art.window_frame_cache.contains_key(&frame_id)
        && !rendered_art.window_frame_errors.contains_key(&frame_id)
    {
        match load_window_frame_art(asset_root, frame_id, images) {
            Ok(frame) => {
                rendered_art.window_frame_cache.insert(frame_id, frame);
            }
            Err(error) => {
                rendered_art
                    .window_frame_errors
                    .insert(frame_id, error.to_string());
            }
        }
    }
    rendered_art.window_frame_cache.get(&frame_id)
}

fn spawn_battle_window_frame_tiles(
    commands: &mut Commands,
    frame: &WindowFrameArt,
    tile_x: f32,
    tile_y: f32,
    width: usize,
    height: usize,
    z: f32,
) {
    let top_y = tile_y;
    let bottom_y = tile_y + height.saturating_sub(1) as f32;
    let left_x = tile_x;
    let right_x = tile_x + width.saturating_sub(1) as f32;
    spawn_battle_window_frame_tile(commands, &frame.top_left, left_x, top_y, z);
    spawn_battle_window_frame_tile(commands, &frame.top_right, right_x, top_y, z);
    spawn_battle_window_frame_tile(commands, &frame.bottom_left, left_x, bottom_y, z);
    spawn_battle_window_frame_tile(commands, &frame.bottom_right, right_x, bottom_y, z);

    for col in 1..width.saturating_sub(1) {
        let x = tile_x + col as f32;
        spawn_battle_window_frame_tile(commands, &frame.top_edge, x, top_y, z);
        spawn_battle_window_frame_tile(commands, &frame.top_edge, x, bottom_y, z);
    }
    for row in 1..height.saturating_sub(1) {
        let y = tile_y + row as f32;
        spawn_battle_window_frame_tile(commands, &frame.side_edge, left_x, y, z);
        spawn_battle_window_frame_tile(commands, &frame.side_edge, right_x, y, z);
    }
}

#[cfg(test)]
fn battle_window_frame_tile_count(width: usize, height: usize) -> usize {
    if width < 2 || height < 2 {
        return 0;
    }
    width * 2 + height.saturating_sub(2) * 2
}

fn spawn_battle_window_frame_tile(
    commands: &mut Commands,
    frame: &SpriteFrame,
    tile_x: f32,
    tile_y: f32,
    z: f32,
) {
    let (x, y) = battle_hud_tile_origin(tile_x, tile_y);
    commands.spawn((
        SpriteBundle {
            texture: frame.handle.clone(),
            sprite: Sprite {
                custom_size: Some(frame.size),
                ..default()
            },
            transform: Transform::from_xyz(x, y, z),
            ..default()
        },
        BattleCommandMarker,
        BattleWindowFrameMarker,
    ));
}

fn spawn_scene_dialog_window_frame_tiles(
    commands: &mut Commands,
    frame: &WindowFrameArt,
    tile_x: f32,
    tile_y: f32,
    width: usize,
    height: usize,
    z: f32,
) {
    let top_y = tile_y;
    let bottom_y = tile_y + height.saturating_sub(1) as f32;
    let left_x = tile_x;
    let right_x = tile_x + width.saturating_sub(1) as f32;
    spawn_scene_dialog_window_frame_tile(commands, &frame.top_left, left_x, top_y, z);
    spawn_scene_dialog_window_frame_tile(commands, &frame.top_right, right_x, top_y, z);
    spawn_scene_dialog_window_frame_tile(commands, &frame.bottom_left, left_x, bottom_y, z);
    spawn_scene_dialog_window_frame_tile(commands, &frame.bottom_right, right_x, bottom_y, z);

    for col in 1..width.saturating_sub(1) {
        let x = tile_x + col as f32;
        spawn_scene_dialog_window_frame_tile(commands, &frame.top_edge, x, top_y, z);
        spawn_scene_dialog_window_frame_tile(commands, &frame.top_edge, x, bottom_y, z);
    }
    for row in 1..height.saturating_sub(1) {
        let y = tile_y + row as f32;
        spawn_scene_dialog_window_frame_tile(commands, &frame.side_edge, left_x, y, z);
        spawn_scene_dialog_window_frame_tile(commands, &frame.side_edge, right_x, y, z);
    }
}

fn spawn_scene_dialog_window_frame_tile(
    commands: &mut Commands,
    frame: &SpriteFrame,
    tile_x: f32,
    tile_y: f32,
    z: f32,
) {
    let (x, y) = battle_hud_tile_origin(tile_x, tile_y);
    commands.spawn((
        SpriteBundle {
            texture: frame.handle.clone(),
            sprite: Sprite {
                custom_size: Some(frame.size),
                ..default()
            },
            transform: Transform::from_xyz(x, y, z),
            ..default()
        },
        SceneDialogMarker,
        SceneDialogWindowFrameMarker,
    ));
}

fn spawn_field_command_window_frame_tiles(
    commands: &mut Commands,
    frame: &WindowFrameArt,
    tile_x: f32,
    tile_y: f32,
    width: usize,
    height: usize,
    z: f32,
) {
    let top_y = tile_y;
    let bottom_y = tile_y + height.saturating_sub(1) as f32;
    let left_x = tile_x;
    let right_x = tile_x + width.saturating_sub(1) as f32;
    spawn_field_command_window_frame_tile(commands, &frame.top_left, left_x, top_y, z);
    spawn_field_command_window_frame_tile(commands, &frame.top_right, right_x, top_y, z);
    spawn_field_command_window_frame_tile(commands, &frame.bottom_left, left_x, bottom_y, z);
    spawn_field_command_window_frame_tile(commands, &frame.bottom_right, right_x, bottom_y, z);

    for col in 1..width.saturating_sub(1) {
        let x = tile_x + col as f32;
        spawn_field_command_window_frame_tile(commands, &frame.top_edge, x, top_y, z);
        spawn_field_command_window_frame_tile(commands, &frame.top_edge, x, bottom_y, z);
    }
    for row in 1..height.saturating_sub(1) {
        let y = tile_y + row as f32;
        spawn_field_command_window_frame_tile(commands, &frame.side_edge, left_x, y, z);
        spawn_field_command_window_frame_tile(commands, &frame.side_edge, right_x, y, z);
    }
}

fn spawn_field_command_window_frame_tile(
    commands: &mut Commands,
    frame: &SpriteFrame,
    tile_x: f32,
    tile_y: f32,
    z: f32,
) {
    let (x, y) = battle_hud_tile_origin(tile_x, tile_y);
    commands.spawn((
        SpriteBundle {
            texture: frame.handle.clone(),
            sprite: Sprite {
                custom_size: Some(frame.size),
                ..default()
            },
            transform: Transform::from_xyz(x, y, z),
            ..default()
        },
        FieldCommandMarker,
        FieldCommandWindowFrameMarker,
    ));
}

fn battle_move_menu_state<'a>(
    snapshot: &'a RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    battle: &crate::RuntimeBattleSnapshot,
) -> Result<(&'a crate::RuntimePartySlotSnapshot, usize, usize, usize)> {
    let active_index = battle
        .active_player_party_index
        .context("battle move menu requires an active player party slot")?;
    let slot = snapshot
        .party
        .slots
        .iter()
        .find(|slot| slot.index == active_index)
        .with_context(|| {
            format!("active player party slot {active_index} is absent from the move menu")
        })?;
    let total = battle.player_moves.len() + 1;
    let cursor_index =
        strict_readonly_cursor_index(&runtime_shell.battle_move_cursor, "battle:moves", total)
            .with_context(|| format!("battle move cursor is invalid for {total} entries"))?;
    let visible_rows = battle_move_visible_rows(total);
    let start = if total > visible_rows {
        cursor_index
            .saturating_sub(visible_rows - 1)
            .min(total - visible_rows)
    } else {
        0
    };
    Ok((slot, cursor_index, visible_rows, start))
}

fn battle_move_menu_option_count(
    snapshot: &RuntimeShellSnapshot,
    battle: &crate::RuntimeBattleSnapshot,
) -> Result<usize> {
    let active_index = battle
        .active_player_party_index
        .context("battle move menu requires an active player party slot")?;
    snapshot
        .party
        .slots
        .iter()
        .find(|slot| slot.index == active_index)
        .with_context(|| {
            format!("active player party slot {active_index} is absent from the move menu")
        })?;
    anyhow::ensure!(
        battle.commands.player_move_slots.len() == battle.player_moves.len(),
        "battle move menu exposes {} moves but command selection exposes {} slots",
        battle.player_moves.len(),
        battle.commands.player_move_slots.len(),
    );
    Ok(battle.player_moves.len() + 1)
}

fn battle_move_menu_option_count_for_slot(slot: &crate::RuntimePartySlotSnapshot) -> usize {
    slot.pokemon.moves.len() + 1
}

fn battle_move_visible_rows(total: usize) -> usize {
    let inner_height = BATTLE_MOVE_SELECTION_HEIGHT_TILES - 2.0;
    let rows = (inner_height / BATTLE_MOVE_MENU_ROW_SPACING_TILES)
        .floor()
        .max(1.0) as usize;
    total.min(rows)
}

fn battle_window_center(
    tile_x: f32,
    tile_y: f32,
    width_tiles: f32,
    height_tiles: f32,
) -> (f32, f32) {
    (
        PLAYFIELD_LEFT + (tile_x + width_tiles * 0.5) * TILE_SIZE,
        PLAYFIELD_TOP - (tile_y + height_tiles * 0.5) * TILE_SIZE,
    )
}

/// Field and battle windows share the native 20x18 LCD coordinate space.
fn field_window_center(
    tile_x: f32,
    tile_y: f32,
    width_tiles: f32,
    height_tiles: f32,
) -> (f32, f32) {
    (
        PLAYFIELD_LEFT + (tile_x + width_tiles * 0.5) * TILE_SIZE,
        PLAYFIELD_TOP - (tile_y + height_tiles * 0.5) * TILE_SIZE,
    )
}

fn battle_move_menu_entry_tile(visible_index: usize) -> (f32, f32) {
    (
        // The rendered string owns the cursor tile; the move name itself
        // still begins at the ASM move-menu origin (6, 13).
        BATTLE_MOVE_MENU_ORIGIN_TILE_X - 1.0,
        BATTLE_MOVE_MENU_ORIGIN_TILE_Y + visible_index as f32 * BATTLE_MOVE_MENU_ROW_SPACING_TILES,
    )
}

fn battle_cursor_glyph(_runtime_shell: &BevyRuntimeShell) -> &'static str {
    "▶"
}

fn animated_battle_cursor_entry(runtime_shell: &BevyRuntimeShell, entry: &str) -> String {
    entry
        .strip_prefix('>')
        .map(|label| format!("{}{label}", battle_cursor_glyph(runtime_shell)))
        .unwrap_or_else(|| entry.to_string())
}

fn battle_move_display_name(snapshot: &RuntimeShellSnapshot, move_id: &str) -> String {
    snapshot
        .moves
        .iter()
        .find(|move_data| move_data.move_id == move_id)
        .map(|move_data| move_data.name.replace('_', " "))
        .unwrap_or_else(|| format!("INVALID MOVE {move_id}"))
}

fn visible_move_pp_text(
    snapshot: &RuntimeShellSnapshot,
    learned: &crate::core::models::LearnedMove,
) -> String {
    snapshot
        .moves
        .iter()
        .find(|entry| entry.move_id == learned.name)
        .map(|entry| {
            let max_pp = crate::core::models::max_move_pp(entry.pp, learned.pp_ups);
            format!("PP {:>2}/{:>2}", learned.current_pp, max_pp)
        })
        .unwrap_or_else(|| "PP INVALID".to_string())
}

fn item_display_name(snapshot: &RuntimeShellSnapshot, item_id: &str) -> String {
    snapshot
        .items
        .iter()
        .find(|item| item.item_id == item_id)
        .map(|item| item.name.replace('_', " "))
        .unwrap_or_else(|| format!("INVALID ITEM {item_id}"))
}

fn battle_type_display_name(move_type: &str) -> String {
    move_type
        .strip_suffix("_TYPE")
        .unwrap_or(move_type)
        .replace('_', " ")
}

fn visible_battle_command_menu_entries(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    battle: &crate::RuntimeBattleSnapshot,
) -> Result<Vec<String>> {
    if runtime_shell.battle_faint_prompt_cursor.is_some() {
        return visible_battle_faint_prompt_entries(runtime_shell);
    }
    if runtime_shell.battle_shift_prompt_cursor.is_some() {
        return visible_battle_shift_prompt_entries(snapshot, runtime_shell, battle);
    }
    if let Some(mode) = runtime_shell.battle_pack_target_mode {
        return Ok(visible_battle_pack_target_entries(snapshot, runtime_shell, mode));
    }
    if runtime_shell.battle_move_cursor.is_some() {
        return Ok(visible_battle_move_entries(snapshot, runtime_shell, battle));
    }
    if runtime_shell.battle_switch_cursor.is_some() {
        return Ok(visible_battle_switch_entries(snapshot, runtime_shell, battle));
    }
    if runtime_shell.bag_cursor.is_some() {
        return Ok(visible_battle_item_entries(snapshot, runtime_shell));
    }
    if runtime_shell.ball_cursor.is_some() {
        return Ok(visible_battle_ball_entries(snapshot, runtime_shell));
    }
    if runtime_shell.key_item_cursor.is_some() || runtime_shell.tmhm_cursor.is_some() {
        return Ok(visible_field_pack_entries(snapshot, runtime_shell));
    }
    if battle.enemy_pokemon.hp == 0 {
        // KO settlement is driven by the retained faint/reward presentation.
        // Crystal never exposes the shell's claim/advance operations as a
        // selectable battle menu between those boundaries.
        return Ok(Vec::new());
    }
    let actions = visible_battle_action_ids(snapshot, battle);
    if actions.is_empty() {
        return Ok(Vec::new());
    }
    let selected = strict_readonly_cursor_index(
        &runtime_shell.battle_action_cursor,
        "battle:actions",
        actions.len(),
    );
    let selected = selected.context("battle main-action cursor is invalid")?;
    let selected_action = actions[selected];
    Ok(battle_main_menu_entries(
        snapshot,
        battle,
        battle_main_menu_index_for_action(selected_action),
    ))
}

fn visible_battle_faint_prompt_entries(runtime_shell: &BevyRuntimeShell) -> Result<Vec<String>> {
    let selected = strict_readonly_cursor_index(
        &runtime_shell.battle_faint_prompt_cursor,
        "battle:faint-prompt",
        2,
    )
    .context("battle faint prompt cursor is invalid")?;
    Ok(vec![
        "Use next <PKMN>?".to_string(),
        format!("{}YES", if selected == 0 { ">" } else { " " }),
        format!("{}NO", if selected == 1 { ">" } else { " " }),
    ])
}

fn battle_main_menu_entries(
    snapshot: &RuntimeShellSnapshot,
    battle: &crate::RuntimeBattleSnapshot,
    selected: usize,
) -> Vec<String> {
    battle_main_menu_entries_for_type(
        &battle.battle_type,
        snapshot.bug_contest.park_balls_remaining,
        selected,
    )
}

fn battle_main_menu_entries_for_type(
    battle_type: &str,
    park_balls_remaining: u8,
    selected: usize,
) -> Vec<String> {
    let labels = if battle_type == "BATTLETYPE_CONTEST" {
        vec![
            "FIGHT".to_string(),
            "<PKMN>".to_string(),
            format!(
                "PARKBALL×{:>2}",
                park_balls_remaining
            ),
            "RUN".to_string(),
        ]
    } else {
        BATTLE_MAIN_MENU_LABELS.iter().map(|label| (*label).to_string()).collect()
    };
    labels
        .iter()
        .enumerate()
        .map(|(index, label)| format!("{}{label}", if selected == index { ">" } else { " " }))
        .collect()
}

fn battle_main_menu_index_for_action(action: VisibleBattleAction) -> usize {
    match action {
        VisibleBattleAction::Fight => 0,
        VisibleBattleAction::Pokemon => 1,
        VisibleBattleAction::Pack => 2,
        VisibleBattleAction::Run => 3,
    }
}

fn visible_battle_switch_entries(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    _battle: &crate::RuntimeBattleSnapshot,
) -> Vec<String> {
    let selected = strict_readonly_cursor_index(
        &runtime_shell.battle_switch_cursor,
        "battle:switch",
        battle_switch_option_count(snapshot),
    );
    let Some(selected) = selected else {
        return vec![compact_scene_label("INVALID CURSOR battle:switch", 30)];
    };
    let mut entries = Vec::new();
    entries.extend(
        windowed_index_range(selected, battle_switch_option_count(snapshot)).map(|index| {
            let Some(slot) = snapshot.party.slots.get(index) else {
                return party_cancel_entry(index == selected);
            };
            party_slot_entry(snapshot, slot, index == selected)
        }),
    );
    entries.into_iter().take(SCENE_MENU_VISIBLE_ROWS).collect()
}

fn visible_battle_shift_prompt_entries(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    battle: &crate::RuntimeBattleSnapshot,
) -> Result<Vec<String>> {
    next_unresolved_trainer_enemy_label(battle)
        .context("trainer shift prompt has no unresolved enemy")?;
    let selected = strict_readonly_cursor_index(
        &runtime_shell.battle_shift_prompt_cursor,
        "battle:shift-prompt",
        2,
    )
    .context("battle trainer-shift cursor is invalid")?;
    Ok(vec![
        compact_scene_label(&format!("Will {}", snapshot.trainer.player_name), 18),
        "change <PKMN>?".to_string(),
        format!("{}YES", if selected == 0 { ">" } else { " " }),
        format!("{}NO", if selected == 1 { ">" } else { " " }),
    ])
}

fn next_unresolved_trainer_enemy_label(battle: &crate::RuntimeBattleSnapshot) -> Option<&str> {
    let enemy_index = battle.active_enemy_party_index?;
    battle
        .enemy_party
        .iter()
        .enumerate()
        .find_map(|(index, pokemon)| {
            (index != enemy_index && pokemon.hp > 0).then_some(pokemon.nickname.as_str())
        })
}

fn visible_battle_item_entries(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
) -> Vec<String> {
    let item_ids = carried_battle_non_ball_item_ids(snapshot);
    let selected = strict_readonly_cursor_index(
        &runtime_shell.bag_cursor,
        "battle:bag-items",
        field_pack_selectable_count(item_ids.len()),
    );
    let Some(selected) = selected else {
        return vec![compact_scene_label("INVALID CURSOR battle:bag-items", 30)];
    };
    let mut entries = Vec::new();
    entries.extend(windowed_index_range(selected, field_pack_selectable_count(item_ids.len())).map(|index| {
        if index >= item_ids.len() {
            return pack_cancel_entry(if index == selected { ">" } else { " " });
        }
        let item_id = &item_ids[index];
        let marker = if index == selected { ">" } else { " " };
        let Some(quantity) = carried_item_quantity(snapshot, item_id) else {
            return compact_scene_label(&format!("{marker}{item_id} INVALID INVENTORY"), 30);
        };
        compact_scene_label(
            &format!("{marker}{} x{quantity}", item_display_name(snapshot, item_id)),
            30,
        )
    }));
    entries.into_iter().take(SCENE_MENU_VISIBLE_ROWS).collect()
}

fn visible_battle_ball_entries(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
) -> Vec<String> {
    let ball_ids = carried_ball_item_ids(snapshot);
    let selected =
        strict_readonly_cursor_index(
            &runtime_shell.ball_cursor,
            "bag:balls",
            field_pack_selectable_count(ball_ids.len()),
        );
    let Some(selected) = selected else {
        return vec![compact_scene_label("INVALID CURSOR bag:balls", 30)];
    };
    let mut entries = Vec::new();
    entries.extend(windowed_index_range(selected, field_pack_selectable_count(ball_ids.len())).map(|index| {
        if index >= ball_ids.len() {
            return pack_cancel_entry(if index == selected { ">" } else { " " });
        }
        let item_id = &ball_ids[index];
        let marker = if index == selected { ">" } else { " " };
        let Some(quantity) = carried_item_quantity(snapshot, item_id) else {
            return compact_scene_label(&format!("{marker}{item_id} INVALID INVENTORY"), 30);
        };
        compact_scene_label(
            &format!("{marker}{} x{quantity}", item_display_name(snapshot, item_id)),
            30,
        )
    }));
    entries.into_iter().take(SCENE_MENU_VISIBLE_ROWS).collect()
}

fn visible_battle_pack_target_entries(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    mode: BattlePackTargetMode,
) -> Vec<String> {
    let selected_party = runtime_shell
        .party_cursor
        .min(snapshot.party.slots.len().saturating_sub(1));
    let mut entries = selected_battle_pack_item_label(snapshot, runtime_shell)
        .map(|_| Vec::new())
        .unwrap_or_else(|| vec![compact_scene_label("INVALID CURSOR battle:item", 30)]);
    if mode == BattlePackTargetMode::PartyMove {
        let Some(slot) = snapshot.party.slots.get(selected_party) else {
            return vec![compact_scene_label(
                &format!("INVALID PARTY SLOT {selected_party}"),
                30,
            )];
        };
        entries.push(party_slot_entry(snapshot, slot, true));
        let selected_move = strict_readonly_cursor_index(
            &runtime_shell.party_move_cursor,
            &party_move_cursor_surface_id(slot.index),
            slot.pokemon.moves.len(),
        );
        let Some(selected_move) = selected_move else {
            entries.push(compact_scene_label(
                &format!("INVALID CURSOR party:{}:moves", slot.index),
                30,
            ));
            return entries.into_iter().take(SCENE_MENU_VISIBLE_ROWS).collect();
        };
        entries.extend(windowed_move_entries(
            snapshot,
            &slot.pokemon.moves,
            selected_move,
        ));
        return entries.into_iter().take(SCENE_MENU_VISIBLE_ROWS).collect();
    }
    entries.extend(windowed_party_slot_entries(snapshot, selected_party));
    entries.into_iter().take(SCENE_MENU_VISIBLE_ROWS).collect()
}

fn visible_battle_move_entries(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    battle: &crate::RuntimeBattleSnapshot,
) -> Vec<String> {
    let Some(active_index) = battle.active_player_party_index else {
        return vec![compact_scene_label(
            "INVALID ACTIVE PARTY SLOT",
            30,
        )];
    };
    if !snapshot.party.slots.iter().any(|slot| slot.index == active_index) {
        return vec![compact_scene_label(
            &format!("INVALID PARTY SLOT {active_index}"),
            30,
        )];
    }
    let total = battle.player_moves.len() + 1;
    let selected =
        strict_readonly_cursor_index(&runtime_shell.battle_move_cursor, "battle:moves", total);
    let Some(selected) = selected else {
        return vec![compact_scene_label("INVALID CURSOR battle:moves", 30)];
    };
    let mut entries = Vec::new();
    entries.extend(windowed_index_range(selected, total).map(|index| {
        let marker = if runtime_shell.battle_move_swap_origin == Some(index) {
            "▷"
        } else if index == selected {
            ">"
        } else {
            " "
        };
        if index >= battle.player_moves.len() {
            format!("{marker}CANCEL")
        } else {
            let learned = &battle.player_moves[index];
            compact_scene_label(
                &format!(
                    "{marker}{}",
                    battle_move_display_name(snapshot, &learned.name)
                ),
                12,
            )
        }
    }));
    entries.into_iter().take(SCENE_MENU_VISIBLE_ROWS).collect()
}

impl TilesetArt {
    #[cfg(test)]
    fn tile_handle(&self, metatile_id: u16, sub_x: usize, sub_y: usize) -> Option<Handle<Image>> {
        self.tile_handle_at_frame(metatile_id, sub_x, sub_y, 0, false)
    }

    fn tile_handle_at_frame(
        &self,
        metatile_id: u16,
        sub_x: usize,
        sub_y: usize,
        frame: u64,
        forest_restless: bool,
    ) -> Option<Handle<Image>> {
        let offset = usize::from(metatile_id)
            .checked_mul(METATILE_TILE_COUNT)?
            .checked_add(sub_y.checked_mul(RENDER_METATILE_WIDTH as usize)?)?
            .checked_add(sub_x)?;
        let tile_index = *self.metatile_layout.get(offset)? as usize;
        if let Some(animation) = self.animated_tiles.get(&tile_index)
            && !animation.frames.is_empty()
        {
            let frame_index = if animation.requires_forest_restless && !forest_restless {
                0
            } else if animation.cave_water_composite {
                let water_frame = (frame / 22) as usize % 4;
                let scroll_frame = if frame < 4 {
                    0
                } else {
                    (((frame - 4) / 19) + 1) as usize % 8
                };
                water_frame * 8 + scroll_frame
            } else if animation.advance_on_phase_offset {
                if frame < animation.phase_offset {
                    0
                } else {
                    (((frame - animation.phase_offset) / animation.frame_ticks.max(1)) + 1)
                        as usize
                        % animation.frames.len()
                }
            } else {
                (frame.saturating_sub(animation.phase_offset)
                    / animation.frame_ticks.max(1)) as usize
                    % animation.frames.len()
            };
            return animation.frames.get(frame_index).cloned();
        }
        self.tile_handles.get(tile_index).cloned()
    }

    fn priority_tile_handle(
        &self,
        metatile_id: u16,
        sub_x: usize,
        sub_y: usize,
    ) -> Option<Handle<Image>> {
        let offset = usize::from(metatile_id)
            .checked_mul(METATILE_TILE_COUNT)?
            .checked_add(sub_y.checked_mul(RENDER_METATILE_WIDTH as usize)?)?
            .checked_add(sub_x)?;
        let tile_index = *self.metatile_layout.get(offset)? as usize;
        self.priority_tile_handles.get(tile_index).cloned()
    }
}

fn load_bitmap_font_art(
    asset_root: &AssetRoot,
    images: &mut Assets<Image>,
) -> Result<BitmapFontArt> {
    let font_path = asset_root.runtime_assets().join("gfx/font/font.png");
    let space_path = asset_root.runtime_assets().join("gfx/font/space.png");
    let source = image::open(&font_path)
        .with_context(|| format!("load bitmap font {}", font_path.display()))?
        .to_rgba8();
    let space = image::open(&space_path)
        .with_context(|| format!("load bitmap font space {}", space_path.display()))?
        .to_rgba8();
    let source_width = source.width() as usize;
    let source_height = source.height() as usize;
    if source_width < BITMAP_FONT_TILE_SIZE || source_height < BITMAP_FONT_TILE_SIZE {
        anyhow::bail!(
            "bitmap font {} has invalid dimensions {}x{}",
            font_path.display(),
            source_width,
            source_height
        );
    }
    let tiles_per_row = source_width / BITMAP_FONT_TILE_SIZE;
    let mut glyphs = HashMap::new();
    for (ch, tile_id) in bitmap_font_char_map() {
        let handle = if ch == ' ' {
            bitmap_font_tile_handle(&space, 0, 1, images)?
        } else {
            // TypeScript keeps both ROM-indexed font tiles (0x80+) and the
            // low-ID tiles copied from the font sheet.  The latter include
            // Poké and several punctuation/control glyphs.
            let tile_index = if tile_id >= 0x80 {
                usize::from(tile_id - 0x80)
            } else {
                usize::from(tile_id)
            };
            bitmap_font_tile_handle(&source, tile_index, tiles_per_row, images)?
        };
        glyphs.insert(
            ch,
            SpriteFrame {
                handle,
                size: Vec2::splat(BITMAP_FONT_GLYPH_SIZE),
            },
        );
    }
    load_bitmap_font_extra_glyphs(
        &asset_root.runtime_assets().join("gfx/font"),
        &mut glyphs,
        images,
    )?;
    load_bitmap_font_frame_glyphs(
        &asset_root.runtime_assets().join("gfx/frames/1.png"),
        &mut glyphs,
        images,
    )?;
    Ok(BitmapFontArt { glyphs })
}

/// `BitmapFont.set_frame_tiles(1)` overwrites these six tile IDs after the
/// font extras load.  Box-drawing characters therefore come from the active
/// textbox frame, never from coincidentally numbered font-sheet tiles.
fn load_bitmap_font_frame_glyphs(
    frame_path: &std::path::Path,
    glyphs: &mut HashMap<char, SpriteFrame>,
    images: &mut Assets<Image>,
) -> Result<()> {
    let frame = image::open(frame_path)
        .with_context(|| format!("load bitmap font frame {}", frame_path.display()))?
        .to_rgba8();
    if frame.width() != 24 || frame.height() != 16 {
        anyhow::bail!(
            "bitmap font frame {} must be 24x16, got {}x{}",
            frame_path.display(),
            frame.width(),
            frame.height()
        );
    }
    for (source_tile, tile_id) in [0x79_u16, 0x7a, 0x7b, 0x7c, 0x7d, 0x7e]
        .into_iter()
        .enumerate()
    {
        let handle = bitmap_font_tile_handle(&frame, source_tile, 3, images)?;
        let sprite = SpriteFrame {
            handle,
            size: Vec2::splat(BITMAP_FONT_GLYPH_SIZE),
        };
        for (ch, mapped_tile) in bitmap_font_char_map() {
            if mapped_tile == tile_id {
                glyphs.insert(ch, sprite.clone());
            }
        }
    }
    Ok(())
}

/// Load the low-ID tiles installed by `LoadFontsExtra` and
/// `LoadFontsBattleExtra`.  They are not present in `font.png`, so using its
/// tile at the same numeric index gives the wrong glyph for control tokens.
fn load_bitmap_font_extra_glyphs(
    font_root: &std::path::Path,
    glyphs: &mut HashMap<char, SpriteFrame>,
    images: &mut Assets<Image>,
) -> Result<()> {
    let install = |data: &[u8],
                   source_tile: usize,
                   tile_id: u16,
                   glyphs: &mut HashMap<char, SpriteFrame>,
                   images: &mut Assets<Image>|
     -> Result<()> {
        let handle = bitmap_font_2bpp_tile_handle(data, source_tile, images)?;
        let frame = SpriteFrame {
            handle,
            size: Vec2::splat(BITMAP_FONT_GLYPH_SIZE),
        };
        for (ch, mapped_tile) in bitmap_font_char_map() {
            if mapped_tile == tile_id {
                glyphs.insert(ch, frame.clone());
            }
        }
        Ok(())
    };

    let battle_extra_path = font_root.join("font_battle_extra.2bpp");
    let battle_extra = std::fs::read(&battle_extra_path)
        .with_context(|| format!("read bitmap battle font extras {}", battle_extra_path.display()))?;
    for source_tile in 0..battle_extra.len() / 16 {
        install(
            &battle_extra,
            source_tile,
            0x60 + source_tile as u16,
            glyphs,
            images,
        )?;
    }
    let up_arrow_path = font_root.join("up_arrow.2bpp");
    let up_arrow = std::fs::read(&up_arrow_path)
        .with_context(|| format!("read bitmap up-arrow glyph {}", up_arrow_path.display()))?;
    install(&up_arrow, 0, 0x61, glyphs, images)?;
    let phone_icon_path = font_root.join("phone_icon.2bpp");
    let phone_icon = std::fs::read(&phone_icon_path)
        .with_context(|| format!("read bitmap phone glyph {}", phone_icon_path.display()))?;
    install(&phone_icon, 0, 0x62, glyphs, images)?;
    let font_extra_path = font_root.join("font_extra.2bpp");
    let font_extra = std::fs::read(&font_extra_path)
        .with_context(|| format!("read bitmap font extras {}", font_extra_path.display()))?;
    // LoadFontsExtra copies source tiles 3..24 into VRAM tiles 0x63..0x78.
    for offset in 0..22 {
        install(&font_extra, offset + 3, 0x63 + offset as u16, glyphs, images)?;
    }
    // LoadFontsExtra installs the extras first, then LoadFontsBattleExtra
    // supplies the battle-specific Lv tile at 0x6e.
    install(
        &battle_extra,
        0x6e - 0x60,
        0x6e,
        glyphs,
        images,
    )?;
    Ok(())
}
