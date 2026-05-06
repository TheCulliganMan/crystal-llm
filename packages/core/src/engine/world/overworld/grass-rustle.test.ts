import { GrassRustleController, GrassRustleTarget } from "@pokecrystal/core/engine/world/overworld/grass-rustle";
import { gameEngine } from "@pokecrystal/core/ui/game-engine";

describe("GrassRustleController", () => {
  it("preloads frames with the async image loader", () => {
    const controller = new GrassRustleController({});
    const baseFrames = controller.getBaseFrames();
    expect(Array.isArray(baseFrames)).toBe(true);
    expect(baseFrames).toHaveLength(2);
  });

  it("advances frames every four ticks to match ASM cadence", () => {
    const controller = new GrassRustleController({ uses_palette: false });
    const frameA = controller.getBaseFrames()[0];
    const frameB = controller.getBaseFrames()[1];
    const target = { x: 2, y: 3 };
    const layouts = new Map<GrassRustleTarget, [number, number, number, number]>([
      [target, [0, 0, 8, 8]],
    ]);
    controller.spawn(target, 8);
    let renderables = controller.renderables(layouts);
    expect(renderables[0][2]).toBe(frameA);
    for (let i = 0; i < 4; i += 1) {
      controller.tick();
    }
    renderables = controller.renderables(layouts);
    expect(renderables[0][2]).toBe(frameB);
  });

  it("reuses caller-provided renderable scratch and trims stale entries", () => {
    const controller = new GrassRustleController({ uses_palette: false });
    const targetA = { x: 2, y: 3 };
    const targetB = { x: 1, y: 1 };
    const layouts = new Map<GrassRustleTarget, [number, number, number, number]>([
      [targetA, [0, 0, 8, 8]],
      [targetB, [8, 8, 8, 8]],
    ]);
    const scratch: Array<[number, number, InstanceType<typeof gameEngine.Surface>, [number, number]]> = [];

    controller.spawn(targetA, 1);
    controller.spawn(targetB, 8);

    const first = controller.renderables(layouts, scratch);
    expect(first).toBe(scratch);
    expect(first).toHaveLength(2);

    controller.tick();

    const second = controller.renderables(layouts, scratch);
    expect(second).toBe(scratch);
    expect(second).toHaveLength(1);
    expect(second[0]?.[0]).toBe(1);
    expect(second[0]?.[1]).toBe(1);
  });
});
