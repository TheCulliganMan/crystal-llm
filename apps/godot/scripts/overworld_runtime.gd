extends Control

const GB_FRAME_RATE := 59.7275
const FRAME_DURATION_MS := 1000.0 / GB_FRAME_RATE
const MAX_CATCH_UP_FRAMES := 5
const GAME_ACTIONS := [
	"game_up",
	"game_down",
	"game_left",
	"game_right",
	"game_a",
	"game_b",
	"game_start",
	"game_select",
]
const OVERWORLD_STATE_PATH := "res://scripts/overworld_state.gd"
const ASSET_INDEX_PATH := "res://scripts/asset_index.gd"
const RUNTIME_QUEUE_STATE_KEY := "_runtime_queue_state"
const QUEUED_SCRIPT_KEYS := ["queued_scripts", "script_queue", "queued_script_queue"]
const QUEUED_EVENT_KEYS := ["queued_events", "event_queue", "queued_event_queue"]
const MAP_CALLBACK_KEYS := ["map_callbacks", "queued_map_callbacks", "map_callback_queue"]
const OBJECT_MOVEMENT_QUEUE_KEYS := ["object_movement_queue", "queued_object_movements", "movement_queue", "object_movement_queues"]
const GB_TILE_DECODER_SCRIPT := preload("res://scripts/gb_tile_decoder.gd")
const TEXT_BOX_SCRIPT := preload("res://scripts/text_box.gd")
const GB_SCREEN_SIZE := Vector2(160.0, 144.0)
const TILE_PIXEL_SIZE := 16

var accumulator_ms := 0.0
var overworld_state = null
var asset_index = null
var dialogue = null

var _last_ui_signature := ""
var _last_map_selector_signature := ""
var _last_map_surface_signature := ""
var _updating_map_selector := false
var _title_label: Label
var _map_controls: HBoxContainer
var _prev_map_button: Button
var _map_selector: OptionButton
var _next_map_button: Button
var _reload_map_button: Button
var _map_texture_rect: TextureRect
var _map_surface_label: Label
var _map_label: Label
var _selection_label: Label
var _player_label: Label
var _movement_label: Label
var _result_label: Label
var _hooks_label: Label
var _assets_label: Label
var _runtime_label: Label
var _debug_label: Label
var _gameplay_surface: Control
var _screen_clip: Control
var _screen_backdrop: ColorRect
var _live_map_rect: TextureRect
var _live_player_rect: ColorRect
var _last_camera_origin := Vector2i.ZERO
var _last_viewport_image_size := Vector2i.ZERO

func _ready() -> void:
	_bind_labels()
	_bind_controls()
	_ensure_gameplay_surface()
	_register_input_actions()
	_ensure_runtime_objects()
	overworld_state.set_asset_index(asset_index)
	asset_index.initialize()
	overworld_state.refresh_assets()
	overworld_state.load_default_map()
	_refresh_map_selector(true)
	_refresh_ui(true)
	set_process(true)
	set_process_unhandled_input(true)

func reset() -> void:
	overworld_state.reset()
	if dialogue != null and dialogue.has_method("reset"):
		dialogue.reset()
	_refresh_map_selector(true)
	_refresh_ui(true)

func set_asset_index(index) -> void:
	overworld_state.set_asset_index(index)
	_refresh_map_selector(true)
	_refresh_ui(true)

func refresh_assets() -> void:
	overworld_state.refresh_assets()
	_refresh_map_selector(true)
	_refresh_ui(true)

func load_assets() -> void:
	overworld_state.load_assets()
	_refresh_map_selector(true)
	_refresh_ui(true)

func _ensure_runtime_objects() -> void:
	if overworld_state == null:
		var state_script: Script = load(OVERWORLD_STATE_PATH)
		overworld_state = state_script.new() if state_script != null else null
	if asset_index == null:
		var asset_script: Script = load(ASSET_INDEX_PATH)
		asset_index = asset_script.new() if asset_script != null else null
	if dialogue == null:
		dialogue = TEXT_BOX_SCRIPT.new()

func _bind_labels() -> void:
	var legacy_margin := get_node_or_null("Margin")
	if legacy_margin is CanvasItem:
		legacy_margin.visible = false
	_title_label = get_node_or_null("Margin/VBox/TitleLabel")
	_map_controls = get_node_or_null("Margin/VBox/MapControls")
	_prev_map_button = get_node_or_null("Margin/VBox/MapControls/PrevMapButton")
	_map_selector = get_node_or_null("Margin/VBox/MapControls/MapSelector")
	_next_map_button = get_node_or_null("Margin/VBox/MapControls/NextMapButton")
	_reload_map_button = get_node_or_null("Margin/VBox/MapControls/ReloadMapButton")
	_map_texture_rect = get_node_or_null("Margin/VBox/MapSurfacePanel/MapSurfaceMargin/MapSurfaceVBox/MapTexture")
	_map_surface_label = get_node_or_null("Margin/VBox/MapSurfacePanel/MapSurfaceMargin/MapSurfaceVBox/MapSurfaceLabel")
	_map_label = get_node_or_null("Margin/VBox/StatusPanel/Margin/StatusVBox/MapLabel")
	_selection_label = get_node_or_null("Margin/VBox/StatusPanel/Margin/StatusVBox/SelectionLabel")
	_player_label = get_node_or_null("Margin/VBox/StatusPanel/Margin/StatusVBox/PlayerLabel")
	_movement_label = get_node_or_null("Margin/VBox/StatusPanel/Margin/StatusVBox/MovementLabel")
	_result_label = get_node_or_null("Margin/VBox/StatusPanel/Margin/StatusVBox/ResultLabel")
	_hooks_label = get_node_or_null("Margin/VBox/StatusPanel/Margin/StatusVBox/HooksLabel")
	_assets_label = get_node_or_null("Margin/VBox/StatusPanel/Margin/StatusVBox/AssetsLabel")
	_runtime_label = get_node_or_null("Margin/VBox/StatusPanel/Margin/StatusVBox/RuntimeLabel")
	_debug_label = get_node_or_null("Margin/VBox/StatusPanel/Margin/StatusVBox/DebugLabel")

func _bind_controls() -> void:
	if is_instance_valid(_map_selector) and not _map_selector.item_selected.is_connected(_on_map_selector_item_selected):
		_map_selector.item_selected.connect(_on_map_selector_item_selected)
	if is_instance_valid(_prev_map_button) and not _prev_map_button.pressed.is_connected(_on_prev_map_pressed):
		_prev_map_button.pressed.connect(_on_prev_map_pressed)
	if is_instance_valid(_next_map_button) and not _next_map_button.pressed.is_connected(_on_next_map_pressed):
		_next_map_button.pressed.connect(_on_next_map_pressed)
	if is_instance_valid(_reload_map_button) and not _reload_map_button.pressed.is_connected(_on_reload_map_pressed):
		_reload_map_button.pressed.connect(_on_reload_map_pressed)

func _register_input_actions() -> void:
	_ensure_action("game_up", [KEY_UP, KEY_W])
	_ensure_action("game_down", [KEY_DOWN, KEY_S])
	_ensure_action("game_left", [KEY_LEFT, KEY_A])
	_ensure_action("game_right", [KEY_RIGHT, KEY_D])
	_ensure_action("game_a", [KEY_Z, KEY_SPACE])
	_ensure_action("game_b", [KEY_X, KEY_ESCAPE])
	_ensure_action("game_start", [KEY_ENTER, KEY_KP_ENTER])
	_ensure_action("game_select", [KEY_TAB])

func _ensure_action(name: String, keys: Array) -> void:
	if not InputMap.has_action(name):
		InputMap.add_action(name)
	else:
		InputMap.action_erase_events(name)
	for key in keys:
		var event := InputEventKey.new()
		event.keycode = key
		event.physical_keycode = key
		InputMap.action_add_event(name, event)

func _process(delta: float) -> void:
	accumulator_ms = min(accumulator_ms + (delta * 1000.0), FRAME_DURATION_MS * MAX_CATCH_UP_FRAMES)
	var steps: int = 0
	while accumulator_ms >= FRAME_DURATION_MS and steps < MAX_CATCH_UP_FRAMES:
		accumulator_ms -= FRAME_DURATION_MS
		_step_simulation()
		steps += 1
	_refresh_ui()

func _notification(what: int) -> void:
	if what == NOTIFICATION_RESIZED:
		_layout_gameplay_surface()

func _step_simulation() -> void:
	_process_runtime_queue_step()
	overworld_state.tick()
	_handle_movement_input()
	_handle_debug_input()

