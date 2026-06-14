extends RefCounted
class_name BattleUIState

const BATTLE_STATE_SCRIPT := preload("res://scripts/battle_state.gd")
const BATTLE_DIALOGUE_SCRIPT := preload("res://scripts/battle_dialogue.gd")

var battle_state = null
var dialogue = null
var ui_phase: String = "INACTIVE"
var submenu_stack: Array[String] = []
var submenu_index: int = 0
var command_menu_entries: Array[Dictionary] = []
var latched_command: Dictionary = {}
var prompt_gate_state: Dictionary = {}
var last_action: String = ""
var presentation_signature: String = ""
var last_resolution_drain: Array = []

func _init() -> void:
	dialogue = BATTLE_DIALOGUE_SCRIPT.new()

func bind_battle_state(state) -> void:
	battle_state = state
	sync_from_battle_state()

func reset() -> void:
	submenu_stack = []
	submenu_index = 0
	command_menu_entries = []
	latched_command = {}
	prompt_gate_state = {}
	last_action = ""
	presentation_signature = ""
	last_resolution_drain = []
	if dialogue != null:
		dialogue.reset()
	sync_from_battle_state()

func sync_from_battle_state() -> void:
	if battle_state == null:
		return
	if battle_state.has_method("sync_ui_state"):
		battle_state.sync_ui_state()
	ui_phase = str(battle_state.ui_phase)
	prompt_gate_state = _read_prompt_gate_state()
	if dialogue != null and dialogue.auto_close_if_idle(bool(prompt_gate_state.get("active", false))):
		last_action = "dialogue:auto-close"
	presentation_signature = "%s|%s|%s|%d|%d|%d|%d|%d|%d" % [
		str(battle_state.battle_label),
		str(battle_state.battle_kind),
		ui_phase,
		int(battle_state.state_revision),
		int(submenu_stack.hash()),
		int(command_menu_entries.hash()),
		int(latched_command.hash()),
		int(prompt_gate_state.hash()),
		int(last_action.hash()),
	]

func set_waiting_for_input(value: bool) -> void:
	if battle_state == null:
		return
	if battle_state.has_method("set_waiting_for_input"):
		battle_state.set_waiting_for_input(value)
	sync_from_battle_state()

func set_fast_animation_request(value: bool) -> void:
	if battle_state == null:
		return
	if battle_state.has_method("set_fast_animation_request"):
		battle_state.set_fast_animation_request(value)
	sync_from_battle_state()

func set_fast_text_request(value: bool) -> void:
	if battle_state == null:
		return
	if battle_state.has_method("set_fast_text_request"):
		battle_state.set_fast_text_request(value)
	sync_from_battle_state()

func push_submenu(name: String) -> void:
	var normalized := name.strip_edges()
	if normalized.is_empty():
		return
	submenu_stack.append(normalized)
	submenu_index = 0
	last_action = "submenu:%s" % normalized
	sync_from_battle_state()

func pop_submenu() -> String:
	if submenu_stack.is_empty():
		return ""
	var popped: String = str(submenu_stack.pop_back())
	submenu_index = 0
	last_action = "submenu-pop:%s" % popped
	sync_from_battle_state()
	return popped

func current_submenu() -> String:
	if submenu_stack.is_empty():
		return ""
	return submenu_stack[submenu_stack.size() - 1]

func set_submenu_index(index: int) -> void:
	submenu_index = _clamp_menu_index(index)
	last_action = "submenu-index:%d" % submenu_index
	sync_from_battle_state()

func open_command_menu(entries: Array) -> void:
	command_menu_entries = _sanitize_command_entries(entries)
	submenu_stack = ["command"]
	submenu_index = 0
	latched_command = {}
	last_action = "command-menu:open"
	if battle_state != null and battle_state.has_method("set_prompt_gate"):
		battle_state.set_prompt_gate(true, "command_menu", "awaiting player command", true, "command menu open")
	sync_from_battle_state()

func close_command_menu() -> void:
	command_menu_entries = []
	latched_command = {}
	if current_submenu() == "command":
		pop_submenu()
	else:
		sync_from_battle_state()

