import { BattleTurn, Stat } from "../../../core/enums";
import { Pokemon } from "../../../core/models/pokemon";
import { BattleContext } from "./battle-context";
import { applyStage } from "./stats";
import { modifyStat } from "./move-effects";
import { HardwareRNG } from "../../games/rng";
import { Event, EventManager } from "../../events/events";
import { GameState } from "../../../core/state";
import { itemsMap } from "../../../core/data-loader";

export function activateBerserkGene(
    context: BattleContext,
    event_manager: EventManager,
): void {
    const pokemons: [BattleTurn, Pokemon][] = [
        [BattleTurn.PLAYER, context.playerPokemon],
        [BattleTurn.ENEMY, context.enemyPokemon],
    ];

    for (const [side, pokemon] of pokemons) {
        if (pokemon.hp <= 0) {
            continue;
        }

        const itemKey = pokemon.item;
        if (itemKey !== "BERSERK_GENE") {
            continue;
        }

        const item = itemsMap.get(itemKey || "");
        const rawName = item ? item.name : (itemKey || "BERSERK_GENE");
        const displayName = rawName.replace(/_/g, " ");

        pokemon.item = undefined;

        event_manager.dispatch(
            new Event(
                "show_text",
                { text: `${pokemon.nickname}'s ${displayName} activated!` }
            )
        );
        modifyStat(pokemon, Stat.ATTACK, 2, event_manager);

        if ((pokemon.confusion_turns ?? 0) <= 0) {
            const rng = new HardwareRNG(event_manager.gameState);
            pokemon.confusion_turns = rng.randrange(4) + 2;
            event_manager.dispatch(
                new Event("show_text", { text: `${pokemon.nickname} became confused!` })
            );
        }
    }
}

function _clamp_stat(value: number): number {
    return Math.max(1, Math.min(value, 999));
}

export function calculateFutureSightDamage(
    attacker: Pokemon,
    defender: Pokemon,
    game_state: GameState,
    predefined_random_value: number | null,
): number {
    const baseAttack = attacker.species.base_stats.special_attack;
    const attackStage = attacker.stat_boosts[Stat.SPECIAL_ATTACK] || 0;
    const attackValue = _clamp_stat(applyStage(baseAttack, attackStage));

    const baseDefense = defender.species.base_stats.special_defense;
    const defenseStage = defender.stat_boosts[Stat.SPECIAL_DEFENSE] || 0;
    const defenseValue = _clamp_stat(applyStage(baseDefense, defenseStage));

    const levelFactor = Math.floor((2 * attacker.level) / 5) + 2;
    let damage = Math.floor(levelFactor * 80 * attackValue);
    damage = Math.floor(damage / Math.max(1, defenseValue));
    damage = Math.floor(damage / 50) + 2;
    damage = Math.max(1, damage);

    let random_roll: number;
    if (predefined_random_value !== null) {
        const clamped = Math.max(0.0, Math.min(1.0, predefined_random_value));
        random_roll = Math.floor(217 + clamped * (255 - 217));
    } else {
        const rng = new HardwareRNG(game_state);
        random_roll = rng.randrange(39) + 217;
    }

    damage = Math.floor(damage * random_roll / 255);
    return Math.max(1, damage);
}

export function queueFutureSight(
    attacker_key: BattleTurn,
    attacker: Pokemon,
    defender: Pokemon,
    context: BattleContext,
    event_manager: EventManager,
    stored_damage: number,
): boolean {
    const targetSide = attacker_key === BattleTurn.PLAYER ? BattleTurn.ENEMY : BattleTurn.PLAYER;
    if (context.futureSightCounter(targetSide) > 0) {
        event_manager.dispatch(new Event("show_text", { text: "But it failed!" }));
        return false;
    }

    context.setFutureSightCounter(targetSide, 3);
    context.setFutureSightDamage(targetSide, stored_damage);
    event_manager.dispatch(
        new Event("show_text", { text: `${attacker.nickname} foresaw an attack!` })
    );
    return true;
}

export function tickFutureSight(
    context: BattleContext,
    event_manager: EventManager,
    side: BattleTurn,
): boolean {
    let counter = context.futureSightCounter(side);
    if (counter <= 0) {
        return false;
    }

    context.setFutureSightCounter(side, counter - 1);
    counter = context.futureSightCounter(side);
    if (counter > 0) {
        return false;
    }

    const damage = context.futureSightDamage(side);
    const target = side === BattleTurn.PLAYER ? context.playerPokemon : context.enemyPokemon;
    if (target.hp <= 0) {
        context.setFutureSightDamage(side, 0);
        return false;
    }

    event_manager.dispatch(
        new Event("show_text", { text: `${target.nickname} was hit by FUTURE SIGHT!` })
    );
    if (damage > 0) {
        target.hp = Math.max(0, target.hp - damage);
    }
    context.setFutureSightDamage(side, 0);
    context.setFutureSightCounter(side, 0);
    return target.hp === 0;
}

export function tickPerishSong(
    context: BattleContext,
    event_manager: EventManager,
    side: BattleTurn,
): boolean {
    const pokemon = side === BattleTurn.PLAYER ? context.playerPokemon : context.enemyPokemon;
    if (pokemon.hp <= 0) {
        pokemon.perish_song_turns = 0;
        return false;
    }

    if (pokemon.perish_song_turns <= 0) {
        return false;
    }

    pokemon.perish_song_turns = Math.max(0, pokemon.perish_song_turns - 1);
    event_manager.dispatch(
        new Event(
            "show_text",
            { text: `${pokemon.nickname}'s PERISH count is ${pokemon.perish_song_turns}!` }
        )
    );
    if (pokemon.perish_song_turns > 0) {
        return false;
    }

    pokemon.hp = 0;
    return true;
}
