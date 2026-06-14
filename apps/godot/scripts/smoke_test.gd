extends SceneTree

const MAIN_SCENE := "res://scenes/main.tscn"
const GAME_CORNER_STATE_SCRIPT := preload("res://scripts/game_corner_state.gd")
const SPECIAL_EVENTS_STATE_SCRIPT := preload("res://scripts/special_events_state.gd")

func _initialize() -> void:
	call_deferred("_run")

func _coerce_array(value: Variant) -> Array:
	if typeof(value) == TYPE_ARRAY:
		return Array(value)
	if typeof(value) == TYPE_DICTIONARY:
		var dict_value: Dictionary = Dictionary(value)
		if dict_value.has("x") and dict_value.has("y"):
			return [dict_value.get("x", 0), dict_value.get("y", 0)]
	if typeof(value) == TYPE_VECTOR2I:
		var vector_value: Vector2i = value
		return [vector_value.x, vector_value.y]
	return []

func _assert_boot_action_payload_round_trip(label: String, script: Script, snapshot: Dictionary, expected_pending_action: String, expected_payload: Dictionary) -> void:
	var model: Variant = script.new()
	if not bool(model.call("from_dictionary", snapshot)):
		push_error("smoke_test: %s snapshot restore failed" % label)
		quit(1)
		return
	var restored: Dictionary = Dictionary(model.call("to_dictionary"))
	if str(restored.get("pending_action", "")) != expected_pending_action:
		push_error("smoke_test: %s pending action did not round-trip" % label)
		quit(1)
		return
	var restored_payload: Dictionary = Dictionary(restored.get("pending_action_payload", {}))
	for key in expected_payload.keys():
		if restored_payload.get(key) != expected_payload.get(key):
			push_error("smoke_test: %s payload field %s did not round-trip" % [label, str(key)])
			quit(1)
			return

func _assert_boot_state_round_trip(label: String, script: Script, snapshot: Dictionary, expected_state: Dictionary) -> void:
	var model: Variant = script.new()
	if not bool(model.call("from_dictionary", snapshot)):
		push_error("smoke_test: %s snapshot restore failed" % label)
		quit(1)
		return
	var restored: Dictionary = Dictionary(model.call("to_dictionary"))
	for key in expected_state.keys():
		if restored.get(key) != expected_state.get(key):
			push_error("smoke_test: %s state field %s did not round-trip" % [label, str(key)])
			quit(1)
			return

func _run_menu_smoke() -> void:
	var menu_state_script: Script = load("res://scripts/menu_state.gd")
	if menu_state_script == null:
		push_error("smoke_test: failed to load menu state script")
		quit(1)
		return
	var menu_state_model: Variant = menu_state_script.new()
	menu_state_model.call("sync_runtime_state", {
		"sram": {
			"phone_numbers": ["MOM", "PROF_OAK", "BILL"],
		},
		"wram": {
			"engine_flags": {
				"ENGINE_POKEGEAR": true,
				"ENGINE_MAP_CARD": true,
				"ENGINE_PHONE_CARD": true,
				"ENGINE_RADIO_CARD": true,
			},
			"wMapGroup": 5,
			"wMapNumber": 6,
			"pokegear_card": 1,
			"pokegear_map_player_landmark": 24,
			"pokegear_map_cursor_landmark": 42,
			"pokegear_phone_cursor_position": 1,
			"pokegear_phone_scroll_position": 0,
			"pokegear_radio_frequency_raw": 32,
		},
		"player_name": "Chris",
		"player_gender": "male",
		"ui_page": "pokegear",
		"map_summary": {
			"map_name": "Azalea Town",
			"map_constant": "AZALEA_TOWN",
			"group_id": 5,
			"map_id": 6,
			"group_name": "JOHTO",
			"phone_service": 0,
		},
	})
	var menu_state_top: Dictionary = Dictionary(menu_state_model.call("activate_menu", "pokegear"))
	if str(menu_state_top.get("id", "")) != "pokegear" or int(menu_state_top.get("cursor", -1)) != 1:
		push_error("smoke_test: menu state failed to activate pokegear: %s" % JSON.stringify(menu_state_top))
		quit(1)
		return
	var menu_state_snapshot: Dictionary = Dictionary(menu_state_model.call("get_state"))
	var menu_state_top_panel: Dictionary = Dictionary(menu_state_snapshot.get("top_panel", {}))
	var menu_state_selection: Dictionary = Dictionary(menu_state_top_panel.get("selection", {}))
	var menu_state_selection_payload: Dictionary = Dictionary(menu_state_selection.get("payload", {}))
	var menu_state_menus: Dictionary = Dictionary(menu_state_snapshot.get("menus", {}))
	var menu_state_pokegear_detail: Dictionary = Dictionary(menu_state_menus.get("pokegear", {}))
	var menu_state_pokegear_state: Dictionary = Dictionary(menu_state_pokegear_detail.get("state", {}))
	if str(menu_state_pokegear_state.get("card", "")) != "MAP" or int(menu_state_pokegear_state.get("card_index", -1)) != 1 or str(menu_state_selection_payload.get("card", "")) != "MAP":
		push_error("smoke_test: pokegear did not reflect the selected map card")
		quit(1)
		return
	var pokegear_up_result: Dictionary = Dictionary(menu_state_model.call("consume_input", {"pressed": {"up": true}}))
	var pokegear_after_up: Dictionary = Dictionary(menu_state_model.call("get_state"))
	var pokegear_after_up_detail: Dictionary = Dictionary(Dictionary(Dictionary(pokegear_after_up.get("menus", {})).get("pokegear", {})).get("state", {}))
	if str(pokegear_up_result.get("action", "")) != "move_up" or int(pokegear_after_up_detail.get("map_cursor_landmark", -1)) != 41:
		push_error("smoke_test: pokegear map cursor did not move")
		quit(1)
		return
	var pokegear_right_result: Dictionary = Dictionary(menu_state_model.call("consume_input", {"pressed": {"right": true}}))
	var pokegear_phone_snapshot: Dictionary = Dictionary(menu_state_model.call("get_state"))
	var pokegear_phone_detail: Dictionary = Dictionary(Dictionary(Dictionary(pokegear_phone_snapshot.get("menus", {})).get("pokegear", {})).get("state", {}))
	var pokegear_phone_selection: Dictionary = Dictionary(Dictionary(pokegear_phone_snapshot.get("top_panel", {})).get("selection", {}))
	var pokegear_phone_payload: Dictionary = Dictionary(pokegear_phone_selection.get("payload", {}))
	if str(pokegear_right_result.get("action", "")) != "switch_card" or str(pokegear_phone_detail.get("card", "")) != "PHONE" or int(pokegear_phone_detail.get("phone_cursor", -1)) != 1 or str(pokegear_phone_payload.get("card", "")) != "PHONE":
		push_error("smoke_test: pokegear phone card switch did not update state")
		quit(1)
		return
	var pokegear_phone_move_result: Dictionary = Dictionary(menu_state_model.call("consume_input", {"pressed": {"down": true}}))
	var pokegear_phone_move_snapshot: Dictionary = Dictionary(menu_state_model.call("get_state"))
	var pokegear_phone_move_detail: Dictionary = Dictionary(Dictionary(Dictionary(pokegear_phone_move_snapshot.get("menus", {})).get("pokegear", {})).get("state", {}))
	if str(pokegear_phone_move_result.get("action", "")) != "move_down" or int(pokegear_phone_move_detail.get("phone_cursor", -1)) != 2:
		push_error("smoke_test: pokegear phone cursor did not move")
		quit(1)
		return
	var pokegear_right_again_result: Dictionary = Dictionary(menu_state_model.call("consume_input", {"pressed": {"right": true}}))
	var pokegear_radio_snapshot: Dictionary = Dictionary(menu_state_model.call("get_state"))
	var pokegear_radio_detail: Dictionary = Dictionary(Dictionary(Dictionary(pokegear_radio_snapshot.get("menus", {})).get("pokegear", {})).get("state", {}))
	var pokegear_radio_selection: Dictionary = Dictionary(Dictionary(pokegear_radio_snapshot.get("top_panel", {})).get("selection", {}))
	var pokegear_radio_payload: Dictionary = Dictionary(pokegear_radio_selection.get("payload", {}))
	if str(pokegear_right_again_result.get("action", "")) != "switch_card" or str(pokegear_radio_detail.get("card", "")) != "RADIO" or int(pokegear_radio_detail.get("radio_frequency_raw", -1)) != 32 or str(pokegear_radio_payload.get("card", "")) != "RADIO":
		push_error("smoke_test: pokegear radio card switch did not update state")
		quit(1)
		return
	var pokegear_radio_move_result: Dictionary = Dictionary(menu_state_model.call("consume_input", {"pressed": {"down": true}}))
	var pokegear_radio_move_snapshot: Dictionary = Dictionary(menu_state_model.call("get_state"))
	var pokegear_radio_move_detail: Dictionary = Dictionary(Dictionary(Dictionary(pokegear_radio_move_snapshot.get("menus", {})).get("pokegear", {})).get("state", {}))
	if str(pokegear_radio_move_result.get("action", "")) != "move_down" or int(pokegear_radio_move_detail.get("radio_frequency_raw", -1)) != 28 or absf(float(pokegear_radio_move_detail.get("radio_frequency", 0.0)) - 7.5) > 0.01:
		push_error("smoke_test: pokegear radio tuning did not update state")
		quit(1)
		return
	if not bool(menu_state_model.call("from_dictionary", pokegear_radio_move_snapshot)):
		push_error("smoke_test: menu state pokegear snapshot restore failed")
		quit(1)
		return
	var restored_menu_state: Dictionary = Dictionary(menu_state_model.call("get_state"))
	var restored_menu_state_selection: Dictionary = Dictionary(Dictionary(restored_menu_state.get("top_panel", {})).get("selection", {}))
	var restored_menus: Dictionary = Dictionary(restored_menu_state.get("menus", {}))
	var restored_pokegear_detail: Dictionary = Dictionary(Dictionary(restored_menus.get("pokegear", {})).get("state", {}))
	if str(restored_pokegear_detail.get("card", "")) != "RADIO" or int(restored_pokegear_detail.get("phone_cursor", -1)) != 2 or int(restored_pokegear_detail.get("radio_frequency_raw", -1)) != 28 or int(restored_pokegear_detail.get("map_cursor_landmark", -1)) != 41:
		push_error("smoke_test: pokegear state did not survive round-trip")
		quit(1)
		return
	if str(restored_menu_state_selection.get("payload", {}).get("card", "")) != "RADIO":
		push_error("smoke_test: pokegear selection did not survive round-trip")
		quit(1)
		return
	menu_state_model.call("reset")
	menu_state_model.call("sync_runtime_state", {
		"sram": {
			"badges": {
				"johto": [true, true, true, true, true, true, true, true],
				"kanto": [true, false, true, false, true, false, true, false],
			},
			"money": 12345,
		},
		"wram": {
			"engine_flags": {},
		},
		"player_name": "Chris",
		"player_gender": "male",
		"ui_page": "trainer_card",
	})
	var trainer_top: Dictionary = Dictionary(menu_state_model.call("activate_menu", "trainer_card"))
	var trainer_snapshot: Dictionary = Dictionary(menu_state_model.call("get_state"))
	var trainer_menu: Dictionary = Dictionary(trainer_snapshot.get("menus", {})).get("trainer_card", {})
	var trainer_detail: Dictionary = Dictionary(Dictionary(trainer_menu).get("state", {}))
	if str(Dictionary(trainer_top.get("selection", {})).get("payload", {}).get("page", "")) != "info" or str(trainer_detail.get("page", "")) != "info" or int(trainer_detail.get("johto_badges", -1)) != 8 or int(trainer_detail.get("kanto_badges", -1)) != 4:
		push_error("smoke_test: trainer card did not initialize info page correctly")
		quit(1)
		return
	var trainer_right_result: Dictionary = Dictionary(menu_state_model.call("consume_input", {"pressed": {"right": true}}))
	var trainer_right_snapshot: Dictionary = Dictionary(menu_state_model.call("get_state"))
	var trainer_right_detail: Dictionary = Dictionary(Dictionary(Dictionary(trainer_right_snapshot.get("menus", {})).get("trainer_card", {})).get("state", {}))
	if str(trainer_right_result.get("action", "")) != "move_right" or str(trainer_right_detail.get("page", "")) != "johto_badges" or int(trainer_right_detail.get("page_index", -1)) != 1:
		push_error("smoke_test: trainer card page switch to johto badges failed")
		quit(1)
		return
	if not bool(menu_state_model.call("from_dictionary", trainer_right_snapshot)):
		push_error("smoke_test: trainer card snapshot restore failed")
		quit(1)
		return
	var restored_trainer_state: Dictionary = Dictionary(menu_state_model.call("get_state"))
	var restored_trainer_detail: Dictionary = Dictionary(Dictionary(Dictionary(restored_trainer_state.get("menus", {})).get("trainer_card", {})).get("state", {}))
	if str(restored_trainer_detail.get("page", "")) != "johto_badges" or int(restored_trainer_detail.get("page_index", -1)) != 1:
		push_error("smoke_test: trainer card did not survive round-trip")
		quit(1)
		return
	menu_state_model.call("reset")
	menu_state_model.call("sync_runtime_state", {
		"sram": {
			"options": {
				"text_speed": "fast",
				"battle_scene": true,
				"battle_style": "shift",
				"sound": "stereo",
				"menu_account": true,
				"frame": 1,
				"print_option": "normal",
			},
		},
		"wram": {},
		"player_name": "Chris",
		"player_gender": "male",
		"ui_page": "options_menu",
	})
	var options_top: Dictionary = Dictionary(menu_state_model.call("activate_menu", "options_menu"))
	if int(options_top.get("cursor", -1)) != 0 or str(Dictionary(options_top.get("selection", {})).get("id", "")) != "text_speed":
		push_error("smoke_test: options menu did not initialize correctly")
		quit(1)
		return
	var options_adjust_result: Dictionary = Dictionary(menu_state_model.call("consume_input", {"pressed": {"right": true}}))
	var options_after_adjust: Dictionary = Dictionary(menu_state_model.call("get_state"))
	var options_runtime: Dictionary = Dictionary(Dictionary(options_after_adjust.get("runtime_context", {})).get("sram", {}))
	var options_values: Dictionary = Dictionary(options_runtime.get("options", {}))
	if str(options_adjust_result.get("action", "")) != "adjust_option" or str(options_values.get("text_speed", "")) != "mid":
		push_error("smoke_test: options mutation did not update text speed")
		quit(1)
		return
	var options_down_result: Dictionary = Dictionary(menu_state_model.call("consume_input", {"pressed": {"down": true}}))
	var options_toggle_result: Dictionary = Dictionary(menu_state_model.call("consume_input", {"pressed": {"left": true}}))
	var options_after_toggle: Dictionary = Dictionary(menu_state_model.call("get_state"))
	var options_toggle_runtime: Dictionary = Dictionary(Dictionary(options_after_toggle.get("runtime_context", {})).get("sram", {}))
	var options_toggle_values: Dictionary = Dictionary(options_toggle_runtime.get("options", {}))
	var options_detail: Dictionary = Dictionary(Dictionary(Dictionary(options_after_toggle.get("menus", {})).get("options_menu", {})).get("state", {}))
	if str(options_down_result.get("action", "")) != "move_down" or str(options_toggle_result.get("action", "")) != "adjust_option" or bool(options_toggle_values.get("battle_scene", true)) or int(options_detail.get("cursor", -1)) != 1:
		push_error("smoke_test: options cursor or toggle state did not update correctly")
		quit(1)
		return
	if not bool(menu_state_model.call("from_dictionary", options_after_toggle)):
		push_error("smoke_test: options snapshot restore failed")
		quit(1)
		return
	var restored_options_state: Dictionary = Dictionary(menu_state_model.call("get_state"))
	var restored_options_detail: Dictionary = Dictionary(Dictionary(Dictionary(restored_options_state.get("menus", {})).get("options_menu", {})).get("state", {}))
	if int(restored_options_detail.get("cursor", -1)) != 1 or str(Dictionary(Dictionary(restored_options_state.get("runtime_context", {})).get("sram", {})).get("options", {}).get("text_speed", "")) != "mid":
		push_error("smoke_test: options did not survive round-trip")
		quit(1)
		return
	menu_state_model.call("reset")
	menu_state_model.call("sync_runtime_state", {
		"sram": {
			"party": {
				"pokemon": [
					{
						"nickname": "Cinder",
						"species": {"id": "CYNDAQUIL"},
						"level": 8,
						"hp": 22,
						"max_hp": 28,
						"status": "",
					},
					{
						"nickname": "Wave",
						"species": {"id": "TOTODILE"},
						"level": 9,
						"hp": 25,
						"max_hp": 30,
						"status": "",
					},
				],
			},
		},
		"wram": {},
		"player_name": "Chris",
		"player_gender": "male",
		"ui_page": "pokemon_menu",
	})
	var pokemon_top: Dictionary = Dictionary(menu_state_model.call("activate_menu", "pokemon_menu"))
	if str(Dictionary(pokemon_top.get("selection", {})).get("payload", {}).get("species", "")) != "CYNDAQUIL":
		push_error("smoke_test: pokemon menu did not initialize the party list correctly")
		quit(1)
		return
	var pokemon_action_result: Dictionary = Dictionary(menu_state_model.call("consume_input", {"pressed": {"a": true}}))
	var pokemon_action_snapshot: Dictionary = Dictionary(menu_state_model.call("get_state"))
	var pokemon_detail: Dictionary = Dictionary(Dictionary(Dictionary(pokemon_action_snapshot.get("menus", {})).get("pokemon_menu", {})).get("state", {}))
	var pokemon_action_payload: Dictionary = Dictionary(Dictionary(Dictionary(pokemon_action_snapshot.get("top_panel", {})).get("selection", {})).get("payload", {}))
	if str(pokemon_action_result.get("action", "")) != "open_action_menu" or str(pokemon_detail.get("mode", "")) != "action" or int(pokemon_detail.get("selected_index", -1)) != 0 or int(pokemon_detail.get("action_cursor", -1)) != 0 or str(pokemon_action_payload.get("intent", "")) != "pokemon_action":
		push_error("smoke_test: pokemon action menu did not open correctly")
		quit(1)
		return
	if not bool(menu_state_model.call("from_dictionary", pokemon_action_snapshot)):
		push_error("smoke_test: pokemon snapshot restore failed")
		quit(1)
		return
	var restored_pokemon_state: Dictionary = Dictionary(menu_state_model.call("get_state"))
	var restored_pokemon_detail: Dictionary = Dictionary(Dictionary(Dictionary(restored_pokemon_state.get("menus", {})).get("pokemon_menu", {})).get("state", {}))
	var restored_pokemon_payload: Dictionary = Dictionary(Dictionary(Dictionary(restored_pokemon_state.get("top_panel", {})).get("selection", {})).get("payload", {}))
	if str(restored_pokemon_detail.get("mode", "")) != "action" or int(restored_pokemon_detail.get("action_cursor", -1)) != 0 or str(restored_pokemon_payload.get("intent", "")) != "pokemon_action":
		push_error("smoke_test: pokemon menu did not survive round-trip")
		quit(1)
		return
	menu_state_model.call("reset")
	menu_state_model.call("sync_runtime_state", {
		"sram": {
			"pc_items": [{"id": "POTION"}, {"id": "REPEL"}],
			"pc_boxes": [
				{
					"name": "BOX 1",
					"pokemon": [
						{
							"nickname": "Leaf",
							"species": {"id": "BULBASAUR"},
							"level": 5,
							"hp": 16,
							"max_hp": 20,
						},
						null,
					],
				},
			],
		},
		"wram": {},
		"player_name": "Chris",
		"player_gender": "male",
		"ui_page": "pc_menu",
	})
	var pc_intent_top: Dictionary = Dictionary(menu_state_model.call("activate_menu", "pc_menu"))
	var pc_intent_payload: Dictionary = Dictionary(Dictionary(Dictionary(pc_intent_top.get("selection", {})).get("payload", {})))
	if str(pc_intent_payload.get("intent", "")) != "pc_action" or str(pc_intent_payload.get("action", "")) != "withdraw":
		push_error("smoke_test: pc intent payload did not initialize correctly")
		quit(1)
		return
	var pc_intent_result: Dictionary = Dictionary(menu_state_model.call("consume_input", {"pressed": {"a": true}}))
	var pc_intent_snapshot: Dictionary = Dictionary(menu_state_model.call("get_state"))
	var pc_intent_detail: Dictionary = Dictionary(Dictionary(Dictionary(pc_intent_snapshot.get("menus", {})).get("pc_menu", {})).get("state", {}))
	if str(pc_intent_result.get("action", "")) != "confirm" or str(Dictionary(pc_intent_result.get("selection", {})).get("intent", "")) != "pc_action" or str(Dictionary(pc_intent_detail.get("pending_action", {})).get("action", "")) != "withdraw":
		push_error("smoke_test: pc intent selection did not serialize correctly")
		quit(1)
		return
	menu_state_model.call("reset")
	menu_state_model.call("sync_runtime_state", {
		"sram": {
			"pc_items": [{"id": "POTION"}, {"id": "REPEL"}],
			"pc_boxes": [
				{
					"name": "BOX 1",
					"pokemon": [
						{
							"nickname": "Leaf",
							"species": {"id": "BULBASAUR"},
							"level": 5,
							"hp": 16,
							"max_hp": 20,
						},
						null,
					],
				},
			],
		},
		"wram": {},
		"player_name": "Chris",
		"player_gender": "male",
		"ui_page": "pc_menu",
	})
	var pc_top: Dictionary = Dictionary(menu_state_model.call("activate_menu", "pc_menu"))
	menu_state_model.call("consume_input", {"pressed": {"down": true}})
	menu_state_model.call("consume_input", {"pressed": {"down": true}})
	menu_state_model.call("consume_input", {"pressed": {"down": true}})
	menu_state_model.call("consume_input", {"pressed": {"down": true}})
	menu_state_model.call("consume_input", {"pressed": {"down": true}})
	var pc_hub_state: Dictionary = Dictionary(menu_state_model.call("get_state"))
	var pc_hub_detail: Dictionary = Dictionary(Dictionary(Dictionary(pc_hub_state.get("menus", {})).get("pc_menu", {})).get("state", {}))
	if int(pc_hub_detail.get("hub_cursor", -1)) != 5 or int(Dictionary(Dictionary(pc_hub_state.get("top_panel", {})).get("selection", {})).get("payload", {}).get("box_index", -1)) != 0:
		push_error("smoke_test: pc hub selection did not update")
		quit(1)
		return
	var pc_open_result: Dictionary = Dictionary(menu_state_model.call("consume_input", {"pressed": {"a": true}}))
	var pc_box_snapshot: Dictionary = Dictionary(menu_state_model.call("get_state"))
	var pc_box_detail: Dictionary = Dictionary(Dictionary(Dictionary(pc_box_snapshot.get("menus", {})).get("pc_menu", {})).get("state", {}))
	if str(pc_open_result.get("action", "")) != "open_box" or str(pc_box_detail.get("mode", "")) != "box" or int(pc_box_detail.get("active_box_index", -1)) != 0 or str(pc_box_detail.get("selected_action", "")) != "open_box":
		push_error("smoke_test: pc box view did not open correctly")
		quit(1)
		return
	if not bool(menu_state_model.call("from_dictionary", pc_box_snapshot)):
		push_error("smoke_test: pc snapshot restore failed")
		quit(1)
		return
	var restored_pc_state: Dictionary = Dictionary(menu_state_model.call("get_state"))
	var restored_pc_detail: Dictionary = Dictionary(Dictionary(Dictionary(restored_pc_state.get("menus", {})).get("pc_menu", {})).get("state", {}))
	if str(restored_pc_detail.get("mode", "")) != "box" or int(restored_pc_detail.get("box_cursor", -1)) != 0 or str(Dictionary(restored_pc_detail.get("action_intent", {})).get("intent", "")) == "":
		push_error("smoke_test: pc did not survive round-trip")
		quit(1)
		return
	menu_state_model.call("reset")
	menu_state_model.call("sync_runtime_state", {
		"sram": {
			"pokedex_seen": [3],
			"pokedex_owned": [3],
		},
		"wram": {},
		"player_name": "Chris",
		"player_gender": "male",
		"ui_page": "pokedex",
		"pokemon_data": [
			{"int_id": 1, "id": "CYNDAQUIL", "type1": "FIRE", "type2": ""},
			{"int_id": 2, "id": "TOTODILE", "type1": "WATER", "type2": ""},
			{"int_id": 3, "id": "GEODUDE", "type1": "ROCK", "type2": "GROUND"},
		],
	})
	var pokedex_top: Dictionary = Dictionary(menu_state_model.call("activate_menu", "pokedex"))
	menu_state_model.call("consume_input", {"pressed": {"down": true}})
	menu_state_model.call("consume_input", {"pressed": {"down": true}})
	menu_state_model.call("consume_input", {"pressed": {"a": true}})
	var pokedex_search_adjust: Dictionary = Dictionary(menu_state_model.call("consume_input", {"pressed": {"right": true}}))
	menu_state_model.call("consume_input", {"pressed": {"down": true}})
	menu_state_model.call("consume_input", {"pressed": {"down": true}})
	var pokedex_begin_result: Dictionary = Dictionary(menu_state_model.call("consume_input", {"pressed": {"a": true}}))
	var pokedex_entry_result: Dictionary = Dictionary(menu_state_model.call("consume_input", {"pressed": {"a": true}}))
	var pokedex_snapshot: Dictionary = Dictionary(menu_state_model.call("get_state"))
	var pokedex_detail: Dictionary = Dictionary(Dictionary(Dictionary(pokedex_snapshot.get("menus", {})).get("pokedex", {})).get("state", {}))
	var pokedex_entry_detail: Dictionary = Dictionary(Dictionary(pokedex_detail.get("entry_detail", {})))
	if str(pokedex_search_adjust.get("action", "")) != "adjust_type" or str(pokedex_begin_result.get("action", "")) != "begin_search" or str(pokedex_entry_result.get("action", "")) != "open_entry" or str(pokedex_detail.get("page", "")) != "entry_detail" or int(pokedex_detail.get("search_results_count", -1)) != 1 or str(pokedex_entry_detail.get("entry_species_id", "")) != "CYNDAQUIL" or str(pokedex_entry_detail.get("entry_source", "")) != "search_results":
		push_error("smoke_test: pokedex search state did not update correctly: %s / %s / %s" % [
			JSON.stringify(pokedex_search_adjust),
			JSON.stringify(pokedex_begin_result),
			JSON.stringify(pokedex_detail),
		])
		quit(1)
		return
	if not bool(menu_state_model.call("from_dictionary", pokedex_snapshot)):
		push_error("smoke_test: pokedex snapshot restore failed")
		quit(1)
		return
	var restored_pokedex_state: Dictionary = Dictionary(menu_state_model.call("get_state"))
	var restored_pokedex_detail: Dictionary = Dictionary(Dictionary(Dictionary(restored_pokedex_state.get("menus", {})).get("pokedex", {})).get("state", {}))
	var restored_pokedex_entry_detail: Dictionary = Dictionary(Dictionary(restored_pokedex_detail.get("entry_detail", {})))
	if str(restored_pokedex_detail.get("page", "")) != "entry_detail" or int(restored_pokedex_detail.get("search_results_count", -1)) != 1 or str(restored_pokedex_entry_detail.get("entry_species_id", "")) != "CYNDAQUIL":
		push_error("smoke_test: pokedex did not survive round-trip")
		quit(1)
		return
	quit(0)

