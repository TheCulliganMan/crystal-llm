extends "res://scripts/boot_scene_base.gd"

const ASSET_INDEX_SCRIPT = preload("res://scripts/asset_index.gd")
const NEXT_ROUTE := "oak_intro"
const INTRO_STEPS := [
	{"id": "copyright", "frames": 60, "input_gate_frames": 12},
	{"id": "opening_logo", "frames": 60, "input_gate_frames": 0},
	{"id": "pokemon_scene", "frames": 60, "input_gate_frames": 0},
	{"id": "title_bridge", "frames": 60, "input_gate_frames": 0},
]
const GB_SCREEN_SIZE := Vector2(160.0, 144.0)

var _asset_index: Variant = ASSET_INDEX_SCRIPT.new()
var _visual_root: Control
var _screen_clip: Control
var _screen_backdrop: ColorRect
var _copyright_rect: TextureRect
var _gamefreak_logo_rect: TextureRect
var _gamefreak_presents_rect: TextureRect
var _intro_background_rect: TextureRect
var _unowns_rect: TextureRect
var _suicune_back_rect: TextureRect
var _suicune_run_rect: TextureRect
var _suicune_close_rect: TextureRect
var _pichu_wooper_rect: TextureRect

func _on_ready() -> void:
	_ensure_intro_surface()
	if state.is_empty():
		state = _default_screen_state("intro_sequence")
		state.merge({
			"scene_index": 0,
			"scene_id": str(INTRO_STEPS[0]["id"]),
			"scene_frame": 0,
			"frame_counter": 0,
			"finished": false,
			"skip_requested": false,
			"input_gate_frames": int(INTRO_STEPS[0]["input_gate_frames"]),
		}, true)
	_refresh_labels()
	_refresh_intro_surface()

func _notification(what: int) -> void:
	if what == NOTIFICATION_RESIZED:
		_layout_intro_surface()

func can_accept_input() -> bool:
	return super.can_accept_input() and not bool(state.get("finished", false)) and int(state.get("scene_frame", 0)) >= int(state.get("input_gate_frames", 0))

func _tick(_delta: float) -> void:
	if bool(state.get("finished", false)) or bool(state.get("input_locked", false)):
		return
	state["frame_counter"] = int(state.get("frame_counter", 0)) + 1
	state["scene_frame"] = int(state.get("scene_frame", 0)) + 1
	var step := _current_step()
	if int(state.get("scene_frame", 0)) >= int(step.get("frames", 60)):
		_advance_scene()

func _handle_boot_input(event: InputEvent) -> void:
	if _input_pressed(event, ["a", "b", "start"]):
		state["skip_requested"] = true
		_finish_intro("intro_skip")

func _advance_scene() -> void:
	var next_index := int(state.get("scene_index", 0)) + 1
	if next_index >= INTRO_STEPS.size():
		_finish_intro("intro_complete")
		return
	state["scene_index"] = next_index
	state["scene_id"] = str(INTRO_STEPS[next_index]["id"])
	state["scene_frame"] = 0
	state["input_gate_frames"] = int(INTRO_STEPS[next_index].get("input_gate_frames", 0))
	_refresh_intro_surface()

func _finish_intro(reason: String) -> void:
	state["finished"] = true
	_refresh_intro_surface()
	request_route(NEXT_ROUTE, reason)

func _current_step() -> Dictionary:
	var index := clampi(int(state.get("scene_index", 0)), 0, INTRO_STEPS.size() - 1)
	return Dictionary(INTRO_STEPS[index])

