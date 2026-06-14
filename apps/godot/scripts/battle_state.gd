extends RefCounted
class_name BattleState

const PHASE_SETUP := "setup"
const PHASE_INTRO := "intro"
const PHASE_TURN_PROMPT := "turn_prompt"
const PHASE_RESOLUTION := "resolution"
const PHASE_POST_TURN := "post_turn"
const PHASE_COMPLETE := "complete"

const SIDE_NONE := "none"
const SIDE_PLAYER := "player"
const SIDE_ENEMY := "enemy"
const UI_PHASE_INACTIVE := "INACTIVE"
const UI_PHASE_MENU := "MENU"
const UI_PHASE_DIALOGUE := "DIALOGUE"
const UI_PHASE_ANIMATION := "ANIMATION"
const UI_PHASE_COMPLETE := "COMPLETE"

const DEFAULT_LOG_LIMIT := 8
const DEFAULT_PROMPT_KIND := "turn_command"
const DEFAULT_PROMPT_MESSAGE := "awaiting player command"
const RESULT_ONGOING := "ongoing"
const RESULT_WIN := "win"
const RESULT_LOSS := "loss"
const RESULT_DRAW := "draw"

const PHYSICAL_TYPES := {
	"NORMAL": true,
	"FIGHTING": true,
	"FLYING": true,
	"POISON": true,
	"GROUND": true,
	"ROCK": true,
	"BUG": true,
	"GHOST": true,
	"STEEL": true,
}

const TYPE_EFFECTIVENESS := {
	"NORMAL": {"ROCK": 0.5, "GHOST": 0.0, "STEEL": 0.5},
	"FIRE": {"FIRE": 0.5, "WATER": 0.5, "GRASS": 2.0, "ICE": 2.0, "BUG": 2.0, "ROCK": 0.5, "DRAGON": 0.5, "STEEL": 2.0},
	"WATER": {"FIRE": 2.0, "WATER": 0.5, "GRASS": 0.5, "GROUND": 2.0, "ROCK": 2.0, "DRAGON": 0.5},
	"GRASS": {"FIRE": 0.5, "WATER": 2.0, "GRASS": 0.5, "POISON": 0.5, "GROUND": 2.0, "FLYING": 0.5, "BUG": 0.5, "ROCK": 2.0, "DRAGON": 0.5, "STEEL": 0.5},
	"ELECTRIC": {"WATER": 2.0, "GRASS": 0.5, "ELECTRIC": 0.5, "GROUND": 0.0, "FLYING": 2.0, "DRAGON": 0.5},
	"ICE": {"FIRE": 0.5, "WATER": 0.5, "GRASS": 2.0, "ICE": 0.5, "GROUND": 2.0, "FLYING": 2.0, "DRAGON": 2.0, "STEEL": 0.5},
	"FIGHTING": {"NORMAL": 2.0, "ICE": 2.0, "POISON": 0.5, "FLYING": 0.5, "PSYCHIC": 0.5, "BUG": 0.5, "ROCK": 2.0, "GHOST": 0.0, "DARK": 2.0, "STEEL": 2.0},
	"POISON": {"GRASS": 2.0, "POISON": 0.5, "GROUND": 0.5, "ROCK": 0.5, "GHOST": 0.5, "STEEL": 0.0},
	"GROUND": {"FIRE": 2.0, "GRASS": 0.5, "ELECTRIC": 2.0, "POISON": 2.0, "FLYING": 0.0, "BUG": 0.5, "ROCK": 2.0, "STEEL": 2.0},
	"FLYING": {"GRASS": 2.0, "ELECTRIC": 0.5, "FIGHTING": 2.0, "BUG": 2.0, "ROCK": 0.5, "STEEL": 0.5},
	"PSYCHIC": {"FIGHTING": 2.0, "POISON": 2.0, "PSYCHIC": 0.5, "DARK": 0.0, "STEEL": 0.5},
	"BUG": {"FIRE": 0.5, "GRASS": 2.0, "FIGHTING": 0.5, "POISON": 0.5, "FLYING": 0.5, "PSYCHIC": 2.0, "GHOST": 0.5, "DARK": 2.0, "STEEL": 0.5},
	"ROCK": {"FIRE": 2.0, "ICE": 2.0, "FIGHTING": 0.5, "GROUND": 0.5, "FLYING": 2.0, "BUG": 2.0, "STEEL": 0.5},
	"GHOST": {"NORMAL": 0.0, "PSYCHIC": 2.0, "GHOST": 2.0, "DARK": 0.5},
	"DRAGON": {"DRAGON": 2.0, "STEEL": 0.5},
	"DARK": {"FIGHTING": 0.5, "PSYCHIC": 2.0, "GHOST": 2.0, "DARK": 0.5, "STEEL": 0.5},
	"STEEL": {"FIRE": 0.5, "WATER": 0.5, "ELECTRIC": 0.5, "ICE": 2.0, "ROCK": 2.0, "STEEL": 0.5},
}
const EVENT_SHOW_TEXT := "show_text"
const EVENT_OPEN_TEXT := "open_text"
const EVENT_CLOSE_TEXT := "close_text"
const EVENT_WAIT_FOR_INPUT := "wait_for_input"
const EVENT_PROMPT_YES_NO := "prompt_yes_no"
const EVENT_NICKNAME_PROMPT := "nickname_prompt"
const EVENT_PLAY_ANIMATION := "play_animation"
const EVENT_FRONTPIC_ANIMATION := "frontpic_animation"
const EVENT_SHOW_TRAINER_SPRITES := "show_trainer_sprites"
const EVENT_TRIGGER_TRAINER_EXIT := "trigger_trainer_exit"

var battle_id: String = ""
var battle_kind: String = "wild"
var battle_label: String = "battle shell"
var turn_phase: String = PHASE_SETUP
var turn_number: int = 0
var active_side: String = SIDE_NONE
var prompt_gate_active: bool = false
var prompt_gate_reason: String = ""
var prompt_kind: String = ""
var prompt_message: String = ""
var prompt_locked: bool = false
var queued_commands: Array = []
var pending_command: Dictionary = {}
var last_resolved_command: Dictionary = {}
var last_turn_resolution: Dictionary = {}
var resolution_events: Array = []
var resolution_event_sequence: int = 0
var pending_animation_events: Array = []
var last_battle_event: Dictionary = {}
var active_text_event: Dictionary = {}
var active_animation_event: Dictionary = {}
var trainer_sprite_state: Dictionary = {}
var battle_finished: bool = false
var battle_result: String = ""
var battle_result_state: Dictionary = {}
var frame_counter: int = 0
var fixed_step_count: int = 0
var state_revision: int = 0
var waiting_for_input: bool = false
var manual_wait_override: bool = false
var ui_phase: String = UI_PHASE_INACTIVE
var dialogue_wait_gate_active: bool = false
var fast_animation_request: bool = false
var fast_text_request: bool = false
var battle_context: Dictionary = {}
var asset_summary: Dictionary = {}
var selected_player_payload: Dictionary = {}
var selected_opponent_payload: Dictionary = {}
var phase_history: Array[String] = []
var log_lines: Array[String] = []

func reset() -> void:
	battle_id = ""
	battle_kind = "wild"
	battle_label = "battle shell"
	turn_phase = PHASE_SETUP
	turn_number = 0
	active_side = SIDE_NONE
	prompt_gate_active = false
	prompt_gate_reason = ""
	prompt_kind = ""
	prompt_message = ""
	prompt_locked = false
	queued_commands = []
	pending_command = {}
	last_resolved_command = {}
	last_turn_resolution = {}
	resolution_events = []
	resolution_event_sequence = 0
	pending_animation_events = []
	last_battle_event = {}
	active_text_event = {}
	active_animation_event = {}
	trainer_sprite_state = {}
	battle_finished = false
	battle_result = ""
	battle_result_state = _default_battle_result_state()
	frame_counter = 0
	fixed_step_count = 0
	state_revision = 0
	waiting_for_input = false
	manual_wait_override = false
	ui_phase = UI_PHASE_INACTIVE
	dialogue_wait_gate_active = false
	fast_animation_request = false
	fast_text_request = false
	battle_context = {}
	asset_summary = {}
	selected_player_payload = {}
	selected_opponent_payload = {}
	phase_history = [PHASE_SETUP]
	log_lines = ["battle shell ready"]

func set_context(context: Dictionary) -> void:
	battle_context = _sanitize_dictionary(context, {})
	if battle_context.has("battle_id"):
		battle_id = _coerce_string(battle_context.get("battle_id", battle_id), battle_id)
	if battle_context.has("battle_kind"):
		battle_kind = _coerce_string(battle_context.get("battle_kind", battle_kind), battle_kind)
	if battle_context.has("battle_label"):
		battle_label = _coerce_string(battle_context.get("battle_label", battle_label), battle_label)
	_bump_revision()

func set_asset_summary(summary: Dictionary) -> void:
	asset_summary = _sanitize_dictionary(summary, {})
	_bump_revision()

func set_selected_player_payload(payload: Dictionary) -> void:
	selected_player_payload = _sanitize_dictionary(payload, {})
	_bump_revision()

