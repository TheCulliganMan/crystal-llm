import { Surface } from "@pokecrystal/core/ui/surface";
import { PartyMenuIconRenderer } from "./party-menu-icons";

const regionHasOpaquePixel = (
  surface: Surface,
  startX: number,
  startY: number,
  width: number,
  height: number
): boolean => {
  for (let y = startY; y < startY + height; y += 1) {
    for (let x = startX; x < startX + width; x += 1) {
      if (surface.get_at([x, y])[3] > 0) {
        return true;
      }
    }
  }
  return false;
};

const regionHasNonGrayOpaquePixel = (
  surface: Surface,
  startX: number,
  startY: number,
  width: number,
  height: number
): boolean => {
  for (let y = startY; y < startY + height; y += 1) {
    for (let x = startX; x < startX + width; x += 1) {
      const [r, g, b, a] = surface.get_at([x, y]);
      if (a > 0 && (r !== g || g !== b)) {
        return true;
      }
    }
  }
  return false;
};

describe("party-menu-icons", () => {
  it("renders species icons using the bundled menu icon map", () => {
    const renderer = new PartyMenuIconRenderer();
    const surface = new Surface(160, 144);

    renderer.draw(
      surface,
      [
        {
          index: 0,
          pokemon: {
            species: { id: "BULBASAUR" },
            nickname: "BULBASAUR",
            hp: 20,
            max_hp: 20,
            item: null,
          },
        },
        {
          index: 1,
          pokemon: {
            species: { id: "TOGEPI" },
            nickname: "EGG",
            hp: 10,
            max_hp: 10,
            item: null,
          },
        },
      ] as never,
      {
        frozen: false,
        highlightSlot: null,
        switchOriginSlot: null,
        switchMode: false,
      }
    );

    expect(regionHasOpaquePixel(surface, 8, 4, 16, 16)).toBe(true);
    expect(regionHasOpaquePixel(surface, 8, 20, 16, 16)).toBe(true);
  });

  it("applies the party menu OBJ palette to grayscale PNG icon fallbacks", () => {
    const renderer = new PartyMenuIconRenderer();
    const surface = new Surface(160, 144);

    renderer.draw(
      surface,
      [
        {
          index: 0,
          pokemon: {
            species: { id: "TOTODILE" },
            nickname: "TOTODILE",
            hp: 20,
            max_hp: 20,
            item: null,
          },
        },
      ] as never,
      {
        frozen: false,
        highlightSlot: null,
        switchOriginSlot: null,
        switchMode: false,
      }
    );

    expect(regionHasNonGrayOpaquePixel(surface, 8, 4, 16, 16)).toBe(true);
  });

  it("keeps transparent icon pixels from covering the party menu background", () => {
    const renderer = new PartyMenuIconRenderer();
    const surface = new Surface(160, 144);
    surface.fill([255, 0, 255, 255]);

    renderer.draw(
      surface,
      [
        {
          index: 0,
          pokemon: {
            species: { id: "TOTODILE" },
            nickname: "TOTODILE",
            hp: 20,
            max_hp: 20,
            item: null,
          },
        },
      ] as never,
      {
        frozen: false,
        highlightSlot: null,
        switchOriginSlot: null,
        switchMode: false,
      }
    );

    expect(surface.get_at([8, 4])).toEqual([255, 0, 255, 255]);
  });

  it("normalizes double-underscore species constants to exported icon keys", () => {
    const renderer = new PartyMenuIconRenderer();
    const surface = new Surface(160, 144);

    renderer.draw(
      surface,
      [
        {
          index: 0,
          pokemon: {
            species: { id: "MR__MIME" },
            nickname: "MR__MIME",
            hp: 20,
            max_hp: 20,
            item: null,
          },
        },
      ] as never,
      {
        frozen: false,
        highlightSlot: null,
        switchOriginSlot: null,
        switchMode: false,
      }
    );

    expect(regionHasOpaquePixel(surface, 8, 4, 16, 16)).toBe(true);
  });

  it("keeps the highlighted party icon out of the nickname column", () => {
    const renderer = new PartyMenuIconRenderer();
    const surface = new Surface(160, 144);

    renderer.draw(
      surface,
      [
        {
          index: 0,
          pokemon: {
            species: { id: "TOTODILE" },
            nickname: "TOTODILE",
            hp: 20,
            max_hp: 20,
            item: null,
          },
        },
      ] as never,
      {
        frozen: false,
        highlightSlot: 0,
        switchOriginSlot: null,
        switchMode: false,
      }
    );

    expect(regionHasOpaquePixel(surface, 8, 4, 16, 16)).toBe(true);
    expect(regionHasOpaquePixel(surface, 24, 4, 8, 16)).toBe(false);
    expect(regionHasOpaquePixel(surface, 8, 20, 16, 1)).toBe(false);
  });
});
