extends RefCounted
class_name ExportedAssetManifest

const REPO_PATHS_SCRIPT := preload("res://scripts/repo_paths.gd")
const GB_TILE_DECODER_SCRIPT := preload("res://scripts/gb_tile_decoder.gd")
const AUDIO_ASSETS_SCRIPT := preload("res://scripts/audio_assets.gd")

const AUDIO_DIR := "audio"
const DATA_DIR := "data"
const GFX_DIR := "gfx"
const PNG_EXTENSION := ".png"
const JSON_EXTENSION := ".json"
const MP3_EXTENSION := ".mp3"
const ONE_BPP_EXTENSION := ".1bpp"
const TWO_BPP_EXTENSION := ".2bpp"

var assets_root := ""
var data_root := ""
var gfx_root := ""
var _initialized := false
var _audio_assets: RefCounted = null

func initialize() -> void:
	if _initialized:
		return
	assets_root = REPO_PATHS_SCRIPT.web_assets_root()
	data_root = REPO_PATHS_SCRIPT.data_root()
	gfx_root = REPO_PATHS_SCRIPT.gfx_root()
	_initialized = true

func build_asset_manifest() -> Dictionary:
	_ensure_initialized()
	var json_assets := list_data_json_assets()
	var png_assets := list_png_assets()
	var native_assets := list_gb_native_graphics()
	var audio_assets := list_mp3_assets()
	var audio_manifests: Array = Array(_get_audio_assets().audio_manifest_paths())
	return {
		"assets_root": assets_root,
		"json_count": json_assets.size(),
		"png_count": png_assets.size(),
		"gb_native_count": native_assets.size(),
		"mp3_count": audio_assets.size(),
		"audio_manifest_count": audio_manifests.size(),
		"json": json_assets,
		"png": png_assets,
		"gb_native": native_assets,
		"mp3": audio_assets,
		"audio_manifests": audio_manifests,
	}

func validate_core_exported_assets() -> Dictionary:
	var checks: Array[Dictionary] = [
		validate_data_json("menu_icons.json"),
		validate_data_json("pokemon_cries.json"),
		validate_data_json("battle_anim_bundle.json"),
		validate_gfx_png("title/logo.png"),
		validate_gfx_png("icons/pikachu.png"),
		validate_gb_native_graphic("title/logo.2bpp"),
		validate_gb_native_graphic("font/font.1bpp"),
		validate_mp3("route29.mp3"),
	]
	var failures: Array[Dictionary] = []
	for check in checks:
		if not bool(check.get("ok", false)):
			failures.append(check)
	return {
		"ok": failures.is_empty(),
		"checked_count": checks.size(),
		"failure_count": failures.size(),
		"checks": checks,
		"failures": failures,
	}

func list_data_json_assets() -> Array[Dictionary]:
	_ensure_initialized()
	return _list_assets(data_root, JSON_EXTENSION, DATA_DIR)

func list_png_assets() -> Array[Dictionary]:
	_ensure_initialized()
	return _list_assets(gfx_root, PNG_EXTENSION, GFX_DIR)

func list_mp3_assets() -> Array[Dictionary]:
	_ensure_initialized()
	var records: Array[Dictionary] = []
	for root in _get_audio_assets().audio_root_candidates():
		var root_string := str(root)
		for record in _list_assets(root_string, MP3_EXTENSION, AUDIO_DIR):
			records.append(record)
	records.sort_custom(func(a: Dictionary, b: Dictionary) -> bool:
		return str(a.get("relative_path", "")) < str(b.get("relative_path", ""))
	)
	return records

func list_gb_native_graphics() -> Array[Dictionary]:
	_ensure_initialized()
	var records: Array[Dictionary] = []
	for record in _list_assets(gfx_root, ONE_BPP_EXTENSION, GFX_DIR):
		records.append(_native_record_from_relative_path(str(record.get("relative_path", ""))))
	for record in _list_assets(gfx_root, TWO_BPP_EXTENSION, GFX_DIR):
		records.append(_native_record_from_relative_path(str(record.get("relative_path", ""))))
	records.sort_custom(func(a: Dictionary, b: Dictionary) -> bool:
		return str(a.get("relative_path", "")) < str(b.get("relative_path", ""))
	)
	return records

