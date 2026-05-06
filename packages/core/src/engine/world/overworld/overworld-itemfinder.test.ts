import { OverworldEngine } from "./overworld";
import { createInitialGameState } from "@pokecrystal/core/core/state";
import { EventManager } from "@pokecrystal/core/engine/world/events";

const useKeyItem = (OverworldEngine as unknown as { prototype: { use_key_item: Function } })
  .prototype.use_key_item;
const overworldPrototype = OverworldEngine.prototype as unknown as {
  _use_squirtbottle: Function;
  _can_use_squirtbottle_on_facing_object: Function;
  _squirtbottle_facing_object: Function;
  _squirtbottle_object_covers_tile: Function;
  _show_squirtbottle_nothing_text: Function;
};

describe("OverworldEngine itemfinder text resolution", () => {
  it("throws instead of prettifying unknown itemfinder labels", () => {
    const overworld = Object.create(OverworldEngine.prototype) as OverworldEngine & {
      data_loader?: null;
      _resolve_itemfinder_text: (label: string) => string;
    };
    overworld.data_loader = null;

    expect(() => overworld._resolve_itemfinder_text("TotallyMissingItemfinderText")).toThrow(
      "Missing ASM itemfinder text for label 'TotallyMissingItemfinderText'.",
    );
  });
});

