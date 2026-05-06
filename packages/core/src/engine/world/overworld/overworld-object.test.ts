import { OverworldObject } from "./overworld-object";

type OverworldObjectAlias = OverworldObject & {
  pixel_x: number;
  pixel_y: number;
  target_pixel_x: number;
  target_pixel_y: number;
  sprite_y_offset: number;
};

describe("OverworldObject", () => {
  it("keeps snake_case alias fields in sync after pixel updates", () => {
    const obj = new OverworldObject({
      sprite: "SPRITE_MOM",
      x: 3,
      y: 5,
      spritemovedata: "SPRITEMOVEDATA_STANDING_LEFT",
      move_range_x: 0,
      move_range_y: 0,
      hram_x: -1,
      hram_y: -1,
      pal: 0,
      object_type: "OBJECTTYPE_SCRIPT",
      radius: 0,
      script: "MomScript",
      event_flag: "EVENT_NONE",
      object_identifier: null,
      sightline_direction_override: null,
    });

    const alias = obj as OverworldObjectAlias;
    alias.pixel_x = 0;
    alias.pixel_y = 0;
    alias.target_pixel_x = 0;
    alias.target_pixel_y = 0;
    alias.sprite_y_offset = 5;

    obj.updatePixelPosition();

    expect(alias.pixel_x).toBe(obj.pixelX);
    expect(alias.pixel_y).toBe(obj.pixelY);
    expect(alias.target_pixel_x).toBe(obj.targetPixelX);
    expect(alias.target_pixel_y).toBe(obj.targetPixelY);
    expect(alias.sprite_y_offset).toBe(obj.spriteYOffset);
  });

  it("advances jump_step by two tiles to match scripted jump steps", () => {
    const obj = new OverworldObject({
      sprite: "SPRITE_MOM",
      x: 2,
      y: 2,
      spritemovedata: "SPRITEMOVEDATA_STANDING_LEFT",
      move_range_x: 0,
      move_range_y: 0,
      hram_x: -1,
      hram_y: -1,
      pal: 0,
      object_type: "OBJECTTYPE_SCRIPT",
      radius: 0,
      script: "MomScript",
      event_flag: "EVENT_NONE",
      object_identifier: null,
      sightline_direction_override: null,
    });

    obj.applyMovement(["jump_step DOWN", "step_end"]);

    expect(obj.y).toBe(6);
    expect(obj.pixelY).toBe(40);
  });
});