func validate_data_json(relative_path: String) -> Dictionary:
	var safe_path := _safe_relative_path(relative_path, JSON_EXTENSION)
	return _validate_file(data_root, safe_path, "json")

func validate_gfx_png(relative_path: String) -> Dictionary:
	var safe_path := _safe_relative_path(relative_path, PNG_EXTENSION)
	var result := _validate_file(gfx_root, safe_path, "png")
	if bool(result.get("ok", false)):
		var image := Image.new()
		var err := image.load(str(result.get("absolute_path", "")))
		result["image_ok"] = err == OK and not image.is_empty()
		result["width"] = image.get_width() if err == OK else 0
		result["height"] = image.get_height() if err == OK else 0
		result["ok"] = bool(result["image_ok"])
	return result

func validate_mp3(relative_path: String) -> Dictionary:
	var safe_path := _safe_relative_path(relative_path, MP3_EXTENSION)
	if safe_path.is_empty():
		return _empty_validation("mp3", relative_path)
	for root in _get_audio_assets().audio_root_candidates():
		var result := _validate_file(str(root), safe_path, "mp3")
		if bool(result.get("ok", false)):
			return result
	var roots: PackedStringArray = _get_audio_assets().audio_root_candidates()
	if roots.is_empty():
		return _empty_validation("mp3", safe_path)
	return _validate_file(str(roots[0]), safe_path, "mp3")

func validate_gb_native_graphic(relative_path: String) -> Dictionary:
	var record := resolve_gb_native_graphic(relative_path)
	var ok := bool(record.get("exists", false)) and int(record.get("byte_size", 0)) > 0 and int(record.get("tile_count", 0)) > 0
	record["ok"] = ok
	return record

func resolve_gb_native_graphic(relative_path: String) -> Dictionary:
	return _native_record_from_relative_path(relative_path)

func decode_gb_native_tiles(relative_path: String, palette: Array[Color] = []) -> Array[Image]:
	var record := resolve_gb_native_graphic(relative_path)
	if not bool(record.get("exists", false)):
		push_error("Missing GB-native graphic %s." % relative_path)
		return []
	var absolute_path := str(record.get("absolute_path", ""))
	var encoding := str(record.get("encoding", ""))
	if encoding == "1bpp":
		return GB_TILE_DECODER_SCRIPT.load_1bpp_tiles_path(absolute_path, palette)
	if encoding == "2bpp":
		return GB_TILE_DECODER_SCRIPT.load_2bpp_tiles_path(absolute_path, palette)
	push_error("Unsupported GB-native graphic encoding for %s." % relative_path)
	return []

func load_gb_native_atlas(relative_path: String, columns: int, palette: Array[Color] = []) -> Image:
	var tiles := decode_gb_native_tiles(relative_path, palette)
	if tiles.is_empty():
		return Image.new()
	return GB_TILE_DECODER_SCRIPT.assemble_tile_atlas(tiles, columns)

func _native_record_from_relative_path(relative_path: String) -> Dictionary:
	var safe_path := _safe_relative_path(relative_path, "")
	if safe_path.is_empty() or (not safe_path.ends_with(ONE_BPP_EXTENSION) and not safe_path.ends_with(TWO_BPP_EXTENSION)):
		return _empty_native_record(relative_path)
	var absolute_path := gfx_root.path_join(safe_path)
	var exists := FileAccess.file_exists(absolute_path)
	var byte_size := _file_size(absolute_path) if exists else 0
	var bytes_per_tile := 8 if safe_path.ends_with(ONE_BPP_EXTENSION) else 16
	var encoding := "1bpp" if bytes_per_tile == 8 else "2bpp"
	var tile_count := int(byte_size / bytes_per_tile) if bytes_per_tile > 0 and byte_size % bytes_per_tile == 0 else 0
	var png_path := safe_path.trim_suffix(ONE_BPP_EXTENSION).trim_suffix(TWO_BPP_EXTENSION) + PNG_EXTENSION
	var png_absolute_path := gfx_root.path_join(png_path)
	var palette_path := safe_path.get_base_dir().path_join("%s.pal" % safe_path.get_file().get_basename())
	var palette_absolute_path := gfx_root.path_join(palette_path)
	return {
		"kind": "gb_native",
		"relative_path": safe_path,
		"absolute_path": absolute_path,
		"exists": exists,
		"byte_size": byte_size,
		"encoding": encoding,
		"bytes_per_tile": bytes_per_tile,
		"tile_count": tile_count,
		"png_relative_path": png_path,
		"png_absolute_path": png_absolute_path,
		"png_exists": FileAccess.file_exists(png_absolute_path),
		"palette_relative_path": palette_path,
		"palette_absolute_path": palette_absolute_path,
		"palette_exists": FileAccess.file_exists(palette_absolute_path),
	}

