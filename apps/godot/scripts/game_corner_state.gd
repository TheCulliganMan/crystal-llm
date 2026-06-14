extends RefCounted
class_name GameCornerState

const RNG_POLY := 0xb400

const SLOT_REEL_LENGTH := 15
const SLOT_MODE_NORMAL := "normal"
const SLOT_MODE_LUCKY := "lucky"

const SLOT_SYMBOL_SEVEN := 0
const SLOT_SYMBOL_POKEBALL := 1
const SLOT_SYMBOL_CHERRY := 2
const SLOT_SYMBOL_PIKACHU := 3
const SLOT_SYMBOL_SQUIRTLE := 4
const SLOT_SYMBOL_STARYU := 5

const SLOT_SYMBOL_PAYOUTS := {
	SLOT_SYMBOL_SEVEN: 300,
	SLOT_SYMBOL_POKEBALL: 50,
	SLOT_SYMBOL_CHERRY: 6,
	SLOT_SYMBOL_PIKACHU: 8,
	SLOT_SYMBOL_SQUIRTLE: 10,
	SLOT_SYMBOL_STARYU: 15,
}

const SLOT_REEL_TILEMAPS := [
	[
		SLOT_SYMBOL_SEVEN,
		SLOT_SYMBOL_CHERRY,
		SLOT_SYMBOL_STARYU,
		SLOT_SYMBOL_PIKACHU,
		SLOT_SYMBOL_SQUIRTLE,
		SLOT_SYMBOL_SEVEN,
		SLOT_SYMBOL_CHERRY,
		SLOT_SYMBOL_STARYU,
		SLOT_SYMBOL_PIKACHU,
		SLOT_SYMBOL_SQUIRTLE,
		SLOT_SYMBOL_POKEBALL,
		SLOT_SYMBOL_CHERRY,
		SLOT_SYMBOL_STARYU,
		SLOT_SYMBOL_PIKACHU,
		SLOT_SYMBOL_SQUIRTLE,
	],
	[
		SLOT_SYMBOL_SEVEN,
		SLOT_SYMBOL_PIKACHU,
		SLOT_SYMBOL_CHERRY,
		SLOT_SYMBOL_SQUIRTLE,
		SLOT_SYMBOL_STARYU,
		SLOT_SYMBOL_POKEBALL,
		SLOT_SYMBOL_PIKACHU,
		SLOT_SYMBOL_CHERRY,
		SLOT_SYMBOL_SQUIRTLE,
		SLOT_SYMBOL_STARYU,
		SLOT_SYMBOL_POKEBALL,
		SLOT_SYMBOL_PIKACHU,
		SLOT_SYMBOL_CHERRY,
		SLOT_SYMBOL_SQUIRTLE,
		SLOT_SYMBOL_STARYU,
	],
	[
		SLOT_SYMBOL_SEVEN,
		SLOT_SYMBOL_PIKACHU,
		SLOT_SYMBOL_CHERRY,
		SLOT_SYMBOL_SQUIRTLE,
		SLOT_SYMBOL_STARYU,
		SLOT_SYMBOL_PIKACHU,
		SLOT_SYMBOL_CHERRY,
		SLOT_SYMBOL_SQUIRTLE,
		SLOT_SYMBOL_STARYU,
		SLOT_SYMBOL_PIKACHU,
		SLOT_SYMBOL_POKEBALL,
		SLOT_SYMBOL_CHERRY,
		SLOT_SYMBOL_SQUIRTLE,
		SLOT_SYMBOL_STARYU,
		SLOT_SYMBOL_PIKACHU,
	],
]

const CARD_TYPES := [
	"ODDISH",
	"POLIWAG",
	"PIKACHU",
	"JIGGLYPUFF",
	"RATTATA",
	"VOLTORB",
]

const MEMORY_SYMBOLS := [
	"ODDISH",
	"POLIWAG",
	"PIKACHU",
	"JIGGLYPUFF",
	"RATTATA",
	"VOLTORB",
	"DITTO",
	"ELECTABUZZ",
]

const TARGET_UNOWN_LAYOUT := [
	[0, 0, 0, 0, 0, 0],
	[0, 1, 2, 3, 4, 0],
	[0, 5, 6, 7, 8, 0],
	[0, 9, 10, 11, 12, 0],
	[0, 13, 14, 15, 16, 0],
	[0, 0, 0, 0, 0, 0],
]

const UNOWN_START_POSITIONS := [
	[0, 0],
	[1, 0],
	[2, 0],
	[3, 0],
	[4, 0],
	[5, 0],
	[0, 1],
	[5, 1],
	[0, 2],
	[5, 2],
	[0, 3],
	[5, 3],
	[0, 4],
	[5, 4],
	[0, 5],
	[5, 5],
]

