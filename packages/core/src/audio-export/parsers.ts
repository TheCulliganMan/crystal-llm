import type { NoiseNote } from "./schemas";
import { parseNumber } from "./utils";

export interface AudioCommand {
  command: string;
  args: string[];
}

export interface AudioSource {
  number?: number;
  commands: AudioCommand[];
}

export interface ParsedMusicData {
  channel_count: number;
  channels: Record<string, AudioSource>;
  subroutines: Record<string, AudioSource>;
  shared_sources?: Record<string, AudioSource>;
}

export class DrumkitParser {
  static readonly DRUMKIT_LABEL = /Drumkit(\d+)$/;

  parseFromText(text: string): Record<number, Record<number, NoiseNote[]>> {
    const kits: Record<number, string[]> = {};
    const instruments: Record<string, NoiseNote[]> = {};
    let currentLabel: string | null = null;

    for (const raw of text.split(/\r?\n/)) {
      const line = raw.split(";", 1)[0].trim();
      if (!line) {
        continue;
      }

      if (line.endsWith(":")) {
        currentLabel = line.slice(0, -1);
        if (currentLabel === "Drumkits") {
          continue;
        }
        if (currentLabel.startsWith("Drumkit")) {
          kits[this.parseKitId(currentLabel)] = kits[this.parseKitId(currentLabel)] ?? [];
        } else {
          instruments[currentLabel] = instruments[currentLabel] ?? [];
        }
        continue;
      }

      if (currentLabel == null || currentLabel === "Drumkits") {
        continue;
      }

      if (currentLabel.startsWith("Drumkit")) {
        if (line.startsWith("dw") || line.startsWith("dr")) {
          const entries = line.slice(2).split(",").map((v) => v.trim()).filter(Boolean);
          kits[this.parseKitId(currentLabel)].push(...entries);
        }
        continue;
      }

      if (line.startsWith("noise_note")) {
        const params = line.split(/[\s,]+/).slice(1).filter(Boolean).map(parseNumber);
        if (params.length !== 4) {
          throw new Error(`Unexpected noise_note: ${line}`);
        }
        instruments[currentLabel].push({
          length: params[0],
          volume: params[1],
          fade: params[2],
          frequency: params[3],
        });
      }
    }

    const out: Record<number, Record<number, NoiseNote[]>> = {};
    for (const [kitIdRaw, names] of Object.entries(kits)) {
      const kitId = Number(kitIdRaw);
      out[kitId] = {};
      names.forEach((name, idx) => {
        out[kitId][idx] = [...(instruments[name] ?? [])];
      });
    }

    return out;
  }

  private parseKitId(label: string): number {
    const match = label.match(DrumkitParser.DRUMKIT_LABEL);
    if (!match) {
      throw new Error(`Bad drumkit label ${label}`);
    }
    return Number(match[1]);
  }
}

export class WaveSampleParser {
  public instrumentMap: Record<number, number> = {};

  parseFromText(samplesText: string, waveInstrumentTableText?: string): Record<number, number[]> {
    const samples: Record<number, number[]> = {};
    let idx = 0;

    for (const raw of samplesText.split(/\r?\n/)) {
      const line = raw.split(";", 1)[0].trim();
      if (!line) {
        continue;
      }
      const match = line.match(/^(dn|db)\s+(.*)$/i);
      if (!match) {
        continue;
      }

      const mode = match[1].toLowerCase();
      const parts = match[2].split(",").map((v) => v.trim()).filter(Boolean);
      if (mode === "dn") {
        if (parts.length !== 32) {
          throw new Error(`Wave sample ${idx} has ${parts.length} nybbles (need 32)`);
        }
        samples[idx] = parts.map(parseNumber);
      } else {
        if (parts.length * 2 !== 32) {
          throw new Error(`Wave sample ${idx} has ${parts.length} bytes (need 16 for 32 nybbles)`);
        }
        const nibbles: number[] = [];
        for (const token of parts) {
          const value = parseNumber(token);
          if (value < 0 || value > 0xff) {
            throw new Error(`Wave sample ${idx} db value ${token} out of range`);
          }
          nibbles.push((value >> 4) & 0xf, value & 0xf);
        }
        samples[idx] = nibbles;
      }
      idx += 1;
    }

    this.instrumentMap = waveInstrumentTableText
      ? this.parseInstrumentTable(waveInstrumentTableText, idx)
      : Object.fromEntries(Array.from({ length: idx }, (_, i) => [i, i]));
    return samples;
  }