func get_selected_player_payload() -> Dictionary:
	return selected_player_payload.duplicate(true)

func clear_selected_player_payload() -> void:
	if selected_player_payload.is_empty():
		return
	selected_player_payload = {}
	_bump_revision()

func set_selected_opponent_payload(payload: Dictionary) -> void:
	selected_opponent_payload = _sanitize_dictionary(payload, {})
	_bump_revision()

func get_selected_opponent_payload() -> Dictionary:
	return selected_opponent_payload.duplicate(true)

func clear_selected_opponent_payload() -> void:
	if selected_opponent_payload.is_empty():
		return
	selected_opponent_payload = {}
	_bump_revision()

func set_selected_battle_payloads(player_payload: Dictionary, opponent_payload: Dictionary) -> void:
	selected_player_payload = _sanitize_dictionary(player_payload, {})
	selected_opponent_payload = _sanitize_dictionary(opponent_payload, {})
	_bump_revision()

func set_phase(next_phase: String) -> void:
	var normalized := _normalize_phase(next_phase)
	if normalized == turn_phase:
		return
	turn_phase = normalized
	_record_phase(normalized)
	_sync_ui_state()
	_bump_revision()
	_push_log("phase -> %s" % turn_phase)

func set_prompt_gate(active: bool, kind: String = "", message: String = "", locked: bool = false, reason: String = "") -> void:
	prompt_gate_active = active
	prompt_kind = _coerce_string(kind, "")
	prompt_message = _coerce_string(message, "")
	prompt_locked = locked
	prompt_gate_reason = _coerce_string(reason, "")
	_sync_ui_state()
	_bump_revision()
	_push_log(_prompt_summary())

func clear_prompt_gate() -> void:
	prompt_gate_active = false
	prompt_kind = ""
	prompt_message = ""
	prompt_locked = false
	prompt_gate_reason = ""
	_sync_ui_state()
	_bump_revision()
	_push_log("prompt gate cleared")

func queue_command(command: Dictionary) -> void:
	enqueue_command(command)

func enqueue_command(command: Dictionary) -> void:
	var normalized := _sanitize_dictionary(command, {})
	if normalized.is_empty():
		return
	queued_commands.append(normalized)
	if pending_command.is_empty():
		pending_command = normalized.duplicate(true)
	_bump_revision()
	_push_log("queued command: %s" % _command_label(normalized))

func consume_pending_command() -> Dictionary:
	if queued_commands.is_empty():
		pending_command = {}
		return {}
	var command: Dictionary = queued_commands.pop_front()
	pending_command = queued_commands[0].duplicate(true) if not queued_commands.is_empty() else {}
	_bump_revision()
	return command

func clear_queued_commands() -> void:
	queued_commands = []
	pending_command = {}
	_bump_revision()
	_push_log("command queue cleared")

func has_pending_command() -> bool:
	return not pending_command.is_empty()

func get_pending_command() -> Dictionary:
	return pending_command.duplicate(true)

func get_last_resolved_command() -> Dictionary:
	return last_resolved_command.duplicate(true)

func get_last_turn_resolution() -> Dictionary:
	return last_turn_resolution.duplicate(true)

func get_battle_result_state() -> Dictionary:
	return battle_result_state.duplicate(true)

func get_state_revision() -> int:
	return state_revision

func get_dialogue_wait_gate_active() -> bool:
	return dialogue_wait_gate_active

func get_fast_animation_request() -> bool:
	return fast_animation_request

func get_fast_text_request() -> bool:
	return fast_text_request

func get_waiting_for_input() -> bool:
	return waiting_for_input

func get_phase_history() -> Array:
	var history: Array = []
	for phase in phase_history:
		history.append(str(phase))
	return history

func get_log_lines() -> Array:
	var lines: Array = []
	for line in log_lines:
		lines.append(str(line))
	return lines

func get_resolution_event_count() -> int:
	return resolution_events.size()

func has_resolution_events() -> bool:
	return not resolution_events.is_empty()

func get_resolution_events() -> Array:
	var events: Array = []
	for event in resolution_events:
		if event is Dictionary:
			events.append(Dictionary(event).duplicate(true))
		else:
			events.append(_sanitize_dictionary(event, {}))
	return events

func get_pending_animation_events() -> Array:
	var events: Array = []
	for event in pending_animation_events:
		if event is Dictionary:
			events.append(Dictionary(event).duplicate(true))
	return events

func has_pending_animation_events() -> bool:
	return not pending_animation_events.is_empty()

func consume_pending_animation_events() -> Array:
	var events := get_pending_animation_events()
	if not pending_animation_events.is_empty():
		pending_animation_events = []
		_bump_revision()
	return events

func get_last_battle_event() -> Dictionary:
	return last_battle_event.duplicate(true)

func get_active_text_event() -> Dictionary:
	return active_text_event.duplicate(true)

func get_active_animation_event() -> Dictionary:
	return active_animation_event.duplicate(true)

func get_trainer_sprite_state() -> Dictionary:
	return trainer_sprite_state.duplicate(true)

func complete_text_gate() -> void:
	if active_text_event.is_empty() and prompt_kind != "battle_text":
		return
	active_text_event = {}
	if prompt_kind == "battle_text":
		clear_prompt_gate()
	else:
		_bump_revision()

func record_resolution(command: Dictionary, resolution_message: String = "") -> void:
	last_resolved_command = _sanitize_dictionary(command, {})
	queue_resolution_event({
		"type": "turn_resolution_recorded",
		"kind": "turn_resolution_recorded",
		"turn_number": turn_number,
		"phase": turn_phase,
		"command": last_resolved_command.duplicate(true),
		"message": _coerce_string(resolution_message, ""),
	})
	if not resolution_message.is_empty():
		_push_log(resolution_message)

func advance_turn(next_side: String = SIDE_PLAYER) -> void:
	turn_number += 1
	active_side = _normalize_side(next_side)
	_bump_revision()
	_push_log("turn %d -> %s" % [turn_number, active_side])

func mark_complete(result: String, detail: Dictionary = {}) -> void:
	battle_finished = true
	battle_result = _coerce_string(result, "unknown")
	battle_result_state = _sanitize_dictionary(detail, _default_battle_result_state())
	battle_result_state["result"] = battle_result
	battle_result_state["finished"] = true
	battle_result_state["turn_number"] = turn_number
	turn_phase = PHASE_COMPLETE
	prompt_gate_active = false
	prompt_locked = false
	_record_phase(PHASE_COMPLETE)
	_sync_ui_state()
	queue_resolution_event({
		"type": "battle_result",
		"kind": "battle_result",
		"result": battle_result,
		"result_state": battle_result_state.duplicate(true),
	})
	_bump_revision()
	_push_log("battle complete: %s" % battle_result)

func tick_fixed_step() -> void:
	fixed_step_count += 1
	frame_counter += 1
	_bump_revision()

func open_turn_prompt(kind: String = DEFAULT_PROMPT_KIND, message: String = DEFAULT_PROMPT_MESSAGE, locked: bool = true, reason: String = "") -> void:
	set_phase(PHASE_TURN_PROMPT)
	if active_side == SIDE_NONE:
		active_side = SIDE_PLAYER
	prompt_gate_active = true
	prompt_kind = _coerce_string(kind, DEFAULT_PROMPT_KIND)
	prompt_message = _coerce_string(message, DEFAULT_PROMPT_MESSAGE)
	prompt_locked = locked
	prompt_gate_reason = _coerce_string(reason, "")
	_sync_ui_state()
	_bump_revision()
	_push_log(_prompt_summary())

func begin_resolution(command_payload: Dictionary = {}) -> bool:
	var command := pending_command.duplicate(true)
	if not command_payload.is_empty():
		command = _sanitize_dictionary(command_payload, {})
		pending_command = command.duplicate(true)
		if queued_commands.is_empty():
			queued_commands = [command.duplicate(true)]
		else:
			queued_commands[0] = command.duplicate(true)
	if command.is_empty() and not queued_commands.is_empty():
		command = queued_commands[0].duplicate(true)
	if command.is_empty():
		return false
	if pending_command.is_empty():
		pending_command = command.duplicate(true)
	set_prompt_gate(true, "resolution", "resolving turn", true, "battle turn in progress")
	set_phase(PHASE_RESOLUTION)
	queue_resolution_event({
		"type": "command_submitted",
		"turn_number": turn_number,
		"phase": turn_phase,
		"command": command.duplicate(true),
	})
	resolve_turn_command(command)
	return true

