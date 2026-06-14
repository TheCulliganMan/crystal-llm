extends "res://scripts/boot_scene_base.gd"

const DAY_LABELS := ["SUN", "MON", "TUES", "WEDNES", "THURS", "FRI", "SATUR"]

func _on_ready() -> void:
	if state.is_empty():
		state = _default_screen_state("day_of_week_screen")
		state.merge({
			"phase": "select_day",
			"phase_frame": 0,
			"selected_day": 0,
			"confirmed": false,
			"ignore_confirm_until_release": true,
			"pending_action": "",
			"pending_action_payload": {},
		}, true)
	_refresh_labels()

func _handle_boot_input(_event: InputEvent) -> void:
	if bool(state.get("confirmed", false)):
		return
	if _input_released(_event, ["a"]):
		state["ignore_confirm_until_release"] = false
		return
	if _input_pressed(_event, ["up"]):
		state["selected_day"] = (int(state.get("selected_day", 0)) + 1) % DAY_LABELS.size()
		return
	if _input_pressed(_event, ["down"]):
		state["selected_day"] = (int(state.get("selected_day", 0)) + DAY_LABELS.size() - 1) % DAY_LABELS.size()
		return
	if _input_pressed(_event, ["a"]):
		if bool(state.get("ignore_confirm_until_release", true)):
			state["ignore_confirm_until_release"] = false
			return
		state["confirmed"] = true
		state["phase"] = "confirm"
		state["phase_frame"] = 0
		if runtime != null and runtime is Object and runtime.has_method("set_boot_day_of_week"):
			runtime.call("set_boot_day_of_week", int(state.get("selected_day", 0)))
		_queue_title_action("day_of_week_confirm", "confirm")
		return
	if _input_pressed(_event, ["b"]):
		_queue_title_action("day_of_week_cancel", "back")
		return

func _tick(_delta: float) -> void:
	state["phase_frame"] = max(0, int(state.get("phase_frame", 0))) + 1

func _queue_title_action(action_id: String, selected_option: String) -> void:
	queue_action("title", action_id, {
		"selected_option": selected_option,
		"selected_day": int(state.get("selected_day", 0)),
		"selected_day_label": str(DAY_LABELS[clampi(int(state.get("selected_day", 0)), 0, DAY_LABELS.size() - 1)]),
		"confirmed": bool(state.get("confirmed", false)),
	})

func _update_labels() -> void:
	var day_index := clampi(int(state.get("selected_day", 0)), 0, DAY_LABELS.size() - 1)
	var selected_day: String = str(DAY_LABELS[day_index])
	var prompt := "What day is it?"
	if bool(state.get("confirmed", false)):
		prompt = "%s, is it?" % selected_day
	_set_labels(
		"DAY OF WEEK",
		"Phase: %s | Selected: %s | Confirmed: %s" % [str(state.get("phase", "select_day")), selected_day, str(bool(state.get("confirmed", false)))],
		"%s" % prompt
	)

func _on_state_restored() -> void:
	state["screen"] = "day_of_week_screen"
	state["phase"] = str(state.get("phase", "select_day"))
	state["phase_frame"] = max(0, int(state.get("phase_frame", 0)))
	state["selected_day"] = clampi(int(state.get("selected_day", 0)), 0, DAY_LABELS.size() - 1)
	state["confirmed"] = bool(state.get("confirmed", false))
	state["ignore_confirm_until_release"] = bool(state.get("ignore_confirm_until_release", true))
	state["pending_action"] = str(state.get("pending_action", ""))
	state["pending_action_payload"] = Dictionary(state.get("pending_action_payload", {})).duplicate(true)
	if _route_entry_reset():
		state["phase"] = "select_day"
		state["phase_frame"] = 0
		state["selected_day"] = 0
		state["confirmed"] = false
		state["ignore_confirm_until_release"] = true
		clear_pending_action()
	_clear_route_entry()
