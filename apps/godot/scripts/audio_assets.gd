extends RefCounted
class_name AudioAssets

const REPO_PATHS_SCRIPT := preload("res://scripts/repo_paths.gd")

const AUDIO_DIR := "audio"
const POKEMON_CRIES_MANIFEST := "pokemon_cries.json"
const CONTENT_PACKS_DIR := "content-packs"
const AUDIO_ASSETS_MANIFEST := "audio_assets.json"
const MP3_EXTENSION := ".mp3"
const MENU_SOUND_ALIASES := {
	"menu_cursor": "SFX_MENU",
	"menu_option": "SFX_READ_TEXT_2",
	"menu_cancel": "SFX_READ_TEXT_2",
}
const PRIORITY_SFX_TOKENS := {
	"SFX_FANFARE": true,
	"SFX_FANFARE_2": true,
	"SFX_CAUGHT_MON": true,
	"SFX_LEVEL_UP": true,
	"SFX_REGISTER_PHONE_NUMBER": true,
	"SFX_PRESENT": true,
	"SFX_1ST_PLACE": true,
	"SFX_2ND_PLACE": true,
	"SFX_3RD_PLACE": true,
	"SFX_GET_EGG": true,
	"SFX_GET_EGG_UNUSED": true,
	"SFX_GET_TM": true,
	"SFX_GET_BADGE": true,
	"SFX_GET_TRADEMON": true,
	"SFX_EVOLVED": true,
}
const PRIORITY_SFX_PREFIXES := ["SFX_DEX_FANFARE_", "SFX_GET_"]

var repo_root := ""
var assets_root := ""
var data_root := ""
var _initialized := false
var _json_cache: Dictionary = {}
var _bytes_cache: Dictionary = {}
var _disassembly_aliases: Dictionary = {}

func initialize() -> void:
	if _initialized:
		return
	repo_root = REPO_PATHS_SCRIPT.repo_root()
	assets_root = REPO_PATHS_SCRIPT.web_assets_root()
	data_root = REPO_PATHS_SCRIPT.data_root()
	_initialized = true

func audio_root_candidates() -> PackedStringArray:
	_ensure_initialized()
	var roots := PackedStringArray()
	_append_unique_path(roots, assets_root.path_join(AUDIO_DIR))
	_append_unique_path(roots, repo_root.path_join("apps/web/public/assets/audio"))
	_append_unique_path(roots, repo_root.path_join("public/assets/audio"))
	_append_unique_path(roots, repo_root.path_join("assets/audio"))
	return roots

func load_pokemon_cries() -> Dictionary:
	var value: Variant = _load_json_data_path(POKEMON_CRIES_MANIFEST)
	if typeof(value) != TYPE_DICTIONARY:
		return {}
	return Dictionary(value)

func load_audio_manifest(relative_path: String) -> Dictionary:
	_ensure_initialized()
	return load_audio_manifest_path(data_root.path_join(relative_path))

func load_audio_manifest_path(absolute_path: String) -> Dictionary:
	var value: Variant = _load_json_path(absolute_path)
	if typeof(value) != TYPE_DICTIONARY:
		return {}
	return Dictionary(value)

func load_disassembly_aliases() -> Dictionary:
	_ensure_initialized()
	if _disassembly_aliases.is_empty():
		_disassembly_aliases = _load_disassembly_aliases()
	return _disassembly_aliases.duplicate(true)

func audio_manifest_paths() -> PackedStringArray:
	_ensure_initialized()
	var paths := PackedStringArray()
	var root := data_root.path_join(CONTENT_PACKS_DIR)
	_collect_audio_manifest_paths(root, paths)
	paths.sort()
	return paths

func load_audio_manifests() -> Array[Dictionary]:
	var manifests: Array[Dictionary] = []
	for manifest_path in audio_manifest_paths():
		var manifest := load_audio_manifest_path(manifest_path)
		if manifest.is_empty():
			continue
		manifests.append({
			"path": manifest_path,
			"manifest": manifest,
			"entry_count": _count_audio_manifest_entries(manifest),
			"music_count": Dictionary(manifest.get("music", {})).size() if typeof(manifest.get("music", {})) == TYPE_DICTIONARY else 0,
			"sound_count": Dictionary(manifest.get("sounds", {})).size() if typeof(manifest.get("sounds", {})) == TYPE_DICTIONARY else 0,
		})
	return manifests

func resolve_music_cue(token: String) -> Dictionary:
	var normalized := _normalize_token(token)
	if normalized.is_empty():
		return _empty_cue("music", token)
	var key := normalized
	if normalized.to_upper().begins_with("MUSIC_"):
		key = normalized.substr(6)
	var alias_map: Dictionary = Dictionary(load_disassembly_aliases().get("music", {}))
	var mapped := str(alias_map.get(normalized.to_upper(), _slugify(key)))
	if mapped.ends_with(MP3_EXTENSION):
		mapped = mapped.trim_suffix(MP3_EXTENSION)
	var path_name := "%s%s" % [mapped, MP3_EXTENSION]
	return _build_cue_metadata("music", token, mapped, path_name, {
		"loop": true,
		"priority_class": "none",
		"priority_sound": false,
		"fade_music": false,
	})

