import fs from "fs";
import path from "path";
import { BATTLETOWER_STREAK_LENGTH } from "@pokecrystal/core/core/constants";
import { BATTLE_TOWER_TRAINER_SLOT_SENTINEL } from "@pokecrystal/core/core/memory/sram";
import {
  PokemonSchema,
  TrainerSchema,
  type LearnedMove,
  type Pokemon,
  type PokemonSpecies,
  type Trainer,
  toPokemon,
} from "@pokecrystal/core/core/models";
import { getDisassemblyRoot } from "@pokecrystal/core/core/paths";
import { GameState } from "@pokecrystal/core/core/state";
import { MoveName } from "@pokecrystal/core/core/enums";
import type { Overworld as OverworldType } from "@pokecrystal/core/types/overworld";

type BattleTowerTrainer = {
  index: number;
  trainer_class: string;
  name: string;
};

type BattleTowerMon = {
  species: string;
  item: string | null;
  moves: string[];
  ot_id: number;
  exp: number;
  stat_exp: number[];
  dvs: number[];
  pp: number[];
  happiness: number;
  pokerus: number[];
  level: number;
  status: number[];
  stats: Record<string, number>;
  nickname: string;
};

type DataLoader = {
  get_pokemon?: (speciesId: string) => PokemonSpecies | null | undefined;
  getPokemon?: (speciesId: string) => PokemonSpecies | null | undefined;
  getPokemonSpecies?: (speciesId: string) => PokemonSpecies | null | undefined;
  get_pokemon_species?: (speciesId: string) => PokemonSpecies | null | undefined;
  getSpecies?: (speciesId: string) => PokemonSpecies | null | undefined;
  move_data?: Record<string, { pp?: number }>;
  moveData?: Record<string, { pp?: number }>;
  pokemon_data?: Record<string, PokemonSpecies>;
  pokemonData?: Record<string, PokemonSpecies>;
  pokemon?: Record<string, PokemonSpecies>;
};

const MOVE_NAME_VALUES = new Set<string>(Object.values(MoveName));

const toMoveName = (value: string): MoveName => {
  let normalized = value.trim().toUpperCase();
  // Handle some common battle tower specific move aliases
  if (normalized === 'PSYCHIC') {
      normalized = 'PSYCHIC_M';
  }
  if (MOVE_NAME_VALUES.has(normalized)) {
    return normalized as MoveName;
  }
  throw new Error(`Unknown Battle Tower move '${value}'`);
};

const DISASSEMBLY_ROOT = getDisassemblyRoot();

const publicDisassemblyRoot = (): string => {
  let current = process.cwd();
  for (let depth = 0; depth < 8; depth += 1) {
    const candidate = path.join(current, "apps", "web", "public", "disassembly");
    if (fs.existsSync(candidate)) {
      return candidate;
    }
    const localCandidate = path.join(current, "public", "disassembly");
    if (fs.existsSync(localCandidate)) {
      return localCandidate;
    }
    const parent = path.dirname(current);
    if (parent === current) {
      break;
    }
    current = parent;
  }
  return path.join(process.cwd(), "apps", "web", "public", "disassembly");
};

const battleTowerDataPath = (...parts: string[]): string => {
  const candidates = [
    path.join(DISASSEMBLY_ROOT, ...parts),
    path.join(publicDisassemblyRoot(), ...parts),
  ];
  const found = candidates.find((candidate) => fs.existsSync(candidate));
  return found ?? candidates[0];
};

const parseNumeric = (token: string): number => {
  const cleaned = token.trim().replace(/,$/, "");
  if (cleaned.startsWith("$")) {
    return parseInt(cleaned.slice(1), 16);
  }
  if (cleaned.toLowerCase().startsWith("0x")) {
    return parseInt(cleaned, 16);
  }
  return parseInt(cleaned, 10);
};

let trainerClassCache: string[] | null = null;
let trainerSpriteCache: string[] | null = null;
let trainerRosterCache: BattleTowerTrainer[] | null = null;
let monGroupCache: BattleTowerMon[][] | null = null;

const parseTrainerClasses = (): string[] => {
  if (trainerClassCache) {
    return trainerClassCache;
  }
  const constantsPath = battleTowerDataPath(
    "constants",
    "trainer_constants.asm"
  );
  if (!fs.existsSync(constantsPath)) {
    throw new Error("Missing trainer_constants.asm for Battle Tower data.");
  }
  const classes: string[] = [];
  const lines = fs.readFileSync(constantsPath, "utf-8").split(/\r?\n/);
  for (const raw of lines) {
    const line = raw.trim();
    if (!line.startsWith("trainerclass ")) {
      continue;
    }
    const [, className] = line.split(/\s+/, 2);
    if (!className || className === "TRAINER_NONE") {
      continue;
    }
    classes.push(className);
  }
  if (!classes.length) {
    throw new Error("Failed to parse trainer class order for Battle Tower.");
  }
  trainerClassCache = classes;
  return classes;
};