func resolve_turn_command(command_payload: Dictionary) -> Dictionary:
	var command := _sanitize_dictionary(command_payload, {})
	if command.is_empty():
		return {}
	var actor := _sanitize_dictionary(command.get("actor", command.get("player", selected_player_payload)), {})
	if actor.is_empty():
		actor = selected_player_payload.duplicate(true)
	var target := _sanitize_dictionary(command.get("target", command.get("opponent", selected_opponent_payload)), {})
	if target.is_empty():
		target = selected_opponent_payload.duplicate(true)
	var move := _sanitize_dictionary(command.get("move_payload", command.get("move", {})), {})
	var resolution := _build_turn_resolution(command, actor, target, move)
	last_turn_resolution = resolution.duplicate(true)
	last_resolved_command = command.duplicate(true)
	queue_resolution_event({
		"type": "turn_resolution_started",
		"kind": "turn_resolution",
		"resolution": resolution.duplicate(true),
	})
	queue_resolution_event({
		"type": "command_validated",
		"kind": "command_validation",
		"valid": bool(resolution.get("valid", false)),
		"reason": str(resolution.get("reason", "")),
		"command": command.duplicate(true),
	})
	queue_resolution_event({
		"type": "turn_order_determined",
		"kind": "turn_order",
		"turn_order": _sanitize_array(resolution.get("turn_order", []), []),
		"actor_side": str(resolution.get("actor_side", SIDE_NONE)),
		"target_side": str(resolution.get("target_side", SIDE_NONE)),
		"command": command.duplicate(true),
	})
	if not move.is_empty():
		queue_resolution_event({
			"type": "move_selected",
			"kind": "move",
			"move": move.duplicate(true),
			"actor": actor.duplicate(true),
			"target": target.duplicate(true),
			"animation_key": _animation_key_for_move(move),
		})
	if bool(resolution.get("valid", false)):
		_apply_turn_resolution(command, resolution, actor, target, move)
		var move_text := _build_move_used_text(actor, move)
		if not move_text.is_empty():
			queue_text_event(move_text, false, {
				"source": "turn_resolution",
				"actor": actor.duplicate(true),
				"target": target.duplicate(true),
				"move": move.duplicate(true),
			})
		queue_animation_event(EVENT_PLAY_ANIMATION, {
			"move_name": _move_event_name(move),
			"is_player_move": _is_player_actor(command, actor),
			"param": 0,
			"animation": _sanitize_dictionary(command.get("animation", {}), {}),
			"move": move.duplicate(true),
			"actor": actor.duplicate(true),
			"target": target.duplicate(true),
		})
	return resolution

func complete_resolution(resolution_summary: String = "") -> void:
	var resolved := consume_pending_command()
	if resolved.is_empty() and not last_resolved_command.is_empty():
		resolved = last_resolved_command.duplicate(true)
	if not resolved.is_empty():
		last_resolved_command = resolved.duplicate(true)
	if not resolution_summary.is_empty():
		_push_log(resolution_summary)
	queue_resolution_event({
		"type": "turn_resolution_complete",
		"turn_number": turn_number,
		"phase": turn_phase,
		"command": resolved.duplicate(true),
		"summary": _coerce_string(resolution_summary, ""),
		"resolution": last_turn_resolution.duplicate(true),
	})
	clear_prompt_gate()
	if battle_finished:
		set_phase(PHASE_COMPLETE)
	else:
		set_phase(PHASE_POST_TURN)

func queue_resolution_event(event: Dictionary) -> Dictionary:
	var normalized := _sanitize_dictionary(event, {})
	if normalized.is_empty():
		return {}
	resolution_event_sequence += 1
	if not normalized.has("sequence"):
		normalized["sequence"] = resolution_event_sequence
	if not normalized.has("turn_number"):
		normalized["turn_number"] = turn_number
	if not normalized.has("phase"):
		normalized["phase"] = turn_phase
	if not normalized.has("frame_counter"):
		normalized["frame_counter"] = frame_counter
	if not normalized.has("fixed_step_count"):
		normalized["fixed_step_count"] = fixed_step_count
	resolution_events.append(normalized)
	_bump_revision()
	var event_kind := _coerce_string(normalized.get("type", normalized.get("kind", "event")), "event")
	_push_log("resolution event: %s" % event_kind)
	return normalized.duplicate(true)

func queue_battle_ui_event(event_name: String, data: Dictionary = {}, defer_until_animation: bool = false) -> Dictionary:
	var normalized_name := _coerce_string(event_name, "").strip_edges()
	if normalized_name.is_empty():
		return {}
	var event := queue_resolution_event({
		"type": normalized_name,
		"kind": "battle_ui_event",
		"name": normalized_name,
		"data": _sanitize_dictionary(data, {}),
	})
	if event.is_empty():
		return {}
	last_battle_event = event.duplicate(true)
	_apply_battle_ui_event_gate(event, defer_until_animation)
	return event.duplicate(true)

func queue_text_event(text: String, wait_for_animation: bool = false, data: Dictionary = {}) -> Dictionary:
	var normalized_text := _coerce_string(text, "").strip_edges()
	if normalized_text.is_empty():
		return {}
	var payload := _sanitize_dictionary(data, {})
	payload["text"] = normalized_text
	payload["wait_for_animation"] = bool(wait_for_animation)
	return queue_battle_ui_event(EVENT_SHOW_TEXT, payload, wait_for_animation)

func queue_animation_event(event_name: String, data: Dictionary = {}) -> Dictionary:
	return queue_battle_ui_event(event_name, data, true)

func consume_resolution_events() -> Array:
	var events: Array = get_resolution_events()
	if not resolution_events.is_empty():
		resolution_events = []
		_bump_revision()
	return events

func advance_phase() -> String:
	match turn_phase:
		PHASE_SETUP:
			set_phase(PHASE_INTRO)
		PHASE_INTRO:
			open_turn_prompt()
		PHASE_TURN_PROMPT:
			set_phase(PHASE_RESOLUTION)
		PHASE_RESOLUTION:
			set_phase(PHASE_POST_TURN)
		PHASE_POST_TURN:
			advance_turn(SIDE_PLAYER)
			open_turn_prompt()
		PHASE_COMPLETE:
			pass
		_:
			set_phase(PHASE_SETUP)
	return turn_phase

func set_waiting_for_input(value: bool) -> void:
	manual_wait_override = bool(value)
	_sync_ui_state()
	_bump_revision()

func set_fast_animation_request(value: bool) -> void:
	fast_animation_request = bool(value)
	_bump_revision()

func set_fast_text_request(value: bool) -> void:
	fast_text_request = bool(value)
	_bump_revision()

func sync_ui_state() -> void:
	_sync_ui_state()

func describe_phase() -> String:
	match turn_phase:
		PHASE_SETUP:
			return "SETUP"
		PHASE_INTRO:
			return "INTRO"
		PHASE_TURN_PROMPT:
			return "TURN PROMPT"
		PHASE_RESOLUTION:
			return "RESOLUTION"
		PHASE_POST_TURN:
			return "POST TURN"
		PHASE_COMPLETE:
			return "COMPLETE"
		_:
			return str(turn_phase).to_upper()

func describe_prompt_gate() -> String:
	var parts: Array[String] = []
	parts.append("prompt gate %s" % ("open" if prompt_gate_active else "closed"))
	if not prompt_kind.is_empty():
		parts.append(prompt_kind)
	if not prompt_message.is_empty():
		parts.append(prompt_message)
	if not prompt_gate_reason.is_empty():
		parts.append(prompt_gate_reason)
	return " | ".join(parts)

func describe_queue_flow() -> String:
	var pending_label := "none"
	if not pending_command.is_empty():
		pending_label = _command_label(pending_command)
	var resolved_label := "none"
	if not last_resolved_command.is_empty():
		resolved_label = _command_label(last_resolved_command)
	var latest_event := "none"
	if not resolution_events.is_empty():
		latest_event = _event_label(resolution_events[resolution_events.size() - 1])
	return "queued=%d pending=%s resolved=%s events=%d latest=%s" % [
		queued_commands.size(),
		pending_label,
		resolved_label,
		resolution_events.size(),
		latest_event,
	]

func describe_recent_history(limit: int = 4) -> String:
	if phase_history.is_empty():
		return "none"
	var recent_limit: int = max(1, limit)
	var start_index: int = max(0, phase_history.size() - recent_limit)
	return _recent_phase_history_from_index(start_index)

func describe_recent_logs(limit: int = 2) -> String:
	if log_lines.is_empty():
		return "none"
	var recent_limit: int = max(1, limit)
	var start_index: int = max(0, log_lines.size() - recent_limit)
	var sample: Array[String] = log_lines.slice(start_index, log_lines.size())
	return " | ".join(sample)

func hud_lines() -> Array[String]:
	var lines: Array[String] = []
	lines.append("battle: %s [%s]" % [_coerce_string(battle_label, "battle shell"), _coerce_string(battle_kind, "wild")])
	lines.append("turn: %d  phase: %s  side: %s" % [turn_number, describe_phase(), _coerce_string(active_side, SIDE_NONE)])
	lines.append("prompt: %s  locked: %s" % [str(prompt_gate_active).to_lower(), str(prompt_locked).to_lower()])
	lines.append("queue: %d command(s)  resolution events: %d" % [queued_commands.size(), resolution_events.size()])
	if not prompt_kind.is_empty() or not prompt_message.is_empty():
		lines.append("prompt detail: %s | %s" % [prompt_kind, prompt_message])
	if not prompt_gate_reason.is_empty():
		lines.append("prompt reason: %s" % prompt_gate_reason)
	if not pending_command.is_empty():
		lines.append("pending command: %s" % _command_label(pending_command))
	if not last_resolved_command.is_empty():
		lines.append("resolved: %s" % _command_label(last_resolved_command))
	if not resolution_events.is_empty():
		lines.append("latest event: %s" % _event_label(resolution_events[resolution_events.size() - 1]))
	if not phase_history.is_empty():
		lines.append("phase history: %s" % _recent_phase_history())
	if battle_finished:
		lines.append("result: %s" % battle_result)
	if not selected_player_payload.is_empty():
		lines.append("player payload: %s" % _payload_label(selected_player_payload))
	if not selected_opponent_payload.is_empty():
		lines.append("opponent payload: %s" % _payload_label(selected_opponent_payload))
	return lines

