import { BattleScene, BattleTurn, MoveName, StatusCondition } from "../../../core/enums";
import { Event, EventManager } from "../../events/events";
import { Pokemon } from "../../../core/models/pokemon";
import { Battle } from "./battle-logic";
import { HardwareRNG } from "../../games/rng";
import { calculateDamage } from "./damage-calculation";
import { movesMap } from "../../../core/data-loader";
import { resetBattleStatStages } from "./stat-stages";

export function moveIsDisabled(pokemon: Pokemon, moveName: MoveName): boolean {
    return Boolean(
        moveName &&
        pokemon.disable_turns > 0 &&
        pokemon.disabled_move === moveName
    );
}

export function tickDisable(battle: Battle, pokemon: Pokemon): void {
    if (!pokemon.disabled_move || pokemon.disable_turns <= 0) {
        if (pokemon.disable_turns <= 0) {
            pokemon.disabled_move = undefined;
            pokemon.disable_turns = 0;
        }
        return;
    }

    pokemon.disable_turns = Math.max(0, pokemon.disable_turns - 1);
    if (pokemon.disable_turns > 0) {
        return;
    }

    pokemon.disabled_move = undefined;
    battle.eventManager.dispatch(
        new Event("show_text", { text: `${pokemon.nickname}'s disabled no more!` })
    );
}

export function expireDestinyBond(battle: Battle, pokemon: Pokemon): void {
    if (!pokemon.destiny_bond_active) {
        return;
    }
    if (pokemon.destiny_bond_action_id === undefined) {
        return;
    }
    if (pokemon.destiny_bond_action_id < battle._actionCounter) {
        pokemon.destiny_bond_active = false;
        pokemon.destiny_bond_action_id = undefined;
    }
}

export function endEncore(eventManager: EventManager, pokemon: Pokemon, showMessage: boolean = true): void {
    if (pokemon.encore_turns_remaining <= 0 && pokemon.encored_move === undefined) {
        return;
    }
    pokemon.encore_turns_remaining = 0;
    pokemon.encored_move = undefined;
    if (showMessage) {
        eventManager.dispatch(
            new Event("show_text", { text: `${pokemon.nickname}'s ENCORE ended!` })
        );
    }
}

export function triggerDestinyBond(battle: Battle, attacker_key: BattleTurn, attacker: Pokemon, defender: Pokemon): void {
    defender.destiny_bond_active = false;
    defender.destiny_bond_action_id = undefined;
    battle.eventManager.dispatch(
        new Event("show_text", { text: `${defender.nickname} took down with it, ${attacker.nickname}!` })
    );
    if (battle.gameState.sram.options.battle_scene) {
        battle.eventManager.dispatch(
            new Event("play_animation", {
                move_name: MoveName.DESTINY_BOND,
                is_player_move: defender === battle.context.playerPokemon,
            })
        );
    }
    attacker.hp = 0;
}

