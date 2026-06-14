extends RefCounted
class_name BattleAssets

const ASSET_INDEX_SCRIPT := preload("res://scripts/asset_index.gd")
const GB_TILE_DECODER_SCRIPT := preload("res://scripts/gb_tile_decoder.gd")
const BATTLE_DATA_FILES := {
	"pokemon_data": "pokemon_data.json",
	"move_data": "moves_data.json",
	"item_data": "items.json",
	"trainer_data": "trainers.json",
	"battle_animation_table": "battle_animation_table.json",
	"battle_anim_bundle": "battle_anim_bundle.json",
	"pokemon_frontpic_anim": "pokemon_frontpic_anim.json",
	"content_pack_index": "content-packs/index.json",
}

var asset_index = null
var pokemon_data: Array = []
var move_data: Dictionary = {}
var item_data: Array = []
var trainer_data: Array = []
var battle_animation_table: Array = []
var battle_anim_bundle: Dictionary = {}
var pokemon_frontpic_anim: Dictionary = {}
var content_pack_index: Dictionary = {}
var summary: Dictionary = {}
var loaded: bool = false

func _init(index = null) -> void:
	asset_index = index if index != null else ASSET_INDEX_SCRIPT.new()

func initialize() -> void:
	asset_index.initialize()

func assets_path(relative_path: String) -> String:
	return asset_index.assets_path(relative_path)

func data_path(relative_path: String) -> String:
	return asset_index.data_path(relative_path)

func gfx_path(relative_path: String) -> String:
	return asset_index.gfx_path(relative_path)

func load_json(relative_path: String) -> Variant:
	return asset_index.load_json(relative_path)

func load_json_path(absolute_path: String) -> Variant:
	return asset_index.load_json_path(absolute_path)

func load_dictionary(relative_path: String) -> Dictionary:
	return asset_index.load_dictionary(relative_path)

func load_dictionary_path(absolute_path: String) -> Dictionary:
	return asset_index.load_dictionary_path(absolute_path)

func load_array(relative_path: String) -> Array:
	return asset_index.load_array(relative_path)

func load_array_path(absolute_path: String) -> Array:
	return asset_index.load_array_path(absolute_path)

func load_text(relative_path: String) -> String:
	return asset_index.load_text(relative_path)

func load_text_path(absolute_path: String) -> String:
	return asset_index.load_text_path(absolute_path)

func load_raw_bytes(relative_path: String) -> PackedByteArray:
	return asset_index.load_raw_bytes(relative_path)

func load_raw_bytes_path(absolute_path: String) -> PackedByteArray:
	return asset_index.load_raw_bytes_path(absolute_path)

func load_image(relative_path: String) -> Image:
	return asset_index.load_image(relative_path)

func load_image_path(absolute_path: String) -> Image:
	return asset_index.load_image_path(absolute_path)

func load_1bpp_tile(relative_path: String, palette: Array[Color] = []) -> Image:
	return GB_TILE_DECODER_SCRIPT.load_1bpp_tile(gfx_path(relative_path), palette)

func load_1bpp_tile_path(absolute_path: String, palette: Array[Color] = []) -> Image:
	return GB_TILE_DECODER_SCRIPT.load_1bpp_tile_path(absolute_path, palette)

func load_2bpp_tile(relative_path: String, palette: Array[Color] = []) -> Image:
	return GB_TILE_DECODER_SCRIPT.load_2bpp_tile(gfx_path(relative_path), palette)

func load_2bpp_tile_path(absolute_path: String, palette: Array[Color] = []) -> Image:
	return GB_TILE_DECODER_SCRIPT.load_2bpp_tile_path(absolute_path, palette)

func load_1bpp_tiles(relative_path: String, palette: Array[Color] = []) -> Array[Image]:
	return GB_TILE_DECODER_SCRIPT.load_1bpp_tiles(gfx_path(relative_path), palette)

func load_1bpp_tiles_path(absolute_path: String, palette: Array[Color] = []) -> Array[Image]:
	return GB_TILE_DECODER_SCRIPT.load_1bpp_tiles(absolute_path, palette)

func load_2bpp_tiles(relative_path: String, palette: Array[Color] = []) -> Array[Image]:
	return GB_TILE_DECODER_SCRIPT.load_2bpp_tiles(gfx_path(relative_path), palette)

