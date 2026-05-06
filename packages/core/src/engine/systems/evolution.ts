// ASM mapping: pokecrystal_disassembly/engine/pokemon/evolve.asm (EvolveAfterBattle, LearnLevelMoves).
import { Pokemon, type LearnedMove } from '../../core/models';
import { DataLoader, loadAllSpecies } from '../../core/data-loader';
import { loadMergedEvolutionsSync, loadMergedMovesDataSync } from '../../core/content-packs';
import { canonicaliseTimeOfDay } from './time';
import { Stat } from '@pokecrystal/core/core/enums/pokemon';
import { levelUpMovesForSpecies } from './learnsets';

export type EvolutionEvent =
  | { type: 'text'; id?: string | number }
  | { type: 'move'; id?: string | number }
  | { type: 'item'; id?: string | number }
  | { type: string; [key: string]: unknown };

export type EvolutionData = {
  species: string;
};

type EvolutionMethod = 'LEVEL' | 'ITEM' | 'HAPPINESS' | 'TRADE' | 'STAT';

type EvolutionEntry = {
  method: EvolutionMethod;
  level?: number | null;
  item?: string | null;
  held_item?: string | null;
  happiness?: string | null;
  stat_ratio?: string | null;
  species: string;
};

type EvolutionTableEntry = {
  species: string;
  evolutions: EvolutionEntry[];
};

type MoveData = Record<string, { pp?: number }>;

type LinkMode = 'NONE' | 'LINK' | 'TIMECAPSULE';

const HAPPINESS_TO_EVOLVE = 220;
const TRADE_ANY_ITEM = '-1';

const memoize = <T>(fn: () => T): () => T => {
  let initialized = false;
  let result: T;
  return () => {
    if (!initialized) {
      result = fn();
      initialized = true;
    }
    return result;
  };
};

const loadEvolutionTable = memoize((): EvolutionTableEntry[] => {
  return loadMergedEvolutionsSync() as EvolutionTableEntry[];
});

const evolutionMap = memoize((): Map<string, EvolutionEntry[]> => {
  const map = new Map<string, EvolutionEntry[]>();
  for (const entry of loadEvolutionTable()) {
    const key = normalizeSpecies(entry.species);
    if (!key) {
      continue;
    }
    map.set(key, entry.evolutions ?? []);
  }
  return map;
});

const loadMoveData = memoize((): MoveData => {
  return loadMergedMovesDataSync() as MoveData;
});

const normalizeSpecies = (value: string | null | undefined): string =>
  String(value ?? '').trim().toUpperCase();

const normalizeItem = (value: string | null | undefined): string | null => {
  if (!value) {
    return null;
  }
  const token = String(value).trim().toUpperCase().replace(/[^0-9A-Z]+/g, '_');
  const normalized = token.replace(/^_+|_+$/g, '');
  return normalized.length ? normalized : null;
};

const normalizeLinkMode = (value?: unknown): LinkMode => {
  if (!value) {
    return 'NONE';
  }
  if (value === true) {
    return 'LINK';
  }
  const token = String(value).trim().toUpperCase();
  if (token === 'TIMECAPSULE' || token === 'TIME_CAPSULE') {
    return 'TIMECAPSULE';
  }
  if (token === 'LINK' || token === 'TRADE') {
    return 'LINK';
  }
  return 'NONE';
};

const isHoldingEverstone = (pokemon: Pokemon): boolean =>
  normalizeItem(pokemon.item ?? null) === 'EVERSTONE';

export class Evolution {
  public events: EvolutionEvent[] = [];
  private _candidate: EvolutionEntry | null = null;

  constructor(
    private readonly pokemon: Pokemon,
    private readonly options?: {
      data_loader?: DataLoader;
      time_of_day?: string;
      current_item?: string | null;
      force_evolution?: boolean;
      link_mode?: LinkMode | string | boolean | null;
    }
  ) {}

  check_for_evolution(): EvolutionData | null {
    const candidate = this._find_candidate();
    this._candidate = candidate;
    return candidate ? { species: candidate.species } : null;
  }

  evolve(_include_intro: boolean = true): Pokemon | null {
    const candidate = this._candidate ?? this._find_candidate();
    this._candidate = null;
    this.events = [];
    if (!candidate) {
      return null;
    }
    if (_include_intro) {
      this.events.push({ type: 'text', id: 'EvolvingText' });
    }

    const oldSpeciesId = normalizeSpecies(this.pokemon.species?.id);
    const nextSpecies = this._resolve_species(candidate.species);
    if (!nextSpecies) {
      throw new Error(`Evolution target species ${candidate.species} was not found.`);
    }

    if (this._should_update_nickname(oldSpeciesId)) {
      this.pokemon.nickname = nextSpecies.id.toUpperCase();
    }

    const oldMaxHp = this.pokemon.max_hp;
    const oldHp = this.pokemon.hp;
    this.pokemon.species = nextSpecies;

    const newMaxHp = this.pokemon._calculateStat(Stat.HP);
    this.pokemon.max_hp = newMaxHp;
    this.pokemon.attack = this.pokemon._calculateStat(Stat.ATTACK);
    this.pokemon.defense = this.pokemon._calculateStat(Stat.DEFENSE);
    this.pokemon.speed = this.pokemon._calculateStat(Stat.SPEED);
    this.pokemon.special_attack = this.pokemon._calculateStat(Stat.SPECIAL_ATTACK);
    this.pokemon.special_defense = this.pokemon._calculateStat(Stat.SPECIAL_DEFENSE);

    const hpDelta = newMaxHp - oldMaxHp;
    const updatedHp = oldHp + hpDelta;
    this.pokemon.hp = Math.min(newMaxHp, Math.max(0, updatedHp));

    this.events.push({ type: 'text', id: 'EvolvedIntoText' });

    if (candidate.method === 'TRADE') {
      const required = normalizeItem(candidate.held_item ?? null);
      if (required && required !== TRADE_ANY_ITEM) {
        this.pokemon.item = null;
        this.events.push({ type: 'item' });
      }
    }

    const learnedMoves = this._learn_evolution_moves();
    for (const learned of learnedMoves) {
      this.events.push({ type: 'move', id: learned.name });
    }

    return this.pokemon;
  }

