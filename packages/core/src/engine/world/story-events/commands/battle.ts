import { GameState } from "@pokecrystal/core/core/state";
import { LearnedMove, Pokemon, PokemonData, PokemonSpecies, Trainer } from "@pokecrystal/core/core/models";
import { MAX_BOX_MONS, NAME_LENGTH } from "@pokecrystal/core/core/constants";
import { Item as ItemId } from "@pokecrystal/core/core/enums/item";
import { recordPokedexCaught } from "@pokecrystal/core/core/pokedex";
import { loadMergedEvolutionsSync } from "@pokecrystal/core/core/content-packs";
import { FORCED_SHINY_DVS } from "@pokecrystal/core/core/pokemon-dvs";
import { createPokemon } from "@pokecrystal/core/engine/systems/pokemon";
import { toPokemon } from "@pokecrystal/core/core/models/pokemon";
import { CatchTutorialRunner } from "@pokecrystal/core/engine/battle/tutorial";
import { determineTrainerEncounterMusic } from "@pokecrystal/core/engine/battle/battle/music";
import { Event, EventManager, StartBattleEvent, openText } from "@pokecrystal/core/engine/events/events";
import { METATILE_WIDTH } from "@pokecrystal/core/engine/world/tile/constants";
import { EGG_LEVEL } from "@pokecrystal/core/engine/systems/breeding";
import type { AudioEngine } from "@pokecrystal/core/engine/systems/audio";
import { gameEngine, type GameEngineEventQueue } from "@pokecrystal/core/ui/game-engine";
import { nextFrame } from "@pokecrystal/core/ui/async-loop";
import { NameEntryScreen } from "@pokecrystal/core/ui/screens/name-entry-screen";
import type { ScreenUI } from "@pokecrystal/core/ui/screens/screen-types";
import { LOGGER, showText, waitForInput } from "../common";
import { applyEventFlag } from "../event-flags";
import { resolveText } from "../text-helpers";
import type { DataLoader } from "@pokecrystal/core/core/data-loader";
import type { ScriptRunner } from "@pokecrystal/core/engine/world/story-events/runner";
import type { OverworldContext, ScriptFrame } from "./base";
import { Command, normalizeScriptName } from "./base";
import { ReloadMapAfterBattleCommand } from "./movement";
import { CloseTextCommand, OpenTextCommand, WaitButtonCommand } from "./text";
import { addPokemon, getFilledSlots } from "@pokecrystal/core/core/models/party";
import { HardwareRNG } from "@pokecrystal/core/engine/games/rng";
import { defaultMovesForLevel } from "@pokecrystal/core/engine/systems/learnsets";
import { loadAllMoves } from "@pokecrystal/core/core/models/move";
import {
  addPokemon as addPokemonToBox,
  getNextOpenSlot as getNextOpenBoxSlot,
  type Box,
  BoxSchema,
  formatDefaultBoxName,
} from "@pokecrystal/core/core/models/box";

const OPPOSITE_DIRECTIONS: Record<string, string> = {
  up: "down",
  down: "up",
  left: "right",
  right: "left",
};

const BATTLE_RESULT_MASK = (1 << 6) | (1 << 7);
const BATTLETYPE_CANLOSE = "BATTLETYPE_CANLOSE";
const BATTLETYPE_FORCESHINY = "BATTLETYPE_FORCESHINY";
const BATTLETYPE_FORCEITEM = "BATTLETYPE_FORCEITEM";
const RUNNING_TRAINER_BATTLE_SCRIPT_ACTIVE = -1;
const GIFT_PARTY_OT_ID = 1001;

const CHERRYGROVE_CITY = "CherrygroveCity";
const CHERRYGROVE_MEET_RIVAL_SCENE = "SCENE_CHERRYGROVECITY_MEET_RIVAL";
const CHERRYGROVE_NOOP_SCENE = "SCENE_CHERRYGROVECITY_NOOP";
const CHERRYGROVE_RIVAL_FLAG = "EVENT_RIVAL_CHERRYGROVE_CITY";
const RIVAL_NAME_PLACEHOLDER = "<RIVAL>";
const UNKNOWN_RIVAL_NAME = "???";
const DEFAULT_FALLBACK_MOVE_PP = 0;

type PlayerProxy = {
  turn?: (direction: string) => void;
};

type OverworldWithPlayer = OverworldContext & {
  player_direction?: string;
  player_object?: PlayerProxy | null;
  playerObject?: PlayerProxy | null;
};

type NamingScreenUi = ScreenUI & {
  eventQueue?: GameEngineEventQueue | null;
};

type OverworldWithUi = {
  ui?: NamingScreenUi | null;
};

type OverworldWithInputCapture = {
  input_capture_active?: boolean;
};

type WinLossTextState = {
  winText: string;
  lossText: string;
};

type OverworldWithDialogue = {
  dialogue?: {
    clear_script_waits?: () => void;
    forceCloseText?: () => void;
    _handle_close_text?: (options?: { force?: boolean }) => void;
  } | null;
};

type OverworldWithAudio = {
  audio_engine?: AudioEngine | null;
  audioEngine?: AudioEngine | null;
};

type NameEntryScreenEvent = Parameters<NameEntryScreen["handleInput"]>[0];

type DataLookupMap<T> = { get: (key: string) => T | null | undefined } | null | undefined;

type TrainerData = {
  party?: Pokemon[];
  [key: string]: unknown;
};

type PokemonLookup = {
  get_pokemon_species?: (id: string) => PokemonSpecies | null | undefined;
  getPokemonSpecies?: (id: string) => PokemonSpecies | null | undefined;
  getSpecies?: (id: string) => PokemonSpecies | null | undefined;
  pokemonData?: DataLookupMap<PokemonSpecies>;
  pokemon_data?: DataLookupMap<PokemonSpecies>;
};

const resolveOverworldAudioEngine = (
  runner: ScriptRunner | null | undefined,
  overworld: OverworldContext | null | undefined,
): AudioEngine | null => {
  return (
    (overworld as OverworldWithAudio | null | undefined)?.audio_engine ??
    (overworld as OverworldWithAudio | null | undefined)?.audioEngine ??
    runner?.audio_engine ??
    runner?.audioEngine ??
    null
  );
};

type ItemLookup = {
  get_item?: (name: string) => { name?: string } | null | undefined;
  getItem?: (name: string) => { name?: string } | null | undefined;
  itemData?: DataLookupMap<{ name?: string }>;
};

type ScriptLookup = {
  get_script?: (label: string) => unknown;
  getScript?: (label: string) => unknown;
  getScriptByLabel?: (label: string) => unknown;
  get_text?: (label: string) => string | null | undefined;
  getText?: (label: string) => string | null | undefined;
  getTextByLabel?: (label: string) => string | null | undefined;
};

type MapSceneRunner = ScriptRunner & {
  _set_map_scene?: (mapName: string, sceneName: string) => void;
};

type TrainerLookup = {
  get_trainer?: (id: string) => TrainerData | null | undefined;
  getTrainer?: (id: string) => TrainerData | null | undefined;
  get_trainer_base_reward?: (id: string) => number;
  getTrainerBaseReward?: (id: string) => number;
};

type DataLoaderLike = PokemonLookup & ItemLookup & ScriptLookup & TrainerLookup;

type BattleRunner = ScriptRunner & {
  _script_stack?: ScriptFrame[];
  set_event_flag?: (flag: string, value: boolean) => void;
  stop_all_scripts?: () => void;
  _terminate_current_script?: () => void;
};

const clearBattleScriptStackForWhiteout = (runner: BattleRunner): void => {
  if (Array.isArray(runner._script_stack)) {
    runner._script_stack.length = 0;
  }
  runner.just_battled = false;
  runner.loaded_trainer = null;
  runner.loaded_trainer_id = null;
  runner.pending_reload_map = null;
};

const battleResultCode = (value: unknown): number => {
  const numeric = Number(value);
  if (!Number.isFinite(numeric)) {
    return 0;
  }
  return numeric & ~BATTLE_RESULT_MASK;
};

