// ASM mapping:
// - pokecrystal_disassembly/engine/overworld/events.asm: CountStep
// - pokecrystal_disassembly/engine/events/happiness_egg.asm: StepHappiness
// - pokecrystal_disassembly/engine/pokemon/breeding.asm: DoEggStep
import type { PokemonData } from "@pokecrystal/core/core/models/pokemon";
import type { GameState } from "@pokecrystal/core/core/state";
import { hatchEgg } from "@pokecrystal/core/engine/systems/breeding";
import type { DayCareSystem } from "@pokecrystal/core/engine/systems/day-care";
import type { PoisonDamageResult } from "@pokecrystal/core/engine/world/poison";
import { applyPoisonToParty, isPoisoned } from "@pokecrystal/core/engine/world/poison";

const POISON_STEP_INTERVAL = 4;

export type StepEventResult = {
  egg_hatched: boolean;
  hatched_species: string | null;
  poison_result: PoisonDamageResult | null;
};

export class StepEventSystem {
  private readonly game_state: GameState;
  private readonly day_care: DayCareSystem | null;

  constructor(game_state: GameState, { day_care = null }: { day_care?: DayCareSystem | null } = {}) {
    this.game_state = game_state;
    this.day_care = day_care;
  }

  process_step(): StepEventResult {
    const wram = this.game_state.wram;
    wram.poison_step_count = StepEventSystem.increment_byte(wram.poison_step_count);
    wram.step_count = StepEventSystem.increment_byte(wram.step_count);

    if (wram.step_count === 0) {
      this.apply_happiness_step();
    }

    if (wram.step_count === 0x80) {
      const hatched_species = this.process_egg_step();
      if (hatched_species !== null) {
        return {
          egg_hatched: true,
          hatched_species,
          poison_result: null,
        };
      }
    }

    if (this.day_care) {
      this.day_care.advance_steps?.(1);
    }

    const poison_result = this.process_poison_step();

    return {
      egg_hatched: false,
      hatched_species: null,
      poison_result,
    };
  }

  private apply_happiness_step(): void {
    const wram = this.game_state.wram;
    wram.happiness_step_count = ((wram.happiness_step_count ?? 0) + 1) & 1;
    if (wram.happiness_step_count !== 0) {
      return;
    }
    const party = this.game_state.sram?.party?.pokemon ?? [];
    if (!party.some(Boolean)) {
      return;
    }
    for (const pokemon of party) {
      if (!pokemon || StepEventSystem.is_egg(pokemon)) {
        continue;
      }
      pokemon.happiness = Math.min((pokemon.happiness ?? 0) + 1, 0xff);
    }
  }

  private process_egg_step(): string | null {
    const party = this.game_state.sram?.party?.pokemon ?? [];
    for (const pokemon of party) {
      if (!pokemon || !StepEventSystem.is_egg(pokemon)) {
        continue;
      }
      pokemon.happiness = ((pokemon.happiness ?? 0) - 1) & 0xff;
      if ((pokemon.happiness ?? 0) !== 0) {
        continue;
      }
      hatchEgg(this.game_state, pokemon);
      return String(pokemon.species?.id ?? "").toUpperCase() || null;
    }
    return null;
  }

  private process_poison_step(): PoisonDamageResult | null {
    const wram = this.game_state.wram;
    let counter = (wram.poison_step_count ?? 0) & 0xff;
    if (counter < 0) {
      counter = 0;
    }
    if (counter < POISON_STEP_INTERVAL) {
      wram.poison_step_count = counter;
      return null;
    }
    wram.poison_step_count = 0;
    const party = this.game_state.sram?.party?.pokemon ?? [];
    if (!party.some(Boolean)) {
      return null;
    }
    const poisoned_before_step = party.filter(
      (pokemon): pokemon is PokemonData =>
        pokemon !== null && isPoisoned(pokemon.status) && (pokemon.hp ?? 0) > 0
    );
    const result = applyPoisonToParty(party);
    if (poisoned_before_step.length) {
      this.apply_poison_faint_happiness(poisoned_before_step);
    }
    if (!result.damagedNames.length && !result.faintedNames.length) {
      return null;
    }
    return result;
  }

  private apply_poison_faint_happiness(poisoned_before_step: PokemonData[]): void {
    for (const pokemon of poisoned_before_step) {
      if ((pokemon.hp ?? 0) > 0) {
        continue;
      }
      const delta = StepEventSystem.poison_faint_happiness_delta(pokemon.happiness ?? 0);
      pokemon.happiness = Math.max(0, Math.min(0xff, (pokemon.happiness ?? 0) + delta));
    }
  }

  private static poison_faint_happiness_delta(happiness: number): number {
    if (happiness < 100) {
      return -5;
    }
    if (happiness < 200) {
      return -5;
    }
    return -10;
  }

  private static is_egg(pokemon: PokemonData): boolean {
    return String(pokemon.nickname ?? "").toUpperCase() === "EGG";
  }

  private static increment_byte(value: number | undefined): number {
    return (((value ?? 0) & 0xff) + 1) & 0xff;
  }
}