func resolve_sfx_cue(token: String) -> Dictionary:
	var normalized := _normalize_token(token)
	if normalized.is_empty():
		return _empty_cue("sfx", token)
	var key := normalized
	if normalized.to_upper().begins_with("SFX_"):
		key = normalized.substr(4)
	var normalized_token := normalized.to_upper()
	var priority_class := "priority" if _is_priority_sound_token(normalized_token) else "none"
	var alias_map: Dictionary = Dictionary(load_disassembly_aliases().get("sfx", {}))
	var mapped := str(alias_map.get(normalized_token, "sfx/%s" % _slugify(key)))
	if mapped.ends_with(MP3_EXTENSION):
		mapped = mapped.trim_suffix(MP3_EXTENSION)
	var path_name := "%s%s" % [mapped, MP3_EXTENSION]
	return _build_cue_metadata("sfx", token, mapped, path_name, {
		"loop": false,
		"priority_class": priority_class,
		"priority_sound": priority_class != "none",
		"fade_music": priority_class != "none",
	})

func resolve_cry_cue(species_or_cry: String) -> Dictionary:
	var normalized := _normalize_token(species_or_cry)
	if normalized.is_empty():
		return _empty_cue("cry", species_or_cry)
	var species := normalized.to_upper()
	if species.begins_with("CRY_"):
		species = species.substr(4)
	elif species.ends_with("_CRY"):
		species = species.substr(0, species.length() - 4)
	var cry_base := _resolve_cry_base(species)
	return _build_cue_metadata("cry", species_or_cry, cry_base, "cries/%s%s" % [cry_base, MP3_EXTENSION], {
		"loop": false,
		"priority_class": "cry",
		"priority_sound": true,
		"fade_music": true,
	})

func resolve_audio_cue(category: String, token: String) -> Dictionary:
	match category.strip_edges().to_lower():
		"music":
			var manifest_cue := resolve_audio_manifest_entry("music", token)
			return manifest_cue if not manifest_cue.is_empty() else resolve_music_cue(token)
		"sfx", "sound", "sounds":
			var manifest_sfx_cue := resolve_audio_manifest_entry("sounds", token)
			if not manifest_sfx_cue.is_empty():
				return manifest_sfx_cue
			return resolve_sfx_cue(token)
		"cry", "cries":
			return resolve_cry_cue(token)
		_:
			return _empty_cue(category, token)

func resolve_audio_manifest_entry(category: String, token: String) -> Dictionary:
	var normalized := _normalize_token(token)
	if normalized.is_empty():
		return {}
	var lookup := _normalize_manifest_key(normalized)
	if lookup.is_empty():
		return {}
	var bucket_name := "music" if category.strip_edges().to_lower() == "music" else "sounds"
	for manifest_entry in load_audio_manifests():
		var manifest_path := str(manifest_entry.get("path", ""))
		var manifest: Dictionary = Dictionary(manifest_entry.get("manifest", {}))
		var bucket: Variant = manifest.get(bucket_name, {})
		if typeof(bucket) != TYPE_DICTIONARY:
			continue
		var bucket_dict := Dictionary(bucket)
		for key in bucket_dict.keys():
			var key_name := str(key)
			var entry_value: Variant = bucket_dict[key]
			var entry_path := ""
			if typeof(entry_value) == TYPE_STRING:
				entry_path = str(entry_value)
			elif typeof(entry_value) == TYPE_DICTIONARY:
				var entry := Dictionary(entry_value)
				entry_path = str(entry.get("path", entry.get("source", entry.get("file", entry.get("assetPath", entry.get("mixedPath", ""))))))
			if entry_path.is_empty():
				continue
			if _normalize_manifest_key(key_name) == lookup:
				return cue_metadata_for_manifest_entry(category, key_name, entry_value, manifest_path)
			var stripped := key_name
			if stripped.to_upper().begins_with("MUSIC_"):
				stripped = stripped.substr(6)
			elif stripped.to_upper().begins_with("SFX_"):
				stripped = stripped.substr(4)
			if _normalize_manifest_key(stripped) == lookup:
				return cue_metadata_for_manifest_entry(category, key_name, entry_value, manifest_path)
	return {}

func load_audio_cue_bytes(cue: Dictionary) -> PackedByteArray:
	var absolute_path := str(cue.get("absolute_path", ""))
	if absolute_path.is_empty() or not FileAccess.file_exists(absolute_path):
		push_error("Missing audio cue file: %s" % str(cue.get("relative_path", "")))
		return PackedByteArray()
	return load_audio_bytes_path(absolute_path)

func load_audio_bytes_path(absolute_path: String) -> PackedByteArray:
	if absolute_path.is_empty() or not FileAccess.file_exists(absolute_path):
		return PackedByteArray()
	if _bytes_cache.has(absolute_path):
		return PackedByteArray(_bytes_cache[absolute_path])
	var file := FileAccess.open(absolute_path, FileAccess.READ)
	if file == null:
		return PackedByteArray()
	var bytes := file.get_buffer(file.get_length())
	file = null
	_bytes_cache[absolute_path] = bytes
	return bytes

func load_audio_stream(cue: Dictionary) -> AudioStream:
	var bytes := load_audio_cue_bytes(cue)
	if bytes.is_empty():
		return null
	var stream := AudioStreamMP3.new()
	stream.data = bytes
	return stream

func build_audio_playback_plan(category: String, token: String, options: Dictionary = {}) -> Dictionary:
	var cue := resolve_audio_cue(category, token)
	return build_audio_playback_plan_for_cue(cue, options)

