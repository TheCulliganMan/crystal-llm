import { fadeOutToWhite } from "./graphics";

describe("engine events graphics fades", () => {
  it("binds overworld context when queueing fade frames", () => {
    const queue_delay = jest.fn(function (this: { bound?: unknown }, _frames: number) {
      this.bound = this;
      return true;
    });
    const overworld = {
      fade_to_white: jest.fn(),
      queue_delay,
    };
    const runner = {
      _queue_overworld_task: (scheduler: (callback: () => void) => boolean | void) => scheduler(jest.fn()),
    };

    expect(() => fadeOutToWhite(overworld, runner)).not.toThrow();

    expect(queue_delay).toHaveBeenCalled();
    expect((overworld as { bound?: unknown }).bound).toBe(overworld);
  });
});
