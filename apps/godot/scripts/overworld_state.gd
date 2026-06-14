extends RefCounted
class_name OverworldState

const ASSET_INDEX_SCRIPT := preload("res://scripts/asset_index.gd")
const MOVE_NONE := "none"
const MOVE_UP := "up"
const MOVE_DOWN := "down"
const MOVE_LEFT := "left"
const MOVE_RIGHT := "right"
const TEXT_BOX_SCRIPT := preload("res://scripts/text_box.gd")
const RUNTIME_QUEUE_STATE_KEY := "_runtime_queue_state"
const QUEUED_SCRIPT_KEYS := ["queued_scripts", "script_queue", "queued_script_queue"]
const QUEUED_EVENT_KEYS := ["queued_events", "event_queue", "queued_event_queue"]
const MAP_CALLBACK_KEYS := ["map_callbacks", "queued_map_callbacks", "map_callback_queue"]
const OBJECT_MOVEMENT_QUEUE_KEYS := ["object_movement_queue", "queued_object_movements", "movement_queue", "object_movement_queues"]

var asset_index = ASSET_INDEX_SCRIPT.new()
var asset_summary: Dictionary = {}
var map_manifest: Dictionary = {}
var map_blocks: Dictionary = {}
var runtime_summary: Dictionary = {}
var available_map_keys: Array[String] = []
var selected_map_key: String = ""
var selected_map_index: int = -1
var map_scenes: Dictionary = {}
var map_scene_indices: Dictionary = {}
var scene_name: String = ""

var map_id: String = ""
var map_title: String = ""
var current_map_key: String = ""
var current_map_name: String = ""
var current_map_constant: String = ""
var current_map_group_name: String = ""
var current_map_environment: String = ""
var current_map_block_key: String = ""
var current_width: int = 0
var current_height: int = 0
var current_group_id: int = -1
var current_map_id: int = -1
var current_phone_service: int = 0
var current_tileset_name: String = ""
var current_location: String = ""
var map_dimensions: Vector2i = Vector2i(0, 0)
var player_tile: Vector2i = Vector2i(0, 0)
var player_facing: String = MOVE_DOWN
var movement_locked: bool = false
var collision_detected: bool = false
var collision_reason: String = ""
var warp_requested: bool = false
var warp_target: String = ""
var movement_state: String = "idle"
var last_move_request: Dictionary = {}
var last_move_result: Dictionary = {}
var last_collision_result: Dictionary = {}
var last_warp_result: Dictionary = {}
var last_runtime_note: String = ""
var movement_hook: Callable = Callable()
var collision_hook: Callable = Callable()
var warp_hook: Callable = Callable()
var last_move_direction: String = MOVE_NONE
var fixed_step_count: int = 0
var map_summary: Dictionary = {}
var spawn_summary: Dictionary = {}
var current_spawn_point: Dictionary = {}
var current_connections: Array = []
var current_warps: Array = []
var current_bg_events: Array = []
var current_object_events: Array = []
var debug_lines: Array[String] = []
var pending_move: String = MOVE_NONE
var reload_map_after_battle: bool = false
var music_request: Dictionary = {}
var follow_state: Dictionary = {}
var object_states: Dictionary = {}
var special_state: Dictionary = {}
var player_object: Dictionary = {}
var movement_data: Dictionary = {}
var dialogue_state: Dictionary = {}
var event_flags: Dictionary = {}
var engine_flags: Dictionary = {}
var current_map_payload: Dictionary = {}
var warp_permission_cache: Dictionary = {}
var active_warp_tile: Array = []
var warp_cooldown: int = 0
var tile_animation_state: Dictionary = {}
var wild_encounter_state: Dictionary = {}

func reset() -> void:
	asset_summary = {}
	map_manifest = {}
	map_blocks = {}
	runtime_summary = {}
	available_map_keys = []
	selected_map_key = ""
	selected_map_index = -1
	map_scenes = {}
	map_scene_indices = {}
	scene_name = ""
	map_id = ""
	map_title = ""
	current_map_key = ""
	current_map_name = ""
	current_map_constant = ""
	current_map_group_name = ""
	current_map_environment = ""
	current_map_block_key = ""
	current_width = 0
	current_height = 0
	current_group_id = -1
	current_map_id = -1
	current_phone_service = 0
	current_tileset_name = ""
	current_location = ""
	map_dimensions = Vector2i(0, 0)
	player_tile = Vector2i(0, 0)
	player_facing = MOVE_DOWN
	movement_locked = false
	collision_detected = false
	collision_reason = ""
	warp_requested = false
	warp_target = ""
	movement_state = "idle"
	last_move_request = {}
	last_move_result = {}
	last_collision_result = {}
	last_warp_result = {}
	last_runtime_note = ""
	last_move_direction = MOVE_NONE
	fixed_step_count = 0
	map_summary = {}
	spawn_summary = {}
	current_spawn_point = {}
	current_connections = []
	current_warps = []
	current_bg_events = []
	current_object_events = []
	debug_lines = ["overworld ready"]
	pending_move = MOVE_NONE
	reload_map_after_battle = false
	music_request = {}
	follow_state = {}
	object_states = {}
	special_state = {}
	player_object = {}
	movement_data = {}
	dialogue_state = {}
	event_flags = {}
	engine_flags = {}
	current_map_payload = {}
	warp_permission_cache = {}
	active_warp_tile = []
	warp_cooldown = 0
	tile_animation_state = {}
	wild_encounter_state = {}
	movement_hook = Callable()
	collision_hook = Callable()
	warp_hook = Callable()

func set_asset_index(index) -> void:
	asset_index = index
	refresh_assets()

func refresh_assets() -> void:
	if asset_index == null:
		return
	asset_summary = _dictionary_value(asset_index.load_summary())
	map_manifest = _dictionary_value(asset_index.load_map_manifest())
	map_blocks = _dictionary_value(asset_index.load_map_blocks())
	available_map_keys = _sorted_map_keys()
	_sync_selected_map_key()
	runtime_summary = _build_runtime_summary()
	last_runtime_note = "assets refreshed"
	debug_lines.append(last_runtime_note)
	while debug_lines.size() > 8:
		debug_lines.pop_front()

func load_assets() -> void:
	refresh_assets()

func to_dictionary() -> Dictionary:
	return get_state()

func from_dictionary(data: Variant) -> bool:
	if typeof(data) != TYPE_DICTIONARY:
		return false
	from_state(Dictionary(data))
	return true

func set_map_key(map_key: String) -> bool:
	return load_map(map_key)

func load_map(map_key: String = "") -> bool:
	if map_manifest.is_empty():
		refresh_assets()
	var chosen_key := _resolve_map_key(map_key)
	if chosen_key.is_empty() or not map_manifest.has(chosen_key):
		last_runtime_note = "map load failed: %s" % map_key
		return false
	var summary: Dictionary = _dictionary_value(map_manifest.get(chosen_key, {}))
	var spawn: Dictionary = {}
	set_map(summary, spawn)
	current_map_key = chosen_key
	current_map_name = map_title
	current_map_constant = str(summary.get("map_constant", summary.get("constant", map_id)))
	current_map_block_key = _resolve_blocks_key()
	_sync_scene_for_map(chosen_key)
	selected_map_key = chosen_key
	selected_map_index = _index_for_map_key(chosen_key)
	movement_state = "idle"
	last_move_request = {}
	last_move_result = {}
	last_collision_result = {}
	last_warp_result = {}
	last_runtime_note = "map loaded: %s" % chosen_key
	warp_requested = false
	warp_target = ""
	active_warp_tile = []
	warp_cooldown = 0
	collision_detected = false
	collision_reason = ""
	pending_move = MOVE_NONE
	runtime_summary = _build_runtime_summary()
	return true

func load_default_map() -> bool:
	return load_map("")

func set_map_data(data: Dictionary) -> void:
	if data.is_empty():
		return
	map_summary = data.duplicate(true)
	current_map_payload = data.duplicate(true)
	current_map_data_from_summary()
	_update_wild_encounter_metadata()
	runtime_summary = _build_runtime_summary()

func set_player_position(x: int, y: int) -> void:
	player_tile = Vector2i(max(0, x), max(0, y))
	current_spawn_point = _normalize_spawn_point_state(current_spawn_point, player_tile)
	_refresh_player_object()
	last_runtime_note = "player position set"
	runtime_summary = _build_runtime_summary()

func set_player_facing(direction: String) -> void:
	var normalized: String = _normalize_direction(direction)
	if normalized.is_empty():
		return
	player_facing = normalized
	_refresh_player_object()
	runtime_summary = _build_runtime_summary()

func tick_frame() -> void:
	tick()

func set_collision_hook(hook: Callable) -> void:
	collision_hook = hook

func set_warp_hook(hook: Callable) -> void:
	warp_hook = hook

func get_available_map_keys() -> Array[String]:
	return available_map_keys.duplicate()

func get_selected_map_key() -> String:
	return selected_map_key if not selected_map_key.is_empty() else current_map_key

func get_selected_map_index() -> int:
	return selected_map_index

func load_map_by_index(index: int) -> bool:
	var keys: Array[String] = get_available_map_keys()
	if index < 0 or index >= keys.size():
		return false
	return load_map(keys[index])

func cycle_map(offset: int = 1) -> bool:
	var keys: Array[String] = get_available_map_keys()
	if keys.is_empty():
		return false
	var start_index := selected_map_index
	if start_index < 0 or start_index >= keys.size():
		start_index = _index_for_map_key(current_map_key)
	if start_index < 0:
		start_index = 0
	var next_index := posmod(start_index + offset, keys.size())
	return load_map(keys[next_index])

func reload_current_map() -> bool:
	return load_map(current_map_key)

func request_interaction(action: String = "confirm") -> void:
	debug_lines.append("interaction -> %s" % str(action))
	last_runtime_note = "interaction -> %s" % str(action)
	while debug_lines.size() > 8:
		debug_lines.pop_front()
	runtime_summary = _build_runtime_summary()

func player_movement_locked() -> bool:
	return movement_locked or movement_state in ["moving", "blocked", "warping"]

func playerMovementLocked() -> bool:
	return player_movement_locked()

func get_object_by_id(object_id) -> Variant:
	var normalized_id := _normalize_object_id(object_id)
	if normalized_id.is_empty():
		return null
	if normalized_id == "PLAYER":
		return player_object.duplicate(true)
	if normalized_id == "LAST_TALKED":
		var last_talked_index := int(special_state.get("last_talked_object_index", 0))
		if last_talked_index > 0:
			return get_object_by_id(last_talked_index)
		return null
	if _is_numeric_identifier(normalized_id):
		var numeric_index := int(normalized_id)
		if numeric_index <= 0:
			return null
		var indexed_record := _get_object_record_by_index(numeric_index)
		if not indexed_record.is_empty():
			return indexed_record
	for key in object_states.keys():
		var record: Dictionary = Dictionary(object_states.get(key, {}))
		if _object_record_matches(record, normalized_id):
			return record.duplicate(true)
	var event_record := _get_object_record_by_identifier(normalized_id)
	if not event_record.is_empty():
		return event_record
	return null

func getObjectById(object_id) -> Variant:
	return get_object_by_id(object_id)

func get_object_motion_states() -> Array:
	return _build_object_motion_states().duplicate(true)

func getObjectMotionStates() -> Array:
	return get_object_motion_states()

func get_trainer_sightline_payloads() -> Array:
	return _build_trainer_sightline_payloads().duplicate(true)

func getTrainerSightlinePayloads() -> Array:
	return get_trainer_sightline_payloads()

func get_field_move_state() -> Dictionary:
	return Dictionary(special_state.get("last_field_move", {})).duplicate(true)

func getFieldMoveState() -> Dictionary:
	return get_field_move_state()

func get_map_callback_queue_state() -> Array:
	return Array(_runtime_queue_state().get("map_callbacks", [])).duplicate(true)

func getMapCallbackQueueState() -> Array:
	return get_map_callback_queue_state()

func get_warp_transition_payloads() -> Array:
	return _build_warp_transition_payloads().duplicate(true)

func getWarpTransitionPayloads() -> Array:
	return get_warp_transition_payloads()

func get_connection_transition_payloads() -> Array:
	return _build_connection_transition_payloads().duplicate(true)

func getConnectionTransitionPayloads() -> Array:
	return get_connection_transition_payloads()

func get_event_activation_records() -> Array:
	return _build_event_activation_records().duplicate(true)

func getEventActivationRecords() -> Array:
	return get_event_activation_records()

func get_object_event_gating_records() -> Array:
	return _build_object_event_gating_records().duplicate(true)

func getObjectEventGatingRecords() -> Array:
	return get_object_event_gating_records()

func get_tile_animation_state() -> Dictionary:
	return Dictionary(tile_animation_state).duplicate(true)

func getTileAnimationState() -> Dictionary:
	return get_tile_animation_state()

func get_wild_encounter_state() -> Dictionary:
	return Dictionary(wild_encounter_state).duplicate(true)

func getWildEncounterState() -> Dictionary:
	return get_wild_encounter_state()

func get_wild_encounter_eligibility_payloads() -> Array:
	return _build_wild_encounter_eligibility_payloads().duplicate(true)

func getWildEncounterEligibilityPayloads() -> Array:
	return get_wild_encounter_eligibility_payloads()

func get_last_wild_encounter_roll() -> Dictionary:
	return Dictionary(wild_encounter_state.get("last_roll", {})).duplicate(true)

func getLastWildEncounterRoll() -> Dictionary:
	return get_last_wild_encounter_roll()

func resolve_object_index(identifier: String) -> int:
	var normalized := _normalize_object_id(identifier)
	if normalized.is_empty():
		return 0
	if normalized == "PLAYER":
		return 0
	if normalized == "LAST_TALKED":
		return int(special_state.get("last_talked_object_index", 0))
	if _is_numeric_identifier(normalized):
		var parsed := int(normalized)
		return parsed if parsed > 0 else 0
	for key in object_states.keys():
		var record: Dictionary = Dictionary(object_states.get(key, {}))
		if _object_record_matches(record, normalized):
			var record_index := int(record.get("object_index", record.get("index", 0)))
			if record_index > 0:
				return record_index
	return _resolve_object_index_from_events(normalized)

func resolveObjectIndex(identifier: String) -> int:
	return resolve_object_index(identifier)

func get_movement_data(movement_data_label: String, parent_script: String = "") -> Array[String]:
	return _lookup_movement_data(movement_data_label, parent_script)

func getMovementData(movement_data_label: String, parent_script: String = "") -> Array[String]:
	return get_movement_data(movement_data_label, parent_script)

func queue_movement_task(obj, movement_commands, options: Dictionary = {}) -> void:
	var object_record := _coerce_object_record(obj)
	if object_record.is_empty():
		return
	var command_list := _normalize_movement_commands(movement_commands)
	last_runtime_note = "movement task queued: %s" % str(object_record.get("object_id", "unknown"))
	debug_lines.append(last_runtime_note)
	while debug_lines.size() > 8:
		debug_lines.pop_front()
	if not command_list.is_empty():
		_apply_movement_commands(object_record, command_list)
	_store_object_record(object_record)
	movement_state = "idle" if not movement_locked else "locked"
	runtime_summary = _build_runtime_summary()
	var callback := Callable()
	if options.has("onComplete") and typeof(options.get("onComplete")) == TYPE_CALLABLE:
		callback = options.get("onComplete")
	elif options.has("on_complete") and typeof(options.get("on_complete")) == TYPE_CALLABLE:
		callback = options.get("on_complete")
	if callback.is_valid():
		callback.call()

func queueMovementTask(obj, movement_commands, options: Dictionary = {}) -> void:
	queue_movement_task(obj, movement_commands, options)

func queue_movement(obj, movement_commands, options: Dictionary = {}) -> void:
	queue_movement_task(obj, movement_commands, options)

func queueMovement(obj, movement_commands, options: Dictionary = {}) -> void:
	queue_movement_task(obj, movement_commands, options)

func queue_follow_task(follower, leader, options: Dictionary = {}) -> void:
	start_following(follower, leader, options)
	var callback := Callable()
	if options.has("onComplete") and typeof(options.get("onComplete")) == TYPE_CALLABLE:
		callback = options.get("onComplete")
	elif options.has("on_complete") and typeof(options.get("on_complete")) == TYPE_CALLABLE:
		callback = options.get("on_complete")
	if callback.is_valid():
		callback.call()

