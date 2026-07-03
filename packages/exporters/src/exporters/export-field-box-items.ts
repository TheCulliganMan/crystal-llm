import { decorations } from "@pokecrystal/assets/content/decorations";
import { writeJsonToTargets } from "./asm-utils";

export type ExportedFieldBoxItemRule = {
  item_id: string;
  effect: string;
  decoration_flag: string;
};

const BOX_DECORATION_NAMES: Record<string, string> = {
  NORMAL_BOX: "SILVER_TROPHY",
  GORGEOUS_BOX: "GOLD_TROPHY",
};

const BOX_EFFECTS: Record<string, string> = {
  NORMAL_BOX: "NORMAL_BOX",
  GORGEOUS_BOX: "GORGEOUS_BOX",
};

export function exportFieldBoxItems(): Record<string, ExportedFieldBoxItemRule> {
  const rules: Record<string, ExportedFieldBoxItemRule> = Object.fromEntries(
    Object.entries(BOX_DECORATION_NAMES).map(([itemId, decorationName]) => {
      const decoration = decorations.find((entry) => entry.name_token === decorationName);
      if (!decoration) {
        throw new Error(`Missing decoration row ${decorationName} for field box item ${itemId}`);
      }
      return [
        itemId,
        {
          item_id: itemId,
          effect: BOX_EFFECTS[itemId],
          decoration_flag: decoration.event_flag,
        },
      ];
    }),
  );

  writeJsonToTargets("field_box_items.json", rules, { indent: 2 });
  return rules;
}
