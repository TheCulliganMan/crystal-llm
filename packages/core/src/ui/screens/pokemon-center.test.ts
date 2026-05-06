import { PokemonCenterSystem } from "./pokemon-center";
import { createInitialGameState } from "@pokecrystal/core/core/state";
import { PokemonSchema } from "@pokecrystal/core/core/models";
import { EventManager } from "@pokecrystal/core/engine/world/events";
import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import * as asyncLoop from "@pokecrystal/core/ui/async-loop";
import { DialogueWindow, FieldDialogueManager } from "@pokecrystal/core/ui/text/dialogue";
import { Surface } from "@pokecrystal/core/ui/surface";
import { getMapMetadataByName } from "@pokecrystal/core/engine/world/maps";
import {
  Ability,
  EggGroup,
  GenderRatio,
  GrowthRate,
  PokemonType,
} from "@pokecrystal/core/core/enums";
import { StatusCondition } from "@pokecrystal/core/core/enums/battle";

const buildPokemon = () =>
  PokemonSchema.parse({
    species: {
      id: "CHIKORITA",
      int_id: 152,
      base_stats: {
        hp: 45,
        attack: 49,
        defense: 65,
        speed: 45,
        special_attack: 49,
        special_defense: 65,
      },
      type1: PokemonType.GRASS,
      type2: PokemonType.GRASS,
      catch_rate: 45,
      base_exp: 64,
      gender_ratio: GenderRatio.GENDER_F12_5,
      unknown1: 0,
      step_cycles_to_hatch: 20,
      unknown2: 0,
      growth_rate: GrowthRate.GROWTH_MEDIUM_SLOW,
      egg_group1: EggGroup.EGG_MONSTER,
      egg_group2: EggGroup.EGG_PLANT,
      tmhm_learnset: [],
      ability: Ability.NONE,
      pic_size: 0,
      front_pic: 0,
      back_pic: 0,
      weight: 0,
    },
    nickname: "CHIKORITA",
    level: 5,
    hp: 3,
    max_hp: 20,
    status: StatusCondition.POISON,
    sleep_turns: 2,
    original_trainer_name: "PLAYER",
    original_trainer_id: 0,
    experience: 0,
    happiness: 70,
  });

describe("PokemonCenterSystem heal_party", () => {
  it("restores HP and clears status", () => {
    const gameState = createInitialGameState();
    const pokemon = buildPokemon();
    gameState.sram.party.pokemon[0] = pokemon;
    const system = new PokemonCenterSystem(gameState);

    const summary = system.heal_party();

    expect(summary.healed_slots).toEqual([0]);
    expect(pokemon.hp).toBe(pokemon.max_hp);
    expect(pokemon.status).toBeNull();
    expect(pokemon.sleep_turns).toBe(2);
  });

  it("does not heal eggs", () => {
    const gameState = createInitialGameState();
    const egg = buildPokemon();
    egg.species.id = "EGG";
    egg.nickname = "EGG";
    egg.hp = 1;
    egg.max_hp = 20;
    egg.status = StatusCondition.POISON;
    gameState.sram.party.pokemon[0] = egg;
    const system = new PokemonCenterSystem(gameState);

    const summary = system.heal_party();

    expect(summary.healed_slots).toEqual([]);
    expect(egg.hp).toBe(1);
    expect(egg.status).toBe(StatusCondition.POISON);
  });
});

