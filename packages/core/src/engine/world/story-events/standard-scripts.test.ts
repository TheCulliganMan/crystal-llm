import { createInitialGameState } from "@pokecrystal/core/core/state";
import { DataLoader } from "@pokecrystal/core/core/data-loader";
import { asmTextLoader } from "@pokecrystal/core/core/asm-text-loader";
import { Event, EventManager } from "@pokecrystal/core/engine/events/events";
import { DAY_NAME_LABELS, STANDARD_TEXT_FALLBACKS, loadInitializeEventsConfig } from "./common";
import { ScriptRunnerImpl } from "./runner";
import type { OverworldEngine } from "@pokecrystal/core/engine/world/overworld/overworld";
import { getMapMetadataByConstant } from "@pokecrystal/core/engine/world/maps";
import * as bugContestEvents from "@pokecrystal/core/engine/world/special-events/bug-contest";
import { GB_FRAME_DURATION_MS } from "@pokecrystal/core/core/gb-timing";
import { GiveItemCommand } from "./commands/items";
import { dispatchRadioChannel } from "./specials/helpers";
import { RadioEventController } from "../radio";

type RunnerSetup = {
  runner: ScriptRunnerImpl;
  gameState: ReturnType<typeof createInitialGameState>;
  eventManager: EventManager;
};

const getRunnerLastValue = <T extends object>(runner: ScriptRunnerImpl): T => {
  const value = runner.last_value;
  if (!value || typeof value !== "object") {
    throw new Error("Expected last_value from standard script to be an object.");
  }
  return value as T;
};

type BugContestWarpResult = {
  bug_contest_warp: {
    movement: string;
    target: string;
  };
  warp?: {
    map_constant?: string;
  };
};

type BugContestResult = {
  bug_contest: {
    rank: number;
    reward: string;
  };
};

type PhoneTextResult = {
  phone_text: {
    contact: string;
    label: string;
    message: string;
    female: boolean;
  };
};

type GymStatueResult = {
  gym_statue: {
    variant: number;
    location: string;
    message: string;
    messages?: string[];
  };
};

const createRunner = (
  overrides: {
    overworld?: OverworldEngine | null;
    textMap?: Record<string, string>;
  } = {}
): RunnerSetup => {
  const gameState = createInitialGameState();
  const eventManager = new EventManager(gameState);
  const textMap = overrides.textMap ?? {};
  const dataLoader = new DataLoader();
  dataLoader.get_script = (_name: string, _parent?: string) => null;
  dataLoader.get_text = (label: string) => textMap[label] ?? "";
  const overworld = (overrides.overworld ?? {}) as OverworldEngine;
  const runner = new ScriptRunnerImpl(gameState, eventManager, dataLoader, overworld);
  return { runner, gameState, eventManager };
};

const setPhoneContact = (runner: ScriptRunnerImpl, contact: string): void => {
  runner.variables.VAR_CALLERID = contact;
};

