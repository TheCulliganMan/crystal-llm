import fs from "fs";
import { gbc5To8, type RGB } from "@pokecrystal/core/core/gbc-colors";
import { getAssetPath } from "@pokecrystal/core/core/paths";
import { createInitialGameState } from "@pokecrystal/core/core/state";
import { SlotSymbol } from "@pokecrystal/core/engine/games/slots";
import { GameButton } from "@pokecrystal/core/input/buttons";
import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import { Surface } from "@pokecrystal/core/ui/surface";
import { SlotMachineOverlay } from "./slot-machine";

const TILE_SIZE = 8;
const SLOTS_TILEMAP_WIDTH = 20;
const SLOTS_VTILES2_OVERLAY_START_TILE = 0x25;
const SLOT_ICON_TILE_STRIDE = 4;
const SLOT_ICON_SIZE = 16;
const SLOT_REEL_X_TILES = [5, 9, 13] as const;
const SLOT_REEL_Y_TILES = [4, 6, 8] as const;

const paletteIndexFromGray = (gray: number): number => {
  if (gray >= 213) return 0;
  if (gray >= 128) return 1;
  if (gray >= 43) return 2;
  return 3;
};

const loadSlotsPalettes = (): RGB[][] => {
  const content = fs.readFileSync(getAssetPath("gfx", "slots", "slots.pal"), "utf8");
  const colors = Array.from(content.matchAll(/RGB\s+(\d+),\s*(\d+),\s*(\d+)/g), (match): RGB => [
    gbc5To8(Number(match[1]), "slot red"),
    gbc5To8(Number(match[2]), "slot green"),
    gbc5To8(Number(match[3]), "slot blue"),
  ]);
  return Array.from({ length: 16 }, (_, index) => colors.slice(index * 4, index * 4 + 4));
};

const assertSlotIconAt = (
  screen: Surface,
  symbolSheet: Surface,
  palettes: RGB[][],
  symbol: SlotSymbol,
  destTileX: number,
  destTileY: number,
): void => {
  const baseTileIndex = symbol * SLOT_ICON_TILE_STRIDE;
  const tileLayout = [baseTileIndex, baseTileIndex + 1, baseTileIndex + 2, baseTileIndex + 3];
  const sourceColumns = Math.floor(symbolSheet.get_width() / TILE_SIZE);
  const palette = palettes[symbol];
  let checkedPixels = 0;

  for (let tileOffset = 0; tileOffset < tileLayout.length; tileOffset += 1) {
    const sourceTileId = tileLayout[tileOffset];
    const sourceBaseX = (sourceTileId % sourceColumns) * TILE_SIZE;
    const sourceBaseY = Math.floor(sourceTileId / sourceColumns) * TILE_SIZE;
    const destBaseX = destTileX * TILE_SIZE + (tileOffset % 2) * TILE_SIZE;
    const destBaseY = destTileY * TILE_SIZE + Math.floor(tileOffset / 2) * TILE_SIZE;

    for (let y = 0; y < TILE_SIZE; y += 1) {
      for (let x = 0; x < TILE_SIZE; x += 1) {
        const sourceColor = symbolSheet.get_at([sourceBaseX + x, sourceBaseY + y]);
        const sourcePaletteIndex = paletteIndexFromGray(sourceColor[0]);
        if (sourcePaletteIndex === 0) {
          continue;
        }
        const [r, g, b] = palette[sourcePaletteIndex];
        expect(screen.get_at([destBaseX + x, destBaseY + y])).toEqual([r, g, b, 255]);
        checkedPixels += 1;
      }
    }
  }

  expect(checkedPixels).toBeGreaterThan(SLOT_ICON_SIZE);
};

const renderSlotScreen = async (): Promise<{ screen: Surface; renderText: jest.Mock; renderSnapshot: jest.Mock }> => {
  const gameState = createInitialGameState();
  gameState.sram.coins = 100;
  const eventQueue = gameEngine.event.createQueue();
  const screen = new Surface(160, 144);
  screen.fill([12, 34, 56, 255]);
  const renderText = jest.fn();
  const renderSnapshot = jest.fn();
  const ui = {
    eventQueue,
    screen,
    renderSnapshot,
    update: jest.fn(),
    font: { renderText },
  };

  gameEngine.event.post(
    new gameEngine.event.Event("keydown", { button: GameButton.B, is_press: true }),
    eventQueue,
  );

  await new SlotMachineOverlay(ui, gameState).runAsync();

  return { screen, renderText, renderSnapshot };
};

