extends RefCounted
class_name AssetIndex

const REPO_PATHS_SCRIPT = preload("res://scripts/repo_paths.gd")
const GB_TILE_DECODER_SCRIPT = preload("res://scripts/gb_tile_decoder.gd")
const AUDIO_ASSETS_SCRIPT = preload("res://scripts/audio_assets.gd")
const EXPORTED_ASSET_MANIFEST_SCRIPT = preload("res://scripts/exported_asset_manifest.gd")

var repo_root := ""
var assets_root := ""
var data_root := ""
var gfx_root := ""
var _initialized := false
var _manifest_cache: Dictionary = {}
var _text_cache: Dictionary = {}
var _bytes_cache: Dictionary = {}
var _audio_assets: RefCounted = null
var _exported_asset_manifest: RefCounted = null

const MAP_METADATA_MANIFEST := "runtime_map_metadata.json"
const RUNTIME_SPAWN_POINTS_MANIFEST := "runtime_spawn_points.json"
const MAP_ATTRIBUTES_MANIFEST := "map_attributes.json"
const MAP_BLOCKS_MANIFEST := "map_blocks.json"
const COLLISION_PERMISSIONS_MANIFEST := "collision/collision_permissions.json"
const COLLISION_STDSCRIPTS_MANIFEST := "collision/collision_stdscripts.json"
const BATTLE_ANIMATION_TABLE_MANIFEST := "battle_animation_table.json"
const BATTLE_ANIM_BUNDLE_MANIFEST := "battle_anim_bundle.json"
const MENU_ICONS_MANIFEST := "menu_icons.json"
const SPRITE_ANIM_BUNDLE_MANIFEST := "sprite_anim_bundle.json"
const SPRITE_PALETTE_DEFAULTS_MANIFEST := "sprite_palette_defaults.json"
const TITLE_PALETTE_PATH := "title.pal"
const TITLE_LOGO_PATH := "logo.2bpp"
const TITLE_SUICUNE_PATH := "suicune.2bpp"
const TITLE_CRYSTAL_PATH := "crystal.2bpp"
const TITLE_LOGO_TILE_COUNT := 20 * 8
const TITLE_SUICUNE_TILE_COUNT := 16 * 16
const TITLE_CRYSTAL_TILE_COUNT := 6 * 10
const TILESET_GFX_DIR := "tilesets"
const TILESET_DATA_DIR := "tilesets"
const TILESET_GLOBAL_PALETTE_PATH := "tilesets/bg_tiles.pal"
const TILESET_PALETTE_PERIOD_OFFSETS := {
	"morn": 0,
	"morning": 0,
	"day": 8,
	"nite": 16,
	"night": 16,
	"dark": 24,
}

func initialize() -> void:
	if _initialized:
		return
	repo_root = REPO_PATHS_SCRIPT.repo_root()
	assets_root = REPO_PATHS_SCRIPT.web_assets_root()
	data_root = REPO_PATHS_SCRIPT.data_root()
	gfx_root = REPO_PATHS_SCRIPT.gfx_root()
	_initialized = true

func assets_path(relative_path: String) -> String:
	_ensure_initialized()
	return _resolve_path(assets_root, relative_path)

func data_path(relative_path: String) -> String:
	_ensure_initialized()
	return _resolve_path(data_root, relative_path)

func gfx_path(relative_path: String) -> String:
	_ensure_initialized()
	return _resolve_path(gfx_root, relative_path)

func load_json(relative_path: String) -> Variant:
	return load_json_path(data_path(relative_path))

func load_json_path(absolute_path: String) -> Variant:
	return _load_json_path_cached(absolute_path)

func load_dictionary(relative_path: String) -> Dictionary:
	return load_dictionary_path(data_path(relative_path))

func load_dictionary_path(absolute_path: String) -> Dictionary:
	var value: Variant = load_json_path(absolute_path)
	if typeof(value) != TYPE_DICTIONARY:
		return {}
	return Dictionary(value)

func load_array(relative_path: String) -> Array:
	return load_array_path(data_path(relative_path))

func load_array_path(absolute_path: String) -> Array:
	var value: Variant = load_json_path(absolute_path)
	if typeof(value) != TYPE_ARRAY:
		return []
	return Array(value)