func _on_state_restored() -> void:
	_ensure_intro_surface()
	state["screen"] = "intro_sequence"
	state["scene_index"] = clampi(int(state.get("scene_index", 0)), 0, INTRO_STEPS.size() - 1)
	state["scene_id"] = str(state.get("scene_id", INTRO_STEPS[int(state.get("scene_index", 0))]["id"]))
	state["scene_frame"] = max(0, int(state.get("scene_frame", state.get("frame_counter", 0))))
	state["frame_counter"] = max(0, int(state.get("frame_counter", 0)))
	state["finished"] = bool(state.get("finished", false))
	state["skip_requested"] = bool(state.get("skip_requested", false))
	state["input_gate_frames"] = max(0, int(state.get("input_gate_frames", _current_step().get("input_gate_frames", 0))))
	if _route_entry_reset():
		state["scene_index"] = 0
		state["scene_id"] = str(INTRO_STEPS[0]["id"])
		state["scene_frame"] = 0
		state["frame_counter"] = 0
		state["finished"] = false
		state["skip_requested"] = false
		state["input_gate_frames"] = int(INTRO_STEPS[0]["input_gate_frames"])
		_clear_route_entry()
	_refresh_intro_surface()

func _update_labels() -> void:
	_set_labels(
		"INTRO SEQUENCE",
		"Scene: %s (%d/%d) | Frame: %d" % [
			str(state.get("scene_id", "copyright")),
			int(state.get("scene_index", 0)) + 1,
			INTRO_STEPS.size(),
			int(state.get("scene_frame", 0)),
		],
		"A/B/Start skip after input gate"
	)
	_refresh_intro_surface()

func _ensure_intro_surface() -> void:
	if is_instance_valid(_visual_root):
		return
	if not is_inside_tree():
		return
	var legacy_margin := get_node_or_null("Margin")
	if legacy_margin is CanvasItem:
		legacy_margin.visible = false
	_visual_root = Control.new()
	_visual_root.name = "CrystalIntroSurface"
	_visual_root.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_visual_root.set_anchors_preset(Control.PRESET_FULL_RECT)
	add_child(_visual_root)
	move_child(_visual_root, 0)
	var window_background := ColorRect.new()
	window_background.name = "WindowBackground"
	window_background.color = Color(0.0, 0.0, 0.0, 1.0)
	window_background.set_anchors_preset(Control.PRESET_FULL_RECT)
	_visual_root.add_child(window_background)
	_screen_clip = Control.new()
	_screen_clip.name = "GameBoyScreen"
	_screen_clip.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_screen_clip.clip_contents = true
	_visual_root.add_child(_screen_clip)
	_screen_backdrop = ColorRect.new()
	_screen_backdrop.name = "Backdrop"
	_screen_backdrop.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_screen_backdrop.set_anchors_preset(Control.PRESET_FULL_RECT)
	_screen_clip.add_child(_screen_backdrop)
	_copyright_rect = _add_texture_rect("Copyright", "gfx/splash/copyright.png")
	_gamefreak_logo_rect = _add_texture_rect("GameFreakLogo", "gfx/splash/gamefreak_logo.png")
	_gamefreak_presents_rect = _add_texture_rect("GameFreakPresents", "gfx/splash/gamefreak_presents.png")
	_intro_background_rect = _add_texture_rect("IntroBackground", "gfx/intro/background.png")
	_unowns_rect = _add_texture_rect("Unowns", "gfx/intro/unowns.png")
	_suicune_back_rect = _add_texture_rect("SuicuneBack", "gfx/intro/suicune_back.png")
	_suicune_run_rect = _add_texture_rect("SuicuneRun", "gfx/intro/suicune_run.png")
	_suicune_close_rect = _add_texture_rect("SuicuneClose", "gfx/intro/suicune_close.png")
	_pichu_wooper_rect = _add_texture_rect("PichuWooper", "gfx/intro/pichu_wooper.png")
	_layout_intro_surface()

func _add_texture_rect(node_name: String, relative_path: String) -> TextureRect:
	var rect := TextureRect.new()
	rect.name = node_name
	rect.mouse_filter = Control.MOUSE_FILTER_IGNORE
	rect.texture_filter = CanvasItem.TEXTURE_FILTER_NEAREST
	rect.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
	rect.stretch_mode = TextureRect.STRETCH_SCALE
	var image: Image = _asset_index.load_image(relative_path)
	if not image.is_empty():
		rect.texture = ImageTexture.create_from_image(image)
	_screen_clip.add_child(rect)
	return rect

