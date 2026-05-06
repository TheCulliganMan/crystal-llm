import { ItemSchema, normalizeItemEffectName } from './item';
import { ItemEffect, ItemPocket } from '../enums';

describe('Item', () => {
    describe('normalizeItemEffectName', () => {
        it('should not strip unicode alphanumeric characters', () => {
            const result = normalizeItemEffectName('Pokéball');
            expect(result).toContain('POKÉBALL');
        });

        it('should normalize readable names into enum keys', () => {
            expect(normalizeItemEffectName('Status Healing')).toBe('STATUS_HEAL');
            expect(normalizeItemEffectName('status heal')).toBe('STATUSHEAL');
            expect(normalizeItemEffectName('statusHeal')).toBe('STATUS_HEAL');
        });

        it('should normalize legacy or compact tokens', () => {
            expect(normalizeItemEffectName('No Effect')).toBe('NONE');
            expect(normalizeItemEffectName('NO_EFFECT')).toBe('NONE');
            expect(normalizeItemEffectName('EnergyPowder')).toBe('ENERGY_POWDER');
        });
    });

    describe('ItemSchema', () => {
        it('should derive script_name from spaced names', () => {
            const item = ItemSchema.parse({
                name: 'X SPECIAL'
            });
            expect(item.script_name).toBe('X_SPECIAL');
        });

        it('should preserve scripted item names for HM and TM style items', () => {
            const hm = ItemSchema.parse({
                name: 'HM01'
            });
            expect(hm.script_name).toBe('HM01');

            const tm = ItemSchema.parse({
                name: 'TM01'
            });
            expect(tm.script_name).toBe('TM01');
        });

        it('should apply defaults for omitted optional metadata fields', () => {
            const item = ItemSchema.parse({
                name: 'Test Item'
            });
            expect(item.description).toBe('');
            expect(item.effect).toBe(ItemEffect.NONE);
            expect(item.price).toBe(0);
            expect(item.held_effect).toBe('HELD_NONE');
            expect(item.parameter).toBe(0);
            expect(item.property).toBe('');
            expect(item.pocket).toBe(ItemPocket.ITEM);
            expect(item.field_menu).toBe('');
            expect(item.battle_menu).toBe('');
            expect(item.script_name).toBe('TEST_ITEM');
        });

        it('should preserve held_effect, field_menu, and battle_menu', () => {
            const item = ItemSchema.parse({
                name: 'Quick Claw',
                held_effect: 'HELD_QUICK_CLAW',
                field_menu: 'ITEMMENU_PARTY',
                battle_menu: 'BATTLEMENU_ITEM'
            });
            expect(item.held_effect).toBe('HELD_QUICK_CLAW');
            expect(item.field_menu).toBe('ITEMMENU_PARTY');
            expect(item.battle_menu).toBe('BATTLEMENU_ITEM');
        });

        it('should avoid silently accepting legacy param keys as parser defaults', () => {
            const item = ItemSchema.parse({
                name: 'POTION',
                effect: ItemEffect.RESTORE_HP,
                param: 20
            });
            expect(item.parameter).toBe(0);
        });
    });

    describe('resolveItemEffect', () => {
        it('should resolve valid string to ItemEffect', () => {
            const item = ItemSchema.parse({
                name: 'Test Item',
                effect: 'STATUS_HEAL'
            });
            expect(item.effect).toBe(ItemEffect.STATUS_HEAL);
        });

        it('should resolve valid number to ItemEffect', () => {
            const item = ItemSchema.parse({
                name: 'Test Item',
                effect: ItemEffect.STATUS_HEAL
            });
            expect(item.effect).toBe(ItemEffect.STATUS_HEAL);
        });

        it('should normalize and resolve a string to ItemEffect', () => {
            const item = ItemSchema.parse({
                name: 'Test Item',
                effect: 'statusHeal'
            });
            expect(item.effect).toBe(ItemEffect.STATUS_HEAL);
        });

        it('should throw an error for an unknown item effect', () => {
            expect(() => {
                ItemSchema.parse({
                    name: 'Test Item',
                    effect: 'UNKNOWN_EFFECT'
                });
            }).toThrow('Unknown item effect: UNKNOWN_EFFECT');
        });

        it('should handle null and undefined by defaulting to NONE', () => {
            const itemNull = ItemSchema.parse({
                name: 'Test Item',
                effect: null
            });
            expect(itemNull.effect).toBe(ItemEffect.NONE);

            const itemUndefined = ItemSchema.parse({
                name: 'Test Item',
                effect: undefined
            });
            expect(itemUndefined.effect).toBe(ItemEffect.NONE);
        });

        it('should default empty strings and blank text to NONE', () => {
            const itemEmpty = ItemSchema.parse({
                name: 'Test Item',
                effect: ''
            });
            expect(itemEmpty.effect).toBe(ItemEffect.NONE);

            const itemSpaces = ItemSchema.parse({
                name: 'Test Item',
                effect: '   '
            });
            expect(itemSpaces.effect).toBe(ItemEffect.NONE);
        });
    });
});
