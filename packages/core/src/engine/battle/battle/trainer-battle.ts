import { Pokemon, Trainer, Move as MoveData, toPokemon } from "@pokecrystal/core/core/models";
import { GameState } from "@pokecrystal/core/core/state";
import { MoveName } from "@pokecrystal/core/core/enums";
import { Event, EventManager } from "@pokecrystal/core/engine/events/events";
import type { AudioEngine } from "@pokecrystal/core/engine/systems/audio";
import type { DudeAutoInputController } from "@pokecrystal/core/engine/battle/auto-input";
import type { Overworld } from "@pokecrystal/core/engine/world/overworld/overworld";
import type { BattleUIState } from "@pokecrystal/core/ui/overlays/battle-ui-state";
import { should_block_state_advance } from "@pokecrystal/core/ui/overlays/battle-ui-core";
import { BattleStateEnum } from "./battle-context";
import { Battle } from "./battle-logic";

enum TrainerStartPhase {
  ANNOUNCE,
  WAIT_ANNOUNCE,
  EXITING,
  ENEMY_TEXT,
  ENEMY_ANIM,
  PLAYER_TEXT,
  PLAYER_EXITING,
  PLAYER_ANIM,
  COMPLETE,
}

export class TrainerBattle extends Battle {
  public trainer: Trainer;
  private trainerStartPhase = TrainerStartPhase.ANNOUNCE;

  constructor(
    playerPokemon: Pokemon,
    trainer: Trainer,
    gameState: GameState,
    eventManager: EventManager,
    battleUiState: BattleUIState,
    movesMap: Map<MoveName, MoveData>,
    audioEngine?: AudioEngine | null,
    playerParty?: Pokemon[],
    enemyParty?: Pokemon[],
    overworld?: Overworld | null,
    trainerId?: string,
    trainerReward = 0,
    autoInput?: DudeAutoInputController | null
  ) {
    super(
      playerPokemon,
      toPokemon(trainer.party[0]),
      gameState,
      eventManager,
      battleUiState,
      movesMap,
      audioEngine,
      trainer,
      playerParty,
      enemyParty,
      overworld,
      trainerId,
      trainerReward,
      autoInput
    );
    this.trainer = trainer;
  }

