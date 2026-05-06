import { BattleAnimObjectDef } from './_battle-anim-data';
import { adjust_enemy_object_coords, build_command_handlers, mirror_enemy_x } from './battle-animation-helpers';

const makeHandler = (name: string, calls: string[], contexts: unknown[]) =>
  function handler(this: unknown) {
    calls.push(name);
    contexts.push(this);
  };

const makePlayerProxy = () => {
  const calls: string[] = [];
  const contexts: unknown[] = [];
  const handlers = new Map<string, (...args: unknown[]) => unknown>();
  const proxy = new Proxy(
    {},
    {
      get(_target, prop: string | symbol) {
        if (typeof prop !== "string") {
          return undefined;
        }
        if (prop.startsWith("_handle_")) {
          return undefined;
        }
        if (prop.startsWith("handle_")) {
          if (!handlers.has(prop)) {
            handlers.set(prop, makeHandler(prop, calls, contexts));
          }
          return handlers.get(prop);
        }
        return undefined;
      },
    }
  );
  return { proxy, calls, contexts };
};

describe("build_command_handlers", () => {
  it("falls back to handle_* methods when _handle_* is missing", () => {
    const { proxy, calls, contexts } = makePlayerProxy();
    const handlers = build_command_handlers(proxy as Record<string, (...args: unknown[]) => unknown>);

    expect(typeof handlers.anim_wait).toBe("function");
    expect(typeof handlers.anim_1gfx).toBe("function");
    expect(typeof handlers.anim_beatup).toBe("function");

    const command = { command: "noop", args: [] as string[] };
    handlers.anim_wait(command);
    handlers.anim_1gfx(command);
    handlers.anim_beatup(command);

    expect(calls).toEqual(expect.arrayContaining(["handle_wait", "handle_gfx", "handle_noop"]));
    expect(contexts).toEqual(expect.arrayContaining([proxy, proxy, proxy]));
  });
});

describe('battle animation math helpers', () => {
  it('mirrors enemy X with 8-bit wraparound', () => {
    expect(mirror_enemy_x(0xb4)).toBe(0x00);
    expect(mirror_enemy_x(0xb5)).toBe(0xff);
    expect(mirror_enemy_x(0x00)).toBe(0xb4);
    expect(mirror_enemy_x(0x14)).toBe(0xa0);
  });

  it('keeps 8-bit subtraction semantics for enemy Y adjustment', () => {
    const objDef = {
      flags: 0x01,
      fix_y: 0x00,
    } as unknown as BattleAnimObjectDef;
    const [, y] = adjust_enemy_object_coords(0x20, 0x10, objDef, 0, 0x00);
    expect(y).toBe(0xf0);
  });
});
