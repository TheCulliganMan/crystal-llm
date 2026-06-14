extends RefCounted
class_name UIMenuState

signal menu_changed(active_menu: String, top_panel: Dictionary)
signal state_changed(state: Dictionary)

const UP_BUTTONS := ["up", "left"]
const DOWN_BUTTONS := ["down", "right"]
const CONFIRM_BUTTONS := ["a", "start"]
const CANCEL_BUTTONS := ["b"]

const MENU_TITLES := {
	"main_menu": "MAIN MENU",
	"start_menu": "START MENU",
	"bag_menu": "BAG",
	"party_menu": "PARTY",
	"pokemon_menu": "POKEMON MENU",
	"move_menu": "MOVE REORDER",
	"pokedex": "POKEDEX",
	"pc_menu": "PC",
	"pokegear": "POKEGEAR",
	"trainer_card": "TRAINER CARD",
	"options_menu": "OPTIONS",
	"continue": "CONTINUE",
	"delete_save": "DELETE SAVE",
	"clock_reset": "CLOCK RESET",
	"gender": "GENDER",
	"name_entry": "NAME ENTRY",
	"title": "TITLE",
}

const MENU_KINDS := {
	"continue": "prompt",
	"delete_save": "prompt",
	"clock_reset": "prompt",
	"gender": "prompt",
	"name_entry": "prompt",
}

const POKEDEX_SEARCH_TYPE_SEQUENCE := [
	"NONE",
	"NORMAL",
	"FIRE",
	"WATER",
	"GRASS",
	"ELECTRIC",
	"ICE",
	"FIGHTING",
	"POISON",
	"GROUND",
	"FLYING",
	"PSYCHIC_TYPE",
	"BUG",
	"ROCK",
	"GHOST",
	"DRAGON",
	"DARK",
	"STEEL",
]

const BAG_POCKET_SEQUENCE := ["ITEMS", "BALL", "KEY", "TM/HM"]
const BAG_LIST_VISIBLE_ROWS := 7
const BAG_ACTION_OPTIONS := {
	"ITEMS": ["USE", "GIVE", "TOSS", "QUIT"],
	"BALL": ["USE", "GIVE", "TOSS", "QUIT"],
	"KEY": ["USE", "SEL", "QUIT"],
	"TM/HM": ["USE", "QUIT"],
}

const PARTY_ACTION_OPTIONS := ["STATS", "SWITCH", "ITEM", "CANCEL"]
const OPTIONS_FIELD_ORDER := ["text_speed", "battle_scene", "battle_style", "sound", "menu_account", "frame", "print_option"]
const OPTIONS_FIELD_LABELS := {
	"text_speed": "TEXT SPEED",
	"battle_scene": "BATTLE SCENE",
	"battle_style": "BATTLE STYLE",
	"sound": "SOUND",
	"print_option": "PRINT",
	"menu_account": "MENU ACCOUNT",
	"frame": "FRAME",
}
const OPTIONS_FIELD_VALUES := {
	"text_speed": ["FAST", "MID", "SLOW"],
	"battle_scene": ["ON", "OFF"],
	"battle_style": ["SHIFT", "SET"],
	"sound": ["MONO", "STEREO"],
	"print_option": ["LIGHTEST", "LIGHTER", "NORMAL", "DARKER", "DARKEST"],
	"menu_account": ["ON", "OFF"],
	"frame": ["1", "2", "3", "4", "5", "6", "7", "8"],
}

const POKEGEAR_RADIO_FREQUENCIES := [
	{"raw": 16, "frequency": 4.5, "handler": "PKMNTalkAndPokedexShow", "station": "POKEDEX_SHOW"},
	{"raw": 28, "frequency": 7.5, "handler": "PokemonMusic", "station": "POKEMON_MUSIC"},
	{"raw": 32, "frequency": 8.5, "handler": "LuckyChannel", "station": "LUCKY_CHANNEL"},
	{"raw": 40, "frequency": 10.5, "handler": "BuenasPassword", "station": "BUENAS_PASSWORD"},
	{"raw": 52, "frequency": 13.5, "handler": "RuinsOfAlphRadio", "station": "UNOWN_RADIO"},
	{"raw": 64, "frequency": 16.5, "handler": "PlacesAndPeople", "station": "PLACES_AND_PEOPLE"},
	{"raw": 72, "frequency": 18.5, "handler": "LetsAllSing", "station": "LETS_ALL_SING"},
	{"raw": 78, "frequency": 20.0, "handler": "PokeFluteRadio", "station": "POKE_FLUTE_RADIO"},
	{"raw": 80, "frequency": 20.5, "handler": "EvolutionRadio", "station": "EVOLUTION_RADIO"},
]

const POKEGEAR_RADIO_STATION_NAMES := {
	"BUENAS_PASSWORD": "BUENA'S PASSWORD",
	"LETS_ALL_SING": "LET'S ALL SING!",
	"LUCKY_CHANNEL": "LUCKY CHANNEL",
	"OAKS_POKEMON_TALK": "OAK's POKé TALK",
	"PLACES_AND_PEOPLE": "PLACES & PEOPLE",
	"POKE_FLUTE_RADIO": "POKé FLUTE",
	"POKEDEX_SHOW": "POKéDEX SHOW",
	"POKEMON_MUSIC": "POKéMON MUSIC",
	"UNOWN_RADIO": "?????",
	"ROCKET_RADIO": "ROCKET RADIO",
}

const DEFAULT_MENU_ENTRIES := {
	"main_menu": ["CONTINUE", "NEW GAME", "OPTION", "MYSTERY GIFT"],
	"start_menu": ["POKEDEX", "POKEMON", "PACK", "PLAYER", "SAVE", "OPTION", "EXIT", "POKEGEAR", "QUIT"],
	"bag_menu": ["ITEMS", "BALL", "KEY", "TM/HM"],
	"party_menu": ["STATS", "SWITCH", "ITEM", "CANCEL"],
	"pokemon_menu": ["STATS", "SWITCH", "ITEM", "CANCEL"],
	"move_menu": ["MOVE 1", "MOVE 2", "MOVE 3", "MOVE 4", "CANCEL"],
	"pokedex": ["LIST", "SEARCH", "OPTIONS", "UNOWN"],
	"pc_menu": ["WITHDRAW", "DEPOSIT", "CHANGE BOX", "MOVE W/O MAIL", "SEE YA"],
	"pokegear": ["CLOCK", "MAP", "PHONE", "RADIO"],
	"trainer_card": ["INFO", "JOHTO BADGES", "KANTO BADGES"],
	"options_menu": ["TEXT SPEED", "BATTLE SCENE", "BATTLE STYLE", "SOUND", "PRINT", "MENU ACCOUNT", "FRAME", "CANCEL"],
	"continue": ["YES", "NO"],
	"delete_save": ["YES", "NO"],
	"clock_reset": ["YES", "NO"],
	"gender": ["BOY", "GIRL"],
	"name_entry": ["OK", "CANCEL"],
}

var _active_menu: String = ""
var _menu_states: Dictionary = {}
var _input_locked: bool = false
var _runtime_state: Dictionary = {}

func reset() -> void:
	_active_menu = ""
	_menu_states = {}
	_input_locked = false
	_runtime_state = {}

func sync_runtime_state(data: Variant) -> void:
	if typeof(data) != TYPE_DICTIONARY:
		return
	var snapshot := _sanitize_runtime_state(Dictionary(data))
	if snapshot.is_empty():
		return
	_runtime_state = snapshot
	_refresh_dynamic_menu_states()

func to_dictionary() -> Dictionary:
	return get_state()

func from_dictionary(data: Variant) -> bool:
	if typeof(data) != TYPE_DICTIONARY:
		return false
	from_state(Dictionary(data))
	return true

func is_active() -> bool:
	return not _active_menu.is_empty()

func is_menu_open() -> bool:
	return is_active()

func is_input_locked() -> bool:
	return _input_locked

func is_input_owned() -> bool:
	return is_active()

func can_accept_input() -> bool:
	if _active_menu.is_empty():
		return false
	var state := _ensure_menu_state(_active_menu)
	return not _input_locked and not bool(state.get("locked", false))

func has_panels() -> bool:
	return is_active()

func should_block_gameplay_input() -> bool:
	return is_active()

func get_active_menu() -> String:
	return _active_menu

func set_input_locked(is_locked: bool) -> void:
	_input_locked = is_locked
	state_changed.emit(get_state())

func activate_menu(menu_name: String) -> Dictionary:
	var normalized := _normalize_menu_name(menu_name)
	_active_menu = normalized
	var state := _ensure_menu_state(normalized)
	var top := _panel_snapshot_from_entry(state, _clamp_cursor(state, int(state.get("cursor", 0))), 1)
	menu_changed.emit(_active_menu, top)
	state_changed.emit(get_state())
	return top

func deactivate_menu() -> void:
	_active_menu = ""
	state_changed.emit(get_state())
	menu_changed.emit("", {})

func set_menu_state(menu_name: String, data: Variant) -> Dictionary:
	var normalized := _normalize_menu_name(menu_name)
	var normalized_state := _normalize_state(data)
	_menu_states[normalized] = normalized_state
	if normalized == _active_menu:
		var top := get_top_panel()
		menu_changed.emit(_active_menu, top)
		state_changed.emit(get_state())
		return top
	return _panel_snapshot_from_entry(normalized_state, _clamp_cursor(normalized_state, int(normalized_state.get("cursor", 0))), 1)

func get_menu_state(menu_name: String) -> Dictionary:
	var normalized := _normalize_menu_name(menu_name)
	return Dictionary(_ensure_menu_state(normalized)).duplicate(true)

func set_menu_entries(
	menu_name: String,
	entries: Variant,
	cursor: int = 0,
	cancelable: bool = true,
	wrap: bool = true,
	locked: bool = false,
) -> Dictionary:
	var normalized := _normalize_menu_name(menu_name)
	var state := _ensure_menu_state(normalized)
	state["entries"] = _normalize_entries(entries)
	state["cursor"] = _clamp_panel_cursor(state, cursor)
	state["cancelable"] = cancelable
	state["wrap"] = wrap
	state["locked"] = locked
	_menu_states[normalized] = state
	if normalized == _active_menu:
		var top := get_top_panel()
		menu_changed.emit(_active_menu, top)
		state_changed.emit(get_state())
		return top
	return _panel_snapshot_from_entry(state, _clamp_cursor(state, int(state.get("cursor", 0))), 1)

func set_menu_cursor(menu_name: String, cursor: int) -> Dictionary:
	var normalized := _normalize_menu_name(menu_name)
	var state := _ensure_menu_state(normalized)
	state["cursor"] = _clamp_panel_cursor(state, cursor)
	_menu_states[normalized] = state
	if normalized == _active_menu:
		var top := get_top_panel()
		menu_changed.emit(_active_menu, top)
		state_changed.emit(get_state())
		return top
	return _panel_snapshot_from_entry(state, int(state.get("cursor", 0)), 1)

func move_menu_cursor(menu_name: String, step: int) -> Dictionary:
	var normalized := _normalize_menu_name(menu_name)
	var state := _ensure_menu_state(normalized)
	var entries: Array = Array(state.get("entries", []))
	if entries.is_empty():
		return get_menu_state(normalized)
	var cursor := _clamp_cursor(state, int(state.get("cursor", 0)))
	var next_cursor := _next_enabled_cursor(entries, cursor, step, bool(state.get("wrap", true)))
	state["cursor"] = next_cursor
	_menu_states[normalized] = state
	if normalized == _active_menu:
		var top := get_top_panel()
		menu_changed.emit(_active_menu, top)
		state_changed.emit(get_state())
		return top
	return _panel_snapshot_from_entry(state, next_cursor, 1)

func get_selected_index() -> int:
	return int(get_top_panel().get("cursor", 0))

func get_menu_cursor(menu_name: String) -> int:
	var normalized := _normalize_menu_name(menu_name)
	if normalized.is_empty() or not _menu_states.has(normalized):
		return 0
	return int(Dictionary(_menu_states[normalized]).get("cursor", 0))

func get_selected_entry() -> Dictionary:
	var top := get_top_panel()
	var selection: Variant = top.get("selection", {})
	if selection is Dictionary:
		return Dictionary(selection).duplicate(true)
	return {}

func get_selected_label() -> String:
	return str(get_selected_entry().get("label", ""))

func get_top_panel() -> Dictionary:
	if _active_menu.is_empty():
		return {}
	var state := _ensure_menu_state(_active_menu)
	var cursor := _clamp_cursor(state, int(state.get("cursor", 0)))
	var snapshot := _panel_snapshot_from_entry(state, cursor, 1)
	snapshot["source"] = "menu_state"
	snapshot["active_menu"] = _active_menu
	return snapshot

func get_state() -> Dictionary:
	return {
		"active": is_active(),
		"menu_open": is_menu_open(),
			"input_locked": _input_locked,
			"can_accept_input": can_accept_input(),
			"active_menu": _active_menu,
			"menu": _active_menu,
			"menu_name": _active_menu,
			"depth": 1 if is_active() else 0,
			"top_panel": get_top_panel(),
			"current_panel": get_top_panel(),
			"menus": _duplicate_dictionary(_menu_states),
			"menu_cursors": _menu_cursors(),
			"runtime_context": _duplicate_dictionary(_runtime_state),
		}

func from_state(data: Dictionary) -> void:
	if data.is_empty():
		reset()
		return
	_menu_states = {}
	_restore_menu_states_from_snapshot(data)
	_runtime_state = _sanitize_runtime_state(Dictionary(data.get("runtime_context", data.get("runtime_state", {}))))
	_active_menu = _active_menu_from_state(data)
	if _active_menu.is_empty() and not _menu_states.is_empty():
		_active_menu = _normalize_menu_name(str(_menu_states.keys()[0]))
	_input_locked = bool(data.get("input_locked", false))
	if _active_menu.is_empty() or not _menu_states.has(_active_menu):
		_active_menu = ""
		_input_locked = false
	_refresh_dynamic_menu_states()
	state_changed.emit(get_state())

func consume_input(frame_input: Dictionary) -> Dictionary:
	var result := {
		"consumed": false,
		"action": "",
		"top_panel": get_top_panel(),
		"selection": {},
		"depth": 1 if is_active() else 0,
		"active_menu": _active_menu,
		"input_locked": _input_locked,
		"can_accept_input": can_accept_input(),
	}
	if _active_menu.is_empty():
		return result
	var state := _ensure_menu_state(_active_menu)
	if _active_menu == "bag_menu":
		var bag_result: Dictionary = _handle_bag_input(state, frame_input)
		if bool(bag_result.get("consumed", false)):
			return bag_result
	if _active_menu == "pc_menu":
		var pc_result := _handle_pc_menu_input(state, frame_input)
		if bool(pc_result.get("consumed", false)):
			return pc_result
	if _active_menu == "options_menu":
		var options_result := _handle_options_input(state, frame_input)
		if bool(options_result.get("consumed", false)):
			return options_result
	elif _active_menu == "pokedex":
		var pokedex_result := _handle_pokedex_input(state, frame_input)
		if bool(pokedex_result.get("consumed", false)):
			return pokedex_result
	elif _active_menu == "pokegear":
		var pokegear_result: Dictionary = _handle_pokegear_input(state, frame_input)
		if bool(pokegear_result.get("consumed", false)):
			return pokegear_result
	elif _active_menu == "trainer_card":
		var trainer_result := _handle_trainer_card_input(state, frame_input)
		if bool(trainer_result.get("consumed", false)):
			return trainer_result
	if _active_menu == "party_menu" or _active_menu == "pokemon_menu":
		var party_result := _handle_party_menu_input(state, frame_input, _active_menu)
		if bool(party_result.get("consumed", false)):
			return party_result
	if _input_locked or bool(state.get("locked", false)):
		var pressed_locked: Dictionary = Dictionary(frame_input.get("pressed", {}))
		result["consumed"] = _has_any_pressed_button(pressed_locked)
		return result
	var pressed: Dictionary = Dictionary(frame_input.get("pressed", {}))
	var handled := false
	if bool(pressed.get("up", false)):
		handled = _move_cursor(1, -1)
		if handled:
			result["action"] = "move_up"
	if not handled and _has_pressed_in_group(pressed, DOWN_BUTTONS):
		handled = _move_cursor(1, 1)
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
		result["top_panel"] = _cancel_active_menu()
		return result
	result["consumed"] = _has_any_pressed_button(pressed)
	result["top_panel"] = get_top_panel()
	if handled:
		result["consumed"] = true
		result["selection"] = _selection_for_top()
	return result

func _cancel_active_menu() -> Dictionary:
	if _active_menu.is_empty():
		return {}
	var state := _ensure_menu_state(_active_menu)
	if bool(state.get("cancelable", true)):
		deactivate_menu()
		return {}
	return get_top_panel()

func _move_cursor(_depth: int, step: int) -> bool:
	if _active_menu.is_empty():
		return false
	var state := _ensure_menu_state(_active_menu)
	var entries: Array = Array(state.get("entries", []))
	if entries.is_empty():
		return false
	var cursor := _clamp_cursor(state, int(state.get("cursor", 0)))
	var next_cursor := _next_enabled_cursor(entries, cursor, step, bool(state.get("wrap", true)))
	if next_cursor == cursor:
		return false
	state["cursor"] = next_cursor
	if _active_menu == "pc_menu":
		var controller_state := _pc_controller_state()
		controller_state["selected_index"] = next_cursor
		controller_state["selected_entry"] = _selection_for(state, next_cursor)
		if str(controller_state.get("mode", "hub")) == "box":
			controller_state["box_cursor"] = next_cursor
		else:
			controller_state["hub_cursor"] = next_cursor
		state["state"] = controller_state
	_menu_states[_active_menu] = state
	menu_changed.emit(_active_menu, get_top_panel())
	state_changed.emit(get_state())
	return true

func _selection_for_top() -> Dictionary:
	if _active_menu.is_empty():
		return {}
	var state := _ensure_menu_state(_active_menu)
	return _selection_for(state, _clamp_cursor(state, int(state.get("cursor", 0))))

func _selection_for(panel: Dictionary, cursor: int) -> Dictionary:
	var entries: Array = Array(panel.get("entries", []))
	if cursor < 0 or cursor >= entries.size():
		return {}
	var entry: Variant = entries[cursor]
	return _normalize_entry(entry)

func _ensure_menu_state(menu_name: String) -> Dictionary:
	var normalized := _normalize_menu_name(menu_name)
	if not _menu_states.has(normalized):
		_menu_states[normalized] = _build_menu_state(normalized, _default_menu_state(normalized))
	var state: Dictionary = Dictionary(_menu_states[normalized])
	state = _build_menu_state(normalized, state)
	state["cursor"] = _clamp_panel_cursor(state, int(state.get("cursor", 0)))
	_menu_states[normalized] = state
	return state

