extends Control
class_name UIShell

signal ui_page_changed(page_name)

const MENU_STATE_SCRIPT = preload("res://scripts/menu_state.gd")
const PAGE_TITLES := {
	"title": "TITLE",
	"intro": "INTRO",
	"oak_intro": "OAK INTRO",
	"main_menu": "MAIN MENU",
	"start_menu": "START MENU",
	"bag_menu": "BAG",
	"party_menu": "PARTY",
	"pokemon_menu": "POKEMON MENU",
	"move_menu": "MOVE MENU",
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
	"day_of_week": "DAY OF WEEK",
}

@onready var text_box: Node = get_node_or_null("TextBox")
@onready var menu_stack: Node = get_node_or_null("MenuStack")
var menu_state: Variant = null
var _title_label: Label
var _status_label: Label
var _panel_label: Label
var _page_label: Label
var _debug_label: Label
var _last_signature := ""
var ui_page: String = "title"
var page_snapshots: Dictionary = {}

func _ready() -> void:
	set_process_unhandled_input(false)
	_ensure_menu_state()
	_bind_labels()
	set_process(true)
	_sync_menu_state_from_runtime()

func _ensure_menu_state() -> Variant:
	if menu_state == null:
		menu_state = MENU_STATE_SCRIPT.new()
	return menu_state

func _bind_labels() -> void:
	_title_label = get_node_or_null("Margin/VBox/TitleLabel")
	_status_label = get_node_or_null("Margin/VBox/StatusLabel")
	_panel_label = get_node_or_null("Margin/VBox/PanelLabel")
	_page_label = get_node_or_null("Margin/VBox/PageLabel")
	_debug_label = get_node_or_null("Margin/VBox/DebugLabel")

func _process(_delta: float) -> void:
	_refresh_ui()

func reset(clear_page_snapshots: bool = true) -> void:
	_call_if_present(text_box, "reset")
	_call_if_present(menu_stack, "reset")
	_call_if_present(_ensure_menu_state(), "reset")
	if clear_page_snapshots:
		page_snapshots = {}
	ui_page = "title"
	_last_signature = ""
	_sync_menu_state_from_runtime()
	_refresh_ui(true)

func set_ui_page(page_name: String) -> void:
	var next_page := _normalize_page_name(page_name)
	if next_page == ui_page:
		ui_page = next_page
		_sync_menu_state_page(ui_page)
		_refresh_ui(true)
		return
	_store_current_page_snapshot()
	ui_page = next_page
	_restore_page_snapshot(ui_page)
	_sync_menu_state_page(ui_page)
	emit_signal("ui_page_changed", ui_page)
	_refresh_ui(true)

func set_page_name(page_name: String) -> void:
	set_ui_page(page_name)

func show_title_screen() -> void:
	set_ui_page("title")

func show_intro_screen() -> void:
	set_ui_page("intro")

func show_intro_sequence() -> void:
	set_ui_page("intro")

func show_oak_intro() -> void:
	set_ui_page("oak_intro")

func show_main_menu() -> void:
	set_ui_page("main_menu")

func show_continue_screen() -> void:
	set_ui_page("continue")

func show_delete_save_screen() -> void:
	set_ui_page("delete_save")

func show_delete_save_prompt() -> void:
	set_ui_page("delete_save")

func show_clock_reset_screen() -> void:
	set_ui_page("clock_reset")

func show_clock_reset_prompt() -> void:
	set_ui_page("clock_reset")

func show_gender_screen() -> void:
	set_ui_page("gender")

func show_gender_prompt() -> void:
	set_ui_page("gender")

func show_name_entry_screen() -> void:
	set_ui_page("name_entry")

func show_name_entry_prompt() -> void:
	set_ui_page("name_entry")

func show_start_menu() -> void:
	set_ui_page("start_menu")

func show_bag_menu() -> void:
	set_ui_page("bag_menu")

func show_pack_menu() -> void:
	set_ui_page("bag_menu")

func show_party_menu() -> void:
	set_ui_page("party_menu")

func show_pokemon_menu() -> void:
	set_ui_page("pokemon_menu")

func show_pokedex_menu() -> void:
	set_ui_page("pokedex")

func show_pokedex_screen() -> void:
	set_ui_page("pokedex")

func show_pc_menu() -> void:
	set_ui_page("pc_menu")

func show_pc_screen() -> void:
	set_ui_page("pc_menu")

func show_pokegear_menu() -> void:
	set_ui_page("pokegear")