func debug_text() -> String:
	return "\n".join(hud_lines())

func get_state() -> Dictionary:
	return to_dictionary()

func from_state(data: Dictionary) -> void:
	if data.is_empty():
		reset()
		return
	from_dictionary(data)

func to_dictionary() -> Dictionary:
	var serialized_queued_commands: Array = _sanitize_array(queued_commands, [])
	var serialized_pending_command: Dictionary = _sanitize_dictionary(pending_command, {})
	if not serialized_queued_commands.is_empty():
		var queued_head: Variant = serialized_queued_commands[0]
		if queued_head is Dictionary:
			serialized_pending_command = Dictionary(queued_head).duplicate(true)
		else:
			serialized_queued_commands = []
	elif not serialized_pending_command.is_empty():
		serialized_queued_commands = [serialized_pending_command.duplicate(true)]
	return {
		"battle_id": battle_id,
		"battle_kind": battle_kind,
		"battle_label": battle_label,
		"turn_phase": turn_phase,
		"turn_number": turn_number,
		"active_side": active_side,
		"prompt_gate_active": prompt_gate_active,
		"prompt_gate_reason": prompt_gate_reason,
		"prompt_kind": prompt_kind,
		"prompt_message": prompt_message,
		"prompt_locked": prompt_locked,
		"queued_commands": serialized_queued_commands,
		"pending_command": serialized_pending_command,
		"last_resolved_command": _sanitize_dictionary(last_resolved_command, {}),
		"last_turn_resolution": _sanitize_dictionary(last_turn_resolution, {}),
		"resolution_events": _sanitize_array(resolution_events, []),
		"resolution_event_sequence": resolution_event_sequence,
		"pending_animation_events": _sanitize_array(pending_animation_events, []),
		"last_battle_event": _sanitize_dictionary(last_battle_event, {}),
		"active_text_event": _sanitize_dictionary(active_text_event, {}),
		"active_animation_event": _sanitize_dictionary(active_animation_event, {}),
		"trainer_sprite_state": _sanitize_dictionary(trainer_sprite_state, {}),
		"battle_finished": battle_finished,
		"battle_result": battle_result,
		"battle_result_state": _sanitize_dictionary(battle_result_state, _default_battle_result_state()),
		"frame_counter": frame_counter,
		"fixed_step_count": fixed_step_count,
		"state_revision": state_revision,
		"waiting_for_input": waiting_for_input,
		"manual_wait_override": manual_wait_override,
		"ui_phase": ui_phase,
		"dialogue_wait_gate_active": dialogue_wait_gate_active,
		"fast_animation_request": fast_animation_request,
		"fast_text_request": fast_text_request,
		"battle_context": _sanitize_dictionary(battle_context, {}),
		"asset_summary": _sanitize_dictionary(asset_summary, {}),
		"selected_player_payload": _sanitize_dictionary(selected_player_payload, {}),
		"selected_opponent_payload": _sanitize_dictionary(selected_opponent_payload, {}),
		"phase_history": _sanitize_string_array(phase_history, [PHASE_SETUP]),
		"log_lines": _sanitize_string_array(log_lines, ["battle shell ready"]),
	}

func from_dictionary(data: Variant) -> bool:
	if typeof(data) != TYPE_DICTIONARY:
		return false
	reset()
	var source: Dictionary = data
	battle_id = _coerce_string(source.get("battle_id", battle_id), battle_id)
	battle_kind = _coerce_string(source.get("battle_kind", battle_kind), battle_kind)
	battle_label = _coerce_string(source.get("battle_label", battle_label), battle_label)
	turn_phase = _normalize_phase(_coerce_string(source.get("turn_phase", turn_phase), turn_phase))
	turn_number = max(0, _coerce_int(source.get("turn_number", 0), 0))
	active_side = _normalize_side(_coerce_string(source.get("active_side", active_side), active_side))
	prompt_gate_active = _coerce_bool(source.get("prompt_gate_active", false), false)
	prompt_gate_reason = _coerce_string(source.get("prompt_gate_reason", ""), "")
	prompt_kind = _coerce_string(source.get("prompt_kind", ""), "")
	prompt_message = _coerce_string(source.get("prompt_message", ""), "")
	prompt_locked = _coerce_bool(source.get("prompt_locked", false), false)
	queued_commands = _sanitize_array(source.get("queued_commands", []), [])
	pending_command = _sanitize_dictionary(source.get("pending_command", {}), {})
	if not queued_commands.is_empty():
		var queued_head: Variant = queued_commands[0]
		if queued_head is Dictionary:
			pending_command = Dictionary(queued_head).duplicate(true)
		else:
			queued_commands = []
	elif not pending_command.is_empty():
		queued_commands = [pending_command.duplicate(true)]
	last_resolved_command = _sanitize_dictionary(source.get("last_resolved_command", {}), {})
	last_turn_resolution = _sanitize_dictionary(source.get("last_turn_resolution", {}), {})
	resolution_events = _sanitize_array(source.get("resolution_events", []), [])
	resolution_event_sequence = max(0, _coerce_int(source.get("resolution_event_sequence", _max_event_sequence(resolution_events)), 0))
	pending_animation_events = _sanitize_array(source.get("pending_animation_events", []), [])
	last_battle_event = _sanitize_dictionary(source.get("last_battle_event", {}), {})
	active_text_event = _sanitize_dictionary(source.get("active_text_event", {}), {})
	active_animation_event = _sanitize_dictionary(source.get("active_animation_event", {}), {})
	trainer_sprite_state = _sanitize_dictionary(source.get("trainer_sprite_state", {}), {})
	battle_finished = _coerce_bool(source.get("battle_finished", false), false)
	battle_result = _coerce_string(source.get("battle_result", ""), "")
	battle_result_state = _sanitize_dictionary(source.get("battle_result_state", {}), _default_battle_result_state())
	if battle_finished:
		battle_result_state["finished"] = true
	if not battle_result.is_empty():
		battle_result_state["result"] = battle_result
	frame_counter = max(0, _coerce_int(source.get("frame_counter", 0), 0))
	fixed_step_count = max(0, _coerce_int(source.get("fixed_step_count", 0), 0))
	state_revision = max(0, _coerce_int(source.get("state_revision", 0), 0))
	waiting_for_input = _coerce_bool(source.get("waiting_for_input", false), false)
	manual_wait_override = _coerce_bool(source.get("manual_wait_override", false), false)
	ui_phase = _coerce_string(source.get("ui_phase", UI_PHASE_INACTIVE), UI_PHASE_INACTIVE)
	dialogue_wait_gate_active = _coerce_bool(source.get("dialogue_wait_gate_active", false), false)
	fast_animation_request = _coerce_bool(source.get("fast_animation_request", false), false)
	fast_text_request = _coerce_bool(source.get("fast_text_request", false), false)
	battle_context = _sanitize_dictionary(source.get("battle_context", {}), {})
	asset_summary = _sanitize_dictionary(source.get("asset_summary", {}), {})
	selected_player_payload = _sanitize_dictionary(source.get("selected_player_payload", {}), {})
	selected_opponent_payload = _sanitize_dictionary(source.get("selected_opponent_payload", {}), {})
	phase_history = _sanitize_string_array(source.get("phase_history", [PHASE_SETUP]), [PHASE_SETUP])
	log_lines = _sanitize_string_array(source.get("log_lines", []), ["battle shell ready"])
	if log_lines.is_empty():
		log_lines = ["battle shell ready"]
	_sync_ui_state()
	_record_phase(turn_phase)
	return true

func _record_phase(phase: String) -> void:
	if phase_history.is_empty() or phase_history[phase_history.size() - 1] != phase:
		phase_history.append(phase)

func _sync_ui_state() -> void:
	dialogue_wait_gate_active = prompt_gate_active and not battle_finished
	waiting_for_input = manual_wait_override or dialogue_wait_gate_active or prompt_locked
	if battle_finished:
		ui_phase = UI_PHASE_COMPLETE
	elif prompt_gate_active or prompt_locked or waiting_for_input:
		ui_phase = UI_PHASE_DIALOGUE
	elif turn_phase == PHASE_SETUP:
		ui_phase = UI_PHASE_INACTIVE
	else:
		ui_phase = UI_PHASE_MENU

func _recent_phase_history() -> String:
	return _recent_phase_history_from_index(max(0, phase_history.size() - 4))

func _recent_phase_history_from_index(start_index: int) -> String:
	var sample: Array[String] = phase_history.slice(start_index, phase_history.size())
	return " -> ".join(sample).to_upper()

