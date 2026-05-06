import { BaseUI } from "@pokecrystal/core/ui/base-ui";
import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import { Surface } from "@pokecrystal/core/ui/surface";
import { SelectionPrompt } from "@pokecrystal/core/ui/text/prompts";

class TestPromptUI extends BaseUI {
  public update = jest.fn();
  public font = {
    renderText: jest.fn(),
  };

  protected createScreenSurface(): Surface {
    return new Surface(160, 144);
  }
}

describe("SelectionPrompt.run", () => {
  it("moves silently and clicks only when confirming", () => {
    const ui = new TestPromptUI();
    ui.eventQueue = gameEngine.event.createQueue();
    const audioEngine = { playSound: jest.fn() } as any;
    const prompt = new SelectionPrompt(ui, ["YES", "NO"], { audioEngine });

    prompt.handleInput({ type: "keydown", code: "ArrowDown", key: "ArrowDown" } as any);
    expect(audioEngine.playSound).not.toHaveBeenCalled();

    prompt.handleInput({ type: "keydown", code: "KeyZ", key: "z", button: "a" } as any);
    expect(audioEngine.playSound).toHaveBeenCalledWith("menu_option");
  });

  it("accepts direct player direction-only events", () => {
    const ui = new TestPromptUI();
    const prompt = new SelectionPrompt(ui, ["YES", "NO"]);

    prompt.handleInput({ type: "keydown", direction: "down", is_press: true } as any);
    prompt.handleInput({ type: "keydown", button: "a", is_press: true } as any);

    expect(prompt.result()).toBe(1);
  });

  it("consumes queued input synchronously and returns the confirmed option", () => {
    const ui = new TestPromptUI();
    ui.eventQueue = gameEngine.event.createQueue();
    gameEngine.event.post({ type: "keydown", code: "ArrowDown", key: "ArrowDown" }, ui.eventQueue);
    gameEngine.event.post({ type: "keydown", code: "KeyZ", key: "z", button: "a" }, ui.eventQueue);

    const prompt = new SelectionPrompt(ui, ["BILL'S PC", "PLAYER'S PC", "TURN OFF"]);

    const result = prompt.run();

    expect(result).toBe(1);
    expect(ui.update).toHaveBeenCalled();
  });

  it("maps cancel input to the last option in synchronous mode", () => {
    const ui = new TestPromptUI();
    ui.eventQueue = gameEngine.event.createQueue();
    gameEngine.event.post({ type: "keydown", code: "KeyX", key: "x", button: "b" }, ui.eventQueue);

    const prompt = new SelectionPrompt(ui, ["YES", "NO"]);

    const result = prompt.run();

    expect(result).toBe(1);
    expect(prompt.result()).toBe(1);
  });
});
