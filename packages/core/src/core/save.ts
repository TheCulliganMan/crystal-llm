import { promises as fs } from 'fs';
import os from 'os';
import path from 'path';
import { GameState, GameStateSchema } from './state';
import logger from './logger';
import { FrameType, PrintOption } from './enums/ui-enums';
import { getSettings } from './config';
import {
  deleteBrowserCloudSave,
  deleteIdentityCloudSave,
  isBrowserCloudSaveConfigured,
  loadBrowserCloudSave,
  loadIdentityCloudSave,
  saveBrowserCloudSave,
  saveIdentityCloudSave,
} from '@pokecrystal/core/adapters/cloud-save';
import { MAX_COINS, MAX_PC_BOXES, MAX_PC_ITEMS, MAX_PC_ITEM_QUANTITY } from './constants';
import { calculateExperience } from '@pokecrystal/core/engine/experience';
import { MAX_ITEM_STACK, POCKET_CAPACITY } from '@pokecrystal/core/engine/systems/items';
import type { PokemonData } from './models/pokemon';
import { GrowthRate } from './enums/pokemon';
import { validateBoxState } from './models/box';
import {
  deleteGuestSessionSlot,
  listGuestSessionSlotCandidates,
  readGuestSessionMetadata,
  readGuestSessionSlot,
  writeGuestSessionMetadata,
  writeGuestSessionSlot,
} from './guest-session-storage';

type ErrnoException = NodeJS.ErrnoException;

const SAVE_EXTENSION = '.sav';
const BACKUP_EXTENSION = '.bak';
const SAVE_METADATA_EXTENSION = '.meta.json';
const IDENTITY_SLOT_SEPARATOR = '__';

const isErrnoException = (value: unknown): value is ErrnoException =>
  typeof value === 'object' &&
  value !== null &&
  'code' in value &&
  typeof (value as ErrnoException).code === 'string';

const isQuotaExceededError = (value: unknown): boolean => {
  if (typeof value !== 'object' || value === null) {
    return false;
  }

  const err = value as { name?: unknown; message?: unknown; code?: unknown };
  if (err.name === 'QuotaExceededError' || err.name === 'NS_ERROR_DOM_QUOTA_REACHED') {
    return true;
  }

  if (err.code === 22 || err.code === 1014) {
    return true;
  }

  return (
    typeof err.message === 'string' &&
    err.message.toLowerCase().includes('exceeded the quota')
  );
};

export class SaveGameError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'SaveGameError';
  }
}

export class SaveFileNotFoundError extends SaveGameError {
  constructor(message: string) {
    super(message);
    this.name = 'SaveFileNotFoundError';
  }
}

export class SaveGameValidationError extends SaveGameError {
  constructor(message: string) {
    super(message);
    this.name = 'SaveGameValidationError';
  }
}

function expandUserPath(value: string): string {
  if (value.startsWith('~')) {
    return path.join(os.homedir(), value.slice(1));
  }
  return value;
}

function getSaveRootDirectory(): string {
  const { saveRoot } = getSettings();
  if (saveRoot) {
    return path.resolve(expandUserPath(saveRoot));
  }
  if (
    typeof window === 'undefined' &&
    (process.env.VERCEL === '1' ||
      Boolean(process.env.AWS_LAMBDA_FUNCTION_NAME) ||
      Boolean(process.env.LAMBDA_TASK_ROOT))
  ) {
    return os.tmpdir();
  }
  return process.cwd();
}

function normaliseSavePath(filepath: string): string {
  let p = filepath;
  if (!path.isAbsolute(p)) {
    p = path.resolve(getSaveRootDirectory(), p);
  }
  if (path.extname(p) !== SAVE_EXTENSION) {
    const parsed = path.parse(p);
    p = path.join(parsed.dir, `${parsed.name}${SAVE_EXTENSION}`);
  }
  return p;
}

function backupSavePath(p: string): string {
  return `${p}${BACKUP_EXTENSION}`;
}

function metadataSavePath(p: string): string {
  return `${p}${SAVE_METADATA_EXTENSION}`;
}

function resolveSaveSlot(filepath: string): string {
  const normalized = normaliseSavePath(filepath);
  return path.basename(normalized);
}

function shouldUseSupabase(): boolean {
  return typeof window !== 'undefined' && isBrowserCloudSaveConfigured();
}

function isBrowserRuntime(): boolean {
  return typeof window !== 'undefined';
}

function shouldValidateSaveRoundTrip(): boolean {
  // The browser save path already snapshots through the schema once. Re-running
  // a second full parse+diff in production browsers adds noticeable save latency
  // without improving deployed-user recovery.
  if (typeof window !== 'undefined') {
    return process.env.NODE_ENV !== 'production';
  }
  return true;
}

type McpIdentityContextValue = {
  playerId: string;
  token: string;
  name?: string | null;
};

type ScopedSaveTarget = {
  slot: string;
  localSlot: string;
  localPath: string;
};

type SaveMetadata = {
  saved_at: string;
};

