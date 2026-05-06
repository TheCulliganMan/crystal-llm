import {
    Pokemon
  } from '@pokecrystal/core/core/models';
  import {
    BattleTurn,
    StatusCondition
  } from '@pokecrystal/core/core/enums';
  import {
    Battle
  } from './battle-logic';
import {
    clearTransientStatus
  } from './status-effects';
import { restoreTransformState } from './transform-state';
import {
    grantPlayerExperience
  } from './experience';
  import {
    handlePostBattleRoamers
  } from '@pokecrystal/core/engine/world/roamers';
import {
    Event
  } from '@pokecrystal/core/engine/events/events';
import { MAX_MONEY } from '@pokecrystal/core/core/constants';
import { pushDebugLog } from '@pokecrystal/core/core/debug-log';

const ASM_MAX_BATTLE_MONEY = 0xff_ff_ff;

const toAsmBattleMoney = (value: number, label: string): number => {
  if (!Number.isFinite(value)) {
    throw new Error(`${label} must be a finite number.`);
  }
  if (value < 0) {
    throw new Error(`${label} cannot be negative.`);
  }
  return Math.min(ASM_MAX_BATTLE_MONEY, Math.trunc(value));
};

const doubleAsmBattleMoney = (value: number): number => {
  const amount = toAsmBattleMoney(value, "ASM battle money");
  if (amount > (ASM_MAX_BATTLE_MONEY >> 1)) {
    return ASM_MAX_BATTLE_MONEY;
  }
  return amount * 2;
};

