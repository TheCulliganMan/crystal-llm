import { createInitialGameState } from "@pokecrystal/core/core/state";
import { Event, EventManager } from "@pokecrystal/core/engine/events/events";
import { Surface } from "@pokecrystal/core/ui/surface";
import { KEYS } from "@pokecrystal/core/core/keycodes";
import { SCREEN_TILE_WIDTH, TEXTBOX_HEIGHT_TILES, TEXTBOX_Y_TILES } from "@pokecrystal/core/core/text-constants";
import { TILE_SIZE } from "@pokecrystal/core/engine/world/tile";
import { Z_INDEX_PROMPT } from "@pokecrystal/core/ui/z-index";
import { B_PAD_A, B_PAD_DOWN } from "@pokecrystal/core/input/controls";
import { DialogueWindow, FieldDialogueManager, YesNoPrompt } from "./dialogue";

type DialogueSnapshot = {
  viewportLines: string[];
  infoLines: string[];
  viewportTitle: string;
  infoTitle: string;
  menuLines: string[] | null;
  promptLines: string[] | null;
  dialogueLines: string[] | null;
};

describe("FieldDialogueManager wait_for_input", () => {
  const buildDialogue = () => {
    const gameState = createInitialGameState();
    const pause = jest.fn();
    const runner = {
      pause,
      _script_stack: [{}],
      _awaiting_resume: 0,
    };
    const ui = {
      screen: new Surface(160, 144),
      font: { renderText: jest.fn() },
      drawTextBox: jest.fn(),
      draw_window: jest.fn(),
    };
    const dialogue = new FieldDialogueManager(ui, gameState, runner);
    return { dialogue, gameState, pause };
  };

  const buildDialogueWithEvents = () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const pause = jest.fn();
    const resume = jest.fn();
    const runner = {
      pause,
      resume,
      _script_stack: [{}],
      _awaiting_resume: 1,
      event_manager: eventManager,
    };
    const ui = {
      screen: new Surface(160, 144),
      font: { renderText: jest.fn() },
      drawTextBox: jest.fn(),
      draw_window: jest.fn(),
    };
    const dialogue = new FieldDialogueManager(ui, gameState, runner);
    const eventNames = ["open_text", "close_text", "show_text", "wait_for_input", "prompt_yes_no"];
    for (const eventName of eventNames) {
      eventManager.on(eventName, dialogue.handle_event.bind(dialogue));
    }
    return { dialogue, gameState, eventManager, pause, resume };
  };

  it("pauses the runner when pauseRunner is set", () => {
    const { dialogue, gameState, pause } = buildDialogue();

    dialogue.handle_event(new Event("wait_for_input", { pauseRunner: true }), gameState);

    expect(pause).toHaveBeenCalledTimes(1);
  });

  it("formats raw show_text payloads before they reach the textbox renderer", () => {
    const gameState = createInitialGameState();
    const runner = {
      formatText: jest.fn((text: string) => text.replace("{d:ROUTE43GATE_TOLL}", "1000")),
      _script_stack: [{}],
      _awaiting_resume: 0,
    };
    const drawTextBox = jest.fn();
    const ui = {
      screen: new Surface(160, 144),
      font: { renderText: jest.fn() },
      drawTextBox,
      draw_window: jest.fn(),
    };
    const dialogue = new FieldDialogueManager(ui, gameState, runner);

    dialogue.handle_event(new Event("show_text", { text: "The toll is ¥{d:ROUTE43GATE_TOLL}" }), gameState);

    expect(runner.formatText).toHaveBeenCalledWith("The toll is ¥{d:ROUTE43GATE_TOLL}");
    expect((dialogue as unknown as { current_text: string }).current_text).toBe("The toll is ¥1000");
    expect(drawTextBox).not.toHaveBeenCalled();
  });

  it("pauses the runner when pause_runner is set", () => {
    const { dialogue, gameState, pause } = buildDialogue();

    dialogue.handle_event(new Event("wait_for_input", { pause_runner: true }), gameState);

    expect(pause).toHaveBeenCalledTimes(1);
  });

  it("records waits and resumes only after input is acknowledged", () => {
    const { dialogue, eventManager, pause, resume } = buildDialogueWithEvents();

    eventManager.dispatch(new Event("open_text", {}));
    eventManager.dispatch(new Event("show_text", { text: "MART SIGN" }));
    eventManager.dispatch(new Event("wait_for_input", { pauseRunner: true }));
    eventManager.dispatch(new Event("close_text", {}));

    expect(pause).toHaveBeenCalledTimes(1);
    expect(dialogue.visible).toBe(true);
    expect(dialogue.waiting_for_input).toBe(true);
    expect(dialogue.pending_waits).toBe(1);
    expect(dialogue.is_script_paused).toBe(true);

    dialogue.handle_input({ type: KEYS.KEYUP, key: KEYS.Z });
    dialogue.handle_input({ type: KEYS.KEYDOWN, key: KEYS.Z });

    expect(dialogue.waiting_for_input).toBe(false);
    expect(dialogue.visible).toBe(false);
    expect(dialogue.pending_waits).toBe(0);
    expect(dialogue.pending_script_waits_count).toBe(0);
    expect(resume).toHaveBeenCalledTimes(1);
  });

  it("auto-closes after wait when show_text uses autoCloseAfterWait", () => {
    const { dialogue, eventManager } = buildDialogueWithEvents();

    eventManager.dispatch(new Event("open_text", {}));
    eventManager.dispatch(new Event("show_text", { text: "MART SIGN", autoCloseAfterWait: true }));
    eventManager.dispatch(new Event("wait_for_input", { pauseRunner: false }));

    (dialogue as unknown as { window: DialogueWindow }).window.complete();
    dialogue.handle_input({ type: KEYS.KEYUP, key: KEYS.Z });
    dialogue.handle_input({ type: KEYS.KEYDOWN, key: KEYS.Z });

    expect(dialogue.waiting_for_input).toBe(false);
    expect(dialogue.visible).toBe(false);
    expect(dialogue.pending_waits).toBe(0);
  });

  it("auto-closes deferred close_text once the final page is complete", () => {
    const { dialogue, eventManager } = buildDialogueWithEvents();

    eventManager.dispatch(new Event("open_text", {}));
    eventManager.dispatch(new Event("show_text", { text: "There are only two of us, so we're always busy." }));
    eventManager.dispatch(new Event("close_text", {}));

    const window = (dialogue as unknown as { window: DialogueWindow }).window;
    window.complete();
    dialogue.update();

    expect(dialogue.visible).toBe(false);
    expect(dialogue.waiting_for_input).toBe(false);
  });

  it("marks wait_for_input as pending even without open_text", () => {
    const { dialogue, eventManager } = buildDialogueWithEvents();

    eventManager.dispatch(new Event("wait_for_input", { pauseRunner: false }));

    expect(dialogue.waiting_for_input).toBe(true);
    expect(dialogue.pending_waits).toBe(1);
    expect(dialogue.is_script_paused).toBe(true);
  });

  it("resumes yes/no prompts even when the script stack is empty", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const resume = jest.fn();
    const runner = {
      pause: jest.fn(),
      resume,
      _script_stack: [],
      _awaiting_resume: 1,
      event_manager: eventManager,
    };
    const ui = {
      screen: new Surface(160, 144),
      font: { renderText: jest.fn() },
      drawTextBox: jest.fn(),
      draw_window: jest.fn(),
    };
    const dialogue = new FieldDialogueManager(ui, gameState, runner);
    const eventNames = ["open_text", "close_text", "show_text", "wait_for_input", "prompt_yes_no"];
    for (const eventName of eventNames) {
      eventManager.on(eventName, dialogue.handle_event.bind(dialogue));
    }

    eventManager.dispatch(new Event("open_text", {}));
    eventManager.dispatch(new Event("show_text", { text: "Elm needs help." }));
    eventManager.dispatch(new Event("wait_for_input", { pauseRunner: true }));
    eventManager.dispatch(new Event("prompt_yes_no", {}));

    (dialogue as unknown as { window: DialogueWindow }).window.complete();
    dialogue.update();
    dialogue.handle_input({ type: KEYS.KEYUP, key: KEYS.Z });
    dialogue.handle_input({ type: KEYS.KEYDOWN, key: KEYS.Z });

    expect(dialogue.pending_script_waits_count).toBe(0);
    expect(resume).toHaveBeenCalledTimes(1);
  });

  it("accepts a confirm press after text finishes loading", () => {
    const { dialogue, eventManager, resume } = buildDialogueWithEvents();

    eventManager.dispatch(new Event("open_text", {}));
    eventManager.dispatch(new Event("show_text", { text: "Elm needs help." }));
    eventManager.dispatch(new Event("wait_for_input", { pauseRunner: true }));
    eventManager.dispatch(new Event("prompt_yes_no", {}));

    dialogue.handle_input({ type: KEYS.KEYDOWN, key: KEYS.Z });
    dialogue.handle_input({ type: KEYS.KEYUP, key: KEYS.Z });

    (dialogue as unknown as { window: DialogueWindow }).window.complete();
    dialogue.update();

    dialogue.handle_input({ type: KEYS.KEYDOWN, key: KEYS.Z });

    expect(resume).toHaveBeenCalledTimes(1);
  });

  it("surfaces the yes/no prompt on the first confirm after text completes", () => {
    const { dialogue, eventManager, resume } = buildDialogueWithEvents();

    eventManager.dispatch(new Event("open_text", {}));
    eventManager.dispatch(new Event("show_text", { text: "Will you help me?" }));
    eventManager.dispatch(new Event("wait_for_input", { pauseRunner: true }));
    eventManager.dispatch(new Event("prompt_yes_no", {}));

    const window = (dialogue as unknown as { window: DialogueWindow }).window;

    dialogue.handle_input({ type: KEYS.KEYDOWN, key: KEYS.Z });
    dialogue.handle_input({ type: KEYS.KEYUP, key: KEYS.Z });

    dialogue.handle_input({ type: KEYS.KEYDOWN, key: KEYS.Z });
    dialogue.handle_input({ type: KEYS.KEYUP, key: KEYS.Z });

    expect(window.is_complete()).toBe(true);
    expect(dialogue.pending_waits).toBe(0);
    expect(dialogue.waiting_for_input).toBe(true);
    expect(dialogue._yes_no_prompt).not.toBeNull();
    expect(resume).toHaveBeenCalledTimes(1);

    dialogue.handle_input({ type: KEYS.KEYDOWN, key: KEYS.Z });

    expect(resume).toHaveBeenCalledTimes(2);
    expect(dialogue._yes_no_prompt).toBeNull();
  });

  it("includes recent dialogue lines when yes/no prompts are drawn", () => {
    const gameState = createInitialGameState();
    const runner = {
      pause: jest.fn(),
      _script_stack: [{}],
      _awaiting_resume: 0,
    };
    let snapshot: DialogueSnapshot = {
      viewportLines: ["OVERWORLD"],
      infoLines: ["LEGEND LINE"],
      viewportTitle: "Overworld",
      infoTitle: "Legend",
      menuLines: ["MENU"],
      promptLines: null as string[] | null,
      dialogueLines: ["Do it?"],
    };
    const ui = {
      screen: new Surface(160, 144),
      font: { renderText: jest.fn() },
      drawTextBox: jest.fn(),
      draw_window: jest.fn(),
      renderSnapshot: (
        viewportLines: string[],
        infoLines: string[],
        viewportTitle: string,
        infoTitle: string,
        menuLines: string[] | null,
        promptLines: string[] | null,
        dialogueLines: string[] | null
      ) => {
        snapshot = {
          viewportLines,
          infoLines,
          viewportTitle,
          infoTitle,
          menuLines,
          promptLines,
          dialogueLines,
        };
      },
      getSnapshot: () => snapshot,
    };
    const dialogue = new FieldDialogueManager(ui, gameState, runner);

    dialogue.handle_event(new Event("open_text", {}), gameState);
    dialogue.handle_event(new Event("show_text", { text: "Answer me." }), gameState);
    dialogue.handle_event(new Event("wait_for_input", { pauseRunner: true }), gameState);
    dialogue.handle_event(new Event("prompt_yes_no", {}), gameState);

    (dialogue as unknown as { window: DialogueWindow }).window.complete();
    dialogue.update();
    dialogue.draw();

    expect(snapshot.viewportLines).toEqual(["Prompt"]);
    expect(snapshot.viewportTitle).toBe("Prompt");
    expect(snapshot.infoLines).toEqual(["Up/Down=Choose A=OK B=Cancel"]);
    expect(snapshot.infoTitle).toBe("Legend");
    expect(snapshot.menuLines).toEqual(["MENU"]);
    expect(snapshot.promptLines).toEqual(["Do it?", "▶ YES", "  NO"]);
    expect(snapshot.dialogueLines).toBeNull();
  });

  it("omits status lines when building yes/no prompt context", () => {
    const gameState = createInitialGameState();
    const runner = {
      pause: jest.fn(),
      _script_stack: [{}],
      _awaiting_resume: 0,
    };
    let snapshot: DialogueSnapshot = {
      viewportLines: ["OVERWORLD"],
      infoLines: ["LEGEND LINE"],
      viewportTitle: "Overworld",
      infoTitle: "Legend",
      menuLines: ["MENU"],
      promptLines: null as string[] | null,
      dialogueLines: ["Do it?", "Text queue: 0 (press A to advance)"],
    };
    const ui = {
      screen: new Surface(160, 144),
      font: { renderText: jest.fn() },
      drawTextBox: jest.fn(),
      draw_window: jest.fn(),
      renderSnapshot: (
        viewportLines: string[],
        infoLines: string[],
        viewportTitle: string,
        infoTitle: string,
        menuLines: string[] | null,
        promptLines: string[] | null,
        dialogueLines: string[] | null
      ) => {
        snapshot = {
          viewportLines,
          infoLines,
          viewportTitle,
          infoTitle,
          menuLines,
          promptLines,
          dialogueLines,
        };
      },
      getSnapshot: () => snapshot,
    };
    const dialogue = new FieldDialogueManager(ui, gameState, runner);

    dialogue.handle_event(new Event("open_text", {}), gameState);
    dialogue.handle_event(new Event("show_text", { text: "Answer me." }), gameState);
    dialogue.handle_event(new Event("wait_for_input", { pauseRunner: true }), gameState);
    dialogue.handle_event(new Event("prompt_yes_no", {}), gameState);

    (dialogue as unknown as { window: DialogueWindow }).window.complete();
    dialogue.update();
    dialogue.draw();

    expect(snapshot.promptLines).toEqual(["Do it?", "▶ YES", "  NO"]);
    expect(snapshot.dialogueLines).toBeNull();
  });
});

