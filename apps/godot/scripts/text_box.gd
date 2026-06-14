extends Node
class_name UITextBox

signal opened
signal page_changed(page_index: int, page_count: int)
signal closed

const CONFIRM_BUTTONS := ["a", "start"]
const POKE_GLYPH := "#"
const SIX_DOTS_TEXT := "……"
const TEXTBOX_WIDTH_TILES := 20
const TEXTBOX_HEIGHT_TILES := 6
const TEXTBOX_BORDER_WIDTH := 2
const TEXTBOX_Y_TILES := 12
const TEXTBOX_INNER_HEIGHT_TILES := TEXTBOX_HEIGHT_TILES - TEXTBOX_BORDER_WIDTH
const TEXTBOX_INNER_Y_TILES := TEXTBOX_Y_TILES + TEXTBOX_BORDER_WIDTH
const DEFAULT_LINES_PER_PAGE := 2
const DEFAULT_CHARS_PER_LINE := 18
const DEFAULT_INPUT_DELAY_FRAMES := 2
const WAIT_CONTROL_TOKENS := ["<WAIT>", "<PAUSE>"]
const CONTROL_CODE_REPLACEMENTS := [
	["<TRAINER>", "\ue103"],
	["<ROCKET>", "\ue104"],
	["<PKMN>", "\ue105\ue106"],
	["<POKE>", POKE_GLYPH],
	["<PC>", "\ue101"],
	["<TM>", "\ue102"],
	["<PK>", "\ue105"],
	["<MN>", "\ue106"],
	["<DOT>", "\ue107"],
	["<PO>", "\ue108"],
	["<KE>", "\ue109"],
	["<LV>", "\ue10a"],
	["<ID>", "\ue10b"],
	["<……>", SIX_DOTS_TEXT],
]

var _pages: Array[Dictionary] = []
var _page_index: int = -1
var _active: bool = false
var _input_locked: bool = false
var _pending_waits: int = 0
var _auto_close_after_wait: bool = false
var _page_frame: int = 0
var _page_token_specs: Array[Dictionary] = []
var _page_tokens: Array[String] = []
var _page_token_cursor: int = 0
var _page_visible_tokens: int = 0
var _page_token_glyph_cursor: int = 0
var _page_token_frame_timer: int = 0
var _page_wait_token_pending: bool = false
var _page_visible_chars: int = 0
var _page_reveal_timer: int = 0

func reset() -> void:
	_pages.clear()
	_page_index = -1
	_active = false
	_input_locked = false
	_pending_waits = 0
	_auto_close_after_wait = false
	_page_frame = 0
	_page_token_specs = []
	_page_tokens = []
	_page_token_cursor = 0
	_page_visible_tokens = 0
	_page_token_glyph_cursor = 0
	_page_token_frame_timer = 0
	_page_wait_token_pending = false
	_page_visible_chars = 0
	_page_reveal_timer = 0

func to_dictionary() -> Dictionary:
	return get_state()

func from_dictionary(data: Variant) -> bool:
	if typeof(data) != TYPE_DICTIONARY:
		return false
	from_state(Dictionary(data))
	return true

func open_dialogue(content: Variant) -> void:
	_pages = _normalize_pages(content)
	if _pages.is_empty():
		reset()
		return
	_page_index = 0
	_active = true
	_input_locked = false
	_pending_waits = 0
	_auto_close_after_wait = false
	_page_frame = 0
	_page_token_specs = []
	_page_tokens = []
	_page_token_cursor = 0
	_page_visible_tokens = 0
	_page_token_glyph_cursor = 0
	_page_token_frame_timer = 0
	_page_wait_token_pending = false
	_sync_page_tokens()
	_page_visible_chars = _initial_visible_chars()
	_page_reveal_timer = 0
	opened.emit()
	page_changed.emit(_page_index, _pages.size())

func tick() -> void:
	if _active:
		_page_frame += 1
		_step_typewriter()

func close_dialogue() -> void:
	if not _active and _pages.is_empty():
		return
	reset()
	closed.emit()

func set_input_locked(is_locked: bool) -> void:
	_input_locked = is_locked

func is_active() -> bool:
	return _active

func is_visible() -> bool:
	return _active

func is_dialog_active() -> bool:
	return _active

func is_text_box_open() -> bool:
	return _active

func is_input_owned() -> bool:
	return _active

func is_input_locked() -> bool:
	return _input_locked

func has_pages() -> bool:
	return not _pages.is_empty()

func has_more_pages() -> bool:
	return _active and _page_index >= 0 and _page_index < _pages.size() - 1

func should_block_gameplay_input() -> bool:
	return _active

func is_waiting_for_input() -> bool:
	if not _active or _page_index < 0 or _page_index >= _pages.size():
		return false
	return _pending_waits > 0 or _page_wait_token_pending or str(_current_token_spec().get("kind", "")) == "wait" or (is_complete() and bool(_pages[_page_index].get("wait_for_input", true)))

func is_page_input_ready() -> bool:
	if _pending_waits > 0:
		return false
	if _page_wait_token_pending or str(_current_token_spec().get("kind", "")) == "wait":
		return false
	if not _active or _page_index < 0 or _page_index >= _pages.size():
		return false
	if not bool(_pages[_page_index].get("wait_for_input", true)):
		return false
	return _page_frame >= _current_page_input_delay()

func can_advance() -> bool:
	return _active and not _input_locked and is_page_input_ready() and is_complete()

func can_accept_input() -> bool:
	return can_advance()

func is_complete() -> bool:
	if not _active or _page_index < 0 or _page_index >= _pages.size():
		return true
	return not _page_wait_token_pending and _page_token_cursor >= _current_page_token_count()

