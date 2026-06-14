extends RefCounted
class_name GBTileDecoder

const REPO_PATHS_SCRIPT := preload("res://scripts/repo_paths.gd")
const TILE_SIZE := 8
const GB_PALETTE_SIZE := 4
const ONE_BPP_BYTES_PER_ROW := 1
const TWO_BPP_BYTES_PER_ROW := 2
const ONE_BPP_BYTES_PER_TILE := TILE_SIZE * ONE_BPP_BYTES_PER_ROW
const TWO_BPP_BYTES_PER_TILE := TILE_SIZE * TWO_BPP_BYTES_PER_ROW
const METATILE_SIZE_TILES := 4
const METATILE_SIZE_PIXELS := METATILE_SIZE_TILES * TILE_SIZE
const METATILE_BYTES := METATILE_SIZE_TILES * METATILE_SIZE_TILES
const LZ_TERMINATOR := 0xFF

static func make_grayscale_palette(steps: int = 4, alpha: float = 1.0) -> Array[Color]:
	var palette: Array[Color] = []
	if steps <= 0:
		return palette
	if steps == 1:
		palette.append(Color(1.0, 1.0, 1.0, alpha))
		return palette

	for step in range(steps):
		var shade := 1.0 - (float(step) / float(steps - 1))
		palette.append(Color(shade, shade, shade, alpha))
	return palette

static func make_gb_palette(alpha: float = 1.0, transparent_last: bool = false) -> Array[Color]:
	var first_alpha := 0.0 if transparent_last else alpha
	return [
		Color(1.0, 1.0, 1.0, first_alpha),
		Color(0.70, 0.70, 0.70, alpha),
		Color(0.40, 0.40, 0.40, alpha),
		Color(0.10, 0.10, 0.10, alpha),
	]

static func make_transparent_gb_palette(alpha: float = 1.0) -> Array[Color]:
	return make_gb_palette(alpha, true)

static func make_transparent_grayscale_palette(steps: int = 4, alpha: float = 1.0) -> Array[Color]:
	var palette: Array[Color] = []
	if steps <= 0:
		return palette
	if steps == 1:
		palette.append(Color(1.0, 1.0, 1.0, 0.0))
		return palette
	for step in range(steps):
		var shade := 1.0 - (float(step) / float(steps - 1))
		var shade_alpha := 0.0 if step == 0 else alpha
		palette.append(Color(shade, shade, shade, shade_alpha))
	return palette

static func _palette_color(palette: Array[Color], index: int, fallback_palette: Array[Color]) -> Color:
	if index >= 0 and index < palette.size():
		return palette[index]
	if index >= 0 and index < fallback_palette.size():
		return fallback_palette[index]
	if not fallback_palette.is_empty():
		return fallback_palette[0]
	return Color(0.0, 0.0, 0.0, 0.0)

static func _decode_tile(bytes: PackedByteArray, bytes_per_row: int, palette: Array[Color], fallback_palette: Array[Color]) -> Image:
	return _decode_tile_at(bytes, 0, bytes_per_row, palette, fallback_palette)

static func _decode_tile_at(
	bytes: PackedByteArray,
	offset: int,
	bytes_per_row: int,
	palette: Array[Color],
	fallback_palette: Array[Color]
) -> Image:
	var image := Image.create(TILE_SIZE, TILE_SIZE, false, Image.FORMAT_RGBA8)
	var expected_bytes := offset + TILE_SIZE * bytes_per_row
	if bytes.size() < expected_bytes:
		return image

	var has_two_bitplanes := bytes_per_row == TWO_BPP_BYTES_PER_ROW
	for y in range(TILE_SIZE):
		var row_offset := offset + y * bytes_per_row
		var lo := bytes[row_offset]
		var hi := 0
		if has_two_bitplanes:
			hi = bytes[row_offset + 1]

		for x in range(TILE_SIZE):
			var bit := 7 - x
			var index := (lo >> bit) & 0x01
			if has_two_bitplanes:
				index |= ((hi >> bit) & 0x01) << 1
			image.set_pixel(x, y, _palette_color(palette, index, fallback_palette))
	return image

