extends RefCounted
class_name GameState

const DEFAULT_ACTIVE_SCENE := "intro"
const DEFAULT_LAST_SAVE_SLOT := "debug-shell"

var sram: Dictionary = {}
var wram: Dictionary = {}
var vram: Dictionary = {}
var hram: Dictionary = {}
var overworld: Dictionary = {}
var battle: Dictionary = {}
var ui: Dictionary = {}
var gameplay: Dictionary = {}
var scene_route: String = DEFAULT_ACTIVE_SCENE
var scene_context: Dictionary = {}
var scene_handoff: Dictionary = {}
var pending_scene_handoff: Dictionary = {}
var ui_page: String = "title"
var ui_dialogue_state: Dictionary = {}
var ui_menu_state: Dictionary = {}
var ui_shell_state: Dictionary = {}
var overworld_state: Dictionary = {}
var battle_state: Dictionary = {}
var frame_counter: int = 0
var has_seen_intro: bool = false
var active_scene: String = "intro"
var scene_history: Array[String] = []
var last_save_slot: String = "debug-shell"
var loaded_asset_summary: Dictionary = {}
var save_metadata: Dictionary = {}

func reset() -> void:
	active_scene = DEFAULT_ACTIVE_SCENE
	last_save_slot = DEFAULT_LAST_SAVE_SLOT
	frame_counter = 0
	has_seen_intro = false
	scene_history = []
	scene_history.append(active_scene)
	sram = _default_sram()
	wram = _default_wram(active_scene)
	vram = _default_vram()
	hram = _default_hram()
	overworld = _default_overworld(active_scene)
	battle = _default_battle()
	ui = _default_ui()
	gameplay = _compose_gameplay(active_scene, overworld, battle, ui)
	scene_route = active_scene
	scene_context = {}
	scene_handoff = {}
	pending_scene_handoff = {}
	ui_page = "title"
	ui_dialogue_state = _default_ui_dialogue_state()
	ui_menu_state = _default_ui_menu_state()
	ui_shell_state = {
		"ui_page": ui_page,
		"text_box": ui_dialogue_state.duplicate(true),
		"menu_stack": ui_menu_state.duplicate(true),
		"page_snapshots": {},
	}
	overworld_state = overworld.duplicate(true)
	battle_state = battle.duplicate(true)
	loaded_asset_summary = {}
	save_metadata = _default_save_metadata()

func _default_sram() -> Dictionary:
	return {
		"options": {
			"text_speed": "fast",
			"battle_scene": true,
			"battle_style": "shift",
			"sound": "stereo",
			"menu_account": true,
			"frame": 1,
		},
		"party": {"pokemon": [null, null, null, null, null, null]},
		"link_battle_stats": {"wins": 0, "losses": 0, "draws": 0},
		"badges": {
			"johto": [false, false, false, false, false, false, false, false],
			"kanto": [false, false, false, false, false, false, false, false],
		},
	}

func _default_wram(scene_name: String) -> Dictionary:
	return {
		"scene": scene_name,
		"scene_route": scene_name,
		"scene_transition": {},
		"flags": {},
		"variables": {},
	}

func _default_vram() -> Dictionary:
	return {
		"palette_bank": 0,
		"tile_cache_ready": false,
	}

func _default_hram() -> Dictionary:
	return {
		"joypad": {
			"hJoypadReleased": 0,
			"hJoypadPressed": 0,
			"hJoypadDown": 0,
			"hJoypadSum": 0,
			"hJoyReleased": 0,
			"hJoyPressed": 0,
			"hJoyDown": 0,
			"hJoyLast": 0,
		},
		"hardware_divider": 0,
		"hRandomAdd": 0,
		"hRandomSub": 0,
	}

func _default_save_metadata() -> Dictionary:
	return {
		"schema_version": 0,
		"slot": "",
		"saved_at": "",
		"kind": "",
		"frame_counter": 0,
	}

