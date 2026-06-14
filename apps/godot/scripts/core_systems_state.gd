extends RefCounted
class_name CoreSystemsState

const MAX_MONEY := 999999
const MAX_ITEM_STACK := 99
const POISON_STEP_INTERVAL := 4
const MART_MENU_PAGE_SIZE := 4

var state: Dictionary = {}

func _init() -> void:
	reset()

func reset() -> void:
	state = {
		"time": {
			"year": 2000,
			"month": 1,
			"day": 1,
			"hour": 10,
			"minute": 0,
			"time_of_day": "day",
			"last_daily_reset": {"year": 2000, "month": 1, "day": 1},
		},
		"wram": {
			"step_count": 0,
			"poison_step_count": 0,
			"happiness_step_count": 0,
			"daily_reset_timer": 0,
			"daily_flags1": 0,
			"daily_flags2": 0,
			"swarm_flags": 0,
			"daily_rematch_flags": [],
			"daily_phone_item_flags": [],
			"daily_phone_time_of_day_flags": [],
			"event_flags": {},
			"engine_flags": {"ENGINE_DAILY_BUG_CONTEST": false},
			"wKenjiBreakTimer": 0,
		},
		"sram": {
			"money": 0,
			"party": {"pokemon": []},
			"items": {},
			"event_flags": {},
			"mystery_gift_unlocked": false,
			"mystery_gift": {"daily_partner_ids": []},
			"current_date": {"year": 2000, "month": 1, "day": 1},
		},
		"shop": {
			"mart": "",
			"items": [],
			"selection": 0,
			"scroll": 0,
			"last_result": {},
		},
		"last_step_result": {
			"egg_hatched": false,
			"hatched_species": null,
			"poison_result": null,
		},
		"last_daily_reset_result": {},
	}

func from_dictionary(snapshot: Dictionary) -> bool:
	reset()
	if snapshot.is_empty():
		return true
	state = _merge_dictionary(state, snapshot)
	_normalize_state()
	return true

func to_dictionary() -> Dictionary:
	_normalize_state()
	return state.duplicate(true)

func configure(snapshot: Dictionary) -> void:
	state = _merge_dictionary(state, snapshot)
	_normalize_state()

func process_step() -> Dictionary:
	_normalize_state()
	var wram: Dictionary = Dictionary(state.get("wram", {}))
	var sram: Dictionary = Dictionary(state.get("sram", {}))
	var party_root: Dictionary = Dictionary(sram.get("party", {}))
	var party: Array = Array(party_root.get("pokemon", []))

	wram["poison_step_count"] = _increment_byte(int(wram.get("poison_step_count", 0)))
	wram["step_count"] = _increment_byte(int(wram.get("step_count", 0)))

	if int(wram.get("step_count", 0)) == 0:
		wram["happiness_step_count"] = (int(wram.get("happiness_step_count", 0)) + 1) & 1
		if int(wram.get("happiness_step_count", 0)) == 0:
			for index in range(party.size()):
				if typeof(party[index]) != TYPE_DICTIONARY:
					continue
				var pokemon: Dictionary = Dictionary(party[index])
				if _is_egg(pokemon):
					continue
				pokemon["happiness"] = mini(255, int(pokemon.get("happiness", 0)) + 1)
				party[index] = pokemon

	var hatched_species: Variant = null
	if int(wram.get("step_count", 0)) == 0x80:
		for index in range(party.size()):
			if typeof(party[index]) != TYPE_DICTIONARY:
				continue
			var pokemon: Dictionary = Dictionary(party[index])
			if not _is_egg(pokemon):
				continue
			pokemon["happiness"] = (int(pokemon.get("happiness", 0)) - 1) & 0xff
			if int(pokemon.get("happiness", 0)) == 0:
				var species := str(pokemon.get("species", pokemon.get("species_id", ""))).to_upper()
				pokemon["nickname"] = species
				pokemon["is_egg"] = false
				hatched_species = species
			party[index] = pokemon
			break

	var poison_result: Variant = null
	if int(wram.get("poison_step_count", 0)) >= POISON_STEP_INTERVAL:
		wram["poison_step_count"] = 0
		poison_result = _apply_poison_step(party)

	party_root["pokemon"] = party
	sram["party"] = party_root
	state["sram"] = sram
	state["wram"] = wram
	state["last_step_result"] = {
		"egg_hatched": hatched_species != null,
		"hatched_species": hatched_species,
		"poison_result": poison_result,
	}
	return Dictionary(state.get("last_step_result", {})).duplicate(true)

