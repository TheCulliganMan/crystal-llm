extends SceneTree

const SAVE_ROOT := "/private/tmp/crystal-llm-godot/saves"
const SAVE_SLOT := "parity-audit-canonical"
const BATTLE_SIDE_PLAYER := "player"
const BATTLE_SIDE_ENEMY := "enemy"
const BATTLE_PHASE_TURN_PROMPT := "turn_prompt"
const BATTLE_RESULT_WIN := "win"
const GAME_STATE_SCRIPT := preload("res://scripts/game_state.gd")
const MAP_DATA_SCRIPT := preload("res://scripts/map_data.gd")
const INPUT_LATCH_SCRIPT := preload("res://scripts/input_latch.gd")
const BATTLE_STATE_SCRIPT := preload("res://scripts/battle_state.gd")
const GAME_CORNER_STATE_SCRIPT := preload("res://scripts/game_corner_state.gd")
const SAVE_STORE_SCRIPT := preload("res://scripts/save_store.gd")
const CORE_SYSTEMS_SCRIPT := preload("res://scripts/core_systems_state.gd")
const STORY_EVENTS_SCRIPT := preload("res://scripts/story_events_state.gd")
const PARITY_COVERAGE_INVENTORY_SCRIPT := preload("res://scripts/parity_coverage_inventory.gd")
const SPECIAL_EVENTS_STATE_SCRIPT := preload("res://scripts/special_events_state.gd")
const RENDER_SNAPSHOT_STATE_SCRIPT := preload("res://scripts/render_snapshot_state.gd")

func _initialize() -> void:
	call_deferred("_run")