func load_text(relative_path: String) -> String:
	return load_text_path(data_path(relative_path))

func load_text_path(absolute_path: String) -> String:
	if absolute_path.is_empty():
		return ""
	if _text_cache.has(absolute_path):
		return str(_text_cache[absolute_path])
	if not FileAccess.file_exists(absolute_path):
		return ""
	var file := FileAccess.open(absolute_path, FileAccess.READ)
	if file == null:
		return ""
	var text := file.get_as_text()
	file = null
	_text_cache[absolute_path] = text
	return text

func load_raw_bytes(relative_path: String) -> PackedByteArray:
	return load_raw_bytes_path(assets_path(relative_path))

func load_raw_bytes_path(absolute_path: String) -> PackedByteArray:
	if absolute_path.is_empty():
		return PackedByteArray()
	if _bytes_cache.has(absolute_path):
		return _bytes_cache[absolute_path]
	if not FileAccess.file_exists(absolute_path):
		return PackedByteArray()
	var file := FileAccess.open(absolute_path, FileAccess.READ)
	if file == null:
		return PackedByteArray()
	var bytes := file.get_buffer(file.get_length())
	file = null
	_bytes_cache[absolute_path] = bytes
	return bytes

func load_image(relative_path: String) -> Image:
	return load_image_path(assets_path(relative_path))

func load_image_path(absolute_path: String) -> Image:
	var image := Image.new()
	if absolute_path.is_empty():
		return image
	if not FileAccess.file_exists(absolute_path):
		return image
	var error := image.load(absolute_path)
	if error != OK:
		return Image.new()
	return image

func load_map_manifest() -> Dictionary:
	return load_runtime_map_metadata()

func load_runtime_map_metadata() -> Dictionary:
	return _load_dictionary_manifest(MAP_METADATA_MANIFEST)

func load_runtime_spawn_points() -> Dictionary:
	return _load_dictionary_manifest(RUNTIME_SPAWN_POINTS_MANIFEST)

func load_map_blocks() -> Dictionary:
	return _load_dictionary_manifest(MAP_BLOCKS_MANIFEST)

func load_map_attributes() -> Dictionary:
	return _load_dictionary_manifest(MAP_ATTRIBUTES_MANIFEST)

func load_map_attributes_for_map(map_name: String) -> Dictionary:
	var safe_map_name := _strict_asset_key(map_name)
	if safe_map_name.is_empty():
		push_error("Map name is required.")
		return {}
	var attributes := load_map_attributes()
	if not attributes.has(safe_map_name):
		push_error("Missing map attributes for %s." % safe_map_name)
		return {}
	var value: Variant = attributes[safe_map_name]
	if typeof(value) != TYPE_DICTIONARY:
		push_error("Map attributes for %s must be a dictionary." % safe_map_name)
		return {}
	return Dictionary(value)

func load_map_file(map_name: String) -> Dictionary:
	if map_name.is_empty():
		return {}
	return load_dictionary("maps/%s.json" % map_name)

func load_map_file_path(absolute_path: String) -> Dictionary:
	return load_dictionary_path(absolute_path)

func load_collision_permissions() -> Array:
	return _load_array_manifest(COLLISION_PERMISSIONS_MANIFEST)

func load_collision_stdscripts() -> Dictionary:
	return _load_dictionary_manifest(COLLISION_STDSCRIPTS_MANIFEST)

func load_tileset_collision(tileset_name: String) -> Dictionary:
	var safe_tileset := _strict_asset_key(tileset_name)
	if safe_tileset.is_empty():
		return {}
	return load_dictionary("%s/%s.json" % [TILESET_DATA_DIR, safe_tileset])

func load_tileset_palette_map(tileset_name: String) -> Variant:
	var safe_tileset := _strict_asset_key(tileset_name)
	if safe_tileset.is_empty():
		return []
	return load_json("%s/%s_palette_map.json" % [TILESET_DATA_DIR, safe_tileset])

func load_tileset_metatiles(tileset_name: String) -> PackedByteArray:
	return load_tileset_metatile_bytes(tileset_name)

