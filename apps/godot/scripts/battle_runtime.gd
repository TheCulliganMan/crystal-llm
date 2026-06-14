extends Control

const GB_FRAME_RATE := 59.7275
const FRAME_DURATION_MS := 1000.0 / GB_FRAME_RATE
const MAX_CATCH_UP_FRAMES := 5
const BATTLE_PHASE_SETUP := "setup"
const BATTLE_PHASE_INTRO := "intro"
const BATTLE_PHASE_TURN_PROMPT := "turn_prompt"
const BATTLE_PHASE_RESOLUTION := "resolution"
const BATTLE_PHASE_POST_TURN := "post_turn"
const BATTLE_PHASE_COMPLETE := "complete"
const SIDE_PLAYER := "player"
const SIDE_NONE := "none"
const UI_PHASE_INACTIVE := "INACTIVE"
const UI_PHASE_MENU := "MENU"
const UI_PHASE_DIALOGUE := "DIALOGUE"
const UI_PHASE_ANIMATION := "ANIMATION"
const UI_PHASE_COMPLETE := "COMPLETE"

signal state_changed
signal battle_started
signal battle_completed
signal command_queued
signal resolution_event_queued

const BATTLE_STATE_SCRIPT := preload("res://scripts/battle_state.gd")
const BATTLE_ASSETS_SCRIPT := preload("res://scripts/battle_assets.gd")
const BATTLE_UI_STATE_SCRIPT := preload("res://scripts/battle_ui_state.gd")

var accumulator_ms := 0.0
var battle_frame := 0
var battle_state = null
var battle_assets = null
var battle_ui_state = null
var _battle_ui_root: Node = null
var _last_ui_signature := ""
var _title_label: Label
var _battle_label: Label
var _phase_label: Label
var _turn_label: Label
var _prompt_label: Label
var _queue_label: Label
var _event_label: Label
var _flow_label: Label
var _history_label: Label
var _assets_label: Label
var _debug_label: Label

func _ready() -> void:
	_bind_labels()
	_ensure_runtime_objects()
	_bind_battle_ui()
	_reset_shell()
	_refresh_assets()
	set_process(true)
	_refresh_ui(true)

func _ensure_runtime_objects() -> void:
	if battle_state == null:
		battle_state = BATTLE_STATE_SCRIPT.new()
	if battle_assets == null:
		battle_assets = BATTLE_ASSETS_SCRIPT.new()
	if battle_ui_state == null:
		battle_ui_state = BATTLE_UI_STATE_SCRIPT.new()

func _bind_labels() -> void:
	_title_label = _resolve_ui_node("Margin/VBox/TitleLabel")
	_battle_label = _resolve_ui_node("Margin/VBox/StatusPanel/Margin/StatusVBox/BattleLabel")
	_phase_label = _resolve_ui_node("Margin/VBox/StatusPanel/Margin/StatusVBox/PhaseLabel")
	_turn_label = _resolve_ui_node("Margin/VBox/StatusPanel/Margin/StatusVBox/TurnLabel")
	_prompt_label = _resolve_ui_node("Margin/VBox/StatusPanel/Margin/StatusVBox/PromptLabel")
	_queue_label = _resolve_ui_node("Margin/VBox/StatusPanel/Margin/StatusVBox/QueueLabel")
	_event_label = _resolve_ui_node("Margin/VBox/StatusPanel/Margin/StatusVBox/EventLabel")
	_flow_label = _resolve_ui_node("Margin/VBox/StatusPanel/Margin/StatusVBox/FlowLabel")
	_history_label = _resolve_ui_node("Margin/VBox/StatusPanel/Margin/StatusVBox/HistoryLabel")
	_assets_label = _resolve_ui_node("Margin/VBox/StatusPanel/Margin/StatusVBox/AssetsLabel")
	_debug_label = _resolve_ui_node("Margin/VBox/StatusPanel/Margin/StatusVBox/DebugLabel")