var hram: Dictionary = {}
var slot_machine_state: Dictionary = {}
var card_flip_state: Dictionary = {}
var memory_game_state: Dictionary = {}
var unown_puzzle_state: Dictionary = {}

func _init() -> void:
	reset()

func reset() -> void:
	hram = _default_hram()
	slot_machine_state = _default_slot_machine_state()
	card_flip_state = _default_card_flip_state()
	memory_game_state = _default_memory_game_state()
	unown_puzzle_state = _default_unown_puzzle_state()

func seed_rng_state(hardware_divider: int, h_random_add: int = 0, h_random_sub: int = 0) -> void:
	hram["hardware_divider"] = hardware_divider & 0xffff
	hram["hRandomAdd"] = h_random_add & 0xff
	hram["hRandomSub"] = h_random_sub & 0xff

func get_rng_state() -> Dictionary:
	return hram.duplicate(true)

func next_byte() -> int:
	var add_acc: int = int(hram.get("hRandomAdd", 0))
	var sub_acc: int = int(hram.get("hRandomSub", 0))
	var divider: int = _step_divider()
	add_acc = (add_acc + divider) & 0xff
	sub_acc = (sub_acc - divider) & 0xff
	hram["hRandomAdd"] = add_acc
	hram["hRandomSub"] = sub_acc
	return sub_acc

func randrange(upper_bound: int) -> int:
	if upper_bound <= 0:
		push_error("upperBound must be positive")
		return 0
	var mask: int = 1
	while mask < upper_bound:
		mask = (mask << 1) | 1
	var bit_length: int = str(mask).length()
	var byte_count: int = maxi(1, int(ceil(float(bit_length) / 8.0)))
	var value: int = 0
	for _attempt in range(256):
		value = 0
		for _i in range(byte_count):
			value = (value << 8) | next_byte()
		value &= mask
		if value < upper_bound:
			return value
	value = 0
	for _i in range(byte_count):
		value = (value << 8) | next_byte()
	return value % upper_bound

func coin_flip(probability: float) -> bool:
	var threshold: int = int(floor(probability * 256.0))
	return next_byte() < threshold

func randint(a: int, b: int) -> int:
	return a + randrange(b - a + 1)

func choice(seq: Array) -> Variant:
	if seq.is_empty():
		push_error("Cannot choose from an empty sequence")
		return null
	return seq[randrange(seq.size())]

func spin_slots(bet: int, mode: String = SLOT_MODE_NORMAL, bias: Variant = null, reel_positions: Variant = null) -> Dictionary:
	if bet not in [1, 2, 3]:
		push_error("bet must be 1, 2, or 3")
		return {}
	var resolved_bias: Variant = bias
	if resolved_bias == null:
		resolved_bias = _slot_get_bias(mode)
	var offsets: Array = _slot_initial_offsets(reel_positions)
	if offsets.is_empty():
		return {}
	offsets[0] = _slot_stop_reel1(int(offsets[0]), resolved_bias)
	_slot_stop_reel2(offsets, resolved_bias, bet)
	_slot_stop_reel3(offsets, resolved_bias, bet)
	var windows: Array = [
		_slot_window_for_reel(SLOT_REEL_TILEMAPS[0], int(offsets[0])),
		_slot_window_for_reel(SLOT_REEL_TILEMAPS[1], int(offsets[1])),
		_slot_window_for_reel(SLOT_REEL_TILEMAPS[2], int(offsets[2])),
	]
	var match_result: Dictionary = _slot_check_all_three_reels(windows, bet)
	var matched_symbol: Variant = match_result.get("matched_symbol", null)
	var winning_lines: Array = Array(match_result.get("winning_lines", []))
	var payout: int = 0
	if matched_symbol != null:
		payout = int(SLOT_SYMBOL_PAYOUTS.get(int(matched_symbol), 0))
	slot_machine_state = {
		"mode": mode,
		"bet": bet,
		"bias": resolved_bias,
		"reel_positions": Array(offsets).duplicate(true),
		"windows": windows,
		"matched_symbol": matched_symbol,
		"winning_lines": winning_lines,
		"payout": payout,
		"result": {
			"windows": windows,
			"matchedSymbol": matched_symbol,
			"winningLines": winning_lines,
			"payout": payout,
		},
	}
	return slot_machine_state.get("result", {})

func build_card_flip_deck() -> Array:
	var deck: Array = []
	for card_name in CARD_TYPES:
		for _i in range(4):
			deck.append(card_name)
	return deck

