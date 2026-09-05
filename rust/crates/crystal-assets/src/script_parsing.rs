fn parse_script_phone_commands(
    map_name: &str,
    scripts: &BTreeMap<String, Value>,
) -> Result<Vec<ScriptPhoneCommand>> {
    let mut commands = Vec::new();
    for (script_name, payload) in scripts {
        let Some(entries) = payload.as_array() else {
            continue;
        };
        for (index, entry) in entries.iter().enumerate() {
            let Some(command_name) = entry.get("command").and_then(Value::as_str) else {
                continue;
            };
            if !SCRIPT_PHONE_CHECK_COMMANDS.contains(&command_name)
                && !SCRIPT_PHONE_MUTATION_COMMANDS.contains(&command_name)
                && !SCRIPT_PHONE_REGISTRATION_COMMANDS.contains(&command_name)
            {
                continue;
            }
            let args = script_command_args(map_name, script_name, command_name, entry)?;
            if args.len() != 1 {
                anyhow::bail!(
                    "Malformed {command_name} command in {script_name} for {map_name}: expected 1 arg, found {}.",
                    args.len()
                );
            }
            commands.push(ScriptPhoneCommand {
                command: command_name.to_string(),
                contact_id: args[0].to_string(),
                source_script: script_name.clone(),
                command_index: index,
            });
        }
    }
    Ok(commands)
}

fn parse_script_runtime_commands(
    map_name: &str,
    scripts: &BTreeMap<String, Value>,
) -> Result<Vec<ScriptRuntimeCommand>> {
    let expected_arg_counts = script_runtime_command_arg_counts();
    let mut commands = Vec::new();
    for (script_name, payload) in scripts {
        let Some(entries) = payload.as_array() else {
            continue;
        };
        for (index, entry) in entries.iter().enumerate() {
            let Some(command_name) = entry.get("command").and_then(Value::as_str) else {
                continue;
            };
            let Some(expected) = expected_arg_counts.get(command_name) else {
                continue;
            };
            let args = script_command_args(map_name, script_name, command_name, entry)?;
            let arity_is_valid = if command_name == "ret" {
                args.len() <= 1
            } else if command_name == "memcall" {
                matches!(args.len(), 1 | 3)
            } else {
                args.len() == *expected
            };
            if !arity_is_valid {
                anyhow::bail!(
                    "Malformed {command_name} command in {script_name} for {map_name}: expected {expected} args, found {}.",
                    args.len()
                );
            }
            if command_name == "ret"
                && let Some(condition) = args.first()
                && ScriptRuntimeCpuCondition::from_asm_token(condition).is_none()
            {
                anyhow::bail!(
                    "Malformed ret command in {script_name} for {map_name}: unknown CPU condition {condition}."
                );
            }
            commands.push(ScriptRuntimeCommand {
                command: command_name.to_string(),
                args: args.iter().map(|arg| (*arg).to_string()).collect(),
                source_script: script_name.clone(),
                command_index: index,
            });
        }
    }
    Ok(commands)
}

fn parse_script_swarm_commands(
    map_name: &str,
    scripts: &BTreeMap<String, Value>,
) -> Result<Vec<ScriptSwarmCommand>> {
    let mut commands = Vec::new();
    for (script_name, payload) in scripts {
        let Some(entries) = payload.as_array() else {
            continue;
        };
        for (index, entry) in entries.iter().enumerate() {
            let Some(command_name) = entry.get("command").and_then(Value::as_str) else {
                continue;
            };
            if command_name != "swarm" {
                continue;
            }
            let args = script_command_args(map_name, script_name, command_name, entry)?;
            if args.len() != 2 {
                anyhow::bail!(
                    "Malformed swarm command in {script_name} for {map_name}: expected 2 args, found {}.",
                    args.len()
                );
            }
            commands.push(ScriptSwarmCommand {
                command: command_name.to_string(),
                swarm_token: args[0].to_string(),
                map_id: args[1].to_string(),
                source_script: script_name.clone(),
                command_index: index,
            });
        }
    }
    Ok(commands)
}

fn parse_script_economy_commands(
    map_name: &str,
    scripts: &BTreeMap<String, Value>,
) -> Result<Vec<ScriptEconomyCommand>> {
    let mut commands = Vec::new();
    for (script_name, payload) in scripts {
        let Some(entries) = payload.as_array() else {
            continue;
        };
        for (index, entry) in entries.iter().enumerate() {
            let Some(command_name) = entry.get("command").and_then(Value::as_str) else {
                continue;
            };
            match command_name {
                command
                    if SCRIPT_MONEY_CHECK_COMMANDS.contains(&command)
                        || SCRIPT_MONEY_MUTATION_COMMANDS.contains(&command) =>
                {
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if args.len() < 2 {
                        anyhow::bail!(
                            "Malformed {command_name} command in {script_name} for {map_name}: expected account and amount args, found {}.",
                            args.len()
                        );
                    }
                    commands.push(ScriptEconomyCommand {
                        command: command_name.to_string(),
                        account: Some(args[0].to_string()),
                        amount_tokens: script_economy_amount_tokens(&args[1..]),
                        source_script: script_name.clone(),
                        command_index: index,
                    });
                }
                command
                    if SCRIPT_COIN_CHECK_COMMANDS.contains(&command)
                        || SCRIPT_COIN_MUTATION_COMMANDS.contains(&command) =>
                {
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if args.is_empty() {
                        anyhow::bail!(
                            "Malformed {command_name} command in {script_name} for {map_name}: expected amount args, found 0."
                        );
                    }
                    commands.push(ScriptEconomyCommand {
                        command: command_name.to_string(),
                        account: None,
                        amount_tokens: script_economy_amount_tokens(&args),
                        source_script: script_name.clone(),
                        command_index: index,
                    });
                }
                _ => {}
            }
        }
    }
    Ok(commands)
}

fn script_economy_amount_tokens(args: &[&str]) -> Vec<String> {
    args.iter()
        .flat_map(|arg| arg.split_ascii_whitespace())
        .map(str::to_string)
        .collect()
}

fn parse_gift_pokemon_scripts(
    map_name: &str,
    scripts: &BTreeMap<String, Value>,
    constants: &StoryEventScriptConstants,
) -> Result<Vec<GiftPokemonScript>> {
    let mut gifts = Vec::new();
    for (script_name, payload) in scripts {
        let Some(entries) = payload.as_array() else {
            continue;
        };
        for (index, entry) in entries.iter().enumerate() {
            let Some(command_name) = entry.get("command").and_then(Value::as_str) else {
                continue;
            };
            match command_name {
                "givepoke" => {
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if args.len() != 2 && args.len() != 3 && args.len() != 5 {
                        anyhow::bail!(
                            "Malformed givepoke command in {script_name} for {map_name}: expected 2, 3, or 5 args, found {}.",
                            args.len()
                        );
                    }
                    let level = resolve_gift_level_token(map_name, args[1], constants)?;
                    gifts.push(GiftPokemonScript {
                        species_id: args[0].to_string(),
                        level_token: args[1].to_string(),
                        level,
                        held_item_id: args.get(2).and_then(|item| {
                            if *item == NO_ITEM {
                                None
                            } else {
                                Some((*item).to_string())
                            }
                        }),
                        nickname_label: args.get(3).map(|value| (*value).to_string()),
                        ot_label: args.get(4).map(|value| (*value).to_string()),
                        source_script: script_name.clone(),
                        command_index: index,
                        egg: false,
                    });
                }
                "giveegg" => {
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if args.len() != 2 {
                        anyhow::bail!(
                            "Malformed giveegg command in {script_name} for {map_name}: expected 2 args, found {}.",
                            args.len()
                        );
                    }
                    let level = resolve_gift_level_token(map_name, args[1], constants)?;
                    gifts.push(GiftPokemonScript {
                        species_id: args[0].to_string(),
                        level_token: args[1].to_string(),
                        level,
                        held_item_id: None,
                        nickname_label: None,
                        ot_label: None,
                        source_script: script_name.clone(),
                        command_index: index,
                        egg: true,
                    });
                }
                _ => {}
            }
        }
    }
    Ok(gifts)
}

fn parse_script_flag_commands(
    map_name: &str,
    scripts: &BTreeMap<String, Value>,
) -> Result<Vec<ScriptFlagCommand>> {
    let mut commands = Vec::new();
    for (script_name, payload) in scripts {
        let Some(entries) = payload.as_array() else {
            continue;
        };
        for (index, entry) in entries.iter().enumerate() {
            let Some(command_name) = entry.get("command").and_then(Value::as_str) else {
                continue;
            };
            if is_known_script_flag_command(command_name) {
                let args = script_command_args(map_name, script_name, command_name, entry)?;
                if args.len() != 1 {
                    anyhow::bail!(
                        "Malformed {command_name} command in {script_name} for {map_name}: expected 1 arg, found {}.",
                        args.len()
                    );
                }
                commands.push(ScriptFlagCommand {
                    command: command_name.to_string(),
                    flag_id: args[0].to_string(),
                    source_script: script_name.clone(),
                    command_index: index,
                });
            }
        }
    }
    Ok(commands)
}

fn parse_script_scene_commands(
    map_name: &str,
    scripts: &BTreeMap<String, Value>,
) -> Result<Vec<ScriptSceneCommand>> {
    let mut commands = Vec::new();
    for (script_name, payload) in scripts {
        let Some(entries) = payload.as_array() else {
            continue;
        };
        for (index, entry) in entries.iter().enumerate() {
            let Some(command_name) = entry.get("command").and_then(Value::as_str) else {
                continue;
            };
            match command_name {
                command if SCRIPT_SCENE_CHECK_COMMANDS.contains(&command) => {
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if !args.is_empty() {
                        anyhow::bail!(
                            "Malformed {command_name} command in {script_name} for {map_name}: expected 0 args, found {}.",
                            args.len()
                        );
                    }
                    commands.push(ScriptSceneCommand {
                        command: command_name.to_string(),
                        map_id: None,
                        scene_id: None,
                        source_script: script_name.clone(),
                        command_index: index,
                    });
                }
                command if SCRIPT_SCENE_TARGET_MAP_CHECK_COMMANDS.contains(&command) => {
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if args.len() != 1 {
                        anyhow::bail!(
                            "Malformed {command_name} command in {script_name} for {map_name}: expected 1 arg, found {}.",
                            args.len()
                        );
                    }
                    commands.push(ScriptSceneCommand {
                        command: command_name.to_string(),
                        map_id: Some(args[0].to_string()),
                        scene_id: None,
                        source_script: script_name.clone(),
                        command_index: index,
                    });
                }
                command if SCRIPT_SCENE_CURRENT_MAP_MUTATION_COMMANDS.contains(&command) => {
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if args.len() != 1 {
                        anyhow::bail!(
                            "Malformed {command_name} command in {script_name} for {map_name}: expected 1 arg, found {}.",
                            args.len()
                        );
                    }
                    commands.push(ScriptSceneCommand {
                        command: command_name.to_string(),
                        map_id: None,
                        scene_id: Some(args[0].to_string()),
                        source_script: script_name.clone(),
                        command_index: index,
                    });
                }
                command if SCRIPT_SCENE_TARGET_MAP_MUTATION_COMMANDS.contains(&command) => {
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if args.len() != 2 {
                        anyhow::bail!(
                            "Malformed {command_name} command in {script_name} for {map_name}: expected 2 args, found {}.",
                            args.len()
                        );
                    }
                    commands.push(ScriptSceneCommand {
                        command: command_name.to_string(),
                        map_id: Some(args[0].to_string()),
                        scene_id: Some(args[1].to_string()),
                        source_script: script_name.clone(),
                        command_index: index,
                    });
                }
                _ => {}
            }
        }
    }
    Ok(commands)
}

