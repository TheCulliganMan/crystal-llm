import { Pokemon, PokemonSpecies, PokemonSchema, toPokemon } from "@pokecrystal/core/core/models/pokemon";
import {
  Ability,
  BattleTurn,
  EggGroup,
  GenderRatio,
  GrowthRate,
  MoveEffect,
  MoveName,
  PokemonType,
  Stat,
  StatusCondition,
} from "@pokecrystal/core/core/enums";
import { EventManager } from "@pokecrystal/core/engine/events/events";
import { createInitialGameState } from "@pokecrystal/core/core/state";
import { BattleContext, Weather } from "./battle-context";
import { executeMove } from "./move-execution";
import * as damageCalc from "./damage-calculation";
import * as debugLog from "@pokecrystal/core/core/debug-log";
import * as dataLoader from "@pokecrystal/core/core/data-loader";
import { animation_label_for_move } from "@pokecrystal/core/ui/overlays/battle-animation-util";
import Fraction from "fraction.js";

const mockSpecies: PokemonSpecies = {
  id: "MOCK",
  int_id: 1,
  base_stats: { hp: 50, attack: 50, defense: 50, speed: 50, special_attack: 50, special_defense: 50 },
  type1: PokemonType.NORMAL,
  type2: PokemonType.NONE,
  catch_rate: 255,
  base_exp: 64,
  item1: null,
  item2: null,
  gender_ratio: GenderRatio.GENDER_F50,
  unknown1: 0,
  step_cycles_to_hatch: 20,
  unknown2: 0,
  growth_rate: GrowthRate.GROWTH_MEDIUM_FAST,
  egg_group1: EggGroup.EGG_MONSTER,
  egg_group2: EggGroup.EGG_MONSTER,
  tmhm_learnset: [],
  ability: Ability.NONE,
  pic_size: 0,
  front_pic: 0,
  back_pic: 0,
  weight: 0,
  evolutions: null,
} as PokemonSpecies;

const createMockPokemon = (hp: number, max_hp: number, nickname: string): Pokemon => {
  return toPokemon(
    PokemonSchema.parse({
      species: mockSpecies,
      nickname,
      level: 5,
      hp,
      max_hp,
      attack: 10,
      defense: 10,
      speed: 10,
      special_attack: 10,
      special_defense: 10,
      stat_boosts: {
        HP: 0,
        ATTACK: 0,
        DEFENSE: 0,
        SPEED: 0,
        SPECIAL_ATTACK: 0,
        SPECIAL_DEFENSE: 0,
        ACCURACY: 0,
        EVASION: 0,
      },
      moves: [],
      dvs: { attack: 0, defense: 0, speed: 0, special: 0, hp: 0 },
      status: undefined,
      original_trainer_name: "TRAINER",
      original_trainer_id: 12345,
      experience: 0,
      happiness: 70,
    })
  );
};

