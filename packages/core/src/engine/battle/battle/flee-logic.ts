import type { Pokemon } from '@pokecrystal/core/core/models';
import { StatusCondition, Stat } from '../../../core/enums';
import { loadAllItems } from '../../../core/data-loader';
import { HardwareRNG } from '../../games/rng';
import { Event } from '../../events/events';
import { calculateBattleStat } from './stats';
import type { Battle } from './battle-logic';
import {
  getAlwaysFleeSpecies,
  getOftenFleeSpecies,
  getSometimesFleeSpecies,
} from './flee-constants';
import { getBattleItem } from './item-lookup';

// ASM: engine/battle/core.asm::TryToRunAwayFromBattle
function _dispatch_text(battle: Battle, text: string) {
  const eventManager = battle.eventManager;
  if (!eventManager?.dispatch) {
    return;
  }
  eventManager.dispatch(new Event("show_text", { text }));
}

function _battle_speed(battle: Battle, pokemon: Pokemon): number {
  return calculateBattleStat(battle, pokemon, Stat.SPEED);
}

export function enemyShouldFlee(battle: Battle): boolean {
  if (battle.context.trainerBattle) {
    return false;
  }

  const enemy = battle.context.enemyPokemon;
  if (enemy.hp <= 0) {
    return false;
  }
  if (enemy.status === StatusCondition.SLEEP || enemy.status === StatusCondition.FREEZE) {
    return false;
  }

  const speciesId = enemy.species.id;
  if (getAlwaysFleeSpecies().has(speciesId)) {
    return true;
  }

  const rng = new HardwareRNG(battle.gameState);
  const roll = rng.randrange(256);
  if (roll >= 129) {
    return false;
  }

  if (getOftenFleeSpecies().has(speciesId)) {
    return true;
  }

  if (roll >= 26) {
    return false;
  }

  return getSometimesFleeSpecies().has(speciesId);
}

export function attemptRun(battle: Battle): boolean {
  const battleType = (battle.gameState?.wram?.battle_type ?? "BATTLETYPE_NORMAL").toUpperCase();
  if (["BATTLETYPE_DEBUG", "BATTLETYPE_CONTEST"].includes(battleType)) {
    _dispatch_text(battle, "Got away safely!");
    return true;
  }
  if (["BATTLETYPE_TRAP", "BATTLETYPE_CELEBI", "BATTLETYPE_FORCESHINY", "BATTLETYPE_SUICUNE"].includes(battleType)) {
    _dispatch_text(battle, "Can't escape!");
    return false;
  }

  if (battle.gameState?.wram?.wLinkMode) {
    _dispatch_text(battle, "Got away safely!");
    return true;
  }

  if (battle.context.trainerBattle) {
    _dispatch_text(battle, "No! There's no running from a trainer battle!");
    return false;
  }

  const player = battle.context.playerPokemon;
  if (battle.context.playerCantRun) {
    _dispatch_text(battle, "Can't escape!");
    return false;
  }

  const heldItem = getBattleItem(player.item);
  if (heldItem && heldItem.held_effect === "HELD_ESCAPE") {
    _dispatch_text(battle, `${player.nickname} fled using a ${heldItem.name}!`);
    return true;
  }

  battle.context.playerRunAttempts++;
  const attempts = battle.context.playerRunAttempts;

  const playerSpeed = _battle_speed(battle, player);
  const enemySpeed = _battle_speed(battle, battle.context.enemyPokemon);
  if (enemySpeed <= 0 || playerSpeed > enemySpeed) {
    _dispatch_text(battle, "Got away safely!");
    return true;
  }

  const baseChance = Math.floor((playerSpeed * 32) / Math.max(1, Math.floor(enemySpeed / 4)));
  if (baseChance > 255) {
    _dispatch_text(battle, "Got away safely!");
    return true;
  }

  const bonus = 30 * Math.max(0, attempts - 1);
  const chance = baseChance + bonus;
  if (chance >= 256) {
    _dispatch_text(battle, "Got away safely!");
    return true;
  }

  const rng = new HardwareRNG(battle.gameState);
  if (rng.randrange(256) < chance) {
    _dispatch_text(battle, "Got away safely!");
    return true;
  }

  _dispatch_text(battle, "Can't escape!");
  return false;
}