func complete() -> void:
	if not _active or _page_index < 0 or _page_index >= _pages.size():
		return
	_page_visible_tokens = _current_page_token_count()
	_page_token_cursor = _page_visible_tokens
	_page_token_glyph_cursor = 0
	_page_token_frame_timer = 0
	_page_wait_token_pending = false
	_page_visible_chars = _current_page_visible_text().length()
	_page_reveal_timer = 0

func push_wait(count: int = 1) -> int:
	if count <= 0:
		return _pending_waits
	_pending_waits += count
	return _pending_waits

func acknowledge_wait() -> bool:
	if _page_wait_token_pending:
		_page_wait_token_pending = false
		_page_token_cursor = min(_page_token_cursor + 1, _page_token_specs.size())
		_page_token_glyph_cursor = 0
		_page_token_frame_timer = 0
		return true
	if str(_current_token_spec().get("kind", "")) == "wait":
		_page_wait_token_pending = false
		_page_token_cursor = min(_page_token_cursor + 1, _page_token_specs.size())
		_page_token_glyph_cursor = 0
		_page_token_frame_timer = 0
		_page_visible_chars = _current_page_visible_text().length()
		return true
	if _pending_waits <= 0:
		return false
	_pending_waits = max(0, _pending_waits - 1)
	return true

func set_auto_close_after_wait(should_auto_close: bool) -> void:
	_auto_close_after_wait = should_auto_close

func should_auto_close_after_wait() -> bool:
	return _auto_close_after_wait

func has_pending_waits() -> bool:
	return _pending_waits > 0

func get_pending_waits() -> int:
	return _pending_waits

func get_page_count() -> int:
	return _pages.size()

func get_page_index() -> int:
	return _page_index

func get_page_cursor() -> Dictionary:
	var current_token_index := _page_token_cursor - 1
	var current_token := ""
	if current_token_index >= 0 and current_token_index < _page_tokens.size():
		current_token = str(_page_tokens[current_token_index])
	return {
		"index": _page_index,
		"page_index": _page_index,
		"page_count": _pages.size(),
		"page_frame": _page_frame,
		"token_cursor": _page_token_cursor,
		"current_token_index": current_token_index,
		"current_token": current_token,
		"current_token_kind": str(_current_token_spec().get("kind", "")),
		"current_token_frame_delay": int(_current_token_spec().get("frame_delay_frames", 0)),
		"current_token_glyph_delay": int(_current_token_spec().get("glyph_delay_frames", 0)),
		"current_token_glyph_cursor": _page_token_glyph_cursor,
		"current_token_glyph_count": int(_current_token_spec().get("glyph_count", 0)),
		"current_token_frame_timer": _page_token_frame_timer,
		"visible_tokens": _page_visible_tokens,
		"page_token_count": _current_page_token_count(),
		"page_tokens": _page_tokens.duplicate(true),
		"page_token_specs": _duplicate_token_specs(_page_token_specs),
		"visible_chars": _page_visible_chars,
		"page_reveal_timer": _page_reveal_timer,
		"pending_waits": _pending_waits,
		"token_wait_pending": _page_wait_token_pending,
		"pending_script_waits": _pending_waits,
		"auto_close_after_wait": _auto_close_after_wait,
		"page_complete": is_complete(),
		"input_delay_frames": _current_page_input_delay(),
		"input_ready": is_page_input_ready(),
		"has_more_pages": has_more_pages(),
	}

func set_page_index(page_index: int) -> Dictionary:
	if _pages.is_empty():
		_page_index = -1
		_active = false
		return get_state()
	_page_index = clampi(page_index, 0, _pages.size() - 1)
	_page_frame = 0
	_page_visible_tokens = 0
	_page_token_cursor = 0
	_sync_page_tokens()
	_page_visible_chars = _initial_visible_chars()
	_page_reveal_timer = 0
	page_changed.emit(_page_index, _pages.size())
	return get_state()

func get_panel_kind() -> String:
	return "dialogue"

func get_panel_id() -> String:
	return "text_box"

func get_current_page() -> Dictionary:
	if not _active or _page_index < 0 or _page_index >= _pages.size():
		return {}
	return _pages[_page_index].duplicate(true)

func get_current_page_text() -> String:
	var page := get_current_page()
	if page.is_empty():
		return ""
	return str(page.get("display_text", page.get("text", "")))

func get_visible_text() -> String:
	return _current_page_visible_text()

func _current_page_visible_text() -> String:
	var text := _current_page_visible_source_text()
	if text.is_empty():
		return text
	if _page_token_specs.is_empty():
		if is_complete():
			return text
		return text.substr(0, clampi(_page_visible_chars, 0, text.length()))
	var visible_text := ""
	for token_index in range(clampi(_page_visible_tokens, 0, _page_token_specs.size())):
		var spec := Dictionary(_page_token_specs[token_index])
		if str(spec.get("kind", "text")) != "wait":
			visible_text += str(spec.get("text", ""))
	if _page_token_cursor < _page_token_specs.size():
		var current_spec := Dictionary(_page_token_specs[_page_token_cursor])
		if str(current_spec.get("kind", "text")) != "wait":
			visible_text += str(current_spec.get("text", "")).substr(0, clampi(_page_token_glyph_cursor, 0, int(current_spec.get("glyph_count", 0))))
	return visible_text

func get_current_page_lines() -> Array[String]:
	var page := get_current_page()
	if page.is_empty():
		return []
	var lines: Array = Array(page.get("display_lines", page.get("page_lines", [])))
	var result: Array[String] = []
	for line in lines:
		result.append(str(line))
	return result

func get_dialogue_lines() -> Array[String]:
	return get_current_page_lines()

