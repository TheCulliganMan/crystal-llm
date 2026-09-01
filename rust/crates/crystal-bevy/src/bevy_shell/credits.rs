fn split_visible_credits_label(line: &str) -> (String, String) {
    if let Some((label, inline)) = line.split_once(':') {
        (
            label.trim().trim_start_matches('.').to_string(),
            inline.trim().to_string(),
        )
    } else {
        (
            line.trim().trim_start_matches('.').to_string(),
            String::new(),
        )
    }
}

fn append_visible_credits_string_directive(buffer: &mut String, line: &str) {
    let trimmed = line.trim();
    let (directive, rest) = if let Some(rest) = trimmed.strip_prefix("next") {
        ("next", rest)
    } else if let Some(rest) = trimmed.strip_prefix("db") {
        ("db", rest)
    } else {
        return;
    };
    let text = rest.trim().trim_matches('"');
    if directive == "next" && !buffer.is_empty() {
        buffer.push('\n');
    }
    buffer.push_str(text);
}

fn append_visible_credits_tile_directive(lines: &mut Vec<Vec<u16>>, line: &str) -> Result<()> {
    let trimmed = line.trim();
    let (new_line, rest) = if let Some(rest) = trimmed.strip_prefix("next") {
        (true, rest)
    } else if let Some(rest) = trimmed.strip_prefix("db") {
        (lines.is_empty(), rest)
    } else {
        return Ok(());
    };
    let rest = rest.trim();
    if rest.is_empty() {
        return Ok(());
    }
    if new_line {
        lines.push(Vec::new());
    }
    if lines.is_empty() {
        lines.push(Vec::new());
    }
    let line_tiles = lines
        .last_mut()
        .context("credits tile parser has no active output line")?;
    if let Some(text) = visible_credits_quoted_text(rest) {
        append_visible_credits_text_tiles(line_tiles, text)?;
        return Ok(());
    }
    for token in rest.split(',') {
        let cleaned = token.trim();
        if cleaned.is_empty() || cleaned == "\"@\"" || cleaned == "@" {
            continue;
        }
        let value = parse_visible_credits_u8(cleaned)
            .with_context(|| format!("parse credits tile token {cleaned:?}"))?;
        line_tiles.push(u16::from(value));
    }
    Ok(())
}

fn visible_credits_quoted_text(line: &str) -> Option<&str> {
    let start = line.find('"')?;
    let end = line.rfind('"')?;
    (end > start).then(|| &line[start + 1..end])
}

fn append_visible_credits_text_tiles(target: &mut Vec<u16>, text: &str) -> Result<()> {
    let char_map = bitmap_font_char_map();
    for ch in text.chars() {
        if ch == '@' {
            break;
        }
        let tile_id = char_map
            .get(&ch)
            .copied()
            .with_context(|| format!("credits glyph {ch:?} is not defined"))?;
        target.push(tile_id);
    }
    Ok(())
}

fn load_visible_credits_script(asset_root: &AssetRoot) -> Result<Vec<VisibleCreditsOp>> {
    let path = asset_root.resolve_vendor("data/credits_script.asm");
    let content = crate::read_runtime_asset_to_string(&path)
        .with_context(|| format!("read credits script {}", path.display()))?;
    let mut tokens = Vec::new();
    for raw_line in content.lines() {
        let line = strip_visible_asm_comment(raw_line);
        let Some(rest) = line.strip_prefix("db") else {
            continue;
        };
        for token in rest.split(',') {
            let cleaned = token.trim();
            if !cleaned.is_empty() {
                tokens.push(cleaned.to_string());
            }
        }
    }
    let mut ops = Vec::new();
    let mut index = 0;
    while index < tokens.len() {
        let token = tokens[index].as_str();
        match token {
            "CREDITS_END" => {
                ops.push(VisibleCreditsOp::End);
                index += 1;
            }
            "CREDITS_CLEAR" => {
                ops.push(VisibleCreditsOp::Clear);
                index += 1;
            }
            "CREDITS_MUSIC" => {
                ops.push(VisibleCreditsOp::Music);
                index += 1;
            }
            "CREDITS_THEEND" => {
                ops.push(VisibleCreditsOp::TheEnd);
                index += 1;
            }
            "CREDITS_WAIT" | "CREDITS_WAIT2" | "CREDITS_SCENE" => {
                let value_token = tokens
                    .get(index + 1)
                    .with_context(|| format!("{token} missing argument at token {index}"))?;
                let value = parse_visible_credits_u8(value_token)
                    .with_context(|| format!("parse {token} argument {value_token}"))?;
                match token {
                    "CREDITS_WAIT" => ops.push(VisibleCreditsOp::Wait(value)),
                    "CREDITS_WAIT2" => ops.push(VisibleCreditsOp::Wait2(value)),
                    "CREDITS_SCENE" => ops.push(VisibleCreditsOp::Scene(value)),
                    _ => unreachable!(),
                }
                index += 2;
            }
            string_token => {
                let line_token = tokens
                    .get(index + 1)
                    .with_context(|| format!("credits string {string_token} missing line index"))?;
                let line_index = parse_visible_credits_u8(line_token).with_context(|| {
                    format!("parse credits string {string_token} line index {line_token}")
                })?;
                ops.push(VisibleCreditsOp::String {
                    token: string_token.to_string(),
                    line_index,
                });
                index += 2;
            }
        }
    }
    if ops.is_empty() {
        anyhow::bail!("credits script {} produced no operations", path.display());
    }
    Ok(ops)
}

fn parse_visible_credits_u8(token: &str) -> Result<u8> {
    let value = parse_visible_numeric_token(token)?;
    if !(0..=u8::MAX as i32).contains(&value) {
        anyhow::bail!("credits numeric token {token} is outside 8-bit range");
    }
    Ok(value as u8)
}

fn parse_visible_numeric_token(token: &str) -> Result<i32> {
    let trimmed = token.trim();
    let (sign, unsigned) = if let Some(rest) = trimmed.strip_prefix('-') {
        (-1_i32, rest)
    } else if let Some(rest) = trimmed.strip_prefix('+') {
        (1_i32, rest)
    } else {
        (1_i32, trimmed)
    };
    let (radix, digits) = if let Some(rest) = unsigned.strip_prefix('$') {
        (16, rest)
    } else if let Some(rest) = unsigned
        .strip_prefix("0x")
        .or_else(|| unsigned.strip_prefix("0X"))
    {
        (16, rest)
    } else {
        (10, unsigned)
    };
    if digits.is_empty() {
        anyhow::bail!("empty numeric token {token}");
    }
    Ok(sign * i32::from_str_radix(digits, radix)?)
}

fn strip_visible_asm_comment(line: &str) -> String {
    line.split_once(';')
        .map(|(code, _)| code)
        .unwrap_or(line)
        .trim()
        .to_string()
}

fn tick_visible_credits_screen(runtime_shell: &mut BevyRuntimeShell) {
    let Some(credits) = runtime_shell.credits_screen.as_mut() else {
        return;
    };
    if credits.awaiting_exit {
        return;
    }
    credits.frame = credits.frame.saturating_add(1);
    if let Err(error) = run_visible_credits_jumptable_step(runtime_shell) {
        record_visible_runtime_system_error(runtime_shell, error);
    }
}

fn visible_credits_can_skip(credits: &VisibleCreditsScreen) -> bool {
    credits.allow_skip && credits.consumed_bytes >= VISIBLE_CREDITS_SKIP_THRESHOLD
}

