import { createInitialGameState } from "@pokecrystal/core/core/state";
import { BoxSchema, formatDefaultBoxName, type Pokemon, type PokemonSpecies } from "@pokecrystal/core/core/models";
import { setSlot } from "@pokecrystal/core/core/models/box";
import { toPokemon } from "@pokecrystal/core/core/models/pokemon";
import { Ability, EggGroup, GenderRatio, GrowthRate, PokemonType } from "@pokecrystal/core/core/enums";
import { KEYS } from "@pokecrystal/core/core/keycodes";
import type { KeyEvent } from "@pokecrystal/core/input/buttons";
import { Surface } from "@pokecrystal/core/ui/surface";
import { BaseUI } from "@pokecrystal/core/ui/base-ui";
import { CompositeUI, type CompositeChild } from "@pokecrystal/core/ui/composite-ui";
import { TextUI } from "@pokecrystal/core/ui/text-ui";
import { BitmapFont } from "@pokecrystal/core/ui/text/bitmap-font";
import { createPokemon } from "@pokecrystal/core/engine/systems/pokemon";
import type { PCFont, SupportsPokemonPCUI } from "./pc-components";
import { PokemonPCMenu } from "./pc-components";
import { INFO_ITEM_ORIGIN } from "./pc-layout";
import { getPcCursorTile, getPcItemIcon } from "./pc-wallpaper";

let fontProxy: PCFont;
const speciesCache = new Map<string, PokemonSpecies>();

const DEFAULT_BASE_STATS = {
  hp: 20,
  attack: 10,
  defense: 10,
  speed: 10,
  special_attack: 10,
  special_defense: 10,
};

const ensureSpecies = (id: string): PokemonSpecies => {
  const upperId = id.toUpperCase();
  const cached = speciesCache.get(upperId);
  if (cached) {
    return cached;
  }
  const species: PokemonSpecies = {
    id: upperId,
    int_id: 0,
    base_stats: DEFAULT_BASE_STATS,
    type1: PokemonType.NORMAL,
    type2: PokemonType.NONE,
    catch_rate: 45,
    base_exp: 64,
    item1: undefined,
    item2: undefined,
    gender_ratio: GenderRatio.GENDER_F50,
    unknown1: 0,
    step_cycles_to_hatch: 5120,
    unknown2: 0,
    growth_rate: GrowthRate.GROWTH_MEDIUM_FAST,
    egg_group1: EggGroup.EGG_MONSTER,
    egg_group2: EggGroup.EGG_MONSTER,
    tmhm_learnset: [],
    ability: Ability.NONE,
    pic_size: 0,
    front_pic: 0,
    back_pic: 0,
    evolutions: null,
    weight: 0,
  };
  speciesCache.set(upperId, species);
  return species;
};

const makePokemon = (gameState: ReturnType<typeof createInitialGameState>, speciesId: string, level = 5): Pokemon =>
  toPokemon(createPokemon(gameState, ensureSpecies(speciesId), level));

const makeUi = (eventBatches: KeyEvent[][]): SupportsPokemonPCUI => {
  let callCount = 0;
  const screen = new Surface(160, 144);
  return {
    screen,
    font: fontProxy,
    drawWindow: jest.fn(),
    drawSprite: jest.fn(),
    drawRect: jest.fn(),
    update: jest.fn(),
    pollEvents: () => eventBatches[callCount++] ?? [],
    renderSnapshot: jest.fn(),
  };
};

const sample = (surface: Surface, x: number, y: number): [number, number, number, number] => {
  return surface.get_at([x, y]);
};

const expectTileAt = (screen: Surface, tileX: number, tileY: number, expected: Surface): void => {
  for (let y = 0; y < 8; y += 1) {
    for (let x = 0; x < 8; x += 1) {
      expect(sample(screen, tileX * 8 + x, tileY * 8 + y)).toEqual(sample(expected, x, y));
    }
  }
};

const dominantPixel = (surface: Surface): [number, number, number, number] => {
  const counts = new Map<string, number>();
  for (let row = 0; row < 8; row += 1) {
    for (let col = 0; col < 8; col += 1) {
      const key = sample(surface, col, row).join(",");
      counts.set(key, (counts.get(key) ?? 0) + 1);
    }
  }
  let dominant = "0,0,0,255";
  let dominantCount = -1;
  for (const [key, count] of counts) {
    if (count > dominantCount) {
      dominant = key;
      dominantCount = count;
    }
  }
  return dominant.split(",").map((component) => Number(component)) as [number, number, number, number];
};

