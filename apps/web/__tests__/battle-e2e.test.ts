import { createInitialGameState } from "@/core/state";
import { EventManager } from "@/engine/events/events";
import { createPokemon, PokemonSchema, PokemonSpeciesSchema } from "@/core/models/pokemon";
import {
  Ability,
  BattleActionType,
  BattleTurn,
  EggGroup,
  GenderRatio,
  GrowthRate,
  MoveEffect,
  MoveName,
  PokemonType,
} from "@/core/enums";
import { Battle } from "@/engine/battle/battle/battle-logic";
import type { BattleUIState } from "@/ui/overlays/battle-ui-state";
import { TrainerSchema } from "@/core/models/trainer";
import type { ScriptRunner } from "@/engine/world/story-events/runner";
import { LoadTrainerCommand, StartBattleCommand } from "@/engine/world/story-events/commands/battle";

const buildSpecies = (overrides: Partial<ReturnType<typeof PokemonSpeciesSchema.parse>> = {}) =>
  PokemonSpeciesSchema.parse({
    id: "TESTMON",
    int_id: 1,
    base_stats: {
      hp: 60,
      attack: 60,
      defense: 60,
      speed: 60,
      special_attack: 60,
      special_defense: 60,
    },
    type1: PokemonType.NORMAL,
    type2: PokemonType.NORMAL,
    catch_rate: 255,
    base_exp: 1,
    gender_ratio: GenderRatio.GENDER_F50,
    unknown1: 0,
    step_cycles_to_hatch: 0,
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
    ...overrides,
    base_stats: {
      hp: 60,
      attack: 60,
      defense: 60,
      speed: 60,
      special_attack: 60,
      special_defense: 60,
      ...(overrides.base_stats ?? {}),
    },
  });