func load_tileset_metadata(tileset_name: String) -> Dictionary:
	var safe_tileset := _strict_asset_key(tileset_name)
	var collision: Dictionary = load_tileset_collision(safe_tileset)
	var palette_map: Variant = load_tileset_palette_map(safe_tileset)
	var metatiles: PackedByteArray = load_tileset_metatiles(safe_tileset)
	return {
		"tileset_name": safe_tileset,
		"collision_count": collision.size(),
		"palette_count": _count_entries(palette_map),
		"metatile_bytes": metatiles.size(),
		"metatile_count": int(metatiles.size() / 16) if metatiles.size() > 0 and metatiles.size() % 16 == 0 else 0,
		"collision_path": data_path("%s/%s.json" % [TILESET_DATA_DIR, safe_tileset]),
		"palette_map_path": data_path("%s/%s_palette_map.json" % [TILESET_DATA_DIR, safe_tileset]),
		"metatiles_path": data_path("%s/%s_metatiles.bin" % [TILESET_DATA_DIR, safe_tileset]),
	}

func load_battle_animation_table() -> Array:
	return _load_array_manifest(BATTLE_ANIMATION_TABLE_MANIFEST)

func load_battle_anim_bundle() -> Dictionary:
	return _load_dictionary_manifest(BATTLE_ANIM_BUNDLE_MANIFEST)

func load_menu_icons() -> Dictionary:
	return _load_dictionary_manifest(MENU_ICONS_MANIFEST)

func load_sprite_anim_bundle() -> Dictionary:
	return _load_dictionary_manifest(SPRITE_ANIM_BUNDLE_MANIFEST)

func load_sprite_palette_defaults() -> Dictionary:
	return _load_dictionary_manifest(SPRITE_PALETTE_DEFAULTS_MANIFEST)

func load_pokemon_cries() -> Dictionary:
	return _get_audio_assets().load_pokemon_cries()

func audio_root_candidates() -> PackedStringArray:
	return _get_audio_assets().audio_root_candidates()

func audio_manifest_paths() -> PackedStringArray:
	return _get_audio_assets().audio_manifest_paths()

func load_audio_manifests() -> Array[Dictionary]:
	return _get_audio_assets().load_audio_manifests()

func load_audio_manifest(relative_path: String) -> Dictionary:
	return _get_audio_assets().load_audio_manifest(relative_path)

func load_audio_manifest_path(absolute_path: String) -> Dictionary:
	return _get_audio_assets().load_audio_manifest_path(absolute_path)

func load_disassembly_aliases() -> Dictionary:
	return _get_audio_assets().load_disassembly_aliases()

func resolve_music_cue(token: String) -> Dictionary:
	return _get_audio_assets().resolve_music_cue(token)

func resolve_sfx_cue(token: String) -> Dictionary:
	return _get_audio_assets().resolve_sfx_cue(token)

func resolve_cry_cue(species_or_cry: String) -> Dictionary:
	return _get_audio_assets().resolve_cry_cue(species_or_cry)

func resolve_audio_cue(category: String, token: String) -> Dictionary:
	return _get_audio_assets().resolve_audio_cue(category, token)

func resolve_audio_manifest_entry(category: String, token: String) -> Dictionary:
	return _get_audio_assets().resolve_audio_manifest_entry(category, token)

func cue_metadata_for_manifest_entry(category: String, key: String, value: Variant, manifest_path: String = "") -> Dictionary:
	return _get_audio_assets().cue_metadata_for_manifest_entry(category, key, value, manifest_path)

func load_audio_cue_bytes(cue: Dictionary) -> PackedByteArray:
	return _get_audio_assets().load_audio_cue_bytes(cue)

func load_audio_bytes_path(absolute_path: String) -> PackedByteArray:
	return _get_audio_assets().load_audio_bytes_path(absolute_path)

func load_audio_stream(cue: Dictionary) -> AudioStream:
	return _get_audio_assets().load_audio_stream(cue)

func build_audio_playback_plan(category: String, token: String, options: Dictionary = {}) -> Dictionary:
	return _get_audio_assets().build_audio_playback_plan(category, token, options)

func build_audio_playback_plan_for_cue(cue: Dictionary, options: Dictionary = {}) -> Dictionary:
	return _get_audio_assets().build_audio_playback_plan_for_cue(cue, options)

