import { GameState } from "@pokecrystal/core/core/state";
import { EventManager, openText, closeText } from "@pokecrystal/core/engine/events/events";
import { getFruitTreeItem } from "@pokecrystal/assets/content/fruit-trees";
import { DEFAULT_FRUIT_TREE_ITEM, LOGGER, showText, waitForInput } from "../common";
import { applyEventFlag } from "../event-flags";
import { resolveText } from "../text-helpers";
import type { AudioEngine } from "@pokecrystal/core/engine/systems/audio";
import {
  Command,
  ScriptFrame,
  OverworldContext,
  addItemToBag,
  resolveDisplayName,
  resolveItemSystem,
} from "./base";
import { CloseTextCommand, OpenTextCommand, WaitButtonCommand } from "./text";
import type { ScriptRunner } from "../runner";

const FRUIT_TREE_RESULT_KEY = "_fruit_tree_success";
const FRUIT_TREE_SFX = "SFX_ITEM";

type FruitTreeRunner = ScriptRunner & {
  audio_engine?: AudioEngine | null;
  audioEngine?: AudioEngine | null;
  set_event_flag?: (flag: string, value: boolean) => void;
  _script_stack?: ScriptFrame[];
};

type FruitTreeOverworld = OverworldContext & {
  dialogue?: { draw?: () => void } | null;
  audio_engine?: AudioEngine | null;
  audioEngine?: AudioEngine | null;
};

const waitForPlayerAck = (
  runner: FruitTreeRunner | undefined,
  eventManager: EventManager,
  overworld: FruitTreeOverworld | null,
): void => {
  const dialogue = overworld?.dialogue ?? null;
  if (runner && dialogue) {
    runner.pause?.();
  }
  waitForInput(eventManager);
};

class ShowResolvedTextCommand extends Command {
  constructor(private label: string) {
    super();
  }

  public execute(_gameState: GameState, eventManager: EventManager, overworld: FruitTreeOverworld): void {
    const runner = this.runner as FruitTreeRunner | undefined;
    const message = resolveText(runner ?? null, overworld, this.label);
    showText(eventManager, message);
  }
}

class ShowFruitTreeOutcomeCommand extends Command {
  constructor(
    private resultKey: string,
    private successLabel: string,
    private failureLabel: string,
  ) {
    super();
  }

  public execute(_gameState: GameState, eventManager: EventManager, overworld: FruitTreeOverworld): void {
    const runner = this.runner as FruitTreeRunner | undefined;
    const success = Boolean(runner?.variables?.[this.resultKey]);
    const label = success ? this.successLabel : this.failureLabel;
    const message = resolveText(runner ?? null, overworld, label);
    showText(eventManager, message);
  }
}

class AttemptFruitTreePickupCommand extends Command {
  constructor(
    private flag: string,
    private itemName: string,
    private resultKey: string,
    private readonly audioEngine?: AudioEngine | null,
  ) {
    super();
  }

  public execute(gameState: GameState, _eventManager: EventManager, overworld: FruitTreeOverworld): void {
    const runner = this.runner as FruitTreeRunner | undefined;
    const itemSystem = resolveItemSystem(runner, overworld);
    const success = addItemToBag(gameState, itemSystem, this.itemName);
    if (runner) {
      if (!runner.variables) {
        runner.variables = {};
      }
      runner.variables[this.resultKey] = success;
      runner.last_condition_result = success;
    }

    if (success) {
      if (runner?.set_event_flag) {
        runner.set_event_flag(this.flag, true);
      } else {
        applyEventFlag(gameState, this.flag, { value: true, overworld });
      }
      let audioEngine = this.audioEngine;
      if (!audioEngine && overworld) {
        audioEngine = overworld.audio_engine ?? overworld.audioEngine ?? null;
      }
      if (audioEngine?.play_sound) {
        audioEngine.play_sound(FRUIT_TREE_SFX);
      } else if (audioEngine?.playSound) {
        audioEngine.playSound(FRUIT_TREE_SFX);
      }
      if (runner) {
        runner.last_sound_effect = FRUIT_TREE_SFX;
      }
    } else if (runner) {
      runner.last_sound_effect = null;
    }
  }
}

class FinalizeFruitTreeResultCommand extends Command {
  constructor(private resultKey: string) {
    super();
  }

  public execute(_gameState: GameState, _eventManager: EventManager, _overworld: FruitTreeOverworld): void {
    const runner = this.runner as FruitTreeRunner | undefined;
    if (!runner) {
      return;
    }
    const success = Boolean(runner.variables?.[this.resultKey]);
    runner.last_condition_result = success;
    if (runner.variables) {
      delete runner.variables[this.resultKey];
    }

    const stack = runner._script_stack as ScriptFrame[] | undefined;
    if (!stack || stack.length === 0) {
      return;
    }

    const frame = stack[stack.length - 1];
    frame.index = frame.commands.length;
    frame.allowFallthrough = false;

    if (stack.length >= 2) {
      const parent = stack[stack.length - 2];
      parent.index = parent.commands.length;
      parent.allowFallthrough = false;
    }
  }
}

class SetRunnerConditionCommand extends Command {
  constructor(private value: boolean) {
    super();
  }

  public execute(_gameState: GameState, _eventManager: EventManager, _overworld: FruitTreeOverworld): void {
    const runner = this.runner as FruitTreeRunner | undefined;
    if (runner) {
      runner.last_condition_result = this.value;
    }
  }
}

export class FruitTreeCommand extends Command {
  constructor(private treeId: string) {
    super();
  }