func _run_special_events_smoke() -> void:
	var special_events: Variant = SPECIAL_EVENTS_STATE_SCRIPT.new()
	special_events.call("sync_runtime_state", {
		"sram": {
			"day_care": {
				"man": {"pokemon": true, "species": "TOGEPI", "level": 8, "nickname": "Egg", "steps_since_last_egg": 12},
				"lady": {"pokemon": true, "species": "PICHU", "level": 6, "nickname": "Volt", "steps_since_last_egg": 3},
				"egg_present": true,
				"steps_since_last_egg": 128,
				"can_breed": true,
			},
			"mystery_gift_unlocked": true,
			"mystery_gift": {
				"stored_item": "BERRY",
				"backup_item": "REVIVE",
				"daily_partner_ids": ["FALKNER", "BUG_CATCHER_ARNIE"],
			},
			"lucky_number_day": 5,
			"lucky_id_number": 12345,
			"current_pc_box": 7,
			"money": 42000,
			"moms_money": 9000,
			"mom_saving_active": true,
			"mom_saving_some_money": true,
			"buenas_password_category": 2,
			"buenas_password_index": 1,
			"items": {"RED_APRICORN": 2, "PINK_APRICORN": 1},
			"bug_contest_state": {
				"timer_active": true,
				"park_balls_remaining": 20,
				"caught_species": "SCYTHER",
				"caught_level": 16,
				"pending_caught_mon": {"species": "YANMA", "level": 13},
			},
			"bug_contest_results": {"winner_species": "SCYTHER", "winner_level": 16},
			"player_name": "Chris",
			"hall_of_fame": ["CHAMPION"],
			"johto_pokedex": true,
		},
		"wram": {
			"engine_flags": {"ENGINE_POKEDEX": true},
			"wHallOfFameCount": 1,
			"specials": {
				"magnet_train": {
					"count": 1,
					"direction_token": "northbound",
					"destination": "SAFFRON_CITY",
					"scene": "MagnetTrainScene",
				},
			},
		},
		"hram": {
			"hHours": 14,
			"hMinutes": 22,
			"hSeconds": 11,
		},
	})
	var special_state: Dictionary = Dictionary(special_events.call("get_state"))
	var special_domains: Dictionary = Dictionary(special_state.get("domains", {}))
	if special_domains.keys().size() < 10:
		push_error("smoke_test: special events did not expose the expected domain summaries")
		quit(1)
		return
	var day_care: Dictionary = Dictionary(special_domains.get("day_care", {}))
	var day_care_summary: Dictionary = Dictionary(day_care.get("summary", {}))
	if not bool(day_care_summary.get("can_breed", false)) or int(day_care_summary.get("steps_since_last_egg", -1)) != 128:
		push_error("smoke_test: special events day care state did not serialize correctly")
		quit(1)
		return
	var mystery_gift: Dictionary = Dictionary(special_domains.get("mystery_gift", {}))
	if not bool(Dictionary(mystery_gift.get("summary", {})).get("unlocked", false)):
		push_error("smoke_test: special events mystery gift state did not serialize correctly")
		quit(1)
		return
	var lucky_number: Dictionary = Dictionary(special_domains.get("lucky_number", {}))
	if int(Dictionary(lucky_number.get("summary", {})).get("lucky_id_number", -1)) != 12345:
		push_error("smoke_test: special events lucky number state did not serialize correctly")
		quit(1)
		return
	var mom_state: Dictionary = Dictionary(special_domains.get("mom", {}))
	if not bool(Dictionary(mom_state.get("summary", {})).get("mom_saving_active", false)):
		push_error("smoke_test: special events mom state did not serialize correctly")
		quit(1)
		return
	var pc_helpers: Dictionary = Dictionary(special_domains.get("pc_helpers", {}))
	if Array(Dictionary(pc_helpers.get("summary", {})).get("entries", [])).size() < 3:
		push_error("smoke_test: special events pc helper entries were not built")
		quit(1)
		return
	special_events.call("queue_intent", "mom", "bank_of_mom", {"action": "take"})
	special_events.call("queue_intent", "pc_helpers", "pokemon_center_pc", {"selected_index": 0, "selected_action": "player_pc"})
	var special_snapshot: Dictionary = Dictionary(special_events.call("to_dictionary"))
	var restored_special_events: Variant = SPECIAL_EVENTS_STATE_SCRIPT.new()
	if not bool(restored_special_events.call("from_dictionary", special_snapshot)):
		push_error("smoke_test: special events snapshot restore failed")
		quit(1)
		return
	if Dictionary(restored_special_events.call("to_dictionary")) != special_snapshot:
		push_error("smoke_test: special events snapshot did not round-trip")
		quit(1)
		return

func _run_game_corner_smoke() -> void:
	var game_corner_model: Variant = GAME_CORNER_STATE_SCRIPT.new()
	game_corner_model.call("seed_rng_state", 0x1234, 0xab, 0xcd)
	var slot_result: Dictionary = Dictionary(game_corner_model.call("spin_slots", 3, "lucky"))
	var card_state: Dictionary = Dictionary(game_corner_model.call("shuffle_card_flip"))
	var memory_state: Dictionary = Dictionary(game_corner_model.call("shuffle_memory_game"))
	var unown_state: Dictionary = Dictionary(game_corner_model.call("shuffle_unown_puzzle"))
	if slot_result.is_empty() or card_state.is_empty() or memory_state.is_empty() or unown_state.is_empty():
		push_error("smoke_test: game corner state initialization failed")
		quit(1)
		return
	var snapshot: Dictionary = Dictionary(game_corner_model.call("to_dictionary"))
	var restored_model: Variant = GAME_CORNER_STATE_SCRIPT.new()
	if not bool(restored_model.call("from_dictionary", snapshot)):
		push_error("smoke_test: game corner snapshot restore failed")
		quit(1)
		return
	var restored_snapshot: Dictionary = Dictionary(restored_model.call("to_dictionary"))
	if snapshot != restored_snapshot:
		push_error("smoke_test: game corner snapshot did not round-trip exactly")
		quit(1)
		return
	var slot_state: Dictionary = Dictionary(snapshot.get("slot_machine_state", {}))
	var card_flip_state: Dictionary = Dictionary(snapshot.get("card_flip_state", {}))
	var memory_game_state: Dictionary = Dictionary(snapshot.get("memory_game_state", {}))
	var unown_puzzle_state: Dictionary = Dictionary(snapshot.get("unown_puzzle_state", {}))
	if Array(slot_state.get("reel_positions", [])).size() != 3 or Array(card_flip_state.get("deck", [])).size() != 24 or Array(memory_game_state.get("board", [])).size() != 16 or Array(unown_puzzle_state.get("layout", [])).size() != 6:
		push_error("smoke_test: game corner state shapes were not preserved")
		quit(1)
		return

func _run_render_snapshot_smoke(render_snapshot_script: Script) -> void:
	var render_snapshot_a: Variant = render_snapshot_script.new()
	var render_snapshot_b: Variant = render_snapshot_script.new()
	var render_specs: Array = [
		{
			"id": "title",
			"method": "capture_title_frame",
			"payload": {
				"viewport_size": {"width": 160, "height": 144},
				"tilemap_layer_ids": ["bg0", "bg1", "sprites"],
				"sprite_draw_order": ["title_logo", "suicune"],
				"palette_bank": 0,
				"animation_frame": 2,
				"text_overlay": {
					"viewportLines": ["POKEMON CRYSTAL", "PRESS START"],
					"infoLines": [],
					"menuLines": [],
					"viewportTitle": "Title",
				},
				"menu_overlay": {
					"visible": false,
					"menuLines": [],
				},
			},
		},
		{
			"id": "intro",
			"method": "capture_intro_frame",
			"payload": {
				"viewport_size": {"width": 160, "height": 144},
				"tilemap_layer_ids": ["bg0", "bg1", "sprites"],
				"sprite_draw_order": ["copyright", "logo", "unown", "suicune"],
				"palette_bank": 1,
				"animation_frame": 0,
				"text_overlay": {
					"viewportLines": ["CRYSTAL INTRO", "SCENE: opening_logo"],
					"infoLines": [],
					"menuLines": [],
					"viewportTitle": "Intro",
				},
				"menu_overlay": {
					"visible": false,
					"menuLines": [],
				},
			},
		},
		{
			"id": "overworld",
			"method": "capture_overworld_frame",
			"payload": {
				"viewport_size": {"width": 160, "height": 144},
				"tilemap_layer_ids": ["bg0", "bg1", "window", "sprites"],
				"sprite_draw_order": ["player", "npc_1", "npc_2"],
				"palette_bank": 2,
				"animation_frame": 3,
				"text_overlay": {
					"viewportLines": ["OVERWORLD", "NEW BARK TOWN"],
					"infoLines": ["MAP: TEST_MAP"],
					"menuLines": [],
					"viewportTitle": "Overworld",
				},
				"menu_overlay": {
					"visible": false,
					"menuLines": [],
				},
			},
		},
		{
			"id": "menu",
			"method": "capture_menu_frame",
			"payload": {
				"viewport_size": {"width": 160, "height": 144},
				"tilemap_layer_ids": ["bg0", "window", "sprites"],
				"sprite_draw_order": ["cursor", "panel", "selection"],
				"palette_bank": 3,
				"animation_frame": 1,
				"text_overlay": {
					"viewportLines": ["MENU", "BAG"],
					"infoLines": ["CURSOR: 0"],
					"menuLines": ["ITEMS", "POKEMON", "EXIT"],
					"viewportTitle": "Menu",
				},
				"menu_overlay": {
					"visible": true,
					"menuLines": ["ITEMS", "POKEMON", "EXIT"],
				},
			},
		},
		{
			"id": "battle",
			"method": "capture_battle_frame",
			"payload": {
				"viewport_size": {"width": 160, "height": 144},
				"tilemap_layer_ids": ["bg0", "bg1", "window", "sprites"],
				"sprite_draw_order": ["player_mon", "enemy_mon", "dialogue_box"],
				"palette_bank": 4,
				"animation_frame": 0,
				"text_overlay": {
					"viewportLines": ["CHIKORITA VS PIDGEY"],
					"infoLines": ["TURN: 1"],
					"menuLines": ["FIGHT", "BAG", "PKMN", "RUN"],
					"viewportTitle": "Battle",
				},
				"menu_overlay": {
					"visible": true,
					"menuLines": ["FIGHT", "BAG", "PKMN", "RUN"],
				},
			},
		},
	]
	for render_spec in render_specs:
		var render_payload: Dictionary = Dictionary(render_spec.get("payload", {}))
		var render_method := str(render_spec.get("method", ""))
		var render_id := str(render_spec.get("id", ""))
		var render_frame_a: Dictionary = Dictionary(render_snapshot_a.call(render_method, render_payload))
		var render_frame_b: Dictionary = Dictionary(render_snapshot_b.call(render_method, render_payload))
		if render_frame_a.is_empty() or render_frame_b.is_empty():
			push_error("smoke_test: render %s frame did not capture" % render_id)
			quit(1)
			return
		if render_frame_a != render_payload or render_frame_b != render_payload:
			push_error("smoke_test: render %s frame did not round-trip" % render_id)
			quit(1)
			return
		if render_frame_a != render_frame_b:
			push_error("smoke_test: render %s frame was not deterministic" % render_id)
			quit(1)
			return
	var render_snapshot: Dictionary = Dictionary(render_snapshot_a.call("to_dictionary"))
	var restored_render_snapshot: Variant = render_snapshot_script.new()
	if not bool(restored_render_snapshot.call("from_dictionary", render_snapshot)):
		push_error("smoke_test: render snapshot restore failed")
		quit(1)
		return
	if Dictionary(restored_render_snapshot.call("to_dictionary")) != render_snapshot:
		push_error("smoke_test: render snapshot did not round-trip")
		quit(1)
		return

func _run_core_systems_smoke(core_systems_script: Script) -> void:
	var core_systems: Variant = core_systems_script.new()
	core_systems.call("configure", {
		"time": {
			"hour": 23,
			"last_daily_reset": {"year": 2000, "month": 1, "day": 1},
		},
		"wram": {
			"step_count": 255,
			"poison_step_count": 3,
			"happiness_step_count": 1,
			"daily_rematch_flags": [1, 1],
			"daily_phone_item_flags": [1],
			"daily_phone_time_of_day_flags": [1],
			"event_flags": {"FRUITTREE_ROUTE_30_COLLECTED": true},
			"engine_flags": {"ENGINE_DAILY_BUG_CONTEST": true},
		},
		"sram": {
			"money": 900,
			"party": {
				"pokemon": [
					{"species": "CYNDAQUIL", "nickname": "CYNDAQUIL", "hp": 5, "status": "PSN", "happiness": 100},
				],
			},
			"items": {"POTION": 1},
			"event_flags": {"FRUITTREE_ROUTE_30_COLLECTED": true},
			"mystery_gift_unlocked": true,
			"mystery_gift": {"daily_partner_ids": [7]},
		},
	})
	var step_result: Dictionary = Dictionary(core_systems.call("process_step"))
	var step_snapshot: Dictionary = Dictionary(core_systems.call("to_dictionary"))
	var step_wram: Dictionary = Dictionary(step_snapshot.get("wram", {}))
	var step_sram: Dictionary = Dictionary(step_snapshot.get("sram", {}))
	var step_party: Array = Array(Dictionary(step_sram.get("party", {})).get("pokemon", []))
	if int(step_wram.get("step_count", -1)) != 0 or int(Dictionary(step_party[0]).get("hp", -1)) != 4 or Dictionary(step_result.get("poison_result", {})).get("damagedNames", []) != ["CYNDAQUIL"]:
		push_error("smoke_test: core systems step/poison processing mismatch")
		quit(1)
		return
	var daily_result: Dictionary = Dictionary(core_systems.call("process_daily_events", {"year": 2000, "month": 1, "day": 2}))
	var daily_snapshot: Dictionary = Dictionary(core_systems.call("to_dictionary"))
	if not bool(daily_result.get("reset", false)) or Dictionary(Dictionary(daily_snapshot.get("wram", {})).get("event_flags", {})).has("FRUITTREE_ROUTE_30_COLLECTED") or not Array(Dictionary(Dictionary(daily_snapshot.get("sram", {})).get("mystery_gift", {})).get("daily_partner_ids", [])).is_empty():
		push_error("smoke_test: core systems daily reset mismatch")
		quit(1)
		return
	core_systems.call("configure_shop", "new_bark_shop", [{"identifier": "POTION", "displayName": "POTION", "price": 300}], 900, {"POTION": 1})
	var buy_result: Dictionary = Dictionary(core_systems.call("buy_selected", 2))
	var shop_snapshot: Dictionary = Dictionary(core_systems.call("to_dictionary"))
	if not bool(buy_result.get("success", false)) or str(buy_result.get("message", "")) != "¥000600" or int(Dictionary(Dictionary(shop_snapshot.get("sram", {})).get("items", {})).get("POTION", -1)) != 3:
		push_error("smoke_test: core systems shop buy mismatch")
		quit(1)
		return
	var core_round_trip: Variant = core_systems_script.new()
	if not bool(core_round_trip.call("from_dictionary", shop_snapshot)) or Dictionary(core_round_trip.call("to_dictionary")) != shop_snapshot:
		push_error("smoke_test: core systems snapshot did not round-trip")
		quit(1)
		return

func _run_story_events_smoke(story_events_script: Script) -> void:
	var story_events: Variant = story_events_script.new()
	story_events.call("enqueue_script", [
		{"op": "setflag", "flag": "EVENT_TEST_FLAG"},
		{"op": "writetext", "text": "HELLO"},
		{"op": "warp", "map": "CHERRYGROVE_CITY", "x": 3, "y": 4},
		{"op": "playmusic", "cue": "MUSIC_CHERRYGROVE_CITY"},
		{"op": "end"},
	])
	var flag_result: Dictionary = Dictionary(story_events.call("step"))
	var text_result: Dictionary = Dictionary(story_events.call("step"))
	if not bool(story_events.call("get_flag", "EVENT_TEST_FLAG")) or str(flag_result.get("op", "")) != "setflag" or not bool(Dictionary(text_result.get("runner", {})).get("waiting_for_input", false)):
		push_error("smoke_test: story events flag/text handling mismatch")
		quit(1)
		return
	story_events.call("answer_yes_no", true)
	var warp_result: Dictionary = Dictionary(story_events.call("step"))
	var audio_result: Dictionary = Dictionary(story_events.call("step"))
	if str(Dictionary(warp_result.get("payload", {})).get("map", "")) != "CHERRYGROVE_CITY" or str(Dictionary(audio_result.get("payload", {})).get("cue", "")) != "MUSIC_CHERRYGROVE_CITY":
		push_error("smoke_test: story events warp/audio payload mismatch")
		quit(1)
		return
	var story_snapshot: Dictionary = Dictionary(story_events.call("to_dictionary"))
	var story_round_trip: Variant = story_events_script.new()
	if not bool(story_round_trip.call("from_dictionary", story_snapshot)) or Dictionary(story_round_trip.call("to_dictionary")) != story_snapshot:
		push_error("smoke_test: story events snapshot did not round-trip")
		quit(1)
		return

