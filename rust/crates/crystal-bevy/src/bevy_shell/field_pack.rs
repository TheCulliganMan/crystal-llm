fn selected_field_pack_item_action_summary(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    pocket: &FieldPackPocket,
) -> Option<String> {
    let item_id = selected_field_pack_item_id_from_snapshot(snapshot, runtime_shell, pocket)?;
    let item = snapshot.items.iter().find(|item| item.item_id == item_id)?;
    Some(field_item_action_summary(
        &runtime_shell.shell,
        item,
        &item_id,
    ))
}

fn selected_field_pack_item_id_from_snapshot(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    pocket: &FieldPackPocket,
) -> Option<String> {
    match pocket {
        FieldPackPocket::Items => strict_readonly_cursor_index(
            &runtime_shell.bag_cursor,
            "bag:items",
            field_pack_selectable_count(carried_item_count(&snapshot.bag.items)),
        )
        .filter(|index| *index < carried_item_count(&snapshot.bag.items))
        .and_then(|index| carried_item_offset(&snapshot.bag.items, index))
        .and_then(|offset| snapshot.bag.items.get(offset))
        .map(|item| item.item_id.clone()),
        FieldPackPocket::Balls => strict_readonly_cursor_index(
            &runtime_shell.ball_cursor,
            "bag:balls",
            field_pack_selectable_count(carried_item_count(&snapshot.bag.balls)),
        )
        .filter(|index| *index < carried_item_count(&snapshot.bag.balls))
        .and_then(|index| carried_item_offset(&snapshot.bag.balls, index))
        .and_then(|offset| snapshot.bag.balls.get(offset))
        .map(|item| item.item_id.clone()),
        FieldPackPocket::KeyItems => strict_readonly_cursor_index(
            &runtime_shell.key_item_cursor,
            "bag:key-items",
            field_pack_selectable_count(carried_item_count(&snapshot.bag.key_items)),
        )
        .filter(|index| *index < carried_item_count(&snapshot.bag.key_items))
        .and_then(|index| carried_item_offset(&snapshot.bag.key_items, index))
        .and_then(|offset| snapshot.bag.key_items.get(offset))
        .map(|item| item.item_id.clone()),
        FieldPackPocket::TmHm => strict_readonly_cursor_index(
            &runtime_shell.tmhm_cursor,
            "bag:tmhm",
            field_pack_selectable_count(snapshot.bag.tm_hm.len()),
        )
        .filter(|index| *index < snapshot.bag.tm_hm.len())
        .and_then(|index| snapshot.bag.tm_hm.get(index))
        .map(|tmhm| tmhm.item_id.clone()),
        FieldPackPocket::Custom(pocket_id) => {
            let items = snapshot.bag.custom_pockets.get(pocket_id)?;
            strict_readonly_cursor_index(
                &runtime_shell.custom_item_cursor,
                &custom_pack_surface_id(pocket_id),
                field_pack_selectable_count(carried_item_count(items)),
            )
            .filter(|index| *index < carried_item_count(items))
            .and_then(|index| carried_item_offset(items, index))
            .and_then(|offset| items.get(offset))
            .map(|item| item.item_id.clone())
        }
    }
}

fn field_item_action_summary(
    shell: &RuntimeGameShell,
    item: &crate::RuntimeItemCatalogSnapshot,
    item_id: &str,
) -> String {
    if !item.field_usable {
        return "not field usable".to_string();
    }
    if item.repel_steps.is_some() {
        return "repel".to_string();
    }
    for (rule_id, label) in [
        ("escape_rope", "escape rope"),
        ("bicycle", "bicycle"),
        ("itemfinder", "itemfinder"),
        ("squirtbottle", "squirtbottle"),
        ("coin_case", "coin case"),
        ("blue_card", "blue card"),
        ("town_map", "town map"),
    ] {
        if field_rule_item_matches(shell, rule_id, item_id) {
            return label.to_string();
        }
    }
    if shell.fishing_rod_ids().contains(item_id) {
        return "fishing rod".to_string();
    }
    if shell.is_bag_box_item(item_id) {
        return "decoration box".to_string();
    }
    if item.party_revive_hp_percent.is_some() {
        return "whole party".to_string();
    }
    if item_targets_party_move_fields(
        item.pp_restore_scope.as_deref(),
        item.pp_restore_points,
        item.pp_up_stages,
    ) {
        return "target party move".to_string();
    }
    if item_targets_party_pokemon_fields(item) {
        return "target party Pokemon".to_string();
    }
    if shell.is_bag_pokegear_item(item_id) {
        return "pokegear".to_string();
    }
    item.field_menu.clone()
}

fn selected_battle_pack_item_label(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
) -> Option<String> {
    let item_ids = carried_battle_usable_item_ids(snapshot);
    let selected = strict_readonly_cursor_index(
        &runtime_shell.bag_cursor,
        "battle:bag-items",
        item_ids.len(),
    )?;
    let item_id = item_ids.get(selected)?;
    let item = RuntimeBagItemSnapshot {
        item_id: item_id.clone(),
        quantity: carried_item_quantity(snapshot, item_id)?,
    };
    Some(pack_item_entry(snapshot, &item, ""))
}

fn append_field_pack_item_rows(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    pocket: &FieldPackPocket,
    lines: &mut Vec<String>,
) {
    let entries = match pocket {
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
        FieldPackPocket::TmHm => {
            let selected = strict_readonly_cursor_index(
                &runtime_shell.tmhm_cursor,
                "bag:tmhm",
                snapshot.bag.tm_hm.len(),
            );
            let Some(selected) = selected else {
                return lines.push("INVALID CURSOR bag:tmhm".to_string());
            };
            windowed_index_range(selected, snapshot.bag.tm_hm.len())
                .map(|index| {
                    let tmhm = &snapshot.bag.tm_hm[index];
                    let marker = if index == selected { ">" } else { " " };
                    tmhm_pack_entry(snapshot, tmhm, marker)
                })
                .collect()
        }
        FieldPackPocket::Custom(pocket_id) => {
            let Some(items) = snapshot.bag.custom_pockets.get(pocket_id) else {
                return lines.push(format!("INVALID POCKET {pocket_id}"));
            };
            selected_pack_entries(
                snapshot,
                items,
                &runtime_shell.custom_item_cursor,
                &custom_pack_surface_id(pocket_id),
            )
        }
    };
    for entry in entries {
        lines.push(entry);
    }
}

fn selected_pack_entries(
    snapshot: &RuntimeShellSnapshot,
    items: &[RuntimeBagItemSnapshot],
    cursor: &Option<MenuCursor>,
    surface_id: &str,
) -> Vec<String> {
    let carried = items
        .iter()
        .filter(|item| item.quantity > 0)
        .collect::<Vec<_>>();
    let row_count = field_pack_selectable_count(carried.len());
    let Some(selected) = strict_readonly_cursor_index(cursor, surface_id, row_count) else {
        return vec![compact_scene_label(
            &format!("INVALID CURSOR {surface_id}"),
            30,
        )];
    };
    windowed_index_range(selected, row_count)
        .map(|index| {
            let marker = if index == selected { ">" } else { " " };
            if index >= carried.len() {
                pack_cancel_entry(marker)
            } else {
                let item = carried[index];
                pack_item_entry(snapshot, item, marker)
            }
        })
        .collect()
}