func _handle_movement_input() -> void:
	if Input.is_action_just_pressed("game_up"):
		overworld_state.request_move("up")
	elif Input.is_action_just_pressed("game_down"):
		overworld_state.request_move("down")
	elif Input.is_action_just_pressed("game_left"):
		overworld_state.request_move("left")
	elif Input.is_action_just_pressed("game_right"):
		overworld_state.request_move("right")

func _handle_debug_input() -> void:
	if Input.is_action_just_pressed("game_start"):
		overworld_state.refresh_assets()
		overworld_state.load_default_map()
		_refresh_map_selector(true)
	if Input.is_action_just_pressed("game_select"):
		overworld_state.request_interaction("debug")

func _unhandled_input(event: InputEvent) -> void:
	if event is InputEventKey and event.echo:
		return
	if event.is_action_pressed("game_a"):
		overworld_state.request_interaction("confirm")
	elif event.is_action_pressed("game_b"):
		overworld_state.request_interaction("cancel")

func set_map_key(map_key: String) -> bool:
	return select_map_key(map_key)

func select_map_key(map_key: String) -> bool:
	var loaded: bool = overworld_state.set_map_key(map_key)
	_refresh_map_selector(true)
	_refresh_ui(true)
	return loaded

func load_map(map_key: String = "") -> bool:
	var loaded: bool = overworld_state.load_map(map_key)
	_refresh_map_selector(true)
	_refresh_ui(true)
	return loaded

func load_default_map() -> bool:
	var loaded: bool = overworld_state.load_default_map()
	_refresh_map_selector(true)
	_refresh_ui(true)
	return loaded

func load_map_by_index(index: int) -> bool:
	var loaded: bool = overworld_state.load_map_by_index(index)
	_refresh_map_selector(true)
	_refresh_ui(true)
	return loaded

func cycle_map(offset: int = 1) -> bool:
	var loaded: bool = overworld_state.cycle_map(offset)
	_refresh_map_selector(true)
	_refresh_ui(true)
	return loaded

func reload_current_map() -> bool:
	var loaded: bool = overworld_state.reload_current_map()
	_refresh_map_selector(true)
	_refresh_ui(true)
	return loaded

func set_map_scene(map_key: String, scene: String) -> void:
	overworld_state.set_map_scene(map_key, scene)
	_refresh_ui(true)

func check_scene(map_key: String = "") -> int:
	var index: int = overworld_state.check_scene(map_key)
	_refresh_ui(true)
	return index

func queue_script(script_entry) -> void:
	if overworld_state == null:
		_ensure_runtime_objects()
	if overworld_state == null:
		return
	overworld_state.queue_script(script_entry)
	_refresh_ui(true)

func queueScript(script_entry) -> void:
	queue_script(script_entry)

func queue_event(event_entry) -> void:
	if overworld_state == null:
		_ensure_runtime_objects()
	if overworld_state == null:
		return
	overworld_state.queue_event(event_entry)
	_refresh_ui(true)

func queueEvent(event_entry) -> void:
	queue_event(event_entry)

func queue_map_callback(callback_entry) -> void:
	if overworld_state == null:
		_ensure_runtime_objects()
	if overworld_state == null:
		return
	overworld_state.queue_map_callback(callback_entry)
	_refresh_ui(true)

func queueMapCallback(callback_entry) -> void:
	queue_map_callback(callback_entry)

func open_text(content: Variant) -> void:
	if dialogue == null:
		_ensure_runtime_objects()
	if dialogue != null and dialogue.has_method("open_dialogue"):
		dialogue.open_dialogue(content)
	overworld_state.open_text(content)
	_sync_dialogue_state()
	_refresh_ui(true)

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
	if dialogue != null and dialogue.has_method("close_dialogue"):
		dialogue.close_dialogue()
	overworld_state.close_text()
	_sync_dialogue_state()
	_refresh_ui(true)

func closeText() -> void:
	close_text()

func close_dialogue() -> void:
	close_text()

func closeDialogue() -> void:
	close_text()

func wait_for_input() -> void:
	if dialogue == null:
		_ensure_runtime_objects()
	if dialogue != null and dialogue.has_method("set_input_locked"):
		dialogue.set_input_locked(false)
	overworld_state.wait_for_input()
	_sync_dialogue_state()
	_refresh_ui(true)

func waitForInput() -> void:
	wait_for_input()

func acknowledge_wait() -> void:
	overworld_state.acknowledge_wait()
	_sync_dialogue_state()
	_refresh_ui(true)

func acknowledgeWait() -> void:
	acknowledge_wait()

func prompt_yes_no(prompt: Variant = null) -> void:
	if dialogue == null:
		_ensure_runtime_objects()
	if prompt != null:
		open_text(prompt)
	overworld_state.prompt_yes_no(prompt)
	_sync_dialogue_state()
	_refresh_ui(true)

func promptYesNo(prompt: Variant = null) -> void:
	prompt_yes_no(prompt)

func set_yes_no_result(value: bool) -> void:
	overworld_state.set_yes_no_result(value)
	_sync_dialogue_state()
	_refresh_ui(true)

func setYesNoResult(value: bool) -> void:
	set_yes_no_result(value)

func has_dialogue() -> bool:
	return dialogue != null and dialogue.has_method("is_active") and bool(dialogue.is_active())

func hasDialogue() -> bool:
	return has_dialogue()

func queue_object_movement(object_id, movement_commands, options: Dictionary = {}) -> void:
	if overworld_state == null:
		_ensure_runtime_objects()
	if overworld_state == null:
		return
	overworld_state.queue_object_movement(object_id, movement_commands, options)
	_refresh_ui(true)

func queueObjectMovement(object_id, movement_commands, options: Dictionary = {}) -> void:
	queue_object_movement(object_id, movement_commands, options)

func start_following(follower, leader, options: Dictionary = {}) -> void:
	overworld_state.start_following(follower, leader, options)
	_refresh_ui(true)

func startFollowing(follower, leader, options: Dictionary = {}) -> void:
	start_following(follower, leader, options)

func stop_following() -> void:
	overworld_state.stop_following()
	_refresh_ui(true)

func stopFollowing() -> void:
	stop_following()

func lock_player_movement() -> void:
	overworld_state.lock_player_movement()
	_refresh_ui(true)

func lockPlayerMovement() -> void:
	lock_player_movement()

func unlock_player_movement() -> void:
	overworld_state.unlock_player_movement()
	_refresh_ui(true)

func unlockPlayerMovement() -> void:
	unlock_player_movement()

func lock_all_movement() -> void:
	overworld_state.lock_all_movement()
	_refresh_ui(true)

func lockAllMovement() -> void:
	lock_all_movement()

func unlock_all_movement() -> void:
	overworld_state.unlock_all_movement()
	_refresh_ui(true)

func unlockAllMovement() -> void:
	unlock_all_movement()

func player_movement_locked() -> bool:
	return overworld_state.player_movement_locked()

func playerMovementLocked() -> bool:
	return player_movement_locked()

func get_object_by_id(object_id) -> Variant:
	return overworld_state.get_object_by_id(object_id)

func getObjectById(object_id) -> Variant:
	return get_object_by_id(object_id)

func resolve_object_index(identifier: String) -> int:
	return overworld_state.resolve_object_index(identifier)

func resolveObjectIndex(identifier: String) -> int:
	return resolve_object_index(identifier)

func get_movement_data(movement_data_label: String, parent_script: String = "") -> Array[String]:
	return overworld_state.get_movement_data(movement_data_label, parent_script)

func getMovementData(movement_data_label: String, parent_script: String = "") -> Array[String]:
	return get_movement_data(movement_data_label, parent_script)

func queue_movement_task(obj, movement_commands, options: Dictionary = {}) -> void:
	overworld_state.queue_movement_task(obj, movement_commands, options)
	_refresh_ui(true)

func queueMovementTask(obj, movement_commands, options: Dictionary = {}) -> void:
	queue_movement_task(obj, movement_commands, options)

func queue_movement(obj, movement_commands, options: Dictionary = {}) -> void:
	queue_movement_task(obj, movement_commands, options)

func queueMovement(obj, movement_commands, options: Dictionary = {}) -> void:
	queue_movement_task(obj, movement_commands, options)

func queue_follow_task(follower, leader, options: Dictionary = {}) -> void:
	overworld_state.queue_follow_task(follower, leader, options)
	_refresh_ui(true)

func queueFollowTask(follower, leader, options: Dictionary = {}) -> void:
	queue_follow_task(follower, leader, options)