func _event_label(event: Dictionary) -> String:
	if event.has("type"):
		return _coerce_string(event.get("type", ""), "event")
	if event.has("kind"):
		return _coerce_string(event.get("kind", ""), "event")
	return "event"

func _bump_revision() -> void:
	state_revision += 1

func _push_log(message: String) -> void:
	var entry := _coerce_string(message, "")
	if entry.is_empty():
		return
	log_lines.append(entry)
	while log_lines.size() > DEFAULT_LOG_LIMIT:
		log_lines.pop_front()

func _apply_battle_ui_event_gate(event: Dictionary, defer_until_animation: bool) -> void:
	var event_name := _coerce_string(event.get("name", event.get("type", "")), "")
	var data := _sanitize_dictionary(event.get("data", {}), {})
	if _is_text_gate_event(event_name):
		active_text_event = event.duplicate(true)
		if event_name == EVENT_CLOSE_TEXT:
			active_text_event = {}
			clear_prompt_gate()
		else:
			var text := _coerce_string(data.get("text", data.get("message", "")), "")
			set_prompt_gate(true, "battle_text", text, true, event_name)
	if _is_animation_gate_event(event_name):
		active_animation_event = event.duplicate(true)
		if event_name == EVENT_SHOW_TRAINER_SPRITES or event_name == EVENT_TRIGGER_TRAINER_EXIT:
			trainer_sprite_state[event_name] = data.duplicate(true)
		if defer_until_animation:
			pending_animation_events.append(event.duplicate(true))
			_sync_ui_state()
			_bump_revision()

func _is_text_gate_event(event_name: String) -> bool:
	return event_name in [
		EVENT_SHOW_TEXT,
		EVENT_OPEN_TEXT,
		EVENT_CLOSE_TEXT,
		EVENT_WAIT_FOR_INPUT,
		EVENT_PROMPT_YES_NO,
		EVENT_NICKNAME_PROMPT,
	]

func _is_animation_gate_event(event_name: String) -> bool:
	return event_name in [
		EVENT_PLAY_ANIMATION,
		EVENT_FRONTPIC_ANIMATION,
		EVENT_SHOW_TRAINER_SPRITES,
		EVENT_TRIGGER_TRAINER_EXIT,
	]

func _prompt_summary() -> String:
	var parts: Array[String] = []
	parts.append("prompt gate %s" % ("open" if prompt_gate_active else "closed"))
	if not prompt_kind.is_empty():
		parts.append(prompt_kind)
	if not prompt_message.is_empty():
		parts.append(prompt_message)
	if not prompt_gate_reason.is_empty():
		parts.append(prompt_gate_reason)
	return " | ".join(parts)

func _command_label(command: Dictionary) -> String:
	if command.has("label"):
		return _coerce_string(command.get("label", ""), "command")
	if command.has("kind"):
		return _coerce_string(command.get("kind", ""), "command")
	return "command"

func _payload_label(payload: Dictionary) -> String:
	for key in ["label", "name", "trainer_id", "id", "species"]:
		if payload.has(key):
			var value: Variant = payload.get(key)
			if value is Dictionary:
				var nested: Dictionary = value
				if nested.has("id"):
					return _coerce_string(nested.get("id", ""), "payload")
				if nested.has("name"):
					return _coerce_string(nested.get("name", ""), "payload")
			return _coerce_string(value, "payload")
	return "payload"

func _build_turn_resolution(command: Dictionary, actor: Dictionary, target: Dictionary, move: Dictionary) -> Dictionary:
	var valid := not command.is_empty()
	var reason := "ok"
	if move.is_empty():
		valid = false
		reason = "missing_move_payload"
	var move_power: int = max(0, _coerce_int(move.get("power", 0), 0))
	var move_accuracy: int = max(0, _coerce_int(move.get("accuracy", 0), 0))
	var move_pp: int = max(0, _coerce_int(move.get("pp", 0), 0))
	var actor_species: Dictionary = _species_payload(actor)
	var target_species: Dictionary = _species_payload(target)
	var actor_stats: Dictionary = _stat_block(actor_species)
	var target_stats: Dictionary = _stat_block(target_species)
	var actor_side: String = _resolve_side(actor, command.get("actor_side", command.get("side", SIDE_PLAYER)))
	var target_side: String = _resolve_target_side(command, actor_side)
	var move_type: String = _normalize_type_name(_coerce_string(move.get("type", ""), "NORMAL"))
	var turn_order: Array = _resolve_turn_order(command, actor_side, target_side, actor_stats, target_stats, move)
	var damage_floor := 0
	if valid and move_power > 0:
		damage_floor = _calculate_base_damage(move_power, actor_stats, target_stats, move_type, actor, target, move, command)
	return {
		"valid": valid,
		"reason": reason,
		"turn_number": turn_number,
		"active_side": active_side,
		"actor_side": actor_side,
		"target_side": target_side,
		"turn_order": turn_order,
		"command": command.duplicate(true),
		"actor": actor.duplicate(true),
		"target": target.duplicate(true),
		"move": move.duplicate(true),
		"move_id": _payload_identifier(move),
		"move_type": move_type,
		"move_power": move_power,
		"move_accuracy": move_accuracy,
		"move_pp": move_pp,
		"actor_species_id": _payload_identifier(actor_species),
		"target_species_id": _payload_identifier(target_species),
		"actor_hp_before": _pokemon_hp(actor),
		"target_hp_before": _pokemon_hp(target),
		"actor_pp_before": _move_current_pp(actor, move),
		"damage_floor": damage_floor,
		"damage": damage_floor,
		"recoil_damage": 0,
		"hit": true,
		"status_result": {},
		"result": RESULT_ONGOING,
		"event_order": [
			"turn_resolution_started",
			"command_validated",
			"turn_order_determined",
			"move_selected",
			"pp_consumed",
			"accuracy_checked",
			"damage_calculated",
			"hp_changed",
			"recoil_applied",
			"status_applied",
			"battle_result",
			"turn_resolution_complete",
		],
	}

func _build_move_used_text(actor: Dictionary, move: Dictionary) -> String:
	if move.is_empty():
		return ""
	var actor_label := _battle_actor_label(actor)
	var move_label := _battle_move_label(move)
	if actor_label.is_empty() or move_label.is_empty():
		return ""
	return "%s used %s!" % [actor_label, move_label]

func _battle_actor_label(actor: Dictionary) -> String:
	for key in ["nickname", "display_name", "name", "label", "id"]:
		var value := _coerce_string(actor.get(key, ""), "").strip_edges()
		if not value.is_empty():
			return value
	var species := _species_payload(actor)
	if not species.is_empty():
		for key in ["name", "id", "species_id"]:
			var species_value := _coerce_string(species.get(key, ""), "").strip_edges()
			if not species_value.is_empty():
				return _display_identifier(species_value)
	return ""

func _battle_move_label(move: Dictionary) -> String:
	for key in ["display_name", "name", "label", "id", "move_id", "kind"]:
		var value := _coerce_string(move.get(key, ""), "").strip_edges()
		if not value.is_empty():
			return _display_identifier(value)
	return ""

func _move_event_name(move: Dictionary) -> String:
	var identifier := _payload_identifier(move)
	if not identifier.is_empty():
		return identifier
	return _battle_move_label(move)

func _display_identifier(value: String) -> String:
	var normalized := value.strip_edges()
	if normalized.is_empty():
		return ""
	normalized = normalized.replace("_", " ")
	normalized = normalized.replace("-", " ")
	return normalized

func _is_player_actor(command: Dictionary, actor: Dictionary) -> bool:
	var side := _normalize_side(_coerce_string(command.get("side", command.get("actor_side", active_side)), active_side))
	if side == SIDE_PLAYER:
		return true
	if side == SIDE_ENEMY:
		return false
	if selected_player_payload.is_empty():
		return active_side == SIDE_PLAYER
	var actor_id := _payload_identifier(actor)
	var player_id := _payload_identifier(selected_player_payload)
	return not actor_id.is_empty() and actor_id == player_id

func _build_animation_hooks(resolution: Dictionary) -> Dictionary:
	return {
		"event_order": _sanitize_array(resolution.get("event_order", []), []),
		"move_key": _coerce_string(resolution.get("move_id", ""), ""),
		"actor_key": _coerce_string(resolution.get("actor_species_id", ""), ""),
		"target_key": _coerce_string(resolution.get("target_species_id", ""), ""),
	}

func _resolve_side(payload: Dictionary, fallback: String) -> String:
	var candidates: Array[String] = []
	for key in ["side", "battle_side", "actor_side", "turn_side"]:
		candidates.append(_coerce_string(payload.get(key, ""), ""))
	candidates.append(_coerce_string(fallback, SIDE_NONE))
	for candidate in candidates:
		var normalized: String = _normalize_side(candidate)
		if normalized != SIDE_NONE:
			return normalized
	return SIDE_NONE

func _resolve_target_side(command: Dictionary, actor_side: String) -> String:
	var explicit: String = _resolve_side(command, SIDE_NONE)
	if explicit == SIDE_PLAYER or explicit == SIDE_ENEMY:
		if explicit != actor_side:
			return explicit
	if actor_side == SIDE_PLAYER:
		return SIDE_ENEMY
	if actor_side == SIDE_ENEMY:
		return SIDE_PLAYER
	return SIDE_ENEMY