func _bind_battle_ui() -> void:
	if battle_ui_state != null:
		battle_ui_state.bind_battle_state(battle_state)
	_battle_ui_root = get_node_or_null("BattleUI")
	if is_instance_valid(_battle_ui_root) and _battle_ui_root.has_method("bind_runtime"):
		_battle_ui_root.call("bind_runtime", self)

func _resolve_ui_node(path: String) -> Node:
	var direct: Node = get_node_or_null(path)
	if direct != null:
		return direct
	var nested: Node = get_node_or_null("BattleUI/%s" % path)
	if nested != null:
		return nested
	return null

func _reset_shell() -> void:
	battle_frame = 0
	accumulator_ms = 0.0
	battle_state.reset()
	if battle_ui_state != null and battle_ui_state.has_method("reset"):
		battle_ui_state.reset()
	battle_state.set_phase(BATTLE_PHASE_SETUP)
	battle_state.set_prompt_gate(false, "", "battle shell awaiting context", false, "no turn execution yet")
	_sync_battle_ui_state()

func _refresh_assets() -> void:
	battle_assets.ensure_loaded()
	battle_state.set_asset_summary(battle_assets.summary)

func begin_battle(context: Dictionary = {}) -> void:
	_ensure_runtime_objects()
	_reset_shell()
	battle_state.set_context(context)
	_apply_selected_payload_context(context)
	battle_state.set_phase(BATTLE_PHASE_INTRO)
	battle_state.open_turn_prompt("turn_command", "awaiting player command", true, "battle shell armed")
	_sync_battle_ui_state()
	emit_signal("battle_started")
	emit_signal("state_changed")
	_refresh_ui(true)

func reset() -> void:
	_ensure_runtime_objects()
	_reset_shell()
	_refresh_assets()
	_sync_battle_ui_state()
	emit_signal("state_changed")
	_refresh_ui(true)

func set_context(context: Dictionary) -> void:
	_ensure_runtime_objects()
	battle_state.set_context(context)
	_sync_battle_ui_state()
	emit_signal("state_changed")
	_refresh_ui(true)

func set_asset_summary(summary: Dictionary) -> void:
	_ensure_runtime_objects()
	battle_state.set_asset_summary(summary)
	_sync_battle_ui_state()
	emit_signal("state_changed")
	_refresh_ui(true)

func set_selected_player_payload(payload: Dictionary) -> void:
	_ensure_runtime_objects()
	battle_state.set_selected_player_payload(battle_assets.hydrate_actor_payload(payload))
	_sync_battle_ui_state()
	emit_signal("state_changed")
	_refresh_ui(true)

func get_selected_player_payload() -> Dictionary:
	_ensure_runtime_objects()
	return battle_state.get_selected_player_payload()

func set_selected_opponent_payload(payload: Dictionary) -> void:
	_ensure_runtime_objects()
	battle_state.set_selected_opponent_payload(battle_assets.hydrate_actor_payload(payload))
	_sync_battle_ui_state()
	emit_signal("state_changed")
	_refresh_ui(true)

func get_selected_opponent_payload() -> Dictionary:
	_ensure_runtime_objects()
	return battle_state.get_selected_opponent_payload()

func set_selected_battle_payloads(player_payload: Dictionary, opponent_payload: Dictionary) -> void:
	_ensure_runtime_objects()
	battle_state.set_selected_battle_payloads(
		battle_assets.hydrate_actor_payload(player_payload),
		battle_assets.hydrate_actor_payload(opponent_payload)
	)
	_sync_battle_ui_state()
	emit_signal("state_changed")
	_refresh_ui(true)

func clear_selected_battle_payloads() -> void:
	_ensure_runtime_objects()
	battle_state.set_selected_battle_payloads({}, {})
	_sync_battle_ui_state()
	emit_signal("state_changed")
	_refresh_ui(true)

func get_pokemon_asset(identifier: Variant) -> Dictionary:
	_ensure_runtime_objects()
	battle_assets.ensure_loaded()
	return battle_assets.get_pokemon(identifier)

