import fs from "fs";
import path from "path";
import { getTypeScriptDataDir } from "./asm-utils";
import { exportAudioAssets, exportPokemonCryMetadataFromAsm } from "./export-audio-assets";

describe("export-audio-assets", () => {
  it("exports Pokemon cry metadata with exact species ids from ASM", () => {
    const cries = exportPokemonCryMetadataFromAsm(["CHIKORITA", "AMPHAROS", "MILTANK"]);

    expect(cries).toEqual({
      CHIKORITA: { cry: "CRY_CHIKORITA", pitch: -16, length: 176 },
      AMPHAROS: { cry: "CRY_AMPHAROS", pitch: -124, length: 232 },
      MILTANK: { cry: "CRY_GLIGAR", pitch: -461, length: 416 },
    });
    expect(cries).not.toHaveProperty("252");
  });

  it("rejects missing species cry metadata instead of leaving runtime fallback work", () => {
    expect(() => exportPokemonCryMetadataFromAsm(["NOT_A_SPECIES"])).toThrow(
      "Pokemon cry metadata is missing exact species 'NOT_A_SPECIES'"
    );
  });

  it("generates music, sfx, and cry declarations that point to .mid modpack files", () => {
    const assets = exportAudioAssets({
      NIDORAN_M: { cry: "CRY_NIDORAN_M", pitch: 0, length: 0 },
      SANDSHREW: { cry: "CRY_NIDORAN_M", pitch: 32, length: 192 },
      FARFETCH_D: { cry: "CRY_FARFETCH_D", pitch: 0, length: 0 },
    });

    expect(assets).toEqual(
      expect.objectContaining({
        MUSIC_NONE: {
          id: "MUSIC_NONE",
          path: "content-packs/core-modular/music/MUSIC_NONE.mid",
          kind: "music",
          source: "midi",
        },
        MUSIC_ROUTE_29: {
          id: "MUSIC_ROUTE_29",
          path: "content-packs/core-modular/music/MUSIC_ROUTE_29.mid",
          kind: "music",
          source: "midi",
        },
        SFX_TACKLE: {
          id: "SFX_TACKLE",
          path: "content-packs/core-modular/sfx/SFX_TACKLE.mid",
          kind: "sound_effect",
          source: "midi",
        },
        CRY_NIDORAN_M: {
          id: "CRY_NIDORAN_M",
          path: "content-packs/core-modular/cries/CRY_NIDORAN_M.mid",
          kind: "cry",
          source: "midi",
        },
        CRY_FARFETCH_D: {
          id: "CRY_FARFETCH_D",
          path: "content-packs/core-modular/cries/CRY_FARFETCH_D.mid",
          kind: "cry",
          source: "midi",
        },
        CRY_GLIGAR: {
          id: "CRY_GLIGAR",
          path: "content-packs/core-modular/cries/CRY_GLIGAR.mid",
          kind: "cry",
          source: "midi",
        },
      })
    );
    expect(Object.keys(assets).length).toBeGreaterThan(100);
    expect(Object.values(assets).every((asset) => asset.path.endsWith(".mid"))).toBe(true);
    expect(Object.values(assets).every((asset) => !("sample_rate_hz" in asset))).toBe(true);
    expect(Object.values(assets).filter((asset) => asset.kind === "cry" && asset.id === "CRY_NIDORAN_M")).toHaveLength(1);
    const titleMidi = fs.readFileSync(path.join(getTypeScriptDataDir(), assets.MUSIC_NONE.path));
    expect(titleMidi.subarray(0, 4).toString("ascii")).toBe("MThd");
    expect(titleMidi.length).toBeGreaterThan(26);
    expect(titleMidi.includes(Buffer.from([0x90]))).toBe(true);
  });

  it("rejects case-changed cry labels instead of normalizing them", () => {
    expect(() => exportAudioAssets({ LUGIA: { cry: "cry_lugia", pitch: 0, length: 0 } })).toThrow(
      "Pokemon cry metadata must use exact CRY_* labels"
    );
  });
});
