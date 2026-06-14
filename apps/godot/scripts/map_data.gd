extends RefCounted
class_name MapData

signal changed

const DEFAULT_MAP_NAME := "NewBarkTown"
const DEFAULT_SPAWN_PRIORITY := ["NEW_BARK_TOWN", "CHERRYGROVE_CITY", "OLIVINE_CITY"]
const ASSET_INDEX_SCRIPT := preload("res://scripts/asset_index.gd")

var asset_index = ASSET_INDEX_SCRIPT.new()
var asset_summary: Dictionary = {}
var runtime_map_metadata: Dictionary = {}
var runtime_spawn_points: Dictionary = {}
var map_attributes: Dictionary = {}
var map_blocks: Dictionary = {}
var collision_permissions: Array = []
var collision_stdscripts: Dictionary = {}

var current_map_key: String = ""
var current_map_name: String = ""
var current_map_constant: String = ""
var current_map_group_name: String = ""
var current_group_id: int = -1
var current_map_group_id: int = -1
var current_map_id: int = -1
var current_environment: String = ""
var current_music: String = ""
var current_tileset_name: String = ""
var current_location: String = ""
var current_phone_service: int = 0
var current_width: int = 0
var current_height: int = 0
var current_border_block: int = 0
var current_connections: Array = []
var current_coord_events: Array = []
var current_warps: Array = []
var current_bg_events: Array = []
var current_object_events: Array = []
var current_spawn_points: Array = []
var current_spawn_point: Dictionary = {}
var current_blocks_label: String = ""
var current_map_events_label: String = ""
var current_map_scripts_label: String = ""
var current_map_payload: Dictionary = {}
var current_map_summary: Dictionary = {}

var _map_key_by_identifier: Dictionary = {}
var _map_key_by_name: Dictionary = {}
var _map_key_by_constant: Dictionary = {}
var _map_key_by_group: Dictionary = {}
var _loading_assets: bool = false
var _assets_refreshed: bool = false

func _init() -> void:
	reset()

func reset() -> void:
	asset_summary = {}
	runtime_map_metadata = {}
	runtime_spawn_points = {}
	map_attributes = {}
	map_blocks = {}
	collision_permissions = []
	collision_stdscripts = {}
	current_map_key = ""
	current_map_name = ""
	current_map_constant = ""
	current_map_group_name = ""
	current_group_id = -1
	current_map_group_id = -1
	current_map_id = -1
	current_environment = ""
	current_music = ""
	current_tileset_name = ""
	current_location = ""
	current_phone_service = 0
	current_width = 0
	current_height = 0
	current_border_block = 0
	current_connections = []
	current_coord_events = []
	current_warps = []
	current_bg_events = []
	current_object_events = []
	current_spawn_points = []
	current_spawn_point = {}
	current_blocks_label = ""
	current_map_events_label = ""
	current_map_scripts_label = ""
	current_map_payload = {}
	current_map_summary = _build_summary()
	_map_key_by_identifier = {}
	_map_key_by_name = {}
	_map_key_by_constant = {}
	_map_key_by_group = {}
	_loading_assets = false
	_assets_refreshed = false

func set_asset_index(index) -> void:
	if index != null:
		asset_index = index

func _load_dictionary(relative_path: String) -> Dictionary:
	if asset_index != null and asset_index.has_method("load_dictionary"):
		var loaded: Variant = asset_index.call("load_dictionary", relative_path)
		if typeof(loaded) == TYPE_DICTIONARY:
			return Dictionary(loaded).duplicate(true)
	return {}

func _load_array(relative_path: String) -> Array:
	if asset_index != null and asset_index.has_method("load_array"):
		var loaded: Variant = asset_index.call("load_array", relative_path)
		if typeof(loaded) == TYPE_ARRAY:
			return Array(loaded).duplicate(true)
	return []

func refresh_assets() -> void:
	if _loading_assets:
		return
	_ensure_asset_index()
	_loading_assets = true
	asset_index.initialize()
	asset_summary = asset_index.load_summary()
	runtime_map_metadata = _load_dictionary("runtime_map_metadata.json")
	runtime_spawn_points = _load_dictionary("runtime_spawn_points.json")
	map_attributes = _load_dictionary("map_attributes.json")
	map_blocks = _load_dictionary("map_blocks.json")
	collision_permissions = _load_array("collision/collision_permissions.json")
	collision_stdscripts = _load_dictionary("collision/collision_stdscripts.json")
	_rebuild_lookup_tables()
	_loading_assets = false
	_assets_refreshed = true
	if current_map_key.is_empty() or not _has_map_identifier(current_map_key):
		if not load_default_map():
			_clear_current_map()
	emit_signal("changed")

func load_default_map() -> bool:
	_ensure_assets_loaded()
	var spawn := _choose_default_spawn_point()
	if not spawn.is_empty():
		return apply_spawn_point(spawn)
	if not current_map_key.is_empty() and _has_map_identifier(current_map_key):
		return load_map(current_map_key)
	return load_map(DEFAULT_MAP_NAME)

func select_map(identifier: Variant) -> bool:
	_ensure_assets_loaded()
	var resolved := _resolve_map_key(identifier)
	if resolved.is_empty():
		if identifier is String:
			var candidate := _coerce_string(identifier, "")
			if candidate.is_empty() and not current_map_key.is_empty():
				resolved = current_map_key
		elif identifier is Dictionary:
			var payload: Dictionary = identifier
			if payload.has("map_key"):
				resolved = _resolve_map_key(payload.get("map_key", ""))
			elif payload.has("map_name"):
				resolved = _resolve_map_key(payload.get("map_name", ""))
			elif payload.has("map_constant"):
				resolved = _resolve_map_key(payload.get("map_constant", ""))
			elif payload.has("constant"):
				resolved = _resolve_map_key(payload.get("constant", ""))
			elif payload.has("group_id") and payload.has("map_id"):
				resolved = _resolve_map_key({
					"group_id": payload.get("group_id", -1),
					"map_id": payload.get("map_id", -1),
				})
		if resolved.is_empty():
			return false
	return _apply_selected_map(resolved)

func load_map(map_name: String) -> bool:
	return select_map(map_name)

func load_runtime_map_metadata() -> Dictionary:
	_ensure_assets_loaded()
	return runtime_map_metadata.duplicate(true)

func _load_map_file_payload(map_name: String) -> Dictionary:
	if map_name.is_empty():
		return {}
	if asset_index != null and asset_index.has_method("load_map_file"):
		var loaded: Variant = asset_index.call("load_map_file", map_name)
		return _dictionary_from_export(loaded, "maps/%s.json" % map_name)
	return _load_dictionary("maps/%s.json" % map_name)

func _dictionary_from_export(value: Variant, relative_path: String = "") -> Dictionary:
	if typeof(value) == TYPE_DICTIONARY:
		var source: Dictionary = Dictionary(value)
		var unwrapped: Variant = _unwrap_export_dictionary(source, relative_path)
		var unwrapped_dictionary: Dictionary = _sanitize_dictionary(unwrapped, {})
		if not unwrapped_dictionary.is_empty() and unwrapped_dictionary != source:
			return _dictionary_from_export(unwrapped_dictionary, relative_path)
		var unwrapped_array: Array = _sanitize_array(unwrapped, [])
		if not unwrapped_array.is_empty():
			return _dictionary_from_export(unwrapped_array, relative_path)
		return source.duplicate(true)
	if typeof(value) == TYPE_ARRAY:
		var records: Dictionary = {}
		var index: int = 0
		for item in Array(value):
			var item_dict: Dictionary = _sanitize_dictionary(item, {})
			if item_dict.is_empty():
				index += 1
				continue
			var key: String = _coerce_string(_first_present(item_dict, ["key", "id", "identifier", "name", "map_name", "mapName", "constant", "map_constant", "mapConstant"]), "")
			if key.is_empty():
				key = str(index)
			records[key] = item_dict
			index += 1
		return records
	return {}

func _unwrap_export_dictionary(source: Dictionary, relative_path: String) -> Variant:
	var stem: String = relative_path.get_file().get_basename()
	var unwrap_keys: Array = [
		stem,
		"data",
		"records",
		"entries",
		"maps",
		"map_attributes",
		"mapAttributes",
		"runtime_map_metadata",
		"runtimeMapMetadata",
		"runtime_spawn_points",
		"runtimeSpawnPoints",
		"map_blocks",
		"mapBlocks",
	]
	for key in unwrap_keys:
		if source.has(key):
			var candidate: Variant = source.get(key)
			if typeof(candidate) == TYPE_DICTIONARY or typeof(candidate) == TYPE_ARRAY:
				return candidate
	return source

func _has_event_collection(source: Dictionary) -> bool:
	for key in ["coord_events", "coordEvents", "warps", "bg_events", "bgEvents", "object_events", "objectEvents", "objects"]:
		if source.has(key):
			return true
	return false

func _normalize_event_collection(source: Dictionary) -> Dictionary:
	return {
		"coord_events": _normalize_coord_events(_first_present(source, ["coord_events", "coordEvents"])),
		"warps": _normalize_warps(_first_present(source, ["warps", "warp_events", "warpEvents"])),
		"bg_events": _normalize_bg_events(_first_present(source, ["bg_events", "bgEvents", "background_events", "backgroundEvents"])),
		"object_events": _normalize_object_events(_first_present(source, ["object_events", "objectEvents", "objects"])),
		"map_events_label": _coerce_string(_first_present(source, ["map_events_label", "mapEventsLabel"]), ""),
		"map_scripts_label": _coerce_string(_first_present(source, ["map_scripts_label", "mapScriptsLabel"]), ""),
	}

func _normalize_runtime_entry(entry: Variant, fallback_key: String = "") -> Dictionary:
	var source: Dictionary = _sanitize_dictionary(entry, {})
	if source.is_empty():
		return {}
	var constant := _coerce_string(_first_present(source, ["constant", "mapConstant", "map_constant"]), fallback_key)
	var map_name := _coerce_string(_first_present(source, ["name", "mapName", "map_name"]), "")
	return {
		"constant": constant,
		"map_constant": constant,
		"mapConstant": constant,
		"name": map_name,
		"map_name": map_name,
		"mapName": map_name,
		"groupName": _coerce_string(_first_present(source, ["groupName", "group_name"]), ""),
		"group_name": _coerce_string(_first_present(source, ["groupName", "group_name"]), ""),
		"groupId": _coerce_int(_first_present(source, ["groupId", "group_id"]), -1),
		"group_id": _coerce_int(_first_present(source, ["groupId", "group_id"]), -1),
		"mapId": _coerce_int(_first_present(source, ["mapId", "map_id"]), -1),
		"map_id": _coerce_int(_first_present(source, ["mapId", "map_id"]), -1),
		"width": _coerce_int(_first_present(source, ["width"]), 0),
		"height": _coerce_int(_first_present(source, ["height"]), 0),
		"environment": _coerce_string(_first_present(source, ["environment"]), ""),
		"phoneService": _coerce_int(_first_present(source, ["phoneService", "phone_service"]), 0),
		"phone_service": _coerce_int(_first_present(source, ["phoneService", "phone_service"]), 0),
	}

func _normalize_map_attribute_entry(entry: Variant, fallback_name: String = "") -> Dictionary:
	var source: Dictionary = _sanitize_dictionary(entry, {})
	if source.is_empty():
		return {}
	var map_name := _coerce_string(_first_present(source, ["name", "map_name", "mapName"]), fallback_name)
	var map_constant := _coerce_string(_first_present(source, ["map_constant", "constant", "mapConstant"]), "")
	var group_constant := _coerce_string(_first_present(source, ["map_group_constant", "groupName", "group_name"]), "")
	var group_id := _coerce_int(_first_present(source, ["group_id", "groupId"]), -1)
	var map_id := _coerce_int(_first_present(source, ["map_id", "mapId"]), -1)
	return {
		"name": map_name if not map_name.is_empty() else map_constant,
		"map_name": map_name if not map_name.is_empty() else map_constant,
		"mapName": map_name if not map_name.is_empty() else map_constant,
		"map_constant": map_constant,
		"mapConstant": map_constant,
		"map_group_constant": group_constant,
		"group_name": group_constant,
		"groupName": group_constant,
		"group_id": group_id,
		"groupId": group_id,
		"map_id": map_id,
		"mapId": map_id,
		"border_block": _coerce_int(_first_present(source, ["border_block", "borderBlock"]), 0),
		"environment": _coerce_string(_first_present(source, ["environment"]), ""),
		"location": _coerce_string(_first_present(source, ["location"]), ""),
		"music": _coerce_string(_first_present(source, ["music"]), ""),
		"tileset_name": _coerce_string(_first_present(source, ["tileset_name", "tilesetName"]), ""),
		"phone_service": _coerce_int(_first_present(source, ["phone_service", "phoneService"]), 0),
		"phone_flag": _coerce_bool(_first_present(source, ["phone_flag", "phoneFlag"]), false),
		"width": _coerce_int(_first_present(source, ["width"]), 0),
		"height": _coerce_int(_first_present(source, ["height"]), 0),
		"connections": _sanitize_array(_first_present(source, ["connections"]), []),
		"time_of_day": _coerce_string(_first_present(source, ["time_of_day", "timeOfDay"]), ""),
		"palette": _coerce_string(_first_present(source, ["palette"]), ""),
		"fishing_group": _coerce_string(_first_present(source, ["fishing_group", "fishingGroup"]), ""),
		"blocks_label": _coerce_string(_first_present(source, ["blocks_label", "blocksLabel"]), ""),
		"map_events_label": _coerce_string(_first_present(source, ["map_events_label", "mapEventsLabel"]), ""),
		"map_scripts_label": _coerce_string(_first_present(source, ["map_scripts_label", "mapScriptsLabel"]), ""),
		"connection_flags": _coerce_string(_first_present(source, ["connection_flags", "connectionFlags"]), ""),
	}

