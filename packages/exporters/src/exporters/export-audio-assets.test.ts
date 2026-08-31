import fs from "fs";
import path from "path";
import { getTypeScriptDataDir } from "./asm-utils";
import {
  downsampleStereoPcm,
  exportAudioAssets,
  exportPokemonCryMetadataFromAsm,
  pokemonCryVariantIds,
} from "./export-audio-assets";
import { countMidiNoteOnEvents } from "@pokecrystal/core/audio-export/crystal-midi";

describe("export-audio-assets", () => {
  it("maps odd 44.1 kHz loop boundaries to retained 22.05 kHz frames", () => {
    const downsampled = downsampleStereoPcm(
      new Int16Array([10, 11, 20, 21, 30, 31]),
      1,
      3,
    );

    expect(Array.from(downsampled.pcm)).toEqual([10, 11, 30, 31]);
    expect(downsampled.loopStartSample).toBe(1);
    expect(downsampled.loopEndSample).toBe(2);
  });

  it("exports Pokemon cry metadata with exact species ids from ASM", () => {
    const cries = exportPokemonCryMetadataFromAsm([
      "CHIKORITA",
      "AMPHAROS",
      "MILTANK",
    ]);

    expect(cries).toEqual({
      CHIKORITA: { cry: "CRY_CHIKORITA", pitch: -16, length: 176 },
      AMPHAROS: { cry: "CRY_AMPHAROS", pitch: -124, length: 232 },
      MILTANK: { cry: "CRY_GLIGAR", pitch: -461, length: 416 },
    });
    expect(cries).not.toHaveProperty("252");
  });

  it("rejects missing species cry metadata instead of leaving runtime fallback work", () => {
    expect(() => exportPokemonCryMetadataFromAsm(["NOT_A_SPECIES"])).toThrow(
      "Pokemon cry metadata is missing exact species 'NOT_A_SPECIES'",
    );
  });

  it("generates only exact ASM-rendered PCM music, SFX, and cries", () => {
    const pokemon = JSON.parse(
      fs.readFileSync(path.join(getTypeScriptDataDir(), "pokemon_data.json"), "utf8"),
    ) as Array<{ id: string }>;
    const assets = exportAudioAssets(
      exportPokemonCryMetadataFromAsm(pokemon.map(({ id }) => id)),
    );

    expect(assets).toEqual(
      expect.objectContaining({
        MUSIC_ROUTE_29: expect.objectContaining({
          id: "MUSIC_ROUTE_29",
          path: "content-packs/core-modular/music/MUSIC_ROUTE_29.pcm",
          kind: "music",
          source: "pcm",
        }),
        SFX_TACKLE: expect.objectContaining({
          id: "SFX_TACKLE",
          path: "content-packs/core-modular/sfx/SFX_TACKLE.pcm",
          kind: "sound_effect",
          source: "pcm",
          sfx_priority: 0x41,
        }),
        CRY_NIDORAN_M: expect.objectContaining({
          id: "CRY_NIDORAN_M",
          path: "content-packs/core-modular/cries/CRY_NIDORAN_M.pcm",
          kind: "cry",
          source: "pcm",
        }),
        CRY_MON_FARFETCH_D: expect.objectContaining({
          id: "CRY_MON_FARFETCH_D",
          path: "content-packs/core-modular/cries/CRY_MON_FARFETCH_D.pcm",
          kind: "cry",
          source: "pcm",
        }),
        CRY_GLIGAR: expect.objectContaining({
          id: "CRY_GLIGAR",
          path: "content-packs/core-modular/cries/CRY_GLIGAR.pcm",
          kind: "cry",
          source: "pcm",
        }),
      }),
    );
    expect(Object.keys(assets).length).toBeGreaterThan(100);
    expect(assets).not.toHaveProperty("MUSIC_NONE");
    expect(
      Object.values(assets).every((asset) => asset.path.endsWith(".pcm")),
    ).toBe(true);
    expect(Object.values(assets).every((asset) => asset.source === "pcm")).toBe(
      true,
    );
    expect(
      Object.values(assets).every(
        (asset) =>
          asset.midi_program?.profile === "pokecrystal-midi-v1" &&
          Buffer.from(asset.midi_program.midi_base64, "base64")
            .subarray(0, 4)
            .toString("ascii") === "MThd",
      ),
    ).toBe(true);
    expect(assets.SFX_DEX_FANFARE_50_79.sfx_priority).toBe(0x00);
    expect(assets.SFX_ITEM.sfx_priority).toBe(0x01);
    expect(
      Object.values(assets).every((asset) =>
        asset.kind === "sound_effect"
          ? Number.isInteger(asset.sfx_priority)
          : asset.sfx_priority === undefined,
      ),
    ).toBe(true);
    const sfxPriorities = Object.values(assets)
      .filter((asset) => asset.kind === "sound_effect")
      .map((asset) => asset.sfx_priority);
    expect(sfxPriorities).toEqual(sfxPriorities.map((_, index) => index));
    expect(
      Object.values(assets).every(
        (asset) =>
          asset.pcm_format.sample_rate_hz === 22_050 &&
          asset.pcm_format.channels === 2,
      ),
    ).toBe(true);
    const route29Metadata = JSON.parse(
      fs.readFileSync(
        path.join(
          getTypeScriptDataDir(),
          "content-packs/core-modular/music/MUSIC_ROUTE_29.json",
        ),
        "utf8",
      ),
    ) as Record<string, typeof assets.MUSIC_ROUTE_29>;
    expect(route29Metadata.MUSIC_ROUTE_29).toEqual(assets.MUSIC_ROUTE_29);
    expect(
      countMidiNoteOnEvents(
        Buffer.from(assets.MUSIC_ROUTE_29.midi_program!.midi_base64, "base64"),
      ),
    ).toBeGreaterThan(0);
    expect(
      fs.existsSync(
        path.join(
          getTypeScriptDataDir(),
          "content-packs/core-modular/audio-synth-context.json",
        ),
      ),
    ).toBe(true);
    expect(
      Object.values(assets).filter(
        (asset) => asset.kind === "cry" && asset.id === "CRY_NIDORAN_M",
      ),
    ).toHaveLength(1);
    expect(
      fs.existsSync(
        path.join(
          getTypeScriptDataDir(),
          "content-packs/core-modular/music/MUSIC_NONE.pcm",
        ),
      ),
    ).toBe(false);
    expect(
      ["music", "sfx", "cries"]
        .flatMap((directory) =>
          fs.readdirSync(
            path.join(
              getTypeScriptDataDir(),
              "content-packs/core-modular",
              directory,
            ),
          ),
        )
        .filter((fileName) => /\.midi?$/i.test(fileName)),
    ).toHaveLength(Object.keys(assets).length);
  });

  it("rejects case-changed cry labels instead of normalizing them", () => {
    expect(() =>
      exportAudioAssets({ LUGIA: { cry: "cry_lugia", pitch: 0, length: 0 } }),
    ).toThrow("Pokemon cry metadata must use exact CRY_* labels");
  });

  it("uses exact normal, Growl, and Roar runtime ids for each species", () => {
    expect(pokemonCryVariantIds("SANDSHREW")).toEqual([
      "CRY_MON_SANDSHREW",
      "CRY_MON_SANDSHREW_GROWL",
      "CRY_MON_SANDSHREW_ROAR",
    ]);
    expect(() => pokemonCryVariantIds("sandshrew")).toThrow(
      "exact constant token",
    );
  });
});