func create_audio_playback_state() -> Dictionary:
	return _get_audio_assets().create_audio_playback_state()

func schedule_audio_playback_plan(state: Dictionary, plan: Dictionary) -> Dictionary:
	return _get_audio_assets().schedule_audio_playback_plan(state, plan)

func release_audio_playback_plan(state: Dictionary, plan_or_token: Variant) -> Dictionary:
	return _get_audio_assets().release_audio_playback_plan(state, plan_or_token)

func build_audio_playback_snapshot(state: Dictionary) -> Dictionary:
	return _get_audio_assets().build_audio_playback_snapshot(state)

func apply_audio_playback_plan(player: Object, plan: Dictionary) -> Dictionary:
	return _get_audio_assets().apply_audio_playback_plan(player, plan)

func play_audio_cue(player: Object, category: String, token: String, options: Dictionary = {}) -> Dictionary:
	return _get_audio_assets().play_audio_cue(player, category, token, options)

func validate_audio_playback_plan(plan: Dictionary) -> Dictionary:
	return _get_audio_assets().validate_audio_playback_plan(plan)

func validate_audio_cue(cue: Dictionary) -> Dictionary:
	return _get_audio_assets().validate_audio_cue(cue)

func validate_canonical_audio_assets() -> Dictionary:
	return _get_audio_assets().validate_canonical_audio_assets()

func build_exported_asset_manifest() -> Dictionary:
	return _get_exported_asset_manifest().build_asset_manifest()

func validate_core_exported_assets() -> Dictionary:
	return _get_exported_asset_manifest().validate_core_exported_assets()

func list_exported_data_json_assets() -> Array[Dictionary]:
	return _get_exported_asset_manifest().list_data_json_assets()

func list_exported_png_assets() -> Array[Dictionary]:
	return _get_exported_asset_manifest().list_png_assets()

func list_exported_mp3_assets() -> Array[Dictionary]:
	return _get_exported_asset_manifest().list_mp3_assets()

func list_gb_native_graphics() -> Array[Dictionary]:
	return _get_exported_asset_manifest().list_gb_native_graphics()

func validate_data_json(relative_path: String) -> Dictionary:
	return _get_exported_asset_manifest().validate_data_json(relative_path)

func validate_gfx_png(relative_path: String) -> Dictionary:
	return _get_exported_asset_manifest().validate_gfx_png(relative_path)

func validate_mp3(relative_path: String) -> Dictionary:
	return _get_exported_asset_manifest().validate_mp3(relative_path)

func resolve_gb_native_graphic(relative_path: String) -> Dictionary:
	return _get_exported_asset_manifest().resolve_gb_native_graphic(relative_path)

func validate_gb_native_graphic(relative_path: String) -> Dictionary:
	return _get_exported_asset_manifest().validate_gb_native_graphic(relative_path)

func decode_gb_native_tiles(relative_path: String, palette: Array[Color] = []) -> Array[Image]:
	return _get_exported_asset_manifest().decode_gb_native_tiles(relative_path, palette)

func load_gb_native_atlas(relative_path: String, columns: int, palette: Array[Color] = []) -> Image:
	return _get_exported_asset_manifest().load_gb_native_atlas(relative_path, columns, palette)

func load_map_block_bytes(blocks_label: String) -> PackedByteArray:
	var safe_label := _strict_asset_key(blocks_label)
	if safe_label.is_empty():
		push_error("Map blocks label is required.")
		return PackedByteArray()
	var blocks := load_map_blocks()
	if not blocks.has(safe_label):
		push_error("Missing map block entry %s." % safe_label)
		return PackedByteArray()
	var encoded := str(blocks[safe_label])
	if encoded.is_empty():
		push_error("Map block entry %s is empty." % safe_label)
		return PackedByteArray()
	var decoded: PackedByteArray = Marshalls.base64_to_raw(encoded)
	if decoded.is_empty():
		push_error("Map block entry %s did not decode to bytes." % safe_label)
	return decoded