func _resolve_turn_order(command: Dictionary, actor_side: String, target_side: String, actor_stats: Dictionary, target_stats: Dictionary, move: Dictionary) -> Array[String]:
	var explicit: Variant = command.get("turn_order", command.get("order", []))
	if typeof(explicit) == TYPE_ARRAY and not Array(explicit).is_empty():
		var normalized_order: Array[String] = []
		for entry in Array(explicit):
			var side: String = _normalize_side(_coerce_string(entry, ""))
			if side == SIDE_PLAYER or side == SIDE_ENEMY:
				normalized_order.append(side)
		if not normalized_order.is_empty():
			return normalized_order
	var move_priority: int = _coerce_int(move.get("priority", command.get("priority", 0)), 0)
	var action_type: String = _coerce_string(command.get("action_type", command.get("type", "")), "").to_upper()
	if action_type in ["ITEM", "SWITCH", "RUN"]:
		move_priority = max(move_priority, 10)
	if move_priority > 0:
		return _turn_order_pair(actor_side, target_side)
	if move_priority < 0:
		return _turn_order_pair(target_side, actor_side)
	var actor_speed: int = max(1, _battle_speed(actor_stats, command.get("actor", {})))
	var target_speed: int = max(1, _battle_speed(target_stats, command.get("target", {})))
	if actor_speed > target_speed:
		return _turn_order_pair(actor_side, target_side)
	if target_speed > actor_speed:
		return _turn_order_pair(target_side, actor_side)
	return _turn_order_pair(actor_side, target_side) if _resolution_random_value(command, 0.0) < 0.5 else _turn_order_pair(target_side, actor_side)

func _turn_order_pair(first_side: String, second_side: String) -> Array[String]:
	var order: Array[String] = []
	order.append(first_side)
	order.append(second_side)
	return order

func _battle_speed(stats: Dictionary, payload: Variant) -> int:
	var direct_payload: Dictionary = _sanitize_dictionary(payload, {})
	var direct: int = _coerce_int(direct_payload.get("speed", direct_payload.get("spe", 0)), 0)
	if direct > 0:
		return direct
	return max(1, _coerce_int(stats.get("speed", 1), 1))

func _normalize_type_name(value: String) -> String:
	var normalized: String = _coerce_string(value, "NORMAL").strip_edges().to_upper()
	if normalized.is_empty():
		return "NORMAL"
	return normalized.replace(" ", "_").replace("-", "_")

func _calculate_base_damage(move_power: int, actor_stats: Dictionary, target_stats: Dictionary, move_type: String, actor: Dictionary, target: Dictionary, move: Dictionary, command: Dictionary = {}) -> int:
	if move_power <= 0:
		return 0
	var actor_level: int = max(1, _coerce_int(actor.get("level", actor.get("lv", actor.get("level_no", 1))), 1))
	var attack_key: String = "attack" if _is_physical_type(move_type) else "special_attack"
	var defense_key: String = "defense" if _is_physical_type(move_type) else "special_defense"
	var attack_value: int = max(1, _coerce_int(actor.get(attack_key, actor_stats.get(attack_key, 1)), 1))
	var defense_value: int = max(1, _coerce_int(target.get(defense_key, target_stats.get(defense_key, 1)), 1))
	var base: int = int(floor((((2.0 * float(actor_level)) / 5.0) + 2.0) * float(move_power) * float(attack_value) / float(defense_value) / 50.0)) + 2
	var modifier: float = 1.0
	if _actor_has_type(actor, move_type):
		modifier *= 1.5
	modifier *= _type_effectiveness_multiplier(move_type, _pokemon_types(target))
	if _coerce_string(actor.get("status", ""), "") == "BURN" and _is_physical_type(move_type):
		modifier *= 0.5
	modifier *= _resolution_random_value(command, 1.0)
	return max(1, int(floor(float(base) * modifier)))

func _pokemon_hp(payload: Dictionary) -> int:
	return max(0, _coerce_int(payload.get("hp", payload.get("current_hp", payload.get("currentHp", 0))), 0))

func _move_current_pp(payload: Dictionary, move: Dictionary) -> int:
	if move.has("current_pp"):
		return max(0, _coerce_int(move.get("current_pp", 0), 0))
	if move.has("pp"):
		return max(0, _coerce_int(move.get("pp", 0), 0))
	var move_id: String = _payload_identifier(move)
	for move_slot in _sanitize_array(payload.get("moves", []), []):
		var slot: Dictionary = _sanitize_dictionary(move_slot, {})
		if _payload_identifier(slot) == move_id:
			return max(0, _coerce_int(slot.get("current_pp", slot.get("pp", 0)), 0))
	return 0

func _move_hits(move: Dictionary, accuracy_roll: float) -> bool:
	var accuracy: int = max(0, _coerce_int(move.get("accuracy", 100), 100))
	if accuracy <= 0:
		return true
	return accuracy_roll < (float(accuracy) / 100.0)

func _resolution_random_value(command: Dictionary, default_value: float) -> float:
	if command.has("predefined_random_value"):
		return clampf(float(command.get("predefined_random_value", default_value)), 0.0, 1.0)
	if battle_context.has("predefined_random_value"):
		return clampf(float(battle_context.get("predefined_random_value", default_value)), 0.0, 1.0)
	if command.has("random_value"):
		return clampf(float(command.get("random_value", default_value)), 0.0, 1.0)
	if battle_context.has("random_value"):
		return clampf(float(battle_context.get("random_value", default_value)), 0.0, 1.0)
	return clampf(default_value, 0.0, 1.0)

func _status_allows_action(payload: Dictionary, resolution: Dictionary, command: Dictionary) -> bool:
	var status: String = _coerce_string(payload.get("status", ""), "").to_upper()
	if status == "SLEEP":
		var sleep_turns: int = max(0, _coerce_int(payload.get("sleep_turns", 0), 0))
		if sleep_turns > 0:
			sleep_turns -= 1
			payload["sleep_turns"] = sleep_turns
			if sleep_turns <= 0:
				payload["status"] = ""
				queue_text_event("%s woke up!" % _battle_actor_label(payload))
				return true
			queue_text_event("%s is fast asleep!" % _battle_actor_label(payload))
			return false
	if status == "FREEZE":
		queue_text_event("%s is frozen solid!" % _battle_actor_label(payload))
		return false
	if status == "PARALYSIS":
		var paralysis_roll: float = _resolution_random_value(command, 0.0)
		if paralysis_roll < 0.25:
			queue_text_event("%s is fully paralyzed!" % _battle_actor_label(payload))
			return false
	return true

func _apply_secondary_effects(command: Dictionary, resolution: Dictionary, actor: Dictionary, target: Dictionary, move: Dictionary) -> Dictionary:
	var effect: String = _normalize_type_name(_coerce_string(move.get("effect", ""), "NORMAL_HIT"))
	var effect_chance: int = max(0, _coerce_int(move.get("effect_chance", 0), 0))
	var should_apply: bool = effect_chance == 0 and effect != "NORMAL_HIT" and effect != "NONE"
	if effect_chance > 0:
		should_apply = _resolution_random_value(command, 0.0) < (float(effect_chance) / 100.0)
	var result: Dictionary = {}
	if not should_apply:
		return result
	match effect:
		"RECOIL_HIT":
			var recoil_damage: int = _apply_recoil_effect(actor, resolution)
			if recoil_damage > 0:
				result = {"recoil_damage": recoil_damage}
		"PARALYZE", "PARALYZE_HIT":
			if _apply_status(target, "PARALYSIS"):
				result = {"status": "PARALYSIS"}
		"BURN_HIT":
			if _apply_status(target, "BURN"):
				result = {"status": "BURN"}
		"POISON", "POISON_HIT":
			if _apply_status(target, "POISON"):
				result = {"status": "POISON"}
		"SLEEP":
			if _apply_status(target, "SLEEP"):
				result = {"status": "SLEEP"}
		"CONFUSE", "CONFUSE_HIT":
			if _apply_confusion(target):
				result = {"status": "CONFUSION"}
		"ATTACK_DOWN", "DEFENSE_DOWN", "SPEED_DOWN", "SP_ATTACK_DOWN", "SP_DEFENSE_DOWN", "ALL_DOWN":
			var stat_name: String = _normalize_stat_name(_coerce_string(move.get("stat", ""), "ATTACK"))
			var amount: int = _coerce_int(move.get("amount", -1), -1)
			if amount == 0:
				amount = -1
			if _modify_stat_stage(target, stat_name, amount):
				result = {"stat": stat_name, "amount": amount}
		_:
			pass
	return result

func _apply_status(payload: Dictionary, status: String) -> bool:
	var normalized: String = _coerce_string(status, "").to_upper()
	if normalized.is_empty():
		return false
	payload["status"] = normalized
	if normalized == "SLEEP" and not payload.has("sleep_turns"):
		payload["sleep_turns"] = 2
	if normalized == "CONFUSION" and not payload.has("confusion_turns"):
		payload["confusion_turns"] = 3
	return true

