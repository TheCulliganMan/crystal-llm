extends RefCounted
class_name StoryEventsState

var state: Dictionary = {}

func _init() -> void:
	reset()

func reset() -> void:
	state = {
		"flags": {},
		"variables": {},
		"script_queue": [],
		"movement_queue": [],
		"text_queue": [],
		"audio_queue": [],
		"warp_queue": [],
		"battle_queue": [],
		"last_command": {},
		"waiting_for_input": false,
		"runner_paused": false,
		"command_index": 0,
	}

func from_dictionary(snapshot: Dictionary) -> bool:
	reset()
	state = _merge_dictionary(state, snapshot)
	_normalize_state()
	return true

func to_dictionary() -> Dictionary:
	_normalize_state()
	return state.duplicate(true)

func enqueue_script(commands: Array) -> Dictionary:
	_normalize_state()
	var normalized: Array[Dictionary] = []
	for command_value in commands:
		if typeof(command_value) == TYPE_DICTIONARY:
			normalized.append(_normalize_command(Dictionary(command_value)))
	state["script_queue"] = normalized
	state["command_index"] = 0
	state["runner_paused"] = false
	return get_runner_state()

func step() -> Dictionary:
	_normalize_state()
	if bool(state.get("runner_paused", false)) or bool(state.get("waiting_for_input", false)):
		return {
			"executed": false,
			"reason": "blocked",
			"runner": get_runner_state(),
		}
	var queue: Array = Array(state.get("script_queue", []))
	var command_index := int(state.get("command_index", 0))
	if command_index < 0 or command_index >= queue.size():
		return {
			"executed": false,
			"reason": "complete",
			"runner": get_runner_state(),
		}
	var command: Dictionary = _normalize_command(Dictionary(queue[command_index]))
	state["command_index"] = command_index + 1
	var result := execute_command(command)
	result["runner"] = get_runner_state()
	return result

func execute_command(command: Dictionary) -> Dictionary:
	var normalized := _normalize_command(command)
	var op := str(normalized.get("op", "")).to_lower()
	state["last_command"] = normalized.duplicate(true)
	match op:
		"setflag":
			_set_flag(str(normalized.get("flag", "")), true)
			return _command_result(op, {"flag": str(normalized.get("flag", "")), "value": true})
		"clearflag":
			_set_flag(str(normalized.get("flag", "")), false)
			return _command_result(op, {"flag": str(normalized.get("flag", "")), "value": false})
		"checkflag":
			return _command_result(op, {"flag": str(normalized.get("flag", "")), "value": get_flag(str(normalized.get("flag", "")))})
		"setvar":
			_set_variable(str(normalized.get("name", "")), normalized.get("value"))
			return _command_result(op, {"name": str(normalized.get("name", "")), "value": normalized.get("value")})
		"loadvar":
			_set_variable(str(normalized.get("name", "")), normalized.get("value"))
			return _command_result(op, {"name": str(normalized.get("name", "")), "value": normalized.get("value")})
		"jump":
			state["command_index"] = clampi(int(normalized.get("target", state.get("command_index", 0))), 0, Array(state.get("script_queue", [])).size())
			return _command_result(op, {"target": int(state.get("command_index", 0))})
		"pause":
			state["runner_paused"] = true
			return _command_result(op, {"paused": true})
		"resume":
			state["runner_paused"] = false
			return _command_result(op, {"paused": false})
		"jumptext", "writetext", "text":
			return _queue_text(normalized)
		"yesorno":
			state["waiting_for_input"] = true
			return _command_result(op, {"waiting_for_input": true, "prompt": str(normalized.get("text", ""))})
		"applymovement", "follow", "stopfollow":
			return _queue_movement(normalized)
		"warp", "warpfacing", "refreshmap":
			return _queue_warp(normalized)
		"trainerbattle", "startbattle", "loadtrainer":
			return _queue_battle(normalized)
		"playmusic", "musicfadeout", "playsound", "cry":
			return _queue_audio(normalized)
		"giveitem", "takeitem":
			return _queue_item(normalized)
		"end", "endcallback":
			state["command_index"] = Array(state.get("script_queue", [])).size()
			return _command_result(op, {"complete": true})
		_:
			return _command_result(op, {"intent": "script_command", "payload": normalized.duplicate(true)})

func answer_yes_no(answer: bool) -> Dictionary:
	state["waiting_for_input"] = false
	state["variables"]["last_yes_no"] = answer
	return {
		"answered": true,
		"answer": answer,
		"runner": get_runner_state(),
	}

func get_runner_state() -> Dictionary:
	return {
		"command_index": int(state.get("command_index", 0)),
		"queue_length": Array(state.get("script_queue", [])).size(),
		"waiting_for_input": bool(state.get("waiting_for_input", false)),
		"runner_paused": bool(state.get("runner_paused", false)),
		"last_command": Dictionary(state.get("last_command", {})).duplicate(true),
	}

func get_flag(flag: String) -> bool:
	return bool(Dictionary(state.get("flags", {})).get(flag, false))

func _queue_text(command: Dictionary) -> Dictionary:
	var text_queue: Array = Array(state.get("text_queue", []))
	var payload := {
		"intent": "text",
		"op": str(command.get("op", "")),
		"text": str(command.get("text", command.get("label", ""))),
		"speaker": str(command.get("speaker", "")),
	}
	text_queue.append(payload)
	state["text_queue"] = text_queue
	state["waiting_for_input"] = bool(command.get("wait", true))
	return _command_result(str(command.get("op", "")), payload)

