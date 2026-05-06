import { serializeStructuredPayload } from "./serialization";

describe("serializeStructuredPayload", () => {
  it("preserves empty arrays because they are valid structured state, not absent fields", async () => {
    const serialized = await serializeStructuredPayload({
      total: 0,
      events: [],
      nested: {
        actions: [],
      },
      missing: undefined,
    });

    expect(serialized).toEqual({
      text: JSON.stringify({
        total: 0,
        events: [],
        nested: {
          actions: [],
        },
      }),
      mimeType: "application/json",
    });
  });
});
