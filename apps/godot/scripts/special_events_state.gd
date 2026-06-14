extends RefCounted
class_name SpecialEventsState

const BUENA_PASSWORD_CATEGORIES := [
	{
		"label": "Johto Starters",
		"category_type": "MON",
		"points": 10,
		"options": ["CYNDAQUIL", "TOTODILE", "CHIKORITA"],
	},
	{
		"label": "Beverages",
		"category_type": "ITEM",
		"points": 12,
		"options": ["FRESH_WATER", "SODA_POP", "LEMONADE"],
	},
	{
		"label": "Healing Items",
		"category_type": "ITEM",
		"points": 12,
		"options": ["POTION", "ANTIDOTE", "PARLYZ_HEAL"],
	},
	{
		"label": "Balls",
		"category_type": "ITEM",
		"points": 12,
		"options": ["POKE_BALL", "GREAT_BALL", "ULTRA_BALL"],
	},
	{
		"label": "Pokémon 1",
		"category_type": "MON",
		"points": 10,
		"options": ["PIKACHU", "RATTATA", "GEODUDE"],
	},
	{
		"label": "Pokémon 2",
		"category_type": "MON",
		"points": 10,
		"options": ["HOOTHOOT", "SPINARAK", "DROWZEE"],
	},
	{
		"label": "Johto Towns",
		"category_type": "STRING",
		"points": 16,
		"options": ["NEW BARK TOWN", "CHERRYGROVE CITY", "AZALEA TOWN"],
	},
	{
		"label": "Types",
		"category_type": "STRING",
		"points": 6,
		"options": ["FLYING", "BUG", "GRASS"],
	},
	{
		"label": "Moves",
		"category_type": "MOVE",
		"points": 12,
		"options": ["TACKLE", "GROWL", "MUD_SLAP"],
	},
	{
		"label": "X Items",
		"category_type": "ITEM",
		"points": 12,
		"options": ["X_ATTACK", "X_DEFEND", "X_SPEED"],
	},
	{
		"label": "Radio Stations",
		"category_type": "STRING",
		"points": 13,
		"options": ["#MON TALK", "#MON MUSIC", "LUCKY CHANNEL"],
	},
]

const BUENA_PRIZES := [
	{"item": "ULTRA_BALL", "cost": 2},
	{"item": "FULL_RESTORE", "cost": 2},
	{"item": "NUGGET", "cost": 3},
	{"item": "RARE_CANDY", "cost": 3},
	{"item": "PROTEIN", "cost": 5},
	{"item": "IRON", "cost": 5},
	{"item": "CARBOS", "cost": 5},
	{"item": "CALCIUM", "cost": 5},
	{"item": "HP_UP", "cost": 5},
]

const PC_HUB_ACTIONS := ["bills_pc", "player_pc", "oak_pc", "hall_of_fame", "turn_off"]

var _runtime_state: Dictionary = {}
var _state: Dictionary = {}

func _init() -> void:
	reset()

func reset() -> void:
	_runtime_state = {}
	_state = _build_state()

func configure(snapshot: Dictionary) -> void:
	sync_runtime_state(snapshot)

func sync_runtime_state(snapshot: Dictionary) -> void:
	if typeof(snapshot) != TYPE_DICTIONARY:
		return
	_runtime_state = _merge_dictionary(_runtime_state, snapshot)
	_state = _build_state()

func get_state() -> Dictionary:
	return _state.duplicate(true)

func from_dictionary(data: Variant) -> bool:
	reset()
	if typeof(data) != TYPE_DICTIONARY:
		return true
	var source: Dictionary = Dictionary(data)
	if typeof(source.get("runtime_state", {})) == TYPE_DICTIONARY:
		_runtime_state = Dictionary(source.get("runtime_state", {})).duplicate(true)
	if typeof(source.get("state", {})) == TYPE_DICTIONARY:
		_state = Dictionary(source.get("state", {})).duplicate(true)
		_state["runtime_state"] = _runtime_state.duplicate(true)
		if not _state.has("domains") or typeof(_state.get("domains", {})) != TYPE_DICTIONARY:
			_state["domains"] = _build_domains()
		if not _state.has("intents") or typeof(_state.get("intents", {})) != TYPE_DICTIONARY:
			_state["intents"] = {}
		return true
	_state = _build_state()
	return true