const parseTrainerSprites = (): string[] => {
  if (trainerSpriteCache) {
    return trainerSpriteCache;
  }
  const spritesPath = battleTowerDataPath(
    "data",
    "trainers",
    "sprites.asm"
  );
  if (!fs.existsSync(spritesPath)) {
    throw new Error("Missing trainers/sprites.asm for Battle Tower data.");
  }
  const sprites: string[] = [];
  const lines = fs.readFileSync(spritesPath, "utf-8").split(/\r?\n/);
  for (const raw of lines) {
    const line = raw.trim();
    if (!line || line.startsWith(";") || line.startsWith("table_width")) {
      continue;
    }
    if (!line.startsWith("db ")) {
      continue;
    }
    const tokens = line
      .slice(3)
      .split(",")
      .map((token) => token.trim())
      .filter(Boolean);
    sprites.push(...tokens);
  }
  if (!sprites.length) {
    throw new Error("Failed to parse Battle Tower sprite table.");
  }
  trainerSpriteCache = sprites;
  return sprites;
};

const parseTrainerRoster = (): BattleTowerTrainer[] => {
  if (trainerRosterCache) {
    return trainerRosterCache;
  }
  const rosterPath = battleTowerDataPath(
    "data",
    "battle_tower",
    "classes.asm"
  );
  if (!fs.existsSync(rosterPath)) {
    throw new Error("Missing battle_tower/classes.asm for Battle Tower data.");
  }
  const roster: BattleTowerTrainer[] = [];
  const pattern = /bt_trainer\s+([A-Z0-9_]+),\s+"([^"]+)"/;
  const lines = fs.readFileSync(rosterPath, "utf-8").split(/\r?\n/);
  lines.forEach((raw, index) => {
    const match = pattern.exec(raw);
    if (!match) {
      return;
    }
    const [, trainerClass, name] = match;
    roster.push({ index, trainer_class: trainerClass, name });
  });
  if (!roster.length) {
    throw new Error("Battle Tower trainer roster could not be parsed.");
  }
  trainerRosterCache = roster;
  return roster;
};