type SaveCandidate = {
  source: 'remote' | 'guest-session' | 'filesystem';
  gameState: GameState;
  timestampMs: number | null;
  priority: number;
};

type SaveDiscoveryResult = {
  candidates: SaveCandidate[];
  loadErrors: SaveGameError[];
};

const resolveScopedTarget = (filepath: string, identity: McpIdentityContextValue | null): ScopedSaveTarget => {
  const slot = resolveSaveSlot(filepath);
  if (!identity?.playerId) {
    return {
      slot,
      localSlot: slot,
      localPath: normaliseSavePath(filepath),
    };
  }
  const localSlot = `${identity.playerId}${IDENTITY_SLOT_SEPARATOR}${slot}`;
  return {
    slot,
    localSlot,
    localPath: normaliseSavePath(localSlot),
  };
};

async function loadMcpIdentityContext(): Promise<McpIdentityContextValue | null> {
  if (typeof window !== 'undefined') {
    return null;
  }
  try {
    const requireFn = (0, eval)('require') as NodeRequire | undefined;
    if (typeof requireFn !== 'function') {
      return null;
    }
    const module = requireFn('./mcp-identity-context.server') as {
      getMcpIdentityContext: () => McpIdentityContextValue | null;
    };
    return module.getMcpIdentityContext();
  } catch {
    return null;
  }
}

const parseSaveTimestampMs = (value: string | null | undefined): number | null => {
  if (!value) {
    return null;
  }
  const timestamp = Date.parse(value);
  if (!Number.isFinite(timestamp)) {
    return null;
  }
  return timestamp;
};

const candidatePriority = (source: SaveCandidate['source']): number => {
  if (source === 'remote') {
    return 0;
  }
  if (source === 'guest-session') {
    return 1;
  }
  return 2;
};

const buildCandidate = (
  source: SaveCandidate['source'],
  gameState: GameState,
  savedAt: string | null
): SaveCandidate => ({
  source,
  gameState,
  timestampMs: parseSaveTimestampMs(savedAt),
  priority: candidatePriority(source),
});

const chooseBestSaveCandidate = (candidates: SaveCandidate[]): SaveCandidate | null => {
  if (candidates.length === 0) {
    return null;
  }

  const timestamped = candidates.filter((candidate) => candidate.timestampMs !== null);
  if (timestamped.length > 0) {
    return timestamped.reduce((best, candidate) => {
      if (best.timestampMs === null) {
        return candidate;
      }
      if ((candidate.timestampMs ?? -1) > best.timestampMs) {
        return candidate;
      }
      if ((candidate.timestampMs ?? -1) < best.timestampMs) {
        return best;
      }
      return candidate.priority < best.priority ? candidate : best;
    });
  }

  return candidates.reduce((best, candidate) =>
    candidate.priority < best.priority ? candidate : best
  );
};

async function writeSaveText(p: string, payload: string): Promise<void> {
  await fs.mkdir(path.dirname(p), { recursive: true });
  const tempPath = `${p}.${process.pid}.${Date.now()}.tmp`;
  try {
    await fs.writeFile(tempPath, payload, 'utf-8');
    await fs.rename(tempPath, p);
  } catch (err) {
    await fs.unlink(tempPath).catch(() => undefined);
    throw err;
  }
}

async function writeSaveMetadataText(p: string, savedAt: string): Promise<void> {
  await writeSaveText(metadataSavePath(p), JSON.stringify({ saved_at: savedAt }, null, 4));
}

async function readSaveMetadataText(p: string): Promise<SaveMetadata | null> {
  try {
    const rawText = await fs.readFile(metadataSavePath(p), 'utf-8');
    const parsed = JSON.parse(rawText) as Partial<SaveMetadata> | null;
    const savedAt = typeof parsed?.saved_at === 'string' ? parsed.saved_at : null;
    if (!savedAt) {
      return null;
    }
    return { saved_at: savedAt };
  } catch (err) {
    if (isErrnoException(err) && err.code === 'ENOENT') {
      return null;
    }
    logger.warn(`[save] Unable to read save metadata for ${p}: ${String(err)}`);
    return null;
  }
}

async function copySaveMetadataText(source: string, destination: string): Promise<boolean> {
  const metadata = await readSaveMetadataText(source);
  if (!metadata) {
    return false;
  }
  try {
    await writeSaveMetadataText(destination, metadata.saved_at);
    return true;
  } catch (err) {
    logger.warn(
      `[save] Unable to copy save metadata from ${source} to ${destination}: ${String(err)}`
    );
    return false;
  }
}

async function copyFileAtomic(source: string, destination: string): Promise<void> {
  await fs.mkdir(path.dirname(destination), { recursive: true });
  const tempPath = `${destination}.${process.pid}.${Date.now()}.tmp`;
  try {
    await fs.copyFile(source, tempPath);
    await fs.rename(tempPath, destination);
  } catch (err) {
    await fs.unlink(tempPath).catch(() => undefined);
    throw err;
  }
}