func get_move_asset(identifier: Variant) -> Dictionary:
	_ensure_runtime_objects()
	battle_assets.ensure_loaded()
	return battle_assets.get_move(identifier)

func get_item_asset(identifier: Variant) -> Dictionary:
	_ensure_runtime_objects()
	battle_assets.ensure_loaded()
	return battle_assets.get_item(identifier)

func get_trainer_asset(identifier: Variant) -> Dictionary:
	_ensure_runtime_objects()
	battle_assets.ensure_loaded()
	return battle_assets.get_trainer(identifier)

func set_turn_phase(phase: String) -> void:
	_ensure_runtime_objects()
	battle_state.set_phase(phase)
	_sync_battle_ui_state()
	emit_signal("state_changed")
	_refresh_ui(true)

func set_prompt_gate(active: bool, kind: String = "", message: String = "", locked: bool = false, reason: String = "") -> void:
	_ensure_runtime_objects()
	battle_state.set_prompt_gate(active, kind, message, locked, reason)
	_sync_battle_ui_state()
	emit_signal("state_changed")
	_refresh_ui(true)

func clear_prompt_gate() -> void:
	_ensure_runtime_objects()
	battle_state.clear_prompt_gate()
	_sync_battle_ui_state()
	emit_signal("state_changed")
	_refresh_ui(true)

func open_turn_prompt(kind: String = "turn_command", message: String = "awaiting player command", locked: bool = true, reason: String = "") -> void:
	_ensure_runtime_objects()
	battle_state.open_turn_prompt(kind, message, locked, reason)
	_sync_battle_ui_state()
	emit_signal("state_changed")
	_refresh_ui(true)

func queue_command(command: Dictionary) -> void:
	_ensure_runtime_objects()
	battle_state.enqueue_command(command)
	emit_signal("command_queued")
	_sync_battle_ui_state()
	emit_signal("state_changed")
	_refresh_ui(true)

func has_pending_command() -> bool:
	_ensure_runtime_objects()
	return battle_state.has_pending_command()

func get_pending_command() -> Dictionary:
	_ensure_runtime_objects()
	return battle_state.get_pending_command()

func get_last_resolved_command() -> Dictionary:
	_ensure_runtime_objects()
	return battle_state.get_last_resolved_command()

func get_last_turn_resolution() -> Dictionary:
	_ensure_runtime_objects()
	return battle_state.get_last_turn_resolution()

func get_battle_result_state() -> Dictionary:
	_ensure_runtime_objects()
	return battle_state.get_battle_result_state()

func get_state_revision() -> int:
	_ensure_runtime_objects()
	return battle_state.get_state_revision()

func get_dialogue_wait_gate_active() -> bool:
	_ensure_runtime_objects()
	return battle_state.get_dialogue_wait_gate_active()

func get_fast_animation_request() -> bool:
	_ensure_runtime_objects()
	return battle_state.get_fast_animation_request()

func get_fast_text_request() -> bool:
	_ensure_runtime_objects()
	return battle_state.get_fast_text_request()

func get_waiting_for_input() -> bool:
	_ensure_runtime_objects()
	return battle_state.get_waiting_for_input()

func get_phase_history() -> Array:
	_ensure_runtime_objects()
	return battle_state.get_phase_history()

func get_log_lines() -> Array:
	_ensure_runtime_objects()
	return battle_state.get_log_lines()

func has_resolution_events() -> bool:
	_ensure_runtime_objects()
	return battle_state.has_resolution_events()

func get_resolution_events() -> Array:
	_ensure_runtime_objects()
	return battle_state.get_resolution_events()

func get_pending_animation_events() -> Array:
	_ensure_runtime_objects()
	return battle_state.get_pending_animation_events()

func has_pending_animation_events() -> bool:
	_ensure_runtime_objects()
	return battle_state.has_pending_animation_events()

func consume_pending_animation_events() -> Array:
	_ensure_runtime_objects()
	var events: Array = battle_state.consume_pending_animation_events()
	if not events.is_empty():
		_sync_battle_ui_state()
		emit_signal("state_changed")
		_refresh_ui(true)
	return events

