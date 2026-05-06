import { createInitialGameState } from "@pokecrystal/core/core/state";
import type { MenuUI } from "./types";
import { MenuState } from "./menu-state";
import type { AudioEngine } from "@pokecrystal/core/engine/systems/audio";
import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import { PartyMenuAction } from "@pokecrystal/core/core/enums/party-menu";
import { ItemEffect, ItemPocket } from "@pokecrystal/core/core/enums/item";
import { Surface } from "@pokecrystal/core/ui/game-engine";
import type { Item } from "@pokecrystal/core/core/models";

const HP_TILE_START = 0x60;
const HP_TILE_END = 0x6c;

const buildFontTiles = (): Record<number, Surface> => {
  const tiles: Record<number, Surface> = {};
  for (let id = HP_TILE_START; id < HP_TILE_END; id += 1) {
    tiles[id] = new Surface(8, 8);
  }
  return tiles;
};

const createMenuUi = (): MenuUI => ({
  screen: null,
  tileSize: 8,
  font: { renderText: jest.fn(), fontTiles: buildFontTiles() },
  drawWindow: jest.fn(),
});

describe("MenuState bag item use", () => {
  it("starts field item selection even without menu data loaded", () => {
    const gameState = createInitialGameState();
    gameState.sram.items.POTION = 1;

    const audioEngine = { playSound: jest.fn() } as unknown as AudioEngine;
    const menuState = new MenuState(createMenuUi(), gameState, audioEngine, null);
    menuState.currentMenu = "bag_menu";

    menuState.handleInput(new gameEngine.event.Event("keydown", { key: "z", code: "KeyZ" }));
    menuState.handleInput(new gameEngine.event.Event("keydown", { key: "z", code: "KeyZ" }));

    expect(menuState.currentMenu).toBe("pokemon_menu");
    expect(menuState.pokemonMenu?.getAction()).toBe(PartyMenuAction.HEALING_ITEM);
  });

  it("heals the selected party member from the bag flow", () => {
    const gameState = createInitialGameState();
    gameState.sram.items.POTION = 1;
    gameState.sram.party.pokemon[0] = {
      species: { id: "CYNDAQUIL" },
      nickname: "CYNDAQUIL",
      hp: 6,
      max_hp: 22,
      status: undefined,
      sleep_turns: 0,
      confusion_turns: 0,
      level: 6,
      item: undefined,
      moves: [],
    } as never;

    const audioEngine = { playSound: jest.fn() } as unknown as AudioEngine;
    const menuState = new MenuState(createMenuUi(), gameState, audioEngine, null);
    menuState.currentMenu = "bag_menu";

    menuState.handleInput(new gameEngine.event.Event("keydown", { key: "z", code: "KeyZ" }));
    menuState.handleInput(new gameEngine.event.Event("keydown", { key: "z", code: "KeyZ" }));
    menuState.handleInput(new gameEngine.event.Event("keydown", { key: "z", code: "KeyZ" }));

    expect(gameState.sram.party.pokemon[0]?.hp).toBe(22);
    expect(gameState.sram.items.POTION).toBeUndefined();
  });

  it("heals from the bag flow when loader item data omits the effect like the live runtime", () => {
    const gameState = createInitialGameState();
    gameState.sram.items.POTION = 1;
    gameState.sram.party.pokemon[0] = {
      species: { id: "CYNDAQUIL" },
      nickname: "CYNDAQUIL",
      hp: 6,
      max_hp: 22,
      status: undefined,
      sleep_turns: 0,
      confusion_turns: 0,
      level: 6,
      item: undefined,
      moves: [],
    } as never;

    const audioEngine = { playSound: jest.fn() } as unknown as AudioEngine;
    const itemData = new Map<string, Item>([
      ["POTION", {
        name: "POTION",
        pocket: ItemPocket.ITEM,
        price: 300,
        description: "Restores #MON HP by 20.",
        effect: ItemEffect.NONE,
        parameter: 20,
        script_name: "POTION",
        held_effect: "HELD_NONE",
        property: "CANT_SELECT",
        field_menu: "ITEMMENU_PARTY",
        battle_menu: "ITEMMENU_PARTY",
      }],
    ]);
    const menuState = new MenuState(createMenuUi(), gameState, audioEngine, null, { itemData } as never);
    menuState.currentMenu = "bag_menu";

    menuState.handleInput(new gameEngine.event.Event("keydown", { key: "z", code: "KeyZ" }));
    menuState.handleInput(new gameEngine.event.Event("keydown", { key: "z", code: "KeyZ" }));
    menuState.handleInput(new gameEngine.event.Event("keydown", { key: "z", code: "KeyZ" }));

    expect(gameState.sram.party.pokemon[0]?.hp).toBe(22);
    expect(gameState.sram.items.POTION).toBeUndefined();
  });

  it("keeps the item when field use has no effect on a fully healed party member", () => {
    const gameState = createInitialGameState();
    gameState.sram.items.POTION = 1;
    gameState.sram.party.pokemon[0] = {
      species: { id: "CYNDAQUIL" },
      nickname: "CYNDAQUIL",
      hp: 20,
      max_hp: 20,
      status: undefined,
      sleep_turns: 0,
      confusion_turns: 0,
      level: 6,
      item: undefined,
      moves: [],
    } as never;

    const audioEngine = { playSound: jest.fn() } as unknown as AudioEngine;
    const menuState = new MenuState(createMenuUi(), gameState, audioEngine, null);
    menuState.currentMenu = "bag_menu";

    menuState.handleInput(new gameEngine.event.Event("keydown", { key: "z", code: "KeyZ" }));
    menuState.handleInput(new gameEngine.event.Event("keydown", { key: "z", code: "KeyZ" }));
    menuState.handleInput(new gameEngine.event.Event("keydown", { key: "z", code: "KeyZ" }));

    menuState.handleInput(new gameEngine.event.Event("keydown", { key: "z", code: "KeyZ" }));

    expect(gameState.sram.party.pokemon[0]?.hp).toBe(20);
    expect(gameState.sram.items.POTION).toBe(1);
  });

  it("keeps the item when a held item is canceled on an egg", () => {
    const gameState = createInitialGameState();
    gameState.sram.items.POTION = 1;
    gameState.sram.party.pokemon[0] = {
      species: { id: "EGG" },
      nickname: "EGG",
      hp: 0,
      max_hp: 10,
      status: undefined,
      sleep_turns: 0,
      confusion_turns: 0,
      level: 1,
      item: undefined,
      moves: [],
    } as never;

    const audioEngine = { playSound: jest.fn() } as unknown as AudioEngine;
    const menuState = new MenuState(createMenuUi(), gameState, audioEngine, null);
    menuState.currentMenu = "bag_menu";

    menuState.handleInput(new gameEngine.event.Event("keydown", { key: "z", code: "KeyZ" }));
    menuState.handleInput(new gameEngine.event.Event("keydown", { key: "z", code: "KeyZ" }));
    menuState.handleInput(new gameEngine.event.Event("keydown", { key: "z", code: "KeyZ" }));

    menuState.handleInput(new gameEngine.event.Event("keydown", { key: "z", code: "KeyZ" }));

    expect(gameState.sram.party.pokemon[0]?.hp).toBe(0);
    expect(gameState.sram.items.POTION).toBe(1);
  });

  it("routes non-held key item use through the non-field branch when no key-use callback is provided", () => {
    const gameState = createInitialGameState();
    gameState.sram.key_items.BICYCLE = 1;

    const audioEngine = { playSound: jest.fn() } as unknown as AudioEngine;
    const menuState = new MenuState(createMenuUi(), gameState, audioEngine, null);
    menuState.currentMenu = "bag_menu";

    menuState.handleInput(new gameEngine.event.Event("keydown", { key: gameEngine.K_RIGHT, code: gameEngine.K_RIGHT }));
    menuState.handleInput(new gameEngine.event.Event("keydown", { key: "z", code: "KeyZ" }));
    menuState.handleInput(new gameEngine.event.Event("keydown", { key: "z", code: "KeyZ" }));

    expect(gameState.sram.key_items.BICYCLE).toBe(1);
    expect(menuState.currentMenu).toBe("bag_menu");
  });

  it("uses SQUIRTBOTTLE from the key item pocket through the overworld key item callback", () => {
    const gameState = createInitialGameState();
    gameState.sram.key_items.SQUIRTBOTTLE = 1;

    const audioEngine = { playSound: jest.fn() } as unknown as AudioEngine;
    const useKeyItem = jest.fn(() => true);
    const showFieldMoveText = jest.fn();
    const menuState = new MenuState(
      createMenuUi(),
      gameState,
      audioEngine,
      null,
      null,
      null,
      {
        use_key_item: useKeyItem,
        _show_field_move_text: showFieldMoveText,
      } as unknown as ConstructorParameters<typeof MenuState>[6],
    );
    menuState.currentMenu = "bag_menu";

    menuState.handleInput(new gameEngine.event.Event("keydown", { key: gameEngine.K_RIGHT, code: gameEngine.K_RIGHT }));
    menuState.handleInput(new gameEngine.event.Event("keydown", { key: gameEngine.K_RIGHT, code: gameEngine.K_RIGHT }));
    menuState.handleInput(new gameEngine.event.Event("keydown", { key: "z", code: "KeyZ" }));
    const result = menuState.handleInput(new gameEngine.event.Event("keydown", { key: "z", code: "KeyZ" }));

    expect(useKeyItem).toHaveBeenCalledWith("SQUIRTBOTTLE");
    expect(showFieldMoveText).not.toHaveBeenCalled();
    expect(result).toBe("close_menu");
    expect(gameState.sram.key_items.SQUIRTBOTTLE).toBe(1);
  });

  it("returns to bag menu after cancelling held-item action selection and can still use later", () => {
    const gameState = createInitialGameState();
    gameState.sram.items.POTION = 1;
    gameState.sram.party.pokemon[0] = {
      species: { id: "CYNDAQUIL" },
      nickname: "CYNDAQUIL",
      hp: 6,
      max_hp: 22,
      status: undefined,
      sleep_turns: 0,
      confusion_turns: 0,
      level: 6,
      item: undefined,
      moves: [],
    } as never;

    const audioEngine = { playSound: jest.fn() } as unknown as AudioEngine;
    const menuState = new MenuState(createMenuUi(), gameState, audioEngine, null);
    menuState.currentMenu = "bag_menu";

    menuState.handleInput(new gameEngine.event.Event("keydown", { key: "z", code: "KeyZ" }));
    menuState.handleInput(new gameEngine.event.Event("keydown", { key: "Backspace", code: "Backspace" }));
    menuState.handleInput(new gameEngine.event.Event("keydown", { key: "z", code: "KeyZ" }));
    menuState.handleInput(new gameEngine.event.Event("keydown", { key: "z", code: "KeyZ" }));
    menuState.handleInput(new gameEngine.event.Event("keydown", { key: "z", code: "KeyZ" }));

    expect(gameState.sram.party.pokemon[0]?.hp).toBe(22);
    expect(gameState.sram.items.POTION).toBeUndefined();
  });

  it("treats B in the pack action menu like QUIT instead of closing the pack", () => {
    const gameState = createInitialGameState();
    gameState.sram.items.POTION = 1;

    const audioEngine = { playSound: jest.fn() } as unknown as AudioEngine;
    const menuState = new MenuState(createMenuUi(), gameState, audioEngine, null);
    menuState.currentMenu = "bag_menu";

    menuState.handleInput(new gameEngine.event.Event("keydown", { key: "z", code: "KeyZ" }));
    const result = menuState.handleInput(
      new gameEngine.event.Event("keydown", { key: "x", code: "KeyX", button: "b", is_press: true }),
    );

    expect(result).toBeNull();
    expect(menuState.currentMenu).toBe("bag_menu");
    expect((menuState as unknown as { bagMenu: { getMode: () => string } }).bagMenu.getMode()).toBe("list");
  });
});