const resolveBattleResult = (result: unknown): number => {
  const numeric = Number(result);
  if (!Number.isFinite(numeric)) {
    throw new Error(`Invalid ASM battle result '${String(result)}'.`);
  }
  return numeric;
};

const resolveTrainerDisplayName = (gameState: GameState, trainerName: string): string => {
  const normalized = String(trainerName ?? "").trim();
  if (!normalized.includes(RIVAL_NAME_PLACEHOLDER)) {
    return normalized;
  }
  const rivalName = String(gameState.sram.rival_name ?? "").trim();
  return rivalName || UNKNOWN_RIVAL_NAME;
};

const buildFreshTrainerMoves = (
  speciesId: string,
  level: number,
  moves: LearnedMove[] | null | undefined,
): LearnedMove[] => {
  const moveData = loadAllMoves();
  const providedMoves = (moves ?? []).filter(Boolean);

  if (!providedMoves.length) {
    return defaultMovesForLevel(speciesId, level).map((name) => ({
      name,
      current_pp: moveData[name]?.pp ?? DEFAULT_FALLBACK_MOVE_PP,
    }));
  }

  return providedMoves.map((move) => {
    const maxPp = moveData[move.name]?.pp ?? DEFAULT_FALLBACK_MOVE_PP;
    return {
      ...move,
      current_pp: maxPp > 0 ? maxPp : move.current_pp ?? DEFAULT_FALLBACK_MOVE_PP,
    };
  });
};

const normalizeTrainerBattlePokemon = (pokemon: PokemonData): Pokemon =>
  toPokemon({
    ...pokemon,
    moves: buildFreshTrainerMoves(
      String(pokemon.species?.id ?? ""),
      Number(pokemon.level ?? 0),
      pokemon.moves,
    ),
  });

const normalizeTrainerForBattle = (gameState: GameState, trainer: Trainer): Trainer => ({
  ...trainer,
  name: resolveTrainerDisplayName(gameState, trainer.name),
  party: (trainer.party ?? []).filter(Boolean).map((pokemon) => normalizeTrainerBattlePokemon(pokemon)),
});

const facePlayerTowardTrainer = (
  gameState: GameState,
  overworld: OverworldWithPlayer | null | undefined,
): void => {
  if (!overworld) {
    return;
  }
  const direction = String(gameState.wram.seen_trainer_direction ?? "").toLowerCase();
  const opposite = OPPOSITE_DIRECTIONS[direction];
  if (!opposite) {
    return;
  }
  overworld.player_direction = opposite;
  const playerProxy = overworld.player_object ?? overworld.playerObject ?? null;
  const turn = playerProxy?.turn;
  if (typeof turn === "function") {
    turn.call(playerProxy, opposite);
  }
};

const resolveDataLoader = (runner?: ScriptRunner, overworld?: OverworldContext | null): DataLoader => {
  const typedOverworld = overworld as OverworldContext & {
    data_loader?: DataLoader | null;
    dataLoader?: DataLoader | null;
  };
  const typedRunner = runner as ScriptRunner & {
    data_loader?: DataLoader | null;
    dataLoader?: DataLoader | null;
  };
  const loader =
    typedOverworld?.data_loader ??
    typedOverworld?.dataLoader ??
    typedRunner?.data_loader ??
    typedRunner?.dataLoader;
  if (!loader) {
    throw new Error("Battle command requires a data loader context.");
  }
  return loader;
};

export const resolveSpecies = (dataLoader: DataLoader, speciesName: string): PokemonSpecies => {
  const upper = String(speciesName).toUpperCase();
  const lookup: PokemonLookup = dataLoader;
  const resolver =
    lookup.get_pokemon_species ??
    lookup.getPokemonSpecies ??
    lookup.getSpecies;
  if (typeof resolver === "function") {
    const species = resolver.call(lookup, upper);
    if (!species) {
      throw new Error(`Unknown Pokemon species '${speciesName}'.`);
    }
    return species;
  }
  const speciesMap = lookup.pokemonData ?? lookup.pokemon_data;
  const resolved = speciesMap?.get(upper);
  if (resolved) {
    return resolved;
  }
  throw new Error(`Unknown Pokemon species '${speciesName}'.`);
};

export const resolveItemName = (dataLoader: DataLoader, itemName: string): string | null => {
  const normalized = String(itemName ?? "").trim();
  if (!normalized) {
    return null;
  }
  const upper = normalized.toUpperCase();
  if (upper === "NO_ITEM" || upper === "NONE") {
    return null;
  }
  const lookupSource: ItemLookup = dataLoader;
  const lookup = lookupSource.get_item ?? lookupSource.getItem;
  if (typeof lookup === "function") {
    const item = lookup.call(lookupSource, upper);
    if (item?.name) {
      return item.name;
    }
  }
  const itemMap = lookupSource.itemData;
  if (itemMap?.get) {
    const item = itemMap.get(upper);
    if (item?.name) {
      return item.name;
    }
  }
  if (upper in ItemId) {
    return upper;
  }
  throw new Error(`Unknown gift item '${itemName}'.`);
};

type ScriptFetcher = {
  get_script?: (label: string) => unknown;
  getScript?: (label: string) => unknown;
  getScriptByLabel?: (label: string) => unknown;
};

const extractCustomName = (dataLoader: DataLoader, label: string | null | undefined): string | null => {
  if (!label) {
    return null;
  }
  const fetcher: ScriptFetcher = dataLoader;
  const getScript = fetcher.get_script ?? fetcher.getScript ?? fetcher.getScriptByLabel;
  let scriptEntries: unknown[] = [];
  if (typeof getScript === "function") {
    try {
      const scriptCandidate = getScript.call(fetcher, label);
      if (Array.isArray(scriptCandidate)) {
        scriptEntries = scriptCandidate;
      } else if (scriptCandidate && typeof scriptCandidate === "object") {
        scriptEntries = [scriptCandidate];
      }
    } catch {
      scriptEntries = [];
    }
  }
  if (scriptEntries.length) {
    const chars: string[] = [];
    for (const entry of scriptEntries) {
      if (!entry || typeof entry !== "object") {
        continue;
      }
      const record = entry as Record<string, unknown>;
      if (record.command !== "db") {
        continue;
      }
      let rawArgs: unknown[] = [];
      const rawArgsCandidate = record.args;
      if (Array.isArray(rawArgsCandidate)) {
        rawArgs = rawArgsCandidate;
      } else if (rawArgsCandidate !== undefined) {
        rawArgs = [rawArgsCandidate];
      }
      for (const token of rawArgs) {
        const text = String(token ?? "").trim();
        if (!text) {
          continue;
        }
        if (text.startsWith('"') && text.endsWith('"')) {
          let literal = text.slice(1, -1);
          try {
            literal = JSON.parse(text);
          } catch {
            // Fall back to the raw literal without quotes.
          }
          const terminates = literal.endsWith("@");
          const cleaned = literal.replace(/@/g, "");
          if (cleaned) {
            chars.push(cleaned);
          }
          if (terminates) {
            const candidate = chars.join("").trim();
            return candidate || null;
          }
        } else {
          let numeric: number | null = null;
          if (text.startsWith("$")) {
            if (!/^\$[0-9A-F]+$/i.test(text)) {
              throw new Error(`Invalid ASM custom name token '${text}' in label '${label}'.`);
            }
            numeric = Number.parseInt(text.slice(1), 16);
          } else if (text.toLowerCase().startsWith("0x")) {
            if (!/^0x[0-9A-F]+$/i.test(text)) {
              throw new Error(`Invalid ASM custom name token '${text}' in label '${label}'.`);
            }
            numeric = Number.parseInt(text, 16);
          } else {
            if (!/^\d+$/.test(text)) {
              throw new Error(`Invalid ASM custom name token '${text}' in label '${label}'.`);
            }
            numeric = Number.parseInt(text, 10);
          }
          if (Number.isNaN(numeric)) {
            continue;
          }
          if (numeric === 0) {
            const candidate = chars.join("").trim();
            return candidate || null;
          }
          chars.push(String.fromCharCode(numeric));
        }
      }
    }
    const candidate = chars.join("").replace(/@/g, "").trim();
    if (candidate) {
      return candidate;
    }
  }

  const rawText =
    dataLoader?.get_text?.(label) ??
    dataLoader?.getText?.(label) ??
    dataLoader?.getTextByLabel?.(label) ??
    "";
  const cleaned = String(rawText ?? "").replace(/@/g, "").replace(/\n/g, " ").trim();
  if (cleaned && cleaned !== String(label).trim()) {
    return cleaned;
  }
  throw new Error(`Missing ASM custom name for label '${label}'.`);
};

