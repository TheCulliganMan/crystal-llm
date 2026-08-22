import { exportDecorations } from "./export-decorations";

describe("exportDecorations", () => {
  test("preserves the exact ASM category order, ids, ownership flags, and names", () => {
    const catalog = exportDecorations();

    expect(catalog.category_order).toEqual([
      "BED",
      "CARPET",
      "PLANT",
      "POSTER",
      "GAME_CONSOLE",
      "ORNAMENT",
      "BIG_DOLL",
    ]);
    expect(catalog.decorations).toHaveLength(45);
    expect(catalog.decorations[0]).toEqual({
      index: 2,
      id: "DECO_FEATHERY_BED",
      category: "BED",
      display_name: "FEATHERY BED",
      action: "SET_UP_BED",
      event_flag: "EVENT_DECO_BED_1",
      sprite: "$1b",
    });
    expect(catalog.decorations.find((entry) => entry.id === "DECO_TOWN_MAP")).toMatchObject({
      index: 16,
      category: "POSTER",
      display_name: "TOWN MAP",
      event_flag: "EVENT_DECO_POSTER_1",
    });
    expect(
      catalog.decorations.find((entry) => entry.id === "DECO_SURF_PIKACHU_DOLL"),
    ).toMatchObject({
      category: "ORNAMENT",
      display_name: "SURF PIKACHU DOLL",
      event_flag: "EVENT_DECO_SURFING_PIKACHU_DOLL",
    });
    expect(catalog.decorations.find((entry) => entry.id === "DECO_BIG_LAPRAS_DOLL")).toMatchObject({
      index: 28,
      category: "BIG_DOLL",
      display_name: "BIG LAPRAS",
      event_flag: "EVENT_DECO_BIG_LAPRAS_DOLL",
    });
    expect(catalog.decorations.at(-1)).toMatchObject({
      id: "DECO_SILVER_TROPHY_DOLL",
      category: "ORNAMENT",
      display_name: "SILVER TROPHY",
      event_flag: "EVENT_DECO_SILVER_TROPHY",
    });
  });
});