func _layout_intro_surface() -> void:
	if not is_instance_valid(_visual_root):
		return
	var viewport_size: Vector2 = size
	if viewport_size.x <= 0.0 or viewport_size.y <= 0.0:
		viewport_size = get_viewport_rect().size if is_inside_tree() else GB_SCREEN_SIZE * 2.0
	var scale_value: float = max(1.0, floor(min(viewport_size.x / GB_SCREEN_SIZE.x, viewport_size.y / GB_SCREEN_SIZE.y)))
	var screen_size: Vector2 = GB_SCREEN_SIZE * scale_value
	_screen_clip.position = (viewport_size - screen_size) * 0.5
	_screen_clip.size = screen_size
	_set_logical_rect(_copyright_rect, scale_value, Vector2(-36.0, 72.0), Vector2(232.0, 8.0))
	_set_logical_rect(_gamefreak_logo_rect, scale_value, Vector2(68.0, 38.0), Vector2(24.0, 40.0))
	_set_logical_rect(_gamefreak_presents_rect, scale_value, Vector2(28.0, 88.0), Vector2(104.0, 8.0))
	_set_logical_rect(_intro_background_rect, scale_value, Vector2(16.0, 48.0), Vector2(128.0, 64.0))
	_set_logical_rect(_unowns_rect, scale_value, Vector2(16.0, 24.0), Vector2(128.0, 64.0))
	_set_logical_rect(_suicune_back_rect, scale_value, Vector2(16.0, 60.0), Vector2(128.0, 64.0))
	_set_logical_rect(_suicune_run_rect, scale_value, Vector2(16.0, 32.0), Vector2(128.0, 96.0))
	_set_logical_rect(_suicune_close_rect, scale_value, Vector2(16.0, 8.0), Vector2(128.0, 128.0))
	_set_logical_rect(_pichu_wooper_rect, scale_value, Vector2(16.0, 56.0), Vector2(128.0, 64.0))

func _set_logical_rect(node: Control, scale_value: float, logical_position: Vector2, logical_size: Vector2) -> void:
	if not is_instance_valid(node):
		return
	node.position = logical_position * scale_value
	node.size = logical_size * scale_value

func _refresh_intro_surface() -> void:
	if not is_instance_valid(_visual_root):
		return
	var scene_id := str(state.get("scene_id", "copyright"))
	var scene_frame := int(state.get("scene_frame", 0))
	var finished := bool(state.get("finished", false))
	_copyright_rect.visible = scene_id == "copyright" and not finished
	_gamefreak_logo_rect.visible = scene_id == "opening_logo" and not finished
	_gamefreak_presents_rect.visible = scene_id == "opening_logo" and not finished
	_intro_background_rect.visible = scene_id == "pokemon_scene" and not finished
	_unowns_rect.visible = scene_id == "pokemon_scene" and scene_frame < 30 and not finished
	_suicune_back_rect.visible = scene_id == "pokemon_scene" and scene_frame >= 18 and scene_frame < 42 and not finished
	_suicune_run_rect.visible = scene_id == "pokemon_scene" and scene_frame >= 42 and not finished
	_suicune_close_rect.visible = scene_id == "title_bridge" and not finished
	_pichu_wooper_rect.visible = scene_id == "title_bridge" and scene_frame >= 30 and not finished
	match scene_id:
		"copyright", "opening_logo":
			_screen_backdrop.color = Color(0.95, 0.98, 0.96, 1.0)
		"pokemon_scene":
			_screen_backdrop.color = Color(0.46, 0.66, 0.80, 1.0)
		"title_bridge":
			_screen_backdrop.color = Color(0.08, 0.12, 0.18, 1.0)
		_:
			_screen_backdrop.color = Color(0.0, 0.0, 0.0, 1.0)