func queue_follow(follower, leader, options: Dictionary = {}) -> void:
	queue_follow_task(follower, leader, options)

func queueFollow(follower, leader, options: Dictionary = {}) -> void:
	queue_follow_task(follower, leader, options)

func get_event_flag_for_object_index(index: int) -> String:
	return overworld_state.get_event_flag_for_object_index(index)

func getEventFlagForObjectIndex(index: int) -> String:
	return get_event_flag_for_object_index(index)

func get_dialogue_state() -> Dictionary:
	if dialogue != null and dialogue.has_method("get_state"):
		return _merge_dialogue_state(Dictionary(dialogue.get_state()))
	return Dictionary(overworld_state.dialogue_state).duplicate(true)

func getDialogueState() -> Dictionary:
	return get_dialogue_state()

func get_event_flag(flag_name: String) -> bool:
	return overworld_state.get_event_flag(flag_name)

func getEventFlag(flag_name: String) -> bool:
	return get_event_flag(flag_name)

func set_event_flag(flag_name: String, value: bool) -> void:
	overworld_state.set_event_flag(flag_name, value)
	_refresh_ui(true)

func setEventFlag(flag_name: String, value: bool) -> void:
	set_event_flag(flag_name, value)

func clear_event_flag(flag_name: String) -> void:
	overworld_state.clear_event_flag(flag_name)
	_refresh_ui(true)

func clearEventFlag(flag_name: String) -> void:
	clear_event_flag(flag_name)

func set_engine_flag(flag_name: String, value: bool) -> void:
	overworld_state.set_engine_flag(flag_name, value)
	_refresh_ui(true)

func setEngineFlag(flag_name: String, value: bool) -> void:
	set_engine_flag(flag_name, value)

func refresh_event_flag(event_name: String, options: Dictionary = {}) -> void:
	overworld_state.refresh_event_flag(event_name, options)
	_refresh_ui(true)

func refreshEventFlag(event_name: String, options: Dictionary = {}) -> void:
	refresh_event_flag(event_name, options)

func _write_metatile(metatile_x: int, metatile_y: int, block_id: int) -> void:
	overworld_state._write_metatile(metatile_x, metatile_y, block_id)
	_refresh_map_surface(true)
	_refresh_ui(true)

func write_metatile(metatile_x: int, metatile_y: int, block_id: int) -> void:
	_write_metatile(metatile_x, metatile_y, block_id)

func refresh_warp_permissions() -> void:
	overworld_state.refresh_warp_permissions()
	_refresh_ui(true)

func refreshWarpPermissions() -> void:
	refresh_warp_permissions()

func _refresh_warp_permissions() -> void:
	refresh_warp_permissions()

func run_map_callbacks(map_key: String = "", callback_type: String = "") -> Array:
	var executed: Array = overworld_state.run_map_callbacks(map_key, callback_type)
	_refresh_ui(true)
	return executed

func runMapCallbacks(map_key: String = "", callback_type: String = "") -> Array:
	return run_map_callbacks(map_key, callback_type)

func stop_player_movement() -> void:
	overworld_state.stop_player_movement()
	_refresh_ui(true)

func stopPlayerMovement() -> void:
	stop_player_movement()

func appear_object(object_id, options: Dictionary = {}) -> void:
	overworld_state.appear_object(object_id, options)
	_refresh_ui(true)

func appearObject(object_id, options: Dictionary = {}) -> void:
	appear_object(object_id, options)

func remove_object(object_id, options: Dictionary = {}) -> void:
	overworld_state.remove_object(object_id, options)
	_refresh_ui(true)

func removeObject(object_id, options: Dictionary = {}) -> void:
	remove_object(object_id, options)

func move_object(object_id, map_x, map_y) -> void:
	overworld_state.move_object(object_id, int(map_x), int(map_y))
	_refresh_ui(true)

func moveObject(object_id, map_x, map_y) -> void:
	move_object(object_id, map_x, map_y)

func show_emote(emote_id: String, obj, duration: int) -> void:
	overworld_state.show_emote(emote_id, obj, duration)
	_refresh_ui(true)

func showEmote(emote_id: String, obj, duration: int) -> void:
	show_emote(emote_id, obj, duration)

func wait_sfx(callback: Callable = Callable()) -> void:
	overworld_state.wait_sfx(callback)
	_refresh_ui(true)

func waitSFX(callback: Callable = Callable()) -> void:
	wait_sfx(callback)

func check_for_warp_event(options: Dictionary = {}) -> bool:
	var warped: bool = overworld_state.check_for_warp_event(options)
	_refresh_ui(true)
	return warped

func checkForWarpEvent(options: Dictionary = {}) -> bool:
	return check_for_warp_event(options)

func handle_cut(x: int, y: int) -> void:
	overworld_state.handle_cut(x, y)
	_refresh_ui(true)

func handle_surf(x: int, y: int) -> void:
	overworld_state.handle_surf(x, y)
	_refresh_ui(true)

func _handle_hm(move_name: String, x: int, y: int, player_state) -> void:
	overworld_state._handle_hm(move_name, x, y, player_state)
	_refresh_ui(true)

func handle_flash() -> void:
	overworld_state.handle_flash()
	_refresh_ui(true)

func handle_fly(x: int, y: int) -> void:
	overworld_state.handle_fly(x, y)
	_refresh_ui(true)

func request_music(music_id: String, role: String = "") -> void:
	overworld_state.request_music(music_id, role)
	_refresh_ui(true)

func requestMusic(music_id: String, role: String = "") -> void:
	request_music(music_id, role)

func fade_to_music(music_id: String, speed_frames: int, role: String = "") -> void:
	overworld_state.fade_to_music(music_id, speed_frames, role)
	_refresh_ui(true)

func fadeToMusic(music_id: String, speed_frames: int, role: String = "") -> void:
	fade_to_music(music_id, speed_frames, role)

func execute_special(function_name: String, context: Dictionary = {}) -> Variant:
	var result: Variant = overworld_state.execute_special(function_name, context)
	_refresh_ui(true)
	return result

func executeSpecial(function_name: String, context: Dictionary = {}) -> Variant:
	return execute_special(function_name, context)

func handle_special(function_name: String, context: Dictionary = {}) -> Variant:
	return execute_special(function_name, context)

func handleSpecial(function_name: String, context: Dictionary = {}) -> Variant:
	return execute_special(function_name, context)

func set_map_data(data: Dictionary) -> void:
	overworld_state.set_map_data(data)
	_refresh_map_selector(true)
	_refresh_ui(true)

func set_map(summary: Dictionary, spawn: Dictionary) -> void:
	overworld_state.set_map(summary, spawn)
	_refresh_map_selector(true)
	_refresh_ui(true)

func set_player_position(x: int, y: int) -> void:
	overworld_state.set_player_position(x, y)
	_refresh_ui(true)

func set_player_facing(direction: String) -> void:
	overworld_state.set_player_facing(direction)
	_refresh_ui(true)

func set_movement_locked(locked: bool, reason: String = "") -> void:
	overworld_state.set_movement_locked(locked, reason)
	_refresh_ui(true)

func request_move(direction: String) -> void:
	overworld_state.request_move(direction)
	_refresh_ui(true)

func tick_frame() -> void:
	overworld_state.tick_frame()
	_refresh_ui(true)

func tick() -> void:
	overworld_state.tick()
	_refresh_ui(true)

func set_collision_hook(hook: Callable) -> void:
	overworld_state.set_collision_hook(hook)
	_refresh_ui(true)

func register_collision_hook(hook: Callable) -> void:
	overworld_state.set_collision_hook(hook)

func set_warp_hook(hook: Callable) -> void:
	overworld_state.set_warp_hook(hook)
	_refresh_ui(true)

func register_warp_hook(hook: Callable) -> void:
	overworld_state.set_warp_hook(hook)

func set_warp_target(target: String) -> void:
	overworld_state.set_warp_target(target)
	_refresh_ui(true)

func request_interaction(action: String = "confirm") -> void:
	overworld_state.request_interaction(action)
	_refresh_ui(true)

func get_available_map_keys() -> Array[String]:
	return overworld_state.get_available_map_keys()

func get_selected_map_key() -> String:
	return overworld_state.get_selected_map_key()

func get_selected_map_index() -> int:
	return overworld_state.get_selected_map_index()

func get_player_tile() -> Vector2i:
	return overworld_state.player_tile

func get_player_facing() -> String:
	return overworld_state.player_facing

func get_movement_state() -> String:
	return overworld_state.movement_state

func is_movement_locked() -> bool:
	return overworld_state.movement_locked