func to_dictionary() -> Dictionary:
	return {
		"runtime_state": _runtime_state.duplicate(true),
		"state": _state.duplicate(true),
	}

func queue_intent(domain: String, action: String, payload: Dictionary = {}) -> Dictionary:
	var normalized_domain := domain.strip_edges().to_lower()
	if normalized_domain.is_empty():
		normalized_domain = "unknown"
	var normalized_action := action.strip_edges().to_lower()
	var intent := {
		"domain": normalized_domain,
		"action": normalized_action,
		"payload": Dictionary(payload).duplicate(true),
	}
	var intents := Dictionary(_state.get("intents", {})).duplicate(true)
	intents[normalized_domain] = intent
	_state["intents"] = intents
	return Dictionary(intent).duplicate(true)

func get_domain_state(domain: String) -> Dictionary:
	var domains := Dictionary(_state.get("domains", {}))
	return Dictionary(domains.get(domain.strip_edges().to_lower(), {})).duplicate(true)

func _build_state() -> Dictionary:
	var intents := {}
	if _state.has("intents") and typeof(_state.get("intents", {})) == TYPE_DICTIONARY:
		intents = Dictionary(_state.get("intents", {})).duplicate(true)
	return {
		"runtime_state": _runtime_state.duplicate(true),
		"domains": _build_domains(),
		"intents": intents,
	}

func _build_domains() -> Dictionary:
	return {
		"day_care": _build_day_care_state(),
		"decorations": _build_decorations_state(),
		"mystery_gift": _build_mystery_gift_state(),
		"lucky_number": _build_lucky_number_state(),
		"kurt": _build_kurt_state(),
		"buena": _build_buena_state(),
		"bug_contest": _build_bug_contest_state(),
		"magnet_train": _build_magnet_train_state(),
		"mom": _build_mom_state(),
		"pc_helpers": _build_pc_helpers_state(),
	}

func _build_day_care_state() -> Dictionary:
	var sram := _runtime_sram()
	var day_care := Dictionary(sram.get("day_care", {}))
	var man := _resident_summary(Dictionary(day_care.get("man", {})))
	var lady := _resident_summary(Dictionary(day_care.get("lady", {})))
	var summary := {
		"man": man,
		"lady": lady,
		"egg_present": day_care.get("egg_present", false) == true,
		"steps_since_last_egg": int(day_care.get("steps_since_last_egg", 0)),
		"can_breed": day_care.get("can_breed", false) == true,
	}
	return {
		"source": "runtime",
		"summary": summary,
		"actions": [
			{"action": "day_care_man", "intent": "day_care", "payload": {"target": "man", "summary": summary.duplicate(true)}},
			{"action": "day_care_lady", "intent": "day_care", "payload": {"target": "lady", "summary": summary.duplicate(true)}},
			{"action": "day_care_man_outside", "intent": "day_care", "payload": {"target": "outside", "summary": summary.duplicate(true)}},
			{"action": "day_care_mon1", "intent": "day_care", "payload": {"target": "mon1", "summary": summary.duplicate(true)}},
			{"action": "day_care_mon2", "intent": "day_care", "payload": {"target": "mon2", "summary": summary.duplicate(true)}},
		],
	}

func _build_decorations_state() -> Dictionary:
	var wram := _runtime_wram()
	var event_flags := Dictionary(wram.get("event_flags", {})).duplicate(true)
	var variable_sprites := Dictionary(wram.get("variable_sprites", {})).duplicate(true)
	var summary := {
		"maptile_visible": wram.get("maptile_decorations_visible", false) == true,
		"decorations_visible": wram.get("decorations_visible", false) == true,
		"event_flags": event_flags,
		"variable_sprites": variable_sprites,
	}
	return {
		"summary": summary,
		"actions": [
			{"action": "toggle_maptile_decorations", "intent": "decorations", "payload": {"field": "maptile_decorations_visible"}},
			{"action": "toggle_decorations_visibility", "intent": "decorations", "payload": {"field": "decorations_visible"}},
		],
	}

