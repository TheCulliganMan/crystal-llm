// ASM mapping: pokecrystal_disassembly/engine/overworld/player.asm (Joypad handling + CheckFacingObject).
import type { GameEngineEvent } from "@pokecrystal/core/ui/game-engine";
import type { AudioEngine } from "@pokecrystal/core/engine/systems/audio";
import type { GameState } from "@pokecrystal/core/core/state";
import type { PlayerState } from "@pokecrystal/core/core/enums/overworld";
import type { OverworldDialogue } from "@pokecrystal/core/engine/world/overworld/dialogue-types";
import type { TownMapOverlayLike } from "@pokecrystal/core/ui/overlays/town-map-overlay";
import type { BaseUI } from "@pokecrystal/core/ui/base-ui";
import { adjustCounterTile } from "./counter";
import { GameButton, isButtonEvent, isButtonKey, isKeyDownEvent, isKeyUpEvent } from "@pokecrystal/core/input/buttons";
import type { OverworldObject } from "./overworld-object";
import { playOverworldSound } from "./audio-guards";
import { pushDebugLog } from "@pokecrystal/core/core/debug-log";
import { isDebugEnabled } from "@pokecrystal/core/core/debug-flags";
import type { OverworldInputContext } from "./overworld-input.types";
import type { OverworldMap } from "@pokecrystal/core/engine/world/overworld/overworld-map";
import type { OverworldTilesetLike } from "@pokecrystal/core/engine/world/overworld/tileset-types";
import type { ScriptRunner } from "@pokecrystal/core/engine/world/story-events/runner";
import type { LoggerLike } from "@pokecrystal/core/engine/world/overworld/logger";
import type { Pokemon } from "@pokecrystal/core/core/models";

type OverworldScriptRunner = ScriptRunner & {
  allow_event_flag_refresh?: boolean;
  is_busy?: boolean;
};

export class OverworldInputMixin {
  _logger: LoggerLike = console;
  _held_directions: Map<string, null> = new Map();
  _queued_direction: string | null = null;
  _blocking_task_count: number = 0;
  _ignore_a_until_release: boolean = false;
  _ignore_select_until_release: boolean = false;
  input_capture_active: boolean = false;
  player_direction: string = "down";
  player_x: number = 0;
  player_y: number = 0;
  prev_player_x: number = 0;
  prev_player_y: number = 0;
  is_moving: boolean = false;
  map: OverworldMap | null = null;
  tileset: OverworldTilesetLike | null = null;
  dialogue: OverworldDialogue | null = null;
  script_runner: OverworldScriptRunner | null = null;
  game_state?: GameState | null;
  ui: BaseUI | null = null;
  audio_engine: AudioEngine | null = null;
  _town_map_overlay?: TownMapOverlayLike;
  protected _debug_inputs_enabled?: boolean;
  player_state: PlayerState | string | number | null = null;
  npcs?: OverworldObject[];
  TILES_PER_COLLISION: number = 2;

  handle_cut!: (x: number, y: number, pokemon?: Pokemon | null) => Promise<boolean>;
  handle_whirlpool!: (x: number, y: number) => Promise<boolean> | boolean | void;
  handle_waterfall!: (x: number, y: number) => Promise<boolean> | boolean | void;
  handle_surf!: (x: number, y: number) => Promise<boolean> | boolean | void;
  handle_a_button!: () => void;
  use_key_item!: (itemName: string) => boolean | Promise<boolean>;
  _show_field_move_text?: (label: string) => void;
  _show_field_move_text_async?: (label: string) => Promise<void>;
  _npc_occupying_subtile!: (x: number, y: number) => OverworldObject | null;

  get_facing_tile_coords!: () => [number, number];
  move_player!: (direction: string, forced?: boolean) => void;
  player_movement_locked!: () => boolean;
  public _describe_input_event?: (event: GameEngineEvent | null | undefined) => string;
  _direction_from_key!: (key: string | number | null | undefined) => string | null;

  public handle_a_button_press(): void {
    const context = this as unknown as OverworldInputContext;
    context.handle_a_button();
  }