func queueFollowTask(follower, leader, options: Dictionary = {}) -> void:
	queue_follow_task(follower, leader, options)

func queue_follow(follower, leader, options: Dictionary = {}) -> void:
	queue_follow_task(follower, leader, options)

func queueFollow(follower, leader, options: Dictionary = {}) -> void:
	queue_follow_task(follower, leader, options)

func queue_script(script_entry) -> void:
	_enqueue_runtime_queue_entry("queued_scripts", script_entry, "")
	last_runtime_note = "script queued"
	debug_lines.append(last_runtime_note)
	while debug_lines.size() > 8:
		debug_lines.pop_front()
	runtime_summary = _build_runtime_summary()

func queueScript(script_entry) -> void:
	queue_script(script_entry)

func queue_event(event_entry) -> void:
	_enqueue_runtime_queue_entry("queued_events", event_entry, "")
	last_runtime_note = "event queued"
	debug_lines.append(last_runtime_note)
	while debug_lines.size() > 8:
		debug_lines.pop_front()
	runtime_summary = _build_runtime_summary()

func queueEvent(event_entry) -> void:
	queue_event(event_entry)

func queue_map_callback(callback_entry) -> void:
	_enqueue_runtime_queue_entry("map_callbacks", callback_entry, "")
	last_runtime_note = "map callback queued"
	debug_lines.append(last_runtime_note)
	while debug_lines.size() > 8:
		debug_lines.pop_front()
	runtime_summary = _build_runtime_summary()

func queueMapCallback(callback_entry) -> void:
	queue_map_callback(callback_entry)

func queue_object_movement(object_id, movement_commands, options: Dictionary = {}) -> void:
	var entry := {
		"object": _normalize_object_id(object_id),
		"commands": _normalize_movement_commands(movement_commands),
		"options": options.duplicate(true),
	}
	_enqueue_runtime_queue_entry("object_movement_queue", entry, str(entry.get("object", "")))
	last_runtime_note = "object movement queued: %s" % str(entry.get("object", "unknown"))
	debug_lines.append(last_runtime_note)
	while debug_lines.size() > 8:
		debug_lines.pop_front()
	runtime_summary = _build_runtime_summary()

func queueObjectMovement(object_id, movement_commands, options: Dictionary = {}) -> void:
	queue_object_movement(object_id, movement_commands, options)

func get_runtime_queue_state() -> Dictionary:
	return _runtime_queue_state().duplicate(true)

func getRuntimeQueueState() -> Dictionary:
	return get_runtime_queue_state()

func _runtime_queue_state() -> Dictionary:
	var queues: Dictionary = {}
	var stored: Variant = special_state.get(RUNTIME_QUEUE_STATE_KEY, {})
	if typeof(stored) == TYPE_DICTIONARY:
		queues = Dictionary(stored).duplicate(true)
	_ensure_runtime_queue_arrays(queues)
	special_state[RUNTIME_QUEUE_STATE_KEY] = queues
	return queues

func _store_runtime_queue_state(queues: Dictionary) -> void:
	_ensure_runtime_queue_arrays(queues)
	special_state[RUNTIME_QUEUE_STATE_KEY] = queues.duplicate(true)

func _ensure_runtime_queue_arrays(queues: Dictionary) -> void:
	for key in ["queued_scripts", "queued_events", "map_callbacks", "object_movement_queue", "completed"]:
		if typeof(queues.get(key, [])) != TYPE_ARRAY:
			queues[key] = []

func _enqueue_runtime_queue_entry(queue_name: String, entry: Variant, object_id: String) -> void:
	var queues: Dictionary = _runtime_queue_state()
	_append_runtime_queue_values(queues, queue_name, entry, object_id)
	_store_runtime_queue_state(queues)

func _append_runtime_queue_values(queues: Dictionary, queue_name: String, value: Variant, object_id: String) -> void:
	_ensure_runtime_queue_arrays(queues)
	var queue: Array = Array(queues.get(queue_name, []))
	if value == null:
		queues[queue_name] = queue
		return
	if typeof(value) == TYPE_ARRAY:
		for entry in Array(value):
			_append_runtime_queue_values(queues, queue_name, entry, object_id)
		return
	elif typeof(value) == TYPE_DICTIONARY and queue_name == "object_movement_queue" and not object_id.is_empty() and not Dictionary(value).has("object"):
		queue.append({
			"object": object_id,
			"commands": _normalize_movement_commands(Dictionary(value).get("commands", Dictionary(value).get("movement_commands", value))),
			"options": _dictionary_value(Dictionary(value).get("options", {})),
		})
	elif typeof(value) == TYPE_DICTIONARY and queue_name == "object_movement_queue" and object_id.is_empty():
		var value_dict: Dictionary = Dictionary(value)
		if value_dict.has("object") or value_dict.has("object_id") or value_dict.has("commands") or value_dict.has("movement_commands"):
			queue.append(value_dict.duplicate(true))
		else:
			for key in value_dict.keys():
				_append_runtime_queue_values(queues, queue_name, value_dict.get(key), str(key))
			return
	elif queue_name == "object_movement_queue" and not object_id.is_empty():
		queue.append({
			"object": object_id,
			"commands": _normalize_movement_commands(value),
			"options": {},
		})
	else:
		queue.append(value)
	queues[queue_name] = queue

func get_event_flag_for_object_index(index: int) -> String:
	if index <= 0 or index > current_object_events.size():
		return ""
	var event_variant: Variant = current_object_events[index - 1]
	if typeof(event_variant) != TYPE_DICTIONARY:
		return ""
	var event_flag := str(Dictionary(event_variant).get("event_flag", "")).strip_edges()
	if event_flag.is_empty() or event_flag == "-1":
		return ""
	return event_flag

func getEventFlagForObjectIndex(index: int) -> String:
	return get_event_flag_for_object_index(index)

func get_event_flag(flag_name: String) -> bool:
	var normalized := flag_name.strip_edges()
	if normalized.is_empty():
		return false
	if _is_engine_flag(normalized):
		return bool(engine_flags.get(normalized, false))
	return bool(event_flags.get(normalized, false))

func getEventFlag(flag_name: String) -> bool:
	return get_event_flag(flag_name)

func set_event_flag(flag_name: String, value: bool) -> void:
	var normalized := flag_name.strip_edges()
	if normalized.is_empty():
		return
	if _is_engine_flag(normalized):
		engine_flags[normalized] = value
	else:
		event_flags[normalized] = value
		refresh_event_flag(normalized, {"value": value})
	last_runtime_note = "%s flag: %s" % ["set" if value else "cleared", normalized]
	debug_lines.append(last_runtime_note)
	while debug_lines.size() > 8:
		debug_lines.pop_front()
	_update_state_summary_metadata()
	runtime_summary = _build_runtime_summary()

func setEventFlag(flag_name: String, value: bool) -> void:
	set_event_flag(flag_name, value)

func clear_event_flag(flag_name: String) -> void:
	set_event_flag(flag_name, false)

func clearEventFlag(flag_name: String) -> void:
	clear_event_flag(flag_name)

func set_engine_flag(flag_name: String, value: bool) -> void:
	set_event_flag(flag_name, value)

func setEngineFlag(flag_name: String, value: bool) -> void:
	set_engine_flag(flag_name, value)

func set_wild_encounter_state(state: Dictionary) -> void:
	wild_encounter_state = _normalize_wild_encounter_state(state)
	_update_wild_encounter_metadata()
	last_runtime_note = "wild encounter state updated"
	debug_lines.append(last_runtime_note)
	while debug_lines.size() > 8:
		debug_lines.pop_front()
	runtime_summary = _build_runtime_summary()

func setWildEncounterState(state: Dictionary) -> void:
	set_wild_encounter_state(state)

func advance_wild_encounter_step(surface: String = "") -> void:
	var state: Dictionary = _normalize_wild_encounter_state(wild_encounter_state)
	state["step_counter"] = int(state.get("step_counter", 0)) + 1
	var normalized_surface: String = _normalize_wild_encounter_surface(surface)
	if normalized_surface.is_empty():
		normalized_surface = _normalize_wild_encounter_surface(str(state.get("surface", "")))
	if normalized_surface.is_empty():
		normalized_surface = _default_wild_encounter_surface()
	if not normalized_surface.is_empty():
		state["surface"] = normalized_surface
	var repel_steps_remaining: int = max(0, int(state.get("repel_steps_remaining", 0)))
	if repel_steps_remaining > 0:
		state["repel_steps_remaining"] = repel_steps_remaining - 1
	var eligibility_payloads: Array = _build_wild_encounter_eligibility_payloads(state)
	var last_roll: Dictionary = Dictionary(state.get("last_roll", {}))
	if last_roll.is_empty():
		last_roll = {
			"step_counter": int(state.get("step_counter", 0)),
			"surface": str(state.get("surface", "")),
			"time_of_day": str(state.get("time_of_day", "")),
			"eligible": bool(eligibility_payloads.size() > 0 and bool(Dictionary(eligibility_payloads[0]).get("eligible", false))),
			"roll": int(state.get("step_counter", 0)) % 256,
			"repel_steps_remaining": int(state.get("repel_steps_remaining", 0)),
		}
	state["last_roll"] = last_roll.duplicate(true)
	wild_encounter_state = state
	_update_wild_encounter_metadata()
	last_runtime_note = "wild encounter step advanced"
	debug_lines.append(last_runtime_note)
	while debug_lines.size() > 8:
		debug_lines.pop_front()
	runtime_summary = _build_runtime_summary()

func advanceWildEncounterStep(surface: String = "") -> void:
	advance_wild_encounter_step(surface)

func record_wild_encounter_roll(record: Dictionary) -> void:
	var state: Dictionary = _normalize_wild_encounter_state(wild_encounter_state)
	state["last_roll"] = Dictionary(record).duplicate(true)
	if record.has("step_counter"):
		state["step_counter"] = int(record.get("step_counter", state.get("step_counter", 0)))
	if record.has("surface"):
		state["surface"] = _normalize_wild_encounter_surface(str(record.get("surface", state.get("surface", ""))))
	if record.has("time_of_day"):
		state["time_of_day"] = _normalize_wild_encounter_time_of_day(str(record.get("time_of_day", state.get("time_of_day", ""))))
	if record.has("repel_steps_remaining"):
		state["repel_steps_remaining"] = max(0, int(record.get("repel_steps_remaining", state.get("repel_steps_remaining", 0))))
	wild_encounter_state = state
	_update_wild_encounter_metadata()
	last_runtime_note = "wild encounter roll recorded"
	debug_lines.append(last_runtime_note)
	while debug_lines.size() > 8:
		debug_lines.pop_front()
	runtime_summary = _build_runtime_summary()

func recordWildEncounterRoll(record: Dictionary) -> void:
	record_wild_encounter_roll(record)

func refresh_event_flag(event_name: String, options: Dictionary = {}) -> void:
	var normalized := event_name.strip_edges()
	if normalized.is_empty():
		return
	var value := bool(options.get("value", get_event_flag(normalized)))
	_apply_event_flag_update(normalized, value)
	last_runtime_note = "event flag refresh: %s=%s" % [normalized, str(value).to_lower()]
	debug_lines.append(last_runtime_note)
	while debug_lines.size() > 8:
		debug_lines.pop_front()
	_update_state_summary_metadata()
	runtime_summary = _build_runtime_summary()

func refreshEventFlag(event_name: String, options: Dictionary = {}) -> void:
	refresh_event_flag(event_name, options)

func get_render_object_states() -> Array:
	return _build_render_object_states().duplicate(true)

func get_render_object_payloads() -> Array:
	return get_render_object_states()

func open_text(content: Variant) -> void:
	dialogue_state = _build_dialogue_state(content)
	movement_locked = true
	movement_state = "locked"
	last_runtime_note = "text opened"
	debug_lines.append(last_runtime_note)
	while debug_lines.size() > 8:
		debug_lines.pop_front()
	runtime_summary = _build_runtime_summary()

func openText(content: Variant) -> void:
	open_text(content)

func open_dialogue(content: Variant) -> void:
	open_text(content)

func openDialogue(content: Variant) -> void:
	open_text(content)

func show_text(content: Variant) -> void:
	open_text(content)

func showText(content: Variant) -> void:
	show_text(content)

func close_text() -> void:
	if dialogue_state.is_empty():
		return
	dialogue_state["active"] = false
	dialogue_state["visible"] = false
	dialogue_state["waiting_for_input"] = false
	dialogue_state["prompt_active"] = false
	dialogue_state["pending_waits"] = 0
	movement_locked = false
	movement_state = "idle" if pending_move == MOVE_NONE else "moving"
	last_runtime_note = "text closed"
	debug_lines.append(last_runtime_note)
	while debug_lines.size() > 8:
		debug_lines.pop_front()
	runtime_summary = _build_runtime_summary()

func closeText() -> void:
	close_text()

func close_dialogue() -> void:
	close_text()

func closeDialogue() -> void:
	close_text()

func wait_for_input() -> void:
	if dialogue_state.is_empty():
		dialogue_state = _build_dialogue_state("")
	movement_locked = true
	movement_state = "locked"
	dialogue_state["active"] = true
	dialogue_state["visible"] = true
	dialogue_state["waiting_for_input"] = true
	dialogue_state["pending_waits"] = int(dialogue_state.get("pending_waits", 0)) + 1
	last_runtime_note = "waiting for input"
	debug_lines.append(last_runtime_note)
	while debug_lines.size() > 8:
		debug_lines.pop_front()
	runtime_summary = _build_runtime_summary()

func waitForInput() -> void:
	wait_for_input()

func acknowledge_wait() -> void:
	if dialogue_state.is_empty():
		return
	var pending: int = max(0, int(dialogue_state.get("pending_waits", 0)) - 1)
	dialogue_state["pending_waits"] = pending
	dialogue_state["waiting_for_input"] = pending > 0 or bool(dialogue_state.get("prompt_active", false))
	if pending == 0 and not bool(dialogue_state.get("prompt_active", false)):
		movement_locked = false
		movement_state = "idle" if pending_move == MOVE_NONE else "moving"
	last_runtime_note = "wait acknowledged"
	debug_lines.append(last_runtime_note)
	while debug_lines.size() > 8:
		debug_lines.pop_front()
	runtime_summary = _build_runtime_summary()

func acknowledgeWait() -> void:
	acknowledge_wait()

func prompt_yes_no(prompt: Variant = null) -> void:
	if dialogue_state.is_empty():
		dialogue_state = _build_dialogue_state(prompt if prompt != null else "")
	movement_locked = true
	movement_state = "locked"
	dialogue_state["active"] = true
	dialogue_state["visible"] = true
	dialogue_state["waiting_for_input"] = true
	dialogue_state["prompt_active"] = true
	dialogue_state["prompt_kind"] = "yesno"
	dialogue_state["prompt_options"] = ["yes", "no"]
	dialogue_state["yes_no_result"] = null
	last_runtime_note = "yes/no prompt"
	debug_lines.append(last_runtime_note)
	while debug_lines.size() > 8:
		debug_lines.pop_front()
	runtime_summary = _build_runtime_summary()

func promptYesNo(prompt: Variant = null) -> void:
	prompt_yes_no(prompt)

func set_yes_no_result(value: bool) -> void:
	if dialogue_state.is_empty():
		dialogue_state = _build_dialogue_state("")
	dialogue_state["yes_no_result"] = value
	dialogue_state["prompt_active"] = false
	dialogue_state["waiting_for_input"] = false
	movement_locked = false
	movement_state = "idle" if pending_move == MOVE_NONE else "moving"
	last_runtime_note = "yes/no result: %s" % str(value).to_lower()
	debug_lines.append(last_runtime_note)
	while debug_lines.size() > 8:
		debug_lines.pop_front()
	runtime_summary = _build_runtime_summary()

func setYesNoResult(value: bool) -> void:
	set_yes_no_result(value)

func get_dialogue_state() -> Dictionary:
	return dialogue_state.duplicate(true)

func getDialogueState() -> Dictionary:
	return get_dialogue_state()