static func decode_1bpp_tile(bytes: PackedByteArray, palette: Array[Color] = []) -> Image:
	return _decode_tile(bytes, ONE_BPP_BYTES_PER_ROW, palette, make_grayscale_palette(2))

static func decode_2bpp_tile(bytes: PackedByteArray, palette: Array[Color] = []) -> Image:
	return _decode_tile(bytes, TWO_BPP_BYTES_PER_ROW, palette, make_gb_palette())

static func load_raw_bytes(path: String) -> PackedByteArray:
	var file := FileAccess.open(_resolve_path(path), FileAccess.READ)
	if file == null:
		return PackedByteArray()
	return file.get_buffer(file.get_length())

static func load_raw_bytes_path(absolute_path: String) -> PackedByteArray:
	var file := FileAccess.open(absolute_path, FileAccess.READ)
	if file == null:
		return PackedByteArray()
	return file.get_buffer(file.get_length())

static func load_tile_bytes(path: String) -> PackedByteArray:
	return load_tile_bytes_path(_resolve_path(path))

static func load_tile_bytes_path(absolute_path: String) -> PackedByteArray:
	if absolute_path.ends_with(".lz"):
		var compressed := load_raw_bytes_path(absolute_path)
		if not compressed.is_empty():
			return decompress_lz(compressed)
		return load_raw_bytes_path(absolute_path.trim_suffix(".lz"))
	return load_raw_bytes_path(absolute_path)

static func load_1bpp_tile(path: String, palette: Array[Color] = []) -> Image:
	return decode_1bpp_tile(load_tile_bytes(path), palette)

static func load_1bpp_tile_path(absolute_path: String, palette: Array[Color] = []) -> Image:
	return decode_1bpp_tile(load_tile_bytes_path(absolute_path), palette)

static func load_2bpp_tile(path: String, palette: Array[Color] = []) -> Image:
	return decode_2bpp_tile(load_tile_bytes(path), palette)

static func load_2bpp_tile_path(absolute_path: String, palette: Array[Color] = []) -> Image:
	return decode_2bpp_tile(load_tile_bytes_path(absolute_path), palette)

static func _decode_tiles_range(
	bytes: PackedByteArray,
	bytes_per_tile: int,
	bytes_per_row: int,
	start_tile: int,
	tile_count: int,
	palette: Array[Color],
	fallback_palette: Array[Color]
) -> Array[Image]:
	var tiles: Array[Image] = []
	if bytes_per_tile <= 0 or bytes.size() < bytes_per_tile:
		return tiles
	var safe_start: int = int(max(0, start_tile))
	var start_offset: int = safe_start * bytes_per_tile
	if start_offset >= bytes.size():
		return tiles
	var available_tiles: int = int((bytes.size() - start_offset) / bytes_per_tile)
	var count: int = available_tiles
	if tile_count >= 0:
		count = int(min(tile_count, available_tiles))
	for tile_index in range(count):
		var offset: int = start_offset + tile_index * bytes_per_tile
		tiles.append(_decode_tile_at(bytes, offset, bytes_per_row, palette, fallback_palette))
	return tiles

static func _fit_tile_bytes(bytes: PackedByteArray, tile_count: int, bytes_per_tile: int) -> PackedByteArray:
	if tile_count <= 0 or bytes_per_tile <= 0:
		return bytes
	var expected_bytes: int = tile_count * bytes_per_tile
	var fitted := PackedByteArray()
	fitted.resize(expected_bytes)
	var copy_count: int = int(min(bytes.size(), expected_bytes))
	for idx in range(copy_count):
		fitted[idx] = bytes[idx]
	return fitted

static func decode_1bpp_tiles(bytes: PackedByteArray, palette: Array[Color] = []) -> Array[Image]:
	return _decode_tiles(bytes, ONE_BPP_BYTES_PER_TILE, ONE_BPP_BYTES_PER_ROW, palette, make_grayscale_palette(2))

