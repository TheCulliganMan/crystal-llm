extends Control

const GB_FRAME_RATE := 59.7275
const FRAME_DURATION_MS := 1000.0 / GB_FRAME_RATE
const MAX_CATCH_UP_FRAMES := 5
const SCENES := [
	"title",
	"intro_sequence",
	"oak_intro",
	"name_entry",
	"continue_screen",
	"delete_save_screen",
	"clock_reset_screen",
	"day_of_week_screen",
	"ui_shell",
	"overworld",
	"battle",
]
const BOOT_SCENES := [
	"title",
	"intro_sequence",
	"oak_intro",
	"name_entry",
	"continue_screen",
	"delete_save_screen",
	"clock_reset_screen",
	"day_of_week_screen",
]
const GAME_ACTIONS := ["game_up", "game_down", "game_left", "game_right", "game_a", "game_b", "game_start", "game_select"]
const REPO_PATHS_SCRIPT = preload("res://scripts/repo_paths.gd")
const ASSET_INDEX_SCRIPT = preload("res://scripts/asset_index.gd")
const INPUT_LATCH_SCRIPT = preload("res://scripts/input_latch.gd")
const SAVE_STORE_SCRIPT = preload("res://scripts/save_store.gd")

var accumulator_ms := 0.0
var state: Dictionary = {}
var asset_index: Variant = ASSET_INDEX_SCRIPT.new()
var input_latch: Variant = INPUT_LATCH_SCRIPT.new()
var save_store: Variant = SAVE_STORE_SCRIPT.new()
var current_scene_index := 0
var current_scene_route := "title"
var scene_nodes: Dictionary = {}
var pending_scene_handoff: Dictionary = {}
var last_scene_handoff: Dictionary = {}
var _status_label: Label
var _frame_label: Label
var _scene_label: Label
var _assets_label: Label
var _ui_shell_page_signal_bound := false

func _ready() -> void:
	_register_input_actions()
	_bind_labels()
	_bind_scene_nodes()
	_initialize_input_state()
	_reset_state()
	asset_index.initialize()
	state["loaded_asset_summary"] = asset_index.load_summary()
	_sync_scene_state(false)
	_refresh_ui()
	set_process(true)
	set_process_input(true)
	set_process_unhandled_input(true)

func _bind_labels() -> void:
	_status_label = get_node_or_null("Margin/VBox/StatusLabel")
	_frame_label = get_node_or_null("Margin/VBox/FrameLabel")
	_scene_label = get_node_or_null("Margin/VBox/SceneLabel")
	_assets_label = get_node_or_null("Margin/VBox/AssetsLabel")

func _bind_scene_nodes() -> void:
	scene_nodes["title"] = get_node_or_null("Title")
	scene_nodes["intro_sequence"] = get_node_or_null("IntroSequence")
	scene_nodes["oak_intro"] = get_node_or_null("OakIntro")
	scene_nodes["name_entry"] = get_node_or_null("NameEntry")
	scene_nodes["continue_screen"] = get_node_or_null("ContinueScreen")
	scene_nodes["delete_save_screen"] = get_node_or_null("DeleteSaveScreen")
	scene_nodes["clock_reset_screen"] = get_node_or_null("ClockResetScreen")
	scene_nodes["day_of_week_screen"] = get_node_or_null("DayOfWeekScreen")
	scene_nodes["ui_shell"] = get_node_or_null("UIShell")
	scene_nodes["overworld"] = get_node_or_null("Overworld")
	scene_nodes["battle"] = get_node_or_null("Battle")
	for route in scene_nodes.keys():
		var node: Variant = scene_nodes.get(route, null)
		if node != null and node is Object and node.has_method("set_runtime"):
			node.call("set_runtime", self)
	_bind_ui_shell_page_signal()

func _register_input_actions() -> void:
	_ensure_action("game_up", [KEY_UP, KEY_W])
	_ensure_action("game_down", [KEY_DOWN, KEY_S])
	_ensure_action("game_left", [KEY_LEFT, KEY_A])
	_ensure_action("game_right", [KEY_RIGHT, KEY_D])
	_ensure_action("game_a", [KEY_Z, KEY_SPACE])
	_ensure_action("game_b", [KEY_X, KEY_ESCAPE])
	_ensure_action("game_start", [KEY_ENTER, KEY_KP_ENTER])
	_ensure_action("game_select", [KEY_TAB])

func _ensure_action(name: String, keys: Array) -> void:
	if not InputMap.has_action(name):
		InputMap.add_action(name)
	else:
		InputMap.action_erase_events(name)
	for key in keys:
		var event := InputEventKey.new()
		event.keycode = key
		event.physical_keycode = key
		InputMap.action_add_event(name, event)

func _unhandled_input(event: InputEvent) -> void:
	if event is InputEventKey and event.echo:
		return
	for action in GAME_ACTIONS:
		if event.is_action_pressed(action):
			_queue_button(_button_name_for_action(action), true)
		elif event.is_action_released(action):
			_queue_button(_button_name_for_action(action), false)

func _button_name_for_action(action: String) -> String:
	match action:
		"game_up":
			return "up"
		"game_down":
			return "down"
		"game_left":
			return "left"
		"game_right":
			return "right"
		"game_a":
			return "a"
		"game_b":
			return "b"
		"game_start":
			return "start"
		"game_select":
			return "select"
		_:
			return ""

func _process(delta: float) -> void:
	accumulator_ms = min(accumulator_ms + (delta * 1000.0), FRAME_DURATION_MS * MAX_CATCH_UP_FRAMES)
	var steps: int = 0
	while accumulator_ms >= FRAME_DURATION_MS and steps < MAX_CATCH_UP_FRAMES:
		accumulator_ms -= FRAME_DURATION_MS
		_step_simulation()
		steps += 1
	_refresh_ui()

func _step_simulation() -> void:
	_step_simulation_with_input(_begin_frame_input())

func step_fixed_frame(frame_input_override: Dictionary = {}) -> Dictionary:
	var frame_input: Dictionary = _normalize_frame_input(frame_input_override) if not frame_input_override.is_empty() else _begin_frame_input()
	_step_simulation_with_input(frame_input)
	return get_state()

func _step_simulation_with_input(frame_input: Dictionary) -> void:
	state["frame_counter"] = int(state.get("frame_counter", 0)) + 1
	_sync_joypad(frame_input)
	_route_active_scene_input(frame_input)
	_handle_debug_scene_flow()
	_capture_active_scene_state()
	_poll_title_scene_action()
	_poll_boot_scene_action()
	_capture_route_handoff()

func _normalize_frame_input(frame_input: Dictionary) -> Dictionary:
	var down: Dictionary = Dictionary(frame_input.get("down", frame_input.get("held", {}))).duplicate(true)
	var pressed: Dictionary = Dictionary(frame_input.get("pressed", {})).duplicate(true)
	var released: Dictionary = Dictionary(frame_input.get("released", {})).duplicate(true)
	return {
		"down": down,
		"held": down.duplicate(true),
		"pressed": pressed,
		"released": released,
	}