func load_2bpp_tiles_path(absolute_path: String, palette: Array[Color] = []) -> Array[Image]:
	return GB_TILE_DECODER_SCRIPT.load_2bpp_tiles(absolute_path, palette)

func load_gbcpal_palette(relative_path: String) -> Array[Color]:
	return GB_TILE_DECODER_SCRIPT.load_gbcpal_palette(gfx_path(relative_path))

func load_gbcpal_palette_path(absolute_path: String) -> Array[Color]:
	return GB_TILE_DECODER_SCRIPT.load_gbcpal_palette_path(absolute_path)

func load_palette(relative_path: String, palette_name: String = "") -> Array[Color]:
	return GB_TILE_DECODER_SCRIPT.load_palette(gfx_path(relative_path), palette_name)

func load_palette_path(absolute_path: String, palette_name: String = "") -> Array[Color]:
	return GB_TILE_DECODER_SCRIPT.load_palette_path(absolute_path, palette_name)

func load_predef_palette(relative_path: String, palette_name: String) -> Array[Color]:
	return GB_TILE_DECODER_SCRIPT.load_predef_palette(gfx_path(relative_path), palette_name)

func load_predef_palette_path(absolute_path: String, palette_name: String) -> Array[Color]:
	return GB_TILE_DECODER_SCRIPT.load_predef_palette_path(absolute_path, palette_name)

func load_palette_bank(relative_path: String) -> Array:
	return asset_index.load_palette_bank(relative_path)

func load_palette_bank_path(absolute_path: String) -> Array:
	return asset_index.load_palette_bank_path(absolute_path)

func load_title_palette_bank() -> Array:
	return asset_index.load_title_palette_bank()

func load_title_palette_bank_path(absolute_path: String) -> Array:
	return asset_index.load_title_palette_bank_path(absolute_path)

func load_title_graphics() -> Dictionary:
	return asset_index.load_title_graphics()

func load_title_graphics_path() -> Dictionary:
	return asset_index.load_title_graphics_path()

func load_audio_manifest(relative_path: String) -> Dictionary:
	return asset_index.load_audio_manifest(relative_path)

func load_audio_manifest_path(absolute_path: String) -> Dictionary:
	return asset_index.load_audio_manifest_path(absolute_path)

func load_disassembly_aliases() -> Dictionary:
	return asset_index.load_disassembly_aliases()

func load_audio_manifests() -> Array[Dictionary]:
	return asset_index.load_audio_manifests()

func resolve_audio_manifest_entry(category: String, token: String) -> Dictionary:
	return asset_index.resolve_audio_manifest_entry(category, token)

func resolve_audio_cue(category: String, token: String) -> Dictionary:
	return asset_index.resolve_audio_cue(category, token)

func build_audio_playback_plan(category: String, token: String, options: Dictionary = {}) -> Dictionary:
	return asset_index.build_audio_playback_plan(category, token, options)

func build_audio_playback_plan_for_cue(cue: Dictionary, options: Dictionary = {}) -> Dictionary:
	return asset_index.build_audio_playback_plan_for_cue(cue, options)

func create_audio_playback_state() -> Dictionary:
	return asset_index.create_audio_playback_state()

func schedule_audio_playback_plan(state: Dictionary, plan: Dictionary) -> Dictionary:
	return asset_index.schedule_audio_playback_plan(state, plan)

func release_audio_playback_plan(state: Dictionary, plan_or_token: Variant) -> Dictionary:
	return asset_index.release_audio_playback_plan(state, plan_or_token)

func build_audio_playback_snapshot(state: Dictionary) -> Dictionary:
	return asset_index.build_audio_playback_snapshot(state)

func apply_audio_playback_plan(player: Object, plan: Dictionary) -> Dictionary:
	return asset_index.apply_audio_playback_plan(player, plan)

func play_audio_cue(player: Object, category: String, token: String, options: Dictionary = {}) -> Dictionary:
	return asset_index.play_audio_cue(player, category, token, options)

func validate_audio_playback_plan(plan: Dictionary) -> Dictionary:
	return asset_index.validate_audio_playback_plan(plan)

func load_manifest(relative_path: String) -> Variant:
	return asset_index.load_manifest(relative_path)

func load_manifest_path(absolute_path: String) -> Variant:
	return asset_index.load_manifest_path(absolute_path)

