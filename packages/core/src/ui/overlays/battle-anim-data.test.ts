import fs from 'fs';
import path from 'path';

describe('BattleAnimData', () => {
  const registerBattlerSurfaces = (data: import('./battle-anim-data').BattleAnimData) => {
    const { Surface } = require('../surface') as typeof import('../surface');
    const enemySurface = new Surface(56, 56);
    const playerSurface = new Surface(48, 48);

    enemySurface.fill([255, 255, 255, 0]);
    playerSurface.fill([255, 255, 255, 0]);
    for (let y = 40; y < 56; y += 1) {
      for (let x = 0; x < 56; x += 1) {
        enemySurface.set_at([x, y], [0, 0, 0, 255]);
      }
    }
    for (let y = 0; y < 16; y += 1) {
      for (let x = 0; x < 48; x += 1) {
        playerSurface.set_at([x, y], [0, 0, 0, 255]);
      }
    }

    data.register_battler_surfaces({ playerSurface, enemySurface });
  };

  const configureBundledAssets = () => {
    delete process.env.POKECRYSTAL_DISASSEMBLY_ROOT;
  };

  afterEach(() => {
    jest.restoreAllMocks();
  });

  it('renders smoke object gfx in browser runtime from a canonical /assets path', () => {
    configureBundledAssets();
    jest.resetModules();
    const originalWindow = (globalThis as typeof globalThis & { window?: unknown }).window;
    const originalXHR = (globalThis as typeof globalThis & { XMLHttpRequest?: unknown }).XMLHttpRequest;
    const bundlePath = path.resolve(process.cwd(), '..', '..', 'apps', 'web', 'assets', 'data', 'battle_anim_bundle.json');
    const palettePath = path.resolve(process.cwd(), '..', '..', 'apps', 'web', 'assets', 'gfx', 'battle_anims', 'battle_anims.pal');
    const smokePath = path.resolve(process.cwd(), '..', '..', 'apps', 'web', 'assets', 'gfx', 'battle_anims', 'smoke.2bpp');
    const requestedTargets: string[] = [];
    const smokeBinary = fs.readFileSync(smokePath);
    const smokeResponseText = Array.from(smokeBinary, (value) => String.fromCharCode(value)).join('');
    const fsBrowserPath = path.resolve(
      process.cwd(),
      '..',
      '..',
      'apps',
      'web',
      'src',
      'shims',
      'fs-browser',
    );

    class MockXMLHttpRequest {
      status = 0;
      responseText = '';
      response: ArrayBuffer | null = null;
      private target = '';
      private method = 'GET';

      open(method: string, target: string): void {
        this.method = method;
        this.target = target;
        requestedTargets.push(target);
      }

      send(): void {
        if (this.target === '/assets/data/battle_anim_bundle.json') {
          this.status = 200;
          this.responseText = fs.readFileSync(bundlePath, 'utf-8');
          return;
        }
        if (this.target === '/assets/gfx/battle_anims/battle_anims.pal') {
          this.status = 200;
          this.responseText = fs.readFileSync(palettePath, 'utf-8');
          return;
        }
        if (this.target === '/assets/gfx/battle_anims/smoke.2bpp') {
          this.status = 200;
          this.responseText = this.method === 'HEAD' ? '' : smokeResponseText;
          return;
        }
        this.status = 404;
        this.responseText = '';
      }

      overrideMimeType(): void {}
    }

    try {
      (globalThis as typeof globalThis & { window?: unknown }).window = {} as unknown;
      (globalThis as typeof globalThis & { XMLHttpRequest?: unknown }).XMLHttpRequest =
        MockXMLHttpRequest as unknown;
      jest.doMock('fs', () => {
        const shim = require(fsBrowserPath) as typeof import('../../../../../apps/web/src/shims/fs-browser');
        return {
          __esModule: true,
          ...shim,
          default: shim.default ?? shim,
        };
      });

      jest.isolateModules(() => {
        const { BattleAnimData } = require('./battle-anim-data') as typeof import('./battle-anim-data');
        const data = new BattleAnimData() as import('./battle-anim-data').BattleAnimData & {
          gfxSources: Map<string, string>;
        };

        expect(data.gfxSources.get('AnimObjSmokeGFX')).toBe('/assets/gfx/battle_anims/smoke.2bpp');
        const rendered = data.render_sprite('BATTLE_ANIM_OBJ_BALL_POOF', 0);
        expect(rendered).not.toBeNull();
        if (rendered) {
          expect(rendered.surface.get_width()).toBeGreaterThan(0);
          expect(rendered.surface.get_height()).toBeGreaterThan(0);
        }
      });
    } finally {
      (globalThis as typeof globalThis & { window?: unknown }).window = originalWindow;
      (globalThis as typeof globalThis & { XMLHttpRequest?: unknown }).XMLHttpRequest = originalXHR;
      jest.dontMock('fs');
    }

    expect(requestedTargets).toEqual(
      expect.arrayContaining([
        '/assets/data/battle_anim_bundle.json',
        '/assets/gfx/battle_anims/battle_anims.pal',
        '/assets/gfx/battle_anims/smoke.2bpp',
      ]),
    );
    expect(requestedTargets).not.toContain('/assets/gfx/battle_anims/smoke.2bpp.lz');
  });

  it('loads object definitions and renders a battler sprite frame', () => {
    configureBundledAssets();
    jest.resetModules();
    const { BattleAnimData } = require('./battle-anim-data') as typeof import('./battle-anim-data');
    const data = new BattleAnimData();
    const obj = data.object_defs.get('BATTLE_ANIM_OBJ_PLAYERHEAD_2ROW');
    expect(obj).toBeDefined();
    const dataAny = data as unknown as {
      framesets?: Map<string, unknown>;
      oamSets?: Map<string, unknown>;
      gfxTable?: Map<string, unknown>;
      gfxSources?: Map<string, string>;
    };
    expect(dataAny.framesets?.size ?? 0).toBeGreaterThan(0);
    expect(dataAny.oamSets?.size ?? 0).toBeGreaterThan(0);
    expect(dataAny.gfxTable?.size ?? 0).toBeGreaterThan(0);
    expect(dataAny.gfxSources?.size ?? 0).toBeGreaterThan(0);
    registerBattlerSurfaces(data);
    const rendered = data.render_sprite('BATTLE_ANIM_OBJ_PLAYERHEAD_2ROW', 0);
    expect(rendered).not.toBeNull();
    if (rendered) {
      expect(rendered.surface.get_width()).toBeGreaterThan(0);
      expect(rendered.surface.get_height()).toBeGreaterThan(0);
    }
  });

  it('renders bundled smoke object gfx even when the bundle source points at legacy .lz paths', () => {
    configureBundledAssets();
    jest.resetModules();
    const { BattleAnimData } = require('./battle-anim-data') as typeof import('./battle-anim-data');
    const data = new BattleAnimData();

    const rendered = data.render_sprite('BATTLE_ANIM_OBJ_BETA_BALL_POOF', 0);

    expect(rendered).not.toBeNull();
    if (rendered) {
      expect(rendered.surface.get_width()).toBeGreaterThan(0);
      expect(rendered.surface.get_height()).toBeGreaterThan(0);
    }
  });

  it('normalizes legacy smoke gfx bundle paths to the cached uncompressed asset path', () => {
    configureBundledAssets();
    jest.resetModules();
    const { BattleAnimData } = require('./battle-anim-data') as typeof import('./battle-anim-data');
    const data = new BattleAnimData() as import('./battle-anim-data').BattleAnimData & {
      gfxSources: Map<string, string>;
    };

    expect(data.gfxSources.get('AnimObjSmokeGFX')).toMatch(/smoke\.2bpp$/);
    data.render_sprite('BATTLE_ANIM_OBJ_BALL_POOF', 0);
    expect(data.gfxSources.get('AnimObjSmokeGFX')).toMatch(/smoke\.2bpp$/);
  });

  it('mirrors frame-level X flip flags like the ASM', () => {
    configureBundledAssets();
    jest.resetModules();
    const { BattleAnimData } = require('./battle-anim-data') as typeof import('./battle-anim-data');
    const data = new BattleAnimData();
    const left = data.get_frameset_frames('BATTLE_ANIM_FRAMESET_CUT_DOWN_LEFT') as Array<{ command: string; xflip: boolean }> | null;
    const right = data.get_frameset_frames('BATTLE_ANIM_FRAMESET_CUT_DOWN_RIGHT') as Array<{ command: string; xflip: boolean }> | null;
    expect(left).not.toBeNull();
    expect(right).not.toBeNull();
    expect(left?.find((entry) => entry.command === 'frame')?.xflip).toBe(false);
    expect(right?.find((entry) => entry.command === 'frame')?.xflip).toBe(true);
  });

  it('renders battler row objects from registered battler surfaces instead of NULL gfx placeholders', () => {
    configureBundledAssets();
    jest.resetModules();
    const { BattleAnimData } = require('./battle-anim-data') as typeof import('./battle-anim-data');
    const { Surface } = require('../surface') as typeof import('../surface');
    const data = new BattleAnimData();
    const enemySurface = new Surface(56, 56);
    const playerSurface = new Surface(48, 48);

    enemySurface.fill([255, 255, 255, 0]);
    playerSurface.fill([255, 255, 255, 0]);
    for (let y = 40; y < 56; y += 1) {
      for (let x = 0; x < 56; x += 1) {
        enemySurface.set_at([x, y], [0, 0, 0, 255]);
      }
    }
    for (let y = 0; y < 16; y += 1) {
      for (let x = 0; x < 48; x += 1) {
        playerSurface.set_at([x, y], [0, 0, 0, 255]);
      }
    }

    data.register_battler_surfaces({ playerSurface, enemySurface });

    const enemyFeet = data.render_sprite('BATTLE_ANIM_OBJ_ENEMYFEET_2ROW', 0);
    const playerHead = data.render_sprite('BATTLE_ANIM_OBJ_PLAYERHEAD_2ROW', 0);

    expect(enemyFeet).not.toBeNull();
    expect(playerHead).not.toBeNull();
    if (enemyFeet && playerHead) {
      expect(enemyFeet.surface.get_width()).toBeGreaterThan(0);
      expect(enemyFeet.surface.get_height()).toBeGreaterThan(0);
      expect(playerHead.surface.get_width()).toBeGreaterThan(0);
      expect(playerHead.surface.get_height()).toBeGreaterThan(0);
    }
  });

  it('keeps palette index 0 transparent for battler row objects', () => {
    configureBundledAssets();
    jest.resetModules();
    const { BattleAnimData } = require('./battle-anim-data') as typeof import('./battle-anim-data');
    const { Surface } = require('../surface') as typeof import('../surface');
    const data = new BattleAnimData();
    const playerSurface = new Surface(48, 48);

    playerSurface.fill([255, 255, 255, 0]);
    playerSurface.set_at([1, 1], [0, 0, 0, 255]);

    data.register_battler_surfaces({ playerSurface });

    const dataAny = data as unknown as {
      tile_surface: (
        gfxLabel: string,
        paletteName: string,
        tileIndex: number,
      ) => any;
    };
    const rendered = dataAny.tile_surface('BATTLE_ANIM_GFX_ENEMYFEET', 'red', 0);

    expect(rendered).not.toBeNull();
    expect(rendered?.get_at([0, 0])[3]).toBe(0);
    expect(rendered?.get_at([1, 1])[3]).toBe(255);
  });

  it('maps palette override constants to palette names', () => {
    configureBundledAssets();
    jest.resetModules();
    const { BattleAnimData } = require('./battle-anim-data') as typeof import('./battle-anim-data');
    const data = new BattleAnimData();
    const dataAny = data as unknown as { tileCache: Map<string, unknown> };
    registerBattlerSurfaces(data);
    const rendered = data.render_sprite('BATTLE_ANIM_OBJ_PLAYERHEAD_2ROW', 0, {
      palette_override: 'PAL_BATTLE_OB_RED',
    });
    expect(rendered).not.toBeNull();
    const keys = Array.from(dataAny.tileCache.keys());
    expect(keys.some((key) => key.includes(':red:'))).toBe(true);
    expect(keys.some((key) => key.includes(':PAL_BATTLE_OB_RED:'))).toBe(false);
  });

  it('throws on unknown palette override constants instead of mapping them to gray', () => {
    configureBundledAssets();
    jest.resetModules();
    const { BattleAnimData } = require('./battle-anim-data') as typeof import('./battle-anim-data');
    const data = new BattleAnimData();

    expect(() =>
      data.render_sprite('BATTLE_ANIM_OBJ_POKE_BALL', 0, {
        palette_override: 'PAL_BATTLE_OB_NOT_REAL',
      })
    ).toThrow("Unknown battle animation palette constant 'PAL_BATTLE_OB_NOT_REAL'.");
  });

  it('throws when a required battle animation palette section is missing', () => {
    configureBundledAssets();
    jest.resetModules();
    const assetReader = require('@pokecrystal/core/core/asset-reader') as typeof import('@pokecrystal/core/core/asset-reader');
    const originalReadText = assetReader.readTextAssetSync;
    jest.spyOn(assetReader, 'readTextAssetSync').mockImplementation(((target: string) => {
      if (target.endsWith(path.join('gfx', 'battle_anims', 'battle_anims.pal'))) {
        const raw = originalReadText(target);
        const lines = raw.split(/\r?\n/);
        const filtered: string[] = [];
        let skipping = false;
        for (const line of lines) {
          const trimmed = line.trim().toLowerCase();
          if (trimmed.startsWith(';')) {
            skipping = trimmed === '; red';
            if (skipping) {
              continue;
            }
          }
          if (!skipping) {
            filtered.push(line);
          }
        }
        return filtered.join('\n');
      }
      return originalReadText(target);
    }) as typeof assetReader.readTextAssetSync);

    const { BattleAnimData } = require('./battle-anim-data') as typeof import('./battle-anim-data');
    const data = new BattleAnimData();
    registerBattlerSurfaces(data);

    expect(() =>
      data.render_sprite('BATTLE_ANIM_OBJ_PLAYERHEAD_2ROW', 0, {
        palette_override: 'PAL_BATTLE_OB_RED',
      })
    ).toThrow("Missing battle animation palette 'red'.");
  });

  it('parses ASM fallthrough framesets and dual-flip oamframe flags', () => {
    configureBundledAssets();
    jest.resetModules();
    const { BattleAnimData } = require('./battle-anim-data') as typeof import('./battle-anim-data');
    const data = new BattleAnimData();

    const iceBuildup = data.get_frameset_frames('BATTLE_ANIM_FRAMESET_ICE_BUILDUP') as Array<{
      command: string;
      oam_set: string | null;
    }> | null;
    expect(iceBuildup).not.toBeNull();
    if (iceBuildup) {
      expect(iceBuildup[0]?.oam_set).toBe('BATTLE_ANIM_OAMSET_32');
      expect(iceBuildup.some((entry) => entry.command === 'wait')).toBe(true);
      expect(iceBuildup[iceBuildup.length - 1]?.command).toBe('delete');
    }

    const speedLine1 = data.get_frameset_frames('BATTLE_ANIM_FRAMESET_SPEED_LINE_1') as Array<{
      command: string;
      oam_set: string | null;
    }> | null;
    expect(speedLine1).not.toBeNull();
    if (speedLine1) {
      expect(speedLine1[0]?.oam_set).toBe('BATTLE_ANIM_OAMSET_A0');
      expect(speedLine1[1]?.oam_set).toBe('BATTLE_ANIM_OAMSET_A1');
      expect(speedLine1[2]?.oam_set).toBe('BATTLE_ANIM_OAMSET_A2');
      expect(speedLine1[3]?.command).toBe('delete');
    }

    const cutUpRight = data.get_frameset_frames('BATTLE_ANIM_FRAMESET_CUT_UP_RIGHT') as Array<{
      command: string;
      xflip: boolean;
      yflip: boolean;
    }> | null;
    expect(cutUpRight).not.toBeNull();
    if (cutUpRight) {
      const firstFrame = cutUpRight.find((entry) => entry.command === 'frame');
      expect(firstFrame?.xflip).toBe(true);
      expect(firstFrame?.yflip).toBe(true);
    }
  });

  it('honors battleanimoam entry lengths when rendering ember fire sprites', () => {
    configureBundledAssets();
    jest.resetModules();
    const { BattleAnimData } = require('./battle-anim-data') as typeof import('./battle-anim-data');
    const data = new BattleAnimData();
    const dataAny = data as unknown as {
      oamSets: Map<string, { entries: Array<unknown> }>;
    };

    expect(dataAny.oamSets.get('BATTLE_ANIM_OAMSET_0F')?.entries).toHaveLength(1);
    expect(dataAny.oamSets.get('BATTLE_ANIM_OAMSET_10')?.entries).toHaveLength(1);
    expect(() => data.render_sprite('BATTLE_ANIM_OBJ_EMBER', 0)).not.toThrow();
  });

  it('throws when the bundled runtime contains an invalid frame duration instead of defaulting timing', () => {
    configureBundledAssets();
    jest.resetModules();
    const assetReader = require('@pokecrystal/core/core/asset-reader') as typeof import('@pokecrystal/core/core/asset-reader');
    const originalReadJson = assetReader.readJsonAssetSync;
    jest.spyOn(assetReader, 'readJsonAssetSync').mockImplementation(((target: string) => {
      const bundle = originalReadJson(target) as Record<string, unknown>;
      const cloned = JSON.parse(JSON.stringify(bundle));
      cloned.framesets.BATTLE_ANIM_FRAMESET_PUNCH[0].duration = 'NOT_A_NUMBER';
      return cloned;
    }) as typeof assetReader.readJsonAssetSync);

    const { BattleAnimData } = require('./battle-anim-data') as typeof import('./battle-anim-data');
    expect(() => new BattleAnimData()).toThrow('Cannot parse integer token: NOT_A_NUMBER');
  });

  it('throws when an OAM piece tile is missing instead of rendering a partial sprite', () => {
    configureBundledAssets();
    jest.resetModules();
    const { BattleAnimData } = require('./battle-anim-data') as typeof import('./battle-anim-data');
    const data = new BattleAnimData() as BattleAnimData & {
      tile_surface: (gfxLabel: string, paletteName: string, tileIndex: number) => unknown;
    };

    const originalTileSurface = data.tile_surface.bind(data);
    let failed = false;
    data.tile_surface = ((gfxLabel: string, paletteName: string, tileIndex: number) => {
      if (!failed) {
        failed = true;
        return null;
      }
      return originalTileSurface(gfxLabel, paletteName, tileIndex);
    }) as typeof data.tile_surface;

    expect(() => data.render_sprite('BATTLE_ANIM_OBJ_PUNCH', 0)).toThrow(
      /Missing battle animation tile \d+ for BATTLE_ANIM_OBJ_PUNCH/,
    );
  });

  it('falls back to an uncompressed sibling when a legacy .lz gfx path is missing', () => {
    configureBundledAssets();
    jest.resetModules();
    const originalExistsSync = fs.existsSync;
    const originalReadFileSync = fs.readFileSync;
    const { BattleAnimData } = require('./battle-anim-data') as typeof import('./battle-anim-data');
    const data = new BattleAnimData() as BattleAnimData & {
      gfxSources: Map<string, string>;
    };

    const compressedPath = '/tmp/fake-battle-anim.2bpp.lz';
    const fallbackPath = '/tmp/fake-battle-anim.2bpp';
    const realFallbackAsset = path.resolve(process.cwd(), '..', '..', 'apps', 'web', 'assets', 'gfx', 'battle_anims', 'hit.2bpp');
    data.gfxSources.set('AnimObjHitGFX', compressedPath);

    jest.spyOn(fs, 'existsSync').mockImplementation(((target: fs.PathLike) => {
      const value = String(target);
      if (value === compressedPath) {
        return false;
      }
      if (value === fallbackPath) {
        return true;
      }
      return originalExistsSync.call(fs, target);
    }) as typeof fs.existsSync);

    jest.spyOn(fs, 'readFileSync').mockImplementation(((target: fs.PathLike, options?: unknown) => {
      const value = String(target);
      if (value === fallbackPath) {
        return originalReadFileSync.call(
          fs,
          realFallbackAsset,
          options as Parameters<typeof fs.readFileSync>[1],
        );
      }
      return originalReadFileSync.call(fs, target, options as Parameters<typeof fs.readFileSync>[1]);
    }) as typeof fs.readFileSync);

    expect(() => data.render_sprite('BATTLE_ANIM_OBJ_PUNCH', 0)).not.toThrow();
  });

  it('retries the uncompressed sibling when the primary compressed asset is unreadable', () => {
    configureBundledAssets();
    jest.resetModules();
    const originalExistsSync = fs.existsSync;
    const originalReadFileSync = fs.readFileSync;
    const { BattleAnimData } = require('./battle-anim-data') as typeof import('./battle-anim-data');
    const data = new BattleAnimData() as BattleAnimData & {
      gfxSources: Map<string, string>;
    };

    const compressedPath = '/tmp/fake-battle-anim.2bpp.lz';
    const fallbackPath = '/tmp/fake-battle-anim.2bpp';
    const realFallbackAsset = path.resolve(process.cwd(), '..', '..', 'apps', 'web', 'assets', 'gfx', 'battle_anims', 'hit.2bpp');
    data.gfxSources.set('AnimObjHitGFX', compressedPath);

    jest.spyOn(fs, 'existsSync').mockImplementation(((target: fs.PathLike) => {
      const value = String(target);
      if (value === compressedPath || value === fallbackPath) {
        return true;
      }
      return originalExistsSync.call(fs, target);
    }) as typeof fs.existsSync);

    jest.spyOn(fs, 'readFileSync').mockImplementation(((target: fs.PathLike, options?: unknown) => {
      const value = String(target);
      if (value === compressedPath) {
        return Buffer.alloc(0);
      }
      if (value === fallbackPath) {
        return originalReadFileSync.call(
          fs,
          realFallbackAsset,
          options as Parameters<typeof fs.readFileSync>[1],
        );
      }
      return originalReadFileSync.call(fs, target, options as Parameters<typeof fs.readFileSync>[1]);
    }) as typeof fs.readFileSync);

    expect(() => data.render_sprite('BATTLE_ANIM_OBJ_PUNCH', 0)).not.toThrow();
    expect(data.gfxSources.get('AnimObjHitGFX')).toBe(fallbackPath);
  });

  it('does not substitute alternate graphics paths when a bundled asset is missing', () => {
    configureBundledAssets();
    jest.resetModules();
    const originalExistsSync = fs.existsSync;
    const { BattleAnimData } = require('./battle-anim-data') as typeof import('./battle-anim-data');
    const data = new BattleAnimData() as import('./battle-anim-data').BattleAnimData & {
      gfxSources: Map<string, string>;
    };
    const disassemblyPath = '/tmp/fake-disassembly-root/gfx/battle_anims/hit.2bpp.lz';
    const assetsPath = '/tmp/fake-assets-root/gfx/battle_anims/hit.2bpp.lz';
    data.gfxSources.set('AnimObjHitGFX', disassemblyPath);

    jest.spyOn(fs, 'existsSync').mockImplementation(((target: fs.PathLike) => {
      const value = String(target);
      if (value === disassemblyPath) {
        return false;
      }
      if (value === assetsPath) {
        return true;
      }
      return originalExistsSync.call(fs, target);
    }) as typeof fs.existsSync);

    expect(() => data.render_sprite('BATTLE_ANIM_OBJ_PUNCH', 0)).toThrow(
      /Missing battle animation tile \d+ for BATTLE_ANIM_OBJ_PUNCH/,
    );
  });
});
