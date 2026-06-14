extends RefCounted
class_name SaveStore

const GAME_STATE_SCRIPT = preload("res://scripts/game_state.gd")
const REPO_PATHS_SCRIPT = preload("res://scripts/repo_paths.gd")
const SAVE_EXTENSION := ".sav"
const BACKUP_EXTENSION := ".bak"
const SAVE_METADATA_EXTENSION := ".meta.json"
const SAVE_METADATA_VERSION := 1
const MANUAL_SAVE_SLOT := "savegame.sav"
const MANUAL_SAVE_HISTORY_SLOTS := ["savegame-recent-1.sav", "savegame-recent-2.sav"]
const MANUAL_SAVE_SLOTS := ["savegame.sav", "savegame-recent-1.sav", "savegame-recent-2.sav"]
const AUTOSAVE_SLOT := "autosave.sav"

var _save_root := ""

func set_save_root(path: String) -> void:
	_save_root = path

func save_root() -> String:
	if _save_root.is_empty():
		_save_root = REPO_PATHS_SCRIPT.saves_root()
	return _save_root

func ensure_save_root() -> void:
	var absolute := ProjectSettings.globalize_path(save_root())
	if not DirAccess.dir_exists_absolute(absolute):
		DirAccess.make_dir_recursive_absolute(absolute)

func slot_path(slot: String) -> String:
	var normalized := slot.strip_edges()
	if normalized.is_empty():
		normalized = "debug-shell"
	if not normalized.ends_with(SAVE_EXTENSION):
		normalized += SAVE_EXTENSION
	return save_root().path_join(normalized)

func manual_save_history_slots() -> Array:
	return MANUAL_SAVE_HISTORY_SLOTS.duplicate()

func manual_save_slots() -> Array:
	return MANUAL_SAVE_SLOTS.duplicate()

func autosave_slot() -> String:
	return AUTOSAVE_SLOT

func history_slots_for_slot(slot: String) -> Array:
	if is_manual_save_slot(slot):
		return manual_save_history_slots()
	return []

func is_manual_save_slot(slot: String) -> bool:
	return slot_path(slot).get_file() == MANUAL_SAVE_SLOT

func is_autosave_slot(slot: String) -> bool:
	return slot_path(slot).get_file() == AUTOSAVE_SLOT