func run_map_callbacks(map_key: String = "", callback_type: String = "") -> Array:
	var resolved_map: String = map_key.strip_edges()
	if resolved_map.is_empty():
		resolved_map = current_map_key if not current_map_key.is_empty() else get_selected_map_key()
	var payloads: Array = _resolve_map_callbacks()
	var executed: Array = []
	if payloads.is_empty():
		last_runtime_note = "map callbacks: none"
		debug_lines.append(last_runtime_note)
		while debug_lines.size() > 8:
			debug_lines.pop_front()
		_update_state_summary_metadata()
		runtime_summary = _build_runtime_summary()
		return executed
	for entry in payloads:
		var callback_entry: Dictionary = _normalize_map_callback_entry(entry, resolved_map)
		if callback_entry.is_empty():
			continue
		var entry_type: String = str(callback_entry.get("callback_type", callback_entry.get("type", callback_entry.get("action", "")))).strip_edges()
		if not callback_type.is_empty() and entry_type != callback_type:
			continue
		executed.append(callback_entry.duplicate(true))
		var history: Array = Array(special_state.get("map_callbacks_executed", []))
		history.append(callback_entry.duplicate(true))
		special_state["map_callbacks_executed"] = history
		last_runtime_note = "map callback: %s -> %s" % [
			entry_type if not entry_type.is_empty() else "unknown",
			str(callback_entry.get("script_name", callback_entry.get("script", ""))),
		]
		debug_lines.append(last_runtime_note)
		while debug_lines.size() > 8:
			debug_lines.pop_front()
	_update_state_summary_metadata()
	runtime_summary = _build_runtime_summary()
	return executed

func runMapCallbacks(map_key: String = "", callback_type: String = "") -> Array:
	return run_map_callbacks(map_key, callback_type)

func _write_metatile(metatile_x: int, metatile_y: int, block_id: int) -> void:
	var change_key: String = "%d,%d" % [metatile_x, metatile_y]
	var changes: Dictionary = Dictionary(current_map_payload.get("changed_blocks", {}))
	changes[change_key] = int(block_id) & 0xff
	current_map_payload["changed_blocks"] = changes
	map_summary["changed_blocks"] = changes.duplicate(true)
	var block_bytes: Variant = current_map_payload.get("block_bytes", current_map_payload.get("bytes", []))
	if typeof(block_bytes) == TYPE_ARRAY:
		var bytes: Array = Array(block_bytes).duplicate(true)
		var width: int = int(current_map_payload.get("width", current_width))
		if width > 0:
			var index: int = metatile_y * width + metatile_x
			if index >= 0 and index < bytes.size():
				bytes[index] = int(block_id) & 0xff
				current_map_payload["block_bytes"] = bytes
				current_map_payload["bytes"] = bytes
	current_map_payload["last_changed_block"] = {
		"x": metatile_x,
		"y": metatile_y,
		"block_id": int(block_id) & 0xff,
	}
	map_summary["last_changed_block"] = Dictionary(current_map_payload["last_changed_block"]).duplicate(true)
	last_runtime_note = "metatile changed: %s" % change_key
	debug_lines.append(last_runtime_note)
	while debug_lines.size() > 8:
		debug_lines.pop_front()
	_update_state_summary_metadata()
	runtime_summary = _build_runtime_summary()

func refresh_warp_permissions() -> void:
	_refresh_warp_permissions()

func _refresh_warp_permissions() -> void:
	var cache: Dictionary = {}
	for warp_entry in current_warps:
		if typeof(warp_entry) != TYPE_DICTIONARY:
			continue
		var warp: Dictionary = Dictionary(warp_entry)
		var tile_x := int(warp.get("x", warp.get("tile_x", warp.get("map_x", 0))))
		var tile_y := int(warp.get("y", warp.get("tile_y", warp.get("map_y", 0))))
		var key := "%d,%d" % [tile_x, tile_y]
		var list: Array = Array(cache.get(key, []))
		list.append({
			"warp_id": int(warp.get("warp_id", warp.get("warpId", warp.get("index", 0)))),
			"target_map_constant": str(warp.get("target_map_constant", warp.get("targetMapConstant", current_map_constant))),
			"target_warp_id": int(warp.get("target_warp_id", warp.get("targetWarpId", 0))),
			"permission": null,
		})
		cache[key] = list
	if current_map_payload.has("warp_permissions"):
		var payload_permissions: Variant = current_map_payload.get("warp_permissions", {})
		if typeof(payload_permissions) == TYPE_DICTIONARY:
			for key in Dictionary(payload_permissions).keys():
				cache[str(key)] = Array(Dictionary(payload_permissions).get(key, []))
	warp_permission_cache = cache
	current_map_payload["warp_permissions_refreshed"] = true
	current_map_payload["warp_permission_cache"] = warp_permission_cache.duplicate(true)
	map_summary["warp_permissions_refreshed"] = true
	map_summary["warp_permission_cache"] = warp_permission_cache.duplicate(true)
	last_runtime_note = "warp permissions refreshed"
	debug_lines.append(last_runtime_note)
	while debug_lines.size() > 8:
		debug_lines.pop_front()
	_update_state_summary_metadata()
	runtime_summary = _build_runtime_summary()

func _build_runtime_summary() -> Dictionary:
	var counts: Dictionary = _build_state_summary_counts()
	return {
		"map_count": map_manifest.size(),
		"block_count": map_blocks.size(),
		"available_map_count": available_map_keys.size(),
		"selected_map_key": get_selected_map_key(),
		"selected_map_index": selected_map_index,
		"map_id": map_id,
		"map_title": map_title,
		"current_map_key": current_map_key,
		"current_map_name": current_map_name,
		"current_map_constant": current_map_constant,
		"scene_name": scene_name,
		"reload_map_after_battle": reload_map_after_battle,
		"follow_active": bool(follow_state.get("active", false)),
		"object_count": object_states.size(),
		"player_tile": {"x": player_tile.x, "y": player_tile.y},
		"player_facing": player_facing,
		"movement_state": movement_state,
		"movement_locked": movement_locked,
		"collision_detected": collision_detected,
		"collision_reason": collision_reason,
		"warp_requested": warp_requested,
		"warp_target": warp_target,
		"last_move_direction": last_move_direction,
		"last_move_result": Dictionary(last_move_result),
		"last_collision_result": Dictionary(last_collision_result),
		"last_warp_result": Dictionary(last_warp_result),
		"last_runtime_note": last_runtime_note,
		"object_state_count": int(counts.get("object_state_count", 0)),
		"hidden_object_count": int(counts.get("hidden_object_count", 0)),
		"removed_object_count": int(counts.get("removed_object_count", 0)),
		"defeated_object_count": int(counts.get("defeated_object_count", 0)),
		"event_flag_count": int(counts.get("event_flag_count", 0)),
		"engine_flag_count": int(counts.get("engine_flag_count", 0)),
		"map_callbacks_executed_count": int(counts.get("map_callbacks_executed_count", 0)),
		"changed_block_count": int(counts.get("changed_block_count", 0)),
		"render_object_count": int(counts.get("render_object_count", 0)),
		"object_motion_count": int(counts.get("object_motion_count", 0)),
		"trainer_sighting_count": int(counts.get("trainer_sighting_count", 0)),
		"warp_transition_count": int(counts.get("warp_transition_count", 0)),
		"connection_transition_count": int(counts.get("connection_transition_count", 0)),
		"event_activation_count": int(counts.get("event_activation_count", 0)),
		"tile_animation_frame": int(counts.get("tile_animation_frame", 0)),
		"object_event_gating_count": int(counts.get("object_event_gating_count", 0)),
		"wild_encounter_count": int(counts.get("wild_encounter_count", 0)),
		"wild_encounter_step_counter": int(wild_encounter_state.get("step_counter", 0)),
		"wild_encounter_repel_steps_remaining": int(wild_encounter_state.get("repel_steps_remaining", 0)),
		"wild_encounter_time_of_day": str(wild_encounter_state.get("time_of_day", "")),
	}

func _build_state_summary_counts() -> Dictionary:
	var hidden_object_count := 0
	var removed_object_count := 0
	var defeated_object_count := 0
	for key in object_states.keys():
		var record_variant: Variant = object_states.get(key, {})
		if typeof(record_variant) != TYPE_DICTIONARY:
			continue
		var record: Dictionary = Dictionary(record_variant)
		if not bool(record.get("visible", true)):
			hidden_object_count += 1
		if bool(record.get("removed", false)):
			removed_object_count += 1
		if bool(record.get("defeated", false)):
			defeated_object_count += 1
	var callback_history: Array = Array(special_state.get("map_callbacks_executed", []))
	var render_object_count := _build_render_object_states().size()
	var object_motion_count := _build_object_motion_states().size()
	var trainer_sighting_count := _build_trainer_sightline_payloads().size()
	var warp_transition_count := _build_warp_transition_payloads().size()
	var connection_transition_count := _build_connection_transition_payloads().size()
	var event_activation_count := _build_event_activation_records().size()
	var object_event_gating_count := _build_object_event_gating_records().size()
	var wild_encounter_count := _build_wild_encounter_eligibility_payloads().size()
	var tile_animation_frame := int(tile_animation_state.get("frame_index", 0))
	return {
		"object_state_count": object_states.size(),
		"hidden_object_count": hidden_object_count,
		"removed_object_count": removed_object_count,
		"defeated_object_count": defeated_object_count,
		"event_flag_count": event_flags.size(),
		"engine_flag_count": engine_flags.size(),
		"map_callbacks_executed_count": callback_history.size(),
		"changed_block_count": Dictionary(current_map_payload.get("changed_blocks", {})).size(),
		"render_object_count": render_object_count,
		"object_motion_count": object_motion_count,
		"trainer_sighting_count": trainer_sighting_count,
		"warp_transition_count": warp_transition_count,
		"connection_transition_count": connection_transition_count,
		"event_activation_count": event_activation_count,
		"object_event_gating_count": object_event_gating_count,
		"wild_encounter_count": wild_encounter_count,
		"tile_animation_frame": tile_animation_frame,
	}

func _update_state_summary_metadata() -> void:
	var counts: Dictionary = _build_state_summary_counts()
	map_summary["object_state_count"] = int(counts.get("object_state_count", 0))
	map_summary["hidden_object_count"] = int(counts.get("hidden_object_count", 0))
	map_summary["removed_object_count"] = int(counts.get("removed_object_count", 0))
	map_summary["defeated_object_count"] = int(counts.get("defeated_object_count", 0))
	map_summary["event_flag_count"] = int(counts.get("event_flag_count", 0))
	map_summary["engine_flag_count"] = int(counts.get("engine_flag_count", 0))
	map_summary["map_callbacks_executed_count"] = int(counts.get("map_callbacks_executed_count", 0))
	map_summary["changed_block_count"] = int(counts.get("changed_block_count", 0))
	map_summary["render_object_count"] = int(counts.get("render_object_count", 0))
	map_summary["object_motion_count"] = int(counts.get("object_motion_count", 0))
	map_summary["trainer_sighting_count"] = int(counts.get("trainer_sighting_count", 0))
	map_summary["warp_transition_count"] = int(counts.get("warp_transition_count", 0))
	map_summary["connection_transition_count"] = int(counts.get("connection_transition_count", 0))
	map_summary["event_activation_count"] = int(counts.get("event_activation_count", 0))
	map_summary["tile_animation_frame"] = int(counts.get("tile_animation_frame", 0))
	map_summary["object_event_gating_count"] = int(counts.get("object_event_gating_count", 0))
	map_summary["wild_encounter_count"] = int(counts.get("wild_encounter_count", 0))
	map_summary["wild_encounter_state"] = wild_encounter_state.duplicate(true)
	map_summary["wild_encounter_payloads"] = _build_wild_encounter_eligibility_payloads().duplicate(true)
	map_summary["warp_transition_payloads"] = _build_warp_transition_payloads().duplicate(true)
	map_summary["connection_transition_payloads"] = _build_connection_transition_payloads().duplicate(true)
	map_summary["event_activation_records"] = _build_event_activation_records().duplicate(true)
	map_summary["object_event_gating_records"] = _build_object_event_gating_records().duplicate(true)
	map_summary["tile_animation_state"] = tile_animation_state.duplicate(true)
	current_map_payload["object_state_count"] = int(counts.get("object_state_count", 0))
	current_map_payload["hidden_object_count"] = int(counts.get("hidden_object_count", 0))
	current_map_payload["removed_object_count"] = int(counts.get("removed_object_count", 0))
	current_map_payload["defeated_object_count"] = int(counts.get("defeated_object_count", 0))
	current_map_payload["event_flag_count"] = int(counts.get("event_flag_count", 0))
	current_map_payload["engine_flag_count"] = int(counts.get("engine_flag_count", 0))
	current_map_payload["map_callbacks_executed_count"] = int(counts.get("map_callbacks_executed_count", 0))
	current_map_payload["changed_block_count"] = int(counts.get("changed_block_count", 0))
	current_map_payload["render_object_count"] = int(counts.get("render_object_count", 0))
	current_map_payload["object_motion_count"] = int(counts.get("object_motion_count", 0))
	current_map_payload["trainer_sighting_count"] = int(counts.get("trainer_sighting_count", 0))
	current_map_payload["warp_transition_count"] = int(counts.get("warp_transition_count", 0))
	current_map_payload["connection_transition_count"] = int(counts.get("connection_transition_count", 0))
	current_map_payload["event_activation_count"] = int(counts.get("event_activation_count", 0))
	current_map_payload["tile_animation_frame"] = int(counts.get("tile_animation_frame", 0))
	current_map_payload["object_event_gating_count"] = int(counts.get("object_event_gating_count", 0))
	current_map_payload["wild_encounter_count"] = int(counts.get("wild_encounter_count", 0))
	current_map_payload["wild_encounter_state"] = wild_encounter_state.duplicate(true)
	current_map_payload["wild_encounter_payloads"] = _build_wild_encounter_eligibility_payloads().duplicate(true)
	current_map_payload["warp_transition_payloads"] = _build_warp_transition_payloads().duplicate(true)
	current_map_payload["connection_transition_payloads"] = _build_connection_transition_payloads().duplicate(true)
	current_map_payload["event_activation_records"] = _build_event_activation_records().duplicate(true)
	current_map_payload["object_event_gating_records"] = _build_object_event_gating_records().duplicate(true)
	current_map_payload["tile_animation_state"] = tile_animation_state.duplicate(true)

func _build_render_object_states() -> Array:
	var render_objects: Array = []
	var seen_ids: Dictionary = {}
	for index in range(current_object_events.size()):
		var event_record := _get_object_record_from_event(index + 1)
		if event_record.is_empty():
			continue
		var object_id := _normalize_object_id(event_record.get("object_id", ""))
		if object_id.is_empty() or object_id == "PLAYER":
			continue
		seen_ids[object_id] = true
		var state: Dictionary = Dictionary(object_states.get(object_id, event_record)).duplicate(true)
		if state.is_empty():
			state = event_record.duplicate(true)
		state["object_id"] = object_id
		state["object_index"] = int(state.get("object_index", event_record.get("object_index", index + 1)))
		state["event"] = Dictionary(event_record.get("event", state.get("event", {}))).duplicate(true)
		var event_flag := str(state.get("event_flag", event_record.get("event_flag", ""))).strip_edges()
		state["event_flag"] = event_flag
		state["event_flag_state"] = bool(state.get("event_flag_state", not event_flag.is_empty() and get_event_flag(event_flag)))
		state["visible"] = bool(state.get("visible", event_record.get("visible", true)))
		state["hidden"] = bool(state.get("hidden", not bool(state.get("visible", true))))
		state["removed"] = bool(state.get("removed", false))
		state["defeated"] = bool(state.get("defeated", false))
		if bool(state.get("visible", true)) and not bool(state.get("removed", false)):
			render_objects.append(state)
	var extra_ids: Array[String] = []
	for key in object_states.keys():
		var object_id := _normalize_object_id(key)
		if object_id.is_empty() or object_id == "PLAYER" or seen_ids.has(object_id):
			continue
		extra_ids.append(object_id)
	extra_ids.sort()
	for object_id in extra_ids:
		var state: Dictionary = Dictionary(object_states.get(object_id, {})).duplicate(true)
		if state.is_empty():
			continue
		state["object_id"] = object_id
		if not state.has("object_index"):
			state["object_index"] = int(state.get("index", 0))
		if not state.has("visible"):
			state["visible"] = true
		if not state.has("hidden"):
			state["hidden"] = not bool(state.get("visible", true))
		if not state.has("removed"):
			state["removed"] = false
		if not state.has("defeated"):
			state["defeated"] = false
		if bool(state.get("visible", true)) and not bool(state.get("removed", false)):
			render_objects.append(state)
	return render_objects