func is_collision_detected() -> bool:
	return overworld_state.collision_detected

func get_collision_reason() -> String:
	return overworld_state.collision_reason

func is_warp_requested() -> bool:
	return overworld_state.warp_requested

func get_warp_target() -> String:
	return overworld_state.warp_target

func get_last_move_request() -> Dictionary:
	return Dictionary(overworld_state.last_move_request).duplicate(true)

func get_last_move_result() -> Dictionary:
	return Dictionary(overworld_state.last_move_result).duplicate(true)

func get_last_collision_result() -> Dictionary:
	return Dictionary(overworld_state.last_collision_result).duplicate(true)

func get_last_warp_result() -> Dictionary:
	return Dictionary(overworld_state.last_warp_result).duplicate(true)

func get_last_runtime_note() -> String:
	return overworld_state.last_runtime_note

func get_current_spawn_point() -> Dictionary:
	return Dictionary(overworld_state.current_spawn_point).duplicate(true)

func get_asset_summary() -> Dictionary:
	return Dictionary(overworld_state.asset_summary).duplicate(true)

func get_runtime_summary() -> Dictionary:
	return Dictionary(overworld_state.runtime_summary).duplicate(true)

func get_map_summary() -> Dictionary:
	return Dictionary(overworld_state.map_summary).duplicate(true)

func get_spawn_summary() -> Dictionary:
	return Dictionary(overworld_state.spawn_summary).duplicate(true)

func get_map_manifest() -> Dictionary:
	return Dictionary(overworld_state.map_manifest).duplicate(true)

func get_pending_move() -> String:
	return overworld_state.pending_move

func get_scene_name() -> String:
	return overworld_state.scene_name

func get_map_scenes() -> Dictionary:
	return Dictionary(overworld_state.map_scenes).duplicate(true)

func get_map_scene_indices() -> Dictionary:
	return Dictionary(overworld_state.map_scene_indices).duplicate(true)

func get_follow_state() -> Dictionary:
	return Dictionary(overworld_state.follow_state).duplicate(true)

func get_object_states() -> Dictionary:
	return Dictionary(overworld_state.object_states).duplicate(true)

func get_object_motion_states() -> Array:
	if overworld_state == null:
		return []
	if overworld_state.has_method("get_object_motion_states"):
		return Array(overworld_state.call("get_object_motion_states")).duplicate(true)
	return []

func getObjectMotionStates() -> Array:
	return get_object_motion_states()

func get_trainer_sightline_payloads() -> Array:
	if overworld_state == null:
		return []
	if overworld_state.has_method("get_trainer_sightline_payloads"):
		return Array(overworld_state.call("get_trainer_sightline_payloads")).duplicate(true)
	return []

func getTrainerSightlinePayloads() -> Array:
	return get_trainer_sightline_payloads()

func get_field_move_state() -> Dictionary:
	if overworld_state == null:
		return {}
	if overworld_state.has_method("get_field_move_state"):
		return Dictionary(overworld_state.call("get_field_move_state")).duplicate(true)
	return Dictionary(overworld_state.special_state.get("last_field_move", {})).duplicate(true)

func getFieldMoveState() -> Dictionary:
	return get_field_move_state()

func get_render_object_states() -> Array:
	if overworld_state == null:
		return []
	if overworld_state.has_method("get_render_object_states"):
		return Array(overworld_state.call("get_render_object_states")).duplicate(true)
	return []

func get_render_object_payloads() -> Array:
	return get_render_object_states()

func get_map_callback_queue_state() -> Array:
	if overworld_state == null:
		return []
	if overworld_state.has_method("get_map_callback_queue_state"):
		return Array(overworld_state.call("get_map_callback_queue_state")).duplicate(true)
	return []

func getMapCallbackQueueState() -> Array:
	return get_map_callback_queue_state()

func get_warp_transition_payloads() -> Array:
	if overworld_state == null:
		return []
	if overworld_state.has_method("get_warp_transition_payloads"):
		return Array(overworld_state.call("get_warp_transition_payloads")).duplicate(true)
	return []

func getWarpTransitionPayloads() -> Array:
	return get_warp_transition_payloads()

func get_connection_transition_payloads() -> Array:
	if overworld_state == null:
		return []
	if overworld_state.has_method("get_connection_transition_payloads"):
		return Array(overworld_state.call("get_connection_transition_payloads")).duplicate(true)
	return []

func getConnectionTransitionPayloads() -> Array:
	return get_connection_transition_payloads()

func get_event_activation_records() -> Array:
	if overworld_state == null:
		return []
	if overworld_state.has_method("get_event_activation_records"):
		return Array(overworld_state.call("get_event_activation_records")).duplicate(true)
	return []

func getEventActivationRecords() -> Array:
	return get_event_activation_records()

func get_object_event_gating_records() -> Array:
	if overworld_state == null:
		return []
	if overworld_state.has_method("get_object_event_gating_records"):
		return Array(overworld_state.call("get_object_event_gating_records")).duplicate(true)
	return []

func getObjectEventGatingRecords() -> Array:
	return get_object_event_gating_records()

func get_tile_animation_state() -> Dictionary:
	if overworld_state == null:
		return {}
	if overworld_state.has_method("get_tile_animation_state"):
		return Dictionary(overworld_state.call("get_tile_animation_state")).duplicate(true)
	return Dictionary(overworld_state.tile_animation_state).duplicate(true)

func getTileAnimationState() -> Dictionary:
	return get_tile_animation_state()

func get_wild_encounter_state() -> Dictionary:
	if overworld_state == null:
		return {}
	if overworld_state.has_method("get_wild_encounter_state"):
		return Dictionary(overworld_state.call("get_wild_encounter_state")).duplicate(true)
	return Dictionary(overworld_state.wild_encounter_state).duplicate(true)

func getWildEncounterState() -> Dictionary:
	return get_wild_encounter_state()

func get_wild_encounter_eligibility_payloads() -> Array:
	if overworld_state == null:
		return []
	if overworld_state.has_method("get_wild_encounter_eligibility_payloads"):
		return Array(overworld_state.call("get_wild_encounter_eligibility_payloads")).duplicate(true)
	return []

func getWildEncounterEligibilityPayloads() -> Array:
	return get_wild_encounter_eligibility_payloads()

func get_last_wild_encounter_roll() -> Dictionary:
	if overworld_state == null:
		return {}
	if overworld_state.has_method("get_last_wild_encounter_roll"):
		return Dictionary(overworld_state.call("get_last_wild_encounter_roll")).duplicate(true)
	return Dictionary(overworld_state.wild_encounter_state.get("last_roll", {})).duplicate(true)

func getLastWildEncounterRoll() -> Dictionary:
	return get_last_wild_encounter_roll()

func set_wild_encounter_state(state: Dictionary) -> void:
	if overworld_state == null:
		return
	if overworld_state.has_method("set_wild_encounter_state"):
		overworld_state.call("set_wild_encounter_state", state)

func setWildEncounterState(state: Dictionary) -> void:
	set_wild_encounter_state(state)

func advance_wild_encounter_step(surface: String = "") -> void:
	if overworld_state == null:
		return
	if overworld_state.has_method("advance_wild_encounter_step"):
		overworld_state.call("advance_wild_encounter_step", surface)

func advanceWildEncounterStep(surface: String = "") -> void:
	advance_wild_encounter_step(surface)

func record_wild_encounter_roll(record: Dictionary) -> void:
	if overworld_state == null:
		return
	if overworld_state.has_method("record_wild_encounter_roll"):
		overworld_state.call("record_wild_encounter_roll", record)

func recordWildEncounterRoll(record: Dictionary) -> void:
	record_wild_encounter_roll(record)

func get_special_state() -> Dictionary:
	return Dictionary(overworld_state.special_state).duplicate(true)

func get_reload_map_after_battle() -> bool:
	return bool(overworld_state.reload_map_after_battle)

func get_state() -> Dictionary:
	if overworld_state == null:
		return {}
	return overworld_state.to_dictionary() if overworld_state.has_method("to_dictionary") else overworld_state.get_state()

func from_state(data: Dictionary) -> void:
	_ensure_runtime_objects()
	if overworld_state == null:
		return
	if overworld_state.has_method("from_dictionary"):
		overworld_state.from_dictionary(data)
	else:
		overworld_state.from_state(data)
	if dialogue != null and dialogue.has_method("from_dictionary"):
		dialogue.from_dictionary(Dictionary(overworld_state.dialogue_state))
	_sync_dialogue_state()
	_ingest_runtime_queue_fields(data)
	_refresh_map_selector(true)
	_refresh_ui(true)