  public handle_input(event: GameEngineEvent): void {
    if (this._town_map_overlay?.handle_input?.(event)) {
      return;
    }
    const traceInput =
      isDebugEnabled("overworld:input") || isDebugEnabled("input");
    const description = traceInput ? (this._describe_input_event?.(event) ?? "") : "";
    const dialogueActive = Boolean(this.dialogue?.active);
    const waitingForInput = Boolean(this.dialogue?.waiting_for_input);

    if (this.dialogue?.handle_input?.(event)) {
      if (traceInput) {
        const label = description || "event";
        pushDebugLog(`Dialogue consumed ${label}`);
      }
      return;
    }
    if (
      this.input_capture_active &&
      (isKeyDownEvent(event) || isKeyUpEvent(event))
    ) {
      return;
    }
    if (
      this.dialogue &&
      (dialogueActive || waitingForInput) &&
      (isKeyDownEvent(event) || isKeyUpEvent(event))
    ) {
      return;
    }
    this.prev_player_x = this.player_x;
    this.prev_player_y = this.player_y;
    if (
      this._blocking_task_count > 0 &&
      (isKeyDownEvent(event) || isKeyUpEvent(event))
    ) {
      return;
    }
    if (isKeyDownEvent(event)) {
      const direction = this._direction_from_key(event.code ?? event.key);
      if (direction !== null) {
        if (this.player_movement_locked()) {
          return;
        }
        pushDebugLog(`[input] direction ${direction}`);
        this._register_direction_press(direction);
      } else if (isButtonEvent(event, GameButton.A)) {
        if (this._ignore_a_until_release) {
          return;
        }
        if (this.player_movement_locked()) {
          return;
        }
        pushDebugLog("[input] A pressed");
        // Edge-trigger A interactions so key repeat cannot retrigger heavy handlers.
        this._ignore_a_until_release = true;
        this.handle_a_button_press();
      } else if (isButtonEvent(event, GameButton.Select)) {
        if (this._ignore_select_until_release) {
          return;
        }
        if (this.player_movement_locked()) {
          return;
        }
        this._ignore_select_until_release = true;
        const itemName = String(this.game_state?.wram?.wRegisteredItem ?? "").trim();
        if (!itemName) {
          return;
        }
        const used = this.use_key_item?.(itemName) ?? false;
        if (used instanceof Promise) {
          used.catch((error) => {
            this._logger?.error?.("Registered key item failed: %s", error instanceof Error ? error.message : String(error));
          });
          return;
        }
        if (!used) {
          const asyncText = this._show_field_move_text_async;
          if (typeof asyncText === "function") {
            asyncText.call(this, "CantUseItemText").catch((error) => {
              this._logger?.error?.(
                "Registered key item failure text failed: %s",
                error instanceof Error ? error.message : String(error),
              );
            });
          } else {
            this._show_field_move_text?.("CantUseItemText");
          }
        }
      }
    } else if (isKeyUpEvent(event)) {
      const keyCode = event.code ?? event.key;
      if (isButtonKey(keyCode, GameButton.A)) {
        this._ignore_a_until_release = false;
      }
      if (isButtonKey(keyCode, GameButton.Select)) {
        this._ignore_select_until_release = false;
      }
      const direction = this._direction_from_key(event.code ?? event.key);
      if (direction !== null) {
        this._register_direction_release(direction);
      }
    }
  }

  private _register_direction_press(direction: string): void {
    if (this.player_movement_locked()) {
      return;
    }
    if (this._held_directions.has(direction)) {
      this._held_directions.delete(direction);
    }
    this._held_directions.set(direction, null);
    this._queued_direction = direction;
    const debugInputs = Boolean(this._debug_inputs_enabled);
    if (debugInputs) {
      const state = this.player_state ?? null;
      console.error(`[OverworldInput] press ${direction} state=${state}`);
    }
  }

  private _register_direction_release(direction: string): void {
    if (this._held_directions.has(direction)) {
      this._held_directions.delete(direction);
    }
    if (this._queued_direction === direction) {
      this._queued_direction = this._current_held_direction();
    }
    const new_facing = this._current_held_direction();
    if (new_facing !== null) {
      this.player_direction = new_facing;
    }
    if (!this._held_directions.size) {
      this._queued_direction = null;
    }
  }

  private _current_held_direction(): string | null {
    if (!this._held_directions.size) {
      return null;
    }
    return this._held_directions.keys().next().value ?? null;
  }

  public _next_direction_to_continue(): string | null {
    if (this._queued_direction !== null) {
      const direction = this._queued_direction;
      this._queued_direction = null;
      return direction;
    }
    return this._current_held_direction();
  }

  public next_direction_to_continue(): string | null {
    return this._next_direction_to_continue();
  }

  protected _play_interaction_sound(): void {
    playOverworldSound(this.audio_engine, "SFX_READ_TEXT_2", {
      logger: this._logger,
      context: "interaction SFX",
    });
  }