fn pack_item_entry(
    snapshot: &RuntimeShellSnapshot,
    item: &RuntimeBagItemSnapshot,
    marker: &str,
) -> String {
    let Some(catalog) = snapshot
        .items
        .iter()
        .find(|catalog| catalog.item_id == item.item_id)
    else {
        return compact_scene_label(&format!("{marker}{} INVALID ITEM", item.item_id), 30);
    };
    compact_scene_label(
        &format!(
            "{marker}{} x{:02}",
            catalog.name.replace('_', " "),
            item.quantity
        ),
        30,
    )
}

fn pack_cancel_entry(marker: &str) -> String {
    format!("{marker}CANCEL")
}

fn selected_tmhm_pack_entries(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
) -> Vec<String> {
    if snapshot.bag.tm_hm.is_empty() {
        return Vec::new();
    }
    let selected = strict_readonly_cursor_index(
        &runtime_shell.tmhm_cursor,
        "bag:tmhm",
        field_pack_selectable_count(snapshot.bag.tm_hm.len()),
    );
    let Some(selected) = selected else {
        return vec![compact_scene_label("INVALID CURSOR bag:tmhm", 30)];
    };
    let row_count = field_pack_selectable_count(snapshot.bag.tm_hm.len());
    windowed_index_range(selected, row_count)
        .map(|index| {
            let marker = if index == selected { ">" } else { " " };
            if index >= snapshot.bag.tm_hm.len() {
                pack_cancel_entry(marker)
            } else {
                let tmhm = &snapshot.bag.tm_hm[index];
                tmhm_pack_entry(snapshot, tmhm, marker)
            }
        })
        .collect()
}

fn tmhm_pack_entry(
    snapshot: &RuntimeShellSnapshot,
    tmhm: &crate::RuntimeTmHmSnapshot,
    marker: &str,
) -> String {
    let Some(move_id) = tmhm.move_id.as_deref() else {
        return compact_scene_label(
            &format!("{marker}{} #{} INVALID MOVE", tmhm.item_id, tmhm.tmhm_index),
            30,
        );
    };
    let Some(_move_data) = snapshot
        .moves
        .iter()
        .find(|move_data| move_data.move_id == move_id)
    else {
        return compact_scene_label(
            &format!("{marker}{} #{} INVALID MOVE", tmhm.item_id, tmhm.tmhm_index),
            30,
        );
    };
    compact_scene_label(
        &format!(
            "{marker}{} x01",
            item_display_name(snapshot, &tmhm.item_id)
        ),
        30,
    )
}

fn party_action_entry(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    action: PartyAction,
    marker: &str,
) -> String {
    match action {
        PartyAction::Summary => compact_scene_label(
            &format!(
                "{marker}STATS {}",
                selected_party_action_subject(snapshot, runtime_shell)
            ),
            30,
        ),
        PartyAction::Switch => compact_scene_label(
            &format!(
                "{marker}SWITCH {}",
                selected_party_action_subject(snapshot, runtime_shell)
            ),
            30,
        ),
        PartyAction::Move => compact_scene_label(
            &format!("{marker}MOVE {}", selected_party_action_subject(snapshot, runtime_shell)),
            30,
        ),
        PartyAction::Item => compact_scene_label(
            &format!(
                "{marker}ITEM {}",
                selected_party_held_item_label(snapshot, runtime_shell)
            ),
            30,
        ),
        PartyAction::Cancel => format!("{marker}CANCEL"),
        PartyAction::FieldMove(field_move) => compact_scene_label(
            &format!(
                "{marker}{} {}",
                party_action_label(action),
                field_move_requirement_summary(snapshot, runtime_shell, field_move)
            ),
            30,
        ),
    }
}

fn party_submenu_action_entry(action: PartyAction, marker: &str) -> String {
    compact_scene_label(&format!("{marker}{}", party_action_label(action)), 30)
}

fn selected_party_action_subject(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
) -> String {
    snapshot
        .party
        .slots
        .get(
            runtime_shell
                .party_cursor
                .min(snapshot.party.slots.len().saturating_sub(1)),
        )
        .map(|slot| {
            format!(
                "{} L{} {}/{}",
                crate::core::models::pokemon_species_display_name(&slot.pokemon.species.id),
                slot.pokemon.level,
                slot.pokemon.hp,
                slot.pokemon.max_hp
            )
        })
        .unwrap_or_else(|| "INVALID PARTY".to_string())
}

fn selected_party_held_item_label(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
) -> String {
    let Some(slot) = snapshot.party.slots.get(
        runtime_shell
            .party_cursor
            .min(snapshot.party.slots.len().saturating_sub(1)),
    ) else {
        return "INVALID PARTY".to_string();
    };
    let Some(item_id) = slot.pokemon.item.as_deref() else {
        return "NO ITEM".to_string();
    };
    item_display_name(snapshot, item_id)
}

fn field_move_requirement_summary(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    field_move: PartyFieldMove,
) -> String {
    let rule_id = party_field_move_rule_id(field_move);
    let Some(rule) = runtime_shell
        .shell
        .field_move_rule_keys()
        .into_iter()
        .find(|key| key.rule_id == rule_id)
    else {
        return format!("INVALID RULE {rule_id}");
    };
    let Some(move_id) = rule.move_id.as_deref() else {
        return format!("INVALID RULE {rule_id}");
    };
    let badge = match (rule.badge_region.as_deref(), rule.badge_index) {
        (Some("johto"), Some(index)) => {
            visible_field_move_badge_summary(&snapshot.progression.badges.johto, rule_id, index)
        }
        (Some("kanto"), Some(index)) => {
            visible_field_move_badge_summary(&snapshot.progression.badges.kanto, rule_id, index)
        }
        (None, None) => "badge=-",
        (region, index) => {
            return format!("INVALID BADGE {region:?}:{index:?}");
        }
    };
    format!("{move_id} {badge}")
}

fn visible_field_move_badge_summary(
    badges: &[bool; 8],
    rule_id: &str,
    index: usize,
) -> &'static str {
    badges
        .get(index)
        .copied()
        .map(|owned| if owned { "badge=ok" } else { "badge=no" })
        .unwrap_or("badge=invalid")
}

fn pokedex_entry_row(
    snapshot: &RuntimeShellSnapshot,
    species: &crate::RuntimePokemonCatalogSnapshot,
    marker: &str,
) -> String {
    let Some(entry) = snapshot
        .presentation
        .pokedex_entries
        .get(&species.species_id)
    else {
        return compact_scene_label(&format!("{marker}#{} INVALID DEX", species.int_id), 30);
    };
    compact_scene_label(
        &format!(
            "{marker}#{:03} {} {} {}pg",
            species.int_id,
            species.species_id,
            entry.classification,
            entry.pages.len()
        ),
        30,
    )
}

fn windowed_party_slot_entries(
    snapshot: &RuntimeShellSnapshot,
    selected_party: usize,
) -> Vec<String> {
    windowed_index_range(selected_party, snapshot.party.slots.len())
        .map(|index| {
            let slot = &snapshot.party.slots[index];
            party_slot_entry(snapshot, slot, index == selected_party)
        })
        .collect()
}

fn party_slot_entry(
    snapshot: &RuntimeShellSnapshot,
    slot: &crate::RuntimePartySlotSnapshot,
    selected: bool,
) -> String {
    let marker = if selected { ">" } else { " " };
    compact_scene_label(&format!("{marker}{}", party_slot_summary(snapshot, slot)), 30)
}

fn party_cancel_entry(selected: bool) -> String {
    let marker = if selected { ">" } else { " " };
    format!("{marker}CANCEL")
}

fn party_switch_slot_entry(
    snapshot: &RuntimeShellSnapshot,
    slot: &crate::RuntimePartySlotSnapshot,
    selected: bool,
    source: bool,
) -> String {
    let marker = if selected { ">" } else { " " };
    let source_marker = if source { "*" } else { "" };
    compact_scene_label(
        &format!("{marker}{source_marker}{}", party_slot_summary(snapshot, slot)),
        30,
    )
}

