extends RefCounted
class_name BattleDialogueState

const DEFAULT_LINES_PER_PAGE := 2
const DEFAULT_CHARS_PER_LINE := 18

var queue: Array[Dictionary] = []
var pending_waits: int = 0
var forced_visible: bool = false
var auto_close_after_display: bool = false
var current_text: String = ""
var current_control: String = ""
var current_page: int = 0
var current_pages: Array[Dictionary] = []
var dialogue_complete: bool = true
var input_locked: bool = false
var last_action: String = ""

func reset() -> void:
	queue = []
	pending_waits = 0
	forced_visible = false
	auto_close_after_display = false
	current_text = ""
	current_control = ""
	current_page = 0
	current_pages = []
	dialogue_complete = true
	input_locked = false
	last_action = ""

func enqueue_text(text: String, control: String = "") -> bool:
	var normalized := _normalize_text(text)
	if normalized.is_empty():
		return false
	queue.append({
		"pages": _paginate_text(normalized),
		"control": control.strip_edges(),
	})
	if not forced_visible:
		start_next_dialogue()
	return true

func push_wait() -> void:
	pending_waits += 1
	forced_visible = true

func force_text_box(value: bool) -> void:
	forced_visible = value
	if not value and pending_waits <= 0 and queue.is_empty():
		current_pages = []
		current_text = ""
		dialogue_complete = true

func close_text_box() -> void:
	queue = []
	pending_waits = 0
	forced_visible = false
	auto_close_after_display = false
	current_text = ""
	current_control = ""
	current_page = 0
	current_pages = []
	dialogue_complete = true
	input_locked = false

func start_next_dialogue() -> void:
	if queue.is_empty():
		forced_visible = false
		current_text = ""
		current_control = ""
		current_page = 0
		current_pages = []
		dialogue_complete = true
		return
	var entry: Dictionary = queue.pop_front()
	current_pages = _sanitize_pages(entry.get("pages", []))
	current_control = str(entry.get("control", ""))
	current_page = 0
	current_text = _current_page_text()
	dialogue_complete = false
	forced_visible = true
	auto_close_after_display = current_control == "done"

func advance_dialogue() -> bool:
	if input_locked:
		return false
	if forced_visible:
		if not dialogue_complete:
			if current_page < current_pages.size() - 1:
				current_page += 1
				current_text = _current_page_text()
				last_action = "page:%d" % current_page
				return true
			dialogue_complete = true
			last_action = "complete"
			return true
		if not queue.is_empty():
			start_next_dialogue()
			last_action = "next"
			return true
		if dialogue_complete:
			close_text_box()
			last_action = "close"
			return true
	if pending_waits > 0:
		pending_waits = max(0, pending_waits - 1)
		if pending_waits == 0 and queue.is_empty():
			forced_visible = false
		last_action = "wait:%d" % pending_waits
		return true
	return false

func consume_input(action: String) -> bool:
	var normalized := action.strip_edges().to_lower()
	if normalized in ["a", "b", "start", "confirm", "advance"]:
		return advance_dialogue()
	return false

func is_visible() -> bool:
	return forced_visible or pending_waits > 0

func waiting_flag() -> bool:
	return pending_waits > 0 or forced_visible or not queue.is_empty() or not dialogue_complete or input_locked

func auto_close_if_idle(prompt_active: bool) -> bool:
	if not auto_close_after_display:
		return false
	if prompt_active or pending_waits > 0 or not queue.is_empty():
		return false
	if not dialogue_complete:
		return false
	forced_visible = false
	auto_close_after_display = false
	current_text = ""
	current_control = ""
	current_page = 0
	current_pages = []
	return true

func set_input_locked(value: bool) -> void:
	input_locked = bool(value)

func get_page_count() -> int:
	return current_pages.size()

func has_more_pages() -> bool:
	return forced_visible and current_page >= 0 and current_page < current_pages.size() - 1

func get_current_page() -> Dictionary:
	if current_page < 0 or current_page >= current_pages.size():
		return {}
	return current_pages[current_page].duplicate(true)

func get_state() -> Dictionary:
	return to_dictionary()

func to_dictionary() -> Dictionary:
	return {
		"queue": _duplicate_queue(queue),
		"pending_waits": pending_waits,
		"forced_visible": forced_visible,
		"auto_close_after_display": auto_close_after_display,
		"current_text": current_text,
		"current_control": current_control,
		"current_page": current_page,
		"page_index": current_page,
		"page_count": current_pages.size(),
		"current_pages": _duplicate_pages(current_pages),
		"pages": _duplicate_pages(current_pages),
		"dialogue_complete": dialogue_complete,
		"input_locked": input_locked,
		"visible": is_visible(),
		"waiting_for_input": waiting_flag(),
		"has_more_pages": has_more_pages(),
		"last_action": last_action,
	}

