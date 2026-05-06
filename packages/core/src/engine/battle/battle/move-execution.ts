import {
    Pokemon,
    Item,
    LearnedMove,
    Move as MoveData,
  } from '@pokecrystal/core/core/models';
import {
    BattleScene,
    BattleTurn,
    MoveName,
    MoveEffect,
    Stat,
    StatusCondition,
    PokemonType,
  } from '@pokecrystal/core/core/enums';
  import {
    Battle
  } from './battle-logic';
import { Weather } from './battle-context';
  import {
    DamageResult,
    calculateDamage,
    calculateTypeEffectivenessMultiplier
  } from './damage-calculation';
import {
    applyMoveEffect
  } from './move-effects';
import { endEncore } from './status-effects';
  import {
    HardwareRNG
  } from '@pokecrystal/core/engine/games/rng';
import {
    EventManager,
    Event
  } from '@pokecrystal/core/engine/events/events';
import { pushDebugLog } from '@pokecrystal/core/core/debug-log';
import { GameState } from '@pokecrystal/core/core/state';
import { loadAllItems, loadAllMoves } from '@pokecrystal/core/core/data-loader';
import { animation_label_for_move } from '@pokecrystal/core/ui/overlays/battle-animation-util';
import Fraction from 'fraction.js';


  const ACCURACY_LEVEL_MULTIPLIERS: [number, number][] = [
    [33, 100],
    [36, 100],
    [43, 100],
    [50, 100],
    [60, 100],
    [75, 100],
    [1, 1],
    [133, 100],
    [166, 100],
    [2, 1],
    [233, 100],
    [133, 50],
    [3, 1],
  ];

  const CRITICAL_THRESHOLDS: number[] = [17, 32, 64, 85, 128, 128, 128];
  const TYPE_IMMUNITIES: Partial<Record<PokemonType, Set<PokemonType>>> = {
    [PokemonType.NORMAL]: new Set([PokemonType.GHOST]),
    [PokemonType.FIGHTING]: new Set([PokemonType.GHOST]),
    [PokemonType.POISON]: new Set([PokemonType.STEEL]),
    [PokemonType.GROUND]: new Set([PokemonType.FLYING]),
    [PokemonType.GHOST]: new Set([PokemonType.NORMAL]),
    [PokemonType.ELECTRIC]: new Set([PokemonType.GROUND]),
    [PokemonType.PSYCHIC_TYPE]: new Set([PokemonType.DARK]),
  };
  const HIGH_CRITICAL_MOVES: Set < MoveName > = new Set([
    MoveName.KARATE_CHOP,
    MoveName.RAZOR_WIND,
    MoveName.RAZOR_LEAF,
    MoveName.CRABHAMMER,
    MoveName.SLASH,
    MoveName.AEROBLAST,
    MoveName.CROSS_CHOP,
  ]);
  let ITEM_CACHE: Map < string,
  Item > | null = null;
  const TWO_TURN_MOVES_TEXT: Map < MoveName,
  string > = new Map([
    [MoveName.FLY, 'flew up high!'],
    [MoveName.DIG, 'dug a hole!'],
    [MoveName.SOLARBEAM, 'took in sunlight!'],
    [MoveName.SKULL_BASH, 'lowered its head!'],
    [MoveName.SKY_ATTACK, 'is glowing!'],
    [MoveName.RAZOR_WIND, 'whipped up a whirlwind!'],
  ]);

  const log_move_start = (
    attackerKey: BattleTurn,
    attacker: Pokemon,
    defender: Pokemon,
    move: MoveData,
  ): void => {
    pushDebugLog("[battle] move start", {
      side: attackerKey,
      attacker: attacker.nickname,
      move: move.name,
      target: defender.nickname,
      attacker_hp: attacker.hp,
      attacker_max_hp: attacker.max_hp,
      defender_hp: defender.hp,
      defender_max_hp: defender.max_hp,
    });
  };

  const log_hp_change = (
    pokemon: Pokemon,
    before: number,
    after: number,
    details: {
      source: "damage" | "effect";
      move: string;
      attacker: string;
      target: string;
      side: BattleTurn;
      damage?: number;
    },
  ): void => {
    if (before === after) {
      return;
    }
    pushDebugLog("[battle] hp", {
      pokemon: pokemon.nickname,
      from: before,
      to: after,
      delta: after - before,
      max_hp: pokemon.max_hp,
      ...details,
    });
  };

  function normalizeItemName(name ? : string): string | undefined {
    if (!name) {
      return undefined;
    }
    const cleaned = name.replace(/[-_]/g, ' ').toUpperCase();
    return cleaned.split(' ').filter(Boolean).join(' ');
  }

  function getHeldItem(itemName ? : string): Item | undefined {
    if (!itemName) {
      return undefined;
    }

    if (ITEM_CACHE === null) {
      ITEM_CACHE = new Map(loadAllItems());
    }

    const normalized = normalizeItemName(itemName);
    if (!normalized) {
      return undefined;
    }

    const item = ITEM_CACHE.get(normalized);
    if (item) {
      return item;
    }

    const underscored = normalized.replace(/ /g, '_');
    return ITEM_CACHE.get(underscored);
  }

  function maybeConsumeHeldHpItem(battle: Battle, pokemon: Pokemon): boolean {
    if (pokemon.hp <= 0) {
      return false;
    }

    const item = getHeldItem(pokemon.item ?? undefined);
    if (!item || item.held_effect !== 'HELD_BERRY') {
      return false;
    }

    if (pokemon.hp > Math.floor(pokemon.max_hp / 2)) {
      return false;
    }

    const healAmount = Math.max(1, Number(item.parameter ?? 0));
    const beforeHp = pokemon.hp;
    pokemon.hp = Math.min(pokemon.max_hp, pokemon.hp + healAmount);
    pokemon.item = undefined;
    log_hp_change(pokemon, beforeHp, pokemon.hp, {
      source: "effect",
      move: item.name ?? "ITEM",
      attacker: pokemon.nickname,
      target: pokemon.nickname,
      side: BattleTurn.PLAYER,
      damage: -(pokemon.hp - beforeHp),
    });
    battle.eventManager.dispatch(
      new Event('show_text', { text: `${pokemon.nickname} recovered using a ${item.name}!` })
    );
    return true;
  }

  function stageToIndex(stage: number): number {
    const clamped = Math.max(-6, Math.min(6, stage));
    return clamped + 6;
  }

  function applyAccuracyModifier(value: number, stage: number): number {
    const [numerator, denominator] = ACCURACY_LEVEL_MULTIPLIERS[stageToIndex(stage)];
    const adjusted = Math.floor((value * numerator) / denominator);
    return Math.max(1, adjusted);
  }

  function percentToByte(percent: number): number {
    return Math.max(0, Math.min(255, Math.floor((percent * 255) / 100)));
  }

  function accuracyByte(
    move: MoveData,
    attacker: Pokemon,
    defender: Pokemon,
    ignoreEvasion = false
  ): number {
    const base = percentToByte(move.accuracy);
    const accuracyStage = attacker.stat_boosts[Stat.ACCURACY] || 0;
    const evasionStage = ignoreEvasion ? 0 : defender.stat_boosts[Stat.EVASION] || 0;
    let modified = applyAccuracyModifier(base, accuracyStage);
    modified = applyAccuracyModifier(modified, -evasionStage);
    if (defender.item === 'BRIGHTPOWDER') {
      modified = Math.floor(modified * (230 / 255));
    }
    return Math.min(255, Math.max(1, modified));
  }

  function defenderTypesFor(pokemon: Pokemon): PokemonType[] {
    const defenderTypes: PokemonType[] = [];
    if (pokemon.species.type1) {
      defenderTypes.push(pokemon.species.type1);
    }
    if (pokemon.species.type2 && pokemon.species.type2 !== pokemon.species.type1) {
      defenderTypes.push(pokemon.species.type2);
    }
    return defenderTypes;
  }

  function hasTypeImmunity(moveType: PokemonType, defenderTypes: PokemonType[]): boolean {
    const immunities = TYPE_IMMUNITIES[moveType];
    return immunities !== undefined && defenderTypes.some((type) => immunities.has(type));
  }

  function sampleRandomByte(
    battle: Battle,
    rng: HardwareRNG,
    respectPredefined = true
  ): number {
    if (respectPredefined && battle.context.predefinedRandomValue !== undefined) {
      const clamped = Math.max(0.0, Math.min(1.0, battle.context.predefinedRandomValue));
      return Math.floor(clamped * 255);
    }
    return rng.nextByte();
  }

  function getLearnedMove(pokemon: Pokemon, moveName: MoveName): LearnedMove | undefined {
    for (const learnedMove of pokemon.moves) {
      if (learnedMove?.name === moveName) {
        return learnedMove;
      }
    }
    return undefined;
  }

  function shouldPlayBattleScene(gameState: GameState | null): boolean {
    if (gameState?.wram?.instant_mode) {
      return false;
    }
    const options = gameState?.sram?.options as { battle_scene?: unknown } | undefined;
    if (!options || options.battle_scene === undefined) {
      return true;
    }
    const setting = options.battle_scene as unknown;
    if (typeof setting === 'boolean') {
      return setting;
    }
    return setting === BattleScene.ON;
  }

  const resolveMoveDefinition = (battle: Battle, moveName: MoveName): MoveData => {
    const loaded = battle.movesMap.get(moveName);
    if (loaded) {
      return loaded;
    }

    const fallback = loadAllMoves().get(moveName);
    if (fallback) {
      battle.movesMap.set(moveName, fallback);
      return fallback;
    }

    throw new Error(`Battle move data is missing for ${String(moveName)}.`);
  };

  const clone_move = (move: MoveData, overrides: Partial<MoveData>): MoveData => ({
    ...move,
    ...overrides,
  });

  const PHYSICAL_TYPES: Set<PokemonType> = new Set([
    PokemonType.NORMAL,
    PokemonType.FIGHTING,
    PokemonType.FLYING,
    PokemonType.POISON,
    PokemonType.GROUND,
    PokemonType.ROCK,
    PokemonType.BUG,
    PokemonType.GHOST,
    PokemonType.STEEL,
  ]);

  const HIDDEN_POWER_TYPES: PokemonType[] = [
    PokemonType.FIGHTING,
    PokemonType.FLYING,
    PokemonType.POISON,
    PokemonType.GROUND,
    PokemonType.ROCK,
    PokemonType.BUG,
    PokemonType.GHOST,
    PokemonType.STEEL,
    PokemonType.FIRE,
    PokemonType.WATER,
    PokemonType.GRASS,
    PokemonType.ELECTRIC,
    PokemonType.PSYCHIC_TYPE,
    PokemonType.ICE,
    PokemonType.DRAGON,
    PokemonType.DARK,
  ];

  const sample_move_random_byte = (battle: Battle): number => {
    if (battle.context.predefinedRandomValue !== undefined) {
      const clamped = Math.max(0.0, Math.min(1.0, battle.context.predefinedRandomValue));
      return Math.floor(clamped * 255);
    }
    return new HardwareRNG(battle.gameState).nextByte();
  };

  const reversal_power = (attacker: Pokemon): number => {
    const maxHp = Math.max(1, attacker.max_hp);
    const ratio = (attacker.hp * 48) / maxHp;
    if (ratio <= 1) return 200;
    if (ratio <= 4) return 150;
    if (ratio <= 9) return 100;
    if (ratio <= 16) return 80;
    if (ratio <= 32) return 40;
    return 20;
  };

  const return_power = (attacker: Pokemon): number =>
    Math.max(1, Math.floor((Math.max(0, Math.min(255, attacker.happiness)) * 10) / 25));

  const frustration_power = (attacker: Pokemon): number => {
    const unhappiness = 255 - Math.max(0, Math.min(255, attacker.happiness));
    return Math.max(1, Math.floor((unhappiness * 10) / 25));
  };

  const prepare_move_for_damage = (move: MoveData, attacker: Pokemon): MoveData => {
    switch (move.effect) {
      case MoveEffect.RETURN:
        return clone_move(move, { power: return_power(attacker) });
      case MoveEffect.FRUSTRATION:
        return clone_move(move, { power: frustration_power(attacker) });
      case MoveEffect.REVERSAL:
        return clone_move(move, { power: reversal_power(attacker) });
      default:
        return move;
    }
  };

  const present_roll = (
    battle: Battle,
    move: MoveData,
    defender: Pokemon,
  ): { move: MoveData | null; healing: number } => {
    const roll = sample_move_random_byte(battle);
    if (roll <= 102) {
      return { move: clone_move(move, { power: 40 }), healing: 0 };
    }
    if (roll <= 178) {
      return { move: clone_move(move, { power: 80 }), healing: 0 };
    }
    if (roll <= 204) {
      return { move: clone_move(move, { power: 120 }), healing: 0 };
    }
    if (defender.hp >= defender.max_hp) {
      return { move: null, healing: 0 };
    }
    return { move: null, healing: Math.max(1, Math.floor(defender.max_hp / 4)) };
  };

  const magnitude_power = (battle: Battle): number => {
    const roll = sample_move_random_byte(battle);
    if (roll <= 12) return 10;
    if (roll <= 38) return 30;
    if (roll <= 89) return 50;
    if (roll <= 166) return 70;
    if (roll <= 217) return 90;
    if (roll <= 242) return 110;
    return 150;
  };

  const hidden_power_move = (move: MoveData, attacker: Pokemon): MoveData => {
    const attack = attacker.dvs.attack & 0xf;
    const defense = attacker.dvs.defense & 0xf;
    const speed = attacker.dvs.speed & 0xf;
    const special = attacker.dvs.special & 0xf;
    const typeIndex = ((attack & 0x3) << 2) | (defense & 0x3);
    const power =
      Math.floor(
        (((attack >> 3) + ((defense >> 3) << 1) + ((speed >> 3) << 2) + ((special >> 3) << 3)) * 5 +
          (special & 0x3)) /
          2
      ) + 31;

    return clone_move(move, {
      type: HIDDEN_POWER_TYPES[typeIndex] ?? PokemonType.NORMAL,
      power: Math.max(31, Math.min(70, power)),
    });
  };

  const counter_damage = (
    battle: Battle,
    attackerKey: BattleTurn,
    attacker: Pokemon,
    move: MoveData,
  ): number | null => {
    const lastDamage = Math.max(0, attacker.last_damage_taken ?? 0);
    const lastType = attacker.last_damage_type;
    if (lastDamage <= 0 || !lastType) {
      return null;
    }

    const requiresPhysical = move.effect === MoveEffect.COUNTER;
    if (PHYSICAL_TYPES.has(lastType) !== requiresPhysical) {
      return null;
    }

    const order = battle.context.turnOrder ?? [];
    const attackerIndex = order.indexOf(attackerKey);
    const defenderKey = attackerKey === BattleTurn.PLAYER ? BattleTurn.ENEMY : BattleTurn.PLAYER;
    const defenderIndex = order.indexOf(defenderKey);
    if (attackerIndex !== -1 && defenderIndex !== -1 && attackerIndex < defenderIndex) {
      return null;
    }

    return Math.min(0xffff, lastDamage * 2);
  };

  const prepare_dynamic_move = (
    battle: Battle,
    attackerKey: BattleTurn,
    attacker: Pokemon,
    defender: Pokemon,
    move: MoveData,
  ): { move: MoveData | null; specialDamage: number | null; handled: boolean } => {
    switch (move.effect) {
      case MoveEffect.PRESENT: {
        const result = present_roll(battle, move, defender);
        if (result.healing > 0) {
          const before = defender.hp;
          defender.hp = Math.min(defender.max_hp, defender.hp + result.healing);
          battle.eventManager.dispatch(new Event('show_text', { text: `${defender.nickname} regained health!` }));
          log_hp_change(defender, before, defender.hp, {
            source: "effect",
            move: move.name,
            attacker: attacker.nickname,
            target: defender.nickname,
            side: attackerKey,
            damage: -(defender.hp - before),
          });
        } else if (result.move === null) {
          battle.eventManager.dispatch(new Event('show_text', { text: "But it failed!" }));
        }
        return { move: result.move, specialDamage: null, handled: result.move === null };
      }
      case MoveEffect.MAGNITUDE:
        return { move: clone_move(move, { power: magnitude_power(battle) }), specialDamage: null, handled: false };
      case MoveEffect.HIDDEN_POWER:
        return { move: hidden_power_move(move, attacker), specialDamage: null, handled: false };
      case MoveEffect.COUNTER:
      case MoveEffect.MIRROR_COAT: {
        const damage = counter_damage(battle, attackerKey, attacker, move);
        return { move, specialDamage: damage, handled: damage === null };
      }
      default:
        return { move, specialDamage: null, handled: false };
    }
  };

  const resolve_special_damage = (
    battle: Battle,
    attacker: Pokemon,
    defender: Pokemon,
    move: MoveData,
  ): number | null => {
    switch (move.effect) {
      case MoveEffect.STATIC_DAMAGE:
        if (move.name === MoveName.SONICBOOM) {
          return 20;
        }
        if (move.name === MoveName.DRAGON_RAGE) {
          return 40;
        }
        return Math.max(1, move.power);
      case MoveEffect.LEVEL_DAMAGE:
        return Math.max(1, attacker.level);
      case MoveEffect.SUPER_FANG:
        return Math.max(1, Math.floor(defender.hp / 2));
      case MoveEffect.PSYWAVE: {
        const rng = new HardwareRNG(battle.gameState);
        const roll =
          battle.context.predefinedRandomValue !== undefined
            ? Math.max(0, Math.min(1, battle.context.predefinedRandomValue))
            : rng.nextByte() / 255;
        const scaled = Math.floor(attacker.level * (0.5 + roll));
        return Math.max(1, scaled);
      }
      default:
        return null;
    }
  };