fn parse_script_audio_commands(
    map_name: &str,
    scripts: &BTreeMap<String, Value>,
) -> Result<Vec<ScriptAudioCommand>> {
    let mut commands = Vec::new();
    for (script_name, payload) in scripts {
        let Some(entries) = payload.as_array() else {
            continue;
        };
        for (index, entry) in entries.iter().enumerate() {
            let Some(command_name) = entry.get("command").and_then(Value::as_str) else {
                continue;
            };
            match command_name {
                "playmusic" | "playsound" | "cry" => {
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if args.len() != 1 {
                        anyhow::bail!(
                            "Malformed {command_name} command in {script_name} for {map_name}: expected 1 arg, found {}.",
                            args.len()
                        );
                    }
                    commands.push(ScriptAudioCommand {
                        command: command_name.to_string(),
                        audio_id: Some(args[0].to_string()),
                        fade_frames: None,
                        source_script: script_name.clone(),
                        command_index: index,
                    });
                }
                "musicfadeout" => {
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if args.len() != 2 {
                        anyhow::bail!(
                            "Malformed musicfadeout command in {script_name} for {map_name}: expected 2 args, found {}.",
                            args.len()
                        );
                    }
                    commands.push(ScriptAudioCommand {
                        command: command_name.to_string(),
                        audio_id: Some(args[0].to_string()),
                        fade_frames: Some(parse_script_u16(args[1])?),
                        source_script: script_name.clone(),
                        command_index: index,
                    });
                }
                "waitsfx" => {
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if !args.is_empty() {
                        anyhow::bail!(
                            "Malformed waitsfx command in {script_name} for {map_name}: expected 0 args, found {}.",
                            args.len()
                        );
                    }
                    commands.push(ScriptAudioCommand {
                        command: command_name.to_string(),
                        audio_id: None,
                        fade_frames: None,
                        source_script: script_name.clone(),
                        command_index: index,
                    });
                }
                _ => {}
            }
        }
    }
    Ok(commands)
}

fn parse_script_block_changes(
    map_name: &str,
    scripts: &BTreeMap<String, Value>,
) -> Result<Vec<ScriptBlockChange>> {
    let mut changes = Vec::new();
    for (script_name, payload) in scripts {
        let Some(entries) = payload.as_array() else {
            continue;
        };
        for (index, entry) in entries.iter().enumerate() {
            let Some(command_name) = entry.get("command").and_then(Value::as_str) else {
                continue;
            };
            if command_name != "changeblock" {
                continue;
            }
            let args = script_command_args(map_name, script_name, command_name, entry)?;
            if args.len() != 3 {
                anyhow::bail!(
                    "Malformed changeblock command in {script_name} for {map_name}: expected 3 args, found {}.",
                    args.len()
                );
            }
            changes.push(ScriptBlockChange {
                x: parse_script_u16(args[0])?,
                y: parse_script_u16(args[1])?,
                block_id: parse_script_u16(args[2])?,
                source_script: script_name.clone(),
                command_index: index,
            });
        }
    }
    Ok(changes)
}

fn parse_script_object_commands(
    map_name: &str,
    scripts: &BTreeMap<String, Value>,
) -> Result<Vec<ScriptObjectCommand>> {
    let mut commands = Vec::new();
    for (script_name, payload) in scripts {
        let Some(entries) = payload.as_array() else {
            continue;
        };
        for (index, entry) in entries.iter().enumerate() {
            let Some(command_name) = entry.get("command").and_then(Value::as_str) else {
                continue;
            };
            match command_name {
                command if SCRIPT_OBJECT_NO_PAYLOAD_COMMANDS.contains(&command) => {
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if !args.is_empty() {
                        anyhow::bail!(
                            "Malformed {command_name} command in {script_name} for {map_name}: expected 0 args, found {}.",
                            args.len()
                        );
                    }
                    commands.push(ScriptObjectCommand {
                        command: command_name.to_string(),
                        object_id: None,
                        target_object_id: None,
                        x: None,
                        y: None,
                        direction: None,
                        movement: None,
                        emote: None,
                        duration: None,
                        source_script: script_name.clone(),
                        command_index: index,
                    });
                }
                command if SCRIPT_OBJECT_VISIBILITY_COMMANDS.contains(&command) => {
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if args.len() != 1 {
                        anyhow::bail!(
                            "Malformed {command_name} command in {script_name} for {map_name}: expected 1 arg, found {}.",
                            args.len()
                        );
                    }
                    commands.push(ScriptObjectCommand {
                        command: command_name.to_string(),
                        object_id: Some(args[0].to_string()),
                        target_object_id: None,
                        x: None,
                        y: None,
                        direction: None,
                        movement: None,
                        emote: None,
                        duration: None,
                        source_script: script_name.clone(),
                        command_index: index,
                    });
                }
                command if SCRIPT_OBJECT_WRITE_COORDINATE_COMMANDS.contains(&command) => {
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if args.len() != 1 {
                        anyhow::bail!(
                            "Malformed {command_name} command in {script_name} for {map_name}: expected 1 arg, found {}.",
                            args.len()
                        );
                    }
                    commands.push(ScriptObjectCommand {
                        command: command_name.to_string(),
                        object_id: Some(args[0].to_string()),
                        target_object_id: None,
                        x: None,
                        y: None,
                        direction: None,
                        movement: None,
                        emote: None,
                        duration: None,
                        source_script: script_name.clone(),
                        command_index: index,
                    });
                }
                command if SCRIPT_OBJECT_COORDINATE_COMMANDS.contains(&command) => {
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if args.len() != 3 {
                        anyhow::bail!(
                            "Malformed {command_name} command in {script_name} for {map_name}: expected 3 args, found {}.",
                            args.len()
                        );
                    }
                    let (x, y) =
                        parse_map_event_runtime_coords(map_name, command_name, args[1], args[2])?;
                    commands.push(ScriptObjectCommand {
                        command: command_name.to_string(),
                        object_id: Some(args[0].to_string()),
                        target_object_id: None,
                        x: Some(x),
                        y: Some(y),
                        direction: None,
                        movement: None,
                        emote: None,
                        duration: None,
                        source_script: script_name.clone(),
                        command_index: index,
                    });
                }
                command if SCRIPT_OBJECT_DIRECTION_COMMANDS.contains(&command) => {
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if args.len() != 2 {
                        anyhow::bail!(
                            "Malformed {command_name} command in {script_name} for {map_name}: expected 2 args, found {}.",
                            args.len()
                        );
                    }
                    commands.push(ScriptObjectCommand {
                        command: command_name.to_string(),
                        object_id: Some(args[0].to_string()),
                        target_object_id: None,
                        x: None,
                        y: None,
                        direction: Some(args[1].to_string()),
                        movement: None,
                        emote: None,
                        duration: None,
                        source_script: script_name.clone(),
                        command_index: index,
                    });
                }
                command if SCRIPT_OBJECT_DIRECT_MOVEMENT_COMMANDS.contains(&command) => {
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if args.len() != 2 {
                        anyhow::bail!(
                            "Malformed {command_name} command in {script_name} for {map_name}: expected 2 args, found {}.",
                            args.len()
                        );
                    }
                    commands.push(ScriptObjectCommand {
                        command: command_name.to_string(),
                        object_id: Some(args[0].to_string()),
                        target_object_id: None,
                        x: None,
                        y: None,
                        direction: None,
                        movement: Some(args[1].to_string()),
                        emote: None,
                        duration: None,
                        source_script: script_name.clone(),
                        command_index: index,
                    });
                }
                command if SCRIPT_OBJECT_LAST_TALKED_MOVEMENT_COMMANDS.contains(&command) => {
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if args.len() != 1 {
                        anyhow::bail!(
                            "Malformed {command_name} command in {script_name} for {map_name}: expected 1 arg, found {}.",
                            args.len()
                        );
                    }
                    commands.push(ScriptObjectCommand {
                        command: command_name.to_string(),
                        object_id: None,
                        target_object_id: None,
                        x: None,
                        y: None,
                        direction: None,
                        movement: Some(args[0].to_string()),
                        emote: None,
                        duration: None,
                        source_script: script_name.clone(),
                        command_index: index,
                    });
                }
                command if SCRIPT_OBJECT_TARGET_COMMANDS.contains(&command) => {
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if args.len() != 2 {
                        anyhow::bail!(
                            "Malformed {command_name} command in {script_name} for {map_name}: expected 2 args, found {}.",
                            args.len()
                        );
                    }
                    commands.push(ScriptObjectCommand {
                        command: command_name.to_string(),
                        object_id: Some(args[0].to_string()),
                        target_object_id: Some(args[1].to_string()),
                        x: None,
                        y: None,
                        direction: None,
                        movement: None,
                        emote: None,
                        duration: None,
                        source_script: script_name.clone(),
                        command_index: index,
                    });
                }
                command if SCRIPT_OBJECT_EMOTE_COMMANDS.contains(&command) => {
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if args.len() != 3 {
                        anyhow::bail!(
                            "Malformed {command_name} command in {script_name} for {map_name}: expected 3 args, found {}.",
                            args.len()
                        );
                    }
                    commands.push(ScriptObjectCommand {
                        command: command_name.to_string(),
                        object_id: Some(args[1].to_string()),
                        target_object_id: None,
                        x: None,
                        y: None,
                        direction: None,
                        movement: None,
                        emote: Some(args[0].to_string()),
                        duration: Some(parse_script_u16(args[2])?),
                        source_script: script_name.clone(),
                        command_index: index,
                    });
                }
                _ => {}
            }
        }
    }
    Ok(commands)
}