func show_pokegear_screen() -> void:
	set_ui_page("pokegear")

func show_trainer_card_menu() -> void:
	set_ui_page("trainer_card")

func show_trainer_card_screen() -> void:
	set_ui_page("trainer_card")

func show_options_menu() -> void:
	set_ui_page("options_menu")

func show_move_menu() -> void:
	set_ui_page("move_menu")

func show_day_of_week_screen() -> void:
	set_ui_page("day_of_week")

func get_ui_page() -> String:
	return ui_page

func get_page_name() -> String:
	return ui_page

func get_ui_page_name() -> String:
	return ui_page

func get_ui_page_title() -> String:
	return str(PAGE_TITLES.get(ui_page, ui_page.to_upper()))

func get_ui_page_kind() -> String:
	if is_title_page():
		return "title"
	if is_intro_page():
		return "intro"
	if is_menu_page():
		return "menu"
	return "page"

func is_title_page() -> bool:
	return ui_page == "title"

func is_intro_page() -> bool:
	return ui_page == "intro" or ui_page == "oak_intro"

func is_menu_page() -> bool:
	return _is_menu_page(ui_page)

func open_dialogue(content: Variant) -> void:
	_call_if_present(text_box, "open_dialogue", [content])

func close_dialogue() -> void:
	_call_if_present(text_box, "close_dialogue")

func push_menu_panel(panel: Variant) -> Dictionary:
	if _has_callable(menu_stack, "push_panel"):
		return Dictionary(menu_stack.call("push_panel", panel))
	return {}

func pop_menu_panel() -> Dictionary:
	if _has_callable(menu_stack, "pop_panel"):
		return Dictionary(menu_stack.call("pop_panel"))
	return {}

func clear_menu_stack() -> void:
	_call_if_present(menu_stack, "clear")

func has_dialogue() -> bool:
	return _has_callable(text_box, "is_active") and bool(text_box.call("is_active"))

func has_menu_stack() -> bool:
	return _has_callable(menu_stack, "is_active") and bool(menu_stack.call("is_active"))

func has_menu_state() -> bool:
	return _has_callable(_ensure_menu_state(), "is_active") and bool(menu_state.call("is_active"))

func is_idle() -> bool:
	return not should_block_gameplay_input()

func is_input_locked() -> bool:
	return (_has_callable(text_box, "is_input_locked") and bool(text_box.call("is_input_locked"))) or (
		_has_callable(menu_stack, "is_input_locked") and bool(menu_stack.call("is_input_locked"))
	) or (_has_callable(_ensure_menu_state(), "is_input_locked") and bool(menu_state.call("is_input_locked")))

func should_block_gameplay_input() -> bool:
	if _has_callable(menu_stack, "should_block_gameplay_input") and bool(menu_stack.call("should_block_gameplay_input")):
		return true
	if _has_callable(_ensure_menu_state(), "should_block_gameplay_input") and bool(menu_state.call("should_block_gameplay_input")):
		return true
	if _has_callable(text_box, "should_block_gameplay_input") and bool(text_box.call("should_block_gameplay_input")):
		return true
	return false

func can_accept_input() -> bool:
	return not should_block_gameplay_input()

func get_active_layer() -> String:
	if has_menu_stack():
		return "menu"
	if has_menu_state():
		return "menu"
	if has_dialogue():
		return "text_box"
	return "none"

func get_active_panel_kind() -> String:
	return str(get_top_panel().get("kind", "none"))

func get_active_panel_id() -> String:
	return str(get_top_panel().get("id", ""))

func get_top_panel() -> Dictionary:
	if _has_callable(menu_stack, "is_active") and bool(menu_stack.call("is_active")) and _has_callable(menu_stack, "get_top_panel"):
		var top: Dictionary = Dictionary(menu_stack.call("get_top_panel"))
		top["source"] = "menu"
		return top
	if _has_callable(_ensure_menu_state(), "is_active") and bool(menu_state.call("is_active")) and _has_callable(menu_state, "get_top_panel"):
		var state_top: Dictionary = Dictionary(menu_state.call("get_top_panel"))
		state_top["source"] = "menu"
		return state_top
	if _has_callable(text_box, "is_active") and bool(text_box.call("is_active")) and _has_callable(text_box, "get_current_panel"):
		var panel: Dictionary = Dictionary(text_box.call("get_current_panel"))
		panel["source"] = "text_box"
		panel["depth"] = 1
		return panel
	return {
		"source": "none",
		"id": "",
		"title": "",
		"kind": "none",
		"depth": 0,
	}