func _resolve_blocks_key() -> String:
	if not map_title.is_empty():
		var title_key: String = "%s_Blocks" % map_title
		if map_blocks.has(title_key):
			return title_key
	if not map_id.is_empty():
		var id_key: String = "%s_Blocks" % map_id
		if map_blocks.has(id_key):
			return id_key
	return ""

func _first_available_map_key() -> String:
	var keys: Array[String] = _sorted_map_keys()
	if keys.is_empty():
		return ""
	return keys[0]

func _dictionary_value(value: Variant) -> Dictionary:
	if typeof(value) == TYPE_DICTIONARY:
		return Dictionary(value)
	return {}

func _array_value(value: Variant) -> Array:
	if typeof(value) == TYPE_ARRAY:
		return Array(value)
	return []

func current_map_data_from_summary() -> void:
	runtime_summary = _build_runtime_summary()

func _is_int_like(value: Variant) -> bool:
	match typeof(value):
		TYPE_INT:
			return true
		TYPE_FLOAT:
			return true
		TYPE_STRING:
			return str(value).is_valid_int()
		_:
			return false

func _normalized_map_summary(summary: Dictionary) -> Dictionary:
	var result: Dictionary = summary.duplicate(true)
	var runtime_name := str(result.get("name", result.get("map_name", "")))
	var map_constant := str(result.get("map_constant", result.get("constant", result.get("map_key", ""))))
	var attribute_entry: Dictionary = {}
	if asset_index != null and asset_index.has_method("load_map_attributes_for_map") and not runtime_name.is_empty():
		attribute_entry = _dictionary_value(asset_index.call("load_map_attributes_for_map", runtime_name))
	if attribute_entry.is_empty() and asset_index != null and asset_index.has_method("load_map_attributes_for_map") and not map_constant.is_empty():
		attribute_entry = _dictionary_value(asset_index.call("load_map_attributes_for_map", map_constant))
	if not attribute_entry.is_empty():
		for key in attribute_entry.keys():
			if not result.has(key) or result.get(key) == null or str(result.get(key)).is_empty():
				result[key] = attribute_entry.get(key)
	if not result.has("map_key") or str(result.get("map_key", "")).is_empty():
		result["map_key"] = map_constant if not map_constant.is_empty() else runtime_name
	if not result.has("map_name") or str(result.get("map_name", "")).is_empty():
		result["map_name"] = runtime_name if not runtime_name.is_empty() else str(result.get("title", result.get("map_key", "")))
	if not result.has("title") or str(result.get("title", "")).is_empty():
		result["title"] = str(result.get("map_name", result.get("name", result.get("map_key", ""))))
	if not result.has("map_constant") or str(result.get("map_constant", "")).is_empty():
		result["map_constant"] = map_constant if not map_constant.is_empty() else str(result.get("constant", result.get("map_key", "")))
	if not result.has("group_name") or str(result.get("group_name", "")).is_empty():
		result["group_name"] = str(result.get("map_group_constant", result.get("groupName", "")))
	if not result.has("group_id"):
		result["group_id"] = int(result.get("groupId", -1))
	if not result.has("map_id") or not _is_int_like(result.get("map_id")):
		result["map_id"] = int(result.get("mapId", -1))
	if not result.has("phone_service"):
		result["phone_service"] = int(result.get("phoneService", 0))
	if not result.has("tileset_name"):
		result["tileset_name"] = str(result.get("tilesetName", ""))
	return result

func set_map(summary: Dictionary, spawn: Dictionary) -> void:
	var normalized_summary: Dictionary = _normalized_map_summary(summary)
	map_summary = normalized_summary.duplicate(true)
	current_map_payload = normalized_summary.duplicate(true)
	spawn_summary = spawn.duplicate(true)
	map_id = str(normalized_summary.get("map_id", normalized_summary.get("id", map_id)))
	map_title = str(normalized_summary.get("title", normalized_summary.get("name", map_id)))
	map_dimensions = _vector_from_dictionary(normalized_summary, Vector2i(0, 0))
	player_tile = _player_start_from_spawn(spawn)
	player_facing = _normalize_direction(str(spawn.get("facing", MOVE_DOWN)))
	current_map_key = str(normalized_summary.get("map_key", normalized_summary.get("constant", map_id)))
	current_map_name = map_title
	current_map_constant = str(normalized_summary.get("map_constant", normalized_summary.get("constant", map_id)))
	current_map_group_name = str(normalized_summary.get("group_name", ""))
	current_map_environment = str(normalized_summary.get("environment", ""))
	current_width = map_dimensions.x
	current_height = map_dimensions.y
	current_group_id = int(normalized_summary.get("group_id", -1))
	current_map_id = int(normalized_summary.get("map_id", -1))
	current_phone_service = int(normalized_summary.get("phone_service", 0))
	current_tileset_name = str(normalized_summary.get("tileset_name", ""))
	current_location = str(normalized_summary.get("location", ""))
	current_map_block_key = _resolve_blocks_key()
	current_spawn_point = _normalize_spawn_point_state(spawn, player_tile)
	current_connections = _array_value(normalized_summary.get("connections", []))
	current_warps = _array_value(normalized_summary.get("warps", []))
	current_bg_events = _array_value(normalized_summary.get("bg_events", []))
	current_object_events = _array_value(normalized_summary.get("object_events", []))
	_sync_scene_for_map(current_map_key)
	_sync_object_event_states()
	_refresh_player_object()
	_update_tile_animation_state()
	_update_wild_encounter_metadata()
	map_summary["map_key"] = current_map_key
	map_summary["map_name"] = current_map_name
	map_summary["map_constant"] = current_map_constant
	map_summary["width"] = current_width
	map_summary["height"] = current_height
	map_summary["group_id"] = current_group_id
	map_summary["map_id"] = current_map_id
	map_summary["group_name"] = current_map_group_name
	run_map_callbacks(current_map_key, "MAPCALLBACK_NEWMAP")
	_update_state_summary_metadata()
	current_map_data_from_summary()
	_sync_selected_map_key()
	_update_tile_animation_state()
	_update_wild_encounter_metadata()
	runtime_summary = _build_runtime_summary()
	last_runtime_note = "map -> %s" % map_id
	debug_lines.append(last_runtime_note)

func request_move(direction: String) -> void:
	var normalized := _normalize_direction(direction)
	if normalized == MOVE_NONE:
		return
	pending_move = normalized
	last_move_direction = normalized
	movement_state = "moving" if not movement_locked else "locked"
	last_move_request = {
		"direction": normalized,
		"from_tile": {"x": player_tile.x, "y": player_tile.y},
		"map_key": current_map_key,
		"step": fixed_step_count,
		"movement_locked": movement_locked,
	}
	last_runtime_note = "move requested: %s" % normalized
	runtime_summary = _build_runtime_summary()

func set_movement_locked(is_locked: bool, reason: String = "") -> void:
	movement_locked = is_locked
	if not reason.is_empty():
		collision_reason = reason
	if movement_locked:
		movement_state = "locked"
	else:
		movement_state = "idle" if pending_move == MOVE_NONE else "moving"
	last_runtime_note = "movement locked" if movement_locked else "movement unlocked"
	runtime_summary = _build_runtime_summary()

func set_warp_target(target: String) -> void:
	warp_requested = not target.is_empty()
	warp_target = target
	last_warp_result = {
		"requested": warp_requested,
		"target": warp_target,
		"source": "manual",
	}
	if warp_requested:
		debug_lines.append("warp -> %s" % warp_target)
	last_runtime_note = "warp target set: %s" % warp_target
	runtime_summary = _build_runtime_summary()

func set_map_scene(map_key: String, scene: String) -> void:
	var normalized_map := _resolve_map_key(map_key)
	if normalized_map.is_empty():
		normalized_map = current_map_key if not current_map_key.is_empty() else get_selected_map_key()
	if normalized_map.is_empty():
		return
	var normalized_scene := scene.strip_edges()
	if normalized_scene.is_empty():
		map_scenes.erase(normalized_map)
		map_scene_indices.erase(normalized_map)
		if normalized_map == current_map_key:
			scene_name = ""
	else:
		map_scenes[normalized_map] = normalized_scene
		map_scene_indices[normalized_map] = int(map_scene_indices.get(normalized_map, 0))
		if normalized_map == current_map_key or normalized_map == get_selected_map_key():
			scene_name = normalized_scene
	last_runtime_note = "scene -> %s:%s" % [normalized_map, normalized_scene if not normalized_scene.is_empty() else "none"]
	debug_lines.append(last_runtime_note)
	while debug_lines.size() > 8:
		debug_lines.pop_front()
	runtime_summary = _build_runtime_summary()

func check_scene(map_key: String = "") -> int:
	var normalized_map := _resolve_map_key(map_key)
	if normalized_map.is_empty():
		normalized_map = current_map_key if not current_map_key.is_empty() else get_selected_map_key()
	if normalized_map.is_empty():
		return 0
	_sync_scene_for_map(normalized_map)
	var index := int(map_scene_indices.get(normalized_map, 0))
	var map_scene := str(map_scenes.get(normalized_map, ""))
	scene_name = map_scene
	_set_scene_state(map_scene, index)
	last_runtime_note = "scene check: %s -> %s (%d)" % [normalized_map, map_scene if not map_scene.is_empty() else "none", index]
	debug_lines.append(last_runtime_note)
	while debug_lines.size() > 8:
		debug_lines.pop_front()
	runtime_summary = _build_runtime_summary()
	return index

func start_following(follower, leader, options: Dictionary = {}) -> void:
	follow_state = {
		"active": true,
		"follower": _normalize_object_id(follower),
		"leader": _normalize_object_id(leader),
		"options": options.duplicate(true),
	}
	last_runtime_note = "follow: %s -> %s" % [str(follow_state.get("follower", "")), str(follow_state.get("leader", ""))]
	debug_lines.append(last_runtime_note)
	while debug_lines.size() > 8:
		debug_lines.pop_front()
	runtime_summary = _build_runtime_summary()

func stop_following() -> void:
	if follow_state.is_empty():
		follow_state = {"active": false}
	else:
		follow_state["active"] = false
	last_runtime_note = "follow stopped"
	debug_lines.append(last_runtime_note)
	while debug_lines.size() > 8:
		debug_lines.pop_front()
	_update_state_summary_metadata()
	runtime_summary = _build_runtime_summary()

func lock_player_movement() -> void:
	set_movement_locked(true, "player movement locked")

func unlock_player_movement() -> void:
	set_movement_locked(false, "")

func lock_all_movement() -> void:
	lock_player_movement()

func unlock_all_movement() -> void:
	unlock_player_movement()

func stop_player_movement() -> void:
	pending_move = MOVE_NONE
	movement_state = "idle" if not movement_locked else "locked"
	last_move_request = {}
	last_move_result = {
		"state": "stopped",
		"blocked": false,
		"moved": false,
		"reason": "stopped",
	}
	last_runtime_note = "player movement stopped"
	debug_lines.append(last_runtime_note)
	while debug_lines.size() > 8:
		debug_lines.pop_front()
	runtime_summary = _build_runtime_summary()

func appear_object(object_id, options: Dictionary = {}) -> void:
	var normalized_id := _normalize_object_id(object_id)
	if normalized_id.is_empty():
		return
	var state := Dictionary(object_states.get(normalized_id, {}))
	state["visible"] = true
	state["object_id"] = normalized_id
	state["options"] = options.duplicate(true)
	object_states[normalized_id] = state
	last_runtime_note = "object appeared: %s" % normalized_id
	debug_lines.append(last_runtime_note)
	while debug_lines.size() > 8:
		debug_lines.pop_front()
	_update_state_summary_metadata()
	runtime_summary = _build_runtime_summary()

func remove_object(object_id, options: Dictionary = {}) -> void:
	var normalized_id := _normalize_object_id(object_id)
	if normalized_id.is_empty():
		return
	if object_states.has(normalized_id):
		var state := Dictionary(object_states.get(normalized_id, {}))
		state["visible"] = false
		state["removed"] = true
		state["options"] = options.duplicate(true)
		object_states[normalized_id] = state
	else:
		object_states[normalized_id] = {
			"object_id": normalized_id,
			"visible": false,
			"removed": true,
			"options": options.duplicate(true),
		}
	last_runtime_note = "object removed: %s" % normalized_id
	debug_lines.append(last_runtime_note)
	while debug_lines.size() > 8:
		debug_lines.pop_front()
	_update_state_summary_metadata()
	runtime_summary = _build_runtime_summary()

func move_object(object_id, map_x: int, map_y: int) -> void:
	var normalized_id := _normalize_object_id(object_id)
	if normalized_id.is_empty():
		return
	var state := Dictionary(object_states.get(normalized_id, {}))
	state["object_id"] = normalized_id
	state["tile_x"] = int(map_x)
	state["tile_y"] = int(map_y)
	state["visible"] = true
	object_states[normalized_id] = state
	last_runtime_note = "object moved: %s -> %d,%d" % [normalized_id, map_x, map_y]
	debug_lines.append(last_runtime_note)
	while debug_lines.size() > 8:
		debug_lines.pop_front()
	_update_state_summary_metadata()
	runtime_summary = _build_runtime_summary()

func show_emote(emote_id: String, obj, duration: int) -> void:
	var normalized_id := _normalize_object_id(obj)
	special_state = {
		"emote_id": emote_id,
		"object_id": normalized_id,
		"duration": max(0, duration),
	}
	last_runtime_note = "emote: %s on %s" % [emote_id, normalized_id if not normalized_id.is_empty() else "unknown"]
	debug_lines.append(last_runtime_note)
	while debug_lines.size() > 8:
		debug_lines.pop_front()
	_update_state_summary_metadata()
	runtime_summary = _build_runtime_summary()

func _apply_event_flag_update(event_name: String, value: bool) -> void:
	var updated := false
	for index in range(current_object_events.size()):
		var event_record := _get_object_record_from_event(index + 1)
		if event_record.is_empty():
			continue
		var event_flag := str(event_record.get("event_flag", event_record.get("eventFlag", ""))).strip_edges()
		if event_flag.is_empty() or event_flag != event_name:
			continue
		var object_id := _normalize_object_id(event_record.get("object_id", event_record.get("script", event_record.get("label", ""))))
		if object_id.is_empty():
			object_id = str(index + 1)
		var state := Dictionary(object_states.get(object_id, event_record)).duplicate(true)
		state["object_id"] = object_id
		state["object_index"] = index + 1
		state["event_flag"] = event_flag
		state["event_flag_state"] = value
		state["visible"] = not value
		state["hidden"] = value
		state["removed"] = value
		state["defeated"] = value
		state["event"] = Dictionary(event_record.get("event", {})).duplicate(true)
		object_states[object_id] = state
		updated = true
	if updated:
		last_runtime_note = "event flag applied: %s=%s" % [event_name, str(value).to_lower()]
		debug_lines.append(last_runtime_note)
		while debug_lines.size() > 8:
			debug_lines.pop_front()
		_update_state_summary_metadata()
		runtime_summary = _build_runtime_summary()

func wait_sfx(callback: Callable = Callable()) -> void:
	if callback.is_valid():
		callback.call()
	last_runtime_note = "wait sfx"
	debug_lines.append(last_runtime_note)
	while debug_lines.size() > 8:
		debug_lines.pop_front()
	runtime_summary = _build_runtime_summary()