async function backupExistingSaveFile(p: string): Promise<void> {
  try {
    await loadGameFromPath(p);
  } catch (err) {
    if (err instanceof SaveFileNotFoundError) {
      return;
    }
    if (err instanceof SaveGameValidationError) {
      logger.warn(`[save] Existing save ${p} is invalid; leaving ${backupSavePath(p)} unchanged`);
      return;
    }
    throw err;
  }

  await copyFileAtomic(p, backupSavePath(p));
}

const isAutosaveSlot = (slot: string): boolean =>
  slot.toLowerCase().includes('autosave');

const FRAME_VALUES = Object.values(FrameType).filter(
  (value): value is number => typeof value === 'number'
);
const MAX_FRAME_VALUE = Math.max(...FRAME_VALUES);
const PRINT_OPTION_VALUES = Object.values(PrintOption).filter(
  (value): value is number => typeof value === 'number'
);
const MAX_STAT_EXP = 65535;
const MAX_LEVEL = 100;
const MAX_HP = 65535;

const assertIntegerInRange = (
  value: number,
  min: number,
  max: number,
  label: string
): void => {
  if (!Number.isFinite(value) || Math.floor(value) !== value || value < min || value > max) {
    throw new SaveGameValidationError(
      `Invalid ${label}: expected integer in range ${min}-${max}, got ${String(value)}.`
    );
  }
};

const assertNonNegativeInteger = (value: number, label: string): void => {
  assertIntegerInRange(value, 0, Number.MAX_SAFE_INTEGER, label);
};

const validateInventory = (
  inventory: Record<string, number>,
  label: string,
  stackLimit: number,
  maxSlots: number | null
): void => {
  const entries = Object.entries(inventory ?? {});
  for (const [name, quantity] of entries) {
    assertIntegerInRange(quantity, 0, stackLimit, `${label}.${name}`);
  }
  if (maxSlots !== null) {
    const activeSlots = entries.filter(([, quantity]) => quantity > 0).length;
    if (activeSlots > maxSlots) {
      throw new SaveGameValidationError(
        `Invalid ${label}: expected at most ${maxSlots} slots, got ${activeSlots}.`
      );
    }
  }
};

const derivedHpDv = (dvs: PokemonData['dvs']): number => {
  const attack = Math.trunc(dvs.attack) & 0xf;
  const defense = Math.trunc(dvs.defense) & 0xf;
  const speed = Math.trunc(dvs.speed) & 0xf;
  const special = Math.trunc(dvs.special) & 0xf;
  let hp = 0;
  if (attack & 1) hp += 8;
  if (defense & 1) hp += 4;
  if (speed & 1) hp += 2;
  if (special & 1) hp += 1;
  return hp;
};

const validatePokemonIntegrity = (pokemon: PokemonData, label: string): void => {
  assertIntegerInRange(pokemon.level, 1, MAX_LEVEL, `${label}.level`);
  assertIntegerInRange(pokemon.happiness, 0, 255, `${label}.happiness`);

  const dvs = pokemon.dvs;
  assertIntegerInRange(dvs.attack, 0, 15, `${label}.dvs.attack`);
  assertIntegerInRange(dvs.defense, 0, 15, `${label}.dvs.defense`);
  assertIntegerInRange(dvs.speed, 0, 15, `${label}.dvs.speed`);
  assertIntegerInRange(dvs.special, 0, 15, `${label}.dvs.special`);
  assertIntegerInRange(dvs.hp, 0, 15, `${label}.dvs.hp`);

  const expectedHpDv = derivedHpDv(dvs);
  if (dvs.hp !== expectedHpDv) {
    throw new SaveGameValidationError(
      `Invalid ${label}.dvs.hp: expected ${expectedHpDv} from DV parity, got ${dvs.hp}.`
    );
  }

  assertIntegerInRange(pokemon.hp_exp, 0, MAX_STAT_EXP, `${label}.hp_exp`);
  assertIntegerInRange(pokemon.attack_exp, 0, MAX_STAT_EXP, `${label}.attack_exp`);
  assertIntegerInRange(pokemon.defense_exp, 0, MAX_STAT_EXP, `${label}.defense_exp`);
  assertIntegerInRange(pokemon.speed_exp, 0, MAX_STAT_EXP, `${label}.speed_exp`);
  assertIntegerInRange(pokemon.special_exp, 0, MAX_STAT_EXP, `${label}.special_exp`);

  assertIntegerInRange(pokemon.max_hp, 1, MAX_HP, `${label}.max_hp`);
  assertIntegerInRange(pokemon.hp, 0, pokemon.max_hp, `${label}.hp`);

  if (pokemon.moves.length > 4) {
    throw new SaveGameValidationError(
      `Invalid ${label}.moves: expected at most 4 moves, got ${pokemon.moves.length}.`
    );
  }

  assertNonNegativeInteger(pokemon.experience, `${label}.experience`);
  const growthRate = pokemon.species.growth_rate as GrowthRate;
  let minimumExp = 0;
  let nextLevelExp = 0;
  try {
    minimumExp = Math.max(0, calculateExperience(growthRate, pokemon.level));
    if (pokemon.level < MAX_LEVEL) {
      nextLevelExp = Math.max(0, calculateExperience(growthRate, pokemon.level + 1));
    }
  } catch (err) {
    throw new SaveGameValidationError(
      `Invalid ${label}.species.growth_rate: ${err instanceof Error ? err.message : String(err)}`
    );
  }

  if (pokemon.experience < minimumExp) {
    throw new SaveGameValidationError(
      `Invalid ${label}.experience: ${pokemon.experience} below minimum ${minimumExp} for level ${pokemon.level}.`
    );
  }
  if (pokemon.level < MAX_LEVEL && nextLevelExp > minimumExp && pokemon.experience >= nextLevelExp) {
    throw new SaveGameValidationError(
      `Invalid ${label}.experience: ${pokemon.experience} exceeds next level threshold ${nextLevelExp}.`
    );
  }
};

