import { GameState } from "@pokecrystal/core/core/state";
import { GB_FRAME_DURATION_MS } from "@pokecrystal/core/core/gb-timing";
import { EventManager } from "@pokecrystal/core/engine/events/events";
import { Command, OverworldContext } from "./base";
import { LOGGER } from "../common";
import { PlayerState } from "@pokecrystal/core/core/enums/overworld";

// ASM: engine/overworld/scripting.asm::Script_playmusic, Script_musicfadeout,
// Script_dontrestartmapmusic, Script_reloadmapafterbattle, Script_sjump, Script_sdefer.
// ASM: engine/events/field_moves.asm::OWCutJumptable (HM triggers).

const normalizeScriptName = (name: string): string => {
  const cleaned = name.split(";", 1)[0].trim();
  if (!cleaned) {
    throw new Error("Script reference resolved to an empty name.");
  }
  return cleaned;
};

type AudioEngineLike = {
  play_music?: (musicId: string, role?: string) => void;
  playMusic?: (musicId: string, role?: string) => void;
  requestMusic?: (musicId: string, role?: string) => void;
  fadeToMusic?: (musicId: string, speedFrames: number, role?: string) => void;
  fadeOut?: (musicId: string, speedFrames: number) => void;
  fade_out?: (musicId: string, speedFrames: number) => void;
  fadeOutMusic?: (durationMs: number) => void;
};

type FieldMoveOverworld = OverworldContext & {
  player_state?: PlayerState;
  game_state?: GameState;
  audio_engine?: AudioEngineLike | null;
  audioEngine?: AudioEngineLike | null;
  requestMusic?: (musicId: string, role?: string) => void;
  fadeToMusic?: (musicId: string, speedFrames: number, role?: string) => void;
};

const getAudioEngine = (overworld: FieldMoveOverworld): AudioEngineLike | null => {
  return overworld.audio_engine ?? overworld.audioEngine ?? null;
};

const resolveFieldMoveState = (overworld: OverworldContext): PlayerState => {
  return (overworld as FieldMoveOverworld).player_state ?? PlayerState.NORMAL;
};

type MethodOwner = Record<string, unknown> | null | undefined;

const requireCallable = <T extends (...args: unknown[]) => unknown>(
  owner: OverworldContext | MethodOwner,
  methodName: string
): T => {
  const candidate: MethodOwner = owner as MethodOwner;
  const fn = candidate?.[methodName];
  if (typeof fn !== "function") {
    throw new Error(`Overworld implementation missing ${methodName}().`);
  }
  return fn.bind(candidate) as T;
};

export class CutCommand extends Command {
  constructor(private readonly x: number, private readonly y: number) {
    super();
  }

  execute(_gameState: GameState, _eventManager: EventManager, overworld: OverworldContext): void {
    requireCallable(overworld, "handle_cut")(this.x, this.y);
  }
}

export class SurfCommand extends Command {
  constructor(private readonly x: number, private readonly y: number) {
    super();
  }

  execute(_gameState: GameState, _eventManager: EventManager, overworld: OverworldContext): void {
    requireCallable(overworld, "handle_surf")(this.x, this.y);
  }
}

export class StrengthCommand extends Command {
  constructor(private readonly x: number, private readonly y: number) {
    super();
  }

  execute(_gameState: GameState, _eventManager: EventManager, overworld: OverworldContext): void {
    const handle = requireCallable(overworld, "_handle_hm");
    handle("Strength", this.x, this.y, resolveFieldMoveState(overworld));
  }
}

export class WhirlpoolCommand extends Command {
  constructor(private readonly x: number, private readonly y: number) {
    super();
  }

  execute(_gameState: GameState, _eventManager: EventManager, overworld: OverworldContext): void {
    const handle = requireCallable(overworld, "_handle_hm");
    handle("Whirlpool", this.x, this.y, resolveFieldMoveState(overworld));
  }
}

export class WaterfallCommand extends Command {
  constructor(private readonly x: number, private readonly y: number) {
    super();
  }

  execute(_gameState: GameState, _eventManager: EventManager, overworld: OverworldContext): void {
    const handle = requireCallable(overworld, "_handle_hm");
    handle("Waterfall", this.x, this.y, resolveFieldMoveState(overworld));
  }
}

export class FlashCommand extends Command {
  execute(_gameState: GameState, _eventManager: EventManager, overworld: OverworldContext): void {
    requireCallable(overworld, "handle_flash")();
  }
}

export class FlyCommand extends Command {
  constructor(private readonly x: number, private readonly y: number) {
    super();
  }

  execute(_gameState: GameState, _eventManager: EventManager, overworld: OverworldContext): void {
    requireCallable(overworld, "handle_fly")(this.x, this.y);
  }
}

export class JumpCommand extends Command {
  private readonly scriptName: string;

  constructor(scriptName: string) {
    super();
    this.scriptName = normalizeScriptName(scriptName);
  }