func load_runtime_spawn_points() -> Dictionary:
	_ensure_assets_loaded()
	return runtime_spawn_points.duplicate(true)

func load_map_attributes() -> Dictionary:
	_ensure_assets_loaded()
	return map_attributes.duplicate(true)

func load_map_blocks() -> Dictionary:
	_ensure_assets_loaded()
	return map_blocks.duplicate(true)

func load_map_file(map_name: String) -> Dictionary:
	_ensure_assets_loaded()
	return _load_map_file_payload(map_name)

func get_map_key(identifier: Variant) -> String:
	_ensure_assets_loaded()
	return _resolve_map_key(identifier)

func get_map_name(identifier: Variant) -> String:
	_ensure_assets_loaded()
	return _resolve_map_name(identifier)

func get_map_constant(identifier: Variant) -> String:
	_ensure_assets_loaded()
	return _resolve_map_constant(identifier)

func get_map_group_ids(identifier: Variant) -> Dictionary:
	_ensure_assets_loaded()
	var runtime_entry := _find_runtime_entry(identifier)
	var attribute_entry := _find_attribute_entry(identifier)
	var group_id := _coerce_int(_first_present(runtime_entry, ["groupId", "group_id"]), -1)
	if group_id < 0:
		group_id = _coerce_int(_first_present(attribute_entry, ["group_id", "groupId"]), -1)
	var map_id := _coerce_int(_first_present(runtime_entry, ["mapId", "map_id"]), -1)
	if map_id < 0:
		map_id = _coerce_int(_first_present(attribute_entry, ["map_id", "mapId"]), -1)
	var group_name := _coerce_string(_first_present(runtime_entry, ["groupName", "group_name"]), "")
	if group_name.is_empty():
		group_name = _coerce_string(_first_present(attribute_entry, ["map_group_constant", "groupName", "group_name"]), "")
	return {
		"group_id": group_id,
		"groupId": group_id,
		"map_id": map_id,
		"mapId": map_id,
		"group_name": group_name,
		"groupName": group_name,
	}

func get_map_lookup(identifier: Variant) -> Dictionary:
	_ensure_assets_loaded()
	var map_key := _resolve_map_key(identifier)
	var map_name := _resolve_map_name(identifier)
	return {
		"map_key": map_key,
		"map_name": map_name,
		"map_constant": _resolve_map_constant(identifier),
		"group_ids": get_map_group_ids(identifier),
		"runtime_metadata": _sanitize_dictionary(_find_runtime_entry(identifier), {}),
		"map_attributes": _sanitize_dictionary(_find_attribute_entry(identifier), {}),
	}

func get_maps_for_group_id(group_id: int) -> Array:
	_ensure_assets_loaded()
	var matches: Array = []
	for key in runtime_map_metadata.keys():
		var entry := _normalize_runtime_entry(runtime_map_metadata.get(key, {}), _coerce_string(key, ""))
		if _coerce_int(_first_present(entry, ["groupId", "group_id"]), -1) == group_id:
			matches.append(get_map_lookup(entry))
	if matches.is_empty():
		for key in map_attributes.keys():
			var entry := _normalize_map_attribute_entry(map_attributes.get(key, {}), _coerce_string(key, ""))
			if _coerce_int(_first_present(entry, ["group_id", "groupId"]), -1) == group_id:
				matches.append(get_map_lookup(entry))
	return matches

func select_map_by_group_ids(group_id: int, map_id: int) -> bool:
	return select_map({
		"group_id": group_id,
		"map_id": map_id,
	})

func get_map_payload(map_identifier: Variant) -> Dictionary:
	return build_map_payload(map_identifier)

func get_current_payload() -> Dictionary:
	return current_map_payload.duplicate(true)

func serialize_current_map() -> Dictionary:
	var payload := current_map_payload.duplicate(true)
	payload["selection"] = get_current_selection()
	payload["summary"] = get_current_summary()
	payload["events"] = get_current_events()
	payload["tileset_metadata"] = get_current_tileset_metadata()
	payload["block_metadata"] = get_current_block_metadata()
	payload["manifest_entry"] = get_current_manifest_entry()
	payload["event_metadata"] = get_current_event_metadata()
	payload["script_metadata"] = get_current_map_script_metadata()
	return payload

func get_events_for_map(map_identifier: Variant) -> Dictionary:
	return _normalize_event_collection(build_map_payload(map_identifier))

func get_current_events() -> Dictionary:
	return {
		"coord_events": current_coord_events.duplicate(true),
		"warps": current_warps.duplicate(true),
		"bg_events": current_bg_events.duplicate(true),
		"object_events": current_object_events.duplicate(true),
	}

func get_current_warps() -> Array:
	return current_warps.duplicate(true)

func get_current_coord_events() -> Array:
	return current_coord_events.duplicate(true)

func get_current_bg_events() -> Array:
	return current_bg_events.duplicate(true)

func get_current_object_events() -> Array:
	return current_object_events.duplicate(true)

func load_collision_permissions() -> Array:
	_ensure_assets_loaded()
	return collision_permissions.duplicate(true)

func load_collision_stdscripts() -> Dictionary:
	_ensure_assets_loaded()
	return collision_stdscripts.duplicate(true)

func get_collision_permission(value: Variant) -> Dictionary:
	_ensure_assets_loaded()
	var normalized_value: int = _resolve_collision_value(value)
	for entry in collision_permissions:
		var record: Dictionary = _sanitize_dictionary(entry, {})
		if _coerce_int(record.get("value", -1), -1) == normalized_value:
			return _normalize_collision_permission(record)
	return {}

func get_collision_stdscript(value: Variant) -> String:
	_ensure_assets_loaded()
	var key: String = _collision_constant_name(value)
	if key.is_empty():
		return ""
	return _coerce_string(collision_stdscripts.get(key, ""), "")

func load_tileset_collision(tileset_identifier: Variant = "") -> Dictionary:
	var tileset_name: String = _resolve_tileset_name(tileset_identifier)
	if tileset_name.is_empty():
		return {}
	if asset_index != null and asset_index.has_method("load_tileset_collision"):
		return _sanitize_dictionary(asset_index.call("load_tileset_collision", tileset_name), {})
	return _load_dictionary("tilesets/%s.json" % tileset_name)

func load_tileset_palette_map(tileset_identifier: Variant = "") -> Variant:
	var tileset_name: String = _resolve_tileset_name(tileset_identifier)
	if tileset_name.is_empty():
		return []
	if asset_index != null and asset_index.has_method("load_tileset_palette_map"):
		return _normalize_variant(asset_index.call("load_tileset_palette_map", tileset_name))
	return _normalize_variant(asset_index.load_manifest("tilesets/%s_palette_map.json" % tileset_name))

func get_tileset_metadata(tileset_identifier: Variant = "") -> Dictionary:
	_ensure_assets_loaded()
	var tileset_name: String = _resolve_tileset_name(tileset_identifier)
	if tileset_name.is_empty():
		return {}
	var collision: Dictionary = load_tileset_collision(tileset_name)
	var palette_map: Variant = load_tileset_palette_map(tileset_name)
	var metatile_bytes: PackedByteArray = _load_tileset_metatiles(tileset_name)
	var metadata: Dictionary = {
		"tileset_name": tileset_name,
		"collision_count": collision.size(),
		"palette_count": _count_entries(palette_map),
		"metatile_bytes": metatile_bytes.size(),
		"metatile_count": int(metatile_bytes.size() / 16) if metatile_bytes.size() > 0 and metatile_bytes.size() % 16 == 0 else 0,
		"collision": collision.duplicate(true),
		"palette_map": _normalize_variant(palette_map),
	}
	if asset_index != null and asset_index.has_method("load_tileset_metadata"):
		metadata.merge(_sanitize_dictionary(asset_index.call("load_tileset_metadata", tileset_name), {}), true)
	return metadata

func get_current_tileset_metadata() -> Dictionary:
	return get_tileset_metadata(current_tileset_name)

func get_metatile_collision(metatile_id: int, tileset_identifier: Variant = "") -> Dictionary:
	var collision: Dictionary = load_tileset_collision(tileset_identifier)
	var key: String = _metatile_key(metatile_id)
	var raw_collision: Array = _sanitize_array(collision.get(key, collision.get(str(metatile_id), [])), [])
	var permissions: Array = []
	for entry in raw_collision:
		permissions.append(get_collision_permission(entry))
	return {
		"metatile_id": metatile_id,
		"key": key,
		"collision": raw_collision,
		"permissions": permissions,
	}

func get_block_metadata(map_identifier: Variant = "") -> Dictionary:
	_ensure_assets_loaded()
	var payload: Dictionary = current_map_payload if _is_current_map_identifier(map_identifier) else build_map_payload(map_identifier)
	var label: String = _coerce_string(payload.get("blocks_label", current_blocks_label), current_blocks_label)
	var encoded: String = _coerce_string(map_blocks.get(label, payload.get("blocks_data", "")), "")
	var bytes: PackedByteArray = _decode_block_bytes(encoded)
	var width: int = max(0, _coerce_int(payload.get("width", current_width), current_width))
	var height: int = max(0, _coerce_int(payload.get("height", current_height), current_height))
	return {
		"map_key": _coerce_string(payload.get("map_key", current_map_key), current_map_key),
		"map_name": _coerce_string(payload.get("map_name", current_map_name), current_map_name),
		"tileset_name": _coerce_string(payload.get("tileset_name", current_tileset_name), current_tileset_name),
		"blocks_label": label,
		"encoded": encoded,
		"bytes": _packed_bytes_to_array(bytes),
		"byte_count": bytes.size(),
		"block_count": bytes.size(),
		"width": width,
		"height": height,
		"expected_block_count": width * height,
	}

func get_current_block_metadata() -> Dictionary:
	return get_block_metadata(current_map_key)

func get_block_at_tile(x: int, y: int, map_identifier: Variant = "") -> Dictionary:
	var metadata: Dictionary = get_block_metadata(map_identifier)
	var width: int = _coerce_int(metadata.get("width", 0), 0)
	var height: int = _coerce_int(metadata.get("height", 0), 0)
	var bytes: Array = _sanitize_array(metadata.get("bytes", []), [])
	if x < 0 or y < 0 or width <= 0 or height <= 0 or x >= width or y >= height:
		return {}
	var index: int = y * width + x
	if index < 0 or index >= bytes.size():
		return {}
	var block_id: int = _coerce_int(bytes[index], 0)
	return {
		"x": x,
		"y": y,
		"index": index,
		"block_id": block_id,
		"metatile": get_metatile_collision(block_id, _coerce_string(_first_present(metadata, ["tileset_name"]), current_tileset_name)),
	}

func get_warp_by_id(warp_id: int, map_identifier: Variant = "") -> Dictionary:
	var events: Dictionary = current_map_payload if _is_current_map_identifier(map_identifier) else build_map_payload(map_identifier)
	for warp in _normalize_warps(events.get("warps", [])):
		var entry: Dictionary = Dictionary(warp)
		if _coerce_int(entry.get("warp_id", -1), -1) == warp_id or _coerce_int(entry.get("index", -1), -1) == warp_id:
			return entry
	return {}

func get_warp_targets(map_identifier: Variant = "") -> Dictionary:
	var events: Dictionary = current_map_payload if _is_current_map_identifier(map_identifier) else build_map_payload(map_identifier)
	var targets: Dictionary = {}
	for warp in _normalize_warps(events.get("warps", [])):
		var entry: Dictionary = Dictionary(warp)
		targets[str(entry.get("warp_id", entry.get("index", 0)))] = entry
	return targets

func get_events_at_tile(x: int, y: int, map_identifier: Variant = "") -> Dictionary:
	var events: Dictionary = current_map_payload if _is_current_map_identifier(map_identifier) else build_map_payload(map_identifier)
	return _events_at_tile_from_collection(x, y, events)

func get_current_events_at_tile(x: int, y: int) -> Dictionary:
	return get_events_at_tile(x, y, current_map_key)

func get_objects_at_tile(x: int, y: int, map_identifier: Variant = "") -> Array:
	return _sanitize_array(get_events_at_tile(x, y, map_identifier).get("object_events", []), [])

func get_event_by_object(identifier: Variant, map_identifier: Variant = "") -> Dictionary:
	var matches: Array = get_events_for_object(identifier, map_identifier)
	if matches.is_empty():
		return {}
	return Dictionary(matches[0]).duplicate(true)

func get_events_for_object(identifier: Variant, map_identifier: Variant = "") -> Array:
	var events: Dictionary = current_map_payload if _is_current_map_identifier(map_identifier) else build_map_payload(map_identifier)
	var normalized: String = _normalize_identifier(identifier)
	var matches: Array = []
	if normalized.is_empty():
		return matches
	for event in _normalize_object_events(events.get("object_events", [])):
		var object_event: Dictionary = Dictionary(event)
		var candidates: Array = [
			object_event.get("object_identifier", null),
			object_event.get("label", null),
			object_event.get("script", ""),
			object_event.get("event_flag", ""),
			object_event.get("sprite", ""),
		]
		for candidate in candidates:
			if _normalize_identifier(candidate) == normalized:
				matches.append(object_event)
				break
	return matches

