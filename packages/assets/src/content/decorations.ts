
import { z } from 'zod';

const DecorationAttributeSchema = z.object({
  index: z.number(),
  deco_type: z.string(),
  name_token: z.string(),
  action_token: z.string(),
  event_flag: z.string(),
  sprite_token: z.string(),
  sprite_value: z.number().nullable(),
});

export type DecorationAttribute = z.infer<typeof DecorationAttributeSchema>;

export const decorations: DecorationAttribute[] = [
    {
        "index": 0,
        "deco_type": "DECO_PLANT",
        "name_token": "0",
        "action_token": "0",
        "event_flag": "EVENT_TEMPORARY_UNTIL_MAP_RELOAD_1",
        "sprite_token": "0",
        "sprite_value": 0
    },
    {
        "index": 1,
        "deco_type": "DECO_PLANT",
        "name_token": "PUT_IT_AWAY",
        "action_token": "PUT_AWAY_BED",
        "event_flag": "EVENT_TEMPORARY_UNTIL_MAP_RELOAD_1",
        "sprite_token": "0",
        "sprite_value": 0
    },
    {
        "index": 2,
        "deco_type": "DECO_BED",
        "name_token": "FEATHERY_BED",
        "action_token": "SET_UP_BED",
        "event_flag": "EVENT_DECO_BED_1",
        "sprite_token": "$1b",
        "sprite_value": 27
    },
    {
        "index": 3,
        "deco_type": "DECO_BED",
        "name_token": "PINK_BED",
        "action_token": "SET_UP_BED",
        "event_flag": "EVENT_DECO_BED_2",
        "sprite_token": "$1c",
        "sprite_value": 28
    },
    {
        "index": 4,
        "deco_type": "DECO_BED",
        "name_token": "POLKADOT_BED",
        "action_token": "SET_UP_BED",
        "event_flag": "EVENT_DECO_BED_3",
        "sprite_token": "$1d",
        "sprite_value": 29
    },
    {
        "index": 5,
        "deco_type": "DECO_BED",
        "name_token": "PIKACHU_BED",
        "action_token": "SET_UP_BED",
        "event_flag": "EVENT_DECO_BED_4",
        "sprite_token": "$1e",
        "sprite_value": 30
    },
    {
        "index": 6,
        "deco_type": "DECO_PLANT",
        "name_token": "PUT_IT_AWAY",
        "action_token": "PUT_AWAY_CARPET",
        "event_flag": "EVENT_TEMPORARY_UNTIL_MAP_RELOAD_1",
        "sprite_token": "0",
        "sprite_value": 0
    },
    {
        "index": 7,
        "deco_type": "DECO_CARPET",
        "name_token": "RED_CARPET",
        "action_token": "SET_UP_CARPET",
        "event_flag": "EVENT_DECO_CARPET_1",
        "sprite_token": "$08",
        "sprite_value": 8
    },
    {
        "index": 8,
        "deco_type": "DECO_CARPET",
        "name_token": "BLUE_CARPET",
        "action_token": "SET_UP_CARPET",
        "event_flag": "EVENT_DECO_CARPET_2",
        "sprite_token": "$0b",
        "sprite_value": 11
    },
    {
        "index": 9,
        "deco_type": "DECO_CARPET",
        "name_token": "YELLOW_CARPET",
        "action_token": "SET_UP_CARPET",
        "event_flag": "EVENT_DECO_CARPET_3",
        "sprite_token": "$0e",
        "sprite_value": 14
    },
    {
        "index": 10,
        "deco_type": "DECO_CARPET",
        "name_token": "GREEN_CARPET",
        "action_token": "SET_UP_CARPET",
        "event_flag": "EVENT_DECO_CARPET_4",
        "sprite_token": "$11",
        "sprite_value": 17
    },
    {
        "index": 11,
        "deco_type": "DECO_PLANT",
        "name_token": "PUT_IT_AWAY",
        "action_token": "PUT_AWAY_PLANT",
        "event_flag": "EVENT_TEMPORARY_UNTIL_MAP_RELOAD_1",
        "sprite_token": "0",
        "sprite_value": 0
    },
    {
        "index": 12,
        "deco_type": "DECO_PLANT",
        "name_token": "MAGNAPLANT",
        "action_token": "SET_UP_PLANT",
        "event_flag": "EVENT_DECO_PLANT_1",
        "sprite_token": "$20",
        "sprite_value": 32
    },
    {
        "index": 13,
        "deco_type": "DECO_PLANT",
        "name_token": "TROPICPLANT",
        "action_token": "SET_UP_PLANT",
        "event_flag": "EVENT_DECO_PLANT_2",
        "sprite_token": "$21",
        "sprite_value": 33
    },
    {
        "index": 14,
        "deco_type": "DECO_PLANT",
        "name_token": "JUMBOPLANT",
        "action_token": "SET_UP_PLANT",
        "event_flag": "EVENT_DECO_PLANT_3",
        "sprite_token": "$22",
        "sprite_value": 34
    },
    {
        "index": 15,
        "deco_type": "DECO_PLANT",
        "name_token": "PUT_IT_AWAY",
        "action_token": "PUT_AWAY_POSTER",
        "event_flag": "EVENT_TEMPORARY_UNTIL_MAP_RELOAD_1",
        "sprite_token": "0",
        "sprite_value": 0
    },
    {
        "index": 16,
        "deco_type": "DECO_PLANT",
        "name_token": "TOWN_MAP_POSTER",
        "action_token": "SET_UP_POSTER",
        "event_flag": "EVENT_DECO_POSTER_1",
        "sprite_token": "$1f",
        "sprite_value": 31
    },
    {
        "index": 17,
        "deco_type": "DECO_POSTER",
        "name_token": "PIKACHU",
        "action_token": "SET_UP_POSTER",
        "event_flag": "EVENT_DECO_POSTER_2",
        "sprite_token": "$23",
        "sprite_value": 35
    },
    {
        "index": 18,
        "deco_type": "DECO_POSTER",
        "name_token": "CLEFAIRY",
        "action_token": "SET_UP_POSTER",
        "event_flag": "EVENT_DECO_POSTER_3",
        "sprite_token": "$24",
        "sprite_value": 36
    },
    {
        "index": 19,
        "deco_type": "DECO_POSTER",
        "name_token": "JIGGLYPUFF",
        "action_token": "SET_UP_POSTER",
        "event_flag": "EVENT_DECO_POSTER_4",
        "sprite_token": "$25",
        "sprite_value": 37
    },
    {
        "index": 20,
        "deco_type": "DECO_PLANT",
        "name_token": "PUT_IT_AWAY",
        "action_token": "PUT_AWAY_CONSOLE",
        "event_flag": "EVENT_TEMPORARY_UNTIL_MAP_RELOAD_1",
        "sprite_token": "0",
        "sprite_value": 0
    },
    {
        "index": 21,
        "deco_type": "DECO_PLANT",
        "name_token": "FAMICOM",
        "action_token": "SET_UP_CONSOLE",
        "event_flag": "EVENT_DECO_FAMICOM",
        "sprite_token": "SPRITE_FAMICOM",
        "sprite_value": null
    },
    {
        "index": 22,
        "deco_type": "DECO_PLANT",
        "name_token": "SUPER_NES",
        "action_token": "SET_UP_CONSOLE",
        "event_flag": "EVENT_DECO_SNES",
        "sprite_token": "SPRITE_SNES",
        "sprite_value": null
    },
    {
        "index": 23,
        "deco_type": "DECO_PLANT",
        "name_token": "NINTENDO_64",
        "action_token": "SET_UP_CONSOLE",
        "event_flag": "EVENT_DECO_N64",
        "sprite_token": "SPRITE_N64",
        "sprite_value": null
    },
    {
        "index": 24,
        "deco_type": "DECO_PLANT",
        "name_token": "VIRTUAL_BOY",
        "action_token": "SET_UP_CONSOLE",
        "event_flag": "EVENT_DECO_VIRTUAL_BOY",
        "sprite_token": "SPRITE_VIRTUAL_BOY",
        "sprite_value": null
    },
    {
        "index": 25,
        "deco_type": "DECO_PLANT",
        "name_token": "PUT_IT_AWAY",
        "action_token": "PUT_AWAY_BIG_DOLL",
        "event_flag": "EVENT_TEMPORARY_UNTIL_MAP_RELOAD_1",
        "sprite_token": "0",
        "sprite_value": 0
    },
    {
        "index": 26,
        "deco_type": "DECO_BIGDOLL",
        "name_token": "SNORLAX",
        "action_token": "SET_UP_BIG_DOLL",
        "event_flag": "EVENT_DECO_BIG_SNORLAX_DOLL",
        "sprite_token": "SPRITE_BIG_SNORLAX",
        "sprite_value": null
    },
    {
        "index": 27,
        "deco_type": "DECO_BIGDOLL",
        "name_token": "ONIX",
        "action_token": "SET_UP_BIG_DOLL",
        "event_flag": "EVENT_DECO_BIG_ONIX_DOLL",
        "sprite_token": "SPRITE_BIG_ONIX",
        "sprite_value": null
    },
    {
        "index": 28,
        "deco_type": "DECO_BIGDOLL",
        "name_token": "LAPRAS",
        "action_token": "SET_UP_BIG_DOLL",
        "event_flag": "EVENT_DECO_BIG_LAPRAS_DOLL",
        "sprite_token": "SPRITE_BIG_LAPRAS",
        "sprite_value": null
    },
    {
        "index": 29,
        "deco_type": "DECO_PLANT",
        "name_token": "PUT_IT_AWAY",
        "action_token": "PUT_AWAY_DOLL",
        "event_flag": "EVENT_TEMPORARY_UNTIL_MAP_RELOAD_1",
        "sprite_token": "0",
        "sprite_value": 0
    },
    {
        "index": 30,
        "deco_type": "DECO_DOLL",
        "name_token": "PIKACHU",
        "action_token": "SET_UP_DOLL",
        "event_flag": "EVENT_DECO_PIKACHU_DOLL",
        "sprite_token": "SPRITE_PIKACHU",
        "sprite_value": null
    },
    {
        "index": 31,
        "deco_type": "DECO_PLANT",
        "name_token": "SURF_PIKA_DOLL",
        "action_token": "SET_UP_DOLL",
        "event_flag": "EVENT_DECO_SURFING_PIKACHU_DOLL",
        "sprite_token": "SPRITE_SURFING_PIKACHU",
        "sprite_value": null
    },
    {
        "index": 32,
        "deco_type": "DECO_DOLL",
        "name_token": "CLEFAIRY",
        "action_token": "SET_UP_DOLL",
        "event_flag": "EVENT_DECO_CLEFAIRY_DOLL",
        "sprite_token": "SPRITE_CLEFAIRY",
        "sprite_value": null
    },
    {
        "index": 33,
        "deco_type": "DECO_DOLL",
        "name_token": "JIGGLYPUFF",
        "action_token": "SET_UP_DOLL",
        "event_flag": "EVENT_DECO_JIGGLYPUFF_DOLL",
        "sprite_token": "SPRITE_JIGGLYPUFF",
        "sprite_value": null
    },
    {
        "index": 34,
        "deco_type": "DECO_DOLL",
        "name_token": "BULBASAUR",
        "action_token": "SET_UP_DOLL",
        "event_flag": "EVENT_DECO_BULBASAUR_DOLL",
        "sprite_token": "SPRITE_BULBASAUR",
        "sprite_value": null
    },
    {
        "index": 35,
        "deco_type": "DECO_DOLL",
        "name_token": "CHARMANDER",
        "action_token": "SET_UP_DOLL",
        "event_flag": "EVENT_DECO_CHARMANDER_DOLL",
        "sprite_token": "SPRITE_CHARMANDER",
        "sprite_value": null
    },
    {
        "index": 36,
        "deco_type": "DECO_DOLL",
        "name_token": "SQUIRTLE",
        "action_token": "SET_UP_DOLL",
        "event_flag": "EVENT_DECO_SQUIRTLE_DOLL",
        "sprite_token": "SPRITE_SQUIRTLE",
        "sprite_value": null
    },
    {
        "index": 37,
        "deco_type": "DECO_DOLL",
        "name_token": "POLIWAG",
        "action_token": "SET_UP_DOLL",
        "event_flag": "EVENT_DECO_POLIWAG_DOLL",
        "sprite_token": "SPRITE_POLIWAG",
        "sprite_value": null
    },
    {
        "index": 38,
        "deco_type": "DECO_DOLL",
        "name_token": "DIGLETT",
        "action_token": "SET_UP_DOLL",
        "event_flag": "EVENT_DECO_DIGLETT_DOLL",
        "sprite_token": "SPRITE_DIGLETT",
        "sprite_value": null
    },
    {
        "index": 39,
        "deco_type": "DECO_DOLL",
        "name_token": "STARYU",
        "action_token": "SET_UP_DOLL",
        "event_flag": "EVENT_DECO_STARYU_DOLL",
        "sprite_token": "SPRITE_STARMIE",
        "sprite_value": null
    },
    {
        "index": 40,
        "deco_type": "DECO_DOLL",
        "name_token": "MAGIKARP",
        "action_token": "SET_UP_DOLL",
        "event_flag": "EVENT_DECO_MAGIKARP_DOLL",
        "sprite_token": "SPRITE_MAGIKARP",
        "sprite_value": null
    },
    {
        "index": 41,
        "deco_type": "DECO_DOLL",
        "name_token": "ODDISH",
        "action_token": "SET_UP_DOLL",
        "event_flag": "EVENT_DECO_ODDISH_DOLL",
        "sprite_token": "SPRITE_ODDISH",
        "sprite_value": null
    },
    {
        "index": 42,
        "deco_type": "DECO_DOLL",
        "name_token": "GENGAR",
        "action_token": "SET_UP_DOLL",
        "event_flag": "EVENT_DECO_GENGAR_DOLL",
        "sprite_token": "SPRITE_GENGAR",
        "sprite_value": null
    },
    {
        "index": 43,
        "deco_type": "DECO_DOLL",
        "name_token": "SHELLDER",
        "action_token": "SET_UP_DOLL",
        "event_flag": "EVENT_DECO_SHELLDER_DOLL",
        "sprite_token": "SPRITE_SHELLDER",
        "sprite_value": null
    },
    {
        "index": 44,
        "deco_type": "DECO_DOLL",
        "name_token": "GRIMER",
        "action_token": "SET_UP_DOLL",
        "event_flag": "EVENT_DECO_GRIMER_DOLL",
        "sprite_token": "SPRITE_GRIMER",
        "sprite_value": null
    },
    {
        "index": 45,
        "deco_type": "DECO_DOLL",
        "name_token": "VOLTORB",
        "action_token": "SET_UP_DOLL",
        "event_flag": "EVENT_DECO_VOLTORB_DOLL",
        "sprite_token": "SPRITE_VOLTORB",
        "sprite_value": null
    },
    {
        "index": 46,
        "deco_type": "DECO_DOLL",
        "name_token": "WEEDLE",
        "action_token": "SET_UP_DOLL",
        "event_flag": "EVENT_DECO_WEEDLE_DOLL",
        "sprite_token": "SPRITE_WEEDLE",
        "sprite_value": null
    },
    {
        "index": 47,
        "deco_type": "DECO_DOLL",
        "name_token": "UNOWN",
        "action_token": "SET_UP_DOLL",
        "event_flag": "EVENT_DECO_UNOWN_DOLL",
        "sprite_token": "SPRITE_UNOWN",
        "sprite_value": null
    },
    {
        "index": 48,
        "deco_type": "DECO_DOLL",
        "name_token": "GEODUDE",
        "action_token": "SET_UP_DOLL",
        "event_flag": "EVENT_DECO_GEODUDE_DOLL",
        "sprite_token": "SPRITE_GEODUDE",
        "sprite_value": null
    },
    {
        "index": 49,
        "deco_type": "DECO_DOLL",
        "name_token": "MACHOP",
        "action_token": "SET_UP_DOLL",
        "event_flag": "EVENT_DECO_MACHOP_DOLL",
        "sprite_token": "SPRITE_MACHOP",
        "sprite_value": null
    },
    {
        "index": 50,
        "deco_type": "DECO_DOLL",
        "name_token": "TENTACOOL",
        "action_token": "SET_UP_DOLL",
        "event_flag": "EVENT_DECO_TENTACOOL_DOLL",
        "sprite_token": "SPRITE_TENTACOOL",
        "sprite_value": null
    },
    {
        "index": 51,
        "deco_type": "DECO_PLANT",
        "name_token": "GOLD_TROPHY",
        "action_token": "SET_UP_DOLL",
        "event_flag": "EVENT_DECO_GOLD_TROPHY",
        "sprite_token": "SPRITE_GOLD_TROPHY",
        "sprite_value": null
    },
    {
        "index": 52,
        "deco_type": "DECO_PLANT",
        "name_token": "SILVER_TROPHY",
        "action_token": "SET_UP_DOLL",
        "event_flag": "EVENT_DECO_SILVER_TROPHY",
        "sprite_token": "SPRITE_SILVER_TROPHY",
        "sprite_value": null
    }
].map(d => DecorationAttributeSchema.parse(d));

