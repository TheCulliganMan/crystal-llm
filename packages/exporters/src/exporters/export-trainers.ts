import fs from "fs";
import path from "path";
import type { Trainer } from "@pokecrystal/core/core/models/trainer";
import { pokemonSpeciesDisplayName, type PokemonSpecies } from "@pokecrystal/core/core/models/pokemon";
import { getDisassemblyRoot } from "@pokecrystal/core/core/paths";
import { stripInlineComment, writeJsonToTargets } from "./asm-utils";
import { parseMoves } from "./export-data";

const RIVAL_NAME_PLACEHOLDER = "<RIVAL>";

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
      let moves: Array<{ name: Trainer["party"][number]["moves"][number]["name"]; current_pp: number }> = [];
      if (trainerType === "TRAINERTYPE_MOVES") {
        moves = parts.slice(2).filter((move) => move !== "NO_MOVE").map((move) => ({
          name: move as Trainer["party"][number]["moves"][number]["name"],
          current_pp: movesMap[move].pp,
        }));
      } else if (trainerType === "TRAINERTYPE_ITEM") {
        item = parts[2];
      } else if (trainerType === "TRAINERTYPE_ITEM_MOVES") {
        item = parts[2];
        moves = parts.slice(3).filter((move) => move !== "NO_MOVE").map((move) => ({
          name: move as Trainer["party"][number]["moves"][number]["name"],
          current_pp: movesMap[move].pp,
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
  if (metadata.length !== trainers.length) {
    throw new Error(`Parsed trainer count does not match ASM trainer metadata count: ${trainers.length} != ${metadata.length}`);
  }
  for (let index = 0; index < trainers.length; index += 1) {
    trainers[index].trainer_class = metadata[index][0];
    trainers[index].trainer_id = metadata[index][1];
  }
  return trainers;
}

export function exportTrainers(pokemonData?: PokemonSpecies[]): Trainer[] {
  const trainerPath = path.join(getDisassemblyRoot(), "data", "trainers", "parties.asm");
  const pokemonList = pokemonData ?? JSON.parse(fs.readFileSync(path.join(path.dirname(path.dirname(getDisassemblyRoot())), "src", "pokecrystal-ts", "assets", "data", "pokemon_data.json"), "utf8"));
  const pokemonSpeciesMap = Object.fromEntries((pokemonList as PokemonSpecies[]).map((pokemon) => [pokemon.id, pokemon]));
  const trainers = parseTrainers(trainerPath, pokemonSpeciesMap);
  writeJsonToTargets("trainers.json", trainers, { indent: 2 });
  return trainers;
}
