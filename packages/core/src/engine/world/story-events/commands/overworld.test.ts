import { GB_FRAME_DURATION_MS } from "@pokecrystal/core/core/gb-timing";
import { CryCommand, GetMonNameCommand, LoadMenuCommand, PauseCommand, PlaySoundCommand, VerticalMenuCommand, WaitSFXCommand } from "./overworld";

type RunnerStub = {
  pause: jest.Mock;
  resume: jest.Mock;
  last_sound_effect?: string | null;
};

const buildRunner = (): RunnerStub => ({
  pause: jest.fn(),
  resume: jest.fn(),
  last_sound_effect: "SFX_TEST",
});

class TestAudioEngine {
  public activeSounds = new Map<string, number>();

  isSoundPlaying(): boolean {
    return this.activeSounds.size > 0;
  }
}

const runWaitSfx = (audioEngine: unknown): RunnerStub => {
  const command = new WaitSFXCommand();
  const runner = buildRunner();
  command.runner = runner as any;
  command.execute({} as any, {} as any, { audio_engine: audioEngine } as any);
  return runner;
};

describe("WaitSFXCommand", () => {
  it("binds isSoundPlaying to the audio engine instance", () => {
    jest.useFakeTimers();
    jest.setSystemTime(0);
    const pollMs = Math.ceil(GB_FRAME_DURATION_MS);

    const runner = runWaitSfx(new TestAudioEngine());
    expect(runner.pause).toHaveBeenCalledTimes(1);

    jest.advanceTimersByTime(pollMs);
    expect(runner.resume).toHaveBeenCalledTimes(1);

    jest.useRealTimers();
  });

  it("waits until all SFX have finished", () => {
    jest.useFakeTimers();
    const pollMs = Math.ceil(GB_FRAME_DURATION_MS);
    const engine = {
      isSoundPlaying: jest.fn((...args: unknown[]) => {
        expect(args.length).toBe(0);
        return false;
      }),
    };
    const command = new WaitSFXCommand();
    const runner = buildRunner();
    command.runner = runner as any;
    command.execute({} as any, {} as any, { audio_engine: engine } as any);

    expect(runner.pause).toHaveBeenCalledTimes(1);
    jest.advanceTimersByTime(pollMs);
    expect(runner.resume).toHaveBeenCalledTimes(1);

    jest.useRealTimers();
  });
});

describe("PauseCommand", () => {
  it("converts pause frames using Game Boy frame timing", () => {
    jest.useFakeTimers();
    const runner = buildRunner();
    const setTimeoutSpy = jest.spyOn(global, "setTimeout");
    const command = new PauseCommand(120);
    command.runner = runner as any;

    command.execute({ wram: { instant_mode: false } } as any, {} as any, {} as any);

    expect(runner.pause).toHaveBeenCalledTimes(1);
    const scheduledDelay = setTimeoutSpy.mock.calls.at(-1)?.[1];
    expect(typeof scheduledDelay).toBe("number");
    // ASM: Script_pause waits 2 frames per counter decrement.
    expect(Number(scheduledDelay)).toBeCloseTo(120 * 2 * GB_FRAME_DURATION_MS, 5);

    setTimeoutSpy.mockRestore();
    jest.useRealTimers();
  });
});