func get_object_event_records(map_identifier: Variant = "") -> Array:
	var events: Dictionary = current_map_payload if _is_current_map_identifier(map_identifier) else build_map_payload(map_identifier)
	var records: Array = []
	var index: int = 0
	for event in _normalize_object_events(events.get("object_events", [])):
		var record: Dictionary = _serialize_object_event_record(Dictionary(event), index)
		record["script_metadata"] = get_script_metadata(_coerce_string(record.get("script", ""), ""), map_identifier)
		records.append(record)
		index += 1
	return records

func get_current_object_event_records() -> Array:
	return get_object_event_records(current_map_key)

func get_npc_event_records(map_identifier: Variant = "") -> Array:
	return get_object_event_records(map_identifier)

func get_current_npc_event_records() -> Array:
	return get_object_event_records(current_map_key)

func get_object_event_record(identifier: Variant, map_identifier: Variant = "") -> Dictionary:
	var normalized: String = _normalize_identifier(identifier)
	if normalized.is_empty():
		return {}
	for record in get_object_event_records(map_identifier):
		var object_record: Dictionary = Dictionary(record)
		var candidates: Array = [
			object_record.get("object_identifier", null),
			object_record.get("label", null),
			object_record.get("script", ""),
			object_record.get("event_flag", ""),
			object_record.get("sprite", ""),
			object_record.get("index", -1),
			object_record.get("object_id", -1),
		]
		for candidate in candidates:
			if _normalize_identifier(candidate) == normalized:
				return object_record
	return {}

func get_npc_event_record(identifier: Variant, map_identifier: Variant = "") -> Dictionary:
	return get_object_event_record(identifier, map_identifier)

func get_object_event_records_at_tile(x: int, y: int, map_identifier: Variant = "") -> Array:
	var records: Array = []
	for record in get_object_event_records(map_identifier):
		var object_record: Dictionary = Dictionary(record)
		if _coerce_int(object_record.get("x", -1), -1) == x and _coerce_int(object_record.get("y", -1), -1) == y:
			records.append(object_record)
	return records

func get_warp_event_command_payload(warp_id: int, map_identifier: Variant = "") -> Dictionary:
	var warp: Dictionary = get_warp_by_id(warp_id, map_identifier)
	if warp.is_empty():
		return {}
	return _serialize_command_payload(warp, "warp_event")

func get_warp_event_command_payloads(map_identifier: Variant = "") -> Array:
	var events: Dictionary = current_map_payload if _is_current_map_identifier(map_identifier) else build_map_payload(map_identifier)
	var payloads: Array = []
	for warp in _normalize_warps(events.get("warps", [])):
		payloads.append(_serialize_command_payload(Dictionary(warp), "warp_event"))
	return payloads

func get_coord_event_command_payloads_at_tile(x: int, y: int, map_identifier: Variant = "") -> Array:
	var events: Dictionary = current_map_payload if _is_current_map_identifier(map_identifier) else build_map_payload(map_identifier)
	var payloads: Array = []
	for coord in _normalize_coord_events(events.get("coord_events", [])):
		var coord_event: Dictionary = Dictionary(coord)
		if _coerce_int(coord_event.get("x", -1), -1) == x and _coerce_int(coord_event.get("y", -1), -1) == y:
			payloads.append(_serialize_command_payload(coord_event, "coord_event"))
	return payloads

func get_bg_event_command_payloads_at_tile(x: int, y: int, map_identifier: Variant = "") -> Array:
	var events: Dictionary = current_map_payload if _is_current_map_identifier(map_identifier) else build_map_payload(map_identifier)
	var payloads: Array = []
	for bg in _normalize_bg_events(events.get("bg_events", [])):
		var bg_event: Dictionary = Dictionary(bg)
		if _coerce_int(bg_event.get("x", -1), -1) == x and _coerce_int(bg_event.get("y", -1), -1) == y:
			payloads.append(_serialize_command_payload(bg_event, "bg_event"))
	return payloads

func get_event_command_payloads_at_tile(x: int, y: int, map_identifier: Variant = "") -> Dictionary:
	var object_payloads: Array = []
	for object_record in get_object_event_records_at_tile(x, y, map_identifier):
		object_payloads.append(_serialize_command_payload(Dictionary(object_record), "object_event"))
	return {
		"x": x,
		"y": y,
		"warps": _command_payloads_from_events(_sanitize_array(get_events_at_tile(x, y, map_identifier).get("warps", []), []), "warp_event"),
		"coord_events": get_coord_event_command_payloads_at_tile(x, y, map_identifier),
		"bg_events": get_bg_event_command_payloads_at_tile(x, y, map_identifier),
		"object_events": object_payloads,
	}

func get_map_manifest_entry(map_identifier: Variant = "") -> Dictionary:
	_ensure_assets_loaded()
	if _is_current_map_identifier(map_identifier) and not current_map_payload.is_empty():
		return _manifest_entry_from_sources(current_map_payload, current_map_payload, current_map_key)
	var attribute_entry: Dictionary = _find_attribute_entry(map_identifier)
	var runtime_entry: Dictionary = _find_runtime_entry(map_identifier)
	var fallback_key: String = _coerce_string(_resolve_map_key(map_identifier), _coerce_string(map_identifier, ""))
	return _manifest_entry_from_sources(attribute_entry, runtime_entry, fallback_key)

func get_current_manifest_entry() -> Dictionary:
	return get_map_manifest_entry(current_map_key)

func _serialize_warp_record(warp: Dictionary) -> Dictionary:
	var target_constant: String = _coerce_string(warp.get("target_map_constant", warp.get("target_map", "")), "")
	return {
		"index": _coerce_int(warp.get("index", 0), 0),
		"warp_id": _coerce_int(warp.get("warp_id", warp.get("warpId", 0)), 0),
		"warpId": _coerce_int(warp.get("warp_id", warp.get("warpId", 0)), 0),
		"x": _coerce_int(warp.get("x", 0), 0),
		"y": _coerce_int(warp.get("y", 0), 0),
		"target_map_constant": _resolve_map_constant(target_constant),
		"target_map": _coerce_string(warp.get("target_map", target_constant), target_constant),
		"target_warp_id": _coerce_int(warp.get("target_warp_id", 0), 0),
		"target_lookup": get_map_lookup(target_constant),
		"command": "warp_event",
		"args": _event_args(warp, "warp_event"),
		"command_payload": _event_command_payload(warp, "warp_event"),
	}

func _manifest_entry_from_sources(attribute_entry: Dictionary, runtime_entry: Dictionary, fallback_key: String) -> Dictionary:
	var map_name: String = _coerce_string(_first_present(attribute_entry, ["name", "map_name", "mapName"]), "")
	if map_name.is_empty():
		map_name = _coerce_string(_first_present(runtime_entry, ["name", "map_name", "mapName"]), fallback_key)
	var map_constant: String = _coerce_string(_first_present(attribute_entry, ["map_constant", "constant", "mapConstant"]), "")
	if map_constant.is_empty():
		map_constant = _coerce_string(_first_present(runtime_entry, ["constant", "map_constant", "mapConstant"]), fallback_key)
	var group_name: String = _coerce_string(_first_present(attribute_entry, ["map_group_constant", "group_name", "groupName"]), "")
	if group_name.is_empty():
		group_name = _coerce_string(_first_present(runtime_entry, ["group_name", "groupName"]), "")
	return {
		"map_key": _coerce_string(map_constant, fallback_key),
		"map_name": map_name,
		"map_constant": map_constant,
		"group_name": group_name,
		"group_id": _coerce_int(_first_present(attribute_entry, ["group_id", "groupId"]), _coerce_int(_first_present(runtime_entry, ["group_id", "groupId"]), -1)),
		"map_id": _coerce_int(_first_present(attribute_entry, ["map_id", "mapId"]), _coerce_int(_first_present(runtime_entry, ["map_id", "mapId"]), -1)),
		"tileset_name": _coerce_string(_first_present(attribute_entry, ["tileset_name", "tilesetName"]), ""),
		"blocks_label": _coerce_string(_first_present(attribute_entry, ["blocks_label", "blocksLabel"]), ""),
		"map_events_label": _coerce_string(_first_present(attribute_entry, ["map_events_label", "mapEventsLabel"]), ""),
		"map_scripts_label": _coerce_string(_first_present(attribute_entry, ["map_scripts_label", "mapScriptsLabel"]), ""),
		"width": _coerce_int(_first_present(attribute_entry, ["width"]), _coerce_int(_first_present(runtime_entry, ["width"]), 0)),
		"height": _coerce_int(_first_present(attribute_entry, ["height"]), _coerce_int(_first_present(runtime_entry, ["height"]), 0)),
		"environment": _coerce_string(_first_present(attribute_entry, ["environment"]), _coerce_string(_first_present(runtime_entry, ["environment"]), "")),
		"location": _coerce_string(_first_present(attribute_entry, ["location"]), ""),
		"music": _coerce_string(_first_present(attribute_entry, ["music"]), ""),
	}

func _map_file_for_identifier(map_identifier: Variant = "") -> Dictionary:
	var payload: Dictionary = current_map_payload if _is_current_map_identifier(map_identifier) else build_map_payload(map_identifier)
	var map_file: Dictionary = _sanitize_dictionary(payload.get("map_file", {}), {})
	if not map_file.is_empty():
		return map_file
	var map_name: String = _coerce_string(payload.get("map_name", _resolve_map_name(map_identifier)), "")
	if map_name.is_empty() and _is_current_map_identifier(map_identifier):
		map_name = current_map_name
	return _load_map_file_payload(map_name)

func _normalize_command_array(commands: Array) -> Array:
	var result: Array = []
	var index: int = 0
	for command in commands:
		var entry: Dictionary = _sanitize_dictionary(command, {})
		if entry.is_empty():
			index += 1
			continue
		var command_name: String = _coerce_string(entry.get("command", ""), "")
		var args: Array = _sanitize_array(entry.get("args", []), [])
		result.append({
			"index": index,
			"command": command_name,
			"args": args.duplicate(true),
			"command_payload": _command_payload_from_args(command_name, args),
		})
		index += 1
	return result

func _parse_scene_script_metadata(commands: Array) -> Array:
	var result: Array = []
	var index: int = 0
	for command in commands:
		var entry: Dictionary = _sanitize_dictionary(command, {})
		if _coerce_string(entry.get("command", ""), "") != "scene_script":
			continue
		var args: Array = _sanitize_array(entry.get("args", []), [])
		if args.size() < 2:
			continue
		result.append({
			"index": index,
			"script": _coerce_string(args[0], ""),
			"scene_id": _coerce_string(args[1], ""),
			"command": "scene_script",
			"args": args.duplicate(true),
		})
		index += 1
	return result

func _parse_callback_metadata(commands: Array) -> Array:
	var result: Array = []
	var index: int = 0
	for command in commands:
		var entry: Dictionary = _sanitize_dictionary(command, {})
		if _coerce_string(entry.get("command", ""), "") != "callback":
			continue
		var args: Array = _sanitize_array(entry.get("args", []), [])
		if args.size() < 2:
			continue
		result.append({
			"index": index,
			"callback_type": _coerce_string(args[0], ""),
			"script": _coerce_string(args[1], ""),
			"command": "callback",
			"args": args.duplicate(true),
		})
		index += 1
	return result

func _script_references(label: String, map_identifier: Variant = "") -> Dictionary:
	var normalized: String = _normalize_identifier(label)
	var events: Dictionary = current_map_payload if _is_current_map_identifier(map_identifier) else build_map_payload(map_identifier)
	var object_refs: Array = []
	var bg_refs: Array = []
	var coord_refs: Array = []
	var script_refs: Array = []
	var index: int = 0
	for object_event in _normalize_object_events(events.get("object_events", [])):
		var object_record: Dictionary = Dictionary(object_event)
		if _normalize_identifier(object_record.get("script", "")) == normalized:
			object_refs.append(_serialize_object_event_record(object_record, index))
		index += 1
	index = 0
	for bg_event in _normalize_bg_events(events.get("bg_events", [])):
		var bg_record: Dictionary = Dictionary(bg_event)
		if _normalize_identifier(bg_record.get("script", "")) == normalized:
			bg_record["index"] = index
			bg_refs.append(bg_record)
		index += 1
	index = 0
	for coord_event in _normalize_coord_events(events.get("coord_events", [])):
		var coord_record: Dictionary = Dictionary(coord_event)
		if _normalize_identifier(coord_record.get("script_name", "")) == normalized:
			coord_record["index"] = index
			coord_refs.append(coord_record)
		index += 1
	var script_meta: Dictionary = _map_script_metadata_without_refs(map_identifier)
	for scene in _sanitize_array(script_meta.get("scene_scripts", []), []):
		var scene_record: Dictionary = Dictionary(scene)
		if _normalize_identifier(scene_record.get("script", "")) == normalized:
			script_refs.append(scene_record)
	for callback in _sanitize_array(script_meta.get("callbacks", []), []):
		var callback_record: Dictionary = Dictionary(callback)
		if _normalize_identifier(callback_record.get("script", "")) == normalized:
			script_refs.append(callback_record)
	return {
		"object_events": object_refs,
		"bg_events": bg_refs,
		"coord_events": coord_refs,
		"map_scripts": script_refs,
	}