export function executeMove(
    battle: Battle,
    attackerKey: BattleTurn,
    attacker: Pokemon,
    defender: Pokemon,
    moveName: MoveName
  ): void {
    if (
      attacker.encore_turns_remaining > 0 &&
      attacker.encored_move &&
      getLearnedMove(attacker, attacker.encored_move)
    ) {
      moveName = attacker.encored_move;
      attacker.encore_turns_remaining = Math.max(0, attacker.encore_turns_remaining - 1);
      if (attacker.encore_turns_remaining === 0) {
        endEncore(battle.eventManager, attacker, false);
      }
    }

    const move = resolveMoveDefinition(battle, moveName);

    reset_rage_if_move_changes(attacker, move);

    const displayName = move.name.replace(/_/g, ' ');

    if (
      attacker.disable_turns > 0 &&
      attacker.disabled_move &&
      attacker.disabled_move === move.name
    ) {
      battle.eventManager.dispatch(
        new Event(
          'show_text', {
            text: `${attacker.nickname}'s ${displayName} is disabled!`
          }
        )
      );
      return;
    }

    const learnedMove = getLearnedMove(attacker, moveName);
    if (learnedMove && learnedMove.current_pp === 0) {
      battle.eventManager.dispatch(
        new Event(
          'show_text', {
            text: `${attacker.nickname} has no PP left for ${displayName}!`
          }
        )
      );
      return;
    }

    attacker.last_move_used = move.name;

    if (learnedMove) {
      learnedMove.current_pp = Math.max(0, learnedMove.current_pp - 1);
    }

    log_move_start(attackerKey, attacker, defender, move);

    battle.eventManager.dispatch(
      new Event('show_text', {
        text: `${attacker.nickname} used ${displayName}!`
      })
    );

    const hitResult = checkMove(battle, attacker, defender, move);
    if (
      attacker.lock_on_active &&
      attacker.lock_on_target_index !== undefined &&
      battle.context.activeIndexFor(battle.context.sideFor(defender) ?? BattleTurn.ENEMY) === attacker.lock_on_target_index &&
      move.effect !== MoveEffect.LOCK_ON
    ) {
      attacker.lock_on_active = false;
      attacker.lock_on_target_index = undefined;
    }
    if (hitResult === 'miss') {
      battle.eventManager.dispatch(
        new Event('show_text', {
          text: `${attacker.nickname}'s attack missed!`
        })
      );
      return;
    }
    if (hitResult === 'no_effect') {
      if (shouldPlayBattleScene(battle.gameState)) {
        battle.eventManager.dispatch(
          new Event('play_animation', {
            move_name: move.name,
            animation_label: animation_label_for_move(move.name),
            is_player_move: attackerKey === BattleTurn.PLAYER,
          })
        );
      }
      dispatchEffectivenessText(
        battle.eventManager,
        defender,
        new Fraction(0),
        move,
      );
      return;
    }

    const dynamic = prepare_dynamic_move(battle, attackerKey, attacker, defender, move);
    if (dynamic.handled) {
      return;
    }
    const damageMove = prepare_move_for_damage(dynamic.move ?? move, attacker);

    if (shouldPlayBattleScene(battle.gameState)) {
      // ASM alignment: usedmovetext -> checkhit -> moveanim (engine/battle/effect_commands.asm)
      // checkhit sets wAttackMissed and blocks animation; type-ineffective moves must still animate
      // and continue into failuretext for "doesn't affect" (data/moves/effects.asm).
      battle.eventManager.dispatch(
        new Event('play_animation', {
          move_name: move.name,
          animation_label: animation_label_for_move(move.name),
          is_player_move: attackerKey === BattleTurn.PLAYER,
        })
      );
    }

    if (defender.protect_active) {
      battle.eventManager.dispatch(
        new Event('show_text', {
          text: `${defender.nickname} protected itself!`
        })
      );
      return;
    }

    const damageResult: DamageResult = calculateDamage(
      attacker,
      defender,
      damageMove,
      battle.context,
    );
    const baseDamage = dynamic.specialDamage ?? resolve_special_damage(battle, attacker, defender, damageMove) ?? damageResult.damage;
    const modifiedDamage = applyContinuousMoveModifiers(attacker, damageMove, baseDamage)
    const inflicted = applyDamage(battle, attackerKey, attacker, defender, damageMove, modifiedDamage);
    dispatchEffectivenessText(
      battle.eventManager,
      defender,
      damageResult.type_multiplier,
      damageMove,
    );
    if (
      damageResult.type_multiplier.compare(0) === 0 ||
      hasTypeImmunity(damageMove.type, defenderTypesFor(defender))
    ) {
      return;
    }
    if (inflicted > 0 && damageMove.effect === MoveEffect.PAY_DAY) {
      battle.context.payDayMoney += attacker.level * 5;
    }
    build_opponent_rage(battle, defender, inflicted);
    const attackerHpBeforeEffect = attacker.hp;
    const defenderHpBeforeEffect = defender.hp;
    applyMoveEffect(
      attacker,
      defender,
      damageMove,
      inflicted,
      battle.context,
      battle.eventManager,
      battle.gameState,
      battle,
    );
    log_hp_change(attacker, attackerHpBeforeEffect, attacker.hp, {
      source: "effect",
      move: damageMove.name,
      attacker: attacker.nickname,
      target: defender.nickname,
      side: attackerKey,
    });
    log_hp_change(defender, defenderHpBeforeEffect, defender.hp, {
      source: "effect",
      move: damageMove.name,
      attacker: attacker.nickname,
      target: defender.nickname,
      side: attackerKey,
    });
  }


  function checkMove(
    battle: Battle,
    attacker: Pokemon,
    defender: Pokemon,
    move: MoveData
  ): 'hit' | 'miss' | 'no_effect' {
    const defenderSide = battle.context.sideFor(defender);
    if (
      attacker.lock_on_active &&
      attacker.lock_on_target_index !== undefined &&
      defenderSide !== undefined &&
      battle.context.activeIndexFor(defenderSide) === attacker.lock_on_target_index
    ) {
      return 'hit';
    }

    const defenderTypes = defenderTypesFor(defender);
    if (move.effect === MoveEffect.ALWAYS_HIT) {
      return 'hit';
    }
    if (move.effect === MoveEffect.THUNDER && battle.context.weather === Weather.RAIN) {
      return 'hit';
    }

    if (move.accuracy === 0) {
      return 'hit';
    }

    const rng = new HardwareRNG(battle.gameState);
    const accuracyValue =
      move.effect === MoveEffect.THUNDER && battle.context.weather === Weather.SUN
        ? percentToByte(50) + 1
        : accuracyByte(move, attacker, defender);

    const roll = sampleRandomByte(battle, rng);
    if (roll >= accuracyValue) {
      return 'miss';
    }

    // ASM alignment: type immunity is a separate failure case (DoesntAffectText), not a miss.
    // wAttackMissed in ASM is only set by checkhit, so we continue to animation for no-effect.
    if (hasTypeImmunity(move.type, defenderTypes)) {
      return 'no_effect';
    }
    const typeMultiplier = calculateTypeEffectivenessMultiplier(move.type, defenderTypes);
    if (typeMultiplier.compare(0) === 0) {
      return 'no_effect';
    }

    return 'hit';
  }

  function applyDamage(
    battle: Battle,
    attackerKey: BattleTurn,
    attacker: Pokemon,
    defender: Pokemon,
    move: MoveData,
    damage: number,
  ): number {
    if (damage <= 0) {
      return 0;
    }

    const beforeHp = defender.hp;
    let inflicted = Math.min(defender.hp, damage);
    if (move.effect === MoveEffect.FALSE_SWIPE && defender.hp > 1) {
      inflicted = Math.min(inflicted, defender.hp - 1);
    }
    if (defender.endure_active && defender.hp > 1) {
      inflicted = Math.min(inflicted, defender.hp - 1);
    }

    if (inflicted < 0) {
      return 0;
    }

    defender.hp = Math.max(0, defender.hp - inflicted);
    defender.last_damage_taken = inflicted;
    defender.last_damage_type = move.type;
    maybeConsumeHeldHpItem(battle, defender);
    log_hp_change(defender, beforeHp, defender.hp, {
      source: "damage",
      move: move.name,
      attacker: attacker.nickname,
      target: defender.nickname,
      side: attackerKey,
      damage: inflicted,
    });
    return inflicted;
  }

