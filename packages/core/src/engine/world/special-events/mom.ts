import { showText, waitForInput, Event, type EventManager } from "@pokecrystal/core/engine/events/events";
import { GameState } from "@pokecrystal/core/core/state";
import { resolveScriptText, type ScriptRunner } from "./utils";
import { SelectionPrompt } from "@pokecrystal/core/ui/text/prompts";
import type { Overworld } from "@pokecrystal/core/types/overworld";

type ScriptRunnerWithCommands = ScriptRunner & {
  command_map?: {
    yesno?: () => YesNoCommand;
  };
};

type YesNoCommand = {
  runner?: ScriptRunner | null;
  on_result?: (value: boolean) => void;
  execute: (
    gameState: GameState,
    eventManager: EventManager | null | undefined,
    overworld: Overworld | null | undefined
  ) => void;
};

type MomOverworld = Overworld & {
  ui?: unknown;
  draw?: () => void;
};

type PokemonCenterOwner = {
  pokemon_center?: {
    healParty?: () => void;
    playHealMachineAnimation?: (animationId: string | null, overworld: MomOverworld | null) => void;
  } | null;
};

type MomContext = {
  runner?: (ScriptRunnerWithCommands & PokemonCenterOwner) | null;
  overworld?: unknown;
  event_manager?: EventManager | null;
};

const _MOM_TEXT_FALLBACKS: Record<string, string> = {
  MomLeavingText1: "I'll save your money. Trust me!",
  MomLeavingText2: "Okay, I'll save a little for you.",
  MomLeavingText3: "Come home anytime!",
  MomIsThisAboutYourMoneyText: "Is this about your money?",
  MomBankWhatDoYouWantToDoText: "What do you want to do?",
  MomStoreMoneyText: "I'll hold on to your money.",
  MomTakeMoneyText: "You'd like some of your money?",
  MomSaveMoneyText: "I'll stop saving your money, dear.",
  MomStartSavingMoneyText: "I'll start saving again!",
  MomStoredMoneyText: "I'll save it for you.",
  MomTakenMoneyText: "Here you go!",
  MomHaventSavedThatMuchText: "I haven't saved that much yet!",
  MomInsufficientFundsInWalletText: "You don't have enough!",
  MomNotEnoughRoomInBankText: "I can't hold that much!",
  MomJustDoWhatYouCanText: "Just do what you can!",
};

const MOM_MENU_WINDOW_ORIGIN_TILES: [number, number] = [0, 0];
const MOM_MENU_WINDOW_MIN_WIDTH = 11;
const MOM_MENU_WINDOW_MIN_HEIGHT = 11;

function _queue_text(
  labels: string[],
  { runner, event_manager }: { runner?: ScriptRunner | null; event_manager?: EventManager | null }
) {
  if (!event_manager) {
    return;
  }
  for (const label of labels) {
    const text = resolveScriptText(label, { runner, defaultValue: _MOM_TEXT_FALLBACKS[label] ?? "" });
    if (!text) {
      continue;
    }
    showText(event_manager, text);
    waitForInput(event_manager);
  }
}

function _prompt_yes_no(
  game_state: GameState,
  event_manager: EventManager | null | undefined,
  runner: ScriptRunnerWithCommands | null | undefined,
  overworld: MomOverworld | null
) {
  const runnerWithCommands = runner as ScriptRunnerWithCommands | null;
  if (runnerWithCommands) {
    const commandFactory = runnerWithCommands.command_map?.yesno;
    if (commandFactory) {
      const command = commandFactory() as YesNoCommand;
      command.runner = runnerWithCommands;
      const resultHolder: boolean[] = [];
      command.on_result = (value: boolean) => {
        resultHolder.splice(0, resultHolder.length, Boolean(value));
      };
      if (event_manager) {
        command.execute(game_state, event_manager, overworld);
      }
      if (resultHolder.length) {
        return resultHolder[0];
      }
      return Boolean(runnerWithCommands.last_yes_no_result ?? false);
    }
  }

  if (!overworld?.ui) {
    return false;
  }

  const result = { value: Boolean(runner?.last_yes_no_result ?? false) };
  if (event_manager) {
    event_manager.dispatch(
      new Event("prompt_yes_no", {
        callback: (value: boolean) => {
          result.value = Boolean(value);
        },
      })
    );
  }
  return result.value;
}