func shuffle_card_flip(deck: Variant = null) -> Dictionary:
	var next_deck: Array = []
	if typeof(deck) == TYPE_ARRAY:
		next_deck = Array(deck).duplicate(true)
	else:
		next_deck = build_card_flip_deck()
	for index in range(next_deck.size() - 1, 0, -1):
		var swap_index: int = randrange(index + 1)
		var temp: Variant = next_deck[index]
		next_deck[index] = next_deck[swap_index]
		next_deck[swap_index] = temp
	card_flip_state = {
		"deck": next_deck,
		"revealed": _false_array(next_deck.size()),
		"last_result": {},
	}
	return card_flip_state.duplicate(true)

func remaining_card_count(target: String) -> int:
	var deck: Array = Array(card_flip_state.get("deck", []))
	var revealed: Array = Array(card_flip_state.get("revealed", []))
	var count: int = 0
	for index in range(deck.size()):
		if not bool(revealed[index]) and str(deck[index]) == target:
			count += 1
	return count

func flip_card(index: int) -> Dictionary:
	var deck: Array = Array(card_flip_state.get("deck", []))
	var revealed: Array = Array(card_flip_state.get("revealed", []))
	if index < 0 or index >= deck.size():
		push_error("card index out of range")
		return {}
	if bool(revealed[index]):
		push_error("card already revealed")
		return {}
	revealed[index] = true
	var card_name: String = str(deck[index])
	var payout: int = _card_flip_payout_for(card_name, revealed)
	var result: Dictionary = {
		"cardIndex": index,
		"cardName": card_name,
		"payout": payout,
	}
	card_flip_state["deck"] = deck
	card_flip_state["revealed"] = revealed
	card_flip_state["last_result"] = result
	return result

func build_memory_board() -> Array:
	var board: Array = []
	for symbol in MEMORY_SYMBOLS:
		board.append(symbol)
		board.append(symbol)
	return board

func shuffle_memory_game() -> Dictionary:
	var board: Array = build_memory_board()
	for index in range(board.size() - 1, 0, -1):
		var swap_index: int = randrange(index + 1)
		var temp: Variant = board[index]
		board[index] = board[swap_index]
		board[swap_index] = temp
	memory_game_state = {
		"board": board,
		"revealed": _false_array(board.size()),
		"last_result": {},
	}
	return memory_game_state.duplicate(true)

func reveal_memory_pair(first: int, second: int) -> Dictionary:
	var board: Array = Array(memory_game_state.get("board", []))
	var revealed: Array = Array(memory_game_state.get("revealed", []))
	if first == second:
		push_error("must select two distinct tiles")
		return {}
	for index in [first, second]:
		if index < 0 or index >= board.size():
			push_error("tile index out of range")
			return {}
		if bool(revealed[index]):
			push_error("tile already revealed")
			return {}
	var first_symbol: String = str(board[first])
	var second_symbol: String = str(board[second])
	var matched: bool = first_symbol == second_symbol
	var symbol: Variant = null
	if matched:
		revealed[first] = true
		revealed[second] = true
		symbol = first_symbol
	var result: Dictionary = {
		"matched": matched,
		"firstIndex": first,
		"secondIndex": second,
		"symbol": symbol,
	}
	memory_game_state["board"] = board
	memory_game_state["revealed"] = revealed
	memory_game_state["last_result"] = result
	return result

func shuffle_unown_puzzle() -> Dictionary:
	var layout: Array = _empty_unown_layout()
	for piece_id in range(1, 17):
		while true:
			var slot_index: int = next_byte() & 0x0f
			var slot: Array = Array(UNOWN_START_POSITIONS[slot_index])
			var x: int = int(slot[0])
			var y: int = int(slot[1])
			var row: Array = Array(layout[y])
			if int(row[x]) == 0:
				row[x] = piece_id
				layout[y] = row
				break
	unown_puzzle_state = {
		"layout": layout,
		"holding_piece": null,
		"moves": 0,
		"last_result": {},
	}
	return unown_puzzle_state.duplicate(true)

func load_unown_state(layout: Variant, holding_piece: Variant = null, moves: int = 0) -> bool:
	if moves < 0:
		push_error("move count cannot be negative")
		return false
	if holding_piece != null and (int(holding_piece) < 1 or int(holding_piece) > 16):
		push_error("holding_piece must be between 1 and 16")
		return false
	var normalized_layout: Array = _normalize_unown_layout(layout)
	if normalized_layout.is_empty():
		return false
	if not _assert_unique_unown_pieces(normalized_layout, holding_piece):
		return false
	unown_puzzle_state = {
		"layout": normalized_layout,
		"holding_piece": holding_piece,
		"moves": moves,
		"last_result": {},
	}
	return true