describe("Battle end-to-end", () => {
  it("runs a full turn and finalizes the battle", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);

    const playerSpecies = buildSpecies({
      id: "PLAYERMON",
      base_stats: { speed: 30 },
    });
    const enemySpecies = buildSpecies({
      id: "ENEMYMON",
      base_stats: { speed: 200 },
    });

    const player = createPokemon(gameState, playerSpecies, 5);
    const enemy = createPokemon(gameState, enemySpecies, 5);

    player.moves = [{ name: MoveName.TACKLE, current_pp: 35 }];
    enemy.moves = [{ name: MoveName.TACKLE, current_pp: 35 }];

    player.max_hp = 100;
    player.hp = 100;
    enemy.max_hp = 1;
    enemy.hp = 1;

    const movesMap = new Map([
      [
        MoveName.TACKLE,
        {
          name: MoveName.TACKLE,
          type: PokemonType.NORMAL,
          power: 50,
          accuracy: 100,
          pp: 35,
          effect: MoveEffect.NORMAL_HIT,
          effect_chance: 0,
        },
      ],
    ]);

    const battle = new Battle(
      player,
      enemy,
      gameState,
      eventManager,
      null as unknown as BattleUIState,
      movesMap
    );
    battle.context.predefinedRandomValue = 0;
    battle.queuePlayerAction({
      actionType: BattleActionType.MOVE,
      moveName: MoveName.TACKLE,
    });

    let safety = 0;
    while (!battle.isFinished() && safety < 20) {
      battle.update();
      safety += 1;
    }

    expect(battle.isFinished()).toBe(true);
    expect(battle.context.enemyPokemon.hp).toBe(0);
    expect(gameState.wram.battle_result).toBe(0);
  });

  it("runs a rival trainer battle end-to-end with null trainer payload defaults", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const movesMap = new Map([
      [
        MoveName.TACKLE,
        {
          name: MoveName.TACKLE,
          type: PokemonType.NORMAL,
          power: 50,
          accuracy: 100,
          pp: 35,
          effect: MoveEffect.NORMAL_HIT,
          effect_chance: 0,
        },
      ],
    ]);

    const playerSpecies = buildSpecies({ id: "PLAYERMON" });
    const rivalSpecies = buildSpecies({ id: "RIVALMON", base_stats: { hp: 10, defense: 5 } });
    const player = createPokemon(gameState, playerSpecies, 5);

    player.moves = [{ name: MoveName.TACKLE, current_pp: 35 }];
    player.max_hp = 30;
    player.hp = 30;
    gameState.sram.party!.pokemon[0] = player;

    const rivalPokemonPayload = {
      species: rivalSpecies,
      nickname: "RIVALMON",
      level: 4,
      hp: 5,
      max_hp: 5,
      original_trainer_name: "RIVAL",
      original_trainer_id: 1,
      experience: 0,
      happiness: 0,
      status: null,
      stat_boosts: {},
      locked_move: null,
      trapped_by_side: null,
      trapped_source_index: null,
      trapped_move: null,
      leech_seed_source_side: null,
      curse_source_side: null,
      last_move_used: null,
      encored_move: null,
      destiny_bond_action_id: null,
    };
    const rivalPokemon = PokemonSchema.parse(rivalPokemonPayload);

    expect(rivalPokemon.status).toBeUndefined();
    expect(rivalPokemon.stat_boosts.HP).toBe(0);
    expect(rivalPokemon.trapped_by_side).toBeUndefined();

    const rivalTrainer = TrainerSchema.parse({
      name: "RIVAL1_1_TOTODILE",
      trainer_id: "RIVAL1_1_TOTODILE",
      trainer_class: "RIVAL1",
      party: [rivalPokemon],
      win_quote: "You lose!",
      lose_quote: "You win.",
    });
    const trainerMap = new Map([[rivalTrainer.name, rivalTrainer]]);
    const dataLoader = {
      get_trainer(name: string) {
        return trainerMap.get(name);
      },
      get_trainer_base_reward() {
        return 1;
      },
    };

    const runner: ScriptRunner = {
      data_loader: dataLoader,
      variables: {},
      string_buffers: {},
      last_condition_result: false,
      last_value: null,
      loaded_trainer: null,
      loaded_trainer_id: null,
      just_battled: false,
      pause: jest.fn(),
      resume: jest.fn(),
      stop_all_scripts: jest.fn(),
      _set_map_scene: jest.fn((mapName: string, sceneName: string) => {
        gameState.wram.map_scenes[mapName] = sceneName;
      }),
    } as ScriptRunner;
    const overworld = { current_map_name: "CherrygroveCity" };
    gameState.wram.map_scenes["CherrygroveCity"] = "SCENE_CHERRYGROVECITY_MEET_RIVAL";

    eventManager.on("start_battle", (event) => {
      const { player_pokemon, enemy_pokemon } = event.data;
      player_pokemon.moves = [{ name: MoveName.TACKLE, current_pp: 35 }];
      enemy_pokemon.moves = [{ name: MoveName.TACKLE, current_pp: 35 }];
      enemy_pokemon.max_hp = 1;
      enemy_pokemon.hp = 1;
      const battle = new Battle(
        player_pokemon,
        enemy_pokemon,
        gameState,
        eventManager,
        null as unknown as BattleUIState,
        movesMap
      );
      battle.context.predefinedRandomValue = 0;
      battle.queuePlayerAction({
        actionType: BattleActionType.MOVE,
        moveName: MoveName.TACKLE,
      });

      let safety = 0;
      while (!battle.isFinished() && safety < 50) {
        battle.update();
        safety += 1;
      }
    });

    const loadTrainer = new LoadTrainerCommand("RIVAL1", "RIVAL1_1_TOTODILE");
    loadTrainer.runner = runner;
    loadTrainer.execute(gameState, eventManager, { data_loader: dataLoader });

    const startBattle = new StartBattleCommand();
    startBattle.runner = runner;
    startBattle.execute(gameState, eventManager, overworld);

    expect(runner.pause).toHaveBeenCalled();
    expect(runner.resume).toHaveBeenCalled();
    expect(gameState.wram.other_trainer_id).toBe("RIVAL1_1_TOTODILE");
    expect(gameState.wram.battle_result).toBe(0);
    expect(gameState.wram.map_scenes["CherrygroveCity"]).toBe("SCENE_CHERRYGROVECITY_NOOP");
    expect(runner.loaded_trainer).toBeNull();
    expect(runner.loaded_trainer_id).toBeNull();
    expect(runner.just_battled).toBe(true);
  });
});
