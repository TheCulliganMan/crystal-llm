extends RefCounted
class_name InputLatch

const BUTTONS := ["up", "down", "left", "right", "a", "b", "start", "select"]
const BUTTON_BITS := {
	"right": 0x01,
	"left": 0x02,
	"up": 0x04,
	"down": 0x08,
	"a": 0x10,
	"b": 0x20,
	"select": 0x40,
	"start": 0x80,
}

var _queued_events: Array[Dictionary] = []
var _held: Dictionary = {}
var _pressed: Dictionary = {}
var _released: Dictionary = {}
var _frame_index: int = 0
var _last_mask: int = 0
var _down_mask: int = 0
var _pressed_mask: int = 0
var _released_mask: int = 0

func _init() -> void:
	_reset_buttons()

func queue_button(button: String, is_pressed: bool) -> void:
	if not BUTTONS.has(button):
		return
	_queued_events.append({"button": button, "pressed": is_pressed})

func begin_frame() -> Dictionary:
	var frame_mask := _down_mask & 0xff
	var event_pressed_mask := 0
	var event_released_mask := 0
	for event in _queued_events:
		var button := str(event.get("button", ""))
		var is_pressed := bool(event.get("pressed", false))
		var bit := _button_bit(button)
		if bit == 0:
			continue
		if is_pressed:
			if (frame_mask & bit) == 0:
				event_pressed_mask = (event_pressed_mask | bit) & 0xff
			frame_mask = (frame_mask | bit) & 0xff
		else:
			if (frame_mask & bit) != 0:
				event_released_mask = (event_released_mask | bit) & 0xff
			frame_mask = (frame_mask & ~bit) & 0xff
	_queued_events.clear()
	_apply_frame_mask(frame_mask)
	_pressed_mask = (_pressed_mask | event_pressed_mask) & 0xff
	_released_mask = (_released_mask | event_released_mask) & 0xff
	_set_button_states_from_masks()
	return snapshot()

func is_down(button: String) -> bool:
	return bool(_held.get(button, false))

func is_held(button: String) -> bool:
	return bool(_held.get(button, false))

func is_pressed(button: String) -> bool:
	return bool(_pressed.get(button, false))

func is_released(button: String) -> bool:
	return bool(_released.get(button, false))

func down_mask() -> int:
	return _down_mask & 0xff

func pressed_mask() -> int:
	return _pressed_mask & 0xff

func released_mask() -> int:
	return _released_mask & 0xff

func frame_index() -> int:
	return _frame_index

func to_dictionary() -> Dictionary:
	return snapshot()

func from_dictionary(data: Variant) -> bool:
	if typeof(data) != TYPE_DICTIONARY:
		return false
	var source: Dictionary = data
	_reset_buttons()
	_frame_index = max(0, _coerce_int(source.get("frame_index", 0), 0))
	_last_mask = _coerce_int(source.get("last_mask", source.get("hJoyLast", 0)), 0) & 0xff
	var source_down_mask := _mask_from_source(source, "down_mask", "held")
	var source_pressed_mask := _mask_from_source(source, "pressed_mask", "pressed")
	var source_released_mask := _mask_from_source(source, "released_mask", "released")
	_down_mask = source_down_mask & 0xff
	_pressed_mask = source_pressed_mask & 0xff
	_released_mask = source_released_mask & 0xff
	if source.has("queued_events"):
		if typeof(source.get("queued_events")) != TYPE_ARRAY:
			return false
		var queued_events := Array(source.get("queued_events"))
		_queued_events = _sanitize_queued_events(queued_events)
		if _queued_events.size() != queued_events.size():
			return false
	_set_button_states_from_masks()
	return true

