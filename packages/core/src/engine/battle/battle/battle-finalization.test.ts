import { createInitialGameState } from "@pokecrystal/core/core/state";
import { EventManager } from "@pokecrystal/core/engine/events/events";
import { PokemonSpeciesSchema, createPokemon } from "@pokecrystal/core/core/models/pokemon";
import { MAX_MONEY } from "@pokecrystal/core/core/constants";
import { Battle } from "./battle-logic";
import { BattleTurn, MoveName, Stat, StatusCondition } from "@pokecrystal/core/core/enums";
import { finaliseBattle, handleFaint } from "./battle-finalization";

const buildSpecies = () =>
  PokemonSpeciesSchema.parse({
    id: "TOTODILE",
    int_id: 158,
    base_stats: {
      hp: 50,
      attack: 65,
      defense: 64,
      speed: 43,
      special_attack: 44,
      special_defense: 48,
    },
    type1: "WATER",
    type2: "WATER",
    catch_rate: 45,
    base_exp: 66,
    gender_ratio: 31,
    unknown1: 0,
    step_cycles_to_hatch: 20,
    unknown2: 0,
    growth_rate: "GROWTH_MEDIUM_SLOW",
    egg_group1: "EGG_MONSTER",
    egg_group2: "EGG_WATER_1",
  });

const buildBattle = () => {
  const gameState = createInitialGameState();
  const eventManager = new EventManager(gameState);
  const species = buildSpecies();
  const playerPokemon = createPokemon(gameState, species, 5);
  const enemyPokemon = createPokemon(gameState, species, 5);
  const movesMap = new Map();
  const battle = new Battle(
    playerPokemon,
    enemyPokemon,
    gameState,
    eventManager,
    null as any,
    movesMap,
    null,
    undefined,
    [playerPokemon],
    [enemyPokemon],
  );
  return { battle, eventManager };
};

const collectTexts = (eventManager: EventManager): string[] => {
  const texts: string[] = [];
  eventManager.on("show_text", (event) => {
    const payload = event.data as { text?: string };
    if (payload.text) {
      texts.push(payload.text);
    }
  });
  return texts;
};

const defeatEnemyParty = (battle: Battle): void => {
  for (const pokemon of battle.context.enemyParty) {
    pokemon.hp = 0;
  }
  battle.context.enemyPokemon.hp = 0;
};

