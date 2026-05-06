import { Pokemon } from "../../../core/models/pokemon";
import { BattleTurn, PokemonType, StatusCondition, MoveName } from "../../../core/enums";
import { BattleContext, Weather } from "./battle-context";
import { Event, EventManager } from "../../events/events";

export interface ResidualOutcome {
    player_fainted: boolean;
    enemy_fainted: boolean;
}

const SIDES: [BattleTurn, BattleTurn] = [BattleTurn.PLAYER, BattleTurn.ENEMY];

function ownerPrefix(side: BattleTurn): string {
    return side === BattleTurn.PLAYER ? "Your" : "Enemy";
}

export function resolveEndOfTurnEffects(
    context: BattleContext,
    event_manager: EventManager
): ResidualOutcome {
    const outcome: ResidualOutcome = { player_fainted: false, enemy_fainted: false };
    mergeOutcome(outcome, applyWeatherEffects(context, event_manager));
    mergeOutcome(outcome, applyLeechSeed(context, event_manager));
    mergeOutcome(outcome, applyPoisonAndBurn(context, event_manager));
    mergeOutcome(outcome, applyNightmare(context, event_manager));
    mergeOutcome(outcome, applyCurse(context, event_manager));
    mergeOutcome(outcome, applyPartialTrap(context, event_manager));
    mergeOutcome(outcome, applyLeftovers(context, event_manager));
    tickSafeguard(context, event_manager);
    tickScreens(context, event_manager);
    resetTurnFlags(context);
    return outcome;
}

function mergeOutcome(outcome: ResidualOutcome, fainted: Record<BattleTurn, boolean>): void {
    if (fainted[BattleTurn.PLAYER]) {
        outcome.player_fainted = true;
    }
    if (fainted[BattleTurn.ENEMY]) {
        outcome.enemy_fainted = true;
    }
}

function pokemonFor(context: BattleContext, side: BattleTurn): Pokemon {
    return side === BattleTurn.PLAYER ? context.playerPokemon : context.enemyPokemon;
}

function resetTurnFlags(context: BattleContext): void {
    const protectMoves = new Set([MoveName.PROTECT, MoveName.DETECT, MoveName.ENDURE]);
    for (const side of SIDES) {
        const pokemon = pokemonFor(context, side);
        pokemon.flinching = false;

        const lastMoveName = pokemon.last_move_used;
        if (!lastMoveName || !protectMoves.has(lastMoveName)) {
            pokemon.protect_counter = 0;
            pokemon.endure_counter = 0;
        }

        pokemon.protect_active = false;
        pokemon.endure_active = false;
    }
}

function applyWeatherEffects(
    context: BattleContext,
    event_manager: EventManager
): Record<BattleTurn, boolean> {
    const fainted: Record<BattleTurn, boolean> = { [BattleTurn.PLAYER]: false, [BattleTurn.ENEMY]: false };
    const weather = context.weather;
    if (weather === Weather.NORMAL) {
        return fainted;
    }

    const weatherMessages: { [key in Weather]?: string } = {
        [Weather.RAIN]: "Rain continues to fall.",
        [Weather.SUN]: "The sunlight is strong.",
        [Weather.SANDSTORM]: "The sandstorm rages.",
    };
    const message = weatherMessages[weather];
    if (message) {
        event_manager.dispatch(new Event("show_text", { text: message }));
    }

    if (weather === Weather.SANDSTORM) {
        for (const side of SIDES) {
            const pokemon = pokemonFor(context, side);
            if (isSandstormImmune(pokemon) || pokemon.hp <= 0) {
                continue;
            }
            const damage = Math.max(1, Math.floor(pokemon.max_hp / 16));
            pokemon.hp = Math.max(0, pokemon.hp - damage);
            event_manager.dispatch(new Event("show_text", { text: `${pokemon.nickname} is buffeted by the sandstorm!` }));
            if (pokemon.hp === 0) {
                fainted[side] = true;
            }
        }
    }

    context.weatherTurns = Math.max(0, context.weatherTurns - 1);
    if (context.weatherTurns === 0) {
        context.weather = Weather.NORMAL;
        event_manager.dispatch(new Event("show_text", { text: "The weather returned to normal." }));
    }

    return fainted;
}

