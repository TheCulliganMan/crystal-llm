import { Pokemon, Move as MoveData, LearnedMove } from '@pokecrystal/core/core/models';
import { BattleContext, BattleStateEnum, Weather } from './battle-context';
import { Battle } from './battle-logic';
import {
  BattleTurn,
  MoveEffect,
  MoveName,
  PokemonType,
  GenderRatio,
  PlayerGender,
  Stat,
  StatusCondition,
} from '@pokecrystal/core/core/enums';
import { Event, EventManager } from '@pokecrystal/core/engine/events/events';
import { GameState } from '@pokecrystal/core/core/state';
import { HardwareRNG } from '@pokecrystal/core/engine/games/rng';
import { placeSpikes } from './hazards';
import { applyTransformState } from './transform-state';
import { calculateFutureSightDamage, queueFutureSight } from './between-turn-effects';
import { getBattleItem } from './item-lookup';
import { resetBattleStatStages } from './stat-stages';

const _HELD_STATUS_MAP: { [key: string]: Set<StatusCondition> } = {
    "HELD_HEAL_POISON": new Set([StatusCondition.POISON]),
    "HELD_HEAL_BURN": new Set([StatusCondition.BURN]),
    "HELD_HEAL_FREEZE": new Set([StatusCondition.FREEZE]),
    "HELD_HEAL_PARALYZE": new Set([StatusCondition.PARALYSIS]),
    "HELD_HEAL_SLEEP": new Set([StatusCondition.SLEEP]),
    "HELD_HEAL_CONFUSION": new Set([StatusCondition.CONFUSION]),
    "HELD_HEAL_STATUS": new Set([
        StatusCondition.POISON,
        StatusCondition.BURN,
        StatusCondition.FREEZE,
        StatusCondition.PARALYSIS,
        StatusCondition.SLEEP,
        StatusCondition.CONFUSION,
    ]),
};

function _format_item_name(itemName: string): string {
    return itemName.replace(/_/g, " ");
}

function _consume_held_status_item(
    pokemon: Pokemon,
    status: StatusCondition,
    context: BattleContext,
    eventManager: EventManager,
): boolean {
    const itemKey = pokemon.item;
    if (!itemKey) {
        return false;
    }

    const item = getBattleItem(itemKey);
    if (!item) {
        return false;
    }

    const statuses = _HELD_STATUS_MAP[item.held_effect];
    if (!statuses || !statuses.has(status)) {
        return false;
    }

    // Clear the relevant ailment.
    if (status === StatusCondition.CONFUSION) {
        pokemon.confusion_turns = 0;
    } else {
        pokemon.status = undefined;
        if (status === StatusCondition.SLEEP) {
            pokemon.sleep_turns = 0;
            pokemon.nightmare = false;
        }
    }

    const displayName = _format_item_name(item.name);
    if (
        status === StatusCondition.CONFUSION &&
        item.held_effect === "HELD_HEAL_CONFUSION"
    ) {
        eventManager.dispatch(
            new Event(
                "show_text",
                { text: `A ${displayName} rid ${pokemon.nickname} of its confusion.` }
            )
        );
    } else {
        eventManager.dispatch(
            new Event(
                "show_text",
                { text: `${pokemon.nickname} recovered using a ${displayName}!` }
            )
        );
    }

    pokemon.item = undefined;

    return true;
}


export function modifyStat(
  pokemon: Pokemon,
  stat: Stat,
  stages: number,
  eventManager: EventManager,
  showMessage = true,
): boolean {
  const currentBoost = pokemon.stat_boosts[stat] || 0;
  const newBoost = Math.max(-6, Math.min(6, currentBoost + stages));

  if (newBoost === currentBoost) {
    if (showMessage) {
      const statName = stat.toLowerCase().replace('_', ' ');
      if (stages > 0) {
        eventManager.dispatch(
          new Event(
            'show_text', {
              text: `${pokemon.nickname}'s ${statName} won't go any higher!`
            }
          )
        );
      } else {
        eventManager.dispatch(
          new Event(
            'show_text', {
              text: `${pokemon.nickname}'s ${statName} won't go any lower!`
            }
          )
        );
      }
    }
    return false;
  }

  pokemon.stat_boosts[stat] = newBoost;

  if (showMessage) {
    const statName = stat.toLowerCase().replace('_', ' ');
    if (stages > 0) {
      if (Math.abs(stages) === 1) {
        eventManager.dispatch(
          new Event('show_text', {
            text: `${pokemon.nickname}'s ${statName} rose!`
          })
        );
      } else {
        eventManager.dispatch(
          new Event(
            'show_text', {
              text: `${pokemon.nickname}'s ${statName} rose sharply!`
            }
          )
        );
      }
    } else {
      if (Math.abs(stages) === 1) {
        eventManager.dispatch(
          new Event('show_text', {
            text: `${pokemon.nickname}'s ${statName} fell!`
          })
        );
      } else {
        eventManager.dispatch(
          new Event(
            'show_text', {
              text: `${pokemon.nickname}'s ${statName} fell sharply!`
            }
          )
        );
      }
    }
  }

  return true;
}