static func decode_2bpp_tiles(bytes: PackedByteArray, palette: Array[Color] = []) -> Array[Image]:
	return _decode_tiles(bytes, TWO_BPP_BYTES_PER_TILE, TWO_BPP_BYTES_PER_ROW, palette, make_gb_palette())

static func decode_1bpp_tiles_range(
	bytes: PackedByteArray,
	start_tile: int = 0,
	tile_count: int = -1,
	palette: Array[Color] = []
) -> Array[Image]:
	return _decode_tiles_range(bytes, ONE_BPP_BYTES_PER_TILE, ONE_BPP_BYTES_PER_ROW, start_tile, tile_count, palette, make_grayscale_palette(2))

static func decode_2bpp_tiles_range(
	bytes: PackedByteArray,
	start_tile: int = 0,
	tile_count: int = -1,
	palette: Array[Color] = []
) -> Array[Image]:
	return _decode_tiles_range(bytes, TWO_BPP_BYTES_PER_TILE, TWO_BPP_BYTES_PER_ROW, start_tile, tile_count, palette, make_gb_palette())

static func decode_1bpp_tiles_padded(bytes: PackedByteArray, tile_count: int, palette: Array[Color] = []) -> Array[Image]:
	return decode_1bpp_tiles(_fit_tile_bytes(bytes, tile_count, ONE_BPP_BYTES_PER_TILE), palette)

static func decode_2bpp_tiles_padded(bytes: PackedByteArray, tile_count: int, palette: Array[Color] = []) -> Array[Image]:
	return decode_2bpp_tiles(_fit_tile_bytes(bytes, tile_count, TWO_BPP_BYTES_PER_TILE), palette)

static func load_1bpp_tiles(path: String, palette: Array[Color] = []) -> Array[Image]:
	return decode_1bpp_tiles(load_tile_bytes(path), palette)

static func load_1bpp_tiles_path(absolute_path: String, palette: Array[Color] = []) -> Array[Image]:
	return decode_1bpp_tiles(load_tile_bytes_path(absolute_path), palette)

static func load_2bpp_tiles(path: String, palette: Array[Color] = []) -> Array[Image]:
	return decode_2bpp_tiles(load_tile_bytes(path), palette)

static func load_2bpp_tiles_path(absolute_path: String, palette: Array[Color] = []) -> Array[Image]:
	return decode_2bpp_tiles(load_tile_bytes_path(absolute_path), palette)

static func load_1bpp_tiles_padded(path: String, tile_count: int, palette: Array[Color] = []) -> Array[Image]:
	return load_1bpp_tiles_padded_path(_resolve_path(path), tile_count, palette)

static func load_1bpp_tiles_padded_path(absolute_path: String, tile_count: int, palette: Array[Color] = []) -> Array[Image]:
	if absolute_path.is_empty():
		return []
	if not FileAccess.file_exists(absolute_path) and not FileAccess.file_exists(absolute_path.trim_suffix(".lz")):
		return []
	return decode_1bpp_tiles_padded(load_tile_bytes_path(absolute_path), tile_count, palette)

static func load_2bpp_tiles_padded(path: String, tile_count: int, palette: Array[Color] = []) -> Array[Image]:
	return load_2bpp_tiles_padded_path(_resolve_path(path), tile_count, palette)

static func load_2bpp_tiles_padded_path(absolute_path: String, tile_count: int, palette: Array[Color] = []) -> Array[Image]:
	if absolute_path.is_empty():
		return []
	if not FileAccess.file_exists(absolute_path) and not FileAccess.file_exists(absolute_path.trim_suffix(".lz")):
		return []
	return decode_2bpp_tiles_padded(load_tile_bytes_path(absolute_path), tile_count, palette)