func _default_overworld(scene_name: String) -> Dictionary:
	return {
		"location": {
			"scene": scene_name,
			"map_id": "",
			"warp_id": "",
			"x": 0,
			"y": 0,
		},
		"player": {
			"x": 0,
			"y": 0,
			"facing": "down",
			"moving": false,
			"surfing": false,
			"biking": false,
		},
		"encounter": {
			"kind": "none",
			"pending": false,
		},
		"interaction": {
			"target": "",
			"script": "",
			"menu": "",
		},
	}

func _default_battle() -> Dictionary:
	return {
		"active": false,
		"kind": "none",
		"phase": "idle",
		"turn": 0,
		"last_command": "",
		"result": "none",
		"player": {
			"name": "",
			"hp": 0,
			"max_hp": 0,
			"status": "",
			"fainted": false,
		},
		"opponent": {
			"name": "",
			"hp": 0,
			"max_hp": 0,
			"status": "",
			"fainted": false,
		},
	}

func _default_ui() -> Dictionary:
	return {
		"open": false,
		"screen": "none",
		"cursor": {
			"index": 0,
			"row": 0,
			"column": 0,
		},
		"stack": [],
		"dialogue": {
			"open": false,
			"page": 0,
			"speaker": "",
			"prompt": "",
		},
	}

func _default_ui_dialogue_state() -> Dictionary:
	return {
		"active": false,
		"visible": false,
		"page_index": 0,
		"page_count": 0,
		"text": "",
		"current_text": "",
		"visible_text": "",
	}

func _default_ui_menu_state() -> Dictionary:
	return {
		"menu_open": false,
		"input_locked": false,
		"depth": 0,
		"stack": [],
	}

func _default_gameplay(scene_name: String) -> Dictionary:
	var overworld_defaults := _default_overworld(scene_name)
	var battle_defaults := _default_battle()
	var ui_defaults := _default_ui()
	return {
		"mode": scene_name,
		"overworld": overworld_defaults,
		"battle": battle_defaults,
		"ui": ui_defaults,
		"menu": ui_defaults.duplicate(true),
		"progress": {
			"story_flags": {},
			"event_flags": {},
			"badges": {
				"johto": [false, false, false, false, false, false, false, false],
				"kanto": [false, false, false, false, false, false, false, false],
			},
			"money": 0,
			"play_time_frames": 0,
		},
	}

func _compose_gameplay(scene_name: String, overworld_state: Dictionary, battle_state: Dictionary, ui_state: Dictionary) -> Dictionary:
	var defaults := _default_gameplay(scene_name)
	defaults["mode"] = scene_name
	defaults["overworld"] = _sanitize_dictionary(overworld_state, _default_overworld(scene_name))
	defaults["battle"] = _sanitize_dictionary(battle_state, _default_battle())
	defaults["ui"] = _sanitize_dictionary(ui_state, _default_ui())
	defaults["menu"] = defaults["ui"].duplicate(true)
	return defaults

