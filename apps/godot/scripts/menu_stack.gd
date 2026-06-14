extends Node
class_name UIMenuStack

signal panel_pushed(top_panel: Dictionary)
signal panel_popped(top_panel: Dictionary)
signal panel_changed(top_panel: Dictionary)

const UP_BUTTONS := ["up", "left"]
const DOWN_BUTTONS := ["down", "right"]
const CONFIRM_BUTTONS := ["a", "start"]
const CANCEL_BUTTONS := ["b"]

var _stack: Array[Dictionary] = []
var _input_locked: bool = false
var _cursor_memory: Dictionary = {}

func reset() -> void:
	_stack.clear()
	_input_locked = false
	_cursor_memory = {}

func to_dictionary() -> Dictionary:
	return get_state()

func from_dictionary(data: Variant) -> bool:
	if typeof(data) != TYPE_DICTIONARY:
		return false
	from_state(Dictionary(data))
	return true

func get_current_panel() -> Dictionary:
	return get_top_panel()

func set_input_locked(is_locked: bool) -> void:
	_input_locked = is_locked

func is_active() -> bool:
	return not _stack.is_empty()

func is_menu_open() -> bool:
	return is_active()

func is_input_locked() -> bool:
	return _input_locked

func is_input_owned() -> bool:
	return is_active()

func has_panels() -> bool:
	return not _stack.is_empty()

func should_block_gameplay_input() -> bool:
	return is_active()

func get_depth() -> int:
	return _stack.size()

func get_panel_count() -> int:
	return _stack.size()

func get_top_panel_id() -> String:
	return str(get_top_panel().get("id", ""))

func get_top_panel_kind() -> String:
	return str(get_top_panel().get("kind", "menu"))

func get_top_panel_title() -> String:
	return str(get_top_panel().get("title", ""))

func has_selection() -> bool:
	return not get_selected_entry().is_empty()

func get_selected_index() -> int:
	return int(get_top_panel().get("cursor", 0))

func get_menu_cursor(panel_id: String) -> int:
	var normalized := panel_id.strip_edges()
	return int(_cursor_memory.get(normalized, 0))

func set_menu_cursor(panel_id: String, cursor: int) -> Dictionary:
	var normalized := panel_id.strip_edges()
	if normalized.is_empty():
		return get_top_panel()
	_cursor_memory[normalized] = max(0, cursor)
	if not _stack.is_empty() and str(_stack.back().get("id", "")) == normalized:
		return set_top_cursor(cursor)
	return get_top_panel()

func get_cursor_memory() -> Dictionary:
	return _cursor_memory.duplicate(true)

func can_accept_input() -> bool:
	return is_active() and not _input_locked and not bool(get_top_panel().get("locked", false))

func get_selected_entry() -> Dictionary:
	var top := get_top_panel()
	var selection: Variant = top.get("selection", {})
	if selection is Dictionary:
		return Dictionary(selection).duplicate(true)
	return {}

func get_selected_label() -> String:
	return str(get_selected_entry().get("label", ""))

func get_top_panel() -> Dictionary:
	if _stack.is_empty():
		return {}
	var top: Dictionary = _stack.back()
	var entries: Array = Array(top.get("entries", []))
	var cursor: int = _clamp_cursor(top, int(top.get("cursor", 0)))
	var selection := _selection_for(top, cursor)
	return {
		"id": str(top.get("id", "")),
		"title": str(top.get("title", "")),
		"kind": str(top.get("kind", "menu")),
		"cursor": cursor,
		"entry_count": entries.size(),
		"entries": _duplicate_entries(entries),
		"selection": selection,
		"cancelable": bool(top.get("cancelable", true)),
		"wrap": bool(top.get("wrap", true)),
		"locked": bool(top.get("locked", false)),
		"depth": _stack.size(),
	}

func get_state() -> Dictionary:
	return {
		"active": is_active(),
		"menu_open": is_menu_open(),
		"input_locked": _input_locked,
		"can_accept_input": can_accept_input(),
		"depth": _stack.size(),
		"top_panel": get_top_panel(),
		"current_panel": get_top_panel(),
		"stack": _duplicate_stack(_stack),
		"panels": _duplicate_stack(_stack),
		"selected_path": _selected_path(),
		"cursor_memory": _cursor_memory.duplicate(true),
		"menu_cursors": _cursor_memory.duplicate(true),
	}