function isSandstormImmune(pokemon: Pokemon): boolean {
    const immuneTypes = new Set<PokemonType>([PokemonType.ROCK, PokemonType.GROUND, PokemonType.STEEL]);
    return immuneTypes.has(pokemon.species.type1) || (pokemon.species.type2 ? immuneTypes.has(pokemon.species.type2) : false);
}

function applyLeechSeed(
    context: BattleContext,
    event_manager: EventManager
): Record<BattleTurn, boolean> {
    const fainted: Record<BattleTurn, boolean> = { [BattleTurn.PLAYER]: false, [BattleTurn.ENEMY]: false };
    for (const side of SIDES) {
        const pokemon = pokemonFor(context, side);
        if (!pokemon.leech_seeded || pokemon.hp <= 0) {
            continue;
        }

        const damage = Math.max(1, Math.floor(pokemon.max_hp / 8));
        pokemon.hp = Math.max(0, pokemon.hp - damage);
        event_manager.dispatch(new Event("show_text", { text: `${pokemon.nickname}'s health is sapped by Leech Seed!` }));
        if (pokemon.hp === 0) {
            fainted[side] = true;
        }

        const sourceSide = pokemon.leech_seed_source_side;
        if (sourceSide !== undefined) {
            const recipient = pokemonFor(context, sourceSide);
            if (recipient.hp > 0 && recipient.hp < recipient.max_hp) {
                recipient.hp = Math.min(recipient.max_hp, recipient.hp + damage);
                event_manager.dispatch(new Event("show_text", { text: `${recipient.nickname} absorbed nutrients!` }));
            }
        }
    }
    return fainted;
}

function applyPoisonAndBurn(
    context: BattleContext,
    event_manager: EventManager
): Record<BattleTurn, boolean> {
    const fainted: Record<BattleTurn, boolean> = { [BattleTurn.PLAYER]: false, [BattleTurn.ENEMY]: false };
    for (const side of SIDES) {
        const pokemon = pokemonFor(context, side);
        if (pokemon.hp <= 0 || (pokemon.status !== StatusCondition.POISON && pokemon.status !== StatusCondition.BURN)) {
            continue;
        }

        const statusName = pokemon.status!;
        event_manager.dispatch(new Event("show_text", { text: `${pokemon.nickname} is hurt by ${statusName}!` }));
        const damage = Math.max(1, Math.floor(pokemon.max_hp / 8));
        pokemon.hp = Math.max(0, pokemon.hp - damage);
        if (pokemon.hp === 0) {
            fainted[side] = true;
        }
    }
    return fainted;
}

function applyNightmare(
    context: BattleContext,
    event_manager: EventManager
): Record<BattleTurn, boolean> {
    const fainted: Record<BattleTurn, boolean> = { [BattleTurn.PLAYER]: false, [BattleTurn.ENEMY]: false };
    for (const side of SIDES) {
        const pokemon = pokemonFor(context, side);
        if (!pokemon.nightmare || pokemon.hp <= 0) {
            continue;
        }

        if (pokemon.status !== StatusCondition.SLEEP) {
            pokemon.nightmare = false;
            continue;
        }

        const damage = Math.max(1, Math.floor(pokemon.max_hp / 4));
        pokemon.hp = Math.max(0, pokemon.hp - damage);
        event_manager.dispatch(new Event("show_text", { text: `${pokemon.nickname} is locked in a nightmare!` }));
        if (pokemon.hp === 0) {
            fainted[side] = true;
        }
    }
    return fainted;
}

function applyCurse(
    context: BattleContext,
    event_manager: EventManager
): Record<BattleTurn, boolean> {
    const fainted: Record<BattleTurn, boolean> = { [BattleTurn.PLAYER]: false, [BattleTurn.ENEMY]: false };
    for (const side of SIDES) {
        const pokemon = pokemonFor(context, side);
        if (!pokemon.cursed || pokemon.hp <= 0) {
            continue;
        }
        const damage = Math.max(1, Math.floor(pokemon.max_hp / 4));
        pokemon.hp = Math.max(0, pokemon.hp - damage);
        event_manager.dispatch(new Event("show_text", { text: `${pokemon.nickname} is afflicted by the curse!` }));
        if (pokemon.hp === 0) {
            fainted[side] = true;
        }
    }
    return fainted;
}