func get_state() -> Dictionary:
	var scene_name := _coerce_string(active_scene, DEFAULT_ACTIVE_SCENE)
	var sanitized_overworld := _sanitize_dictionary(overworld, _default_overworld(scene_name))
	var overworld_location: Dictionary = Dictionary(sanitized_overworld.get("location", _default_overworld(scene_name).get("location", {})))
	overworld_location["scene"] = scene_name
	sanitized_overworld["location"] = overworld_location
	var sanitized_battle := _sanitize_dictionary(battle, _default_battle())
	var sanitized_ui := _sanitize_dictionary(ui, _default_ui())
	var sanitized_gameplay := _sanitize_dictionary(gameplay, _default_gameplay(scene_name))
	sanitized_gameplay["mode"] = scene_name
	sanitized_gameplay["overworld"] = sanitized_overworld.duplicate(true)
	sanitized_gameplay["battle"] = sanitized_battle.duplicate(true)
	sanitized_gameplay["ui"] = sanitized_ui.duplicate(true)
	sanitized_gameplay["menu"] = sanitized_ui.duplicate(true)
	var sanitized_wram := _sanitize_dictionary(wram, _default_wram(scene_name))
	sanitized_wram["scene"] = scene_name
	sanitized_wram["scene_route"] = scene_name
	var sanitized_ui_shell := _sanitize_dictionary(ui_shell_state, {
		"ui_page": "title",
		"text_box": _default_ui_dialogue_state(),
		"menu_stack": _default_ui_menu_state(),
		"page_snapshots": {},
	})
	sanitized_ui_shell = _normalize_ui_shell_state(sanitized_ui_shell, ui_page)
	var sanitized_ui_menu_state := _normalize_ui_menu_state(ui_menu_state, _default_ui_menu_state())
	return Dictionary(_normalize_variant({
		"sram": _sanitize_dictionary(sram, _default_sram()),
		"wram": sanitized_wram,
		"vram": _sanitize_dictionary(vram, _default_vram()),
		"hram": _sanitize_dictionary(hram, _default_hram()),
		"overworld": sanitized_overworld,
		"battle": sanitized_battle,
		"ui": sanitized_ui,
		"frame_counter": max(0, _coerce_int(frame_counter, 0)),
		"has_seen_intro": _coerce_bool(has_seen_intro, false),
		"active_scene": scene_name,
		"scene_route": _coerce_string(scene_route, scene_name),
		"scene_context": _sanitize_dictionary(scene_context, {}),
		"scene_handoff": _sanitize_dictionary(scene_handoff, {}),
		"pending_scene_handoff": _sanitize_dictionary(pending_scene_handoff, {}),
		"scene_history": _sanitize_array(scene_history, [scene_name]),
		"last_save_slot": _coerce_string(last_save_slot, DEFAULT_LAST_SAVE_SLOT),
		"ui_page": _coerce_string(ui_page, "title"),
		"ui_dialogue_state": _sanitize_dictionary(ui_dialogue_state, _default_ui_dialogue_state()),
		"ui_menu_state": sanitized_ui_menu_state,
		"ui_shell_state": sanitized_ui_shell,
		"overworld_state": _sanitize_dictionary(overworld_state, _default_overworld(scene_name)),
		"battle_state": _sanitize_dictionary(battle_state, _default_battle()),
		"gameplay": sanitized_gameplay,
		"loaded_asset_summary": _sanitize_dictionary(loaded_asset_summary, {}),
		"save_metadata": _sanitize_dictionary(save_metadata, _default_save_metadata()),
	}))

func from_state(data: Dictionary) -> void:
	from_dictionary(data)

func to_dictionary() -> Dictionary:
	return get_state()

func get_save_metadata() -> Dictionary:
	return _sanitize_dictionary(save_metadata, _default_save_metadata())

func set_save_metadata(metadata: Variant) -> bool:
	if typeof(metadata) != TYPE_DICTIONARY:
		return false
	save_metadata = _sanitize_dictionary(metadata, _default_save_metadata())
	return true

func get_scene_route() -> String:
	return _coerce_string(scene_route, active_scene)

func get_scene_context() -> Dictionary:
	return _sanitize_dictionary(scene_context, {})

func get_scene_handoff() -> Dictionary:
	return _sanitize_dictionary(scene_handoff, {})

func get_pending_scene_handoff() -> Dictionary:
	return _sanitize_dictionary(pending_scene_handoff, {})

func get_loaded_asset_summary() -> Dictionary:
	return _sanitize_dictionary(loaded_asset_summary, {})

func get_ui_page() -> String:
	return _coerce_string(ui_page, "title")

func get_ui_dialogue_state() -> Dictionary:
	return _sanitize_dictionary(ui_dialogue_state, _default_ui_dialogue_state())

func get_ui_menu_state() -> Dictionary:
	return _normalize_ui_menu_state(_sanitize_dictionary(ui_menu_state, _default_ui_menu_state()), _default_ui_menu_state())

func get_ui_shell_state() -> Dictionary:
	return _normalize_ui_shell_state(_sanitize_dictionary(ui_shell_state, {
		"ui_page": get_ui_page(),
		"text_box": _default_ui_dialogue_state(),
		"menu_stack": _default_ui_menu_state(),
		"page_snapshots": {},
	}), get_ui_page())

func get_overworld_state() -> Dictionary:
	return _sanitize_dictionary(overworld_state, _default_overworld(_coerce_string(active_scene, DEFAULT_ACTIVE_SCENE)))

func get_battle_state() -> Dictionary:
	return _sanitize_dictionary(battle_state, _default_battle())