func _event_script_labels(events: Array, script_keys: Array) -> Array:
	var labels: Array = []
	for event in events:
		var record: Dictionary = _sanitize_dictionary(event, {})
		for key in script_keys:
			var label: String = _coerce_string(record.get(key, ""), "")
			if not label.is_empty() and not labels.has(label):
				labels.append(label)
	labels.sort()
	return labels

func get_map_manifest_entries() -> Array:
	_ensure_assets_loaded()
	var entries: Array = []
	for key in map_attributes.keys():
		var map_name: String = _coerce_string(key, "")
		var attribute_entry: Dictionary = _normalize_map_attribute_entry(map_attributes.get(key, {}), map_name)
		var runtime_entry: Dictionary = _find_runtime_entry(map_name)
		entries.append(_manifest_entry_from_sources(attribute_entry, runtime_entry, map_name))
	entries.sort_custom(func(left, right): return _coerce_string(Dictionary(left).get("map_name", ""), "") < _coerce_string(Dictionary(right).get("map_name", ""), ""))
	return entries

func get_maps_by_tileset(tileset_name: String) -> Array:
	var normalized: String = _normalize_identifier(tileset_name)
	var entries: Array = []
	for entry in get_map_manifest_entries():
		var manifest: Dictionary = Dictionary(entry)
		if _normalize_identifier(manifest.get("tileset_name", "")) == normalized:
			entries.append(manifest)
	return entries

func get_maps_by_environment(environment: String) -> Array:
	var normalized: String = _normalize_identifier(environment)
	var entries: Array = []
	for entry in get_map_manifest_entries():
		var manifest: Dictionary = Dictionary(entry)
		if _normalize_identifier(manifest.get("environment", "")) == normalized:
			entries.append(manifest)
	return entries

func get_event_metadata(map_identifier: Variant = "") -> Dictionary:
	var events: Dictionary = current_map_payload if _is_current_map_identifier(map_identifier) else build_map_payload(map_identifier)
	var manifest: Dictionary = get_map_manifest_entry(map_identifier)
	return {
		"manifest": manifest,
		"map_events_label": _coerce_string(events.get("map_events_label", manifest.get("map_events_label", "")), ""),
		"map_scripts_label": _coerce_string(events.get("map_scripts_label", manifest.get("map_scripts_label", "")), ""),
		"warp_count": _normalize_warps(events.get("warps", [])).size(),
		"coord_event_count": _normalize_coord_events(events.get("coord_events", [])).size(),
		"bg_event_count": _normalize_bg_events(events.get("bg_events", [])).size(),
		"object_event_count": _normalize_object_events(events.get("object_events", [])).size(),
		"script_labels": get_script_labels(map_identifier),
		"object_scripts": _event_script_labels(_normalize_object_events(events.get("object_events", [])), ["script"]),
		"bg_event_scripts": _event_script_labels(_normalize_bg_events(events.get("bg_events", [])), ["script"]),
		"coord_event_scripts": _event_script_labels(_normalize_coord_events(events.get("coord_events", [])), ["script_name"]),
	}

func get_current_event_metadata() -> Dictionary:
	return get_event_metadata(current_map_key)

func get_script_labels(map_identifier: Variant = "") -> Array:
	var map_file: Dictionary = _map_file_for_identifier(map_identifier)
	var labels: Array = []
	for key in map_file.keys():
		if typeof(map_file.get(key)) == TYPE_ARRAY:
			labels.append(_coerce_string(key, ""))
	labels.sort()
	return labels

func get_script_metadata(label: String, map_identifier: Variant = "") -> Dictionary:
	var map_file: Dictionary = _map_file_for_identifier(map_identifier)
	var commands: Array = _sanitize_array(map_file.get(label, []), [])
	if commands.is_empty() and not map_file.has(label):
		return {}
	return {
		"label": label,
		"command_count": commands.size(),
		"commands": _normalize_command_array(commands),
		"referenced_by": _script_references(label, map_identifier),
	}

func get_current_script_metadata(label: String) -> Dictionary:
	return get_script_metadata(label, current_map_key)

func get_map_script_metadata(map_identifier: Variant = "") -> Dictionary:
	var payload: Dictionary = current_map_payload if _is_current_map_identifier(map_identifier) else build_map_payload(map_identifier)
	var map_file: Dictionary = _map_file_for_identifier(map_identifier)
	var scripts_label: String = _coerce_string(payload.get("map_scripts_label", ""), "")
	var script_commands: Array = _sanitize_array(map_file.get(scripts_label, []), [])
	return {
		"map_scripts_label": scripts_label,
		"scene_scripts": _parse_scene_script_metadata(script_commands),
		"callbacks": _parse_callback_metadata(script_commands),
		"script_labels": get_script_labels(map_identifier),
	}

func get_current_map_script_metadata() -> Dictionary:
	return get_map_script_metadata(current_map_key)

func get_bg_event_records(map_identifier: Variant = "") -> Array:
	var events: Dictionary = current_map_payload if _is_current_map_identifier(map_identifier) else build_map_payload(map_identifier)
	var records: Array = []
	var index: int = 0
	for bg in _normalize_bg_events(events.get("bg_events", [])):
		var record: Dictionary = Dictionary(bg)
		record["index"] = index
		record["script_metadata"] = get_script_metadata(_coerce_string(record.get("script", ""), ""), map_identifier)
		records.append(record)
		index += 1
	return records

func get_coord_event_records(map_identifier: Variant = "") -> Array:
	var events: Dictionary = current_map_payload if _is_current_map_identifier(map_identifier) else build_map_payload(map_identifier)
	var records: Array = []
	var index: int = 0
	for coord in _normalize_coord_events(events.get("coord_events", [])):
		var record: Dictionary = Dictionary(coord)
		record["index"] = index
		record["script_metadata"] = get_script_metadata(_coerce_string(record.get("script_name", ""), ""), map_identifier)
		records.append(record)
		index += 1
	return records

func get_coord_event_record(identifier: Variant, map_identifier: Variant = "") -> Dictionary:
	var normalized: String = _normalize_identifier(identifier)
	if normalized.is_empty():
		return {}
	for record in get_coord_event_records(map_identifier):
		var coord_record: Dictionary = Dictionary(record)
		var candidates: Array = [
			coord_record.get("script_name", ""),
			coord_record.get("scene_id", ""),
			coord_record.get("index", -1),
		]
		for candidate in candidates:
			if _normalize_identifier(candidate) == normalized:
				return coord_record
	return {}

func get_bg_event_record(identifier: Variant, map_identifier: Variant = "") -> Dictionary:
	var normalized: String = _normalize_identifier(identifier)
	if normalized.is_empty():
		return {}
	for record in get_bg_event_records(map_identifier):
		var bg_record: Dictionary = Dictionary(record)
		var candidates: Array = [
			bg_record.get("script", ""),
			bg_record.get("event_type", ""),
			bg_record.get("index", -1),
		]
		for candidate in candidates:
			if _normalize_identifier(candidate) == normalized:
				return bg_record
	return {}

func get_warp_record(warp_id: int, map_identifier: Variant = "") -> Dictionary:
	var warp: Dictionary = get_warp_by_id(warp_id, map_identifier)
	if warp.is_empty():
		return {}
	return _serialize_warp_record(warp)

func get_warp_records(map_identifier: Variant = "") -> Array:
	var events: Dictionary = current_map_payload if _is_current_map_identifier(map_identifier) else build_map_payload(map_identifier)
	var records: Array = []
	for warp in _normalize_warps(events.get("warps", [])):
		records.append(_serialize_warp_record(Dictionary(warp)))
	return records

func apply_spawn_point(spawn_point: Variant) -> bool:
	var normalized_spawn := _normalize_spawn_point(spawn_point)
	if normalized_spawn.is_empty():
		return false
	current_spawn_point = normalized_spawn
	var resolved := _resolve_map_key(normalized_spawn)
	if resolved.is_empty():
		resolved = _resolve_map_key(_coerce_string(_first_present(normalized_spawn, ["mapName", "map_name"]), ""))
	if resolved.is_empty():
		resolved = _resolve_map_key(_coerce_string(_first_present(normalized_spawn, ["mapConstant", "map_constant"]), ""))
	if resolved.is_empty():
		return false
	if not _apply_selected_map(resolved):
		return false
	current_spawn_point = normalized_spawn
	_update_summary()
	emit_signal("changed")
	return true

func get_state() -> Dictionary:
	return {
		"asset_summary": asset_summary.duplicate(true),
		"runtime_map_metadata": runtime_map_metadata.duplicate(true),
		"runtime_spawn_points": runtime_spawn_points.duplicate(true),
		"map_attributes": map_attributes.duplicate(true),
		"map_blocks": map_blocks.duplicate(true),
		"collision_permissions": collision_permissions.duplicate(true),
		"collision_stdscripts": collision_stdscripts.duplicate(true),
		"current_map_key": current_map_key,
		"current_map_name": current_map_name,
		"current_map_constant": current_map_constant,
		"current_map_group_name": current_map_group_name,
		"current_group_id": current_group_id,
		"current_map_group_id": current_map_group_id,
		"current_map_id": current_map_id,
		"current_environment": current_environment,
		"current_music": current_music,
		"current_tileset_name": current_tileset_name,
		"current_location": current_location,
		"current_phone_service": current_phone_service,
		"current_width": current_width,
		"current_height": current_height,
		"current_border_block": current_border_block,
		"current_connections": current_connections.duplicate(true),
		"current_coord_events": current_coord_events.duplicate(true),
		"current_warps": current_warps.duplicate(true),
		"current_bg_events": current_bg_events.duplicate(true),
		"current_object_events": current_object_events.duplicate(true),
		"current_spawn_points": current_spawn_points.duplicate(true),
		"current_spawn_point": current_spawn_point.duplicate(true),
		"current_blocks_label": current_blocks_label,
		"current_map_events_label": current_map_events_label,
		"current_map_scripts_label": current_map_scripts_label,
		"current_map_payload": current_map_payload.duplicate(true),
		"current_map_summary": current_map_summary.duplicate(true),
	}

func from_state(data: Dictionary) -> void:
	if data.is_empty():
		return
	asset_summary = Dictionary(data.get("asset_summary", asset_summary))
	runtime_map_metadata = Dictionary(data.get("runtime_map_metadata", runtime_map_metadata))
	runtime_spawn_points = Dictionary(data.get("runtime_spawn_points", runtime_spawn_points))
	map_attributes = Dictionary(data.get("map_attributes", map_attributes))
	map_blocks = Dictionary(data.get("map_blocks", map_blocks))
	collision_permissions = Array(data.get("collision_permissions", collision_permissions)).duplicate(true)
	collision_stdscripts = Dictionary(data.get("collision_stdscripts", collision_stdscripts)).duplicate(true)
	current_map_key = _coerce_string(data.get("current_map_key", current_map_key), current_map_key)
	current_map_name = _coerce_string(data.get("current_map_name", current_map_name), current_map_name)
	current_map_constant = _coerce_string(data.get("current_map_constant", current_map_constant), current_map_constant)
	current_map_group_name = _coerce_string(data.get("current_map_group_name", current_map_group_name), current_map_group_name)
	current_group_id = _coerce_int(data.get("current_group_id", data.get("current_map_group_id", current_group_id)), current_group_id)
	current_map_group_id = _coerce_int(data.get("current_map_group_id", current_map_group_id), current_map_group_id)
	if current_map_group_id < 0:
		current_map_group_id = current_group_id
	if current_group_id < 0:
		current_group_id = current_map_group_id
	current_map_id = _coerce_int(data.get("current_map_id", current_map_id), current_map_id)
	current_environment = _coerce_string(data.get("current_environment", current_environment), current_environment)
	current_music = _coerce_string(data.get("current_music", current_music), current_music)
	current_tileset_name = _coerce_string(data.get("current_tileset_name", current_tileset_name), current_tileset_name)
	current_location = _coerce_string(data.get("current_location", current_location), current_location)
	current_phone_service = _coerce_int(data.get("current_phone_service", current_phone_service), current_phone_service)
	current_width = max(0, _coerce_int(data.get("current_width", current_width), current_width))
	current_height = max(0, _coerce_int(data.get("current_height", current_height), current_height))
	current_border_block = _coerce_int(data.get("current_border_block", current_border_block), current_border_block)
	current_connections = Array(data.get("current_connections", current_connections)).duplicate(true)
	current_coord_events = _normalize_coord_events(data.get("current_coord_events", current_coord_events))
	current_warps = _normalize_warps(data.get("current_warps", current_warps))
	current_bg_events = _normalize_bg_events(data.get("current_bg_events", current_bg_events))
	current_object_events = _normalize_object_events(data.get("current_object_events", current_object_events))
	current_spawn_points = Array(data.get("current_spawn_points", current_spawn_points)).duplicate(true)
	current_spawn_point = Dictionary(data.get("current_spawn_point", current_spawn_point)).duplicate(true)
	current_blocks_label = _coerce_string(data.get("current_blocks_label", current_blocks_label), current_blocks_label)
	current_map_events_label = _coerce_string(data.get("current_map_events_label", current_map_events_label), current_map_events_label)
	current_map_scripts_label = _coerce_string(data.get("current_map_scripts_label", current_map_scripts_label), current_map_scripts_label)
	current_map_payload = Dictionary(data.get("current_map_payload", current_map_payload)).duplicate(true)
	current_map_summary = Dictionary(data.get("current_map_summary", current_map_summary)).duplicate(true)
	_rebuild_lookup_tables()
	_update_summary()

func to_dictionary() -> Dictionary:
	return get_state()