func _build_mystery_gift_state() -> Dictionary:
	var sram := _runtime_sram()
	var mystery_gift := Dictionary(sram.get("mystery_gift", {}))
	var summary := {
		"unlocked": sram.get("mystery_gift_unlocked", false) == true,
		"stored_item": str(mystery_gift.get("stored_item", "")),
		"backup_item": str(mystery_gift.get("backup_item", "")),
		"daily_partner_ids": Array(mystery_gift.get("daily_partner_ids", [])).duplicate(true),
	}
	return {
		"summary": summary,
		"actions": [
			{"action": "check_mystery_gift", "intent": "mystery_gift", "payload": {"check": true}},
			{"action": "get_mystery_gift_item", "intent": "mystery_gift", "payload": {"stored_item": summary["stored_item"]}},
			{"action": "unlock_mystery_gift", "intent": "mystery_gift", "payload": {"unlock": true}},
		],
	}

func _build_lucky_number_state() -> Dictionary:
	var sram := _runtime_sram()
	var wram := _runtime_wram()
	var summary := {
		"lucky_number_day": int(sram.get("lucky_number_day", -1)),
		"lucky_id_number": int(sram.get("lucky_id_number", 0)),
		"show_flag": wram.get("lucky_number_show_flag", false) == true,
		"party_count": int(wram.get("wPartyCount", Array(Dictionary(sram.get("party", {})).get("pokemon", [])).size())),
		"current_box_index": int(sram.get("current_pc_box", 0)),
		"winner_tier": int(wram.get("lucky_number_winner_tier", 0)),
		"winner_species": str(wram.get("wCurPartySpecies", "")),
		"winner_source": str(wram.get("lucky_number_winner_source", "")),
	}
	return {
		"summary": summary,
		"actions": [
			{"action": "check_for_lucky_number_winners", "intent": "lucky_number", "payload": {"check": true}},
			{"action": "check_lucky_number_show_flag", "intent": "lucky_number", "payload": {"check": true}},
			{"action": "reset_lucky_number_show_flag", "intent": "lucky_number", "payload": {"reset": true}},
		],
	}

func _build_kurt_state() -> Dictionary:
	var apricorn_choices := _collect_apricorn_entries()
	var intent := _current_intent("kurt")
	var payload := Dictionary(intent.get("payload", {}))
	return {
		"summary": {
			"apricorn_choices": apricorn_choices.duplicate(true),
			"choice_count": apricorn_choices.size(),
			"selected_apricorn": str(payload.get("selected_apricorn", "")),
			"selected_quantity": int(payload.get("quantity", 0)),
		},
		"actions": [
			{"action": "select_apricorn_for_kurt", "intent": "kurt", "payload": {"choices": apricorn_choices.duplicate(true)}},
		],
	}

func _build_buena_state() -> Dictionary:
	var wram := _runtime_wram()
	var current_category_index := int(wram.get("buenas_password_category", 0))
	var current_option_index := int(wram.get("buenas_password_index", 0))
	var category := Dictionary(BUENA_PASSWORD_CATEGORIES[clampi(current_category_index, 0, BUENA_PASSWORD_CATEGORIES.size() - 1)])
	var option := ""
	if not Array(category.get("options", [])).is_empty():
		var options := Array(category.get("options", []))
		option = str(options[clampi(current_option_index, 0, options.size() - 1)])
	return {
		"summary": {
			"balance": int(wram.get("blue_card_balance", 0)),
			"password_category_index": current_category_index,
			"password_category_label": str(category.get("label", "")),
			"password_option_index": current_option_index,
			"password_option": option,
			"password_categories": BUENA_PASSWORD_CATEGORIES.duplicate(true),
			"prizes": BUENA_PRIZES.duplicate(true),
		},
		"actions": [
			{"action": "buenas_password", "intent": "buena", "payload": {"category_index": current_category_index, "option_index": current_option_index}},
			{"action": "buena_prize", "intent": "buena", "payload": {"prizes": BUENA_PRIZES.duplicate(true)}},
			{"action": "ask_remember_password", "intent": "buena", "payload": {"prompt": true}},
		],
	}