export function applyMoveEffect(
  attacker: Pokemon,
  defender: Pokemon,
  move: MoveData,
  damage: number,
  context: BattleContext,
  eventManager: EventManager,
  gameState: GameState,
  battle ? : Battle,
): void {
  const guaranteedEffect =
    move.effect !== MoveEffect.NORMAL_HIT &&
    move.effect !== MoveEffect.NONE &&
    move.effect_chance === 0;

  if (!guaranteedEffect) {
    if (context.predefinedRandomValue !== undefined) {
      if (context.predefinedRandomValue >= (move.effect_chance || 0) / 100) {
        return;
      }
    } else {
      const rng = new HardwareRNG(gameState);
      const threshold = Math.floor((move.effect_chance || 0) * 2.55);
      if (rng.nextByte() >= threshold) {
        return;
      }
    }
  }

  const effectHandlers: {
    [key in MoveEffect] ? : () => void
  } = {
    [MoveEffect.NORMAL_HIT]: () => {},
    [MoveEffect.SLEEP]: () =>
      _applySleepEffect(attacker, defender, context, eventManager, gameState),
    [MoveEffect.POISON_HIT]: () =>
      _applyPoisonEffect(attacker, defender, context, eventManager),
    [MoveEffect.LEECH_HIT]: () =>
      _applyLeechEffect(attacker, defender, damage, eventManager),
    [MoveEffect.DREAM_EATER]: () =>
      _applyLeechEffect(attacker, defender, damage, eventManager),
    [MoveEffect.LEECH_SEED]: () =>
      _applyLeechSeedStatus(attacker, defender, context, eventManager),
    [MoveEffect.BURN_HIT]: () =>
      _applyBurnEffect(attacker, defender, context, eventManager),
    [MoveEffect.PARALYZE_HIT]: () =>
      _applyParalyzeEffect(attacker, defender, context, eventManager),
    [MoveEffect.FREEZE_HIT]: () =>
      _applyFreezeEffect(attacker, defender, context, eventManager),
    [MoveEffect.CONFUSE]: () =>
      _applyConfuseEffect(attacker, defender, context, eventManager, gameState),
    [MoveEffect.SELFDESTRUCT]: () => _applySelfdestructEffect(attacker),
    [MoveEffect.ALL_UP_HIT]: () =>
        _applyAllUpHit(attacker, damage, eventManager),
    [MoveEffect.ATTACK_UP_HIT]: () =>
        _applyAttackUpHit(attacker, damage, eventManager),
    [MoveEffect.ATTRACT]: () =>
        _applyAttractEffect(attacker, defender, context, eventManager),
    [MoveEffect.BIDE]: () =>
        _applyBideEffect(attacker, eventManager),
    [MoveEffect.BELLY_DRUM]: () =>
        _applyBellyDrumEffect(attacker, eventManager),
    [MoveEffect.CONFUSE_HIT]: () =>
        _applyConfuseHitEffect(attacker, defender, damage, context, eventManager, gameState),
    [MoveEffect.CURSE]: () =>
        _applyCurseEffect(attacker, defender, context, eventManager),
    [MoveEffect.DEFENSE_CURL]: () =>
        _applyDefenseCurlEffect(attacker, eventManager),
    [MoveEffect.DESTINY_BOND]: () =>
        _applyDestinyBondEffect(attacker, battle, eventManager),
    [MoveEffect.PROTECT]: () =>
        _applyProtectEffect(attacker, context, eventManager, gameState),
    [MoveEffect.ENDURE]: () =>
        _applyEndureEffect(attacker, context, eventManager, gameState),
    [MoveEffect.ENCORE]: () =>
        _applyEncoreEffect(defender, eventManager, gameState),
    [MoveEffect.FOCUS_ENERGY]: () =>
        _applyFocusEnergyEffect(attacker, eventManager),
    [MoveEffect.FORESIGHT]: () =>
        _applyForesightEffect(defender, eventManager),
    [MoveEffect.FUTURE_SIGHT]: () =>
        _applyFutureSightEffect(attacker, defender, context, eventManager, gameState),
    [MoveEffect.HEAL]: () =>
        _applyHealEffect(attacker, move, eventManager),
    [MoveEffect.HEAL_BELL]: () =>
        _applyHealBellEffect(attacker, context, eventManager),
    [MoveEffect.LIGHT_SCREEN]: () =>
        _applyBarrierEffect(attacker, context, eventManager, 'light_screen', 'LIGHT SCREEN'),
    [MoveEffect.LOCK_ON]: () =>
        _applyLockOnEffect(attacker, defender, context, eventManager),
    [MoveEffect.MEAN_LOOK]: () =>
        _applyMeanLookEffect(defender, context, eventManager),
    [MoveEffect.MIST]: () =>
        _applyBarrierEffect(attacker, context, eventManager, 'mist', 'MIST'),
    [MoveEffect.MOONLIGHT]: () =>
        _applyWeatherHealEffect(attacker, context, eventManager),
    [MoveEffect.MORNING_SUN]: () =>
        _applyWeatherHealEffect(attacker, context, eventManager),
    [MoveEffect.NIGHTMARE]: () =>
        _applyNightmareEffect(defender, eventManager),
    [MoveEffect.PARALYZE]: () =>
        _applyParalyzeEffect(attacker, defender, context, eventManager),
    [MoveEffect.PERISH_SONG]: () =>
        _applyPerishSongEffect(context, eventManager),
    [MoveEffect.POISON]: () =>
        _applyPoisonEffect(attacker, defender, context, eventManager),
    [MoveEffect.RAGE]: () =>
        _applyRageEffect(attacker),
    [MoveEffect.DEFENSE_UP_HIT]: () =>
        _applyDefenseUpHit(attacker, defender, damage, eventManager),
    [MoveEffect.ATTACK_UP]: () =>
        modifyStat(attacker, Stat.ATTACK, 1, eventManager),
    [MoveEffect.DEFENSE_UP]: () =>
        modifyStat(attacker, Stat.DEFENSE, 1, eventManager),
    [MoveEffect.SPEED_UP]: () =>
        modifyStat(attacker, Stat.SPEED, 1, eventManager),
    [MoveEffect.SPECIAL_ATTACK_UP]: () =>
        modifyStat(attacker, Stat.SPECIAL_ATTACK, 1, eventManager),
    [MoveEffect.SPECIAL_DEFENSE_UP]: () =>
        modifyStat(attacker, Stat.SPECIAL_DEFENSE, 1, eventManager),
    [MoveEffect.ACCURACY_UP]: () =>
        modifyStat(attacker, Stat.ACCURACY, 1, eventManager),
    [MoveEffect.EVASION_UP]: () =>
        modifyStat(attacker, Stat.EVASION, 1, eventManager),
    [MoveEffect.ATTACK_UP_2]: () =>
        modifyStat(attacker, Stat.ATTACK, 2, eventManager),
    [MoveEffect.DEFENSE_UP_2]: () =>
        modifyStat(attacker, Stat.DEFENSE, 2, eventManager),
    [MoveEffect.SPEED_UP_2]: () =>
        modifyStat(attacker, Stat.SPEED, 2, eventManager),
    [MoveEffect.SPECIAL_ATTACK_UP_2]: () =>
        modifyStat(attacker, Stat.SPECIAL_ATTACK, 2, eventManager),
    [MoveEffect.SPECIAL_DEFENSE_UP_2]: () =>
        modifyStat(attacker, Stat.SPECIAL_DEFENSE, 2, eventManager),
    [MoveEffect.ACCURACY_UP_2]: () =>
        modifyStat(attacker, Stat.ACCURACY, 2, eventManager),
    [MoveEffect.EVASION_UP_2]: () =>
        modifyStat(attacker, Stat.EVASION, 2, eventManager),
    [MoveEffect.ATTACK_DOWN]: () =>
        modifyStat(defender, Stat.ATTACK, -1, eventManager),
    [MoveEffect.DEFENSE_DOWN]: () =>
        modifyStat(defender, Stat.DEFENSE, -1, eventManager),
    [MoveEffect.SPEED_DOWN]: () =>
        modifyStat(defender, Stat.SPEED, -1, eventManager),
    [MoveEffect.SPECIAL_ATTACK_DOWN]: () =>
        modifyStat(defender, Stat.SPECIAL_ATTACK, -1, eventManager),
    [MoveEffect.SPECIAL_DEFENSE_DOWN]: () =>
        modifyStat(defender, Stat.SPECIAL_DEFENSE, -1, eventManager),
    [MoveEffect.ACCURACY_DOWN]: () =>
        modifyStat(defender, Stat.ACCURACY, -1, eventManager),
    [MoveEffect.EVASION_DOWN]: () =>
        modifyStat(defender, Stat.EVASION, -1, eventManager),
    [MoveEffect.ATTACK_DOWN_2]: () =>
        modifyStat(defender, Stat.ATTACK, -2, eventManager),
    [MoveEffect.DEFENSE_DOWN_2]: () =>
        modifyStat(defender, Stat.DEFENSE, -2, eventManager),
    [MoveEffect.SPEED_DOWN_2]: () =>
        modifyStat(defender, Stat.SPEED, -2, eventManager),
    [MoveEffect.SPECIAL_ATTACK_DOWN_2]: () =>
        modifyStat(defender, Stat.SPECIAL_ATTACK, -2, eventManager),
    [MoveEffect.SPECIAL_DEFENSE_DOWN_2]: () =>
        modifyStat(defender, Stat.SPECIAL_DEFENSE, -2, eventManager),
    [MoveEffect.ACCURACY_DOWN_2]: () =>
        modifyStat(defender, Stat.ACCURACY, -2, eventManager),
    [MoveEffect.EVASION_DOWN_2]: () =>
        modifyStat(defender, Stat.EVASION, -2, eventManager),
    [MoveEffect.FLINCH_HIT]: () =>
        _applyFlinchHit(defender, damage, eventManager, gameState),
    [MoveEffect.PSYCH_UP]: () =>
        _applyPsychUpEffect(attacker, defender, eventManager),
    [MoveEffect.RAPID_SPIN]: () =>
        _applyRapidSpinEffect(attacker, context, eventManager),
    [MoveEffect.RAIN_DANCE]: () =>
        _applyWeatherEffect(context, eventManager, Weather.RAIN, 'It started to rain!'),
    [MoveEffect.REFLECT]: () =>
        _applyBarrierEffect(attacker, context, eventManager, 'reflect', 'REFLECT'),
    [MoveEffect.RESET_STATS]: () =>
        _applyResetStatsEffect(context, eventManager),
    [MoveEffect.TRAP_TARGET]: () =>
        _applyTrapTarget(attacker, defender, move, context, eventManager, gameState),
    [MoveEffect.RECOIL_HIT]: () =>
        _applyRecoilHit(attacker, damage, eventManager),
    [MoveEffect.SAFEGUARD]: () =>
        _applyBarrierEffect(attacker, context, eventManager, 'safeguard', 'SAFEGUARD'),
    [MoveEffect.SANDSTORM]: () =>
        _applyWeatherEffect(context, eventManager, Weather.SANDSTORM, 'A sandstorm brewed!'),
    [MoveEffect.SPIKES]: () =>
        _applySpikesEffect(context, attacker, eventManager),
    [MoveEffect.SPITE]: () =>
        _applySpiteEffect(defender, eventManager, gameState),
    [MoveEffect.SPLASH]: () =>
        eventManager.dispatch(new Event("show_text", { text: "But nothing happened!" })),
    [MoveEffect.SUNNY_DAY]: () =>
        _applyWeatherEffect(context, eventManager, Weather.SUN, 'The sunlight got bright!'),
    [MoveEffect.SYNTHESIS]: () =>
        _applyWeatherHealEffect(attacker, context, eventManager),
    [MoveEffect.TELEPORT]: () =>
        _applyTeleportEffect(battle, attacker, context, eventManager),
    [MoveEffect.THUNDER]: () =>
        _applyThunderEffect(attacker, defender, damage, context, eventManager),
    [MoveEffect.TOXIC]: () =>
        _applyPoisonEffect(attacker, defender, context, eventManager),
    [MoveEffect.ATTACK_DOWN_HIT]: () =>
        _applyAttackDownHit(defender, damage, eventManager),
    [MoveEffect.DEFENSE_DOWN_HIT]: () =>
        _applyDefenseDownHit(defender, damage, eventManager),
    [MoveEffect.SPEED_DOWN_HIT]: () =>
        _applySpeedDownHit(defender, damage, eventManager),
    [MoveEffect.SPECIAL_DEFENSE_DOWN_HIT]: () =>
        _applySpecialDefenseDownHit(defender, damage, eventManager),
    [MoveEffect.ACCURACY_DOWN_HIT]: () =>
        _applyAccuracyDownHit(defender, damage, eventManager),
    [MoveEffect.EVASION_DOWN_HIT]: () =>
        _applyEvasionDownHit(defender, damage, eventManager),
    [MoveEffect.DISABLE]: () =>
        _applyDisableEffect(attacker, defender, context, eventManager, gameState),
    [MoveEffect.TRANSFORM]: () =>
        _applyTransformEffect(attacker, defender, eventManager),
  };

  const handler = effectHandlers[move.effect];
  if (handler) {
    handler();
  }
}