func from_dictionary(data: Variant) -> bool:
	if typeof(data) != TYPE_DICTIONARY:
		return false
	from_state(Dictionary(data))
	return true

func get_spawn_points_for_map(map_identifier: Variant) -> Array:
	_ensure_assets_loaded()
	var resolved := _resolve_map_key(map_identifier)
	var matches: Array = []
	for key in runtime_spawn_points.keys():
		var spawn := _normalize_spawn_point(runtime_spawn_points.get(key, {}))
		if spawn.is_empty():
			continue
		if _spawn_matches_identifier(spawn, resolved, map_identifier):
			matches.append(spawn)
	if matches.is_empty() and not resolved.is_empty():
		var alt_name := _map_name_for_key(resolved)
		for key in runtime_spawn_points.keys():
			var fallback_spawn := _normalize_spawn_point(runtime_spawn_points.get(key, {}))
			if fallback_spawn.is_empty():
				continue
			if _spawn_matches_identifier(fallback_spawn, alt_name, alt_name):
				matches.append(fallback_spawn)
	return matches

func get_spawn_point_for_map(map_identifier: Variant) -> Dictionary:
	var matches := get_spawn_points_for_map(map_identifier)
	if matches.is_empty():
		return {}
	return Dictionary(matches[0])

func get_current_selection() -> Dictionary:
	return {
		"map_key": current_map_key,
		"map_name": current_map_name,
		"map_constant": current_map_constant,
		"group_name": current_map_group_name,
		"current_group_id": current_group_id,
		"group_id": current_group_id,
		"map_id": current_map_id,
		"environment": current_environment,
		"music": current_music,
		"tileset_name": current_tileset_name,
		"location": current_location,
		"phone_service": current_phone_service,
		"width": current_width,
		"height": current_height,
		"border_block": current_border_block,
	}

func get_current_summary() -> Dictionary:
	_update_summary()
	return current_map_summary.duplicate(true)

func get_summary() -> Dictionary:
	return get_current_summary()

func get_available_map_keys() -> Array[String]:
	_ensure_assets_loaded()
	var keys: Array[String] = []
	for key in runtime_map_metadata.keys():
		var normalized_key := _coerce_string(key, "")
		if not normalized_key.is_empty():
			keys.append(normalized_key)
	for key in map_attributes.keys():
		var normalized_attribute_key := _coerce_string(key, "")
		if not normalized_attribute_key.is_empty():
			keys.append(normalized_attribute_key)
	var unique_keys: Array[String] = []
	for key in keys:
		if not unique_keys.has(key):
			unique_keys.append(key)
	unique_keys.sort()
	return unique_keys

func get_selected_map_key() -> String:
	var resolved := _resolve_map_key(current_map_key)
	if not resolved.is_empty():
		return resolved
	resolved = _resolve_map_key(current_map_constant)
	if not resolved.is_empty():
		return resolved
	resolved = _resolve_map_key(current_map_name)
	if not resolved.is_empty():
		return resolved
	resolved = _resolve_map_key(current_map_key)
	if not resolved.is_empty():
		return resolved
	resolved = _resolve_map_key(current_spawn_point)
	if not resolved.is_empty():
		return resolved
	if not current_map_key.is_empty():
		return current_map_key
	var keys := get_available_map_keys()
	if keys.is_empty():
		return ""
	return keys[0]

func get_selected_map_index() -> int:
	var keys := get_available_map_keys()
	if keys.is_empty():
		return -1
	var selected_key := get_selected_map_key()
	if selected_key.is_empty():
		return -1
	return keys.find(selected_key)

func get_map_manifest() -> Dictionary:
	_ensure_assets_loaded()
	var manifest: Dictionary = {}
	for entry in get_map_manifest_entries():
		var manifest_entry: Dictionary = Dictionary(entry).duplicate(true)
		var map_key := _coerce_string(manifest_entry.get("map_key", ""), "")
		var map_name := _coerce_string(manifest_entry.get("map_name", ""), "")
		var map_constant := _coerce_string(manifest_entry.get("map_constant", ""), "")
		if not map_key.is_empty():
			manifest[map_key] = manifest_entry
		if not map_name.is_empty() and not manifest.has(map_name):
			manifest[map_name] = manifest_entry.duplicate(true)
		if not map_constant.is_empty() and not manifest.has(map_constant):
			manifest[map_constant] = manifest_entry.duplicate(true)
	return manifest

func get_map_summary() -> Dictionary:
	return get_current_summary()

func get_spawn_summary() -> Dictionary:
	return _sanitize_dictionary(current_spawn_point, {})

func get_current_map_block_key() -> String:
	return current_blocks_label

func build_map_payload(map_identifier: Variant) -> Dictionary:
	_ensure_assets_loaded()
	var resolved_key := _resolve_map_key(map_identifier)
	var map_name := _map_name_for_key(resolved_key)
	if map_name.is_empty():
		map_name = _resolve_map_name(map_identifier)
	var attribute_entry := _find_attribute_entry(map_identifier)
	var runtime_entry := _find_runtime_entry(map_identifier)
	if map_name.is_empty():
		map_name = _resolve_map_name(attribute_entry)
	if map_name.is_empty():
		map_name = _resolve_map_name(runtime_entry)
	if map_name.is_empty() and not resolved_key.is_empty():
		map_name = _map_name_for_key(resolved_key)

	var payload: Dictionary = {}
	if not attribute_entry.is_empty():
		payload.merge(_normalize_map_attribute_entry(attribute_entry), true)
	if not runtime_entry.is_empty():
		payload.merge(_normalize_runtime_entry(runtime_entry), true)

	if map_name.is_empty():
		map_name = _coerce_string(payload.get("map_name", ""), "")
	if map_name.is_empty():
		map_name = DEFAULT_MAP_NAME

	var map_file := _load_map_file_payload(map_name)
	var parsed_events := _parse_map_events(map_file)
	if not map_file.is_empty():
		payload["source"] = "assets"
		payload["map_file"] = map_file
	else:
		payload["source"] = "metadata"
		payload["map_file"] = {}

	payload["map_key"] = resolved_key
	payload["map_name"] = map_name
	payload["map_constant"] = _coerce_string(payload.get("map_constant", resolved_key), resolved_key)
	payload["border_block"] = _coerce_int(payload.get("border_block", current_border_block), current_border_block)
	payload["runtime_metadata"] = _sanitize_dictionary(runtime_entry, {})
	payload["connections"] = _sanitize_array(payload.get("connections", []), [])
	payload.merge(_normalize_event_collection(parsed_events), true)
	payload["map_events_label"] = _coerce_string(parsed_events.get("map_events_label", payload.get("map_events_label", "")), "")
	payload["map_scripts_label"] = _coerce_string(parsed_events.get("map_scripts_label", payload.get("map_scripts_label", "")), "")
	payload["blocks_data"] = map_blocks.get(_coerce_string(payload.get("blocks_label", ""), ""), "")
	payload["spawn_points"] = get_spawn_points_for_map(map_name)
	return _sanitize_dictionary(payload, {})

func _apply_selected_map(map_key: String) -> bool:
	var normalized_key := _resolve_map_key(map_key)
	if normalized_key.is_empty():
		return false
	var runtime_entry := _find_runtime_entry(normalized_key)
	var attribute_entry := _find_attribute_entry(normalized_key)
	var map_name := _resolve_map_name(runtime_entry)
	if map_name.is_empty():
		map_name = _resolve_map_name(attribute_entry)
	if map_name.is_empty():
		map_name = _map_name_for_key(normalized_key)
	if map_name.is_empty():
		map_name = DEFAULT_MAP_NAME

	current_map_key = normalized_key
	current_map_name = map_name
	current_map_constant = _coerce_string(_first_present(attribute_entry, ["map_constant", "constant"]), normalized_key)
	if current_map_constant.is_empty():
		current_map_constant = _coerce_string(_first_present(runtime_entry, ["constant", "mapConstant", "map_constant"]), normalized_key)
	current_map_group_name = _coerce_string(_first_present(attribute_entry, ["map_group_constant", "groupName", "group_name"]), "")
	if current_map_group_name.is_empty():
		current_map_group_name = _coerce_string(_first_present(runtime_entry, ["groupName", "group_name"]), "")
	current_group_id = _coerce_int(_first_present(attribute_entry, ["group_id", "groupId"]), -1)
	if current_group_id < 0:
		current_group_id = _coerce_int(_first_present(runtime_entry, ["groupId", "group_id"]), -1)
	current_map_group_id = current_group_id
	current_map_id = _coerce_int(_first_present(attribute_entry, ["map_id", "mapId"]), -1)
	if current_map_id < 0:
		current_map_id = _coerce_int(_first_present(runtime_entry, ["mapId", "map_id"]), -1)
	current_border_block = _coerce_int(_first_present(attribute_entry, ["border_block", "borderBlock"]), current_border_block)
	current_spawn_points = get_spawn_points_for_map(current_map_name)
	if current_spawn_point.is_empty():
		current_spawn_point = get_spawn_point_for_map(current_map_name)
	current_map_payload = build_map_payload(current_map_name)
	_apply_payload_fields(current_map_payload)
	_update_summary()
	emit_signal("changed")
	return true

func _apply_payload_fields(payload: Dictionary) -> void:
	current_map_name = _coerce_string(payload.get("map_name", current_map_name), current_map_name)
	current_map_constant = _coerce_string(payload.get("map_constant", current_map_constant), current_map_constant)
	current_map_group_name = _coerce_string(payload.get("map_group_constant", payload.get("group_name", current_map_group_name)), current_map_group_name)
	current_group_id = _coerce_int(payload.get("current_group_id", payload.get("group_id", payload.get("groupId", current_group_id))), current_group_id)
	current_map_group_id = current_group_id
	current_map_id = _coerce_int(payload.get("map_id", payload.get("mapId", current_map_id)), current_map_id)
	current_environment = _coerce_string(payload.get("environment", current_environment), current_environment)
	current_music = _coerce_string(payload.get("music", current_music), current_music)
	current_tileset_name = _coerce_string(payload.get("tileset_name", current_tileset_name), current_tileset_name)
	current_location = _coerce_string(payload.get("location", current_location), current_location)
	current_phone_service = _coerce_int(payload.get("phone_service", current_phone_service), current_phone_service)
	current_width = _coerce_int(payload.get("width", current_width), current_width)
	current_height = _coerce_int(payload.get("height", current_height), current_height)
	current_border_block = _coerce_int(payload.get("border_block", current_border_block), current_border_block)
	current_connections = _sanitize_array(payload.get("connections", current_connections), [])
	var events := _normalize_event_collection(payload)
	current_coord_events = _sanitize_array(events.get("coord_events", current_coord_events), [])
	current_warps = _sanitize_array(events.get("warps", current_warps), [])
	current_bg_events = _sanitize_array(events.get("bg_events", current_bg_events), [])
	current_object_events = _sanitize_array(events.get("object_events", current_object_events), [])
	current_blocks_label = _coerce_string(payload.get("blocks_label", current_blocks_label), current_blocks_label)
	current_map_events_label = _coerce_string(payload.get("map_events_label", current_map_events_label), current_map_events_label)
	current_map_scripts_label = _coerce_string(payload.get("map_scripts_label", current_map_scripts_label), current_map_scripts_label)
	current_map_payload = _sanitize_dictionary(payload, {})

func _update_summary() -> void:
	current_map_summary = _build_summary()

func _build_summary() -> Dictionary:
	var summary: Dictionary = {}
	summary["asset_summary"] = _sanitize_dictionary(asset_summary, {})
	summary["selection"] = get_current_selection()
	summary["map_key"] = current_map_key
	summary["map_name"] = current_map_name
	summary["map_constant"] = current_map_constant
	summary["group_name"] = current_map_group_name
	summary["group_id"] = current_group_id
	summary["map_id"] = current_map_id
	summary["environment"] = current_environment
	summary["music"] = current_music
	summary["tileset_name"] = current_tileset_name
	summary["location"] = current_location
	summary["phone_service"] = current_phone_service
	summary["width"] = current_width
	summary["height"] = current_height
	summary["border_block"] = current_border_block
	summary["connections"] = current_connections.size()
	summary["coord_events"] = current_coord_events.size()
	summary["warps"] = current_warps.size()
	summary["bg_events"] = current_bg_events.size()
	summary["object_events"] = current_object_events.size()
	summary["spawn_points"] = current_spawn_points.duplicate(true)
	summary["spawn_point"] = _sanitize_dictionary(current_spawn_point, {})
	summary["blocks_label"] = current_blocks_label
	summary["map_events_label"] = current_map_events_label
	summary["map_scripts_label"] = current_map_scripts_label
	summary["runtime_metadata"] = _sanitize_dictionary(_find_runtime_entry(current_map_key), {})
	return summary

func _rebuild_lookup_tables() -> void:
	_map_key_by_identifier = {}
	_map_key_by_name = {}
	_map_key_by_constant = {}
	_map_key_by_group = {}
	for key in runtime_map_metadata.keys():
		var entry := _normalize_runtime_entry(runtime_map_metadata.get(key, {}), _coerce_string(key, ""))
		var map_key := _coerce_string(entry.get("constant", key), key)
		var map_name := _coerce_string(entry.get("name", ""), "")
		if map_key.is_empty() and not map_name.is_empty():
			map_key = map_name
		if map_key.is_empty():
			map_key = _coerce_string(key, "")
		_register_lookup(map_key, map_key)
		_register_lookup(map_name, map_key)
		_register_lookup(_coerce_string(entry.get("constant", map_key), map_key), map_key)
		_register_lookup(_group_lookup_key(entry), map_key)
	for key in map_attributes.keys():
		var entry := _normalize_map_attribute_entry(map_attributes.get(key, {}), _coerce_string(key, ""))
		var map_name := _coerce_string(entry.get("name", key), key)
		var map_key := _coerce_string(entry.get("map_constant", ""), "")
		if not map_name.is_empty():
			_register_lookup(map_name, map_name)
		if not map_key.is_empty():
			_register_lookup(map_key, map_name)
		_register_lookup(_group_lookup_key(entry), map_name)