func load_runtime_map_metadata() -> Dictionary:
	return asset_index.load_runtime_map_metadata()

func load_runtime_spawn_points() -> Dictionary:
	return asset_index.load_runtime_spawn_points()

func load_map_manifest() -> Dictionary:
	return asset_index.load_map_manifest()

func load_map_attributes() -> Dictionary:
	return asset_index.load_map_attributes()

func load_map_blocks() -> Dictionary:
	return asset_index.load_map_blocks()

func load_battle_animation_table() -> Array:
	return asset_index.load_battle_animation_table()

func load_battle_anim_bundle() -> Dictionary:
	return asset_index.load_battle_anim_bundle()

func load_menu_icons() -> Dictionary:
	return asset_index.load_menu_icons()

func load_sprite_anim_bundle() -> Dictionary:
	return asset_index.load_sprite_anim_bundle()

func load_sprite_palette_defaults() -> Dictionary:
	return asset_index.load_sprite_palette_defaults()

func load_content_pack_index() -> Dictionary:
	return asset_index.load_content_pack_index()

func has_asset(relative_path: String) -> bool:
	return asset_index.has_asset(relative_path)

func has_data(relative_path: String) -> bool:
	return asset_index.has_data(relative_path)

func has_gfx(relative_path: String) -> bool:
	return asset_index.has_gfx(relative_path)

func ensure_loaded() -> void:
	if loaded:
		return
	initialize()
	pokemon_data = load_array(BATTLE_DATA_FILES["pokemon_data"])
	move_data = load_dictionary(BATTLE_DATA_FILES["move_data"])
	item_data = load_array(BATTLE_DATA_FILES["item_data"])
	trainer_data = load_array(BATTLE_DATA_FILES["trainer_data"])
	battle_animation_table = load_battle_animation_table()
	battle_anim_bundle = load_battle_anim_bundle()
	pokemon_frontpic_anim = load_dictionary(BATTLE_DATA_FILES["pokemon_frontpic_anim"])
	content_pack_index = load_content_pack_index()
	summary = _build_summary()
	loaded = true

func refresh() -> void:
	loaded = false
	ensure_loaded()

func load_summary() -> Dictionary:
	ensure_loaded()
	return summary.duplicate(true)

func get_pokemon(identifier: Variant) -> Dictionary:
	ensure_loaded()
	return _lookup_array_entry(pokemon_data, identifier, ["id", "name"], ["int_id"])

func get_move(identifier: Variant) -> Dictionary:
	ensure_loaded()
	if typeof(identifier) == TYPE_INT or typeof(identifier) == TYPE_FLOAT:
		var move_index := int(identifier)
		var move_keys: Array = move_data.keys()
		if move_index >= 0 and move_index < move_keys.size():
			var indexed_entry: Variant = move_data.get(move_keys[move_index], {})
			if typeof(indexed_entry) == TYPE_DICTIONARY:
				return Dictionary(indexed_entry).duplicate(true)
		return {}
	var normalized := _normalize_lookup_key(identifier)
	if normalized.is_empty():
		return {}
	if move_data.has(normalized):
		var direct_entry: Variant = move_data.get(normalized, {})
		if typeof(direct_entry) == TYPE_DICTIONARY:
			return Dictionary(direct_entry).duplicate(true)
	for key in move_data.keys():
		var entry: Variant = move_data.get(key, {})
		if typeof(entry) != TYPE_DICTIONARY:
			continue
		var entry_dictionary: Dictionary = entry
		if _lookup_key_matches(key, normalized) or _lookup_key_matches(entry_dictionary.get("name", ""), normalized):
			return entry_dictionary.duplicate(true)
	return {}

func get_item(identifier: Variant) -> Dictionary:
	ensure_loaded()
	return _lookup_array_entry(item_data, identifier, ["id", "name"], ["int_id"])

func get_trainer(identifier: Variant) -> Dictionary:
	ensure_loaded()
	return _lookup_array_entry(trainer_data, identifier, ["trainer_id", "trainer_class", "name", "id"], ["int_id"])

func has_pokemon(identifier: Variant) -> bool:
	return not get_pokemon(identifier).is_empty()

func has_move(identifier: Variant) -> bool:
	return not get_move(identifier).is_empty()

