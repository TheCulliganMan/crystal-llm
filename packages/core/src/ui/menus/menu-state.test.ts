import { createInitialGameState } from "../../core/state";
import { MoveName } from "../../core/enums/move";
import { createTestPokemon } from "../../engine/world/story-events/test-utils";
import type { AudioEngine } from "../../engine/systems/audio";
import { gameEngine, Surface } from "../game-engine";
import { TextUI } from "../text-ui";
import { MenuState } from "./menu-state";
import type { MenuUI } from "./types";

const createMenuUi = (): MenuUI => ({
  screen: new Surface(160, 144),
  tileSize: 8,
  font: { renderText: jest.fn() },
  drawWindow: jest.fn(),
  drawTextBox: jest.fn(),
});

const createPokemonMenuUi = (): MenuUI => {
  const fontTiles: Record<number, Surface> = {};
  for (let tileId = 0; tileId <= 0xff; tileId += 1) {
    fontTiles[tileId] = new Surface(8, 8);
  }
  return {
    ...createMenuUi(),
    font: {
      renderText: jest.fn(),
      fontTiles,
      font_tiles: fontTiles,
    },
  } as unknown as MenuUI;
};

const press = (menuState: MenuState, button: "a" | "b" | "down"): string | null => {
  const key = button === "a" ? "KeyZ" : button === "b" ? "KeyX" : gameEngine.K_DOWN;
  const payload = button === "down"
    ? { key, code: key }
    : { key, code: key, button, is_press: true };
  const result = menuState.handleInput(new gameEngine.event.Event("keydown", payload));
  const releasePayload = button === "down"
    ? { key, code: key }
    : { key, code: key, button, is_press: false };
  menuState.handleInput(new gameEngine.event.Event("keyup", releasePayload));
  return result;
};

describe("MenuState save text formatting", () => {
  it("throws instead of prettifying missing save-text labels", async () => {
    jest.resetModules();
    jest.doMock("../../core/asm-text-loader", () => ({
      asmTextLoader: { get: () => "" },
    }));

    const { MenuState } = await import("./menu-state");

    expect(() =>
      (
        MenuState.prototype as unknown as {
          formatSaveText: (this: { gameState: { sram: { player_name: string; rival_name: string } } }, label: string) => string;
        }
      ).formatSaveText.call(
        { gameState: { sram: { player_name: "RED", rival_name: "BLUE" } } },
        "MissingSaveTextLabel",
      ),
    ).toThrow("Missing ASM text for label 'MissingSaveTextLabel'.");
  });

  it("still formats known ASM save text with player substitutions", async () => {
    jest.resetModules();
    jest.doMock("../../core/asm-text-loader", () => ({
      asmTextLoader: { get: (label: string) => (label === "SaveAskOverwriteText" ? "<PLAYER> saved the game." : "") },
    }));

    const { MenuState } = await import("./menu-state");

    expect(
      (
        MenuState.prototype as unknown as {
          formatSaveText: (this: { gameState: { sram: { player_name: string; rival_name: string } } }, label: string) => string;
        }
      ).formatSaveText.call(
        { gameState: { sram: { player_name: "RED", rival_name: "BLUE" } } },
        "SaveAskOverwriteText",
      ),
    ).toBe("RED saved the game.");
  });
});

describe("MenuState register text formatting", () => {
  it("throws instead of falling back to synthetic register-item text", async () => {
    jest.resetModules();
    jest.doMock("../../core/asm-text-loader", () => ({
      asmTextLoader: { get: () => "" },
    }));

    const { MenuState } = await import("./menu-state");

    expect(() =>
      (
        MenuState.prototype as unknown as {
          showRegisterText: (
            this: {
              gameState: { sram: { player_name: string; rival_name: string } };
              dialogue: { open: (text: string) => void };
              dialogueVisible: boolean;
            },
            label: string,
            buffers?: Record<string, string>,
          ) => void;
        }
      ).showRegisterText.call(
        {
          gameState: { sram: { player_name: "RED", rival_name: "BLUE" } },
          dialogue: { open: jest.fn() },
          dialogueVisible: false,
        },
        "_RegisteredItemText",
        { STRING_BUFFER_2: "BICYCLE" },
      ),
    ).toThrow("Missing ASM text for label '_RegisteredItemText'.");
  });
});

