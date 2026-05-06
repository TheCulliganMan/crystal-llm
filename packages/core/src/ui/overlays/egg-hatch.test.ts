import { EggHatchAnimation } from "./egg-hatch";
import { Surface } from "@pokecrystal/core/ui/game-engine";

describe("EggHatchAnimation audio routing", () => {
  it("plays evolution music without overwriting remembered map music", () => {
    const playMusic = jest.fn();
    const ui = {
      loadSprite: jest.fn(),
      _getPokemonFrameSurface: jest.fn(() => ({
        get_width: () => 56,
        get_height: () => 56,
      })),
    } as any;

    new EggHatchAnimation(ui, {
      speciesId: "TOGEPI",
      audioEngine: { playMusic } as any,
    });

    expect(playMusic).toHaveBeenCalledWith("MUSIC_EVOLUTION", "evolution");
    expect(playMusic).not.toHaveBeenCalledWith("MUSIC_EVOLUTION", "map");
  });

  it("draws the reveal frame with headless sprite surfaces", () => {
    const sprite = new Surface(8, 8);
    sprite.fill([85, 170, 255, 255]);
    const screen = new Surface(160, 144);
    const ui = {
      loadSprite: jest.fn(),
      _getPokemonFrameSurface: jest.fn(() => sprite),
    } as any;
    const animation = new EggHatchAnimation(ui, { speciesId: "TOGEPI" });

    for (let frame = 0; frame < 16 + 56 + 34 + 1; frame += 1) {
      animation.advance();
    }

    expect(() => animation.draw(screen)).not.toThrow();
  });
});
