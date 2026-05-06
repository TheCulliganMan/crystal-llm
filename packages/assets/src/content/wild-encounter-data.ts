import { z } from 'zod';

export const WildEncounterSchema = z.object({
  level: z.number(),
  species: z.string(),
});
export type WildEncounter = z.infer<typeof WildEncounterSchema>;

export const WildEncounterTableSchema = z.object({
  morning: z.array(WildEncounterSchema),
  day: z.array(WildEncounterSchema),
  night: z.array(WildEncounterSchema),
});
export type WildEncounterTable = z.infer<typeof WildEncounterTableSchema>;

export const WildEncounterDataSchema = z.object({
  map_name: z.string(),
  grass_rates: z.record(z.string(), z.number()).optional().nullable(),
  water_rate: z.number().optional().nullable(),
  grass: WildEncounterTableSchema.optional().nullable(),
  water: WildEncounterTableSchema.optional().nullable(),
});
export type WildEncounterData = z.infer<typeof WildEncounterDataSchema>;

export const wildEncounterData: WildEncounterData[] = [
  {
    "map_name": "SPROUT_TOWER_2F",
    "grass_rates": {
      "morning": 2,
      "day": 2,
      "night": 2
    },
    "water_rate": null,
    "grass": {
      "morning": [
        {
          "level": 3,
          "species": "RATTATA"
        },
        {
          "level": 4,
          "species": "RATTATA"
        },
        {
          "level": 5,
          "species": "RATTATA"
        },
        {
          "level": 3,
          "species": "RATTATA"
        },
        {
          "level": 6,
          "species": "RATTATA"
        },
        {
          "level": 5,
          "species": "RATTATA"
        },
        {
          "level": 5,
          "species": "RATTATA"
        }
      ],
      "day": [
        {
          "level": 3,
          "species": "RATTATA"
        },
        {
          "level": 4,
          "species": "RATTATA"
        },
        {
          "level": 5,
          "species": "RATTATA"
        },
        {
          "level": 3,
          "species": "RATTATA"
        },
        {
          "level": 6,
          "species": "RATTATA"
        },
        {
          "level": 5,
          "species": "RATTATA"
        },
        {
          "level": 5,
          "species": "RATTATA"
        }
      ],
      "night": [
        {
          "level": 3,
          "species": "GASTLY"
        },
        {
          "level": 4,
          "species": "GASTLY"
        },
        {
          "level": 5,
          "species": "GASTLY"
        },
        {
          "level": 3,
          "species": "RATTATA"
        },
        {
          "level": 6,
          "species": "GASTLY"
        },
        {
          "level": 5,
          "species": "RATTATA"
        },
        {
          "level": 5,
          "species": "RATTATA"
        }
      ]
    },
    "water": {
      "morning": [],
      "day": [],
      "night": []
    }
  },
  {
    "map_name": "SPROUT_TOWER_3F",
    "grass_rates": {
      "morning": 2,
      "day": 2,
      "night": 2
    },
    "water_rate": null,
    "grass": {
      "morning": [
        {
          "level": 3,
          "species": "RATTATA"
        },
        {
          "level": 4,
          "species": "RATTATA"
        },
        {
          "level": 5,
          "species": "RATTATA"
        },
        {
          "level": 3,
          "species": "RATTATA"
        },
        {
          "level": 6,
          "species": "RATTATA"
        },
        {
          "level": 5,
          "species": "RATTATA"
        },
        {
          "level": 5,
          "species": "RATTATA"
        }
      ],
      "day": [
        {
          "level": 3,
          "species": "RATTATA"
        },
        {
          "level": 4,
          "species": "RATTATA"
        },
        {
          "level": 5,
          "species": "RATTATA"
        },
        {
          "level": 3,
          "species": "RATTATA"
        },
        {
          "level": 6,
          "species": "RATTATA"
        },
        {
          "level": 5,
          "species": "RATTATA"
        },
        {
          "level": 5,
          "species": "RATTATA"
        }
      ],
      "night": [
        {
          "level": 3,
          "species": "GASTLY"
        },
        {
          "level": 4,
          "species": "GASTLY"
        },
        {
          "level": 5,
          "species": "GASTLY"
        },
        {
          "level": 3,
          "species": "RATTATA"
        },
        {
          "level": 6,
          "species": "GASTLY"
        },
        {
          "level": 5,
          "species": "RATTATA"
        },
        {
          "level": 5,
          "species": "RATTATA"
        }
      ]
    },
    "water": {
      "morning": [],
      "day": [],
      "night": []
    }
  },
  {
    "map_name": "TIN_TOWER_2F",
    "grass_rates": {
      "morning": 2,
      "day": 2,
      "night": 2
    },
    "water_rate": null,
    "grass": {
      "morning": [
        {
          "level": 20,
          "species": "RATTATA"
        },
        {
          "level": 21,
          "species": "RATTATA"
        },
        {
          "level": 22,
          "species": "RATTATA"
        },
        {
          "level": 22,
          "species": "RATTATA"
        },
        {
          "level": 23,
          "species": "RATTATA"
        },
        {
          "level": 24,
          "species": "RATTATA"
        },
        {
          "level": 24,
          "species": "RATTATA"
        }
      ],
      "day": [
        {
          "level": 20,
          "species": "RATTATA"
        },
        {
          "level": 21,
          "species": "RATTATA"
        },
        {
          "level": 22,
          "species": "RATTATA"
        },
        {
          "level": 22,
          "species": "RATTATA"
        },
        {
          "level": 23,
          "species": "RATTATA"
        },
        {
          "level": 24,
          "species": "RATTATA"
        },
        {
          "level": 24,
          "species": "RATTATA"
        }
      ],
      "night": [
        {
          "level": 20,
          "species": "GASTLY"
        },
        {
          "level": 21,
          "species": "GASTLY"
        },
        {
          "level": 22,
          "species": "GASTLY"
        },
        {
          "level": 22,
          "species": "RATTATA"
        },
        {
          "level": 23,
          "species": "RATTATA"
        },
        {
          "level": 24,
          "species": "RATTATA"
        },
        {
          "level": 24,
          "species": "RATTATA"
        }
      ]
    },
    "water": {
      "morning": [],
      "day": [],
      "night": []
    }
  },
  {
    "map_name": "TIN_TOWER_3F",
    "grass_rates": {
      "morning": 2,
      "day": 2,
      "night": 2
    },
    "water_rate": null,
    "grass": {
      "morning": [
        {
          "level": 20,
          "species": "RATTATA"
        },
        {
          "level": 21,
          "species": "RATTATA"
        },
        {
          "level": 22,
          "species": "RATTATA"
        },
        {
          "level": 22,
          "species": "RATTATA"
        },
        {
          "level": 23,
          "species": "RATTATA"
        },
        {
          "level": 24,
          "species": "RATTATA"
        },
        {
          "level": 24,
          "species": "RATTATA"
        }
      ],
      "day": [
        {
          "level": 20,
          "species": "RATTATA"
        },
        {
          "level": 21,
          "species": "RATTATA"
        },
        {
          "level": 22,
          "species": "RATTATA"
        },
        {
          "level": 22,
          "species": "RATTATA"
        },
        {
          "level": 23,
          "species": "RATTATA"
        },
        {
          "level": 24,
          "species": "RATTATA"
        },
        {
          "level": 24,
          "species": "RATTATA"
        }
      ],
      "night": [
        {
          "level": 20,
          "species": "GASTLY"
        },
        {
          "level": 21,
          "species": "GASTLY"
        },
        {
          "level": 22,
          "species": "GASTLY"
        },
        {
          "level": 22,
          "species": "RATTATA"
        },
        {
          "level": 23,
          "species": "RATTATA"
        },
        {
          "level": 24,
          "species": "RATTATA"
        },
        {
          "level": 24,
          "species": "RATTATA"
        }
      ]
    },
    "water": {
      "morning": [],
      "day": [],
      "night": []
    }
  },
  {
    "map_name": "TIN_TOWER_4F",
    "grass_rates": {
      "morning": 2,
      "day": 2,
      "night": 2
    },
    "water_rate": null,
    "grass": {
      "morning": [
        {
          "level": 20,
          "species": "RATTATA"
        },
        {
          "level": 21,
          "species": "RATTATA"
        },
        {
          "level": 22,
          "species": "RATTATA"
        },
        {
          "level": 22,
          "species": "RATTATA"
        },
        {
          "level": 23,
          "species": "RATTATA"
        },
        {
          "level": 24,
          "species": "RATTATA"
        },
        {
          "level": 24,
          "species": "RATTATA"
        }
      ],
      "day": [
        {
          "level": 20,
          "species": "RATTATA"
        },
        {
          "level": 21,
          "species": "RATTATA"
        },
        {
          "level": 22,
          "species": "RATTATA"
        },
        {
          "level": 22,
          "species": "RATTATA"
        },
        {
          "level": 23,
          "species": "RATTATA"
        },
        {
          "level": 24,
          "species": "RATTATA"
        },
        {
          "level": 24,
          "species": "RATTATA"
        }
      ],
      "night": [
        {
          "level": 20,
          "species": "GASTLY"
        },
        {
          "level": 21,
          "species": "GASTLY"
        },
        {
          "level": 22,
          "species": "GASTLY"
        },
        {
          "level": 22,
          "species": "RATTATA"
        },
        {
          "level": 23,
          "species": "RATTATA"
        },
        {
          "level": 24,
          "species": "RATTATA"
        },
        {
          "level": 24,
          "species": "RATTATA"
        }
      ]
    },
    "water": {
      "morning": [],
      "day": [],
      "night": []
    }
  },
  {
    "map_name": "TIN_TOWER_5F",
    "grass_rates": {
      "morning": 2,
      "day": 2,
      "night": 2
    },
    "water_rate": null,
    "grass": {
      "morning": [
        {
          "level": 20,
          "species": "RATTATA"
        },
        {
          "level": 21,
          "species": "RATTATA"
        },
        {
          "level": 22,
          "species": "RATTATA"
        },
        {
          "level": 22,
          "species": "RATTATA"
        },
        {
          "level": 23,
          "species": "RATTATA"
        },
        {
          "level": 24,
          "species": "RATTATA"
        },
        {
          "level": 24,
          "species": "RATTATA"
        }
      ],
      "day": [
        {
          "level": 20,
          "species": "RATTATA"
        },
        {
          "level": 21,
          "species": "RATTATA"
        },
        {
          "level": 22,
          "species": "RATTATA"
        },
        {
          "level": 22,
          "species": "RATTATA"
        },
        {
          "level": 23,
          "species": "RATTATA"
        },
        {
          "level": 24,
          "species": "RATTATA"
        },
        {
          "level": 24,
          "species": "RATTATA"
        }
      ],
      "night": [
        {
          "level": 20,
          "species": "GASTLY"
        },
        {
          "level": 21,
          "species": "GASTLY"
        },
        {
          "level": 22,
          "species": "GASTLY"
        },
        {
          "level": 22,
          "species": "RATTATA"
        },
        {
          "level": 23,
          "species": "RATTATA"
        },
        {
          "level": 24,
          "species": "RATTATA"
        },
        {
          "level": 24,
          "species": "RATTATA"
        }
      ]
    },
    "water": {
      "morning": [],
      "day": [],
      "night": []
    }
  },
  {
    "map_name": "TIN_TOWER_6F",
    "grass_rates": {
      "morning": 2,
      "day": 2,
      "night": 2
    },
    "water_rate": null,
    "grass": {
      "morning": [
        {
          "level": 20,
          "species": "RATTATA"
        },
        {
          "level": 21,
          "species": "RATTATA"
        },
        {
          "level": 22,
          "species": "RATTATA"
        },
        {
          "level": 22,
          "species": "RATTATA"
        },
        {
          "level": 23,
          "species": "RATTATA"
        },
        {
          "level": 24,
          "species": "RATTATA"
        },
        {
          "level": 24,
          "species": "RATTATA"
        }
      ],
      "day": [
        {
          "level": 20,
          "species": "RATTATA"
        },
        {
          "level": 21,
          "species": "RATTATA"
        },
        {
          "level": 22,
          "species": "RATTATA"
        },
        {
          "level": 22,
          "species": "RATTATA"
        },
        {
          "level": 23,
          "species": "RATTATA"
        },
        {
          "level": 24,
          "species": "RATTATA"
        },
        {
          "level": 24,
          "species": "RATTATA"
        }
      ],
      "night": [
        {
          "level": 20,
          "species": "GASTLY"
        },
        {
          "level": 21,
          "species": "GASTLY"
        },
        {
          "level": 22,
          "species": "GASTLY"
        },
        {
          "level": 22,
          "species": "RATTATA"
        },
        {
          "level": 23,
          "species": "RATTATA"
        },
        {
          "level": 24,
          "species": "RATTATA"
        },
        {
          "level": 24,
          "species": "RATTATA"
        }
      ]
    },
    "water": {
      "morning": [],
      "day": [],
      "night": []
    }
  },
  {
    "map_name": "TIN_TOWER_7F",
    "grass_rates": {
      "morning": 2,
      "day": 2,
      "night": 2
    },
    "water_rate": null,
    "grass": {
      "morning": [
        {
          "level": 20,
          "species": "RATTATA"
        },
        {
          "level": 21,
          "species": "RATTATA"
        },
        {
          "level": 22,
          "species": "RATTATA"
        },
        {
          "level": 22,
          "species": "RATTATA"
        },
        {
          "level": 23,
          "species": "RATTATA"
        },
        {
          "level": 24,
          "species": "RATTATA"
        },
        {
          "level": 24,
          "species": "RATTATA"
        }
      ],
      "day": [
        {
          "level": 20,
          "species": "RATTATA"
        },
        {
          "level": 21,
          "species": "RATTATA"
        },
        {
          "level": 22,
          "species": "RATTATA"
        },
        {
          "level": 22,
          "species": "RATTATA"
        },
        {
          "level": 23,
          "species": "RATTATA"
        },
        {
          "level": 24,
          "species": "RATTATA"
        },
        {
          "level": 24,
          "species": "RATTATA"
        }
      ],
      "night": [
        {
          "level": 20,
          "species": "GASTLY"
        },
        {
          "level": 21,
          "species": "GASTLY"
        },
        {
          "level": 22,
          "species": "GASTLY"
        },
        {
          "level": 22,
          "species": "RATTATA"
        },
        {
          "level": 23,
          "species": "RATTATA"
        },
        {
          "level": 24,
          "species": "RATTATA"
        },
        {
          "level": 24,
          "species": "RATTATA"
        }
      ]
    },
    "water": {
      "morning": [],
      "day": [],
      "night": []
    }
  },
  {
    "map_name": "TIN_TOWER_8F",
    "grass_rates": {
      "morning": 2,
      "day": 2,
      "night": 2
    },
    "water_rate": null,
    "grass": {
      "morning": [
        {
          "level": 20,
          "species": "RATTATA"
        },
        {
          "level": 21,
          "species": "RATTATA"
        },
        {
          "level": 22,
          "species": "RATTATA"
        },
        {
          "level": 22,
          "species": "RATTATA"
        },
        {
          "level": 23,
          "species": "RATTATA"
        },
        {
          "level": 24,
          "species": "RATTATA"
        },
        {
          "level": 24,
          "species": "RATTATA"
        }
      ],
      "day": [
        {
          "level": 20,
          "species": "RATTATA"
        },
        {
          "level": 21,
          "species": "RATTATA"
        },
        {
          "level": 22,
          "species": "RATTATA"
        },
        {
          "level": 22,
          "species": "RATTATA"
        },
        {
          "level": 23,
          "species": "RATTATA"
        },
        {
          "level": 24,
          "species": "RATTATA"
        },
        {
          "level": 24,
          "species": "RATTATA"
        }
      ],
      "night": [
        {
          "level": 20,
          "species": "GASTLY"
        },
        {
          "level": 21,
          "species": "GASTLY"
        },
        {
          "level": 22,
          "species": "GASTLY"
        },
        {
          "level": 22,
          "species": "RATTATA"
        },
        {
          "level": 23,
          "species": "RATTATA"
        },
        {
          "level": 24,
          "species": "RATTATA"
        },
        {
          "level": 24,
          "species": "RATTATA"
        }
      ]
    },
    "water": {
      "morning": [],
      "day": [],
      "night": []
    }
  },
  {
    "map_name": "TIN_TOWER_9F",
    "grass_rates": {
      "morning": 2,
      "day": 2,
      "night": 2
    },
    "water_rate": null,
    "grass": {
      "morning": [
        {
          "level": 20,
          "species": "RATTATA"
        },
        {
          "level": 21,
          "species": "RATTATA"
        },
        {
          "level": 22,
          "species": "RATTATA"
        },
        {
          "level": 22,
          "species": "RATTATA"
        },
        {
          "level": 23,
          "species": "RATTATA"
        },
        {
          "level": 24,
          "species": "RATTATA"
        },
        {
          "level": 24,
          "species": "RATTATA"
        }
      ],
      "day": [
        {
          "level": 20,
          "species": "RATTATA"
        },
        {
          "level": 21,
          "species": "RATTATA"
        },
        {
          "level": 22,
          "species": "RATTATA"
        },
        {
          "level": 22,
          "species": "RATTATA"
        },
        {
          "level": 23,
          "species": "RATTATA"
        },
        {
          "level": 24,
          "species": "RATTATA"
        },
        {
          "level": 24,
          "species": "RATTATA"
        }
      ],
      "night": [
        {
          "level": 20,
          "species": "GASTLY"
        },
        {
          "level": 21,
          "species": "GASTLY"
        },
        {
          "level": 22,
          "species": "GASTLY"
        },
        {
          "level": 22,
          "species": "RATTATA"
        },
        {
          "level": 23,
          "species": "RATTATA"
        },
        {
          "level": 24,
          "species": "RATTATA"
        },
        {
          "level": 24,
          "species": "RATTATA"
        }
      ]
    },
    "water": {
      "morning": [],
      "day": [],
      "night": []
    }
  },
  {
    "map_name": "BURNED_TOWER_1F",
    "grass_rates": {
      "morning": 4,
      "day": 4,
      "night": 4
    },
    "water_rate": null,
    "grass": {
      "morning": [
        {
          "level": 13,
          "species": "RATTATA"
        },
        {
          "level": 14,
          "species": "KOFFING"
        },
        {
          "level": 15,
          "species": "RATTATA"
        },
        {
          "level": 14,
          "species": "ZUBAT"
        },
        {
          "level": 15,
          "species": "RATTATA"
        },
        {
          "level": 15,
          "species": "RATICATE"
        },
        {
          "level": 15,
          "species": "RATICATE"
        }
      ],
      "day": [
        {
          "level": 13,
          "species": "RATTATA"
        },
        {
          "level": 14,
          "species": "KOFFING"
        },
        {
          "level": 15,
          "species": "RATTATA"
        },
        {
          "level": 14,
          "species": "ZUBAT"
        },
        {
          "level": 15,
          "species": "RATTATA"
        },
        {
          "level": 15,
          "species": "RATICATE"
        },
        {
          "level": 15,
          "species": "RATICATE"
        }
      ],
      "night": [
        {
          "level": 13,
          "species": "RATTATA"
        },
        {
          "level": 14,
          "species": "KOFFING"
        },
        {
          "level": 15,
          "species": "RATTATA"
        },
        {
          "level": 14,
          "species": "ZUBAT"
        },
        {
          "level": 15,
          "species": "RATTATA"
        },
        {
          "level": 15,
          "species": "RATICATE"
        },
        {
          "level": 15,
          "species": "RATICATE"
        }
      ]
    },
    "water": {
      "morning": [],
      "day": [],
      "night": []
    }
  },
  {
    "map_name": "BURNED_TOWER_B1F",
    "grass_rates": {
      "morning": 6,
      "day": 6,
      "night": 6
    },
    "water_rate": null,
    "grass": {
      "morning": [
        {
          "level": 14,
          "species": "RATTATA"
        },
        {
          "level": 14,
          "species": "KOFFING"
        },
        {
          "level": 16,
          "species": "KOFFING"
        },
        {
          "level": 15,
          "species": "ZUBAT"
        },
        {
          "level": 12,
          "species": "KOFFING"
        },
        {
          "level": 16,
          "species": "KOFFING"
        },
        {
          "level": 16,
          "species": "WEEZING"
        }
      ],
      "day": [
        {
          "level": 14,
          "species": "RATTATA"
        },
        {
          "level": 14,
          "species": "KOFFING"
        },
        {
          "level": 16,
          "species": "KOFFING"
        },
        {
          "level": 15,
          "species": "ZUBAT"
        },
        {
          "level": 12,
          "species": "KOFFING"
        },
        {
          "level": 16,
          "species": "KOFFING"
        },
        {
          "level": 16,
          "species": "WEEZING"
        }
      ],
      "night": [
        {
          "level": 14,
          "species": "RATTATA"
        },
        {
          "level": 14,
          "species": "KOFFING"
        },
        {
          "level": 16,
          "species": "KOFFING"
        },
        {
          "level": 15,
          "species": "ZUBAT"
        },
        {
          "level": 12,
          "species": "KOFFING"
        },
        {
          "level": 16,
          "species": "KOFFING"
        },
        {
          "level": 16,
          "species": "WEEZING"
        }
      ]
    },
    "water": {
      "morning": [],
      "day": [],
      "night": []
    }
  },
  {
    "map_name": "NATIONAL_PARK",
    "grass_rates": {
      "morning": 10,
      "day": 10,
      "night": 10
    },
    "water_rate": null,
    "grass": {
      "morning": [
        {
          "level": 12,
          "species": "NIDORAN_M"
        },
        {
          "level": 12,
          "species": "NIDORAN_F"
        },
        {
          "level": 14,
          "species": "LEDYBA"
        },
        {
          "level": 13,
          "species": "PIDGEY"
        },
        {
          "level": 10,
          "species": "CATERPIE"
        },
        {
          "level": 10,
          "species": "WEEDLE"
        },
        {
          "level": 10,
          "species": "WEEDLE"
        }
      ],
      "day": [
        {
          "level": 12,
          "species": "NIDORAN_F"
        },
        {
          "level": 12,
          "species": "NIDORAN_M"
        },
        {
          "level": 14,
          "species": "SUNKERN"
        },
        {
          "level": 13,
          "species": "PIDGEY"
        },
        {
          "level": 10,
          "species": "CATERPIE"
        },
        {
          "level": 10,
          "species": "WEEDLE"
        },
        {
          "level": 10,
          "species": "WEEDLE"
        }
      ],
      "night": [
        {
          "level": 12,
          "species": "PSYDUCK"
        },
        {
          "level": 13,
          "species": "HOOTHOOT"
        },
        {
          "level": 14,
          "species": "SPINARAK"
        },
        {
          "level": 15,
          "species": "HOOTHOOT"
        },
        {
          "level": 10,
          "species": "VENONAT"
        },
        {
          "level": 12,
          "species": "VENONAT"
        },
        {
          "level": 12,
          "species": "VENONAT"
        }
      ]
    },
    "water": {
      "morning": [],
      "day": [],
      "night": []
    }
  },
  {
    "map_name": "RUINS_OF_ALPH_INNER_CHAMBER",
    "grass_rates": {
      "morning": 6,
      "day": 6,
      "night": 6
    },
    "water_rate": null,
    "grass": {
      "morning": [
        {
          "level": 5,
          "species": "UNOWN"
        },
        {
          "level": 5,
          "species": "UNOWN"
        },
        {
          "level": 5,
          "species": "UNOWN"
        },
        {
          "level": 5,
          "species": "UNOWN"
        },
        {
          "level": 5,
          "species": "UNOWN"
        },
        {
          "level": 5,
          "species": "UNOWN"
        },
        {
          "level": 5,
          "species": "UNOWN"
        }
      ],
      "day": [
        {
          "level": 5,
          "species": "UNOWN"
        },
        {
          "level": 5,
          "species": "UNOWN"
        },
        {
          "level": 5,
          "species": "UNOWN"
        },
        {
          "level": 5,
          "species": "UNOWN"
        },
        {
          "level": 5,
          "species": "UNOWN"
        },
        {
          "level": 5,
          "species": "UNOWN"
        },
        {
          "level": 5,
          "species": "UNOWN"
        }
      ],
      "night": [
        {
          "level": 5,
          "species": "UNOWN"
        },
        {
          "level": 5,
          "species": "UNOWN"
        },
        {
          "level": 5,
          "species": "UNOWN"
        },
        {
          "level": 5,
          "species": "UNOWN"
        },
        {
          "level": 5,
          "species": "UNOWN"
        },
        {
          "level": 5,
          "species": "UNOWN"
        },
        {
          "level": 5,
          "species": "UNOWN"
        }
      ]
    },
    "water": {
      "morning": [],
      "day": [],
      "night": []
    }
  },
  {
    "map_name": "MOUNT_MORTAR_1F_INSIDE",
    "grass_rates": {
      "morning": 6,
      "day": 6,
      "night": 6
    },
    "water_rate": null,
    "grass": {
      "morning": [
        {
          "level": 13,
          "species": "GEODUDE"
        },
        {
          "level": 14,
          "species": "RATTATA"
        },
        {
          "level": 15,
          "species": "MACHOP"
        },
        {
          "level": 14,
          "species": "RATICATE"
        },
        {
          "level": 15,
          "species": "ZUBAT"
        },
        {
          "level": 15,
          "species": "GOLBAT"
        },
        {
          "level": 15,
          "species": "GOLBAT"
        }
      ],
      "day": [
        {
          "level": 13,
          "species": "GEODUDE"
        },
        {
          "level": 14,
          "species": "RATTATA"
        },
        {
          "level": 15,
          "species": "MACHOP"
        },
        {
          "level": 14,
          "species": "RATICATE"
        },
        {
          "level": 15,
          "species": "ZUBAT"
        },
        {
          "level": 15,
          "species": "GOLBAT"
        },
        {
          "level": 15,
          "species": "GOLBAT"
        }
      ],
      "night": [
        {
          "level": 13,
          "species": "GEODUDE"
        },
        {
          "level": 14,
          "species": "RATTATA"
        },
        {
          "level": 15,
          "species": "RATICATE"
        },
        {
          "level": 14,
          "species": "ZUBAT"
        },
        {
          "level": 15,
          "species": "MARILL"
        },
        {
          "level": 15,
          "species": "GOLBAT"
        },
        {
          "level": 15,
          "species": "GOLBAT"
        }
      ]
    },
    "water": {
      "morning": [],
      "day": [],
      "night": []
    }
  }
]