func get_last_battle_event() -> Dictionary:
	_ensure_runtime_objects()
	return battle_state.get_last_battle_event()

func get_active_text_event() -> Dictionary:
	_ensure_runtime_objects()
	return battle_state.get_active_text_event()

func get_active_animation_event() -> Dictionary:
	_ensure_runtime_objects()
	return battle_state.get_active_animation_event()

func get_trainer_sprite_state() -> Dictionary:
	_ensure_runtime_objects()
	return battle_state.get_trainer_sprite_state()

func get_battle_ui_state() -> Dictionary:
	_ensure_runtime_objects()
	if battle_ui_state != null and battle_ui_state.has_method("get_state"):
		return battle_ui_state.get_state()
	return {}

func get_dialogue_state() -> Dictionary:
	_ensure_runtime_objects()
	if battle_ui_state != null and battle_ui_state.has_method("get_dialogue_state"):
		return battle_ui_state.get_dialogue_state()
	return {}

func enqueue_dialogue_text(text: String, control: String = "") -> bool:
	_ensure_runtime_objects()
	if battle_ui_state == null or not battle_ui_state.has_method("enqueue_dialogue_text"):
		return false
	var queued := bool(battle_ui_state.enqueue_dialogue_text(text, control))
	if queued:
		_sync_battle_ui_state()
		emit_signal("state_changed")
		_refresh_ui(true)
	return queued

func close_dialogue() -> void:
	_ensure_runtime_objects()
	if battle_ui_state != null and battle_ui_state.has_method("close_dialogue"):
		battle_ui_state.close_dialogue()
	_sync_battle_ui_state()
	emit_signal("state_changed")
	_refresh_ui(true)

func open_command_menu(entries: Array) -> void:
	_ensure_runtime_objects()
	if battle_ui_state != null and battle_ui_state.has_method("open_command_menu"):
		battle_ui_state.open_command_menu(entries)
	_sync_battle_ui_state()
	emit_signal("state_changed")
	_refresh_ui(true)

func get_latched_command() -> Dictionary:
	_ensure_runtime_objects()
	if battle_ui_state != null and battle_ui_state.has_method("get_latched_command"):
		return battle_ui_state.get_latched_command()
	return {}

func clear_latched_command() -> void:
	_ensure_runtime_objects()
	if battle_ui_state != null and battle_ui_state.has_method("clear_latched_command"):
		battle_ui_state.clear_latched_command()
	_sync_battle_ui_state()
	emit_signal("state_changed")
	_refresh_ui(true)

func get_prompt_gate_state() -> Dictionary:
	_ensure_runtime_objects()
	if battle_ui_state != null:
		var snapshot: Dictionary = battle_ui_state.prompt_gate_state
		return snapshot.duplicate(true)
	return {}

func clear_queued_commands() -> void:
	_ensure_runtime_objects()
	battle_state.clear_queued_commands()
	_sync_battle_ui_state()
	emit_signal("state_changed")
	_refresh_ui(true)

func consume_pending_command() -> Dictionary:
	_ensure_runtime_objects()
	var command: Dictionary = battle_state.consume_pending_command()
	_sync_battle_ui_state()
	emit_signal("state_changed")
	_refresh_ui(true)
	return command

func begin_resolution(command: Dictionary = {}) -> bool:
	_ensure_runtime_objects()
	var enqueued_command := false
	if not command.is_empty():
		battle_state.enqueue_command(command)
		enqueued_command = true
	var command_payload := _hydrated_pending_command(command)
	if battle_state.begin_resolution(command_payload):
		if enqueued_command:
			emit_signal("command_queued")
		emit_signal("resolution_event_queued")
		_sync_battle_ui_state()
		emit_signal("state_changed")
		_refresh_ui(true)
		return true
	return false

func queue_resolution_event(event: Dictionary) -> void:
	_ensure_runtime_objects()
	battle_state.queue_resolution_event(event)
	emit_signal("resolution_event_queued")
	_sync_battle_ui_state()
	emit_signal("state_changed")
	_refresh_ui(true)