func _register_lookup(identifier: Variant, map_key: String) -> void:
	var key := _normalize_identifier(identifier)
	if key.is_empty() or map_key.is_empty():
		return
	_map_key_by_identifier[key] = map_key
	if _map_key_by_name.has(key) == false:
		_map_key_by_name[key] = map_key
	if _map_key_by_constant.has(key) == false:
		_map_key_by_constant[key] = map_key
	if _map_key_by_group.has(key) == false:
		_map_key_by_group[key] = map_key

func _resolve_map_key(identifier: Variant) -> String:
	if typeof(identifier) == TYPE_DICTIONARY:
		var payload: Dictionary = identifier
		if payload.has("group_id") and payload.has("map_id"):
			var grouped := _group_lookup_key(payload)
			if _map_key_by_group.has(grouped):
				return _coerce_string(_map_key_by_group[grouped], "")
		for key in ["map_key", "map_name", "name", "map_constant", "constant", "mapConstant", "mapName"]:
			if payload.has(key):
				var resolved := _resolve_map_key(payload.get(key, ""))
				if not resolved.is_empty():
					return resolved
		return ""
	var normalized := _normalize_identifier(identifier)
	if normalized.is_empty():
		return ""
	if _map_key_by_identifier.has(normalized):
		return _coerce_string(_map_key_by_identifier[normalized], "")
	if _map_key_by_name.has(normalized):
		return _coerce_string(_map_key_by_name[normalized], "")
	if _map_key_by_constant.has(normalized):
		return _coerce_string(_map_key_by_constant[normalized], "")
	if _map_key_by_group.has(normalized):
		return _coerce_string(_map_key_by_group[normalized], "")
	return _resolve_map_key_from_lookup(normalized)

func _resolve_map_key_from_lookup(normalized_identifier: String) -> String:
	for key in runtime_map_metadata.keys():
		var runtime_entry := _normalize_runtime_entry(runtime_map_metadata.get(key, {}), _coerce_string(key, ""))
		var candidates := [
			_coerce_string(key, ""),
			_coerce_string(runtime_entry.get("name", ""), ""),
			_coerce_string(runtime_entry.get("constant", ""), ""),
			_group_lookup_key(runtime_entry),
		]
		for candidate in candidates:
			if _normalize_identifier(candidate) == normalized_identifier:
				return _coerce_string(runtime_entry.get("constant", key), key)
	for key in map_attributes.keys():
		var attribute_entry := _normalize_map_attribute_entry(map_attributes.get(key, {}), _coerce_string(key, ""))
		var candidates := [
			_coerce_string(key, ""),
			_coerce_string(attribute_entry.get("name", ""), ""),
			_coerce_string(attribute_entry.get("map_constant", ""), ""),
			_group_lookup_key(attribute_entry),
		]
		for candidate in candidates:
			if _normalize_identifier(candidate) == normalized_identifier:
				return _coerce_string(attribute_entry.get("map_constant", key), key)
	return ""

func _resolve_map_name(identifier: Variant) -> String:
	if typeof(identifier) == TYPE_DICTIONARY:
		var payload: Dictionary = identifier
		for key in ["map_name", "name", "mapName"]:
			if payload.has(key):
				var value := _coerce_string(payload.get(key, ""), "")
				if not value.is_empty():
					return value
		if payload.has("map_constant"):
			return _map_name_for_constant(_coerce_string(payload.get("map_constant", ""), ""))
		if payload.has("constant"):
			return _map_name_for_constant(_coerce_string(payload.get("constant", ""), ""))
		return ""
	var key := _resolve_map_key(identifier)
	if not key.is_empty():
		return _map_name_for_key(key)
	var normalized := _normalize_identifier(identifier)
	if normalized.is_empty():
		return ""
	for key_name in map_attributes.keys():
		var attribute_entry := _normalize_map_attribute_entry(map_attributes.get(key_name, {}), _coerce_string(key_name, ""))
		var candidates := [
			_coerce_string(key_name, ""),
			_coerce_string(attribute_entry.get("map_constant", ""), ""),
		]
		for candidate in candidates:
			if _normalize_identifier(candidate) == normalized:
				return _coerce_string(attribute_entry.get("name", key_name), key_name)
	return ""

func _map_name_for_key(map_key: String) -> String:
	var runtime_entry := _find_runtime_entry(map_key)
	if not runtime_entry.is_empty():
		var runtime_name := _coerce_string(runtime_entry.get("name", ""), "")
		if not runtime_name.is_empty():
			return runtime_name
	var attribute_entry := _find_attribute_entry(map_key)
	if not attribute_entry.is_empty():
		var attribute_name := _coerce_string(attribute_entry.get("name", ""), "")
		if not attribute_name.is_empty():
			return attribute_name
	return _coerce_string(map_key, "")

func _map_name_for_constant(map_constant: String) -> String:
	var normalized := _normalize_identifier(map_constant)
	if normalized.is_empty():
		return ""
	for key in map_attributes.keys():
		var entry := _normalize_map_attribute_entry(map_attributes.get(key, {}))
		var entry_constant := _coerce_string(entry.get("map_constant", ""), "")
		if _normalize_identifier(entry_constant) == normalized:
			return _coerce_string(entry.get("name", key), key)
	return ""

func _find_runtime_entry(identifier: Variant) -> Dictionary:
	var resolved := _resolve_map_key(identifier)
	if resolved.is_empty():
		resolved = _resolve_map_key(_resolve_map_name(identifier))
	if resolved.is_empty():
		return {}
	for key in runtime_map_metadata.keys():
		var entry := _normalize_runtime_entry(runtime_map_metadata.get(key, {}), _coerce_string(key, ""))
		var entry_key := _coerce_string(entry.get("constant", key), key)
		if _normalize_identifier(entry_key) == _normalize_identifier(resolved):
			return entry
		var entry_name := _coerce_string(entry.get("name", ""), "")
		if _normalize_identifier(entry_name) == _normalize_identifier(resolved):
			return entry
	return {}

func _find_attribute_entry(identifier: Variant) -> Dictionary:
	var resolved := _resolve_map_name(identifier)
	if resolved.is_empty():
		resolved = _coerce_string(identifier, "")
	if resolved.is_empty():
		return {}
	for key in map_attributes.keys():
		var entry := _normalize_map_attribute_entry(map_attributes.get(key, {}), _coerce_string(key, ""))
		var entry_name := _coerce_string(entry.get("name", key), key)
		if _normalize_identifier(entry_name) == _normalize_identifier(resolved):
			return entry
		var entry_constant := _coerce_string(entry.get("map_constant", ""), "")
		if _normalize_identifier(entry_constant) == _normalize_identifier(resolved):
			return entry
		if _normalize_identifier(_coerce_string(key, "")) == _normalize_identifier(resolved):
			return entry
	return {}

func _choose_default_spawn_point() -> Dictionary:
	for constant in DEFAULT_SPAWN_PRIORITY:
		var spawn := _find_spawn_point_by_constant(constant)
		if not spawn.is_empty():
			return spawn
	if runtime_spawn_points.is_empty():
		return {}
	var keys := runtime_spawn_points.keys()
	keys.sort()
	if keys.is_empty():
		return {}
	return _normalize_spawn_point(runtime_spawn_points.get(keys[0], {}))

func _find_spawn_point_by_constant(map_constant: String) -> Dictionary:
	var normalized := _normalize_identifier(map_constant)
	if normalized.is_empty():
		return {}
	for key in runtime_spawn_points.keys():
		var spawn := _normalize_spawn_point(runtime_spawn_points.get(key, {}))
		if _normalize_identifier(_coerce_string(spawn.get("map_constant", ""), "")) == normalized:
			return spawn
	return {}

func _spawn_matches_identifier(spawn: Dictionary, resolved_key: String, identifier: Variant) -> bool:
	if spawn.is_empty():
		return false
	var spawn_name := _coerce_string(spawn.get("map_name", ""), "")
	var spawn_constant := _coerce_string(spawn.get("map_constant", ""), "")
	var candidates := [
		normalized_spawn_lookup(spawn_name),
		normalized_spawn_lookup(spawn_constant),
		normalized_spawn_lookup(_map_name_for_key(resolved_key)),
		normalized_spawn_lookup(_resolve_map_name(identifier)),
	]
	if not resolved_key.is_empty():
		candidates.append(_normalize_identifier(resolved_key))
		if _normalize_identifier(spawn_name) == _normalize_identifier(_map_name_for_key(resolved_key)):
			return true
		if _normalize_identifier(spawn_constant) == _normalize_identifier(_map_name_for_key(resolved_key)):
			return true
	for candidate in candidates:
		if not candidate.is_empty() and _normalize_identifier(spawn_name) == candidate:
			return true
		if not candidate.is_empty() and _normalize_identifier(spawn_constant) == candidate:
			return true
	var group_id := _coerce_int(spawn.get("group_id", -1), -1)
	var map_id := _coerce_int(spawn.get("map_id", -1), -1)
	if group_id >= 0 and map_id >= 0 and typeof(identifier) == TYPE_DICTIONARY:
		var payload: Dictionary = identifier
		if payload.has("group_id") and payload.has("map_id"):
			return _coerce_int(payload.get("group_id", -1), -1) == group_id and _coerce_int(payload.get("map_id", -1), -1) == map_id
	return false

func normalized_spawn_lookup(value: String) -> String:
	return _normalize_identifier(value)

func _normalize_spawn_point(spawn_point: Variant) -> Dictionary:
	var source := _sanitize_dictionary(spawn_point, {})
	if source.is_empty():
		return {}
	var identifier := _coerce_int(_first_present(source, ["identifier", "id"]), -1)
	var map_name := _coerce_string(_first_present(source, ["mapName", "map_name"]), "")
	var map_constant := _coerce_string(_first_present(source, ["mapConstant", "map_constant"]), "")
	var group_name := _coerce_string(_first_present(source, ["groupName", "group_name"]), "")
	var group_id := _coerce_int(_first_present(source, ["groupId", "group_id"]), -1)
	var map_id := _coerce_int(_first_present(source, ["mapId", "map_id"]), -1)
	var tile_x := _coerce_int(_first_present(source, ["tileX", "tile_x", "x", "player_x"]), 0)
	var tile_y := _coerce_int(_first_present(source, ["tileY", "tile_y", "y", "player_y"]), 0)
	var metatile_x := _coerce_int(_first_present(source, ["metatileX", "metatile_x"]), 0)
	var metatile_y := _coerce_int(_first_present(source, ["metatileY", "metatile_y"]), 0)
	var subtile_x := _coerce_int(_first_present(source, ["subtileX", "subtile_x"]), 0)
	var subtile_y := _coerce_int(_first_present(source, ["subtileY", "subtile_y"]), 0)
	return {
		"identifier": identifier,
		"map_name": map_name,
		"mapName": map_name,
		"map_constant": map_constant,
		"mapConstant": map_constant,
		"group_name": group_name,
		"groupName": group_name,
		"group_id": group_id,
		"groupId": group_id,
		"map_id": map_id,
		"mapId": map_id,
		"player_tile": {
			"x": tile_x,
			"y": tile_y,
		},
		"tile_x": tile_x,
		"tileX": tile_x,
		"tile_y": tile_y,
		"tileY": tile_y,
		"metatile_x": metatile_x,
		"metatileX": metatile_x,
		"metatile_y": metatile_y,
		"metatileY": metatile_y,
		"subtile_x": subtile_x,
		"subtileX": subtile_x,
		"subtile_y": subtile_y,
		"subtileY": subtile_y,
	}

func _normalize_identifier(value: Variant) -> String:
	if typeof(value) == TYPE_INT:
		return str(int(value))
	if typeof(value) == TYPE_DICTIONARY:
		return _normalize_identifier(_first_present(Dictionary(value), ["map_key", "map_name", "map_constant", "name", "constant"]))
	var result := _coerce_string(value, "").to_lower()
	if result.is_empty():
		return ""
	var compact := ""
	for index in range(result.length()):
		var character := result.substr(index, 1)
		var code := character.unicode_at(0)
		if (code >= 48 and code <= 57) or (code >= 65 and code <= 90) or (code >= 97 and code <= 122) or code == 95:
			compact += character
	return compact

func _group_lookup_key(entry: Variant) -> String:
	var group_id := _coerce_int(_first_present(_sanitize_dictionary(entry, {}), ["groupId", "group_id"]), -1)
	var map_id := _coerce_int(_first_present(_sanitize_dictionary(entry, {}), ["mapId", "map_id"]), -1)
	if group_id < 0 or map_id < 0:
		return ""
	return "%d:%d" % [group_id, map_id]