func save_game(slot: String, state: Variant) -> bool:
	if state == null or not (state is Object):
		_report_save_error("invalid-state-object")
		return false
	var state_object: Object = state
	if not state_object.has_method("to_dictionary"):
		_report_save_error("missing-to_dictionary")
		return false
	ensure_save_root()
	var save_path := slot_path(slot)
	var absolute_save_path := ProjectSettings.globalize_path(save_path)
	var absolute_backup_path := absolute_save_path + BACKUP_EXTENSION
	var absolute_metadata_path := absolute_save_path + SAVE_METADATA_EXTENSION
	var absolute_backup_metadata_path := absolute_backup_path + SAVE_METADATA_EXTENSION
	var absolute_temp_path := _temporary_save_path(absolute_save_path)
	var absolute_metadata_temp_path := _temporary_save_path(absolute_metadata_path)
	var had_existing_save := FileAccess.file_exists(absolute_save_path)
	if had_existing_save:
		if not _copy_file_absolute(absolute_save_path, absolute_backup_path):
			_report_save_error("backup-copy-failed")
			return false
		if FileAccess.file_exists(absolute_metadata_path):
			if not _copy_file_absolute(absolute_metadata_path, absolute_backup_metadata_path):
				_report_save_error("backup-metadata-copy-failed")
				return false
	var snapshot: Variant = state_object.call("to_dictionary")
	if typeof(snapshot) != TYPE_DICTIONARY:
		_report_save_error("invalid-snapshot")
		return false
	var saved_at := _current_timestamp()
	var save_metadata := _build_save_metadata(save_path.get_file(), saved_at, snapshot)
	if state_object.has_method("set_save_metadata"):
		state_object.call("set_save_metadata", save_metadata)
		snapshot = state_object.call("to_dictionary")
		if typeof(snapshot) != TYPE_DICTIONARY:
			_report_save_error("invalid-metadata-snapshot")
			return false
	else:
		var snapshot_dictionary: Dictionary = snapshot
		snapshot_dictionary["save_metadata"] = save_metadata.duplicate(true)
		snapshot = snapshot_dictionary
	var normalized_snapshot: Variant = _normalize_variant(snapshot)
	if typeof(normalized_snapshot) != TYPE_DICTIONARY:
		_report_save_error("invalid-normalized-snapshot")
		return false
	var normalized_snapshot_dictionary: Dictionary = normalized_snapshot
	normalized_snapshot_dictionary["save_metadata"] = save_metadata.duplicate(true)
	normalized_snapshot = normalized_snapshot_dictionary
	var payload := JSON.stringify(normalized_snapshot, "    ")
	if not _write_temp_text(absolute_temp_path, payload):
		_report_save_error("temp-write-failed")
		return false
	if not _write_temp_text(absolute_metadata_temp_path, JSON.stringify(_metadata_sidecar(save_metadata), "    ")):
		if FileAccess.file_exists(absolute_temp_path):
			DirAccess.remove_absolute(absolute_temp_path)
		_report_save_error("metadata-temp-write-failed")
		return false
	if had_existing_save and FileAccess.file_exists(absolute_save_path):
		DirAccess.remove_absolute(absolute_save_path)
	if FileAccess.file_exists(absolute_metadata_path):
		DirAccess.remove_absolute(absolute_metadata_path)
	var rename_result := DirAccess.rename_absolute(absolute_temp_path, absolute_save_path)
	if rename_result != OK:
		_restore_backup_pair(absolute_backup_path, absolute_save_path, absolute_backup_metadata_path, absolute_metadata_path)
		if FileAccess.file_exists(absolute_temp_path):
			DirAccess.remove_absolute(absolute_temp_path)
		if FileAccess.file_exists(absolute_metadata_temp_path):
			DirAccess.remove_absolute(absolute_metadata_temp_path)
		_report_save_error("rename-failed")
		return false
	var metadata_rename_result := DirAccess.rename_absolute(absolute_metadata_temp_path, absolute_metadata_path)
	if metadata_rename_result != OK:
		if FileAccess.file_exists(absolute_save_path):
			DirAccess.remove_absolute(absolute_save_path)
		_restore_backup_pair(absolute_backup_path, absolute_save_path, absolute_backup_metadata_path, absolute_metadata_path)
		if FileAccess.file_exists(absolute_metadata_temp_path):
			DirAccess.remove_absolute(absolute_metadata_temp_path)
		_report_save_error("metadata-rename-failed")
		return false
	if FileAccess.file_exists(absolute_temp_path):
		DirAccess.remove_absolute(absolute_temp_path)
	return true

func save_manual_game(state: Variant) -> bool:
	return save_game_with_history(MANUAL_SAVE_SLOT, manual_save_history_slots(), state)

func save_autosave_game(state: Variant) -> bool:
	return save_game(AUTOSAVE_SLOT, state)

func save_game_with_history(slot: String, history_slots: Array, state: Variant) -> bool:
	if not is_manual_save_slot(slot) or typeof(history_slots) != TYPE_ARRAY or history_slots.is_empty():
		return save_game(slot, state)
	var primary_path := ProjectSettings.globalize_path(slot_path(slot))
	if FileAccess.file_exists(primary_path):
		for index in range(history_slots.size() - 1, 0, -1):
			_copy_save_slot(str(history_slots[index - 1]), str(history_slots[index]))
		_copy_save_slot(slot, str(history_slots[0]))
	return save_game(slot, state)

func load_game(slot: String) -> Dictionary:
	var save_path := slot_path(slot)
	var absolute_save_path := ProjectSettings.globalize_path(save_path)
	var absolute_backup_path := absolute_save_path + BACKUP_EXTENSION
	var primary_result := _load_state_from_path(absolute_save_path)
	if primary_result.get("ok", false):
		return primary_result
	if not FileAccess.file_exists(absolute_backup_path):
		return primary_result
	var backup_result := _load_state_from_path(absolute_backup_path)
	if not backup_result.get("ok", false):
		return primary_result
	if not _copy_file_absolute(absolute_backup_path, absolute_save_path):
		_report_save_error("restore-after-load-failed")
	else:
		var backup_metadata := absolute_backup_path + SAVE_METADATA_EXTENSION
		var primary_metadata := absolute_save_path + SAVE_METADATA_EXTENSION
		if FileAccess.file_exists(backup_metadata):
			_copy_file_absolute(backup_metadata, primary_metadata)
	return backup_result