fn party_slot_summary(
    snapshot: &RuntimeShellSnapshot,
    slot: &crate::RuntimePartySlotSnapshot,
) -> String {
    let active = if slot.is_active_battle_pokemon {
        "*"
    } else {
        ""
    };
    let status = party_status_token(&slot.pokemon);
    let held = slot
        .pokemon
        .item
        .as_deref()
        .map(|item| format!(" @{}", item_display_name(snapshot, item)))
        .unwrap_or_default();
    format!(
        "{active}{} \u{e10a}{} {}/{} {status}{held}",
        slot.pokemon.nickname, slot.pokemon.level, slot.pokemon.hp, slot.pokemon.max_hp
    )
}

fn windowed_move_entries(
    snapshot: &RuntimeShellSnapshot,
    moves: &[crate::core::models::pokemon::LearnedMove],
    selected_move: usize,
) -> Vec<String> {
    windowed_index_range(selected_move, moves.len())
        .map(|index| {
            let learned = &moves[index];
            let marker = if index == selected_move { ">" } else { " " };
            move_menu_entry(snapshot, learned, marker)
        })
        .collect()
}

fn move_menu_entry(
    snapshot: &RuntimeShellSnapshot,
    learned: &crate::core::models::pokemon::LearnedMove,
    marker: &str,
) -> String {
    let Some(move_data) = snapshot
        .moves
        .iter()
        .find(|move_data| move_data.move_id == learned.name)
    else {
        return compact_scene_label(&format!("{marker}{} INVALID MOVE", learned.name), 30);
    };
    compact_scene_label(
        &format!(
            "{marker}{} {}/{} {} P{} A{}",
            move_data.name.replace('_', " "),
            learned.current_pp,
            crate::core::models::max_move_pp(move_data.pp, learned.pp_ups),
            battle_type_display_name(&move_data.move_type),
            move_data.power,
            move_data.accuracy
        ),
        30,
    )
}

fn windowed_index_range(selected: usize, len: usize) -> std::ops::Range<usize> {
    let start = visible_window_start(selected, len, SCENE_MENU_VISIBLE_ROWS);
    let end = (start + SCENE_MENU_VISIBLE_ROWS).min(len);
    start..end
}

fn visible_window_start(selected: usize, len: usize, window_len: usize) -> usize {
    if window_len == 0 || len <= window_len {
        return 0;
    }
    selected
        .saturating_sub(window_len / 2)
        .min(len.saturating_sub(window_len))
}

fn scene_menu_uses_two_columns(entries: &[String]) -> bool {
    entries.len() <= 4 && entries.iter().all(|entry| entry.chars().count() <= 18)
}

fn scene_menu_display_entry(entry: &str, two_columns: bool) -> String {
    let max_chars = if two_columns { 18 } else { 30 };
    compact_scene_label(entry, max_chars)
}

fn scene_menu_entry_position(
    origin_x: f32,
    origin_y: f32,
    index: usize,
    two_columns: bool,
) -> (f32, f32) {
    if two_columns {
        let row = index / 2;
        let col = index % 2;
        (
            origin_x + col as f32 * TILE_SIZE * 3.2,
            origin_y + TILE_SIZE * 0.86 - row as f32 * TILE_SIZE * 0.78,
        )
    } else {
        (
            origin_x + TILE_SIZE * 0.15,
            origin_y + TILE_SIZE * 0.95 - index as f32 * TILE_SIZE * 0.66,
        )
    }
}

fn split_overlay_row(row: &str, entries_per_line: usize) -> Vec<String> {
    let entries = row.split(" | ").collect::<Vec<_>>();
    if entries.len() <= entries_per_line || entries_per_line == 0 {
        return vec![row.to_string()];
    }
    entries
        .chunks(entries_per_line)
        .map(|chunk| chunk.join(" | "))
        .collect()
}

fn field_pack_pocket_label(pocket: &FieldPackPocket) -> String {
    match pocket {
        FieldPackPocket::Items => "Items".to_string(),
        FieldPackPocket::Balls => "Balls".to_string(),
        FieldPackPocket::KeyItems => "Key".to_string(),
        FieldPackPocket::TmHm => "TM/HM".to_string(),
        FieldPackPocket::Custom(pocket_id) => pocket_id.clone(),
    }
}

fn append_pokedex_context(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    lines: &mut Vec<String>,
) {
    if !runtime_shell.pokedex_menu_open {
        return;
    }
    lines.extend(visible_pokedex_menu_entries(snapshot, runtime_shell));
}

fn append_pokegear_context(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    lines: &mut Vec<String>,
) {
    if !runtime_shell.pokegear_menu_open {
        return;
    }
    lines.extend(visible_pokegear_menu_entries(snapshot, runtime_shell));
}

fn append_options_context(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    lines: &mut Vec<String>,
) {
    if !runtime_shell.options_menu_open {
        return;
    }
    lines.extend(visible_options_menu_entries(snapshot, runtime_shell));
}

fn append_trainer_card_context(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    lines: &mut Vec<String>,
) {
    if !runtime_shell.trainer_card_open {
        return;
    }
    lines.extend(visible_trainer_card_entries(snapshot, runtime_shell));
}

fn append_save_context(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    lines: &mut Vec<String>,
) {
    if !runtime_shell.save_menu_open {
        return;
    }
    let mut entries = Vec::new();
    if push_visible_save_dialog_entries(&mut entries, snapshot, runtime_shell).is_ok() {
        lines.extend(entries);
    }
}

fn visible_badge_count(badges: &crate::core::state::Badges) -> usize {
    badges
        .johto
        .iter()
        .chain(badges.kanto.iter())
        .filter(|awarded| **awarded)
        .count()
}

fn append_nearby_map_context(
    runtime_shell: &BevyRuntimeShell,
    snapshot: &RuntimeShellSnapshot,
    map: &crate::RuntimeMapCatalogSnapshot,
    visible_objects: &[crate::core::map::ObjectEvent],
    lines: &mut Vec<String>,
) -> Result<()> {
    let TilePosition {
        x: front_x,
        y: front_y,
    } = facing_runtime_tile(snapshot)?;
    lines.push(format!(
        "front tile=({}, {}) facing={:?}",
        front_x, front_y, snapshot.overworld.facing
    ));
    if let Some(prompt) = field_prompt_label(runtime_shell, snapshot, map, front_x, front_y) {
        lines.push(format!("front_prompt={prompt}"));
    }

    let front_tile = TilePosition::new(front_x, front_y);
    let mut front_objects = 0;
    for object in visible_objects.iter() {
        if !snapshot_object_tile_matches_checked(snapshot, object, front_tile)? {
            continue;
        }
        lines.push(format!(
            "front_object id={:?} sprite={} script={} flag={} type={}",
            object.object_identifier,
            object.sprite,
            object.script,
            object.event_flag,
            object.object_type
        ));
        front_objects += 1;
        if front_objects >= 4 {
            break;
        }
    }
    let mut front_background_events = 0;
    for bg in map.events.bg_events.iter() {
        if !background_event_tile_matches_checked(bg, front_tile)? {
            continue;
        }
        lines.push(format!(
            "front_bg type={} script={} tile=({}, {})",
            bg.event_type, bg.script, bg.x, bg.y
        ));
        front_background_events += 1;
        if front_background_events >= 4 {
            break;
        }
    }
    let mut standing_warps = 0;
    for warp in map.events.warps.iter() {
        if !warp_tile_matches_checked(warp, snapshot.overworld.tile)? {
            continue;
        }
        lines.push(format!(
            "standing_warp index={} target={} target_warp={}",
            warp.index, warp.target_map, warp.target_warp_id
        ));
        standing_warps += 1;
        if standing_warps >= 4 {
            break;
        }
    }
    let mut standing_coord_events = 0;
    for coord in map.events.coord_events.iter() {
        if !coord_event_tile_matches_checked(coord, snapshot.overworld.tile)? {
            continue;
        }
        lines.push(format!(
            "standing_coord scene={} script={}",
            coord.scene_id, coord.script_name
        ));
        standing_coord_events += 1;
        if standing_coord_events >= 4 {
            break;
        }
    }
    Ok(())
}