func snapshot_unown() -> Array:
	return _clone_layout(Array(unown_puzzle_state.get("layout", TARGET_UNOWN_LAYOUT)))

func unown_status() -> Dictionary:
	return {
		"solved": is_unown_solved(),
		"moves": int(unown_puzzle_state.get("moves", 0)),
		"layout": snapshot_unown(),
		"holding_piece": unown_puzzle_state.get("holding_piece", null),
	}

func is_unown_solved() -> bool:
	return unown_puzzle_state.get("holding_piece", null) == null and snapshot_unown() == TARGET_UNOWN_LAYOUT

func pickup_unown_piece(x: int, y: int) -> int:
	if not _assert_unown_coords(x, y):
		return 0
	if unown_puzzle_state.get("holding_piece", null) != null:
		push_error("cannot pick up a piece while already holding one")
		return 0
	var layout: Array = Array(unown_puzzle_state.get("layout", _empty_unown_layout()))
	var row: Array = Array(layout[y])
	var piece: int = int(row[x])
	if piece == 0:
		push_error("no piece present at that coordinate")
		return 0
	row[x] = 0
	layout[y] = row
	unown_puzzle_state["layout"] = layout
	unown_puzzle_state["holding_piece"] = piece
	return piece

func place_unown_piece(x: int, y: int) -> int:
	if not _assert_unown_coords(x, y):
		return 0
	if unown_puzzle_state.get("holding_piece", null) == null:
		push_error("no piece is currently held")
		return 0
	var layout: Array = Array(unown_puzzle_state.get("layout", _empty_unown_layout()))
	var row: Array = Array(layout[y])
	if int(row[x]) != 0:
		push_error("target coordinate is already occupied")
		return 0
	var piece: int = int(unown_puzzle_state.get("holding_piece", 0))
	row[x] = piece
	layout[y] = row
	unown_puzzle_state["layout"] = layout
	unown_puzzle_state["holding_piece"] = null
	unown_puzzle_state["moves"] = int(unown_puzzle_state.get("moves", 0)) + 1
	return piece

func to_dictionary() -> Dictionary:
	return {
		"hram": hram.duplicate(true),
		"slot_machine_state": slot_machine_state.duplicate(true),
		"card_flip_state": card_flip_state.duplicate(true),
		"memory_game_state": memory_game_state.duplicate(true),
		"unown_puzzle_state": unown_puzzle_state.duplicate(true),
	}

func from_dictionary(data: Variant) -> bool:
	if typeof(data) != TYPE_DICTIONARY:
		return false
	var source: Dictionary = Dictionary(data)
	hram = _sanitize_dictionary(source.get("hram", source.get("rng_state", _default_hram())), _default_hram())
	slot_machine_state = _sanitize_dictionary(source.get("slot_machine_state", source.get("slots", _default_slot_machine_state())), _default_slot_machine_state())
	card_flip_state = _sanitize_dictionary(source.get("card_flip_state", source.get("card_flip", _default_card_flip_state())), _default_card_flip_state())
	memory_game_state = _sanitize_dictionary(source.get("memory_game_state", source.get("memory_game", _default_memory_game_state())), _default_memory_game_state())
	unown_puzzle_state = _sanitize_dictionary(source.get("unown_puzzle_state", source.get("unown", _default_unown_puzzle_state())), _default_unown_puzzle_state())
	hram["hardware_divider"] = int(hram.get("hardware_divider", 0)) & 0xffff
	hram["hRandomAdd"] = int(hram.get("hRandomAdd", 0)) & 0xff
	hram["hRandomSub"] = int(hram.get("hRandomSub", 0)) & 0xff
	slot_machine_state["reel_positions"] = _coerce_int_array(slot_machine_state.get("reel_positions", []))
	slot_machine_state["windows"] = _sanitize_array(slot_machine_state.get("windows", []), [])
	slot_machine_state["winning_lines"] = _sanitize_array(slot_machine_state.get("winning_lines", []), [])
	card_flip_state["deck"] = _sanitize_string_array(card_flip_state.get("deck", []))
	card_flip_state["revealed"] = _sanitize_bool_array(card_flip_state.get("revealed", []))
	memory_game_state["board"] = _sanitize_string_array(memory_game_state.get("board", []))
	memory_game_state["revealed"] = _sanitize_bool_array(memory_game_state.get("revealed", []))
	unown_puzzle_state["layout"] = _normalize_unown_layout(unown_puzzle_state.get("layout", TARGET_UNOWN_LAYOUT))
	return true

func _default_hram() -> Dictionary:
	return {
		"hardware_divider": 0,
		"hRandomAdd": 0,
		"hRandomSub": 0,
	}