function applyContinuousMoveModifiers(
    attacker: Pokemon,
    move: MoveData,
    damage: number
  ): number {
    if (damage <= 0) {
      return 0;
    }

    let adjusted = damage;

    if (move.effect === MoveEffect.FURY_CUTTER) {
        attacker.fury_cutter_count = Math.min((attacker.fury_cutter_count ?? 0) + 1, 5);
        const stage = attacker.fury_cutter_count;
        const multiplier = 1 << Math.max(0, stage - 1);
        adjusted = Math.min(0xFFFF, damage * multiplier);
    } else {
        attacker.fury_cutter_count = 0;
    }

    if (move.effect === MoveEffect.ROLLOUT) {
        attacker.rollout_step = Math.min(attacker.rollout_step + 1, 5);
        let stage = attacker.rollout_step;
        if (attacker.defense_curled) {
            stage++;
        }
        const multiplier = 1 << Math.max(0, stage - 1);
        adjusted = Math.min(0xFFFF, damage * multiplier);
        attacker.rollout_active = attacker.rollout_step < 5;
        if (attacker.rollout_active) {
            attacker.locked_move = MoveName.ROLLOUT;
            attacker.locked_turns_remaining = Math.max(attacker.locked_turns_remaining ?? 0, 1);
        } else {
            attacker.locked_move = undefined;
            attacker.locked_turns_remaining = 0;
            attacker.rollout_step = 0;
        }
    } else {
        if (!attacker.rollout_active) {
            attacker.rollout_step = 0;
        }
    }

    return Math.max(0, adjusted);
}