func check_for_warp_event(_options: Dictionary = {}) -> bool:
	var options: Dictionary = _dictionary_value(_options)
	var allow_script := bool(options.get("allow_script", options.get("allowScript", false)))
	var ignore_cooldown := bool(options.get("ignore_cooldown", options.get("ignoreCooldown", false)))
	if warp_requested:
		last_runtime_note = "warp pending: %s" % warp_target
		debug_lines.append(last_runtime_note)
		while debug_lines.size() > 8:
			debug_lines.pop_front()
		runtime_summary = _build_runtime_summary()
		return true
	if warp_cooldown > 0 and not ignore_cooldown:
		last_runtime_note = "warp cooldown: %d" % warp_cooldown
		debug_lines.append(last_runtime_note)
		while debug_lines.size() > 8:
			debug_lines.pop_front()
		runtime_summary = _build_runtime_summary()
		return false
	if not active_warp_tile.is_empty() and not ignore_cooldown and active_warp_tile.size() >= 3:
		if str(active_warp_tile[0]) == current_map_key and int(active_warp_tile[1]) == player_tile.x and int(active_warp_tile[2]) == player_tile.y:
			last_runtime_note = "warp blocked: active warp tile"
			debug_lines.append(last_runtime_note)
			while debug_lines.size() > 8:
				debug_lines.pop_front()
			runtime_summary = _build_runtime_summary()
			return false
	if not allow_script and dialogue_state.has("active") and bool(dialogue_state.get("active", false)):
		return false
	var tile_key := "%d,%d" % [player_tile.x, player_tile.y]
	var warps_on_tile: Array = Array(warp_permission_cache.get(tile_key, []))
	if warps_on_tile.is_empty():
		_refresh_warp_permissions()
		warps_on_tile = Array(warp_permission_cache.get(tile_key, []))
	if warps_on_tile.is_empty():
		if _should_request_warp(player_tile):
			warps_on_tile = [{
				"warp_id": 0,
				"target_map_constant": current_map_constant if not current_map_constant.is_empty() else map_id,
				"target_warp_id": 0,
				"permission": null,
			}]
		else:
			return false
	var warp_entry: Dictionary = Dictionary(warps_on_tile[0])
	warp_requested = true
	active_warp_tile = [current_map_key, player_tile.x, player_tile.y]
	warp_cooldown = 1
	var target_map_constant := str(warp_entry.get("target_map_constant", current_map_constant if not current_map_constant.is_empty() else map_id))
	var target_warp_id := int(warp_entry.get("target_warp_id", 0))
	warp_target = "%s:%d" % [target_map_constant, target_warp_id]
	last_warp_result = {
		"requested": true,
		"target": warp_target,
		"tile": {"x": player_tile.x, "y": player_tile.y},
		"map_key": current_map_key,
		"warp_id": int(warp_entry.get("warp_id", 0)),
		"target_map_constant": target_map_constant,
		"target_warp_id": target_warp_id,
	}
	last_runtime_note = "warp requested: %s" % warp_target
	debug_lines.append(last_runtime_note)
	while debug_lines.size() > 8:
		debug_lines.pop_front()
	runtime_summary = _build_runtime_summary()
	return true
	return false

func handle_cut(x: int, y: int) -> void:
	_record_field_move("Cut", x, y)
	runtime_summary = _build_runtime_summary()

func handle_surf(x: int, y: int) -> void:
	_record_field_move("Surf", x, y)
	runtime_summary = _build_runtime_summary()

func _handle_hm(move_name: String, x: int, y: int, _player_state) -> void:
	_record_field_move(move_name, x, y)
	runtime_summary = _build_runtime_summary()

func handle_flash() -> void:
	_record_field_move("Flash", player_tile.x, player_tile.y)
	runtime_summary = _build_runtime_summary()

func handle_fly(x: int, y: int) -> void:
	reload_map_after_battle = true
	set_warp_target("%s:%d,%d" % [current_map_key, x, y])
	_record_field_move("Fly", x, y)
	runtime_summary = _build_runtime_summary()

func request_music(music_id: String, role: String = "") -> void:
	music_request = {
		"music_id": music_id,
		"role": role,
		"action": "request",
	}
	last_runtime_note = "music request: %s" % music_id
	debug_lines.append(last_runtime_note)
	while debug_lines.size() > 8:
		debug_lines.pop_front()
	runtime_summary = _build_runtime_summary()

func fade_to_music(music_id: String, speed_frames: int, role: String = "") -> void:
	music_request = {
		"music_id": music_id,
		"speed_frames": max(0, speed_frames),
		"role": role,
		"action": "fade",
	}
	last_runtime_note = "music fade: %s" % music_id
	debug_lines.append(last_runtime_note)
	while debug_lines.size() > 8:
		debug_lines.pop_front()
	runtime_summary = _build_runtime_summary()

func execute_special(function_name: String, context: Dictionary = {}) -> Variant:
	var normalized := function_name.strip_edges()
	if normalized.is_empty():
		return null
	var result: Variant = null
	match normalized:
		"HealParty":
			special_state["last_special"] = normalized
			special_state["healed_party"] = true
			result = true
		"HealMachineAnim":
			special_state["last_special"] = normalized
			special_state["heal_machine_anim"] = context.duplicate(true)
			result = true
		"PlayMapMusic":
			request_music(str(context.get("music_id", context.get("music", ""))), str(context.get("role", "special")))
			result = true
		"RestartMapMusic":
			request_music(str(context.get("music_id", current_map_constant)), "restart")
			result = true
		"FadeOutMusic":
			fade_to_music(str(context.get("music_id", "")), int(context.get("speed_frames", 0)), str(context.get("role", "special")))
			result = true
		"RefreshSprites":
			special_state["refresh_sprites"] = true
			result = true
		"LoadMapPalettes", "UpdatePlayerSprite", "UpdateSprites", "ReloadSpritesNoPalettes":
			special_state["last_special"] = normalized
			result = true
		_:
			special_state["last_special"] = normalized
			special_state["context"] = context.duplicate(true)
			last_runtime_note = "unsupported special: %s" % normalized
			result = false
	if result == true:
		last_runtime_note = "special: %s" % normalized
	debug_lines.append(last_runtime_note)
	while debug_lines.size() > 8:
		debug_lines.pop_front()
	runtime_summary = _build_runtime_summary()
	return result

func handle_special(function_name: String, context: Dictionary = {}) -> Variant:
	return execute_special(function_name, context)

func tick() -> void:
	fixed_step_count += 1
	if warp_cooldown > 0:
		warp_cooldown = max(0, warp_cooldown - 1)
		if warp_cooldown == 0 and not active_warp_tile.is_empty():
			active_warp_tile = []
	_update_tile_animation_state()
	_advance_object_motion_counters()
	collision_detected = false
	collision_reason = ""
	if movement_locked or pending_move == MOVE_NONE:
		movement_state = "locked" if movement_locked else ("warping" if warp_requested else "idle")
		last_runtime_note = "tick %d: %s" % [fixed_step_count, movement_state]
		runtime_summary = _build_runtime_summary()
		_push_debug()
		return
	var next_tile := _next_tile_for_move(pending_move)
	last_move_request = {
		"direction": pending_move,
		"from_tile": {"x": player_tile.x, "y": player_tile.y},
		"to_tile": {"x": next_tile.x, "y": next_tile.y},
		"map_key": current_map_key,
		"step": fixed_step_count,
		"movement_locked": movement_locked,
	}
	if collision_hook.is_valid():
		var collision_override: Variant = collision_hook.call(self, next_tile, pending_move)
		if typeof(collision_override) == TYPE_DICTIONARY:
			var collision_data: Dictionary = Dictionary(collision_override)
			if bool(collision_data.get("blocked", false)):
				collision_detected = true
				collision_reason = str(collision_data.get("reason", "collision_hook"))
				movement_state = "blocked"
				last_collision_result = {
					"blocked": true,
					"reason": collision_reason,
					"tile": {"x": next_tile.x, "y": next_tile.y},
					"direction": pending_move,
					"step": fixed_step_count,
				}
				last_move_result = {
					"state": "blocked",
					"blocked": true,
					"moved": false,
					"reason": collision_reason,
					"direction": pending_move,
					"step": fixed_step_count,
				}
				last_warp_result = {}
				last_runtime_note = "collision blocked: %s" % collision_reason
				runtime_summary = _build_runtime_summary()
				_push_debug()
				pending_move = MOVE_NONE
				return
	if _is_tile_out_of_bounds(next_tile):
		collision_detected = true
		collision_reason = "out_of_bounds"
		movement_state = "blocked"
		last_collision_result = {
			"blocked": true,
			"reason": collision_reason,
			"tile": {"x": next_tile.x, "y": next_tile.y},
			"direction": pending_move,
			"step": fixed_step_count,
		}
		last_move_result = {
			"state": "blocked",
			"blocked": true,
			"moved": false,
			"reason": collision_reason,
			"direction": pending_move,
			"step": fixed_step_count,
		}
		last_warp_result = {}
		last_runtime_note = "collision blocked: %s" % collision_reason
		runtime_summary = _build_runtime_summary()
		_push_debug()
		pending_move = MOVE_NONE
		return
	var previous_tile := player_tile
	player_tile = next_tile
	player_facing = pending_move
	movement_state = "moving"
	last_move_result = {
		"state": "moved",
		"blocked": false,
		"moved": true,
		"reason": "",
		"direction": pending_move,
		"from_tile": {"x": previous_tile.x, "y": previous_tile.y},
		"to_tile": {"x": player_tile.x, "y": player_tile.y},
		"step": fixed_step_count,
	}
	last_collision_result = {}
	if _should_request_warp(player_tile):
		warp_requested = true
		warp_target = _default_warp_target()
		last_warp_result = {
			"requested": true,
			"target": warp_target,
			"tile": {"x": player_tile.x, "y": player_tile.y},
			"map_key": current_map_key,
			"step": fixed_step_count,
		}
		if warp_hook.is_valid():
			var warp_override: Variant = warp_hook.call(self, player_tile, warp_target)
			if typeof(warp_override) == TYPE_DICTIONARY:
				var warp_data: Dictionary = Dictionary(warp_override)
				if bool(warp_data.get("requested", true)) == false:
					warp_requested = false
					warp_target = ""
					last_warp_result["requested"] = false
					last_warp_result["target"] = ""
				elif warp_data.has("warp_target"):
					warp_target = str(warp_data.get("warp_target", warp_target))
					last_warp_result["target"] = warp_target
		debug_lines.append("warp hook -> %s" % warp_target)
		movement_state = "warping"
		last_runtime_note = "warp requested: %s" % warp_target
	else:
		last_runtime_note = "move complete: %s" % pending_move
		movement_state = "idle"
	pending_move = MOVE_NONE
	runtime_summary = _build_runtime_summary()
	_push_debug()

func get_state() -> Dictionary:
	return {
		"map_id": map_id,
		"map_title": map_title,
		"map_name": current_map_name,
		"map_constant": current_map_constant,
		"current_map_key": current_map_key,
		"selected_map_key": get_selected_map_key(),
		"selected_map_index": selected_map_index,
		"available_map_keys": available_map_keys.duplicate(),
		"map_manifest": map_manifest.duplicate(true),
		"map_blocks": map_blocks.duplicate(true),
		"map_scenes": map_scenes.duplicate(true),
		"map_scene_indices": map_scene_indices.duplicate(true),
		"scene_name": scene_name,
		"current_map_group_name": current_map_group_name,
		"current_map_environment": current_map_environment,
		"current_map_block_key": current_map_block_key,
		"current_width": current_width,
		"current_height": current_height,
		"current_group_id": current_group_id,
		"current_map_id": current_map_id,
		"current_phone_service": current_phone_service,
		"current_tileset_name": current_tileset_name,
		"current_location": current_location,
		"map_dimensions": {"x": map_dimensions.x, "y": map_dimensions.y},
		"player_tile": {"x": player_tile.x, "y": player_tile.y},
		"player_facing": player_facing,
		"movement_locked": movement_locked,
		"movement_state": movement_state,
		"collision_detected": collision_detected,
		"collision_reason": collision_reason,
		"warp_requested": warp_requested,
		"warp_target": warp_target,
		"last_move_direction": last_move_direction,
		"last_move_request": last_move_request.duplicate(true),
		"last_move_result": last_move_result.duplicate(true),
		"last_collision_result": last_collision_result.duplicate(true),
		"last_warp_result": last_warp_result.duplicate(true),
		"last_runtime_note": last_runtime_note,
		"fixed_step_count": fixed_step_count,
		"map_summary": map_summary.duplicate(true),
		"spawn_summary": spawn_summary.duplicate(true),
		"current_spawn_point": current_spawn_point.duplicate(true),
		"current_connections": current_connections.duplicate(true),
		"current_warps": current_warps.duplicate(true),
		"current_bg_events": current_bg_events.duplicate(true),
		"current_object_events": current_object_events.duplicate(true),
		"pending_move": pending_move,
		"reload_map_after_battle": reload_map_after_battle,
		"music_request": music_request.duplicate(true),
		"follow_state": follow_state.duplicate(true),
		"object_states": object_states.duplicate(true),
		"special_state": special_state.duplicate(true),
		"player_object": player_object.duplicate(true),
		"dialogue_state": dialogue_state.duplicate(true),
		"event_flags": event_flags.duplicate(true),
		"engine_flags": engine_flags.duplicate(true),
		"current_map_payload": current_map_payload.duplicate(true),
		"warp_permission_cache": warp_permission_cache.duplicate(true),
		"active_warp_tile": active_warp_tile.duplicate(true),
		"warp_cooldown": warp_cooldown,
		"tile_animation_state": tile_animation_state.duplicate(true),
		"wild_encounter_state": wild_encounter_state.duplicate(true),
		"debug_lines": debug_lines.duplicate(),
	}

