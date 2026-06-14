extends RefCounted
class_name BattleUIInput

const MOVE_NONE := "none"

func handle_input(state, action: String) -> bool:
	if state == null:
		return false
	if state.has_method("handle_action"):
		return bool(state.handle_action(action))
	return false

func get_player_input(state, choices: Array = []) -> Variant:
	if state == null:
		return null
	if choices.is_empty():
		return null
	if state.has_method("open_command_menu") and not state.has_method("has_command_menu"):
		state.open_command_menu(choices)
	elif state.has_method("open_command_menu") and not bool(state.has_command_menu()):
		state.open_command_menu(choices)
	var index := int(state.submenu_index)
	if index < 0 or index >= choices.size():
		index = 0
	state.last_action = "choice:%d" % index
	return choices[index]
