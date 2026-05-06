import { createInitialGameState } from "@pokecrystal/core/core/state";
import { EventManager } from "@pokecrystal/core/engine/events/events";
import { createPokemon, PokemonSpeciesSchema } from "@pokecrystal/core/core/models/pokemon";
import { GrowthRate, GenderRatio, PokemonType, EggGroup, Ability } from "@pokecrystal/core/core/enums";
import { Battle } from "./battle-logic";
import { BattleContext } from "./battle-context";
import { attemptRun } from "./flee-logic";

const buildSpecies = (speed = 100) =>
  PokemonSpeciesSchema.parse({
    id: "CHIKORITA",
    int_id: 152,
    base_stats: {
      hp: 45,
      attack: 49,
      defense: 65,
      speed,
      special_attack: 49,
      special_defense: 65,
    },
    type1: PokemonType.NORMAL,
    type2: PokemonType.NORMAL,
    catch_rate: 45,
    base_exp: 64,
    gender_ratio: GenderRatio.GENDER_F50,
    unknown1: 0,
    step_cycles_to_hatch: 20,
    unknown2: 0,
    growth_rate: GrowthRate.GROWTH_MEDIUM_SLOW,
    egg_group1: EggGroup.EGG_MONSTER,
    egg_group2: EggGroup.EGG_MONSTER,
    tmhm_learnset: [],
    evolutions: null,
    ability: Ability.NONE,
    pic_size: 0,
    front_pic: 0,
    back_pic: 0,
    front_palette: 0,
  });

const buildBattle = (playerSpeed = 100, enemySpeed = 100, playerItem?: string) => {
  const gameState = createInitialGameState();
  const player = createPokemon(gameState, buildSpecies(playerSpeed), 50);
  const enemy = createPokemon(gameState, buildSpecies(enemySpeed), 50);
  player.item = playerItem;

  const context = new BattleContext(
    [player],
    [enemy],
    player,
    enemy,
    undefined,
    false,
    undefined,
    0
  );
  const eventManager = new EventManager(gameState);

  const battle = ({ gameState, context, eventManager } as unknown) as Battle;
  return { battle, context, player, enemy, eventManager };
};

const captureBattleTexts = (eventManager: EventManager): string[] => {
  const texts: string[] = [];
  eventManager.on("show_text", (event) => {
    const payload = event.data as { text?: string };
    if (typeof payload?.text === "string") {
      texts.push(payload.text);
    }
  });
  return texts;
};

describe("attemptRun", () => {
  it("escapes immediately with a held escape item by script key", () => {
    const { battle, player, eventManager } = buildBattle(100, 120, "SMOKE_BALL");
    const texts = captureBattleTexts(eventManager);

    const success = attemptRun(battle);

    expect(success).toBe(true);
    expect(player.item).toBe("SMOKE_BALL");
    expect(texts).toContain(`${player.nickname} fled using a SMOKE BALL!`);
  });

  it("can still fail run attempts without quick escape item when player is slower", () => {
    const { battle, context, eventManager } = buildBattle(1, 255);
    const texts = captureBattleTexts(eventManager);

    const success = attemptRun(battle);

    expect(success).toBe(false);
    expect(context.playerRunAttempts).toBe(1);
    expect(texts).toContain("Can't escape!");
  });
});