export function attackerCannotMove(battle: Battle, attacker: Pokemon): boolean {
    if (attacker.status === StatusCondition.SLEEP) {
        attacker.sleep_turns = Math.max(0, (attacker.sleep_turns ?? 0) - 1);
        if (attacker.sleep_turns <= 0) {
            attacker.status = undefined;
            attacker.sleep_turns = 0;
            attacker.nightmare = false;
            battle.eventManager.dispatch(
                new Event("show_text", { text: `${attacker.nickname} woke up!` })
            );
            return false;
        }
        battle.eventManager.dispatch(
            new Event("show_text", { text: `${attacker.nickname} is fast asleep!` })
        );
        return true;
    }

    if (attacker.status === StatusCondition.FREEZE) {
        battle.eventManager.dispatch(
            new Event("show_text", { text: `${attacker.nickname} is frozen solid!` })
        );
        return true;
    }
    if (attacker.flinching) {
        attacker.flinching = false;
        battle.eventManager.dispatch(
            new Event("show_text", { text: `${attacker.nickname} flinched!` })
        );
        return true;
    }

    tickDisable(battle, attacker);

    const rng = new HardwareRNG(battle.gameState);
    if (attacker.status === StatusCondition.PARALYSIS && rng.coinFlip(0.25)) {
        battle.eventManager.dispatch(
            new Event("show_text", { text: `${attacker.nickname} is fully paralyzed!` })
        );
        return true;
    }

    if (attacker.attract_source_side !== undefined) {
        const sourceSide = attacker.attract_source_side;
        const sourcePokemon =
            sourceSide === BattleTurn.PLAYER ? battle.context.playerPokemon : battle.context.enemyPokemon;
        if (sourcePokemon.hp <= 0) {
            attacker.attract_source_side = undefined;
        } else if (rng.coinFlip(0.5)) {
            battle.eventManager.dispatch(
                new Event("show_text", { text: `${attacker.nickname} is in love!` })
            );
            battle.eventManager.dispatch(
                new Event("show_text", { text: `${attacker.nickname} is immobilized by love!` })
            );
            return true;
        }
    }

    if (attacker.trapped_turns > 0) {
        const moveName = attacker.trapped_move
            ? attacker.trapped_move.replace(/_/g, " ").replace(/\w\S*/g, (txt) => txt.charAt(0).toUpperCase() + txt.substr(1).toLowerCase())
            : "the attack";
        battle.eventManager.dispatch(
            new Event("show_text", { text: `${attacker.nickname} is trapped by ${moveName}!` })
        );
        return true;
    }

    return false;
}

export function resolveConfusion(battle: Battle, attacker: Pokemon): boolean {
    if ((attacker.confusion_turns ?? 0) <= 0) {
        if (attacker.status === StatusCondition.CONFUSION) {
            attacker.status = undefined;
        }
        return false;
    }

    // In Gen 2, the confusion counter is decremented at the start of the turn.
    // If it reaches zero, the Pokemon snaps out of it immediately.
    // See pokecrystal disassembly: engine/battle/effects/confusion.asm
    attacker.confusion_turns -= 1;
    if (attacker.confusion_turns <= 0) {
        if (attacker.status === StatusCondition.CONFUSION) {
            attacker.status = undefined;
        }
        attacker.confusion_turns = 0;
        battle.eventManager.dispatch(
            new Event("show_text", { text: `${attacker.nickname} snapped out of confusion!` })
        );
        return false;
    }

    const rng = new HardwareRNG(battle.gameState);
    if (rng.coinFlip(0.5)) {
        battle.eventManager.dispatch(
            new Event("show_text", { text: `${attacker.nickname} is confused!` })
        );
        const poundMove = movesMap.get(MoveName.POUND);
        if(!poundMove){
            throw new Error("Pound move not found")
        }
        const damage = calculateDamage(
            attacker,
            attacker,
            poundMove,
            battle.context,
            false,
            true,
            battle.gameState,
        ).damage;
        attacker.hp -= damage;
        return true;
    }

    return false;
}

export function clearTransientStatus(pokemon: Pokemon): void {
    resetBattleStatStages(pokemon);
    pokemon.encore_turns_remaining = 0;
    pokemon.encored_move = undefined;
    pokemon.disable_turns = 0;
    pokemon.disabled_move = undefined;
    pokemon.bide_active = false;
    pokemon.bide_turns_remaining = 0;
    pokemon.bide_damage = 0;
    pokemon.rollout_active = false;
    pokemon.rollout_step = 0;
    pokemon.rage_active = false;
    pokemon.rage_counter = 0;
    pokemon.fury_cutter_count = 0;
    pokemon.destiny_bond_active = false;
    pokemon.destiny_bond_action_id = undefined;
    pokemon.locked_move = undefined;
    pokemon.locked_turns_remaining = 0;
    pokemon.defense_curled = false;
}