describe("MenuState text-only draw path", () => {
  it("skips start-menu pixel drawing when the UI is pure text", () => {
    const ui = new TextUI(160, 144, 1, null, false, 0) as TextUI & {
      font: NonNullable<TextUI["font"]>;
    };
    const gameState = createInitialGameState();
    const audioEngine = {
      playSound: jest.fn(),
      play_sound: jest.fn(),
    } as unknown as ConstructorParameters<typeof MenuState>[2];
    const menuState = new MenuState(ui as unknown as ConstructorParameters<typeof MenuState>[0], gameState, audioEngine, null);
    const startMenuDraw = jest.spyOn((menuState as unknown as { startMenu: { draw: () => void } }).startMenu, "draw");

    menuState.draw();

    expect(startMenuDraw).not.toHaveBeenCalled();
    expect(ui.getSnapshot()).toMatchObject({
      viewportTitle: "Start Menu",
      viewportLines: expect.arrayContaining(["START MENU"]),
    });
  });

  it("opens the Pokedex from the text TUI without an injected species loader", () => {
    const ui = new TextUI(160, 144, 1, null, false, 0) as TextUI & {
      font: NonNullable<TextUI["font"]>;
    };
    const gameState = createInitialGameState();
    gameState.wram.engine_flags.ENGINE_POKEDEX = true;
    const audioEngine = {
      playSound: jest.fn(),
      play_sound: jest.fn(),
    } as unknown as ConstructorParameters<typeof MenuState>[2];
    const menuState = new MenuState(
      ui as unknown as ConstructorParameters<typeof MenuState>[0],
      gameState,
      audioEngine,
      null,
    );

    const result = menuState.handleInput(
      new gameEngine.event.Event("keydown", { button: "a", is_press: true })
    );
    menuState.draw();

    expect(result).toBe("pokedex");
    expect(menuState.currentMenu).toBe("pokedex");
    expect(ui.getSnapshot()).toMatchObject({
      viewportTitle: "Pokedex",
      viewportLines: expect.arrayContaining(["POKEDEX MAIN"]),
    });
  });
});

describe("MenuState reset behavior", () => {
  it("clears the stored start-menu cursor on reset", () => {
    const gameState = createInitialGameState();
    gameState.wram.start_menu_cursor = 3;
    const audioEngine = {
      playSound: jest.fn(),
      play_sound: jest.fn(),
    } as unknown as ConstructorParameters<typeof MenuState>[2];

    const menuState = new MenuState(new TextUI(160, 144, 1, null, false, 0) as unknown as ConstructorParameters<typeof MenuState>[0], gameState, audioEngine, null);

    expect(gameState.wram.start_menu_cursor).toBe(0);

    gameState.wram.start_menu_cursor = 2;
    menuState.reset();

    expect(gameState.wram.start_menu_cursor).toBe(0);
    expect((menuState as unknown as { startMenu: { cursorIndex: number } }).startMenu.cursorIndex).toBe(0);
  });
});

describe("MenuState pokemon stats return", () => {
  const createPokemonMenuState = () => {
    const gameState = createInitialGameState();
    gameState.sram.party.pokemon[0] = createTestPokemon("CYNDAQUIL", 8);
    gameState.sram.party.pokemon[1] = createTestPokemon("TOTODILE", 9);
    const audioEngine = {
      playSound: jest.fn(),
      play_sound: jest.fn(),
    } as unknown as ConstructorParameters<typeof MenuState>[2];
    const menuState = new MenuState(createPokemonMenuUi(), gameState, audioEngine, null);
    menuState.currentMenu = "pokemon_menu";
    return { gameState, menuState };
  };

  it("reinitializes the party menu after leaving stats, matching StartMenu_Pokemon .choosemenu", () => {
    const { menuState } = createPokemonMenuState();

    press(menuState, "a");
    press(menuState, "a");

    expect(menuState.currentMenu).toBe("pokemon_stats");

    const pokemonMenu = menuState.pokemonMenu;
    expect(pokemonMenu).not.toBeNull();

    // This is the stale action/menu state the ASM avoids by jumping back through
    // StartMenu_Pokemon .choosemenu after OpenPartyStats returns 0.
    (pokemonMenu as unknown as { mode: string }).mode = "give_take";
    (pokemonMenu as unknown as { giveTakeIndex: number }).giveTakeIndex = 1;

    press(menuState, "b");

    expect(menuState.currentMenu).toBe("pokemon_menu");
    expect(pokemonMenu?.getMode()).toBe("list");
    expect(pokemonMenu?.getGiveTakeIndex()).toBe(0);
    expect(pokemonMenu?.getSubmenuChoices()).toHaveLength(0);
  });

  it("uses B from stats to return to the pokemon list instead of closing the pokemon menu", () => {
    const { menuState } = createPokemonMenuState();

    press(menuState, "a");
    press(menuState, "a");

    expect(menuState.currentMenu).toBe("pokemon_stats");

    const result = press(menuState, "b");

    expect(result).toBeNull();
    expect(menuState.currentMenu).toBe("pokemon_menu");
    expect(menuState.pokemonMenu?.getMode()).toBe("list");
  });
});

