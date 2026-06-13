
import { GameState } from '../../core/state';
import { Event, EventManager } from './events';
import { fade_in_from_white, fade_out_to_white, heal_party, warp_to_spawn_point } from './special-events';
import { findSpawnForMap, getSpawnPoint, Spawn } from './maps';
import { STANDARD_SCRIPT_HANDLERS } from './story-events/specials/handlers';
import { close_text, open_text, show_text, wait_for_input } from './events';
import type { ScriptRunner } from '@pokecrystal/core/engine/world/story-events/runner';
import type { OverworldEngine } from '@pokecrystal/core/engine/world/overworld/overworld';
import type { Overworld as OverworldType } from '@pokecrystal/core/types/overworld';
import type { PokemonCenterOverworld, PokemonCenterRunner } from '@pokecrystal/core/engine/events/misc';
import { pushDebugLog } from '@pokecrystal/core/core/debug-log';

interface _WhiteoutSequence {
    context: string;
    phase: string;
    timer: number;
    text_displayed: boolean;
    text_closed: boolean;
    fade_started: boolean;
    movement_locked: boolean;
}

type DialogueContext = {
    waiting_for_input?: boolean;
    pending_waits?: number;
    visible?: boolean;
    active?: boolean;
    clear_script_waits?: () => void;
};

type WhiteoutOverworld = {
    dialogue?: DialogueContext | null;
    script_runner?: ScriptRunner | null;
    lock_player_movement?: () => void;
    unlock_player_movement?: () => void;
};

type PokemonCenterOverworldEngine = OverworldEngine & PokemonCenterOverworld;

export class WhiteoutManager {
    private static readonly _FADE_WAIT_FRAMES = 40;
    private static readonly _CANLOSE_BATTLE_TYPE = "BATTLETYPE_CANLOSE";

    private game_state: GameState;
    private overworld: WhiteoutOverworld;
    private event_manager: EventManager;
    private _sequence: _WhiteoutSequence | null = null;
    private _pending_trigger = false;
    private _last_all_fainted_signature: string | null = null;

    constructor(game_state: GameState, overworld: WhiteoutOverworld, event_manager: EventManager) {
        this.game_state = game_state;
        this.overworld = overworld;
        this.event_manager = event_manager;
        this.event_manager.on('battle_complete', this._handle_battle_complete.bind(this));
    }

    private _handle_battle_complete(event: Event, state: GameState): void {
        const battleResult = Number(event.data?.result ?? state.wram.battle_result);
        const battleType = String(state.wram.battle_type ?? "").replace(/,+$/, "").trim().toUpperCase();
        if (battleType === WhiteoutManager._CANLOSE_BATTLE_TYPE) {
            // ASM: engine/battle/core.asm::LostBattle skips whiteout for BATTLETYPE_CANLOSE.
            return;
        }
        const playerLost = battleResult === 1;
        const hasUsable = this._playerHasUsableParty();
        pushDebugLog('[whiteout] battle_complete', {
            battle_result: battleResult,
            battle_type: battleType,
            pending_trigger: this._pending_trigger,
            active_sequence: Boolean(this._sequence),
            player_lost: playerLost,
            usable_party: hasUsable,
        });
        if (this._pending_trigger || this._sequence) {
            return;
        }

        if (playerLost && !hasUsable) {
            this._pending_trigger = true;
            this._log_pending_party_status();
        }
    }

    public update(): void {
        if (!this._sequence) {
            this._queue_whiteout_if_party_fainted();
            if (this._pending_trigger) {
                this._begin_sequence();
            }
            return;
        }

        switch (this._sequence.phase) {
            case 'text':
                this._progress_text_phase();
                break;
            case 'fade':
                this._progress_fade_phase();
                break;
            case 'heal':
                this._progress_heal_phase();
                break;
            case 'warp':
                this._progress_warp_phase();
                break;
            case 'complete':
                this._finalise_sequence();
                break;
        }
    }

    private _begin_sequence(): void {
        this._clear_stale_dialogue_wait_state();
        this._clear_stale_input_capture_state();
        this._sequence = {
            context: 'battle',
            phase: 'text',
            timer: 0,
            text_displayed: false,
            text_closed: false,
            fade_started: false,
            movement_locked: false,
        };
        this._pending_trigger = false;
        this.game_state.wram.reload_map_after_battle = false;
        this._lock_movement();
    }