describe("VerticalMenuCommand", () => {
  it("restores menu options from the preceding loadmenu when runtime variables were not restored", () => {
    const command = new VerticalMenuCommand();
    const runner = {
      pause: jest.fn(),
      resume: jest.fn(),
      variables: {},
      _script_stack: [{ name: "GoldenrodGameCornerTMVendor_LoopScript", index: 3 }],
      _consume_script_choice: jest.fn(() => 2),
      dataLoader: {
        get_script: jest.fn((label: string) => {
          if (label === "GoldenrodGameCornerTMVendor_LoopScript") {
            return [
              { command: "special", args: ["DisplayCoinCaseBalance"] },
              { command: "loadmenu", args: ["GoldenrodGameCornerTMVendorMenuHeader"] },
              { command: "verticalmenu", args: [] },
            ];
          }
          if (label === "GoldenrodGameCornerTMVendorMenuHeader") {
            return [
              { command: "db", args: ["\"TM25    5500@\""] },
              { command: "db", args: ["\"TM14    5500@\""] },
              { command: "db", args: ["\"TM38    5500@\""] },
              { command: "db", args: ["\"CANCEL@\""] },
            ];
          }
          return null;
        }),
      },
    };
    command.runner = runner as any;
    const gameState = { wram: { script_memory: {} } };

    expect(() => command.execute(gameState as any, {} as any, {} as any)).not.toThrow();

    expect(gameState.wram.script_memory.wScriptVar).toBe(2);
    expect(runner.variables).toEqual({
      _loaded_menu: {
        label: "GoldenrodGameCornerTMVendorMenuHeader",
        options: ["TM25    5500", "TM14    5500", "TM38    5500", "CANCEL"],
      },
    });
  });

  it("restores relative menu headers from the active script parent", () => {
    const command = new VerticalMenuCommand();
    const runner = {
      pause: jest.fn(),
      resume: jest.fn(),
      variables: {},
      _script_stack: [{ name: "GoldenrodGameCornerPrizeMonVendorScript", index: 9 }],
      _consume_script_choice: jest.fn(() => 1),
      dataLoader: {
        get_script: jest.fn((label: string, parent?: string) => {
          if (label === "GoldenrodGameCornerPrizeMonVendorScript") {
            return [
              { command: "faceplayer", args: [] },
              { command: "opentext", args: [] },
              { command: "writetext", args: ["GoldenrodGameCornerPrizeVendorIntroText"] },
              { command: "waitbutton", args: [] },
              { command: "checkitem", args: ["COIN_CASE"] },
              { command: "iffalse", args: ["GoldenrodGameCornerPrizeVendor_NoCoinCaseScript"] },
              { command: "writetext", args: ["GoldenrodGameCornerPrizeVendorWhichPrizeText"] },
              { command: "loadmenu", args: [".MenuHeader"] },
              { command: "verticalmenu", args: [] },
            ];
          }
          if (label === ".MenuHeader" && parent === "GoldenrodGameCornerPrizeMonVendorScript") {
            return [
              { command: "db", args: ["MENU_BACKUP_TILES"] },
              { command: "menu_coords", args: ["0", "2", "17", "TEXTBOX_Y - 1"] },
              { command: "dw", args: [".MenuData"] },
              { command: "db", args: ["1"] },
            ];
          }
          if (label === ".MenuData" && parent === "GoldenrodGameCornerPrizeMonVendorScript") {
            return [
              { command: "db", args: ["STATICMENU_CURSOR"] },
              { command: "db", args: ["4"] },
              { command: "db", args: ["\"ABRA        100@\""] },
              { command: "db", args: ["\"CUBONE      800@\""] },
              { command: "db", args: ["\"WOBBUFFET  1500@\""] },
              { command: "db", args: ["\"CANCEL@\""] },
            ];
          }
          return null;
        }),
      },
    };
    command.runner = runner as any;
    const gameState = { wram: { script_memory: {} } };

    expect(() => command.execute(gameState as any, {} as any, {} as any)).not.toThrow();

    expect(gameState.wram.script_memory.wScriptVar).toBe(1);
    expect(runner.variables).toEqual({
      _loaded_menu: {
        label: ".MenuHeader",
        options: ["ABRA        100", "CUBONE      800", "WOBBUFFET  1500", "CANCEL"],
      },
    });
  });

  it("loads relative menu headers from a local script frame parent", () => {
    const command = new LoadMenuCommand(".MenuHeader");
    const runner = {
      pause: jest.fn(),
      resume: jest.fn(),
      variables: {},
      _script_stack: [{ name: ".loop", parent: "GoldenrodGameCornerPrizeMonVendorScript", index: 3 }],
      dataLoader: {
        get_script: jest.fn((label: string, parent?: string) => {
          if (label === ".MenuHeader" && parent === "GoldenrodGameCornerPrizeMonVendorScript") {
            return [
              { command: "db", args: ["MENU_BACKUP_TILES"] },
              { command: "menu_coords", args: ["0", "2", "17", "TEXTBOX_Y - 1"] },
              { command: "dw", args: [".MenuData"] },
              { command: "db", args: ["1"] },
            ];
          }
          if (label === ".MenuData" && parent === "GoldenrodGameCornerPrizeMonVendorScript") {
            return [
              { command: "db", args: ["STATICMENU_CURSOR"] },
              { command: "db", args: ["4"] },
              { command: "db", args: ["\"ABRA        100@\""] },
              { command: "db", args: ["\"CUBONE      800@\""] },
              { command: "db", args: ["\"WOBBUFFET  1500@\""] },
              { command: "db", args: ["\"CANCEL@\""] },
            ];
          }
          return null;
        }),
      },
    };
    command.runner = runner as any;

    expect(() => command.execute({} as any, {} as any, {} as any)).not.toThrow();

    expect(runner.variables).toEqual({
      _loaded_menu: {
        label: ".MenuHeader",
        options: ["ABRA        100", "CUBONE      800", "WOBBUFFET  1500", "CANCEL"],
      },
    });
  });

  it("restores relative menu headers from local script history after autosave restore", () => {
    const command = new VerticalMenuCommand();
    const runner = {
      pause: jest.fn(),
      resume: jest.fn(),
      variables: {},
      _script_stack: [{ name: ".loop", parent: "GoldenrodGameCornerPrizeMonVendorScript", index: 4 }],
      _consume_script_choice: jest.fn(() => 3),
      dataLoader: {
        get_script: jest.fn((label: string, parent?: string) => {
          if (label === ".loop" && parent === "GoldenrodGameCornerPrizeMonVendorScript") {
            return [
              { command: "writetext", args: ["GoldenrodGameCornerPrizeVendorWhichPrizeText"] },
              { command: "special", args: ["DisplayCoinCaseBalance"] },
              { command: "loadmenu", args: [".MenuHeader"] },
              { command: "verticalmenu", args: [] },
            ];
          }
          if (label === ".MenuHeader" && parent === "GoldenrodGameCornerPrizeMonVendorScript") {
            return [
              { command: "db", args: ["MENU_BACKUP_TILES"] },
              { command: "menu_coords", args: ["0", "2", "17", "TEXTBOX_Y - 1"] },
              { command: "dw", args: [".MenuData"] },
              { command: "db", args: ["1"] },
            ];
          }
          if (label === ".MenuData" && parent === "GoldenrodGameCornerPrizeMonVendorScript") {
            return [
              { command: "db", args: ["STATICMENU_CURSOR"] },
              { command: "db", args: ["4"] },
              { command: "db", args: ["\"ABRA        100@\""] },
              { command: "db", args: ["\"CUBONE      800@\""] },
              { command: "db", args: ["\"WOBBUFFET  1500@\""] },
              { command: "db", args: ["\"CANCEL@\""] },
            ];
          }
          return null;
        }),
      },
    };
    command.runner = runner as any;
    const gameState = { wram: { script_memory: {} } };

    expect(() => command.execute(gameState as any, {} as any, {} as any)).not.toThrow();

    expect(gameState.wram.script_memory.wScriptVar).toBe(3);
    expect(runner.variables).toEqual({
      _loaded_menu: {
        label: ".MenuHeader",
        options: ["ABRA        100", "CUBONE      800", "WOBBUFFET  1500", "CANCEL"],
      },
    });
  });
});