func from_state(data: Dictionary) -> void:
	if data.is_empty():
		reset()
		return
	_stack = []
	_cursor_memory = {}
	for entry in _stack_entries_from_state(data):
		var normalized_panel := _normalize_panel(entry)
		_stack.append(normalized_panel)
		_remember_panel_cursor(normalized_panel)
	_cursor_memory = _normalize_cursor_memory(data.get("cursor_memory", data.get("menu_cursors", data.get("cursors", _cursor_memory))))
	for index in range(_stack.size()):
		var panel: Dictionary = _stack[index]
		var panel_id := str(panel.get("id", ""))
		if not panel_id.is_empty() and _cursor_memory.has(panel_id):
			panel["cursor"] = _clamp_panel_cursor(panel, int(_cursor_memory.get(panel_id, 0)))
			_stack[index] = panel
	_input_locked = bool(data.get("input_locked", false))
	if _stack.is_empty():
		_input_locked = false

func push_panel(panel: Variant) -> Dictionary:
	var normalized := _normalize_panel(panel)
	if not _panel_has_explicit_cursor(panel):
		var panel_id := str(normalized.get("id", ""))
		if _cursor_memory.has(panel_id):
			normalized["cursor"] = _clamp_panel_cursor(normalized, int(_cursor_memory.get(panel_id, 0)))
	_stack.append(normalized)
	_remember_panel_cursor(normalized)
	var top := get_top_panel()
	panel_pushed.emit(top)
	panel_changed.emit(top)
	return top

func pop_panel() -> Dictionary:
	if _stack.is_empty():
		return {}
	var popped: Dictionary = _stack.pop_back()
	_remember_panel_cursor(popped)
	var top := get_top_panel()
	var popped_snapshot := _panel_snapshot_from_entry(popped, _clamp_cursor(popped, int(popped.get("cursor", 0))), _stack.size() + 1)
	panel_popped.emit(popped_snapshot)
	panel_changed.emit(top)
	return popped_snapshot

func replace_top_panel(panel: Variant) -> Dictionary:
	if _stack.is_empty():
		return push_panel(panel)
	var normalized := _normalize_panel(panel)
	if not _panel_has_explicit_cursor(panel):
		var panel_id := str(normalized.get("id", ""))
		if _cursor_memory.has(panel_id):
			normalized["cursor"] = _clamp_panel_cursor(normalized, int(_cursor_memory.get(panel_id, 0)))
	_stack[_stack.size() - 1] = normalized
	_remember_panel_cursor(normalized)
	var top := get_top_panel()
	panel_changed.emit(top)
	return top

func clear() -> void:
	reset()
	panel_changed.emit({})

func set_top_cursor(cursor: int) -> Dictionary:
	if _stack.is_empty():
		return {}
	var top: Dictionary = _stack[_stack.size() - 1]
	top["cursor"] = _clamp_panel_cursor(top, cursor)
	_stack[_stack.size() - 1] = top
	_remember_panel_cursor(top)
	var snapshot := get_top_panel()
	panel_changed.emit(snapshot)
	return snapshot

func consume_input(frame_input: Dictionary) -> Dictionary:
	var result := {
		"consumed": false,
		"action": "",
		"top_panel": get_top_panel(),
		"selection": {},
		"depth": _stack.size(),
		"input_locked": _input_locked,
		"can_accept_input": can_accept_input(),
	}
	if _stack.is_empty():
		return result
	var pressed: Dictionary = Dictionary(frame_input.get("pressed", {}))
	if _input_locked or bool(get_top_panel().get("locked", false)):
		result["consumed"] = _has_any_pressed_button(pressed)
		return result
	var handled := false
	if _has_pressed_in_group(pressed, UP_BUTTONS):
		handled = _move_cursor(-1)
		if handled:
			result["action"] = "move_up"
	if not handled and _has_pressed_in_group(pressed, DOWN_BUTTONS):
		handled = _move_cursor(1)
		if handled:
			result["action"] = "move_down"
	if not handled and _has_pressed_in_group(pressed, CONFIRM_BUTTONS):
		var selection := _selection_for_top()
		result["action"] = "confirm"
		result["selection"] = selection
		result["consumed"] = true
		result["top_panel"] = get_top_panel()
		return result
	if not handled and _has_pressed_in_group(pressed, CANCEL_BUTTONS):
		result["action"] = "cancel"
		result["consumed"] = true
		result["top_panel"] = _cancel_top_panel()
		return result
	result["consumed"] = _has_any_pressed_button(pressed)
	result["top_panel"] = get_top_panel()
	if handled:
		result["consumed"] = true
		result["selection"] = _selection_for_top()
	return result