func load_map_block_bytes_for_map(map_name: String) -> PackedByteArray:
	var attributes := load_map_attributes_for_map(map_name)
	if attributes.is_empty():
		return PackedByteArray()
	var blocks_label := str(attributes.get("blocks_label", ""))
	if blocks_label.is_empty():
		push_error("Map %s is missing blocks_label." % map_name)
		return PackedByteArray()
	return load_map_block_bytes(blocks_label)

func load_tileset_image(tileset_name: String) -> Image:
	var safe_tileset := _strict_asset_key(tileset_name)
	if safe_tileset.is_empty():
		push_error("Tileset name is required.")
		return Image.new()
	var absolute_path := gfx_path("%s/%s.png" % [TILESET_GFX_DIR, safe_tileset])
	if absolute_path.is_empty() or not FileAccess.file_exists(absolute_path):
		push_error("Missing tileset image for %s at %s." % [safe_tileset, absolute_path])
		return Image.new()
	var image := load_image_path(absolute_path)
	if image.is_empty():
		push_error("Failed to load tileset image for %s at %s." % [safe_tileset, absolute_path])
	return image

func load_tileset_tiles(tileset_name: String) -> Array[Image]:
	var image := load_tileset_image(tileset_name)
	var tiles: Array[Image] = GB_TILE_DECODER_SCRIPT.slice_image_tiles(image)
	if tiles.is_empty():
		push_error("Tileset %s did not produce 8x8 tiles." % tileset_name)
	return tiles

func load_tileset_metatile_bytes(tileset_name: String) -> PackedByteArray:
	var safe_tileset := _strict_asset_key(tileset_name)
	if safe_tileset.is_empty():
		push_error("Tileset name is required.")
		return PackedByteArray()
	var absolute_path := data_path("%s/%s_metatiles.bin" % [TILESET_DATA_DIR, safe_tileset])
	if absolute_path.is_empty() or not FileAccess.file_exists(absolute_path):
		push_error("Missing metatile bytes for tileset %s at %s." % [safe_tileset, absolute_path])
		return PackedByteArray()
	var bytes := load_raw_bytes_path(absolute_path)
	if bytes.is_empty():
		push_error("Metatile bytes for tileset %s are empty." % safe_tileset)
	return bytes

func load_tileset_metatile_ids(tileset_name: String) -> Array[PackedInt32Array]:
	var bytes := load_tileset_metatile_bytes(tileset_name)
	var metatile_ids: Array[PackedInt32Array] = GB_TILE_DECODER_SCRIPT.decode_metatile_ids(bytes)
	if metatile_ids.is_empty():
		push_error("Tileset %s did not produce metatile ids." % tileset_name)
	return metatile_ids

func load_tileset_metatile_images(tileset_name: String) -> Array[Image]:
	var tiles := load_tileset_tiles(tileset_name)
	var metatile_ids := load_tileset_metatile_ids(tileset_name)
	if tiles.is_empty() or metatile_ids.is_empty():
		return []
	return GB_TILE_DECODER_SCRIPT.assemble_metatiles(tiles, metatile_ids)

func load_tileset_palette_values(tileset_name: String) -> PackedInt32Array:
	var safe_tileset := _strict_asset_key(tileset_name)
	if safe_tileset.is_empty():
		push_error("Tileset name is required.")
		return PackedInt32Array()
	var value: Variant = load_json("%s/%s_palette_map.json" % [TILESET_DATA_DIR, safe_tileset])
	if value == null:
		push_error("Missing palette map for tileset %s." % safe_tileset)
		return PackedInt32Array()
	return _parse_palette_map(value, safe_tileset)

func load_tileset_palette_bank(tileset_name: String) -> Array:
	var palette_path := resolve_tileset_palette_path(tileset_name)
	if palette_path.is_empty():
		return []
	var bank: Array = GB_TILE_DECODER_SCRIPT.load_palette_bank_path(palette_path)
	if bank.is_empty():
		push_error("Failed to load palette bank at %s." % palette_path)
	return bank

func load_tileset_palette_set(tileset_name: String, time_of_day: String = "day") -> Array:
	return select_tileset_palette_set(load_tileset_palette_bank(tileset_name), time_of_day)