static func assemble_tile_grid(
	tiles: Array[Image],
	columns: int,
	rows: int = -1,
	background: Color = Color(0.0, 0.0, 0.0, 0.0)
) -> Image:
	var tile_count: int = tiles.size()
	if columns <= 0 or tile_count <= 0:
		return Image.new()
	var grid_rows: int = rows
	if grid_rows <= 0:
		grid_rows = int(ceil(float(tile_count) / float(columns)))
	if grid_rows <= 0:
		return Image.new()
	var image: Image = Image.create(columns * TILE_SIZE, grid_rows * TILE_SIZE, false, Image.FORMAT_RGBA8)
	image.fill(background)
	var max_tiles: int = int(min(tile_count, columns * grid_rows))
	for tile_index in range(max_tiles):
		var tile: Image = tiles[tile_index]
		if tile == null or tile.is_empty():
			continue
		var src_rect := Rect2i(Vector2i.ZERO, Vector2i(int(min(TILE_SIZE, tile.get_width())), int(min(TILE_SIZE, tile.get_height()))))
		var dest := Vector2i(int(tile_index % columns) * TILE_SIZE, int(tile_index / columns) * TILE_SIZE)
		image.blit_rect(tile, src_rect, dest)
	return image

static func assemble_tile_atlas(
	tiles: Array[Image],
	columns: int,
	background: Color = Color(0.0, 0.0, 0.0, 0.0)
) -> Image:
	return assemble_tile_grid(tiles, columns, -1, background)

static func assemble_indexed_tilemap(
	tiles: Array[Image],
	tile_indices: PackedInt32Array,
	width_tiles: int,
	height_tiles: int = -1,
	background: Color = Color(0.0, 0.0, 0.0, 0.0)
) -> Image:
	if width_tiles <= 0 or tile_indices.is_empty():
		return Image.new()
	var map_height: int = height_tiles
	if map_height <= 0:
		map_height = int(ceil(float(tile_indices.size()) / float(width_tiles)))
	if map_height <= 0:
		return Image.new()
	var image: Image = Image.create(width_tiles * TILE_SIZE, map_height * TILE_SIZE, false, Image.FORMAT_RGBA8)
	image.fill(background)
	var max_indices: int = int(min(tile_indices.size(), width_tiles * map_height))
	for map_index in range(max_indices):
		var tile_index: int = int(tile_indices[map_index])
		if tile_index < 0 or tile_index >= tiles.size():
			continue
		var tile: Image = tiles[tile_index]
		if tile == null or tile.is_empty():
			continue
		var src_rect := Rect2i(Vector2i.ZERO, Vector2i(int(min(TILE_SIZE, tile.get_width())), int(min(TILE_SIZE, tile.get_height()))))
		var dest := Vector2i(int(map_index % width_tiles) * TILE_SIZE, int(map_index / width_tiles) * TILE_SIZE)
		image.blit_rect(tile, src_rect, dest)
	return image

static func slice_image_tiles(source: Image, columns: int = -1, rows: int = -1) -> Array[Image]:
	var tiles: Array[Image] = []
	if source == null or source.is_empty():
		return tiles
	var source_image: Image = source.duplicate()
	if source_image.get_format() != Image.FORMAT_RGBA8:
		source_image.convert(Image.FORMAT_RGBA8)
	var source_columns: int = columns
	if source_columns <= 0:
		source_columns = int(source_image.get_width() / TILE_SIZE)
	var source_rows: int = rows
	if source_rows <= 0:
		source_rows = int(source_image.get_height() / TILE_SIZE)
	if source_columns <= 0 or source_rows <= 0:
		return tiles
	if source_image.get_width() < source_columns * TILE_SIZE or source_image.get_height() < source_rows * TILE_SIZE:
		return tiles
	for tile_y in range(source_rows):
		for tile_x in range(source_columns):
			var tile := Image.create(TILE_SIZE, TILE_SIZE, false, Image.FORMAT_RGBA8)
			var src_rect := Rect2i(Vector2i(tile_x * TILE_SIZE, tile_y * TILE_SIZE), Vector2i(TILE_SIZE, TILE_SIZE))
			tile.blit_rect(source_image, src_rect, Vector2i.ZERO)
			tiles.append(tile)
	return tiles

