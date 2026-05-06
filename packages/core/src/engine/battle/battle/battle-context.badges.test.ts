import type { Pokemon } from '@pokecrystal/core/core/models';
import { BattleTurn, Stat } from '@pokecrystal/core/core/enums';
import { BattleContext } from './battle-context';

const makePokemon = (): Pokemon => ({ hp: 100 } as unknown as Pokemon);

const makeContext = (): BattleContext => {
  const player = makePokemon();
  const enemy = makePokemon();
  return new BattleContext([player], [enemy], player, enemy, undefined, false, undefined, 0);
};

const johto = (...owned: number[]): boolean[] =>
  Array.from({ length: 8 }, (_unused, idx) => owned.includes(idx));

describe("BattleContext badge boosts", () => {
  it("maps Zephyr badge to ATTACK", () => {
    const context = makeContext();
    context.setBadgeBoostState(johto(0));

    expect(context.badgeBoostActive(BattleTurn.PLAYER, Stat.ATTACK)).toBe(true);
    expect(context.badgeBoostActive(BattleTurn.PLAYER, Stat.DEFENSE)).toBe(false);
  });

  it("applies ASM Plain/Mineral swap (Plain->SPEED, Mineral->DEFENSE)", () => {
    const plainContext = makeContext();
    plainContext.setBadgeBoostState(johto(2));
    expect(plainContext.badgeBoostActive(BattleTurn.PLAYER, Stat.SPEED)).toBe(true);
    expect(plainContext.badgeBoostActive(BattleTurn.PLAYER, Stat.DEFENSE)).toBe(false);

    const mineralContext = makeContext();
    mineralContext.setBadgeBoostState(johto(4));
    expect(mineralContext.badgeBoostActive(BattleTurn.PLAYER, Stat.DEFENSE)).toBe(true);
    expect(mineralContext.badgeBoostActive(BattleTurn.PLAYER, Stat.SPEED)).toBe(false);
  });

  it("maps Glacier badge to both special stats", () => {
    const context = makeContext();
    context.setBadgeBoostState(johto(6));

    expect(context.badgeBoostActive(BattleTurn.PLAYER, Stat.SPECIAL_ATTACK)).toBe(true);
    expect(context.badgeBoostActive(BattleTurn.PLAYER, Stat.SPECIAL_DEFENSE)).toBe(true);
  });

  it("never applies player badge boosts to enemy side", () => {
    const context = makeContext();
    context.setBadgeBoostState(johto(0, 2, 4, 6));

    expect(context.badgeBoostActive(BattleTurn.ENEMY, Stat.ATTACK)).toBe(false);
  });

  it("disables boosts in link mode and Battle Tower", () => {
    const linkContext = makeContext();
    linkContext.setBadgeBoostState(johto(0, 2, 4, 6), { linkMode: true });
    expect(linkContext.badgeBoostActive(BattleTurn.PLAYER, Stat.ATTACK)).toBe(false);

    const towerContext = makeContext();
    towerContext.setBadgeBoostState(johto(0, 2, 4, 6), { inBattleTowerBattle: true });
    expect(towerContext.badgeBoostActive(BattleTurn.PLAYER, Stat.ATTACK)).toBe(false);
  });

  it("throws when Johto badge bank is not ASM-sized", () => {
    const context = makeContext();
    expect(() => context.setBadgeBoostState([true] as boolean[])).toThrow("must contain exactly 8");
  });
});