func load_save_metadata(slot: String) -> Dictionary:
	var metadata := _read_metadata(ProjectSettings.globalize_path(slot_path(slot)))
	if metadata.is_empty():
		return {}
	metadata["slot"] = slot_path(slot).get_file()
	metadata["kind"] = _slot_kind(slot)
	return metadata

func has_save_metadata(slot: String) -> bool:
	return not load_save_metadata(slot).is_empty()

func save_history(slot: String) -> Array:
	var entries: Array = []
	var slots := [slot_path(slot).get_file()]
	for history_slot in history_slots_for_slot(slot):
		slots.append(slot_path(str(history_slot)).get_file())
	for index in range(slots.size()):
		var entry_slot := str(slots[index])
		var absolute_save_path := ProjectSettings.globalize_path(slot_path(str(entry_slot)))
		if not FileAccess.file_exists(absolute_save_path):
			continue
		var metadata := _read_metadata(absolute_save_path)
		entries.append({
			"slot": slot_path(str(entry_slot)).get_file(),
			"path": slot_path(str(entry_slot)),
			"kind": _slot_kind(str(entry_slot)),
			"saved_at": str(metadata.get("saved_at", "")),
			"frame_counter": max(0, int(metadata.get("frame_counter", 0))),
			"history_index": index,
			"is_current": index == 0,
			"exists": true,
		})
	return entries

func manual_save_history() -> Array:
	return save_history(MANUAL_SAVE_SLOT)

func autosave_history() -> Array:
	return save_history(AUTOSAVE_SLOT)

func save_slot_metadata() -> Array:
	var entries: Array = []
	for slot in manual_save_slots():
		var metadata := load_save_metadata(str(slot))
		if not metadata.is_empty():
			metadata["path"] = slot_path(str(slot))
			metadata["exists"] = true
			entries.append(metadata)
	var autosave_metadata := load_save_metadata(AUTOSAVE_SLOT)
	if not autosave_metadata.is_empty():
		autosave_metadata["path"] = slot_path(AUTOSAVE_SLOT)
		autosave_metadata["exists"] = true
		entries.append(autosave_metadata)
	return entries

func latest_save_metadata() -> Dictionary:
	var entries := save_slot_metadata()
	if entries.is_empty():
		return {}
	var latest: Dictionary = Dictionary(entries[0])
	for index in range(1, entries.size()):
		var candidate: Dictionary = Dictionary(entries[index])
		if str(candidate.get("saved_at", "")) > str(latest.get("saved_at", "")):
			latest = candidate
	return latest

func has_save_game(slot: String) -> bool:
	var absolute_save_path := ProjectSettings.globalize_path(slot_path(slot))
	return FileAccess.file_exists(absolute_save_path) or FileAccess.file_exists(absolute_save_path + BACKUP_EXTENSION)

func delete_save_game(slot: String) -> bool:
	var absolute_save_path := ProjectSettings.globalize_path(slot_path(slot))
	var deleted := false
	for target in [absolute_save_path, absolute_save_path + BACKUP_EXTENSION, absolute_save_path + SAVE_METADATA_EXTENSION, absolute_save_path + BACKUP_EXTENSION + SAVE_METADATA_EXTENSION]:
		if FileAccess.file_exists(target):
			DirAccess.remove_absolute(target)
			deleted = true
	var temp_dir := DirAccess.open(absolute_save_path.get_base_dir())
	if temp_dir != null:
		temp_dir.list_dir_begin()
		var temp_prefix := absolute_save_path.get_file() + "."
		var temp_suffix := ".tmp"
		while true:
			var entry := temp_dir.get_next()
			if entry.is_empty():
				break
			if temp_dir.current_is_dir():
				continue
			if entry.begins_with(temp_prefix) and entry.ends_with(temp_suffix):
				if DirAccess.remove_absolute(absolute_save_path.get_base_dir().path_join(entry)) == OK:
					deleted = true
		temp_dir.list_dir_end()
	return deleted