func _cancel_top_panel() -> Dictionary:
	if _stack.is_empty():
		return {}
	var top: Dictionary = _stack.back()
	if bool(top.get("cancelable", true)):
		_stack.pop_back()
		_remember_panel_cursor(top)
		var popped_snapshot := _panel_snapshot_from_entry(top, _clamp_cursor(top, int(top.get("cursor", 0))), _stack.size() + 1)
		var snapshot := get_top_panel()
		panel_popped.emit(popped_snapshot)
		panel_changed.emit(snapshot)
		return snapshot
	return get_top_panel()

func _move_cursor(step: int) -> bool:
	if _stack.is_empty():
		return false
	var top: Dictionary = _stack[_stack.size() - 1]
	var entries: Array = Array(top.get("entries", []))
	if entries.is_empty():
		return false
	var cursor: int = _clamp_cursor(top, int(top.get("cursor", 0)))
	var next_cursor: int = _next_enabled_cursor(entries, cursor, step, bool(top.get("wrap", true)))
	if next_cursor == cursor:
		return false
	top["cursor"] = next_cursor
	_stack[_stack.size() - 1] = top
	_remember_panel_cursor(top)
	panel_changed.emit(get_top_panel())
	return true

func _selection_for_top() -> Dictionary:
	if _stack.is_empty():
		return {}
	var top: Dictionary = _stack.back()
	return _selection_for(top, _clamp_cursor(top, int(top.get("cursor", 0))))

func _selection_for(top: Dictionary, cursor: int) -> Dictionary:
	var entries: Array = Array(top.get("entries", []))
	if cursor < 0 or cursor >= entries.size():
		return {}
	var entry: Variant = entries[cursor]
	return _normalize_entry(entry)

func _normalize_panel(panel: Variant) -> Dictionary:
	var result := {
		"id": "",
		"title": "",
		"kind": "menu",
		"entries": [],
		"cursor": 0,
		"cancelable": true,
		"wrap": true,
		"locked": false,
	}
	match typeof(panel):
		TYPE_STRING:
			result["id"] = str(panel)
			result["title"] = str(panel)
		TYPE_DICTIONARY:
			var source: Dictionary = panel
			result["id"] = str(source.get("id", source.get("name", "")))
			result["title"] = str(source.get("title", source.get("label", result["id"])))
			result["kind"] = str(source.get("kind", "menu"))
			result["entries"] = _normalize_entries(source.get("entries", []))
			result["cursor"] = max(0, int(source.get("cursor", source.get("selected_index", 0))))
			result["cancelable"] = bool(source.get("cancelable", true))
			result["wrap"] = bool(source.get("wrap", true))
			result["locked"] = bool(source.get("locked", false))
		_:
			pass
	result["cursor"] = _clamp_panel_cursor(result, int(result["cursor"]))
	return result

func _stack_entries_from_state(data: Dictionary) -> Array:
	var source: Variant = data.get("stack", data.get("panels", []))
	if typeof(source) == TYPE_ARRAY:
		var source_array: Array = source
		if not source_array.is_empty():
			return source_array
	var top_panel: Variant = data.get("top_panel", data.get("current_panel", {}))
	if typeof(top_panel) == TYPE_DICTIONARY and not Dictionary(top_panel).is_empty():
		return [top_panel]
	return []

func _panel_has_explicit_cursor(panel: Variant) -> bool:
	if typeof(panel) != TYPE_DICTIONARY:
		return false
	var source: Dictionary = panel
	return source.has("cursor") or source.has("selected_index")

func _remember_panel_cursor(panel: Dictionary) -> void:
	var panel_id := str(panel.get("id", "")).strip_edges()
	if panel_id.is_empty():
		return
	_cursor_memory[panel_id] = _clamp_cursor(panel, int(panel.get("cursor", 0)))

func _normalize_cursor_memory(value: Variant) -> Dictionary:
	var result: Dictionary = {}
	if typeof(value) != TYPE_DICTIONARY:
		return result
	var source: Dictionary = value
	for key in source.keys():
		var cursor_value: Variant = source.get(key, 0)
		if typeof(cursor_value) == TYPE_DICTIONARY:
			cursor_value = Dictionary(cursor_value).get("cursor", Dictionary(cursor_value).get("selected_index", 0))
		result[str(key)] = max(0, int(cursor_value))
	return result

func _normalize_entries(value: Variant) -> Array:
	var result: Array = []
	if typeof(value) != TYPE_ARRAY:
		return result
	var source: Array = value
	for entry in source:
		result.append(_normalize_entry(entry))
	return result