func _apply_confusion(payload: Dictionary) -> bool:
	payload["status"] = "CONFUSION"
	payload["confusion_turns"] = max(1, _coerce_int(payload.get("confusion_turns", 3), 3))
	return true

func _modify_stat_stage(payload: Dictionary, stat_name: String, amount: int) -> bool:
	var boosts: Dictionary = _sanitize_dictionary(payload.get("stat_boosts", {}), {})
	var normalized: String = _normalize_stat_name(stat_name)
	var current: int = _coerce_int(boosts.get(normalized, 0), 0)
	var updated: int = clampi(current + amount, -6, 6)
	if updated == current:
		return false
	boosts[normalized] = updated
	payload["stat_boosts"] = boosts
	return true

func _apply_status_effect_name(payload: Dictionary, status_name: String) -> bool:
	return _apply_status(payload, status_name)

func _apply_recoil_effect(actor: Dictionary, resolution: Dictionary) -> int:
	var damage: int = max(0, _coerce_int(resolution.get("damage", 0), 0))
	if damage <= 0:
		return 0
	var recoil_damage: int = max(1, int(floor(float(damage) / 4.0)))
	var before_hp: int = _pokemon_hp(actor)
	if before_hp <= 0:
		return 0
	var after_hp: int = max(0, before_hp - recoil_damage)
	_set_pokemon_hp(actor, after_hp)
	queue_resolution_event({
		"type": "recoil_applied",
		"kind": "recoil",
		"actor_side": _coerce_string(resolution.get("actor_side", SIDE_NONE), SIDE_NONE),
		"actor": actor.duplicate(true),
		"hp_before": before_hp,
		"hp_after": after_hp,
		"damage": recoil_damage,
	})
	queue_text_event("%s was hit with recoil!" % _battle_actor_label(actor))
	if after_hp <= 0:
		queue_resolution_event({
			"type": "pokemon_fainted",
			"kind": "faint",
			"side": _coerce_string(resolution.get("actor_side", SIDE_NONE), SIDE_NONE),
			"pokemon": actor.duplicate(true),
		})
		if not battle_finished and _coerce_string(resolution.get("actor_side", SIDE_NONE), SIDE_NONE) == SIDE_PLAYER:
			_set_battle_result(RESULT_LOSS, {
				"winner": SIDE_ENEMY,
				"reason": "player_fainted_recoil",
				"turn_number": turn_number,
			})
	return recoil_damage

func _actor_has_type(payload: Dictionary, move_type: String) -> bool:
	var types: Array[String] = _pokemon_types(payload)
	for type_name in types:
		if type_name == move_type:
			return true
	return false

func _pokemon_types(payload: Dictionary) -> Array[String]:
	var types: Array[String] = []
	for key in ["type1", "type2", "type"]:
		var normalized := _normalize_type_name(_coerce_string(payload.get(key, ""), ""))
		if not normalized.is_empty() and not types.has(normalized):
			types.append(normalized)
	return types

func _type_effectiveness_multiplier(move_type: String, defender_types: Array[String]) -> float:
	var table: Dictionary = TYPE_EFFECTIVENESS.get(_normalize_type_name(move_type), {})
	var multiplier := 1.0
	for type_name in defender_types:
		multiplier *= float(table.get(_normalize_type_name(type_name), 1.0))
	return multiplier

func _is_physical_type(move_type: String) -> bool:
	return PHYSICAL_TYPES.has(_normalize_type_name(move_type))

func _normalize_stat_name(value: String) -> String:
	var normalized := _coerce_string(value, "").strip_edges().to_upper().replace(" ", "_")
	match normalized:
		"SPECIAL":
			return "special_attack"
		"SP_ATTACK", "SPECIAL_ATTACK":
			return "special_attack"
		"SP_DEFENSE", "SPECIAL_DEFENSE":
			return "special_defense"
		"ATTACK":
			return "attack"
		"DEFENSE":
			return "defense"
		"SPEED":
			return "speed"
		"ACCURACY":
			return "accuracy"
		"EVASION":
			return "evasion"
		_:
			return normalized.to_lower()

func _set_pokemon_hp(payload: Dictionary, value: int) -> void:
	payload["hp"] = max(0, value)
	if payload.has("current_hp"):
		payload["current_hp"] = max(0, value)

func _consume_move_pp(payload: Dictionary, move: Dictionary) -> int:
	var move_name := _move_identity(move)
	var current_pp := _move_current_pp(payload, move)
	if current_pp <= 0:
		return 0
	var remaining := current_pp - 1
	var moves: Variant = payload.get("moves", [])
	if typeof(moves) == TYPE_ARRAY and not move_name.is_empty():
		var updated_moves: Array = []
		for entry in Array(moves):
			if typeof(entry) != TYPE_DICTIONARY:
				updated_moves.append(entry)
				continue
			var move_entry: Dictionary = Dictionary(entry).duplicate(true)
			if _move_identity(move_entry) == move_name:
				move_entry["current_pp"] = remaining
				move_entry["pp"] = move_entry.get("pp", move.get("pp", remaining))
			updated_moves.append(move_entry)
		payload["moves"] = updated_moves
	return current_pp

func _move_identity(move: Dictionary) -> String:
	for key in ["id", "move_id", "name", "label", "kind"]:
		var value := _coerce_string(move.get(key, ""), "").strip_edges()
		if not value.is_empty():
			return _normalize_type_name(value)
	return ""

func _store_resolution_payloads(actor_side: String, actor_payload: Dictionary, target_side: String, target_payload: Dictionary) -> void:
	if actor_side == SIDE_PLAYER:
		selected_player_payload = actor_payload.duplicate(true)
	elif actor_side == SIDE_ENEMY:
		selected_opponent_payload = actor_payload.duplicate(true)
	if target_side == SIDE_PLAYER:
		selected_player_payload = target_payload.duplicate(true)
	elif target_side == SIDE_ENEMY:
		selected_opponent_payload = target_payload.duplicate(true)

func _set_battle_result(result: String, detail: Dictionary) -> void:
	battle_finished = true
	battle_result = _coerce_string(result, RESULT_ONGOING)
	battle_result_state = _sanitize_dictionary(detail, _default_battle_result_state())
	battle_result_state["result"] = battle_result
	battle_result_state["finished"] = true
	battle_result_state["turn_number"] = turn_number
	if not battle_result_state.has("reason"):
		battle_result_state["reason"] = ""
	queue_resolution_event({
		"type": "battle_result",
		"kind": "battle_result",
		"result": battle_result,
		"result_state": battle_result_state.duplicate(true),
	})
	_push_log("battle complete: %s" % battle_result)