func has_command_menu() -> bool:
	return not command_menu_entries.is_empty()

func get_command_entries() -> Array:
	var result: Array = []
	for entry in command_menu_entries:
		result.append(entry.duplicate(true))
	return result

func get_latched_command() -> Dictionary:
	return latched_command.duplicate(true)

func clear_latched_command() -> void:
	latched_command = {}
	last_action = "command:clear"
	sync_from_battle_state()

func latch_selected_command() -> Dictionary:
	if command_menu_entries.is_empty():
		return {}
	var index := _clamp_menu_index(submenu_index)
	var command := command_menu_entries[index].duplicate(true)
	latched_command = command.duplicate(true)
	last_action = "command:%s" % _command_label(command)
	if battle_state != null and battle_state.has_method("enqueue_command"):
		battle_state.enqueue_command(command)
	sync_from_battle_state()
	return command

func enqueue_dialogue_text(text: String, control: String = "") -> bool:
	if dialogue == null:
		return false
	var queued := bool(dialogue.enqueue_text(text, control))
	if queued:
		last_action = "dialogue:enqueue"
		if battle_state != null and battle_state.has_method("set_prompt_gate"):
			battle_state.set_prompt_gate(true, "dialogue", dialogue.current_text, true, "battle dialogue")
		sync_from_battle_state()
	return queued

func close_dialogue() -> void:
	if dialogue != null:
		dialogue.close_text_box()
	last_action = "dialogue:close"
	if battle_state != null and battle_state.has_method("clear_prompt_gate"):
		battle_state.clear_prompt_gate()
	sync_from_battle_state()

func get_dialogue_state() -> Dictionary:
	if dialogue == null:
		return {}
	return dialogue.get_state()

func get_resolution_events() -> Array:
	if battle_state == null:
		return []
	if battle_state.has_method("get_resolution_events"):
		return battle_state.get_resolution_events()
	if battle_state.has_method("consume_resolution_events"):
		return battle_state.consume_resolution_events()
	return []

func drain_resolution_events() -> Array:
	if battle_state == null:
		return []
	var events: Array = []
	if battle_state.has_method("consume_resolution_events"):
		events = battle_state.consume_resolution_events()
	elif battle_state.has_method("get_resolution_events"):
		events = battle_state.get_resolution_events()
	last_resolution_drain = events.duplicate(true)
	last_action = "resolution-drain:%d" % events.size()
	sync_from_battle_state()
	return events

func has_resolution_events() -> bool:
	if battle_state == null:
		return false
	if battle_state.has_method("has_resolution_events"):
		return battle_state.has_resolution_events()
	return not get_resolution_events().is_empty()

func queue_resolution_event(event: Dictionary) -> void:
	if battle_state == null:
		return
	if battle_state.has_method("queue_resolution_event"):
		battle_state.queue_resolution_event(event)
	sync_from_battle_state()

func handle_action(action: String) -> bool:
	var normalized := action.strip_edges().to_lower()
	if dialogue != null and dialogue.is_visible():
		if dialogue.consume_input(normalized):
			last_action = "dialogue:%s" % normalized
			if dialogue.is_visible() and battle_state != null and battle_state.has_method("set_prompt_gate"):
				battle_state.set_prompt_gate(true, "dialogue", dialogue.current_text, true, "battle dialogue")
			elif battle_state != null and battle_state.has_method("clear_prompt_gate"):
				battle_state.clear_prompt_gate()
			sync_from_battle_state()
			return true
	if normalized in ["up", "left"]:
		set_submenu_index(submenu_index - 1)
		return true
	if normalized in ["down", "right"]:
		set_submenu_index(submenu_index + 1)
		return true
	if normalized in ["back", "cancel", "b"]:
		if not submenu_stack.is_empty():
			pop_submenu()
			return true
		return false
	if normalized in ["confirm", "advance", "a", "enter"]:
		if has_command_menu():
			latch_selected_command()
			return true
		last_action = "confirm"
		sync_from_battle_state()
		return true
	return false

func get_state() -> Dictionary:
	return to_dictionary()