func process_daily_events(current_date: Dictionary) -> Dictionary:
	_normalize_state()
	var time_state: Dictionary = Dictionary(state.get("time", {}))
	var last_reset: Dictionary = Dictionary(time_state.get("last_daily_reset", {}))
	var target := _normalize_date(current_date)
	if _same_date(target, last_reset):
		state["last_daily_reset_result"] = {
			"reset": false,
			"date": target,
		}
		return Dictionary(state.get("last_daily_reset_result", {})).duplicate(true)

	var wram: Dictionary = Dictionary(state.get("wram", {}))
	var sram: Dictionary = Dictionary(state.get("sram", {}))
	wram["daily_reset_timer"] = 0
	wram["daily_flags1"] = 0
	wram["daily_flags2"] = 0
	wram["swarm_flags"] = 0
	wram["daily_rematch_flags"] = _zero_array(Array(wram.get("daily_rematch_flags", [])))
	wram["daily_phone_item_flags"] = _zero_array(Array(wram.get("daily_phone_item_flags", [])))
	wram["daily_phone_time_of_day_flags"] = _zero_array(Array(wram.get("daily_phone_time_of_day_flags", [])))
	var engine_flags: Dictionary = Dictionary(wram.get("engine_flags", {}))
	engine_flags["ENGINE_DAILY_BUG_CONTEST"] = false
	wram["engine_flags"] = engine_flags
	_clear_fruit_tree_flags(wram, sram)
	if bool(sram.get("mystery_gift_unlocked", false)):
		var mystery_gift: Dictionary = Dictionary(sram.get("mystery_gift", {}))
		mystery_gift["daily_partner_ids"] = []
		sram["mystery_gift"] = mystery_gift
	var kenji_timer := int(wram.get("wKenjiBreakTimer", 0))
	if kenji_timer > 0:
		kenji_timer -= 1
	if kenji_timer <= 0:
		kenji_timer = 3 + (_daily_rng_byte(target) & 0x03)
	wram["wKenjiBreakTimer"] = kenji_timer
	sram["current_date"] = target.duplicate(true)
	time_state["last_daily_reset"] = target.duplicate(true)
	time_state["year"] = int(target.get("year", 2000))
	time_state["month"] = int(target.get("month", 1))
	time_state["day"] = int(target.get("day", 1))
	state["time"] = time_state
	state["wram"] = wram
	state["sram"] = sram
	state["last_daily_reset_result"] = {
		"reset": true,
		"date": target,
		"kenji_break_timer": kenji_timer,
	}
	return Dictionary(state.get("last_daily_reset_result", {})).duplicate(true)

func format_price(value: int) -> String:
	var clamped := clampi(value, 0, MAX_MONEY)
	return "¥%06d" % clamped

func configure_shop(mart: String, items: Array, money: int, inventory: Dictionary = {}) -> Dictionary:
	_normalize_state()
	var shop_state: Dictionary = Dictionary(state.get("shop", {}))
	var sram: Dictionary = Dictionary(state.get("sram", {}))
	shop_state["mart"] = mart.strip_edges().to_upper()
	shop_state["items"] = _normalize_mart_items(items)
	shop_state["selection"] = 0
	shop_state["scroll"] = 0
	shop_state["last_result"] = {}
	sram["money"] = clampi(money, 0, MAX_MONEY)
	sram["items"] = inventory.duplicate(true)
	state["shop"] = shop_state
	state["sram"] = sram
	return shop_state.duplicate(true)

func paginate_shop(direction: String) -> Dictionary:
	var shop_state: Dictionary = Dictionary(state.get("shop", {}))
	var total_items := Array(shop_state.get("items", [])).size()
	var selection := clampi(int(shop_state.get("selection", 0)), 0, maxi(total_items - 1, 0))
	var scroll := clampi(int(shop_state.get("scroll", 0)), 0, maxi(total_items - 1, 0))
	if direction == "up":
		selection = maxi(0, selection - 1)
	elif direction == "down":
		selection = mini(maxi(total_items - 1, 0), selection + 1)
	if selection < scroll:
		scroll = selection
	elif selection >= scroll + MART_MENU_PAGE_SIZE:
		scroll = selection - MART_MENU_PAGE_SIZE + 1
	shop_state["selection"] = selection
	shop_state["scroll"] = scroll
	state["shop"] = shop_state
	return {
		"selection": selection,
		"scroll": scroll,
	}

