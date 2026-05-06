import { z } from "zod";
import { Pokemon, PokemonSchema } from "./pokemon";
import { PARTY_SIZE } from "../constants";

export const PartySchema = z.object({
  pokemon: z
    .array(PokemonSchema.nullable())
    .length(PARTY_SIZE)
    .default(Array(PARTY_SIZE).fill(null)),
});
export type Party = z.infer<typeof PartySchema>;

export function getFilledSlots(party: Party): number {
  return party.pokemon.filter((p) => p !== null).length;
}

export function hasSpace(party: Party): boolean {
  return getFilledSlots(party) < PARTY_SIZE;
}

export function getNextOpenSlot(party: Party): number | null {
  const index = party.pokemon.findIndex((p) => p === null);
  return index === -1 ? null : index;
}

export function addPokemon(party: Party, newPokemon: Pokemon): boolean {
  const slot = getNextOpenSlot(party);
  if (slot === null) {
    return false;
  }
  party.pokemon[slot] = newPokemon;
  return true;
}