func _run() -> void:
	if OS.get_environment("SPECIAL_EVENTS_SMOKE_ONLY") == "1":
		_run_special_events_smoke()
		quit(0)
		return
	if OS.get_environment("MENU_SMOKE_ONLY") == "1":
		_run_menu_smoke()
		quit(0)
		return
	var runtime_script: Script = load("res://scripts/game_runtime.gd")
	if runtime_script == null:
		push_error("smoke_test: failed to load game runtime script")
		quit(1)
		return
	var game_state_script_preflight: Script = load("res://scripts/game_state.gd")
	if game_state_script_preflight == null:
		push_error("smoke_test: failed to load game state script")
		quit(1)
		return
	var map_data_script_preflight: Script = load("res://scripts/map_data.gd")
	if map_data_script_preflight == null:
		push_error("smoke_test: failed to load map data script")
		quit(1)
		return
	var input_latch_script_preflight: Script = load("res://scripts/input_latch.gd")
	if input_latch_script_preflight == null:
		push_error("smoke_test: failed to load input latch script")
		quit(1)
		return
	var save_store_script_preflight: Script = load("res://scripts/save_store.gd")
	if save_store_script_preflight == null:
		push_error("smoke_test: failed to load save store script")
		quit(1)
		return
	var core_systems_script_preflight: Script = load("res://scripts/core_systems_state.gd")
	if core_systems_script_preflight == null:
		push_error("smoke_test: failed to load core systems script")
		quit(1)
		return
	_run_core_systems_smoke(core_systems_script_preflight)
	var story_events_script_preflight: Script = load("res://scripts/story_events_state.gd")
	if story_events_script_preflight == null:
		push_error("smoke_test: failed to load story events script")
		quit(1)
		return
	_run_story_events_smoke(story_events_script_preflight)
	var game_corner_script_preflight: Script = load("res://scripts/game_corner_state.gd")
	if game_corner_script_preflight == null:
		push_error("smoke_test: failed to load game corner script")
		quit(1)
		return
	_run_game_corner_smoke()
	var render_snapshot_script_preflight: Script = load("res://scripts/render_snapshot_state.gd")
	if render_snapshot_script_preflight == null:
		push_error("smoke_test: failed to load render snapshot script")
		quit(1)
		return
	_run_render_snapshot_smoke(render_snapshot_script_preflight)
	var scene := load(MAIN_SCENE)
	if scene == null:
		push_error("smoke_test: failed to load main scene")
		quit(1)
		return
	var root: Node = scene.instantiate()
	if root == null:
		push_error("smoke_test: failed to instantiate main scene")
		quit(1)
		return
	get_root().add_child(root)
	await process_frame

	var runtime: Node = root
	if not runtime.has_method("request_scene_route"):
		push_error("smoke_test: main runtime is missing scene routing")
		quit(1)
		return
	if not runtime.has_method("get_scene_route") or not runtime.has_method("get_scene_handoff") or not runtime.has_method("get_pending_scene_handoff") or not runtime.has_method("get_loaded_asset_summary") or not runtime.has_method("get_ui_page") or not runtime.has_method("get_last_frame_input") or not runtime.has_method("get_last_routed_input"):
		push_error("smoke_test: main runtime is missing coordinator getters")
		quit(1)
		return
	var ui_shell: CanvasItem = root.get_node_or_null("UIShell")
	var overworld: CanvasItem = root.get_node_or_null("Overworld")
	var battle: CanvasItem = root.get_node_or_null("Battle")
	if ui_shell == null or overworld == null or battle == null:
		push_error("smoke_test: route instances are missing")
		quit(1)
		return

	runtime.call("request_scene_route", "overworld", "smoke_test")
	await process_frame
	if str(runtime.get("current_scene_route")) != "overworld":
		push_error("smoke_test: failed to route to overworld")
		quit(1)
		return
	if not bool(overworld.visible) or bool(ui_shell.visible) or bool(battle.visible):
		push_error("smoke_test: overworld visibility mismatch")
		quit(1)
		return
	var overworld_runtime: Node = overworld
	if not overworld_runtime.has_method("set_player_position") or not overworld_runtime.has_method("request_move"):
		push_error("smoke_test: overworld route is missing movement methods")
		quit(1)
		return
	overworld_runtime.call("set_player_position", 0, 0)
	overworld_runtime.call("request_move", "right")
	overworld_runtime.call("tick")
	var overworld_state: Dictionary = Dictionary(overworld_runtime.call("get_state"))
	var move_result: Dictionary = Dictionary(overworld_state.get("last_move_result", {}))
	if str(move_result.get("state", "")) != "moved" or not bool(move_result.get("moved", false)):
		push_error("smoke_test: overworld movement did not move")
		quit(1)
		return
	var player_tile := _coerce_array(overworld_state.get("player_tile", []))
	if player_tile.size() < 2 or int(player_tile[0]) != 1 or int(player_tile[1]) != 0 or str(overworld_state.get("player_facing", "")) != "right":
		push_error("smoke_test: overworld movement did not update tile or facing")
		quit(1)
		return
	overworld_runtime.call("tick")
	var idle_overworld_state: Dictionary = Dictionary(overworld_runtime.call("get_state"))
	var idle_move_result: Dictionary = Dictionary(idle_overworld_state.get("last_move_result", {}))
	if str(idle_move_result.get("state", "")) != "moved" or not bool(idle_move_result.get("moved", false)):
		push_error("smoke_test: overworld last move result was cleared on idle tick")
		quit(1)
		return
	if str(idle_overworld_state.get("movement_state", "")) != "idle":
		push_error("smoke_test: overworld movement state did not settle to idle")
		quit(1)
		return
	var original_overworld_state: Dictionary = idle_overworld_state.duplicate(true)
	var available_map_keys := _coerce_array(original_overworld_state.get("available_map_keys", []))
	if available_map_keys.size() >= 2:
		var remapped_state: Dictionary = original_overworld_state.duplicate(true)
		remapped_state["current_map_key"] = ""
		remapped_state["selected_map_key"] = ""
		remapped_state["selected_map_index"] = 1
		remapped_state["available_map_keys"] = [available_map_keys[0], available_map_keys[1]]
		if not bool(overworld_runtime.call("from_dictionary", remapped_state)):
			push_error("smoke_test: overworld from_dictionary failed")
			quit(1)
			return
		if str(overworld_runtime.call("get_selected_map_key")) != str(available_map_keys[1]):
			push_error("smoke_test: overworld selected map did not restore from index")
			quit(1)
			return
		if int(overworld_runtime.call("get_selected_map_index")) != 1:
			push_error("smoke_test: overworld selected map index did not restore")
			quit(1)
			return
		if not bool(overworld_runtime.call("from_dictionary", original_overworld_state)):
			push_error("smoke_test: overworld restore after selector test failed")
			quit(1)
			return
	var overworld_state_snapshot := original_overworld_state.duplicate(true)
	overworld_state_snapshot["map_dimensions"] = {"x": 12, "y": 8}
	overworld_state_snapshot["current_group_id"] = 7
	overworld_state_snapshot["current_map_id"] = 42
	overworld_state_snapshot["current_map_key"] = "TEST_MAP"
	overworld_state_snapshot["selected_map_key"] = "TEST_MAP"
	overworld_state_snapshot["selected_map_index"] = 0
	overworld_state_snapshot["available_map_keys"] = ["TEST_MAP"]
	overworld_state_snapshot["current_map_block_key"] = "TEST_MAP_BLOCKS"
	overworld_state_snapshot["map_manifest"] = {"TEST_MAP": {"map_name": "Test Map", "map_constant": "TEST_MAP_CONST"}}
	overworld_state_snapshot["map_summary"] = {
		"map_key": "TEST_MAP",
		"map_name": "Test Map",
		"map_constant": "TEST_MAP_CONST",
		"group_id": 7,
		"groupId": 7,
		"map_id": 42,
		"mapId": 42,
		"width": 12,
		"height": 8,
		"changed_blocks": {"1,2": 7},
	}
	overworld_state_snapshot["spawn_summary"] = {"spawn_count": 1, "selected_spawn": "spawn-test"}
	overworld_state_snapshot["map_blocks"] = {"TestBlocks": {"blocks": [1, 2, 3]}}
	overworld_state_snapshot["current_map_payload"] = {
		"map_name": "Test Map",
		"blocks_label": "TEST_MAP_BLOCKS",
		"changed_blocks": {"1,2": 7},
	}
	overworld_state_snapshot["current_spawn_point"] = {
		"name": "spawn-test",
		"player_tile": {"x": 3, "y": 4},
		"facing": "left",
	}
	overworld_state_snapshot["player_tile"] = {"x": 3, "y": 4}
	overworld_state_snapshot["player_facing"] = "left"
	overworld_state_snapshot["fixed_step_count"] = 9
	overworld_state_snapshot["movement_state"] = "moving"
	overworld_state_snapshot["movement_locked"] = true
	overworld_state_snapshot["current_connections"] = [{"direction": "north", "map": "Route 29", "x": 8, "y": 0}]
	overworld_state_snapshot["current_warps"] = [{"x": 1, "y": 2, "warp_id": 1, "target_map_constant": "TEST_MAP_CONST", "target_warp_id": 1}]
	overworld_state_snapshot["current_bg_events"] = [
		{"x": 3, "y": 4, "event_type": "signpost", "script": "TEST_SIGNPOST"},
		{"x": 7, "y": 8, "event_type": "coord_event", "event_flag": "EVENT_BG_SEEN", "script": "TEST_COORD"},
	]
	overworld_state_snapshot["current_object_events"] = [{"x": 5, "y": 4, "sprite": "SPRITE", "script": "TRAINER_DANA", "event_flag": "EVENT_BEAT_DANA", "direction": "left", "facing": "left", "radius": 4}]
	overworld_state_snapshot["object_states"] = {
		"TRAINER_DANA": {
			"object_id": "TRAINER_DANA",
			"object_index": 1,
			"visible": false,
			"hidden": true,
			"removed": true,
			"defeated": true,
			"event_flag": "EVENT_BEAT_DANA",
			"event_flag_state": true,
			"direction": "left",
			"facing": "left",
			"tile_x": 5,
			"tile_y": 4,
			"step_animation_count": 2,
			"facing_update_count": 1,
			"event": {"x": 5, "y": 4, "sprite": "SPRITE", "script": "TRAINER_DANA", "event_flag": "EVENT_BEAT_DANA", "direction": "left", "facing": "left", "radius": 4},
		}
	}
	overworld_state_snapshot["event_flags"] = {"EVENT_BEAT_DANA": true, "EVENT_BG_SEEN": true}
	overworld_state_snapshot["last_move_result"] = {
		"state": "blocked",
		"blocked": true,
		"moved": false,
		"reason": "smoke",
		"direction": "left",
		"from_tile": {"x": 3, "y": 4},
		"to_tile": {"x": 2, "y": 4},
		"step": 99,
		"collision": true,
	}
	overworld_state_snapshot["last_warp_result"] = {
		"requested": true,
		"target": "spawn-test",
		"tile": {"x": 3, "y": 4},
	}
	overworld_state_snapshot["queued_scripts"] = [{"action": "special", "function": "smoke_script"}]
	overworld_state_snapshot["queued_events"] = [{"action": "interaction", "button": "confirm"}]
	overworld_state_snapshot["map_callbacks"] = [{"action": "check_scene", "map_key": "TEST_MAP"}]
	overworld_state_snapshot["object_movement_queue"] = [{"object": "PLAYER", "commands": ["step_left"]}]
	overworld_state_snapshot["tile_animation_state"] = {"map_key": "TEST_MAP", "tileset_name": "TestTileset", "frame_index": 1, "frame_count": 4, "step": 9}
	overworld_state_snapshot["wild_encounter_state"] = {
		"step_counter": 12,
		"repel_steps_remaining": 3,
		"time_of_day": "night",
		"surface": "grass",
		"last_roll": {
			"step_counter": 11,
			"surface": "grass",
			"time_of_day": "night",
			"eligible": true,
			"roll": 37,
			"repel_steps_remaining": 4,
		},
	}
	overworld_state_snapshot["special_state"] = {
		"map_callbacks_executed": [
			{"map_key": "TEST_MAP", "callback_type": "MAPCALLBACK_OBJECTS", "script_name": "TEST_CALLBACK"},
		],
		"last_field_move": {"move": "Cut", "x": 5, "y": 4, "map_key": "TEST_MAP", "scene_name": "TestScene"},
		"_runtime_queue_state": {
			"queued_scripts": [{"action": "special", "function": "stored_script"}],
			"queued_events": [],
			"map_callbacks": [],
			"object_movement_queue": [],
			"completed": [{"queue": "queued_scripts", "processed": true, "frame": 1}],
			"last_processed": {"queue": "queued_scripts", "processed": true, "frame": 1},
		},
	}
	overworld_state_snapshot["current_map_payload"]["time_of_day"] = "night"
	overworld_state_snapshot["current_map_payload"]["wild_encounter_data"] = {
		"map_name": "Test Map",
		"grass_rates": {"morning": 2, "day": 4, "night": 6},
		"water_rate": 3,
		"grass": {
			"morning": [{"level": 2, "species": "PIDGEY"}],
			"day": [{"level": 4, "species": "RATTATA"}],
			"night": [{"level": 6, "species": "GASTLY"}],
		},
		"water": {
			"morning": [{"level": 5, "species": "MAGIKARP"}],
			"day": [{"level": 6, "species": "MAGIKARP"}],
			"night": [{"level": 7, "species": "MAGIKARP"}],
		},
	}
	overworld_state_snapshot["pending_move"] = "left"
	overworld_state_snapshot["debug_lines"] = ["overworld ready", "smoke"]
	if not bool(overworld_runtime.call("from_dictionary", overworld_state_snapshot)):
		push_error("smoke_test: overworld from_dictionary deep round-trip failed")
		quit(1)
		return
	var restored_overworld_state: Dictionary = Dictionary(overworld_runtime.call("get_state"))
	if int(Dictionary(restored_overworld_state.get("map_dimensions", {})).get("x", -1)) != 12 or int(Dictionary(restored_overworld_state.get("map_dimensions", {})).get("y", -1)) != 8:
		push_error("smoke_test: overworld map dimensions did not round-trip")
		quit(1)
		return
	var restored_spawn_point: Dictionary = Dictionary(restored_overworld_state.get("current_spawn_point", {}))
	var restored_spawn_tile: Dictionary = Dictionary(restored_spawn_point.get("player_tile", {}))
	if int(restored_spawn_tile.get("x", -1)) != 3 or int(restored_spawn_tile.get("y", -1)) != 4:
		push_error("smoke_test: overworld spawn point did not round-trip")
		quit(1)
		return
	if int(Dictionary(restored_overworld_state.get("player_tile", {})).get("x", -1)) != 3 or int(Dictionary(restored_overworld_state.get("player_tile", {})).get("y", -1)) != 4:
		push_error("smoke_test: overworld tile did not round-trip")
		quit(1)
		return
	if str(restored_overworld_state.get("player_facing", "")) != "left":
		push_error("smoke_test: overworld facing did not round-trip")
		quit(1)
		return
	if int(restored_overworld_state.get("current_group_id", -1)) != 7 or int(restored_overworld_state.get("current_map_id", -1)) != 42:
		push_error("smoke_test: overworld map ids did not round-trip")
		quit(1)
		return
	if str(restored_overworld_state.get("selected_map_key", "")) != "TEST_MAP" or int(restored_overworld_state.get("selected_map_index", -1)) != 0:
		push_error("smoke_test: overworld selector state did not round-trip")
		quit(1)
		return
	if int(Array(restored_overworld_state.get("available_map_keys", [])).size()) != 1:
		push_error("smoke_test: overworld available map keys did not round-trip")
		quit(1)
		return
	if str(Array(restored_overworld_state.get("available_map_keys", []))[0]) != "TEST_MAP":
		push_error("smoke_test: overworld available map key did not round-trip")
		quit(1)
		return
	if str(restored_overworld_state.get("current_map_block_key", "")) != "TEST_MAP_BLOCKS":
		push_error("smoke_test: overworld map block key did not round-trip")
		quit(1)
		return
	if str(Dictionary(restored_overworld_state.get("map_manifest", {})).get("TEST_MAP", {}).get("map_name", "")) != "Test Map":
		push_error("smoke_test: overworld manifest did not round-trip")
		quit(1)
		return
	if str(Dictionary(restored_overworld_state.get("map_summary", {})).get("map_key", "")) != "TEST_MAP":
		push_error("smoke_test: overworld map summary did not round-trip")
		quit(1)
		return
	var restored_map_summary: Dictionary = Dictionary(restored_overworld_state.get("map_summary", {}))
	if str(restored_map_summary.get("map_constant", "")) != "TEST_MAP_CONST" or int(restored_map_summary.get("group_id", -1)) != 7 or int(restored_map_summary.get("map_id", -1)) != 42:
		push_error("smoke_test: overworld normalized map summary identity fields did not round-trip")
		quit(1)
		return
	if int(restored_map_summary.get("object_state_count", -1)) != 1 or int(restored_map_summary.get("hidden_object_count", -1)) != 1 or int(restored_map_summary.get("defeated_object_count", -1)) != 1 or int(restored_map_summary.get("map_callbacks_executed_count", -1)) != 1 or int(restored_map_summary.get("changed_block_count", -1)) != 1:
		push_error("smoke_test: overworld map summary did not expose event/object mutation counts")
		quit(1)
		return
	var restored_object_states: Dictionary = Dictionary(restored_overworld_state.get("object_states", {}))
	var restored_object_record: Dictionary = Dictionary(restored_object_states.get("TRAINER_DANA", {}))
	if not bool(restored_object_record.get("removed", false)) or bool(restored_object_record.get("visible", true)) or not bool(restored_object_record.get("hidden", false)) or not bool(restored_object_record.get("defeated", false)):
		push_error("smoke_test: overworld object state did not preserve hidden/defeated event flags")
		quit(1)
		return
	if str(restored_object_record.get("event_flag", "")) != "EVENT_BEAT_DANA" or not bool(restored_object_record.get("event_flag_state", false)):
		push_error("smoke_test: overworld object event flag metadata did not round-trip")
		quit(1)
		return
	if not bool(Dictionary(restored_overworld_state.get("event_flags", {})).get("EVENT_BEAT_DANA", false)):
		push_error("smoke_test: overworld event flag state did not round-trip")
		quit(1)
		return
	if not bool(Dictionary(restored_overworld_state.get("event_flags", {})).get("EVENT_BG_SEEN", false)):
		push_error("smoke_test: overworld bg event flag state did not round-trip")
		quit(1)
		return
	var restored_callback_queue: Array = Array(overworld_runtime.call("get_map_callback_queue_state"))
	if restored_callback_queue.size() != 1:
		push_error("smoke_test: overworld map callback queue state did not round-trip")
		quit(1)
		return
	var restored_warp_payloads: Array = Array(overworld_runtime.call("get_warp_transition_payloads"))
	var restored_connection_payloads: Array = Array(overworld_runtime.call("get_connection_transition_payloads"))
	if restored_warp_payloads.size() != 1 or restored_connection_payloads.size() != 1:
		push_error("smoke_test: overworld warp or connection transition payloads did not round-trip")
		quit(1)
		return
	var restored_event_activation_records: Array = Array(overworld_runtime.call("get_event_activation_records"))
	if restored_event_activation_records.size() != 2:
		push_error("smoke_test: overworld event activation records did not round-trip")
		quit(1)
		return
	var restored_coord_record: Dictionary = Dictionary(restored_event_activation_records[1])
	if str(restored_coord_record.get("gating_reason", "")) != "event_flag_set" or bool(restored_coord_record.get("active", true)):
		push_error("smoke_test: overworld coord event gating reason was malformed")
		quit(1)
		return
	var restored_object_gating_records: Array = Array(overworld_runtime.call("get_object_event_gating_records"))
	if restored_object_gating_records.size() != 1 or str(Dictionary(restored_object_gating_records[0]).get("gating_reason", "")) != "event_flag_set":
		push_error("smoke_test: overworld object event gating reason did not round-trip")
		quit(1)
		return
	var restored_tile_animation_state: Dictionary = Dictionary(overworld_runtime.call("get_tile_animation_state"))
	if int(restored_tile_animation_state.get("frame_index", -1)) != 1 or int(restored_tile_animation_state.get("frame_count", -1)) != 4 or int(restored_tile_animation_state.get("step", -1)) != 9:
		push_error("smoke_test: overworld tile animation state did not round-trip")
		quit(1)
		return
	var restored_field_move_state: Dictionary = Dictionary(overworld_runtime.call("get_field_move_state"))
	if str(restored_field_move_state.get("move", "")) != "Cut" or int(restored_field_move_state.get("x", -1)) != 5 or int(restored_field_move_state.get("y", -1)) != 4:
		push_error("smoke_test: overworld field move state did not round-trip")
		quit(1)
		return
	var restored_wild_encounter_state: Dictionary = Dictionary(overworld_runtime.call("get_wild_encounter_state"))
	if int(restored_wild_encounter_state.get("step_counter", -1)) != 12 or int(restored_wild_encounter_state.get("repel_steps_remaining", -1)) != 3 or str(restored_wild_encounter_state.get("time_of_day", "")) != "night" or str(restored_wild_encounter_state.get("surface", "")) != "grass":
		push_error("smoke_test: overworld wild encounter state did not round-trip")
		quit(1)
		return
	var restored_last_roll: Dictionary = Dictionary(overworld_runtime.call("get_last_wild_encounter_roll"))
	if int(restored_last_roll.get("step_counter", -1)) != 11 or str(restored_last_roll.get("surface", "")) != "grass" or str(restored_last_roll.get("time_of_day", "")) != "night" or int(restored_last_roll.get("repel_steps_remaining", -1)) != 4 or not bool(restored_last_roll.get("eligible", false)) or int(restored_last_roll.get("roll", -1)) != 37:
		push_error("smoke_test: overworld wild encounter roll did not round-trip")
		quit(1)
		return
	var restored_wild_encounter_payloads: Array = Array(overworld_runtime.call("get_wild_encounter_eligibility_payloads"))
	if restored_wild_encounter_payloads.size() != 2:
		push_error("smoke_test: overworld wild encounter payloads were missing")
		quit(1)
		return
	var restored_grass_wild_payload: Dictionary = Dictionary(restored_wild_encounter_payloads[0])
	var restored_water_wild_payload: Dictionary = Dictionary(restored_wild_encounter_payloads[1])
	if str(restored_grass_wild_payload.get("surface", "")) != "grass" or str(restored_water_wild_payload.get("surface", "")) != "water" or str(restored_grass_wild_payload.get("time_of_day", "")) != "night" or str(restored_water_wild_payload.get("time_of_day", "")) != "night":
		push_error("smoke_test: overworld wild encounter payload surfaces did not resolve")
		quit(1)
		return
	if int(restored_grass_wild_payload.get("base_rate", -1)) != 6 or int(restored_water_wild_payload.get("base_rate", -1)) != 3 or int(restored_grass_wild_payload.get("table_size", -1)) != 1 or int(restored_water_wild_payload.get("table_size", -1)) != 1:
		push_error("smoke_test: overworld wild encounter payload rates did not resolve")
		quit(1)
		return
	if bool(restored_grass_wild_payload.get("eligible", true)) or bool(restored_water_wild_payload.get("eligible", true)) or str(restored_grass_wild_payload.get("eligibility_reason", "")) != "repel_active" or str(restored_water_wild_payload.get("eligibility_reason", "")) != "repel_active":
		push_error("smoke_test: overworld wild encounter eligibility did not respect repel state")
		quit(1)
		return
	var restored_wild_map_summary: Dictionary = Dictionary(overworld_runtime.call("get_map_summary"))
	if int(restored_wild_map_summary.get("wild_encounter_count", -1)) != 2 or int(restored_wild_map_summary.get("wild_encounter_step_counter", -1)) != 12 or int(restored_wild_map_summary.get("wild_encounter_repel_steps_remaining", -1)) != 3 or str(restored_wild_map_summary.get("wild_encounter_time_of_day", "")) != "night":
		push_error("smoke_test: overworld wild encounter summary did not update")
		quit(1)
		return
	var restored_script_queue_state: Dictionary = Dictionary(overworld_runtime.call("get_script_queue_state"))
	if Array(restored_script_queue_state.get("queued_scripts", [])).size() < 2 or Array(restored_script_queue_state.get("queued_events", [])).is_empty():
		push_error("smoke_test: overworld script queue state did not expose queued payloads")
		quit(1)
		return
	var restored_motion_states: Array = Array(overworld_runtime.call("get_object_motion_states"))
	if restored_motion_states.size() != 1:
		push_error("smoke_test: overworld object motion state did not expose the trainer record")
		quit(1)
		return
	var restored_motion_record: Dictionary = Dictionary(restored_motion_states[0])
	if str(restored_motion_record.get("object_id", "")) != "TRAINER_DANA" or str(restored_motion_record.get("facing", "")) != "left" or int(restored_motion_record.get("step_animation_count", -1)) != 2 or int(restored_motion_record.get("facing_update_count", -1)) != 1:
		push_error("smoke_test: overworld object motion counters did not round-trip")
		quit(1)
		return
	var restored_trainer_payloads: Array = Array(overworld_runtime.call("get_trainer_sightline_payloads"))
	if restored_trainer_payloads.size() != 1:
		push_error("smoke_test: overworld trainer sightline payload was missing")
		quit(1)
		return
	var restored_trainer_payload: Dictionary = Dictionary(restored_trainer_payloads[0])
	if str(restored_trainer_payload.get("object_id", "")) != "TRAINER_DANA" or int(restored_trainer_payload.get("distance_tiles", -1)) != 2 or str(restored_trainer_payload.get("sightline_direction", "")) != "left" or not bool(restored_trainer_payload.get("event_flag_state", false)) or bool(restored_trainer_payload.get("in_sightline", true)):
		push_error("smoke_test: overworld trainer sightline payload was malformed")
		quit(1)
		return
	var hidden_render_object_states: Array = Array(overworld_runtime.call("get_render_object_states"))
	if hidden_render_object_states.size() != 0:
		push_error("smoke_test: hidden overworld object should not be in the render list")
		quit(1)
		return
	var hidden_runtime_summary: Dictionary = Dictionary(overworld_runtime.call("get_runtime_summary"))
	if int(hidden_runtime_summary.get("render_object_count", -1)) != 0 or int(Dictionary(overworld_runtime.call("get_map_summary")).get("render_object_count", -1)) != 0 or int(hidden_runtime_summary.get("warp_transition_count", -1)) != 1 or int(hidden_runtime_summary.get("connection_transition_count", -1)) != 1 or int(hidden_runtime_summary.get("event_activation_count", -1)) != 2 or int(hidden_runtime_summary.get("object_event_gating_count", -1)) != 1 or int(hidden_runtime_summary.get("tile_animation_frame", -1)) != 1:
		push_error("smoke_test: hidden overworld object did not clear render counts")
		quit(1)
		return
	overworld_runtime.call("set_event_flag", "EVENT_BEAT_DANA", false)
	var revealed_render_object_states: Array = Array(overworld_runtime.call("get_render_object_states"))
	if revealed_render_object_states.size() != 1:
		push_error("smoke_test: overworld object flag toggle did not rebuild the render list")
		quit(1)
		return
	var revealed_render_object: Dictionary = Dictionary(revealed_render_object_states[0])
	if str(revealed_render_object.get("object_id", "")) != "TRAINER_DANA" or not bool(revealed_render_object.get("visible", false)) or bool(revealed_render_object.get("hidden", true)):
		push_error("smoke_test: revealed overworld render object metadata did not update")
		quit(1)
		return
	var revealed_runtime_summary: Dictionary = Dictionary(overworld_runtime.call("get_runtime_summary"))
	if int(revealed_runtime_summary.get("render_object_count", -1)) != 1 or int(Dictionary(overworld_runtime.call("get_map_summary")).get("render_object_count", -1)) != 1 or int(revealed_runtime_summary.get("trainer_sighting_count", -1)) != 1 or int(revealed_runtime_summary.get("object_motion_count", -1)) != 1 or int(revealed_runtime_summary.get("warp_transition_count", -1)) != 1 or int(revealed_runtime_summary.get("connection_transition_count", -1)) != 1 or int(revealed_runtime_summary.get("event_activation_count", -1)) != 2 or int(revealed_runtime_summary.get("object_event_gating_count", -1)) != 1:
		push_error("smoke_test: revealed overworld object did not update render counts")
		quit(1)
		return
	var revealed_trainer_payloads: Array = Array(overworld_runtime.call("get_trainer_sightline_payloads"))
	if revealed_trainer_payloads.size() != 1 or bool(Dictionary(revealed_trainer_payloads[0]).get("event_flag_state", true)) or not bool(Dictionary(revealed_trainer_payloads[0]).get("in_sightline", false)):
		push_error("smoke_test: revealed overworld trainer sightline payload did not update")
		quit(1)
		return
	var revealed_overworld_state_snapshot := Dictionary(overworld_runtime.call("get_state"))
	if not bool(overworld_runtime.call("from_dictionary", revealed_overworld_state_snapshot)):
		push_error("smoke_test: overworld restore after render object toggle failed")
		quit(1)
		return
	var restored_revealed_render_objects: Array = Array(overworld_runtime.call("get_render_object_states"))
	if restored_revealed_render_objects.size() != 1:
		push_error("smoke_test: overworld render object list did not survive snapshot restore")
		quit(1)
		return
	var restored_revealed_render_object: Dictionary = Dictionary(restored_revealed_render_objects[0])
	if str(restored_revealed_render_object.get("object_id", "")) != "TRAINER_DANA" or not bool(restored_revealed_render_object.get("visible", false)) or bool(restored_revealed_render_object.get("hidden", true)):
		push_error("smoke_test: restored overworld render object metadata did not survive snapshot restore")
		quit(1)
		return
	var post_restore_motion_states: Array = Array(overworld_runtime.call("get_object_motion_states"))
	if post_restore_motion_states.size() != 1 or int(Dictionary(post_restore_motion_states[0]).get("step_animation_count", -1)) != 2:
		push_error("smoke_test: overworld motion state did not survive snapshot restore")
		quit(1)
		return
	overworld_runtime.call("tick")
	var advanced_motion_states: Array = Array(overworld_runtime.call("get_object_motion_states"))
	if int(Dictionary(advanced_motion_states[0]).get("step_animation_count", -1)) != 3:
		push_error("smoke_test: overworld motion state did not advance on tick")
		quit(1)
		return
	var advanced_field_move_state: Dictionary = Dictionary(overworld_runtime.call("get_field_move_state"))
	if str(advanced_field_move_state.get("move", "")) != "Cut":
		push_error("smoke_test: overworld field move state changed unexpectedly after restore")
		quit(1)
		return
	if int(Dictionary(restored_overworld_state.get("spawn_summary", {})).get("spawn_count", -1)) != 1:
		push_error("smoke_test: overworld spawn summary did not round-trip")
		quit(1)
		return
	if int(Array(Dictionary(Dictionary(restored_overworld_state.get("map_blocks", {})).get("TestBlocks", {})).get("blocks", [])).size()) != 3:
		push_error("smoke_test: overworld map blocks did not round-trip")
		quit(1)
		return
	if str(restored_overworld_state.get("movement_state", "")) != "moving" or not bool(restored_overworld_state.get("movement_locked", false)):
		push_error("smoke_test: overworld movement state did not round-trip")
		quit(1)
		return
	if str(Dictionary(restored_overworld_state.get("last_move_result", {})).get("state", "")) != "blocked":
		push_error("smoke_test: overworld last move result did not round-trip")
		quit(1)
		return
	if str(Dictionary(restored_overworld_state.get("last_warp_result", {})).get("target", "")) != "spawn-test":
		push_error("smoke_test: overworld last warp result did not round-trip")
		quit(1)
		return
	if int(Array(restored_overworld_state.get("current_connections", [])).size()) != 1 or int(Array(restored_overworld_state.get("current_warps", [])).size()) != 1:
		push_error("smoke_test: overworld event arrays did not round-trip")
		quit(1)
		return
	if str(Dictionary(overworld_runtime.call("get_map_summary")).get("map_key", "")) != "TEST_MAP":
		push_error("smoke_test: overworld map summary getter did not preserve metadata")
		quit(1)
		return
		var runtime_summary_after_restore: Dictionary = Dictionary(overworld_runtime.call("get_runtime_summary"))
		if int(runtime_summary_after_restore.get("object_state_count", -1)) != 1 or int(runtime_summary_after_restore.get("map_callbacks_executed_count", -1)) != 1 or int(runtime_summary_after_restore.get("changed_block_count", -1)) != 1 or int(runtime_summary_after_restore.get("render_object_count", -1)) != 1:
			push_error("smoke_test: overworld runtime summary did not expose event/object mutation counts")
			quit(1)
			return
	if int(Dictionary(overworld_runtime.call("get_spawn_summary")).get("spawn_count", -1)) != 1:
		push_error("smoke_test: overworld spawn summary getter did not preserve metadata")
		quit(1)
		return
	if str(Dictionary(Dictionary(overworld_runtime.call("get_map_manifest")).get("TEST_MAP", {})).get("map_name", "")) != "Test Map":
		push_error("smoke_test: overworld manifest getter did not preserve metadata")
		quit(1)
		return
	if str(Dictionary(overworld_runtime.call("get_current_spawn_point")).get("facing", "")) != "left":
		push_error("smoke_test: overworld spawn point getter did not preserve metadata")
		quit(1)
		return
	if overworld_runtime.has_method("get_runtime_queue_state"):
		var restored_runtime_queue: Dictionary = Dictionary(overworld_runtime.call("get_runtime_queue_state"))
		if Array(restored_runtime_queue.get("queued_scripts", [])).size() < 2 or Array(restored_runtime_queue.get("queued_events", [])).is_empty() or Array(restored_runtime_queue.get("map_callbacks", [])).is_empty() or Array(restored_runtime_queue.get("object_movement_queue", [])).is_empty():
			push_error("smoke_test: overworld runtime queue state did not ingest queue aliases")
			quit(1)
			return
		if str(Dictionary(restored_runtime_queue.get("last_processed", {})).get("queue", "")) != "queued_scripts":
			push_error("smoke_test: overworld runtime queue last processed state did not persist")
			quit(1)
			return
	if str(Array(restored_overworld_state.get("debug_lines", [])).back()) != "smoke":
		push_error("smoke_test: overworld debug lines did not round-trip")
		quit(1)
		return
	if str(restored_overworld_state.get("pending_move", "")) != "left":
		push_error("smoke_test: overworld pending move did not round-trip")
		quit(1)
		return
	if not bool(overworld_runtime.call("from_dictionary", original_overworld_state)):
		push_error("smoke_test: overworld restore after deep round-trip failed")
		quit(1)
		return
	var runtime_snapshot: Dictionary = Dictionary(runtime.call("to_dictionary"))
	if runtime_snapshot.is_empty():
		push_error("smoke_test: coordinator snapshot missing")
		quit(1)
		return
	runtime_snapshot["frame_counter"] = 321
	runtime_snapshot["ui_page"] = "title"
	runtime_snapshot["scene_history"] = ["ui_shell", "overworld", "battle"]
	runtime_snapshot["scene_handoff"] = {
		"from_scene": "overworld",
		"to_scene": "battle",
		"reason": "smoke",
		"frame": 123,
	}
	runtime_snapshot["pending_scene_handoff"] = {
		"from_scene": "battle",
		"to_scene": "ui_shell",
		"reason": "pending",
		"frame": 124,
	}
	if not bool(runtime.call("from_dictionary", runtime_snapshot)):
		push_error("smoke_test: coordinator from_dictionary failed")
		quit(1)
		return
	var expected_scene_route := str(runtime_snapshot.get("active_scene", "ui_shell"))
	var expected_ui_page := str(runtime_snapshot.get("ui_page", "title"))
	var coordinator_scene_handoff: Dictionary = Dictionary(runtime.call("get_scene_handoff"))
	var coordinator_pending_handoff: Dictionary = Dictionary(runtime.call("get_pending_scene_handoff"))
	var coordinator_asset_summary: Dictionary = Dictionary(runtime.call("get_loaded_asset_summary"))
	if str(coordinator_scene_handoff.get("reason", "")) != "smoke":
		push_error("smoke_test: coordinator getter scene handoff did not restore")
		quit(1)
		return
	if str(coordinator_pending_handoff.get("reason", "")) != "pending":
		push_error("smoke_test: coordinator getter pending handoff did not restore")
		quit(1)
		return
	if int(coordinator_asset_summary.get("content_pack_count", -1)) < 0:
		push_error("smoke_test: coordinator getter asset summary missing pack count")
		quit(1)
		return
	if str(runtime.call("get_scene_route")) != expected_scene_route or str(runtime.call("get_ui_page")) != expected_ui_page:
		push_error("smoke_test: coordinator getters did not reflect route state")
		quit(1)
		return
	if int(runtime.state.get("frame_counter", -1)) != 321:
		push_error("smoke_test: coordinator frame counter did not restore")
		quit(1)
		return
	if str(runtime.state.get("ui_page", "")) != "title":
		push_error("smoke_test: coordinator ui page did not restore")
		quit(1)
		return
	if str(Array(runtime.state.get("scene_history", [])).back()) != "battle":
		push_error("smoke_test: coordinator scene history did not restore")
		quit(1)
		return
	if str(Dictionary(runtime.state.get("scene_handoff", {})).get("reason", "")) != "smoke":
		push_error("smoke_test: coordinator scene handoff did not restore")
		quit(1)
		return
	if str(Dictionary(runtime.state.get("pending_scene_handoff", {})).get("reason", "")) != "pending":
		push_error("smoke_test: coordinator pending scene handoff did not restore")
		quit(1)
		return
	var map_data_script: Script = load("res://scripts/map_data.gd")
	if map_data_script == null:
		push_error("smoke_test: failed to load map data script")
		quit(1)
		return
	var map_model: Variant = map_data_script.new()
	var map_payload := {
		"asset_summary": {"content_pack_count": 2, "content_pack_version": 1},
		"runtime_map_metadata": {"TEST_MAP": {"constant": "TEST_MAP_CONST", "name": "Test Map", "groupId": 7}},
		"runtime_spawn_points": {"spawn-test": {"map_name": "Test Map", "map_constant": "TEST_MAP_CONST", "group_id": 7, "map_id": 42}},
		"map_attributes": {"Test Map": {"name": "Test Map", "map_constant": "TEST_MAP_CONST", "group_id": 7, "map_id": 42}},
		"map_blocks": {"TestBlocks": {"blocks": [1, 2, 3]}},
		"current_map_key": "TEST_MAP",
		"current_map_name": "Test Map",
		"current_map_constant": "TEST_MAP_CONST",
		"current_group_id": 7,
		"current_map_group_id": 7,
		"current_map_id": 42,
		"current_width": 5,
		"current_height": 4,
		"current_environment": "INDOOR",
		"current_music": "test_theme",
		"current_blocks_label": "TestBlocks",
		"current_map_events_label": "Test_MapEvents",
		"current_map_scripts_label": "Test_MapScripts",
		"current_map_summary": {
			"map_key": "TEST_MAP",
			"map_name": "Test Map",
			"map_constant": "TEST_MAP_CONST",
			"group_id": 7,
			"groupId": 7,
			"map_id": 42,
			"mapId": 42,
			"width": 5,
			"height": 4,
		},
		"current_map_payload": {
			"source": "smoke",
			"map_name": "Test Map",
			"blocks_label": "TestBlocks",
		},
		"current_spawn_point": {
			"map_name": "Test Map",
			"map_constant": "TEST_MAP_CONST",
			"group_id": 7,
			"map_id": 42,
			"player_tile": {"x": 2, "y": 3},
			"facing": "up",
		},
	}
	if not bool(map_model.call("from_dictionary", map_payload)):
		push_error("smoke_test: map data from_dictionary failed")
		quit(1)
		return
	var map_snapshot: Dictionary = Dictionary(map_model.call("to_dictionary"))
	if str(map_snapshot.get("current_map_key", "")) != "TEST_MAP" or int(map_snapshot.get("current_width", -1)) != 5 or int(map_snapshot.get("current_height", -1)) != 4:
		push_error("smoke_test: map data did not round-trip")
		quit(1)
		return
	if str(Dictionary(map_snapshot.get("current_map_summary", {})).get("map_key", "")) != "TEST_MAP":
		push_error("smoke_test: map summary did not round-trip")
		quit(1)
		return
	var map_identity_summary: Dictionary = Dictionary(map_snapshot.get("current_map_summary", {}))
	if str(map_identity_summary.get("map_name", "")) != "Test Map" or str(map_identity_summary.get("map_constant", "")) != "TEST_MAP_CONST" or int(map_identity_summary.get("group_id", -1)) != 7 or int(map_identity_summary.get("map_id", -1)) != 42:
		push_error("smoke_test: map summary identity fields did not round-trip")
		quit(1)
		return
	if str(Dictionary(map_snapshot.get("current_map_payload", {})).get("source", "")) != "smoke":
		push_error("smoke_test: map payload did not round-trip")
		quit(1)
		return
	if int(Dictionary(map_snapshot.get("asset_summary", {})).get("content_pack_count", -1)) != 2:
		push_error("smoke_test: map asset summary did not round-trip")
		quit(1)
		return
	if str(Dictionary(Dictionary(map_snapshot.get("runtime_map_metadata", {})).get("TEST_MAP", {})).get("constant", "")) != "TEST_MAP_CONST":
		push_error("smoke_test: map runtime metadata did not round-trip")
		quit(1)
		return
	if int(Dictionary(Dictionary(map_snapshot.get("runtime_spawn_points", {})).get("spawn-test", {})).get("map_id", -1)) != 42:
		push_error("smoke_test: map runtime spawn points did not round-trip")
		quit(1)
		return
	if int(Dictionary(Dictionary(map_snapshot.get("map_attributes", {})).get("Test Map", {})).get("group_id", -1)) != 7:
		push_error("smoke_test: map attributes did not round-trip")
		quit(1)
		return
	if int(Array(Dictionary(Dictionary(map_snapshot.get("map_blocks", {})).get("TestBlocks", {})).get("blocks", [])).size()) != 3:
		push_error("smoke_test: map blocks did not round-trip")
		quit(1)
		return
	if int(map_snapshot.get("current_group_id", -1)) != 7 or int(map_snapshot.get("current_map_group_id", -1)) != 7 or int(map_snapshot.get("current_map_id", -1)) != 42:
		push_error("smoke_test: map group ids did not round-trip")
		quit(1)
		return
	var map_spawn_snapshot: Dictionary = Dictionary(map_snapshot.get("current_spawn_point", {}))
	var map_spawn_tile: Dictionary = Dictionary(map_spawn_snapshot.get("player_tile", {}))
	if int(map_spawn_tile.get("x", -1)) != 2 or int(map_spawn_tile.get("y", -1)) != 3:
		push_error("smoke_test: map spawn tile did not round-trip")
		quit(1)
		return
	if str(map_spawn_snapshot.get("facing", "")) != "up":
		push_error("smoke_test: map spawn facing did not round-trip")
		quit(1)
		return
	if str(map_snapshot.get("current_blocks_label", "")) != "TestBlocks" or str(map_snapshot.get("current_map_events_label", "")) != "Test_MapEvents" or str(map_snapshot.get("current_map_scripts_label", "")) != "Test_MapScripts":
		push_error("smoke_test: map labels did not round-trip")
		quit(1)
		return
	map_model.call("refresh_assets")
	var loaded_map_metadata: Dictionary = Dictionary(map_model.call("load_runtime_map_metadata"))
	var loaded_spawn_points: Dictionary = Dictionary(map_model.call("load_runtime_spawn_points"))
	var loaded_map_attributes: Dictionary = Dictionary(map_model.call("load_map_attributes"))
	var loaded_map_blocks: Dictionary = Dictionary(map_model.call("load_map_blocks"))
	if loaded_map_metadata.is_empty() or loaded_spawn_points.is_empty() or loaded_map_attributes.is_empty() or loaded_map_blocks.is_empty():
		push_error("smoke_test: map data public asset loaders returned empty data")
		quit(1)
		return
	if not bool(map_model.call("load_default_map")):
		push_error("smoke_test: map data failed to load default map")
		quit(1)
		return
	var live_map_snapshot: Dictionary = Dictionary(map_model.call("to_dictionary"))
	if str(live_map_snapshot.get("current_map_key", "")).is_empty() or int(live_map_snapshot.get("current_width", 0)) <= 0 or int(live_map_snapshot.get("current_height", 0)) <= 0:
		push_error("smoke_test: map data default map did not expose dimensions")
		quit(1)
		return
	if not map_model.has_method("get_available_map_keys") or not map_model.has_method("get_selected_map_key") or not map_model.has_method("get_selected_map_index") or not map_model.has_method("get_map_manifest") or not map_model.has_method("get_spawn_summary") or not map_model.has_method("get_current_map_block_key"):
		push_error("smoke_test: map data missing selector or manifest helpers")
		quit(1)
		return
	var map_available_keys: Array = Array(map_model.call("get_available_map_keys"))
	if map_available_keys.is_empty():
		push_error("smoke_test: map data available map keys were empty")
		quit(1)
		return
	var selected_map_key := str(map_model.call("get_selected_map_key"))
	if selected_map_key.is_empty() or selected_map_key != str(live_map_snapshot.get("current_map_key", "")):
		push_error("smoke_test: map data selected map key did not match the active map")
		quit(1)
		return
	if int(map_model.call("get_selected_map_index")) < 0:
		push_error("smoke_test: map data selected map index was invalid")
		quit(1)
		return
	var map_manifest: Dictionary = Dictionary(map_model.call("get_map_manifest"))
	if map_manifest.is_empty() or not map_manifest.has(selected_map_key):
		push_error("smoke_test: map data manifest did not contain the selected map")
		quit(1)
		return
	var live_map_summary: Dictionary = Dictionary(map_model.call("get_map_summary"))
	if str(live_map_summary.get("map_key", "")).is_empty() or str(live_map_summary.get("map_name", "")).is_empty() or str(live_map_summary.get("map_constant", "")).is_empty():
		push_error("smoke_test: imported map summary missing normalized identity strings")
		quit(1)
		return
	if int(live_map_summary.get("group_id", -1)) < 0 or int(live_map_summary.get("map_id", -1)) < 0:
		push_error("smoke_test: imported map summary missing normalized numeric identity")
		quit(1)
		return
	var exported_map_summary: Dictionary = Dictionary(map_model.call("get_map_summary"))
	if str(exported_map_summary.get("map_key", "")) != selected_map_key or str(exported_map_summary.get("map_name", "")).is_empty():
		push_error("smoke_test: map summary helper did not expose the selected asset")
		quit(1)
		return
	var exported_spawn_summary: Dictionary = Dictionary(map_model.call("get_spawn_summary"))
	if exported_spawn_summary.is_empty() or str(exported_spawn_summary.get("map_name", "")).is_empty():
		push_error("smoke_test: spawn summary helper did not expose the selected asset")
		quit(1)
		return
	if str(map_model.call("get_current_map_block_key")).is_empty():
		push_error("smoke_test: current map block key helper returned empty data")
		quit(1)
		return
	var live_payload: Dictionary = Dictionary(map_model.call("build_map_payload", live_map_summary.get("map_key", "")))
	if str(live_payload.get("map_key", "")) != str(live_map_summary.get("map_key", "")) or str(live_payload.get("map_constant", "")) != str(live_map_summary.get("map_constant", "")):
		push_error("smoke_test: imported map payload identity did not match summary")
		quit(1)
		return
	var live_block_metadata: Dictionary = Dictionary(map_model.call("get_current_block_metadata"))
	if str(live_block_metadata.get("map_key", "")) != str(live_map_summary.get("map_key", "")) or int(live_block_metadata.get("expected_block_count", 0)) <= 0:
		push_error("smoke_test: imported block metadata missing current map identity/count")
		quit(1)
		return
	var live_block_zero: Dictionary = Dictionary(map_model.call("get_block_at_tile", 0, 0))
	if int(live_block_zero.get("x", -1)) != 0 or int(live_block_zero.get("y", -1)) != 0 or not live_block_zero.has("block_id"):
		push_error("smoke_test: imported block tile lookup missing concrete block identity")
		quit(1)
		return
	var live_tileset_metadata: Dictionary = Dictionary(map_model.call("get_current_tileset_metadata"))
	if str(live_tileset_metadata.get("tileset_name", "")).is_empty() or int(live_tileset_metadata.get("metatile_count", -1)) < 0:
		push_error("smoke_test: imported tileset metadata missing current identity/counts")
		quit(1)
		return
	var live_event_payloads: Dictionary = Dictionary(map_model.call("get_event_command_payloads_at_tile", 0, 0))
	if int(live_event_payloads.get("x", -1)) != 0 or int(live_event_payloads.get("y", -1)) != 0 or not live_event_payloads.has("warps") or not live_event_payloads.has("object_events"):
		push_error("smoke_test: imported map event payload lookup missing stable fields")
		quit(1)
		return
	var live_object_records: Array = Array(map_model.call("get_current_object_event_records"))
	var live_warp_targets: Dictionary = Dictionary(map_model.call("get_warp_targets"))
	if live_object_records.size() < 0 or live_warp_targets.size() < 0:
		push_error("smoke_test: imported object/warp lookup returned invalid collections")
		quit(1)
		return
	var live_map_file: Dictionary = Dictionary(map_model.call("load_map_file", str(live_map_snapshot.get("current_map_name", ""))))
	if live_map_file.is_empty():
		push_error("smoke_test: map data public map file lookup returned empty data")
		quit(1)
		return
	var spawn_keys: Array = loaded_spawn_points.keys()
	if spawn_keys.is_empty():
		push_error("smoke_test: map data spawn manifest missing keys")
		quit(1)
		return
	var first_spawn: Dictionary = Dictionary(loaded_spawn_points.get(spawn_keys[0], {}))
	if first_spawn.is_empty() or not bool(map_model.call("apply_spawn_point", first_spawn)):
		push_error("smoke_test: map data failed to apply a loaded spawn point")
		quit(1)
		return
	var spawn_map_snapshot: Dictionary = Dictionary(map_model.call("to_dictionary"))
	if Dictionary(spawn_map_snapshot.get("current_spawn_point", {})).is_empty() or str(spawn_map_snapshot.get("current_map_key", "")).is_empty():
		push_error("smoke_test: map data did not persist applied spawn metadata")
		quit(1)
		return
	var spawn_matches: Array = Array(map_model.call("get_spawn_points_for_map", spawn_map_snapshot.get("current_map_key", "")))
	if spawn_matches.is_empty():
		push_error("smoke_test: map data spawn lookup returned no matches for current map")
		quit(1)
		return
	var game_state_script: Script = load("res://scripts/game_state.gd")
	if game_state_script == null:
		push_error("smoke_test: failed to load game state script")
		quit(1)
		return
	var game_state_model: Variant = game_state_script.new()
	var original_game_state: Dictionary = Dictionary(game_state_model.call("get_state"))
	if original_game_state.is_empty():
		push_error("smoke_test: game state snapshot missing")
		quit(1)
		return
	var game_state_snapshot := original_game_state.duplicate(true)
	game_state_snapshot["active_scene"] = "battle"
	game_state_snapshot["scene_route"] = "battle"
	game_state_snapshot["frame_counter"] = 222
	game_state_snapshot["has_seen_intro"] = true
	game_state_snapshot["scene_history"] = ["ui_shell", "overworld", "battle"]
	game_state_snapshot["scene_context"] = {"route": "battle", "mode": "combat", "frame_counter": 222}
	game_state_snapshot["scene_handoff"] = {"from_scene": "overworld", "to_scene": "battle", "reason": "smoke"}
	game_state_snapshot["pending_scene_handoff"] = {"from_scene": "battle", "to_scene": "ui_shell", "reason": "pending"}
	game_state_snapshot["loaded_asset_summary"] = {"pokemon_count": 1, "move_count": 2}
	game_state_snapshot["ui_page"] = "oak_intro"
	game_state_snapshot["ui_dialogue_state"] = {
		"active": true,
		"visible": true,
		"page_index": 1,
		"page_count": 2,
		"text": "Round trip",
		"current_text": "Round trip",
		"visible_text": "Round trip",
	}
	game_state_snapshot["ui_menu_state"] = {
		"menu_open": true,
		"input_locked": true,
		"depth": 1,
		"stack": [{"id": "main_menu", "title": "Main Menu"}],
	}
	game_state_snapshot["ui_shell_state"] = {
		"ui_page": "oak_intro",
		"text_box": {
			"active": true,
			"visible": true,
			"page_index": 1,
			"page_count": 2,
			"current_text": "Round trip",
		},
		"menu_stack": {
			"menu_open": true,
			"input_locked": true,
			"depth": 1,
		},
		"page_snapshots": {
			"oak_intro": {"current_text": "Round trip"},
		},
	}
	game_state_snapshot["overworld_state"] = {
		"map_name": "Test Map",
		"current_map_block_key": "TEST_MAP_BLOCKS",
		"pending_move": "left",
	}
	game_state_snapshot["battle_state"] = {
		"battle_id": "battle-001",
		"turn_phase": "resolution",
		"phase_history": ["setup", "turn_prompt", "resolution"],
	}
	game_state_snapshot["sram"] = {
		"options": {
			"text_speed": "slow",
			"battle_scene": false,
			"battle_style": "set",
			"sound": "mono",
			"menu_account": false,
			"frame": 3,
		},
		"party": {"pokemon": [null, null, null, null, null, null]},
		"link_battle_stats": {"wins": 1, "losses": 2, "draws": 3},
		"badges": {"johto": [true, false, false, false, false, false, false, false], "kanto": [false, true, false, false, false, false, false, false]},
	}
	game_state_snapshot["wram"] = {"scene": "battle", "flags": {"test": true}, "variables": {"step": 1}}
	game_state_snapshot["wram"]["scene_route"] = "battle"
	game_state_snapshot["wram"]["scene_transition"] = {"from": "overworld", "to": "battle", "reason": "smoke"}
	game_state_snapshot["vram"] = {"palette_bank": 3, "tile_cache_ready": true}
	game_state_snapshot["hram"] = {"joypad": {"hJoypadReleased": 1, "hJoypadPressed": 2, "hJoypadDown": 3, "hJoypadSum": 4, "hJoyReleased": 5, "hJoyPressed": 6, "hJoyDown": 7, "hJoyLast": 8}, "hardware_divider": 9, "hRandomAdd": 10, "hRandomSub": 11}
	game_state_snapshot["battle"] = {
		"active": true,
		"kind": "wild",
		"phase": "intro",
		"turn": 2,
		"last_command": "attack",
		"result": "win",
		"player": {"name": "CHRIS", "hp": 25, "max_hp": 30, "status": "ok", "fainted": false},
		"opponent": {"name": "PIDGEY", "hp": 0, "max_hp": 18, "status": "fainted", "fainted": true},
	}
	game_state_snapshot["ui"] = {
		"open": true,
		"screen": "menu",
		"cursor": {"index": 2, "row": 1, "column": 0},
		"stack": [{"id": "main_menu", "title": "Main Menu"}],
		"dialogue": {"open": true, "page": 1, "speaker": "TEST", "prompt": "Continue?"},
	}
	var game_state_overworld: Dictionary = Dictionary(game_state_snapshot.get("overworld", {}))
	game_state_overworld["player"] = {"x": 9, "y": 4, "facing": "left", "moving": true, "surfing": false, "biking": false}
	game_state_snapshot["overworld"] = game_state_overworld
	game_state_snapshot["gameplay"] = {
		"mode": "battle",
		"overworld": game_state_overworld.duplicate(true),
		"battle": game_state_snapshot["battle"].duplicate(true),
		"ui": game_state_snapshot["ui"].duplicate(true),
		"menu": game_state_snapshot["ui"].duplicate(true),
		"progress": {"story_flags": {"intro_done": true}, "event_flags": {"demo": true}, "badges": {"johto": [true, false, false, false, false, false, false, false], "kanto": [false, true, false, false, false, false, false, false]}, "money": 1234, "play_time_frames": 567},
	}
	if not bool(game_state_model.call("from_dictionary", game_state_snapshot)):
		push_error("smoke_test: game state from_dictionary failed")
		quit(1)
		return
	var restored_game_state: Dictionary = Dictionary(game_state_model.call("get_state"))
	if str(restored_game_state.get("active_scene", "")) != "battle":
		push_error("smoke_test: game state active scene did not round-trip")
		quit(1)
		return
	if int(restored_game_state.get("frame_counter", -1)) != 222 or not bool(restored_game_state.get("has_seen_intro", false)):
		push_error("smoke_test: game state core flags did not round-trip")
		quit(1)
		return
	if str(restored_game_state.get("scene_route", "")) != "battle":
		push_error("smoke_test: game state scene route did not round-trip")
		quit(1)
		return
	if str(Array(restored_game_state.get("scene_history", [])).back()) != "battle":
		push_error("smoke_test: game state scene history did not round-trip")
		quit(1)
		return
	if str(Dictionary(restored_game_state.get("scene_handoff", {})).get("reason", "")) != "smoke" or str(Dictionary(restored_game_state.get("pending_scene_handoff", {})).get("reason", "")) != "pending":
		push_error("smoke_test: game state handoff fields did not round-trip")
		quit(1)
		return
	if str(Dictionary(restored_game_state.get("scene_context", {})).get("mode", "")) != "combat":
		push_error("smoke_test: game state scene context did not round-trip")
		quit(1)
		return
	if int(Dictionary(restored_game_state.get("loaded_asset_summary", {})).get("move_count", -1)) != 2:
		push_error("smoke_test: game state asset summary did not round-trip")
		quit(1)
		return
	if str(game_state_model.call("get_scene_route")) != "battle":
		push_error("smoke_test: game state scene route getter did not round-trip")
		quit(1)
		return
	if str(Dictionary(game_state_model.call("get_scene_handoff")).get("reason", "")) != "smoke" or str(Dictionary(game_state_model.call("get_pending_scene_handoff")).get("reason", "")) != "pending":
		push_error("smoke_test: game state handoff getters did not round-trip")
		quit(1)
		return
	if str(Dictionary(game_state_model.call("get_scene_context")).get("mode", "")) != "combat":
		push_error("smoke_test: game state scene context getter did not round-trip")
		quit(1)
		return
	if int(Dictionary(game_state_model.call("get_loaded_asset_summary")).get("move_count", -1)) != 2:
		push_error("smoke_test: game state loaded asset summary getter did not round-trip")
		quit(1)
		return
	if str(game_state_model.call("get_ui_page")) != "oak_intro":
		push_error("smoke_test: game state ui page getter did not round-trip")
		quit(1)
		return
	if str(Dictionary(game_state_model.call("get_ui_shell_state")).get("ui_page", "")) != "oak_intro":
		push_error("smoke_test: game state ui shell state getter did not round-trip")
		quit(1)
		return
	if str(restored_game_state.get("ui_page", "")) != "oak_intro":
		push_error("smoke_test: game state ui page did not round-trip")
		quit(1)
		return
	if int(Dictionary(restored_game_state.get("ui_dialogue_state", {})).get("page_count", -1)) != 2:
		push_error("smoke_test: game state ui dialogue state did not round-trip")
		quit(1)
		return
	if not bool(Dictionary(restored_game_state.get("ui_menu_state", {})).get("menu_open", false)):
		push_error("smoke_test: game state ui menu state did not round-trip")
		quit(1)
		return
	if str(Dictionary(restored_game_state.get("ui_shell_state", {})).get("ui_page", "")) != "oak_intro":
		push_error("smoke_test: game state ui shell state did not round-trip")
		quit(1)
		return
	if str(Dictionary(restored_game_state.get("overworld_state", {})).get("current_map_block_key", "")) != "TEST_MAP_BLOCKS":
		push_error("smoke_test: game state overworld state did not round-trip")
		quit(1)
		return
	if str(Dictionary(restored_game_state.get("battle_state", {})).get("battle_id", "")) != "battle-001":
		push_error("smoke_test: game state battle state did not round-trip")
		quit(1)
		return
	var restored_sram: Dictionary = Dictionary(restored_game_state.get("sram", {}))
	var restored_options: Dictionary = Dictionary(restored_sram.get("options", {}))
	if str(restored_options.get("text_speed", "")) != "slow" or int(restored_options.get("frame", -1)) != 3:
		push_error("smoke_test: game state sram options did not round-trip")
		quit(1)
		return
	var restored_wram: Dictionary = Dictionary(restored_game_state.get("wram", {}))
	if str(restored_wram.get("scene", "")) != "battle" or str(restored_wram.get("scene_route", "")) != "battle" or int(Dictionary(restored_wram.get("flags", {})).get("test", false)) != 1:
		push_error("smoke_test: game state wram payload did not round-trip")
		quit(1)
		return
	if str(Dictionary(restored_wram.get("scene_transition", {})).get("reason", "")) != "smoke":
		push_error("smoke_test: game state wram scene transition did not round-trip")
		quit(1)
		return
	var restored_vram: Dictionary = Dictionary(restored_game_state.get("vram", {}))
	if int(restored_vram.get("palette_bank", -1)) != 3 or not bool(restored_vram.get("tile_cache_ready", false)):
		push_error("smoke_test: game state vram payload did not round-trip")
		quit(1)
		return
	var restored_hram: Dictionary = Dictionary(restored_game_state.get("hram", {}))
	var restored_joypad: Dictionary = Dictionary(restored_hram.get("joypad", {}))
	if int(restored_joypad.get("hJoyDown", -1)) != 7 or int(restored_hram.get("hardware_divider", -1)) != 9:
		push_error("smoke_test: game state hram payload did not round-trip")
		quit(1)
		return
	if str(Dictionary(restored_game_state.get("battle", {})).get("result", "")) != "win":
		push_error("smoke_test: game state battle payload did not round-trip")
		quit(1)
		return
	if str(Dictionary(restored_game_state.get("ui", {})).get("screen", "")) != "menu":
		push_error("smoke_test: game state ui payload did not round-trip")
		quit(1)
		return
	if str(Dictionary(restored_game_state.get("gameplay", {})).get("mode", "")) != "battle":
		push_error("smoke_test: game state gameplay payload did not round-trip")
		quit(1)
		return
	if str(Dictionary(Dictionary(restored_game_state.get("overworld", {})).get("player", {})).get("facing", "")) != "left":
		push_error("smoke_test: game state overworld payload did not round-trip")
		quit(1)
		return
	var save_store_model: Variant = save_store_script_preflight.new()
	save_store_model.call("set_save_root", "/private/tmp/crystal-llm-godot/smoke-state")
	if not bool(save_store_model.call("save_game", "state-parity", game_state_model)):
		push_error("smoke_test: save store failed to save game state")
		quit(1)
		return
	var saved_metadata: Dictionary = Dictionary(save_store_model.call("load_save_metadata", "state-parity"))
	if str(saved_metadata.get("slot", "")) != "state-parity.sav" or str(saved_metadata.get("kind", "")) != "custom" or str(saved_metadata.get("saved_at", "")).is_empty():
		push_error("smoke_test: save store metadata sidecar missing slot/kind/timestamp")
		quit(1)
		return
	var save_history: Array = Array(save_store_model.call("save_history", "state-parity"))
	if save_history.is_empty() or not bool(Dictionary(save_history[0]).get("is_current", false)) or str(Dictionary(save_history[0]).get("slot", "")) != "state-parity.sav":
		push_error("smoke_test: save store history missing current save metadata")
		quit(1)
		return
	var saved_state_result: Dictionary = Dictionary(save_store_model.call("load_game", "state-parity"))
	if not bool(saved_state_result.get("ok", false)):
		push_error("smoke_test: save store failed to load game state")
		quit(1)
		return
	var saved_state_object: Variant = saved_state_result.get("state", null)
	if saved_state_object == null or not saved_state_object.has_method("get_state"):
		push_error("smoke_test: save store returned invalid game state object")
		quit(1)
		return
	var saved_state_snapshot: Dictionary = Dictionary(saved_state_object.call("get_state"))
	if str(saved_state_snapshot.get("scene_route", "")) != "battle" or str(Dictionary(saved_state_snapshot.get("scene_handoff", {})).get("reason", "")) != "smoke":
		push_error("smoke_test: saved game state did not preserve coordinator fields")
		quit(1)
		return
	var loaded_save_metadata: Dictionary = Dictionary(saved_state_snapshot.get("save_metadata", {}))
	if str(loaded_save_metadata.get("slot", "")) != "state-parity.sav" or str(loaded_save_metadata.get("kind", "")) != "custom" or int(loaded_save_metadata.get("frame_counter", -1)) != 222:
		push_error("smoke_test: loaded game state did not preserve embedded save metadata")
		quit(1)
		return
	if str(Dictionary(Dictionary(saved_state_snapshot.get("wram", {})).get("scene_transition", {})).get("reason", "")) != "smoke":
		push_error("smoke_test: saved game state did not preserve scene transition")
		quit(1)
		return
	if not bool(save_store_model.call("delete_save_game", "state-parity")):
		push_error("smoke_test: save store cleanup failed")
		quit(1)
		return
	game_state_model.call("from_state", original_game_state)
	var battle_assets_script: Script = load("res://scripts/battle_assets.gd")
	if battle_assets_script == null:
		push_error("smoke_test: failed to load battle assets script")
		quit(1)
		return
	var battle_assets_model: Variant = battle_assets_script.new()
	battle_assets_model.call("ensure_loaded")
	var battle_assets_snapshot: Dictionary = Dictionary(battle_assets_model.call("load_summary"))
	if int(battle_assets_snapshot.get("content_pack_version", -1)) < 0 or int(battle_assets_snapshot.get("content_pack_count", -1)) < 0:
		push_error("smoke_test: battle assets summary missing pack fields")
		quit(1)
		return
	if int(battle_assets_snapshot.get("pokemon_count", -1)) < 0 or int(battle_assets_snapshot.get("move_count", -1)) < 0:
		push_error("smoke_test: battle assets summary missing core counts")
		quit(1)
		return
	if Array(battle_assets_model.get("pokemon_data")).is_empty() or Dictionary(battle_assets_model.get("move_data")).is_empty() or Array(battle_assets_model.get("trainer_data")).is_empty():
		push_error("smoke_test: battle assets did not populate loaded datasets")
		quit(1)
		return
	if not Dictionary(battle_assets_model.get("move_data")).has("POUND"):
		push_error("smoke_test: battle assets move lookup missing loaded move key")
		quit(1)
		return
	var first_loaded_pokemon: Dictionary = Dictionary(Array(battle_assets_model.get("pokemon_data"))[0])
	if str(first_loaded_pokemon.get("id", "")).is_empty() or int(first_loaded_pokemon.get("int_id", 0)) <= 0:
		push_error("smoke_test: battle assets pokemon lookup missing identity fields")
		quit(1)
		return
	var first_loaded_trainer: Dictionary = Dictionary(Array(battle_assets_model.get("trainer_data"))[0])
	if str(first_loaded_trainer.get("trainer_id", "")).is_empty() or Array(first_loaded_trainer.get("party", [])).is_empty():
		push_error("smoke_test: battle assets trainer lookup missing party fields")
		quit(1)
		return
	if Array(battle_assets_model.call("load_battle_animation_table")).is_empty() or Dictionary(battle_assets_model.call("load_battle_anim_bundle")).is_empty():
		push_error("smoke_test: battle asset lookup returned empty animation manifests")
		quit(1)
		return
	var battle_tile_image: Image = battle_assets_model.call("load_1bpp_tile", "battle/expbar.1bpp")
	if battle_tile_image.get_width() != 8 or battle_tile_image.get_height() != 8:
		push_error("smoke_test: battle assets failed to decode 1bpp battle tile")
		quit(1)
		return
	var battle_tiles: Array = Array(battle_assets_model.call("load_2bpp_tiles", "battle/balls.2bpp"))
	if battle_tiles.is_empty() or (battle_tiles[0] as Image).get_width() != 8:
		push_error("smoke_test: battle assets failed to decode 2bpp battle tiles")
		quit(1)
		return
	var battle_palette: Array = Array(battle_assets_model.call("load_palette", "battle/hp_bar.pal"))
	if battle_palette.size() != 4:
		push_error("smoke_test: battle assets failed to load battle palette")
		quit(1)
		return
	var input_latch_script: Script = load("res://scripts/input_latch.gd")
	if input_latch_script == null:
		push_error("smoke_test: failed to load input latch script")
		quit(1)
		return
	var input_latch_model: Variant = input_latch_script.new()
	input_latch_model.call("queue_button", "a", true)
	input_latch_model.call("queue_button", "a", false)
	var first_input_frame: Dictionary = Dictionary(input_latch_model.call("begin_frame"))
	if not bool(Dictionary(first_input_frame.get("pressed", {})).get("a", false)) or not bool(Dictionary(first_input_frame.get("released", {})).get("a", false)):
		push_error("smoke_test: input latch did not report press/release")
		quit(1)
		return
		if bool(input_latch_model.call("is_down", "a")):
			push_error("smoke_test: input latch did not clear held button")
			quit(1)
			return
		input_latch_model.call("queue_button", "a", true)
		var second_input_frame: Dictionary = Dictionary(input_latch_model.call("begin_frame"))
		if not bool(Dictionary(second_input_frame.get("pressed", {})).get("a", false)) or not bool(input_latch_model.call("is_down", "a")):
			push_error("smoke_test: input latch did not hold button")
			quit(1)
			return
		input_latch_model.call("queue_button", "a", true)
		var repeated_hold_frame: Dictionary = Dictionary(input_latch_model.call("begin_frame"))
		if bool(Dictionary(repeated_hold_frame.get("pressed", {})).get("a", false)) or not bool(Dictionary(repeated_hold_frame.get("down", {})).get("a", false)) or int(repeated_hold_frame.get("pressed_mask", -1)) != 0:
			push_error("smoke_test: input latch repeated held press produced a false edge")
			quit(1)
			return
		input_latch_model.call("queue_button", "a", false)
		var third_input_frame: Dictionary = Dictionary(input_latch_model.call("begin_frame"))
		if not bool(Dictionary(third_input_frame.get("released", {})).get("a", false)) or bool(input_latch_model.call("is_down", "a")):
			push_error("smoke_test: input latch did not release button")
			quit(1)
			return
		if int(input_latch_model.call("pressed_mask")) != 0 or int(input_latch_model.call("released_mask")) != 0x10 or int(input_latch_model.call("down_mask")) != 0:
			push_error("smoke_test: input latch masks did not reflect release edge")
			quit(1)
			return
		if not bool(input_latch_model.call("from_dictionary", {
			"frame_index": 8,
			"hJoypadDown": 0x91,
			"hJoypadPressed": 0x10,
			"hJoypadReleased": 0x80,
		})):
			push_error("smoke_test: input latch joypad snapshot restore failed")
			quit(1)
			return
		if int(input_latch_model.call("frame_index")) != 8 or int(input_latch_model.call("down_mask")) != 0x91 or int(input_latch_model.call("pressed_mask")) != 0x10 or int(input_latch_model.call("released_mask")) != 0x80:
			push_error("smoke_test: input latch joypad masks did not round-trip")
			quit(1)
			return
		if not bool(input_latch_model.call("is_down", "right")) or not bool(input_latch_model.call("is_down", "a")) or not bool(input_latch_model.call("is_down", "start")):
			push_error("smoke_test: input latch restored held buttons from mask incorrectly")
			quit(1)
			return
		input_latch_model.call("queue_button", "not-a-button", true)
		input_latch_model.call("queue_button", "select", true)
		input_latch_model.call("queue_button", "right", false)
		var restored_input_frame: Dictionary = Dictionary(input_latch_model.call("begin_frame"))
		if int(restored_input_frame.get("frame_index", -1)) != 9 or int(restored_input_frame.get("down_mask", -1)) != 0xd0 or not bool(Dictionary(restored_input_frame.get("pressed", {})).get("select", false)) or not bool(Dictionary(restored_input_frame.get("released", {})).get("right", false)):
			push_error("smoke_test: input latch queued edge cases did not produce stable masks")
			quit(1)
			return
	var repo_paths_script: Script = load("res://scripts/repo_paths.gd")
	if repo_paths_script == null:
		push_error("smoke_test: failed to load repo paths script")
		quit(1)
		return
	var repo_root := str(repo_paths_script.call("repo_root"))
	var web_assets_root := str(repo_paths_script.call("web_assets_root"))
	if repo_root.is_empty() or web_assets_root.is_empty():
		push_error("smoke_test: repo paths did not resolve roots")
		quit(1)
		return
	if str(repo_paths_script.call("project_dir")).is_empty() or str(repo_paths_script.call("data_root")).is_empty() or str(repo_paths_script.call("gfx_root")).is_empty():
		push_error("smoke_test: repo paths did not resolve asset roots")
		quit(1)
		return
	var title_runtime_script: Script = load("res://scripts/title_runtime.gd")
	var title_screen_script: Script = load("res://scripts/title_screen.gd")
	var continue_screen_script: Script = load("res://scripts/continue_screen.gd")
	var day_of_week_screen_script: Script = load("res://scripts/day_of_week_screen.gd")
	var name_entry_script: Script = load("res://scripts/name_entry.gd")
	var oak_intro_script: Script = load("res://scripts/oak_intro.gd")
	var intro_sequence_script: Script = load("res://scripts/intro_sequence.gd")
	var clock_reset_screen_script: Script = load("res://scripts/clock_reset_screen.gd")
	var delete_save_screen_script: Script = load("res://scripts/delete_save_screen.gd")
	if title_runtime_script == null or title_screen_script == null or continue_screen_script == null or day_of_week_screen_script == null or name_entry_script == null or oak_intro_script == null or intro_sequence_script == null or clock_reset_screen_script == null or delete_save_screen_script == null:
		push_error("smoke_test: failed to load boot screen scripts")
		quit(1)
		return
	var title_runtime_model: Variant = title_runtime_script.new()
	if not bool(title_runtime_model.call("from_dictionary", {
		"screen": "title",
		"phase": "timeout",
		"phase_frame": 11,
		"frame_counter": 77,
		"title_timer": 180,
		"input_gate_frames": 8,
		"route_queue_hold_frames": 12,
		"pending_action": "intro_sequence",
		"pending_action_payload": {
			"action_id": "title_timeout",
			"route": "intro_sequence",
			"source_screen": "title",
			"phase": "timeout",
			"frame_counter": 77,
			"tick_counter": 4,
			"selected_option": "timeout",
		},
		"route_entry": true,
		"last_input": {"pressed": {"start": true}, "released": {}, "down": {"start": true}},
	})):
		push_error("smoke_test: title runtime snapshot restore failed")
		quit(1)
		return
	var restored_title_runtime: Dictionary = Dictionary(title_runtime_model.call("to_dictionary"))
	if str(restored_title_runtime.get("phase", "")) != "main" or str(restored_title_runtime.get("pending_action", "")) != "" or bool(restored_title_runtime.get("input_locked", true)):
		push_error("smoke_test: title runtime route-entry restore did not normalize state")
		quit(1)
		return
	if not Dictionary(restored_title_runtime.get("pending_action_payload", {})).is_empty():
		push_error("smoke_test: title runtime route-entry restore did not clear pending payload")
		quit(1)
		return
	if not bool(title_runtime_model.call("can_accept_input")):
		push_error("smoke_test: title runtime route-entry restore did not unlock input")
		quit(1)
		return
	var title_new_game_model: Variant = title_runtime_script.new()
	if not bool(title_new_game_model.call("from_dictionary", {
		"screen": "title",
		"phase": "main",
		"phase_frame": 8,
		"frame_counter": 12,
		"title_timer": 12,
		"logo_palette_phase": "steady",
		"suicune_palette_phase": "steady",
		"suicune_frame": 1,
		"suicune_animation_timer": 3,
		"input_gate_frames": 8,
		"route_queue_hold_frames": 12,
		"pending_action": "",
		"pending_action_payload": {},
	})):
		push_error("smoke_test: title runtime new-game setup failed")
		quit(1)
		return
	var title_new_game_event := InputEventAction.new()
	title_new_game_event.action = "game_a"
	title_new_game_event.pressed = true
	title_new_game_model.call("_handle_boot_input", title_new_game_event)
	var title_new_game_state: Dictionary = Dictionary(title_new_game_model.call("get_state"))
	var title_new_game_payload: Dictionary = Dictionary(title_new_game_state.get("pending_action_payload", {}))
	if str(title_new_game_state.get("pending_action", "")) != "intro_sequence" or str(title_new_game_payload.get("action_id", "")) != "title_new_game" or str(title_new_game_payload.get("route", "")) != "intro_sequence" or str(title_new_game_payload.get("selected_option", "")) != "new_game":
		push_error("smoke_test: title runtime new-game payload was malformed")
		quit(1)
		return
	if Array(title_new_game_payload.get("boot_flow_path", [])) != ["intro_sequence", "clock_reset_screen", "day_of_week_screen", "oak_intro", "name_entry"]:
		push_error("smoke_test: title runtime new-game payload did not include the boot flow path")
		quit(1)
		return
	_assert_boot_state_round_trip(
		"title runtime visual restore",
		title_runtime_script,
		{
			"screen": "title",
			"phase": "main",
			"phase_frame": 11,
			"frame_counter": 77,
			"title_timer": 77,
			"logo_palette_phase": "steady",
			"suicune_palette_phase": "steady",
			"suicune_frame": 2,
			"suicune_animation_timer": 5,
			"input_gate_frames": 8,
			"route_queue_hold_frames": 12,
			"pending_action": "",
			"pending_action_payload": {},
		},
		{
			"phase": "main",
			"phase_frame": 11,
			"frame_counter": 77,
			"title_timer": 77,
			"logo_palette_phase": "steady",
			"suicune_palette_phase": "steady",
			"suicune_frame": 2,
			"suicune_animation_timer": 5,
			"input_gate_frames": 8,
			"route_queue_hold_frames": 12,
		}
	)
	var title_timeout_model: Variant = title_runtime_script.new()
	if not bool(title_timeout_model.call("from_dictionary", {
		"screen": "title",
		"phase": "timeout",
		"phase_frame": 11,
		"frame_counter": 77,
		"title_timer": 180,
		"input_gate_frames": 8,
		"route_queue_hold_frames": 12,
		"attract_timeout_frames": 180,
		"pending_action": "",
		"pending_action_payload": {},
	})):
		push_error("smoke_test: title runtime timeout setup failed")
		quit(1)
		return
	title_timeout_model.call("_tick", 0.016)
	var queued_title_timeout: Dictionary = Dictionary(title_timeout_model.call("get_state"))
	var queued_title_timeout_payload: Dictionary = Dictionary(queued_title_timeout.get("pending_action_payload", {}))
	if str(queued_title_timeout.get("phase", "")) != "exiting" or str(queued_title_timeout.get("pending_action", "")) != "intro_sequence" or str(queued_title_timeout_payload.get("action_id", "")) != "title_timeout" or str(queued_title_timeout_payload.get("route", "")) != "intro_sequence" or str(queued_title_timeout_payload.get("selected_option", "")) != "timeout":
		push_error("smoke_test: title runtime timeout did not queue intro route")
		quit(1)
		return
	if str(queued_title_timeout_payload.get("logo_palette_phase", "")) != "steady" or str(queued_title_timeout_payload.get("suicune_palette_phase", "")) != "steady" or int(queued_title_timeout_payload.get("suicune_frame", -1)) != 2 or int(queued_title_timeout_payload.get("suicune_animation_timer", -1)) != 4:
		push_error("smoke_test: title runtime timeout payload did not preserve palette state")
		quit(1)
		return
	if str(title_timeout_model.call("pop_action")) != "intro_sequence":
		push_error("smoke_test: title runtime queued route was not consumable once")
		quit(1)
		return
	if title_timeout_model.call("pop_action") != null:
		push_error("smoke_test: title runtime queued route was consumed more than once")
		quit(1)
		return
	var title_screen_model: Variant = title_screen_script.new()
	if not bool(title_screen_model.call("from_dictionary", {
		"screen": "title",
		"phase": "timeout",
		"title_timer": 77,
		"pending_action": "intro_sequence",
		"clock_reset_trigger": true,
		"route_entry": true,
		"last_input": {"pressed": {"start": true}, "released": {}, "down": {"start": true}},
	})):
		push_error("smoke_test: title screen snapshot restore failed")
		quit(1)
		return
	var restored_title_screen: Dictionary = Dictionary(title_screen_model.call("to_dictionary"))
	if str(restored_title_screen.get("phase", "")) != "main" or str(restored_title_screen.get("pending_action", "")) != "" or bool(restored_title_screen.get("input_locked", true)):
		push_error("smoke_test: title screen route-entry restore did not normalize state")
		quit(1)
		return
	if not Dictionary(restored_title_screen.get("pending_action_payload", {})).is_empty():
		push_error("smoke_test: title screen route-entry restore did not clear action payload")
		quit(1)
		return
	_assert_boot_action_payload_round_trip(
		"title screen payload restore",
		title_screen_script,
		{
			"screen": "title",
			"phase": "main",
			"title_timer": 34,
			"pending_action": "continue_screen",
			"pending_action_payload": {
				"action_id": "title_continue",
				"route": "continue_screen",
				"source_screen": "title",
				"phase": "main",
				"frame_counter": 34,
				"tick_counter": 8,
				"selected_option": "continue",
			},
			"action_sequence": 5,
		},
		"continue_screen",
		{
			"action_id": "title_continue",
			"route": "continue_screen",
			"selected_option": "continue",
			"phase": "main",
			"frame_counter": 34,
			"tick_counter": 8,
		}
	)
	_assert_boot_action_payload_round_trip(
		"continue screen payload restore",
		continue_screen_script,
		{
			"screen": "continue_screen",
			"selection": 0,
			"confirmed": true,
			"phase": "exiting",
			"phase_frame": 6,
			"prompt_phase": "exiting",
			"prompt_phase_frame": 6,
			"pending_action": "overworld",
			"pending_action_payload": {
				"action_id": "continue_confirm",
				"route": "overworld",
				"source_screen": "continue_screen",
				"phase": "",
				"frame_counter": 12,
				"tick_counter": 3,
				"selected_option": "continue",
				"selection": 0,
				"confirmed": true,
				"prompt_phase": "exiting",
				"prompt_phase_frame": 6,
			},
		},
		"overworld",
		{
			"action_id": "continue_confirm",
			"route": "overworld",
			"selected_option": "continue",
			"selection": 0,
			"confirmed": true,
			"prompt_phase": "exiting",
			"prompt_phase_frame": 6,
		}
	)
	_assert_boot_state_round_trip(
		"continue screen prompt state",
		continue_screen_script,
		{
			"screen": "continue_screen",
			"phase": "opening",
			"phase_frame": 7,
			"prompt_phase": "opening",
			"prompt_phase_frame": 7,
			"selection": 1,
			"confirmed": false,
		},
		{
			"phase": "opening",
			"phase_frame": 7,
			"prompt_phase": "opening",
			"prompt_phase_frame": 7,
			"selection": 1,
			"confirmed": false,
		}
	)
	_assert_boot_action_payload_round_trip(
		"delete save payload restore",
		delete_save_screen_script,
		{
			"screen": "delete_save_screen",
			"selection": 0,
			"confirmed": true,
			"phase": "exiting",
			"phase_frame": 6,
			"prompt_phase": "exiting",
			"prompt_phase_frame": 6,
			"pending_action": "title",
			"pending_action_payload": {
				"action_id": "delete_save_confirm",
				"route": "title",
				"source_screen": "delete_save_screen",
				"selected_option": "yes",
				"selection": 0,
				"confirmed": true,
				"delete_requested": true,
				"prompt_phase": "exiting",
				"prompt_phase_frame": 6,
			},
		},
		"title",
		{
			"action_id": "delete_save_confirm",
			"route": "title",
			"selected_option": "yes",
			"selection": 0,
			"confirmed": true,
			"delete_requested": true,
			"prompt_phase": "exiting",
			"prompt_phase_frame": 6,
		}
	)
	_assert_boot_state_round_trip(
		"delete save prompt state",
		delete_save_screen_script,
		{
			"screen": "delete_save_screen",
			"phase": "opening",
			"phase_frame": 4,
			"prompt_phase": "opening",
			"prompt_phase_frame": 4,
			"selection": 1,
			"confirmed": false,
		},
		{
			"phase": "opening",
			"phase_frame": 4,
			"prompt_phase": "opening",
			"prompt_phase_frame": 4,
			"selection": 1,
			"confirmed": false,
		}
	)
	_assert_boot_action_payload_round_trip(
		"clock reset payload restore",
		clock_reset_screen_script,
		{
			"screen": "clock_reset_screen",
			"phase": "set_minute",
			"selection": 0,
			"day": 6,
			"hour": 23,
			"minute": 45,
			"confirmed": true,
			"pending_action": "title",
			"pending_action_payload": {
				"action_id": "clock_reset_done",
				"route": "title",
				"source_screen": "clock_reset_screen",
				"selected_option": "confirm",
				"phase": "set_minute",
				"selection": 0,
				"day": 6,
				"hour": 23,
				"minute": 45,
				"confirmed": true,
			},
		},
		"title",
		{
			"action_id": "clock_reset_done",
			"route": "title",
			"selected_option": "confirm",
			"selection": 0,
			"day": 6,
			"hour": 23,
			"minute": 45,
			"confirmed": true,
		}
	)
	_assert_boot_action_payload_round_trip(
		"day of week payload restore",
		day_of_week_screen_script,
		{
			"screen": "day_of_week_screen",
			"selected_day": 4,
			"confirmed": true,
			"ignore_confirm_until_release": false,
			"pending_action": "title",
			"pending_action_payload": {
				"action_id": "day_of_week_confirm",
				"route": "title",
				"source_screen": "day_of_week_screen",
				"selected_option": "confirm",
				"selected_day": 4,
				"selected_day_label": "THURS",
				"confirmed": true,
			},
		},
		"title",
		{
			"action_id": "day_of_week_confirm",
			"route": "title",
			"selected_option": "confirm",
			"selected_day": 4,
			"selected_day_label": "THURS",
			"confirmed": true,
		}
	)
	_assert_boot_action_payload_round_trip(
		"name entry payload restore",
		name_entry_script,
		{
			"screen": "name_entry",
			"phase": "finished",
			"phase_frame": 4,
			"cursor_blink_frame": 12,
			"cursor_visible": false,
			"keyboard_page": "upper",
			"name": "CHRIS",
			"cursor_index": 5,
			"cursor_column": 8,
			"cursor_row": 4,
			"cursor_grid_row": 4,
			"cursor_grid_column": 8,
			"case": "upper",
			"finished": true,
			"pending_action": "overworld",
			"pending_action_payload": {
				"action_id": "name_entry_confirm",
				"route": "overworld",
				"source_screen": "name_entry",
				"selected_option": "end",
				"name": "CHRIS",
				"cursor_index": 5,
				"cursor_column": 8,
				"cursor_row": 4,
				"cursor_grid_row": 4,
				"cursor_grid_column": 8,
				"case": "upper",
				"finished": true,
			},
		},
		"overworld",
		{
			"action_id": "name_entry_confirm",
			"route": "overworld",
			"selected_option": "end",
			"name": "CHRIS",
			"cursor_index": 5,
			"cursor_column": 8,
			"cursor_row": 4,
			"cursor_grid_row": 4,
			"cursor_grid_column": 8,
			"case": "upper",
			"finished": true,
			"keyboard_page": "upper",
		}
	)
	_assert_boot_state_round_trip(
		"name entry grid state",
		name_entry_script,
		{
			"screen": "name_entry",
			"phase": "editing",
			"phase_frame": 8,
			"cursor_blink_frame": 13,
			"cursor_visible": true,
			"keyboard_page": "lower",
			"name": "CHRIS",
			"cursor_index": 5,
			"cursor_column": 8,
			"cursor_row": 4,
			"cursor_grid_row": 4,
			"cursor_grid_column": 8,
			"case": "lower",
			"finished": false,
		},
		{
			"phase": "editing",
			"phase_frame": 8,
			"cursor_blink_frame": 13,
			"cursor_visible": true,
			"keyboard_page": "lower",
			"cursor_grid_row": 4,
			"cursor_grid_column": 8,
			"case": "lower",
			"finished": false,
		}
	)
	_assert_boot_action_payload_round_trip(
		"oak intro payload restore",
		oak_intro_script,
		{
			"screen": "oak_intro",
			"mode": "intro",
			"scene_index": 2,
			"scene_state": "oak_intro_2",
			"scene_phase": "select",
			"text_checkpoint": "oak_intro_2",
			"text_page_index": 2,
			"text_page_count": 3,
			"text_waiting_for_input": true,
			"gender": "female",
			"frame_counter": 17,
			"confirmed": true,
			"pending_action": "name_entry",
			"pending_action_payload": {
				"action_id": "oak_intro_confirm",
				"route": "name_entry",
				"source_screen": "oak_intro",
				"selected_option": "female",
				"mode": "intro",
				"scene_index": 2,
				"scene_state": "oak_intro_2",
				"scene_phase": "select",
				"text_checkpoint": "oak_intro_2",
				"text_page_index": 2,
				"text_page_count": 3,
				"text_waiting_for_input": true,
				"gender": "female",
				"frame_counter": 17,
				"confirmed": true,
			},
		},
		"name_entry",
		{
			"action_id": "oak_intro_confirm",
			"route": "name_entry",
			"selected_option": "female",
			"mode": "intro",
			"scene_index": 2,
			"scene_state": "oak_intro_2",
			"scene_phase": "select",
			"text_checkpoint": "oak_intro_2",
			"text_page_index": 2,
			"text_page_count": 3,
			"text_waiting_for_input": true,
			"gender": "female",
			"frame_counter": 17,
			"confirmed": true,
		}
	)
	_assert_boot_state_round_trip(
		"oak intro text checkpoint",
		oak_intro_script,
		{
			"screen": "oak_intro",
			"mode": "intro",
			"scene_index": 1,
			"scene_state": "wooper_showcase",
			"scene_phase": "text",
			"text_checkpoint": "wooper_showcase",
			"text_page_index": 1,
			"text_page_count": 3,
			"text_waiting_for_input": false,
			"gender": "male",
			"frame_counter": 8,
			"confirmed": false,
		},
		{
			"mode": "intro",
			"scene_index": 1,
			"scene_state": "wooper_showcase",
			"scene_phase": "text",
			"text_checkpoint": "wooper_showcase",
			"text_page_index": 1,
			"text_page_count": 3,
			"text_waiting_for_input": false,
			"gender": "male",
			"frame_counter": 8,
			"confirmed": false,
		}
	)
	var oak_gate_model: Variant = oak_intro_script.new()
	if not bool(oak_gate_model.call("from_dictionary", {
		"screen": "oak_intro",
		"mode": "intro",
		"scene_index": 2,
		"scene_state": "oak_intro_2",
		"scene_phase": "text",
		"text_checkpoint": "oak_intro_2",
		"text_page_index": 2,
		"text_page_count": 3,
		"text_waiting_for_input": false,
		"gender": "male",
		"frame_counter": 89,
		"confirmed": false,
		"pending_action": "",
		"pending_action_payload": {},
	})):
		push_error("smoke_test: oak intro gated text setup failed")
		quit(1)
		return
	oak_gate_model.call("_tick", 0.016)
	var oak_gate_state: Dictionary = Dictionary(oak_gate_model.call("get_state"))
	if str(oak_gate_state.get("scene_phase", "")) != "select" or not bool(oak_gate_state.get("text_waiting_for_input", false)):
		push_error("smoke_test: oak intro did not transition to waiting input state")
		quit(1)
		return
	var oak_gate_event := InputEventAction.new()
	oak_gate_event.action = "game_a"
	oak_gate_event.pressed = true
	oak_gate_model.call("_handle_boot_input", oak_gate_event)
	var oak_gate_state_with_action: Dictionary = Dictionary(oak_gate_model.call("get_state"))
	var oak_gate_payload: Dictionary = Dictionary(oak_gate_state_with_action.get("pending_action_payload", {}))
	if str(oak_gate_state_with_action.get("pending_action", "")) != "name_entry" or str(oak_gate_payload.get("action_id", "")) != "oak_intro_confirm" or str(oak_gate_payload.get("route", "")) != "name_entry" or not bool(oak_gate_payload.get("text_gate_open", false)) or not bool(oak_gate_payload.get("text_waiting_for_input", false)):
		push_error("smoke_test: oak intro gated confirm payload was malformed")
		quit(1)
		return
	_assert_boot_action_payload_round_trip(
		"intro sequence payload restore",
		intro_sequence_script,
		{
			"screen": "intro_sequence",
			"phase": "running",
			"phase_frame": 12,
			"scene_checkpoint": 3,
			"scene_index": 3,
			"frame_counter": 12,
			"finished": true,
			"skip_requested": true,
			"pending_action": "oak_intro",
			"pending_action_payload": {
				"action_id": "intro_skip",
				"route": "oak_intro",
				"source_screen": "intro_sequence",
				"selected_option": "advance",
				"phase": "running",
				"phase_frame": 12,
				"scene_checkpoint": 3,
				"scene_index": 3,
				"frame_counter": 12,
				"input_gate_frames": 8,
				"finished": true,
				"skip_requested": true,
			},
		},
		"oak_intro",
		{
			"action_id": "intro_skip",
			"route": "oak_intro",
			"selected_option": "advance",
			"phase": "running",
			"phase_frame": 12,
			"scene_checkpoint": 3,
			"scene_index": 3,
			"frame_counter": 12,
			"input_gate_frames": 8,
			"finished": true,
			"skip_requested": true,
		}
	)
	_assert_boot_state_round_trip(
		"intro sequence checkpoint",
		intro_sequence_script,
		{
			"screen": "intro_sequence",
			"phase": "running",
			"phase_frame": 7,
			"scene_checkpoint": 1,
			"scene_index": 1,
			"frame_counter": 7,
			"input_gate_frames": 8,
			"finished": false,
			"skip_requested": false,
		},
		{
			"phase": "running",
			"phase_frame": 7,
			"scene_checkpoint": 1,
			"scene_index": 1,
			"frame_counter": 7,
			"input_gate_frames": 8,
			"finished": false,
			"skip_requested": false,
		}
	)
	_assert_boot_state_round_trip(
		"intro sequence gate restore",
		intro_sequence_script,
		{
			"screen": "intro_sequence",
			"phase": "running",
			"phase_frame": 7,
			"scene_index": 1,
			"frame_counter": 7,
			"input_gate_frames": 8,
			"finished": false,
			"skip_requested": false,
		},
		{
			"phase": "running",
			"phase_frame": 7,
			"scene_index": 1,
			"frame_counter": 7,
			"input_gate_frames": 8,
			"finished": false,
			"skip_requested": false,
		}
	)
	var intro_gate_model: Variant = intro_sequence_script.new()
	if not bool(intro_gate_model.call("from_dictionary", {
		"screen": "intro_sequence",
		"phase": "running",
		"phase_frame": 7,
		"scene_index": 1,
		"frame_counter": 7,
		"input_gate_frames": 8,
		"finished": false,
		"skip_requested": false,
		"pending_action": "",
		"pending_action_payload": {},
	})):
		push_error("smoke_test: intro sequence input gate setup failed")
		quit(1)
		return
	if bool(intro_gate_model.call("can_accept_input")):
		push_error("smoke_test: intro sequence accepted input before gate opened")
		quit(1)
		return
	intro_gate_model.call("_tick", 0.016)
	if not bool(intro_gate_model.call("can_accept_input")):
		push_error("smoke_test: intro sequence gate did not open after timing advance")
		quit(1)
		return
	var intro_gate_event := InputEventAction.new()
	intro_gate_event.action = "game_a"
	intro_gate_event.pressed = true
	intro_gate_model.call("_handle_boot_input", intro_gate_event)
	var intro_gate_state_with_action: Dictionary = Dictionary(intro_gate_model.call("get_state"))
	var intro_gate_payload: Dictionary = Dictionary(intro_gate_state_with_action.get("pending_action_payload", {}))
	if str(intro_gate_state_with_action.get("pending_action", "")) != "oak_intro" or str(intro_gate_payload.get("action_id", "")) != "intro_skip" or str(intro_gate_payload.get("route", "")) != "oak_intro" or not bool(intro_gate_payload.get("input_gate_open", false)):
		push_error("smoke_test: intro sequence gated skip payload was malformed")
		quit(1)
		return
	_assert_boot_state_round_trip(
		"continue screen timing restore",
		continue_screen_script,
		{
			"screen": "continue_screen",
			"phase": "opening",
			"phase_frame": 7,
			"prompt_phase": "opening",
			"prompt_phase_frame": 7,
			"selection": 1,
			"confirmed": false,
		},
		{
			"phase": "opening",
			"phase_frame": 7,
			"prompt_phase": "opening",
			"prompt_phase_frame": 7,
			"selection": 1,
			"confirmed": false,
		}
	)
	_assert_boot_state_round_trip(
		"delete save timing restore",
		delete_save_screen_script,
		{
			"screen": "delete_save_screen",
			"phase": "opening",
			"phase_frame": 4,
			"prompt_phase": "opening",
			"prompt_phase_frame": 4,
			"selection": 1,
			"confirmed": false,
		},
		{
			"phase": "opening",
			"phase_frame": 4,
			"prompt_phase": "opening",
			"prompt_phase_frame": 4,
			"selection": 1,
			"confirmed": false,
		}
	)
	_assert_boot_state_round_trip(
		"clock reset timing restore",
		clock_reset_screen_script,
		{
			"screen": "clock_reset_screen",
			"phase": "set_minute",
			"phase_frame": 9,
			"selection": 0,
			"day": 6,
			"hour": 23,
			"minute": 45,
			"confirmed": false,
		},
		{
			"phase": "set_minute",
			"phase_frame": 9,
			"selection": 0,
			"day": 6,
			"hour": 23,
			"minute": 45,
			"confirmed": false,
		}
	)
	_assert_boot_state_round_trip(
		"day of week timing restore",
		day_of_week_screen_script,
		{
			"screen": "day_of_week_screen",
			"phase": "select_day",
			"phase_frame": 5,
			"selected_day": 4,
			"confirmed": false,
			"ignore_confirm_until_release": true,
		},
		{
			"phase": "select_day",
			"phase_frame": 5,
			"selected_day": 4,
			"confirmed": false,
			"ignore_confirm_until_release": true,
		}
	)
	_assert_boot_state_round_trip(
		"name entry timing restore",
		name_entry_script,
		{
			"screen": "name_entry",
			"phase": "editing",
			"phase_frame": 8,
			"cursor_blink_frame": 13,
			"cursor_visible": true,
			"keyboard_page": "upper",
			"cursor_grid_row": 4,
			"cursor_grid_column": 8,
			"name": "CHRIS",
			"cursor_index": 5,
			"cursor_column": 8,
			"cursor_row": 4,
			"case": "upper",
			"finished": false,
		},
		{
			"phase": "editing",
			"phase_frame": 8,
			"cursor_blink_frame": 13,
			"cursor_visible": true,
			"keyboard_page": "upper",
			"cursor_grid_row": 4,
			"cursor_grid_column": 8,
			"finished": false,
		}
	)
	var continue_opening_model: Variant = continue_screen_script.new()
	if not bool(continue_opening_model.call("from_dictionary", {
		"screen": "continue_screen",
		"phase": "opening",
		"phase_frame": 11,
		"selection": 0,
		"confirmed": false,
	})):
		push_error("smoke_test: continue screen opening-phase setup failed")
		quit(1)
		return
	continue_opening_model.call("_tick", 0.016)
	var continue_opening_state: Dictionary = Dictionary(continue_opening_model.call("get_state"))
	if str(continue_opening_state.get("phase", "")) != "main" or int(continue_opening_state.get("phase_frame", -1)) != 0:
		push_error("smoke_test: continue screen opening phase did not advance to main")
		quit(1)
		return
	runtime.call("route_to_continue_screen", "smoke_test")
	await process_frame
	if str(runtime.get("current_scene_route")) != "continue_screen":
		push_error("smoke_test: failed to route to continue screen")
		quit(1)
		return
	var continue_screen_node: Node = root.get_node_or_null("ContinueScreen")
	if continue_screen_node == null or not continue_screen_node.has_method("from_dictionary") or not continue_screen_node.has_method("get_state"):
		push_error("smoke_test: continue screen node is missing action methods")
		quit(1)
		return
	if not bool(continue_screen_node.call("from_dictionary", {
		"screen": "continue_screen",
		"selection": 0,
		"confirmed": false,
		"pending_action": "",
		"pending_action_payload": {},
	})):
		push_error("smoke_test: continue screen setup failed")
		quit(1)
		return
	continue_screen_node.call("_confirm")
	var queued_continue_state: Dictionary = Dictionary(continue_screen_node.call("get_state"))
	var queued_continue_payload: Dictionary = Dictionary(queued_continue_state.get("pending_action_payload", {}))
	if str(queued_continue_state.get("pending_action", "")) != "overworld" or str(queued_continue_payload.get("action_id", "")) != "continue_confirm" or str(queued_continue_payload.get("route", "")) != "overworld" or str(queued_continue_payload.get("selected_option", "")) != "continue":
		push_error("smoke_test: continue screen queued payload was malformed")
		quit(1)
		return
	await process_frame
	if str(runtime.get("current_scene_route")) != "overworld":
		push_error("smoke_test: continue screen handoff was not consumed by runtime")
		quit(1)
		return
	var consumed_continue_state: Dictionary = Dictionary(continue_screen_node.call("get_state"))
	var consumed_continue_payload: Dictionary = Dictionary(consumed_continue_state.get("last_action_payload", {}))
	if str(consumed_continue_state.get("pending_action", "")) != "" or str(consumed_continue_payload.get("action_id", "")) != "continue_confirm" or str(consumed_continue_payload.get("route", "")) != "overworld" or int(consumed_continue_payload.get("consumed_frame_counter", -1)) < 0:
		push_error("smoke_test: continue screen action was not consumed exactly once")
		quit(1)
		return
	var continue_cancel_model: Variant = continue_screen_script.new()
	if not bool(continue_cancel_model.call("from_dictionary", {
		"screen": "continue_screen",
		"phase": "main",
		"phase_frame": 2,
		"prompt_phase": "main",
		"prompt_phase_frame": 2,
		"selection": 1,
		"confirmed": false,
		"pending_action": "",
		"pending_action_payload": {},
	})):
		push_error("smoke_test: continue screen cancellation setup failed")
		quit(1)
		return
	continue_cancel_model.call("_confirm")
	var continue_cancel_state: Dictionary = Dictionary(continue_cancel_model.call("get_state"))
	var continue_cancel_payload: Dictionary = Dictionary(continue_cancel_state.get("pending_action_payload", {}))
	if str(continue_cancel_state.get("pending_action", "")) != "title" or str(continue_cancel_payload.get("action_id", "")) != "continue_cancel" or str(continue_cancel_payload.get("route", "")) != "title" or str(continue_cancel_payload.get("selected_option", "")) != "back" or not bool(continue_cancel_payload.get("cancelled", false)):
		push_error("smoke_test: continue screen cancellation payload was malformed")
		quit(1)
		return
	await process_frame
	if str(runtime.get("current_scene_route")) != "overworld":
		push_error("smoke_test: continue screen handoff repeated unexpectedly")
		quit(1)
		return
	var intro_sequence_model: Variant = intro_sequence_script.new()
	if not bool(intro_sequence_model.call("from_dictionary", {
		"screen": "intro_sequence",
		"scene_index": 99,
		"frame_counter": 12,
		"finished": true,
		"skip_requested": true,
		"last_input": {"pressed": {"a": true}, "released": {}, "down": {}},
	})):
		push_error("smoke_test: intro sequence snapshot restore failed")
		quit(1)
		return
	var restored_intro_sequence: Dictionary = Dictionary(intro_sequence_model.call("to_dictionary"))
	if int(restored_intro_sequence.get("scene_index", -1)) != 3 or int(restored_intro_sequence.get("frame_counter", -1)) != 12 or not bool(restored_intro_sequence.get("finished", false)):
		push_error("smoke_test: intro sequence restore did not clamp/preserve state")
		quit(1)
		return
	var clock_reset_model: Variant = clock_reset_screen_script.new()
	if not bool(clock_reset_model.call("from_dictionary", {
		"screen": "clock_reset_screen",
		"phase": "day_of_week",
		"selection": 99,
		"day": 9,
	})):
		push_error("smoke_test: clock reset snapshot restore failed")
		quit(1)
		return
	var restored_clock_reset: Dictionary = Dictionary(clock_reset_model.call("to_dictionary"))
	if int(restored_clock_reset.get("selection", -1)) != 1 or int(restored_clock_reset.get("day", -1)) != 6:
		push_error("smoke_test: clock reset restore did not clamp fields")
		quit(1)
		return
	var delete_save_model: Variant = delete_save_screen_script.new()
	if not bool(delete_save_model.call("from_dictionary", {
		"screen": "delete_save_screen",
		"selection": -3,
		"confirmed": true,
		"route_entry": true,
	})):
		push_error("smoke_test: delete save snapshot restore failed")
		quit(1)
		return
	var restored_delete_save: Dictionary = Dictionary(delete_save_model.call("to_dictionary"))
	if int(restored_delete_save.get("selection", -1)) != 1 or bool(restored_delete_save.get("confirmed", true)) or bool(restored_delete_save.get("input_locked", true)):
		push_error("smoke_test: delete save route-entry restore did not reset state")
		quit(1)
		return
	var delete_cancel_model: Variant = delete_save_screen_script.new()
	if not bool(delete_cancel_model.call("from_dictionary", {
		"screen": "delete_save_screen",
		"phase": "main",
		"phase_frame": 3,
		"prompt_phase": "main",
		"prompt_phase_frame": 3,
		"selection": 1,
		"confirmed": false,
		"pending_action": "",
		"pending_action_payload": {},
	})):
		push_error("smoke_test: delete save cancellation setup failed")
		quit(1)
		return
	delete_cancel_model.call("_confirm")
	var delete_cancel_state: Dictionary = Dictionary(delete_cancel_model.call("get_state"))
	var delete_cancel_payload: Dictionary = Dictionary(delete_cancel_state.get("pending_action_payload", {}))
	if str(delete_cancel_state.get("pending_action", "")) != "title" or str(delete_cancel_payload.get("action_id", "")) != "delete_save_cancel" or str(delete_cancel_payload.get("route", "")) != "title" or str(delete_cancel_payload.get("selected_option", "")) != "no" or not bool(delete_cancel_payload.get("cancelled", false)):
		push_error("smoke_test: delete save cancellation payload was malformed")
		quit(1)
		return
	var text_box_script: Script = load("res://scripts/text_box.gd")
	if text_box_script == null:
		push_error("smoke_test: failed to load text box script")
		quit(1)
		return
	var text_box_model: Variant = text_box_script.new()
	text_box_model.call("open_dialogue", [
		{"speaker": "Tester", "text": "Alpha"},
		{"speaker": "Tester", "text": "Beta"},
	])
	var text_box_snapshot: Dictionary = Dictionary(text_box_model.call("get_state"))
	if int(text_box_snapshot.get("page_count", 0)) != 2 or str(text_box_snapshot.get("current_text", "")) != "Alpha":
		push_error("smoke_test: text box state missing page data: %s" % JSON.stringify(text_box_snapshot))
		quit(1)
		return
	var text_box_round_trip := text_box_snapshot.duplicate(true)
	text_box_round_trip["page_index"] = 1
	text_box_round_trip["current_text"] = "Beta"
	text_box_round_trip["current_page_text"] = "Beta"
	text_box_round_trip["visible_text"] = "Beta"
	if not bool(text_box_model.call("from_dictionary", text_box_round_trip)):
		push_error("smoke_test: text box from_dictionary failed")
		quit(1)
		return
	var restored_text_box_state: Dictionary = Dictionary(text_box_model.call("get_state"))
	if int(restored_text_box_state.get("page_index", -1)) != 1 or str(restored_text_box_state.get("current_text", "")) != "Beta":
		push_error("smoke_test: text box state did not round-trip: %s" % JSON.stringify(restored_text_box_state))
		quit(1)
		return
	text_box_model.call("open_dialogue", [
		{"speaker": "Guide", "text": "Alpha <POKE> Beta", "wait_for_input": true, "input_delay_frames": 1},
		{"speaker": "Guide", "text": "Second page", "wait_for_input": true, "input_delay_frames": 0},
	])
	text_box_model.call("push_wait", 1)
	text_box_model.call("complete")
	var token_text_box_state: Dictionary = Dictionary(text_box_model.call("get_state"))
	if int(token_text_box_state.get("page_index", -1)) != 0 or int(token_text_box_state.get("pending_waits", 0)) != 1 or str(token_text_box_state.get("visible_text", "")) != "Alpha POKé Beta":
		push_error("smoke_test: text box token state did not expose visible text or waits: %s" % JSON.stringify(token_text_box_state))
		quit(1)
		return
	var token_text_box_round_trip := token_text_box_state.duplicate(true)
	if not bool(text_box_model.call("from_dictionary", token_text_box_round_trip)):
		push_error("smoke_test: text box token snapshot restore failed")
		quit(1)
		return
	var restored_token_text_box_state: Dictionary = Dictionary(text_box_model.call("get_state"))
	if int(restored_token_text_box_state.get("page_index", -1)) != 0 or int(restored_token_text_box_state.get("pending_waits", 0)) != 1 or str(restored_token_text_box_state.get("visible_text", "")) != "Alpha POKé Beta" or int(restored_token_text_box_state.get("token_cursor", -1)) != int(token_text_box_state.get("token_cursor", -1)):
		push_error("smoke_test: text box token snapshot restore did not preserve token/page state: %s" % JSON.stringify(restored_token_text_box_state))
		quit(1)
		return
	text_box_model.call("open_dialogue", [
		{"speaker": "Guide", "text": "A<WAIT>BC", "wait_for_input": true, "input_delay_frames": 1, "reveal_delay_frames": 2, "reveal_chars_per_tick": 1},
		{"speaker": "Guide", "text": "Second page", "wait_for_input": true, "input_delay_frames": 0},
	])
	text_box_model.call("tick")
	var token_tick_one_state: Dictionary = Dictionary(text_box_model.call("get_state"))
	if str(token_tick_one_state.get("visible_text", "")) != "" or int(token_tick_one_state.get("token_cursor", -1)) != 0 or int(token_tick_one_state.get("current_token_frame_timer", -1)) != 1 or int(token_tick_one_state.get("input_delay_frames", -1)) != 1:
		push_error("smoke_test: text box token timing did not advance deterministically on the first tick: %s" % JSON.stringify(token_tick_one_state))
		quit(1)
		return
	text_box_model.call("tick")
	var token_tick_two_state: Dictionary = Dictionary(text_box_model.call("get_state"))
	if str(token_tick_two_state.get("visible_text", "")) != "A" or str(token_tick_two_state.get("current_token_kind", "")) != "wait" or not bool(token_tick_two_state.get("token_wait_pending", false)) or int(token_tick_two_state.get("token_cursor", -1)) != 1 or int(token_tick_two_state.get("current_token_frame_timer", -1)) != 0:
		push_error("smoke_test: text box token timing did not reach the wait token deterministically: %s" % JSON.stringify(token_tick_two_state))
		quit(1)
		return
	if not bool(text_box_model.call("from_dictionary", token_tick_one_state)):
		push_error("smoke_test: text box token tick-one restore failed")
		quit(1)
		return
	text_box_model.call("tick")
	var restored_token_tick_two_state: Dictionary = Dictionary(text_box_model.call("get_state"))
	if str(restored_token_tick_two_state.get("visible_text", "")) != str(token_tick_two_state.get("visible_text", "")) or int(restored_token_tick_two_state.get("token_cursor", -1)) != int(token_tick_two_state.get("token_cursor", -1)) or int(restored_token_tick_two_state.get("current_token_frame_timer", -1)) != int(token_tick_two_state.get("current_token_frame_timer", -1)) or bool(restored_token_tick_two_state.get("token_wait_pending", false)) != bool(token_tick_two_state.get("token_wait_pending", false)):
		push_error("smoke_test: text box token timing did not round-trip across restore: %s" % JSON.stringify(restored_token_tick_two_state))
		quit(1)
		return
	var token_wait_ack_result: Dictionary = Dictionary(text_box_model.call("consume_input", {"pressed": {"a": true}}))
	if not bool(token_wait_ack_result.get("consumed", false)) or bool(token_wait_ack_result.get("waiting_for_input", true)) or bool(Dictionary(token_wait_ack_result.get("page", {})).get("token_wait_pending", false)):
		push_error("smoke_test: text box wait token did not acknowledge cleanly: %s" % JSON.stringify(token_wait_ack_result))
		quit(1)
		return
	var text_box_snapshot_restore := {
		"active": true,
		"visible": true,
		"input_locked": true,
		"page_cursor": {"index": 1},
		"dialogue_pages": [
			{"speaker": "Guide", "text": "First page", "meta": {"tone": "neutral"}},
			{"speaker": "Guide", "text": "Restored page", "display_text": "Restored page"},
		],
	}
	if not bool(text_box_model.call("from_dictionary", text_box_snapshot_restore)):
		push_error("smoke_test: text box snapshot restore failed")
		quit(1)
		return
	var restored_text_box_snapshot: Dictionary = Dictionary(text_box_model.call("to_dictionary"))
	if int(restored_text_box_snapshot.get("page_index", -1)) != 1 or str(restored_text_box_snapshot.get("current_text", "")) != "Restored page" or not bool(restored_text_box_snapshot.get("input_locked", false)):
		push_error("smoke_test: text box snapshot restore did not preserve cursor/text/lock")
		quit(1)
		return
	if not bool(text_box_model.call("from_dictionary", {
		"active": true,
		"visible": true,
		"input_locked": false,
		"page_index": 0,
		"page_frame": 2,
		"dialogue_pages": [
			{"speaker": "Guide", "text": "Input page one", "input_delay_frames": 0},
			{"speaker": "Guide", "text": "Input page two", "input_delay_frames": 0},
		],
	})):
		push_error("smoke_test: text box input restore failed")
		quit(1)
		return
	var text_advance_result: Dictionary = Dictionary(text_box_model.call("consume_input", {"pressed": {"a": true}}))
	if not bool(text_advance_result.get("consumed", false)) or not bool(text_advance_result.get("advanced", false)) or int(text_advance_result.get("page_index", -1)) != 1:
		push_error("smoke_test: text box confirm input did not advance page")
		quit(1)
		return
	var text_close_result: Dictionary = Dictionary(text_box_model.call("consume_input", {"pressed": {"start": true}}))
	if not bool(text_close_result.get("consumed", false)) or not bool(text_close_result.get("closed", false)) or bool(text_box_model.call("is_active")):
		push_error("smoke_test: text box confirm input did not close final page")
		quit(1)
		return
	var menu_stack_script: Script = load("res://scripts/menu_stack.gd")
	if menu_stack_script == null:
		push_error("smoke_test: failed to load menu stack script")
		quit(1)
		return
	var menu_state_script: Script = load("res://scripts/menu_state.gd")
	if menu_state_script == null:
		push_error("smoke_test: failed to load menu state script")
		quit(1)
		return
	var menu_stack_model: Variant = menu_stack_script.new()
	menu_stack_model.call("push_panel", {
		"id": "alpha_menu",
		"title": "Alpha Menu",
		"entries": [{"id": "alpha", "label": "Alpha"}, {"id": "beta", "label": "Beta"}],
	})
	var menu_stack_snapshot: Dictionary = Dictionary(menu_stack_model.call("get_state"))
	if int(menu_stack_snapshot.get("depth", 0)) != 1 or not bool(menu_stack_snapshot.get("menu_open", false)):
		push_error("smoke_test: menu stack state missing depth")
		quit(1)
		return
	var menu_stack_round_trip := menu_stack_snapshot.duplicate(true)
	menu_stack_round_trip["input_locked"] = true
	if not bool(menu_stack_model.call("from_dictionary", menu_stack_round_trip)):
		push_error("smoke_test: menu stack from_dictionary failed")
		quit(1)
		return
	var restored_menu_stack_state: Dictionary = Dictionary(menu_stack_model.call("get_state"))
	if not bool(restored_menu_stack_state.get("input_locked", false)) or int(restored_menu_stack_state.get("depth", 0)) != 1:
		push_error("smoke_test: menu stack state did not round-trip")
		quit(1)
		return
	var menu_stack_snapshot_restore := {
		"input_locked": true,
		"cursor_memory": {"restore_menu": 1},
		"stack": [{
			"id": "restore_menu",
			"title": "Restore Menu",
			"kind": "menu",
			"entries": [{"id": "first", "label": "First"}, {"id": "second", "label": "Second"}],
		}],
	}
	if not bool(menu_stack_model.call("from_dictionary", menu_stack_snapshot_restore)):
		push_error("smoke_test: menu stack snapshot restore failed")
		quit(1)
		return
	var restored_menu_stack_snapshot: Dictionary = Dictionary(menu_stack_model.call("to_dictionary"))
	var restored_top_panel: Dictionary = Dictionary(restored_menu_stack_snapshot.get("top_panel", {}))
	if str(restored_top_panel.get("id", "")) != "restore_menu" or int(restored_top_panel.get("cursor", -1)) != 1 or str(Dictionary(restored_top_panel.get("selection", {})).get("id", "")) != "second":
		push_error("smoke_test: menu stack snapshot restore did not preserve panel cursor")
		quit(1)
		return
	if not bool(menu_stack_model.call("from_dictionary", {
		"input_locked": false,
		"stack": [{
			"id": "input_menu",
			"title": "Input Menu",
			"kind": "menu",
			"cursor": 0,
			"entries": [{"id": "first", "label": "First"}, {"id": "second", "label": "Second"}],
		}],
	})):
		push_error("smoke_test: menu stack input restore failed")
		quit(1)
		return
	var menu_move_result: Dictionary = Dictionary(menu_stack_model.call("consume_input", {"pressed": {"down": true}}))
	if str(menu_move_result.get("action", "")) != "move_down" or int(Dictionary(menu_move_result.get("top_panel", {})).get("cursor", -1)) != 1:
		push_error("smoke_test: menu stack down input did not move cursor")
		quit(1)
		return
	var menu_confirm_result: Dictionary = Dictionary(menu_stack_model.call("consume_input", {"pressed": {"a": true}}))
	if str(menu_confirm_result.get("action", "")) != "confirm" or str(Dictionary(menu_confirm_result.get("selection", {})).get("id", "")) != "second":
		push_error("smoke_test: menu stack confirm input did not expose selection")
		quit(1)
		return
	var menu_cancel_result: Dictionary = Dictionary(menu_stack_model.call("consume_input", {"pressed": {"b": true}}))
	if str(menu_cancel_result.get("action", "")) != "cancel" or int(menu_stack_model.call("get_depth")) != 0:
		push_error("smoke_test: menu stack cancel input did not close menu")
		quit(1)
		return
	var menu_state_model: Variant = menu_state_script.new()
	menu_state_model.call("sync_runtime_state", {
		"sram": {
			"phone_numbers": ["MOM", "PROF_OAK", "BILL"],
			"options": {
				"text_speed": "fast",
				"battle_scene": true,
				"battle_style": "shift",
				"sound": "stereo",
				"menu_account": true,
				"frame": 1,
			},
		},
		"wram": {
			"engine_flags": {
				"ENGINE_POKEGEAR": true,
				"ENGINE_MAP_CARD": true,
				"ENGINE_PHONE_CARD": true,
				"ENGINE_RADIO_CARD": true,
			},
			"wMapGroup": 5,
			"wMapNumber": 6,
			"pokegear_card": 1,
			"pokegear_map_player_landmark": 24,
			"pokegear_map_cursor_landmark": 42,
			"pokegear_phone_cursor_position": 1,
			"pokegear_phone_scroll_position": 0,
			"pokegear_radio_frequency_raw": 32,
		},
		"player_name": "Chris",
		"player_gender": "male",
		"ui_page": "pokegear",
		"map_summary": {
			"map_name": "Azalea Town",
			"map_constant": "AZALEA_TOWN",
			"group_id": 5,
			"map_id": 6,
			"group_name": "JOHTO",
			"phone_service": 0,
		},
		"save_metadata": {
			"slot": "menu-state.sav",
			"kind": "custom",
			"saved_at": "2026-03-30T12:00:00.000Z",
			"frame_counter": 8,
		},
		"loaded_asset_summary": {"content_pack_count": 4},
	})
	var menu_state_top: Dictionary = Dictionary(menu_state_model.call("activate_menu", "pokegear"))
	if str(menu_state_top.get("id", "")) != "pokegear" or int(menu_state_top.get("cursor", -1)) != 1:
		push_error("smoke_test: menu state failed to activate pokegear: %s" % JSON.stringify(menu_state_top))
		quit(1)
		return
	var menu_state_snapshot: Dictionary = Dictionary(menu_state_model.call("get_state"))
	var menu_state_runtime: Dictionary = Dictionary(menu_state_snapshot.get("runtime_context", {}))
	var menu_state_top_panel: Dictionary = Dictionary(menu_state_snapshot.get("top_panel", {}))
	var menu_state_selection: Dictionary = Dictionary(menu_state_top_panel.get("selection", {}))
	var menu_state_selection_payload: Dictionary = Dictionary(menu_state_selection.get("payload", {}))
	if str(menu_state_runtime.get("player_name", "")) != "Chris" or str(menu_state_runtime.get("ui_page", "")) != "pokegear":
		push_error("smoke_test: menu state runtime context did not round-trip")
		quit(1)
		return
	var menu_state_menus: Dictionary = Dictionary(menu_state_snapshot.get("menus", {}))
	var menu_state_pokegear_detail: Dictionary = Dictionary(menu_state_menus.get("pokegear", {}))
	var menu_state_pokegear_state: Dictionary = Dictionary(menu_state_pokegear_detail.get("state", {}))
	if str(menu_state_pokegear_state.get("card", "")) != "MAP" or int(menu_state_pokegear_state.get("card_index", -1)) != 1 or str(menu_state_selection_payload.get("card", "")) != "MAP":
		push_error("smoke_test: pokegear did not reflect the selected map card")
		quit(1)
		return
	var pokegear_up_result: Dictionary = Dictionary(menu_state_model.call("consume_input", {"pressed": {"up": true}}))
	var pokegear_after_up: Dictionary = Dictionary(menu_state_model.call("get_state"))
	var pokegear_after_up_detail: Dictionary = Dictionary(Dictionary(Dictionary(pokegear_after_up.get("menus", {})).get("pokegear", {})).get("state", {}))
	if str(pokegear_up_result.get("action", "")) != "move_up" or int(pokegear_after_up_detail.get("map_cursor_landmark", -1)) != 41:
		push_error("smoke_test: pokegear map cursor did not move")
		quit(1)
		return
	var pokegear_right_result: Dictionary = Dictionary(menu_state_model.call("consume_input", {"pressed": {"right": true}}))
	var pokegear_phone_snapshot: Dictionary = Dictionary(menu_state_model.call("get_state"))
	var pokegear_phone_detail: Dictionary = Dictionary(Dictionary(Dictionary(pokegear_phone_snapshot.get("menus", {})).get("pokegear", {})).get("state", {}))
	var pokegear_phone_selection: Dictionary = Dictionary(Dictionary(pokegear_phone_snapshot.get("top_panel", {})).get("selection", {}))
	var pokegear_phone_payload: Dictionary = Dictionary(pokegear_phone_selection.get("payload", {}))
	if str(pokegear_right_result.get("action", "")) != "switch_card" or str(pokegear_phone_detail.get("card", "")) != "PHONE" or int(pokegear_phone_detail.get("phone_cursor", -1)) != 1 or str(pokegear_phone_payload.get("card", "")) != "PHONE":
		push_error("smoke_test: pokegear phone card switch did not update state")
		quit(1)
		return
	var pokegear_phone_move_result: Dictionary = Dictionary(menu_state_model.call("consume_input", {"pressed": {"down": true}}))
	var pokegear_phone_move_snapshot: Dictionary = Dictionary(menu_state_model.call("get_state"))
	var pokegear_phone_move_detail: Dictionary = Dictionary(Dictionary(Dictionary(pokegear_phone_move_snapshot.get("menus", {})).get("pokegear", {})).get("state", {}))
	if str(pokegear_phone_move_result.get("action", "")) != "move_down" or int(pokegear_phone_move_detail.get("phone_cursor", -1)) != 2:
		push_error("smoke_test: pokegear phone cursor did not move")
		quit(1)
		return
	var pokegear_right_again_result: Dictionary = Dictionary(menu_state_model.call("consume_input", {"pressed": {"right": true}}))
	var pokegear_radio_snapshot: Dictionary = Dictionary(menu_state_model.call("get_state"))
	var pokegear_radio_detail: Dictionary = Dictionary(Dictionary(Dictionary(pokegear_radio_snapshot.get("menus", {})).get("pokegear", {})).get("state", {}))
	var pokegear_radio_selection: Dictionary = Dictionary(Dictionary(pokegear_radio_snapshot.get("top_panel", {})).get("selection", {}))
	var pokegear_radio_payload: Dictionary = Dictionary(pokegear_radio_selection.get("payload", {}))
	if str(pokegear_right_again_result.get("action", "")) != "switch_card" or str(pokegear_radio_detail.get("card", "")) != "RADIO" or int(pokegear_radio_detail.get("radio_frequency_raw", -1)) != 32 or str(pokegear_radio_payload.get("card", "")) != "RADIO":
		push_error("smoke_test: pokegear radio card switch did not update state")
		quit(1)
		return
	var pokegear_radio_move_result: Dictionary = Dictionary(menu_state_model.call("consume_input", {"pressed": {"down": true}}))
	var pokegear_radio_move_snapshot: Dictionary = Dictionary(menu_state_model.call("get_state"))
	var pokegear_radio_move_detail: Dictionary = Dictionary(Dictionary(Dictionary(pokegear_radio_move_snapshot.get("menus", {})).get("pokegear", {})).get("state", {}))
	if str(pokegear_radio_move_result.get("action", "")) != "move_down" or int(pokegear_radio_move_detail.get("radio_frequency_raw", -1)) != 28 or absf(float(pokegear_radio_move_detail.get("radio_frequency", 0.0)) - 7.5) > 0.01:
		push_error("smoke_test: pokegear radio tuning did not update state")
		quit(1)
		return
	if not bool(menu_state_model.call("from_dictionary", pokegear_radio_move_snapshot)):
		push_error("smoke_test: menu state pokegear snapshot restore failed")
		quit(1)
		return
	var restored_menu_state: Dictionary = Dictionary(menu_state_model.call("get_state"))
	var restored_menu_state_runtime: Dictionary = Dictionary(restored_menu_state.get("runtime_context", {}))
	var restored_menu_state_selection: Dictionary = Dictionary(Dictionary(restored_menu_state.get("top_panel", {})).get("selection", {}))
	var restored_menu_state_metadata: Dictionary = Dictionary(restored_menu_state_runtime.get("save_metadata", {}))
	var restored_menus: Dictionary = Dictionary(restored_menu_state.get("menus", {}))
	var restored_pokegear_detail: Dictionary = Dictionary(Dictionary(restored_menus.get("pokegear", {})).get("state", {}))
	if str(restored_menu_state_metadata.get("slot", "")) != "menu-state.sav" or str(restored_menu_state_metadata.get("kind", "")) != "custom":
		push_error("smoke_test: menu state runtime metadata did not survive round-trip")
		quit(1)
		return
	if str(restored_pokegear_detail.get("card", "")) != "RADIO" or int(restored_pokegear_detail.get("phone_cursor", -1)) != 2 or int(restored_pokegear_detail.get("radio_frequency_raw", -1)) != 28 or int(restored_pokegear_detail.get("map_cursor_landmark", -1)) != 41:
		push_error("smoke_test: pokegear state did not survive round-trip")
		quit(1)
		return
	if str(restored_menu_state_selection.get("payload", {}).get("card", "")) != "RADIO":
		push_error("smoke_test: pokegear selection did not survive round-trip")
		quit(1)
		return
	if is_instance_valid(text_box_model):
		text_box_model.free()
	if is_instance_valid(menu_stack_model):
		menu_stack_model.free()
	var asset_index_script: Script = load("res://scripts/asset_index.gd")
	if asset_index_script == null:
		push_error("smoke_test: failed to load asset index script")
		quit(1)
		return
	var asset_index_model: Variant = asset_index_script.new()
	asset_index_model.call("initialize")
	var asset_summary: Dictionary = Dictionary(asset_index_model.call("load_summary"))
	if asset_summary.is_empty() or not asset_summary.has("content_pack_version") or not asset_summary.has("content_pack_count"):
		push_error("smoke_test: asset summary missing content pack fields")
		quit(1)
		return
	if int(asset_summary.get("content_pack_version", -1)) < 0 or int(asset_summary.get("content_pack_count", -1)) < 0:
		push_error("smoke_test: asset summary content pack fields invalid")
		quit(1)
		return
	if not bool(asset_index_model.call("has_data", "pokemon_data.json")) or not bool(asset_index_model.call("has_data", "runtime_map_metadata.json")) or not bool(asset_index_model.call("has_gfx", "battle/expbar.1bpp")):
		push_error("smoke_test: asset index existence checks failed")
		quit(1)
		return
	if str(asset_index_model.call("data_path", "pokemon_data.json")).is_empty() or str(asset_index_model.call("gfx_path", "battle/expbar.1bpp")).is_empty():
		push_error("smoke_test: asset index path helpers returned empty paths")
		quit(1)
		return
	if Array(asset_index_model.call("load_array", "pokemon_data.json")).is_empty() or Dictionary(asset_index_model.call("load_dictionary", "moves_data.json")).is_empty():
		push_error("smoke_test: asset index typed JSON loaders returned empty core data")
		quit(1)
		return
	if str(asset_index_model.call("load_text", "content-packs/index.json")).is_empty():
		push_error("smoke_test: asset index text loader returned empty content")
		quit(1)
		return
	var loaded_asset_bytes: PackedByteArray = PackedByteArray(asset_index_model.call("load_raw_bytes", "gfx/battle/expbar.1bpp"))
	if loaded_asset_bytes.is_empty():
		push_error("smoke_test: asset index raw byte loader returned empty content")
		quit(1)
		return
	var loaded_asset_image: Image = asset_index_model.call("load_image", "gfx/battle/balls.png")
	if loaded_asset_image.get_width() <= 0 or loaded_asset_image.get_height() <= 0:
		push_error("smoke_test: asset index image loader returned empty image")
		quit(1)
		return
	if Dictionary(asset_index_model.call("load_runtime_map_metadata")).is_empty() or Dictionary(asset_index_model.call("load_runtime_spawn_points")).is_empty() or Dictionary(asset_index_model.call("load_map_blocks")).is_empty():
		push_error("smoke_test: asset index map manifest loaders returned empty data")
		quit(1)
		return
	if Array(asset_index_model.call("load_battle_animation_table")).is_empty() or Dictionary(asset_index_model.call("load_battle_anim_bundle")).is_empty():
		push_error("smoke_test: asset index battle manifest loaders returned empty data")
		quit(1)
		return
	if Dictionary(asset_index_model.call("load_menu_icons")).is_empty() or Dictionary(asset_index_model.call("load_sprite_anim_bundle")).is_empty() or Dictionary(asset_index_model.call("load_sprite_palette_defaults")).is_empty():
		push_error("smoke_test: asset index sprite/menu manifest loaders returned empty data")
		quit(1)
		return
	if Array(asset_index_model.call("load_palette_bank", "title/title.pal")).size() < 8:
		push_error("smoke_test: asset index palette bank loader returned too few palettes")
		quit(1)
		return
	var audio_script: Script = load("res://scripts/audio_assets.gd")
	if audio_script == null:
		push_error("smoke_test: failed to load audio assets script")
		quit(1)
		return
	var audio_model: Variant = audio_script.new()
	audio_model.call("initialize")
	var music_cue: Dictionary = Dictionary(audio_model.call("resolve_audio_cue", "music", "MUSIC_NEW_BARK_TOWN"))
	if str(music_cue.get("category", "")) != "music" or not bool(music_cue.get("loop", false)) or str(music_cue.get("relative_path", "")).get_extension() != "mp3":
		push_error("smoke_test: audio music cue metadata did not normalize")
		quit(1)
		return
	var priority_sfx_cue: Dictionary = Dictionary(audio_model.call("resolve_audio_cue", "sfx", "SFX_GET_BADGE"))
	if str(priority_sfx_cue.get("category", "")) != "sfx" or not bool(priority_sfx_cue.get("priority_sound", false)) or not bool(priority_sfx_cue.get("fade_music", false)):
		push_error("smoke_test: audio priority sfx metadata did not normalize")
		quit(1)
		return
	var cry_cue: Dictionary = Dictionary(audio_model.call("resolve_audio_cue", "cry", "CHIKORITA"))
	if str(cry_cue.get("category", "")) != "cry" or str(cry_cue.get("relative_path", "")).find("cries/") != 0 or str(cry_cue.get("priority_class", "")) != "cry":
		push_error("smoke_test: audio cry cue metadata did not normalize")
		quit(1)
		return
	var audio_state: Dictionary = Dictionary(audio_model.call("create_audio_playback_state"))
	var fake_music_plan: Dictionary = Dictionary(audio_model.call("build_audio_playback_plan_for_cue", music_cue))
	fake_music_plan["ok"] = true
	var music_schedule: Dictionary = Dictionary(audio_model.call("schedule_audio_playback_plan", audio_state, fake_music_plan))
	if not bool(music_schedule.get("allowed", false)) or str(audio_state.get("music_token", "")) != "MUSIC_NEW_BARK_TOWN":
		push_error("smoke_test: audio scheduler did not accept music plan")
		quit(1)
		return
	var fake_priority_plan: Dictionary = Dictionary(audio_model.call("build_audio_playback_plan_for_cue", priority_sfx_cue))
	fake_priority_plan["ok"] = true
	var priority_schedule: Dictionary = Dictionary(audio_model.call("schedule_audio_playback_plan", audio_state, fake_priority_plan))
	var audio_snapshot: Dictionary = Dictionary(audio_model.call("build_audio_playback_snapshot", audio_state))
	if not bool(priority_schedule.get("allowed", false)) or not bool(audio_snapshot.get("musicMutedByPriority", false)) or Array(audio_snapshot.get("activeChannels", [])).is_empty():
		push_error("smoke_test: audio scheduler did not apply priority sfx muting")
		quit(1)
		return
	var released_audio: Dictionary = Dictionary(audio_model.call("release_audio_playback_plan", audio_state, fake_priority_plan))
	if bool(released_audio.get("musicMutedByPriority", true)) or int(released_audio.get("priorityMuteCount", -1)) != 0:
		push_error("smoke_test: audio scheduler did not release priority sfx muting")
		quit(1)
		return
	var decoder_script: Script = load("res://scripts/gb_tile_decoder.gd")
	if decoder_script == null:
		push_error("smoke_test: failed to load tile decoder script")
		quit(1)
		return
	var decoder_model: Variant = decoder_script.new()
	var lz_bytes := PackedByteArray([0x60, 0xFF])
	var decompressed: PackedByteArray = PackedByteArray(decoder_model.call("decompress_lz", lz_bytes))
	if decompressed.size() != 1 or int(decompressed[0]) != 0:
		push_error("smoke_test: lz decoder did not expand the smoke payload")
		quit(1)
		return
	var tile_image: Image = decoder_model.call("decode_1bpp_tile", PackedByteArray([0x80, 0, 0, 0, 0, 0, 0, 0]))
	if tile_image.get_width() != 8 or tile_image.get_height() != 8:
		push_error("smoke_test: tile decoder returned wrong dimensions")
		quit(1)
		return
	var white_pixel: Color = tile_image.get_pixel(0, 0)
	var black_pixel: Color = tile_image.get_pixel(1, 0)
	if white_pixel.r >= black_pixel.r:
		push_error("smoke_test: tile decoder 1bpp bit ordering did not darken the set pixel")
		quit(1)
		return
	var smoke_palette: Array[Color] = [
		Color(0.0, 0.0, 0.0, 1.0),
		Color(1.0, 0.0, 0.0, 1.0),
		Color(0.0, 1.0, 0.0, 1.0),
		Color(0.0, 0.0, 1.0, 1.0),
	]
	var two_bpp_bytes := PackedByteArray([0x80, 0x40, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0])
	var two_bpp_tile: Image = decoder_model.call("decode_2bpp_tile", two_bpp_bytes, smoke_palette)
	if two_bpp_tile.get_width() != 8 or two_bpp_tile.get_height() != 8:
		push_error("smoke_test: tile decoder 2bpp returned wrong dimensions")
		quit(1)
		return
	var first_two_bpp_pixel: Color = two_bpp_tile.get_pixel(0, 0)
	var second_two_bpp_pixel: Color = two_bpp_tile.get_pixel(1, 0)
	if first_two_bpp_pixel.r < 0.9 or second_two_bpp_pixel.g < 0.9:
		push_error("smoke_test: tile decoder 2bpp bitplanes decoded unexpected colors")
		quit(1)
		return
	var decoded_1bpp_tiles: Array = Array(decoder_model.call("decode_1bpp_tiles", PackedByteArray([0x80, 0, 0, 0, 0, 0, 0, 0, 0x40, 0, 0, 0, 0, 0, 0, 0])))
	if decoded_1bpp_tiles.size() != 2:
		push_error("smoke_test: tile decoder 1bpp tile batch size mismatch")
		quit(1)
		return
	var padded_2bpp_tiles: Array = Array(decoder_model.call("decode_2bpp_tiles_padded", two_bpp_bytes, 2, smoke_palette))
	if padded_2bpp_tiles.size() != 2:
		push_error("smoke_test: tile decoder padded 2bpp tile batch size mismatch")
		quit(1)
		return
	var double_two_bpp_bytes := PackedByteArray([0x80, 0x40, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x80, 0x40, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0])
	var ranged_2bpp_tiles: Array = Array(decoder_model.call("decode_2bpp_tiles_range", double_two_bpp_bytes, 1, 1, smoke_palette))
	if ranged_2bpp_tiles.size() != 1:
		push_error("smoke_test: tile decoder ranged 2bpp tile batch size mismatch")
		quit(1)
		return
	var tile_atlas: Image = decoder_model.call("decode_2bpp_atlas", double_two_bpp_bytes, 2, smoke_palette)
	if tile_atlas.get_width() != 16 or tile_atlas.get_height() != 8:
		push_error("smoke_test: tile decoder atlas dimensions mismatch")
		quit(1)
		return
	var indexed_tilemap: Image = decoder_model.call("assemble_indexed_tilemap", padded_2bpp_tiles, PackedInt32Array([0, 1, 1, 0]), 2, 2)
	if indexed_tilemap.get_width() != 16 or indexed_tilemap.get_height() != 16:
		push_error("smoke_test: tile decoder indexed tilemap dimensions mismatch")
		quit(1)
		return

	runtime.call("request_scene_route", "battle", "smoke_test")
	await process_frame
	if str(runtime.get("current_scene_route")) != "battle":
		push_error("smoke_test: failed to route to battle")
		quit(1)
		return
	if not bool(battle.visible) or bool(ui_shell.visible) or bool(overworld.visible):
		push_error("smoke_test: battle visibility mismatch")
		quit(1)
		return
	var battle_runtime: Node = battle
	if not battle_runtime.has_method("begin_battle") or not battle_runtime.has_method("queue_command"):
		push_error("smoke_test: battle route is missing battle flow methods")
		quit(1)
		return
	battle_runtime.call("begin_battle", {
		"battle_id": "smoke-battle",
		"battle_kind": "wild",
		"battle_label": "Smoke Battle",
	})
	var battle_state_obj: Variant = battle_runtime.call("get", "battle_state")
	if battle_state_obj == null:
		push_error("smoke_test: battle state missing")
		quit(1)
		return
	if str(battle_state_obj.turn_phase) != "turn_prompt":
		push_error("smoke_test: battle did not enter prompt phase")
		quit(1)
		return
	if not bool(battle_state_obj.prompt_gate_active):
		push_error("smoke_test: battle prompt gate not active")
		quit(1)
		return
	if bool(battle_runtime.call("get_dialogue_wait_gate_active")) != bool(battle_state_obj.dialogue_wait_gate_active):
		push_error("smoke_test: battle dialogue gate getter did not match state")
		quit(1)
		return
	if bool(battle_runtime.call("get_waiting_for_input")) != bool(battle_state_obj.waiting_for_input):
		push_error("smoke_test: battle waiting getter did not match state")
		quit(1)
		return
	battle_runtime.call("set_fast_animation_request", true)
	if not bool(battle_runtime.call("get_fast_animation_request")):
		push_error("smoke_test: battle fast animation getter did not match state")
		quit(1)
		return
	battle_runtime.call("set_fast_text_request", true)
	if not bool(battle_runtime.call("get_fast_text_request")):
		push_error("smoke_test: battle fast text getter did not match state")
		quit(1)
		return
	battle_runtime.call("set_waiting_for_input", true)
	if not bool(battle_runtime.call("get_waiting_for_input")):
		push_error("smoke_test: battle waiting getter did not match state after set")
		quit(1)
		return
	battle_runtime.call("set_waiting_for_input", false)
	if battle_runtime.has_method("set_selected_battle_payloads"):
		battle_runtime.call("set_selected_battle_payloads",
			{"id": "PLAYER_MON", "name": "Player Mon", "hp": 20, "max_hp": 20},
			{"id": "ENEMY_MON", "name": "Enemy Mon", "hp": 15, "max_hp": 15}
		)
	battle_runtime.call("queue_command", {
		"kind": "attack",
		"label": "Tackle",
		"move": {"id": "TACKLE", "name": "Tackle", "power": 35},
	})
	if int(battle_state_obj.queued_commands.size()) < 1:
		push_error("smoke_test: battle command queue did not populate")
		quit(1)
		return
	battle_runtime.call("begin_resolution")
	if str(battle_state_obj.turn_phase) != "resolution":
		push_error("smoke_test: battle did not enter resolution")
		quit(1)
		return
	if battle_runtime.has_method("get_last_turn_resolution"):
		var active_turn_resolution: Dictionary = Dictionary(battle_runtime.call("get_last_turn_resolution"))
		if str(Dictionary(active_turn_resolution.get("command", {})).get("label", "")) != "Tackle" or not active_turn_resolution.has("valid"):
			push_error("smoke_test: battle integrated resolution did not expose command validation")
			quit(1)
			return
	var active_resolution_events: Array = Array(battle_runtime.call("get_resolution_events"))
	if active_resolution_events.size() < 3:
		push_error("smoke_test: battle integrated resolution did not queue scaffold events")
		quit(1)
		return
	battle_runtime.call("complete_turn", "smoke resolution")
	if str(battle_state_obj.turn_phase) != "turn_prompt":
		push_error("smoke_test: battle did not return to prompt phase")
		quit(1)
		return
	if battle_runtime.has_method("get_last_turn_resolution"):
		var completed_turn_resolution: Dictionary = Dictionary(battle_runtime.call("get_last_turn_resolution"))
		if str(Dictionary(completed_turn_resolution.get("move", {})).get("id", "")) != "TACKLE":
			push_error("smoke_test: battle integrated resolution did not preserve move payload")
			quit(1)
			return
	if battle_runtime.has_method("complete_battle") and battle_runtime.has_method("get_battle_result_state"):
		battle_runtime.call("complete_battle", "win", {"reason": "smoke", "winner": "player"})
		var completed_battle_result: Dictionary = Dictionary(battle_runtime.call("get_battle_result_state"))
		if str(completed_battle_result.get("result", "")) != "win" or str(completed_battle_result.get("reason", "")) != "smoke" or not bool(completed_battle_result.get("finished", false)):
			push_error("smoke_test: battle result state did not persist completion detail")
			quit(1)
			return
	if int(battle_state_obj.turn_number) < 1:
		push_error("smoke_test: battle turn did not advance")
		quit(1)
		return
	if int(battle_runtime.call("get_state_revision")) != int(battle_state_obj.state_revision):
		push_error("smoke_test: battle runtime revision getter did not match state")
		quit(1)
		return
	var battle_phase_history: Array = Array(battle_runtime.call("get_phase_history"))
	if battle_phase_history.is_empty() or str(battle_phase_history.back()) != str(battle_state_obj.turn_phase):
		push_error("smoke_test: battle phase history getter did not match state")
		quit(1)
		return
	if Array(battle_runtime.call("get_log_lines")).is_empty():
		push_error("smoke_test: battle log getter did not expose state")
		quit(1)
		return
	if not bool(battle_runtime.call("has_resolution_events")):
		push_error("smoke_test: battle resolution events did not queue")
		quit(1)
		return
	var battle_revision_before_resolution := int(battle_state_obj.state_revision)
	var queued_resolution_events: Array = Array(battle_runtime.call("get_resolution_events"))
	if queued_resolution_events.is_empty():
		push_error("smoke_test: battle resolution event did not queue")
		quit(1)
		return
	battle_runtime.call("queue_resolution_event", {"type": "manual_smoke_event", "detail": {"source": "smoke"}})
	battle_runtime.call("record_resolution", {"kind": "status", "label": "Smoke Log"}, "manual smoke resolution")
	var enriched_resolution_events: Array = Array(battle_runtime.call("get_resolution_events"))
	if enriched_resolution_events.size() < queued_resolution_events.size() + 2:
		push_error("smoke_test: battle public event queue did not append manual events")
		quit(1)
		return
	var previous_event_sequence := -1
	var saw_manual_event := false
	var saw_recorded_resolution := false
	for event in enriched_resolution_events:
		var event_dict: Dictionary = Dictionary(event)
		var event_sequence := int(event_dict.get("sequence", 0))
		if event_sequence <= previous_event_sequence:
			push_error("smoke_test: battle resolution event sequence was not monotonic")
			quit(1)
			return
		previous_event_sequence = event_sequence
		if str(event_dict.get("type", "")) == "manual_smoke_event":
			saw_manual_event = true
		if str(event_dict.get("type", "")) == "turn_resolution_recorded" and str(Dictionary(event_dict.get("command", {})).get("label", "")) == "Smoke Log":
			saw_recorded_resolution = true
	if not saw_manual_event or not saw_recorded_resolution:
		push_error("smoke_test: battle resolution events missing manual public event details")
		quit(1)
		return
	var drained_resolution_events: Array = Array(battle_runtime.call("drain_resolution_events"))
	if drained_resolution_events.size() != enriched_resolution_events.size() or str(Dictionary(drained_resolution_events.back()).get("type", "")) != str(Dictionary(enriched_resolution_events.back()).get("type", "")):
		push_error("smoke_test: battle resolution drain returned no events")
		quit(1)
		return
	if int(Array(battle_state_obj.call("consume_resolution_events")).size()) != 0:
		push_error("smoke_test: battle resolution events did not fully drain")
		quit(1)
		return
	if int(battle_state_obj.state_revision) <= battle_revision_before_resolution:
		push_error("smoke_test: battle state revision did not advance")
		quit(1)
		return
	var original_battle_state: Dictionary = Dictionary(battle_state_obj.call("to_dictionary"))
	var battle_state_snapshot := original_battle_state.duplicate(true)
	battle_state_snapshot["turn_number"] = 321
	battle_state_snapshot["turn_phase"] = "resolution"
	battle_state_snapshot["prompt_gate_active"] = true
	battle_state_snapshot["prompt_gate_reason"] = "smoke"
	battle_state_snapshot["prompt_kind"] = "turn_command"
	battle_state_snapshot["prompt_message"] = "awaiting player command"
	battle_state_snapshot["prompt_locked"] = true
	battle_state_snapshot["battle_id"] = "battle-001"
	battle_state_snapshot["battle_kind"] = "trainer"
	battle_state_snapshot["battle_label"] = "Smoke Label"
	battle_state_snapshot["active_side"] = "player"
	battle_state_snapshot["pending_command"] = {"kind": "attack", "label": "Tackle"}
	battle_state_snapshot["queued_commands"] = [{"kind": "attack", "label": "Tackle"}]
	battle_state_snapshot["last_resolved_command"] = {"kind": "attack", "label": "Tackle"}
	battle_state_snapshot["last_turn_resolution"] = {
		"valid": true,
		"reason": "ok",
		"command": {"kind": "attack", "label": "Tackle"},
		"actor": {"id": "PLAYER_MON", "name": "Player Mon"},
		"target": {"id": "ENEMY_MON", "name": "Enemy Mon"},
		"move": {"id": "TACKLE", "name": "Tackle"},
	}
	battle_state_snapshot["resolution_events"] = [
		{"type": "turn_resolution_complete", "sequence": 3, "turn_number": 321, "command": {"kind": "attack", "label": "Tackle"}, "summary": "smoke"},
	]
	battle_state_snapshot["resolution_event_sequence"] = 3
	battle_state_snapshot["battle_finished"] = true
	battle_state_snapshot["battle_result"] = "win"
	battle_state_snapshot["battle_result_state"] = {"finished": true, "result": "win", "turn_number": 321, "reason": "smoke", "winner": "player"}
	battle_state_snapshot["waiting_for_input"] = true
	battle_state_snapshot["manual_wait_override"] = true
	battle_state_snapshot["ui_phase"] = "DIALOGUE"
	battle_state_snapshot["dialogue_wait_gate_active"] = true
	battle_state_snapshot["fast_animation_request"] = true
	battle_state_snapshot["fast_text_request"] = true
	battle_state_snapshot["state_revision"] = 9
	battle_state_snapshot["battle_context"] = {"map": "route1", "trainer": "BUGSY"}
	battle_state_snapshot["asset_summary"] = {"pokemon_count": 2, "content_pack_count": 3}
	battle_state_snapshot["selected_player_payload"] = {"id": "PLAYER_MON", "name": "Player Mon"}
	battle_state_snapshot["selected_opponent_payload"] = {"id": "ENEMY_MON", "name": "Enemy Mon"}
	battle_state_snapshot["phase_history"] = ["setup", "intro", "turn_prompt", "resolution"]
	battle_state_snapshot["log_lines"] = ["battle shell ready", "smoke"]
	if not bool(battle_state_obj.call("from_dictionary", battle_state_snapshot)):
		push_error("smoke_test: battle state from_dictionary failed")
		quit(1)
		return
	var restored_battle_state: Dictionary = Dictionary(battle_state_obj.call("to_dictionary"))
	if int(restored_battle_state.get("turn_number", -1)) != 321:
		push_error("smoke_test: battle turn number did not round-trip")
		quit(1)
		return
	if str(restored_battle_state.get("battle_id", "")) != "battle-001" or str(restored_battle_state.get("battle_kind", "")) != "trainer":
		push_error("smoke_test: battle identity fields did not round-trip")
		quit(1)
		return
	if str(restored_battle_state.get("turn_phase", "")) != "resolution":
		push_error("smoke_test: battle turn phase did not round-trip")
		quit(1)
		return
	if not bool(restored_battle_state.get("prompt_gate_active", false)):
		push_error("smoke_test: battle prompt gate did not round-trip")
		quit(1)
		return
	if str(Dictionary(restored_battle_state.get("pending_command", {})).get("label", "")) != "Tackle":
		push_error("smoke_test: battle pending command did not round-trip")
		quit(1)
		return
	if str(Dictionary(restored_battle_state.get("last_resolved_command", {})).get("label", "")) != "Tackle":
		push_error("smoke_test: battle resolved command did not round-trip")
		quit(1)
		return
	if str(Dictionary(Dictionary(restored_battle_state.get("last_turn_resolution", {})).get("move", {})).get("id", "")) != "TACKLE":
		push_error("smoke_test: battle last turn resolution did not round-trip")
		quit(1)
		return
	var restored_resolution_events: Array = Array(restored_battle_state.get("resolution_events", []))
	if restored_resolution_events.is_empty() or str(Dictionary(restored_resolution_events[0]).get("type", "")) != "turn_resolution_complete":
		push_error("smoke_test: battle resolution events did not round-trip")
		quit(1)
		return
	if int(restored_battle_state.get("resolution_event_sequence", -1)) != 3:
		push_error("smoke_test: battle resolution event sequence did not round-trip")
		quit(1)
		return
	if not bool(restored_battle_state.get("battle_finished", false)) or str(restored_battle_state.get("battle_result", "")) != "win":
		push_error("smoke_test: battle completion state did not round-trip")
		quit(1)
		return
	if str(Dictionary(restored_battle_state.get("battle_result_state", {})).get("reason", "")) != "smoke" or str(Dictionary(restored_battle_state.get("battle_result_state", {})).get("winner", "")) != "player":
		push_error("smoke_test: battle result detail did not round-trip")
		quit(1)
		return
	if not bool(restored_battle_state.get("waiting_for_input", false)) or not bool(restored_battle_state.get("manual_wait_override", false)):
		push_error("smoke_test: battle wait flags did not round-trip")
		quit(1)
		return
	if str(restored_battle_state.get("ui_phase", "")) != "COMPLETE" or bool(restored_battle_state.get("dialogue_wait_gate_active", true)):
		push_error("smoke_test: battle ui phase flags did not round-trip")
		quit(1)
		return
	if not bool(restored_battle_state.get("fast_animation_request", false)) or not bool(restored_battle_state.get("fast_text_request", false)):
		push_error("smoke_test: battle fast request flags did not round-trip")
		quit(1)
		return
	if int(restored_battle_state.get("state_revision", -1)) != 9:
		push_error("smoke_test: battle state revision did not round-trip")
		quit(1)
		return
	if str(Dictionary(restored_battle_state.get("battle_context", {})).get("trainer", "")) != "BUGSY":
		push_error("smoke_test: battle context did not round-trip")
		quit(1)
		return
	if int(Dictionary(restored_battle_state.get("asset_summary", {})).get("content_pack_count", -1)) != 3:
		push_error("smoke_test: battle asset summary did not round-trip")
		quit(1)
		return
	if str(Dictionary(restored_battle_state.get("selected_player_payload", {})).get("id", "")) != "PLAYER_MON" or str(Dictionary(restored_battle_state.get("selected_opponent_payload", {})).get("id", "")) != "ENEMY_MON":
		push_error("smoke_test: battle selected payloads did not round-trip")
		quit(1)
		return
	if str(Array(restored_battle_state.get("phase_history", [])).back()) != "resolution":
		push_error("smoke_test: battle phase history did not round-trip")
		quit(1)
		return
	if str(Array(restored_battle_state.get("log_lines", [])).back()) != "smoke":
		push_error("smoke_test: battle log lines did not round-trip")
		quit(1)
		return
	if not bool(battle_state_obj.call("from_dictionary", original_battle_state)):
		push_error("smoke_test: battle state restore after round-trip check failed")
		quit(1)
		return
	var original_battle_runtime_state: Dictionary = Dictionary(battle_runtime.call("get_state"))
	if original_battle_runtime_state.is_empty():
		push_error("smoke_test: battle runtime snapshot missing")
		quit(1)
		return
	var battle_runtime_snapshot := original_battle_runtime_state.duplicate(true)
	battle_runtime_snapshot["battle_frame"] = 77
	battle_runtime_snapshot["accumulator_ms"] = 12.5
	var battle_runtime_state: Dictionary = Dictionary(battle_runtime_snapshot.get("battle_state", {}))
	battle_runtime_state["turn_number"] = 19
	battle_runtime_state["battle_result"] = "draw"
	battle_runtime_snapshot["battle_state"] = battle_runtime_state
	battle_runtime.call("from_state", battle_runtime_snapshot)
	var restored_battle_runtime_state: Dictionary = Dictionary(battle_runtime.call("get_state"))
	if int(restored_battle_runtime_state.get("battle_frame", -1)) != 77:
		push_error("smoke_test: battle runtime frame did not round-trip")
		quit(1)
		return
	if absf(float(restored_battle_runtime_state.get("accumulator_ms", -1.0)) - 12.5) > 0.001:
		push_error("smoke_test: battle runtime accumulator did not round-trip")
		quit(1)
		return
	var restored_battle_runtime_state_payload: Dictionary = Dictionary(restored_battle_runtime_state.get("battle_state", {}))
	if int(restored_battle_runtime_state_payload.get("turn_number", -1)) != 19:
		push_error("smoke_test: battle runtime state did not round-trip")
		quit(1)
		return
	if str(restored_battle_runtime_state_payload.get("battle_result", "")) != "draw":
		push_error("smoke_test: battle runtime result did not round-trip")
		quit(1)
		return
	battle_runtime.call("from_state", original_battle_runtime_state)

	runtime.call("route_to_ui_shell", "intro", "smoke_test")
	await process_frame
	if str(runtime.get("current_scene_route")) != "ui_shell":
		push_error("smoke_test: failed to route back to ui shell")
		quit(1)
		return
	if str(runtime.state.get("ui_page", "")) != "intro":
		push_error("smoke_test: ui page did not persist through route")
		quit(1)
		return
	if not bool(ui_shell.visible) or bool(overworld.visible) or bool(battle.visible):
		push_error("smoke_test: ui shell visibility mismatch")
		quit(1)
		return
	var ui_shell_node: Node = ui_shell
	var original_ui_shell_state: Dictionary = Dictionary(ui_shell_node.call("get_state"))
	if original_ui_shell_state.is_empty():
		push_error("smoke_test: ui shell snapshot missing")
		quit(1)
		return
	if bool(Dictionary(runtime.call("get_state")).get("debug_scene_flow_enabled", true)):
		push_error("smoke_test: debug scene flow should be disabled by default")
		quit(1)
		return
	runtime.call("_queue_button", "start", true)
	runtime.call("_step_simulation")
	var routed_start_frame_input: Dictionary = Dictionary(runtime.call("get_last_frame_input"))
	var routed_start_input: Dictionary = Dictionary(runtime.call("get_last_routed_input"))
	if not bool(Dictionary(routed_start_frame_input.get("pressed", {})).get("start", false)) or str(routed_start_input.get("route", "")) != "ui_shell" or str(runtime.call("get_scene_route")) != "ui_shell":
		push_error("smoke_test: fixed-step start input changed the route unexpectedly")
		quit(1)
		return
	var ui_shell_snapshot := original_ui_shell_state.duplicate(true)
	ui_shell_snapshot["ui_page"] = "oak_intro"
	ui_shell_snapshot["text_box"] = {
		"active": true,
		"visible": true,
		"waiting_for_input": true,
		"input_locked": false,
		"page_index": 0,
		"page_count": 1,
		"page": {"speaker": "TEST", "text": "Round trip", "wait_for_input": true},
		"pages": [{"speaker": "TEST", "text": "Round trip", "wait_for_input": true}],
	}
	ui_shell_snapshot["menu_stack"] = {
		"active": true,
		"menu_open": true,
		"input_locked": false,
		"depth": 1,
		"top_panel": {
			"id": "round_trip_menu",
			"title": "Round Trip Menu",
			"kind": "menu",
			"cursor": 0,
			"entry_count": 1,
			"entries": [{"id": "only", "label": "Only"}],
			"selection": {"id": "only", "label": "Only"},
			"cancelable": true,
			"wrap": true,
			"locked": false,
			"depth": 1,
		},
		"stack": [{
			"id": "round_trip_menu",
			"title": "Round Trip Menu",
			"kind": "menu",
			"entries": [{"id": "only", "label": "Only"}],
			"cursor": 0,
			"cancelable": true,
			"wrap": true,
			"locked": false,
		}],
	}
	ui_shell_node.call("from_state", ui_shell_snapshot)
	var restored_ui_shell_state: Dictionary = Dictionary(ui_shell_node.call("get_state"))
	if str(restored_ui_shell_state.get("ui_page", "")) != "oak_intro":
		push_error("smoke_test: ui shell page did not round-trip")
		quit(1)
		return
	if not bool(Dictionary(restored_ui_shell_state.get("text_box", {})).get("active", false)):
		push_error("smoke_test: ui shell text box did not round-trip")
		quit(1)
		return
	if not bool(Dictionary(restored_ui_shell_state.get("menu_stack", {})).get("menu_open", false)):
		push_error("smoke_test: ui shell menu stack did not round-trip")
		quit(1)
		return
	var ui_shell_dictionary_round_trip: Dictionary = Dictionary(ui_shell_node.call("to_dictionary"))
	ui_shell_dictionary_round_trip["ui_page"] = "continue"
	ui_shell_dictionary_round_trip["page_snapshots"] = {
		"continue": {
			"ui_page": "continue",
			"text_box": {
				"active": true,
				"visible": true,
				"page_index": 0,
				"page_count": 1,
				"pages": [{"speaker": "SAVE", "text": "Continue?"}],
				"current_text": "Continue?",
			},
			"menu_stack": {
				"active": true,
				"menu_open": true,
				"depth": 1,
				"stack": [{
					"id": "continue_menu",
					"title": "Continue",
					"kind": "menu",
					"entries": [{"id": "yes", "label": "Yes"}],
					"cursor": 0,
				}],
			},
			"menu_state": {},
		},
	}
	if not bool(ui_shell_node.call("from_dictionary", ui_shell_dictionary_round_trip)):
		push_error("smoke_test: ui shell from_dictionary failed")
		quit(1)
		return
	var restored_ui_shell_dictionary_state: Dictionary = Dictionary(ui_shell_node.call("to_dictionary"))
	if str(restored_ui_shell_dictionary_state.get("ui_page", "")) != "continue":
		push_error("smoke_test: ui shell dictionary page did not round-trip")
		quit(1)
		return
	if Dictionary(restored_ui_shell_dictionary_state.get("page_snapshots", {})).is_empty():
		push_error("smoke_test: ui shell page snapshots did not round-trip")
		quit(1)
		return
	ui_shell_node.call("from_state", original_ui_shell_state)
	if not ui_shell_node.has_method("open_dialogue") or not ui_shell_node.has_method("consume_input"):
		push_error("smoke_test: ui shell route is missing dialogue methods")
		quit(1)
		return
	ui_shell_node.call("open_dialogue", [
		{"speaker": "Test", "text": "line one"},
		{"speaker": "Test", "text": "line two"},
	])
	if not bool(ui_shell_node.call("has_dialogue")):
		push_error("smoke_test: dialogue did not open")
		quit(1)
		return
	if not bool(ui_shell_node.call("should_block_gameplay_input")):
		push_error("smoke_test: dialogue did not block gameplay input")
		quit(1)
		return
	runtime.call("_queue_button", "a", true)
	runtime.call("_step_simulation")
	var routed_dialogue_frame_input: Dictionary = Dictionary(runtime.call("get_last_frame_input"))
	var dialogue_state: Dictionary = Dictionary(ui_shell_node.call("get_state"))
	var routed_dialogue_input: Dictionary = Dictionary(runtime.call("get_last_routed_input"))
	if not bool(Dictionary(routed_dialogue_frame_input.get("pressed", {})).get("a", false)) or str(routed_dialogue_input.get("route", "")) != "ui_shell" or not bool(routed_dialogue_input.get("consumed", false)):
		push_error("smoke_test: ui shell dialogue did not consume the fixed-step packet")
		quit(1)
		return
	if not bool(dialogue_state.get("blocking_gameplay_input", false)):
		push_error("smoke_test: dialogue state did not persist")
		quit(1)
		return
	if str(dialogue_state.get("ui_page", "")) != "intro":
		push_error("smoke_test: ui page did not persist")
		quit(1)
		return
	var text_box_state: Dictionary = Dictionary(dialogue_state.get("text_box", {}))
	if not bool(text_box_state.get("dialog_active", false)) or not bool(text_box_state.get("text_box_open", false)):
		push_error("smoke_test: dialogue state summary did not expose TS-style flags")
		quit(1)
		return
	if int(text_box_state.get("page_index", -1)) != 0 or int(text_box_state.get("page_count", 0)) != 2:
		push_error("smoke_test: dialogue page counters did not round-trip")
		quit(1)
		return
	if str(text_box_state.get("current_text", "")) != "line one" or str(text_box_state.get("visible_text", "")) != "line one":
		push_error("smoke_test: dialogue text summary did not match the current page")
		quit(1)
		return
	var dialogue_step: Dictionary = Dictionary(ui_shell_node.call("consume_input", {"pressed": {"a": true}}))
	if not bool(dialogue_step.get("consumed", false)):
		push_error("smoke_test: dialogue input was not consumed")
		quit(1)
		return
	var menu_step: Dictionary = Dictionary(ui_shell_node.call("push_menu_panel", {
		"id": "test_menu",
		"title": "Test Menu",
		"entries": [{"id": "one", "label": "One"}, {"id": "two", "label": "Two"}],
	}))
	if str(menu_step.get("id", "")) != "test_menu":
		push_error("smoke_test: menu panel did not push")
		quit(1)
		return
	var menu_state: Dictionary = Dictionary(ui_shell_node.call("get_state"))
	var menu_stack_state: Dictionary = Dictionary(menu_state.get("menu_stack", {}))
	if not bool(menu_stack_state.get("menu_open", false)) or int(menu_stack_state.get("depth", 0)) != 1:
		push_error("smoke_test: menu stack state did not expose TS-style flags")
		quit(1)
		return
	if not str(Dictionary(menu_state.get("top_panel", {})).get("id", "")).begins_with("test_menu"):
		push_error("smoke_test: menu top panel mismatch")
		quit(1)
		return
	ui_shell_node.call("set_ui_page", "oak_intro")
	ui_shell_node.call("clear_menu_stack")
	ui_shell_node.call("close_dialogue")
	text_box_model = null
	text_box_script = null
	menu_stack_model = null
	menu_stack_script = null
	asset_index_model = null
	asset_index_script = null
	decoder_model = null
	decoder_script = null
	game_state_model = null
	game_state_script = null
	map_model = null
	map_data_script = null
	root.queue_free()
	await process_frame

	quit(0)