func to_dictionary() -> Dictionary:
	return get_state()

func from_dictionary(data: Variant) -> bool:
	if typeof(data) != TYPE_DICTIONARY:
		return false
	from_state(Dictionary(data))
	return true

func get_runtime_queue_state() -> Dictionary:
	return _runtime_queue_state().duplicate(true)

func get_script_queue_state() -> Dictionary:
	return get_runtime_queue_state()

func getScriptQueueState() -> Dictionary:
	return get_script_queue_state()

func _process_runtime_queue_step() -> void:
	if overworld_state == null:
		return
	var queues: Dictionary = _runtime_queue_state()
	var processed: bool = false
	for queue_name in ["map_callbacks", "queued_scripts", "queued_events", "object_movement_queue"]:
		var entry: Variant = _pop_runtime_queue_entry(queues, queue_name)
		if entry == null:
			continue
		processed = _process_runtime_queue_entry(queue_name, entry)
		_record_runtime_queue_result(queues, queue_name, entry, processed)
		break
	_store_runtime_queue_state(queues)

func _runtime_queue_state() -> Dictionary:
	var special: Dictionary = Dictionary(overworld_state.special_state)
	var queues: Dictionary = {}
	var stored: Variant = special.get(RUNTIME_QUEUE_STATE_KEY, {})
	if typeof(stored) == TYPE_DICTIONARY:
		queues = Dictionary(stored).duplicate(true)
	_ensure_runtime_queue_arrays(queues)
	_migrate_queue_aliases_from_dictionary(special, queues, true)
	_ingest_object_state_movement_queues(queues)
	_ingest_map_callback_fields(queues)
	special[RUNTIME_QUEUE_STATE_KEY] = queues
	overworld_state.special_state = special
	return queues

func _store_runtime_queue_state(queues: Dictionary) -> void:
	_ensure_runtime_queue_arrays(queues)
	var special: Dictionary = Dictionary(overworld_state.special_state)
	special[RUNTIME_QUEUE_STATE_KEY] = queues.duplicate(true)
	overworld_state.special_state = special

func _enqueue_runtime_entry(queue_name: String, entry: Variant) -> void:
	if overworld_state == null:
		_ensure_runtime_objects()
	if overworld_state == null:
		return
	var queues: Dictionary = _runtime_queue_state()
	_append_runtime_queue_values(queues, queue_name, entry, "")
	_store_runtime_queue_state(queues)

func _ensure_runtime_queue_arrays(queues: Dictionary) -> void:
	for key in ["queued_scripts", "queued_events", "map_callbacks", "object_movement_queue", "completed"]:
		if typeof(queues.get(key, [])) != TYPE_ARRAY:
			queues[key] = []

func _ingest_runtime_queue_fields(payload: Dictionary) -> void:
	if overworld_state == null:
		return
	var queues: Dictionary = _runtime_queue_state()
	_migrate_queue_aliases_from_dictionary(payload, queues, false)
	var special_payload: Variant = payload.get("special_state", {})
	if typeof(special_payload) == TYPE_DICTIONARY:
		_migrate_queue_aliases_from_dictionary(Dictionary(special_payload), queues, false)
	_store_runtime_queue_state(queues)

func _migrate_queue_aliases_from_dictionary(source: Dictionary, queues: Dictionary, remove_source_keys: bool) -> void:
	for key in MAP_CALLBACK_KEYS:
		if source.has(key):
			_append_runtime_queue_values(queues, "map_callbacks", source.get(key), "")
			if remove_source_keys:
				source.erase(key)
	for key in QUEUED_SCRIPT_KEYS:
		if source.has(key):
			_append_runtime_queue_values(queues, "queued_scripts", source.get(key), "")
			if remove_source_keys:
				source.erase(key)
	for key in QUEUED_EVENT_KEYS:
		if source.has(key):
			_append_runtime_queue_values(queues, "queued_events", source.get(key), "")
			if remove_source_keys:
				source.erase(key)
	for key in OBJECT_MOVEMENT_QUEUE_KEYS:
		if source.has(key):
			_append_runtime_queue_values(queues, "object_movement_queue", source.get(key), "")
			if remove_source_keys:
				source.erase(key)

func _ingest_object_state_movement_queues(queues: Dictionary) -> void:
	var object_states: Dictionary = Dictionary(overworld_state.object_states)
	var changed: bool = false
	for object_key in object_states.keys():
		var record_changed: bool = false
		var record_variant: Variant = object_states.get(object_key, {})
		if typeof(record_variant) != TYPE_DICTIONARY:
			continue
		var record: Dictionary = Dictionary(record_variant)
		for queue_key in OBJECT_MOVEMENT_QUEUE_KEYS:
			if not record.has(queue_key):
				continue
			_append_runtime_queue_values(queues, "object_movement_queue", record.get(queue_key), str(object_key))
			record.erase(queue_key)
			changed = true
			record_changed = true
		if record_changed:
			object_states[object_key] = record
	if changed:
		overworld_state.object_states = object_states

func _ingest_map_callback_fields(queues: Dictionary) -> void:
	var map_key: String = str(overworld_state.current_map_key)
	if map_key.is_empty():
		map_key = str(overworld_state.get_selected_map_key()) if overworld_state.has_method("get_selected_map_key") else ""
	if map_key.is_empty() or str(queues.get("_map_callback_source_key", "")) == map_key:
		return
	var map_payloads: Array = [
		overworld_state.map_summary,
		Dictionary(overworld_state.map_manifest).get(map_key, {}),
	]
	for payload_variant in map_payloads:
		if typeof(payload_variant) == TYPE_DICTIONARY:
			var payload: Dictionary = Dictionary(payload_variant)
			for key in MAP_CALLBACK_KEYS:
				if payload.has(key):
					_append_runtime_queue_values(queues, "map_callbacks", payload.get(key), map_key)
	queues["_map_callback_source_key"] = map_key

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
			"commands": _normalize_runtime_command_array(value),
			"options": {},
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
			"commands": _normalize_runtime_command_array(value),
			"options": {},
		})
	else:
		queue.append(value)
	queues[queue_name] = queue

func _pop_runtime_queue_entry(queues: Dictionary, queue_name: String) -> Variant:
	var queue: Array = Array(queues.get(queue_name, []))
	if queue.is_empty():
		return null
	var entry: Variant = queue.pop_front()
	queues[queue_name] = queue
	return entry

func _process_runtime_queue_entry(queue_name: String, entry: Variant) -> bool:
	match queue_name:
		"map_callbacks":
			return _dispatch_runtime_map_callback(entry)
		"object_movement_queue":
			return _dispatch_runtime_object_movement(entry)
		_:
			return _dispatch_runtime_action_entry(entry, queue_name)

func _record_runtime_queue_result(queues: Dictionary, queue_name: String, entry: Variant, processed: bool) -> void:
	var result: Dictionary = {
		"queue": queue_name,
		"processed": processed,
		"frame": int(overworld_state.fixed_step_count),
		"entry": _serializable_runtime_value(entry),
	}
	queues["last_processed"] = result
	var completed: Array = Array(queues.get("completed", []))
	completed.append(result)
	while completed.size() > 12:
		completed.pop_front()
	queues["completed"] = completed

func _dispatch_runtime_map_callback(entry: Variant) -> bool:
	if typeof(entry) == TYPE_DICTIONARY:
		var callback: Dictionary = Dictionary(entry)
		var actions: Array = _runtime_actions_from_payload(callback)
		if not actions.is_empty():
			return _dispatch_runtime_actions(actions, "map_callbacks")
		var action_name: String = _runtime_action_name(callback)
		if not action_name.is_empty() and action_name != "map_callback":
			return _dispatch_runtime_action_entry(callback, "map_callbacks")
		var callback_script_name: String = str(callback.get("script_name", callback.get("script", callback.get("function_name", callback.get("name", ""))))).strip_edges()
		if not callback_script_name.is_empty():
			overworld_state.execute_special(callback_script_name, callback.duplicate(true))
			return true
		var callback_map_key: String = str(callback.get("map_key", callback.get("map", ""))).strip_edges()
		if not callback_map_key.is_empty():
			overworld_state.check_scene(callback_map_key)
			return true
		return false
	var script_name: String = str(entry).strip_edges()
	if script_name.is_empty():
		return false
	overworld_state.execute_special(script_name, {"source_queue": "map_callbacks"})
	return true

