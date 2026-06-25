import { exportCurrencyConstants } from "./export-currency-constants";

describe("export-currency-constants", () => {
  it("exports exact global and map script constants as one currency catalog", () => {
    expect(
      exportCurrencyConstants({
        global: {
          MAX_MONEY: 999999,
          MAX_COINS: 9999,
        },
        maps: {
          Route43Gate: {
            ROUTE43GATE_TOLL: 1000,
          },
        },
      })
    ).toEqual({
      MAX_COINS: 9999,
      MAX_MONEY: 999999,
      ROUTE43GATE_TOLL: 1000,
    });
  });

  it("rejects duplicate constants instead of choosing a fallback source", () => {
    expect(() =>
      exportCurrencyConstants({
        global: { MAX_COINS: 9999 },
        maps: { GameCorner: { MAX_COINS: 9999 } },
      })
    ).toThrow("must be exported once");
  });

  it("omits negative sentinel constants that cannot be currency constants", () => {
    expect(
      exportCurrencyConstants({
        global: { STANDING: -1, MAX_MONEY: 999999 },
      })
    ).toEqual({ MAX_MONEY: 999999 });
  });

  it("rejects non-u32 currency constant values", () => {
    expect(() =>
      exportCurrencyConstants({
        global: { TOO_LARGE: 0x1_0000_0000 },
      })
    ).toThrow("exceeds u32 range");
  });
});