function _statusBlockedBySafeguard(
  attacker: Pokemon,
  defender: Pokemon,
  context: BattleContext,
  eventManager: EventManager,
): boolean {
  const attackerSide = context.sideFor(attacker);
  const defenderSide = context.sideFor(defender);
  if (attackerSide === undefined || defenderSide === undefined) {
    return false;
  }
  if (attackerSide === defenderSide) {
    return false;
  }
  if (context.barrierTurns(defenderSide, 'safeguard') <= 0) {
    return false;
  }

  eventManager.dispatch(
    new Event(
      'show_text', {
        text: `${defender.nickname} is protected by SAFEGUARD!`
      }
    )
  );
  return true;
}

function _applySleepEffect(
  attacker: Pokemon,
  defender: Pokemon,
  context: BattleContext,
  eventManager: EventManager,
  gameState: GameState,
): void {
  if (defender.status) {
    eventManager.dispatch(new Event('show_text', {
      text: 'But it failed!'
    }));
    return;
  }

  defender.status = StatusCondition.SLEEP;
  defender.sleep_turns = (new HardwareRNG(gameState)).randrange(4) + 2;
  eventManager.dispatch(
    new Event('show_text', {
      text: `${defender.nickname} fell asleep!`
    })
  );
  _consume_held_status_item(defender, StatusCondition.SLEEP, context, eventManager);
}

