import { createPlayerTools } from "./tools";

describe("player tool selection", () => {
  it("keeps only the small local-model gameplay surface in compact mode", async () => {
    const allTools = {
      observe: {},
      map_info: {},
      route_render: {},
      flow_state: {},
      move: {},
      press: {},
      type_text: {},
      hold_button: {},
      status: {},
      recent_events: {},
    };
    const session = {
      listPlayerTools: jest.fn().mockResolvedValue(allTools),
    };

    await expect(createPlayerTools(session as never, { compact: true })).resolves.toEqual({
      observe: {},
      map_info: {},
      route_render: {},
      move: {},
      press: {},
      status: {},
    });
  });

  it("recognizes namespaced MCP tool names when compacting", async () => {
    const session = {
      listPlayerTools: jest.fn().mockResolvedValue({
        krabbyclaw_observe: {},
        krabbyclaw_route_render: {},
        krabbyclaw_move: {},
        krabbyclaw_recent_events: {},
      }),
    };

    await expect(createPlayerTools(session as never, { compact: true })).resolves.toEqual({
      krabbyclaw_observe: {},
      krabbyclaw_route_render: {},
      krabbyclaw_move: {},
    });
  });

  it("keeps the full toolset outside compact mode", async () => {
    const allTools = {
      observe: {},
      recent_events: {},
    };
    const session = {
      listPlayerTools: jest.fn().mockResolvedValue(allTools),
    };

    await expect(createPlayerTools(session as never)).resolves.toBe(allTools);
  });
});