func select_tileset_palette_set(palette_bank: Array, time_of_day: String = "day") -> Array:
	if palette_bank.is_empty():
		return []
	if palette_bank.size() <= 8:
		return palette_bank.duplicate(true)
	var key := time_of_day.strip_edges().to_lower()
	var offset := int(TILESET_PALETTE_PERIOD_OFFSETS.get(key, TILESET_PALETTE_PERIOD_OFFSETS["day"]))
	if offset + 8 > palette_bank.size():
		push_error("Palette bank does not contain an 8-palette %s set." % time_of_day)
		return []
	var palettes: Array = []
	for index in range(offset, offset + 8):
		palettes.append(Array(palette_bank[index]).duplicate(true))
	return palettes

func resolve_tileset_palette_path(tileset_name: String) -> String:
	var safe_tileset := _strict_asset_key(tileset_name)
	if safe_tileset.is_empty():
		push_error("Tileset name is required.")
		return ""
	var specific_path := gfx_path("%s/%s.pal" % [TILESET_GFX_DIR, safe_tileset])
	if not specific_path.is_empty() and FileAccess.file_exists(specific_path):
		return specific_path
	var global_path := gfx_path(TILESET_GLOBAL_PALETTE_PATH)
	if not global_path.is_empty() and FileAccess.file_exists(global_path):
		return global_path
	push_error("Missing palette bank for tileset %s; checked %s and %s." % [safe_tileset, specific_path, global_path])
	return ""

func palette_index_for_tile(palette_map: PackedInt32Array, tile_id: int) -> int:
	if tile_id < 0 or tile_id >= palette_map.size():
		return -1
	return int(palette_map[tile_id]) & 0x07

func select_palette_from_bank(palette_bank: Array, palette_value: int) -> Array[Color]:
	var palette_index := palette_value & 0x07
	if palette_index < 0 or palette_index >= palette_bank.size():
		return []
	var source := Array(palette_bank[palette_index])
	var palette: Array[Color] = []
	for color in source:
		if typeof(color) == TYPE_COLOR:
			palette.append(color)
	return palette

func load_map_tile_surface(map_name: String, time_of_day: String = "day") -> Dictionary:
	var attributes := load_map_attributes_for_map(map_name)
	if attributes.is_empty():
		return {}
	var tileset_name := str(attributes.get("tileset_name", ""))
	var blocks_label := str(attributes.get("blocks_label", ""))
	var width := int(attributes.get("width", 0))
	var height := int(attributes.get("height", 0))
	if tileset_name.is_empty() or blocks_label.is_empty() or width <= 0 or height <= 0:
		push_error("Map %s is missing renderer-facing tileset, blocks, or dimensions." % map_name)
		return {}
	var block_ids := load_map_block_bytes(blocks_label)
	if block_ids.size() != width * height:
		push_error("Map %s block data size %d does not match dimensions %dx%d." % [map_name, block_ids.size(), width, height])
		return {}
	var tiles := load_tileset_tiles(tileset_name)
	var metatile_ids := load_tileset_metatile_ids(tileset_name)
	var metatiles: Array[Image] = GB_TILE_DECODER_SCRIPT.assemble_metatiles(tiles, metatile_ids)
	if metatiles.is_empty():
		push_error("Map %s could not assemble metatile images for tileset %s." % [map_name, tileset_name])
		return {}
	var map_image: Image = GB_TILE_DECODER_SCRIPT.assemble_map_blocks(metatiles, block_ids, width, height)
	if map_image.is_empty():
		push_error("Map %s could not assemble a map image." % map_name)
		return {}
	var palette_bank := load_tileset_palette_bank(tileset_name)
	var palette_set := select_tileset_palette_set(palette_bank, time_of_day)
	var palette_map := load_tileset_palette_values(tileset_name)
	return {
		"map_name": map_name,
		"tileset_name": tileset_name,
		"blocks_label": blocks_label,
		"width": width,
		"height": height,
		"block_ids": block_ids,
		"tiles": tiles,
		"metatile_ids": metatile_ids,
		"metatiles": metatiles,
		"image": map_image,
		"palette_bank": palette_bank,
		"palette_set": palette_set,
		"palette_map": palette_map,
	}

func load_palette_bank(relative_path: String) -> Array:
	return GB_TILE_DECODER_SCRIPT.load_palette_bank(gfx_path(relative_path))

