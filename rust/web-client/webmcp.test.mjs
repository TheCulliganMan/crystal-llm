import test from 'node:test';
import assert from 'node:assert/strict';
import { createGameBridge, gameTools, registerGameTools } from './webmcp.js';

test('tools register with current Document API and unregister through abort', async () => {
  const registered = [];
  let canceled = 0;
  const doc = { modelContext: { async registerTool(tool, options) { registered.push({ tool, options }); } } };
  const result = await registerGameTools(doc, { cancel() { canceled++; } });
  assert.equal(result.supported, true);
  assert.equal(registered.length, 7);
  assert.ok(registered.every(({ options }) => options.signal instanceof AbortSignal));
  assert.ok(registered.some(({ tool }) => tool.name === 'pokemon_press'));
  result.dispose();
  assert.ok(registered.every(({ options }) => options.signal.aborted));
  assert.equal(canceled, 1);
});

test('unsupported browsers stay unsupported without installing a shim', async () => {
  const doc = {};
  assert.deepEqual(await registerGameTools(doc, {}), { supported: false, tools: [] });
  assert.equal(doc.modelContext, undefined);
});

test('partial registration failure aborts every registered tool', async () => {
  const signals = [];
  await assert.rejects(registerGameTools({ modelContext: { async registerTool(tool, { signal }) {
    signals.push(signal); if (signals.length === 2) throw new Error('denied');
  } } }, { cancel() {} }), /denied/);
  assert.ok(signals.every(signal => signal.aborted));
});

test('press validates bounds, rejects hidden actions, and returns actual outcome', async () => {
  const calls = [];
  const tools = gameTools({ async execute(command) { calls.push(command); return { status: { screen: 'battle' } }; } });
  const press = tools.find(tool => tool.name === 'pokemon_press');
  for (const input of [{ button: 'wait' }, { button: 'a', frames: 0 }, { button: 'a', frames: 61 }, { button: 'a', frames: 1.5 }, { button: 'a', warp: 'x' }]) {
    await assert.rejects(press.execute(input), TypeError);
  }
  assert.equal(calls.length, 0);
  assert.deepEqual(await press.execute({ button: 'a' }), { status: { screen: 'battle' } });
  assert.deepEqual(calls, [{ kind: 'press', button: 'a', frames: 1 }]);
  assert.equal(press.annotations.readOnlyHint, false);
});

test('bridge polls a completed game frame and rejects simultaneous calls', async () => {
  let polls = 0;
  const bridge = createGameBridge({ crystal_webmcp_request: () => 7,
    crystal_webmcp_poll: id => { assert.equal(id, 7); return ++polls === 1 ? undefined : '{"frame":42}'; },
  }, { intervalMs: 1 });
  const first = bridge.execute({ kind: 'observe' });
  await assert.rejects(bridge.execute({ kind: 'observe' }), /already in progress/);
  assert.deepEqual(await first, { frame: 42 });
});

test('abort and timeout cancel WASM input without replaying actions', async () => {
  let cancels = 0;
  let requests = 0;
  const wasm = { crystal_webmcp_request: () => ++requests, crystal_webmcp_poll() {}, crystal_webmcp_cancel() { cancels++; } };
  const bridge = createGameBridge(wasm, { timeoutMs: 5, intervalMs: 1 });
  const controller = new AbortController();
  const result = bridge.execute({ kind: 'press', button: 'right', frames: 60 }, { signal: controller.signal });
  controller.abort();
  await assert.rejects(result, { name: 'AbortError' });
  assert.ok(cancels > 0);
  await assert.rejects(bridge.execute({ kind: 'observe' }), /timed out/);
  assert.equal(requests, 2);
});

test('multiplayer tools use only the existing facing-player interactions', async () => {
  const calls = [];
  const tool = gameTools({ async execute(command) { calls.push(command); return { multiplayer: { session_active: false } }; } }).find(tool => tool.name === 'pokemon_multiplayer');
  await assert.rejects(tool.execute({ interaction: 'teleport' }), TypeError);
  await tool.execute({ interaction: 'trade' });
  assert.deepEqual(calls, [{ kind: 'multiplayer', interaction: 'trade' }]);
});