describe("PlaySoundCommand", () => {
  it("skips audio playback in instant mode while keeping script metadata", () => {
    const audioEngine = {
      playSound: jest.fn(),
    };
    const command = new PlaySoundCommand("SFX_ITEM");
    const runner = buildRunner();
    command.runner = runner as any;

    command.execute({ wram: { instant_mode: true } } as any, {} as any, { audio_engine: audioEngine } as any);

    expect(runner.last_sound_effect).toBe("SFX_ITEM");
    expect(audioEngine.playSound).not.toHaveBeenCalled();
  });
});

describe("CryCommand", () => {
  it("skips cry playback in instant mode while recording the resolved cry id", () => {
    const audioEngine = {
      playSound: jest.fn(),
    };
    const command = new CryCommand("PIKACHU");
    const runner = buildRunner();
    command.runner = runner as any;

    command.execute(
      { wram: { instant_mode: true } } as any,
      {} as any,
      { audio_engine: audioEngine, data_loader: { get_pokemon_cry: () => ({ cry_id: "CRY_PIKACHU" }) } } as any
    );

    expect(runner.last_sound_effect).toBe("CRY_PIKACHU");
    expect(audioEngine.playSound).not.toHaveBeenCalled();
  });
});

describe("GetMonNameCommand", () => {
  it("resolves the real Pokemon name instead of prettifying the symbol", () => {
    const command = new GetMonNameCommand("STRING_BUFFER_3", "MR__MIME");
    const runner = { string_buffers: {}, dataLoader: { get_pokemon: () => ({ name: "Mr. Mime" }) } };
    command.runner = runner as any;

    command.execute({} as any, {} as any, {} as any);

    expect(runner.string_buffers).toEqual({ STRING_BUFFER_3: "Mr. Mime" });
  });

  it("throws when Pokemon data cannot resolve the ASM species name", () => {
    const command = new GetMonNameCommand("STRING_BUFFER_3", "MR__MIME");
    const runner = { string_buffers: {}, dataLoader: { get_pokemon: () => null } };
    command.runner = runner as any;

    expect(() => command.execute({} as any, {} as any, {} as any)).toThrow(
      "Missing ASM Pokemon name for 'MR__MIME'.",
    );
  });
});