fn parse_script_map_commands(
    map_name: &str,
    scripts: &BTreeMap<String, Value>,
    map_name_by_constant: &BTreeMap<String, String>,
) -> Result<Vec<ScriptMapCommand>> {
    let mut commands = Vec::new();
    for (script_name, payload) in scripts {
        let Some(entries) = payload.as_array() else {
            continue;
        };
        for (index, entry) in entries.iter().enumerate() {
            let Some(command_name) = entry.get("command").and_then(Value::as_str) else {
                continue;
            };
            match command_name {
                command if SCRIPT_MAP_WARP_COMMANDS.contains(&command) => {
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if args.len() != 3 {
                        anyhow::bail!(
                            "Malformed {command_name} command in {script_name} for {map_name}: expected 3 args, found {}.",
                            args.len()
                        );
                    }
                    let (x, y) =
                        parse_map_event_runtime_coords(map_name, command_name, args[1], args[2])?;
                    commands.push(ScriptMapCommand {
                        command: command_name.to_string(),
                        target_map: Some(script_warp_target_map(
                            map_name,
                            script_name,
                            command_name,
                            args[0],
                            map_name_by_constant,
                        )?),
                        x: Some(x),
                        y: Some(y),
                        facing: None,
                        map_setup: None,
                        source_script: script_name.clone(),
                        command_index: index,
                    });
                }
                command if SCRIPT_MAP_FACING_WARP_COMMANDS.contains(&command) => {
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if args.len() != 4 {
                        anyhow::bail!(
                            "Malformed {command_name} command in {script_name} for {map_name}: expected 4 args, found {}.",
                            args.len()
                        );
                    }
                    let (x, y) =
                        parse_map_event_runtime_coords(map_name, command_name, args[1], args[2])?;
                    commands.push(ScriptMapCommand {
                        command: command_name.to_string(),
                        target_map: Some(script_warp_target_map(
                            map_name,
                            script_name,
                            command_name,
                            args[0],
                            map_name_by_constant,
                        )?),
                        x: Some(x),
                        y: Some(y),
                        facing: Some(args[3].to_string()),
                        map_setup: None,
                        source_script: script_name.clone(),
                        command_index: index,
                    });
                }
                command if SCRIPT_MAP_NEW_LOAD_COMMANDS.contains(&command) => {
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if args.len() != 1 {
                        anyhow::bail!(
                            "Malformed {command_name} command in {script_name} for {map_name}: expected 1 arg, found {}.",
                            args.len()
                        );
                    }
                    commands.push(ScriptMapCommand {
                        command: command_name.to_string(),
                        target_map: None,
                        x: None,
                        y: None,
                        facing: None,
                        map_setup: Some(args[0].to_string()),
                        source_script: script_name.clone(),
                        command_index: index,
                    });
                }
                command if SCRIPT_MAP_NO_PAYLOAD_COMMANDS.contains(&command) => {
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if !args.is_empty() {
                        anyhow::bail!(
                            "Malformed {command_name} command in {script_name} for {map_name}: expected 0 args, found {}.",
                            args.len()
                        );
                    }
                    commands.push(ScriptMapCommand {
                        command: command_name.to_string(),
                        target_map: None,
                        x: None,
                        y: None,
                        facing: None,
                        map_setup: None,
                        source_script: script_name.clone(),
                        command_index: index,
                    });
                }
                command if SCRIPT_MAP_REANCHOR_COMMANDS.contains(&command) => {
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if args.len() > 1 {
                        anyhow::bail!(
                            "Malformed {command_name} command in {script_name} for {map_name}: expected 0 or 1 args, found {}.",
                            args.len()
                        );
                    }
                    commands.push(ScriptMapCommand {
                        command: command_name.to_string(),
                        target_map: None,
                        x: None,
                        y: None,
                        facing: None,
                        map_setup: args.first().map(|setup| (*setup).to_string()),
                        source_script: script_name.clone(),
                        command_index: index,
                    });
                }
                _ => {}
            }
        }
    }
    Ok(commands)
}

fn parse_script_text_commands(
    map_name: &str,
    scripts: &BTreeMap<String, Value>,
) -> Result<Vec<ScriptTextCommand>> {
    let mut commands = Vec::new();
    for (script_name, payload) in scripts {
        let Some(entries) = payload.as_array() else {
            continue;
        };
        for (index, entry) in entries.iter().enumerate() {
            let Some(command_name) = entry.get("command").and_then(Value::as_str) else {
                continue;
            };
            match command_name {
                command if SCRIPT_TEXT_NO_LABEL_COMMANDS.contains(&command) => {
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if !args.is_empty() {
                        anyhow::bail!(
                            "Malformed {command_name} command in {script_name} for {map_name}: expected 0 args, found {}.",
                            args.len()
                        );
                    }
                    commands.push(ScriptTextCommand {
                        command: command_name.to_string(),
                        text_label: None,
                        source_script: script_name.clone(),
                        command_index: index,
                    });
                }
                command if SCRIPT_TEXT_LABEL_COMMANDS.contains(&command) => {
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if args.len() != 1 {
                        anyhow::bail!(
                            "Malformed {command_name} command in {script_name} for {map_name}: expected 1 arg, found {}.",
                            args.len()
                        );
                    }
                    commands.push(ScriptTextCommand {
                        command: command_name.to_string(),
                        text_label: Some(resolve_local_script_label(script_name, args[0])?),
                        source_script: script_name.clone(),
                        command_index: index,
                    });
                }
                _ => {}
            }
        }
    }
    Ok(commands)
}

fn resolve_local_script_label(source_script: &str, label: &str) -> Result<String> {
    if !label.starts_with('.') {
        return Ok(label.to_string());
    }
    let parent_script = script_label_parent(source_script);
    if label.contains('@') {
        anyhow::ensure!(
            script_label_parent(label) == parent_script,
            "relative text reference {label} from {source_script} crosses ASM parent scope"
        );
        Ok(label.to_string())
    } else {
        Ok(format!("{label}@{parent_script}"))
    }
}

fn parse_script_text_bodies(
    map_name: &str,
    scripts: &BTreeMap<String, Value>,
) -> Result<BTreeMap<String, ScriptTextBody>> {
    let expected_arg_counts = text_body_command_arg_counts();
    let mut bodies = BTreeMap::new();
    for (script_name, payload) in scripts {
        if !is_text_script(payload) {
            continue;
        }
        let Some(entries) = payload.as_array() else {
            continue;
        };
        let start_index = entries
            .iter()
            .position(|entry| {
                entry
                    .get("command")
                    .and_then(Value::as_str)
                    .is_some_and(|command| expected_arg_counts.contains_key(command))
            })
            .unwrap_or(0);
        let mut commands = Vec::new();
        for (index, entry) in entries.iter().enumerate().skip(start_index) {
            let Some(command_name) = entry.get("command").and_then(Value::as_str) else {
                anyhow::bail!(
                    "Malformed text body command in {script_name} for {map_name}: command {index} is missing command."
                );
            };
            let Some(expected) = expected_arg_counts.get(command_name) else {
                anyhow::bail!(
                    "Malformed text body command in {script_name} for {map_name}: unknown command '{command_name}' at index {index}."
                );
            };
            let args = text_body_command_args(map_name, script_name, command_name, entry)?;
            if args.len() != *expected {
                anyhow::bail!(
                    "Malformed {command_name} command in {script_name} for {map_name}: expected {expected} args, found {}.",
                    args.len()
                );
            }
            commands.push(ScriptTextBodyCommand {
                command: command_name.to_string(),
                args,
                command_index: index,
            });
            if command_name == "text_asm" {
                break;
            }
        }
        if !commands.is_empty() {
            bodies.insert(
                script_name.clone(),
                ScriptTextBody {
                    label: script_name.clone(),
                    commands,
                },
            );
        }
    }
    Ok(bodies)
}

fn parse_script_menu_definitions(
    map_name: &str,
    scripts: &BTreeMap<String, Value>,
) -> Result<BTreeMap<String, ScriptMenuDefinition>> {
    let expected_arg_counts = menu_definition_command_arg_counts();
    let mut menus = BTreeMap::new();
    for (script_name, payload) in scripts {
        let Some(entries) = payload.as_array() else {
            continue;
        };
        let command_names: Vec<&str> = entries
            .iter()
            .filter_map(|entry| entry.get("command").and_then(Value::as_str))
            .collect();
        let menu_coords_index = command_names
            .iter()
            .position(|command| *command == "menu_coords");
        let is_menu_data_label = !command_names.is_empty()
            && command_names
                .iter()
                .all(|command| expected_arg_counts.contains_key(command));
        if menu_coords_index.is_none() && !is_menu_data_label {
            continue;
        };
        let menu_coords_index = menu_coords_index.unwrap_or(0);
        let start_index = if menu_coords_index > 0
            && command_names
                .get(menu_coords_index - 1)
                .is_some_and(|command| *command == "db")
        {
            menu_coords_index - 1
        } else {
            menu_coords_index
        };
        let mut commands = Vec::new();
        for (index, entry) in entries.iter().enumerate().skip(start_index) {
            let command_name = entry
                .get("command")
                .and_then(Value::as_str)
                .with_context(|| {
                    format!("Malformed menu definition command in {script_name} for {map_name}: command must be a string.")
                })?;
            let Some(expected) = expected_arg_counts.get(command_name) else {
                if !commands.is_empty() {
                    break;
                }
                anyhow::bail!(
                    "Malformed menu definition command in {script_name} for {map_name}: unknown command '{command_name}' at index {index}."
                );
            };
            let args = script_command_args(map_name, script_name, command_name, entry)?;
            if !expected.contains(&args.len()) {
                anyhow::bail!(
                    "Malformed {command_name} menu command in {script_name} for {map_name}: expected one of {:?} args, found {}.",
                    expected,
                    args.len()
                );
            }
            if command_name == "menu_coords" {
                validate_menu_coord_args(&args).with_context(|| {
                    format!("Malformed menu_coords command in {script_name} for {map_name}")
                })?;
            }
            commands.push(ScriptMenuCommand {
                command: command_name.to_string(),
                args: args.iter().map(|arg| (*arg).to_string()).collect(),
                command_index: index,
            });
        }
        menus.insert(
            script_name.clone(),
            ScriptMenuDefinition {
                label: script_name.clone(),
                commands,
            },
        );
    }
    Ok(menus)
}

fn parse_script_vertical_menus(
    map_name: &str,
    scripts: &BTreeMap<String, Value>,
    menus: &BTreeMap<String, ScriptMenuDefinition>,
) -> Result<BTreeMap<String, ScriptVerticalMenuDefinition>> {
    let mut vertical_menus = BTreeMap::new();
    for (script_name, payload) in scripts {
        let Some(entries) = payload.as_array() else {
            continue;
        };
        let mut pending_loadmenu: Option<(usize, String)> = None;
        for (index, entry) in entries.iter().enumerate() {
            let Some(command_name) = entry.get("command").and_then(Value::as_str) else {
                continue;
            };
            match command_name {
                "loadmenu" => {
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if args.len() != 1 {
                        anyhow::bail!(
                            "Malformed loadmenu command in {script_name} for {map_name}: expected 1 arg, found {}.",
                            args.len()
                        );
                    }
                    pending_loadmenu = Some((index, args[0].to_string()));
                }
                "verticalmenu" | "_2dmenu" => {
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if !args.is_empty() {
                        anyhow::bail!(
                            "Malformed verticalmenu command in {script_name} for {map_name}: expected 0 args, found {}.",
                            args.len()
                        );
                    }
                    let Some((loadmenu_command_index, header_label)) = pending_loadmenu.take()
                    else {
                        anyhow::bail!(
                            "verticalmenu command in {script_name} for {map_name} at index {index} requires a preceding loadmenu"
                        );
                    };
                    let resolved_header_label =
                        resolve_menu_reference(script_name, &header_label, menus)?;
                    let (data_label, options) =
                        vertical_menu_options(map_name, &resolved_header_label, menus)?;
                    let two_dimensional = command_name == "_2dmenu";
                    let (rows, columns, spacing) = if two_dimensional {
                        let data = data_label
                            .as_deref()
                            .and_then(|label| menus.get(label))
                            .unwrap_or_else(|| {
                                menus
                                    .get(&resolved_header_label)
                                    .expect("resolved menu header must exist")
                            });
                        let dimensions = data
                            .commands
                            .iter()
                            .find(|command| command.command == "dn")
                            .with_context(|| {
                                format!(
                                    "2D menu {resolved_header_label} for {map_name} has no row/column declaration"
                                )
                            })?;
                        let rows = dimensions.args[0].parse::<usize>().with_context(|| {
                            format!("parse 2D menu row count {}", dimensions.args[0])
                        })?;
                        let columns = dimensions.args[1].parse::<usize>().with_context(|| {
                            format!("parse 2D menu column count {}", dimensions.args[1])
                        })?;
                        if rows == 0 || columns == 0 || rows * columns != options.len() {
                            anyhow::bail!(
                                "2D menu {resolved_header_label} for {map_name} declares {rows}x{columns} but has {} options",
                                options.len()
                            );
                        }
                        let dn_index = data
                            .commands
                            .iter()
                            .position(|command| command.command == "dn")
                            .expect("2D menu dimensions were located above");
                        let spacing = data.commands[dn_index + 1..]
                            .iter()
                            .find(|command| command.command == "db")
                            .and_then(|command| command.args.first())
                            .with_context(|| {
                                format!("2D menu {resolved_header_label} for {map_name} has no spacing byte")
                            })?
                            .parse::<usize>()
                            .with_context(|| format!("parse 2D menu spacing for {resolved_header_label}"))?;
                        (Some(rows), Some(columns), Some(spacing))
                    } else {
                        (None, None, None)
                    };
                    let key = format!("{script_name}:{index}");
                    if vertical_menus
                        .insert(
                            key.clone(),
                            ScriptVerticalMenuDefinition {
                                source_script: script_name.clone(),
                                loadmenu_command_index,
                                verticalmenu_command_index: index,
                                header_label: resolved_header_label,
                                data_label,
                                options,
                                two_dimensional,
                                rows,
                                columns,
                                spacing,
                            },
                        )
                        .is_some()
                    {
                        anyhow::bail!("duplicate vertical menu definition key {key} in {map_name}");
                    }
                }
                _ => {}
            }
        }
    }
    Ok(vertical_menus)
}

