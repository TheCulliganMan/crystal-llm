/**
 * This module contains constants converted from the disassembly.
 */

import { MoveName } from './enums';

export const GB_CPU_FREQUENCY = 4_194_304;
export const GB_FRAME_RATE = 60;
export const GB_CYCLES_PER_FRAME = Math.floor(GB_CPU_FREQUENCY / GB_FRAME_RATE);

export const MOBILE_LOGIN_PASSWORD_LENGTH = 17;
export const NUM_NON_TROPHY_DECOS = 43;
export const BLUE_CARD_POINT_CAP = 30;
export const BATTLETOWER_STREAK_LENGTH = 7;
export const MAX_MONEY = 999_999;
export const MAX_COINS = 9_999;
// ASM: pokecrystal_disassembly/constants/misc_constants.asm::HOF_MASTER_COUNT
export const HOF_MASTER_COUNT = 200;

/**
 * An enumeration of all Pokémon species, indexed by their National Pokédex number.
 *
 * The integer value of each member corresponds to the official Pokédex number.
 * The special value `EGG` is also included.
 */
export enum Pokemon {
  BULBASAUR = 1,
  IVYSAUR,
  VENUSAUR,
  CHARMANDER,
  CHARMELEON,
  CHARIZARD,
  SQUIRTLE,
  WARTORTLE,
  BLASTOISE,
  CATERPIE,
  METAPOD,
  BUTTERFREE,
  WEEDLE,
  KAKUNA,
  BEEDRILL,
  PIDGEY,
  PIDGEOTTO,
  PIDGEOT,
  RATTATA,
  RATICATE,
  SPEAROW,
  FEAROW,
  EKANS,
  ARBOK,
  PIKACHU,
  RAICHU,
  SANDSHREW,
  SANDSLASH,
  NIDORAN_F,
  NIDORINA,
  NIDOQUEEN,
  NIDORAN_M,
  NIDORINO,
  NIDOKING,
  CLEFAIRY,
  CLEFABLE,
  VULPIX,
  NINETALES,
  JIGGLYPUFF,
  WIGGLYTUFF,
  ZUBAT,
  GOLBAT,
  ODDISH,
  GLOOM,
  VILEPLUME,
  PARAS,
  PARASECT,
  VENONAT,
  VENOMOTH,
  DIGLETT,
  DUGTRIO,
  MEOWTH,
  PERSIAN,
  PSYDUCK,
  GOLDUCK,
  MANKEY,
  PRIMEAPE,
  GROWLITHE,
  ARCANINE,
  POLIWAG,
  POLIWHIRL,
  POLIWRATH,
  ABRA,
  KADABRA,
  ALAKAZAM,
  MACHOP,
  MACHOKE,
  MACHAMP,
  BELLSPROUT,
  WEEPINBELL,
  VICTREEBEL,
  TENTACOOL,
  TENTACRUEL,
  GEODUDE,
  GRAVELER,
  GOLEM,
  PONYTA,
  RAPIDASH,
  SLOWPOKE,
  SLOWBRO,
  MAGNEMITE,
  MAGNETON,
  FARFETCH_D,
  DODUO,
  DODRIO,
  SEEL,
  DEWGONG,
  GRIMER,
  MUK,
  SHELLDER,
  CLOYSTER,
  GASTLY,
  HAUNTER,
  GENGAR,
  ONIX,
  DROWZEE,
  HYPNO,
  KRABBY,
  KINGLER,
  VOLTORB,
  ELECTRODE,
  EXEGGCUTE,
  EXEGGUTOR,
  CUBONE,
  MAROWAK,
  HITMONLEE,
  HITMONCHAN,
  LICKITUNG,
  KOFFING,
  WEEZING,
  RHYHORN,
  RHYDON,
  CHANSEY,
  TANGELA,
  KANGASKHAN,
  HORSEA,
  SEADRA,
  GOLDEEN,
  SEAKING,
  STARYU,
  STARMIE,
  MR__MIME,
  SCYTHER,
  JYNX,
  ELECTABUZZ,
  MAGMAR,
  PINSIR,
  TAUROS,
  MAGIKARP,
  GYARADOS,
  LAPRAS,
  DITTO,
  EEVEE,
  VAPOREON,
  JOLTEON,
  FLAREON,
  PORYGON,
  OMANYTE,
  OMASTAR,
  KABUTO,
  KABUTOPS,
  AERODACTYL,
  SNORLAX,
  ARTICUNO,
  ZAPDOS,
  MOLTRES,
  DRATINI,
  DRAGONAIR,
  DRAGONITE,
  MEWTWO,
  MEW,
  CHIKORITA,
  BAYLEEF,
  MEGANIUM,
  CYNDAQUIL,
  QUILAVA,
  TYPHLOSION,
  TOTODILE,
  CROCONAW,
  FERALIGATR,
  SENTRET,
  FURRET,
  HOOTHOOT,
  NOCTOWL,
  LEDYBA,
  LEDIAN,
  SPINARAK,
  ARIADOS,
  CROBAT,
  CHINCHOU,
  LANTURN,
  PICHU,
  CLEFFA,
  IGGLYBUFF,
  TOGEPI,
  TOGETIC,
  NATU,
  XATU,
  MAREEP,
  FLAAFFY,
  AMPHAROS,
  BELLOSSOM,
  MARILL,
  AZUMARILL,
  SUDOWOODO,
  POLITOED,
  HOPPIP,
  SKIPLOOM,
  JUMPLUFF,
  AIPOM,
  SUNKERN,
  SUNFLORA,
  YANMA,
  WOOPER,
  QUAGSIRE,
  ESPEON,
  UMBREON,
  MURKROW,
  SLOWKING,
  MISDREAVUS,
  UNOWN,
  WOBBUFFET,
  GIRAFARIG,
  PINECO,
  FORRETRESS,
  DUNSPARCE,
  GLIGAR,
  STEELIX,
  SNUBBULL,
  GRANBULL,
  QWILFISH,
  SCIZOR,
  SHUCKLE,
  HERACROSS,
  SNEASEL,
  TEDDIURSA,
  URSARING,
  SLUGMA,
  MAGCARGO,
  SWINUB,
  PILOSWINE,
  CORSOLA,
  REMORAID,
  OCTILLERY,
  DELIBIRD,
  MANTINE,
  SKARMORY,
  HOUNDOUR,
  HOUNDOOM,
  KINGDRA,
  PHANPY,
  DONPHAN,
  PORYGON2,
  STANTLER,
  SMEARGLE,
  TYROGUE,
  HITMONTOP,
  SMOOCHUM,
  ELEKID,
  MAGBY,
  MILTANK,
  BLISSEY,
  RAIKOU,
  ENTEI,
  SUICUNE,
  LARVITAR,
  PUPITAR,
  TYRANITAR,
  LUGIA,
  HO_OH,
  CELEBI,
  EGG = 0xfd,
}