describe("DialogueWindow pagination", () => {
  it("maps frame type options to render ids", () => {
    const gameState = createInitialGameState();
    gameState.sram.options.frame = 1;
    const drawTextBox = jest.fn();
    const ui = {
      screen: new Surface(160, 144),
      font: { renderText: jest.fn(), charWidth: 8 },
      drawTextBox,
      default_frame_id: 1,
    };
    const window = new DialogueWindow(ui, gameState, 2);

    window.open("HI");
    window.complete();
    window.draw();

    expect(drawTextBox).toHaveBeenCalledWith(
      ui.screen,
      "HI",
      0,
      TEXTBOX_Y_TILES,
      SCREEN_TILE_WIDTH,
      TEXTBOX_HEIGHT_TILES,
      2,
      undefined,
      undefined,
      expect.any(Number),
    );
  });

  it("falls back to wrapping when the font lacks wrapText", () => {
    const gameState = createInitialGameState();
    const ui = {
      screen: new Surface(160, 144),
      font: { renderText: jest.fn(), charWidth: 8 },
    };
    const window = new DialogueWindow(ui, gameState, 2);

    window.open("Wild HOPPIP appeared!");
    window.complete();

    expect(window.visible_text).toContain("HOPPIP");
  });

  it("falls back to internal wrapping when wrapText returns empty output", () => {
    const gameState = createInitialGameState();
    const ui = {
      screen: new Surface(160, 144),
      font: { renderText: jest.fn(), charWidth: 8, wrapText: jest.fn(() => []) },
    };
    const window = new DialogueWindow(ui, gameState, 2);

    window.open("Wild HOPPIP appeared!");
    window.complete();

    expect(window.visible_text).toContain("HOPPIP");
  });

  it("preserves explicit line breaks from input text", () => {
    const gameState = createInitialGameState();
    const ui = {
      screen: new Surface(160, 144),
      font: { renderText: jest.fn(), charWidth: 8 },
    };
    const window = new DialogueWindow(ui, gameState, 2);

    window.open("LINE ONE\nLINE TWO");
    window.complete();

    expect(window.visible_text).toBe("LINE ONE\nLINE TWO");
  });

  it("does not collapse later pages into the first textbox on snapshot-capable UIs", () => {
    const gameState = createInitialGameState();
    const drawTextBox = jest.fn();
    const snapshot = {
      viewportLines: ["ROUTE 30"],
      infoLines: ["INFO"],
      dialogueLines: null,
    };
    const ui = {
      screen: new Surface(160, 144),
      font: { renderText: jest.fn(), charWidth: 8 },
      drawTextBox,
      renderSnapshot: jest.fn(),
      getSnapshot: () => snapshot,
    };
    const window = new DialogueWindow(ui, gameState, 2);

    window.open("TRAINER TIPS\nNo stealing other\npeople's #MON!");
    window.complete();
    window.draw();

    expect(window.total_pages).toBe(2);
    expect(window.page_index).toBe(0);
    expect(window.visible_text).toBe("TRAINER TIPS\nNo stealing other");
    expect(drawTextBox).toHaveBeenCalledWith(
      ui.screen,
      "TRAINER TIPS\nNo stealing other",
      0,
      TEXTBOX_Y_TILES,
      SCREEN_TILE_WIDTH,
      TEXTBOX_HEIGHT_TILES,
      expect.any(Number),
      undefined,
      undefined,
      expect.any(Number),
    );
  });
});