const addToWallet = (current: number, amount: number): number =>
  Math.min(MAX_MONEY, current + amount);

  function _resetPartyAfterBattle(party: (Pokemon | null)[]): void {
    for (const pokemon of party) {
      if (pokemon) {
        restoreTransformState(pokemon);
        clearTransientStatus(pokemon);
        pokemon.turns_in_battle = 0;
        pokemon.last_move_used = undefined;
        pokemon.flinching = false;
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
        pokemon.rampage_turns = 0;
        pokemon.locked_move = undefined;
        pokemon.locked_turns_remaining = 0;
        pokemon.fury_cutter_count = 0;
        pokemon.defense_curled = false;
        pokemon.bide_active = false;
        pokemon.bide_turns_remaining = 0;
        pokemon.bide_damage = 0;
        if (pokemon.status === StatusCondition.CONFUSION) {
          pokemon.status = undefined;
        }
      }
    }
  }

  export function handleFaint(battle: Battle, side: BattleTurn): boolean {
    const active =
      side === BattleTurn.PLAYER ?
      battle.context.playerPokemon :
      battle.context.enemyPokemon;
    if (active.hp > 0) {
      battle._activeFaintSides.delete(side);
      return false;
    }

    const noReplacement = battle.context.availablePartyIndices(side, true).length === 0;
    if (battle._activeFaintSides.has(side)) {
      return noReplacement;
    }
    battle._activeFaintSides.add(side);

    const index = battle.context.activeIndexFor(side);
    const nickname = active.nickname || '<unnamed>';
    const speciesName = String(active.species?.id ?? 'UNKNOWN').toUpperCase();
    pushDebugLog('[battle] fainted', {
      side: side === BattleTurn.PLAYER ? 'PLAYER' : 'ENEMY',
      index,
      nickname,
      species: speciesName,
      hp: active.hp,
    });

    active.hp = 0;
    // ASM: pokecrystal_disassembly/data/text/battle.asm::BattleText_MonFainted / BattleText_EnemyMonFainted
    const faintedText = side === BattleTurn.ENEMY
      ? `Enemy ${active.nickname} fainted!`
      : `${active.nickname} fainted!`;
    battle.eventManager.dispatch(new Event('show_text', { text: faintedText }));
    if (side === BattleTurn.PLAYER) {
      battle.context.markPlayerFainted(battle.context.activeIndexFor(side));
    }
    clearTransientStatus(active);
    if (side === BattleTurn.ENEMY) {
      grantPlayerExperience(battle, active);
    }

    if (noReplacement) {
      return true;
    }

    if (side === BattleTurn.PLAYER) {
      return false;
    }

    return false;
  }

  type BattlePokemon = Pokemon & { _sram_slot?: number };

  export function finaliseBattle(battle: Battle): void {
    _resetPartyAfterBattle(battle.context.playerParty);

    const sram = battle.gameState.sram;
    if (sram.party) {
      for (let i = 0; i < sram.party.pokemon.length; i++) {
        const pokemon = sram.party.pokemon[i];
        if (pokemon) {
          const battleMon = battle.context.playerParty.find(
            (mon) => (mon as BattlePokemon)._sram_slot === i,
          ) as BattlePokemon | undefined;
          if (battleMon) {
            sram.party.pokemon[i] = battleMon;
          }
        }
      }
    }

    const poisonActive = battle.context.playerParty.some(
      (p) => p.status === StatusCondition.POISON
    );
    battle.gameState.wram.poison_step_count = poisonActive ? 0 : -1;

    const enemyDefeated = battle.context.isPartyDefeated(BattleTurn.ENEMY);
    const playerDefeated = battle.context.isPartyDefeated(BattleTurn.PLAYER);

    let outcome = 1;
    if (battle._caughtPokemon) {
      outcome = 0;
    } else if (battle._playerRan) {
      outcome = 2;
    } else if (playerDefeated) {
      outcome = 1;
    } else if (enemyDefeated) {
      outcome = 0;
    }

    if (outcome === 0) {
      const trainerReward = battle.context.trainerReward;
      if (
        trainerReward > 0 &&
        battle.context.trainerBattle &&
        !battle._playerRan &&
        enemyDefeated
      ) {
        // ASM: engine/battle/core.asm::WinTrainerBattle::give_money.
        let rewardUnit = toAsmBattleMoney(trainerReward, "Trainer reward");
        if (battle.context.amuletCoinActive) {
          rewardUnit = doubleAsmBattleMoney(rewardUnit);
        }

        // ASM supports three MOM modes; this save shape currently mirrors "some money" only.
        const sendSomeToMom = Boolean(sram.mom_saving_some_money) && sram.moms_money < MAX_MONEY;
        const momShares = sendSomeToMom ? 1 : 0;
        const walletShares = 4 - momShares;

        for (let i = 0; i < momShares; i++) {
          sram.moms_money = addToWallet(sram.moms_money, rewardUnit);
        }
        for (let i = 0; i < walletShares; i++) {
          sram.money = addToWallet(sram.money, rewardUnit);
        }

        const displayedTrainerReward = doubleAsmBattleMoney(doubleAsmBattleMoney(rewardUnit));
        battle.eventManager.dispatch(
          new Event('show_text', {
            text: `${sram.player_name} got ¥${displayedTrainerReward}\nfor winning!`
          })
        );
        if (momShares > 0) {
          battle.eventManager.dispatch(
            new Event('show_text', {
              text: 'Sent some to MOM!'
            })
          );
        }
      }

      let payout = toAsmBattleMoney(battle.context.payDayMoney, "Pay Day money");
      if (payout > 0) {
        // ASM: engine/battle/core.asm::CheckPayDay.
        if (battle.context.amuletCoinActive) {
          payout = doubleAsmBattleMoney(payout);
        }
        sram.money = addToWallet(sram.money, payout);
        battle.eventManager.dispatch(
          new Event('show_text', {
            text: `You picked up ¥${payout}!`
          })
        );
        battle.context.payDayMoney = 0;
      }
    }

    battle.gameState.wram.battle_result = outcome;
    const playerPartySummary = battle.context.playerParty
      .map((mon, idx) => `${idx}:${mon.nickname || mon.species.id}`)
      .join("; ");
    const enemyPartySummary = battle.context.enemyParty
      .map((mon, idx) => `${idx}:${mon.nickname || mon.species.id}`)
      .join("; ");
    pushDebugLog('[battle] final outcome', {
      outcome,
      player_party: playerPartySummary,
      enemy_party: enemyPartySummary,
    });

    handlePostBattleRoamers(
      battle.gameState,
      outcome,
      battle.context.enemyPokemon,
    );

    battle.prepareForOverworldResume();
    battle.eventManager.dispatch(
      new Event(
        'battle_complete', {
          result: outcome,
          trainer: battle.context.enemyTrainer,
        }
      )
    );
    battle.gameState.wram.battle_type = "BATTLETYPE_NORMAL";
  }
