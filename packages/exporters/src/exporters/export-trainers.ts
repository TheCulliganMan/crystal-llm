import fs from "fs";
import path from "path";
import type { Trainer } from "@pokecrystal/core/core/models/trainer";
import { pokemonSpeciesDisplayName, type PokemonSpecies } from "@pokecrystal/core/core/models/pokemon";
import { getDisassemblyRoot } from "@pokecrystal/core/core/paths";
import { parseAsmNumber, stripInlineComment, writeJsonToTargets } from "./asm-utils";
import { parseMoves } from "./export-data";

const RIVAL_NAME_PLACEHOLDER = "<RIVAL>";

const TRAINER_ENCOUNTER_MUSIC: Record<string, string> = {
  FALKNER: "MUSIC_YOUNGSTER_ENCOUNTER",
  WHITNEY: "MUSIC_LASS_ENCOUNTER",
  BUGSY: "MUSIC_YOUNGSTER_ENCOUNTER",
  MORTY: "MUSIC_OFFICER_ENCOUNTER",
  PRYCE: "MUSIC_OFFICER_ENCOUNTER",
  JASMINE: "MUSIC_LASS_ENCOUNTER",
  CHUCK: "MUSIC_OFFICER_ENCOUNTER",
  CLAIR: "MUSIC_BEAUTY_ENCOUNTER",
  RIVAL1: "MUSIC_RIVAL_ENCOUNTER",
  POKEMON_PROF: "MUSIC_HIKER_ENCOUNTER",
  WILL: "MUSIC_HIKER_ENCOUNTER",
  CAL: "MUSIC_HIKER_ENCOUNTER",
  BRUNO: "MUSIC_OFFICER_ENCOUNTER",
  KAREN: "MUSIC_HIKER_ENCOUNTER",
  KOGA: "MUSIC_HIKER_ENCOUNTER",
  CHAMPION: "MUSIC_OFFICER_ENCOUNTER",
  BROCK: "MUSIC_YOUNGSTER_ENCOUNTER",
  MISTY: "MUSIC_LASS_ENCOUNTER",
  LT_SURGE: "MUSIC_OFFICER_ENCOUNTER",
  SCIENTIST: "MUSIC_ROCKET_ENCOUNTER",
  ERIKA: "MUSIC_OFFICER_ENCOUNTER",
  YOUNGSTER: "MUSIC_YOUNGSTER_ENCOUNTER",
  SCHOOLBOY: "MUSIC_YOUNGSTER_ENCOUNTER",
  BIRD_KEEPER: "MUSIC_YOUNGSTER_ENCOUNTER",
  LASS: "MUSIC_LASS_ENCOUNTER",
  JANINE: "MUSIC_LASS_ENCOUNTER",
  COOLTRAINERM: "MUSIC_HIKER_ENCOUNTER",
  COOLTRAINERF: "MUSIC_BEAUTY_ENCOUNTER",
  BEAUTY: "MUSIC_BEAUTY_ENCOUNTER",
  POKEMANIAC: "MUSIC_POKEMANIAC_ENCOUNTER",
  GRUNTM: "MUSIC_ROCKET_ENCOUNTER",
  GENTLEMAN: "MUSIC_HIKER_ENCOUNTER",
  SKIER: "MUSIC_BEAUTY_ENCOUNTER",
  TEACHER: "MUSIC_BEAUTY_ENCOUNTER",
  SABRINA: "MUSIC_BEAUTY_ENCOUNTER",
  BUG_CATCHER: "MUSIC_YOUNGSTER_ENCOUNTER",
  FISHER: "MUSIC_HIKER_ENCOUNTER",
  SWIMMERM: "MUSIC_HIKER_ENCOUNTER",
  SWIMMERF: "MUSIC_BEAUTY_ENCOUNTER",
  SAILOR: "MUSIC_HIKER_ENCOUNTER",
  SUPER_NERD: "MUSIC_POKEMANIAC_ENCOUNTER",
  RIVAL2: "MUSIC_RIVAL_ENCOUNTER",
  GUITARIST: "MUSIC_HIKER_ENCOUNTER",
  HIKER: "MUSIC_HIKER_ENCOUNTER",
  BIKER: "MUSIC_HIKER_ENCOUNTER",
  BLAINE: "MUSIC_OFFICER_ENCOUNTER",
  BURGLAR: "MUSIC_POKEMANIAC_ENCOUNTER",
  FIREBREATHER: "MUSIC_HIKER_ENCOUNTER",
  JUGGLER: "MUSIC_POKEMANIAC_ENCOUNTER",
  BLACKBELT_T: "MUSIC_HIKER_ENCOUNTER",
  EXECUTIVEM: "MUSIC_ROCKET_ENCOUNTER",
  PSYCHIC_T: "MUSIC_YOUNGSTER_ENCOUNTER",
  PICNICKER: "MUSIC_LASS_ENCOUNTER",
  CAMPER: "MUSIC_YOUNGSTER_ENCOUNTER",
  EXECUTIVEF: "MUSIC_ROCKET_ENCOUNTER",
  SAGE: "MUSIC_SAGE_ENCOUNTER",
  MEDIUM: "MUSIC_SAGE_ENCOUNTER",
  BOARDER: "MUSIC_HIKER_ENCOUNTER",
  POKEFANM: "MUSIC_HIKER_ENCOUNTER",
  KIMONO_GIRL: "MUSIC_KIMONO_ENCOUNTER",
  TWINS: "MUSIC_LASS_ENCOUNTER",
  POKEFANF: "MUSIC_BEAUTY_ENCOUNTER",
  RED: "MUSIC_HIKER_ENCOUNTER",
  BLUE: "MUSIC_RIVAL_ENCOUNTER",
  OFFICER: "MUSIC_HIKER_ENCOUNTER",
  GRUNTF: "MUSIC_ROCKET_ENCOUNTER",
  MYSTICALMAN: "MUSIC_HIKER_ENCOUNTER",
};

