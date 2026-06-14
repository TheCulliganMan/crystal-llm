extends "res://scripts/boot_scene_base.gd"

const MAX_NAME_LENGTH := 7
const LETTER_COLUMN_COUNT := 9
const BOTTOM_ROW_INDEX := 4

const UPPER_ROWS := [
	["A", "B", "C", "D", "E", "F", "G", "H", "I"],
	["J", "K", "L", "M", "N", "O", "P", "Q", "R"],
	["S", "T", "U", "V", "W", "X", "Y", "Z", " "],
	["-", "?", "!", "/", ".", ",", "<PK>", "<MN>", " "],
]

const LOWER_ROWS := [
	["a", "b", "c", "d", "e", "f", "g", "h", "i"],
	["j", "k", "l", "m", "n", "o", "p", "q", "r"],
	["s", "t", "u", "v", "w", "x", "y", "z", " "],
	["×", "(", ")", ":", ";", "[", "]", "<PK>", "<MN>"],
]

func _on_ready() -> void:
	if state.is_empty():
		state = _default_screen_state("name_entry")
		state.merge({
			"phase": "editing",
			"phase_frame": 0,
			"cursor_blink_frame": 0,
			"cursor_visible": true,
			"keyboard_page": "upper",
			"cursor_grid_row": 0,
			"cursor_grid_column": 0,
			"name": "",
			"cursor_index": 0,
			"cursor_column": 0,
			"cursor_row": 0,
			"case": "upper",
			"finished": false,
			"max_name_length": MAX_NAME_LENGTH,
			"pending_action": "",
			"pending_action_payload": {},
		}, true)
	_refresh_labels()

func _handle_boot_input(event: InputEvent) -> void:
	if bool(state.get("finished", false)):
		return
	if event is not InputEventKey:
		return
	var key_event := event as InputEventKey
	if key_event.echo or not key_event.pressed:
		return

	if _input_pressed(event, ["up"]):
		_move_vertical(-1)
		return
	if _input_pressed(event, ["down"]):
		_move_vertical(1)
		return
	if _input_pressed(event, ["left"]):
		_move_horizontal(-1)
		return
	if _input_pressed(event, ["right"]):
		_move_horizontal(1)
		return
	if _input_pressed(event, ["select"]):
		_toggle_case()
		return
	if _input_pressed(event, ["start"]):
		_move_cursor_to_end()
		return
	if _input_pressed(event, ["b"]):
		_pop_character()
		return
	if _input_pressed(event, ["a"]):
		_press_a()
		return

	if key_event.keycode == KEY_BACKSPACE:
		_pop_character()
		return
	if key_event.keycode == KEY_TAB:
		_toggle_case()
		return
	if key_event.keycode == KEY_ENTER or key_event.keycode == KEY_KP_ENTER:
		_move_cursor_to_end()
		return
	if _append_from_event(key_event):
		return

func _append_from_event(event: InputEventKey) -> bool:
	if event.unicode < 32 or event.unicode > 126:
		return false
	var char := String.chr(event.unicode)
	var code := event.unicode
	if char == " " or (code >= 48 and code <= 57) or (code >= 65 and code <= 90) or (code >= 97 and code <= 122):
		_append_character(char)
		return true
	return false

func _current_layout() -> Array:
	return LOWER_ROWS if str(state.get("case", "upper")).to_lower() == "lower" else UPPER_ROWS

func _selected_character() -> String:
	var layout: Array = _current_layout()
	var row := clampi(int(state.get("cursor_row", 0)), 0, BOTTOM_ROW_INDEX - 1)
	var column := clampi(int(state.get("cursor_column", 0)), 0, LETTER_COLUMN_COUNT - 1)
	var row_values: Array = Array(layout[row])
	if row_values.is_empty():
		return ""
	return str(row_values[column])

func _get_bottom_group() -> int:
	var column := clampi(int(state.get("cursor_column", 0)), 0, LETTER_COLUMN_COUNT - 1)
	if column < 3:
		return 1
	if column < 6:
		return 2
	return 3

