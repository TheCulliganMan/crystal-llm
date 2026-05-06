// ASM mapping: pokecrystal_disassembly/engine/overworld/decorations.asm (ToggleMaptileDecorations/ToggleDecorationsVisibility).
import { GameState } from "@pokecrystal/core/core/state";
import { decorations } from "@pokecrystal/assets/content/decorations";
import { METATILE_WIDTH } from "@pokecrystal/core/engine/world/tile/constants";
import { ScriptRunner } from "./utils";

type Overworld = {
  _write_metatile?: (metatile_x: number, metatile_y: number, block_id: number) => void;
  refresh_event_flag?: (event_name: string, options?: { value?: boolean }) => void;
};

type RunnerWithOverworld = {
  overworld?: Overworld | null;
  last_condition_result?: boolean;
};

const resolveOverworld = (
  overworld: Overworld | null | undefined,
  runner: ScriptRunner | RunnerWithOverworld | null | undefined
): Overworld | null => {
  if (overworld && typeof overworld._write_metatile === "function") {
    return overworld;
  }
  const runnerOverworld = runner?.overworld as Overworld | null | undefined;
  if (runnerOverworld && typeof runnerOverworld._write_metatile === "function") {
    return runnerOverworld;
  }
  return overworld ?? runnerOverworld ?? null;
};

const blocksPerMetatile = (): number => {
  const stride = METATILE_WIDTH / 2;
  if (!Number.isFinite(stride) || stride <= 0 || !Number.isInteger(stride)) {
    throw new Error(`METATILE_WIDTH must be an even positive integer, got ${METATILE_WIDTH}.`);
  }
  return stride;
};

// ASM uses changeblock-style block coords (2x2 tiles); engine stores metatiles (METATILE_WIDTH).
const resolveBlockCoords = (x: number, y: number): [number, number] => {
  const stride = blocksPerMetatile();
  return [Math.trunc(x / stride), Math.trunc(y / stride)];
};

const resolveDecorationEntry = (decoId: number) =>
  decorations.find((entry) => entry.index === decoId) ?? null;

const resolveDecorationSpriteToken = (decoId: number): string | null => {
  if (!decoId) {
    return null;
  }
  const entry = resolveDecorationEntry(decoId);
  if (!entry) {
    throw new Error(`Unknown decoration id ${decoId}`);
  }
  if (typeof entry.sprite_value === "number") {
    return String(entry.sprite_value);
  }
  const token = String(entry.sprite_token ?? "").trim();
  if (!token || token === "0") {
    return null;
  }
  return token;
};

const resolveDecorationSpriteValue = (decoId: number): number | null => {
  if (!decoId) {
    return null;
  }
  const entry = resolveDecorationEntry(decoId);
  if (!entry) {
    throw new Error(`Unknown decoration id ${decoId}`);
  }
  if (typeof entry.sprite_value === "number") {
    return entry.sprite_value;
  }
  const token = String(entry.sprite_token ?? "").trim();
  if (!token || token === "0") {
    return null;
  }
  if (token.startsWith("$")) {
    const parsed = Number.parseInt(token.slice(1), 16);
    return Number.isNaN(parsed) ? null : parsed;
  }
  const parsed = Number(token);
  return Number.isNaN(parsed) ? null : parsed;
};

const setEventFlag = (game_state: GameState, flag: string, value: boolean): void => {
  const flags = game_state.wram.event_flags;
  if (flags && typeof flags === "object") {
    (flags as Record<string, boolean>)[flag] = value;
  }
};

const refreshEventFlag = (overworld: Overworld | null | undefined, flag: string, value: boolean): void => {
  overworld?.refresh_event_flag?.(flag, { value });
};