func _default_slot_machine_state() -> Dictionary:
	return {
		"mode": SLOT_MODE_NORMAL,
		"bet": 1,
		"bias": null,
		"reel_positions": [0, 0, 0],
		"windows": [],
		"matched_symbol": null,
		"winning_lines": [],
		"payout": 0,
		"result": {},
	}

func _default_card_flip_state() -> Dictionary:
	return {
		"deck": build_card_flip_deck(),
		"revealed": _false_array(24),
		"last_result": {},
	}

func _default_memory_game_state() -> Dictionary:
	return {
		"board": build_memory_board(),
		"revealed": _false_array(16),
		"last_result": {},
	}

func _default_unown_puzzle_state() -> Dictionary:
	return {
		"layout": _clone_layout(TARGET_UNOWN_LAYOUT),
		"holding_piece": null,
		"moves": 0,
		"last_result": {},
	}

func _step_divider() -> int:
	var divider: int = int(hram.get("hardware_divider", 0))
	if divider == 0:
		divider = 0xace1
	var feedback: int = divider & 1
	divider >>= 1
	if feedback != 0:
		divider ^= RNG_POLY
	hram["hardware_divider"] = divider
	return divider & 0xff

func _slot_get_bias(mode: String) -> Variant:
	var table: Array = []
	if mode == SLOT_MODE_NORMAL:
		table = [
			{"threshold": 0x02 - 1, "symbol": SLOT_SYMBOL_SEVEN},
			{"threshold": 0x02 + 1, "symbol": SLOT_SYMBOL_POKEBALL},
			{"threshold": 0x0a, "symbol": SLOT_SYMBOL_STARYU},
			{"threshold": 0x14, "symbol": SLOT_SYMBOL_SQUIRTLE},
			{"threshold": 0x28, "symbol": SLOT_SYMBOL_PIKACHU},
			{"threshold": 0x30, "symbol": SLOT_SYMBOL_CHERRY},
			{"threshold": 0xff, "symbol": null},
		]
	else:
		table = [
			{"threshold": 0x02, "symbol": SLOT_SYMBOL_SEVEN},
			{"threshold": 0x02 + 1, "symbol": SLOT_SYMBOL_POKEBALL},
			{"threshold": 0x07 + 1, "symbol": SLOT_SYMBOL_STARYU},
			{"threshold": 0x0f + 1, "symbol": SLOT_SYMBOL_SQUIRTLE},
			{"threshold": 0x1e, "symbol": SLOT_SYMBOL_PIKACHU},
			{"threshold": 0x4f + 1, "symbol": SLOT_SYMBOL_CHERRY},
			{"threshold": 0xff, "symbol": null},
		]
	var roll: int = next_byte()
	for entry in table:
		if roll <= int(entry["threshold"]):
			return entry["symbol"]
	return null

func _slot_initial_offsets(reel_positions: Variant) -> Array:
	if reel_positions == null:
		return [
			next_byte() % SLOT_REEL_LENGTH,
			next_byte() % SLOT_REEL_LENGTH,
			next_byte() % SLOT_REEL_LENGTH,
		]
	if typeof(reel_positions) != TYPE_ARRAY:
		push_error("reel_positions must contain three entries")
		return []
	var positions: Array = Array(reel_positions)
	if positions.size() != 3:
		push_error("reel_positions must contain three entries")
		return []
	return [
		int(positions[0]) % SLOT_REEL_LENGTH,
		int(positions[1]) % SLOT_REEL_LENGTH,
		int(positions[2]) % SLOT_REEL_LENGTH,
	]

func _slot_stop_reel1(offset: int, bias: Variant) -> int:
	if bias == null:
		return offset
	var counter: int = 4
	while counter > 0:
		var window: Array = _slot_window_for_reel(Array(SLOT_REEL_TILEMAPS[0]), offset)
		if window.has(bias):
			break
		offset = _slot_advance(offset)
		counter -= 1
	return offset

func _slot_stop_reel2(offsets: Array, bias: Variant, bet: int) -> void:
	var maybe_skip: bool = false
	if bet >= 2 and (bias == null or int(bias) == SLOT_SYMBOL_SEVEN):
		maybe_skip = next_byte() < 0x4f + 1
	if maybe_skip:
		var aligned: Variant = _slot_attempt_skip_to_seven(offsets, bet)
		if typeof(aligned) == TYPE_ARRAY:
			offsets[0] = aligned[0]
			offsets[1] = aligned[1]
			offsets[2] = aligned[2]
			return
	var counter: int = 4
	while true:
		var windows: Array = [
			_slot_window_for_reel(Array(SLOT_REEL_TILEMAPS[0]), int(offsets[0])),
			_slot_window_for_reel(Array(SLOT_REEL_TILEMAPS[1]), int(offsets[1])),
		]
		var match_result: Dictionary = _slot_check_first_two_reels(windows, bet)
		var matched_symbol: Variant = match_result.get("matched_symbol", null)
		if matched_symbol != null and matched_symbol == bias:
			return
		if bias == null or counter == 0:
			return
		offsets[1] = _slot_advance(int(offsets[1]))
		counter -= 1