func _move_vertical(direction: int) -> void:
	var row := clampi(int(state.get("cursor_row", 0)), 0, BOTTOM_ROW_INDEX)
	if direction < 0:
		row = 0 if row == BOTTOM_ROW_INDEX else row - 1
	elif direction > 0:
		row = BOTTOM_ROW_INDEX if row == 0 else row + 1
	state["cursor_row"] = row
	state["cursor_grid_row"] = row

func _move_horizontal(direction: int) -> void:
	var row := clampi(int(state.get("cursor_row", 0)), 0, BOTTOM_ROW_INDEX)
	var column := clampi(int(state.get("cursor_column", 0)), 0, LETTER_COLUMN_COUNT - 1)
	if row == BOTTOM_ROW_INDEX:
		var group := _get_bottom_group()
		if direction > 0:
			column = 0 if group == 3 else 3 if group == 1 else 6
		elif direction < 0:
			column = 6 if group == 1 else 0 if group == 2 else 3
	else:
		column = (column + direction + LETTER_COLUMN_COUNT) % LETTER_COLUMN_COUNT
	state["cursor_column"] = column
	state["cursor_grid_column"] = column

func _move_cursor_to_end() -> void:
	state["cursor_row"] = BOTTOM_ROW_INDEX
	state["cursor_column"] = 8
	state["cursor_grid_row"] = BOTTOM_ROW_INDEX
	state["cursor_grid_column"] = 8

func _press_a() -> void:
	if int(state.get("cursor_row", 0)) == BOTTOM_ROW_INDEX:
		match _get_bottom_group():
			1:
				_toggle_case()
			2:
				_pop_character()
			3:
				_confirm_name()
	else:
		if str(state.get("name", "")).length() >= int(state.get("max_name_length", MAX_NAME_LENGTH)):
			return
		var char := _selected_character()
		if not char.is_empty():
			_append_character(char)

func _append_character(char: String) -> void:
	var name := str(state.get("name", ""))
	var max_name_length := int(state.get("max_name_length", MAX_NAME_LENGTH))
	if name.length() >= max_name_length:
		return
	var next_char := char.to_upper() if str(state.get("case", "upper")) == "upper" else char.to_lower()
	state["name"] = name + next_char
	state["cursor_index"] = state["name"].length()

func _pop_character() -> void:
	var name := str(state.get("name", ""))
	if name.is_empty():
		return
	state["name"] = name.substr(0, name.length() - 1)
	state["cursor_index"] = state["name"].length()

func _toggle_case() -> void:
	state["case"] = "lower" if str(state.get("case", "upper")) == "upper" else "upper"
	state["keyboard_page"] = str(state.get("case", "upper"))

func _confirm_name() -> void:
	state["finished"] = true
	state["phase"] = "finished"
	state["phase_frame"] = 0
	state["cursor_blink_frame"] = 0
	state["cursor_visible"] = false
	state["keyboard_page"] = str(state.get("case", "upper"))
	if runtime != null and runtime is Object and runtime.has_method("set_player_name"):
		runtime.call("set_player_name", str(state.get("name", "")))
	queue_action("overworld", "name_entry_confirm", {
		"selected_option": "end",
		"name": str(state.get("name", "")),
		"cursor_index": int(state.get("cursor_index", 0)),
		"cursor_column": int(state.get("cursor_column", 0)),
		"cursor_row": int(state.get("cursor_row", 0)),
		"cursor_grid_row": int(state.get("cursor_grid_row", state.get("cursor_row", 0))),
		"cursor_grid_column": int(state.get("cursor_grid_column", state.get("cursor_column", 0))),
		"case": str(state.get("case", "upper")),
		"keyboard_page": str(state.get("keyboard_page", state.get("case", "upper"))),
		"finished": bool(state.get("finished", false)),
	})

func _tick(_delta: float) -> void:
	var phase := str(state.get("phase", "editing"))
	var phase_frame := int(state.get("phase_frame", 0)) + 1
	state["phase_frame"] = phase_frame
	if phase == "editing" and not bool(state.get("finished", false)):
		var cursor_blink_frame := (int(state.get("cursor_blink_frame", 0)) + 1) % 30
		state["cursor_blink_frame"] = cursor_blink_frame
		state["cursor_visible"] = cursor_blink_frame < 15
	elif phase == "finished":
		state["cursor_visible"] = false