func from_dictionary(data: Variant) -> bool:
	reset()
	if typeof(data) != TYPE_DICTIONARY:
		return false

	var source: Dictionary = data
	var gameplay_source: Dictionary = {}
	if typeof(source.get("gameplay", {})) == TYPE_DICTIONARY:
		gameplay_source = Dictionary(source.get("gameplay", {}))
	var legacy_gameplay: Dictionary = _sanitize_dictionary(gameplay_source, _default_gameplay(DEFAULT_ACTIVE_SCENE))
	var gameplay_scene := _coerce_string(legacy_gameplay.get("mode", DEFAULT_ACTIVE_SCENE), DEFAULT_ACTIVE_SCENE)
	active_scene = _coerce_string(source.get("active_scene", gameplay_scene), gameplay_scene)
	last_save_slot = _coerce_string(source.get("last_save_slot", DEFAULT_LAST_SAVE_SLOT), DEFAULT_LAST_SAVE_SLOT)
	frame_counter = max(0, _coerce_int(source.get("frame_counter", 0), 0))
	has_seen_intro = _coerce_bool(source.get("has_seen_intro", false), false)
	sram = _sanitize_dictionary(source.get("sram", {}), _default_sram())
	wram = _sanitize_dictionary(source.get("wram", {}), _default_wram(active_scene))
	wram["scene"] = active_scene
	wram["scene_route"] = _coerce_string(wram.get("scene_route", active_scene), active_scene)
	vram = _sanitize_dictionary(source.get("vram", {}), _default_vram())
	hram = _sanitize_dictionary(source.get("hram", {}), _default_hram())
	scene_route = _coerce_string(source.get("scene_route", active_scene), active_scene)
	scene_context = _sanitize_dictionary(source.get("scene_context", {}), {})
	scene_handoff = _sanitize_dictionary(source.get("scene_handoff", {}), {})
	pending_scene_handoff = _sanitize_dictionary(source.get("pending_scene_handoff", {}), {})
	scene_history = []
	for entry in _sanitize_array(source.get("scene_history", [active_scene]), [active_scene]):
		scene_history.append(_coerce_string(entry, active_scene))
	if scene_history.is_empty():
		scene_history = [active_scene]

	var overworld_defaults := _default_overworld(active_scene)
	var battle_defaults := _default_battle()
	var ui_defaults := _default_ui()
	var overworld_source: Variant = source.get("overworld", gameplay_source.get("overworld", overworld_defaults))
	var battle_source: Variant = source.get("battle", gameplay_source.get("battle", battle_defaults))
	var ui_source: Variant = source.get("ui", gameplay_source.get("ui", gameplay_source.get("menu", ui_defaults)))
	overworld = _sanitize_dictionary(overworld_source, overworld_defaults)
	battle = _sanitize_dictionary(battle_source, battle_defaults)
	ui = _sanitize_dictionary(ui_source, ui_defaults)
	_ensure_overworld_scene(overworld, active_scene)
	gameplay = _sanitize_dictionary(gameplay_source, _default_gameplay(active_scene))
	gameplay["mode"] = active_scene
	gameplay["overworld"] = overworld.duplicate(true)
	gameplay["battle"] = battle.duplicate(true)
	gameplay["ui"] = ui.duplicate(true)
	gameplay["menu"] = ui.duplicate(true)
	ui_page = _coerce_string(source.get("ui_page", ui_page), ui_page)
	ui_dialogue_state = _sanitize_dictionary(source.get("ui_dialogue_state", ui.get("dialogue", {})), _default_ui_dialogue_state())
	ui_menu_state = _normalize_ui_menu_state(_sanitize_dictionary(source.get("ui_menu_state", ui.get("menu_stack", ui.get("stack", {}))), _default_ui_menu_state()), _default_ui_menu_state())
	ui_shell_state = _normalize_ui_shell_state(_sanitize_dictionary(source.get("ui_shell_state", {
		"ui_page": ui_page,
		"text_box": ui_dialogue_state.duplicate(true),
		"menu_stack": ui_menu_state.duplicate(true),
		"page_snapshots": {},
	}), {
		"ui_page": "title",
		"text_box": _default_ui_dialogue_state(),
		"menu_stack": _default_ui_menu_state(),
		"page_snapshots": {},
	}), ui_page)
	if ui_dialogue_state.is_empty() and ui_shell_state.has("text_box"):
		ui_dialogue_state = _sanitize_dictionary(ui_shell_state.get("text_box", {}), _default_ui_dialogue_state())
	if ui_menu_state.is_empty() and ui_shell_state.has("menu_stack"):
		ui_menu_state = _normalize_ui_menu_state(_sanitize_dictionary(ui_shell_state.get("menu_stack", {}), _default_ui_menu_state()), _default_ui_menu_state())
	ui_menu_state = _normalize_ui_menu_state(ui_menu_state, _default_ui_menu_state())
	overworld_state = _sanitize_dictionary(source.get("overworld_state", overworld), _default_overworld(active_scene))
	battle_state = _sanitize_dictionary(source.get("battle_state", battle), _default_battle())
	loaded_asset_summary = _sanitize_dictionary(source.get("loaded_asset_summary", {}), {})
	save_metadata = _sanitize_dictionary(source.get("save_metadata", {}), _default_save_metadata())
	return true

