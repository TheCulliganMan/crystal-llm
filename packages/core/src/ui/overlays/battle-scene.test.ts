import { Surface } from "@pokecrystal/core/ui/surface";
import { BattleBackgroundTilemap } from "./_battle-background";
import { render_battle_background } from "./battle-scene";
import { _copy_tilemap } from "./battle-ui-core";

describe("battle scene cache", () => {
  it("re-renders when the tilemap revision changes", () => {
    const tilemap = BattleBackgroundTilemap.fromDimensions(1, 1);
    const tileset = {
      0: { 0: new Surface(8, 8) },
      1: { 0: new Surface(8, 8) },
      0x7f: { 0: new Surface(8, 8) },
    };
    tilemap.setTile(0, 0, 0);
    const surface = new Surface(8, 8);
    const blitSpy = jest.spyOn(tilemap, "blit");

    render_battle_background(surface, tilemap, tileset);
    render_battle_background(surface, tilemap, tileset);

    expect(blitSpy).toHaveBeenCalledTimes(1);

    tilemap.setTile(0, 0, 1);
    render_battle_background(surface, tilemap, tileset);

    expect(blitSpy).toHaveBeenCalledTimes(2);
  });

  it("invalidates cache when tilemap arrays are copied", () => {
    const base = BattleBackgroundTilemap.fromDimensions(1, 1);
    const target = BattleBackgroundTilemap.fromDimensions(1, 1);
    base.setTile(0, 0, 1);
    const tileset = {
      0: { 0: new Surface(8, 8) },
      1: { 0: new Surface(8, 8) },
      0x7f: { 0: new Surface(8, 8) },
    };
    const surface = new Surface(8, 8);
    render_battle_background(surface, target, tileset);

    const initialRevision = target.revision;
    _copy_tilemap(base, target);
    expect(target.revision).toBeGreaterThan(initialRevision);

    render_battle_background(surface, target, tileset);
  });

  it("re-renders after ASM ClearBox-style tilemap clears", () => {
    const tilemap = BattleBackgroundTilemap.fromDimensions(1, 1);
    const tileset = {
      0: { 0: new Surface(8, 8) },
      1: { 0: new Surface(8, 8) },
      0x4f: { 0: new Surface(8, 8) },
      0x7f: { 0: new Surface(8, 8) },
    };
    tilemap.setTile(0, 0, 1);
    const surface = new Surface(8, 8);
    const blitSpy = jest.spyOn(tilemap, "blit");

    render_battle_background(surface, tilemap, tileset);
    const initialRevision = tilemap.revision;

    tilemap.clear_box(0, 0, 1, 1);

    expect(tilemap.revision).toBeGreaterThan(initialRevision);

    render_battle_background(surface, tilemap, tileset);

    expect(blitSpy).toHaveBeenCalledTimes(2);
  });
});