func build_audio_playback_plan_for_cue(cue: Dictionary, options: Dictionary = {}) -> Dictionary:
	var normalized_cue := cue.duplicate(true)
	var stream: AudioStream = null
	if bool(options.get("load_stream", false)):
		stream = load_audio_stream(normalized_cue)
	var priority_class := str(normalized_cue.get("priority_class", "none"))
	var priority_sound := bool(normalized_cue.get("priority_sound", false))
	var fade_music := bool(normalized_cue.get("fade_music", false))
	var plan := {
		"ok": bool(normalized_cue.get("exists", false)),
		"cue": normalized_cue,
		"stream": stream,
		"category": str(normalized_cue.get("category", "")),
		"token": str(normalized_cue.get("token", "")),
		"key": str(normalized_cue.get("key", "")),
		"relative_path": str(normalized_cue.get("relative_path", "")),
		"absolute_path": str(normalized_cue.get("absolute_path", "")),
		"manifest_path": str(normalized_cue.get("manifest_path", "")),
		"loop": bool(normalized_cue.get("loop", false)),
		"priority_class": priority_class,
		"priority_sound": priority_sound,
		"fade_music": fade_music,
		"should_mute_music": priority_sound,
		"should_fade_music": fade_music,
		"owned_channels": Array(normalized_cue.get("owned_channels", [])),
		"duration_frames": int(normalized_cue.get("duration_frames", 0)),
		"play": bool(options.get("play", false)),
		"autoplay": bool(options.get("autoplay", true)),
		"music_fade_frames": int(options.get("music_fade_frames", 0)),
		"music_fade_ms": int(options.get("music_fade_ms", 0)),
		"applied": false,
		"played": false,
	}
	if typeof(options) == TYPE_DICTIONARY:
		for key in Dictionary(options).keys():
			if not plan.has(key):
				plan[key] = options[key]
	return plan

func create_audio_playback_state() -> Dictionary:
	return {
		"music_token": "",
		"music_role": "general",
		"music_source": "",
		"music_frame": 0,
		"faded_volume": 1.0,
		"active_channels": [],
		"recent_events": [],
		"pending_sounds": [],
		"channel_owners": {},
		"current_sfx_priority": null,
		"priority_mute_count": 0,
		"music_muted_by_priority": false,
		"music_muted_by_controller": false,
		"suppressed_music_channels": [],
		"event_sequence": 0,
	}

func schedule_audio_playback_plan(state: Dictionary, plan: Dictionary) -> Dictionary:
	var scheduler := _ensure_audio_playback_state(state)
	var normalized_plan := plan.duplicate(true)
	var decision := {
		"ok": bool(normalized_plan.get("ok", false)),
		"allowed": false,
		"queued": false,
		"replaced": [],
		"priority_class": str(normalized_plan.get("priority_class", "none")),
		"priority_sound": bool(normalized_plan.get("priority_sound", false)),
		"fade_music": bool(normalized_plan.get("fade_music", false)),
		"should_fade_music": bool(normalized_plan.get("should_fade_music", false)),
		"should_mute_music": bool(normalized_plan.get("should_mute_music", false)),
		"active_channels": [],
	}
	if not bool(decision["ok"]):
		return decision
	var category := str(normalized_plan.get("category", ""))
	var token := str(normalized_plan.get("token", ""))
	var event := {
		"kind": category if not category.is_empty() else "other",
		"token": token,
		"source": str(normalized_plan.get("relative_path", normalized_plan.get("absolute_path", ""))),
		"loop": bool(normalized_plan.get("loop", false)),
		"role": str(normalized_plan.get("role", scheduler.get("music_role", "general"))),
	}
	if category == "music":
		scheduler["music_token"] = token
		scheduler["music_role"] = str(normalized_plan.get("role", scheduler.get("music_role", "general")))
		scheduler["music_source"] = str(normalized_plan.get("relative_path", normalized_plan.get("absolute_path", "")))
		scheduler["music_frame"] = int(normalized_plan.get("music_frame", scheduler.get("music_frame", 0)))
		scheduler["faded_volume"] = float(normalized_plan.get("faded_volume", scheduler.get("faded_volume", 1.0)))
		_register_audio_event(scheduler, event)
		decision["allowed"] = true
		return decision
	var priority: Variant = _playback_priority_for_plan(normalized_plan)
	var current_priority: Variant = scheduler.get("current_sfx_priority", null)
	if priority != null and (current_priority == null or int(current_priority) >= int(priority)):
		decision["replaced"] = _release_sound_channels(scheduler)
		for entry in Array(decision["replaced"]):
			if typeof(entry) == TYPE_DICTIONARY and bool(Dictionary(entry).get("priority_sound", false)):
				scheduler["priority_mute_count"] = max(0, int(scheduler.get("priority_mute_count", 0)) - 1)
		scheduler["current_sfx_priority"] = int(priority)
		decision["allowed"] = true
	elif priority != null:
		var pending := Array(scheduler.get("pending_sounds", []))
		pending.append(normalized_plan.duplicate(true))
		scheduler["pending_sounds"] = pending
		decision["queued"] = true
		return decision
	var owned_channels := Array(normalized_plan.get("owned_channels", []))
	if owned_channels.is_empty() and category != "music":
		owned_channels = _default_owned_channels_for_category(category)
	for channel in owned_channels:
		_register_active_channel(scheduler, channel, token, category, str(normalized_plan.get("priority_class", "none")), priority)
	decision["active_channels"] = _snapshot_active_channels(scheduler)
	if bool(normalized_plan.get("priority_sound", false)) or bool(normalized_plan.get("fade_music", false)):
		scheduler["priority_mute_count"] = max(0, int(scheduler.get("priority_mute_count", 0)) + 1)
	scheduler["music_muted_by_priority"] = int(scheduler.get("priority_mute_count", 0)) > 0
	_refresh_suppressed_music_channels(scheduler)
	_register_audio_event(scheduler, event)
	decision["should_mute_music"] = bool(scheduler.get("music_muted_by_priority", false))
	decision["allowed"] = true
	return decision

