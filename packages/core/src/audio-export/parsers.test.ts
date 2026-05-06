import { AsmAudioParser, DrumkitParser, WaveSampleParser } from "@pokecrystal/core/audio-export/parsers";

describe("audio-export parsers", () => {
  it("parses drumkits", () => {
    const text = `
Drumkits:
Drumkit0:
  dw Snare
Snare:
  noise_note 4, 10, 0, 0x2f
`;
    const parser = new DrumkitParser();
    const parsed = parser.parseFromText(text);
    expect(parsed[0][0][0].length).toBe(4);
  });

  it("parses wave samples and instrument mapping", () => {
    const waves = `dn ${new Array(32).fill("1").join(", ")}`;
    const inst = "dw WaveSamples + 0";
    const parser = new WaveSampleParser();
    const parsed = parser.parseFromText(waves, inst);
    expect(parsed[0]).toHaveLength(32);
    expect(parser.instrumentMap[0]).toBe(0);
  });

  it("parses asm channels and commands", () => {
    const asm = `
Music_Test:
  channel_count 1
  channel 1, Music_Test_Ch1
Music_Test_Ch1:
  note_type 12
.mainloop:
  note C_, 4
  rest 2
  sound_jump .mainloop
`;
    const parser = new AsmAudioParser(asm);
    const out = parser.parse();
    expect(out.channel_count).toBe(1);
    expect(out.channels.Music_Test_Ch1.number).toBe(1);
    expect(out.channels.Music_Test_Ch1.commands.some((c) => c.command === "note")).toBe(true);
  });

  it("scopes local sound_call targets to the current channel", () => {
    const asm = `
Music_Test:
  channel_count 2
  channel 1, Music_Test_Ch1
  channel 2, Music_Test_Ch2
Music_Test_Ch1:
  note_type 12
  sound_call .sub1
  rest 1
.sub1:
  note C_, 1
  sound_ret
Music_Test_Ch2:
  note_type 12
  sound_call .sub1
  rest 1
.sub1:
  note D_, 1
  sound_ret
`;
    const parser = new AsmAudioParser(asm);
    const out = parser.parse();
    const call1 = out.channels.Music_Test_Ch1.commands.find((cmd) => cmd.command === "sound_call");
    const call2 = out.channels.Music_Test_Ch2.commands.find((cmd) => cmd.command === "sound_call");
    expect(call1?.args[0]).toBe("Music_Test_Ch1.sub1");
    expect(call2?.args[0]).toBe("Music_Test_Ch2.sub1");
    expect(out.subroutines["Music_Test_Ch1.sub1"]).toBeDefined();
    expect(out.subroutines["Music_Test_Ch2.sub1"]).toBeDefined();
  });

  it("maps alias labels that immediately follow a channel label", () => {
    const asm = `
Music_Test:
  channel_count 1
  channel 1, Music_Test_Ch1
Music_Test_Ch1:
Music_Test_Ch1_Alias:
  note_type 12
  note C_, 1
`;
    const parser = new AsmAudioParser(asm);
    const out = parser.parse();
    expect(out.channels.Music_Test_Ch1_Alias).toBeDefined();
    expect(out.channels.Music_Test_Ch1_Alias).toBe(out.channels.Music_Test_Ch1);
    expect(out.channels.Music_Test_Ch1.commands.some((cmd) => cmd.command === "label" && cmd.args[0] === "Music_Test_Ch1_Alias")).toBe(true);
  });
});