function validateGameStateIntegrity(gameState: GameState): void {
  assertIntegerInRange(gameState.sram.coins ?? 0, 0, MAX_COINS, 'sram.coins');

  validateInventory(
    gameState.sram.items ?? {},
    'sram.items',
    MAX_ITEM_STACK,
    POCKET_CAPACITY.ITEM
  );
  validateInventory(
    gameState.sram.balls ?? {},
    'sram.balls',
    MAX_ITEM_STACK,
    POCKET_CAPACITY.BALL
  );
  validateInventory(
    gameState.sram.key_items ?? {},
    'sram.key_items',
    1,
    POCKET_CAPACITY.KEY_ITEM
  );
  const tmhmFlags = gameState.sram.tm_hm ?? [];
  tmhmFlags.forEach((value, index) => {
    assertIntegerInRange(value, 0, 1, `sram.tm_hm[${index}]`);
  });

  const party = gameState.sram.party?.pokemon ?? [];
  party.forEach((pokemon, index) => {
    if (pokemon) {
      validatePokemonIntegrity(pokemon, `party[${index}]`);
    }
  });

  const boxes = gameState.sram.pc_boxes ?? [];
  if (boxes.length > MAX_PC_BOXES) {
    throw new SaveGameValidationError(
      `Invalid pc_boxes: expected at most ${MAX_PC_BOXES} boxes, got ${boxes.length}.`
    );
  }
  boxes.forEach((box, boxIndex) => {
    try {
      validateBoxState(box, `pc_boxes[${boxIndex}]`);
    } catch (err) {
      throw new SaveGameValidationError(
        `Invalid pc_boxes[${boxIndex}]: ${err instanceof Error ? err.message : String(err)}`
      );
    }
    box.pokemon.forEach((pokemon, slotIndex) => {
      if (pokemon) {
        validatePokemonIntegrity(pokemon, `pc_boxes[${boxIndex}].pokemon[${slotIndex}]`);
      }
    });
  });

  const pcItems = gameState.sram.pc_items ?? [];
  if (pcItems.length > MAX_PC_ITEMS) {
    throw new SaveGameValidationError(
      `Invalid pc_items: expected at most ${MAX_PC_ITEMS} slots, got ${pcItems.length}.`
    );
  }
  pcItems.forEach((slot, index) => {
    assertIntegerInRange(slot.quantity, 0, MAX_PC_ITEM_QUANTITY, `pc_items[${index}].quantity`);
  });
}

function normalizePrintOption(value: unknown): PrintOption {
  if (typeof value === 'number' && Number.isFinite(value)) {
    if (PRINT_OPTION_VALUES.includes(value as PrintOption)) {
      return value as PrintOption;
    }
  }

  throw new Error(`Unsupported print option '${String(value)}'.`);
}

const isRecord = (value: unknown): value is Record<string, unknown> =>
  typeof value === 'object' && value !== null;

const resolveRawOptions = (data: unknown): Record<string, unknown> | undefined => {
  if (!isRecord(data)) {
    return undefined;
  }
  const sram = data['sram'];
  if (!isRecord(sram)) {
    return undefined;
  }
  const options = sram['options'];
  if (!isRecord(options)) {
    return undefined;
  }
  return options;
};

function normalizeFrame(value: unknown): FrameType {
  if (typeof value === 'number' && Number.isFinite(value)) {
    if (FRAME_VALUES.includes(value as FrameType)) {
      return value as FrameType;
    }
    throw new Error(`Unsupported frame '${String(value)}'.`);
  }

  throw new Error(`Unsupported frame '${String(value)}'.`);
}

