extends RefCounted
class_name ParityCoverageInventory

const REPO_PATHS_SCRIPT := preload("res://scripts/repo_paths.gd")

const HARNESS_SCRIPTS := [
	"smoke_test.gd",
	"parity_journey_test.gd",
	"parity_audit_test.gd",
]

const DOMAIN_DEFINITIONS := [
	{
		"id": "coordinator",
		"required": true,
		"ts_paths": ["core/state.ts", "core/guest-session-storage.ts"],
		"godot_scripts": ["game_state.gd", "game_runtime.gd", "save_store.gd"],
		"audit_assertions": [
			"boot_title_scene_route_round_trip",
			"boot_title_handoff_round_trip",
			"boot_title_pending_handoff_round_trip",
			"save_load_snapshot_identity",
			"save_load_metadata_identity",
		],
	},
	{
		"id": "boot_title",
		"required": true,
		"ts_paths": ["engine/home.ts", "engine/index.ts", "ui/surface.ts"],
		"godot_scripts": [
			"boot_scene_base.gd",
			"title_runtime.gd",
			"title_screen.gd",
			"intro_runtime.gd",
			"intro_sequence.gd",
			"oak_intro.gd",
			"continue_screen.gd",
			"clock_reset_screen.gd",
			"day_of_week_screen.gd",
			"delete_save_screen.gd",
			"name_entry.gd",
		],
		"audit_assertions": [
			"boot_title_active_scene",
			"boot_title_ui_page",
			"boot_title_scene_route",
			"boot_title_scene_route_round_trip",
			"boot_title_handoff_round_trip",
			"boot_title_pending_handoff_round_trip",
		],
	},
	{
		"id": "menus",
		"required": true,
		"ts_paths": ["ui/player-backpics.ts", "ui/surface.ts"],
		"godot_scripts": ["menu_state.gd", "menu_stack.gd", "ui_shell.gd"],
		"audit_assertions": ["ui_shell_round_trip", "ui_menu_round_trip", "menu_packet_pressed_start"],
	},
	{
		"id": "render",
		"required": true,
		"ts_paths": [
			"ui/screens/title-screen.test.ts",
			"ui/screens/intro/intro-sequence.ts",
			"engine/world/overworld/overworld-rendering.ts",
			"ui/text-overlays.ts",
			"ui/overlays/battle-ui-render.ts",
			"ui/overlays/battle-ui-draw.ts",
			"ui/menus/pokedex-render.ts",
			"ui/menus/pc-fidelity.test.ts",
		],
		"godot_scripts": ["render_snapshot_state.gd"],
		"audit_assertions": [
			"render_title_frame_round_trip",
			"render_intro_frame_round_trip",
			"render_overworld_frame_round_trip",
			"render_menu_frame_round_trip",
			"render_battle_frame_round_trip",
		],
	},
	{
		"id": "special_events",
		"required": true,
		"ts_paths": ["engine/world/special-events/"],
		"godot_scripts": ["special_events_state.gd"],
		"audit_assertions": [
			"special_events_state_round_trip",
			"special_events_day_care_payloads",
			"special_events_pc_helper_entries",
		],
	},
	{
		"id": "text",
		"required": true,
		"ts_paths": ["core/asm-text-loader.ts", "ui/text-ui.ts", "ui/text-snapshot-render.test.ts"],
		"godot_scripts": ["text_box.gd", "battle_dialogue.gd"],
		"audit_assertions": ["ui_dialogue_round_trip"],
	},
	{
		"id": "battle",
		"required": true,
		"ts_paths": [
			"engine/battle/",
			"engine/world/special_events/battle-tower.ts",
			"engine/world/special_events/battle-tower-loader.test.ts",
		],
		"godot_scripts": [
			"battle_state.gd",
			"battle_runtime.gd",
			"battle_assets.gd",
			"battle_ui.gd",
			"battle_ui_input.gd",
			"battle_ui_render.gd",
			"battle_ui_state.gd",
		],
		"audit_assertions": [
			"battle_resolution_valid",
			"battle_phase_history_contains_turn_prompt",
			"battle_resolution_events_consumed",
			"battle_state_revision_advanced",
		],
	},
		{
			"id": "overworld",
			"required": true,
			"ts_paths": ["engine/world/", "types/overworld.ts"],
			"godot_scripts": ["map_data.gd", "overworld_state.gd", "overworld_runtime.gd", "story_events_state.gd"],
			"audit_assertions": [
				"map_selected_key_present",
				"map_manifest_contains_selected_map",
				"map_round_trip_selected_key",
				"story_events_queue_round_trip",
			],
		},
		{
			"id": "story_events",
			"required": true,
			"ts_paths": ["engine/world/story-events/"],
			"godot_scripts": ["story_events_state.gd"],
			"audit_assertions": [
				"story_events_flag_set",
				"story_events_text_wait",
				"story_events_warp_payload",
				"story_events_battle_payload",
				"story_events_round_trip",
			],
		},
	{
		"id": "audio",
		"required": true,
		"ts_paths": [
			"audio-export/",
			"engine/systems/audio.ts",
			"engine/world/map-music.ts",
			"engine/world/radio.ts",
			"engine/world/radio-music.ts",
			"engine/world/special_events/audio.ts",
		],
		"godot_scripts": ["audio_assets.gd"],
		"audit_assertions": [],
	},
	{
		"id": "assets",
		"required": true,
		"ts_paths": ["core/base-data.ts", "core/content-packs.test.ts", "core/lz.ts", "core/map-blocks.runtime-fallback.test.ts", "core/cry-data.ts", "core/gbc-colors.ts"],
		"godot_scripts": ["asset_index.gd", "gb_tile_decoder.gd", "exported_asset_manifest.gd"],
		"audit_assertions": [
			"map_default_load",
			"map_block_key_in_map_blocks",
			"save_load_snapshot_identity",
		],
	},
		{
			"id": "input",
			"required": true,
			"ts_paths": ["input/"],
			"godot_scripts": ["input_latch.gd"],
			"audit_assertions": ["movement_packet_round_trip", "menu_packet_pressed_start"],
		},
	{
		"id": "core_systems",
		"required": true,
		"ts_paths": ["engine/systems/", "engine/games/"],
		"godot_scripts": ["core_systems_state.gd"],
		"audit_assertions": [
			"core_systems_step_increment",
			"core_systems_poison_tick",
			"core_systems_daily_reset",
			"core_systems_shop_buy",
			"core_systems_round_trip",
		],
	},
	{
		"id": "game_corner",
		"required": true,
		"ts_paths": ["engine/games/"],
		"godot_scripts": ["game_corner_state.gd"],
		"audit_assertions": [
			"game_corner_rng_round_trip",
			"game_corner_slots_deterministic",
			"game_corner_card_flip_round_trip",
			"game_corner_memory_round_trip",
			"game_corner_unown_round_trip",
		],
	},
	{
		"id": "save",
		"required": true,
	"ts_paths": ["core/state.ts", "adapters/cloud-save.ts", "core/guest-session-storage.ts"],
		"godot_scripts": ["save_store.gd"],
		"audit_assertions": ["save_metadata_present", "save_load_snapshot_identity", "save_load_metadata_identity"],
	},
]