const formatSpeciesName = (speciesId: string): string => {
  return speciesId.replace(/__/g, " ").replace(/_/g, " ").replace(/\s+/g, " ").trim();
};

const resolveGiftOtName = (gameState: GameState): string => {
  const playerName = String(gameState.sram.player_name ?? "").trim();
  if (!playerName) {
    throw new Error("Gift Pokemon requires a non-empty ASM player name.");
  }
  return playerName;
};

const resolveBattleTextLabel = (
  runner: ScriptRunner | null | undefined,
  overworld: OverworldContext,
  label: string,
): string => {
  const resolved = resolveText(runner ?? null, overworld, label);
  const cleaned = String(resolved ?? "").replace(/@/g, "").replace(/\n/g, " ").trim();
  if (cleaned === String(label).trim()) {
    throw new Error(`Missing ASM battle text for label '${label}'.`);
  }
  return resolved;
};

class ResolvedBattleTextCommand extends Command {
  constructor(
    private readonly text: string,
    private readonly autoCloseAfterWait: boolean = false,
  ) {
    super();
  }

  public execute(_gameState: GameState, eventManager: EventManager, _overworld: OverworldContext): void {
    showText(eventManager, this.text, { auto_close_after_wait: this.autoCloseAfterWait });
    if (this.autoCloseAfterWait) {
      waitForInput(eventManager, false);
    }
    if (this.runner) {
      this.runner.stopExecution = false;
    }
  }
}

class TrainerBattleCallbackCommand extends Command {
  constructor(
    private readonly callback: string,
    private readonly parentScript: string | null,
  ) {
    super();
  }

  public execute(_gameState: GameState, _eventManager: EventManager, _overworld: OverworldContext): void {
    const runner = this.runner as BattleRunner | undefined;
    if (!runner || !this.callback) {
      return;
    }
    const target = normalizeScriptName(this.callback);
    const stack = runner._script_stack as ScriptFrame[] | undefined;
    if (this.parentScript && stack) {
      const parentFrame = [...stack].reverse().find((frame) => frame.name === this.parentScript);
      if (parentFrame) {
        parentFrame.index = parentFrame.commands.length;
        parentFrame.allowFallthrough = false;
      }
    }
    if (target.startsWith(".")) {
      if (this.parentScript) {
        runner.jump?.(target, this.parentScript);
      } else {
        runner.jump?.(target);
      }
      return;
    }
    runner.defer?.(target);
  }
}

class ClearTrainerBattleScriptCommand extends Command {
  public execute(gameState: GameState, _eventManager: EventManager, _overworld: OverworldContext): void {
    const runner = this.runner as BattleRunner | undefined;
    gameState.wram.wRunningTrainerBattleScript = 0;
    if (runner) {
      runner.just_battled = false;
    }
  }
}

const setStringBuffers = (runner: ScriptRunner | undefined, buffer1: string, buffer3: string): void => {
  if (!runner) {
    return;
  }
  if (!runner.string_buffers) {
    runner.string_buffers = {};
  }
  runner.string_buffers.STRING_BUFFER_1 = buffer1;
  runner.string_buffers.STRING_BUFFER_3 = buffer3;
};

const updateRunnerStatus = (
  runner: ScriptRunner | undefined,
  status: number,
  speciesId: string,
  nickname: string,
  location: string,
  boxIndex: number | null,
  slotIndex: number | null,
): void => {
  if (!runner) {
    return;
  }
  if (!runner.variables) {
    runner.variables = {};
  }
  runner.last_condition_result = status !== 2;
  runner.variables._value = status;
  runner.variables._last_result = status;
  runner.last_value = {
    givepoke: {
      status,
      species: speciesId,
      nickname,
      location,
      box_index: boxIndex,
      slot_index: slotIndex,
    },
  };
};

const emitPcOverflowText = (eventManager: EventManager, nickname: string): void => {
  const message = `${nickname} was\nsent to BILL's PC.`;
  showText(eventManager, message);
  waitForInput(eventManager);
};

const pushFinalizerFrame = (runner: BattleRunner | undefined, command: Command): boolean => {
  const stack = runner?._script_stack;
  if (!stack || !stack.length) {
    return false;
  }
  const parentName = stack[stack.length - 1]?.name ?? "givepoke";
  stack.push({
    name: `${parentName}#givepoke_finish`,
    commands: [command],
    index: 0,
  });
  return true;
};

const ensureBox = (gameState: GameState, index: number): Box => {
  const boxes = gameState.sram.pc_boxes;
  while (boxes.length <= index) {
    boxes.push(
      BoxSchema.parse({ name: formatDefaultBoxName(boxes.length) })
    );
  }
  const box = boxes[index];
  if (!box.name) {
    box.name = formatDefaultBoxName(index);
  }
  box.count = Math.max(0, Math.min(box.count, MAX_BOX_MONS));
  return box;
};

const addPokemonToPc = (
  gameState: GameState,
  pokemon: Pokemon,
): { stored: boolean; boxIndex: number | null; slotIndex: number | null } => {
  const boxes = gameState.sram.pc_boxes;
  if (!boxes.length) {
    ensureBox(gameState, 13);
  }
  for (let boxIndex = 0; boxIndex < boxes.length; boxIndex += 1) {
    const box = ensureBox(gameState, boxIndex);
    const targetSlot = getNextOpenBoxSlot(box);
    if (targetSlot === null) {
      continue;
    }
    const stored = addPokemonToBox(box, toPokemon(pokemon));
    if (stored) {
      return { stored: true, boxIndex, slotIndex: targetSlot };
    }
  }
  return { stored: false, boxIndex: null, slotIndex: null };
};

const createGiftBoxOtId = (gameState: GameState): number => {
  const rng = new HardwareRNG(gameState);
  return ((rng.nextByte() << 8) | rng.nextByte()) & 0xffff;
};

type EvolutionEntry = {
  species?: string;
  evolutions?: Array<{ species?: string }>;
};

const getImmediatePreEvolutionSpeciesId = (speciesId: string): string | null => {
  const target = String(speciesId ?? "").trim().toUpperCase();
  if (!target) {
    return null;
  }
  for (const entry of loadMergedEvolutionsSync() as EvolutionEntry[]) {
    const parent = String(entry?.species ?? "").trim().toUpperCase();
    if (!parent || !Array.isArray(entry?.evolutions)) {
      continue;
    }
    for (const evolution of entry.evolutions) {
      const child = String(evolution?.species ?? "").trim().toUpperCase();
      if (child === target) {
        return parent;
      }
    }
  }
  return null;
};

const resolveEggSpecies = (dataLoader: DataLoader, speciesName: string): PokemonSpecies => {
  let species = resolveSpecies(dataLoader, speciesName);
  for (let index = 0; index < 2; index += 1) {
    const predecessor = getImmediatePreEvolutionSpeciesId(species.id);
    if (!predecessor) {
      break;
    }
    species = resolveSpecies(dataLoader, predecessor);
  }
  return species;
};

