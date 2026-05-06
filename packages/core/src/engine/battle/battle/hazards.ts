import { BattleContext } from './battle-context';
import { BattleTurn, PokemonType } from '@pokecrystal/core/core/enums';
import { Event, EventManager } from '../../events/events';
import type { Pokemon } from '@pokecrystal/core/core/models';

// ASM: engine/battle/move_effects/spikes.asm::BattleCommand_Spikes
// ASM: engine/battle/core.asm::SpikesDamage
const SPIKES_MAX_LAYERS_GEN2 = 1;

export class EntryHazardResult {
  public spikesDamage: number;
  public applied: boolean;
  public fainted: boolean;

  constructor({ spikesDamage = 0, applied = false, fainted = false }: {
    spikesDamage?: number;
    applied?: boolean;
    fainted?: boolean;
  } = {}) {
    this.spikesDamage = spikesDamage;
    this.applied = applied;
    this.fainted = fainted;
  }

  get totalDamage(): number {
    return this.spikesDamage;
  }
}

export function placeSpikes(
  context: BattleContext,
  userSide: BattleTurn,
  eventManager: EventManager,
): boolean {
  const targetSide = opposingSide(userSide);
  if (context.spikesLayers(targetSide) >= SPIKES_MAX_LAYERS_GEN2) {
    eventManager.dispatch(new Event('show_text', { text: 'But it failed!' }));
    return false;
  }

  context.setSpikesLayers(targetSide, SPIKES_MAX_LAYERS_GEN2);
  eventManager.dispatch(
    new Event('show_text', {
      text: `SPIKES scattered all around ${targetText(targetSide)}!`,
    })
  );
  return true;
}

export function applyEntryHazards(
  context: BattleContext,
  enteringSide: BattleTurn,
  eventManager: EventManager,
): EntryHazardResult {
  const pokemon = activePokemon(context, enteringSide);
  const spikesLayers = context.spikesLayers(enteringSide);
  if (spikesLayers <= 0) {
    return new EntryHazardResult();
  }

  if (!isGrounded(pokemon)) {
    return new EntryHazardResult();
  }

  const damage = spikesDamage(pokemon);
  pokemon.hp = Math.max(0, pokemon.hp - damage);
  eventManager.dispatch(
    new Event('show_text', { text: `${pokemon.nickname}'s hurt by SPIKES!` })
  );
  const fainted = pokemon.hp === 0;
  return new EntryHazardResult({ spikesDamage: damage, applied: true, fainted });
}

function opposingSide(side: BattleTurn): BattleTurn {
  return side === BattleTurn.PLAYER ? BattleTurn.ENEMY : BattleTurn.PLAYER;
}

function targetText(targetSide: BattleTurn): string {
  return targetSide === BattleTurn.ENEMY ? "the foe's team" : 'your team';
}

function activePokemon(context: BattleContext, side: BattleTurn): Pokemon {
  return side === BattleTurn.PLAYER ? context.playerPokemon : context.enemyPokemon;
}

function isGrounded(pokemon: Pokemon): boolean {
  const type1 = pokemon.species.type1;
  const type2 = pokemon.species.type2;
  return type1 !== PokemonType.FLYING && type2 !== PokemonType.FLYING;
}

function spikesDamage(pokemon: Pokemon): number {
  const base = Math.floor(pokemon.max_hp / 8);
  return base > 0 ? base : 1;
}
