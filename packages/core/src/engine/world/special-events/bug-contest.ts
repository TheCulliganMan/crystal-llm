import { GameState } from "@pokecrystal/core/core/state";
import { HardwareRNG } from "@pokecrystal/core/engine/games/rng";
import { TimeSystem } from "@pokecrystal/core/engine/systems/time";
import { Event } from "@pokecrystal/core/engine/world/events";
import { MAX_PC_BOXES } from "@pokecrystal/core/core/constants";
import { BoxSchema, addPokemon as addBoxPokemon, formatDefaultBoxName, type Box } from "@pokecrystal/core/core/models/box";
import { addPokemon as addPartyPokemon } from "@pokecrystal/core/core/models/party";
import {
  BugContestResults as BugContestResultsSchema,
  type BugContestResults,
} from "@pokecrystal/core/core/models/bug-contest";
import type { Pokemon } from "@pokecrystal/core/core/models";
import { ensureRunnerVariables, ScriptRunner } from "./utils";
import type { EventManager } from "@pokecrystal/core/engine/events/events";

type BugContestOverworldContext = object & {
  script_runner?: ScriptRunner | null;
  event_manager?: EventManager | null;
};

type BugContestContext = Record<string, unknown> & {
  runner?: ScriptRunner | null;
  overworld?: BugContestOverworldContext | null;
  event_manager?: EventManager | null;
};

type BugContestWinner = {
  winner_id: number;
  mon_species: string;
  score: number;
};

type ContestantRecord = {
  trainer_class: string;
  trainer_name: string;
  placements: [
    [string, number],
    [string, number],
    [string, number]
  ];
};

type WildContestEntry = {
  percent: number;
  species: string;
  min_level: number;
  max_level: number;
};

export const BUG_CONTESTANT_FLAGS = Array.from({ length: 10 }, (_, index) =>
  `EVENT_BUG_CATCHING_CONTESTANT_${index + 1}A`
);
export const BUG_CONTEST_BALLS = 20;
export const BUG_CONTEST_MINUTES = 20;
export const BUG_CONTEST_SECONDS = 0;
export const BUG_CONTEST_PLAYER_ID = 1;

export const BUGCONTEST_CAUGHT_MON = 0;
export const BUGCONTEST_BOXED_MON = 1;
export const BUGCONTEST_NO_CATCH = 2;
export const BUGCONTEST_DECISION_PENDING = 3;

const CONTEST_DAY_WRAP = 20 * 7;

const CONTESTANTS: ContestantRecord[] = [
  {
    trainer_class: "BUG CATCHER",
    trainer_name: "DON",
    placements: [
      ["KAKUNA", 300],
      ["METAPOD", 285],
      ["CATERPIE", 226],
    ],
  },
  {
    trainer_class: "BUG CATCHER",
    trainer_name: "ED",
    placements: [
      ["BUTTERFREE", 286],
      ["BUTTERFREE", 251],
      ["CATERPIE", 237],
    ],
  },
  {
    trainer_class: "COOLTRAINER",
    trainer_name: "NICK",
    placements: [
      ["SCYTHER", 357],
      ["BUTTERFREE", 349],
      ["PINSIR", 368],
    ],
  },
  {
    trainer_class: "POKEFAN",
    trainer_name: "WILLIAM",
    placements: [
      ["PINSIR", 332],
      ["BUTTERFREE", 324],
      ["VENONAT", 321],
    ],
  },
  {
    trainer_class: "BUG CATCHER",
    trainer_name: "BENNY",
    placements: [
      ["BUTTERFREE", 318],
      ["WEEDLE", 295],
      ["CATERPIE", 285],
    ],
  },
  {
    trainer_class: "CAMPER",
    trainer_name: "BARRY",
    placements: [
      ["PINSIR", 366],
      ["VENONAT", 329],
      ["KAKUNA", 314],
    ],
  },
  {
    trainer_class: "PICNICKER",
    trainer_name: "CINDY",
    placements: [
      ["BUTTERFREE", 341],
      ["METAPOD", 301],
      ["CATERPIE", 264],
    ],
  },
  {
    trainer_class: "BUG CATCHER",
    trainer_name: "JOSH",
    placements: [
      ["SCYTHER", 326],
      ["BUTTERFREE", 292],
      ["METAPOD", 282],
    ],
  },
  {
    trainer_class: "YOUNGSTER",
    trainer_name: "SAMUEL",
    placements: [
      ["WEEDLE", 270],
      ["PINSIR", 282],
      ["CATERPIE", 251],
    ],
  },
  {
    trainer_class: "SCHOOLBOY",
    trainer_name: "KIPP",
    placements: [
      ["VENONAT", 267],
      ["PARAS", 254],
      ["KAKUNA", 259],
    ],
  },
];

