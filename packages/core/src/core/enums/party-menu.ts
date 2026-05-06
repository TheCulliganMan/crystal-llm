export enum PartyMenuAction {
    CHOOSE_POKEMON = 0,
    HEALING_ITEM = 1,
    SWITCH = 2,
    TEACH_TMHM = 3,
    MOVE = 4,
    EVO_STONE = 5,
    GIVE_MON = 6,
    GIVE_MON_FEMALE = 7,
    GIVE_ITEM = 8,
    MOBILE = 9,
}

export enum PartyMenuQuality {
    NICKNAMES = "nicknames",
    HP_BAR = "hp_bar",
    HP_DIGITS = "hp_digits",
    LEVEL = "level",
    STATUS = "status",
    TMHM_COMPAT = "tmhm_compat",
    EVO_STONE_COMPAT = "evo_stone_compat",
    GENDER = "gender",
    MOBILE_SELECTION = "mobile_selection",
}

const DEFAULT_SEQUENCE: PartyMenuQuality[] = [
    PartyMenuQuality.NICKNAMES,
    PartyMenuQuality.HP_BAR,
    PartyMenuQuality.HP_DIGITS,
    PartyMenuQuality.LEVEL,
    PartyMenuQuality.STATUS,
];

export const PARTY_MENU_QUALITY_POINTERS: Record<PartyMenuAction, PartyMenuQuality[]> = {
    [PartyMenuAction.CHOOSE_POKEMON]: DEFAULT_SEQUENCE,
    [PartyMenuAction.HEALING_ITEM]: DEFAULT_SEQUENCE,
    [PartyMenuAction.SWITCH]: DEFAULT_SEQUENCE,
    [PartyMenuAction.TEACH_TMHM]: [
        PartyMenuQuality.NICKNAMES,
        PartyMenuQuality.TMHM_COMPAT,
        PartyMenuQuality.LEVEL,
        PartyMenuQuality.STATUS,
    ],
    [PartyMenuAction.MOVE]: DEFAULT_SEQUENCE,
    [PartyMenuAction.EVO_STONE]: [
        PartyMenuQuality.NICKNAMES,
        PartyMenuQuality.EVO_STONE_COMPAT,
        PartyMenuQuality.LEVEL,
        PartyMenuQuality.STATUS,
    ],
    [PartyMenuAction.GIVE_MON]: [
        PartyMenuQuality.NICKNAMES,
        PartyMenuQuality.GENDER,
        PartyMenuQuality.LEVEL,
        PartyMenuQuality.STATUS,
    ],
    [PartyMenuAction.GIVE_MON_FEMALE]: [
        PartyMenuQuality.NICKNAMES,
        PartyMenuQuality.GENDER,
        PartyMenuQuality.LEVEL,
        PartyMenuQuality.STATUS,
    ],
    [PartyMenuAction.GIVE_ITEM]: DEFAULT_SEQUENCE,
    [PartyMenuAction.MOBILE]: [
        PartyMenuQuality.NICKNAMES,
        PartyMenuQuality.MOBILE_SELECTION,
        PartyMenuQuality.LEVEL,
        PartyMenuQuality.STATUS,
    ],
};

export const PARTY_MENU_PROMPTS: Record<PartyMenuAction, string> = {
    [PartyMenuAction.CHOOSE_POKEMON]: "Choose a POKéMON.",
    [PartyMenuAction.HEALING_ITEM]: "Use on which POKéMON?",
    [PartyMenuAction.SWITCH]: "Which POKéMON?",
    [PartyMenuAction.TEACH_TMHM]: "Teach which POKéMON?",
    [PartyMenuAction.MOVE]: "Move to where?",
    [PartyMenuAction.EVO_STONE]: "Use on which POKéMON?",
    [PartyMenuAction.GIVE_MON]: "Choose a POKéMON.",
    [PartyMenuAction.GIVE_MON_FEMALE]: "Choose a POKéMON.",
    [PartyMenuAction.GIVE_ITEM]: "To which POKéMON?",
    [PartyMenuAction.MOBILE]: "Choose a POKéMON.",
};
