// ASM mapping: pokecrystal_disassembly/engine/events/overworld.asm (Fishing_CheckFacingUp, Fish).
import { DataLoader } from "@pokecrystal/core/core/data-loader";
import { GameState } from "@pokecrystal/core/core/state";
import { HardwareRNG } from "@pokecrystal/core/engine/games/rng";
import { normalizeMapKey } from "./overworld-base";
import { WildEncounter } from "@pokecrystal/assets/content/wild-encounter-data";
import { canonicaliseTimeOfDay } from "@pokecrystal/core/engine/systems/time";

const FISH_SWARM_FLAG_BIT = 1 << 2;
const SWARM_QWILFISH = 1;
const SWARM_REMORAID = 2;

const ROD_INDEX: Record<string, number> = { OLD_ROD: 0, GOOD_ROD: 1, SUPER_ROD: 2 };
const getFrameCounter = (state: GameState): number => Math.max(0, Math.trunc(state.frame_counter ?? 0));

export enum FishingRodState {
  IDLE = 0,
  WAITING = 1,
  BITE = 2,
  BATTLE = 3,
}

export type TimeFishEntry = {
  day_species: string;
  day_level: number;
  night_species: string;
  night_level: number;
};

export type FishingSlot = {
  threshold: number;
  species: string | null;
  level: number;
  time_group?: number | null;
};

export type RodTable = {
  slots: FishingSlot[];
};

export type FishingGroup = {
  bite_threshold: number;
  rod_tables: Record<string, RodTable>;
};

export type FishingOutcome = {
  bite: boolean;
  encounter: WildEncounter | null;
  group: string | null;
  bite_roll: number;
};

const percent_to_byte = (percent: number): number => {
  const clamped = Math.max(0, Math.min(100, percent));
  return Math.trunc((clamped * 0xff) / 100);
};

const threshold = (percent: number, { add_one = false }: { add_one?: boolean } = {}): number => {
  const base = percent_to_byte(percent);
  return add_one ? Math.min(0xff, base + 1) : base;
};

const slot = (
  percent: number,
  species: string | null,
  level: number,
  { add_one = false, time_group = null }: { add_one?: boolean; time_group?: number | null } = {}
): FishingSlot => ({
  threshold: threshold(percent, { add_one }),
  species,
  level,
  time_group,
});

const rod = (slots: FishingSlot[]): RodTable => ({ slots });

const TIME_GROUPS: TimeFishEntry[] = [
  { day_species: "CORSOLA", day_level: 20, night_species: "STARYU", night_level: 20 },
  { day_species: "CORSOLA", day_level: 40, night_species: "STARYU", night_level: 40 },
  { day_species: "SHELLDER", day_level: 20, night_species: "SHELLDER", night_level: 20 },
  { day_species: "SHELLDER", day_level: 40, night_species: "SHELLDER", night_level: 40 },
  { day_species: "GOLDEEN", day_level: 20, night_species: "GOLDEEN", night_level: 20 },
  { day_species: "GOLDEEN", day_level: 40, night_species: "GOLDEEN", night_level: 40 },
  { day_species: "POLIWAG", day_level: 20, night_species: "POLIWAG", night_level: 20 },
  { day_species: "POLIWAG", day_level: 40, night_species: "POLIWAG", night_level: 40 },
  { day_species: "DRATINI", day_level: 20, night_species: "DRATINI", night_level: 20 },
  { day_species: "DRATINI", day_level: 40, night_species: "DRATINI", night_level: 40 },
  { day_species: "QWILFISH", day_level: 20, night_species: "QWILFISH", night_level: 20 },
  { day_species: "QWILFISH", day_level: 40, night_species: "QWILFISH", night_level: 40 },
  { day_species: "REMORAID", day_level: 20, night_species: "REMORAID", night_level: 20 },
  { day_species: "REMORAID", day_level: 40, night_species: "REMORAID", night_level: 40 },
  { day_species: "GYARADOS", day_level: 20, night_species: "GYARADOS", night_level: 20 },
  { day_species: "GYARADOS", day_level: 40, night_species: "GYARADOS", night_level: 40 },
  { day_species: "DRATINI", day_level: 10, night_species: "DRATINI", night_level: 10 },
  { day_species: "DRATINI", day_level: 10, night_species: "DRATINI", night_level: 10 },
  { day_species: "HORSEA", day_level: 20, night_species: "HORSEA", night_level: 20 },
  { day_species: "HORSEA", day_level: 40, night_species: "HORSEA", night_level: 40 },
  { day_species: "TENTACOOL", day_level: 20, night_species: "TENTACOOL", night_level: 20 },
  { day_species: "TENTACOOL", day_level: 40, night_species: "TENTACOOL", night_level: 40 },
];