fn snapshot_object_tile_matches_checked(
    snapshot: &RuntimeShellSnapshot,
    object: &crate::core::map::ObjectEvent,
    tile: TilePosition,
) -> Result<bool> {
    let object_tile = object
        .object_identifier
        .as_ref()
        .and_then(|object_id| {
            snapshot
                .visible_object_runtime_tiles
                .get(object_id)
                .copied()
        })
        .or_else(|| object_tile_position_checked(object))
        .with_context(|| {
            format!(
                "object {:?} has out-of-range runtime coordinates ({}, {})",
                object.object_identifier, object.x, object.y
            )
        })?;
    Ok(object_tile == tile)
}

fn background_event_tile_matches_checked(
    event: &crate::core::map::BackgroundEvent,
    tile: TilePosition,
) -> Result<bool> {
    let Some(event_tile) = background_event_tile_position_checked(event) else {
        anyhow::bail!(
            "background event '{}' has out-of-range runtime coordinates ({}, {})",
            event.script,
            event.x,
            event.y
        );
    };
    Ok(event_tile == tile)
}

fn warp_tile_matches_checked(
    event: &crate::core::map::WarpEvent,
    tile: TilePosition,
) -> Result<bool> {
    let Some(event_tile) = warp_tile_position_checked(event) else {
        anyhow::bail!(
            "warp {} has out-of-range runtime coordinates ({}, {})",
            event.index,
            event.x,
            event.y
        );
    };
    Ok(event_tile == tile)
}

fn coord_event_tile_matches_checked(
    event: &crate::core::map::CoordEvent,
    tile: TilePosition,
) -> Result<bool> {
    let Some(event_tile) = coord_event_tile_position_checked(event) else {
        anyhow::bail!(
            "coord event '{}' has out-of-range runtime coordinates ({}, {})",
            event.script_name,
            event.x,
            event.y
        );
    };
    Ok(event_tile == tile)
}

fn format_party_details(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    lines: &mut Vec<String>,
) {
    if snapshot.party.slots.is_empty() {
        lines.push("party empty".to_string());
        return;
    }
    let selected_slot = runtime_shell
        .party_cursor
        .min(snapshot.party.slots.len().saturating_sub(1));
    let selected_party_index = snapshot.party.slots[selected_slot].index;
    let selected_move_slot = readonly_cursor_index(
        &runtime_shell.party_move_cursor,
        &party_move_cursor_surface_id(selected_party_index),
        snapshot.party.slots[selected_slot].pokemon.moves.len(),
    );
    lines.push(format!(
        "party_cursor={} move_cursor={}",
        selected_slot + 1,
        selected_move_slot
            .map(|slot| (slot + 1).to_string())
            .unwrap_or_else(|| "-".to_string())
    ));
    for (slot_offset, slot) in snapshot.party.slots.iter().enumerate() {
        let moves = slot
            .pokemon
            .moves
            .iter()
            .enumerate()
            .map(|(move_index, learned)| {
                let move_marker =
                    if slot_offset == selected_slot && Some(move_index) == selected_move_slot {
                        ">"
                    } else {
                        ""
                    };
                format!("{}{}({}pp)", move_marker, learned.name, learned.current_pp)
            })
            .collect::<Vec<_>>()
            .join(", ");
        let marker = if slot_offset == selected_slot {
            ">"
        } else if slot.is_active_battle_pokemon {
            "*"
        } else {
            " "
        };
        lines.push(format!(
            "{}{} {} lvl={} hp={}/{} status={:?} item={:?} moves=[{}]",
            marker,
            slot.index,
            slot.pokemon.species.id,
            slot.pokemon.level,
            slot.pokemon.hp,
            slot.pokemon.max_hp,
            slot.pokemon.status,
            slot.pokemon.item,
            moves
        ));
    }
}

fn format_bag_details(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    lines: &mut Vec<String>,
) {
    let selected_item = readonly_cursor_index(
        &runtime_shell.bag_cursor,
        "bag:items",
        carried_item_count(&snapshot.bag.items),
    )
    .and_then(|index| carried_item_offset(&snapshot.bag.items, index));
    let selected_ball = readonly_cursor_index(
        &runtime_shell.ball_cursor,
        "bag:balls",
        carried_item_count(&snapshot.bag.balls),
    )
    .and_then(|index| carried_item_offset(&snapshot.bag.balls, index));
    let selected_key_item = readonly_cursor_index(
        &runtime_shell.key_item_cursor,
        "bag:key-items",
        carried_item_count(&snapshot.bag.key_items),
    )
    .and_then(|index| carried_item_offset(&snapshot.bag.key_items, index));
    let selected_pc_item = readonly_cursor_index(
        &runtime_shell.pc_item_cursor,
        "pc:items",
        carried_item_count(&snapshot.bag.pc_items),
    )
    .and_then(|index| carried_item_offset(&snapshot.bag.pc_items, index));
    append_item_section(snapshot, "items", &snapshot.bag.items, selected_item, lines);
    append_item_section(snapshot, "balls", &snapshot.bag.balls, selected_ball, lines);
    append_item_section(
        snapshot,
        "key_items",
        &snapshot.bag.key_items,
        selected_key_item,
        lines,
    );
    if !snapshot.bag.tm_hm.is_empty() {
        lines.push("tm_hm:".to_string());
        let selected_tmhm = readonly_cursor_index(
            &runtime_shell.tmhm_cursor,
            "bag:tmhm",
            snapshot.bag.tm_hm.len(),
        );
        for (index, tm) in snapshot.bag.tm_hm.iter().enumerate() {
            let marker = if Some(index) == selected_tmhm {
                ">"
            } else {
                " "
            };
            lines.push(format!("{marker} {tm:?}"));
        }
    }
    for (pocket_id, items) in &snapshot.bag.custom_pockets {
        let selected_custom = readonly_cursor_index(
            &runtime_shell.custom_item_cursor,
            &custom_pack_surface_id(pocket_id),
            carried_item_count(items),
        )
        .and_then(|index| carried_item_offset(items, index));
        append_item_section(
            snapshot,
            &format!("custom_pocket:{pocket_id}"),
            items,
            selected_custom,
            lines,
        );
    }
    append_item_section(snapshot, "pc_items", &snapshot.bag.pc_items, selected_pc_item, lines);
}

fn carried_item_count(items: &[RuntimeBagItemSnapshot]) -> usize {
    items.iter().filter(|item| item.quantity > 0).count()
}

fn carried_item_offset(items: &[RuntimeBagItemSnapshot], carried_index: usize) -> Option<usize> {
    items
        .iter()
        .enumerate()
        .filter(|(_, item)| item.quantity > 0)
        .nth(carried_index)
        .map(|(offset, _)| offset)
}