  execute(_gameState: GameState, _eventManager: EventManager, _overworld: OverworldContext): void {
    const runner = this.runner;
    if (!runner) {
      throw new Error("JumpCommand requires an active ScriptRunner.");
    }
    if (LOGGER.debug) {
      LOGGER.debug("JumpCommand jumping to %s", this.scriptName);
    }
    let parentScript: string | null = null;
    if (this.scriptName.startsWith(".") && typeof runner._find_parent_script_name === "function") {
      parentScript = runner._find_parent_script_name();
    }
    const jump = runner.jump;
    if (typeof jump !== "function") {
      throw new Error("ScriptRunner implementation missing jump().");
    }
    if (parentScript !== null && parentScript !== undefined) {
      runner.jump(this.scriptName, parentScript);
    } else {
      runner.jump(this.scriptName);
    }
  }
}

export class SdeferCommand extends Command {
  private readonly scriptName: string;

  constructor(scriptName: string) {
    super();
    this.scriptName = normalizeScriptName(scriptName);
  }

  execute(_gameState: GameState, _eventManager: EventManager, _overworld: OverworldContext): void {
    const runner = this.runner;
    if (!runner) {
      throw new Error("SdeferCommand requires an active script runner.");
    }
    if (LOGGER.debug) {
      LOGGER.debug("SdeferCommand deferring %s", this.scriptName);
    }
    const defer = runner.defer;
    if (typeof defer !== "function") {
      throw new Error("ScriptRunner implementation missing defer().");
    }
    runner.defer(this.scriptName);
  }
}

export class ScriptCallCommand extends Command {
  private readonly scriptName: string;

  constructor(scriptName: string) {
    super();
    this.scriptName = normalizeScriptName(scriptName.replace(/,$/, ""));
  }

    execute(_gameState: GameState, _eventManager: EventManager, _overworld: OverworldContext): void {
        const runner = this.runner;
        if (!runner) {
            throw new Error("ScriptCallCommand requires an active script runner.");
        }
        if (LOGGER.debug) {
            LOGGER.debug("ScriptCallCommand invoking %s", this.scriptName);
        }
        const callScript = runner.call ?? runner.run;
        if (typeof callScript !== "function") {
            throw new Error("ScriptRunner implementation missing call().");
        }
        callScript.call(runner, this.scriptName);
    }
}

export class PlayMusicCommand extends Command {
  constructor(private readonly musicId: string) {
    super();
  }

  execute(_gameState: GameState, _eventManager: EventManager, overworld: OverworldContext): void {
    const runner = this.runner;
    if (runner) {
      runner.last_sound_effect = null;
    }
    const host = overworld as FieldMoveOverworld;
    if (typeof host.requestMusic === "function") {
      host.requestMusic(this.musicId, "general");
      return;
    }
    const audioEngine = getAudioEngine(host);
    if (typeof audioEngine?.requestMusic === "function") {
      audioEngine.requestMusic(this.musicId, "general");
      return;
    }
    if (typeof audioEngine?.playMusic === "function") {
      audioEngine.playMusic(this.musicId, "general");
      return;
    }
    audioEngine?.play_music?.(this.musicId, "general");
  }
}

export class MusicFadeOutCommand extends Command {
  constructor(private readonly musicId: string, private readonly speedFrames: number) {
    super();
  }

  execute(_gameState: GameState, _eventManager: EventManager, overworld: OverworldContext): void {
    const runner = this.runner;
    if (runner) {
      runner.last_sound_effect = null;
    }
    const host = overworld as FieldMoveOverworld;
    if (typeof host.fadeToMusic === "function") {
      host.fadeToMusic(this.musicId, this.speedFrames, "general");
      return;
    }
    const audioEngine = getAudioEngine(host);
    if (!audioEngine) {
      return;
    }
    if (typeof audioEngine.fadeToMusic === "function") {
      audioEngine.fadeToMusic(this.musicId, this.speedFrames, "general");
      return;
    }

    if (typeof audioEngine.fadeOut === "function") {
      audioEngine.fadeOut(this.musicId, this.speedFrames);
      return;
    }

    if (typeof audioEngine.fade_out === "function") {
      audioEngine.fade_out(this.musicId, this.speedFrames);
      return;
    }

    if (typeof audioEngine.fadeOutMusic === "function") {
      const durationMs = MusicFadeOutCommand.framesToMilliseconds(this.speedFrames);
      audioEngine.fadeOutMusic(durationMs);
      if (typeof audioEngine.playMusic === "function") {
        audioEngine.playMusic(this.musicId, "general");
      } else {
        audioEngine.play_music?.(this.musicId, "general");
      }
      return;
    }

    if (typeof audioEngine.playMusic === "function") {
      audioEngine.playMusic(this.musicId, "general");
      return;
    }
    audioEngine.play_music?.(this.musicId, "general");
  }

  private static framesToMilliseconds(frames: number): number {
    if (frames <= 0) {
      return 0;
    }
    return Math.round(frames * GB_FRAME_DURATION_MS);
  }
}

export class DontRestartMapMusicCommand extends Command {
  execute(gameState: GameState, _eventManager: EventManager, _overworld: OverworldContext): void {
    gameState.wram.dont_restart_map_music = true;
  }
}

export class ReloadMapAfterBattleCommand extends Command {
  execute(gameState: GameState, _eventManager: EventManager, _overworld: OverworldContext): void {
    gameState.wram.reload_map_after_battle = true;
  }
}
