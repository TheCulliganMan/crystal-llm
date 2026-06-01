
import type { GameState } from '@pokecrystal/core/core/state';
import { Battle } from './battle-logic';
import { Pokemon } from '../../../core/models';
import { calculateExperience } from '../../experience';
import { EventManager, Event } from '../../events/events';
import { _normalizeItemName } from './damage-calculation';
import { buildLevelQueue, type LevelUpInfo } from '../../../ui/overlays/battle-experience';

type BattleGameStateWithAutoShare = GameState & {
    auto_exp_share_enabled?: boolean;
};

export function grantPlayerExperience(battle: Battle, fainted: Pokemon): void {
    const participants = new Set<number>(battle.context.playerParticipantsNotFainted);
    const autoShareEnabled = Boolean(
        (battle.gameState as BattleGameStateWithAutoShare)?.auto_exp_share_enabled ?? false
    );
    const expShareHolders = autoShareEnabled ? new Set<number>() : _expShareHolders(battle);

    // In Gen 2, only participants gain Stat Exp. It is not split.
    // We intentionally fix the Gen 2 bug where holding an Exp.Share halves stat exp.
    const halveStatYield = false;
    _awardStatExpToAll(battle, fainted, participants, halveStatYield);

    // Now, distribute level-based experience.
    const halveBaseYield = expShareHolders.size > 0 || autoShareEnabled;
    _distributeExp(battle, fainted, participants, halveBaseYield);

    if (expShareHolders.size > 0) {
        _distributeExp(battle, fainted, expShareHolders, true);
    }
    if (autoShareEnabled) {
        const benchRecipients = _autoExpShareRecipients(battle);
        _distributeAutoExpShare(battle, fainted, benchRecipients);
    }
}

function _isTradedMon(battle: Battle, pokemon: Pokemon): boolean {
    const playerId = battle.gameState.sram.player_id;
    if (playerId === null) {
        return false;
    }
    return pokemon.original_trainer_id !== playerId;
}

function _distributeExp(battle: Battle, fainted: Pokemon, recipientIndices: Set<number>, halveBase: boolean): void {
    const resolved = _resolveRecipients(battle, recipientIndices);
    if (resolved.length === 0) {
        return;
    }

    let baseExp = fainted.species.base_exp || 0;

    if (halveBase) {
        baseExp = Math.floor(baseExp / 2);
    }

    if (resolved.length >= 2) {
        baseExp = Math.floor(baseExp / resolved.length);
    }

    const faintedLevel = fainted.level > 0 ? fainted.level : 1;
    for (const pokemon of resolved) {
        const expGain = _calculateExpGain(
            battle,
            pokemon,
            baseExp,
            faintedLevel,
            battle.context.trainerBattle
        );
        _grantExpGain(battle, pokemon, expGain);
    }
}

function _awardStatExpToAll(battle: Battle, fainted: Pokemon, recipientIndices: Set<number>, halveStatYield: boolean): void {
    const resolved = _resolveRecipients(battle, recipientIndices);
    if (resolved.length === 0) {
        return;
    }

    const baseStats = fainted.species.base_stats;
    const statsYield = {
        hp: baseStats.hp,
        attack: baseStats.attack,
        defense: baseStats.defense,
        speed: baseStats.speed,
        special: baseStats.special_attack,
    };

    if (halveStatYield) {
        for (const key in statsYield) {
            statsYield[key as keyof typeof statsYield] = Math.floor(statsYield[key as keyof typeof statsYield] / 2);
        }
    }

    for (const pokemon of resolved) {
        awardStatExp(pokemon, statsYield);
    }
}

export function awardStatExp(pokemon: Pokemon, statsYield: { [key: string]: number }): void {
    const MAX_STAT_EXP = 65535;
    const pokerus = pokemon.pokerus ?? false;
    const multiplier = pokerus ? 2 : 1;

    pokemon.hp_exp = Math.min(MAX_STAT_EXP, pokemon.hp_exp + statsYield['hp'] * multiplier);
    pokemon.attack_exp = Math.min(MAX_STAT_EXP, pokemon.attack_exp + statsYield['attack'] * multiplier);
    pokemon.defense_exp = Math.min(MAX_STAT_EXP, pokemon.defense_exp + statsYield['defense'] * multiplier);
    pokemon.speed_exp = Math.min(MAX_STAT_EXP, pokemon.speed_exp + statsYield['speed'] * multiplier);
    pokemon.special_exp = Math.min(MAX_STAT_EXP, pokemon.special_exp + statsYield['special'] * multiplier);
}

function _calculateExpGain(battle: Battle, receiver: Pokemon, baseExp: number, faintedLevel: number, trainerBattle: boolean): number {
    let exp = Math.floor((baseExp * faintedLevel) / 7);
    if (_isTradedMon(battle, receiver)) {
        exp = _boostOneAndAHalf(exp);
    }
    if (trainerBattle) {
        exp = _boostOneAndAHalf(exp);
    }
    if (_holdingItem(receiver, "LUCKY_EGG")) {
        exp = _boostOneAndAHalf(exp);
    }
    return Math.max(0, exp);
}