func _normalize_entry(entry: Variant) -> Dictionary:
	var result := {
		"id": "",
		"label": "",
		"enabled": true,
		"payload": null,
	}
	match typeof(entry):
		TYPE_STRING:
			var text := str(entry)
			result["id"] = text
			result["label"] = text
		TYPE_DICTIONARY:
			var source: Dictionary = entry
			result["id"] = str(source.get("id", source.get("name", "")))
			result["label"] = str(source.get("label", source.get("text", result["id"])))
			result["enabled"] = bool(source.get("enabled", true))
			result["payload"] = _normalize_payload(source.get("payload", source.get("value", null)))
		_:
			var fallback := str(entry)
			result["id"] = fallback
			result["label"] = fallback
	return result

func _normalize_payload(value: Variant) -> Variant:
	match typeof(value):
		TYPE_DICTIONARY:
			var normalized: Dictionary = {}
			var source: Dictionary = value
			for key in source.keys():
				normalized[key] = _normalize_payload(source[key])
			return normalized
		TYPE_ARRAY:
			var normalized_array: Array = []
			var source_array: Array = value
			for entry in source_array:
				normalized_array.append(_normalize_payload(entry))
			return normalized_array
		TYPE_STRING, TYPE_INT, TYPE_FLOAT, TYPE_BOOL, TYPE_NIL:
			return value
		_:
			return null

func _duplicate_entries(entries: Array) -> Array:
	var result: Array = []
	for entry in entries:
		if typeof(entry) == TYPE_DICTIONARY:
			result.append(Dictionary(entry).duplicate(true))
		else:
			result.append(entry)
	return result

func _duplicate_stack(stack: Array[Dictionary]) -> Array:
	var result: Array = []
	var depth := 1
	for panel in stack:
		var snapshot := panel.duplicate(true)
		snapshot["depth"] = depth
		snapshot["stack_index"] = depth - 1
		snapshot["selection"] = _selection_for(panel, _clamp_cursor(panel, int(panel.get("cursor", 0))))
		result.append(snapshot)
		depth += 1
	return result

func _selected_path() -> Array:
	var result: Array = []
	var depth := 1
	for panel in _stack:
		var cursor := _clamp_cursor(panel, int(panel.get("cursor", 0)))
		result.append({
			"id": str(panel.get("id", "")),
			"depth": depth,
			"cursor": cursor,
			"selection": _selection_for(panel, cursor),
		})
		depth += 1
	return result

func _panel_snapshot_from_entry(panel: Dictionary, cursor: int, depth: int) -> Dictionary:
	var entries: Array = Array(panel.get("entries", []))
	return {
		"id": str(panel.get("id", "")),
		"title": str(panel.get("title", "")),
		"kind": str(panel.get("kind", "menu")),
		"cursor": cursor,
		"entry_count": entries.size(),
		"entries": _duplicate_entries(entries),
		"selection": _selection_for(panel, cursor),
		"cancelable": bool(panel.get("cancelable", true)),
		"wrap": bool(panel.get("wrap", true)),
		"locked": bool(panel.get("locked", false)),
		"depth": depth,
	}

func _clamp_cursor(panel: Dictionary, cursor: int) -> int:
	var entries: Array = Array(panel.get("entries", []))
	if entries.is_empty():
		return 0
	return clamp(cursor, 0, entries.size() - 1)

func _clamp_panel_cursor(panel: Dictionary, cursor: int) -> int:
	var entries: Array = Array(panel.get("entries", []))
	if entries.is_empty():
		return 0
	var wrapped: int = cursor
	if wrapped < 0:
		wrapped = entries.size() - 1
	elif wrapped >= entries.size():
		wrapped = 0
	return wrapped

func _next_enabled_cursor(entries: Array, cursor: int, step: int, wrap: bool) -> int:
	if entries.is_empty():
		return 0
	var next_cursor: int = cursor
	for _i in range(entries.size()):
		next_cursor += step
		if wrap:
			if next_cursor < 0:
				next_cursor = entries.size() - 1
			elif next_cursor >= entries.size():
				next_cursor = 0
		else:
			next_cursor = clamp(next_cursor, 0, entries.size() - 1)
		if bool(Dictionary(entries[next_cursor]).get("enabled", true)):
			return next_cursor
		if not wrap and (next_cursor == 0 or next_cursor == entries.size() - 1):
			break
	return cursor

func _has_pressed_in_group(pressed: Dictionary, buttons: Array) -> bool:
	for button in buttons:
		if bool(pressed.get(button, false)):
			return true
	return false

func _has_any_pressed_button(pressed: Dictionary) -> bool:
	for button in pressed.keys():
		if bool(pressed.get(button, false)):
			return true
	return false