func buy_selected(quantity: int) -> Dictionary:
	var shop_state: Dictionary = Dictionary(state.get("shop", {}))
	var items: Array = Array(shop_state.get("items", []))
	var selection := clampi(int(shop_state.get("selection", 0)), 0, maxi(items.size() - 1, 0))
	var item: Dictionary = Dictionary(items[selection]) if not items.is_empty() else {}
	var result := _buy_item(item, quantity)
	shop_state["last_result"] = result.duplicate(true)
	state["shop"] = shop_state
	return result

func sell_item(identifier: String, display_name: String, price: int, quantity: int) -> Dictionary:
	var result := _sell_item({
		"identifier": identifier,
		"display_name": display_name,
		"price": price,
	}, quantity)
	var shop_state: Dictionary = Dictionary(state.get("shop", {}))
	shop_state["last_result"] = result.duplicate(true)
	state["shop"] = shop_state
	return result

func _buy_item(item: Dictionary, quantity: int) -> Dictionary:
	var sram: Dictionary = Dictionary(state.get("sram", {}))
	if quantity <= 0:
		return {"success": false, "message": "Quantity must be positive.", "credited": 0}
	var price := int(item.get("price", 0))
	var total_cost := price * quantity
	if total_cost > int(sram.get("money", 0)):
		return {"success": false, "message": "You don't have enough money.", "credited": 0}
	if not _add_item(str(item.get("identifier", "")), quantity):
		return {"success": false, "message": "Your Pack is full.", "credited": 0}
	sram = Dictionary(state.get("sram", {}))
	sram["money"] = int(sram.get("money", 0)) - total_cost
	state["sram"] = sram
	return {"success": true, "message": format_price(total_cost), "credited": total_cost}

func _sell_item(item: Dictionary, quantity: int) -> Dictionary:
	var sell_price := maxi(0, int(item.get("price", 0)) / 2)
	if quantity <= 0:
		return {"success": false, "message": "Quantity must be positive.", "credited": 0}
	if sell_price <= 0:
		return {"success": false, "message": "We can't offer anything for that item.", "credited": 0}
	if _get_item_quantity(str(item.get("identifier", ""))) < quantity:
		return {"success": false, "message": "Looks like you don't have that many.", "credited": 0}
	if not _remove_item(str(item.get("identifier", "")), quantity):
		return {"success": false, "message": "Looks like you don't have that many.", "credited": 0}
	var sram: Dictionary = Dictionary(state.get("sram", {}))
	var payout := sell_price * quantity
	var starting_money := int(sram.get("money", 0))
	var new_money := mini(MAX_MONEY, starting_money + payout)
	sram["money"] = new_money
	state["sram"] = sram
	return {"success": true, "message": format_price(payout), "credited": new_money - starting_money}

func _add_item(identifier: String, quantity: int) -> bool:
	var normalized := identifier.strip_edges().to_upper()
	if normalized.is_empty() or normalized == "CANCEL":
		return false
	var sram: Dictionary = Dictionary(state.get("sram", {}))
	var items: Dictionary = Dictionary(sram.get("items", {}))
	var owned := int(items.get(normalized, 0))
	if owned + quantity > MAX_ITEM_STACK:
		return false
	items[normalized] = owned + quantity
	sram["items"] = items
	state["sram"] = sram
	return true

func _remove_item(identifier: String, quantity: int) -> bool:
	var normalized := identifier.strip_edges().to_upper()
	var sram: Dictionary = Dictionary(state.get("sram", {}))
	var items: Dictionary = Dictionary(sram.get("items", {}))
	var owned := int(items.get(normalized, 0))
	if owned < quantity:
		return false
	var remaining := owned - quantity
	if remaining <= 0:
		items.erase(normalized)
	else:
		items[normalized] = remaining
	sram["items"] = items
	state["sram"] = sram
	return true

func _get_item_quantity(identifier: String) -> int:
	var sram: Dictionary = Dictionary(state.get("sram", {}))
	var items: Dictionary = Dictionary(sram.get("items", {}))
	return int(items.get(identifier.strip_edges().to_upper(), 0))

func _apply_poison_step(party: Array) -> Variant:
	var damaged: Array[String] = []
	var fainted: Array[String] = []
	for index in range(party.size()):
		if typeof(party[index]) != TYPE_DICTIONARY:
			continue
		var pokemon: Dictionary = Dictionary(party[index])
		if not _is_poisoned(str(pokemon.get("status", ""))) or int(pokemon.get("hp", 0)) <= 0:
			continue
		var hp := int(pokemon.get("hp", 0))
		pokemon["hp"] = maxi(0, hp - 1)
		var name := str(pokemon.get("nickname", pokemon.get("species", ""))).to_upper()
		damaged.append(name)
		if int(pokemon.get("hp", 0)) == 0:
			fainted.append(name)
			pokemon["happiness"] = maxi(0, int(pokemon.get("happiness", 0)) - _poison_faint_happiness_penalty(int(pokemon.get("happiness", 0))))
		party[index] = pokemon
	if damaged.is_empty() and fainted.is_empty():
		return null
	return {
		"damagedNames": damaged,
		"faintedNames": fainted,
	}