func load_palette_bank_path(absolute_path: String) -> Array:
	return GB_TILE_DECODER_SCRIPT.load_palette_bank_path(absolute_path)

func load_title_palette_bank() -> Array:
	return load_palette_bank(TITLE_PALETTE_PATH)

func load_title_palette_bank_path(absolute_path: String) -> Array:
	return load_palette_bank_path(absolute_path)

func load_title_graphics() -> Dictionary:
	return load_title_graphics_path()

func load_title_graphics_path() -> Dictionary:
	var palette_bank := load_title_palette_bank()
	if palette_bank.size() < 16:
		push_error("title.pal must contain 16 palettes.")
		return {}
	var bg_palettes: Array = []
	var obj_palettes: Array = []
	for index in range(16):
		var palette := Array(palette_bank[index])
		if index < 8:
			bg_palettes.append(palette)
		else:
			obj_palettes.append(palette)
	var transparent_palette: Array[Color] = Array(GB_TILE_DECODER_SCRIPT.make_transparent_grayscale_palette())
	var logo_tiles: Array[Image] = GB_TILE_DECODER_SCRIPT.load_2bpp_tiles_padded_path(
		gfx_path(TITLE_LOGO_PATH),
		TITLE_LOGO_TILE_COUNT,
		transparent_palette
	)
	var suicune_tiles: Array[Image] = GB_TILE_DECODER_SCRIPT.load_2bpp_tiles_padded_path(
		gfx_path(TITLE_SUICUNE_PATH),
		TITLE_SUICUNE_TILE_COUNT,
		transparent_palette
	)
	var crystal_tiles: Array[Image] = GB_TILE_DECODER_SCRIPT.load_2bpp_tiles_padded_path(
		gfx_path(TITLE_CRYSTAL_PATH),
		TITLE_CRYSTAL_TILE_COUNT,
		transparent_palette
	)
	crystal_tiles = GB_TILE_DECODER_SCRIPT.reorder_sprite_pair_tiles(crystal_tiles, 6, 10)
	if logo_tiles.size() != TITLE_LOGO_TILE_COUNT or suicune_tiles.size() != TITLE_SUICUNE_TILE_COUNT or crystal_tiles.size() != TITLE_CRYSTAL_TILE_COUNT:
		push_error("Failed to load title graphics tile banks.")
		return {}
	return {
		"bg_palettes": bg_palettes,
		"obj_palettes": obj_palettes,
		"logo": logo_tiles,
		"suicune": suicune_tiles,
		"crystal": crystal_tiles,
	}

func load_content_pack_index() -> Dictionary:
	return load_dictionary("content-packs/index.json")

func load_manifest(relative_path: String) -> Variant:
	return load_json(relative_path)

func load_manifest_path(absolute_path: String) -> Variant:
	return load_json_path(absolute_path)

func _load_json_path_cached(absolute_path: String) -> Variant:
	if absolute_path.is_empty():
		return null
	if _manifest_cache.has(absolute_path):
		return _manifest_cache[absolute_path]
	if not FileAccess.file_exists(absolute_path):
		return null
	var file := FileAccess.open(absolute_path, FileAccess.READ)
	if file == null:
		return null
	var text := file.get_as_text()
	file = null
	var json := JSON.new()
	if json.parse(text) != OK:
		return null
	_manifest_cache[absolute_path] = json.data
	return json.data

func _load_dictionary_manifest(relative_path: String) -> Dictionary:
	return load_dictionary(relative_path)

func _load_array_manifest(relative_path: String) -> Array:
	return load_array(relative_path)

func load_summary() -> Dictionary:
	initialize()
	var content_pack_index := load_content_pack_index()
	var summary: Dictionary = {
		"repo_root": repo_root,
		"assets_root": assets_root,
		"data_root": data_root,
		"gfx_root": gfx_root,
		"pokemon_count": 0,
		"move_count": 0,
		"item_count": 0,
		"map_attribute_count": 0,
		"content_pack_version": 0,
		"content_pack_count": 0,
	}
	summary["pokemon_count"] = _count_entries(load_array("pokemon_data.json"))
	summary["move_count"] = _count_entries(load_dictionary("moves_data.json"))
	summary["item_count"] = _count_entries(load_array("items.json"))
	var maps: Variant = load_map_attributes()
	summary["map_attribute_count"] = _count_entries(maps)
	summary["content_pack_version"] = int(Dictionary(content_pack_index).get("version", 1))
	summary["content_pack_count"] = _count_content_packs(content_pack_index)
	return summary