function _applyFlinchHit(
  defender: Pokemon,
  damage: number,
  eventManager: EventManager,
  gameState: GameState,
): void {
  if (damage > 0) {
    defender.flinching = true;
  }
}

function _applySelfdestructEffect(attacker: Pokemon): void {
    attacker.hp = 0;
}

function _applyRageEffect(attacker: Pokemon): void {
    // ASM mapping: pokecrystal_disassembly/engine/battle/move_effects/rage.asm (BattleCommand_Rage).
    attacker.rage_active = true;
}

function _applyDefenseUpHit(
  attacker: Pokemon,
  defender: Pokemon,
  damage: number,
    eventManager: EventManager,
): void {
    if (damage > 0) {
        modifyStat(attacker, Stat.DEFENSE, 1, eventManager);
    }
}

function _applyAttackUpHit(
  attacker: Pokemon,
  damage: number,
  eventManager: EventManager,
): void {
  if (damage > 0) {
    modifyStat(attacker, Stat.ATTACK, 1, eventManager);
  }
}

function _applyAllUpHit(
  attacker: Pokemon,
  damage: number,
  eventManager: EventManager,
): void {
  if (damage <= 0) {
    return;
  }
  modifyStat(attacker, Stat.ATTACK, 1, eventManager, false);
  modifyStat(attacker, Stat.DEFENSE, 1, eventManager, false);
  modifyStat(attacker, Stat.SPEED, 1, eventManager, false);
  modifyStat(attacker, Stat.SPECIAL_ATTACK, 1, eventManager, false);
  modifyStat(attacker, Stat.SPECIAL_DEFENSE, 1, eventManager, false);
  eventManager.dispatch(new Event("show_text", { text: `${attacker.nickname}'s stats rose!` }));
}

function _opponentWentFirst(
    attacker: Pokemon,
    context: BattleContext,
): boolean {
    const side = context.sideFor(attacker);
    if (side === undefined || context.turnOrder.length === 0) {
        return false;
    }
    return context.turnOrder.indexOf(side) > 0;
}

function _protectChance(
    attacker: Pokemon,
    context: BattleContext,
    eventManager: EventManager,
    gameState: GameState,
): boolean {
    if (_opponentWentFirst(attacker, context) || attacker.substitute_hp > 0) {
        attacker.protect_counter = 0;
        attacker.endure_counter = 0;
        eventManager.dispatch(new Event("show_text", { text: "But it failed!" }));
        return false;
    }

    const rng = new HardwareRNG(gameState);
    let success = false;
    if (attacker.protect_counter === 0) {
        success = true;
    } else {
        const divisor = 2 ** Math.min(attacker.protect_counter, 8);
        const threshold = Math.floor(255 / divisor);
        if (rng.nextByte() < threshold) {
            success = true;
        }
    }

    if (!success) {
        eventManager.dispatch(new Event("show_text", { text: "But it failed!" }));
        attacker.protect_counter = 0;
        attacker.endure_counter = 0;
        return false;
    }

    attacker.protect_counter += 1;
    attacker.endure_counter = attacker.protect_counter;
    return true;
}

function _applyProtectEffect(
    attacker: Pokemon,
    context: BattleContext,
    eventManager: EventManager,
    gameState: GameState,
): void {
    if (!_protectChance(attacker, context, eventManager, gameState)) {
        return;
    }

    attacker.protect_active = true;
    eventManager.dispatch(
        new Event(
            "show_text",
            { text: `${attacker.nickname} protected itself!` }
        )
    );
}

function _applyEndureEffect(
    attacker: Pokemon,
    context: BattleContext,
    eventManager: EventManager,
    gameState: GameState,
): void {
    if (!_protectChance(attacker, context, eventManager, gameState)) {
        return;
    }

    attacker.endure_active = true;
    eventManager.dispatch(
        new Event(
            "show_text",
            { text: `${attacker.nickname} braced itself!` }
        )
    );
}

