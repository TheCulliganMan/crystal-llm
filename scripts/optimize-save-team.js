#!/usr/bin/env node

const childProcess = require("node:child_process");
const fs = require("node:fs");
const path = require("node:path");

const PARTY_SIZE = 6;

const MOVE_PP = {
  AEROBLAST: 5,
  BODY_SLAM: 15,
  CRUNCH: 15,
  CURSE: 10,
  CUT: 30,
  EARTHQUAKE: 10,
  FIRE_BLAST: 5,
  FLY: 15,
  ICE_BEAM: 10,
  PSYCHIC_M: 10,
  RECOVER: 20,
  REST: 10,
  ROCK_SLIDE: 10,
  SACRED_FIRE: 5,
  STRENGTH: 15,
  SURF: 15,
  THUNDERBOLT: 15,
  WATERFALL: 15,
  WHIRLPOOL: 15,
};

const TEAM = [
  {
    species: "MEWTWO",
    nickname: "MEWTWO",
    item: "LEFTOVERS",
    moves: ["PSYCHIC_M", "THUNDERBOLT", "ICE_BEAM", "RECOVER"],
  },
  {
    species: "LUGIA",
    nickname: "LUGIA",
    item: "LEFTOVERS",
    moves: ["AEROBLAST", "SURF", "FLY", "RECOVER"],
  },
  {
    species: "HO_OH",
    nickname: "HO-OH",
    item: "LEFTOVERS",
    moves: ["SACRED_FIRE", "EARTHQUAKE", "THUNDERBOLT", "RECOVER"],
  },
  {
    species: "SNORLAX",
    nickname: "SNORLAX",
    item: "LEFTOVERS",
    moves: ["CURSE", "BODY_SLAM", "EARTHQUAKE", "REST"],
  },
  {
    species: "TYRANITAR",
    nickname: "TYRANITAR",
    item: "LEFTOVERS",
    moves: ["ROCK_SLIDE", "CRUNCH", "EARTHQUAKE", "FIRE_BLAST"],
  },
  {
    species: "MEW",
    nickname: "MEW",
    item: "LEFTOVERS",
    moves: ["CUT", "STRENGTH", "WHIRLPOOL", "WATERFALL"],
  },
];

