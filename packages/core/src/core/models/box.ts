import { z } from "zod";
import { Pokemon, PokemonData, PokemonSchema, toPokemon } from "./pokemon";
import { MAX_BOX_MONS } from "../constants";

type BoxMetadataViolation = {
  path: (string | number)[];
  message: string;
};

type PokemonDraft = Pokemon | PokemonData;

type BoxDraft = {
  pokemon: Array<PokemonDraft | null>;
  nicknames: string[];
  original_trainer_names: string[];
  original_trainer_ids: number[];
  count: number;
  slot_species: number[];
};

const collectBoxMetadataViolations = (box: BoxDraft): BoxMetadataViolation[] => {
  const violations: BoxMetadataViolation[] = [];

  if (!Number.isFinite(box.count) || !Number.isInteger(box.count)) {
    violations.push({ path: ["count"], message: `count must be an integer (${box.count})` });
  }
  if (box.count < 0 || box.count > MAX_BOX_MONS) {
    violations.push({ path: ["count"], message: `count must be between 0 and ${MAX_BOX_MONS} (${box.count})` });
  }

  if (box.pokemon.length !== MAX_BOX_MONS) {
    violations.push({ path: ["pokemon"], message: `pokemon must contain exactly ${MAX_BOX_MONS} slots` });
  }
  if (box.nicknames.length !== MAX_BOX_MONS) {
    violations.push({ path: ["nicknames"], message: `nicknames must contain exactly ${MAX_BOX_MONS} slots` });
  }
  if (box.original_trainer_names.length !== MAX_BOX_MONS) {
    violations.push({ path: ["original_trainer_names"], message: `original_trainer_names must contain exactly ${MAX_BOX_MONS} slots` });
  }
  if (box.original_trainer_ids.length !== MAX_BOX_MONS) {
    violations.push({ path: ["original_trainer_ids"], message: `original_trainer_ids must contain exactly ${MAX_BOX_MONS} slots` });
  }
  if (box.slot_species.length !== MAX_BOX_MONS + 1) {
    violations.push({ path: ["slot_species"], message: `slot_species must contain exactly ${MAX_BOX_MONS + 1} slots` });
  }

  const filledSlots = box.pokemon.filter((pokemon) => pokemon !== null).length;
  if (box.count !== filledSlots) {
    violations.push({
      path: ["count"],
      message: `count (${box.count}) must match filled pokemon slots (${filledSlots})`,
    });
  }

  const slotCount = Math.min(
    MAX_BOX_MONS,
    box.pokemon.length,
    box.nicknames.length,
    box.original_trainer_names.length,
    box.original_trainer_ids.length,
    box.slot_species.length,
  );
  for (let index = 0; index < slotCount; index += 1) {
    const pokemon = box.pokemon[index];
    if (pokemon === null) {
      if (box.nicknames[index] !== "") {
        violations.push({ path: ["nicknames", index], message: `empty slot ${index} must have empty nickname` });
      }
      if (box.original_trainer_names[index] !== "") {
        violations.push({
          path: ["original_trainer_names", index],
          message: `empty slot ${index} must have empty original_trainer_name`,
        });
      }
      if (box.original_trainer_ids[index] !== 0) {
        violations.push({
          path: ["original_trainer_ids", index],
          message: `empty slot ${index} must have original_trainer_id 0`,
        });
      }
      if (box.slot_species[index] !== 0) {
        violations.push({
          path: ["slot_species", index],
          message: `empty slot ${index} must have slot_species 0`,
        });
      }
      continue;
    }

    if (box.slot_species[index] !== pokemon.species.int_id) {
      violations.push({
        path: ["slot_species", index],
        message: `slot_species ${box.slot_species[index]} must match ${pokemon.species.id} (${pokemon.species.int_id})`,
      });
    }
  }

  return violations;
};

export const validateBoxState = (box: BoxDraft, label = "pc_box"): void => {
  const violations = collectBoxMetadataViolations(box);
  if (violations.length > 0) {
    const details = violations.map((violation) => `
  - ${violation.path.join('.')}: ${violation.message}`).join("");
    throw new Error(`Invalid ${label}: ${details}`);
  }
};

