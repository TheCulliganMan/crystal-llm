import { PhoneContactDirectory } from "./pokegear-contacts";

describe("PhoneContactDirectory", () => {
  afterEach(() => {
    delete (globalThis as typeof globalThis & {
      __POKECRYSTAL_PHONE_CONTACT_RUNTIME_CACHE__?: unknown;
    }).__POKECRYSTAL_PHONE_CONTACT_RUNTIME_CACHE__;
    jest.resetModules();
    jest.restoreAllMocks();
  });

  it("loads every shipped non-trainer phone contact from bundled assets", () => {
    const directory = new PhoneContactDirectory();

    expect(directory.resolveContactId("PHONE_MOM")).toBe("PHONE_MOM");
    expect(directory.resolveContactId("PHONE_BILL")).toBe("PHONE_BILL");
    expect(directory.resolveContactId("PHONE_ELM")).toBe("PHONE_ELM");
    expect(directory.resolveContactId("PHONE_BUENA")).toBe("PHONE_BUENA");
    expect(directory.resolveContactId("PHONE_OAK")).toBe("PHONE_OAK");
  });

  it("maps Bike Shop aliases to the PHONE_OAK contact constant from bundled assets", () => {
    const directory = new PhoneContactDirectory();

    expect(directory.resolveContactId("PHONECONTACT_BIKESHOP")).toBe("PHONE_OAK");
    expect(directory.resolveContactId("PHONE_BIKESHOP")).toBe("PHONE_OAK");
    expect(directory.resolveContactId("PHONE_BIKE_SHOP")).toBe("PHONE_OAK");
  });

  it("uses the shipped Bike Shop contact text for synthesized Bike Shop ids", () => {
    const directory = new PhoneContactDirectory();

    expect(directory.displayLines("PHONE_BIKE_SHOP")).toEqual(["BIKE SHOP:"]);
    expect(directory.primaryLabel("PHONE_BIKE_SHOP")).toBe("BIKE SHOP");
  });

  it("throws for unknown contacts instead of synthesizing display text", () => {
    const directory = new PhoneContactDirectory();

    expect(() => directory.displayLines("PHONE_FAKE_CONTACT")).toThrow(
      "Unknown phone contact 'PHONE_FAKE_CONTACT'.",
    );
    expect(() => directory.primaryLabel("PHONE_FAKE_CONTACT")).toThrow(
      "Unknown phone contact 'PHONE_FAKE_CONTACT'.",
    );
  });

  it("does not resolve unknown PHONE_* ids as valid contacts", () => {
    const directory = new PhoneContactDirectory();

    expect(directory.resolveContactId("PHONE_FAKE_CONTACT")).toBeNull();
  });

  it("uses the shipped Mom contact text instead of dropping the non-trainer record", () => {
    const directory = new PhoneContactDirectory();

    expect(directory.displayLines("PHONE_MOM")).toEqual(["MOM:"]);
    expect(directory.primaryLabel("PHONE_MOM")).toBe("MOM");
  });

  it("throws an explicit asset-only error when the bundled contact file is missing", () => {
    jest.doMock("../../core/asset-reader", () => ({
      readJsonAssetSync: () => {
        throw new Error("Failed to load asset /assets/data/phone_contacts.json (status 404)");
      },
    }));
    jest.doMock("../../core/paths", () => ({
      getDataDir: () => "/assets/data",
    }));
    jest.doMock("path", () => ({
      join: (...parts: string[]) => parts.join("/"),
    }));

    jest.isolateModules(() => {
      const { PhoneContactDirectory: IsolatedPhoneContactDirectory } = require("./pokegear-contacts");
      expect(() => new IsolatedPhoneContactDirectory()).toThrow(
        "Phone contact directory requires bundled asset /assets/data/phone_contacts.json: Failed to load asset /assets/data/phone_contacts.json (status 404)",
      );
    });
  });

  it("primes the bundled contact directory so later sync reads are not needed", async () => {
    jest.doMock("../../core/asset-reader", () => ({
      readJsonAsset: async () => ({
        PHONE_MOM: {
          contactId: "PHONE_MOM",
          trainerClass: null,
          trainerLabel: null,
          lines: ["MOM:"],
          primaryLabel: "MOM",
          mapConstant: null,
          calleeTimeMask: 0,
          calleeScript: null,
          callerTimeMask: 0,
          callerScript: null,
        },
      }),
      readJsonAssetSync: () => {
        throw new Error("sync reads should not be needed after priming");
      },
    }));
    jest.doMock("../../core/paths", () => ({
      getDataDir: () => "/assets/data",
    }));
    jest.doMock("path", () => ({
      join: (...parts: string[]) => parts.join("/"),
    }));

    let isolatedModule: typeof import("./pokegear-contacts");
    jest.isolateModules(() => {
      isolatedModule = require("./pokegear-contacts") as typeof import("./pokegear-contacts");
    });

    const directory = await isolatedModule!.primePhoneContactDirectory();

    expect(directory.resolveContactId("PHONE_MOM")).toBe("PHONE_MOM");
    expect(isolatedModule!.loadPhoneContactDirectory().displayLines("PHONE_MOM")).toEqual(["MOM:"]);
  });

  it("keeps the primed contact directory across module reloads", async () => {
    jest.doMock("../../core/asset-reader", () => ({
      readJsonAsset: async () => ({
        PHONE_MOM: {
          contactId: "PHONE_MOM",
          trainerClass: null,
          trainerLabel: null,
          lines: ["MOM:"],
          primaryLabel: "MOM",
          mapConstant: null,
          calleeTimeMask: 0,
          calleeScript: null,
          callerTimeMask: 0,
          callerScript: null,
        },
      }),
      readJsonAssetSync: () => {
        throw new Error("sync reads should not be needed after priming");
      },
    }));
    jest.doMock("../../core/paths", () => ({
      getDataDir: () => "/assets/data",
    }));
    jest.doMock("path", () => ({
      join: (...parts: string[]) => parts.join("/"),
    }));

    let firstModule: typeof import("./pokegear-contacts");
    jest.isolateModules(() => {
      firstModule = require("./pokegear-contacts") as typeof import("./pokegear-contacts");
    });

    await firstModule!.primePhoneContactDirectory();

    jest.resetModules();

    let reloadedModule: typeof import("./pokegear-contacts");
    jest.isolateModules(() => {
      reloadedModule = require("./pokegear-contacts") as typeof import("./pokegear-contacts");
    });

    expect(reloadedModule!.loadPhoneContactDirectory().displayLines("PHONE_MOM")).toEqual(["MOM:"]);
  });
});
