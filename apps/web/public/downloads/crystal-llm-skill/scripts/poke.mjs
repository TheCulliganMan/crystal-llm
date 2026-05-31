#!/usr/bin/env node
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { callMcpTool } from './mcp_call.mjs';

const SCRIPT_DIR = path.dirname(fileURLToPath(import.meta.url));
const HOME = process.env.HOME || os.homedir();
const CODEX_HOME = process.env.CODEX_HOME || path.join(HOME, '.codex');
const ROOT = process.env.POKECRYSTAL_REPO || process.cwd();
const LEARNING_STATE =
  process.env.POKECRYSTAL_LEARNING_STATE ||
  path.join(CODEX_HOME, 'pokecrystal/poke_learning_state.json');
const DEFAULT_IMAGE_DIR = path.join(CODEX_HOME, 'pokecrystal/mcp-images/stock-tools');

function usage() {
  console.error(`Usage:
  poke.mjs status
  poke.mjs observe [--grid] [--no-image] [--save-images <dir>]
  poke.mjs route [--no-image] [--tiles] [--save-images <dir>] [--cell-size 8]
  poke.mjs proof [label]
  poke.mjs context
  poke.mjs move <up|down|left|right> [--steps 1]
  poke.mjs press <A|B|Start|Select|up|down|left|right> [--times 1]
  poke.mjs clear [--max 20]
  poke.mjs events [--limit 8]
  poke.mjs tools

Outputs compact JSON for token-efficient agent use.`);
}

function parseCli(argv) {
  const opts = { cmd: argv[0], args: [], flags: {} };
  for (let i = 1; i < argv.length; i += 1) {
    const value = argv[i];
    if (value === '--help' || value === '-h') {
      usage();
      process.exit(0);
    }
    if (value.startsWith('--')) {
      const key = value.slice(2);
      const next = argv[i + 1];
      if (!next || next.startsWith('--')) {
        opts.flags[key] = true;
      } else {
        opts.flags[key] = next;
        i += 1;
      }
      continue;
    }
    opts.args.push(value);
  }
  if (!opts.cmd) throw new Error('missing command');
  return opts;
}

function numberFlag(flags, name, fallback) {
  const raw = flags[name];
  if (raw === undefined || raw === true) return fallback;
  const parsed = Number(raw);
  if (!Number.isFinite(parsed) || parsed < 1) throw new Error(`--${name} must be a positive number`);
  return Math.floor(parsed);
}

function readJson(filePath, fallback = null) {
  try {
    return JSON.parse(fs.readFileSync(filePath, 'utf8'));
  } catch {
    return fallback;
  }
}

function truncate(value, max = 900) {
  const text = String(value || '');
  return text.length <= max ? text : `${text.slice(0, max - 3)}...`;
}

async function callMcp(tool, args = {}, saveImages = null) {
  return callMcpTool(tool, args || {}, { repo: ROOT, saveImages });
}

function contentTexts(result) {
  return (result?.content || [])
    .filter((item) => item.type === 'text')
    .map((item) => item.text);
}

function firstObject(result) {
  for (const text of contentTexts(result)) {
    if (text && typeof text === 'object') return text;
    if (typeof text === 'string') {
      try {
        const parsed = JSON.parse(text);
        if (parsed && typeof parsed === 'object') return parsed;
      } catch {}
    }
  }
  return null;
}

function compactStatus(status) {
  if (!status) return null;
  const money = status.money ?? status.resources?.money ?? null;
  const momsMoney = status.moms_money ?? status.momsMoney ?? status.resources?.moms_money ?? status.resources?.momsMoney ?? null;
  const momSavingSomeMoney =
    status.mom_saving_some_money ??
    status.momSavingSomeMoney ??
    status.resources?.mom_saving_some_money ??
    status.resources?.momSavingSomeMoney ??
    null;
  return {
    mode: status.mode || null,
    map: status.map || status.location || null,
    coords: status.coords || null,
    facing: status.facing || null,
    canMove: status.canMove ?? null,
    blockedReason: status.blockedReason || null,
    inBattle: Boolean(status.inBattle),
    inDialog: Boolean(status.inDialog || status.textBoxOpen),
    partyCount: status.partyCount ?? null,
    badges: status.badges ?? null,
    money,
    momsMoney,
    momSavingSomeMoney,
    goal: status.flowNextGoal || status.flowSummary || null,
    lastAudio: status.audio?.recentEvents?.at?.(-1)?.token || status.audio?.musicToken || null,
  };
}