func queue_battle_ui_event(event_name: String, data: Dictionary = {}, defer_until_animation: bool = false) -> Dictionary:
	_ensure_runtime_objects()
	var event: Dictionary = battle_state.queue_battle_ui_event(event_name, data, defer_until_animation)
	if not event.is_empty():
		emit_signal("resolution_event_queued")
	_sync_battle_ui_state()
	emit_signal("state_changed")
	_refresh_ui(true)
	return event

func queue_text_event(text: String, wait_for_animation: bool = false, data: Dictionary = {}) -> Dictionary:
	_ensure_runtime_objects()
	var event: Dictionary = battle_state.queue_text_event(text, wait_for_animation, data)
	if not event.is_empty():
		emit_signal("resolution_event_queued")
	_sync_battle_ui_state()
	emit_signal("state_changed")
	_refresh_ui(true)
	return event

func queue_animation_event(event_name: String, data: Dictionary = {}) -> Dictionary:
	_ensure_runtime_objects()
	var event: Dictionary = battle_state.queue_animation_event(event_name, data)
	if not event.is_empty():
		emit_signal("resolution_event_queued")
	_sync_battle_ui_state()
	emit_signal("state_changed")
	_refresh_ui(true)
	return event

func record_resolution(command: Dictionary, resolution_message: String = "") -> void:
	_ensure_runtime_objects()
	battle_state.record_resolution(command, resolution_message)
	emit_signal("resolution_event_queued")
	_sync_battle_ui_state()
	emit_signal("state_changed")
	_refresh_ui(true)

func complete_resolution(resolution_summary: String = "") -> void:
	_ensure_runtime_objects()
	battle_state.complete_resolution(resolution_summary)
	emit_signal("resolution_event_queued")
	_sync_battle_ui_state()
	emit_signal("state_changed")
	_refresh_ui(true)

func advance_turn(next_side: String = SIDE_PLAYER) -> void:
	_ensure_runtime_objects()
	battle_state.advance_turn(next_side)
	_sync_battle_ui_state()
	emit_signal("state_changed")
	_refresh_ui(true)

func complete_turn(resolution_summary: String = "") -> void:
	_ensure_runtime_objects()
	battle_state.complete_resolution(resolution_summary)
	if not battle_state.battle_finished:
		battle_state.advance_turn(SIDE_PLAYER)
		battle_state.open_turn_prompt("turn_command", "awaiting player command", true, "next turn ready")
	emit_signal("resolution_event_queued")
	_sync_battle_ui_state()
	emit_signal("state_changed")
	_refresh_ui(true)

func advance_phase() -> void:
	_ensure_runtime_objects()
	battle_state.advance_phase()
	_sync_battle_ui_state()
	emit_signal("state_changed")
	_refresh_ui(true)

func drain_resolution_events() -> Array:
	_ensure_runtime_objects()
	var events: Array = battle_state.consume_resolution_events()
	if not events.is_empty():
		_sync_battle_ui_state()
		emit_signal("state_changed")
		_refresh_ui(true)
	return events

func complete_battle(result: String, detail: Dictionary = {}) -> void:
	_ensure_runtime_objects()
	battle_state.mark_complete(result, detail)
	_sync_battle_ui_state()
	emit_signal("resolution_event_queued")
	emit_signal("battle_completed")
	emit_signal("state_changed")
	_refresh_ui(true)

func handle_ui_action(action: String) -> bool:
	_ensure_runtime_objects()
	var had_pending: bool = bool(battle_state.has_pending_command())
	if is_instance_valid(_battle_ui_root) and _battle_ui_root.has_method("handle_action"):
		var handled: Variant = _battle_ui_root.call("handle_action", action)
		if handled is bool:
			_sync_battle_ui_state()
			if bool(handled) and not had_pending and battle_state.has_pending_command():
				emit_signal("command_queued")
			if bool(handled):
				emit_signal("state_changed")
			_refresh_ui(true)
			return handled
	if battle_ui_state != null and battle_ui_state.has_method("handle_action"):
		var state_handled: Variant = battle_ui_state.call("handle_action", action)
		if state_handled is bool and state_handled:
			_sync_battle_ui_state()
			if not had_pending and battle_state.has_pending_command():
				emit_signal("command_queued")
			emit_signal("state_changed")
			_refresh_ui(true)
			return true
	return false

