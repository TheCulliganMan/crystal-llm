import path from 'path';

describe('battle catch animation', () => {
  const expectStateSequence = (states: number[], expected: number[]): void => {
    let matchIndex = 0;
    for (const state of states) {
      if (state === expected[matchIndex]) {
        matchIndex += 1;
      }
      if (matchIndex >= expected.length) {
        break;
      }
    }
    expect(matchIndex).toBe(expected.length);
  };

  it('shrinks and hides the target during Throw Poke Ball', () => {
    const disassemblyRoot = path.resolve(
      __dirname,
      '../../../../../vendor/pokecrystal',
    );
    process.env.POKECRYSTAL_DISASSEMBLY_ROOT = disassemblyRoot;
    jest.resetModules();
    const { AnimationPlayer } = require('./battle-animation') as typeof import('./battle-animation');
    const player = new AnimationPlayer({ tile_size: 8 });
    player.play_animation('Throw Poke Ball', true, 0, {
      param_label: 'POKE_BALL',
      shake_count: 1,
    });
    const runtime = player.runtime_state;
    const rowStates: number[] = [];
    let hidden = false;

    for (let i = 0; i < 200; i += 1) {
      player.update();
      if (runtime.enemy_row_mode) {
        rowStates.push(runtime.enemy_row_state);
      }
      if (runtime.enemy_visible === false) {
        hidden = true;
        break;
      }
    }

    expect(hidden).toBe(true);
    expectStateSequence(rowStates, [7, 5, 3]);
  });

  it('re-expands the target after a break-free outcome', () => {
    const disassemblyRoot = path.resolve(
      __dirname,
      '../../../../../vendor/pokecrystal',
    );
    process.env.POKECRYSTAL_DISASSEMBLY_ROOT = disassemblyRoot;
    jest.resetModules();
    const { AnimationPlayer } = require('./battle-animation') as typeof import('./battle-animation');
    const player = new AnimationPlayer({ tile_size: 8 });
    player.play_animation('Throw Poke Ball', true, 0, {
      param_label: 'POKE_BALL',
      shake_count: 0,
    });
    const runtime = player.runtime_state;
    const rowStates: number[] = [];
    let sawHidden = false;
    let sawVisibleAfterHidden = false;

    for (let i = 0; i < 300; i += 1) {
      player.update();
      if (runtime.enemy_row_mode) {
        rowStates.push(runtime.enemy_row_state);
      }
      if (runtime.enemy_visible === false) {
        sawHidden = true;
      } else if (sawHidden) {
        sawVisibleAfterHidden = true;
      }
      if (!player.is_active()) {
        break;
      }
    }

    expect(sawHidden).toBe(true);
    expect(sawVisibleAfterHidden).toBe(true);
    expect(runtime.enemy_visible).toBe(true);
    expectStateSequence(rowStates, [7, 5, 3]);
    expectStateSequence(rowStates, [3, 5, 7]);
  });

  it('finishes the Throw Poke Ball animation within a bounded number of frames', () => {
    const disassemblyRoot = path.resolve(
      __dirname,
      '../../../../../vendor/pokecrystal',
    );
    process.env.POKECRYSTAL_DISASSEMBLY_ROOT = disassemblyRoot;
    jest.resetModules();
    const { AnimationPlayer } = require('./battle-animation') as typeof import('./battle-animation');
    const player = new AnimationPlayer({ tile_size: 8 });
    player.play_animation('Throw Poke Ball', true, 0, {
      param_label: 'POKE_BALL',
      shake_count: 0,
    });

    for (let i = 0; i < 500; i += 1) {
      player.update();
      if (!player.is_active()) {
        break;
      }
    }

    expect(player.is_active()).toBe(false);
  });

  it('finishes the successful Throw Poke Ball animation within a bounded number of frames', () => {
    const disassemblyRoot = path.resolve(
      __dirname,
      '../../../../../vendor/pokecrystal',
    );
    process.env.POKECRYSTAL_DISASSEMBLY_ROOT = disassemblyRoot;
    jest.resetModules();
    const { AnimationPlayer } = require('./battle-animation') as typeof import('./battle-animation');
    const player = new AnimationPlayer({ tile_size: 8 });
    player.play_animation('Throw Poke Ball', true, 0, {
      param_label: 'POKE_BALL',
      shake_count: 4,
    });

    for (let i = 0; i < 1000; i += 1) {
      player.update();
      if (!player.is_active()) {
        break;
      }
    }

    expect(player.is_active()).toBe(false);
  });
});
