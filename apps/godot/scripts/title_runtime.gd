extends "res://scripts/boot_scene_base.gd"

const ASSET_INDEX_SCRIPT = preload("res://scripts/asset_index.gd")
const INTRO_ROUTE := "intro_sequence"
const CONTINUE_ROUTE := "continue_screen"
const DELETE_ROUTE := "delete_save_screen"
const CLOCK_ROUTE := "clock_reset_screen"
const ENTRANCE_FRAMES := 30
const MAIN_INPUT_GATE_FRAMES := 8
const ATTRACT_TIMEOUT_FRAMES := 180
const EXIT_HOLD_FRAMES := 12
const GB_SCREEN_SIZE := Vector2(160.0, 144.0)

var _asset_index: Variant = ASSET_INDEX_SCRIPT.new()
var _visual_root: Control
var _screen_rect: ColorRect
var _logo_rect: TextureRect
var _suicune_rect: TextureRect
var _crystal_rect: TextureRect
var _copyright_label: Label
var _prompt_label: Label

func _on_ready() -> void:
	_ensure_title_surface()
	if state.is_empty():
		state = _default_screen_state("title")
		state.merge({
			"phase": "entrance",
			"phase_frame": 0,
			"frame_counter": 0,
			"title_timer": 0,
			"logo_palette_phase": "intro",
			"suicune_palette_phase": "intro",
			"suicune_frame": 0,
			"suicune_animation_timer": 0,
			"input_gate_frames": ENTRANCE_FRAMES,
			"route_queue_hold_frames": EXIT_HOLD_FRAMES,
			"pending_action": "",
			"attract_timeout_frames": ATTRACT_TIMEOUT_FRAMES,
		}, true)
	_refresh_labels()
	_refresh_title_surface()

func _notification(what: int) -> void:
	if what == NOTIFICATION_RESIZED:
		_layout_title_surface()

func can_accept_input() -> bool:
	return super.can_accept_input() and str(state.get("phase", "entrance")) == "main" and int(state.get("phase_frame", 0)) >= int(state.get("input_gate_frames", MAIN_INPUT_GATE_FRAMES))

func _tick(_delta: float) -> void:
	state["frame_counter"] = int(state.get("frame_counter", 0)) + 1
	state["phase_frame"] = int(state.get("phase_frame", 0)) + 1
	state["title_timer"] = int(state.get("title_timer", 0)) + 1
	state["suicune_animation_timer"] = (int(state.get("suicune_animation_timer", 0)) + 1) % 8
	if int(state.get("suicune_animation_timer", 0)) == 0:
		state["suicune_frame"] = (int(state.get("suicune_frame", 0)) + 1) % 4
	var phase := str(state.get("phase", "entrance"))
	if phase == "entrance" and int(state.get("phase_frame", 0)) >= ENTRANCE_FRAMES:
		_enter_phase("timer", 0)
		return
	if phase == "timer":
		_enter_phase("main", MAIN_INPUT_GATE_FRAMES)
		return
	if phase == "main" and int(state.get("title_timer", 0)) >= int(state.get("attract_timeout_frames", ATTRACT_TIMEOUT_FRAMES)):
		_enter_phase("timeout", 0)
		return
	if phase == "timeout" and int(state.get("phase_frame", 0)) >= int(state.get("route_queue_hold_frames", EXIT_HOLD_FRAMES)):
		_queue_title_route(INTRO_ROUTE, "title_timeout")
		return
	if phase == "exiting" and int(state.get("phase_frame", 0)) >= EXIT_HOLD_FRAMES:
		state["input_locked"] = true

func _handle_boot_input(event: InputEvent) -> void:
	if _input_pressed(event, ["a", "start"]):
		_queue_title_route(INTRO_ROUTE, "title_new_game")
		return
	if _input_pressed(event, ["b"]):
		_queue_title_route(CONTINUE_ROUTE, "title_continue")
		return
	var select_down := _last_down("select") or _input_pressed(event, ["select"])
	if select_down and (_last_down("up") or _input_pressed(event, ["up"])):
		_queue_title_route(DELETE_ROUTE, "title_delete_save")
		return
	if select_down and (_last_down("down") or _input_pressed(event, ["down"])):
		_queue_title_route(CLOCK_ROUTE, "title_clock_reset")

