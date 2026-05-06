import { GameState } from "@pokecrystal/core/core/state";
import { AudioEngine } from "@pokecrystal/core/engine/systems/audio";
import { defaultMusicTokenForMap } from "@pokecrystal/core/engine/world/map-music";
import { ScriptRunner, ensureRunnerVariables } from "./utils";
import type { SpecialContext } from "./special-types";

type CryDataLoader = {
  get_pokemon_cry?: (speciesId: string) => { cry_id?: string; cry?: string } | null;
  getPokemonCry?: (speciesId: string) => { cry_id?: string; cry?: string } | null;
};

type Overworld = {
  audio_engine?: AudioEngine | null;
  audioEngine?: AudioEngine | null;
  start_map_music?: () => void;
  startMapMusic?: () => void;
  restart_map_music?: () => void;
  restartMapMusic?: () => void;
  requestMapMusic?: (mapName: string) => void;
  requestMusic?: (token: string, role?: string) => void;
  playCry?: (cryId: string) => void;
  fadeToMusic?: (token: string, durationFrames: number, role?: string) => void;
  current_map_name?: string;
  currentMapName?: string;
  data_loader?: CryDataLoader | null;
  dataLoader?: CryDataLoader | null;
};

type LegacyFadeOutEngine = {
  fadeOut?: (name: string, durationMs: number) => void;
  fadeOutMusic?: (durationMs: number) => void;
};

type PokemonCenterOwner = {
  pokemon_center?: unknown;
};

const normalizeSpeciesIdentifier = (value: unknown): string => {
  if (value === null || value === undefined) {
    return "";
  }
  if (typeof value === "object") {
    const candidate = (value as { id?: unknown }).id;
    if (candidate !== undefined && candidate !== null) {
      return String(candidate).toUpperCase();
    }
  }
  return String(value).toUpperCase();
};

export function play_map_music(
  game_state: GameState,
  {
    overworld,
    audio_engine,
  }: { overworld?: Overworld | null; audio_engine?: AudioEngine | null } = {}
): boolean {
  // ASM: engine/overworld/events.asm::PlayMapMusic
  const engine =
    audio_engine ?? overworld?.audio_engine ?? overworld?.audioEngine ?? null;
  if (!engine) {
    throw new Error("PlayMapMusic requires an available audio engine.");
  }

  if (game_state.wram.dont_restart_map_music) {
    if (typeof overworld?.restartMapMusic === "function") {
      overworld.restartMapMusic();
    } else {
      game_state.wram.dont_restart_map_music = false;
    }
    return true;
  }

  const mapName = overworld?.current_map_name ?? overworld?.currentMapName;
  if (!mapName) {
    throw new Error("PlayMapMusic cannot resolve the current map name.");
  }

  const startMusic = overworld?.start_map_music ?? overworld?.startMapMusic;
  if (startMusic) {
    startMusic.call(overworld);
    return true;
  }
  if (typeof overworld?.requestMapMusic === "function") {
    overworld.requestMapMusic(mapName);
    return true;
  }

  const token = defaultMusicTokenForMap(mapName);
  const current = String(game_state.wram.wMapMusic ?? "").trim();
  if (current === token) {
    return true;
  }
  engine.playMusic(token, "map");
  game_state.wram.wMapMusic = token;
  return true;
}

export function play_cur_mon_cry(
  game_state: GameState,
  {
    runner,
    overworld,
    audio_engine,
  }: { runner?: ScriptRunner & PokemonCenterOwner; overworld?: Overworld | null; audio_engine?: AudioEngine | null } = {}
): string {
  // ASM: engine/events/cry.asm::PlayCurMonCry
  const engine =
    audio_engine ?? overworld?.audio_engine ?? overworld?.audioEngine ?? null;
  if (!engine) {
    throw new Error("PlayCurMonCry requires an available audio engine.");
  }

  let speciesId = String(game_state.wram.wCurPartySpecies ?? "").trim().toUpperCase();
  const party = game_state.sram.party?.pokemon ?? [];
  let partyCount = game_state.wram.wPartyCount ?? party.length;
  if (!partyCount) {
    partyCount = party.length;
  }
  const visibleParty = party.slice(0, partyCount);

  if (!speciesId) {
    const index = Number(game_state.wram.wCurPartyMon ?? 0);
    const indexed = index >= 0 && index < visibleParty.length ? visibleParty[index] : null;
    const mon = indexed ?? visibleParty.find((member) => member != null) ?? null;
    if (!mon?.species?.id) {
      throw new Error("No party Pokemon available for cry playback.");
    }
    speciesId = String(mon.species.id).toUpperCase();
    game_state.wram.wCurPartySpecies = speciesId;
  }

  const dataLoader = (runner?.data_loader ??
    runner?.dataLoader ??
    overworld?.data_loader ??
    overworld?.dataLoader) as CryDataLoader | null | undefined;
  const cryLookup = dataLoader?.get_pokemon_cry ?? dataLoader?.getPokemonCry;
  let cryId = `CRY_${speciesId}`;
  if (cryLookup) {
    const cry = cryLookup.call(dataLoader, speciesId);
    if (cry?.cry_id) {
      cryId = cry.cry_id;
    } else if (cry?.cry) {
      cryId = cry.cry;
    }
  }

  overworld?.playCry?.(cryId);
  if (typeof overworld?.playCry !== "function") {
    engine.playSound(cryId);
  }
  if (runner) {
    runner.last_sound_effect = cryId;
    runner.last_condition_result = true;
  }
  return cryId;
}