fn vertical_menu_options(
    map_name: &str,
    header_label: &str,
    menus: &BTreeMap<String, ScriptMenuDefinition>,
) -> Result<(Option<String>, Vec<String>)> {
    let header = menus.get(header_label).with_context(|| {
        format!("vertical menu header {header_label} for {map_name} is missing")
    })?;
    let inline_options = menu_text_options(&header.commands);
    if !inline_options.is_empty() {
        return Ok((None, inline_options));
    }
    for command in &header.commands {
        if command.command != "dw" {
            continue;
        }
        let Some(raw_data_label) = command.args.first() else {
            continue;
        };
        let data_label = resolve_menu_reference(header_label, raw_data_label, menus)?;
        let data = menus.get(&data_label).with_context(|| {
            format!(
                "vertical menu header {header_label} for {map_name} references missing data label {data_label}"
            )
        })?;
        let options = menu_text_options(&data.commands);
        if !options.is_empty() {
            return Ok((Some(data_label), options));
        }
        for data_command in &data.commands {
            if !matches!(data_command.command.as_str(), "dw" | "dba") {
                continue;
            }
            let Some(raw_text_label) = data_command.args.first() else {
                continue;
            };
            let text_label = resolve_menu_reference(&data_label, raw_text_label, menus)?;
            let Some(text) = menus.get(&text_label) else {
                continue;
            };
            let options = menu_text_options(&text.commands);
            if !options.is_empty() {
                return Ok((Some(data_label), options));
            }
        }
    }
    anyhow::bail!("vertical menu header {header_label} for {map_name} has no options")
}

fn menu_text_options(commands: &[ScriptMenuCommand]) -> Vec<String> {
    commands
        .iter()
        .filter(|command| command.command == "db")
        .flat_map(|command| command.args.iter())
        .filter_map(|arg| strip_asm_menu_text(arg))
        .collect()
}

fn strip_asm_menu_text(value: &str) -> Option<String> {
    let mut text = value.trim();
    if !(text.starts_with('"') && text.ends_with('"')) {
        return None;
    }
    text = text.trim_matches('"');
    text = text.strip_suffix('@').unwrap_or(text);
    (!text.is_empty()).then(|| text.to_string())
}

fn resolve_menu_reference(
    source_script: &str,
    label: &str,
    menus: &BTreeMap<String, ScriptMenuDefinition>,
) -> Result<String> {
    if !label.starts_with('.') {
        return Ok(label.to_string());
    }
    let parent_script = script_label_parent(source_script);
    let local = if label.contains('@') {
        anyhow::ensure!(
            script_label_parent(label) == parent_script,
            "relative vertical menu reference {label} from {source_script} crosses ASM parent scope"
        );
        label.to_string()
    } else {
        format!("{label}@{parent_script}")
    };
    if menus.contains_key(&local) {
        Ok(local)
    } else {
        anyhow::bail!(
            "unresolved relative vertical menu reference {label} from {source_script}: scoped label {local} does not exist"
        )
    }
}

