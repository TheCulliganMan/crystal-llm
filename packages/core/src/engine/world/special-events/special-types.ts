import type { EventManager } from "@pokecrystal/core/engine/events/events";
import type { AudioEngine } from "@pokecrystal/core/engine/systems/audio";
import type { OverworldContext } from "@pokecrystal/core/engine/world/story-events/commands/base";
import type { ScriptRunner } from "@pokecrystal/core/engine/world/story-events/runner";

export type RNGSource = { randrange: (upper: number) => number; nextByte: () => number };

export type PokemonCenterService = {
  healParty?: () => void;
  playHealMachineAnimation?: (
    animationId: string | null,
    overworld: SpecialOverworld | null
  ) => void;
  playHealMachineAnimationAsync?: (
    animationId: string | null,
    overworld: SpecialOverworld | null
  ) => Promise<void>;
};

export type PokemonCenterOwner = {
  pokemon_center?: PokemonCenterService | null;
};

export type SpecialOverworld = OverworldContext | Record<string, unknown>;

export type SpecialContext = Partial<{
  runner?: (ScriptRunner & PokemonCenterOwner) | null;
  overworld?: SpecialOverworld | null;
  event_manager?: EventManager | null;
  audio_engine?: AudioEngine | null;
  rng?: RNGSource;
}> & Record<string, unknown>;

export type SpecialFunction = (...args: any[]) => unknown;

export type BuenaPasswordCategory = {
  label: string;
  category_type: string;
  points: number;
  options: readonly string[];
};

export type BuenaPrize = {
  item: string;
  cost: number;
};
