import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import { SCREEN_TILE_WIDTH, TEXTBOX_HEIGHT_TILES, TEXTBOX_Y_TILES } from "@pokecrystal/core/core/text-constants";
import { PlayerGender } from "@pokecrystal/core/core/enums";
import { OakIntroSequence } from "./oak-intro-sequence";

describe("OakIntroSequence sprite and text rendering parity", () => {
  const makeSequence = (): any =>
    new (OakIntroSequence as any)({
      playMusic: jest.fn(),
      playSound: jest.fn(),
      isSoundPlaying: jest.fn().mockReturnValue(false),
      loadMusic: jest.fn(),
      loadSound: jest.fn(),
      music: {},
      sounds: {},
    }, {}, {
      render_text: jest.fn(),
      renderText: jest.fn(),
      get_char_tile: jest.fn(),
      getCharTile: jest.fn(),
    });

  it("splits animated pokemon strips into square frames before rendering", () => {
    const sequence = makeSequence();
    const strip = new gameEngine.Surface(40, 80);
    strip.fill([255, 0, 0, 255], new gameEngine.Rect(0, 0, 40, 40));
    strip.fill([0, 0, 255, 255], new gameEngine.Rect(0, 40, 40, 40));

    const frames = sequence.extractPokemonFrames(strip) as Array<InstanceType<typeof gameEngine.Surface>>;

    expect(frames).toHaveLength(2);
    expect(frames[0].get_size()).toEqual([40, 40]);
    expect(frames[1].get_size()).toEqual([40, 40]);
    expect(frames[0].get_at([0, 0]).slice(0, 3)).toEqual([255, 0, 0]);
    expect(frames[1].get_at([0, 0]).slice(0, 3)).toEqual([0, 0, 255]);
  });

  it("throws when animated pokemon geometry is not a square strip", () => {
    const sequence = makeSequence();
    const malformed = new gameEngine.Surface(40, 70);

    expect(() => sequence.extractPokemonFrames(malformed)).toThrow(
      "Oak intro animated frontpic must be a square-frame strip."
    );
  });

  it("normalizes frontpics into centered 56x56 surfaces", () => {
    const sequence = makeSequence();
    const compact = new gameEngine.Surface(40, 40);
    compact.fill([0, 0, 0, 255]);

    const normalized = sequence.normalizeFrontpicSurface(compact) as InstanceType<typeof gameEngine.Surface>;

    expect(normalized.get_size()).toEqual([56, 56]);
    expect(normalized.get_at([8, 8]).slice(0, 3)).toEqual([0, 0, 0]);
    expect(normalized.get_at([0, 0]).slice(0, 3)).toEqual([255, 255, 255]);
  });

  it("delegates Oak textbox rendering to the shared boot textbox renderer", () => {
    const sequence = makeSequence();
    const drawTextBox = jest.fn();
    sequence.bootTextboxRenderer = {
      drawTextBox,
      drawPromptArrow: jest.fn(),
    };

    const ctx = {
      canvas: { width: 160, height: 144 },
    } as unknown as CanvasRenderingContext2D;

    sequence.showTextBox(ctx, "AB");

    expect(drawTextBox).toHaveBeenCalledWith(
      ctx,
      "AB",
      0,
      TEXTBOX_Y_TILES,
      SCREEN_TILE_WIDTH,
      TEXTBOX_HEIGHT_TILES
    );
  });

  it("advances Whooper animation using the cached frame count", () => {
    const sequence = makeSequence();
    sequence.currentSprite = "wooper";
    sequence.spriteAnimationCache.set("pokemon:wooper", [
      new gameEngine.Surface(56, 56),
      new gameEngine.Surface(56, 56),
    ]);

    for (let step = 0; step < 15; step += 1) {
      sequence.updateAnimation();
    }
    expect(sequence.spriteFrame).toBe(1);

    for (let step = 0; step < 15; step += 1) {
      sequence.updateAnimation();
    }
    expect(sequence.spriteFrame).toBe(0);
  });

  it("applies the ASM trainer palette to the player intro picture", () => {
    const sequence = makeSequence();
    sequence.playerGender = PlayerGender.MALE;
    sequence.loadPlayerPalette = jest.fn(() => [
      [255, 255, 255],
      [200, 120, 80],
      [80, 40, 200],
      [0, 0, 0],
    ]);
    const grayscale = new gameEngine.Surface(2, 2);
    grayscale.set_at([0, 0], [255, 255, 255, 255]);
    grayscale.set_at([1, 0], [170, 170, 170, 255]);
    grayscale.set_at([0, 1], [85, 85, 85, 255]);
    grayscale.set_at([1, 1], [0, 0, 0, 255]);

    const recolored = sequence.applyPlayerPalette(grayscale) as InstanceType<typeof gameEngine.Surface>;

    expect(recolored.get_at([0, 0]).slice(0, 3)).toEqual([255, 255, 255]);
    expect(recolored.get_at([1, 0]).slice(0, 3)).toEqual([200, 120, 80]);
    expect(recolored.get_at([0, 1]).slice(0, 3)).toEqual([80, 40, 200]);
    expect(recolored.get_at([1, 1]).slice(0, 3)).toEqual([0, 0, 0]);
  });

  it("builds a text snapshot for the active Oak dialogue page", () => {
    const sequence = makeSequence();
    sequence.timeSetComplete = true;
    sequence.mode = "intro";
    sequence.sceneState = "oak_intro_1";
    sequence.scenePhase = "text";
    sequence.currentSprite = "oak";
    sequence.currentText = "Hello!\nWelcome!";
    sequence.visibleChars = sequence.currentText.length;
    sequence.waitingForInput = true;

    const snapshot = sequence.getTextSnapshot();

    expect(snapshot.viewportTitle).toBe("Oak Intro");
    expect(snapshot.viewportLines).toEqual(expect.arrayContaining(["OAK INTRO", "SPRITE: OAK"]));
    expect(snapshot.infoLines).toEqual(
      expect.arrayContaining(["STATE: oak_intro", "SCENE: oak_intro_1", "PHASE: text", "A/START=Advance", "B=Skip intro"])
    );
    expect(snapshot.dialogueLines).toEqual(["Hello!", "Welcome!"]);
    expect(snapshot.promptLines).toBeNull();
  });

  it("starts Oak speech with Route 30 music instead of overworld music", () => {
    const playMusic = jest.fn();
    const sequence = new (OakIntroSequence as any)({
      playMusic,
      playSound: jest.fn(),
      isSoundPlaying: jest.fn().mockReturnValue(false),
      loadMusic: jest.fn(),
      loadSound: jest.fn(),
      music: {},
      sounds: {},
    }, {}, {
      render_text: jest.fn(),
      renderText: jest.fn(),
      get_char_tile: jest.fn(),
      getCharTile: jest.fn(),
    });

    sequence.timeSetComplete = true;
    sequence.mode = "intro";
    sequence.sceneState = "";
    sequence.scenePhase = "fade_in";
    sequence.sceneOakIntroFirst();

    expect(playMusic).toHaveBeenCalledWith("MUSIC_ROUTE_30", "intro");
    expect(playMusic).not.toHaveBeenCalledWith("MUSIC_NEW_BARK_TOWN", "map");
  });

  it("skips Oak intro fades immediately in instant mode", () => {
    const sequence = makeSequence();

    sequence.setInstantMode(true);
    sequence.setupScene({
      name: "oak_intro_1",
      sprite: "oak",
      spriteType: "trainer",
      textPages: ["Hello!"],
      fadeInSteps: 4,
      fadeOutSteps: 3,
    });

    expect(sequence.fadeActive).toBe(false);
    expect(sequence.fadeAlpha).toBe(0);
    expect(sequence.scenePhase).toBe("text");
  });

  it("makes each Oak intro A press advance exactly one completed text page in instant mode", () => {
    const sequence = makeSequence();
    sequence.setInstantMode(true);
    sequence.timeSetComplete = true;
    sequence.queueText(["First page.", "Second page."], true);

    sequence.advanceTextQueue();
    expect(sequence.currentText).toBe("First page.");
    expect(sequence.visibleChars).toBe("First page.".length);
    expect(sequence.waitingForInput).toBe(true);

    sequence.handleInput({ type: "keydown", button: "a", is_press: true } as any);
    sequence.advanceTextQueue();
    sequence.advanceTextQueue();

    expect(sequence.currentText).toBe("Second page.");
    expect(sequence.visibleChars).toBe("Second page.".length);
    expect(sequence.waitingForInput).toBe(true);
  });

  it("exposes intro text and time-set phase through the debug state", () => {
    const sequence = makeSequence();
    sequence.timeSetComplete = false;
    sequence.timeSetScreen = { getPhase: jest.fn(() => "set_hour") };

    expect(sequence.getDebugState()).toEqual(
      expect.objectContaining({
        timeSetPhase: "set_hour",
        visibleText: "",
      })
    );

    sequence.timeSetComplete = true;
    sequence.currentText = "HELLO";
    sequence.visibleChars = 3;

    expect(sequence.getDebugState()).toEqual(
      expect.objectContaining({
        timeSetPhase: null,
        visibleText: "HEL",
      })
    );
  });
});
