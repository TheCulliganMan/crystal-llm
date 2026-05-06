import { GameState } from "@pokecrystal/core/core/state";
import { EventManager } from "@pokecrystal/core/engine/events/events";
import {
  HALL_OF_FAME_NICKNAME_LENGTH,
  HALL_OF_FAME_TEAM_SIZE,
  HOF_MASTER_COUNT,
  NUM_HALL_OF_FAME_ENTRIES,
  Pokemon as PokemonId,
} from "@pokecrystal/core/core/constants";
import { HallOfFameEntry, HallOfFamePokemon, Pokemon } from "@pokecrystal/core/core/models";
import { Command, type OverworldContext } from "./base";
import { CreditsCommand } from "./credits";
import type { ScriptRunner } from "../runner";

const partyMembers = (gameState: GameState): Pokemon[] => {
  const members = gameState.sram.party.pokemon;
  if (!members.length) {
    return [];
  }
  const rawPartyCount = gameState.wram.wPartyCount;
  const partyCount =
    typeof rawPartyCount === "number" && Number.isFinite(rawPartyCount) && rawPartyCount > 0
      ? Math.min(rawPartyCount, HALL_OF_FAME_TEAM_SIZE)
      : members.length;
  return members.slice(0, partyCount).filter((mon): mon is Pokemon => Boolean(mon));
};

const isEgg = (mon: Pokemon): boolean => {
  const speciesId = mon.species.id.trim().toUpperCase();
  return speciesId === "EGG" || mon.species.int_id === PokemonId.EGG;
};

const clampNickname = (value: string | undefined | null): string => {
  const nickname = String(value ?? "");
  if (nickname.length > HALL_OF_FAME_NICKNAME_LENGTH) {
    return nickname.slice(0, HALL_OF_FAME_NICKNAME_LENGTH);
  }
  return nickname;
};

const coerceHallOfFameCount = (value: unknown): number | null => {
  if (typeof value !== "number" || !Number.isFinite(value)) {
    return null;
  }
  return value & 0xff;
};

const resolveHallOfFameCount = (
  gameState: GameState,
  entries: HallOfFameEntry[],
): number => {
  const count = coerceHallOfFameCount(gameState.wram?.wHallOfFameCount);
  if (count !== null) {
    return count < HOF_MASTER_COUNT ? (count + 1) & 0xff : count;
  }

  if (!entries.length) {
    return 1;
  }
  const maxCount = Math.max(...entries.map((entry) => entry.win_count ?? 0));
  return Math.min(maxCount + 1, HOF_MASTER_COUNT);
};

const hasPriorHallOfFameRecord = (
  gameState: GameState,
  entries: HallOfFameEntry[],
): boolean => {
  const count = coerceHallOfFameCount(gameState.wram?.wHallOfFameCount);
  if (count !== null && count > 0) {
    return true;
  }

  // Compatibility for pre-WRAM-migration saves: parsed saves default missing count to 0.
  return Boolean(entries.length);
};

const packDvs = (mon: Pokemon): number => {
  const { attack, defense, speed, special } = mon.dvs;
  return ((attack & 0xf) << 12) | ((defense & 0xf) << 8) | ((speed & 0xf) << 4) | (special & 0xf);
};

const resolveSpeciesId = (mon: Pokemon): { id: number; token: string } | null => {
  const rawId = mon.species.id.trim().toUpperCase();
  if (!rawId) {
    return null;
  }
  const resolved = PokemonId[rawId as keyof typeof PokemonId];
  if (typeof resolved !== "number") {
    return null;
  }
  return { id: resolved, token: rawId };
};

const canSkipCredits = (gameState: GameState, entries: HallOfFameEntry[]): boolean => {
  const flags = gameState.wram.engine_flags ?? {};
  // Mirrors pokecrystal_disassembly/engine/movie/credits.asm::Credits:
  // skip is allowed after the first Hall of Fame sequence for this save.
  if (Boolean(flags["STATUSFLAGS_HALL_OF_FAME_F"])) {
    return true;
  }
  return hasPriorHallOfFameRecord(gameState, entries);
};

export class HallOfFameCommand extends Command {
  public execute(gameState: GameState, eventManager: EventManager, overworld: OverworldContext): void {
    const runner: ScriptRunner | undefined = this.runner;
    const entries = gameState.sram.hall_of_fame;
    const allowSkip = canSkipCredits(gameState, entries);
    // ASM mapping:
    // engine/events/halloffame.asm::HallOfFame:
    //   read/increment wHallOfFameCount with HOF_MASTER_COUNT saturation
    //   then AddHallOfFameEntry inserts the new team at slot 0.
    const winCount = resolveHallOfFameCount(gameState, entries);

    const members: HallOfFamePokemon[] = [];
    for (const mon of partyMembers(gameState)) {
      if (isEgg(mon)) {
        continue;
      }
      const resolved = resolveSpeciesId(mon);
      if (!resolved) {
        continue;
      }
      members.push({
        species: resolved.token,
        id: resolved.id,
        trainer_id: mon.original_trainer_id,
        dvs: packDvs(mon),
        level: mon.level,
        nickname: clampNickname(mon.nickname),
      });
    }
    while (members.length < HALL_OF_FAME_TEAM_SIZE) {
      members.push({});
    }

    const entry: HallOfFameEntry = {
      win_count: winCount,
      team: members,
      pokemon: members,
    };
    entries.unshift(entry);
    if (entries.length > NUM_HALL_OF_FAME_ENTRIES) {
      entries.splice(NUM_HALL_OF_FAME_ENTRIES);
    }
    gameState.wram.wHallOfFameCount = winCount;

    gameState.wram.engine_flags["STATUSFLAGS_HALL_OF_FAME_F"] = true;
    gameState.wram.wSpawnAfterChampion = 1;

    const summary = {
      win_count: winCount,
      team: members.map((member) => {
        if (member.nickname) {
          return member.nickname;
        }
        const species = (member.species ?? "").toUpperCase();
        if (species === "EGG" || member.id === PokemonId.EGG) {
          return "";
        }
        return member.species ?? "";
      }),
      total_entries: entries.length,
    };
    if (runner) {
      runner.last_condition_result = true;
      runner.last_value = { hall_of_fame: summary };
    }

    const credits = new CreditsCommand(allowSkip);
    credits.runner = runner;
    credits.execute(gameState, eventManager, overworld);
  }
}
