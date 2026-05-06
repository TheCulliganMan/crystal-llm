
import { z } from 'zod';
import { ItemEnum } from './enums';
import { MAIL_MSG_LENGTH, PLAYER_NAME_LENGTH, NUM_POKEMON } from './constants';

export enum MailLanguage {
    ENGLISH = 0,
    FRENCH,
    GERMAN,
    ITALIAN,
    SPANISH,
}

export const _MAIL_ITEMS = new Set([
    ItemEnum.FLOWER_MAIL,
    ItemEnum.SURF_MAIL,
    ItemEnum.LITEBLUEMAIL,
    ItemEnum.PORTRAITMAIL,
    ItemEnum.LOVELY_MAIL,
    ItemEnum.EON_MAIL,
    ItemEnum.MORPH_MAIL,
    ItemEnum.BLUESKY_MAIL,
    ItemEnum.MUSIC_MAIL,
    ItemEnum.MIRAGE_MAIL,
]);


export const MailMessageSchema = z.object({
    message: z.string().max(MAIL_MSG_LENGTH).default(''),
    author: z.string().max(PLAYER_NAME_LENGTH).default('PLAYER'),
    nationality: z.nativeEnum(MailLanguage).default(MailLanguage.ENGLISH),
    author_id: z.number().int().min(0).max(0xFFFF).default(0),
    species_id: z.number().int().min(0).max(NUM_POKEMON).default(0),
    mail_type: z.nativeEnum(ItemEnum).refine(value => _MAIL_ITEMS.has(value), {
        message: "Mail type must be one of the Game Boy mail items.",
    }).default(ItemEnum.FLOWER_MAIL),
});

export type MailMessage = z.infer<typeof MailMessageSchema>;

export function preview(mail: MailMessage, maxLength: number = 16): string {
    if (!mail.message) {
        return "(empty)";
    }
    return mail.message.substring(0, maxLength);
}