func get_textbox_rect() -> Dictionary:
	return {
		"x_tiles": 0,
		"y_tiles": TEXTBOX_Y_TILES,
		"width_tiles": TEXTBOX_WIDTH_TILES,
		"height_tiles": TEXTBOX_HEIGHT_TILES,
		"inner_x_tiles": 1,
		"inner_y_tiles": TEXTBOX_INNER_Y_TILES,
		"inner_width_tiles": TEXTBOX_WIDTH_TILES - TEXTBOX_BORDER_WIDTH,
		"inner_height_tiles": TEXTBOX_INNER_HEIGHT_TILES,
	}

func build_cursor_line(label: String, active: bool, suffix: String = "") -> String:
	var prefix := "▶" if active else "  "
	var display_label := normalize_text(label)
	var display_suffix := normalize_text(suffix)
	if display_suffix.is_empty():
		return "%s %s" % [prefix, display_label]
	return "%s %s (%s)" % [prefix, display_label, display_suffix]

func build_prompt_lines(options: Array, selection: int) -> Array[String]:
	var lines: Array[String] = []
	for index in range(options.size()):
		lines.append(build_cursor_line(str(options[index]), index == selection))
	return lines

func build_menu_lines(entries: Array, cursor: int, cancelable: bool = true, cancel_label: String = "CANCEL") -> Array[String]:
	var lines: Array[String] = []
	for index in range(entries.size()):
		lines.append(build_cursor_line(_menu_entry_label(entries[index]), index == cursor))
	if cancelable:
		lines.append(build_cursor_line(cancel_label, cursor >= entries.size()))
	return lines

func build_dialogue_control_lines() -> Array[String]:
	return ["A=Advance B=Close"]

func build_prompt_control_lines() -> Array[String]:
	return ["Up/Down=Choose A=OK B=Cancel"]

func normalize_text(text: String) -> String:
	var normalized := text
	for replacement in CONTROL_CODE_REPLACEMENTS:
		normalized = normalized.replace(str(replacement[0]), str(replacement[1]))
	return normalized.replace(POKE_GLYPH, "POKé")

func wrap_text(text: String, max_chars_per_line: int = DEFAULT_CHARS_PER_LINE) -> Array[String]:
	var max_chars: int = max(1, max_chars_per_line)
	var lines: Array[String] = []
	for raw_line in text.split("\n"):
		var line := str(raw_line)
		if line.is_empty():
			lines.append("")
			continue
		var words := _split_words(line)
		var current_line := ""
		for raw_word in words:
			var word := str(raw_word)
			if word.is_empty():
				continue
			if word.find("@") != -1:
				if not current_line.is_empty():
					lines.append(current_line.strip_edges(false, true))
					current_line = ""
				word = word.replace("@", "")
				if word.is_empty():
					continue
			var test_line := word if current_line.is_empty() else "%s %s" % [current_line, word]
			if test_line.length() <= max_chars:
				current_line = test_line
			else:
				if not current_line.is_empty():
					lines.append(current_line)
				current_line = word
		if not current_line.is_empty():
			lines.append(current_line)
	return lines if not lines.is_empty() else [""]

func paginate_text(text: String, lines_per_page: int = DEFAULT_LINES_PER_PAGE, max_chars_per_line: int = DEFAULT_CHARS_PER_LINE) -> Array[String]:
	var wrapped_lines := wrap_text(normalize_text(text), max_chars_per_line)
	var pages: Array[String] = []
	for index in range(0, wrapped_lines.size(), max(1, lines_per_page)):
		var page_lines: Array[String] = []
		for line_index in range(index, min(index + lines_per_page, wrapped_lines.size())):
			page_lines.append(wrapped_lines[line_index])
		pages.append("\n".join(page_lines))
	return pages if not pages.is_empty() else [""]

func get_current_panel() -> Dictionary:
	var page := get_current_page()
	if page.is_empty():
		return {
			"id": get_panel_id(),
			"kind": get_panel_kind(),
			"title": "",
			"page_index": _page_index,
			"page_count": _pages.size(),
			"page": {},
		}
	return {
		"id": get_panel_id(),
		"kind": get_panel_kind(),
		"title": str(page.get("speaker", "")),
		"page_index": _page_index,
		"page_count": _pages.size(),
		"page": page,
	}

func get_state() -> Dictionary:
	var current_page := get_current_page()
	var page_lines := get_current_page_lines()
	return {
		"active": _active,
		"visible": is_visible(),
		"waiting_for_input": is_waiting_for_input(),
		"input_locked": _input_locked,
		"page_index": _page_index,
		"current_page_index": _page_index,
		"page_frame": _page_frame,
		"token_cursor": _page_token_cursor,
		"current_token_index": max(-1, _page_token_cursor - 1),
		"current_token": _page_tokens[_page_token_cursor - 1] if _page_token_cursor > 0 and _page_token_cursor - 1 < _page_tokens.size() else "",
		"current_token_kind": str(_current_token_spec().get("kind", "")),
		"current_token_frame_delay": int(_current_token_spec().get("frame_delay_frames", 0)),
		"current_token_glyph_delay": int(_current_token_spec().get("glyph_delay_frames", 0)),
		"current_token_glyph_cursor": _page_token_glyph_cursor,
		"current_token_glyph_count": int(_current_token_spec().get("glyph_count", 0)),
		"current_token_frame_timer": _page_token_frame_timer,
		"visible_tokens": _page_visible_tokens,
		"page_token_count": _current_page_token_count(),
		"page_tokens": _page_tokens.duplicate(true),
		"page_token_specs": _duplicate_token_specs(_page_token_specs),
		"visible_chars": _page_visible_chars,
		"page_reveal_timer": _page_reveal_timer,
		"pending_waits": _pending_waits,
		"token_wait_pending": _page_wait_token_pending,
		"pending_script_waits": _pending_waits,
		"auto_close_after_wait": _auto_close_after_wait,
		"page_complete": is_complete(),
		"cursor": get_page_cursor(),
		"page_cursor": get_page_cursor(),
		"page_count": _pages.size(),
		"page": current_page.duplicate(true),
		"current_page": current_page.duplicate(true),
		"current_text": str(current_page.get("text", "")),
		"current_page_text": get_current_page_text(),
		"visible_text": get_visible_text(),
		"page_visible_text": get_visible_text(),
		"display_lines": page_lines,
		"dialogue_lines": page_lines,
		"pages": _duplicate_pages(_pages),
		"dialogue_pages": _duplicate_pages(_pages),
		"page_list": _duplicate_pages(_pages),
		"has_more_pages": has_more_pages(),
		"page_input_ready": is_page_input_ready(),
		"input_delay_frames": _current_page_input_delay(),
		"can_advance": can_advance(),
		"can_accept_input": can_accept_input(),
		"input_owned": is_input_owned(),
		"dialog_active": is_dialog_active(),
		"text_box_open": is_text_box_open(),
	}