async function _run_bank_menu(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: { runner?: (ScriptRunnerWithCommands & PokemonCenterOwner) | null; overworld?: unknown; event_manager?: EventManager | null }
): Promise<void> {
  const resolvedOverworld = (overworld ?? null) as MomOverworld | null;
  const ui = resolvedOverworld?.ui;
  const drawCallback = resolvedOverworld?.draw ? () => resolvedOverworld.draw!() : undefined;
  if (!ui || !drawCallback) {
    // Cannot present an interactive menu without a rendered UI.
    if (runner) {
      runner.last_condition_result = true;
    }
    return;
  }

  // ASM: engine/events/mom.asm::BankOfMom_MenuHeader (menu_coords 0, 0, 10, 10).
  const menu = new SelectionPrompt(ui as ConstructorParameters<typeof SelectionPrompt>[0], ["GET", "SAVE", "CHANGE", "CANCEL"], {
    windowOriginTiles: MOM_MENU_WINDOW_ORIGIN_TILES,
    windowMinWidth: MOM_MENU_WINDOW_MIN_WIDTH,
    windowMinHeight: MOM_MENU_WINDOW_MIN_HEIGHT,
    // Keep title blank to avoid introducing an ASM-visible header line, but preserve the
    // extra top spacing used by STATICMENU_CURSOR-only menus.
    title: " ",
  });
  const selection = await menu.runAsync({ drawCallback });

  if (selection === 0) {
    _queue_text(["MomTakeMoneyText"], { runner, event_manager });
    if (game_state.sram.moms_money <= 0) {
      _queue_text(["MomHaventSavedThatMuchText"], { runner, event_manager });
      return;
    }
    game_state.sram.money += game_state.sram.moms_money;
    game_state.sram.moms_money = 0;
    _queue_text(["MomTakenMoneyText"], { runner, event_manager });
  } else if (selection === 1) {
    _queue_text(["MomStoreMoneyText"], { runner, event_manager });
    if (game_state.sram.money <= 0) {
      _queue_text(["MomInsufficientFundsInWalletText"], { runner, event_manager });
      return;
    }
    game_state.sram.moms_money += game_state.sram.money;
    game_state.sram.money = 0;
    _queue_text(["MomStoredMoneyText"], { runner, event_manager });
  } else if (selection === 2) {
    game_state.sram.mom_saving_some_money = !game_state.sram.mom_saving_some_money;
    const label = game_state.sram.mom_saving_some_money
      ? "MomStartSavingMoneyText"
      : "MomSaveMoneyText";
    _queue_text([label], { runner, event_manager });
  } else {
    _queue_text(["MomJustDoWhatYouCanText"], { runner, event_manager });
  }
  if (runner) {
    runner.last_condition_result = true;
  }
}

export function bank_of_mom(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: MomContext = {}
): void | Promise<void> {
  // ASM: engine/events/mom.asm::BankOfMom
  const resolvedRunner = runner ?? null;
  let resolvedEventManager = event_manager ?? null;
  if (resolvedRunner && !resolvedEventManager) {
    resolvedEventManager = resolvedRunner.event_manager ?? null;
    if (!resolvedEventManager && "eventManager" in resolvedRunner) {
      const typed = resolvedRunner as { eventManager?: EventManager | null };
      resolvedEventManager = typed.eventManager ?? null;
    }
  }

  const resolvedOverworld = (overworld ?? null) as MomOverworld | null;
  if (!resolvedEventManager) {
    game_state.sram.mom_saving_active = true;
    return;
  }

  if (!game_state.sram.mom_saving_active) {
    const yes = _prompt_yes_no(game_state, resolvedEventManager, resolvedRunner, resolvedOverworld);
    if (yes) {
      game_state.sram.mom_saving_some_money = true;
      _queue_text(["MomLeavingText1", "MomLeavingText2", "MomLeavingText3"], {
        runner: resolvedRunner,
        event_manager: resolvedEventManager,
      });
    } else {
      game_state.sram.mom_saving_some_money = false;
      _queue_text(["MomLeavingText1", "MomLeavingText3"], {
        runner: resolvedRunner,
        event_manager: resolvedEventManager,
      });
    }
    game_state.sram.mom_saving_active = true;
    return;
  }

  const wants_money = _prompt_yes_no(game_state, resolvedEventManager, resolvedRunner, resolvedOverworld);
  if (!wants_money) {
    _queue_text(["MomJustDoWhatYouCanText"], {
      runner: resolvedRunner,
      event_manager: resolvedEventManager,
    });
    return;
  }

  _queue_text(["MomBankWhatDoYouWantToDoText"], {
    runner: resolvedRunner,
    event_manager: resolvedEventManager,
  });
  return _run_bank_menu(game_state, {
    runner: resolvedRunner,
    overworld: resolvedOverworld,
    event_manager: resolvedEventManager,
  });
}