const FALLBACK_SPECIES = {
  MEWTWO: {
    id: "MEWTWO",
    int_id: 150,
    base_stats: {
      hp: 106,
      attack: 110,
      defense: 90,
      speed: 130,
      special_attack: 154,
      special_defense: 90,
    },
    type1: "PSYCHIC_TYPE",
    type2: "PSYCHIC_TYPE",
    catch_rate: 3,
    base_exp: 220,
    item1: null,
    item2: "BERSERK_GENE",
    gender_ratio: 255,
    unknown1: 0,
    step_cycles_to_hatch: 120,
    unknown2: 0,
    growth_rate: "GROWTH_SLOW",
    egg_group1: "EGG_NONE",
    egg_group2: "EGG_NONE",
  },
  MEW: {
    id: "MEW",
    int_id: 151,
    base_stats: {
      hp: 100,
      attack: 100,
      defense: 100,
      speed: 100,
      special_attack: 100,
      special_defense: 100,
    },
    type1: "PSYCHIC_TYPE",
    type2: "PSYCHIC_TYPE",
    catch_rate: 45,
    base_exp: 64,
    item1: null,
    item2: "MIRACLEBERRY",
    gender_ratio: 255,
    unknown1: 0,
    step_cycles_to_hatch: 120,
    unknown2: 0,
    growth_rate: "GROWTH_MEDIUM_SLOW",
    egg_group1: "EGG_NONE",
    egg_group2: "EGG_NONE",
  },
  SNORLAX: {
    id: "SNORLAX",
    int_id: 143,
    base_stats: {
      hp: 160,
      attack: 110,
      defense: 65,
      speed: 30,
      special_attack: 65,
      special_defense: 110,
    },
    type1: "NORMAL",
    type2: "NORMAL",
    catch_rate: 25,
    base_exp: 154,
    item1: "LEFTOVERS",
    item2: "LEFTOVERS",
    gender_ratio: 31,
    unknown1: 0,
    step_cycles_to_hatch: 40,
    unknown2: 0,
    growth_rate: "GROWTH_SLOW",
    egg_group1: "EGG_MONSTER",
    egg_group2: "EGG_MONSTER",
  },
  TYRANITAR: {
    id: "TYRANITAR",
    int_id: 248,
    base_stats: {
      hp: 100,
      attack: 134,
      defense: 110,
      speed: 61,
      special_attack: 95,
      special_defense: 100,
    },
    type1: "ROCK",
    type2: "DARK",
    catch_rate: 45,
    base_exp: 218,
    item1: null,
    item2: null,
    gender_ratio: 127,
    unknown1: 0,
    step_cycles_to_hatch: 40,
    unknown2: 0,
    growth_rate: "GROWTH_SLOW",
    egg_group1: "EGG_MONSTER",
    egg_group2: "EGG_MONSTER",
  },
  LUGIA: {
    id: "LUGIA",
    int_id: 249,
    base_stats: {
      hp: 106,
      attack: 90,
      defense: 130,
      speed: 110,
      special_attack: 90,
      special_defense: 154,
    },
    type1: "PSYCHIC_TYPE",
    type2: "FLYING",
    catch_rate: 3,
    base_exp: 220,
    item1: null,
    item2: null,
    gender_ratio: 255,
    unknown1: 0,
    step_cycles_to_hatch: 120,
    unknown2: 0,
    growth_rate: "GROWTH_SLOW",
    egg_group1: "EGG_NONE",
    egg_group2: "EGG_NONE",
  },
  HO_OH: {
    id: "HO_OH",
    int_id: 250,
    base_stats: {
      hp: 106,
      attack: 130,
      defense: 90,
      speed: 90,
      special_attack: 110,
      special_defense: 154,
    },
    type1: "FIRE",
    type2: "FLYING",
    catch_rate: 3,
    base_exp: 220,
    item1: "SACRED_ASH",
    item2: "SACRED_ASH",
    gender_ratio: 255,
    unknown1: 0,
    step_cycles_to_hatch: 120,
    unknown2: 0,
    growth_rate: "GROWTH_SLOW",
    egg_group1: "EGG_NONE",
    egg_group2: "EGG_NONE",
  },
};

const usage = () => `
Usage:
  node scripts/optimize-save-team.js [--session-id testrun] [--save PATH] [--kill-running]

Options:
  --session-id ID    MCP session id. Default: testrun.
  --save PATH        Exact autosave path. Default: ./mcp-<session-id>-autosave.sav.
  --kill-running     Stop a same-session play process before editing.
  --no-runtime-backup
                     Skip backing up ./mcp-<session-id>-runtime.json.
`.trim();

const parseArgs = (argv) => {
  const values = {
    sessionId: "testrun",
    savePath: null,
    killRunning: false,
    runtimeBackup: true,
  };

  for (let index = 0; index < argv.length; index += 1) {
    const token = argv[index];
    if (token === "--help" || token === "-h") {
      process.stdout.write(`${usage()}\n`);
      process.exit(0);
    }
    if (token === "--kill-running") {
      values.killRunning = true;
      continue;
    }
    if (token === "--no-runtime-backup") {
      values.runtimeBackup = false;
      continue;
    }
    if (token === "--session-id") {
      values.sessionId = argv[index + 1];
      index += 1;
      continue;
    }
    if (token === "--save") {
      values.savePath = argv[index + 1];
      index += 1;
      continue;
    }
    throw new Error(`Unknown argument: ${token}\n\n${usage()}`);
  }

  if (!values.sessionId || values.sessionId.startsWith("--")) {
    throw new Error("--session-id requires a value.");
  }
  if (values.savePath?.startsWith("--")) {
    throw new Error("--save requires a value.");
  }

  return values;
};