func _dispatch_runtime_action_entry(entry: Variant, source_queue: String) -> bool:
	if typeof(entry) == TYPE_STRING or typeof(entry) == TYPE_STRING_NAME:
		var function_name: String = str(entry).strip_edges()
		if function_name.is_empty():
			return false
		overworld_state.execute_special(function_name, {"source_queue": source_queue})
		return true
	if typeof(entry) != TYPE_DICTIONARY:
		return false
	var payload: Dictionary = Dictionary(entry)
	var action_name: String = _runtime_action_name(payload)
	if action_name.is_empty() or action_name in ["actions", "sequence"]:
		var actions: Array = _runtime_actions_from_payload(payload)
		if not actions.is_empty():
			return _dispatch_runtime_actions(actions, source_queue)
	match action_name:
		"", "script", "special", "call_special":
			var special_function_name: String = str(payload.get("function_name", payload.get("function", payload.get("special", payload.get("script", payload.get("label", "")))))).strip_edges()
			if special_function_name.is_empty():
				return false
			overworld_state.execute_special(special_function_name, payload.duplicate(true))
			return true
		"event", "interaction":
			overworld_state.request_interaction(str(payload.get("input", payload.get("interaction", payload.get("button", "confirm")))))
			return true
		"text", "open_text", "show_text":
			open_text(payload.get("content", payload.get("text", payload.get("message", ""))))
			return true
		"close_text":
			close_text()
			return true
		"wait_for_input", "wait_input":
			wait_for_input()
			return true
		"prompt_yes_no", "yes_no":
			prompt_yes_no(payload.get("prompt", payload.get("content", payload.get("text", ""))))
			return true
		"acknowledge_wait", "ack_wait":
			acknowledge_wait()
			return true
		"set_yes_no_result", "yes_no_result":
			set_yes_no_result(bool(payload.get("value", payload.get("result", false))))
			return true
		"map_callback", "check_scene":
			overworld_state.check_scene(str(payload.get("map_key", payload.get("map", ""))))
			return true
		"run_map_callbacks":
			run_map_callbacks(str(payload.get("map_key", payload.get("map", ""))), str(payload.get("callback_type", payload.get("type", ""))))
			return true
		"object_movement", "movement", "queue_movement":
			return _dispatch_runtime_object_movement(payload)
		"applymovement":
			return _dispatch_runtime_object_movement({
				"object": payload.get("object", payload.get("object_id", payload.get("target", ""))),
				"commands": payload.get("commands", payload.get("movement_commands", payload.get("movement", []))),
				"options": payload.get("options", {}),
			})
		"move_object":
			overworld_state.move_object(payload.get("object", payload.get("object_id", "")), int(payload.get("x", payload.get("tile_x", 0))), int(payload.get("y", payload.get("tile_y", 0))))
			return true
		"appear_object":
			overworld_state.appear_object(payload.get("object", payload.get("object_id", "")), _dictionary_from_variant(payload.get("options", {})))
			return true
		"remove_object":
			overworld_state.remove_object(payload.get("object", payload.get("object_id", "")), _dictionary_from_variant(payload.get("options", {})))
			return true
		"show_emote", "emote":
			overworld_state.show_emote(str(payload.get("emote_id", payload.get("emote", ""))), payload.get("object", payload.get("object_id", "")), int(payload.get("duration", 0)))
			return true
		"music", "request_music":
			overworld_state.request_music(str(payload.get("music_id", payload.get("music", ""))), str(payload.get("role", "")))
			return true
		"fade_music", "fade_to_music":
			overworld_state.fade_to_music(str(payload.get("music_id", payload.get("music", ""))), int(payload.get("speed_frames", payload.get("speed", 0))), str(payload.get("role", "")))
			return true
		"check_warp", "warp_check":
			return overworld_state.check_for_warp_event(_dictionary_from_variant(payload.get("options", {})))
		"set_warp_target":
			overworld_state.set_warp_target(str(payload.get("target", payload.get("warp_target", ""))))
			return true
		"set_event_flag":
			set_event_flag(str(payload.get("flag", payload.get("name", ""))), bool(payload.get("value", true)))
			return true
		"clear_event_flag":
			clear_event_flag(str(payload.get("flag", payload.get("name", ""))))
			return true
		"refresh_event_flag":
			refresh_event_flag(str(payload.get("flag", payload.get("name", ""))), _dictionary_from_variant(payload.get("options", {"value": payload.get("value", true)})))
			return true
		"set_engine_flag":
			set_engine_flag(str(payload.get("flag", payload.get("name", ""))), bool(payload.get("value", true)))
			return true
		"changeblock", "write_metatile":
			_write_metatile(int(payload.get("x", payload.get("metatile_x", 0))), int(payload.get("y", payload.get("metatile_y", 0))), int(payload.get("block_id", payload.get("block", 0))))
			return true
		"refresh_warp_permissions", "refresh_map":
			refresh_warp_permissions()
			return true
		"lock_movement":
			overworld_state.lock_player_movement()
			return true
		"unlock_movement":
			overworld_state.unlock_player_movement()
			return true
		"follow", "start_following":
			start_following(payload.get("follower", payload.get("object", payload.get("object_id", ""))), payload.get("leader", payload.get("target", "")), _dictionary_from_variant(payload.get("options", {})))
			return true
		"stop_following":
			stop_following()
			return true
	return false

func _dispatch_runtime_actions(actions: Array, source_queue: String) -> bool:
	var processed: bool = false
	for action in actions:
		processed = _dispatch_runtime_action_entry(action, source_queue) or processed
	return processed

func _dispatch_runtime_object_movement(entry: Variant) -> bool:
	if typeof(entry) != TYPE_DICTIONARY:
		return false
	var payload: Dictionary = Dictionary(entry)
	var object_ref: Variant = payload.get("object", payload.get("object_id", payload.get("target", "")))
	var commands: Array = _normalize_runtime_command_array(payload.get("commands", payload.get("movement_commands", payload.get("movement", []))))
	if commands.is_empty():
		return false
	var options: Dictionary = _dictionary_from_variant(payload.get("options", {}))
	overworld_state.queue_movement_task(object_ref, commands, options)
	return true

func _runtime_actions_from_payload(payload: Dictionary) -> Array:
	var actions: Variant = payload.get("actions", payload.get("commands", []))
	if typeof(actions) == TYPE_ARRAY:
		return Array(actions)
	return []

func _runtime_action_name(payload: Dictionary) -> String:
	return str(payload.get("action", payload.get("type", payload.get("op", payload.get("command", payload.get("kind", "")))))).strip_edges().to_lower()

func _normalize_runtime_command_array(value: Variant) -> Array:
	if typeof(value) == TYPE_ARRAY:
		return Array(value).duplicate(true)
	if typeof(value) == TYPE_DICTIONARY:
		var payload: Dictionary = Dictionary(value)
		if payload.has("commands"):
			return _normalize_runtime_command_array(payload.get("commands"))
		if payload.has("movement_commands"):
			return _normalize_runtime_command_array(payload.get("movement_commands"))
		return [payload.duplicate(true)]
	if value == null:
		return []
	return [value]

func _serializable_runtime_value(value: Variant) -> Variant:
	if typeof(value) == TYPE_DICTIONARY:
		return Dictionary(value).duplicate(true)
	if typeof(value) == TYPE_ARRAY:
		return Array(value).duplicate(true)
	return value

func _dictionary_from_variant(value: Variant) -> Dictionary:
	if typeof(value) == TYPE_DICTIONARY:
		return Dictionary(value).duplicate(true)
	return {}

func _merge_dialogue_state(dialogue_state: Dictionary) -> Dictionary:
	var merged: Dictionary = {}
	if overworld_state != null and typeof(overworld_state.dialogue_state) == TYPE_DICTIONARY:
		merged = Dictionary(overworld_state.dialogue_state).duplicate(true)
	for key in dialogue_state.keys():
		if not merged.has(key):
			merged[key] = dialogue_state.get(key)
	return merged

func _sync_dialogue_state() -> void:
	if overworld_state == null:
		return
	var merged: Dictionary = Dictionary(overworld_state.dialogue_state).duplicate(true)
	if dialogue != null and dialogue.has_method("get_state"):
		var dialogue_state: Dictionary = Dictionary(dialogue.get_state())
		for key in dialogue_state.keys():
			if not merged.has(key):
				merged[key] = dialogue_state.get(key)
	overworld_state.dialogue_state = merged