func from_state(data: Dictionary) -> void:
	if data.is_empty():
		reset()
		return
	var page_source: Variant = data.get(
		"pages",
		data.get("dialogue_pages", data.get("page_list", data.get("text_pages", data.get("current_page", data.get("page", []))))),
	)
	_pages = _normalize_pages(page_source)
	if _pages.is_empty():
		var fallback_text := str(data.get("current_text", data.get("visible_text", data.get("current_page_text", ""))))
		if not fallback_text.is_empty():
			_pages = [_normalize_page({
				"text": fallback_text,
				"speaker": str(data.get("speaker", "")),
				"wait_for_input": bool(data.get("waiting_for_input", true)),
				"meta": Dictionary(data.get("meta", {})),
			})]
	_page_index = clampi(_page_index_from_state(data), -1, max(-1, _pages.size() - 1))
	_page_frame = max(0, _page_frame_from_state(data))
	_restore_page_tokens_from_state(data)
	_page_visible_chars = max(0, int(data.get("visible_chars", data.get("page_visible_chars", 0))))
	_page_reveal_timer = max(0, int(data.get("page_reveal_timer", 0)))
	_pending_waits = max(0, int(data.get("pending_waits", data.get("pendingWaits", data.get("pending_script_waits", data.get("pendingScriptWaits", 0))))))
	_auto_close_after_wait = bool(data.get("auto_close_after_wait", data.get("autoCloseAfterWait", false)))
	_active = bool(data.get("active", data.get("visible", data.get("dialog_active", data.get("text_box_open", not _pages.is_empty())))))
	_input_locked = bool(data.get("input_locked", false))
	if _pages.is_empty():
		_page_index = -1
		_page_frame = 0
		_page_tokens = []
		_page_token_cursor = 0
		_page_visible_tokens = 0
		_page_visible_chars = 0
		_page_reveal_timer = 0
		_pending_waits = 0
		_auto_close_after_wait = false
		_active = false
	elif _page_index < 0:
		_page_index = 0
		_page_frame = 0
		_sync_page_tokens()
		_page_visible_chars = _initial_visible_chars()
		_page_reveal_timer = 0
	elif _page_index >= _pages.size():
		_page_index = _pages.size() - 1
		_sync_page_tokens()
		_page_visible_chars = _initial_visible_chars()
		_page_reveal_timer = 0
	if _active and _page_index >= 0 and _page_index < _pages.size():
		var active_page: Dictionary = _pages[_page_index].duplicate(true)
		var page_override: Variant = data.get("current_page", data.get("page", {}))
		if typeof(page_override) == TYPE_DICTIONARY:
			active_page.merge(Dictionary(page_override), true)
		var override_text := str(data.get("current_text", data.get("current_page_text", data.get("visible_text", ""))))
		if not override_text.is_empty():
			active_page["text"] = override_text
		var override_display_text := str(data.get("display_text", data.get("visible_text", "")))
		if not override_display_text.is_empty():
			active_page["display_text"] = override_display_text
		var override_lines: Variant = data.get("display_lines", data.get("dialogue_lines", []))
		if typeof(override_lines) == TYPE_ARRAY and not Array(override_lines).is_empty():
			active_page["display_lines"] = Array(override_lines).duplicate(true)
		_pages[_page_index] = _normalize_page(active_page)
		_sync_page_tokens()
		if _page_visible_chars <= 0:
			_page_visible_chars = _initial_visible_chars()
	if bool(data.get("page_complete", false)):
		complete()

func advance() -> Dictionary:
	var result := {
		"consumed": false,
		"advanced": false,
		"closed": false,
		"page_index": _page_index,
		"page_count": _pages.size(),
		"page_frame": _page_frame,
		"waiting_for_input": is_waiting_for_input(),
		"page_input_ready": is_page_input_ready(),
		"input_locked": _input_locked,
		"can_advance": can_advance(),
	}
	if not _active or _input_locked:
		return result
	result["consumed"] = true
	if not is_complete():
		complete()
		result["completed"] = true
		result["page_visible_text"] = get_visible_text()
		result["visible_text"] = get_visible_text()
		return result
	if (_pending_waits > 0 or is_waiting_for_input()) and _page_index < _pages.size() - 1:
		_page_index += 1
		_page_frame = 0
		_page_visible_tokens = 0
		_page_token_cursor = 0
		_sync_page_tokens()
		_page_visible_chars = _initial_visible_chars()
		_page_reveal_timer = 0
		result["advanced"] = true
		result["page_index"] = _page_index
		result["page_frame"] = _page_frame
		result["page_input_ready"] = is_page_input_ready()
		result["can_advance"] = can_advance()
		page_changed.emit(_page_index, _pages.size())
		return result
	if _page_index < _pages.size() - 1:
		_page_index += 1
		_page_frame = 0
		_page_visible_tokens = 0
		_page_token_cursor = 0
		_sync_page_tokens()
		_page_visible_chars = _initial_visible_chars()
		_page_reveal_timer = 0
		result["advanced"] = true
		result["page_index"] = _page_index
		result["page_frame"] = _page_frame
		result["page_input_ready"] = is_page_input_ready()
		result["can_advance"] = can_advance()
		page_changed.emit(_page_index, _pages.size())
	else:
		close_dialogue()
		result["closed"] = true
		result["page_index"] = _page_index
		result["page_frame"] = _page_frame
		result["page_input_ready"] = is_page_input_ready()
		result["can_advance"] = can_advance()
	return result

