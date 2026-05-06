import fs from 'fs';
import path from 'path';
import {
  build_battle_animation_audit_variants,
  build_battle_animation_audit_matrix,
  build_disassembly_reference_trace,
  compare_battle_animation_trace,
  trace_battle_animation,
  validate_battle_animation_trace,
} from './battle-animation-audit';

const configureDisassemblyRoot = () => {
  const disassemblyRoot = path.resolve(
    __dirname,
    '../../../../../vendor/pokecrystal',
  );
  process.env.POKECRYSTAL_DISASSEMBLY_ROOT = disassemblyRoot;
  expect(fs.existsSync(disassemblyRoot)).toBe(true);
};

describe('battle-animation audit', () => {
  beforeAll(() => {
    configureDisassemblyRoot();
  });

  it('builds an audit matrix from move-table entries and reachable shared support scripts', () => {
    const matrix = build_battle_animation_audit_matrix();
    expect(matrix.move_animation_labels.length).toBeGreaterThan(200);
    expect(matrix.support_animation_labels).toEqual(
      expect.arrayContaining([
        'BattleAnim_TargetObj_1Row',
        'BattleAnim_TargetObj_2Row',
        'BattleAnim_ShowMon_0',
        'BattleAnim_UserObj_1Row',
        'BattleAnim_UserObj_2Row',
        'BattleAnim_ShowMon_1',
      ]),
    );
    expect(matrix.audited_animation_labels.length).toBeGreaterThan(
      matrix.move_animation_labels.length,
    );
  });

  it('matches the disassembly-derived reference trace for every audited animation on both sides', () => {
    const matrix = build_battle_animation_audit_matrix();
    for (const animation_name of matrix.audited_animation_labels) {
      const variants = build_battle_animation_audit_variants(animation_name);
      for (const is_player_move of [true, false]) {
        for (const variant of variants) {
          const actual = trace_battle_animation({
            animation_name,
            is_player_move,
            param: variant.param,
            param_label: variant.param_label,
            shake_count: variant.shake_count,
            tile_size: 8,
          });
          const reference = build_disassembly_reference_trace({
            animation_name,
            is_player_move,
            param: variant.param,
            param_label: variant.param_label,
            shake_count: variant.shake_count,
            tile_size: 8,
          });
          compare_battle_animation_trace(actual, reference);
          validate_battle_animation_trace(actual);
        }
      }
    }
  });
});
