import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import { GameState } from "@pokecrystal/core/core/state";
import type { DataLoader } from "@pokecrystal/core/core/data-loader";
import { AudioEngine } from "@pokecrystal/core/engine/systems/audio";
import {
  MysteryGiftAdapter,
  MysteryGiftOutcome,
  MysteryGiftOutcomeStatus,
  MysteryGiftStateMachine,
  PRESS_TO_LINK_TEXT,
  UnavailableMysteryGiftAdapter,
} from "@pokecrystal/core/engine/systems/mystery-gift";
import { buttonKeys, isKeyDownEvent } from "@pokecrystal/core/input/controls";
import { ScreenUI } from "@pokecrystal/core/ui/screens/screen-types";

const confirmKeys = new Set(buttonKeys("a"));
const cancelKeys = new Set(buttonKeys("b"));

type MessageState = {
  text: string;
  outcome?: MysteryGiftOutcome | null;
};

export class MysteryGiftScreen {
  private readonly screen: gameEngine.Surface | null;
  private readonly machine: MysteryGiftStateMachine;
  private message: MessageState = { text: PRESS_TO_LINK_TEXT };
  private awaitingExchange = true;

  constructor(
    private readonly ui: ScreenUI,
    private readonly audioEngine: AudioEngine,
    gameState: GameState,
    adapter: MysteryGiftAdapter | null = null,
    dataLoader: DataLoader | null = null
  ) {
    this.screen = ui.screen;
    this.machine = new MysteryGiftStateMachine(
      gameState,
      adapter ?? new UnavailableMysteryGiftAdapter(),
      dataLoader
    );
  }

  handleInput(event: KeyboardEvent): "exit" | null {
    if (!isKeyDownEvent(event)) {
      return null;
    }
    const key = event.code || event.key;

    if (this.awaitingExchange) {
      if (cancelKeys.has(key)) {
        this.audioEngine.playSound("menu_cancel");
        return "exit";
      }
      if (confirmKeys.has(key)) {
        this.audioEngine.playSound("menu_option");
        const outcome = this.machine.performExchange();
        this.message = { text: outcome.message, outcome };
        this.awaitingExchange = false;
        if (outcome.status === MysteryGiftOutcomeStatus.ERROR) {
          this.machine.reset();
        }
        return null;
      }
      return null;
    }

    if (confirmKeys.has(key)) {
      this.audioEngine.playSound("menu_option");
      if (
        this.message.outcome &&
        this.message.outcome.status === MysteryGiftOutcomeStatus.ERROR
      ) {
        this.resetToPrompt();
        return null;
      }
      return "exit";
    }
    if (cancelKeys.has(key)) {
      this.audioEngine.playSound("menu_cancel");
      return "exit";
    }
    return null;
  }

  draw(): void {
    if (!this.screen) {
      return;
    }
    if (!this.ui.drawTextBox) {
      throw new Error("MysteryGiftScreen requires UI.drawTextBox to render.");
    }
    this.screen.fill([0, 0, 0, 255]);
    const text = this.message.text;
    const boxWidth = 18;
    const boxHeight = Math.max(this.lineCount(text) + 2, 6);
    this.ui.drawTextBox(this.screen, text, 1, 6, boxWidth, boxHeight);
  }

  private resetToPrompt(): void {
    this.message = { text: PRESS_TO_LINK_TEXT };
    this.awaitingExchange = true;
  }

  private lineCount(message: string): number {
    const lines = message.split("\n");
    return lines.length || 1;
  }
}
