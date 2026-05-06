describe('BattleAnimData required source files', () => {
  afterEach(() => {
    jest.resetModules();
    jest.restoreAllMocks();
    jest.dontMock('@pokecrystal/core/core/asset-reader');
    jest.dontMock('@pokecrystal/core/core/paths');
  });

  it('throws when bundled battle animation runtime JSON is missing', () => {
    const dataDir = '/tmp/assets/data';
    const missingPath = `${dataDir}/battle_anim_bundle.json`;

    jest.doMock('@pokecrystal/core/core/asset-reader', () => ({
      readJsonAssetSync: (target: string) => {
        if (target === missingPath) {
          throw new Error(`Failed to load asset ${target} (status 404)`);
        }
        return {};
      },
      readTextAssetSync: () => '',
    }));
    jest.doMock('@pokecrystal/core/core/paths', () => ({
      getAssetPath: (...parts: string[]) => `/tmp/assets/${parts.join('/')}`,
      getAssetsRoot: () => '/tmp/assets',
      getDataDir: () => dataDir,
      getDisassemblyRoot: () => '/tmp/unused-disassembly',
    }));

    jest.isolateModules(() => {
      const { BattleAnimData } = require('./battle-anim-data') as typeof import('./battle-anim-data');

      expect(() => new BattleAnimData()).toThrow(
        `Battle animation runtime bundle is required for bundled runtime: missing or invalid ${missingPath}`
      );
    });
  });

  it('throws when bundled battle animation runtime JSON is invalid', () => {
    const dataDir = '/tmp/assets/data';
    const bundlePath = `${dataDir}/battle_anim_bundle.json`;

    jest.doMock('@pokecrystal/core/core/asset-reader', () => ({
      readJsonAssetSync: (target: string) => {
        if (target !== bundlePath) {
          throw new Error(`unexpected read: ${target}`);
        }
        return {
          objects: {},
          framesets: {},
          oam_sets: {},
          gfx_table: {},
        };
      },
      readTextAssetSync: () => '',
    }));
    jest.doMock('@pokecrystal/core/core/paths', () => ({
      getAssetPath: (...parts: string[]) => `/tmp/assets/${parts.join('/')}`,
      getAssetsRoot: () => '/tmp/assets',
      getDataDir: () => dataDir,
      getDisassemblyRoot: () => '/tmp/unused-disassembly',
    }));

    jest.isolateModules(() => {
      const { BattleAnimData } = require('./battle-anim-data') as typeof import('./battle-anim-data');

      expect(() => new BattleAnimData()).toThrow(
        `Battle animation runtime bundle is required for bundled runtime: missing or invalid ${bundlePath}`
      );
    });
  });

  it('loads battle animation runtime data from bundled JSON instead of raw asm assets', () => {
    const dataDir = '/tmp/assets/data';
    const bundlePath = `${dataDir}/battle_anim_bundle.json`;

    jest.doMock('@pokecrystal/core/core/asset-reader', () => ({
      readJsonAssetSync: (target: string) => {
        if (target !== bundlePath) {
          throw new Error(`unexpected read: ${target}`);
        }
        return {
          objects: {
            BATTLE_ANIM_OBJ_TEST: {
              object_id: 'BATTLE_ANIM_OBJ_TEST',
              flags: 0,
              fix_y: 0,
              function: 'BATTLE_ANIM_FUNC_NULL',
              frameset: 'BATTLE_ANIM_FRAMESET_TEST',
              palette: 'PAL_BATTLE_OB_GRAY',
              gfx_id: 'BATTLE_ANIM_GFX_TEST',
            },
          },
          framesets: {
            BATTLE_ANIM_FRAMESET_TEST: [
              {
                command: 'frame',
                oam_set: 'BATTLE_ANIM_OAMSET_TEST',
                duration: 1,
                xflip: false,
                yflip: false,
              },
              {
                command: 'delete',
                oam_set: null,
                duration: 0,
                xflip: false,
                yflip: false,
              },
            ],
          },
          oam_sets: {
            BATTLE_ANIM_OAMSET_TEST: {
              name: 'BATTLE_ANIM_OAMSET_TEST',
              tile_offset: 0,
              entries: [{ x: 0, y: 0, tile_id: 0, xflip: false, yflip: false }],
            },
          },
          gfx_table: {
            BATTLE_ANIM_GFX_TEST: [0, 'AnimObjHitGFX'],
            BATTLE_ANIM_GFX_0: [0, 'AnimObjHitGFX'],
          },
          gfx_sources: {
            AnimObjHitGFX: 'gfx/battle_anims/hit.2bpp.lz',
          },
        };
      },
      readTextAssetSync: () => '; gray\nRGB 31, 31, 31\nRGB 21, 21, 21\nRGB 10, 10, 10\nRGB 0, 0, 0\n',
    }));
    jest.doMock('@pokecrystal/core/core/paths', () => ({
      getAssetPath: (...parts: string[]) => `/tmp/assets/${parts.join('/')}`,
      getAssetsRoot: () => '/tmp/assets',
      getDataDir: () => dataDir,
      getDisassemblyRoot: () => '/tmp/unused-disassembly',
    }));

    jest.isolateModules(() => {
      const { BattleAnimData } = require('./battle-anim-data') as typeof import('./battle-anim-data');
      const data = new BattleAnimData();
      expect(data.object_defs.get('BATTLE_ANIM_OBJ_TEST')).toBeDefined();
      expect(data.get_frameset_frames('BATTLE_ANIM_FRAMESET_TEST')).toHaveLength(2);
      expect(data.get_oam_set('BATTLE_ANIM_OAMSET_TEST')?.entries).toHaveLength(1);
    });
  });
});