func _sync_joypad(frame_input: Dictionary) -> void:
	var down: Dictionary = Dictionary(frame_input.get("down", {}))
	var pressed: Dictionary = Dictionary(frame_input.get("pressed", {}))
	var released: Dictionary = Dictionary(frame_input.get("released", {}))
	state["last_frame_input"] = {
		"down": down.duplicate(true),
		"pressed": pressed.duplicate(true),
		"released": released.duplicate(true),
	}
	var hram: Dictionary = Dictionary(state.get("hram", {}))
	var joypad: Dictionary = Dictionary(hram.get("joypad", {}))
	joypad["hJoyDown"] = _mask_from_input(down)
	joypad["hJoyPressed"] = _mask_from_input(pressed)
	joypad["hJoyReleased"] = _mask_from_input(released)
	joypad["hJoypadSum"] = int(joypad["hJoyDown"]) | int(joypad["hJoyPressed"])
	joypad["hJoyLast"] = int(joypad["hJoyDown"])
	joypad["hJoyPressedRaw"] = down
	hram["joypad"] = joypad
	state["hram"] = hram

func _route_active_scene_input(frame_input: Dictionary) -> Dictionary:
	var routed: Dictionary = {
		"route": current_scene_route,
		"consumed": false,
		"source": "none",
		"frame": int(state.get("frame_counter", 0)),
	}
	var node: Variant = scene_nodes.get(current_scene_route, null)
	if node != null and node is Object:
		if node.has_method("route_input"):
			routed.merge(Dictionary(node.call("route_input", frame_input)), true)
		elif node.has_method("consume_input"):
			routed.merge(Dictionary(node.call("consume_input", frame_input)), true)
	routed["route"] = current_scene_route
	routed["frame"] = int(state.get("frame_counter", 0))
	state["last_routed_input"] = routed.duplicate(true)
	return routed

func _mask_from_input(buttons: Dictionary) -> int:
	var mask := 0
	if bool(buttons.get("right", false)):
		mask |= 1 << 0
	if bool(buttons.get("left", false)):
		mask |= 1 << 1
	if bool(buttons.get("up", false)):
		mask |= 1 << 2
	if bool(buttons.get("down", false)):
		mask |= 1 << 3
	if bool(buttons.get("a", false)):
		mask |= 1 << 4
	if bool(buttons.get("b", false)):
		mask |= 1 << 5
	if bool(buttons.get("select", false)):
		mask |= 1 << 6
	if bool(buttons.get("start", false)):
		mask |= 1 << 7
	return mask

func _handle_debug_scene_flow() -> void:
	if not bool(state.get("debug_scene_flow_enabled", false)):
		return
	if _is_pressed("start"):
		var next_index: int = (current_scene_index + 1) % SCENES.size()
		request_scene_route(SCENES[next_index], "start")
	if _is_pressed("select"):
		state["last_save_slot"] = "debug-shell"
		save_current_state(str(state.get("last_save_slot", "debug-shell")))
	if _is_pressed("b"):
		load_saved_state(str(state.get("last_save_slot", "debug-shell")))
	if _is_pressed("a"):
		state["has_seen_intro"] = true

func request_scene_route(scene_name: String, reason: String = "runtime") -> void:
	var target_route := _route_for_scene(scene_name)
	if target_route == current_scene_route:
		_sync_scene_state(false)
		return
	var outgoing_state: Dictionary = _capture_scene_state(current_scene_route)
	_store_scene_state(current_scene_route, outgoing_state)
	pending_scene_handoff = _build_scene_handoff(current_scene_route, target_route, reason, outgoing_state)
	state["pending_scene_handoff"] = pending_scene_handoff.duplicate(true)
	last_scene_handoff = pending_scene_handoff.duplicate(true)
	state["scene_handoff"] = last_scene_handoff.duplicate(true)
	_apply_scene_route(target_route)
	_capture_route_handoff()

func route_to_ui_shell(scene_name: String = "title", reason: String = "runtime") -> void:
	var normalized := _normalize_ui_shell_scene_page(scene_name)
	state["ui_page"] = normalized
	_sync_ui_shell_state_page(normalized)
	request_scene_route("ui_shell", reason)

func route_to_overworld(reason: String = "runtime") -> void:
	request_scene_route("overworld", reason)

func route_to_battle(reason: String = "runtime") -> void:
	request_scene_route("battle", reason)

func route_to_title(reason: String = "runtime") -> void:
	request_scene_route("title", reason)

func route_to_intro_sequence(reason: String = "runtime") -> void:
	request_scene_route("intro_sequence", reason)

func route_to_oak_intro(reason: String = "runtime") -> void:
	request_scene_route("oak_intro", reason)

func route_to_name_entry(reason: String = "runtime") -> void:
	request_scene_route("name_entry", reason)

func route_to_continue_screen(reason: String = "runtime") -> void:
	request_scene_route("continue_screen", reason)

func route_to_delete_save_screen(reason: String = "runtime") -> void:
	request_scene_route("delete_save_screen", reason)

func route_to_clock_reset_screen(reason: String = "runtime") -> void:
	request_scene_route("clock_reset_screen", reason)

func route_to_day_of_week_screen(reason: String = "runtime") -> void:
	request_scene_route("day_of_week_screen", reason)

func get_state() -> Dictionary:
	_capture_active_scene_state()
	return _snapshot_state()

func from_state(data: Dictionary) -> void:
	if data.is_empty():
		return
	state = _coerce_loaded_state(data)
	last_scene_handoff = Dictionary(state.get("scene_handoff", {})).duplicate(true)
	if data.has("pending_scene_handoff"):
		pending_scene_handoff = Dictionary(data.get("pending_scene_handoff", {})).duplicate(true)
	elif state.has("pending_scene_handoff"):
		pending_scene_handoff = Dictionary(state.get("pending_scene_handoff", {})).duplicate(true)
	else:
		pending_scene_handoff = {}
	state["scene_handoff"] = last_scene_handoff.duplicate(true)
	state["pending_scene_handoff"] = pending_scene_handoff.duplicate(true)
	_sync_scene_state(false)

func to_dictionary() -> Dictionary:
	return get_state()

func from_dictionary(data: Variant) -> bool:
	if typeof(data) != TYPE_DICTIONARY:
		return false
	from_state(Dictionary(data))
	return true

func _capture_route_handoff() -> void:
	if pending_scene_handoff.is_empty():
		return
	last_scene_handoff = pending_scene_handoff.duplicate(true)
	pending_scene_handoff = {}
	state["scene_handoff"] = last_scene_handoff.duplicate(true)
	state["pending_scene_handoff"] = {}

func _build_scene_handoff(from_scene: String, to_scene: String, reason: String, outgoing_state: Dictionary) -> Dictionary:
	var handoff := {
		"from_scene": from_scene,
		"from_route": current_scene_route,
		"to_scene": to_scene,
		"to_route": to_scene,
		"reason": reason,
		"frame": int(state.get("frame_counter", 0)),
		"save_slot": str(state.get("last_save_slot", "debug-shell")),
		"has_seen_intro": bool(state.get("has_seen_intro", false)),
		"joypad": Dictionary(Dictionary(state.get("hram", {})).get("joypad", {})).duplicate(true),
		"scene_context": _scene_context_for_route(to_scene),
		"state_snapshot": outgoing_state.duplicate(true),
	}
	if from_scene == "title":
		handoff["title_pending_action"] = str(state.get("title_pending_action", ""))
	return handoff