const persistLastTalkedPosition = (
  gameState: GameState,
  overworld: OverworldContext | null | undefined,
): void => {
  if (!overworld) {
    return;
  }
  const lastTalked = Number(gameState.wram.last_talked ?? 0);
  if (!lastTalked) {
    return;
  }
  const getObject = overworld.get_object_by_id ?? overworld.getObjectById;
  if (typeof getObject !== "function") {
    return;
  }
  const target = getObject.call(overworld, lastTalked);
  if (!target) {
    return;
  }
  const tileStride = Math.max(1, Math.floor(METATILE_WIDTH / 2));
  const collisionStride = Number(target.collision_stride ?? overworld.TILES_PER_COLLISION ?? 2);
  const footprint = Math.max(0, collisionStride - 1);

  const mapCoord = (rawValue: unknown): number | null => {
    if (rawValue === null || rawValue === undefined) {
      return null;
    }
    const origin = Number(rawValue);
    if (!Number.isFinite(origin)) {
      return null;
    }
    return Math.floor((origin - footprint) / tileStride);
  };

  let trainerX = mapCoord(target.x);
  let trainerY = mapCoord(target.y);
  if (trainerX === null || trainerY === null) {
    const event = target.event ?? null;
    trainerX = mapCoord(event?.x);
    trainerY = mapCoord(event?.y);
  }
  if (trainerX === null || trainerY === null) {
    return;
  }
  const currentMap = overworld.current_map_name ?? null;
  if (!currentMap) {
    return;
  }
  gameState.wram.pending_last_talked_position = [trainerX, trainerY];
  gameState.wram.pending_last_talked_map = currentMap;
  gameState.wram.pending_last_talked_object = lastTalked;
};

export const handlePostBattleSceneFixes = (
  runner: ScriptRunner | undefined | null,
  gameState: GameState,
  overworld: OverworldContext | null | undefined,
  result: unknown,
): void => {
  if (!runner || !overworld) {
    return;
  }
  const mapName = overworld.current_map_name ?? null;
  if (mapName !== CHERRYGROVE_CITY) {
    return;
  }
  const numericResult = Number(result);
  if (!Number.isFinite(numericResult) || numericResult !== 0) {
    return;
  }
  const scene = gameState.wram.map_scenes?.[mapName];
  if (scene !== CHERRYGROVE_MEET_RIVAL_SCENE) {
    return;
  }
  LOGGER.debug?.(
    "Post-battle: advancing %s scene %s -> %s",
    mapName,
    scene,
    CHERRYGROVE_NOOP_SCENE,
  );
  const mapSceneRunner = runner as MapSceneRunner;
  if (typeof mapSceneRunner._set_map_scene === "function") {
    mapSceneRunner._set_map_scene(mapName, CHERRYGROVE_NOOP_SCENE);
  } else if (gameState.wram.map_scenes) {
    gameState.wram.map_scenes[mapName] = CHERRYGROVE_NOOP_SCENE;
  }
  applyEventFlag(gameState, CHERRYGROVE_RIVAL_FLAG, { value: true, overworld });
};

export class GivePokeCommand extends Command {
  constructor(
    private speciesName: string,
    private level: number,
    private item: string | null,
    private nicknameLabel?: string | null,
    private otLabel?: string | null,
  ) {
    super();
  }

  public execute(gameState: GameState, eventManager: EventManager, overworld: OverworldContext): void {
    const runner: BattleRunner | undefined = this.runner;
    const dataLoader = resolveDataLoader(runner, overworld);
    const species = resolveSpecies(dataLoader, this.speciesName);
    const pokemon = toPokemon(createPokemon(gameState, species, this.level));

    const playerName = resolveGiftOtName(gameState);
    pokemon.original_trainer_name = playerName;
    pokemon.original_trainer_id = Number(gameState.sram.player_id ?? 0);

    const customNickname = extractCustomName(dataLoader, this.nicknameLabel);
    if (customNickname) {
      pokemon.nickname = customNickname;
    }
    const customOt = extractCustomName(dataLoader, this.otLabel);
    // ASM GivePoke skips GiveANickname_YesNo entirely for the custom gift path.
    const usesCustomGiftMetadata = Boolean(this.nicknameLabel || this.otLabel);
    const usesGiftOtMetadata = Boolean(customOt);
    if (customOt) {
      pokemon.original_trainer_name = customOt;
    }
    const itemName = resolveItemName(dataLoader, this.item ?? "");
    if (itemName) {
      pokemon.item = itemName;
    }

    pokemon.hp = pokemon.max_hp ?? pokemon.hp ?? 0;
    pokemon.status = undefined;
    pokemon.sleep_turns = 0;
    pokemon.flinching = false;
    pokemon.confusion_turns = 0;
    pokemon.rampage_turns = 0;

    const speciesIntId = Number(species.int_id ?? 0);
    if (Number.isFinite(speciesIntId) && speciesIntId > 0) {
      recordPokedexCaught(gameState, speciesIntId);
    }

    let status = 2;
    let location = "failed";
    let boxIndex: number | null = null;
    let slotIndex: number | null = null;

    if (addPokemon(gameState.sram.party, pokemon)) {
      status = 0;
      location = "party";
      if (usesGiftOtMetadata) {
        pokemon.original_trainer_id = GIFT_PARTY_OT_ID;
      }
      gameState.wram.wPartyCount = getFilledSlots(gameState.sram.party);
    } else {
      if (usesGiftOtMetadata) {
        pokemon.original_trainer_id = createGiftBoxOtId(gameState);
      }
      const stored = addPokemonToPc(gameState, pokemon);
      if (stored.stored) {
        status = 1;
        location = "pc";
        boxIndex = stored.boxIndex;
        slotIndex = stored.slotIndex;
      } else {
        updateRunnerStatus(
          runner,
          status,
          species.id,
          pokemon.nickname,
          location,
          boxIndex,
          slotIndex,
        );
        return;
      }
    }

    const speciesDisplay = formatSpeciesName(String(species.id ?? this.speciesName));
    if (runner) {
      setStringBuffers(runner, speciesDisplay, pokemon.nickname ?? speciesDisplay);
    }

    const finalizer = new FinalizeGivePokeCommand({
      status,
      speciesId: String(species.id ?? this.speciesName),
      pokemon,
      speciesDisplay,
      location,
      boxIndex,
      slotIndex,
    });
    finalizer.runner = runner;

    const promptCommand = new GiveNicknamePromptCommand();
    promptCommand.runner = runner;

    const continuationInserted = pushFinalizerFrame(runner, finalizer);

    if (usesCustomGiftMetadata) {
      if (runner?.variables) {
        delete runner.variables._givepoke_nickname_choice;
        delete runner.variables._givepoke_nickname_value;
      }
      if (!continuationInserted) {
        finalizer.execute(gameState, eventManager, overworld);
      }
      return;
    }

    promptCommand.execute(gameState, eventManager, overworld);

    if (!continuationInserted) {
      finalizer.execute(gameState, eventManager, overworld);
    }
  }
}

const normalizeGivepokeChoice = (value: unknown): boolean | null => {
  if (value === null || value === undefined) {
    return null;
  }
  if (typeof value === "boolean") {
    return value;
  }
  const parsed = Number(value);
  if (!Number.isFinite(parsed)) {
    return null;
  }
  return parsed === 0;
};

const consumeGivepokeChoice = (runner: ScriptRunner | undefined): boolean | null => {
  if (!runner?.variables) {
    return null;
  }
  if (!Object.prototype.hasOwnProperty.call(runner.variables, "_givepoke_nickname_choice")) {
    return null;
  }
  const stored = runner.variables._givepoke_nickname_choice;
  if (Array.isArray(stored)) {
    const choice = stored.shift();
    if (!stored.length) {
      delete runner.variables._givepoke_nickname_choice;
    }
    return normalizeGivepokeChoice(choice);
  }
  delete runner.variables._givepoke_nickname_choice;
  return normalizeGivepokeChoice(stored);
};

