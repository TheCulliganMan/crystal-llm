import { ElevatorRideStateMachine, type ElevatorOverworld } from "./events";

describe("ElevatorRideStateMachine audio guards", () => {
  it("plays elevator sound via playSound when play_sound is missing", () => {
    const machine = new ElevatorRideStateMachine();
    const playSound = jest.fn();
    const overworld: ElevatorOverworld = {
      audio_engine: { play_sound: playSound },
      lock_player_movement: jest.fn(),
      unlock_player_movement: jest.fn(),
      fade_to_black: jest.fn(),
      fade_from_black: jest.fn(),
    };

    machine.door_close_frames = 0;
    machine.fade_frames = 0;
    machine.travel_frames = 1;
    machine.start(overworld);

    expect(() => machine.update(overworld)).not.toThrow();
    expect(() => machine.update(overworld)).not.toThrow();
    expect(playSound).toHaveBeenCalledWith("SFX_ELEVATOR");
  });
});
