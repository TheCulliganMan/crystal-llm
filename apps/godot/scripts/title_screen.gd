extends "res://scripts/boot_scene_base.gd"

const INTRO_ROUTE := "intro_sequence"
const CONTINUE_ROUTE := "continue_screen"
const DELETE_ROUTE := "delete_save_screen"
const CLOCK_ROUTE := "clock_reset_screen"
const ENTRANCE_FRAMES := 30
const MAIN_INPUT_GATE_FRAMES := 8
const ATTRACT_TIMEOUT_FRAMES := 180
const EXIT_HOLD_FRAMES := 12

func can_accept_input() -> bool:
	return super.can_accept_input() and str(state.get("phase", "entrance")) == "main" and int(state.get("phase_frame", 0)) >= int(state.get("input_gate_frames", MAIN_INPUT_GATE_FRAMES))

func _on_ready() -> void:
	if state.is_empty():
		state = _default_screen_state("title")
		state.merge({
			"phase": "entrance",
			"phase_frame": 0,
			"title_timer": 0,
			"input_gate_frames": ENTRANCE_FRAMES,
			"pending_action": "",
			"pending_action_payload": {},
			"attract_timeout_frames": ATTRACT_TIMEOUT_FRAMES,
			"clock_reset_trigger": false,
		}, true)
	_refresh_labels()

func _tick(_delta: float) -> void:
	if bool(state.get("input_locked", false)):
		return
	state["phase_frame"] = int(state.get("phase_frame", 0)) + 1
	state["title_timer"] = int(state.get("title_timer", 0)) + 1
	var phase := str(state.get("phase", "entrance"))
	if phase == "entrance" and int(state.get("phase_frame", 0)) >= int(state.get("input_gate_frames", ENTRANCE_FRAMES)):
		_enter_phase("main", MAIN_INPUT_GATE_FRAMES)
		return
	if phase == "main" and int(state.get("title_timer", 0)) >= int(state.get("attract_timeout_frames", ATTRACT_TIMEOUT_FRAMES)):
		_queue_action(INTRO_ROUTE, "title_timeout", "restart_intro")
		return
	if phase == "exiting" and int(state.get("phase_frame", 0)) >= EXIT_HOLD_FRAMES:
		state["input_locked"] = true

func _handle_boot_input(_event: InputEvent) -> void:
	var phase := str(state.get("phase", "entrance"))
	if phase != "main" or str(state.get("pending_action", "")) != "":
		return
	if _input_pressed(_event, ["a", "start"]):
		_queue_action(INTRO_ROUTE, "title_new_game", "new_game")
		return
	if _input_pressed(_event, ["b"]):
		_queue_action(CONTINUE_ROUTE, "title_continue", "continue")
		return
	if (_last_down("select") or _input_pressed(_event, ["select"])) and (_last_down("up") or _input_pressed(_event, ["up"])):
		_queue_action(DELETE_ROUTE, "title_delete_save", "delete_save")
		return
	if (_last_down("select") or _input_pressed(_event, ["select"])) and (_last_down("down") or _input_pressed(_event, ["down"])):
		_queue_action(CLOCK_ROUTE, "title_clock_reset", "reset_clock")
		return

func _queue_action(route_name: String, action_id: String, selected_option: String) -> void:
	queue_action(route_name, action_id, {
		"selected_option": selected_option,
		"phase": str(state.get("phase", "entrance")),
		"phase_frame": int(state.get("phase_frame", 0)),
		"title_timer": int(state.get("title_timer", 0)),
		"input_gate_frames": int(state.get("input_gate_frames", ENTRANCE_FRAMES)),
		"attract_timeout_frames": int(state.get("attract_timeout_frames", ATTRACT_TIMEOUT_FRAMES)),
		"clock_reset_trigger": bool(state.get("clock_reset_trigger", false)),
	})
	_enter_phase("exiting", EXIT_HOLD_FRAMES)
	state["title_timer"] = 0

func _enter_phase(phase_name: String, input_gate_frames: int) -> void:
	state["phase"] = phase_name
	state["phase_frame"] = 0
	state["input_gate_frames"] = max(0, input_gate_frames)
	if phase_name == "main":
		state["title_timer"] = 0

func _update_labels() -> void:
	_set_labels(
		"TITLE SCREEN",
		"Phase: %s | Frame: %d | Timer: %d" % [
			str(state.get("phase", "entrance")),
			int(state.get("phase_frame", 0)),
			int(state.get("title_timer", 0)),
		],
		"A/Start=New Game  B=Continue  Select+Up=Delete  Select+Down=Clock"
	)

func _on_state_restored() -> void:
	state["screen"] = "title"
	state["phase"] = _normalize_phase(str(state.get("phase", "entrance")))
	state["phase_frame"] = max(0, int(state.get("phase_frame", state.get("title_timer", 0))))
	state["title_timer"] = max(0, int(state.get("title_timer", 0)))
	state["input_gate_frames"] = max(0, int(state.get("input_gate_frames", MAIN_INPUT_GATE_FRAMES)))
	state["pending_action"] = str(state.get("pending_action", ""))
	state["pending_action_payload"] = Dictionary(state.get("pending_action_payload", {})).duplicate(true)
	state["attract_timeout_frames"] = max(1, int(state.get("attract_timeout_frames", ATTRACT_TIMEOUT_FRAMES)))
	state["clock_reset_trigger"] = bool(state.get("clock_reset_trigger", false))
	if _route_entry_reset():
		state["phase"] = "main"
		state["phase_frame"] = 0
		state["input_gate_frames"] = 0
		clear_pending_action()
		state["input_locked"] = false
		state["pending_route"] = ""
		state["route_reason"] = ""
		state["clock_reset_trigger"] = false
	_clear_route_entry()

func _normalize_phase(value: String) -> String:
	match value:
		"entrance", "main", "exiting":
			return value
		_:
			return "entrance"
