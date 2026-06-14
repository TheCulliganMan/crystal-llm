extends RefCounted
class_name RenderSnapshotState

const REQUIRED_FRAME_KEYS := [
	"viewport_size",
	"tilemap_layer_ids",
	"sprite_draw_order",
	"palette_bank",
	"animation_frame",
	"text_overlay",
	"menu_overlay",
]

var frame_golden_state: Dictionary = {}

func reset() -> void:
	frame_golden_state = {}

func capture_title_frame(payload: Variant) -> Dictionary:
	return capture_frame("title", payload)

func capture_intro_frame(payload: Variant) -> Dictionary:
	return capture_frame("intro", payload)

func capture_overworld_frame(payload: Variant) -> Dictionary:
	return capture_frame("overworld", payload)

func capture_menu_frame(payload: Variant) -> Dictionary:
	return capture_frame("menu", payload)

func capture_battle_frame(payload: Variant) -> Dictionary:
	return capture_frame("battle", payload)

func capture_frame(frame_name: String, payload: Variant) -> Dictionary:
	if typeof(payload) != TYPE_DICTIONARY:
		push_error("render snapshot payload must be a dictionary")
		return {}
	var source: Dictionary = Dictionary(payload)
	for key in REQUIRED_FRAME_KEYS:
		if not source.has(key):
			push_error("render snapshot payload missing %s" % str(key))
			return {}
	var normalized := _normalize_frame_payload(source)
	frame_golden_state[frame_name] = normalized.duplicate(true)
	return normalized.duplicate(true)

func get_frame(frame_name: String) -> Dictionary:
	return Dictionary(frame_golden_state.get(frame_name, {})).duplicate(true)

func has_frame(frame_name: String) -> bool:
	return frame_golden_state.has(frame_name) and not Dictionary(frame_golden_state.get(frame_name, {})).is_empty()

func get_frames() -> Dictionary:
	return frame_golden_state.duplicate(true)

func to_dictionary() -> Dictionary:
	return {
		"frame_golden_state": frame_golden_state.duplicate(true),
	}

func from_dictionary(data: Variant) -> bool:
	if typeof(data) != TYPE_DICTIONARY:
		return false
	var source: Dictionary = Dictionary(data)
	var raw_frames: Variant = source.get("frame_golden_state", source.get("frames", {}))
	frame_golden_state = {}
	if typeof(raw_frames) != TYPE_DICTIONARY:
		return true
	for frame_name in Dictionary(raw_frames).keys():
		var frame_payload: Dictionary = Dictionary(Dictionary(raw_frames).get(frame_name, {}))
		if frame_payload.is_empty():
			continue
		frame_golden_state[str(frame_name)] = _normalize_frame_payload(frame_payload)
	return true

func _normalize_frame_payload(payload: Dictionary) -> Dictionary:
	var normalized: Dictionary = Dictionary(_normalize_variant(payload))
	normalized["viewport_size"] = _normalize_viewport_size(normalized.get("viewport_size", {}))
	normalized["tilemap_layer_ids"] = _normalize_string_array(normalized.get("tilemap_layer_ids", []))
	normalized["sprite_draw_order"] = _normalize_string_array(normalized.get("sprite_draw_order", []))
	normalized["palette_bank"] = int(normalized.get("palette_bank", 0))
	normalized["animation_frame"] = int(normalized.get("animation_frame", 0))
	normalized["text_overlay"] = _normalize_variant(normalized.get("text_overlay", {}))
	normalized["menu_overlay"] = _normalize_variant(normalized.get("menu_overlay", {}))
	return normalized

func _normalize_viewport_size(value: Variant) -> Dictionary:
	if typeof(value) == TYPE_DICTIONARY:
		var source: Dictionary = Dictionary(value)
		return {
			"width": int(source.get("width", 0)),
			"height": int(source.get("height", 0)),
		}
	if typeof(value) == TYPE_ARRAY:
		var array_value: Array = Array(value)
		if array_value.size() >= 2:
			return {
				"width": int(array_value[0]),
				"height": int(array_value[1]),
			}
	return {
		"width": 0,
		"height": 0,
	}

func _normalize_string_array(value: Variant) -> Array:
	var normalized: Array = []
	if typeof(value) != TYPE_ARRAY:
		return normalized
	for entry in Array(value):
		normalized.append(str(entry))
	return normalized

func _normalize_variant(value: Variant) -> Variant:
	match typeof(value):
		TYPE_DICTIONARY:
			var normalized: Dictionary = {}
			for key in Dictionary(value).keys():
				normalized[key] = _normalize_variant(Dictionary(value).get(key))
			return normalized
		TYPE_ARRAY:
			var normalized_array: Array = []
			for entry in Array(value):
				normalized_array.append(_normalize_variant(entry))
			return normalized_array
		_:
			return value