function reset_rage_if_move_changes(attacker: Pokemon, move: MoveData): void {
  if (move.effect === MoveEffect.RAGE) {
    return;
  }

  if (attacker.rage_active || attacker.rage_counter) {
    // ASM mapping: pokecrystal_disassembly/engine/battle/core.asm (ResetVarsForSubstatusRage).
    attacker.rage_active = false;
    attacker.rage_counter = 0;
  }
}

function build_opponent_rage(battle: Battle, defender: Pokemon, inflicted: number): void {
  if (inflicted <= 0 || defender.hp <= 0 || !defender.rage_active) {
    return;
  }

  const current = defender.rage_counter ?? 0;
  if (current >= 0xFF) {
    return;
  }
  defender.rage_counter = current + 1;

  // ASM mapping: pokecrystal_disassembly/engine/battle/effect_commands.asm (BattleCommand_BuildOpponentRage).
  battle.eventManager.dispatch(
    new Event("show_text", { text: `${defender.nickname}'s RAGE is building!` })
  );
}

  function dispatchEffectivenessText(
    eventManager: EventManager,
    defender: Pokemon,
    multiplier: Fraction,
    move: MoveData,
): void {
    if (multiplier.compare(1) === 0) {
      return;
    }
    if (move.power <= 0 && multiplier.compare(0) !== 0) {
      return;
    }

    let text: string;
    if (multiplier.compare(0) === 0) {
      text = `It doesn't affect\n${defender.nickname}!`;
    } else if (multiplier.compare(1) > 0) {
      text = "It's super-\neffective!";
    } else {
      text = "It's not very\neffective…";
    }
    eventManager.dispatch(new Event('show_text', {
      text
    }));
  }