func generate_report() -> Dictionary:
	var repo_root := str(REPO_PATHS_SCRIPT.repo_root())
	var ts_root := repo_root.path_join("packages/core/src")
	var godot_root := repo_root.path_join("apps/godot/scripts")

	var ts_files := _collect_files(ts_root, ".ts")
	var godot_files := _collect_files(godot_root, ".gd")
	var indexed_godot_files := {}
	for godot_file in godot_files:
		indexed_godot_files[godot_file] = true

	var domains: Array[Dictionary] = []
	var required_gaps: Array[Dictionary] = []
	var domain_metrics: Dictionary = {}
	var unmapped_required_by_subdomain: Array[Dictionary] = []
	var harness_report := _build_harness_report(indexed_godot_files)

	for definition in DOMAIN_DEFINITIONS:
		var domain_id := str(definition.get("id", ""))
		var domain_ts_files := _collect_domain_files(ts_files, definition)
		var godot_scripts: Array = Array(definition.get("godot_scripts", [])).duplicate(true)
		var matched_godot_scripts: Array = []
		var missing_godot_scripts: Array = []
		for godot_script in godot_scripts:
			if indexed_godot_files.has(godot_script):
				matched_godot_scripts.append(godot_script)
			else:
				missing_godot_scripts.append(godot_script)
		var covered := missing_godot_scripts.is_empty()
		var domain_entry := {
			"id": domain_id,
			"required": bool(definition.get("required", false)),
			"ts_count": domain_ts_files.size(),
			"godot_count": godot_scripts.size(),
			"covered_godot_count": matched_godot_scripts.size(),
			"missing_godot_count": missing_godot_scripts.size(),
			"ts_files": domain_ts_files,
			"godot_scripts": godot_scripts,
			"covered_godot_scripts": matched_godot_scripts,
			"missing_godot_scripts": missing_godot_scripts,
			"covered": covered,
			"audit_assertions": Array(definition.get("audit_assertions", [])).duplicate(true),
		}
		domains.append(domain_entry)
		domain_metrics[domain_id] = {
			"ts_count": domain_ts_files.size(),
			"godot_count": godot_scripts.size(),
			"missing_godot_count": missing_godot_scripts.size(),
			"covered": covered,
		}
		if bool(definition.get("required", false)) and not covered:
			var gap_entry := {
				"id": domain_id,
				"detail": "required Godot coverage missing for game domain %s" % domain_id,
				"subdomain": domain_id,
				"ts_files": domain_ts_files,
				"missing_godot_scripts": missing_godot_scripts,
				"audit_assertions": Array(definition.get("audit_assertions", [])).duplicate(true),
			}
			required_gaps.append(gap_entry)
			unmapped_required_by_subdomain.append({
				"subdomain": domain_id,
				"ts_count": domain_ts_files.size(),
				"ts_files": domain_ts_files,
				"godot_scripts": godot_scripts,
				"missing_godot_scripts": missing_godot_scripts,
			})

	return {
		"domains": domains,
		"harness_scripts": harness_report,
		"gaps": required_gaps,
		"required_gaps": required_gaps,
		"unmapped_required_by_subdomain": unmapped_required_by_subdomain,
		"domain_metrics": domain_metrics,
		"summary": _build_summary(ts_files, godot_files, domain_metrics, harness_report, required_gaps.size(), unmapped_required_by_subdomain.size()),
	}

