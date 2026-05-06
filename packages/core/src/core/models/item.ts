import { z } from 'zod';
import { ItemEffect, ItemPocket } from '../enums';

// Helper functions for validation, ported from Python
const ITEM_EFFECT_OVERRIDES: Record<string, string> = {
    "STATUS_HEALING": "STATUS_HEAL",
    "ENERGYPOWDER": "ENERGY_POWDER",
};

export function normalizeItemEffectName(value: string): string {
    const sanitized = value.replace(/[^\p{L}\p{N}]/gu, '');
    if (!sanitized) {
        return "NONE";
    }
    let normalized = sanitized.replace(/([a-z0-9])([A-Z])/g, '$1_$2').toUpperCase();
    if (normalized.endsWith("_EFFECT")) {
        normalized = normalized.slice(0, -7);
    } else if (normalized.endsWith("EFFECT")) {
        normalized = normalized.slice(0, -6);
    }
    normalized = ITEM_EFFECT_OVERRIDES[normalized] ?? normalized;
    if (["NO", "NO_EFFECT"].includes(normalized)) {
        normalized = "NONE";
    }
    return normalized;
}

function resolveItemEffect(value: unknown): ItemEffect {
    if (typeof value === 'string' && ItemEffect[value as keyof typeof ItemEffect] !== undefined) {
        return ItemEffect[value as keyof typeof ItemEffect];
    }
    const normalized = normalizeItemEffectName(String(value ?? ''));
    const effect = ItemEffect[normalized as keyof typeof ItemEffect];
    if (effect === undefined) {
        throw new Error(`Unknown item effect: ${value}`);
    }
    return effect;
}


/**
 * Represents the static data for an in-game item.
 * This model stores all the attributes of an item, such as its price, effect,
 * and which pocket of the bag it belongs to.
 */
export const ItemSchema = z.object({
  name: z.string(),
  description: z.string().default(''),
  effect: z.preprocess(resolveItemEffect, z.nativeEnum(ItemEffect)).default(ItemEffect.NONE),
  price: z.number().int().min(0).max(0xFFFF).default(0),
  held_effect: z.string().optional().default('HELD_NONE'),
  parameter: z.number().int().min(-0x80).max(0xFF).default(0),
  property: z.string().optional().default(''),
  pocket: z.nativeEnum(ItemPocket).default(ItemPocket.ITEM),
  field_menu: z.string().optional().default(''),
  battle_menu: z.string().optional().default(''),
}).transform(item => ({
    ...item,
    script_name: item.name.replace(/ /g, '_').toUpperCase()
}));

export type Item = z.infer<typeof ItemSchema>;