fn parse_script_elevators(
    map_name: &str,
    scripts: &BTreeMap<String, Value>,
    map_name_by_constant: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, ScriptElevatorDefinition>> {
    let mut elevators = BTreeMap::new();
    for (script_name, payload) in scripts {
        let Some(entries) = payload.as_array() else {
            continue;
        };
        for (index, entry) in entries.iter().enumerate() {
            let Some(command_name) = entry.get("command").and_then(Value::as_str) else {
                continue;
            };
            if command_name != "elevator" {
                continue;
            }
            let args = script_command_args(map_name, script_name, command_name, entry)?;
            if args.len() != 1 {
                anyhow::bail!(
                    "Malformed elevator command in {script_name} for {map_name}: expected 1 arg, found {}.",
                    args.len()
                );
            }
            let data_label = resolve_script_reference(scripts, script_name, args[0])?;
            let floors =
                parse_elevator_floors(map_name, &data_label, scripts, map_name_by_constant)?;
            let key = format!("{script_name}:{index}");
            if elevators
                .insert(
                    key.clone(),
                    ScriptElevatorDefinition {
                        source_script: script_name.clone(),
                        elevator_command_index: index,
                        data_label,
                        floors,
                    },
                )
                .is_some()
            {
                anyhow::bail!("duplicate elevator definition key {key} in {map_name}");
            }
        }
    }
    Ok(elevators)
}

fn parse_elevator_floors(
    map_name: &str,
    data_label: &str,
    scripts: &BTreeMap<String, Value>,
    map_name_by_constant: &BTreeMap<String, String>,
) -> Result<Vec<ScriptRuntimeElevatorFloor>> {
    let payload = scripts
        .get(data_label)
        .with_context(|| format!("elevator data label {data_label} for {map_name} is missing"))?;
    let entries = payload.as_array().with_context(|| {
        format!("elevator data label {data_label} for {map_name} must be a command array")
    })?;
    let mut floors = Vec::new();
    for (index, entry) in entries.iter().enumerate() {
        let Some(command_name) = entry.get("command").and_then(Value::as_str) else {
            anyhow::bail!(
                "Malformed elevator data command in {data_label} for {map_name}: command {index} is missing command."
            );
        };
        if command_name != "elevfloor" {
            continue;
        }
        let args = script_command_args(map_name, data_label, command_name, entry)?;
        if args.len() != 3 {
            anyhow::bail!(
                "Malformed elevfloor command in {data_label} for {map_name}: expected 3 args, found {}.",
                args.len()
            );
        }
        let target_map = map_name_by_constant.get(args[2]).with_context(|| {
            format!(
                "elevfloor command in {data_label} for {map_name} references missing map constant {}",
                args[2]
            )
        })?;
        floors.push(ScriptRuntimeElevatorFloor {
            floor: args[0].to_string(),
            warp: u16::try_from(parse_script_i32(args[1])?).with_context(|| {
                format!(
                    "Malformed elevfloor command in {data_label} for {map_name}: warp '{}' is outside u16",
                    args[1]
                )
            })?,
            target_map: target_map.clone(),
            source_script: data_label.to_string(),
            command_index: index,
        });
    }
    if floors.is_empty() {
        anyhow::bail!("elevator data label {data_label} for {map_name} has no elevfloor entries");
    }
    Ok(floors)
}

fn resolve_script_reference(
    scripts: &BTreeMap<String, Value>,
    source_script: &str,
    label: &str,
) -> Result<String> {
    if !label.starts_with('.') {
        return Ok(label.to_string());
    }
    let parent_script = script_label_parent(source_script);
    let local = if label.contains('@') {
        anyhow::ensure!(
            script_label_parent(label) == parent_script,
            "relative script reference {label} from {source_script} crosses ASM parent scope"
        );
        label.to_string()
    } else {
        format!("{label}@{parent_script}")
    };
    if scripts.contains_key(&local) {
        Ok(local)
    } else {
        anyhow::bail!(
            "unresolved relative script reference {label} from {source_script}: scoped label {local} does not exist"
        )
    }
}

fn text_body_command_args(
    map_name: &str,
    script_name: &str,
    command_name: &str,
    entry: &Value,
) -> Result<Vec<String>> {
    let args = entry.get("args").with_context(|| {
        format!("Malformed {command_name} command in {script_name} for {map_name}: missing args.")
    })?;
    if let Some(text) = args.as_str() {
        if text.is_empty() {
            return Ok(Vec::new());
        }
        return Ok(vec![text.to_string()]);
    }
    let Some(array) = args.as_array() else {
        anyhow::bail!(
            "Malformed {command_name} command in {script_name} for {map_name}: args must be a string or an array."
        );
    };
    array
        .iter()
        .enumerate()
        .map(|(index, value)| {
            value.as_str().map(str::to_string).with_context(|| {
                format!(
                    "Malformed {command_name} command in {script_name} for {map_name}: arg {index} must be a string."
                )
            })
        })
        .collect()
}

fn parse_script_variable_commands(
    map_name: &str,
    scripts: &BTreeMap<String, Value>,
) -> Result<Vec<ScriptVariableCommand>> {
    let mut commands = Vec::new();
    for (script_name, payload) in scripts {
        let Some(entries) = payload.as_array() else {
            continue;
        };
        for (index, entry) in entries.iter().enumerate() {
            let Some(command_name) = entry.get("command").and_then(Value::as_str) else {
                continue;
            };
            match command_name {
                "setval" => {
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if args.is_empty() {
                        anyhow::bail!(
                            "Malformed setval command in {script_name} for {map_name}: expected at least 1 arg, found 0."
                        );
                    }
                    commands.push(ScriptVariableCommand {
                        command: command_name.to_string(),
                        target: None,
                        value_tokens: args.iter().map(|arg| (*arg).to_string()).collect(),
                        source_script: script_name.clone(),
                        command_index: index,
                    });
                }
                "readvar" | "readmem" | "writemem" => {
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if args.len() != 1 {
                        anyhow::bail!(
                            "Malformed {command_name} command in {script_name} for {map_name}: expected 1 arg, found {}.",
                            args.len()
                        );
                    }
                    commands.push(ScriptVariableCommand {
                        command: command_name.to_string(),
                        target: Some(args[0].to_string()),
                        value_tokens: Vec::new(),
                        source_script: script_name.clone(),
                        command_index: index,
                    });
                }
                "loadvar" | "loadmem" => {
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if args.len() < 2 {
                        anyhow::bail!(
                            "Malformed {command_name} command in {script_name} for {map_name}: expected at least 2 args, found {}.",
                            args.len()
                        );
                    }
                    commands.push(ScriptVariableCommand {
                        command: command_name.to_string(),
                        target: Some(args[0].to_string()),
                        value_tokens: args[1..].iter().map(|arg| (*arg).to_string()).collect(),
                        source_script: script_name.clone(),
                        command_index: index,
                    });
                }
                "checktime" => {
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if args.len() != 1 {
                        anyhow::bail!(
                            "Malformed checktime command in {script_name} for {map_name}: expected 1 arg, found {}.",
                            args.len()
                        );
                    }
                    commands.push(ScriptVariableCommand {
                        command: command_name.to_string(),
                        target: None,
                        value_tokens: vec![args[0].to_string()],
                        source_script: script_name.clone(),
                        command_index: index,
                    });
                }
                _ => {}
            }
        }
    }
    Ok(commands)
}

fn parse_script_control_commands(
    map_name: &str,
    scripts: &BTreeMap<String, Value>,
) -> Result<Vec<ScriptControlCommand>> {
    let mut commands = Vec::new();
    for (script_name, payload) in scripts {
        let Some(entries) = payload.as_array() else {
            continue;
        };
        for (index, entry) in entries.iter().enumerate() {
            let Some(command_name) = entry.get("command").and_then(Value::as_str) else {
                continue;
            };
            match command_name {
                "ifequal" | "ifnotequal" | "ifgreater" | "ifless" => {
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if args.len() != 2 {
                        anyhow::bail!(
                            "Malformed {command_name} command in {script_name} for {map_name}: expected 2 args, found {}.",
                            args.len()
                        );
                    }
                    let target_label = args[1].to_string();
                    commands.push(ScriptControlCommand {
                        command: command_name.to_string(),
                        compare_value: Some(args[0].to_string()),
                        resolved_target_script: resolve_script_target_label(
                            scripts,
                            script_name,
                            args[1],
                        ),
                        target_label: Some(target_label),
                        source_script: script_name.clone(),
                        command_index: index,
                    });
                }
                "iftrue" | "iffalse" | "sjump" | "farsjump" | "scall" | "farscall" | "sdefer" => {
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if args.len() != 1 {
                        anyhow::bail!(
                            "Malformed {command_name} command in {script_name} for {map_name}: expected 1 arg, found {}.",
                            args.len()
                        );
                    }
                    let target_label = args[0].to_string();
                    commands.push(ScriptControlCommand {
                        command: command_name.to_string(),
                        compare_value: None,
                        resolved_target_script: resolve_script_target_label(
                            scripts,
                            script_name,
                            args[0],
                        ),
                        target_label: Some(target_label),
                        source_script: script_name.clone(),
                        command_index: index,
                    });
                }
                "jumpstd" | "callstd" => {
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if args.len() != 1 {
                        anyhow::bail!(
                            "Malformed {command_name} command in {script_name} for {map_name}: expected 1 arg, found {}.",
                            args.len()
                        );
                    }
                    commands.push(ScriptControlCommand {
                        command: command_name.to_string(),
                        compare_value: None,
                        target_label: Some(args[0].to_string()),
                        resolved_target_script: None,
                        source_script: script_name.clone(),
                        command_index: index,
                    });
                }
                "end" | "endcallback" | "endifjustbattled" => {
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if !args.is_empty() {
                        anyhow::bail!(
                            "Malformed {command_name} command in {script_name} for {map_name}: expected 0 args, found {}.",
                            args.len()
                        );
                    }
                    commands.push(ScriptControlCommand {
                        command: command_name.to_string(),
                        compare_value: None,
                        target_label: None,
                        resolved_target_script: None,
                        source_script: script_name.clone(),
                        command_index: index,
                    });
                }
                _ => {}
            }
        }
    }
    Ok(commands)
}

fn resolve_script_target_label(
    scripts: &BTreeMap<String, Value>,
    source_script: &str,
    target_label: &str,
) -> Option<String> {
    if target_label.starts_with('.') {
        let parent_script = script_label_parent(source_script);
        let local = if target_label.contains('@') {
            (script_label_parent(target_label) == parent_script)
                .then(|| target_label.to_string())?
        } else {
            format!("{target_label}@{parent_script}")
        };
        return scripts.contains_key(&local).then_some(local);
    }
    if scripts.contains_key(target_label) {
        return Some(target_label.to_string());
    }
    None
}

fn script_label_parent(source_script: &str) -> &str {
    source_script
        .rsplit_once('@')
        .map(|(_, parent)| parent)
        .unwrap_or(source_script)
}

fn parse_script_movements(
    map_name: &str,
    scripts: &BTreeMap<String, Value>,
    object_commands: &[ScriptObjectCommand],
) -> Result<Vec<ScriptMovement>> {
    let mut movements = Vec::new();
    let mut movement_refs = BTreeMap::<(&str, &str), Vec<&ScriptObjectCommand>>::new();
    for command in object_commands
        .iter()
        .filter(|command| SCRIPT_OBJECT_MOVEMENT_COMMANDS.contains(&command.command.as_str()))
    {
        if let Some(movement) = command.movement.as_deref() {
            movement_refs
                .entry((movement, script_label_parent(&command.source_script)))
                .or_default()
                .push(command);
        }
    }
    for ((movement_label, parent_script), commands) in movement_refs {
        let script_key = if movement_label.starts_with('.') {
            if movement_label.contains('@') {
                anyhow::ensure!(
                    script_label_parent(movement_label) == parent_script,
                    "movement reference '{movement_label}' from {parent_script} on {map_name} crosses ASM parent scope"
                );
                movement_label.to_string()
            } else {
                format!("{movement_label}@{parent_script}")
            }
        } else {
            movement_label.to_string()
        };
        if !scripts.contains_key(&script_key) {
            if movement_label == "wMovementBuffer"
                && commands.iter().all(|command| {
                    command.command == "applymovement"
                        && command.object_id.as_deref() == Some("PLAYER")
                        && scripts
                            .get(&command.source_script)
                            .and_then(Value::as_array)
                            .and_then(|entries| {
                                command
                                    .command_index
                                    .checked_sub(1)
                                    .and_then(|index| entries.get(index))
                            })
                            .is_some_and(|entry| {
                                entry.get("command").and_then(Value::as_str) == Some("special")
                                    && entry.get("args").and_then(Value::as_array).is_some_and(
                                        |args| {
                                            args.len() == 1
                                                && args[0].as_str() == Some("SurfStartStep")
                                        },
                                    )
                            })
                })
            {
                movements.push(ScriptMovement {
                    label: movement_label.to_string(),
                    source_script: Some(parent_script.to_string()),
                    steps: vec![
                        ScriptMovementStep {
                            command: "slow_step".to_string(),
                            direction: Some(SCRIPT_MOVEMENT_PLAYER_FACING_DIRECTION.to_string()),
                            duration: None,
                            index: 0,
                        },
                        ScriptMovementStep {
                            command: "step_end".to_string(),
                            direction: None,
                            duration: None,
                            index: 1,
                        },
                    ],
                });
                continue;
            }
            anyhow::bail!(
                "movement reference '{movement_label}' from {parent_script} on {map_name} resolves to missing script"
            );
        }
        let Some(payload) = scripts.get(&script_key) else {
            anyhow::bail!("movement script {script_key} for {map_name} is missing");
        };
        let Some(entries) = payload.as_array() else {
            anyhow::bail!("movement script {script_key} for {map_name} must be an array");
        };
        let mut steps = Vec::new();
        for (index, entry) in entries.iter().enumerate() {
            let Some(command_name) = entry.get("command").and_then(Value::as_str) else {
                anyhow::bail!(
                    "Malformed movement script {script_key} for {map_name}: command {index} is missing command."
                );
            };
            if !is_known_script_movement_command(command_name) {
                anyhow::bail!(
                    "Malformed movement script {script_key} for {map_name}: non-movement command '{command_name}' at index {index}."
                );
            }
            let args = script_command_args(map_name, &script_key, command_name, entry)?;
            let (direction, duration) = match command_name {
                command if SCRIPT_MOVEMENT_DIRECTION_COMMANDS.contains(&command) => {
                    if args.len() != 1 {
                        anyhow::bail!(
                            "Malformed {command_name} movement in {script_key} for {map_name}: expected 1 arg, found {}.",
                            args.len()
                        );
                    }
                    (Some(args[0].to_string()), None)
                }
                command if SCRIPT_MOVEMENT_REQUIRED_DURATION_COMMANDS.contains(&command) => {
                    if args.len() != 1 {
                        anyhow::bail!(
                            "Malformed {command_name} movement in {script_key} for {map_name}: expected 1 arg, found {}.",
                            args.len()
                        );
                    }
                    (None, Some(parse_script_u16(args[0])?))
                }
                command if SCRIPT_MOVEMENT_NO_ARG_COMMANDS.contains(&command) => {
                    if !args.is_empty() {
                        anyhow::bail!(
                            "Malformed {command_name} movement in {script_key} for {map_name}: expected 0 args, found {}.",
                            args.len()
                        );
                    }
                    (None, None)
                }
                _ => unreachable!("movement command checked above"),
            };
            steps.push(ScriptMovementStep {
                command: command_name.to_string(),
                direction,
                duration,
                index,
            });
        }
        if !steps
            .last()
            .is_some_and(|step| is_script_movement_terminator(&step.command))
        {
            anyhow::bail!(
                "Malformed movement script {script_key} for {map_name}: movement must end with a terminating opcode."
            );
        }
        movements.push(ScriptMovement {
            label: movement_label.to_string(),
            source_script: Some(parent_script.to_string()),
            steps,
        });
    }
    Ok(movements)
}

fn parse_scripted_trainer_battles(
    map_name: &str,
    scripts: &BTreeMap<String, Value>,
) -> Result<Vec<ScriptedTrainerBattle>> {
    let mut scripted_battles = Vec::new();
    for (script_name, payload) in scripts {
        let Some(entries) = payload.as_array() else {
            continue;
        };
        let mut last_win_text = String::new();
        let mut last_loss_text = String::new();
        let mut battle_type = "BATTLETYPE_TRAINER".to_string();
        let mut pending: Option<PendingScriptedTrainerBattle> = None;

        for (index, entry) in entries.iter().enumerate() {
            let Some(command_name) = entry.get("command").and_then(Value::as_str) else {
                continue;
            };
            match command_name {
                "winlosstext" => {
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if args.len() != 2 {
                        anyhow::bail!(
                            "Malformed winlosstext command in {script_name} for {map_name}: expected 2 args, found {}.",
                            args.len()
                        );
                    }
                    last_win_text = trainer_command_optional_arg(args[0]);
                    last_loss_text = trainer_command_optional_arg(args[1]);
                }
                "loadvar" => {
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if args.len() != 2 {
                        anyhow::bail!(
                            "Malformed loadvar command in {script_name} for {map_name}: expected 2 args, found {}.",
                            args.len()
                        );
                    }
                    if args[0] == "VAR_BATTLETYPE" {
                        battle_type = args[1].to_string();
                    }
                }
                "loadtrainer" => {
                    if let Some(done) = pending.take() {
                        scripted_battles.push(done.into_battle(map_name)?);
                    }
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if args.len() != 2 {
                        anyhow::bail!(
                            "Malformed loadtrainer command in {script_name} for {map_name}: expected 2 args, found {}.",
                            args.len()
                        );
                    }
                    let mut request = TrainerBattleRequest::new(args[0], args[1], "");
                    request.battle_type = battle_type.clone();
                    request.win_text = last_win_text.clone();
                    request.loss_text = last_loss_text.clone();
                    request.source_script = script_name.clone();
                    pending = Some(PendingScriptedTrainerBattle {
                        source_script: script_name.clone(),
                        loadtrainer_command_index: index,
                        startbattle_command_index: None,
                        request,
                    });
                }
                "startbattle" => {
                    if let Some(pending) = pending.as_mut() {
                        pending.startbattle_command_index = Some(index);
                    }
                }
                "catchtutorial" => {
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if args.len() != 1 {
                        anyhow::bail!(
                            "Malformed catchtutorial command in {script_name} for {map_name}: expected 1 arg, found {}.",
                            args.len()
                        );
                    }
                    if let Some(pending) = pending.as_mut() {
                        pending.request.battle_type = args[0].to_string();
                        pending.startbattle_command_index = Some(index);
                    }
                }
                _ => {}
            }
        }

        if let Some(done) = pending {
            scripted_battles.push(done.into_battle(map_name)?);
        }
    }

    Ok(scripted_battles)
}

struct PendingScriptedTrainerBattle {
    source_script: String,
    loadtrainer_command_index: usize,
    startbattle_command_index: Option<usize>,
    request: TrainerBattleRequest,
}

impl PendingScriptedTrainerBattle {
    fn into_battle(self, map_name: &str) -> Result<ScriptedTrainerBattle> {
        let startbattle_command_index = self.startbattle_command_index.with_context(|| {
            format!(
                "loadtrainer command in {} for {map_name} is not followed by startbattle",
                self.source_script
            )
        })?;
        Ok(ScriptedTrainerBattle {
            source_script: self.source_script,
            loadtrainer_command_index: self.loadtrainer_command_index,
            startbattle_command_index,
            request: self.request,
        })
    }
}

fn parse_scripted_wild_battles(
    map_name: &str,
    scripts: &BTreeMap<String, Value>,
) -> Result<Vec<ScriptedWildBattle>> {
    let mut scripted_battles = Vec::new();
    for (script_name, payload) in scripts {
        let Some(entries) = payload.as_array() else {
            continue;
        };
        let mut battle_type = "BATTLETYPE_NORMAL".to_string();
        let mut pending: Option<PendingScriptedWildBattle> = None;

        for (index, entry) in entries.iter().enumerate() {
            let Some(command_name) = entry.get("command").and_then(Value::as_str) else {
                continue;
            };
            match command_name {
                "loadvar" => {
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if args.len() != 2 {
                        anyhow::bail!(
                            "Malformed loadvar command in {script_name} for {map_name}: expected 2 args, found {}.",
                            args.len()
                        );
                    }
                    if args[0] == "VAR_BATTLETYPE" {
                        battle_type = args[1].to_string();
                        if let Some(pending) = pending.as_mut() {
                            pending.request.battle_type = battle_type.clone();
                        }
                    }
                }
                "loadwildmon" => {
                    if let Some(done) = pending.take() {
                        scripted_battles.push(done.into_battle(map_name)?);
                    }
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if args.len() != 2 {
                        anyhow::bail!(
                            "Malformed loadwildmon command in {script_name} for {map_name}: expected 2 args, found {}.",
                            args.len()
                        );
                    }
                    let level = parse_script_u16(args[1])?;
                    let level = u8::try_from(level).with_context(|| {
                        format!(
                            "loadwildmon level '{}' in {script_name} for {map_name} is outside u8 range",
                            args[1]
                        )
                    })?;
                    let mut request = StaticWildBattleRequest::new(args[0], level);
                    request.battle_type = battle_type.clone();
                    request.source_script = script_name.clone();
                    pending = Some(PendingScriptedWildBattle {
                        source_script: script_name.clone(),
                        loadwildmon_command_index: index,
                        startbattle_command_index: None,
                        request,
                    });
                }
                "startbattle" => {
                    if let Some(pending) = pending.as_mut() {
                        pending.startbattle_command_index = Some(index);
                    }
                }
                "catchtutorial" => {
                    let args = script_command_args(map_name, script_name, command_name, entry)?;
                    if args.len() != 1 {
                        anyhow::bail!(
                            "Malformed catchtutorial command in {script_name} for {map_name}: expected 1 arg, found {}.",
                            args.len()
                        );
                    }
                    if let Some(pending) = pending.as_mut() {
                        pending.request.battle_type = args[0].to_string();
                        pending.startbattle_command_index = Some(index);
                    }
                }
                _ => {}
            }
        }

        if let Some(done) = pending {
            scripted_battles.push(done.into_battle(map_name)?);
        }
    }
    Ok(scripted_battles)
}

struct PendingScriptedWildBattle {
    source_script: String,
    loadwildmon_command_index: usize,
    startbattle_command_index: Option<usize>,
    request: StaticWildBattleRequest,
}

impl PendingScriptedWildBattle {
    fn into_battle(self, map_name: &str) -> Result<ScriptedWildBattle> {
        let startbattle_command_index = self.startbattle_command_index.with_context(|| {
            format!(
                "loadwildmon command in {} for {map_name} is not followed by startbattle",
                self.source_script
            )
        })?;
        Ok(ScriptedWildBattle {
            source_script: self.source_script,
            loadwildmon_command_index: self.loadwildmon_command_index,
            startbattle_command_index,
            request: self.request,
        })
    }
}

fn script_command_args<'a>(
    map_name: &str,
    script_name: &str,
    command_name: &str,
    entry: &'a Value,
) -> Result<Vec<&'a str>> {
    let args = entry
        .get("args")
        .and_then(Value::as_array)
        .with_context(|| {
            format!(
                "Malformed {command_name} command in {script_name} for {map_name}: args must be an array."
            )
        })?;
    args.iter()
        .enumerate()
        .map(|(index, value)| {
            let arg = value.as_str().with_context(|| {
                format!(
                    "Malformed {command_name} command in {script_name} for {map_name}: arg {index} must be a string."
                )
            })?;
            if arg.is_empty() || arg.trim() != arg || arg.chars().any(char::is_control) {
                anyhow::bail!(
                    "Malformed {command_name} command in {script_name} for {map_name}: arg {index} must be exact and non-empty."
                );
            }
            Ok(arg)
        })
        .collect()
}

