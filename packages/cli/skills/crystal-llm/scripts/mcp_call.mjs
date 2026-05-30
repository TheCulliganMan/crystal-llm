#!/usr/bin/env node
import fs from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

const DEFAULT_REPO = '$POKECRYSTAL_REPO';
const DEFAULT_ENDPOINT = 'http://127.0.0.1:3003/api/mcp?session_id=codex-service';
const RETRYABLE_NEXT_MANIFEST_ERROR =
  /Unexpected end of JSON input|loadManifest|manifest|Internal Server Error|Error POSTing to endpoint/i;
const RETRY_DELAYS_MS = [250, 750, 1500];

function usage() {
  console.error(`Usage:
  mcp_call.mjs <tool> [--args '<json>'] [--save-images <dir>] [--endpoint <url>]
  mcp_call.mjs list-tools [--endpoint <url>]

Examples:
  mcp_call.mjs status
  mcp_call.mjs move --args '{"direction":"left","times":1,"steps":1,"count":1,"format":"json","detail":"compact"}'
  mcp_call.mjs observe --args '{"include_image":true,"image_scale":2,"advance_frames":1,"detail":"compact","format":"json"}' --save-images /tmp/poke-images`);
}

export function parseArgs(argv) {
  const opts = {
    tool: null,
    args: {},
    endpoint: process.env.POKECRYSTAL_MCP_ENDPOINT || DEFAULT_ENDPOINT,
    saveImages: null,
    repo: process.env.POKECRYSTAL_REPO || DEFAULT_REPO,
  };

  for (let i = 0; i < argv.length; i += 1) {
    const value = argv[i];
    if (value === '--help' || value === '-h') {
      usage();
      process.exit(0);
    }
    if (value === '--args') {
      const raw = argv[++i];
      if (!raw) throw new Error('--args requires JSON');
      opts.args = JSON.parse(raw);
      continue;
    }
    if (value === '--endpoint') {
      opts.endpoint = argv[++i];
      if (!opts.endpoint) throw new Error('--endpoint requires a URL');
      continue;
    }
    if (value === '--save-images') {
      opts.saveImages = argv[++i];
      if (!opts.saveImages) throw new Error('--save-images requires a directory');
      continue;
    }
    if (value === '--repo') {
      opts.repo = argv[++i];
      if (!opts.repo) throw new Error('--repo requires a path');
      continue;
    }
    if (!opts.tool) {
      opts.tool = value;
      continue;
    }
    throw new Error(`unexpected argument: ${value}`);
  }

  if (!opts.tool) throw new Error('missing tool name');
  return opts;
}

async function importSdk(repo) {
  const clientPath = path.join(repo, 'node_modules/@modelcontextprotocol/sdk/dist/esm/client/index.js');
  const transportPath = path.join(repo, 'node_modules/@modelcontextprotocol/sdk/dist/esm/client/streamableHttp.js');
  const [{ Client }, { StreamableHTTPClientTransport }] = await Promise.all([
    import(pathToFileURL(clientPath).href),
    import(pathToFileURL(transportPath).href),
  ]);
  return { Client, StreamableHTTPClientTransport };
}

function parseJsonMaybe(text) {
  try {
    return JSON.parse(text);
  } catch {
    return text;
  }
}

function writeImage(content, dir, index) {
  fs.mkdirSync(dir, { recursive: true });
  const file = path.join(dir, `mcp-image-${Date.now()}-${index}.png`);
  fs.writeFileSync(file, Buffer.from(content.data, 'base64'));
  return file;
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function isRetryableNextManifestError(error) {
  const text = `${error?.stack || ''}\n${error?.message || ''}`;
  return RETRYABLE_NEXT_MANIFEST_ERROR.test(text);
}

async function withRetry(operation) {
  let lastError;
  for (let attempt = 0; attempt <= RETRY_DELAYS_MS.length; attempt += 1) {
    try {
      return await operation();
    } catch (error) {
      lastError = error;
      if (attempt >= RETRY_DELAYS_MS.length || !isRetryableNextManifestError(error)) {
        throw error;
      }
      const delay = RETRY_DELAYS_MS[attempt];
      console.error(`[mcp_call] transient Next manifest error; retrying in ${delay}ms`);
      await sleep(delay);
    }
  }
  throw lastError;
}

async function callToolOnce(opts) {
  const { Client, StreamableHTTPClientTransport } = await importSdk(opts.repo);
  const client = new Client({ name: 'pokecrystal-direct-http', version: '1.0.0' });
  const transport = new StreamableHTTPClientTransport(new URL(opts.endpoint), {
    requestInit: { headers: { accept: 'application/json, text/event-stream' } },
  });

  await client.connect(transport);
  try {
    if (opts.tool === 'list-tools') {
      const tools = await client.listTools();
      return tools.tools.map((tool) => ({
        name: tool.name,
        description: tool.description,
        inputSchema: tool.inputSchema,
      }));
    }

    return await client.callTool({ name: opts.tool, arguments: opts.args });
  } finally {
    await transport.close();
    await client.close();
  }
}

export async function callMcpTool(tool, args = {}, options = {}) {
  const endpoint = options.endpoint || process.env.POKECRYSTAL_MCP_ENDPOINT || DEFAULT_ENDPOINT;
  const repo = options.repo || process.env.POKECRYSTAL_REPO || DEFAULT_REPO;
  const saveImages = options.saveImages || null;
  const result = await withRetry(() => callToolOnce({ tool, args, endpoint, repo, saveImages }));

  if (tool === 'list-tools') {
    return result;
  }

  const imagePaths = [];
  const content = [];
  let imageIndex = 0;

  for (const item of result.content || []) {
    if (item.type === 'image') {
      if (saveImages) imagePaths.push(writeImage(item, saveImages, ++imageIndex));
      content.push({ type: 'image', mimeType: item.mimeType, savedPath: imagePaths.at(-1) || null });
    } else if (item.type === 'text') {
      content.push({ type: 'text', text: parseJsonMaybe(item.text) });
    } else {
      content.push(item);
    }
  }

  return {
    endpoint,
    tool,
    args,
    content,
    imagePaths,
    snapshot: result.snapshot || null,
  };
}

async function main() {
  const opts = parseArgs(process.argv.slice(2));
  const result = await callMcpTool(opts.tool, opts.args, {
    endpoint: opts.endpoint,
    repo: opts.repo,
    saveImages: opts.saveImages,
  });
  console.log(JSON.stringify(result, null, 2));
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  main().catch((error) => {
    console.error(error?.stack || String(error));
    process.exit(1);
  });
}