static func decode_metatile_ids(bytes: PackedByteArray) -> Array[PackedInt32Array]:
	var metatiles: Array[PackedInt32Array] = []
	if bytes.is_empty() or bytes.size() % METATILE_BYTES != 0:
		return metatiles
	for offset in range(0, bytes.size(), METATILE_BYTES):
		var tile_ids := PackedInt32Array()
		tile_ids.resize(METATILE_BYTES)
		for index in range(METATILE_BYTES):
			tile_ids[index] = int(bytes[offset + index])
		metatiles.append(tile_ids)
	return metatiles

static func assemble_metatile(
	tiles: Array[Image],
	tile_ids: PackedInt32Array,
	background: Color = Color(0.0, 0.0, 0.0, 0.0)
) -> Image:
	if tile_ids.size() < METATILE_BYTES:
		return Image.new()
	var image := Image.create(METATILE_SIZE_PIXELS, METATILE_SIZE_PIXELS, false, Image.FORMAT_RGBA8)
	image.fill(background)
	for tile_index in range(METATILE_BYTES):
		var source_index: int = int(tile_ids[tile_index])
		if source_index < 0 or source_index >= tiles.size():
			continue
		var tile: Image = tiles[source_index]
		if tile == null or tile.is_empty():
			continue
		var dest := Vector2i((tile_index % METATILE_SIZE_TILES) * TILE_SIZE, int(tile_index / METATILE_SIZE_TILES) * TILE_SIZE)
		image.blit_rect(tile, Rect2i(Vector2i.ZERO, Vector2i(TILE_SIZE, TILE_SIZE)), dest)
	return image

static func assemble_metatiles(
	tiles: Array[Image],
	metatile_ids: Array[PackedInt32Array],
	background: Color = Color(0.0, 0.0, 0.0, 0.0)
) -> Array[Image]:
	var metatiles: Array[Image] = []
	for tile_ids in metatile_ids:
		metatiles.append(assemble_metatile(tiles, tile_ids, background))
	return metatiles

static func assemble_map_blocks(
	metatiles: Array[Image],
	block_ids: PackedByteArray,
	width_blocks: int,
	height_blocks: int = -1,
	background: Color = Color(0.0, 0.0, 0.0, 0.0)
) -> Image:
	if width_blocks <= 0 or block_ids.is_empty():
		return Image.new()
	var map_height: int = height_blocks
	if map_height <= 0:
		map_height = int(ceil(float(block_ids.size()) / float(width_blocks)))
	if map_height <= 0:
		return Image.new()
	var image := Image.create(width_blocks * METATILE_SIZE_PIXELS, map_height * METATILE_SIZE_PIXELS, false, Image.FORMAT_RGBA8)
	image.fill(background)
	var max_blocks: int = int(min(block_ids.size(), width_blocks * map_height))
	for block_index in range(max_blocks):
		var metatile_index: int = int(block_ids[block_index])
		if metatile_index < 0 or metatile_index >= metatiles.size():
			continue
		var metatile: Image = metatiles[metatile_index]
		if metatile == null or metatile.is_empty():
			continue
		var dest := Vector2i((block_index % width_blocks) * METATILE_SIZE_PIXELS, int(block_index / width_blocks) * METATILE_SIZE_PIXELS)
		image.blit_rect(metatile, Rect2i(Vector2i.ZERO, Vector2i(METATILE_SIZE_PIXELS, METATILE_SIZE_PIXELS)), dest)
	return image

static func decode_1bpp_atlas(bytes: PackedByteArray, columns: int, palette: Array[Color] = []) -> Image:
	return assemble_tile_atlas(decode_1bpp_tiles(bytes, palette), columns)

static func decode_2bpp_atlas(bytes: PackedByteArray, columns: int, palette: Array[Color] = []) -> Image:
	return assemble_tile_atlas(decode_2bpp_tiles(bytes, palette), columns)