function cleanTrainerName(rawName: string): string {
  const normalized = rawName.trim();
  if (normalized === "?" || normalized === "?@") {
    return `${RIVAL_NAME_PLACEHOLDER}@`;
  }
  return normalized;
}

function parseTrainerMetadata(): Array<[string, string]> {
  const root = getDisassemblyRoot();
  const constantsPath = path.join(root, "constants", "trainer_constants.asm");
  const groupsPath = path.join(root, "data", "trainers", "party_pointers.asm");
  const classOrder: string[] = [];
  const classLabels = new Map<string, string[]>();
  let currentClass: string | null = null;
  let skipClass = false;

  for (const raw of fs.readFileSync(constantsPath, "utf8").split(/\r?\n/)) {
    const line = raw.trim();
    if (!line || line.startsWith(";")) continue;
    if (line.startsWith("trainerclass ")) {
      const className = line.split(/\s+/)[1];
      if (className === "TRAINER_NONE") {
        skipClass = true;
        currentClass = null;
        continue;
      }
      skipClass = false;
      currentClass = className;
      classOrder.push(className);
      classLabels.set(className, []);
      continue;
    }
    if (line.startsWith("const ") && !skipClass && currentClass) {
      classLabels.set(currentClass, [...(classLabels.get(currentClass) ?? []), line.split(/\s+/)[1]]);
    }
  }

  const orderedMetadata: Array<[string, string]> = [];
  for (const className of classOrder) {
    for (const label of classLabels.get(className) ?? []) {
      orderedMetadata.push([className, label]);
    }
  }
  return orderedMetadata;
}