function _applyBellyDrumEffect(
    attacker: Pokemon,
    eventManager: EventManager,
): void {
    if ((attacker.stat_boosts[Stat.ATTACK] ?? 0) >= 6) {
        eventManager.dispatch(new Event("show_text", { text: "But it failed!" }));
        return;
    }

    // ASM bug: BattleCommand_AttackUp2 runs before the HP check, so below half HP
    // Belly Drum still gives +2 Attack, then fails without cutting HP.
    const halfMaxHp = Math.floor(attacker.max_hp / 2);
    if (attacker.hp < halfMaxHp) {
        modifyStat(attacker, Stat.ATTACK, 2, eventManager);
        eventManager.dispatch(new Event("show_text", { text: "But it failed!" }));
        return;
    }

    attacker.hp -= halfMaxHp;
    attacker.stat_boosts[Stat.ATTACK] = 6;
    eventManager.dispatch(
        new Event(
            "show_text",
            { text: `${attacker.nickname} cut its own HP and maximized ATTACK!` }
        )
    );
}

function _applyTransformEffect(
  attacker: Pokemon,
  defender: Pokemon,
  eventManager: EventManager,
): void {
  if (!applyTransformState(attacker, defender)) {
    eventManager.dispatch(new Event("show_text", { text: "But it failed!" }));
    return;
  }

  const speciesName = String(defender.species.id ?? "UNKNOWN").replace(/_/g, " ");
  eventManager.dispatch(
    new Event(
      "show_text",
      { text: `${attacker.nickname} transformed into ${speciesName}!` }
    )
  );
}

function _applyAttractEffect(
  attacker: Pokemon,
  defender: Pokemon,
  context: BattleContext,
  eventManager: EventManager,
): void {
  if (!hasOppositeGender(attacker, defender)) {
    eventManager.dispatch(new Event("show_text", { text: "But it failed!" }));
    return;
  }
  if (defender.attract_source_side !== undefined) {
    eventManager.dispatch(new Event("show_text", { text: "But it failed!" }));
    return;
  }
  const side = context.sideFor(attacker);
  if (side === undefined) {
    throw new Error("Unable to determine Attract source side.");
  }
  defender.attract_source_side = side;
  eventManager.dispatch(new Event("show_text", { text: `${defender.nickname} fell in love!` }));
}

function battleGender(pokemon: Pokemon): PlayerGender | null {
  if (pokemon.gender === PlayerGender.MALE || pokemon.gender === PlayerGender.FEMALE) {
    return pokemon.gender;
  }
  const ratio = pokemon.species.gender_ratio;
  if (ratio === GenderRatio.GENDER_UNKNOWN) {
    return null;
  }
  if (ratio === GenderRatio.GENDER_F0) {
    return PlayerGender.MALE;
  }
  if (ratio === GenderRatio.GENDER_F100) {
    return PlayerGender.FEMALE;
  }
  const attackDv = pokemon.dvs.attack & 0xf;
  return attackDv < ratio / 16 ? PlayerGender.FEMALE : PlayerGender.MALE;
}

function hasOppositeGender(attacker: Pokemon, defender: Pokemon): boolean {
  const attackerGender = battleGender(attacker);
  const defenderGender = battleGender(defender);
  return attackerGender !== null && defenderGender !== null && attackerGender !== defenderGender;
}

function _applyBideEffect(attacker: Pokemon, eventManager: EventManager): void {
  attacker.bide_active = true;
  attacker.bide_turns_remaining = 2;
  attacker.bide_damage = 0;
  eventManager.dispatch(new Event("show_text", { text: `${attacker.nickname} is storing energy!` }));
}

function _applyConfuseHitEffect(
  attacker: Pokemon,
  defender: Pokemon,
  damage: number,
  context: BattleContext,
  eventManager: EventManager,
  gameState: GameState,
): void {
  if (damage > 0) {
    _applyConfuseEffect(attacker, defender, context, eventManager, gameState);
  }
}

function _applyCurseEffect(
  attacker: Pokemon,
  defender: Pokemon,
  context: BattleContext,
  eventManager: EventManager,
): void {
  const isGhost =
    attacker.species.type1 === PokemonType.GHOST || attacker.species.type2 === PokemonType.GHOST;
  if (isGhost) {
    if (defender.substitute_hp > 0 || defender.cursed) {
      eventManager.dispatch(new Event("show_text", { text: "But it failed!" }));
      return;
    }
    attacker.hp = Math.max(0, attacker.hp - Math.floor(attacker.max_hp / 2));
    defender.cursed = true;
    defender.curse_source_side = context.sideFor(attacker);
    eventManager.dispatch(new Event("show_text", { text: `${defender.nickname} was afflicted by the curse!` }));
    return;
  }

  if ((attacker.stat_boosts[Stat.ATTACK] ?? 0) >= 6 && (attacker.stat_boosts[Stat.DEFENSE] ?? 0) >= 6) {
    eventManager.dispatch(new Event("show_text", { text: "But it failed!" }));
    return;
  }

  modifyStat(attacker, Stat.ATTACK, 1, eventManager, false);
  modifyStat(attacker, Stat.DEFENSE, 1, eventManager, false);
  modifyStat(attacker, Stat.SPEED, -1, eventManager, false);
  eventManager.dispatch(
    new Event("show_text", { text: `${attacker.nickname}'s ATTACK and DEFENSE rose while SPEED fell!` })
  );
}

function _applyDefenseCurlEffect(attacker: Pokemon, eventManager: EventManager): void {
  attacker.defense_curled = true;
  modifyStat(attacker, Stat.DEFENSE, 1, eventManager);
}

function _applyDestinyBondEffect(
  attacker: Pokemon,
  battle: Battle | undefined,
  eventManager: EventManager,
): void {
  attacker.destiny_bond_active = true;
  attacker.destiny_bond_action_id = battle?._actionCounter ?? 0;
  eventManager.dispatch(new Event("show_text", { text: `${attacker.nickname} is trying to take its foe with it!` }));
}

function _applyPoisonEffect(
  attacker: Pokemon,
  defender: Pokemon,
  context: BattleContext,
  eventManager: EventManager,
): void {
  if (defender.status) {
    eventManager.dispatch(new Event('show_text', {
      text: 'But it failed!'
    }));
    return;
  }

  const defenderTypes = new Set([defender.species.type1, defender.species.type2]);
  if (defenderTypes.has(PokemonType.POISON)) {
    eventManager.dispatch(new Event('show_text', {
      text: 'But it failed!'
    }));
    return;
  }

  if (_statusBlockedBySafeguard(attacker, defender, context, eventManager)) {
    return;
  }

  defender.status = StatusCondition.POISON;
  eventManager.dispatch(
    new Event('show_text', {
      text: `${defender.nickname} was poisoned!`
    })
  );
  _consume_held_status_item(defender, StatusCondition.POISON, context, eventManager);
}