fn append_item_section(
    snapshot: &RuntimeShellSnapshot,
    label: &str,
    items: &[crate::RuntimeBagItemSnapshot],
    selected_index: Option<usize>,
    lines: &mut Vec<String>,
) {
    if items.is_empty() {
        return;
    }
    lines.push(format!("{label}:"));
    for (index, item) in items.iter().take(12).enumerate() {
        let marker = if Some(index) == selected_index {
            ">"
        } else {
            " "
        };
        lines.push(format!(
            "{marker} {} x{}",
            item_display_name(snapshot, &item.item_id),
            item.quantity
        ));
    }
    if items.len() > 12 {
        lines.push(format!("  ... {} more", items.len() - 12));
    }
}

fn readonly_cursor_index(
    cursor: &Option<MenuCursor>,
    surface_id: &str,
    option_count: usize,
) -> Option<usize> {
    if option_count == 0 {
        return None;
    }
    cursor
        .as_ref()
        .filter(|cursor| cursor.surface_id == surface_id)
        .map(|cursor| cursor.option_index.min(option_count - 1))
}

fn strict_readonly_cursor_index(
    cursor: &Option<MenuCursor>,
    surface_id: &str,
    option_count: usize,
) -> Option<usize> {
    if option_count == 0 {
        return None;
    }
    cursor
        .as_ref()
        .filter(|cursor| cursor.surface_id == surface_id && cursor.option_index < option_count)
        .map(|cursor| cursor.option_index)
}

fn selected_party_move_name(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
) -> String {
    let selected_slot = runtime_shell
        .party_cursor
        .min(snapshot.party.slots.len().saturating_sub(1));
    let Some(slot) = snapshot.party.slots.get(selected_slot) else {
        return "-".to_string();
    };
    readonly_cursor_index(
        &runtime_shell.party_move_cursor,
        &party_move_cursor_surface_id(slot.index),
        slot.pokemon.moves.len(),
    )
    .and_then(|move_slot| slot.pokemon.moves.get(move_slot))
    .map(|learned| learned.name.clone())
    .unwrap_or_else(|| "-".to_string())
}

fn format_battle_details(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    lines: &mut Vec<String>,
) {
    let Some(battle) = &snapshot.battle else {
        lines.push("battle none".to_string());
        return;
    };
    lines.push(format!(
        "battle {:?} type={} enemy={} lvl={} hp={}/{}",
        battle.kind,
        battle.battle_type,
        battle.enemy_pokemon.species.id,
        battle.enemy_pokemon.level,
        battle.enemy_pokemon.hp,
        battle.enemy_pokemon.max_hp
    ));
    lines.push(format!(
        "active_player={:?} active_enemy={:?} rewarded={:?}",
        battle.active_player_party_index,
        battle.active_enemy_party_index,
        battle.rewarded_enemy_party_indices
    ));
    lines.push(format!(
        "commands player_moves={:?} enemy_moves={:?} switches={:?} items={} balls={} run={} escapes={}",
        battle.commands.player_move_slots,
        battle.commands.enemy_move_slots,
        battle.commands.switch_party_indices,
        battle.commands.can_use_items,
        carried_ball_item_ids(snapshot).len(),
        battle.commands.can_run,
        battle.escape_attempts
    ));
    append_battle_cursor_context(snapshot, runtime_shell, lines);
    if !battle.enemy_party.is_empty() {
        lines.push(format!("enemy_party_count={}", battle.enemy_party.len()));
    }
    let ball_items = carried_ball_item_ids(snapshot);
    if !ball_items.is_empty() {
        for (index, ball) in ball_items.iter().take(8).enumerate() {
            lines.push(format!("ball {} item={}", index + 1, ball));
        }
    }
}

fn format_ui_details(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    lines: &mut Vec<String>,
) {
    lines.push(format!(
        "window={} text_window={} coords={:?} picture={:?}",
        snapshot.ui.window_open,
        snapshot.ui.text_window_open,
        snapshot.ui.coords,
        snapshot.ui.active_pokemon_picture
    ));
    if !snapshot.linked_menu_results.is_empty() {
        lines.push(format!(
            "linked_menu_results={}",
            snapshot.linked_menu_results.len()
        ));
        for result in snapshot.linked_menu_results.iter().take(4) {
            lines.push(format!("  linked_result {result:?}"));
        }
    }
    if let Some(shop) = &snapshot.pending_shop {
        lines.push(format!("shop={shop:?}"));
    }
    if let Some(text) = &snapshot.ui.text {
        lines.push(format!(
            "text label={} source={:?} asm={:?} queued={}",
            text.label, text.source, text.asm_text, text.queued_text_events
        ));
    }
    if let Some(menu) = &snapshot.ui.menu {
        lines.push(format!(
            "menu={} source={:?} coords={:?} vertical_menus={}",
            menu.menu_id,
            menu.source,
            menu.coords,
            menu.layout.vertical_menus.len()
        ));
        for vertical in &menu.layout.vertical_menus {
            lines.push(format!(
                "  {} command={} options={:?}",
                vertical.source_script, vertical.verticalmenu_command_index, vertical.options
            ));
        }
    }
    if let Some(prompt) = &snapshot.ui.pending_yes_no {
        if let Some(selected) =
            strict_readonly_cursor_index(&runtime_shell.yes_no_cursor, "ui:yes-no", 2)
        {
            let selected_label = if selected == 0 { "YES" } else { "NO" };
            lines.push(format!(
                "yes_no_selected={selected_label} prompt={prompt:?}"
            ));
        } else {
            lines.push(format!("invalid_cursor=yes_no prompt={prompt:?}"));
        }
    }
    if let Some(wait) = &snapshot.ui.pending_text_wait {
        lines.push(format!("text_wait={wait:?}"));
    }
    if !snapshot.presentation.pc_strings.is_empty() {
        lines.push(format!(
            "pc_strings={}",
            snapshot.presentation.pc_strings.len()
        ));
        for (key, text) in snapshot.presentation.pc_strings.iter().take(6) {
            lines.push(format!("  {key}={text}"));
        }
    }
    if !snapshot.ui.elevators.is_empty() {
        lines.push(format!("elevators={:?}", snapshot.ui.elevators));
    }
    if !snapshot.ui.gift_pokemon.is_empty() {
        lines.push(format!("gift_pokemon={:?}", snapshot.ui.gift_pokemon));
    }
}

fn format_progress_details(snapshot: &RuntimeShellSnapshot, lines: &mut Vec<String>) {
    lines.push(format!(
        "trainer={} id={} money={} moms_money={} coins={} pc_box={} event_flags={} engine_flags={}",
        snapshot.trainer.player_name,
        snapshot.trainer.player_id,
        snapshot.trainer.money,
        snapshot.trainer.moms_money,
        snapshot.trainer.coins,
        snapshot.trainer.current_pc_box,
        snapshot.progression.active_event_flags.len(),
        snapshot.progression.active_engine_flags.len()
    ));
    lines.push(format!(
        "pokedex seen={} owned={} badges={:?}",
        snapshot.progression.pokedex_seen,
        snapshot.progression.pokedex_owned,
        snapshot.progression.badges
    ));
    lines.push(format!(
        "link wins={} losses={} draws={} repel={} active_repel={:?} last_spawn={:?}",
        snapshot.progression.link_wins,
        snapshot.progression.link_losses,
        snapshot.progression.link_draws,
        snapshot.progression.repel_steps_remaining,
        snapshot.progression.active_repel_item,
        snapshot.progression.last_spawn_identifier
    ));
    lines.push(format!("time={:?}", snapshot.progression.time));
    lines.push(format!(
        "multiplayer frame={} state_hash={:#010x} rng_seed={:#010x} linked_menu_results={}",
        snapshot.state_checksum.frame(),
        snapshot.state_checksum.hash(),
        snapshot.progression.rng_seed,
        snapshot.linked_menu_results.len()
    ));
    for flag in snapshot.progression.active_engine_flags.iter().take(8) {
        lines.push(format!("engine_flag {flag}"));
    }
    for flag in snapshot.progression.active_event_flags.iter().take(8) {
        lines.push(format!("event_flag {flag}"));
    }
}

