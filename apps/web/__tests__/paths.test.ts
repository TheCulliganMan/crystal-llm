import fs from 'fs';
import os from 'os';
import path from 'path';

const writeSentinel = (filePath: string) => {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, '{}');
};

const canonicalPath = (filePath: string) => {
  const realpathSync = fs.realpathSync.native ?? fs.realpathSync;
  return realpathSync(filePath);
};

describe('paths asset resolution', () => {
  const originalCwd = process.cwd();
  const originalDisassemblyRootEnv = process.env.POKECRYSTAL_DISASSEMBLY_ROOT;

  beforeEach(() => {
    delete process.env.POKECRYSTAL_DISASSEMBLY_ROOT;
  });

  afterEach(() => {
    process.chdir(originalCwd);
    if (typeof originalDisassemblyRootEnv === 'string') {
      process.env.POKECRYSTAL_DISASSEMBLY_ROOT = originalDisassemblyRootEnv;
    } else {
      delete process.env.POKECRYSTAL_DISASSEMBLY_ROOT;
    }
    jest.resetModules();
  });

  it('resolves the workspace asset tree when it is complete', async () => {
    const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'pokecrystal-paths-'));
    const workspaceRoot = path.join(tempRoot, 'apps', 'web');
    writeSentinel(
      path.join(workspaceRoot, 'assets', 'data', 'map_attributes.json')
    );
    writeSentinel(
      path.join(workspaceRoot, 'assets', 'gfx', 'tilesets', 'bg_tiles.pal')
    );

    process.chdir(tempRoot);
    jest.resetModules();

    const { getAssetsRoot } = await import('@pokecrystal/core/core/paths');
    const { normalizePath } = await import('@pokecrystal/core/core/path-utils');

    const assetsRoot = getAssetsRoot();

    expect(assetsRoot).toBe(
      normalizePath(canonicalPath(path.join(workspaceRoot, 'assets')))
    );
  });

  it('falls back to workspace assets when discovered from the cwd root', async () => {
    const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'pokecrystal-paths-'));
    const workspaceRoot = path.join(tempRoot, 'apps', 'web');
    writeSentinel(
      path.join(workspaceRoot, 'assets', 'data', 'map_attributes.json')
    );
    writeSentinel(
      path.join(workspaceRoot, 'assets', 'gfx', 'tilesets', 'bg_tiles.pal')
    );

    process.chdir(workspaceRoot);
    jest.resetModules();

    const { getAssetsRoot } = await import('@pokecrystal/core/core/paths');
    const { normalizePath } = await import('@pokecrystal/core/core/path-utils');

    const assetsRoot = getAssetsRoot();

    expect(assetsRoot).toBe(
      normalizePath(canonicalPath(path.join(workspaceRoot, 'assets')))
    );
  });

  it('ignores partial workspace assets that are missing required graphics sentinels', async () => {
    const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'pokecrystal-paths-'));
    const workspaceRoot = path.join(tempRoot, 'apps', 'web');
    writeSentinel(
      path.join(workspaceRoot, 'assets', 'data', 'map_attributes.json')
    );

    process.chdir(workspaceRoot);
    jest.resetModules();

    const { getAssetsRoot } = await import('@pokecrystal/core/core/paths');
    const { normalizePath } = await import('@pokecrystal/core/core/path-utils');

    const assetsRoot = getAssetsRoot();

    expect(assetsRoot).toBe(
      normalizePath(canonicalPath(path.join(workspaceRoot, 'assets')))
    );
  });

  it('resolves the repo disassembly checkout when present', async () => {
    const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'pokecrystal-paths-'));
    const workspaceRoot = path.join(tempRoot, 'apps', 'web');
    fs.mkdirSync(workspaceRoot, { recursive: true });
    writeSentinel(
      path.join(tempRoot, 'pokecrystal_disassembly', 'engine', 'events', 'specials.asm')
    );

    process.chdir(workspaceRoot);
    jest.resetModules();

    const { getDisassemblyRoot } = await import('@pokecrystal/core/core/paths');
    const { normalizePath } = await import('@pokecrystal/core/core/path-utils');

    const disassemblyRoot = getDisassemblyRoot();

    expect(disassemblyRoot).toBe(
      normalizePath(canonicalPath(path.join(tempRoot, 'pokecrystal_disassembly')))
    );
  });

  it('recovers disassembly root from cwd traversal', async () => {
    const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'pokecrystal-paths-'));
    const workspaceRoot = path.join(tempRoot, 'apps', 'web');
    fs.mkdirSync(workspaceRoot, { recursive: true });
    writeSentinel(
      path.join(tempRoot, 'pokecrystal_disassembly', 'engine', 'events', 'specials.asm')
    );

    process.chdir(workspaceRoot);
    jest.resetModules();

    const { getDisassemblyRoot } = await import('@pokecrystal/core/core/paths');
    const { normalizePath } = await import('@pokecrystal/core/core/path-utils');

    const disassemblyRoot = getDisassemblyRoot();

    expect(disassemblyRoot).toBe(
      normalizePath(canonicalPath(path.join(tempRoot, 'pokecrystal_disassembly')))
    );
  });
});