describe("standard scripts", () => {
  const pokecenterNurseScripts = [
    "AzaleaPokecenter1FNurseScript",
    "BlackthornPokecenter1FNurseScript",
    "CeladonPokecenter1FNurseScript",
    "CeruleanPokecenter1FNurseScript",
    "CherrygrovePokecenter1FNurseScript",
    "CianwoodPokecenter1FNurseScript",
    "CinnabarPokecenter1FNurseScript",
    "EcruteakPokecenter1FNurseScript",
    "FuchsiaPokecenter1FNurseScript",
    "GoldenrodPokecenter1FNurseScript",
    "IndigoPlateauPokecenter1FNurseScript",
    "LavenderPokecenter1FNurseScript",
    "MahoganyPokecenter1FNurseScript",
    "OlivinePokecenter1FNurseScript",
    "PewterPokecenter1FNurseScript",
    "Route10Pokecenter1FNurseScript",
    "Route32Pokecenter1FNurseScript",
    "SaffronPokecenter1FNurseScript",
    "SilverCavePokecenter1FNurseScript",
    "VermilionPokecenter1FNurseScript",
    "VioletPokecenterNurse",
    "ViridianPokecenter1FNurseScript",
  ] as const;

  it.each(pokecenterNurseScripts)("resolves %s through the shared PokecenterNurseScript", (scriptName) => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const dataLoader = new DataLoader();
    const overworld = {} as OverworldEngine;
    const runner = new ScriptRunnerImpl(gameState, eventManager, dataLoader, overworld);
    const nurseSpy = jest
      .spyOn(runner.pokemon_center, "runNurseInteraction")
      .mockResolvedValue(undefined);

    runner.run(scriptName);

    expect(nurseSpy).toHaveBeenCalledWith(runner, eventManager, overworld);
    nurseSpy.mockRestore();
  });

  it("opens dialogue for the Game Corner coin vendor standard script without a Coin Case", () => {
    const { runner, eventManager } = createRunner();
    const dispatchSpy = jest.spyOn(eventManager, "dispatch");

    runner.run("GameCornerCoinVendorScript");

    const names = dispatchSpy.mock.calls.map((call) => (call[0] as Event).name);
    expect(names).toEqual(["open_text", "show_text", "wait_for_input", "close_text"]);
    expect((dispatchSpy.mock.calls[1][0] as Event).data.text).toContain("COIN CASE");
    expect(runner.last_value).toEqual({
      coin_vendor: {
        status: "no_coin_case",
      },
    });
  });

  it("waits for the Game Corner coin vendor intro before opening the buy prompt", () => {
    const { runner, gameState, eventManager } = createRunner();
    gameState.sram.key_items.COIN_CASE = 1;
    const dispatchSpy = jest.spyOn(eventManager, "dispatch");

    runner.run("GameCornerCoinVendorScript");

    const names = dispatchSpy.mock.calls.map((call) => (call[0] as Event).name);
    expect(names).toEqual(["open_text", "show_text", "show_text", "wait_for_input", "prompt_yes_no"]);
    expect((dispatchSpy.mock.calls[2][0] as Event).data.text).toContain("Do you need some");
    expect((dispatchSpy.mock.calls[4][0] as Event).data.callback).toEqual(expect.any(Function));
    expect(runner.last_value).toEqual({
      coin_vendor: {
        status: "prompt",
      },
    });
  });

  it("runs the Goldenrod Game Corner slot machine bg-event script through SlotMachine", () => {
    const gameState = createInitialGameState();
    gameState.sram.key_items.COIN_CASE = 1;
    gameState.sram.coins = 10;
    const eventManager = new EventManager(gameState);
    const dataLoader = new DataLoader();
    dataLoader.ensure_map_scripts("GoldenrodGameCorner");
    const runner = new ScriptRunnerImpl(
      gameState,
      eventManager,
      dataLoader,
      { script_runner: null } as unknown as OverworldEngine
    );
    (runner.overworld as { script_runner: ScriptRunnerImpl }).script_runner = runner;

    runner.run("GoldenrodGameCornerSlotsMachineScript");

    expect(runner.last_value).toEqual(
      expect.objectContaining({
        played: true,
        bet: expect.any(Number),
        coins: expect.any(Number),
      })
    );
    expect(gameState.sram.coins).toBe((runner.last_value as { coins: number }).coins);
  });

  it("opens the Goldenrod Game Corner TM vendor menu and completes a purchase", () => {
    const gameState = createInitialGameState();
    gameState.sram.key_items.COIN_CASE = 1;
    gameState.sram.coins = 6000;
    gameState.sram.items = {};
    const eventManager = new EventManager(gameState);
    const dataLoader = new DataLoader();
    dataLoader.ensure_map_scripts("GoldenrodGameCorner");
    const runner = new ScriptRunnerImpl(
      gameState,
      eventManager,
      dataLoader,
      {
        current_map_name: "GoldenrodGameCorner",
        get_object_by_id: jest.fn(() => null),
        audio_engine: {
          playSound: jest.fn(),
          isSoundPlaying: jest.fn(() => false),
        },
      } as unknown as OverworldEngine
    );
    const menuChoices = [1, 4];
    runner._consume_script_choice = jest.fn((key: string, defaultValue: unknown) => {
      if (key === "_vertical_menu_choice") {
        return menuChoices.shift() ?? 4;
      }
      if (key === "_yesorno_choice") {
        return true;
      }
      return defaultValue;
    });

    expect(() => runner.run("GoldenrodGameCornerTMVendorScript")).not.toThrow();
    for (let attempt = 0; attempt < 12 && (runner._script_stack.length > 0 || runner._awaiting_resume > 0); attempt += 1) {
      expect(() => runner.resume()).not.toThrow();
    }

    expect(gameState.sram.coins).toBe(500);
    expect(gameState.sram.tm_hm[24]).toBe(1);
  });

  it("runs InitializeEventsScript against parsed config", () => {
    const { runner, gameState } = createRunner();
    const config = loadInitializeEventsConfig();

    runner.run("InitializeEventsScript");

    for (const flag of config.eventFlags) {
      expect(gameState.wram.event_flags[flag]).toBe(true);
      expect(gameState.sram.event_flags[flag]).toBe(true);
    }
    for (const flag of config.engineFlags) {
      expect(gameState.wram.engine_flags[flag]).toBe(true);
    }
    for (const [spriteId, replacement] of Object.entries(config.variableSprites)) {
      expect(gameState.wram.variable_sprites[spriteId]).toBe(replacement);
    }
    expect(runner.is_busy).toBe(false);
  });

  it("prompts and activates Strength from the standard boulder script", async () => {
    const handleHm = jest.fn(async () => true);
    const { runner, eventManager } = createRunner({
      overworld: { _handle_hm: handleHm, player_state: "NORMAL" } as unknown as OverworldEngine,
    });
    const dispatchSpy = jest.spyOn(eventManager, "dispatch");

    runner.run("StrengthBoulderScript");

    const names = dispatchSpy.mock.calls.map((call) => (call[0] as Event).name);
    expect(names).toEqual(["open_text", "show_text", "wait_for_input", "prompt_yes_no"]);
    const prompt = dispatchSpy.mock.calls[3][0] as Event;
    expect(prompt.data.callback).toEqual(expect.any(Function));

    (prompt.data.callback as (accepted: boolean) => void)(true);
    await Promise.resolve();

    expect(handleHm).toHaveBeenCalledWith("Strength", 0, 0, "NORMAL");
    expect(runner.last_condition_result).toBe(true);
    expect(runner.last_value).toEqual({
      strength_boulder: {
        prompt: expect.stringContaining("Want to use\nSTRENGTH?"),
        used: true,
        message: expect.stringContaining("STRENGTH!"),
      },
    });
    expect(runner.is_busy).toBe(false);
  });

  it("toggles rocket flags and queues special calls", () => {
    const { runner, gameState } = createRunner();
    gameState.wram.event_flags.EVENT_GOLDENROD_CITY_ROCKET_TAKEOVER = true;

    runner.run("GoldenrodRocketsScript");

    expect(gameState.wram.event_flags.EVENT_GOLDENROD_CITY_ROCKET_TAKEOVER).toBe(false);

    runner.run("RadioTowerRocketsScript");

    expect(gameState.wram.engine_flags.ENGINE_ROCKETS_IN_RADIO_TOWER).toBe(true);
    expect(gameState.wram.scheduled_phone_calls).toContain("SPECIALCALL_WEIRDBROADCAST");
  });

  it("runs BugContestResultsWarpScript and records warp state", () => {
    const { runner, gameState } = createRunner();
    gameState.wram.event_flags.EVENT_BUG_CATCHING_CONTESTANT_1A = false;
    gameState.wram.event_flags.EVENT_BUG_CATCHING_CONTESTANT_1B = true;

    runner.run("BugContestResultsWarpScript");

    expect(gameState.wram.event_flags.EVENT_BUG_CATCHING_CONTESTANT_1B).toBe(false);
    expect(gameState.wram.event_flags.EVENT_ROUTE_36_NATIONAL_PARK_GATE_OFFICER_CONTEST_DAY).toBe(true);
    expect(gameState.wram.event_flags.EVENT_ROUTE_36_NATIONAL_PARK_GATE_OFFICER_NOT_CONTEST_DAY).toBe(false);
    expect(gameState.wram.event_flags.EVENT_WARPED_FROM_ROUTE_35_NATIONAL_PARK_GATE).toBe(true);

    const details = getRunnerLastValue<BugContestWarpResult>(runner);
    expect(details.bug_contest_warp).toEqual({
      movement: "Movement_ContestResults_WalkAfterWarp",
      target: "ROUTE_36_NATIONAL_PARK_GATE",
    });
    expect(details.warp?.map_constant).toBe(
      "ROUTE_36_NATIONAL_PARK_GATE"
    );
    expect(runner.is_busy).toBe(false);
  });

  it("runs BugContestResultsScript and awards the first prize", () => {
    const { runner, gameState } = createRunner();
    runner.variables._bug_contest_rank = 1;
    gameState.wram.event_flags.EVENT_LEFT_MONS_WITH_CONTEST_OFFICER = true;

    runner.run("BugContestResultsScript");

    const { bug_contest: result } = getRunnerLastValue<BugContestResult>(runner);
    expect(result.rank).toBe(1);
    expect(result.reward).toBe("SUN_STONE");
    expect(gameState.sram.items.SUN_STONE).toBe(1);
    expect(gameState.wram.engine_flags.ENGINE_DAILY_BUG_CONTEST).toBe(true);
    expect(gameState.wram.event_flags.EVENT_LEFT_MONS_WITH_CONTEST_OFFICER).toBe(false);
  });

  it("answers the Dragon Shrine test and awards Clair's Rising Badge", () => {
    const gameState = createInitialGameState();
    gameState.wram.instant_mode = true;
    const eventManager = new EventManager(gameState);
    const dataLoader = new DataLoader();
    dataLoader.ensure_map_scripts("DragonShrine");
    dataLoader.ensure_map_scripts("DragonsDenB1F");
    const object = {
      turn: jest.fn(),
      applyMovement: jest.fn(),
      x: 0,
      y: 0,
      event: { x: 0, y: 0 },
    };
    const overworld = {
      current_map_name: "DragonShrine",
      get_object_by_id: jest.fn(() => object),
      get_movement_data: jest.fn(() => ["step_end"]),
      queue_movement_task: jest.fn((_obj, _movement, options) => options?.onComplete?.()),
      show_emote: jest.fn(),
      appear_object: jest.fn(),
      remove_object: jest.fn(),
      wait_sfx: jest.fn((callback) => callback()),
      audio_engine: {
        playSound: jest.fn(),
        play_music: jest.fn(),
        fadeOutMusic: jest.fn(),
        restartMapMusic: jest.fn(),
        isSoundPlaying: jest.fn(() => false),
      },
    } as unknown as OverworldEngine;
    const runner = new ScriptRunnerImpl(gameState, eventManager, dataLoader, overworld);
    const answers = [1, 2, 2, 1, 2];
    runner._consume_script_choice = jest.fn((key: string, defaultValue: unknown) =>
      key === "_vertical_menu_choice" ? answers.shift() : defaultValue
    );

    runner.run("DragonShrineTakeTestScript");

    expect(gameState.wram.engine_flags.ENGINE_RISINGBADGE).toBe(true);
    expect(gameState.sram.badges.johto[7]).toBe(true);
    expect(gameState.wram.map_scenes.DragonShrine).toBe("SCENE_DRAGONSHRINE_NOOP");
    expect(gameState.wram.map_scenes.DragonsDenB1F).toBe("SCENE_DRAGONSDENB1F_CLAIR_GIVES_TM");
    expect(gameState.wram.scheduled_phone_calls).toContain("SPECIALCALL_MASTERBALL");
    expect(runner.is_busy).toBe(false);
  });

  it("runs DayToTextScript with VAR_WEEKDAY overrides", () => {
    const { runner, gameState } = createRunner();
    gameState.sram.day_of_week = 2;

    runner.run("DayToTextScript");

    expect(runner.string_buffers.STRING_BUFFER_3).toBe(DAY_NAME_LABELS[2]);
    expect(runner.last_value).toEqual({ day_to_text: DAY_NAME_LABELS[2] });

    runner.variables.VAR_WEEKDAY = 6;
    runner.run("DayToTextScript");

    expect(runner.string_buffers.STRING_BUFFER_3).toBe(DAY_NAME_LABELS[6]);
  });

  it("normalizes out-of-range weekdays in DayToTextScript", () => {
    const { runner } = createRunner();
    runner.variables.VAR_WEEKDAY = 9;

    runner.run("DayToTextScript");

    expect(runner.string_buffers.STRING_BUFFER_3).toBe(DAY_NAME_LABELS[2]);

    runner.variables.VAR_WEEKDAY = -1;
    runner.run("DayToTextScript");

    expect(runner.string_buffers.STRING_BUFFER_3).toBe(DAY_NAME_LABELS[6]);
  });

  it("runs ElevatorButtonScript and logs sound effects", () => {
    const playSound = jest.fn();
    const overworld = ({ audio_engine: { play_sound: playSound } } as unknown) as OverworldEngine;
    const { runner } = createRunner({ overworld });

    runner.run("ElevatorButtonScript");

    expect(playSound).toHaveBeenCalledTimes(2);
    expect(playSound).toHaveBeenNthCalledWith(1, "SFX_READ_TEXT_2");
    expect(playSound).toHaveBeenNthCalledWith(2, "SFX_ELEVATOR_END");
    expect(runner.last_sound_effect).toBe("SFX_ELEVATOR_END");
    expect(runner.last_value).toEqual({
      elevator_button: {
        sounds: ["SFX_READ_TEXT_2", "SFX_ELEVATOR_END"],
        delay_frames: 15,
      },
    });
  });

  it("runs bookshelf scripts and captures messages", () => {
    const { runner } = createRunner({
      textMap: {
        DifficultBookshelfText: "Hard to read.",
        MagazineBookshelfText: "Lots of magazines.",
      },
    });

    runner.run("DifficultBookshelfScript");
    expect(runner.last_value).toEqual({ bookshelf: "Hard to read." });

    runner.run("MagazineBookshelfScript");
    expect(runner.last_value).toEqual({ bookshelf: "Lots of magazines." });
  });

  it("runs PictureBookshelfScript and records text", () => {
    const { runner } = createRunner({
      textMap: { PictureBookshelfText: "Lots of picture books." },
    });

    runner.run("PictureBookshelfScript");

    expect(runner.last_value).toEqual({ bookshelf: "Lots of picture books." });
    expect(runner.is_busy).toBe(false);
  });

  it("falls back to standard bookshelf text when loader returns empty", () => {
    const { runner, eventManager } = createRunner();
    const handler = jest.fn();
    eventManager.on("show_text", handler);
    const asmSpy = jest.spyOn(asmTextLoader, "get").mockReturnValue("");

    runner.run("PictureBookshelfScript");

    expect(handler).toHaveBeenCalledTimes(1);
    const event = handler.mock.calls[0]?.[0] as { data?: { text?: string } } | undefined;
    expect(event?.data?.text).toBe("A whole collection\nof POKéMON picture\nbooks!");
    asmSpy.mockRestore();
  });

  it("runs TownMapScript and dispatches the overlay event", () => {
    const { runner, eventManager } = createRunner({
      textMap: { LookTownMapText: "It's a map of the region." },
    });
    const handler = jest.fn();
    eventManager.on("show_town_map", handler);

    runner.run("TownMapScript");

    expect(handler).toHaveBeenCalledTimes(1);
    const event = handler.mock.calls[0]?.[0] as { data?: { runner?: unknown } } | undefined;
    expect(event?.data?.runner).toBe(runner);
    expect(runner.last_value).toEqual({
      town_map: { opened: true, message: "It's a map of the region." },
    });
  });

  it("runs TVScript and shows TV text without opening the town map", () => {
    const { runner, eventManager } = createRunner({
      textMap: {
        TVText: "A television. Better get going!",
      },
    });
    const townMapHandler = jest.fn();
    eventManager.on("show_town_map", townMapHandler);

    runner.run("TVScript");

    expect(townMapHandler).not.toHaveBeenCalled();
    expect(runner.last_value).toEqual({
      tv: { message: "A television. Better get going!" },
    });
  });

  it("runs phone scripts using the provided contact", () => {
    const { runner } = createRunner({
      textMap: {
        JackGiftText: "Jack gift.",
        JackPackFullText: "Jack pack full.",
        JackRematchGiftText: "Jack rematch gift.",
        BeverlyRematchText: "Beverly rematch.",
        BeverlyRematchGiftText: "Beverly rematch gift.",
      },
    });
    setPhoneContact(runner, "PHONE_SCHOOLBOY_JACK");

    for (const scriptName of [
      "AskNumber1MScript",
      "AskNumber2MScript",
      "NumberAcceptedMScript",
      "NumberDeclinedMScript",
      "PhoneFullMScript",
      "RematchMScript",
      "GiftMScript",
      "PackFullMScript",
      "RematchGiftMScript",
    ]) {
      runner.run(scriptName);
      const details = getRunnerLastValue<PhoneTextResult>(runner).phone_text;
      expect(details.contact).toBe("PHONE_SCHOOLBOY_JACK");
    }

    setPhoneContact(runner, "PHONE_POKEFAN_BEVERLY");
    for (const scriptName of [
      "AskNumber1FScript",
      "AskNumber2FScript",
      "NumberAcceptedFScript",
      "NumberDeclinedFScript",
      "PhoneFullFScript",
      "RematchFScript",
      "GiftFScript",
      "PackFullFScript",
      "RematchGiftFScript",
    ]) {
      runner.run(scriptName);
      const details = getRunnerLastValue<PhoneTextResult>(runner).phone_text;
      expect(details.contact).toBe("PHONE_POKEFAN_BEVERLY");
    }
  });

  it("records phone script labels and genders", () => {
    const { runner } = createRunner({
      textMap: {
        JackGiftText: "Jack gift.",
        JackPackFullText: "Jack pack full.",
        JackRematchGiftText: "Jack rematch gift.",
        BeverlyRematchText: "Beverly rematch.",
        BeverlyRematchGiftText: "Beverly rematch gift.",
      },
    });
    const scriptMatrix = [
      { script: "AskNumber1MScript", suffix: "AskNumber1Text", female: false },
      { script: "AskNumber2MScript", suffix: "AskNumber2Text", female: false },
      { script: "NumberAcceptedMScript", suffix: "NumberAcceptedText", female: false },
      { script: "NumberDeclinedMScript", suffix: "NumberDeclinedText", female: false },
      { script: "PhoneFullMScript", suffix: "PhoneFullText", female: false },
      { script: "RematchMScript", suffix: "RematchText", female: false },
      { script: "GiftMScript", suffix: "GiftText", female: false },
      { script: "PackFullMScript", suffix: "PackFullText", female: false },
      { script: "RematchGiftMScript", suffix: "RematchGiftText", female: false },
      { script: "AskNumber1FScript", suffix: "AskNumber1Text", female: true },
      { script: "AskNumber2FScript", suffix: "AskNumber2Text", female: true },
      { script: "NumberAcceptedFScript", suffix: "NumberAcceptedText", female: true },
      { script: "NumberDeclinedFScript", suffix: "NumberDeclinedText", female: true },
      { script: "PhoneFullFScript", suffix: "PhoneFullText", female: true },
      { script: "RematchFScript", suffix: "RematchText", female: true },
      { script: "GiftFScript", suffix: "GiftText", female: true },
      { script: "PackFullFScript", suffix: "PackFullText", female: true },
      { script: "RematchGiftFScript", suffix: "RematchGiftText", female: true },
    ];

    for (const entry of scriptMatrix) {
      const contact = entry.female ? "PHONE_POKEFAN_BEVERLY" : "PHONE_SCHOOLBOY_JACK";
      setPhoneContact(runner, contact);
      runner.run(entry.script);
      const details = getRunnerLastValue<PhoneTextResult>(runner).phone_text;
      const base = entry.female ? "Beverly" : "Jack";
      expect(details.label).toBe(`${base}${entry.suffix}`);
      expect(details.female).toBe(entry.female);
    }
  });

  it("falls back to the last phone contact when caller is missing", () => {
    const { runner } = createRunner();
    delete runner.variables.VAR_CALLERID;
    runner.variables._last_phone_contact = "PHONE_SCHOOLBOY_JACK";

    runner.run("AskNumber2MScript");

    const details = getRunnerLastValue<PhoneTextResult>(runner).phone_text;
    expect(details.contact).toBe("PHONE_SCHOOLBOY_JACK");
  });

  it("loads trainer phone text from the bundled asm_text export", () => {
    const { runner } = createRunner();
    setPhoneContact(runner, "PHONE_YOUNGSTER_JOEY");

    runner.run("AskNumber1MScript");

    const details = getRunnerLastValue<PhoneTextResult>(runner).phone_text;
    expect(details.contact).toBe("PHONE_YOUNGSTER_JOEY");
    expect(details.label).toBe("JoeyAskNumber1Text");
    expect(details.message).toContain("phone number");
  });

  it("records registration sounds for phone scripts", () => {
    const { runner } = createRunner();
    setPhoneContact(runner, "PHONE_SCHOOLBOY_JACK");
    runner.run("RegisteredNumberMScript");
    expect(runner.last_sound_effect).toBe("SFX_REGISTER_PHONE_NUMBER");

    setPhoneContact(runner, "PHONE_POKEFAN_BEVERLY");
    runner.run("RegisteredNumberFScript");
    expect(runner.last_sound_effect).toBe("SFX_REGISTER_PHONE_NUMBER");
  });

  it("records registration messages for phone scripts", () => {
    const { runner } = createRunner({
      textMap: {
        RegisteredNumber1Text: "Registered male.",
        RegisteredNumber2Text: "Registered female.",
      },
    });

    setPhoneContact(runner, "PHONE_SCHOOLBOY_JACK");
    runner.run("RegisteredNumberMScript");
    expect(runner.last_value).toEqual({
      phone_registration: { message: "Registered male.", female: false },
    });

    setPhoneContact(runner, "PHONE_POKEFAN_BEVERLY");
    runner.run("RegisteredNumberFScript");
    expect(runner.last_value).toEqual({
      phone_registration: { message: "Registered female.", female: true },
    });
  });

  it("uses STRING_BUFFER_3 for registered number text", () => {
    const { runner, gameState } = createRunner();
    gameState.sram.player_name = "Ryan";
    runner.string_buffers = {
      STRING_BUFFER_1: "POKE BALL",
      STRING_BUFFER_3: "JOEY",
    };

    runner.run("RegisteredNumberMScript");

    const details = getRunnerLastValue<{ phone_registration: { message: string } }>(runner);
    expect(details.phone_registration.message).toContain("JOEY");
    expect(details.phone_registration.message).not.toContain("POKE BALL");
  });

  it("runs ReceiveItemScript with STRING_BUFFER_4 and ASM punctuation", () => {
    const playSound = jest.fn();
    const overworld = ({ audio_engine: { play_sound: playSound } } as unknown) as OverworldEngine;
    const { runner, eventManager, gameState } = createRunner({ overworld });
    const showHandler = jest.fn();
    const waitHandler = jest.fn();
    eventManager.on("show_text", showHandler);
    eventManager.on("wait_for_input", waitHandler);

    gameState.sram.player_name = "KRIS";
    runner.string_buffers = { STRING_BUFFER_4: "POTION" };
    runner.last_condition_result = true;
    const pauseSpy = jest.spyOn(runner, "pause");

    runner.run("ReceiveItemScript");

    expect(playSound).toHaveBeenCalledWith("SFX_ITEM");
    expect(pauseSpy).toHaveBeenCalled();
    expect(showHandler).toHaveBeenCalledTimes(1);
    const event = showHandler.mock.calls[0]?.[0] as { data?: { text?: string } } | undefined;
    expect(event?.data?.text).toBe("KRIS received\nPOTION.");
    expect(waitHandler).toHaveBeenCalledTimes(1);
  });

  it("throws when ReceiveItemScript is missing STRING_BUFFER_4", () => {
    const { runner } = createRunner();
    runner.string_buffers = { STRING_BUFFER_1: "POTION" };

    expect(() => runner.run("ReceiveItemScript")).toThrow(
      "ReceiveItemScript requires STRING_BUFFER_4 to contain the received item name."
    );
  });

  it("uses GB frame timing for FindItemInBallScript pause handling", () => {
    jest.useFakeTimers();
    const { runner } = createRunner();
    runner.last_condition_result = true;
    const pauseSpy = jest.spyOn(runner, "pause");
    const resumeSpy = jest.spyOn(runner, "resume");
    const setTimeoutSpy = jest.spyOn(global, "setTimeout");

    try {
      runner.run("FindItemInBallScript");
      expect(pauseSpy).toHaveBeenCalled();

      const scheduledDelay = setTimeoutSpy.mock.calls.at(-1)?.[1];
      expect(typeof scheduledDelay).toBe("number");
      expect(Number(scheduledDelay)).toBeCloseTo(60 * GB_FRAME_DURATION_MS, 5);

      jest.advanceTimersByTime(Math.ceil(60 * GB_FRAME_DURATION_MS));
      expect(resumeSpy).toHaveBeenCalled();
    } finally {
      setTimeoutSpy.mockRestore();
      jest.useRealTimers();
    }
  });

  it("keeps ReceiveItemScript presentation-only because giveitem already updated the bag", () => {
    const { runner, gameState } = createRunner();
    gameState.sram.player_name = "KRIS";
    runner.string_buffers = { STRING_BUFFER_4: "POTION" };
    gameState.sram.items.POTION = 1;
    runner.last_condition_result = false;

    runner.run("ReceiveItemScript");

    expect(gameState.sram.items.POTION).toBe(1);
    expect(runner.last_sound_effect).toBe("SFX_ITEM");
  });

  it("adds the item before queueing ReceiveItemScript", () => {
    const { runner, gameState, eventManager } = createRunner();
    const command = new GiveItemCommand("POTION");
    command.runner = runner;

    command.execute(gameState, eventManager, runner.overworld as OverworldEngine);

    expect(gameState.sram.items.POTION).toBe(1);
    expect(runner.string_buffers.STRING_BUFFER_4).toBe("POTION");
    expect(runner.stopExecution).toBe(true);
    expect(runner.last_sound_effect).toBe("SFX_ITEM");
  });

  it("uses the Lucky Channel transcript for MAPRADIO_LUCKY_CHANNEL", () => {
    const { runner } = createRunner({
      textMap: {
        LC_Text1: "REED: Yeehaw! How",
        LC_Text2: "y'all doin' now?",
        LC_Text3: "Whether you're up",
        LC_Text4: "or way down low,",
        LC_Text5: "don't you miss the",
        LC_Text6: "LUCKY NUMBER SHOW!",
        LC_Text7: "This week's Lucky",
        LC_Text8: "Number is 12345!",
        LC_Text9: "I'll repeat that!",
        LC_Text10: "Match it and go to",
        LC_Text11: "the RADIO TOWER!",
        PlayersRadioText1: "pokemon-channel-fallback-1",
      },
    });
    (runner as unknown as { eventManager: EventManager | null }).eventManager = null;

    dispatchRadioChannel(runner, "RADIO", "MAPRADIO_LUCKY_CHANNEL");

    const details = getRunnerLastValue<{
      radio: {
        station: string;
        transcript: Array<{ label: string; message: string }>;
      };
    }>(runner);

    expect(details.radio.station).toBe("LUCKY_CHANNEL");
    expect(details.radio.transcript.map((entry) => entry.label)).toEqual([
      "LC_Text1",
      "LC_Text2",
      "LC_Text3",
      "LC_Text4",
      "LC_Text5",
      "LC_Text6",
      "LC_Text7",
      "LC_Text8",
      "LC_Text9",
      "LC_Text7",
      "LC_Text8",
      "LC_Text10",
      "LC_Text11",
    ]);
    expect(details.radio.transcript.some((entry) => entry.message.includes("pokemon-channel-fallback"))).toBe(false);
  });

  it("routes MAPRADIO_POKEMON_CHANNEL to Places and People music in Kanto", () => {
    const metadata = getMapMetadataByConstant("PALLET_TOWN");
    if (!metadata) {
      throw new Error("Missing PALLET_TOWN metadata required for Kanto radio parity test.");
    }

    const { runner, eventManager, gameState } = createRunner({
      textMap: {
        PlayersRadioText1: "fallback-1",
        PlayersRadioText2: "fallback-2",
        PlayersRadioText3: "fallback-3",
        PlayersRadioText4: "fallback-4",
      },
    });
    gameState.wram.wMapGroup = metadata.groupId;
    gameState.wram.wMapNumber = metadata.mapId;
    gameState.wram.current_map_group = metadata.groupId;
    gameState.wram.current_map_id = metadata.mapId;
    gameState.wram.engine_flags.ENGINE_EXPN_CARD = false;

    const warn = jest.spyOn(console, "warn").mockImplementation(() => {});
    const playMusic = jest.fn();
    const controller = new RadioEventController({
      eventManager,
      audioEngine: {
        playMusic,
        restartMapMusic: jest.fn(),
      } as any,
    });
    controller.register();

    dispatchRadioChannel(runner, "RADIO", "MAPRADIO_POKEMON_CHANNEL");

    expect(playMusic).toHaveBeenCalledWith("MUSIC_VIRIDIAN_CITY", "general");
    expect(warn).not.toHaveBeenCalled();
  });

  it("populates gym statue buffers and messages", () => {
    const { runner, gameState } = createRunner({
      textMap: {
        GymStatue_CityGymText: "@\n#MON GYM",
        GymStatue_WinningTrainersText: "LEADER: @\n\nWINNING TRAINERS:\n<PLAYER>",
      },
    });
    const metadata = getMapMetadataByConstant("VIOLET_GYM");
    expect(metadata).toBeDefined();
    if (!metadata) {
      return;
    }
    gameState.wram.current_map_group = metadata.groupId;
    gameState.wram.current_map_id = metadata.mapId;

    runner.run("GymStatue1Script");
    const first = getRunnerLastValue<GymStatueResult>(runner).gym_statue;
    expect(first.variant).toBe(1);
    expect(runner.string_buffers.STRING_BUFFER_3).toBe(first.location);
    expect(first.message).toBe("VIOLETGYM\nPOK\u00e9MON GYM");

    runner.string_buffers.STRING_BUFFER_4 = "Falkner Falkner1";
    runner.run("GymStatue2Script");
    const second = getRunnerLastValue<GymStatueResult>(runner).gym_statue;
    const secondMessages = second.messages ?? [];
    expect(second.variant).toBe(2);
    expect(secondMessages).toHaveLength(2);
    expect(secondMessages[0]).toBe("VIOLETGYM\nPOK\u00e9MON GYM");
    expect(secondMessages[1]).toBe("LEADER: Falkner Falkner1\n\nWINNING TRAINERS:\nPLAYER");
  });

  it("records radio tower takeover flags and scene", () => {
    const { runner, gameState } = createRunner();

    runner.run("RadioTowerRocketsScript");

    expect(gameState.wram.engine_flags.ENGINE_ROCKETS_IN_RADIO_TOWER).toBe(true);
    expect(gameState.wram.event_flags.EVENT_GOLDENROD_CITY_CIVILIANS).toBe(true);
    expect(gameState.wram.event_flags.EVENT_RADIO_TOWER_BLACKBELT_BLOCKS_STAIRS).toBe(true);
    expect(gameState.wram.event_flags.EVENT_RADIO_TOWER_ROCKET_TAKEOVER).toBe(false);
    expect(gameState.wram.event_flags.EVENT_USED_THE_CARD_KEY_IN_THE_RADIO_TOWER).toBe(false);
    expect(gameState.wram.event_flags.EVENT_MAHOGANY_TOWN_POKEFAN_M_BLOCKS_EAST).toBe(true);
    expect(gameState.wram.scheduled_phone_calls).toContain("SPECIALCALL_WEIRDBROADCAST");
    expect(gameState.wram.map_scenes.MahoganyTown).toBe("SCENE_MAHOGANYTOWN_NOOP");
    expect(runner.last_value).toEqual({
      radio_tower: {
        call: "SPECIALCALL_WEIRDBROADCAST",
        map_scene: "SCENE_MAHOGANYTOWN_NOOP",
      },
    });
  });

  it("runs BugContestResultsScript for non-first-place rewards", () => {
    const cases = [
      { rank: 2, reward: "EVERSTONE" },
      { rank: 3, reward: "GOLD_BERRY" },
      { rank: 4, reward: "BERRY" },
    ];

    for (const { rank, reward } of cases) {
      const { runner, gameState } = createRunner();
      runner.variables._bug_contest_rank = rank;

      runner.run("BugContestResultsScript");

      const { bug_contest: result } = getRunnerLastValue<BugContestResult>(runner);
      expect(result.rank).toBe(rank);
      expect(result.reward).toBe(reward);
      expect(gameState.sram.items[reward]).toBe(1);
      expect(gameState.wram.engine_flags.ENGINE_DAILY_BUG_CONTEST).toBe(true);
    }
  });

  it("invokes bug contest judging when rank is missing", () => {
    const { runner } = createRunner();
    const spy = jest.spyOn(bugContestEvents, "bug_contest_judging").mockImplementation(
      (_state, { runner: contextRunner } = {}) => {
        if (contextRunner) {
          contextRunner.variables._bug_contest_rank = 2;
        }
        return 2;
      }
    );

    runner.run("BugContestResultsScript");

    expect(spy).toHaveBeenCalledTimes(1);
    const { bug_contest: result } = getRunnerLastValue<BugContestResult>(runner);
    expect(result.rank).toBe(2);
    expect(result.reward).toBe("EVERSTONE");
    spy.mockRestore();
  });
});