const CONTEST_WILD_TABLE: WildContestEntry[] = [
  { percent: 20, species: "CATERPIE", min_level: 7, max_level: 18 },
  { percent: 20, species: "WEEDLE", min_level: 7, max_level: 18 },
  { percent: 10, species: "METAPOD", min_level: 9, max_level: 18 },
  { percent: 10, species: "KAKUNA", min_level: 9, max_level: 18 },
  { percent: 5, species: "BUTTERFREE", min_level: 12, max_level: 15 },
  { percent: 5, species: "BEEDRILL", min_level: 12, max_level: 15 },
  { percent: 10, species: "VENONAT", min_level: 10, max_level: 16 },
  { percent: 10, species: "PARAS", min_level: 10, max_level: 17 },
  { percent: 5, species: "SCYTHER", min_level: 13, max_level: 14 },
  { percent: 5, species: "PINSIR", min_level: 13, max_level: 14 },
  { percent: -1, species: "VENOMOTH", min_level: 30, max_level: 40 },
];

const currentContestTime = (game_state: GameState): number[] => {
  const wram = game_state.wram;
  const hram = game_state.hram;
  return [
    Number(wram.wCurDay ?? 0) % 256,
    Number(hram.hHours ?? 0) % 24,
    Number(hram.hMinutes ?? 0) % 60,
    Number(hram.hSeconds ?? 0) % 60,
  ];
};

const formatContestSpecies = (species: string | null | undefined): string => {
  const token = String(species ?? "POKEMON").trim();
  const base = token.length ? token : "POKEMON";
  return base.replace(/_/g, " ").toUpperCase();
};

const elapsedSince = (startTime: number[], currentTime: number[]): [number, number, number, number] => {
  let [startDay, startHour, startMinute, startSecond] = startTime;
  let [currentDay, currentHour, currentMinute, currentSecond] = currentTime;

  startDay = Number(startDay);
  startHour = Number(startHour);
  startMinute = Number(startMinute);
  startSecond = Number(startSecond);

  currentDay = Number(currentDay);
  currentHour = Number(currentHour);
  currentMinute = Number(currentMinute);
  currentSecond = Number(currentSecond);

  let secondsSince = currentSecond - startSecond;
  let borrow = 0;
  if (secondsSince < 0) {
    secondsSince += 60;
    borrow = 1;
  }

  let minutesSince = currentMinute - startMinute - borrow;
  borrow = 0;
  if (minutesSince < 0) {
    minutesSince += 60;
    borrow = 1;
  }

  let hoursSince = currentHour - startHour - borrow;
  borrow = 0;
  if (hoursSince < 0) {
    hoursSince += 24;
    borrow = 1;
  }

  let daysSince = currentDay - startDay - borrow;
  if (daysSince < 0) {
    daysSince += CONTEST_DAY_WRAP;
  }

  return [daysSince, hoursSince, minutesSince, secondsSince];
};