describe("finaliseBattle", () => {
  it("prepares overworld resume before dispatching battle_complete", () => {
    const { battle, eventManager } = buildBattle();
    const prepareSpy = jest.spyOn(battle, "prepareForOverworldResume");
    let preparedAtDispatch = false;

    eventManager.on("battle_complete", () => {
      preparedAtDispatch = prepareSpy.mock.calls.length > 0;
    });

    finaliseBattle(battle);

    expect(prepareSpy).toHaveBeenCalledTimes(1);
    expect(preparedAtDispatch).toBe(true);
  });

  it("clears Disable from the saved party when battle ends", () => {
    const { battle } = buildBattle();
    const player = battle.context.playerPokemon;
    (player as { _sram_slot?: number })._sram_slot = 0;
    battle.gameState.sram.party.pokemon[0] = player;
    player.disabled_move = MoveName.BITE;
    player.disable_turns = 6;

    finaliseBattle(battle);

    expect(player.disabled_move).toBeUndefined();
    expect(player.disable_turns).toBe(0);
    expect(battle.gameState.sram.party.pokemon[0]?.disabled_move).toBeUndefined();
    expect(battle.gameState.sram.party.pokemon[0]?.disable_turns).toBe(0);
  });

  it("clears battle stat stages without curing persistent status when battle ends", () => {
    const { battle } = buildBattle();
    const player = battle.context.playerPokemon;
    (player as { _sram_slot?: number })._sram_slot = 0;
    battle.gameState.sram.party.pokemon[0] = player;
    player.status = StatusCondition.POISON;
    player.stat_boosts[Stat.DEFENSE] = -2;
    player.stat_boosts[Stat.ACCURACY] = -1;
    player.stat_boosts[Stat.EVASION] = 3;

    finaliseBattle(battle);

    expect(player.status).toBe(StatusCondition.POISON);
    expect(player.stat_boosts[Stat.DEFENSE]).toBe(0);
    expect(player.stat_boosts[Stat.ACCURACY]).toBe(0);
    expect(player.stat_boosts[Stat.EVASION]).toBe(0);
    expect(battle.gameState.sram.party.pokemon[0]?.stat_boosts[Stat.DEFENSE]).toBe(0);
    expect(battle.gameState.sram.party.pokemon[0]?.stat_boosts[Stat.ACCURACY]).toBe(0);
  });

  it("uses ASM trainer payout scaling (base * level * 4)", () => {
    const { battle } = buildBattle();
    defeatEnemyParty(battle);
    battle.context.trainerBattle = true;
    battle.context.trainerReward = 225;

    finaliseBattle(battle);

    expect(battle.gameState.sram.money).toBe(900);
  });

  it("applies Amulet Coin to trainer payout before ASM scaling", () => {
    const { battle } = buildBattle();
    defeatEnemyParty(battle);
    battle.context.trainerBattle = true;
    battle.context.trainerReward = 225;
    battle.context.amuletCoinActive = true;

    finaliseBattle(battle);

    expect(battle.gameState.sram.money).toBe(1800);
  });

  it("sends one share to MOM when saving some money", () => {
    const { battle, eventManager } = buildBattle();
    const texts = collectTexts(eventManager);
    defeatEnemyParty(battle);
    battle.context.trainerBattle = true;
    battle.context.trainerReward = 200;
    battle.gameState.sram.mom_saving_some_money = true;
    battle.gameState.sram.money = 1000;
    battle.gameState.sram.moms_money = 50;

    finaliseBattle(battle);

    expect(battle.gameState.sram.money).toBe(1600);
    expect(battle.gameState.sram.moms_money).toBe(250);
    expect(texts).toContain("Sent some to MOM!");
  });

  it("keeps all trainer payout when MOM account is maxed out", () => {
    const { battle, eventManager } = buildBattle();
    const texts = collectTexts(eventManager);
    defeatEnemyParty(battle);
    battle.context.trainerBattle = true;
    battle.context.trainerReward = 200;
    battle.gameState.sram.mom_saving_some_money = true;
    battle.gameState.sram.moms_money = MAX_MONEY;

    finaliseBattle(battle);

    expect(battle.gameState.sram.money).toBe(800);
    expect(battle.gameState.sram.moms_money).toBe(MAX_MONEY);
    expect(texts).not.toContain("Sent some to MOM!");
  });

  it("runs trainer payout text before Pay Day pickup text", () => {
    const { battle, eventManager } = buildBattle();
    const texts = collectTexts(eventManager);
    defeatEnemyParty(battle);
    battle.context.trainerBattle = true;
    battle.context.trainerReward = 100;
    battle.context.payDayMoney = 50;
    battle.context.amuletCoinActive = true;

    finaliseBattle(battle);

    expect(battle.gameState.sram.money).toBe(900);
    const trainerTextIndex = texts.indexOf(`${battle.gameState.sram.player_name} got ¥800\nfor winning!`);
    const payDayTextIndex = texts.indexOf("You picked up ¥100!");
    expect(trainerTextIndex).toBeGreaterThanOrEqual(0);
    expect(payDayTextIndex).toBeGreaterThan(trainerTextIndex);
  });

  it("does not pay Pay Day money after a loss", () => {
    const { battle, eventManager } = buildBattle();
    const texts = collectTexts(eventManager);
    battle.context.payDayMoney = 75;
    battle.context.playerPokemon.hp = 0;
    battle.context.playerParty[0].hp = 0;

    finaliseBattle(battle);

    expect(battle.gameState.sram.money).toBe(0);
    expect(texts).not.toContain("You picked up ¥75!");
  });

  it("treats simultaneous all-party defeat as a loss before trainer payout", () => {
    const { battle, eventManager } = buildBattle();
    const texts = collectTexts(eventManager);
    const completeSpy = jest.fn();
    eventManager.on("battle_complete", completeSpy);
    battle.context.trainerBattle = true;
    battle.context.trainerReward = 100;
    battle.context.playerPokemon.hp = 0;
    battle.context.playerParty[0].hp = 0;
    defeatEnemyParty(battle);

    finaliseBattle(battle);

    expect(battle.gameState.wram.battle_result).toBe(1);
    expect(completeSpy).toHaveBeenCalledWith(
      expect.objectContaining({ data: expect.objectContaining({ result: 1 }) }),
      battle.gameState
    );
    expect(battle.gameState.sram.money).toBe(0);
    expect(texts).not.toContain(`${battle.gameState.sram.player_name} got ¥400\nfor winning!`);
  });

  it("restores a transformed player pokemon before party data is written back", () => {
    const { battle } = buildBattle();
    defeatEnemyParty(battle);
    const player = battle.context.playerPokemon;
    (player as { _sram_slot?: number })._sram_slot = 0;
    battle.gameState.sram.party.pokemon[0] = player;
    player.transformed = true;
    player.original_species = { ...player.species, id: "DITTO" };
    player.transform_backup_dvs = { attack: 1, defense: 2, speed: 3, special: 4, hp: 5 };
    player.transform_backup_moves = [{ name: MoveName.TRANSFORM, current_pp: 9 }];
    player.transform_backup_stat_boosts = {
      HP: 0,
      ATTACK: 0,
      DEFENSE: 0,
      SPEED: 0,
      SPECIAL_ATTACK: 0,
      SPECIAL_DEFENSE: 0,
      ACCURACY: 0,
      EVASION: 0,
    };
    player.transform_backup_stats = {
      attack: 10,
      defense: 11,
      speed: 12,
      special_attack: 13,
      special_defense: 14,
    };
    player.species = { ...player.species, id: "MEW" };
    player.moves = [{ name: MoveName.PSYCHIC_M, current_pp: 5 }];

    finaliseBattle(battle);

    expect(player.transformed).toBe(false);
    expect(player.species.id).toBe("DITTO");
    expect(player.moves).toEqual([{ name: MoveName.TRANSFORM, current_pp: 9 }]);
    expect(battle.gameState.sram.party.pokemon[0]?.species?.id).toBe("DITTO");
  });
});

describe("handleFaint", () => {
  it("dispatches the player fainted text", () => {
    const { battle, eventManager } = buildBattle();
    const texts: string[] = [];
    eventManager.on("show_text", (event) => {
      const payload = event.data as { text?: string };
      if (payload.text) {
        texts.push(payload.text);
      }
    });

    battle.context.playerPokemon.hp = 0;
    handleFaint(battle, BattleTurn.PLAYER);

    expect(texts).toContain(`${battle.context.playerPokemon.nickname} fainted!`);
  });

  it("dispatches the enemy fainted text", () => {
    const { battle, eventManager } = buildBattle();
    const texts: string[] = [];
    eventManager.on("show_text", (event) => {
      const payload = event.data as { text?: string };
      if (payload.text) {
        texts.push(payload.text);
      }
    });

    battle.context.enemyPokemon.hp = 0;
    handleFaint(battle, BattleTurn.ENEMY);

    expect(texts).toContain(`Enemy ${battle.context.enemyPokemon.nickname} fainted!`);
  });
});