function _applyEncoreEffect(
  defender: Pokemon,
  eventManager: EventManager,
  gameState: GameState,
): void {
  if (!defender.last_move_used || defender.encore_turns_remaining > 0) {
    eventManager.dispatch(new Event("show_text", { text: "But it failed!" }));
    return;
  }
  const rng = new HardwareRNG(gameState);
  defender.encored_move = defender.last_move_used;
  defender.encore_turns_remaining = rng.randrange(4) + 3;
  eventManager.dispatch(new Event("show_text", { text: `${defender.nickname} got an encore!` }));
}

function _applyFocusEnergyEffect(attacker: Pokemon, eventManager: EventManager): void {
  if (attacker.focus_energy) {
    eventManager.dispatch(new Event("show_text", { text: "But it failed!" }));
    return;
  }
  attacker.focus_energy = true;
  eventManager.dispatch(new Event("show_text", { text: `${attacker.nickname} is getting pumped!` }));
}

function _applyForesightEffect(defender: Pokemon, eventManager: EventManager): void {
  defender.foresight_active = true;
  eventManager.dispatch(new Event("show_text", { text: `${defender.nickname} was identified!` }));
}

function _restoreHp(
  pokemon: Pokemon,
  amount: number,
  eventManager: EventManager,
  text: string,
): void {
  if (pokemon.hp >= pokemon.max_hp || amount <= 0) {
    eventManager.dispatch(new Event("show_text", { text: "But it failed!" }));
    return;
  }
  pokemon.hp = Math.min(pokemon.max_hp, pokemon.hp + amount);
  eventManager.dispatch(new Event("show_text", { text }));
}

function _applyHealEffect(
  attacker: Pokemon,
  move: MoveData,
  eventManager: EventManager,
): void {
  if (move.name === MoveName.REST) {
    if (attacker.status === StatusCondition.SLEEP || (attacker.hp >= attacker.max_hp && !attacker.status)) {
      eventManager.dispatch(new Event("show_text", { text: "But it failed!" }));
      return;
    }
    attacker.hp = attacker.max_hp;
    attacker.status = StatusCondition.SLEEP;
    attacker.sleep_turns = 2;
    attacker.nightmare = false;
    eventManager.dispatch(new Event("show_text", { text: `${attacker.nickname} went to sleep and became healthy!` }));
    return;
  }
  _restoreHp(attacker, Math.max(1, Math.floor(attacker.max_hp / 2)), eventManager, `${attacker.nickname} regained health!`);
}

function _applyHealBellEffect(
  attacker: Pokemon,
  context: BattleContext,
  eventManager: EventManager,
): void {
  const side = context.sideFor(attacker);
  if (side === undefined) {
    throw new Error("Unable to determine Heal Bell side.");
  }
  for (const pokemon of context.partyFor(side)) {
    pokemon.status = undefined;
    pokemon.sleep_turns = 0;
    pokemon.nightmare = false;
    pokemon.confusion_turns = 0;
  }
  eventManager.dispatch(new Event("show_text", { text: "A bell chimed!" }));
}

function _applyBarrierEffect(
  attacker: Pokemon,
  context: BattleContext,
  eventManager: EventManager,
  barrier: 'light_screen' | 'reflect' | 'mist' | 'safeguard',
  label: string,
): void {
  const side = context.sideFor(attacker);
  if (side === undefined) {
    throw new Error(`Unable to determine ${label} side.`);
  }
  if (context.barrierTurns(side, barrier) > 0) {
    eventManager.dispatch(new Event("show_text", { text: "But it failed!" }));
    return;
  }
  context.setBarrier(side, barrier, 5);
  eventManager.dispatch(new Event("show_text", { text: `${label} raised a barrier!` }));
}

function _applyLockOnEffect(
  attacker: Pokemon,
  defender: Pokemon,
  context: BattleContext,
  eventManager: EventManager,
): void {
  const side = context.sideFor(defender);
  if (side === undefined) {
    throw new Error("Unable to determine Lock-On target side.");
  }
  attacker.lock_on_active = true;
  attacker.lock_on_target_index = context.activeIndexFor(side);
  eventManager.dispatch(new Event("show_text", { text: `${attacker.nickname} took aim!` }));
}

function _applyMeanLookEffect(
  defender: Pokemon,
  context: BattleContext,
  eventManager: EventManager,
): void {
  defender.cant_run = true;
  if (context.sideFor(defender) === BattleTurn.PLAYER) {
    context.playerCantRun = true;
  }
  eventManager.dispatch(new Event("show_text", { text: `${defender.nickname} can no longer escape!` }));
}

function _weatherHealAmount(maxHp: number, weather: Weather): number {
  if (weather === Weather.SUN) {
    return Math.max(1, Math.floor((maxHp * 2) / 3));
  }
  if (weather === Weather.RAIN || weather === Weather.SANDSTORM) {
    return Math.max(1, Math.floor(maxHp / 4));
  }
  return Math.max(1, Math.floor(maxHp / 2));
}

function _applyWeatherHealEffect(
  attacker: Pokemon,
  context: BattleContext,
  eventManager: EventManager,
): void {
  _restoreHp(
    attacker,
    _weatherHealAmount(attacker.max_hp, context.weather),
    eventManager,
    `${attacker.nickname} regained health!`,
  );
}

function _applyNightmareEffect(defender: Pokemon, eventManager: EventManager): void {
  if (defender.status !== StatusCondition.SLEEP || defender.nightmare) {
    eventManager.dispatch(new Event("show_text", { text: "But it failed!" }));
    return;
  }
  defender.nightmare = true;
  eventManager.dispatch(new Event("show_text", { text: `${defender.nickname} began having a nightmare!` }));
}

