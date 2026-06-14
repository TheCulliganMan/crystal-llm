extends Control
class_name BootSceneBase

var runtime: Variant = null
var state: Dictionary = {}
var _title_label: Label
var _status_label: Label
var _detail_label: Label

const BOOT_ACTIONS := {
	"game_up": "up",
	"game_down": "down",
	"game_left": "left",
	"game_right": "right",
	"game_a": "a",
	"game_b": "b",
	"game_start": "start",
	"game_select": "select",
}

func _ready() -> void:
	_bind_labels()
	_on_ready()
	_normalize_common_state()
	set_process(true)
	set_process_unhandled_input(true)

func _on_ready() -> void:
	pass

func set_runtime(node: Variant) -> void:
	runtime = node

func get_state() -> Dictionary:
	_normalize_common_state()
	return state.duplicate(true)

func from_state(data: Dictionary) -> void:
	state = Dictionary(data).duplicate(true)
	_normalize_common_state()
	_on_state_restored()
	_normalize_common_state()
	_refresh_labels()

func to_dictionary() -> Dictionary:
	return get_state()

func from_dictionary(data: Variant) -> bool:
	if typeof(data) != TYPE_DICTIONARY:
		return false
	from_state(Dictionary(data))
	return true

func request_route(route_name: String, reason: String = "boot") -> void:
	_mark_route_pending(route_name, reason)
	var target: Variant = runtime
	if target == null:
		target = get_parent()
	if target != null and target is Object and target.has_method("request_scene_route"):
		target.call("request_scene_route", route_name, reason)

func request_public_route(method_name: String) -> void:
	_mark_route_pending(method_name, "boot_public_route")
	var target: Variant = runtime
	if target == null:
		target = get_parent()
	if target != null and target is Object and target.has_method(method_name):
		target.call(method_name)

func _on_state_restored() -> void:
	pass

func _tick(_delta: float) -> void:
	pass

func _handle_boot_input(_event: InputEvent) -> void:
	pass

func can_accept_input() -> bool:
	return not bool(state.get("input_locked", false)) and str(state.get("pending_route", "")).is_empty()

func should_block_gameplay_input() -> bool:
	return true

func is_input_locked() -> bool:
	return bool(state.get("input_locked", false))

func get_pending_route() -> String:
	return str(state.get("pending_route", ""))

func reset_route_transition() -> void:
	state["pending_route"] = ""
	state["route_reason"] = ""
	state["input_locked"] = false

func _default_screen_state(screen_name: String) -> Dictionary:
	return {
		"screen": screen_name,
		"input_owned": true,
		"input_locked": false,
		"pending_action": "",
		"pending_action_payload": {},
		"last_action_payload": {},
		"action_sequence": 0,
		"last_input": _empty_input_snapshot(),
		"handled_input_count": 0,
		"pending_route": "",
		"route_reason": "",
	}

func _normalize_common_state() -> void:
	if not state.has("screen"):
		state["screen"] = _screen_key()
	state["input_owned"] = bool(state.get("input_owned", true))
	state["input_locked"] = bool(state.get("input_locked", false))
	state["pending_action"] = str(state.get("pending_action", ""))
	state["pending_action_payload"] = _normalize_action_payload(state.get("pending_action_payload", {}))
	state["last_action_payload"] = _normalize_action_payload(state.get("last_action_payload", {}))
	state["action_sequence"] = max(0, int(state.get("action_sequence", 0)))
	state["last_input"] = _normalize_input_snapshot(state.get("last_input", {}))
	state["handled_input_count"] = int(state.get("handled_input_count", 0))
	state["pending_route"] = str(state.get("pending_route", ""))
	state["route_reason"] = str(state.get("route_reason", ""))

func queue_action(route_name: String, action_id: String = "", payload: Dictionary = {}) -> void:
	var normalized_route := route_name.strip_edges()
	var normalized_action_id := action_id.strip_edges()
	if normalized_action_id.is_empty():
		normalized_action_id = normalized_route
	var sequence := int(state.get("action_sequence", 0)) + 1
	state["action_sequence"] = sequence
	state["pending_action"] = normalized_route
	state["pending_action_payload"] = _build_action_payload(normalized_route, normalized_action_id, payload, sequence)
	state["input_locked"] = true

func pop_action() -> Variant:
	var action: Variant = state.get("pending_action", null)
	if action == null:
		return null
	var action_text := str(action).strip_edges()
	if action_text.is_empty():
		return null
	state["last_action_payload"] = _consume_action_payload()
	state["pending_action"] = ""
	state["pending_action_payload"] = {}
	return action_text

func popAction() -> Variant:
	return pop_action()

func clear_pending_action() -> void:
	state["pending_action"] = ""
	state["pending_action_payload"] = {}

func _record_input(event: InputEvent) -> Dictionary:
	var snapshot := _input_snapshot_from_event(event)
	state["last_input"] = snapshot
	state["handled_input_count"] = int(state.get("handled_input_count", 0)) + 1
	return snapshot