func _slot_stop_reel3(offsets: Array, bias: Variant, bet: int) -> void:
	var windows_first_two: Array = [
		_slot_window_for_reel(Array(SLOT_REEL_TILEMAPS[0]), int(offsets[0])),
		_slot_window_for_reel(Array(SLOT_REEL_TILEMAPS[1]), int(offsets[1])),
	]
	var check_first_two: Dictionary = _slot_check_first_two_reels(windows_first_two, bet)
	var matched_symbol: Variant = check_first_two.get("matched_symbol", null)
	var saw_seven: bool = bool(check_first_two.get("saw_seven", false))
	if matched_symbol == null or not saw_seven:
		_slot_apply_reel3_stop(offsets, bias, bet)
		return
	var action: String = _slot_select_reel3_action(bias)
	if action == "stop":
		_slot_apply_reel3_stop(offsets, bias, bet)
	elif action == "slow":
		_slot_apply_reel3_slow_advance(offsets, bias, bet)
	elif action == "golem":
		_slot_apply_reel3_golem(offsets, bias, bet)
	else:
		_slot_apply_reel3_chansey(offsets, bet)

func _slot_apply_reel3_stop(offsets: Array, bias: Variant, bet: int) -> void:
	var counter: int = 4
	while true:
		var windows: Array = [
			_slot_window_for_reel(Array(SLOT_REEL_TILEMAPS[0]), int(offsets[0])),
			_slot_window_for_reel(Array(SLOT_REEL_TILEMAPS[1]), int(offsets[1])),
			_slot_window_for_reel(Array(SLOT_REEL_TILEMAPS[2]), int(offsets[2])),
		]
		var match_result: Dictionary = _slot_check_all_three_reels(windows, bet)
		var matched_symbol: Variant = match_result.get("matched_symbol", null)
		if matched_symbol != null:
			if matched_symbol == bias:
				return
			offsets[2] = _slot_advance(int(offsets[2]))
			if counter > 0:
				counter -= 1
			continue
		if bias == null or counter == 0:
			return
		offsets[2] = _slot_advance(int(offsets[2]))
		counter -= 1

func _slot_select_reel3_action(bias: Variant) -> String:
	if bias == SLOT_SYMBOL_SEVEN:
		var roll: int = next_byte()
		if roll >= 0xb4:
			return "stop"
		if roll >= 0x78:
			return "slow"
		if roll >= 0x3c:
			return "golem"
		return "chansey"
	var roll_other: int = next_byte()
	if roll_other >= 0xa0:
		return "stop"
	if roll_other >= 0x4f + 1:
		return "slow"
	return "golem"

func _slot_apply_reel3_slow_advance(offsets: Array, bias: Variant, bet: int) -> void:
	var target: Variant = SLOT_SYMBOL_SEVEN if bias == SLOT_SYMBOL_SEVEN else null
	offsets[2] = _slot_find_offset_for_match(offsets, bet, target, 1)

func _slot_apply_reel3_golem(offsets: Array, bias: Variant, bet: int) -> void:
	if bias == SLOT_SYMBOL_SEVEN:
		offsets[2] = _slot_find_offset_for_match(offsets, bet, SLOT_SYMBOL_SEVEN, 1)
		return
	var stride: int = 0
	while stride < 4:
		stride = next_byte() & 0x7
	var step: int = stride
	for _i in range(SLOT_REEL_LENGTH * 2):
		var windows: Array = [
			_slot_window_for_reel(Array(SLOT_REEL_TILEMAPS[0]), int(offsets[0])),
			_slot_window_for_reel(Array(SLOT_REEL_TILEMAPS[1]), int(offsets[1])),
			_slot_window_for_reel(Array(SLOT_REEL_TILEMAPS[2]), int(offsets[2])),
		]
		var match_result: Dictionary = _slot_check_all_three_reels(windows, bet)
		if match_result.get("matched_symbol", null) == null:
			return
		offsets[2] = _slot_advance(int(offsets[2]), step)
		step += 1
	push_error("Golem manipulation failed to break matching reels")