func _normalize_state() -> void:
	state["time"] = Dictionary(state.get("time", {}))
	state["wram"] = Dictionary(state.get("wram", {}))
	state["sram"] = Dictionary(state.get("sram", {}))
	state["shop"] = Dictionary(state.get("shop", {}))
	var time_state: Dictionary = Dictionary(state.get("time", {}))
	time_state["time_of_day"] = _time_of_day(int(time_state.get("hour", 10)))
	state["time"] = time_state

func _normalize_mart_items(items: Array) -> Array:
	var normalized: Array[Dictionary] = []
	for item_value in items:
		if typeof(item_value) != TYPE_DICTIONARY:
			continue
		var item: Dictionary = Dictionary(item_value)
		normalized.append({
			"identifier": str(item.get("identifier", item.get("name", ""))).strip_edges().to_upper(),
			"displayName": str(item.get("displayName", item.get("display_name", item.get("identifier", "")))),
			"price": maxi(0, int(item.get("price", 0))),
			"quantity": maxi(0, int(item.get("quantity", 0))),
		})
	normalized.append({"identifier": "CANCEL", "displayName": "CANCEL", "price": 0})
	return normalized

func _clear_fruit_tree_flags(wram: Dictionary, sram: Dictionary) -> void:
	var wram_flags: Dictionary = Dictionary(wram.get("event_flags", {}))
	var sram_flags: Dictionary = Dictionary(sram.get("event_flags", {}))
	var keys_to_clear: Array[String] = []
	for key in wram_flags.keys():
		var flag := str(key)
		if flag.begins_with("FRUITTREE_") and flag.ends_with("_COLLECTED"):
			keys_to_clear.append(flag)
	for key in keys_to_clear:
		wram_flags.erase(key)
		sram_flags.erase(key)
	wram["event_flags"] = wram_flags
	sram["event_flags"] = sram_flags

func _merge_dictionary(base: Dictionary, overlay: Dictionary) -> Dictionary:
	var merged := base.duplicate(true)
	for key in overlay.keys():
		if typeof(merged.get(key)) == TYPE_DICTIONARY and typeof(overlay.get(key)) == TYPE_DICTIONARY:
			merged[key] = _merge_dictionary(Dictionary(merged.get(key)), Dictionary(overlay.get(key)))
		else:
			merged[key] = overlay.get(key)
	return merged

func _normalize_date(value: Dictionary) -> Dictionary:
	return {
		"year": int(value.get("year", 2000)),
		"month": clampi(int(value.get("month", 1)), 1, 12),
		"day": clampi(int(value.get("day", 1)), 1, 31),
	}

func _same_date(a: Dictionary, b: Dictionary) -> bool:
	return int(a.get("year", 0)) == int(b.get("year", 0)) and int(a.get("month", 0)) == int(b.get("month", 0)) and int(a.get("day", 0)) == int(b.get("day", 0))

func _daily_rng_byte(date: Dictionary) -> int:
	return (int(date.get("year", 0)) + int(date.get("month", 0)) * 17 + int(date.get("day", 0)) * 31) & 0xff

func _time_of_day(hour: int) -> String:
	if hour >= 4 and hour < 10:
		return "morning"
	if hour >= 10 and hour < 18:
		return "day"
	return "night"

func _increment_byte(value: int) -> int:
	return ((value & 0xff) + 1) & 0xff

func _zero_array(values: Array) -> Array[int]:
	var zeroed: Array[int] = []
	for _value in values:
		zeroed.append(0)
	return zeroed

func _is_egg(pokemon: Dictionary) -> bool:
	return bool(pokemon.get("is_egg", false)) or str(pokemon.get("nickname", "")).to_upper() == "EGG"

func _is_poisoned(status: String) -> bool:
	var normalized := status.strip_edges().to_upper()
	return normalized == "PSN" or normalized == "POISON" or normalized == "POISONED"

func _poison_faint_happiness_penalty(happiness: int) -> int:
	if happiness < 200:
		return 5
	return 10
