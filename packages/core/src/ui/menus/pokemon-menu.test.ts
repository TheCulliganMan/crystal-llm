import { createInitialGameState } from "@pokecrystal/core/core/state";
import { createTestPokemon } from "@pokecrystal/core/engine/world/story-events/test-utils";
import { createPokemon } from "@pokecrystal/core/engine/systems/pokemon";
import { BitmapFont } from "@pokecrystal/core/ui/text/bitmap-font";
import { buildDefaultCharMap } from "@pokecrystal/core/ui/text/glyph-map";
import { gameEngine } from "../game-engine";
import { PokemonMenu } from "./pokemon-menu";
import { NAME_COLUMN } from "./party-menu-layout";

jest.mock("./party-menu-icons", () => ({
  PartyMenuIconRenderer: jest.fn().mockImplementation(() => ({
    draw: jest.fn(),
  })),
}));

describe("PokemonMenu item selector", () => {
  let fontProxy: {
    renderText: BitmapFont["renderText"];
    paletteVariants: BitmapFont["paletteVariants"];
    fontTiles: BitmapFont["fontTiles"];
    font_tiles: BitmapFont["fontTiles"];
  };

  beforeAll(async () => {
    const font = new BitmapFont();
    await font.load();
    fontProxy = {
      renderText: font.renderText.bind(font),
      paletteVariants: font.paletteVariants.bind(font),
      fontTiles: font.fontTiles,
      font_tiles: font.fontTiles,
    };
  });

  const createMenu = () => {
    const gameState = createInitialGameState();
    const ui = {
      screen: null,
      font: fontProxy,
    };

    return {
      gameState,
      menu: new PokemonMenu(ui, gameState),
    };
  };

  it("renders Ho-Oh's generated nickname with a hyphen tile", () => {
    const { gameState, menu } = createMenu();
    const species = createTestPokemon("HO_OH", 250).species;
    gameState.sram.party.pokemon[0] = createPokemon(gameState, species, 60);

    const tilemap = (menu as any).buildTilemap();
    const charMap = buildDefaultCharMap();
    const hoOhDashColumn = NAME_COLUMN + 2;

    expect(gameState.sram.party.pokemon[0]?.nickname).toBe("HO-OH");
    expect(tilemap.getTile(hoOhDashColumn, menu._name_row_y(0))).toBe(charMap["-"]);
    expect(tilemap.getTile(hoOhDashColumn, menu._name_row_y(0))).not.toBe(charMap["_"]);
  });

  it("does not throw when the item selector is not wired", () => {
    const { gameState, menu } = createMenu();
    gameState.sram.party.pokemon[0] = createTestPokemon("CYNDAQUIL", 1);
    gameState.sram.items.POTION = 1;

    const [entry] = menu.getPartyEntries();
    const applySelection = () => {
      (menu as any).giveItemToPokemon(entry);
    };

    expect(() => applySelection()).not.toThrow();
    expect(gameState.sram.party.pokemon[0]?.item).toBeUndefined();
    expect(gameState.sram.items.POTION).toBe(1);
  });

  it("does not mutate held items without the full interactive flow", () => {
    const { gameState, menu } = createMenu();
    gameState.sram.party.pokemon[0] = createTestPokemon("CYNDAQUIL", 2);
    gameState.sram.items.POTION = 2;
    menu.setItemSelector(() => "POTION");

    const [entry] = menu.getPartyEntries();
    (menu as any).giveItemToPokemon(entry);

    expect(gameState.sram.party.pokemon[0]?.item).toBeUndefined();
    expect(gameState.sram.items.POTION).toBe(1);
  });

  it("does not keep a submenu d-pad direction latched into the next face-button press", () => {
    const { gameState, menu } = createMenu();
    gameState.sram.party.pokemon[0] = createTestPokemon("CYNDAQUIL", 8);

    menu.handleInput(new gameEngine.event.Event(gameEngine.KEYDOWN, { key: "KeyJ", code: "KeyJ" }));
    menu.handleInput(new gameEngine.event.Event(gameEngine.KEYDOWN, { key: gameEngine.K_DOWN, code: gameEngine.K_DOWN }));

    expect(menu.getSubmenuIndex()).toBe(1);

    menu.handleInput(new gameEngine.event.Event(gameEngine.KEYDOWN, { key: "KeyK", code: "KeyK" }));

    expect(menu.getMode()).toBe("list");
    expect(menu.getSubmenuChoices()).toHaveLength(0);
  });
});