describe("MenuState pokemon HM menu actions", () => {
  it.each([
    MoveName.CUT,
    MoveName.FLY,
    MoveName.SURF,
    MoveName.STRENGTH,
    MoveName.FLASH,
    MoveName.WATERFALL,
    MoveName.WHIRLPOOL,
  ])("routes %s through the overworld HM menu hook and closes on success", (moveName) => {
    const gameState = createInitialGameState();
    gameState.sram.party.pokemon[0] = createTestPokemon("LAPRAS", 131, {
      moves: [{ name: moveName, current_pp: 15 }],
    });
    const audioEngine = {
      playSound: jest.fn(),
      play_sound: jest.fn(),
    } as unknown as ConstructorParameters<typeof MenuState>[2];
    const useHmFromMenu = jest.fn(() => true);
    const menuState = new MenuState(
      createPokemonMenuUi(),
      gameState,
      audioEngine,
      null,
      null,
      null,
      { use_hm_from_menu: useHmFromMenu } as unknown as ConstructorParameters<typeof MenuState>[6],
    );
    menuState.currentMenu = "pokemon_menu";

    press(menuState, "a");
    const result = press(menuState, "a");

    expect(useHmFromMenu).toHaveBeenCalledWith(moveName, expect.objectContaining({ nickname: "LAPRAS" }));
    expect(result).toBe("close_menu");
  });

  it("closes the party menu immediately when an async field move starts", () => {
    const gameState = createInitialGameState();
    gameState.sram.party.pokemon[0] = createTestPokemon("LAPRAS", 131, {
      moves: [{ name: MoveName.CUT, current_pp: 15 }],
    });
    const audioEngine = {
      playSound: jest.fn(),
      play_sound: jest.fn(),
    } as unknown as ConstructorParameters<typeof MenuState>[2];
    const useHmFromMenu = jest.fn(() => new Promise<boolean>(() => undefined));
    const menuState = new MenuState(
      createPokemonMenuUi(),
      gameState,
      audioEngine,
      null,
      null,
      null,
      { use_hm_from_menu: useHmFromMenu } as unknown as ConstructorParameters<typeof MenuState>[6],
    );
    menuState.currentMenu = "pokemon_menu";

    press(menuState, "a");
    const result = press(menuState, "a");

    expect(useHmFromMenu).toHaveBeenCalledWith(MoveName.CUT, expect.objectContaining({ nickname: "LAPRAS" }));
    expect(result).toBe("close_menu");
  });

  it.each([
    [MoveName.DIG, "handle_dig", true],
    [MoveName.HEADBUTT, "handle_headbutt", true],
    [MoveName.SWEET_SCENT, "handle_sweet_scent", true],
    [MoveName.TELEPORT, "handle_teleport", false],
    [MoveName.ROCK_SMASH, "handle_rock_smash", true],
  ])("routes %s through its overworld field-move hook and closes on success", (moveName, handlerName, passesPokemon) => {
    const gameState = createInitialGameState();
    gameState.sram.party.pokemon[0] = createTestPokemon("LAPRAS", 131, {
      moves: [{ name: moveName, current_pp: 15 }],
    });
    const audioEngine = {
      playSound: jest.fn(),
      play_sound: jest.fn(),
    } as unknown as ConstructorParameters<typeof MenuState>[2];
    const handler = jest.fn(() => true);
    const menuState = new MenuState(
      createPokemonMenuUi(),
      gameState,
      audioEngine,
      null,
      null,
      null,
      { [handlerName]: handler } as unknown as ConstructorParameters<typeof MenuState>[6],
    );
    menuState.currentMenu = "pokemon_menu";

    press(menuState, "a");
    const result = press(menuState, "a");

    if (passesPokemon) {
      expect(handler).toHaveBeenCalledWith(expect.objectContaining({ nickname: "LAPRAS" }));
    } else {
      expect(handler).toHaveBeenCalledWith();
    }
    expect(result).toBe("close_menu");
  });

  it.each([
    [MoveName.DIG, "handle_dig", true],
    [MoveName.HEADBUTT, "handle_headbutt", true],
    [MoveName.SWEET_SCENT, "handle_sweet_scent", true],
    [MoveName.TELEPORT, "handle_teleport", false],
    [MoveName.ROCK_SMASH, "handle_rock_smash", true],
  ])("closes the party menu immediately when async %s starts", (moveName, handlerName, passesPokemon) => {
    const gameState = createInitialGameState();
    gameState.sram.party.pokemon[0] = createTestPokemon("LAPRAS", 131, {
      moves: [{ name: moveName, current_pp: 15 }],
    });
    const audioEngine = {
      playSound: jest.fn(),
      play_sound: jest.fn(),
    } as unknown as ConstructorParameters<typeof MenuState>[2];
    const handler = jest.fn(() => new Promise<boolean>(() => undefined));
    const menuState = new MenuState(
      createPokemonMenuUi(),
      gameState,
      audioEngine,
      null,
      null,
      null,
      { [handlerName]: handler } as unknown as ConstructorParameters<typeof MenuState>[6],
    );
    menuState.currentMenu = "pokemon_menu";

    press(menuState, "a");
    const result = press(menuState, "a");

    if (passesPokemon) {
      expect(handler).toHaveBeenCalledWith(expect.objectContaining({ nickname: "LAPRAS" }));
    } else {
      expect(handler).toHaveBeenCalledWith();
    }
    expect(result).toBe("close_menu");
  });
});