func _slot_apply_reel3_chansey(offsets: Array, bet: int) -> void:
	offsets[2] = _slot_find_offset_for_match(offsets, bet, SLOT_SYMBOL_SEVEN, 17)

func _slot_find_offset_for_match(offsets: Array, bet: int, target_symbol: Variant, step: int = 1) -> int:
	var working_offsets: Array = offsets.duplicate(true)
	for _i in range(SLOT_REEL_LENGTH * 2):
		var windows: Array = [
			_slot_window_for_reel(Array(SLOT_REEL_TILEMAPS[0]), int(working_offsets[0])),
			_slot_window_for_reel(Array(SLOT_REEL_TILEMAPS[1]), int(working_offsets[1])),
			_slot_window_for_reel(Array(SLOT_REEL_TILEMAPS[2]), int(working_offsets[2])),
		]
		var match_result: Dictionary = _slot_check_all_three_reels(windows, bet)
		var matched_symbol: Variant = match_result.get("matched_symbol", null)
		if target_symbol == null:
			if matched_symbol == null:
				return int(working_offsets[2])
		elif matched_symbol == target_symbol:
			return int(working_offsets[2])
		working_offsets[2] = _slot_advance(int(working_offsets[2]), step)
	push_error("Failed to resolve reel 3 action within bounds")
	return int(offsets[2])

func _slot_attempt_skip_to_seven(offsets: Array, bet: int) -> Variant:
	var first_window: Array = _slot_window_for_reel(Array(SLOT_REEL_TILEMAPS[0]), int(offsets[0]))
	if not first_window.has(SLOT_SYMBOL_SEVEN):
		return null
	var offset_two: int = int(offsets[1])
	for _i in range(SLOT_REEL_LENGTH * 2):
		var windows: Array = [
			first_window,
			_slot_window_for_reel(Array(SLOT_REEL_TILEMAPS[1]), offset_two),
		]
		var check_result: Dictionary = _slot_check_first_two_reels(windows, bet)
		if bool(check_result.get("saw_seven", false)):
			return [int(offsets[0]), offset_two, int(offsets[2])]
		offset_two = _slot_advance(offset_two)
	push_error("Failed to align reel 2 to seven after skip-to-7 setup")
	return null

func _slot_window_for_reel(reel: Array, offset: int) -> Array:
	return [
		reel[_slot_wrap_index(offset)],
		reel[_slot_wrap_index(offset + 1)],
		reel[_slot_wrap_index(offset + 2)],
	]

func _slot_line_order_for_bet(bet: int) -> Array:
	if bet == 1:
		return ["middle"]
	if bet == 2:
		return ["bottom", "top", "middle"]
	if bet == 3:
		return ["diagonal_up", "diagonal_down", "bottom", "top", "middle"]
	push_error("bet must be 1, 2, or 3")
	return []

func _slot_line_symbols(windows: Array, line: String) -> Array:
	if line == "middle":
		return [windows[0][1], windows[1][1], windows[2][1]]
	if line == "top":
		return [windows[0][0], windows[1][0], windows[2][0]]
	if line == "bottom":
		return [windows[0][2], windows[1][2], windows[2][2]]
	if line == "diagonal_up":
		return [windows[0][2], windows[1][1], windows[2][0]]
	if line == "diagonal_down":
		return [windows[0][0], windows[1][1], windows[2][2]]
	push_error("Unknown line identifier %s" % line)
	return []

func _slot_check_first_two_reels(windows: Array, bet: int) -> Dictionary:
	var matched_symbol: Variant = null
	var matched_lines: Array = []
	var saw_seven: bool = false
	for line in _slot_line_order_for_bet(bet):
		var first: int = 0
		var second: int = 0
		if line == "middle":
			first = windows[0][1]
			second = windows[1][1]
		elif line == "top":
			first = windows[0][0]
			second = windows[1][0]
		elif line == "bottom":
			first = windows[0][2]
			second = windows[1][2]
		elif line == "diagonal_up":
			first = windows[0][2]
			second = windows[1][1]
		else:
			first = windows[0][0]
			second = windows[1][1]
		if first == second:
			matched_symbol = first
			matched_lines.append(line)
			saw_seven = saw_seven or first == SLOT_SYMBOL_SEVEN
	return {
		"matched_symbol": matched_symbol,
		"saw_seven": saw_seven,
		"matched_lines": matched_lines,
	}

func _slot_check_all_three_reels(windows: Array, bet: int) -> Dictionary:
	var matched_symbol: Variant = null
	var matched_lines: Array = []
	for line in _slot_line_order_for_bet(bet):
		var symbols: Array = _slot_line_symbols(windows, line)
		if symbols[0] == symbols[1] and symbols[1] == symbols[2]:
			matched_symbol = symbols[0]
			matched_lines.append(line)
	return {
		"matched_symbol": matched_symbol,
		"winning_lines": matched_lines,
	}

