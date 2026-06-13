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
      flow_state: {},
      move: {},
      press: {},
      type_text: {},
      hold_button: {},
      execute_macro: {},
      wait: {},
      noop: {},
      recent_events: {},
    };
    const session = {
      listPlayerTools: jest.fn().mockResolvedValue(allTools),
    };

    await expect(createPlayerTools(session as never)).resolves.toEqual({
      observe: {},
      flow_state: {},
      move: {},
      press: {},
      type_text: {},
      hold_button: {},
      recent_events: {},
    });
  });

  it("filters disallowed gameplay tools from namespaced full mode", async () => {
    const session = {
      listPlayerTools: jest.fn().mockResolvedValue({
        krabbyclaw_status: {},
        krabbyclaw_move: {},
        krabbyclaw_press: {},
        krabbyclaw_execute_macro: {},
        krabbyclaw_wait: {},
        krabbyclaw_skip: {},
        unrelated_tool: {},
      }),
    };

    await expect(createPlayerTools(session as never)).resolves.toEqual({
      krabbyclaw_status: {},
      krabbyclaw_move: {},
      krabbyclaw_press: {},
    });
  });
});