func has_item(identifier: Variant) -> bool:
	return not get_item(identifier).is_empty()

func has_trainer(identifier: Variant) -> bool:
	return not get_trainer(identifier).is_empty()

func hydrate_actor_payload(payload: Dictionary) -> Dictionary:
	ensure_loaded()
	var hydrated := payload.duplicate(true)
	var species_payload := _resolve_species_payload(payload)
	var trainer_payload := _resolve_trainer_payload(payload)
	if not trainer_payload.is_empty():
		hydrated["trainer"] = trainer_payload
	if species_payload.is_empty() and not trainer_payload.is_empty():
		species_payload = _first_trainer_party_species(trainer_payload)
	if not species_payload.is_empty():
		hydrated["species"] = species_payload
	return hydrated

func hydrate_turn_command(command: Dictionary, player_payload: Dictionary = {}, opponent_payload: Dictionary = {}) -> Dictionary:
	ensure_loaded()
	var hydrated := command.duplicate(true)
	var move_payload := _resolve_move_payload(command)
	if not move_payload.is_empty():
		hydrated["move_payload"] = move_payload
		hydrated["animation"] = get_move_animation(move_payload)
	var actor_payload := _resolve_actor_payload(command, player_payload, ["actor", "player", "user"])
	var target_payload := _resolve_actor_payload(command, opponent_payload, ["target", "opponent", "enemy"])
	if not actor_payload.is_empty():
		hydrated["actor"] = hydrate_actor_payload(actor_payload)
	if not target_payload.is_empty():
		hydrated["target"] = hydrate_actor_payload(target_payload)
	return hydrated

func get_move_animation(identifier: Variant) -> Dictionary:
	ensure_loaded()
	var resolved := _resolve_animation_table_entry(identifier)
	if resolved.is_empty():
		return {}
	return {
		"move_id": str(resolved.get("move_id", "")),
		"move_index": int(resolved.get("move_index", -1)),
		"table_index": int(resolved.get("table_index", -1)),
		"animation_id": str(resolved.get("animation_id", "")),
		"animation_label": str(resolved.get("animation_id", "")),
		"bundle": summarize_battle_anim_bundle(),
	}

func get_battle_animation_table_entry(identifier: Variant) -> Dictionary:
	ensure_loaded()
	return _resolve_animation_table_entry(identifier)

func get_battle_anim_bundle_section(section: String) -> Dictionary:
	ensure_loaded()
	var normalized := section.strip_edges()
	var value: Variant = battle_anim_bundle.get(normalized, {})
	if typeof(value) == TYPE_DICTIONARY:
		return Dictionary(value).duplicate(true)
	return {}

func get_battle_anim_object(identifier: Variant) -> Dictionary:
	return _lookup_bundle_dictionary_entry("objects", identifier)

func get_battle_anim_frameset(identifier: Variant) -> Array:
	return _lookup_bundle_array_entry("framesets", identifier)

func get_battle_anim_oam_set(identifier: Variant) -> Dictionary:
	return _lookup_bundle_dictionary_entry("oam_sets", identifier)

func get_battle_anim_gfx_entry(identifier: Variant) -> Array:
	return _lookup_bundle_array_entry("gfx_table", identifier)

func get_battle_anim_gfx_source(identifier: Variant) -> Dictionary:
	ensure_loaded()
	var normalized := _normalize_lookup_key(identifier)
	if normalized.is_empty():
		return {}
	var sources: Dictionary = Dictionary(battle_anim_bundle.get("gfx_sources", {}))
	for key in sources.keys():
		if _lookup_key_matches(key, normalized):
			return {
				"source_id": str(key),
				"path": str(sources.get(key, "")),
			}
	return {}

func get_frontpic_animation(identifier: Variant) -> Dictionary:
	ensure_loaded()
	var normalized := _normalize_frontpic_key(identifier)
	if normalized.is_empty():
		return {}
	var direct: Variant = pokemon_frontpic_anim.get(normalized, {})
	if typeof(direct) == TYPE_DICTIONARY:
		var entry: Dictionary = Dictionary(direct).duplicate(true)
		entry["species_id"] = normalized.to_upper()
		entry["frontpic_key"] = normalized
		entry["command_count"] = _count_entries(entry.get("commands", []))
		entry["frame_count"] = _count_frontpic_frames(entry)
		entry["total_duration"] = _sum_frontpic_frame_duration(entry)
		return entry
	return {}

