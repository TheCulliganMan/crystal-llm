describe("preloadCoreDataAssets browser preload coverage", () => {
  const originalWindow = global.window;
  const originalXHR = global.XMLHttpRequest;
  const originalFetch = global.fetch;

  afterEach(() => {
    jest.resetModules();
    jest.restoreAllMocks();
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

  it("preloads phone registration assets so permanent numbers resolve without sync XHR", async () => {
    (global as typeof globalThis & { window?: Window }).window = {} as Window;
    const fetchedTargets: string[] = [];
    const syncTargets: string[] = [];
    const phoneContactsPayload = JSON.stringify({
      PHONE_MOM: {
        contactId: "PHONE_MOM",
        trainerClass: "TRAINER_NONE",
        trainerLabel: "PHONECONTACT_MOM",
        lines: ["MOM:"],
        primaryLabel: "MOM",
        mapConstant: "PLAYERS_HOUSE_1F",
        calleeTimeMask: 7,
        calleeScript: "MomPhoneCalleeScript",
        callerTimeMask: 0,
        callerScript: "UnusedPhoneScript",
      },
    });

    (global as typeof globalThis & { fetch?: typeof fetch }).fetch = jest.fn(
      async (target: string | URL | Request) => {
        const url = String(target);
        fetchedTargets.push(url);
        if (url.endsWith("/phone_contacts.json")) {
          return {
            ok: true,
            status: 200,
            text: async () => phoneContactsPayload,
          } as Response;
        }
        if (url.endsWith("/permanent_phone_numbers.json")) {
          return {
            ok: true,
            status: 200,
            text: async () => '["PHONE_MOM"]',
          } as Response;
        }
        if (url.endsWith("/initialize_events.json")) {
          return {
            ok: true,
            status: 200,
            text: async () =>
              '{"eventFlags":["EVENT_TEST"],"engineFlags":[],"variableSprites":{}}',
          } as Response;
        }
        return {
          ok: true,
          status: 200,
          text: async () => "{}",
        } as Response;
      },
    );

    class FailingXMLHttpRequest {
      status = 0;
      responseText = "";

      open(_method: string, target: string): void {
        syncTargets.push(target);
      }

      send(): void {
        throw new Error("sync XHR should not be needed after preload");
      }
    }

    (global as typeof globalThis & { XMLHttpRequest?: typeof XMLHttpRequest }).XMLHttpRequest =
      FailingXMLHttpRequest as unknown as typeof XMLHttpRequest;

    let preloadCoreDataAssets: typeof import("./data-loader").preloadCoreDataAssets;
    let loadPermanentPhoneNumbers: typeof import("../engine/world/story-events/common").loadPermanentPhoneNumbers;
    let loadInitializeEventsConfig: typeof import("../engine/world/story-events/common").loadInitializeEventsConfig;

    jest.isolateModules(() => {
      ({ preloadCoreDataAssets } = require("./data-loader") as typeof import("./data-loader"));
      ({ loadPermanentPhoneNumbers, loadInitializeEventsConfig } = require(
        "../engine/world/story-events/common"
      ) as typeof import("../engine/world/story-events/common"));
    });

    await preloadCoreDataAssets!("core");

    expect(loadPermanentPhoneNumbers!()).toEqual(["PHONE_MOM"]);
    expect(loadInitializeEventsConfig!()).toEqual({
      eventFlags: ["EVENT_TEST"],
      engineFlags: [],
      variableSprites: {},
    });
    expect(syncTargets).toEqual([]);
    expect(fetchedTargets).toEqual(
      expect.arrayContaining([
        "/assets/data/phone_contacts.json",
        "/assets/data/permanent_phone_numbers.json",
        "/assets/data/initialize_events.json",
      ])
    );
  });
});