const GROUP_BITE = threshold(50, { add_one: true });

const GROUPS: Record<string, FishingGroup> = {
  FISHGROUP_SHORE: {
    bite_threshold: GROUP_BITE,
    rod_tables: {
      OLD_ROD: rod([
        slot(70, "MAGIKARP", 10, { add_one: true }),
        slot(85, "MAGIKARP", 10, { add_one: true }),
        slot(100, "KRABBY", 10),
      ]),
      GOOD_ROD: rod([
        slot(35, "MAGIKARP", 20),
        slot(70, "KRABBY", 20),
        slot(90, "KRABBY", 20, { add_one: true }),
        slot(100, null, 0, { time_group: 0 }),
      ]),
      SUPER_ROD: rod([
        slot(40, "KRABBY", 40),
        slot(70, null, 0, { time_group: 1 }),
        slot(90, "KRABBY", 40, { add_one: true }),
        slot(100, "KINGLER", 40),
      ]),
    },
  },
  FISHGROUP_OCEAN: {
    bite_threshold: GROUP_BITE,
    rod_tables: {
      OLD_ROD: rod([
        slot(70, "MAGIKARP", 10, { add_one: true }),
        slot(85, "MAGIKARP", 10, { add_one: true }),
        slot(100, "TENTACOOL", 10),
      ]),
      GOOD_ROD: rod([
        slot(35, "MAGIKARP", 20),
        slot(70, "TENTACOOL", 20),
        slot(90, "CHINCHOU", 20, { add_one: true }),
        slot(100, null, 0, { time_group: 2 }),
      ]),
      SUPER_ROD: rod([
        slot(40, "CHINCHOU", 40),
        slot(70, null, 0, { time_group: 3 }),
        slot(90, "TENTACRUEL", 40, { add_one: true }),
        slot(100, "LANTURN", 40),
      ]),
    },
  },
  FISHGROUP_LAKE: {
    bite_threshold: GROUP_BITE,
    rod_tables: {
      OLD_ROD: rod([
        slot(70, "MAGIKARP", 10, { add_one: true }),
        slot(85, "MAGIKARP", 10, { add_one: true }),
        slot(100, "GOLDEEN", 10),
      ]),
      GOOD_ROD: rod([
        slot(35, "MAGIKARP", 20),
        slot(70, "GOLDEEN", 20),
        slot(90, "GOLDEEN", 20, { add_one: true }),
        slot(100, null, 0, { time_group: 4 }),
      ]),
      SUPER_ROD: rod([
        slot(40, "GOLDEEN", 40),
        slot(70, null, 0, { time_group: 5 }),
        slot(90, "MAGIKARP", 40, { add_one: true }),
        slot(100, "SEAKING", 40),
      ]),
    },
  },
  FISHGROUP_POND: {
    bite_threshold: GROUP_BITE,
    rod_tables: {
      OLD_ROD: rod([
        slot(70, "MAGIKARP", 10, { add_one: true }),
        slot(85, "MAGIKARP", 10, { add_one: true }),
        slot(100, "POLIWAG", 10),
      ]),
      GOOD_ROD: rod([
        slot(35, "MAGIKARP", 20),
        slot(70, "POLIWAG", 20),
        slot(90, "POLIWAG", 20, { add_one: true }),
        slot(100, null, 0, { time_group: 6 }),
      ]),
      SUPER_ROD: rod([
        slot(40, "POLIWAG", 40),
        slot(70, null, 0, { time_group: 7 }),
        slot(90, "MAGIKARP", 40, { add_one: true }),
        slot(100, "POLIWAG", 40),
      ]),
    },
  },
  FISHGROUP_DRATINI: {
    bite_threshold: GROUP_BITE,
    rod_tables: {
      OLD_ROD: rod([
        slot(70, "MAGIKARP", 10, { add_one: true }),
        slot(85, "MAGIKARP", 10, { add_one: true }),
        slot(100, "MAGIKARP", 10),
      ]),
      GOOD_ROD: rod([
        slot(35, "MAGIKARP", 20),
        slot(70, "MAGIKARP", 20),
        slot(90, "MAGIKARP", 20, { add_one: true }),
        slot(100, null, 0, { time_group: 8 }),
      ]),
      SUPER_ROD: rod([
        slot(40, "MAGIKARP", 40),
        slot(70, null, 0, { time_group: 9 }),
        slot(90, "MAGIKARP", 40, { add_one: true }),
        slot(100, "DRAGONAIR", 40),
      ]),
    },
  },
  FISHGROUP_QWILFISH_SWARM: {
    bite_threshold: GROUP_BITE,
    rod_tables: {
      OLD_ROD: rod([
        slot(70, "MAGIKARP", 5, { add_one: true }),
        slot(85, "MAGIKARP", 5, { add_one: true }),
        slot(100, "QWILFISH", 5),
      ]),
      GOOD_ROD: rod([
        slot(35, "MAGIKARP", 20),
        slot(70, "QWILFISH", 20),
        slot(90, "QWILFISH", 20, { add_one: true }),
        slot(100, null, 0, { time_group: 10 }),
      ]),
      SUPER_ROD: rod([
        slot(40, "QWILFISH", 40),
        slot(70, null, 0, { time_group: 11 }),
        slot(90, "QWILFISH", 40, { add_one: true }),
        slot(100, "QWILFISH", 40),
      ]),
    },
  },
  FISHGROUP_REMORAID_SWARM: {
    bite_threshold: GROUP_BITE,
    rod_tables: {
      OLD_ROD: rod([
        slot(70, "MAGIKARP", 10, { add_one: true }),
        slot(85, "MAGIKARP", 10, { add_one: true }),
        slot(100, "REMORAID", 10),
      ]),
      GOOD_ROD: rod([
        slot(35, "MAGIKARP", 20),
        slot(70, "REMORAID", 20),
        slot(90, "REMORAID", 20, { add_one: true }),
        slot(100, null, 0, { time_group: 12 }),
      ]),
      SUPER_ROD: rod([
        slot(40, "REMORAID", 40),
        slot(70, null, 0, { time_group: 13 }),
        slot(90, "REMORAID", 40, { add_one: true }),
        slot(100, "REMORAID", 40),
      ]),
    },
  },
  FISHGROUP_GYARADOS: {
    bite_threshold: GROUP_BITE,
    rod_tables: {
      OLD_ROD: rod([
        slot(70, "MAGIKARP", 10, { add_one: true }),
        slot(85, "MAGIKARP", 10, { add_one: true }),
        slot(100, "MAGIKARP", 10),
      ]),
      GOOD_ROD: rod([
        slot(35, "MAGIKARP", 20),
        slot(70, "MAGIKARP", 20),
        slot(90, "MAGIKARP", 20, { add_one: true }),
        slot(100, null, 0, { time_group: 14 }),
      ]),
      SUPER_ROD: rod([
        slot(40, "MAGIKARP", 40),
        slot(70, null, 0, { time_group: 15 }),
        slot(90, "MAGIKARP", 40, { add_one: true }),
        slot(100, "MAGIKARP", 40),
      ]),
    },
  },
  FISHGROUP_DRATINI_2: {
    bite_threshold: GROUP_BITE,
    rod_tables: {
      OLD_ROD: rod([
        slot(70, "MAGIKARP", 10, { add_one: true }),
        slot(85, "MAGIKARP", 10, { add_one: true }),
        slot(100, "MAGIKARP", 10),
      ]),
      GOOD_ROD: rod([
        slot(35, "MAGIKARP", 10),
        slot(70, "MAGIKARP", 10),
        slot(90, "MAGIKARP", 10, { add_one: true }),
        slot(100, null, 0, { time_group: 16 }),
      ]),
      SUPER_ROD: rod([
        slot(40, "MAGIKARP", 10),
        slot(70, null, 0, { time_group: 17 }),
        slot(90, "MAGIKARP", 10, { add_one: true }),
        slot(100, "DRAGONAIR", 10),
      ]),
    },
  },
  FISHGROUP_WHIRL_ISLANDS: {
    bite_threshold: GROUP_BITE,
    rod_tables: {
      OLD_ROD: rod([
        slot(70, "MAGIKARP", 10, { add_one: true }),
        slot(85, "MAGIKARP", 10, { add_one: true }),
        slot(100, "KRABBY", 10),
      ]),
      GOOD_ROD: rod([
        slot(35, "MAGIKARP", 20),
        slot(70, "KRABBY", 20),
        slot(90, "KRABBY", 20, { add_one: true }),
        slot(100, null, 0, { time_group: 18 }),
      ]),
      SUPER_ROD: rod([
        slot(40, "KRABBY", 40),
        slot(70, null, 0, { time_group: 19 }),
        slot(90, "KINGLER", 40, { add_one: true }),
        slot(100, "SEADRA", 40),
      ]),
    },
  },
  FISHGROUP_QWILFISH: {
    bite_threshold: GROUP_BITE,
    rod_tables: {
      OLD_ROD: rod([
        slot(70, "MAGIKARP", 10, { add_one: true }),
        slot(85, "MAGIKARP", 10, { add_one: true }),
        slot(100, "TENTACOOL", 10),
      ]),
      GOOD_ROD: rod([
        slot(35, "MAGIKARP", 20),
        slot(70, "TENTACOOL", 20),
        slot(90, "TENTACOOL", 20, { add_one: true }),
        slot(100, null, 0, { time_group: 20 }),
      ]),
      SUPER_ROD: rod([
        slot(40, "TENTACOOL", 40),
        slot(70, null, 0, { time_group: 21 }),
        slot(90, "MAGIKARP", 40, { add_one: true }),
        slot(100, "QWILFISH", 40),
      ]),
    },
  },
  FISHGROUP_REMORAID: {
    bite_threshold: GROUP_BITE,
    rod_tables: {
      OLD_ROD: rod([
        slot(70, "MAGIKARP", 10, { add_one: true }),
        slot(85, "MAGIKARP", 10, { add_one: true }),
        slot(100, "POLIWAG", 10),
      ]),
      GOOD_ROD: rod([
        slot(35, "MAGIKARP", 20),
        slot(70, "POLIWAG", 20),
        slot(90, "POLIWAG", 20, { add_one: true }),
        slot(100, null, 0, { time_group: 6 }),
      ]),
      SUPER_ROD: rod([
        slot(40, "POLIWAG", 40),
        slot(70, null, 0, { time_group: 7 }),
        slot(90, "MAGIKARP", 40, { add_one: true }),
        slot(100, "REMORAID", 40),
      ]),
    },
  },
  FISHGROUP_QWILFISH_NO_SWARM: {
    bite_threshold: GROUP_BITE,
    rod_tables: {},
  },
};