  protected handleBattleStart(): void {
    if (this.gameState.wram.instant_mode) {
      this._checkPlayerAmuletCoin();
      this.trainerStartPhase = TrainerStartPhase.COMPLETE;
      this.context.currentState = BattleStateEnum.PLAYER_ACTION_SELECT;
      this._clearBattleHasJustStartedFlag();
      this.clearTrainerBattleIntroAnimations();
      return;
    }
    if (this._trainerIntroActive()) {
      return;
    }
    if (!this.battleUi || this.battleUi.is_mock) {
      this._checkPlayerAmuletCoin();
      this.context.currentState = BattleStateEnum.PLAYER_ACTION_SELECT;
      this._clearBattleHasJustStartedFlag();
      return;
    }
    switch (this.trainerStartPhase) {
      case TrainerStartPhase.ANNOUNCE: {
        // ASM: engine/battle/core.asm::BattleStartMessage (trainer announcement).
        this.eventManager.dispatch(
          new Event("show_text", { text: `${this.trainer.name} wants to battle!` })
        );
        this.trainerStartPhase = TrainerStartPhase.WAIT_ANNOUNCE;
        return;
      }
      case TrainerStartPhase.WAIT_ANNOUNCE: {
        if (this.uiBlocked()) {
          return;
        }
        // ASM intent: after the trainer challenge text, the enemy trainer sprite exits before send-out text.
        this.eventManager.dispatch(new Event("trigger_trainer_exit", { side: "enemy" }));
        this.trainerStartPhase = TrainerStartPhase.EXITING;
        return;
      }
      case TrainerStartPhase.EXITING: {
        if (this.trainerExitActive()) {
          return;
        }
        // ASM: engine/battle/core.asm::ShowBattleTextEnemySentOut.
        this.eventManager.dispatch(
          new Event("show_text", {
            text: `${this.trainer.name} sent out ${this.context.enemyPokemon.nickname}!`,
          })
        );
        this.trainerStartPhase = TrainerStartPhase.ENEMY_TEXT;
        return;
      }
      case TrainerStartPhase.ENEMY_TEXT: {
        if (this.uiBlocked()) {
          return;
        }
        // ASM: engine/battle/core.asm::ShowSetEnemyMonAndSendOutAnimation.
        this.eventManager.dispatch(
          new Event("play_animation", {
            move_name: "SEND_OUT_MON",
            is_player_move: false,
            param: 0,
          })
        );
        this.trainerStartPhase = TrainerStartPhase.ENEMY_ANIM;
        return;
      }
      case TrainerStartPhase.ENEMY_ANIM: {
        if (this.uiBlocked()) {
          return;
        }
        // ASM: engine/gfx/pic_animation.asm::AnimateFrontpic (trainer battle uses ANIM_MON_SLOW).
        this.eventManager.dispatch(
          new Event("frontpic_animation", { side: "enemy", speed: 4 })
        );
        // ASM: engine/battle/core.asm::ShowSetEnemyMonAndSendOutAnimation (enemy cry).
        this._playBattleStartCry();
        // ASM: engine/battle/core.asm::BattleMonEntrance shows the player backpic before SendOutMonText.
        this.eventManager.dispatch(new Event("show_trainer_sprites", { mode: "player" }));
        // ASM: engine/battle/core.asm::SendOutMonText.
        this.eventManager.dispatch(
          new Event("show_text", { text: `Go! ${this.context.playerPokemon.nickname}!` })
        );
        this.trainerStartPhase = TrainerStartPhase.PLAYER_TEXT;
        return;
      }
      case TrainerStartPhase.PLAYER_TEXT: {
        if (this.uiBlocked()) {
          return;
        }
        // ASM mapping: engine/battle/core.asm::SlideBattlePicOut (a=9 for player-side box) before player send-out.
        this.eventManager.dispatch(new Event("trigger_trainer_exit", { side: "player" }));
        this.trainerStartPhase = TrainerStartPhase.PLAYER_EXITING;
        return;
      }
      case TrainerStartPhase.PLAYER_EXITING: {
        if (this.trainerExitActive()) {
          return;
        }
        // ASM: engine/battle/core.asm::SendOutPlayerMon.
        this._checkPlayerAmuletCoin();
        this.eventManager.dispatch(
          new Event("play_animation", {
            move_name: "SEND_OUT_MON",
            is_player_move: true,
            param: 0,
          })
        );
        this.trainerStartPhase = TrainerStartPhase.PLAYER_ANIM;
        return;
      }
      case TrainerStartPhase.PLAYER_ANIM: {
        if (this.uiBlocked()) {
          return;
        }
        this.trainerStartPhase = TrainerStartPhase.COMPLETE;
        this.context.currentState = BattleStateEnum.PLAYER_ACTION_SELECT;
        this._clearBattleHasJustStartedFlag();
        return;
      }
      case TrainerStartPhase.COMPLETE:
      default:
        return;
    }
  }

  private uiBlocked(): boolean {
    if (!this.battleUi || this.battleUi.is_mock) {
      return false;
    }
    return should_block_state_advance(this.battleUi);
  }

  private trainerExitActive(): boolean {
    if (!this.battleUi || this.battleUi.is_mock) {
      return false;
    }
    return Boolean(this.battleUi.trainer_exit || this.battleUi.pending_trainer_exit);
  }

  private clearTrainerBattleIntroAnimations(): void {
    if (!this.battleUi || this.battleUi.is_mock) {
      return;
    }
    this.battleUi.trainer_intro = null;
    this.battleUi.trainer_exit = null;
    this.battleUi.pending_trainer_exit = false;
    this.battleUi.pending_trainer_exit_side = null;
    this.battleUi.frontpic_animation = null;
    this.battleUi.trainer_hud_visible = false;
    this.battleUi.sprites_enabled = true;
    this.battleUi.animation_player.reset?.();
  }
}