const timestamp = () => {
  const date = new Date();
  const pad = (value) => String(value).padStart(2, "0");
  return [
    date.getFullYear(),
    pad(date.getMonth() + 1),
    pad(date.getDate()),
    "-",
    pad(date.getHours()),
    pad(date.getMinutes()),
    pad(date.getSeconds()),
  ].join("");
};

const backupFile = (filePath, suffix) => {
  if (!fs.existsSync(filePath)) {
    return null;
  }
  const backupPath = `${filePath}.${suffix}.bak`;
  fs.copyFileSync(filePath, backupPath);
  return backupPath;
};

const sameSessionPlayProcesses = (sessionId) => {
  const output = childProcess
    .execFileSync("ps", ["ax", "-o", "pid=,command="], { encoding: "utf8" })
    .trim();
  if (!output) {
    return [];
  }
  return output
    .split(/\n/)
    .map((line) => line.match(/^\s*(\d+)\s+(.+)$/))
    .filter(Boolean)
    .map((match) => ({ pid: Number(match[1]), command: match[2] }))
    .filter(
      (entry) =>
        entry.pid !== process.pid &&
        entry.command.includes("pokecrystal-cli") &&
        entry.command.includes("play") &&
        entry.command.includes("--session-id") &&
        entry.command.includes(sessionId)
    );
};

const stopRunningSession = (sessionId, shouldKill) => {
  const running = sameSessionPlayProcesses(sessionId);
  if (running.length === 0) {
    return [];
  }
  if (!shouldKill) {
    const pids = running.map((entry) => entry.pid).join(", ");
    throw new Error(
      `Session ${sessionId} is currently running as PID(s) ${pids}. Re-run with --kill-running or quit the play session first.`
    );
  }
  for (const entry of running) {
    process.kill(entry.pid, "SIGTERM");
  }
  return running;
};

const loadSpeciesData = (repoRoot) => {
  const distPath = path.join(repoRoot, "packages", "core", "dist", "assets", "src", "data", "pokemon-data.json");
  if (fs.existsSync(distPath)) {
    const data = JSON.parse(fs.readFileSync(distPath, "utf8"));
    if (Array.isArray(data)) {
      return Object.fromEntries(data.map((species) => [species.id, species]));
    }
  }
  return FALLBACK_SPECIES;
};

const completeSpecies = (species) => ({
  evolutions: null,
  tmhm_learnset: [],
  ability: "NONE",
  pic_size: 0,
  front_pic: 0,
  back_pic: 0,
  weight: 0,
  ...species,
});

const growthExperience = (growthRate, level) => {
  switch (growthRate) {
    case "GROWTH_SLOW":
      return Math.floor((5 * level ** 3) / 4);
    case "GROWTH_MEDIUM_SLOW":
      return Math.floor((6 * level ** 3) / 5 - 15 * level ** 2 + 100 * level - 140);
    case "GROWTH_FAST":
      return Math.floor((4 * level ** 3) / 5);
    case "GROWTH_MEDIUM_FAST":
    default:
      return level ** 3;
  }
};

const calculateStats = (species, level) => {
  const dv = 15;
  const statExpBonus = Math.floor(Math.floor(Math.sqrt(65535)) / 4);
  const stat = (base) => Math.floor((((base + dv) * 2 + statExpBonus) * level) / 100) + 5;
  return {
    maxHp: Math.floor((((species.base_stats.hp + dv) * 2 + statExpBonus) * level) / 100) + level + 10,
    attack: stat(species.base_stats.attack),
    defense: stat(species.base_stats.defense),
    speed: stat(species.base_stats.speed),
    specialAttack: stat(species.base_stats.special_attack),
    specialDefense: stat(species.base_stats.special_defense),
  };
};