GROUPS["FISHGROUP_QWILFISH_NO_SWARM"] = GROUPS["FISHGROUP_QWILFISH"];

const resolve_fishing_time_of_day = (time_of_day: string): "DAY" | "NIGHT" | "MORN" => {
  const raw = String(time_of_day ?? "").trim();
  const canonical = canonicaliseTimeOfDay(raw);
  if (!raw) {
    return canonical;
  }
  const normalized = raw.toUpperCase();
  if (canonical === "DAY" && normalized !== "DAY" && normalized !== "AFTERNOON") {
    throw new Error(`Unknown fishing time of day '${time_of_day}'.`);
  }
  return canonical;
};

const resolve_time_group = (time_group: number, time_of_day: string): [string, number] => {
  const entry = TIME_GROUPS[time_group];
  if (!entry) {
    throw new Error(`Fishing time group ${time_group} is not defined.`);
  }
  // ASM: Fish.TimeEncounter uses wTimeOfDay >= NITE_F to select the night entry.
  if (resolve_fishing_time_of_day(time_of_day) === "NIGHT") {
    return [entry.night_species, entry.night_level];
  }
  return [entry.day_species, entry.day_level];
};

const resolve_group_token = (group: string | null | undefined, game_state: GameState | null): string | null => {
  const normalized = group?.toUpperCase() ?? null;
  if (!normalized || normalized === "FISHGROUP_NONE") {
    return null;
  }
  if (!game_state) {
    return normalized;
  }
  const daily_flags = game_state.wram.daily_flags1 ?? 0;
  const swarm_flag = game_state.wram.wFishingSwarmFlag ?? 0;
  const swarm_active = Boolean(daily_flags & FISH_SWARM_FLAG_BIT);
  if (swarm_active && normalized === "FISHGROUP_QWILFISH" && swarm_flag === SWARM_QWILFISH) {
    return "FISHGROUP_QWILFISH_SWARM";
  }
  if (swarm_active && normalized === "FISHGROUP_REMORAID" && swarm_flag === SWARM_REMORAID) {
    return "FISHGROUP_REMORAID_SWARM";
  }
  return normalized;
};