  private parseInstrumentTable(tableText: string, count: number): Record<number, number> {
    const mapping: Record<number, number> = {};
    let instrumentId = 0;

    for (const raw of tableText.split(/\r?\n/)) {
      const line = raw.split(";", 1)[0].trim();
      if (!line || line.endsWith(":") || !line.startsWith("dw")) {
        continue;
      }
      const tokens = line.slice(2).split(",").map((v) => v.trim()).filter(Boolean);
      for (const token of tokens) {
        const sampleIndex = this.tokenToSampleIndex(token);
        if (sampleIndex != null && sampleIndex < count) {
          mapping[instrumentId] = sampleIndex;
          instrumentId += 1;
        }
      }
    }

    return Object.keys(mapping).length > 0 ? mapping : Object.fromEntries(Array.from({ length: count }, (_, i) => [i, i]));
  }

  private tokenToSampleIndex(token: string): number | null {
    const parts = token.trim().split("+").map((v) => v.trim()).filter(Boolean);
    if (parts.length === 0) {
      return null;
    }

    let seenBase = false;
    let offset = 0;
    for (const part of parts) {
      if (part.startsWith("WaveSamples")) {
        seenBase = true;
        const rem = part.slice("WaveSamples".length).trim();
        if (rem) {
          offset += parseNumber(rem);
        }
      } else {
        offset += parseNumber(part);
      }
    }

    if (!seenBase) {
      return null;
    }
    if (offset % 16 !== 0) {
      throw new Error(`Unaligned wave instrument pointer '${token}'`);
    }
    return Math.floor(offset / 16);
  }
}

export class AsmAudioParser {
  private static readonly SUBROUTINE_LABEL_RE = /^\.sub\d+$/;
  private readonly lines: string[];

  private musicData: ParsedMusicData = {
    channel_count: 0,
    channels: {},
    subroutines: {},
  };

  private currentLabel: string | null = null;
  private currentScopeLabel: string | null = null;
  private currentChannelNumber: number | null = null;
  private primaryLabel: string | null = null;
  private commandsSinceLabel = 0;

  constructor(asmText: string) {
    this.lines = asmText.split(/\r?\n/);
  }

  parse(): ParsedMusicData {
    for (const raw of this.lines) {
      const line = this.cleanLine(raw);
      if (!line) {
        continue;
      }
      if (line.endsWith(":")) {
        this.processLabel(line.slice(0, -1));
        continue;
      }
      this.processLine(line);
    }
    return this.musicData;
  }

  private cleanLine(s: string): string {
    return s.split(";", 1)[0].trim();
  }

  private processLabel(name: string): void {
    if (this.handleAliasLabel(name)) {
      return;
    }

    if (!name.startsWith(".")) {
      this.currentScopeLabel = name;
    }

    if (this.musicData.channels[name]) {
      this.currentLabel = name;
      this.storeCommand({ command: "label", args: [name] });
      this.commandsSinceLabel = 0;
      return;
    }

    if (this.musicData.subroutines[name]) {
      this.currentLabel = name;
      this.storeCommand({ command: "label", args: [name] });
      this.commandsSinceLabel = 0;
      return;
    }

    if (name.startsWith(".")) {
      if (this.isSubroutineLabel(name)) {
        const owner = this.currentScopeLabel;
        const scopedName = owner ? `${owner}${name}` : name;
        if (!this.musicData.subroutines[scopedName]) {
          this.musicData.subroutines[scopedName] = { commands: [] };
        }
        this.currentLabel = scopedName;
        this.storeCommand({ command: "label", args: [name] });
        this.commandsSinceLabel = 0;
        return;
      }
      if (!this.currentLabel) {
        return;
      }
      this.storeCommand({ command: "label", args: [name] });
      this.commandsSinceLabel = 0;
      return;
    }

    if (this.primaryLabel == null) {
      this.primaryLabel = name;
      this.currentLabel = name;
      this.commandsSinceLabel = 0;
      return;
    }

    this.musicData.subroutines[name] = this.musicData.subroutines[name] ?? { commands: [] };
    this.currentLabel = name;
    this.storeCommand({ command: "label", args: [name] });
    this.commandsSinceLabel = 0;
  }