const expectObjectPixelsAt = (screen: Surface, x: number, y: number, expected: Surface): void => {
  const transparent = dominantPixel(expected).join(",");
  for (let row = 0; row < 8; row += 1) {
    for (let col = 0; col < 8; col += 1) {
      const expectedPixel = sample(expected, col, row);
      if (expectedPixel.join(",") === transparent) {
        continue;
      }
      expect(sample(screen, x + col, y + row)).toEqual(expectedPixel);
    }
  }
};

const hasNonWhitePixel = (
  surface: Surface,
  rect: { x: number; y: number; width: number; height: number },
): boolean => {
  for (let y = rect.y; y < rect.y + rect.height; y += 1) {
    for (let x = rect.x; x < rect.x + rect.width; x += 1) {
      const [r, g, b, a] = sample(surface, x, y);
      if (a > 0 && (r !== 255 || g !== 255 || b !== 255)) {
        return true;
      }
    }
  }
  return false;
};

class LivePCUI extends BaseUI {
  constructor() {
    super(160, 144, 1);
    this.font = fontProxy;
  }

  protected createScreenSurface(): Surface {
    return new Surface(160, 144);
  }

  update(): void {
    return;
  }
}

describe("PokemonPCMenu", () => {
  beforeAll(async () => {
    const font = new BitmapFont();
    await font.load();
    fontProxy = {
      paletteVariants: font.paletteVariants.bind(font),
      renderText: font.renderText.bind(font),
    };
  });

  it("exits the async loop on cancel input", async () => {
    const gameState = createInitialGameState();
    gameState.sram.pc_boxes = [BoxSchema.parse({ name: formatDefaultBoxName(0) })];
    const cancelEvent: KeyEvent = {
      type: "keydown",
      key: KEYS.X,
      code: KEYS.X,
      is_press: true,
    };
    const ui = makeUi([[cancelEvent], []]);
    const menu = new PokemonPCMenu(ui, gameState, null);

    const result = await menu.runInteractiveAsync();

    expect(result).toEqual([]);
    expect(ui.update).toHaveBeenCalled();
  });

  it("keeps the overlay empty when move selection starts from an empty slot", () => {
    const gameState = createInitialGameState();
    gameState.sram.pc_boxes = [BoxSchema.parse({ name: formatDefaultBoxName(0) })];
    const ui = makeUi([]);
    const menu = new PokemonPCMenu(ui, gameState, null);

    (menu as unknown as { beginMoveSelection: () => void }).beginMoveSelection();

    expect((menu as unknown as { overlayMessage: string | null }).overlayMessage).toBeNull();
  });

  it("draw clears stale pixels instead of leaking the overworld behind blank PC tiles", () => {
    const gameState = createInitialGameState();
    gameState.sram.pc_boxes = [BoxSchema.parse({ name: formatDefaultBoxName(0) })];
    const ui = makeUi([]);
    ui.screen.fill([255, 0, 255, 255]);
    const menu = new PokemonPCMenu(ui, gameState, null);

    menu.draw();

    expect(ui.screen.getAt(19 * 8, 17 * 8)).toEqual([255, 255, 255, 255]);
  });

  it("draws the selected Pokemon frontpic at the ASM PCMonInfo tile origin", () => {
    const gameState = createInitialGameState();
    const box = BoxSchema.parse({ name: formatDefaultBoxName(0) });
    setSlot(box, 0, makePokemon(gameState, "TOTODILE", 8));
    gameState.sram.pc_boxes = [box];
    const ui = makeUi([]);
    const menu = new PokemonPCMenu(ui, gameState, null);

    menu.draw();

    expect(ui.drawSprite).toHaveBeenCalledWith("TOTODILE", 8, 32, "pokemon", 0);
  });

  it("draws the selected Pokemon frontpic through the live BaseUI and CompositeUI sprite path", () => {
    const gameState = createInitialGameState();
    const box = BoxSchema.parse({ name: formatDefaultBoxName(0) });
    setSlot(box, 0, makePokemon(gameState, "TOTODILE", 8));
    gameState.sram.pc_boxes = [box];
    const tileUi = new LivePCUI();
    const textUi = new TextUI(160, 144, 1, null, true, null, true);
    const composite = new CompositeUI(
      tileUi as unknown as CompositeChild,
      textUi as unknown as CompositeChild,
    ) as unknown as {
      screen: Surface;
      drawWindow: SupportsPokemonPCUI["drawWindow"];
      drawSprite: SupportsPokemonPCUI["drawSprite"];
      update: () => void;
      renderSnapshot: NonNullable<SupportsPokemonPCUI["renderSnapshot"]>;
    };
    const ui: SupportsPokemonPCUI = {
      screen: composite.screen,
      font: fontProxy,
      drawWindow: composite.drawWindow.bind(composite),
      drawSprite: composite.drawSprite.bind(composite),
      drawRect: jest.fn(),
      update: composite.update.bind(composite),
      renderSnapshot: composite.renderSnapshot.bind(composite),
    };
    const menu = new PokemonPCMenu(ui, gameState, null);

    menu.draw();

    expect(hasNonWhitePixel(tileUi.screen, { x: 8, y: 32, width: 56, height: 56 })).toBe(true);
    expect(textUi.getSnapshot()?.viewportTitle).toBe("Bill's PC");
  });

  it("emits a Bill's PC text snapshot for text-only renderers", () => {
    const gameState = createInitialGameState();
    const box = BoxSchema.parse({ name: formatDefaultBoxName(0) });
    setSlot(box, 0, makePokemon(gameState, "TOTODILE", 8));
    gameState.sram.pc_boxes = [box];
    const ui = makeUi([]);
    const menu = new PokemonPCMenu(ui, gameState, null);

    menu.draw();

    expect(ui.renderSnapshot).toHaveBeenCalledWith(
      expect.arrayContaining(["WITHDRAW #MON", "BOX 01", "▶ TOTODILE"]),
      expect.arrayContaining(["SELECTED: TOTODILE", "LEVEL: 8"]),
      "Bill's PC",
      "Legend",
      null,
      expect.arrayContaining(["CHOOSE A POKéMON."]),
      null,
    );
  });

  it("draws Bill's PC on TextUI without requiring bitmap font tiles", () => {
    const gameState = createInitialGameState();
    const box = BoxSchema.parse({ name: formatDefaultBoxName(0) });
    setSlot(box, 0, makePokemon(gameState, "TOTODILE", 8));
    gameState.sram.pc_boxes = [box];
    const textUi = new TextUI(160, 144, 1, false, false, 0, true);
    const ui: SupportsPokemonPCUI = {
      screen: textUi.screen,
      font: textUi.font as PCFont,
      drawWindow: textUi.drawWindow.bind(textUi),
      drawSprite: jest.fn(),
      drawRect: jest.fn(),
      update: textUi.update.bind(textUi),
      renderSnapshot: textUi.renderSnapshot.bind(textUi),
    };
    const menu = new PokemonPCMenu(ui, gameState, null);

    expect(() => menu.draw()).not.toThrow();
    expect(textUi.getSnapshot()?.viewportTitle).toBe("Bill's PC");
    expect(textUi.getSnapshot()?.viewportLines).toEqual(
      expect.arrayContaining(["WITHDRAW #MON", "▶ TOTODILE"]),
    );
  });

  it("renders the held-item icon in the fixed info-panel tile", () => {
    const gameState = createInitialGameState();
    const box = BoxSchema.parse({ name: formatDefaultBoxName(0) });
    const stored = makePokemon(gameState, "TOTODILE", 8);
    stored.item = "POTION";
    setSlot(box, 0, stored);
    gameState.sram.pc_boxes = [box];
    const ui = makeUi([]);
    const menu = new PokemonPCMenu(ui, gameState, null);

    menu.draw();

    expectTileAt(ui.screen!, INFO_ITEM_ORIGIN.x, INFO_ITEM_ORIGIN.y, getPcItemIcon());
  });

  it("switches to the party list for Bill's deposit flow", () => {
    const gameState = createInitialGameState();
    gameState.sram.current_pc_box = 0;
    gameState.sram.pc_boxes = [BoxSchema.parse({ name: formatDefaultBoxName(0) })];
    gameState.sram.party.pokemon = [
      makePokemon(gameState, "CYNDAQUIL", 5),
      makePokemon(gameState, "TOGEPI", 6),
      null,
      null,
      null,
      null,
    ];
    const ui = makeUi([]);
    const menu = new PokemonPCMenu(ui, gameState, null);

    menu.showBillAction("deposit");

    expect((menu as unknown as { loadedBoxValue: number }).loadedBoxValue).toBe(0);
    expect((menu as unknown as { mode: string }).mode).toBe("deposit");
    expect((menu as unknown as { monEntries: Array<{ boxIndex: number | null }> }).monEntries).toHaveLength(2);
    expect(
      (menu as unknown as { monEntries: Array<{ boxIndex: number | null }> }).monEntries.every(
        (entry) => entry.boxIndex === null,
      ),
    ).toBe(true);
  });

  it("uses the ASM current-box sentinel for withdraw and ignores horizontal input", () => {
    const gameState = createInitialGameState();
    gameState.sram.current_pc_box = 0;
    gameState.sram.pc_boxes = [
      BoxSchema.parse({ name: formatDefaultBoxName(0) }),
      BoxSchema.parse({ name: formatDefaultBoxName(1) }),
    ];
    const ui = makeUi([]);
    const menu = new PokemonPCMenu(ui, gameState, null);

    menu.showBillAction("withdraw");
    expect((menu as unknown as { loadedBoxValue: number }).loadedBoxValue).toBe(15);

    const moved = (menu as unknown as { pressRight: () => boolean }).pressRight();

    expect(moved).toBe(false);
    expect((menu as unknown as { loadedBoxValue: number }).loadedBoxValue).toBe(15);
    expect(gameState.sram.current_pc_box).toBe(0);
  });

  it("returns a deposit action from the async path for MCP button-confirm events", async () => {
    const gameState = createInitialGameState();
    gameState.sram.party.pokemon = [
      makePokemon(gameState, "CYNDAQUIL", 5),
      makePokemon(gameState, "GEODUDE", 25),
      makePokemon(gameState, "SANDSHREW", 17),
      makePokemon(gameState, "BELLSPROUT", 18),
      makePokemon(gameState, "BUTTERFREE", 20),
      makePokemon(gameState, "TOGEPI", 5),
    ];
    gameState.sram.pc_boxes = [BoxSchema.parse({ name: formatDefaultBoxName(0) })];
    const ui = makeUi([
      [{ type: "keydown", key: "ArrowDown", code: "ArrowDown", is_press: true }],
      [{ type: "keydown", key: "ArrowDown", code: "ArrowDown", is_press: true }],
      [{ type: "keydown", key: "ArrowDown", code: "ArrowDown", is_press: true }],
      [{ type: "keydown", key: "ArrowDown", code: "ArrowDown", is_press: true }],
      [{ type: "keydown", key: "ArrowDown", code: "ArrowDown", is_press: true }],
      [{ type: "keydown", button: "a", is_press: true }],
      [{ type: "keydown", button: "b", is_press: true }],
    ]);
    const menu = new PokemonPCMenu(ui, gameState, null);
    menu.showBillAction("deposit");

    const actions = await menu.runInteractiveAsync({ actionHandler: (payload) => payload });

    expect(actions).toEqual([
      expect.objectContaining({ action: "deposit", box: 0, party_slot: 5, slot: null }),
    ]);
  });

  it("uses the ASM move submenu labels before entering move-target selection", () => {
    const gameState = createInitialGameState();
    gameState.sram.current_pc_box = 0;
    const box = BoxSchema.parse({ name: formatDefaultBoxName(0) });
    setSlot(box, 0, makePokemon(gameState, "TOTODILE", 8));
    gameState.sram.pc_boxes = [box];
    gameState.sram.party.pokemon = [makePokemon(gameState, "CYNDAQUIL", 5), null, null, null, null, null];
    const ui = makeUi([]);
    const menu = new PokemonPCMenu(ui, gameState, null);
    menu.showBillAction("move");

    expect(menu.startMoveSession({ confirmationProvider: () => true })).toBe(true);

    const internals = menu as unknown as {
      currentSelectionEntry: () => unknown;
      enterActionMode: (entry: unknown) => void;
      advanceJumptableState: () => void;
      actionLabels: string[];
      mode: string;
    };
    internals.enterActionMode(internals.currentSelectionEntry());
    internals.advanceJumptableState();

    expect(internals.actionLabels).toEqual(["MOVE", "STATS", "CANCEL"]);
    expect(internals.mode).toBe("actions");
  });

  it("keeps the source Pokemon preview fixed while drawing the move insert cursor", () => {
    const gameState = createInitialGameState();
    gameState.sram.current_pc_box = 0;
    const box = BoxSchema.parse({ name: formatDefaultBoxName(0) });
    setSlot(box, 0, makePokemon(gameState, "TOTODILE", 8));
    setSlot(box, 1, makePokemon(gameState, "SQUIRTLE", 10));
    gameState.sram.pc_boxes = [box];
    gameState.sram.party.pokemon = [makePokemon(gameState, "CYNDAQUIL", 5), null, null, null, null, null];
    const ui = makeUi([]);
    const menu = new PokemonPCMenu(ui, gameState, null);
    menu.showBillAction("move");
    expect(menu.startMoveSession({ confirmationProvider: () => true })).toBe(true);
    (menu as unknown as { beginMoveSelection: () => void }).beginMoveSelection();
    menu.handleInput({
      type: "keydown",
      key: "ArrowDown",
      code: "ArrowDown",
      is_press: true,
    });
    (ui.drawSprite as jest.Mock).mockClear();

    menu.draw();

    expect(ui.drawSprite).toHaveBeenCalledWith("TOTODILE", 8, 32, "pokemon", 0);
    expect(ui.drawSprite).not.toHaveBeenCalledWith("SQUIRTLE", 8, 32, "pokemon", 0);
    expectObjectPixelsAt(ui.screen!, 72, 39, getPcCursorTile(0x06));
  });
});