export const formatDefaultBoxName = (index: number): string =>
  `BOX ${String(index + 1).padStart(2, "0")}`;

export const BoxSchema = z.object({
  name: z.string(),
  pokemon: z
    .array(PokemonSchema.nullable())
    .length(MAX_BOX_MONS)
    .default(Array(MAX_BOX_MONS).fill(null)),
  nicknames: z
    .array(z.string())
    .length(MAX_BOX_MONS)
    .default(Array(MAX_BOX_MONS).fill("")),
  original_trainer_names: z
    .array(z.string())
    .length(MAX_BOX_MONS)
    .default(Array(MAX_BOX_MONS).fill("")),
  original_trainer_ids: z
    .array(z.number())
    .length(MAX_BOX_MONS)
    .default(Array(MAX_BOX_MONS).fill(0)),
  count: z.number().default(0),
  slot_species: z
    .array(z.number())
    .length(MAX_BOX_MONS + 1)
    .default(Array(MAX_BOX_MONS + 1).fill(0)),
}).superRefine((box, context) => {
  const violations = collectBoxMetadataViolations(box);
  violations.forEach((violation) => {
    context.addIssue({
      code: z.ZodIssueCode.custom,
      path: violation.path,
      message: violation.message,
    });
  });
});
export type Box = z.infer<typeof BoxSchema>;

export function getFilledSlots(box: Box): number {
  return box.pokemon.filter((p) => p !== null).length;
}

export function hasSpace(box: Box): boolean {
  return getFilledSlots(box) < MAX_BOX_MONS;
}

export function getNextOpenSlot(box: Box): number | null {
  const index = box.pokemon.findIndex((p) => p === null);
  return index === -1 ? null : index;
}

export function addPokemon(box: Box, newPokemon: Pokemon): boolean {
  const slot = getNextOpenSlot(box);
  if (slot === null) {
    return false;
  }
  setSlot(box, slot, newPokemon);
  return true;
}

export function setSlot(box: Box, index: number, pokemon: Pokemon | PokemonData | null): void {
  if (index < 0 || index >= MAX_BOX_MONS) {
    throw new Error(`Box slot ${index} is out of range`);
  }
  validateBoxState(box, "pc_box (before setSlot)");
  const nextPokemon = pokemon ? toPokemonIfNeeded(pokemon) : null;
  const previous = box.pokemon[index];
  const previousPokemon = previous ? toPokemon(previous) : null;
  box.pokemon[index] = nextPokemon;
  setSlotMetadata(box, index, nextPokemon, previousPokemon);
  validateBoxState(box, "pc_box (after setSlot)");
}

export function clearSlot(box: Box, index: number): void {
  setSlot(box, index, null);
}

function setSlotMetadata(
  box: Box,
  index: number,
  pokemon: Pokemon | null,
  previous: Pokemon | null
): void {
  writeSlotMetadataFields(box, index, pokemon);
  const delta = (pokemon !== null ? 1 : 0) - (previous !== null ? 1 : 0);
  box.count += delta;
  clampCount(box);
}

function writeSlotMetadataFields(
  box: Box,
  index: number,
  pokemon: Pokemon | null
): void {
  if (pokemon === null) {
    box.nicknames[index] = "";
    box.original_trainer_names[index] = "";
    box.original_trainer_ids[index] = 0;
    setSlotSpecies(box, index, null);
    return;
  }

  box.nicknames[index] = pokemon.nickname || "";
  box.original_trainer_names[index] = pokemon.original_trainer_name || "";
  box.original_trainer_ids[index] = pokemon.original_trainer_id;
  setSlotSpecies(box, index, pokemon);
}

function setSlotSpecies(
  box: Box,
  index: number,
  pokemon: Pokemon | null
): void {
  box.slot_species[index] = pokemon?.species.int_id ?? 0;
}

function toPokemonIfNeeded(pokemon: PokemonData | Pokemon): Pokemon {
  if ("_calculateStat" in pokemon && "_statExpForStat" in pokemon) {
    return pokemon as Pokemon;
  }
  return toPokemon(pokemon);
}

function clampCount(box: Box): void {
  box.count = Math.max(0, Math.min(box.count, MAX_BOX_MONS));
}