func _parse_map_events(map_file: Dictionary) -> Dictionary:
	var result := {
		"coord_events": [],
		"warps": [],
		"bg_events": [],
		"object_events": [],
		"map_events_label": "",
		"map_scripts_label": "",
	}
	if map_file.is_empty():
		return result
	if _has_event_collection(map_file):
		return _normalize_event_collection(map_file)
	var nested_events: Dictionary = _sanitize_dictionary(_first_present(map_file, ["events", "map_events", "mapEvents"]), {})
	if not nested_events.is_empty() and _has_event_collection(nested_events):
		var nested_result := _normalize_event_collection(nested_events)
		nested_result["map_events_label"] = _coerce_string(_first_present(map_file, ["map_events_label", "mapEventsLabel"]), "")
		nested_result["map_scripts_label"] = _coerce_string(_first_present(map_file, ["map_scripts_label", "mapScriptsLabel"]), "")
		return nested_result
	var warp_index := 0
	for key in map_file.keys():
		var label := str(key)
		if result["map_events_label"].is_empty() and label.ends_with("_MapEvents"):
			result["map_events_label"] = label
		if result["map_scripts_label"].is_empty() and label.ends_with("_MapScripts"):
			result["map_scripts_label"] = label
		var commands: Variant = map_file[key]
		if typeof(commands) != TYPE_ARRAY:
			continue
		for command in Array(commands):
			if typeof(command) != TYPE_DICTIONARY:
				continue
			var command_name := str(command.get("command", ""))
			var args: Array = _sanitize_array(command.get("args", []), [])
			match command_name:
				"warp_event":
					if args.size() >= 4:
						(result["warps"] as Array).append({
							"index": warp_index,
							"warp_id": warp_index + 1,
							"warpId": warp_index + 1,
							"x": _coerce_int(args[0], 0),
							"y": _coerce_int(args[1], 0),
							"target_map_constant": _resolve_map_constant(str(args[2])),
							"target_map": str(args[2]),
							"target_warp_id": _coerce_int(args[3], 0),
							"command": "warp_event",
							"args": args.duplicate(true),
							"command_payload": _command_payload_from_args("warp_event", args),
						})
						warp_index += 1
				"coord_event":
					if args.size() >= 4:
						(result["coord_events"] as Array).append({
							"x": _coerce_int(args[0], 0),
							"y": _coerce_int(args[1], 0),
							"scene_id": str(args[2]),
							"script_name": str(args[3]),
							"command": "coord_event",
							"args": args.duplicate(true),
							"command_payload": _command_payload_from_args("coord_event", args),
						})
				"bg_event":
					if args.size() >= 4:
						(result["bg_events"] as Array).append({
							"x": _coerce_int(args[0], 0),
							"y": _coerce_int(args[1], 0),
							"event_type": str(args[2]),
							"script": str(args[3]),
							"command": "bg_event",
							"args": args.duplicate(true),
							"command_payload": _command_payload_from_args("bg_event", args),
						})
				"object_event":
					if args.size() >= 13:
						(result["object_events"] as Array).append({
							"x": _coerce_int(args[0], 0),
							"y": _coerce_int(args[1], 0),
							"sprite": str(args[2]),
							"spritemovedata": str(args[3]),
							"move_range_x": _coerce_int(args[4], 0),
							"move_range_y": _coerce_int(args[5], 0),
							"hram_x": _coerce_int(args[6], 0),
							"hram_y": _coerce_int(args[7], 0),
							"pal": _coerce_int(args[8], 0),
							"object_type": str(args[9]),
							"radius": _coerce_int(args[10], 0),
							"script": str(args[11]),
							"event_flag": str(args[12]),
							"label": null,
							"object_identifier": null,
							"sightline_direction_override": null,
							"command": "object_event",
							"args": args.duplicate(true),
							"command_payload": _command_payload_from_args("object_event", args),
						})
	return result

func _normalize_warps(value: Variant) -> Array:
	var result: Array = []
	var source: Array = _sanitize_array(value, [])
	var index: int = 0
	for item in source:
		var event: Dictionary = _sanitize_dictionary(item, {})
		if event.is_empty():
			index += 1
			continue
		var target: String = _coerce_string(_first_present(event, ["target_map_constant", "targetMapConstant", "target_map", "targetMap", "map"]), "")
		result.append({
			"index": _coerce_int(_first_present(event, ["index", "warp_index", "warpIndex"]), index),
			"warp_id": _coerce_int(_first_present(event, ["warp_id", "warpId", "source_warp_id", "sourceWarpId"]), index + 1),
			"warpId": _coerce_int(_first_present(event, ["warp_id", "warpId", "source_warp_id", "sourceWarpId"]), index + 1),
			"x": _coerce_int(_first_present(event, ["x", "tile_x", "tileX"]), 0),
			"y": _coerce_int(_first_present(event, ["y", "tile_y", "tileY"]), 0),
			"target_map_constant": _resolve_map_constant(target),
			"target_map": _coerce_string(_first_present(event, ["target_map", "targetMap", "map"]), target),
			"target_warp_id": _coerce_int(_first_present(event, ["target_warp_id", "targetWarpId", "warp_id", "warpId"]), 0),
			"command": "warp_event",
			"args": _event_args(event, "warp_event"),
			"command_payload": _event_command_payload(event, "warp_event"),
		})
		index += 1
	return result

func _normalize_coord_events(value: Variant) -> Array:
	var result: Array = []
	for item in _sanitize_array(value, []):
		var event: Dictionary = _sanitize_dictionary(item, {})
		if event.is_empty():
			continue
		result.append({
			"x": _coerce_int(_first_present(event, ["x", "tile_x", "tileX"]), 0),
			"y": _coerce_int(_first_present(event, ["y", "tile_y", "tileY"]), 0),
			"scene_id": _coerce_string(_first_present(event, ["scene_id", "sceneId", "scene"]), ""),
			"script_name": _coerce_string(_first_present(event, ["script_name", "scriptName", "script"]), ""),
			"command": "coord_event",
			"args": _event_args(event, "coord_event"),
			"command_payload": _event_command_payload(event, "coord_event"),
		})
	return result

func _normalize_bg_events(value: Variant) -> Array:
	var result: Array = []
	for item in _sanitize_array(value, []):
		var event: Dictionary = _sanitize_dictionary(item, {})
		if event.is_empty():
			continue
		result.append({
			"x": _coerce_int(_first_present(event, ["x", "tile_x", "tileX"]), 0),
			"y": _coerce_int(_first_present(event, ["y", "tile_y", "tileY"]), 0),
			"event_type": _coerce_string(_first_present(event, ["event_type", "eventType", "type"]), ""),
			"script": _coerce_string(_first_present(event, ["script", "script_name", "scriptName"]), ""),
			"command": "bg_event",
			"args": _event_args(event, "bg_event"),
			"command_payload": _event_command_payload(event, "bg_event"),
		})
	return result

func _normalize_object_events(value: Variant) -> Array:
	var result: Array = []
	for item in _sanitize_array(value, []):
		var event: Dictionary = _sanitize_dictionary(item, {})
		if event.is_empty():
			continue
		result.append({
			"x": _coerce_int(_first_present(event, ["x", "tile_x", "tileX"]), 0),
			"y": _coerce_int(_first_present(event, ["y", "tile_y", "tileY"]), 0),
			"sprite": _coerce_string(_first_present(event, ["sprite"]), ""),
			"spritemovedata": _coerce_string(_first_present(event, ["spritemovedata", "sprite_move_data", "spriteMoveData", "movement"]), ""),
			"move_range_x": _coerce_int(_first_present(event, ["move_range_x", "moveRangeX"]), 0),
			"move_range_y": _coerce_int(_first_present(event, ["move_range_y", "moveRangeY"]), 0),
			"hram_x": _coerce_int(_first_present(event, ["hram_x", "hramX"]), -1),
			"hram_y": _coerce_int(_first_present(event, ["hram_y", "hramY"]), -1),
			"pal": _coerce_int(_first_present(event, ["pal", "palette"]), 0),
			"object_type": _coerce_string(_first_present(event, ["object_type", "objectType", "type"]), ""),
			"radius": _coerce_int(_first_present(event, ["radius"]), 0),
			"script": _coerce_string(_first_present(event, ["script", "script_name", "scriptName"]), ""),
			"event_flag": _coerce_string(_first_present(event, ["event_flag", "eventFlag", "flag"]), ""),
			"label": _first_present(event, ["label"]),
			"object_identifier": _first_present(event, ["object_identifier", "objectIdentifier", "identifier"]),
			"sightline_direction_override": _first_present(event, ["sightline_direction_override", "sightlineDirectionOverride"]),
			"command": "object_event",
			"args": _event_args(event, "object_event"),
			"command_payload": _event_command_payload(event, "object_event"),
		})
	return result

func _events_at_tile_from_collection(x: int, y: int, source: Dictionary) -> Dictionary:
	var warps: Array = []
	var coord_events: Array = []
	var bg_events: Array = []
	var object_events: Array = []
	for warp in _normalize_warps(source.get("warps", [])):
		var warp_event: Dictionary = Dictionary(warp)
		if _coerce_int(warp_event.get("x", -1), -1) == x and _coerce_int(warp_event.get("y", -1), -1) == y:
			warps.append(warp_event)
	for coord in _normalize_coord_events(source.get("coord_events", [])):
		var coord_event: Dictionary = Dictionary(coord)
		if _coerce_int(coord_event.get("x", -1), -1) == x and _coerce_int(coord_event.get("y", -1), -1) == y:
			coord_events.append(coord_event)
	for bg in _normalize_bg_events(source.get("bg_events", [])):
		var bg_event: Dictionary = Dictionary(bg)
		if _coerce_int(bg_event.get("x", -1), -1) == x and _coerce_int(bg_event.get("y", -1), -1) == y:
			bg_events.append(bg_event)
	for object_entry in _normalize_object_events(source.get("object_events", [])):
		var object_event: Dictionary = Dictionary(object_entry)
		if _coerce_int(object_event.get("x", -1), -1) == x and _coerce_int(object_event.get("y", -1), -1) == y:
			object_events.append(object_event)
	return {
		"x": x,
		"y": y,
		"warps": warps,
		"coord_events": coord_events,
		"bg_events": bg_events,
		"object_events": object_events,
	}

func _serialize_object_event_record(event: Dictionary, index: int) -> Dictionary:
	var object_id: int = _coerce_int(_first_present(event, ["object_id", "objectId", "object_identifier", "objectIdentifier", "identifier"]), index + 1)
	return {
		"index": index,
		"object_id": object_id,
		"objectId": object_id,
		"x": _coerce_int(event.get("x", 0), 0),
		"y": _coerce_int(event.get("y", 0), 0),
		"sprite": _coerce_string(event.get("sprite", ""), ""),
		"spritemovedata": _coerce_string(event.get("spritemovedata", ""), ""),
		"move_range_x": _coerce_int(event.get("move_range_x", 0), 0),
		"move_range_y": _coerce_int(event.get("move_range_y", 0), 0),
		"hram_x": _coerce_int(event.get("hram_x", -1), -1),
		"hram_y": _coerce_int(event.get("hram_y", -1), -1),
		"pal": _coerce_int(event.get("pal", 0), 0),
		"object_type": _coerce_string(event.get("object_type", ""), ""),
		"radius": _coerce_int(event.get("radius", 0), 0),
		"script": _coerce_string(event.get("script", ""), ""),
		"event_flag": _coerce_string(event.get("event_flag", ""), ""),
		"label": _normalize_variant(event.get("label", null)),
		"object_identifier": _normalize_variant(event.get("object_identifier", null)),
		"sightline_direction_override": _normalize_variant(event.get("sightline_direction_override", null)),
		"command": "object_event",
		"args": _event_args(event, "object_event"),
		"command_payload": _event_command_payload(event, "object_event"),
	}

func _map_script_metadata_without_refs(map_identifier: Variant = "") -> Dictionary:
	var payload: Dictionary = current_map_payload if _is_current_map_identifier(map_identifier) else build_map_payload(map_identifier)
	var map_file: Dictionary = _map_file_for_identifier(map_identifier)
	var scripts_label: String = _coerce_string(payload.get("map_scripts_label", ""), "")
	var script_commands: Array = _sanitize_array(map_file.get(scripts_label, []), [])
	return {
		"map_scripts_label": scripts_label,
		"scene_scripts": _parse_scene_script_metadata(script_commands),
		"callbacks": _parse_callback_metadata(script_commands),
	}

func _command_payloads_from_events(events: Array, command_name: String) -> Array:
	var payloads: Array = []
	for event in events:
		payloads.append(_serialize_command_payload(_sanitize_dictionary(event, {}), command_name))
	return payloads

func _serialize_command_payload(event: Dictionary, command_name: String) -> Dictionary:
	var payload: Dictionary = _event_command_payload(event, command_name)
	if payload.is_empty():
		payload = _command_payload_from_args(command_name, _event_args(event, command_name))
	return payload

