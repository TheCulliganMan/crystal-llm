import { AnimationObjectTable, AnimationSpriteSchema } from './battle-animation-state';

describe('AnimationObjectTable', () => {
  afterEach(() => {
    jest.restoreAllMocks();
  });

  it('returns null when full without warning', () => {
    const warn = jest.spyOn(console, 'warn').mockImplementation(() => {});
    const table = new AnimationObjectTable(2);

    const first = AnimationSpriteSchema.parse({
      object_id: 'BATTLE_ANIM_OBJ_TEST_1',
      x: 0,
      y: 0,
      param: 0,
    });
    const second = AnimationSpriteSchema.parse({
      object_id: 'BATTLE_ANIM_OBJ_TEST_2',
      x: 1,
      y: 1,
      param: 0,
    });
    const overflow = AnimationSpriteSchema.parse({
      object_id: 'BATTLE_ANIM_OBJ_TEST_3',
      x: 2,
      y: 2,
      param: 0,
    });

    expect(table.add(first)).toBe(first);
    expect(table.add(second)).toBe(second);
    expect(table.add(overflow)).toBeNull();

    expect(warn).not.toHaveBeenCalled();
    expect(table.sprites).toHaveLength(2);
    expect(table.sprites.map((sprite) => sprite.index)).toEqual([1, 2]);
  });
});