function synchroniseRuntimeState(gameState: GameState): void {
  try {
    gameState.wram.event_flags = gameState.sram.event_flags;
    gameState.wram.script_memory = gameState.sram.script_memory;

    const options = gameState.sram.options;
    if (options) {
      if ('print_option' in options) {
        options.print_option = normalizePrintOption(options.print_option);
      }
      if ('frame' in options) {
        options.frame = normalizeFrame(options.frame);
      }
    }
  } catch (err) {
    throw new SaveGameValidationError(
      `Failed to synchronize shared memory regions: ${err instanceof Error ? err.message : String(err)}`
    );
  }

  const party = gameState.sram.party?.pokemon || [];
  const partySize = party.filter(p => p !== null).length;
  gameState.wram.wPartyCount = partySize;

  if (partySize > 0) {
    gameState.wram.wCurPartyMon = Math.max(
      0,
      Math.min(gameState.wram.wCurPartyMon, partySize - 1)
    );
  } else {
    gameState.wram.wCurPartyMon = 0;
  }
}

function describeRoundTripDiff(original: unknown, roundTrip: unknown, path = '$'): string | null {
  if (typeof original !== typeof roundTrip) {
    return `${path}: type mismatch (original=${typeof original}, round_trip=${typeof roundTrip}) values original=${String(original)}, round_trip=${String(roundTrip)}`;
  }

  if (Array.isArray(original)) {
    if (!Array.isArray(roundTrip)) {
      return `${path}: type mismatch (original=array, round_trip=${typeof roundTrip})`;
    }
    if (original.length !== roundTrip.length) {
      return `${path}: list length mismatch (original=${original.length}, round_trip=${roundTrip.length})`;
    }
    for (let i = 0; i < original.length; i++) {
      const nested = describeRoundTripDiff(original[i], roundTrip[i], `${path}[${i}]`);
      if (nested) {
        return nested;
      }
    }
    return null;
  }

  if (original !== null && typeof original === 'object') {
    if (roundTrip === null || typeof roundTrip !== 'object' || Array.isArray(roundTrip)) {
      return `${path}: type mismatch (original=object, round_trip=${typeof roundTrip})`;
    }
    const originalKeys = Object.keys(original as Record<string, unknown>);
    const roundTripKeys = Object.keys(roundTrip as Record<string, unknown>);

    const missingKeys = originalKeys.filter(key => !roundTripKeys.includes(key));
    if (missingKeys.length > 0) {
      return `${path}: round-trip missing keys ${missingKeys.sort().join(', ')}`;
    }

    const extraKeys = roundTripKeys.filter(key => !originalKeys.includes(key));
    if (extraKeys.length > 0) {
      return `${path}: round-trip introduced keys ${extraKeys.sort().join(', ')}`;
    }

    for (const key of originalKeys) {
      const nested = describeRoundTripDiff(
        (original as Record<string, unknown>)[key],
        (roundTrip as Record<string, unknown>)[key],
        `${path}.${key}`
      );
      if (nested) {
        return nested;
      }
    }
    return null;
  }

  if (original !== roundTrip) {
    return `${path}: value mismatch original=${String(original)}, round_trip=${String(roundTrip)}`;
  }

  return null;
}

function normaliseCollection(value: unknown, pathParts: string[] = []): unknown {
  const sortValues = (values: unknown[]): unknown[] => {
    return [...values].sort((left, right) => {
      if (typeof left === 'number' && typeof right === 'number') {
        return left - right;
      }
      const leftText = String(left);
      const rightText = String(right);
      if (leftText < rightText) {
        return -1;
      }
      if (leftText > rightText) {
        return 1;
      }
      return 0;
    });
  };

  if (value instanceof Set) {
    const normalizedValues = Array.from(value).map(entry =>
      normaliseCollection(entry, [...pathParts, '<set>'])
    );
    return sortValues(normalizedValues);
  }
  if (Array.isArray(value)) {
    const normalized = value.map(entry => normaliseCollection(entry, [...pathParts, '[]']));
    const last = pathParts[pathParts.length - 1];
    if (last === 'pokedex_caught') {
      return sortValues(normalized);
    }
    return normalized;
  }
  if (value !== null && typeof value === 'object' && !(value instanceof Date)) {
    const normalizedObject: Record<string, unknown> = {};
    for (const [key, entry] of Object.entries(value as Record<string, unknown>)) {
      normalizedObject[key] = normaliseCollection(entry, [...pathParts, key]);
    }
    return normalizedObject;
  }
  return value;
}

function snapshotState(gameState: GameState): Record<string, unknown> {
  synchroniseRuntimeState(gameState);
  validateGameStateIntegrity(gameState);
  const snapshot = GameStateSchema.parse(gameState) as Record<string, unknown>;
  return normaliseCollection(snapshot) as Record<string, unknown>;
}

function validateRoundTrip(snapshot: Record<string, unknown>): void {
  const result = GameStateSchema.safeParse(snapshot);
  if (!result.success) {
    logger.error(`Round-trip validation failed before comparison: ${result.error}`);
    throw new SaveGameValidationError('Serialized save data failed validation.');
  }
  const roundTripDump = normaliseCollection(result.data) as Record<string, unknown>;
  const diff = describeRoundTripDiff(snapshot, roundTripDump);
  if (diff) {
    logger.error(`Round-trip mismatch detected: ${diff}`);
    throw new SaveGameValidationError(
      `Serialized save data does not round-trip cleanly. ${diff}`
    );
  }
}

