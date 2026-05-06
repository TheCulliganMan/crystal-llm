import type { Move, Pokemon } from "@pokecrystal/core/core/models";
import type { MoveName } from "@pokecrystal/core/core/enums";
import { learnMove, pokemonCanLearnTmhm, pokemonKnowsMove } from "@pokecrystal/core/engine/systems/tmhm";

// ASM mapping: engine/items/tmhm.asm (AskTeachTMHM, TeachTMHM).

export type TMHMLearningResult = {
  success: boolean;
  reason?: string | null;
  replacedIndex?: number | null;
};

export type TMHMForgetPrompt = (pokemon: Pokemon, move: MoveName) => number | null;

export class TMHMMoveTeacher {
  constructor(private promptForget?: TMHMForgetPrompt) {}

  public teach(
    pokemon: Pokemon,
    move: MoveName,
    moveData?: Move,
  ): TMHMLearningResult {
    if (this.isEgg(pokemon)) {
      return { success: false, reason: "egg" };
    }
    if (pokemonKnowsMove(pokemon, move)) {
      return { success: false, reason: "already_known" };
    }
    if (!pokemonCanLearnTmhm(pokemon, move)) {
      return { success: false, reason: "incompatible" };
    }

    const moves = pokemon.moves.filter((entry) => entry !== null);
    if (moves.length < 4) {
      learnMove(pokemon, move, moveData);
      return { success: true };
    }

    let replaceIndex: number | null | undefined = null;
    if (this.promptForget) {
      replaceIndex = this.promptForget(pokemon, move);
    }
    if (replaceIndex === null || replaceIndex === undefined) {
      return { success: false, reason: "replace_required" };
    }

    learnMove(pokemon, move, moveData, replaceIndex);
    return { success: true, replacedIndex: replaceIndex };
  }

  private isEgg(pokemon: Pokemon): boolean {
    const species = pokemon.species.id.toUpperCase();
    const nickname = pokemon.nickname.toUpperCase();
    return species === "EGG" || nickname === "EGG";
  }
}
