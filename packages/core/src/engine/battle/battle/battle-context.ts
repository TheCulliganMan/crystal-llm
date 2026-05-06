import { Item, Pokemon, Trainer } from '@pokecrystal/core/core/models';
import { BattleActionType, BattleTurn, MoveName, Stat, StatusCondition } from '@pokecrystal/core/core/enums';
import { johtoBadgeMaskAsm } from '@pokecrystal/core/core/badges';
import { restoreTransformState } from './transform-state';
import { resetBattleStatStages } from './stat-stages';

export { BattleActionType };

export enum BattleStateEnum {
  BATTLE_TRANSITION,
  BATTLE_START,
  PLAYER_ACTION_SELECT,
  ENEMY_ACTION_SELECT,
  PRE_TURN_EFFECTS,
  TURN_EXECUTION,
  POST_TURN_EFFECTS,
  BATTLE_END,
}

export type BattleAction = {
  actionType: BattleActionType;
  moveName?: MoveName;
  targetPokemon?: Pokemon;
  item?: Item;
  switchToPokemonIndex?: number;
  targetMoveIndex?: number;
};

export enum Weather {
  NORMAL,
  RAIN,
  SUN,
  SANDSTORM,
}

const ZEPHYR_BADGE_BIT = 0;
const PLAIN_BADGE_BIT = 2;
const MINERAL_BADGE_BIT = 4;
const GLACIER_BADGE_BIT = 6;

export class BattleContext {
  playerParty: Pokemon[];
  enemyParty: Pokemon[];
  playerActiveIndex: number;
  enemyActiveIndex: number;
  playerPokemon: Pokemon;
  enemyPokemon: Pokemon;
  enemyTrainer?: Trainer;
  enemyTrainerId?: string;
  trainerBattle: boolean;
  currentState: BattleStateEnum;
  trainerReward: number;
  playerAction?: BattleAction;
  enemyAction?: BattleAction;
  turnOrder: BattleTurn[];
  predefinedRandomValue?: number;
  predefined_random_value?: number;
  weather: Weather;
  weatherTurns: number;
  playerSpikesLayers: number;
  enemySpikesLayers: number;
  playerReflectTurns: number;
  enemyReflectTurns: number;
  playerLightScreenTurns: number;
  enemyLightScreenTurns: number;
  playerSafeguardTurns: number;
  enemySafeguardTurns: number;
  playerMistTurns: number;
  enemyMistTurns: number;
  playerRunAttempts: number;
  playerForcedStruggle: boolean;
  enemyForcedStruggle: boolean;
  runAttemptSuccess: boolean;
  playerFutureSightCounter: number;
  playerFutureSightDamage: number;
  enemyFutureSightCounter: number;
  enemyFutureSightDamage: number;
  payDayMoney: number;
  amuletCoinActive: boolean;
  playerQuickClawActivated: boolean;
  enemyQuickClawActivated: boolean;
  playerParticipantsNotFainted: Set<number>;
  playerParticipantsIncludingFainted: Set<number>;
  playerCantRun: boolean;
  badgeBoostsEnabled: boolean;
  playerJohtoBadgeMask: number;

  constructor(
    playerParty: Pokemon[],
    enemyParty: Pokemon[],
    playerPokemon: Pokemon,
    enemyPokemon: Pokemon,
    enemyTrainer: Trainer | undefined,
    trainerBattle: boolean,
    enemyTrainerId: string | undefined,
    trainerReward: number,
  ) {
    this.playerParty = playerParty;
    this.enemyParty = enemyParty;
    this.playerActiveIndex = 0;
    this.enemyActiveIndex = 0;
    this.playerPokemon = playerPokemon;
    this.enemyPokemon = enemyPokemon;
    this.enemyTrainer = enemyTrainer;
    this.enemyTrainerId = enemyTrainerId;
    this.trainerBattle = trainerBattle;
    this.currentState = BattleStateEnum.BATTLE_START;
    this.trainerReward = trainerReward;
    this.turnOrder = [];
    this.weather = Weather.NORMAL;
    this.weatherTurns = 0;
    this.playerSpikesLayers = 0;
    this.enemySpikesLayers = 0;
    this.playerReflectTurns = 0;
    this.enemyReflectTurns = 0;
    this.playerLightScreenTurns = 0;
    this.enemyLightScreenTurns = 0;
    this.playerSafeguardTurns = 0;
    this.enemySafeguardTurns = 0;
    this.playerMistTurns = 0;
    this.enemyMistTurns = 0;
    this.playerRunAttempts = 0;
    this.playerForcedStruggle = false;
    this.enemyForcedStruggle = false;
    this.runAttemptSuccess = false;
    this.playerFutureSightCounter = 0;
    this.playerFutureSightDamage = 0;
    this.enemyFutureSightCounter = 0;
    this.enemyFutureSightDamage = 0;
    this.payDayMoney = 0;
    this.amuletCoinActive = false;
    this.playerQuickClawActivated = false;
    this.enemyQuickClawActivated = false;
    this.playerParticipantsNotFainted = new Set();
    this.playerParticipantsIncludingFainted = new Set();
    this.playerCantRun = false;
    this.badgeBoostsEnabled = false;
    this.playerJohtoBadgeMask = 0;
  }