describe("DialogueWindow text audio parity", () => {
  it("does not play per-glyph audio while revealing ordinary dialogue text", () => {
    const gameState = createInitialGameState();
    const audio_engine = {
      playSound: jest.fn(),
    };
    const ui = {
      screen: new Surface(160, 144),
      font: { renderText: jest.fn(), charWidth: 8 },
      drawTextBox: jest.fn(),
    };
    const window = new DialogueWindow(ui, gameState, 2, audio_engine as never);

    window.open("HELLO");
    window.update();

    expect(window.visible_text).toBe("H");
    expect(audio_engine.playSound).not.toHaveBeenCalled();
  });
});

describe("DialogueWindow text masking", () => {
  it("replaces visible text with spaces when masking is active", () => {
    const gameState = createInitialGameState();
    const drawTextBox = jest.fn();
    const ui = {
      screen: new Surface(160, 144),
      font: { renderText: jest.fn(), charWidth: 8 },
      drawTextBox,
    };
    const window = new DialogueWindow(ui, gameState, 2, null, { mask_text: () => true });

    window.open("LINE\nTWO");
    window.complete();
    window.draw();

    expect(window.visible_text).toBe("    \n   ");
    expect(drawTextBox).toHaveBeenCalledWith(
      ui.screen,
      "    \n   ",
      0,
      TEXTBOX_Y_TILES,
      SCREEN_TILE_WIDTH,
      TEXTBOX_HEIGHT_TILES,
      expect.any(Number),
      undefined,
      undefined,
      expect.any(Number),
    );
  });
});