static func load_1bpp_atlas(path: String, columns: int, palette: Array[Color] = []) -> Image:
	return decode_1bpp_atlas(load_tile_bytes(path), columns, palette)

static func load_1bpp_atlas_path(absolute_path: String, columns: int, palette: Array[Color] = []) -> Image:
	return decode_1bpp_atlas(load_tile_bytes_path(absolute_path), columns, palette)

static func load_2bpp_atlas(path: String, columns: int, palette: Array[Color] = []) -> Image:
	return decode_2bpp_atlas(load_tile_bytes(path), columns, palette)

static func load_2bpp_atlas_path(absolute_path: String, columns: int, palette: Array[Color] = []) -> Image:
	return decode_2bpp_atlas(load_tile_bytes_path(absolute_path), columns, palette)

static func load_gbcpal_palette(path: String) -> Array[Color]:
	return load_gbcpal_palette_path(_resolve_path(path))

static func load_gbcpal_palette_path(absolute_path: String) -> Array[Color]:
	return _load_binary_palette(absolute_path)

static func load_palette(path: String, palette_name: String = "") -> Array[Color]:
	return load_palette_path(_resolve_path(path), palette_name)

static func load_palette_path(absolute_path: String, palette_name: String = "") -> Array[Color]:
	if absolute_path.is_empty():
		var empty_palette: Array[Color] = []
		return empty_palette
	if not palette_name.strip_edges().is_empty() and not absolute_path.ends_with(".gbcpal"):
		return _load_ascii_palette(absolute_path, palette_name)
	var banks := load_palette_bank_path(absolute_path)
	if banks.is_empty():
		if absolute_path.ends_with(".gbcpal"):
			var empty_binary_palette: Array[Color] = []
			return empty_binary_palette
		return _load_ascii_palette(absolute_path, palette_name)
	if palette_name.strip_edges().is_empty():
		var default_palette: Array[Color] = []
		for color in Array(banks[0]):
			if typeof(color) == TYPE_COLOR:
				default_palette.append(color)
		return default_palette
	for bank in banks:
		if typeof(bank) != TYPE_ARRAY:
			continue
		var palette: Array[Color] = []
		for color in Array(bank):
			if typeof(color) == TYPE_COLOR:
				palette.append(color)
		if palette.size() == GB_PALETTE_SIZE:
			return palette
	var missing_palette: Array[Color] = []
	return missing_palette

static func load_predef_palette(path: String, palette_name: String) -> Array[Color]:
	return load_predef_palette_path(_resolve_path(path), palette_name)

static func load_predef_palette_path(absolute_path: String, palette_name: String) -> Array[Color]:
	return _load_ascii_palette(absolute_path, palette_name)

static func load_palette_bank(path: String) -> Array:
	return load_palette_bank_path(_resolve_path(path))

static func load_palette_bank_path(absolute_path: String) -> Array:
	if absolute_path.is_empty():
		return []
	if absolute_path.ends_with(".gbcpal"):
		return _load_binary_palette_bank(absolute_path)
	return _load_ascii_palette_bank(absolute_path)

static func reorder_sprite_pair_tiles(tiles: Array, tiles_wide: int, tiles_high: int) -> Array:
	if tiles_wide <= 0 or tiles_high <= 0 or tiles_high % 2 != 0:
		return tiles.duplicate(true)
	var ordered: Array = []
	for sprite_row in range(int(tiles_high / 2)):
		for tile_x in range(tiles_wide):
			var top_index := sprite_row * 2 * tiles_wide + tile_x
			var bottom_index := top_index + tiles_wide
			if top_index >= tiles.size() or bottom_index >= tiles.size():
				continue
			ordered.append(tiles[top_index])
			ordered.append(tiles[bottom_index])
	return ordered