function sanityCheckState(gameState: GameState): void {
  synchroniseRuntimeState(gameState);
  if (gameState.wram.event_flags !== gameState.sram.event_flags) {
    throw new SaveGameValidationError('WRAM and SRAM event flags are desynchronized.');
  }
  if (gameState.wram.script_memory !== gameState.sram.script_memory) {
    throw new SaveGameValidationError('WRAM and SRAM script memory are desynchronized.');
  }
  validateGameStateIntegrity(gameState);
}

export function parseGameStateSnapshot(data: unknown, source: string): GameState {
  const options = resolveRawOptions(data);

  if (options) {
    try {
      if ('print_option' in options) {
        options.print_option = normalizePrintOption(options.print_option);
      }
      if ('frame' in options) {
        options.frame = normalizeFrame(options.frame);
      }
    } catch (err) {
      logger.error(`Failed to validate options from ${source}: ${String(err)}`);
      throw new SaveGameValidationError(`Invalid save data in ${source}`);
    }
  }

  const result = GameStateSchema.safeParse(data);
  if (!result.success) {
    logger.error(`Failed to validate save data from ${source}: ${result.error}`);
    throw new SaveGameValidationError(`Invalid save data in ${source}: ${result.error.message}`);
  }

  const gameState: GameState = result.data;
  sanityCheckState(gameState);
  return gameState;
}

export function normalizeSaveSnapshot(data: unknown, source: string): Record<string, unknown> {
  const parsed = parseGameStateSnapshot(data, source);
  const snapshot = snapshotState(parsed);
  validateRoundTrip(snapshot);
  return snapshot;
}

async function loadGameFromPath(p: string): Promise<GameState> {
  let rawText: string;
  try {
    rawText = await fs.readFile(p, 'utf-8');
  } catch (err) {
    if (isErrnoException(err) && err.code === 'ENOENT') {
      logger.info(`No save file found at ${p}`);
      throw new SaveFileNotFoundError(`Save file not found: ${p}`);
    }
    logger.error(`Failed to read save file ${p}: ${String(err)}`);
    throw new SaveGameError(`Failed to read save file ${p}`);
  }

  let data: unknown;
  try {
    data = JSON.parse(rawText);
  } catch (err) {
    logger.error(`Failed to parse save file ${p}: ${String(err)}`);
    throw new SaveGameValidationError(`Invalid save data in ${p}`);
  }

  const gameState = parseGameStateSnapshot(data, p);
  logger.info(`Game loaded from ${p}`);
  return gameState;
}

async function loadGameFromSupabase(slot: string): Promise<SaveCandidate | null> {
  const record = await loadBrowserCloudSave(slot);
  if (!record) {
    return null;
  }
  const payload = parseGameStateSnapshot(record.payload, `supabase:${slot}`);
  return buildCandidate('remote', payload, record.saved_at ?? record.updated_at ?? null);
}

async function loadGameFromSupabaseForIdentity(
  slot: string,
  playerId: string
): Promise<GameState | null> {
  let payload: Record<string, unknown> | null;
  try {
    payload = await loadIdentityCloudSave(slot, playerId);
  } catch (error) {
    throw new SaveGameError(
      error instanceof Error ? error.message : String(error),
    );
  }
  if (!payload) {
    return null;
  }
  return parseGameStateSnapshot(payload, `supabase:${playerId}:${slot}`);
}

async function loadGameFromSupabaseCandidateForIdentity(
  slot: string,
  playerId: string
): Promise<SaveCandidate | null> {
  const gameState = await loadGameFromSupabaseForIdentity(slot, playerId);
  if (!gameState) {
    return null;
  }
  return buildCandidate('remote', gameState, null);
}

async function loadGameFromFilesystemCandidate(filepath: string): Promise<SaveCandidate> {
  const gameState = await loadGameFromPath(filepath);
  const metadata = await readSaveMetadataText(normaliseSavePath(filepath));
  return buildCandidate('filesystem', gameState, metadata?.saved_at ?? null);
}

async function loadGameFromGuestSessionCandidate(slot: string): Promise<SaveCandidate | null> {
  const slotCandidates = listGuestSessionSlotCandidates(slot);
  if (slotCandidates.length === 0) {
    return null;
  }
  const parsedCandidates: SaveCandidate[] = [];
  let firstError: SaveGameError | null = null;
  for (const slotCandidate of slotCandidates) {
    let savedAt: string | null = null;
    if (slotCandidate.metadata) {
      try {
        const metadata = JSON.parse(slotCandidate.metadata) as Partial<SaveMetadata> | null;
        savedAt = typeof metadata?.saved_at === 'string' ? metadata.saved_at : null;
      } catch (error) {
        logger.warn(
          `[save] Unable to parse guest save metadata for ${slot} from ${slotCandidate.sourceKey}: ${String(error)}`
        );
      }
    }
    try {
      const payload = JSON.parse(slotCandidate.payload) as unknown;
      const gameState = parseGameStateSnapshot(payload, `guest-session:${slotCandidate.sourceKey}`);
      parsedCandidates.push(buildCandidate('guest-session', gameState, savedAt));
    } catch (error) {
      if (!firstError && error instanceof SaveGameError) {
        firstError = error;
      }
    }
  }
  const bestCandidate = chooseBestSaveCandidate(parsedCandidates);
  if (bestCandidate) {
    return bestCandidate;
  }
  if (firstError) {
    throw firstError;
  }
  return null;
}

