extends Control
class_name BattleUI

const BATTLE_UI_RENDER_SCRIPT := preload("res://scripts/battle_ui_render.gd")
const BATTLE_UI_STATE_SCRIPT := preload("res://scripts/battle_ui_state.gd")
const BATTLE_UI_INPUT_SCRIPT := preload("res://scripts/battle_ui_input.gd")

var _renderer = null
var _ui_state = null
var _input = null
var _runtime = null
var _last_signature := ""
var _title_label: Label
var _battle_label: Label
var _phase_label: Label
var _turn_label: Label
var _prompt_label: Label
var _queue_label: Label
var _event_label: Label
var _flow_label: Label
var _history_label: Label
var _assets_label: Label
var _debug_label: Label

func _ready() -> void:
	_renderer = BATTLE_UI_RENDER_SCRIPT.new()
	_ui_state = BATTLE_UI_STATE_SCRIPT.new()
	_input = BATTLE_UI_INPUT_SCRIPT.new()
	_bind_labels()

func bind_runtime(runtime) -> void:
	_runtime = runtime
	if runtime == null:
		return
	if runtime.has_method("get") and runtime.get("battle_ui_state") != null:
		_ui_state = runtime.get("battle_ui_state")
	elif _ui_state == null:
		_ui_state = BATTLE_UI_STATE_SCRIPT.new()
	if _ui_state != null:
		_ui_state.bind_battle_state(runtime.battle_state)

func refresh_display(runtime, force: bool = false) -> void:
	if runtime == null or runtime.battle_state == null:
		return
	if _renderer == null:
		_renderer = BATTLE_UI_RENDER_SCRIPT.new()
	if _ui_state == null:
		_ui_state = BATTLE_UI_STATE_SCRIPT.new()
	if _runtime != runtime:
		bind_runtime(runtime)
	_ui_state.sync_from_battle_state()
	var signature: String = str(_renderer.build_signature(runtime))
	if not force and signature == _last_signature:
		return
	_last_signature = signature
	var lines: Dictionary = _renderer.build_lines(runtime)
	if is_instance_valid(_title_label):
		_title_label.text = str(lines.get("title", "BATTLE"))
	if is_instance_valid(_battle_label):
		_battle_label.text = str(lines.get("battle", "Battle: unknown"))
	if is_instance_valid(_phase_label):
		_phase_label.text = str(lines.get("phase", "Phase: unknown"))
	if is_instance_valid(_turn_label):
		_turn_label.text = str(lines.get("turn", "Turn State: unknown"))
	if is_instance_valid(_prompt_label):
		_prompt_label.text = str(lines.get("prompt", "Prompt: unknown"))
	if is_instance_valid(_queue_label):
		_queue_label.text = str(lines.get("queue", "Queued commands: 0"))
	if is_instance_valid(_event_label):
		_event_label.text = "Resolution events: %d" % int(runtime.battle_state.resolution_events.size())
	if is_instance_valid(_flow_label):
		_flow_label.text = str(lines.get("flow", "Prompt flow: unknown"))
	if is_instance_valid(_history_label):
		_history_label.text = str(lines.get("history", "History: unknown"))
	if is_instance_valid(_assets_label):
		var summary: Dictionary = runtime.battle_state.asset_summary
		_assets_label.text = "Assets: %d pokemon, %d moves, %d items, %d trainers, %d anim tables, %d bundles, %d frontpics, %d packs" % [
			int(summary.get("pokemon_count", 0)),
			int(summary.get("move_count", 0)),
			int(summary.get("item_count", 0)),
			int(summary.get("trainer_count", 0)),
			int(summary.get("battle_animation_count", 0)),
			int(summary.get("battle_anim_bundle_count", 0)),
			int(summary.get("frontpic_animation_count", 0)),
			int(summary.get("content_pack_count", 0)),
		]
	if is_instance_valid(_debug_label):
		_debug_label.text = str(lines.get("debug", runtime.battle_state.debug_text()))

func handle_action(action: String) -> bool:
	if _input == null:
		_input = BATTLE_UI_INPUT_SCRIPT.new()
	return _input.handle_input(_ui_state, action)

func _bind_labels() -> void:
	_title_label = get_node_or_null("Margin/VBox/TitleLabel")
	_battle_label = get_node_or_null("Margin/VBox/StatusPanel/Margin/StatusVBox/BattleLabel")
	_phase_label = get_node_or_null("Margin/VBox/StatusPanel/Margin/StatusVBox/PhaseLabel")
	_turn_label = get_node_or_null("Margin/VBox/StatusPanel/Margin/StatusVBox/TurnLabel")
	_prompt_label = get_node_or_null("Margin/VBox/StatusPanel/Margin/StatusVBox/PromptLabel")
	_queue_label = get_node_or_null("Margin/VBox/StatusPanel/Margin/StatusVBox/QueueLabel")
	_event_label = get_node_or_null("Margin/VBox/StatusPanel/Margin/StatusVBox/EventLabel")
	_flow_label = get_node_or_null("Margin/VBox/StatusPanel/Margin/StatusVBox/FlowLabel")
	_history_label = get_node_or_null("Margin/VBox/StatusPanel/Margin/StatusVBox/HistoryLabel")
	_assets_label = get_node_or_null("Margin/VBox/StatusPanel/Margin/StatusVBox/AssetsLabel")
	_debug_label = get_node_or_null("Margin/VBox/StatusPanel/Margin/StatusVBox/DebugLabel")