func _default_menu_state(menu_name: String) -> Dictionary:
	return {
		"id": menu_name,
		"title": _title_for_menu(menu_name),
		"kind": MENU_KINDS.get(menu_name, "menu"),
		"entries": _normalize_entries(DEFAULT_MENU_ENTRIES.get(menu_name, [])),
		"cursor": 0,
		"cancelable": true,
		"wrap": true,
		"locked": false,
		"state": {},
	}

func _normalize_state(value: Variant) -> Dictionary:
	var raw_id := ""
	if typeof(value) == TYPE_DICTIONARY:
		var source_id: Dictionary = Dictionary(value)
		raw_id = str(source_id.get("id", source_id.get("menu", "")))
	var result := _default_menu_state(_normalize_menu_name(raw_id))
	if typeof(value) != TYPE_DICTIONARY:
		return result
	var source: Dictionary = Dictionary(value)
	result["id"] = str(source.get("id", source.get("menu", result["id"])))
	result["title"] = str(source.get("title", source.get("label", result["title"])))
	result["kind"] = str(source.get("kind", result["kind"]))
	result["entries"] = _normalize_entries(source.get("entries", []))
	result["cursor"] = _clamp_panel_cursor(result, int(source.get("cursor", source.get("selected_index", 0))))
	result["cancelable"] = bool(source.get("cancelable", true))
	result["wrap"] = bool(source.get("wrap", true))
	result["locked"] = bool(source.get("locked", false))
	result["state"] = _normalize_payload(source.get("state", source.get("meta", {})))
	for key in source.keys():
		if not result.has(key):
			result[key] = _normalize_payload(source[key])
	return result

func _normalize_menu_name(menu_name: String) -> String:
	var normalized := menu_name.strip_edges().to_lower()
	match normalized:
		"", "none":
			return ""
		"menu":
			return "main_menu"
		"pack_menu":
			return "bag_menu"
		"title_screen":
			return "title"
		"continue_screen":
			return "continue"
		"delete_save_screen":
			return "delete_save"
		"clock_reset_screen":
			return "clock_reset"
		"gender_selection":
			return "gender"
		"name_entry_screen":
			return "name_entry"
		"pokedex_menu":
			return "pokedex"
		"pokegear_menu":
			return "pokegear"
		"trainer_card_screen":
			return "trainer_card"
		"day_of_week_screen":
			return "day_of_week"
		_:
			return normalized

func _title_for_menu(menu_name: String) -> String:
	if MENU_TITLES.has(menu_name):
		return str(MENU_TITLES[menu_name])
	var title := menu_name.replace("_", " ").strip_edges()
	return title.to_upper()

func _normalize_entries(value: Variant) -> Array:
	var result: Array = []
	if typeof(value) != TYPE_ARRAY:
		return result
	for entry in Array(value):
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
			var source: Dictionary = Dictionary(entry)
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
			var source: Dictionary = Dictionary(value)
			for key in source.keys():
				normalized[key] = _normalize_payload(source[key])
			return normalized
		TYPE_ARRAY:
			var normalized_array: Array = []
			for entry in Array(value):
				normalized_array.append(_normalize_payload(entry))
			return normalized_array
		TYPE_STRING, TYPE_INT, TYPE_FLOAT, TYPE_BOOL, TYPE_NIL:
			return value
		_:
			return null

func _sanitize_runtime_state(value: Dictionary) -> Dictionary:
	if value.is_empty():
		return {}
	var result := {
		"sram": {},
		"wram": {},
		"player_name": "",
		"player_gender": "",
		"ui_page": "",
		"save_metadata": {},
		"loaded_asset_summary": {},
		"map_summary": {},
		"pokemon_data": [],
	}
	result["sram"] = Dictionary(value.get("sram", {})).duplicate(true)
	result["wram"] = Dictionary(value.get("wram", {})).duplicate(true)
	result["player_name"] = str(value.get("player_name", ""))
	result["player_gender"] = str(value.get("player_gender", ""))
	result["ui_page"] = str(value.get("ui_page", ""))
	result["save_metadata"] = Dictionary(value.get("save_metadata", {})).duplicate(true)
	result["loaded_asset_summary"] = Dictionary(value.get("loaded_asset_summary", {})).duplicate(true)
	result["map_summary"] = Dictionary(value.get("map_summary", {})).duplicate(true)
	result["pokemon_data"] = Array(value.get("pokemon_data", [])).duplicate(true)
	return result

func _refresh_dynamic_menu_states() -> void:
	if _runtime_state.is_empty():
		return
	for key in _menu_states.keys():
		var normalized := _normalize_menu_name(str(key))
		_menu_states[normalized] = _build_menu_state(normalized, Dictionary(_menu_states[key]))
	if not _active_menu.is_empty() and _menu_states.has(_active_menu):
		menu_changed.emit(_active_menu, get_top_panel())

func _build_menu_state(menu_name: String, base_state: Dictionary) -> Dictionary:
	var state := base_state.duplicate(true)
	if state.is_empty():
		state = _default_menu_state(menu_name)
	var runtime_entries := _build_runtime_entries(menu_name)
	if not runtime_entries.is_empty():
		state["entries"] = runtime_entries
		var detail := _build_menu_detail(menu_name)
		if not detail.is_empty():
			var merged_state: Dictionary = {}
			if typeof(state.get("state")) == TYPE_DICTIONARY:
				merged_state = Dictionary(state.get("state", {})).duplicate(true)
			for key in detail.keys():
				merged_state[key] = detail[key]
			state["state"] = merged_state
			if _normalize_menu_name(menu_name) == "pokegear":
				state["cursor"] = int(merged_state.get("card_index", state.get("cursor", 0)))
	else:
		if not state.has("entries") or Array(state.get("entries", [])).is_empty():
			state["entries"] = _normalize_entries(DEFAULT_MENU_ENTRIES.get(menu_name, []))
	if not state.has("state") or typeof(state.get("state")) != TYPE_DICTIONARY:
		state["state"] = {}
	state["id"] = str(state.get("id", menu_name))
	state["title"] = str(state.get("title", _title_for_menu(menu_name)))
	state["kind"] = str(state.get("kind", MENU_KINDS.get(menu_name, "menu")))
	state["cursor"] = _clamp_panel_cursor(state, int(state.get("cursor", 0)))
	state["cancelable"] = bool(state.get("cancelable", true))
	state["wrap"] = bool(state.get("wrap", true))
	state["locked"] = bool(state.get("locked", false))
	return state

func _build_runtime_entries(menu_name: String) -> Array:
	match _normalize_menu_name(menu_name):
		"party_menu":
			return _build_party_entries("party_menu")
		"pokemon_menu":
			return _build_party_entries("pokemon_menu")
		"bag_menu":
			return _build_bag_entries()
		"pokedex":
			return _build_pokedex_entries()
		"pc_menu":
			return _build_pc_entries()
		"pokegear":
			return _build_pokegear_entries()
		"trainer_card":
			return _build_trainer_card_entries()
		"options_menu":
			return _build_options_entries()
		_:
			return []

func _build_menu_detail(menu_name: String) -> Dictionary:
	match _normalize_menu_name(menu_name):
		"party_menu":
			return _party_detail("party_menu")
		"pokemon_menu":
			return _party_detail("pokemon_menu")
		"bag_menu":
			return _bag_detail()
		"pokedex":
			return _pokedex_detail()
		"pc_menu":
			return _pc_detail()
		"pokegear":
			return _pokegear_detail()
		"trainer_card":
			return _trainer_card_detail()
		"options_menu":
			return _options_detail()
		_:
			return {}

func _runtime_sram() -> Dictionary:
	return Dictionary(_runtime_state.get("sram", {}))

func _runtime_wram() -> Dictionary:
	return Dictionary(_runtime_state.get("wram", {}))

func _runtime_save_metadata() -> Dictionary:
	return Dictionary(_runtime_state.get("save_metadata", {}))

func _party_entries_from_party(party: Variant) -> Array:
	var entries: Array = []
	if typeof(party) != TYPE_DICTIONARY:
		return entries
	var members: Array = Array(Dictionary(party).get("pokemon", []))
	for index in range(members.size()):
		var member: Variant = members[index]
		if member == null:
			continue
		var mon: Dictionary = Dictionary(member)
		var species := Dictionary(mon.get("species", {}))
		var species_name := str(species.get("id", "POKEMON"))
		var nickname := str(mon.get("nickname", species_name))
		var level := int(mon.get("level", 0))
		var hp := int(mon.get("hp", mon.get("current_hp", 0)))
		var max_hp := int(mon.get("max_hp", 0))
		var label_name := nickname if not nickname.is_empty() else species_name
		var label := "%s Lv%d %d/%d" % [label_name, level, hp, max_hp]
		entries.append({
			"id": "party_%d" % index,
			"label": label,
			"payload": {
				"party_index": index,
				"nickname": nickname,
				"species": species_name,
				"level": level,
				"hp": hp,
				"max_hp": max_hp,
				"status": str(mon.get("status", "")),
			},
		})
	return entries

func _build_party_entries(menu_name: String = "party_menu") -> Array:
	var controller_state := _party_controller_state(menu_name)
	var mode := str(controller_state.get("mode", "list"))
	if mode == "action":
		return _build_party_action_entries(controller_state, menu_name)
	var entries := _party_entries_from_party(_runtime_sram().get("party", {}))
	if entries.is_empty():
		entries = _normalize_entries(DEFAULT_MENU_ENTRIES.get(menu_name, DEFAULT_MENU_ENTRIES.get("party_menu", [])))
		return entries
	entries.append({
		"id": "%s_cancel" % menu_name,
		"label": "CANCEL",
		"payload": {"action": "cancel", "menu": menu_name},
	})
	return entries

func _build_party_action_entries(controller_state: Dictionary, menu_name: String) -> Array:
	var action_options: Array = Array(controller_state.get("action_options", PARTY_ACTION_OPTIONS))
	var action_cursor := int(controller_state.get("action_cursor", 0))
	var selected_index := int(controller_state.get("selected_index", 0))
	var entries: Array = []
	for index in range(action_options.size()):
		var action_name := str(action_options[index])
		var label := action_name
		if index == action_cursor:
			label = "> %s" % label
		entries.append({
			"id": "%s_action_%s" % [menu_name, action_name.to_lower()],
			"label": label,
			"payload": {
				"action": action_name.to_lower(),
				"intent": "pokemon_action",
				"menu": menu_name,
				"selected_index": selected_index,
				"selected_action": action_name,
			},
		})
	return entries

func _build_bag_entries() -> Array:
	var controller_state := _bag_controller_state()
	if str(controller_state.get("mode", "list")) == "actions":
		return _bag_action_entries(controller_state)
	return _bag_item_entries(controller_state)

func _build_pokedex_entries() -> Array:
	var controller_state := _pokedex_controller_state()
	var page := str(controller_state.get("page", "main"))
	if page == "search":
		return _build_pokedex_search_entries(controller_state)
	if page == "search_results":
		return _build_pokedex_search_result_entries(controller_state)
	if page == "entry_detail":
		return _build_pokedex_entry_detail_entries(controller_state)
	var sram := _runtime_sram()
	var seen := _pokedex_count(sram.get("pokedex_seen", []))
	var owned := _pokedex_count(sram.get("pokedex_owned", []))
	return [
		{"id": "pokedex_seen", "label": "SEEN (%d)" % seen, "payload": {"count": seen, "kind": "seen"}},
		{"id": "pokedex_owned", "label": "OWNED (%d)" % owned, "payload": {"count": owned, "kind": "owned"}},
		{"id": "pokedex_search", "label": "SEARCH", "payload": {"action": "search"}},
		{"id": "pokedex_options", "label": "OPTIONS", "payload": {"action": "options"}},
		{"id": "pokedex_unown", "label": "UNOWN", "payload": {"action": "unown"}},
	]

func _build_pc_entries() -> Array:
	var controller_state := _pc_controller_state()
	if str(controller_state.get("mode", "hub")) == "box":
		var active_box_index := int(controller_state.get("active_box_index", controller_state.get("box_index", 0)))
		var box_entries := _build_pc_box_entries(active_box_index)
		if not box_entries.is_empty():
			return box_entries
	var entries := _build_pc_hub_entries()
	if entries.is_empty():
		entries = _normalize_entries(DEFAULT_MENU_ENTRIES.get("pc_menu", []))
	return entries

func _build_pc_hub_entries() -> Array:
	var sram := _runtime_sram()
	var pc_items: Array = Array(sram.get("pc_items", []))
	var entries: Array = []
	entries.append({"id": "pc_withdraw", "label": "WITHDRAW", "payload": {"action": "withdraw", "intent": "pc_action", "mode": "withdraw"}})
	entries.append({"id": "pc_deposit", "label": "DEPOSIT", "payload": {"action": "deposit", "intent": "pc_action", "mode": "deposit"}})
	entries.append({"id": "pc_change_box", "label": "CHANGE BOX", "payload": {"action": "change_box", "intent": "pc_action", "mode": "change_box"}})
	entries.append({"id": "pc_move", "label": "MOVE W/O MAIL", "payload": {"action": "move", "intent": "pc_action", "mode": "move"}})
	entries.append({"id": "pc_see_ya", "label": "SEE YA!", "payload": {"action": "see_ya", "intent": "pc_action", "mode": "see_ya"}})
	var boxes: Array = Array(sram.get("pc_boxes", []))
	for index in range(boxes.size()):
		var box: Dictionary = Dictionary(boxes[index])
		var name := str(box.get("name", "BOX %d" % (index + 1)))
		var occupied := _count_non_null(Array(box.get("pokemon", [])))
		entries.append({
			"id": "box_%d" % index,
			"label": "%s (%d)" % [name, occupied],
			"payload": {
				"box_index": index,
				"name": name,
				"occupied": occupied,
				"intent": "pc_box",
			},
		})
	entries.append({
		"id": "pc_items",
		"label": "ITEMS (%d)" % pc_items.size(),
		"payload": {"count": pc_items.size(), "items": _normalize_payload(pc_items), "intent": "pc_items"},
	})
	return entries

func _build_pc_box_entries(box_index: int) -> Array:
	var boxes: Array = Array(_runtime_sram().get("pc_boxes", []))
	if box_index < 0 or box_index >= boxes.size():
		return []
	var box: Dictionary = Dictionary(boxes[box_index])
	var box_name := str(box.get("name", "BOX %d" % (box_index + 1)))
	var pokemon: Array = Array(box.get("pokemon", []))
	var entries: Array = []
	for slot_index in range(pokemon.size()):
		var member: Variant = pokemon[slot_index]
		if member == null:
			entries.append({
				"id": "pc_box_%d_slot_%d" % [box_index, slot_index],
				"label": "EMPTY",
				"payload": {
					"box_index": box_index,
					"box_name": box_name,
					"slot_index": slot_index,
					"empty": true,
					"intent": "pc_slot",
				},
			})
			continue
		var mon: Dictionary = Dictionary(member)
		var species := Dictionary(mon.get("species", {}))
		var species_name := str(species.get("id", "POKEMON"))
		var nickname := str(mon.get("nickname", species_name))
		var level := int(mon.get("level", 0))
		var hp := int(mon.get("hp", mon.get("current_hp", 0)))
		var max_hp := int(mon.get("max_hp", 0))
		var label_name := nickname if not nickname.is_empty() else species_name
		entries.append({
			"id": "pc_box_%d_slot_%d" % [box_index, slot_index],
			"label": "%s Lv%d %d/%d" % [label_name, level, hp, max_hp],
			"payload": {
				"box_index": box_index,
				"box_name": box_name,
				"slot_index": slot_index,
				"nickname": nickname,
				"species": species_name,
				"level": level,
				"hp": hp,
				"max_hp": max_hp,
				"status": str(mon.get("status", "")),
				"intent": "pc_slot",
			},
		})
	return entries

func _build_pokegear_entries() -> Array:
	var controller_state := _pokegear_controller_state()
	var available_cards: Array = Array(controller_state.get("available_cards", _pokegear_available_cards()))
	var current_card := str(controller_state.get("card", "CLOCK"))
	var entries: Array = []
	for card in available_cards:
		var card_name := str(card)
		var label := _pokegear_card_label(card_name, controller_state)
		if card_name == current_card:
			label = "> %s" % label
		entries.append({
			"id": "pokegear_%s" % card_name.to_lower(),
			"label": label,
			"payload": _pokegear_card_payload(card_name, controller_state),
		})
	return entries

func _build_trainer_card_entries() -> Array:
	var detail := _trainer_card_detail()
	var page := str(detail.get("page", "info"))
	var info_label := "INFO"
	var johto_label := "JOHTO BADGES (%d)" % int(detail.get("johto_badges", 0))
	var kanto_label := "KANTO BADGES (%d)" % int(detail.get("kanto_badges", 0))
	if page == "info":
		info_label = "> INFO"
	elif page == "johto_badges":
		johto_label = "> %s" % johto_label
	else:
		kanto_label = "> %s" % kanto_label
	return [
		{"id": "trainer_info", "label": info_label, "payload": {"page": "info", "action": "info", "summary": detail}},
		{"id": "trainer_johto", "label": johto_label, "payload": {"page": "johto_badges", "action": "johto_badges", "summary": detail}},
		{"id": "trainer_kanto", "label": kanto_label, "payload": {"page": "kanto_badges", "action": "kanto_badges", "summary": detail}},
	]

func _build_options_entries() -> Array:
	var controller_state := _options_controller_state()
	var cursor := int(controller_state.get("cursor", 0))
	var options := Dictionary(_runtime_sram().get("options", {}))
	var entries: Array = []
	for index in range(OPTIONS_FIELD_ORDER.size()):
		var field := str(OPTIONS_FIELD_ORDER[index])
		var label := str(OPTIONS_FIELD_LABELS.get(field, field.replace("_", " ").to_upper()))
		var current_value := _options_value_label(field, options.get(field, null))
		var row_label := "%s: %s" % [label, current_value]
		if index == cursor:
			row_label = "> %s" % row_label
		entries.append({
			"id": field,
			"label": row_label,
			"payload": {
				"field": field,
				"value": options.get(field, null),
				"action": "select",
			},
		})
	var cancel_label := "CANCEL"
	if cursor == OPTIONS_FIELD_ORDER.size():
		cancel_label = "> %s" % cancel_label
	entries.append({
		"id": "options_cancel",
		"label": cancel_label,
		"payload": {"action": "cancel"},
	})
	return entries

func _pokegear_default_controller_state() -> Dictionary:
	return {
		"card": "CLOCK",
		"card_index": 0,
		"available_cards": [],
		"map_cursor_landmark": 0,
		"map_player_landmark": 0,
		"map_region": "",
		"map_group": 0,
		"map_number": 0,
		"map_summary": {},
		"phone_cursor": 0,
		"phone_scroll": 0,
		"phone_numbers": [],
		"phone_contact_id": "",
		"radio_index": 0,
		"radio_frequency_raw": 0,
		"radio_frequency": 0.0,
		"radio_station_constant": "",
		"radio_station_name": "",
		"radio_station_song": "",
	}