func _queue_title_route(route_name: String, reason: String) -> void:
	if str(state.get("pending_action", "")) != "":
		return
	var title_actions: Dictionary = {
		INTRO_ROUTE: "title_new_game",
		CONTINUE_ROUTE: "title_continue",
		DELETE_ROUTE: "title_delete_save",
		CLOCK_ROUTE: "title_clock_reset",
	}
	var action_id: String = str(reason)
	var source_phase: String = str(state.get("phase", "entrance"))
	var source_phase_frame := int(state.get("phase_frame", 0))
	var source_frame_counter := int(state.get("frame_counter", 0))
	var source_tick_counter := int(state.get("handled_input_count", 0))
	var source_logo_palette_phase: String = str(state.get("logo_palette_phase", "intro"))
	var source_suicune_palette_phase: String = str(state.get("suicune_palette_phase", "intro"))
	var source_suicune_frame: int = int(state.get("suicune_frame", 0))
	var source_suicune_animation_timer: int = int(state.get("suicune_animation_timer", 0))
	var title_options: Dictionary = {
		"title_new_game": "new_game",
		"title_continue": "continue",
		"title_delete_save": "delete_save",
		"title_clock_reset": "clock_reset",
		"title_timeout": "timeout",
	}
	var selected_option: String = str(title_options.get(action_id, str(title_actions.get(route_name, route_name))))
	var boot_flow_path: Array = []
	if action_id == "title_new_game":
		boot_flow_path = ["intro_sequence", "clock_reset_screen", "day_of_week_screen", "oak_intro", "name_entry"]
	_enter_phase("exiting", EXIT_HOLD_FRAMES)
	queue_action(route_name, str(action_id), {
		"selected_option": str(selected_option),
		"reason": reason,
		"phase": source_phase,
		"phase_frame": source_phase_frame,
		"frame_counter": source_frame_counter,
		"tick_counter": source_tick_counter,
		"logo_palette_phase": source_logo_palette_phase,
		"suicune_palette_phase": source_suicune_palette_phase,
		"suicune_frame": source_suicune_frame,
		"suicune_animation_timer": source_suicune_animation_timer,
		"title_timer": int(state.get("title_timer", 0)),
		"attract_timeout_frames": int(state.get("attract_timeout_frames", ATTRACT_TIMEOUT_FRAMES)),
		"input_gate_frames": int(state.get("input_gate_frames", MAIN_INPUT_GATE_FRAMES)),
		"route_queue_hold_frames": int(state.get("route_queue_hold_frames", EXIT_HOLD_FRAMES)),
		"boot_flow_path": boot_flow_path,
	})

func _enter_phase(phase_name: String, input_gate_frames: int) -> void:
	state["phase"] = phase_name
	state["phase_frame"] = 0
	state["input_gate_frames"] = max(0, input_gate_frames)
	state["logo_palette_phase"] = _palette_phase_for_title_phase(phase_name)
	state["suicune_palette_phase"] = _palette_phase_for_title_phase(phase_name)
	if phase_name == "main":
		state["title_timer"] = 0

func _on_state_restored() -> void:
	_ensure_title_surface()
	state["screen"] = "title"
	state["phase"] = _normalize_phase(str(state.get("phase", "entrance")))
	state["phase_frame"] = max(0, int(state.get("phase_frame", state.get("title_timer", 0))))
	state["frame_counter"] = max(0, int(state.get("frame_counter", state.get("phase_frame", 0))))
	state["title_timer"] = max(0, int(state.get("title_timer", 0)))
	var restored_phase: String = str(state.get("phase", "entrance"))
	var default_palette_phase: String = _palette_phase_for_title_phase(restored_phase)
	state["logo_palette_phase"] = str(state.get("logo_palette_phase", default_palette_phase))
	state["suicune_palette_phase"] = str(state.get("suicune_palette_phase", default_palette_phase))
	state["suicune_frame"] = int(state.get("suicune_frame", 2 if restored_phase == "timeout" else 0)) % 4
	state["suicune_animation_timer"] = int(state.get("suicune_animation_timer", 3 if restored_phase == "timeout" else 0)) % 8
	state["input_gate_frames"] = max(0, int(state.get("input_gate_frames", MAIN_INPUT_GATE_FRAMES)))
	state["route_queue_hold_frames"] = max(0, int(state.get("route_queue_hold_frames", EXIT_HOLD_FRAMES)))
	state["pending_action"] = str(state.get("pending_action", ""))
	state["attract_timeout_frames"] = max(1, int(state.get("attract_timeout_frames", ATTRACT_TIMEOUT_FRAMES)))
	if _route_entry_reset():
		_enter_phase("main", MAIN_INPUT_GATE_FRAMES)
		state["phase_frame"] = int(state.get("input_gate_frames", MAIN_INPUT_GATE_FRAMES))
		state["frame_counter"] = 0
		state["suicune_frame"] = 0
		state["suicune_animation_timer"] = 0
		clear_pending_action()
		_clear_route_entry()
	_refresh_title_surface()

func _normalize_phase(value: String) -> String:
	match value:
		"entrance", "timer", "main", "timeout", "exiting":
			return value
		_:
			return "entrance"

func _palette_phase_for_title_phase(phase_name: String) -> String:
	match phase_name:
		"entrance", "timer":
			return "intro"
		"main":
			return "steady"
		"timeout":
			return "steady"
		"exiting":
			return "handoff"
		_:
			return "intro"

func _update_labels() -> void:
	_set_labels(
		"TITLE SCREEN",
		"Phase: %s | Frame: %d | Timer: %d" % [
			str(state.get("phase", "entrance")),
			int(state.get("phase_frame", 0)),
			int(state.get("title_timer", 0)),
		],
		"A/Start=New Game  B=Continue  Select+Up=Delete  Select+Down=Clock"
	)
	_refresh_title_surface()