func _apply_scene_route(scene_route: String) -> void:
	current_scene_route = scene_route
	current_scene_index = SCENES.find(current_scene_route)
	if current_scene_index < 0:
		current_scene_index = 0
	state["active_scene"] = current_scene_route
	state["scene_route"] = current_scene_route
	_set_active_scene_name(current_scene_route, false)
	var wram: Dictionary = Dictionary(state.get("wram", {}))
	wram["scene"] = current_scene_route
	wram["scene_route"] = current_scene_route
	wram["scene_transition"] = {
		"from": str(last_scene_handoff.get("from_scene", current_scene_route)),
		"to": current_scene_route,
		"reason": str(last_scene_handoff.get("reason", "runtime")),
	}
	state["wram"] = wram
	state["scene_context"] = _scene_context_for_route(current_scene_route)
	state["scene_handoff"] = last_scene_handoff.duplicate(true)
	state["pending_scene_handoff"] = pending_scene_handoff.duplicate(true)
	if scene_route != "title":
		state["title_pending_action"] = ""
	_refresh_active_scene_node()

func _capture_scene_state(scene_route: String) -> Dictionary:
	if _is_boot_scene_route(scene_route):
		return _capture_generic_scene_state(scene_route)
	match scene_route:
		"overworld":
			return _capture_overworld_state()
		"battle":
			return _capture_battle_state()
		_:
			return _capture_ui_shell_state()

func _capture_active_scene_state() -> void:
	var payload: Dictionary = _capture_scene_state(current_scene_route)
	_store_scene_state(current_scene_route, payload)

func _capture_ui_shell_state() -> Dictionary:
	var shell_node: Variant = scene_nodes.get("ui_shell", null)
	var shell_state: Dictionary = {}
	var ui_page := str(state.get("ui_page", "title"))
	if shell_node != null and shell_node is Object and shell_node.has_method("get_state"):
		shell_state = Dictionary(shell_node.call("get_state"))
		if shell_node.has_method("get_ui_page"):
			ui_page = str(shell_node.call("get_ui_page"))
	return {
		"route": "ui_shell",
		"ui_page": ui_page,
		"ui_dialogue_state": Dictionary(shell_state.get("text_box", state.get("ui_dialogue_state", {}))).duplicate(true),
		"ui_menu_state": Dictionary(shell_state.get("menu_stack", state.get("ui_menu_state", {}))).duplicate(true),
		"ui_shell_state": shell_state.duplicate(true),
	}

func _capture_generic_scene_state(scene_route: String) -> Dictionary:
	var scene_node: Variant = scene_nodes.get(scene_route, null)
	var scene_state: Dictionary = Dictionary(Dictionary(state.get("boot_scene_state", {})).get(scene_route, {}))
	if scene_node != null and scene_node is Object and scene_node.has_method("get_state"):
		scene_state = Dictionary(scene_node.call("get_state"))
	return {
		"route": scene_route,
		"scene_state": scene_state.duplicate(true),
	}

func _capture_overworld_state() -> Dictionary:
	var overworld_state: Dictionary = Dictionary(state.get("overworld_state", {}))
	var overworld_node: Variant = scene_nodes.get("overworld", null)
	if overworld_node != null and overworld_node is Object and overworld_node.has_method("get"):
		var runtime_state: Variant = overworld_node.call("get", "overworld_state")
		if runtime_state != null and runtime_state is Object:
			if runtime_state.has_method("to_dictionary"):
				return Dictionary(runtime_state.call("to_dictionary")).duplicate(true)
			if runtime_state.has_method("get_state"):
				return Dictionary(runtime_state.call("get_state")).duplicate(true)
			var runtime_overworld: Variant = runtime_state
			var current_map_name := str(runtime_overworld.current_map_name)
			if not current_map_name.is_empty():
				overworld_state["map_name"] = current_map_name
			var player_tile: Vector2i = runtime_overworld.player_tile
			overworld_state["player_tile"] = {"x": int(player_tile.x), "y": int(player_tile.y)}
			var player_facing := str(runtime_overworld.player_facing)
			if not player_facing.is_empty():
				overworld_state["player_facing"] = player_facing
			overworld_state["movement_state"] = str(runtime_overworld.movement_state)
			overworld_state["movement_locked"] = bool(runtime_overworld.movement_locked)
			overworld_state["last_move_request"] = {
				"direction": str(runtime_overworld.last_move_request.get("direction", runtime_overworld.last_move_direction)),
				"from_tile": Dictionary(runtime_overworld.last_move_request.get("from_tile", {})).duplicate(true),
				"to_tile": Dictionary(runtime_overworld.last_move_request.get("to_tile", {})).duplicate(true),
				"map_key": str(runtime_overworld.last_move_request.get("map_key", runtime_overworld.current_map_key)),
				"step": int(runtime_overworld.last_move_request.get("step", runtime_overworld.fixed_step_count)),
				"movement_locked": bool(runtime_overworld.last_move_request.get("movement_locked", runtime_overworld.movement_locked)),
			}
			overworld_state["last_move_result"] = {
				"state": str(runtime_overworld.last_move_result.get("state", runtime_overworld.movement_state)),
				"blocked": bool(runtime_overworld.last_move_result.get("blocked", false)),
				"moved": bool(runtime_overworld.last_move_result.get("moved", false)),
				"reason": str(runtime_overworld.last_move_result.get("reason", "")),
				"direction": str(runtime_overworld.last_move_result.get("direction", runtime_overworld.last_move_direction)),
				"from_tile": Dictionary(runtime_overworld.last_move_result.get("from_tile", {})).duplicate(true),
				"to_tile": Dictionary(runtime_overworld.last_move_result.get("to_tile", {})).duplicate(true),
				"step": int(runtime_overworld.last_move_result.get("step", runtime_overworld.fixed_step_count)),
				"collision": bool(runtime_overworld.collision_detected),
			}
			overworld_state["last_collision_result"] = Dictionary({
				"blocked": bool(runtime_overworld.collision_detected),
				"reason": str(runtime_overworld.collision_reason),
			}).duplicate(true)
			overworld_state["last_warp_result"] = Dictionary({
				"requested": bool(runtime_overworld.warp_requested),
				"target": str(runtime_overworld.warp_target),
				"tile": {"x": int(runtime_overworld.player_tile.x), "y": int(runtime_overworld.player_tile.y)},
			}).duplicate(true)
			overworld_state["last_runtime_note"] = str(runtime_overworld.debug_lines.back() if not runtime_overworld.debug_lines.is_empty() else overworld_state.get("last_runtime_note", ""))
			overworld_state["current_spawn_point"] = runtime_overworld.current_spawn_point.duplicate(true)
	return {
		"route": "overworld",
		"map_name": str(overworld_state.get("map_name", "")),
		"player_tile": Dictionary(overworld_state.get("player_tile", {"x": 0, "y": 0})).duplicate(true),
		"player_facing": str(overworld_state.get("player_facing", "down")),
		"movement_state": str(overworld_state.get("movement_state", "idle")),
		"movement_locked": bool(overworld_state.get("movement_locked", false)),
		"last_move_request": Dictionary(overworld_state.get("last_move_request", {})).duplicate(true),
		"last_move_result": Dictionary(overworld_state.get("last_move_result", {})).duplicate(true),
		"last_collision_result": Dictionary(overworld_state.get("last_collision_result", {})).duplicate(true),
		"last_warp_result": Dictionary(overworld_state.get("last_warp_result", {})).duplicate(true),
		"last_runtime_note": str(overworld_state.get("last_runtime_note", "")),
		"current_spawn_point": Dictionary(overworld_state.get("current_spawn_point", {})).duplicate(true),
	}