func get_frontpic_animation_for_payload(payload: Dictionary) -> Dictionary:
	var species := _resolve_species_payload(payload)
	if not species.is_empty():
		var species_id := _first_identifier(species, ["id", "name", "species_id"])
		if not species_id.is_empty():
			return get_frontpic_animation(species_id)
	return get_frontpic_animation(_first_identifier(payload, ["species_id", "pokemon_id", "id", "name"]))

func summarize_battle_anim_bundle() -> Dictionary:
	ensure_loaded()
	return {
		"object_count": _count_entries(battle_anim_bundle.get("objects", {})),
		"frameset_count": _count_entries(battle_anim_bundle.get("framesets", {})),
		"oam_set_count": _count_entries(battle_anim_bundle.get("oam_sets", {})),
		"gfx_entry_count": _count_entries(battle_anim_bundle.get("gfx_table", {})),
		"gfx_source_count": _count_entries(battle_anim_bundle.get("gfx_sources", {})),
	}

func _build_summary() -> Dictionary:
	return {
		"pokemon_count": _count_entries(pokemon_data),
		"move_count": _count_entries(move_data),
		"item_count": _count_entries(item_data),
		"trainer_count": _count_entries(trainer_data),
		"battle_animation_count": _count_entries(battle_animation_table),
		"battle_anim_bundle_count": _count_battle_anim_bundle_entries(battle_anim_bundle),
		"frontpic_animation_count": _count_entries(pokemon_frontpic_anim),
		"content_pack_version": int(Dictionary(content_pack_index).get("version", 1)),
		"content_pack_count": _count_content_packs(content_pack_index),
	}

func _count_entries(value: Variant) -> int:
	match typeof(value):
		TYPE_ARRAY:
			return Array(value).size()
		TYPE_DICTIONARY:
			return Dictionary(value).size()
		_:
			return 0

func _count_battle_anim_bundle_entries(bundle: Dictionary) -> int:
	if bundle.is_empty():
		return 0
	var total := 0
	total += _count_entries(bundle.get("objects", {}))
	total += _count_entries(bundle.get("framesets", {}))
	total += _count_entries(bundle.get("oam_sets", {}))
	total += _count_entries(bundle.get("gfx_table", {}))
	total += _count_entries(bundle.get("gfx_sources", {}))
	return total

func _count_content_packs(index: Dictionary) -> int:
	var packs: Variant = index.get("packs", [])
	if typeof(packs) != TYPE_ARRAY:
		return 0
	var enabled_count := 0
	for pack in Array(packs):
		if typeof(pack) == TYPE_DICTIONARY and bool(Dictionary(pack).get("enabled", true)):
			enabled_count += 1
	return enabled_count

func _lookup_array_entry(entries: Array, identifier: Variant, text_fields: Array[String], int_fields: Array[String]) -> Dictionary:
	if typeof(identifier) == TYPE_INT or typeof(identifier) == TYPE_FLOAT:
		var numeric_id := int(identifier)
		for entry in entries:
			if typeof(entry) != TYPE_DICTIONARY:
				continue
			var entry_dictionary: Dictionary = entry
			for field in int_fields:
				if entry_dictionary.has(field) and int(entry_dictionary.get(field, -1)) == numeric_id:
					return entry_dictionary.duplicate(true)
		if numeric_id >= 0 and numeric_id < entries.size() and typeof(entries[numeric_id]) == TYPE_DICTIONARY:
			return Dictionary(entries[numeric_id]).duplicate(true)
		return {}
	var normalized := _normalize_lookup_key(identifier)
	if normalized.is_empty():
		return {}
	for entry in entries:
		if typeof(entry) != TYPE_DICTIONARY:
			continue
		var entry_dictionary: Dictionary = entry
		for field in text_fields:
			if entry_dictionary.has(field) and _lookup_key_matches(entry_dictionary.get(field, ""), normalized):
				return entry_dictionary.duplicate(true)
	return {}

func _lookup_key_matches(value: Variant, normalized_key: String) -> bool:
	return _normalize_lookup_key(value) == normalized_key