func has_asset(relative_path: String) -> bool:
	return FileAccess.file_exists(assets_path(relative_path))

func has_data(relative_path: String) -> bool:
	return FileAccess.file_exists(data_path(relative_path))

func has_gfx(relative_path: String) -> bool:
	return FileAccess.file_exists(gfx_path(relative_path))

func _get_audio_assets() -> RefCounted:
	if _audio_assets == null:
		_audio_assets = AUDIO_ASSETS_SCRIPT.new()
		_audio_assets.initialize()
	return _audio_assets

func _get_exported_asset_manifest() -> RefCounted:
	if _exported_asset_manifest == null:
		_exported_asset_manifest = EXPORTED_ASSET_MANIFEST_SCRIPT.new()
		_exported_asset_manifest.initialize()
	return _exported_asset_manifest

func _strict_asset_key(value: String) -> String:
	var key := value.strip_edges()
	if key.is_empty():
		return ""
	if key.is_absolute_path() or key.contains("/") or key.contains("\\") or key.contains(".."):
		return ""
	return key

func _parse_palette_map(value: Variant, label: String) -> PackedInt32Array:
	var palette_map := PackedInt32Array()
	if typeof(value) == TYPE_ARRAY:
		var source := Array(value)
		palette_map.resize(source.size())
		for index in range(source.size()):
			palette_map[index] = _parse_int(source[index], 0)
		return palette_map
	if typeof(value) != TYPE_DICTIONARY:
		push_error("Palette map for %s must be an array or dictionary." % label)
		return palette_map
	var source_dict := Dictionary(value)
	var max_key := -1
	for raw_key in source_dict.keys():
		var key := _parse_int(raw_key, -1)
		if key > max_key:
			max_key = key
	if max_key < 0:
		return palette_map
	palette_map.resize(max_key + 1)
	for raw_key in source_dict.keys():
		var key := _parse_int(raw_key, -1)
		if key < 0:
			continue
		var raw_value: Variant = source_dict[raw_key]
		if typeof(raw_value) == TYPE_ARRAY:
			var entries := Array(raw_value)
			if entries.is_empty():
				continue
			palette_map[key] = _parse_int(entries[0], 0)
		else:
			palette_map[key] = _parse_int(raw_value, 0)
	return palette_map

func _parse_int(value: Variant, fallback: int) -> int:
	match typeof(value):
		TYPE_INT:
			return int(value)
		TYPE_FLOAT:
			return int(value)
		TYPE_STRING:
			var text := str(value).strip_edges()
			if text.is_empty():
				return fallback
			if text.begins_with("0x") or text.begins_with("0X"):
				return text.hex_to_int()
			if text.is_valid_int():
				return int(text)
			return fallback
		_:
			return fallback

func _ensure_initialized() -> void:
	if not _initialized:
		initialize()

func _resolve_path(base_dir: String, relative_path: String) -> String:
	if relative_path.is_empty():
		return ""
	if relative_path.is_absolute_path():
		return relative_path
	if base_dir.is_empty():
		return ""
	return base_dir.path_join(relative_path)

func _count_entries(value: Variant) -> int:
	match typeof(value):
		TYPE_ARRAY:
			return Array(value).size()
		TYPE_DICTIONARY:
			return Dictionary(value).size()
		_:
			return 0

func _count_content_packs(value: Variant) -> int:
	if typeof(value) != TYPE_DICTIONARY:
		return 0
	var index := Dictionary(value)
	var packs: Variant = index.get("packs", [])
	if typeof(packs) != TYPE_ARRAY:
		return 0
	var enabled_count := 0
	for pack in Array(packs):
		if typeof(pack) == TYPE_DICTIONARY and bool(Dictionary(pack).get("enabled", true)):
			enabled_count += 1
	return enabled_count