  initializeBattleParticipants(): void {
    this.playerParticipantsNotFainted.clear();
    this.playerParticipantsIncludingFainted.clear();

    if (!this.playerParty.length) return;

    if (this.playerActiveIndex < 0 || this.playerActiveIndex >= this.playerParty.length) return;

    const pokemon = this.playerParty[this.playerActiveIndex];
    if (pokemon && pokemon.hp > 0) {
      this.addPlayerParticipant(this.playerActiveIndex);
    }
  }

  resetPlayerStatLevels(): void {
    for (const pokemon of this.playerParty) {
      if (pokemon) {
        resetBattleStatStages(pokemon);
        pokemon.turns_in_battle = 0;
      }
    }
  }

  get playerReflect(): boolean {
    return this.playerReflectTurns > 0;
  }

  get enemyReflect(): boolean {
    return this.enemyReflectTurns > 0;
  }

  get playerLightScreen(): boolean {
    return this.playerLightScreenTurns > 0;
  }

  get enemyLightScreen(): boolean {
    return this.enemyLightScreenTurns > 0;
  }

  get playerSafeguard(): boolean {
    return this.playerSafeguardTurns > 0;
  }

  get enemySafeguard(): boolean {
    return this.enemySafeguardTurns > 0;
  }

  private barrierAttr(side: BattleTurn, barrier: string): keyof this {
    const prefix = side === BattleTurn.PLAYER ? 'player' : 'enemy';
    switch (barrier) {
      case 'reflect':
        return `${prefix}ReflectTurns` as keyof this;
      case 'light_screen':
        return `${prefix}LightScreenTurns` as keyof this;
      case 'safeguard':
        return `${prefix}SafeguardTurns` as keyof this;
      case 'mist':
        return `${prefix}MistTurns` as keyof this;
      default:
        throw new Error(`Unknown barrier '${barrier}'.`);
    }
  }

  barrierTurns(side: BattleTurn, barrier: string): number {
    const attr = this.barrierAttr(side, barrier);
    const value = this[attr];
    return typeof value === 'number' ? value : 0;
  }

  setBarrier(side: BattleTurn, barrier: string, turns: number): void {
    const attr = this.barrierAttr(side, barrier);
    (this[attr] as number) = Math.max(0, Math.trunc(turns));
  }

  clearBarrier(side: BattleTurn, barrier: string): void {
    this.setBarrier(side, barrier, 0);
  }

  tickBarrier(side: BattleTurn, barrier: string): number {
    const attr = this.barrierAttr(side, barrier);
    const remaining = Math.max(0, (this[attr] as number) - 1);
    (this[attr] as number) = remaining;
    return remaining;
  }


  sideFor(pokemon: Pokemon): BattleTurn | undefined {
    if (pokemon === this.playerPokemon) {
      return BattleTurn.PLAYER;
    }
    if (pokemon === this.enemyPokemon) {
      return BattleTurn.ENEMY;
    }
    if (this.playerParty.includes(pokemon)) {
      return BattleTurn.PLAYER;
    }
    if (this.enemyParty.includes(pokemon)) {
      return BattleTurn.ENEMY;
    }
    return undefined;
  }

  addPlayerParticipant(index: number): void {
    if (index < 0 || index >= this.playerParty.length) {
      throw new Error(`Party index ${index} out of range for player.`);
    }

    this.playerParticipantsIncludingFainted.add(index);
    const pokemon = this.playerParty[index];
    if (pokemon && pokemon.hp > 0) {
      this.playerParticipantsNotFainted.add(index);
    }
  }

  markPlayerFainted(index: number): void {
    this.playerParticipantsNotFainted.delete(index);
  }

  partyFor(side: BattleTurn): Pokemon[] {
    return side === BattleTurn.PLAYER ? this.playerParty : this.enemyParty;
  }

  activeIndexFor(side: BattleTurn): number {
    return side === BattleTurn.PLAYER ? this.playerActiveIndex : this.enemyActiveIndex;
  }

  setActiveIndex(side: BattleTurn, index: number): void {
    const party = this.partyFor(side);
    if (index < 0 || index >= party.length) {
      throw new Error(`Party index ${index} out of range for ${side}.`);
    }
    if (side === BattleTurn.PLAYER) {
      this.playerActiveIndex = index;
      this.playerPokemon = party[index];
    } else {
      this.enemyActiveIndex = index;
      this.enemyPokemon = party[index];
    }
  }

