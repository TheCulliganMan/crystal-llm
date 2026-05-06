import {
  DefaultPokedexPrinter,
  __test__renderPokedexPrintOutput,
} from "@pokecrystal/core/ui/menus/pokedex";
import { PrintOption } from "@pokecrystal/core/core/enums/ui-enums";
import type { PokedexEntryData } from "@pokecrystal/core/ui/menus/pokedex-entry-loader";

describe("pokedex print output", () => {
  it("formats pokédex entries for output", () => {
    const entryData: PokedexEntryData = {
      classification: "Seed Pokémon",
      heightDigits: 7,
      weightDigits: 15,
      pages: ["Some lines@Second line", "More info"],
    };

    expect(
      __test__renderPokedexPrintOutput("BULBASAUR", 1, entryData, PrintOption.DARKEST)
    ).toBe(
      [
        "BULBASAUR  #001",
        "Seed Pokémon | HT:7 WT:15 | MODE:DARKEST",
        "",
        "Some lines",
        "Second line",
        "",
        "More info",
        "",
      ].join("\n")
    );
  });

  it("stores printed output in localStorage when available", () => {
    const entryData: PokedexEntryData = {
      classification: "Seed Pokémon",
      heightDigits: 7,
      weightDigits: 15,
      pages: ["Some lines@Second line", "More info"],
    };
    const setItem = jest.fn();
    const descriptor = Object.getOwnPropertyDescriptor(globalThis, "localStorage");
    Object.defineProperty(globalThis, "localStorage", {
      configurable: true,
      writable: true,
      value: {
        setItem,
      } as Storage,
    });

    const restoreStorage = () => {
      if (descriptor) {
        Object.defineProperty(globalThis, "localStorage", descriptor);
      } else {
        delete (globalThis as { localStorage?: Storage }).localStorage;
      }
    };
    try {
      const printer = new DefaultPokedexPrinter();
      const expected = __test__renderPokedexPrintOutput(
        "bulbasaur",
        1,
        entryData,
        PrintOption.NORMAL
      );
      printer.printDexEntry("bulbasaur", 1, entryData, PrintOption.NORMAL);

      expect(setItem).toHaveBeenCalledWith(
        "pokecrystal-ts:pokedex-print:001_bulbasaur.txt",
        expected
      );
    } finally {
      restoreStorage();
    }
  });
});