fn trainer_command_optional_arg(value: &str) -> String {
    if value == "0" || value == "-1" {
        String::new()
    } else {
        value.to_string()
    }
}

fn parse_map_scene_table(map_name: &str, payload: &Value) -> Result<MapSceneTable> {
    let commands: Vec<ScriptCommand> =
        serde_json::from_value(payload.clone()).context("parse map scene command list")?;
    let mut in_scene_section = false;
    let mut scenes = Vec::new();
    let mut last_scene_script: Option<String> = None;

    for command in commands {
        match command.command.as_str() {
            "def_scene_scripts" => {
                in_scene_section = true;
                last_scene_script = None;
            }
            "scene_script" if in_scene_section => {
                if command.args.len() != 1 && command.args.len() != 2 {
                    anyhow::bail!(
                        "Malformed scene_script in {map_name}: expected 1 or 2 args, found {}.",
                        command.args.len()
                    );
                }
                last_scene_script = Some(command.args[0].clone());
                if command.args.len() == 1 {
                    continue;
                }
                scenes.push(MapScene {
                    script_name: Some(command.args[0].clone()),
                    scene_id: command.args[1].clone(),
                });
            }
            "scene_const" if in_scene_section => {
                if command.args.len() != 1 {
                    anyhow::bail!(
                        "Malformed scene_const in {map_name}: expected 1 arg, found {}.",
                        command.args.len()
                    );
                }
                scenes.push(MapScene {
                    scene_id: command.args[0].clone(),
                    script_name: last_scene_script.clone(),
                });
            }
            "def_callbacks" | "def_warp_events" | "def_coord_events" | "def_bg_events"
            | "def_object_events" => {
                in_scene_section = false;
                last_scene_script = None;
            }
            _ => {}
        }
    }

    Ok(MapSceneTable { scenes })
}

fn parse_map_event_runtime_coords(
    map_name: &str,
    event_kind: &str,
    x_token: &str,
    y_token: &str,
) -> Result<(u16, u16)> {
    let x = parse_script_u16(x_token)
        .with_context(|| format!("{event_kind} x coordinate '{x_token}' in {map_name}"))?;
    let y = parse_script_u16(y_token)
        .with_context(|| format!("{event_kind} y coordinate '{y_token}' in {map_name}"))?;
    raw_event_tile_to_runtime_tile_checked(x, y).with_context(|| {
        format!("{event_kind} coordinate ({x}, {y}) in {map_name} overflows runtime tile range")
    })?;
    Ok((x, y))
}

fn parse_script_i32(token: &str) -> Result<i32> {
    if token.is_empty() {
        anyhow::bail!("cannot parse an empty numeric token");
    }
    if token.trim() != token {
        anyhow::bail!("numeric token '{token}' must be exact and untrimmed");
    }
    if token.as_bytes()[0] == b'+' {
        anyhow::bail!("numeric token '{token}' must not use an explicit plus sign");
    }
    parse_script_i32_token("raw_script", token)
        .map_err(anyhow::Error::new)
        .with_context(|| format!("parse numeric token '{token}'"))
}

fn parse_script_u16(token: &str) -> Result<u16> {
    let value = parse_script_i32(token)?;
    u16::try_from(value).with_context(|| format!("numeric token '{token}' is outside u16 range"))
}

fn resolve_gift_level_token(
    map_name: &str,
    token: &str,
    constants: &StoryEventScriptConstants,
) -> Result<u8> {
    let value = if let Ok(value) = parse_script_i32(token) {
        i64::from(value)
    } else if let Some(value) = constants.maps.get(map_name).and_then(|map| map.get(token)) {
        *value
    } else if let Some(value) = constants.global.get(token) {
        *value
    } else {
        anyhow::bail!("gift level token '{token}' does not resolve from pack constants");
    };
    let level = u8::try_from(value)
        .with_context(|| format!("gift level token '{token}' is outside u8 range"))?;
    if level == 0 {
        anyhow::bail!("gift level token '{token}' resolves to zero");
    }
    Ok(level)
}

fn script_warp_target_map(
    map_name: &str,
    script_name: &str,
    command_name: &str,
    constant: &str,
    map_name_by_constant: &BTreeMap<String, String>,
) -> Result<String> {
    if constant == "NONE" {
        return Ok(constant.to_string());
    }
    map_name_by_constant.get(constant).cloned().with_context(|| {
        format!(
            "{command_name} command in {script_name} for {map_name} references missing map constant {constant}"
        )
    })
}

fn item_key(item: &Item) -> Result<String> {
    let issues = item_payload_issues_with_known_field_rules(item, true);
    if issues.contains(&ItemPayloadIssue::MissingScriptName) {
        anyhow::bail!("item '{}' is missing explicit script_name", item.name);
    } else if let Some(ItemPayloadIssue::InvalidScriptName { script_name }) = issues
        .iter()
        .find(|issue| matches!(issue, ItemPayloadIssue::InvalidScriptName { .. }))
    {
        anyhow::bail!(
            "item '{}' has invalid script_name '{}'",
            item.name,
            script_name
        );
    } else {
        Ok(item.script_name.clone())
    }
}