func _update_labels() -> void:
	var name_value := str(state.get("name", ""))
	var cursor_row := clampi(int(state.get("cursor_row", 0)), 0, BOTTOM_ROW_INDEX)
	var cursor_column := clampi(int(state.get("cursor_column", 0)), 0, LETTER_COLUMN_COUNT - 1)
	var selected_cell := _selected_character()
	if cursor_row == BOTTOM_ROW_INDEX:
		selected_cell = ["lower", "DEL", "END"][_get_bottom_group() - 1]
	_set_labels(
		"NAME ENTRY",
		"Name: %s | Case: %s | Length: %d/%d" % [
			name_value if not name_value.is_empty() else "(blank)",
			str(state.get("case", "upper")),
			name_value.length(),
			int(state.get("max_name_length", MAX_NAME_LENGTH)),
		],
		"Cursor: row %d col %d | Selected: %s" % [cursor_row, cursor_column, selected_cell]
	)

func _on_state_restored() -> void:
	state["screen"] = "name_entry"
	state["name"] = _normalize_name(str(state.get("name", "")))
	state["cursor_index"] = clampi(int(state.get("cursor_index", state["name"].length())), 0, str(state.get("name", "")).length())
	state["cursor_column"] = clampi(int(state.get("cursor_column", 0)), 0, LETTER_COLUMN_COUNT - 1)
	state["cursor_row"] = clampi(int(state.get("cursor_row", 0)), 0, BOTTOM_ROW_INDEX)
	state["cursor_grid_row"] = clampi(int(state.get("cursor_grid_row", state.get("cursor_row", 0))), 0, BOTTOM_ROW_INDEX)
	state["cursor_grid_column"] = clampi(int(state.get("cursor_grid_column", state.get("cursor_column", 0))), 0, LETTER_COLUMN_COUNT - 1)
	state["phase"] = "finished" if bool(state.get("finished", false)) else str(state.get("phase", "editing"))
	state["phase_frame"] = max(0, int(state.get("phase_frame", 0)))
	state["cursor_blink_frame"] = max(0, int(state.get("cursor_blink_frame", 0))) % 30
	state["cursor_visible"] = bool(state.get("cursor_visible", true))
	state["case"] = "lower" if str(state.get("case", "upper")).to_lower() == "lower" else "upper"
	state["keyboard_page"] = str(state.get("keyboard_page", state.get("case", "upper")))
	state["finished"] = bool(state.get("finished", false))
	state["max_name_length"] = clampi(int(state.get("max_name_length", MAX_NAME_LENGTH)), 1, MAX_NAME_LENGTH)
	state["pending_action"] = str(state.get("pending_action", ""))
	var pending_action_payload: Dictionary = Dictionary(state.get("pending_action_payload", {})).duplicate(true)
	if str(state.get("pending_action", "")) == "overworld" and str(pending_action_payload.get("action_id", "")) == "name_entry_confirm":
		if not pending_action_payload.has("keyboard_page"):
			pending_action_payload["keyboard_page"] = str(state.get("keyboard_page", state.get("case", "upper")))
	state["pending_action_payload"] = pending_action_payload
	if _route_entry_reset():
		state["name"] = ""
		state["cursor_index"] = 0
		state["cursor_column"] = 0
		state["cursor_row"] = 0
		state["cursor_grid_row"] = 0
		state["cursor_grid_column"] = 0
		state["case"] = "upper"
		state["keyboard_page"] = "upper"
		state["finished"] = false
		state["phase"] = "editing"
		state["phase_frame"] = 0
		state["cursor_blink_frame"] = 0
		state["cursor_visible"] = true
		clear_pending_action()
	_clear_route_entry()

func _normalize_name(value: String) -> String:
	var result := ""
	var max_name_length := MAX_NAME_LENGTH
	for index in range(value.length()):
		var code := value.unicode_at(index)
		if result.length() >= max_name_length:
			break
		if code == 32 or (code >= 48 and code <= 57) or (code >= 65 and code <= 90) or (code >= 97 and code <= 122):
			result += String.chr(code)
	return result
