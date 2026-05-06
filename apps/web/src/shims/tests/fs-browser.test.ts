import path from "path";

type FsBrowserModule = {
  existsSync: (filePath: string) => boolean;
  readdirSync: (dirPath: string) => string[];
  readFileSync: (filePath: string, encoding?: BufferEncoding) => string | Buffer;
  promises: {
    access: (filePath: string) => Promise<void>;
  };
};

type FakeXHRClass = {
  new (): FakeXHRInstance;
  lastUrl: string | null;
};

type FakeXHRInstance = {
  status: number;
  responseText: string;
  response: ArrayBuffer | null;
  open: (method: string, url: string, async: boolean) => void;
  send: () => void;
  overrideMimeType: () => void;
};

type GlobalWithFakeXHR = Omit<typeof globalThis, "XMLHttpRequest"> & {
  XMLHttpRequest?: typeof globalThis.XMLHttpRequest | undefined;
};

const setGlobalXMLHttpRequest = (value: FakeXHRClass | undefined): void => {
  const target = globalThis as GlobalWithFakeXHR;
  if (value === undefined) {
    delete target.XMLHttpRequest;
    return;
  }
  target.XMLHttpRequest = value as unknown as typeof globalThis.XMLHttpRequest;
};

const loadFsBrowser = (env?: {
  assetBase?: string;
  disassemblyBase?: string;
}): FsBrowserModule => {
  jest.resetModules();
  const prevAsset = process.env.NEXT_PUBLIC_ASSET_BASE;
  const prevDisasm = process.env.NEXT_PUBLIC_DISASSEMBLY_BASE;
  if (env?.assetBase === undefined) {
    delete process.env.NEXT_PUBLIC_ASSET_BASE;
  } else {
    process.env.NEXT_PUBLIC_ASSET_BASE = env.assetBase;
  }
  if (env?.disassemblyBase === undefined) {
    delete process.env.NEXT_PUBLIC_DISASSEMBLY_BASE;
  } else {
    process.env.NEXT_PUBLIC_DISASSEMBLY_BASE = env.disassemblyBase;
  }

  let fsBrowser: FsBrowserModule | null = null;
  jest.isolateModules(() => {
    fsBrowser = require("../fs-browser") as FsBrowserModule;
  });

  if (prevAsset === undefined) {
    delete process.env.NEXT_PUBLIC_ASSET_BASE;
  } else {
    process.env.NEXT_PUBLIC_ASSET_BASE = prevAsset;
  }
  if (prevDisasm === undefined) {
    delete process.env.NEXT_PUBLIC_DISASSEMBLY_BASE;
  } else {
    process.env.NEXT_PUBLIC_DISASSEMBLY_BASE = prevDisasm;
  }

  if (!fsBrowser) {
    throw new Error("fs-browser module failed to load");
  }
  return fsBrowser;
};

const installXHR = (responseText: string): FakeXHRClass => {
  class FakeXHR implements FakeXHRInstance {
    static readonly UNSENT = 0;
    static readonly OPENED = 1;
    static readonly HEADERS_RECEIVED = 2;
    static readonly LOADING = 3;
    static readonly DONE = 4;
    static lastUrl: string | null = null;
    status = 0;
    responseText = "";
    response: ArrayBuffer | null = null;

    open(method: string, url: string, _async: boolean): void {
      FakeXHR.lastUrl = url;
    }

    send(): void {
      this.status = 200;
      this.responseText = responseText;
    }

    overrideMimeType(): void {}
  }

  setGlobalXMLHttpRequest(FakeXHR);
  return FakeXHR;
};

const installAssetXHR = (): FakeXHRClass => {
  class FakeXHR implements FakeXHRInstance {
    static readonly UNSENT = 0;
    static readonly OPENED = 1;
    static readonly HEADERS_RECEIVED = 2;
    static readonly LOADING = 3;
    static readonly DONE = 4;
    static lastUrl: string | null = null;
    status = 0;
    responseText = "";
    response: ArrayBuffer | null = null;
    private method = "GET";
    private url = "";

    open(method: string, url: string, _async: boolean): void {
      this.method = method;
      this.url = url;
      FakeXHR.lastUrl = url;
    }

    send(): void {
      if (this.url === "/assets/gfx/battle_anims/smoke.2bpp") {
        this.status = 200;
        this.responseText = this.method === "HEAD" ? "" : "tiledata";
        return;
      }
      this.status = 404;
      this.responseText = "";
    }

    overrideMimeType(): void {}
  }

  setGlobalXMLHttpRequest(FakeXHR);
  return FakeXHR;
};

afterEach(() => {
    setGlobalXMLHttpRequest(undefined);
  });