func get_state() -> Dictionary:
	_ensure_runtime_objects()
	_sync_battle_ui_state()
	return {
		"battle_state": battle_state.get_state(),
		"battle_ui_state": battle_ui_state.get_state() if battle_ui_state != null and battle_ui_state.has_method("get_state") else {},
		"battle_frame": battle_frame,
		"accumulator_ms": accumulator_ms,
	}

func from_state(data: Dictionary) -> void:
	_ensure_runtime_objects()
	if data.is_empty():
		_reset_shell()
		_refresh_assets()
		emit_signal("state_changed")
		_refresh_ui(true)
		return
	var payload: Dictionary = data
	if data.has("battle_state"):
		payload = Dictionary(data.get("battle_state", {}))
	if battle_state.has_method("from_state"):
		battle_state.call("from_state", payload)
	elif battle_state.has_method("from_dictionary"):
		battle_state.call("from_dictionary", payload)
	if data.has("battle_ui_state") and battle_ui_state != null and battle_ui_state.has_method("from_dictionary"):
		battle_ui_state.call("from_dictionary", data.get("battle_ui_state", {}))
	if data.has("battle_frame"):
		battle_frame = max(0, int(data.get("battle_frame", 0)))
	elif payload.has("frame_counter"):
		battle_frame = max(0, int(payload.get("frame_counter", battle_frame)))
	var accumulator_source: Variant = data.get("accumulator_ms", payload.get("accumulator_ms", 0.0))
	if data.has("accumulator_ms") or payload.has("accumulator_ms"):
		accumulator_ms = clampf(float(accumulator_source), 0.0, FRAME_DURATION_MS * MAX_CATCH_UP_FRAMES)
	else:
		accumulator_ms = 0.0
	_sync_battle_ui_state()
	_refresh_assets()
	emit_signal("state_changed")
	_refresh_ui(true)

func to_dictionary() -> Dictionary:
	return get_state()

func from_dictionary(data: Variant) -> bool:
	if typeof(data) != TYPE_DICTIONARY:
		return false
	from_state(Dictionary(data))
	return true

func tick() -> void:
	_ensure_runtime_objects()
	_advance_fixed_step()
	_refresh_ui(true)

func tick_fixed_step() -> void:
	tick()

func _process(delta: float) -> void:
	accumulator_ms = min(accumulator_ms + (delta * 1000.0), FRAME_DURATION_MS * MAX_CATCH_UP_FRAMES)
	var steps := 0
	var stepped := false
	while accumulator_ms >= FRAME_DURATION_MS and steps < MAX_CATCH_UP_FRAMES:
		accumulator_ms -= FRAME_DURATION_MS
		_advance_fixed_step()
		steps += 1
		stepped = true
	if stepped:
		_refresh_ui()

func _step_simulation() -> void:
	_advance_fixed_step()

func _advance_fixed_step() -> void:
	battle_frame += 1
	if not _should_block_state_advance():
		battle_state.tick_fixed_step()
	_sync_battle_ui_state()
	emit_signal("state_changed")

func set_waiting_for_input(value: bool) -> void:
	_ensure_runtime_objects()
	battle_state.set_waiting_for_input(value)
	_sync_battle_ui_state()
	emit_signal("state_changed")
	_refresh_ui(true)

func is_waiting_for_input() -> bool:
	_ensure_runtime_objects()
	return battle_state.waiting_for_input or battle_state.manual_wait_override

func set_fast_animation_request(value: bool) -> void:
	_ensure_runtime_objects()
	battle_state.set_fast_animation_request(value)
	emit_signal("state_changed")
	_refresh_ui(true)