  private _find_candidate(): EvolutionEntry | null {
    const speciesId = normalizeSpecies(this.pokemon.species?.id);
    if (!speciesId) {
      throw new Error('Evolution requires a pokemon species id.');
    }
    const evolutions = evolutionMap().get(speciesId) ?? [];
    if (!evolutions.length) {
      return null;
    }

    const currentItem = normalizeItem(this.options?.current_item ?? null);
    const forceEvolution = Boolean(this.options?.force_evolution);
    const linkMode = normalizeLinkMode(this.options?.link_mode);
    const timeOfDay = canonicaliseTimeOfDay(this.options?.time_of_day ?? 'DAY');

    for (const evo of evolutions) {
      switch (evo.method) {
        case 'ITEM': {
          if (!forceEvolution || linkMode !== 'NONE') {
            break;
          }
          const required = normalizeItem(evo.item ?? null);
          if (!required) {
            throw new Error(`Item evolution missing required item for ${speciesId}.`);
          }
          if (required === currentItem) {
            return evo;
          }
          break;
        }
        case 'LEVEL': {
          if (forceEvolution) {
            break;
          }
          if ((evo.level ?? 0) > this.pokemon.level) {
            break;
          }
          if (isHoldingEverstone(this.pokemon)) {
            break;
          }
          return evo;
        }
        case 'HAPPINESS': {
          if (forceEvolution) {
            break;
          }
          if (this.pokemon.happiness < HAPPINESS_TO_EVOLVE) {
            break;
          }
          if (isHoldingEverstone(this.pokemon)) {
            break;
          }
          const window = String(evo.happiness ?? '').toUpperCase();
          if (window === 'TR_ANYTIME') {
            return evo;
          }
          if (window === 'TR_MORNDAY') {
            if (timeOfDay !== 'NIGHT') {
              return evo;
            }
            break;
          }
          if (window === 'TR_NITE') {
            if (timeOfDay === 'NIGHT') {
              return evo;
            }
            break;
          }
          throw new Error(`Unknown happiness window ${evo.happiness ?? ''} for ${speciesId}.`);
        }
        case 'STAT': {
          if (forceEvolution) {
            break;
          }
          if ((evo.level ?? 0) > this.pokemon.level) {
            break;
          }
          if (isHoldingEverstone(this.pokemon)) {
            break;
          }
          const ratio = String(evo.stat_ratio ?? '').toUpperCase();
          const attack = this.pokemon.attack ?? 0;
          const defense = this.pokemon.defense ?? 0;
          if (ratio === 'ATK_GT_DEF' && attack > defense) {
            return evo;
          }
          if (ratio === 'ATK_LT_DEF' && attack < defense) {
            return evo;
          }
          if (ratio === 'ATK_EQ_DEF' && attack === defense) {
            return evo;
          }
          break;
        }
        case 'TRADE': {
          if (linkMode === 'NONE') {
            break;
          }
          if (isHoldingEverstone(this.pokemon)) {
            break;
          }
          const required = normalizeItem(evo.held_item ?? null);
          if (!required || required === TRADE_ANY_ITEM) {
            return evo;
          }
          if (linkMode === 'TIMECAPSULE') {
            break;
          }
          const held = normalizeItem(this.pokemon.item ?? null);
          if (held === required) {
            return evo;
          }
          break;
        }
        default:
          throw new Error(`Unknown evolution method ${(evo as { method?: string }).method ?? ''} for ${speciesId}.`);
      }
    }
    return null;
  }

  private _resolve_species(target: string) {
    const normalized = normalizeSpecies(target);
    if (!normalized) {
      return null;
    }
    const loader = this.options?.data_loader;
    if (loader?.get_pokemon_species) {
      const result = loader.get_pokemon_species(normalized);
      if (result) {
        return result;
      }
    }
    return loadAllSpecies().get(normalized) ?? null;
  }

  private _should_update_nickname(oldSpeciesId: string): boolean {
    const nickname = String(this.pokemon.nickname ?? '').trim().toUpperCase();
    return nickname.length > 0 && nickname === oldSpeciesId;
  }

  private _learn_evolution_moves(): LearnedMove[] {
    const learned: LearnedMove[] = [];
    const level = this.pokemon.level;
    if (level <= 0) {
      return learned;
    }
    const moveData = loadMoveData();
    const moves = levelUpMovesForSpecies(this.pokemon.species.id);
    const current = (this.pokemon.moves ?? []).filter(Boolean) as LearnedMove[];
    const known = new Set(current.map((move) => move.name));
    for (const [learnLevel, moveName] of moves) {
      if (learnLevel !== level) {
        continue;
      }
      if (known.has(moveName)) {
        continue;
      }
      if (current.length >= 4) {
        continue;
      }
      const pp = moveData[moveName]?.pp ?? 0;
      const entry: LearnedMove = { name: moveName, current_pp: pp };
      current.push(entry);
      learned.push(entry);
      known.add(moveName);
    }
    this.pokemon.moves = current;
    return learned;
  }
}

export { HAPPINESS_TO_EVOLVE };