func _ensure_title_surface() -> void:
	if is_instance_valid(_visual_root):
		return
	if not is_inside_tree():
		return
	var legacy_margin := get_node_or_null("Margin")
	if legacy_margin is CanvasItem:
		legacy_margin.visible = false
	_visual_root = Control.new()
	_visual_root.name = "CrystalTitleSurface"
	_visual_root.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_visual_root.set_anchors_preset(Control.PRESET_FULL_RECT)
	add_child(_visual_root)
	move_child(_visual_root, 0)
	var background := ColorRect.new()
	background.name = "WindowBackground"
	background.color = Color(0.0, 0.0, 0.0, 1.0)
	background.set_anchors_preset(Control.PRESET_FULL_RECT)
	_visual_root.add_child(background)
	_screen_rect = ColorRect.new()
	_screen_rect.name = "GameBoyScreen"
	_screen_rect.color = Color(0.88, 0.94, 0.92, 1.0)
	_visual_root.add_child(_screen_rect)
	_logo_rect = _add_texture_rect("Logo", "gfx/title/logo.png")
	_suicune_rect = _add_texture_rect("Suicune", "gfx/title/suicune.png")
	_crystal_rect = _add_texture_rect("Crystal", "gfx/title/crystal.png")
	_copyright_label = _add_title_label("Copyright", "(C)1995-2001 Nintendo")
	_prompt_label = _add_title_label("Prompt", "PRESS START")
	_layout_title_surface()

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
	_visual_root.add_child(rect)
	return rect

func _add_title_label(node_name: String, text_value: String) -> Label:
	var label := Label.new()
	label.name = node_name
	label.text = text_value
	label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	label.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	label.mouse_filter = Control.MOUSE_FILTER_IGNORE
	label.add_theme_font_size_override("font_size", 8)
	label.add_theme_color_override("font_color", Color(0.06, 0.12, 0.14, 1.0))
	_visual_root.add_child(label)
	return label

func _layout_title_surface() -> void:
	if not is_instance_valid(_visual_root):
		return
	var viewport_size: Vector2 = size
	if viewport_size.x <= 0.0 or viewport_size.y <= 0.0:
		viewport_size = get_viewport_rect().size if is_inside_tree() else GB_SCREEN_SIZE * 2.0
	var scale_value: float = max(1.0, floor(min(viewport_size.x / GB_SCREEN_SIZE.x, viewport_size.y / GB_SCREEN_SIZE.y)))
	var screen_size: Vector2 = GB_SCREEN_SIZE * scale_value
	var origin: Vector2 = (viewport_size - screen_size) * 0.5
	_screen_rect.position = origin
	_screen_rect.size = screen_size
	_set_logical_rect(_logo_rect, origin, scale_value, Vector2(0.0, 8.0), Vector2(160.0, 64.0))
	_set_logical_rect(_suicune_rect, origin, scale_value, Vector2(16.0, 56.0), Vector2(128.0, 128.0))
	_set_logical_rect(_crystal_rect, origin, scale_value, Vector2(108.0, 58.0), Vector2(48.0, 80.0))
	_set_logical_rect(_copyright_label, origin, scale_value, Vector2(0.0, 130.0), Vector2(160.0, 8.0))
	_set_logical_rect(_prompt_label, origin, scale_value, Vector2(32.0, 112.0), Vector2(96.0, 10.0))
	var label_font_size: int = max(8, int(8.0 * scale_value))
	_copyright_label.add_theme_font_size_override("font_size", label_font_size)
	_prompt_label.add_theme_font_size_override("font_size", label_font_size)

func _set_logical_rect(node: Control, origin: Vector2, scale_value: float, logical_position: Vector2, logical_size: Vector2) -> void:
	if not is_instance_valid(node):
		return
	node.position = origin + logical_position * scale_value
	node.size = logical_size * scale_value

func _refresh_title_surface() -> void:
	if not is_instance_valid(_visual_root):
		return
	var phase := str(state.get("phase", "entrance"))
	var frame := int(state.get("phase_frame", 0))
	var blink_on := (int(state.get("title_timer", 0)) / 24) % 2 == 0
	_prompt_label.visible = phase == "main" and blink_on
	var entrance_alpha := 1.0
	if phase == "entrance":
		entrance_alpha = clamp(float(frame) / float(ENTRANCE_FRAMES), 0.0, 1.0)
	_logo_rect.modulate = Color(1.0, 1.0, 1.0, entrance_alpha)
	_suicune_rect.modulate = Color(1.0, 1.0, 1.0, entrance_alpha)
	_crystal_rect.modulate = Color(1.0, 1.0, 1.0, entrance_alpha)
	_copyright_label.modulate = Color(1.0, 1.0, 1.0, entrance_alpha)
	_prompt_label.modulate = Color(1.0, 1.0, 1.0, entrance_alpha)
