import {
    Trainer
  } from '@pokecrystal/core/core/models';
import {
    Pokemon,
    PokemonSchema,
    toPokemon
  } from '@pokecrystal/core/core/models/pokemon';
  import {
    Battle
  } from './battle-logic';
import {
    StatusCondition
  } from '@pokecrystal/core/core/enums';
import { isAsleepTreeMon, TREEMON_SLEEP_TURNS } from '@pokecrystal/assets/content/tree-encounters';
  import {
    clearTransientStatus
  } from './status-effects';
  import {
    recordPokedexSeen
  } from '@pokecrystal/core/core/pokedex';
  import {
    GameState
  } from '@pokecrystal/core/core/state';

  function isFitForBattle(pokemon: Pokemon | null): boolean {
    if (!pokemon) {
      return false;
    }
    if (pokemon.hp <= 0) {
      return false;
    }
    const speciesId = pokemon.species.id.toUpperCase();
    if (speciesId === 'EGG') {
      return false;
    }
    const nickname = (pokemon.nickname || '').toUpperCase();
    if (nickname === 'EGG') {
      return false;
    }
    return true;
  }

  type BattlePokemon = Pokemon & { _sram_slot?: number };

  export function initialisePlayerParty(
    battle: Battle,
    active: Pokemon,
    providedParty ? : (Pokemon | null) [],
  ): BattlePokemon[] {
    let party: BattlePokemon[] = [];
    if (providedParty) {
      party = providedParty.filter((p) => p !== null) as BattlePokemon[];
    } else {
      const sramParty = battle.gameState.sram.party;
      if (sramParty) {
        for (let index = 0; index < sramParty.pokemon.length; index += 1) {
          const pokemon = sramParty.pokemon[index];
          if (!pokemon) {
            continue;
          }
          const clone = toPokemon(pokemon) as BattlePokemon;
          clone._sram_slot = index;
          party.push(clone);
        }
      }
    }

    if (!party.length) {
      party = [active];
    } else if (!party.includes(active)) {
      party.unshift(active);
    } else {
      const activeIndex = party.indexOf(active);
      if (activeIndex !== 0) {
        [party[0], party[activeIndex]] = [party[activeIndex], party[0]];
      }
    }

    const firstFitIndex = party.findIndex(isFitForBattle);
    if (firstFitIndex === -1) {
      return [];
    }
    if (firstFitIndex !== 0) {
      [party[0], party[firstFitIndex]] = [party[firstFitIndex], party[0]];
    }

    for (const pokemon of party) {
      clearTransientStatus(pokemon);
    }

    return party;
  }

  export function initialiseEnemyParty(
    battle: Battle,
    active: Pokemon,
    providedParty ? : (Pokemon | null) [],
    trainer ? : Trainer,
  ): Pokemon[] {
    let clones: Pokemon[] = [];
    let sourceParty: (Pokemon | null)[] | undefined;

    if (providedParty) {
      sourceParty = providedParty;
    } else if (trainer && trainer.party) {
      sourceParty = trainer.party.map((p) => toPokemon(p));
    }

    if (sourceParty) {
      clones = sourceParty.filter((p) => p !== null).map((p) => cloneForBattle(toPokemon(p as any)));
    }

    if (!clones.length) {
      clones = [cloneForBattle(toPokemon(active))];
    } else {
      let activeIndex = 0;
      if (sourceParty) {
        activeIndex = sourceParty.indexOf(active);
        if (activeIndex === -1) {
          activeIndex = 0;
        }
      }
      if (activeIndex < clones.length) {
        [clones[0], clones[activeIndex]] = [clones[activeIndex], clones[0]];
      }
    }

    if (!trainer && clones.length > 0) {
      _resetWildEnemyStatus(clones[0]);
      _applyTreeMonSleep(battle, clones[0]);
    }

    return clones;
  }

  function _resetWildEnemyStatus(pokemon: Pokemon): void {
    pokemon.status = undefined;
    pokemon.sleep_turns = 0;
    pokemon.turns_in_battle = 0;
    pokemon.flinching = false;
    pokemon.rampage_turns = 0;
    pokemon.confusion_turns = 0;
    pokemon.perish_song_turns = 0;
    pokemon.trapped_turns = 0;
    pokemon.trapped_by_side = undefined;
    pokemon.trapped_source_index = undefined;
    pokemon.trapped_move = undefined;
    pokemon.leech_seeded = false;
    pokemon.leech_seed_source_side = undefined;
    pokemon.nightmare = false;
    pokemon.cursed = false;
    pokemon.curse_source_side = undefined;
    clearTransientStatus(pokemon);
  }

  // ASM: engine/battle/core.asm::CheckSleepingTreeMon.
  function _applyTreeMonSleep(battle: Battle, pokemon: Pokemon): void {
    const battleType = String(battle.gameState?.wram?.battle_type ?? '').toUpperCase();
    if (battleType !== 'BATTLETYPE_TREE') {
      return;
    }
    const timeOfDay = battle.gameState?.wram?.time_of_day ?? 'day';
    const speciesId = pokemon?.species?.id ?? '';
    if (!isAsleepTreeMon(speciesId, timeOfDay)) {
      return;
    }
    pokemon.status = StatusCondition.SLEEP;
    pokemon.sleep_turns = TREEMON_SLEEP_TURNS;
  }

  export function cloneForBattle(pokemon: Pokemon): Pokemon {
    return toPokemon(PokemonSchema.parse(pokemon));
  }

  export function recordEnemySeen(battle: Battle): void {
    const enemy = battle.context.enemyPokemon;
    if (enemy) {
      recordPokedexSeen(battle.gameState, enemy.species);
    }
  }
