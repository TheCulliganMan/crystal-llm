import { draw_battle_party_menu } from "./_battle-party-menu";
import { PARTY_MENU_QUALITY_POINTERS, PartyMenuAction } from "@pokecrystal/core/core/enums/party-menu";

const mockApply = jest.fn();

jest.mock("../menus/party-menu-qualities", () => ({
  PartyMenuQualityRenderer: jest.fn(() => ({ apply: mockApply })),
}));

describe("draw_battle_party_menu", () => {
  type StateArg = Parameters<typeof draw_battle_party_menu>[0];

  beforeEach(() => {
    mockApply.mockClear();
  });

  it("lays out party rows, cursor, and cancel option", () => {
    const tilemap = {
      set_tile: jest.fn(),
      write_text: jest.fn(),
    };
    const state = {
      tilemap,
      wram: { wPartyMenuCursorPosition: 1 },
    } as unknown as StateArg;
    const battleContext = {
      playerParty: [
        { species: { id: "PIKACHU" } },
        { species: { id: "EGG" }, nickname: "EGG" },
      ],
    };
    const cursorTile = 9;
    draw_battle_party_menu(state, battleContext, 2, 4, cursorTile);

    expect(mockApply).toHaveBeenCalledWith(
      expect.objectContaining({
        _name_row_y: expect.any(Function),
        _status_row_y: expect.any(Function),
        _cancel_row_y: expect.any(Function),
        _is_egg: expect.any(Function),
        gameState: expect.objectContaining({
          wram: expect.objectContaining({
            wHPPals: [],
            wCurHPPal: 0,
            wSGBPals: 0,
          }),
        }),
      }),
      tilemap,
      [
        { index: 0, pokemon: battleContext.playerParty[0] },
        { index: 1, pokemon: battleContext.playerParty[1] },
      ],
      PARTY_MENU_QUALITY_POINTERS[PartyMenuAction.CHOOSE_POKEMON]
    );

    expect(tilemap.set_tile).toHaveBeenCalledWith(2, 6, cursorTile, expect.any(Number));
    expect(tilemap.write_text).toHaveBeenCalledWith(3, 8, "CANCEL");
  });

  it("does nothing when party is empty", () => {
    const tilemap = {
      set_tile: jest.fn(),
      write_text: jest.fn(),
    };
    const state = { tilemap, wram: { wPartyMenuCursorPosition: 0 } } as unknown as StateArg;
    const battleContext = { playerParty: [] };

    draw_battle_party_menu(state, battleContext, 1, 2, 9);

    expect(mockApply).not.toHaveBeenCalled();
    expect(tilemap.set_tile).not.toHaveBeenCalled();
    expect(tilemap.write_text).not.toHaveBeenCalled();
  });
});