func release_audio_playback_plan(state: Dictionary, plan_or_token: Variant) -> Dictionary:
	var scheduler := _ensure_audio_playback_state(state)
	var token := ""
	var priority_sound := false
	if typeof(plan_or_token) == TYPE_DICTIONARY:
		var plan: Dictionary = plan_or_token
		token = str(plan.get("token", ""))
		priority_sound = bool(plan.get("priority_sound", false))
	else:
		token = str(plan_or_token)
	if token.is_empty():
		return build_audio_playback_snapshot(scheduler)
	var active_channels := Array(scheduler.get("active_channels", []))
	var next_channels: Array = []
	var removed_priority := false
	for entry in active_channels:
		if typeof(entry) != TYPE_DICTIONARY:
			continue
		var channel_entry: Dictionary = Dictionary(entry)
		if str(channel_entry.get("ownerToken", "")) == token:
			if bool(channel_entry.get("priority_sound", false)):
				removed_priority = true
			continue
		next_channels.append(channel_entry.duplicate(true))
	scheduler["active_channels"] = next_channels
	if removed_priority or priority_sound:
		scheduler["priority_mute_count"] = max(0, int(scheduler.get("priority_mute_count", 0)) - 1)
		scheduler["music_muted_by_priority"] = int(scheduler.get("priority_mute_count", 0)) > 0
	var next_priority: Variant = null
	for entry in next_channels:
		if typeof(entry) != TYPE_DICTIONARY:
			continue
		var channel_entry := Dictionary(entry)
		if str(channel_entry.get("category", "")) == "music":
			continue
		var entry_priority: Variant = channel_entry.get("priority", null)
		if entry_priority == null:
			continue
		if next_priority == null or int(entry_priority) < int(next_priority):
			next_priority = int(entry_priority)
	scheduler["current_sfx_priority"] = next_priority
	_refresh_suppressed_music_channels(scheduler)
	return build_audio_playback_snapshot(scheduler)

func build_audio_playback_snapshot(state: Dictionary) -> Dictionary:
	var scheduler := _ensure_audio_playback_state(state)
	return {
		"musicToken": str(scheduler.get("music_token", "")),
		"musicRole": str(scheduler.get("music_role", "general")),
		"musicSource": str(scheduler.get("music_source", "")),
		"musicFrame": int(scheduler.get("music_frame", 0)),
		"fadedVolume": float(scheduler.get("faded_volume", 1.0)),
		"activeChannels": _snapshot_active_channels(scheduler),
		"recentEvents": Array(scheduler.get("recent_events", [])).duplicate(true),
		"currentSfxPriority": scheduler.get("current_sfx_priority", null),
		"priorityMuteCount": int(scheduler.get("priority_mute_count", 0)),
		"musicMutedByPriority": bool(scheduler.get("music_muted_by_priority", false)),
		"musicMutedByController": bool(scheduler.get("music_muted_by_controller", false)),
		"suppressedMusicChannels": Array(scheduler.get("suppressed_music_channels", [])).duplicate(true),
		"pendingSounds": Array(scheduler.get("pending_sounds", [])).duplicate(true),
	}

func apply_audio_playback_plan(player: Object, plan: Dictionary) -> Dictionary:
	var applied := plan.duplicate(true)
	if player == null:
		applied["applied"] = false
		return applied
	var stream: Variant = plan.get("stream", null)
	if stream != null and _object_has_property(player, "stream"):
		player.set("stream", stream)
		if _object_has_property(stream, "loop") and bool(plan.get("loop", false)):
			stream.set("loop", true)
		applied["applied"] = true
	if _object_has_property(player, "loop"):
		player.set("loop", bool(plan.get("loop", false)))
	if _object_has_property(player, "autoplay"):
		player.set("autoplay", bool(plan.get("autoplay", true)))
	if _object_has_property(player, "bus") and not str(plan.get("bus", "")).is_empty():
		player.set("bus", str(plan.get("bus", "")))
	if _object_has_property(player, "volume_db") and plan.has("volume_db"):
		player.set("volume_db", float(plan.get("volume_db", 0.0)))
	if _object_has_property(player, "pitch_scale") and plan.has("pitch_scale"):
		player.set("pitch_scale", float(plan.get("pitch_scale", 1.0)))
	if player.has_method("set_meta"):
		player.set_meta("audio_playback_plan", applied.duplicate(true))
		player.set_meta("audio_playback_cue", Dictionary(plan.get("cue", {})).duplicate(true))
	return applied

func play_audio_cue(player: Object, category: String, token: String, options: Dictionary = {}) -> Dictionary:
	var plan := build_audio_playback_plan(category, token, options)
	if plan.get("stream", null) == null:
		plan["stream"] = load_audio_stream(Dictionary(plan.get("cue", {})))
	var scheduler_state: Variant = options.get("scheduler_state", options.get("audio_scheduler_state", null))
	var schedule_result: Dictionary = {}
	if typeof(scheduler_state) == TYPE_DICTIONARY:
		schedule_result = schedule_audio_playback_plan(Dictionary(scheduler_state), plan)
		plan["scheduler"] = schedule_result.duplicate(true)
		if not bool(schedule_result.get("allowed", true)):
			plan["applied"] = false
			plan["played"] = false
			return plan
	var applied := apply_audio_playback_plan(player, plan)
	var should_play := bool(applied.get("autoplay", true)) or bool(applied.get("play", false))
	if player != null and should_play and player.has_method("play") and bool(applied.get("applied", false)):
		player.call("play")
		applied["played"] = true
	var audio_engine: Variant = options.get("audio_engine", null)
	if audio_engine != null and should_play and bool(applied.get("should_fade_music", false)):
		var fade_frames := int(applied.get("music_fade_frames", 0))
		var fade_ms := int(applied.get("music_fade_ms", 0))
		if audio_engine.has_method("fade_out_music_frames") and fade_frames > 0:
			audio_engine.call("fade_out_music_frames", fade_frames)
		elif audio_engine.has_method("fadeOutMusicFrames") and fade_frames > 0:
			audio_engine.call("fadeOutMusicFrames", fade_frames)
		elif audio_engine.has_method("fade_out_music") and fade_ms > 0:
			audio_engine.call("fade_out_music", fade_ms)
		elif audio_engine.has_method("fadeOutMusic") and fade_ms > 0:
			audio_engine.call("fadeOutMusic", fade_ms)
	if typeof(scheduler_state) == TYPE_DICTIONARY:
		applied["scheduler"] = schedule_result.duplicate(true)
	return applied

