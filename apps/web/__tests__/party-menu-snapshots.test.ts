import fs from "fs";
import path from "path";
import { createInitialGameState } from "@/core/state";
import { loadAllSpecies } from "@/core/data-loader";
import { MoveName } from "@/core/enums";
import { VRAMManager } from "@/core/memory/vram";
import { createPokemon } from "@/engine/systems/pokemon";
import { gameEngine } from "@/ui/game-engine";
import { Surface } from "@/ui/surface";
import { PokemonMenu, type PokemonMenuUI } from "@/ui/menus/pokemon-menu";

const FIXTURE_DIR = path.resolve(__dirname, "../../../tests/fixtures/party_menu_snapshots");

const loadFixture = (name: string): { tiles: number[]; attrs: number[] } | null => {
  const fixturePath = path.join(FIXTURE_DIR, `${name}.json`);
  if (!fs.existsSync(fixturePath)) {
    return null;
  }
  const raw = fs.readFileSync(fixturePath, "utf8");
  return JSON.parse(raw) as { tiles: number[]; attrs: number[] };
};

const installBgMapWriter = (gameState: ReturnType<typeof createInitialGameState>): void => {
  const runtime = gameState as ReturnType<typeof createInitialGameState> & {
    write_bg_map_with_wait?: (
      name: string,
      width: number,
      height: number,
      tiles: number[],
      attrs: number[],
      options?: { origin_x?: number; origin_y?: number }
    ) => void;
    bg_map_sync?: { is_busy: boolean; remaining_frames?: number };
  };
  runtime.write_bg_map_with_wait = (name, width, height, tiles, attrs, options = {}) => {
    const manager = new VRAMManager(gameState.vram);
    manager.writeBgRegion(name, width, height, tiles, attrs, {
      originX: options.origin_x ?? 0,
      originY: options.origin_y ?? 0,
    });
    runtime.bg_map_sync = { is_busy: false, remaining_frames: 0 };
  };
};

const buildFontStub = (): PokemonMenuUI["font"] => {
  const fontTiles: Record<number, Surface> = {};
  for (let tileId = 0; tileId <= 0xff; tileId += 1) {
    fontTiles[tileId] = new Surface(8, 8);
  }
  return {
    fontTiles,
    font_tiles: fontTiles,
    renderText: jest.fn(),
    render_text: jest.fn(),
  };
};

const captureVram = (gameState: ReturnType<typeof createInitialGameState>): { tiles: number[]; attrs: number[] } => {
  const manager = new VRAMManager(gameState.vram);
  const bgMap = manager.resolveBgMap("vBGMap0");
  const tiles: number[] = [];
  const attrs: number[] = [];
  for (let row = 0; row < 18; row += 1) {
    for (let col = 0; col < 20; col += 1) {
      const index = row * bgMap.width + col;
      tiles.push(bgMap.tiles[index]);
      attrs.push(bgMap.attributes[index]);
    }
  }
  return { tiles, attrs };
};

const pressKey = (menu: PokemonMenu, code: string): void => {
  menu.handleInput(new gameEngine.event.Event(gameEngine.KEYDOWN, { code }));
  menu.handleInput(new gameEngine.event.Event(gameEngine.KEYUP, { code }));
};

const pressKeyForResult = (menu: PokemonMenu, code: string): [string, number] | null => {
  const result = menu.handleInput(new gameEngine.event.Event(gameEngine.KEYDOWN, { code }));
  menu.handleInput(new gameEngine.event.Event(gameEngine.KEYUP, { code }));
  return result;
};

const buildMenu = (): { menu: PokemonMenu; gameState: ReturnType<typeof createInitialGameState> } => {
  const gameState = createInitialGameState();
  installBgMapWriter(gameState);

  const species = loadAllSpecies();
  const chikorita = species.get("CHIKORITA");
  const cyndaquil = species.get("CYNDAQUIL");
  if (!chikorita || !cyndaquil) {
    throw new Error("Missing expected party species data.");
  }
  const first = createPokemon(gameState, chikorita, 5);
  const second = createPokemon(gameState, cyndaquil, 5);
  const simpleMoves = [{ name: MoveName.TACKLE, current_pp: 35 }];
  first.moves = simpleMoves;
  second.moves = simpleMoves;
  gameState.sram.party.pokemon[0] = first;
  gameState.sram.party.pokemon[1] = second;

  const ui: PokemonMenuUI = {
    screen: new Surface(160, 144),
    font: buildFontStub(),
  };

  return { menu: new PokemonMenu(ui, gameState, null), gameState };
};

describe("Pokemon menu snapshots (python parity)", () => {
  let debugSpy: jest.SpyInstance;

  beforeAll(() => {
    debugSpy = jest.spyOn(console, "debug").mockImplementation(() => {});
  });

  afterAll(() => {
    debugSpy.mockRestore();
  });

  it("matches the default party menu snapshot", () => {
    const { menu, gameState } = buildMenu();
    menu.draw();
    const snapshot = captureVram(gameState);
    const fixture = loadFixture("default");
    expect(snapshot.tiles.length).toBeGreaterThan(0);
    expect(snapshot.attrs.length).toBeGreaterThan(0);
    if (fixture) {
      expect(snapshot.tiles).toEqual(fixture.tiles);
      expect(snapshot.attrs).toEqual(fixture.attrs);
    }
  });

  it("matches the switch-mode snapshot", () => {
    const { menu, gameState } = buildMenu();
    pressKey(menu, "KeyZ");
    pressKey(menu, "ArrowDown");
    pressKey(menu, "KeyZ");
    menu.draw();
    const snapshot = captureVram(gameState);
    const fixture = loadFixture("switch");
    expect(snapshot.tiles.length).toBeGreaterThan(0);
    expect(snapshot.attrs.length).toBeGreaterThan(0);
    if (fixture) {
      expect(snapshot.tiles).toEqual(fixture.tiles);
      expect(snapshot.attrs).toEqual(fixture.attrs);
    }
  });

  it("matches the give/take snapshot", () => {
    const { menu, gameState } = buildMenu();
    pressKey(menu, "ArrowDown");
    pressKey(menu, "KeyZ");
    menu.draw();
    const snapshot = captureVram(gameState);
    const fixture = loadFixture("give_take");
    expect(snapshot.tiles.length).toBeGreaterThan(0);
    expect(snapshot.attrs.length).toBeGreaterThan(0);
    if (fixture) {
      expect(snapshot.tiles).toEqual(fixture.tiles);
      expect(snapshot.attrs).toEqual(fixture.attrs);
    }
  });

  it("returns cancel when selecting the cancel row in switch mode", () => {
    const { menu } = buildMenu();
    pressKey(menu, "KeyZ");
    pressKey(menu, "ArrowDown");
    pressKey(menu, "KeyZ");
    pressKey(menu, "ArrowDown");
    pressKey(menu, "ArrowDown");
    pressKey(menu, "ArrowDown");
    const result = pressKeyForResult(menu, "KeyZ");
    expect(result).toBeNull();
  });
});