fn press_visible_credits_a_button(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let Some(credits) = runtime_shell.credits_screen.as_ref() else {
        return handle_visible_no_credits_screen(runtime_shell, "a");
    };
    if credits.awaiting_exit {
        return close_visible_credits_screen(runtime_shell, "acknowledge");
    }
    record_visible_runtime_action(runtime_shell, "credits:a:ignored")?;
    runtime_shell
        .last_audio_events
        .push("credits A before exit".to_string());
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn press_visible_credits_b_button(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let Some(credits) = runtime_shell.credits_screen.as_mut() else {
        return handle_visible_no_credits_screen(runtime_shell, "b");
    };
    if visible_credits_can_skip(credits) && credits.timer > 0 {
        credits.timer = credits.timer.saturating_sub(1);
        record_visible_runtime_action(runtime_shell, "credits:b:advance")?;
        runtime_shell
            .last_audio_events
            .push("credits B advanced timer".to_string());
    } else {
        record_visible_runtime_action(runtime_shell, "credits:b:ignored")?;
        runtime_shell
            .last_audio_events
            .push("credits B ignored".to_string());
    }
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn close_visible_credits_screen(
    runtime_shell: &mut BevyRuntimeShell,
    reason: &'static str,
) -> Result<()> {
    let Some(credits) = runtime_shell.credits_screen.take() else {
        return handle_visible_no_credits_screen(runtime_shell, reason);
    };
    stop_visible_music(runtime_shell, format!("credits:{reason}:music_stop"))?;
    record_visible_runtime_action(
        runtime_shell,
        format!(
            "credits:{reason}:close:frame={}:consumed={}",
            credits.frame, credits.consumed_bytes
        ),
    )?;
    runtime_shell.last_audio_events.push(format!(
        "closed credits reason={reason} frame={} consumed={}",
        credits.frame, credits.consumed_bytes
    ));
    if credits.resume_game_timer_on_exit {
        // Script_halloffame restores GAME_TIMER_COUNTING_F immediately after
        // Credits returns. RedCredits/Script_credits never changed the bit.
        runtime_shell.shell.set_game_timer_counting(true)?;
    }
    set_shell_action_status(runtime_shell, "CREDITS CLOSED");
    trim_event_log(&mut runtime_shell.last_audio_events);
    continue_visible_script_after_prompt(runtime_shell)
}

fn select_visible_title_menu_option(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    if let Some(continue_screen) = runtime_shell.visible_continue_screen.take() {
        let save_path = continue_screen.save_path;
        record_visible_runtime_action(
            runtime_shell,
            format!("title:continue:confirm:{}", save_path.display()),
        )?;
        runtime_shell.title_menu = None;
        runtime_shell
            .last_audio_events
            .push(format!("title continue loaded {}", save_path.display()));
        set_shell_action_status(runtime_shell, format!("CONTINUE {}", save_path.display()));
        return load_visible_runtime_save(runtime_shell, &save_path, "title_continue");
    }
    let Some(title) = runtime_shell.title_menu.clone() else {
        return handle_visible_no_active_title_menu(runtime_shell, "confirm");
    };
    if !visible_title_main_menu_ready(&title) {
        return open_visible_title_main_menu(runtime_shell);
    }
    let mut arm_new_game_arrival = false;
    let selected_title_option = selected_visible_title_menu_option(runtime_shell, &title)?;
    match selected_title_option.dispatch_target.as_str() {
        "MainMenu_Continue" => {
            let save_path = title
                .save_path
                .context("title Continue requires an exact save path")?;
            let state = match runtime_shell.shell.runtime().load_save(&save_path) {
                Ok(state) => state,
                Err(error) => {
                    record_visible_runtime_action(
                        runtime_shell,
                        format!("title:continue:invalid:{}:{error}", save_path.display()),
                    )?;
                    set_shell_action_status(
                        runtime_shell,
                        compact_scene_label(&format!("INVALID SAVE {error}"), 30),
                    );
                    return Err(error).with_context(|| {
                        format!("title Continue rejected {}", save_path.display())
                    });
                }
            };
            record_visible_runtime_action(
                runtime_shell,
                format!("title:continue:open:{}", save_path.display()),
            )?;
            let badge_count = state
                .badges
                .johto
                .iter()
                .chain(state.badges.kanto.iter())
                .filter(|owned| **owned)
                .count();
            let has_pokedex = state.flags.is_engine_flag_set("ENGINE_POKEDEX")?;
            runtime_shell.visible_continue_screen = Some(VisibleContinueScreen {
                save_path,
                player_name: state.player_name,
                badge_count,
                pokedex_count: has_pokedex.then(|| state.pokedex.caught_count()),
                hours: state.time.game_time_hours,
                minutes: state.time.game_time_minutes,
            });
            set_shell_action_status(runtime_shell, "CONTINUE DATA");
            mark_runtime_snapshot_dirty(runtime_shell);
        }
        "MainMenu_NewGame" => {
            runtime_shell
                .shell
                .reset_new_game_from_title(title.spawn_identifier)?;
            runtime_shell.title_menu = None;
            runtime_shell.new_game_pre_overworld = true;
            reset_visible_navigation_state(runtime_shell);
            runtime_shell.last_field_pack_pocket = FieldPackPocket::Items;
            runtime_shell.field_pack_cursor_positions = [0; 4];
            reset_visible_deterministic_session_history(runtime_shell)?;
            let snapshot = runtime_shell.shell.snapshot()?;
            let modpack_id = snapshot.boot.modpack_id.clone();
            let modpack_hash = runtime_shell.shell.runtime().modpack().hash().to_string();
            let pack_content_hash = snapshot.boot.pack_content_hash.clone();
            record_visible_runtime_action(
                runtime_shell,
                format!(
                    "title:new_game:{}:{}:{}:{}",
                    title.spawn_identifier, modpack_id, modpack_hash, pack_content_hash
                ),
            )?;
            reset_visible_music_state(runtime_shell);
            runtime_shell
                .last_audio_events
                .push(format!("title new game spawn={}", title.spawn_identifier));
            set_shell_action_status(
                runtime_shell,
                format!("NEW GAME SPAWN {}", title.spawn_identifier),
            );
            open_visible_gender_selection(runtime_shell)?;
            arm_new_game_arrival = runtime_shell.pending_time_set.is_none()
                && runtime_shell.pending_oak_intro.is_none()
                && runtime_shell.pending_gender_selection.is_none()
                && runtime_shell.pending_name_choice.is_none()
                && runtime_shell.pending_name_input.is_none();
        }
        "MainMenu_Option" => {
            record_visible_runtime_action(runtime_shell, "title:options")?;
            runtime_shell
                .last_audio_events
                .push("title opened Options".to_string());
            open_visible_options_menu(runtime_shell)?;
        }
        "MainMenu_MysteryGift" => {
            open_visible_mystery_gift_screen(runtime_shell)?;
        }
        target => anyhow::bail!("unsupported source main-menu dispatch target {target}"),
    }
    if arm_new_game_arrival {
        settle_visible_overworld_arrival(runtime_shell, "new_game")?;
    }
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

const MYSTERY_GIFT_PRESS_TO_LINK_TEXT: &str = "Press A to\nlink IR-Device\nPress B to\ncancel it.";
const MYSTERY_GIFT_COMMUNICATION_ERROR_TEXT: &str = "Communication\nerror.";

fn open_visible_mystery_gift_screen(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    if runtime_shell.title_menu.is_none() {
        return handle_visible_no_active_title_menu(runtime_shell, "mystery_gift");
    }
    runtime_shell.pending_mystery_gift = Some(VisibleMysteryGiftScreen {
        message: MYSTERY_GIFT_PRESS_TO_LINK_TEXT.to_string(),
        awaiting_exchange: true,
    });
    record_visible_runtime_action(runtime_shell, "title:mystery_gift:open")?;
    runtime_shell
        .last_audio_events
        .push("title opened Mystery Gift".to_string());
    set_shell_action_status(runtime_shell, "MYSTERY GIFT");
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn close_visible_mystery_gift_screen(
    runtime_shell: &mut BevyRuntimeShell,
    reason: &'static str,
) -> Result<()> {
    runtime_shell.pending_mystery_gift = None;
    record_visible_runtime_action(runtime_shell, format!("mystery_gift:{reason}:close"))?;
    runtime_shell
        .last_audio_events
        .push(format!("Mystery Gift closed {reason}"));
    set_shell_action_status(runtime_shell, "TITLE");
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn press_visible_mystery_gift_a_button(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let Some(awaiting_exchange) = runtime_shell
        .pending_mystery_gift
        .as_ref()
        .map(|mystery_gift| mystery_gift.awaiting_exchange)
    else {
        return handle_visible_no_active_title_menu(runtime_shell, "mystery_gift_confirm");
    };
    record_visible_runtime_action(runtime_shell, "mystery_gift:confirm")?;
    if awaiting_exchange {
        let mystery_gift = runtime_shell
            .pending_mystery_gift
            .as_mut()
            .context("Mystery Gift screen closed during confirm")?;
        mystery_gift.message = MYSTERY_GIFT_COMMUNICATION_ERROR_TEXT.to_string();
        mystery_gift.awaiting_exchange = false;
        runtime_shell
            .last_audio_events
            .push("Mystery Gift communication error".to_string());
        set_shell_action_status(runtime_shell, "COMMUNICATION ERROR");
        trim_event_log(&mut runtime_shell.last_audio_events);
        Ok(())
    } else {
        close_visible_mystery_gift_screen(runtime_shell, "acknowledge")
    }
}

fn visible_name_choice_options(player_gender: u8) -> Vec<String> {
    if player_gender == PLAYER_GENDER_FEMALE {
        ["NEW NAME", "KRIS", "AMANDA", "JUANA", "JODI"]
            .into_iter()
            .map(str::to_string)
            .collect()
    } else {
        ["NEW NAME", "CHRIS", "MAT", "ALLAN", "JON"]
            .into_iter()
            .map(str::to_string)
            .collect()
    }
}

fn open_visible_name_choice(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    if !snapshot.trainer.player_name.is_empty() {
        return Ok(());
    }
    runtime_shell.pending_name_choice = Some(VisibleNameChoice {
        options: visible_name_choice_options(snapshot.trainer.player_gender),
        selected: 0,
    });
    runtime_shell
        .last_audio_events
        .push("opened player naming choices".to_string());
    set_shell_action_status(runtime_shell, "PLAYER NAME");
    Ok(())
}

fn move_visible_name_choice(runtime_shell: &mut BevyRuntimeShell, delta: isize) -> Result<()> {
    let selected = {
        let Some(choice) = runtime_shell.pending_name_choice.as_mut() else {
            return handle_visible_no_player_name_input(runtime_shell, "choice");
        };
        let count = choice.options.len();
        if count == 0 {
            anyhow::bail!("player naming choice menu has no options");
        }
        choice.selected = if delta.is_negative() {
            choice
                .selected
                .checked_sub(delta.unsigned_abs())
                .unwrap_or(count - 1)
        } else {
            (choice.selected + delta as usize) % count
        };
        choice.selected
    };
    record_visible_runtime_action(runtime_shell, format!("player_name:choice:{selected}"))?;
    Ok(())
}

fn confirm_visible_name_choice(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let Some(choice) = runtime_shell.pending_name_choice.take() else {
        return handle_visible_no_player_name_input(runtime_shell, "choice-confirm");
    };
    if runtime_shell.pending_egg_hatch_nickname.is_some() {
        if choice.selected == 0 {
            let species_name = runtime_shell
                .pending_egg_hatch_nickname
                .as_ref()
                .context("egg hatch nickname choice lost its pending Pokemon")?
                .default_name
                .clone();
            runtime_shell.pending_name_input = Some(PendingNameInput {
                label: visible_pokemon_nickname_label(&species_name),
                // NamingScreen_InitNameEntry clears the destination buffer;
                // the species name is display context, not prefilled input.
                value: String::new(),
                max_length: 10,
                cursor_column: 0,
                cursor_row: 0,
                case: NameInputCase::Upper,
            });
            set_shell_action_status(runtime_shell, "POKEMON NAME");
            mark_runtime_snapshot_dirty(runtime_shell);
            return Ok(());
        }
        return finish_visible_egg_hatch_nickname(runtime_shell, None);
    }
    if runtime_shell.pending_gift_pokemon_nickname.is_some() {
        if choice.selected == 0 {
            let species_name = runtime_shell
                .pending_gift_pokemon_nickname
                .as_ref()
                .context("gift nickname choice lost its pending Pokemon")?
                .default_name
                .clone();
            runtime_shell.pending_name_input = Some(PendingNameInput {
                label: visible_pokemon_nickname_label(&species_name),
                value: String::new(),
                max_length: 10,
                cursor_column: 0,
                cursor_row: 0,
                case: NameInputCase::Upper,
            });
            set_shell_action_status(runtime_shell, "POKEMON NAME");
            mark_runtime_snapshot_dirty(runtime_shell);
            return Ok(());
        }
        return finish_visible_gift_pokemon_nickname(runtime_shell, None);
    }
    if runtime_shell.pending_standard_capture.is_some() {
        if choice.selected == 0 {
            let species_name = runtime_shell
                .pending_standard_capture
                .as_ref()
                .context("capture nickname choice lost its pending Pokemon")?
                .default_name
                .clone();
            runtime_shell.pending_name_input = Some(PendingNameInput {
                label: visible_pokemon_nickname_label(&species_name),
                value: String::new(),
                max_length: 10,
                cursor_column: 0,
                cursor_row: 0,
                case: NameInputCase::Upper,
            });
            set_shell_action_status(runtime_shell, "POKEMON NAME");
            mark_runtime_snapshot_dirty(runtime_shell);
            return Ok(());
        }
        return finish_visible_capture_nickname(runtime_shell, None);
    }
    let selected = choice
        .options
        .get(choice.selected)
        .context("player naming choice selection is out of range")?
        .clone();
    if choice.selected == 0 {
        return open_visible_player_name_input(runtime_shell);
    }
    apply_visible_player_name(runtime_shell, selected)
}

fn visible_pokemon_nickname_label(species_name: &str) -> String {
    format!("{species_name}'S\nNICKNAME?")
}

fn open_visible_player_name_input(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    if !snapshot.trainer.player_name.is_empty() {
        return Ok(());
    }
    runtime_shell.pending_name_input = Some(PendingNameInput {
        label: "YOUR NAME?".to_string(),
        value: String::new(),
        max_length: VISIBLE_NAME_ENTRY_MAX_LENGTH,
        cursor_column: 0,
        cursor_row: 0,
        case: NameInputCase::Upper,
    });
    runtime_shell
        .last_audio_events
        .push("opened player name input".to_string());
    set_shell_action_status(runtime_shell, "PLAYER NAME");
    Ok(())
}

fn default_visible_player_name(player_gender: u8) -> &'static str {
    if player_gender == PLAYER_GENDER_FEMALE {
        DEFAULT_FEMALE_PLAYER_NAME
    } else {
        DEFAULT_MALE_PLAYER_NAME
    }
}

fn apply_visible_name_input_keys(
    keys: &ButtonInput<KeyCode>,
    runtime_shell: &mut BevyRuntimeShell,
) {
    let shift_pressed = keys.pressed(KeyCode::ShiftRight);
    let alt_pressed = keys.pressed(KeyCode::AltLeft) || keys.pressed(KeyCode::AltRight);
    let ctrl_pressed = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    if alt_pressed || ctrl_pressed {
        return;
    }
    if keys.just_pressed(KeyCode::ArrowLeft) {
        run_bevy_action(runtime_shell, move_visible_player_name_cursor_left);
        return;
    }
    if keys.just_pressed(KeyCode::ArrowRight) {
        run_bevy_action(runtime_shell, move_visible_player_name_cursor_right);
        return;
    }
    if keys.just_pressed(KeyCode::ArrowUp) {
        run_bevy_action(runtime_shell, move_visible_player_name_cursor_up);
        return;
    }
    if keys.just_pressed(KeyCode::ArrowDown) {
        run_bevy_action(runtime_shell, move_visible_player_name_cursor_down);
        return;
    }
    if keys.just_pressed(KeyCode::ShiftRight) {
        run_bevy_action(runtime_shell, toggle_visible_player_name_case);
        return;
    }
    if keys.just_pressed(KeyCode::KeyZ) && !shift_pressed {
        run_bevy_action(runtime_shell, select_visible_player_name_grid_key);
        return;
    }
    if keys.just_pressed(KeyCode::KeyX) && !shift_pressed {
        run_bevy_action(runtime_shell, delete_visible_player_name_char);
        return;
    }
    if keys.just_pressed(KeyCode::Enter) {
        let cursor_is_on_end = runtime_shell.pending_name_input.as_ref().is_some_and(|input| {
            input.cursor_row == visible_name_input_bottom_row_index()
                && visible_name_input_bottom_group(input.cursor_column) == 3
        });
        if cursor_is_on_end {
            run_bevy_action(runtime_shell, select_visible_player_name_grid_key);
        } else {
            run_bevy_action(runtime_shell, move_visible_player_name_cursor_to_end);
        }
        return;
    }
}

const MAIL_INPUT_COLUMNS: usize = 10;
const MAIL_INPUT_ROWS: usize = 6;
const MAIL_INPUT_MESSAGE_LENGTH: usize = 32;
const MAIL_INPUT_LINE_LENGTH: usize = 16;

fn apply_visible_mail_input_keys(
    keys: &ButtonInput<KeyCode>,
    runtime_shell: &mut BevyRuntimeShell,
) {
    let shift_pressed = keys.pressed(KeyCode::ShiftRight);
    let alt_pressed = keys.pressed(KeyCode::AltLeft) || keys.pressed(KeyCode::AltRight);
    let ctrl_pressed = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    if alt_pressed || ctrl_pressed {
        return;
    }
    let action = if keys.just_pressed(KeyCode::ArrowLeft) {
        Some((-1, 0))
    } else if keys.just_pressed(KeyCode::ArrowRight) {
        Some((1, 0))
    } else if keys.just_pressed(KeyCode::ArrowUp) {
        Some((0, -1))
    } else if keys.just_pressed(KeyCode::ArrowDown) {
        Some((0, 1))
    } else {
        None
    };
    if let Some((dx, dy)) = action {
        run_bevy_action(runtime_shell, |shell| move_visible_mail_cursor(shell, dx, dy));
        return;
    }
    if keys.just_pressed(KeyCode::ShiftRight) {
        run_bevy_action(runtime_shell, toggle_visible_mail_case);
        return;
    }
    if keys.just_pressed(KeyCode::KeyZ) && !shift_pressed {
        run_bevy_action(runtime_shell, select_visible_mail_grid_key);
        return;
    }
    if keys.just_pressed(KeyCode::KeyX) && !shift_pressed {
        run_bevy_action(runtime_shell, delete_visible_mail_character);
        return;
    }
    if keys.just_pressed(KeyCode::Enter) {
        run_bevy_action(runtime_shell, |shell| {
            let input = shell
                .pending_mail_input
                .as_mut()
                .context("no Mail composer is open")?;
            input.cursor_row = MAIL_INPUT_ROWS - 1;
            input.cursor_column = 9;
            record_visible_runtime_action(shell, "party:mail:start:end")
        });
    }
}

fn visible_mail_input_layout(case: NameInputCase) -> &'static [&'static str; 6] {
    match case {
        NameInputCase::Upper => &[
            "A B C D E F G H I J",
            "K L M N O P Q R S T",
            "U V W X Y Z   , ? !",
            "1 2 3 4 5 6 7 8 9 0",
            "<PK> <MN> <PO> <KE> é ♂ ♀ ¥ … ×",
            "lower  DEL   END   ",
        ],
        NameInputCase::Lower => &[
            "a b c d e f g h i j",
            "k l m n o p q r s t",
            "u v w x y z   . - /",
            "'d 'l 'm 'r 's 't 'v & ( )",
            "“ ” [ ] ' : ;       ",
            "UPPER  DEL   END   ",
        ],
    }
}

fn visible_mail_input_row_chars(case: NameInputCase, row: usize) -> [Option<char>; 10] {
    let text = match (case, row) {
        (NameInputCase::Upper, 0) => "ABCDEFGHIJ",
        (NameInputCase::Upper, 1) => "KLMNOPQRST",
        (NameInputCase::Upper, 2) => "UVWXYZ ,?!",
        (NameInputCase::Upper, 3) => "1234567890",
        (NameInputCase::Upper, 4) => "\u{e105}\u{e106}\u{e108}\u{e109}é♂♀¥…×",
        (NameInputCase::Lower, 0) => "abcdefghij",
        (NameInputCase::Lower, 1) => "klmnopqrst",
        (NameInputCase::Lower, 2) => "uvwxyz .-/",
        (NameInputCase::Lower, 3) => "\u{e120}\u{e121}\u{e122}\u{e123}\u{e124}\u{e125}\u{e126}&()",
        (NameInputCase::Lower, 4) => "“”[]':;   ",
        _ => "          ",
    };
    let mut row_chars = [None; 10];
    for (index, character) in text.chars().take(10).enumerate() {
        row_chars[index] = Some(character);
    }
    row_chars
}

fn visible_mail_bottom_group(column: usize) -> usize {
    if column < 3 { 0 } else if column < 6 { 1 } else { 2 }
}

fn move_visible_mail_cursor(
    runtime_shell: &mut BevyRuntimeShell,
    dx: isize,
    dy: isize,
) -> Result<()> {
    let input = runtime_shell
        .pending_mail_input
        .as_ref()
        .context("no Mail composer is open")?;
    let mut row = input.cursor_row;
    let mut column = input.cursor_column;
    if dy != 0 {
        row = (row as isize + dy).rem_euclid(MAIL_INPUT_ROWS as isize) as usize;
    }
    if dx != 0 {
        if row == MAIL_INPUT_ROWS - 1 {
            let group = visible_mail_bottom_group(column);
            let next_group = (group as isize + dx).rem_euclid(3) as usize;
            column = next_group * 3;
        } else {
            column = (column as isize + dx).rem_euclid(MAIL_INPUT_COLUMNS as isize) as usize;
        }
    }
    record_visible_runtime_action(runtime_shell, format!("party:mail:cursor:{row}:{column}"))?;
    let input = runtime_shell
        .pending_mail_input
        .as_mut()
        .context("no Mail composer is open")?;
    input.cursor_row = row;
    input.cursor_column = column;
    Ok(())
}

fn toggle_visible_mail_case(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    record_visible_runtime_action(runtime_shell, "party:mail:case")?;
    let input = runtime_shell
        .pending_mail_input
        .as_mut()
        .context("no Mail composer is open")?;
    input.case = match input.case {
        NameInputCase::Upper => NameInputCase::Lower,
        NameInputCase::Lower => NameInputCase::Upper,
    };
    Ok(())
}

fn select_visible_mail_grid_key(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let (row, column, case) = runtime_shell
        .pending_mail_input
        .as_ref()
        .map(|input| (input.cursor_row, input.cursor_column, input.case))
        .context("no Mail composer is open")?;
    record_visible_runtime_action(runtime_shell, format!("party:mail:grid:{row}:{column}"))?;
    if row == MAIL_INPUT_ROWS - 1 {
        return match visible_mail_bottom_group(column) {
            0 => toggle_visible_mail_case(runtime_shell),
            1 => delete_visible_mail_character(runtime_shell),
            _ => confirm_visible_mail_input(runtime_shell),
        };
    }
    let Some(character) = visible_mail_input_row_chars(case, row)[column] else {
        return Ok(());
    };
    let input = runtime_shell
        .pending_mail_input
        .as_mut()
        .context("no Mail composer is open")?;
    if input.value.chars().count() < MAIL_INPUT_MESSAGE_LENGTH {
        input.value.push(character);
        if input.value.chars().count() == MAIL_INPUT_MESSAGE_LENGTH {
            input.cursor_row = MAIL_INPUT_ROWS - 1;
            input.cursor_column = 9;
        }
    }
    Ok(())
}

fn delete_visible_mail_character(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    record_visible_runtime_action(runtime_shell, "party:mail:delete")?;
    runtime_shell
        .pending_mail_input
        .as_mut()
        .context("no Mail composer is open")?
        .value
        .pop();
    Ok(())
}

fn confirm_visible_mail_input(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let input = runtime_shell
        .pending_mail_input
        .take()
        .context("no Mail composer is open")?;
    let mut glyphs = input.value.chars();
    let first = glyphs
        .by_ref()
        .take(MAIL_INPUT_LINE_LENGTH)
        .collect::<String>();
    let second = glyphs.collect::<String>();
    let message = if second.is_empty() {
        first
    } else {
        format!("{first}\n{second}")
    };
    record_visible_runtime_action(
        runtime_shell,
        format!("party:mail:confirm:{}:{}", input.party_index, input.item_id),
    )?;
    let transfer = runtime_shell.shell.compose_bag_mail_to_party(
        &input.item_id,
        input.party_index,
        message,
    )?;
    runtime_shell.party_held_item_give_target = None;
    runtime_shell.held_item_swap_prompt = false;
    runtime_shell.yes_no_cursor = None;
    close_visible_field_pack_without_log(runtime_shell);
    runtime_shell.battle_pack_target_mode = None;
    runtime_shell.field_notice = Some(format!(
        "Made party #{} hold {}.",
        input.party_index, transfer.item_id
    ));
    runtime_shell.last_audio_events.push(format!(
        "composed Mail item={} party_index={} checksum={:?}",
        transfer.item_id, input.party_index, transfer.state_checksum
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    mark_runtime_snapshot_dirty(runtime_shell);
    continue_visible_script_after_prompt(runtime_shell)
}

fn close_visible_mail_read(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let mail = runtime_shell
        .pending_mail_read
        .take()
        .context("no Mail reader is open")?;
    record_visible_runtime_action(
        runtime_shell,
        format!("mail:read:close:{}", mail.mail.mail_type),
    )?;
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(())
}

fn apply_visible_name_input_smoke_char(
    runtime_shell: &mut BevyRuntimeShell,
    ch: char,
) -> Result<()> {
    append_visible_player_name_char(runtime_shell, ch)
}

fn apply_visible_name_input_smoke_key(runtime_shell: &mut BevyRuntimeShell, key: KeyCode) {
    let mut keys = ButtonInput::<KeyCode>::default();
    keys.press(key);
    apply_visible_name_input_keys(&keys, runtime_shell);
}

fn visible_name_input_grid_width() -> usize {
    9
}

fn visible_name_input_grid_height() -> usize {
    5
}

fn visible_name_input_bottom_row_index() -> usize {
    visible_name_input_grid_height() - 1
}

fn visible_name_input_layout(case: NameInputCase) -> &'static [&'static str; 5] {
    match case {
        NameInputCase::Upper => &[
            "A B C D E F G H I",
            "J K L M N O P Q R",
            "S T U V W X Y Z  ",
            "- ? ! / . ,      ",
            "lower  DEL   END ",
        ],
        NameInputCase::Lower => &[
            "a b c d e f g h i",
            "j k l m n o p q r",
            "s t u v w x y z  ",
            "× ( ) : ; [ ] <PK> <MN>",
            "UPPER  DEL   END ",
        ],
    }
}

fn visible_name_input_row_chars(case: NameInputCase, row: usize) -> [Option<char>; 9] {
    match (case, row) {
        (NameInputCase::Upper, 0) => chars_to_name_row("ABCDEFGHI"),
        (NameInputCase::Upper, 1) => chars_to_name_row("JKLMNOPQR"),
        (NameInputCase::Upper, 2) => chars_to_name_row("STUVWXYZ "),
        (NameInputCase::Upper, 3) => chars_to_name_row("-?!/.,   "),
        (NameInputCase::Lower, 0) => chars_to_name_row("abcdefghi"),
        (NameInputCase::Lower, 1) => chars_to_name_row("jklmnopqr"),
        (NameInputCase::Lower, 2) => chars_to_name_row("stuvwxyz "),
        (NameInputCase::Lower, 3) => [
            Some('×'),
            Some('('),
            Some(')'),
            Some(':'),
            Some(';'),
            Some('['),
            Some(']'),
            None,
            None,
        ],
        _ => [None; 9],
    }
}

fn chars_to_name_row(chars: &str) -> [Option<char>; 9] {
    let mut row = [None; 9];
    for (index, ch) in chars.chars().take(9).enumerate() {
        if ch != ' ' {
            row[index] = Some(ch);
        }
    }
    row
}

fn visible_name_input_bottom_group(column: usize) -> usize {
    if column < 3 {
        1
    } else if column < 6 {
        2
    } else {
        3
    }
}

fn visible_name_input_bottom_group_next_column(column: usize, dx: isize) -> usize {
    let group = visible_name_input_bottom_group(column);
    if dx.is_positive() {
        match group {
            1 => 3,
            2 => 6,
            _ => 0,
        }
    } else {
        match group {
            1 => 6,
            2 => 0,
            _ => 3,
        }
    }
}

fn toggle_visible_player_name_case(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let next_case = {
        let Some(input) = runtime_shell.pending_name_input.as_mut() else {
            return handle_visible_no_player_name_input(runtime_shell, "case");
        };
        input.case = match input.case {
            NameInputCase::Upper => NameInputCase::Lower,
            NameInputCase::Lower => NameInputCase::Upper,
        };
        input.case
    };
    record_visible_runtime_action(runtime_shell, "player_name:case")?;
    runtime_shell
        .last_audio_events
        .push(format!("player name case {next_case:?}"));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn move_visible_player_name_cursor_to_end(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let Some(input) = runtime_shell.pending_name_input.as_mut() else {
        return handle_visible_no_player_name_input(runtime_shell, "start");
    };
    input.cursor_row = visible_name_input_bottom_row_index();
    input.cursor_column = 8;
    record_visible_runtime_action(runtime_shell, "player_name:start:end")?;
    runtime_shell
        .last_audio_events
        .push("player name cursor END".to_string());
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn move_visible_player_name_cursor_left(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    move_visible_player_name_cursor(runtime_shell, -1, 0)
}

fn move_visible_player_name_cursor_right(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    move_visible_player_name_cursor(runtime_shell, 1, 0)
}

fn move_visible_player_name_cursor_up(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    move_visible_player_name_cursor(runtime_shell, 0, -1)
}

fn move_visible_player_name_cursor_down(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    move_visible_player_name_cursor(runtime_shell, 0, 1)
}

fn move_visible_player_name_cursor(
    runtime_shell: &mut BevyRuntimeShell,
    dx: isize,
    dy: isize,
) -> Result<()> {
    let Some(input) = runtime_shell.pending_name_input.as_ref() else {
        return handle_visible_no_player_name_input(runtime_shell, "cursor");
    };
    let width = visible_name_input_grid_width();
    let height = visible_name_input_grid_height();
    let mut next_row = input.cursor_row;
    let mut next_col = input.cursor_column;
    if dy != 0 {
        next_row = (next_row as isize + dy).rem_euclid(height as isize) as usize;
    }
    if dx != 0 {
        if next_row == visible_name_input_bottom_row_index() {
            next_col = visible_name_input_bottom_group_next_column(next_col, dx);
        } else {
            next_col = (next_col as isize + dx).rem_euclid(width as isize) as usize;
        }
    }
    record_visible_runtime_action(
        runtime_shell,
        format!("player_name:cursor:{next_row}:{next_col}"),
    )?;
    let input = runtime_shell
        .pending_name_input
        .as_mut()
        .context("no player name input is open")?;
    input.cursor_column = next_col;
    input.cursor_row = next_row;
    runtime_shell
        .last_audio_events
        .push(format!("player name cursor row={next_row} col={next_col}"));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn select_visible_player_name_grid_key(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let (row, col, case) = {
        let input = runtime_shell
            .pending_name_input
            .as_ref()
            .context("no player name input is open")?;
        (input.cursor_row, input.cursor_column, input.case)
    };
    record_visible_runtime_action(runtime_shell, format!("player_name:grid:{row}:{col}"))?;
    if row == visible_name_input_bottom_row_index() {
        return match visible_name_input_bottom_group(col) {
            1 => toggle_visible_player_name_case(runtime_shell),
            2 => delete_visible_player_name_char(runtime_shell),
            _ => confirm_visible_player_name_input(runtime_shell),
        };
    }
    if let Some(ch) = visible_name_input_row_chars(case, row)[col] {
        append_visible_player_name_char(runtime_shell, ch)
    } else {
        Ok(())
    }
}

fn append_visible_player_name_char(runtime_shell: &mut BevyRuntimeShell, ch: char) -> Result<()> {
    let Some(input) = runtime_shell.pending_name_input.as_ref() else {
        return handle_visible_no_player_name_input(runtime_shell, "append");
    };
    if input.value.chars().count() >= input.max_length {
        record_visible_runtime_action(runtime_shell, format!("player_name:full:{ch}"))?;
        runtime_shell
            .last_audio_events
            .push("player name full".to_string());
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    record_visible_runtime_action(runtime_shell, format!("player_name:append:{ch}"))?;
    let input = runtime_shell
        .pending_name_input
        .as_mut()
        .context("no player name input is open")?;
    input.value.push(ch);
    if input.value.chars().count() >= input.max_length {
        input.cursor_row = visible_name_input_bottom_row_index();
        input.cursor_column = 8;
    }
    runtime_shell
        .last_audio_events
        .push(format!("player name add {ch}"));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn delete_visible_player_name_char(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    if runtime_shell.pending_name_input.is_none() {
        return handle_visible_no_player_name_input(runtime_shell, "delete");
    }
    record_visible_runtime_action(runtime_shell, "player_name:delete")?;
    let input = runtime_shell
        .pending_name_input
        .as_mut()
        .context("no player name input is open")?;
    input.value.pop();
    runtime_shell
        .last_audio_events
        .push("player name delete".to_string());
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn complete_visible_smoke_player_name_if_needed(
    runtime_shell: &mut BevyRuntimeShell,
    smoke_player_name: Option<&str>,
) -> Result<()> {
    complete_visible_smoke_gender_if_needed(runtime_shell)?;
    complete_visible_smoke_time_set_if_needed(runtime_shell)?;
    complete_visible_smoke_oak_intro_if_needed(runtime_shell)?;
    if runtime_shell.pending_name_choice.is_some() {
        confirm_visible_name_choice(runtime_shell)?;
    }
    if runtime_shell.pending_name_input.is_none() {
        let snapshot = runtime_shell.shell.snapshot()?;
        if snapshot.trainer.player_name.is_empty() {
            if let Some(smoke_player_name) = smoke_player_name {
                runtime_shell.shell.session_mut().state.player_name = smoke_player_name.to_string();
                runtime_shell.snapshot_revision = runtime_shell.snapshot_revision.wrapping_add(1);
                return Ok(());
            }
            anyhow::bail!("visible smoke reached gameplay with no player name");
        }
        return Ok(());
    }
    let smoke_player_name =
        smoke_player_name.context("visible smoke requires explicit --smoke-player-name")?;
    for ch in smoke_player_name.chars() {
        apply_visible_name_input_smoke_char(runtime_shell, ch)?;
    }
    apply_visible_name_input_smoke_key(runtime_shell, KeyCode::Enter);
    apply_visible_name_input_smoke_key(runtime_shell, KeyCode::KeyZ);
    complete_visible_smoke_oak_intro_if_needed(runtime_shell)?;
    Ok(())
}

fn complete_visible_smoke_time_set_if_needed(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    if runtime_shell.pending_time_set.is_none() {
        return Ok(());
    }
    const MAX_TIME_SET_SMOKE_STEPS: usize = 512;
    for _ in 0..MAX_TIME_SET_SMOKE_STEPS {
        let Some(phase) = runtime_shell
            .pending_time_set
            .as_ref()
            .map(|time_set| time_set.phase)
        else {
            return Ok(());
        };
        match phase {
            VisibleTimeSetPhase::WakeDialogue
            | VisibleTimeSetPhase::HourConfirm
            | VisibleTimeSetPhase::MinuteConfirm
            | VisibleTimeSetPhase::FinalReaction
            | VisibleTimeSetPhase::SetHour
            | VisibleTimeSetPhase::SetMinute
            | VisibleTimeSetPhase::Complete => {
                press_visible_time_set_a_button(runtime_shell)?;
            }
        }
        if runtime_shell.pending_time_set.is_none() {
            return Ok(());
        }
        if let Some(time_set) = runtime_shell.pending_time_set.as_mut() {
            advance_visible_time_set_dialog(time_set);
        }
    }
    anyhow::bail!("visible smoke time set did not advance")
}

fn complete_visible_smoke_gender_if_needed(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    if runtime_shell.pending_gender_selection.is_none() {
        return Ok(());
    }
    let confirm_delay_frames = runtime_shell
        .pending_gender_selection
        .as_ref()
        .expect("checked gender selection")
        .definition
        .confirm_delay_frames;
    confirm_visible_gender_selection(runtime_shell)?;
    for _ in 0..=confirm_delay_frames {
        tick_visible_gender_selection(runtime_shell)?;
        if runtime_shell.pending_gender_selection.is_none() {
            return Ok(());
        }
    }
    anyhow::bail!("visible smoke gender selection did not advance")
}

fn complete_visible_smoke_oak_intro_if_needed(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    if runtime_shell.pending_oak_intro.is_none() {
        return Ok(());
    }
    const MAX_OAK_INTRO_SMOKE_STEPS: usize = 256;
    for _ in 0..MAX_OAK_INTRO_SMOKE_STEPS {
        if runtime_shell.pending_oak_intro.is_none() {
            return Ok(());
        }
        tick_visible_oak_intro(runtime_shell)?;
        if let Some(oak_intro) = runtime_shell.pending_oak_intro.as_mut() {
            if !oak_intro.current_text.is_empty() && !visible_oak_intro_dialog_complete(oak_intro) {
                oak_intro.visible_chars = oak_intro.current_text.chars().count();
                oak_intro.waiting_for_input = true;
            }
        }
        let should_press = runtime_shell
            .pending_oak_intro
            .as_ref()
            .is_some_and(|oak_intro| oak_intro.waiting_for_input || oak_intro.finished);
        if should_press {
            press_visible_oak_intro_a_button(runtime_shell)?;
        }
    }
    anyhow::bail!("visible smoke Oak intro did not advance")
}

fn confirm_visible_player_name_input(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let Some(input) = runtime_shell.pending_name_input.take() else {
        return handle_visible_no_player_name_input(runtime_shell, "confirm");
    };
    if input.value.trim() != input.value {
        runtime_shell.pending_name_input = Some(input);
        anyhow::bail!("player name must be exact");
    }
    if input.label == "BOX NAME?" {
        let box_index = strict_readonly_cursor_index(
            &runtime_shell.bill_pc_box_cursor,
            "pc:bill-boxes",
            crate::core::models::MAX_PC_BOXES,
        )
        .context("box naming screen lost its selected PC box")?;
        let snapshot = runtime_shell.shell.snapshot()?;
        let old_name = snapshot
            .storage
            .boxes
            .iter()
            .find(|pc_box| pc_box.index == box_index)
            .map(|pc_box| pc_box.name.clone())
            .with_context(|| format!("selected PC box {box_index} is missing"))?;
        let name = if input.value.is_empty() {
            old_name
        } else {
            input.value
        };
        record_visible_runtime_action(
            runtime_shell,
            format!("pc:bill:box_name:{box_index}:{name}"),
        )?;
        let named = runtime_shell.shell.name_pc_box(box_index, name)?;
        runtime_shell.last_audio_events.push(format!(
            "named PC box {} {}->{} checksum={:?}",
            named.box_index, named.previous_name, named.name, named.state_checksum
        ));
        set_shell_action_status(runtime_shell, "CHOOSE A BOX");
        mark_runtime_snapshot_dirty(runtime_shell);
        return Ok(());
    }
    if input.label == "RIVAL'S NAME?" {
        let rival_name = if input.value.is_empty() {
            "SILVER".to_string()
        } else {
            input.value
        };
        record_visible_runtime_action(
            runtime_shell,
            format!("script:special:name_rival:confirm:{rival_name}"),
        )?;
        let special = runtime_shell.shell.name_rival_special(rival_name)?;
        runtime_shell.last_audio_events.push(format!(
            "named rival outcome={:?} checksum={:?}",
            special.outcome.effect, special.state_checksum
        ));
        mark_runtime_snapshot_dirty(runtime_shell);
        return continue_visible_script_after_prompt(runtime_shell);
    }
    if input.label == "POKéMON'S NAME?" {
        let party_index = selected_party_index(runtime_shell)?;
        let snapshot = runtime_shell.shell.snapshot()?;
        let old_nickname = snapshot
            .party
            .slots
            .iter()
            .find(|slot| slot.index == party_index)
            .map(|slot| slot.pokemon.nickname.clone())
            .context("Name Rater selection is no longer in the party")?;
        let nickname = if input.value.trim().is_empty() {
            old_nickname.clone()
        } else {
            input.value
        };
        record_visible_runtime_action(
            runtime_shell,
            format!("script:special:name_rater:confirm:{party_index}:{nickname}"),
        )?;
        let special = runtime_shell
            .shell
            .rate_party_nickname_special(party_index, nickname.clone())?;
        runtime_shell.last_audio_events.push(format!(
            "name rater outcome={:?} checksum={:?}",
            special.outcome.effect, special.state_checksum
        ));
        let mut boundaries = visible_exported_special_text_boundaries_with_buffer(
            runtime_shell,
            "NameRaterNamedText",
            "_NameRaterNamedText",
            Some(&nickname),
        )?;
        let (completion_label, completion_target) = if nickname == old_nickname {
            ("NameRaterSameNameText", "_NameRaterSameNameText")
        } else {
            ("NameRaterFinishedText", "_NameRaterFinishedText")
        };
        boundaries.extend(visible_exported_special_text_boundaries(
            runtime_shell,
            completion_label,
            completion_target,
        )?);
        runtime_shell.special_boundary = boundaries.pop_front();
        runtime_shell.special_boundary_queue = boundaries;
        mark_runtime_snapshot_dirty(runtime_shell);
        return Ok(());
    }
    if runtime_shell.pending_standard_capture.is_some() {
        let nickname = if input.value.is_empty() {
            runtime_shell
                .pending_standard_capture
                .as_ref()
                .context("capture naming screen lost its pending capture")?
                .default_name
                .clone()
        } else {
            input.value
        };
        return finish_visible_capture_nickname(runtime_shell, Some(nickname));
    }
    if let Some(pending) = runtime_shell.pending_egg_hatch_nickname.as_ref() {
        let nickname = if input.value.is_empty() {
            pending.default_name.clone()
        } else {
            input.value
        };
        return finish_visible_egg_hatch_nickname(runtime_shell, Some(nickname));
    }
    if let Some(pending) = runtime_shell.pending_gift_pokemon_nickname.as_ref() {
        let nickname = if input.value.is_empty() {
            pending.default_name.clone()
        } else {
            input.value
        };
        return finish_visible_gift_pokemon_nickname(runtime_shell, Some(nickname));
    }
    apply_visible_player_name(runtime_shell, input.value)
}

fn finish_visible_egg_hatch_nickname(
    runtime_shell: &mut BevyRuntimeShell,
    nickname: Option<String>,
) -> Result<()> {
    let pending = runtime_shell
        .pending_egg_hatch_nickname
        .take()
        .context("no hatched Pokemon is awaiting a nickname")?;
    if let Some(nickname) = nickname.as_ref() {
        let renamed = runtime_shell
            .shell
            .rename_party_pokemon(pending.party_index, nickname.clone())?;
        runtime_shell.last_audio_events.push(format!(
            "hatched Pokemon nickname party_index={} name={} checksum={:?}",
            pending.party_index, nickname, renamed.state_checksum
        ));
    }
    record_visible_runtime_action(
        runtime_shell,
        format!(
            "overworld:egg_hatch_nickname:{}:{}",
            pending.party_index,
            nickname.as_deref().unwrap_or("declined")
        ),
    )?;
    runtime_shell.pending_name_input = None;
    runtime_shell.pending_name_choice = None;
    queue_visible_current_music(runtime_shell)?;
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(())
}

fn finish_visible_capture_nickname(
    runtime_shell: &mut BevyRuntimeShell,
    nickname: Option<String>,
) -> Result<()> {
    if nickname.is_none()
        && runtime_shell
            .pending_standard_capture
            .as_ref()
            .is_some_and(|pending| pending.prompt_for_nickname)
        && runtime_shell
            .runtime
            .data()
            .nuzlocke_rules()
            .require_capture_nickname
    {
        let species_name = runtime_shell
            .pending_standard_capture
            .as_ref()
            .context("Nuzlocke capture nickname prompt lost its pending Pokemon")?
            .default_name
            .clone();
        runtime_shell.pending_name_input = Some(PendingNameInput {
            label: visible_pokemon_nickname_label(&species_name),
            value: String::new(),
            max_length: 10,
            cursor_column: 0,
            cursor_row: 0,
            case: NameInputCase::Upper,
        });
        runtime_shell.pending_name_choice = None;
        set_shell_action_status(runtime_shell, "NUZLOCKE: NICKNAME REQUIRED");
        mark_runtime_snapshot_dirty(runtime_shell);
        return Ok(());
    }
    let pending = runtime_shell
        .pending_standard_capture
        .take()
        .context("no standard capture is awaiting a nickname")?;
    record_visible_runtime_action(
        runtime_shell,
        format!("battle:capture_nickname:{}", nickname.as_deref().unwrap_or("declined")),
    )?;
    complete_visible_standard_capture(
        runtime_shell,
        pending.outcome,
        nickname,
        pending.scripted_static_wild,
    )?;
    runtime_shell.pending_name_input = None;
    runtime_shell.pending_name_choice = None;
    if runtime_shell.battle_messages.is_empty() {
        runtime_shell.battle_message_scene = None;
        runtime_shell.visible_capture_animation = None;
    }
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(())
}

fn apply_visible_player_name(
    runtime_shell: &mut BevyRuntimeShell,
    player_name: String,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    record_visible_runtime_action(
        runtime_shell,
        format!(
            "trainer:identity:{}:{}",
            player_name, snapshot.trainer.player_id
        ),
    )?;
    let player_name = if player_name.is_empty() {
        default_visible_player_name(snapshot.trainer.player_gender).to_string()
    } else {
        player_name
    };
    let identity = runtime_shell
        .shell
        .set_trainer_identity(player_name, snapshot.trainer.player_id)?;
    let identity_checksum = identity.state_checksum.clone();
    runtime_shell.deterministic_session_start = identity_checksum.clone();
    runtime_shell.deterministic_session_checkpoint = Some(
        visible_deterministic_session_checkpoint(&runtime_shell.shell, identity_checksum)
            .context("refresh deterministic session checkpoint after trainer identity")?,
    );
    runtime_shell.last_audio_events.push(format!(
        "trainer identity name={} id={} checksum={:?}",
        identity.player_name_after, identity.player_id_after, identity.state_checksum
    ));
    set_shell_action_status(runtime_shell, "OAK FINALE");
    open_visible_oak_final_sequence(runtime_shell, &identity.player_name_after)?;
    Ok(())
}

fn reset_visible_music_state(runtime_shell: &mut BevyRuntimeShell) {
    runtime_shell.active_music = None;
    runtime_shell.faded_music = None;
    runtime_shell.music_fade = None;
    runtime_shell.music_volume = 7;
    runtime_shell.pending_music_stop = true;
    runtime_shell.pending_full_audio_reset = true;
    clear_pending_music_commands(&mut runtime_shell.pending_audio);
}

fn elevator_surface_id(source_script: &str, command_index: usize) -> String {
    format!("ui:elevators:{source_script}:{command_index}")
}

fn visible_elevator_prompt_options<'a>(
    snapshot: &'a RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
) -> Vec<&'a RuntimeElevatorSnapshot> {
    let Some(cursor) = runtime_shell.elevator_cursor.as_ref() else {
        return Vec::new();
    };
    snapshot
        .ui
        .elevators
        .iter()
        .filter(|elevator| {
            elevator.map_name == snapshot.overworld.map_name
                && elevator_surface_id(&elevator.source_script, elevator.elevator_command_index)
                    == cursor.surface_id
        })
        .collect()
}

fn has_visible_elevator_prompt(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
) -> bool {
    !visible_elevator_prompt_options(snapshot, runtime_shell).is_empty()
}

fn visible_elevator_option_count(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
) -> usize {
    visible_elevator_prompt_options(snapshot, runtime_shell)
        .iter()
        .map(|elevator| elevator.floors.len())
        .sum()
}

fn selected_visible_elevator_option(
    runtime_shell: &mut BevyRuntimeShell,
    snapshot: &RuntimeShellSnapshot,
) -> Result<(usize, usize)> {
    let elevators = visible_elevator_prompt_options(snapshot, runtime_shell);
    let option_count = visible_elevator_option_count(snapshot, runtime_shell);
    if option_count == 0 {
        anyhow::bail!("compiled visible elevators have no floors");
    }
    let surface_id = runtime_shell
        .elevator_cursor
        .as_ref()
        .map(|cursor| cursor.surface_id.clone())
        .context("elevator prompt requires a cursor surface")?;
    let selected =
        strict_readonly_cursor_index(&runtime_shell.elevator_cursor, &surface_id, option_count)
            .context("elevator prompt is active without a valid cursor")?;
    let mut offset = 0usize;
    for (elevator_index, elevator) in elevators.iter().enumerate() {
        let next_offset = offset + elevator.floors.len();
        if selected < next_offset {
            return Ok((elevator_index, selected - offset));
        }
        offset = next_offset;
    }
    anyhow::bail!("selected elevator option {selected} is outside visible elevators")
}

fn move_visible_elevator_cursor(runtime_shell: &mut BevyRuntimeShell, delta: isize) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let surface_id = runtime_shell
        .elevator_cursor
        .as_ref()
        .map(|cursor| cursor.surface_id.clone())
        .context("elevator prompt requires a cursor surface")?;
    let option_count = visible_elevator_option_count(&snapshot, runtime_shell);
    if option_count == 0 {
        anyhow::bail!(
            "retained elevator surface {surface_id} has no matching compiled floors on map {}",
            snapshot.overworld.map_name
        );
    }
    move_visible_cursor_slot(
        &mut runtime_shell.elevator_cursor,
        surface_id,
        option_count,
        delta,
        &mut runtime_shell.last_audio_events,
    )
}

fn move_visible_yes_no_cursor(runtime_shell: &mut BevyRuntimeShell, delta: isize) -> Result<()> {
    move_visible_cursor_slot(
        &mut runtime_shell.yes_no_cursor,
        "ui:yes-no".to_string(),
        2,
        delta,
        &mut runtime_shell.last_audio_events,
    )
}

fn move_visible_phone_prompt_cursor(
    runtime_shell: &mut BevyRuntimeShell,
    delta: isize,
) -> Result<()> {
    move_visible_cursor_slot(
        &mut runtime_shell.yes_no_cursor,
        "ui:phone-number".to_string(),
        2,
        delta,
        &mut runtime_shell.last_audio_events,
    )
}

fn move_visible_sell_cursor_up(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    move_visible_sell_cursor(runtime_shell, -1)
}

fn move_visible_sell_cursor_down(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    move_visible_sell_cursor(runtime_shell, 1)
}

fn move_visible_start_menu_cursor(
    runtime_shell: &mut BevyRuntimeShell,
    delta: isize,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let options = visible_start_menu_options(runtime_shell, &snapshot);
    move_visible_cursor_slot(
        &mut runtime_shell.start_menu_cursor,
        START_MENU_SURFACE_ID.to_string(),
        options.len(),
        delta,
        &mut runtime_shell.last_audio_events,
    )
}

fn move_visible_party_cursor_up(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    move_visible_party_cursor(runtime_shell, -1)
}

fn move_visible_party_cursor_down(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    move_visible_party_cursor(runtime_shell, 1)
}

fn move_visible_bag_cursor_up(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    move_visible_bag_cursor(runtime_shell, -1)
}

fn move_visible_bag_cursor_down(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    move_visible_bag_cursor(runtime_shell, 1)
}

fn move_visible_key_item_cursor_up(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    move_visible_key_item_cursor(runtime_shell, -1)
}

fn move_visible_key_item_cursor_down(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    move_visible_key_item_cursor(runtime_shell, 1)
}

fn move_visible_ball_cursor_up(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    move_visible_ball_cursor(runtime_shell, -1)
}

fn move_visible_ball_cursor_down(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    move_visible_ball_cursor(runtime_shell, 1)
}

fn move_visible_tmhm_cursor_up(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    move_visible_tmhm_cursor(runtime_shell, -1)
}

fn move_visible_tmhm_cursor_down(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    move_visible_tmhm_cursor(runtime_shell, 1)
}

fn move_visible_storage_cursor_up(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    move_visible_storage_cursor(runtime_shell, -1)
}

fn move_visible_storage_cursor_down(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    move_visible_storage_cursor(runtime_shell, 1)
}

fn move_visible_pc_item_cursor_up(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    move_visible_pc_item_cursor(runtime_shell, -1)
}

fn move_visible_pc_item_cursor_down(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    move_visible_pc_item_cursor(runtime_shell, 1)
}

fn move_visible_fly_cursor_up(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    move_visible_fly_cursor(runtime_shell, -1)
}

fn move_visible_fly_cursor_down(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    move_visible_fly_cursor(runtime_shell, 1)
}

fn move_visible_battle_switch_cursor(
    runtime_shell: &mut BevyRuntimeShell,
    delta: isize,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let Some(_battle) = snapshot.battle.as_ref() else {
        return handle_visible_no_active_battle(runtime_shell, "switch_cursor");
    };
    let option_count = battle_switch_option_count(&snapshot);
    if option_count == 0 {
        runtime_shell.battle_switch_cursor = None;
        runtime_shell
            .last_audio_events
            .push("active battle has no available party switches".to_string());
        set_shell_action_status(runtime_shell, "NO SWITCHES");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    move_visible_cursor_slot(
        &mut runtime_shell.battle_switch_cursor,
        "battle:switch".to_string(),
        option_count,
        delta,
        &mut runtime_shell.last_audio_events,
    )
}

fn move_visible_battle_action_cursor(
    runtime_shell: &mut BevyRuntimeShell,
    delta: isize,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let Some(battle) = snapshot.battle.as_ref() else {
        return handle_visible_no_active_battle(runtime_shell, "action_cursor");
    };
    let actions = visible_battle_action_ids(&snapshot, battle);
    if actions.is_empty() {
        runtime_shell.battle_action_cursor = None;
        record_visible_runtime_action(runtime_shell, "battle:actions:none")?;
        runtime_shell
            .last_audio_events
            .push("active battle has no available player action".to_string());
        set_shell_action_status(runtime_shell, "NO BATTLE ACTION");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    move_visible_cursor_slot(
        &mut runtime_shell.battle_action_cursor,
        "battle:actions".to_string(),
        actions.len(),
        delta,
        &mut runtime_shell.last_audio_events,
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BattleMenuAxis {
    Horizontal,
    Vertical,
}

fn move_visible_battle_action_cursor_axis(
    runtime_shell: &mut BevyRuntimeShell,
    axis: BattleMenuAxis,
    delta: isize,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let Some(battle) = snapshot.battle.as_ref() else {
        return handle_visible_no_active_battle(runtime_shell, "action_cursor");
    };
    let actions = visible_battle_action_ids(&snapshot, battle);
    if actions.len() != 4 {
        // Forced replacement is a one-entry boundary, not the 2x2 menu.
        return move_visible_battle_action_cursor(runtime_shell, 0);
    }
    let current = strict_readonly_cursor_index(
        &runtime_shell.battle_action_cursor,
        "battle:actions",
        actions.len(),
    )
    .context("battle action menu has no valid cursor")?;
    let (row, column) = (current / 2, current % 2);
    let next = match axis {
        BattleMenuAxis::Horizontal => {
            let next_column = if delta.is_negative() {
                column.saturating_sub(1)
            } else {
                (column + 1).min(1)
            };
            row * 2 + next_column
        }
        BattleMenuAxis::Vertical => {
            let next_row = if delta.is_negative() {
                row.saturating_sub(1)
            } else {
                (row + 1).min(1)
            };
            next_row * 2 + column
        }
    };
    if next == current {
        return Ok(());
    }
    runtime_shell.battle_action_cursor = Some(MenuCursor {
        surface_id: "battle:actions".to_string(),
        option_index: next,
    });
    runtime_shell.last_audio_events.push(format!(
        "battle action cursor {}->{} {:?}",
        current + 1,
        next + 1,
        axis
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn move_visible_party_move_cursor_up(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    move_visible_party_move_cursor(runtime_shell, -1)
}

fn move_visible_party_move_cursor_down(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    move_visible_party_move_cursor(runtime_shell, 1)
}

fn move_visible_party_cursor(runtime_shell: &mut BevyRuntimeShell, delta: isize) -> Result<()> {
    move_visible_party_slot_cursor(runtime_shell, delta)
}

fn move_visible_regular_party_menu_cursor(
    runtime_shell: &mut BevyRuntimeShell,
    delta: isize,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    if snapshot.party.slots.is_empty() {
        runtime_shell.party_cursor = 0;
        runtime_shell.party_action_cursor = None;
        runtime_shell.party_switch_cursor = None;
        runtime_shell
            .last_audio_events
            .push("party is empty".to_string());
        set_shell_action_status(runtime_shell, "NO POKEMON");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    let row_count = normal_visible_party_menu_row_count(&snapshot);
    anyhow::ensure!(
        runtime_shell.party_cursor < row_count,
        "party cursor {} is outside {row_count} Pokemon/CANCEL rows",
        runtime_shell.party_cursor
    );
    let current = runtime_shell.party_cursor;
    let next = wrapped_index(current, row_count, delta);
    runtime_shell.party_cursor = next;
    runtime_shell.party_action_cursor = None;
    runtime_shell.party_switch_cursor = None;
    runtime_shell.last_audio_events.push(format!(
        "party cursor {}->{}",
        party_menu_cursor_label(&snapshot, current),
        party_menu_cursor_label(&snapshot, next)
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn move_visible_party_slot_cursor(
    runtime_shell: &mut BevyRuntimeShell,
    delta: isize,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    if snapshot.party.slots.is_empty() {
        runtime_shell.party_cursor = 0;
        runtime_shell.party_action_cursor = None;
        runtime_shell.party_switch_cursor = None;
        runtime_shell
            .last_audio_events
            .push("party is empty".to_string());
        set_shell_action_status(runtime_shell, "NO POKEMON");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    anyhow::ensure!(
        runtime_shell.party_cursor < snapshot.party.slots.len(),
        "party cursor {} is outside {} Pokemon rows",
        runtime_shell.party_cursor,
        snapshot.party.slots.len()
    );
    let current = runtime_shell.party_cursor;
    let next = if delta.is_negative() {
        current
            .checked_sub(delta.unsigned_abs())
            .unwrap_or(snapshot.party.slots.len() - 1)
    } else {
        (current + delta as usize) % snapshot.party.slots.len()
    };
    runtime_shell.party_cursor = next;
    runtime_shell.party_action_cursor = None;
    runtime_shell.party_switch_cursor = None;
    runtime_shell
        .last_audio_events
        .push(format!("party cursor {}->{}", current + 1, next + 1));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn normal_visible_party_menu_row_count(snapshot: &RuntimeShellSnapshot) -> usize {
    snapshot.party.slots.len() + 1
}

fn party_menu_cursor_label(snapshot: &RuntimeShellSnapshot, cursor: usize) -> String {
    if cursor >= snapshot.party.slots.len() {
        "CANCEL".to_string()
    } else {
        (cursor + 1).to_string()
    }
}

fn initialize_visible_party_cursor(
    runtime_shell: &mut BevyRuntimeShell,
    snapshot: &RuntimeShellSnapshot,
) {
    if snapshot.party.slots.is_empty() {
        runtime_shell.party_cursor = 0;
        runtime_shell.party_action_cursor = None;
        runtime_shell.party_switch_cursor = None;
        return;
    }
    if runtime_shell.party_cursor >= snapshot.party.slots.len() {
        // InitPartyMenuWithCancel/NoCancel use wPartyMenuCursor only when it
        // is in 1..=wPartyCount; zero or an out-of-range value starts at row 1.
        runtime_shell.party_cursor = 0;
        runtime_shell.party_action_cursor = None;
        runtime_shell.party_switch_cursor = None;
    }
}

fn selected_party_index(runtime_shell: &mut BevyRuntimeShell) -> Result<usize> {
    let snapshot = runtime_shell.shell.snapshot()?;
    if snapshot.party.slots.is_empty() {
        anyhow::bail!("party is empty");
    }
    snapshot
        .party
        .slots
        .get(runtime_shell.party_cursor)
        .map(|slot| slot.index)
        .with_context(|| {
            format!(
                "party cursor {} is outside {} Pokemon rows",
                runtime_shell.party_cursor,
                snapshot.party.slots.len()
            )
        })
}

fn selected_party_move_slot(
    runtime_shell: &mut BevyRuntimeShell,
    party_index: usize,
) -> Result<usize> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let slot = snapshot
        .party
        .slots
        .iter()
        .find(|slot| slot.index == party_index)
        .with_context(|| format!("selected party index {party_index} is not in the party"))?;
    if slot.pokemon.moves.is_empty() {
        anyhow::bail!("selected party index {party_index} has no moves");
    }
    strict_readonly_cursor_index(
        &runtime_shell.party_move_cursor,
        &party_move_cursor_surface_id(party_index),
        slot.pokemon.moves.len(),
    )
    .context("selected party move cursor is invalid")
}

fn selected_pending_move_learn_replacement_slot(
    runtime_shell: &mut BevyRuntimeShell,
) -> Result<usize> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let pending = snapshot
        .pending_move_learn
        .as_ref()
        .context("no pending move learn")?;
    let slot = snapshot
        .party
        .slots
        .iter()
        .find(|slot| slot.index == pending.party_index)
        .with_context(|| {
            format!(
                "pending move learn party index {} is not in the party",
                pending.party_index
            )
        })?;
    if slot.pokemon.moves.is_empty() {
        anyhow::bail!(
            "pending move learn party index {} has no moves",
            pending.party_index
        );
    }
    strict_readonly_cursor_index(
        &runtime_shell.party_move_cursor,
        &party_move_cursor_surface_id(pending.party_index),
        slot.pokemon.moves.len() + 1,
    )
    .context("pending move-learn move-or-CANCEL cursor is invalid")
}

fn open_visible_move_learn_decision(
    runtime_shell: &mut BevyRuntimeShell,
    decision: VisibleTmHmDecision,
) -> Result<()> {
    if runtime_shell.shell.snapshot()?.pending_move_learn.is_none() {
        anyhow::bail!("cannot open move-learn decision without a pending move");
    }
    runtime_shell.move_learn_decision = Some(decision);
    visible_cursor_index(
        &mut runtime_shell.move_learn_decision_cursor,
        "move-learn:decision",
        2,
    );
    set_shell_action_status(
        runtime_shell,
        match decision {
            VisibleTmHmDecision::ForgetMove => "FORGET A MOVE?",
            VisibleTmHmDecision::StopLearning => "STOP LEARNING?",
        },
    );
    Ok(())
}

fn open_visible_pending_move_forget_menu(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let pending = snapshot
        .pending_move_learn
        .as_ref()
        .context("no pending move learn")?;
    let slot = snapshot
        .party
        .slots
        .iter()
        .find(|slot| slot.index == pending.party_index)
        .with_context(|| {
            format!(
                "pending move learn party index {} is not in the party",
                pending.party_index
            )
        })?;
    runtime_shell.party_move_cursor = None;
    visible_cursor_index(
        &mut runtime_shell.party_move_cursor,
        &party_move_cursor_surface_id(pending.party_index),
        slot.pokemon.moves.len() + 1,
    );
    runtime_shell.move_learn_forget_menu_open = true;
    set_shell_action_status(runtime_shell, "WHICH MOVE SHOULD BE FORGOTTEN?");
    Ok(())
}

fn resolve_visible_move_learn_decision(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let selected = strict_readonly_cursor_index(
        &runtime_shell.move_learn_decision_cursor,
        "move-learn:decision",
        2,
    )
    .context("move-learn decision requires a valid cursor")?;
    let decision = runtime_shell
        .move_learn_decision
        .context("move-learn decision kind is missing")?;
    runtime_shell.move_learn_decision_cursor = None;
    runtime_shell.move_learn_decision = None;
    match (decision, selected) {
        (VisibleTmHmDecision::ForgetMove, 0) => {
            open_visible_pending_move_forget_menu(runtime_shell)
        }
        (VisibleTmHmDecision::ForgetMove, _) => {
            open_visible_move_learn_decision(runtime_shell, VisibleTmHmDecision::StopLearning)
        }
        (VisibleTmHmDecision::StopLearning, 0) => {
            decline_visible_pending_move_learn(runtime_shell)
        }
        (VisibleTmHmDecision::StopLearning, _) => {
            open_visible_pending_move_forget_menu(runtime_shell)
        }
    }
}

fn confirm_visible_pending_move_learn(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    if runtime_shell.move_learn_decision_cursor.is_some() {
        return resolve_visible_move_learn_decision(runtime_shell);
    }
    if !runtime_shell.move_learn_forget_menu_open {
        return open_visible_move_learn_decision(
            runtime_shell,
            VisibleTmHmDecision::ForgetMove,
        );
    }
    let selected = selected_pending_move_learn_replacement_slot(runtime_shell)?;
    let snapshot = runtime_shell.shell.snapshot()?;
    let pending = snapshot
        .pending_move_learn
        .as_ref()
        .context("no pending move learn")?;
    let move_count = snapshot
        .party
        .slots
        .iter()
        .find(|slot| slot.index == pending.party_index)
        .map(|slot| slot.pokemon.moves.len())
        .with_context(|| {
            format!(
                "pending move learn party index {} is not in the party",
                pending.party_index
            )
        })?;
    if selected == move_count {
        runtime_shell.move_learn_forget_menu_open = false;
        runtime_shell.party_move_cursor = None;
        return open_visible_move_learn_decision(
            runtime_shell,
            VisibleTmHmDecision::StopLearning,
        );
    }
    replace_visible_pending_move_learn(runtime_shell)
}

fn cancel_visible_pending_move_learn(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    if runtime_shell.move_learn_decision_cursor.is_some() {
        runtime_shell.move_learn_decision_cursor = Some(MenuCursor {
            surface_id: "move-learn:decision".to_string(),
            option_index: 1,
        });
        return resolve_visible_move_learn_decision(runtime_shell);
    }
    if runtime_shell.move_learn_forget_menu_open {
        runtime_shell.move_learn_forget_menu_open = false;
        runtime_shell.party_move_cursor = None;
        return open_visible_move_learn_decision(
            runtime_shell,
            VisibleTmHmDecision::StopLearning,
        );
    }
    open_visible_move_learn_decision(runtime_shell, VisibleTmHmDecision::ForgetMove)
}

fn replace_visible_pending_move_learn(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let move_slot = selected_pending_move_learn_replacement_slot(runtime_shell)?;
    let snapshot = runtime_shell.shell.snapshot()?;
    let pending = snapshot
        .pending_move_learn
        .as_ref()
        .context("no pending move learn")?;
    let move_count = snapshot
        .party
        .slots
        .iter()
        .find(|slot| slot.index == pending.party_index)
        .map(|slot| slot.pokemon.moves.len())
        .with_context(|| {
            format!(
                "pending move learn party index {} is not in the party",
                pending.party_index
            )
        })?;
    if move_slot == move_count {
        return decline_visible_pending_move_learn(runtime_shell);
    }
    record_visible_runtime_action(runtime_shell, format!("move_learn:replace:{move_slot}"))?;
    let outcome = match runtime_shell.shell.replace_pending_move_learn(move_slot) {
        Ok(outcome) => outcome,
        Err(error)
            if matches!(
                error.downcast_ref::<BattleRewardError>(),
                Some(BattleRewardError::CannotForgetHmMove { .. })
            ) =>
        {
            let move_id = match error.downcast_ref::<BattleRewardError>() {
                Some(BattleRewardError::CannotForgetHmMove { move_id }) => move_id.as_str(),
                _ => "HM",
            };
            runtime_shell
                .last_audio_events
                .push(format!("move learn cannot forget HM {move_id}"));
            runtime_shell.battle_messages.extend(visible_move_learning_text_pages(
                runtime_shell,
                "_MoveCantForgetHMText",
                "",
                "",
                move_id,
            )?);
            set_shell_action_status(runtime_shell, "HM MOVES CAN'T BE FORGOTTEN NOW");
            trim_event_log(&mut runtime_shell.last_audio_events);
            return Ok(());
        }
        Err(error) => return Err(error),
    };
    let resolution = &outcome.resolution;
    let recipient_name = visible_party_pokemon_name(runtime_shell, resolution.party_index)?;
    let snapshot = runtime_shell.shell.snapshot()?;
    let replaced_move_name = resolution
        .replaced_move
        .as_deref()
        .map(|move_id| battle_move_display_name(&snapshot, move_id))
        .unwrap_or_else(|| "a move".to_string());
    runtime_shell.party_move_cursor = None;
    runtime_shell.move_learn_decision_cursor = None;
    runtime_shell.move_learn_decision = None;
    runtime_shell.move_learn_forget_menu_open = false;
    runtime_shell.last_audio_events.push(format!(
        "pending move learn replaced party_index={} slot={:?} learned={} replaced={:?}",
        resolution.party_index,
        resolution.replaced_slot,
        resolution.learned_move,
        resolution.replaced_move
    ));
    install_visible_move_learn_result_sequence(
        runtime_shell,
        &recipient_name,
        Some(&replaced_move_name),
        &resolution.learned_move,
    )?;
    push_visible_deferred_evolution_events(
        runtime_shell,
        outcome.deferred_evolution.as_ref(),
        resolution.party_index,
    )?;
    refresh_visible_battle_scene_after_party_progression(runtime_shell)?;
    set_shell_action_status(
        runtime_shell,
        visible_pending_move_learn_resolution_status("LEARNED", &outcome),
    );
    trim_event_log(&mut runtime_shell.last_audio_events);
    // The source sequence owns two automatic text pauses and remains the
    // active presentation until its final learned-move page is acknowledged.
    // `close_visible_special_boundary` resumes any suspended script then.
    Ok(())
}

fn decline_visible_pending_move_learn(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    record_visible_runtime_action(runtime_shell, "move_learn:decline")?;
    let outcome = runtime_shell.shell.decline_pending_move_learn()?;
    let resolution = &outcome.resolution;
    let recipient_name = visible_party_pokemon_name(runtime_shell, resolution.party_index)?;
    runtime_shell.party_move_cursor = None;
    runtime_shell.move_learn_decision_cursor = None;
    runtime_shell.move_learn_decision = None;
    runtime_shell.move_learn_forget_menu_open = false;
    runtime_shell.last_audio_events.push(format!(
        "pending move learn declined party_index={} learned={}",
        resolution.party_index, resolution.learned_move
    ));
    runtime_shell.battle_messages.extend(visible_move_learning_text_pages(
        runtime_shell,
        "_DidNotLearnMoveText",
        &recipient_name,
        &recipient_name,
        &resolution.learned_move,
    )?);
    push_visible_deferred_evolution_events(
        runtime_shell,
        outcome.deferred_evolution.as_ref(),
        resolution.party_index,
    )?;
    refresh_visible_battle_scene_after_party_progression(runtime_shell)?;
    set_shell_action_status(
        runtime_shell,
        visible_pending_move_learn_resolution_status("DID NOT LEARN", &outcome),
    );
    trim_event_log(&mut runtime_shell.last_audio_events);
    continue_visible_script_after_prompt(runtime_shell)
}

fn visible_pending_move_learn_resolution_status(
    prefix: &str,
    outcome: &crate::RuntimePendingMoveLearnResolution,
) -> String {
    let mut parts = vec![format!(
        "{prefix} {}",
        compact_scene_label(&outcome.resolution.learned_move, 32)
    )];
    if let Some(evolution) = outcome.deferred_evolution.as_ref() {
        if let Some(target_species) = evolution.target_species.as_ref() {
            parts.push(format!("EVOLVED {target_species}"));
        }
        if !evolution.pending_move_learns.is_empty() {
            let pending = evolution
                .pending_move_learns
                .iter()
                .map(|learned| learned.name.as_str())
                .collect::<Vec<_>>()
                .join(",");
            parts.push(format!("WANTS {pending}"));
        }
    }
    compact_scene_label(&parts.join(" / "), 76)
}

fn push_visible_deferred_evolution_events(
    runtime_shell: &mut BevyRuntimeShell,
    evolution: Option<&crate::core::systems::evolution::EvolutionReport>,
    party_index: usize,
) -> Result<()> {
    let Some(evolution) = evolution else {
        return Ok(());
    };
    let recipient_name = visible_party_pokemon_name(runtime_shell, party_index)?;
    if let Some(target_species) = evolution.target_species.as_ref() {
        let evolving_message = format!("What? {} is evolving!", recipient_name);
        let evolved_message = format!(
            "Congratulations! {} evolved into {}!",
            recipient_name,
            crate::core::models::pokemon_species_display_name(target_species)
        );
        runtime_shell.battle_messages.push_back(evolving_message.clone());
        runtime_shell.battle_messages.push_back(evolved_message.clone());
        let mut pending_move_messages = Vec::new();
        for learned in &evolution.pending_move_learns {
            pending_move_messages.extend(visible_pending_move_learn_intro_pages(
                runtime_shell,
                &recipient_name,
                &learned.name,
            )?);
        }
        if evolution.cancel_snapshot.is_some() {
            runtime_shell.battle_evolution_cancellations.push_back(
                VisibleEvolutionCancellation {
                    party_index,
                    trigger_message: evolving_message.clone(),
                    evolved_message: evolved_message.clone(),
                    pending_move_messages,
                    report: evolution.clone(),
                },
            );
        }
        runtime_shell
            .battle_evolution_cries
            .push_back((target_species.clone(), evolving_message.clone()));
        runtime_shell
            .battle_sounds_after_messages
            .push_back(("SFX_CAUGHT_MON".to_string(), evolving_message.clone()));
        runtime_shell
            .last_audio_events
            .push(format!("deferred evolution evolved {target_species}"));
    }
    for learned in &evolution.pending_move_learns {
        runtime_shell.battle_messages.extend(
            visible_pending_move_learn_intro_pages(
                runtime_shell,
                &recipient_name,
                &learned.name,
            )?,
        );
        runtime_shell.last_audio_events.push(format!(
            "deferred evolution pending move learn {}",
            learned.name
        ));
    }
    for event in &evolution.events {
        let label = match event {
            crate::core::systems::evolution::EvolutionEvent::Text(text) => {
                format!("deferred evolution text {text}")
            }
            crate::core::systems::evolution::EvolutionEvent::ItemConsumed(item_id) => {
                format!("deferred evolution consumed {item_id}")
            }
            crate::core::systems::evolution::EvolutionEvent::MoveLearned(move_id) => {
                format!("deferred evolution learned move {move_id}")
            }
        };
        runtime_shell.last_audio_events.push(label);
    }
    Ok(())
}

fn refresh_visible_battle_scene_after_party_progression(
    runtime_shell: &mut BevyRuntimeShell,
) -> Result<()> {
    let latest = runtime_shell.shell.snapshot()?;
    if latest.battle.is_some() {
        runtime_shell.battle_message_scene = Some(Box::new(latest));
        mark_runtime_snapshot_dirty(runtime_shell);
    }
    Ok(())
}

fn visible_party_pokemon_name(
    runtime_shell: &BevyRuntimeShell,
    party_index: usize,
) -> Result<String> {
    runtime_shell
        .shell
        .snapshot()?
        .party
        .slots
        .into_iter()
        .find(|slot| slot.index == party_index)
        .map(|slot| slot.pokemon.nickname)
        .with_context(|| format!("party slot {party_index} is missing from visible progression"))
}

fn selected_pokedex_species_id(runtime_shell: &mut BevyRuntimeShell) -> Result<String> {
    let snapshot = runtime_shell.shell.snapshot()?;
    if runtime_shell.pokedex_menu_open {
        return selected_pokedex_catalog_species(&snapshot, runtime_shell.pokedex_cursor)
            .map(|species| species.species_id.clone());
    }
    if !snapshot.party.slots.is_empty() {
        let party_index = selected_party_index(runtime_shell)?;
        return snapshot
            .party
            .slots
            .iter()
            .find(|slot| slot.index == party_index)
            .map(|slot| slot.pokemon.species.id.clone())
            .with_context(|| format!("selected party index {party_index} is not in the party"));
    }
    if snapshot.pokemon.is_empty() {
        anyhow::bail!("compiled pack has no Pokemon species");
    }
    let selected_index = runtime_shell.script_command_cursor % snapshot.pokemon.len();
    Ok(snapshot.pokemon[selected_index].species_id.clone())
}

fn move_visible_bag_cursor(runtime_shell: &mut BevyRuntimeShell, delta: isize) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    runtime_shell.key_item_cursor = None;
    runtime_shell.ball_cursor = None;
    runtime_shell.tmhm_cursor = None;
    runtime_shell.custom_item_cursor = None;
    runtime_shell.pc_item_cursor = None;
    runtime_shell.field_pack_pocket = Some(FieldPackPocket::Items);
    move_visible_cursor_slot(
        &mut runtime_shell.bag_cursor,
        "bag:items".to_string(),
        field_pack_selectable_count(carried_item_count(&snapshot.bag.items)),
        delta,
        &mut runtime_shell.last_audio_events,
    )?;
    runtime_shell.field_pack_cursor_positions[0] = runtime_shell
        .bag_cursor
        .as_ref()
        .map_or(0, |cursor| cursor.option_index);
    Ok(())
}

fn move_visible_key_item_cursor(runtime_shell: &mut BevyRuntimeShell, delta: isize) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    runtime_shell.bag_cursor = None;
    runtime_shell.ball_cursor = None;
    runtime_shell.tmhm_cursor = None;
    runtime_shell.custom_item_cursor = None;
    runtime_shell.pc_item_cursor = None;
    if snapshot.battle.is_none() {
        runtime_shell.field_pack_pocket = Some(FieldPackPocket::KeyItems);
    } else {
        runtime_shell.field_pack_pocket = None;
        runtime_shell.field_pack_action_cursor = None;
        runtime_shell.field_pack_target_mode = None;
        runtime_shell.battle_pack_target_mode = None;
    }
    move_visible_cursor_slot(
        &mut runtime_shell.key_item_cursor,
        "bag:key-items".to_string(),
        field_pack_selectable_count(carried_item_count(&snapshot.bag.key_items)),
        delta,
        &mut runtime_shell.last_audio_events,
    )?;
    runtime_shell.field_pack_cursor_positions[2] = runtime_shell
        .key_item_cursor
        .as_ref()
        .map_or(0, |cursor| cursor.option_index);
    Ok(())
}

fn move_visible_battle_bag_cursor(
    runtime_shell: &mut BevyRuntimeShell,
    delta: isize,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    runtime_shell.key_item_cursor = None;
    runtime_shell.ball_cursor = None;
    runtime_shell.tmhm_cursor = None;
    runtime_shell.custom_item_cursor = None;
    runtime_shell.pc_item_cursor = None;
    runtime_shell.field_pack_pocket = None;
    runtime_shell.field_pack_action_cursor = None;
    runtime_shell.field_pack_target_mode = None;
    runtime_shell.battle_pack_target_mode = None;
    let item_ids = carried_battle_non_ball_item_ids(&snapshot);
    move_visible_cursor_slot(
        &mut runtime_shell.bag_cursor,
        "battle:bag-items".to_string(),
        field_pack_selectable_count(item_ids.len()),
        delta,
        &mut runtime_shell.last_audio_events,
    )?;
    runtime_shell.field_pack_cursor_positions[0] = runtime_shell
        .bag_cursor
        .as_ref()
        .map_or(0, |cursor| cursor.option_index);
    Ok(())
}

fn move_visible_ball_cursor(runtime_shell: &mut BevyRuntimeShell, delta: isize) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    runtime_shell.bag_cursor = None;
    runtime_shell.key_item_cursor = None;
    runtime_shell.tmhm_cursor = None;
    runtime_shell.custom_item_cursor = None;
    runtime_shell.pc_item_cursor = None;
    if snapshot.battle.is_none() {
        runtime_shell.field_pack_pocket = Some(FieldPackPocket::Balls);
    } else {
        runtime_shell.field_pack_pocket = None;
        runtime_shell.field_pack_action_cursor = None;
        runtime_shell.field_pack_target_mode = None;
        runtime_shell.battle_pack_target_mode = None;
    }
    let option_count = if snapshot.battle.is_none() {
        field_pack_selectable_count(carried_item_count(&snapshot.bag.balls))
    } else {
        field_pack_selectable_count(carried_ball_item_ids(&snapshot).len())
    };
    move_visible_cursor_slot(
        &mut runtime_shell.ball_cursor,
        "bag:balls".to_string(),
        option_count,
        delta,
        &mut runtime_shell.last_audio_events,
    )?;
    runtime_shell.field_pack_cursor_positions[1] = runtime_shell
        .ball_cursor
        .as_ref()
        .map_or(0, |cursor| cursor.option_index);
    Ok(())
}

fn move_visible_tmhm_cursor(runtime_shell: &mut BevyRuntimeShell, delta: isize) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    runtime_shell.bag_cursor = None;
    runtime_shell.key_item_cursor = None;
    runtime_shell.ball_cursor = None;
    runtime_shell.custom_item_cursor = None;
    runtime_shell.pc_item_cursor = None;
    if snapshot.battle.is_none() {
        runtime_shell.field_pack_pocket = Some(FieldPackPocket::TmHm);
    } else {
        runtime_shell.field_pack_pocket = None;
        runtime_shell.field_pack_action_cursor = None;
        runtime_shell.field_pack_target_mode = None;
        runtime_shell.battle_pack_target_mode = None;
    }
    move_visible_cursor_slot(
        &mut runtime_shell.tmhm_cursor,
        "bag:tmhm".to_string(),
        field_pack_selectable_count(snapshot.bag.tm_hm.len()),
        delta,
        &mut runtime_shell.last_audio_events,
    )?;
    runtime_shell.field_pack_cursor_positions[3] = runtime_shell
        .tmhm_cursor
        .as_ref()
        .map_or(0, |cursor| cursor.option_index);
    Ok(())
}

fn move_visible_custom_item_cursor(
    runtime_shell: &mut BevyRuntimeShell,
    pocket_id: &str,
    delta: isize,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let items = snapshot
        .bag
        .custom_pockets
        .get(pocket_id)
        .with_context(|| format!("bag custom pocket {pocket_id} is not present"))?;
    runtime_shell.bag_cursor = None;
    runtime_shell.key_item_cursor = None;
    runtime_shell.ball_cursor = None;
    runtime_shell.tmhm_cursor = None;
    runtime_shell.pc_item_cursor = None;
    runtime_shell.field_pack_pocket = Some(FieldPackPocket::Custom(pocket_id.to_string()));
    move_visible_cursor_slot(
        &mut runtime_shell.custom_item_cursor,
        custom_pack_surface_id(pocket_id),
        field_pack_selectable_count(carried_item_count(items)),
        delta,
        &mut runtime_shell.last_audio_events,
    )
}

fn move_visible_storage_cursor(runtime_shell: &mut BevyRuntimeShell, delta: isize) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    if runtime_shell.bill_pc_move_open && runtime_shell.bill_pc_move_party_open {
        let count = snapshot.party.slots.len();
        return move_visible_cursor_slot(
            &mut runtime_shell.storage_cursor,
            pc_move_party_surface_id().to_string(),
            if runtime_shell.bill_pc_move_source.is_some() {
                count + 1
            } else {
                count.max(1)
            },
            delta,
            &mut runtime_shell.last_audio_events,
        );
    }
    let current_box = current_storage_box(&snapshot)?;
    if current_box.slots.is_empty() && !runtime_shell.bill_pc_move_open {
        runtime_shell.storage_cursor = Some(MenuCursor {
            surface_id: storage_cursor_surface_id(snapshot.storage.current_pc_box),
            option_index: 0,
        });
        runtime_shell.last_audio_events.push(format!(
            "pc box {} has no stored Pokemon",
            snapshot.storage.current_pc_box
        ));
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    move_visible_cursor_slot(
        &mut runtime_shell.storage_cursor,
        storage_cursor_surface_id(snapshot.storage.current_pc_box),
        if runtime_shell.bill_pc_move_open {
            if runtime_shell.bill_pc_move_source.is_some() {
                current_box.slots.len() + 1
            } else {
                current_box.slots.len().max(1)
            }
        } else {
            current_box.slots.len()
        },
        delta,
        &mut runtime_shell.last_audio_events,
    )
}

fn move_visible_pc_item_cursor(runtime_shell: &mut BevyRuntimeShell, delta: isize) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    runtime_shell.bag_cursor = None;
    runtime_shell.key_item_cursor = None;
    runtime_shell.ball_cursor = None;
    runtime_shell.tmhm_cursor = None;
    move_visible_cursor_slot(
        &mut runtime_shell.pc_item_cursor,
        "pc:items".to_string(),
        snapshot
            .bag
            .pc_items
            .iter()
            .filter(|item| item.quantity > 0)
            .count(),
        delta,
        &mut runtime_shell.last_audio_events,
    )
}

fn move_visible_fly_cursor(runtime_shell: &mut BevyRuntimeShell, delta: isize) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    move_visible_cursor_slot(
        &mut runtime_shell.fly_cursor,
        "fly:destinations".to_string(),
        active_fly_destinations(&snapshot, &runtime_shell.shell)?.len(),
        delta,
        &mut runtime_shell.last_audio_events,
    )
}

fn move_visible_battle_move_cursor(
    runtime_shell: &mut BevyRuntimeShell,
    delta: isize,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let Some(ref battle) = snapshot.battle else {
        return handle_visible_no_active_battle(runtime_shell, "move_cursor");
    };
    if battle.commands.player_move_slots.is_empty() {
        runtime_shell.battle_move_cursor = None;
        runtime_shell.battle_move_swap_origin = None;
        runtime_shell
            .last_audio_events
            .push("active battle has no available player moves".to_string());
        set_shell_action_status(runtime_shell, "NO MOVES");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    let move_menu_count = battle_move_menu_option_count(&snapshot, &battle)?;
    move_visible_cursor_slot(
        &mut runtime_shell.battle_move_cursor,
        "battle:moves".to_string(),
        move_menu_count,
        delta,
        &mut runtime_shell.last_audio_events,
    )
}

fn select_visible_battle_move_swap(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let battle = snapshot
        .battle
        .as_ref()
        .context("battle move swap requires an active battle")?;
    let active_index = battle
        .active_player_party_index
        .context("battle move swap requires an active player party index")?;
    snapshot
        .party
        .slots
        .iter()
        .find(|slot| slot.index == active_index)
        .with_context(|| format!("active battle party index {active_index} is missing"))?;
    let selected = strict_readonly_cursor_index(
        &runtime_shell.battle_move_cursor,
        "battle:moves",
        battle.player_moves.len() + 1,
    )
    .context("battle move swap requires a valid FIGHT cursor")?;
    if selected >= battle.player_moves.len() {
        runtime_shell.battle_move_swap_origin = None;
        record_visible_runtime_action(runtime_shell, "battle:move_swap:cancel_row")?;
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    let Some(origin) = runtime_shell.battle_move_swap_origin else {
        runtime_shell.battle_move_swap_origin = Some(selected);
        record_visible_runtime_action(runtime_shell, format!("battle:move_swap:start:{selected}"))?;
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    };
    runtime_shell.battle_move_swap_origin = None;
    if origin == selected {
        record_visible_runtime_action(runtime_shell, format!("battle:move_swap:clear:{selected}"))?;
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    let swapped = runtime_shell
        .shell
        .swap_party_pokemon_moves(active_index, origin, selected)?;
    record_visible_runtime_action(
        runtime_shell,
        format!("battle:move_swap:{active_index}:{origin}:{selected}"),
    )?;
    runtime_shell.last_audio_events.push(format!(
        "swapped battle moves {} and {} -> {}, {} checksum={:?}",
        origin,
        selected,
        swapped.first_move_after,
        swapped.second_move_after,
        swapped.state_checksum
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn move_visible_party_move_cursor(
    runtime_shell: &mut BevyRuntimeShell,
    delta: isize,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let party_index = selected_party_index(runtime_shell)?;
    let slot = snapshot
        .party
        .slots
        .iter()
        .find(|slot| slot.index == party_index)
        .with_context(|| format!("selected party index {party_index} is not in the party"))?;
    if slot.pokemon.moves.is_empty() {
        runtime_shell.party_move_cursor = None;
        runtime_shell
            .last_audio_events
            .push(format!("party index {party_index} has no moves"));
        set_shell_action_status(runtime_shell, "NO MOVES");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    move_visible_cursor_slot(
        &mut runtime_shell.party_move_cursor,
        party_move_cursor_surface_id(party_index),
        slot.pokemon.moves.len(),
        delta,
        &mut runtime_shell.last_audio_events,
    )
}

fn move_visible_pending_move_learn_cursor(
    runtime_shell: &mut BevyRuntimeShell,
    delta: isize,
) -> Result<()> {
    if runtime_shell.move_learn_decision_cursor.is_some() {
        return move_visible_cursor_slot(
            &mut runtime_shell.move_learn_decision_cursor,
            "move-learn:decision".to_string(),
            2,
            delta,
            &mut runtime_shell.last_audio_events,
        );
    }
    if !runtime_shell.move_learn_forget_menu_open {
        open_visible_move_learn_decision(runtime_shell, VisibleTmHmDecision::ForgetMove)?;
        return move_visible_cursor_slot(
            &mut runtime_shell.move_learn_decision_cursor,
            "move-learn:decision".to_string(),
            2,
            delta,
            &mut runtime_shell.last_audio_events,
        );
    }
    let snapshot = runtime_shell.shell.snapshot()?;
    let pending = snapshot
        .pending_move_learn
        .as_ref()
        .context("no pending move learn")?;
    let slot = snapshot
        .party
        .slots
        .iter()
        .find(|slot| slot.index == pending.party_index)
        .with_context(|| {
            format!(
                "pending move learn party index {} is not in the party",
                pending.party_index
            )
        })?;
    if slot.pokemon.moves.is_empty() {
        runtime_shell.party_move_cursor = None;
        anyhow::bail!(
            "pending move learn party index {} has no replacement moves",
            pending.party_index
        );
    }
    move_visible_cursor_slot(
        &mut runtime_shell.party_move_cursor,
        party_move_cursor_surface_id(pending.party_index),
        slot.pokemon.moves.len() + 1,
        delta,
        &mut runtime_shell.last_audio_events,
    )
}

fn selected_current_box_slot_index(runtime_shell: &mut BevyRuntimeShell) -> Result<usize> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let current_box = current_storage_box(&snapshot)?;
    if current_box.slots.is_empty() && !runtime_shell.bill_pc_move_open {
        anyhow::bail!(
            "current PC box {} has no Pokemon",
            snapshot.storage.current_pc_box
        );
    }
    let surface_id = storage_cursor_surface_id(snapshot.storage.current_pc_box);
    let option_count = if runtime_shell.bill_pc_move_open {
        if runtime_shell.bill_pc_move_source.is_some() {
            current_box.slots.len() + 1
        } else {
            current_box.slots.len().max(1)
        }
    } else {
        current_box.slots.len()
    };
    let slot_offset = strict_readonly_cursor_index(
        &runtime_shell.storage_cursor,
        &surface_id,
        option_count,
    )
    .with_context(|| format!("PC storage surface {surface_id} is active without a valid cursor"))?;
    if runtime_shell.bill_pc_move_open {
        Ok(slot_offset)
    } else {
        Ok(current_box.slots[slot_offset].index)
    }
}

fn selected_pc_move_slot_index(runtime_shell: &mut BevyRuntimeShell) -> Result<usize> {
    if !runtime_shell.bill_pc_move_party_open {
        return selected_current_box_slot_index(runtime_shell);
    }
    let snapshot = runtime_shell.shell.snapshot()?;
    let count = snapshot.party.slots.len();
    let option_count = if runtime_shell.bill_pc_move_source.is_some() {
        count + 1
    } else {
        count.max(1)
    };
    strict_readonly_cursor_index(
        &runtime_shell.storage_cursor,
        pc_move_party_surface_id(),
        option_count,
    )
    .context("PC MOVE party surface pc:move-party is active without a valid cursor")
}

fn current_storage_box(snapshot: &RuntimeShellSnapshot) -> Result<&crate::RuntimePcBoxSnapshot> {
    snapshot
        .storage
        .boxes
        .iter()
        .find(|pc_box| pc_box.index == snapshot.storage.current_pc_box)
        .with_context(|| {
            format!(
                "current PC box {} is missing from storage snapshot",
                snapshot.storage.current_pc_box
            )
        })
}

fn storage_cursor_surface_id(box_index: usize) -> String {
    format!("pc:box:{box_index}")
}

fn pc_move_party_surface_id() -> &'static str {
    "pc:move:party"
}

fn party_move_cursor_surface_id(party_index: usize) -> String {
    format!("party:{party_index}:moves")
}

fn move_visible_sell_cursor(runtime_shell: &mut BevyRuntimeShell, delta: isize) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    if snapshot.pending_shop.is_none() {
        return Ok(());
    }
    let sellable = sellable_carried_item_ids(&snapshot);
    if sellable.is_empty() {
        runtime_shell.sell_cursor = None;
        runtime_shell
            .last_audio_events
            .push("bag has no sellable carried item".to_string());
        set_shell_action_status(runtime_shell, "NOTHING TO SELL");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    move_visible_mart_cursor_slot(
        &mut runtime_shell.sell_cursor,
        "sell:bag".to_string(),
        sellable.len(),
        delta,
        &mut runtime_shell.last_audio_events,
    )
}

fn move_visible_menu_cursor(runtime_shell: &mut BevyRuntimeShell, delta: isize) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    if let Some(shop) = &snapshot.pending_shop {
        return move_visible_cursor_for_surface(
            runtime_shell,
            shop_cursor_surface_id(shop),
            shop.inventory.len(),
            delta,
        );
    }
    if snapshot.ui.menu.is_none() {
        return Ok(());
    }
    let menu_target = active_menu_target(&snapshot, &runtime_shell.menu_cursor)?;
    if menu_target.two_dimensional {
        return move_visible_2d_menu_cursor(runtime_shell, &menu_target, delta, false);
    }
    move_visible_cursor_for_surface(
        runtime_shell,
        menu_target.surface_id,
        menu_target.option_count,
        delta,
    )
}

fn move_visible_menu_cursor_horizontal(
    runtime_shell: &mut BevyRuntimeShell,
    delta: isize,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let menu_target = active_menu_target(&snapshot, &runtime_shell.menu_cursor)?;
    if !menu_target.two_dimensional {
        return Ok(());
    }
    move_visible_2d_menu_cursor(runtime_shell, &menu_target, delta, true)
}

fn move_visible_2d_menu_cursor(
    runtime_shell: &mut BevyRuntimeShell,
    target: &ActiveMenuTarget,
    delta: isize,
    horizontal: bool,
) -> Result<()> {
    let rows = target.rows.context("2D menu is missing its row count")?;
    let columns = target.columns.context("2D menu is missing its column count")?;
    let current = strict_readonly_cursor_index(
        &runtime_shell.menu_cursor,
        &target.surface_id,
        target.option_count,
    )
    .with_context(|| format!("2D menu {} has no valid cursor", target.surface_id))?;
    let row = current / columns;
    let column = current % columns;
    let (next_row, next_column) = if horizontal {
        (row, wrapped_index(column, columns, delta))
    } else {
        (wrapped_index(row, rows, delta), column)
    };
    let next = next_row * columns + next_column;
    runtime_shell.menu_cursor = Some(MenuCursor {
        surface_id: target.surface_id.clone(),
        option_index: next,
    });
    runtime_shell
        .last_audio_events
        .push(format!("2D menu cursor {}->{}", current + 1, next + 1));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn move_visible_shop_buy_cursor(runtime_shell: &mut BevyRuntimeShell, delta: isize) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let Some(shop) = snapshot.pending_shop else {
        return handle_visible_no_active_shop(runtime_shell, "buy_cursor");
    };
    move_visible_mart_cursor_slot(
        &mut runtime_shell.menu_cursor,
        shop_cursor_surface_id(&shop),
        shop.inventory.len(),
        delta,
        &mut runtime_shell.last_audio_events,
    )
}

/// Mart lists stop at their first and last entries. Unlike ordinary vertical
/// menus, the TypeScript/ASM mart flow does not wrap the cursor on Up/Down.
fn move_visible_mart_cursor_slot(
    cursor_slot: &mut Option<MenuCursor>,
    surface_id: String,
    option_count: usize,
    delta: isize,
    event_log: &mut Vec<String>,
) -> Result<()> {
    if option_count == 0 {
        anyhow::bail!("{surface_id} has no selectable options");
    }
    let current = strict_readonly_cursor_index(cursor_slot, &surface_id, option_count)
        .with_context(|| format!("menu {surface_id} has no valid cursor"))?;
    let next = if delta.is_negative() {
        current.saturating_sub(delta.unsigned_abs())
    } else {
        current.saturating_add(delta as usize).min(option_count - 1)
    };
    *cursor_slot = Some(MenuCursor {
        surface_id,
        option_index: next,
    });
    if next != current {
        event_log.push(format!("cursor {}->{}", current + 1, next + 1));
        trim_event_log(event_log);
    }
    Ok(())
}

fn move_visible_cursor_for_surface(
    runtime_shell: &mut BevyRuntimeShell,
    surface_id: String,
    option_count: usize,
    delta: isize,
) -> Result<()> {
    move_visible_cursor_slot(
        &mut runtime_shell.menu_cursor,
        surface_id,
        option_count,
        delta,
        &mut runtime_shell.last_audio_events,
    )
}

fn move_visible_cursor_slot(
    cursor_slot: &mut Option<MenuCursor>,
    surface_id: String,
    option_count: usize,
    delta: isize,
    event_log: &mut Vec<String>,
) -> Result<()> {
    if option_count == 0 {
        anyhow::bail!("{surface_id} has no selectable options");
    }
    let current = strict_readonly_cursor_index(cursor_slot, &surface_id, option_count)
        .with_context(|| format!("menu {surface_id} has no valid cursor"))?;
    let next = if delta.is_negative() {
        current
            .checked_sub(delta.unsigned_abs())
            .unwrap_or(option_count - 1)
    } else {
        (current + delta as usize) % option_count
    };
    *cursor_slot = Some(MenuCursor {
        surface_id,
        option_index: next,
    });
    event_log.push(format!("cursor {}->{}", current + 1, next + 1));
    trim_event_log(event_log);
    Ok(())
}

fn select_visible_menu_cursor_option(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    if snapshot.ui.menu.is_none() {
        return handle_visible_no_active_menu(runtime_shell, "cursor_select");
    }
    let menu_target = active_menu_target(&snapshot, &runtime_shell.menu_cursor)?;
    let option_index = strict_readonly_cursor_index(
        &runtime_shell.menu_cursor,
        &menu_target.surface_id,
        menu_target.option_count,
    )
    .context("runtime menu surface is active without a valid cursor")?;
    select_visible_menu_option(runtime_shell, option_index)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ActiveMenuTarget {
    surface_id: String,
    option_count: usize,
    two_dimensional: bool,
    rows: Option<usize>,
    columns: Option<usize>,
}

fn active_menu_target(
    snapshot: &RuntimeShellSnapshot,
    cursor: &Option<MenuCursor>,
) -> Result<ActiveMenuTarget> {
    let Some(menu) = &snapshot.ui.menu else {
        anyhow::bail!("no active menu");
    };
    active_menu_target_from_live_cursor(menu, cursor)
}

fn active_menu_target_from_live_cursor(
    menu: &crate::RuntimeMenuSnapshot,
    cursor: &Option<MenuCursor>,
) -> Result<ActiveMenuTarget> {
    let cursor = cursor
        .as_ref()
        .with_context(|| format!("menu {} is active without a live cursor", menu.menu_id))?;
    let vertical = menu
        .layout
        .vertical_menus
        .iter()
        .find(|vertical| vertical_menu_surface_id(menu, vertical) == cursor.surface_id)
        .with_context(|| {
            format!(
                "menu {} has no vertical surface {}",
                menu.menu_id, cursor.surface_id
            )
        })?;
    if vertical.options.is_empty() {
        anyhow::bail!("menu {} vertical menu has no options", menu.menu_id);
    }
    Ok(ActiveMenuTarget {
        surface_id: cursor.surface_id.clone(),
        option_count: vertical.options.len(),
        two_dimensional: vertical.two_dimensional,
        rows: vertical.rows,
        columns: vertical.columns,
    })
}

fn first_selectable_vertical_menu<'a>(
    menu: &'a crate::RuntimeMenuSnapshot,
) -> Result<&'a crate::RuntimeVerticalMenuSnapshot> {
    menu.layout
        .vertical_menus
        .iter()
        .find(|vertical| !vertical.options.is_empty())
        .with_context(|| format!("menu {} has no selectable options", menu.menu_id))
}

fn selected_vertical_menu<'a>(
    menu: &'a crate::RuntimeMenuSnapshot,
    cursor: &Option<MenuCursor>,
) -> Result<&'a crate::RuntimeVerticalMenuSnapshot> {
    let cursor = cursor
        .as_ref()
        .with_context(|| format!("menu {} is active without a live cursor", menu.menu_id))?;
    menu.layout
        .vertical_menus
        .iter()
        .find(|vertical| vertical_menu_surface_id(menu, vertical) == cursor.surface_id)
        .with_context(|| {
            format!(
                "menu {} has no vertical surface {}",
                menu.menu_id, cursor.surface_id
            )
        })
}

fn vertical_menu_surface_id(
    menu: &crate::RuntimeMenuSnapshot,
    vertical: &crate::RuntimeVerticalMenuSnapshot,
) -> String {
    format!(
        "{}:{}:{}",
        menu.menu_id, vertical.source_script, vertical.verticalmenu_command_index
    )
}

fn visible_cursor_index(
    cursor_slot: &mut Option<MenuCursor>,
    surface_id: &str,
    option_count: usize,
) -> usize {
    match cursor_slot {
        Some(cursor) if cursor.surface_id == surface_id && cursor.option_index < option_count => {
            cursor.option_index
        }
        _ => {
            *cursor_slot = Some(MenuCursor {
                surface_id: surface_id.to_string(),
                option_index: 0,
            });
            0
        }
    }
}

fn visible_local_link_descriptor(
    runtime_shell: &mut BevyRuntimeShell,
    session_id: String,
) -> Result<RuntimeLinkSessionDescriptor> {
    let snapshot = runtime_shell.shell.snapshot()?;
    runtime_shell.shell.link_session_descriptor(
        session_id,
        LOCAL_PLAYER_ID,
        snapshot.trainer.player_name,
    )
}

fn explicit_script_runtime_inputs(
    _runtime_shell: &BevyRuntimeShell,
    _command: &str,
    _args: &[String],
    _command_index: usize,
) -> Result<ScriptRuntimeInputs> {
    Ok(ScriptRuntimeInputs::default())
}

fn explicit_compiled_script_runtime_inputs(
    runtime_shell: &BevyRuntimeShell,
    source_script: &str,
    command_index: usize,
) -> Result<ScriptRuntimeInputs> {
    let origin_map_name = runtime_shell.shell.current_map_name().to_string();
    runtime_shell
        .shell
        .compiled_script_runtime_inputs(&origin_map_name, source_script, command_index)
}

fn explicit_compiled_script_phone_inputs(
    runtime_shell: &BevyRuntimeShell,
    source_script: &str,
    command_index: usize,
) -> ScriptPhoneInputs {
    let accepted = runtime_shell
        .pending_phone_prompt
        .as_ref()
        .filter(|prompt| {
            prompt.source_script == source_script && prompt.command_index == command_index
        })
        .and_then(|_| {
            readonly_cursor_index(&runtime_shell.yes_no_cursor, "ui:phone-number", 2)
                .map(|selected| selected == 0)
        });
    ScriptPhoneInputs { accepted }
}

fn compiled_special_routine_at(
    runtime_shell: &BevyRuntimeShell,
    source_script: &str,
    command_index: usize,
) -> Result<Option<String>> {
    // Read the canonical compiled command body. Derived per-map runtime
    // indexes can omit a shared ASM script label, while the script runner and
    // its cursor always address this body directly.
    let command = runtime_shell
        .shell
        .runtime()
        .compiled_script_commands(source_script)?
        .into_iter()
        .nth(command_index);
    Ok(command
        .filter(|command| command.get("command").and_then(serde_json::Value::as_str) == Some("special"))
        .and_then(|command| {
            command
                .get("args")
                .and_then(serde_json::Value::as_array)
                .and_then(|args| args.first())
                .and_then(serde_json::Value::as_str)
                .map(str::to_string)
        }))
}

fn close_active_runtime_surface(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    if snapshot.ui.menu.is_some() {
        record_visible_runtime_action(runtime_shell, "ui:menu:close")?;
        if snapshot.ui.menu.as_ref().is_some_and(|menu| {
            menu.layout
                .vertical_menus
                .iter()
                .any(|vertical| !vertical.options.is_empty())
        }) {
            runtime_shell.shell.set_script_runtime_accumulator("0")?;
        }
        let close = runtime_shell.shell.close_active_menu()?;
        mark_runtime_snapshot_dirty(runtime_shell);
        reset_visible_selection_cursors(runtime_shell);
        runtime_shell.last_audio_events.push(format!(
            "closed menu {} {:?}",
            close.menu, close.state_checksum
        ));
        trim_event_log(&mut runtime_shell.last_audio_events);
        continue_visible_script_after_prompt(runtime_shell)?;
        return Ok(());
    }
    if snapshot.ui.text_window_open {
        return close_visible_text_window(runtime_shell);
    }
    if snapshot.ui.window_open {
        record_visible_runtime_action(runtime_shell, "ui:runtime_window:close")?;
        let close = runtime_shell.shell.close_runtime_window()?;
        mark_runtime_snapshot_dirty(runtime_shell);
        runtime_shell
            .last_audio_events
            .push(format!("closed runtime window {:?}", close.state_checksum));
        trim_event_log(&mut runtime_shell.last_audio_events);
        continue_visible_script_after_prompt(runtime_shell)?;
        return Ok(());
    }
    if snapshot.ui.active_pokemon_picture.is_some() {
        return close_visible_pokemon_picture(runtime_shell);
    }
    if snapshot.pending_shop.is_some() {
        return close_visible_shop(runtime_shell);
    }
    record_visible_runtime_action(runtime_shell, "ui:close:no_surface")?;
    runtime_shell
        .last_audio_events
        .push("no active runtime surface to close".to_string());
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn cancel_visible_2d_menu(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    record_visible_runtime_action(runtime_shell, "ui:2d_menu:cancel")?;
    runtime_shell
        .shell
        .consume_script_runtime_flag(RuntimeScriptRuntimeFlag::Menu2dRequested)?;
    runtime_shell.shell.set_script_runtime_accumulator("0")?;
    let close = runtime_shell.shell.close_active_menu()?;
    runtime_shell.menu_cursor = None;
    runtime_shell.last_audio_events.push(format!(
        "cancelled 2D menu {} {:?}",
        close.menu, close.state_checksum
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    continue_visible_script_after_prompt(runtime_shell)
}

fn close_visible_text_window(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    record_visible_runtime_action(runtime_shell, "ui:text_window:close")?;
    let close = runtime_shell.shell.close_text_window()?;
    runtime_shell
        .last_audio_events
        .push(format!("closed text window {:?}", close.state_checksum));
    trim_event_log(&mut runtime_shell.last_audio_events);
    continue_visible_script_after_prompt(runtime_shell)?;
    Ok(())
}

fn close_visible_pokemon_picture(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    record_visible_runtime_action(runtime_shell, "ui:pokemon_picture:close")?;
    let close = runtime_shell.shell.close_active_pokemon_picture()?;
    runtime_shell.last_audio_events.push(format!(
        "closed pokemon picture {} {:?}",
        close.species_id, close.state_checksum
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    continue_visible_script_after_prompt(runtime_shell)?;
    Ok(())
}

fn select_visible_menu_option(
    runtime_shell: &mut BevyRuntimeShell,
    option_index: usize,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let Some(menu) = snapshot.ui.menu else {
        return handle_visible_no_active_menu(runtime_shell, "select");
    };
    let vertical = selected_vertical_menu(&menu, &runtime_shell.menu_cursor)?;
    let Some(option) = vertical.options.get(option_index) else {
        record_visible_runtime_action(
            runtime_shell,
            format!("ui:menu:{}:option:{option_index}:unavailable", menu.menu_id),
        )?;
        runtime_shell.last_audio_events.push(format!(
            "menu {} has no option index {}",
            menu.menu_id, option_index
        ));
        set_shell_action_status(runtime_shell, "OPTION UNAVAILABLE");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    };
    let menu_id = menu.menu_id.clone();
    let source_script = vertical.source_script.clone();
    let verticalmenu_command_index = vertical.verticalmenu_command_index;
    let selected_option = option.clone();
    record_visible_runtime_action(
        runtime_shell,
        format!(
            "ui:menu:{}:{}:{}:{}:{}",
            menu_id.as_str(),
            source_script.as_str(),
            verticalmenu_command_index,
            option_index,
            selected_option.as_str()
        ),
    )?;
    if menu.menu_2d_requested {
        runtime_shell
            .shell
            .consume_script_runtime_flag(RuntimeScriptRuntimeFlag::Menu2dRequested)?;
    }
    let next_cursor = visible_active_compiled_script_cursor(runtime_shell);
    let selected = if let Some(cursor) = next_cursor {
        runtime_shell
            .shell
            .select_vertical_menu_option_and_run_compiled_script(
                menu_id,
                source_script,
                verticalmenu_command_index,
                option_index,
                selected_option,
                Some(cursor),
                256,
                ScriptRuntimeInputs::default(),
                ScriptPhoneInputs::default(),
            )?
    } else {
        let selection = runtime_shell.shell.select_vertical_menu_option(
            menu_id,
            source_script,
            verticalmenu_command_index,
            option_index,
            selected_option,
        )?;
        crate::RuntimeMenuSelectionCompiledScriptRun {
            selection,
            run: crate::RuntimeCompiledScriptRun {
                steps: Vec::new(),
                next_cursor: None,
                boundary: None,
                ended: false,
            },
        }
    };
    let selection = selected.selection;
    let menu_result = visible_menu_choice_result_frame(
        &selection,
        snapshot.state_checksum.frame(),
        verticalmenu_command_index,
    )?;
    runtime_shell
        .deterministic_menu_results
        .push_back(menu_result.clone());
    runtime_shell.last_audio_events.push(format!(
        "selected menu option {}={} script_value={} resumed_steps={} choice_frame={} result_frame={} checksum={:?}",
        selection.option_index,
        selection.option,
        selection.script_value,
        selected.run.steps.len(),
        menu_result.choice().frame(),
        menu_result.checksum().frame(),
        selection.state_checksum
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    runtime_shell.menu_cursor = None;
    let reached_boundary =
        integrate_visible_compiled_script_run(runtime_shell, &selected.run.steps)?;
    arm_visible_active_script_cursor_from_run(runtime_shell, selected.run.next_cursor);
    if reached_boundary {
        return Ok(());
    }
    continue_visible_script_after_prompt(runtime_shell)?;
    Ok(())
}

fn visible_menu_choice_result_frame(
    selection: &crate::RuntimeVerticalMenuOptionSelection,
    choice_frame: u64,
    verticalmenu_command_index: usize,
) -> Result<MenuChoiceResultFrame> {
    let choice = MenuChoiceFrame::new(
        LOCAL_PLAYER_ID,
        Frame(choice_frame),
        selection.menu_id.clone(),
        selection.option_index,
        verticalmenu_command_index,
    )
    .context("build visible menu choice frame")?;
    MenuChoiceResultFrame::new(
        choice,
        StateChecksumFrame::new(
            LOCAL_PLAYER_ID,
            Frame(selection.state_checksum.frame()),
            selection.state_checksum.hash(),
        ),
        selection.script_value.clone(),
    )
    .context("build visible menu choice result frame")
}

fn select_visible_linked_menu_option(
    runtime_shell: &mut BevyRuntimeShell,
    option_index: usize,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let Some(menu) = snapshot.ui.menu else {
        return handle_visible_no_active_menu(runtime_shell, "linked_select");
    };
    let vertical = selected_vertical_menu(&menu, &runtime_shell.menu_cursor)?;
    let Some(option) = vertical.options.get(option_index) else {
        record_visible_runtime_action(
            runtime_shell,
            format!(
                "ui:linked_menu:{}:option:{option_index}:unavailable",
                menu.menu_id
            ),
        )?;
        runtime_shell.last_audio_events.push(format!(
            "linked menu {} has no option index {}",
            menu.menu_id, option_index
        ));
        set_shell_action_status(runtime_shell, "OPTION UNAVAILABLE");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    };
    let session_id = format!(
        "bevy-local-menu-{}-{}",
        snapshot.state_checksum.frame(),
        option_index
    );
    let descriptor = visible_local_link_descriptor(runtime_shell, session_id.clone())?;
    let menu_id = menu.menu_id.clone();
    let source_script = vertical.source_script.clone();
    let verticalmenu_command_index = vertical.verticalmenu_command_index;
    let selected_option = option.clone();
    record_visible_runtime_action(
        runtime_shell,
        format!(
            "ui:linked_menu:{}:{}:{}:{}:{}:{}:{}",
            session_id,
            descriptor.local_player.id(),
            menu_id.as_str(),
            source_script.as_str(),
            verticalmenu_command_index,
            option_index,
            selected_option.as_str()
        ),
    )?;
    let choice = runtime_shell.shell.select_linked_vertical_menu_option(
        &descriptor,
        menu_id,
        source_script,
        verticalmenu_command_index,
        option_index,
        selected_option,
    )?;
    let result = runtime_shell
        .shell
        .record_linked_menu_choice_result(&descriptor, &choice)?;
    runtime_shell
        .deterministic_menu_results
        .push_back(result.clone());
    runtime_shell.last_audio_events.push(format!(
        "linked menu session={} player={} option {}={} script_value={} choice_frame={} result_frame={} checksum={:?}",
        session_id,
        descriptor.local_player.id(),
        choice.selection.option_index,
        choice.selection.option,
        choice.selection.script_value,
        choice.frame.frame(),
        result.checksum().frame(),
        choice.selection.state_checksum
    ));
    runtime_shell.menu_cursor = None;
    trim_event_log(&mut runtime_shell.last_audio_events);
    continue_visible_script_after_prompt(runtime_shell)?;
    Ok(())
}

fn advance_visible_text_label(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    record_visible_runtime_action(runtime_shell, "script:pending:text_label")?;
    let label = runtime_shell
        .shell
        .take_pending_script_request(RuntimePendingScriptRequestKind::TextLabel)?;
    runtime_shell
        .last_audio_events
        .push(format!("advanced text label {:?}", label));
    if visible_pending_text_wait_closes_window(runtime_shell) {
        let advance = runtime_shell.shell.advance_pending_text_wait()?;
        runtime_shell.last_audio_events.push(format!(
            "advanced paired text wait {:?}",
            advance.state_checksum
        ));
    }
    continue_visible_script_after_prompt(runtime_shell)?;
    Ok(())
}

fn visible_pending_text_wait_closes_window(runtime_shell: &BevyRuntimeShell) -> bool {
    runtime_shell
        .shell
        .session()
        .state
        .script_runtime
        .pending_text_wait
        .as_ref()
        .map(|wait| {
            matches!(
                wait.command.as_str(),
                "jumptext" | "jumptextfaceplayer" | "farjumptext"
            )
        })
        .unwrap_or(false)
}

fn take_visible_pending_map_load(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    record_visible_runtime_action(runtime_shell, "script:pending:map_load")?;
    let request = runtime_shell
        .shell
        .take_pending_script_request(RuntimePendingScriptRequestKind::MapLoad)?;
    let RuntimePendingScriptRequest::MapLoad(load) = &request else {
        anyhow::bail!("pending map-load request resolved to a different request kind");
    };
    let reload_current_map = matches!(load.command.as_str(), "reloadmap" | "reloadmapafterbattle");
    let map_setup = if reload_current_map {
        "MAPSETUP_RELOADMAP"
    } else {
        load.map_setup
            .as_deref()
            .with_context(|| format!("{} is missing its map setup", load.command))?
    };
    map_setup_callback_kinds(map_setup)
        .with_context(|| format!("unknown map setup callback path {map_setup}"))?;
    let new_map_load = matches!(
        &request,
        RuntimePendingScriptRequest::MapLoad(load) if load.command == "newloadmap"
    );
    if new_map_load {
        if runtime_shell
            .shell
            .session
            .state
            .script_runtime
            .pending_field_travel
            .is_some()
        {
            let committed = runtime_shell.shell.commit_pending_field_travel()?;
            runtime_shell.last_audio_events.push(format!(
                "newloadmap committed field travel move={} destination={} tile=({}, {})",
                committed.move_id,
                committed.destination_map,
                committed.destination_tile.x,
                committed.destination_tile.y
            ));
        } else if runtime_shell
            .shell
            .session
            .state
            .script_runtime
            .pending_script_warp
            .is_some()
        {
            let warp = runtime_shell.shell.execute_pending_script_warp()?;
            runtime_shell.last_audio_events.push(format!(
                "newloadmap committed staged warp destination={} tile=({}, {})",
                warp.target_map, warp.tile.x, warp.tile.y
            ));
        }
    }
    let reload_return_cursor = reload_current_map
        .then(|| visible_active_compiled_script_cursor(runtime_shell))
        .flatten();
    reset_visible_navigation_state(runtime_shell);
    runtime_shell
        .last_audio_events
        .push(format!("took pending map load {:?}", request));
    if reload_current_map {
        // MAPSETUP_RELOADMAP preserves the live object buffer and does not
        // run the map's scene, NEWMAP, OBJECTS, or CMDQUEUE callbacks. Its
        // LoadBlockData and LoadMapGraphics steps invoke TILES then SPRITES.
        runtime_shell.pending_scene_script = None;
        runtime_shell.map_reload_return_cursor = reload_return_cursor;
        runtime_shell
            .shell
            .apply_current_map_setup_callbacks(map_setup)?;
        continue_visible_script_after_prompt(runtime_shell)?;
        runtime_shell.visible_walk_warp_phase = Some(VisibleWalkWarpPhase::MapReloadFadeIn);
        runtime_shell.screen_fade = Some(VisibleScreenFade::new(
            ScriptFadeColor::White,
            ScriptFadeDirection::In,
            8,
        ));
    } else {
        if matches!(map_setup, "MAPSETUP_BADWARP" | "MAPSETUP_LINKRETURN") {
            runtime_shell
                .shell
                .apply_current_map_setup_callbacks(map_setup)?;
        }
        arm_visible_current_scene_script(runtime_shell, "map_load")?;
        take_visible_pending_scene_script(runtime_shell)?;
    }
    queue_visible_current_music(runtime_shell)?;
    if !reload_current_map {
        continue_visible_script_after_prompt(runtime_shell)?;
    }
    Ok(())
}

fn take_visible_pending_map_refresh(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    record_visible_runtime_action(runtime_shell, "script:pending:map_refresh")?;
    let request = runtime_shell
        .shell
        .take_pending_script_request(RuntimePendingScriptRequestKind::MapRefresh)?;
    runtime_shell
        .last_audio_events
        .push(format!("took pending map refresh {:?}", request));
    reset_visible_selection_cursors(runtime_shell);
    continue_visible_script_after_prompt(runtime_shell)?;
    Ok(())
}

fn arm_visible_current_scene_script(
    runtime_shell: &mut BevyRuntimeShell,
    reason: &str,
) -> Result<()> {
    let Some(scene) = runtime_shell.shell.current_scene_script()? else {
        runtime_shell.pending_scene_script = None;
        runtime_shell
            .last_audio_events
            .push(format!("scene script none reason={reason}"));
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    };
    runtime_shell.pending_scene_script = scene.script_name.clone();
    runtime_shell.last_audio_events.push(format!(
        "scene armed map={} scene={} script={:?} reason={}",
        scene.map_name, scene.scene_id, scene.script_name, reason
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}