const recordGivepokeChoice = (runner: ScriptRunner, accepted: boolean): void => {
  runner.last_yes_no_result = accepted;
  runner.last_condition_result = accepted;
  if (!runner.variables) {
    runner.variables = {};
  }
  runner.variables._givepoke_nickname_choice = [accepted ? 0 : 1];
};

export class GiveNicknamePromptCommand extends Command {
  constructor(private promptLabel: string = "CaughtAskNicknameText") {
    super();
  }

  public execute(_gameState: GameState, eventManager: EventManager, overworld: OverworldContext): void {
    const runner = this.runner;
    if (!runner) {
      return;
    }
    const choiceValues = runner.variables?._givepoke_nickname_choice;
    if (Array.isArray(choiceValues) && choiceValues.length) {
      const accepted = choiceValues[0] === 0;
      runner.last_yes_no_result = accepted;
      runner.last_condition_result = accepted;
      return;
    }

    const consumeChoice = runner._consume_script_choice ?? runner._consumeScriptChoice;
    if (typeof consumeChoice === "function") {
      const override = consumeChoice("_givepoke_nickname_choice", null);
      const resolved = normalizeGivepokeChoice(override);
      if (resolved !== null) {
        recordGivepokeChoice(runner, resolved);
        return;
      }
    }

    const dialogue = overworld?.dialogue ?? null;
    if (!dialogue) {
      recordGivepokeChoice(runner, false);
      return;
    }

    if (!dialogue.active) {
      openText(eventManager);
    }
    const text = resolveText(runner ?? null, overworld, this.promptLabel);
    if (text) {
      showText(eventManager, text);
    }
    runner.pause?.();
    waitForInput(eventManager);
    eventManager.dispatch(
      new Event("prompt_yes_no", {
        callback: (value: boolean) => recordGivepokeChoice(runner, Boolean(value)),
      }),
    );
  }
}

class FinalizeGivePokeCommand extends Command {
  private status: number;
  private speciesId: string;
  private pokemon: Pokemon;
  private speciesDisplay: string;
  private location: string;
  private boxIndex: number | null;
  private slotIndex: number | null;

  constructor(options: {
    status: number;
    speciesId: string;
    pokemon: Pokemon;
    speciesDisplay: string;
    location: string;
    boxIndex: number | null;
    slotIndex: number | null;
  }) {
    super();
    this.status = options.status;
    this.speciesId = options.speciesId;
    this.pokemon = options.pokemon;
    this.speciesDisplay = options.speciesDisplay;
    this.location = options.location;
    this.boxIndex = options.boxIndex;
    this.slotIndex = options.slotIndex;
  }

  public execute(gameState: GameState, eventManager: EventManager, overworld: OverworldContext): void {
    const runner = this.runner;
    let accepted = false;
    if (runner) {
      const consumeChoice = runner._consume_script_choice ?? runner._consumeScriptChoice;
      if (typeof consumeChoice === "function") {
        const override = consumeChoice("_givepoke_nickname_choice", null);
        const resolved = normalizeGivepokeChoice(override);
        if (resolved !== null) {
          accepted = resolved;
        } else {
          const stored = consumeGivepokeChoice(runner);
          if (stored !== null) {
            accepted = stored;
          } else {
            accepted = Boolean(runner.last_yes_no_result);
          }
        }
      } else {
        const stored = consumeGivepokeChoice(runner);
        if (stored !== null) {
          accepted = stored;
        } else {
          accepted = Boolean(runner.last_yes_no_result);
        }
      }
    }

    const applyNicknameOverride = (): string => {
      const override = consumeNicknameOverride(runner);
      if (override) {
        this.pokemon.nickname = override;
        return override;
      }
      return String(this.pokemon.nickname ?? "").trim() || this.speciesDisplay;
    };

    const finalize = (finalNickname: string): void => {
      if (runner) {
        setStringBuffers(runner, finalNickname, finalNickname);
      }
      updateRunnerStatus(
        runner,
        this.status,
        this.speciesId,
        finalNickname,
        this.location,
        this.boxIndex,
        this.slotIndex,
      );
      if (this.status === 1) {
        emitPcOverflowText(eventManager, finalNickname);
      }
    };

    if (accepted) {
      const startingName = String(this.pokemon.nickname ?? "").trim() || this.speciesDisplay;
      const queued = this.queueNamingScreen(gameState, runner ?? null, overworld, startingName, () => {
        const finalNickname = applyNicknameOverride();
        finalize(finalNickname);
      });
      if (queued) {
        return;
      }
      this.runNamingScreenBlocking(gameState, runner ?? null, overworld, startingName);
      const finalNickname = applyNicknameOverride();
      finalize(finalNickname);
      return;
    }

    const finalNickname = String(this.pokemon.nickname ?? "").trim() || this.speciesDisplay;
    finalize(finalNickname);
  }

  private queueNamingScreen(
    gameState: GameState,
    runner: ScriptRunner | null,
    overworld: OverworldContext | null,
    currentNickname: string,
    onComplete: () => void,
  ): boolean {
    if (!runner || !overworld) {
      return false;
    }
    const queueTask = runner._queue_overworld_task ?? runner._queueOverworldTask;
    if (typeof queueTask !== "function") {
      return false;
    }

    const ui: NamingScreenUi | null =
      (overworld as OverworldWithUi).ui ??
      (runner.overworld as OverworldWithUi).ui ??
      null;
    if (!ui || !ui.screen || typeof ui.clearScreen !== "function") {
      return false;
    }

    const promptText = "NAME YOUR POKEMON?";
    const audioEngine =
      (overworld as OverworldWithAudio).audio_engine ??
      (overworld as OverworldWithAudio).audioEngine ??
      runner.audio_engine ??
      runner.audioEngine ??
      null;
    const defaultName = currentNickname || this.speciesDisplay;
    const activeOverworld = (overworld as OverworldWithInputCapture) ?? null;
    const previousCapture = activeOverworld?.input_capture_active ?? false;
    const dialogue = (overworld as OverworldWithDialogue)?.dialogue ?? null;
    if (activeOverworld) {
      activeOverworld.input_capture_active = true;
    }
    if (dialogue) {
      dialogue.clear_script_waits?.();
      if (typeof dialogue.forceCloseText === "function") {
        dialogue.forceCloseText();
      } else if (typeof dialogue._handle_close_text === "function") {
        dialogue._handle_close_text({ force: true });
      }
    }

    const screen = new NameEntryScreen(ui, promptText, audioEngine);
    screen.reset({ prompt: promptText, maxNameLength: NAME_LENGTH - 1 });
    screen.fillName(defaultName);

    const runNamingScreen = async (): Promise<void> => {
      try {
        while (!screen.finished) {
          for (const event of gameEngine.event.get(ui.eventQueue ?? undefined)) {
            if (event.type === gameEngine.QUIT) {
              gameEngine.quit();
              throw new Error("Quit requested during naming screen.");
            }
            screen.handleInput(event as NameEntryScreenEvent);
          }
          screen.update();
          screen.draw();
          ui.update?.();
          await nextFrame();
        }
        const nickname = screen.name.trim() || defaultName;
        if (!runner.variables) {
          runner.variables = {};
        }
        runner.variables._givepoke_nickname_value = nickname;
      } finally {
        if (activeOverworld) {
          activeOverworld.input_capture_active = previousCapture;
        }
      }
    };

    queueTask.call(runner, (callback: () => void) => {
      void runNamingScreen()
        .catch((error) => {
          console.error("Name entry screen failed:", error);
        })
        .finally(() => {
          onComplete();
          callback();
        });
      return true;
    });

    void gameState;
    return true;
  }