func _apply_turn_resolution(command: Dictionary, resolution: Dictionary, actor: Dictionary, target: Dictionary, move: Dictionary) -> void:
	var actor_side: String = _coerce_string(resolution.get("actor_side", SIDE_PLAYER), SIDE_PLAYER)
	var target_side: String = _coerce_string(resolution.get("target_side", SIDE_ENEMY), SIDE_ENEMY)
	var move_type: String = _normalize_type_name(_coerce_string(resolution.get("move_type", move.get("type", "NORMAL")), "NORMAL"))
	var actor_payload: Dictionary = actor.duplicate(true)
	var target_payload: Dictionary = target.duplicate(true)

	if not _status_allows_action(actor_payload, resolution, command):
		resolution["valid"] = false
		resolution["reason"] = "status_blocked"
		resolution["result"] = "blocked"
		queue_resolution_event({
			"type": "turn_resolution_blocked",
			"kind": "status_blocked",
			"reason": "status_blocked",
			"actor_side": actor_side,
			"actor": actor_payload.duplicate(true),
			"move": move.duplicate(true),
		})
		_store_resolution_payloads(actor_side, actor_payload, target_side, target_payload)
		last_turn_resolution = resolution.duplicate(true)
		return

	var pp_before: int = _move_current_pp(actor_payload, move)
	if pp_before <= 0:
		resolution["valid"] = false
		resolution["reason"] = "no_pp"
		resolution["result"] = "blocked"
		queue_resolution_event({
			"type": "move_failed",
			"kind": "move_failed",
			"reason": "no_pp",
			"actor_side": actor_side,
			"actor": actor_payload.duplicate(true),
			"move": move.duplicate(true),
		})
		_store_resolution_payloads(actor_side, actor_payload, target_side, target_payload)
		last_turn_resolution = resolution.duplicate(true)
		return

	pp_before = _consume_move_pp(actor_payload, move)
	resolution["actor_pp_before"] = pp_before
	resolution["actor_pp_after"] = _move_current_pp(actor_payload, move)
	queue_resolution_event({
		"type": "pp_consumed",
		"kind": "pp",
		"actor_side": actor_side,
		"actor": actor_payload.duplicate(true),
		"move": move.duplicate(true),
		"pp_before": pp_before,
		"pp_after": resolution["actor_pp_after"],
	})

	var accuracy_roll: float = _resolution_random_value(command, 0.0)
	var hit: bool = _move_hits(move, accuracy_roll)
	resolution["accuracy_roll"] = accuracy_roll
	resolution["hit"] = hit
	queue_resolution_event({
		"type": "accuracy_checked",
		"kind": "accuracy",
		"actor_side": actor_side,
		"target_side": target_side,
		"move": move.duplicate(true),
		"roll": accuracy_roll,
		"accuracy": resolution["move_accuracy"],
		"hit": hit,
	})
	if not hit:
		resolution["result"] = "miss"
		queue_resolution_event({
			"type": "move_missed",
			"kind": "move_missed",
			"actor_side": actor_side,
			"target_side": target_side,
			"move": move.duplicate(true),
		})
		_store_resolution_payloads(actor_side, actor_payload, target_side, target_payload)
		last_turn_resolution = resolution.duplicate(true)
		return

	var damage: int = _calculate_base_damage(
		int(resolution.get("move_power", 0)),
		_stat_block(_species_payload(actor_payload)),
		_stat_block(_species_payload(target_payload)),
		move_type,
		actor_payload,
		target_payload,
		move,
		command
	)
	resolution["damage"] = damage
	queue_resolution_event({
		"type": "damage_calculated",
		"kind": "damage",
		"actor_side": actor_side,
		"target_side": target_side,
		"move": move.duplicate(true),
			"damage": damage,
		})

	var target_hp_before: int = _pokemon_hp(target_payload)
	if damage > 0:
		_set_pokemon_hp(target_payload, max(0, target_hp_before - damage))
		queue_resolution_event({
			"type": "hp_changed",
			"kind": "hp",
			"side": target_side,
			"pokemon": target_payload.duplicate(true),
			"hp_before": target_hp_before,
			"hp_after": _pokemon_hp(target_payload),
			"damage": damage,
		})
		if _pokemon_hp(target_payload) <= 0:
			queue_resolution_event({
				"type": "pokemon_fainted",
				"kind": "faint",
				"side": target_side,
				"pokemon": target_payload.duplicate(true),
			})
			if target_side == SIDE_ENEMY:
				_set_battle_result(RESULT_WIN, {
					"winner": SIDE_PLAYER,
					"reason": "enemy_fainted",
					"turn_number": turn_number,
				})
			elif target_side == SIDE_PLAYER:
				_set_battle_result(RESULT_LOSS, {
					"winner": SIDE_ENEMY,
					"reason": "player_fainted",
					"turn_number": turn_number,
				})

	var status_result: Dictionary = _apply_secondary_effects(command, resolution, actor_payload, target_payload, move)
	resolution["status_result"] = status_result.duplicate(true)
	if status_result.has("recoil_damage"):
		resolution["recoil_damage"] = int(status_result.get("recoil_damage", 0))
	if not status_result.is_empty():
		queue_resolution_event({
			"type": "status_applied",
			"kind": "status",
			"actor_side": actor_side,
			"target_side": target_side,
			"move": move.duplicate(true),
			"status_result": status_result.duplicate(true),
		})

	_store_resolution_payloads(actor_side, actor_payload, target_side, target_payload)
	if battle_finished:
		resolution["result"] = battle_result
		resolution["battle_result_state"] = battle_result_state.duplicate(true)
	last_turn_resolution = resolution.duplicate(true)
	if not battle_finished and int(_pokemon_hp(target_payload)) <= 0 and target_side == SIDE_PLAYER:
		# If the target side is the player and the battle did not already finish, treat it as a loss.
		_set_battle_result(RESULT_LOSS, {
			"winner": SIDE_ENEMY,
			"reason": "player_fainted",
			"turn_number": turn_number,
		})
	if battle_finished:
		resolution["result"] = battle_result
		resolution["battle_result_state"] = battle_result_state.duplicate(true)
	last_turn_resolution = resolution.duplicate(true)

func _animation_key_for_move(move: Dictionary) -> String:
	var move_id := _payload_identifier(move)
	if move_id.is_empty():
		return ""
	return "move:%s" % move_id

func _payload_identifier(payload: Dictionary) -> String:
	for key in ["id", "move_id", "species_id", "trainer_id", "name", "label", "kind"]:
		if payload.has(key):
			return _coerce_string(payload.get(key, ""), "")
	return ""

func _species_payload(payload: Dictionary) -> Dictionary:
	var species: Variant = payload.get("species", payload)
	if typeof(species) == TYPE_DICTIONARY:
		return Dictionary(species).duplicate(true)
	return {}

func _stat_block(species: Dictionary) -> Dictionary:
	var stats: Variant = species.get("base_stats", {})
	if typeof(stats) == TYPE_DICTIONARY:
		return Dictionary(stats).duplicate(true)
	return {}

func _default_battle_result_state() -> Dictionary:
	return {
		"finished": false,
		"result": RESULT_ONGOING,
		"turn_number": 0,
		"reason": "",
	}

func _max_event_sequence(events: Array) -> int:
	var highest := 0
	for event in events:
		if typeof(event) == TYPE_DICTIONARY:
			highest = max(highest, int(Dictionary(event).get("sequence", 0)))
	return highest

func _normalize_phase(phase: String) -> String:
	var normalized := _coerce_string(phase, PHASE_SETUP)
	match normalized:
		PHASE_SETUP, PHASE_INTRO, PHASE_TURN_PROMPT, PHASE_RESOLUTION, PHASE_POST_TURN, PHASE_COMPLETE:
			return normalized
		_:
			return PHASE_SETUP

func _normalize_side(side: String) -> String:
	var normalized := _coerce_string(side, SIDE_NONE)
	match normalized:
		SIDE_PLAYER, SIDE_ENEMY, SIDE_NONE:
			return normalized
		_:
			return SIDE_NONE

func _sanitize_dictionary(value: Variant, defaults: Dictionary) -> Dictionary:
	var result: Dictionary = defaults.duplicate(true)
	if typeof(value) != TYPE_DICTIONARY:
		return result
	var source: Dictionary = value
	for key in source.keys():
		var raw_value: Variant = source[key]
		if result.has(key):
			result[key] = _sanitize_value(raw_value, result[key])
		else:
			result[key] = _normalize_variant(raw_value)
	return result

func _sanitize_string_array(value: Variant, defaults: Array[String]) -> Array[String]:
	var result: Array[String] = []
	if typeof(value) != TYPE_ARRAY:
		return defaults.duplicate(true)
	var source: Array = value
	for entry in source:
		result.append(_coerce_string(entry, ""))
	if result.is_empty():
		return defaults.duplicate(true)
	return result

func _sanitize_value(value: Variant, template: Variant) -> Variant:
	match typeof(template):
		TYPE_DICTIONARY:
			return _sanitize_dictionary(value, template)
		TYPE_ARRAY:
			return _sanitize_array(value, template)
		TYPE_STRING:
			return _coerce_string(value, str(template))
		TYPE_INT:
			return _coerce_int(value, int(template))
		TYPE_FLOAT:
			return _coerce_float(value, float(template))
		TYPE_BOOL:
			return _coerce_bool(value, bool(template))
		TYPE_NIL:
			return _normalize_variant(value)
		_:
			if typeof(value) == typeof(template):
				return _normalize_variant(value)
			return template

func _sanitize_array(value: Variant, defaults: Array) -> Array:
	var result: Array = defaults.duplicate(true)
	if typeof(value) != TYPE_ARRAY:
		return result
	var source: Array = value
	for index in range(source.size()):
		var template: Variant = null
		if index < result.size():
			template = result[index]
			result[index] = _sanitize_value(source[index], template)
		else:
			if not defaults.is_empty():
				template = defaults[defaults.size() - 1]
			result.append(_sanitize_value(source[index], template))
	return result

func _normalize_variant(value: Variant) -> Variant:
	match typeof(value):
		TYPE_DICTIONARY:
			var normalized: Dictionary = {}
			var source: Dictionary = value
			for key in source.keys():
				normalized[key] = _normalize_variant(source[key])
			return normalized
		TYPE_ARRAY:
			var normalized_array: Array = []
			var source_array: Array = value
			for entry in source_array:
				normalized_array.append(_normalize_variant(entry))
			return normalized_array
		TYPE_STRING, TYPE_INT, TYPE_FLOAT, TYPE_BOOL, TYPE_NIL:
			return value
		_:
			return null

func _coerce_string(value: Variant, fallback: String) -> String:
	if typeof(value) == TYPE_NIL:
		return fallback
	var result := str(value).strip_edges()
	if result.is_empty():
		return fallback
	return result

func _coerce_int(value: Variant, fallback: int) -> int:
	if typeof(value) == TYPE_NIL:
		return fallback
	return int(value)

func _coerce_float(value: Variant, fallback: float) -> float:
	if typeof(value) == TYPE_NIL:
		return fallback
	return float(value)

func _coerce_bool(value: Variant, fallback: bool) -> bool:
	match typeof(value):
		TYPE_NIL:
			return fallback
		TYPE_BOOL:
			return value
		TYPE_INT, TYPE_FLOAT:
			return value != 0
		TYPE_STRING:
			var normalized := str(value).strip_edges().to_lower()
			if normalized in ["true", "1", "yes", "on"]:
				return true
			if normalized in ["false", "0", "no", "off", ""]:
				return false
			return fallback
		_:
			return fallback
