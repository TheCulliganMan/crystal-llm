import { MoveName } from "./move";

export enum MonMenuCategory {
    FIELD_MOVE = 0,
    MENU_OPTION = 1,
}

export enum MonMenuItem {
    CUT = 1,
    FLY = 2,
    SURF = 3,
    STRENGTH = 4,
    WATERFALL = 5,
    FLASH = 6,
    WHIRLPOOL = 7,
    DIG = 8,
    TELEPORT = 9,
    SOFTBOILED = 10,
    HEADBUTT = 11,
    ROCKSMASH = 12,
    MILKDRINK = 13,
    SWEETSCENT = 14,
    STATS = 15,
    SWITCH = 16,
    ITEM = 17,
    CANCEL = 18,
    MOVE = 19,
    MAIL = 20,
    ERROR = 21,
}

export enum MonMenuValue {
    STATS = 1,
    SWITCH = 2,
    ITEM = 3,
    CANCEL = 4,
    MOVE = 5,
    MAIL = 6,
    ERROR = 7,
}

export const MON_MENU_OPTION_STRINGS: Record<MonMenuValue, string> = {
    [MonMenuValue.STATS]: "STATS",
    [MonMenuValue.SWITCH]: "SWITCH",
    [MonMenuValue.ITEM]: "ITEM",
    [MonMenuValue.CANCEL]: "CANCEL",
    [MonMenuValue.MOVE]: "MOVE",
    [MonMenuValue.MAIL]: "MAIL",
    [MonMenuValue.ERROR]: "ERROR!",
};

export interface MonMenuEntry {
    category: MonMenuCategory;
    item: MonMenuItem;
    value: MoveName | MonMenuValue;
}

export const MON_MENU_OPTIONS: MonMenuEntry[] = [
    // Field moves
    { category: MonMenuCategory.FIELD_MOVE, item: MonMenuItem.CUT, value: MoveName.CUT },
    { category: MonMenuCategory.FIELD_MOVE, item: MonMenuItem.FLY, value: MoveName.FLY },
    { category: MonMenuCategory.FIELD_MOVE, item: MonMenuItem.SURF, value: MoveName.SURF },
    { category: MonMenuCategory.FIELD_MOVE, item: MonMenuItem.STRENGTH, value: MoveName.STRENGTH },
    { category: MonMenuCategory.FIELD_MOVE, item: MonMenuItem.FLASH, value: MoveName.FLASH },
    { category: MonMenuCategory.FIELD_MOVE, item: MonMenuItem.WATERFALL, value: MoveName.WATERFALL },
    { category: MonMenuCategory.FIELD_MOVE, item: MonMenuItem.WHIRLPOOL, value: MoveName.WHIRLPOOL },
    { category: MonMenuCategory.FIELD_MOVE, item: MonMenuItem.DIG, value: MoveName.DIG },
    { category: MonMenuCategory.FIELD_MOVE, item: MonMenuItem.TELEPORT, value: MoveName.TELEPORT },
    { category: MonMenuCategory.FIELD_MOVE, item: MonMenuItem.SOFTBOILED, value: MoveName.SOFTBOILED },
    { category: MonMenuCategory.FIELD_MOVE, item: MonMenuItem.HEADBUTT, value: MoveName.HEADBUTT },
    { category: MonMenuCategory.FIELD_MOVE, item: MonMenuItem.ROCKSMASH, value: MoveName.ROCK_SMASH },
    { category: MonMenuCategory.FIELD_MOVE, item: MonMenuItem.MILKDRINK, value: MoveName.MILK_DRINK },
    { category: MonMenuCategory.FIELD_MOVE, item: MonMenuItem.SWEETSCENT, value: MoveName.SWEET_SCENT },
    // Static options
    { category: MonMenuCategory.MENU_OPTION, item: MonMenuItem.STATS, value: MonMenuValue.STATS },
    { category: MonMenuCategory.MENU_OPTION, item: MonMenuItem.SWITCH, value: MonMenuValue.SWITCH },
    { category: MonMenuCategory.MENU_OPTION, item: MonMenuItem.ITEM, value: MonMenuValue.ITEM },
    { category: MonMenuCategory.MENU_OPTION, item: MonMenuItem.CANCEL, value: MonMenuValue.CANCEL },
    { category: MonMenuCategory.MENU_OPTION, item: MonMenuItem.MOVE, value: MonMenuValue.MOVE },
    { category: MonMenuCategory.MENU_OPTION, item: MonMenuItem.MAIL, value: MonMenuValue.MAIL },
    { category: MonMenuCategory.MENU_OPTION, item: MonMenuItem.ERROR, value: MonMenuValue.ERROR },
];