  private runNamingScreenBlocking(
    gameState: GameState,
    runner: ScriptRunner | null,
    overworld: OverworldContext | null,
    currentNickname: string,
  ): void {
    if (!runner || !overworld) {
      return;
    }
    const ui: NamingScreenUi | null =
      (overworld as OverworldWithUi).ui ??
      (runner.overworld as OverworldWithUi).ui ??
      null;
    if (!ui || !ui.screen || typeof ui.clearScreen !== "function") {
      return;
    }
    const defaultName = currentNickname || this.speciesDisplay;
    const activeOverworld = (overworld as OverworldWithInputCapture) ?? null;
    const previousCapture = activeOverworld?.input_capture_active ?? false;
    const dialogue = (overworld as OverworldWithDialogue)?.dialogue ?? null;
    if (activeOverworld) {
      activeOverworld.input_capture_active = true;
    }
    if (dialogue) {
      dialogue.clear_script_waits?.();
      if (typeof dialogue.forceCloseText === "function") {
        dialogue.forceCloseText();
      } else if (typeof dialogue._handle_close_text === "function") {
        dialogue._handle_close_text({ force: true });
      }
    }
    if (activeOverworld) {
      activeOverworld.input_capture_active = previousCapture;
    }
    if (!runner.variables) {
      runner.variables = {};
    }
    runner.variables._givepoke_nickname_value = defaultName;
    void gameState;
  }
}

export class GiveEggCommand extends Command {
  constructor(private speciesName: string, private levelToken: string) {
    super();
  }

  public execute(gameState: GameState, _eventManager: EventManager, overworld: OverworldContext): void {
    const runner = this.runner;
    const dataLoader = resolveDataLoader(runner, overworld);
    const species = resolveEggSpecies(dataLoader, this.speciesName);
    const level = resolveEggLevel(this.levelToken);
    const pokemon = toPokemon(createPokemon(gameState, species, level));

    const playerName = resolveGiftOtName(gameState);
    pokemon.original_trainer_name = playerName;
    pokemon.original_trainer_id = Number(gameState.sram.player_id ?? 0);
    pokemon.nickname = "EGG";
    pokemon.happiness = Number(species.step_cycles_to_hatch ?? 0);
    // ASM GiveEgg zeros the egg's current HP after inserting it into the party.
    pokemon.hp = 0;

    if (!addPokemon(gameState.sram.party, pokemon)) {
      if (runner) {
        runner.last_condition_result = false;
        runner.last_value = { egg: { status: "party_full" } };
      }
      return;
    }

    gameState.wram.wPartyCount = getFilledSlots(gameState.sram.party);
    if (runner) {
      runner.last_condition_result = true;
      runner.last_value = { egg: { species: species.id } };
    }
  }
}

const resolveEggLevel = (token: string): number => {
  const normalized = String(token ?? "").trim();
  if (!normalized) {
    throw new Error("giveegg command requires a level token");
  }
  const upper = normalized.toUpperCase();
  if (upper === "EGG_LEVEL") {
    return EGG_LEVEL;
  }
  if (upper.startsWith("$")) {
    if (!/^\$[0-9A-F]+$/i.test(upper)) {
      throw new Error(`Invalid giveegg level '${token}'.`);
    }
    return Number.parseInt(upper.slice(1), 16);
  }
  if (upper.startsWith("0X")) {
    if (!/^0X[0-9A-F]+$/i.test(upper)) {
      throw new Error(`Invalid giveegg level '${token}'.`);
    }
    return Number.parseInt(upper, 16);
  }
  if (!/^\d+$/.test(normalized)) {
    throw new Error(`Invalid giveegg level '${token}'.`);
  }
  return Number.parseInt(normalized, 10);
};

const consumeNicknameOverride = (runner: ScriptRunner | undefined): string | null => {
  if (!runner?.variables) {
    return null;
  }
  const rawValue = runner.variables._givepoke_nickname_value;
  delete runner.variables._givepoke_nickname_value;
  if (typeof rawValue !== "string") {
    return null;
  }
  const cleaned = rawValue.trim();
  return cleaned || null;
};

export class BattleCommand extends Command {
  constructor(private trainerName: string) {
    super();
  }

  public execute(_gameState: GameState, eventManager: EventManager, _overworld: OverworldContext): void {
    eventManager.dispatch(new Event("start_trainer_battle", { trainer_name: this.trainerName }));
  }
}

export class WinLossTextCommand extends Command {
  constructor(private readonly winText: string, private readonly lossText: string = "0") {
    super();
  }

  public execute(_gameState: GameState, _eventManager: EventManager, _overworld: OverworldContext): void {
    const runner = this.runner;
    if (!runner) {
      throw new Error("WinLossTextCommand requires an active script runner.");
    }
    runner.variables["_win_loss_text"] = {
      winText: this.winText && this.winText !== "0" ? this.winText : "",
      lossText: this.lossText && this.lossText !== "0" ? this.lossText : "",
    } satisfies WinLossTextState;
  }
}

export class TrainerCommand extends Command {
  public parent_script: string | null = null;
  public event_flag: string;
  public seen_text: string;
  public win_text: string;
  public loss_text: string;
  public callback: string;

  constructor(
    public trainer_class: string,
    public trainer_id: string,
    eventFlag: string,
    seenText: string,
    winText: string,
    lossText: string,
    callback: string,
  ) {
    super();
    this.event_flag = eventFlag && eventFlag !== "0" ? eventFlag : "";
    this.seen_text = seenText && seenText !== "0" ? seenText : "";
    this.win_text = winText && winText !== "0" ? winText : "";
    this.loss_text = lossText && lossText !== "0" ? lossText : "";
    this.callback = callback && callback !== "0" ? callback : "";
  }

  public execute(gameState: GameState, eventManager: EventManager, overworld: OverworldContext): void {
    const runner = this.runner ?? null;
    if (!runner) {
      throw new Error("TrainerCommand requires an active script runner.");
    }

    const stack = runner._script_stack as ScriptFrame[] | undefined;
    if (stack && stack.length) {
      this.parent_script = stack[stack.length - 1].name;
    }

    const dataLoader = resolveDataLoader(runner, overworld);
    void dataLoader;

    const alreadyDefeated = this.event_flag
      ? Boolean(gameState.wram.event_flags[this.event_flag])
      : false;

    runner.last_value = this.trainer_id;
    runner.last_condition_result = !alreadyDefeated;

    if (alreadyDefeated) {
      if (this.callback) {
        const target = normalizeScriptName(this.callback);
        if (target.startsWith(".")) {
          runner.jump?.(target);
        } else {
          runner.defer?.(target);
        }
      }
      return;
    }

    // ASM mapping: engine/events/trainer_scripts.asm::{TalkToTrainerScript,SeenByTrainerScript}
    // call encountermusic after loadtemptrainer and before any seen text.
    const audioEngine = resolveOverworldAudioEngine(runner, overworld);
    if (typeof (overworld as OverworldContext & { requestEncounterMusic?: (trainerClass: string) => void }).requestEncounterMusic === "function") {
      (overworld as OverworldContext & { requestEncounterMusic?: (trainerClass: string) => void }).requestEncounterMusic?.(this.trainer_class);
    } else {
      const encounterMusic = determineTrainerEncounterMusic(this.trainer_class);
      if (typeof audioEngine?.playMusic === "function") {
        audioEngine.playMusic(encounterMusic, "encounter");
      } else if (typeof audioEngine?.play_music === "function") {
        audioEngine.play_music(encounterMusic, "encounter");
      }
    }

    facePlayerTowardTrainer(gameState, overworld);

    if (this.seen_text) {
      const seenText = resolveBattleTextLabel(runner, overworld, this.seen_text);
      const continuation = new TrainerBattleContinuationCommand(this);
      continuation.runner = runner;
      const frameName = stack && stack.length ? stack[stack.length - 1].name : "trainer";
      const continuationFrame: ScriptFrame = {
        name: `${frameName}#trainer_continue`,
        commands: [continuation],
        index: 0,
        allowFallthrough: false,
      };
      const textCommands = [
        new OpenTextCommand(),
        new ResolvedBattleTextCommand(seenText),
        new WaitButtonCommand(),
        new CloseTextCommand(),
      ];
      for (const command of textCommands) {
        command.runner = runner;
      }
      const textFrame: ScriptFrame = {
        name: `${frameName}#trainer_text`,
        commands: textCommands,
        index: 0,
        allowFallthrough: false,
      };
      if (stack) {
        stack.push(continuationFrame, textFrame);
      }
      return;
    }

    runTrainerBattleSequence(this, runner, gameState, eventManager, overworld);
  }
}