fn validate_manifest_move(move_data: &Move) -> Result<()> {
    let issues = move_payload_issues(move_data);
    if issues.contains(&MovePayloadIssue::MissingName) {
        anyhow::bail!("move payload must declare explicit name");
    }
    if let Some(MovePayloadIssue::InvalidName { name }) = issues
        .iter()
        .find(|issue| matches!(issue, MovePayloadIssue::InvalidName { .. }))
    {
        anyhow::bail!("move '{}' has invalid name '{}'", move_data.name, name);
    }
    if issues.contains(&MovePayloadIssue::MissingType) {
        anyhow::bail!("move '{}' must declare explicit type", move_data.name);
    }
    if let Some(MovePayloadIssue::InvalidType { move_type }) = issues
        .iter()
        .find(|issue| matches!(issue, MovePayloadIssue::InvalidType { .. }))
    {
        anyhow::bail!("move '{}' has invalid type '{}'", move_data.name, move_type);
    }
    if issues.contains(&MovePayloadIssue::MissingEffect) {
        anyhow::bail!("move '{}' must declare explicit effect", move_data.name);
    }
    if let Some(MovePayloadIssue::InvalidEffect { effect }) = issues
        .iter()
        .find(|issue| matches!(issue, MovePayloadIssue::InvalidEffect { .. }))
    {
        anyhow::bail!("move '{}' has invalid effect '{}'", move_data.name, effect);
    }
    Ok(())
}

fn validate_manifest_item(item: &Item) -> Result<()> {
    let issues = item_payload_issues_with_known_field_rules(item, true);
    if issues.contains(&ItemPayloadIssue::MissingName) {
        anyhow::bail!("item '{}' must declare explicit name", item.script_name);
    }
    if let Some(ItemPayloadIssue::InvalidName { name }) = issues
        .iter()
        .find(|issue| matches!(issue, ItemPayloadIssue::InvalidName { .. }))
    {
        anyhow::bail!("item '{}' has invalid name '{}'", item.script_name, name);
    }
    if issues.contains(&ItemPayloadIssue::MissingDescription) {
        anyhow::bail!(
            "item '{}' must declare explicit description",
            item.script_name
        );
    }
    if let Some(ItemPayloadIssue::InvalidDescription { description }) = issues
        .iter()
        .find(|issue| matches!(issue, ItemPayloadIssue::InvalidDescription { .. }))
    {
        anyhow::bail!(
            "item '{}' has invalid description '{}'",
            item.script_name,
            description
        );
    }
    if issues.contains(&ItemPayloadIssue::MissingPocket) {
        anyhow::bail!("item '{}' must declare explicit pocket", item.script_name);
    }
    if let Some(ItemPayloadIssue::InvalidPocket { pocket }) = issues
        .iter()
        .find(|issue| matches!(issue, ItemPayloadIssue::InvalidPocket { .. }))
    {
        anyhow::bail!(
            "item '{}' has invalid pocket '{}'",
            item.script_name,
            pocket
        );
    }
    if issues.contains(&ItemPayloadIssue::MissingEffect) {
        anyhow::bail!("item '{}' must declare explicit effect", item.script_name);
    }
    if let Some(ItemPayloadIssue::InvalidEffect { effect }) = issues
        .iter()
        .find(|issue| matches!(issue, ItemPayloadIssue::InvalidEffect { .. }))
    {
        anyhow::bail!(
            "item '{}' has invalid effect '{}'",
            item.script_name,
            effect
        );
    }
    if issues.contains(&ItemPayloadIssue::MissingHeldEffect) {
        anyhow::bail!(
            "item '{}' must declare explicit held_effect",
            item.script_name
        );
    }
    if let Some(ItemPayloadIssue::InvalidHeldEffect { held_effect }) = issues
        .iter()
        .find(|issue| matches!(issue, ItemPayloadIssue::InvalidHeldEffect { .. }))
    {
        anyhow::bail!(
            "item '{}' has invalid held_effect '{}'",
            item.script_name,
            held_effect
        );
    }
    if let Some(ItemPayloadIssue::InvalidProperty { property }) = issues
        .iter()
        .find(|issue| matches!(issue, ItemPayloadIssue::InvalidProperty { .. }))
    {
        anyhow::bail!(
            "item '{}' has invalid property '{}'",
            item.script_name,
            property
        );
    }
    if issues.contains(&ItemPayloadIssue::MissingFieldMenu) {
        anyhow::bail!(
            "item '{}' must declare explicit field_menu",
            item.script_name
        );
    }
    if let Some(ItemPayloadIssue::InvalidFieldMenu { menu }) = issues
        .iter()
        .find(|issue| matches!(issue, ItemPayloadIssue::InvalidFieldMenu { .. }))
    {
        anyhow::bail!(
            "item '{}' has invalid field_menu '{}'",
            item.script_name,
            menu
        );
    }
    if issues.contains(&ItemPayloadIssue::MissingBattleMenu) {
        anyhow::bail!(
            "item '{}' must declare explicit battle_menu",
            item.script_name
        );
    }
    if let Some(ItemPayloadIssue::InvalidBattleMenu { menu }) = issues
        .iter()
        .find(|issue| matches!(issue, ItemPayloadIssue::InvalidBattleMenu { .. }))
    {
        anyhow::bail!(
            "item '{}' has invalid battle_menu '{}'",
            item.script_name,
            menu
        );
    }
    if issues.contains(&ItemPayloadIssue::MissingTmhmIndex) {
        anyhow::bail!(
            "TM/HM item '{}' must declare explicit tmhm_index",
            item.script_name
        );
    }
    if let Some(ItemPayloadIssue::InvalidTmhmIndex { index }) = issues
        .iter()
        .find(|issue| matches!(issue, ItemPayloadIssue::InvalidTmhmIndex { .. }))
    {
        anyhow::bail!(
            "TM/HM item '{}' must declare positive tmhm_index, found {}",
            item.script_name,
            index
        );
    }
    if issues.contains(&ItemPayloadIssue::MissingTmhmMove) {
        anyhow::bail!(
            "TM/HM item '{}' must declare explicit tmhm_move",
            item.script_name
        );
    }
    if let Some(ItemPayloadIssue::InvalidTmhmMove { move_id }) = issues
        .iter()
        .find(|issue| matches!(issue, ItemPayloadIssue::InvalidTmhmMove { .. }))
    {
        anyhow::bail!(
            "TM/HM item '{}' must declare exact tmhm_move, found '{}'",
            item.script_name,
            move_id
        );
    }
    if let Some(ItemPayloadIssue::InvalidFieldUsableMenu { menu, usable }) = issues
        .iter()
        .find(|issue| matches!(issue, ItemPayloadIssue::InvalidFieldUsableMenu { .. }))
    {
        anyhow::bail!(
            "item '{}' field_usable {} contradicts field_menu '{}'",
            item.script_name,
            usable,
            menu
        );
    }
    if let Some(ItemPayloadIssue::InvalidBattleUsableMenu { menu, usable }) = issues
        .iter()
        .find(|issue| matches!(issue, ItemPayloadIssue::InvalidBattleUsableMenu { .. }))
    {
        anyhow::bail!(
            "item '{}' battle_usable {} contradicts battle_menu '{}'",
            item.script_name,
            usable,
            menu
        );
    }
    if issues.contains(&ItemPayloadIssue::MissingScriptName) {
        anyhow::bail!("item '{}' is missing explicit script_name", item.name);
    }
    if let Some(ItemPayloadIssue::InvalidScriptName { script_name }) = issues
        .iter()
        .find(|issue| matches!(issue, ItemPayloadIssue::InvalidScriptName { .. }))
    {
        anyhow::bail!(
            "item '{}' has invalid script_name '{}'",
            item.name,
            script_name
        );
    }
    for issue in issues {
        match issue {
            ItemPayloadIssue::MissingName
            | ItemPayloadIssue::InvalidName { .. }
            | ItemPayloadIssue::MissingDescription
            | ItemPayloadIssue::InvalidDescription { .. }
            | ItemPayloadIssue::MissingScriptName
            | ItemPayloadIssue::InvalidScriptName { .. }
            | ItemPayloadIssue::MissingPocket
            | ItemPayloadIssue::InvalidPocket { .. }
            | ItemPayloadIssue::MissingEffect
            | ItemPayloadIssue::InvalidEffect { .. }
            | ItemPayloadIssue::MissingHeldEffect
            | ItemPayloadIssue::InvalidHeldEffect { .. }
            | ItemPayloadIssue::InvalidProperty { .. }
            | ItemPayloadIssue::MissingFieldMenu
            | ItemPayloadIssue::InvalidFieldMenu { .. }
            | ItemPayloadIssue::MissingBattleMenu
            | ItemPayloadIssue::InvalidBattleMenu { .. }
            | ItemPayloadIssue::MissingTmhmIndex
            | ItemPayloadIssue::InvalidTmhmIndex { .. }
            | ItemPayloadIssue::MissingTmhmMove
            | ItemPayloadIssue::InvalidTmhmMove { .. }
            | ItemPayloadIssue::InvalidFieldUsableMenu { .. }
            | ItemPayloadIssue::InvalidBattleUsableMenu { .. } => {}
            ItemPayloadIssue::MissingFieldItemPayload => anyhow::bail!(
                "item '{}' is field_usable without an exact field item payload",
                item.script_name
            ),
            ItemPayloadIssue::MissingBattleItemPayload => anyhow::bail!(
                "item '{}' is battle_usable without an exact battle item payload",
                item.script_name
            ),
            ItemPayloadIssue::InvalidStatusHeal { index, status } => anyhow::bail!(
                "item '{}' status_heals[{index}] has invalid status '{}'",
                item.script_name,
                status
            ),
            ItemPayloadIssue::InvalidHealAmount { amount } => anyhow::bail!(
                "item '{}' has invalid heal parameter {}",
                item.script_name,
                amount
            ),
            ItemPayloadIssue::InvalidReviveHpPercent { percent } => anyhow::bail!(
                "item '{}' has invalid revive_hp_percent {}",
                item.script_name,
                percent
            ),
            ItemPayloadIssue::InvalidPartyReviveHpPercent { percent } => anyhow::bail!(
                "item '{}' has invalid party_revive_hp_percent {}",
                item.script_name,
                percent
            ),
            ItemPayloadIssue::MissingPpRestoreScope => anyhow::bail!(
                "item '{}' must declare explicit pp_restore_scope",
                item.script_name
            ),
            ItemPayloadIssue::InvalidPpRestoreScope { scope } => anyhow::bail!(
                "item '{}' has invalid pp_restore_scope '{}'",
                item.script_name,
                scope
            ),
            ItemPayloadIssue::InvalidPpRestorePoints { points } => anyhow::bail!(
                "item '{}' has invalid pp_restore_points {}",
                item.script_name,
                points
            ),
            ItemPayloadIssue::InvalidPpUpStages { stages } => anyhow::bail!(
                "item '{}' has invalid pp_up_stages {}",
                item.script_name,
                stages
            ),
            ItemPayloadIssue::MissingVitaminStat => anyhow::bail!(
                "item '{}' must declare explicit vitamin_stat",
                item.script_name
            ),
            ItemPayloadIssue::InvalidVitaminStat { stat } => anyhow::bail!(
                "item '{}' has invalid vitamin_stat '{}'",
                item.script_name,
                stat
            ),
            ItemPayloadIssue::MissingVitaminStatExp => anyhow::bail!(
                "item '{}' must declare explicit vitamin_stat_exp",
                item.script_name
            ),
            ItemPayloadIssue::InvalidVitaminStatExp { amount } => anyhow::bail!(
                "item '{}' has invalid vitamin_stat_exp {}",
                item.script_name,
                amount
            ),
            ItemPayloadIssue::MissingVitaminMaxStatExp => anyhow::bail!(
                "item '{}' must declare explicit vitamin_max_stat_exp",
                item.script_name
            ),
            ItemPayloadIssue::InvalidVitaminMaxStatExp { max } => anyhow::bail!(
                "item '{}' has invalid vitamin_max_stat_exp {}",
                item.script_name,
                max
            ),
            ItemPayloadIssue::InvalidRareCandyLevelGain { level_gain } => anyhow::bail!(
                "item '{}' has invalid rare_candy_level_gain {}",
                item.script_name,
                level_gain
            ),
            ItemPayloadIssue::MissingBattleStatBoostStat => anyhow::bail!(
                "item '{}' must declare explicit battle_stat_boost_stat",
                item.script_name
            ),
            ItemPayloadIssue::InvalidBattleStatBoostStat { stat } => anyhow::bail!(
                "item '{}' has invalid battle_stat_boost_stat '{}'",
                item.script_name,
                stat
            ),
            ItemPayloadIssue::MissingBattleStatBoostStages => anyhow::bail!(
                "item '{}' must declare explicit battle_stat_boost_stages",
                item.script_name
            ),
            ItemPayloadIssue::InvalidBattleStatBoostStages { stages } => anyhow::bail!(
                "item '{}' has invalid battle_stat_boost_stages {}",
                item.script_name,
                stages
            ),
            ItemPayloadIssue::MissingBattleStatDropGuard => anyhow::bail!(
                "item '{}' must declare explicit battle_stat_drop_guard",
                item.script_name
            ),
            ItemPayloadIssue::InvalidBattleStatDropGuard => anyhow::bail!(
                "item '{}' has invalid battle_stat_drop_guard false",
                item.script_name
            ),
            ItemPayloadIssue::InvalidBattleEscapeMode { mode } => anyhow::bail!(
                "item '{}' has invalid battle_escape_mode '{}'",
                item.script_name,
                mode
            ),
            ItemPayloadIssue::InvalidBattleCaptureBall => anyhow::bail!(
                "item '{}' has invalid battle_capture_ball false",
                item.script_name
            ),
            ItemPayloadIssue::InvalidEscapeRopeMode { mode } => anyhow::bail!(
                "item '{}' has invalid escape_rope_mode '{}'",
                item.script_name,
                mode
            ),
            ItemPayloadIssue::InvalidRepelSteps { steps } => anyhow::bail!(
                "item '{}' has invalid repel_steps {}",
                item.script_name,
                steps
            ),
            ItemPayloadIssue::InvalidBattleFocusEnergy => anyhow::bail!(
                "item '{}' has invalid battle_focus_energy false",
                item.script_name
            ),
            ItemPayloadIssue::InvalidConfusionHeal => anyhow::bail!(
                "item '{}' has invalid confusion_heal false",
                item.script_name
            ),
        }
    }
    Ok(())
}

