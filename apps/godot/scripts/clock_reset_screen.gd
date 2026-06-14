extends "res://scripts/boot_scene_base.gd"

const DAY_LABELS := ["SUN", "MON", "TUES", "WEDNES", "THURS", "FRI", "SATUR"]

func _on_ready() -> void:
	if state.is_empty():
		state = _default_screen_state("clock_reset_screen")
		state.merge({
			"phase": "confirm",
			"phase_frame": 0,
			"selection": 1,
			"day": 0,
			"hour": 0,
			"minute": 0,
			"pending_action": "",
			"pending_action_payload": {},
		}, true)
	_refresh_labels()

func _handle_boot_input(_event: InputEvent) -> void:
	if bool(state.get("confirmed", false)):
		return
	var phase := str(state.get("phase", "confirm"))
	if _input_pressed(_event, ["b"]):
		_queue_title_action("clock_reset_cancel", "back")
		return

	match phase:
		"confirm":
			if _input_pressed(_event, ["up", "down", "left", "right"]):
				state["selection"] = 1 - int(state.get("selection", 1))
				return
			if _input_pressed(_event, ["a"]):
				if int(state.get("selection", 1)) == 0:
					state["phase"] = "set_day"
					state["phase_frame"] = 0
					state["selection"] = 0
				else:
					_queue_title_action("clock_reset_cancel", "back")
				return
		"set_day":
			if _input_pressed(_event, ["up", "right"]):
				state["day"] = (int(state.get("day", 0)) + 1) % DAY_LABELS.size()
				return
			if _input_pressed(_event, ["down", "left"]):
				state["day"] = (int(state.get("day", 0)) + DAY_LABELS.size() - 1) % DAY_LABELS.size()
				return
			if _input_pressed(_event, ["a"]):
				state["phase"] = "set_hour"
				state["phase_frame"] = 0
				return
		"set_hour":
			if _input_pressed(_event, ["up", "right"]):
				state["hour"] = (int(state.get("hour", 0)) + 1) % 24
				return
			if _input_pressed(_event, ["down", "left"]):
				state["hour"] = (int(state.get("hour", 0)) + 23) % 24
				return
			if _input_pressed(_event, ["a"]):
				state["phase"] = "set_minute"
				state["phase_frame"] = 0
				return
		"set_minute":
			if _input_pressed(_event, ["up", "right"]):
				state["minute"] = (int(state.get("minute", 0)) + 1) % 60
				return
			if _input_pressed(_event, ["down", "left"]):
				state["minute"] = (int(state.get("minute", 0)) + 59) % 60
				return
			if _input_pressed(_event, ["a"]):
				_apply_clock()
				state["confirmed"] = true
				state["phase"] = "done"
				state["phase_frame"] = 0
				_queue_title_action("clock_reset_done", "confirm")
				return

func _tick(_delta: float) -> void:
	state["phase_frame"] = max(0, int(state.get("phase_frame", 0))) + 1

func _queue_title_action(action_id: String, selected_option: String) -> void:
	queue_action("title", action_id, {
		"selected_option": selected_option,
		"phase": str(state.get("phase", "confirm")),
		"selection": int(state.get("selection", 1)),
		"day": int(state.get("day", 0)),
		"hour": int(state.get("hour", 0)),
		"minute": int(state.get("minute", 0)),
		"confirmed": bool(state.get("confirmed", false)),
	})

func _apply_clock() -> void:
	if runtime != null and runtime is Object and runtime.has_method("set_boot_time"):
		runtime.call(
			"set_boot_time",
			int(state.get("day", 0)),
			int(state.get("hour", 0)),
			int(state.get("minute", 0))
		)
	elif runtime != null and runtime is Object and runtime.has_method("set_boot_day_of_week"):
		runtime.call("set_boot_day_of_week", int(state.get("day", 0)))

func _update_labels() -> void:
	var phase := str(state.get("phase", "confirm"))
	var detail := ""
	match phase:
		"confirm":
			detail = "Selection: %s" % ["yes", "no"][int(state.get("selection", 1))]
		"set_day":
			detail = "Day: %s" % DAY_LABELS[int(state.get("day", 0))]
		"set_hour":
			detail = "Hour: %02d" % int(state.get("hour", 0))
		"set_minute":
			detail = "Minute: %02d" % int(state.get("minute", 0))
		_:
			detail = "Selection: %s" % ["yes", "no"][int(state.get("selection", 1))]
	_set_labels(
		"CLOCK RESET",
		"Phase: %s | %s" % [phase, detail],
		"A advance  B cancel  Up/Down/Left/Right adjust"
	)

func _on_state_restored() -> void:
	state["screen"] = "clock_reset_screen"
	state["phase"] = str(state.get("phase", "confirm"))
	state["phase_frame"] = max(0, int(state.get("phase_frame", 0)))
	state["selection"] = _clamp_selection(state.get("selection", 1), 2, 1)
	state["day"] = clampi(int(state.get("day", 0)), 0, DAY_LABELS.size() - 1)
	state["hour"] = clampi(int(state.get("hour", 0)), 0, 23)
	state["minute"] = clampi(int(state.get("minute", 0)), 0, 59)
	state["confirmed"] = bool(state.get("confirmed", false))
	state["pending_action"] = str(state.get("pending_action", ""))
	state["pending_action_payload"] = Dictionary(state.get("pending_action_payload", {})).duplicate(true)
	if _route_entry_reset():
		state["phase"] = "confirm"
		state["phase_frame"] = 0
		state["selection"] = 1
		state["day"] = 0
		state["hour"] = 0
		state["minute"] = 0
		state["confirmed"] = false
		clear_pending_action()
	_clear_route_entry()