function applyPartialTrap(
    context: BattleContext,
    event_manager: EventManager
): Record<BattleTurn, boolean> {
    const fainted: Record<BattleTurn, boolean> = { [BattleTurn.PLAYER]: false, [BattleTurn.ENEMY]: false };
    for (const side of SIDES) {
        const pokemon = pokemonFor(context, side);
        if (pokemon.trapped_turns <= 0 || pokemon.hp <= 0) {
            continue;
        }

        const sourceSide = pokemon.trapped_by_side;
        const sourceIndex = pokemon.trapped_source_index;
        if (sourceSide !== undefined && sourceIndex !== undefined) {
            if (context.activeIndexFor(sourceSide) !== sourceIndex) {
                releaseTrap(pokemon, event_manager);
                continue;
            }
            const sourceParty = context.partyFor(sourceSide);
            if (sourceIndex >= 0 && sourceIndex < sourceParty.length) {
                const sourcePokemon = sourceParty[sourceIndex];
                if (sourcePokemon.hp <= 0) {
                    releaseTrap(pokemon, event_manager);
                    continue;
                }
            }
        }

        const textMove = pokemon.trapped_move ? pokemon.trapped_move.replace(/_/g, " ") : "the attack";
        event_manager.dispatch(new Event("show_text", { text: `${pokemon.nickname} is hurt by ${textMove}!` }));
        const damage = Math.max(1, Math.floor(pokemon.max_hp / 16));
        pokemon.hp = Math.max(0, pokemon.hp - damage);
        pokemon.trapped_turns = Math.max(0, pokemon.trapped_turns - 1);
        if (pokemon.trapped_turns === 0) {
            releaseTrap(pokemon, event_manager);
        }
        if (pokemon.hp === 0) {
            fainted[side] = true;
        }
    }
    return fainted;
}

function releaseTrap(pokemon: Pokemon, event_manager: EventManager): void {
    if (pokemon.hp > 0) {
        event_manager.dispatch(new Event("show_text", { text: `${pokemon.nickname} was freed from the bind!` }));
    }
    pokemon.trapped_turns = 0;
    pokemon.trapped_by_side = undefined;
    pokemon.trapped_source_index = undefined;
    pokemon.trapped_move = undefined;
}

function applyLeftovers(
    context: BattleContext,
    event_manager: EventManager
): Record<BattleTurn, boolean> {
    const fainted: Record<BattleTurn, boolean> = { [BattleTurn.PLAYER]: false, [BattleTurn.ENEMY]: false };
    for (const side of SIDES) {
        const pokemon = pokemonFor(context, side);
        if (pokemon.hp <= 0 || pokemon.item !== "LEFTOVERS" || pokemon.hp >= pokemon.max_hp) {
            continue;
        }
        const heal = Math.max(1, Math.floor(pokemon.max_hp / 16));
        pokemon.hp = Math.min(pokemon.max_hp, pokemon.hp + heal);
        event_manager.dispatch(new Event("show_text", { text: `${pokemon.nickname} regained health with Leftovers!` }));
    }
    return fainted;
}

function tickSafeguard(context: BattleContext, event_manager: EventManager): void {
    for (const side of SIDES) {
        if (context.barrierTurns(side, "safeguard") <= 0) {
            continue;
        }
        const remaining = context.tickBarrier(side, "safeguard");
        if (remaining === 0) {
            const owner = ownerPrefix(side);
            event_manager.dispatch(new Event("show_text", { text: `${owner} Pokémon's SAFEGUARD faded!` }));
        }
    }
}

function tickScreens(context: BattleContext, event_manager: EventManager): void {
    for (const side of SIDES) {
        if (context.barrierTurns(side, "light_screen") > 0) {
            const remaining = context.tickBarrier(side, "light_screen");
            if (remaining === 0) {
                const owner = ownerPrefix(side);
                event_manager.dispatch(new Event("show_text", { text: `${owner} Pokémon's LIGHT SCREEN fell!` }));
            }
        }
        if (context.barrierTurns(side, "reflect") > 0) {
            const remaining = context.tickBarrier(side, "reflect");
            if (remaining === 0) {
                const owner = ownerPrefix(side);
                event_manager.dispatch(new Event("show_text", { text: `${owner} Pokémon's REFLECT faded!` }));
            }
        }
    }
}