    private _progress_text_phase(): void {
        if (!this._sequence!.text_displayed) {
            this._display_text();
            this._sequence!.text_displayed = true;
            return;
        }
        if (this._dialogue_ready()) {
            this._sequence!.phase = 'fade';
            if (!this._sequence!.fade_started) {
                this._start_fade();
            }
        }
    }

    private _progress_fade_phase(): void {
        if (!this._sequence!.fade_started) {
            this._start_fade();
        } else if (this._sequence!.timer > 0) {
            this._sequence!.timer--;
        } else {
            this._sequence!.phase = 'heal';
        }
    }

    private _progress_heal_phase(): void {
        this._heal_party();
        if (this._handle_bug_contest_warp()) {
            this._sequence!.phase = 'complete';
        } else {
            this._sequence!.phase = 'warp';
        }
    }

    private _progress_warp_phase(): void {
        this._halve_money();
        this._resolve_whiteout_spawn();
        this._abort_bug_contest();
        const warped = warp_to_spawn_point(this.game_state, { overworld: this.overworld as unknown as OverworldType });
        if (!warped) {
            throw new Error('Whiteout failed to resolve a spawn warp.');
        }
        this._sequence!.phase = 'complete';
    }

    private _finalise_sequence(): void {
        fade_in_from_white(this.game_state, {
            overworld: this.overworld as unknown as OverworldEngine,
            runner: this.overworld.script_runner ?? undefined,
        });
        this._unlock_movement();
        this._close_text_box(true);
        this._sequence = null;
    }

    private _display_text(): void {
        this._clear_stale_dialogue_wait_state();
        const player_name = this.game_state.sram.player_name.trim() || 'PLAYER';
        const message = `${player_name} is out of\nuseable POKéMON!\n\n${player_name} whited\nout!`;
        open_text(this.event_manager);
        show_text(this.event_manager, message);
        wait_for_input(this.event_manager);
    }

    private _dialogue_ready(): boolean {
        const dialogue = this.overworld.dialogue;
        if (!dialogue) {
            return true;
        }
        const dialogueState = dialogue as { visible?: boolean; active?: boolean };
        const dialogueVisible = Boolean(dialogueState.visible ?? dialogueState.active);
        const pending = dialogue.pending_waits ?? 0;
        const waiting = dialogue.waiting_for_input ?? false;
        if (!dialogueVisible) {
            this._clear_stale_dialogue_wait_state();
            return true;
        }
        return !waiting && pending === 0;
    }

    private _clear_stale_dialogue_wait_state(): void {
        const dialogue = this.overworld.dialogue;
        if (!dialogue) {
            return;
        }
        const dialogueState = dialogue as { visible?: boolean; active?: boolean };
        if (dialogueState.visible || dialogueState.active) {
            return;
        }
        const waiting = Boolean(dialogue.waiting_for_input);
        const pending = Number(dialogue.pending_waits ?? 0);
        if (waiting || pending > 0) {
            dialogue.clear_script_waits?.();
            if (!dialogue.clear_script_waits) {
                dialogue.waiting_for_input = false;
                dialogue.pending_waits = 0;
            }
        }
    }

    private _clear_stale_input_capture_state(): void {
        const overworld = this.overworld as WhiteoutOverworld & {
            input_capture_active?: boolean;
        };
        if (overworld.input_capture_active) {
            overworld.input_capture_active = false;
        }
    }

    private _start_fade(): void {
        fade_out_to_white(this.game_state, {
            overworld: this.overworld as unknown as OverworldEngine,
            runner: this.overworld.script_runner ?? undefined,
        });
        this._close_text_box();
        this._sequence!.fade_started = true;
        this._sequence!.timer = WhiteoutManager._FADE_WAIT_FRAMES;
    }

    private _heal_party(): void {
        heal_party({
            game_state: this.game_state,
            runner: this.overworld.script_runner
                ? (this.overworld.script_runner as unknown as PokemonCenterRunner)
                : undefined,
            overworld: this.overworld as unknown as PokemonCenterOverworldEngine | undefined,
            event_manager: this.event_manager as unknown as EventManager | undefined,
        });
    }

    private _halve_money(): void {
        const current = Math.max(0, Number(this.game_state.sram.money) || 0);
        this.game_state.sram.money = Math.floor(current / 2);
    }

