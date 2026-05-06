import path from "node:path";
import fs from "node:fs";
import os from "node:os";

type AssetModules = {
  assetExists: (filePath: string) => boolean;
  listAssetDir: (dirPath: string) => string[];
  toPublicAssetUrl: (filePath: string) => string;
  getAssetPath: (...parts: string[]) => string;
  getDisassemblyRoot: () => string;
};

const loadModules = (cwd: string, env?: { disassemblyRoot?: string }): AssetModules => {
  jest.resetModules();
  const cwdSpy = jest.spyOn(process, "cwd").mockReturnValue(cwd);
  const previousDisassemblyRoot = process.env.POKECRYSTAL_DISASSEMBLY_ROOT;
  if (env?.disassemblyRoot) {
    process.env.POKECRYSTAL_DISASSEMBLY_ROOT = env.disassemblyRoot;
  } else {
    delete process.env.POKECRYSTAL_DISASSEMBLY_ROOT;
  }
  let modules: AssetModules | null = null;
  jest.isolateModules(() => {
    const assetManifest = require("../asset-manifest") as Omit<
      AssetModules,
      "getAssetPath" | "getDisassemblyRoot"
    >;
    const paths = require("../paths") as Pick<
      AssetModules,
      "getAssetPath" | "getDisassemblyRoot"
    >;
    modules = { ...assetManifest, ...paths } as AssetModules;
  });
  cwdSpy.mockRestore();
  if (previousDisassemblyRoot === undefined) {
    delete process.env.POKECRYSTAL_DISASSEMBLY_ROOT;
  } else {
    process.env.POKECRYSTAL_DISASSEMBLY_ROOT = previousDisassemblyRoot;
  }
  if (!modules) {
    throw new Error("Asset modules failed to load.");
  }
  return modules;
};

describe("asset-manifest", () => {
  const repoRoot = path.resolve(__dirname, "../../../../../");

  test("resolves absolute asset paths built from getAssetPath", () => {
    const { assetExists, getAssetPath } = loadModules(repoRoot);
    const assetPath = getAssetPath("gfx", "debug", "color_test.png");
    expect(assetExists(assetPath)).toBe(true);
  });

  test("resolves absolute disassembly paths built from getDisassemblyRoot in Node", () => {
    const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), "pokecrystal-disassembly-"));
    const disassemblyRoot = path.join(tempRoot, "pokecrystal_disassembly");
    const disassemblyPath = path.join(disassemblyRoot, "engine", "events", "specials.asm");
    fs.mkdirSync(path.dirname(disassemblyPath), { recursive: true });
    fs.writeFileSync(disassemblyPath, "Specials:\n");

    const { assetExists, getDisassemblyRoot } = loadModules(repoRoot, { disassemblyRoot });

    expect(getDisassemblyRoot()).toBe(disassemblyRoot);
    expect(assetExists(path.join(getDisassemblyRoot(), "engine", "events", "specials.asm"))).toBe(true);
  });

  test("listAssetDir normalizes trailing slashes", () => {
    const { listAssetDir } = loadModules(repoRoot);
    const entries = listAssetDir("/assets/gfx/debug/");
    expect(entries).toContain("color_test.png");
  });

  test("maps absolute asset paths back to public asset urls", () => {
    const { getAssetPath, toPublicAssetUrl } = loadModules(repoRoot);
    const assetPath = getAssetPath("gfx", "pokemon", "cyndaquil", "front.png");
    expect(toPublicAssetUrl(assetPath)).toBe("/assets/gfx/pokemon/cyndaquil/front.png");
  });

  test("does not publish disassembly paths as public urls", () => {
    const { getDisassemblyRoot, toPublicAssetUrl } = loadModules(repoRoot);
    const disassemblyPath = path.join(
      getDisassemblyRoot(),
      "engine",
      "events",
      "specials.asm"
    );
    expect(toPublicAssetUrl(disassemblyPath)).toBe(disassemblyPath);
  });

  test("rewrites browser disassembly gfx urls to the bundled /assets path", () => {
    const { toPublicAssetUrl, assetExists } = loadModules(repoRoot);
    expect(toPublicAssetUrl("/disassembly/gfx/overworld/npc_sprites.pal")).toBe(
      "/assets/gfx/overworld/npc_sprites.pal"
    );
    expect(assetExists("/disassembly/gfx/overworld/npc_sprites.pal")).toBe(true);
  });

  test("assetExists does not probe the browser for manifest-managed asset misses", () => {
    const { assetExists } = loadModules(repoRoot);
    const globalAny = globalThis as {
      window?: unknown;
      XMLHttpRequest?: typeof XMLHttpRequest;
    };
    const originalWindow = globalAny.window;
    const originalXMLHttpRequest = globalAny.XMLHttpRequest;

    class FakeXHR {
      static lastUrl: string | null = null;
      public status = 200;
      public responseText = "";
      public response: ArrayBuffer | null = null;

      open(_method: string, url: string, _async: boolean): void {
        FakeXHR.lastUrl = url;
      }

      send(): void {}

      overrideMimeType(): void {}
    }

    Object.defineProperty(globalAny, "window", {
      value: {},
      configurable: true,
      writable: true,
    });
    Object.defineProperty(globalAny, "XMLHttpRequest", {
      value: FakeXHR,
      configurable: true,
      writable: true,
    });

    expect(assetExists("/assets/gfx/pokemon/cyndaquil/front-preview.png")).toBe(false);
    expect(FakeXHR.lastUrl).toBeNull();

    if (originalWindow === undefined) {
      delete globalAny.window;
    } else {
      Object.defineProperty(globalAny, "window", {
        value: originalWindow,
        configurable: true,
        writable: true,
      });
    }
    if (originalXMLHttpRequest === undefined) {
      delete globalAny.XMLHttpRequest;
    } else {
      Object.defineProperty(globalAny, "XMLHttpRequest", {
        value: originalXMLHttpRequest,
        configurable: true,
        writable: true,
      });
    }
  });
});