func to_dictionary() -> Dictionary:
	return {
		"ui_phase": ui_phase,
		"submenu_stack": _duplicate_string_array(submenu_stack),
		"submenu_index": submenu_index,
		"command_menu_entries": _duplicate_dictionary_array(command_menu_entries),
		"latched_command": latched_command.duplicate(true),
		"prompt_gate_state": prompt_gate_state.duplicate(true),
		"last_action": last_action,
		"presentation_signature": presentation_signature,
		"last_resolution_drain": _duplicate_array(last_resolution_drain),
		"dialogue": get_dialogue_state(),
	}

func from_dictionary(data: Variant) -> bool:
	if typeof(data) != TYPE_DICTIONARY:
		return false
	from_state(Dictionary(data))
	return true

func from_state(data: Dictionary) -> void:
	if data.is_empty():
		reset()
		return
	submenu_stack = _sanitize_string_array(data.get("submenu_stack", []))
	submenu_index = max(0, int(data.get("submenu_index", 0)))
	command_menu_entries = _sanitize_command_entries(data.get("command_menu_entries", []))
	latched_command = _sanitize_dictionary(data.get("latched_command", {}))
	prompt_gate_state = _sanitize_dictionary(data.get("prompt_gate_state", {}))
	last_action = str(data.get("last_action", ""))
	presentation_signature = str(data.get("presentation_signature", ""))
	last_resolution_drain = _sanitize_array(data.get("last_resolution_drain", []))
	if dialogue != null:
		dialogue.from_dictionary(data.get("dialogue", {}))
	ui_phase = str(data.get("ui_phase", ui_phase))
	submenu_index = _clamp_menu_index(submenu_index)
	sync_from_battle_state()

func _read_prompt_gate_state() -> Dictionary:
	if battle_state == null:
		return {}
	return {
		"active": bool(battle_state.prompt_gate_active),
		"kind": str(battle_state.prompt_kind),
		"message": str(battle_state.prompt_message),
		"locked": bool(battle_state.prompt_locked),
		"reason": str(battle_state.prompt_gate_reason),
		"waiting_for_input": bool(battle_state.waiting_for_input),
		"dialogue_wait_gate_active": bool(battle_state.dialogue_wait_gate_active),
	}

func _clamp_menu_index(index: int) -> int:
	var upper_bound := command_menu_entries.size() - 1
	if upper_bound < 0:
		return max(0, index)
	return clampi(index, 0, upper_bound)

func _sanitize_command_entries(value: Variant) -> Array[Dictionary]:
	var result: Array[Dictionary] = []
	if typeof(value) != TYPE_ARRAY:
		return result
	for entry in Array(value):
		var command: Dictionary = {}
		if typeof(entry) == TYPE_DICTIONARY:
			command = Dictionary(entry).duplicate(true)
		else:
			command = {"kind": str(entry), "label": str(entry)}
		if command.is_empty():
			continue
		if not command.has("label"):
			command["label"] = _command_label(command)
		result.append(command)
	return result

func _command_label(command: Dictionary) -> String:
	if command.has("label"):
		return str(command.get("label", "command"))
	if command.has("kind"):
		return str(command.get("kind", "command"))
	if command.has("id"):
		return str(command.get("id", "command"))
	return "command"

func _sanitize_dictionary(value: Variant) -> Dictionary:
	if typeof(value) != TYPE_DICTIONARY:
		return {}
	return Dictionary(value).duplicate(true)

func _sanitize_array(value: Variant) -> Array:
	if typeof(value) != TYPE_ARRAY:
		return []
	return Array(value).duplicate(true)

func _sanitize_string_array(value: Variant) -> Array[String]:
	var result: Array[String] = []
	if typeof(value) != TYPE_ARRAY:
		return result
	for entry in Array(value):
		result.append(str(entry))
	return result

func _duplicate_string_array(value: Array[String]) -> Array:
	var result: Array = []
	for entry in value:
		result.append(str(entry))
	return result

func _duplicate_dictionary_array(value: Array[Dictionary]) -> Array:
	var result: Array = []
	for entry in value:
		result.append(entry.duplicate(true))
	return result

func _duplicate_array(value: Array) -> Array:
	return value.duplicate(true)