fn format_storage_details(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    lines: &mut Vec<String>,
) {
    lines.push(format!(
        "storage current_box={} party_count={} boxes={}",
        snapshot.storage.current_pc_box,
        snapshot.storage.party_count,
        snapshot.storage.boxes.len()
    ));
    for box_snapshot in snapshot.storage.boxes.iter().take(8) {
        let selected_slot = if box_snapshot.index == snapshot.storage.current_pc_box {
            readonly_cursor_index(
                &runtime_shell.storage_cursor,
                &storage_cursor_surface_id(box_snapshot.index),
                box_snapshot.slots.len(),
            )
        } else {
            None
        };
        lines.push(format!(
            "box {} {} count={}",
            box_snapshot.index, box_snapshot.name, box_snapshot.count
        ));
        for (slot_offset, slot) in box_snapshot.slots.iter().take(4).enumerate() {
            let marker = if Some(slot_offset) == selected_slot {
                ">"
            } else {
                " "
            };
            lines.push(format!(
                "{marker} {} {} lvl={} hp={}/{} item={:?}",
                slot.index,
                slot.pokemon.species.id,
                slot.pokemon.level,
                slot.pokemon.hp,
                slot.pokemon.max_hp,
                slot.pokemon.item
            ));
        }
        if box_snapshot.slots.len() > 4 {
            lines.push(format!("  ... {} more", box_snapshot.slots.len() - 4));
        }
    }
    if snapshot.storage.boxes.len() > 8 {
        lines.push(format!(
            "... {} more boxes",
            snapshot.storage.boxes.len() - 8
        ));
    }
}

fn format_map_details(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    lines: &mut Vec<String>,
) {
    let Some(map) = snapshot
        .maps
        .iter()
        .find(|map| map.map_name == snapshot.overworld.map_name)
    else {
        lines.push(format!(
            "map {} missing from catalog",
            snapshot.overworld.map_name
        ));
        return;
    };
    lines.push(format!(
        "map={} id={} size={}x{} tileset={} border={} blocks={} music={:?}",
        map.map_name,
        map.id,
        map.attributes.width,
        map.attributes.height,
        map.attributes.tileset_name,
        map.attributes.border_block,
        map.blocks.len(),
        map.attributes.music
    ));
    if let Some(metadata) = &map.metadata {
        lines.push(format!(
            "meta constant={} display={} group={} ids={}:{} env={} phone={}",
            metadata.constant,
            metadata.name,
            metadata.group_name,
            metadata.group_id,
            metadata.map_id,
            metadata.environment,
            metadata.phone_service
        ));
    }
    for spawn in snapshot
        .spawn_points
        .iter()
        .filter(|spawn| spawn.map_name == map.map_name)
        .take(8)
    {
        lines.push(format!(
            "spawn {} map_constant={} tile=({}, {}) group={}:{}",
            spawn.identifier,
            spawn.map_constant,
            spawn.tile_x,
            spawn.tile_y,
            spawn.group_name,
            spawn.group_id
        ));
    }
    lines.push(format!(
        "events warps={} coord={} bg={} objects={} visible_objects={} scenes={} connections={}",
        map.events.warps.len(),
        map.events.coord_events.len(),
        map.events.bg_events.len(),
        map.objects.len(),
        snapshot.visible_objects.len(),
        map.scenes.scenes.len(),
        map.attributes.connections.len()
    ));
    for warp in map.events.warps.iter().take(8) {
        lines.push(format_warp_event_detail_line(warp));
    }
    for object in snapshot.visible_objects.iter().take(8) {
        lines.push(format_visible_object_detail_line(object));
    }
    let active_flypoints = active_fly_destinations(snapshot, &runtime_shell.shell)
        .into_iter()
        .map(|destination| destination.flypoint_flag)
        .take(8)
        .collect::<Vec<_>>();
    if !active_flypoints.is_empty() {
        lines.push(format!("active_flypoints={active_flypoints:?}"));
    }
    if let Some(wild) = snapshot.encounters.wild.get(&map.map_name) {
        lines.push(format!(
            "wild grass_rates={:?} water_rate={:?}",
            wild.grass_rates, wild.water_rate
        ));
    }
    if let Some(field) = snapshot.encounters.field.get(&map.map_name) {
        lines.push(format!("field encounter tables={}", field.tables.len()));
    }
}

fn format_script_details(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    lines: &mut Vec<String>,
) {
    let scripts = &snapshot.script_events;
    lines.push(format!(
        "script_value={:?} command_cursor={} variables={} memory={} buffers={} phones={} last_special={:?} last_talked={:?}",
        scripts.script_value,
        runtime_shell.script_command_cursor,
        scripts.variables.len(),
        scripts.memory.len(),
        scripts.named_buffers.len(),
        scripts.phone_numbers.len(),
        scripts.last_special_routine,
        scripts.last_talked_object
    ));
    lines.push(format!(
        "queues delays={} emotes={} commands={} call_stack={} deferred={} ended={} variable_writes={} effects={} asm={} text={} audio={} graphics={}",
        scripts.pending_delays.len(),
        scripts.pending_emotes.len(),
        scripts.command_queue.len(),
        scripts.call_stack.len(),
        scripts.deferred_scripts.len(),
        scripts.script_ended.is_some(),
        scripts.variable_writes.len(),
        scripts.effects.len(),
        scripts.asm_directives.len(),
        scripts.text_events.len(),
        scripts.audio_events.len(),
        scripts.graphics_events.len()
    ));
    lines.push(format!(
        "pending warp={:?} load={:?} refresh={:?} yes_no={:?} text_wait={:?}",
        scripts.pending_script_warp,
        scripts.pending_map_load,
        scripts.pending_map_refresh,
        snapshot.ui.pending_yes_no,
        snapshot.ui.pending_text_wait
    ));
    for (key, value) in scripts.variables.iter().take(8) {
        lines.push(format!("var {key}={value}"));
    }
    for command in scripts.command_queue.iter().take(8) {
        lines.push(format!("command {command:?}"));
    }
    for effect in scripts.effects.iter().take(8) {
        lines.push(format!("effect {effect:?}"));
    }
    append_current_map_script_command_summary(snapshot, runtime_shell, lines);
}

