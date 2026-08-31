#[cfg(test)]
fn connection_trigger_tile(
    context: &MapPlayabilityContext,
    connection: &MapConnection,
) -> Option<TilePosition> {
    let source = connection_source_tile(context, connection)?;
    connection_trigger_tile_from_source(source, connection)
}

fn verify_maps(
    _asset_root: &AssetRoot,
    data: &GameDataSet,
    map_names: &BTreeSet<String>,
    rules: &PlayabilityRules,
    diagnostics: &mut Vec<VerificationError>,
) -> PlayabilityGraph {
    let constants = map_constants(data);
    let mut context_cache: BTreeMap<String, Option<Rc<MapPlayabilityContext>>> = BTreeMap::new();
    let mut graph = PlayabilityGraph::default();
    for map_name in map_names {
        let Some(module) = validation_map_for_playability(data, map_name, rules, diagnostics)
        else {
            continue;
        };
        let context = cached_map_playability_context_for_map(
            &mut context_cache,
            data,
            map_name,
            rules,
            diagnostics,
        );
        if let Some(context) = &context {
            graph
                .components
                .insert(map_name.clone(), context.component_count);
            for start in rules
                .start_tiles
                .iter()
                .filter(|start| start.map == *map_name)
            {
                let start_tile = start.tile;
                if let Some(component) = context.component_at(start_tile) {
                    graph.start_states.push((map_name.clone(), component));
                } else {
                    diagnostics.push(VerificationError::error(
                        "invalid_start_tile",
                        map_name,
                        format!(
                            "start tile ({}, {}) is not walkable under map collision",
                            start_tile.x, start_tile.y
                        ),
                    ));
                }
            }
        }
        let expected_blocks = module.attributes.width as usize * module.attributes.height as usize;
        if expected_blocks == 0 {
            diagnostics.push(VerificationError::error(
                "empty_map_dimensions",
                map_name,
                "map width and height must both be greater than zero",
            ));
        }
        if module.blocks.len() != expected_blocks {
            diagnostics.push(VerificationError::error(
                "wrong_map_block_count",
                map_name,
                format!(
                    "map has {} blocks but dimensions require {expected_blocks}",
                    module.blocks.len()
                ),
            ));
        }
        if rules.require_walkable_maps {
            verify_walkable_map(
                &module.id,
                context.as_ref().map(|context| context.as_ref()),
                rules,
                diagnostics,
            );
        }
        verify_warp_tiles_are_walkable(
            map_name,
            &module,
            context.as_ref().map(|context| context.as_ref()),
            diagnostics,
        );
        verify_coord_event_tiles_are_walkable(
            map_name,
            &module,
            context.as_ref().map(|context| context.as_ref()),
            diagnostics,
        );
        verify_object_tiles_are_walkable(
            map_name,
            &module,
            context.as_ref().map(|context| context.as_ref()),
            diagnostics,
        );
        let mut connection_directions = BTreeSet::new();
        for connection in &module.attributes.connections {
            if !is_exact_map_connection_direction(&connection.direction) {
                diagnostics.push(VerificationError::error(
                    "invalid_connection_direction",
                    map_name,
                    format!(
                        "connection direction must be one of north, south, west, east; found {:?}",
                        connection.direction
                    ),
                ));
                continue;
            }
            if !connection_directions.insert(connection.direction.as_str()) {
                diagnostics.push(VerificationError::error(
                    "duplicate_connection_direction",
                    map_name,
                    format!(
                        "connection direction '{}' must be unique for runtime lookup",
                        connection.direction
                    ),
                ));
            }
            if !is_exact_map_reference_token(&connection.target_map) {
                diagnostics.push(VerificationError::error(
                    "invalid_connection_target",
                    map_name,
                    format!(
                        "connection target map id must be exact, found {:?}",
                        connection.target_map
                    ),
                ));
                continue;
            }
            if !map_names.contains(&connection.target_map) {
                diagnostics.push(VerificationError::error(
                    "unknown_connection_target",
                    map_name,
                    format!(
                        "connection references missing map '{}'",
                        connection.target_map
                    ),
                ));
                continue;
            }
            let Some(source_context) = context.as_ref() else {
                continue;
            };
            let Some(target_module) = data.maps.get(&connection.target_map) else {
                diagnostics.push(map_validation_diagnostic(
                    rules,
                    "unassemblable_map",
                    &connection.target_map,
                    format!("missing compiled map module for {}", connection.target_map),
                ));
                continue;
            };
            let Some((source_tile, source_component)) =
                connection_source_tile_and_component_for_target(
                    source_context,
                    connection,
                    &target_module.attributes,
                )
            else {
                if connection_source_tile(source_context, connection).is_none() {
                    diagnostics.push(transition_diagnostic(
                        rules,
                        "unreachable_connection",
                        map_name,
                        format!(
                            "connection to '{}' has no reachable walkable border tile",
                            connection.target_map
                        ),
                    ));
                }
                continue;
            };
            let Some(trigger_tile) = connection_trigger_tile_from_source(source_tile, connection)
            else {
                diagnostics.push(transition_diagnostic(
                    rules,
                    "unreachable_connection",
                    map_name,
                    format!(
                        "connection to '{}' has no reachable trigger tile",
                        connection.target_map
                    ),
                ));
                continue;
            };
            let destination_tile = match connection_destination_tile(
                trigger_tile,
                &connection.direction,
                connection.offset,
                &target_module.attributes,
            ) {
                Ok(tile) => tile,
                Err(error) => {
                    diagnostics.push(transition_diagnostic(
                        rules,
                        "invalid_connection_transition",
                        map_name,
                        error.to_string(),
                    ));
                    continue;
                }
            };
            let target_context = cached_map_playability_context_for_map(
                &mut context_cache,
                data,
                &connection.target_map,
                rules,
                diagnostics,
            );
            let Some(target_component) = target_context
                .as_ref()
                .and_then(|context| context.component_at(destination_tile))
            else {
                diagnostics.push(transition_diagnostic(
                    rules,
                    "unreachable_connection_destination",
                    map_name,
                    format!(
                        "connection to '{}' lands on an unwalkable tile",
                        connection.target_map
                    ),
                ));
                continue;
            };
            graph.edges.push(ComponentGraphEdge {
                from_map: map_name.clone(),
                from_component: source_component,
                to_map: connection.target_map.clone(),
                to_component: target_component,
                kind: "connection".to_string(),
            });
        }
        let mut warp_indices = BTreeSet::new();
        let mut warp_tiles = BTreeSet::new();
        for warp in &module.events.warps {
            if !warp_indices.insert(warp.index) {
                diagnostics.push(VerificationError::error(
                    "duplicate_warp_index",
                    map_name,
                    format!(
                        "warp index {} must be unique for runtime lookup",
                        warp.index
                    ),
                ));
            }
            if let Some(warp_tile) = checked_runtime_map_event_tile(warp.x, warp.y) {
                if !warp_tiles.insert((warp_tile.x, warp_tile.y)) {
                    diagnostics.push(VerificationError::error(
                        "duplicate_warp_tile",
                        map_name,
                        format!(
                            "warp runtime tile {},{} must be unique for runtime lookup",
                            warp_tile.x, warp_tile.y
                        ),
                    ));
                }
            }
            if !is_exact_map_reference_token(&warp.target_map) {
                diagnostics.push(VerificationError::error(
                    "invalid_warp_target_map",
                    map_name,
                    format!(
                        "warp {} target map field must be exact, found {:?}",
                        warp.index, warp.target_map
                    ),
                ));
                continue;
            }
            if warp.target_map != warp.target_map_constant {
                diagnostics.push(VerificationError::error(
                    "warp_target_map_mismatch",
                    map_name,
                    format!(
                        "warp {} target_map {:?} does not match target_map_constant {:?}",
                        warp.index, warp.target_map, warp.target_map_constant
                    ),
                ));
                continue;
            }
            if !is_exact_map_reference_token(&warp.target_map_constant) {
                diagnostics.push(VerificationError::error(
                    "invalid_warp_target",
                    map_name,
                    format!(
                        "warp {} target map constant must be exact, found {:?}",
                        warp.index, warp.target_map_constant
                    ),
                ));
                continue;
            }
            let Some(target_map) = constants.get(&warp.target_map_constant) else {
                diagnostics.push(VerificationError::error(
                    "unknown_warp_target",
                    map_name,
                    format!(
                        "warp {} references unknown map constant '{}'",
                        warp.index, warp.target_map_constant
                    ),
                ));
                continue;
            };
            if warp.target_warp_id < 1 {
                continue;
            }
            let Some(target_warp) =
                target_warp(data, target_map, warp.target_warp_id, rules, diagnostics)
            else {
                diagnostics.push(transition_diagnostic(
                    rules,
                    "unknown_warp_index",
                    map_name,
                    format!(
                        "warp {} targets missing warp id {} on {}",
                        warp.index, warp.target_warp_id, target_map
                    ),
                ));
                continue;
            };
            let Some(source_tile) = checked_runtime_map_event_tile(warp.x, warp.y) else {
                diagnostics.push(transition_diagnostic(
                    rules,
                    "map_event_runtime_position_overflow",
                    map_name,
                    format!(
                        "warp {} coordinate ({}, {}) overflows runtime tile coordinates",
                        warp.index, warp.x, warp.y
                    ),
                ));
                continue;
            };
            let Some(source_component) = context
                .as_ref()
                .and_then(|context| context.component_at(source_tile))
            else {
                diagnostics.push(transition_diagnostic(
                    rules,
                    "unreachable_warp",
                    map_name,
                    format!("warp {} is not on a reachable walkable tile", warp.index),
                ));
                continue;
            };
            let target_context = cached_map_playability_context_for_map(
                &mut context_cache,
                data,
                target_map,
                rules,
                diagnostics,
            );
            let Some(target_tile) = checked_runtime_map_event_tile(target_warp.x, target_warp.y)
            else {
                diagnostics.push(transition_diagnostic(
                    rules,
                    "map_event_runtime_position_overflow",
                    target_map,
                    format!(
                        "target warp {} coordinate ({}, {}) overflows runtime tile coordinates",
                        target_warp.index, target_warp.x, target_warp.y
                    ),
                ));
                continue;
            };
            let Some(target_component) = target_context
                .as_ref()
                .and_then(|context| context.component_at(target_tile))
            else {
                diagnostics.push(transition_diagnostic(
                    rules,
                    "unreachable_warp_destination",
                    map_name,
                    format!(
                        "warp {} lands on an unwalkable tile on {}",
                        warp.index, target_map
                    ),
                ));
                continue;
            };
            graph.edges.push(ComponentGraphEdge {
                from_map: map_name.clone(),
                from_component: source_component,
                to_map: target_map.clone(),
                to_component: target_component,
                kind: "warp".to_string(),
            });
        }
    }
    graph
}

fn transition_diagnostic(
    rules: &PlayabilityRules,
    code: impl Into<String>,
    map_name: &str,
    message: impl Into<String>,
) -> VerificationError {
    if rules.require_all_maps_reachable || playability_mentions_map(rules, map_name) {
        VerificationError::error(code, map_name, message)
    } else {
        VerificationError::warning(code, map_name, message)
    }
}