func _normalize_lookup_key(value: Variant) -> String:
	if typeof(value) == TYPE_NIL:
		return ""
	var normalized := str(value).strip_edges().trim_suffix("@").to_upper()
	normalized = normalized.replace("-", "_")
	normalized = normalized.replace(" ", "_")
	while normalized.find("__") != -1:
		normalized = normalized.replace("__", "_")
	return normalized

func _resolve_move_payload(command: Dictionary) -> Dictionary:
	for key in ["move_payload", "move"]:
		var value: Variant = command.get(key, {})
		if typeof(value) == TYPE_DICTIONARY:
			var existing: Dictionary = value
			if existing.has("power") or existing.has("pp") or existing.has("type"):
				return existing.duplicate(true)
			var nested_identifier := _first_identifier(existing, ["id", "move_id", "name", "label", "kind"])
			if not nested_identifier.is_empty():
				var nested_move := get_move(nested_identifier)
				if not nested_move.is_empty():
					return nested_move
		elif typeof(value) != TYPE_NIL:
			var move := get_move(value)
			if not move.is_empty():
				return move
	var identifier := _first_identifier(command, ["move_id", "move_name", "id", "label"])
	return get_move(identifier) if not identifier.is_empty() else {}

func _resolve_actor_payload(command: Dictionary, default_payload: Dictionary, keys: Array[String]) -> Dictionary:
	for key in keys:
		var value: Variant = command.get(key, {})
		if typeof(value) == TYPE_DICTIONARY:
			return Dictionary(value).duplicate(true)
	return default_payload.duplicate(true)

func _resolve_species_payload(payload: Dictionary) -> Dictionary:
	var species: Variant = payload.get("species", payload.get("species_id", payload.get("pokemon", payload.get("pokemon_id", payload.get("id", "")))))
	if typeof(species) == TYPE_DICTIONARY:
		var species_dictionary: Dictionary = species
		if species_dictionary.has("base_stats"):
			return species_dictionary.duplicate(true)
		var nested_identifier := _first_identifier(species_dictionary, ["id", "name", "species_id"])
		return get_pokemon(nested_identifier) if not nested_identifier.is_empty() else {}
	if typeof(species) != TYPE_NIL and not str(species).strip_edges().is_empty():
		return get_pokemon(species)
	return {}

func _resolve_trainer_payload(payload: Dictionary) -> Dictionary:
	var trainer: Variant = payload.get("trainer", payload.get("trainer_id", payload.get("trainer_class", "")))
	if typeof(trainer) == TYPE_DICTIONARY:
		var trainer_dictionary: Dictionary = trainer
		if trainer_dictionary.has("party") or trainer_dictionary.has("trainer_id"):
			return trainer_dictionary.duplicate(true)
		var nested_identifier := _first_identifier(trainer_dictionary, ["trainer_id", "trainer_class", "id", "name"])
		return get_trainer(nested_identifier) if not nested_identifier.is_empty() else {}
	if typeof(trainer) != TYPE_NIL and not str(trainer).strip_edges().is_empty():
		return get_trainer(trainer)
	return {}

func _first_identifier(payload: Dictionary, keys: Array[String]) -> String:
	for key in keys:
		if payload.has(key):
			var value := str(payload.get(key, "")).strip_edges()
			if not value.is_empty():
				return value
	return ""

func _first_trainer_party_species(trainer_payload: Dictionary) -> Dictionary:
	var party: Variant = trainer_payload.get("party", [])
	if typeof(party) != TYPE_ARRAY:
		return {}
	for member in Array(party):
		if typeof(member) != TYPE_DICTIONARY:
			continue
		var member_dictionary: Dictionary = member
		var species: Variant = member_dictionary.get("species", {})
		if typeof(species) == TYPE_DICTIONARY:
			return Dictionary(species).duplicate(true)
		if typeof(species) != TYPE_NIL and not str(species).strip_edges().is_empty():
			var species_payload := get_pokemon(species)
			if not species_payload.is_empty():
				return species_payload
	return {}

func _resolve_animation_table_entry(identifier: Variant) -> Dictionary:
	var table_index := _animation_table_index(identifier)
	if table_index < 0 or table_index >= battle_animation_table.size():
		return {}
	var animation_id := str(battle_animation_table[table_index])
	if animation_id.is_empty():
		return {}
	return {
		"animation_id": animation_id,
		"table_index": table_index,
		"move_index": table_index - 1,
		"move_id": _move_id_for_table_index(table_index),
	}

