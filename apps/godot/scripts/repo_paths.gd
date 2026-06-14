extends RefCounted
class_name RepoPaths

static var _repo_root: String = ""
static var _web_assets_root: String = ""

static func project_dir() -> String:
	return _normalize_dir(ProjectSettings.globalize_path("res://"))

static func assets_root() -> String:
	return web_assets_root()

static func user_data_root() -> String:
	return _normalize_dir(OS.get_user_data_dir())

static func logs_root() -> String:
	var root := user_data_root()
	if root.is_empty():
		return ""
	return root.path_join("logs")

static func saves_root() -> String:
	var root := user_data_root()
	if root.is_empty():
		return ""
	return root.path_join("saves")

static func repo_root() -> String:
	if _repo_root.is_empty():
		_repo_root = _find_workspace_root()
	return _repo_root

static func workspace_root() -> String:
	return repo_root()

static func web_assets_root() -> String:
	if _web_assets_root.is_empty():
		_web_assets_root = _find_web_assets_root()
	return _web_assets_root

static func data_root() -> String:
	var assets_root := web_assets_root()
	if assets_root.is_empty():
		return ""
	return assets_root.path_join("data")

static func gfx_root() -> String:
	var assets_root := web_assets_root()
	if assets_root.is_empty():
		return ""
	return assets_root.path_join("gfx")

static func _find_workspace_root() -> String:
	var anchors := _candidate_anchors()
	for anchor in anchors:
		var root := _search_upwards(anchor)
		if not root.is_empty():
			return root
	return ""

static func _candidate_anchors() -> Array[String]:
	var anchors: Array[String] = []
	var project := project_dir()
	_append_anchor(anchors, project)
	_append_anchor(anchors, _parent_dir(project))
	_append_anchor(anchors, _parent_dir(_parent_dir(project)))
	var executable_dir := _normalize_dir(OS.get_executable_path().get_base_dir())
	_append_anchor(anchors, executable_dir)
	_append_anchor(anchors, _parent_dir(executable_dir))
	_append_anchor(anchors, _parent_dir(_parent_dir(executable_dir)))
	return anchors

static func _append_anchor(anchors: Array[String], path: String) -> void:
	var normalized := _normalize_dir(path)
	if normalized.is_empty():
		return
	if anchors.has(normalized):
		return
	anchors.append(normalized)

static func _search_upwards(start_dir: String) -> String:
	var current := _normalize_dir(start_dir)
	while not current.is_empty():
		if _looks_like_workspace_root(current):
			return current
		var parent := _parent_dir(current)
		if parent.is_empty() or parent == current:
			break
		current = parent
	return ""

static func _find_web_assets_root() -> String:
	var anchors := _candidate_anchors()
	for anchor in anchors:
		var root := _search_for_assets_root(anchor)
		if not root.is_empty():
			return root
	return ""

static func _search_for_assets_root(start_dir: String) -> String:
	var current := _normalize_dir(start_dir)
	while not current.is_empty():
		var candidate := _find_assets_root_under(current)
		if not candidate.is_empty():
			return candidate
		var parent := _parent_dir(current)
		if parent.is_empty() or parent == current:
			break
		current = parent
	return ""

static func _find_assets_root_under(base_dir: String) -> String:
	var candidates := [
		base_dir,
		base_dir.path_join("apps/web/assets"),
		base_dir.path_join("apps/godot/assets"),
		base_dir.path_join("assets"),
		base_dir.path_join("public/assets"),
	]
	for candidate in candidates:
		if _looks_like_assets_root(candidate):
			return candidate
	return ""

static func _looks_like_workspace_root(path: String) -> bool:
	return FileAccess.file_exists(path.path_join("package.json")) and _looks_like_assets_root(
		path.path_join("apps/web/assets")
	)

static func _looks_like_assets_root(path: String) -> bool:
	if path.is_empty():
		return false
	if not DirAccess.dir_exists_absolute(path):
		return false
	var data_dir := path.path_join("data")
	if not DirAccess.dir_exists_absolute(data_dir):
		return false
	var gfx_dir := path.path_join("gfx")
	if not DirAccess.dir_exists_absolute(gfx_dir):
		return false
	return FileAccess.file_exists(data_dir.path_join("pokemon_data.json")) or FileAccess.file_exists(
		data_dir.path_join("runtime_map_metadata.json")
	) or FileAccess.file_exists(data_dir.path_join("runtime_spawn_points.json")) or FileAccess.file_exists(
		data_dir.path_join("map_attributes.json")
	) or FileAccess.file_exists(data_dir.path_join("map_blocks.json")) or FileAccess.file_exists(
		data_dir.path_join("battle_animation_table.json")
	)

static func _parent_dir(path: String) -> String:
	if path.is_empty():
		return ""
	return _normalize_dir(path.get_base_dir())

static func _normalize_dir(path: String) -> String:
	if path.is_empty():
		return ""
	var normalized := path.trim_suffix("/")
	if normalized.is_empty():
		return path
	return normalized