func _pokegear_controller_state() -> Dictionary:
	var controller_state := _pokegear_default_controller_state()
	var wram := _runtime_wram()
	var has_saved_state := false
	var current_state: Variant = _menu_states.get("pokegear", {})
	if typeof(current_state) == TYPE_DICTIONARY:
		var state: Dictionary = Dictionary(current_state)
		var detail: Variant = state.get("state", {})
		if typeof(detail) == TYPE_DICTIONARY:
			var source: Dictionary = Dictionary(detail)
			if not source.is_empty():
				has_saved_state = true
			for key in source.keys():
				controller_state[key] = _normalize_payload(source[key])
	var available_cards: Array = _pokegear_available_cards()
	controller_state["available_cards"] = available_cards.duplicate(true)
	controller_state["phone_numbers"] = Array(_runtime_sram().get("phone_numbers", [])).duplicate(true)
	if not has_saved_state:
		controller_state["map_cursor_landmark"] = int(wram.get("pokegear_map_cursor_landmark", 0))
		controller_state["map_player_landmark"] = int(wram.get("pokegear_map_player_landmark", 0))
		controller_state["map_group"] = int(wram.get("wMapGroup", 0))
		controller_state["map_number"] = int(wram.get("wMapNumber", 0))
		controller_state["phone_cursor"] = max(0, int(wram.get("pokegear_phone_cursor_position", 0)))
		controller_state["phone_scroll"] = max(0, int(wram.get("pokegear_phone_scroll_position", 0)))
		controller_state["radio_index"] = max(0, int(_pokegear_radio_index_for_raw(int(wram.get("pokegear_radio_frequency_raw", 0)))))
		controller_state["radio_frequency_raw"] = int(wram.get("pokegear_radio_frequency_raw", 0))
	var current_card := str(_pokegear_card_name_from_index(int(wram.get("pokegear_card", 0)))).to_upper() if not has_saved_state else str(controller_state.get("card", _pokegear_card_name_from_index(int(wram.get("pokegear_card", 0))))).to_upper()
	if not available_cards.has(current_card):
		current_card = str(available_cards[0]) if not available_cards.is_empty() else "CLOCK"
	controller_state["card"] = current_card
	controller_state["card_index"] = max(0, available_cards.find(current_card))
	controller_state["pokegear_card"] = _pokegear_card_index_from_name(current_card)
	controller_state["radio_frequency"] = float(controller_state.get("radio_frequency", 0.0))
	var map_summary := Dictionary(_runtime_state.get("map_summary", {})).duplicate(true)
	if not has_saved_state or typeof(controller_state.get("map_summary", {})) != TYPE_DICTIONARY or Dictionary(controller_state.get("map_summary", {})).is_empty():
		controller_state["map_summary"] = map_summary
	return controller_state

func _pokegear_available_cards() -> Array:
	var wram := _runtime_wram()
	var flags := Dictionary(wram.get("engine_flags", {}))
	var cards: Array = ["CLOCK"]
	if bool(flags.get("ENGINE_POKEGEAR", false)):
		if bool(flags.get("ENGINE_MAP_CARD", false)):
			cards.append("MAP")
		if bool(flags.get("ENGINE_PHONE_CARD", false)):
			cards.append("PHONE")
		if bool(flags.get("ENGINE_RADIO_CARD", false)):
			cards.append("RADIO")
	return cards

func _pokegear_card_name_from_index(index: int) -> String:
	match index:
		1:
			return "MAP"
		2:
			return "PHONE"
		3:
			return "RADIO"
		_:
			return "CLOCK"

func _pokegear_card_index_from_name(card: String) -> int:
	match card:
		"MAP":
			return 1
		"PHONE":
			return 2
		"RADIO":
			return 3
		_:
			return 0

func _pokegear_card_label(card: String, controller_state: Dictionary = {}) -> String:
	match card:
		"CLOCK":
			return "CLOCK"
		"MAP":
			var map_state := _pokegear_map_state(controller_state)
			var map_name := str(map_state.get("map_name", ""))
			if not map_name.is_empty():
				return "MAP %s" % map_name
			return "MAP"
		"PHONE":
			var phone_numbers: Array = Array(controller_state.get("phone_numbers", _runtime_sram().get("phone_numbers", [])))
			return "PHONE (%d)" % phone_numbers.size()
		"RADIO":
			var radio_state := _pokegear_radio_state(controller_state)
			var frequency := float(radio_state.get("frequency", 0.0))
			if frequency > 0.0:
				return "RADIO %.1f" % frequency
			return "RADIO"
		_:
			return card.to_upper()

func _pokegear_card_payload(card: String, controller_state: Dictionary) -> Dictionary:
	var payload := _pokegear_detail()
	payload["card"] = card
	payload["selected"] = card == str(controller_state.get("card", "CLOCK"))
	payload["label"] = _pokegear_card_label(card, controller_state)
	if card == "PHONE":
		var phone_numbers: Array = Array(controller_state.get("phone_numbers", []))
		if phone_numbers.is_empty():
			phone_numbers = Array(_runtime_sram().get("phone_numbers", []))
		payload["contacts"] = phone_numbers.duplicate(true)
		payload["phone_contact_id"] = _pokegear_current_phone_contact(controller_state)
	elif card == "RADIO":
		var radio_state := _pokegear_radio_state(controller_state)
		for key in radio_state.keys():
			payload[key] = radio_state[key]
		payload["radio_index"] = int(controller_state.get("radio_index", 0))
	elif card == "MAP":
		var map_state := _pokegear_map_state(controller_state)
		for key in map_state.keys():
			payload[key] = map_state[key]
	elif card == "CLOCK":
		payload["clock"] = true
	return payload

func _party_detail(menu_name: String = "party_menu") -> Dictionary:
	var controller_state := _party_controller_state(menu_name)
	var sram := _runtime_sram()
	var members: Array = Array(Dictionary(sram.get("party", {})).get("pokemon", []))
	return {
		"menu": menu_name,
		"mode": str(controller_state.get("mode", "list")),
		"count": _count_non_null(members),
		"slots": members.size(),
		"selected_index": int(controller_state.get("selected_index", _runtime_wram().get("wCurPartyMon", 0))),
		"action_cursor": int(controller_state.get("action_cursor", 0)),
		"action_options": Array(controller_state.get("action_options", PARTY_ACTION_OPTIONS)).duplicate(true),
		"selected_action": str(controller_state.get("selected_action", "")),
		"selected_entry": Dictionary(controller_state.get("selected_entry", {})).duplicate(true),
		"action_intent": _party_action_intent(controller_state),
	}

func _party_action_intent(controller_state: Dictionary) -> Dictionary:
	var selected_entry := Dictionary(controller_state.get("selected_entry", {}))
	var payload: Dictionary = Dictionary(selected_entry.get("payload", {})).duplicate(true)
	var selected_action := str(controller_state.get("selected_action", ""))
	if selected_action.is_empty():
		selected_action = str(payload.get("action", ""))
	return {
		"intent": "pokemon_action",
		"action": selected_action.to_lower(),
		"selected_index": int(controller_state.get("selected_index", 0)),
		"selected_action": selected_action,
		"selected_entry": selected_entry.duplicate(true),
		"payload": payload,
	}

func _bag_detail() -> Dictionary:
	var controller_state := _bag_controller_state()
	var sram := _runtime_sram()
	var pocket_summaries: Array = []
	var current_pocket_index := _bag_clamp_pocket_index(int(controller_state.get("pocket_index", 0)))
	for pocket_index in range(BAG_POCKET_SEQUENCE.size()):
		var pocket_entries := _bag_entries_for_pocket(pocket_index)
		var quantity_total := 0
		var item_count := 0
		for entry in pocket_entries:
			var payload: Dictionary = Dictionary(Dictionary(entry).get("payload", {}))
			var qty := int(payload.get("quantity", 0))
			if qty <= 0:
				continue
			item_count += 1
			quantity_total += qty
		pocket_summaries.append({
			"pocket_index": pocket_index,
			"pocket": BAG_POCKET_SEQUENCE[pocket_index],
			"item_count": item_count,
			"quantity_total": quantity_total,
			"selected": pocket_index == current_pocket_index,
		})
	var selected_entry := _bag_selected_entry(controller_state)
	var selected_payload: Dictionary = Dictionary(selected_entry.get("payload", {}))
	var selected_pocket_index := int(selected_payload.get("pocket_index", current_pocket_index))
	var selected_pocket := _bag_pocket_label(selected_pocket_index)
	return {
		"mode": str(controller_state.get("mode", "list")),
		"pocket_index": current_pocket_index,
		"pocket": selected_pocket,
		"list_index": int(controller_state.get("list_index", 0)),
		"scroll_offset": int(controller_state.get("scroll_offset", 0)),
		"action_index": int(controller_state.get("action_index", 0)),
		"action_options": Array(controller_state.get("action_options", [])).duplicate(true),
		"selected_item_id": str(controller_state.get("selected_item_id", "")),
		"selected_item_label": str(controller_state.get("selected_item_label", "")),
		"selected_quantity": int(controller_state.get("selected_quantity", 0)),
		"selected_entry": selected_entry,
		"items": _count_dictionary_entries(Dictionary(sram.get("items", {}))),
		"balls": _count_dictionary_entries(Dictionary(sram.get("balls", {}))),
		"key_items": _count_dictionary_entries(Dictionary(sram.get("key_items", {}))),
		"tm_hm": _count_dictionary_entries(Dictionary(sram.get("tm_hm", {}))),
		"pocket_summaries": pocket_summaries,
		"registered_item": str(_runtime_wram().get("wRegisteredItem", sram.get("registered_item", ""))),
	}

func _bag_default_controller_state() -> Dictionary:
	return {
		"mode": "list",
		"pocket_index": 0,
		"list_index": 0,
		"scroll_offset": 0,
		"action_index": 0,
		"action_options": [],
		"pocket_cursors": {},
		"pocket_scroll_offsets": {},
		"selected_item_id": "",
		"selected_item_label": "",
		"selected_quantity": 0,
		"selected_entry": {},
		"last_action": "",
	}

func _bag_controller_state() -> Dictionary:
	var controller_state := _bag_default_controller_state()
	if not _menu_states.has("bag_menu"):
		controller_state["action_options"] = _bag_action_options_for_pocket(int(controller_state.get("pocket_index", 0)))
		return controller_state
	var state: Dictionary = Dictionary(_menu_states["bag_menu"])
	var detail: Variant = state.get("state", {})
	if typeof(detail) == TYPE_DICTIONARY:
		var source: Dictionary = Dictionary(detail)
		for key in source.keys():
			controller_state[key] = _normalize_payload(source[key])
	controller_state["mode"] = str(controller_state.get("mode", "list"))
	controller_state["pocket_index"] = _bag_clamp_pocket_index(int(controller_state.get("pocket_index", 0)))
	controller_state["list_index"] = max(0, int(controller_state.get("list_index", 0)))
	controller_state["scroll_offset"] = max(0, int(controller_state.get("scroll_offset", 0)))
	controller_state["action_index"] = max(0, int(controller_state.get("action_index", 0)))
	if typeof(controller_state.get("pocket_cursors", {})) != TYPE_DICTIONARY:
		controller_state["pocket_cursors"] = {}
	if typeof(controller_state.get("pocket_scroll_offsets", {})) != TYPE_DICTIONARY:
		controller_state["pocket_scroll_offsets"] = {}
	if typeof(controller_state.get("selected_entry", {})) != TYPE_DICTIONARY:
		controller_state["selected_entry"] = {}
	controller_state["action_options"] = _bag_action_options_for_pocket(int(controller_state.get("pocket_index", 0)))
	return controller_state

func _bag_commit_state(controller_state: Dictionary) -> Dictionary:
	var state := _ensure_menu_state("bag_menu")
	var normalized := controller_state.duplicate(true)
	normalized["mode"] = "actions" if str(normalized.get("mode", "list")) == "actions" else "list"
	normalized["pocket_index"] = _bag_clamp_pocket_index(int(normalized.get("pocket_index", 0)))
	normalized["action_options"] = _bag_action_options_for_pocket(int(normalized.get("pocket_index", 0)))
	if typeof(normalized.get("pocket_cursors", {})) != TYPE_DICTIONARY:
		normalized["pocket_cursors"] = {}
	if typeof(normalized.get("pocket_scroll_offsets", {})) != TYPE_DICTIONARY:
		normalized["pocket_scroll_offsets"] = {}
	if normalized["mode"] == "actions":
		var action_entries := _bag_action_entries(normalized)
		normalized["action_index"] = clamp(int(normalized.get("action_index", 0)), 0, max(0, action_entries.size() - 1))
		normalized["selected_entry"] = _bag_selected_entry(normalized)
		state["entries"] = action_entries
		state["cursor"] = _clamp_panel_cursor({"entries": action_entries}, int(normalized.get("action_index", 0)))
		state["kind"] = "submenu"
	else:
		var item_entries := _bag_item_entries(normalized)
		normalized["list_index"] = clamp(int(normalized.get("list_index", 0)), 0, max(0, item_entries.size() - 1))
		normalized["selected_entry"] = _bag_selected_entry(normalized)
		state["entries"] = item_entries
		state["cursor"] = _clamp_panel_cursor({"entries": item_entries}, int(normalized.get("list_index", 0)))
		state["kind"] = "menu"
	state["state"] = normalized
	state["id"] = "bag_menu"
	state["title"] = _title_for_menu("bag_menu")
	state["cancelable"] = true
	state["wrap"] = true
	state["locked"] = false
	_menu_states["bag_menu"] = state
	if _active_menu == "bag_menu":
		menu_changed.emit(_active_menu, get_top_panel())
		state_changed.emit(get_state())
	return state

func _bag_pocket_label(pocket_index: int) -> String:
	return BAG_POCKET_SEQUENCE[_bag_clamp_pocket_index(pocket_index)]

func _bag_clamp_pocket_index(pocket_index: int) -> int:
	return clamp(pocket_index, 0, BAG_POCKET_SEQUENCE.size() - 1)

func _bag_storage_for_pocket(pocket_index: int) -> Dictionary:
	match _bag_pocket_label(pocket_index):
		"ITEMS":
			return Dictionary(_runtime_sram().get("items", {}))
		"BALL":
			return Dictionary(_runtime_sram().get("balls", {}))
		"KEY":
			return Dictionary(_runtime_sram().get("key_items", {}))
		"TM/HM":
			return Dictionary(_runtime_sram().get("tm_hm", {}))
		_:
			return {}

func _bag_action_options_for_pocket(pocket_index: int) -> Array:
	var pocket_label := _bag_pocket_label(pocket_index)
	if BAG_ACTION_OPTIONS.has(pocket_label):
		return Array(BAG_ACTION_OPTIONS[pocket_label]).duplicate(true)
	return ["USE", "QUIT"]

func _bag_entries_for_pocket(pocket_index: int) -> Array:
	var pocket_label := _bag_pocket_label(pocket_index)
	var storage := _bag_storage_for_pocket(pocket_index)
	var keys: Array = Array(storage.keys())
	keys.sort()
	var entries: Array = []
	for key in keys:
		var qty := int(storage.get(key, 0))
		if qty <= 0:
			continue
		var item_id := str(key)
		var item_label := item_id.replace("_", " ")
		var label := item_label
		if pocket_label != "KEY":
			label = "%s x%02d" % [item_label, qty]
		entries.append({
			"id": "bag_%s_%s" % [pocket_label.to_lower(), item_id.to_lower()],
			"label": label,
			"payload": {
				"action": "item",
				"item_id": item_id,
				"item_label": item_label,
				"pocket": pocket_label,
				"pocket_index": _bag_clamp_pocket_index(pocket_index),
				"quantity": qty,
				"can_register": pocket_label == "KEY",
				"can_toss": pocket_label != "KEY" and pocket_label != "TM/HM",
			},
		})
	return entries

func _bag_item_entries(controller_state: Dictionary) -> Array:
	var pocket_index := _bag_clamp_pocket_index(int(controller_state.get("pocket_index", 0)))
	var item_entries := _bag_entries_for_pocket(pocket_index)
	var entries := item_entries.duplicate(true)
	entries.append({
		"id": "bag_cancel",
		"label": "CANCEL",
		"payload": {
			"action": "cancel",
			"pocket": _bag_pocket_label(pocket_index),
			"pocket_index": pocket_index,
		},
	})
	return entries

func _bag_action_entries(controller_state: Dictionary) -> Array:
	var selected_entry := _bag_selected_entry(controller_state)
	if selected_entry.is_empty():
		return [{
			"id": "bag_action_cancel",
			"label": "CANCEL",
			"payload": {"action": "cancel"},
		}]
	var payload: Dictionary = Dictionary(selected_entry.get("payload", {}))
	var item_id := str(payload.get("item_id", ""))
	var item_label := str(payload.get("item_label", item_id))
	var pocket_label := str(payload.get("pocket", _bag_pocket_label(int(payload.get("pocket_index", 0)))))
	var quantity := int(payload.get("quantity", 0))
	var action_options: Array = _bag_action_options_for_pocket(int(payload.get("pocket_index", 0)))
	var action_index: int = clampi(int(controller_state.get("action_index", 0)), 0, action_options.size() - 1)
	var entries: Array = []
	for index in range(action_options.size()):
		var option := str(action_options[index])
		var label := option if index != action_index else "> %s" % option
		entries.append({
			"id": "bag_action_%s" % option.to_lower(),
			"label": label,
			"payload": {
				"action": option.to_lower(),
				"item_id": item_id,
				"item_label": item_label,
				"pocket": pocket_label,
				"quantity": quantity,
			},
		})
	return entries

func _bag_selected_entry(controller_state: Dictionary) -> Dictionary:
	var mode := str(controller_state.get("mode", "list"))
	var pocket_index := _bag_clamp_pocket_index(int(controller_state.get("pocket_index", 0)))
	var item_entries := _bag_entries_for_pocket(pocket_index)
	if mode == "actions":
		if item_entries.is_empty():
			return {}
		var item_id := str(controller_state.get("selected_item_id", ""))
		if item_id.is_empty():
			var fallback_index: int = clampi(int(controller_state.get("list_index", 0)), 0, item_entries.size() - 1)
			return _normalize_entry(item_entries[fallback_index])
		for entry in item_entries:
			var payload: Dictionary = Dictionary(Dictionary(entry).get("payload", {}))
			if str(payload.get("item_id", "")) == item_id:
				return _normalize_entry(entry)
		var fallback: int = clampi(int(controller_state.get("list_index", 0)), 0, item_entries.size() - 1)
		return _normalize_entry(item_entries[fallback])
	var list_entries := _bag_item_entries(controller_state)
	if list_entries.is_empty():
		return {}
	var list_index: int = clampi(int(controller_state.get("list_index", 0)), 0, list_entries.size() - 1)
	if list_index >= item_entries.size():
		return {
			"id": "bag_cancel",
			"label": "CANCEL",
			"payload": {
				"action": "cancel",
				"pocket": _bag_pocket_label(pocket_index),
				"pocket_index": pocket_index,
			},
		}
	return _normalize_entry(list_entries[list_index])