func _capture_battle_state() -> Dictionary:
	var battle_state: Dictionary = Dictionary(state.get("battle_state", {}))
	var battle_node: Variant = scene_nodes.get("battle", null)
	if battle_node != null and battle_node is Object and battle_node.has_method("get"):
		var runtime_state: Variant = battle_node.call("get", "battle_state")
		if runtime_state != null and runtime_state is Object and runtime_state.has_method("to_dictionary"):
			battle_state = Dictionary(runtime_state.call("to_dictionary"))
			if battle_node.has_method("get_state"):
				var battle_snapshot: Dictionary = Dictionary(battle_node.call("get_state"))
				battle_state["frame_counter"] = int(battle_snapshot.get("battle_frame", battle_state.get("frame_counter", 0)))
			battle_state["asset_summary"] = Dictionary(runtime_state.asset_summary).duplicate(true)
	battle_state["route"] = "battle"
	battle_state["battle_label"] = str(battle_state.get("battle_label", "battle shell"))
	battle_state["battle_kind"] = str(battle_state.get("battle_kind", "wild"))
	battle_state["turn_phase"] = str(battle_state.get("turn_phase", "setup"))
	battle_state["turn_number"] = int(battle_state.get("turn_number", 0))
	battle_state["active_side"] = str(battle_state.get("active_side", "none"))
	battle_state["prompt_gate_active"] = bool(battle_state.get("prompt_gate_active", false))
	battle_state["prompt_gate_reason"] = str(battle_state.get("prompt_gate_reason", ""))
	battle_state["prompt_kind"] = str(battle_state.get("prompt_kind", ""))
	battle_state["prompt_message"] = str(battle_state.get("prompt_message", ""))
	battle_state["prompt_locked"] = bool(battle_state.get("prompt_locked", false))
	battle_state["pending_command"] = Dictionary(battle_state.get("pending_command", {})).duplicate(true)
	battle_state["last_resolved_command"] = Dictionary(battle_state.get("last_resolved_command", {})).duplicate(true)
	battle_state["battle_finished"] = bool(battle_state.get("battle_finished", false))
	battle_state["battle_result"] = str(battle_state.get("battle_result", ""))
	battle_state["frame_counter"] = int(battle_state.get("frame_counter", 0))
	battle_state["fixed_step_count"] = int(battle_state.get("fixed_step_count", 0))
	battle_state["battle_context"] = Dictionary(battle_state.get("battle_context", {})).duplicate(true)
	battle_state["asset_summary"] = Dictionary(battle_state.get("asset_summary", {})).duplicate(true)
	battle_state["log_lines"] = Array(battle_state.get("log_lines", [])).duplicate(true)
	return battle_state.duplicate(true)

func _scene_context_for_route(scene_route: String) -> Dictionary:
	var context: Dictionary = {
		"route": scene_route,
		"frame_counter": int(state.get("frame_counter", 0)),
		"active_scene": str(state.get("active_scene", "ui_shell")),
		"last_save_slot": str(state.get("last_save_slot", "debug-shell")),
	}
	if _is_boot_scene_route(scene_route):
		context["mode"] = "boot"
		context["scene_state"] = Dictionary(Dictionary(state.get("boot_scene_state", {})).get(scene_route, {})).duplicate(true)
		context["player_name"] = str(state.get("player_name", ""))
		context["player_gender"] = str(state.get("player_gender", "male"))
		context["boot_day_of_week"] = int(state.get("boot_day_of_week", 0))
		context["boot_time_hour"] = int(state.get("boot_time_hour", 0))
		context["boot_time_minute"] = int(state.get("boot_time_minute", 0))
		return context
	if scene_route == "overworld":
		var overworld_state: Dictionary = Dictionary(state.get("overworld_state", {}))
		context["mode"] = "explore"
		context["map_name"] = str(overworld_state.get("map_name", ""))
		context["player_tile"] = Dictionary(overworld_state.get("player_tile", {"x": 0, "y": 0})).duplicate(true)
		context["player_facing"] = str(overworld_state.get("player_facing", "down"))
		context["movement_state"] = str(overworld_state.get("movement_state", "idle"))
		context["movement_locked"] = bool(overworld_state.get("movement_locked", false))
	elif scene_route == "ui_shell":
		context["mode"] = "menu"
		context["ui_page"] = str(state.get("ui_page", "title"))
		context["ui_shell_state"] = Dictionary(state.get("ui_shell_state", {})).duplicate(true)
	elif scene_route == "battle":
		var battle_state: Dictionary = Dictionary(state.get("battle_state", {}))
		context["mode"] = "combat"
		context["battle_state"] = battle_state.duplicate(true)
	else:
		context["mode"] = "menu"
		context["ui_page"] = str(state.get("ui_page", "title"))
		context["ui_dialogue_state"] = Dictionary(state.get("ui_dialogue_state", {})).duplicate(true)
		context["ui_menu_state"] = Dictionary(state.get("ui_menu_state", {})).duplicate(true)
	return context

func _store_scene_state(scene_route: String, payload: Dictionary) -> void:
	if _is_boot_scene_route(scene_route):
		var boot_scene_state: Dictionary = Dictionary(state.get("boot_scene_state", {}))
		var scene_state: Dictionary = Dictionary(payload.get("scene_state", payload)).duplicate(true)
		if scene_route == "title":
			var title_action := str(state.get("title_pending_action", ""))
			if not title_action.is_empty():
				scene_state["pending_action"] = title_action
		boot_scene_state[scene_route] = scene_state.duplicate(true)
		state["boot_scene_state"] = boot_scene_state
		if scene_route == "name_entry":
			state["player_name"] = str(scene_state.get("name", state.get("player_name", "")))
		if scene_route == "oak_intro":
			state["player_gender"] = str(scene_state.get("gender", state.get("player_gender", "male")))
		if scene_route == "clock_reset_screen":
			state["boot_day_of_week"] = int(scene_state.get("day", state.get("boot_day_of_week", 0))) % 7
			state["boot_time_hour"] = clampi(int(scene_state.get("hour", state.get("boot_time_hour", 0))), 0, 23)
			state["boot_time_minute"] = clampi(int(scene_state.get("minute", state.get("boot_time_minute", 0))), 0, 59)
		if scene_route == "day_of_week_screen":
			state["boot_day_of_week"] = int(scene_state.get("selected_day", state.get("boot_day_of_week", 0))) % 7
		return
	match scene_route:
		"overworld":
			state["overworld_state"] = payload.duplicate(true)
		"battle":
			state["battle_state"] = payload.duplicate(true)
		_:
			var ui_page := str(payload.get("ui_page", state.get("ui_page", "title")))
			state["ui_page"] = ui_page
			state["ui_dialogue_state"] = Dictionary(payload.get("ui_dialogue_state", payload.get("text_box", {}))).duplicate(true)
			state["ui_menu_state"] = Dictionary(payload.get("ui_menu_state", payload.get("menu_stack", {}))).duplicate(true)
			var ui_shell_state: Dictionary = Dictionary(payload.get("ui_shell_state", {})).duplicate(true)
			if ui_shell_state.is_empty():
				ui_shell_state = {
					"ui_page": ui_page,
					"text_box": Dictionary(state["ui_dialogue_state"]).duplicate(true),
					"menu_stack": Dictionary(state["ui_menu_state"]).duplicate(true),
				}
			else:
				ui_shell_state["ui_page"] = ui_page
				ui_shell_state["text_box"] = Dictionary(state["ui_dialogue_state"]).duplicate(true)
				ui_shell_state["menu_stack"] = Dictionary(state["ui_menu_state"]).duplicate(true)
			state["ui_shell_state"] = ui_shell_state