func get_state() -> Dictionary:
	_sync_menu_state_from_runtime()
	_store_current_page_snapshot()
	var dialogue_open := has_dialogue()
	var menu_open := has_menu_stack()
	var text_box_state := _get_child_state(text_box)
	var menu_stack_state := _get_child_state(menu_stack)
	var menu_state_state := _get_child_state(_ensure_menu_state())
	return {
		"blocking_gameplay_input": should_block_gameplay_input(),
		"active_layer": get_active_layer(),
		"ui_page": ui_page,
		"page_name": ui_page,
		"ui_page_name": ui_page,
		"active_page": ui_page,
		"route_page": ui_page,
		"current_page": ui_page,
		"page_title": get_ui_page_title(),
		"page_kind": get_ui_page_kind(),
		"page_snapshots": _duplicate_dictionary(page_snapshots),
		"dialogue_open": dialogue_open,
		"menu_open": menu_open or has_menu_state(),
		"dialog_active": dialogue_open or menu_open or has_menu_state(),
		"input_owned": should_block_gameplay_input(),
		"text_box_open": dialogue_open,
		"top_panel": get_top_panel(),
		"text_box": text_box_state,
		"ui_dialogue_state": text_box_state,
		"dialogue_state": text_box_state,
		"menu_stack": menu_stack_state,
		"ui_menu_state": menu_stack_state,
		"menu_state": menu_state_state,
	}

func from_state(data: Dictionary) -> void:
	if data.is_empty():
		reset()
		return
	reset(false)
	page_snapshots = _sanitize_page_snapshots(data.get("page_snapshots", {}))
	ui_page = _normalize_page_name(str(data.get("ui_page", data.get("page_name", data.get("ui_page_name", data.get("active_page", data.get("route_page", data.get("current_page", ui_page))))))))
	var text_box_state: Dictionary = Dictionary(data.get("text_box", data.get("ui_dialogue_state", data.get("dialogue_state", {}))))
	var menu_stack_state: Dictionary = Dictionary(data.get("menu_stack", data.get("ui_menu_state", {})))
	var menu_state_state: Dictionary = Dictionary(data.get("menu_state", {}))
	if text_box_state.is_empty() and menu_stack_state.is_empty() and menu_state_state.is_empty() and page_snapshots.has(ui_page):
		_restore_page_snapshot(ui_page)
	else:
		_restore_child_state(text_box, text_box_state)
		_restore_child_state(menu_stack, menu_stack_state)
		_restore_child_state(_ensure_menu_state(), menu_state_state)
	_sync_menu_state_from_runtime()
	_sync_menu_state_page(ui_page)
	_store_current_page_snapshot()
	_refresh_ui(true)

func to_dictionary() -> Dictionary:
	return get_state()

func from_dictionary(data: Variant) -> bool:
	if typeof(data) != TYPE_DICTIONARY:
		return false
	from_state(Dictionary(data))
	return true

func consume_input(frame_input: Dictionary) -> Dictionary:
	_sync_menu_state_from_runtime()
	var menu_result: Dictionary = {}
	if _has_callable(menu_stack, "is_active") and bool(menu_stack.call("is_active")) and _has_callable(menu_stack, "consume_input"):
		menu_result = Dictionary(menu_stack.call("consume_input", frame_input))
		menu_result["source"] = "menu"
		return menu_result
	if _has_callable(_ensure_menu_state(), "is_active") and bool(menu_state.call("is_active")) and _has_callable(menu_state, "consume_input"):
		menu_result = Dictionary(menu_state.call("consume_input", frame_input))
		menu_result["source"] = "menu_state"
		return menu_result
	var text_result: Dictionary = {}
	if _has_callable(text_box, "is_active") and bool(text_box.call("is_active")) and _has_callable(text_box, "consume_input"):
		text_result = Dictionary(text_box.call("consume_input", frame_input))
		text_result["source"] = "text_box"
		return text_result
	return {
		"consumed": false,
		"source": "none",
		"top_panel": get_top_panel(),
	}

func route_input(frame_input: Dictionary) -> Dictionary:
	return consume_input(frame_input)