func _input_pressed(event: InputEvent, buttons: Array) -> bool:
	for button in buttons:
		var action := _action_for_button(str(button))
		if not action.is_empty() and event.is_action_pressed(action):
			return true
	return false

func _input_released(event: InputEvent, buttons: Array) -> bool:
	for button in buttons:
		var action := _action_for_button(str(button))
		if not action.is_empty() and event.is_action_released(action):
			return true
	return false

func _last_pressed(button: String) -> bool:
	var pressed: Dictionary = Dictionary(Dictionary(state.get("last_input", {})).get("pressed", {}))
	return bool(pressed.get(button, false))

func _last_down(button: String) -> bool:
	var down: Dictionary = Dictionary(Dictionary(state.get("last_input", {})).get("down", {}))
	return bool(down.get(button, false))

func _set_input_locked(is_locked: bool) -> void:
	state["input_locked"] = is_locked

func _mark_route_pending(route_name: String, reason: String) -> void:
	state["pending_route"] = route_name
	state["route_reason"] = reason
	state["input_locked"] = true

func _route_entry_reset() -> bool:
	return bool(state.get("route_entry", false))

func _clear_route_entry() -> void:
	state.erase("route_entry")
	reset_route_transition()

func _clamp_selection(value: Variant, option_count: int, fallback: int = 0) -> int:
	if option_count <= 0:
		return 0
	return clampi(int(value), 0, option_count - 1)

func _bind_labels() -> void:
	_title_label = get_node_or_null("Margin/VBox/TitleLabel")
	_status_label = get_node_or_null("Margin/VBox/StatusLabel")
	_detail_label = get_node_or_null("Margin/VBox/DetailLabel")

func _process(delta: float) -> void:
	_tick(delta)
	_refresh_labels()

func _unhandled_input(event: InputEvent) -> void:
	if event is InputEventKey and event.echo:
		return
	_record_input(event)
	if not can_accept_input():
		return
	_handle_boot_input(event)

func _set_labels(title: String, status: String, detail: String) -> void:
	if is_instance_valid(_title_label):
		_title_label.text = title
	if is_instance_valid(_status_label):
		_status_label.text = status
	if is_instance_valid(_detail_label):
		_detail_label.text = detail

func _refresh_labels() -> void:
	_update_labels()

func _update_labels() -> void:
	pass

func _screen_key() -> String:
	var node_name := str(name).to_snake_case()
	return node_name if not node_name.is_empty() else "boot_scene"

func _action_for_button(button: String) -> String:
	for action in BOOT_ACTIONS.keys():
		if str(BOOT_ACTIONS[action]) == button:
			return str(action)
	return ""

func _input_snapshot_from_event(event: InputEvent) -> Dictionary:
	var pressed: Dictionary = {}
	var released: Dictionary = {}
	for action in BOOT_ACTIONS.keys():
		var button := str(BOOT_ACTIONS[action])
		if event.is_action_pressed(str(action)):
			pressed[button] = true
		if event.is_action_released(str(action)):
			released[button] = true
	return {
		"pressed": pressed,
		"released": released,
		"down": _current_down_buttons(),
	}

func _current_down_buttons() -> Dictionary:
	var down: Dictionary = {}
	for action in BOOT_ACTIONS.keys():
		down[str(BOOT_ACTIONS[action])] = Input.is_action_pressed(str(action))
	return down

func _empty_input_snapshot() -> Dictionary:
	return {
		"pressed": {},
		"released": {},
		"down": {},
	}

func _normalize_input_snapshot(value: Variant) -> Dictionary:
	if typeof(value) != TYPE_DICTIONARY:
		return _empty_input_snapshot()
	var source: Dictionary = value
	return {
		"pressed": Dictionary(source.get("pressed", {})).duplicate(true),
		"released": Dictionary(source.get("released", {})).duplicate(true),
		"down": Dictionary(source.get("down", {})).duplicate(true),
	}

func _build_action_payload(route_name: String, action_id: String, payload: Dictionary, sequence: int) -> Dictionary:
	var action_payload := {
		"action_id": action_id,
		"route": route_name,
		"source_screen": str(state.get("screen", _screen_key())),
		"phase": str(state.get("phase", state.get("scene_phase", state.get("mode", "")))),
		"frame_counter": int(state.get("frame_counter", state.get("phase_frame", 0))),
		"tick_counter": int(state.get("handled_input_count", 0)),
		"action_sequence": max(0, sequence),
	}
	for key in payload.keys():
		action_payload[key] = payload[key]
	return action_payload

func _consume_action_payload() -> Dictionary:
	var payload := _normalize_action_payload(state.get("pending_action_payload", {}))
	if payload.is_empty():
		return {}
	payload["consumed_frame_counter"] = int(state.get("frame_counter", state.get("phase_frame", 0)))
	payload["consumed_tick_counter"] = int(state.get("handled_input_count", 0))
	return payload

func _normalize_action_payload(value: Variant) -> Dictionary:
	if typeof(value) != TYPE_DICTIONARY:
		return {}
	return Dictionary(value).duplicate(true)