func _ensure_overworld_scene(value: Dictionary, scene_name: String) -> void:
	if not value.has("location") or typeof(value.get("location")) != TYPE_DICTIONARY:
		value["location"] = _default_overworld(scene_name).get("location", {})
	var location: Dictionary = Dictionary(value.get("location", {}))
	location["scene"] = scene_name
	value["location"] = location

func _normalize_ui_shell_state(shell_state: Variant, fallback_ui_page: String) -> Dictionary:
	var normalized := _sanitize_dictionary(shell_state, {
		"ui_page": fallback_ui_page,
		"text_box": _default_ui_dialogue_state(),
		"menu_stack": _default_ui_menu_state(),
		"page_snapshots": {},
	})
	normalized["ui_page"] = _coerce_string(normalized.get("ui_page", fallback_ui_page), fallback_ui_page)
	if typeof(normalized.get("text_box", {})) == TYPE_DICTIONARY:
		normalized["text_box"] = _sanitize_dictionary(normalized.get("text_box", {}), _default_ui_dialogue_state())
	else:
		normalized["text_box"] = _default_ui_dialogue_state()
	if typeof(normalized.get("menu_stack", {})) == TYPE_DICTIONARY:
		normalized["menu_stack"] = _normalize_ui_menu_stack(Dictionary(normalized.get("menu_stack", {})))
	else:
		normalized["menu_stack"] = _default_ui_menu_state()
	normalized["page_snapshots"] = _normalize_variant(normalized.get("page_snapshots", {}))
	return normalized

func _normalize_ui_menu_state(menu_state: Variant, defaults: Dictionary) -> Dictionary:
	var normalized := _sanitize_dictionary(menu_state, defaults)
	if typeof(normalized.get("stack", [])) == TYPE_ARRAY:
		var normalized_stack: Array = []
		for entry in Array(normalized.get("stack", [])):
			if typeof(entry) != TYPE_DICTIONARY:
				continue
			var item := Dictionary(entry).duplicate(true)
			if item.has("index"):
				item["index"] = _coerce_int(item.get("index", 0), 0)
			if item.has("row"):
				item["row"] = _coerce_int(item.get("row", 0), 0)
			if item.has("column"):
				item["column"] = _coerce_int(item.get("column", 0), 0)
			normalized_stack.append(item)
		normalized["stack"] = normalized_stack
	else:
		normalized["stack"] = []
	return normalized

func _normalize_ui_menu_stack(menu_stack: Dictionary) -> Dictionary:
	var normalized := _sanitize_dictionary(menu_stack, _default_ui_menu_state())
	var stack_value: Variant = normalized.get("stack", [])
	if typeof(stack_value) != TYPE_ARRAY:
		normalized["stack"] = []
		return normalized
	var normalized_stack: Array = []
	for entry in Array(stack_value):
		if typeof(entry) != TYPE_DICTIONARY:
			continue
		var item := Dictionary(entry).duplicate(true)
		if item.has("index"):
			item["index"] = _coerce_int(item.get("index", 0), 0)
		if item.has("row"):
			item["row"] = _coerce_int(item.get("row", 0), 0)
		if item.has("column"):
			item["column"] = _coerce_int(item.get("column", 0), 0)
		normalized_stack.append(item)
	normalized["stack"] = normalized_stack
	return normalized