describe("FieldDialogueManager prompt cursor", () => {
  it("draws the prompt cursor when waiting for input and text is complete", () => {
    const gameState = createInitialGameState();
    gameState.frame_counter = 16;
    const renderText = jest.fn();
    const ui = {
      screen: new Surface(160, 144),
      font: { renderText },
      drawTextBox: jest.fn(),
      draw_window: jest.fn(),
    };
    const runner = { pause: jest.fn(), _script_stack: [{}], _awaiting_resume: 0 };
    const dialogue = new FieldDialogueManager(ui, gameState, runner);

    dialogue.handle_event(new Event("open_text", {}), gameState);
    dialogue.handle_event(new Event("show_text", { text: "Hi" }), gameState);
    dialogue.handle_event(new Event("wait_for_input", {}), gameState);
    (dialogue as unknown as { window: DialogueWindow }).window.complete();

    dialogue.draw();

    const expectedX = (SCREEN_TILE_WIDTH - 2) * TILE_SIZE;
    const expectedY = (TEXTBOX_Y_TILES + TEXTBOX_HEIGHT_TILES - 1) * TILE_SIZE;
    expect(renderText).toHaveBeenCalledWith(
      "▼",
      expectedX,
      expectedY,
      ui.screen,
      expect.objectContaining({ max_lines: 1 })
    );
  });

  it("skips the prompt cursor during the blink-off phase", () => {
    const gameState = createInitialGameState();
    gameState.frame_counter = 0;
    const renderText = jest.fn();
    const ui = {
      screen: new Surface(160, 144),
      font: { renderText },
      drawTextBox: jest.fn(),
      draw_window: jest.fn(),
    };
    const runner = { pause: jest.fn(), _script_stack: [{}], _awaiting_resume: 0 };
    const dialogue = new FieldDialogueManager(ui, gameState, runner);

    dialogue.handle_event(new Event("open_text", {}), gameState);
    dialogue.handle_event(new Event("show_text", { text: "Hi" }), gameState);
    dialogue.handle_event(new Event("wait_for_input", {}), gameState);
    (dialogue as unknown as { window: DialogueWindow }).window.complete();

    dialogue.draw();

    expect(renderText).not.toHaveBeenCalled();
  });
});

