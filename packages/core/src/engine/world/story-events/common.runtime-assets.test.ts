describe("story-events common asset runtime", () => {
  afterEach(() => {
    delete (globalThis as typeof globalThis & {
      __POKECRYSTAL_STORY_EVENT_RUNTIME_CACHE__?: unknown;
      __POKECRYSTAL_PHONE_CONTACT_RUNTIME_CACHE__?: unknown;
    }).__POKECRYSTAL_STORY_EVENT_RUNTIME_CACHE__;
    delete (globalThis as typeof globalThis & {
      __POKECRYSTAL_STORY_EVENT_RUNTIME_CACHE__?: unknown;
      __POKECRYSTAL_PHONE_CONTACT_RUNTIME_CACHE__?: unknown;
    }).__POKECRYSTAL_PHONE_CONTACT_RUNTIME_CACHE__;
    jest.resetModules();
    jest.restoreAllMocks();
  });

  it("loads permanent phone numbers and initialize events config from bundled assets", () => {
    jest.doMock("@pokecrystal/core/core/asset-reader", () => ({
      readJsonAssetSync: (target: string) => {
        if (target.endsWith("permanent_phone_numbers.json")) {
          return ["PHONE_MOM"];
        }
        if (target.endsWith("initialize_events.json")) {
          return {
            eventFlags: ["EVENT_TEST"],
            engineFlags: ["ENGINE_TEST"],
            variableSprites: {
              SPRITE_FUCHSIA_GYM_1: "SPRITE_ROCKER",
            },
          };
        }
        throw new Error(`unexpected asset read: ${target}`);
      },
    }));
    jest.doMock("@pokecrystal/core/core/paths", () => ({
      getDataDir: () => "/tmp/assets/data",
    }));
    jest.doMock("@pokecrystal/core/ui/menus/pokegear-contacts", () => ({
      loadPhoneContactDirectory: () => ({
        resolveContactId: (token: string) => (token === "PHONE_MOM" ? "PHONE_MOM" : null),
      }),
    }));

    jest.isolateModules(() => {
      const { loadPermanentPhoneNumbers, loadInitializeEventsConfig } =
        require("./common") as typeof import("./common");

      expect(loadPermanentPhoneNumbers()).toEqual(["PHONE_MOM"]);
      expect(loadInitializeEventsConfig()).toEqual({
        eventFlags: ["EVENT_TEST"],
        engineFlags: ["ENGINE_TEST"],
        variableSprites: {
          SPRITE_FUCHSIA_GYM_1: "SPRITE_ROCKER",
        },
      });
    });
  });

  it("throws when bundled permanent phone numbers are missing", () => {
    jest.doMock("@pokecrystal/core/core/asset-reader", () => ({
      readJsonAssetSync: (target: string) => {
        if (target.endsWith("permanent_phone_numbers.json")) {
          throw new Error("missing");
        }
        return {
          eventFlags: ["EVENT_TEST"],
          engineFlags: [],
          variableSprites: {},
        };
      },
    }));
    jest.doMock("@pokecrystal/core/core/paths", () => ({
      getDataDir: () => "/tmp/assets/data",
    }));
    jest.doMock("@pokecrystal/core/ui/menus/pokegear-contacts", () => ({
      loadPhoneContactDirectory: () => ({
        resolveContactId: () => null,
      }),
    }));

    jest.isolateModules(() => {
      const { loadPermanentPhoneNumbers } = require("./common") as typeof import("./common");

      expect(() => loadPermanentPhoneNumbers()).toThrow(
        "Permanent phone numbers is required for the asset-only runtime: missing or invalid /tmp/assets/data/permanent_phone_numbers.json."
      );
    });
  });

  it("throws when bundled initialize events config is missing", () => {
    jest.doMock("@pokecrystal/core/core/asset-reader", () => ({
      readJsonAssetSync: (target: string) => {
        if (target.endsWith("initialize_events.json")) {
          throw new Error("missing");
        }
        return ["PHONE_MOM"];
      },
    }));
    jest.doMock("@pokecrystal/core/core/paths", () => ({
      getDataDir: () => "/tmp/assets/data",
    }));
    jest.doMock("@pokecrystal/core/ui/menus/pokegear-contacts", () => ({
      loadPhoneContactDirectory: () => ({
        resolveContactId: (token: string) => token,
      }),
    }));

    jest.isolateModules(() => {
      const { loadInitializeEventsConfig } = require("./common") as typeof import("./common");

      expect(() => loadInitializeEventsConfig()).toThrow(
        "Initialize events config is required for the asset-only runtime: missing or invalid /tmp/assets/data/initialize_events.json."
      );
    });
  });

  it("primes story-event runtime assets so later sync reads are not needed", async () => {
    jest.doMock("@pokecrystal/core/core/asset-reader", () => ({
      readJsonAsset: async (target: string) => {
        if (target.endsWith("permanent_phone_numbers.json")) {
          return ["PHONE_MOM"];
        }
        if (target.endsWith("initialize_events.json")) {
          return {
            eventFlags: ["EVENT_TEST"],
            engineFlags: ["ENGINE_TEST"],
            variableSprites: {
              SPRITE_FUCHSIA_GYM_1: "SPRITE_ROCKER",
            },
          };
        }
        throw new Error(`unexpected async asset read: ${target}`);
      },
      readJsonAssetSync: () => {
        throw new Error("sync reads should not be needed after priming");
      },
    }));
    jest.doMock("@pokecrystal/core/core/paths", () => ({
      getDataDir: () => "/tmp/assets/data",
    }));
    jest.doMock("@pokecrystal/core/ui/menus/pokegear-contacts", () => ({
      loadPhoneContactDirectory: () => ({
        resolveContactId: (token: string) => (token === "PHONE_MOM" ? "PHONE_MOM" : null),
      }),
      primePhoneContactDirectory: async () => ({
        resolveContactId: (token: string) => (token === "PHONE_MOM" ? "PHONE_MOM" : null),
      }),
    }));

    let commonModule: typeof import("./common");
    jest.isolateModules(() => {
      commonModule = require("./common") as typeof import("./common");
    });

    await commonModule!.primeStoryEventRuntimeAssets();

    expect(commonModule!.loadPermanentPhoneNumbers()).toEqual(["PHONE_MOM"]);
    expect(commonModule!.loadInitializeEventsConfig()).toEqual({
      eventFlags: ["EVENT_TEST"],
      engineFlags: ["ENGINE_TEST"],
      variableSprites: {
        SPRITE_FUCHSIA_GYM_1: "SPRITE_ROCKER",
      },
    });
  });

  it("keeps primed story-event assets across module reloads", async () => {
    jest.doMock("@pokecrystal/core/core/asset-reader", () => ({
      readJsonAsset: async (target: string) => {
        if (target.endsWith("permanent_phone_numbers.json")) {
          return ["PHONE_MOM"];
        }
        if (target.endsWith("initialize_events.json")) {
          return {
            eventFlags: ["EVENT_TEST"],
            engineFlags: ["ENGINE_TEST"],
            variableSprites: {
              SPRITE_FUCHSIA_GYM_1: "SPRITE_ROCKER",
            },
          };
        }
        throw new Error(`unexpected async asset read: ${target}`);
      },
      readJsonAssetSync: () => {
        throw new Error("sync reads should not be needed after priming");
      },
    }));
    jest.doMock("@pokecrystal/core/core/paths", () => ({
      getDataDir: () => "/tmp/assets/data",
    }));
    jest.doMock("@pokecrystal/core/ui/menus/pokegear-contacts", () => ({
      loadPhoneContactDirectory: () => ({
        resolveContactId: (token: string) => (token === "PHONE_MOM" ? "PHONE_MOM" : null),
      }),
      primePhoneContactDirectory: async () => ({
        resolveContactId: (token: string) => (token === "PHONE_MOM" ? "PHONE_MOM" : null),
      }),
    }));

    let firstModule: typeof import("./common");
    jest.isolateModules(() => {
      firstModule = require("./common") as typeof import("./common");
    });

    await firstModule!.primeStoryEventRuntimeAssets();

    jest.resetModules();

    let reloadedModule: typeof import("./common");
    jest.isolateModules(() => {
      reloadedModule = require("./common") as typeof import("./common");
    });

    expect(reloadedModule!.loadPermanentPhoneNumbers()).toEqual(["PHONE_MOM"]);
    expect(reloadedModule!.loadInitializeEventsConfig()).toEqual({
      eventFlags: ["EVENT_TEST"],
      engineFlags: ["ENGINE_TEST"],
      variableSprites: {
        SPRITE_FUCHSIA_GYM_1: "SPRITE_ROCKER",
      },
    });
  });
});
