import { ItemSchema } from './item';
import { ItemPocket } from '../enums';

const itemFixture = (overrides: Partial<Record<string, unknown>> = {}) => ({
  name: 'POTION',
  script_name: 'POTION',
  description: 'Restores HP.',
  effect: 'RESTORE_HP',
  status_heals: [],
  revive_hp_percent: null,
  party_revive_hp_percent: null,
  pp_restore_scope: null,
  pp_restore_points: null,
  pp_up_stages: null,
  vitamin_stat: null,
  vitamin_stat_exp: null,
  vitamin_max_stat_exp: null,
  rare_candy_level_gain: null,
  battle_stat_boost_stat: null,
  battle_stat_boost_stages: null,
  battle_escape_mode: null,
  battle_focus_energy: null,
  battle_stat_drop_guard: null,
  confusion_heal: null,
  repel_steps: null,
  escape_rope_mode: null,
  price: 300,
  held_effect: 'HELD_NONE',
  parameter: 20,
  property: 'CANT_SELECT',
  pocket: ItemPocket.ITEM,
  field_menu: 'ITEMMENU_PARTY',
  field_usable: true,
  battle_menu: 'ITEMMENU_PARTY',
  battle_usable: true,
  battle_capture_ball: null,
  consumable: true,
  tmhm_index: null,
  tmhm_move: null,
  ...overrides,
});

describe('ItemSchema', () => {
  it('preserves definitive modpack item effect strings without enum validation', () => {
    const item = ItemSchema.parse(itemFixture({ effect: 'MODDED_FIELD_EFFECT' }));

    expect(item.effect).toBe('MODDED_FIELD_EFFECT');
  });

  it('requires the exporter-provided script_name instead of deriving it from display name', () => {
    expect(() => ItemSchema.parse(itemFixture({ script_name: undefined }))).toThrow(/script_name/);
  });

  it('requires an explicit item effect instead of defaulting to NONE', () => {
    expect(() => ItemSchema.parse(itemFixture({ effect: undefined }))).toThrow(/effect/);
    expect(() => ItemSchema.parse(itemFixture({ effect: null }))).toThrow(/effect/);
  });

  it('preserves effect spelling exactly instead of normalizing legacy aliases', () => {
    const item = ItemSchema.parse(itemFixture({ effect: 'statusHeal' }));

    expect(item.effect).toBe('statusHeal');
  });

  it('accepts the exported field and battle usability metadata', () => {
    const item = ItemSchema.parse(itemFixture({ battle_capture_ball: true }));

    expect(item).toMatchObject({
      field_usable: true,
      battle_usable: true,
      battle_capture_ball: true,
    });
  });

  it('rejects legacy param keys instead of stripping them', () => {
    expect(() => ItemSchema.parse(itemFixture({ param: 20 }))).toThrow(/param/);
  });
});
