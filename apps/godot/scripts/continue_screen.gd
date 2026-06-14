extends "res://scripts/boot_scene_base.gd"

const OPENING_FRAMES := 12
const EXITING_FRAMES := 6

func _on_ready() -> void:
	if state.is_empty():
		state = _default_screen_state("continue_screen")
		state.merge({
			"phase": "opening",
			"phase_frame": 0,
			"prompt_phase": "opening",
			"prompt_phase_frame": 0,
			"selection": 0,
			"confirmed": false,
			"pending_action": "",
			"pending_action_payload": {},
		}, true)
		_refresh_labels()

func can_accept_input() -> bool:
	return super.can_accept_input() and str(state.get("phase", "opening")) == "main"

func _handle_boot_input(_event: InputEvent) -> void:
	if bool(state.get("confirmed", false)):
		return
	if _input_pressed(_event, ["up", "down"]):
		state["selection"] = 1 - int(state.get("selection", 0))
	if _input_pressed(_event, ["a"]):
		_confirm()
	elif _input_pressed(_event, ["b"]):
		_queue_title_action("continue_cancel", "back")

func _confirm() -> void:
	state["confirmed"] = true
	state["phase"] = "exiting"
	state["phase_frame"] = 0
	state["prompt_phase"] = "exiting"
	state["prompt_phase_frame"] = 0
	if int(state.get("selection", 0)) == 0:
		queue_action("overworld", "continue_confirm", {
			"selected_option": "continue",
			"selection": int(state.get("selection", 0)),
			"confirmed": bool(state.get("confirmed", false)),
			"prompt_phase": str(state.get("prompt_phase", "opening")),
			"prompt_phase_frame": int(state.get("prompt_phase_frame", 0)),
		})
	else:
		_queue_title_action("continue_cancel", "back")

func _queue_title_action(action_id: String, selected_option: String) -> void:
	queue_action("title", action_id, {
		"selected_option": selected_option,
		"cancelled": selected_option == "back",
		"selection": int(state.get("selection", 0)),
		"confirmed": bool(state.get("confirmed", false)),
		"prompt_phase": str(state.get("prompt_phase", "opening")),
		"prompt_phase_frame": int(state.get("prompt_phase_frame", 0)),
	})

func _tick(_delta: float) -> void:
	var phase := str(state.get("phase", "opening"))
	var phase_frame := int(state.get("phase_frame", 0)) + 1
	state["phase_frame"] = phase_frame
	state["prompt_phase"] = phase
	state["prompt_phase_frame"] = phase_frame
	if phase == "opening" and phase_frame >= OPENING_FRAMES:
		state["phase"] = "main"
		state["phase_frame"] = 0
		state["prompt_phase"] = "main"
		state["prompt_phase_frame"] = 0
	elif phase == "exiting" and phase_frame >= EXITING_FRAMES:
		state["phase"] = "done"
		state["phase_frame"] = 0

func _update_labels() -> void:
	_set_labels(
		"CONTINUE",
		"Phase: %s | Selection: %s" % [str(state.get("phase", "opening")), ["continue", "back"][int(state.get("selection", 0))]],
		"A confirm  B cancel  Up/Down toggle"
	)

func _on_state_restored() -> void:
	state["screen"] = "continue_screen"
	state["phase"] = str(state.get("phase", "opening"))
	state["phase_frame"] = max(0, int(state.get("phase_frame", 0)))
	state["prompt_phase"] = str(state.get("prompt_phase", state.get("phase", "opening")))
	state["prompt_phase_frame"] = max(0, int(state.get("prompt_phase_frame", state.get("phase_frame", 0))))
	state["selection"] = _clamp_selection(state.get("selection", 0), 2)
	state["confirmed"] = bool(state.get("confirmed", false))
	state["pending_action"] = str(state.get("pending_action", ""))
	state["pending_action_payload"] = Dictionary(state.get("pending_action_payload", {})).duplicate(true)
	if _route_entry_reset():
		state["phase"] = "main"
		state["phase_frame"] = 0
		state["prompt_phase"] = "main"
		state["prompt_phase_frame"] = 0
		state["selection"] = 0
		state["confirmed"] = false
		clear_pending_action()
	_clear_route_entry()