describe("PokemonCenterSystem runNurseInteraction", () => {
  it("stops map music before running the heal machine animation", async () => {
    const gameState = createInitialGameState();
    const system = new PokemonCenterSystem(gameState);
    const eventManager = new EventManager(gameState);
    eventManager.on("prompt_yes_no", (event) => {
      (event.data as { callback?: (choice: boolean) => void }).callback?.(true);
    });

    const audioEngine = {
      playMusic: jest.fn(),
      fadeOutMusic: jest.fn(),
      restartMapMusic: jest.fn(),
    };
    const overworld = {
      dialogue: {
        waiting_for_input: false,
        window: {
          has_more_pages: () => false,
          is_complete: () => true,
        },
      },
      handleInput: jest.fn(),
      update: jest.fn(),
      draw: jest.fn(),
      ui: {
        screen: new Surface(160, 144),
        font: { renderText: jest.fn() },
        eventQueue: gameEngine.event.createQueue(),
        update: jest.fn(),
      },
      audio_engine: audioEngine,
    };

    const delaySpy = jest.spyOn(gameEngine.time, "delay").mockImplementation(() => {});
    const playSpy = jest
      .spyOn(system, "playHealMachineAnimationAsync")
      .mockResolvedValue(undefined);

    await system.runNurseInteraction({ variables: {} }, eventManager, overworld);

    expect(audioEngine.playMusic).toHaveBeenCalledWith("MUSIC_NONE", "heal");
    expect(audioEngine.fadeOutMusic).not.toHaveBeenCalled();

    playSpy.mockRestore();
    delaySpy.mockRestore();
  });

  it("waits for async heal-machine playback before restarting map music", async () => {
    const gameState = createInitialGameState();
    const system = new PokemonCenterSystem(gameState);
    const eventManager = new EventManager(gameState);
    eventManager.on("prompt_yes_no", (event) => {
      (event.data as { callback?: (choice: boolean) => void }).callback?.(true);
    });

    const audioEngine = {
      playMusic: jest.fn(),
      fadeOutMusic: jest.fn(),
      restartMapMusic: jest.fn(),
    };
    const shownTexts: string[] = [];
    eventManager.on("show_text", (event) => {
      shownTexts.push(String((event.data as { text?: unknown }).text ?? ""));
    });
    const overworld = {
      dialogue: {
        waiting_for_input: false,
        window: {
          has_more_pages: () => false,
          is_complete: () => true,
        },
      },
      handleInput: jest.fn(),
      update: jest.fn(),
      draw: jest.fn(),
      ui: {
        screen: new Surface(160, 144),
        font: { renderText: jest.fn() },
        eventQueue: gameEngine.event.createQueue(),
        update: jest.fn(),
      },
      audio_engine: audioEngine,
    };

    const delaySpy = jest.spyOn(gameEngine.time, "delay").mockImplementation(() => {});
    let resolveAnimation!: () => void;
    const animationGate = new Promise<void>((resolve) => {
      resolveAnimation = resolve;
    });
    const nextFrameSpy = jest.spyOn(asyncLoop, "nextFrame").mockResolvedValue(undefined);
    const playSpy = jest
      .spyOn(system, "playHealMachineAnimationAsync")
      .mockReturnValue(animationGate);

    const interaction = system.runNurseInteraction({ variables: {} }, eventManager, overworld);
    await Promise.resolve();
    expect(audioEngine.restartMapMusic).not.toHaveBeenCalled();
    expect(
      shownTexts.some((text) => text.includes("Thank you for waiting."))
    ).toBe(false);

    resolveAnimation();
    await interaction;
    expect(audioEngine.restartMapMusic).toHaveBeenCalledTimes(1);
    expect(
      shownTexts.some((text) => text.includes("Thank you for waiting."))
    ).toBe(true);

    playSpy.mockRestore();
    delaySpy.mockRestore();
  });

  it("heals immediately in instant mode without running heal-machine playback or pause frames", async () => {
    const gameState = createInitialGameState();
    gameState.wram.instant_mode = true;
    const pokemon = buildPokemon();
    gameState.sram.party.pokemon[0] = pokemon;
    const system = new PokemonCenterSystem(gameState);
    const eventManager = new EventManager(gameState);
    const audioEngine = {
      playMusic: jest.fn(),
      restartMapMusic: jest.fn(),
    };
    const overworld = {
      dialogue: {
        waiting_for_input: false,
        window: {
          has_more_pages: () => false,
          is_complete: () => true,
        },
      },
      handleInput: jest.fn(),
      update: jest.fn(),
      draw: jest.fn(),
      ui: {
        screen: new Surface(160, 144),
        font: { renderText: jest.fn() },
        eventQueue: gameEngine.event.createQueue(),
        update: jest.fn(),
      },
      audio_engine: audioEngine,
    };

    const promptSpy = jest
      .spyOn(system as unknown as { promptYesNo: typeof system["promptYesNo"] }, "promptYesNo")
      .mockResolvedValue(true);
    const waitSpy = jest
      .spyOn(system as unknown as { waitForButton: typeof system["waitForButton"] }, "waitForButton")
      .mockResolvedValue(undefined);
    const playSpy = jest.spyOn(system, "playHealMachineAnimationAsync");

    await system.runNurseInteraction({ variables: {} }, eventManager, overworld);

    expect(pokemon.hp).toBe(pokemon.max_hp);
    expect(pokemon.status).toBeNull();
    expect(playSpy).toHaveBeenCalledTimes(1);
    expect(audioEngine.restartMapMusic).toHaveBeenCalledTimes(1);

    playSpy.mockRestore();
    waitSpy.mockRestore();
    promptSpy.mockRestore();
  });

  it("locks player movement while the heal machine animation is running", async () => {
    const gameState = createInitialGameState();
    const system = new PokemonCenterSystem(gameState);
    const eventManager = new EventManager(gameState);
    eventManager.on("prompt_yes_no", (event) => {
      (event.data as { callback?: (choice: boolean) => void }).callback?.(true);
    });

    const lockMovement = jest.fn();
    const unlockMovement = jest.fn();
    let resolveAnimation!: () => void;
    const animationGate = new Promise<void>((resolve) => {
      resolveAnimation = resolve;
    });
    const nextFrameSpy = jest.spyOn(asyncLoop, "nextFrame").mockResolvedValue(undefined);
    const playSpy = jest
      .spyOn(system, "playHealMachineAnimationAsync")
      .mockReturnValue(animationGate);

    const overworld = {
      lock_player_movement: lockMovement,
      unlock_player_movement: unlockMovement,
      dialogue: {
        waiting_for_input: false,
        window: {
          has_more_pages: () => false,
          is_complete: () => true,
        },
      },
      handleInput: jest.fn(),
      update: jest.fn(),
      draw: jest.fn(),
      ui: {
        screen: new Surface(160, 144),
        font: { renderText: jest.fn() },
        eventQueue: gameEngine.event.createQueue(),
        update: jest.fn(),
      },
      audio_engine: {
        playMusic: jest.fn(),
        restartMapMusic: jest.fn(),
      },
    };

    const interaction = system.runNurseInteraction({ variables: {} }, eventManager, overworld);
    for (let i = 0; i < 10 && playSpy.mock.calls.length === 0; i += 1) {
      await new Promise((resolve) => setTimeout(resolve, 0));
    }

    expect(lockMovement).toHaveBeenCalledTimes(1);
    expect(unlockMovement).not.toHaveBeenCalled();

    resolveAnimation();
    await interaction;

    expect(unlockMovement).toHaveBeenCalledTimes(1);

    nextFrameSpy.mockRestore();
    playSpy.mockRestore();
  });

  it("holds input capture for the full nurse interaction, including the heal animation window", async () => {
    const gameState = createInitialGameState();
    const system = new PokemonCenterSystem(gameState);
    const eventManager = new EventManager(gameState);
    eventManager.on("prompt_yes_no", (event) => {
      (event.data as { callback?: (choice: boolean) => void }).callback?.(true);
    });

    let resolveAnimation!: () => void;
    const animationGate = new Promise<void>((resolve) => {
      resolveAnimation = resolve;
    });
    const playSpy = jest
      .spyOn(system, "playHealMachineAnimationAsync")
      .mockReturnValue(animationGate);

    const overworld = {
      input_capture_active: false,
      dialogue: {
        waiting_for_input: false,
        window: {
          has_more_pages: () => false,
          is_complete: () => true,
        },
      },
      handleInput: jest.fn(),
      update: jest.fn(),
      draw: jest.fn(),
      ui: {
        screen: new Surface(160, 144),
        font: { renderText: jest.fn() },
        eventQueue: gameEngine.event.createQueue(),
        update: jest.fn(),
      },
      audio_engine: {
        playMusic: jest.fn(),
        restartMapMusic: jest.fn(),
      },
    };

    const interaction = system.runNurseInteraction({ variables: {} }, eventManager, overworld);
    for (let i = 0; i < 10 && playSpy.mock.calls.length === 0; i += 1) {
      await new Promise((resolve) => setTimeout(resolve, 0));
    }

    expect(overworld.input_capture_active).toBe(true);

    resolveAnimation();
    await interaction;

    expect(overworld.input_capture_active).toBe(false);
    playSpy.mockRestore();
  });

  it("coalesces repeated nurse interaction requests while one is already in progress", async () => {
    const gameState = createInitialGameState();
    const system = new PokemonCenterSystem(gameState);
    const eventManager = new EventManager(gameState);
    eventManager.on("prompt_yes_no", (event) => {
      (event.data as { callback?: (choice: boolean) => void }).callback?.(true);
    });

    let resolveAnimation!: () => void;
    const animationGate = new Promise<void>((resolve) => {
      resolveAnimation = resolve;
    });
    const runSpy = jest
      .spyOn(system as unknown as { runNurseInteractionAsync: typeof system["runNurseInteractionAsync"] }, "runNurseInteractionAsync")
      .mockImplementation(async () => {
        await animationGate;
      });

    const overworld = {
      input_capture_active: false,
      dialogue: {
        waiting_for_input: false,
        window: {
          has_more_pages: () => false,
          is_complete: () => true,
        },
      },
      ui: {
        screen: new Surface(160, 144),
        font: { renderText: jest.fn() },
        eventQueue: gameEngine.event.createQueue(),
        update: jest.fn(),
      },
    };

    const first = system.runNurseInteraction({ variables: {} }, eventManager, overworld);
    const second = system.runNurseInteraction({ variables: {} }, eventManager, overworld);

    expect(second).toBe(first);
    expect(runSpy).toHaveBeenCalledTimes(1);

    resolveAnimation();
    await Promise.all([first, second]);
    runSpy.mockRestore();
  });

  it("shows the Pokerus nurse branch and queues the Elm phone call", async () => {
    const gameState = createInitialGameState();
    const infected = buildPokemon();
    infected.pokerus = true;
    gameState.sram.party.pokemon[0] = infected;
    const system = new PokemonCenterSystem(gameState);
    const eventManager = new EventManager(gameState);
    eventManager.on("prompt_yes_no", (event) => {
      (event.data as { callback?: (choice: boolean) => void }).callback?.(true);
    });

    const shownTexts: string[] = [];
    eventManager.on("show_text", (event) => {
      shownTexts.push(String((event.data as { text?: unknown }).text ?? ""));
    });

    const audioEngine = {
      playMusic: jest.fn(),
      fadeOutMusic: jest.fn(),
      restartMapMusic: jest.fn(),
    };
    const overworld = {
      dialogue: {
        waiting_for_input: false,
        window: {
          has_more_pages: () => false,
          is_complete: () => true,
        },
      },
      handleInput: jest.fn(),
      update: jest.fn(),
      draw: jest.fn(),
      ui: {
        screen: new Surface(160, 144),
        font: { renderText: jest.fn() },
        eventQueue: gameEngine.event.createQueue(),
        update: jest.fn(),
      },
      audio_engine: audioEngine,
    };

    const delaySpy = jest.spyOn(gameEngine.time, "delay").mockImplementation(() => {});
    const playSpy = jest
      .spyOn(system, "playHealMachineAnimationAsync")
      .mockResolvedValue(undefined);

    await system.runNurseInteraction({ variables: {} }, eventManager, overworld);

    expect(shownTexts.some((text) => text.includes("infected by tiny"))).toBe(true);
    expect(
      shownTexts.some((text) => text.includes("Thank you for waiting."))
    ).toBe(false);
    expect(shownTexts).not.toContain("We hope to see you again.");
    expect(gameState.wram.engine_flags.ENGINE_CAUGHT_POKERUS).toBe(true);
    expect(gameState.wram.scheduled_phone_calls).toContain("SPECIALCALL_POKERUS");

    playSpy.mockRestore();
    delaySpy.mockRestore();
  });

  it("processes overworld input events while waiting for button input", async () => {
    jest.useFakeTimers();
    const gameState = createInitialGameState();
    const system = new PokemonCenterSystem(gameState);
    const eventManager = new EventManager(gameState);
    const ui = {
      screen: new Surface(160, 144),
      font: { renderText: jest.fn() },
      drawTextBox: jest.fn(),
      draw_window: jest.fn(),
    };
    const runner = { pause: jest.fn(), _script_stack: [{}], _awaiting_resume: 0 };
    const dialogue = new FieldDialogueManager(ui, gameState, runner);
    const windowStub = {
      is_complete: () => false,
      has_more_pages: () => false,
      complete: jest.fn(),
      advance_page: jest.fn(),
    };
    (dialogue as unknown as { window: typeof windowStub }).window = windowStub;
    for (const eventName of ["open_text", "close_text", "show_text", "wait_for_input", "prompt_yes_no"]) {
      eventManager.on(eventName, dialogue.handle_event.bind(dialogue));
    }
    const eventQueue = gameEngine.event.createQueue();
    const overworld = {
      dialogue,
      handleInput: (event: Parameters<typeof dialogue.handle_input>[0]) => {
        dialogue.handle_input(event);
      },
      ui: { eventQueue },
    };

    const waitPromise = (system as any).waitForButton(eventManager, overworld, "test");

    jest.advanceTimersByTime(1);
    gameEngine.event.post({ type: "keyup", code: "KeyZ" }, eventQueue);
    gameEngine.event.post({ type: "keydown", code: "KeyZ" }, eventQueue);
    jest.advanceTimersByTime(20);

    await waitPromise;

    expect(dialogue.waiting_for_input).toBe(false);
    jest.useRealTimers();
  });

  it("processes input from the active event queue when it differs from the overworld queue", async () => {
    jest.useFakeTimers();
    const gameState = createInitialGameState();
    const system = new PokemonCenterSystem(gameState);
    const eventManager = new EventManager(gameState);
    const ui = {
      screen: new Surface(160, 144),
      font: { renderText: jest.fn() },
      drawTextBox: jest.fn(),
      draw_window: jest.fn(),
    };
    const runner = { pause: jest.fn(), _script_stack: [{}], _awaiting_resume: 0 };
    const dialogue = new FieldDialogueManager(ui, gameState, runner);
    const windowStub = {
      is_complete: () => false,
      has_more_pages: () => false,
      complete: jest.fn(),
      advance_page: jest.fn(),
    };
    (dialogue as unknown as { window: typeof windowStub }).window = windowStub;
    for (const eventName of ["open_text", "close_text", "show_text", "wait_for_input", "prompt_yes_no"]) {
      eventManager.on(eventName, dialogue.handle_event.bind(dialogue));
    }
    const overworldQueue = gameEngine.event.createQueue();
    const activeQueue = gameEngine.event.createQueue();
    const overworld = {
      dialogue,
      handleInput: (event: Parameters<typeof dialogue.handle_input>[0]) => {
        dialogue.handle_input(event);
      },
      ui: { eventQueue: overworldQueue },
    };

    const previousQueue = gameEngine.event.getActiveQueue();
    gameEngine.event.setActiveQueue(activeQueue);

    try {
      const waitPromise = (system as any).waitForButton(eventManager, overworld, "active-queue");

      jest.advanceTimersByTime(1);
      gameEngine.event.post({ type: "keyup", code: "KeyZ" }, activeQueue);
      gameEngine.event.post({ type: "keydown", code: "KeyZ" }, activeQueue);
      jest.advanceTimersByTime(20);

      await waitPromise;

      expect(dialogue.waiting_for_input).toBe(false);
    } finally {
      gameEngine.event.setActiveQueue(previousQueue);
      jest.useRealTimers();
    }
  });

  it("awaits question pages using bound dialogue window methods", async () => {
    const gameState = createInitialGameState();
    const system = new PokemonCenterSystem(gameState);
    const ui = {
      screen: new Surface(160, 144),
      font: { renderText: jest.fn() },
      drawTextBox: jest.fn(),
      draw_window: jest.fn(),
    };
    const window = new DialogueWindow(ui, gameState, 2);
    window.open("Hello!");
    window.complete();

    const overworld = {
      dialogue: { window },
    };

    await expect(
      (system as unknown as { awaitQuestionPage: (ow: typeof overworld, ctx: string) => Promise<void> })
        .awaitQuestionPage(overworld, "binding")
    ).resolves.toBeUndefined();
  });

  it("replaces the post-heal return text before the single final goodbye wait and releases input capture", async () => {
    const gameState = createInitialGameState();
    const system = new PokemonCenterSystem(gameState);
    const eventManager = new EventManager(gameState);
    const events: Array<{ name: string; text?: string }> = [];
    const lockMovement = jest.fn();
    const unlockMovement = jest.fn();
    for (const eventName of ["open_text", "close_text", "show_text"]) {
      eventManager.on(eventName, (event) => {
        events.push({
          name: event.name,
          text:
            event.name === "show_text"
              ? String((event.data as { text?: unknown }).text ?? "")
              : undefined,
        });
      });
    }

    const runner = { pause: jest.fn(), _script_stack: [{}], _awaiting_resume: 0, variables: {} };
    const overworld = {
      input_capture_active: false,
      lock_player_movement: lockMovement,
      unlock_player_movement: unlockMovement,
      dialogue: {
        waiting_for_input: false,
        window: {
          has_more_pages: () => false,
          is_complete: () => true,
        },
      },
      ui: { eventQueue: gameEngine.event.createQueue(), update: jest.fn() },
      update: jest.fn(),
      draw: jest.fn(),
      audio_engine: {
        playMusic: jest.fn(),
        restartMapMusic: jest.fn(),
      },
    };

    const promptSpy = jest
      .spyOn(system as unknown as { promptYesNo: typeof system["promptYesNo"] }, "promptYesNo")
      .mockResolvedValue(true);
    const waitSpy = jest
      .spyOn(system as unknown as { waitForButton: typeof system["waitForButton"] }, "waitForButton")
      .mockResolvedValue(undefined);
    const pauseSpy = jest
      .spyOn(system as unknown as { pauseFrames: typeof system["pauseFrames"] }, "pauseFrames")
      .mockResolvedValue(undefined);
    const playSpy = jest
      .spyOn(system, "playHealMachineAnimationAsync")
      .mockResolvedValue(undefined);

    await system.runNurseInteraction(runner, eventManager, overworld);

    expect(overworld.input_capture_active).toBe(false);
    expect(lockMovement).toHaveBeenCalledTimes(1);
    expect(unlockMovement).toHaveBeenCalledTimes(1);
    const returnTextIndex = events.findIndex((event) =>
      event.text?.includes("Thank you for waiting.")
    );
    expect(returnTextIndex).toBeGreaterThanOrEqual(0);
    expect(events.slice(returnTextIndex, returnTextIndex + 4)).toEqual([
      {
        name: "show_text",
        text: "Thank you for waiting.\n\nYour #MON are fully healed.",
      },
      {
        name: "close_text",
        text: undefined,
      },
      {
        name: "open_text",
        text: undefined,
      },
      {
        name: "show_text",
        text: "We hope to see you again.",
      },
    ]);

    playSpy.mockRestore();
    pauseSpy.mockRestore();
    waitSpy.mockRestore();
    promptSpy.mockRestore();
  });
});