export class LoadTrainerCommand extends Command {
  constructor(private trainerClass: string, private trainerId: string) {
    super();
  }

  public execute(gameState: GameState, _eventManager: EventManager, overworld: OverworldContext): void {
    const runner = this.runner;
    if (!runner) {
      throw new Error("LoadTrainerCommand requires an active script runner.");
    }

    const dataLoader = resolveDataLoader(runner, overworld);
    const getTrainer =
      dataLoader.get_trainer ??
      (dataLoader as DataLoader & { getTrainer?: DataLoader["get_trainer"] }).getTrainer;
    if (typeof getTrainer !== "function") {
      throw new Error("LoadTrainerCommand requires access to trainer data.");
    }
    const trainer = getTrainer.call(dataLoader, this.trainerId);
    if (!trainer) {
      throw new Error(`Unknown trainer '${this.trainerId}' for class ${this.trainerClass}`);
    }
    const resolvedTrainer = normalizeTrainerForBattle(gameState, trainer);

    const party = (resolvedTrainer.party ?? []).filter(Boolean);
    gameState.wram.other_trainer_class = this.trainerClass;
    gameState.wram.other_trainer_id = this.trainerId;
    gameState.wram.other_trainer = resolvedTrainer;
    gameState.wram.other_trainer_party = [...party];
    gameState.wram.wRunningTrainerBattleScript = 0;

    runner.loaded_trainer = resolvedTrainer;
    runner.loaded_trainer_id = this.trainerId;
    runner.last_value = resolvedTrainer;
    runner.last_condition_result = true;
  }
}

export class LoadWildMonCommand extends Command {
  constructor(private species: string, private level: number) {
    super();
  }

  public execute(gameState: GameState, _eventManager: EventManager, _overworld: OverworldContext): void {
    const entry = { species: this.species, level: this.level };
    gameState.wram.wild_pokemon = entry;
    const runner = this.runner;
    if (runner) {
      runner.last_value = entry;
      runner.last_condition_result = true;
    }
  }
}

export class CatchTutorialCommand extends Command {
  constructor(private battleType: string) {
    super();
  }

  public execute(gameState: GameState, eventManager: EventManager, overworld: OverworldContext): void {
    const runner = this.runner;
    if (!runner) {
      throw new Error("CatchTutorialCommand requires an active script runner.");
    }

    const wildInfo = gameState.wram.wild_pokemon;
    if (!wildInfo) {
      throw new Error("CatchTutorialCommand expects LoadWildMonCommand to run beforehand.");
    }
    const species = wildInfo.species;
    const levelValue = wildInfo.level;
    if (species === null || levelValue === null || levelValue === undefined) {
      throw new Error("CatchTutorialCommand missing wild species or level.");
    }
    const level = Number(levelValue);
    if (!Number.isFinite(level)) {
      throw new Error(`Invalid wild level '${levelValue}' supplied to CatchTutorialCommand`);
    }

    const dataLoader = resolveDataLoader(runner, overworld);
    const tutorialRunner = new CatchTutorialRunner(gameState, eventManager, dataLoader);

    const reloadAfterTutorial = (): void => {
      const scriptOverworld = (runner?.overworld ?? overworld) as OverworldContext | null;
      if (!scriptOverworld) {
        return;
      }
      const activeCoord = scriptOverworld._active_coord_event ?? null;
      const reload = scriptOverworld.reload_current_map ?? scriptOverworld.reloadCurrentMap;
      if (typeof reload === "function") {
        reload.call(scriptOverworld);
      }
      if (activeCoord !== null) {
        scriptOverworld._active_coord_event = activeCoord;
      }
    };

    const finish = (): void => {
      runner.just_battled = true;
      runner.last_condition_result = true;
      runner.last_value = this.battleType;
    };

    const runTutorial = (callback?: () => void): void => {
      const onComplete = () => {
        reloadAfterTutorial();
        finish();
        if (callback) {
          callback();
        }
      };
      tutorialRunner.run({
        wild_species: String(species),
        wild_level: level,
        battle_type: this.battleType,
        on_complete: onComplete,
      });
    };

    const hasOverworld = Boolean(runner?.overworld ?? overworld);
    if (!hasOverworld) {
      runTutorial();
      return;
    }

    const schedule = (callback: () => void): boolean => {
      runTutorial(callback);
      return true;
    };

    const queueTask = runner._queue_overworld_task ?? runner._queueOverworldTask;
    if (typeof queueTask === "function") {
      queueTask.call(runner, schedule);
      return;
    }
    runTutorial();
  }
}