func _is_boot_scene_route(scene_route: String) -> bool:
	return BOOT_SCENES.has(scene_route)

func _refresh_active_scene_node() -> void:
	for route in SCENES:
		var node: Variant = scene_nodes.get(route, null)
		if node == null or not (node is CanvasItem):
			continue
		node.visible = route == current_scene_route
		if node.has_method("set_process"):
			node.set_process(route == current_scene_route)
		if node.has_method("set_process_unhandled_input"):
			node.set_process_unhandled_input(route == current_scene_route)
	if current_scene_route == "ui_shell":
		_apply_ui_shell_state()
	elif current_scene_route == "overworld":
		_apply_overworld_state()
	elif current_scene_route == "battle":
		_apply_battle_state()
	elif _is_boot_scene_route(current_scene_route):
		_apply_boot_scene_state(current_scene_route)

func _apply_ui_shell_state() -> void:
	var shell_node: Variant = scene_nodes.get("ui_shell", null)
	if shell_node == null or not (shell_node is Object):
		return
	var payload: Dictionary = Dictionary(state.get("ui_shell_state", {}))
	if payload.is_empty():
		payload = {
			"ui_page": str(state.get("ui_page", "title")),
			"text_box": Dictionary(state.get("ui_dialogue_state", {})).duplicate(true),
			"menu_stack": Dictionary(state.get("ui_menu_state", {})).duplicate(true),
		}
	else:
		payload["ui_page"] = str(state.get("ui_page", payload.get("ui_page", "title")))
		payload["text_box"] = Dictionary(payload.get("text_box", state.get("ui_dialogue_state", {}))).duplicate(true)
		payload["menu_stack"] = Dictionary(payload.get("menu_stack", state.get("ui_menu_state", {}))).duplicate(true)
	if shell_node.has_method("from_state"):
		shell_node.call("from_state", payload)
		_bind_ui_shell_page_signal()
		return
	if shell_node.has_method("reset"):
		shell_node.call("reset")
	if shell_node.has_method("set_ui_page"):
		shell_node.call("set_ui_page", str(payload.get("ui_page", "title")))
	_bind_ui_shell_page_signal()

func _bind_ui_shell_page_signal() -> void:
	if _ui_shell_page_signal_bound:
		return
	var shell_node: Variant = scene_nodes.get("ui_shell", null)
	if shell_node == null or not (shell_node is Object):
		return
	if not shell_node.has_signal("ui_page_changed"):
		return
	var callback := Callable(self, "_on_ui_shell_page_changed")
	if shell_node.is_connected("ui_page_changed", callback):
		_ui_shell_page_signal_bound = true
		return
	shell_node.connect("ui_page_changed", callback)
	_ui_shell_page_signal_bound = true

func _on_ui_shell_page_changed(page_name: String) -> void:
	var normalized := _normalize_ui_shell_scene_page(page_name)
	state["ui_page"] = normalized
	_sync_ui_shell_state_page(normalized)
	var context: Dictionary = Dictionary(state.get("scene_context", {}))
	context["ui_page"] = normalized
	state["scene_context"] = context

func _apply_overworld_state() -> void:
	var overworld_node: Variant = scene_nodes.get("overworld", null)
	if overworld_node == null or not (overworld_node is Object):
		return
	var payload: Dictionary = Dictionary(state.get("overworld_state", {}))
	var runtime_state: Variant = overworld_node.get("overworld_state")
	if runtime_state != null and runtime_state is Object:
		if runtime_state.has_method("from_state"):
			runtime_state.call("from_state", payload)
		elif runtime_state.has_method("from_dictionary"):
			runtime_state.call("from_dictionary", payload)
		else:
			var map_name := str(payload.get("map_name", ""))
			if map_name.is_empty():
				if runtime_state.has_method("load_default_map"):
					runtime_state.call("load_default_map")
			elif runtime_state.has_method("load_map"):
				runtime_state.call("load_map", map_name)
			var tile := Dictionary(payload.get("player_tile", {"x": 0, "y": 0}))
			if runtime_state.has_method("set_player_position"):
				runtime_state.call("set_player_position", int(tile.get("x", 0)), int(tile.get("y", 0)))
			else:
				runtime_state.set("player_tile", Vector2i(int(tile.get("x", 0)), int(tile.get("y", 0))))
			if runtime_state.has_method("set_player_facing"):
				runtime_state.call("set_player_facing", str(payload.get("player_facing", "down")))
			else:
				runtime_state.set("player_facing", str(payload.get("player_facing", "down")))
			runtime_state.set("movement_state", str(payload.get("movement_state", "idle")))
			runtime_state.set("movement_locked", bool(payload.get("movement_locked", false)))
			runtime_state.set("last_move_request", Dictionary(payload.get("last_move_request", {})).duplicate(true))
			runtime_state.set("last_move_result", Dictionary(payload.get("last_move_result", {})).duplicate(true))
			runtime_state.set("last_collision_result", Dictionary(payload.get("last_collision_result", {})).duplicate(true))
			runtime_state.set("last_warp_result", Dictionary(payload.get("last_warp_result", {})).duplicate(true))
			runtime_state.set("last_runtime_note", str(payload.get("last_runtime_note", "")))
			runtime_state.set("current_spawn_point", Dictionary(payload.get("current_spawn_point", {})).duplicate(true))
	if overworld_node.has_method("_refresh_ui"):
		overworld_node.call("_refresh_ui", true)

func _apply_boot_scene_state(scene_route: String) -> void:
	var scene_node: Variant = scene_nodes.get(scene_route, null)
	if scene_node == null or not (scene_node is Object):
		return
	var payload: Dictionary = Dictionary(Dictionary(state.get("boot_scene_state", {})).get(scene_route, {}))
	if payload.is_empty():
		payload = Dictionary(state.get("boot_scene_state", {})).get(scene_route, {})
	payload["route_entry"] = true
	if scene_node.has_method("from_state"):
		scene_node.call("from_state", payload)
	elif scene_node.has_method("from_dictionary"):
		scene_node.call("from_dictionary", payload)

func _apply_battle_state() -> void:
	var battle_node: Variant = scene_nodes.get("battle", null)
	if battle_node == null or not (battle_node is Object):
		return
	var payload: Dictionary = Dictionary(state.get("battle_state", {}))
	if battle_node.has_method("from_state"):
		battle_node.call("from_state", {
			"battle_state": payload,
			"battle_frame": int(payload.get("frame_counter", 0)),
		})
		return
	var runtime_state: Variant = battle_node.get("battle_state")
	if runtime_state != null and runtime_state is Object:
		if runtime_state.has_method("from_dictionary"):
			runtime_state.call("from_dictionary", payload)
		battle_node.set("battle_frame", int(payload.get("frame_counter", 0)))
		battle_node.set("accumulator_ms", 0.0)
		if battle_node.has_method("_refresh_ui"):
			battle_node.call("_refresh_ui", true)

func _normalize_ui_shell_scene_page(scene_name: String) -> String:
	var normalized := scene_name.strip_edges().to_lower()
	return normalized if not normalized.is_empty() else "title"