func _copy_file_absolute(source_path: String, destination_path: String) -> bool:
	if source_path.is_empty() or destination_path.is_empty():
		_report_save_error("copy-invalid-path")
		return false
	if not FileAccess.file_exists(source_path):
		_report_save_error("copy-missing-source")
		return false
	if FileAccess.file_exists(destination_path):
		DirAccess.remove_absolute(destination_path)
	var source_file := FileAccess.open(source_path, FileAccess.READ)
	if source_file == null:
		_report_save_error("copy-source-open-failed")
		return false
	var destination_file := FileAccess.open(destination_path, FileAccess.WRITE)
	if destination_file == null:
		source_file.close()
		_report_save_error("copy-destination-open-failed")
		return false
	destination_file.store_buffer(source_file.get_buffer(source_file.get_length()))
	source_file.close()
	destination_file.close()
	return FileAccess.file_exists(destination_path)

func _copy_file_atomic(source_path: String, destination_path: String) -> bool:
	if source_path.is_empty() or destination_path.is_empty():
		_report_save_error("atomic-copy-invalid-path")
		return false
	if not FileAccess.file_exists(source_path):
		_report_save_error("atomic-copy-missing-source")
		return false
	var temp_path := _temporary_save_path(destination_path)
	if not _copy_file_absolute(source_path, temp_path):
		if FileAccess.file_exists(temp_path):
			DirAccess.remove_absolute(temp_path)
		return false
	if FileAccess.file_exists(destination_path):
		DirAccess.remove_absolute(destination_path)
	var rename_result := DirAccess.rename_absolute(temp_path, destination_path)
	if rename_result != OK:
		if FileAccess.file_exists(temp_path):
			DirAccess.remove_absolute(temp_path)
		_report_save_error("atomic-copy-rename-failed")
		return false
	return true

func _copy_save_slot(source_slot: String, destination_slot: String) -> bool:
	var source_path := ProjectSettings.globalize_path(slot_path(source_slot))
	var destination_path := ProjectSettings.globalize_path(slot_path(destination_slot))
	if not FileAccess.file_exists(source_path):
		return false
	if not _copy_file_atomic(source_path, destination_path):
		return false
	var source_metadata_path := source_path + SAVE_METADATA_EXTENSION
	var destination_metadata_path := destination_path + SAVE_METADATA_EXTENSION
	if FileAccess.file_exists(source_metadata_path):
		return _copy_file_atomic(source_metadata_path, destination_metadata_path)
	if FileAccess.file_exists(destination_metadata_path):
		DirAccess.remove_absolute(destination_metadata_path)
	return true

func _write_temp_text(absolute_temp_path: String, text: String) -> bool:
	var file := FileAccess.open(absolute_temp_path, FileAccess.WRITE)
	if file == null:
		if FileAccess.file_exists(absolute_temp_path):
			DirAccess.remove_absolute(absolute_temp_path)
		return false
	file.store_string(text)
	file.flush()
	file.close()
	return FileAccess.file_exists(absolute_temp_path)

func _build_save_metadata(slot: String, saved_at: String, snapshot: Variant) -> Dictionary:
	var frame := 0
	if typeof(snapshot) == TYPE_DICTIONARY:
		frame = int(Dictionary(snapshot).get("frame_counter", 0))
	return {
		"schema_version": SAVE_METADATA_VERSION,
		"slot": slot,
		"saved_at": saved_at,
		"kind": _slot_kind(slot),
		"frame_counter": max(0, frame),
	}

func _metadata_sidecar(metadata: Dictionary) -> Dictionary:
	return {
		"schema_version": SAVE_METADATA_VERSION,
		"slot": str(metadata.get("slot", "")),
		"kind": str(metadata.get("kind", "")),
		"saved_at": str(metadata.get("saved_at", "")),
		"frame_counter": max(0, int(metadata.get("frame_counter", 0))),
	}

