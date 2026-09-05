import { z } from 'zod';
import { ItemPocket } from '../enums';


/**
 * Represents the static data for an in-game item.
 * This model stores all the attributes of an item, such as its price, effect,
 * and which pocket of the bag it belongs to.
 */
export const ItemSchema = z.object({
  name: z.string(),
  script_name: z.string(),
  description: z.string(),
  effect: z.string(),
  status_heals: z.array(z.string()),
  revive_hp_percent: z.number().int().min(0).max(100).nullable(),
  party_revive_hp_percent: z.number().int().min(0).max(100).nullable(),
  pp_restore_scope: z.string().nullable(),
  pp_restore_points: z.number().int().min(0).max(0xFF).nullable(),
  pp_up_stages: z.number().int().min(0).max(0xFF).nullable(),
  vitamin_stat: z.string().nullable(),
  vitamin_stat_exp: z.number().int().min(0).max(0xFFFF).nullable(),
  vitamin_max_stat_exp: z.number().int().min(0).max(0xFFFF).nullable(),
  rare_candy_level_gain: z.number().int().min(0).max(0xFF).nullable(),
  battle_stat_boost_stat: z.string().nullable(),
  battle_stat_boost_stages: z.number().int().min(0).max(0xFF).nullable(),
  battle_escape_mode: z.string().nullable(),
  battle_focus_energy: z.boolean().nullable(),
  battle_stat_drop_guard: z.boolean().nullable(),
  confusion_heal: z.boolean().nullable(),
  repel_steps: z.number().int().min(0).max(0xFFFF).nullable(),
  escape_rope_mode: z.string().nullable(),
  price: z.number().int().min(0).max(0xFFFF),
  held_effect: z.string(),
  parameter: z.number().int().min(-0x80).max(0xFF),
  property: z.string(),
  pocket: z.nativeEnum(ItemPocket),
  field_menu: z.string(),
  field_usable: z.boolean(),
  battle_menu: z.string(),
  battle_usable: z.boolean(),
  battle_capture_ball: z.boolean().nullable(),
  consumable: z.boolean(),
  tmhm_index: z.number().int().min(0).nullable(),
  tmhm_move: z.string().nullable(),
}).strict();

export type Item = z.infer<typeof ItemSchema>;