const parseMonGroups = (): BattleTowerMon[][] => {
  if (monGroupCache) {
    return monGroupCache;
  }
  const partiesPath = battleTowerDataPath(
    "data",
    "battle_tower",
    "parties.asm"
  );
  if (!fs.existsSync(partiesPath)) {
    throw new Error("Missing battle_tower/parties.asm for Battle Tower data.");
  }
  const groups: BattleTowerMon[][] = [];
  let current: BattleTowerMon[] = [];
  let mon: Partial<BattleTowerMon> | null = null;
  let stage = "";
  let statExp: number[] = [];
  let statValues: number[] = [];
  const lines = fs.readFileSync(partiesPath, "utf-8").split(/\r?\n/);
  for (const raw of lines) {
    if (raw.trim().startsWith("; BattleTowerMons group")) {
      if (current.length) {
        groups.push(current);
        current = [];
      }
      continue;
    }
    const line = raw.split(";", 1)[0].trim();
    if (!line) {
      continue;
    }
    if (stage === "ot_id" && line.startsWith("db ")) {
      continue;
    }
    if (line.startsWith("db ") && !mon) {
      const tokens = line
        .slice(3)
        .split(",")
        .map((token) => token.trim())
        .filter(Boolean);
      mon = { species: tokens[0] };
      stage = "item";
      continue;
    }
    if (!mon) {
      continue;
    }
    if (stage === "item" && line.startsWith("db ")) {
      const tokens = line
        .slice(3)
        .split(",")
        .map((token) => token.trim())
        .filter(Boolean);
      const item = tokens[0];
      mon.item = item === "NO_ITEM" || item === "0" ? null : item;
      stage = "moves";
      continue;
    }
    if (stage === "moves" && line.startsWith("db ")) {
      const moves = line
        .slice(3)
        .split(",")
        .map((token) => token.trim())
        .filter(Boolean);
      if (!mon.moves) mon.moves = [];
      mon.moves.push(...moves);
      if (mon.moves.length >= 4) {
        mon.moves = mon.moves.slice(0, 4);
      }
      continue;
    }
    if (stage === "moves" && line.startsWith("dw ")) {
      stage = "ot_id";
      mon.ot_id = parseNumeric(line.slice(3));
      stage = "exp";
      continue;
    }
    if (stage === "ot_id" && line.startsWith("dw ")) {
      mon.ot_id = parseNumeric(line.slice(3));
      stage = "exp";
      continue;
    }
    if (stage === "exp" && !line.startsWith("bigdt") && !line.startsWith("dname")) {
      continue;
    }
    if (stage === "exp" && line.startsWith("bigdt")) {
      mon.exp = parseNumeric(line.split(/\s+/)[1] ?? "0");
      stage = "stat_exp";
      statExp = [];
      continue;
    }
    if (stage === "stat_exp" && line.startsWith("bigdw")) {
      statExp.push(parseNumeric(line.split(/\s+/)[1] ?? "0"));
      if (statExp.length >= 5) {
        mon.stat_exp = statExp;
        stage = "dvs";
      }
      continue;
    }
    if (stage === "dvs" && line.startsWith("dn")) {
      const tokens = line
        .slice(2)
        .split(",")
        .map((token) => token.trim())
        .filter(Boolean);
      mon.dvs = tokens.map((token) => parseNumeric(token));
      stage = "pp";
      continue;
    }
    if (stage === "pp" && line.startsWith("db ")) {
      const tokens = line
        .slice(3)
        .split(",")
        .map((token) => token.trim())
        .filter(Boolean);
      mon.pp = tokens.map((token) => parseNumeric(token));
      stage = "happiness";
      continue;
    }
    if (stage === "happiness" && line.startsWith("db ")) {
      mon.happiness = parseNumeric(line.slice(3));
      stage = "pokerus";
      continue;
    }
    if (stage === "pokerus" && line.startsWith("db ")) {
      const tokens = line
        .slice(3)
        .split(",")
        .map((token) => token.trim())
        .filter(Boolean);
      mon.pokerus = tokens.map((token) => parseNumeric(token));
      stage = "level";
      continue;
    }
    if (stage === "level" && line.startsWith("db ")) {
      mon.level = parseNumeric(line.slice(3));
      stage = "status";
      continue;
    }
    if (stage === "status" && line.startsWith("db ")) {
      const tokens = line
        .slice(3)
        .split(",")
        .map((token) => token.trim())
        .filter(Boolean);
      mon.status = tokens.map((token) => parseNumeric(token));
      statValues = [];
      stage = "stats";
      continue;
    }
    if (stage === "stats" && line.startsWith("bigdw")) {
      statValues.push(parseNumeric(line.split(/\s+/)[1] ?? "0"));
      if (statValues.length >= 7) {
        mon.stats = {
          hp: statValues[0],
          max_hp: statValues[1],
          attack: statValues[2],
          defense: statValues[3],
          speed: statValues[4],
          special_attack: statValues[5],
          special_defense: statValues[6],
        };
        stage = "nickname";
      }
      continue;
    }
    if (stage === "nickname" && line.startsWith("dname")) {
      const nicknameMatch = /"([^"]+)"/.exec(line);
      if (!nicknameMatch) {
        throw new Error("Failed to parse Battle Tower nickname.");
      }
      mon.nickname = nicknameMatch[1];
      current.push({
        species: String(mon.species ?? ""),
        item: mon.item ?? null,
        moves: mon.moves ?? [],
        ot_id: Number(mon.ot_id ?? 0),
        exp: Number(mon.exp ?? 0),
        stat_exp: mon.stat_exp ?? [],
        dvs: mon.dvs ?? [0, 0, 0, 0],
        pp: mon.pp ?? [0, 0, 0, 0],
        happiness: Number(mon.happiness ?? 0),
        pokerus: mon.pokerus ?? [0, 0, 0],
        level: Number(mon.level ?? 1),
        status: mon.status ?? [0, 0],
        stats: mon.stats ?? {},
        nickname: String(mon.nickname ?? ""),
      });
      mon = null;
      stage = "";
      continue;
    }
  }
  if (mon) {
    console.error("Battle Tower party parsing terminated mid-entry with mon:", mon, "at stage:", stage);
    throw new Error("Battle Tower party parsing terminated mid-entry.");
  }
  if (current.length) {
    groups.push(current);
  }
  if (!groups.length || groups.some((group) => group.length === 0)) {
    throw new Error("Battle Tower parties file yielded no Pokemon groups.");
  }
  monGroupCache = groups;
  return groups;
};