fn append_current_map_script_command_summary(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    lines: &mut Vec<String>,
) {
    let current_map = snapshot.overworld.map_name.as_str();
    let map_commands = runtime_shell
        .shell
        .script_map_command_keys()
        .into_iter()
        .filter(|key| key.map_name == current_map)
        .collect::<Vec<_>>();
    let shop_commands = runtime_shell
        .shell
        .script_shop_command_keys()
        .into_iter()
        .filter(|key| key.map_name == current_map)
        .collect::<Vec<_>>();
    let flag_commands = runtime_shell
        .shell
        .script_flag_command_keys()
        .into_iter()
        .filter(|key| key.map_name == current_map)
        .collect::<Vec<_>>();
    let scene_commands = runtime_shell
        .shell
        .script_scene_command_keys()
        .into_iter()
        .filter(|key| key.map_name == current_map)
        .collect::<Vec<_>>();
    let object_commands = runtime_shell
        .shell
        .script_object_command_keys()
        .into_iter()
        .filter(|key| key.map_name == current_map)
        .collect::<Vec<_>>();
    let variable_commands = runtime_shell
        .shell
        .script_variable_command_keys()
        .into_iter()
        .filter(|key| key.map_name == current_map)
        .collect::<Vec<_>>();
    let control_commands = runtime_shell
        .shell
        .script_control_command_keys()
        .into_iter()
        .filter(|key| key.map_name == current_map)
        .collect::<Vec<_>>();
    let item_grants = runtime_shell
        .shell
        .script_item_grant_keys()
        .into_iter()
        .filter(|key| key.map_name == current_map)
        .collect::<Vec<_>>();
    let item_access = runtime_shell
        .shell
        .script_item_access_keys()
        .into_iter()
        .filter(|key| key.map_name == current_map)
        .collect::<Vec<_>>();
    let field_pickups = runtime_shell
        .shell
        .script_field_pickup_keys()
        .into_iter()
        .filter(|key| key.map_name == current_map)
        .collect::<Vec<_>>();
    let economy_commands = runtime_shell
        .shell
        .script_economy_command_keys()
        .into_iter()
        .filter(|key| key.map_name == current_map)
        .collect::<Vec<_>>();
    let audio_commands = runtime_shell
        .shell
        .script_audio_command_keys()
        .into_iter()
        .filter(|key| key.map_name == current_map)
        .collect::<Vec<_>>();
    let text_commands = runtime_shell
        .shell
        .script_text_command_keys()
        .into_iter()
        .filter(|key| key.map_name == current_map)
        .collect::<Vec<_>>();
    let phone_commands = runtime_shell
        .shell
        .script_phone_command_keys()
        .into_iter()
        .filter(|key| key.map_name == current_map)
        .collect::<Vec<_>>();
    let swarm_commands = runtime_shell
        .shell
        .script_swarm_command_keys()
        .into_iter()
        .filter(|key| key.map_name == current_map)
        .collect::<Vec<_>>();
    let runtime_commands = runtime_shell
        .shell
        .script_runtime_command_keys()
        .into_iter()
        .filter(|key| key.map_name == current_map)
        .collect::<Vec<_>>();

    lines.push(format!(
        "current_map_commands map={} cursor={} script_map={} shop={} flags={} scenes={} objects={} vars={} control={} grants={} access={} pickups={} economy={} audio={} text={} phone={} swarm={} runtime={}",
        current_map,
        runtime_shell.script_command_cursor,
        map_commands.len(),
        shop_commands.len(),
        flag_commands.len(),
        scene_commands.len(),
        object_commands.len(),
        variable_commands.len(),
        control_commands.len(),
        item_grants.len(),
        item_access.len(),
        field_pickups.len(),
        economy_commands.len(),
        audio_commands.len(),
        text_commands.len(),
        phone_commands.len(),
        swarm_commands.len(),
        runtime_commands.len()
    ));

    for command in map_commands.iter().take(3) {
        lines.push(format!(
            "  mapcmd {}:{} command={} target={:?} xy=({:?},{:?})",
            command.source_script,
            command.command_index,
            command.command,
            command.target_map,
            command.x,
            command.y
        ));
    }
    for command in shop_commands.iter().take(3) {
        lines.push(format!(
            "  shopcmd {}:{} command={} mart={}:{}",
            command.source_script,
            command.command_index,
            command.command,
            command.mart_type,
            command.mart_id
        ));
    }
    for command in flag_commands.iter().take(3) {
        lines.push(format!(
            "  flagcmd {}:{} command={} flag={}",
            command.source_script, command.command_index, command.command, command.flag_id
        ));
    }
    for command in scene_commands.iter().take(3) {
        lines.push(format!(
            "  scenecmd {}:{} command={} map={:?} scene={:?}",
            command.source_script,
            command.command_index,
            command.command,
            command.map_id,
            command.scene_id
        ));
    }
    for command in object_commands.iter().take(3) {
        lines.push(format!(
            "  objectcmd {}:{} command={} object={:?} target={:?} xy=({:?},{:?})",
            command.source_script,
            command.command_index,
            command.command,
            command.object_id,
            command.target_object_id,
            command.x,
            command.y
        ));
    }
    for command in variable_commands.iter().take(3) {
        lines.push(format!(
            "  varcmd {}:{} command={} target={:?} values={:?}",
            command.source_script,
            command.command_index,
            command.command,
            command.target,
            command.value_tokens
        ));
    }
    for command in control_commands.iter().take(3) {
        lines.push(format!(
            "  controlcmd {}:{} command={} target={:?} resolved={:?}",
            command.source_script,
            command.command_index,
            command.command,
            command.target_label,
            command.resolved_target_script
        ));
    }
    for command in item_grants.iter().take(3) {
        lines.push(format!(
            "  grantcmd {}:{} command={} item={} quantity={}",
            command.source_script,
            command.command_index,
            command.command,
            command.item_id,
            command.quantity
        ));
    }
    for command in item_access.iter().take(3) {
        lines.push(format!(
            "  itemcmd {}:{} command={} item={}",
            command.source_script, command.command_index, command.command, command.item_id
        ));
    }
    for command in field_pickups.iter().take(3) {
        lines.push(format!(
            "  pickupcmd {}:{} command={} item={:?} fruit={:?}",
            command.source_script,
            command.command_index,
            command.command,
            command.item_id,
            command.fruit_tree_id
        ));
    }
    for command in economy_commands.iter().take(3) {
        lines.push(format!(
            "  economycmd {}:{} command={} account={:?} amount={:?}",
            command.source_script,
            command.command_index,
            command.command,
            command.account,
            command.amount_tokens
        ));
    }
    for command in audio_commands.iter().take(3) {
        lines.push(format!(
            "  audiocmd {}:{} command={} audio={:?} fade={:?}",
            command.source_script,
            command.command_index,
            command.command,
            command.audio_id,
            command.fade_frames
        ));
    }
    for command in text_commands.iter().take(3) {
        lines.push(format!(
            "  textcmd {}:{} command={} label={:?}",
            command.source_script, command.command_index, command.command, command.text_label
        ));
    }
    for command in phone_commands.iter().take(3) {
        lines.push(format!(
            "  phonecmd {}:{} command={} contact={}",
            command.source_script, command.command_index, command.command, command.contact_id
        ));
    }
    for command in swarm_commands.iter().take(3) {
        lines.push(format!(
            "  swarmcmd {}:{} command={} token={} map_id={}",
            command.source_script,
            command.command_index,
            command.command,
            command.swarm_token,
            command.map_id
        ));
    }
    for command in runtime_commands.iter().take(3) {
        lines.push(format!(
            "  runtimecmd {}:{} command={} args={:?}",
            command.source_script, command.command_index, command.command, command.args
        ));
    }
}

fn format_audio_details(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    lines: &mut Vec<String>,
) {
    lines.push(format!(
        "audio current_music={:?} queued={} catalog music={} sfx={} cries={}",
        snapshot.audio.current_music,
        snapshot.audio.queued_events.len(),
        snapshot.audio_catalog.music.len(),
        snapshot.audio_catalog.sound_effects.len(),
        snapshot.audio_catalog.cries.len()
    ));
    lines.push(format!(
        "resolved_events={} pending_audio={}",
        runtime_shell.last_audio_events.len(),
        runtime_shell.pending_audio.len()
    ));
    for event in runtime_shell.last_audio_events.iter().take(8) {
        lines.push(format!("event {event}"));
    }
    for (music, _) in snapshot.audio_catalog.music.iter().take(8) {
        lines.push(format!("music {music}"));
    }
    for (effect, _) in snapshot.audio_catalog.sound_effects.iter().take(8) {
        lines.push(format!("sfx {effect}"));
    }
    for (cry, _) in snapshot.audio_catalog.cries.iter().take(8) {
        lines.push(format!("cry {cry}"));
    }
}

