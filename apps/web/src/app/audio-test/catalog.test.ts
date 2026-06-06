import { buildAudioTestCatalog } from "./catalog";

describe("audio test catalog", () => {
  it("builds direct PCM entries for music, SFX, and cries", () => {
    const catalog = buildAudioTestCatalog();

    expect(catalog.stats.music).toBeGreaterThan(100);
    expect(catalog.stats.sfx).toBeGreaterThan(100);
    expect(catalog.stats.cry).toBeGreaterThan(50);
    expect(catalog.total).toBe(catalog.entries.length);

    expect(catalog.entries).toContainEqual(
      expect.objectContaining({
        group: "music",
        token: "MUSIC_TITLE",
        source: "/api/audio/pcm/music/titlescreen.json",
      }),
    );
    expect(catalog.entries).toContainEqual(
      expect.objectContaining({
        group: "sfx",
        token: "SFX_ITEM",
        source: "/api/audio/pcm/sfx/item.json",
      }),
    );
    expect(catalog.entries).toContainEqual(
      expect.objectContaining({
        group: "cry",
        token: "CRY_NIDORAN_M",
        source: "/api/audio/pcm/cries/nidoran_m.json",
      }),
    );
  });

  it("keeps entries unique and skips the silence music token", () => {
    const catalog = buildAudioTestCatalog();
    const ids = new Set(catalog.entries.map((entry) => entry.id));

    expect(ids.size).toBe(catalog.entries.length);
    expect(catalog.entries.some((entry) => entry.token === "MUSIC_NONE")).toBe(false);
  });
});
