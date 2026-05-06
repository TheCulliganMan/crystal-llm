import { TerminalInputAdapter } from "./adapters";
import { keycodes } from "../core/keycodes";

class TestTerminalAdapter extends TerminalInputAdapter {
  public exposeEventsForToken(token: string) {
    return this.eventsForToken(token);
  }
}

describe("TerminalInputAdapter", () => {
  it("uses the configured button bindings for start/select tokens", () => {
    const adapter = new TestTerminalAdapter({ stdin: null });
    const [startEvent] = adapter.exposeEventsForToken("start");
    const [selectEvent] = adapter.exposeEventsForToken("select");

    expect(startEvent.key).toBe(keycodes.Enter);
    expect(selectEvent.key).toBe(keycodes.Backspace);
  });
});