const cachedTrainerData = (): {
  trainers: BattleTowerTrainer[];
  classToSprite: Record<string, string>;
} => {
  const trainers = [...parseTrainerRoster()];
  const classes = parseTrainerClasses();
  const sprites = parseTrainerSprites();
  const classToSprite: Record<string, string> = {};
  classes.forEach((className, index) => {
    if (index < sprites.length) {
      classToSprite[className] = sprites[index];
    }
  });
  return { trainers, classToSprite };
};

const cachedMonGroups = (): BattleTowerMon[][] => [...parseMonGroups()];

const chooseTrainerIndex = (history: number[], rosterSize: number): number => {
  const available = Array.from({ length: rosterSize }, (_, idx) => idx);
  const recent = new Set(
    history.filter(
      (value) => value !== BATTLE_TOWER_TRAINER_SLOT_SENTINEL && value >= 0 && value < rosterSize
    )
  );
  const candidates = available.filter((value) => !recent.has(value));
  const pickFrom = candidates.length ? candidates : available;
  const index = Math.floor(Math.random() * pickFrom.length);
  return pickFrom[index] ?? 0;
};

const computeHpDv = (dvs: number[]): number => {
  const [attack, defense, speed, special] = dvs.map((value) => value & 0xf);
  let hp = 0;
  if (attack & 1) hp |= 0x8;
  if (defense & 1) hp |= 0x4;
  if (speed & 1) hp |= 0x2;
  if (special & 1) hp |= 0x1;
  return hp;
};

const randomSample = <T,>(items: T[], count: number): T[] => {
  const copy = [...items];
  const result: T[] = [];
  while (result.length < count && copy.length) {
    const index = Math.floor(Math.random() * copy.length);
    result.push(copy.splice(index, 1)[0]);
  }
  return result;
};

const pickParty = (
  group: BattleTowerMon[],
  {
    groupIndex,
    history,
  }: { groupIndex: number; history: Array<[number, number]> }
): Array<[number, BattleTowerMon]> => {
  const recentSpecies = new Set(
    history
      .slice(-6)
      .filter(([lvl, idx]) => lvl === groupIndex && idx >= 0 && idx < group.length)
      .map(([, idx]) => group[idx].species)
  );
  const selected: Array<[number, BattleTowerMon]> = [];
  const pool = group.map((mon, index) => [index, mon] as [number, BattleTowerMon]);

  const validForSelection = ([, mon]: [number, BattleTowerMon]): boolean => {
    if (recentSpecies.has(mon.species)) {
      return false;
    }
    return !selected.some(([, selectedMon]) => {
      if (selectedMon.species === mon.species) {
        return true;
      }
      return Boolean(selectedMon.item && mon.item && selectedMon.item === mon.item);
    });
  };

  while (selected.length < 3) {
    const candidates = pool.filter(
      (entry) => !selected.some(([index]) => index === entry[0]) && validForSelection(entry)
    );
    if (!candidates.length) {
      break;
    }
    selected.push(randomSample(candidates, 1)[0]);
  }

  if (selected.length === 3) {
    return selected;
  }

  throw new Error("Battle Tower could not build a legal opponent party from ASM data.");
};

const resolveSpecies = (speciesId: string, dataLoader?: DataLoader | null): PokemonSpecies => {
  const id = String(speciesId ?? "").toUpperCase();
  const lookup =
    dataLoader?.get_pokemon ??
    dataLoader?.getPokemon ??
    dataLoader?.getPokemonSpecies ??
    dataLoader?.get_pokemon_species ??
    dataLoader?.getSpecies;
  if (lookup) {
    const species = lookup.call(dataLoader, id);
    if (!species) {
      throw new Error(`Unknown Battle Tower species '${id}'.`);
    }
    return species;
  }
  const map =
    dataLoader?.pokemon_data ?? dataLoader?.pokemonData ?? dataLoader?.pokemon;
  const cached = map?.[id];
  if (cached) {
    return cached;
  }
  throw new Error("Battle Tower requires a species data loader.");
};

const buildLearnedMoves = (mon: BattleTowerMon, dataLoader?: DataLoader | null): LearnedMove[] => {
  const moves: LearnedMove[] = [];
  const moveData = dataLoader?.move_data ?? dataLoader?.moveData ?? {};
  mon.moves.forEach((moveName, index) => {
    if (!moveName || moveName === "NO_MOVE") {
      return;
    }
    const normalizedName = toMoveName(moveName);
    let currentPp = mon.pp[index] ?? 0;
    const cachedMoveData = moveData[normalizedName];
    if (currentPp <= 0 && cachedMoveData?.pp) {
      currentPp = Number(cachedMoveData.pp);
    }
    moves.push({ name: normalizedName, current_pp: currentPp });
  });
  return moves;
};

