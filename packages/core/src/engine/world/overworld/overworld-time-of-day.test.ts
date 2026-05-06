import { OverworldEngine } from "@pokecrystal/core/engine/world/overworld/overworld";

describe("OverworldEngine time-of-day hooks", () => {
  it("syncs grass rustle and field-move palettes when time-of-day changes", () => {
    const refreshTileset = jest.fn();
    const refreshSprites = jest.fn();
    const normalizeTime = jest.fn().mockReturnValue("nite");
    const setTimeOfDay = jest.fn();
    const setFieldMoveTimeOfDay = jest.fn();

    const handler = (OverworldEngine as unknown as { prototype: { _handle_time_of_day_change: Function } })
      .prototype._handle_time_of_day_change;

    const stub = {
      _refresh_tileset_for_current_map: refreshTileset,
      refresh_map_sprites: refreshSprites,
      _normalise_time_of_day_label: normalizeTime,
      _current_map_attributes: () => null,
      current_map_name: "Test",
      _grass_rustle: { set_time_of_day: setTimeOfDay },
      _field_move_animation_renderer: { set_time_of_day: setFieldMoveTimeOfDay },
    };

    handler.call(stub, "day", "night");

    expect(refreshTileset).toHaveBeenCalled();
    expect(refreshSprites).toHaveBeenCalledWith({ reload_standing: true, reload_walking: true });
    expect(normalizeTime).toHaveBeenCalledWith("night");
    expect(setTimeOfDay).toHaveBeenCalledWith("nite");
    expect(setFieldMoveTimeOfDay).toHaveBeenCalledWith("nite");
  });
});
