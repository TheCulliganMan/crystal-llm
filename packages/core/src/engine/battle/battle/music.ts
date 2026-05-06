// ASM mapping: pokecrystal_disassembly/engine/battle/start_battle.asm (PlayBattleMusic).
import { Region } from "@pokecrystal/core/core/constants";
import { isInJohto } from "@pokecrystal/core/core/home";
import { GameState } from "@pokecrystal/core/core/state";

const CHAMPION_CLASSES = new Set(["CHAMPION", "RED"]);
const ELITE_FOUR_CLASSES = new Set(["WILL", "BRUNO", "KAREN", "KOGA"]);
const ROCKET_GRUNTS = new Set(["GRUNTM", "GRUNTF"]);
const KANTO_GYM_LEADERS = new Set([
  "BROCK",
  "MISTY",
  "LT_SURGE",
  "ERIKA",
  "JANINE",
  "SABRINA",
  "BLAINE",
  "BLUE",
]);
const JOHTO_GYM_LEADERS = new Set([
  "FALKNER",
  "BUGSY",
  "WHITNEY",
  "MORTY",
  "CHUCK",
  "JASMINE",
  "PRYCE",
  "CLAIR",
]);
const RIVAL2_INDIGO_IDS = new Set([
  "RIVAL2_2_CHIKORITA",
  "RIVAL2_2_CYNDAQUIL",
  "RIVAL2_2_TOTODILE",
]);
const SUICUNE_BATTLE_TYPES = new Set(["BATTLETYPE_SUICUNE", "BATTLETYPE_ROAMING"]);
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

export function determineBattleMusic(gameState: GameState): string {
  const wram = gameState.wram;
  const battleType = normalise(wram.battle_type ?? "BATTLETYPE_NORMAL");
  if (SUICUNE_BATTLE_TYPES.has(battleType)) {
    return "MUSIC_SUICUNE_BATTLE";
  }

  const trainerClass = normalise(wram.other_trainer_class ?? "");
  if (!trainerClass) {
    return wildBattleMusic(gameState);
  }
  const trainerId = normalise(wram.other_trainer_id ?? "");
  return trainerBattleMusic(trainerClass, gameState, trainerId);
}

export function determineTrainerEncounterMusic(trainerClass: string | null | undefined): string {
  const normalized = normalise(trainerClass ?? "");
  return TRAINER_ENCOUNTER_MUSIC[normalized] ?? "MUSIC_HIKER_ENCOUNTER";
}

function wildBattleMusic(gameState: GameState): string {
  const region = isInJohto(gameState);
  if (region === Region.KANTO) {
    return "MUSIC_KANTO_WILD_BATTLE";
  }

  const timeOfDay = normalise(gameState.wram.time_of_day ?? "");
  if (timeOfDay === "NITE" || timeOfDay === "NIGHT") {
    return "MUSIC_JOHTO_WILD_BATTLE_NIGHT";
  }
  return "MUSIC_JOHTO_WILD_BATTLE";
}

function trainerBattleMusic(trainerClass: string, gameState: GameState, trainerId: string): string {
  if (CHAMPION_CLASSES.has(trainerClass)) {
    return "MUSIC_CHAMPION_BATTLE";
  }
  if (ROCKET_GRUNTS.has(trainerClass)) {
    return "MUSIC_ROCKET_BATTLE";
  }
  if (KANTO_GYM_LEADERS.has(trainerClass)) {
    return "MUSIC_KANTO_GYM_LEADER_BATTLE";
  }
  if (JOHTO_GYM_LEADERS.has(trainerClass) || ELITE_FOUR_CLASSES.has(trainerClass)) {
    return "MUSIC_JOHTO_GYM_LEADER_BATTLE";
  }
  if (trainerClass === "RIVAL1") {
    return "MUSIC_RIVAL_BATTLE";
  }
  if (trainerClass === "RIVAL2") {
    if (RIVAL2_INDIGO_IDS.has(trainerId)) {
      return "MUSIC_CHAMPION_BATTLE";
    }
    return "MUSIC_RIVAL_BATTLE";
  }

  const region = isInJohto(gameState);
  return region === Region.JOHTO
    ? "MUSIC_JOHTO_TRAINER_BATTLE"
    : "MUSIC_KANTO_TRAINER_BATTLE";
}

export function determineVictoryMusic(trainerClass: string | null, trainerBattle: boolean): string {
  if (!trainerBattle) {
    return "MUSIC_WILD_VICTORY";
  }
  if (isGymLeaderClass(trainerClass)) {
    return "MUSIC_GYM_VICTORY";
  }
  return "MUSIC_TRAINER_VICTORY";
}

export function isGymLeaderClass(trainerClass: string | null): boolean {
  if (!trainerClass) {
    return false;
  }
  return gymVictoryClassNames().has(normalise(trainerClass));
}

export function registerTrainerClasses(options: { johto?: Iterable<string> | null; kanto?: Iterable<string> | null }): void {
  if (options.johto) {
    for (const entry of options.johto) {
      JOHTO_GYM_LEADERS.add(normalise(entry));
    }
  }
  if (options.kanto) {
    for (const entry of options.kanto) {
      KANTO_GYM_LEADERS.add(normalise(entry));
    }
  }
}

function normalise(value: unknown): string {
  return String(value ?? "").trim().toUpperCase();
}

function gymVictoryClassNames(): Set<string> {
  const names = new Set<string>();
  for (const entry of JOHTO_GYM_LEADERS) {
    names.add(entry);
  }
  for (const entry of KANTO_GYM_LEADERS) {
    names.add(entry);
  }
  for (const entry of ELITE_FOUR_CLASSES) {
    names.add(entry);
  }
  for (const entry of CHAMPION_CLASSES) {
    names.add(entry);
  }
  return names;
}