func validate_audio_cue(cue: Dictionary) -> Dictionary:
	var absolute_path := str(cue.get("absolute_path", ""))
	var exists := not absolute_path.is_empty() and FileAccess.file_exists(absolute_path)
	var byte_size := int(cue.get("byte_size", 0))
	if exists and byte_size <= 0:
		byte_size = _file_size(absolute_path)
	var ok := exists and byte_size > 0
	return {
		"ok": ok,
		"category": str(cue.get("category", "")),
		"token": str(cue.get("token", "")),
		"key": str(cue.get("key", "")),
		"relative_path": str(cue.get("relative_path", "")),
		"absolute_path": absolute_path,
		"exists": exists,
		"byte_size": byte_size,
		"priority_class": str(cue.get("priority_class", "none")),
		"priority_sound": bool(cue.get("priority_sound", false)),
		"fade_music": bool(cue.get("fade_music", false)),
		"loop": bool(cue.get("loop", false)),
		"manifest_path": str(cue.get("manifest_path", "")),
	}

func validate_audio_playback_plan(plan: Dictionary) -> Dictionary:
	var cue := Dictionary(plan.get("cue", {}))
	var validation := validate_audio_cue(cue)
	return {
		"ok": bool(validation.get("ok", false)) or not bool(plan.get("ok", false)),
		"validation": validation,
		"plan": plan.duplicate(true),
		"applied": bool(plan.get("applied", false)),
		"played": bool(plan.get("played", false)),
		"priority_class": str(plan.get("priority_class", "none")),
		"priority_sound": bool(plan.get("priority_sound", false)),
		"fade_music": bool(plan.get("fade_music", false)),
		"should_fade_music": bool(plan.get("should_fade_music", false)),
		"should_mute_music": bool(plan.get("should_mute_music", false)),
	}

func _object_has_property(object: Object, property_name: String) -> bool:
	if object == null or property_name.is_empty():
		return false
	for property in object.get_property_list():
		if String(property.get("name", "")) == property_name:
			return true
	return false

func _ensure_audio_playback_state(state: Dictionary) -> Dictionary:
	if state.is_empty():
		return create_audio_playback_state()
	if not state.has("active_channels"):
		state["active_channels"] = []
	if not state.has("recent_events"):
		state["recent_events"] = []
	if not state.has("pending_sounds"):
		state["pending_sounds"] = []
	if not state.has("channel_owners"):
		state["channel_owners"] = {}
	if not state.has("suppressed_music_channels"):
		state["suppressed_music_channels"] = []
	if not state.has("priority_mute_count"):
		state["priority_mute_count"] = 0
	if not state.has("music_muted_by_priority"):
		state["music_muted_by_priority"] = false
	if not state.has("music_muted_by_controller"):
		state["music_muted_by_controller"] = false
	if not state.has("event_sequence"):
		state["event_sequence"] = 0
	return state

func _register_audio_event(state: Dictionary, event: Dictionary) -> void:
	var sequence := int(state.get("event_sequence", 0)) + 1
	state["event_sequence"] = sequence
	var recent := Array(state.get("recent_events", []))
	var snapshot := event.duplicate(true)
	snapshot["sequence"] = sequence
	recent.append(snapshot)
	if recent.size() > 32:
		var sliced := []
		for index in range(recent.size() - 32, recent.size()):
			sliced.append(recent[index])
		recent = sliced
	state["recent_events"] = recent

func _playback_priority_for_plan(plan: Dictionary) -> Variant:
	var priority_class := str(plan.get("priority_class", "none"))
	if priority_class == "cry":
		return 0
	if priority_class == "priority":
		return 1
	var token := _normalize_priority_token(str(plan.get("token", "")))
	if token.is_empty():
		return null
	var aliases := load_disassembly_aliases()
	var priorities: Dictionary = Dictionary(aliases.get("sfxPriority", {}))
	return priorities.get(token, null)

func _register_active_channel(state: Dictionary, channel: int, token: String, category: String, priority_class: String, priority: Variant) -> void:
	var active_channels := Array(state.get("active_channels", []))
	active_channels.append({
		"channel": channel,
		"ownerToken": token,
		"category": category,
		"role": str(state.get("music_role", "general")),
		"priority_class": priority_class,
		"priority": priority,
	})
	state["active_channels"] = active_channels
	var channel_owners: Dictionary = Dictionary(state.get("channel_owners", {}))
	channel_owners[str(channel)] = token
	state["channel_owners"] = channel_owners

func _release_owned_audio_channels(state: Dictionary, token: String) -> Array:
	if token.is_empty():
		return []
	var active_channels := Array(state.get("active_channels", []))
	var next_channels: Array = []
	var removed: Array = []
	for entry in active_channels:
		if typeof(entry) != TYPE_DICTIONARY:
			continue
		var channel_entry: Dictionary = Dictionary(entry)
		if str(channel_entry.get("ownerToken", "")) == token:
			removed.append(channel_entry.duplicate(true))
			continue
		next_channels.append(channel_entry.duplicate(true))
	state["active_channels"] = next_channels
	var channel_owners: Dictionary = Dictionary(state.get("channel_owners", {}))
	for channel in channel_owners.keys():
		if str(channel_owners[channel]) == token:
			channel_owners.erase(channel)
	state["channel_owners"] = channel_owners
	return removed