const ensureContestResults = (game_state: GameState): BugContestResults => {
  let results = game_state.wram.bug_contest_results;
  if (!results || !("first_place" in results)) {
    results = BugContestResultsSchema.parse({});
    game_state.wram.bug_contest_results = results;
  }
  return results;
};

const resolveBugContestContext = (
  context: BugContestContext = {},
): Required<Pick<BugContestContext, "runner" | "overworld" | "event_manager">> => {
  const overworld = context.overworld ?? null;
  const runner = context.runner ?? overworld?.script_runner ?? null;
  const event_manager = context.event_manager ?? overworld?.event_manager ?? runner?.event_manager ?? null;
  return { runner, overworld, event_manager };
};

const storeRunnerResult = (
  runner: ScriptRunner | null | undefined,
  value: unknown,
  condition: boolean,
): void => {
  if (!runner) {
    return;
  }
  runner.last_value = value;
  const variables = ensureRunnerVariables(runner);
  variables._value = value;
  runner.last_condition_result = condition;
};

export function give_park_balls(
  game_state: GameState,
  context: BugContestContext = {}
): number {
  // ASM: engine/events/bug_contest/contest.asm::GiveParkBalls
  const { runner, overworld, event_manager } = resolveBugContestContext(context);
  void overworld;
  void event_manager;

  const contest = game_state.wram.bug_contest_state;
  contest.park_balls_remaining = BUG_CONTEST_BALLS;
  contest.caught_species = undefined;
  contest.caught_level = undefined;
  contest.party_backup = [];
  contest.pending_caught_mon = undefined;
  contest.timer_active = true;
  game_state.wram.bug_contest_caught_mon = undefined;
  resetContestResults(game_state);
  const flags = game_state.wram.engine_flags as Record<string, boolean>;
  flags["ENGINE_BUG_CONTEST_TIMER"] = true;

  start_bug_contest_timer(game_state, { runner, overworld, event_manager });

  storeRunnerResult(runner, BUG_CONTEST_BALLS, true);
  return BUG_CONTEST_BALLS;
}

export function select_random_bug_contest_contestants(
  game_state: GameState,
  context: BugContestContext & { rng?: HardwareRNG } = {}
): string[] {
  // ASM: engine/events/bug_contest/contest_2.asm::SelectRandomBugContestContestants
  const { runner, overworld, event_manager } = resolveBugContestContext(context);
  const { rng } = context;
  void overworld;
  void event_manager;

  const flags = game_state.wram.event_flags as Record<string, boolean>;
  for (const flag of BUG_CONTESTANT_FLAGS) {
    flags[flag] = false;
  }

  const rngSource = rng ?? new HardwareRNG(game_state);
  const chosen = new Set<number>();
  while (chosen.size < 5) {
    const candidate = rngSource.randrange(BUG_CONTESTANT_FLAGS.length);
    if (!chosen.has(candidate)) {
      chosen.add(candidate);
    }
  }

  const hiddenFlags: string[] = [];
  for (const index of Array.from(chosen).sort((a, b) => a - b)) {
    const flag = BUG_CONTESTANT_FLAGS[index];
    flags[flag] = true;
    hiddenFlags.push(flag);
  }

  if (runner) {
    runner.last_value = hiddenFlags;
    runner.last_condition_result = true;
  }
  return hiddenFlags;
}