func _empty_native_record(relative_path: String) -> Dictionary:
	return {
		"kind": "gb_native",
		"relative_path": relative_path,
		"absolute_path": "",
		"exists": false,
		"byte_size": 0,
		"encoding": "",
		"bytes_per_tile": 0,
		"tile_count": 0,
		"png_relative_path": "",
		"png_absolute_path": "",
		"png_exists": false,
		"palette_relative_path": "",
		"palette_absolute_path": "",
		"palette_exists": false,
	}

func _list_assets(root: String, extension: String, category: String) -> Array[Dictionary]:
	var records: Array[Dictionary] = []
	if root.is_empty():
		return records
	_collect_assets(root, root, extension, category, records)
	records.sort_custom(func(a: Dictionary, b: Dictionary) -> bool:
		return str(a.get("relative_path", "")) < str(b.get("relative_path", ""))
	)
	return records

func _collect_assets(root: String, current_dir: String, extension: String, category: String, records: Array[Dictionary]) -> void:
	var dir := DirAccess.open(current_dir)
	if dir == null:
		return
	dir.list_dir_begin()
	while true:
		var entry := dir.get_next()
		if entry.is_empty():
			break
		if entry.begins_with("."):
			continue
		var absolute_path := current_dir.path_join(entry)
		if dir.current_is_dir():
			_collect_assets(root, absolute_path, extension, category, records)
			continue
		if not entry.to_lower().ends_with(extension):
			continue
		var relative_path := absolute_path.substr(root.length() + 1)
		records.append({
			"category": category,
			"relative_path": relative_path,
			"absolute_path": absolute_path,
			"byte_size": _file_size(absolute_path),
		})
	dir.list_dir_end()

func _validate_file(root: String, relative_path: String, category: String) -> Dictionary:
	if root.is_empty() or relative_path.is_empty():
		return _empty_validation(category, relative_path)
	var absolute_path := root.path_join(relative_path)
	var exists := FileAccess.file_exists(absolute_path)
	var byte_size := _file_size(absolute_path) if exists else 0
	return {
		"category": category,
		"relative_path": relative_path,
		"absolute_path": absolute_path,
		"exists": exists,
		"byte_size": byte_size,
		"ok": exists and byte_size > 0,
	}

func _empty_validation(category: String, relative_path: String) -> Dictionary:
	return {
		"category": category,
		"relative_path": relative_path,
		"absolute_path": "",
		"exists": false,
		"byte_size": 0,
		"ok": false,
	}

func _safe_relative_path(relative_path: String, extension: String) -> String:
	var safe_path := relative_path.strip_edges()
	if safe_path.is_empty() or safe_path.is_absolute_path() or safe_path.contains("\\") or safe_path.contains(".."):
		return ""
	if safe_path.begins_with("/") or safe_path.ends_with("/"):
		return ""
	if not extension.is_empty() and not safe_path.to_lower().ends_with(extension):
		return ""
	for part in safe_path.split("/", false):
		if str(part).strip_edges().is_empty():
			return ""
	return safe_path

func _file_size(absolute_path: String) -> int:
	if absolute_path.is_empty() or not FileAccess.file_exists(absolute_path):
		return 0
	var file := FileAccess.open(absolute_path, FileAccess.READ)
	if file == null:
		return 0
	var size := int(file.get_length())
	file = null
	return size

func _get_audio_assets() -> RefCounted:
	if _audio_assets == null:
		_audio_assets = AUDIO_ASSETS_SCRIPT.new()
		_audio_assets.initialize()
	return _audio_assets

func _ensure_initialized() -> void:
	if not _initialized:
		initialize()
