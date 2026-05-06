import { createInitialGameState } from "@pokecrystal/core/core/state";
import { Surface } from "@pokecrystal/core/ui/surface";

const mockDraw = jest.fn();
const mockHandleInput = jest.fn(() => null);
const mockReset = jest.fn();
const mockSetJumptableState = jest.fn();
const mockRedisplayEntryScreen = jest.fn();
jest.mock("@pokecrystal/core/ui/menus/pokedex", () => ({
  DexJumptableState: {
    MAIN_SCR: 0,
    UPDATE_MAIN_SCR: 1,
    DEX_ENTRY_SCR: 2,
    UPDATE_DEX_ENTRY_SCR: 3,
    REINIT_DEX_ENTRY_SCR: 4,
    SEARCH_SCR: 5,
    UPDATE_SEARCH_SCR: 6,
    OPTION_SCR: 7,
    UPDATE_OPTION_SCR: 8,
    SEARCH_RESULTS_SCR: 9,
    UPDATE_SEARCH_RESULTS_SCR: 10,
    UNOWN_MODE: 11,
    UPDATE_UNOWN_MODE: 12,
    EXIT: 13,
  },
  PokedexScreen: jest.fn(() => ({
    draw: mockDraw,
    handleInput: mockHandleInput,
    reset: mockReset,
    setJumptableState: mockSetJumptableState,
    redisplayEntryScreen: mockRedisplayEntryScreen,
  })),
}));

jest.mock("@pokecrystal/core/ui/menus/pokedex-assets", () => ({
  resetPokedexHardwareState: jest.fn(),
}));

import { PokedexScreen } from "./pokedex-screen";
import { resetPokedexHardwareState } from "@pokecrystal/core/ui/menus/pokedex-assets";

describe("PokedexScreen wrapper", () => {
  beforeEach(() => {
    mockDraw.mockClear();
    mockHandleInput.mockClear();
    mockReset.mockClear();
    mockSetJumptableState.mockClear();
    mockRedisplayEntryScreen.mockClear();
    (resetPokedexHardwareState as jest.Mock).mockClear();
  });

  it("draws the menu immediately on the main screen instead of forcing black transition frames", () => {
    const gameState = createInitialGameState();
    const fill = jest.fn();
    const ui = {
      screen: Object.assign(new Surface(160, 144), { fill }),
      font: { renderText: jest.fn() },
      drawWindow: jest.fn(),
    };

    const screen = new PokedexScreen(ui as never, gameState, null);
    const result = screen.step(null);

    expect(result).toBeNull();
    expect(resetPokedexHardwareState).toHaveBeenCalled();
    expect(mockReset).toHaveBeenCalled();
    expect(mockDraw).toHaveBeenCalled();
    expect(fill).not.toHaveBeenCalled();
    expect(screen.blackoutActive).toBe(false);
  });

  it("forwards the pokemon front-sprite loader into the menu renderer", () => {
    const gameState = createInitialGameState();
    const sprite = new Surface(56, 56);
    const getPokemonFrontSurface = jest.fn(() => sprite);
    const ui = {
      screen: new Surface(160, 144),
      font: { renderText: jest.fn() },
      drawWindow: jest.fn(),
      getPokemonFrontSurface,
    };

    new PokedexScreen(ui as never, gameState, null);

    const { PokedexScreen: MockMenuPokedexScreen } = jest.requireMock("@pokecrystal/core/ui/menus/pokedex");
    const menuUi = (MockMenuPokedexScreen as jest.Mock).mock.calls.at(-1)?.[0];
    expect(menuUi.getPokemonFrontSurface("CHIKORITA", 0)).toBe(sprite);
    expect(getPokemonFrontSurface).toHaveBeenCalledWith("CHIKORITA", 0);
  });

  it("preserves rich text-render options when the UI only exposes renderText", () => {
    const gameState = createInitialGameState();
    const renderText = jest.fn();
    const ui = {
      screen: new Surface(160, 144),
      font: { renderText },
      drawWindow: jest.fn(),
    };

    new PokedexScreen(ui as never, gameState, null);

    const { PokedexScreen: MockMenuPokedexScreen } = jest.requireMock("@pokecrystal/core/ui/menus/pokedex");
    const menuUi = (MockMenuPokedexScreen as jest.Mock).mock.calls.at(-1)?.[0];
    menuUi.font.renderText("Entry text", 8, 16, ui.screen, {
      textWidth: 40,
      maxLines: 2,
      uppercase: false,
      color: [1, 2, 3],
    });

    expect(renderText).toHaveBeenCalledWith(
      "Entry text",
      8,
      16,
      ui.screen,
      expect.objectContaining({
        textWidth: 40,
        maxLines: 2,
        uppercase: false,
        color: [1, 2, 3],
      }),
    );
  });
});
