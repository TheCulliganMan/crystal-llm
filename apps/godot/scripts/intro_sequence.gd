extends "res://scripts/boot_scene_base.gd"

const SCENE_COUNT := 4
const SCENE_FRAMES := 60
const INPUT_GATE_FRAMES := 8

func _on_ready() -> void:
	if state.is_empty():
		state = _default_screen_state("intro_sequence")
		state.merge({
			"phase": "running",
			"phase_frame": 0,
			"scene_checkpoint": 0,
			"scene_index": 0,
			"frame_counter": 0,
			"finished": false,
			"skip_requested": false,
			"input_gate_frames": INPUT_GATE_FRAMES,
			"pending_action": "",
			"pending_action_payload": {},
		}, true)
		_refresh_labels()

func can_accept_input() -> bool:
	return super.can_accept_input() and str(state.get("phase", "running")) == "running" and int(state.get("frame_counter", 0)) >= int(state.get("input_gate_frames", INPUT_GATE_FRAMES)) and not bool(state.get("finished", false))

func _tick(_delta: float) -> void:
	if bool(state.get("finished", false)):
		return
	state["phase_frame"] = int(state.get("phase_frame", 0)) + 1
	state["frame_counter"] = int(state.get("frame_counter", 0)) + 1
	state["scene_checkpoint"] = int(state.get("scene_index", 0))
	if int(state.get("frame_counter", 0)) >= SCENE_FRAMES:
		state["frame_counter"] = 0
		state["scene_index"] = int(state.get("scene_index", 0)) + 1
		state["scene_checkpoint"] = int(state.get("scene_index", 0))
		if int(state.get("scene_index", 0)) >= SCENE_COUNT:
			state["finished"] = true
			state["phase"] = "exiting"
			state["phase_frame"] = 0
			_queue_action("intro_complete")

func _handle_boot_input(_event: InputEvent) -> void:
	if bool(state.get("finished", false)):
		return
	if _input_pressed(_event, ["a", "start", "b"]):
		state["skip_requested"] = true
		state["finished"] = true
		state["phase"] = "exiting"
		state["phase_frame"] = 0
		_queue_action("intro_skip")

func _queue_action(action_id: String) -> void:
	queue_action("oak_intro", action_id, {
		"selected_option": "advance",
		"phase": str(state.get("phase", "running")),
		"phase_frame": int(state.get("phase_frame", 0)),
		"scene_checkpoint": int(state.get("scene_checkpoint", 0)),
		"scene_index": int(state.get("scene_index", 0)),
		"frame_counter": int(state.get("frame_counter", 0)),
		"input_gate_open": int(state.get("frame_counter", 0)) >= int(state.get("input_gate_frames", INPUT_GATE_FRAMES)),
		"input_gate_frames": int(state.get("input_gate_frames", INPUT_GATE_FRAMES)),
		"finished": bool(state.get("finished", false)),
		"skip_requested": bool(state.get("skip_requested", false)),
	})

func _update_labels() -> void:
	_set_labels(
		"INTRO SEQUENCE",
		"Scene: %d/%d | Frame: %d" % [int(state.get("scene_index", 0)) + 1, SCENE_COUNT, int(state.get("frame_counter", 0))],
		"Skip with A/B/Start"
	)

func _on_state_restored() -> void:
	state["screen"] = "intro_sequence"
	state["phase"] = str(state.get("phase", "running"))
	state["phase_frame"] = max(0, int(state.get("phase_frame", 0)))
	state["scene_checkpoint"] = clampi(int(state.get("scene_checkpoint", state.get("scene_index", 0))), 0, SCENE_COUNT - 1)
	state["scene_index"] = clampi(int(state.get("scene_index", 0)), 0, SCENE_COUNT - 1)
	state["frame_counter"] = max(0, int(state.get("frame_counter", 0)))
	state["finished"] = bool(state.get("finished", false))
	state["skip_requested"] = bool(state.get("skip_requested", false))
	state["input_gate_frames"] = max(0, int(state.get("input_gate_frames", INPUT_GATE_FRAMES)))
	state["pending_action"] = str(state.get("pending_action", ""))
	state["pending_action_payload"] = Dictionary(state.get("pending_action_payload", {})).duplicate(true)
	if _route_entry_reset():
		state["phase"] = "running"
		state["phase_frame"] = 0
		state["scene_checkpoint"] = 0
		state["scene_index"] = 0
		state["frame_counter"] = 0
		state["finished"] = false
		state["skip_requested"] = false
		state["input_gate_frames"] = INPUT_GATE_FRAMES
		clear_pending_action()
	_clear_route_entry()
