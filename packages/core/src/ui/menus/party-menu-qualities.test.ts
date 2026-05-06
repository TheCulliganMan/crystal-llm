import { PartyMenuQualityRenderer } from './party-menu-qualities';
import { PartyMenuTilemap } from './party-menu-layout';
import { PartyMenuQuality } from '@pokecrystal/core/core/enums/party-menu';

describe('PartyMenuQualityRenderer', () => {
  test('ignores unknown qualities without throwing', () => {
    const renderer = new PartyMenuQualityRenderer();
    const tilemap = new PartyMenuTilemap();
    const menu = {
      _is_egg: () => false,
      _name_row_y: () => 0,
      _status_row_y: () => 0,
      _cancel_row_y: () => 0,
      gameState: {
        wram: {
          wHPPals: [0, 0, 0, 0, 0, 0],
          wCurHPPal: 0,
          wSGBPals: 0,
        },
      },
    };

    expect(() => {
      renderer.apply(
        menu as any,
        tilemap,
        [],
        [PartyMenuQuality.HP_BAR, 'unknown' as PartyMenuQuality],
      );
    }).not.toThrow();
  });
});