func _handle_bag_input(state: Dictionary, frame_input: Dictionary) -> Dictionary:
	var result := {
		"consumed": false,
		"action": "",
		"top_panel": get_top_panel(),
		"selection": {},
		"depth": 1 if is_active() else 0,
		"active_menu": _active_menu,
		"input_locked": _input_locked,
		"can_accept_input": can_accept_input(),
	}
	var pressed: Dictionary = Dictionary(frame_input.get("pressed", {}))
	var controller_state := _bag_controller_state()
	if str(controller_state.get("mode", "list")) == "actions":
		return _handle_bag_action_input(pressed, result)
	return _handle_bag_list_input(pressed, result)

func _handle_bag_list_input(pressed: Dictionary, result: Dictionary) -> Dictionary:
	var controller_state := _bag_controller_state()
	var pocket_index := _bag_clamp_pocket_index(int(controller_state.get("pocket_index", 0)))
	var item_entries := _bag_entries_for_pocket(pocket_index)
	var total_entries := item_entries.size() + 1
	var cursor: int = clampi(int(controller_state.get("list_index", 0)), 0, max(0, total_entries - 1))
	var scroll_offset: int = max(0, int(controller_state.get("scroll_offset", 0)))
	if bool(pressed.get("left", false)) or bool(pressed.get("right", false)):
		var delta := -1 if bool(pressed.get("left", false)) else 1
		var next_pocket := posmod(pocket_index + delta, BAG_POCKET_SEQUENCE.size())
		var pocket_cursors: Dictionary = Dictionary(controller_state.get("pocket_cursors", {}))
		var pocket_scrolls: Dictionary = Dictionary(controller_state.get("pocket_scroll_offsets", {}))
		pocket_cursors[pocket_index] = cursor
		pocket_scrolls[pocket_index] = scroll_offset
		controller_state["pocket_cursors"] = pocket_cursors
		controller_state["pocket_scroll_offsets"] = pocket_scrolls
		controller_state["pocket_index"] = next_pocket
		var restored_cursor: int = int(pocket_cursors.get(next_pocket, 0))
		var restored_scroll: int = int(pocket_scrolls.get(next_pocket, 0))
		var restored_total: int = _bag_entries_for_pocket(next_pocket).size() + 1
		controller_state["list_index"] = clampi(restored_cursor, 0, max(0, restored_total - 1))
		controller_state["scroll_offset"] = _bag_scroll_offset_for(controller_state["list_index"], restored_scroll, restored_total)
		_bag_commit_state(controller_state)
		result["action"] = "switch_pocket"
		result["consumed"] = true
		result["top_panel"] = get_top_panel()
		return result
	if bool(pressed.get("up", false)):
		cursor = _bag_step_cursor(cursor, -1, total_entries)
		scroll_offset = _bag_scroll_offset_for(cursor, scroll_offset, total_entries)
		controller_state["list_index"] = cursor
		controller_state["scroll_offset"] = scroll_offset
		controller_state["pocket_cursors"] = _bag_update_pocket_cursor_map(Dictionary(controller_state.get("pocket_cursors", {})), pocket_index, cursor)
		controller_state["pocket_scroll_offsets"] = _bag_update_pocket_scroll_map(Dictionary(controller_state.get("pocket_scroll_offsets", {})), pocket_index, scroll_offset)
		_bag_commit_state(controller_state)
		result["action"] = "move_up"
		result["consumed"] = true
		result["top_panel"] = get_top_panel()
		return result
	if bool(pressed.get("down", false)):
		cursor = _bag_step_cursor(cursor, 1, total_entries)
		scroll_offset = _bag_scroll_offset_for(cursor, scroll_offset, total_entries)
		controller_state["list_index"] = cursor
		controller_state["scroll_offset"] = scroll_offset
		controller_state["pocket_cursors"] = _bag_update_pocket_cursor_map(Dictionary(controller_state.get("pocket_cursors", {})), pocket_index, cursor)
		controller_state["pocket_scroll_offsets"] = _bag_update_pocket_scroll_map(Dictionary(controller_state.get("pocket_scroll_offsets", {})), pocket_index, scroll_offset)
		_bag_commit_state(controller_state)
		result["action"] = "move_down"
		result["consumed"] = true
		result["top_panel"] = get_top_panel()
		return result
	if _has_pressed_in_group(pressed, CONFIRM_BUTTONS):
		var selection := _bag_selected_entry(controller_state)
		var payload: Dictionary = Dictionary(selection.get("payload", {}))
		if str(payload.get("action", "")) == "cancel":
			deactivate_menu()
			result["action"] = "cancel"
			result["consumed"] = true
			result["top_panel"] = {}
			return result
		if bool(payload.get("can_register", false)) and bool(pressed.get("start", false)):
			var register_state := controller_state.duplicate(true)
			register_state["selected_entry"] = selection
			register_state["selected_item_id"] = str(payload.get("item_id", ""))
			register_state["selected_item_label"] = str(payload.get("item_label", ""))
			register_state["selected_quantity"] = int(payload.get("quantity", 0))
			register_state["last_action"] = "sel"
			_bag_commit_state(register_state)
			result["action"] = "sel"
			result["consumed"] = true
			result["selection"] = selection
			result["top_panel"] = get_top_panel()
			return result
		var action_state := controller_state.duplicate(true)
		action_state["mode"] = "actions"
		action_state["selected_entry"] = selection
		action_state["selected_item_id"] = str(payload.get("item_id", ""))
		action_state["selected_item_label"] = str(payload.get("item_label", ""))
		action_state["selected_quantity"] = int(payload.get("quantity", 0))
		action_state["action_options"] = _bag_action_options_for_pocket(int(payload.get("pocket_index", pocket_index)))
		action_state["action_index"] = 0
		_bag_commit_state(action_state)
		result["action"] = "open_actions"
		result["consumed"] = true
		result["selection"] = selection
		result["top_panel"] = get_top_panel()
		return result
	if bool(pressed.get("select", false)):
		var selection := _bag_selected_entry(controller_state)
		var payload: Dictionary = Dictionary(selection.get("payload", {}))
		if bool(payload.get("can_register", false)):
			var register_state := controller_state.duplicate(true)
			register_state["selected_entry"] = selection
			register_state["selected_item_id"] = str(payload.get("item_id", ""))
			register_state["selected_item_label"] = str(payload.get("item_label", ""))
			register_state["selected_quantity"] = int(payload.get("quantity", 0))
			register_state["last_action"] = "sel"
			_bag_commit_state(register_state)
			result["action"] = "sel"
			result["consumed"] = true
			result["selection"] = selection
			result["top_panel"] = get_top_panel()
			return result
		if _has_any_pressed_button(pressed):
			result["consumed"] = true
		return result
	if _has_pressed_in_group(pressed, CANCEL_BUTTONS):
		deactivate_menu()
		result["action"] = "cancel"
		result["consumed"] = true
		result["top_panel"] = {}
		return result
	if _has_any_pressed_button(pressed):
		result["consumed"] = true
	return result

func _handle_bag_action_input(pressed: Dictionary, result: Dictionary) -> Dictionary:
	var controller_state := _bag_controller_state()
	var action_options := Array(controller_state.get("action_options", []))
	if action_options.is_empty():
		controller_state["mode"] = "list"
		_bag_commit_state(controller_state)
		return result
	var action_index: int = clampi(int(controller_state.get("action_index", 0)), 0, action_options.size() - 1)
	if bool(pressed.get("up", false)):
		action_index = _bag_step_cursor(action_index, -1, action_options.size())
		controller_state["action_index"] = action_index
		_bag_commit_state(controller_state)
		result["action"] = "move_up"
		result["consumed"] = true
		result["top_panel"] = get_top_panel()
		return result
	if bool(pressed.get("down", false)):
		action_index = _bag_step_cursor(action_index, 1, action_options.size())
		controller_state["action_index"] = action_index
		_bag_commit_state(controller_state)
		result["action"] = "move_down"
		result["consumed"] = true
		result["top_panel"] = get_top_panel()
		return result
	if _has_pressed_in_group(pressed, CANCEL_BUTTONS):
		controller_state["mode"] = "list"
		controller_state["action_index"] = 0
		_bag_commit_state(controller_state)
		result["action"] = "cancel"
		result["consumed"] = true
		result["top_panel"] = get_top_panel()
		return result
	if _has_pressed_in_group(pressed, CONFIRM_BUTTONS):
		var action := str(action_options[action_index])
		var payload: Dictionary = Dictionary(_bag_selected_entry(controller_state).get("payload", {}))
		controller_state["last_action"] = action.to_lower()
		controller_state["mode"] = "list"
		controller_state["action_index"] = action_index
		_bag_commit_state(controller_state)
		result["consumed"] = true
		result["selection"] = {
			"id": str(payload.get("item_id", "")),
			"label": str(payload.get("item_label", "")),
			"payload": payload,
		}
		result["top_panel"] = get_top_panel()
		if action == "QUIT":
			result["action"] = "quit"
			return result
		if action == "TOSS":
			result["action"] = "toss"
			return result
		result["action"] = action.to_lower()
		return result
	if _has_any_pressed_button(pressed):
		result["consumed"] = true
	return result

func _bag_step_cursor(cursor: int, delta: int, total: int) -> int:
	if total <= 0:
		return 0
	var next_cursor: int = cursor + delta
	if next_cursor < 0:
		next_cursor = total - 1
	elif next_cursor >= total:
		next_cursor = 0
	return next_cursor

func _bag_scroll_offset_for(cursor: int, current_scroll: int, total: int) -> int:
	if total <= BAG_LIST_VISIBLE_ROWS:
		return 0
	var scroll: int = max(0, current_scroll)
	if cursor < scroll:
		return cursor
	if cursor >= scroll + BAG_LIST_VISIBLE_ROWS:
		return cursor - BAG_LIST_VISIBLE_ROWS + 1
	return scroll

func _bag_update_pocket_cursor_map(cursors: Dictionary, pocket_index: int, cursor: int) -> Dictionary:
	var result := cursors.duplicate(true)
	result[pocket_index] = cursor
	return result

func _bag_update_pocket_scroll_map(scrolls: Dictionary, pocket_index: int, scroll_offset: int) -> Dictionary:
	var result := scrolls.duplicate(true)
	result[pocket_index] = scroll_offset
	return result

func _pokedex_detail() -> Dictionary:
	var sram := _runtime_sram()
	var controller_state := _pokedex_controller_state()
	var page := str(controller_state.get("page", "main"))
	return {
		"seen": _pokedex_count(sram.get("pokedex_seen", [])),
		"owned": _pokedex_count(sram.get("pokedex_owned", [])),
		"page": page,
		"cursor": int(get_menu_cursor("pokedex")),
		"search_cursor": int(controller_state.get("search_cursor", 0)),
		"search_results_cursor": int(controller_state.get("search_results_cursor", 0)),
		"search_results_scroll": int(controller_state.get("search_results_scroll", 0)),
		"search_type_1": int(controller_state.get("search_type_1", 1)),
		"search_type_2": int(controller_state.get("search_type_2", 0)),
		"search_type_1_label": _pokedex_search_type_label(int(controller_state.get("search_type_1", 1))),
		"search_type_2_label": _pokedex_search_type_label(int(controller_state.get("search_type_2", 0))),
		"search_results_count": _pokedex_search_results(controller_state).size(),
		"selected_action": str(controller_state.get("selected_action", "")),
		"entry_detail": _pokedex_entry_detail(controller_state),
	}

func _build_pokedex_search_entries(state: Dictionary) -> Array:
	return _pokedex_search_entries(state)

func _pc_detail() -> Dictionary:
	var sram := _runtime_sram()
	var controller_state := _pc_controller_state()
	var box_index := int(controller_state.get("active_box_index", controller_state.get("box_index", 0)))
	var boxes: Array = Array(sram.get("pc_boxes", []))
	var current_box: Dictionary = {}
	if box_index >= 0 and box_index < boxes.size():
		current_box = Dictionary(boxes[box_index])
	return {
		"boxes": Array(sram.get("pc_boxes", [])).size(),
		"items": Array(sram.get("pc_items", [])).size(),
		"mode": str(controller_state.get("mode", "hub")),
		"active_box_index": box_index,
		"active_box_name": str(current_box.get("name", "")),
		"active_box_occupied": _count_non_null(Array(current_box.get("pokemon", []))),
		"box_cursor": int(controller_state.get("box_cursor", 0)),
		"hub_cursor": int(controller_state.get("hub_cursor", 0)),
		"selected_index": int(controller_state.get("selected_index", 0)),
		"selected_action": str(controller_state.get("selected_action", "")),
		"selected_entry": Dictionary(controller_state.get("selected_entry", {})).duplicate(true),
		"action_intent": _pc_action_intent(controller_state),
		"pending_action": Dictionary(controller_state.get("pending_action", {})).duplicate(true),
	}

func _pc_action_intent(controller_state: Dictionary) -> Dictionary:
	var selected_entry := Dictionary(controller_state.get("selected_entry", {}))
	var payload: Dictionary = Dictionary(selected_entry.get("payload", {})).duplicate(true)
	var pending_action: Dictionary = Dictionary(controller_state.get("pending_action", {})).duplicate(true)
	var action := str(pending_action.get("action", controller_state.get("selected_action", payload.get("action", ""))))
	var intent := str(pending_action.get("intent", payload.get("intent", "pc_action")))
	return {
		"intent": intent if not intent.is_empty() else "pc_action",
		"action": action,
		"mode": str(controller_state.get("mode", "hub")),
		"box_index": int(payload.get("box_index", controller_state.get("active_box_index", 0))),
		"slot_index": int(payload.get("slot_index", -1)),
		"target_box": int(payload.get("target_box", -1)),
		"target_slot": int(payload.get("target_slot", -1)),
		"party_slot": int(payload.get("party_slot", -1)),
		"selected_entry": selected_entry.duplicate(true),
		"payload": payload,
	}

func _pokegear_detail() -> Dictionary:
	var controller_state := _pokegear_controller_state()
	var map_state := _pokegear_map_state(controller_state)
	var phone_state := _pokegear_phone_state(controller_state)
	var radio_state := _pokegear_radio_state(controller_state)
	var current_card := str(controller_state.get("card", "CLOCK"))
	return {
		"card": current_card,
		"page": current_card.to_lower(),
		"available_cards": Array(controller_state.get("available_cards", [])).duplicate(true),
		"card_index": int(controller_state.get("card_index", 0)),
		"map_region": str(map_state.get("map_region", "")),
		"map_group": int(map_state.get("map_group", 0)),
		"map_number": int(map_state.get("map_number", 0)),
		"map_name": str(map_state.get("map_name", "")),
		"map_constant": str(map_state.get("map_constant", "")),
		"map_summary": Dictionary(map_state.get("map_summary", {})).duplicate(true),
		"map_player_landmark": int(map_state.get("map_player_landmark", 0)),
		"map_cursor_landmark": int(map_state.get("map_cursor_landmark", 0)),
		"phone_numbers": int(phone_state.get("phone_numbers", 0)),
		"phone_cursor": int(phone_state.get("phone_cursor", 0)),
		"phone_scroll": int(phone_state.get("phone_scroll", 0)),
		"phone_contact_id": str(phone_state.get("phone_contact_id", "")),
		"phone_contact_label": str(phone_state.get("phone_contact_label", "")),
		"radio_index": int(radio_state.get("radio_index", 0)),
		"radio_frequency_raw": int(radio_state.get("radio_frequency_raw", 0)),
		"radio_frequency": float(radio_state.get("radio_frequency", 0.0)),
		"radio_handler": str(radio_state.get("radio_handler", "")),
		"radio_station_constant": str(radio_state.get("radio_station_constant", "")),
		"radio_station_name": str(radio_state.get("radio_station_name", "")),
		"radio_station_song": str(radio_state.get("radio_station_song", "")),
	}

func _pokegear_map_state(controller_state: Dictionary = {}) -> Dictionary:
	var sram := _runtime_sram()
	var wram := _runtime_wram()
	var summary := Dictionary(_runtime_state.get("map_summary", {}))
	var map_group := int(summary.get("group_id", summary.get("groupId", wram.get("wMapGroup", 0))))
	var map_number := int(summary.get("map_id", summary.get("mapId", wram.get("wMapNumber", 0))))
	var map_name := str(summary.get("map_name", summary.get("name", summary.get("mapName", ""))))
	var map_constant := str(summary.get("map_constant", summary.get("constant", "")))
	if map_name.is_empty() and not map_constant.is_empty():
		map_name = map_constant.replace("_", " ")
	var map_region := str(summary.get("group_name", summary.get("region", wram.get("pokegear_map_region", ""))))
	var player_landmark := int(controller_state.get("map_player_landmark", wram.get("pokegear_map_player_landmark", 0)))
	var cursor_landmark := int(controller_state.get("map_cursor_landmark", wram.get("pokegear_map_cursor_landmark", player_landmark)))
	return {
		"map_summary": summary.duplicate(true),
		"map_group": map_group,
		"map_number": map_number,
		"map_name": map_name,
		"map_constant": map_constant,
		"map_region": map_region,
		"map_player_landmark": player_landmark,
		"map_cursor_landmark": cursor_landmark,
		"phone_service": int(summary.get("phone_service", summary.get("phoneService", wram.get("wMapPhoneService", 0)))),
	}

func _pokegear_phone_state(controller_state: Dictionary) -> Dictionary:
	var sram := _runtime_sram()
	var numbers: Array = Array(sram.get("phone_numbers", []))
	var cursor: int = max(0, int(controller_state.get("phone_cursor", _runtime_wram().get("pokegear_phone_cursor_position", 0))))
	var scroll: int = max(0, int(controller_state.get("phone_scroll", _runtime_wram().get("pokegear_phone_scroll_position", 0))))
	if numbers.is_empty():
		return {
			"phone_numbers": 0,
			"phone_cursor": 0,
			"phone_scroll": 0,
			"phone_contact_id": "",
			"phone_contact_label": "",
		}
	cursor = clampi(cursor, 0, numbers.size() - 1)
	scroll = clampi(scroll, 0, max(0, numbers.size() - 4))
	if cursor < scroll:
		scroll = cursor
	elif cursor >= scroll + 4:
		scroll = max(0, cursor - 3)
	var contact_id := str(numbers[cursor])
	return {
		"phone_numbers": numbers.size(),
		"phone_cursor": cursor,
		"phone_scroll": scroll,
		"phone_contact_id": contact_id,
		"phone_contact_label": contact_id.replace("_", " "),
	}