func _read_metadata(absolute_save_path: String) -> Dictionary:
	var metadata_path := absolute_save_path + SAVE_METADATA_EXTENSION
	if not FileAccess.file_exists(metadata_path):
		return {}
	var file := FileAccess.open(metadata_path, FileAccess.READ)
	if file == null:
		return {}
	var parsed: Variant = JSON.parse_string(file.get_as_text())
	file.close()
	if typeof(parsed) != TYPE_DICTIONARY:
		return {}
	var source: Dictionary = parsed
	if int(source.get("schema_version", 0)) != SAVE_METADATA_VERSION:
		return {}
	var saved_at := str(source.get("saved_at", "")).strip_edges()
	var slot := str(source.get("slot", "")).strip_edges()
	var kind := str(source.get("kind", "")).strip_edges()
	if saved_at.is_empty():
		return {}
	if slot.is_empty() or kind.is_empty():
		return {}
	return {
		"schema_version": SAVE_METADATA_VERSION,
		"slot": slot,
		"kind": kind,
		"saved_at": saved_at,
		"frame_counter": max(0, int(source.get("frame_counter", 0))),
	}

func _restore_backup_pair(backup_save_path: String, save_path: String, backup_metadata_path: String, metadata_path: String) -> void:
	if FileAccess.file_exists(backup_save_path):
		if not _copy_file_absolute(backup_save_path, save_path):
			_report_save_error("restore-failed")
	if FileAccess.file_exists(backup_metadata_path):
		if not _copy_file_absolute(backup_metadata_path, metadata_path):
			_report_save_error("restore-metadata-failed")

func _slot_kind(slot: String) -> String:
	var normalized := slot_path(slot).get_file()
	if normalized == MANUAL_SAVE_SLOT:
		return "manual"
	if MANUAL_SAVE_HISTORY_SLOTS.has(normalized):
		return "manual_history"
	if normalized == AUTOSAVE_SLOT:
		return "autosave"
	return "custom"

func _current_timestamp() -> String:
	return Time.get_datetime_string_from_system(true, true).replace(" ", "T") + ".000Z"

func _temporary_save_path(absolute_save_path: String) -> String:
	return "%s.%d.%d.tmp" % [absolute_save_path, OS.get_process_id(), Time.get_ticks_usec()]

func _load_state_from_path(absolute_path: String) -> Dictionary:
	if not FileAccess.file_exists(absolute_path):
		return {"ok": false, "error": "missing"}
	var file := FileAccess.open(absolute_path, FileAccess.READ)
	if file == null:
		return {"ok": false, "error": "unreadable"}
	var text := file.get_as_text()
	file.close()
	var parsed: Variant = JSON.parse_string(text)
	if typeof(parsed) != TYPE_DICTIONARY:
		return {"ok": false, "error": "invalid-json"}
	var normalized_parsed: Variant = _normalize_variant(parsed)
	if typeof(normalized_parsed) != TYPE_DICTIONARY:
		return {"ok": false, "error": "invalid-json"}
	var state_object: Object = GAME_STATE_SCRIPT.new()
	if state_object == null or not state_object.has_method("from_dictionary"):
		return {"ok": false, "error": "invalid-state-script"}
	if not bool(state_object.call("from_dictionary", normalized_parsed)):
		return {"ok": false, "error": "invalid-state"}
	if state_object.has_method("set_save_metadata"):
		var metadata := _read_metadata(absolute_path)
		if not metadata.is_empty():
			metadata["slot"] = absolute_path.get_file()
			metadata["kind"] = _slot_kind(absolute_path.get_file())
			state_object.call("set_save_metadata", metadata)
	return {"ok": true, "state": state_object}

func _report_save_error(message: String) -> void:
	push_error("SaveStore: %s" % message)

func _normalize_variant(value: Variant) -> Variant:
	match typeof(value):
		TYPE_DICTIONARY:
			return _normalize_dictionary(value)
		TYPE_ARRAY:
			return _normalize_array(value)
		TYPE_STRING, TYPE_INT, TYPE_FLOAT, TYPE_BOOL, TYPE_NIL:
			return value
		_:
			return null

func _normalize_dictionary(value: Variant) -> Dictionary:
	var normalized: Dictionary = {}
	if typeof(value) != TYPE_DICTIONARY:
		return normalized
	var source: Dictionary = value
	for key in source.keys():
		normalized[_normalize_key(key)] = _normalize_variant(source[key])
	return normalized

func _normalize_array(value: Variant) -> Array:
	var normalized_array: Array = []
	if typeof(value) != TYPE_ARRAY:
		return normalized_array
	var source_array: Array = value
	for entry in source_array:
		normalized_array.append(_normalize_variant(entry))
	return normalized_array

func _normalize_key(value: Variant) -> String:
	return str(value)