func _release_sound_channels(state: Dictionary) -> Array:
	var active_channels := Array(state.get("active_channels", []))
	var next_channels: Array = []
	var removed: Array = []
	for entry in active_channels:
		if typeof(entry) != TYPE_DICTIONARY:
			continue
		var channel_entry: Dictionary = Dictionary(entry)
		var category := str(channel_entry.get("category", ""))
		if category == "sfx" or category == "cry":
			removed.append(channel_entry.duplicate(true))
			continue
		next_channels.append(channel_entry.duplicate(true))
	state["active_channels"] = next_channels
	var channel_owners: Dictionary = Dictionary(state.get("channel_owners", {}))
	for channel in channel_owners.keys():
		var owner := str(channel_owners[channel])
		for entry in removed:
			if typeof(entry) == TYPE_DICTIONARY and str(Dictionary(entry).get("ownerToken", "")) == owner:
				channel_owners.erase(channel)
				break
	state["channel_owners"] = channel_owners
	_refresh_suppressed_music_channels(state)
	return removed

func _refresh_suppressed_music_channels(state: Dictionary) -> void:
	var suppressed: Array = []
	for entry in Array(state.get("active_channels", [])):
		if typeof(entry) != TYPE_DICTIONARY:
			continue
		var channel_entry: Dictionary = Dictionary(entry)
		var category := str(channel_entry.get("category", ""))
		if category != "sfx" and category != "cry":
			continue
		var channel := int(channel_entry.get("channel", 0))
		if channel >= 5 and channel <= 8:
			suppressed.append(channel - 4)
		elif channel >= 1 and channel <= 4:
			suppressed.append(channel)
	state["suppressed_music_channels"] = suppressed
	state["music_muted_by_priority"] = int(state.get("priority_mute_count", 0)) > 0

func _snapshot_active_channels(state: Dictionary) -> Array:
	return Array(state.get("active_channels", [])).duplicate(true)

func _default_owned_channels_for_category(category: String) -> Array:
	match category:
		"cry":
			return [5, 6, 8]
		"sfx":
			return [5, 6]
		_:
			return []

func validate_canonical_audio_assets() -> Dictionary:
	var cues: Array[Dictionary] = [
		resolve_music_cue("MUSIC_NEW_BARK_TOWN"),
		resolve_music_cue("MUSIC_ROUTE_29"),
		resolve_sfx_cue("SFX_READ_TEXT_2"),
		resolve_cry_cue("BULBASAUR"),
	]
	var results: Array[Dictionary] = []
	var missing: Array[String] = []
	for cue in cues:
		var result := validate_audio_cue(cue)
		results.append(result)
		if not bool(result.get("ok", false)):
			missing.append(str(result.get("relative_path", "")))
	return {
		"ok": missing.is_empty(),
		"checked_count": results.size(),
		"missing_count": missing.size(),
		"missing": missing,
		"results": results,
		"audio_roots": Array(audio_root_candidates()),
		"manifest_paths": Array(audio_manifest_paths()),
	}

func cue_metadata_for_manifest_entry(category: String, key: String, value: Variant, manifest_path: String = "") -> Dictionary:
	var token := key
	var relative_path := ""
	var manifest_metadata: Dictionary = {}
	if typeof(value) == TYPE_STRING:
		relative_path = str(value)
	elif typeof(value) == TYPE_DICTIONARY:
		var entry := Dictionary(value)
		token = str(entry.get("id", entry.get("token", key)))
		relative_path = str(entry.get("path", entry.get("source", entry.get("file", entry.get("assetPath", entry.get("mixedPath", ""))))))
		manifest_metadata = _extract_manifest_metadata(entry)
	if relative_path.is_empty():
		var cue := resolve_audio_cue(category, token)
		if not manifest_metadata.is_empty():
			cue.merge(manifest_metadata, true)
		if not manifest_path.is_empty():
			cue["manifest_path"] = manifest_path
		return cue
	if not _is_safe_relative_audio_path(relative_path):
		return _empty_cue(category, token)
	var cue := _build_cue_metadata(category, token, _slugify(key), relative_path, manifest_metadata)
	cue["manifest_path"] = manifest_path
	return cue

func _build_cue_metadata(category: String, token: String, key: String, relative_path: String, extra: Dictionary = {}) -> Dictionary:
	if key.is_empty() or not _is_safe_relative_audio_path(relative_path):
		return _empty_cue(category, token)
	var absolute_path := _resolve_audio_path(relative_path)
	var exists := not absolute_path.is_empty() and FileAccess.file_exists(absolute_path)
	var byte_size := _file_size(absolute_path) if exists else 0
	var cue := {
		"category": category,
		"token": token,
		"key": key,
		"file_name": relative_path.get_file(),
		"relative_path": relative_path,
		"absolute_path": absolute_path,
		"exists": exists,
		"byte_size": byte_size,
		"format": "mp3",
		"loop": false,
		"priority_class": "none",
		"priority_sound": false,
		"fade_music": false,
	}
	if typeof(extra) == TYPE_DICTIONARY:
		for extra_key in Dictionary(extra).keys():
			cue[extra_key] = extra[extra_key]
	return cue