export function contest_drop_off_mons(
  game_state: GameState,
  context: BugContestContext = {}
): number {
  // ASM: engine/events/bug_contest/contest_2.asm::ContestDropOffMons
  const { runner, overworld, event_manager } = resolveBugContestContext(context);
  void overworld;
  void event_manager;

  const party = [...(game_state.sram.party?.pokemon ?? [])];
  if (!party.length || party[0] === null) {
    throw new Error("ContestDropOffMons requires a lead party Pokemon.");
  }

  const lead = party[0] as Pokemon;
  if ((lead.hp ?? 0) <= 0) {
    storeRunnerResult(runner, 1, false);
    return 1;
  }

  const contest = game_state.wram.bug_contest_state;
  contest.party_backup = party.slice(1);
  contest.pending_caught_mon = undefined;
  const secondMon = party[1] ?? null;
  if (secondMon) {
    game_state.wram.bug_contest_second_party_species = String(secondMon.species.id).toUpperCase();
  } else {
    game_state.wram.bug_contest_second_party_species = "";
  }
  game_state.sram.party.pokemon = [lead, ...Array(party.length - 1).fill(null)];
  contest.caught_species = undefined;
  contest.caught_level = undefined;
  game_state.wram.bug_contest_caught_mon = undefined;
  syncPartyCount(game_state);

  storeRunnerResult(runner, 0, true);
  return 0;
}

export function start_bug_contest_timer(
  game_state: GameState,
  context: BugContestContext = {}
): void {
  // ASM: engine/events/bug_contest/contest.asm::StartBugContestTimer
  const { runner, overworld, event_manager } = resolveBugContestContext(context);
  void overworld;
  void event_manager;

  new TimeSystem(game_state).updateTime();
  const timer = game_state.wram.bug_contest_timer;
  timer.mins_remaining = BUG_CONTEST_MINUTES;
  timer.secs_remaining = BUG_CONTEST_SECONDS;
  timer.start_time = currentContestTime(game_state);
  timer.started_at = undefined;
  game_state.wram.bug_contest_state.timer_active = true;

  if (runner) {
    runner.last_condition_result = true;
  }
}

export function check_bug_contest_timer(
  game_state: GameState,
  context: BugContestContext = {}
): boolean {
  // ASM: engine/events/bug_contest/contest.asm::CheckBugContestTimer
  const { runner, overworld, event_manager } = resolveBugContestContext(context);
  void overworld;
  void event_manager;

  const timer = game_state.wram.bug_contest_timer;
  if (!timer.start_time) {
    throw new Error("Bug contest timer has not been started.");
  }

  const currentTime = currentContestTime(game_state);
  const [daysSince, hoursSince, minutesSince, secondsSince] = elapsedSince(
    timer.start_time,
    currentTime
  );
  timer.start_time = currentTime;

  let timedOut = false;
  if (daysSince > 0 || hoursSince > 0) {
    timedOut = true;
  } else {
    let secondsRemaining = timer.secs_remaining - secondsSince;
    let borrow = 0;
    if (secondsRemaining < 0) {
      secondsRemaining += 60;
      borrow = 1;
    }
    const minutesRemaining = timer.mins_remaining - minutesSince - borrow;
    if (minutesRemaining < 0) {
      timedOut = true;
    } else {
      timer.mins_remaining = minutesRemaining;
      timer.secs_remaining = secondsRemaining;
    }
  }

  if (timedOut) {
    timer.mins_remaining = 0;
    timer.secs_remaining = 0;
  }

  const active = !timedOut;
  game_state.wram.bug_contest_state.timer_active = active;

  if (runner) {
    runner.last_condition_result = active;
  }
  return active;
}

export function contest_score(game_state: GameState, { contest_mon }: { contest_mon?: Pokemon | null } = {}): number {
  const mon = contest_mon ?? (game_state.wram.bug_contest_caught_mon as Pokemon | null);
  if (!mon) {
    return 0;
  }

  let score = 0;
  score += (mon.max_hp ?? 0) * 4;
  score += mon.attack ?? 0;
  score += mon.defense ?? 0;
  score += mon.speed ?? 0;
  score += mon.special_attack ?? 0;
  score += mon.special_defense ?? 0;

  const dvByte0 = ((mon.dvs.attack & 0xf) << 4) | (mon.dvs.defense & 0xf);
  const dvByte1 = ((mon.dvs.speed & 0xf) << 4) | (mon.dvs.special & 0xf);
  score += contestDvBonus(dvByte0, dvByte1);

  score += Math.max(0, mon.hp ?? 0) >> 3;
  if (mon.item) {
    score += 1;
  }
  return score;
}

