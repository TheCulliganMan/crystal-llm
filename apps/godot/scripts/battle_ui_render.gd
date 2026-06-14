extends RefCounted
class_name BattleUIRender

func build_signature(runtime) -> String:
	if runtime == null or runtime.battle_state == null:
		return "empty"
	var state = runtime.battle_state
	var dialogue_visible := false
	if runtime.battle_ui_state != null and runtime.battle_ui_state.dialogue != null:
		dialogue_visible = runtime.battle_ui_state.dialogue.is_visible()
	return "%s|%s|%d|%s|%s|%s|%s" % [
		str(state.battle_label),
		str(state.battle_kind),
		int(runtime.battle_frame),
		str(state.ui_phase),
		str(dialogue_visible),
		str(runtime.battle_ui_state.submenu_stack.hash() if runtime.battle_ui_state != null else 0),
		str(_build_animation_metadata(runtime).hash()),
	]

func build_lines(runtime) -> Dictionary:
	var state = runtime.battle_state
	var ui_state = runtime.battle_ui_state
	var animation_metadata := _build_animation_metadata(runtime)
	return {
		"title": "BATTLE",
		"battle": "Battle: %s | Kind: %s | Frame: %d | Rev: %d | UI: %s" % [
			state.battle_label,
			state.battle_kind,
			runtime.battle_frame,
			state.state_revision,
			state.ui_phase,
		],
		"phase": "Phase: %s | Active Side: %s | Turn: %d" % [
			state.describe_phase(),
			state.active_side,
			state.turn_number,
		],
		"turn": "Turn State: gate=%s locked=%s waiting=%s manual=%s finished=%s result=%s" % [
			str(state.prompt_gate_active).to_lower(),
			str(state.prompt_locked).to_lower(),
			str(state.waiting_for_input).to_lower(),
			str(state.manual_wait_override).to_lower(),
			str(state.battle_finished).to_lower(),
			state.battle_result,
		],
		"prompt": "Prompt: %s / %s | Gate: %s" % [
			state.prompt_kind if not state.prompt_kind.is_empty() else "none",
			state.prompt_message if not state.prompt_message.is_empty() else "idle",
			str(state.dialogue_wait_gate_active).to_lower(),
		],
		"queue": "Queued commands: %d | Pending: %s | Resolution: %d" % [
			int(state.queued_commands.size()),
			"yes" if not state.pending_command.is_empty() else "no",
			int(state.resolution_events.size()),
		],
		"flow": "Prompt flow: %s | Resolved: %s | UI phase: %s" % [
			state.describe_prompt_gate(),
			"yes" if not state.last_resolved_command.is_empty() else "no",
			state.ui_phase,
		],
		"history": "History: %s | Logs: %s" % [
			state.describe_recent_history(),
			state.describe_recent_logs(2),
		],
		"submenu": "Submenu: %s [%d]" % [
			ui_state.current_submenu() if ui_state != null else "none",
			int(ui_state.submenu_index if ui_state != null else 0),
		],
		"animation": _animation_line(animation_metadata),
		"animation_metadata": animation_metadata,
		"debug": state.debug_text(),
	}

func _build_animation_metadata(runtime) -> Dictionary:
	if runtime == null or runtime.battle_state == null:
		return {}
	var state = runtime.battle_state
	var assets = runtime.battle_assets if runtime.has_method("get") and runtime.get("battle_assets") != null else null
	if assets == null:
		return {}
	var resolution: Dictionary = {}
	if state.has_method("get_last_turn_resolution"):
		resolution = state.get_last_turn_resolution()
	elif state.has("last_turn_resolution"):
		resolution = Dictionary(state.last_turn_resolution).duplicate(true)
	var move_payload: Dictionary = Dictionary(resolution.get("move", {}))
	var move_animation: Dictionary = {}
	if not move_payload.is_empty() and assets.has_method("get_move_animation"):
		move_animation = Dictionary(assets.call("get_move_animation", move_payload))
	var player_frontpic := _frontpic_metadata_for_payload(assets, state.get_selected_player_payload() if state.has_method("get_selected_player_payload") else {})
	var opponent_frontpic := _frontpic_metadata_for_payload(assets, state.get_selected_opponent_payload() if state.has_method("get_selected_opponent_payload") else {})
	return {
		"move_animation": move_animation,
		"player_frontpic_animation": player_frontpic,
		"opponent_frontpic_animation": opponent_frontpic,
		"resolution_animation_hooks": Dictionary(resolution.get("animation_hooks", {})),
		"latest_animation_event": _latest_animation_event(state),
	}

func _frontpic_metadata_for_payload(assets, payload: Dictionary) -> Dictionary:
	if assets == null or payload.is_empty() or not assets.has_method("get_frontpic_animation_for_payload"):
		return {}
	var entry: Dictionary = Dictionary(assets.call("get_frontpic_animation_for_payload", payload))
	if entry.is_empty():
		return {}
	return {
		"species_id": str(entry.get("species_id", "")),
		"frontpic_key": str(entry.get("frontpic_key", "")),
		"command_count": int(entry.get("command_count", 0)),
		"frame_count": int(entry.get("frame_count", 0)),
		"total_duration": int(entry.get("total_duration", 0)),
	}

func _latest_animation_event(state) -> Dictionary:
	var events: Array = state.get_resolution_events() if state.has_method("get_resolution_events") else []
	for offset in range(events.size()):
		var index := events.size() - 1 - offset
		var event: Variant = events[index]
		if typeof(event) != TYPE_DICTIONARY:
			continue
		var event_dictionary: Dictionary = event
		if event_dictionary.has("animation_key") or event_dictionary.has("animation_hooks"):
			return event_dictionary.duplicate(true)
	return {}

func _animation_line(metadata: Dictionary) -> String:
	var move_animation: Dictionary = Dictionary(metadata.get("move_animation", {}))
	var player_frontpic: Dictionary = Dictionary(metadata.get("player_frontpic_animation", {}))
	var opponent_frontpic: Dictionary = Dictionary(metadata.get("opponent_frontpic_animation", {}))
	var move_label := str(move_animation.get("animation_label", "none"))
	var table_index := int(move_animation.get("table_index", -1))
	var player_frames := int(player_frontpic.get("frame_count", 0))
	var opponent_frames := int(opponent_frontpic.get("frame_count", 0))
	return "Animation: move=%s table=%d frontpic[player=%d enemy=%d]" % [
		move_label if not move_label.is_empty() else "none",
		table_index,
		player_frames,
		opponent_frames,
	]
