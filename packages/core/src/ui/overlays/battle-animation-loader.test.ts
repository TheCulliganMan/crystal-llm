describe('battle-animation-loader', () => {
  afterEach(() => {
    jest.resetModules();
    jest.restoreAllMocks();
  });

  it('throws when generated battle animation sources are unavailable', () => {
    const jsonPath = '/tmp/missing-animations.json';

    jest.doMock('../../core/asset-reader', () => ({
      readJsonAssetSync: () => {
        throw new Error('missing');
      },
    }));
    jest.doMock('../../core/paths', () => ({
      getAnimationsOutputPath: () => jsonPath,
      getDataDir: () => '/tmp',
      getDisassemblyRoot: () => '/tmp/../public/disassembly',
    }));

    jest.isolateModules(() => {
      const { load_animation_scripts } = require('./battle-animation-loader') as typeof import('./battle-animation-loader');

      expect(() => load_animation_scripts()).toThrow(
        `Generated battle animation scripts are required for the asset-only runtime: missing or invalid ${jsonPath}.`
      );
    });
  });

  it('throws when the generated battle animation table is unavailable', () => {
    jest.doMock('../../core/asset-reader', () => ({
      readJsonAssetSync: () => {
        throw new Error('missing');
      },
    }));
    jest.doMock('../../core/paths', () => ({
      getAnimationsOutputPath: () => '/tmp/unused-animations.json',
      getDataDir: () => '/tmp',
      getDisassemblyRoot: () => '/tmp/../public/disassembly',
    }));

    jest.isolateModules(() => {
      const { load_animation_table } = require('./battle-animation-loader') as typeof import('./battle-animation-loader');

      expect(() => load_animation_table()).toThrow(
        'Generated battle animation table is required for the asset-only runtime: missing or invalid /tmp/battle_animation_table.json.'
      );
    });
  });

  it('loads the generated battle animation table', () => {
    jest.doMock('../../core/asset-reader', () => ({
      readJsonAssetSync: (target: string) => {
        if (target !== '/tmp/battle_animation_table.json') {
          throw new Error(`unexpected read: ${target}`);
        }
        return ['BattleAnim_Dummy', 'BattleAnim_Pound'];
      },
    }));
    jest.doMock('../../core/paths', () => ({
      getAnimationsOutputPath: () => '/tmp/unused-animations.json',
      getDataDir: () => '/tmp',
      getDisassemblyRoot: () => '/tmp/../public/disassembly',
    }));

    jest.isolateModules(() => {
      const { load_animation_table } = require('./battle-animation-loader') as typeof import('./battle-animation-loader');

      expect(load_animation_table()).toEqual([
        'BattleAnim_Dummy',
        'BattleAnim_Pound',
      ]);
    });
  });

  it('preserves exported local labels when loading generated animation JSON', () => {
    const jsonPath = '/tmp/generated-animations.json';

    jest.doMock('../../core/asset-reader', () => ({
      readJsonAssetSync: (target: string) => {
        if (target !== jsonPath) {
          throw new Error(`unexpected read: ${target}`);
        }
        return {
          BattleAnim_TestMove: ['anim_wait 4', '.loop', 'anim_obj BATTLE_ANIM_OBJ_HIT, 0, 0, $0', 'anim_loop 3, .loop', 'anim_ret'],
        };
      },
    }));
    jest.doMock('../../core/paths', () => ({
      getAnimationsOutputPath: () => jsonPath,
      getDataDir: () => '/tmp',
      getDisassemblyRoot: () => '/tmp/../public/disassembly',
    }));

    jest.isolateModules(() => {
      const { load_animation_scripts } = require('./battle-animation-loader') as typeof import('./battle-animation-loader');

      const scripts = load_animation_scripts();
      expect(scripts.get('BattleAnim_TestMove')).toEqual({
        name: 'BattleAnim_TestMove',
        script: [
          { command: 'anim_wait', args: ['4'] },
          { command: 'anim_obj', args: ['BATTLE_ANIM_OBJ_HIT', '0', '0', '$0'] },
          { command: 'anim_loop', args: ['3', '.loop'] },
          { command: 'anim_ret', args: [] },
        ],
        labels: { BattleAnim_TestMove: 0, '.loop': 1 },
      });
    });
  });

  it('prefers bundled animation asm to recover stripped labels and split support scripts', () => {
    const jsonPath = '/tmp/generated-animations.json';
    const asmPath = '/tmp/../public/disassembly/data/moves/animations.asm';

    jest.doMock('../../core/asset-reader', () => ({
      readJsonAssetSync: (target: string) => {
        if (target !== jsonPath) {
          throw new Error(`unexpected read: ${target}`);
        }
        return {
          BattleAnim_TestMove: [
            'anim_wait 4',
            'anim_jump .done',
            'anim_wait 99',
            'anim_ret',
            'anim_obj BATTLE_ANIM_OBJ_DRAIN, 132, 44, $0',
            'anim_ret',
          ],
        };
      },
      readTextAssetSync: (target: string) => {
        if (target !== asmPath) {
          throw new Error(`unexpected asm read: ${target}`);
        }
        return [
          'BattleAnim_TestMove:',
          '\tanim_wait 4',
          '\tanim_jump .done',
          '\tanim_wait 99',
          '.done',
          '\tanim_ret',
          '',
          'BattleAnimSub_Drain:',
          '\tanim_obj BATTLE_ANIM_OBJ_DRAIN, 132, 44, $0',
          '\tanim_ret',
        ].join('\n');
      },
    }));
    jest.doMock('../../core/paths', () => ({
      getAnimationsOutputPath: () => jsonPath,
      getDataDir: () => '/tmp/data',
      getDisassemblyRoot: () => '/tmp/../public/disassembly',
    }));

    jest.isolateModules(() => {
      const { load_animation_scripts } = require('./battle-animation-loader') as typeof import('./battle-animation-loader');

      const scripts = load_animation_scripts();
      expect(scripts.get('BattleAnim_TestMove')?.labels).toMatchObject({
        BattleAnim_TestMove: 0,
        '.done': 3,
      });
      expect(scripts.get('BattleAnim_TestMove')?.script).toEqual([
        { command: 'anim_wait', args: ['4'] },
        { command: 'anim_jump', args: ['.done'] },
        { command: 'anim_wait', args: ['99'] },
        { command: 'anim_ret', args: [] },
      ]);
      expect(scripts.get('BattleAnimSub_Drain')?.script).toEqual([
        { command: 'anim_obj', args: ['BATTLE_ANIM_OBJ_DRAIN', '132', '44', '$0'] },
        { command: 'anim_ret', args: [] },
      ]);
    });
  });

  it('preserves global fallthrough labels inside an asm script', () => {
    const jsonPath = '/tmp/generated-animations.json';
    const asmPath = '/tmp/../public/disassembly/data/moves/animations.asm';

    jest.doMock('../../core/asset-reader', () => ({
      readJsonAssetSync: (target: string) => {
        if (target !== jsonPath) {
          throw new Error(`unexpected read: ${target}`);
        }
        return {
          BattleAnim_ReturnMon: [
            'anim_sound 0, 0, SFX_BALL_POOF',
            'anim_bgeffect BATTLE_BG_EFFECT_RETURN_MON, $0, BG_EFFECT_USER, $0',
            'anim_wait 32',
            'anim_ret',
          ],
        };
      },
      readTextAssetSync: (target: string) => {
        if (target !== asmPath) {
          throw new Error(`unexpected asm read: ${target}`);
        }
        return [
          'BattleAnim_ReturnMon:',
          '\tanim_sound 0, 0, SFX_BALL_POOF',
          'BattleAnimSub_Return:',
          '\tanim_bgeffect BATTLE_BG_EFFECT_RETURN_MON, $0, BG_EFFECT_USER, $0',
          '\tanim_wait 32',
          '\tanim_ret',
          '',
          'BattleAnim_Confused:',
          '\tanim_wait 96',
          '\tanim_ret',
        ].join('\n');
      },
    }));
    jest.doMock('../../core/paths', () => ({
      getAnimationsOutputPath: () => jsonPath,
      getDataDir: () => '/tmp/data',
      getDisassemblyRoot: () => '/tmp/../public/disassembly',
    }));

    jest.isolateModules(() => {
      const { load_animation_scripts } = require('./battle-animation-loader') as typeof import('./battle-animation-loader');

      const scripts = load_animation_scripts();
      expect(scripts.get('BattleAnim_ReturnMon')).toEqual({
        name: 'BattleAnim_ReturnMon',
        script: [
          { command: 'anim_sound', args: ['0', '0', 'SFX_BALL_POOF'] },
          { command: 'anim_bgeffect', args: ['BATTLE_BG_EFFECT_RETURN_MON', '$0', 'BG_EFFECT_USER', '$0'] },
          { command: 'anim_wait', args: ['32'] },
          { command: 'anim_ret', args: [] },
        ],
        labels: {
          BattleAnim_ReturnMon: 0,
          BattleAnimSub_Return: 1,
        },
      });
      expect(scripts.get('BattleAnimSub_Return')).toEqual({
        name: 'BattleAnimSub_Return',
        script: [
          { command: 'anim_bgeffect', args: ['BATTLE_BG_EFFECT_RETURN_MON', '$0', 'BG_EFFECT_USER', '$0'] },
          { command: 'anim_wait', args: ['32'] },
          { command: 'anim_ret', args: [] },
        ],
        labels: {
          BattleAnimSub_Return: 0,
        },
      });
      expect(scripts.get('BattleAnim_Confused')?.script).toEqual([
        { command: 'anim_wait', args: ['96'] },
        { command: 'anim_ret', args: [] },
      ]);
    });
  });

  it('registers consecutive global labels as zero-offset aliases when parsing bundled asm', () => {
    const jsonPath = '/tmp/generated-animations.json';
    const asmPath = '/tmp/../public/disassembly/data/moves/animations.asm';

    jest.doMock('../../core/asset-reader', () => ({
      readJsonAssetSync: (target: string) => {
        if (target !== jsonPath) {
          throw new Error(`unexpected read: ${target}`);
        }
        return {
          BattleAnim_Gust: [],
        };
      },
      readTextAssetSync: (target: string) => {
        if (target !== asmPath) {
          throw new Error(`unexpected asm read: ${target}`);
        }
        return [
          'BattleAnim_Gust:',
          'BattleAnim_Sonicboom:',
          '\tanim_1gfx BATTLE_ANIM_GFX_WIND',
          '\tanim_wait 4',
          '\tanim_ret',
        ].join('\n');
      },
    }));
    jest.doMock('../../core/paths', () => ({
      getAnimationsOutputPath: () => jsonPath,
      getDataDir: () => '/tmp/data',
      getDisassemblyRoot: () => '/tmp/../public/disassembly',
    }));

    jest.isolateModules(() => {
      const { load_animation_scripts } = require('./battle-animation-loader') as typeof import('./battle-animation-loader');

      const scripts = load_animation_scripts();
      expect(scripts.get('BattleAnim_Sonicboom')).toEqual({
        name: 'BattleAnim_Sonicboom',
        script: [
          { command: 'anim_1gfx', args: ['BATTLE_ANIM_GFX_WIND'] },
          { command: 'anim_wait', args: ['4'] },
          { command: 'anim_ret', args: [] },
        ],
        labels: {
          BattleAnim_Gust: 0,
          BattleAnim_Sonicboom: 0,
        },
      });
    });
  });

  it('loads generated animation JSON', () => {
    const jsonPath = '/tmp/generated-animations.json';

    jest.doMock('../../core/asset-reader', () => ({
      readJsonAssetSync: (target: string) => {
        if (target !== jsonPath) {
          throw new Error(`unexpected read: ${target}`);
        }
        return {
          BattleAnim_TestMove: ['anim_wait 4', 'anim_ret'],
        };
      },
    }));
    jest.doMock('../../core/paths', () => ({
      getAnimationsOutputPath: () => jsonPath,
      getDataDir: () => '/tmp',
      getDisassemblyRoot: () => '/tmp/../public/disassembly',
    }));

    jest.isolateModules(() => {
      const { load_animation_scripts } = require('./battle-animation-loader') as typeof import('./battle-animation-loader');

      const scripts = load_animation_scripts();
      expect(scripts.get('BattleAnim_TestMove')).toEqual({
        name: 'BattleAnim_TestMove',
        script: [
          { command: 'anim_wait', args: ['4'] },
          { command: 'anim_ret', args: [] },
        ],
        labels: { BattleAnim_TestMove: 0 },
      });
      expect(scripts.has('BattleAnim_FaintMon')).toBe(true);
    });
  });
});