func set_fast_text_request(value: bool) -> void:
	_ensure_runtime_objects()
	battle_state.set_fast_text_request(value)
	emit_signal("state_changed")
	_refresh_ui(true)

func _refresh_ui(force: bool = false) -> void:
	_sync_battle_ui_state()
	if is_instance_valid(_battle_ui_root) and _battle_ui_root.has_method("refresh_display"):
		_battle_ui_root.call("refresh_display", self, force)
		return
	var signature := _build_ui_signature()
	if not force and signature == _last_ui_signature:
		return
	_last_ui_signature = signature
	if is_instance_valid(_title_label):
		_title_label.text = "BATTLE"
	if is_instance_valid(_battle_label):
		_battle_label.text = "Battle: %s | Kind: %s | Frame: %d | Rev: %d | UI: %s" % [
			battle_state.battle_label,
			battle_state.battle_kind,
			battle_frame,
			battle_state.state_revision,
			battle_state.ui_phase,
		]
	if is_instance_valid(_phase_label):
		_phase_label.text = "Phase: %s | Active Side: %s | Turn: %d" % [
			battle_state.describe_phase(),
			battle_state.active_side,
			battle_state.turn_number,
		]
	if is_instance_valid(_turn_label):
		_turn_label.text = "Turn State: gate=%s locked=%s waiting=%s manual=%s finished=%s result=%s" % [
			str(battle_state.prompt_gate_active).to_lower(),
			str(battle_state.prompt_locked).to_lower(),
			str(battle_state.waiting_for_input).to_lower(),
			str(battle_state.manual_wait_override).to_lower(),
			str(battle_state.battle_finished).to_lower(),
			battle_state.battle_result,
		]
	if is_instance_valid(_prompt_label):
		var prompt_line := "Prompt: %s / %s" % [
			battle_state.prompt_kind if not battle_state.prompt_kind.is_empty() else "none",
			battle_state.prompt_message if not battle_state.prompt_message.is_empty() else "idle",
		]
		if not battle_state.prompt_gate_reason.is_empty():
			prompt_line += " | Reason: %s" % battle_state.prompt_gate_reason
		prompt_line += " | Gate: %s" % str(battle_state.dialogue_wait_gate_active).to_lower()
		_prompt_label.text = prompt_line
	if is_instance_valid(_queue_label):
		var pending_value := "no"
		if not battle_state.pending_command.is_empty():
			var pending_kind: Variant = battle_state.pending_command.get("kind", "")
			if str(pending_kind).is_empty():
				pending_kind = battle_state.pending_command.get("label", "yes")
			pending_value = str(pending_kind)
		_queue_label.text = "Queued commands: %d | Pending: %s" % [
			int(battle_state.queued_commands.size()),
			pending_value,
		]
	if is_instance_valid(_event_label):
		var events: Array = battle_state.resolution_events
		var event_text := "Resolution events: %d" % events.size()
		if not events.is_empty():
			event_text += " | Latest: %s" % _describe_event(Dictionary(events[events.size() - 1]))
		_event_label.text = event_text
	if is_instance_valid(_flow_label):
		var resolved_value := "no"
		if not battle_state.last_resolved_command.is_empty():
			var resolved_kind: Variant = battle_state.last_resolved_command.get("kind", "")
			if str(resolved_kind).is_empty():
				resolved_kind = battle_state.last_resolved_command.get("label", "yes")
			resolved_value = str(resolved_kind)
		_flow_label.text = "Prompt flow: %s | Resolved: %s | UI phase: %s" % [
			battle_state.describe_prompt_gate(),
			resolved_value,
			battle_state.ui_phase,
		]
	if is_instance_valid(_history_label):
		var log_tail: String = battle_state.describe_recent_logs(2)
		_history_label.text = "History: %s | Logs: %s" % [
			battle_state.describe_recent_history(),
			log_tail,
		]
	if is_instance_valid(_assets_label):
		var summary: Dictionary = battle_state.asset_summary
		_assets_label.text = "Assets: %d pokemon, %d moves, %d items, %d trainers, %d anim tables, %d bundles, %d frontpics, %d packs" % [
			int(summary.get("pokemon_count", 0)),
			int(summary.get("move_count", 0)),
			int(summary.get("item_count", 0)),
			int(summary.get("trainer_count", 0)),
			int(summary.get("battle_animation_count", 0)),
			int(summary.get("battle_anim_bundle_count", 0)),
			int(summary.get("frontpic_animation_count", 0)),
			int(summary.get("content_pack_count", 0)),
		]
	if is_instance_valid(_debug_label):
		_debug_label.text = battle_state.debug_text()