export class StartBattleCommand extends Command {
  public execute(gameState: GameState, eventManager: EventManager, overworld: OverworldContext): void {
    const runner = this.runner;
    if (!runner) {
      throw new Error("StartBattleCommand requires an active script runner.");
    }

    const dataLoader = resolveDataLoader(runner, overworld);

    const playerParty: Pokemon[] = (gameState.sram.party?.pokemon ?? []).filter(
      (pokemon): pokemon is Pokemon => pokemon !== null,
    );
    if (!playerParty.length) {
      throw new Error("Cannot start a battle without at least one Pokemon.");
    }

    const playerPokemon = playerParty[0];
    let trainer = (runner.loaded_trainer as Trainer | null) ?? null;
    let enemyParty: Pokemon[] = [];
    let enemyPokemon: Pokemon;

    if (trainer) {
      trainer = normalizeTrainerForBattle(gameState, trainer);
      runner.loaded_trainer = trainer;
      const trainerParty = Array.isArray(trainer.party) ? trainer.party : null;
      if (!trainerParty?.length) {
        throw new Error("Loaded trainer is missing party data.");
      }
      enemyParty = trainerParty.filter((p): p is Pokemon => !!p).map(p => toPokemon(p));
      if (!enemyParty.length) {
        throw new Error("Loaded trainer has an empty party.");
      }
      enemyPokemon = enemyParty[0];
      gameState.wram.other_trainer = trainer;
      gameState.wram.other_trainer_party = [...enemyParty];
    } else {
      const wildData = gameState.wram.wild_pokemon ?? {};
      const speciesName = wildData.species;
      const levelValue = wildData.level;
      if (speciesName === null || levelValue === null || levelValue === undefined) {
        throw new Error("StartBattleCommand requires either trainer data or a wild encounter.");
      }
      const level = Number(levelValue);
      if (!Number.isFinite(level)) {
        throw new Error(`Invalid wild level '${levelValue}' supplied to StartBattleCommand`);
      }
      const species = resolveSpecies(dataLoader, String(speciesName));
      const battleType = String(gameState.wram.battle_type ?? "").toUpperCase();
      enemyPokemon = createPokemon(
        gameState,
        species,
        level,
        battleType === BATTLETYPE_FORCESHINY ? { dvs: FORCED_SHINY_DVS } : {},
      );
      if (battleType === BATTLETYPE_FORCEITEM) {
        enemyPokemon.item = species.item1 ?? species.item2 ?? null;
      }
      enemyParty = [enemyPokemon];
      trainer = null;
      gameState.wram.wild_pokemon = {};
      gameState.wram.other_trainer_class = "";
      gameState.wram.other_trainer_id = "";
      gameState.wram.other_trainer = undefined;
      gameState.wram.other_trainer_party = [];
    }

    const trainerId =
      runner.loaded_trainer_id ??
      (trainer?.trainer_id ? trainer.trainer_id : null);
    let trainerReward = 0;
    if (trainer && enemyParty.length) {
      const rewardFn =
        dataLoader.get_trainer_base_reward ??
        (dataLoader as DataLoader & { getTrainerBaseReward?: DataLoader["get_trainer_base_reward"] })
          .getTrainerBaseReward;
      let baseReward = 0;
      if (trainerId && typeof rewardFn === "function") {
        baseReward = rewardFn.call(dataLoader, trainerId);
      }
      if (baseReward <= 0 && trainer.base_reward) {
        baseReward = trainer.base_reward;
      }
      if (baseReward > 0) {
        const lastLevel = Number(enemyParty[enemyParty.length - 1].level ?? 0);
        trainerReward = Math.max(0, baseReward * lastLevel);
      }
    }

    const battleEvent = new StartBattleEvent({
      player_pokemon: playerPokemon,
      enemy_pokemon: enemyPokemon,
      player_party: playerParty,
      enemy_party: enemyParty,
      trainer,
      trainer_id: trainerId,
      trainer_reward: trainerReward,
    });

    if (runner.just_battled) {
      runner.just_battled = false;
    }

    runner.pause?.();
    runner.last_condition_result = false;
    runner.last_value = null;
    gameState.wram.battle_result = 0;

    const handleBattleComplete = (event: Event, state: GameState): void => {
      eventManager.off("battle_complete", handleBattleComplete);
      const result = resolveBattleResult(event?.data?.result ?? state.wram.battle_result ?? 0);
      state.wram.battle_result = result;
      const resultCode = battleResultCode(result);
      const battleType = String(state.wram.battle_type ?? "").replace(/,+$/, "").trim().toUpperCase();
      const canLose = battleType === BATTLETYPE_CANLOSE;
      runner.last_condition_result = resultCode !== 0;
      runner.last_value = resultCode;
      runner.loaded_trainer = null;
      runner.loaded_trainer_id = null;
      const winLossText = runner.variables?.["_win_loss_text"] as WinLossTextState | undefined;
      if (trainer && winLossText) {
        delete runner.variables["_win_loss_text"];
        const label = resultCode === 0 ? winLossText.winText : winLossText.lossText;
        if (label) {
          const message = resolveBattleTextLabel(runner, overworld, label);
          const stack = runner._script_stack as ScriptFrame[] | undefined;
          if (stack) {
            const commands: Command[] = [
              new OpenTextCommand(),
              new ResolvedBattleTextCommand(message, true),
            ];
            for (const command of commands) {
              command.runner = runner;
            }
            stack.push({
              name: "winlosstext#post_battle",
              commands,
              index: 0,
              allowFallthrough: false,
            });
          } else {
            openText(eventManager);
            showText(eventManager, message, { auto_close_after_wait: true });
            waitForInput(eventManager);
          }
        }
      }
      handlePostBattleSceneFixes(runner, gameState, overworld, resultCode);
      if (resultCode === 1 && !canLose) {
        // ASM: reloadmapafterbattle jumps to the whiteout script on an ordinary loss,
        // so it must not retain pending reload-map state.
        state.wram.reload_map_after_battle = false;
        runner.stop_all_scripts?.();
        clearBattleScriptStackForWhiteout(runner);
        return;
      }
      runner.just_battled = true;
      if (state.wram.reload_map_after_battle && overworld) {
        persistLastTalkedPosition(gameState, overworld);
        runner.pending_reload_map = overworld.current_map_name ?? null;
        state.wram.reload_map_after_battle = false;
      }
      runner.resume?.();
    };

    eventManager.on("battle_complete", handleBattleComplete);
    eventManager.dispatch(battleEvent);
  }
}

class TrainerBattleContinuationCommand extends Command {
  constructor(private trainer: TrainerCommand) {
    super();
  }

  public execute(gameState: GameState, eventManager: EventManager, overworld: OverworldContext): void {
    const runner: BattleRunner | undefined = this.runner;
    if (!runner) {
      throw new Error("Trainer battle continuation requires an active script runner.");
    }
    runTrainerBattleSequence(this.trainer, runner, gameState, eventManager, overworld);
  }
}

const runTrainerBattleSequence = (
  trainer: TrainerCommand,
  runner: BattleRunner,
  gameState: GameState,
  eventManager: EventManager,
  overworld: OverworldContext,
): void => {
  const loadTrainer = new LoadTrainerCommand(trainer.trainer_class, trainer.trainer_id);
  loadTrainer.runner = runner;
  loadTrainer.execute(gameState, eventManager, overworld);

  const reloadAfterBattle = new ReloadMapAfterBattleCommand();
  reloadAfterBattle.runner = runner;
  reloadAfterBattle.execute(gameState, eventManager, overworld);

  const queuePostBattleFrame = (message: string | null, callback: string | null): boolean => {
    const stack = runner._script_stack as ScriptFrame[] | undefined;
    if (!stack) {
      return false;
    }
    const commands: Command[] = [];
    if (message) {
      commands.push(new OpenTextCommand(), new ResolvedBattleTextCommand(message, true));
    }
    if (callback) {
      commands.push(new TrainerBattleCallbackCommand(callback, trainer.parent_script));
    } else {
      commands.push(new ClearTrainerBattleScriptCommand());
    }
    if (!commands.length) {
      return false;
    }
    for (const command of commands) {
      command.runner = runner;
    }
    const parentName = stack.length ? stack[stack.length - 1].name : (trainer.parent_script ?? "trainer");
    stack.push({
      name: `${parentName}#trainer_post_battle`,
      commands,
      index: 0,
      allowFallthrough: false,
    });
    return true;
  };

  let handled = false;
  const handleBattleComplete = (event: Event, state: GameState): void => {
    if (handled) {
      return;
    }
    handled = true;
    eventManager.off("battle_complete", handleBattleComplete);
    const result = resolveBattleResult(event?.data?.result ?? state.wram.battle_result ?? 0);
    const resultCode = battleResultCode(result);
    const win = resultCode === 0;

    const battleType = String(state.wram.battle_type ?? "").replace(/,+$/, "").trim().toUpperCase();
    const canLose = battleType === BATTLETYPE_CANLOSE;
    const continuedAfterBattle = win || canLose;

    if (continuedAfterBattle) {
      state.wram.wRunningTrainerBattleScript = RUNNING_TRAINER_BATTLE_SCRIPT_ACTIVE;
    }

    if (continuedAfterBattle && trainer.event_flag) {
      if (runner?.set_event_flag) {
        runner.set_event_flag(trainer.event_flag, true);
      } else {
        applyEventFlag(gameState, trainer.event_flag, { value: true, overworld });
      }
    }

    if (continuedAfterBattle) {
      const textLabel = win ? trainer.win_text : trainer.loss_text;
      const message = textLabel ? resolveBattleTextLabel(runner, overworld, textLabel) : null;
      const queued = queuePostBattleFrame(message, trainer.callback || null);
      if (!queued) {
        if (message) {
          openText(eventManager);
          showText(eventManager, message, { auto_close_after_wait: true });
          waitForInput(eventManager);
        }
        if (!trainer.callback) {
          state.wram.wRunningTrainerBattleScript = 0;
          runner.just_battled = false;
        }
        if (trainer.callback) {
          const target = normalizeScriptName(trainer.callback);
          if (target.startsWith(".")) {
            if (trainer.parent_script) {
              runner.jump?.(target, trainer.parent_script);
            } else {
              runner.jump?.(target);
            }
          } else {
            runner.defer?.(target);
          }
        }
      }
    }

    runner.just_battled = continuedAfterBattle;
    if (!continuedAfterBattle) {
      state.wram.wRunningTrainerBattleScript = 0;
      clearBattleScriptStackForWhiteout(runner);
    }
  };

  eventManager.on("battle_complete", handleBattleComplete);

  const startBattle = new StartBattleCommand();
  startBattle.runner = runner;
  startBattle.execute(gameState, eventManager, overworld);
};

export class EndCommand extends Command {
  public execute(_gameState: GameState, _eventManager: EventManager, _overworld: OverworldContext): void {
    const runner = this.runner;
    if (runner && typeof runner._terminate_current_script === "function") {
      runner._terminate_current_script();
    }
  }
}
