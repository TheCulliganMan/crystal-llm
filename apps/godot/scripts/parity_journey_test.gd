extends SceneTree

const MAIN_SCENE := "res://scenes/main.tscn"
const SAVE_SLOT := "parity-journey-smoke"

func _initialize() -> void:
	call_deferred("_run")

func _fail(message: String) -> void:
	push_error("parity_journey_test: %s" % message)
	quit(1)

func _load_main_runtime() -> Node:
	var runtime_script: Script = load("res://scripts/game_runtime.gd")
	if runtime_script == null:
		_fail("failed to load game runtime script")
		return null
	var map_data_script: Script = load("res://scripts/map_data.gd")
	if map_data_script == null:
		_fail("failed to load map data script")
		return null
	var save_store_script: Script = load("res://scripts/save_store.gd")
	if save_store_script == null:
		_fail("failed to load save store script")
		return null
	var scene: PackedScene = load(MAIN_SCENE)
	if scene == null:
		_fail("failed to load main scene")
		return null
	var runtime: Node = scene.instantiate()
	if runtime == null:
		_fail("failed to instantiate main scene")
		return null
	get_root().add_child(runtime)
	await process_frame
	return runtime

func _require_methods(target: Variant, label: String, methods: Array[String]) -> bool:
	if target == null or not (target is Object):
		_fail("%s is missing" % label)
		return false
	for method_name in methods:
		if not target.has_method(method_name):
			_fail("%s missing public method %s" % [label, method_name])
			return false
	return true

func _assert_route(runtime: Node, route_name: String, reason: String) -> bool:
	if str(runtime.call("get_scene_route")) != route_name:
		_fail("%s route mismatch: expected %s, got %s" % [reason, route_name, str(runtime.call("get_scene_route"))])
		return false
	var snapshot: Dictionary = Dictionary(runtime.call("to_dictionary"))
	if str(snapshot.get("scene_route", "")) != route_name or str(snapshot.get("active_scene", "")) != route_name:
		_fail("%s route did not persist in runtime snapshot" % reason)
		return false
	var context: Dictionary = Dictionary(runtime.call("get_scene_context"))
	if str(context.get("route", "")) != route_name:
		_fail("%s scene context route mismatch" % reason)
		return false
	return true

func _assert_visibility(root: Node, visible_route: String) -> bool:
	var expected_visible := {
		"Title": visible_route == "title",
		"IntroSequence": visible_route == "intro_sequence",
		"UIShell": visible_route == "ui_shell",
		"Overworld": visible_route == "overworld",
		"Battle": visible_route == "battle",
	}
	for node_name in expected_visible.keys():
		var item: CanvasItem = root.get_node_or_null(node_name)
		if item == null:
			_fail("route node missing: %s" % node_name)
			return false
		if bool(item.visible) != bool(expected_visible[node_name]):
			_fail("visibility mismatch for %s on route %s" % [node_name, visible_route])
			return false
	return true