func snapshot() -> Dictionary:
	var held := _held.duplicate(true)
	return {
		"frame_index": _frame_index,
		"last_mask": _last_mask & 0xff,
		"held": held,
		"down": held.duplicate(true),
		"pressed": _pressed.duplicate(true),
		"released": _released.duplicate(true),
		"queued_events": _queued_events.duplicate(true),
		"pending_event_count": _queued_events.size(),
		"down_mask": _down_mask & 0xff,
		"pressed_mask": _pressed_mask & 0xff,
		"released_mask": _released_mask & 0xff,
		"hJoyDown": _down_mask & 0xff,
		"hJoyPressed": _pressed_mask & 0xff,
		"hJoyReleased": _released_mask & 0xff,
		"hJoypadDown": _down_mask & 0xff,
		"hJoypadPressed": _pressed_mask & 0xff,
		"hJoypadReleased": _released_mask & 0xff,
		"hJoyLast": _down_mask & 0xff,
		"hJoypadSum": _down_mask & 0xff,
	}

func joypad_snapshot() -> Dictionary:
	return {
		"hJoypadReleased": _released_mask & 0xff,
		"hJoypadPressed": _pressed_mask & 0xff,
		"hJoypadDown": _down_mask & 0xff,
		"hJoypadSum": _down_mask & 0xff,
		"hJoyReleased": _released_mask & 0xff,
		"hJoyPressed": _pressed_mask & 0xff,
		"hJoyDown": _down_mask & 0xff,
		"hJoyLast": _down_mask & 0xff,
	}

func _reset_buttons() -> void:
	_queued_events = []
	_held = {}
	_pressed = {}
	_released = {}
	for button in BUTTONS:
		_held[button] = false
		_pressed[button] = false
		_released[button] = false
	_frame_index = 0
	_last_mask = 0
	_down_mask = 0
	_pressed_mask = 0
	_released_mask = 0

func _apply_frame_mask(current_mask: int) -> void:
	current_mask = current_mask & 0xff
	var previous_mask := _down_mask & 0xff
	var delta := previous_mask ^ current_mask
	_pressed_mask = delta & current_mask
	_released_mask = delta & previous_mask
	_last_mask = previous_mask
	_down_mask = current_mask
	_frame_index += 1
	_set_button_states_from_masks()

func _set_button_states_from_masks() -> void:
	for button in BUTTONS:
		var bit := _button_bit(button)
		_held[button] = (_down_mask & bit) != 0
		_pressed[button] = (_pressed_mask & bit) != 0
		_released[button] = (_released_mask & bit) != 0

func _mask_from_source(source: Dictionary, mask_key: String, dictionary_key: String) -> int:
	if source.has(mask_key):
		return _coerce_int(source.get(mask_key), 0) & 0xff
	if source.has("hJoypad%s" % mask_key.trim_suffix("_mask").capitalize()):
		return _coerce_int(source.get("hJoypad%s" % mask_key.trim_suffix("_mask").capitalize()), 0) & 0xff
	if source.has("hJoy%s" % mask_key.trim_suffix("_mask").capitalize()):
		return _coerce_int(source.get("hJoy%s" % mask_key.trim_suffix("_mask").capitalize()), 0) & 0xff
	if source.has(dictionary_key) and typeof(source.get(dictionary_key)) == TYPE_DICTIONARY:
		return _mask_from_buttons(Dictionary(source.get(dictionary_key)))
	return 0

func _mask_from_buttons(buttons: Dictionary) -> int:
	var mask := 0
	for button in BUTTONS:
		if bool(buttons.get(button, false)):
			mask = (mask | _button_bit(button)) & 0xff
	return mask

func _sanitize_queued_events(events: Array) -> Array[Dictionary]:
	var sanitized: Array[Dictionary] = []
	for event in events:
		if typeof(event) != TYPE_DICTIONARY:
			return []
		var source: Dictionary = event
		var button := str(source.get("button", ""))
		if not BUTTONS.has(button):
			return []
		sanitized.append({
			"button": button,
			"pressed": bool(source.get("pressed", false)),
		})
	return sanitized

func _button_bit(button: String) -> int:
	return int(BUTTON_BITS.get(button, 0))

func _coerce_int(value: Variant, fallback: int) -> int:
	if typeof(value) == TYPE_NIL:
		return fallback
	return int(value)
