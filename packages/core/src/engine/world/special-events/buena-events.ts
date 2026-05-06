
import {
    BuenaPasswordCategory,
    BuenaPrize
} from './special-types';

export const BUENA_PASSWORD_CATEGORIES: BuenaPasswordCategory[] = [{
    label: "Johto Starters",
    category_type: "MON",
    points: 10,
    options: ["CYNDAQUIL", "TOTODILE", "CHIKORITA"],
}, {
    label: "Beverages",
    category_type: "ITEM",
    points: 12,
    options: ["FRESH_WATER", "SODA_POP", "LEMONADE"],
}, {
    label: "Healing Items",
    category_type: "ITEM",
    points: 12,
    options: ["POTION", "ANTIDOTE", "PARLYZ_HEAL"],
}, {
    label: "Balls",
    category_type: "ITEM",
    points: 12,
    options: ["POKE_BALL", "GREAT_BALL", "ULTRA_BALL"],
}, {
    label: "Pokémon 1",
    category_type: "MON",
    points: 10,
    options: ["PIKACHU", "RATTATA", "GEODUDE"],
}, {
    label: "Pokémon 2",
    category_type: "MON",
    points: 10,
    options: ["HOOTHOOT", "SPINARAK", "DROWZEE"],
}, {
    label: "Johto Towns",
    category_type: "STRING",
    points: 16,
    options: ["NEW BARK TOWN", "CHERRYGROVE CITY", "AZALEA TOWN"],
}, {
    label: "Types",
    category_type: "STRING",
    points: 6,
    options: ["FLYING", "BUG", "GRASS"],
}, {
    label: "Moves",
    category_type: "MOVE",
    points: 12,
    options: ["TACKLE", "GROWL", "MUD_SLAP"],
}, {
    label: "X Items",
    category_type: "ITEM",
    points: 12,
    options: ["X_ATTACK", "X_DEFEND", "X_SPEED"],
}, {
    label: "Radio Stations",
    category_type: "STRING",
    points: 13,
    options: ["#MON TALK", "#MON MUSIC", "LUCKY CHANNEL"],
}, ];

export const BUENA_PRIZES: BuenaPrize[] = [{
    item: "ULTRA_BALL",
    cost: 2
}, {
    item: "FULL_RESTORE",
    cost: 2
}, {
    item: "NUGGET",
    cost: 3
}, {
    item: "RARE_CANDY",
    cost: 3
}, {
    item: "PROTEIN",
    cost: 5
}, {
    item: "IRON",
    cost: 5
}, {
    item: "CARBOS",
    cost: 5
}, {
    item: "CALCIUM",
    cost: 5
}, {
    item: "HP_UP",
    cost: 5
}, ];