func _animation_table_index(identifier: Variant) -> int:
	if typeof(identifier) == TYPE_INT or typeof(identifier) == TYPE_FLOAT:
		return int(identifier)
	if typeof(identifier) == TYPE_DICTIONARY:
		var source: Dictionary = identifier
		for key in ["animation_id", "animation_label"]:
			if source.has(key):
				var label_index := _animation_table_label_index(str(source.get(key, "")))
				if label_index >= 0:
					return label_index
		var identifier_value := _first_identifier(source, ["id", "move_id", "name", "label", "kind"])
		return _animation_table_index(identifier_value)
	var normalized := _normalize_lookup_key(identifier)
	if normalized.is_empty():
		return -1
	var direct_label_index := _animation_table_label_index(str(identifier))
	if direct_label_index >= 0:
		return direct_label_index
	var move_keys: Array = move_data.keys()
	for index in range(move_keys.size()):
		if _lookup_key_matches(move_keys[index], normalized):
			return index + 1
		var move_entry: Variant = move_data.get(move_keys[index], {})
		if typeof(move_entry) == TYPE_DICTIONARY and _lookup_key_matches(Dictionary(move_entry).get("name", ""), normalized):
			return index + 1
	return -1

func _animation_table_label_index(label: String) -> int:
	var normalized := _normalize_lookup_key(label)
	if normalized.is_empty():
		return -1
	for index in range(battle_animation_table.size()):
		if _lookup_key_matches(battle_animation_table[index], normalized):
			return index
	return -1

func _move_id_for_table_index(table_index: int) -> String:
	var move_index := table_index - 1
	var move_keys: Array = move_data.keys()
	if move_index >= 0 and move_index < move_keys.size():
		return str(move_keys[move_index])
	return ""

func _lookup_bundle_dictionary_entry(section: String, identifier: Variant) -> Dictionary:
	ensure_loaded()
	var entries: Dictionary = Dictionary(battle_anim_bundle.get(section, {}))
	var normalized := _normalize_lookup_key(identifier)
	if normalized.is_empty():
		return {}
	for key in entries.keys():
		if _lookup_key_matches(key, normalized):
			var entry: Variant = entries.get(key, {})
			if typeof(entry) == TYPE_DICTIONARY:
				return Dictionary(entry).duplicate(true)
	return {}

func _lookup_bundle_array_entry(section: String, identifier: Variant) -> Array:
	ensure_loaded()
	var entries: Dictionary = Dictionary(battle_anim_bundle.get(section, {}))
	var normalized := _normalize_lookup_key(identifier)
	if normalized.is_empty():
		return []
	for key in entries.keys():
		if _lookup_key_matches(key, normalized):
			var entry: Variant = entries.get(key, [])
			if typeof(entry) == TYPE_ARRAY:
				return Array(entry).duplicate(true)
	return []

func _normalize_frontpic_key(identifier: Variant) -> String:
	if typeof(identifier) == TYPE_DICTIONARY:
		var source: Dictionary = identifier
		var species := _resolve_species_payload(source)
		if not species.is_empty():
			return _normalize_frontpic_key(_first_identifier(species, ["id", "name", "species_id"]))
		return _normalize_frontpic_key(_first_identifier(source, ["species_id", "pokemon_id", "id", "name"]))
	return str(identifier).strip_edges().trim_suffix("@").to_lower().replace("_", "-").replace(" ", "-")

func _count_frontpic_frames(entry: Dictionary) -> int:
	var count := 0
	var commands: Variant = entry.get("commands", [])
	if typeof(commands) != TYPE_ARRAY:
		return 0
	for command in Array(commands):
		if typeof(command) == TYPE_DICTIONARY and str(Dictionary(command).get("kind", "")) == "frame":
			count += 1
	return count

func _sum_frontpic_frame_duration(entry: Dictionary) -> int:
	var total := 0
	var commands: Variant = entry.get("commands", [])
	if typeof(commands) != TYPE_ARRAY:
		return 0
	for command in Array(commands):
		if typeof(command) == TYPE_DICTIONARY and str(Dictionary(command).get("kind", "")) == "frame":
			total += int(Dictionary(command).get("duration", 0))
	return total