function _applyPerishSongEffect(context: BattleContext, eventManager: EventManager): void {
  context.playerPokemon.perish_song_turns = 4;
  context.enemyPokemon.perish_song_turns = 4;
  eventManager.dispatch(new Event("show_text", { text: "All battling Pokémon will faint in 3 turns!" }));
}

function _applyPsychUpEffect(attacker: Pokemon, defender: Pokemon, eventManager: EventManager): void {
  attacker.stat_boosts = { ...attacker.stat_boosts, ...defender.stat_boosts };
  eventManager.dispatch(new Event("show_text", { text: `${attacker.nickname} copied ${defender.nickname}'s stat changes!` }));
}

function _applyRapidSpinEffect(
  attacker: Pokemon,
  context: BattleContext,
  eventManager: EventManager,
): void {
  attacker.trapped_turns = 0;
  attacker.trapped_by_side = undefined;
  attacker.trapped_source_index = undefined;
  attacker.trapped_move = undefined;
  attacker.leech_seeded = false;
  attacker.leech_seed_source_side = undefined;
  const side = context.sideFor(attacker);
  if (side !== undefined) {
    context.setSpikesLayers(side, 0);
    if (side === BattleTurn.PLAYER) {
      context.playerCantRun = false;
    }
  }
  eventManager.dispatch(new Event("show_text", { text: `${attacker.nickname} blew away the hazards!` }));
}

function _applyWeatherEffect(
  context: BattleContext,
  eventManager: EventManager,
  weather: Weather,
  text: string,
): void {
  context.weather = weather;
  context.weatherTurns = 5;
  eventManager.dispatch(new Event("show_text", { text }));
}

function _applyResetStatsEffect(context: BattleContext, eventManager: EventManager): void {
  for (const pokemon of [context.playerPokemon, context.enemyPokemon]) {
    resetBattleStatStages(pokemon);
  }
  eventManager.dispatch(new Event("show_text", { text: "All stat changes were eliminated!" }));
}

function _applySpikesEffect(
  context: BattleContext,
  attacker: Pokemon,
  eventManager: EventManager,
): void {
  const side = context.sideFor(attacker);
  if (side === undefined) {
    throw new Error("Unable to determine Spikes side.");
  }
  placeSpikes(context, side, eventManager);
}

function _applySpiteEffect(
  defender: Pokemon,
  eventManager: EventManager,
  gameState: GameState,
): void {
  if (!defender.last_move_used) {
    eventManager.dispatch(new Event("show_text", { text: "But it failed!" }));
    return;
  }
  const move = defender.moves.find((entry) => entry.name === defender.last_move_used);
  if (!move || move.current_pp <= 0) {
    eventManager.dispatch(new Event("show_text", { text: "But it failed!" }));
    return;
  }
  const rng = new HardwareRNG(gameState);
  const reduction = rng.randrange(4) + 2;
  move.current_pp = Math.max(0, move.current_pp - reduction);
  eventManager.dispatch(new Event("show_text", { text: `${defender.last_move_used.replace(/_/g, " ")} lost PP!` }));
}

function _applyTeleportEffect(
  battle: Battle | undefined,
  attacker: Pokemon,
  context: BattleContext,
  eventManager: EventManager,
): void {
  if (!battle || context.trainerBattle) {
    eventManager.dispatch(new Event("show_text", { text: "But it failed!" }));
    return;
  }
  battle._playerRan = true;
  context.currentState = BattleStateEnum.BATTLE_END;
  eventManager.dispatch(new Event("show_text", { text: `${attacker.nickname} fled using TELEPORT!` }));
}

function _applyFutureSightEffect(
  attacker: Pokemon,
  defender: Pokemon,
  context: BattleContext,
  eventManager: EventManager,
  gameState: GameState,
): void {
  const side = context.sideFor(attacker);
  if (side === undefined) {
    throw new Error("Unable to determine Future Sight attacker side.");
  }
  const damage = calculateFutureSightDamage(
    attacker,
    defender,
    gameState,
    context.predefinedRandomValue ?? context.predefined_random_value ?? null,
  );
  queueFutureSight(side, attacker, defender, context, eventManager, damage);
}

function _applyLeechEffect(
  attacker: Pokemon,
  defender: Pokemon,
  damage: number,
  eventManager: EventManager,
): void {
  attacker.hp = Math.min(attacker.max_hp, attacker.hp + Math.floor(damage / 2));
  eventManager.dispatch(
    new Event(
      'show_text', {
        text: `${attacker.nickname} drained health from ${defender.nickname}!`
      }
    )
  );
}

function _applyBurnEffect(
  attacker: Pokemon,
  defender: Pokemon,
  context: BattleContext,
  eventManager: EventManager,
): void {
  if (defender.status) {
    eventManager.dispatch(new Event('show_text', {
      text: 'But it failed!'
    }));
    return;
  }

  defender.status = StatusCondition.BURN;
  eventManager.dispatch(
    new Event('show_text', {
      text: `${defender.nickname} was burned!`
    })
  );
  _consume_held_status_item(defender, StatusCondition.BURN, context, eventManager);
}

function _applyParalyzeEffect(
  attacker: Pokemon,
  defender: Pokemon,
  context: BattleContext,
  eventManager: EventManager,
): void {
    if (defender.status) {
        eventManager.dispatch(new Event('show_text', {
            text: 'But it failed!'
        }));
        return;
    }

    if (_statusBlockedBySafeguard(attacker, defender, context, eventManager)) {
        return;
    }

    defender.status = StatusCondition.PARALYSIS;
    eventManager.dispatch(
        new Event('show_text', {
            text: `${defender.nickname} was paralyzed!`
        })
    );
  _consume_held_status_item(defender, StatusCondition.PARALYSIS, context, eventManager);
}

function _applyThunderEffect(
  attacker: Pokemon,
  defender: Pokemon,
  damage: number,
  context: BattleContext,
  eventManager: EventManager,
): void {
  if (damage <= 0) {
    return;
  }
  _applyParalyzeEffect(attacker, defender, context, eventManager);
}