  public check_for_npc_interaction(): boolean {
    const runner = this.script_runner;
    if (runner) {
      runner.last_interaction_object_index = null;
    }
    if (runner && runner.is_busy) {
      return false;
    }
    let [tile_x, tile_y] = this.get_facing_tile_coords();
    if (tile_x < 0 || tile_y < 0) {
      return false;
    }
    if (!this.map) {
      return false;
    }
    const findNpc = (x: number, y: number): OverworldObject | null =>
      this._npc_on_tile(x, y) ?? this._nearest_npc_covering_subtile(x, y);

    let best_npc = findNpc(tile_x, tile_y);
    if (!best_npc) {
      [tile_x, tile_y] = this._counter_adjusted_tile(tile_x, tile_y);
      best_npc = findNpc(tile_x, tile_y);
    }
    if (!best_npc) {
      if (this.game_state?.wram) {
        this.game_state.wram.last_talked = 0;
      }
      return false;
    }
    // ASM: CheckFacingObject rejects NPCs that are still walking.
    if (best_npc.walking || best_npc.jumping) {
      if (this.game_state?.wram) {
        this.game_state.wram.last_talked = 0;
      }
      return false;
    }
    if (this.game_state?.wram) {
      this.game_state.wram.last_talked = best_npc.objectIndex;
    }
    if (runner) {
      runner.last_interaction_object_index = best_npc.objectIndex ?? null;
    }
    if (typeof best_npc.facePlayer === "function") {
      best_npc.facePlayer(this.player_x, this.player_y);
    } else if (typeof (best_npc as any).face_player === "function") {
      (best_npc as any).face_player(this.player_x, this.player_y);
    }
    this._play_interaction_sound();
    const script = best_npc.event?.script ?? null;
    if (script) {
      this._remember_trainer_contact(best_npc);
      const objectType = String(best_npc.event?.object_type ?? "").toUpperCase();
      if (objectType === "OBJECTTYPE_TRAINER") {
        this.script_runner?.run?.(script, { allow_fallthrough: false });
      } else {
        this.script_runner?.run?.(script);
      }
    }
    return true;
  }

  private _npc_on_tile(tile_x: number, tile_y: number): OverworldObject | null {
    const npcs = this.npcs;
    if (!npcs || !npcs.length) {
      return null;
    }
    const ordered = [...npcs].sort((a, b) => a.objectIndex - b.objectIndex);
    for (const npc of ordered) {
      if (npc.x === tile_x && npc.y === tile_y) {
        return npc;
      }
      const alias = npc as { prev_x?: number; prev_y?: number };
      const prevX = alias.prev_x ?? npc.prevX ?? npc.x;
      const prevY = alias.prev_y ?? npc.prevY ?? npc.y;
      if (prevX === tile_x && prevY === tile_y) {
        return npc;
      }
    }
    return null;
  }

  private _nearest_npc_covering_subtile(
    facing_x: number,
    facing_y: number
  ): OverworldObject | null {
    let best_npc: OverworldObject | null = null;
    let best_distance = Infinity;
    for (let dx = 0; dx < this.TILES_PER_COLLISION; dx += 1) {
      for (let dy = 0; dy < this.TILES_PER_COLLISION; dy += 1) {
        const subtile_x = facing_x - dx;
        const subtile_y = facing_y - dy;
        const npc = this._npc_occupying_subtile(subtile_x, subtile_y);
        if (!npc) {
          continue;
        }
        const distance =
          Math.abs(npc.x - facing_x) + Math.abs(npc.y - facing_y);
        if (!best_npc || distance < best_distance) {
          best_npc = npc;
          best_distance = distance;
        } else if (distance === best_distance) {
          const current_index = best_npc.objectIndex;
          const next_index = npc.objectIndex;
          if (next_index < current_index) {
            best_npc = npc;
          }
        }
      }
    }
    return best_npc;
  }

  protected _counter_adjusted_tile(tile_x: number, tile_y: number): [number, number] {
    if (!this.map || !this.tileset) {
      return [tile_x, tile_y];
    }
    return adjustCounterTile(
      this.map,
      this.tileset,
      this.player_x,
      this.player_y,
      tile_x,
      tile_y,
      this.TILES_PER_COLLISION
    );
  }

  private _remember_trainer_contact(npc: OverworldObject): void {
    if (!npc || !this.game_state) {
      return;
    }
    const event = npc.event;
    if (!event) {
      return;
    }
    if (String(event.object_type ?? "").toUpperCase() !== "OBJECTTYPE_TRAINER") {
      return;
    }
    this.game_state.wram.seen_trainer_distance = 1;
    this.game_state.wram.seen_trainer_direction = "";
  }
}