func _run() -> void:
	var passed: Array[String] = []
	var gaps: Array[Dictionary] = []

	var game_state: Object = GAME_STATE_SCRIPT.new()
	game_state.reset()
	var map_data: Object = MAP_DATA_SCRIPT.new()
	var input_latch: Object = INPUT_LATCH_SCRIPT.new()
	var battle_state: Object = BATTLE_STATE_SCRIPT.new()
	var game_corner_a: Object = GAME_CORNER_STATE_SCRIPT.new()
	var game_corner_b: Object = GAME_CORNER_STATE_SCRIPT.new()
	var save_store: Object = SAVE_STORE_SCRIPT.new()
	var core_systems: Object = CORE_SYSTEMS_SCRIPT.new()
	var story_events: Object = STORY_EVENTS_SCRIPT.new()
	var coverage_inventory: Object = PARITY_COVERAGE_INVENTORY_SCRIPT.new()
	var special_events: Object = SPECIAL_EVENTS_STATE_SCRIPT.new()
	save_store.set_save_root(SAVE_ROOT)
	save_store.ensure_save_root()
	save_store.delete_save_game(SAVE_SLOT)

	game_corner_a.call("seed_rng_state", 0x1234, 0xab, 0xcd)
	game_corner_b.call("seed_rng_state", 0x1234, 0xab, 0xcd)
	var game_corner_slot_result_a: Dictionary = Dictionary(game_corner_a.call("spin_slots", 3, "lucky"))
	var game_corner_card_state_a: Dictionary = Dictionary(game_corner_a.call("shuffle_card_flip"))
	var game_corner_memory_state_a: Dictionary = Dictionary(game_corner_a.call("shuffle_memory_game"))
	var game_corner_unown_state_a: Dictionary = Dictionary(game_corner_a.call("shuffle_unown_puzzle"))
	var game_corner_slot_result_b: Dictionary = Dictionary(game_corner_b.call("spin_slots", 3, "lucky"))
	var game_corner_card_state_b: Dictionary = Dictionary(game_corner_b.call("shuffle_card_flip"))
	var game_corner_memory_state_b: Dictionary = Dictionary(game_corner_b.call("shuffle_memory_game"))
	var game_corner_unown_state_b: Dictionary = Dictionary(game_corner_b.call("shuffle_unown_puzzle"))
	if game_corner_slot_result_a.is_empty() or game_corner_card_state_a.is_empty() or game_corner_memory_state_a.is_empty() or game_corner_unown_state_a.is_empty():
		_record_gap(gaps, "game_corner_state_init", "GameCornerState should initialize slots/card flip/memory/unown state", true, false)
	else:
		_assert_equal(passed, gaps, "game_corner_rng_round_trip", Dictionary(game_corner_a.call("get_rng_state")), Dictionary(game_corner_b.call("get_rng_state")), "GameCornerState RNG state should remain deterministic for identical seeds")
		_assert_equal(passed, gaps, "game_corner_slots_deterministic", Dictionary(game_corner_a.get("slot_machine_state")), Dictionary(game_corner_b.get("slot_machine_state")), "SlotMachine state should remain deterministic for identical seeds")
		_assert_equal(passed, gaps, "game_corner_card_flip_round_trip", game_corner_card_state_a, game_corner_card_state_b, "CardFlipGame state should remain deterministic for identical seeds")
		_assert_equal(passed, gaps, "game_corner_memory_round_trip", game_corner_memory_state_a, game_corner_memory_state_b, "MemoryGame state should remain deterministic for identical seeds")
		_assert_equal(passed, gaps, "game_corner_unown_round_trip", game_corner_unown_state_a, game_corner_unown_state_b, "UnownPuzzle state should remain deterministic for identical seeds")
		var game_corner_snapshot: Dictionary = Dictionary(game_corner_a.call("to_dictionary"))
		var restored_game_corner: Object = GAME_CORNER_STATE_SCRIPT.new()
		if not bool(restored_game_corner.call("from_dictionary", game_corner_snapshot)):
			_record_gap(gaps, "game_corner_snapshot_restore", "GameCornerState should restore its canonical snapshot", game_corner_snapshot, {})
		else:
			_assert_equal(passed, gaps, "game_corner_snapshot_round_trip", game_corner_snapshot, Dictionary(restored_game_corner.call("to_dictionary")), "GameCornerState snapshot should round-trip exactly")

	var render_snapshot_a: Object = RENDER_SNAPSHOT_STATE_SCRIPT.new()
	var render_snapshot_b: Object = RENDER_SNAPSHOT_STATE_SCRIPT.new()
	var render_frame_specs: Array[Dictionary] = [
		{
			"id": "render_title_frame_round_trip",
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
			"id": "render_intro_frame_round_trip",
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
			"id": "render_overworld_frame_round_trip",
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
			"id": "render_menu_frame_round_trip",
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
			"id": "render_battle_frame_round_trip",
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
	for render_spec in render_frame_specs:
		var render_payload: Dictionary = Dictionary(render_spec.get("payload", {}))
		var render_method := str(render_spec.get("method", ""))
		var render_id := str(render_spec.get("id", ""))
		var render_frame_a: Dictionary = Dictionary(render_snapshot_a.call(render_method, render_payload))
		var render_frame_b: Dictionary = Dictionary(render_snapshot_b.call(render_method, render_payload))
		if render_frame_a.is_empty() or render_frame_b.is_empty():
			_record_gap(gaps, render_id, "RenderSnapshotState should capture %s frame payloads" % render_id, render_payload, render_frame_a if render_frame_a.is_empty() else render_frame_b)
		else:
			_assert_equal(passed, gaps, render_id, render_payload, render_frame_a, "render frame payload should round-trip exactly")
			_assert_equal(passed, gaps, "%s_deterministic" % render_id, render_frame_a, render_frame_b, "render frame payload should be deterministic for identical inputs")
	var render_snapshot: Dictionary = Dictionary(render_snapshot_a.call("to_dictionary"))
	var restored_render_snapshot: Object = RENDER_SNAPSHOT_STATE_SCRIPT.new()
	if not bool(restored_render_snapshot.call("from_dictionary", render_snapshot)):
		_record_gap(gaps, "render_snapshot_restore", "RenderSnapshotState should restore its canonical snapshot", render_snapshot, {})
	else:
		_assert_equal(passed, gaps, "render_snapshot_round_trip", render_snapshot, Dictionary(restored_render_snapshot.call("to_dictionary")), "render snapshot state should round-trip exactly")

	_assert_equal(passed, gaps, "boot_title_active_scene", "intro", game_state.active_scene, "default active scene should begin on intro")
	_assert_equal(passed, gaps, "boot_title_ui_page", "title", game_state.get_ui_page(), "default ui page should begin on the title shell")
	_assert_equal(passed, gaps, "boot_title_scene_route", "intro", game_state.get_scene_route(), "default scene route should begin on intro")

	game_state.scene_route = "title"
	game_state.scene_context = {
		"route": "title",
		"source": "boot",
	}
	game_state.scene_handoff = {
		"from": "boot",
		"to": "title",
		"complete": false,
	}
	game_state.pending_scene_handoff = {
		"from": "boot",
		"to": "title",
	}
	game_state.wram["scene_transition"] = {
		"from": "boot",
		"to": "title",
	}
	var boot_round_trip: Object = GAME_STATE_SCRIPT.new()
	boot_round_trip.from_dictionary(game_state.to_dictionary())
	_assert_equal(passed, gaps, "boot_title_scene_route_round_trip", "title", boot_round_trip.get_scene_route(), "scene_route should survive the boot/title round-trip")
	_assert_equal(passed, gaps, "boot_title_handoff_round_trip", game_state.get_scene_handoff(), boot_round_trip.get_scene_handoff(), "scene_handoff should survive the boot/title round-trip")
	_assert_equal(passed, gaps, "boot_title_pending_handoff_round_trip", game_state.get_pending_scene_handoff(), boot_round_trip.get_pending_scene_handoff(), "pending_scene_handoff should survive the boot/title round-trip")

	input_latch.queue_button("right", true)
	var movement_packet: Dictionary = input_latch.begin_frame()
	_assert_equal(passed, gaps, "movement_packet_pressed_right", true, bool(Dictionary(movement_packet.get("pressed", {})).get("right", false)), "fixed-step movement packet should press right")
	_assert_equal(passed, gaps, "movement_packet_down_mask", true, int(movement_packet.get("down_mask", 0)) != 0, "fixed-step movement packet should carry a non-zero down mask")
	var movement_latch: Object = INPUT_LATCH_SCRIPT.new()
	if not bool(movement_latch.from_dictionary(movement_packet)):
		_record_gap(gaps, "movement_packet_round_trip", "InputLatch should restore the movement packet snapshot", movement_packet, {})
	else:
		_assert_equal(passed, gaps, "movement_packet_round_trip", movement_packet, movement_latch.to_dictionary(), "movement packet snapshot should round-trip exactly")

	if not bool(map_data.call("load_default_map")):
		_record_gap(gaps, "map_default_load", "MapData.load_default_map() should succeed on exported assets", true, false)
	else:
		var available_map_keys: Array = Array(map_data.call("get_available_map_keys"))
		var selected_map_key := str(map_data.call("get_selected_map_key"))
		var selected_map_index := int(map_data.call("get_selected_map_index"))
		var map_manifest: Dictionary = Dictionary(map_data.call("get_map_manifest"))
		var map_summary: Dictionary = Dictionary(map_data.call("get_map_summary"))
		var spawn_summary: Dictionary = Dictionary(map_data.call("get_spawn_summary"))
		var current_map_block_key := str(map_data.call("get_current_map_block_key"))
		_assert_equal(passed, gaps, "map_selected_key_present", true, not selected_map_key.is_empty(), "selected map key should resolve from exported metadata")
		_assert_equal(passed, gaps, "map_selected_key_in_available_keys", true, available_map_keys.has(selected_map_key), "selected map key should be part of the available map key list")
		_assert_equal(passed, gaps, "map_selected_index_valid", true, selected_map_index >= 0, "selected map index should be valid")
		_assert_equal(passed, gaps, "map_manifest_contains_selected_map", true, map_manifest.has(selected_map_key), "map manifest should contain the selected map")
		_assert_equal(passed, gaps, "map_summary_key", selected_map_key, str(map_summary.get("map_key", "")), "map summary should report the selected map key")
		_assert_equal(passed, gaps, "map_spawn_summary_non_empty", true, not spawn_summary.is_empty(), "spawn summary should expose the current spawn record")
		_assert_equal(passed, gaps, "map_block_key_non_empty", true, not current_map_block_key.is_empty(), "current map block key should be present")
		if not current_map_block_key.is_empty():
			var map_blocks: Dictionary = map_data.map_blocks
			_assert_equal(passed, gaps, "map_block_key_in_map_blocks", true, map_blocks.has(current_map_block_key), "current map block key should resolve into the map block table")
		game_state.active_scene = "overworld"
		game_state.scene_route = "overworld"
		game_state.overworld_state = {
			"location": {
				"scene": "overworld",
				"map_id": selected_map_key,
				"warp_id": "",
				"x": 0,
				"y": 0,
			},
			"player": {
				"x": 0,
				"y": 0,
				"facing": "right",
				"moving": true,
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
		var map_round_trip: Object = MAP_DATA_SCRIPT.new()
		if not bool(map_round_trip.from_dictionary(map_data.to_dictionary())):
			_record_gap(gaps, "map_round_trip", "MapData should restore its exported-asset-backed state", map_data.to_dictionary(), {})
		else:
			_assert_equal(passed, gaps, "map_round_trip_selected_key", selected_map_key, map_round_trip.get_selected_map_key(), "MapData selected key should survive round-trip")
			_assert_equal(passed, gaps, "map_round_trip_manifest_contains_selected_map", true, Dictionary(map_round_trip.call("get_map_manifest")).has(selected_map_key), "MapData manifest should survive round-trip")

	game_state.ui_page = "ui_shell"
	game_state.ui_dialogue_state = {
		"active": true,
		"visible": true,
		"page_index": 0,
		"page_count": 2,
		"text": "PARITY AUDIT",
		"current_text": "PARITY AUDIT",
		"visible_text": "PARITY AUDIT",
	}
	game_state.ui_menu_state = {
		"menu_open": true,
		"input_locked": true,
		"depth": 1,
		"stack": [
			{"kind": "main_menu", "index": 0},
		],
	}
	game_state.ui_shell_state = {
		"ui_page": "ui_shell",
		"text_box": game_state.ui_dialogue_state.duplicate(true),
		"menu_stack": game_state.ui_menu_state.duplicate(true),
		"page_snapshots": {
			"title": {
				"ui_page": "title",
			},
		},
	}
	input_latch.queue_button("start", true)
	var menu_packet: Dictionary = input_latch.begin_frame()
	_assert_equal(passed, gaps, "menu_packet_pressed_start", true, bool(Dictionary(menu_packet.get("pressed", {})).get("start", false)), "menu packet should press start")
	game_state.ui_menu_state["input_locked"] = false
	game_state.ui_menu_state["menu_open"] = false
	game_state.ui_dialogue_state["active"] = false
	game_state.ui_dialogue_state["visible"] = false
	game_state.ui_dialogue_state["current_text"] = ""
	game_state.ui_dialogue_state["visible_text"] = ""
	game_state.ui_shell_state["text_box"] = game_state.ui_dialogue_state.duplicate(true)
	game_state.ui_shell_state["menu_stack"] = game_state.ui_menu_state.duplicate(true)
	var ui_round_trip: Object = GAME_STATE_SCRIPT.new()
	ui_round_trip.from_dictionary(game_state.to_dictionary())
	_assert_equal(passed, gaps, "ui_shell_round_trip", game_state.get_ui_shell_state(), ui_round_trip.get_ui_shell_state(), "ui shell nested state should survive the round-trip")
	_assert_equal(passed, gaps, "ui_dialogue_round_trip", game_state.get_ui_dialogue_state(), ui_round_trip.get_ui_dialogue_state(), "ui dialogue state should survive the round-trip")
	_assert_equal(passed, gaps, "ui_menu_round_trip", game_state.get_ui_menu_state(), ui_round_trip.get_ui_menu_state(), "ui menu state should survive the round-trip")

	core_systems.configure({
		"time": {
			"year": 2000,
			"month": 1,
			"day": 1,
			"hour": 21,
			"last_daily_reset": {"year": 2000, "month": 1, "day": 1},
		},
		"wram": {
			"step_count": 127,
			"poison_step_count": 3,
			"happiness_step_count": 1,
			"daily_rematch_flags": [1, 1],
			"daily_phone_item_flags": [1],
			"daily_phone_time_of_day_flags": [1],
			"event_flags": {"FRUITTREE_ROUTE_29_COLLECTED": true, "STORY_FLAG": true},
			"engine_flags": {"ENGINE_DAILY_BUG_CONTEST": true},
			"wKenjiBreakTimer": 1,
		},
		"sram": {
			"money": 3000,
			"party": {
				"pokemon": [
					{"species": "CHIKORITA", "nickname": "CHIKORITA", "hp": 12, "status": "PSN", "happiness": 180},
					{"species": "TOGEPI", "nickname": "EGG", "hp": 1, "status": "", "happiness": 1, "is_egg": true},
				],
			},
			"items": {"POTION": 1},
			"event_flags": {"FRUITTREE_ROUTE_29_COLLECTED": true, "STORY_FLAG": true},
			"mystery_gift_unlocked": true,
			"mystery_gift": {"daily_partner_ids": [101, 202]},
		},
	})
	var step_result: Dictionary = Dictionary(core_systems.process_step())
	var core_snapshot_after_step: Dictionary = Dictionary(core_systems.to_dictionary())
	var core_wram: Dictionary = Dictionary(core_snapshot_after_step.get("wram", {}))
	var core_sram: Dictionary = Dictionary(core_snapshot_after_step.get("sram", {}))
	var core_party: Array = Array(Dictionary(core_sram.get("party", {})).get("pokemon", []))
	_assert_equal(passed, gaps, "core_systems_step_increment", 128, int(core_wram.get("step_count", -1)), "step system should increment the WRAM step counter as a byte")
	_assert_equal(passed, gaps, "core_systems_egg_hatch", true, bool(step_result.get("egg_hatched", false)), "step system should hatch an egg when the 0x80 egg counter reaches zero")
	_assert_equal(passed, gaps, "core_systems_poison_tick", 11, int(Dictionary(core_party[0]).get("hp", -1)), "poison should damage the poisoned party member every fourth poison step")
	var daily_result: Dictionary = Dictionary(core_systems.process_daily_events({"year": 2000, "month": 1, "day": 2}))
	var core_after_daily: Dictionary = Dictionary(core_systems.to_dictionary())
	var daily_wram: Dictionary = Dictionary(core_after_daily.get("wram", {}))
	var daily_sram: Dictionary = Dictionary(core_after_daily.get("sram", {}))
	_assert_equal(passed, gaps, "core_systems_daily_reset", true, bool(daily_result.get("reset", false)), "daily system should reset when the calendar day changes")
	_assert_equal(passed, gaps, "core_systems_daily_flags_clear", 0, int(daily_wram.get("daily_flags1", -1)), "daily reset should clear daily flags")
	_assert_equal(passed, gaps, "core_systems_fruit_tree_clear", false, Dictionary(daily_wram.get("event_flags", {})).has("FRUITTREE_ROUTE_29_COLLECTED"), "daily reset should clear fruit tree event flags")
	_assert_equal(passed, gaps, "core_systems_mystery_gift_clear", [], Array(Dictionary(daily_sram.get("mystery_gift", {})).get("daily_partner_ids", [])), "daily reset should clear mystery gift partner ids")
	core_systems.configure_shop("cherrygrove_mart", [{"identifier": "POTION", "displayName": "POTION", "price": 300}, {"identifier": "ANTIDOTE", "displayName": "ANTIDOTE", "price": 100}], 1000, {"POTION": 1})
	var buy_result: Dictionary = Dictionary(core_systems.buy_selected(2))
	var shop_after_buy: Dictionary = Dictionary(core_systems.to_dictionary())
	_assert_equal(passed, gaps, "core_systems_shop_buy", true, bool(buy_result.get("success", false)), "shop buy should add items and debit money")
	_assert_equal(passed, gaps, "core_systems_shop_buy_price", "¥000600", str(buy_result.get("message", "")), "shop buy should format credited price with six digits")
	_assert_equal(passed, gaps, "core_systems_shop_inventory", 3, int(Dictionary(Dictionary(shop_after_buy.get("sram", {})).get("items", {})).get("POTION", -1)), "shop buy should update the item stack")
	var core_round_trip: Object = CORE_SYSTEMS_SCRIPT.new()
	if not bool(core_round_trip.from_dictionary(shop_after_buy)):
		_record_gap(gaps, "core_systems_round_trip", "CoreSystemsState should restore system payloads", shop_after_buy, {})
	else:
		_assert_equal(passed, gaps, "core_systems_round_trip", shop_after_buy, Dictionary(core_round_trip.to_dictionary()), "core system payloads should survive round-trip")

	special_events.sync_runtime_state({
		"sram": {
			"day_care": {
				"man": {"pokemon": {"species": "CYNDAQUIL", "nickname": "Cinder", "level": 12}},
				"lady": {"pokemon": {"species": "TOTODILE", "nickname": "Wave", "level": 11}},
				"egg_present": true,
				"steps_since_last_egg": 8,
				"can_breed": true,
			},
			"mystery_gift_unlocked": true,
			"mystery_gift": {"stored_item": "NUGGET", "backup_item": "NUGGET", "daily_partner_ids": [101, 202]},
			"lucky_number_day": 4,
			"lucky_id_number": 12345,
			"current_pc_box": 1,
			"money": 4567,
			"moms_money": 1200,
			"mom_saving_active": true,
			"mom_saving_some_money": false,
			"blue_card_balance": 24,
			"buenas_password_category": 0,
			"buenas_password_index": 1,
			"items": {"SLOWPOKE_TAIL_APRICORN": 2},
			"party": {
				"pokemon": [
					{"species": "CYNDAQUIL", "original_trainer_id": 12345},
				],
			},
		},
		"wram": {
			"maptile_decorations_visible": true,
			"decorations_visible": true,
			"event_flags": {"EVENT_TOGGLED": true},
			"engine_flags": {"ENGINE_POKEDEX": true, "ENGINE_DAILY_BUG_CONTEST": true},
			"lucky_number_show_flag": true,
			"wPartyCount": 1,
			"wCurDay": 4,
			"wHallOfFameCount": 1,
			"bug_contest_state": {
				"timer_active": true,
				"park_balls_remaining": 20,
				"caught_species": "CATERPIE",
				"caught_level": 12,
				"pending_caught_mon": {"species": "SCYTHER"},
			},
			"bug_contest_results": {"first_place": "SCYTHER"},
			"buenas_password_category": 0,
			"buenas_password_index": 1,
		},
		"hram": {
			"hHours": 13,
			"hMinutes": 5,
			"hSeconds": 7,
		},
		"specials": {
			"magnet_train": {
				"count": 2,
				"direction_token": "1",
				"destination": "GoldenrodMagnetTrainStation",
				"scene": "SCENE_GOLDENRODMAGNETTRAINSTATION_ARRIVE_FROM_SAFFRON",
			},
		},
		"player_name": "Chris",
	})
	special_events.queue_intent("mom", "bank_of_mom", {"action": "take"})
	special_events.queue_intent("pc_helpers", "pokemon_center_pc", {"selected_index": 0, "selected_action": "player_pc"})
	var special_state: Dictionary = Dictionary(special_events.call("get_state"))
	var special_domains: Dictionary = Dictionary(special_state.get("domains", {}))
	_assert_equal(passed, gaps, "special_events_domain_count", true, special_domains.keys().size() >= 10, "special events state should expose all major domain summaries")
	_assert_equal(passed, gaps, "special_events_day_care_payloads", true, Array(Dictionary(special_domains.get("day_care", {})).get("actions", [])).size() >= 5, "day care should expose serialized action payloads")
	_assert_equal(passed, gaps, "special_events_pc_helper_entries", true, Array(Dictionary(Dictionary(special_domains.get("pc_helpers", {})).get("summary", {})).get("entries", [])).size() >= 3, "PC helper flows should expose hub entries")
	_assert_equal(passed, gaps, "special_events_mom_state", true, bool(Dictionary(Dictionary(special_domains.get("mom", {})).get("summary", {})).get("mom_saving_active", false)), "Mom banking state should preserve saving flags")
	var special_snapshot: Dictionary = Dictionary(special_events.call("to_dictionary"))
	var restored_special_events: Object = SPECIAL_EVENTS_STATE_SCRIPT.new()
	if not bool(restored_special_events.call("from_dictionary", special_snapshot)):
		_record_gap(gaps, "special_events_state_round_trip", "SpecialEventsState should restore its canonical snapshot", special_snapshot, {})
	else:
		_assert_equal(passed, gaps, "special_events_state_round_trip", special_snapshot, Dictionary(restored_special_events.call("to_dictionary")), "special events snapshot should round-trip exactly")

	story_events.enqueue_script([
		{"op": "setflag", "flag": "EVENT_GOT_STARTER"},
		{"op": "setvar", "name": "starter", "value": "CHIKORITA"},
		{"op": "writetext", "text": "Come again!", "speaker": "CLERK"},
		{"op": "applymovement", "object_id": "rival", "steps": ["step_left", "step_left"]},
		{"op": "warp", "map": "NEW_BARK_TOWN", "x": 5, "y": 6, "facing": "down"},
		{"op": "trainerbattle", "trainer": "RIVAL1", "battle_type": "scripted"},
		{"op": "playmusic", "cue": "MUSIC_RIVAL_ENCOUNTER"},
		{"op": "giveitem", "item": "POTION", "quantity": 2},
		{"op": "end"},
	])
	story_events.step()
	var story_var_result: Dictionary = Dictionary(story_events.step())
	var story_text_result: Dictionary = Dictionary(story_events.step())
	_assert_equal(passed, gaps, "story_events_flag_set", true, bool(story_events.get_flag("EVENT_GOT_STARTER")), "story event setflag should update the flag table")
	_assert_equal(passed, gaps, "story_events_setvar", "CHIKORITA", str(Dictionary(story_var_result.get("payload", {})).get("value", "")), "story event setvar should preserve variable payloads")
	_assert_equal(passed, gaps, "story_events_text_wait", true, bool(Dictionary(story_text_result.get("runner", {})).get("waiting_for_input", false)), "writetext should gate the script runner on input")
	story_events.answer_yes_no(true)
	var movement_result: Dictionary = Dictionary(story_events.step())
	var warp_result: Dictionary = Dictionary(story_events.step())
	var battle_event_result: Dictionary = Dictionary(story_events.step())
	var audio_result: Dictionary = Dictionary(story_events.step())
	var item_result: Dictionary = Dictionary(story_events.step())
	_assert_equal(passed, gaps, "story_events_movement_payload", ["step_left", "step_left"], Array(Dictionary(movement_result.get("payload", {})).get("steps", [])), "applymovement should serialize object movement steps")
	_assert_equal(passed, gaps, "story_events_warp_payload", "NEW_BARK_TOWN", str(Dictionary(warp_result.get("payload", {})).get("map", "")), "warp should serialize map target")
	_assert_equal(passed, gaps, "story_events_battle_payload", "RIVAL1", str(Dictionary(battle_event_result.get("payload", {})).get("trainer", "")), "trainerbattle should serialize trainer target")
	_assert_equal(passed, gaps, "story_events_audio_payload", "MUSIC_RIVAL_ENCOUNTER", str(Dictionary(audio_result.get("payload", {})).get("cue", "")), "playmusic should serialize the cue")
	_assert_equal(passed, gaps, "story_events_item_payload", "POTION", str(Dictionary(item_result.get("payload", {})).get("item", "")), "giveitem should serialize item payload")
	var story_snapshot: Dictionary = Dictionary(story_events.to_dictionary())
	var story_round_trip: Object = STORY_EVENTS_SCRIPT.new()
	if not bool(story_round_trip.from_dictionary(story_snapshot)):
		_record_gap(gaps, "story_events_round_trip", "StoryEventsState should restore command-runner payloads", story_snapshot, {})
	else:
		_assert_equal(passed, gaps, "story_events_round_trip", story_snapshot, Dictionary(story_round_trip.to_dictionary()), "story event runner payloads should survive round-trip")

	battle_state.reset()
	var starting_revision: int = battle_state.get_state_revision()
	battle_state.set_context({
		"battle_id": "canonical-journey",
		"battle_kind": "wild",
		"battle_label": "parity audit",
	})
	battle_state.set_asset_summary({
		"pokemon_count": 1,
		"move_count": 1,
	})
	battle_state.set_selected_player_payload({
		"name": "CHIKORITA",
		"side": "player",
	})
	battle_state.set_selected_opponent_payload({
		"name": "PIDGEY",
		"side": "enemy",
	})
	battle_state.advance_phase()
	battle_state.advance_phase()
	battle_state.enqueue_command({
		"actor": {
			"side": BATTLE_SIDE_PLAYER,
			"name": "CHIKORITA",
		},
		"target": {
			"side": BATTLE_SIDE_ENEMY,
			"name": "PIDGEY",
		},
		"move_payload": {
			"name": "TACKLE",
			"type": "NORMAL",
			"pp": 10,
			"current_pp": 10,
			"power": 35,
			"accuracy": 100,
		},
	})
	if not bool(battle_state.begin_resolution()):
		_record_gap(gaps, "battle_begin_resolution", "battle resolution should start from a queued command", true, false)
	else:
		var resolution: Dictionary = battle_state.get_last_turn_resolution()
		_assert_equal(passed, gaps, "battle_resolution_valid", true, bool(resolution.get("valid", false)), "battle resolution should be valid for the queued command")
		_assert_equal(passed, gaps, "battle_phase_history_contains_turn_prompt", true, battle_state.get_phase_history().has(BATTLE_PHASE_TURN_PROMPT), "battle phase history should capture the turn prompt phase")
		battle_state.complete_resolution("turn resolved")
		battle_state.mark_complete(BATTLE_RESULT_WIN, {
			"winner": "player",
		})
		var consumed_events: Array = battle_state.consume_resolution_events()
		_assert_equal(passed, gaps, "battle_resolution_events_consumed", true, consumed_events.size() > 0, "battle should emit at least one resolution event")
		_assert_equal(passed, gaps, "battle_resolution_events_cleared", true, battle_state.get_resolution_event_count() == 0, "battle resolution events should clear after consumption")
		_assert_equal(passed, gaps, "battle_state_revision_advanced", true, battle_state.get_state_revision() > starting_revision, "battle state revision should advance through the turn")
		var battle_round_trip: Object = BATTLE_STATE_SCRIPT.new()
		if not bool(battle_round_trip.from_dictionary(battle_state.to_dictionary())):
			_record_gap(gaps, "battle_round_trip", "BattleState should restore its resolved state", battle_state.to_dictionary(), {})
		else:
			_assert_equal(passed, gaps, "battle_round_trip_phase_history", battle_state.get_phase_history(), battle_round_trip.get_phase_history(), "battle phase history should survive round-trip")
			_assert_equal(passed, gaps, "battle_round_trip_result_state", battle_state.get_battle_result_state(), battle_round_trip.get_battle_result_state(), "battle result state should survive round-trip")

	game_state.battle_state = battle_state.to_dictionary()
	game_state.frame_counter = input_latch.frame_index()
	game_state.loaded_asset_summary = {
		"map_key": map_data.get_selected_map_key(),
		"map_name": map_data.get_map_name(map_data.get_selected_map_key()),
		"map_block_key": map_data.get_current_map_block_key(),
	}
	if not bool(save_store.save_game(SAVE_SLOT, game_state)):
		_record_gap(gaps, "save_game", "SaveStore should save the canonical journey snapshot", true, false)
	else:
		var saved_snapshot: Dictionary = game_state.to_dictionary()
		var save_metadata: Dictionary = save_store.load_save_metadata(SAVE_SLOT)
		_assert_equal(passed, gaps, "save_metadata_present", true, not save_metadata.is_empty(), "save metadata should be written alongside the save")
		var load_result: Dictionary = save_store.load_game(SAVE_SLOT)
		if not bool(load_result.get("ok", false)):
			_record_gap(gaps, "load_game", "SaveStore should reload the canonical journey snapshot", true, load_result)
		else:
			var loaded_state_value: Variant = load_result.get("state", null)
			if loaded_state_value == null or not (loaded_state_value is Object):
				_record_gap(gaps, "loaded_state_object", "SaveStore should return a loaded GameState object", "Object", loaded_state_value)
			else:
				var loaded_state: Object = loaded_state_value
				var loaded_state_dict := Dictionary(loaded_state.call("to_dictionary"))
				var normalized_saved_snapshot: Dictionary = _normalize_numeric_payload(saved_snapshot)
				var normalized_loaded_state_dict: Dictionary = _normalize_numeric_payload(loaded_state_dict)
				_assert_equal(passed, gaps, "save_load_snapshot_identity", normalized_saved_snapshot, normalized_loaded_state_dict, "loaded state should match the saved snapshot exactly")
				_assert_equal(passed, gaps, "save_load_metadata_identity", save_metadata, Dictionary(loaded_state.call("get_save_metadata")), "loaded state should restore save metadata identity")
				_assert_equal(passed, gaps, "save_load_scene_route_identity", game_state.get_scene_route(), str(loaded_state.call("get_scene_route")), "loaded scene route should match the saved route")
				_assert_equal(passed, gaps, "save_load_ui_shell_identity", game_state.get_ui_shell_state(), Dictionary(loaded_state.call("get_ui_shell_state")), "loaded ui shell state should match the saved state")

	save_store.delete_save_game(SAVE_SLOT)
	var coverage_report: Dictionary = Dictionary(coverage_inventory.call("generate_report"))
	var harness_scripts: Dictionary = Dictionary(coverage_report.get("harness_scripts", {}))
	_assert_equal(passed, gaps, "harness_scripts_present", true, bool(harness_scripts.get("covered", false)), "parity harness scripts should be represented in the inventory")
	_assert_equal(passed, gaps, "harness_scripts_missing_count", 0, int(harness_scripts.get("missing_count", 0)), "no parity harness scripts should be missing from the inventory")
	_assert_equal(passed, gaps, "harness_scripts_present_list", ["smoke_test.gd", "parity_journey_test.gd", "parity_audit_test.gd"], Array(harness_scripts.get("present", [])).duplicate(true), "the canonical harness scripts should be listed as present")
	var special_events_domain: Dictionary = {}
	for domain in Array(coverage_report.get("domains", [])):
		var domain_entry: Dictionary = Dictionary(domain)
		if str(domain_entry.get("id", "")) == "special_events":
			special_events_domain = domain_entry
			break
	_assert_equal(passed, gaps, "special_events_domain_in_inventory", true, not special_events_domain.is_empty(), "special events should be represented in the coverage inventory")
	var required_coverage_gaps: Array = Array(coverage_report.get("required_gaps", []))
	for gap in required_coverage_gaps:
		gaps.append(Dictionary(gap))

	var report := {
		"journey": "canonical_state_audit",
		"passed": passed,
		"gaps": gaps,
		"coverage_inventory": coverage_report,
		"summary": {
			"scene_route": game_state.get_scene_route(),
			"ui_page": game_state.get_ui_page(),
			"selected_map_key": map_data.get_selected_map_key(),
			"selected_map_index": map_data.get_selected_map_index(),
			"battle_phase": battle_state.turn_phase,
			"battle_result": battle_state.battle_result,
			"save_slot": SAVE_SLOT,
		},
	}
	print(JSON.stringify(report))

	if gaps.is_empty():
		quit(0)
		return

	push_error("parity_audit_test: %d parity gaps" % gaps.size())
	quit(1)

func _assert_equal(passed: Array[String], gaps: Array[Dictionary], id: String, expected: Variant, actual: Variant, detail: String) -> bool:
	if expected == actual:
		passed.append(id)
		return true
	_record_gap(gaps, id, detail, expected, actual)
	return false

func _record_gap(gaps: Array[Dictionary], id: String, detail: String, expected: Variant, actual: Variant) -> void:
	gaps.append({
		"id": id,
		"detail": detail,
		"expected": expected,
		"actual": actual,
	})

func _normalize_numeric_payload(value: Variant) -> Variant:
	match typeof(value):
		TYPE_DICTIONARY:
			var normalized: Dictionary = {}
			for key in Dictionary(value).keys():
				normalized[key] = _normalize_numeric_payload(Dictionary(value)[key])
			return normalized
		TYPE_ARRAY:
			var normalized_array: Array = []
			for entry in Array(value):
				normalized_array.append(_normalize_numeric_payload(entry))
			return normalized_array
		TYPE_FLOAT:
			var float_value := float(value)
			if is_equal_approx(float_value, floor(float_value)):
				return int(roundi(float_value))
			return float_value
		_:
			return value