export const NUM_POKEMON = 251;

/**
 * An enumeration of the different forms of the Pokémon Unown.
 *
 * Each member represents one of the 26 alphabetic forms, from A to Z.
 */
export enum UnownForm {
  A = 1,
  B,
  C,
  D,
  E,
  F,
  G,
  H,
  I,
  J,
  K,
  L,
  M,
  N,
  O,
  P,
  Q,
  R,
  S,
  T,
  U,
  V,
  W,
  X,
  Y,
  Z,
}

export const NUM_UNOWN = 26;

/** Length of the general name fields stored by the game (matching the ASM). */
export const NAME_LENGTH = 11;

/** Maximum length of the player's name stored in SRAM (matching the ASM). */
export const PLAYER_NAME_LENGTH = 8;

/** Characters per line in a mail message (matching the ASM mail format). */
export const MAIL_LINE_LENGTH = 0x10;

/** Maximum characters stored for a mail message (matching the ASM mail format). */
export const MAIL_MSG_LENGTH = 0x20;

/** Maximum number of mail messages that can be stored in the player's mailbox. */
export const MAILBOX_CAPACITY = 10;

/** Total byte size of a mail message data block (matching the ASM). */
export const MAIL_STRUCT_LENGTH = 0x2f;

/** Number of Pokémon slots available in the player's party. */
export const PARTY_SIZE = 6;

/** How many Pokémon are stored in each Hall of Fame entry. */
export const HALL_OF_FAME_TEAM_SIZE = PARTY_SIZE;