export function toggle_maptile_decorations(
  game_state: GameState,
  {
    runner,
    overworld,
  }: { runner?: ScriptRunner | RunnerWithOverworld; overworld?: Overworld | null } = {}
): boolean {
  // ASM: engine/overworld/decorations.asm::ToggleMaptileDecorations
  const resolvedOverworld = resolveOverworld(overworld, runner);
  const writer = resolvedOverworld?._write_metatile;
  if (typeof writer !== "function") {
    throw new Error("ToggleMaptileDecorations requires an overworld with _write_metatile.");
  }
  const writeMetatile = writer.bind(resolvedOverworld);
  const wram = game_state.wram;
  wram.maptile_decorations_visible = true;

  const bedId = Number(wram.wDecoBed ?? 0);
  if (bedId) {
    const sprite = resolveDecorationSpriteValue(bedId);
    if (sprite !== null) {
      const [x, y] = resolveBlockCoords(0, 4);
      writeMetatile(x, y, sprite);
    }
  }

  const plantId = Number(wram.wDecoPlant ?? 0);
  if (plantId) {
    const sprite = resolveDecorationSpriteValue(plantId);
    if (sprite !== null) {
      const [x, y] = resolveBlockCoords(7, 4);
      writeMetatile(x, y, sprite);
    }
  }

  const posterId = Number(wram.wDecoPoster ?? 0);
  if (posterId) {
    const sprite = resolveDecorationSpriteValue(posterId);
    if (sprite !== null) {
      const [x, y] = resolveBlockCoords(6, 0);
      writeMetatile(x, y, sprite);
    }
  }
  const posterHidden = !posterId;
  setEventFlag(game_state, "EVENT_PLAYERS_ROOM_POSTER", posterHidden);
  refreshEventFlag(resolvedOverworld, "EVENT_PLAYERS_ROOM_POSTER", posterHidden);

  const carpetId = Number(wram.wDecoCarpet ?? 0);
  if (!carpetId) {
    if (runner) {
      runner.last_condition_result = true;
    }
    return true;
  }
  const carpetSprite = resolveDecorationSpriteValue(carpetId);
  if (carpetSprite === null) {
    if (runner) {
      runner.last_condition_result = true;
    }
    return true;
  }
  {
    const [x, y] = resolveBlockCoords(0, 0);
    writeMetatile(x, y, carpetSprite);
  }
  {
    const [x0, y0] = resolveBlockCoords(0, 2);
    const [x1, y1] = resolveBlockCoords(1, 2);
    const [x2, y2] = resolveBlockCoords(2, 2);
    writeMetatile(x0, y0, carpetSprite + 1);
    writeMetatile(x1, y1, carpetSprite + 2);
    writeMetatile(x2, y2, carpetSprite + 1);
  }
  if (runner) {
    runner.last_condition_result = true;
  }
  return true;
}

const applyDecorationVisibility = (
  game_state: GameState,
  overworld: Overworld | null | undefined,
  {
    decorationId,
    eventFlag,
    spriteBase,
  }: { decorationId: number; eventFlag: string; spriteBase: string }
): void => {
  const variableSprites = game_state.wram.variable_sprites ?? {};
  const sprite = resolveDecorationSpriteToken(decorationId);
  if (sprite) {
    variableSprites[spriteBase] = sprite;
    setEventFlag(game_state, eventFlag, false);
    refreshEventFlag(overworld, eventFlag, false);
  } else {
    delete variableSprites[spriteBase];
    setEventFlag(game_state, eventFlag, true);
    refreshEventFlag(overworld, eventFlag, true);
  }
  game_state.wram.variable_sprites = variableSprites;
};

export function toggle_decorations_visibility(
  game_state: GameState,
  {
    runner,
    overworld,
  }: { runner?: ScriptRunner; overworld?: Overworld | null } = {}
): boolean {
  // ASM: engine/overworld/decorations.asm::ToggleDecorationsVisibility
  const resolvedOverworld = resolveOverworld(overworld, runner);
  const wram = game_state.wram;
  wram.decorations_visible = true;

  applyDecorationVisibility(game_state, resolvedOverworld, {
    decorationId: Number(wram.wDecoConsole ?? 0),
    eventFlag: "EVENT_PLAYERS_HOUSE_2F_CONSOLE",
    spriteBase: "SPRITE_CONSOLE",
  });
  applyDecorationVisibility(game_state, resolvedOverworld, {
    decorationId: Number(wram.wDecoLeftOrnament ?? 0),
    eventFlag: "EVENT_PLAYERS_HOUSE_2F_DOLL_1",
    spriteBase: "SPRITE_DOLL_1",
  });
  applyDecorationVisibility(game_state, resolvedOverworld, {
    decorationId: Number(wram.wDecoRightOrnament ?? 0),
    eventFlag: "EVENT_PLAYERS_HOUSE_2F_DOLL_2",
    spriteBase: "SPRITE_DOLL_2",
  });
  applyDecorationVisibility(game_state, resolvedOverworld, {
    decorationId: Number(wram.wDecoBigDoll ?? 0),
    eventFlag: "EVENT_PLAYERS_HOUSE_2F_BIG_DOLL",
    spriteBase: "SPRITE_BIG_DOLL",
  });

  if (runner) {
    runner.last_condition_result = true;
  }
  return true;
}