func _pokegear_radio_state(controller_state: Dictionary = {}) -> Dictionary:
	var wram := _runtime_wram()
	var raw := int(controller_state.get("radio_frequency_raw", wram.get("pokegear_radio_frequency_raw", 0)))
	var index := int(controller_state.get("radio_index", -1))
	if index < 0:
		index = _pokegear_radio_index_for_raw(raw)
	if index < 0 or index >= POKEGEAR_RADIO_FREQUENCIES.size():
		index = 0
	var entry: Dictionary = Dictionary(POKEGEAR_RADIO_FREQUENCIES[index])
	var station_constant: String = _pokegear_radio_station_for_handler(str(entry.get("handler", "")))
	var station_name: String = str(POKEGEAR_RADIO_STATION_NAMES.get(station_constant, station_constant.replace("_", " ")))
	return {
		"radio_index": index,
		"radio_frequency_raw": int(entry.get("raw", raw)),
		"radio_frequency": float(entry.get("frequency", 0.0)),
		"radio_handler": str(entry.get("handler", "")),
		"radio_station_constant": station_constant,
		"radio_station_name": station_name,
		"radio_station_song": _pokegear_radio_song_for_constant(station_constant),
	}

func _pokegear_radio_index_for_raw(raw: int) -> int:
	for index in range(POKEGEAR_RADIO_FREQUENCIES.size()):
		var entry: Dictionary = Dictionary(POKEGEAR_RADIO_FREQUENCIES[index])
		if int(entry.get("raw", -1)) == raw:
			return index
	return 0

func _pokegear_radio_station_for_handler(handler: String) -> String:
	match handler:
		"PKMNTalkAndPokedexShow":
			return "POKEDEX_SHOW"
		"PokemonMusic":
			return "POKEMON_MUSIC"
		"LuckyChannel":
			return "LUCKY_CHANNEL"
		"BuenasPassword":
			return "BUENAS_PASSWORD"
		"RuinsOfAlphRadio":
			return "UNOWN_RADIO"
		"PlacesAndPeople":
			return "PLACES_AND_PEOPLE"
		"LetsAllSing":
			return "LETS_ALL_SING"
		"PokeFluteRadio":
			return "POKE_FLUTE_RADIO"
		"EvolutionRadio":
			return "EVOLUTION_RADIO"
		_:
			return ""

func _pokegear_radio_song_for_constant(constant: String) -> String:
	match constant:
		"POKEDEX_SHOW":
			return "MUSIC_POKEMON_CENTER"
		"POKEMON_MUSIC":
			return "MUSIC_TITLE"
		"LUCKY_CHANNEL":
			return "MUSIC_GAME_CORNER"
		"BUENAS_PASSWORD":
			return "MUSIC_BUENAS_PASSWORD"
		"PLACES_AND_PEOPLE":
			return "MUSIC_VIRIDIAN_CITY"
		"LETS_ALL_SING":
			return "MUSIC_BICYCLE"
		"POKE_FLUTE_RADIO":
			return "MUSIC_POKE_FLUTE_CHANNEL"
		"UNOWN_RADIO":
			return "MUSIC_RUINS_OF_ALPH_RADIO"
		"EVOLUTION_RADIO":
			return "MUSIC_LAKE_OF_RAGE_ROCKET_RADIO"
		"ROCKET_RADIO":
			return "MUSIC_ROCKET_OVERTURE"
		_:
			return ""

func _pokegear_current_phone_contact(controller_state: Dictionary) -> String:
	var phone_state := _pokegear_phone_state(controller_state)
	return str(phone_state.get("phone_contact_id", ""))

func _handle_pokegear_input(state: Dictionary, frame_input: Dictionary) -> Dictionary:
	var result := {
		"consumed": false,
		"action": "",
		"top_panel": get_top_panel(),
		"selection": {},
		"depth": 1 if is_active() else 0,
		"active_menu": _active_menu,
		"input_locked": _input_locked,
		"can_accept_input": can_accept_input(),
	}
	var pressed: Dictionary = Dictionary(frame_input.get("pressed", {}))
	var controller_state := _pokegear_controller_state()
	var card := str(controller_state.get("card", "CLOCK"))
	if _has_pressed_in_group(pressed, CANCEL_BUTTONS):
		deactivate_menu()
		result["action"] = "cancel"
		result["consumed"] = true
		result["top_panel"] = {}
		return result
	if bool(pressed.get("left", false)) or bool(pressed.get("right", false)):
		var direction := -1 if bool(pressed.get("left", false)) else 1
		var switched := _pokegear_switch_card(controller_state, direction)
		if switched != card:
			result["action"] = "switch_card"
			result["consumed"] = true
			result["top_panel"] = get_top_panel()
			return result
		result["consumed"] = true
		return result
	if card == "MAP":
		if _has_pressed_in_group(pressed, UP_BUTTONS):
			_pokegear_move_map_cursor(controller_state, -1)
			result["action"] = "move_up"
			result["consumed"] = true
			result["top_panel"] = get_top_panel()
			return result
		if _has_pressed_in_group(pressed, DOWN_BUTTONS):
			_pokegear_move_map_cursor(controller_state, 1)
			result["action"] = "move_down"
			result["consumed"] = true
			result["top_panel"] = get_top_panel()
			return result
	elif card == "PHONE":
		if _has_pressed_in_group(pressed, UP_BUTTONS):
			_pokegear_move_phone_cursor(controller_state, -1)
			result["action"] = "move_up"
			result["consumed"] = true
			result["top_panel"] = get_top_panel()
			return result
		if _has_pressed_in_group(pressed, DOWN_BUTTONS):
			_pokegear_move_phone_cursor(controller_state, 1)
			result["action"] = "move_down"
			result["consumed"] = true
			result["top_panel"] = get_top_panel()
			return result
		if _has_pressed_in_group(pressed, CONFIRM_BUTTONS):
			var phone_state := _pokegear_phone_state(controller_state)
			result["action"] = "call"
			result["selection"] = {
				"id": str(phone_state.get("phone_contact_id", "")),
				"label": str(phone_state.get("phone_contact_label", "")),
				"payload": phone_state,
			}
			result["consumed"] = true
			return result
	elif card == "RADIO":
		if _has_pressed_in_group(pressed, UP_BUTTONS):
			_pokegear_tune_radio(controller_state, 1)
			result["action"] = "move_up"
			result["consumed"] = true
			result["top_panel"] = get_top_panel()
			return result
		if _has_pressed_in_group(pressed, DOWN_BUTTONS):
			_pokegear_tune_radio(controller_state, -1)
			result["action"] = "move_down"
			result["consumed"] = true
			result["top_panel"] = get_top_panel()
			return result
	if _has_any_pressed_button(pressed):
		result["consumed"] = true
	return result

func _pokegear_switch_card(controller_state: Dictionary, direction: int) -> String:
	var available_cards: Array = Array(controller_state.get("available_cards", _pokegear_available_cards()))
	if available_cards.is_empty():
		available_cards = ["CLOCK"]
	var current_card := str(controller_state.get("card", "CLOCK"))
	var current_index := available_cards.find(current_card)
	if current_index < 0:
		current_index = 0
	var next_index := posmod(current_index + direction, available_cards.size())
	var next_card := str(available_cards[next_index])
	controller_state["card"] = next_card
	controller_state["card_index"] = next_index
	_pokegear_commit_state(controller_state)
	return next_card

func _pokegear_commit_state(controller_state: Dictionary) -> Dictionary:
	var state := _ensure_menu_state("pokegear")
	var normalized := controller_state.duplicate(true)
	var available_cards: Array = Array(normalized.get("available_cards", _pokegear_available_cards()))
	if available_cards.is_empty():
		available_cards = ["CLOCK"]
	normalized["available_cards"] = available_cards.duplicate(true)
	var card := str(normalized.get("card", "CLOCK"))
	if not available_cards.has(card):
		card = str(available_cards[0])
	normalized["card"] = card
	normalized["card_index"] = max(0, available_cards.find(card))
	normalized["pokegear_card"] = _pokegear_card_index_from_name(card)
	if card == "PHONE":
		var phone_state := _pokegear_phone_state(normalized)
		normalized["phone_cursor"] = int(phone_state.get("phone_cursor", 0))
		normalized["phone_scroll"] = int(phone_state.get("phone_scroll", 0))
		normalized["phone_contact_id"] = str(phone_state.get("phone_contact_id", ""))
	elif card == "RADIO":
		var radio_state := _pokegear_radio_state(normalized)
		normalized["radio_index"] = int(radio_state.get("radio_index", 0))
		normalized["radio_frequency_raw"] = int(radio_state.get("radio_frequency_raw", 0))
		normalized["radio_frequency"] = float(radio_state.get("radio_frequency", 0.0))
		normalized["radio_station_constant"] = str(radio_state.get("radio_station_constant", ""))
		normalized["radio_station_name"] = str(radio_state.get("radio_station_name", ""))
		normalized["radio_station_song"] = str(radio_state.get("radio_station_song", ""))
	elif card == "MAP":
		var map_state := _pokegear_map_state(normalized)
		for key in map_state.keys():
			if key == "map_summary":
				normalized[key] = Dictionary(map_state[key]).duplicate(true)
			else:
				normalized[key] = map_state[key]
	else:
		normalized["card_index"] = 0 if available_cards.has("CLOCK") else normalized["card_index"]
	normalized["page"] = card.to_lower()
	state["state"] = normalized
	state["entries"] = _build_pokegear_entries()
	state["cursor"] = _clamp_panel_cursor({"entries": state["entries"]}, normalized["card_index"])
	state["id"] = "pokegear"
	state["title"] = _title_for_menu("pokegear")
	state["kind"] = "menu"
	state["cancelable"] = true
	state["wrap"] = true
	state["locked"] = false
	_menu_states["pokegear"] = state
	if _active_menu == "pokegear":
		menu_changed.emit(_active_menu, get_top_panel())
		state_changed.emit(get_state())
	return state

func _pokegear_move_map_cursor(controller_state: Dictionary, offset: int) -> void:
	var map_state := _pokegear_map_state(controller_state)
	var cursor := int(map_state.get("map_cursor_landmark", 0))
	controller_state["map_cursor_landmark"] = cursor + offset
	_pokegear_commit_state(controller_state)

func _pokegear_move_phone_cursor(controller_state: Dictionary, offset: int) -> void:
	var phone_state := _pokegear_phone_state(controller_state)
	controller_state["phone_cursor"] = int(phone_state.get("phone_cursor", 0)) + offset
	_pokegear_commit_state(controller_state)

func _pokegear_tune_radio(controller_state: Dictionary, offset: int) -> void:
	var radio_state := _pokegear_radio_state(controller_state)
	var next_index := int(radio_state.get("radio_index", 0)) + offset
	if next_index < 0:
		next_index = POKEGEAR_RADIO_FREQUENCIES.size() - 1
	elif next_index >= POKEGEAR_RADIO_FREQUENCIES.size():
		next_index = 0
	controller_state["radio_index"] = next_index
	_pokegear_commit_state(controller_state)

func _trainer_card_detail() -> Dictionary:
	var sram := _runtime_sram()
	var badges := Dictionary(sram.get("badges", {}))
	var controller_state := _trainer_card_controller_state()
	var page := str(controller_state.get("page", _trainer_card_page_from_cursor(get_menu_cursor("trainer_card"))))
	var badge_summary := {
		"johto": _count_true(_normalize_bool_array(badges.get("johto", []))),
		"kanto": _count_true(_normalize_bool_array(badges.get("kanto", []))),
	}
	return {
		"player_name": str(_runtime_state.get("player_name", "")),
		"player_gender": str(_runtime_state.get("player_gender", "")),
		"money": int(sram.get("money", 0)),
		"johto_badges": int(badge_summary["johto"]),
		"kanto_badges": int(badge_summary["kanto"]),
		"badge_summary": badge_summary,
		"page": page,
		"page_index": _trainer_card_cursor_from_page(page),
		"has_kanto_badges": _trainer_card_has_kanto_badges(),
	}

func _options_detail() -> Dictionary:
	var controller_state := _options_controller_state()
	var options := Dictionary(_runtime_sram().get("options", {})).duplicate(true)
	return {
		"cursor": int(controller_state.get("cursor", 0)),
		"selected_field": str(controller_state.get("selected_field", "")),
		"selected_value": controller_state.get("selected_value", null),
		"values": Dictionary(controller_state.get("values", {})).duplicate(true),
		"options": options,
	}

func _pokedex_count(value: Variant) -> int:
	if typeof(value) == TYPE_ARRAY:
		var total := 0
		for entry in Array(value):
			var number := int(entry)
			for bit in range(8):
				if number & (1 << bit):
					total += 1
		return total
	if typeof(value) == TYPE_DICTIONARY:
		var dict_value: Dictionary = Dictionary(value)
		var total_dict := 0
		for key in dict_value.keys():
			total_dict += int(dict_value.get(key, 0) != 0)
		return total_dict
	return 0

func _count_non_null(entries: Array) -> int:
	var total := 0
	for entry in entries:
		if entry != null:
			total += 1
	return total

func _count_dictionary_entries(value: Dictionary) -> int:
	var total := 0
	for key in value.keys():
		if int(value.get(key, 0)) > 0:
			total += 1
	return total

func _count_true(entries: Array) -> int:
	var total := 0
	for entry in entries:
		if bool(entry):
			total += 1
	return total

func _normalize_bool_array(value: Variant) -> Array:
	var result: Array = []
	if typeof(value) != TYPE_ARRAY:
		return result
	for entry in Array(value):
		result.append(bool(entry))
	return result

func _duplicate_dictionary(value: Dictionary) -> Dictionary:
	return Dictionary(value).duplicate(true)

func _restore_menu_states_from_snapshot(data: Dictionary) -> void:
	var menus_value: Variant = data.get("menus", {})
	if typeof(menus_value) == TYPE_DICTIONARY:
		var source_menus: Dictionary = menus_value
		for key in source_menus.keys():
			var menu_name := _normalize_menu_name(str(key))
			if not menu_name.is_empty():
				var state := _normalize_state(source_menus.get(key, {}))
				state["id"] = menu_name
				_menu_states[menu_name] = state
	if _menu_states.is_empty():
		var stack_value: Variant = data.get("stack", data.get("panels", []))
		if typeof(stack_value) == TYPE_ARRAY:
			for entry in Array(stack_value):
				var state := _normalize_state(entry)
				var menu_name := _normalize_menu_name(str(state.get("id", "")))
				if not menu_name.is_empty():
					state["id"] = menu_name
					_menu_states[menu_name] = state
	if _menu_states.is_empty():
		var panel_value: Variant = data.get("top_panel", data.get("current_panel", {}))
		if typeof(panel_value) == TYPE_DICTIONARY and not Dictionary(panel_value).is_empty():
			var panel_state := _normalize_state(panel_value)
			var panel_name := _normalize_menu_name(str(panel_state.get("id", "")))
			if not panel_name.is_empty():
				panel_state["id"] = panel_name
				_menu_states[panel_name] = panel_state

func _active_menu_from_state(data: Dictionary) -> String:
	var explicit_name := _normalize_menu_name(str(data.get("active_menu", data.get("menu", data.get("menu_name", "")))))
	if not explicit_name.is_empty():
		return explicit_name
	var panel_value: Variant = data.get("top_panel", data.get("current_panel", {}))
	if typeof(panel_value) == TYPE_DICTIONARY:
		return _normalize_menu_name(str(Dictionary(panel_value).get("id", "")))
	return ""

func _menu_cursors() -> Dictionary:
	var result: Dictionary = {}
	for key in _menu_states.keys():
		result[str(key)] = int(Dictionary(_menu_states[key]).get("cursor", 0))
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

func _duplicate_entries(entries: Array) -> Array:
	var result: Array = []
	for entry in entries:
		if typeof(entry) == TYPE_DICTIONARY:
			result.append(Dictionary(entry).duplicate(true))
		else:
			result.append(entry)
	return result

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

func _pc_controller_state() -> Dictionary:
	var controller_state := _pc_default_controller_state()
	if _menu_states.has("pc_menu"):
		var state: Dictionary = Dictionary(_menu_states["pc_menu"])
		var detail: Variant = state.get("state", {})
		if typeof(detail) == TYPE_DICTIONARY:
			var source: Dictionary = Dictionary(detail)
			for key in source.keys():
				controller_state[key] = _normalize_payload(source[key])
	var wram := _runtime_wram()
	if not controller_state.has("mode") or str(controller_state.get("mode", "hub")).is_empty():
		controller_state["mode"] = str(wram.get("pc_menu_mode", "hub"))
	return controller_state

func _pokedex_controller_state() -> Dictionary:
	var controller_state := _pokedex_default_controller_state()
	if _menu_states.has("pokedex"):
		var state: Dictionary = Dictionary(_menu_states["pokedex"])
		var detail: Variant = state.get("state", {})
		if typeof(detail) == TYPE_DICTIONARY:
			var source: Dictionary = Dictionary(detail)
			for key in source.keys():
				controller_state[key] = _normalize_payload(source[key])
	return controller_state

func _pc_default_controller_state() -> Dictionary:
	return {
		"mode": "hub",
		"active_box_index": 0,
		"active_box_name": "",
		"box_cursor": 0,
		"hub_cursor": 0,
		"selected_index": 0,
		"selected_action": "",
		"selected_entry": {},
		"pending_action": {},
		"box_entries": [],
		"hub_entries": [],
	}

func _pokedex_default_controller_state() -> Dictionary:
	return {
		"page": "main",
		"search_cursor": 0,
		"search_type_1": 1,
		"search_type_2": 0,
		"search_results_cursor": 0,
		"search_results_scroll": 0,
		"search_results_count": 0,
		"selected_index": 0,
		"selected_action": "",
		"entry_detail_index": 0,
		"entry_detail_page_index": 0,
		"entry_detail_action_index": 0,
		"entry_detail_source": "search_results",
		"entry_detail_entry": {},
		"entry_detail_actions": [],
	}

func _party_controller_state(menu_name: String = "party_menu") -> Dictionary:
	var state_key := _normalize_menu_name(menu_name)
	if state_key != "party_menu" and state_key != "pokemon_menu":
		state_key = "party_menu"
	var controller_state := {
		"menu": state_key,
		"mode": "list",
		"selected_index": 0,
		"action_cursor": 0,
		"action_options": PARTY_ACTION_OPTIONS.duplicate(true),
		"selected_action": "",
		"selected_entry": {},
	}
	if not _menu_states.has(state_key):
		return controller_state
	var state: Dictionary = Dictionary(_menu_states[state_key])
	var detail: Variant = state.get("state", {})
	if typeof(detail) == TYPE_DICTIONARY:
		var source: Dictionary = Dictionary(detail)
		for key in source.keys():
			controller_state[key] = _normalize_payload(source[key])
	return controller_state