func _slot_advance(offset: int, step: int = 1) -> int:
	return (offset + step) % SLOT_REEL_LENGTH

func _slot_wrap_index(index: int) -> int:
	return index % SLOT_REEL_LENGTH

func _card_flip_payout_for(card_name: String, revealed: Array) -> int:
	var remaining: int = 0
	for index in range(card_flip_state.get("deck", []).size()):
		if not bool(revealed[index]) and str(card_flip_state.get("deck", [])[index]) == card_name:
			remaining += 1
	if card_name == "PIKACHU":
		var payouts: Dictionary = {6: 6, 5: 12, 4: 24, 3: 36, 2: 48, 1: 72}
		return int(payouts.get(remaining, 6))
	var normal_payouts: Dictionary = {4: 6, 3: 12, 2: 18, 1: 36}
	return int(normal_payouts.get(remaining, 6))

func _false_array(length: int) -> Array:
	var output: Array = []
	for _i in range(length):
		output.append(false)
	return output

func _empty_unown_layout() -> Array:
	return [
		[0, 0, 0, 0, 0, 0],
		[0, 0, 0, 0, 0, 0],
		[0, 0, 0, 0, 0, 0],
		[0, 0, 0, 0, 0, 0],
		[0, 0, 0, 0, 0, 0],
		[0, 0, 0, 0, 0, 0],
	]

func _clone_layout(layout: Array) -> Array:
	var output: Array = []
	for row in layout:
		output.append(Array(row).duplicate(true))
	return output

func _normalize_unown_layout(layout: Variant) -> Array:
	if typeof(layout) != TYPE_ARRAY:
		push_error("layout must contain six rows")
		return []
	var source_layout: Array = Array(layout)
	if source_layout.size() != 6:
		push_error("layout must contain six rows")
		return []
	var normalized: Array = []
	for row in source_layout:
		if typeof(row) != TYPE_ARRAY:
			push_error("layout rows must contain six columns")
			return []
		var source_row: Array = Array(row)
		if source_row.size() != 6:
			push_error("layout rows must contain six columns")
			return []
		var normalized_row: Array = []
		for value in source_row:
			if int(value) != value:
				push_error("layout entries must be integers")
				return []
			var piece_value: int = int(value)
			if piece_value != 0 and (piece_value < 1 or piece_value > 16):
				push_error("layout entries must be 0 or between 1 and 16")
				return []
			normalized_row.append(piece_value)
		normalized.append(normalized_row)
	return normalized

func _assert_unique_unown_pieces(layout: Array, holding_piece: Variant) -> bool:
	var seen: Dictionary = {}
	for row in layout:
		for value in Array(row):
			var piece_value: int = int(value)
			if piece_value == 0:
				continue
			if seen.has(piece_value):
				push_error("piece %d appears more than once in the puzzle state" % piece_value)
				return false
			seen[piece_value] = true
	if holding_piece != null and seen.has(int(holding_piece)):
		push_error("held piece %d also appears in the puzzle layout" % int(holding_piece))
		return false
	return true

func _assert_unown_coords(x: int, y: int) -> bool:
	if x != int(x) or y != int(y):
		push_error("coordinates must be integer puzzle grid cells")
		return false
	if x < 0 or x >= 6 or y < 0 or y >= 6:
		push_error("coordinates must be inside the 6x6 puzzle grid")
		return false
	return true

func _sanitize_dictionary(value: Variant, fallback: Dictionary) -> Dictionary:
	if typeof(value) == TYPE_DICTIONARY:
		return Dictionary(value).duplicate(true)
	return fallback.duplicate(true)

func _sanitize_array(value: Variant, fallback: Array) -> Array:
	if typeof(value) == TYPE_ARRAY:
		return Array(value).duplicate(true)
	return fallback.duplicate(true)

func _sanitize_string_array(value: Variant) -> Array:
	var output: Array = []
	if typeof(value) != TYPE_ARRAY:
		return output
	for entry in Array(value):
		output.append(str(entry))
	return output

func _sanitize_bool_array(value: Variant) -> Array:
	var output: Array = []
	if typeof(value) != TYPE_ARRAY:
		return output
	for entry in Array(value):
		output.append(bool(entry))
	return output

func _coerce_int_array(value: Variant) -> Array:
	var output: Array = []
	if typeof(value) != TYPE_ARRAY:
		return output
	for entry in Array(value):
		output.append(int(entry))
	return output