describe("executeMove logging", () => {
  afterEach(() => {
    jest.restoreAllMocks();
  });

  it("logs the move and hp changes for damage", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const attacker = createMockPokemon(30, 30, "ATTACKER");
    const defender = createMockPokemon(20, 20, "DEFENDER");
    attacker.moves = [{ name: MoveName.TACKLE, current_pp: 35 }];
    const context = new BattleContext(
      [attacker],
      [defender],
      attacker,
      defender,
      undefined,
      false,
      undefined,
      0
    );
    context.predefinedRandomValue = 0;
    const movesMap = new Map([
      [
        MoveName.TACKLE,
        {
          name: MoveName.TACKLE,
          effect: MoveEffect.NONE,
          power: 40,
          type: PokemonType.NORMAL,
          accuracy: 100,
          pp: 35,
          effect_chance: 0,
        },
      ],
    ]);
    const battle = { movesMap, eventManager, gameState, context } as unknown as {
      movesMap: Map<MoveName, unknown>;
      eventManager: EventManager;
      gameState: ReturnType<typeof createInitialGameState>;
      context: BattleContext;
    };

    jest.spyOn(damageCalc, "calculateDamage")
      .mockReturnValueOnce({
        damage: 0,
        type_multiplier: new Fraction(1),
      })
      .mockReturnValue({
        damage: 5,
        type_multiplier: new Fraction(1),
      });
    const logSpy = jest.spyOn(debugLog, "pushDebugLog").mockImplementation(() => {});

    executeMove(battle as any, BattleTurn.PLAYER, attacker, defender, MoveName.TACKLE);

    const moveLog = logSpy.mock.calls.find(([message]) => message.includes("[battle] move start"));
    expect(moveLog).toBeTruthy();
    expect(moveLog?.[1]).toEqual(
      expect.objectContaining({
        move: MoveName.TACKLE,
        attacker: attacker.nickname,
        target: defender.nickname,
      })
    );
  });

  it("hydrates missing move definitions from the canonical move table", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const attacker = createMockPokemon(30, 30, "ATTACKER");
    const defender = createMockPokemon(20, 20, "DEFENDER");
    attacker.moves = [{ name: MoveName.TACKLE, current_pp: 35 }];
    const context = new BattleContext(
      [attacker],
      [defender],
      attacker,
      defender,
      undefined,
      false,
      undefined,
      0
    );
    context.predefinedRandomValue = 0;
    const fallbackMove = {
      name: MoveName.TACKLE,
      effect: MoveEffect.NONE,
      power: 40,
      type: PokemonType.NORMAL,
      accuracy: 100,
      pp: 35,
      effect_chance: 0,
    };
    const battle = {
      movesMap: new Map(),
      eventManager,
      gameState,
      context,
    } as unknown as {
      movesMap: Map<MoveName, unknown>;
      eventManager: EventManager;
      gameState: ReturnType<typeof createInitialGameState>;
      context: BattleContext;
    };

    jest.spyOn(dataLoader, "loadAllMoves").mockReturnValue(
      new Map([[MoveName.TACKLE, fallbackMove]])
    );
    jest.spyOn(damageCalc, "calculateDamage").mockReturnValue({
      damage: 5,
      type_multiplier: new Fraction(1),
    });

    executeMove(battle as any, BattleTurn.PLAYER, attacker, defender, MoveName.TACKLE);

    expect((battle.movesMap as Map<MoveName, unknown>).get(MoveName.TACKLE)).toEqual(fallbackMove);
    expect(defender.hp).toBe(15);
  });

  it("consumes a held BERRY after damage drops HP to half or less", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const attacker = createMockPokemon(30, 30, "ATTACKER");
    const defender = createMockPokemon(16, 26, "DEFENDER");
    attacker.moves = [{ name: MoveName.TACKLE, current_pp: 35 }];
    defender.item = "BERRY";
    const context = new BattleContext(
      [attacker],
      [defender],
      attacker,
      defender,
      undefined,
      false,
      undefined,
      0
    );
    context.predefinedRandomValue = 0;
    const battle = {
      movesMap: new Map([
        [
          MoveName.TACKLE,
          {
            name: MoveName.TACKLE,
            effect: MoveEffect.NONE,
            power: 40,
            type: PokemonType.NORMAL,
            accuracy: 100,
            pp: 35,
            effect_chance: 0,
          },
        ],
      ]),
      eventManager,
      gameState,
      context,
    } as unknown as {
      movesMap: Map<MoveName, unknown>;
      eventManager: EventManager;
      gameState: ReturnType<typeof createInitialGameState>;
      context: BattleContext;
    };

    jest.spyOn(damageCalc, "calculateDamage").mockReturnValue({
      damage: 5,
      type_multiplier: new Fraction(1),
    });
    const texts: string[] = [];
    eventManager.on("show_text", (event) => {
      const payload = event.data as { text?: string };
      if (typeof payload.text === "string") {
        texts.push(payload.text);
      }
    });

    executeMove(battle as any, BattleTurn.PLAYER, attacker, defender, MoveName.TACKLE);

    expect(defender.hp).toBe(21);
    expect(defender.item).toBeUndefined();
    expect(texts).toContain("DEFENDER recovered using a BERRY!");
  });

  it("executes Transform end to end with animation, copied moves, and follow-up text", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const attacker = createMockPokemon(30, 30, "DITTO");
    const defender = createMockPokemon(20, 20, "MEW");
    attacker.species = { ...attacker.species, id: "DITTO" };
    defender.species = { ...defender.species, id: "MEW", type1: PokemonType.PSYCHIC_TYPE };
    attacker.moves = [{ name: MoveName.TRANSFORM, current_pp: 10 }];
    defender.moves = [
      { name: MoveName.PSYCHIC_M, current_pp: 16 },
      { name: MoveName.SKETCH, current_pp: 1 },
    ];
    defender.stat_boosts[Stat.ATTACK] = 2;
    const context = new BattleContext(
      [attacker],
      [defender],
      attacker,
      defender,
      undefined,
      false,
      undefined,
      0
    );
    context.predefinedRandomValue = 0;
    const movesMap = new Map([
      [
        MoveName.TRANSFORM,
        {
          name: MoveName.TRANSFORM,
          effect: MoveEffect.TRANSFORM,
          power: 0,
          type: PokemonType.NORMAL,
          accuracy: 100,
          pp: 10,
          effect_chance: 0,
        },
      ],
    ]);
    const texts: string[] = [];
    const animations: Array<{ move_name?: MoveName; animation_label?: string }> = [];
    eventManager.on("show_text", (event) => {
      const payload = event.data as { text?: string };
      if (payload.text) {
        texts.push(payload.text);
      }
    });
    eventManager.on("play_animation", (event) => {
      animations.push(event.data as { move_name?: MoveName });
    });
    const battle = {
      movesMap,
      eventManager,
      gameState,
      context,
    } as unknown as {
      movesMap: Map<MoveName, unknown>;
      eventManager: EventManager;
      gameState: ReturnType<typeof createInitialGameState>;
      context: BattleContext;
    };

    jest.spyOn(damageCalc, "calculateDamage").mockReturnValue({
      damage: 0,
      type_multiplier: new Fraction(1),
    });

    executeMove(battle as any, BattleTurn.PLAYER, attacker, defender, MoveName.TRANSFORM);

    expect(animations).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          move_name: MoveName.TRANSFORM,
          animation_label: animation_label_for_move(MoveName.TRANSFORM),
        }),
      ])
    );
    expect(texts).toEqual(
      expect.arrayContaining([
        "DITTO used TRANSFORM!",
        "DITTO transformed into MEW!",
      ])
    );
    expect(attacker.transformed).toBe(true);
    expect(attacker.species.id).toBe("MEW");
    expect(attacker.moves).toEqual([
      { name: MoveName.PSYCHIC_M, current_pp: 5 },
      { name: MoveName.SKETCH, current_pp: 1 },
    ]);
    expect(attacker.stat_boosts[Stat.ATTACK]).toBe(2);
  });

  it("lets a locked-on follow-up move hit despite accuracy", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const attacker = createMockPokemon(30, 30, "ATTACKER");
    const defender = createMockPokemon(20, 20, "DEFENDER");
    attacker.moves = [
      { name: MoveName.LOCK_ON, current_pp: 5 },
      { name: MoveName.ZAP_CANNON, current_pp: 5 },
    ];
    const context = new BattleContext(
      [attacker],
      [defender],
      attacker,
      defender,
      undefined,
      false,
      undefined,
      0
    );
    const movesMap = new Map<MoveName, any>([
      [MoveName.LOCK_ON, {
        name: MoveName.LOCK_ON,
        effect: MoveEffect.LOCK_ON,
        power: 0,
        type: PokemonType.NORMAL,
        accuracy: 100,
        pp: 5,
        effect_chance: 0,
      }],
      [MoveName.ZAP_CANNON, {
        name: MoveName.ZAP_CANNON,
        effect: MoveEffect.NORMAL_HIT,
        power: 100,
        type: PokemonType.ELECTRIC,
        accuracy: 1,
        pp: 5,
        effect_chance: 0,
      }],
    ]);
    const battle = {
      movesMap,
      eventManager,
      gameState,
      context,
    } as any;

    jest.spyOn(damageCalc, "calculateDamage").mockReturnValue({
      damage: 5,
      type_multiplier: new Fraction(1),
    });

    executeMove(battle, BattleTurn.PLAYER, attacker, defender, MoveName.LOCK_ON);
    expect(attacker.lock_on_active).toBe(true);
    const hpAfterLockOn = defender.hp;

    executeMove(battle, BattleTurn.PLAYER, attacker, defender, MoveName.ZAP_CANNON);
    expect(defender.hp).toBeLessThan(hpAfterLockOn);
    expect(attacker.lock_on_active).toBe(false);
  });

  it("builds rage when a raging target is hit", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const attacker = createMockPokemon(30, 30, "ATTACKER");
    const defender = createMockPokemon(20, 20, "DEFENDER");
    attacker.moves = [{ name: MoveName.TACKLE, current_pp: 35 }];
    defender.rage_active = true;
    defender.rage_counter = 0;
    const context = new BattleContext(
      [attacker],
      [defender],
      attacker,
      defender,
      undefined,
      false,
      undefined,
      0
    );
    context.predefinedRandomValue = 0;
    const movesMap = new Map([
      [
        MoveName.TACKLE,
        {
          name: MoveName.TACKLE,
          effect: MoveEffect.NONE,
          power: 40,
          type: PokemonType.NORMAL,
          accuracy: 100,
          pp: 35,
          effect_chance: 0,
        },
      ],
    ]);
    const battle = { movesMap, eventManager, gameState, context } as unknown as {
      movesMap: Map<MoveName, unknown>;
      eventManager: EventManager;
      gameState: ReturnType<typeof createInitialGameState>;
      context: BattleContext;
    };

    jest.spyOn(damageCalc, "calculateDamage").mockReturnValue({
      damage: 5,
      type_multiplier: new Fraction(1),
    });
    const dispatchSpy = jest.spyOn(eventManager, "dispatch");

    executeMove(battle as any, BattleTurn.PLAYER, attacker, defender, MoveName.TACKLE);

    expect(defender.rage_counter).toBe(1);
    const rageText = dispatchSpy.mock.calls.find(
      ([event]) => event.data?.text === `${defender.nickname}'s RAGE is building!`
    );
    expect(rageText).toBeTruthy();
  });

  it("clears rage state when using a different move", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const attacker = createMockPokemon(30, 30, "ATTACKER");
    const defender = createMockPokemon(20, 20, "DEFENDER");
    attacker.moves = [{ name: MoveName.TACKLE, current_pp: 35 }];
    attacker.rage_active = true;
    attacker.rage_counter = 3;
    const context = new BattleContext(
      [attacker],
      [defender],
      attacker,
      defender,
      undefined,
      false,
      undefined,
      0
    );
    context.predefinedRandomValue = 0;
    const movesMap = new Map([
      [
        MoveName.TACKLE,
        {
          name: MoveName.TACKLE,
          effect: MoveEffect.NONE,
          power: 40,
          type: PokemonType.NORMAL,
          accuracy: 100,
          pp: 35,
          effect_chance: 0,
        },
      ],
    ]);
    const battle = { movesMap, eventManager, gameState, context } as unknown as {
      movesMap: Map<MoveName, unknown>;
      eventManager: EventManager;
      gameState: ReturnType<typeof createInitialGameState>;
      context: BattleContext;
    };

    jest.spyOn(damageCalc, "calculateDamage").mockReturnValue({
      damage: 5,
      type_multiplier: new Fraction(1),
    });

    executeMove(battle as any, BattleTurn.PLAYER, attacker, defender, MoveName.TACKLE);

    expect(attacker.rage_active).toBe(false);
    expect(attacker.rage_counter).toBe(0);
  });

  it("does not play attack animation when the move misses", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const attacker = createMockPokemon(30, 30, "ATTACKER");
    const defender = createMockPokemon(20, 20, "DEFENDER");
    attacker.moves = [{ name: MoveName.TACKLE, current_pp: 35 }];
    const context = new BattleContext(
      [attacker],
      [defender],
      attacker,
      defender,
      undefined,
      false,
      undefined,
      0
    );
    context.predefinedRandomValue = 1;
    const movesMap = new Map([
      [
        MoveName.TACKLE,
        {
          name: MoveName.TACKLE,
          effect: MoveEffect.NONE,
          power: 40,
          type: PokemonType.NORMAL,
          accuracy: 1,
          pp: 35,
          effect_chance: 0,
        },
      ],
    ]);
    const battle = { movesMap, eventManager, gameState, context } as unknown as {
      movesMap: Map<MoveName, unknown>;
      eventManager: EventManager;
      gameState: ReturnType<typeof createInitialGameState>;
      context: BattleContext;
    };

    const calculateDamageSpy = jest
      .spyOn(damageCalc, "calculateDamage")
      .mockReturnValue({
        damage: 5,
        type_multiplier: new Fraction(1),
      });
    const dispatchSpy = jest.spyOn(eventManager, "dispatch");

    executeMove(battle as any, BattleTurn.PLAYER, attacker, defender, MoveName.TACKLE);

    const events = dispatchSpy.mock.calls.map(([event]) => event);
    expect(
      events.find((event) => event.name === "play_animation")
    ).toBeUndefined();
    expect(
      events.find(
        (event) => event.name === "show_text" && event.data?.text === `${attacker.nickname}'s attack missed!`
      )
    ).toBeTruthy();
    expect(calculateDamageSpy).not.toHaveBeenCalled();
  });

  it("still plays attack animation for type-immune moves and shows no-effect text", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const attacker = createMockPokemon(30, 30, "ATTACKER");
    const defender = createMockPokemon(20, 20, "DEFENDER");
    defender.species.type1 = PokemonType.GHOST;
    defender.species.type2 = PokemonType.NONE;
    attacker.moves = [{ name: MoveName.TACKLE, current_pp: 35 }];
    const context = new BattleContext(
      [attacker],
      [defender],
      attacker,
      defender,
      undefined,
      false,
      undefined,
      0
    );
    context.predefinedRandomValue = 0;
    const movesMap = new Map([
      [
        MoveName.TACKLE,
        {
          name: MoveName.TACKLE,
          effect: MoveEffect.NONE,
          power: 40,
          type: PokemonType.NORMAL,
          accuracy: 100,
          pp: 35,
          effect_chance: 0,
        },
      ],
    ]);
    const battle = { movesMap, eventManager, gameState, context } as unknown as {
      movesMap: Map<MoveName, unknown>;
      eventManager: EventManager;
      gameState: ReturnType<typeof createInitialGameState>;
      context: BattleContext;
    };

    jest.spyOn(damageCalc, "calculateDamage").mockReturnValue({
      damage: 0,
      type_multiplier: new Fraction(0),
    });
    const dispatchSpy = jest.spyOn(eventManager, "dispatch");

    executeMove(battle as any, BattleTurn.PLAYER, attacker, defender, MoveName.TACKLE);

    const events = dispatchSpy.mock.calls.map(([event]) => event);
    expect(
      events.find(
        (event) =>
          event.name === "play_animation" &&
          event.data?.move_name === MoveName.TACKLE &&
          event.data?.animation_label === animation_label_for_move(MoveName.TACKLE)
      )
    ).toBeTruthy();
    expect(
      events.find(
        (event) => event.name === "show_text" && event.data?.text === `${attacker.nickname}'s attack missed!`
      )
    ).toBeFalsy();
  });

  it("does not apply move effects after a type immunity no-effect result", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const attacker = createMockPokemon(30, 30, "ATTACKER");
    const defender = createMockPokemon(20, 20, "DEFENDER");
    defender.species.type1 = PokemonType.GROUND;
    defender.species.type2 = PokemonType.NONE;
    attacker.moves = [{ name: MoveName.THUNDER_WAVE, current_pp: 20 }];
    const context = new BattleContext(
      [attacker],
      [defender],
      attacker,
      defender,
      undefined,
      false,
      undefined,
      0
    );
    context.predefinedRandomValue = 0;
    const movesMap = new Map([
      [
        MoveName.THUNDER_WAVE,
        {
          name: MoveName.THUNDER_WAVE,
          effect: MoveEffect.PARALYZE,
          power: 0,
          type: PokemonType.ELECTRIC,
          accuracy: 100,
          pp: 20,
          effect_chance: 0,
        },
      ],
    ]);
    const battle = { movesMap, eventManager, gameState, context } as unknown as {
      movesMap: Map<MoveName, unknown>;
      eventManager: EventManager;
      gameState: ReturnType<typeof createInitialGameState>;
      context: BattleContext;
    };

    jest.spyOn(damageCalc, "calculateDamage").mockReturnValue({
      damage: 0,
      type_multiplier: new Fraction(0),
    });
    const texts: string[] = [];
    eventManager.on("show_text", (event) => {
      const payload = event.data as { text?: string };
      if (typeof payload.text === "string") {
        texts.push(payload.text);
      }
    });

    executeMove(battle as any, BattleTurn.PLAYER, attacker, defender, MoveName.THUNDER_WAVE);

    expect(defender.status).toBeUndefined();
    expect(texts).toContain(`It doesn't affect\n${defender.nickname}!`);
    expect(texts).not.toContain(`${defender.nickname} was paralyzed!`);
  });

  it("blocks incoming damage and effects when the defender is protected", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const attacker = createMockPokemon(30, 30, "ATTACKER");
    const defender = createMockPokemon(20, 20, "DEFENDER");
    attacker.moves = [{ name: MoveName.TACKLE, current_pp: 35 }];
    defender.protect_active = true;
    const context = new BattleContext(
      [attacker],
      [defender],
      attacker,
      defender,
      undefined,
      false,
      undefined,
      0
    );
    context.predefinedRandomValue = 0;
    const battle = {
      movesMap: new Map([
        [
          MoveName.TACKLE,
          {
            name: MoveName.TACKLE,
            effect: MoveEffect.FLINCH_HIT,
            power: 40,
            type: PokemonType.NORMAL,
            accuracy: 100,
            pp: 35,
            effect_chance: 100,
          },
        ],
      ]),
      eventManager,
      gameState,
      context,
    };
    jest.spyOn(damageCalc, "calculateDamage").mockReturnValue({
      damage: 10,
      type_multiplier: new Fraction(1),
    });

    executeMove(battle as any, BattleTurn.PLAYER, attacker, defender, MoveName.TACKLE);

    expect(defender.hp).toBe(20);
    expect(defender.flinching).toBe(false);
  });

  it("leaves the defender at 1 HP when Endure is active", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const attacker = createMockPokemon(30, 30, "ATTACKER");
    const defender = createMockPokemon(20, 20, "DEFENDER");
    attacker.moves = [{ name: MoveName.TACKLE, current_pp: 35 }];
    defender.endure_active = true;
    const context = new BattleContext(
      [attacker],
      [defender],
      attacker,
      defender,
      undefined,
      false,
      undefined,
      0
    );
    context.predefinedRandomValue = 0;
    const battle = {
      movesMap: new Map([
        [
          MoveName.TACKLE,
          {
            name: MoveName.TACKLE,
            effect: MoveEffect.NONE,
            power: 40,
            type: PokemonType.NORMAL,
            accuracy: 100,
            pp: 35,
            effect_chance: 0,
          },
        ],
      ]),
      eventManager,
      gameState,
      context,
    };
    jest.spyOn(damageCalc, "calculateDamage").mockReturnValue({
      damage: 100,
      type_multiplier: new Fraction(1),
    });

    executeMove(battle as any, BattleTurn.PLAYER, attacker, defender, MoveName.TACKLE);

    expect(defender.hp).toBe(1);
  });

  it("publishes an explicit animation label for shared-runtime battle animation dispatch", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const attacker = createMockPokemon(30, 30, "ATTACKER");
    const defender = createMockPokemon(20, 20, "DEFENDER");
    attacker.moves = [{ name: MoveName.RETURN, current_pp: 20 }];
    attacker.happiness = 255;
    const context = new BattleContext(
      [attacker],
      [defender],
      attacker,
      defender,
      undefined,
      false,
      undefined,
      0
    );
    context.predefinedRandomValue = 0;
    const movesMap = new Map([
      [
        MoveName.RETURN,
        {
          name: MoveName.RETURN,
          effect: MoveEffect.RETURN,
          power: 1,
          type: PokemonType.NORMAL,
          accuracy: 100,
          pp: 20,
          effect_chance: 0,
        },
      ],
    ]);
    const battle = { movesMap, eventManager, gameState, context } as any;
    const dispatchSpy = jest.spyOn(eventManager, "dispatch");

    jest.spyOn(damageCalc, "calculateDamage").mockReturnValue({
      damage: 5,
      type_multiplier: new Fraction(1),
    });

    executeMove(battle, BattleTurn.PLAYER, attacker, defender, MoveName.RETURN);

    expect(
      dispatchSpy.mock.calls.some(
        ([event]) =>
          event.name === "play_animation" &&
          event.data?.move_name === MoveName.RETURN &&
          event.data?.animation_label === animation_label_for_move(MoveName.RETURN)
      )
    ).toBe(true);
  });

  it("caps False Swipe so the target is left with at least 1 HP", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const attacker = createMockPokemon(30, 30, "ATTACKER");
    const defender = createMockPokemon(20, 20, "DEFENDER");
    attacker.moves = [{ name: MoveName.FALSE_SWIPE, current_pp: 40 }];
    const context = new BattleContext(
      [attacker],
      [defender],
      attacker,
      defender,
      undefined,
      false,
      undefined,
      0
    );
    context.predefinedRandomValue = 0;
    const movesMap = new Map([
      [
        MoveName.FALSE_SWIPE,
        {
          name: MoveName.FALSE_SWIPE,
          effect: MoveEffect.FALSE_SWIPE,
          power: 40,
          type: PokemonType.NORMAL,
          accuracy: 100,
          pp: 40,
          effect_chance: 0,
        },
      ],
    ]);
    const battle = { movesMap, eventManager, gameState, context } as any;

    jest.spyOn(damageCalc, "calculateDamage").mockReturnValue({
      damage: 50,
      type_multiplier: new Fraction(1),
    });

    executeMove(battle, BattleTurn.PLAYER, attacker, defender, MoveName.FALSE_SWIPE);

    expect(defender.hp).toBe(1);
  });

  it("uses fixed damage for static and level-based attacks", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const attacker = createMockPokemon(30, 30, "ATTACKER");
    const defender = createMockPokemon(50, 50, "DEFENDER");
    attacker.level = 17;
    attacker.moves = [
      { name: MoveName.DRAGON_RAGE, current_pp: 10 },
      { name: MoveName.SEISMIC_TOSS, current_pp: 20 },
    ];
    const context = new BattleContext(
      [attacker],
      [defender],
      attacker,
      defender,
      undefined,
      false,
      undefined,
      0
    );
    context.predefinedRandomValue = 0;
    const movesMap = new Map([
      [
        MoveName.DRAGON_RAGE,
        {
          name: MoveName.DRAGON_RAGE,
          effect: MoveEffect.STATIC_DAMAGE,
          power: 40,
          type: PokemonType.DRAGON,
          accuracy: 100,
          pp: 10,
          effect_chance: 0,
        },
      ],
      [
        MoveName.SEISMIC_TOSS,
        {
          name: MoveName.SEISMIC_TOSS,
          effect: MoveEffect.LEVEL_DAMAGE,
          power: 1,
          type: PokemonType.FIGHTING,
          accuracy: 100,
          pp: 20,
          effect_chance: 0,
        },
      ],
    ]);
    const battle = { movesMap, eventManager, gameState, context } as any;

    jest.spyOn(damageCalc, "calculateDamage").mockReturnValue({
      damage: 1,
      type_multiplier: new Fraction(1),
    });

    executeMove(battle, BattleTurn.PLAYER, attacker, defender, MoveName.DRAGON_RAGE);
    expect(defender.hp).toBe(10);

    defender.hp = 50;
    executeMove(battle, BattleTurn.PLAYER, attacker, defender, MoveName.SEISMIC_TOSS);
    expect(defender.hp).toBe(33);
  });

  it("adjusts Return power from happiness and tracks Pay Day money", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const attacker = createMockPokemon(30, 30, "ATTACKER");
    const defender = createMockPokemon(40, 40, "DEFENDER");
    attacker.level = 12;
    attacker.happiness = 255;
    attacker.moves = [
      { name: MoveName.RETURN, current_pp: 20 },
      { name: MoveName.PAY_DAY, current_pp: 20 },
    ];
    const context = new BattleContext(
      [attacker],
      [defender],
      attacker,
      defender,
      undefined,
      false,
      undefined,
      0
    );
    context.predefinedRandomValue = 0;
    const movesMap = new Map([
      [
        MoveName.RETURN,
        {
          name: MoveName.RETURN,
          effect: MoveEffect.RETURN,
          power: 1,
          type: PokemonType.NORMAL,
          accuracy: 100,
          pp: 20,
          effect_chance: 0,
        },
      ],
      [
        MoveName.PAY_DAY,
        {
          name: MoveName.PAY_DAY,
          effect: MoveEffect.PAY_DAY,
          power: 40,
          type: PokemonType.NORMAL,
          accuracy: 100,
          pp: 20,
          effect_chance: 0,
        },
      ],
    ]);
    const battle = { movesMap, eventManager, gameState, context } as any;
    const calcSpy = jest.spyOn(damageCalc, "calculateDamage");

    calcSpy.mockReturnValueOnce({
      damage: 7,
      type_multiplier: new Fraction(1),
    });
    calcSpy.mockReturnValueOnce({
      damage: 5,
      type_multiplier: new Fraction(1),
    });

    executeMove(battle, BattleTurn.PLAYER, attacker, defender, MoveName.RETURN);
    expect(calcSpy.mock.calls[0]?.[2]).toEqual(
      expect.objectContaining({ power: 102, effect: MoveEffect.RETURN })
    );

    executeMove(battle, BattleTurn.PLAYER, attacker, defender, MoveName.PAY_DAY);
    expect(context.payDayMoney).toBe(60);
  });

  it("stops no-effect moves before damage or secondary effects", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const attacker = createMockPokemon(30, 30, "ATTACKER");
    const defender = createMockPokemon(20, 20, "DEFENDER");
    defender.species = { ...mockSpecies, type1: PokemonType.GROUND };
    attacker.moves = [{ name: MoveName.THUNDER, current_pp: 10 }];
    const context = new BattleContext([attacker], [defender], attacker, defender, undefined, false, undefined, 0);
    context.predefinedRandomValue = 0;
    const movesMap = new Map([
      [MoveName.THUNDER, {
        name: MoveName.THUNDER,
        effect: MoveEffect.THUNDER,
        power: 120,
        type: PokemonType.ELECTRIC,
        accuracy: 100,
        pp: 10,
        effect_chance: 30,
      }],
    ]);
    const battle = { movesMap, eventManager, gameState, context } as any;
    const calcSpy = jest.spyOn(damageCalc, "calculateDamage");

    executeMove(battle, BattleTurn.PLAYER, attacker, defender, MoveName.THUNDER);

    expect(calcSpy).not.toHaveBeenCalled();
    expect(defender.hp).toBe(20);
    expect(defender.status).toBeUndefined();
  });

  it("uses ASM weather accuracy and paralysis handling for Thunder", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const attacker = createMockPokemon(30, 30, "ATTACKER");
    const defender = createMockPokemon(40, 40, "DEFENDER");
    attacker.moves = [{ name: MoveName.THUNDER, current_pp: 10 }];
    const context = new BattleContext([attacker], [defender], attacker, defender, undefined, false, undefined, 0);
    context.weather = Weather.RAIN;
    context.predefinedRandomValue = 1;
    const movesMap = new Map([
      [MoveName.THUNDER, {
        name: MoveName.THUNDER,
        effect: MoveEffect.THUNDER,
        power: 120,
        type: PokemonType.ELECTRIC,
        accuracy: 70,
        pp: 10,
        effect_chance: 0,
      }],
    ]);
    const battle = { movesMap, eventManager, gameState, context } as any;

    jest.spyOn(damageCalc, "calculateDamage").mockReturnValue({
      damage: 5,
      type_multiplier: new Fraction(1),
    });

    executeMove(battle, BattleTurn.PLAYER, attacker, defender, MoveName.THUNDER);

    expect(defender.hp).toBe(35);
    expect(defender.status).toBe(StatusCondition.PARALYSIS);
  });

  it("uses Present's ASM random power table instead of placeholder power", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const attacker = createMockPokemon(30, 30, "ATTACKER");
    const defender = createMockPokemon(40, 40, "DEFENDER");
    attacker.moves = [{ name: MoveName.PRESENT, current_pp: 15 }];
    const context = new BattleContext([attacker], [defender], attacker, defender, undefined, false, undefined, 0);
    context.predefinedRandomValue = 0;
    const movesMap = new Map([
      [MoveName.PRESENT, {
        name: MoveName.PRESENT,
        effect: MoveEffect.PRESENT,
        power: 1,
        type: PokemonType.NORMAL,
        accuracy: 100,
        pp: 15,
        effect_chance: 0,
      }],
    ]);
    const battle = { movesMap, eventManager, gameState, context } as any;
    const calcSpy = jest.spyOn(damageCalc, "calculateDamage").mockReturnValue({
      damage: 6,
      type_multiplier: new Fraction(1),
    });

    executeMove(battle, BattleTurn.PLAYER, attacker, defender, MoveName.PRESENT);

    expect(calcSpy.mock.calls[0]?.[2]).toEqual(expect.objectContaining({ power: 40 }));
    expect(defender.hp).toBe(34);
  });

  it("reflects prior physical damage for Counter", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const attacker = createMockPokemon(30, 30, "ATTACKER");
    const defender = createMockPokemon(40, 40, "DEFENDER");
    attacker.last_damage_taken = 12;
    attacker.last_damage_type = PokemonType.NORMAL;
    attacker.moves = [{ name: MoveName.COUNTER, current_pp: 20 }];
    const context = new BattleContext([attacker], [defender], attacker, defender, undefined, false, undefined, 0);
    context.turnOrder = [BattleTurn.ENEMY, BattleTurn.PLAYER];
    context.predefinedRandomValue = 0;
    const movesMap = new Map([
      [MoveName.COUNTER, {
        name: MoveName.COUNTER,
        effect: MoveEffect.COUNTER,
        power: 1,
        type: PokemonType.FIGHTING,
        accuracy: 100,
        pp: 20,
        effect_chance: 0,
      }],
    ]);
    const battle = { movesMap, eventManager, gameState, context } as any;

    jest.spyOn(damageCalc, "calculateDamage").mockReturnValue({
      damage: 1,
      type_multiplier: new Fraction(1),
    });

    executeMove(battle, BattleTurn.PLAYER, attacker, defender, MoveName.COUNTER);

    expect(defender.hp).toBe(16);
  });
});