fn is_exact_map_reference_token(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn validate_map_reference_token(value: &str, description: &str) -> Result<()> {
    if !is_exact_map_reference_token(value) {
        anyhow::bail!("{description} '{value}' must be an exact map token");
    }
    validate_no_reserved_payload_token(value, description)?;
    Ok(())
}

fn is_exact_map_connection_direction(value: &str) -> bool {
    matches!(value, "north" | "south" | "west" | "east")
}

fn map_validation_diagnostic(
    rules: &PlayabilityRules,
    code: impl Into<String>,
    map_name: &str,
    message: impl Into<String>,
) -> VerificationError {
    if rules.require_all_maps_reachable || playability_mentions_map(rules, map_name) {
        VerificationError::error(code, map_name, message)
    } else {
        VerificationError::warning(code, map_name, message)
    }
}

fn playability_mentions_map(rules: &PlayabilityRules, map_name: &str) -> bool {
    rules.start_maps.iter().any(|map| map == map_name)
        || rules.start_tiles.iter().any(|start| start.map == map_name)
        || rules.goal_maps.iter().any(|map| map == map_name)
        || rules.map_access.iter().any(|access| access.map == map_name)
}

fn verify_walkable_map(
    map_name: &str,
    context: Option<&MapPlayabilityContext>,
    rules: &PlayabilityRules,
    diagnostics: &mut Vec<VerificationError>,
) {
    if context
        .map(|context| context.component_count == 0)
        .unwrap_or(true)
    {
        diagnostics.push(map_validation_diagnostic(
            rules,
            "unwalkable_map",
            map_name,
            "map has no walkable tile under its tileset collision",
        ));
    }
}

fn verify_warp_tiles_are_walkable(
    map_name: &str,
    module: &ValidationMapModule,
    context: Option<&MapPlayabilityContext>,
    diagnostics: &mut Vec<VerificationError>,
) {
    let Some(context) = context else {
        return;
    };
    for warp in &module.events.warps {
        let Some(tile) = checked_runtime_map_event_tile(warp.x, warp.y) else {
            diagnostics.push(VerificationError::error(
                "map_event_runtime_position_overflow",
                format!("{map_name}:warp:{}:{},{}", warp.index, warp.x, warp.y),
                format!(
                    "warp {} coordinate ({}, {}) overflows runtime tile coordinates",
                    warp.index, warp.x, warp.y
                ),
            ));
            continue;
        };
        let _ = (context, tile);
    }
}

fn verify_coord_event_tiles_are_walkable(
    map_name: &str,
    module: &ValidationMapModule,
    context: Option<&MapPlayabilityContext>,
    diagnostics: &mut Vec<VerificationError>,
) {
    let Some(context) = context else {
        return;
    };
    for event in &module.events.coord_events {
        let Some(tile) = checked_runtime_map_event_tile(event.x, event.y) else {
            diagnostics.push(VerificationError::error(
                "map_event_runtime_position_overflow",
                format!(
                    "{map_name}:coord:{}:{}:{},{}",
                    event.scene_id, event.script_name, event.x, event.y
                ),
                format!(
                    "coord event '{}' coordinate ({}, {}) overflows runtime tile coordinates",
                    event.script_name, event.x, event.y
                ),
            ));
            continue;
        };
        let _ = (context, tile);
    }
}

fn verify_object_tiles_are_walkable(
    map_name: &str,
    module: &ValidationMapModule,
    context: Option<&MapPlayabilityContext>,
    diagnostics: &mut Vec<VerificationError>,
) {
    let Some(context) = context else {
        return;
    };
    for object in &module.objects {
        let Some(tile) = checked_runtime_map_event_tile(object.x, object.y) else {
            diagnostics.push(VerificationError::error(
                "map_event_runtime_position_overflow",
                format!(
                    "{map_name}:object:{}:{},{}",
                    object
                        .object_identifier
                        .as_deref()
                        .unwrap_or(&object.script),
                    object.x,
                    object.y
                ),
                format!(
                    "object '{}' coordinate ({}, {}) overflows runtime tile coordinates",
                    object
                        .object_identifier
                        .as_deref()
                        .unwrap_or(&object.script),
                    object.x,
                    object.y
                ),
            ));
            continue;
        };
        let _ = (context, tile);
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct ProgressionState {
    maps: BTreeSet<String>,
    events: BTreeSet<String>,
    items: BTreeSet<String>,
}

fn solve_progression(
    reachable_maps: &[String],
    loaded_maps: &BTreeSet<String>,
    rules: &PlayabilityRules,
) -> ProgressionState {
    let physical_maps: BTreeSet<String> = reachable_maps.iter().cloned().collect();
    let mut state = ProgressionState {
        events: rules.initial_events.iter().cloned().collect(),
        items: rules.initial_items.iter().cloned().collect(),
        ..ProgressionState::default()
    };
    let mut applied_rules = BTreeSet::new();
    loop {
        let mut changed = false;
        for map in &physical_maps {
            if !state.maps.contains(map) && map_accessible(map, &state, rules) {
                state.maps.insert(map.clone());
                changed = true;
            }
        }
        for rule in &rules.progression_rules {
            if applied_rules.contains(&rule.id) || !requirements_met(&rule.requires, &state) {
                continue;
            }
            applied_rules.insert(rule.id.clone());
            for event in &rule.grants.events {
                changed |= state.events.insert(event.clone());
            }
            for item in &rule.grants.items {
                changed |= state.items.insert(item.clone());
            }
            for map in &rule.grants.maps {
                if loaded_maps.contains(map) {
                    changed |= state.maps.insert(map.clone());
                }
            }
        }
        if !changed {
            break;
        }
    }
    state
}

fn map_accessible(map: &str, state: &ProgressionState, rules: &PlayabilityRules) -> bool {
    rules
        .map_access
        .iter()
        .filter(|rule| rule.map == map)
        .all(|rule| requirements_met(&rule.requires, state))
}

fn requirements_met(requirements: &ProgressionRequirements, state: &ProgressionState) -> bool {
    requirements
        .events
        .iter()
        .all(|event| state.events.contains(event))
        && requirements
            .items
            .iter()
            .all(|item| state.items.contains(item))
        && requirements.maps.iter().all(|map| state.maps.contains(map))
}

fn verify_solubility(
    map_names: &BTreeSet<String>,
    reachable_maps: &[String],
    progression: &ProgressionState,
    loaded_progression: &ProgressionState,
    rules: &PlayabilityRules,
    diagnostics: &mut Vec<VerificationError>,
) {
    if rules.start_maps.is_empty()
        && rules.start_tiles.is_empty()
        && (!rules.goal_maps.is_empty()
            || !rules.goal_events.is_empty()
            || !rules.goal_items.is_empty()
            || rules.require_all_maps_reachable)
    {
        diagnostics.push(VerificationError::error(
            "missing_start_map",
            "playability",
            "playability rules require at least one explicit start map",
        ));
    }
    for start in &rules.start_maps {
        if !map_names.contains(start) {
            diagnostics.push(VerificationError::error(
                "unknown_start_map",
                start,
                "playability start map is not loaded",
            ));
        }
    }
    for start in &rules.start_tiles {
        if !map_names.contains(&start.map) {
            diagnostics.push(VerificationError::error(
                "unknown_start_map",
                &start.map,
                "playability start tile map is not loaded",
            ));
        }
    }
    let reachable: BTreeSet<&str> = reachable_maps.iter().map(String::as_str).collect();
    for goal in &rules.goal_maps {
        if !map_names.contains(goal) {
            diagnostics.push(VerificationError::error(
                "unknown_goal_map",
                goal,
                "playability goal map is not loaded",
            ));
        } else if !reachable.contains(goal.as_str()) {
            diagnostics.push(VerificationError::error(
                "unreachable_goal_map",
                goal,
                "playability goal map cannot be reached from the configured starts",
            ));
        } else if !progression.maps.contains(goal) {
            diagnostics.push(VerificationError::error(
                "unsolved_goal_map",
                goal,
                "playability goal map is physically reachable but blocked by progression rules",
            ));
        }
    }
    for goal in &rules.goal_events {
        if !loaded_progression.events.contains(goal) {
            diagnostics.push(VerificationError::error(
                "unsolved_goal_event",
                goal,
                "playability goal event cannot be produced by progression rules",
            ));
        }
    }
    for goal in &rules.goal_items {
        if !loaded_progression.items.contains(goal) {
            diagnostics.push(VerificationError::error(
                "unsolved_goal_item",
                goal,
                "playability goal item cannot be produced by progression rules",
            ));
        }
    }
    if rules.require_all_maps_reachable {
        for map_name in map_names {
            if !reachable.contains(map_name.as_str()) {
                diagnostics.push(VerificationError::error(
                    "unreachable_map",
                    map_name,
                    "map cannot be reached from the configured starts",
                ));
            } else if !progression.maps.contains(map_name) {
                diagnostics.push(VerificationError::error(
                    "unsolved_map",
                    map_name,
                    "map is physically reachable but blocked by progression rules",
                ));
            }
        }
    }
}

fn runtime_module_script_subset<'a>(
    all_scripts: &BTreeMap<String, Value>,
    seeds: impl IntoIterator<Item = &'a str>,
    follow_callasm_definitions: bool,
) -> BTreeMap<String, Value> {
    let mut scripts = BTreeMap::new();
    let mut pending: Vec<String> = seeds.into_iter().map(str::to_string).collect();
    while let Some(label) = pending.pop() {
        if scripts.contains_key(&label) {
            continue;
        }
        let Some(payload) = all_scripts.get(&label) else {
            continue;
        };
        scripts.insert(label.clone(), payload.clone());
        for reference in
            script_payload_references(&label, payload, all_scripts, follow_callasm_definitions)
        {
            if !scripts.contains_key(&reference) {
                pending.push(reference);
            }
        }
    }
    scripts
}

fn script_payload_references(
    current_label: &str,
    payload: &Value,
    all_scripts: &BTreeMap<String, Value>,
    follow_callasm_definitions: bool,
) -> Vec<String> {
    let Some(commands) = payload.as_array() else {
        return Vec::new();
    };
    let mut references = Vec::new();
    for command in commands {
        let Some(command_name) = command.get("command").and_then(Value::as_str) else {
            continue;
        };
        if command_name == "callasm" && !follow_callasm_definitions {
            continue;
        }
        if matches!(
            command_name,
            "adc"
                | "add"
                | "and"
                | "bit"
                | "call"
                | "ccf"
                | "cp"
                | "cpl"
                | "daa"
                | "dec"
                | "di"
                | "ei"
                | "farcall"
                | "inc"
                | "jp"
                | "jr"
                | "ld"
                | "ldh"
                | "nop"
                | "or"
                | "pop"
                | "push"
                | "res"
                | "ret"
                | "reti"
                | "rl"
                | "rla"
                | "rlc"
                | "rlca"
                | "rr"
                | "rra"
                | "rrc"
                | "rrca"
                | "rst"
                | "sbc"
                | "scf"
                | "set"
                | "sla"
                | "sra"
                | "srl"
                | "sub"
                | "swap"
                | "xor"
        ) {
            continue;
        }
        let Some(args) = command.get("args").and_then(Value::as_array) else {
            continue;
        };
        for arg in args.iter().filter_map(Value::as_str) {
            if arg.starts_with('.') {
                let parent_label = script_label_parent(current_label);
                let scoped = if arg.contains('@') {
                    if script_label_parent(arg) != parent_label {
                        continue;
                    }
                    arg.to_string()
                } else {
                    format!("{arg}@{parent_label}")
                };
                if all_scripts.contains_key(&scoped) {
                    references.push(scoped);
                }
            } else if all_scripts.contains_key(arg) {
                references.push(arg.to_string());
            }
        }
    }
    references
}

fn map_constants(data: &GameDataSet) -> BTreeMap<String, String> {
    data.maps
        .iter()
        .filter_map(|(map_name, module)| {
            module
                .attributes
                .map_constant
                .as_ref()
                .map(|constant| (constant.clone(), map_name.clone()))
        })
        .collect()
}

fn validation_map_for_playability(
    data: &GameDataSet,
    map_name: &str,
    _rules: &PlayabilityRules,
    _diagnostics: &mut Vec<VerificationError>,
) -> Option<ValidationMapModule> {
    data.maps.get(map_name).map(|module| ValidationMapModule {
        id: module.id.clone(),
        attributes: module.attributes.clone(),
        events: module.events.clone(),
        objects: module.objects.clone(),
        blocks: module.blocks.clone(),
    })
}

fn cached_map_playability_context_for_map(
    cache: &mut BTreeMap<String, Option<Rc<MapPlayabilityContext>>>,
    data: &GameDataSet,
    map_name: &str,
    rules: &PlayabilityRules,
    diagnostics: &mut Vec<VerificationError>,
) -> Option<Rc<MapPlayabilityContext>> {
    if let Some(context) = cache.get(map_name) {
        return context.clone();
    }
    let context = data
        .maps
        .get(map_name)
        .and_then(|module| map_playability_context(data, module, rules, diagnostics))
        .map(Rc::new);
    cache.insert(map_name.to_string(), context.clone());
    context
}

fn target_warp(
    data: &GameDataSet,
    target_map: &str,
    target_warp_id: i16,
    rules: &PlayabilityRules,
    diagnostics: &mut Vec<VerificationError>,
) -> Option<WarpEvent> {
    if target_warp_id < 1 {
        return None;
    }
    let Some(module) = data.maps.get(target_map) else {
        diagnostics.push(map_validation_diagnostic(
            rules,
            "unassemblable_map",
            target_map,
            format!("missing compiled map module for {target_map}"),
        ));
        return None;
    };
    module
        .events
        .warps
        .get(target_warp_id as usize - 1)
        .cloned()
}

fn reachable_maps(
    map_names: &BTreeSet<String>,
    graph: &PlayabilityGraph,
    rules: &PlayabilityRules,
) -> Vec<String> {
    let mut adjacency: BTreeMap<(&str, usize), Vec<(&str, usize)>> = BTreeMap::new();
    for edge in &graph.edges {
        adjacency
            .entry((edge.from_map.as_str(), edge.from_component))
            .or_default()
            .push((edge.to_map.as_str(), edge.to_component));
    }

    let mut seen_states = BTreeSet::new();
    let mut queue = VecDeque::new();
    if rules.start_tiles.is_empty() {
        let start_maps: Vec<String> = rules
            .start_maps
            .iter()
            .filter(|map_name| map_names.contains(*map_name))
            .cloned()
            .collect();
        for map_name in start_maps {
            let component_count = graph.components.get(&map_name).copied().unwrap_or(0);
            for component in 0..component_count {
                queue.push_back((map_name.clone(), component));
            }
        }
    } else {
        for (map_name, component) in &graph.start_states {
            queue.push_back((map_name.clone(), *component));
        }
    }

    while let Some((map_name, component)) = queue.pop_front() {
        if !seen_states.insert((map_name.clone(), component)) {
            continue;
        }
        for (next_map, next_component) in adjacency
            .get(&(map_name.as_str(), component))
            .into_iter()
            .flatten()
        {
            let next = ((*next_map).to_string(), *next_component);
            if !seen_states.contains(&next) {
                queue.push_back(next);
            }
        }
    }

    seen_states
        .into_iter()
        .map(|(map_name, _)| map_name)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverworldInputFrame {
    pub snapshot: OverworldSnapshot,
    pub input_mask: u8,
    pub pressed_mask: u8,
    /// True when the frame-driven object scheduler changed a live NPC's
    /// runtime tile or facing.  The renderer must invalidate its snapshot
    /// even when the player supplied no movement input.
    #[serde(default)]
    pub autonomous_objects_changed: bool,
    pub movement: Option<StepOutcome>,
    #[serde(default)]
    pub ledge_jump: Option<LedgeJumpOutcome>,
    #[serde(default)]
    pub grass_rustle: Option<OverworldGrassRustle>,
    #[serde(default)]
    pub phone_call: Option<IncomingPhoneCall>,
    pub step_events: Option<StepEventResult>,
    pub coord_event: Option<CoordEventTrigger>,
    pub trainer_sight: Option<OverworldInteraction>,
    pub interaction: Option<OverworldInteraction>,
    pub warp: Option<WarpTransition>,
    pub connection: Option<ConnectionTransition>,
    pub wild_encounter: Option<WildEncounterRoll>,
    pub wild_battle: Option<WildBattleStart>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum IncomingPhoneCallKind {
    Ordinary,
    Special { call_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IncomingPhoneCall {
    pub kind: IncomingPhoneCallKind,
    pub contact_id: String,
    pub caller_script: String,
    pub receive_script: String,
    pub delay_frames: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverworldGrassRustle {
    pub tile: TilePosition,
    pub duration_frames: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlyDestination {
    pub flypoint_flag: String,
    pub destination_spawn_identifier: u16,
    pub label: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecorationSide {
    Right,
    Left,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum DecorationActionOutcome {
    SetUp {
        decoration: String,
    },
    Replaced {
        decoration: String,
        previous: String,
    },
    PutAway {
        decoration: String,
    },
    AlreadySetUp {
        decoration: String,
    },
    NothingToPutAway,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeDecorationSetupCommand {
    pub decoration_id: String,
    pub side: Option<DecorationSide>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeDecorationPutAwayCommand {
    pub category: DecorationCategory,
    pub side: Option<DecorationSide>,
}

impl DecorationActionOutcome {
    pub fn changed(&self) -> bool {
        matches!(
            self,
            Self::SetUp { .. } | Self::Replaced { .. } | Self::PutAway { .. }
        )
    }
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GameDataSet {
    #[serde(default, skip_serializing_if = "crate::NuzlockeRules::is_disabled")]
    pub nuzlocke_rules: crate::NuzlockeRules,
    pub pokemon: BTreeMap<String, PokemonSpecies>,
    pub moves: BTreeMap<String, Move>,
    pub growth_rates: crystal_core::systems::experience::GrowthRateCatalog,
    pub learnsets: SpeciesLearnsets,
    pub level_up_moves: BTreeMap<String, Value>,
    pub egg_moves: BTreeMap<String, Value>,
    pub evolutions: EvolutionTable,
    pub maps: BTreeMap<String, MapModule>,
    pub map_scripts: BTreeMap<String, Value>,
    pub map_attributes: BTreeMap<String, MapAttributes>,
    pub map_dimensions: BTreeMap<String, Value>,
    pub map_blocks: BTreeMap<String, String>,
    pub items: BTreeMap<String, Item>,
    pub marts: MartCatalog,
    pub currency_constants: CurrencyCatalog,
    pub battle_reward_rules: BattleRewardRules,
    pub battle_escape_rules: BattleEscapeRules,
    pub step_event_rules: StepEventRules,
    pub fishing: FishingCatalog,
    pub fruit_trees: FruitTreeCatalog,
    pub field_moves: FieldMoveCatalog,
    pub field_box_items: BTreeMap<String, FieldBoxItemRule>,
    pub decorations: DecorationCatalog,
    pub runtime_title_screen: RuntimeTitleScreen,
    pub fly_destinations: BTreeMap<String, FlyDestination>,
    pub runtime_spawn_points: BTreeMap<String, RuntimeSpawnPoint>,
    pub runtime_map_metadata: BTreeMap<String, RuntimeMapMetadata>,
    pub flee_mons: FleeMonTables,
    pub buena_password_categories: BuenaPasswordCategories,
    pub roaming_pokemon: RoamingPokemonCatalog,
    pub buena_prizes: BuenaPrizeDefinitions,
    pub kurt_apricorn_recipes: KurtApricornRecipes,
    #[serde(deserialize_with = "required_nullable_value")]
    pub shuckie_gift: Option<ShuckieGiftDefinition>,
    pub dratini_move_sets: DratiniMoveSets,
    #[serde(deserialize_with = "required_nullable_value")]
    pub bug_contest_config: Option<BugContestConfig>,
    #[serde(deserialize_with = "required_nullable_value")]
    pub battle_tower_rules: Option<BattleTowerRules>,
    pub oak_ratings: Vec<OakRatingEntry>,
    pub odd_egg_definitions: Vec<OddEggDefinition>,
    pub magikarp_lengths: Vec<MagikarpLengthEntry>,
    #[serde(deserialize_with = "required_nullable_value")]
    pub happiness_data: Option<HappinessData>,
    pub encounter_slot_tables: EncounterSlotTables,
    pub encounter_music_modifiers: EncounterMusicModifiers,
    pub battle_stat_multipliers: BattleStatMultiplierTables,
    pub capture_wobble_probabilities: Vec<CaptureWobbleProbability>,
    pub move_priorities: MovePriorityTable,
    pub type_categories: TypeCategories,
    pub type_effectiveness: TypeEffectivenessTable,
    pub weather_modifiers: WeatherModifiers,
    pub pc_strings: BTreeMap<String, String>,
    pub menu_icons: BTreeMap<String, String>,
    pub pokedex_entries: BTreeMap<String, RuntimePokedexEntry>,
    pub pokemon_frontpic_anim: BTreeMap<String, FrontpicAnimProgram>,
    pub initialize_events: InitializeEventsConfig,
    pub story_event_script_constants: StoryEventScriptConstants,
    pub asm_text: BTreeMap<String, String>,
    pub move_names: Vec<String>,
    pub battle_animations: BTreeMap<String, Vec<String>>,
    pub battle_animation_table: Vec<String>,
    pub battle_anim_bundle: String,
    pub sprite_anim_bundle: String,
    pub sprite_palette_defaults: BTreeMap<String, i64>,
    pub pokegear_town_map_palette_map: BTreeMap<String, Vec<String>>,
    pub pokemon_cries: BTreeMap<String, PokemonCryMetadata>,
    pub wild_encounters: BTreeMap<String, WildEncounterData>,
    pub field_encounters: BTreeMap<String, FieldEncounterData>,
    pub npcs: BTreeMap<String, Value>,
    pub pokegear_landmarks: PokegearLandmarksPayload,
    pub trainers: TrainerCatalog,
    pub trainer_class_names: BTreeMap<String, String>,
    pub pokedex: Vec<Value>,
    pub story_events: Vec<Value>,
    pub phone_scripts: Vec<Value>,
    #[serde(deserialize_with = "required_nullable_value")]
    pub global_scripts: Option<GlobalScriptModule>,
    pub phone_contacts: PhoneContactCatalog,
    pub permanent_phone_numbers: BTreeMap<String, PermanentPhoneNumberRule>,
    pub special_phone_calls: BTreeMap<String, SpecialPhoneCallRule>,
    pub npc_trades: BTreeMap<String, NpcTradeRule>,
    pub special_routines: BTreeMap<String, SpecialRoutineRule>,
    pub audio: Vec<ModpackAudioAsset>,
    pub capture_rules: CaptureRules,
    pub tilesets: BTreeMap<String, TilesetDefinition>,
    pub playability: PlayabilityRules,
}

fn tileset_declares_metatile(tileset: &TilesetDefinition, block_id: u16) -> bool {
    tileset
        .collision
        .keys()
        .filter_map(|metatile_id| u16::from_str_radix(metatile_id, 16).ok())
        .any(|metatile_id| metatile_id == block_id)
}

fn wild_encounter_data_has(encounters: &WildEncounterData, species: &str, level: u8) -> bool {
    encounters.grass.as_ref().is_some_and(|table| {
        wild_encounter_table_has_reachable_level(table, EncounterSurface::Grass, species, level)
    }) || encounters.water.as_ref().is_some_and(|table| {
        wild_encounter_table_has_reachable_level(table, EncounterSurface::Water, species, level)
    }) || encounters.swarm_overrides.values().any(|swarm| {
        wild_encounter_table_has_reachable_level(
            &swarm.grass,
            EncounterSurface::Grass,
            species,
            level,
        )
    })
}

fn wild_encounter_table_has_reachable_level(
    table: &WildEncounterTable,
    surface: EncounterSurface,
    species: &str,
    level: u8,
) -> bool {
    table
        .morning
        .iter()
        .chain(table.day.iter())
        .chain(table.night.iter())
        .any(|encounter| wild_encounter_has_reachable_level(encounter, surface, species, level))
}

fn wild_encounter_has_reachable_level(
    encounter: &WildEncounter,
    surface: EncounterSurface,
    species: &str,
    level: u8,
) -> bool {
    encounter.species == species
        && (0..=u8::MAX)
            .any(|roll| apply_surf_level_variance(encounter.level, surface, roll) == level)
}

fn field_encounter_data_has(encounters: &FieldEncounterData, species: &str, level: u8) -> bool {
    [
        encounters.table(FieldEncounterKind::Headbutt),
        encounters.table(FieldEncounterKind::RockSmash),
    ]
    .into_iter()
    .flatten()
    .flat_map(|table| table.common.iter().chain(table.rare.iter()))
    .any(|encounter| encounter.species == species && encounter.level == level)
}

fn fishing_slot_has(
    time_groups: &BTreeMap<String, crystal_core::world::fishing::TimeFishEntry>,
    slot: &crystal_core::world::fishing::FishingSlot,
    species: &str,
    level: u8,
) -> bool {
    if slot.species.as_deref() == Some(species) && slot.level == level {
        return true;
    }
    let Some(time_group) = slot
        .time_group
        .as_deref()
        .and_then(|time_group| time_groups.get(time_group))
    else {
        return false;
    };
    (time_group.day_species == species && time_group.day_level == level)
        || (time_group.night_species == species && time_group.night_level == level)
}

fn find_script_entry<'a, T>(
    entries: &'a [T],
    map_name: &str,
    entry_name: &str,
    source_script: &str,
    command_index: usize,
    key: impl Fn(&T) -> (&str, usize),
) -> Result<&'a T> {
    entries
        .iter()
        .find(|entry| {
            let (entry_source, entry_command_index) = key(entry);
            entry_source == source_script && entry_command_index == command_index
        })
        .with_context(|| {
            format!("map {map_name} has no {entry_name} at {source_script}:{command_index}")
        })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeScriptCommandRef {
    pub map_name: String,
    pub source_script: String,
    pub command_index: usize,
}

impl RuntimeScriptCommandRef {
    pub fn new(
        map_name: impl Into<String>,
        source_script: impl Into<String>,
        command_index: usize,
    ) -> Self {
        Self {
            map_name: map_name.into(),
            source_script: source_script.into(),
            command_index,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeItemCommand {
    pub item_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeMoveLearnReplacementCommand {
    pub move_slot: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeBagItemDeltaCommand {
    pub item_id: String,
    pub quantity: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimePartyItemCommand {
    pub item_id: String,
    pub party_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimePartyMoveItemCommand {
    pub item_id: String,
    pub party_index: usize,
    pub move_slot: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeTmHmCommand {
    pub item_id: String,
    pub party_index: usize,
    pub replace_slot: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeFieldBlockMoveCommand {
    pub party_index: usize,
    pub metatile_x: u16,
    pub metatile_y: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeFieldPartyCommand {
    pub party_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeHeadbuttScriptCommand {
    pub party_index: usize,
    pub from_menu: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeFlyCommand {
    pub party_index: usize,
    pub destination_spawn_identifier: u16,
    pub flypoint_flag: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeBattleTurnCommand {
    pub player_action: BattleAction,
    /// Bag inventory consumption paired atomically with a player Item action.
    /// The core turn owns the battle effect; this field owns only the bag use.
    pub player_bag_item_id: Option<String>,
    pub enemy_action: BattleAction,
    pub enemy_ai_divider_trace: RuntimeDividerTrace,
    pub enemy_ai_selected_move_slot: Option<usize>,
    pub enemy_move_ai_random_calls: u16,
    pub enemy_post_order_ai_random_calls: u16,
    pub divider_trace: RuntimeDividerTrace,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeBattleEnemyActionCommand {
    pub enemy_action: BattleAction,
    pub enemy_ai_divider_trace: RuntimeDividerTrace,
    pub divider_trace: RuntimeDividerTrace,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeBattleEscapeCommand {
    pub divider_trace: RuntimeDividerTrace,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeBattleItemCommand {
    pub item_id: String,
    pub divider_trace: RuntimeDividerTrace,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeCaptureCompletionCommand {
    pub outcome: CaptureOutcome,
    pub nickname: Option<String>,
    pub divider_trace: RuntimeDividerTrace,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeOverworldInputCommand {
    pub buttons: Vec<GameButton>,
    pub divider_trace: RuntimeDividerTrace,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeGiftPokemonCommand {
    pub command: RuntimeScriptCommandRef,
    pub original_trainer_name: String,
    pub original_trainer_id: u16,
    pub nickname_accepted: bool,
    pub nickname: Option<String>,
    pub divider_trace: RuntimeDividerTrace,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeRandomScriptCommand {
    pub command: RuntimeScriptCommandRef,
    pub divider_trace: RuntimeDividerTrace,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeRandomScriptMapCommand {
    pub command: RuntimeScriptCommandRef,
    pub divider_trace: RuntimeDividerTrace,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimePartyPokemonCommand {
    pub species_id: String,
    pub level: u8,
    pub held_item_id: Option<String>,
    pub nickname: Option<String>,
    pub original_trainer_name: String,
    pub original_trainer_id: u16,
    pub dvs: Dv,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeDividerTrace {
    pub samples: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeSweetScentEncounterCommand {
    pub command: RuntimeScriptCommandRef,
    pub divider_trace: RuntimeDividerTrace,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeFishingCommand {
    pub rod: String,
    pub divider_trace: RuntimeDividerTrace,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeFishingItemCommand {
    pub item_id: String,
    pub divider_trace: RuntimeDividerTrace,
}

impl RuntimeDividerTrace {
    pub fn new(samples: impl IntoIterator<Item = u8>) -> Self {
        Self {
            samples: samples.into_iter().collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeRockMonEncounterCommand {
    pub command: RuntimeScriptCommandRef,
    pub divider_trace: RuntimeDividerTrace,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeTreeMonEncounterCommand {
    pub command: RuntimeScriptCommandRef,
    pub divider_trace: RuntimeDividerTrace,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeScriptedWildBattleStartCommand {
    pub command: RuntimeScriptCommandRef,
    pub divider_trace: RuntimeDividerTrace,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeStaticWildBattleOrigin {
    pub map_name: String,
    pub source_script: String,
    pub startbattle_command_index: usize,
    pub resume_command_index: usize,
    pub battle_type: String,
    pub species: String,
    pub level: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeScriptedWildBattleTerminal {
    /// Exact persisted `wBattleResult`, including caught/box-full flag bits.
    pub battle_result: u8,
    /// Whether the base-WIN-only Pay Day/Pokerus cleanup already ran before
    /// the source cursor was resumed.
    pub win_cleanup_applied: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeRockSmashMenuOutcome {
    pub party_index: usize,
    pub object_identifier: String,
    pub next_script: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeHeadbuttScriptOutcome {
    pub party_index: usize,
    pub next_script: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeStrengthMenuOutcome {
    pub party_index: usize,
    pub next_script: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSweetScentMenuOutcome {
    pub party_index: usize,
    pub next_script: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeRandomSpecialRoutineCommand {
    pub routine: String,
    pub divider_trace: RuntimeDividerTrace,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeScriptedWildBattleCompletionCommand {
    pub origin: RuntimeStaticWildBattleOrigin,
    pub terminal: RuntimeScriptedWildBattleTerminal,
    pub divider_trace: RuntimeDividerTrace,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeTrainerBattleCompletionCommand {
    pub command: RuntimeScriptCommandRef,
    pub won: bool,
    pub can_lose: bool,
    pub divider_trace: RuntimeDividerTrace,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeMapTrainerInteractionCommand {
    pub command: RuntimeScriptCommandRef,
    pub defer_battle_start: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeMapTrainerInteractionOutcome {
    ReadyForSeenText,
    BattleStarted(TrainerBattleStartStatus),
    AlreadyDefeated { callback: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeShopTransactionCommand {
    pub item_id: String,
    pub quantity: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeClockUpdateCommand {
    pub date: GameDate,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    pub divider_trace: RuntimeDividerTrace,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeManualClockCommand {
    pub now_date: GameDate,
    pub now_hour: u8,
    pub now_minute: u8,
    pub now_second: u8,
    pub target: ClockTime,
    pub divider_trace: RuntimeDividerTrace,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeGameTimerAdvanceCommand {
    pub vblanks: u32,
    /// Exact DIV samples consumed by VBlank_Normal within this batch. Each
    /// normal handler reads DIV twice; special handlers contribute no sample.
    pub normal_divider_trace: RuntimeDividerTrace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeGameTimerCountingCommand {
    pub counting: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeGameLogicPauseCommand {
    pub paused: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeGameTimerOutcome {
    pub counted: bool,
    pub counting: bool,
    pub logic_paused: bool,
    pub hours: u16,
    pub minutes: u8,
    pub seconds: u8,
    pub frames: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum RuntimeDayCareCaretaker {
    Man,
    Lady,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum RuntimeDayCareAction {
    Open,
    Deposit,
    Withdraw,
    Inspect,
    CollectEgg,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum RuntimeBugContestAction {
    GiveParkBalls,
    SelectContestants,
    DropOffMons,
    ReturnMons,
    CheckPartyFull,
    Judge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum RuntimeShuckieAction {
    Give,
    Return,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum RuntimeBadgeRegion {
    Johto,
    Kanto,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum RuntimeCurrencyAccount {
    Money,
    Coins,
}

fn runtime_currency_cap(
    currency_constants: &CurrencyCatalog,
    account: RuntimeCurrencyAccount,
) -> Result<u32> {
    match account {
        RuntimeCurrencyAccount::Money => currency_constants
            .get("MAX_MONEY")
            .context("currency constants missing MAX_MONEY"),
        RuntimeCurrencyAccount::Coins => currency_constants
            .get("MAX_COINS")
            .context("currency constants missing MAX_COINS"),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum RuntimeLinkBattleResult {
    Win,
    Loss,
    Draw,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum RuntimeGameCornerService {
    SlotMachine,
    CardFlip,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeGameCornerCommand {
    pub service: RuntimeGameCornerService,
    pub divider_trace: RuntimeDividerTrace,
}

fn runtime_game_corner_divider_trace(command: &RuntimeGameCornerCommand) -> &RuntimeDividerTrace {
    &command.divider_trace
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum RuntimeHappinessServiceRoutine {
    OlderHaircutBrother,
    YoungerHaircutBrother,
    DaisysGrooming,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum RuntimeMysteryGiftAction {
    Check,
    ClaimItem,
    Unlock,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum RuntimeStoryGateSpecial {
    CheckCaughtCelebi,
    CelebiShrineEvent,
    SnorlaxAwake,
    CheckForBattleTowerRules,
}

impl RuntimeStoryGateSpecial {
    pub const fn routine(self) -> &'static str {
        match self {
            Self::CheckCaughtCelebi => "CheckCaughtCelebi",
            Self::CelebiShrineEvent => "CelebiShrineEvent",
            Self::SnorlaxAwake => "SnorlaxAwake",
            Self::CheckForBattleTowerRules => "CheckForBattleTowerRules",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum RuntimePartyCheckSpecial {
    CheckFirstMonIsEgg,
    GetFirstPokemonHappiness,
    FindPartyMonThatSpecies,
    FindPartyMonAboveLevel,
    FindPartyMonAtLeastThatHappy,
    FindPartyMonThatSpeciesYourTrainerId,
    MonCheck,
    BeastsCheck,
    GameCornerPrizeMonCheckDex,
    UnusedSetSeenMon,
}

impl RuntimePartyCheckSpecial {
    pub const fn routine(self) -> &'static str {
        match self {
            Self::CheckFirstMonIsEgg => "CheckFirstMonIsEgg",
            Self::GetFirstPokemonHappiness => "GetFirstPokemonHappiness",
            Self::FindPartyMonThatSpecies => "FindPartyMonThatSpecies",
            Self::FindPartyMonAboveLevel => "FindPartyMonAboveLevel",
            Self::FindPartyMonAtLeastThatHappy => "FindPartyMonAtLeastThatHappy",
            Self::FindPartyMonThatSpeciesYourTrainerId => "FindPartyMonThatSpeciesYourTrainerId",
            Self::MonCheck => "MonCheck",
            Self::BeastsCheck => "BeastsCheck",
            Self::GameCornerPrizeMonCheckDex => "GameCornerPrizeMonCheckDex",
            Self::UnusedSetSeenMon => "UnusedSetSeenMon",
        }
    }

    pub const fn requires_species(self) -> bool {
        matches!(
            self,
            Self::FindPartyMonThatSpecies | Self::FindPartyMonThatSpeciesYourTrainerId
        )
    }

    pub const fn requires_threshold(self) -> bool {
        matches!(
            self,
            Self::FindPartyMonAboveLevel | Self::FindPartyMonAtLeastThatHappy
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum RuntimePhoneRandomSpecial {
    RandomUnseenWildMon,
    RandomPhoneWildMon,
    RandomPhoneMon,
}

impl RuntimePhoneRandomSpecial {
    pub const fn routine(self) -> &'static str {
        match self {
            Self::RandomUnseenWildMon => "RandomUnseenWildMon",
            Self::RandomPhoneWildMon => "RandomPhoneWildMon",
            Self::RandomPhoneMon => "RandomPhoneMon",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum RuntimeGraphicsSpecial {
    ClearBgPalettesBufferScreen,
    ClearBgPalettes,
    UpdateTimePals,
    ClearTilemap,
    LoadMapPalettes,
    RefreshSprites,
    UpdateSprites,
    ReloadSpritesNoPalettes,
    FadeOutToWhite,
    FadeInFromWhite,
    FadeOutToBlack,
    FadeInFromBlack,
    GameboyCheck,
    CheckMobileAdapterStatus,
    BattleTowerFade,
    UpdatePlayerSprite,
    HealMachineAnim,
    SurfStartStep,
    LoadUsedSpritesGfx,
    ToggleMaptileDecorations,
    ToggleDecorationsVisibility,
    MagnetTrain,
    Diploma,
    PrintDiploma,
    UnownPuzzle,
    OmanyteChamber,
    DisplayUnownWords,
}

impl RuntimeGraphicsSpecial {
    pub const fn routine(self) -> &'static str {
        match self {
            Self::ClearBgPalettesBufferScreen => "ClearBgPalettesBufferScreen",
            Self::ClearBgPalettes => "ClearBgPalettes",
            Self::UpdateTimePals => "UpdateTimePals",
            Self::ClearTilemap => "ClearTilemap",
            Self::LoadMapPalettes => "LoadMapPalettes",
            Self::RefreshSprites => "RefreshSprites",
            Self::UpdateSprites => "UpdateSprites",
            Self::ReloadSpritesNoPalettes => "ReloadSpritesNoPalettes",
            Self::FadeOutToWhite => "FadeOutToWhite",
            Self::FadeInFromWhite => "FadeInFromWhite",
            Self::FadeOutToBlack => "FadeOutToBlack",
            Self::FadeInFromBlack => "FadeInFromBlack",
            Self::GameboyCheck => "GameboyCheck",
            Self::CheckMobileAdapterStatus => "CheckMobileAdapterStatus",
            Self::BattleTowerFade => "BattleTowerFade",
            Self::UpdatePlayerSprite => "UpdatePlayerSprite",
            Self::HealMachineAnim => "HealMachineAnim",
            Self::SurfStartStep => "SurfStartStep",
            Self::LoadUsedSpritesGfx => "LoadUsedSpritesGfx",
            Self::ToggleMaptileDecorations => "ToggleMaptileDecorations",
            Self::ToggleDecorationsVisibility => "ToggleDecorationsVisibility",
            Self::MagnetTrain => "MagnetTrain",
            Self::Diploma => "Diploma",
            Self::PrintDiploma => "PrintDiploma",
            Self::UnownPuzzle => "UnownPuzzle",
            Self::OmanyteChamber => "OmanyteChamber",
            Self::DisplayUnownWords => "DisplayUnownWords",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeDayCareCommand {
    pub caretaker: RuntimeDayCareCaretaker,
    pub action: RuntimeDayCareAction,
    pub party_index: Option<usize>,
    pub divider_trace: RuntimeDividerTrace,
}

fn runtime_day_care_party_slot(command: &RuntimeDayCareCommand) -> Result<Option<usize>> {
    match command.action {
        RuntimeDayCareAction::Deposit => command
            .party_index
            .map(Some)
            .with_context(|| "Day Care deposit command requires party_index"),
        RuntimeDayCareAction::Open
        | RuntimeDayCareAction::Withdraw
        | RuntimeDayCareAction::Inspect
        | RuntimeDayCareAction::CollectEgg => {
            if command.party_index.is_some() {
                anyhow::bail!(
                    "Day Care {} command must not declare party_index",
                    runtime_day_care_action_name(command.action)
                );
            }
            Ok(None)
        }
    }
}

fn runtime_day_care_input(command: &RuntimeDayCareCommand) -> Result<DayCareInput> {
    let party_slot = runtime_day_care_party_slot(command)?;
    Ok(match command.action {
        RuntimeDayCareAction::Open => DayCareInput::Open {},
        RuntimeDayCareAction::Deposit => DayCareInput::Deposit {
            party_slot: party_slot.expect("validated deposit party slot"),
        },
        RuntimeDayCareAction::Withdraw => DayCareInput::Withdraw {},
        RuntimeDayCareAction::Inspect => DayCareInput::Inspect {},
        RuntimeDayCareAction::CollectEgg => DayCareInput::CollectEgg {},
    })
}

fn runtime_day_care_action_name(action: RuntimeDayCareAction) -> &'static str {
    match action {
        RuntimeDayCareAction::Open => "open",
        RuntimeDayCareAction::Deposit => "deposit",
        RuntimeDayCareAction::Withdraw => "withdraw",
        RuntimeDayCareAction::Inspect => "inspect",
        RuntimeDayCareAction::CollectEgg => "collect_egg",
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum RuntimeBugContestCommand {
    GiveParkBalls {},
    SelectContestants { divider_trace: RuntimeDividerTrace },
    DropOffMons {},
    ReturnMons {},
    CheckPartyFull {},
    Judge { divider_trace: RuntimeDividerTrace },
}

impl RuntimeBugContestCommand {
    pub(crate) fn action(&self) -> RuntimeBugContestAction {
        match self {
            Self::GiveParkBalls {} => RuntimeBugContestAction::GiveParkBalls,
            Self::SelectContestants { .. } => RuntimeBugContestAction::SelectContestants,
            Self::DropOffMons {} => RuntimeBugContestAction::DropOffMons,
            Self::ReturnMons {} => RuntimeBugContestAction::ReturnMons,
            Self::CheckPartyFull {} => RuntimeBugContestAction::CheckPartyFull,
            Self::Judge { .. } => RuntimeBugContestAction::Judge,
        }
    }

    pub(crate) fn divider_trace(&self) -> Option<&RuntimeDividerTrace> {
        match self {
            Self::SelectContestants { divider_trace } | Self::Judge { divider_trace } => {
                Some(divider_trace)
            }
            Self::GiveParkBalls {}
            | Self::DropOffMons {}
            | Self::ReturnMons {}
            | Self::CheckPartyFull {} => None,
        }
    }
}

fn runtime_bug_contest_action_name(action: RuntimeBugContestAction) -> &'static str {
    match action {
        RuntimeBugContestAction::GiveParkBalls => "give_park_balls",
        RuntimeBugContestAction::SelectContestants => "select_contestants",
        RuntimeBugContestAction::DropOffMons => "drop_off_mons",
        RuntimeBugContestAction::ReturnMons => "return_mons",
        RuntimeBugContestAction::CheckPartyFull => "check_party_full",
        RuntimeBugContestAction::Judge => "judge",
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeKurtApricornCommand {
    pub apricorn_id: String,
    pub quantity: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeBuenaPasswordCommand {
    pub guess: Option<String>,
    pub divider_trace: RuntimeDividerTrace,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeBuenaPrizeCommand {
    pub item_id: String,
    pub quantity: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum RuntimeShuckieCommand {
    Give { divider_trace: RuntimeDividerTrace },
    Return { party_index: Option<usize> },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeGiveDratiniCommand {
    pub mode: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeOddEggCommand {
    pub divider_trace: RuntimeDividerTrace,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeBillsGrandfatherCommand {
    pub party_index: Option<usize>,
    pub species_id: Option<String>,
}

fn runtime_bills_grandfather_inputs(
    command: &RuntimeBillsGrandfatherCommand,
) -> Result<(Option<usize>, Option<String>)> {
    match (&command.party_index, &command.species_id) {
        (Some(party_index), None) => Ok((Some(*party_index), None)),
        (None, Some(species_id)) => Ok((None, Some(species_id.clone()))),
        (Some(_), Some(_)) => {
            anyhow::bail!(
                "Bills Grandfather command must declare either party_index or species_id, not both"
            );
        }
        (None, None) => {
            anyhow::bail!("Bills Grandfather command requires party_index or species_id");
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeMagikarpLengthCommand {
    pub party_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeBadgeCommand {
    pub region: RuntimeBadgeRegion,
    pub index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimePokedexCommand {
    pub species_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeCurrencyDeltaCommand {
    pub account: RuntimeCurrencyAccount,
    pub amount: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeLinkBattleRecordCommand {
    pub result: RuntimeLinkBattleResult,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeLinkFriendReadyCommand {
    pub serial_connection_status: LinkSerialConnectionStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeLinkTimeoutCommand {
    pub other_player_link_mode: u8,
    pub serial_connection_status: LinkSerialConnectionStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeLinkRoomSelectionCommand {
    pub other_player_room: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeCableClubGenderCommand {
    pub gender: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeOptionsCommand {
    pub options: Options,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimePokegearRadioTuningCommand {
    pub tuning_knob: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeTrainerIdentityCommand {
    pub player_name: String,
    pub player_id: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimePlayerGenderCommand {
    pub player_gender: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimePartyNicknameCommand {
    pub party_index: usize,
    pub nickname: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeStoredPokemonNicknameCommand {
    pub location: crystal_core::models::CaptureStorageLocation,
    pub nickname: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimePartySlotCommand {
    pub party_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeMailboxSlotCommand {
    pub mailbox_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeMailboxPartyCommand {
    pub mailbox_index: usize,
    pub party_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeComposeMailCommand {
    pub item_id: String,
    pub party_index: usize,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimePartyRecoverySetupCommand {
    pub party_index: usize,
    pub hp: u16,
    pub status: Option<String>,
    pub first_move_pp: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimePartyHpTransferCommand {
    pub source_party_index: usize,
    pub target_party_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimePartySwapCommand {
    pub first_party_index: usize,
    pub second_party_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimePartyMoveSwapCommand {
    pub party_index: usize,
    pub first_move_index: usize,
    pub second_move_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimePcBoxCommand {
    pub box_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimePcBoxNameCommand {
    pub box_index: usize,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimePcDepositCommand {
    pub party_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimePcWithdrawCommand {
    pub box_slot: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimePcReleaseCommand {
    pub box_slot: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RuntimePokemonStorageLocation {
    Party { slot: usize },
    Box { box_index: usize, slot: usize },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimePcMoveCommand {
    pub source: RuntimePokemonStorageLocation,
    pub target: RuntimePokemonStorageLocation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimePcItemCommand {
    pub item_id: String,
    pub stack_index: usize,
    pub quantity: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeHeldItemCommand {
    pub item_id: String,
    pub party_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeHappinessServiceCommand {
    pub routine: RuntimeHappinessServiceRoutine,
    pub party_index: usize,
    pub divider_trace: RuntimeDividerTrace,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeMoveDeletionCommand {
    pub party_index: usize,
    pub move_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeMoveTutorCommand {
    pub party_index: usize,
    pub move_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeNameRivalCommand {
    pub rival_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeRememberPasswordCommand {
    pub remember: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimePlayerPaletteCommand {
    pub raw_value: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeFishingSwarmCommand {
    pub value: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeMobileHandshakeCommand {
    pub accepted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeMobileSelectThreeMonsCommand {
    pub party_indexes: [usize; 3],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimePcBagItemCheckCommand {
    pub item_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimePhoneCallerCommand {
    pub special: RuntimePhoneRandomSpecial,
    pub contact_id: String,
    pub divider_trace: RuntimeDividerTrace,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimePokegearPhoneCallCommand {
    pub contact_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePokegearPhoneCallOutcome {
    pub contact_id: String,
    pub callback_script: String,
    pub callee_script: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeSpecialCryCommand {
    pub species_id: String,
}

fn runtime_special_cry_species(command: &RuntimeSpecialCryCommand) -> Result<&str> {
    validate_modpack_payload_token(&command.species_id, "special cry species id")?;
    Ok(command.species_id.as_str())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum RuntimeCableClubRequest {
    Trade,
    Battle,
    TimeCapsule,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum RuntimeLinkRoomSpecial {
    TradeCenter,
    Colosseum,
    TimeCapsule,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum RuntimeBattleTowerMobileFlag {
    Enabled,
    Disabled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeBattleTowerActionCommand {
    pub action: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeBattleTowerChallengeMenuCommand {
    pub english: bool,
    pub selection: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeBattleTowerRoomMenuCommand {
    pub selection: Option<u8>,
    pub cancelled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeBattleTowerBattleCommand {
    pub battle_result: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeBattleTowerOpponentCommand {
    pub target_object: String,
    pub divider_trace: RuntimeDividerTrace,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeMapRadioCommand {
    pub station: String,
}

fn runtime_map_radio_station(command: &RuntimeMapRadioCommand) -> Result<&str> {
    validate_modpack_payload_token(&command.station, "MapRadio station")?;
    Ok(command.station.as_str())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimePartyCheckCommand {
    pub special: RuntimePartyCheckSpecial,
    pub species_id: Option<String>,
    pub threshold: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeRegisteredKeyItemCommand {
    pub item_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeRegisteredKeyItemOutcome {
    pub previous_item_id: Option<String>,
    pub item_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "payload",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum RuntimeMutationCommand {
    ApplyOverworldInput(RuntimeOverworldInputCommand),
    GrantScriptItem(RuntimeScriptCommandRef),
    CheckScriptItem(RuntimeScriptCommandRef),
    TakeScriptItem(RuntimeScriptCommandRef),
    PickupScriptFieldItem(RuntimeScriptCommandRef),
    ApplyScriptEconomy(RuntimeScriptCommandRef),
    ApplyScriptPhone {
        command: RuntimeScriptCommandRef,
        inputs: ScriptPhoneInputs,
    },
    ApplyScriptFlagMutation(RuntimeScriptCommandRef),
    CheckScriptFlag(RuntimeScriptCommandRef),
    ApplyScriptScene(RuntimeScriptCommandRef),
    ApplyScriptBlockChange(RuntimeScriptCommandRef),
    ApplyScriptAudio(RuntimeScriptCommandRef),
    ApplyScriptMap(RuntimeScriptCommandRef),
    ApplyRandomScriptMap(RuntimeRandomScriptMapCommand),
    TransitionPendingScriptWarp,
    ApplyMapSetupCallbacks {
        map_setup: String,
    },
    ApplyScriptText(RuntimeScriptCommandRef),
    ApplyScriptVariableNow(RuntimeScriptCommandRef),
    ApplyScriptControl(RuntimeScriptCommandRef),
    ApplyScriptObjectMutation(RuntimeScriptCommandRef),
    ApplyScriptMovement(RuntimeScriptCommandRef),
    ApplyScriptRuntime {
        command: RuntimeScriptCommandRef,
        inputs: ScriptRuntimeInputs,
    },
    ApplyRandomScriptRuntime(RuntimeRandomScriptCommand),
    TakeNextScript,
    DrainScriptEventQueue(RuntimeScriptEventDrainCommand),
    DrainScriptRuntimeQueue(RuntimeScriptRuntimeQueueDrainCommand),
    PopScriptCallStack,
    PopDeferredScript,
    TakeScriptEndState,
    TakePendingScriptRequest(RuntimePendingScriptRequestCommand),
    ResolvePendingYesNo(RuntimePendingYesNoResolutionCommand),
    OpenVerticalMenu(RuntimeVerticalMenuOpenCommand),
    SelectVerticalMenuOption(RuntimeVerticalMenuSelectionCommand),
    SelectElevatorFloor(RuntimeElevatorFloorSelectionCommand),
    ConsumeScriptRuntimeFlag(RuntimeScriptRuntimeFlagCommand),
    TakeScriptRuntimeMemoryValue(RuntimeScriptRuntimeMemoryValueCommand),
    RemoveScriptRuntimeMemoryEntry(RuntimeScriptRuntimeMemoryEntryCommand),
    OpenScriptShop(RuntimeScriptCommandRef),
    CloseActiveMenu,
    CloseRuntimeWindow,
    CloseTextWindow,
    CloseActivePokemonPicture,
    CloseScriptShop,
    BuyShopItem(RuntimeShopTransactionCommand),
    SellShopItem(RuntimeShopTransactionCommand),
    ApplySpecialRoutine {
        routine: String,
    },
    ApplyRandomSpecialRoutine(RuntimeRandomSpecialRoutineCommand),
    ResolveBugContestCaughtMon {
        keep_new: bool,
    },
    GrantScriptedGiftPokemon(RuntimeGiftPokemonCommand),
    AddPartyPokemon(RuntimePartyPokemonCommand),
    StartScriptedWildBattle(RuntimeScriptedWildBattleStartCommand),
    StartScriptedTrainerBattle(RuntimeScriptCommandRef),
    ResolveMapTrainerInteraction(RuntimeMapTrainerInteractionCommand),
    CompleteScriptedWildBattle(RuntimeScriptedWildBattleCompletionCommand),
    CompleteScriptedTrainerBattle(RuntimeTrainerBattleCompletionCommand),
    UseBagItem {
        item_id: String,
        context: ItemUseContext,
    },
    ReplacePendingMoveLearn(RuntimeMoveLearnReplacementCommand),
    DeclinePendingMoveLearn,
    RegisterKeyItem(RuntimeRegisteredKeyItemCommand),
    UseBagRepelInField(RuntimeItemCommand),
    UseBagBicycleInField(RuntimeItemCommand),
    UseBagItemfinderInField(RuntimeItemCommand),
    UseBagSquirtbottleInField(RuntimeItemCommand),
    UseBagStoryKeyInField(RuntimeItemCommand),
    UseBagCoinCaseInField(RuntimeItemCommand),
    UseBagBlueCardInField(RuntimeItemCommand),
    UseBagTownMapInField(RuntimeItemCommand),
    UseBagPokegearInField(RuntimeItemCommand),
    UseBagBoxInField(RuntimeItemCommand),
    UseBagEscapeRopeInField(RuntimeItemCommand),
    UseCutFieldMove(RuntimeFieldBlockMoveCommand),
    UseWhirlpoolFieldMove(RuntimeFieldBlockMoveCommand),
    QueueStrengthFromMenu(RuntimeFieldPartyCommand),
    UseFlashFieldMove(RuntimeFieldPartyCommand),
    UseSurfFieldMove(RuntimeFieldPartyCommand),
    UseWaterfallFieldMove(RuntimeFieldPartyCommand),
    UseFlyFieldMove(RuntimeFlyCommand),
    UseDigFieldMove(RuntimeFieldPartyCommand),
    UseTeleportFieldMove(RuntimeFieldPartyCommand),
    CommitPendingFieldTravel,
    QueueHeadbuttScript(RuntimeHeadbuttScriptCommand),
    QueueRockSmashFromMenu(RuntimeFieldPartyCommand),
    ResolveRockMonEncounter(RuntimeRockMonEncounterCommand),
    ResolveTreeMonEncounter(RuntimeTreeMonEncounterCommand),
    QueueSweetScentFromMenu(RuntimeFieldPartyCommand),
    ResolveSweetScentEncounter(RuntimeSweetScentEncounterCommand),
    UseBagItemOnPartyPokemon(RuntimePartyItemCommand),
    UseBagItemOnWholeParty(RuntimeItemCommand),
    UseBagItemOnPartyMove(RuntimePartyMoveItemCommand),
    UseBagTmHmOnPartyPokemon(RuntimeTmHmCommand),
    UseBagItemOnActiveBattlePokemon(RuntimeItemCommand),
    UseBagItemOnBattlePartyPokemon(RuntimePartyItemCommand),
    UseBagItemOnBattlePartyMove(RuntimePartyMoveItemCommand),
    ThrowBallAtActiveBattle(RuntimeBattleItemCommand),
    CompleteActiveWildCapture(RuntimeCaptureCompletionCommand),
    SwitchActiveBattleParty(RuntimePartySlotCommand),
    ResolveActiveBattleTurn(RuntimeBattleTurnCommand),
    ResolveActiveBattleCommand(RuntimeBattleTurnCommand),
    ResolveActiveBattleEnemyAction(RuntimeBattleEnemyActionCommand),
    AttemptEscapeActiveWildBattle(RuntimeBattleEscapeCommand),
    UseBagItemToEscapeActiveWildBattle(RuntimeBattleItemCommand),
    UseBagGuardSpecInActiveBattle(RuntimeItemCommand),
    AdvanceActiveTrainerBattle,
    ClaimActiveTrainerBattleRewardsNow,
    ClaimActiveWildBattleRewardsNow(RuntimeDividerTrace),
    CastFishingRod(RuntimeFishingCommand),
    UseBagFishingRodInField(RuntimeFishingItemCommand),
    AdvanceGameTimerVBlanks(RuntimeGameTimerAdvanceCommand),
    SetGameTimerCounting(RuntimeGameTimerCountingCommand),
    SetGameLogicPaused(RuntimeGameLogicPauseCommand),
    UpdateClockFromDatetime(RuntimeClockUpdateCommand),
    SetManualClockTime(RuntimeManualClockCommand),
    ApplyScriptSwarm(RuntimeScriptCommandRef),
    ExecuteNextQueuedScriptCommand,
    UseDayCare(RuntimeDayCareCommand),
    CheckDayCareManOutsideSpecial(RuntimeDividerTrace),
    CheckDayCareResidentSpecial(RuntimeDayCareCaretaker),
    UseBugContest(RuntimeBugContestCommand),
    UseKurtApricorn(RuntimeKurtApricornCommand),
    UseBuenaPassword(RuntimeBuenaPasswordCommand),
    UseBuenaPrize(RuntimeBuenaPrizeCommand),
    UseShuckie(RuntimeShuckieCommand),
    GiveOddEgg(RuntimeOddEggCommand),
    GiveDratini(RuntimeGiveDratiniCommand),
    UseBillsGrandfather(RuntimeBillsGrandfatherCommand),
    InitRoamMons,
    CheckMagikarpLength(RuntimeMagikarpLengthCommand),
    ShowProfOaksPcBoot,
    ShowMagikarpHouseSign,
    ApplyBattleTowerAction(RuntimeBattleTowerActionCommand),
    UseBattleTowerRoomMenu(RuntimeBattleTowerRoomMenuCommand),
    StartBattleTowerBattleSpecial(RuntimeBattleTowerBattleCommand),
    LoadBattleTowerOpponentSpecial(RuntimeBattleTowerOpponentCommand),
    ShowBattleTowerMobileErrorSpecial,
    AskRememberPasswordSpecial(RuntimeRememberPasswordCommand),
    OpenBattleTowerLeaderboardSpecial,
    ApplyMobileHandshakeSpecial(RuntimeMobileHandshakeCommand),
    EndMobileSessionSpecial,
    SetBattleTowerMobileFlagSpecial(RuntimeBattleTowerMobileFlag),
    SelectThreeMobileMonsSpecial(RuntimeMobileSelectThreeMonsCommand),
    ApplyHappinessService(RuntimeHappinessServiceCommand),
    UseMysteryGift(RuntimeMysteryGiftAction),
    WarpToSpawnPoint,
    HealPartySpecial,
    FadeOutMusicSpecial,
    WaitSfxSpecial,
    PlayMapMusicSpecial,
    RestartMapMusicSpecial,
    PlayCurMonCry(RuntimeSpecialCryCommand),
    PlaySlowCry(RuntimeSpecialCryCommand),
    OpenPokemonCenterPcSpecial,
    OpenPlayersHousePcSpecial,
    OpenOverworldTownMapSpecial,
    OpenUnownPrinterSpecial,
    OpenMapRadioSpecial(RuntimeMapRadioCommand),
    NameRivalSpecial(RuntimeNameRivalCommand),
    DeletePartyMoveSpecial(RuntimeMoveDeletionCommand),
    CheckPokerusSpecial,
    RatePartyNicknameSpecial(RuntimePartyNicknameCommand),
    SeePartyPokemonSpecial(RuntimePartySlotCommand),
    TeachPartyMoveSpecial(RuntimeMoveTutorCommand),
    OpenBankOfMomSpecial,
    OpenGameCornerSpecial(RuntimeGameCornerCommand),
    OpenDisplayLinkRecordSpecial,
    OpenTrainerHouseSpecial,
    OpenPhotoStudioSpecial(RuntimePartySlotCommand),
    UseBattleTowerChallengeMenu(RuntimeBattleTowerChallengeMenuCommand),
    ApplyGraphicsSpecial(RuntimeGraphicsSpecial),
    ApplyPartyCheckSpecial(RuntimePartyCheckCommand),
    ApplyPhoneRandomSpecial(RuntimePhoneCallerCommand),
    CheckItemInPcOrBagSpecial(RuntimePcBagItemCheckCommand),
    CheckAnotherUsablePartyMonSpecial(RuntimePartySlotCommand),
    ActivateFishingSwarmSpecial(RuntimeFishingSwarmCommand),
    ApplyStoryGateSpecial(RuntimeStoryGateSpecial),
    SetPlayerPalette(RuntimePlayerPaletteCommand),
    SetDayOfWeek,
    UpdateTime,
    SwitchCurrentPcBox(RuntimePcBoxCommand),
    NamePcBox(RuntimePcBoxNameCommand),
    DepositPartyPokemonToCurrentBox(RuntimePcDepositCommand),
    WithdrawCurrentBoxPokemonToParty(RuntimePcWithdrawCommand),
    ReleaseCurrentBoxPokemon(RuntimePcReleaseCommand),
    MovePcPokemonWithoutMail(RuntimePcMoveCommand),
    DepositBagItemToPc(RuntimePcItemCommand),
    WithdrawPcItemToBag(RuntimePcItemCommand),
    TossPcItem(RuntimePcItemCommand),
    SetUpDecoration(RuntimeDecorationSetupCommand),
    PutAwayDecoration(RuntimeDecorationPutAwayCommand),
    GiveBagItemToPartyPokemon(RuntimeHeldItemCommand),
    ComposeBagMailToParty(RuntimeComposeMailCommand),
    TakeHeldItemFromPartyPokemon(RuntimePartySlotCommand),
    SendPartyMailToMailbox(RuntimePartySlotCommand),
    DiscardPartyMailToBag(RuntimePartySlotCommand),
    DeleteMailboxMail(RuntimeMailboxSlotCommand),
    MoveMailboxMailToBag(RuntimeMailboxSlotCommand),
    AttachMailboxMailToParty(RuntimeMailboxPartyCommand),
    AwardBadge(RuntimeBadgeCommand),
    RecordPokedexSeen(RuntimePokedexCommand),
    RecordPokedexCaught(RuntimePokedexCommand),
    AddBagItem(RuntimeBagItemDeltaCommand),
    AddCurrency(RuntimeCurrencyDeltaCommand),
    TakeCurrency(RuntimeCurrencyDeltaCommand),
    RecordLinkBattleResult(RuntimeLinkBattleRecordCommand),
    SetCableClubRequest(RuntimeCableClubRequest),
    WaitForLinkedFriendSpecial(RuntimeLinkFriendReadyCommand),
    CheckLinkTimeoutReceptionistSpecial(RuntimeLinkTimeoutCommand),
    CheckBothSelectedSameRoomSpecial(RuntimeLinkRoomSelectionCommand),
    CloseLinkSpecial,
    WaitForOtherPlayerToExitSpecial,
    FailedLinkToPastSpecial,
    OpenLinkRoomSpecial(RuntimeLinkRoomSpecial),
    CheckTimeCapsuleCompatibilitySpecial,
    TryQuickSaveSpecial,
    AskMobileOrCableSpecial,
    CableClubCheckWhichChrisSpecial(RuntimeCableClubGenderCommand),
    SetOptions(RuntimeOptionsCommand),
    SetPokegearRadioTuning(RuntimePokegearRadioTuningCommand),
    SetTrainerIdentity(RuntimeTrainerIdentityCommand),
    SetPlayerGender(RuntimePlayerGenderCommand),
    RenamePartyPokemon(RuntimePartyNicknameCommand),
    RenameStoredPokemon(RuntimeStoredPokemonNicknameCommand),
    SetPartyPokemonRecoveryState(RuntimePartyRecoverySetupCommand),
    TransferPartyPokemonHp(RuntimePartyHpTransferCommand),
    FullHealPartyPokemon(RuntimePartySlotCommand),
    FullHealWholeParty,
    ResolveBlackoutToLastSpawn,
    SwapPartyPokemon(RuntimePartySwapCommand),
    SwapPartyPokemonMoves(RuntimePartyMoveSwapCommand),
    InitializePermanentPhoneNumbers,
    StartPokegearPhoneCall(RuntimePokegearPhoneCallCommand),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum RuntimeScriptEventQueue {
    Audio,
    Graphics,
    Money,
    Map,
    Text,
    Control,
    Shop,
    ItemUse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeScriptEventDrainCommand {
    pub queue: RuntimeScriptEventQueue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum RuntimeScriptRuntimeQueue {
    PendingDelay,
    PendingEarthquake,
    PendingEmote,
    Command,
    CallStack,
    DeferredScript,
    MapReentryScript,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeScriptRuntimeQueueDrainCommand {
    pub queue: RuntimeScriptRuntimeQueue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum RuntimePendingScriptRequestKind {
    MusicFade,
    ScreenFade,
    ScriptWarp,
    MapLoad,
    MapRefresh,
    TextLabel,
    TextWait,
    YesNo,
    Shop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimePendingScriptRequestCommand {
    pub kind: RuntimePendingScriptRequestKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimePendingYesNoResolutionCommand {
    pub accepted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeVerticalMenuSelectionCommand {
    pub menu_id: String,
    pub source_script: String,
    pub verticalmenu_command_index: usize,
    pub option_index: usize,
    pub option: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeVerticalMenuOpenCommand {
    pub map_name: String,
    pub menu_key: String,
    pub source_script: String,
    pub loadmenu_command_index: usize,
    pub verticalmenu_command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeElevatorFloorSelectionCommand {
    pub map_name: String,
    pub data_label: String,
    pub source_script: String,
    pub elevator_command_index: usize,
    pub floor_index: usize,
    pub floor: String,
    pub warp: u16,
    pub target_map: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum RuntimeScriptRuntimeFlag {
    MapMusicRestartDisabled,
    MapMusicRequested,
    WaitingForSoundEffect,
    ItemNotifyQueued,
    WarpSoundQueued,
    TeleportFromQueued,
    HallOfFameRequested,
    CreditsRequested,
    ResetRequested,
    Menu2dRequested,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeScriptRuntimeFlagCommand {
    pub flag: RuntimeScriptRuntimeFlag,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum RuntimeScriptRuntimeMemoryValue {
    ScriptValue,
    LastTalkedObject,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeScriptRuntimeMemoryValueCommand {
    pub value: RuntimeScriptRuntimeMemoryValue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum RuntimeScriptRuntimeMemoryEntry {
    Variable,
    Memory,
    NamedBuffer,
    VariableSprite,
    PhoneNumber,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeScriptRuntimeMemoryEntryCommand {
    pub entry: RuntimeScriptRuntimeMemoryEntry,
    pub key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum RuntimeScriptEventDrainResult {
    Audio(Vec<ScriptAudioRuntimeEvent>),
    Graphics(Vec<ScriptGraphicsRuntimeEvent>),
    Money(Vec<ScriptMoneyRuntimeEvent>),
    Map(Vec<ScriptMapRuntimeEvent>),
    Text(Vec<ScriptTextRuntimeEvent>),
    Control(Vec<ScriptControlRuntimeEvent>),
    Shop(Vec<ScriptShopRuntimeEvent>),
    ItemUse(Vec<ItemUseRuntimeEvent>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum RuntimeScriptRuntimeQueueDrainResult {
    PendingDelay(Vec<ScriptRuntimeDelay>),
    PendingEarthquake(Vec<ScriptRuntimeEarthquake>),
    PendingEmote(Vec<ScriptRuntimeEmote>),
    Command(Vec<ScriptRuntimeQueuedCommand>),
    CallStack(Vec<ScriptReturnFrame>),
    DeferredScript(Vec<ScriptLocation>),
    MapReentryScript(Vec<ScriptLocation>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum RuntimePendingScriptRequest {
    MusicFade(ScriptMusicFade),
    ScreenFade(ScriptScreenFade),
    ScriptWarp(ScriptWarpRequest),
    MapLoad(ScriptMapLoadRequest),
    MapRefresh(ScriptMapRefreshRequest),
    TextLabel(String),
    TextWait(ScriptTextWait),
    YesNo(ScriptYesNoPrompt),
    Shop(crystal_core::state::ScriptShopRequest),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePendingYesNoResolution {
    pub prompt: ScriptYesNoPrompt,
    pub accepted: bool,
    pub script_value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeVerticalMenuSelection {
    pub menu_id: String,
    pub source_script: String,
    pub verticalmenu_command_index: usize,
    pub option_index: usize,
    pub option: String,
    pub script_value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeVerticalMenuOpen {
    pub map_name: String,
    pub menu_key: String,
    pub menu_id: String,
    pub source_script: String,
    pub loadmenu_command_index: usize,
    pub verticalmenu_command_index: usize,
    pub options: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeElevatorFloorSelection {
    pub map_name: String,
    pub data_label: String,
    pub source_script: String,
    pub elevator_command_index: usize,
    pub floor_index: usize,
    pub floor: String,
    pub warp: u16,
    pub target_map: String,
    pub destination_tile: TilePosition,
    pub script_value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum RuntimeScriptRuntimeFlagValue {
    MapMusicRestartDisabled,
    MapMusicRequested,
    WaitingForSoundEffect,
    ItemNotifyQueued,
    WarpSoundQueued,
    TeleportFromQueued,
    HallOfFameRequested,
    CreditsRequested,
    ResetRequested,
    Menu2dRequested,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum RuntimeScriptRuntimeMemoryValueTaken {
    ScriptValue(String),
    LastTalkedObject(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum RuntimeScriptRuntimeMemoryEntryRemoved {
    Variable { key: String, value: String },
    Memory { key: String, value: String },
    NamedBuffer { key: String, value: String },
    VariableSprite { key: String, value: String },
    PhoneNumber { key: String },
}

pub const RUNTIME_MUTATION_COMMAND_SCHEMA: &str = "crystal_runtime_mutation_command_v49";

pub fn encode_runtime_mutation_command_payload(
    command: &RuntimeMutationCommand,
) -> Result<RuntimeCommandPayload> {
    let bytes =
        serde_json::to_vec(command).context("serialize runtime mutation command payload")?;
    RuntimeCommandPayload::new(RUNTIME_MUTATION_COMMAND_SCHEMA, bytes)
        .map_err(|error| anyhow::anyhow!("build runtime mutation command payload: {error}"))
}

pub fn decode_runtime_mutation_command_payload(
    payload: &RuntimeCommandPayload,
) -> Result<RuntimeMutationCommand> {
    if payload.schema() != RUNTIME_MUTATION_COMMAND_SCHEMA {
        anyhow::bail!(
            "runtime command payload schema '{}' does not match expected '{}'",
            payload.schema(),
            RUNTIME_MUTATION_COMMAND_SCHEMA
        );
    }
    serde_json::from_slice(payload.bytes()).context("decode runtime mutation command payload")
}

pub fn validate_runtime_mutation_command_payload(
    payload: &RuntimeCommandPayload,
) -> Result<(), RuntimeCommandFrameError> {
    payload.validate()?;
    if payload.schema() != RUNTIME_MUTATION_COMMAND_SCHEMA {
        return Err(RuntimeCommandFrameError::InvalidToken {
            field: "runtime mutation command payload schema",
        });
    }
    Ok(())
}

pub fn runtime_mutation_command_frame(
    player_id: PlayerId,
    sequence: u64,
    command: &RuntimeMutationCommand,
    state: &GameState,
) -> Result<RuntimeCommandFrame> {
    let expected_state =
        game_state_checksum(state).context("checksum runtime command expected state")?;
    let payload = encode_runtime_mutation_command_payload(command)?;
    RuntimeCommandFrame::new(player_id, sequence, payload, expected_state)
        .map_err(|error| anyhow::anyhow!("build runtime mutation command frame: {error}"))
}

pub fn decode_runtime_mutation_command_frame(
    request: &RuntimeCommandFrame,
    state: &GameState,
) -> Result<RuntimeMutationCommand> {
    let actual_state =
        game_state_checksum(state).context("checksum runtime command actual state")?;
    request
        .require_expected_state(&actual_state)
        .map_err(|error| anyhow::anyhow!("validate runtime command expected state: {error}"))?;
    decode_runtime_mutation_command_payload(request.payload())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeStorageBoxSwitchOutcome {
    pub box_index_before: usize,
    pub box_index_after: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeStorageBoxNameOutcome {
    pub box_index: usize,
    pub previous_name: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeStorageDepositOutcome {
    pub party_index: usize,
    pub box_index: usize,
    pub box_slot: usize,
    pub pokemon: Pokemon,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeStorageWithdrawOutcome {
    pub box_index: usize,
    pub box_slot: usize,
    pub party_index: usize,
    pub pokemon: Pokemon,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeStorageReleaseOutcome {
    pub box_index: usize,
    pub box_slot: usize,
    pub pokemon: Pokemon,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeStorageMoveOutcome {
    pub source: RuntimePokemonStorageLocation,
    pub target: RuntimePokemonStorageLocation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePcItemTransferOutcome {
    pub item_id: String,
    pub quantity: u16,
    pub bag_quantity_after: u16,
    pub pc_quantity_after: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeHeldItemTransferOutcome {
    pub party_index: usize,
    pub item_id: String,
    pub bag_quantity_after: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeMailTransferOutcome {
    pub party_index: Option<usize>,
    pub mailbox_index: Option<usize>,
    pub item_id: String,
    pub mail: crystal_core::models::pokemon::MailData,
    pub mailbox_count_after: usize,
    pub bag_quantity_after: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeBadgeAwardOutcome {
    pub region: RuntimeBadgeRegion,
    pub index: usize,
    pub already_awarded: bool,
    pub awarded_count_after: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePokedexRecordOutcome {
    pub species_id: String,
    pub already_seen: bool,
    pub already_caught: bool,
    pub seen_count_after: usize,
    pub caught_count_after: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeCurrencyMutationOutcome {
    pub account: RuntimeCurrencyAccount,
    pub amount: u32,
    pub value_before: u32,
    pub value_after: u32,
    pub cap: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeBagItemMutationOutcome {
    pub item_id: String,
    pub quantity: u16,
    pub added: bool,
    pub quantity_before: u16,
    pub quantity_after: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeLinkBattleRecordOutcome {
    pub result: RuntimeLinkBattleResult,
    pub wins_after: u16,
    pub losses_after: u16,
    pub draws_after: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeOptionsSetOutcome {
    pub options_before: Options,
    pub options_after: Options,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePokegearRadioTuningOutcome {
    pub tuning_knob_before: u8,
    pub tuning_knob_after: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeTrainerIdentityOutcome {
    pub player_name_before: String,
    pub player_id_before: u16,
    pub player_name_after: String,
    pub player_id_after: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePlayerGenderOutcome {
    pub player_gender_before: u8,
    pub player_gender_after: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePartyNicknameOutcome {
    pub party_index: usize,
    pub species_id: String,
    pub nickname_before: String,
    pub nickname_after: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeStoredPokemonNicknameOutcome {
    pub location: crystal_core::models::CaptureStorageLocation,
    pub species_id: String,
    pub nickname_before: String,
    pub nickname_after: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartyRecoveryOutcome {
    pub party_index: usize,
    pub species_id: String,
    pub hp_before: u16,
    pub hp_after: u16,
    pub status_before: Option<String>,
    pub status_after: Option<String>,
    pub pp_restored: Vec<(String, u8, u8)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePartyRecoverySetupOutcome {
    pub party_index: usize,
    pub species_id: String,
    pub hp_before: u16,
    pub hp_after: u16,
    pub status_before: Option<String>,
    pub status_after: Option<String>,
    pub first_move: Option<String>,
    pub first_move_pp_before: Option<u8>,
    pub first_move_pp_after: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePartyHpTransferOutcome {
    pub source_party_index: usize,
    pub target_party_index: usize,
    pub amount: u16,
    pub source_hp_before: u16,
    pub source_hp_after: u16,
    pub target_hp_before: u16,
    pub target_hp_after: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlackoutRecoveryOutcome {
    pub spawn_identifier: Option<u16>,
    pub map_name: String,
    pub tile: TilePosition,
    pub healed: Vec<PartyRecoveryOutcome>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePartySwapOutcome {
    pub first_party_index: usize,
    pub second_party_index: usize,
    pub first_species_after: String,
    pub second_species_after: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePartyMoveSwapOutcome {
    pub party_index: usize,
    pub first_move_index: usize,
    pub second_move_index: usize,
    pub first_move_after: String,
    pub second_move_after: String,
}

fn full_heal_party_slot(
    state: &mut GameState,
    moves: &BTreeMap<String, Move>,
    party_index: usize,
    nuzlocke_rules: crate::NuzlockeRules,
) -> Result<PartyRecoveryOutcome> {
    let pokemon = state
        .storage
        .party
        .pokemon
        .get_mut(party_index)
        .with_context(|| format!("party index {party_index} is outside party"))?
        .as_mut()
        .with_context(|| format!("party index {party_index} has no Pokemon to heal"))?;
    let hp_before = pokemon.hp;
    let status_before = pokemon.status.clone();
    let mut pp_restored = Vec::new();
    for learned in &mut pokemon.moves {
        let move_data = moves.get(&learned.name).with_context(|| {
            format!(
                "party move {} is missing from modpack move catalog",
                learned.name
            )
        })?;
        let before_pp = learned.current_pp;
        let after_pp = move_data.pp;
        learned.current_pp = after_pp;
        pp_restored.push((learned.name.clone(), before_pp, after_pp));
    }
    if !nuzlocke_rules.permadeath || hp_before > 0 {
        pokemon.hp = pokemon.max_hp;
    }
    pokemon.status = None;
    let outcome = PartyRecoveryOutcome {
        party_index,
        species_id: pokemon.species.id.clone(),
        hp_before,
        hp_after: pokemon.hp,
        status_before,
        status_after: pokemon.status.clone(),
        pp_restored,
    };
    state.sync_party_from_storage();
    Ok(outcome)
}

fn take_party_pokemon_compact(state: &mut GameState, party_index: usize) -> Result<Pokemon> {
    if party_index >= state.storage.party.pokemon.len() {
        anyhow::bail!("party index {party_index} is outside party");
    }
    let pokemon = state.storage.party.pokemon[party_index]
        .take()
        .with_context(|| format!("party index {party_index} has no Pokemon"))?;
    for index in party_index..state.storage.party.pokemon.len() - 1 {
        state.storage.party.pokemon[index] = state.storage.party.pokemon[index + 1].take();
    }
    state.sync_party_from_storage();
    Ok(pokemon)
}

fn prepare_pc_move_locations(
    state: &mut GameState,
    source: &RuntimePokemonStorageLocation,
    target: &RuntimePokemonStorageLocation,
) -> Result<()> {
    let highest_box = [source, target]
        .into_iter()
        .filter_map(|location| match location {
            RuntimePokemonStorageLocation::Party { .. } => None,
            RuntimePokemonStorageLocation::Box { box_index, .. } => Some(*box_index),
        })
        .max();
    if highest_box.is_some_and(|box_index| box_index >= MAX_PC_BOXES) {
        anyhow::bail!("PC move box index is outside 0..{MAX_PC_BOXES}");
    }
    if let Some(highest_box) = highest_box {
        while state.storage.pc_boxes.len() <= highest_box {
            let next = state.storage.pc_boxes.len();
            state.storage.pc_boxes.push(PcBox::new(next));
        }
    }

    let source_count = pc_move_location_count(state, source);
    let source_slot = pc_move_location_slot(source);
    if source_slot >= source_count {
        anyhow::bail!(
            "PC move source slot {source_slot} is outside compact source count {source_count}"
        );
    }
    let target_count = pc_move_location_count(state, target);
    let target_slot = pc_move_location_slot(target);
    if target_slot > target_count {
        anyhow::bail!(
            "PC move target insertion slot {target_slot} is outside compact range 0..={target_count}"
        );
    }
    if !pc_move_locations_share_container(source, target) {
        match target {
            RuntimePokemonStorageLocation::Party { .. }
                if target_count >= crystal_core::models::PARTY_SIZE =>
            {
                anyhow::bail!("PC move destination party is full");
            }
            RuntimePokemonStorageLocation::Box { .. } if target_count >= MAX_BOX_MONS => {
                anyhow::bail!("PC move destination box is full");
            }
            _ => {}
        }
    }
    Ok(())
}

fn pc_move_location_count(state: &GameState, location: &RuntimePokemonStorageLocation) -> usize {
    match location {
        RuntimePokemonStorageLocation::Party { .. } => state.storage.party.filled_slots(),
        RuntimePokemonStorageLocation::Box { box_index, .. } => {
            state.storage.pc_boxes[*box_index].count
        }
    }
}

fn pc_move_location_slot(location: &RuntimePokemonStorageLocation) -> usize {
    match location {
        RuntimePokemonStorageLocation::Party { slot }
        | RuntimePokemonStorageLocation::Box { slot, .. } => *slot,
    }
}

fn pc_move_locations_share_container(
    source: &RuntimePokemonStorageLocation,
    target: &RuntimePokemonStorageLocation,
) -> bool {
    match (source, target) {
        (
            RuntimePokemonStorageLocation::Party { .. },
            RuntimePokemonStorageLocation::Party { .. },
        ) => true,
        (
            RuntimePokemonStorageLocation::Box {
                box_index: source_box,
                ..
            },
            RuntimePokemonStorageLocation::Box {
                box_index: target_box,
                ..
            },
        ) => source_box == target_box,
        _ => false,
    }
}

fn adjust_pc_move_target_after_removal(
    source: &RuntimePokemonStorageLocation,
    target: RuntimePokemonStorageLocation,
) -> RuntimePokemonStorageLocation {
    if pc_move_locations_share_container(source, &target)
        && pc_move_location_slot(source) < pc_move_location_slot(&target)
    {
        match target {
            RuntimePokemonStorageLocation::Party { slot } => {
                RuntimePokemonStorageLocation::Party { slot: slot - 1 }
            }
            RuntimePokemonStorageLocation::Box { box_index, slot } => {
                RuntimePokemonStorageLocation::Box {
                    box_index,
                    slot: slot - 1,
                }
            }
        }
    } else {
        target
    }
}

fn insert_party_pokemon(party: &mut Party, slot: usize, pokemon: Pokemon) -> Result<()> {
    let count = party.filled_slots();
    if count >= crystal_core::models::PARTY_SIZE {
        anyhow::bail!("PC move destination party is full");
    }
    if slot > count {
        anyhow::bail!("party insertion slot {slot} is outside compact range 0..={count}");
    }
    for index in (slot..count).rev() {
        party.pokemon[index + 1] = party.pokemon[index].take();
    }
    party.pokemon[slot] = Some(pokemon);
    Ok(())
}

fn restore_deposited_pokemon_pp(
    moves: &BTreeMap<String, Move>,
    pokemon: &mut Pokemon,
) -> Result<()> {
    for learned in &mut pokemon.moves {
        let move_data = moves.get(&learned.name).with_context(|| {
            format!(
                "deposited Pokemon {} knows missing move {}",
                pokemon.species.id, learned.name
            )
        })?;
        learned.current_pp = crystal_core::models::max_move_pp(move_data.pp, learned.pp_ups);
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StandardScriptExecutionPath {
    CommonInterpreter,
}

fn standard_script_execution_path(
    script: &str,
    compiled_body: &[Value],
) -> Result<StandardScriptExecutionPath> {
    if compiled_body.is_empty() {
        anyhow::bail!("compiled standard script {script} has an empty command body");
    }
    for (command_index, command) in compiled_body.iter().enumerate() {
        let command_name = command
            .get("command")
            .and_then(Value::as_str)
            .with_context(|| {
                format!(
                    "compiled standard script {script} command {command_index} has no command name"
                )
            })?;
        if !standard_script_common_command_supported(command_name) {
            anyhow::bail!(
                "compiled standard script {script} has no executable runtime path: command {command_index} '{command_name}' is unsupported by the common interpreter"
            );
        }
    }
    Ok(StandardScriptExecutionPath::CommonInterpreter)
}

fn standard_script_common_command_supported(command: &str) -> bool {
    matches!(
        command,
        "applymovement"
            | "checkevent"
            | "checkflag"
            | "checkitem"
            | "checkphonecall"
            | "checktime"
            | "clearevent"
            | "clearflag"
            | "closetext"
            | "db"
            | "end"
            | "endcallback"
            | "faceplayer"
            | "farjumptext"
            | "farsjump"
            | "farwritetext"
            | "getcurlandmarkname"
            | "getnum"
            | "getstring"
            | "ifequal"
            | "iffalse"
            | "ifless"
            | "iftrue"
            | "opentext"
            | "pause"
            | "playmusic"
            | "playsound"
            | "promptbutton"
            | "readvar"
            | "scall"
            | "setevent"
            | "setflag"
            | "setmapscene"
            | "setval"
            | "sjump"
            | "special"
            | "specialphonecall"
            | "turnobject"
            | "variablesprite"
            | "verbosegiveitem"
            | "waitbutton"
            | "waitsfx"
            | "warp"
            | "yesorno"
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeMutationResult {
    OverworldInputApplied(OverworldInputFrame),
    ScriptItemGranted(ScriptItemGrantOutcome),
    ScriptItemChecked(ScriptItemCheckOutcome),
    ScriptItemTaken(ScriptItemTakeOutcome),
    ScriptFieldItemPickedUp(FieldItemPickupOutcome),
    ScriptEconomyApplied(ScriptEconomyOutcome),
    ScriptPhoneApplied(ScriptPhoneOutcome),
    ScriptFlagMutated(ScriptFlagMutationOutcome),
    ScriptFlagChecked(ScriptFlagCheckOutcome),
    ScriptSceneApplied(ScriptSceneOutcome),
    ScriptBlockChanged(ScriptBlockChangeOutcome),
    ScriptAudioApplied(ScriptAudioCue),
    ScriptMapApplied(ScriptMapAction),
    PendingScriptWarpTransitioned(ScriptWarpRequest),
    MapSetupCallbacksApplied(String),
    ScriptTextApplied(ScriptTextAction),
    ScriptVariableApplied(ScriptVariableOutcome),
    ScriptControlApplied(ScriptControlAction),
    ScriptObjectMutated(ScriptObjectMutationOutcome),
    ScriptMovementApplied(ScriptMovementOutcome),
    ScriptRuntimeApplied(ScriptRuntimeCommand, ScriptRuntimeOutcome),
    NextScriptTaken(ScriptLocation),
    ScriptEventQueueDrained(RuntimeScriptEventDrainResult),
    ScriptRuntimeQueueDrained(RuntimeScriptRuntimeQueueDrainResult),
    ScriptCallStackPopped(ScriptReturnFrame),
    DeferredScriptPopped(ScriptLocation),
    ScriptEndStateTaken(ScriptEndState),
    PendingScriptRequestTaken(RuntimePendingScriptRequest),
    PendingYesNoResolved(RuntimePendingYesNoResolution),
    VerticalMenuOpened(RuntimeVerticalMenuOpen),
    VerticalMenuOptionSelected(RuntimeVerticalMenuSelection),
    ElevatorFloorSelected(RuntimeElevatorFloorSelection),
    ScriptRuntimeFlagConsumed(RuntimeScriptRuntimeFlagValue),
    ScriptRuntimeMemoryValueTaken(RuntimeScriptRuntimeMemoryValueTaken),
    ScriptRuntimeMemoryEntryRemoved(RuntimeScriptRuntimeMemoryEntryRemoved),
    ScriptShopOpened(ScriptShopOutcome),
    ActiveMenuClosed(String),
    RuntimeWindowClosed,
    TextWindowClosed,
    ActivePokemonPictureClosed(String),
    ScriptShopClosed(crystal_core::state::ScriptShopRequest),
    ShopItemBought(ShopResult),
    ShopItemSold(ShopResult),
    SpecialRoutineApplied(SpecialRoutineOutcome),
    ScriptedGiftPokemonGranted(GiftPokemonOutcome),
    PartyPokemonAdded(GiftPokemonOutcome),
    ScriptedWildBattleStarted(StaticWildBattleStart),
    ScriptedTrainerBattleStarted(TrainerBattleStartStatus),
    MapTrainerInteractionResolved(RuntimeMapTrainerInteractionOutcome),
    ScriptedWildBattleCompleted,
    ScriptedTrainerBattleCompleted(TrainerBattleCompletionOutcome),
    BagItemUsed(ItemUseOutcome),
    PendingMoveLearnReplaced(PendingMoveLearnRuntimeResolution),
    PendingMoveLearnDeclined(PendingMoveLearnRuntimeResolution),
    KeyItemRegistered(RuntimeRegisteredKeyItemOutcome),
    FieldRepelUsed(FieldRepelItemUseOutcome),
    FieldBicycleUsed(FieldBicycleItemUseOutcome),
    FieldItemfinderUsed(FieldItemfinderUseOutcome),
    FieldSquirtbottleUsed(FieldSquirtBottleUseOutcome),
    FieldStoryKeyUsed(FieldStoryKeyUseOutcome),
    FieldCoinCaseUsed(FieldKeyItemBalanceUseOutcome),
    FieldBlueCardUsed(FieldKeyItemBalanceUseOutcome),
    FieldTownMapUsed(FieldTownMapUseOutcome),
    FieldPokegearUsed(FieldPokegearUseOutcome),
    FieldBoxUsed(FieldBoxItemUseOutcome),
    FieldEscapeRopeUsed(FieldEscapeRopeUseOutcome),
    CutFieldMoveUsed(FieldMoveBlockOutcome),
    WhirlpoolFieldMoveUsed(FieldMoveBlockOutcome),
    StrengthFromMenuQueued(RuntimeStrengthMenuOutcome),
    FlashFieldMoveUsed(FieldMoveFlagOutcome),
    SurfFieldMoveUsed(FieldMoveTravelOutcome),
    WaterfallFieldMoveUsed(FieldMoveTravelOutcome),
    FlyFieldMoveUsed(FlyFieldMoveOutcome),
    DigFieldMoveUsed(DigFieldMoveOutcome),
    TeleportFieldMoveUsed(TeleportFieldMoveOutcome),
    FieldTravelCommitted(PendingFieldTravel),
    HeadbuttScriptQueued(RuntimeHeadbuttScriptOutcome),
    RockSmashFromMenuQueued(RuntimeRockSmashMenuOutcome),
    RockMonEncounterResolved(RockMonEncounterOutcome),
    TreeMonEncounterResolved(HeadbuttEncounterOutcome),
    SweetScentFromMenuQueued(RuntimeSweetScentMenuOutcome),
    SweetScentEncounterResolved(SweetScentEncounterOutcome),
    PartyPokemonItemUsed(ItemUseOutcome, BattleItemOutcome),
    WholePartyItemUsed(ItemUseOutcome, PartyItemOutcome),
    PartyMoveItemUsed(ItemUseOutcome, BattleItemOutcome),
    TmHmItemUsed(ItemUseOutcome, TmHmLearnOutcome),
    ActiveBattlePokemonItemUsed(ItemUseOutcome, BattleItemOutcome),
    BattlePartyPokemonItemUsed(ItemUseOutcome, BattleItemOutcome),
    BattlePartyMoveItemUsed(ItemUseOutcome, BattleItemOutcome),
    BallThrown(CaptureOutcome),
    ActiveWildCaptureCompleted(CaptureCompletion),
    ActiveBattlePartySwitched(ActiveBattlePartySwitchOutcome),
    ActiveBattleTurnResolved(BattleTurnOutcome),
    ActiveBattleCommandResolved(ActiveBattleCommandOutcome),
    ActiveBattleEnemyActionResolved(BattleTurnOutcome),
    ActiveWildBattleEscapeAttempted(BattleEscapeAttempt),
    ActiveWildBattleEscapeItemUsed(BattleEscapeItemUseOutcome),
    ActiveBattleGuardSpecUsed(BattleStateItemUseOutcome),
    ActiveTrainerBattleAdvanced(TrainerBattleAdvanceOutcome),
    ActiveTrainerBattleRewardsClaimed(BattleRewardOutcome),
    ActiveWildBattleRewardsClaimed(BattleRewardOutcome),
    FishingRodCast(FishingCastOutcome),
    BagFishingRodUsed(FishingRodItemUseOutcome),
    GameTimerVBlanksAdvanced(RuntimeGameTimerOutcome),
    GameTimerCountingSet(RuntimeGameTimerOutcome),
    GameLogicPauseSet(RuntimeGameTimerOutcome),
    ClockUpdated,
    ManualClockSet,
    ScriptSwarmApplied(ScriptSwarmOutcome),
    QueuedScriptCommandExecuted(ScriptRuntimeQueuedCommand),
    DayCareUsed(SpecialRoutineOutcome),
    DayCareManOutsideChecked(SpecialRoutineOutcome),
    DayCareResidentChecked(SpecialRoutineOutcome),
    BugContestUsed(SpecialRoutineOutcome),
    KurtApricornUsed(SpecialRoutineOutcome),
    BuenaPasswordUsed(SpecialRoutineOutcome),
    BuenaPrizeUsed(SpecialRoutineOutcome),
    ShuckieUsed(SpecialRoutineOutcome),
    OddEggGiven(SpecialRoutineOutcome),
    DratiniGiven(SpecialRoutineOutcome),
    BillsGrandfatherUsed(SpecialRoutineOutcome),
    RoamersInitialized(SpecialRoutineOutcome),
    MagikarpLengthChecked(SpecialRoutineOutcome),
    ProfOaksPcBootShown(SpecialRoutineOutcome),
    MagikarpHouseSignShown(SpecialRoutineOutcome),
    BattleTowerActionApplied(SpecialRoutineOutcome),
    BattleTowerRoomMenuUsed(SpecialRoutineOutcome),
    BattleTowerBattleStarted(SpecialRoutineOutcome),
    BattleTowerOpponentLoaded(SpecialRoutineOutcome),
    BattleTowerMobileErrorShown(SpecialRoutineOutcome),
    RememberPasswordAsked(SpecialRoutineOutcome),
    BattleTowerLeaderboardOpened(SpecialRoutineOutcome),
    MobileHandshakeApplied(SpecialRoutineOutcome),
    MobileSessionEnded(SpecialRoutineOutcome),
    BattleTowerMobileFlagSet(SpecialRoutineOutcome),
    MobileThreeMonsSelected(SpecialRoutineOutcome),
    HappinessServiceApplied(SpecialRoutineOutcome),
    MysteryGiftUsed(SpecialRoutineOutcome),
    SpawnPointWarped(SpecialRoutineOutcome),
    PartyHealedBySpecial(SpecialRoutineOutcome),
    MusicFadedOutBySpecial(SpecialRoutineOutcome),
    SoundEffectWaitQueued(SpecialRoutineOutcome),
    MapMusicPlayedBySpecial(SpecialRoutineOutcome),
    MapMusicRestartedBySpecial(SpecialRoutineOutcome),
    CurrentMonCryPlayed(SpecialRoutineOutcome),
    SlowCryPlayed(SpecialRoutineOutcome),
    PokemonCenterPcOpened(SpecialRoutineOutcome),
    PlayersHousePcOpened(SpecialRoutineOutcome),
    OverworldTownMapOpened(SpecialRoutineOutcome),
    UnownPrinterOpened(SpecialRoutineOutcome),
    MapRadioOpened(SpecialRoutineOutcome),
    RivalNamed(SpecialRoutineOutcome),
    PartyMoveDeletedBySpecial(SpecialRoutineOutcome),
    PokerusChecked(SpecialRoutineOutcome),
    PartyNicknameRated(SpecialRoutineOutcome),
    PartyPokemonSeenBySeer(SpecialRoutineOutcome),
    PartyMoveTaughtBySpecial(SpecialRoutineOutcome),
    BankOfMomOpened(SpecialRoutineOutcome),
    GameCornerOpened(SpecialRoutineOutcome),
    DisplayLinkRecordOpened(SpecialRoutineOutcome),
    TrainerHouseOpened(SpecialRoutineOutcome),
    PhotoStudioOpened(SpecialRoutineOutcome),
    BattleTowerChallengeMenuUsed(SpecialRoutineOutcome),
    GraphicsSpecialApplied(SpecialRoutineOutcome),
    PartyCheckSpecialApplied(SpecialRoutineOutcome),
    PhoneRandomSpecialApplied(SpecialRoutineOutcome),
    ItemInPcOrBagChecked(SpecialRoutineOutcome),
    AnotherUsablePartyMonChecked(SpecialRoutineOutcome),
    FishingSwarmActivated(SpecialRoutineOutcome),
    StoryGateSpecialApplied(SpecialRoutineOutcome),
    PlayerPaletteSet(SpecialRoutineOutcome),
    DayOfWeekSet(SpecialRoutineOutcome),
    TimeUpdated(SpecialRoutineOutcome),
    CurrentPcBoxSwitched(RuntimeStorageBoxSwitchOutcome),
    PcBoxNamed(RuntimeStorageBoxNameOutcome),
    PartyPokemonDeposited(RuntimeStorageDepositOutcome),
    PcPokemonWithdrawn(RuntimeStorageWithdrawOutcome),
    PcPokemonReleased(RuntimeStorageReleaseOutcome),
    PcPokemonMoved(RuntimeStorageMoveOutcome),
    BagItemDepositedToPc(RuntimePcItemTransferOutcome),
    PcItemWithdrawnToBag(RuntimePcItemTransferOutcome),
    PcItemTossed(RuntimePcItemTransferOutcome),
    DecorationSetUp(DecorationActionOutcome),
    DecorationPutAway(DecorationActionOutcome),
    PartyPokemonHeldItemGiven(RuntimeHeldItemTransferOutcome),
    PartyMailComposed(RuntimeMailTransferOutcome),
    PartyPokemonHeldItemTaken(RuntimeHeldItemTransferOutcome),
    PartyMailSentToMailbox(RuntimeMailTransferOutcome),
    PartyMailDiscardedToBag(RuntimeMailTransferOutcome),
    MailboxMailDeleted(RuntimeMailTransferOutcome),
    MailboxMailMovedToBag(RuntimeMailTransferOutcome),
    MailboxMailAttachedToParty(RuntimeMailTransferOutcome),
    BadgeAwarded(RuntimeBadgeAwardOutcome),
    PokedexSeenRecorded(RuntimePokedexRecordOutcome),
    PokedexCaughtRecorded(RuntimePokedexRecordOutcome),
    BagItemAdded(RuntimeBagItemMutationOutcome),
    CurrencyAdded(RuntimeCurrencyMutationOutcome),
    CurrencyTaken(RuntimeCurrencyMutationOutcome),
    LinkBattleResultRecorded(RuntimeLinkBattleRecordOutcome),
    CableClubRequestSet(SpecialRoutineOutcome),
    LinkedFriendWaitedFor(SpecialRoutineOutcome),
    LinkTimeoutReceptionistChecked(SpecialRoutineOutcome),
    BothSelectedSameRoomChecked(SpecialRoutineOutcome),
    LinkClosed(SpecialRoutineOutcome),
    OtherPlayerExitWaitedFor(SpecialRoutineOutcome),
    LinkToPastFailed(SpecialRoutineOutcome),
    LinkRoomOpened(SpecialRoutineOutcome),
    TimeCapsuleCompatibilityChecked(SpecialRoutineOutcome),
    QuickSaveTried(SpecialRoutineOutcome),
    MobileOrCableAsked(SpecialRoutineOutcome),
    CableClubChrisChecked(SpecialRoutineOutcome),
    OptionsSet(RuntimeOptionsSetOutcome),
    PokegearRadioTuningSet(RuntimePokegearRadioTuningOutcome),
    TrainerIdentitySet(RuntimeTrainerIdentityOutcome),
    PlayerGenderSet(RuntimePlayerGenderOutcome),
    PartyPokemonRenamed(RuntimePartyNicknameOutcome),
    StoredPokemonRenamed(RuntimeStoredPokemonNicknameOutcome),
    PartyPokemonRecoveryStateSet(RuntimePartyRecoverySetupOutcome),
    PartyPokemonHpTransferred(RuntimePartyHpTransferOutcome),
    PartyPokemonFullHealed(PartyRecoveryOutcome),
    WholePartyFullHealed(Vec<PartyRecoveryOutcome>),
    BlackoutResolved(BlackoutRecoveryOutcome),
    PartyPokemonSwapped(RuntimePartySwapOutcome),
    PartyPokemonMovesSwapped(RuntimePartyMoveSwapOutcome),
    PermanentPhoneNumbersInitialized(Vec<String>),
    PokegearPhoneCallStarted(RuntimePokegearPhoneCallOutcome),
}

impl RuntimeMutationResult {
    pub const fn result_tag(&self) -> &'static str {
        match self {
            Self::OverworldInputApplied(_) => "overworld_input_applied",
            Self::ScriptItemGranted(_) => "script_item_granted",
            Self::ScriptItemChecked(_) => "script_item_checked",
            Self::ScriptItemTaken(_) => "script_item_taken",
            Self::ScriptFieldItemPickedUp(_) => "script_field_item_picked_up",
            Self::ScriptEconomyApplied(_) => "script_economy_applied",
            Self::ScriptPhoneApplied(_) => "script_phone_applied",
            Self::ScriptFlagMutated(_) => "script_flag_mutated",
            Self::ScriptFlagChecked(_) => "script_flag_checked",
            Self::ScriptSceneApplied(_) => "script_scene_applied",
            Self::ScriptBlockChanged(_) => "script_block_changed",
            Self::ScriptAudioApplied(_) => "script_audio_applied",
            Self::ScriptMapApplied(_) => "script_map_applied",
            Self::PendingScriptWarpTransitioned(_) => "pending_script_warp_transitioned",
            Self::MapSetupCallbacksApplied(_) => "map_setup_callbacks_applied",
            Self::ScriptTextApplied(_) => "script_text_applied",
            Self::ScriptVariableApplied(_) => "script_variable_applied",
            Self::ScriptControlApplied(_) => "script_control_applied",
            Self::ScriptObjectMutated(_) => "script_object_mutated",
            Self::ScriptMovementApplied(_) => "script_movement_applied",
            Self::ScriptRuntimeApplied(_, _) => "script_runtime_applied",
            Self::NextScriptTaken(_) => "next_script_taken",
            Self::ScriptEventQueueDrained(_) => "script_event_queue_drained",
            Self::ScriptRuntimeQueueDrained(_) => "script_runtime_queue_drained",
            Self::ScriptCallStackPopped(_) => "script_call_stack_popped",
            Self::DeferredScriptPopped(_) => "deferred_script_popped",
            Self::ScriptEndStateTaken(_) => "script_end_state_taken",
            Self::PendingScriptRequestTaken(_) => "pending_script_request_taken",
            Self::PendingYesNoResolved(_) => "pending_yes_no_resolved",
            Self::VerticalMenuOpened(_) => "vertical_menu_opened",
            Self::VerticalMenuOptionSelected(_) => "vertical_menu_option_selected",
            Self::ElevatorFloorSelected(_) => "elevator_floor_selected",
            Self::ScriptRuntimeFlagConsumed(_) => "script_runtime_flag_consumed",
            Self::ScriptRuntimeMemoryValueTaken(_) => "script_runtime_memory_value_taken",
            Self::ScriptRuntimeMemoryEntryRemoved(_) => "script_runtime_memory_entry_removed",
            Self::ScriptShopOpened(_) => "script_shop_opened",
            Self::ActiveMenuClosed(_) => "active_menu_closed",
            Self::RuntimeWindowClosed => "runtime_window_closed",
            Self::TextWindowClosed => "text_window_closed",
            Self::ActivePokemonPictureClosed(_) => "active_pokemon_picture_closed",
            Self::ScriptShopClosed(_) => "script_shop_closed",
            Self::ShopItemBought(_) => "shop_item_bought",
            Self::ShopItemSold(_) => "shop_item_sold",
            Self::SpecialRoutineApplied(_) => "special_routine_applied",
            Self::ScriptedGiftPokemonGranted(_) => "scripted_gift_pokemon_granted",
            Self::PartyPokemonAdded(_) => "party_pokemon_added",
            Self::ScriptedWildBattleStarted(_) => "scripted_wild_battle_started",
            Self::ScriptedTrainerBattleStarted(_) => "scripted_trainer_battle_started",
            Self::MapTrainerInteractionResolved(_) => "map_trainer_interaction_resolved",
            Self::ScriptedWildBattleCompleted => "scripted_wild_battle_completed",
            Self::ScriptedTrainerBattleCompleted(_) => "scripted_trainer_battle_completed",
            Self::BagItemUsed(_) => "bag_item_used",
            Self::PendingMoveLearnReplaced(_) => "pending_move_learn_replaced",
            Self::PendingMoveLearnDeclined(_) => "pending_move_learn_declined",
            Self::KeyItemRegistered(_) => "key_item_registered",
            Self::FieldRepelUsed(_) => "field_repel_used",
            Self::FieldBicycleUsed(_) => "field_bicycle_used",
            Self::FieldItemfinderUsed(_) => "field_itemfinder_used",
            Self::FieldSquirtbottleUsed(_) => "field_squirtbottle_used",
            Self::FieldStoryKeyUsed(_) => "field_story_key_used",
            Self::FieldCoinCaseUsed(_) => "field_coin_case_used",
            Self::FieldBlueCardUsed(_) => "field_blue_card_used",
            Self::FieldTownMapUsed(_) => "field_town_map_used",
            Self::FieldPokegearUsed(_) => "field_pokegear_used",
            Self::FieldBoxUsed(_) => "field_box_used",
            Self::FieldEscapeRopeUsed(_) => "field_escape_rope_used",
            Self::CutFieldMoveUsed(_) => "cut_field_move_used",
            Self::WhirlpoolFieldMoveUsed(_) => "whirlpool_field_move_used",
            Self::StrengthFromMenuQueued(_) => "strength_from_menu_queued",
            Self::FlashFieldMoveUsed(_) => "flash_field_move_used",
            Self::SurfFieldMoveUsed(_) => "surf_field_move_used",
            Self::WaterfallFieldMoveUsed(_) => "waterfall_field_move_used",
            Self::FlyFieldMoveUsed(_) => "fly_field_move_used",
            Self::DigFieldMoveUsed(_) => "dig_field_move_used",
            Self::TeleportFieldMoveUsed(_) => "teleport_field_move_used",
            Self::FieldTravelCommitted(_) => "field_travel_committed",
            Self::HeadbuttScriptQueued(_) => "headbutt_script_queued",
            Self::RockSmashFromMenuQueued(_) => "rock_smash_from_menu_queued",
            Self::RockMonEncounterResolved(_) => "rock_mon_encounter_resolved",
            Self::TreeMonEncounterResolved(_) => "tree_mon_encounter_resolved",
            Self::SweetScentFromMenuQueued(_) => "sweet_scent_from_menu_queued",
            Self::SweetScentEncounterResolved(_) => "sweet_scent_encounter_resolved",
            Self::PartyPokemonItemUsed(_, _) => "party_pokemon_item_used",
            Self::WholePartyItemUsed(_, _) => "whole_party_item_used",
            Self::PartyMoveItemUsed(_, _) => "party_move_item_used",
            Self::TmHmItemUsed(_, _) => "tm_hm_item_used",
            Self::ActiveBattlePokemonItemUsed(_, _) => "active_battle_pokemon_item_used",
            Self::BattlePartyPokemonItemUsed(_, _) => "battle_party_pokemon_item_used",
            Self::BattlePartyMoveItemUsed(_, _) => "battle_party_move_item_used",
            Self::BallThrown(_) => "ball_thrown",
            Self::ActiveWildCaptureCompleted(_) => "active_wild_capture_completed",
            Self::ActiveBattlePartySwitched(_) => "active_battle_party_switched",
            Self::ActiveBattleTurnResolved(_) => "active_battle_turn_resolved",
            Self::ActiveBattleCommandResolved(_) => "active_battle_command_resolved",
            Self::ActiveBattleEnemyActionResolved(_) => "active_battle_enemy_action_resolved",
            Self::ActiveWildBattleEscapeAttempted(_) => "active_wild_battle_escape_attempted",
            Self::ActiveWildBattleEscapeItemUsed(_) => "active_wild_battle_escape_item_used",
            Self::ActiveBattleGuardSpecUsed(_) => "active_battle_guard_spec_used",
            Self::ActiveTrainerBattleAdvanced(_) => "active_trainer_battle_advanced",
            Self::ActiveTrainerBattleRewardsClaimed(_) => "active_trainer_battle_rewards_claimed",
            Self::ActiveWildBattleRewardsClaimed(_) => "active_wild_battle_rewards_claimed",
            Self::FishingRodCast(_) => "fishing_rod_cast",
            Self::BagFishingRodUsed(_) => "bag_fishing_rod_used",
            Self::GameTimerVBlanksAdvanced(_) => "game_timer_vblanks_advanced",
            Self::GameTimerCountingSet(_) => "game_timer_counting_set",
            Self::GameLogicPauseSet(_) => "game_logic_pause_set",
            Self::ClockUpdated => "clock_updated",
            Self::ManualClockSet => "manual_clock_set",
            Self::ScriptSwarmApplied(_) => "script_swarm_applied",
            Self::QueuedScriptCommandExecuted(_) => "queued_script_command_executed",
            Self::DayCareUsed(_) => "day_care_used",
            Self::DayCareManOutsideChecked(_) => "day_care_man_outside_checked",
            Self::DayCareResidentChecked(_) => "day_care_resident_checked",
            Self::BugContestUsed(_) => "bug_contest_used",
            Self::KurtApricornUsed(_) => "kurt_apricorn_used",
            Self::BuenaPasswordUsed(_) => "buena_password_used",
            Self::BuenaPrizeUsed(_) => "buena_prize_used",
            Self::ShuckieUsed(_) => "shuckie_used",
            Self::OddEggGiven(_) => "odd_egg_given",
            Self::DratiniGiven(_) => "dratini_given",
            Self::BillsGrandfatherUsed(_) => "bills_grandfather_used",
            Self::RoamersInitialized(_) => "roamers_initialized",
            Self::MagikarpLengthChecked(_) => "magikarp_length_checked",
            Self::ProfOaksPcBootShown(_) => "prof_oaks_pc_boot_shown",
            Self::MagikarpHouseSignShown(_) => "magikarp_house_sign_shown",
            Self::BattleTowerActionApplied(_) => "battle_tower_action_applied",
            Self::BattleTowerRoomMenuUsed(_) => "battle_tower_room_menu_used",
            Self::BattleTowerBattleStarted(_) => "battle_tower_battle_started",
            Self::BattleTowerOpponentLoaded(_) => "battle_tower_opponent_loaded",
            Self::BattleTowerMobileErrorShown(_) => "battle_tower_mobile_error_shown",
            Self::RememberPasswordAsked(_) => "remember_password_asked",
            Self::BattleTowerLeaderboardOpened(_) => "battle_tower_leaderboard_opened",
            Self::MobileHandshakeApplied(_) => "mobile_handshake_applied",
            Self::MobileSessionEnded(_) => "mobile_session_ended",
            Self::BattleTowerMobileFlagSet(_) => "battle_tower_mobile_flag_set",
            Self::MobileThreeMonsSelected(_) => "mobile_three_mons_selected",
            Self::HappinessServiceApplied(_) => "happiness_service_applied",
            Self::MysteryGiftUsed(_) => "mystery_gift_used",
            Self::SpawnPointWarped(_) => "spawn_point_warped",
            Self::PartyHealedBySpecial(_) => "party_healed_by_special",
            Self::MusicFadedOutBySpecial(_) => "music_faded_out_by_special",
            Self::SoundEffectWaitQueued(_) => "sound_effect_wait_queued",
            Self::MapMusicPlayedBySpecial(_) => "map_music_played_by_special",
            Self::MapMusicRestartedBySpecial(_) => "map_music_restarted_by_special",
            Self::CurrentMonCryPlayed(_) => "current_mon_cry_played",
            Self::SlowCryPlayed(_) => "slow_cry_played",
            Self::PokemonCenterPcOpened(_) => "pokemon_center_pc_opened",
            Self::PlayersHousePcOpened(_) => "players_house_pc_opened",
            Self::OverworldTownMapOpened(_) => "overworld_town_map_opened",
            Self::UnownPrinterOpened(_) => "unown_printer_opened",
            Self::MapRadioOpened(_) => "map_radio_opened",
            Self::RivalNamed(_) => "rival_named",
            Self::PartyMoveDeletedBySpecial(_) => "party_move_deleted_by_special",
            Self::PokerusChecked(_) => "pokerus_checked",
            Self::PartyNicknameRated(_) => "party_nickname_rated",
            Self::PartyPokemonSeenBySeer(_) => "party_pokemon_seen_by_seer",
            Self::PartyMoveTaughtBySpecial(_) => "party_move_taught_by_special",
            Self::BankOfMomOpened(_) => "bank_of_mom_opened",
            Self::GameCornerOpened(_) => "game_corner_opened",
            Self::DisplayLinkRecordOpened(_) => "display_link_record_opened",
            Self::TrainerHouseOpened(_) => "trainer_house_opened",
            Self::PhotoStudioOpened(_) => "photo_studio_opened",
            Self::BattleTowerChallengeMenuUsed(_) => "battle_tower_challenge_menu_used",
            Self::GraphicsSpecialApplied(_) => "graphics_special_applied",
            Self::PartyCheckSpecialApplied(_) => "party_check_special_applied",
            Self::PhoneRandomSpecialApplied(_) => "phone_random_special_applied",
            Self::ItemInPcOrBagChecked(_) => "item_in_pc_or_bag_checked",
            Self::AnotherUsablePartyMonChecked(_) => "another_usable_party_mon_checked",
            Self::FishingSwarmActivated(_) => "fishing_swarm_activated",
            Self::StoryGateSpecialApplied(_) => "story_gate_special_applied",
            Self::PlayerPaletteSet(_) => "player_palette_set",
            Self::DayOfWeekSet(_) => "day_of_week_set",
            Self::TimeUpdated(_) => "time_updated",
            Self::CurrentPcBoxSwitched(_) => "current_pc_box_switched",
            Self::PcBoxNamed(_) => "pc_box_named",
            Self::PartyPokemonDeposited(_) => "party_pokemon_deposited",
            Self::PcPokemonWithdrawn(_) => "pc_pokemon_withdrawn",
            Self::PcPokemonReleased(_) => "pc_pokemon_released",
            Self::PcPokemonMoved(_) => "pc_pokemon_moved",
            Self::BagItemDepositedToPc(_) => "bag_item_deposited_to_pc",
            Self::PcItemWithdrawnToBag(_) => "pc_item_withdrawn_to_bag",
            Self::PcItemTossed(_) => "pc_item_tossed",
            Self::DecorationSetUp(_) => "decoration_set_up",
            Self::DecorationPutAway(_) => "decoration_put_away",
            Self::PartyPokemonHeldItemGiven(_) => "party_pokemon_held_item_given",
            Self::PartyMailComposed(_) => "party_mail_composed",
            Self::PartyPokemonHeldItemTaken(_) => "party_pokemon_held_item_taken",
            Self::PartyMailSentToMailbox(_) => "party_mail_sent_to_mailbox",
            Self::PartyMailDiscardedToBag(_) => "party_mail_discarded_to_bag",
            Self::MailboxMailDeleted(_) => "mailbox_mail_deleted",
            Self::MailboxMailMovedToBag(_) => "mailbox_mail_moved_to_bag",
            Self::MailboxMailAttachedToParty(_) => "mailbox_mail_attached_to_party",
            Self::BadgeAwarded(_) => "badge_awarded",
            Self::PokedexSeenRecorded(_) => "pokedex_seen_recorded",
            Self::PokedexCaughtRecorded(_) => "pokedex_caught_recorded",
            Self::BagItemAdded(_) => "bag_item_added",
            Self::CurrencyAdded(_) => "currency_added",
            Self::CurrencyTaken(_) => "currency_taken",
            Self::LinkBattleResultRecorded(_) => "link_battle_result_recorded",
            Self::CableClubRequestSet(_) => "cable_club_request_set",
            Self::LinkedFriendWaitedFor(_) => "linked_friend_waited_for",
            Self::LinkTimeoutReceptionistChecked(_) => "link_timeout_receptionist_checked",
            Self::BothSelectedSameRoomChecked(_) => "both_selected_same_room_checked",
            Self::LinkClosed(_) => "link_closed",
            Self::OtherPlayerExitWaitedFor(_) => "other_player_exit_waited_for",
            Self::LinkToPastFailed(_) => "link_to_past_failed",
            Self::LinkRoomOpened(_) => "link_room_opened",
            Self::TimeCapsuleCompatibilityChecked(_) => "time_capsule_compatibility_checked",
            Self::QuickSaveTried(_) => "quick_save_tried",
            Self::MobileOrCableAsked(_) => "mobile_or_cable_asked",
            Self::CableClubChrisChecked(_) => "cable_club_chris_checked",
            Self::OptionsSet(_) => "options_set",
            Self::PokegearRadioTuningSet(_) => "pokegear_radio_tuning_set",
            Self::TrainerIdentitySet(_) => "trainer_identity_set",
            Self::PlayerGenderSet(_) => "player_gender_set",
            Self::PartyPokemonRenamed(_) => "party_pokemon_renamed",
            Self::StoredPokemonRenamed(_) => "stored_pokemon_renamed",
            Self::PartyPokemonRecoveryStateSet(_) => "party_pokemon_recovery_state_set",
            Self::PartyPokemonHpTransferred(_) => "party_pokemon_hp_transferred",
            Self::PartyPokemonFullHealed(_) => "party_pokemon_full_healed",
            Self::WholePartyFullHealed(_) => "whole_party_full_healed",
            Self::BlackoutResolved(_) => "blackout_resolved",
            Self::PartyPokemonSwapped(_) => "party_pokemon_swapped",
            Self::PartyPokemonMovesSwapped(_) => "party_pokemon_moves_swapped",
            Self::PermanentPhoneNumbersInitialized(_) => "permanent_phone_numbers_initialized",
            Self::PokegearPhoneCallStarted(_) => "pokegear_phone_call_started",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeMutationOutcome {
    pub result: RuntimeMutationResult,
    pub state_checksum: StateChecksum,
}

pub fn runtime_mutation_result_frame(
    request: RuntimeCommandFrame,
    outcome: &RuntimeMutationOutcome,
    state: &GameState,
) -> Result<RuntimeCommandResultFrame> {
    let checksum = StateChecksumFrame::from_game_state(request.player_id(), state)
        .context("checksum runtime mutation result state")?;
    if checksum.checksum() != outcome.state_checksum {
        anyhow::bail!("runtime mutation outcome checksum does not match authoritative state");
    }
    RuntimeCommandResultFrame::new(request, checksum, outcome.result.result_tag())
        .map_err(|error| anyhow::anyhow!("build runtime mutation result frame: {error}"))
}

fn initial_tmhm_flags(items: &BTreeMap<String, Item>) -> Vec<u8> {
    items
        .values()
        .filter_map(|item| item.tmhm_index)
        .max()
        .map(|max_index| vec![0; max_index + 1])
        .unwrap_or_default()
}

fn set_script_battle_result_accumulator(state: &mut GameState) {
    const BATTLE_RESULT_SCRIPT_FLAG_MASK: u8 = (1 << 6) | (1 << 7);
    let result_code = state.battle_result & !BATTLE_RESULT_SCRIPT_FLAG_MASK;
    let value = result_code.to_string();
    state.script_runtime.script_value = Some(value.clone());
    state
        .script_runtime
        .variables
        .insert("_value".to_string(), value);
}

fn set_running_trainer_battle_script(state: &mut GameState, running: bool) {
    state.script_runtime.memory.insert(
        "wRunningTrainerBattleScript".to_string(),
        if running { "-1" } else { "0" }.to_string(),
    );
}

fn clear_transient_map_object_context(state: &mut GameState, session: &mut OverworldSession) {
    state.script_runtime.last_talked_object = None;
    session.last_talked_object_identifier = None;
}

fn initialize_loaded_object_roster(session: &mut OverworldSession, state: &GameState) {
    // Crystal builds the map-object roster once from the current time and
    // event flags when entering a map. Later flag mutations deliberately keep
    // that loaded roster until an explicit object command or another entry.
    session.set_time_of_day(state.time.time_of_day);
    session.sync_event_flag_memory(&state.flags);
}

fn reset_map_bike_flags(state: &mut GameState) -> Result<()> {
    state.script_runtime.stone_table_entries.clear();
    for queue_slot in 0..4 {
        state
            .script_runtime
            .memory
            .insert(format!("wCmdQueueType{queue_slot}"), "0".to_string());
    }
    for flag in [
        "ENGINE_STRENGTH_ACTIVE",
        "ENGINE_ALWAYS_ON_BIKE",
        "ENGINE_DOWNHILL",
    ] {
        state
            .flags
            .set_engine_flag(flag, false)
            .map_err(|error| anyhow::anyhow!("reset map bike flag {flag}: {error}"))?;
    }
    Ok(())
}

fn movement_mode_script_byte(mode: MovementMode) -> u8 {
    match mode {
        MovementMode::Normal => 0,
        MovementMode::Bike => 1,
        MovementMode::Skate => 2,
        MovementMode::Surf => 4,
        MovementMode::SurfPika => 8,
    }
}

fn movement_mode_from_script_value(value: &str) -> Result<MovementMode> {
    let numeric = match value {
        "PLAYER_NORMAL" => 0,
        "PLAYER_BIKE" => 1,
        "PLAYER_SKATE" => 2,
        "PLAYER_SURF" => 4,
        "PLAYER_SURF_PIKA" => 8,
        _ => parse_script_i32(value)
            .with_context(|| format!("VAR_MOVEMENT value {value:?} is not an exact source byte"))?,
    };
    match numeric {
        0 => Ok(MovementMode::Normal),
        1 => Ok(MovementMode::Bike),
        2 => Ok(MovementMode::Skate),
        4 => Ok(MovementMode::Surf),
        8 => Ok(MovementMode::SurfPika),
        _ => anyhow::bail!(
            "VAR_MOVEMENT value {value:?} resolves to unsupported wPlayerState byte {numeric}"
        ),
    }
}

pub fn runtime_special_routine_requires_divider_trace(routine: &str) -> bool {
    matches!(
        routine,
        "SampleKenjiBreakCountdown"
            | "BattleTowerAction"
            | "ResetLuckyNumberShowFlag"
            | "RandomUnseenWildMon"
            | "RandomPhoneWildMon"
            | "RandomPhoneMon"
            | "UnownPuzzle"
            | "SelectRandomBugContestContestants"
            | "BugContestJudging"
            | "CardFlip"
            | "SlotMachine"
            | "UnusedMemoryGame"
            | "MemoryGame"
            | "DayCareMan"
            | "DayCareLady"
            | "DayCareManOutside"
            | "GiveShuckle"
            | "BuenasPassword"
            | "LoadOpponentTrainerAndPokemonWithOTSprite"
            | "GiveOddEgg"
    )
}

fn compiled_standard_script_catalog(data: &GameDataSet) -> Result<&serde_json::Map<String, Value>> {
    let mut standard_scripts = data.story_events.iter().filter_map(|payload| {
        payload
            .as_object()
            .and_then(|payload| payload.get("StandardScripts"))
            .and_then(Value::as_object)
    });
    let catalog = standard_scripts
        .next()
        .context("compiled game pack is missing the StandardScripts story-event catalog")?;
    if standard_scripts.next().is_some() {
        anyhow::bail!("compiled game pack contains duplicate StandardScripts catalogs");
    }
    Ok(catalog)
}

fn validate_compiled_standard_script_catalog(data: &GameDataSet) -> Result<()> {
    let catalog = compiled_standard_script_catalog(data)?;
    let pointer_table = catalog
        .get("StdScripts")
        .and_then(Value::as_array)
        .context("compiled StandardScripts catalog is missing the StdScripts pointer table")?;
    if pointer_table.is_empty() {
        anyhow::bail!("compiled StandardScripts pointer table is empty");
    }
    let mut labels = BTreeSet::new();
    for (index, entry) in pointer_table.iter().enumerate() {
        if entry.get("command").and_then(Value::as_str) != Some("add_stdscript") {
            anyhow::bail!("compiled StdScripts pointer {index} is not an add_stdscript command");
        }
        let args = entry
            .get("args")
            .and_then(Value::as_array)
            .with_context(|| {
                format!("compiled StdScripts pointer {index} args are not an array")
            })?;
        if args.len() != 1 {
            anyhow::bail!(
                "compiled StdScripts pointer {index} requires exactly one label, found {}",
                args.len()
            );
        }
        let label = args[0]
            .as_str()
            .filter(|label| !label.is_empty())
            .with_context(|| format!("compiled StdScripts pointer {index} has an invalid label"))?;
        if !labels.insert(label) {
            anyhow::bail!("compiled StdScripts pointer table repeats {label}");
        }
        let body = catalog
            .get(label)
            .and_then(Value::as_array)
            .with_context(|| format!("compiled StdScripts pointer {label} has no command body"))?;
        if body.is_empty() {
            anyhow::bail!("compiled StdScripts pointer {label} has an empty command body");
        }
        standard_script_execution_path(label, body)?;
    }
    Ok(())
}

fn compiled_overworld_event_catalog(data: &GameDataSet) -> Result<&serde_json::Map<String, Value>> {
    let mut overworld_events = data.story_events.iter().filter_map(|payload| {
        payload
            .as_object()
            .and_then(|payload| payload.get("OverworldEvents"))
            .and_then(Value::as_object)
    });
    let catalog = overworld_events
        .next()
        .context("compiled game pack is missing the OverworldEvents story-event catalog")?;
    if overworld_events.next().is_some() {
        anyhow::bail!("compiled game pack contains duplicate OverworldEvents catalogs");
    }
    Ok(catalog)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlayerEventExecutionPath {
    CommonInterpreter,
    TypedConsumer,
}

const PLAYER_EVENT_POINTER_LABELS: &[&str] = &[
    "InvalidEventScript",
    "SeenByTrainerScript",
    "TalkToTrainerScript",
    "FindItemInBallScript",
    "EdgeWarpScript",
    "WarpToNewMapScript",
    "FallIntoMapScript",
    "OverworldWhiteoutScript",
    "HatchEggScript",
    "ChangeDirectionScript",
    "InvalidEventScript",
];

fn exact_player_event_pointer_labels(pointer_table: &[Value]) -> Result<Vec<String>> {
    let pointer_entries = pointer_table
        .iter()
        .filter(|entry| entry.get("command").and_then(Value::as_str) == Some("dba"))
        .collect::<Vec<_>>();
    if pointer_entries.len() != PLAYER_EVENT_POINTER_LABELS.len() {
        anyhow::bail!(
            "compiled PlayerEventScriptPointers requires exactly {} dba pointers, found {}",
            PLAYER_EVENT_POINTER_LABELS.len(),
            pointer_entries.len()
        );
    }

    pointer_entries
        .iter()
        .zip(PLAYER_EVENT_POINTER_LABELS)
        .enumerate()
        .map(|(index, (&entry, expected_label))| {
            let command = entry
                .get("command")
                .and_then(Value::as_str)
                .unwrap_or("<invalid>");
            let args = entry.get("args").and_then(Value::as_array);
            let label = args
                .filter(|args| args.len() == 1)
                .and_then(|args| args[0].as_str())
                .unwrap_or("<invalid>");
            if command != "dba" || label != *expected_label {
                anyhow::bail!(
                    "compiled PlayerEventScriptPointers pointer {index} requires {expected_label} via dba, found {label} via {command}"
                );
            }
            Ok(label.to_string())
        })
        .collect()
}

fn player_event_execution_path(
    label: &str,
    definitions: &BTreeMap<String, Value>,
) -> Result<PlayerEventExecutionPath> {
    const INVALID: &[(&str, &[&str])] = &[("end", &[])];
    const EDGE_WARP: &[(&str, &[&str])] = &[("reloadend", &["MAPSETUP_CONNECTION"])];
    const WARP_TO_NEW_MAP: &[(&str, &[&str])] = &[
        ("warpsound", &[]),
        ("newloadmap", &["MAPSETUP_DOOR"]),
        ("end", &[]),
    ];
    const FALL_INTO_MAP: &[(&str, &[&str])] = &[
        ("newloadmap", &["MAPSETUP_FALL"]),
        ("playsound", &["SFX_KINESIS"]),
        ("applymovement", &["PLAYER", ".SkyfallMovement"]),
        ("playsound", &["SFX_STRENGTH"]),
        ("scall", &["LandAfterPitfallScript"]),
        ("end", &[]),
    ];
    const CHANGE_DIRECTION: &[(&str, &[&str])] = &[
        ("deactivatefacing", &["3"]),
        ("callasm", &["EnableWildEncounters"]),
        ("end", &[]),
    ];
    const SEEN_BY_TRAINER: &[(&str, &[&str])] = &[
        ("loadtemptrainer", &[]),
        ("encountermusic", &[]),
        ("showemote", &["EMOTE_SHOCK", "LAST_TALKED", "30"]),
        ("callasm", &["TrainerWalkToPlayer"]),
        ("applymovementlasttalked", &["wMovementBuffer"]),
        ("writeobjectxy", &["LAST_TALKED"]),
        ("faceobject", &["PLAYER", "LAST_TALKED"]),
        ("sjump", &["StartBattleWithMapTrainerScript"]),
    ];
    const TALK_TO_TRAINER: &[(&str, &[&str])] = &[
        ("faceplayer", &[]),
        ("trainerflagaction", &["CHECK_FLAG"]),
        ("iftrue", &["AlreadyBeatenTrainerScript"]),
        ("loadtemptrainer", &[]),
        ("encountermusic", &[]),
        ("sjump", &["StartBattleWithMapTrainerScript"]),
    ];
    const FIND_ITEM: &[(&str, &[&str])] = &[
        ("callasm", &[".TryReceiveItem"]),
        ("iffalse", &[".no_room"]),
        ("disappear", &["LAST_TALKED"]),
        ("opentext", &[]),
        ("writetext", &[".FoundItemText"]),
        ("playsound", &["SFX_ITEM"]),
        ("pause", &["60"]),
        ("itemnotify", &[]),
        ("closetext", &[]),
        ("end", &[]),
    ];
    const WHITEOUT: &[(&str, &[&str])] = &[
        ("reanchormap", &[]),
        ("callasm", &["OverworldBGMap"]),
        ("sjump", &["Script_Whiteout"]),
    ];
    const HATCH_EGG: &[(&str, &[&str])] = &[("callasm", &["OverworldHatchEgg"]), ("end", &[])];

    let (expected, path) = match label {
        "InvalidEventScript" => (INVALID, PlayerEventExecutionPath::CommonInterpreter),
        "EdgeWarpScript" => (EDGE_WARP, PlayerEventExecutionPath::CommonInterpreter),
        "WarpToNewMapScript" => (WARP_TO_NEW_MAP, PlayerEventExecutionPath::CommonInterpreter),
        "FallIntoMapScript" => (FALL_INTO_MAP, PlayerEventExecutionPath::CommonInterpreter),
        "ChangeDirectionScript" => (
            CHANGE_DIRECTION,
            PlayerEventExecutionPath::CommonInterpreter,
        ),
        "SeenByTrainerScript" => (SEEN_BY_TRAINER, PlayerEventExecutionPath::TypedConsumer),
        "TalkToTrainerScript" => (TALK_TO_TRAINER, PlayerEventExecutionPath::TypedConsumer),
        "FindItemInBallScript" => (FIND_ITEM, PlayerEventExecutionPath::TypedConsumer),
        "OverworldWhiteoutScript" => (WHITEOUT, PlayerEventExecutionPath::TypedConsumer),
        "HatchEggScript" => (HATCH_EGG, PlayerEventExecutionPath::TypedConsumer),
        _ => return Ok(PlayerEventExecutionPath::CommonInterpreter),
    };
    certify_exact_callasm_body(definitions, label, expected).map_err(|error| {
        let owner = match path {
            PlayerEventExecutionPath::CommonInterpreter => "common interpreter",
            PlayerEventExecutionPath::TypedConsumer => "typed consumer",
        };
        anyhow::anyhow!("player-event {owner} certificate failed for {label}: {error:?}")
    })?;
    if label == "FallIntoMapScript" {
        for (target, target_expected) in [
            (
                ".SkyfallMovement@FallIntoMapScript",
                &[("skyfall", &[][..]), ("step_end", &[][..])][..],
            ),
            (
                "LandAfterPitfallScript",
                &[("earthquake", &["16"][..]), ("end", &[][..])][..],
            ),
        ] {
            certify_exact_callasm_body(definitions, target, target_expected).map_err(|error| {
                anyhow::anyhow!(
                    "player-event common interpreter certificate failed for FallIntoMapScript target {target}: {error:?}"
                )
            })?;
        }
    }
    if label == "ChangeDirectionScript" {
        const ENABLE_WILD_ENCOUNTERS: &[(&str, &[&str])] = &[
            ("ld", &["hl", "wEnabledPlayerEvents"]),
            ("set", &["PLAYEREVENTS_WILD_ENCOUNTERS", "[hl]"]),
            ("ret", &[]),
        ];
        certify_exact_callasm_body(
            definitions,
            "EnableWildEncounters",
            ENABLE_WILD_ENCOUNTERS,
        )
        .map_err(|error| {
            anyhow::anyhow!(
                "player-event common interpreter certificate failed for ChangeDirectionScript target EnableWildEncounters: {error:?}"
            )
        })?;
    }
    Ok(path)
}

fn validate_compiled_overworld_event_catalog(data: &GameDataSet) -> Result<()> {
    let catalog = compiled_overworld_event_catalog(data)?;
    let standard_catalog = compiled_standard_script_catalog(data)?;
    let mut definitions = standard_catalog
        .iter()
        .filter(|(label, _)| *label != "StdScripts" && *label != "GlobalScriptRoots")
        .map(|(label, body)| (label.clone(), body.clone()))
        .collect::<BTreeMap<_, _>>();
    for (label, body) in catalog {
        if definitions.insert(label.clone(), body.clone()).is_some() {
            anyhow::bail!("duplicate global player-event definition {label}");
        }
    }
    let pointer_table = catalog
        .get("PlayerEventScriptPointers")
        .and_then(Value::as_array)
        .context("compiled OverworldEvents catalog is missing PlayerEventScriptPointers")?;
    for label in exact_player_event_pointer_labels(pointer_table)? {
        let body = catalog
            .get(&label)
            .or_else(|| standard_catalog.get(&label))
            .and_then(Value::as_array)
            .with_context(|| {
                format!("compiled player-event pointer {label} has no command body")
            })?;
        if body.is_empty() {
            anyhow::bail!("compiled player-event pointer {label} has an empty command body");
        }
        player_event_execution_path(&label, &definitions)?;
    }
    Ok(())
}