func _party_commit_state(menu_name: String, controller_state: Dictionary) -> Dictionary:
	var normalized_menu := _normalize_menu_name(menu_name)
	if normalized_menu != "party_menu" and normalized_menu != "pokemon_menu":
		normalized_menu = "party_menu"
	var state := _ensure_menu_state(normalized_menu)
	var normalized := controller_state.duplicate(true)
	normalized["menu"] = normalized_menu
	if not normalized.has("action_options") or Array(normalized.get("action_options", [])).is_empty():
		normalized["action_options"] = PARTY_ACTION_OPTIONS.duplicate(true)
	normalized["selected_index"] = max(0, int(normalized.get("selected_index", 0)))
	normalized["action_cursor"] = max(0, int(normalized.get("action_cursor", 0)))
	state["state"] = normalized
	state["entries"] = _build_party_entries(normalized_menu)
	var cursor := int(normalized.get("selected_index", 0))
	if str(normalized.get("mode", "list")) == "action":
		cursor = int(normalized.get("action_cursor", 0))
	state["cursor"] = _clamp_panel_cursor({"entries": state["entries"]}, cursor)
	if str(normalized.get("mode", "list")) == "list":
		normalized["selected_entry"] = _selection_for(state, state["cursor"])
	elif Dictionary(normalized.get("selected_entry", {})).is_empty():
		normalized["selected_entry"] = _selection_for(state, state["cursor"])
	state["id"] = normalized_menu
	state["title"] = _title_for_menu(normalized_menu)
	state["kind"] = "menu"
	state["cancelable"] = true
	state["wrap"] = true
	state["locked"] = false
	_menu_states[normalized_menu] = state
	if _active_menu == normalized_menu:
		menu_changed.emit(_active_menu, get_top_panel())
		state_changed.emit(get_state())
	return state

func _options_controller_state() -> Dictionary:
	var controller_state := {
		"cursor": 0,
		"selected_field": str(OPTIONS_FIELD_ORDER[0]),
		"selected_value": null,
		"values": {},
	}
	if not _menu_states.has("options_menu"):
		return controller_state
	var state: Dictionary = Dictionary(_menu_states["options_menu"])
	var detail: Variant = state.get("state", {})
	if typeof(detail) == TYPE_DICTIONARY:
		var source: Dictionary = Dictionary(detail)
		for key in source.keys():
			controller_state[key] = _normalize_payload(source[key])
	if typeof(controller_state.get("values", {})) == TYPE_DICTIONARY:
		controller_state["values"] = Dictionary(controller_state.get("values", {})).duplicate(true)
	else:
		controller_state["values"] = {}
	var cursor := clampi(int(controller_state.get("cursor", 0)), 0, OPTIONS_FIELD_ORDER.size())
	controller_state["cursor"] = cursor
	controller_state["selected_field"] = _options_field_for_cursor(cursor)
	controller_state["selected_value"] = _options_value_for_field(controller_state["selected_field"], Dictionary(_runtime_sram().get("options", {})).get(controller_state["selected_field"], null))
	return controller_state

func _options_field_for_cursor(cursor: int) -> String:
	if cursor >= 0 and cursor < OPTIONS_FIELD_ORDER.size():
		return str(OPTIONS_FIELD_ORDER[cursor])
	return "cancel"

func _options_value_label(field: String, value: Variant) -> String:
	match field:
		"text_speed":
			return str(value if value != null else "MID").to_upper()
		"battle_scene", "menu_account":
			return "ON" if bool(value) else "OFF"
		"battle_style":
			return str(value if value != null else "SHIFT").to_upper()
		"sound":
			return str(value if value != null else "STEREO").to_upper()
		"print_option":
			return str(value if value != null else "NORMAL").to_upper()
		"frame":
			return str(int(value) if value != null else 1)
		_:
			return str(value)

func _options_value_for_field(field: String, value: Variant) -> Variant:
	match field:
		"text_speed":
			return str(value if value != null else "mid").to_lower()
		"battle_scene", "menu_account":
			return bool(value) if value != null else true
		"battle_style":
			return str(value if value != null else "shift").to_lower()
		"sound":
			return str(value if value != null else "stereo").to_lower()
		"print_option":
			return str(value if value != null else "normal").to_lower()
		"frame":
			return int(value) if value != null else 1
		_:
			return value

func _options_next_value(field: String, current: Variant, step: int) -> Variant:
	var values: Array = Array(OPTIONS_FIELD_VALUES.get(field, []))
	if values.is_empty():
		return current
	var current_label := _options_value_label(field, current)
	var index := values.find(current_label)
	if index < 0:
		index = 0
	index = posmod(index + step, values.size())
	var next_label := str(values[index])
	match field:
		"battle_scene", "menu_account":
			return next_label == "ON"
		"frame":
			return clampi(int(next_label), 1, 8)
		_:
			return next_label.to_lower()

func _options_commit_state(controller_state: Dictionary) -> Dictionary:
	var state := _ensure_menu_state("options_menu")
	var normalized := controller_state.duplicate(true)
	var options := Dictionary(_runtime_sram().get("options", {})).duplicate(true)
	var field := str(normalized.get("selected_field", _options_field_for_cursor(int(normalized.get("cursor", 0)))))
	if field != "cancel":
		options[field] = _options_value_for_field(field, normalized.get("selected_value", options.get(field, null)))
	normalized["selected_value"] = options.get(field, null)
	normalized["values"] = {
		"cursor": int(normalized.get("cursor", 0)),
		"selected_field": field,
		"selected_value": normalized.get("selected_value", null),
	}
	var sram := _runtime_sram().duplicate(true)
	sram["options"] = options
	_runtime_state["sram"] = sram
	state["state"] = normalized
	state["entries"] = _build_options_entries()
	state["cursor"] = _clamp_panel_cursor({"entries": state["entries"]}, int(normalized.get("cursor", 0)))
	state["id"] = "options_menu"
	state["title"] = _title_for_menu("options_menu")
	state["kind"] = "menu"
	state["cancelable"] = true
	state["wrap"] = true
	state["locked"] = false
	_menu_states["options_menu"] = state
	if _active_menu == "options_menu":
		menu_changed.emit(_active_menu, get_top_panel())
		state_changed.emit(get_state())
	return state

func _pokedex_search_type_label(type_index: int) -> String:
	var clamped: int = clampi(type_index, 0, POKEDEX_SEARCH_TYPE_SEQUENCE.size() - 1)
	var label: String = str(POKEDEX_SEARCH_TYPE_SEQUENCE[clamped])
	match label:
		"PSYCHIC_TYPE":
			return "PSYCHIC"
		"NONE":
			return "----"
		_:
			return label.replace("_TYPE", "").replace("_", " ")

func _pokedex_search_type_from_index(type_index: int) -> String:
	var clamped: int = clampi(type_index, 0, POKEDEX_SEARCH_TYPE_SEQUENCE.size() - 1)
	return str(POKEDEX_SEARCH_TYPE_SEQUENCE[clamped])

func _pokedex_search_entries(state: Dictionary) -> Array:
	var search_cursor: int = int(state.get("search_cursor", 0))
	var type1: String = _pokedex_search_type_label(int(state.get("search_type_1", 1)))
	var type2: String = _pokedex_search_type_label(int(state.get("search_type_2", 0)))
	return [
		{"id": "search_type_1", "label": "TYPE 1: %s" % type1, "payload": {"action": "type_1"}},
		{"id": "search_type_2", "label": "TYPE 2: %s" % type2, "payload": {"action": "type_2"}},
		{"id": "search_begin", "label": "> BEGIN SEARCH!!" if search_cursor == 2 else "BEGIN SEARCH!!", "payload": {"action": "begin"}},
		{"id": "search_cancel", "label": "> CANCEL" if search_cursor == 3 else "CANCEL", "payload": {"action": "cancel"}},
	]

func _pokedex_search_result_entries(state: Dictionary) -> Array:
	var results: Array = _pokedex_search_results(state)
	if results.is_empty():
		return [
			{"id": "pokedex_no_match", "label": "NO MATCHES", "payload": {"action": "empty"}},
		]
	var cursor: int = int(state.get("search_results_cursor", 0))
	var entries: Array = []
	for index in range(results.size()):
		var result: Dictionary = Dictionary(results[index])
		var label := "%03d %s" % [int(result.get("pokedex_number", 0)), str(result.get("species_id", "POKEMON"))]
		if index == cursor:
			label = "> %s" % label
		entries.append({
			"id": "dex_result_%d" % int(result.get("pokedex_number", index)),
			"label": label,
			"payload": result,
		})
	return entries

func _pokedex_entry_detail_entries(state: Dictionary) -> Array:
	var entry_detail: Dictionary = _pokedex_entry_detail(state)
	var page_index: int = int(entry_detail.get("entry_page_index", 0))
	var action_index: int = int(entry_detail.get("entry_action_index", 0))
	var pages_count: int = maxi(1, int(entry_detail.get("entry_pages_count", 1)))
	var actions: Array = Array(entry_detail.get("entry_actions", ["PAGE", "AREA", "CRY", "PRNT"]))
	if actions.is_empty():
		actions = ["PAGE", "AREA", "CRY", "PRNT"]
	var entries: Array = []
	entries.append({
		"id": "dex_entry_summary",
		"label": "%03d %s" % [int(entry_detail.get("entry_number", 0)), str(entry_detail.get("entry_species_id", "POKEMON"))],
		"payload": {
			"action": "summary",
			"intent": "pokedex_entry",
			"entry": entry_detail.duplicate(true),
		},
	})
	entries.append({
		"id": "dex_entry_page",
		"label": "PAGE %d/%d" % [page_index + 1, pages_count],
		"payload": {
			"action": "page",
			"intent": "pokedex_entry",
			"entry_page_index": page_index,
			"entry_pages_count": pages_count,
			"entry": entry_detail.duplicate(true),
		},
	})
	for index in range(actions.size()):
		var action_name: String = str(actions[index])
		var label: String = action_name
		if index == action_index:
			label = "> %s" % label
		entries.append({
			"id": "dex_entry_%s" % action_name.to_lower(),
			"label": label,
			"payload": {
				"action": action_name.to_lower(),
				"intent": "pokedex_entry",
				"entry_page_index": page_index,
				"entry_action_index": index,
				"entry": entry_detail.duplicate(true),
			},
		})
	return entries

func _build_pokedex_entry_detail_entries(state: Dictionary) -> Array:
	return _pokedex_entry_detail_entries(state)

func _pokedex_entry_detail(controller_state: Dictionary) -> Dictionary:
	var entry: Dictionary = _pokedex_entry_detail_entry(controller_state)
	var pages: Array = _pokedex_entry_detail_pages(entry)
	var page_index: int = clampi(int(controller_state.get("entry_detail_page_index", 0)), 0, maxi(0, pages.size() - 1))
	var action_index: int = clampi(int(controller_state.get("entry_detail_action_index", 0)), 0, 3)
	var source: String = str(controller_state.get("entry_detail_source", "search_results"))
	var source_index: int = int(controller_state.get("entry_detail_index", 0))
	return {
		"entry_number": int(entry.get("pokedex_number", 0)),
		"entry_species_id": str(entry.get("species_id", "")),
		"entry_seen": bool(entry.get("seen", false)),
		"entry_owned": bool(entry.get("owned", false)),
		"entry_type1": str(entry.get("type1", "")),
		"entry_type2": str(entry.get("type2", "")),
		"entry": entry.duplicate(true),
		"entry_page_index": page_index,
		"entry_pages_count": pages.size(),
		"entry_actions": ["PAGE", "AREA", "CRY", "PRNT"],
		"entry_action_index": action_index,
		"entry_source": source,
		"entry_index": source_index,
		"entry_page_text": pages[page_index] if not pages.is_empty() else "",
	}

func _pokedex_entry_detail_entry(controller_state: Dictionary) -> Dictionary:
	var entry: Dictionary = Dictionary(controller_state.get("entry_detail_entry", {}))
	if not entry.is_empty():
		return entry
	var source: String = str(controller_state.get("entry_detail_source", "search_results"))
	var source_index: int = maxi(0, int(controller_state.get("entry_detail_index", 0)))
	if source == "search_results":
		var results: Array = _pokedex_search_results(controller_state)
		if source_index >= 0 and source_index < results.size():
			return Dictionary(results[source_index]).duplicate(true)
	var number := int(controller_state.get("entry_detail_number", 0))
	if number > 0:
		for species in _pokedex_species_entries():
			var entry_number := int(Dictionary(species).get("pokedex_number", 0))
			if entry_number == number:
				return Dictionary(species).duplicate(true)
	return {}

func _pokedex_entry_detail_pages(entry: Dictionary) -> Array:
	if entry.is_empty():
		return [""]
	var pages: Array = []
	var species := str(entry.get("species_id", "POKEMON"))
	var type1 := str(entry.get("type1", ""))
	var type2 := str(entry.get("type2", ""))
	pages.append("%s DATA" % species)
	pages.append("TYPE1: %s\nTYPE2: %s" % [type1, type2])
	pages.append("SEEN: %s\nOWNED: %s" % [str(bool(entry.get("seen", false))).to_upper(), str(bool(entry.get("owned", false))).to_upper()])
	return pages

func _pokedex_search_results(state: Dictionary) -> Array:
	var species_entries: Array = _pokedex_species_entries()
	if species_entries.is_empty():
		return []
	var seen_flags: Dictionary = _pokedex_seen_flags()
	var owned_flags: Dictionary = _pokedex_owned_flags()
	var type1: String = _pokedex_search_type_from_index(int(state.get("search_type_1", 1)))
	var type2: String = _pokedex_search_type_from_index(int(state.get("search_type_2", 0)))
	var results: Array = []
	for entry in species_entries:
		var species: Dictionary = Dictionary(entry)
		var pokedex_number: int = int(species.get("pokedex_number", species.get("int_id", 0)))
		var species_id: String = str(species.get("species_id", species.get("id", "")))
		if pokedex_number <= 0 or species_id.is_empty():
			continue
		if not bool(owned_flags.get(pokedex_number, false)):
			continue
		if not _pokedex_species_matches_search(species, type1, type2):
			continue
		results.append({
			"pokedex_number": pokedex_number,
			"species_id": species_id,
			"seen": bool(seen_flags.get(pokedex_number, false)),
			"owned": true,
			"type1": str(species.get("type1", "")),
			"type2": str(species.get("type2", "")),
		})
	return results

func _pokedex_species_matches_search(species: Dictionary, type1: String, type2: String) -> bool:
	if type1 == "NONE" and type2 == "NONE":
		return false
	if type1 == "NONE":
		return _pokedex_species_matches_type(species, type2)
	if type2 == "NONE":
		return _pokedex_species_matches_type(species, type1)
	return _pokedex_species_matches_type(species, type1) and _pokedex_species_matches_type(species, type2)

func _pokedex_species_matches_type(species: Dictionary, type_name: String) -> bool:
	if type_name == "NONE":
		return true
	var species_type1: String = _pokedex_normalize_type_name(str(species.get("type1", "")))
	var species_type2: String = _pokedex_normalize_type_name(str(species.get("type2", "")))
	var target: String = _pokedex_normalize_type_name(type_name)
	if target == "UNKNOWN":
		return species_type1 == "UNKNOWN" or species_type2 == "UNKNOWN"
	return species_type1 == target or species_type2 == target

func _pokedex_normalize_type_name(value: String) -> String:
	var normalized := value.strip_edges().to_upper()
	match normalized:
		"PSYCHIC":
			return "PSYCHIC_TYPE"
		"CURSE":
			return "CURSE_TYPE"
		"NONE", "UNKNOWN":
			return normalized
		_:
			return normalized.replace(" ", "_")

func _pokedex_species_entries() -> Array:
	var raw: Variant = _runtime_state.get("pokemon_data", [])
	if typeof(raw) != TYPE_ARRAY:
		return []
	var entries: Array = []
	for entry in Array(raw):
		if typeof(entry) != TYPE_DICTIONARY:
			continue
		var species: Dictionary = Dictionary(entry)
		var int_id: int = int(species.get("int_id", species.get("id", 0)))
		var species_id: String = str(species.get("id", species.get("name", "")))
		if int_id <= 0 or species_id.is_empty():
			continue
		entries.append({
			"pokedex_number": int_id,
			"species_id": species_id,
			"type1": str(species.get("type1", "")),
			"type2": str(species.get("type2", "")),
		})
	return entries

func _pokedex_seen_flags() -> Dictionary:
	return _pokedex_flag_map(_runtime_sram().get("pokedex_seen", []))

func _pokedex_owned_flags() -> Dictionary:
	return _pokedex_flag_map(_runtime_sram().get("pokedex_owned", []))

func _pokedex_flag_map(value: Variant) -> Dictionary:
	var result: Dictionary = {}
	if typeof(value) == TYPE_ARRAY:
		var bytes: Array = Array(value)
		for byte_index in range(bytes.size()):
			var number := int(bytes[byte_index])
			for bit in range(8):
				if number & (1 << bit):
					result[byte_index * 8 + bit + 1] = true
	elif typeof(value) == TYPE_DICTIONARY:
		var dict_value: Dictionary = Dictionary(value)
		for key in dict_value.keys():
			if bool(dict_value.get(key, false)):
				result[int(key)] = true
	return result

func _build_pokedex_search_result_entries(state: Dictionary) -> Array:
	return _pokedex_search_result_entries(state)

func _handle_pokedex_input(state: Dictionary, frame_input: Dictionary) -> Dictionary:
	var result := {
		"consumed": false,
		"action": "",
		"top_panel": get_top_panel(),
		"selection": {},
		"depth": 1 if is_active() else 0,
		"active_menu": _active_menu,
		"input_locked": _input_locked,
		"can_accept_input": can_accept_input(),
	}
	var pressed: Dictionary = Dictionary(frame_input.get("pressed", {}))
	var controller_state := _pokedex_controller_state()
	var page := str(controller_state.get("page", "main"))
	if page == "main":
		return _handle_pokedex_main_input(state, pressed, result)
	if page == "search":
		return _handle_pokedex_search_input(state, pressed, result)
	if page == "search_results":
		return _handle_pokedex_results_input(state, pressed, result)
	if page == "entry_detail":
		return _handle_pokedex_entry_input(state, pressed, result)
	return result

