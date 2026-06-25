import {
    Pokemon,
    Trainer,
    Item,
    Move as MoveData,
    LearnedMove
  } from '@pokecrystal/core/core/models';
  import {
    GameState
  } from '@pokecrystal/core/core/state';
  import {
    BattleActionType,
    BattleTurn,
    ItemEffect,
    MoveName,
    PlayerGender,
  } from '@pokecrystal/core/core/enums';
  import {
    BattleContext,
    BattleStateEnum,
    BattleAction
  } from './battle-context';
  import {
    determineTurnOrder
  } from './turn-order';
  import {
    executeMove
  } from './move-execution';
import { attackerCannotMove, moveIsDisabled, resolveConfusion } from './status-effects';
  import { getBestMove } from '../ai';
import { attemptRun, enemyShouldFlee } from './flee-logic';
import { applyItemEffect, effectiveItemEffect } from './item-effects';
  import {
    initialisePlayerParty,
    initialiseEnemyParty,
    recordEnemySeen
  } from './battle-setup';
import {
    finaliseBattle,
    handleFaint
  } from './battle-finalization';
import { resolveEndOfTurnEffects } from './residual-effects';
import { tickFutureSight, tickPerishSong } from './between-turn-effects';
import { NameEntryScreen } from '@pokecrystal/core/ui/screens/name-entry-screen';
import { renderTextSnapshot } from '@pokecrystal/core/ui/text-overlays';
import { update as updateBattleUi } from '@pokecrystal/core/ui/overlays/battle-ui-render';
import { NAME_LENGTH } from '@pokecrystal/core/core/constants';
import { B_PAD_A, B_PAD_B } from '@pokecrystal/core/input/controls';
import {
  clear_yes_no_prompt,
  force_party_menu_selection,
  is_waiting_for_input,
  reset_menu_selection,
  should_block_state_advance,
  start_trainer_intro,
  trainer_intro_active,
} from '@pokecrystal/core/ui/overlays/battle-ui-core';
import { maybe_start_pending_evolution } from '@pokecrystal/core/ui/overlays/battle-ui-moves';
import { handle_event as handleBattleEvent, handle_input as handleBattleInput, get_player_input } from '@pokecrystal/core/ui/overlays/battle-ui-input';
import { pushDebugLog } from '@pokecrystal/core/core/debug-log';
import { enqueue_exp_gain } from '@pokecrystal/core/ui/overlays/battle-ui-moves';
import type { GameEngineEvent } from '@pokecrystal/core/ui/game-engine';
  import { Event } from '@pokecrystal/core/engine/events/events';
  import type { EventManager } from '@pokecrystal/core/engine/events/events';
  import type { AudioEngine } from '@pokecrystal/core/engine/systems/audio';
  import { ItemSystem } from '@pokecrystal/core/engine/systems/items';
  import { AUTO_INPUT, type DudeAutoInputController } from '@pokecrystal/core/engine/battle/auto-input';
  import type { Overworld } from '@pokecrystal/core/engine/world/overworld/overworld';
  import { BattleMenu } from '@pokecrystal/core/ui/overlays/_battle-menu';
  import { BattleUIPhase, type BattleUIState } from '@pokecrystal/core/ui/overlays/battle-ui-state';
  import { BattleTransitionManager } from './battle-transition';
  import type { BattleStatusScheduler } from './status-queue';
  import { BattleItemTimeline, type QueuedBattleItem } from './item-timeline';
import type { Surface } from '@pokecrystal/core/ui/surface';

	  enum BattleStartStage {
	    PREPARE,
	    START_TEXT,
	    PLAYER_TEXT,
	    PLAYER_EXITING,
	    PLAYER_ANIM,
	    COMPLETE,
	  }

const AMULET_COIN_ITEM = "AMULET_COIN";
const PARTY_TARGET_BATTLE_ITEM_EFFECTS = new Set<string>([
  ItemEffect.STATUS_HEAL,
  ItemEffect.BITTER_BERRY,
  ItemEffect.FULL_RESTORE,
  ItemEffect.RESTORE_HP,
  ItemEffect.RESTORE_PP,
  ItemEffect.REVIVE,
]);

type NameEntryScreenEvent = Parameters<NameEntryScreen["handleInput"]>[0];