func _refresh_ui(force: bool = false) -> void:
	_sync_menu_state_from_runtime()
	var text_box_state: Dictionary = Dictionary(text_box.call("get_state")) if _has_callable(text_box, "get_state") else {}
	var menu_stack_state: Dictionary = Dictionary(menu_stack.call("get_state")) if _has_callable(menu_stack, "get_state") else {}
	var menu_state_state: Dictionary = {}
	if _has_callable(_ensure_menu_state(), "get_state"):
		menu_state_state = Dictionary(menu_state.call("get_state"))
	var signature := "%s|%s|%s|%s" % [
		get_active_layer(),
		str(get_top_panel().hash()),
		str(get_state().hash()),
		str(is_input_locked()),
	]
	if not force and signature == _last_signature:
		return
	_last_signature = signature
	if is_instance_valid(_title_label):
		_title_label.text = "UI SHELL"
	if is_instance_valid(_status_label):
		_status_label.text = "Page: %s | Layer: %s | Blocked: %s | Locked: %s" % [
			ui_page,
			get_active_layer(),
			str(should_block_gameplay_input()).to_lower(),
			str(is_input_locked()).to_lower(),
		]
	if is_instance_valid(_page_label):
		_page_label.text = "UI page: %s" % ui_page
	if is_instance_valid(_panel_label):
		var top_panel := get_top_panel()
		_panel_label.text = "Top: %s / %s | Depth: %d" % [
			str(top_panel.get("kind", "none")),
			str(top_panel.get("title", "")),
			int(top_panel.get("depth", 0)),
		]
	if is_instance_valid(_debug_label):
		_debug_label.text = "TextBox active=%s visible=%s wait=%s page=%d/%d | MenuStack active=%s depth=%d | MenuState active=%s menu=%s" % [
			str(bool(text_box_state.get("active", false))).to_lower(),
			str(bool(text_box_state.get("visible", false))).to_lower(),
			str(bool(text_box_state.get("waiting_for_input", false))).to_lower(),
			int(text_box_state.get("page_index", -1)) + 1,
			int(text_box_state.get("page_count", 0)),
			str(bool(menu_stack_state.get("active", false))).to_lower(),
			int(menu_stack_state.get("depth", 0)),
			str(bool(menu_state_state.get("active", false))).to_lower(),
			str(menu_state_state.get("active_menu", "")),
		]

func _has_callable(target: Variant, method_name: String) -> bool:
	return target != null and target is Object and target.has_method(method_name)

func _call_if_present(target: Variant, method_name: String, args: Array = []) -> void:
	if not _has_callable(target, method_name):
		return
	if args.is_empty():
		target.call(method_name)
	elif args.size() == 1:
		target.call(method_name, args[0])
	elif args.size() == 2:
		target.call(method_name, args[0], args[1])
	elif args.size() == 3:
		target.call(method_name, args[0], args[1], args[2])
	else:
		target.callv(method_name, args)

func _get_child_state(target: Variant) -> Dictionary:
	if _has_callable(target, "to_dictionary"):
		return Dictionary(target.call("to_dictionary"))
	if _has_callable(target, "get_state"):
		return Dictionary(target.call("get_state"))
	return {}

func _store_current_page_snapshot() -> void:
	var text_box_state := _get_child_state(text_box)
	var menu_stack_state := _get_child_state(menu_stack)
	var menu_state_state := _get_child_state(_ensure_menu_state())
	page_snapshots[ui_page] = {
		"ui_page": ui_page,
		"page_name": ui_page,
		"route_page": ui_page,
		"text_box": text_box_state,
		"ui_dialogue_state": text_box_state,
		"dialogue_state": text_box_state,
		"menu_stack": menu_stack_state,
		"ui_menu_state": menu_stack_state,
		"menu_state": menu_state_state,
	}

func _restore_page_snapshot(page_name: String) -> void:
	var snapshot: Dictionary = Dictionary(page_snapshots.get(page_name, {}))
	if snapshot.is_empty():
		_call_if_present(text_box, "reset")
		_call_if_present(menu_stack, "reset")
		_call_if_present(_ensure_menu_state(), "reset")
		return
	_restore_child_state(text_box, Dictionary(snapshot.get("text_box", snapshot.get("ui_dialogue_state", snapshot.get("dialogue_state", {})))))
	_restore_child_state(menu_stack, Dictionary(snapshot.get("menu_stack", snapshot.get("ui_menu_state", {}))))
	_restore_child_state(_ensure_menu_state(), Dictionary(snapshot.get("menu_state", {})))
	_sync_menu_state_from_runtime()

