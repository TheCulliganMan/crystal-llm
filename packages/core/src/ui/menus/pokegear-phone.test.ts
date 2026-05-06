import { createInitialGameState } from "@pokecrystal/core/core/state";
import { getMapMetadataByConstant } from "@pokecrystal/core/engine/world/maps";
import { PokegearScreen } from "./pokegear";
import { PokegearCard, PokegearScriptRunner } from "./pokegear-state";

class StubPhoneScriptRunner implements PokegearScriptRunner {
  public executed: string[] = [];
  public queued: string[] = [];
  public consumed: string[] = [];

  runPhoneScript(scriptName: string): void {
    this.executed.push(scriptName);
  }

  queuePhoneCall(contactId: string): void {
    this.queued.push(contactId);
  }

  consumePhoneCall(contactId: string): { contact: string } {
    this.consumed.push(contactId);
    return { contact: contactId };
  }
}

const createPokegearUi = () => ({ screen: null, font: {} });

const buildPokegearScreen = (contactId: string): {
  gameState: ReturnType<typeof createInitialGameState>;
  screen: PokegearScreen;
  runner: StubPhoneScriptRunner;
  audioEngine: { playSound: jest.Mock };
} => {
  const gameState = createInitialGameState();
  const runner = new StubPhoneScriptRunner();
  const audioEngine = { playSound: jest.fn() };
  gameState.sram.phone_numbers = [contactId];
  gameState.wram.engine_flags["ENGINE_POKEGEAR"] = true;
  gameState.wram.engine_flags["ENGINE_PHONE_CARD"] = true;
  gameState.wram.time_of_day = "day";
  const screen = new PokegearScreen(createPokegearUi(), gameState, { scriptRunner: runner, audioEngine } as any);
  screen.logic.forceCard(PokegearCard.PHONE);
  return { gameState, screen, runner, audioEngine };
};

const setMap = (gameState: ReturnType<typeof createInitialGameState>, mapConstant: string): void => {
  const metadata = getMapMetadataByConstant(mapConstant);
  if (!metadata) {
    throw new Error(`Unknown map constant ${mapConstant}`);
  }
  gameState.wram.wMapGroup = metadata.groupId;
  gameState.wram.wMapNumber = metadata.mapId;
};

describe("Pokegear phone call resolution", () => {
  it("uses the JustTalk script when calling a contact on the current map", () => {
    const { gameState, screen, runner, audioEngine } = buildPokegearScreen("PHONE_BUG_CATCHER_WADE");
    setMap(gameState, "ROUTE_31");

    screen.handleInput({ type: "keydown", key: "KeyZ" });

    expect(runner.executed).toEqual(["PhoneScript_JustTalkToThem"]);
    expect(audioEngine.playSound).toHaveBeenCalledWith("SFX_CALL");
  });

  it("does not dispatch a phone script when phone service is unavailable", () => {
    const { gameState, screen, runner, audioEngine } = buildPokegearScreen("PHONE_BUG_CATCHER_WADE");
    setMap(gameState, "DARK_CAVE_VIOLET_ENTRANCE");

    screen.handleInput({ type: "keydown", key: "KeyZ" });

    expect(runner.executed).toEqual([]);
    expect(audioEngine.playSound).not.toHaveBeenCalledWith("SFX_CALL");
  });

  it("uses PhoneOutOfAreaScript while in link communication", () => {
    const { gameState, screen, runner, audioEngine } = buildPokegearScreen("PHONE_BUG_CATCHER_WADE");
    setMap(gameState, "ROUTE_31");
    gameState.wram.wLinkMode = 1;

    screen.handleInput({ type: "keydown", key: "KeyZ" });

    expect(runner.executed).toEqual(["PhoneOutOfAreaScript"]);
    expect(audioEngine.playSound).toHaveBeenCalledWith("SFX_CALL");
  });

  it("uses PhoneOutOfAreaScript when contact time mask does not match current time", () => {
    const { gameState, screen, runner, audioEngine } = buildPokegearScreen("PHONE_BUG_CATCHER_WADE");
    setMap(gameState, "ROUTE_31");
    gameState.wram.time_of_day = "morn";
    const directory = (screen as unknown as { contactDirectory: { record: (value: string) => object | null } }).contactDirectory;
    const originalRecord = directory.record("PHONE_BUG_CATCHER_WADE") as {
      calleeTimeMask: number;
      calleeScript: string | null;
    };
    if (!originalRecord) {
      throw new Error("Expected PHONE_BUG_CATCHER_WADE contact to be present");
    }

    const originalRecordFn = directory.record.bind(directory);
    directory.record = (value: string) =>
      value === "PHONE_BUG_CATCHER_WADE" ? { ...originalRecord, calleeTimeMask: 0x4 } : originalRecordFn(value);

    try {
      screen.handleInput({ type: "keydown", key: "KeyZ" });
      expect(runner.executed).toEqual(["PhoneOutOfAreaScript"]);
      expect(audioEngine.playSound).toHaveBeenCalledWith("SFX_CALL");
    } finally {
      directory.record = originalRecordFn;
    }
  });

  it("plays the menu click sound when switching cards", () => {
    const { screen, audioEngine } = buildPokegearScreen("PHONE_BUG_CATCHER_WADE");

    screen.handleInput({ type: "keydown", key: "ArrowLeft" });

    expect(audioEngine.playSound).toHaveBeenCalledWith("SFX_READ_TEXT_2");
  });

  it("plays the menu click sound when exiting pokegear", () => {
    const { screen, audioEngine } = buildPokegearScreen("PHONE_BUG_CATCHER_WADE");

    const result = screen.handleInput({ type: "keydown", key: "x", code: "KeyX" });

    expect(result).toBe("exit");
    expect(audioEngine.playSound).toHaveBeenCalledWith("SFX_READ_TEXT_2");
  });
});