func consume_input(frame_input: Dictionary) -> Dictionary:
	var result := {
		"consumed": false,
		"advanced": false,
		"closed": false,
		"page_index": _page_index,
		"page_count": _pages.size(),
		"page_frame": _page_frame,
		"visible_chars": _page_visible_chars,
		"page": get_current_page(),
		"waiting_for_input": is_waiting_for_input(),
		"page_input_ready": is_page_input_ready(),
		"page_complete": is_complete(),
		"input_locked": _input_locked,
		"can_advance": can_advance(),
		"visible_text": get_visible_text(),
	}
	if not _active:
		return result
	if _input_locked:
		result["consumed"] = _has_any_pressed_button(frame_input)
		return result
	var pressed: Dictionary = Dictionary(frame_input.get("pressed", {}))
	tick()
	result["page_frame"] = _page_frame
	result["visible_chars"] = _page_visible_chars
	result["page_input_ready"] = is_page_input_ready()
	result["page_complete"] = is_complete()
	result["can_advance"] = can_advance()
	if _has_confirm_pressed(pressed) and (_page_wait_token_pending or str(_current_token_spec().get("kind", "")) == "wait"):
		result["consumed"] = acknowledge_wait()
		result["waiting_for_input"] = is_waiting_for_input()
		result["page_input_ready"] = is_page_input_ready()
		result["page_complete"] = is_complete()
		result["can_advance"] = can_advance()
		result["visible_chars"] = _page_visible_chars
		result["visible_text"] = get_visible_text()
		result["page"] = get_current_page()
		return result
	if _has_confirm_pressed(pressed) and can_advance():
		result = advance()
		result["page"] = get_current_page()
		return result
	if _has_confirm_pressed(pressed) and is_page_input_ready() and (_pending_waits > 0 or is_waiting_for_input()) and has_more_pages():
		result = advance()
		result["page"] = get_current_page()
		return result
	if _has_confirm_pressed(pressed) and _pending_waits > 0:
		result["consumed"] = acknowledge_wait()
		result["pending_waits"] = _pending_waits
		result["pending_script_waits"] = _pending_waits
		result["waiting_for_input"] = is_waiting_for_input()
		result["page_complete"] = is_complete()
		result["can_advance"] = can_advance()
		if _pending_waits == 0 and _auto_close_after_wait and is_complete() and not has_more_pages():
			close_dialogue()
			result["closed"] = true
			result["active"] = false
			result["visible"] = false
		return result
	if _has_confirm_pressed(pressed) and is_complete() == false:
		complete()
		result["consumed"] = true
		result["completed"] = true
		result["page_complete"] = is_complete()
		result["visible_chars"] = _page_visible_chars
		result["visible_text"] = get_visible_text()
		return result
	result["consumed"] = _has_any_pressed_button(frame_input)
	return result

func _has_confirm_pressed(pressed: Dictionary) -> bool:
	for button in CONFIRM_BUTTONS:
		if bool(pressed.get(button, false)):
			return true
	return false

func _has_any_pressed_button(frame_input: Dictionary) -> bool:
	var pressed: Dictionary = Dictionary(frame_input.get("pressed", {}))
	for button in pressed.keys():
		if bool(pressed.get(button, false)):
			return true
	return false

func _normalize_pages(content: Variant) -> Array[Dictionary]:
	var pages: Array[Dictionary] = []
	match typeof(content):
		TYPE_STRING:
			for page_text in paginate_text(str(content)):
				pages.append(_normalize_page({"text": page_text}))
		TYPE_ARRAY:
			var source: Array = content
			for entry in source:
				if _is_page_list_container(entry):
					pages.append_array(_normalize_pages(_page_list_from_container(Dictionary(entry))))
				else:
					pages.append(_normalize_page(entry))
		TYPE_DICTIONARY:
			var source_dictionary: Dictionary = content
			if _is_page_list_container(source_dictionary):
				var nested_pages := _normalize_pages(_page_list_from_container(source_dictionary))
				if not nested_pages.is_empty():
					return nested_pages
			pages.append(_normalize_page(source_dictionary))
		_:
			pass
	return pages

func _page_index_from_state(data: Dictionary) -> int:
	if data.has("page_index"):
		return int(data.get("page_index", 0))
	var cursor: Variant = data.get("page_cursor", data.get("cursor", {}))
	if typeof(cursor) == TYPE_DICTIONARY:
		var cursor_dictionary: Dictionary = cursor
		return int(cursor_dictionary.get("page_index", cursor_dictionary.get("index", data.get("page_index", data.get("current_page_index", 0)))))
	if typeof(cursor) == TYPE_INT or typeof(cursor) == TYPE_FLOAT:
		return int(data.get("page_index", data.get("current_page_index", cursor)))
	return int(data.get("page_index", data.get("current_page_index", 0)))