async function filesystemSaveExists(filepath: string): Promise<boolean> {
  const primaryPath = normaliseSavePath(filepath);
  try {
    await fs.access(primaryPath);
    return true;
  } catch (err) {
    if (!isErrnoException(err) || err.code !== 'ENOENT') {
      logger.warn(`[save] Unable to stat save file ${primaryPath}: ${String(err)}`);
    }
  }
  return false;
}

async function saveGameToSupabase(
  slot: string,
  snapshot: Record<string, unknown>,
  savedAt: string
): Promise<boolean> {
  try {
    return await saveBrowserCloudSave(slot, snapshot, savedAt);
  } catch (err) {
    logger.error(`[save] Supabase save failed for ${slot}: ${String(err)}`);
    return false;
  }
}

function saveGameToGuestSession(
  slot: string,
  snapshot: Record<string, unknown>,
  savedAt: string
): boolean {
  const payloadWritten = writeGuestSessionSlot(slot, JSON.stringify(snapshot));
  if (!payloadWritten) {
    return false;
  }
  if (!writeGuestSessionMetadata(slot, JSON.stringify({ saved_at: savedAt }))) {
    logger.warn(`[save] Unable to persist guest save metadata for ${slot}`);
  }
  return true;
}

async function discoverSaveCandidates(filepath: string): Promise<{
  identity: McpIdentityContextValue | null;
  target: ScopedSaveTarget;
  discovery: SaveDiscoveryResult;
}> {
  const identity = await loadMcpIdentityContext();
  const target = resolveScopedTarget(filepath, identity);
  const { slot, localPath } = target;
  const candidates: SaveCandidate[] = [];
  const loadErrors: SaveGameError[] = [];

  if (identity?.playerId) {
    try {
      const remote = await loadGameFromSupabaseCandidateForIdentity(slot, identity.playerId);
      if (remote) {
        candidates.push(remote);
      }
    } catch (err) {
      if (err instanceof SaveGameError) {
        loadErrors.push(err);
      } else {
        throw err;
      }
    }
    return {
      identity,
      target,
      discovery: { candidates, loadErrors },
    };
  }

  if (shouldUseSupabase()) {
    try {
      const remote = await loadGameFromSupabase(slot);
      if (remote) {
        candidates.push(remote);
      }
    } catch (err) {
      if (err instanceof SaveGameError) {
        loadErrors.push(err);
      } else {
        throw err;
      }
    }
  }

  if (isBrowserRuntime()) {
    try {
      const guestSession = await loadGameFromGuestSessionCandidate(slot);
      if (guestSession) {
        candidates.push(guestSession);
      }
    } catch (err) {
      if (err instanceof SaveGameError) {
        loadErrors.push(err);
      } else {
        throw err;
      }
    }
  } else {
    const filesystemPresent = await filesystemSaveExists(localPath);
    if (filesystemPresent) {
      try {
        const filesystemCandidate = await loadGameFromFilesystemCandidate(localPath);
        candidates.push(filesystemCandidate);
      } catch (err) {
        if (err instanceof SaveGameError) {
          loadErrors.push(err);
        } else {
          throw err;
        }
      }
    }
  }

  return {
    identity,
    target,
    discovery: { candidates, loadErrors },
  };
}

async function saveGameToSupabaseForIdentity(
  slot: string,
  snapshot: Record<string, unknown>,
  playerId: string
): Promise<void> {
  try {
    await saveIdentityCloudSave(slot, playerId, snapshot);
  } catch (error) {
    throw new SaveGameError(
      error instanceof Error ? error.message : String(error),
    );
  }
}

async function deleteSaveFromSupabase(slot: string): Promise<boolean> {
  try {
    return await deleteBrowserCloudSave(slot);
  } catch (err) {
    logger.error(`[save] Supabase delete failed for ${slot}: ${String(err)}`);
    return false;
  }
}

async function deleteSaveFromSupabaseForIdentity(
  slot: string,
  playerId: string
): Promise<boolean> {
  try {
    return await deleteIdentityCloudSave(slot, playerId);
  } catch (error) {
    throw new SaveGameError(
      error instanceof Error ? error.message : String(error),
    );
  }
}