static func decompress_lz(bytes: PackedByteArray) -> PackedByteArray:
	var dest := PackedByteArray()
	var src_idx := 0
	while src_idx < bytes.size():
		if bytes[src_idx] == LZ_TERMINATOR:
			break
		var ctrl := bytes[src_idx]
		src_idx += 1

		var cmd := ctrl & 0xE0
		var length := 0
		if cmd == 0xE0:
			cmd = (ctrl & 0x1C) << 3
			var len_hi := ctrl & 0x03
			var len_lo := bytes[src_idx]
			src_idx += 1
			length = ((len_hi << 8) | len_lo) + 1
		else:
			length = (ctrl & 0x1F) + 1

		if cmd < 0x80:
			if cmd == 0x00:
				for _i in range(length):
					dest.append(bytes[src_idx])
					src_idx += 1
			elif cmd == 0x20:
				var repeated := bytes[src_idx]
				src_idx += 1
				for _i in range(length):
					dest.append(repeated)
			elif cmd == 0x40:
				var first := bytes[src_idx]
				var second := bytes[src_idx + 1]
				src_idx += 2
				for i in range(length):
					dest.append(first if (i % 2) == 0 else second)
			elif cmd == 0x60:
				for _i in range(length):
					dest.append(0)
		else:
			var offset_byte1 := bytes[src_idx]
			src_idx += 1
			var rw_idx := 0
			if (offset_byte1 & 0x80) != 0:
				var offset := offset_byte1 & 0x7F
				rw_idx = dest.size() - offset - 1
			else:
				var offset_byte2 := bytes[src_idx]
				src_idx += 1
				rw_idx = ((offset_byte1 & 0x7F) << 8) | offset_byte2

			if cmd == 0x80:
				for _i in range(length):
					if rw_idx < 0 or rw_idx >= dest.size():
						return PackedByteArray()
					dest.append(dest[rw_idx])
					rw_idx += 1
			elif cmd == 0xA0:
				for _i in range(length):
					if rw_idx < 0 or rw_idx >= dest.size():
						return PackedByteArray()
					var original_byte := int(dest[rw_idx])
					var flipped_byte := 0
					for _bit in range(8):
						flipped_byte = (flipped_byte << 1) | (original_byte & 1)
						original_byte >>= 1
					dest.append(flipped_byte)
					rw_idx += 1
			elif cmd == 0xC0:
				var idx := rw_idx
				for _i in range(length):
					if idx < 0 or idx >= dest.size():
						return PackedByteArray()
					dest.append(dest[idx])
					idx -= 1
	return dest

static func _load_binary_palette(absolute_path: String) -> Array[Color]:
	var bytes := load_raw_bytes_path(absolute_path)
	var palette: Array[Color] = []
	if bytes.is_empty() or bytes.size() % 2 != 0:
		return palette
	for offset in range(0, bytes.size(), 2):
		var value := int(bytes[offset]) | (int(bytes[offset + 1]) << 8)
		var red := float(value & 0x1f) / 31.0
		var green := float((value >> 5) & 0x1f) / 31.0
		var blue := float((value >> 10) & 0x1f) / 31.0
		palette.append(Color(red, green, blue, 1.0))
	return palette

static func _load_binary_palette_bank(absolute_path: String) -> Array:
	var palette := _load_binary_palette(absolute_path)
	if palette.is_empty():
		return []
	var bank: Array = []
	for offset in range(0, palette.size(), GB_PALETTE_SIZE):
		var slice: Array = []
		for index in range(offset, min(offset + GB_PALETTE_SIZE, palette.size())):
			slice.append(palette[index])
		if slice.size() == GB_PALETTE_SIZE:
			bank.append(slice)
	return bank