func _handle_pokedex_main_input(state: Dictionary, pressed: Dictionary, result: Dictionary) -> Dictionary:
	if bool(pressed.get("up", false)):
		_move_cursor(1, -1)
		result["action"] = "move_up"
		result["consumed"] = true
		result["top_panel"] = get_top_panel()
		return result
	if bool(pressed.get("down", false)):
		_move_cursor(1, 1)
		result["action"] = "move_down"
		result["consumed"] = true
		result["top_panel"] = get_top_panel()
		return result
	if _has_pressed_in_group(pressed, CONFIRM_BUTTONS):
		var selection := _selection_for_top()
		var payload: Dictionary = Dictionary(selection.get("payload", {}))
		var action := str(payload.get("action", ""))
		if action == "search":
			_set_pokedex_page("search")
			result["action"] = "open_search"
			result["consumed"] = true
			result["top_panel"] = get_top_panel()
			return result
		if action == "options":
			result["action"] = "open_options"
			result["consumed"] = true
			result["selection"] = selection
			return result
		if action == "unown":
			result["action"] = "open_unown"
			result["consumed"] = true
			result["selection"] = selection
			return result
		result["action"] = "confirm"
		result["consumed"] = true
		result["selection"] = selection
		return result
	if _has_pressed_in_group(pressed, CANCEL_BUTTONS):
		deactivate_menu()
		result["action"] = "cancel"
		result["consumed"] = true
		result["top_panel"] = {}
		return result
	return result

func _handle_pokedex_search_input(_state: Dictionary, pressed: Dictionary, result: Dictionary) -> Dictionary:
	var controller_state := _pokedex_controller_state()
	var cursor := int(controller_state.get("search_cursor", 0))
	if _has_pressed_in_group(pressed, CANCEL_BUTTONS) or bool(pressed.get("start", false)):
		_set_pokedex_page("main")
		result["action"] = "cancel"
		result["consumed"] = true
		result["top_panel"] = get_top_panel()
		return result
	if bool(pressed.get("up", false)):
		cursor = max(0, cursor - 1)
		_set_pokedex_search_cursor(cursor)
		result["action"] = "move_up"
		result["consumed"] = true
		result["top_panel"] = get_top_panel()
		return result
	if bool(pressed.get("down", false)):
		cursor = min(3, cursor + 1)
		_set_pokedex_search_cursor(cursor)
		result["action"] = "move_down"
		result["consumed"] = true
		result["top_panel"] = get_top_panel()
		return result
	if bool(pressed.get("left", false)) or bool(pressed.get("right", false)):
		if cursor == 0 or cursor == 1:
			var delta := -1 if bool(pressed.get("left", false)) else 1
			_step_pokedex_search_type(cursor, delta)
			result["action"] = "adjust_type"
			result["consumed"] = true
			result["top_panel"] = get_top_panel()
			return result
	if _has_pressed_in_group(pressed, CONFIRM_BUTTONS):
		if cursor == 2:
			_start_pokedex_search()
			result["action"] = "begin_search"
			result["consumed"] = true
			result["top_panel"] = get_top_panel()
			return result
		if cursor == 3:
			_set_pokedex_page("main")
			result["action"] = "cancel"
			result["consumed"] = true
			result["top_panel"] = get_top_panel()
			return result
		result["action"] = "confirm"
		result["consumed"] = true
		return result
	if _has_any_pressed_button(pressed):
		result["consumed"] = true
	return result

func _handle_pokedex_results_input(_state: Dictionary, pressed: Dictionary, result: Dictionary) -> Dictionary:
	var controller_state := _pokedex_controller_state()
	var total := _pokedex_search_results(controller_state).size()
	if total <= 0:
		if _has_pressed_in_group(pressed, CANCEL_BUTTONS) or _has_pressed_in_group(pressed, CONFIRM_BUTTONS):
			_set_pokedex_page("search")
			result["action"] = "back"
			result["consumed"] = true
			result["top_panel"] = get_top_panel()
			return result
		if _has_any_pressed_button(pressed):
			result["consumed"] = true
		return result
	var cursor := int(controller_state.get("search_results_cursor", 0))
	if bool(pressed.get("up", false)):
		cursor = max(0, cursor - 1)
		_set_pokedex_search_results_cursor(cursor)
		result["action"] = "move_up"
		result["consumed"] = true
		result["top_panel"] = get_top_panel()
		return result
	if bool(pressed.get("down", false)):
		cursor = min(total - 1, cursor + 1)
		_set_pokedex_search_results_cursor(cursor)
		result["action"] = "move_down"
		result["consumed"] = true
		result["top_panel"] = get_top_panel()
		return result
	if _has_pressed_in_group(pressed, CANCEL_BUTTONS):
		_set_pokedex_page("search")
		result["action"] = "cancel"
		result["consumed"] = true
		result["top_panel"] = get_top_panel()
		return result
	if _has_pressed_in_group(pressed, CONFIRM_BUTTONS):
		var selection := _selection_for_top()
		_set_pokedex_entry_detail(selection)
		result["action"] = "open_entry"
		result["selection"] = {
			"id": str(selection.get("id", "")),
			"label": str(selection.get("label", "")),
			"intent": "pokedex_entry",
			"action": "open_entry",
			"payload": Dictionary(selection.get("payload", {})).duplicate(true),
		}
		result["consumed"] = true
		result["top_panel"] = get_top_panel()
		return result
	if _has_any_pressed_button(pressed):
		result["consumed"] = true
	return result

func _handle_pokedex_entry_input(_state: Dictionary, pressed: Dictionary, result: Dictionary) -> Dictionary:
	var controller_state := _pokedex_controller_state()
	if _has_pressed_in_group(pressed, CANCEL_BUTTONS):
		var return_page := str(controller_state.get("entry_detail_source", "search_results"))
		if return_page.is_empty():
			return_page = "search_results"
		_set_pokedex_page(return_page)
		result["action"] = "back"
		result["consumed"] = true
		result["top_panel"] = get_top_panel()
		return result
	if bool(pressed.get("left", false)):
		_step_pokedex_entry_action(-1)
		result["action"] = "move_left"
		result["consumed"] = true
		result["top_panel"] = get_top_panel()
		return result
	if bool(pressed.get("right", false)):
		_step_pokedex_entry_action(1)
		result["action"] = "move_right"
		result["consumed"] = true
		result["top_panel"] = get_top_panel()
		return result
	if _has_pressed_in_group(pressed, CONFIRM_BUTTONS):
		var entry_detail := _pokedex_entry_detail(controller_state)
		var entry_actions: Array = Array(entry_detail.get("entry_actions", ["PAGE"]))
		if entry_actions.is_empty():
			entry_actions = ["PAGE"]
		var entry_action_index := clampi(int(entry_detail.get("entry_action_index", 0)), 0, max(0, entry_actions.size() - 1))
		var action := str(entry_actions[entry_action_index])
		if action == "PAGE":
			_step_pokedex_entry_page(1)
			result["action"] = "page"
			result["consumed"] = true
			result["top_panel"] = get_top_panel()
			return result
		var selection := {
			"payload": {
				"action": action.to_lower(),
				"entry": Dictionary(entry_detail.get("entry", {})).duplicate(true),
			},
		}
		_set_pokedex_entry_detail(selection)
		result["action"] = action.to_lower()
		result["selection"] = {
			"intent": "pokedex_entry",
			"action": action.to_lower(),
			"payload": entry_detail.duplicate(true),
		}
		result["consumed"] = true
		result["top_panel"] = get_top_panel()
		return result
	if _has_any_pressed_button(pressed):
		result["consumed"] = true
	return result

func _set_pokedex_page(page: String) -> void:
	var normalized := page
	if normalized != "main" and normalized != "search" and normalized != "search_results" and normalized != "entry_detail":
		normalized = "main"
	var state := _ensure_menu_state("pokedex")
	var controller_state := _pokedex_controller_state()
	controller_state["page"] = normalized
	if normalized == "main":
		state["entries"] = _normalize_entries(DEFAULT_MENU_ENTRIES.get("pokedex", []))
		state["cursor"] = _clamp_panel_cursor(state, int(state.get("cursor", 0)))
	elif normalized == "search":
		controller_state["search_cursor"] = clamp(int(controller_state.get("search_cursor", 0)), 0, 3)
		state["entries"] = _build_pokedex_search_entries(controller_state)
		state["cursor"] = _clamp_panel_cursor({"entries": state["entries"]}, int(controller_state.get("search_cursor", 0)))
	elif normalized == "entry_detail":
		state["entries"] = _build_pokedex_entry_detail_entries(controller_state)
		state["cursor"] = _clamp_panel_cursor({"entries": state["entries"]}, int(controller_state.get("entry_detail_action_index", 0)) + 2)
	else:
		var results := _pokedex_search_results(controller_state)
		state["entries"] = _build_pokedex_search_result_entries(controller_state)
		var result_cursor := int(controller_state.get("search_results_cursor", 0))
		if results.is_empty():
			result_cursor = 0
		else:
			result_cursor = clamp(result_cursor, 0, results.size() - 1)
		state["cursor"] = _clamp_panel_cursor({"entries": state["entries"]}, result_cursor)
	controller_state["search_results_count"] = _pokedex_search_results(controller_state).size()
	state["state"] = controller_state
	state["title"] = _title_for_menu("pokedex")
	state["kind"] = "submenu" if normalized == "search_results" or normalized == "entry_detail" else "menu"
	_menu_states["pokedex"] = state
	if _active_menu == "pokedex":
		menu_changed.emit(_active_menu, get_top_panel())
		state_changed.emit(get_state())

func _set_pokedex_entry_detail(selection: Dictionary) -> void:
	var state := _ensure_menu_state("pokedex")
	var controller_state := _pokedex_controller_state()
	var payload: Dictionary = Dictionary(selection.get("payload", {}))
	var entry: Dictionary = Dictionary(payload.get("entry", {})).duplicate(true)
	if entry.is_empty():
		entry = Dictionary(payload).duplicate(true)
	var previous_page := str(controller_state.get("page", "search_results"))
	controller_state["page"] = "entry_detail"
	controller_state["entry_detail_source"] = previous_page if previous_page == "search_results" else "search_results"
	controller_state["entry_detail_index"] = int(controller_state.get("search_results_cursor", controller_state.get("entry_detail_index", 0)))
	controller_state["entry_detail_entry"] = entry
	controller_state["entry_detail_number"] = int(entry.get("pokedex_number", entry.get("int_id", 0)))
	controller_state["entry_detail_page_index"] = clampi(int(controller_state.get("entry_detail_page_index", 0)), 0, max(0, _pokedex_entry_detail_pages(entry).size() - 1))
	controller_state["entry_detail_action_index"] = clampi(int(controller_state.get("entry_detail_action_index", 0)), 0, 3)
	controller_state["entry_detail_actions"] = ["PAGE", "AREA", "CRY", "PRNT"]
	controller_state["selected_action"] = str(payload.get("action", controller_state.get("selected_action", "")))
	state["state"] = controller_state
	state["entries"] = _build_pokedex_entry_detail_entries(controller_state)
	state["cursor"] = _clamp_panel_cursor({"entries": state["entries"]}, int(controller_state.get("entry_detail_action_index", 0)) + 2)
	state["kind"] = "submenu"
	_menu_states["pokedex"] = state
	if _active_menu == "pokedex":
		menu_changed.emit(_active_menu, get_top_panel())
		state_changed.emit(get_state())

func _step_pokedex_entry_action(delta: int) -> void:
	var state := _ensure_menu_state("pokedex")
	var controller_state := _pokedex_controller_state()
	var actions: Array = Array(controller_state.get("entry_detail_actions", ["PAGE", "AREA", "CRY", "PRNT"]))
	if actions.is_empty():
		actions = ["PAGE", "AREA", "CRY", "PRNT"]
	var action_index := clampi(int(controller_state.get("entry_detail_action_index", 0)) + delta, 0, max(0, actions.size() - 1))
	controller_state["entry_detail_action_index"] = action_index
	controller_state["entry_detail_actions"] = actions.duplicate(true)
	controller_state["page"] = "entry_detail"
	state["state"] = controller_state
	state["entries"] = _build_pokedex_entry_detail_entries(controller_state)
	state["cursor"] = _clamp_panel_cursor({"entries": state["entries"]}, action_index + 2)
	state["kind"] = "submenu"
	_menu_states["pokedex"] = state
	if _active_menu == "pokedex":
		menu_changed.emit(_active_menu, get_top_panel())
		state_changed.emit(get_state())

func _step_pokedex_entry_page(delta: int) -> void:
	var state := _ensure_menu_state("pokedex")
	var controller_state := _pokedex_controller_state()
	var entry := _pokedex_entry_detail_entry(controller_state)
	var pages := _pokedex_entry_detail_pages(entry)
	if pages.is_empty():
		_set_pokedex_page("search_results")
		return
	var current_page_index := int(controller_state.get("entry_detail_page_index", 0))
	var page_index := clampi(current_page_index + delta, 0, max(0, pages.size() - 1))
	if delta > 0 and current_page_index >= pages.size() - 1:
		_set_pokedex_page(str(controller_state.get("entry_detail_source", "search_results")))
		return
	controller_state["entry_detail_page_index"] = page_index
	controller_state["page"] = "entry_detail"
	state["state"] = controller_state
	state["entries"] = _build_pokedex_entry_detail_entries(controller_state)
	state["cursor"] = _clamp_panel_cursor({"entries": state["entries"]}, page_index + 1)
	state["kind"] = "submenu"
	_menu_states["pokedex"] = state
	if _active_menu == "pokedex":
		menu_changed.emit(_active_menu, get_top_panel())
		state_changed.emit(get_state())

func _set_pokedex_search_cursor(cursor: int) -> void:
	var state := _ensure_menu_state("pokedex")
	var controller_state := _pokedex_controller_state()
	controller_state["page"] = "search"
	controller_state["search_cursor"] = clamp(cursor, 0, 3)
	state["state"] = controller_state
	state["entries"] = _build_pokedex_search_entries(controller_state)
	state["cursor"] = _clamp_panel_cursor({"entries": state["entries"]}, controller_state["search_cursor"])
	state["kind"] = "menu"
	_menu_states["pokedex"] = state
	if _active_menu == "pokedex":
		menu_changed.emit(_active_menu, get_top_panel())
		state_changed.emit(get_state())

func _set_pokedex_search_results_cursor(cursor: int) -> void:
	var state := _ensure_menu_state("pokedex")
	var controller_state := _pokedex_controller_state()
	controller_state["page"] = "search_results"
	controller_state["search_results_cursor"] = max(0, cursor)
	var results := _pokedex_search_results(controller_state)
	controller_state["search_results_count"] = results.size()
	var entries := _build_pokedex_search_result_entries(controller_state)
	state["state"] = controller_state
	state["entries"] = entries
	state["cursor"] = _clamp_panel_cursor({"entries": entries}, controller_state["search_results_cursor"])
	state["kind"] = "submenu"
	_menu_states["pokedex"] = state
	if _active_menu == "pokedex":
		menu_changed.emit(_active_menu, get_top_panel())
		state_changed.emit(get_state())

func _step_pokedex_search_type(index: int, delta: int) -> void:
	var state := _ensure_menu_state("pokedex")
	var controller_state := _pokedex_controller_state()
	if index == 0:
		var value := int(controller_state.get("search_type_1", 1))
		value += delta
		if value < 1:
			value = POKEDEX_SEARCH_TYPE_SEQUENCE.size() - 1
		elif value >= POKEDEX_SEARCH_TYPE_SEQUENCE.size():
			value = 1
		controller_state["search_type_1"] = value
	else:
		var value := int(controller_state.get("search_type_2", 0))
		value += delta
		if value < 0:
			value = POKEDEX_SEARCH_TYPE_SEQUENCE.size() - 1
		elif value >= POKEDEX_SEARCH_TYPE_SEQUENCE.size():
			value = 0
		controller_state["search_type_2"] = value
	controller_state["page"] = "search"
	state["state"] = controller_state
	state["entries"] = _build_pokedex_search_entries(controller_state)
	state["cursor"] = _clamp_panel_cursor({"entries": state["entries"]}, int(controller_state.get("search_cursor", 0)))
	state["kind"] = "menu"
	_menu_states["pokedex"] = state
	if _active_menu == "pokedex":
		menu_changed.emit(_active_menu, get_top_panel())
		state_changed.emit(get_state())

func _start_pokedex_search() -> void:
	var state := _ensure_menu_state("pokedex")
	var controller_state := _pokedex_controller_state()
	controller_state["page"] = "search_results"
	controller_state["search_results_cursor"] = 0
	controller_state["search_results_scroll"] = 0
	var results := _pokedex_search_results(controller_state)
	controller_state["search_results_count"] = results.size()
	var entries := _build_pokedex_search_result_entries(controller_state)
	state["state"] = controller_state
	state["entries"] = entries
	state["cursor"] = _clamp_panel_cursor({"entries": entries}, 0)
	state["kind"] = "submenu"
	_menu_states["pokedex"] = state
	if _active_menu == "pokedex":
		menu_changed.emit(_active_menu, get_top_panel())
		state_changed.emit(get_state())

func _trainer_card_controller_state() -> Dictionary:
	if not _menu_states.has("trainer_card"):
		return {}
	var state: Dictionary = Dictionary(_menu_states["trainer_card"])
	var detail: Variant = state.get("state", {})
	if typeof(detail) == TYPE_DICTIONARY:
		return Dictionary(detail)
	return {}

func _trainer_card_page_from_cursor(cursor: int) -> String:
	match clamp(cursor, 0, 2):
		0:
			return "info"
		1:
			return "johto_badges"
		_:
			return "kanto_badges"

func _trainer_card_cursor_from_page(page: String) -> int:
	match page:
		"johto_badges":
			return 1
		"kanto_badges":
			return 2
		_:
			return 0

func _handle_trainer_card_input(state: Dictionary, frame_input: Dictionary) -> Dictionary:
	var result := {
		"consumed": false,
		"action": "",
		"top_panel": get_top_panel(),
		"selection": {},
		"depth": 1 if is_active() else 0,
		"active_menu": _active_menu,
		"input_locked": _input_locked,
		"can_accept_input": can_accept_input(),
	}
	var pressed: Dictionary = Dictionary(frame_input.get("pressed", {}))
	var controller_state := _trainer_card_controller_state()
	var page := str(controller_state.get("page", _trainer_card_page_from_cursor(int(state.get("cursor", 0)))))
	var next_page := page
	if _has_pressed_in_group(pressed, CANCEL_BUTTONS):
		deactivate_menu()
		result["action"] = "cancel"
		result["consumed"] = true
		result["top_panel"] = {}
		return result
	if bool(pressed.get("up", false)) or bool(pressed.get("down", false)):
		# No vertical movement on the trainer card pages.
		result["consumed"] = _has_any_pressed_button(pressed)
		return result
	if bool(pressed.get("left", false)):
		if page == "johto_badges":
			next_page = "info"
		elif page == "kanto_badges":
			next_page = "johto_badges"
		if next_page != page:
			_set_trainer_card_page(next_page)
			result["action"] = "move_left"
			result["consumed"] = true
			result["top_panel"] = get_top_panel()
			return result
		result["consumed"] = true
		return result
	if bool(pressed.get("right", false)):
		if page == "info":
			next_page = "johto_badges"
		elif page == "johto_badges":
			if _trainer_card_has_kanto_badges():
				next_page = "kanto_badges"
		else:
			next_page = "info"
		if next_page != page:
			_set_trainer_card_page(next_page)
			result["action"] = "move_right"
			result["consumed"] = true
			result["top_panel"] = get_top_panel()
			return result
		result["consumed"] = true
		return result
	if _has_pressed_in_group(pressed, CONFIRM_BUTTONS):
		if page == "info":
			_set_trainer_card_page("johto_badges")
			result["action"] = "confirm"
			result["consumed"] = true
			result["top_panel"] = get_top_panel()
			return result
		deactivate_menu()
		result["action"] = "exit"
		result["consumed"] = true
		result["top_panel"] = {}
		return result
	if _has_any_pressed_button(pressed):
		result["consumed"] = true
	return result