  public execute(gameState: GameState, eventManager: EventManager, overworld: FruitTreeOverworld): void {
    const runner = this.runner as FruitTreeRunner | undefined;
    let audioEngine = overworld?.audio_engine ?? overworld?.audioEngine ?? null;
    if (!audioEngine && runner) {
      audioEngine = runner.audio_engine ?? runner.audioEngine ?? null;
    }

    const treeConstant = this.treeId.startsWith("FRUITTREE_")
      ? this.treeId
      : `FRUITTREE_${this.treeId}`;
    const flag = `${treeConstant}_COLLECTED`;
    let rewardItem = DEFAULT_FRUIT_TREE_ITEM;
    try {
      rewardItem = getFruitTreeItem(treeConstant) ?? DEFAULT_FRUIT_TREE_ITEM;
    } catch {
      LOGGER.debug("Unknown fruit tree '%s'; defaulting to %s", treeConstant, DEFAULT_FRUIT_TREE_ITEM);
    }

    const itemSystem = resolveItemSystem(runner, overworld);
    const displayName = resolveDisplayName(itemSystem, rewardItem);
    if (runner) {
      if (!runner.string_buffers) {
        runner.string_buffers = {};
      }
      runner.string_buffers["STRING_BUFFER_1"] = displayName;
      runner.string_buffers["STRING_BUFFER_3"] = displayName;
    }

    const stack = runner?._script_stack as ScriptFrame[] | undefined;
    if (!runner || !stack || stack.length === 0) {
      this.executeImmediate(gameState, eventManager, overworld, flag, rewardItem, audioEngine);
      return;
    }

    const commands = this.buildInteractiveCommands(
      flag,
      rewardItem,
      Boolean(gameState.wram.event_flags[flag]),
      audioEngine,
    );
    for (const command of commands) {
      command.runner = runner;
    }
    const frameName = `${stack[stack.length - 1].name}#fruit_tree`;
    stack.push({ name: frameName, commands, index: 0 });
  }

  private buildInteractiveCommands(
    flag: string,
    rewardItem: string,
    alreadyCollected: boolean,
    audioEngine: AudioEngine | null,
  ): Command[] {
    const commands: Command[] = [
      new OpenTextCommand(),
      new ShowResolvedTextCommand("FruitBearingTreeText"),
      new WaitButtonCommand(),
    ];
    if (alreadyCollected) {
      commands.push(
        new ShowResolvedTextCommand("NothingHereText"),
        new WaitButtonCommand(),
        new CloseTextCommand(),
        new FinalizeFruitTreeResultCommand(FRUIT_TREE_RESULT_KEY),
      );
      return commands;
    }

    commands.push(
      new ShowResolvedTextCommand("HeyItsFruitText"),
      new WaitButtonCommand(),
      new AttemptFruitTreePickupCommand(flag, rewardItem, FRUIT_TREE_RESULT_KEY, audioEngine),
      new ShowFruitTreeOutcomeCommand(
        FRUIT_TREE_RESULT_KEY,
        "ObtainedFruitText",
        "FruitPackIsFullText",
      ),
      new WaitButtonCommand(),
      new CloseTextCommand(),
      new FinalizeFruitTreeResultCommand(FRUIT_TREE_RESULT_KEY),
    );
    return commands;
  }

  private executeImmediate(
    gameState: GameState,
    eventManager: EventManager,
    overworld: FruitTreeOverworld,
    flag: string,
    rewardItem: string,
    audioEngine: AudioEngine | null,
  ): void {
    const runner = this.runner as FruitTreeRunner | undefined;
    openText(eventManager);
    const bearingText = resolveText(runner ?? null, overworld, "FruitBearingTreeText");
    showText(eventManager, bearingText);
    waitForPlayerAck(runner, eventManager, overworld);

    if (gameState.wram.event_flags[flag]) {
      const nothingText = resolveText(runner ?? null, overworld, "NothingHereText");
      showText(eventManager, nothingText);
      waitForPlayerAck(runner, eventManager, overworld);
      closeText(eventManager);
      if (runner) {
        runner.last_condition_result = false;
      }
      return;
    }

    const heyText = resolveText(runner ?? null, overworld, "HeyItsFruitText");
    showText(eventManager, heyText);
    waitForPlayerAck(runner, eventManager, overworld);

    const itemSystem = resolveItemSystem(runner, overworld);
    const success = addItemToBag(gameState, itemSystem, rewardItem);
    let runnerResult = false;
    if (success) {
      const obtainedText = resolveText(runner ?? null, overworld, "ObtainedFruitText");
      showText(eventManager, obtainedText);
      waitForPlayerAck(runner, eventManager, overworld);
      if (audioEngine?.play_sound) {
        audioEngine.play_sound(FRUIT_TREE_SFX);
      } else if (audioEngine?.playSound) {
        audioEngine.playSound(FRUIT_TREE_SFX);
      }
      if (runner) {
        runner.last_sound_effect = FRUIT_TREE_SFX;
      }
      if (runner?.set_event_flag) {
        runner.set_event_flag(flag, true);
      } else {
        applyEventFlag(gameState, flag, { value: true, overworld });
      }
      runnerResult = true;
    } else {
      const packFullText = resolveText(runner ?? null, overworld, "FruitPackIsFullText");
      showText(eventManager, packFullText);
      waitForPlayerAck(runner, eventManager, overworld);
    }

    closeText(eventManager);
    if (runner) {
      runner.last_condition_result = runnerResult;
    }
  }
}