static func _load_ascii_palette(absolute_path: String, palette_name: String = "") -> Array[Color]:
	var text: String = _read_text_file(absolute_path)
	if text.is_empty():
		return []
	var expected_label := palette_name.strip_edges().to_upper()
	var palette: Array[Color] = []
	var collected: Array[Color] = []
	for raw_line in text.split("\n"):
		var line: Array = raw_line.split(";", 1)
		var body: String = String(line[0]).strip_edges()
		if body.is_empty():
			continue
		var comment: String = ""
		if line.size() > 1:
			comment = String(line[1]).strip_edges().to_upper()
		if not comment.is_empty() and not expected_label.is_empty() and comment.find("PREDEFPAL_%s" % expected_label) == -1:
			continue
		if not body.to_upper().begins_with("RGB"):
			continue
		var values := _parse_rgb_numbers(body)
		if values.size() < 3 or values.size() % 3 != 0:
			continue
		palette.clear()
		for idx in range(0, values.size(), 3):
			palette.append(_gb5_color(values[idx], values[idx + 1], values[idx + 2]))
		if expected_label.is_empty():
			collected.append_array(palette)
		if not expected_label.is_empty() or palette.size() == 4:
			return palette
		if palette.size() and expected_label.is_empty():
			continue
	if expected_label.is_empty() and collected.size() >= 2:
		return [
			Color(1.0, 1.0, 1.0, 1.0),
			collected[0],
			collected[1],
			Color(0.0, 0.0, 0.0, 1.0),
		]
	return palette

static func _load_ascii_palette_bank(absolute_path: String) -> Array:
	var text: String = _read_text_file(absolute_path)
	if text.is_empty():
		return []
	var bank: Array = []
	var palette: Array = []
	for raw_line in text.split("\n"):
		var line: Array = raw_line.split(";", 1)
		var body: String = String(line[0]).strip_edges()
		if body.is_empty() or not body.to_upper().begins_with("RGB"):
			continue
		var values := _parse_rgb_numbers(body)
		if values.size() < 3 or values.size() % 3 != 0:
			continue
		for idx in range(0, values.size(), 3):
			palette.append(_gb5_color(values[idx], values[idx + 1], values[idx + 2]))
			if palette.size() == GB_PALETTE_SIZE:
				bank.append(palette.duplicate(true))
				palette.clear()
	return bank

static func _parse_rgb_numbers(line: String) -> PackedInt32Array:
	var values := PackedInt32Array()
	var cleaned := line.replace("RGB", "").replace(",", " ")
	for token in cleaned.split(" ", false):
		var trimmed := token.strip_edges()
		if trimmed.is_empty():
			continue
		if not trimmed.is_valid_int():
			continue
		values.append(int(trimmed))
	return values

static func _gb5_color(r: int, g: int, b: int) -> Color:
	return Color(float(r) / 31.0, float(g) / 31.0, float(b) / 31.0, 1.0)

static func _read_text_file(absolute_path: String) -> String:
	if absolute_path.is_empty() or not FileAccess.file_exists(absolute_path):
		return ""
	var file := FileAccess.open(absolute_path, FileAccess.READ)
	if file == null:
		return ""
	var text := file.get_as_text()
	file = null
	return text

static func _decode_tiles(
	bytes: PackedByteArray,
	bytes_per_tile: int,
	bytes_per_row: int,
	palette: Array[Color],
	fallback_palette: Array[Color]
) -> Array[Image]:
	var tiles: Array[Image] = []
	if bytes_per_tile <= 0 or bytes.size() < bytes_per_tile:
		return tiles
	var limit := bytes.size() - (bytes.size() % bytes_per_tile)
	for offset in range(0, limit, bytes_per_tile):
		tiles.append(_decode_tile_at(bytes, offset, bytes_per_row, palette, fallback_palette))
	return tiles

static func _resolve_path(path: String) -> String:
	if path.is_empty():
		return ""
	if path.is_absolute_path() or path.begins_with("res://") or path.begins_with("user://"):
		return path
	var assets_root := REPO_PATHS_SCRIPT.web_assets_root()
	if not assets_root.is_empty():
		var assets_candidate := assets_root.path_join(path)
		if FileAccess.file_exists(assets_candidate):
			return assets_candidate
	var repo_root := REPO_PATHS_SCRIPT.repo_root()
	if not repo_root.is_empty():
		var repo_candidate := repo_root.path_join(path)
		if FileAccess.file_exists(repo_candidate):
			return repo_candidate
	return path