  private processLine(line: string): void {
    const [command, ...argTokens] = line.split(/\s+/);

    if (command === "channel_count") {
      this.musicData.channel_count = parseNumber(argTokens[0]);
      return;
    }

    if (command === "channel") {
      const args = line.slice("channel".length).split(",").map((v) => v.trim()).filter(Boolean);
      if (args.length < 2) {
        throw new Error(`Malformed channel declaration: ${line}`);
      }
      const number = parseNumber(args[0]);
      const label = args[1];
      this.musicData.channels[label] = this.musicData.channels[label] ?? { number, commands: [] };
      this.currentChannelNumber = number;
      return;
    }

    if (this.currentLabel == null) {
      return;
    }

    const cmd = this.parseLineAsCommand(line);
    this.storeCommand(cmd);
  }

  private parseLineAsCommand(line: string): AudioCommand {
    const parts = line.split(/\s+/);
    const command = parts[0];
    const argString = line.slice(command.length).trim();
    const args = argString ? argString.split(",").map((v) => v.trim()).filter(Boolean) : [];
    return { command, args };
  }

  private storeCommand(cmd: AudioCommand): void {
    if (this.currentLabel == null) {
      return;
    }

    let normalized = cmd;
    if (
      cmd.command === "sound_call"
      && cmd.args.length > 0
      && cmd.args[0].startsWith(".")
      && this.currentScopeLabel
    ) {
      normalized = { ...cmd, args: [`${this.currentScopeLabel}${cmd.args[0]}`, ...cmd.args.slice(1)] };
    }

    if (this.musicData.channels[this.currentLabel]) {
      this.musicData.channels[this.currentLabel].commands.push(normalized);
      if (normalized.command !== "label") {
        this.commandsSinceLabel += 1;
      }
      return;
    }

    if (this.musicData.subroutines[this.currentLabel]) {
      this.musicData.subroutines[this.currentLabel].commands.push(normalized);
      if (normalized.command !== "label") {
        this.commandsSinceLabel += 1;
      }
      return;
    }

    this.musicData.subroutines[this.currentLabel] = { number: this.currentChannelNumber ?? undefined, commands: [normalized] };
    if (normalized.command !== "label") {
      this.commandsSinceLabel += 1;
    }
  }

  private isSubroutineLabel(name: string): boolean {
    return AsmAudioParser.SUBROUTINE_LABEL_RE.test(name);
  }

  private getCurrentCommandList(): AudioCommand[] | null {
    if (this.currentLabel == null) {
      return null;
    }
    if (this.musicData.channels[this.currentLabel]) {
      return this.musicData.channels[this.currentLabel].commands;
    }
    if (this.musicData.subroutines[this.currentLabel]) {
      return this.musicData.subroutines[this.currentLabel].commands;
    }
    return null;
  }

  private handleAliasLabel(name: string): boolean {
    if (name.startsWith(".")) {
      return false;
    }
    if (this.musicData.channels[name] || this.musicData.subroutines[name]) {
      return false;
    }
    const targetCommands = this.getCurrentCommandList();
    if (!targetCommands || this.commandsSinceLabel !== 0 || this.currentLabel == null) {
      return false;
    }

    if (this.musicData.channels[this.currentLabel]) {
      this.musicData.channels[name] = this.musicData.channels[this.currentLabel];
    } else if (this.musicData.subroutines[this.currentLabel]) {
      this.musicData.subroutines[name] = this.musicData.subroutines[this.currentLabel];
    } else {
      return false;
    }

    this.storeCommand({ command: "label", args: [name] });
    this.commandsSinceLabel = 0;
    return true;
  }
}