func _build_ui_signature() -> String:
	var summary: Dictionary = battle_state.asset_summary
	var pending: Dictionary = battle_state.pending_command
	var parts: Array[String] = [
		str(battle_state.battle_label),
		str(battle_state.battle_kind),
		str(battle_state.turn_phase),
		str(battle_state.active_side),
		str(battle_state.prompt_gate_active),
		str(battle_state.prompt_locked),
		str(battle_state.waiting_for_input),
		str(battle_state.manual_wait_override),
		str(battle_state.ui_phase),
		str(battle_state.dialogue_wait_gate_active),
		str(battle_state.battle_finished),
		str(battle_frame),
		str(battle_state.frame_counter),
		str(battle_state.state_revision),
		str(summary.hash()),
		str(pending.hash()),
		str(battle_state.selected_player_payload.hash()),
		str(battle_state.selected_opponent_payload.hash()),
		str(battle_ui_state.get_state().hash() if battle_ui_state != null and battle_ui_state.has_method("get_state") else 0),
		str(battle_state.fast_animation_request),
		str(battle_state.fast_text_request),
	]
	return "|".join(parts)

func _sync_battle_ui_state() -> void:
	if battle_state == null:
		return
	if battle_state.has_method("sync_ui_state"):
		battle_state.sync_ui_state()
	if battle_ui_state != null and battle_ui_state.has_method("sync_from_battle_state"):
		battle_ui_state.sync_from_battle_state()

func _should_block_state_advance() -> bool:
	if battle_state == null:
		return false
	return bool(
		battle_state.manual_wait_override
			or battle_state.waiting_for_input
			or battle_state.dialogue_wait_gate_active
			or battle_state.has_pending_animation_events()
			or (battle_ui_state != null and battle_ui_state.dialogue != null and battle_ui_state.dialogue.waiting_flag())
			or battle_state.battle_finished
	)

func _describe_event(event: Dictionary) -> String:
	if event.has("type"):
		return str(event.get("type", "event"))
	if event.has("kind"):
		return str(event.get("kind", "event"))
	return "event"

func _apply_selected_payload_context(context: Dictionary) -> void:
	var player_payload := _first_dictionary(context, ["selected_player_payload", "player_payload", "player"])
	var opponent_payload := _first_dictionary(context, ["selected_opponent_payload", "opponent_payload", "opponent", "enemy"])
	if not player_payload.is_empty() or not opponent_payload.is_empty():
		battle_state.set_selected_battle_payloads(
			battle_assets.hydrate_actor_payload(player_payload),
			battle_assets.hydrate_actor_payload(opponent_payload)
		)

func _first_dictionary(source: Dictionary, keys: Array[String]) -> Dictionary:
	for key in keys:
		var value: Variant = source.get(key, {})
		if typeof(value) == TYPE_DICTIONARY:
			return Dictionary(value).duplicate(true)
	return {}

func _hydrated_pending_command(preferred_command: Dictionary = {}) -> Dictionary:
	var command := preferred_command.duplicate(true)
	if command.is_empty():
		command = battle_state.get_pending_command()
	if command.is_empty():
		return {}
	return battle_assets.hydrate_turn_command(
		command,
		battle_state.get_selected_player_payload(),
		battle_state.get_selected_opponent_payload()
	)