type ActiveNicknameScreen = {
  screen: NameEntryScreen;
  pokemon: Pokemon;
  defaultName: string;
};

  export class Battle {
    public context: BattleContext;
    public gameState: GameState;
    public eventManager: EventManager;
    public battleUi: BattleUIState;
    public audioEngine: AudioEngine | null;
    public movesMap: Map < MoveName, MoveData > ;
    public _overworld: Overworld | null;
    public _dialogueSuspended: boolean;
    public pendingPlayerAction ? : BattleAction;
    public _caughtPokemon: boolean;
    public _playerRan: boolean;
    public _finalised: boolean;
    public _turnCursor: number;
    public _itemSystem: ItemSystem | null;
    public _itemTimeline: BattleItemTimeline | null;
    public _autoInput: DudeAutoInputController | null;
    public _autoInputActive: boolean;
    public _tutorialMenuScriptStarted: boolean;
    public _tutorialPackScriptStarted: boolean;
    public _nicknamePromptPending: boolean;
    public _nicknamePromptShown: boolean;
    public _contestPromptPending: boolean;
    public _transition: BattleTransitionManager | null;
    public _actionCounter: number;
    public _victoryMusicPlayed: boolean;
    public _trainerIntroStarted: boolean;
    public _trainerExitStarted: boolean;
    public _trainerVictorySlideStarted: boolean;
    public _battleMusicStarted: boolean;
    public _blockingStateLogged ? : BattleStateEnum;
    public _battleType: string;
    public _tutorialMode: boolean;
    public _debugMode: boolean;
    public _tutorialActionUsed: boolean;
    public _uiEventBindings: [string, (event: Event, gameState: GameState) => void][];
    public _playerActionMenuResetPending: boolean;
    public _forcedPartyMenuSelection: boolean;
    public _awaitingFaintPrompt: boolean;
    public _pendingBattleItemTarget: Item | null;
    public _suppressAnimationEvents: boolean;
    public _battleStartStage: BattleStartStage;
    public _enemySendOutAnimationStarted: boolean;
    public _playerSendOutAnimationStarted: boolean;
    public _activeFaintSides: Set < BattleTurn > ;
    public _startTextDispatched: boolean;
    public _playerSendOutTextDispatched: boolean;
    public _battleTextCleared: boolean;
    public _entryHazardsApplied: boolean;
    public _statusScheduler: BattleStatusScheduler | null;
    public _battleStartSequenceFinalized: boolean;
    public _enemyTrainerLossSpriteShown: boolean;
    public _battleStartCryPlayed: boolean;
    public _activeNicknameScreen: ActiveNicknameScreen | null;

    constructor(
      playerPokemon: Pokemon,
      enemyPokemon: Pokemon,
      gameState: GameState,
      eventManager: EventManager,
      battleUi: BattleUIState,
      movesMap: Map < MoveName, MoveData > ,
      audioEngine ? : AudioEngine | null,
      trainer ? : Trainer,
      playerParty ? : Pokemon[],
      enemyParty ? : Pokemon[],
      overworld ? : Overworld | null,
      trainerId ? : string,
      trainerReward = 0,
      autoInput ? : DudeAutoInputController | null
    ) {
      this.gameState = gameState;
      this.eventManager = eventManager;
      this.battleUi = battleUi;
      this.audioEngine = audioEngine ?? null;
      this.movesMap = movesMap;
      this._overworld = overworld ?? null;
      this._dialogueSuspended = false;
      this._caughtPokemon = false;
      this._playerRan = false;
      this._finalised = false;
      this._turnCursor = 0;
      this._itemSystem = new ItemSystem(gameState);
      this._itemTimeline = new BattleItemTimeline(eventManager ?? null);
      this._autoInput = autoInput ?? null;
      this._autoInputActive = false;
      this._tutorialMenuScriptStarted = false;
      this._tutorialPackScriptStarted = false;
      this._nicknamePromptPending = false;
      this._nicknamePromptShown = false;
      this._contestPromptPending = false;
      this._transition = null;
      this._actionCounter = 0;
      this._victoryMusicPlayed = false;
      this._trainerIntroStarted = false;
      this._trainerExitStarted = false;
      this._trainerVictorySlideStarted = false;
      this._battleMusicStarted = false;
      const battleType = String(this.gameState?.wram?.battle_type ?? 'BATTLETYPE_NORMAL');
      this._battleType = battleType;
      this._tutorialMode = battleType.toUpperCase() === 'BATTLETYPE_TUTORIAL';
      this._debugMode = false;
      this._tutorialActionUsed = false;
      this._uiEventBindings = [];
      this._playerActionMenuResetPending = false;
      this._forcedPartyMenuSelection = false;
      this._awaitingFaintPrompt = false;
      this._pendingBattleItemTarget = null;
      this._suppressAnimationEvents = false;
      this._battleStartStage = BattleStartStage.PREPARE;
      this._enemySendOutAnimationStarted = false;
      this._playerSendOutAnimationStarted = false;
      this._activeFaintSides = new Set();
      this._startTextDispatched = false;
      this._playerSendOutTextDispatched = false;
      this._battleTextCleared = false;
      this._entryHazardsApplied = false;
      this._statusScheduler = null;
      this._battleStartSequenceFinalized = false;
      this._enemyTrainerLossSpriteShown = false;
      this._battleStartCryPlayed = false;
      this._activeNicknameScreen = null;

      const playerPartyList = initialisePlayerParty(this, playerPokemon, playerParty);
      if (!playerPartyList.length) {
        throw new Error("Cannot start battle without a usable player Pokemon.");
      }
      const enemyPartyList = initialiseEnemyParty(this, enemyPokemon, enemyParty, trainer);
      if (!enemyPartyList.length) {
        throw new Error("Cannot start battle without an enemy Pokemon.");
      }

      this.context = new BattleContext(
        playerPartyList,
        enemyPartyList,
        playerPartyList[0],
        enemyPartyList[0],
        trainer,
        trainer !== undefined,
        trainerId,
        trainerReward
      );

      this.context.initializeBattleParticipants();
      const wram = this.gameState?.wram;
      const battleTypeUpper = String(wram?.battle_type ?? "BATTLETYPE_NORMAL").replace(/,+$/, "").trim().toUpperCase();
      const linkMode = Number(wram?.wLinkMode ?? 0) !== 0;
      this.context.setBadgeBoostState(this.gameState.sram.badges.johto, {
        linkMode,
        inBattleTowerBattle: battleTypeUpper.includes("BATTLE_TOWER"),
      });

      recordEnemySeen(this);

      const battleWram = this.gameState?.wram;
      if (battleWram) {
        battleWram.wBattleParticipantsNotFainted = 0;
        battleWram.wBattleParticipantsIncludingFainted = 0;
        battleWram.wBattleHasJustStarted = 1;
        battleWram.wBattleEnded = 0;
        battleWram.wCurBattleMon = this.context.playerActiveIndex;
        battleWram.wLastPlayerMon = this.context.playerActiveIndex;
        battleWram.wTempBattleMonSpecies = String(this.context.playerPokemon.species.id ?? '').toUpperCase();
        battleWram.wCurPartySpecies = String(this.context.playerPokemon.species.id ?? '').toUpperCase();
      }

      if (!this.gameState?.wram?.instant_mode) {
        const uiScreen = this._battleUiSurface();
        const mapName = this._overworld?.current_map_name ?? '';
        this._transition = new BattleTransitionManager(uiScreen, {
          isTrainerBattle: this.context.trainerBattle,
          playerLevel: this.context.playerPokemon.level,
          enemyLevel: this.context.enemyPokemon.level,
          mapName,
        });
        if (!this._transition.isComplete()) {
          this.context.currentState = BattleStateEnum.BATTLE_TRANSITION;
        } else {
          this._maybeStartTrainerIntro();
        }
      }

      this._register_ui_events();
      this._suspend_overworld_dialogue();
    }

    public update(): void {
      if (this._updateActiveNicknameScreen()) {
        return;
      }
      this._applyAutoInput();
      if (this._itemTimeline?.applying) {
        this.eventManager?.advanceFrame?.();
      }
      this.processState();
    }

    private _applyAutoInput(): void {
      const wram = this.gameState?.wram;
      const joypad = this.gameState?.hram?.joypad;
      if (!wram || !joypad) {
        return;
      }
      const inputType = Number(wram.wInputType ?? 0);
      if (inputType !== AUTO_INPUT) {
        this._autoInputActive = false;
        return;
      }
      if (!this._autoInput) {
        // Recover from stale tutorial state that can survive after the catch demo ends.
        wram.wInputType = 0;
        this._autoInputActive = false;
        return;
      }

      const battleUi = this.battleUi;
      if (battleUi && !battleUi.is_mock && !this._autoInputActive) {
        this._queueTutorialAutoInput(battleUi, this._autoInput);
      }
      const manualPressed = joypad.hJoyPressed ?? joypad.hJoypadPressed ?? 0;
      const manualConfirm = (manualPressed & (B_PAD_A | B_PAD_B)) !== 0;
      if (battleUi && this._tutorialMode) {
        battleUi.fast_animation_request = manualConfirm;
      }
      if (battleUi && this._tutorialMode && manualConfirm && is_waiting_for_input(battleUi)) {
        // Let the player advance tutorial prompts without waiting on auto-input delays.
        this._autoInput.resetIdle();
        this._autoInputActive = false;
        return;
      }
      this._autoInputActive = this._autoInput.step(joypad);
    }

    private _queueTutorialAutoInput(state: BattleUIState, autoInput: DudeAutoInputController): void {
      const currentMenu = state.wram.current_menu;
      if (currentMenu !== BattleMenu.MAIN) {
        this._tutorialMenuScriptStarted = false;
      }
      if (currentMenu !== BattleMenu.PACK) {
        this._tutorialPackScriptStarted = false;
      }
      if (
        this._tutorialMode &&
        state.ui_phase === BattleUIPhase.MENU &&
        this.context.currentState === BattleStateEnum.PLAYER_ACTION_SELECT
      ) {
        if (!this._tutorialMenuScriptStarted && currentMenu === BattleMenu.MAIN) {
          // ASM: engine/battle/core.asm::BattleMenu (auto input selects PACK).
          this._tutorialMenuScriptStarted = true;
          autoInput.queueDownA();
          return;
        }
        if (!this._tutorialPackScriptStarted && currentMenu === BattleMenu.PACK) {
          // ASM: engine/items/pack.asm::TutorialPack (auto input switches to BALL pocket).
          this._tutorialPackScriptStarted = true;
          autoInput.queueRightA();
          return;
        }
      }
      if (is_waiting_for_input(state)) {
        // ASM: home/joypad.asm::PromptButton (auto input advances dialogue).
        autoInput.queueA();
      }
    }

    public isFinished(): boolean {
      return (
        this.context.currentState === BattleStateEnum.BATTLE_END && this._finalised
      );
    }

    public draw(): void {
      if (!this.battleUi) {
        return;
      }
      if (this.context.currentState === BattleStateEnum.BATTLE_TRANSITION && this._transition) {
        this._transition.draw();
        renderTextSnapshot(this.battleUi.ui, {
          viewportLines: [
            'BATTLE TRANSITION',
            'The battle is starting...',
          ],
          infoLines: [
            'Wait: battle intro animation',
          ],
          viewportTitle: 'Battle',
          infoTitle: 'Legend',
          menuLines: null,
          promptLines: null,
          dialogueLines: null,
        });
        return;
      }
      if (this._activeNicknameScreen) {
        this._activeNicknameScreen.screen.draw();
        return;
      }
      updateBattleUi(this.battleUi, this.context);
    }

    public handle_input(event: GameEngineEvent): void {
      if (!this.battleUi) {
        return;
      }
      if (this._activeNicknameScreen) {
        this._activeNicknameScreen.screen.handleInput(event as NameEntryScreenEvent);
        this._finishActiveNicknameScreenIfReady();
        return;
      }
      const inputType = Number(this.gameState?.wram?.wInputType ?? 0);
      if (inputType === AUTO_INPUT) {
        return;
      }
      handleBattleInput(this.battleUi, event);
    }

    public battleUiCall(name: string, ...args: unknown[]): unknown {
      if (!this.battleUi) {
        return null;
      }
      if (name === "enqueue_exp_gain") {
        return enqueue_exp_gain(this.battleUi, ...(args as [Pokemon, number]));
      }
      if (name === "reset_menu_selection") {
        return reset_menu_selection(this.battleUi);
      }
      return null;
    }

    private processState(): void {
      if (this._handlePendingFaintPrompt()) {
        return;
      }
      if (this._handlePendingNicknamePrompt()) {
        return;
      }
      switch (this.context.currentState) {
        case BattleStateEnum.BATTLE_TRANSITION:
          this.handleBattleTransition();
          break;
        case BattleStateEnum.BATTLE_START:
          this.handleBattleStart();
          break;
        case BattleStateEnum.PLAYER_ACTION_SELECT:
          this.handlePlayerActionSelect();
          break;
        case BattleStateEnum.ENEMY_ACTION_SELECT:
          this.handleEnemyActionSelect();
          break;
        case BattleStateEnum.PRE_TURN_EFFECTS:
          this.handlePreTurnEffects();
          break;
        case BattleStateEnum.TURN_EXECUTION:
          this.handleTurnExecution();
          break;
        case BattleStateEnum.POST_TURN_EFFECTS:
          this.handlePostTurnEffects();
          break;
        case BattleStateEnum.BATTLE_END:
          if (this.battleUi) {
            maybe_start_pending_evolution(this.battleUi, this.context);
          }
          if (this.battleUi && should_block_state_advance(this.battleUi)) {
            return;
          }
          finaliseBattle(this);
          this._finalised = true;
          break;
      }
    }

    private _updateActiveNicknameScreen(): boolean {
      const active = this._activeNicknameScreen;
      if (!active) {
        return false;
      }
      active.screen.update();
      if (!this._finishActiveNicknameScreenIfReady()) {
        return true;
      }
      return false;
    }

    private _finishActiveNicknameScreenIfReady(): boolean {
      const active = this._activeNicknameScreen;
      if (!active?.screen.finished) {
        return false;
      }
      const nickname = active.screen.name.trim() || active.defaultName;
      active.pokemon.nickname = nickname.trim();
      this._activeNicknameScreen = null;
      return true;
    }

    private _handlePendingNicknamePrompt(): boolean {
      if (!this.battleUi || !this._nicknamePromptPending || this._nicknamePromptShown) {
        return false;
      }
      const prompt = this.battleUi.yes_no_prompt;
      if (prompt.active || prompt.result === null) {
        return true;
      }
      const request = this.battleUi.pending_nickname_request;
      this.battleUi.pending_nickname_request = null;
      this._nicknamePromptPending = false;
      this._nicknamePromptShown = true;
      const accepted = Boolean(prompt.result);
      clear_yes_no_prompt(this.battleUi);
      if (accepted && request?.pokemon) {
        this._launchNicknameScreen(request.pokemon, request.species_name);
      }
      return true;
    }

    private _handlePendingFaintPrompt(): boolean {
      if (!this.battleUi || !this._awaitingFaintPrompt) {
        return false;
      }
      const prompt = this.battleUi.yes_no_prompt;
      if (prompt.active || prompt.result === null) {
        return true;
      }
      const accepted = Boolean(prompt.result);
      clear_yes_no_prompt(this.battleUi);
      this._awaitingFaintPrompt = false;
      if (!accepted) {
        if (this.context.trainerBattle) {
          this._startFaintedPlayerReplacement();
          return true;
        }
        this.context.currentState = BattleStateEnum.BATTLE_END;
        return true;
      }
      this._enterFaintedPlayerReplacementSelection();
      return true;
    }

    private _enterFaintedPlayerReplacementSelection(preferredIndex?: number | null): void {
      this.context.playerAction = undefined;
      this.context.enemyAction = undefined;
      this.context.currentState = BattleStateEnum.PLAYER_ACTION_SELECT;
      this._forcedPartyMenuSelection = true;
      this._force_party_menu_selection(preferredIndex);
    }

    private _startFaintedPlayerReplacement(): void {
      this._awaitingFaintPrompt = false;
      const preferredIndex = this._preferredFaintedReplacementIndex();
      if (this.context.trainerBattle) {
        if (this.battleUi && !this.battleUi.is_mock) {
          // ASM: AskUseNextPokemon returns immediately for trainer battles; ForcePlayerMonChoice follows.
          this._enterFaintedPlayerReplacementSelection(preferredIndex);
        } else {
          this._performSwitch(BattleTurn.PLAYER, preferredIndex);
        }
        return;
      }
      if (this.battleUi && !this.battleUi.is_mock) {
        // ASM: engine/battle/core.asm::AskUseNextPokemon + data/text/battle.asm::BattleText_UseNextMon
        this._awaitingFaintPrompt = true;
        this.eventManager.dispatch(new Event('prompt_yes_no', { text: 'Use next Pokemon?' }));
      } else {
        this._performSwitch(BattleTurn.PLAYER);
      }
    }

    private _preferredFaintedReplacementIndex(): number | null {
      const activeIndex = this.context.playerActiveIndex;
      return (
        this.context.availablePartyIndices(BattleTurn.PLAYER, true).find(
          (index) => index !== activeIndex,
        ) ?? null
      );
    }

    private _force_party_menu_selection(preferredIndex?: number | null): void {
      if (!this.battleUi || this.battleUi.is_mock) {
        return;
      }
      const partySize = this.context.playerParty?.length ?? 0;
      const options =
        preferredIndex === null || preferredIndex === undefined
          ? undefined
          : { preferred_index: preferredIndex };
      force_party_menu_selection(this.battleUi, partySize, options);
    }

    private _startBattleItemTargetSelection(item: Item): void {
      this._pendingBattleItemTarget = item;
      if (!this.battleUi || this.battleUi.is_mock) {
        return;
      }
      const partySize = this.context.playerParty?.length ?? 0;
      const cursor = Number(this.battleUi.wram.wPartyMenuCursorPosition ?? 0);
      this.battleUi.force_party_menu = false;
      this.battleUi.battle_item_target_selection = true;
      this.battleUi.wram.current_menu = BattleMenu.POKEMON;
      this.battleUi.wram.last_party_size = partySize;
      this.battleUi.wram.wPartyMenuCursorPosition =
        partySize > 0 ? Math.max(0, Math.min(cursor, partySize - 1)) : 0;
    }

    private _battleItemNeedsPartyTarget(item: Item): boolean {
      const battleMenu = String(item.battle_menu ?? "").toUpperCase();
      if (battleMenu) {
        return battleMenu === "ITEMMENU_PARTY";
      }
      return PARTY_TARGET_BATTLE_ITEM_EFFECTS.has(effectiveItemEffect(item));
    }

    private _rejectDisabledPlayerMove(moveName: MoveName): boolean {
      if (!moveIsDisabled(this.context.playerPokemon, moveName)) {
        return false;
      }
      this.eventManager.dispatch(new Event("show_text", { text: "The move is DISABLED!" }));
      if (this.battleUi && !this.battleUi.is_mock) {
        const moveIndex = (this.context.playerPokemon.moves ?? []).findIndex(
          (move) => move?.name === moveName
        );
        this.battleUi.wram.current_menu = BattleMenu.FIGHT;
        this.battleUi.wram.wMoveMenuCursorPosition = moveIndex >= 0 ? moveIndex : 0;
        this.battleUi.wram.swapping_move_index = null;
      }
      return true;
    }

    private _launchNicknameScreen(pokemon: Pokemon, speciesName: string): void {
      if (!this.battleUi) {
        return;
      }
      const ui = this.battleUi.ui;
      const promptText = "NAME YOUR POKéMON?";
      const screen = new NameEntryScreen(ui, promptText, this.battleUi.audio_engine ?? null);
      screen.reset({ prompt: promptText, maxNameLength: NAME_LENGTH - 1 });
      const defaultName = this._nicknameDefaultName(pokemon, speciesName);
      screen.fillName(defaultName);
      this._activeNicknameScreen = { screen, pokemon, defaultName };
    }

    private _nicknameDefaultName(pokemon: Pokemon, speciesName: string): string {
      const candidate =
        String(pokemon.nickname ?? "").trim() ||
        String(speciesName ?? "").trim() ||
        String(pokemon.species?.id ?? "").trim();
      return (candidate || "POKEMON").replace(/_/g, " ");
    }
    protected handleBattleStart(): void {
      if (this._trainerIntroActive()) {
        return;
      }
      if (this.context.trainerBattle) {
        throw new Error("Trainer battles must use TrainerBattle for ASM-accurate start flow.");
      }
      if (!this.battleUi || this.battleUi.is_mock) {
        this._checkPlayerAmuletCoin();
        this.context.currentState = BattleStateEnum.PLAYER_ACTION_SELECT;
        this._clearBattleHasJustStartedFlag();
        return;
      }
      if (this.gameState.wram.instant_mode) {
        this._checkPlayerAmuletCoin();
        this._battleStartStage = BattleStartStage.COMPLETE;
        this.context.currentState = BattleStateEnum.PLAYER_ACTION_SELECT;
        this._clearBattleHasJustStartedFlag();
        return;
      }
      switch (this._battleStartStage) {
        case BattleStartStage.PREPARE: {
          // ASM: engine/battle/core.asm::BattleStartMessage (wild battle cry).
          this._playBattleStartCry();
          // ASM: engine/battle/core.asm::BattleStartMessage (wild encounter text).
          this.eventManager.dispatch(
            new Event("show_text", { text: `Wild ${this.context.enemyPokemon.nickname} appeared!` })
          );
          this._battleStartStage = BattleStartStage.START_TEXT;
          return;
        }
        case BattleStartStage.START_TEXT: {
          if (this._tutorialMode) {
            // ASM: engine/battle/core.asm::DoBattle jumps to BattleMenu for BATTLETYPE_TUTORIAL.
            this._battleStartStage = BattleStartStage.COMPLETE;
            this.context.currentState = BattleStateEnum.PLAYER_ACTION_SELECT;
            this._clearBattleHasJustStartedFlag();
            return;
          }
          // ASM: engine/gfx/pic_animation.asm::AnimateFrontpic (wild battles use ANIM_MON_NORMAL).
          this.eventManager.dispatch(
            new Event("frontpic_animation", { side: "enemy", speed: 0 })
          );
          // ASM: engine/battle/core.asm::BattleMonEntrance shows the player backpic before SendOutMonText.
          this.eventManager.dispatch(new Event("show_trainer_sprites", { mode: "player" }));
          // ASM: engine/battle/core.asm::SendOutMonText (player send-out text).
          this.eventManager.dispatch(
            new Event("show_text", { text: `Go! ${this.context.playerPokemon.nickname}!` })
          );
          this._battleStartStage = BattleStartStage.PLAYER_TEXT;
          return;
        }
        case BattleStartStage.PLAYER_TEXT: {
          // ASM mapping: engine/battle/core.asm::SlideBattlePicOut (a=9 for player-side box) before player send-out.
          this.eventManager.dispatch(new Event("trigger_trainer_exit", { side: "player" }));
          this._battleStartStage = BattleStartStage.PLAYER_EXITING;
          return;
        }
        case BattleStartStage.PLAYER_EXITING: {
          if (this._trainerExitActive()) {
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
          this._battleStartStage = BattleStartStage.PLAYER_ANIM;
          return;
        }
        case BattleStartStage.PLAYER_ANIM: {
          this._battleStartStage = BattleStartStage.COMPLETE;
          this.context.currentState = BattleStateEnum.PLAYER_ACTION_SELECT;
          this._clearBattleHasJustStartedFlag();
          return;
        }
        case BattleStartStage.COMPLETE:
        default:
          return;
      }
    }

    private handleBattleTransition(): void {
      if (!this._transition) {
        this.context.currentState = BattleStateEnum.BATTLE_START;
        this._maybeStartTrainerIntro();
        return;
      }
      if (this._transition.isComplete()) {
        this.context.currentState = BattleStateEnum.BATTLE_START;
        this._maybeStartTrainerIntro();
        return;
      }
      this._transition.advance();
    }

    private handlePlayerActionSelect(): void {
      if (this.context.enemyPokemon.hp <= 0) {
        const battleEnded = handleFaint(this, BattleTurn.ENEMY);
        if (battleEnded) {
          this.context.currentState = BattleStateEnum.BATTLE_END;
        } else {
          this.context.currentState = BattleStateEnum.ENEMY_ACTION_SELECT;
        }
        return;
      }
      if (this.context.playerPokemon.hp <= 0) {
        const activeIndex = this.context.playerActiveIndex;
        if (this.context.playerParticipantsNotFainted.has(activeIndex)) {
          const battleEnded = handleFaint(this, BattleTurn.PLAYER);
          if (battleEnded) {
            this.context.currentState = BattleStateEnum.BATTLE_END;
            return;
          }
        }
        if (!this.context.availablePartyIndices(BattleTurn.PLAYER, true).length) {
          this.context.currentState = BattleStateEnum.BATTLE_END;
          return;
        }
        if (this._awaitingFaintPrompt) {
          return;
        }
        if (!this._forcedPartyMenuSelection) {
          this._startFaintedPlayerReplacement();
          return;
        }
      }
      const forcedReplacement =
        this._forcedPartyMenuSelection && this.context.playerPokemon.hp <= 0;
      if (!this.context.playerAction) {
        if (this.pendingPlayerAction) {
          const action = this.pendingPlayerAction;
          this.pendingPlayerAction = undefined;
          if (action.actionType === BattleActionType.RUN && this.context.trainerBattle) {
            attemptRun(this);
            this._queuePlayerActionMenuReset();
            return;
          }
          if (
            action.actionType === BattleActionType.SWITCH &&
            action.switchToPokemonIndex !== undefined
          ) {
            const resolvedIndex = this._resolvePlayerPartySelection(action.switchToPokemonIndex);
            action.switchToPokemonIndex = resolvedIndex;
            if (this._rejectSwitchToActive(resolvedIndex) || this._rejectSwitchToFainted(resolvedIndex)) {
              return;
            }
            if (forcedReplacement) {
              // ASM: pokecrystal_disassembly/engine/battle/core.asm::HandlePlayerMonFaint
              if (this._performSwitch(BattleTurn.PLAYER, resolvedIndex)) {
                this._forcedPartyMenuSelection = false;
                this._queuePlayerActionMenuReset();
              }
              return;
            }
          }
          this.context.playerAction = action;
          if (
            action.actionType === BattleActionType.MOVE &&
            action.moveName &&
            this._rejectDisabledPlayerMove(action.moveName)
          ) {
            this.context.playerAction = undefined;
            return;
          }
        } else if (this.battleUi) {
          if (this._playerActionMenuResetPending) {
            this._playerActionMenuResetPending = false;
            this.battleUiCall("reset_menu_selection");
          }
          const playerParty = this.context.playerParty ?? [this.context.playerPokemon];
          const items = this._battleMenuItems();
          const selection = get_player_input(
            this.battleUi,
            this.context.playerPokemon.moves ?? [],
            playerParty,
            items
          );
          if (selection === null || selection === undefined) {
            if (
              this._pendingBattleItemTarget &&
              this.battleUi.wram.current_menu !== BattleMenu.POKEMON
            ) {
              this._pendingBattleItemTarget = null;
              this.battleUi.battle_item_target_selection = false;
            }
            return;
          }
          const pendingBattleItem = this._pendingBattleItemTarget;
          if ((this._forcedPartyMenuSelection || pendingBattleItem) && typeof selection !== 'number') {
            return;
          }
          let action: BattleAction | null = null;
          if (pendingBattleItem && typeof selection === 'number') {
            const resolvedIndex = this._resolvePlayerPartySelection(selection);
            const targetPokemon = playerParty[resolvedIndex];
            if (!targetPokemon) {
              this._startBattleItemTargetSelection(pendingBattleItem);
              return;
            }
            this._pendingBattleItemTarget = null;
            if (this.battleUi) {
              this.battleUi.battle_item_target_selection = false;
            }
            action = {
              actionType: BattleActionType.ITEM,
              item: pendingBattleItem,
              targetPokemon,
            };
          } else if (typeof selection === 'number') {
            const resolvedIndex = this._resolvePlayerPartySelection(selection);
            if (this._rejectSwitchToActive(resolvedIndex) || this._rejectSwitchToFainted(resolvedIndex)) {
              return;
            }
            if (forcedReplacement) {
              // ASM: pokecrystal_disassembly/engine/battle/core.asm::HandlePlayerMonFaint
              if (this._performSwitch(BattleTurn.PLAYER, resolvedIndex)) {
                this._forcedPartyMenuSelection = false;
                this._queuePlayerActionMenuReset();
              }
              return;
            }
            action = {
              actionType: BattleActionType.SWITCH,
              switchToPokemonIndex: resolvedIndex,
            };
          } else if (selection === 'RUN') {
            action = { actionType: BattleActionType.RUN };
            if (this.context.trainerBattle) {
              attemptRun(this);
              this._queuePlayerActionMenuReset();
              return;
            }
          } else {
            const moveNames = (this.context.playerPokemon.moves ?? [])
              .filter(Boolean)
              .map((move) => (move as LearnedMove).name);
            if (moveNames.includes(selection as MoveName)) {
              const moveName = selection as MoveName;
              if (this._rejectDisabledPlayerMove(moveName)) {
                return;
              }
              this.context.playerRunAttempts = 0;
              action = {
                actionType: BattleActionType.MOVE,
                moveName,
              };
            } else {
              if (!this._itemSystem) {
                throw new Error("Item system is not available.");
              }
              const itemName = this._tutorialMode ? 'POKE_BALL' : String(selection);
              const item = this._itemSystem.getItemDefinition(itemName);
              if (this._battleItemNeedsPartyTarget(item)) {
                this._startBattleItemTargetSelection(item);
                return;
              }
              action = {
                actionType: BattleActionType.ITEM,
                item,
              };
            }
          }
          if (action) {
            this.context.playerAction = action;
            if (action.actionType === BattleActionType.MOVE) {
              pushDebugLog("[battle] player action move", {
                move: action.moveName ?? null,
                pokemon: this.context.playerPokemon.nickname,
              });
            } else {
              pushDebugLog(`[battle] player action ${action.actionType}`);
            }
            if (action.actionType === BattleActionType.SWITCH) {
              this._forcedPartyMenuSelection = false;
            }
          } else {
            return;
          }
        } else {
          const moveName = this.context.playerPokemon.moves[0]?.name;
          if (moveName) {
            this.context.playerRunAttempts = 0;
            this.context.playerAction = {
              actionType: BattleActionType.MOVE,
              moveName: moveName,
            };
          }
        }
      }
      this.context.currentState = BattleStateEnum.ENEMY_ACTION_SELECT;
    }

    private handleEnemyActionSelect(): void {
      if (!this.context.enemyAction) {
        let enemyPokemon = this.context.enemyPokemon;
        // ASM: pokecrystal_disassembly/engine/battle/core.asm::HandleEnemyMonFaint / HandleEnemySwitch
        if (enemyPokemon.hp <= 0) {
          const switched = this._performSwitch(BattleTurn.ENEMY);
          if (!switched) {
            this.context.currentState = BattleStateEnum.BATTLE_END;
            return;
          }
          enemyPokemon = this.context.enemyPokemon;
        }

        if (enemyPokemon.locked_move && enemyPokemon.locked_turns_remaining > 0) {
          this.context.enemyAction = {
            actionType: BattleActionType.MOVE,
            moveName: enemyPokemon.locked_move,
          };
          this.context.currentState = BattleStateEnum.PRE_TURN_EFFECTS;
          return;
        }

        if (!this.hasUsableMoves(enemyPokemon)) {
          this.context.enemyAction = this.forceStruggleAction(BattleTurn.ENEMY);
          this.context.currentState = BattleStateEnum.PRE_TURN_EFFECTS;
          return;
        }

        if (enemyShouldFlee(this)) {
          this.context.enemyAction = { actionType: BattleActionType.RUN };
          this.context.currentState = BattleStateEnum.PRE_TURN_EFFECTS;
          return;
        }

        const moveIsDisabled = (moveName?: MoveName): boolean => {
          return Boolean(
            moveName &&
              enemyPokemon.disable_turns > 0 &&
              enemyPokemon.disabled_move === moveName
          );
        };

        const bestMove = getBestMove(this.context, this.gameState, this.movesMap);
        let chosenMoveName: MoveName | undefined;
        if (bestMove) {
          const learnedMove = this.getLearnedMove(enemyPokemon, bestMove.name);
          if (learnedMove && learnedMove.current_pp > 0 && !moveIsDisabled(bestMove.name)) {
            chosenMoveName = bestMove.name;
          }
        }

        if (!chosenMoveName) {
          for (const learnedMove of enemyPokemon.moves) {
            if (learnedMove && learnedMove.current_pp > 0 && !moveIsDisabled(learnedMove.name)) {
              chosenMoveName = learnedMove.name;
              break;
            }
          }
        }

        if (!chosenMoveName && enemyPokemon.moves.length) {
          for (const learnedMove of enemyPokemon.moves) {
            if (learnedMove && !moveIsDisabled(learnedMove.name)) {
              chosenMoveName = learnedMove.name;
              break;
            }
          }
        }

        if (chosenMoveName) {
          this.context.enemyAction = {
            actionType: BattleActionType.MOVE,
            moveName: chosenMoveName,
          };
        } else {
          this.context.enemyAction = this.forceStruggleAction(BattleTurn.ENEMY);
        }
        if (this.context.enemyAction?.actionType === BattleActionType.MOVE) {
          pushDebugLog("[battle] enemy action move", {
            move: this.context.enemyAction.moveName ?? null,
            pokemon: enemyPokemon.nickname,
          });
        }
      }
      this.context.currentState = BattleStateEnum.PRE_TURN_EFFECTS;
    }

    private hasUsableMoves(pokemon: Pokemon): boolean {
      if (pokemon.hp <= 0) {
        return false;
      }
      let hasLearnedMoves = false;
      for (const learnedMove of pokemon.moves.slice(0, 4)) {
        if (!learnedMove) {
          continue;
        }
        hasLearnedMoves = true;
        const moveDef = this.movesMap.get(learnedMove.name);
        if (!moveDef) {
          continue;
        }
        if (moveDef.pp === 0 || learnedMove.current_pp > 0) {
          return true;
        }
      }
      return !hasLearnedMoves;
    }

    private forceStruggleAction(side: BattleTurn): BattleAction {
      if (side === BattleTurn.PLAYER) {
        this.context.playerForcedStruggle = true;
      } else {
        this.context.enemyForcedStruggle = true;
      }
      return {
        actionType: BattleActionType.MOVE,
        moveName: MoveName.STRUGGLE,
      };
    }

    private getLearnedMove(pokemon: Pokemon, moveName: MoveName): LearnedMove | undefined {
      for (const learnedMove of pokemon.moves) {
        if (learnedMove && learnedMove.name === moveName) {
          return learnedMove;
        }
      }
      return undefined;
    }

    private _queuePlayerActionMenuReset(): void {
      this._playerActionMenuResetPending = true;
    }

    private _resolvePlayerPartySelection(selectionIndex: number): number {
      const party = this.context.playerParty ?? [];
      const mappedIndex = party.findIndex(
        (pokemon) =>
          typeof (pokemon as { _sram_slot?: number })._sram_slot === "number" &&
          (pokemon as { _sram_slot?: number })._sram_slot === selectionIndex
      );
      return mappedIndex >= 0 ? mappedIndex : selectionIndex;
    }

    private _applyPlayerActionMenuReset(): void {
      if (!this._playerActionMenuResetPending) {
        return;
      }
      this._playerActionMenuResetPending = false;
      this.battleUiCall("reset_menu_selection");
    }

    // ASM: pokecrystal_disassembly/engine/battle/core.asm::SwitchMonAlreadyOut
    private _rejectSwitchToActive(selectionIndex: number): boolean {
      if (selectionIndex !== this.context.playerActiveIndex) {
        return false;
      }
      const active = this.context.playerPokemon;
      this.eventManager?.dispatch?.(
        new Event("show_text", { text: `${active.nickname} is already out.` })
      );
      if (this._forcedPartyMenuSelection) {
        this._force_party_menu_selection(selectionIndex);
      } else {
        this._queuePlayerActionMenuReset();
      }
      return true;
    }

    // ASM: pokecrystal_disassembly/data/text/battle.asm::BattleText_TheresNoWillToBattle
    private _rejectSwitchToFainted(selectionIndex: number): boolean {
      const party = this.context.playerParty ?? [];
      if (selectionIndex < 0 || selectionIndex >= party.length) {
        return false;
      }
      const selection = party[selectionIndex];
      if (!selection || selection.hp > 0) {
        return false;
      }
      this.eventManager?.dispatch?.(
        new Event("show_text", { text: "There's no will to battle!" })
      );
      if (this._forcedPartyMenuSelection) {
        this._force_party_menu_selection(selectionIndex);
      } else {
        this._queuePlayerActionMenuReset();
      }
      if (!this._forcedPartyMenuSelection && this.battleUi && !this.battleUi.is_mock) {
        const size = party.length;
        this.battleUi.force_party_menu = false;
        this.battleUi.wram.current_menu = BattleMenu.POKEMON;
        this.battleUi.wram.last_party_size = size;
        if (size > 0) {
          const cursor = Math.max(0, Math.min(selectionIndex, size - 1));
          this.battleUi.wram.wPartyMenuCursorPosition = cursor;
        }
      }
      return true;
    }

    private _resetPlayerRunAttempts(): void {
      this.context.playerRunAttempts = 0;
    }

    private _battleUiSurface(): Surface | null {
      const ui = this.battleUi?.ui as { screen?: Surface } | undefined;
      return ui?.screen ?? null;
    }

    protected _clearBattleHasJustStartedFlag(): void {
      const wram = this.gameState?.wram;
      if (wram) {
        wram.wBattleHasJustStarted = 0;
      }
    }

    protected _checkPlayerAmuletCoin(): void {
      if (this.context.amuletCoinActive) {
        return;
      }
      const heldItem = String(this.context.playerPokemon?.item ?? "").trim().toUpperCase();
      if (heldItem === AMULET_COIN_ITEM) {
        // ASM: engine/battle/core.asm::SendOutPlayerMon -> CheckAmuletCoin.
        this.context.amuletCoinActive = true;
      }
    }

    protected _trainerIntroActive(): boolean {
      if (!this.battleUi || this.battleUi.is_mock) {
        return false;
      }
      if (typeof this.battleUi.trainer_intro === 'undefined') {
        return false;
      }
      return trainer_intro_active(this.battleUi);
    }

    protected _trainerExitActive(): boolean {
      if (!this.battleUi || this.battleUi.is_mock) {
        return false;
      }
      return Boolean(this.battleUi.trainer_exit || this.battleUi.pending_trainer_exit);
    }

    protected _playBattleStartCry(): void {
      if (this._battleStartCryPlayed) {
        return;
      }
      const speciesId = this.context.enemyPokemon?.species?.id;
      if (!speciesId) {
        return;
      }
      const soundId = `CRY_${String(speciesId).toUpperCase()}`;
      try {
        // ASM: battle core uses PlayStereoCry; enemy cries are routed to the right speaker.
        this.audioEngine?.playSound(soundId, { panning: "enemy" });
      } catch {
        // Ignore missing cries in headless environments.
      }
      this._battleStartCryPlayed = true;
    }

    private _maybeStartTrainerIntro(): boolean {
      if (!this.battleUi || this.battleUi.is_mock) {
        return false;
      }
      if (this.gameState?.wram?.instant_mode) {
        return false;
      }
      if (!this.context.trainerBattle) {
        return false;
      }
      const ui = this.battleUi.ui as { get_sprite_surface?: unknown; screen?: unknown } | undefined;
      if (!ui || typeof ui.get_sprite_surface !== 'function' || !ui.screen) {
        return false;
      }
      if (this._trainerIntroActive()) {
        return true;
      }
      if (this._trainerIntroStarted) {
        return false;
      }
      const wram = this.gameState?.wram;
      if (!wram) {
        return false;
      }
      start_trainer_intro(this.battleUi, {
        player_gender: wram.player_gender ?? PlayerGender.MALE,
        trainer_class: String(wram.other_trainer_class ?? ''),
        enemy_species: this.context.enemyPokemon.species.id,
        battle_type: String(wram.battle_type ?? 'BATTLETYPE_NORMAL'),
        enemy_party_size: this.context.enemyParty?.length ?? 0,
      });
      this._trainerIntroStarted = true;
      return this._trainerIntroActive();
    }

    private _drawBattleTransitionPortraits(): void {
      if (!this.context.trainerBattle) {
        return;
      }
      const ui = this.battleUi?.ui as unknown as {
        screen?: Surface;
        draw_sprite?: (
          spriteId: string,
          x: number,
          y: number,
          options?: { sprite_type?: string; frame?: number },
        ) => void;
        get_sprite_surface?: (spriteId: string, spriteType: string) => Surface | null;
      };
      const screen = ui?.screen;
      if (!screen) {
        return;
      }
      const playerGender = this.gameState?.wram?.player_gender ?? PlayerGender.MALE;
      const playerSprite = playerGender === PlayerGender.FEMALE ? 'kris' : 'cal';
      const enemySprite = this._trainerSpriteIdFromClass(
        String(this.gameState?.wram?.other_trainer_class ?? ''),
      );
      if (!playerSprite && !enemySprite) {
        return;
      }
      const margin = 8;
      const placements: Array<[string, number, number]> = [];
      if (playerSprite) {
        placements.push([playerSprite, margin, 16]);
      }
      if (enemySprite) {
        const enemyX = Math.max(margin, screen.width - 56 - margin);
        placements.push([enemySprite, enemyX, 10]);
      }
      if (typeof ui.draw_sprite === 'function') {
        for (const [spriteId, x, y] of placements) {
          ui.draw_sprite(spriteId, x, y, { sprite_type: 'trainer' });
        }
        return;
      }
      if (typeof ui.get_sprite_surface === 'function') {
        for (const [spriteId, x, y] of placements) {
          const sprite = ui.get_sprite_surface(spriteId, 'trainer');
          if (sprite) {
            screen.blit(sprite, [x, y]);
          }
        }
      }
    }

    private _trainerSpriteIdFromClass(trainerClass: string): string | null {
      const normalized = String(trainerClass ?? '').trim().toLowerCase();
      if (!normalized) {
        return null;
      }
      if (normalized.endsWith('m') && !normalized.endsWith('_m')) {
        return `${normalized.slice(0, -1)}_m`;
      }
      if (normalized.endsWith('f') && !normalized.endsWith('_f')) {
        return `${normalized.slice(0, -1)}_f`;
      }
      return normalized;
    }

    // ASM: engine/battle/core.asm::BattleMenu_Run
    private _handleRunSelection(action: BattleAction): boolean {
      if (action.actionType !== BattleActionType.RUN) {
        return false;
      }
      if (!this.context.trainerBattle) {
        return false;
      }
      attemptRun(this);
      this._queuePlayerActionMenuReset();
      return true;
    }

    private _resolveItem(itemName: string): Item {
      if (!this._itemSystem) {
        throw new Error("Item system is not available.");
      }
      return this._itemSystem.getItemDefinition(itemName);
    }

    private _battleMenuItems(): Record<string, number> {
      if (this._tutorialMode) {
        const wram = this.gameState.wram ?? ({} as GameState['wram']);
        return {
          ...(wram.wDudeItems ?? {}),
          ...(wram.wDudeBalls ?? {}),
          ...(wram.wDudeKeyItems ?? {}),
        };
      }
      const sram = this.gameState.sram ?? ({} as GameState['sram']);
      return {
        ...(sram.items ?? {}),
        ...(sram.balls ?? {}),
        ...(sram.key_items ?? {}),
      };
    }

    private _consumePlayerItem(item: Item): boolean {
      if (this._tutorialMode) {
        return true;
      }
      if (!this._itemSystem) {
        return false;
      }
      return this._itemSystem.removeItem(item);
    }

    private _participantsForTurn(
      attackerKey: BattleTurn
    ): { attacker: Pokemon; defender: Pokemon; action?: BattleAction } {
      if (attackerKey === BattleTurn.PLAYER) {
        return {
          attacker: this.context.playerPokemon,
          defender: this.context.enemyPokemon,
          action: this.context.playerAction,
        };
      }
      return {
        attacker: this.context.enemyPokemon,
        defender: this.context.playerPokemon,
        action: this.context.enemyAction,
      };
    }

    private _queueBattleItem(
      attackerKey: BattleTurn,
      attacker: Pokemon,
      action: BattleAction,
    ): void {
      const item = action.item;
      if (!item) {
        return;
      }
      // ASM: engine/battle/core.asm::DoItemEffect
      if (this.eventManager?.dispatch) {
        const playerName = this.gameState.sram.player_name || 'PLAYER';
        const usedText =
          attackerKey === BattleTurn.PLAYER
            ? `${playerName} used the ${item.name}.`
            : `${attacker.nickname} used ${item.name}!`;
        this.eventManager.dispatch(
          new Event("show_text", { text: usedText })
        );
      }

      const queued: QueuedBattleItem = {
        attackerSide: attackerKey,
        item,
        target: action.targetPokemon ?? attacker,
        moveIndex: action.targetMoveIndex ?? null,
      };
      if (!this._itemTimeline) {
        const result = this._applyQueuedItem(queued);
        this._onItemComplete(queued, result);
        return;
      }
      this._itemTimeline.queue(queued, this._applyQueuedItem.bind(this), this._onItemComplete.bind(this));
    }

    private _shouldPromptNickname(): boolean {
      if (this._tutorialMode) {
        return false;
      }
      const battleType = String(this._battleType ?? "").toUpperCase();
      return ![
        "BATTLETYPE_CONTEST",
        "BATTLETYPE_BUG_CONTEST",
        "BATTLETYPE_PARK",
      ].includes(battleType);
    }

    private _applyQueuedItem(queued: QueuedBattleItem): boolean {
      return applyItemEffect(
        queued.item,
        queued.target,
        this.eventManager,
        this.context,
        this.gameState,
        queued.moveIndex,
      );
    }

    private _onItemComplete(_queued: QueuedBattleItem, battleResolved: boolean): void {
      if (!battleResolved) {
        return;
      }
      if (this.context.runAttemptSuccess) {
        this.context.runAttemptSuccess = false;
        this._playerRan = true;
        this.context.currentState = BattleStateEnum.BATTLE_END;
        this._turnCursor = this.context.turnOrder.length;
        return;
      }
      this._caughtPokemon = true;
      this._nicknamePromptPending = this._shouldPromptNickname();
      this._nicknamePromptShown = false;
      this.context.currentState = BattleStateEnum.BATTLE_END;
      this._turnCursor = this.context.turnOrder.length;
    }

    // ASM: pokecrystal_disassembly/engine/battle/core.asm::SwitchTurnCore
    private _performSwitch(side: BattleTurn, targetIndex?: number | null): boolean {
      const available = this.context.availablePartyIndices(side, true);
      if (!available.length) {
        return false;
      }

      let resolvedIndex = targetIndex ?? null;
      if (resolvedIndex === null || !available.includes(resolvedIndex)) {
        const activeIndex = this.context.activeIndexFor(side);
        resolvedIndex = available.find((idx) => idx !== activeIndex) ?? null;
      }
      if (resolvedIndex === null) {
        return false;
      }
      if (resolvedIndex === this.context.activeIndexFor(side)) {
        return false;
      }

      const party = this.context.partyFor(side);
      const next = party[resolvedIndex];
      if (!next || next.hp <= 0) {
        return false;
      }

      const active = side === BattleTurn.PLAYER ? this.context.playerPokemon : this.context.enemyPokemon;
      if (active.hp > 0 && active.trapped_turns > 0) {
        this.eventManager?.dispatch?.(
          new Event("show_text", { text: `${active.nickname} can't be switched out!` })
        );
        return false;
      }

      if (active.rage_active || active.rage_counter) {
        // ASM mapping: pokecrystal_disassembly/engine/battle/core.asm (ResetVarsForSubstatusRage on switch).
        active.rage_active = false;
        active.rage_counter = 0;
      }

      this.context.switchActive(side, resolvedIndex);
      this._activeFaintSides.delete(side);
      if (side === BattleTurn.PLAYER) {
        this._checkPlayerAmuletCoin();
      }
      return true;
    }

    private _executeTurnAttacker(attackerKey: BattleTurn): void {
      const { attacker, defender, action } = this._participantsForTurn(attackerKey);
      if (attacker.hp <= 0) {
        return;
      }

      attacker.turns_in_battle += 1;

      if (action?.actionType === BattleActionType.SWITCH) {
        this._performSwitch(attackerKey, action.switchToPokemonIndex);
        return;
      }

      if (action?.actionType === BattleActionType.RUN) {
        if (attackerKey === BattleTurn.PLAYER) {
          if (attemptRun(this)) {
            this._playerRan = true;
            this.context.currentState = BattleStateEnum.BATTLE_END;
            return;
          }
        } else {
          const fleeText = this.context.trainerBattle
            ? `The opposing ${attacker.nickname} fled!`
            : `Wild ${attacker.nickname} fled!`;
          this.eventManager?.dispatch?.(new Event("show_text", { text: fleeText }));
          this.context.currentState = BattleStateEnum.BATTLE_END;
          return;
        }
        return;
      }

      if (action?.actionType === BattleActionType.ITEM && action.item) {
        if (attackerKey === BattleTurn.PLAYER && !this._consumePlayerItem(action.item)) {
          this.eventManager?.dispatch?.(
            new Event("show_text", { text: `But you have no ${action.item.name}!` })
          );
          return;
        }
        this._queueBattleItem(attackerKey, attacker, action);
        return;
      }

      if (action?.actionType === BattleActionType.MOVE && action.moveName) {
        if (attackerCannotMove(this, attacker) || resolveConfusion(this, attacker)) {
          return;
        }
        executeMove(this, attackerKey, attacker, defender, action.moveName);
      }
    }

    private handlePreTurnEffects(): void {
      this.context.turnOrder = determineTurnOrder(this);
      this._turnCursor = 0;
      this.context.currentState = BattleStateEnum.TURN_EXECUTION;
    }

    private handleTurnExecution(): void {
      if (this.battleUi && should_block_state_advance(this.battleUi)) {
        return;
      }
      if (this._itemTimeline?.applying) {
        return;
      }

      const turnOrder = this.context.turnOrder;
      if (this._turnCursor >= turnOrder.length) {
        if ((this.context.currentState as BattleStateEnum) !== BattleStateEnum.BATTLE_END) {
          this.context.currentState = BattleStateEnum.POST_TURN_EFFECTS;
        }
        this._turnCursor = 0;
        return;
      }

      if (this.context.currentState === BattleStateEnum.BATTLE_END) {
        return;
      }
      const turn = turnOrder[this._turnCursor];
      this._turnCursor += 1;
      this._executeTurnAttacker(turn);
      if (this._itemTimeline?.applying) {
        return;
      }
      if (this._handleFaintAfterAction(turn)) {
        return;
      }
      if (this._turnCursor >= turnOrder.length) {
        if ((this.context.currentState as BattleStateEnum) !== BattleStateEnum.BATTLE_END) {
          this.context.currentState = BattleStateEnum.POST_TURN_EFFECTS;
        }
        this._turnCursor = 0;
      }
    }

    private _handleFaintAfterAction(attackerKey: BattleTurn): boolean {
      if (this.context.currentState === BattleStateEnum.BATTLE_END) {
        return true;
      }

      const first = attackerKey === BattleTurn.PLAYER ? BattleTurn.ENEMY : BattleTurn.PLAYER;
      const second = attackerKey;
      let faintHandled = false;

      for (const side of [first, second]) {
        const active = side === BattleTurn.PLAYER ? this.context.playerPokemon : this.context.enemyPokemon;
        if (active.hp > 0) {
          continue;
        }

        faintHandled = true;
        const battleEnded = handleFaint(this, side);
        if (battleEnded) {
          this.context.currentState = BattleStateEnum.BATTLE_END;
          this._turnCursor = this.context.turnOrder.length;
          return true;
        }
      }

      if (!faintHandled) {
        return false;
      }

      this.context.playerAction = undefined;
      this.context.enemyAction = undefined;
      this._turnCursor = this.context.turnOrder.length;

      if (this.context.playerPokemon.hp <= 0) {
        this.context.currentState = BattleStateEnum.PLAYER_ACTION_SELECT;
      } else if (this.context.enemyPokemon.hp <= 0) {
        this.context.currentState = BattleStateEnum.ENEMY_ACTION_SELECT;
      } else {
        this.context.currentState = BattleStateEnum.POST_TURN_EFFECTS;
      }
      return true;
    }

    private handlePostTurnEffects(): void {
      const futureSightPlayerFainted = tickFutureSight(this.context, this.eventManager, BattleTurn.PLAYER);
      const futureSightEnemyFainted = tickFutureSight(this.context, this.eventManager, BattleTurn.ENEMY);
      const perishPlayerFainted = tickPerishSong(this.context, this.eventManager, BattleTurn.PLAYER);
      const perishEnemyFainted = tickPerishSong(this.context, this.eventManager, BattleTurn.ENEMY);
      const residualOutcome = resolveEndOfTurnEffects(this.context, this.eventManager);

      if (futureSightPlayerFainted || perishPlayerFainted || residualOutcome.player_fainted) {
        this.context.playerPokemon.hp = 0;
      }
      if (futureSightEnemyFainted || perishEnemyFainted || residualOutcome.enemy_fainted) {
        this.context.enemyPokemon.hp = 0;
      }

      const playerBattleEnded = handleFaint(this, BattleTurn.PLAYER);
      const enemyBattleEnded = handleFaint(this, BattleTurn.ENEMY);
      if (playerBattleEnded || enemyBattleEnded) {
        this.context.currentState = BattleStateEnum.BATTLE_END;
        return;
      }
      if (this.battleUi && should_block_state_advance(this.battleUi)) {
        return;
      }

      const needsReplacement =
        this.context.playerPokemon.hp <= 0 &&
        this.context.availablePartyIndices(BattleTurn.PLAYER, true).length > 0;
      if (needsReplacement && !this._awaitingFaintPrompt) {
        this._startFaintedPlayerReplacement();
        return;
      }

      this.context.playerAction = undefined;
      this.context.enemyAction = undefined;
      this.context.currentState = BattleStateEnum.PLAYER_ACTION_SELECT;
    }

    public queuePlayerAction(action: BattleAction): void {
      this.pendingPlayerAction = action;
    }

    public _autoSwitch(side: BattleTurn, suppressReturnAnimation: boolean): void {
      void suppressReturnAnimation;
      this._performSwitch(side);
    }

    public teardown(): void {
      this._unregister_ui_events();
      this._resume_overworld_dialogue();
    }

    public prepareForOverworldResume(): void {
      this._unregister_ui_events();
      this._resume_overworld_dialogue();
    }

    private _register_ui_events(): void {
      if (!this.eventManager || !this.battleUi) {
        return;
      }
      if (this._uiEventBindings.length) {
        return;
      }
      const handler = (event: Event, state: GameState) => {
        handleBattleEvent(this.battleUi, event, state);
      };
      const events = [
        'show_text',
        'open_text',
        'close_text',
        'wait_for_input',
        'prompt_yes_no',
        'nickname_prompt',
        'play_animation',
        'frontpic_animation',
        'show_trainer_sprites',
        'trigger_trainer_exit',
      ];
      for (const eventName of events) {
        this.eventManager.on(eventName, handler);
        this._uiEventBindings.push([eventName, handler]);
      }
    }

    private _unregister_ui_events(): void {
      if (!this.eventManager || !this._uiEventBindings.length) {
        return;
      }
      for (const [eventName, handler] of this._uiEventBindings) {
        this.eventManager.off(eventName, handler);
      }
      this._uiEventBindings = [];
    }

    private _suspend_overworld_dialogue(): void {
      if (this._dialogueSuspended) {
        return;
      }
      if (this._overworld?.suspend_dialogue_events) {
        this._overworld.suspend_dialogue_events();
        this._dialogueSuspended = true;
      }
    }

    private _resume_overworld_dialogue(): void {
      if (!this._dialogueSuspended) {
        return;
      }
      if (this._overworld?.resume_dialogue_events) {
        this._overworld.resume_dialogue_events();
      }
      this._dialogueSuspended = false;
    }
  }