func _sanitize_dictionary(value: Variant, defaults: Dictionary) -> Dictionary:
	var result: Dictionary = defaults.duplicate(true)
	if typeof(value) != TYPE_DICTIONARY:
		return result
	var source: Dictionary = value
	for key in source.keys():
		var raw_value: Variant = source[key]
		var normalized_key := _normalize_key(key)
		if result.has(normalized_key):
			result[normalized_key] = _sanitize_value(raw_value, result[normalized_key])
		else:
			result[normalized_key] = _normalize_variant(raw_value)
	return result

func _sanitize_array(value: Variant, defaults: Array) -> Array:
	var result: Array = defaults.duplicate(true)
	if typeof(value) != TYPE_ARRAY:
		return result
	var source: Array = value
	for index in range(source.size()):
		var template: Variant = null
		if index < result.size():
			template = result[index]
			result[index] = _sanitize_value(source[index], template)
		else:
			if not defaults.is_empty():
				template = defaults[defaults.size() - 1]
			result.append(_sanitize_value(source[index], template))
	return result

func _sanitize_value(value: Variant, template: Variant) -> Variant:
	match typeof(template):
		TYPE_DICTIONARY:
			return _sanitize_dictionary(value, template)
		TYPE_ARRAY:
			return _sanitize_array(value, template)
		TYPE_STRING:
			return _coerce_string(value, str(template))
		TYPE_INT:
			return _coerce_int(value, int(template))
		TYPE_FLOAT:
			return _coerce_float(value, float(template))
		TYPE_BOOL:
			return _coerce_bool(value, bool(template))
		TYPE_NIL:
			return _normalize_variant(value)
		_:
			if typeof(value) == typeof(template):
				return _normalize_variant(value)
			return template

func _normalize_variant(value: Variant) -> Variant:
	match typeof(value):
		TYPE_DICTIONARY:
			return _normalize_dictionary(value)
		TYPE_ARRAY:
			return _normalize_array(value)
		TYPE_STRING, TYPE_INT, TYPE_FLOAT, TYPE_BOOL, TYPE_NIL:
			return value
		_:
			return null

func _normalize_dictionary(value: Variant) -> Dictionary:
	var normalized: Dictionary = {}
	if typeof(value) != TYPE_DICTIONARY:
		return normalized
	var source: Dictionary = value
	for key in source.keys():
		normalized[_normalize_key(key)] = _normalize_variant(source[key])
	return normalized

func _normalize_array(value: Variant) -> Array:
	var normalized_array: Array = []
	if typeof(value) != TYPE_ARRAY:
		return normalized_array
	var source_array: Array = value
	for entry in source_array:
		normalized_array.append(_normalize_variant(entry))
	return normalized_array

func _normalize_key(value: Variant) -> String:
	return str(value)

func _coerce_string(value: Variant, fallback: String) -> String:
	if typeof(value) == TYPE_NIL:
		return fallback
	var result := str(value).strip_edges()
	if result.is_empty():
		return fallback
	return result

func _coerce_int(value: Variant, fallback: int) -> int:
	if typeof(value) == TYPE_NIL:
		return fallback
	return int(value)

func _coerce_float(value: Variant, fallback: float) -> float:
	if typeof(value) == TYPE_NIL:
		return fallback
	return float(value)

func _coerce_bool(value: Variant, fallback: bool) -> bool:
	match typeof(value):
		TYPE_NIL:
			return fallback
		TYPE_BOOL:
			return value
		TYPE_INT, TYPE_FLOAT:
			return value != 0
		TYPE_STRING:
			var normalized := str(value).strip_edges().to_lower()
			if normalized in ["true", "1", "yes", "on"]:
				return true
			if normalized in ["false", "0", "no", "off", ""]:
				return false
			return fallback
		_:
			return fallback