function _applyExpImmediately(pokemon: Pokemon, expGain: number): void {
    const growth = pokemon.species.growth_rate;
    if (!growth) {
        pokemon.experience += expGain;
        return;
    }
    const maxExp = calculateExperience(growth, 100);
    const targetExp = Math.min(maxExp, pokemon.experience + expGain);
    const pendingLevels = buildLevelQueue(pokemon, targetExp);
    pokemon.experience = targetExp;
    for (const levelInfo of pendingLevels) {
        _applyLevelUpImmediately(pokemon, levelInfo);
    }
}

function _applyLevelUpImmediately(pokemon: Pokemon, info: LevelUpInfo): void {
    pokemon.level = info.level;
    const oldHp = pokemon.hp;
    pokemon.max_hp = info.stats.max_hp;
    pokemon.attack = info.stats.attack;
    pokemon.defense = info.stats.defense;
    pokemon.speed = info.stats.speed;
    pokemon.special_attack = info.stats.special_attack;
    pokemon.special_defense = info.stats.special_defense;
    pokemon.hp = Math.min(pokemon.max_hp, Math.max(1, oldHp + info.hpDelta));
    for (const learned of info.learnedMoves) {
        if (pokemon.moves?.some((move) => move?.name === learned.name)) {
            continue;
        }
        const moves = (pokemon.moves ?? []).filter(Boolean);
        if (moves.length >= 4) {
            continue;
        }
        moves.push({ ...learned });
        pokemon.moves = moves;
    }
}

function _boostOneAndAHalf(value: number): number {
    return value + Math.floor(value / 2);
}

function _holdingItem(pokemon: Pokemon, scriptName: string): boolean {
    const held = _normalizeItemName(pokemon.item ?? undefined);
    if (!held) {
        return false;
    }
    return held.replace(/ /g, "_").toUpperCase() === scriptName;
}

function _expShareHolders(battle: Battle): Set<number> {
    const holders = new Set<number>();
    const party = battle.context.playerParty;
    for (let i = 0; i < party.length; i++) {
        const pokemon = party[i];
        if (pokemon && pokemon.hp > 0 && _holdingItem(pokemon, "EXP_SHARE")) {
            holders.add(i);
        }
    }
    return holders;
}

function _resolveRecipients(battle: Battle, indices: Set<number>): Pokemon[] {
    const party = battle.context.playerParty;
    const resolved: Pokemon[] = [];
    indices.forEach(index => {
        if (index >= 0 && index < party.length) {
            const pokemon = party[index];
            if (pokemon && pokemon.hp > 0) {
                resolved.push(pokemon);
            }
        }
    });
    return resolved;
}

function _autoExpShareRecipients(battle: Battle): Set<number> {
    const party = battle.context.playerParty;
    const activeIndex = battle.context.playerActiveIndex;
    const recipients = new Set<number>();
    for (let i = 0; i < party.length; i++) {
        if (i === activeIndex) {
            continue;
        }
        const pokemon = party[i];
        if (pokemon && pokemon.hp > 0) {
            recipients.add(i);
        }
    }
    return recipients;
}

function _distributeAutoExpShare(battle: Battle, fainted: Pokemon, recipientIndices: Set<number>): void {
    const resolved = _resolveRecipients(battle, recipientIndices);
    if (resolved.length === 0) {
        return;
    }

    const baseExp = fainted.species.base_exp || 0;
    const baseStats = fainted.species.base_stats;
    const statsYield = {
        hp: baseStats.hp,
        attack: baseStats.attack,
        defense: baseStats.defense,
        speed: baseStats.speed,
        special: baseStats.special_attack,
    };
    const faintedLevel = fainted.level > 0 ? fainted.level : 1;
    for (const pokemon of resolved) {
        let expGain = _calculateExpGain(
            battle,
            pokemon,
            baseExp,
            faintedLevel,
            battle.context.trainerBattle
        );
        expGain = Math.floor((expGain * 3) / 4);
        _grantExpGain(battle, pokemon, expGain);
    }
}

function _grantExpGain(battle: Battle, pokemon: Pokemon, expGain: number): void {
    if (expGain <= 0) {
        return;
    }

    if (battle.eventManager instanceof EventManager) {
        battle.eventManager.dispatch(
            new Event("show_text", { text: `${pokemon.nickname} gained ${expGain} EXP!` })
        );
    }

    const result = battle.battleUiCall("enqueue_exp_gain", pokemon, expGain);

    if (result === null && !battle.battleUi) {
        _applyExpImmediately(pokemon, expGain);
    }
}