const makePokemon = (state, speciesById, plan) => {
  const species = completeSpecies(speciesById[plan.species]);
  if (!species?.base_stats) {
    throw new Error(`Missing species data for ${plan.species}.`);
  }

  const level = 60;
  const stats = calculateStats(species, level);
  return {
    species,
    nickname: plan.nickname,
    item: plan.item,
    moves: plan.moves.map((name) => ({ name, current_pp: MOVE_PP[name] ?? 0 })),
    level,
    hp: stats.maxHp,
    max_hp: stats.maxHp,
    dvs: { attack: 15, defense: 15, speed: 15, special: 15, hp: 15 },
    sleep_turns: 0,
    flinching: false,
    rampage_turns: 0,
    confusion_turns: 0,
    perish_song_turns: 0,
    focus_energy: false,
    original_trainer_name: state.sram?.player_name || "PLAYER",
    original_trainer_id: state.sram?.player_id || 0,
    experience: growthExperience(species.growth_rate, level),
    hp_exp: 65535,
    attack_exp: 65535,
    defense_exp: 65535,
    speed_exp: 65535,
    special_exp: 65535,
    happiness: 255,
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
    attack: stats.attack,
    defense: stats.defense,
    speed: stats.speed,
    special_attack: stats.specialAttack,
    special_defense: stats.specialDefense,
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
  };
};

const main = () => {
  const repoRoot = process.cwd();
  const options = parseArgs(process.argv.slice(2));
  const savePath = path.resolve(
    repoRoot,
    options.savePath ?? `mcp-${options.sessionId}-autosave.sav`
  );
  const metaPath = `${savePath}.meta.json`;
  const runtimePath = path.resolve(repoRoot, `mcp-${options.sessionId}-runtime.json`);

  const stopped = stopRunningSession(options.sessionId, options.killRunning);

  if (!fs.existsSync(savePath)) {
    throw new Error(`Save file not found: ${savePath}`);
  }

  const suffix = `pre-optimized-team-${timestamp()}`;
  const backups = [
    backupFile(savePath, suffix),
    backupFile(metaPath, suffix),
    options.runtimeBackup ? backupFile(runtimePath, suffix) : null,
  ].filter(Boolean);

  const state = JSON.parse(fs.readFileSync(savePath, "utf8"));
  if (!state.sram || !state.wram) {
    throw new Error(`Save file does not look like a PokeCrystal game-state snapshot: ${savePath}`);
  }

  const speciesById = loadSpeciesData(repoRoot);
  const party = TEAM.map((plan) => makePokemon(state, speciesById, plan));
  if (party.length !== PARTY_SIZE) {
    throw new Error(`Expected ${PARTY_SIZE} Pokemon, received ${party.length}.`);
  }

  state.sram.party = { pokemon: party };
  state.wram.wPartyCount = party.length;
  state.wram.wCurPartyMon = Math.min(state.wram.wCurPartyMon ?? 0, party.length - 1);
  state.wram.wCurPartySpecies = party[state.wram.wCurPartyMon]?.species.id ?? party[0].species.id;
  delete state.wram.wTempMon;

  fs.writeFileSync(savePath, `${JSON.stringify(state, null, 4)}\n`);
  fs.writeFileSync(metaPath, `${JSON.stringify({ saved_at: new Date().toISOString() }, null, 4)}\n`);

  const summary = {
    sessionId: options.sessionId,
    savePath,
    stoppedPids: stopped.map((entry) => entry.pid),
    backups,
    party: party.map((pokemon) => ({
      species: pokemon.species.id,
      level: pokemon.level,
      item: pokemon.item,
      hp: `${pokemon.hp}/${pokemon.max_hp}`,
      moves: pokemon.moves.map((move) => `${move.name}:${move.current_pp}`),
    })),
  };
  process.stdout.write(`${JSON.stringify(summary, null, 2)}\n`);
};

try {
  main();
} catch (error) {
  const message = error instanceof Error ? error.message : String(error);
  process.stderr.write(`${message}\n`);
  process.exit(1);
}