func from_state(data: Dictionary) -> void:
	map_id = str(data.get("map_id", ""))
	map_title = str(data.get("map_title", map_id))
	current_map_name = str(data.get("map_name", map_title))
	current_map_constant = str(data.get("map_constant", map_id))
	current_map_key = str(data.get("current_map_key", data.get("selected_map_key", current_map_key)))
	selected_map_key = str(data.get("selected_map_key", current_map_key))
	selected_map_index = int(data.get("selected_map_index", selected_map_index))
	available_map_keys = []
	for key in Array(data.get("available_map_keys", [])):
		available_map_keys.append(str(key))
	map_manifest = _dictionary_value(data.get("map_manifest", map_manifest))
	map_blocks = _dictionary_value(data.get("map_blocks", map_blocks))
	map_scenes = _dictionary_value(data.get("map_scenes", map_scenes))
	map_scene_indices = _dictionary_value(data.get("map_scene_indices", map_scene_indices))
	scene_name = str(data.get("scene_name", scene_name))
	current_map_group_name = str(data.get("current_map_group_name", current_map_group_name))
	current_map_environment = str(data.get("current_map_environment", current_map_environment))
	current_map_block_key = str(data.get("current_map_block_key", current_map_block_key))
	current_group_id = int(data.get("current_group_id", data.get("current_map_group_id", current_group_id)))
	current_map_id = int(data.get("current_map_id", current_map_id))
	current_phone_service = int(data.get("current_phone_service", current_phone_service))
	current_tileset_name = str(data.get("current_tileset_name", current_tileset_name))
	current_location = str(data.get("current_location", current_location))
	var restored_player_tile := _vector_from_value(data.get("player_tile", {"x": 0, "y": 0}), Vector2i(0, 0))
	var restored_player_facing := _normalize_direction(str(data.get("player_facing", MOVE_DOWN)))
	movement_locked = bool(data.get("movement_locked", false))
	movement_state = str(data.get("movement_state", "idle"))
	collision_detected = bool(data.get("collision_detected", false))
	collision_reason = str(data.get("collision_reason", ""))
	warp_requested = bool(data.get("warp_requested", false))
	warp_target = str(data.get("warp_target", ""))
	last_move_direction = _normalize_direction(str(data.get("last_move_direction", MOVE_NONE)))
	last_move_request = _dictionary_value(data.get("last_move_request", {}))
	last_move_result = _dictionary_value(data.get("last_move_result", {}))
	last_collision_result = _dictionary_value(data.get("last_collision_result", {}))
	last_warp_result = _dictionary_value(data.get("last_warp_result", {}))
	last_runtime_note = str(data.get("last_runtime_note", ""))
	fixed_step_count = max(0, int(data.get("fixed_step_count", 0)))
	map_summary = Dictionary(data.get("map_summary", {}))
	spawn_summary = Dictionary(data.get("spawn_summary", {}))
	var restored_width := int(data.get("current_width", 0))
	var restored_height := int(data.get("current_height", 0))
	if restored_width <= 0:
		restored_width = int(map_summary.get("width", map_summary.get("size_x", 0)))
	if restored_height <= 0:
		restored_height = int(map_summary.get("height", map_summary.get("size_y", 0)))
	map_dimensions = _vector_from_value(data.get("map_dimensions", {"x": restored_width, "y": restored_height}), Vector2i(restored_width, restored_height))
	if restored_width <= 0:
		restored_width = map_dimensions.x
	if restored_height <= 0:
		restored_height = map_dimensions.y
	current_width = restored_width
	current_height = restored_height
	current_spawn_point = _normalize_spawn_point_state(Dictionary(data.get("current_spawn_point", {})), restored_player_tile)
	current_connections = _array_value(data.get("current_connections", current_connections))
	current_warps = _array_value(data.get("current_warps", current_warps))
	current_bg_events = _array_value(data.get("current_bg_events", current_bg_events))
	current_object_events = _array_value(data.get("current_object_events", current_object_events))
	pending_move = _normalize_direction(str(data.get("pending_move", MOVE_NONE)))
	reload_map_after_battle = bool(data.get("reload_map_after_battle", reload_map_after_battle))
	music_request = _dictionary_value(data.get("music_request", music_request))
	follow_state = _dictionary_value(data.get("follow_state", follow_state))
	object_states = _dictionary_value(data.get("object_states", object_states))
	special_state = _dictionary_value(data.get("special_state", special_state))
	player_object = _dictionary_value(data.get("player_object", player_object))
	dialogue_state = _dictionary_value(data.get("dialogue_state", dialogue_state))
	event_flags = _dictionary_value(data.get("event_flags", event_flags))
	engine_flags = _dictionary_value(data.get("engine_flags", engine_flags))
	current_map_payload = _dictionary_value(data.get("current_map_payload", current_map_payload))
	warp_permission_cache = _dictionary_value(data.get("warp_permission_cache", warp_permission_cache))
	active_warp_tile = Array(data.get("active_warp_tile", active_warp_tile)).duplicate(true)
	warp_cooldown = max(0, int(data.get("warp_cooldown", warp_cooldown)))
	tile_animation_state = _dictionary_value(data.get("tile_animation_state", tile_animation_state))
	wild_encounter_state = _normalize_wild_encounter_state(_dictionary_value(data.get("wild_encounter_state", wild_encounter_state)))
	if current_map_constant.is_empty() or current_map_constant == map_id:
		current_map_constant = str(map_summary.get("map_constant", map_summary.get("constant", current_map_constant)))
	if current_map_group_name.is_empty():
		current_map_group_name = str(map_summary.get("group_name", map_summary.get("groupName", current_map_group_name)))
	if current_map_environment.is_empty():
		current_map_environment = str(map_summary.get("environment", current_map_environment))
	if current_map_block_key.is_empty():
		current_map_block_key = str(map_summary.get("blocks_label", current_map_block_key))
	if current_group_id < 0:
		current_group_id = int(map_summary.get("group_id", map_summary.get("groupId", current_group_id)))
	if current_map_id < 0:
		current_map_id = int(map_summary.get("map_id", map_summary.get("mapId", map_summary.get("id", current_map_id))))
	if current_phone_service <= 0:
		current_phone_service = int(map_summary.get("phone_service", map_summary.get("phoneService", current_phone_service)))
	if current_tileset_name.is_empty():
		current_tileset_name = str(map_summary.get("tileset_name", map_summary.get("tilesetName", current_tileset_name)))
	if current_location.is_empty():
		current_location = str(map_summary.get("location", current_location))
	if current_map_block_key.is_empty():
		current_map_block_key = _resolve_blocks_key()
	map_summary["map_key"] = current_map_key
	map_summary["map_name"] = current_map_name
	map_summary["map_constant"] = current_map_constant
	map_summary["width"] = current_width
	map_summary["height"] = current_height
	map_summary["group_id"] = current_group_id
	map_summary["map_id"] = current_map_id
	map_summary["group_name"] = current_map_group_name
	debug_lines = []
	for line in Array(data.get("debug_lines", [])):
		debug_lines.append(str(line))
	if debug_lines.is_empty():
		debug_lines.append("overworld ready")
	_sync_object_event_states()
	_update_state_summary_metadata()
	set_player_position(restored_player_tile.x, restored_player_tile.y)
	set_player_facing(restored_player_facing)
	_sync_scene_for_map(current_map_key)
	if current_map_key.is_empty():
		_sync_scene_for_map(selected_map_key)
	_sync_selected_map_key()
	_update_tile_animation_state()
	_update_wild_encounter_metadata()
	runtime_summary = _build_runtime_summary()

func hud_lines() -> Array[String]:
	var lines: Array[String] = []
	lines.append("map: %s (%s) key=%s" % [map_title, map_id, get_selected_map_key()])
	lines.append("scene: %s" % (scene_name if not scene_name.is_empty() else "none"))
	lines.append("selection: %d/%d available" % [max(0, selected_map_index + 1), available_map_keys.size()])
	lines.append("tile: %d,%d facing=%s" % [player_tile.x, player_tile.y, player_facing])
	lines.append("movement: state=%s locked=%s pending=%s" % [
		movement_state,
		str(movement_locked).to_lower(),
		pending_move,
	])
	lines.append("result: move=%s collision=%s warp=%s" % [
		str(last_move_result.get("state", "idle")),
		str(last_collision_result.get("reason", collision_reason)),
		str(last_warp_result.get("target", warp_target)),
	])
	lines.append("collision: %s %s" % [str(collision_detected).to_lower(), collision_reason])
	lines.append("warp: %s -> %s" % [str(warp_requested).to_lower(), warp_target])
	lines.append("dialogue: active=%s waits=%d prompt=%s" % [
		str(bool(dialogue_state.get("active", false))).to_lower(),
		int(dialogue_state.get("pending_waits", 0)),
		str(bool(dialogue_state.get("prompt_active", false))).to_lower(),
	])
	lines.append("note: %s" % last_runtime_note)
	return lines

func _push_debug() -> void:
	debug_lines.append("step %d: %s" % [fixed_step_count, str(last_move_result.get("state", movement_state))])
	while debug_lines.size() > 8:
		debug_lines.pop_front()

func _sorted_map_keys() -> Array[String]:
	var keys: Array[String] = []
	for key in map_manifest.keys():
		keys.append(str(key))
	keys.sort()
	return keys

func _sync_selected_map_key() -> void:
	var candidate := selected_map_key
	if candidate.is_empty() and not current_map_key.is_empty():
		candidate = current_map_key
	if candidate.is_empty() and selected_map_index >= 0 and selected_map_index < available_map_keys.size():
		candidate = available_map_keys[selected_map_index]
	if candidate.is_empty():
		candidate = _first_available_map_key()
	selected_map_key = _resolve_map_key(candidate)
	if selected_map_key.is_empty():
		selected_map_index = -1
		return
	selected_map_index = _index_for_map_key(selected_map_key)

func _index_for_map_key(map_key: String) -> int:
	var keys: Array[String] = available_map_keys
	if keys.is_empty():
		keys = _sorted_map_keys()
	for index in range(keys.size()):
		if keys[index] == map_key:
			return index
	return -1

func _resolve_map_key(identifier: String) -> String:
	var query := identifier.strip_edges()
	if query.is_empty():
		if not selected_map_key.is_empty() and map_manifest.has(selected_map_key):
			return selected_map_key
		if not current_map_key.is_empty() and map_manifest.has(current_map_key):
			return current_map_key
		return _first_available_map_key()
	if map_manifest.has(query):
		return query
	var query_lower := query.to_lower()
	for key in _sorted_map_keys():
		var summary: Dictionary = _dictionary_value(map_manifest.get(key, {}))
		var candidates: Array[String] = [
			str(summary.get("map_key", "")),
			str(summary.get("map_constant", "")),
			str(summary.get("constant", "")),
			str(summary.get("title", "")),
			str(summary.get("name", "")),
			str(summary.get("map_id", "")),
			str(summary.get("mapId", "")),
			str(summary.get("id", "")),
			key,
		]
		for candidate in candidates:
			if candidate.is_empty():
				continue
			if candidate == query or candidate.to_lower() == query_lower:
				return key
	return query if map_manifest.has(query) else ""

func _normalize_direction(direction: String) -> String:
	match direction.strip_edges().to_lower():
		MOVE_UP, MOVE_DOWN, MOVE_LEFT, MOVE_RIGHT:
			return direction.strip_edges().to_lower()
		_:
			return MOVE_NONE

func _vector_from_dictionary(value: Dictionary, fallback: Vector2i) -> Vector2i:
	var x := int(value.get("x", value.get("width", fallback.x)))
	var y := int(value.get("y", value.get("height", fallback.y)))
	return Vector2i(max(0, x), max(0, y))

func _vector_from_value(value: Variant, fallback: Vector2i) -> Vector2i:
	if typeof(value) == TYPE_ARRAY:
		var arr: Array = value
		if arr.size() >= 2:
			return Vector2i(int(arr[0]), int(arr[1]))
	if typeof(value) == TYPE_DICTIONARY:
		return _vector_from_dictionary(Dictionary(value), fallback)
	return fallback

func _player_start_from_spawn(spawn: Dictionary) -> Vector2i:
	if spawn.has("player_tile"):
		return _vector_from_value(spawn.get("player_tile"), Vector2i(0, 0))
	if spawn.has("tile_x") or spawn.has("tileX") or spawn.has("x") or spawn.has("y"):
		var tile_x := int(spawn.get("tile_x", spawn.get("tileX", spawn.get("x", 0))))
		var tile_y := int(spawn.get("tile_y", spawn.get("tileY", spawn.get("y", 0))))
		return Vector2i(tile_x, tile_y)
	if spawn.has("spawn"):
		var nested: Variant = spawn.get("spawn")
		if typeof(nested) == TYPE_DICTIONARY and Dictionary(nested).has("player_tile"):
			return _vector_from_value(Dictionary(nested).get("player_tile"), Vector2i(0, 0))
		if typeof(nested) == TYPE_DICTIONARY and (Dictionary(nested).has("tile_x") or Dictionary(nested).has("tileX")):
			var nested_spawn: Dictionary = Dictionary(nested)
			var nested_tile_x := int(nested_spawn.get("tile_x", nested_spawn.get("tileX", nested_spawn.get("x", 0))))
			var nested_tile_y := int(nested_spawn.get("tile_y", nested_spawn.get("tileY", nested_spawn.get("y", 0))))
			return Vector2i(nested_tile_x, nested_tile_y)
	return Vector2i(0, 0)

func _normalize_spawn_point_state(spawn_point: Dictionary, tile: Vector2i) -> Dictionary:
	var normalized := spawn_point.duplicate(true)
	normalized["player_tile"] = {"x": tile.x, "y": tile.y}
	normalized["tile_x"] = tile.x
	normalized["tileX"] = tile.x
	normalized["tile_y"] = tile.y
	normalized["tileY"] = tile.y
	return normalized

func _next_tile_for_move(direction: String) -> Vector2i:
	match direction:
		MOVE_UP:
			return Vector2i(player_tile.x, player_tile.y - 1)
		MOVE_DOWN:
			return Vector2i(player_tile.x, player_tile.y + 1)
		MOVE_LEFT:
			return Vector2i(player_tile.x - 1, player_tile.y)
		MOVE_RIGHT:
			return Vector2i(player_tile.x + 1, player_tile.y)
		_:
			return player_tile

func _is_tile_out_of_bounds(tile: Vector2i) -> bool:
	if map_dimensions.x <= 0 or map_dimensions.y <= 0:
		return false
	return tile.x < 0 or tile.y < 0 or tile.x >= map_dimensions.x or tile.y >= map_dimensions.y

func _should_request_warp(tile: Vector2i) -> bool:
	return map_dimensions.x > 0 and map_dimensions.y > 0 and tile.x == map_dimensions.x - 1 and tile.y == map_dimensions.y - 1

func _default_warp_target() -> String:
	return "%s_warp" % map_id

func _sync_scene_for_map(map_key: String) -> void:
	var normalized_map := _resolve_map_key(map_key)
	if normalized_map.is_empty():
		return
	if map_scenes.has(normalized_map):
		var stored_scene := str(map_scenes.get(normalized_map, ""))
		map_scene_indices[normalized_map] = int(map_scene_indices.get(normalized_map, 0))
		if normalized_map == current_map_key or normalized_map == get_selected_map_key():
			scene_name = stored_scene
		return
	if scene_name.is_empty():
		map_scenes[normalized_map] = ""
		map_scene_indices[normalized_map] = 0
		return
	map_scenes[normalized_map] = scene_name
	map_scene_indices[normalized_map] = int(map_scene_indices.get(normalized_map, 0))
	if normalized_map == current_map_key or normalized_map == get_selected_map_key():
		scene_name = str(map_scenes.get(normalized_map, scene_name))

func _record_field_move(move_name: String, x: int, y: int) -> void:
	special_state["last_field_move"] = {
		"move": move_name,
		"x": x,
		"y": y,
		"map_key": current_map_key,
		"scene_name": scene_name,
	}
	last_runtime_note = "field move: %s (%d,%d)" % [move_name, x, y]
	debug_lines.append(last_runtime_note)
	while debug_lines.size() > 8:
		debug_lines.pop_front()

func _advance_object_motion_counters() -> void:
	for key in object_states.keys():
		var record_variant: Variant = object_states.get(key, {})
		if typeof(record_variant) != TYPE_DICTIONARY:
			continue
		var object_id := _normalize_object_id(key)
		if object_id.is_empty() or object_id == "PLAYER":
			continue
		var record: Dictionary = Dictionary(record_variant)
		if not bool(record.get("visible", true)) or bool(record.get("removed", false)):
			continue
		record["facing"] = _normalize_direction(str(record.get("direction", record.get("facing", MOVE_DOWN))))
		record["step_animation_count"] = int(record.get("step_animation_count", 0)) + 1
		object_states[object_id] = record

func _build_object_motion_states() -> Array:
	var motions: Array = []
	for key in object_states.keys():
		var record_variant: Variant = object_states.get(key, {})
		if typeof(record_variant) != TYPE_DICTIONARY:
			continue
		var record: Dictionary = Dictionary(record_variant).duplicate(true)
		var object_id := _normalize_object_id(record.get("object_id", key))
		if object_id.is_empty():
			continue
		record["object_id"] = object_id
		if not record.has("object_index"):
			record["object_index"] = int(record.get("index", 0))
		record["facing"] = _normalize_direction(str(record.get("direction", record.get("facing", MOVE_DOWN))))
		record["step_animation_count"] = int(record.get("step_animation_count", 0))
		record["facing_update_count"] = int(record.get("facing_update_count", 0))
		record["visible"] = bool(record.get("visible", true))
		record["removed"] = bool(record.get("removed", false))
		motions.append(record)
	motions.sort_custom(func(a, b):
		var a_record: Dictionary = Dictionary(a)
		var b_record: Dictionary = Dictionary(b)
		var a_index := int(a_record.get("object_index", 0))
		var b_index := int(b_record.get("object_index", 0))
		if a_index == b_index:
			return str(a_record.get("object_id", "")) < str(b_record.get("object_id", ""))
		return a_index < b_index
	)
	return motions

