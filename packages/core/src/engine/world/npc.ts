import { GameState } from '../../core/state';
import { getMapMetadataByGroup } from './maps';
import { Trainer } from '../../core/models';
import { ObjectEvent as ObjectEventSchema, type ObjectEvent as OverworldObjectEvent } from '../../core/models/map';

type ScriptEntry = {
  command?: unknown;
  args?: unknown[] | null;
};

type ScriptData = unknown[] | null;

type DataLoaderLike = {
  npc_data?: Map<string, OverworldObjectEvent[]> | Record<string, OverworldObjectEvent[]>;
  get_script?: (scriptName: string, parentScript?: string) => ScriptData;
  getScript?: (scriptName: string, parentScript?: string) => ScriptData;
  getScriptByLabel?: (scriptName: string, parentScript?: string) => ScriptData;
  load_npc_data?: () => void;
  load_trainer_data?: () => void;
  get_trainer?: (trainerId: string) => Trainer | undefined | null;
  getTrainer?: (trainerId: string) => Trainer | undefined | null;
  trainer_data?: Map<string, Trainer>;
  trainer_data_by_name?: Map<string, Trainer>;
};

type GameStateWithLoader = GameState & {
  data_loader?: DataLoaderLike | null;
  dataLoader?: DataLoaderLike | null;
};

const resolveDataLoader = (gameState: GameState): DataLoaderLike | null => {
  const state = gameState as GameStateWithLoader;
  return state.data_loader ?? state.dataLoader ?? null;
};

const toNumber = (value: unknown): number | null => {
  const numeric = Number(value);
  return Number.isFinite(numeric) ? Math.trunc(numeric) : null;
};

const normalizeMapName = (mapName: string): string => {
  return String(mapName ?? "")
    .trim()
    .replace(/\s+/g, "")
    .replace(/_/g, "")
    .toUpperCase();
};

const normalizeScriptName = (value: unknown): string => {
  return String(value ?? "")
    .split(";", 1)[0]
    .trim();
};

