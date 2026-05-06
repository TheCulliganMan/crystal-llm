import { GameState } from '@pokecrystal/core/core/state';
import { EventManager } from './events';
import type { ScriptRunner as StoryScriptRunner } from '@pokecrystal/core/engine/world/story-events/runner';

export interface HealSummary {
  healed_slots: number[];
}

export interface PokemonCenterSystem {
  heal_party: () => HealSummary;
  playHealMachineAnimation?: (animationId: string | null, overworld: PokemonCenterOverworld | null) => void;
  playHealMachineAnimationAsync?: (
    animationId: string | null,
    overworld: PokemonCenterOverworld | null
  ) => Promise<void>;
}

export type PokemonCenterOverworld = {
  pokemon_center?: PokemonCenterSystem;
};

export type PokemonCenterRunner = StoryScriptRunner & {
  pokemon_center?: PokemonCenterSystem;
};

export const resolvePokemonCenterSystem = (
  runner?: PokemonCenterRunner,
  overworld?: PokemonCenterOverworld
): PokemonCenterSystem | undefined => {
  let system: PokemonCenterSystem | undefined = undefined;
  if (runner?.pokemon_center) {
    system = runner.pokemon_center;
  }
  if (!system && overworld?.pokemon_center) {
    system = overworld.pokemon_center;
  }
  return system;
};

export const healParty = (
  gameState: GameState,
  {
    runner,
    overworld,
    eventManager,
  }: {
    runner?: PokemonCenterRunner;
    overworld?: PokemonCenterOverworld;
    eventManager?: EventManager;
  }
): void => {
  void eventManager;
  const system = resolvePokemonCenterSystem(runner, overworld);
  if (!system) {
    return;
  }
  const summary = system.heal_party();
  if (runner) {
    runner.last_condition_result = !!summary.healed_slots?.length;
    runner.last_value = summary;
  }
};