func _sanitize_page_snapshots(value: Variant) -> Dictionary:
	var sanitized: Dictionary = {}
	if typeof(value) != TYPE_DICTIONARY:
		return sanitized
	for key in Dictionary(value).keys():
		var page_name := _normalize_page_name(str(key))
		var snapshot_value: Dictionary = Dictionary(Dictionary(value).get(key, {}))
		sanitized[page_name] = {
			"ui_page": page_name,
			"page_name": page_name,
			"route_page": page_name,
			"text_box": Dictionary(snapshot_value.get("text_box", snapshot_value.get("ui_dialogue_state", snapshot_value.get("dialogue_state", {})))).duplicate(true),
			"ui_dialogue_state": Dictionary(snapshot_value.get("ui_dialogue_state", snapshot_value.get("text_box", snapshot_value.get("dialogue_state", {})))).duplicate(true),
			"dialogue_state": Dictionary(snapshot_value.get("dialogue_state", snapshot_value.get("text_box", snapshot_value.get("ui_dialogue_state", {})))).duplicate(true),
			"menu_stack": Dictionary(snapshot_value.get("menu_stack", snapshot_value.get("ui_menu_state", {}))).duplicate(true),
			"ui_menu_state": Dictionary(snapshot_value.get("ui_menu_state", snapshot_value.get("menu_stack", {}))).duplicate(true),
			"menu_state": Dictionary(snapshot_value.get("menu_state", {})).duplicate(true),
		}
	return sanitized

func _capture_runtime_state() -> Dictionary:
	var parent := get_parent()
	if parent == null:
		return {}
	var pokemon_data: Array = []
	var asset_index: Variant = {}
	if parent is Object and parent.has_method("get"):
		asset_index = parent.get("asset_index")
		if asset_index != null and asset_index is Object and asset_index.has_method("load_array"):
			var loaded_pokemon: Variant = asset_index.call("load_array", "pokemon_data.json")
			if typeof(loaded_pokemon) == TYPE_ARRAY:
				pokemon_data = Array(loaded_pokemon).duplicate(true)
	if parent is Object and parent.has_method("get"):
		var runtime_state: Variant = parent.get("state")
		if typeof(runtime_state) == TYPE_DICTIONARY:
			var snapshot := Dictionary(runtime_state).duplicate(true)
			if not pokemon_data.is_empty():
				snapshot["pokemon_data"] = pokemon_data
			return snapshot
	return {}

func _sync_menu_state_from_runtime() -> void:
	var controller: Variant = _ensure_menu_state()
	if not _has_callable(controller, "sync_runtime_state"):
		return
	controller.call("sync_runtime_state", _capture_runtime_state())

func _duplicate_dictionary(value: Dictionary) -> Dictionary:
	return Dictionary(value).duplicate(true)

func _restore_child_state(target: Variant, data: Dictionary) -> void:
	if _has_callable(target, "from_dictionary"):
		target.call("from_dictionary", data)
		return
	if _has_callable(target, "from_state"):
		target.call("from_state", data)
		return
	if _has_callable(target, "reset"):
		target.call("reset")

func _normalize_page_name(page_name: String) -> String:
	var normalized := page_name.strip_edges().to_lower()
	match normalized:
		"":
			return "title"
		"title_screen":
			return "title"
		"intro_sequence":
			return "intro"
		"oak_intro_sequence":
			return "oak_intro"
		"menu":
			return "main_menu"
		"pack_menu":
			return "bag_menu"
		"intro", "title", "main_menu", "oak_intro", "start_menu", "bag_menu", "party_menu", "pokemon_menu", "move_menu", "pokedex", "pc_menu", "pokegear", "trainer_card", "options_menu", "day_of_week":
			return normalized
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
		"day_of_week_screen":
			return "day_of_week"
		"pokedex_menu":
			return "pokedex"
		"pokegear_menu":
			return "pokegear"
		"trainer_card_screen":
			return "trainer_card"
		"continue", "delete_save", "clock_reset", "gender", "name_entry":
			return normalized
		_:
			return normalized

func _sync_menu_state_page(page_name: String) -> void:
	var controller: Variant = _ensure_menu_state()
	if not _has_callable(controller, "activate_menu") or not _has_callable(controller, "deactivate_menu"):
		return
	if _is_menu_page(page_name):
		controller.call("activate_menu", page_name)
	else:
		controller.call("deactivate_menu")

func _is_menu_page(page_name: String) -> bool:
	match _normalize_page_name(page_name):
		"main_menu", "start_menu", "bag_menu", "party_menu", "pokemon_menu", "move_menu", "pokedex", "pc_menu", "pokegear", "trainer_card", "options_menu", "continue", "delete_save", "clock_reset", "gender", "name_entry", "day_of_week":
			return true
		_:
			return false