fn format_special_details(snapshot: &RuntimeShellSnapshot, lines: &mut Vec<String>) {
    let special = &snapshot.special;
    lines.push(format!(
        "special routines={} contacts={} permanent_phones={} calls={} trades={}",
        special.special_routines.len(),
        special.phone_contacts.0.len(),
        special.permanent_phone_numbers.len(),
        special.special_phone_calls.len(),
        special.npc_trades.len()
    ));
    lines.push(format!(
        "features shuckie={} bug_contest={} battle_tower={} happiness={} buena_categories={} prizes={} roaming={}",
        special.shuckie_gift.is_some(),
        special.bug_contest_config.is_some(),
        special.battle_tower_rules.is_some(),
        special.happiness_data.is_some(),
        special.buena_password_categories.categories.len(),
        special.buena_prizes.len(),
        special.roaming_pokemon.len()
    ));
    lines.push(format!(
        "services heal={} pc={} delete={} name_rater={} tutor={} menu={} time={} story={} daycare={} noop={}",
        special.special_routines.contains_key("HealParty"),
        special.special_routines.contains_key("PokemonCenterPC"),
        special.special_routines.contains_key("MoveDeletion"),
        special.special_routines.contains_key("NameRater"),
        special.special_routines.contains_key("MoveTutor"),
        [
            "BankOfMom",
            "SlotMachine",
            "CardFlip",
            "DisplayLinkRecord",
            "TrainerHouse",
            "PhotoStudio",
            "Menu_ChallengeExplanationCancel",
        ]
            .into_iter()
            .filter(|routine| special.special_routines.contains_key(*routine))
            .count(),
        [
            "SetDayOfWeek",
            "InitialSetDSTFlag",
            "InitialClearDSTFlag",
            "UpdateTime",
            "SampleKenjiBreakCountdown",
            "CheckLuckyNumberShowFlag",
            "ResetLuckyNumberShowFlag",
            "PrintTodaysLuckyNumber",
            "CheckForLuckyNumberWinners",
            "PlaceMoneyTopRight",
            "DisplayMoneyAndCoinBalance",
            "DisplayCoinCaseBalance",
            "GSHealings",
            "StubbedTrainerRankings_Healings",
            "Reset",
            "HoOhChamber",
        ]
        .into_iter()
        .filter(|routine| special.special_routines.contains_key(*routine))
        .count(),
        [
            "CheckCaughtCelebi",
            "CelebiShrineEvent",
            "SnorlaxAwake",
            "CheckForBattleTowerRules",
        ]
        .into_iter()
        .filter(|routine| special.special_routines.contains_key(*routine))
        .count(),
        ["DayCareManOutside", "DayCareMon1", "DayCareMon2"]
            .into_iter()
            .filter(|routine| special.special_routines.contains_key(*routine))
            .count(),
        [
            "UnusedDummySpecial",
            "UnusedBattleTowerDummySpecial1",
            "UnusedBattleTowerDummySpecial2",
        ]
        .into_iter()
        .filter(|routine| special.special_routines.contains_key(*routine))
        .count()
    ));
    lines.push(format!(
        "special groups graphics={} party={} phone={} item_check={} fishing={}",
        [
            "ClearBGPalettesBufferScreen",
            "ClearBGPalettes",
            "UpdateTimePals",
            "ClearTilemap",
            "LoadMapPalettes",
            "RefreshSprites",
            "UpdateSprites",
            "ReloadSpritesNoPalettes",
            "FadeOutToWhite",
            "FadeInFromWhite",
            "FadeOutToBlack",
            "FadeInFromBlack",
            "GameboyCheck",
            "CheckMobileAdapterStatusSpecial",
            "BattleTowerFade",
            "UpdatePlayerSprite",
            "HealMachineAnim",
            "SurfStartStep",
            "LoadUsedSpritesGFX",
            "ToggleMaptileDecorations",
            "ToggleDecorationsVisibility",
            "MagnetTrain",
            "Diploma",
            "PrintDiploma",
            "UnownPuzzle",
            "OmanyteChamber",
            "DisplayUnownWords",
        ]
        .into_iter()
        .filter(|routine| special.special_routines.contains_key(*routine))
        .count(),
        [
            "CheckFirstMonIsEgg",
            "GetFirstPokemonHappiness",
            "FindPartyMonThatSpecies",
            "FindPartyMonAboveLevel",
            "FindPartyMonAtLeastThatHappy",
            "FindPartyMonThatSpeciesYourTrainerID",
            "MonCheck",
            "BeastsCheck",
            "GameCornerPrizeMonCheckDex",
            "UnusedSetSeenMon",
        ]
        .into_iter()
        .filter(|routine| special.special_routines.contains_key(*routine))
        .count(),
        [
            "RandomUnseenWildMon",
            "RandomPhoneWildMon",
            "RandomPhoneMon",
        ]
        .into_iter()
        .filter(|routine| special.special_routines.contains_key(*routine))
        .count(),
        special
            .special_routines
            .contains_key("UnusedFindItemInPCOrBag"),
        special
            .special_routines
            .contains_key("ActivateFishingSwarm")
    ));
    lines.push(format!(
        "kurt={} oak_ratings={} odd_eggs={} magikarp_lengths={} flee_mons={:?}",
        special.kurt_apricorn_recipes.len(),
        special.oak_ratings.len(),
        special.odd_egg_definitions.len(),
        special.magikarp_lengths.len(),
        special.flee_mons
    ));
    for routine in special.special_routines.keys().take(8) {
        lines.push(format!("routine {routine}"));
    }
    for (key, text) in snapshot.presentation.pc_strings.iter().take(8) {
        lines.push(format!("pc_string {key}={text}"));
    }
    for trade in special.npc_trades.keys().take(8) {
        lines.push(format!("npc_trade {trade}"));
    }
    for species in special.roaming_pokemon.keys().take(8) {
        lines.push(format!("roaming {species}"));
    }
}

fn format_warp_event_detail_line(warp: &crate::core::map::WarpEvent) -> String {
    match warp_tile_position_checked(warp) {
        Some(tile) => format!(
            "warp {} runtime_tile=({}, {}) raw=({}, {}) target={} target_warp={}",
            warp.index, tile.x, tile.y, warp.x, warp.y, warp.target_map, warp.target_warp_id
        ),
        None => format!(
            "warp {} runtime_tile=<invalid> raw=({}, {}) target={} target_warp={}",
            warp.index, warp.x, warp.y, warp.target_map, warp.target_warp_id
        ),
    }
}

fn format_visible_object_detail_line(object: &crate::core::map::ObjectEvent) -> String {
    match object_tile_position_checked(object) {
        Some(tile) => format!(
            "visible_object {:?} sprite={} runtime_tile=({}, {}) raw=({}, {}) script={} flag={}",
            object.object_identifier,
            object.sprite,
            tile.x,
            tile.y,
            object.x,
            object.y,
            object.script,
            object.event_flag
        ),
        None => format!(
            "visible_object {:?} sprite={} runtime_tile=<invalid> raw=({}, {}) script={} flag={}",
            object.object_identifier,
            object.sprite,
            object.x,
            object.y,
            object.script,
            object.event_flag
        ),
    }
}
