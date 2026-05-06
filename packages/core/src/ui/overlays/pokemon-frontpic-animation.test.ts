import {
  ensure_frontpic_anim_program,
  FrontpicAnimator,
  is_frontpic_anim_program_pending,
  parse_frontpic_anim_script,
  resolve_frontpic_anim_program,
} from "./pokemon-frontpic-animation";

describe("pokemon-frontpic-animation", () => {
  afterEach(() => {
    jest.restoreAllMocks();
    delete (globalThis as { window?: unknown }).window;
    delete (globalThis as { fetch?: unknown }).fetch;
  });

  it("steps repeat loops using asm dorepeat targets", () => {
    const program = parse_frontpic_anim_script(`
      frame 1, 01
      setrepeat 2
      frame 0, 01
      frame 2, 01
      dorepeat 2
      endanim
    `);
    const animator = new FrontpicAnimator(program, 0);
    const frames: number[] = [];
    let guard = 0;
    while (!animator.complete && guard < 20) {
      const result = animator.step();
      frames.push(result.frame);
      guard += 1;
    }
    expect(frames.filter((frame, index) => index === 0 || frame !== frames[index - 1])).toEqual([
      1, 0, 2, 0, 2,
    ]);
    expect(animator.complete).toBe(true);
  });

  it("loads frontpic scripts from the generated JSON bundle", () => {
    expect(ensure_frontpic_anim_program("pikachu")).toBe(true);
    const program = resolve_frontpic_anim_program("pikachu");
    expect(program).not.toBeNull();
    expect(program?.commands.length).toBeGreaterThan(0);
    expect(is_frontpic_anim_program_pending("pikachu")).toBe(false);
  });
});