describe("YesNoPrompt placement", () => {
  it("keeps yes/no cursor movement silent and only clicks on confirm", () => {
    const ui = {
      screen: new Surface(160, 144),
      font: { renderText: jest.fn() },
      draw_window: jest.fn(),
      _record_window_region: jest.fn(),
    };
    const audioEngine = { playSound: jest.fn() };
    const prompt = new YesNoPrompt(ui, audioEngine as any);

    prompt.handle_joypad(B_PAD_DOWN);
    expect(audioEngine.playSound).not.toHaveBeenCalled();

    prompt.handle_joypad(B_PAD_A);
    expect(audioEngine.playSound).toHaveBeenCalledWith("menu_option");
  });

  it("accepts direct player direction and button events", () => {
    const ui = {
      screen: new Surface(160, 144),
      font: { renderText: jest.fn() },
      draw_window: jest.fn(),
      _record_window_region: jest.fn(),
    };
    const prompt = new YesNoPrompt(ui, null);

    prompt.handle_input({ type: "keydown", direction: "down", is_press: true } as any);
    prompt.handle_input({ type: "keydown", button: "a", is_press: true } as any);

    expect(prompt.selection).toBe(1);
    expect(prompt.finished).toBe(true);
  });

  it("records the prompt window at the ASM yes/no box coordinates", () => {
    const ui = {
      screen: new Surface(160, 144),
      font: { renderText: jest.fn() },
      draw_window: jest.fn(),
      _record_window_region: jest.fn(),
    };
    const prompt = new YesNoPrompt(ui, null);

    prompt.draw();

    const expectedX = (SCREEN_TILE_WIDTH - 6) * TILE_SIZE;
    const expectedY = 7 * TILE_SIZE;
    expect(ui._record_window_region).toHaveBeenCalledWith(
      ui.screen,
      expectedX,
      expectedY,
      6,
      4,
      Z_INDEX_PROMPT,
      expect.any(Surface),
    );
  });
});
