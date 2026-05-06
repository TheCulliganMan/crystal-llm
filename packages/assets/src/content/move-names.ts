import { MoveName } from "@pokecrystal/core/core/enums/move";
import {
  asmMoveNamesLoader,
  moveNameForId,
} from "@pokecrystal/core/core/asm-move-names-loader";

type MoveDisplayInput = MoveName | string | number;

const SEPARATOR_PATTERN = /[\s-]+/g;

function resolveMoveName(move: MoveDisplayInput): MoveName {
  if (typeof move === "number") {
    const resolved = moveNameForId(move);
    if (resolved) {
      return resolved;
    }
    throw new Error(`Unknown move id '${move}'.`);
  }

  if (typeof move !== "string") {
    throw new Error(`Invalid move type: ${typeof move}`);
  }

  const trimmed = move.trim().toUpperCase();
  const normalizedCandidates = [
    trimmed,
    trimmed.replace(SEPARATOR_PATTERN, "_"),
    trimmed.replace(SEPARATOR_PATTERN, ""),
  ];

  for (const candidate of normalizedCandidates) {
    if (candidate in MoveName) {
      return candidate as MoveName;
    }
  }

  throw new Error(`Unknown move '${move}'.`);
}

export function moveDisplayName(move: MoveDisplayInput): string {
  const key = resolveMoveName(move);
  const result = asmMoveNamesLoader.get(key);
  if (result) {
    return result;
  }
  return key.replace(/_/g, " ");
}