func _poll_title_scene_action() -> void:
	if current_scene_route != "title":
		return
	var title_node: Variant = scene_nodes.get("title", null)
	if title_node == null or not (title_node is Object) or not title_node.has_method("pop_action"):
		return
	var action_value: Variant = title_node.call("pop_action")
	if action_value == null:
		return
	var action := str(action_value).strip_edges()
	if action.is_empty():
		return
	state["title_pending_action"] = action
	if _route_for_scene(action) == current_scene_route:
		return
	request_scene_route(action, "title_action")

func _poll_boot_scene_action() -> void:
	if not _is_boot_scene_route(current_scene_route) or current_scene_route == "title":
		return
	var boot_node: Variant = scene_nodes.get(current_scene_route, null)
	if boot_node == null or not (boot_node is Object) or not boot_node.has_method("pop_action"):
		return
	var action_value: Variant = boot_node.call("pop_action")
	if action_value == null:
		return
	var action := str(action_value).strip_edges()
	if action.is_empty():
		return
	request_scene_route(action, "boot_action")

func _route_for_scene(scene_name: String) -> String:
	if _is_boot_scene_route(scene_name):
		return scene_name
	if scene_name == "overworld":
		return "overworld"
	if scene_name == "battle":
		return "battle"
	return "ui_shell"

func _sync_scene_state(record_history: bool) -> void:
	var route := _route_for_scene(str(state.get("active_scene", "ui_shell")))
	current_scene_route = route
	current_scene_index = SCENES.find(route)
	if current_scene_index < 0:
		current_scene_index = 0
	_set_active_scene_name(route, record_history)
	state["scene_route"] = current_scene_route
	state["scene_context"] = _scene_context_for_route(current_scene_route)
	state["scene_handoff"] = last_scene_handoff.duplicate(true)
	state["pending_scene_handoff"] = pending_scene_handoff.duplicate(true)
	_refresh_active_scene_node()

func _set_active_scene_index(index: int) -> void:
	var clamped_index: int = index
	if clamped_index < 0:
		clamped_index = 0
	elif clamped_index >= SCENES.size():
		clamped_index = SCENES.size() - 1
	request_scene_route(SCENES[clamped_index], "cycle")

func _set_active_scene_name(scene_name: String, record_history: bool) -> void:
	var next_route := _route_for_scene(scene_name)
	current_scene_index = SCENES.find(next_route)
	if current_scene_index < 0:
		current_scene_index = 0
	state["active_scene"] = next_route
	var wram: Dictionary = Dictionary(state.get("wram", {}))
	wram["scene"] = next_route
	state["wram"] = wram
	if record_history:
		var scene_history: Array = Array(state.get("scene_history", []))
		if scene_history.is_empty() or str(scene_history.back()) != next_route:
			scene_history.append(next_route)
		state["scene_history"] = scene_history

func _refresh_ui() -> void:
	if is_instance_valid(_status_label):
		_status_label.text = "Status: fixed-step runtime coordinator"
	if is_instance_valid(_frame_label):
		_frame_label.text = "Frame: %d" % int(state.get("frame_counter", 0))
	if is_instance_valid(_scene_label):
		var ui_page := str(state.get("ui_page", "title"))
		_scene_label.text = "Scene: %s / %s (%d/%d)" % [str(state.get("active_scene", "ui_shell")), ui_page, current_scene_index + 1, SCENES.size()]
	if is_instance_valid(_assets_label):
		var summary: Dictionary = Dictionary(state.get("loaded_asset_summary", {}))
		_assets_label.text = "Assets: %d pokemon, %d moves, %d items, %d packs" % [
			int(summary.get("pokemon_count", 0)),
			int(summary.get("move_count", 0)),
			int(summary.get("item_count", 0)),
			int(summary.get("content_pack_count", 0)),
		]
	_refresh_active_scene_node()

func get_scene_route() -> String:
	return current_scene_route

func get_scene_context() -> Dictionary:
	return Dictionary(state.get("scene_context", _scene_context_for_route(current_scene_route))).duplicate(true)

func get_scene_handoff() -> Dictionary:
	return Dictionary(state.get("scene_handoff", last_scene_handoff)).duplicate(true)

func get_pending_scene_handoff() -> Dictionary:
	return Dictionary(state.get("pending_scene_handoff", pending_scene_handoff)).duplicate(true)

func get_loaded_asset_summary() -> Dictionary:
	return Dictionary(state.get("loaded_asset_summary", _create_asset_summary())).duplicate(true)

func get_ui_page() -> String:
	return str(state.get("ui_page", "title"))

func get_last_frame_input() -> Dictionary:
	return Dictionary(state.get("last_frame_input", {"down": {}, "pressed": {}, "released": {}})).duplicate(true)

func get_last_routed_input() -> Dictionary:
	return Dictionary(state.get("last_routed_input", {"route": current_scene_route, "consumed": false, "source": "none", "frame": 0})).duplicate(true)

func set_player_name(name: String) -> void:
	state["player_name"] = name.strip_edges()

func set_player_gender(gender: String) -> void:
	state["player_gender"] = gender.strip_edges().to_lower()

func set_boot_day_of_week(day: int) -> void:
	state["boot_day_of_week"] = int(day) % 7

func set_boot_time(day: int, hour: int, minute: int) -> void:
	state["boot_day_of_week"] = int(day) % 7
	state["boot_time_hour"] = clampi(int(hour), 0, 23)
	state["boot_time_minute"] = clampi(int(minute), 0, 59)