func _build_bug_contest_state() -> Dictionary:
	var wram := _runtime_wram()
	var hram := _runtime_hram()
	var contest_state := Dictionary(wram.get("bug_contest_state", {}))
	var results := Dictionary(wram.get("bug_contest_results", {}))
	var pending := Dictionary(contest_state.get("pending_caught_mon", {}))
	return {
		"summary": {
			"timer_active": contest_state.get("timer_active", false) == true,
			"park_balls_remaining": int(contest_state.get("park_balls_remaining", 0)),
			"caught_species": str(contest_state.get("caught_species", "")),
			"caught_level": int(contest_state.get("caught_level", 0)),
			"pending_caught_mon": pending.duplicate(true),
			"results": results.duplicate(true),
			"contest_time": [int(wram.get("wCurDay", 0)) % 256, int(hram.get("hHours", 0)) % 24, int(hram.get("hMinutes", 0)) % 60, int(hram.get("hSeconds", 0)) % 60],
		},
		"actions": [
			{"action": "give_park_balls", "intent": "bug_contest", "payload": {"balls": 20}},
			{"action": "contest_drop_off_mons", "intent": "bug_contest", "payload": {"drop_off": true}},
			{"action": "contest_return_mons", "intent": "bug_contest", "payload": {"return": true}},
			{"action": "bug_contest_judging", "intent": "bug_contest", "payload": {"judge": true}},
			{"action": "bug_contest_set_caught_contest_mon", "intent": "bug_contest", "payload": {"caught": pending.duplicate(true)}},
			{"action": "check_party_full_after_contest", "intent": "bug_contest", "payload": {"validate": true}},
		],
	}

func _build_magnet_train_state() -> Dictionary:
	var specials := Dictionary(_runtime_state.get("specials", {}))
	var magnet := Dictionary(Dictionary(specials.get("magnet_train", {})).duplicate(true))
	return {
		"summary": {
			"count": int(magnet.get("count", 0)),
			"direction_token": str(magnet.get("direction_token", "")),
			"destination": str(magnet.get("destination", "")),
			"scene": str(magnet.get("scene", "")),
		},
		"actions": [
			{"action": "magnet_train", "intent": "magnet_train", "payload": magnet.duplicate(true)},
		],
	}

func _build_mom_state() -> Dictionary:
	var sram := _runtime_sram()
	return {
		"summary": {
			"money": int(sram.get("money", 0)),
			"moms_money": int(sram.get("moms_money", 0)),
			"mom_saving_active": sram.get("mom_saving_active", false) == true,
			"mom_saving_some_money": sram.get("mom_saving_some_money", false) == true,
		},
		"actions": [
			{"action": "bank_of_mom", "intent": "mom", "payload": {"money": int(sram.get("money", 0)), "moms_money": int(sram.get("moms_money", 0))}},
		],
	}