export const decorationIdMap: { [key: number]: string } = {
    "2": "DECO_FEATHERY_BED",
    "3": "DECO_PINK_BED",
    "4": "DECO_POLKADOT_BED",
    "5": "DECO_PIKACHU_BED",
    "7": "DECO_RED_CARPET",
    "8": "DECO_BLUE_CARPET",
    "9": "DECO_YELLOW_CARPET",
    "10": "DECO_GREEN_CARPET",
    "12": "DECO_MAGNAPLANT",
    "13": "DECO_TROPICPLANT",
    "14": "DECO_JUMBOPLANT",
    "16": "DECO_TOWN_MAP",
    "17": "DECO_PIKACHU_POSTER",
    "18": "DECO_CLEFAIRY_POSTER",
    "19": "DECO_JIGGLYPUFF_POSTER",
    "21": "DECO_FAMICOM",
    "22": "DECO_SNES",
    "23": "DECO_N64",
    "24": "DECO_VIRTUAL_BOY",
    "30": "DECO_PIKACHU_DOLL",
    "31": "DECO_SURF_PIKACHU_DOLL",
    "32": "DECO_CLEFAIRY_DOLL",
    "33": "DECO_JIGGLYPUFF_DOLL",
    "34": "DECO_BULBASAUR_DOLL",
    "35": "DECO_CHARMANDER_DOLL",
    "36": "DECO_SQUIRTLE_DOLL",
    "37": "DECO_POLIWAG_DOLL",
    "38": "DECO_DIGLETT_DOLL",
    "39": "DECO_STARYU_DOLL",
    "40": "DECO_MAGIKARP_DOLL",
    "41": "DECO_ODDISH_DOLL",
    "42": "DECO_GENGAR_DOLL",
    "43": "DECO_SHELLDER_DOLL",
    "44": "DECO_GRIMER_DOLL",
    "45": "DECO_VOLTORB_DOLL",
    "46": "DECO_WEEDLE_DOLL",
    "47": "DECO_UNOWN_DOLL",
    "48": "DECO_GEODUDE_DOLL",
    "49": "DECO_MACHOP_DOLL",
    "50": "DECO_TENTACOOL_DOLL",
    "26": "DECO_BIG_SNORLAX_DOLL",
    "27": "DECO_BIG_ONIX_DOLL",
    "28": "DECO_BIG_LAPRAS_DOLL",
    "51": "DECO_GOLD_TROPHY_DOLL",
    "52": "DECO_SILVER_TROPHY_DOLL"
};

export function getDecorationConstant(decoId: number): string | undefined {
  return decorationIdMap[decoId];
}