fn validate_manifest_item_references(item: &Item, move_ids: &BTreeSet<String>) -> Result<()> {
    for issue in item_reference_issues(item, move_ids) {
        match issue {
            ItemReferenceIssue::UnknownTmhmMove { move_id } => {
                anyhow::bail!(
                    "TM/HM item '{}' references missing move '{}'",
                    item.script_name,
                    move_id
                );
            }
        }
    }
    Ok(())
}

fn resolve_collision_token(token: &str) -> Result<u8> {
    if token.is_empty() || token.trim() != token {
        anyhow::bail!("collision token '{token}' must be exact and non-empty");
    }
    if token.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return u8::from_str_radix(token, 16)
            .with_context(|| format!("invalid hexadecimal collision token {token}"));
    }
    Ok(match token {
        "FLOOR" => permissions::FLOOR,
        "01" => 0x01,
        "03" => 0x03,
        "04" => 0x04,
        "WALL" => permissions::WALL,
        "CUT_08" => 0x08,
        "TALL_GRASS_10" => 0x10,
        "CUT_TREE" => 0x12,
        "LONG_GRASS" => 0x14,
        "HEADBUTT_TREE" => 0x15,
        "TALL_GRASS" => permissions::TALL_GRASS,
        "CUT_TREE_1A" => 0x1a,
        "LONG_GRASS_1C" => 0x1c,
        "HEADBUTT_TREE_1D" => 0x1d,
        "WATER_21" => 0x21,
        "ICE" => 0x23,
        "WHIRLPOOL" => permissions::WHIRLPOOL,
        "BUOY" => 0x27,
        "CUT_28" => 0x28,
        "WATER" => permissions::WATER,
        "ICE_2B" => 0x2b,
        "WHIRLPOOL_2C" => permissions::WHIRLPOOL_2C,
        "WATERFALL_RIGHT" => permissions::WATERFALL_RIGHT,
        "WATERFALL_LEFT" => permissions::WATERFALL_LEFT,
        "WATERFALL_UP" => permissions::WATERFALL_UP,
        "WATERFALL" => permissions::WATERFALL,
        "CURRENT_RIGHT" => 0x38,
        "CURRENT_LEFT" => 0x39,
        "CURRENT_UP" => 0x3a,
        "CURRENT_DOWN" => permissions::CURRENT_DOWN,
        "BRAKE" => 0x40,
        "WALK_RIGHT" => 0x41,
        "WALK_LEFT" => 0x42,
        "WALK_UP" => 0x43,
        "WALK_DOWN" => 0x44,
        "BRAKE_45" => 0x45,
        "BRAKE_46" => 0x46,
        "BRAKE_47" => 0x47,
        "GRASS_48" => 0x48,
        "GRASS_49" => 0x49,
        "GRASS_4A" => 0x4a,
        "GRASS_4B" => 0x4b,
        "GRASS_4C" => 0x4c,
        "WALK_RIGHT_ALT" => 0x50,
        "WALK_LEFT_ALT" => 0x51,
        "WALK_UP_ALT" => 0x52,
        "WALK_DOWN_ALT" => 0x53,
        "BRAKE_ALT" => 0x54,
        "BRAKE_55" => 0x55,
        "BRAKE_56" => 0x56,
        "BRAKE_57" => 0x57,
        "5B" => 0x5b,
        "PIT" => permissions::PIT,
        "VIRTUAL_BOY" => 0x61,
        "64" => 0x64,
        "65" => 0x65,
        "PIT_68" => permissions::PIT_68,
        "WARP_CARPET_DOWN" => permissions::WARP_CARPET_DOWN,
        "DOOR" => permissions::DOOR,
        "LADDER" => permissions::LADDER,
        "STAIRCASE_73" => permissions::STAIRCASE_73,
        "CAVE_74" => permissions::CAVE_74,
        "DOOR_75" => permissions::DOOR_75,
        "WARP_CARPET_LEFT" => permissions::WARP_CARPET_LEFT,
        "WARP_77" => permissions::WARP_77,
        "WARP_CARPET_UP" => permissions::WARP_CARPET_UP,
        "DOOR_79" => permissions::DOOR_79,
        "STAIRCASE" => permissions::STAIRCASE,
        "CAVE" => permissions::CAVE,
        "WARP_PANEL" => permissions::WARP_PANEL,
        "DOOR_7D" => permissions::DOOR_7D,
        "WARP_CARPET_RIGHT" => permissions::WARP_CARPET_RIGHT,
        "WARP_7F" => permissions::WARP_7F,
        "COUNTER" => permissions::COUNTER,
        "BOOKSHELF" => permissions::BOOKSHELF,
        "PC" => permissions::PC,
        "RADIO" => permissions::RADIO,
        "TOWN_MAP" => permissions::TOWN_MAP,
        "MART_SHELF" => permissions::MART_SHELF,
        "TV" => permissions::TV,
        "COUNTER_98" => permissions::COUNTER_98,
        "9C" => 0x9c,
        "WINDOW" => permissions::WINDOW,
        "INCENSE_BURNER" => permissions::INCENSE_BURNER,
        "HOP_RIGHT" => permissions::HOP_RIGHT,
        "HOP_LEFT" => permissions::HOP_LEFT,
        "HOP_UP" => permissions::HOP_UP,
        "HOP_DOWN" => permissions::HOP_DOWN,
        "HOP_DOWN_RIGHT" => permissions::HOP_DOWN_RIGHT,
        "HOP_DOWN_LEFT" => permissions::HOP_DOWN_LEFT,
        "HOP_UP_RIGHT" => permissions::HOP_UP_RIGHT,
        "HOP_UP_LEFT" => permissions::HOP_UP_LEFT,
        "RIGHT_WALL" => permissions::RIGHT_WALL,
        "LEFT_WALL" => permissions::LEFT_WALL,
        "UP_WALL" => permissions::UP_WALL,
        "DOWN_WALL" => permissions::DOWN_WALL,
        "DOWN_RIGHT_WALL" => permissions::DOWN_RIGHT_WALL,
        "DOWN_LEFT_WALL" => permissions::DOWN_LEFT_WALL,
        "UP_RIGHT_WALL" => permissions::UP_RIGHT_WALL,
        "UP_LEFT_WALL" => permissions::UP_LEFT_WALL,
        "RIGHT_BUOY" => permissions::RIGHT_BUOY,
        "LEFT_BUOY" => permissions::LEFT_BUOY,
        "UP_BUOY" => permissions::UP_BUOY,
        "DOWN_BUOY" => permissions::DOWN_BUOY,
        "DOWN_RIGHT_BUOY" => 0xc4,
        "DOWN_LEFT_BUOY" => 0xc5,
        "UP_RIGHT_BUOY" => 0xc6,
        "UP_LEFT_BUOY" => 0xc7,
        "FF" => 0xff,
        other => anyhow::bail!("unknown collision token {other}"),
    })
}

fn parse_metatile_id(id: &str) -> Result<usize> {
    if id.is_empty() || id.trim() != id {
        anyhow::bail!("metatile id '{id}' must be exact and non-empty");
    }
    usize::from_str_radix(id, 16).with_context(|| format!("parse hex metatile id '{id}'"))
}

fn decode_base64_bytes(input: &str) -> Result<Vec<u8>> {
    let mut out = Vec::with_capacity(input.len() * 3 / 4);
    let mut chunk = [0_u8; 4];
    let mut chunk_len = 0;
    let mut padding = 0;

    for byte in input.bytes() {
        let value = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b'=' => {
                padding += 1;
                0
            }
            _ => anyhow::bail!("invalid base64 byte 0x{byte:02x}"),
        };
        chunk[chunk_len] = value;
        chunk_len += 1;
        if chunk_len != 4 {
            continue;
        }
        if padding > 2 {
            anyhow::bail!("invalid base64 padding");
        }
        out.push((chunk[0] << 2) | (chunk[1] >> 4));
        if padding < 2 {
            out.push((chunk[1] << 4) | (chunk[2] >> 2));
        }
        if padding == 0 {
            out.push((chunk[2] << 6) | chunk[3]);
        }
        chunk = [0; 4];
        chunk_len = 0;
        padding = 0;
    }

    if chunk_len != 0 {
        anyhow::bail!("base64 length is not a multiple of 4");
    }
    Ok(out)
}