func _build_pc_helpers_state() -> Dictionary:
	var sram := _runtime_sram()
	var wram := _runtime_wram()
	var intent := _current_intent("pc_helpers")
	var intent_payload := Dictionary(intent.get("payload", {}))
	var player_name := str(_runtime_state.get("player_name", sram.get("player_name", ""))).strip_edges()
	if player_name.is_empty():
		player_name = "PLAYER"
	var has_pokedex: bool = false
	if sram.get("johto_pokedex", false) == true or Dictionary(wram.get("engine_flags", {})).get("ENGINE_POKEDEX", false) == true:
		has_pokedex = true
	var has_hof_record: bool = false
	if int(wram.get("wHallOfFameCount", 0)) > 0 or Array(sram.get("hall_of_fame", [])).size() > 0:
		has_hof_record = true
	var entries: Array = [
		{"label": "BILL's PC", "action": "bills_pc"},
		{"label": "%s's PC" % player_name, "action": "player_pc"},
	]
	if has_pokedex:
		entries.append({"label": "PROF.OAK's PC", "action": "oak_pc"})
	if has_hof_record:
		entries.append({"label": "HALL OF FAME", "action": "hall_of_fame"})
	entries.append({"label": "TURN OFF", "action": "turn_off"})
	return {
		"summary": {
			"has_pokedex": has_pokedex,
			"has_hall_of_fame_record": has_hof_record,
			"entries": entries.duplicate(true),
			"selected_index": int(intent_payload.get("selected_index", 0)),
		},
		"actions": [
			{"action": "players_house_pc", "intent": "pc_helpers", "payload": {"selection": "player_pc", "entries": entries.duplicate(true)}},
			{"action": "pokemon_center_pc", "intent": "pc_helpers", "payload": {"selection": "bills_pc", "entries": entries.duplicate(true)}},
			{"action": "BillPC", "intent": "pc_helpers", "payload": {"selection": "bills_pc", "entries": entries.duplicate(true)}},
		],
	}

func _resident_summary(resident: Dictionary) -> Dictionary:
	var pokemon_value: Variant = resident.get("pokemon", false)
	var has_pokemon := false
	if typeof(pokemon_value) == TYPE_DICTIONARY:
		has_pokemon = not Dictionary(pokemon_value).is_empty()
	else:
		has_pokemon = bool(pokemon_value)
	return {
		"pokemon": has_pokemon,
		"species": str(resident.get("species", resident.get("species_id", ""))),
		"level": int(resident.get("level", 0)),
		"nickname": str(resident.get("nickname", "")),
		"steps_since_last_egg": int(resident.get("steps_since_last_egg", 0)),
	}

func _collect_apricorn_entries() -> Array:
	var items := Dictionary(_runtime_sram().get("items", {}))
	var balls := Dictionary(_runtime_sram().get("balls", {}))
	var key_items := Dictionary(_runtime_sram().get("key_items", {}))
	var tm_hm := Dictionary(_runtime_sram().get("tm_hm", {}))
	var result: Array = []
	for storage_name in ["items", "balls", "key_items", "tm_hm"]:
		var storage: Dictionary = items if storage_name == "items" else balls if storage_name == "balls" else key_items if storage_name == "key_items" else tm_hm
		for key in storage.keys():
			var item_id := str(key).to_upper()
			if not item_id.ends_with("_APRICORN"):
				continue
			var quantity := int(storage.get(key, 0))
			if quantity <= 0:
				continue
			result.append({
				"item_id": item_id,
				"quantity": quantity,
				"storage": storage_name,
			})
	return result

func _runtime_sram() -> Dictionary:
	return Dictionary(_runtime_state.get("sram", {}))

func _runtime_wram() -> Dictionary:
	return Dictionary(_runtime_state.get("wram", {}))

func _runtime_hram() -> Dictionary:
	return Dictionary(_runtime_state.get("hram", {}))

func _merge_dictionary(base: Dictionary, overlay: Dictionary) -> Dictionary:
	var result := base.duplicate(true)
	for key in overlay.keys():
		var value: Variant = overlay[key]
		if typeof(value) == TYPE_DICTIONARY and typeof(result.get(key, {})) == TYPE_DICTIONARY:
			result[key] = _merge_dictionary(Dictionary(result.get(key, {})), Dictionary(value))
		else:
			result[key] = _duplicate_payload(value)
	return result

func _duplicate_payload(value: Variant) -> Variant:
	match typeof(value):
		TYPE_DICTIONARY:
			return Dictionary(value).duplicate(true)
		TYPE_ARRAY:
			return Array(value).duplicate(true)
		_:
			return value

func _current_intent(domain: String) -> Dictionary:
	var intents := Dictionary(_state.get("intents", {}))
	return Dictionary(intents.get(domain, {})).duplicate(true)