func _refresh_ui(force: bool = false) -> void:
	_refresh_map_selector(force)
	_refresh_map_surface(force)
	var signature: String = _build_ui_signature()
	if not force and signature == _last_ui_signature:
		return
	_last_ui_signature = signature
	var hud_lines: Array[String] = overworld_state.hud_lines()
	if is_instance_valid(_title_label):
		_title_label.text = "OVERWORLD"
	if is_instance_valid(_map_label):
		_map_label.text = _line_at(hud_lines, 0, "map: none")
	if is_instance_valid(_selection_label):
		_selection_label.text = _line_at(hud_lines, 2, "selection: none")
	if is_instance_valid(_player_label):
		_player_label.text = _line_at(hud_lines, 3, "tile: 0,0")
	if is_instance_valid(_movement_label):
		_movement_label.text = _line_at(hud_lines, 4, "movement: idle")
	if is_instance_valid(_result_label):
		_result_label.text = _line_at(hud_lines, 5, "result: move=idle collision=none warp=none")
	if is_instance_valid(_hooks_label):
		_hooks_label.text = "hooks: collision=%s warp=%s" % [
			str(overworld_state.collision_hook.is_valid()).to_lower(),
			str(overworld_state.warp_hook.is_valid()).to_lower(),
		]
	if is_instance_valid(_assets_label):
		var summary: Dictionary = Dictionary(overworld_state.asset_summary)
		_assets_label.text = "Assets: %d pokemon, %d moves, %d items, %d maps, %d packs" % [
			int(summary.get("pokemon_count", 0)),
			int(summary.get("move_count", 0)),
			int(summary.get("item_count", 0)),
			int(summary.get("map_attribute_count", 0)),
			int(summary.get("content_pack_count", 0)),
		]
	if is_instance_valid(_runtime_label):
		var runtime_summary: Dictionary = overworld_state.runtime_summary
		_runtime_label.text = "Runtime: map=%s [%s] size=%dx%d env=%s blocks=%d selected=%d/%d" % [
			overworld_state.current_map_name if not overworld_state.current_map_name.is_empty() else "unloaded",
			overworld_state.current_map_constant if not overworld_state.current_map_constant.is_empty() else "none",
			overworld_state.current_width,
			overworld_state.current_height,
			overworld_state.current_map_environment if not overworld_state.current_map_environment.is_empty() else "unknown",
			int(runtime_summary.get("block_count", 0)),
			int(runtime_summary.get("selected_map_index", -1)) + 1,
			int(runtime_summary.get("available_map_count", 0)),
		]
	if is_instance_valid(_debug_label):
		var debug_text := ""
		for line in hud_lines:
			var entry: String = line
			if not debug_text.is_empty():
				debug_text += "\n"
			debug_text += entry
			_debug_label.text = debug_text if not debug_text.is_empty() else "overworld ready"

func _refresh_map_surface(force: bool = false) -> void:
	if asset_index == null or overworld_state == null:
		return
	var map_name: String = str(overworld_state.current_map_name)
	if map_name.is_empty():
		map_name = str(overworld_state.map_title)
	if map_name.is_empty():
		if is_instance_valid(_map_surface_label):
			_map_surface_label.text = "map surface: no map loaded"
		return
	var signature: String = "|".join([
		map_name,
		str(int(overworld_state.current_width)),
		str(int(overworld_state.current_height)),
		str(overworld_state.current_map_block_key),
		_map_surface_mutation_signature(),
	])
	if not force and signature == _last_map_surface_signature:
		return
	_last_map_surface_signature = signature
	if not asset_index.has_method("load_map_tile_surface"):
		return
	var surface: Dictionary = Dictionary(asset_index.call("load_map_tile_surface", map_name))
	if surface.is_empty():
		if is_instance_valid(_map_surface_label):
			_map_surface_label.text = "map surface: unavailable for %s" % map_name
		return
	surface = _apply_mutable_map_surface(surface)
	var image: Image = surface.get("image", null)
	if image == null or image.is_empty():
		if is_instance_valid(_map_surface_label):
			_map_surface_label.text = "map surface: empty image for %s" % map_name
		return
	if is_instance_valid(_map_texture_rect):
		_map_texture_rect.texture = ImageTexture.create_from_image(image)
	_refresh_live_map_surface(image)
	if is_instance_valid(_map_surface_label):
		_map_surface_label.text = "map surface: %s %dx%d blocks, %s%s" % [
			map_name,
			int(surface.get("width", 0)),
			int(surface.get("height", 0)),
			str(surface.get("tileset_name", "")),
			_surface_mutation_suffix(),
		]

func _ensure_gameplay_surface() -> void:
	if is_instance_valid(_gameplay_surface):
		return
	if not is_inside_tree():
		return
	_gameplay_surface = Control.new()
	_gameplay_surface.name = "CrystalOverworldSurface"
	_gameplay_surface.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_gameplay_surface.set_anchors_preset(Control.PRESET_FULL_RECT)
	add_child(_gameplay_surface)
	move_child(_gameplay_surface, 0)
	var window_background := ColorRect.new()
	window_background.name = "WindowBackground"
	window_background.color = Color(0.0, 0.0, 0.0, 1.0)
	window_background.set_anchors_preset(Control.PRESET_FULL_RECT)
	window_background.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_gameplay_surface.add_child(window_background)
	_screen_clip = Control.new()
	_screen_clip.name = "GameBoyScreen"
	_screen_clip.clip_contents = true
	_screen_clip.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_gameplay_surface.add_child(_screen_clip)
	_screen_backdrop = ColorRect.new()
	_screen_backdrop.name = "Backdrop"
	_screen_backdrop.color = Color(0.80, 0.88, 0.80, 1.0)
	_screen_backdrop.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_screen_backdrop.set_anchors_preset(Control.PRESET_FULL_RECT)
	_screen_clip.add_child(_screen_backdrop)
	_live_map_rect = TextureRect.new()
	_live_map_rect.name = "MapViewport"
	_live_map_rect.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_live_map_rect.texture_filter = CanvasItem.TEXTURE_FILTER_NEAREST
	_live_map_rect.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
	_live_map_rect.stretch_mode = TextureRect.STRETCH_SCALE
	_screen_clip.add_child(_live_map_rect)
	_live_player_rect = ColorRect.new()
	_live_player_rect.name = "PlayerMarker"
	_live_player_rect.color = Color(0.05, 0.08, 0.10, 1.0)
	_live_player_rect.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_screen_clip.add_child(_live_player_rect)
	_layout_gameplay_surface()

func _layout_gameplay_surface() -> void:
	if not is_instance_valid(_gameplay_surface):
		return
	var viewport_size: Vector2 = size
	if viewport_size.x <= 0.0 or viewport_size.y <= 0.0:
		viewport_size = get_viewport_rect().size if is_inside_tree() else GB_SCREEN_SIZE * 2.0
	var scale_value: float = max(1.0, floor(min(viewport_size.x / GB_SCREEN_SIZE.x, viewport_size.y / GB_SCREEN_SIZE.y)))
	var screen_size: Vector2 = GB_SCREEN_SIZE * scale_value
	_screen_clip.position = (viewport_size - screen_size) * 0.5
	_screen_clip.size = screen_size
	if is_instance_valid(_live_map_rect):
		_live_map_rect.position = Vector2.ZERO
		_live_map_rect.size = screen_size
	if is_instance_valid(_live_player_rect):
		var player_screen := _player_screen_position()
		_live_player_rect.position = Vector2(player_screen.x, player_screen.y) * scale_value
		_live_player_rect.size = Vector2(16.0, 16.0) * scale_value

func _refresh_live_map_surface(source_image: Image) -> void:
	_ensure_gameplay_surface()
	if not is_instance_valid(_live_map_rect) or source_image == null or source_image.is_empty():
		return
	var viewport_image := _build_camera_viewport(source_image)
	if viewport_image == null or viewport_image.is_empty():
		return
	_last_viewport_image_size = Vector2i(viewport_image.get_width(), viewport_image.get_height())
	_live_map_rect.texture = ImageTexture.create_from_image(viewport_image)
	_layout_gameplay_surface()