function compactObserve(result, { grid = false } = {}) {
  const object = firstObject(result) || {};
  const snapshot = result.snapshot || object;
  const view = snapshot.view || {};
  const map = snapshot.map || {};
  const ctx = snapshot.ctx || {};
  const hotspots = Array.isArray(map.hs) ? map.hs.slice(0, 10).map((hotspot) => ({
    type: hotspot.t,
    label: hotspot.l,
    xy: hotspot.xy,
  })) : [];
  const out = {
    map: ctx.map || null,
    coords: ctx.xy || map.p || null,
    facing: ctx.dir || null,
    focus: view.focus || null,
    viewportPos: view.pos || null,
    ahead: view.ahead || null,
    last: ctx.last || null,
    hotspots,
    imagePaths: result.imagePaths || [],
  };
  if (grid && Array.isArray(view.viewport)) out.viewport = view.viewport;
  return out;
}

function compactRouteRender(result, { grid = true } = {}) {
  const snapshot = firstObject(result) || {};
  const out = {
    available: snapshot.available ?? null,
    reason: snapshot.reason || null,
    map: snapshot.map || null,
    mapId: snapshot.map_id || null,
    coordStride: snapshot.coord_stride || null,
    size: snapshot.size || null,
    player: snapshot.player || null,
    legend: snapshot.legend || [],
    warps: Array.isArray(snapshot.warps) ? snapshot.warps.slice(0, 20) : [],
    hotspots: Array.isArray(snapshot.hotspots)
      ? snapshot.hotspots.slice(0, 20).map((hotspot) => ({
          type: hotspot.type,
          label: hotspot.label,
          coords: hotspot.coords,
          token: hotspot.token,
          approach_tiles: hotspot.approach_tiles,
        }))
      : [],
    imagePaths: result.imagePaths || [],
  };
  if (grid && snapshot.grid?.rows) {
    out.grid = {
      origin: snapshot.grid.origin,
      rows: snapshot.grid.rows,
    };
  }
  return out;
}

function compactRouteContext(status) {
  const state = readJson(LEARNING_STATE, {});
  const map = status?.map || status?.location || null;
  const memory = map ? state.routeMemory?.[map] || null : null;
  return {
    map,
    status: compactStatus(status),
    nextPrompt: truncate(state.nextPrompt, 900),
    currentHypothesis: truncate(state.currentHypothesis, 700),
    routeMemory: memory ? {
      lastUpdated: memory.lastUpdated || null,
      note: truncate(memory.note, 500),
      privateNavigationHint: truncate(memory.privateNavigationHint, 1000),
      routeSteps: memory.privateNavigationPlan?.routeSteps || [],
      battleSafety: memory.battleSafety || null,
    } : null,
  };
}

function print(value) {
  console.log(JSON.stringify(value, null, 2));
}