describe("PokemonCenterSystem heal-machine map routing", () => {
  it("skips direct heal-machine playback in instant mode", async () => {
    const gameState = createInitialGameState();
    gameState.wram.instant_mode = true;
    const system = new PokemonCenterSystem(gameState);
    const animator = (system as unknown as {
      healMachineAnimator: {
        play: (id: string | null, slots: number, overworld: unknown) => void;
        playAsync: (id: string | null, slots: number, overworld: unknown) => Promise<void>;
      };
    }).healMachineAnimator;
    const playSpy = jest.spyOn(animator, "play").mockImplementation(() => {});
    const playAsyncSpy = jest.spyOn(animator, "playAsync").mockResolvedValue(undefined);

    system.playHealMachineAnimation("1", { current_map_name: "CherrygrovePokecenter1F" } as any);
    await system.playHealMachineAnimationAsync("1", { current_map_name: "CherrygrovePokecenter1F" } as any);

    expect(playSpy).not.toHaveBeenCalled();
    expect(playAsyncSpy).not.toHaveBeenCalled();

    playAsyncSpy.mockRestore();
    playSpy.mockRestore();
  });

  it("locks movement and captures input for direct async heal-machine playback", async () => {
    const gameState = createInitialGameState();
    const system = new PokemonCenterSystem(gameState);
    const animator = (system as unknown as {
      healMachineAnimator: {
        playAsync: (id: string | null, slots: number, overworld: unknown) => Promise<void>;
      };
    }).healMachineAnimator;
    let resolveAnimation!: () => void;
    const animationGate = new Promise<void>((resolve) => {
      resolveAnimation = resolve;
    });
    const playAsyncSpy = jest.spyOn(animator, "playAsync").mockReturnValue(animationGate);
    const lockMovement = jest.fn();
    const unlockMovement = jest.fn();
    const overworld = {
      current_map_name: "ElmsLab",
      input_capture_active: false,
      lock_player_movement: lockMovement,
      unlock_player_movement: unlockMovement,
    };

    const interaction = system.playHealMachineAnimationAsync("1", overworld as any);

    expect(overworld.input_capture_active).toBe(true);
    expect(lockMovement).toHaveBeenCalledTimes(1);
    expect(unlockMovement).not.toHaveBeenCalled();

    resolveAnimation();
    await interaction;

    expect(overworld.input_capture_active).toBe(false);
    expect(unlockMovement).toHaveBeenCalledTimes(1);
    playAsyncSpy.mockRestore();
  });

  it("locks movement and restores input capture for direct sync heal-machine playback", () => {
    const gameState = createInitialGameState();
    const system = new PokemonCenterSystem(gameState);
    const animator = (system as unknown as {
      healMachineAnimator: {
        play: (id: string | null, slots: number, overworld: unknown) => void;
      };
    }).healMachineAnimator;
    const playSpy = jest.spyOn(animator, "play").mockImplementation(() => undefined);
    const lockMovement = jest.fn();
    const unlockMovement = jest.fn();
    const overworld = {
      current_map_name: "ElmsLab",
      input_capture_active: false,
      lock_player_movement: lockMovement,
      unlock_player_movement: unlockMovement,
    };

    system.playHealMachineAnimation("1", overworld as any);

    expect(overworld.input_capture_active).toBe(false);
    expect(lockMovement).toHaveBeenCalledTimes(1);
    expect(unlockMovement).toHaveBeenCalledTimes(1);
    playSpy.mockRestore();
  });

  it("forces the Pokecenter machine layout on Pokecenter maps", async () => {
    const gameState = createInitialGameState();
    const system = new PokemonCenterSystem(gameState);
    const animator = (system as unknown as { healMachineAnimator: { playAsync: (id: string | null, slots: number, overworld: unknown) => Promise<void> } }).healMachineAnimator;
    const playAsyncSpy = jest.spyOn(animator, "playAsync").mockResolvedValue(undefined);
    const overworld = { current_map_name: "VioletPokecenter1F" };

    await system.playHealMachineAnimationAsync("2", overworld as any);

    expect(playAsyncSpy).toHaveBeenCalledWith(
      "HEALMACHINE_POKECENTER",
      expect.any(Number),
      overworld
    );
    playAsyncSpy.mockRestore();
  });

  it("falls back to WRAM map metadata when overworld map name is unavailable", async () => {
    const gameState = createInitialGameState();
    const metadata = getMapMetadataByName("VioletPokecenter1F");
    expect(metadata).toBeDefined();
    gameState.wram.wMapGroup = metadata?.groupId ?? 0;
    gameState.wram.wMapNumber = metadata?.mapId ?? 0;

    const system = new PokemonCenterSystem(gameState);
    const animator = (system as unknown as { healMachineAnimator: { playAsync: (id: string | null, slots: number, overworld: unknown) => Promise<void> } }).healMachineAnimator;
    const playAsyncSpy = jest.spyOn(animator, "playAsync").mockResolvedValue(undefined);

    await system.playHealMachineAnimationAsync("2", null);

    expect(playAsyncSpy).toHaveBeenCalledWith(
      "HEALMACHINE_POKECENTER",
      expect.any(Number),
      null
    );
    playAsyncSpy.mockRestore();
  });

  it("normalizes punctuation when matching Elms Lab map names", async () => {
    const gameState = createInitialGameState();
    const system = new PokemonCenterSystem(gameState);
    const animator = (system as unknown as { healMachineAnimator: { playAsync: (id: string | null, slots: number, overworld: unknown) => Promise<void> } }).healMachineAnimator;
    const playAsyncSpy = jest.spyOn(animator, "playAsync").mockResolvedValue(undefined);
    const overworld = { current_map_name: "Elm's Lab" };

    await system.playHealMachineAnimationAsync("0", overworld as any);

    expect(playAsyncSpy).toHaveBeenCalledWith(
      "HEALMACHINE_ELMS_LAB",
      expect.any(Number),
      overworld
    );
    playAsyncSpy.mockRestore();
  });

  it("routes PokeCom Center nurse interactions to the Pokecenter heal machine layout", async () => {
    const gameState = createInitialGameState();
    const system = new PokemonCenterSystem(gameState);
    const animator = (system as unknown as { healMachineAnimator: { playAsync: (id: string | null, slots: number, overworld: unknown) => Promise<void> } }).healMachineAnimator;
    const playAsyncSpy = jest.spyOn(animator, "playAsync").mockResolvedValue(undefined);
    const overworld = { current_map_name: "PokecomCenterAdminOfficeMobile" };

    await system.playHealMachineAnimationAsync("2", overworld as any);

    expect(playAsyncSpy).toHaveBeenCalledWith(
      "HEALMACHINE_POKECENTER",
      expect.any(Number),
      overworld
    );
    playAsyncSpy.mockRestore();
  });

  it("routes Hall of Fame interactions to the Hall of Fame heal machine layout", async () => {
    const gameState = createInitialGameState();
    const system = new PokemonCenterSystem(gameState);
    const animator = (system as unknown as { healMachineAnimator: { playAsync: (id: string | null, slots: number, overworld: unknown) => Promise<void> } }).healMachineAnimator;
    const playAsyncSpy = jest.spyOn(animator, "playAsync").mockResolvedValue(undefined);
    const overworld = { current_map_name: "HallOfFame" };

    await system.playHealMachineAnimationAsync("0", overworld as any);

    expect(playAsyncSpy).toHaveBeenCalledWith(
      "HEALMACHINE_HALL_OF_FAME",
      expect.any(Number),
      overworld
    );
    playAsyncSpy.mockRestore();
  });
});

