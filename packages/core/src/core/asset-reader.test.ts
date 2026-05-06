describe("asset-reader browser asset paths", () => {
  const originalWindow = global.window;
  const originalXHR = global.XMLHttpRequest;
  const originalFetch = global.fetch;

  afterEach(() => {
    jest.resetModules();
    jest.restoreAllMocks();
    delete (
      globalThis as typeof globalThis & {
        __POKECRYSTAL_ASSET_READER_CACHE__?: unknown;
      }
    ).__POKECRYSTAL_ASSET_READER_CACHE__;
    if (originalWindow === undefined) {
      delete (global as typeof globalThis & { window?: Window }).window;
    } else {
      (global as typeof globalThis & { window?: Window }).window = originalWindow;
    }
    if (originalXHR === undefined) {
      delete (global as typeof globalThis & { XMLHttpRequest?: typeof XMLHttpRequest }).XMLHttpRequest;
    } else {
      (global as typeof globalThis & { XMLHttpRequest?: typeof XMLHttpRequest }).XMLHttpRequest =
        originalXHR;
    }
    if (originalFetch === undefined) {
      delete (global as typeof globalThis & { fetch?: typeof fetch }).fetch;
    } else {
      (global as typeof globalThis & { fetch?: typeof fetch }).fetch = originalFetch;
    }
  });

  it("requests the direct /assets path for sync browser reads", () => {
    (global as typeof globalThis & { window?: Window }).window = {} as Window;
    const requestedTargets: string[] = [];

    class MockXMLHttpRequest {
      status = 0;
      responseText = "";
      private target = "";

      open(_method: string, target: string): void {
        this.target = target;
        requestedTargets.push(target);
      }

      send(): void {
        if (this.target === "/assets/data/permanent_phone_numbers.json") {
          this.status = 200;
          this.responseText = '["PHONE_MOM","PHONE_ELM"]';
          return;
        }
        this.status = 404;
        this.responseText = "";
      }
    }

    (global as typeof globalThis & { XMLHttpRequest?: typeof XMLHttpRequest }).XMLHttpRequest =
      MockXMLHttpRequest as unknown as typeof XMLHttpRequest;

    jest.isolateModules(() => {
      const { readJsonAssetSync } = require("./asset-reader") as typeof import("./asset-reader");
      expect(readJsonAssetSync("/assets/data/permanent_phone_numbers.json")).toEqual([
        "PHONE_MOM",
        "PHONE_ELM",
      ]);
    });

    expect(requestedTargets).toEqual(["/assets/data/permanent_phone_numbers.json"]);
  });

  it("requests the direct /assets path for async browser reads", async () => {
    (global as typeof globalThis & { window?: Window }).window = {} as Window;
    const requestedTargets: string[] = [];
    (global as typeof globalThis & { fetch?: typeof fetch }).fetch = jest.fn(
      async (target: string | URL | Request) => {
        const url = String(target);
        requestedTargets.push(url);
        if (url === "/assets/data/permanent_phone_numbers.json") {
          return {
            ok: true,
            status: 200,
            text: async () => '["PHONE_MOM","PHONE_ELM"]',
          } as Response;
        }
        return {
          ok: false,
          status: 404,
        } as Response;
      },
    );

    let readJsonAsset: typeof import("./asset-reader").readJsonAsset;
    jest.isolateModules(() => {
      ({ readJsonAsset } = require("./asset-reader") as typeof import("./asset-reader"));
    });

    await expect(readJsonAsset!("/assets/data/permanent_phone_numbers.json")).resolves.toEqual([
      "PHONE_MOM",
      "PHONE_ELM",
    ]);
    expect(requestedTargets).toEqual(["/assets/data/permanent_phone_numbers.json"]);
  });

  it("fails immediately for sync browser reads when the canonical /assets path fails", () => {
    (global as typeof globalThis & { window?: Window }).window = {} as Window;
    const requestedTargets: string[] = [];

    class MockXMLHttpRequest {
      status = 0;
      responseText = "";
      private target = "";

      open(_method: string, target: string): void {
        this.target = target;
        requestedTargets.push(target);
      }

      send(): void {
        if (this.target === "/assets/data/permanent_phone_numbers.json") {
          this.status = 404;
          this.responseText = "";
          return;
        }
        this.status = 404;
        this.responseText = "";
      }
    }

    (global as typeof globalThis & { XMLHttpRequest?: typeof XMLHttpRequest }).XMLHttpRequest =
      MockXMLHttpRequest as unknown as typeof XMLHttpRequest;

    jest.isolateModules(() => {
      const { readJsonAssetSync } = require("./asset-reader") as typeof import("./asset-reader");
      expect(() => readJsonAssetSync("/assets/data/permanent_phone_numbers.json")).toThrow(
        "Invalid JSON asset /assets/data/permanent_phone_numbers.json: Failed to load asset /assets/data/permanent_phone_numbers.json (status 404)"
      );
    });

    expect(requestedTargets).toEqual([
      "/assets/data/permanent_phone_numbers.json",
      "/assets/data/permanent_phone_numbers.json",
    ]);
  });

  it("fails immediately for async browser reads when the canonical /assets path fails", async () => {
    (global as typeof globalThis & { window?: Window }).window = {} as Window;
    const requestedTargets: string[] = [];
    (global as typeof globalThis & { fetch?: typeof fetch }).fetch = jest.fn(
      async (target: string | URL | Request) => {
        const url = String(target);
        requestedTargets.push(url);
        if (url === "/assets/data/permanent_phone_numbers.json") {
          return {
            ok: false,
            status: 404,
          } as Response;
        }
        return {
          ok: false,
          status: 404,
        } as Response;
      },
    );

    let readJsonAsset: typeof import("./asset-reader").readJsonAsset;
    jest.isolateModules(() => {
      ({ readJsonAsset } = require("./asset-reader") as typeof import("./asset-reader"));
    });

    await expect(readJsonAsset!("/assets/data/permanent_phone_numbers.json")).rejects.toThrow(
      "Failed to load asset /assets/data/permanent_phone_numbers.json (status 404)"
    );
    expect(requestedTargets).toEqual([
      "/assets/data/permanent_phone_numbers.json",
      "/assets/data/permanent_phone_numbers.json",
    ]);
  });

  it("recovers from stale invalid cached JSON for sync browser reads", () => {
    (global as typeof globalThis & { window?: Window }).window = {} as Window;
    const requestedTargets: string[] = [];
    (
      globalThis as typeof globalThis & {
        __POKECRYSTAL_ASSET_READER_CACHE__?: {
          textAssetCache: Map<string, string>;
          jsonAssetCache: Map<string, unknown>;
          inFlightTextAssetRequests: Map<string, Promise<string>>;
          inFlightJsonAssetRequests: Map<string, Promise<unknown>>;
        };
      }
    ).__POKECRYSTAL_ASSET_READER_CACHE__ = {
      textAssetCache: new Map([
        ["/assets/data/permanent_phone_numbers.json", "<!doctype html>bad cache"],
      ]),
      jsonAssetCache: new Map(),
      inFlightTextAssetRequests: new Map(),
      inFlightJsonAssetRequests: new Map(),
    };

    class MockXMLHttpRequest {
      status = 0;
      responseText = "";
      private target = "";

      open(_method: string, target: string): void {
        this.target = target;
        requestedTargets.push(target);
      }

      send(): void {
        if (this.target === "/assets/data/permanent_phone_numbers.json") {
          this.status = 200;
          this.responseText = '["PHONE_MOM","PHONE_ELM"]';
          return;
        }
        this.status = 404;
        this.responseText = "";
      }
    }

    (global as typeof globalThis & { XMLHttpRequest?: typeof XMLHttpRequest }).XMLHttpRequest =
      MockXMLHttpRequest as unknown as typeof XMLHttpRequest;

    jest.isolateModules(() => {
      const { readJsonAssetSync } = require("./asset-reader") as typeof import("./asset-reader");
      expect(readJsonAssetSync("/assets/data/permanent_phone_numbers.json")).toEqual([
        "PHONE_MOM",
        "PHONE_ELM",
      ]);
    });

    expect(requestedTargets).toEqual(["/assets/data/permanent_phone_numbers.json"]);
  });

  it("recovers from stale invalid cached JSON for async browser reads", async () => {
    (global as typeof globalThis & { window?: Window }).window = {} as Window;
    const requestedTargets: string[] = [];
    (
      globalThis as typeof globalThis & {
        __POKECRYSTAL_ASSET_READER_CACHE__?: {
          textAssetCache: Map<string, string>;
          jsonAssetCache: Map<string, unknown>;
          inFlightTextAssetRequests: Map<string, Promise<string>>;
          inFlightJsonAssetRequests: Map<string, Promise<unknown>>;
        };
      }
    ).__POKECRYSTAL_ASSET_READER_CACHE__ = {
      textAssetCache: new Map([
        ["/assets/data/permanent_phone_numbers.json", "<!doctype html>bad cache"],
      ]),
      jsonAssetCache: new Map(),
      inFlightTextAssetRequests: new Map(),
      inFlightJsonAssetRequests: new Map(),
    };

    (global as typeof globalThis & { fetch?: typeof fetch }).fetch = jest.fn(
      async (target: string | URL | Request) => {
        const url = String(target);
        requestedTargets.push(url);
        if (url === "/assets/data/permanent_phone_numbers.json") {
          return {
            ok: true,
            status: 200,
            text: async () => '["PHONE_MOM","PHONE_ELM"]',
          } as Response;
        }
        return {
          ok: false,
          status: 404,
        } as Response;
      },
    );

    let readJsonAsset: typeof import("./asset-reader").readJsonAsset;
    jest.isolateModules(() => {
      ({ readJsonAsset } = require("./asset-reader") as typeof import("./asset-reader"));
    });

    await expect(readJsonAsset!("/assets/data/permanent_phone_numbers.json")).resolves.toEqual([
      "PHONE_MOM",
      "PHONE_ELM",
    ]);
    expect(requestedTargets).toEqual(["/assets/data/permanent_phone_numbers.json"]);
  });

  it("rewrites bundled disassembly gfx paths to /assets for sync browser reads", () => {
    (global as typeof globalThis & { window?: Window }).window = {} as Window;
    const requestedTargets: string[] = [];

    class MockXMLHttpRequest {
      status = 0;
      responseText = "";

      open(_method: string, target: string): void {
        requestedTargets.push(target);
      }

      send(): void {
        this.status = 200;
        this.responseText = "; morn\nRGB 31, 31, 31, 21, 21, 21, 10, 10, 10, 0, 0, 0\n";
      }
    }

    (global as typeof globalThis & { XMLHttpRequest?: typeof XMLHttpRequest }).XMLHttpRequest =
      MockXMLHttpRequest as unknown as typeof XMLHttpRequest;

    jest.isolateModules(() => {
      const { readTextAssetSync } = require("./asset-reader") as typeof import("./asset-reader");
      expect(readTextAssetSync("/disassembly/gfx/overworld/npc_sprites.pal")).toContain("; morn");
    });

    expect(requestedTargets).toEqual(["/assets/gfx/overworld/npc_sprites.pal"]);
  });

  it("rewrites bundled disassembly gfx paths to /assets for async browser reads", async () => {
    (global as typeof globalThis & { window?: Window }).window = {} as Window;
    const requestedTargets: string[] = [];
    (global as typeof globalThis & { fetch?: typeof fetch }).fetch = jest.fn(
      async (target: string | URL | Request) => {
        const url = String(target);
        requestedTargets.push(url);
        return {
          ok: true,
          status: 200,
          text: async () => "; morn\nRGB 31, 31, 31, 21, 21, 21, 10, 10, 10, 0, 0, 0\n",
        } as Response;
      },
    );

    let readTextAsset: typeof import("./asset-reader").readTextAsset;
    jest.isolateModules(() => {
      ({ readTextAsset } = require("./asset-reader") as typeof import("./asset-reader"));
    });

    await expect(readTextAsset!("/disassembly/gfx/overworld/npc_sprites.pal")).resolves.toContain("; morn");
    expect(requestedTargets).toEqual(["/assets/gfx/overworld/npc_sprites.pal"]);
  });
});