export function play_slow_cry(
  game_state: GameState,
  {
    runner,
    overworld,
    audio_engine,
  }: { runner?: ScriptRunner; overworld?: Overworld | null; audio_engine?: AudioEngine | null } = {}
): string {
  // ASM: engine/events/cry.asm::PlaySlowCry
  const engine =
    audio_engine ?? overworld?.audio_engine ?? overworld?.audioEngine ?? null;
  if (!engine) {
    throw new Error("PlaySlowCry requires an available audio engine.");
  }

  const variables = ensureRunnerVariables(runner);
  const speciesToken = variables._value ?? game_state.wram.wCurPartySpecies;
  if (!speciesToken) {
    throw new Error("PlaySlowCry requires a species identifier.");
  }

  const dataLoader = (runner?.data_loader ??
    runner?.dataLoader ??
    overworld?.data_loader ??
    overworld?.dataLoader) as CryDataLoader | null | undefined;
  const cryLookup = dataLoader?.get_pokemon_cry ?? dataLoader?.getPokemonCry;

  const speciesId = normalizeSpeciesIdentifier(speciesToken);
  let cryId = `CRY_${speciesId}`;
  if (cryLookup) {
    const cry = cryLookup.call(dataLoader, speciesId);
    if (cry?.cry_id) {
      cryId = cry.cry_id;
    } else if (cry?.cry) {
      cryId = cry.cry;
    }
  }

  overworld?.playCry?.(cryId);
  if (typeof overworld?.playCry !== "function") {
    engine.playSound(cryId);
  }
  if (runner) {
    runner.last_sound_effect = cryId;
    runner.last_condition_result = true;
  }
  return cryId;
}

export function gameboy_check(game_state: GameState, context: SpecialContext): string {
  // ASM: home/hardware.asm::GameboyCheck
  const { runner } = context;
  const token = "GBCHECK_CGB";
  if (runner) {
    runner.last_value = token;
    const variables = ensureRunnerVariables(runner);
    variables._value = token;
    runner.last_condition_result = token === "GBCHECK_CGB";
  }
  return token;
}

export function fade_out_music(game_state: GameState, context: SpecialContext): boolean {
  // ASM: engine/events/specials.asm::FadeOutMusic
  const { runner, audio_engine } = context;
  const overworld = context.overworld as Overworld | null;
  const engine =
    audio_engine ?? overworld?.audio_engine ?? overworld?.audioEngine ?? null;
  if (!engine) {
    return false;
  }
  if (typeof overworld?.fadeToMusic === "function") {
    overworld.fadeToMusic("MUSIC_NONE", 2, "general");
  } else {
    const fadeEngine = engine as LegacyFadeOutEngine;
    if (typeof fadeEngine.fadeOut === "function") {
      fadeEngine.fadeOut("MUSIC_NONE", 2);
    } else if (typeof fadeEngine.fadeOutMusic === "function") {
      fadeEngine.fadeOutMusic(Math.floor((2 * 1000) / 60));
    } else {
      throw new Error("Audio engine missing fadeToMusic(), fadeOut(), or fadeOutMusic().");
    }
  }
  if (runner) {
    runner.last_condition_result = true;
  }
  return true;
}

export function restart_map_music(
  game_state: GameState,
  {
    runner,
    overworld,
    audio_engine,
  }: { runner?: ScriptRunner; overworld?: Overworld | null; audio_engine?: AudioEngine | null } = {}
): boolean {
  // ASM: engine/events/specials.asm::RestartMapMusic
  const engine =
    audio_engine ?? overworld?.audio_engine ?? overworld?.audioEngine ?? null;
  const restartMusic = overworld?.restart_map_music ?? overworld?.restartMapMusic;
  if (restartMusic) {
    restartMusic.call(overworld);
    if (runner) {
      runner.last_condition_result = true;
    }
    return true;
  }
  if (engine?.restartMapMusic) {
    engine.restartMapMusic();
    if (runner) {
      runner.last_condition_result = true;
    }
    return true;
  }
  const mapName = overworld?.current_map_name ?? overworld?.currentMapName ?? null;
  if (engine && mapName) {
    const token = defaultMusicTokenForMap(mapName);
    if (typeof overworld?.requestMapMusic === "function") {
      overworld.requestMapMusic(mapName);
    } else {
      engine.playMusic(token, "map");
    }
    if (runner) {
      runner.last_condition_result = true;
    }
    return true;
  }
  if (runner) {
    runner.last_condition_result = false;
  }
  return false;
}