function _applyFreezeEffect(
    attacker: Pokemon,
    defender: Pokemon,
    context: BattleContext,
    eventManager: EventManager,
): void {
    if (defender.status) {
        eventManager.dispatch(new Event('show_text', {
            text: 'But it failed!'
        }));
        return;
    }

    if (_statusBlockedBySafeguard(attacker, defender, context, eventManager)) {
        return;
    }

    defender.status = StatusCondition.FREEZE;
    eventManager.dispatch(
        new Event('show_text', {
            text: `${defender.nickname} was frozen solid!`
        })
    );
    _consume_held_status_item(defender, StatusCondition.FREEZE, context, eventManager);
}

function _applyConfuseEffect(
    attacker: Pokemon,
    defender: Pokemon,
    context: BattleContext,
    eventManager: EventManager,
    gameState: GameState,
): void {
    if (defender.confusion_turns > 0) {
        eventManager.dispatch(new Event('show_text', {
            text: 'But it failed!'
        }));
        return;
    }

    if (_statusBlockedBySafeguard(attacker, defender, context, eventManager)) {
        return;
    }

    const rng = new HardwareRNG(gameState);
    defender.confusion_turns = rng.randrange(4) + 2;
    if (defender.status === StatusCondition.CONFUSION) {
        defender.status = undefined;
    }
    eventManager.dispatch(
        new Event('show_text', {
            text: `${defender.nickname} became confused!`
        })
    );
    _consume_held_status_item(defender, StatusCondition.CONFUSION, context, eventManager);
}

function _applyTrapTarget(
    attacker: Pokemon,
    defender: Pokemon,
    move: MoveData,
    context: BattleContext,
    eventManager: EventManager,
    gameState: GameState,
): void {
    if (defender.trapped_turns > 0) {
        eventManager.dispatch(new Event("show_text", { text: "But it failed!" }));
        return;
    }

    const sourceSide = context.sideFor(attacker);
    if (sourceSide === undefined) {
        throw new Error("Unable to determine trapping side.");
    }

    const rng = new HardwareRNG(gameState);
    const duration = rng.randrange(4) + 2;

    defender.trapped_turns = Math.max(1, duration);
    defender.trapped_by_side = sourceSide;
    defender.trapped_source_index = context.activeIndexFor(sourceSide);
    defender.trapped_move = move.name;

    const moveText = move.name.replace(/_/g, " ").toLowerCase();
    eventManager.dispatch(
        new Event("show_text", { text: `${defender.nickname} was trapped in ${moveText}!` })
    );

    if (context.sideFor(defender) === BattleTurn.PLAYER) {
        context.playerCantRun = true;
    }
}

function _applyRecoilHit(
    attacker: Pokemon,
    damage: number,
    eventManager: EventManager,
): void {
    if (damage > 0) {
        const recoilDamage = Math.max(1, Math.floor(damage / 4));
        attacker.hp = Math.max(0, attacker.hp - recoilDamage);
        eventManager.dispatch(
            new Event(
                "show_text",
                { text: `${attacker.nickname} was hit with recoil!` }
            )
        );
    }
}

function _applyAttackDownHit(
    defender: Pokemon,
    damage: number,
    eventManager: EventManager
): void {
    modifyStat(defender, Stat.ATTACK, -1, eventManager);
}

function _applyDefenseDownHit(
    defender: Pokemon,
    damage: number,
    eventManager: EventManager
): void {
    modifyStat(defender, Stat.DEFENSE, -1, eventManager);
}

function _applySpeedDownHit(
    defender: Pokemon,
    damage: number,
    eventManager: EventManager
): void {
    modifyStat(defender, Stat.SPEED, -1, eventManager);
}

function _applySpecialDefenseDownHit(
    defender: Pokemon,
    damage: number,
    eventManager: EventManager
): void {
    modifyStat(defender, Stat.SPECIAL_DEFENSE, -1, eventManager);
}

function _applyAccuracyDownHit(
    defender: Pokemon,
    damage: number,
    eventManager: EventManager
): void {
    modifyStat(defender, Stat.ACCURACY, -1, eventManager);
}

function _applyEvasionDownHit(
    defender: Pokemon,
    damage: number,
    eventManager: EventManager
): void {
    modifyStat(defender, Stat.EVASION, -1, eventManager);
}

function _applyLeechSeedStatus(
    attacker: Pokemon,
    defender: Pokemon,
    context: BattleContext,
    eventManager: EventManager,
): void {
    if (defender.leech_seeded) {
        eventManager.dispatch(new Event("show_text", { text: "But it failed!" }));
        return;
    }

    const defenderTypes = new Set([defender.species.type1, defender.species.type2]);
    if (defenderTypes.has(PokemonType.GRASS)) {
        eventManager.dispatch(
            new Event(
                "show_text",
                { text: `It doesn't affect ${defender.nickname}...` }
            )
        );
        return;
    }

    const sourceSide = context.sideFor(attacker);
    if (sourceSide === undefined) {
        throw new Error("Unable to determine Leech Seed source side.");
    }

    defender.leech_seeded = true;
    defender.leech_seed_source_side = sourceSide;
    eventManager.dispatch(
        new Event("show_text", { text: `${defender.nickname} was seeded!` })
    );
}

function _applyDisableEffect(
    attacker: Pokemon,
    defender: Pokemon,
    context: BattleContext,
    eventManager: EventManager,
    gameState: GameState,
): void {
    if (!defender.last_move_used) {
        eventManager.dispatch(new Event("show_text", { text: "But it failed!" }));
        return;
    }
    if (defender.disable_turns > 0 && defender.disabled_move) {
        eventManager.dispatch(new Event("show_text", { text: "But it failed!" }));
        return;
    }

    const rng = new HardwareRNG(gameState);
    defender.disabled_move = defender.last_move_used;
    defender.disable_turns = rng.randrange(4) + 4;
    const moveName = defender.last_move_used.replace(/_/g, " ").toLowerCase();
    eventManager.dispatch(
        new Event(
            "show_text",
            { text: `${defender.nickname}'s ${moveName} was disabled!` }
        )
    );
}