func from_dictionary(data: Variant) -> bool:
	if typeof(data) != TYPE_DICTIONARY:
		return false
	from_state(Dictionary(data))
	return true

func from_state(data: Dictionary) -> void:
	reset()
	if data.is_empty():
		return
	queue = _sanitize_queue(data.get("queue", []))
	pending_waits = max(0, int(data.get("pending_waits", 0)))
	forced_visible = bool(data.get("forced_visible", data.get("visible", false)))
	auto_close_after_display = bool(data.get("auto_close_after_display", false))
	current_control = str(data.get("current_control", ""))
	current_pages = _sanitize_pages(data.get("current_pages", data.get("pages", [])))
	if current_pages.is_empty():
		var fallback_text := _normalize_text(str(data.get("current_text", "")))
		if not fallback_text.is_empty():
			current_pages = _paginate_text(fallback_text)
	current_page = clampi(int(data.get("current_page", data.get("page_index", 0))), 0, max(0, current_pages.size() - 1))
	current_text = _current_page_text()
	dialogue_complete = bool(data.get("dialogue_complete", not forced_visible and current_pages.is_empty()))
	input_locked = bool(data.get("input_locked", false))
	last_action = str(data.get("last_action", ""))
	if current_pages.is_empty() and queue.is_empty() and pending_waits <= 0:
		forced_visible = false
		dialogue_complete = true

func _current_page_text() -> String:
	var page := get_current_page()
	if page.is_empty():
		return ""
	return str(page.get("text", ""))

func _normalize_text(text: String) -> String:
	return text.strip_edges().replace("\r\n", "\n").replace("\r", "\n")

func _paginate_text(text: String) -> Array[Dictionary]:
	var lines := _wrap_text(text, DEFAULT_CHARS_PER_LINE)
	var pages: Array[Dictionary] = []
	for index in range(0, lines.size(), DEFAULT_LINES_PER_PAGE):
		var page_lines: Array[String] = []
		for line_index in range(index, min(index + DEFAULT_LINES_PER_PAGE, lines.size())):
			page_lines.append(lines[line_index])
		pages.append({
			"text": "\n".join(page_lines),
			"lines": page_lines,
		})
	if pages.is_empty():
		pages.append({"text": "", "lines": [""]})
	return pages

func _wrap_text(text: String, max_chars_per_line: int) -> Array[String]:
	var wrapped: Array[String] = []
	for raw_line in text.split("\n"):
		var line := str(raw_line).strip_edges()
		if line.is_empty():
			wrapped.append("")
			continue
		var current := ""
		for raw_word in line.split(" ", false):
			var word := str(raw_word)
			var next_line := word if current.is_empty() else "%s %s" % [current, word]
			if next_line.length() <= max_chars_per_line:
				current = next_line
			else:
				if not current.is_empty():
					wrapped.append(current)
				current = word
		if not current.is_empty():
			wrapped.append(current)
	return wrapped if not wrapped.is_empty() else [""]

func _sanitize_pages(value: Variant) -> Array[Dictionary]:
	var pages: Array[Dictionary] = []
	if typeof(value) != TYPE_ARRAY:
		return pages
	for entry in Array(value):
		if typeof(entry) == TYPE_DICTIONARY:
			var page: Dictionary = Dictionary(entry).duplicate(true)
			var text := _normalize_text(str(page.get("text", page.get("display_text", ""))))
			var lines: Array[String] = []
			var raw_lines: Array = Array(page.get("lines", page.get("display_lines", [])))
			for line in raw_lines:
				lines.append(str(line))
			if lines.is_empty() and not text.is_empty():
				lines = _wrap_text(text, DEFAULT_CHARS_PER_LINE)
			page["text"] = text if not text.is_empty() else "\n".join(lines)
			page["lines"] = lines
			pages.append(page)
		else:
			var normalized_text := _normalize_text(str(entry))
			if not normalized_text.is_empty():
				pages.append_array(_paginate_text(normalized_text))
	return pages

func _sanitize_queue(value: Variant) -> Array[Dictionary]:
	var result: Array[Dictionary] = []
	if typeof(value) != TYPE_ARRAY:
		return result
	for entry in Array(value):
		if typeof(entry) != TYPE_DICTIONARY:
			continue
		var source: Dictionary = entry
		result.append({
			"pages": _sanitize_pages(source.get("pages", [])),
			"control": str(source.get("control", "")),
		})
	return result

func _duplicate_pages(pages: Array[Dictionary]) -> Array:
	var result: Array = []
	for page in pages:
		result.append(page.duplicate(true))
	return result

func _duplicate_queue(entries: Array[Dictionary]) -> Array:
	var result: Array = []
	for entry in entries:
		result.append(entry.duplicate(true))
	return result