func _run() -> void:
	var runtime: Node = await _load_main_runtime()
	if runtime == null:
		return
	if not _require_methods(runtime, "runtime", [
		"route_to_title",
		"route_to_intro_sequence",
		"route_to_ui_shell",
		"route_to_overworld",
		"route_to_battle",
		"get_scene_route",
		"get_scene_context",
		"get_scene_handoff",
		"get_pending_scene_handoff",
		"get_loaded_asset_summary",
		"get_ui_page",
		"get_last_frame_input",
		"get_last_routed_input",
		"to_dictionary",
		"from_dictionary",
		"save_current_state",
		"load_saved_state",
		"delete_saved_state",
	]):
		return

	var asset_summary: Dictionary = Dictionary(runtime.call("get_loaded_asset_summary"))
	if int(asset_summary.get("pokemon_count", 0)) <= 0 or int(asset_summary.get("move_count", 0)) <= 0:
		_fail("runtime asset summary did not load core counts")
		return
	var map_data_script: Script = load("res://scripts/map_data.gd")
	if map_data_script == null:
		_fail("failed to load map data script")
		return
	var map_data: Variant = map_data_script.new()
	if not bool(map_data.call("load_default_map")):
		_fail("map data failed to load the exported default map")
		return
	if not map_data.has_method("get_available_map_keys") or not map_data.has_method("get_selected_map_key") or not map_data.has_method("get_selected_map_index") or not map_data.has_method("get_map_manifest") or not map_data.has_method("get_map_summary") or not map_data.has_method("get_spawn_summary") or not map_data.has_method("get_current_map_block_key"):
		_fail("map data is missing selector or manifest helpers")
		return
	var available_map_keys: Array = Array(map_data.call("get_available_map_keys"))
	if available_map_keys.is_empty():
		_fail("map data available map keys were empty")
		return
	var selected_map_key := str(map_data.call("get_selected_map_key"))
	var selected_map_index := int(map_data.call("get_selected_map_index"))
	var map_summary: Dictionary = Dictionary(map_data.call("get_map_summary"))
	var resolved_map_key := selected_map_key
	if resolved_map_key.is_empty():
		resolved_map_key = str(map_summary.get("map_key", ""))
	if resolved_map_key.is_empty():
		_fail("map data selected map key was invalid")
		return
	var map_manifest: Dictionary = Dictionary(map_data.call("get_map_manifest"))
	if map_manifest.is_empty():
		_fail("map data manifest did not expose imported maps")
		return
	if str(map_summary.get("map_key", "")) != resolved_map_key or str(map_summary.get("map_name", "")).is_empty() or selected_map_index < -1:
		_fail("map data summary did not match the selected exported map")
		return
	var spawn_summary: Dictionary = Dictionary(map_data.call("get_spawn_summary"))
	if spawn_summary.is_empty() or str(spawn_summary.get("map_name", "")).is_empty():
		_fail("map data spawn summary did not expose the selected exported map")
		return
	if str(map_data.call("get_current_map_block_key")).is_empty():
		_fail("map data current map block key was empty")
		return

	runtime.call("delete_saved_state", SAVE_SLOT)

	runtime.call("route_to_title", "parity_journey")
	await process_frame
	if not _assert_route(runtime, "title", "title") or not _assert_visibility(runtime, "title"):
		return
	if not Dictionary(runtime.call("get_pending_scene_handoff")).is_empty():
		_fail("title route left a pending scene handoff behind")
		return
	var title_screen: Node = runtime.get_node_or_null("Title")
	if not _require_methods(title_screen, "title screen", ["from_dictionary", "to_dictionary"]):
		return
	if not bool(title_screen.call("from_dictionary", {
		"screen": "title",
		"phase": "timeout",
		"title_timer": 40,
		"pending_action": "intro_sequence",
		"route_entry": true,
		"last_input": {"pressed": {"start": true}, "released": {}, "down": {"start": true}},
	})):
		_fail("title screen snapshot restore failed")
		return
	var restored_title: Dictionary = Dictionary(title_screen.call("to_dictionary"))
	if str(restored_title.get("screen", "")) != "title" or str(restored_title.get("phase", "")).is_empty():
		_fail("title screen snapshot restore did not preserve title screen state")
		return

	runtime.call("route_to_intro_sequence", "parity_journey")
	await process_frame
	if not _assert_route(runtime, "intro_sequence", "intro") or not _assert_visibility(runtime, "intro_sequence"):
		return
	if not Dictionary(runtime.call("get_pending_scene_handoff")).is_empty():
		_fail("intro route left a pending scene handoff behind")
		return
	var intro_screen: Node = runtime.get_node_or_null("IntroSequence")
	if not _require_methods(intro_screen, "intro screen", ["from_dictionary", "to_dictionary"]):
		return
	if not bool(intro_screen.call("from_dictionary", {
		"screen": "intro_sequence",
		"scene_index": 99,
		"frame_counter": 15,
		"finished": true,
		"skip_requested": true,
	})):
		_fail("intro screen snapshot restore failed")
		return
	var restored_intro: Dictionary = Dictionary(intro_screen.call("to_dictionary"))
	if int(restored_intro.get("scene_index", -1)) != 3 or int(restored_intro.get("frame_counter", -1)) != 15 or not bool(restored_intro.get("finished", false)):
		_fail("intro screen snapshot restore did not clamp/preserve state")
		return
	var intro_handoff: Dictionary = Dictionary(runtime.call("get_scene_handoff"))
	if str(intro_handoff.get("to_scene", "")) != "intro_sequence" or str(intro_handoff.get("reason", "")) != "parity_journey":
		_fail("intro handoff did not record public route transition")
		return

	runtime.call("route_to_ui_shell", "title", "parity_journey")
	await process_frame
	if not _assert_route(runtime, "ui_shell", "ui shell") or not _assert_visibility(runtime, "ui_shell"):
		return
	if not Dictionary(runtime.call("get_pending_scene_handoff")).is_empty():
		_fail("ui shell route left a pending scene handoff behind")
		return
	if str(runtime.call("get_ui_page")) != "title":
		_fail("ui shell did not preserve title page")
		return
	if bool(Dictionary(runtime.call("get_state")).get("debug_scene_flow_enabled", true)):
		_fail("debug scene flow should be disabled by default")
		return
	runtime.call("_queue_button", "start", true)
	runtime.call("_step_simulation")
	var routed_start_frame_input: Dictionary = Dictionary(runtime.call("get_last_frame_input"))
	var routed_start_input: Dictionary = Dictionary(runtime.call("get_last_routed_input"))
	if not bool(Dictionary(routed_start_frame_input.get("pressed", {})).get("start", false)) or str(routed_start_input.get("route", "")) != "ui_shell" or str(runtime.call("get_scene_route")) != "ui_shell":
		_fail("fixed-step start input changed the route unexpectedly")
		return

	var ui_shell: Node = runtime.get_node_or_null("UIShell")
	if not _require_methods(ui_shell, "ui shell", [
		"push_menu_panel",
		"pop_menu_panel",
		"clear_menu_stack",
		"has_menu_stack",
		"get_top_panel",
		"get_state",
		"from_dictionary",
		"to_dictionary",
	]):
		return
	if not bool(ui_shell.call("from_dictionary", {
		"ui_page": "title",
		"text_box": {
			"active": true,
			"visible": true,
			"page_index": 0,
			"pages": [{"speaker": "Parity", "text": "Snapshot restore"}],
			"current_text": "Snapshot restore",
		},
		"menu_stack": {
			"active": true,
			"menu_open": true,
			"depth": 1,
			"stack": [{
				"id": "snapshot_menu",
				"title": "Snapshot Menu",
				"entries": [{"id": "ok", "label": "OK"}],
			}],
		},
	})):
		_fail("ui shell snapshot restore failed")
		return
	var restored_ui_shell: Dictionary = Dictionary(ui_shell.call("to_dictionary"))
	if not bool(Dictionary(restored_ui_shell.get("text_box", {})).get("active", false)) or not bool(Dictionary(restored_ui_shell.get("menu_stack", {})).get("menu_open", false)):
		_fail("ui shell snapshot restore did not preserve text/menu state")
		return
	runtime.call("_queue_button", "a", true)
	runtime.call("_step_simulation")
	var routed_dialogue_frame_input: Dictionary = Dictionary(runtime.call("get_last_frame_input"))
	var routed_dialogue_input: Dictionary = Dictionary(runtime.call("get_last_routed_input"))
	if not bool(Dictionary(routed_dialogue_frame_input.get("pressed", {})).get("a", false)) or str(routed_dialogue_input.get("route", "")) != "ui_shell" or not bool(routed_dialogue_input.get("consumed", false)):
		_fail("ui shell fixed-step packet was not consumed")
		return
	ui_shell.call("clear_menu_stack")
	ui_shell.call("close_dialogue")
	var pushed_panel: Dictionary = Dictionary(ui_shell.call("push_menu_panel", {
		"id": "parity_menu",
		"title": "Parity Menu",
		"kind": "menu",
		"entries": [{"id": "close", "label": "Close"}],
		"cursor": 0,
	}))
	if str(pushed_panel.get("id", "")) != "parity_menu" or not bool(ui_shell.call("has_menu_stack")):
		_fail("menu did not open through public ui shell method")
		return
	var top_panel: Dictionary = Dictionary(ui_shell.call("get_top_panel"))
	if str(top_panel.get("id", "")) != "parity_menu":
		_fail("menu top panel mismatch after open")
		return
	var popped_panel: Dictionary = Dictionary(ui_shell.call("pop_menu_panel"))
	if str(popped_panel.get("id", "")) != "parity_menu" or bool(ui_shell.call("has_menu_stack")):
		_fail("menu did not close through public ui shell method")
		return
	ui_shell.call("clear_menu_stack")

	runtime.call("route_to_overworld", "parity_journey")
	await process_frame
	if not _assert_route(runtime, "overworld", "overworld") or not _assert_visibility(runtime, "overworld"):
		return
	if not Dictionary(runtime.call("get_pending_scene_handoff")).is_empty():
		_fail("overworld route left a pending scene handoff behind")
		return
	var overworld: Node = runtime.get_node_or_null("Overworld")
	if not _require_methods(overworld, "overworld", [
		"load_default_map",
		"set_player_position",
		"request_move",
		"tick",
		"get_state",
		"get_player_tile",
		"get_player_facing",
		"get_last_move_result",
		"get_current_spawn_point",
		"get_map_summary",
		"get_runtime_queue_state",
		"queue_script",
		"queue_event",
		"queue_map_callback",
		"queue_object_movement",
	]):
		return
	if not bool(overworld.call("load_default_map")):
		_fail("overworld failed to load default map")
		return
	var overworld_map_summary: Dictionary = Dictionary(overworld.call("get_map_summary"))
	if str(overworld_map_summary.get("map_key", "")).is_empty() or int(overworld_map_summary.get("width", 0)) <= 0 or int(overworld_map_summary.get("height", 0)) <= 0:
		_fail("overworld default map summary missing concrete metadata")
		return
	if str(overworld_map_summary.get("map_name", "")).is_empty() or str(overworld_map_summary.get("map_constant", "")).is_empty() or int(overworld_map_summary.get("group_id", -1)) < 0 or int(overworld_map_summary.get("map_id", -1)) < 0:
		_fail("overworld default map summary missing normalized identity fields")
		return
	if Dictionary(overworld.call("get_current_spawn_point")).is_empty():
		_fail("overworld default spawn did not persist")
		return
	overworld.call("queue_script", {"action": "special", "function": "parity_script"})
	overworld.call("queue_event", {"action": "interaction", "button": "confirm"})
	overworld.call("queue_map_callback", {"action": "check_scene", "map_key": str(overworld_map_summary.get("map_key", ""))})
	overworld.call("queue_object_movement", "PLAYER", ["step_right"], {"source": "parity"})
	var queued_runtime_state: Dictionary = Dictionary(overworld.call("get_runtime_queue_state"))
	if Array(queued_runtime_state.get("queued_scripts", [])).is_empty() or Array(queued_runtime_state.get("queued_events", [])).is_empty() or Array(queued_runtime_state.get("map_callbacks", [])).is_empty() or Array(queued_runtime_state.get("object_movement_queue", [])).is_empty():
		_fail("overworld runtime queue state did not preserve queued public entries")
		return
	overworld.call("set_player_position", 0, 0)
	overworld.call("request_move", "right")
	overworld.call("tick")
	var move_result: Dictionary = Dictionary(overworld.call("get_last_move_result"))
	if str(move_result.get("state", "")) != "moved" or not bool(move_result.get("moved", false)):
		_fail("overworld deterministic movement did not move right")
		return
	var player_tile: Vector2i = overworld.call("get_player_tile")
	if player_tile.x != 1 or player_tile.y != 0 or str(overworld.call("get_player_facing")) != "right":
		_fail("overworld movement did not update tile and facing")
		return
	var overworld_snapshot: Dictionary = Dictionary(overworld.call("get_state"))
	if str(Dictionary(overworld_snapshot.get("last_move_result", {})).get("direction", "")) != "right":
		_fail("overworld movement did not persist in state")
		return

	runtime.call("route_to_battle", "parity_journey")
	await process_frame
	if not _assert_route(runtime, "battle", "battle") or not _assert_visibility(runtime, "battle"):
		return
	if not Dictionary(runtime.call("get_pending_scene_handoff")).is_empty():
		_fail("battle route left a pending scene handoff behind")
		return
	var battle: Node = runtime.get_node_or_null("Battle")
	if not _require_methods(battle, "battle", [
		"begin_battle",
		"get_state",
		"get_phase_history",
		"get_log_lines",
		"get_dialogue_wait_gate_active",
		"queue_command",
		"begin_resolution",
		"complete_resolution",
		"complete_battle",
		"get_resolution_events",
		"drain_resolution_events",
		"get_last_turn_resolution",
		"get_battle_result_state",
	]):
		return
	battle.call("begin_battle", {
		"battle_id": "parity-journey-battle",
		"battle_kind": "wild",
		"battle_label": "Parity Journey Battle",
	})
	var battle_snapshot: Dictionary = Dictionary(battle.call("get_state"))
	var battle_state: Dictionary = Dictionary(battle_snapshot.get("battle_state", {}))
	if str(battle_state.get("battle_id", "")) != "parity-journey-battle":
		_fail("battle begin did not persist battle id")
		return
	if str(battle_state.get("turn_phase", "")) != "turn_prompt" or not bool(battle_state.get("prompt_gate_active", false)):
		_fail("battle begin did not enter prompt phase")
		return
	if not bool(battle.call("get_dialogue_wait_gate_active")):
		_fail("battle begin did not expose dialogue wait gate")
		return
	if Array(battle.call("get_phase_history")).is_empty() or Array(battle.call("get_log_lines")).is_empty():
		_fail("battle begin did not expose phase/log public state")
		return
	battle.call("queue_command", {
		"kind": "attack",
		"label": "Parity Strike",
		"move": {"id": "PARITY_STRIKE", "name": "Parity Strike"},
	})
	if not bool(battle.call("begin_resolution")):
		_fail("battle resolution did not begin from queued command")
		return
	var turn_resolution: Dictionary = Dictionary(battle.call("get_last_turn_resolution"))
	if str(Dictionary(turn_resolution.get("command", {})).get("label", "")) != "Parity Strike" or str(Dictionary(turn_resolution.get("move", {})).get("id", "")) != "PARITY_STRIKE":
		_fail("battle resolution did not expose command and move payload")
		return
	if Array(battle.call("get_resolution_events")).size() < 3:
		_fail("battle resolution did not queue scaffold events")
		return
	battle.call("complete_resolution", "parity resolution complete")
	var completed_resolution_events: Array = Array(battle.call("get_resolution_events"))
	if completed_resolution_events.is_empty() or str(Dictionary(completed_resolution_events.back()).get("type", "")) != "turn_resolution_complete":
		_fail("battle resolution completion event missing")
		return
	var drained_events: Array = Array(battle.call("drain_resolution_events"))
	if drained_events.is_empty() or not Array(battle.call("get_resolution_events")).is_empty():
		_fail("battle resolution events did not drain")
		return
	battle.call("complete_battle", "win", {"reason": "parity", "winner": "player"})
	var battle_result_state: Dictionary = Dictionary(battle.call("get_battle_result_state"))
	if str(battle_result_state.get("result", "")) != "win" or str(battle_result_state.get("reason", "")) != "parity" or not bool(battle_result_state.get("finished", false)):
		_fail("battle result state did not preserve completion detail")
		return

	var before_save: Dictionary = Dictionary(runtime.call("to_dictionary"))
	if not bool(runtime.call("save_current_state", SAVE_SLOT)):
		_fail("runtime failed to save parity journey state")
		return
	var mutated_state := before_save.duplicate(true)
	mutated_state["active_scene"] = "title"
	mutated_state["scene_route"] = "title"
	mutated_state["frame_counter"] = int(before_save.get("frame_counter", 0)) + 99
	if not bool(runtime.call("from_dictionary", mutated_state)):
		_fail("runtime failed to accept temporary mutation before load")
		return
	if str(runtime.call("get_scene_route")) != "title":
		_fail("runtime temporary mutation did not change route before load")
		return
	if not bool(runtime.call("load_saved_state", SAVE_SLOT)):
		_fail("runtime failed to load parity journey state")
		return
	await process_frame
	var loaded_state: Dictionary = Dictionary(runtime.call("to_dictionary"))
	if str(runtime.call("get_scene_route")) != "battle" or str(loaded_state.get("scene_route", "")) != "battle":
		_fail("loaded state did not restore battle route")
		return
	var loaded_battle_state: Dictionary = Dictionary(loaded_state.get("battle_state", {}))
	if str(loaded_battle_state.get("battle_id", "")) != "parity-journey-battle":
		_fail("loaded state did not restore battle id")
		return
	if str(Dictionary(loaded_battle_state.get("battle_result_state", {})).get("reason", "")) != "parity":
		_fail("loaded state did not restore battle result detail")
		return
	var loaded_save_metadata: Dictionary = Dictionary(loaded_state.get("save_metadata", {}))
	if str(loaded_save_metadata.get("slot", "")) != "%s.sav" % SAVE_SLOT or str(loaded_save_metadata.get("kind", "")) != "custom" or str(loaded_save_metadata.get("saved_at", "")).is_empty():
		_fail("loaded state did not restore save metadata identity")
		return
	var loaded_overworld_state: Dictionary = Dictionary(loaded_state.get("overworld_state", {}))
	if str(Dictionary(loaded_overworld_state.get("last_move_result", {})).get("direction", "")) != "right":
		_fail("loaded state did not restore overworld movement")
		return
	var loaded_queue_state: Dictionary = Dictionary(Dictionary(loaded_overworld_state.get("special_state", {})).get("_runtime_queue_state", {}))
	if loaded_queue_state.is_empty() or Array(loaded_queue_state.get("queued_scripts", [])).is_empty():
		_fail("loaded state did not restore overworld runtime queue state")
		return
	if str(loaded_state.get("last_save_slot", "")) != SAVE_SLOT:
		_fail("loaded state did not restore save slot")
		return
	if not bool(runtime.call("delete_saved_state", SAVE_SLOT)):
		_fail("runtime failed to clean up parity journey save")
		return

	runtime.queue_free()
	await process_frame
	quit(0)