func _build_harness_report(indexed_godot_files: Dictionary) -> Dictionary:
	var present: Array[String] = []
	var missing: Array[String] = []
	for script_name in HARNESS_SCRIPTS:
		if indexed_godot_files.has(script_name):
			present.append(script_name)
		else:
			missing.append(script_name)
	return {
		"required": HARNESS_SCRIPTS.duplicate(true),
		"present": present,
		"missing": missing,
		"present_count": present.size(),
		"missing_count": missing.size(),
		"covered": missing.is_empty(),
	}

func _build_summary(ts_files: Array[String], godot_files: Array[String], domain_metrics: Dictionary, harness_report: Dictionary, required_gap_count: int, unmapped_required_count: int) -> Dictionary:
	var domain_counts := {
		"battle": domain_metrics.get("battle", {}),
		"overworld": domain_metrics.get("overworld", {}),
		"menus": domain_metrics.get("menus", {}),
		"render": domain_metrics.get("render", {}),
		"text": domain_metrics.get("text", {}),
		"game_corner": domain_metrics.get("game_corner", {}),
	}
	return {
		"ts_files_scanned": ts_files.size(),
		"godot_files_scanned": godot_files.size(),
		"required_gap_count": required_gap_count,
		"unmapped_required_subdomain_count": unmapped_required_count,
		"domain_count": DOMAIN_DEFINITIONS.size(),
		"covered_domain_count": _count_covered_domains(domain_metrics),
		"harness_script_count": HARNESS_SCRIPTS.size(),
		"harness_script_missing_count": int(harness_report.get("missing_count", 0)),
		"battle_ts_count": int(Dictionary(domain_counts.get("battle", {})).get("ts_count", 0)),
		"battle_godot_count": int(Dictionary(domain_counts.get("battle", {})).get("godot_count", 0)),
		"overworld_ts_count": int(Dictionary(domain_counts.get("overworld", {})).get("ts_count", 0)),
		"overworld_godot_count": int(Dictionary(domain_counts.get("overworld", {})).get("godot_count", 0)),
		"menu_ts_count": int(Dictionary(domain_counts.get("menus", {})).get("ts_count", 0)),
		"menu_godot_count": int(Dictionary(domain_counts.get("menus", {})).get("godot_count", 0)),
		"render_ts_count": int(Dictionary(domain_counts.get("render", {})).get("ts_count", 0)),
		"render_godot_count": int(Dictionary(domain_counts.get("render", {})).get("godot_count", 0)),
		"text_ts_count": int(Dictionary(domain_counts.get("text", {})).get("ts_count", 0)),
		"text_godot_count": int(Dictionary(domain_counts.get("text", {})).get("godot_count", 0)),
		"game_corner_ts_count": int(Dictionary(domain_counts.get("game_corner", {})).get("ts_count", 0)),
		"game_corner_godot_count": int(Dictionary(domain_counts.get("game_corner", {})).get("godot_count", 0)),
		"battle_counts": domain_counts.get("battle", {}),
		"overworld_counts": domain_counts.get("overworld", {}),
		"menu_counts": domain_counts.get("menus", {}),
		"render_counts": domain_counts.get("render", {}),
		"text_counts": domain_counts.get("text", {}),
		"game_corner_counts": domain_counts.get("game_corner", {}),
	}

