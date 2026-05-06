import { createInitialGameState } from "@pokecrystal/core/core/state";
import { PlayerState } from "@pokecrystal/core/core/enums/overworld";
import { refresh_sprites, update_sprites } from "./sprites";

describe("sprite special events", () => {
  const buildOverworld = () => ({
    player_state: PlayerState.NORMAL,
    player_sprite_id: "chris",
    player_palette_id: 0,
    _create_player_animations: jest.fn(() => ({})),
  });

  it("binds overworld when reloading sprites without palette changes", () => {
    const gameState = createInitialGameState();
    const overworld = buildOverworld() as any;
    const reload = jest.fn(function (this: unknown, options?: unknown) {
      (this as any).reloadContext = this;
      (this as any).reloadOptions = options ?? null;
    });
    overworld.reload_sprites_without_palette_changes = reload;

    expect(() =>
      update_sprites(gameState, { overworld })
    ).not.toThrow();

    expect(overworld.reloadContext).toBe(overworld);
    expect(overworld.reloadOptions).toEqual({
      reload_standing: true,
      reload_walking: true,
    });
  });

  it("binds overworld when refreshing map sprites", () => {
    const gameState = createInitialGameState();
    const overworld = buildOverworld() as any;
    const refresh = jest.fn(function (this: unknown, options?: unknown) {
      (this as any).refreshContext = this;
      (this as any).refreshOptions = options ?? null;
    });
    overworld.refresh_map_sprites = refresh;

    expect(() =>
      refresh_sprites(gameState, { overworld })
    ).not.toThrow();

    expect(overworld.refreshContext).toBe(overworld);
    expect(overworld.refreshOptions).toEqual({
      reload_standing: true,
      reload_walking: true,
    });
  });
});
