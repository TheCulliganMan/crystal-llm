import {
  decorationIdMap,
  decorations,
  type DecorationAttribute,
} from "@pokecrystal/assets/content/decorations";

export const DECORATION_CATEGORY_ORDER = [
  "BED",
  "CARPET",
  "PLANT",
  "POSTER",
  "GAME_CONSOLE",
  "ORNAMENT",
  "BIG_DOLL",
] as const;

export type DecorationCategory = (typeof DECORATION_CATEGORY_ORDER)[number];

export type ExportedDecoration = {
  index: number;
  id: string;
  category: DecorationCategory;
  display_name: string;
  action: string;
  event_flag: string;
  sprite: string;
};

export type ExportedDecorationCatalog = {
  category_order: DecorationCategory[];
  decorations: ExportedDecoration[];
};

const CATEGORY_BY_ACTION: Record<string, DecorationCategory> = {
  SET_UP_BED: "BED",
  SET_UP_CARPET: "CARPET",
  SET_UP_PLANT: "PLANT",
  SET_UP_POSTER: "POSTER",
  SET_UP_CONSOLE: "GAME_CONSOLE",
  SET_UP_DOLL: "ORNAMENT",
  SET_UP_BIG_DOLL: "BIG_DOLL",
};

const DIRECT_NAMES: Record<string, string> = {
  MAGNAPLANT: "MAGNAPLANT",
  TROPICPLANT: "TROPICPLANT",
  JUMBOPLANT: "JUMBOPLANT",
  TOWN_MAP_POSTER: "TOWN MAP",
  FAMICOM: "NES",
  SUPER_NES: "SUPER NES",
  NINTENDO_64: "NINTENDO 64",
  VIRTUAL_BOY: "VIRTUAL BOY",
  GOLD_TROPHY: "GOLD TROPHY",
  SILVER_TROPHY: "SILVER TROPHY",
  SURF_PIKA_DOLL: "SURF PIKACHU DOLL",
};

function decorationDisplayName(entry: DecorationAttribute): string {
  const direct = DIRECT_NAMES[entry.name_token];
  if (direct) return direct;

  switch (entry.deco_type) {
    case "DECO_BED":
      return `${entry.name_token.replace(/_BED$/, "").replaceAll("_", " ")} BED`;
    case "DECO_CARPET":
      return `${entry.name_token.replace(/_CARPET$/, "").replaceAll("_", " ")} CARPET`;
    case "DECO_POSTER":
      return `${entry.name_token.replaceAll("_", " ")} POSTER`;
    case "DECO_DOLL":
      return `${entry.name_token.replaceAll("_", " ")} DOLL`;
    case "DECO_BIGDOLL":
      return `BIG ${entry.name_token.replaceAll("_", " ")}`;
    default:
      throw new Error(
        `Decoration ${entry.index} has unsupported name token ${entry.name_token} for type ${entry.deco_type}`,
      );
  }
}

export function exportDecorations(): ExportedDecorationCatalog {
  const rows = new Map(decorations.map((entry) => [entry.index, entry]));
  const exported = Object.entries(decorationIdMap)
    .map(([rawIndex, id]): ExportedDecoration => {
      const index = Number(rawIndex);
      const entry = rows.get(index);
      if (!entry) {
        throw new Error(`Decoration id ${id} references missing attribute row ${index}`);
      }
      const category = CATEGORY_BY_ACTION[entry.action_token];
      if (!category) {
        throw new Error(
          `Decoration ${id} has unsupported setup action ${entry.action_token}`,
        );
      }
      if (!entry.event_flag.startsWith("EVENT_DECO_")) {
        throw new Error(`Decoration ${id} has invalid ownership flag ${entry.event_flag}`);
      }
      return {
        index,
        id,
        category,
        display_name: decorationDisplayName(entry),
        action: entry.action_token,
        event_flag: entry.event_flag,
        sprite: entry.sprite_token,
      };
    })
    .sort((left, right) => left.index - right.index);

  if (exported.length !== 45) {
    throw new Error(`Expected 45 source decorations, found ${exported.length}`);
  }
  for (const category of DECORATION_CATEGORY_ORDER) {
    if (!exported.some((entry) => entry.category === category)) {
      throw new Error(`Decoration category ${category} has no source entries`);
    }
  }

  return {
    category_order: [...DECORATION_CATEGORY_ORDER],
    decorations: exported,
  };
}
