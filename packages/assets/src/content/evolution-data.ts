import { z } from 'zod';

export const evolutionMethodSchema = z.enum([
  'LEVEL',
  'ITEM',
  'HAPPINESS',
  'TRADE',
  'STAT',
]);

export type EvolutionMethod = z.infer<typeof evolutionMethodSchema>;

export const evolutionDataSchema = z.object({
  method: evolutionMethodSchema,
  level: z.number().optional(),
  item: z.string().optional(),
  held_item: z.string().optional(),
  happiness: z.string().optional(),
  stat_ratio: z.string().optional(),
  species: z.string(),
});

export type EvolutionData = z.infer<typeof evolutionDataSchema>;

export const pokemonEvolutionDataSchema = z.object({
  species: z.string(),
  evolutions: z.array(evolutionDataSchema),
});

export type PokemonEvolutionData = z.infer<typeof pokemonEvolutionDataSchema>;

export const evolutionData: PokemonEvolutionData[] = [
  {
    "species": "BULBASAUR",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 16,
        "species": "IVYSAUR"
      }
    ]
  },
  {
    "species": "IVYSAUR",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 32,
        "species": "VENUSAUR"
      }
    ]
  },
  {
    "species": "CHARMANDER",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 16,
        "species": "CHARMELEON"
      }
    ]
  },
  {
    "species": "CHARMELEON",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 36,
        "species": "CHARIZARD"
      }
    ]
  },
  {
    "species": "SQUIRTLE",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 16,
        "species": "WARTORTLE"
      }
    ]
  },
  {
    "species": "WARTORTLE",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 36,
        "species": "BLASTOISE"
      }
    ]
  },
  {
    "species": "CATERPIE",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 7,
        "species": "METAPOD"
      }
    ]
  },
  {
    "species": "METAPOD",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 10,
        "species": "BUTTERFREE"
      }
    ]
  },
  {
    "species": "WEEDLE",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 7,
        "species": "KAKUNA"
      }
    ]
  },
  {
    "species": "KAKUNA",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 10,
        "species": "BEEDRILL"
      }
    ]
  },
  {
    "species": "PIDGEY",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 18,
        "species": "PIDGEOTTO"
      }
    ]
  },
  {
    "species": "PIDGEOTTO",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 36,
        "species": "PIDGEOT"
      }
    ]
  },
  {
    "species": "RATTATA",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 20,
        "species": "RATICATE"
      }
    ]
  },
  {
    "species": "SPEAROW",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 20,
        "species": "FEAROW"
      }
    ]
  },
  {
    "species": "EKANS",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 22,
        "species": "ARBOK"
      }
    ]
  },
  {
    "species": "PIKACHU",
    "evolutions": [
      {
        "method": "ITEM",
        "item": "THUNDERSTONE",
        "species": "RAICHU"
      }
    ]
  },
  {
    "species": "SANDSHREW",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 22,
        "species": "SANDSLASH"
      }
    ]
  },
  {
    "species": "NIDORANF",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 16,
        "species": "NIDORINA"
      }
    ]
  },
  {
    "species": "NIDORINA",
    "evolutions": [
      {
        "method": "ITEM",
        "item": "MOON_STONE",
        "species": "NIDOQUEEN"
      }
    ]
  },
  {
    "species": "NIDORANM",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 16,
        "species": "NIDORINO"
      }
    ]
  },
  {
    "species": "NIDORINO",
    "evolutions": [
      {
        "method": "ITEM",
        "item": "MOON_STONE",
        "species": "NIDOKING"
      }
    ]
  },
  {
    "species": "CLEFAIRY",
    "evolutions": [
      {
        "method": "ITEM",
        "item": "MOON_STONE",
        "species": "CLEFABLE"
      }
    ]
  },
  {
    "species": "VULPIX",
    "evolutions": [
      {
        "method": "ITEM",
        "item": "FIRE_STONE",
        "species": "NINETALES"
      }
    ]
  },
  {
    "species": "JIGGLYPUFF",
    "evolutions": [
      {
        "method": "ITEM",
        "item": "MOON_STONE",
        "species": "WIGGLYTUFF"
      }
    ]
  },
  {
    "species": "ZUBAT",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 22,
        "species": "GOLBAT"
      }
    ]
  },
  {
    "species": "GOLBAT",
    "evolutions": [
      {
        "method": "HAPPINESS",
        "happiness": "TR_ANYTIME",
        "species": "CROBAT"
      }
    ]
  },
  {
    "species": "ODDISH",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 21,
        "species": "GLOOM"
      }
    ]
  },
  {
    "species": "GLOOM",
    "evolutions": [
      {
        "method": "ITEM",
        "item": "LEAF_STONE",
        "species": "VILEPLUME"
      },
      {
        "method": "ITEM",
        "item": "SUN_STONE",
        "species": "BELLOSSOM"
      }
    ]
  },
  {
    "species": "PARAS",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 24,
        "species": "PARASECT"
      }
    ]
  },
  {
    "species": "VENONAT",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 31,
        "species": "VENOMOTH"
      }
    ]
  },
  {
    "species": "DIGLETT",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 26,
        "species": "DUGTRIO"
      }
    ]
  },
  {
    "species": "MEOWTH",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 28,
        "species": "PERSIAN"
      }
    ]
  },
  {
    "species": "PSYDUCK",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 33,
        "species": "GOLDUCK"
      }
    ]
  },
  {
    "species": "MANKEY",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 28,
        "species": "PRIMEAPE"
      }
    ]
  },
  {
    "species": "GROWLITHE",
    "evolutions": [
      {
        "method": "ITEM",
        "item": "FIRE_STONE",
        "species": "ARCANINE"
      }
    ]
  },
  {
    "species": "POLIWAG",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 25,
        "species": "POLIWHIRL"
      }
    ]
  },
  {
    "species": "POLIWHIRL",
    "evolutions": [
      {
        "method": "ITEM",
        "item": "WATER_STONE",
        "species": "POLIWRATH"
      },
      {
        "method": "TRADE",
        "held_item": "KINGS_ROCK",
        "species": "POLITOED"
      }
    ]
  },
  {
    "species": "ABRA",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 16,
        "species": "KADABRA"
      }
    ]
  },
  {
    "species": "KADABRA",
    "evolutions": [
      {
        "method": "TRADE",
        "held_item": "-1",
        "species": "ALAKAZAM"
      }
    ]
  },
  {
    "species": "MACHOP",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 28,
        "species": "MACHOKE"
      }
    ]
  },
  {
    "species": "MACHOKE",
    "evolutions": [
      {
        "method": "TRADE",
        "held_item": "-1",
        "species": "MACHAMP"
      }
    ]
  },
  {
    "species": "BELLSPROUT",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 21,
        "species": "WEEPINBELL"
      }
    ]
  },
  {
    "species": "WEEPINBELL",
    "evolutions": [
      {
        "method": "ITEM",
        "item": "LEAF_STONE",
        "species": "VICTREEBEL"
      }
    ]
  },
  {
    "species": "TENTACOOL",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 30,
        "species": "TENTACRUEL"
      }
    ]
  },
  {
    "species": "GEODUDE",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 25,
        "species": "GRAVELER"
      }
    ]
  },
  {
    "species": "GRAVELER",
    "evolutions": [
      {
        "method": "TRADE",
        "held_item": "-1",
        "species": "GOLEM"
      }
    ]
  },
  {
    "species": "PONYTA",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 40,
        "species": "RAPIDASH"
      }
    ]
  },
  {
    "species": "SLOWPOKE",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 37,
        "species": "SLOWBRO"
      },
      {
        "method": "TRADE",
        "held_item": "KINGS_ROCK",
        "species": "SLOWKING"
      }
    ]
  },
  {
    "species": "MAGNEMITE",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 30,
        "species": "MAGNETON"
      }
    ]
  },
  {
    "species": "DODUO",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 31,
        "species": "DODRIO"
      }
    ]
  },
  {
    "species": "SEEL",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 34,
        "species": "DEWGONG"
      }
    ]
  },
  {
    "species": "GRIMER",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 38,
        "species": "MUK"
      }
    ]
  },
  {
    "species": "SHELLDER",
    "evolutions": [
      {
        "method": "ITEM",
        "item": "WATER_STONE",
        "species": "CLOYSTER"
      }
    ]
  },
  {
    "species": "GASTLY",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 25,
        "species": "HAUNTER"
      }
    ]
  },
  {
    "species": "HAUNTER",
    "evolutions": [
      {
        "method": "TRADE",
        "held_item": "-1",
        "species": "GENGAR"
      }
    ]
  },
  {
    "species": "ONIX",
    "evolutions": [
      {
        "method": "TRADE",
        "held_item": "METAL_COAT",
        "species": "STEELIX"
      }
    ]
  },
  {
    "species": "DROWZEE",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 26,
        "species": "HYPNO"
      }
    ]
  },
  {
    "species": "KRABBY",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 28,
        "species": "KINGLER"
      }
    ]
  },
  {
    "species": "VOLTORB",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 30,
        "species": "ELECTRODE"
      }
    ]
  },
  {
    "species": "EXEGGCUTE",
    "evolutions": [
      {
        "method": "ITEM",
        "item": "LEAF_STONE",
        "species": "EXEGGUTOR"
      }
    ]
  },
  {
    "species": "CUBONE",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 28,
        "species": "MAROWAK"
      }
    ]
  },
  {
    "species": "KOFFING",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 35,
        "species": "WEEZING"
      }
    ]
  },
  {
    "species": "RHYHORN",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 42,
        "species": "RHYDON"
      }
    ]
  },
  {
    "species": "CHANSEY",
    "evolutions": [
      {
        "method": "HAPPINESS",
        "happiness": "TR_ANYTIME",
        "species": "BLISSEY"
      }
    ]
  },
  {
    "species": "HORSEA",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 32,
        "species": "SEADRA"
      }
    ]
  },
  {
    "species": "SEADRA",
    "evolutions": [
      {
        "method": "TRADE",
        "held_item": "DRAGON_SCALE",
        "species": "KINGDRA"
      }
    ]
  },
  {
    "species": "GOLDEEN",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 33,
        "species": "SEAKING"
      }
    ]
  },
  {
    "species": "STARYU",
    "evolutions": [
      {
        "method": "ITEM",
        "item": "WATER_STONE",
        "species": "STARMIE"
      }
    ]
  },
  {
    "species": "SCYTHER",
    "evolutions": [
      {
        "method": "TRADE",
        "held_item": "METAL_COAT",
        "species": "SCIZOR"
      }
    ]
  },
  {
    "species": "MAGIKARP",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 20,
        "species": "GYARADOS"
      }
    ]
  },
  {
    "species": "EEVEE",
    "evolutions": [
      {
        "method": "ITEM",
        "item": "THUNDERSTONE",
        "species": "JOLTEON"
      },
      {
        "method": "ITEM",
        "item": "WATER_STONE",
        "species": "VAPOREON"
      },
      {
        "method": "ITEM",
        "item": "FIRE_STONE",
        "species": "FLAREON"
      },
      {
        "method": "HAPPINESS",
        "happiness": "TR_MORNDAY",
        "species": "ESPEON"
      },
      {
        "method": "HAPPINESS",
        "happiness": "TR_NITE",
        "species": "UMBREON"
      }
    ]
  },
  {
    "species": "PORYGON",
    "evolutions": [
      {
        "method": "TRADE",
        "held_item": "UP_GRADE",
        "species": "PORYGON2"
      }
    ]
  },
  {
    "species": "OMANYTE",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 40,
        "species": "OMASTAR"
      }
    ]
  },
  {
    "species": "KABUTO",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 40,
        "species": "KABUTOPS"
      }
    ]
  },
  {
    "species": "DRATINI",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 30,
        "species": "DRAGONAIR"
      }
    ]
  },
  {
    "species": "DRAGONAIR",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 55,
        "species": "DRAGONITE"
      }
    ]
  },
  {
    "species": "CHIKORITA",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 16,
        "species": "BAYLEEF"
      }
    ]
  },
  {
    "species": "BAYLEEF",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 32,
        "species": "MEGANIUM"
      }
    ]
  },
  {
    "species": "CYNDAQUIL",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 14,
        "species": "QUILAVA"
      }
    ]
  },
  {
    "species": "QUILAVA",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 36,
        "species": "TYPHLOSION"
      }
    ]
  },
  {
    "species": "TOTODILE",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 18,
        "species": "CROCONAW"
      }
    ]
  },
  {
    "species": "CROCONAW",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 30,
        "species": "FERALIGATR"
      }
    ]
  },
  {
    "species": "SENTRET",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 15,
        "species": "FURRET"
      }
    ]
  },
  {
    "species": "HOOTHOOT",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 20,
        "species": "NOCTOWL"
      }
    ]
  },
  {
    "species": "LEDYBA",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 18,
        "species": "LEDIAN"
      }
    ]
  },
  {
    "species": "SPINARAK",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 22,
        "species": "ARIADOS"
      }
    ]
  },
  {
    "species": "CHINCHOU",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 27,
        "species": "LANTURN"
      }
    ]
  },
  {
    "species": "PICHU",
    "evolutions": [
      {
        "method": "HAPPINESS",
        "happiness": "TR_ANYTIME",
        "species": "PIKACHU"
      }
    ]
  },
  {
    "species": "CLEFFA",
    "evolutions": [
      {
        "method": "HAPPINESS",
        "happiness": "TR_ANYTIME",
        "species": "CLEFAIRY"
      }
    ]
  },
  {
    "species": "IGGLYBUFF",
    "evolutions": [
      {
        "method": "HAPPINESS",
        "happiness": "TR_ANYTIME",
        "species": "JIGGLYPUFF"
      }
    ]
  },
  {
    "species": "TOGEPI",
    "evolutions": [
      {
        "method": "HAPPINESS",
        "happiness": "TR_ANYTIME",
        "species": "TOGETIC"
      }
    ]
  },
  {
    "species": "NATU",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 25,
        "species": "XATU"
      }
    ]
  },
  {
    "species": "MAREEP",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 15,
        "species": "FLAAFFY"
      }
    ]
  },
  {
    "species": "FLAAFFY",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 30,
        "species": "AMPHAROS"
      }
    ]
  },
  {
    "species": "MARILL",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 18,
        "species": "AZUMARILL"
      }
    ]
  },
  {
    "species": "HOPPIP",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 18,
        "species": "SKIPLOOM"
      }
    ]
  },
  {
    "species": "SKIPLOOM",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 27,
        "species": "JUMPLUFF"
      }
    ]
  },
  {
    "species": "SUNKERN",
    "evolutions": [
      {
        "method": "ITEM",
        "item": "SUN_STONE",
        "species": "SUNFLORA"
      }
    ]
  },
  {
    "species": "WOOPER",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 20,
        "species": "QUAGSIRE"
      }
    ]
  },
  {
    "species": "PINECO",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 31,
        "species": "FORRETRESS"
      }
    ]
  },
  {
    "species": "SNUBBULL",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 23,
        "species": "GRANBULL"
      }
    ]
  },
  {
    "species": "TEDDIURSA",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 30,
        "species": "URSARING"
      }
    ]
  },
  {
    "species": "SLUGMA",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 38,
        "species": "MAGCARGO"
      }
    ]
  },
  {
    "species": "SWINUB",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 33,
        "species": "PILOSWINE"
      }
    ]
  },
  {
    "species": "REMORAID",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 25,
        "species": "OCTILLERY"
      }
    ]
  },
  {
    "species": "HOUNDOUR",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 24,
        "species": "HOUNDOOM"
      }
    ]
  },
  {
    "species": "PHANPY",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 25,
        "species": "DONPHAN"
      }
    ]
  },
  {
    "species": "TYROGUE",
    "evolutions": [
      {
        "method": "STAT",
        "level": 20,
        "stat_ratio": "ATK_LT_DEF",
        "species": "HITMONCHAN"
      },
      {
        "method": "STAT",
        "level": 20,
        "stat_ratio": "ATK_GT_DEF",
        "species": "HITMONLEE"
      },
      {
        "method": "STAT",
        "level": 20,
        "stat_ratio": "ATK_EQ_DEF",
        "species": "HITMONTOP"
      }
    ]
  },
  {
    "species": "SMOOCHUM",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 30,
        "species": "JYNX"
      }
    ]
  },
  {
    "species": "ELEKID",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 30,
        "species": "ELECTABUZZ"
      }
    ]
  },
  {
    "species": "MAGBY",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 30,
        "species": "MAGMAR"
      }
    ]
  },
  {
    "species": "LARVITAR",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 30,
        "species": "PUPITAR"
      }
    ]
  },
  {
    "species": "PUPITAR",
    "evolutions": [
      {
        "method": "LEVEL",
        "level": 55,
        "species": "TYRANITAR"
      }
    ]
  }
];