func _queue_movement(command: Dictionary) -> Dictionary:
	var movement_queue: Array = Array(state.get("movement_queue", []))
	var payload := {
		"intent": "movement",
		"op": str(command.get("op", "")),
		"object_id": str(command.get("object_id", command.get("target", ""))),
		"steps": Array(command.get("steps", [])).duplicate(true),
	}
	movement_queue.append(payload)
	state["movement_queue"] = movement_queue
	return _command_result(str(command.get("op", "")), payload)

func _queue_warp(command: Dictionary) -> Dictionary:
	var warp_queue: Array = Array(state.get("warp_queue", []))
	var payload := {
		"intent": "warp",
		"op": str(command.get("op", "")),
		"map": str(command.get("map", command.get("map_id", ""))),
		"x": int(command.get("x", 0)),
		"y": int(command.get("y", 0)),
		"facing": str(command.get("facing", "")),
	}
	warp_queue.append(payload)
	state["warp_queue"] = warp_queue
	return _command_result(str(command.get("op", "")), payload)

func _queue_battle(command: Dictionary) -> Dictionary:
	var battle_queue: Array = Array(state.get("battle_queue", []))
	var payload := {
		"intent": "battle",
		"op": str(command.get("op", "")),
		"trainer": str(command.get("trainer", "")),
		"battle_type": str(command.get("battle_type", command.get("kind", ""))),
	}
	battle_queue.append(payload)
	state["battle_queue"] = battle_queue
	return _command_result(str(command.get("op", "")), payload)

func _queue_audio(command: Dictionary) -> Dictionary:
	var audio_queue: Array = Array(state.get("audio_queue", []))
	var payload := {
		"intent": "audio",
		"op": str(command.get("op", "")),
		"cue": str(command.get("cue", command.get("music", command.get("sound", "")))),
		"fade_frames": int(command.get("fade_frames", 0)),
	}
	audio_queue.append(payload)
	state["audio_queue"] = audio_queue
	return _command_result(str(command.get("op", "")), payload)

func _queue_item(command: Dictionary) -> Dictionary:
	var item_id := str(command.get("item", command.get("item_id", ""))).to_upper()
	var quantity := maxi(1, int(command.get("quantity", 1)))
	var items: Dictionary = Dictionary(state.get("variables", {})).get("items", {})
	var current := int(Dictionary(items).get(item_id, 0))
	if str(command.get("op", "")).to_lower() == "takeitem":
		Dictionary(items)[item_id] = maxi(0, current - quantity)
	else:
		Dictionary(items)[item_id] = current + quantity
	var variables: Dictionary = Dictionary(state.get("variables", {}))
	variables["items"] = Dictionary(items).duplicate(true)
	state["variables"] = variables
	return _command_result(str(command.get("op", "")), {"intent": "item", "item": item_id, "quantity": quantity})

func _command_result(op: String, payload: Dictionary) -> Dictionary:
	return {
		"executed": true,
		"op": op,
		"payload": payload,
	}

func _set_flag(flag: String, value: bool) -> void:
	if flag.is_empty():
		return
	var flags: Dictionary = Dictionary(state.get("flags", {}))
	flags[flag] = value
	state["flags"] = flags

func _set_variable(name: String, value: Variant) -> void:
	if name.is_empty():
		return
	var variables: Dictionary = Dictionary(state.get("variables", {}))
	variables[name] = value
	state["variables"] = variables

func _normalize_state() -> void:
	state["flags"] = Dictionary(state.get("flags", {}))
	state["variables"] = Dictionary(state.get("variables", {}))
	state["script_queue"] = Array(state.get("script_queue", [])).duplicate(true)
	state["movement_queue"] = Array(state.get("movement_queue", [])).duplicate(true)
	state["text_queue"] = Array(state.get("text_queue", [])).duplicate(true)
	state["audio_queue"] = Array(state.get("audio_queue", [])).duplicate(true)
	state["warp_queue"] = Array(state.get("warp_queue", [])).duplicate(true)
	state["battle_queue"] = Array(state.get("battle_queue", [])).duplicate(true)
	state["last_command"] = Dictionary(state.get("last_command", {})).duplicate(true)
	state["waiting_for_input"] = bool(state.get("waiting_for_input", false))
	state["runner_paused"] = bool(state.get("runner_paused", false))
	state["command_index"] = maxi(0, int(state.get("command_index", 0)))

func _normalize_command(command: Dictionary) -> Dictionary:
	var normalized := command.duplicate(true)
	normalized["op"] = str(normalized.get("op", normalized.get("command", ""))).to_lower()
	return normalized

func _merge_dictionary(base: Dictionary, overlay: Dictionary) -> Dictionary:
	var merged := base.duplicate(true)
	for key in overlay.keys():
		if typeof(merged.get(key)) == TYPE_DICTIONARY and typeof(overlay.get(key)) == TYPE_DICTIONARY:
			merged[key] = _merge_dictionary(Dictionary(merged.get(key)), Dictionary(overlay.get(key)))
		else:
			merged[key] = overlay.get(key)
	return merged