func _count_covered_domains(domain_metrics: Dictionary) -> int:
	var count := 0
	for domain_id in domain_metrics.keys():
		var metrics: Dictionary = Dictionary(domain_metrics.get(domain_id, {}))
		if bool(metrics.get("covered", false)):
			count += 1
	return count

func _collect_domain_files(ts_files: Array[String], definition: Dictionary) -> Array[String]:
	var collected: Array[String] = []
	for ts_file in ts_files:
		if _matches_domain(ts_file, definition):
			collected.append(ts_file)
	return collected

func _matches_domain(ts_file: String, definition: Dictionary) -> bool:
	for pattern in Array(definition.get("ts_paths", [])):
		var path_pattern := str(pattern)
		if path_pattern.is_empty():
			continue
		if path_pattern.ends_with("/"):
			if ts_file.begins_with(path_pattern):
				return true
			continue
		if path_pattern.ends_with(".ts"):
			if ts_file == path_pattern:
				return true
			continue
		if ts_file.begins_with(path_pattern):
			return true
	return false

func _collect_files(root: String, suffix: String) -> Array[String]:
	var collected: Array[String] = []
	_collect_files_recursive(root, root, suffix, collected)
	collected.sort()
	return collected

func _collect_files_recursive(root: String, current: String, suffix: String, collected: Array[String]) -> void:
	var directory := DirAccess.open(current)
	if directory == null:
		return
	directory.list_dir_begin()
	while true:
		var entry := directory.get_next()
		if entry.is_empty():
			break
		if entry == "." or entry == "..":
			continue
		var absolute_path := current.path_join(entry)
		if directory.current_is_dir():
			_collect_files_recursive(root, absolute_path, suffix, collected)
			continue
		if not absolute_path.ends_with(suffix):
			continue
		collected.append(_relative_path(root, absolute_path))
	directory.list_dir_end()

func _relative_path(root: String, absolute_path: String) -> String:
	var prefix := root.trim_suffix("/") + "/"
	if absolute_path.begins_with(prefix):
		return absolute_path.substr(prefix.length())
	return absolute_path
