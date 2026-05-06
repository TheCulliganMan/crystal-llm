import { BattleUIState } from "./battle-ui-state";
import {
  PAL_MENU,
} from "./_battle-background";
import { PARTY_MENU_QUALITY_POINTERS, PartyMenuAction } from "@pokecrystal/core/core/enums/party-menu";
import { PartyMenuQualityRenderer } from "../menus/party-menu-qualities";
import type { PartyEntry, PartyMenuTilemap } from "../menus/party-menu-layout";
import { Pokemon } from "@pokecrystal/core/core/models";

type BattlePartyContext = {
  playerParty?: Pokemon[];
};

export const draw_battle_party_menu = (
  _state: BattleUIState,
  battle_context: BattlePartyContext,
  _base_x: number,
  _base_y: number,
  cursor_tile: number
): void => {
  const party = battle_context?.playerParty ?? [];
  if (!party.length) {
    return;
  }
  const entries: PartyEntry[] = party.map((pokemon, index) => ({ index, pokemon }));
  const renderer = new PartyMenuQualityRenderer();

  const menu = {
    _name_row_y: (rowIndex: number): number => _base_y + rowIndex * 2,
    _status_row_y: (rowIndex: number): number => _base_y + rowIndex * 2 + 1,
    _cancel_row_y: (entryCount: number): number => _base_y + entryCount * 2,
    _is_egg: (pokemon: Pokemon): boolean => {
      const nickname = String(pokemon.nickname ?? "").toUpperCase();
      const species = String(pokemon.species?.id ?? "").toUpperCase();
      return nickname === "EGG" || species === "EGG";
    },
    gameState: {
      wram: {
        wHPPals: [],
        wCurHPPal: 0,
        wSGBPals: 0,
      },
    },
  };

  renderer.apply(
    menu,
    _state.tilemap as PartyMenuTilemap,
    entries,
    PARTY_MENU_QUALITY_POINTERS[PartyMenuAction.CHOOSE_POKEMON],
  );

  const cursorY = _base_y + _state.wram.wPartyMenuCursorPosition * 2;
  _state.tilemap.set_tile(_base_x, cursorY, cursor_tile, PAL_MENU);

  const cancelRow = _base_y + party.length * 2;
  _state.tilemap.write_text(_base_x + 1, cancelRow, "CANCEL");
};
