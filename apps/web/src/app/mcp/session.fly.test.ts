import { getMcpSession, __testing } from "./session";

const createInputCaptureGame = () => {
  const overworld = {
    input_capture_active: true,
    dialogue: null,
    script_runner: null,
    player_movement_locked: () => false,
    script_tasks_active: () => false,
  };
  return {
    isBattleActive: () => false,
    isMenuOpen: () => false,
    getDebugStatus: () => ({ mode: "overworld", can_move: true }),
    getOverworld: () => overworld,
  };
};

describe("McpGameSession Fly TUI input capture", () => {
  beforeEach(() => {
    __testing.clearSessions();
  });

  afterEach(() => {
    __testing.clearSessions();
  });

  it("reports first-frame field-move input capture as a prompt before Fly draws menu rows", () => {
    const session = getMcpSession("fly-input-capture-first-frame");
    const sessionAny = session as unknown as {
      lastSnapshot: unknown;
      getModalUiState: (game: ReturnType<typeof createInputCaptureGame>) => {
        prompt_pending: boolean;
        input_capture_active: boolean;
        input_blocked_reason: string | null;
        can_move: boolean;
      };
      getStopReason: (game: ReturnType<typeof createInputCaptureGame>, baselineMap: null) => string | null;
    };
    sessionAny.lastSnapshot = {
      viewport: ["NEW BARK TOWN"],
      info: [],
      menu: [],
      prompt: [],
      dialogue: [],
      titles: { viewport: "Overworld", info: "Info" },
    };

    const game = createInputCaptureGame();
    const modal = sessionAny.getModalUiState(game);

    expect(modal.input_capture_active).toBe(true);
    expect(modal.prompt_pending).toBe(true);
    expect(modal.input_blocked_reason).toBe("prompt");
    expect(modal.can_move).toBe(false);
    expect(sessionAny.getStopReason(game, null)).toBe("prompt");
  });

  it("keeps rendered Fly destination rows as menu-owned input after the map appears", () => {
    const session = getMcpSession("fly-rendered-menu-surface");
    const sessionAny = session as unknown as {
      lastSnapshot: unknown;
      getModalUiState: (game: ReturnType<typeof createInputCaptureGame>) => {
        in_menu: boolean;
        prompt_pending: boolean;
        input_blocked_reason: string | null;
      };
    };
    sessionAny.lastSnapshot = {
      viewport: ["FLY TO WHERE?"],
      info: ["D-Pad=Move A=Select B=Back"],
      menu: ["> NEW BARK TOWN", "  CHERRYGROVE CITY"],
      prompt: [],
      dialogue: [],
      titles: { viewport: "FLY TO WHERE?", info: "Legend" },
    };

    const modal = sessionAny.getModalUiState(createInputCaptureGame());

    expect(modal.in_menu).toBe(true);
    expect(modal.prompt_pending).toBe(false);
    expect(modal.input_blocked_reason).toBe("menu");
  });
});