func _empty_cue(category: String, token: String) -> Dictionary:
	return {
		"category": category,
		"token": token,
		"key": "",
		"file_name": "",
		"relative_path": "",
		"absolute_path": "",
		"exists": false,
		"byte_size": 0,
		"format": "",
		"loop": false,
		"priority_class": "none",
		"priority_sound": false,
		"fade_music": false,
	}

func _resolve_audio_path(relative_path: String) -> String:
	if not _is_safe_relative_audio_path(relative_path):
		return ""
	for root in audio_root_candidates():
		var candidate := root.path_join(relative_path)
		if FileAccess.file_exists(candidate):
			return candidate
	var roots := audio_root_candidates()
	if roots.is_empty():
		return ""
	return roots[0].path_join(relative_path)

func _resolve_cry_base(species: String) -> String:
	var cries := load_pokemon_cries()
	var entry_value: Variant = cries.get(species, null)
	if typeof(entry_value) == TYPE_DICTIONARY:
		var cry := str(Dictionary(entry_value).get("cry", ""))
		if not cry.is_empty():
			return _slugify(cry.replace("CRY_", ""))
	return _slugify(species)

func _extract_manifest_metadata(entry: Dictionary) -> Dictionary:
	var metadata: Dictionary = {}
	var priority_class := str(entry.get("priorityClass", entry.get("priority_class", "none")))
	if not priority_class.is_empty():
		metadata["priority_class"] = priority_class
		metadata["priority_sound"] = priority_class == "priority" or priority_class == "cry"
		metadata["fade_music"] = metadata["priority_sound"]
	if entry.has("loop"):
		metadata["loop"] = bool(entry.get("loop", false))
	if entry.has("durationFrames"):
		metadata["duration_frames"] = int(entry.get("durationFrames", 0))
	if entry.has("loopStartFrame"):
		metadata["loop_start_frame"] = int(entry.get("loopStartFrame", 0))
	if entry.has("loopEndFrame"):
		metadata["loop_end_frame"] = int(entry.get("loopEndFrame", 0))
	if entry.has("loopStartSeconds"):
		metadata["loop_start_seconds"] = float(entry.get("loopStartSeconds", 0.0))
	if entry.has("loopEndSeconds"):
		metadata["loop_end_seconds"] = float(entry.get("loopEndSeconds", 0.0))
	if entry.has("ownedChannels"):
		metadata["owned_channels"] = Array(entry.get("ownedChannels", []))
	if entry.has("path"):
		metadata["source_path"] = str(entry.get("path", ""))
	if entry.has("mixedPath"):
		metadata["mixed_path"] = str(entry.get("mixedPath", ""))
	return metadata

func _count_audio_manifest_entries(value: Variant) -> int:
	if typeof(value) != TYPE_DICTIONARY:
		return 0
	var manifest := Dictionary(value)
	var total := 0
	for key in ["music", "sounds"]:
		var bucket: Variant = manifest.get(key, {})
		if typeof(bucket) == TYPE_DICTIONARY:
			total += Dictionary(bucket).size()
	return total

func _load_disassembly_aliases() -> Dictionary:
	var root := REPO_PATHS_SCRIPT.repo_root()
	if root.is_empty():
		return _load_static_aliases()
	var disassembly_root := root.path_join("vendor/pokecrystal")
	var music_constants := _read_text_file(disassembly_root.path_join("constants/music_constants.asm"))
	var sfx_constants := _read_text_file(disassembly_root.path_join("constants/sfx_constants.asm"))
	var music_pointers := _read_text_file(disassembly_root.path_join("audio/music_pointers.asm"))
	var sfx_pointers := _read_text_file(disassembly_root.path_join("audio/sfx_pointers.asm"))
	if music_constants.is_empty() or sfx_constants.is_empty() or music_pointers.is_empty() or sfx_pointers.is_empty():
		return _load_static_aliases()
	var music_constants_list := _parse_disassembly_constants(music_constants, "MUSIC_")
	var sfx_constants_list := _parse_disassembly_constants(sfx_constants, "SFX_")
	var music_pointer_list := _parse_pointer_labels(music_pointers, "Music_")
	var sfx_pointer_list := _parse_pointer_labels(sfx_pointers, "Sfx_")
	if music_constants_list.size() != music_pointer_list.size() or sfx_constants_list.size() != sfx_pointer_list.size():
		return _load_static_aliases()
	var music: Dictionary = {}
	var sfx: Dictionary = {}
	for index in range(music_constants_list.size()):
		music[music_constants_list[index]] = _normalize_music_pointer_label(music_pointer_list[index])
	for index in range(sfx_constants_list.size()):
		sfx[sfx_constants_list[index]] = "sfx/%s" % _normalize_sfx_pointer_label(sfx_pointer_list[index])
	return {
		"music": music,
		"sfx": sfx,
		"sfxPriority": _build_priority_map(sfx_constants_list),
	}

func _load_static_aliases() -> Dictionary:
	var sfx: Dictionary = {}
	for key in MENU_SOUND_ALIASES.values():
		if str(key).to_upper().begins_with("SFX_"):
			sfx[str(key)] = "sfx/%s" % _slugify(str(key).substr(4))
	return {
		"music": {},
		"sfx": sfx,
		"sfxPriority": _build_priority_map(Array(sfx.keys())),
	}

func _parse_disassembly_constants(contents: String, prefix: String) -> Array[String]:
	var entries: Array[String] = []
	for raw_line in contents.split("\n"):
		var line := _strip_comment(raw_line)
		if line.is_empty():
			continue
		var tokens := _split_tokens(line)
		if tokens.size() < 2:
			continue
		if String(tokens[0]) != "const":
			continue
		var name := String(tokens[1])
		if name.begins_with(prefix):
			entries.append(name)
	return entries

