// Browser tools feed the same joypad resource as physical keyboard input.
// The page never receives a mutable game session or a save-editing interface.
#[derive(serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum WebMcpCommand {
    Observe {},
    Multiplayer { interaction: String },
    Press { button: String, frames: u32 },
}

struct WebMcpPending {
    id: u32,
    command: WebMcpCommand,
    key: Option<KeyCode>,
    started: Option<u64>,
    released: Option<u64>,
    canceled: bool,
}

#[derive(Default)]
struct WebMcpBridge {
    sequence: u32,
    pending: Option<WebMcpPending>,
    result: Option<(u32, String)>,
}

thread_local! {
    static WEBMCP_BRIDGE: std::cell::RefCell<WebMcpBridge> = std::cell::RefCell::new(WebMcpBridge::default());
}

fn webmcp_key(button: &str) -> Option<KeyCode> {
    Some(match button {
        "up" => KeyCode::ArrowUp,
        "down" => KeyCode::ArrowDown,
        "left" => KeyCode::ArrowLeft,
        "right" => KeyCode::ArrowRight,
        "a" => KeyCode::KeyZ,
        "b" => KeyCode::KeyX,
        "start" => KeyCode::Enter,
        "select" => KeyCode::ShiftRight,
        _ => return None,
    })
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub fn crystal_webmcp_request(json: &str) -> std::result::Result<u32, String> {
    if json.len() > 256 {
        return Err("Tool input is too large".into());
    }
    let command: WebMcpCommand = serde_json::from_str(json).map_err(|e| e.to_string())?;
    let key = match &command {
        WebMcpCommand::Observe {} => None,
        WebMcpCommand::Multiplayer { interaction } => Some(match interaction.as_str() {
            "battle" => KeyCode::KeyC,
            "trade" => KeyCode::KeyV,
            "time_capsule" => KeyCode::KeyT,
            _ => return Err("Unknown multiplayer interaction".into()),
        }),
        WebMcpCommand::Press { button, frames } => {
            if !(1..=60).contains(frames) {
                return Err("frames must be between 1 and 60".into());
            }
            Some(webmcp_key(button).ok_or("Unknown Game Boy button")?)
        }
    };
    WEBMCP_BRIDGE.with_borrow_mut(|bridge| {
        if bridge.pending.is_some() {
            return Err("Another game tool is executing".into());
        }
        bridge.sequence = bridge.sequence.wrapping_add(1).max(1);
        let id = bridge.sequence;
        bridge.result = None;
        bridge.pending = Some(WebMcpPending {
            id,
            command,
            key,
            started: None,
            released: None,
            canceled: false,
        });
        Ok(id)
    })
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub fn crystal_webmcp_poll(id: u32) -> Option<String> {
    WEBMCP_BRIDGE.with_borrow(|bridge| {
        bridge
            .result
            .as_ref()
            .filter(|(result_id, _)| *result_id == id)
            .map(|(_, result)| result.clone())
    })
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub fn crystal_webmcp_cancel(id: u32) {
    WEBMCP_BRIDGE.with_borrow_mut(|bridge| {
        if let Some(pending) = bridge.pending.as_mut().filter(|pending| pending.id == id) {
            pending.canceled = true;
        }
    });
}

fn apply_webmcp_input(
    runtime: Res<BevyRuntimeShell>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    _main_thread: Option<NonSend<MultiplayerRuntime>>,
) {
    WEBMCP_BRIDGE.with_borrow_mut(|bridge| {
        let Some(pending) = bridge.pending.as_mut() else {
            return;
        };
        let Some(key) = pending.key else {
            return;
        };
        let frame = runtime.lcd_animation_frame;
        let frames = match pending.command {
            WebMcpCommand::Press { frames, .. } => frames,
            WebMcpCommand::Multiplayer { .. } => 1,
            WebMcpCommand::Observe {} => return,
        };
        if pending.canceled
            || pending
                .started
                .is_some_and(|start| frame.saturating_sub(start) >= u64::from(frames))
        {
            if pending.released.is_none() {
                if pending.started.is_some() {
                    keys.release(key);
                }
                pending.released = Some(frame);
            }
        } else if pending.started.is_none() {
            // Refuse to take ownership of a key the human is already holding.
            if keys.get_pressed().next().is_some() {
                pending.canceled = true;
                return;
            }
            pending.started = Some(frame);
            keys.press(key);
        }
    });
}

fn webmcp_observation(
    runtime: &BevyRuntimeShell,
    multiplayer: Option<&MultiplayerRuntime>,
) -> Result<serde_json::Value> {
    let snapshot = runtime.shell.snapshot()?;
    let (screen, text) = if runtime.intro_screen.is_some() {
        ("intro", format_intro_dialog(runtime))
    } else if runtime.title_menu.is_some() {
        ("title", format_title_dialog(runtime))
    } else if runtime.pending_time_set.is_some() {
        ("clock", format_time_set_dialog_overlay(runtime))
    } else if runtime.pending_oak_intro.is_some() {
        ("introduction", format_oak_intro_dialog_overlay(runtime))
    } else if runtime.pending_gender_selection.is_some() {
        ("gender", format_gender_dialog(runtime))
    } else if runtime.credits_screen.is_some() {
        ("credits", format_credits_dialog(runtime))
    } else {
        (
            if snapshot.battle.is_some() {
                "battle"
            } else if runtime.pending_name_choice.is_some() || runtime.pending_name_input.is_some()
            {
                "naming"
            } else {
                "overworld"
            },
            format_dialog_overlay(&snapshot, runtime),
        )
    };
    let objects: Vec<_> = snapshot
        .visible_object_runtime_tiles
        .iter()
        .map(|(name, tile)| serde_json::json!({"name": name, "x": tile.x, "y": tile.y}))
        .collect();
    let players: Vec<_> = multiplayer.map(|multiplayer| multiplayer.remote_presences.values().filter(|presence| presence.map == snapshot.overworld.map_name).map(|presence| serde_json::json!({"name": presence.display_name, "x": presence.tile_x, "y": presence.tile_y, "facing": presence.direction})).collect()).unwrap_or_default();
    let mut menus = Vec::new();
    if let Some(choice) = &runtime.pending_name_choice {
        menus.push(serde_json::json!({"kind": "name_choices", "options": choice.options, "selected": choice.selected}));
    }
    if let Some(input) = &runtime.pending_name_input {
        menus.push(serde_json::json!({"kind": "name_input", "label": input.label, "value": input.value, "max_length": input.max_length, "keyboard": visible_name_input_layout(input.case), "cursor_column": input.cursor_column, "cursor_row": input.cursor_row}));
    }
    if runtime.start_menu_cursor.is_some() {
        menus.push(
            serde_json::json!({"kind": "start", "entries": visible_start_menu_entries(runtime)?}),
        );
    }
    if runtime.party_menu_open {
        menus.push(serde_json::json!({"kind": "party", "entries": visible_party_menu_entries(&snapshot, runtime)?}));
    }
    if runtime.field_pack_pocket.is_some() {
        menus.push(serde_json::json!({"kind": "pack", "entries": visible_field_pack_entries(&snapshot, runtime)?}));
    }
    if runtime.pokedex_menu_open {
        menus.push(serde_json::json!({"kind": "pokedex", "entries": visible_pokedex_menu_entries(&snapshot, runtime)?}));
    }
    if runtime.pokegear_menu_open {
        menus.push(serde_json::json!({"kind": "pokegear", "entries": visible_pokegear_menu_entries(&snapshot, runtime)?}));
    }
    if let Some(battle) = &snapshot.battle
        && runtime.battle_messages.is_empty()
        && !visible_battle_command_animation_active(runtime)
    {
        menus.push(serde_json::json!({"kind": "battle", "entries": visible_battle_command_menu_entries(&snapshot, runtime, battle)?}));
    }
    let mut terrain = Vec::new();
    if screen == "overworld" {
        let tileset_name = runtime
            .runtime
            .data()
            .map_tileset_name(&snapshot.overworld.map_name)?;
        let collision = runtime.runtime.data().tileset_collision(&tileset_name)?;
        let map = &runtime.shell.session().overworld().map;
        for y in snapshot.overworld.tile.y.saturating_sub(6)
            ..=snapshot.overworld.tile.y.saturating_add(6)
        {
            let mut row = Vec::new();
            for x in snapshot.overworld.tile.x.saturating_sub(6)
                ..=snapshot.overworld.tile.x.saturating_add(6)
            {
                row.push(crate::core::world::collision::sample_collision(map, &collision, TilePosition::new(x, y)).map(|sample| {
                    let attributes = crate::core::world::collision::describe_collision(sample.permission);
                    serde_json::json!({"terrain": format!("{:?}", attributes.terrain), "permission": sample.permission})
                }));
            }
            terrain.push(row);
        }
    }
    Ok(serde_json::json!({
        "frame": runtime.lcd_animation_frame,
        "status": {"screen": screen, "player_name": snapshot.trainer.player_name, "money": snapshot.trainer.money, "badges": snapshot.progression.badges, "party": snapshot.party.slots.iter().map(|slot| { let p = &slot.pokemon; serde_json::json!({"slot": slot.index, "nickname": p.nickname, "level": p.level, "hp": p.hp, "max_hp": p.max_hp, "status": p.status, "item": p.item, "moves": p.moves, "is_egg": p.is_egg}) }).collect::<Vec<_>>()},
        "observe": {"text": text, "visible_dialogue": visible_field_dialog_text(&snapshot, runtime), "menus": menus, "battle_message": runtime.battle_messages.front(), "battle": format_battle_overlay(&snapshot, runtime)},
        "map_info": {"name": snapshot.overworld.map_name, "player": {"x": snapshot.overworld.tile.x, "y": snapshot.overworld.tile.y, "facing": format!("{:?}", snapshot.overworld.facing)}, "dimensions": runtime.runtime.data().saved_map_tile_bounds(&snapshot.overworld.map_name), "objects": objects, "players": players, "terrain": {"origin_x": snapshot.overworld.tile.x.saturating_sub(6), "origin_y": snapshot.overworld.tile.y.saturating_sub(6), "rows": terrain, "note": "Terrain classes describe the current map. Directional permissions, objects, movement mode and game rules still decide whether a move succeeds."}},
        "flow_state": {"animating": visible_noninteractive_field_animation_owns_input(runtime) || visible_battle_command_animation_active(runtime) || runtime.player_walk_frame_ticks > 0, "buttons": ["up", "down", "left", "right", "a", "b", "start", "select"]},
        "multiplayer": multiplayer.map(|m| serde_json::json!({"connected": m.connection.is_some() && !m.failed, "session_active": m.session.is_some(), "pending_request": m.pending_interaction.as_ref().map(|request| serde_json::json!({"player": request.from_display_name, "kind": format!("{:?}", request.kind), "accept": "a", "decline": "b"}))})),
        "recent_events": {"last_action": runtime.last_action_status, "error": runtime.last_error}
    }))
}

fn finish_webmcp_request(
    runtime: Res<BevyRuntimeShell>,
    multiplayer: Option<NonSend<MultiplayerRuntime>>,
    art: Res<RenderedTilesetArt>,
    glyphs: Query<(&Handle<Image>, &Transform, &Visibility)>,
) {
    WEBMCP_BRIDGE.with_borrow_mut(|bridge| {
        let Some(pending) = bridge.pending.as_ref() else { return; };
        let ready = match pending.command {
            WebMcpCommand::Observe {} => true,
            WebMcpCommand::Press { .. } | WebMcpCommand::Multiplayer { .. } => pending.released.is_some_and(|frame| pending.canceled || runtime.lcd_animation_frame.saturating_sub(frame) >= 12),
        };
        if !ready { return; }
        let result = if pending.canceled { serde_json::json!({"error": "Action canceled; the button was released. Already processed input is not undone."}) }
            else { match webmcp_observation(&runtime, multiplayer.as_deref()) { Ok(mut value) => {
                    let mut letters = Vec::new();
                    if let Some(font) = art.font_cache.as_ref() {
                        for (texture, transform, visibility) in &glyphs {
                            if *visibility == Visibility::Hidden { continue; }
                            if let Some((character, _)) = font.glyphs.iter().find(|(_, frame)| frame.handle == *texture) {
                                letters.push((transform.translation.y, transform.translation.x, *character));
                            }
                        }
                    }
                    letters.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.total_cmp(&b.1)));
                    let mut lines = Vec::<String>::new();
                    let mut previous_y = None;
                    for (y, _, character) in letters {
                        if previous_y != Some(y) { lines.push(String::new()); previous_y = Some(y); }
                        if let Some(line) = lines.last_mut() { line.push(character); }
                    }
                    value["observe"]["rendered_text"] = serde_json::json!(lines);
                    value
                }, Err(error) => serde_json::json!({"error": error.to_string()}) } };
        bridge.result = Some((pending.id, result.to_string()));
        bridge.pending = None;
    });
}