export function bug_contest_judge_contestants(game_state: GameState): void {
  resetContestResults(game_state);
  const results = ensureContestResults(game_state);

  const playerMon = game_state.wram.bug_contest_caught_mon as Pokemon | null;
  const playerSpecies = playerMon ? String(playerMon.species.id).toUpperCase() : "";
  const playerScore = contest_score(game_state, { contest_mon: playerMon });
  const temp: BugContestWinner = {
    winner_id: BUG_CONTEST_PLAYER_ID,
    mon_species: playerSpecies,
    score: playerScore,
  };
  results.temp_winner = { ...temp };
  determine_contest_winners(game_state, temp);

  const rng = new HardwareRNG(game_state);
  const flags = game_state.wram.event_flags as Record<string, boolean>;
  CONTESTANTS.forEach((record, index) => {
    if (flags[BUG_CONTESTANT_FLAGS[index]]) {
      return;
    }
    const placement = rng.randrange(record.placements.length);
    const [species, baseScore] = record.placements[placement];
    const aiScore = baseScore + rng.randrange(8);
    const aiWinner: BugContestWinner = {
      winner_id: BUG_CONTEST_PLAYER_ID + index + 1,
      mon_species: species,
      score: aiScore,
    };
    results.temp_winner = { ...aiWinner };
    determine_contest_winners(game_state, aiWinner);
  });
}

export function determine_contest_winners(game_state: GameState, tempWinner: BugContestWinner): void {
  const results = ensureContestResults(game_state);
  const candidate = { ...tempWinner };

  if (candidate.score >= results.first_place.score) {
    results.third_place = { ...results.second_place };
    results.second_place = { ...results.first_place };
    results.first_place = candidate;
  } else if (candidate.score >= results.second_place.score) {
    results.third_place = { ...results.second_place };
    results.second_place = candidate;
  } else if (candidate.score >= results.third_place.score) {
    results.third_place = candidate;
  }
}

export function load_contestant_name(game_state: GameState, winner_id: number): string {
  if (winner_id === BUG_CONTEST_PLAYER_ID) {
    return game_state.sram.player_name || "PLAYER";
  }
  const index = winner_id - 2;
  if (index >= 0 && index < CONTESTANTS.length) {
    const record = CONTESTANTS[index];
    return `${record.trainer_class} ${record.trainer_name}`;
  }
  return "TRAINER";
}

export function BugContestJudging(game_state: GameState, overworld?: BugContestOverworldContext | null): number {
  // ASM: data/events/special_pointers.asm::BugContestJudging
  const runner = overworld?.script_runner;
  const event_manager = overworld?.event_manager;
  return bug_contest_judging(game_state, {
    runner: runner ?? undefined,
    overworld: overworld ?? undefined,
    event_manager: event_manager ?? undefined,
  });
}

export function SelectContestMon(game_state: GameState, overworld?: BugContestOverworldContext | null): number {
  // ASM: data/events/special_pointers.asm::SelectContestMon
  const runner = overworld?.script_runner;
  const event_manager = overworld?.event_manager;
  return check_party_full_after_contest(game_state, {
    runner: runner ?? undefined,
    overworld: overworld ?? undefined,
    event_manager: event_manager ?? undefined,
  });
}

export function bug_contest_judging(
  game_state: GameState,
  context: BugContestContext = {}
): number {
  // ASM: engine/events/std_scripts.asm::BugContestResultsScript
  const { runner, overworld, event_manager } = resolveBugContestContext(context);
  void overworld;
  void event_manager;

  bug_contest_judge_contestants(game_state);
  const results = ensureContestResults(game_state);

  let rank = 4;
  if (results.first_place.winner_id === BUG_CONTEST_PLAYER_ID) {
    rank = 1;
  } else if (results.second_place.winner_id === BUG_CONTEST_PLAYER_ID) {
    rank = 2;
  } else if (results.third_place.winner_id === BUG_CONTEST_PLAYER_ID) {
    rank = 3;
  }

  if (runner) {
    storeRunnerResult(runner, rank, true);
    if (runner.string_buffers) {
      runner.string_buffers["STRING_BUFFER_3"] = String(rank);
    }
  }
  return rank;
}