func _parse_pointer_labels(contents: String, prefix: String) -> Array[String]:
	var entries: Array[String] = []
	for raw_line in contents.split("\n"):
		var line := _strip_comment(raw_line)
		if line.is_empty():
			continue
		var tokens := _split_tokens(line)
		if tokens.size() < 2:
			continue
		if String(tokens[0]) != "dba":
			continue
		var name := String(tokens[1])
		if name.begins_with(prefix):
			entries.append(name)
	return entries

func _build_priority_map(constants: Array[String]) -> Dictionary:
	var priorities: Dictionary = {}
	for index in range(constants.size()):
		priorities[constants[index]] = index
	return priorities

func _normalize_manifest_key(value: String) -> String:
	var normalized := ""
	for index in range(value.length()):
		var ch := value.substr(index, 1)
		if ch.is_valid_int() or (ch >= "A" and ch <= "Z") or (ch >= "a" and ch <= "z"):
			normalized += ch.to_lower()
	return normalized

func _normalize_music_pointer_label(label: String) -> String:
	var suffix := label
	if suffix.to_lower().begins_with("music_"):
		suffix = suffix.substr(6)
	return _normalize_manifest_key(suffix)

func _normalize_sfx_pointer_label(label: String) -> String:
	var suffix := label
	if suffix.to_lower().begins_with("sfx_"):
		suffix = suffix.substr(4)
	return _normalize_manifest_key(suffix)

func _strip_comment(line: String) -> String:
	return line.split(";", true, 1)[0].strip_edges()

func _split_tokens(line: String) -> Array[String]:
	var cleaned := line.replace("\t", " ")
	var tokens: Array[String] = []
	for token in cleaned.split(" ", false):
		var trimmed := str(token).strip_edges()
		if not trimmed.is_empty():
			tokens.append(trimmed)
	return tokens

func _is_priority_sound_token(token: String) -> bool:
	var normalized := _normalize_priority_token(token)
	if normalized.is_empty():
		return false
	if normalized.begins_with("CRY_") or normalized.ends_with("_CRY"):
		return true
	if not normalized.begins_with("SFX_"):
		return false
	if PRIORITY_SFX_TOKENS.has(normalized):
		return true
	for prefix in PRIORITY_SFX_PREFIXES:
		if normalized.begins_with(prefix):
			return true
	return false

func _normalize_priority_token(token: String) -> String:
	var trimmed := token.strip_edges()
	if trimmed.is_empty():
		return ""
	var alias: Variant = MENU_SOUND_ALIASES.get(trimmed.to_lower(), trimmed)
	return str(alias).strip_edges().to_upper()

func _normalize_token(token: String) -> String:
	var trimmed := token.strip_edges()
	if trimmed.is_empty() or trimmed.is_absolute_path() or trimmed.contains("/") or trimmed.contains("\\") or trimmed.contains(".."):
		return ""
	return trimmed

func _slugify(value: String) -> String:
	var slug := ""
	for index in range(value.length()):
		var ch := value.substr(index, 1)
		if ch.is_valid_int() or (ch >= "A" and ch <= "Z") or (ch >= "a" and ch <= "z"):
			slug += ch.to_lower()
	return slug

func _is_safe_relative_audio_path(relative_path: String) -> bool:
	if relative_path.is_empty() or relative_path.is_absolute_path() or relative_path.contains("\\") or relative_path.contains(".."):
		return false
	if not relative_path.to_lower().ends_with(MP3_EXTENSION):
		return false
	var parts := relative_path.split("/", false)
	if parts.is_empty() or parts.size() > 2:
		return false
	if parts.size() == 2 and not ["sfx", "cries"].has(str(parts[0])):
		return false
	for part in parts:
		if str(part).strip_edges().is_empty():
			return false
	return true

func _append_unique_path(paths: PackedStringArray, value: String) -> void:
	if value.is_empty():
		return
	for path in paths:
		if path == value:
			return
	paths.append(value)

func _collect_audio_manifest_paths(root: String, paths: PackedStringArray) -> void:
	if root.is_empty():
		return
	var dir := DirAccess.open(root)
	if dir == null:
		return
	dir.list_dir_begin()
	while true:
		var entry := dir.get_next()
		if entry.is_empty():
			break
		if entry.begins_with("."):
			continue
		var entry_path := root.path_join(entry)
		if dir.current_is_dir():
			_collect_audio_manifest_paths(entry_path, paths)
		elif entry == AUDIO_ASSETS_MANIFEST:
			paths.append(entry_path)
	dir.list_dir_end()

func _read_text_file(absolute_path: String) -> String:
	if absolute_path.is_empty() or not FileAccess.file_exists(absolute_path):
		return ""
	var file := FileAccess.open(absolute_path, FileAccess.READ)
	if file == null:
		return ""
	var text := file.get_as_text()
	file = null
	return text

func _load_json_data_path(relative_path: String) -> Variant:
	_ensure_initialized()
	if relative_path.is_empty() or relative_path.is_absolute_path() or relative_path.contains(".."):
		return null
	return _load_json_path(data_root.path_join(relative_path))

func _load_json_path(absolute_path: String) -> Variant:
	if absolute_path.is_empty():
		return null
	if _json_cache.has(absolute_path):
		return _json_cache[absolute_path]
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
	_json_cache[absolute_path] = json.data
	return json.data

func _file_size(absolute_path: String) -> int:
	if absolute_path.is_empty() or not FileAccess.file_exists(absolute_path):
		return 0
	var file := FileAccess.open(absolute_path, FileAccess.READ)
	if file == null:
		return 0
	var size := int(file.get_length())
	file = null
	return size

func _ensure_initialized() -> void:
	if not _initialized:
		initialize()