const resolve_group = (map_name: string, data_loader: DataLoader, game_state: GameState | null): string | null => {
  const mapAttributes = data_loader.map_attributes;
  const normalizedInput = String(map_name ?? "");
  let attributes = mapAttributes.get(normalizedInput) ?? null;
  if (!attributes && normalizedInput) {
    const normalizedKey = normalizeMapKey(normalizedInput);
    attributes = normalizedKey ? mapAttributes.get(normalizedKey) ?? null : null;
  }
  const group_name = attributes?.fishing_group ?? null;
  return resolve_group_token(group_name, game_state);
};

export function roll_fishing_encounter(
  game_state: GameState,
  data_loader: DataLoader,
  map_name: string,
  rod_item: string,
  rng: HardwareRNG
): FishingOutcome {
  const rod = rod_item.trim().toUpperCase();
  if (!(rod in ROD_INDEX)) {
    throw new Error(`Unknown fishing rod '${rod_item}'.`);
  }
  const group_name = resolve_group(map_name, data_loader, game_state);
  if (!group_name) {
    return { bite: false, encounter: null, group: null, bite_roll: 0 };
  }
  const group = GROUPS[group_name];
  if (!group) {
    throw new Error(`Fishing group ${group_name} is not defined.`);
  }

  const bite_roll = rng.nextByte();
  if (bite_roll >= group.bite_threshold) {
    return { bite: false, encounter: null, group: group_name, bite_roll };
  }

  const rod_table = group.rod_tables[rod];
  if (!rod_table || !rod_table.slots.length) {
    throw new Error(`Fishing group '${group_name}' is missing the ${rod} encounter table.`);
  }

  const slot_roll = rng.nextByte();
  for (const slot_entry of rod_table.slots) {
    if (slot_roll <= slot_entry.threshold) {
      let species = slot_entry.species;
      let level = slot_entry.level;
      if (species === null && slot_entry.time_group !== null && slot_entry.time_group !== undefined) {
        const resolved = resolve_time_group(
          slot_entry.time_group,
          game_state.wram.time_of_day ?? "day"
        );
        species = resolved[0];
        level = resolved[1];
      }
      if (!species) {
        throw new Error(`Fishing slot in ${group_name}/${rod} resolved without a species.`);
      }
      return { bite: true, encounter: { level, species }, group: group_name, bite_roll };
    }
  }
  throw new Error(`Fishing slot roll ${slot_roll} did not resolve within ${group_name}/${rod}.`);
}