/** Maximum count of Hall of Fame entries saved in SRAM. */
export const NUM_HALL_OF_FAME_ENTRIES = 30;

/** Length of the nickname field for Hall of Fame Pokémon (NAME_LENGTH - 1). */
export const HALL_OF_FAME_NICKNAME_LENGTH = 10;

/** Number of characters stored for a link battle trainer name (NAME_LENGTH - 1). */
export const LINK_BATTLE_RECORD_NAME_LENGTH = 10;

/** Total link battle records conserved on the cartridge. */
export const NUM_LINK_BATTLE_RECORDS = 5;

/** Number of overworld steps the Day Care counts per level gain. */
export const DAY_CARE_STEPS_PER_LEVEL = 256;

/**
 * An enumeration for the game regions.
 *
 * @property JOHTO - The Johto region.
 * @property KANTO - The Kanto region.
 */
export enum Region {
  JOHTO = 0,
  KANTO = 1,
}

export const LANDMARK_FAST_SHIP = 0x5f;
export const LANDMARK_SPECIAL = 0x00;
export const KANTO_LANDMARK = 0x36;

export const TMHM_MOVES: MoveName[] = [
  MoveName.DYNAMICPUNCH,
  MoveName.HEADBUTT,
  MoveName.CURSE,
  MoveName.ROLLOUT,
  MoveName.ROAR,
  MoveName.TOXIC,
  MoveName.ZAP_CANNON,
  MoveName.ROCK_SMASH,
  MoveName.PSYCH_UP,
  MoveName.HIDDEN_POWER,
  MoveName.SUNNY_DAY,
  MoveName.SWEET_SCENT,
  MoveName.SNORE,
  MoveName.BLIZZARD,
  MoveName.HYPER_BEAM,
  MoveName.ICY_WIND,
  MoveName.PROTECT,
  MoveName.RAIN_DANCE,
  MoveName.GIGA_DRAIN,
  MoveName.ENDURE,
  MoveName.FRUSTRATION,
  MoveName.IRON_TAIL,
  MoveName.DRAGONBREATH,
  MoveName.THUNDER,
  MoveName.EARTHQUAKE,
  MoveName.RETURN,
  MoveName.DIG,
  MoveName.PSYCHIC_M,
  MoveName.SHADOW_BALL,
  MoveName.MUD_SLAP,
  MoveName.DOUBLE_TEAM,
  MoveName.ICE_PUNCH,
  MoveName.SWAGGER,
  MoveName.SLEEP_TALK,
  MoveName.SLUDGE_BOMB,
  MoveName.SANDSTORM,
  MoveName.FIRE_BLAST,
  MoveName.SWIFT,
  MoveName.DEFENSE_CURL,
  MoveName.THUNDERPUNCH,
  MoveName.DREAM_EATER,
  MoveName.DETECT,
  MoveName.REST,
  MoveName.ATTRACT,
  MoveName.THIEF,
  MoveName.STEEL_WING,
  MoveName.FIRE_PUNCH,
  MoveName.FURY_CUTTER,
  MoveName.NIGHTMARE,
  MoveName.FLAMETHROWER,
  MoveName.CUT,
  MoveName.FLY,
  MoveName.SURF,
  MoveName.STRENGTH,
  MoveName.FLASH,
  MoveName.WHIRLPOOL,
  MoveName.WATERFALL,
];

/** Maximum number of unique items that can be stored in the Pokémon Center PC. */
export const MAX_PC_ITEMS = 50;

/** Maximum quantity that a single Pokémon Center PC item slot can hold. */
export const MAX_PC_ITEM_QUANTITY = 99;

/** Number of Pokémon PC boxes available to the player. */
export const MAX_PC_BOXES = 14;

/** Maximum number of Pokémon that can be stored in a single PC box. */
export const MAX_BOX_MONS = 20;

export const JOY_RIGHT = 0x01;
export const JOY_LEFT = 0x02;
export const JOY_UP = 0x04;
export const JOY_DOWN = 0x08;
export const JOY_A = 0x10;
export const JOY_B = 0x20;
export const JOY_SELECT = 0x40;
export const JOY_START = 0x80;