func _handle_options_input(_state: Dictionary, frame_input: Dictionary) -> Dictionary:
	var result := {
		"consumed": false,
		"action": "",
		"top_panel": get_top_panel(),
		"selection": {},
		"depth": 1 if is_active() else 0,
		"active_menu": _active_menu,
		"input_locked": _input_locked,
		"can_accept_input": can_accept_input(),
	}
	var pressed: Dictionary = Dictionary(frame_input.get("pressed", {}))
	var controller_state := _options_controller_state()
	var cursor := int(controller_state.get("cursor", 0))
	if _has_pressed_in_group(pressed, CANCEL_BUTTONS):
		deactivate_menu()
		result["action"] = "cancel"
		result["consumed"] = true
		result["top_panel"] = {}
		return result
	if bool(pressed.get("up", false)):
		cursor = max(0, cursor - 1)
		controller_state["cursor"] = cursor
		_options_commit_state(controller_state)
		result["action"] = "move_up"
		result["consumed"] = true
		result["top_panel"] = get_top_panel()
		return result
	if bool(pressed.get("down", false)):
		cursor = min(OPTIONS_FIELD_ORDER.size(), cursor + 1)
		controller_state["cursor"] = cursor
		_options_commit_state(controller_state)
		result["action"] = "move_down"
		result["consumed"] = true
		result["top_panel"] = get_top_panel()
		return result
	if bool(pressed.get("left", false)) or bool(pressed.get("right", false)):
		var field: String = _options_field_for_cursor(cursor)
		if field != "cancel":
			var delta: int = -1 if bool(pressed.get("left", false)) else 1
			var next_value: Variant = _options_next_value(field, Dictionary(_runtime_sram().get("options", {})).get(field, null), delta)
			controller_state["selected_field"] = field
			controller_state["selected_value"] = next_value
			_options_commit_state(controller_state)
			result["action"] = "adjust_option"
			result["consumed"] = true
			result["top_panel"] = get_top_panel()
			return result
	if _has_pressed_in_group(pressed, CONFIRM_BUTTONS):
		if cursor == OPTIONS_FIELD_ORDER.size():
			deactivate_menu()
			result["action"] = "cancel"
			result["consumed"] = true
			result["top_panel"] = {}
			return result
		var field := _options_field_for_cursor(cursor)
		controller_state["selected_field"] = field
		controller_state["selected_value"] = _options_next_value(field, Dictionary(_runtime_sram().get("options", {})).get(field, null), 0)
		_options_commit_state(controller_state)
		result["action"] = "confirm"
		result["selection"] = _selection_for_top()
		result["consumed"] = true
		result["top_panel"] = get_top_panel()
		return result
	if _has_any_pressed_button(pressed):
		result["consumed"] = true
	return result

func _handle_party_menu_input(_state: Dictionary, frame_input: Dictionary, menu_name: String) -> Dictionary:
	var result := {
		"consumed": false,
		"action": "",
		"top_panel": get_top_panel(),
		"selection": {},
		"depth": 1 if is_active() else 0,
		"active_menu": _active_menu,
		"input_locked": _input_locked,
		"can_accept_input": can_accept_input(),
	}
	var pressed: Dictionary = Dictionary(frame_input.get("pressed", {}))
	var state: Dictionary = _ensure_menu_state(menu_name)
	var controller_state := _party_controller_state(menu_name)
	var mode := str(controller_state.get("mode", "list"))
	if mode == "action":
		var action_cursor := int(controller_state.get("action_cursor", 0))
		var total_actions := Array(controller_state.get("action_options", PARTY_ACTION_OPTIONS)).size()
		if _has_pressed_in_group(pressed, CANCEL_BUTTONS):
			controller_state["mode"] = "list"
			controller_state["action_cursor"] = 0
			_party_commit_state(menu_name, controller_state)
			result["action"] = "cancel"
			result["consumed"] = true
			result["top_panel"] = get_top_panel()
			return result
		if _has_pressed_in_group(pressed, UP_BUTTONS):
			action_cursor = max(0, action_cursor - 1)
			controller_state["action_cursor"] = action_cursor
			_party_commit_state(menu_name, controller_state)
			result["action"] = "move_up"
			result["consumed"] = true
			result["top_panel"] = get_top_panel()
			return result
		if _has_pressed_in_group(pressed, DOWN_BUTTONS):
			action_cursor = min(max(0, total_actions - 1), action_cursor + 1)
			controller_state["action_cursor"] = action_cursor
			_party_commit_state(menu_name, controller_state)
			result["action"] = "move_down"
			result["consumed"] = true
			result["top_panel"] = get_top_panel()
			return result
		if _has_pressed_in_group(pressed, CONFIRM_BUTTONS):
			var actions: Array = Array(controller_state.get("action_options", PARTY_ACTION_OPTIONS))
			if actions.is_empty():
				actions = PARTY_ACTION_OPTIONS.duplicate(true)
			var selected_action := str(actions[clampi(action_cursor, 0, actions.size() - 1)])
			var selected_entry := Dictionary(controller_state.get("selected_entry", {})).duplicate(true)
			controller_state["selected_action"] = selected_action
			controller_state["mode"] = "list"
			_party_commit_state(menu_name, controller_state)
			result["action"] = selected_action.to_lower()
			result["selection"] = {
				"menu": menu_name,
				"action": selected_action.to_lower(),
				"intent": "pokemon_action",
				"selected_index": int(controller_state.get("selected_index", 0)),
				"selected_action": selected_action,
				"selected_entry": selected_entry,
				"payload": Dictionary(selected_entry.get("payload", {})).duplicate(true),
			}
			result["consumed"] = true
			result["top_panel"] = get_top_panel()
			return result
		if _has_any_pressed_button(pressed):
			result["consumed"] = true
		return result
	if _has_pressed_in_group(pressed, CANCEL_BUTTONS):
		deactivate_menu()
		result["action"] = "cancel"
		result["consumed"] = true
		result["top_panel"] = {}
		return result
	if bool(pressed.get("up", false)):
		var cursor_up: int = max(0, int(state.get("cursor", 0)) - 1)
		controller_state["selected_index"] = cursor_up
		_party_commit_state(menu_name, controller_state)
		result["action"] = "move_up"
		result["consumed"] = true
		result["top_panel"] = get_top_panel()
		return result
	if bool(pressed.get("down", false)):
		var cursor_down: int = min(max(0, Array(state.get("entries", [])).size() - 1), int(state.get("cursor", 0)) + 1)
		controller_state["selected_index"] = cursor_down
		_party_commit_state(menu_name, controller_state)
		result["action"] = "move_down"
		result["consumed"] = true
		result["top_panel"] = get_top_panel()
		return result
	if _has_pressed_in_group(pressed, CONFIRM_BUTTONS):
		var selection: Dictionary = _selection_for_top()
		var payload: Dictionary = Dictionary(selection.get("payload", {}))
		if bool(payload.get("empty", false)) or str(payload.get("action", "")) == "cancel":
			result["action"] = "cancel"
			result["consumed"] = true
			result["top_panel"] = get_top_panel()
			return result
		controller_state["mode"] = "action"
		controller_state["selected_index"] = int(state.get("cursor", 0))
		controller_state["selected_entry"] = selection
		controller_state["action_cursor"] = 0
		controller_state["selected_action"] = ""
		_party_commit_state(menu_name, controller_state)
		result["action"] = "open_action_menu"
		result["selection"] = selection
		result["consumed"] = true
		result["top_panel"] = get_top_panel()
		return result
	if _has_any_pressed_button(pressed):
		result["consumed"] = true
	return result

func _set_trainer_card_page(page: String) -> void:
	var normalized := page
	if normalized != "info" and normalized != "johto_badges" and normalized != "kanto_badges":
		normalized = "info"
	var state := _ensure_menu_state("trainer_card")
	var controller_state := _trainer_card_controller_state()
	controller_state["page"] = normalized
	controller_state["has_kanto_badges"] = _trainer_card_has_kanto_badges()
	controller_state["page_index"] = _trainer_card_cursor_from_page(normalized)
	controller_state["selected_page"] = normalized
	state["state"] = controller_state
	state["cursor"] = _trainer_card_cursor_from_page(normalized)
	state["title"] = _title_for_menu("trainer_card")
	state["kind"] = "menu"
	_menu_states["trainer_card"] = state
	if _active_menu == "trainer_card":
		menu_changed.emit(_active_menu, get_top_panel())
		state_changed.emit(get_state())

func _trainer_card_has_kanto_badges() -> bool:
	var sram := _runtime_sram()
	var badges := Dictionary(sram.get("badges", {}))
	return _count_true(_normalize_bool_array(badges.get("kanto", []))) > 0

func _handle_pc_menu_input(state: Dictionary, frame_input: Dictionary) -> Dictionary:
	var result := {
		"consumed": false,
		"action": "",
		"top_panel": get_top_panel(),
		"selection": {},
		"depth": 1 if is_active() else 0,
		"active_menu": _active_menu,
		"input_locked": _input_locked,
		"can_accept_input": can_accept_input(),
	}
	var pressed: Dictionary = Dictionary(frame_input.get("pressed", {}))
	var controller_state := _pc_controller_state()
	var mode := str(controller_state.get("mode", "hub"))
	if mode == "box":
		if _has_pressed_in_group(pressed, CANCEL_BUTTONS):
			controller_state["selected_action"] = "cancel"
			_pc_commit_state(controller_state)
			_pc_return_to_pc_hub()
			result["action"] = "cancel"
			result["consumed"] = true
			result["top_panel"] = get_top_panel()
			return result
		if _has_pressed_in_group(pressed, CONFIRM_BUTTONS):
			var selection := _selection_for_top()
			var selection_payload: Dictionary = Dictionary(selection.get("payload", {}))
			controller_state["selected_action"] = "select"
			controller_state["selected_entry"] = selection
			if Dictionary(controller_state.get("pending_action", {})).is_empty() and not str(selection_payload.get("intent", "")).is_empty():
				controller_state["pending_action"] = {
					"intent": str(selection_payload.get("intent", "pc_action")),
					"action": str(selection_payload.get("action", "select")),
					"box_index": int(selection_payload.get("box_index", controller_state.get("active_box_index", 0))),
					"slot_index": int(selection_payload.get("slot_index", -1)),
					"party_slot": int(selection_payload.get("party_slot", -1)),
				}
			_pc_commit_state(controller_state)
			result["action"] = "confirm"
			result["selection"] = {
				"id": str(selection.get("id", "")),
				"label": str(selection.get("label", "")),
				"intent": str(Dictionary(controller_state.get("pending_action", {})).get("intent", "pc_action")),
				"action": str(Dictionary(controller_state.get("pending_action", {})).get("action", "select")),
				"payload": Dictionary(controller_state.get("pending_action", {})).duplicate(true),
				"entry": selection,
			}
			result["consumed"] = true
			result["top_panel"] = get_top_panel()
			return result
		return result
	if _has_pressed_in_group(pressed, CONFIRM_BUTTONS):
		var selection := _selection_for_top()
		var payload: Dictionary = Dictionary(selection.get("payload", {}))
		if bool(payload.get("empty", false)):
			controller_state["selected_action"] = "select"
			controller_state["selected_entry"] = selection
			_pc_commit_state(controller_state)
			result["action"] = "confirm"
			result["selection"] = {
				"id": str(selection.get("id", "")),
				"label": str(selection.get("label", "")),
				"intent": "pc_slot",
				"action": "select",
				"payload": Dictionary(payload).duplicate(true),
				"entry": selection,
			}
			result["consumed"] = true
			result["top_panel"] = get_top_panel()
			return result
		if payload.has("box_index"):
			controller_state["selected_action"] = "open_box"
			controller_state["selected_entry"] = selection
			_pc_commit_state(controller_state)
			_pc_enter_box_view(int(payload.get("box_index", 0)))
			result["action"] = "open_box"
			result["selection"] = selection
			result["consumed"] = true
			result["top_panel"] = get_top_panel()
			return result
		if str(payload.get("action", "")) == "see_ya":
			controller_state["selected_action"] = "see_ya"
			controller_state["selected_entry"] = selection
			_pc_commit_state(controller_state)
			deactivate_menu()
			result["action"] = "cancel"
			result["consumed"] = true
			result["top_panel"] = {}
			return result
		controller_state["selected_action"] = str(payload.get("action", "confirm"))
		controller_state["pending_action"] = {
			"intent": str(payload.get("intent", "pc_action")),
			"action": str(payload.get("action", "confirm")),
			"box_index": int(payload.get("box_index", controller_state.get("active_box_index", 0))),
			"slot_index": int(payload.get("slot_index", -1)),
			"target_box": int(payload.get("target_box", -1)),
			"target_slot": int(payload.get("target_slot", -1)),
			"party_slot": int(payload.get("party_slot", -1)),
		}
		controller_state["selected_entry"] = selection
		_pc_commit_state(controller_state)
		result["action"] = "confirm"
		result["selection"] = {
			"id": str(selection.get("id", "")),
			"label": str(selection.get("label", "")),
			"intent": str(Dictionary(controller_state.get("pending_action", {})).get("intent", "pc_action")),
			"action": str(Dictionary(controller_state.get("pending_action", {})).get("action", "confirm")),
			"payload": Dictionary(controller_state.get("pending_action", {})).duplicate(true),
			"entry": selection,
		}
		result["consumed"] = true
		result["top_panel"] = get_top_panel()
		return result
	if _has_pressed_in_group(pressed, CANCEL_BUTTONS):
		deactivate_menu()
		result["action"] = "cancel"
		result["consumed"] = true
		result["top_panel"] = {}
		return result
	return result

func _pc_enter_box_view(box_index: int) -> void:
	var state := _ensure_menu_state("pc_menu")
	var controller_state := _pc_controller_state()
	var hub_cursor := int(state.get("cursor", 0))
	var box_entries := _build_pc_box_entries(box_index)
	if box_entries.is_empty():
		return
	var box_cursor := int(controller_state.get("box_cursor", 0))
	var box_name := ""
	var boxes: Array = Array(_runtime_sram().get("pc_boxes", []))
	if box_index >= 0 and box_index < boxes.size():
		box_name = str(Dictionary(boxes[box_index]).get("name", ""))
	if not box_entries.is_empty():
		box_cursor = _clamp_panel_cursor({"entries": box_entries}, box_cursor)
	controller_state["mode"] = "box"
	controller_state["hub_cursor"] = hub_cursor
	controller_state["active_box_index"] = box_index
	controller_state["active_box_name"] = box_name
	controller_state["box_cursor"] = box_cursor
	controller_state["box_entries"] = _duplicate_entries(box_entries)
	controller_state["hub_entries"] = _duplicate_entries(_build_pc_hub_entries())
	state["state"] = controller_state
	state["entries"] = box_entries
	state["cursor"] = box_cursor
	state["title"] = box_name if not box_name.is_empty() else _title_for_menu("pc_menu")
	state["kind"] = "submenu"
	_menu_states["pc_menu"] = state
	if _active_menu == "pc_menu":
		menu_changed.emit(_active_menu, get_top_panel())
		state_changed.emit(get_state())

func _pc_commit_state(controller_state: Dictionary) -> Dictionary:
	var state := _ensure_menu_state("pc_menu")
	var normalized := controller_state.duplicate(true)
	var mode := str(normalized.get("mode", "hub"))
	normalized["mode"] = mode
	if not normalized.has("hub_entries") or Array(normalized.get("hub_entries", [])).is_empty():
		normalized["hub_entries"] = _duplicate_entries(_build_pc_hub_entries())
	if not normalized.has("box_entries") or Array(normalized.get("box_entries", [])).is_empty():
		var box_index := int(normalized.get("active_box_index", 0))
		normalized["box_entries"] = _duplicate_entries(_build_pc_box_entries(box_index))
	if mode == "box":
		state["entries"] = _duplicate_entries(Array(normalized.get("box_entries", [])))
		state["cursor"] = _clamp_panel_cursor({"entries": state["entries"]}, int(normalized.get("box_cursor", state.get("cursor", 0))))
	else:
		state["entries"] = _duplicate_entries(Array(normalized.get("hub_entries", [])))
		state["cursor"] = _clamp_panel_cursor({"entries": state["entries"]}, int(normalized.get("hub_cursor", state.get("cursor", 0))))
	if Dictionary(normalized.get("selected_entry", {})).is_empty():
		normalized["selected_entry"] = _selection_for(state, state["cursor"])
	normalized["selected_index"] = int(state.get("cursor", 0))
	state["state"] = normalized
	state["id"] = "pc_menu"
	state["title"] = _title_for_menu("pc_menu")
	state["kind"] = "menu"
	state["cancelable"] = true
	state["wrap"] = true
	state["locked"] = false
	_menu_states["pc_menu"] = state
	if _active_menu == "pc_menu":
		menu_changed.emit(_active_menu, get_top_panel())
		state_changed.emit(get_state())
	return state

func _pc_return_to_pc_hub() -> void:
	var state := _ensure_menu_state("pc_menu")
	var controller_state := _pc_controller_state()
	controller_state["mode"] = "hub"
	var hub_entries := Array(controller_state.get("hub_entries", []))
	if hub_entries.is_empty():
		hub_entries = _build_pc_hub_entries()
	var hub_cursor := int(controller_state.get("hub_cursor", state.get("cursor", 0)))
	state["state"] = controller_state
	state["entries"] = _duplicate_entries(hub_entries)
	state["cursor"] = _clamp_panel_cursor(state, hub_cursor)
	state["title"] = _title_for_menu("pc_menu")
	state["kind"] = "menu"
	_menu_states["pc_menu"] = state
	if _active_menu == "pc_menu":
		menu_changed.emit(_active_menu, get_top_panel())
		state_changed.emit(get_state())

func _has_any_pressed_button(pressed: Dictionary) -> bool:
	for button in pressed.keys():
		if bool(pressed.get(button, false)):
			return true
	return false