function parseTrainerClassBaseRewards(): Record<string, number> {
  const root = getDisassemblyRoot();
  const constantsPath = path.join(root, "constants", "trainer_constants.asm");
  const attributesPath = path.join(root, "data", "trainers", "attributes.asm");
  const classOrder: string[] = [];

  for (const raw of fs.readFileSync(constantsPath, "utf8").split(/\r?\n/)) {
    const line = stripInlineComment(raw).trim();
    if (!line.startsWith("trainerclass ")) {
      continue;
    }
    const className = line.split(/\s+/)[1];
    if (className && className !== "TRAINER_NONE") {
      classOrder.push(className);
    }
  }

  const rewards: number[] = [];
  for (const raw of fs.readFileSync(attributesPath, "utf8").split(/\r?\n/)) {
    const line = stripInlineComment(raw).trim();
    if (!line.startsWith("db ") || !raw.includes("base reward")) {
      continue;
    }
    const token = line.slice(3).trim().split(/[,\s]+/)[0];
    if (token) {
      rewards.push(parseAsmNumber(token));
    }
  }

  if (classOrder.length !== rewards.length) {
    throw new Error(
      `Parsed trainer class rewards do not match trainer class count: ${rewards.length} != ${classOrder.length}`
    );
  }

  return Object.fromEntries(classOrder.map((trainerClass, index) => [trainerClass, rewards[index]]));
}