func _page_frame_from_state(data: Dictionary) -> int:
	if data.has("page_frame"):
		return int(data.get("page_frame", 0))
	var cursor: Variant = data.get("page_cursor", data.get("cursor", {}))
	if typeof(cursor) == TYPE_DICTIONARY:
		return int(Dictionary(cursor).get("page_frame", 0))
	return 0

func _duplicate_pages(pages: Array[Dictionary]) -> Array:
	var copy: Array = []
	for page in pages:
		copy.append(page.duplicate(true))
	return copy

func _normalize_page(entry: Variant) -> Dictionary:
	var page := {
		"text": "",
		"display_text": "",
		"speaker": "",
		"wait_for_input": true,
		"input_delay_frames": DEFAULT_INPUT_DELAY_FRAMES,
		"reveal_delay_frames": 0,
		"reveal_chars_per_tick": 1,
		"meta": {},
		"display_lines": [],
	}
	match typeof(entry):
		TYPE_STRING:
			page["text"] = str(entry)
		TYPE_DICTIONARY:
			var source: Dictionary = entry
			if source.has("text"):
				page["text"] = str(source["text"])
			elif source.has("body"):
				page["text"] = str(source["body"])
			elif source.has("display_text"):
				page["text"] = str(source["display_text"])
			elif source.has("current_text"):
				page["text"] = str(source["current_text"])
			elif source.has("visible_text"):
				page["text"] = str(source["visible_text"])
			elif source.has("current_page_text"):
				page["text"] = str(source["current_page_text"])
			else:
				page["text"] = _page_text_from_dictionary(source)
			page["speaker"] = str(source.get("speaker", source.get("title", source.get("name", ""))))
			page["wait_for_input"] = bool(source.get("wait_for_input", true))
			page["input_delay_frames"] = max(0, int(source.get("input_delay_frames", source.get("min_frames", DEFAULT_INPUT_DELAY_FRAMES))))
			page["reveal_delay_frames"] = _first_non_negative_int(source, ["reveal_delay_frames", "typewriter_delay_frames", "text_delay_frames", "char_delay_frames"], 0)
			page["reveal_chars_per_tick"] = max(1, _first_non_negative_int(source, ["reveal_chars_per_tick", "chars_per_tick", "characters_per_tick", "chars_per_frame"], 1))
			page["meta"] = _normalize_meta(source.get("meta", {}))
		_:
			page["text"] = str(entry)
	page["display_text"] = normalize_text(str(page["text"]))
	page["display_lines"] = wrap_text(str(page["display_text"]))
	return page

func _sync_page_tokens() -> void:
	_page_token_specs = _build_token_specs(_current_page_visible_source_text())
	_page_tokens = _token_texts_from_specs(_page_token_specs)
	if _current_page_reveal_delay() <= 0 and _page_visible_tokens <= 0 and not _page_wait_token_pending:
		var wait_index := _first_wait_token_index()
		if wait_index >= 0:
			_page_visible_tokens = _visible_token_count_before_index(wait_index)
			_page_token_cursor = wait_index
			_page_token_glyph_cursor = 0
			_page_token_frame_timer = 0
			_page_wait_token_pending = true
		else:
			_page_visible_tokens = _page_token_specs.size()
			_page_token_cursor = _page_token_specs.size()
			_page_token_glyph_cursor = 0
			_page_token_frame_timer = 0
	else:
		_page_visible_tokens = clampi(_page_visible_tokens, 0, _page_token_specs.size())
		_page_token_cursor = clampi(_page_token_cursor, 0, _page_token_specs.size())
		_page_token_glyph_cursor = clampi(_page_token_glyph_cursor, 0, _current_token_glyph_count())
	_page_visible_chars = _current_page_visible_text().length()

func _restore_page_tokens_from_state(data: Dictionary) -> void:
	var page_token_specs_source: Variant = data.get("page_token_specs", data.get("token_specs", []))
	if typeof(page_token_specs_source) == TYPE_ARRAY:
		_page_token_specs = _normalize_token_specs(Array(page_token_specs_source))
		_page_tokens = _token_texts_from_specs(_page_token_specs)
	else:
		var page_tokens_source: Variant = data.get("page_tokens", data.get("tokens", []))
		if typeof(page_tokens_source) == TYPE_ARRAY:
			_page_tokens = _string_array(Array(page_tokens_source))
		_page_token_specs = _build_token_specs(_current_page_visible_source_text())
		if _page_tokens.is_empty():
			_page_tokens = _token_texts_from_specs(_page_token_specs)
	_page_visible_tokens = max(0, int(data.get("visible_tokens", data.get("page_visible_tokens", data.get("token_cursor", data.get("page_token_cursor", 0))))))
	_page_token_cursor = max(0, int(data.get("token_cursor", data.get("page_token_cursor", _page_visible_tokens))))
	_page_token_glyph_cursor = max(0, int(data.get("current_token_glyph_cursor", data.get("token_glyph_cursor", 0))))
	_page_token_frame_timer = max(0, int(data.get("current_token_frame_timer", data.get("token_frame_timer", 0))))
	_page_wait_token_pending = bool(data.get("token_wait_pending", data.get("page_wait_token_pending", false)))
	_page_visible_tokens = clampi(_page_visible_tokens, 0, _page_token_specs.size())
	_page_token_cursor = clampi(_page_token_cursor, 0, _page_token_specs.size())
	_page_token_glyph_cursor = clampi(_page_token_glyph_cursor, 0, _current_token_glyph_count())
	_page_visible_chars = _current_page_visible_text().length()

func _current_page_visible_source_text() -> String:
	var page := _current_page_for_tokens()
	if page.is_empty():
		return ""
	return str(page.get("display_text", page.get("text", "")))

func _current_page_token_count() -> int:
	return _page_token_specs.size()

