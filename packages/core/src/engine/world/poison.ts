
import { StatusCondition } from "@pokecrystal/core/core/enums/battle";
import type { PokemonData } from "@pokecrystal/core/core/models";

export interface PoisonDamageResult {
    damagedNames: string[];
    faintedNames: string[];
}

const DEFAULT_POISON_NAME = "POKéMON";

function normalizePoisonStatus(status: unknown): string | null {
    if (!status) {
        return null;
    }

    const normalized = (value: unknown): string | null => {
        if (typeof value !== "string") {
            return null;
        }

        const token = value.trim().toLowerCase();
        if (
          token === StatusCondition.POISON ||
          token === "poisoned" ||
          token === "psn" ||
          token === "badly poisoned" ||
          token === "toxic"
        ) {
            return StatusCondition.POISON;
        }
        return null;
    };

    if (typeof status === "string") {
        return normalized(status);
    }

    if (typeof status === "number") {
        return status === 1 ? StatusCondition.POISON : null;
    }

    if (typeof status === "object" && "name" in status) {
        return normalized((status as { name?: unknown }).name);
    }

    return null;
}

export function isPoisoned(status: unknown): status is StatusCondition.POISON {
    return normalizePoisonStatus(status) === StatusCondition.POISON;
}

function poisonDamageName(pokemon: PokemonData): string {
    const nickname = (pokemon.nickname ?? "").trim();
    if (nickname.length) {
        return nickname.toUpperCase();
    }
    const speciesId = pokemon.species?.id ?? "";
    if (speciesId.length) {
        return speciesId.replace(/_/g, " ").toUpperCase();
    }
    return DEFAULT_POISON_NAME;
}

// ASM mapping: pokecrystal_disassembly/engine/events/poisonstep.asm (DamageMonIfPoisoned)
export function applyPoisonToParty(party: (PokemonData | null)[]): PoisonDamageResult {
    const damagedNames: string[] = [];
    const faintedNames: string[] = [];

    for (const pokemon of party) {
        if (!pokemon || !isPoisoned(pokemon.status)) {
            continue;
        }
        if ((pokemon.hp ?? 0) <= 0) {
            continue;
        }
        pokemon.hp = Math.max((pokemon.hp ?? 0) - 1, 0);
        if ((pokemon.hp ?? 0) === 0) {
            pokemon.status = undefined;
            faintedNames.push(poisonDamageName(pokemon));
        } else {
            damagedNames.push(poisonDamageName(pokemon));
        }
    }

    return { damagedNames, faintedNames };
}