export function bug_contest_set_caught_contest_mon(
  game_state: GameState,
  context: BugContestContext & {
    caught_mon: Pokemon | null;
    keep_new?: boolean | null;
  }
): number {
  // ASM: engine/items/item_effects.asm::BugContest_SetCaughtContestMon
  const { runner, overworld, event_manager } = resolveBugContestContext(context);
  const { caught_mon, keep_new } = context;
  void overworld;

  if (!caught_mon) {
    throw new Error("Caught contest Pokemon data is required.");
  }

  const newMon = JSON.parse(JSON.stringify(caught_mon)) as Pokemon;
  newMon.status = undefined;
  newMon.original_trainer_name = game_state.sram.player_name || "PLAYER";
  newMon.original_trainer_id = game_state.sram.player_id;

  const contest = game_state.wram.bug_contest_state;
  const current = game_state.wram.bug_contest_caught_mon as Pokemon | null;

  let decision = true;
  let result = 0;

  if (current && keep_new === undefined) {
    contest.pending_caught_mon = newMon;
    if (event_manager) {
      event_manager.dispatch(
        new Event("show_text", {
          text: `Already caught ${formatContestSpecies(current.species.id)}.`,
        })
      );
      event_manager.dispatch(
        new Event("prompt_yes_no", {
          text: `Switch to ${formatContestSpecies(newMon.species.id)}?`,
          callback: (keep: boolean) => {
            bug_contest_set_caught_contest_mon(game_state, {
              runner,
              overworld,
              event_manager,
              caught_mon: newMon,
              keep_new: keep,
            });
          },
        })
      );
    }
    decision = false;
    result = BUGCONTEST_DECISION_PENDING;
  } else {
    decision = current ? Boolean(keep_new) : true;
    if (decision) {
      game_state.wram.bug_contest_caught_mon = newMon;
      contest.caught_species = String(newMon.species.id).toUpperCase();
      contest.caught_level = newMon.level;
      if (event_manager) {
        const speciesName = formatContestSpecies(newMon.species.id);
        event_manager.dispatch(new Event("show_text", { text: `Caught ${speciesName}!` }));
      }
      result = 0;
    } else {
      result = 1;
    }
    contest.pending_caught_mon = undefined;
  }

  storeRunnerResult(runner, result, decision);
  return result;
}

export function choose_wild_encounter_bug_contest(
  game_state: GameState,
  context: BugContestContext & { rng?: HardwareRNG } = {}
): [string, number] {
  // ASM: engine/events/sweet_scent.asm::ChooseWildEncounter_BugContest
  const { runner, overworld, event_manager } = resolveBugContestContext(context);
  const { rng } = context;
  void overworld;
  void event_manager;

  const rngSource = rng ?? new HardwareRNG(game_state);
  let roll = rngSource.randrange(100);

  for (const entry of CONTEST_WILD_TABLE) {
    roll -= entry.percent >= 0 ? entry.percent : 255;
    if (roll < 0) {
      const levelRange = entry.max_level - entry.min_level + 1;
      const level = levelRange <= 0 ? entry.min_level : entry.min_level + rngSource.randrange(levelRange);
      if (runner) {
        runner.last_value = { species: entry.species, level };
        runner.last_condition_result = true;
      }
      return [entry.species, level];
    }
  }

  throw new Error("Bug contest encounter table exhausted.");
}