describe("OverworldEngine key item dispatch", () => {
  it("normalizes rod names before starting fishing", () => {
    const stub = {
      handle_fishing: jest.fn(() => true),
      _canonical_key_item_name: OverworldEngine.prototype._canonical_key_item_name,
      _script_key_item_name: OverworldEngine.prototype._script_key_item_name,
    };

    expect(useKeyItem.call(stub, "Old Rod")).toBe(true);
    expect(useKeyItem.call(stub, "SUPER_ROD")).toBe(true);

    expect(stub.handle_fishing).toHaveBeenNthCalledWith(1, "OLD_ROD");
    expect(stub.handle_fishing).toHaveBeenNthCalledWith(2, "SUPER_ROD");
  });

  it("opens town map from TOWN MAP key item aliases", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const events: string[] = [];
    eventManager.on("show_town_map", (event) => {
      events.push(String(event.data.source));
    });
    const stub = {
      event_manager: eventManager,
      _canonical_key_item_name: OverworldEngine.prototype._canonical_key_item_name,
      _script_key_item_name: OverworldEngine.prototype._script_key_item_name,
      _use_town_map: OverworldEngine.prototype._use_town_map,
    };

    expect(useKeyItem.call(stub, "TOWN_MAP")).toBe(true);

    expect(events).toEqual(["key_item"]);
  });

  it("displays COIN CASE and BLUE CARD balances", () => {
    const gameState = createInitialGameState();
    gameState.sram.coins = 1234;
    gameState.wram.blue_card_balance = 12;
    const eventManager = new EventManager(gameState);
    const events: Array<{ name: string; value: unknown }> = [];
    eventManager.on("show_coin_case_balance", (event) => {
      events.push({ name: event.name, value: event.data.overlay });
    });
    eventManager.on("show_blue_card_balance", (event) => {
      events.push({ name: event.name, value: event.data.overlay });
    });
    const stub = {
      game_state: gameState,
      event_manager: eventManager,
      _canonical_key_item_name: OverworldEngine.prototype._canonical_key_item_name,
      _script_key_item_name: OverworldEngine.prototype._script_key_item_name,
      _use_coin_case: OverworldEngine.prototype._use_coin_case,
      _use_blue_card: OverworldEngine.prototype._use_blue_card,
    };

    expect(useKeyItem.call(stub, "COIN_CASE")).toBe(true);
    expect(useKeyItem.call(stub, "BLUE CARD")).toBe(true);

    expect(events).toEqual([
      {
        name: "show_coin_case_balance",
        value: { width: 7, height: 1, x: 11, y: 0, label: "COIN", value: 1234 },
      },
      {
        name: "show_blue_card_balance",
        value: { width: 7, height: 1, x: 11, y: 0, label: "POINT", value: 12 },
      },
    ]);
  });

  it("shows itemfinder text through the async dialogue path and closes the textbox", async () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const events: string[] = [];
    for (const eventName of ["open_text", "show_text", "wait_for_input", "close_text"]) {
      eventManager.on(eventName, (event) => {
        events.push(event.name);
      });
    }
    const stub = {
      event_manager: eventManager,
      data_loader: {
        get_text: (label: string) => label,
      },
      _canonical_key_item_name: OverworldEngine.prototype._canonical_key_item_name,
      _script_key_item_name: OverworldEngine.prototype._script_key_item_name,
      _use_itemfinder: OverworldEngine.prototype._use_itemfinder,
      _display_itemfinder_text: OverworldEngine.prototype._display_itemfinder_text,
      _resolve_itemfinder_text: OverworldEngine.prototype._resolve_itemfinder_text,
      _find_hidden_item_event: jest.fn(() => null),
      _play_itemfinder_sound: jest.fn(),
      _wait_for_dialogue_render: jest.fn(() => {
        throw new Error("sync dialogue wait used");
      }),
      _wait_for_dialogue_ack: jest.fn(() => {
        throw new Error("sync dialogue wait used");
      }),
      _wait_for_dialogue_closed: jest.fn(() => {
        throw new Error("sync dialogue wait used");
      }),
      _wait_for_dialogue_render_async: jest.fn(async () => undefined),
      _wait_for_dialogue_ack_async: jest.fn(async () => undefined),
      _wait_for_dialogue_closed_async: jest.fn(async () => undefined),
    };

    await expect(useKeyItem.call(stub, "ITEMFINDER")).resolves.toBe(true);

    expect(events).toEqual(["open_text", "show_text", "wait_for_input", "close_text"]);
    expect(stub._wait_for_dialogue_render).not.toHaveBeenCalled();
    expect(stub._wait_for_dialogue_ack).not.toHaveBeenCalled();
    expect(stub._wait_for_dialogue_closed).not.toHaveBeenCalled();
  });

  it("runs the Route 36 weird tree script when SQUIRTBOTTLE faces Sudowoodo", () => {
    const weirdTree = {
      x: 35,
      y: 9,
      prevX: 35,
      prevY: 9,
      collisionStride: 2,
      objectIndex: 3,
      event: {
        spritemovedata: "SPRITEMOVEDATA_SUDOWOODO",
      },
    };
    const scriptRunner = { run: jest.fn() };
    const stub = {
      current_map_name: "Route36",
      TILES_PER_COLLISION: 2,
      npcs: [weirdTree],
      script_runner: scriptRunner,
      get_facing_tile_coords: jest.fn(() => [35, 9] as [number, number]),
      _canonical_key_item_name: OverworldEngine.prototype._canonical_key_item_name,
      _script_key_item_name: OverworldEngine.prototype._script_key_item_name,
      _use_squirtbottle: overworldPrototype._use_squirtbottle,
      _can_use_squirtbottle_on_facing_object: overworldPrototype._can_use_squirtbottle_on_facing_object,
      _squirtbottle_facing_object: overworldPrototype._squirtbottle_facing_object,
      _squirtbottle_object_covers_tile: overworldPrototype._squirtbottle_object_covers_tile,
      _show_squirtbottle_nothing_text: jest.fn(),
    };

    expect(useKeyItem.call(stub, "SQUIRTBOTTLE")).toBe(true);

    expect(scriptRunner.run).toHaveBeenCalledWith("WateredWeirdTreeScript");
    expect(stub._show_squirtbottle_nothing_text).not.toHaveBeenCalled();
  });

  it("uses the Squirt Bottle nothing text instead of the generic cant-use text off Sudowoodo", async () => {
    const stub = {
      current_map_name: "Route35",
      TILES_PER_COLLISION: 2,
      npcs: [],
      script_runner: { run: jest.fn() },
      get_facing_tile_coords: jest.fn(() => [0, 0] as [number, number]),
      _canonical_key_item_name: OverworldEngine.prototype._canonical_key_item_name,
      _script_key_item_name: OverworldEngine.prototype._script_key_item_name,
      _use_squirtbottle: overworldPrototype._use_squirtbottle,
      _can_use_squirtbottle_on_facing_object: overworldPrototype._can_use_squirtbottle_on_facing_object,
      _squirtbottle_facing_object: overworldPrototype._squirtbottle_facing_object,
      _squirtbottle_object_covers_tile: overworldPrototype._squirtbottle_object_covers_tile,
      _show_squirtbottle_nothing_text: overworldPrototype._show_squirtbottle_nothing_text,
      _show_field_move_text: jest.fn(() => {
        throw new Error("sync dialogue wait used");
      }),
      _show_field_move_text_async: jest.fn(async () => undefined),
    };

    await expect(useKeyItem.call(stub, "SQUIRT_BOTTLE")).resolves.toBe(true);

    expect(stub._show_field_move_text_async).toHaveBeenCalledWith("_SquirtbottleNothingText");
    expect(stub._show_field_move_text).not.toHaveBeenCalled();
    expect(stub.script_runner.run).not.toHaveBeenCalled();
  });
});