    private _resolve_whiteout_spawn(): void {
        const wramGroup = this.game_state.wram.wLastSpawnMapGroup;
        const wramMapId = this.game_state.wram.wLastSpawnMapNumber;
        let resolved = findSpawnForMap(wramGroup, wramMapId);
        if (!resolved) {
            const sramGroup = this.game_state.sram.last_spawn_map_group;
            const sramMapId = this.game_state.sram.last_spawn_map_number;
            resolved = findSpawnForMap(sramGroup, sramMapId);
        }
        const spawn = resolved ? resolved[0] : Spawn.HOME;
        const spawnPoint = getSpawnPoint(spawn);
        this.game_state.wram.wDefaultSpawnpoint = spawn;
        this.game_state.wram.wLastSpawnMapGroup = spawnPoint.groupId;
        this.game_state.wram.wLastSpawnMapNumber = spawnPoint.mapId;
    }

    private _handle_bug_contest_warp(): boolean {
        const flags = this.game_state.wram.engine_flags;
        if (!flags?.ENGINE_BUG_CONTEST_TIMER) {
            return false;
        }
        const handler = STANDARD_SCRIPT_HANDLERS.BugContestResultsWarpScript;
        const runner = this.overworld.script_runner;
        if (!handler || !runner) {
            throw new Error('Bug Contest whiteout requested but handler context is unavailable.');
        }
        handler(runner);
        return true;
    }

    private _abort_bug_contest(): void {
        const flags = this.game_state.wram.engine_flags;
        if (flags?.ENGINE_BUG_CONTEST_TIMER) {
            flags.ENGINE_BUG_CONTEST_TIMER = false;
            flags.ENGINE_DAILY_BUG_CONTEST = true;
        }
    }

    private _lock_movement(): void {
        if (this._sequence && !this._sequence.movement_locked) {
            this.overworld.lock_player_movement?.();
            this._sequence.movement_locked = true;
        }
    }

    private _unlock_movement(): void {
        if (this._sequence?.movement_locked) {
            this.overworld.unlock_player_movement?.();
            this._sequence.movement_locked = false;
        }
    }

    private _close_text_box(force = false): void {
        if (!this._sequence || (this._sequence.text_closed && !force)) {
            return;
        }
        close_text(this.event_manager);
        this._sequence.text_closed = true;
    }

    private _log_pending_party_status(): void {
        const party = this.game_state.sram.party?.pokemon ?? [];
        const summary = party.map((mon, index) => {
            if (!mon) {
                return `${index}: <empty>`;
            }
            const nickname = (mon.nickname ?? '').trim();
            const species = String(mon.species?.id ?? '').replace(/_/g, ' ');
            const label = nickname || species || 'POKéMON';
            const hp = Number(mon.hp ?? 0);
            const max_hp = Number(mon.max_hp ?? 0);
            const status = mon.status ? String(mon.status) : hp <= 0 ? 'FAINTED' : 'OK';
            return `${index}:${label}(${hp}/${max_hp})[${status}]`;
        });
        pushDebugLog('[whiteout] triggered with party state', { party: summary.join('; ') });
    }

    private _queue_whiteout_if_party_fainted(): void {
        if (this._pending_trigger || this._sequence) {
            return;
        }
        const battleType = String(this.game_state.wram.battle_type ?? "").replace(/,+$/, "").trim().toUpperCase();
        if (battleType === WhiteoutManager._CANLOSE_BATTLE_TYPE) {
            return;
        }
        const party = this.game_state.sram.party?.pokemon ?? [];
        const hasPartyMember = party.some((mon) => Boolean(mon));
        const hasUsableParty = this._playerHasUsableParty();
        if (!hasPartyMember || hasUsableParty) {
            this._last_all_fainted_signature = null;
            return;
        }
        const faintedSignature = party
            .map((mon, index) => mon ? `${index}:${mon.species?.id ?? 'MON'}:${Number(mon.hp ?? 0)}/${Number(mon.max_hp ?? 0)}` : `${index}:<empty>`)
            .join('|');
        if (this._last_all_fainted_signature === faintedSignature) {
            return;
        }
        this._last_all_fainted_signature = faintedSignature;
        this._pending_trigger = true;
        this._log_pending_party_status();
    }

    private _playerHasUsableParty(): boolean {
        const party = this.game_state.sram.party?.pokemon ?? [];
        return party.some((mon) => Boolean(mon && (mon.hp ?? 0) > 0));
    }
}
