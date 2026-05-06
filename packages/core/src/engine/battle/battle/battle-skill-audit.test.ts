import { MoveEffect, MoveName } from '@pokecrystal/core/core/enums';
import { build_battle_skill_audit, summarize_battle_skill_audit } from './battle-skill-audit';

describe('battle-skill-audit', () => {
  it('classifies every move with a mechanic owner and animation label', () => {
    const entries = build_battle_skill_audit();

    expect(entries.length).toBeGreaterThan(200);
    expect(entries.every((entry) => entry.animation_label !== null)).toBe(true);
    expect(entries.every((entry) => entry.mechanic_owner)).toBe(true);
  });

  it('covers the shared-execution move families added to move execution', () => {
    const entries = build_battle_skill_audit();
    const byMove = new Map(entries.map((entry) => [entry.move_name, entry]));

    expect(byMove.get(MoveName.FALSE_SWIPE)).toEqual(
      expect.objectContaining({
        effect: MoveEffect.FALSE_SWIPE,
        status: 'delegated_shared',
        mechanic_owner: 'move-execution',
      }),
    );
    expect(byMove.get(MoveName.RETURN)).toEqual(
      expect.objectContaining({
        effect: MoveEffect.RETURN,
        status: 'delegated_shared',
      }),
    );
    expect(byMove.get(MoveName.DRAGON_RAGE)).toEqual(
      expect.objectContaining({
        effect: MoveEffect.STATIC_DAMAGE,
        status: 'delegated_shared',
      }),
    );
    expect(byMove.get(MoveName.SEISMIC_TOSS)).toEqual(
      expect.objectContaining({
        effect: MoveEffect.LEVEL_DAMAGE,
        status: 'delegated_shared',
      }),
    );
  });

  it('summarizes coverage buckets without leaving any move uncounted', () => {
    const entries = build_battle_skill_audit();
    const summary = summarize_battle_skill_audit(entries);
    const total = Object.values(summary).reduce((sum, count) => sum + count, 0);

    expect(total).toBe(entries.length);
    expect(summary.animation_incomplete).toBe(0);
  });
});