  // ASM: pokecrystal_disassembly/engine/battle/core.asm::SwitchPlayerMon
  switchActive(side: BattleTurn, newIndex: number): Pokemon {
    const outgoing = side === BattleTurn.PLAYER ? this.playerPokemon : this.enemyPokemon;
    restoreTransformState(outgoing);
    resetBattleStatStages(outgoing);
    outgoing.trapped_turns = 0;
    outgoing.trapped_by_side = undefined;
    outgoing.trapped_source_index = undefined;
    outgoing.trapped_move = undefined;
    outgoing.leech_seeded = false;
    outgoing.leech_seed_source_side = undefined;
    outgoing.nightmare = false;
    outgoing.cursed = false;
    outgoing.curse_source_side = undefined;

    this.setActiveIndex(side, newIndex);
    const pokemon = this.partyFor(side)[newIndex];
    resetBattleStatStages(pokemon);
    pokemon.turns_in_battle = 0;
    pokemon.locked_move = undefined;
    pokemon.locked_turns_remaining = 0;
    pokemon.rampage_turns = 0;
    pokemon.flinching = false;
    if (pokemon.status === StatusCondition.CONFUSION) {
      pokemon.status = undefined;
    }
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
    if (side === BattleTurn.PLAYER) {
      this.playerForcedStruggle = false;
      this.addPlayerParticipant(newIndex);
    } else {
      this.enemyForcedStruggle = false;
    }
    return pokemon;
  }

  availablePartyIndices(side: BattleTurn, includeActive = false): number[] {
    const indices: number[] = [];
    const activeIndex = this.activeIndexFor(side);
    for (const [idx, pokemon] of this.partyFor(side).entries()) {
      if (pokemon.hp <= 0) {
        continue;
      }
      if (!includeActive && idx === activeIndex) {
        continue;
      }
      indices.push(idx);
    }
    return indices;
  }

  isPartyDefeated(side: BattleTurn): boolean {
    return this.availablePartyIndices(side, true).length === 0;
  }

  private spikesAttr(side: BattleTurn): keyof this {
    return side === BattleTurn.PLAYER ? "playerSpikesLayers" : "enemySpikesLayers";
  }

  spikesLayers(side: BattleTurn): number {
    const value = this[this.spikesAttr(side)];
    return typeof value === "number" ? value : 0;
  }

  setSpikesLayers(side: BattleTurn, layers: number): void {
    if (layers < 0) {
      throw new Error("Spikes layers cannot be negative.");
    }
    (this[this.spikesAttr(side)] as number) = layers;
  }

  futureSightCounter(side: BattleTurn): number {
    return side === BattleTurn.PLAYER ? this.playerFutureSightCounter : this.enemyFutureSightCounter;
  }

  setFutureSightCounter(side: BattleTurn, value: number): void {
    if (side === BattleTurn.PLAYER) {
      this.playerFutureSightCounter = value;
    } else {
      this.enemyFutureSightCounter = value;
    }
  }

  futureSightDamage(side: BattleTurn): number {
    return side === BattleTurn.PLAYER ? this.playerFutureSightDamage : this.enemyFutureSightDamage;
  }

  setFutureSightDamage(side: BattleTurn, value: number): void {
    if (side === BattleTurn.PLAYER) {
      this.playerFutureSightDamage = value;
    } else {
      this.enemyFutureSightDamage = value;
    }
  }

  // ASM mapping: pokecrystal_disassembly/engine/battle/core.asm::BadgeStatBoosts
  // Badge boosts are player-only and disabled in link / Battle Tower battles.
  setBadgeBoostState(
    johtoBadges: readonly boolean[],
    options: { linkMode?: boolean; inBattleTowerBattle?: boolean } = {}
  ): void {
    this.playerJohtoBadgeMask = johtoBadgeMaskAsm(johtoBadges, "BattleContext badge boost setup");
    this.badgeBoostsEnabled = !options.linkMode && !options.inBattleTowerBattle;
  }

  private hasPlayerJohtoBadge(bit: number): boolean {
    return Boolean(this.playerJohtoBadgeMask & (1 << bit));
  }

  badgeBoostActive(side: BattleTurn, stat: Stat): boolean {
    if (side !== BattleTurn.PLAYER || !this.badgeBoostsEnabled) {
      return false;
    }
    switch (stat) {
      case Stat.ATTACK:
        return this.hasPlayerJohtoBadge(ZEPHYR_BADGE_BIT);
      case Stat.DEFENSE:
        // ASM swaps Plain/Mineral when mapping badges to the DEF/SPEED boosts.
        return this.hasPlayerJohtoBadge(MINERAL_BADGE_BIT);
      case Stat.SPEED:
        return this.hasPlayerJohtoBadge(PLAIN_BADGE_BIT);
      case Stat.SPECIAL_ATTACK:
      case Stat.SPECIAL_DEFENSE:
        return this.hasPlayerJohtoBadge(GLACIER_BADGE_BIT);
      default:
        return false;
    }
  }
}