func _build_camera_viewport(source_image: Image) -> Image:
	var viewport_image := Image.create(int(GB_SCREEN_SIZE.x), int(GB_SCREEN_SIZE.y), false, Image.FORMAT_RGBA8)
	viewport_image.fill(Color(0.80, 0.88, 0.80, 1.0))
	var source_width := source_image.get_width()
	var source_height := source_image.get_height()
	if source_width <= 0 or source_height <= 0:
		return viewport_image
	var player_tile: Vector2i = overworld_state.player_tile if overworld_state != null else Vector2i.ZERO
	var player_px := Vector2i(player_tile.x * TILE_PIXEL_SIZE, player_tile.y * TILE_PIXEL_SIZE)
	var max_origin_x: int = max(0, source_width - int(GB_SCREEN_SIZE.x))
	var max_origin_y: int = max(0, source_height - int(GB_SCREEN_SIZE.y))
	var origin_x: int = clampi(player_px.x - int(GB_SCREEN_SIZE.x * 0.5) + int(TILE_PIXEL_SIZE * 0.5), 0, max_origin_x)
	var origin_y: int = clampi(player_px.y - int(GB_SCREEN_SIZE.y * 0.5) + int(TILE_PIXEL_SIZE * 0.5), 0, max_origin_y)
	_last_camera_origin = Vector2i(origin_x, origin_y)
	var copy_width: int = min(source_width, int(GB_SCREEN_SIZE.x))
	var copy_height: int = min(source_height, int(GB_SCREEN_SIZE.y))
	var dest_x: int = int(max(0, floor((GB_SCREEN_SIZE.x - float(copy_width)) * 0.5))) if source_width < int(GB_SCREEN_SIZE.x) else 0
	var dest_y: int = int(max(0, floor((GB_SCREEN_SIZE.y - float(copy_height)) * 0.5))) if source_height < int(GB_SCREEN_SIZE.y) else 0
	var src_rect := Rect2i(Vector2i(origin_x, origin_y), Vector2i(copy_width, copy_height))
	viewport_image.blit_rect(source_image, src_rect, Vector2i(dest_x, dest_y))
	return viewport_image

func _player_screen_position() -> Vector2i:
	if overworld_state == null:
		return Vector2i(72, 64)
	var player_tile: Vector2i = overworld_state.player_tile
	var map_width_px := int(overworld_state.current_width) * 32
	var map_height_px := int(overworld_state.current_height) * 32
	var player_px := Vector2i(player_tile.x * TILE_PIXEL_SIZE, player_tile.y * TILE_PIXEL_SIZE)
	var centered_offset := Vector2i.ZERO
	if map_width_px < int(GB_SCREEN_SIZE.x):
		centered_offset.x = int(floor((GB_SCREEN_SIZE.x - float(map_width_px)) * 0.5))
	if map_height_px < int(GB_SCREEN_SIZE.y):
		centered_offset.y = int(floor((GB_SCREEN_SIZE.y - float(map_height_px)) * 0.5))
	var screen_px := player_px - _last_camera_origin + centered_offset
	screen_px.x = clampi(screen_px.x, 0, int(GB_SCREEN_SIZE.x) - 16)
	screen_px.y = clampi(screen_px.y, 0, int(GB_SCREEN_SIZE.y) - 16)
	return screen_px

func _apply_mutable_map_surface(surface: Dictionary) -> Dictionary:
	if overworld_state == null:
		return surface
	var changed_blocks: Dictionary = Dictionary(overworld_state.current_map_payload.get("changed_blocks", {}))
	if changed_blocks.is_empty():
		return surface
	var block_ids_variant: Variant = surface.get("block_ids", [])
	if typeof(block_ids_variant) != TYPE_PACKED_BYTE_ARRAY:
		return surface
	var block_ids: PackedByteArray = block_ids_variant
	var width: int = int(surface.get("width", 0))
	var height: int = int(surface.get("height", 0))
	if width <= 0 or height <= 0 or block_ids.size() != width * height:
		return surface
	var mutated_block_ids := block_ids.duplicate()
	for key in changed_blocks.keys():
		var parts := str(key).split(",", false)
		if parts.size() != 2:
			continue
		var tile_x := int(parts[0])
		var tile_y := int(parts[1])
		if tile_x < 0 or tile_y < 0 or tile_x >= width or tile_y >= height:
			continue
		var index := tile_y * width + tile_x
		if index < 0 or index >= mutated_block_ids.size():
			continue
		mutated_block_ids[index] = int(changed_blocks.get(key, 0)) & 0xff
	surface["block_ids"] = mutated_block_ids
	var metatiles: Array = Array(surface.get("metatiles", []))
	if metatiles.is_empty():
		return surface
	var rebuilt_image: Image = GB_TILE_DECODER_SCRIPT.assemble_map_blocks(metatiles, mutated_block_ids, width, height)
	if rebuilt_image != null and not rebuilt_image.is_empty():
		surface["image"] = rebuilt_image
	return surface

func _map_surface_mutation_signature() -> String:
	if overworld_state == null:
		return ""
	var payload: Dictionary = Dictionary(overworld_state.current_map_payload)
	var changed_blocks: Dictionary = Dictionary(payload.get("changed_blocks", {}))
	var last_changed_block: Dictionary = Dictionary(payload.get("last_changed_block", {}))
	if changed_blocks.is_empty() and last_changed_block.is_empty():
		return ""
	return "%s|%s" % [str(changed_blocks.hash()), str(last_changed_block.hash())]

func _surface_mutation_suffix() -> String:
	if overworld_state == null:
		return ""
	var changed_blocks: Dictionary = Dictionary(overworld_state.current_map_payload.get("changed_blocks", {}))
	if changed_blocks.is_empty():
		return ""
	return " changed=%d" % changed_blocks.size()

func _build_ui_signature() -> String:
	var hud_lines: Array[String] = overworld_state.hud_lines()
	var runtime_summary: Dictionary = overworld_state.runtime_summary
	var selected_key: String = overworld_state.get_selected_map_key() if overworld_state.has_method("get_selected_map_key") else overworld_state.current_map_key
	var state_hash := str(overworld_state.last_move_result.hash() + overworld_state.asset_summary.hash() + runtime_summary.hash() + hud_lines.hash())
	return "%s|%s|%d,%d|%s|%s|%d|%s|%s" % [
		overworld_state.current_map_key,
		selected_key,
		overworld_state.player_tile.x,
		overworld_state.player_tile.y,
		overworld_state.player_facing,
		str(overworld_state.movement_state),
		int(overworld_state.selected_map_index),
		str(overworld_state.last_runtime_note),
		state_hash,
	]

func _line_at(lines: Array, index: int, fallback: String) -> String:
	if index < 0 or index >= lines.size():
		return fallback
	return str(lines[index])

func _refresh_map_selector(force: bool = false) -> void:
	if not is_instance_valid(_map_selector) or overworld_state == null:
		return
	var keys: Array[String] = overworld_state.get_available_map_keys() if overworld_state.has_method("get_available_map_keys") else []
	var selected_key: String = overworld_state.get_selected_map_key() if overworld_state.has_method("get_selected_map_key") else overworld_state.current_map_key
	var key_signature := ""
	for key in keys:
		key_signature += "|%s" % key
	var signature := "%s%s" % [selected_key, key_signature]
	if not force and signature == _last_map_selector_signature:
		return
	_last_map_selector_signature = signature
	_updating_map_selector = true
	_map_selector.clear()
	if keys.is_empty():
		_map_selector.add_item("no maps available")
		_map_selector.set_item_metadata(0, "")
	else:
		for key in keys:
			var display_label := _map_selector_label_for_key(key)
			_map_selector.add_item(display_label)
			_map_selector.set_item_metadata(_map_selector.get_item_count() - 1, key)
		var selected_index := _map_selector_index_for_key(selected_key, keys)
		if selected_index >= 0:
			_map_selector.select(selected_index)
		elif _map_selector.get_item_count() > 0:
			_map_selector.select(0)
	_updating_map_selector = false

func _map_selector_label_for_key(map_key: String) -> String:
	if overworld_state == null:
		return map_key
	var manifest: Dictionary = Dictionary(overworld_state.map_manifest)
	if not manifest.has(map_key):
		return map_key
	var summary: Dictionary = Dictionary(manifest.get(map_key, {}))
	var title := str(summary.get("title", summary.get("name", map_key)))
	if title.is_empty() or title == map_key:
		return map_key
	return "%s [%s]" % [title, map_key]

func _map_selector_index_for_key(map_key: String, keys: Array[String]) -> int:
	for index in range(keys.size()):
		if keys[index] == map_key:
			return index
	return -1

func _on_map_selector_item_selected(index: int) -> void:
	if _updating_map_selector or overworld_state == null:
		return
	if not is_instance_valid(_map_selector):
		return
	var key := str(_map_selector.get_item_metadata(index))
	if key.is_empty():
		return
	overworld_state.set_map_key(key)
	_refresh_map_selector(true)
	_refresh_ui(true)

func _on_prev_map_pressed() -> void:
	cycle_map(-1)

func _on_next_map_pressed() -> void:
	cycle_map(1)

func _on_reload_map_pressed() -> void:
	reload_current_map()