func _build_trainer_sightline_payloads() -> Array:
	var payloads: Array = []
	var player_x := player_tile.x
	var player_y := player_tile.y
	for index in range(current_object_events.size()):
		var event_record := _get_object_record_from_event(index + 1)
		if event_record.is_empty():
			continue
		var object_id := _normalize_object_id(event_record.get("object_id", ""))
		if object_id.is_empty():
			continue
		var record: Dictionary = Dictionary(object_states.get(object_id, event_record)).duplicate(true)
		var event_flag := str(event_record.get("event_flag", record.get("event_flag", ""))).strip_edges()
		var direction := _normalize_direction(str(record.get("direction", event_record.get("direction", MOVE_DOWN))))
		var tile_x := int(record.get("tile_x", event_record.get("tile_x", 0)))
		var tile_y := int(record.get("tile_y", event_record.get("tile_y", 0)))
		var delta_x := player_x - tile_x
		var delta_y := player_y - tile_y
		var distance_tiles := 0
		var sightline_direction := ""
		if delta_x == 0 and delta_y != 0:
			distance_tiles = abs(delta_y)
			sightline_direction = MOVE_DOWN if delta_y > 0 else MOVE_UP
		elif delta_y == 0 and delta_x != 0:
			distance_tiles = abs(delta_x)
			sightline_direction = MOVE_RIGHT if delta_x > 0 else MOVE_LEFT
		var event_flag_set := not event_flag.is_empty() and get_event_flag(event_flag)
		payloads.append({
			"object_id": object_id,
			"object_index": int(record.get("object_index", index + 1)),
			"script": str(event_record.get("script", "")),
			"event_flag": event_flag,
			"event_flag_state": event_flag_set,
			"visible": bool(record.get("visible", event_record.get("visible", true))),
			"hidden": bool(record.get("hidden", event_record.get("hidden", false))),
			"removed": bool(record.get("removed", event_record.get("removed", false))),
			"defeated": bool(record.get("defeated", event_record.get("defeated", false))),
			"tile_x": tile_x,
			"tile_y": tile_y,
			"direction": direction,
			"player_x": player_x,
			"player_y": player_y,
			"delta_x": delta_x,
			"delta_y": delta_y,
			"distance_tiles": distance_tiles,
			"sightline_direction": sightline_direction,
			"in_sightline": distance_tiles > 0 and not sightline_direction.is_empty() and sightline_direction == direction and not event_flag_set,
			"step_animation_count": int(record.get("step_animation_count", 0)),
			"facing_update_count": int(record.get("facing_update_count", 0)),
		})
	payloads.sort_custom(func(a, b):
		var a_record: Dictionary = Dictionary(a)
		var b_record: Dictionary = Dictionary(b)
		var a_index := int(a_record.get("object_index", 0))
		var b_index := int(b_record.get("object_index", 0))
		if a_index == b_index:
			return str(a_record.get("object_id", "")) < str(b_record.get("object_id", ""))
		return a_index < b_index
		)
	return payloads

func _build_warp_transition_payloads() -> Array:
	var payloads: Array = []
	for warp_entry in current_warps:
		if typeof(warp_entry) != TYPE_DICTIONARY:
			continue
		var warp: Dictionary = Dictionary(warp_entry).duplicate(true)
		var tile_x := int(warp.get("x", warp.get("tile_x", warp.get("map_x", 0))))
		var tile_y := int(warp.get("y", warp.get("tile_y", warp.get("map_y", 0))))
		var target_map_constant := str(warp.get("target_map_constant", warp.get("targetMapConstant", current_map_constant)))
		var target_warp_id := int(warp.get("target_warp_id", warp.get("targetWarpId", 0)))
		payloads.append({
			"kind": "warp",
			"map_key": current_map_key,
			"tile_x": tile_x,
			"tile_y": tile_y,
			"warp_id": int(warp.get("warp_id", warp.get("warpId", warp.get("index", 0)))),
			"target_map_constant": target_map_constant,
			"target_warp_id": target_warp_id,
			"source_map_key": current_map_key,
			"target_transition": "%s:%d" % [target_map_constant, target_warp_id],
		})
	payloads.sort_custom(func(a, b):
		var a_record: Dictionary = Dictionary(a)
		var b_record: Dictionary = Dictionary(b)
		var a_index := int(a_record.get("warp_id", 0))
		var b_index := int(b_record.get("warp_id", 0))
		if a_index == b_index:
			return int(a_record.get("tile_y", 0)) < int(b_record.get("tile_y", 0))
		return a_index < b_index
	)
	return payloads

func _build_connection_transition_payloads() -> Array:
	var payloads: Array = []
	for connection_entry in current_connections:
		if typeof(connection_entry) != TYPE_DICTIONARY:
			continue
		var connection: Dictionary = Dictionary(connection_entry).duplicate(true)
		payloads.append({
			"kind": "connection",
			"map_key": current_map_key,
			"direction": str(connection.get("direction", connection.get("connection", connection.get("name", "")))),
			"target_map": str(connection.get("map", connection.get("target_map", connection.get("targetMap", "")))),
			"source_map_key": current_map_key,
			"source_tile": {
				"x": int(connection.get("x", connection.get("tile_x", 0))),
				"y": int(connection.get("y", connection.get("tile_y", 0))),
			},
		})
	return payloads

func _build_event_activation_records() -> Array:
	var activations: Array = []
	for index in range(current_bg_events.size()):
		var event_variant: Variant = current_bg_events[index]
		if typeof(event_variant) != TYPE_DICTIONARY:
			continue
		var event: Dictionary = Dictionary(event_variant).duplicate(true)
		var event_type := str(event.get("event_type", event.get("type", event.get("kind", "")))).strip_edges()
		var event_flag := str(event.get("event_flag", event.get("eventFlag", ""))).strip_edges()
		var is_signpost := event_type.to_lower().find("signpost") >= 0 or event_type.to_lower().find("coord") >= 0
		var gating_reason := ""
		var active := true
		if event_flag.is_empty():
			gating_reason = "no_event_flag"
		elif get_event_flag(event_flag):
			gating_reason = "event_flag_set"
			active = false
		elif not event.has("script") and not event.has("action") and not event.has("value"):
			gating_reason = "no_activation_payload"
		activations.append({
			"event_index": index + 1,
			"map_key": current_map_key,
			"event_type": event_type,
			"event_flag": event_flag,
			"active": active,
			"triggered": active and is_signpost,
			"gating_reason": gating_reason,
			"tile_x": int(event.get("x", event.get("tile_x", 0))),
			"tile_y": int(event.get("y", event.get("tile_y", 0))),
			"event": event,
		})
	return activations

func _build_object_event_gating_records() -> Array:
	var records: Array = []
	for index in range(current_object_events.size()):
		var event_record := _get_object_record_from_event(index + 1)
		if event_record.is_empty():
			continue
		var object_id := _normalize_object_id(event_record.get("object_id", ""))
		if object_id.is_empty():
			continue
		var state: Dictionary = Dictionary(object_states.get(object_id, event_record)).duplicate(true)
		var event_flag := str(event_record.get("event_flag", state.get("event_flag", ""))).strip_edges()
		var event_flag_state := not event_flag.is_empty() and get_event_flag(event_flag)
		var visible := bool(state.get("visible", not event_flag_state))
		var removed := bool(state.get("removed", event_flag_state))
		var defeated := bool(state.get("defeated", event_flag_state))
		var gating_reason := ""
		if event_flag.is_empty():
			gating_reason = "no_event_flag"
		elif event_flag_state:
			gating_reason = "event_flag_set"
		elif not visible:
			gating_reason = "hidden_state"
		elif removed:
			gating_reason = "removed_state"
		elif defeated:
			gating_reason = "defeated_state"
		records.append({
			"object_id": object_id,
			"object_index": int(state.get("object_index", index + 1)),
			"event_flag": event_flag,
			"event_flag_state": event_flag_state,
			"visible": visible,
			"hidden": bool(state.get("hidden", not visible)),
			"removed": removed,
			"defeated": defeated,
			"gating_reason": gating_reason,
			"tile_x": int(state.get("tile_x", event_record.get("tile_x", 0))),
			"tile_y": int(state.get("tile_y", event_record.get("tile_y", 0))),
			"script": str(event_record.get("script", "")),
			"event": Dictionary(event_record.get("event", {})).duplicate(true),
		})
	return records

func _resolve_wild_encounter_data() -> Dictionary:
	for key in ["wild_encounter_data", "wild_encounters", "encounter_data"]:
		var payload_variant: Variant = current_map_payload.get(key, {})
		if typeof(payload_variant) == TYPE_DICTIONARY:
			return Dictionary(payload_variant).duplicate(true)
	for key in ["wild_encounter_data", "wild_encounters", "encounter_data"]:
		var summary_variant: Variant = map_summary.get(key, {})
		if typeof(summary_variant) == TYPE_DICTIONARY:
			return Dictionary(summary_variant).duplicate(true)
	return {}

func _normalize_wild_encounter_time_of_day(value: String) -> String:
	var normalized := value.strip_edges().to_lower()
	if normalized.is_empty():
		return "day"
	if normalized in ["morning", "day", "night"]:
		return normalized
	if normalized.find("night") >= 0:
		return "night"
	if normalized.find("morning") >= 0 or normalized.find("dawn") >= 0:
		return "morning"
	if normalized.find("day") >= 0 or normalized.find("afternoon") >= 0 or normalized.find("evening") >= 0 or normalized.find("noon") >= 0:
		return "day"
	return "day"

func _normalize_wild_encounter_surface(value: String) -> String:
	var normalized := value.strip_edges().to_lower()
	if normalized in ["grass", "water"]:
		return normalized
	if normalized.find("water") >= 0:
		return "water"
	if normalized.find("grass") >= 0:
		return "grass"
	return ""

func _default_wild_encounter_surface() -> String:
	var data: Dictionary = _resolve_wild_encounter_data()
	if Dictionary(data.get("grass", {})).size() > 0:
		return "grass"
	if Dictionary(data.get("water", {})).size() > 0:
		return "water"
	return ""

func _normalize_wild_encounter_state(state: Dictionary) -> Dictionary:
	var normalized: Dictionary = state.duplicate(true)
	normalized["step_counter"] = max(0, int(normalized.get("step_counter", 0)))
	normalized["repel_steps_remaining"] = max(0, int(normalized.get("repel_steps_remaining", 0)))
	var time_of_day := _normalize_wild_encounter_time_of_day(str(normalized.get("time_of_day", map_summary.get("time_of_day", current_map_payload.get("time_of_day", "day")))))
	normalized["time_of_day"] = time_of_day
	var surface := _normalize_wild_encounter_surface(str(normalized.get("surface", "")))
	if surface.is_empty():
		surface = _default_wild_encounter_surface()
	if not surface.is_empty():
		normalized["surface"] = surface
	if normalized.has("last_roll"):
		var last_roll: Dictionary = Dictionary(normalized.get("last_roll", {})).duplicate(true)
		if not last_roll.is_empty():
			last_roll["time_of_day"] = _normalize_wild_encounter_time_of_day(str(last_roll.get("time_of_day", time_of_day)))
			var last_surface := _normalize_wild_encounter_surface(str(last_roll.get("surface", surface)))
			if not last_surface.is_empty():
				last_roll["surface"] = last_surface
			last_roll["step_counter"] = int(last_roll.get("step_counter", normalized.get("step_counter", 0)))
			last_roll["repel_steps_remaining"] = max(0, int(last_roll.get("repel_steps_remaining", normalized.get("repel_steps_remaining", 0))))
		normalized["last_roll"] = last_roll
	else:
		normalized["last_roll"] = {}
	return normalized

func _build_wild_encounter_eligibility_payloads(state_override: Dictionary = {}) -> Array:
	var data: Dictionary = _resolve_wild_encounter_data()
	if data.is_empty():
		return []
	var state: Dictionary = state_override if not state_override.is_empty() else wild_encounter_state
	state = _normalize_wild_encounter_state(state)
	var time_of_day: String = _normalize_wild_encounter_time_of_day(str(state.get("time_of_day", "day")))
	var step_counter: int = int(state.get("step_counter", 0))
	var repel_steps_remaining: int = max(0, int(state.get("repel_steps_remaining", 0)))
	var last_roll: Dictionary = Dictionary(state.get("last_roll", {})).duplicate(true)
	var payloads: Array = []
	for surface in ["grass", "water"]:
		var table_variant: Variant = data.get(surface, {})
		if typeof(table_variant) != TYPE_DICTIONARY:
			continue
		var table: Dictionary = Dictionary(table_variant)
		var entries: Array = Array(table.get(time_of_day, []))
		var base_rate: int = 0
		var eligible: bool = false
		var eligibility_reason: String = "eligible"
		if entries.is_empty():
			continue
		if surface == "grass":
			var rate_map_variant: Variant = data.get("grass_rates", {})
			if typeof(rate_map_variant) == TYPE_DICTIONARY:
				base_rate = int(Dictionary(rate_map_variant).get(time_of_day, 0))
		else:
			base_rate = int(data.get("water_rate", 0))
		eligible = base_rate > 0 and repel_steps_remaining <= 0
		if base_rate <= 0:
			eligibility_reason = "no_rate"
		elif repel_steps_remaining > 0:
			eligibility_reason = "repel_active"
		payloads.append({
			"map_key": current_map_key,
			"map_name": str(data.get("map_name", current_map_name)),
			"surface": surface,
			"time_of_day": time_of_day,
			"step_counter": step_counter,
			"repel_steps_remaining": repel_steps_remaining,
			"base_rate": base_rate,
			"table_size": entries.size(),
			"table": entries.duplicate(true),
			"eligible": eligible,
			"eligibility_reason": eligibility_reason,
			"last_roll": last_roll.duplicate(true),
		})
	return payloads

func _update_wild_encounter_metadata() -> void:
	wild_encounter_state = _normalize_wild_encounter_state(wild_encounter_state)
	var payloads: Array = _build_wild_encounter_eligibility_payloads(wild_encounter_state)
	var wild_state: Dictionary = wild_encounter_state.duplicate(true)
	map_summary["wild_encounter_state"] = wild_state.duplicate(true)
	current_map_payload["wild_encounter_state"] = wild_state.duplicate(true)
	map_summary["wild_encounter_payloads"] = payloads.duplicate(true)
	current_map_payload["wild_encounter_payloads"] = payloads.duplicate(true)
	map_summary["wild_encounter_count"] = payloads.size()
	current_map_payload["wild_encounter_count"] = payloads.size()
	map_summary["wild_encounter_step_counter"] = int(wild_state.get("step_counter", 0))
	current_map_payload["wild_encounter_step_counter"] = int(wild_state.get("step_counter", 0))
	map_summary["wild_encounter_repel_steps_remaining"] = int(wild_state.get("repel_steps_remaining", 0))
	current_map_payload["wild_encounter_repel_steps_remaining"] = int(wild_state.get("repel_steps_remaining", 0))
	map_summary["wild_encounter_time_of_day"] = str(wild_state.get("time_of_day", "day"))
	current_map_payload["wild_encounter_time_of_day"] = str(wild_state.get("time_of_day", "day"))
	map_summary["wild_encounter_last_roll"] = Dictionary(wild_state.get("last_roll", {})).duplicate(true)
	current_map_payload["wild_encounter_last_roll"] = Dictionary(wild_state.get("last_roll", {})).duplicate(true)

func _update_tile_animation_state() -> void:
	var frame_count := 4
	if int(current_map_payload.get("tile_animation_frames", 0)) > 0:
		frame_count = max(1, int(current_map_payload.get("tile_animation_frames", 4)))
	tile_animation_state = {
		"map_key": current_map_key,
		"tileset_name": current_tileset_name,
		"frame_index": posmod(fixed_step_count, frame_count),
		"frame_count": frame_count,
		"step": fixed_step_count,
	}

func _build_dialogue_state(content: Variant) -> Dictionary:
	var dialogue: Node = TEXT_BOX_SCRIPT.new()
	var base: Dictionary = {
		"active": true,
		"visible": true,
		"waiting_for_input": true,
		"input_locked": false,
		"pending_waits": 0,
		"prompt_active": false,
		"prompt_kind": "",
		"prompt_options": [],
		"yes_no_result": null,
	}
	if dialogue != null and dialogue.has_method("open_dialogue"):
		dialogue.open_dialogue(content if content != null else "")
		base.merge(Dictionary(dialogue.get_state()), true)
	else:
		var text: String = str(content)
		base["current_text"] = text
		base["visible_text"] = text
		base["display_lines"] = [text]
		base["dialogue_lines"] = [text]
		base["pages"] = [{"text": text}]
		base["dialogue_pages"] = [{"text": text}]
	return base