func _create_default_state() -> Dictionary:
	return {
		"sram": {
			"options": {
				"text_speed": "fast",
				"battle_scene": true,
				"battle_style": "shift",
				"sound": "stereo",
				"menu_account": true,
				"frame": 1,
			},
			"party": {"pokemon": [null, null, null, null, null, null]},
			"link_battle_stats": {"wins": 0, "losses": 0, "draws": 0},
			"badges": {
				"johto": [false, false, false, false, false, false, false, false],
				"kanto": [false, false, false, false, false, false, false, false],
			},
		},
		"wram": {
			"scene": "title",
			"scene_route": "title",
			"scene_transition": {},
			"flags": {},
			"variables": {},
		},
		"vram": {
			"palette_bank": 0,
			"tile_cache_ready": false,
		},
		"hram": {
			"joypad": _create_joypad_state(),
			"hardware_divider": 0,
			"hRandomAdd": 0,
			"hRandomSub": 0,
		},
		"frame_counter": 0,
		"has_seen_intro": false,
		"active_scene": "title",
		"scene_route": "title",
		"scene_context": {},
			"scene_handoff": {},
			"pending_scene_handoff": {},
			"last_frame_input": {"down": {}, "pressed": {}, "released": {}},
			"last_routed_input": {"route": "title", "consumed": false, "source": "none", "frame": 0},
			"title_pending_action": "",
		"scene_history": ["title"],
		"boot_scene_state": {
			"title": {},
			"intro_sequence": {},
			"oak_intro": {},
			"name_entry": {},
			"continue_screen": {},
			"delete_save_screen": {},
			"clock_reset_screen": {},
			"day_of_week_screen": {},
		},
		"ui_page": "title",
		"ui_dialogue_state": {},
		"ui_menu_state": {},
		"ui_shell_state": {
			"ui_page": "title",
			"text_box": {},
			"menu_stack": {},
			"page_snapshots": {},
		},
		"overworld_state": {
			"map_name": "",
			"map_constant": "",
			"current_map_key": "",
			"selected_map_key": "",
			"selected_map_index": -1,
			"available_map_keys": [],
			"map_manifest": {},
			"map_blocks": {},
			"map_scenes": {},
			"map_scene_indices": {},
			"scene_name": "",
			"current_map_group_name": "",
			"current_map_environment": "",
			"current_map_block_key": "",
			"current_width": 0,
			"current_height": 0,
			"current_group_id": -1,
			"current_map_id": -1,
			"current_phone_service": 0,
			"current_tileset_name": "",
			"current_location": "",
			"map_dimensions": {"x": 0, "y": 0},
			"player_tile": {"x": 0, "y": 0},
			"player_facing": "down",
			"movement_state": "idle",
			"movement_locked": false,
			"collision_detected": false,
			"collision_reason": "",
			"warp_requested": false,
			"warp_target": "",
			"last_move_direction": "none",
			"last_move_request": {},
			"last_move_result": {},
			"last_collision_result": {},
			"last_warp_result": {},
			"last_runtime_note": "",
			"fixed_step_count": 0,
			"map_summary": {},
			"spawn_summary": {},
			"current_spawn_point": {},
			"current_connections": [],
			"current_warps": [],
			"current_bg_events": [],
			"current_object_events": [],
			"pending_move": "none",
			"reload_map_after_battle": false,
			"music_request": {},
			"follow_state": {},
			"object_states": {},
			"special_state": {},
			"player_object": {},
			"debug_lines": [],
		},
		"battle_state": {
			"battle_id": "",
			"battle_kind": "wild",
			"battle_label": "battle shell",
			"turn_phase": "setup",
			"turn_number": 0,
			"active_side": "none",
			"prompt_gate_active": false,
			"prompt_gate_reason": "",
			"prompt_kind": "",
			"prompt_message": "",
			"prompt_locked": false,
			"pending_command": {},
			"last_resolved_command": {},
			"battle_finished": false,
			"battle_result": "",
			"frame_counter": 0,
			"fixed_step_count": 0,
			"battle_context": {},
			"asset_summary": _create_asset_summary(),
			"log_lines": ["battle shell ready"],
		},
		"last_save_slot": "debug-shell",
		"player_name": "",
		"player_gender": "male",
		"boot_day_of_week": 0,
			"boot_time_hour": 0,
			"boot_time_minute": 0,
			"loaded_asset_summary": _create_asset_summary(),
			"save_metadata": {},
			"debug_scene_flow_enabled": false,
		}

func _create_joypad_state() -> Dictionary:
	return {
		"hJoypadReleased": 0,
		"hJoypadPressed": 0,
		"hJoypadDown": 0,
		"hJoypadSum": 0,
		"hJoyReleased": 0,
		"hJoyPressed": 0,
		"hJoyDown": 0,
		"hJoyLast": 0,
	}

func _create_asset_summary() -> Dictionary:
	return {
		"pokemon_count": 0,
		"move_count": 0,
		"item_count": 0,
		"content_pack_count": 0,
	}

func _reset_state() -> void:
	state = _create_default_state()
	current_scene_index = 0
	current_scene_route = "title"
	pending_scene_handoff = {}
	last_scene_handoff = {}

func _initialize_input_state() -> void:
	input_latch = INPUT_LATCH_SCRIPT.new()

func _queue_button(button: String, is_pressed: bool) -> void:
	if input_latch == null:
		input_latch = INPUT_LATCH_SCRIPT.new()
	input_latch.queue_button(button, is_pressed)

func _begin_frame_input() -> Dictionary:
	if input_latch == null:
		input_latch = INPUT_LATCH_SCRIPT.new()
	return Dictionary(input_latch.begin_frame())

func _is_pressed(button: String) -> bool:
	if input_latch == null:
		return false
	return bool(input_latch.is_pressed(button))

func _ensure_save_root() -> void:
	var absolute: String = ProjectSettings.globalize_path(REPO_PATHS_SCRIPT.saves_root())
	if not DirAccess.dir_exists_absolute(absolute):
		DirAccess.make_dir_recursive_absolute(absolute)

func _slot_path(slot: String) -> String:
	var normalized := slot.strip_edges()
	if normalized.is_empty():
		normalized = "debug-shell"
	if not normalized.ends_with(".sav"):
		normalized += ".sav"
	return REPO_PATHS_SCRIPT.saves_root().path_join(normalized)

func _save_state(slot: String) -> bool:
	return save_current_state(slot)

func save_current_state(slot: String = "") -> bool:
	var target_slot := slot if not slot.strip_edges().is_empty() else str(state.get("last_save_slot", "debug-shell"))
	if save_store == null:
		save_store = SAVE_STORE_SCRIPT.new()
	var is_manual_save_slot := bool(save_store.is_manual_save_slot(target_slot))
	var is_autosave_slot := bool(save_store.is_autosave_slot(target_slot))
	if is_manual_save_slot:
		var history_slots: Array = save_store.manual_save_history_slots()
		var manual_saved := bool(save_store.save_game_with_history(target_slot, history_slots, self))
		if manual_saved and not is_autosave_slot:
			state["last_save_slot"] = target_slot
		return manual_saved
	var saved := bool(save_store.save_game(target_slot, self))
	if saved and not is_autosave_slot:
		state["last_save_slot"] = target_slot
	return saved

func load_saved_state(slot: String = "") -> bool:
	var target_slot := slot if not slot.strip_edges().is_empty() else str(state.get("last_save_slot", "debug-shell"))
	if save_store == null:
		save_store = SAVE_STORE_SCRIPT.new()
	var save_path: String = save_store.slot_path(target_slot)
	var absolute_save_path: String = ProjectSettings.globalize_path(save_path)
	var absolute_backup_path: String = absolute_save_path + ".bak"
	if _load_state_from_path(absolute_save_path):
		if not bool(save_store.is_autosave_slot(target_slot)):
			state["last_save_slot"] = target_slot
		return true
	if _load_state_from_path(absolute_backup_path):
		if not bool(save_store.is_autosave_slot(target_slot)):
			state["last_save_slot"] = target_slot
		if not DirAccess.copy_absolute(absolute_backup_path, absolute_save_path):
			return true
		return true
	return false

func _load_state_from_path(absolute_path: String) -> bool:
	if not FileAccess.file_exists(absolute_path):
		return false
	var file := FileAccess.open(absolute_path, FileAccess.READ)
	if file == null:
		return false
	var parsed: Variant = JSON.parse_string(file.get_as_text())
	if typeof(parsed) != TYPE_DICTIONARY:
		return false
	var loaded_state: Dictionary = Dictionary(parsed)
	var metadata: Dictionary = _read_save_metadata_for_path(absolute_path)
	if not metadata.is_empty():
		loaded_state["save_metadata"] = metadata
	from_state(loaded_state)
	return true

