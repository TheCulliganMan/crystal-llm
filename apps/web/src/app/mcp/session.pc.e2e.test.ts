import fs from "node:fs";
import path from "node:path";
import { PNG } from "pngjs";
import { getMcpSession, __testing } from "@/app/mcp/session";
import { pressHandler } from "../api/[transport]/tools/input";
import { observeHandler } from "../api/[transport]/tools/observe";
import { BoxSchema, formatDefaultBoxName, type PokemonSpecies } from "@pokecrystal/core/core/models";
import { setSlot } from "@pokecrystal/core/core/models/box";
import { createPokemon } from "@pokecrystal/core/engine/systems/pokemon";
import { toPokemon } from "@pokecrystal/core/core/models/pokemon";
import { Ability, EggGroup, GenderRatio, GrowthRate, PokemonType } from "@pokecrystal/core/core/enums";
import { PokemonPCMenu, type SupportsPokemonPCUI } from "@pokecrystal/core/ui/menus/pc-components";
import { getPcCursorTile } from "@pokecrystal/core/ui/menus/pc-wallpaper";
import type { Surface } from "@pokecrystal/core/ui/surface";

const removeAutosave = (sessionId: string): void => {
  for (const baseDir of [process.cwd(), path.resolve(process.cwd(), "apps/web")]) {
    const slot = path.resolve(baseDir, `mcp-${sessionId}-autosave.sav`);
    const runtime = path.resolve(baseDir, `mcp-${sessionId}-runtime.json`);
    fs.rmSync(slot, { force: true });
    fs.rmSync(`${slot}.bak`, { force: true });
    fs.rmSync(runtime, { force: true });
  }
};

const DEFAULT_BASE_STATS = {
  hp: 20,
  attack: 10,
  defense: 10,
  speed: 10,
  special_attack: 10,
  special_defense: 10,
};

const speciesCache = new Map<string, PokemonSpecies>();