async function main() {
  const opts = parseCli(process.argv.slice(2));
  const { cmd, args, flags } = opts;

  if (cmd === 'status') {
    print(compactStatus(firstObject(await callMcp('status'))));
    return;
  }

  if (cmd === 'observe') {
    const saveImages = flags['no-image'] ? null : (flags['save-images'] || DEFAULT_IMAGE_DIR);
    const result = await callMcp('observe', {
      include_image: !flags['no-image'],
      image_scale: 2,
      advance_frames: 1,
      detail: 'compact',
      format: 'json',
    }, saveImages);
    print(compactObserve(result, { grid: Boolean(flags.grid) }));
    return;
  }

  if (cmd === 'route') {
    const includeImage = !flags['no-image'];
    const saveImages = includeImage ? (flags['save-images'] || path.join(DEFAULT_IMAGE_DIR, 'route-render')) : null;
    const result = await callMcp('route_render', {
      include_image: includeImage,
      image_scale: numberFlag(flags, 'image-scale', 2),
      cell_size: numberFlag(flags, 'cell-size', 8),
      detail: flags.full ? 'full' : 'compact',
      image_style: flags.tiles ? 'tiles' : 'schematic',
      format: 'json',
    }, saveImages);
    print(compactRouteRender(result, { grid: !flags['no-grid'] }));
    return;
  }

  if (cmd === 'proof') {
    const label = String(args[0] || 'proof').replace(/[^a-zA-Z0-9._-]+/g, '-');
    const saveImages = path.join(DEFAULT_IMAGE_DIR, label);
    const status = compactStatus(firstObject(await callMcp('status')));
    const observe = compactObserve(await callMcp('observe', {
      include_image: true,
      image_scale: 2,
      advance_frames: 1,
      detail: 'compact',
      format: 'json',
    }, saveImages));
    print({ status, observe, proofImage: observe.imagePaths?.at?.(-1) || null });
    return;
  }

  if (cmd === 'context') {
    print(compactRouteContext(firstObject(await callMcp('status'))));
    return;
  }

  if (cmd === 'move') {
    const direction = args[0];
    if (!/^(up|down|left|right)$/i.test(direction || '')) throw new Error('move requires up|down|left|right');
    const steps = numberFlag(flags, 'steps', 1);
    const before = compactStatus(firstObject(await callMcp('status')));
    const move = await callMcp('move', { direction, steps, times: steps, count: steps, detail: 'compact', format: 'json' });
    const after = compactStatus(firstObject(await callMcp('status')));
    print({
      before,
      after,
      moved: JSON.stringify(before?.coords) !== JSON.stringify(after?.coords) || before?.map !== after?.map,
      moveText: contentTexts(move).map((text) => typeof text === 'string' ? truncate(text, 500) : text),
    });
    return;
  }

  if (cmd === 'press') {
    const button = args[0];
    if (!button) throw new Error('press requires a button');
    const times = numberFlag(flags, 'times', 1);
    const before = compactStatus(firstObject(await callMcp('status')));
    const outputs = [];
    for (let i = 0; i < times; i += 1) outputs.push(await callMcp('press', { button: String(button).toLowerCase(), times: 1, count: 1, detail: 'compact', format: 'json' }));
    const after = compactStatus(firstObject(await callMcp('status')));
    print({ before, after, times, button, moved: JSON.stringify(before?.coords) !== JSON.stringify(after?.coords) || before?.map !== after?.map });
    return;
  }

  if (cmd === 'clear') {
    const max = numberFlag(flags, 'max', 20);
    const before = compactStatus(firstObject(await callMcp('status')));
    const result = await callMcp('execute_macro', { macro: 'advance_dialog', max_presses: max });
    const after = compactStatus(firstObject(await callMcp('status')));
    print({ before, after, max, output: contentTexts(result).map((text) => typeof text === 'string' ? truncate(text, 500) : text) });
    return;
  }

  if (cmd === 'events') {
    const limit = numberFlag(flags, 'limit', 8);
    const result = await callMcp('recent_events', { limit });
    print({ events: firstObject(result) || contentTexts(result) });
    return;
  }

  if (cmd === 'tools') {
    const result = await callMcp('list-tools', null);
    print({ tools: Array.isArray(result) ? result.map((tool) => tool.name) : result });
    return;
  }

  usage();
  throw new Error(`unknown command: ${cmd}`);
}

main().catch((error) => {
  console.error(error?.stack || String(error));
  process.exit(1);
});
