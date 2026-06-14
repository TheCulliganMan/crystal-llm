extends "res://scripts/boot_scene_base.gd"

const TEXT_SCENES := ["oak_intro_1", "wooper_showcase", "oak_intro_2"]
const TEXT_SCENE_FRAMES := 90

func _on_ready() -> void:
	if state.is_empty():
		state = _default_screen_state("oak_intro")
		state.merge({
			"mode": "intro",
			"scene_index": 0,
			"scene_state": "oak_intro_1",
			"scene_phase": "text",
			"text_checkpoint": "oak_intro_1",
			"text_page_index": 0,
			"text_page_count": TEXT_SCENES.size(),
			"text_waiting_for_input": false,
			"gender": "male",
			"frame_counter": 0,
			"confirmed": false,
			"pending_action": "",
			"pending_action_payload": {},
		}, true)
	_refresh_labels()

func _tick(_delta: float) -> void:
	if bool(state.get("confirmed", false)) or bool(state.get("input_locked", false)):
		return
	if str(state.get("scene_phase", "text")) != "text":
		return
	state["frame_counter"] = int(state.get("frame_counter", 0)) + 1
	state["text_page_index"] = int(state.get("scene_index", 0))
	state["text_checkpoint"] = str(state.get("scene_state", "oak_intro_1"))
	state["text_waiting_for_input"] = false
	if int(state.get("frame_counter", 0)) < TEXT_SCENE_FRAMES:
		return
	state["frame_counter"] = 0
	var next_index := int(state.get("scene_index", 0)) + 1
	if next_index >= TEXT_SCENES.size():
		state["scene_index"] = TEXT_SCENES.size() - 1
		state["scene_state"] = "player_picture"
		state["scene_phase"] = "select"
		state["text_page_index"] = int(state.get("scene_index", 0))
		state["text_checkpoint"] = "player_picture"
		state["text_waiting_for_input"] = true
		return
	state["scene_index"] = next_index
	state["scene_state"] = TEXT_SCENES[next_index]
	state["text_page_index"] = int(state.get("scene_index", 0))
	state["text_checkpoint"] = str(state.get("scene_state", TEXT_SCENES[next_index]))
	state["text_waiting_for_input"] = true

func _handle_boot_input(_event: InputEvent) -> void:
	if bool(state.get("confirmed", false)):
		return
	if str(state.get("scene_phase", "text")) != "select":
		if _input_pressed(_event, ["b"]):
			_queue_action("title", "oak_intro_cancel", "back")
		return
	if _input_pressed(_event, ["left", "right"]):
		state["gender"] = "female" if str(state.get("gender", "male")) == "male" else "male"
		if runtime != null and runtime is Object and runtime.has_method("set_player_gender"):
			runtime.call("set_player_gender", str(state.get("gender", "male")))
	if _input_pressed(_event, ["a"]):
		state["confirmed"] = true
		if runtime != null and runtime is Object and runtime.has_method("set_player_gender"):
			runtime.call("set_player_gender", str(state.get("gender", "male")))
		_queue_action("name_entry", "oak_intro_confirm", str(state.get("gender", "male")))
	if _input_pressed(_event, ["b"]):
		_queue_action("title", "oak_intro_cancel", "back")

func _queue_action(route_name: String, action_id: String, selected_option: String) -> void:
	queue_action(route_name, action_id, {
		"selected_option": selected_option,
		"mode": str(state.get("mode", "intro")),
		"scene_index": int(state.get("scene_index", 0)),
		"scene_state": str(state.get("scene_state", "oak_intro_1")),
		"scene_phase": str(state.get("scene_phase", "text")),
		"text_checkpoint": str(state.get("text_checkpoint", state.get("scene_state", "oak_intro_1"))),
		"text_page_index": int(state.get("text_page_index", 0)),
		"text_page_count": int(state.get("text_page_count", TEXT_SCENES.size())),
		"text_waiting_for_input": bool(state.get("text_waiting_for_input", false)),
		"text_gate_open": bool(state.get("text_waiting_for_input", false)),
		"gender": str(state.get("gender", "male")),
		"frame_counter": int(state.get("frame_counter", 0)),
		"confirmed": bool(state.get("confirmed", false)),
	})

func _update_labels() -> void:
	_set_labels(
		"OAK INTRO",
		"Mode: %s | Scene: %s | Phase: %s" % [
			str(state.get("mode", "intro")),
			str(state.get("scene_state", "oak_intro_1")),
			str(state.get("scene_phase", "text")),
		],
		"Gender: %s | Left/Right toggle | A confirm | B back" % str(state.get("gender", "male"))
	)

func _on_state_restored() -> void:
	state["screen"] = "oak_intro"
	state["mode"] = str(state.get("mode", "intro"))
	state["scene_index"] = clampi(int(state.get("scene_index", 0)), 0, TEXT_SCENES.size() - 1)
	state["scene_state"] = str(state.get("scene_state", TEXT_SCENES[int(state.get("scene_index", 0))]))
	state["scene_phase"] = str(state.get("scene_phase", "text"))
	state["text_checkpoint"] = str(state.get("text_checkpoint", state["scene_state"]))
	state["text_page_index"] = clampi(int(state.get("text_page_index", state.get("scene_index", 0))), 0, TEXT_SCENES.size() - 1)
	state["text_page_count"] = max(1, int(state.get("text_page_count", TEXT_SCENES.size())))
	state["text_waiting_for_input"] = bool(state.get("text_waiting_for_input", false))
	state["gender"] = _normalize_gender(str(state.get("gender", "male")))
	state["frame_counter"] = max(0, int(state.get("frame_counter", 0)))
	state["confirmed"] = bool(state.get("confirmed", false))
	state["pending_action"] = str(state.get("pending_action", ""))
	state["pending_action_payload"] = Dictionary(state.get("pending_action_payload", {})).duplicate(true)
	if _route_entry_reset():
		state["scene_index"] = 0
		state["scene_state"] = "oak_intro_1"
		state["scene_phase"] = "text"
		state["text_checkpoint"] = "oak_intro_1"
		state["text_page_index"] = 0
		state["text_page_count"] = TEXT_SCENES.size()
		state["text_waiting_for_input"] = false
		state["frame_counter"] = 0
		state["confirmed"] = false
		clear_pending_action()
	_clear_route_entry()

func _normalize_gender(value: String) -> String:
	var normalized := value.strip_edges().to_lower()
	return "female" if normalized == "female" else "male"