describe("SlotMachineOverlay", () => {
  it("renders the slot-machine interface into the Game Boy screen buffer", async () => {
    const { screen, renderText, renderSnapshot } = await renderSlotScreen();

    expect(screen.get_at([0, 0])).not.toEqual([12, 34, 56, 255]);
    expect(screen.get_at([0, 0])).toEqual([198, 198, 74, 255]);
    let coloredPixelCount = 0;
    for (let y = 0; y < 96; y += 1) {
      for (let x = 0; x < 160; x += 1) {
        const [r, g, b, a] = screen.get_at([x, y]);
        if (a === 255 && (r !== g || g !== b)) {
          coloredPixelCount += 1;
        }
      }
    }
    expect(coloredPixelCount).toBeGreaterThan(0);
    expect(renderText).toHaveBeenCalledWith(
      "0100",
      expect.any(Number),
      expect.any(Number),
      screen,
      expect.any(Object),
    );
    const renderedText = renderText.mock.calls.map(([text]) => text).join("\n");
    expect(renderedText).not.toContain("MODE");
    expect(renderedText).not.toContain("LUCKY");
    expect(renderedText).not.toContain("NORMAL MACHINE");
    expect(renderSnapshot).toHaveBeenCalledWith(
      expect.arrayContaining(["SLOT MACHINE"]),
      expect.arrayContaining(["STATE: slot_machine", "Left/Right=Bet A=Spin B=Quit"]),
      "Slot Machine",
      "Legend",
      null,
      null,
      null,
    );
  });

  it("renders the active slot-machine prompt into the lower text area", async () => {
    const gameState = createInitialGameState();
    gameState.sram.coins = 100;
    const eventQueue = gameEngine.event.createQueue();
    const screen = new Surface(160, 144);
    const renderText = jest.fn((text: string, x: number, y: number, target: Surface) => {
      for (let i = 0; i < text.length; i += 1) {
        if (text[i] === " ") {
          continue;
        }
        target.fill([0, 0, 0, 255], { x: x + i * TILE_SIZE, y, width: TILE_SIZE, height: TILE_SIZE });
      }
    });
    const ui = {
      eventQueue,
      screen,
      renderSnapshot: jest.fn(),
      update: jest.fn(),
      font: { renderText },
    };

    gameEngine.event.post(
      new gameEngine.event.Event("keydown", { button: GameButton.B, is_press: true }),
      eventQueue,
    );

    await new SlotMachineOverlay(ui, gameState).runAsync();

    expect(renderText).toHaveBeenCalledWith(
      "PRESS A TO SPIN",
      TILE_SIZE,
      15 * TILE_SIZE,
      screen,
      expect.objectContaining({ uppercase: true }),
    );
    let nonBlankPromptPixels = 0;
    for (let y = 12 * TILE_SIZE; y < 18 * TILE_SIZE; y += 1) {
      for (let x = 0; x < 20 * TILE_SIZE; x += 1) {
        if (screen.get_at([x, y]).join(",") !== "255,255,255,255") {
          nonBlankPromptPixels += 1;
        }
      }
    }
    expect(nonBlankPromptPixels).toBeGreaterThan(0);
  });

  it("renders reel offsets in ASM visible order instead of top-to-bottom memory order", async () => {
    const { renderSnapshot } = await renderSlotScreen();

    const viewportLines = renderSnapshot.mock.calls[0]?.[0] as string[];
    expect(viewportLines.slice(4, 7)).toEqual([
      "CHERRY | PIKA | PIKA",
      "7 | 7 | 7",
      "SQUIRT | STARYU | PIKA",
    ]);
  });

  it("draws initial reel icons in ASM OAM order on the pixel surface", async () => {
    const { screen } = await renderSlotScreen();
    const palettes = loadSlotsPalettes();
    const symbolSheet = gameEngine.image.loadSync?.(getAssetPath("gfx", "slots", "slots_2.png"));
    if (!symbolSheet) {
      throw new Error("slot symbol asset did not load");
    }

    assertSlotIconAt(screen, symbolSheet, palettes, SlotSymbol.CHERRY, SLOT_REEL_X_TILES[0], SLOT_REEL_Y_TILES[0]);
    assertSlotIconAt(screen, symbolSheet, palettes, SlotSymbol.SEVEN, SLOT_REEL_X_TILES[0], SLOT_REEL_Y_TILES[1]);
    assertSlotIconAt(screen, symbolSheet, palettes, SlotSymbol.SQUIRTLE, SLOT_REEL_X_TILES[0], SLOT_REEL_Y_TILES[2]);
  });

  it("maps slot-machine tilemap regions through the exact ASM CGB palettes", async () => {
    const { screen } = await renderSlotScreen();
    const palettes = loadSlotsPalettes();
    const tilemap = fs.readFileSync(getAssetPath("gfx", "slots", "slots.tilemap"));
    const uiSheet = gameEngine.image.loadSync?.(getAssetPath("gfx", "slots", "slots_1.png"));
    const symbolSheet = gameEngine.image.loadSync?.(getAssetPath("gfx", "slots", "slots_2.png"));
    if (!uiSheet || !symbolSheet) {
      throw new Error("slot assets did not load");
    }

    const assertTileUsesPalette = (tileX: number, tileY: number, paletteIndex: number): void => {
      const tileId = tilemap[tileY * SLOTS_TILEMAP_WIDTH + tileX];
      const source = tileId < SLOTS_VTILES2_OVERLAY_START_TILE ? uiSheet : symbolSheet;
      const sourceTileId = tileId < SLOTS_VTILES2_OVERLAY_START_TILE
        ? tileId
        : tileId - SLOTS_VTILES2_OVERLAY_START_TILE;
      const columns = Math.floor(source.get_width() / TILE_SIZE);
      let sawPaletteColor = false;

      for (let y = 0; y < TILE_SIZE; y += 1) {
        for (let x = 0; x < TILE_SIZE; x += 1) {
          const sourceColor = source.get_at([
            (sourceTileId % columns) * TILE_SIZE + x,
            Math.floor(sourceTileId / columns) * TILE_SIZE + y,
          ]);
          const sourcePaletteIndex = paletteIndexFromGray(sourceColor[0]);
          const [r, g, b] = palettes[paletteIndex][sourcePaletteIndex];
          const expected = [r, g, b, 255];
          const actual = screen.get_at([tileX * TILE_SIZE + x, tileY * TILE_SIZE + y]);
          expect(actual).toEqual(expected);
          if (r !== g || g !== b) {
            sawPaletteColor = true;
          }
        }
      }

      expect(sawPaletteColor).toBe(true);
    };

    // ASM: engine/gfx/cgb_layouts.asm::_CGB_SlotMachine attrmap fills.
    assertTileUsesPalette(0, 0, 0);
    assertTileUsesPalette(4, 2, 1);
    assertTileUsesPalette(0, 2, 2);
    assertTileUsesPalette(0, 4, 3);
    assertTileUsesPalette(0, 6, 4);
  });

  it("animates reel windows across frames before settling on the spin result", async () => {
    const gameState = createInitialGameState();
    gameState.sram.coins = 100;
    const eventQueue = gameEngine.event.createQueue();
    const screen = new Surface(160, 144);
    const ui = {
      eventQueue,
      screen,
      renderSnapshot: jest.fn(),
      update: jest.fn(),
      font: { renderText: jest.fn() },
    };
    let awaitedFrames = 0;
    const frameAwaiter = async () => {
      awaitedFrames += 1;
      if (awaitedFrames === 4) {
        gameEngine.event.post(
          new gameEngine.event.Event("keydown", { button: GameButton.B, is_press: true }),
          eventQueue,
        );
      }
    };

    gameEngine.event.post(
      new gameEngine.event.Event("keydown", { button: GameButton.A, is_press: true }),
      eventQueue,
    );

    const outcome = await new SlotMachineOverlay(ui, gameState, null, {
      frameAwaiter,
      animation: { stopFrames: [2, 3, 4] },
    }).runAsync();

    expect(outcome.played).toBe(true);
    expect(awaitedFrames).toBeGreaterThanOrEqual(4);
    const snapshots = ui.renderSnapshot.mock.calls.map(([viewportLines]) => viewportLines as string[]);
    expect(snapshots.some((lines) => lines.includes("START!"))).toBe(true);
    const reelStates = new Set(
      snapshots.map((lines) => lines.slice(4, 7).join("\n")),
    );
    expect(reelStates.size).toBeGreaterThan(2);
  });
});