func _resolve_map_callbacks() -> Array:
	var payloads: Array = []
	for source in [current_map_payload, map_summary]:
		if typeof(source) != TYPE_DICTIONARY:
			continue
		var dictionary_source: Dictionary = Dictionary(source)
		for key in ["map_callbacks", "mapCallbacks", "callbacks", "callback_queue", "map_callback_queue"]:
			if not dictionary_source.has(key):
				continue
			var value: Variant = dictionary_source.get(key, [])
			if typeof(value) == TYPE_ARRAY:
				for item in Array(value):
					payloads.append(item)
			elif typeof(value) == TYPE_DICTIONARY:
				payloads.append(Dictionary(value))
	return payloads

func _normalize_map_callback_entry(entry: Variant, map_key: String) -> Dictionary:
	if typeof(entry) == TYPE_DICTIONARY:
		var callback: Dictionary = Dictionary(entry).duplicate(true)
		callback["map_key"] = str(callback.get("map_key", callback.get("map", map_key)))
		var callback_type: String = str(callback.get("callback_type", callback.get("type", callback.get("action", "")))).strip_edges()
		if callback_type.is_empty():
			callback_type = str(callback.get("kind", "")).strip_edges()
		callback["callback_type"] = callback_type
		return callback
	if typeof(entry) == TYPE_ARRAY:
		var array_entry: Array = Array(entry)
		if array_entry.size() >= 2:
			return {
				"map_key": map_key,
				"callback_type": str(array_entry[0]).strip_edges(),
				"script_name": str(array_entry[1]).strip_edges(),
			}
		if array_entry.size() == 1:
			return {
				"map_key": map_key,
				"callback_type": "",
				"script_name": str(array_entry[0]).strip_edges(),
			}
	if typeof(entry) == TYPE_STRING:
		return {
			"map_key": map_key,
			"callback_type": "",
			"script_name": str(entry).strip_edges(),
		}
	return {}

func _is_engine_flag(flag_name: String) -> bool:
	return flag_name.begins_with("ENGINE_") or flag_name.begins_with("STATUSFLAGS_")

func _normalize_object_id(object_id) -> String:
	if object_id == null:
		return ""
	if typeof(object_id) == TYPE_STRING:
		var normalized := str(object_id).strip_edges()
		if normalized.is_empty():
			return ""
		if normalized.ends_with("Script"):
			normalized = normalized.substr(0, normalized.length() - 6)
		normalized = normalized.replace(".", "_")
		var builder := ""
		var previous_was_word := false
		for index in range(normalized.length()):
			var char := normalized.substr(index, 1)
			var is_upper := char >= "A" and char <= "Z"
			var is_lower := char >= "a" and char <= "z"
			var is_digit := char >= "0" and char <= "9"
			if is_upper and previous_was_word and not builder.is_empty() and not builder.ends_with("_"):
				builder += "_"
			builder += char
			previous_was_word = is_lower or is_digit
		while builder.find("__") != -1:
			builder = builder.replace("__", "_")
		return builder.strip_edges().to_upper()
	if typeof(object_id) == TYPE_INT:
		return str(int(object_id))
	if typeof(object_id) == TYPE_DICTIONARY:
		var payload: Dictionary = Dictionary(object_id)
		if payload.has("object_id"):
			return _normalize_object_id(payload.get("object_id"))
		if payload.has("id"):
			return _normalize_object_id(payload.get("id"))
		if payload.has("name"):
			return _normalize_object_id(payload.get("name", ""))
	return str(object_id).strip_edges()

func _is_numeric_identifier(value: String) -> bool:
	return not value.is_empty() and value.is_valid_int()

func _lookup_movement_data(movement_data_label: String, parent_script: String = "") -> Array[String]:
	var normalized_label := movement_data_label.strip_edges()
	if normalized_label.is_empty():
		return []
	var candidates: Array[String] = [normalized_label]
	if normalized_label.begins_with(".") and not parent_script.strip_edges().is_empty():
		var normalized_parent := parent_script.strip_edges()
		candidates.append("%s%s" % [normalized_parent, normalized_label])
		if normalized_parent.ends_with("Script"):
			candidates.append("%s%s" % [normalized_parent.substr(0, normalized_parent.length() - 6), normalized_label])
	for candidate in candidates:
		if movement_data.has(candidate):
			return _normalize_movement_commands(movement_data.get(candidate, []))
	return []

func _normalize_movement_commands(commands) -> Array[String]:
	var normalized_commands: Array[String] = []
	if typeof(commands) == TYPE_ARRAY:
		for entry in Array(commands):
			if typeof(entry) == TYPE_STRING:
				var command := str(entry).strip_edges()
				if not command.is_empty():
					normalized_commands.append(command)
			elif typeof(entry) == TYPE_DICTIONARY:
				var payload: Dictionary = Dictionary(entry)
				var command_name := str(payload.get("command", payload.get("name", ""))).strip_edges()
				if command_name.is_empty():
					continue
				var args_value: Variant = payload.get("args", [])
				var args_parts: Array[String] = []
				if typeof(args_value) == TYPE_ARRAY:
					for arg in Array(args_value):
						var arg_text := str(arg).strip_edges()
						if not arg_text.is_empty():
							args_parts.append(arg_text)
				else:
					var arg_text := str(args_value).strip_edges()
					if not arg_text.is_empty():
						args_parts.append(arg_text)
				var combined := command_name.to_lower()
				if not args_parts.is_empty():
					combined += " %s" % " ".join(args_parts)
				normalized_commands.append(combined)
			else:
				var fallback := str(entry).strip_edges()
				if not fallback.is_empty():
					normalized_commands.append(fallback)
		return normalized_commands
	if typeof(commands) == TYPE_DICTIONARY:
		var payload: Dictionary = Dictionary(commands)
		if payload.has("commands"):
			return _normalize_movement_commands(payload.get("commands"))
		if payload.has("movement_commands"):
			return _normalize_movement_commands(payload.get("movement_commands"))
		var command_name := str(payload.get("command", payload.get("name", ""))).strip_edges()
		if not command_name.is_empty():
			var args_value: Variant = payload.get("args", [])
			var args_parts: Array[String] = []
			if typeof(args_value) == TYPE_ARRAY:
				for arg in Array(args_value):
					var arg_text := str(arg).strip_edges()
					if not arg_text.is_empty():
						args_parts.append(arg_text)
			else:
				var arg_text := str(args_value).strip_edges()
				if not arg_text.is_empty():
					args_parts.append(arg_text)
			var combined := command_name.to_lower()
			if not args_parts.is_empty():
				combined += " %s" % " ".join(args_parts)
			normalized_commands.append(combined)
		return normalized_commands
	if commands == null:
		return normalized_commands
	var command := str(commands).strip_edges()
	if not command.is_empty():
		normalized_commands.append(command)
	return normalized_commands

func _refresh_player_object() -> void:
	player_object = {
		"object_id": "PLAYER",
		"object_index": 0,
		"visible": true,
		"tile_x": player_tile.x,
		"tile_y": player_tile.y,
		"direction": player_facing,
		"facing": player_facing,
	}

func _sync_object_event_states() -> void:
	for index in range(current_object_events.size()):
		var event_record := _get_object_record_from_event(index + 1)
		if event_record.is_empty():
			continue
		var object_id := _normalize_object_id(event_record.get("object_id", ""))
		if object_id.is_empty():
			continue
		var event_flag := str(event_record.get("event_flag", "")).strip_edges()
		var flag_set := not event_flag.is_empty() and get_event_flag(event_flag)
		event_record["visible"] = not flag_set
		event_record["hidden"] = flag_set
		event_record["removed"] = flag_set
		event_record["defeated"] = flag_set
		if not object_states.has(object_id):
			object_states[object_id] = event_record.duplicate(true)
			continue
		var existing: Dictionary = Dictionary(object_states.get(object_id, {}))
		if not existing.has("object_index"):
			existing["object_index"] = int(event_record.get("object_index", index + 1))
		if not existing.has("event"):
			existing["event"] = Dictionary(event_record.get("event", {})).duplicate(true)
		if not existing.has("tile_x"):
			existing["tile_x"] = int(event_record.get("tile_x", 0))
		if not existing.has("tile_y"):
			existing["tile_y"] = int(event_record.get("tile_y", 0))
		if not existing.has("direction"):
			existing["direction"] = str(event_record.get("direction", MOVE_DOWN))
		if not event_flag.is_empty():
			existing["event_flag"] = event_flag
			existing["event_flag_state"] = flag_set
			existing["visible"] = not flag_set
			existing["hidden"] = flag_set
			existing["removed"] = flag_set
			existing["defeated"] = flag_set
		object_states[object_id] = existing
	_update_state_summary_metadata()

func _get_object_record_from_event(index: int) -> Dictionary:
	if index <= 0 or index > current_object_events.size():
		return {}
	var event_variant: Variant = current_object_events[index - 1]
	if typeof(event_variant) != TYPE_DICTIONARY:
		return {}
	var event: Dictionary = Dictionary(event_variant)
	var normalized_id := _normalize_object_id(event.get("script", event.get("label", event.get("object_identifier", ""))))
	if normalized_id.is_empty():
		normalized_id = "OBJECT_%d" % index
	var record := {
		"object_id": normalized_id,
		"object_index": index,
		"visible": true,
		"hidden": false,
		"removed": false,
		"defeated": false,
		"tile_x": int(event.get("x", 0)),
		"tile_y": int(event.get("y", 0)),
		"direction": _normalize_direction(str(event.get("facing", event.get("direction", MOVE_DOWN)))),
		"event_flag": str(event.get("event_flag", event.get("eventFlag", ""))).strip_edges(),
		"event": event.duplicate(true),
	}
	return record

func _get_object_record_by_index(index: int) -> Dictionary:
	if index <= 0:
		return {}
	for key in object_states.keys():
		var record: Dictionary = Dictionary(object_states.get(key, {}))
		if int(record.get("object_index", record.get("index", 0))) == index:
			return record.duplicate(true)
	return _get_object_record_from_event(index)

func _get_object_record_by_identifier(identifier: String) -> Dictionary:
	if identifier.is_empty():
		return {}
	for index in range(current_object_events.size()):
		var record := _get_object_record_from_event(index + 1)
		if record.is_empty():
			continue
		if _object_record_matches(record, identifier):
			return record
	return {}

func _resolve_object_index_from_events(identifier: String) -> int:
	if identifier.is_empty():
		return 0
	for index in range(current_object_events.size()):
		var record := _get_object_record_from_event(index + 1)
		if record.is_empty():
			continue
		if _object_record_matches(record, identifier):
			return int(record.get("object_index", index + 1))
	return 0

func _object_record_matches(record: Dictionary, identifier: String) -> bool:
	if record.is_empty() or identifier.is_empty():
		return false
	var candidates: Array[String] = [
		_normalize_object_id(record.get("object_id", "")),
		_normalize_object_id(record.get("name", "")),
		_normalize_object_id(record.get("label", "")),
		_normalize_object_id(record.get("sprite", "")),
		_normalize_object_id(record.get("script", "")),
		_normalize_object_id(record.get("constant_id", "")),
		_normalize_object_id(record.get("object_identifier", "")),
	]
	var index := int(record.get("object_index", record.get("index", 0)))
	if index > 0:
		candidates.append(str(index))
	for candidate in candidates:
		if not candidate.is_empty() and candidate == identifier:
			return true
	return false

func _coerce_object_record(obj) -> Dictionary:
	if obj == null:
		return {}
	if typeof(obj) == TYPE_DICTIONARY:
		var payload := Dictionary(obj).duplicate(true)
		var object_id := _normalize_object_id(payload.get("object_id", payload.get("objectId", payload.get("id", ""))))
		if object_id.is_empty() and payload.has("event"):
			var event_data: Variant = payload.get("event")
			if typeof(event_data) == TYPE_DICTIONARY:
				var event_dict: Dictionary = Dictionary(event_data)
				object_id = _normalize_object_id(event_dict.get("script", event_dict.get("label", event_dict.get("object_identifier", ""))))
		if object_id.is_empty() and bool(payload.get("is_player", false)):
			object_id = "PLAYER"
		if object_id.is_empty():
			var object_index := int(payload.get("object_index", payload.get("index", 0)))
			if object_index > 0:
				object_id = str(object_index)
		if object_id.is_empty():
			return {}
		payload["object_id"] = object_id
		if not payload.has("object_index"):
			payload["object_index"] = int(payload.get("index", 0))
		if not payload.has("visible"):
			payload["visible"] = true
		if not payload.has("tile_x") and payload.has("x"):
			payload["tile_x"] = int(payload.get("x", 0))
		if not payload.has("tile_y") and payload.has("y"):
			payload["tile_y"] = int(payload.get("y", 0))
		return payload
	var normalized_id := _normalize_object_id(obj)
	if normalized_id.is_empty():
		return {}
	if normalized_id == "PLAYER":
		return player_object.duplicate(true)
	var existing: Variant = get_object_by_id(normalized_id)
	if typeof(existing) == TYPE_DICTIONARY:
		return Dictionary(existing)
	var resolved_index := resolve_object_index(normalized_id)
	return {
		"object_id": normalized_id,
		"object_index": resolved_index,
		"visible": true,
		"tile_x": player_tile.x,
		"tile_y": player_tile.y,
		"direction": player_facing,
	}

func _store_object_record(record: Dictionary) -> void:
	if record.is_empty():
		return
	var normalized_id := _normalize_object_id(record.get("object_id", ""))
	if normalized_id.is_empty():
		return
	record["object_id"] = normalized_id
	if normalized_id == "PLAYER":
		player_object = record.duplicate(true)
		_refresh_player_object()
		return
	object_states[normalized_id] = record.duplicate(true)

func _apply_movement_commands(record: Dictionary, commands: Array[String]) -> void:
	var current_x := int(record.get("tile_x", player_tile.x))
	var current_y := int(record.get("tile_y", player_tile.y))
	var current_direction := _normalize_direction(str(record.get("direction", player_facing)))
	var animation_steps := int(record.get("step_animation_count", 0))
	var facing_updates := int(record.get("facing_update_count", 0))
	for raw_command in commands:
		var parts := str(raw_command).strip_edges().split(" ", false)
		if parts.is_empty():
			continue
		var instruction := parts[0].to_lower()
		if instruction == "step_end":
			break
		if instruction == "turn_head" and parts.size() >= 2:
			current_direction = _normalize_direction(parts[1])
			continue
		if instruction in ["step_sleep", "tree_shake", "fix_facing", "remove_fixed_facing", "set_sliding", "remove_sliding"]:
			continue
		var direction := ""
		if parts.size() >= 2:
			direction = _normalize_direction(parts[1])
		if direction.is_empty():
			continue
		match direction:
			MOVE_UP:
				current_y -= 1
			MOVE_DOWN:
				current_y += 1
			MOVE_LEFT:
				current_x -= 1
			MOVE_RIGHT:
				current_x += 1
		current_direction = direction
		animation_steps += 1
		facing_updates += 1
	record["tile_x"] = current_x
	record["tile_y"] = current_y
	record["direction"] = current_direction
	record["facing"] = current_direction
	record["step_animation_count"] = animation_steps
	record["facing_update_count"] = facing_updates
	record["visible"] = true
	if _normalize_object_id(record.get("object_id", "")) == "PLAYER" or int(record.get("object_index", 0)) == 0:
		player_tile = Vector2i(current_x, current_y)
		player_facing = current_direction
		_refresh_player_object()
	last_move_request = {
		"direction": current_direction,
		"from_tile": Dictionary(last_move_request.get("from_tile", {"x": current_x, "y": current_y})).duplicate(true),
		"to_tile": {"x": current_x, "y": current_y},
		"map_key": current_map_key,
		"movement_locked": movement_locked,
	}
	last_move_result = {
		"state": "moved",
		"blocked": false,
		"moved": true,
		"reason": "",
		"direction": current_direction,
		"to_tile": {"x": current_x, "y": current_y},
	}
	last_runtime_note = "movement applied: %s" % _normalize_object_id(record.get("object_id", ""))
	debug_lines.append(last_runtime_note)
	while debug_lines.size() > 8:
		debug_lines.pop_front()

func _set_scene_state(scene: String, index: int) -> void:
	scene_name = scene
	var normalized_map := current_map_key
	if normalized_map.is_empty():
		normalized_map = get_selected_map_key()
	if normalized_map.is_empty():
		return
	map_scenes[normalized_map] = scene
	map_scene_indices[normalized_map] = max(0, index)
