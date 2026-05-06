import type { ScriptEntry } from "@pokecrystal/core/core/data-loader";

const command = (name: string, args?: ScriptEntry["args"]): ScriptEntry =>
  args ? { command: name, args } : { command: name };

function jump(target: string): ScriptEntry {
  return command("jump", [target]);
}

function end(): ScriptEntry {
  return command("end");
}

// ASM mapping: pokecrystal_disassembly/maps/ElmsLab.asm::AideScript_GivePotion
export const elmsLabScripts: Record<string, ScriptEntry[]> = {
  ElmsLabMeetElmScene: [jump("ElmGetsEmail")],
  ElmGetsEmail: [end()],
  MeetCopScript: [jump("CopScript")],
  CopScript: [end()],
  AideScript_GivePotion: [
    command("opentext"),
    command("writetext", ["AideText_GiveYouPotion"]),
    command("promptbutton"),
    command("verbosegiveitem", ["POTION"]),
    command("writetext", ["AideText_AlwaysBusy"]),
    command("waitbutton"),
    command("closetext"),
    command("setscene", ["SCENE_ELMSLAB_NOOP"]),
    command("end"),
  ],
};