export function parseTrainers(filePath: string, pokemonSpeciesMap: Record<string, PokemonSpecies>): Trainer[] {
  const movesMap = parseMoves(path.join(getDisassemblyRoot(), "data", "moves", "moves.asm"));
  const trainers: Trainer[] = [];
  const lines = fs.readFileSync(filePath, "utf8").split(/\r?\n/);

  let rawName: string | null = null;
  let trainerType: string | null = null;
  let pokemonBlock: string[] = [];

  const flushTrainer = () => {
    if (!rawName || !trainerType) {
      rawName = null;
      trainerType = null;
      pokemonBlock = [];
      return;
    }
    const party: Trainer["party"] = [];

    for (const rawLine of pokemonBlock) {
      const line = stripInlineComment(rawLine).trim();
      if (!line.startsWith("db")) continue;
      const parts = line.split(",").map((part) => part.trim());
      const levelText = parts[0].replace("db", "").trim();
      if (!levelText || levelText === "-1") continue;
      const level = Number.parseInt(levelText, 10);
      const speciesName = parts[1];
      const species = pokemonSpeciesMap[speciesName];
      if (!species) throw new Error(`Could not find species data for ${speciesName}`);
      let item: string | null | undefined = null;
      let moves: Array<{ name: Trainer["party"][number]["moves"][number]["name"]; current_pp: number; pp_ups: number }> = [];
      if (trainerType === "TRAINERTYPE_MOVES") {
        moves = parts.slice(2).filter((move) => move !== "NO_MOVE").map((move) => ({
          name: move as Trainer["party"][number]["moves"][number]["name"],
          current_pp: movesMap[move].pp,
          pp_ups: 0,
        }));
      } else if (trainerType === "TRAINERTYPE_ITEM") {
        item = parts[2] === "NO_ITEM" ? null : parts[2];
      } else if (trainerType === "TRAINERTYPE_ITEM_MOVES") {
        item = parts[2] === "NO_ITEM" ? null : parts[2];
        moves = parts.slice(3).filter((move) => move !== "NO_MOVE").map((move) => ({
          name: move as Trainer["party"][number]["moves"][number]["name"],
          current_pp: movesMap[move].pp,
          pp_ups: 0,
        }));
      }

      party.push({
        species,
        nickname: pokemonSpeciesDisplayName(species),
        level,
        item,
        moves,
        hp: species.base_stats.hp,
        max_hp: species.base_stats.hp,
        original_trainer_name: "Trainer",
        original_trainer_id: 0,
        experience: 0,
        happiness: 0,
        dvs: { attack: 0, defense: 0, speed: 0, special: 0, hp: 0 },
        sleep_turns: 0,
        flinching: false,
        rampage_turns: 0,
        confusion_turns: 0,
        perish_song_turns: 0,
        focus_energy: false,
        hp_exp: 0,
        attack_exp: 0,
        defense_exp: 0,
        speed_exp: 0,
        special_exp: 0,
        turns_in_battle: 0,
        stat_boosts: {
          HP: 0,
          ATTACK: 0,
          DEFENSE: 0,
          SPEED: 0,
          SPECIAL_ATTACK: 0,
          SPECIAL_DEFENSE: 0,
          ACCURACY: 0,
          EVASION: 0,
        },
        locked_turns_remaining: 0,
        trapped_turns: 0,
        leech_seeded: false,
        nightmare: false,
        cursed: false,
        attack: 0,
        defense: 0,
        speed: 0,
        special_attack: 0,
        special_defense: 0,
        disable_turns: 0,
        encore_turns_remaining: 0,
        destiny_bond_active: false,
        pokerus: false,
        rage_active: false,
        rage_counter: 0,
        fury_cutter_count: 0,
        rollout_step: 0,
        rollout_active: false,
        defense_curled: false,
        cant_run: false,
        bide_active: false,
        bide_turns_remaining: 0,
        bide_damage: 0,
        protect_active: false,
        protect_counter: 0,
        endure_active: false,
        endure_counter: 0,
        foresight_active: false,
        lock_on_active: false,
        substitute_hp: 0,
        transformed: false,
        last_damage_taken: 0,
      });
    }

    trainers.push({
      name: cleanTrainerName(rawName),
      trainer_id: "",
      trainer_class: "",
      party,
      win_quote: `${cleanTrainerName(rawName)}: I won!`,
      lose_quote: `${cleanTrainerName(rawName)}: I lost!`,
      items: [],
      base_reward: 0,
      ai_move_flags: 0,
      ai_item_switch_flags: 0,
      encounter_music: "",
      ai_layers: [],
    });
    rawName = null;
    trainerType = null;
    pokemonBlock = [];
  };

  for (const rawLine of lines) {
    const line = stripInlineComment(rawLine).trim();
    if (!line) {
      continue;
    }
    const headerMatch = line.match(/^db "([^"]+)",\s*(TRAINERTYPE_\w+)$/);
    if (headerMatch) {
      flushTrainer();
      rawName = headerMatch[1];
      trainerType = headerMatch[2];
      continue;
    }
    if (rawName && trainerType) {
      pokemonBlock.push(rawLine);
      if (/^db\s+-1\b/.test(line)) {
        flushTrainer();
      }
    }
  }
  flushTrainer();

  const metadata = parseTrainerMetadata();
  const classBaseRewards = parseTrainerClassBaseRewards();
  if (metadata.length !== trainers.length) {
    throw new Error(`Parsed trainer count does not match ASM trainer metadata count: ${trainers.length} != ${metadata.length}`);
  }
  for (let index = 0; index < trainers.length; index += 1) {
    trainers[index].trainer_class = metadata[index][0];
    trainers[index].trainer_id = metadata[index][1];
    const encounterMusic = TRAINER_ENCOUNTER_MUSIC[trainers[index].trainer_class];
    if (!encounterMusic) {
      throw new Error(`Missing trainer encounter music for class ${trainers[index].trainer_class}`);
    }
    trainers[index].encounter_music = encounterMusic;
    const baseReward = classBaseRewards[trainers[index].trainer_class];
    if (!baseReward) {
      throw new Error(`Missing trainer base reward for class ${trainers[index].trainer_class}`);
    }
    trainers[index].base_reward = baseReward;
  }
  return trainers;
}

export function exportTrainers(pokemonData: PokemonSpecies[]): Trainer[] {
  if (!pokemonData.length) {
    throw new Error("exportTrainers requires explicit pokemonData from the current core export.");
  }
  const trainerPath = path.join(getDisassemblyRoot(), "data", "trainers", "parties.asm");
  const pokemonSpeciesMap = Object.fromEntries(pokemonData.map((pokemon) => [pokemon.id, pokemon]));
  const trainers = parseTrainers(trainerPath, pokemonSpeciesMap);
  writeJsonToTargets("trainers.json", trainers, { indent: 2 });
  return trainers;
}