export class FishingSession {
  public rod: string;
  public outcome: FishingOutcome;
  public start_frame: number;
  public bite_delay_frames: number;
  public group: string | null;
  public cast_frames: number = 40;
  public bites_remaining: number = 0;
  public resolved: boolean = false;
  public resolution: boolean | null = null;

  constructor(options: {
    rod: string;
    outcome: FishingOutcome;
    start_frame: number;
    bite_delay_frames: number;
    group: string | null;
  }) {
    this.rod = options.rod;
    this.outcome = options.outcome;
    this.start_frame = options.start_frame;
    this.bite_delay_frames = options.bite_delay_frames;
    this.group = options.group;
  }
}

export function DoFishing(
  game_state: GameState,
  data_loader: DataLoader,
  map_name: string,
  rod_item: string,
  rng: HardwareRNG
): FishingSession {
  const outcome = roll_fishing_encounter(game_state, data_loader, map_name, rod_item, rng);
  const bite_windows = 1;
  const session = new FishingSession({
    rod: rod_item.trim().toUpperCase(),
    outcome,
    start_frame: getFrameCounter(game_state),
    // ASM: Script_FishCastRod pauses 40 frames before the bite/no-bite branch.
    bite_delay_frames: 0,
    group: outcome.group,
  });
  session.bites_remaining = bite_windows;

  game_state.wram.wFishingRodState = FishingRodState.WAITING;
  game_state.wram.wFishingRodRod = ROD_INDEX[session.rod] ?? -1;
  game_state.wram.wFishingRodBites = bite_windows;
  game_state.wram.wFishingResult = 0;
  return session;
}

export function FishingBite(
  game_state: GameState,
  session: FishingSession,
  { current_frame = null }: { current_frame?: number | null } = {}
): boolean | null {
  const frame = current_frame === null ? getFrameCounter(game_state) : current_frame;
  const elapsed = frame - session.start_frame;
  const bite_frame = session.cast_frames + session.bite_delay_frames;
  if (elapsed < bite_frame) {
    return null;
  }
  if (session.resolved) {
    return session.resolution;
  }
  if (!session.outcome.bite || !session.outcome.encounter) {
    game_state.wram.wFishingRodState = FishingRodState.IDLE;
    game_state.wram.wFishingRodBites = 0;
    game_state.wram.wFishingResult = session.group === null ? 0 : 2;
    session.resolved = true;
    session.resolution = false;
    return false;
  }
  game_state.wram.wFishingRodState = FishingRodState.BITE;
  game_state.wram.wFishingResult = 1;
  session.resolved = true;
  session.resolution = true;
  return true;
}

export function FishingBattleTrigger(game_state: GameState, session: FishingSession): void {
  void session;
  game_state.wram.wFishingRodState = FishingRodState.BATTLE;
  game_state.wram.wFishingRodBites = 0;
}