export async function saveGame(gameState: GameState, filepath: string): Promise<boolean> {
  const identity = typeof window !== "undefined" ? null : await loadMcpIdentityContext();
  const target = resolveScopedTarget(filepath, identity);
  const primaryPath = target.localPath;
  const slot = target.slot;
  const savedAt = new Date().toISOString();
  const useSupabase = shouldUseSupabase();

  const snapshot = snapshotState(gameState);
  if (shouldValidateSaveRoundTrip()) {
    validateRoundTrip(snapshot);
  }

  const payload = JSON.stringify(snapshot, null, 4);
  if (identity?.playerId) {
    await saveGameToSupabaseForIdentity(slot, snapshot, identity.playerId);
    logger.info(`[save] Game saved to Supabase identity slot ${identity.playerId}:${slot}`);
    return true;
  }

  if (useSupabase) {
    const saved = await saveGameToSupabase(slot, snapshot, savedAt);
    if (saved) {
      logger.info(`[save] Game saved to Supabase slot ${slot}`);
      return true;
    }
    if (saveGameToGuestSession(slot, snapshot, savedAt)) {
      logger.info(`[save] Game saved to guest session slot ${slot}`);
      return true;
    }
    throw new SaveGameError(`Failed to save game to Supabase slot ${slot}`);
  }

  if (isBrowserRuntime()) {
    if (saveGameToGuestSession(slot, snapshot, savedAt)) {
      logger.info(`[save] Game saved to guest session slot ${slot}`);
      return true;
    }
    throw new SaveGameError(`Failed to save game to guest session slot ${slot}`);
  }

  try {
    await backupExistingSaveFile(primaryPath);
    await writeSaveText(primaryPath, payload);
    try {
      await writeSaveMetadataText(primaryPath, savedAt);
    } catch (err) {
      logger.warn(`[save] Unable to persist save metadata for ${primaryPath}: ${String(err)}`);
    }
  } catch (err) {
    logger.error(`Failed to save game to ${primaryPath}: ${String(err)}`);
    throw new SaveGameError(`Failed to save game to ${primaryPath}`);
  }

  logger.info(`Game saved to ${primaryPath}`);
  return true;
}

async function copySaveSlot(source: string, destination: string): Promise<boolean> {
  try {
    const snapshot = await loadGame(source);
    await saveGame(snapshot, destination);
    return true;
  } catch (err) {
    if (err instanceof SaveGameError) {
      return false;
    }
    throw err;
  }
}

export async function saveGameWithHistory(
  gameState: GameState,
  primarySlot: string,
  historySlots: string[]
): Promise<boolean> {
  if (typeof window !== "undefined") {
    await saveGame(gameState, primarySlot);
    return true;
  }

  if (historySlots.length > 0) {
    for (let idx = historySlots.length - 1; idx > 0; idx -= 1) {
      await copySaveSlot(historySlots[idx - 1], historySlots[idx]);
    }
    await copySaveSlot(primarySlot, historySlots[0]);
  }

  return saveGame(gameState, primarySlot);
}

export async function loadGame(filepath: string): Promise<GameState> {
  const { identity, target, discovery } = await discoverSaveCandidates(filepath);
  const { candidates, loadErrors } = discovery;
  const bestCandidate = chooseBestSaveCandidate(candidates);
  if (bestCandidate) {
    return bestCandidate.gameState;
  }

  if (loadErrors.length > 0) {
    throw loadErrors[0];
  }

  if (identity?.playerId) {
    throw new SaveFileNotFoundError(
      `Save slot ${target.slot} not found for identity ${identity.playerId}`
    );
  }

  throw new SaveFileNotFoundError(`Save file not found: ${target.localPath}`);
}

export async function hasSaveGame(filepath: string): Promise<boolean> {
  const { discovery } = await discoverSaveCandidates(filepath);
  return chooseBestSaveCandidate(discovery.candidates) !== null;
}


export async function deleteSaveGame(filepath: string): Promise<boolean> {
  const identity = await loadMcpIdentityContext();
  const target = resolveScopedTarget(filepath, identity);
  const p = target.localPath;
  const slot = target.slot;

  if (identity?.playerId) {
    return deleteSaveFromSupabaseForIdentity(slot, identity.playerId);
  }

  if (shouldUseSupabase()) {
    const remoteDeleted = await deleteSaveFromSupabase(slot);
    const guestDeleted = deleteGuestSessionSlot(slot);
    const deleted = remoteDeleted || guestDeleted;
    if (!deleted) {
      logger.info(`No save file to delete at ${p}`);
    }
    return deleted;
  }

  if (isBrowserRuntime()) {
    const deleted = deleteGuestSessionSlot(slot);
    if (!deleted) {
      logger.info(`No save file to delete at ${p}`);
    }
    return deleted;
  }

  let deleted = false;
  for (const target of [p, backupSavePath(p), metadataSavePath(p)]) {
    try {
      await fs.unlink(target);
      logger.info(`Deleted save file ${target}`);
      deleted = true;
    } catch (err) {
      if (isErrnoException(err) && err.code === 'ENOENT') {
        continue; // File didn't exist, which is fine
      }
      logger.error(`Failed to delete save file ${target}: ${String(err)}`);
      throw new SaveGameError(`Failed to delete save file ${target}`);
    }
  }

  if (!deleted) {
    logger.info(`No save file to delete at ${p}`);
  }
  return deleted;
}