func _current_page_for_tokens() -> Dictionary:
	if _page_index < 0 or _page_index >= _pages.size():
		return {}
	return _pages[_page_index].duplicate(true)

func _current_token_spec() -> Dictionary:
	if _page_token_cursor < 0 or _page_token_cursor >= _page_token_specs.size():
		return {}
	return _page_token_specs[_page_token_cursor]

func _current_token_glyph_count() -> int:
	return int(_current_token_spec().get("glyph_count", 0))

func _current_token_text() -> String:
	return str(_current_token_spec().get("text", ""))

func _visible_token_count_before_cursor() -> int:
	var count := 0
	for index in range(min(_page_token_cursor, _page_token_specs.size())):
		var spec := Dictionary(_page_token_specs[index])
		if str(spec.get("kind", "text")) != "wait":
			count += 1
	return count

func _visible_token_count_before_index(limit: int) -> int:
	var count := 0
	for index in range(min(limit, _page_token_specs.size())):
		var spec := Dictionary(_page_token_specs[index])
		if str(spec.get("kind", "text")) != "wait":
			count += 1
	return count

func _first_wait_token_index() -> int:
	for index in range(_page_token_specs.size()):
		if str(Dictionary(_page_token_specs[index]).get("kind", "")) == "wait":
			return index
	return -1

func _build_token_specs(text: String) -> Array[Dictionary]:
	var specs: Array[Dictionary] = []
	var current := ""
	var index := 0
	while index < text.length():
		var wait_match := _match_wait_token(text, index)
		if wait_match["matched"]:
			if not current.is_empty():
				specs.append(_make_token_spec(current))
				current = ""
			specs.append(_make_wait_token_spec(str(wait_match["token"])))
			index += int(wait_match["length"])
			continue
		var char := text.substr(index, 1)
		if char == "\n" or char == " " or char == "\t" or char == "\r":
			if not current.is_empty():
				specs.append(_make_token_spec(current))
				current = ""
			specs.append(_make_token_spec(char))
			index += 1
			continue
		current += char
		index += 1
	if not current.is_empty():
		specs.append(_make_token_spec(current))
	return specs

func _normalize_token_specs(value: Array) -> Array[Dictionary]:
	var result: Array[Dictionary] = []
	for entry in value:
		if typeof(entry) == TYPE_DICTIONARY:
			result.append(_normalize_token_spec(Dictionary(entry)))
		else:
			result.append(_make_token_spec(str(entry)))
	return result

func _normalize_token_spec(entry: Dictionary) -> Dictionary:
	var kind := str(entry.get("kind", entry.get("type", "text")))
	var text := str(entry.get("text", entry.get("value", "")))
	var glyph_count: int = max(0, int(entry.get("glyph_count", entry.get("glyphCount", text.length()))))
	var frame_delay: int = max(0, int(entry.get("frame_delay_frames", entry.get("frameDelayFrames", entry.get("delay_frames", 0)))))
	var glyph_delay: int = max(0, int(entry.get("glyph_delay_frames", entry.get("glyphDelayFrames", entry.get("per_glyph_frames", 1)))))
	return {
		"kind": kind,
		"text": text,
		"glyph_count": glyph_count,
		"frame_delay_frames": frame_delay,
		"glyph_delay_frames": glyph_delay,
		"requires_ack": bool(entry.get("requires_ack", entry.get("requiresAck", kind == "wait"))),
	}

func _make_token_spec(text: String) -> Dictionary:
	var kind := "text"
	if text == "\n":
		kind = "newline"
	elif text == " " or text == "\t" or text == "\r":
		kind = "space"
	return {
		"kind": kind,
		"text": text,
		"glyph_count": text.length(),
		"frame_delay_frames": max(0, _current_page_reveal_delay()),
		"glyph_delay_frames": max(1, _current_page_reveal_step()),
		"requires_ack": false,
	}

func _duplicate_token_specs(specs: Array[Dictionary]) -> Array:
	var copy: Array = []
	for spec in specs:
		copy.append(spec.duplicate(true))
	return copy

func _make_wait_token_spec(token_text: String) -> Dictionary:
	return {
		"kind": "wait",
		"text": token_text,
		"glyph_count": 0,
		"frame_delay_frames": 0,
		"glyph_delay_frames": 0,
		"requires_ack": true,
	}

func _match_wait_token(text: String, index: int) -> Dictionary:
	for token in WAIT_CONTROL_TOKENS:
		if text.substr(index, token.length()) == token:
			return {"matched": true, "token": token, "length": token.length()}
	return {"matched": false, "token": "", "length": 0}

func _token_texts_from_specs(specs: Array[Dictionary]) -> Array[String]:
	var result: Array[String] = []
	for spec in specs:
		var kind := str(spec.get("kind", "text"))
		if kind == "wait":
			result.append(str(spec.get("text", WAIT_CONTROL_TOKENS[0])))
		else:
			result.append(str(spec.get("text", "")))
	return result

func _current_page_input_delay() -> int:
	if _page_index < 0 or _page_index >= _pages.size():
		return 0
	return max(0, int(_pages[_page_index].get("input_delay_frames", 0)))

func _current_page_reveal_delay() -> int:
	if _page_index < 0 or _page_index >= _pages.size():
		return 0
	var page := _pages[_page_index]
	for key in ["reveal_delay_frames", "typewriter_delay_frames", "text_delay_frames", "char_delay_frames"]:
		if page.has(key):
			return max(0, int(page.get(key, 0)))
	return 0

func _current_page_reveal_step() -> int:
	if _page_index < 0 or _page_index >= _pages.size():
		return 1
	var page := _pages[_page_index]
	for key in ["reveal_chars_per_tick", "chars_per_tick", "characters_per_tick", "chars_per_frame"]:
		if page.has(key):
			return max(1, int(page.get(key, 1)))
	return 1