const buildPokemon = (
  mon: BattleTowerMon,
  trainerName: string,
  dataLoader?: DataLoader | null
): Pokemon => {
  const species = resolveSpecies(mon.species, dataLoader);
  const moves = buildLearnedMoves(mon, dataLoader);
  const dvValues = mon.dvs.slice(0, 4);
  const dvs = {
    attack: dvValues[0] ?? 0,
    defense: dvValues[1] ?? 0,
    speed: dvValues[2] ?? 0,
    special: dvValues[3] ?? 0,
    hp: computeHpDv(dvValues),
  };
  const stats = mon.stats ?? {};
  return toPokemon(
    PokemonSchema.parse({
      species,
      nickname: mon.nickname || species.id,
      gender: null,
      item: mon.item ?? null,
      moves,
      level: mon.level,
      hp: stats.hp ?? stats.max_hp ?? 1,
      max_hp: stats.max_hp ?? stats.hp ?? 1,
      dvs,
      status: null,
      original_trainer_name: trainerName,
      original_trainer_id: mon.ot_id,
      experience: mon.exp,
      happiness: mon.happiness,
      attack: stats.attack ?? 0,
      defense: stats.defense ?? 0,
      speed: stats.speed ?? 0,
      special_attack: stats.special_attack ?? 0,
      special_defense: stats.special_defense ?? 0,
    })
  );
};

export function load_battle_tower_opponent(
  game_state: GameState,
  data_loader: DataLoader,
  overworld?: OverworldType | null
): [Trainer, string] {
  // ASM: data/battle_tower/classes.asm + data/battle_tower/parties.asm loader flow.
  const { trainers, classToSprite } = cachedTrainerData();
  const monGroups = cachedMonGroups();
  const state = game_state.sram.battle_tower;
  const levelGroup = Math.max(
    1,
    Math.min(10, Number(state.level_group ?? game_state.wram.wBTChoiceOfLvlGroup ?? 1))
  );
  const groupIndex = levelGroup - 1;
  if (groupIndex >= monGroups.length) {
    throw new Error(`Battle Tower level group ${levelGroup} has no parties.`);
  }

  const trainerIndex = chooseTrainerIndex(state.trainer_history ?? [], trainers.length);
  const trainerInfo = trainers[trainerIndex];
  const scriptMemory = (game_state.wram.script_memory ?? {}) as Record<string, unknown>;
  if (!game_state.wram.script_memory) {
    game_state.wram.script_memory = scriptMemory as Record<string, number>;
  }
  const specials = (scriptMemory.specials ??= {}) as Record<string, unknown>;
  const monHistory = (specials.battle_tower_mons ??= []) as Array<[number, number]>;

  const partyEntries = pickParty(monGroups[groupIndex], {
    groupIndex,
    history: monHistory,
  });
  const party = partyEntries.map(([, entry]) =>
    buildPokemon(entry, trainerInfo.name, data_loader)
  );

  const trainerId = `BATTLE_TOWER_${trainerIndex}`;
  const trainer: Trainer = TrainerSchema.parse({
    name: `${trainerInfo.name}@`,
    trainer_id: trainerId,
    trainer_class: trainerInfo.trainer_class,
    party,
    win_quote: "",
    lose_quote: "",
    items: [],
    base_reward: 0,
    ai_move_flags: 0,
    ai_item_switch_flags: 0,
    encounter_music: "",
    ai_layers: [],
  });

  const slot = Math.min(
    Math.max(0, Number(game_state.wram.wNrOfBeatenBattleTowerTrainers ?? 0)),
    BATTLETOWER_STREAK_LENGTH - 1
  );
  if (state.trainer_history) {
    state.trainer_history[slot] = trainerIndex;
  }
  for (const [index] of partyEntries) {
    monHistory.push([groupIndex, index]);
  }
  if (monHistory.length > 6) {
    monHistory.splice(0, monHistory.length - 6);
  }

  const spriteConstant = classToSprite[trainerInfo.trainer_class];
  if (!spriteConstant) {
    throw new Error(
      `Missing Battle Tower sprite mapping for trainer class '${trainerInfo.trainer_class}'.`
    );
  }
  return [trainer, spriteConstant];
}