func _event_args(event: Dictionary, command_name: String) -> Array:
	var existing: Array = _sanitize_array(event.get("args", []), [])
	if not existing.is_empty():
		return existing.duplicate(true)
	var payload: Dictionary = _sanitize_dictionary(event.get("command_payload", {}), {})
	var payload_args: Array = _sanitize_array(payload.get("args", []), [])
	if not payload_args.is_empty():
		return payload_args.duplicate(true)
	match command_name:
		"warp_event":
			return [
				_coerce_int(event.get("x", 0), 0),
				_coerce_int(event.get("y", 0), 0),
				_coerce_string(event.get("target_map_constant", event.get("target_map", "")), ""),
				_coerce_int(event.get("target_warp_id", 0), 0),
			]
		"coord_event":
			return [
				_coerce_int(event.get("x", 0), 0),
				_coerce_int(event.get("y", 0), 0),
				_coerce_string(event.get("scene_id", ""), ""),
				_coerce_string(event.get("script_name", ""), ""),
			]
		"bg_event":
			return [
				_coerce_int(event.get("x", 0), 0),
				_coerce_int(event.get("y", 0), 0),
				_coerce_string(event.get("event_type", ""), ""),
				_coerce_string(event.get("script", ""), ""),
			]
		"object_event":
			return [
				_coerce_int(event.get("x", 0), 0),
				_coerce_int(event.get("y", 0), 0),
				_coerce_string(event.get("sprite", ""), ""),
				_coerce_string(event.get("spritemovedata", ""), ""),
				_coerce_int(event.get("move_range_x", 0), 0),
				_coerce_int(event.get("move_range_y", 0), 0),
				_coerce_int(event.get("hram_x", -1), -1),
				_coerce_int(event.get("hram_y", -1), -1),
				_normalize_variant(event.get("pal", 0)),
				_coerce_string(event.get("object_type", ""), ""),
				_coerce_int(event.get("radius", 0), 0),
				_coerce_string(event.get("script", ""), ""),
				_coerce_string(event.get("event_flag", ""), ""),
			]
		_:
			return []

func _event_command_payload(event: Dictionary, command_name: String) -> Dictionary:
	var payload: Dictionary = _sanitize_dictionary(event.get("command_payload", {}), {})
	if not payload.is_empty():
		payload["command"] = _coerce_string(payload.get("command", command_name), command_name)
		payload["args"] = _sanitize_array(payload.get("args", _event_args(event, command_name)), [])
		return payload
	return _command_payload_from_args(command_name, _event_args(event, command_name))

func _command_payload_from_args(command_name: String, args: Array) -> Dictionary:
	var payload: Dictionary = {
		"command": command_name,
		"args": args.duplicate(true),
	}
	match command_name:
		"warp_event":
			if args.size() >= 4:
				payload["x"] = _coerce_int(args[0], 0)
				payload["y"] = _coerce_int(args[1], 0)
				payload["target_map_constant"] = _resolve_map_constant(str(args[2]))
				payload["target_map"] = str(args[2])
				payload["target_warp_id"] = _coerce_int(args[3], 0)
		"coord_event":
			if args.size() >= 4:
				payload["x"] = _coerce_int(args[0], 0)
				payload["y"] = _coerce_int(args[1], 0)
				payload["scene_id"] = str(args[2])
				payload["script_name"] = str(args[3])
		"bg_event":
			if args.size() >= 4:
				payload["x"] = _coerce_int(args[0], 0)
				payload["y"] = _coerce_int(args[1], 0)
				payload["event_type"] = str(args[2])
				payload["script"] = str(args[3])
		"object_event":
			if args.size() >= 13:
				payload["x"] = _coerce_int(args[0], 0)
				payload["y"] = _coerce_int(args[1], 0)
				payload["sprite"] = str(args[2])
				payload["spritemovedata"] = str(args[3])
				payload["move_range_x"] = _coerce_int(args[4], 0)
				payload["move_range_y"] = _coerce_int(args[5], 0)
				payload["hram_x"] = _coerce_int(args[6], -1)
				payload["hram_y"] = _coerce_int(args[7], -1)
				payload["pal"] = _normalize_variant(args[8])
				payload["object_type"] = str(args[9])
				payload["radius"] = _coerce_int(args[10], 0)
				payload["script"] = str(args[11])
				payload["event_flag"] = str(args[12])
	return _sanitize_dictionary(payload, {})

func _resolve_tileset_name(identifier: Variant) -> String:
	if typeof(identifier) == TYPE_NIL:
		return current_tileset_name
	var direct: String = _coerce_string(identifier, "")
	if typeof(identifier) == TYPE_DICTIONARY:
		var source: Dictionary = Dictionary(identifier)
		direct = _coerce_string(_first_present(source, ["tileset_name", "tilesetName", "tileset"]), "")
		if direct.is_empty():
			var payload: Dictionary = build_map_payload(identifier)
			direct = _coerce_string(payload.get("tileset_name", ""), "")
	elif not direct.is_empty() and _has_map_identifier(identifier):
		var payload: Dictionary = build_map_payload(identifier)
		direct = _coerce_string(payload.get("tileset_name", direct), direct)
	if direct.is_empty():
		direct = current_tileset_name
	return direct

func _load_tileset_metatiles(tileset_name: String) -> PackedByteArray:
	if tileset_name.is_empty():
		return PackedByteArray()
	if asset_index != null and asset_index.has_method("load_tileset_metatiles"):
		var loaded: Variant = asset_index.call("load_tileset_metatiles", tileset_name)
		if typeof(loaded) == TYPE_PACKED_BYTE_ARRAY:
			return PackedByteArray(loaded)
	return PackedByteArray()

func _decode_block_bytes(encoded: String) -> PackedByteArray:
	if encoded.is_empty():
		return PackedByteArray()
	return Marshalls.base64_to_raw(encoded)

func _packed_bytes_to_array(bytes: PackedByteArray) -> Array:
	var result: Array = []
	for byte in bytes:
		result.append(int(byte))
	return result

func _metatile_key(metatile_id: int) -> String:
	var hex: String = "%02X" % max(0, metatile_id)
	return hex

func _is_current_map_identifier(identifier: Variant) -> bool:
	if typeof(identifier) == TYPE_NIL:
		return true
	var raw: String = _coerce_string(identifier, "")
	if raw.is_empty():
		return true
	var resolved: String = _resolve_map_key(identifier)
	if resolved.is_empty():
		resolved = _resolve_map_name(identifier)
	return _normalize_identifier(resolved) == _normalize_identifier(current_map_key) or _normalize_identifier(resolved) == _normalize_identifier(current_map_name)

func _count_entries(value: Variant) -> int:
	match typeof(value):
		TYPE_ARRAY:
			return Array(value).size()
		TYPE_DICTIONARY:
			return Dictionary(value).size()
		_:
			return 0

func _resolve_collision_value(value: Variant) -> int:
	if typeof(value) == TYPE_INT:
		return int(value)
	if typeof(value) == TYPE_FLOAT:
		return int(value)
	var token: String = _coerce_string(value, "")
	if token.is_empty():
		return -1
	if token.begins_with("$"):
		return token.substr(1).hex_to_int()
	if token.to_lower().begins_with("0x"):
		return token.substr(2).hex_to_int()
	if token.is_valid_int():
		return int(token)
	var normalized: String = _normalize_collision_token(token)
	for entry in collision_permissions:
		var record: Dictionary = _sanitize_dictionary(entry, {})
		var comment: String = _normalize_collision_token(_coerce_string(record.get("comment", ""), ""))
		if comment == normalized or comment == "coll_%s" % normalized:
			return _coerce_int(record.get("value", -1), -1)
	return -1

func _collision_constant_name(value: Variant) -> String:
	var normalized_value: int = _resolve_collision_value(value)
	for entry in collision_permissions:
		var record: Dictionary = _sanitize_dictionary(entry, {})
		if _coerce_int(record.get("value", -1), -1) == normalized_value:
			return _coerce_string(record.get("comment", ""), "")
	var token: String = _coerce_string(value, "")
	if token.is_empty():
		return ""
	if token.to_upper().begins_with("COLL_"):
		return token.to_upper()
	return "COLL_%s" % token.to_upper()

func _normalize_collision_permission(record: Dictionary) -> Dictionary:
	var value: int = _coerce_int(record.get("value", -1), -1)
	var constant: String = _coerce_string(record.get("comment", ""), "")
	return {
		"value": value,
		"terrain": _coerce_string(record.get("terrain", ""), ""),
		"talk": _coerce_bool(record.get("talk", false), false),
		"raw_expr": _coerce_string(record.get("raw_expr", ""), ""),
		"comment": constant,
		"constant": constant,
		"stdscript": _coerce_string(collision_stdscripts.get(constant, ""), ""),
	}

func _normalize_collision_token(value: String) -> String:
	var token: String = value.strip_edges().to_lower()
	if token.begins_with("coll_"):
		token = token.substr(5)
	var compact: String = ""
	for index in range(token.length()):
		var character: String = token.substr(index, 1)
		var code: int = character.unicode_at(0)
		if (code >= 48 and code <= 57) or (code >= 97 and code <= 122) or code == 95:
			compact += character
	return compact

func _ensure_asset_index() -> void:
	if asset_index == null:
		asset_index = ASSET_INDEX_SCRIPT.new()

func _ensure_assets_loaded() -> void:
	if _loading_assets or _assets_refreshed:
		return
	if runtime_map_metadata.is_empty() and runtime_spawn_points.is_empty() and map_attributes.is_empty() and map_blocks.is_empty():
		refresh_assets()

func _resolve_map_constant(identifier: Variant) -> String:
	var attribute_entry := _find_attribute_entry(identifier)
	if not attribute_entry.is_empty():
		var attribute_constant := _coerce_string(attribute_entry.get("map_constant", ""), "")
		if not attribute_constant.is_empty():
			return attribute_constant
	var runtime_entry := _find_runtime_entry(identifier)
	if not runtime_entry.is_empty():
		var runtime_constant := _coerce_string(runtime_entry.get("constant", ""), "")
		if not runtime_constant.is_empty():
			return runtime_constant
	return _coerce_string(identifier, "")

func _has_map_identifier(identifier: Variant) -> bool:
	return not _resolve_map_key(identifier).is_empty() or not _resolve_map_name(identifier).is_empty()

func _clear_current_map() -> void:
	current_map_key = ""
	current_map_name = ""
	current_map_constant = ""
	current_map_group_name = ""
	current_group_id = -1
	current_map_group_id = -1
	current_map_id = -1
	current_environment = ""
	current_music = ""
	current_tileset_name = ""
	current_location = ""
	current_phone_service = 0
	current_width = 0
	current_height = 0
	current_border_block = 0
	current_connections = []
	current_coord_events = []
	current_warps = []
	current_bg_events = []
	current_object_events = []
	current_spawn_points = []
	current_spawn_point = {}
	current_blocks_label = ""
	current_map_events_label = ""
	current_map_scripts_label = ""
	current_map_payload = {}
	_update_summary()

func _first_present(source: Dictionary, keys: Array) -> Variant:
	for key in keys:
		if source.has(key):
			return source.get(key)
	return null

func _coerce_string(value: Variant, fallback: String) -> String:
	if typeof(value) == TYPE_NIL:
		return fallback
	var result := str(value).strip_edges()
	if result.is_empty():
		return fallback
	return result

func _coerce_int(value: Variant, fallback: int) -> int:
	if typeof(value) == TYPE_NIL:
		return fallback
	return int(value)

func _coerce_bool(value: Variant, fallback: bool) -> bool:
	if typeof(value) == TYPE_NIL:
		return fallback
	if typeof(value) == TYPE_BOOL:
		return bool(value)
	if typeof(value) == TYPE_STRING:
		var normalized := str(value).strip_edges().to_lower()
		if normalized in ["true", "1", "yes", "on"]:
			return true
		if normalized in ["false", "0", "no", "off"]:
			return false
	return bool(value)

func _coerce_float(value: Variant, fallback: float) -> float:
	if typeof(value) == TYPE_NIL:
		return fallback
	return float(value)

func _sanitize_dictionary(value: Variant, defaults: Dictionary) -> Dictionary:
	var result: Dictionary = defaults.duplicate(true)
	if typeof(value) != TYPE_DICTIONARY:
		return result
	var source: Dictionary = value
	for key in source.keys():
		var raw_value: Variant = source[key]
		if result.has(key):
			result[key] = _sanitize_value(raw_value, result[key])
		else:
			result[key] = _normalize_variant(raw_value)
	return result

func _sanitize_array(value: Variant, defaults: Array) -> Array:
	var result: Array = defaults.duplicate(true)
	if typeof(value) != TYPE_ARRAY:
		return result
	var source: Array = value
	for index in range(source.size()):
		var template: Variant = null
		if index < result.size():
			template = result[index]
			result[index] = _sanitize_value(source[index], template)
		else:
			if not defaults.is_empty():
				template = defaults[defaults.size() - 1]
			result.append(_sanitize_value(source[index], template))
	return result

func _sanitize_value(value: Variant, template: Variant) -> Variant:
	match typeof(template):
		TYPE_DICTIONARY:
			return _sanitize_dictionary(value, template)
		TYPE_ARRAY:
			return _sanitize_array(value, template)
		TYPE_STRING:
			return _coerce_string(value, str(template))
		TYPE_INT:
			return _coerce_int(value, int(template))
		TYPE_FLOAT:
			return _coerce_float(value, float(template))
		TYPE_BOOL:
			return _coerce_bool(value, bool(template))
		TYPE_NIL:
			return _normalize_variant(value)
		_:
			if typeof(value) == typeof(template):
				return _normalize_variant(value)
			return template

func _normalize_variant(value: Variant) -> Variant:
	match typeof(value):
		TYPE_DICTIONARY:
			var normalized: Dictionary = {}
			var source: Dictionary = value
			for key in source.keys():
				normalized[key] = _normalize_variant(source[key])
			return normalized
		TYPE_ARRAY:
			var normalized_array: Array = []
			var source_array: Array = value
			for entry in source_array:
				normalized_array.append(_normalize_variant(entry))
			return normalized_array
		TYPE_STRING, TYPE_INT, TYPE_FLOAT, TYPE_BOOL, TYPE_NIL, TYPE_VECTOR2I:
			return value
		_:
			return null