func _current_page_text_length() -> int:
	return get_current_page_text().length()

func _initial_visible_chars() -> int:
	if _page_index < 0 or _page_index >= _pages.size():
		return 0
	if _current_page_reveal_delay() <= 0:
		return _current_page_visible_text().length()
	return 0

func _step_typewriter() -> void:
	if not _active or _page_index < 0 or _page_index >= _pages.size():
		return
	var token_count := _current_page_token_count()
	if token_count <= 0:
		_page_visible_chars = 0
		_page_visible_tokens = 0
		_page_token_cursor = 0
		_page_token_glyph_cursor = 0
		_page_token_frame_timer = 0
		_page_wait_token_pending = false
		return
	if _page_wait_token_pending:
		return
	if _page_token_cursor >= token_count:
		return
	var current_spec := Dictionary(_page_token_specs[_page_token_cursor])
	if str(current_spec.get("kind", "text")) == "wait":
		_page_wait_token_pending = bool(current_spec.get("requires_ack", true))
		_page_token_frame_timer = 0
		_page_token_glyph_cursor = 0
		_page_visible_chars = _current_page_visible_text().length()
		return
	var token_delay: int = max(0, int(current_spec.get("frame_delay_frames", _current_page_reveal_delay())))
	var glyph_step: int = max(1, int(current_spec.get("glyph_delay_frames", _current_page_reveal_step())))
	_page_token_frame_timer += 1
	if token_delay > 0 and _page_token_frame_timer < token_delay:
		return
	if token_delay > 0:
		_page_token_frame_timer = 0
	_page_token_glyph_cursor = min(int(current_spec.get("glyph_count", 0)), _page_token_glyph_cursor + glyph_step)
	if _page_token_glyph_cursor >= int(current_spec.get("glyph_count", 0)):
		_page_visible_tokens = max(_page_visible_tokens, _page_token_cursor + 1)
		_page_token_cursor += 1
		_page_token_glyph_cursor = 0
		_page_token_frame_timer = 0
		if _page_token_cursor < token_count:
			var next_spec := Dictionary(_page_token_specs[_page_token_cursor])
			if str(next_spec.get("kind", "text")) == "wait" and bool(next_spec.get("requires_ack", true)):
				_page_wait_token_pending = true
				_page_visible_chars = _current_page_visible_text().length()
				return
		_page_visible_tokens = max(_page_visible_tokens, _page_token_cursor)
		if _page_token_cursor >= token_count:
			_page_visible_tokens = token_count
	_page_visible_chars = _current_page_visible_text().length()

func _normalize_meta(value: Variant) -> Dictionary:
	if typeof(value) != TYPE_DICTIONARY:
		return {}
	var result: Dictionary = {}
	var source: Dictionary = value
	for key in source.keys():
		result[key] = _normalize_meta_value(source[key])
	return result

func _first_non_negative_int(source: Dictionary, keys: Array[String], fallback: int) -> int:
	for key in keys:
		if source.has(key):
			return max(0, int(source.get(key, fallback)))
	return max(0, fallback)

func _normalize_meta_value(value: Variant) -> Variant:
	match typeof(value):
		TYPE_DICTIONARY:
			return _normalize_meta(value)
		TYPE_ARRAY:
			var normalized: Array = []
			var source: Array = value
			for entry in source:
				normalized.append(_normalize_meta_value(entry))
			return normalized
		TYPE_STRING, TYPE_INT, TYPE_FLOAT, TYPE_BOOL, TYPE_NIL:
			return value
		_:
			return null

func _menu_entry_label(entry: Variant) -> String:
	match typeof(entry):
		TYPE_STRING:
			return normalize_text(str(entry))
		TYPE_DICTIONARY:
			var source: Dictionary = entry
			return normalize_text(str(source.get("label", source.get("text", source.get("title", source.get("id", ""))))))
		_:
			return normalize_text(str(entry))

func _is_page_list_container(value: Variant) -> bool:
	if typeof(value) != TYPE_DICTIONARY:
		return false
	var source: Dictionary = value
	return source.has("pages") or source.has("dialogue_pages") or source.has("page_list") or source.has("text_pages")

func _page_list_from_container(source: Dictionary) -> Variant:
	return source.get("pages", source.get("dialogue_pages", source.get("page_list", source.get("text_pages", []))))

func _page_text_from_dictionary(source: Dictionary) -> String:
	var text_value: Variant = source.get(
		"text",
		source.get("body", source.get("display_text", source.get("current_text", source.get("visible_text", source.get("current_page_text", ""))))),
	)
	if typeof(text_value) == TYPE_ARRAY:
		return "\n".join(_string_array(text_value))
	if typeof(text_value) == TYPE_STRING and not str(text_value).is_empty():
		return str(text_value)
	var line_value: Variant = null
	for key in ["lines", "page_lines", "display_lines", "dialogue_lines"]:
		if source.has(key):
			line_value = source.get(key)
			break
	if typeof(line_value) == TYPE_ARRAY:
		return "\n".join(_string_array(line_value))
	return str(text_value)

func _string_array(value: Array) -> Array[String]:
	var result: Array[String] = []
	for entry in value:
		if typeof(entry) == TYPE_ARRAY:
			result.append(" ".join(_string_array(Array(entry))))
		else:
			result.append(str(entry))
	return result

func _split_words(text: String) -> Array[String]:
	var words: Array[String] = []
	var current := ""
	for index in range(text.length()):
		var char := text.substr(index, 1)
		if char == " " or char == "\t" or char == "\r":
			if not current.is_empty():
				words.append(current)
				current = ""
			continue
		current += char
	if not current.is_empty():
		words.append(current)
	return words
