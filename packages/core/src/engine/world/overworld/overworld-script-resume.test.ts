import { OverworldEngine } from "@pokecrystal/core/engine/world/overworld/overworld";

type UpdateFn = (this: unknown) => boolean;

const updateDialogueAndScripts = (
  OverworldEngine as unknown as { prototype: { _update_dialogue_and_scripts: UpdateFn } }
).prototype._update_dialogue_and_scripts;

const buildOverworldStub = (queuedTasks: number) => {
  const resume = jest.fn();
  const scriptRunner = {
    stop_execution: true,
    _awaiting_resume: 1,
    _script_stack: [{}],
    _queued_overworld_task_count: queuedTasks,
    resume,
  };

  const dialogue = {
    update: jest.fn(),
    pending_waits: 0,
    waiting_for_input: false,
    is_script_paused: false,
    active: false,
    visible: false,
  };

  const stub = {
    dialogue,
    script_runner: scriptRunner,
    script_tasks_active: () => false,
    _record_wait_status: jest.fn(),
    _hatch_text_pending: false,
    _finalize_hatch_sequence: jest.fn(),
    _text_lock_active: false,
    unlock_player_movement: jest.fn(),
    _last_pending_script_status: null,
    _logger: { debug: jest.fn() },
    current_map_name: "VioletPokecenter1F",
  };

  return { stub, resume };
};

describe("OverworldEngine queued task resume guard", () => {
  it("does not auto-resume stale stop_execution while a queued overworld task is still active", () => {
    const { stub, resume } = buildOverworldStub(1);

    const result = updateDialogueAndScripts.call(stub);

    expect(result).toBe(false);
    expect(resume).not.toHaveBeenCalled();
  });

  it("auto-resumes stale stop_execution when no queued overworld task is active", () => {
    const { stub, resume } = buildOverworldStub(0);

    const result = updateDialogueAndScripts.call(stub);

    expect(result).toBe(false);
    expect(resume).toHaveBeenCalledTimes(1);
  });

  it("auto-resumes a paused stack when no dialogue wait remains", () => {
    const { stub, resume } = buildOverworldStub(0);
    stub.script_runner.stop_execution = false;
    stub.script_runner._awaiting_resume = 0;

    const result = updateDialogueAndScripts.call(stub);

    expect(result).toBe(false);
    expect(resume).toHaveBeenCalledTimes(1);
  });

  it("does not clear the text lock while a script is awaiting menu input", () => {
    const { stub } = buildOverworldStub(0);
    stub._text_lock_active = true;

    updateDialogueAndScripts.call(stub);

    expect(stub.unlock_player_movement).not.toHaveBeenCalled();
    expect(stub._text_lock_active).toBe(true);
  });

  it("does not auto-resume while a yes/no prompt is still transitioning into view", () => {
    const { stub, resume } = buildOverworldStub(0);
    stub.dialogue.pending_yes_no_request = true;

    updateDialogueAndScripts.call(stub);

    expect(resume).not.toHaveBeenCalled();
  });
});