describe("MenuState key item field actions", () => {
  it("closes the menu immediately when an async key item action starts", () => {
    const gameState = createInitialGameState();
    const audioEngine = {
      playSound: jest.fn(),
      play_sound: jest.fn(),
    } as unknown as ConstructorParameters<typeof MenuState>[2];
    const useKeyItem = jest.fn(() => new Promise<boolean>(() => undefined));
    const showFieldMoveText = jest.fn();
    const menuState = new MenuState(
      createPokemonMenuUi(),
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

    (
      menuState as unknown as {
        handleKeyItemUse: (itemName: string) => void;
      }
    ).handleKeyItemUse("OLD_ROD");

    expect(useKeyItem).toHaveBeenCalledWith("OLD_ROD");
    expect(showFieldMoveText).not.toHaveBeenCalled();
    expect(menuState.consumeCloseRequest()).toBe(true);
  });

  it("uses async field text and closes the menu when a key item cannot be used", () => {
    const gameState = createInitialGameState();
    const audioEngine = {
      playSound: jest.fn(),
      play_sound: jest.fn(),
    } as unknown as ConstructorParameters<typeof MenuState>[2];
    const useKeyItem = jest.fn(() => false);
    const showFieldMoveText = jest.fn(() => {
      throw new Error("sync dialogue wait used");
    });
    const showFieldMoveTextAsync = jest.fn(async () => undefined);
    const menuState = new MenuState(
      createPokemonMenuUi(),
      gameState,
      audioEngine,
      null,
      null,
      null,
      {
        use_key_item: useKeyItem,
        _show_field_move_text: showFieldMoveText,
        _show_field_move_text_async: showFieldMoveTextAsync,
      } as unknown as ConstructorParameters<typeof MenuState>[6],
    );

    (
      menuState as unknown as {
        handleKeyItemUse: (itemName: string) => void;
      }
    ).handleKeyItemUse("BICYCLE");

    expect(useKeyItem).toHaveBeenCalledWith("BICYCLE");
    expect(showFieldMoveText).not.toHaveBeenCalled();
    expect(showFieldMoveTextAsync).toHaveBeenCalledWith("CantUseItemText");
    expect(menuState.consumeCloseRequest()).toBe(true);
  });
});

describe("MenuState async save flow", () => {
  beforeEach(() => {
    jest.spyOn(gameEngine.display, "get_init").mockReturnValue(true);
    jest.spyOn(gameEngine.display, "flip").mockImplementation(() => undefined);
  });

  afterEach(() => {
    jest.restoreAllMocks();
  });

  it("defers async save execution until after the saving prompt has rendered", async () => {
    const menuState = new MenuState(
      createMenuUi(),
      createInitialGameState(),
      { playSound: jest.fn() } as unknown as AudioEngine,
      null,
    );
    jest
      .spyOn(menuState as unknown as { formatSaveText: (label: string) => string }, "formatSaveText")
      .mockImplementation((label: string) => label);

    const saveCallback = jest.fn().mockResolvedValue(true);
    menuState.beginSaveFlow({ saveExists: false, saveCallback });

    menuState.handleInput(new gameEngine.event.Event("keydown", { key: "z", code: "KeyZ" }));

    expect(saveCallback).not.toHaveBeenCalled();

    await new Promise((resolve) => setTimeout(resolve, 25));

    expect(saveCallback).toHaveBeenCalledTimes(1);
    expect((menuState as unknown as { saveFlow: { stage: string } | null }).saveFlow?.stage).toBe("saved");
  });
});
