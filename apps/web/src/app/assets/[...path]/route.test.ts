import { GET, HEAD } from "./route";
import fs from "node:fs/promises";
import path from "node:path";
import { existsSync } from "node:fs";

describe("direct assets route", () => {
  const resolveFixtureRoot = (): string => {
    const cwd = process.cwd();
    const candidates = [
      path.resolve(cwd, "assets"),
      path.resolve(cwd, "apps", "web", "assets"),
    ];
    for (const candidate of candidates) {
      if (existsSync(candidate)) {
        return candidate;
      }
    }
    return candidates[0];
  };

  const fixtureDir = path.join(resolveFixtureRoot(), "__tests__");
  const fixtureJsonPath = path.join(fixtureDir, "fixture.json");
  const fixtureBinPath = path.join(fixtureDir, "fixture.bin");

  beforeEach(async () => {
    await fs.mkdir(fixtureDir, { recursive: true });
    await fs.writeFile(fixtureJsonPath, JSON.stringify({ ok: true }));
    await fs.writeFile(fixtureBinPath, Buffer.from([1, 2, 3, 4]));
  });

  afterEach(async () => {
    await fs.rm(fixtureDir, { recursive: true, force: true });
  });

  it("serves asset files directly from apps/web/assets", async () => {
    const response = await GET(
      new Request("https://example.com/assets/__tests__/fixture.json"),
      { params: Promise.resolve({ path: ["__tests__", "fixture.json"] }) },
    );

    expect(response.status).toBe(200);
    expect(response.headers.get("Content-Type")).toContain("application/json");
    expect(response.headers.get("Cache-Control")).toBe("public, max-age=31536000, immutable");
    expect(await response.json()).toEqual({ ok: true });
  });

  it("serves committed story-event runtime assets from the /assets path", async () => {
    const response = await GET(
      new Request("https://example.com/assets/data/permanent_phone_numbers.json"),
      { params: Promise.resolve({ path: ["data", "permanent_phone_numbers.json"] }) },
    );

    expect(response.status).toBe(200);
    expect(response.headers.get("Content-Type")).toContain("application/json");
    expect(await response.json()).toEqual(["PHONE_MOM", "PHONE_ELM"]);
  });

  it("supports HEAD from the /assets path", async () => {
    const response = await HEAD(
      new Request("https://example.com/assets/__tests__/fixture.bin"),
      { params: Promise.resolve({ path: ["__tests__", "fixture.bin"] }) },
    );

    expect(response.status).toBe(200);
    expect(response.headers.get("Content-Length")).toBe("4");
    expect(await response.text()).toBe("");
  });

  it("serves committed bundle assets from the /assets path", async () => {
    const response = await GET(
      new Request("https://example.com/assets/data/battle_anim_bundle.json"),
      { params: Promise.resolve({ path: ["data", "battle_anim_bundle.json"] }) },
    );

    expect(response.status).toBe(200);
    const payload = await response.json();
    expect(payload).toHaveProperty("objects");
    expect(payload).toHaveProperty("framesets");
    expect(payload).toHaveProperty("oam_sets");
    expect(payload).toHaveProperty("gfx_table");
    expect(payload).toHaveProperty("gfx_sources");
  });

  it("serves the compiled core content pack from the /assets path", async () => {
    const response = await GET(
      new Request("https://example.com/assets/data/content-packs/core-modular.compiled.json"),
      { params: Promise.resolve({ path: ["data", "content-packs", "core-modular.compiled.json"] }) },
    );

    expect(response.status).toBe(200);
    expect(response.headers.get("Content-Type")).toContain("application/json");
    expect(response.headers.get("Cache-Control")).toBe("public, max-age=31536000, immutable");
    const payload = await response.json();
    expect(payload).toMatchObject({
      version: 1,
      packId: "core-modular",
      categories: expect.objectContaining({
        pokemon: expect.any(Array),
        maps: expect.any(Array),
      }),
    });
  });

  it("rejects traversal outside the assets root", async () => {
    const response = await GET(
      new Request("https://example.com/assets/../../secret.txt"),
      { params: Promise.resolve({ path: ["..", "..", "secret.txt"] }) },
    );

    expect(response.status).toBe(404);
  });
});