const normalizeTrainerId = (value: unknown): string => {
  const normalized = String(value ?? "").replace(/^["']|["']$/g, "").trim().toUpperCase();
  return normalized;
};

const getCurrentMapMetadataName = (gameState: GameState): string | null => {
  const wram = gameState.wram;
  if (!wram) {
    return null;
  }

  const byCurrent = getMapMetadataByGroup(wram.current_map_group, wram.current_map_id)?.name ?? null;
  const byLegacy =
    byCurrent ??
    getMapMetadataByGroup(wram.wMapGroup, wram.wMapNumber)?.name ??
    null;
  const pending = String(wram.pending_last_talked_map ?? "").trim();
  if (pending && byCurrent && pending.toUpperCase() === byCurrent.toUpperCase()) {
    return byCurrent;
  }
  if (pending && !byCurrent && !byLegacy) {
    return pending;
  }
  if (byCurrent) {
    return byCurrent;
  }
  if (byLegacy) {
    return byLegacy;
  }
  return pending || null;
};

const getNpcListForMap = (
  dataLoader: DataLoaderLike,
  mapName: string
): OverworldObjectEvent[] => {
  if (!dataLoader || !dataLoader.npc_data) {
    return [];
  }

  const raw = dataLoader.npc_data;
  const listFromMap = (sourceName: string): OverworldObjectEvent[] | null => {
    if (sourceName === "") {
      return null;
    }
    if (raw instanceof Map) {
      return raw.get(sourceName) ?? null;
    }
    return (raw as Record<string, ObjectEvent[]>)[sourceName] ?? null;
  };

  const normalizedNames = [
    mapName,
    mapName.toUpperCase(),
    normalizeMapName(mapName),
  ].filter(Boolean);
  const uniqueNames = Array.from(new Set(normalizedNames));
  for (const name of uniqueNames) {
    const events = listFromMap(name);
    if (events?.length) {
      return events;
    }
  }
  return [];
};

const getCurrentMapNpcList = (gameState: GameState, dataLoader: DataLoaderLike): ObjectEvent[] => {
  const mapName = getCurrentMapMetadataName(gameState);
  if (!mapName) {
    return [];
  }
  return getNpcListForMap(dataLoader, mapName);
};

type ObjectEvent = OverworldObjectEvent;

const parseObjectEvent = (event: ObjectEvent, context: string): ObjectEvent => {
  const parsed = ObjectEventSchema.safeParse(event);
  if (!parsed.success) {
    throw new Error(`ASM-backed NPC object event is invalid for ${context}.`);
  }
  return parsed.data;
};

const findByIndex = (events: ObjectEvent[], index: number): ObjectEvent | null => {
  if (!events.length || index <= 0) {
    return null;
  }
  const event = events[index - 1];
  if (!event) {
    return null;
  }
  return parseObjectEvent(event, `object index ${index}`);
};

const findByCoordinates = (
  events: ObjectEvent[],
  x: number,
  y: number
): ObjectEvent | null => {
  for (const [index, event] of events.entries()) {
    const parsed = parseObjectEvent(event, `object index ${index + 1}`);
    if (parsed.x === x && parsed.y === y) {
      return parsed;
    }
  }
  return null;
};

const resolveScript = (dataLoader: DataLoaderLike, scriptName: string): ScriptData => {
  const resolver = dataLoader.get_script ?? dataLoader.getScript ?? dataLoader.getScriptByLabel;
  if (typeof resolver !== "function") {
    return null;
  }
  return resolver.call(dataLoader, scriptName);
};

const extractTrainerFromScript = (scriptData: ScriptData): string | null => {
  if (!Array.isArray(scriptData)) {
    return null;
  }
  for (const entry of scriptData) {
    const row = entry as ScriptEntry;
    const command = String(row.command ?? "").trim().toLowerCase();
    if (command !== "trainer" && command !== "loadtrainer") {
      continue;
    }
    const args = Array.isArray(row.args) ? row.args : [];
    if (args.length < 2) {
      continue;
    }
    const trainerId = normalizeTrainerId(args[1]);
    if (!trainerId || trainerId === "0" || trainerId === "-1") {
      continue;
    }
    return trainerId;
  }
  return null;
};

const loadTrainerData = (dataLoader: DataLoaderLike): void => {
  if (typeof dataLoader.load_trainer_data === "function") {
    dataLoader.load_trainer_data();
  }
};

const findTrainerById = (dataLoader: DataLoaderLike, trainerId: string): Trainer | null => {
  const normalized = normalizeTrainerId(trainerId);
  if (!normalized) {
    return null;
  }
  const candidates = [normalized, normalized.toUpperCase(), normalized.toLowerCase()];
  const getTrainerDirect = (candidate: string): Trainer | undefined => {
    const direct =
      dataLoader.get_trainer?.(candidate) ??
      dataLoader.getTrainer?.(candidate) ??
      dataLoader.trainer_data?.get(candidate) ??
      dataLoader.trainer_data_by_name?.get(candidate);
    return direct ?? undefined;
  };
  for (const candidate of candidates) {
    const direct = getTrainerDirect(candidate);
    if (direct) {
      return direct;
    }
  }
  return null;
};

const addTrainerCandidates = (
  candidates: Set<string>,
  value: unknown,
  mapName: string | null
): void => {
  const normalized = normalizeTrainerId(value);
  if (!normalized || normalized === "0" || normalized === "-1") {
    return;
  }
  candidates.add(normalized);
  const trimmed = normalized.replace(/SCRIPT$/i, "");
  if (trimmed !== normalized) {
    candidates.add(trimmed);
  }
  if (normalized.startsWith("TRAINER_")) {
    candidates.add(normalized.slice("TRAINER_".length));
  }
  if (!mapName) {
    return;
  }
  const mapUpper = String(mapName).toUpperCase();
  const mapNoSep = normalizeMapName(mapName);
  const mapCandidates = [mapUpper, mapNoSep];
  for (const mapPrefix of mapCandidates) {
    if (normalized.startsWith(`${mapPrefix}_`)) {
      candidates.add(normalized.slice(mapPrefix.length + 1));
    }
  }
};

export function checkNpcInteractions(gameState: GameState): ObjectEvent | null {
  const wram = gameState.wram;
  if (!wram) {
    return null;
  }
  const dataLoader = resolveDataLoader(gameState);
  if (!dataLoader) {
    return null;
  }

  if (typeof dataLoader.load_npc_data === "function") {
    dataLoader.load_npc_data();
  }

  const events = getCurrentMapNpcList(gameState, dataLoader);
  if (!events.length) {
    return null;
  }

  const lastTalked = toNumber(wram.last_talked) ?? 0;
  const pendingIndex = toNumber(wram.pending_last_talked_object) ?? 0;
  const pendingPosition = wram.pending_last_talked_position ?? null;
  const pendingObjectIndex = pendingIndex || lastTalked || 0;
  if (pendingObjectIndex > 0) {
    const fromIndex = findByIndex(events, pendingObjectIndex);
    if (fromIndex) {
      return fromIndex;
    }
  }

  if (pendingPosition?.length === 2) {
    const x = toNumber(pendingPosition[0]);
    const y = toNumber(pendingPosition[1]);
    if (x !== null && y !== null) {
      const fromCoordinates = findByCoordinates(events, x, y);
      if (fromCoordinates) {
        return fromCoordinates;
      }
    }
  }

  if (lastTalked > 0) {
    const fromLastTalked = findByIndex(events, lastTalked);
    if (fromLastTalked) {
      return fromLastTalked;
    }
  }

  return null;
}

export function getTrainer(gameState: GameState, objectEvent: ObjectEvent): Trainer | null {
  const dataLoader = resolveDataLoader(gameState);
  if (!dataLoader) {
    return null;
  }

  const mapName = getCurrentMapMetadataName(gameState);
  loadTrainerData(dataLoader);

  const candidates = new Set<string>();
  const scriptName = normalizeScriptName(objectEvent.script);
  if (scriptName) {
    addTrainerCandidates(candidates, scriptName, mapName);
    const scriptData = resolveScript(dataLoader, scriptName);
    if (scriptData) {
      const scriptTrainer = extractTrainerFromScript(scriptData);
      addTrainerCandidates(candidates, scriptTrainer, mapName);
    }
  }
  addTrainerCandidates(candidates, objectEvent.object_identifier, mapName);

  for (const candidate of candidates) {
    const trainer = findTrainerById(dataLoader, candidate);
    if (trainer) {
      return trainer;
    }
  }
  return null;
}