describe("fs-browser", () => {
  test("existsSync uses the manifest when XMLHttpRequest is unavailable", () => {
    setGlobalXMLHttpRequest(undefined);
    const fsBrowser = loadFsBrowser();
    expect(fsBrowser.existsSync("/assets/gfx/debug/color_test.png")).toBe(true);
  });

  test("readdirSync normalizes trailing slashes", () => {
    const fsBrowser = loadFsBrowser();
    const entries = fsBrowser.readdirSync("/assets/gfx/debug/");
    expect(entries).toContain("color_test.png");
  });

  test("readFileSync respects NEXT_PUBLIC_ASSET_BASE", () => {
    const FakeXHR = installXHR("asset");
    const fsBrowser = loadFsBrowser({
      assetBase: "/static",
      disassemblyBase: "/disassembly",
    });
    const data = fsBrowser.readFileSync(
      "/static/gfx/debug/color_test.png",
      "utf-8"
    );
    expect(FakeXHR.lastUrl).toBe("/static/gfx/debug/color_test.png");
    expect(data).toBe("asset");
  });

  test("existsSync does not expose disassembly files from the browser manifest", () => {
    const fsBrowser = loadFsBrowser({
      assetBase: "/assets",
      disassemblyBase: "/disasm",
    });
    expect(
      fsBrowser.existsSync("/pokecrystal_disassembly/engine/events/specials.asm")
    ).toBe(false);
  });

  test("promises.access resolves for manifest-backed files", async () => {
    const fsBrowser = loadFsBrowser();
    await expect(fsBrowser.promises.access("/assets/gfx/debug/color_test.png")).resolves.toBeUndefined();
  });

  test("battle animation smoke tiles are manifest-backed for browser reads", () => {
    const FakeXHR = installAssetXHR();
    const fsBrowser = loadFsBrowser();
    expect(fsBrowser.existsSync("/assets/gfx/battle_anims/smoke.2bpp")).toBe(true);
    expect(fsBrowser.readFileSync("/assets/gfx/battle_anims/smoke.2bpp", "utf-8")).toBe("tiledata");
    expect(FakeXHR.lastUrl).toBe("/assets/gfx/battle_anims/smoke.2bpp");
  });

  test("battle animation smoke tiles use a single canonical /assets read path", () => {
    const FakeXHR = installAssetXHR();
    const fsBrowser = loadFsBrowser();
    expect(fsBrowser.readFileSync("/assets/gfx/battle_anims/smoke.2bpp", "utf-8")).toBe("tiledata");
    expect(FakeXHR.lastUrl).toBe("/assets/gfx/battle_anims/smoke.2bpp");
  });

  test("absolute legacy smoke tile paths normalize to the canonical /assets read path", () => {
    const FakeXHR = installAssetXHR();
    const fsBrowser = loadFsBrowser();
    const absoluteLegacyPath = path.resolve(
      process.cwd(),
      "assets",
      "gfx",
      "battle_anims",
      "smoke.2bpp.lz",
    );

    expect(fsBrowser.existsSync(absoluteLegacyPath)).toBe(true);
    expect(fsBrowser.readFileSync(absoluteLegacyPath, "utf-8")).toBe("tiledata");
    expect(FakeXHR.lastUrl).toBe("/assets/gfx/battle_anims/smoke.2bpp");
  });

  test("bundled disassembly gfx palette paths normalize to the canonical /assets read path", () => {
    class FakeXHR implements FakeXHRInstance {
      static readonly UNSENT = 0;
      static readonly OPENED = 1;
      static readonly HEADERS_RECEIVED = 2;
      static readonly LOADING = 3;
      static readonly DONE = 4;
      static lastUrl: string | null = null;
      status = 0;
      responseText = "";
      response: ArrayBuffer | null = null;
      private method = "GET";
      private url = "";

      open(method: string, url: string, _async: boolean): void {
        this.method = method;
        this.url = url;
        FakeXHR.lastUrl = url;
      }

      send(): void {
        if (this.url === "/assets/gfx/overworld/npc_sprites.pal") {
          this.status = 200;
          this.responseText = this.method === "HEAD" ? "" : "; morn\n";
          return;
        }
        this.status = 404;
        this.responseText = "";
      }

      overrideMimeType(): void {}
    }

    setGlobalXMLHttpRequest(FakeXHR as unknown as FakeXHRClass);
    const fsBrowser = loadFsBrowser();
    const disassemblyPalettePath = "/disassembly/gfx/overworld/npc_sprites.pal";

    expect(fsBrowser.existsSync(disassemblyPalettePath)).toBe(true);
    expect(fsBrowser.readFileSync(disassemblyPalettePath, "utf-8")).toBe("; morn\n");
    expect(FakeXHR.lastUrl).toBe("/assets/gfx/overworld/npc_sprites.pal");
  });

  test("promises.access rejects for missing files", async () => {
    const fsBrowser = loadFsBrowser();
    await expect(fsBrowser.promises.access("/assets/does-not-exist.bin")).rejects.toMatchObject({
      code: "ENOENT",
    });
  });
});
