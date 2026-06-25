import { TMHM_MOVES } from "../../core/tmhm";
export { TMHM_MOVES };
import type { LearnedMove, Move, Pokemon } from "../../core/models";
import { MoveName } from "../../core/enums/move";
import { loadAllMoves } from "../../core/models/move";
import { ItemSystem } from "./items";

const TM_PREFIX = "TM";
const HM_PREFIX = "HM";
const TM_HM_PREFIX = "TM_HM_";
const TM_COUNT = 50; // Number of indexed TMs before HMs start.

export class TMHMResolutionError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "TMHMResolutionError";
  }
}

const NON_ALNUM = /[^A-Z0-9]+/g;

const collapseToken = (token: string): string => {
  return token.toUpperCase().replace(NON_ALNUM, "");
};


const COLLAPSED_MOVE_NAMES = new Map<string, MoveName>();
for (const candidate of Object.values(MoveName)) {
  COLLAPSED_MOVE_NAMES.set(collapseToken(candidate), candidate);
}

const normalizeItemName = (itemName: string): string => {
  return itemName.replace(/ /g, "_").toUpperCase();
};

const resolveMoveName = (token: string): MoveName | undefined => {
  if (token in MoveName) {
    return MoveName[token as keyof typeof MoveName];
  }
  return undefined;
};

export const resolveTmhmMove = (
  itemName: string
): [MoveName, boolean] => {
  const normalized = normalizeItemName(itemName);
  let offset: number;
  let isHm: boolean;

  if (normalized.startsWith(TM_HM_PREFIX)) {
    const suffix = normalized.substring(TM_HM_PREFIX.length);
    const match = suffix.match(/^\d+/);
    if (!match) {
      throw new TMHMResolutionError(`${itemName} is missing a TM/HM index`);
    }
    const index = parseInt(match[0], 10) - 1;
    if (index < 0) {
      throw new TMHMResolutionError(`Invalid TM/HM index in ${itemName}`);
    }
    if (index >= TMHM_MOVES.length) {
      throw new TMHMResolutionError(
        `TM/HM index ${index} derived from ${itemName} is out of range`
      );
    }
    isHm = index >= TM_COUNT;
    return [TMHM_MOVES[index], isHm];
  }

  if (normalized.startsWith(TM_PREFIX)) {
    offset = TM_PREFIX.length;
    isHm = false;
  } else if (normalized.startsWith(HM_PREFIX)) {
    offset = HM_PREFIX.length;
    isHm = true;
  } else {
    throw new TMHMResolutionError(`${itemName} is not a TM or HM`);
  }

  const suffix = normalized.substring(offset).replace(/^_/, "");
  let digits = "";
  for (const char of suffix) {
    if (/\d/.test(char)) {
      digits += char;
    } else {
      break;
    }
  }

  let move: MoveName | undefined;
  if (digits) {
    let index = parseInt(digits, 10) - 1;
    if (index < 0) {
      throw new TMHMResolutionError(`Invalid TM/HM index in ${itemName}`);
    }
    if (isHm) {
      index += TM_COUNT;
    }
    if (index >= TMHM_MOVES.length) {
      throw new TMHMResolutionError(
        `TM/HM index ${index} derived from ${itemName} is out of range`
      );
    }
    move = TMHM_MOVES[index];
  } else {
    const moveToken = suffix;
    if (!moveToken) {
      throw new TMHMResolutionError(`${itemName} is missing a move token`);
    }
    const cleaned = moveToken.toUpperCase();
    move = resolveMoveName(cleaned);
    if (!move) {
      const collapsed = collapseToken(cleaned);
      move = COLLAPSED_MOVE_NAMES.get(collapsed);
    }
    if (!move) {
      throw new TMHMResolutionError(`Unknown TM/HM move token ${moveToken}`);
    }
  }
  return [move!, isHm];
};

export const pokemonCanLearnTmhm = (
  pokemon: Pokemon,
  move: MoveName
): boolean => {
  const species = pokemon.species;
  const learnset = species?.tmhm_learnset ?? [];
  return learnset.includes(move);
};

export const pokemonKnowsMove = (pokemon: Pokemon, move: MoveName): boolean => {
  for (const learned of pokemon.moves) {
    if (learned && learned.name === move) {
      return true;
    }
  }
  return false;
};

export const emptyMoveSlots = (pokemon: Pokemon): number => {
  const current = pokemon.moves.filter((move) => move !== null);
  return Math.max(0, 4 - current.length);
};

export const taughtMovePp = (move: MoveName, moveData?: Move): number => {
  const resolvedMoveData = moveData ?? loadAllMoves()[move];
  if (!resolvedMoveData) {
    throw new Error(`Missing move data for taught TM/HM move ${move}`);
  }
  return resolvedMoveData.pp;
};

export const learnMove = (
  pokemon: Pokemon,
  move: MoveName,
  moveData?: Move,
  replaceIndex?: number
): void => {
  const learnt: LearnedMove = {
    name: move,
    current_pp: taughtMovePp(move, moveData),
    pp_ups: 0,
  };

  const currentMoves = [...pokemon.moves];

  if (replaceIndex !== undefined) {
    if (replaceIndex < 0 || replaceIndex >= currentMoves.length) {
      throw new RangeError("replaceIndex is out of range for the known move list");
    }
    currentMoves[replaceIndex] = learnt;
  } else {
    if (currentMoves.length >= 4) {
      throw new Error("Cannot learn a move, all move slots are full and none were chosen to be replaced.");
    }
    currentMoves.push(learnt);
  }

  pokemon.moves = currentMoves;
};

export function* iterMoveNames(pokemon: Pokemon): Iterable<string> {
  for (const entry of pokemon.moves) {
    if (entry === null) {
      continue;
    }
    yield entry.name.replace(/_/g, " ");
  }
}

export const tmhmIndex = (itemName: string): number => {
  const [move] = resolveTmhmMove(itemName);
  const index = TMHM_MOVES.indexOf(move);
  if (index === -1) {
    throw new TMHMResolutionError(`${itemName} does not map to a TM/HM entry`);
  }
  return index;
};

export const tmhmItemName = (index: number): string => {
  if (index < 0 || index >= TMHM_MOVES.length) {
    throw new TMHMResolutionError(`TM/HM index ${index} is out of range`);
  }
  return `TM_HM_${(index + 1).toString().padStart(2, "0")}`;
};

export const isHmIndex = (index: number): boolean => {
  return index >= TM_COUNT;
};

export const isHmMove = (move: MoveName): boolean => {
  const index = TMHM_MOVES.indexOf(move);
  if (index === -1) {
    return false;
  }
  return isHmIndex(index);
};

export const consumeTmhmItem = (
  itemSystem: ItemSystem,
  itemName: string,
  { isHm }: { isHm: boolean }
): void => {
  if (isHm) {
    return;
  }
  if (!itemSystem.removeItem(itemName)) {
    throw new Error(`Failed to consume ${itemName}`);
  }
};
