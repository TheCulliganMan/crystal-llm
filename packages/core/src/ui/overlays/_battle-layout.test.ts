import { BattleUILayoutFactory, DEFAULT_TILE_SIZE } from "./_battle-layout";

describe("BattleUILayoutFactory ASM defaults", () => {
  it("places the enemy level at the ASM DrawEnemyHUD coordinate", () => {
    const layout = BattleUILayoutFactory.fromAsmDefaults();

    // ASM: engine/battle/core.asm::DrawEnemyHUD uses hlcoord 6, 1 before PrintLevel.
    expect(layout.enemy_hud.level_position).toEqual([
      6 * DEFAULT_TILE_SIZE,
      1 * DEFAULT_TILE_SIZE,
    ]);
  });
});