func _read_save_metadata_for_path(absolute_path: String) -> Dictionary:
	var metadata_path: String = absolute_path + ".meta.json"
	if not FileAccess.file_exists(metadata_path):
		return {}
	var file := FileAccess.open(metadata_path, FileAccess.READ)
	if file == null:
		return {}
	var parsed: Variant = JSON.parse_string(file.get_as_text())
	if typeof(parsed) != TYPE_DICTIONARY:
		return {}
	var source: Dictionary = Dictionary(parsed)
	var saved_at: String = str(source.get("saved_at", "")).strip_edges()
	if saved_at.is_empty():
		return {}
	var slot: String = str(source.get("slot", absolute_path.get_file())).strip_edges()
	if slot.is_empty():
		slot = absolute_path.get_file()
	var kind: String = str(source.get("kind", _save_slot_kind(slot))).strip_edges()
	if kind.is_empty():
		kind = _save_slot_kind(slot)
	return {
		"slot": slot,
		"kind": kind,
		"saved_at": saved_at,
		"frame_counter": max(0, int(source.get("frame_counter", 0))),
	}

func _save_slot_kind(slot: String) -> String:
	var file_name: String = slot.get_file()
	if not file_name.ends_with(".sav"):
		file_name += ".sav"
	if file_name == "savegame.sav" or file_name.begins_with("savegame-recent-"):
		return "manual"
	if file_name == "autosave.sav":
		return "autosave"
	return "custom"

func delete_saved_state(slot: String = "") -> bool:
	var target_slot := slot if not slot.strip_edges().is_empty() else str(state.get("last_save_slot", "debug-shell"))
	if save_store == null:
		save_store = SAVE_STORE_SCRIPT.new()
	return bool(save_store.delete_save_game(target_slot))

func _coerce_loaded_state(data: Dictionary) -> Dictionary:
	var next_state: Dictionary = _create_default_state()
	next_state["sram"] = Dictionary(data.get("sram", next_state["sram"]))
	next_state["wram"] = Dictionary(data.get("wram", next_state["wram"]))
	next_state["vram"] = Dictionary(data.get("vram", next_state["vram"]))
	next_state["hram"] = Dictionary(data.get("hram", next_state["hram"]))
	next_state["frame_counter"] = int(data.get("frame_counter", 0))
	next_state["has_seen_intro"] = bool(data.get("has_seen_intro", false))
	next_state["active_scene"] = _route_for_scene(str(data.get("active_scene", "ui_shell")))
	next_state["scene_route"] = _route_for_scene(str(data.get("scene_route", next_state["active_scene"])))
	next_state["scene_context"] = Dictionary(data.get("scene_context", {}))
	next_state["scene_handoff"] = Dictionary(data.get("scene_handoff", {}))
	next_state["pending_scene_handoff"] = Dictionary(data.get("pending_scene_handoff", {}))
	next_state["last_frame_input"] = Dictionary(data.get("last_frame_input", next_state["last_frame_input"])).duplicate(true)
	next_state["last_routed_input"] = Dictionary(data.get("last_routed_input", next_state["last_routed_input"])).duplicate(true)
	next_state["title_pending_action"] = str(data.get("title_pending_action", next_state["title_pending_action"]))
	next_state["scene_history"] = []
	for entry in data.get("scene_history", [next_state["active_scene"]]):
		next_state["scene_history"].append(_route_for_scene(str(entry)))
	if next_state["scene_history"].is_empty():
		next_state["scene_history"] = [next_state["active_scene"]]
	next_state["ui_page"] = _normalize_ui_shell_scene_page(str(data.get("ui_page", next_state["ui_page"])))
	next_state["ui_dialogue_state"] = Dictionary(data.get("ui_dialogue_state", next_state["ui_dialogue_state"]))
	next_state["ui_menu_state"] = Dictionary(data.get("ui_menu_state", next_state["ui_menu_state"]))
	next_state["ui_shell_state"] = _coerce_ui_shell_state(data, next_state)
	next_state["boot_scene_state"] = _coerce_boot_scene_state(data, next_state)
	next_state["overworld_state"] = Dictionary(data.get("overworld_state", next_state["overworld_state"]))
	next_state["battle_state"] = Dictionary(data.get("battle_state", next_state["battle_state"]))
	next_state["player_name"] = str(data.get("player_name", ""))
	next_state["player_gender"] = str(data.get("player_gender", "male"))
	next_state["boot_day_of_week"] = int(data.get("boot_day_of_week", 0))
	next_state["boot_time_hour"] = clampi(int(data.get("boot_time_hour", 0)), 0, 23)
	next_state["boot_time_minute"] = clampi(int(data.get("boot_time_minute", 0)), 0, 59)
	next_state["last_save_slot"] = str(data.get("last_save_slot", "debug-shell"))
	next_state["loaded_asset_summary"] = Dictionary(data.get("loaded_asset_summary", next_state["loaded_asset_summary"]))
	next_state["save_metadata"] = Dictionary(data.get("save_metadata", next_state.get("save_metadata", {}))).duplicate(true)
	next_state["debug_scene_flow_enabled"] = bool(data.get("debug_scene_flow_enabled", false))
	return next_state

func _snapshot_state() -> Dictionary:
	var snapshot_state: Dictionary = state.duplicate(true)
	snapshot_state["scene_route"] = current_scene_route
	snapshot_state["active_scene"] = current_scene_route
	snapshot_state["scene_context"] = _scene_context_for_route(current_scene_route)
	snapshot_state["scene_handoff"] = last_scene_handoff.duplicate(true)
	snapshot_state["pending_scene_handoff"] = pending_scene_handoff.duplicate(true)
	return snapshot_state

func _coerce_ui_shell_state(data: Dictionary, next_state: Dictionary) -> Dictionary:
	var source: Dictionary = Dictionary(data.get("ui_shell_state", {}))
	var ui_page := _normalize_ui_shell_scene_page(str(data.get("ui_page", next_state.get("ui_page", "title"))))
	var text_box_state: Dictionary = Dictionary(data.get("ui_dialogue_state", data.get("ui_text_box_state", next_state.get("ui_dialogue_state", {}))))
	var menu_stack_state: Dictionary = Dictionary(data.get("ui_menu_state", data.get("ui_menu_stack_state", next_state.get("ui_menu_state", {}))))
	var shell_state: Dictionary = source.duplicate(true) if not source.is_empty() else {}
	if shell_state.is_empty():
		return {
			"ui_page": ui_page,
			"text_box": text_box_state.duplicate(true),
			"menu_stack": menu_stack_state.duplicate(true),
			"page_snapshots": {},
		}
	shell_state["ui_page"] = ui_page
	shell_state["text_box"] = Dictionary(shell_state.get("text_box", text_box_state)).duplicate(true)
	shell_state["menu_stack"] = Dictionary(shell_state.get("menu_stack", menu_stack_state)).duplicate(true)
	shell_state["page_snapshots"] = Dictionary(shell_state.get("page_snapshots", {})).duplicate(true)
	return shell_state

func _coerce_boot_scene_state(data: Dictionary, next_state: Dictionary) -> Dictionary:
	var default_state: Dictionary = Dictionary(next_state.get("boot_scene_state", {}))
	var source: Dictionary = Dictionary(data.get("boot_scene_state", {}))
	var boot_scene_state: Dictionary = default_state.duplicate(true)
	for route in BOOT_SCENES:
		var route_state: Dictionary = Dictionary(source.get(route, boot_scene_state.get(route, {})))
		boot_scene_state[route] = route_state.duplicate(true)
	return boot_scene_state

func _sync_ui_shell_state_page(page_name: String) -> void:
	var shell_state: Dictionary = Dictionary(state.get("ui_shell_state", {}))
	shell_state["ui_page"] = page_name
	state["ui_shell_state"] = shell_state