const species = (id: string): PokemonSpecies => {
  const upperId = id.toUpperCase();
  const cached = speciesCache.get(upperId);
  if (cached) {
    return cached;
  }
  const value: PokemonSpecies = {
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
  speciesCache.set(upperId, value);
  return value;
};

const positionOverworldPlayer = (
  overworld: {
    load_map: (mapName: string) => void;
    player_x: number;
    player_y: number;
    prev_player_x: number;
    prev_player_y: number;
    target_tile_x: number;
    target_tile_y: number;
    player_direction: string;
    player_object?: {
      x: number;
      y: number;
      prevX: number;
      prevY: number;
      direction: string;
      updatePixelPosition?: () => void;
    } | null;
    _sync_player_state?: () => void;
  },
  mapName: string,
  position: { x: number; y: number; direction: string }
): void => {
  overworld.load_map(mapName);
  overworld.player_x = position.x;
  overworld.player_y = position.y;
  overworld.prev_player_x = position.x;
  overworld.prev_player_y = position.y;
  overworld.target_tile_x = position.x;
  overworld.target_tile_y = position.y;
  overworld.player_direction = position.direction;
  if (overworld.player_object) {
    overworld.player_object.x = position.x;
    overworld.player_object.y = position.y;
    overworld.player_object.prevX = position.x;
    overworld.player_object.prevY = position.y;
    overworld.player_object.direction = position.direction;
    overworld.player_object.updatePixelPosition?.();
  }
  overworld._sync_player_state?.();
};

type McpTestOverworld = {
  load_map: (mapName: string) => void;
  current_map_name?: string;
  player_x: number;
  player_y: number;
  prev_player_x: number;
  prev_player_y: number;
  target_tile_x: number;
  target_tile_y: number;
  player_direction: string;
  player_object?: {
    x: number;
    y: number;
    prevX: number;
    prevY: number;
    direction: string;
    updatePixelPosition?: () => void;
  } | null;
  _sync_player_state?: () => void;
  script_runner?: { variables?: Record<string, unknown>; last_value?: unknown };
  get_facing_tile_coords?: () => [number, number];
  _counter_adjusted_tile?: (x: number, y: number) => [number, number];
  _bg_event_at?: (x: number, y: number) => { script?: string } | null;
};

type McpTestSession = {
  game: {
    getGameState: () => ReturnType<typeof import("@pokecrystal/core/core/state").createInitialGameState>;
    getOverworld: () => McpTestOverworld;
    draw?: () => void;
  };
  ui?: SupportsPokemonPCUI;
};

const callPressAThroughMcp = (sessionId: string) =>
  pressHandler(
    { button: "a", times: 1, format: "json", detail: "full", include_snapshot_text: true },
    {
      requestInfo: {
        headers: {
          "mcp-session-id": sessionId,
          "x-pokecrystal-session-mode": "interactive",
        },
      },
    }
  );

const callObserveImageThroughMcp = (sessionId: string) =>
  observeHandler(
    { include_snapshot_text: false, include_image: true, image_scale: 1 },
    {
      requestInfo: {
        headers: {
          "mcp-session-id": sessionId,
          "x-pokecrystal-session-mode": "interactive",
        },
      },
    }
  );

type PngLike = {
  width: number;
  height: number;
  data: Buffer | Uint8Array;
};

const decodePng = (base64: string): PngLike => PNG.sync.read(Buffer.from(base64, "base64")) as PngLike;

const pngPixel = (png: PngLike, x: number, y: number): [number, number, number, number] => {
  const index = (y * png.width + x) * 4;
  return [png.data[index], png.data[index + 1], png.data[index + 2], png.data[index + 3]];
};

const surfacePixel = (surface: Surface, x: number, y: number): [number, number, number, number] =>
  surface.get_at([x, y]);

const dominantSurfacePixel = (surface: Surface): [number, number, number, number] => {
  const counts = new Map<string, number>();
  for (let row = 0; row < 8; row += 1) {
    for (let col = 0; col < 8; col += 1) {
      const key = surfacePixel(surface, col, row).join(",");
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

const expectPngObjectPixelsAt = (png: PngLike, x: number, y: number, expected: Surface): void => {
  const transparent = dominantSurfacePixel(expected).join(",");
  for (let row = 0; row < 8; row += 1) {
    for (let col = 0; col < 8; col += 1) {
      const expectedPixel = surfacePixel(expected, col, row);
      if (expectedPixel.join(",") === transparent) {
        continue;
      }
      expect(pngPixel(png, x + col, y + row)).toEqual(expectedPixel);
    }
  }
};

const pngHasNonWhitePixel = (
  png: PngLike,
  rect: { x: number; y: number; width: number; height: number }
): boolean => {
  for (let y = rect.y; y < rect.y + rect.height; y += 1) {
    for (let x = rect.x; x < rect.x + rect.width; x += 1) {
      const [r, g, b, a] = pngPixel(png, x, y);
      if (a > 0 && (r !== 255 || g !== 255 || b !== 255)) {
        return true;
      }
    }
  }
  return false;
};

const pngContainsSyntheticCursorBlue = (png: PngLike): boolean => {
  for (let y = 0; y < png.height; y += 1) {
    for (let x = 0; x < png.width; x += 1) {
      const [r, g, b, a] = pngPixel(png, x, y);
      if (r === 82 && g === 160 && b === 255 && a === 160) {
        return true;
      }
    }
  }
  return false;
};

describe("MCP session PC end-to-end", () => {
  beforeEach(() => {
    __testing.clearSessions();
  });

  afterEach(() => {
    __testing.clearSessions();
  });

  it("deposits a Pokemon into Bill's PC through the live MCP input path", async () => {
    const sessionId = "pc-bill-deposit-e2e";
    removeAutosave(sessionId);
    const session = getMcpSession(sessionId);
    await session.ensureReady();
    const sessionAny = session as unknown as McpTestSession;
    const state = sessionAny.game.getGameState();
    const lead = toPokemon(createPokemon(state, species("CYNDAQUIL"), 5));
    const deposited = toPokemon(createPokemon(state, species("TOTODILE"), 7));
    state.sram.player_name = "CHRIS";
    state.sram.party.pokemon = [lead, deposited, null, null, null, null];
    state.sram.pc_boxes = [BoxSchema.parse({ name: formatDefaultBoxName(0) })];

    const overworld = sessionAny.game.getOverworld();
    positionOverworldPlayer(overworld, "CherrygrovePokecenter1F", { x: 19, y: 5, direction: "up" });
    expect(sessionAny.game.getOverworld().player_x).toBe(19);
    expect(sessionAny.game.getOverworld().player_y).toBe(5);
    expect(sessionAny.game.getOverworld().current_map_name).toBe("CherrygrovePokecenter1F");
    const [rawX, rawY] = overworld.get_facing_tile_coords?.() ?? [0, 0];
    const [pcX, pcY] = overworld._counter_adjusted_tile?.(rawX, rawY) ?? [rawX, rawY];
    expect(overworld._bg_event_at?.(pcX, pcY)?.script).toBe("PCScript");
    overworld.script_runner!.variables = {
      ...(overworld.script_runner!.variables ?? {}),
      _pokemon_center_pc_selection: "BILL's PC",
      _pc_bill_actions: [{ action: "deposit", party_slot: 1, box: 0, slot: 0 }],
    };

    const result = await callPressAThroughMcp(sessionId);

    expect(result.isError).not.toBe(true);
    expect(state.sram.party.pokemon[0]?.species.id).toBe("CYNDAQUIL");
    expect(state.sram.party.pokemon[1]).toBeNull();
    expect(state.sram.pc_boxes[0].pokemon[0]?.species.id).toBe("TOTODILE");
    expect(overworld.script_runner?.last_value).toEqual(
      expect.objectContaining({
        selection_name: "BILL'S PC",
        bill: expect.objectContaining({
          actions: expect.arrayContaining([
            expect.objectContaining({ action: "deposit", status: "ok", species: "TOTODILE" }),
          ]),
        }),
      })
    );
  }, 20000);

  it("deposits an item into the player's PC through the live MCP input path", async () => {
    const sessionId = "pc-item-deposit-e2e";
    removeAutosave(sessionId);
    const session = getMcpSession(sessionId);
    await session.ensureReady();
    const sessionAny = session as unknown as McpTestSession;
    const state = sessionAny.game.getGameState();
    state.sram.player_name = "CHRIS";
    state.sram.party.pokemon = [toPokemon(createPokemon(state, species("CYNDAQUIL"), 5)), null, null, null, null, null];
    state.sram.items.POTION = 5;
    state.sram.pc_items = [];

    const overworld = sessionAny.game.getOverworld();
    positionOverworldPlayer(overworld, "CherrygrovePokecenter1F", { x: 19, y: 5, direction: "up" });
    expect(sessionAny.game.getOverworld().player_x).toBe(19);
    expect(sessionAny.game.getOverworld().player_y).toBe(5);
    expect(sessionAny.game.getOverworld().current_map_name).toBe("CherrygrovePokecenter1F");
    const [rawX, rawY] = overworld.get_facing_tile_coords?.() ?? [0, 0];
    const [pcX, pcY] = overworld._counter_adjusted_tile?.(rawX, rawY) ?? [rawX, rawY];
    expect(overworld._bg_event_at?.(pcX, pcY)?.script).toBe("PCScript");
    overworld.script_runner!.variables = {
      ...(overworld.script_runner!.variables ?? {}),
      _pokemon_center_pc_selection: "CHRIS's PC",
      _pc_player_actions: [{ action: "deposit", item: "POTION", quantity: 3 }],
    };

    const result = await callPressAThroughMcp(sessionId);

    expect(result.isError).not.toBe(true);
    expect(state.sram.items.POTION).toBe(2);
    expect(state.sram.pc_items).toEqual([{ item: "POTION", quantity: 3 }]);
    expect(overworld.script_runner?.last_value).toEqual(
      expect.objectContaining({
        selection_name: "CHRIS'S PC",
        player_pc: expect.objectContaining({
          actions: expect.arrayContaining([
            expect.objectContaining({ action: "deposit", status: "ok", item: "POTION", quantity: 3 }),
          ]),
        }),
      })
    );
  }, 20000);

  it("withdraws a Pokemon from Bill's PC through the live MCP input path", async () => {
    const sessionId = "pc-bill-withdraw-e2e";
    removeAutosave(sessionId);
    const session = getMcpSession(sessionId);
    await session.ensureReady();
    const sessionAny = session as unknown as McpTestSession;
    const state = sessionAny.game.getGameState();
    const lead = toPokemon(createPokemon(state, species("CYNDAQUIL"), 5));
    const stored = toPokemon(createPokemon(state, species("TOTODILE"), 7));
    state.sram.player_name = "CHRIS";
    state.sram.party.pokemon = [lead, null, null, null, null, null];
    const box = BoxSchema.parse({ name: formatDefaultBoxName(0) });
    setSlot(box, 0, stored);
    state.sram.pc_boxes = [box];

    const overworld = sessionAny.game.getOverworld();
    positionOverworldPlayer(overworld, "CherrygrovePokecenter1F", { x: 19, y: 5, direction: "up" });
    expect(sessionAny.game.getOverworld().player_x).toBe(19);
    expect(sessionAny.game.getOverworld().player_y).toBe(5);
    expect(sessionAny.game.getOverworld().current_map_name).toBe("CherrygrovePokecenter1F");
    const [rawX, rawY] = overworld.get_facing_tile_coords?.() ?? [0, 0];
    const [pcX, pcY] = overworld._counter_adjusted_tile?.(rawX, rawY) ?? [rawX, rawY];
    expect(overworld._bg_event_at?.(pcX, pcY)?.script).toBe("PCScript");
    overworld.script_runner!.variables = {
      ...(overworld.script_runner!.variables ?? {}),
      _pokemon_center_pc_selection: "BILL's PC",
      _pc_bill_actions: [{ action: "withdraw", box: 0, slot: 0 }],
    };

    const result = await callPressAThroughMcp(sessionId);

    expect(result.isError).not.toBe(true);
    expect(state.sram.party.pokemon[0]?.species.id).toBe("CYNDAQUIL");
    expect(state.sram.party.pokemon[1]?.species.id).toBe("TOTODILE");
    expect(state.sram.pc_boxes[0].pokemon[0]).toBeNull();
    expect(overworld.script_runner?.last_value).toEqual(
      expect.objectContaining({
        selection_name: "BILL'S PC",
        bill: expect.objectContaining({
          actions: expect.arrayContaining([
            expect.objectContaining({ action: "withdraw", status: "ok", species: "TOTODILE" }),
          ]),
        }),
      })
    );
  }, 20000);

  it("withdraws an item from the player's PC through the live MCP input path", async () => {
    const sessionId = "pc-item-withdraw-e2e";
    removeAutosave(sessionId);
    const session = getMcpSession(sessionId);
    await session.ensureReady();
    const sessionAny = session as unknown as McpTestSession;
    const state = sessionAny.game.getGameState();
    state.sram.player_name = "CHRIS";
    state.sram.party.pokemon = [toPokemon(createPokemon(state, species("CYNDAQUIL"), 5)), null, null, null, null, null];
    state.sram.items.POTION = 1;
    state.sram.pc_items = [{ item: "POTION", quantity: 3 }];

    const overworld = sessionAny.game.getOverworld();
    positionOverworldPlayer(overworld, "CherrygrovePokecenter1F", { x: 19, y: 5, direction: "up" });
    expect(sessionAny.game.getOverworld().player_x).toBe(19);
    expect(sessionAny.game.getOverworld().player_y).toBe(5);
    expect(sessionAny.game.getOverworld().current_map_name).toBe("CherrygrovePokecenter1F");
    const [rawX, rawY] = overworld.get_facing_tile_coords?.() ?? [0, 0];
    const [pcX, pcY] = overworld._counter_adjusted_tile?.(rawX, rawY) ?? [rawX, rawY];
    expect(overworld._bg_event_at?.(pcX, pcY)?.script).toBe("PCScript");
    overworld.script_runner!.variables = {
      ...(overworld.script_runner!.variables ?? {}),
      _pokemon_center_pc_selection: "CHRIS's PC",
      _pc_player_actions: [{ action: "withdraw", item: "POTION", quantity: 2 }],
    };

    const result = await callPressAThroughMcp(sessionId);

    expect(result.isError).not.toBe(true);
    expect(state.sram.items.POTION).toBe(3);
    expect(state.sram.pc_items).toEqual([{ item: "POTION", quantity: 1 }]);
    expect(overworld.script_runner?.last_value).toEqual(
      expect.objectContaining({
        selection_name: "CHRIS'S PC",
        player_pc: expect.objectContaining({
          actions: expect.arrayContaining([
            expect.objectContaining({ action: "withdraw", status: "ok", item: "POTION", quantity: 2 }),
          ]),
        }),
      })
    );
  }, 20000);

  it("returns the corrected Bill's PC render through MCP observe include_image", async () => {
    const sessionId = "pc-bill-observe-image-e2e";
    removeAutosave(sessionId);
    const session = getMcpSession(sessionId);
    await session.ensureReady();
    const sessionAny = session as unknown as McpTestSession;
    const state = sessionAny.game.getGameState();
    const box = BoxSchema.parse({ name: formatDefaultBoxName(0) });
    setSlot(box, 0, toPokemon(createPokemon(state, species("TOTODILE"), 8)));
    setSlot(box, 1, toPokemon(createPokemon(state, species("GEODUDE"), 7)));
    state.sram.pc_boxes = [box];

    if (!sessionAny.ui) {
      throw new Error("MCP session did not expose a render UI for PC image coverage.");
    }
    const menu = new PokemonPCMenu(sessionAny.ui, state, null);
    const originalDraw = sessionAny.game.draw;
    sessionAny.game.draw = () => menu.draw();

    try {
      const result = await callObserveImageThroughMcp(sessionId);
      const image = result.content.find((entry) => entry.type === "image");
      expect(image?.mimeType).toBe("image/png");
      expect(typeof image?.data).toBe("string");

      const png = decodePng(image!.data!);
      expect([png.width, png.height]).toEqual([160, 144]);
      expect(pngHasNonWhitePixel(png, { x: 8, y: 32, width: 56, height: 56 })).toBe(true);
      expectPngObjectPixelsAt(png, 72, 22, getPcCursorTile(0x00));
      expectPngObjectPixelsAt(png, 70, 30, getPcCursorTile(0x01));
      expect(pngContainsSyntheticCursorBlue(png)).toBe(false);
    } finally {
      sessionAny.game.draw = originalDraw;
    }
  }, 20000);
});