describe("PokemonCenterSystem yes/no fallback", () => {
  it("returns false instead of auto-accepting when overworld prompt plumbing is missing", async () => {
    const gameState = createInitialGameState();
    const system = new PokemonCenterSystem(gameState);
    const eventManager = new EventManager(gameState);
    const runner = {
      last_yes_no_result: undefined,
      last_condition_result: undefined,
    };

    const result = await (system as any).promptYesNo(runner, eventManager, { dialogue: null }, "missing-plumbing");

    expect(result).toBe(false);
    expect(runner.last_yes_no_result).toBe(false);
    expect(runner.last_condition_result).toBe(false);
  });

  it("delegates fallback yes/no prompts to a command_map yesno factory when available", async () => {
    const gameState = createInitialGameState();
    const system = new PokemonCenterSystem(gameState);
    const eventManager = new EventManager(gameState);
    let command: {
      runner?: unknown;
      on_result?: (value: boolean) => void;
      execute: jest.Mock;
    };
    command = {
      runner: null,
      on_result: undefined,
      execute: jest.fn(() => {
        command.on_result?.(true);
      }),
    };
    const runner = {
      last_yes_no_result: false,
      last_condition_result: false,
      command_map: {
        yesno: () => command,
      },
    };

    const result = await (system as any).promptYesNo(runner, eventManager, null, "command-map");

    expect(result).toBe(true);
    expect(command.runner).toBe(runner);
    expect(command.execute).toHaveBeenCalledWith(gameState, eventManager, null);
    expect(runner.last_yes_no_result).toBe(true);
    expect(runner.last_condition_result).toBe(true);
  });
});