export function contest_return_mons(
  game_state: GameState,
  context: BugContestContext = {}
): void {
  // ASM: engine/events/bug_contest/contest_2.asm::ContestReturnMons
  const { runner, overworld, event_manager } = resolveBugContestContext(context);
  void overworld;
  void event_manager;

  const contest = game_state.wram.bug_contest_state;
  const party = [...(game_state.sram.party?.pokemon ?? [])];
  if (!party.length) {
    throw new Error("ContestReturnMons requires an active party.");
  }
  const restored = [party[0], ...(contest.party_backup ?? [])];
  while (restored.length < party.length) {
    restored.push(null);
  }
  game_state.sram.party.pokemon = restored.slice(0, party.length);
  contest.party_backup = [];
  contest.pending_caught_mon = undefined;
  game_state.wram.bug_contest_second_party_species = "";
  syncPartyCount(game_state);

  if (runner) {
    runner.last_condition_result = true;
  }
}

const addToPc = (game_state: GameState, pokemon: Pokemon): boolean => {
  const boxes = game_state.sram.pc_boxes ?? [];
  if (boxes.length === 0) {
    boxes.push(BoxSchema.parse({ name: formatDefaultBoxName(0) }));
  }

  for (const box of boxes) {
    if (addBoxPokemon(box, pokemon)) {
      return true;
    }
  }

  if (boxes.length < MAX_PC_BOXES) {
    const newBox = BoxSchema.parse({ name: formatDefaultBoxName(boxes.length) });
    boxes.push(newBox);
    return addBoxPokemon(newBox, pokemon);
  }

  return false;
};

export function check_party_full_after_contest(
  game_state: GameState,
  context: BugContestContext = {}
): number {
  // ASM: engine/events/bug_contest/contest_2.asm::CheckPartyFullAfterContest
  const { runner, overworld, event_manager } = resolveBugContestContext(context);
  void overworld;
  void event_manager;

  const contestMon = game_state.wram.bug_contest_caught_mon as Pokemon | null;
  const contest = game_state.wram.bug_contest_state;

  let result: number;
  if (!contestMon) {
    contest.pending_caught_mon = undefined;
    result = BUGCONTEST_NO_CATCH;
  } else {
    const party = game_state.sram.party;
    if (addPartyPokemon(party, contestMon)) {
      result = BUGCONTEST_CAUGHT_MON;
    } else if (addToPc(game_state, contestMon)) {
      result = BUGCONTEST_BOXED_MON;
    } else {
      throw new Error("No room left in party or PC for the contest prize.");
    }
    game_state.wram.bug_contest_caught_mon = undefined;
    contest.caught_species = undefined;
    contest.caught_level = undefined;
    contest.pending_caught_mon = undefined;
  }
  syncPartyCount(game_state);

  storeRunnerResult(runner, result, true);
  return result;
}

const resetContestResults = (game_state: GameState): void => {
  game_state.wram.bug_contest_results = BugContestResultsSchema.parse({});
};

const contestDvBonus = (byte0: number, byte1: number): number => {
  let a = byte0 & 0x02;
  a = (a + a) & 0xff;
  a = (a + a) & 0xff;
  const c = a;

  const swapped0 = ((byte0 & 0x0f) << 4) | ((byte0 & 0xf0) >> 4);
  a = swapped0 & 0x02;
  a = (a + a) & 0xff;
  a = (a + c) & 0xff;
  const d = a;

  const c2 = byte1 & 0x02;
  const swapped1 = ((byte1 & 0x0f) << 4) | ((byte1 & 0xf0) >> 4);
  a = swapped1 & 0x02;
  a >>= 1;
  a = (a + c2) & 0xff;
  a = (a + c2) & 0xff;
  a = (a + d) & 0xff;
  a = (a + d) & 0xff;
  return a;
};

const syncPartyCount = (game_state: GameState): void => {
  const party = game_state.sram.party?.pokemon ?? [];
  game_state.wram.wPartyCount = party.filter((mon) => mon !== null).length;
};
