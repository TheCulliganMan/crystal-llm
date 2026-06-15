/** @jest-environment jsdom */
import React, { act } from "react";
import { createRoot } from "react-dom/client";
import { PlayPanel } from "./play-panel";
import { useMultiplayerStore } from "@pokecrystal/core/multiplayer/multiplayer-store";
import { createSupabaseBrowserClient } from "@/lib/supabase/browser";
import { MANUAL_SAVE_SLOT } from "@pokecrystal/core/core/save-slots";

let consoleWarnSpy: jest.SpyInstance;
let consoleErrorSpy: jest.SpyInstance;
const mockSettingsPanelSpy = jest.fn();
const mockDesktopMcpPanelSpy = jest.fn();
const mockMultiplayerMenuSpy = jest.fn();
const mockGameCanvasSpy = jest.fn();
const mockGuestSavePanelSpy = jest.fn();
const mockCreateSupabaseBrowserClient = createSupabaseBrowserClient as jest.MockedFunction<
  typeof createSupabaseBrowserClient
>;
const mockTrade = jest.fn();
const mockPresenceManagers: any[] = [];
const mockWebRtcConnections: any[] = [];
jest.mock("./game-canvas", () => ({
  GameCanvas: (props: Record<string, unknown>) => {
    mockGameCanvasSpy(props);
    return <div data-testid="game-canvas" />;
  },
}));

jest.mock("./virtual-gamepad", () => ({
  VirtualGamepad: (props: Record<string, unknown>) => (
    <div data-testid="virtual-gamepad">{props.systemControl as React.ReactNode}</div>
  ),
}));

jest.mock("./settings-panel", () => ({
  SettingsPanel: (props: Record<string, unknown>) => {
    mockSettingsPanelSpy(props);
    return <div data-testid="settings-panel" />;
  },
}));

jest.mock("./desktop-mcp-panel", () => ({
  DesktopMcpPanel: (props: Record<string, unknown>) => {
    mockDesktopMcpPanelSpy(props);
    return <div data-testid="desktop-mcp-panel" />;
  },
}));

jest.mock("./guest-save-panel", () => ({
  GuestSavePanel: (props: Record<string, unknown>) => {
    mockGuestSavePanelSpy(props);
    return (
      <div data-testid="guest-save-panel">
        <button
          type="button"
          onClick={() => {
            const callback =
              typeof props.onLoadSave === "function" ? (props.onLoadSave as () => void) : null;
            callback?.();
          }}
        >
          Mock Reload Save
        </button>
      </div>
    );
  },
}));

jest.mock("./visual-debug-panel", () => ({
  VisualDebugPanel: (props: Record<string, unknown>) => (
    <div data-testid="visual-debug-panel">{props.game ? "connected" : "disconnected"}</div>
  ),
}));

jest.mock("./debug-log-panel", () => ({
  DebugLogPanel: () => <div data-testid="debug-log-panel" />,
}));

jest.mock("@/components/multiplayer-menu", () => ({
  MultiplayerMenu: (props: Record<string, unknown>) => {
    mockMultiplayerMenuSpy(props);
    return <div data-testid="multiplayer-menu" />;
  },
}));

jest.mock("@/lib/supabase/browser", () => ({
  createSupabaseBrowserClient: jest.fn(),
}));

jest.mock("@pokecrystal/core/multiplayer/overworld-presence", () => ({
  OverworldPresenceManager: function MockOverworldPresenceManager(this: any) {
    this.remotePlayersCallbacks = [];
    this.interactionRequestCallbacks = [];
    this.interactionResponseCallbacks = [];
    this.connect = jest.fn(async () => {});
    this.updateLocalState = jest.fn(async () => {});
    this.disconnect = jest.fn(async () => {});
    this.sendInteractionRequest = jest.fn(async (_toUserId: string, _kind: string) => "req-1");
    this.sendInteractionResponse = jest.fn(async () => {});
    this.onRemotePlayersChange = (callback: (players: unknown[]) => void) => {
      this.remotePlayersCallbacks.push(callback);
    };
    this.offRemotePlayersChange = (callback: (players: unknown[]) => void) => {
      this.remotePlayersCallbacks = this.remotePlayersCallbacks.filter((entry: unknown) => entry !== callback);
    };
    this.onInteractionRequest = (callback: (request: unknown) => void) => {
      this.interactionRequestCallbacks.push(callback);
    };
    this.offInteractionRequest = (callback: (request: unknown) => void) => {
      this.interactionRequestCallbacks = this.interactionRequestCallbacks.filter((entry: unknown) => entry !== callback);
    };
    this.onInteractionResponse = (callback: (response: unknown) => void) => {
      this.interactionResponseCallbacks.push(callback);
    };
    this.offInteractionResponse = (callback: (response: unknown) => void) => {
      this.interactionResponseCallbacks = this.interactionResponseCallbacks.filter((entry: unknown) => entry !== callback);
    };
    this.emitInteractionRequest = (request: unknown) => {
      this.interactionRequestCallbacks.forEach((callback: (request: unknown) => void) => callback(request));
    };
    this.emitInteractionResponse = (response: unknown) => {
      this.interactionResponseCallbacks.forEach((callback: (response: unknown) => void) => callback(response));
    };
    mockPresenceManagers.push(this);
  },
}));

jest.mock("@pokecrystal/core/multiplayer/webrtc-connection", () => ({
  WebRTCConnection: function MockWebRTCConnection(
    this: any,
    config: { matchId: string; isHost: boolean }
  ) {
    this.config = config;
    this.onDataCallbacks = [];
    this.statusCallbacks = {};
    this.send = jest.fn();
    this.destroy = jest.fn();
    this.onData = (callback: (message: { type: string; data: unknown }) => void) => {
      this.onDataCallbacks.push(callback);
    };
    this.offData = (callback: (message: { type: string; data: unknown }) => void) => {
      this.onDataCallbacks = this.onDataCallbacks.filter((entry: unknown) => entry !== callback);
    };
    this.onStatus = (callbacks: {
      onConnect?: () => void;
      onDisconnect?: () => void;
      onError?: (error: Error) => void;
    }) => {
      this.statusCallbacks = callbacks;
    };
    this.emitConnect = () => {
      this.statusCallbacks.onConnect?.();
    };
    this.emitDisconnect = () => {
      this.statusCallbacks.onDisconnect?.();
    };
    this.emitError = (error: Error) => {
      this.statusCallbacks.onError?.(error);
    };
    this.emitData = (message: { type: string; data: unknown }) => {
      this.onDataCallbacks.forEach((callback: (message: { type: string; data: unknown }) => void) => callback(message));
    };
    mockWebRtcConnections.push(this);
  },
}));

jest.mock("@pokecrystal/core/multiplayer/link-cable", () => ({
  LinkCableEmulator: jest.fn().mockImplementation(() => ({})),
}));

jest.mock("@pokecrystal/core/multiplayer/trade-manager", () => ({
  TradeManager: jest.fn().mockImplementation(() => ({
    trade: mockTrade,
  })),
}));

const flushPromises = async (): Promise<void> => {
  await Promise.resolve();
};

const findButtonByLabel = (container: HTMLElement, label: string): HTMLButtonElement | undefined =>
  Array.from(container.querySelectorAll("button")).find(
    (button) => button.textContent?.trim() === label
  ) as HTMLButtonElement | undefined;

const createFakeGame = () => {
  const localParty = [
    {
      species: { id: "CHIKORITA", int_id: 152 },
      nickname: "CHIKO",
      original_trainer_name: "RYAN",
      original_trainer_id: 1,
      moves: [{ name: "TACKLE", current_pp: 35 }],
      level: 5,
      hp: 20,
      max_hp: 20,
      attack: 10,
      defense: 10,
      speed: 10,
      special_attack: 10,
      special_defense: 10,
      dvs: { attack: 1, defense: 1, speed: 1, special: 1, hp: 1 },
      experience: 100,
      happiness: 70,
      hp_exp: 0,
      attack_exp: 0,
      defense_exp: 0,
      speed_exp: 0,
      special_exp: 0,
      pokerus: false,
    },
  ];

  return {
    getGameState: jest.fn(() => ({
      sram: {
        player_gender: "male",
        player_name: "Ryan",
        day_of_week: 2,
      },
      wram: {
        time_of_day: "day",
        instant_mode: false,
      },
    })),
    getCurrentMapName: jest.fn(() => "New Bark Town"),
    getOverworld: jest.fn(() => ({
      player_x: 4,
      player_y: 8,
      player_direction: "down",
    })),
    setOverworldRemoteRenderEnabled: jest.fn(),
    setOverworldRemoteCrowdView: jest.fn(),
    setOverworldRemotePlayers: jest.fn(),
    setAudioMuted: jest.fn(),
    setTimeOfDay: jest.fn(),
    setDayOfWeek: jest.fn(),
    setPlayerName: jest.fn(),
    clearMultiplayerBattleTransport: jest.fn(),
    clearOverworldRemotePlayers: jest.fn(),
    getPartyPokemon: jest.fn(() => localParty),
    getFirstPartyPokemon: jest.fn(() => ({ index: 0, pokemon: localParty[0] })),
    setMultiplayerBattleTransport: jest.fn(),
    onMultiplayerBattleComplete: jest.fn(),
    startMultiplayerBattle: jest.fn(),
    replacePartyPokemon: jest.fn(),
  };
};

beforeEach(() => {
  mockCreateSupabaseBrowserClient.mockReset();
  mockCreateSupabaseBrowserClient.mockReturnValue(null);
  mockPresenceManagers.length = 0;
  mockWebRtcConnections.length = 0;
  mockTrade.mockReset();
});

describe("PlayPanel controls dialog", () => {
  beforeAll(() => {
    (globalThis as any).IS_REACT_ACT_ENVIRONMENT = true;
    if (!window.matchMedia) {
      window.matchMedia = jest.fn().mockImplementation(() => ({
        matches: false,
        addEventListener: jest.fn(),
        removeEventListener: jest.fn(),
        addListener: jest.fn(),
        removeListener: jest.fn(),
        dispatchEvent: jest.fn(),
      }));
    }
  });

  afterAll(() => {
    delete (globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT;
  });

  beforeEach(() => {
    mockSettingsPanelSpy.mockClear();
    mockDesktopMcpPanelSpy.mockClear();
    mockMultiplayerMenuSpy.mockClear();
    mockGameCanvasSpy.mockClear();
    mockGuestSavePanelSpy.mockClear();
    act(() => {
      useMultiplayerStore.getState().reset();
    });
    consoleWarnSpy = jest.spyOn(console, "warn").mockImplementation(() => undefined);
    consoleErrorSpy = jest.spyOn(console, "error").mockImplementation(() => undefined);
  });

  afterEach(() => {
    consoleWarnSpy.mockRestore();
    consoleErrorSpy.mockRestore();
  });

  it("renders the play console quick actions", async () => {
    const container = document.createElement("div");
    document.body.appendChild(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(<PlayPanel />);
      await flushPromises();
    });

    expect(container.textContent).toContain("Play Console");
    expect(container.textContent).toContain("Renderer: Pixel");

    await act(async () => {
      root.unmount();
    });
    container.remove();
  });

  it("opens the debug utility panel", async () => {
    const container = document.createElement("div");
    document.body.appendChild(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(<PlayPanel />);
      await flushPromises();
    });

    const debugButton = findButtonByLabel(container, "Debug");
    expect(debugButton).toBeTruthy();

    await act(async () => {
      debugButton?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await flushPromises();
    });

    expect(document.body.querySelector('[data-testid="visual-debug-panel"]')).toBeTruthy();

    await act(async () => {
      root.unmount();
    });
    container.remove();
  });

  it("omits lobby and debug from the desktop variant side menu", async () => {
    window.localStorage.setItem("pokecrystal.desktop.sidebarVisible", "true");
    const container = document.createElement("div");
    document.body.appendChild(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(<PlayPanel variant="desktop" />);
      await flushPromises();
    });

    expect(findButtonByLabel(container, "Lobby")).toBeUndefined();
    expect(findButtonByLabel(container, "MCP")).toBeUndefined();
    expect(findButtonByLabel(container, "Settings")).toBeTruthy();
    expect(findButtonByLabel(container, "Saves")).toBeTruthy();
    expect(findButtonByLabel(container, "Debug")).toBeUndefined();
    expect(container.querySelector('[data-testid="desktop-sidebar"]')?.className).toContain("w-[28rem]");
    expect(container.querySelector('[data-testid="settings-panel"]')).toBeTruthy();
    expect(container.querySelector('[data-testid="multiplayer-menu"]')).toBeNull();
    expect(mockGameCanvasSpy.mock.calls.at(-1)?.[0]).toMatchObject({
      runtimeMode: "server",
      remoteVisualMode: "frame",
      rendererMode: "tile",
      muted: false,
      remoteInstantMode: false,
      remoteAdvanceFrames: 1,
    });
    expect(typeof mockGameCanvasSpy.mock.calls.at(-1)?.[0]?.remoteRefreshMs).toBe("number");
    const instantButton = findButtonByLabel(container, "Instant Off");
    expect(instantButton).toBeTruthy();
    await act(async () => {
      instantButton?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await flushPromises();
    });
    expect(findButtonByLabel(container, "Instant On")).toBeTruthy();
    expect(mockGameCanvasSpy.mock.calls.at(-1)?.[0]).toMatchObject({
      runtimeMode: "server",
      remoteVisualMode: "frame",
      remoteInstantMode: true,
      remoteAdvanceFrames: 0,
      remoteRefreshMs: 100,
    });

    const soundButton = findButtonByLabel(container, "Sound On");
    expect(soundButton).toBeTruthy();
    await act(async () => {
      soundButton?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await flushPromises();
    });
    expect(findButtonByLabel(container, "Sound Muted")).toBeTruthy();
    expect(mockGameCanvasSpy.mock.calls.at(-1)?.[0]).toMatchObject({
      runtimeMode: "server",
      remoteVisualMode: "frame",
      muted: true,
    });

    const rendererButton = findButtonByLabel(container, "Show Tile + Text");
    expect(rendererButton).toBeTruthy();
    await act(async () => {
      rendererButton?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await flushPromises();
    });
    expect(mockGameCanvasSpy.mock.calls.at(-1)?.[0]).toMatchObject({
      runtimeMode: "server",
      remoteVisualMode: "frame",
      rendererMode: "both",
    });

    await act(async () => {
      root.unmount();
    });
    container.remove();
    window.localStorage.removeItem("pokecrystal.desktop.sidebarVisible");
  });

  it("opens MCP configuration from the desktop panel URL", async () => {
    window.localStorage.removeItem("pokecrystal.desktop.sidebarVisible");
    window.history.replaceState({}, "", "/desktop?panel=mcp");
    const container = document.createElement("div");
    document.body.appendChild(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(<PlayPanel variant="desktop" />);
      await flushPromises();
    });

    expect(container.querySelector('[data-testid="desktop-sidebar"]')).toBeNull();
    expect(container.textContent).toContain("MCP Streamable HTTP");
    expect(container.querySelector('[data-testid="desktop-mcp-panel"]')).toBeTruthy();
    expect(mockDesktopMcpPanelSpy).toHaveBeenCalled();

    const closeButton = Array.from(container.querySelectorAll("button")).find(
      (button) => button.textContent?.trim() === "Close"
    );
    expect(closeButton).toBeTruthy();

    await act(async () => {
      closeButton?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await flushPromises();
    });

    expect(window.location.pathname).toBe("/desktop");
    expect(window.location.search).toBe("");
    expect(container.querySelector('[data-testid="desktop-mcp-panel"]')).toBeNull();

    await act(async () => {
      root.unmount();
    });
    container.remove();
    window.history.replaceState({}, "", "/desktop");
  });

  it("keeps desktop animated by default even when stored user settings had instant mode", async () => {
    window.localStorage.setItem("pokecrystal.desktop.sidebarVisible", "true");
    window.localStorage.removeItem("pokecrystal.desktop.instantMode");
    const upsert = jest.fn(async () => ({ error: null }));
    const maybeSingle = jest.fn(async () => ({
      data: {
        user_id: "desktop-user",
        player_name: "Misty",
        player_gender: 1,
        time_of_day: "DAY",
        sound_enabled: true,
        instant_mode_enabled: true,
        brand_theme: "krabby",
      },
      error: null,
    }));
    const eq = jest.fn(() => ({ maybeSingle }));
    const select = jest.fn(() => ({ eq }));
    mockCreateSupabaseBrowserClient.mockReturnValue({
      auth: {
        getUser: jest.fn(async () => ({ data: { user: { id: "desktop-user" } } })),
        onAuthStateChange: jest.fn(() => ({ data: { subscription: { unsubscribe: jest.fn() } } })),
      },
      from: jest.fn(() => ({ select, upsert })),
    } as any);
    const container = document.createElement("div");
    document.body.appendChild(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(<PlayPanel variant="desktop" />);
      await flushPromises();
      await flushPromises();
      await flushPromises();
    });

    expect(findButtonByLabel(container, "Instant Off")).toBeTruthy();
    expect(mockGameCanvasSpy.mock.calls.at(-1)?.[0]).toMatchObject({
      runtimeMode: "server",
      remoteVisualMode: "frame",
      remoteInstantMode: false,
      remoteAdvanceFrames: 1,
    });

    await act(async () => {
      root.unmount();
    });
    container.remove();
    window.localStorage.removeItem("pokecrystal.desktop.sidebarVisible");
  });

  it("hides and restores the desktop variant sidebar", async () => {
    window.localStorage.removeItem("pokecrystal.desktop.sidebarVisible");
    const container = document.createElement("div");
    document.body.appendChild(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(<PlayPanel variant="desktop" />);
      await flushPromises();
    });

    expect(container.querySelector('[data-testid="desktop-sidebar"]')).toBeNull();
    expect(container.querySelector('[data-testid="game-canvas"]')).toBeTruthy();
    expect(findButtonByLabel(container, "Show Sidebar")).toBeTruthy();

    const initialShowButton = findButtonByLabel(container, "Show Sidebar");
    await act(async () => {
      initialShowButton?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await flushPromises();
    });

    expect(container.querySelector('[data-testid="desktop-sidebar"]')).toBeTruthy();
    expect(window.localStorage.getItem("pokecrystal.desktop.sidebarVisible")).toBe("true");

    const hideButton = container.querySelector('button[aria-label="Hide sidebar"]') as HTMLButtonElement | null;
    expect(hideButton).toBeTruthy();
    await act(async () => {
      hideButton?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await flushPromises();
    });

    expect(container.querySelector('[data-testid="desktop-sidebar"]')).toBeNull();
    expect(findButtonByLabel(container, "Show Sidebar")).toBeTruthy();
    expect(window.localStorage.getItem("pokecrystal.desktop.sidebarVisible")).toBe("false");

    const showButton = findButtonByLabel(container, "Show Sidebar");
    await act(async () => {
      showButton?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await flushPromises();
    });

    expect(container.querySelector('[data-testid="desktop-sidebar"]')).toBeTruthy();
    expect(window.localStorage.getItem("pokecrystal.desktop.sidebarVisible")).toBe("true");

    await act(async () => {
      root.unmount();
    });
    container.remove();
    window.localStorage.removeItem("pokecrystal.desktop.sidebarVisible");
  });

  it("opens the controls dialog and renders keycaps", async () => {
    const container = document.createElement("div");
    document.body.appendChild(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(<PlayPanel />);
      await flushPromises();
    });

    expect(document.body.querySelector('[data-keycap="Z"]')).toBeNull();

    const controlsButton = Array.from(container.querySelectorAll("button")).find(
      (button) => button.textContent?.trim() === "Controls"
    );
    expect(controlsButton).toBeTruthy();

    await act(async () => {
      controlsButton?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await flushPromises();
    });

    expect(document.body.querySelector('[data-keycap="Z"]')).not.toBeNull();
    expect(document.body.querySelector('[data-keycap="Space"]')).not.toBeNull();
    expect(document.body.querySelector('[data-keycap="Enter"]')).not.toBeNull();

    await act(async () => {
      root.unmount();
    });
    container.remove();
  });
});

describe("PlayPanel start gate", () => {
  beforeAll(() => {
    (globalThis as any).IS_REACT_ACT_ENVIRONMENT = true;
    if (!window.matchMedia) {
      window.matchMedia = jest.fn().mockImplementation(() => ({
        matches: false,
        addEventListener: jest.fn(),
        removeEventListener: jest.fn(),
        addListener: jest.fn(),
        removeListener: jest.fn(),
        dispatchEvent: jest.fn(),
      }));
    }
  });

  afterAll(() => {
    delete (globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT;
  });

  beforeEach(() => {
    mockSettingsPanelSpy.mockClear();
    mockGameCanvasSpy.mockClear();
    window.localStorage.clear();
  });

  it("starts immediately when right-to-game is enabled by default", async () => {
    window.localStorage.setItem("pokecrystal.play.playIntro", "false");

    const container = document.createElement("div");
    document.body.appendChild(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(
        <PlayPanel />
      );
      await flushPromises();
      await flushPromises();
    });

    expect(container.querySelector('[data-testid="game-canvas"]')).toBeTruthy();

    const startButton = Array.from(container.querySelectorAll("button")).find(
      (button) => button.textContent?.trim() === "Start Game"
    );
    expect(startButton).toBeFalsy();

    const gameCanvasCall = mockGameCanvasSpy.mock.calls.at(-1)?.[0];
    expect(gameCanvasCall?.loadSlot).toBe(MANUAL_SAVE_SLOT);
    expect(gameCanvasCall?.playIntro).toBe(false);
    expect(gameCanvasCall?.newGame).toBe(false);
    expect(gameCanvasCall?.autoStart).toBe(true);

    await act(async () => {
      root.unmount();
    });
    container.remove();
  });

  it("keeps quick actions above the start gate overlay", async () => {
    window.localStorage.setItem("pokecrystal.play.playIntro", "true");

    const container = document.createElement("div");
    document.body.appendChild(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(<PlayPanel />);
      await flushPromises();
    });

    expect(findButtonByLabel(container, "Start Game")).toBeTruthy();
    const quickActions = container.querySelector('[data-testid="play-quick-actions"]');
    expect(quickActions?.className).toContain("z-[4]");

    await act(async () => {
      root.unmount();
    });
    container.remove();
  });

  it("keeps tile renderer as default even on wide screens", async () => {
    window.localStorage.setItem("pokecrystal.play.playIntro", "false");
    const originalMatchMedia = window.matchMedia;
    window.matchMedia = jest.fn().mockImplementation((query: string) => ({
      matches: query === "(min-width: 1200px)",
      media: query,
      onchange: null,
      addEventListener: jest.fn(),
      removeEventListener: jest.fn(),
      addListener: jest.fn(),
      removeListener: jest.fn(),
      dispatchEvent: jest.fn(),
    }));

    const container = document.createElement("div");
    document.body.appendChild(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(<PlayPanel />);
      await flushPromises();
      await flushPromises();
    });

    const gameCanvasCall = mockGameCanvasSpy.mock.calls.at(-1)?.[0];
    expect(gameCanvasCall?.loadSlot).toBe(MANUAL_SAVE_SLOT);
    expect(gameCanvasCall?.rendererMode).toBe("tile");

    await act(async () => {
      root.unmount();
    });
    container.remove();
    window.matchMedia = originalMatchMedia;
  });

  it("shows text-view toggle in mobile gamepad system controls and applies it", async () => {
    window.localStorage.setItem("pokecrystal.play.playIntro", "false");

    const originalMatchMedia = window.matchMedia;
    window.matchMedia = jest.fn().mockImplementation((query: string) => ({
      matches: query.includes("max-width"),
      media: query,
      onchange: null,
      addEventListener: jest.fn(),
      removeEventListener: jest.fn(),
      addListener: jest.fn(),
      removeListener: jest.fn(),
      dispatchEvent: jest.fn(),
    }));

    const container = document.createElement("div");
    document.body.appendChild(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(<PlayPanel />);
      await flushPromises();
    });

    const gamepad = container.querySelector('[data-testid="virtual-gamepad"]');
    const textViewButton = Array.from(gamepad?.querySelectorAll("button") ?? []).find((button) =>
      (button.textContent?.trim() ?? "").startsWith("Show ")
    );
    expect(textViewButton).toBeTruthy();

    await act(async () => {
      textViewButton?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await flushPromises();
    });
    const secondRendererButton = Array.from(gamepad?.querySelectorAll("button") ?? []).find(
      (button) => button.textContent?.trim() === "Show Text View"
    );
    await act(async () => {
      secondRendererButton?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await flushPromises();
    });

    const gameCanvasCall = mockGameCanvasSpy.mock.calls.at(-1)?.[0];
    expect(gameCanvasCall?.loadSlot).toBe(MANUAL_SAVE_SLOT);
    expect(gameCanvasCall?.rendererMode).toBe("text");
    expect(gameCanvasCall?.preloadMode).toBe("auto");

    await act(async () => {
      root.unmount();
    });
    container.remove();
    window.matchMedia = originalMatchMedia;
    window.localStorage.removeItem("pokecrystal.play.playIntro");
  });

  it("collapses the play console into a mobile menu button on compact layouts", async () => {
    window.localStorage.setItem("pokecrystal.play.playIntro", "false");

    const originalMatchMedia = window.matchMedia;
    window.matchMedia = jest.fn().mockImplementation((query: string) => ({
      matches: query.includes("max-width"),
      media: query,
      onchange: null,
      addEventListener: jest.fn(),
      removeEventListener: jest.fn(),
      addListener: jest.fn(),
      removeListener: jest.fn(),
      dispatchEvent: jest.fn(),
    }));

    const container = document.createElement("div");
    document.body.appendChild(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(<PlayPanel />);
      await flushPromises();
      await flushPromises();
    });

    expect(container.textContent).not.toContain("Play Console");
    const menuButton = findButtonByLabel(container, "Menu");
    expect(menuButton).toBeTruthy();

    await act(async () => {
      menuButton?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await flushPromises();
    });

    expect(document.body.textContent).toContain("Game Options");

    await act(async () => {
      root.unmount();
    });
    container.remove();
    window.matchMedia = originalMatchMedia;
    window.localStorage.removeItem("pokecrystal.play.playIntro");
  });

  it("forces the canonical manual slot even when NEXT_PUBLIC_LOAD_SLOT is set", async () => {
    const originalEnv = process.env.NEXT_PUBLIC_LOAD_SLOT;
    process.env.NEXT_PUBLIC_LOAD_SLOT = "slot-01";
    window.localStorage.setItem("pokecrystal.play.playIntro", "false");

    const container = document.createElement("div");
    document.body.appendChild(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(<PlayPanel />);
      await flushPromises();
      await flushPromises();
    });

    const gameCanvasCall = mockGameCanvasSpy.mock.calls.at(-1)?.[0];
    expect(gameCanvasCall?.loadSlot).toBe(MANUAL_SAVE_SLOT);

    await act(async () => {
      root.unmount();
    });
    container.remove();
    window.localStorage.removeItem("pokecrystal.play.playIntro");
    if (originalEnv === undefined) {
      delete process.env.NEXT_PUBLIC_LOAD_SLOT;
    } else {
      process.env.NEXT_PUBLIC_LOAD_SLOT = originalEnv;
    }
  });

  it("reloads the game canvas from the canonical manual slot", async () => {
    window.localStorage.setItem("pokecrystal.play.playIntro", "false");

    const container = document.createElement("div");
    document.body.appendChild(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(<PlayPanel />);
      await flushPromises();
      await flushPromises();
    });

    const savesButton = findButtonByLabel(container, "Saves");
    expect(savesButton).toBeTruthy();

    await act(async () => {
      savesButton?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await flushPromises();
    });

    const initialCallCount = mockGameCanvasSpy.mock.calls.length;
    const reloadButton = Array.from(document.body.querySelectorAll("button")).find(
      (button) => button.textContent?.trim() === "Mock Reload Save"
    );
    expect(reloadButton).toBeTruthy();

    await act(async () => {
      reloadButton?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await flushPromises();
      await flushPromises();
    });

    expect(mockGameCanvasSpy.mock.calls.length).toBeGreaterThan(initialCallCount);
    const gameCanvasCall = mockGameCanvasSpy.mock.calls.at(-1)?.[0];
    expect(gameCanvasCall?.loadSlot).toBe(MANUAL_SAVE_SLOT);

    await act(async () => {
      root.unmount();
    });
    container.remove();
    window.localStorage.removeItem("pokecrystal.play.playIntro");
  });
});

describe("PlayPanel settings panel", () => {
  beforeAll(() => {
    (globalThis as any).IS_REACT_ACT_ENVIRONMENT = true;
    if (!window.matchMedia) {
      window.matchMedia = jest.fn().mockImplementation(() => ({
        matches: false,
        addEventListener: jest.fn(),
        removeEventListener: jest.fn(),
        addListener: jest.fn(),
        removeListener: jest.fn(),
        dispatchEvent: jest.fn(),
      }));
    }
  });

  afterAll(() => {
    delete (globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT;
  });

  beforeEach(() => {
    mockSettingsPanelSpy.mockClear();
    mockGameCanvasSpy.mockClear();
  });

  it("toggles brand theme in settings panel state without exposing removed toggles", async () => {
    const container = document.createElement("div");
    document.body.appendChild(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(<PlayPanel />);
      await flushPromises();
    });

    const settingsButton = findButtonByLabel(container, "Settings");
    expect(settingsButton).toBeTruthy();
    await act(async () => {
      settingsButton?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await flushPromises();
    });

    const lastCall = mockSettingsPanelSpy.mock.calls.at(-1)?.[0];
    expect(lastCall).not.toHaveProperty("instantModeEnabled");
    expect(lastCall).not.toHaveProperty("onInstantModeEnabledChange");
    expect(lastCall).not.toHaveProperty("soundEnabled");
    expect(lastCall).not.toHaveProperty("onSoundEnabledChange");
    expect(lastCall).not.toHaveProperty("playIntroEnabled");
    expect(lastCall).not.toHaveProperty("onPlayIntroEnabledChange");
    expect(lastCall?.brandTheme).toBe("krabby");
    expect(document.documentElement.getAttribute("data-brand-theme")).toBe("krabby");
    const initialFavicon = document.querySelector<HTMLLinkElement>('link[rel~="icon"]');
    expect(initialFavicon?.href ?? "").toMatch(/\/favicon\.png|^data:image\/png;base64,/);

    await act(async () => {
      (lastCall?.onBrandThemeChange as ((theme: string) => void) | undefined)?.("gligar");
      await flushPromises();
    });

    const updatedCall = mockSettingsPanelSpy.mock.calls.at(-1)?.[0];
    expect(updatedCall).not.toHaveProperty("instantModeEnabled");
    expect(updatedCall).not.toHaveProperty("onInstantModeEnabledChange");
    expect(updatedCall?.brandTheme).toBe("gligar");
    expect(document.documentElement.getAttribute("data-brand-theme")).toBe("gligar");
    const updatedFavicon = document.querySelector<HTMLLinkElement>('link[rel~="icon"]');
    expect(updatedFavicon?.href ?? "").toMatch(/\/favicon\.png|^data:image\/png;base64,/);

    await act(async () => {
      root.unmount();
    });
    container.remove();
  });

  it("starts immediately when skip-to-play is enabled", async () => {
    window.localStorage.setItem("pokecrystal.play.playIntro", "false");
    const container = document.createElement("div");
    document.body.appendChild(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(<PlayPanel />);
      await flushPromises();
      await flushPromises();
    });

    const settingsButton = findButtonByLabel(container, "Settings");
    expect(settingsButton).toBeTruthy();
    await act(async () => {
      settingsButton?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await flushPromises();
    });

    const settingsCall = mockSettingsPanelSpy.mock.calls.at(-1)?.[0];
    expect(settingsCall).not.toHaveProperty("playIntroEnabled");
    expect(settingsCall).not.toHaveProperty("onPlayIntroEnabledChange");
    expect(container.querySelector('[data-testid="game-canvas"]')).toBeTruthy();
    const hasStartButton = Array.from(container.querySelectorAll("button")).some(
      (button) => button.textContent?.trim() === "Start Game"
    );
    expect(hasStartButton).toBe(false);

    const gameCanvasCall = mockGameCanvasSpy.mock.calls.at(-1)?.[0];
    expect(gameCanvasCall?.autoStart).toBe(true);
    expect(gameCanvasCall?.playIntro).toBe(false);
    expect(gameCanvasCall?.newGame).toBe(false);
    expect(gameCanvasCall?.preloadMode).toBe("auto");

    await act(async () => {
      root.unmount();
    });
    container.remove();
    window.localStorage.removeItem("pokecrystal.play.playIntro");
  });

  it("starts from title screen flow when skip-to-play is disabled", async () => {
    window.localStorage.setItem("pokecrystal.play.playIntro", "true");
    const container = document.createElement("div");
    document.body.appendChild(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(<PlayPanel />);
      await flushPromises();
      await flushPromises();
    });

    const settingsButton = findButtonByLabel(container, "Settings");
    expect(settingsButton).toBeTruthy();
    await act(async () => {
      settingsButton?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await flushPromises();
    });

    const settingsCall = mockSettingsPanelSpy.mock.calls.at(-1)?.[0];
    expect(settingsCall).not.toHaveProperty("playIntroEnabled");
    expect(settingsCall).not.toHaveProperty("onPlayIntroEnabledChange");
    const startButton = Array.from(container.querySelectorAll("button")).find(
      (button) => button.textContent?.trim() === "Start Game"
    );
    expect(startButton).toBeTruthy();

    await act(async () => {
      startButton?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await flushPromises();
    });

    const gameCanvasCall = mockGameCanvasSpy.mock.calls.at(-1)?.[0];
    expect(gameCanvasCall?.autoStart).toBe(true);
    expect(gameCanvasCall?.playIntro).toBe(true);
    expect(gameCanvasCall?.newGame).toBe(true);
    expect(gameCanvasCall?.preloadMode).toBe("auto");

    await act(async () => {
      root.unmount();
    });
    container.remove();
    window.localStorage.removeItem("pokecrystal.play.playIntro");
  });

  it("does not auto-start when skip-to-play is turned off", async () => {
    window.localStorage.setItem("pokecrystal.play.playIntro", "true");
    const container = document.createElement("div");
    document.body.appendChild(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(<PlayPanel />);
      await flushPromises();
      await flushPromises();
    });

    const startButton = Array.from(container.querySelectorAll("button")).find(
      (button) => button.textContent?.trim() === "Start Game"
    );
    expect(startButton).toBeTruthy();

    const settingsButton = findButtonByLabel(container, "Settings");
    expect(settingsButton).toBeTruthy();
    await act(async () => {
      settingsButton?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await flushPromises();
    });

    const settingsCall = mockSettingsPanelSpy.mock.calls.at(-1)?.[0];
    expect(settingsCall).not.toHaveProperty("playIntroEnabled");
    expect(settingsCall).not.toHaveProperty("onPlayIntroEnabledChange");
    mockGameCanvasSpy.mockClear();

    const postToggleStartButton = Array.from(container.querySelectorAll("button")).find(
      (button) => button.textContent?.trim() === "Start Game"
    );
    expect(postToggleStartButton).toBeTruthy();
    expect(mockGameCanvasSpy).not.toHaveBeenCalled();

    await act(async () => {
      root.unmount();
    });
    container.remove();
    window.localStorage.removeItem("pokecrystal.play.playIntro");
  });

  it("loads and saves settings using authenticated Supabase user", async () => {
    const upsert = jest.fn(async () => ({ error: null }));
    const maybeSingle = jest.fn(async () => ({
      data: {
        user_id: "user-1",
        player_name: "Misty",
        player_gender: 1,
        time_of_day: "NIGHT",
        sound_enabled: true,
        instant_mode_enabled: true,
        brand_theme: "heracross",
      },
      error: null,
    }));
    const eq = jest.fn(() => ({ maybeSingle }));
    const select = jest.fn(() => ({ eq }));
    const from = jest.fn((table: string) => {
      if (table === "play_user_settings") {
        return { select, upsert };
      }
      throw new Error(`Unexpected table ${table}`);
    });
    const unsubscribe = jest.fn();
    mockCreateSupabaseBrowserClient.mockReturnValue({
      auth: {
        getUser: jest.fn(async () => ({ data: { user: { id: "user-1" } } })),
        onAuthStateChange: jest.fn(() => ({ data: { subscription: { unsubscribe } } })),
      },
      from,
    } as any);

    const container = document.createElement("div");
    document.body.appendChild(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(<PlayPanel />);
      await flushPromises();
      await flushPromises();
    });

    const settingsButton = findButtonByLabel(container, "Settings");
    expect(settingsButton).toBeTruthy();
    await act(async () => {
      settingsButton?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await flushPromises();
      await flushPromises();
    });

    const lastCall = mockSettingsPanelSpy.mock.calls.at(-1)?.[0];
    expect(lastCall?.playerName).toBe("Misty");
    expect(lastCall?.brandTheme).toBe("heracross");

    await act(async () => {
      (lastCall?.onBrandThemeChange as ((theme: BrandTheme) => void) | undefined)?.("pinsir");
      await flushPromises();
    });

    expect(select).toHaveBeenCalled();
    expect(eq).toHaveBeenCalledWith("user_id", "user-1");
    expect(upsert).toHaveBeenCalled();
    expect(upsert.mock.calls.at(-1)?.[0]).toMatchObject({
      user_id: "user-1",
      player_name: "Misty",
      brand_theme: "pinsir",
    });

    await act(async () => {
      root.unmount();
    });
    expect(unsubscribe).toHaveBeenCalledTimes(1);
    container.remove();
  });
});

describe("PlayPanel loading overlay progress", () => {
  beforeAll(() => {
    (globalThis as any).IS_REACT_ACT_ENVIRONMENT = true;
    if (!window.matchMedia) {
      window.matchMedia = jest.fn().mockImplementation(() => ({
        matches: false,
        addEventListener: jest.fn(),
        removeEventListener: jest.fn(),
        addListener: jest.fn(),
        removeListener: jest.fn(),
        dispatchEvent: jest.fn(),
      }));
    }
  });

  afterAll(() => {
    delete (globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT;
  });

  beforeEach(() => {
    mockSettingsPanelSpy.mockClear();
    mockGameCanvasSpy.mockClear();
  });

  afterEach(() => {
    jest.useRealTimers();
  });

  it("keeps startup progress below 100% until ready", async () => {
    window.localStorage.setItem("pokecrystal.play.playIntro", "false");
    const container = document.createElement("div");
    document.body.appendChild(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(<PlayPanel />);
      await flushPromises();
      await flushPromises();
    });

    const gameCanvasCall = mockGameCanvasSpy.mock.calls.at(-1)?.[0] as
      | { onLoadProgress?: (progress: { phase: string; ratio: number; completed: number; total: number }) => void }
      | undefined;
    expect(gameCanvasCall?.onLoadProgress).toBeTruthy();

    await act(async () => {
      gameCanvasCall?.onLoadProgress?.({
        phase: "core-assets",
        completed: 300,
        total: 300,
        ratio: 1,
      });
      await flushPromises();
    });
    expect(container.textContent).toContain("75% complete");

    await act(async () => {
      gameCanvasCall?.onLoadProgress?.({
        phase: "core-data",
        completed: 3,
        total: 4,
        ratio: 0.75,
      });
      await flushPromises();
    });
    expect(container.textContent).toContain("90% complete");

    await act(async () => {
      gameCanvasCall?.onLoadProgress?.({
        phase: "ready",
        completed: 1,
        total: 1,
        ratio: 1,
      });
      await flushPromises();
    });
    expect(container.textContent).not.toContain("% complete");

    await act(async () => {
      root.unmount();
    });
    container.remove();
    window.localStorage.removeItem("pokecrystal.play.playIntro");
  });

  it("shows which asset path is currently loading", async () => {
    window.localStorage.setItem("pokecrystal.play.playIntro", "false");
    const container = document.createElement("div");
    document.body.appendChild(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(<PlayPanel />);
      await flushPromises();
      await flushPromises();
    });

    const gameCanvasCall = mockGameCanvasSpy.mock.calls.at(-1)?.[0] as
      | { onLoadProgress?: (progress: { phase: string; ratio: number; completed: number; total: number; label?: string }) => void }
      | undefined;
    expect(gameCanvasCall?.onLoadProgress).toBeTruthy();

    await act(async () => {
      gameCanvasCall?.onLoadProgress?.({
        phase: "core-assets",
        completed: 42,
        total: 300,
        ratio: 0.14,
        label: "/assets/gfx/sprites/chris.png",
      });
      await flushPromises();
    });

    expect(container.textContent).toContain("Loading gfx/sprites/chris.png");

    await act(async () => {
      root.unmount();
    });
    container.remove();
    window.localStorage.removeItem("pokecrystal.play.playIntro");
  });

  it("smooths visible progress if startup stalls after core assets finish", async () => {
    jest.useFakeTimers();
    window.localStorage.setItem("pokecrystal.play.playIntro", "false");
    const container = document.createElement("div");
    document.body.appendChild(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(<PlayPanel />);
      await flushPromises();
      await flushPromises();
    });

    const gameCanvasCall = mockGameCanvasSpy.mock.calls.at(-1)?.[0] as
      | { onLoadProgress?: (progress: { phase: string; ratio: number; completed: number; total: number }) => void }
      | undefined;
    expect(gameCanvasCall?.onLoadProgress).toBeTruthy();

    await act(async () => {
      gameCanvasCall?.onLoadProgress?.({
        phase: "core-assets",
        completed: 300,
        total: 300,
        ratio: 1,
      });
      await flushPromises();
    });
    expect(container.textContent).toContain("75% complete");

    await act(async () => {
      jest.advanceTimersByTime(8_000);
      await flushPromises();
    });

    expect(container.textContent).toContain(
      "Initializing game systems (this can take a few more seconds)"
    );
    const progressLine = Array.from(container.querySelectorAll("p")).find((element) =>
      element.textContent?.includes("% complete")
    );
    expect(progressLine?.textContent).not.toBe("75% complete");

    await act(async () => {
      root.unmount();
    });
    container.remove();
    window.localStorage.removeItem("pokecrystal.play.playIntro");
  });
});

describe("PlayPanel fullscreen mode", () => {
  const originalRequestFullscreen = Object.getOwnPropertyDescriptor(
    HTMLElement.prototype,
    "requestFullscreen"
  );
  const originalExitFullscreen = Object.getOwnPropertyDescriptor(document, "exitFullscreen");
  const originalFullscreenElement = Object.getOwnPropertyDescriptor(document, "fullscreenElement");
  const originalFullscreenEnabled = Object.getOwnPropertyDescriptor(document, "fullscreenEnabled");

  const setFullscreenElement = (element: Element | null) => {
    Object.defineProperty(document, "fullscreenElement", {
      configurable: true,
      writable: true,
      value: element,
    });
  };

  beforeAll(() => {
    (globalThis as any).IS_REACT_ACT_ENVIRONMENT = true;
    if (!window.matchMedia) {
      window.matchMedia = jest.fn().mockImplementation(() => ({
        matches: false,
        addEventListener: jest.fn(),
        removeEventListener: jest.fn(),
        addListener: jest.fn(),
        removeListener: jest.fn(),
        dispatchEvent: jest.fn(),
      }));
    }
  });

  afterAll(() => {
    if (originalRequestFullscreen) {
      Object.defineProperty(HTMLElement.prototype, "requestFullscreen", originalRequestFullscreen);
    } else {
      delete (HTMLElement.prototype as { requestFullscreen?: unknown }).requestFullscreen;
    }
    if (originalExitFullscreen) {
      Object.defineProperty(document, "exitFullscreen", originalExitFullscreen);
    } else {
      delete (document as { exitFullscreen?: unknown }).exitFullscreen;
    }
    if (originalFullscreenElement) {
      Object.defineProperty(document, "fullscreenElement", originalFullscreenElement);
    } else {
      delete (document as { fullscreenElement?: unknown }).fullscreenElement;
    }
    if (originalFullscreenEnabled) {
      Object.defineProperty(document, "fullscreenEnabled", originalFullscreenEnabled);
    } else {
      delete (document as { fullscreenEnabled?: unknown }).fullscreenEnabled;
    }
    delete (globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT;
  });

  beforeEach(() => {
    mockSettingsPanelSpy.mockClear();
    mockMultiplayerMenuSpy.mockClear();
    mockGameCanvasSpy.mockClear();
    useMultiplayerStore.getState().reset();
  });

  it("enters and exits fullscreen via the panel button", async () => {
    const requestFullscreenMock = jest.fn(function requestFullscreen(this: HTMLElement) {
      setFullscreenElement(this);
      document.dispatchEvent(new Event("fullscreenchange"));
      return Promise.resolve();
    });
    const exitFullscreenMock = jest.fn(() => {
      setFullscreenElement(null);
      document.dispatchEvent(new Event("fullscreenchange"));
      return Promise.resolve();
    });
    Object.defineProperty(HTMLElement.prototype, "requestFullscreen", {
      configurable: true,
      writable: true,
      value: requestFullscreenMock,
    });
    Object.defineProperty(document, "exitFullscreen", {
      configurable: true,
      writable: true,
      value: exitFullscreenMock,
    });
    Object.defineProperty(document, "fullscreenEnabled", {
      configurable: true,
      writable: true,
      value: true,
    });
    setFullscreenElement(null);

    const container = document.createElement("div");
    document.body.appendChild(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(<PlayPanel />);
      await flushPromises();
    });

    const fullscreenButton = Array.from(container.querySelectorAll("button")).find(
      (button) => button.textContent?.trim() === "Fullscreen"
    );
    expect(fullscreenButton).toBeTruthy();

    await act(async () => {
      fullscreenButton?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await flushPromises();
    });
    expect(requestFullscreenMock).toHaveBeenCalledTimes(1);
    expect(
      Array.from(container.querySelectorAll("button")).some(
        (button) => button.textContent?.trim() === "Exit Fullscreen"
      )
    ).toBe(true);

    const exitButton = Array.from(container.querySelectorAll("button")).find(
      (button) => button.textContent?.trim() === "Exit Fullscreen"
    );
    await act(async () => {
      exitButton?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await flushPromises();
    });
    expect(exitFullscreenMock).toHaveBeenCalledTimes(1);

    await act(async () => {
      root.unmount();
    });
    container.remove();
  });

  it("recomputes fullscreen canvas frame sizing on viewport resize", async () => {
    const requestFullscreenMock = jest.fn(function requestFullscreen(this: HTMLElement) {
      setFullscreenElement(this);
      document.dispatchEvent(new Event("fullscreenchange"));
      return Promise.resolve();
    });
    Object.defineProperty(HTMLElement.prototype, "requestFullscreen", {
      configurable: true,
      writable: true,
      value: requestFullscreenMock,
    });
    Object.defineProperty(document, "fullscreenEnabled", {
      configurable: true,
      writable: true,
      value: true,
    });
    setFullscreenElement(null);

    Object.defineProperty(window, "innerWidth", { configurable: true, writable: true, value: 1200 });
    Object.defineProperty(window, "innerHeight", { configurable: true, writable: true, value: 800 });

    const container = document.createElement("div");
    document.body.appendChild(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(<PlayPanel />);
      await flushPromises();
    });

    const fullscreenButton = Array.from(container.querySelectorAll("button")).find(
      (button) => button.textContent?.trim() === "Fullscreen"
    );
    await act(async () => {
      fullscreenButton?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await flushPromises();
    });

    const frameBefore = container.querySelector('[data-testid="play-canvas-frame"]') as HTMLDivElement | null;
    const widthBeforeResize = frameBefore?.style.width ?? "";
    expect(widthBeforeResize).toBeTruthy();

    Object.defineProperty(window, "innerWidth", { configurable: true, writable: true, value: 780 });
    Object.defineProperty(window, "innerHeight", { configurable: true, writable: true, value: 500 });

    await act(async () => {
      window.dispatchEvent(new Event("resize"));
      await flushPromises();
    });

    const frameAfter = container.querySelector('[data-testid="play-canvas-frame"]') as HTMLDivElement | null;
    expect(frameAfter?.style.width).not.toEqual(widthBeforeResize);

    await act(async () => {
      root.unmount();
    });
    container.remove();
  });

  it("applies emulator shell styling hooks for the play viewport", async () => {
    const container = document.createElement("div");
    document.body.appendChild(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(<PlayPanel />);
      await flushPromises();
    });

    const frame = container.querySelector('[data-testid="play-canvas-frame"]');
    const shell = container.querySelector('[data-testid="play-canvas-shell"]');

    expect(frame).toBeTruthy();
    expect(shell).toBeTruthy();
    expect(frame?.className).toContain("playui-screen-frame");
    expect(shell?.className).toContain("playui-bezel");

    await act(async () => {
      root.unmount();
    });
    container.remove();
  });
});

describe("PlayPanel multiplayer controls", () => {
  let consoleWarnSpy: jest.SpyInstance;
  let consoleErrorSpy: jest.SpyInstance;

  beforeAll(() => {
    (globalThis as any).IS_REACT_ACT_ENVIRONMENT = true;
    if (!window.matchMedia) {
      window.matchMedia = jest.fn().mockImplementation(() => ({
        matches: false,
        addEventListener: jest.fn(),
        removeEventListener: jest.fn(),
        addListener: jest.fn(),
        removeListener: jest.fn(),
        dispatchEvent: jest.fn(),
      }));
    }
  });

  afterAll(() => {
    delete (globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT;
  });

  beforeEach(() => {
    mockSettingsPanelSpy.mockClear();
    mockMultiplayerMenuSpy.mockClear();
    mockGameCanvasSpy.mockClear();
    mockCreateSupabaseBrowserClient.mockReturnValue({
      auth: {
        getUser: jest.fn(async () => ({ data: { user: { id: "local-1" } } })),
        onAuthStateChange: jest.fn(() => ({ data: { subscription: { unsubscribe: jest.fn() } } })),
      },
      from: jest.fn((table: string) => {
        if (table === "play_user_settings") {
          return {
            select: jest.fn(() => ({
              eq: jest.fn(() => ({
                maybeSingle: jest.fn(async () => ({ data: null, error: null })),
              })),
            })),
            upsert: jest.fn(async () => ({ error: null })),
          };
        }
        throw new Error(`Unexpected table ${table}`);
      }),
    } as any);
    act(() => {
      useMultiplayerStore.getState().reset();
    });
    consoleWarnSpy = jest.spyOn(console, "warn").mockImplementation(() => undefined);
    consoleErrorSpy = jest.spyOn(console, "error").mockImplementation(() => undefined);
  });

  afterEach(() => {
    consoleWarnSpy.mockRestore();
    consoleErrorSpy.mockRestore();
  });

  it("enables crowd view and remote sprites when toggled", async () => {
    const container = document.createElement("div");
    document.body.appendChild(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(<PlayPanel />);
      await flushPromises();
    });

    const lobbyButton = findButtonByLabel(container, "Lobby");
    expect(lobbyButton).toBeTruthy();
    await act(async () => {
      lobbyButton?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await flushPromises();
    });

    const props = mockMultiplayerMenuSpy.mock.calls.at(-1)?.[0] as
      | { onToggleCrowdView?: () => void }
      | undefined;
    expect(props?.onToggleCrowdView).toBeTruthy();

    await act(async () => {
      props?.onToggleCrowdView?.();
      await flushPromises();
    });

    const state = useMultiplayerStore.getState();
    expect(state.crowdViewEnabled).toBe(true);
    expect(state.remoteSpritesVisible).toBe(true);
    expect(mockPresenceManagers).toHaveLength(1);
    expect(useMultiplayerStore.getState().connectionState).toBe("connected");
    expect(consoleWarnSpy).not.toHaveBeenCalled();
    expect(consoleErrorSpy).not.toHaveBeenCalled();

    await act(async () => {
      root.unmount();
    });
    container.remove();
  });

  it("starts a multiplayer battle session after an accepted request", async () => {
    window.localStorage.setItem("pokecrystal.play.playIntro", "false");
    const fakeGame = createFakeGame();
    const container = document.createElement("div");
    document.body.appendChild(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(<PlayPanel />);
      await flushPromises();
      await flushPromises();
    });

    const gameCanvasCall = mockGameCanvasSpy.mock.calls.at(-1)?.[0];
    expect(gameCanvasCall?.onGameReady).toBeTruthy();
    await act(async () => {
      gameCanvasCall?.onGameReady?.(fakeGame);
      await flushPromises();
    });

    const lobbyButton = findButtonByLabel(container, "Lobby");
    await act(async () => {
      lobbyButton?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await flushPromises();
    });

    const menuProps = mockMultiplayerMenuSpy.mock.calls.at(-1)?.[0];
    expect(menuProps?.onConnect).toBeTruthy();
    await act(async () => {
      menuProps?.onConnect?.();
      await flushPromises();
      await flushPromises();
    });

    expect(mockPresenceManagers).toHaveLength(1);
    const manager = mockPresenceManagers[0];
    manager.sendInteractionRequest.mockResolvedValueOnce("battle-req");
    await act(async () => {
      manager.remotePlayersCallbacks.forEach((callback) =>
        callback([
          {
            userId: "peer-1",
            playerName: "Leaf",
            entityType: "player",
            mapName: "New Bark Town",
            tileX: 5,
            tileY: 8,
            direction: "left",
            updatedAtMs: Date.now(),
          },
        ])
      );
      await flushPromises();
    });

    await act(async () => {
      menuProps?.onRequestBattle?.();
      await flushPromises();
    });

    expect(manager.sendInteractionRequest).toHaveBeenCalledWith(expect.any(String), "battle");

    await act(async () => {
      manager.emitInteractionResponse({
        requestId: "battle-req",
        fromUserId: "peer-1",
        toUserId: "local-1",
        kind: "battle",
        accepted: true,
        timestampMs: Date.now(),
      });
      await flushPromises();
    });

    expect(mockWebRtcConnections).toHaveLength(1);
    const connection = mockWebRtcConnections[0];
    expect(connection.config).toEqual({ matchId: "battle-req", isHost: true });

    await act(async () => {
      connection.emitConnect();
      await flushPromises();
    });

    expect(connection.send).toHaveBeenCalledWith(
      expect.objectContaining({
        type: "session:hello",
        data: expect.objectContaining({ kind: "battle", playerName: "Ryan" }),
      })
    );

    await act(async () => {
      connection.emitData({
        type: "session:hello",
        data: {
          kind: "battle",
          playerName: "Leaf",
          party: [
            {
              species: { id: "CYNDAQUIL", int_id: 155 },
              nickname: "CYNDA",
              original_trainer_name: "LEAF",
              original_trainer_id: 2,
              moves: [{ name: "TACKLE", current_pp: 35 }],
              level: 5,
              hp: 19,
              max_hp: 19,
              attack: 10,
              defense: 9,
              speed: 11,
              special_attack: 10,
              special_defense: 10,
              dvs: { attack: 1, defense: 1, speed: 1, special: 1, hp: 1 },
              experience: 100,
              happiness: 70,
              hp_exp: 0,
              attack_exp: 0,
              defense_exp: 0,
              speed_exp: 0,
              special_exp: 0,
              pokerus: false,
            },
          ],
        },
      });
      await flushPromises();
    });

    expect(fakeGame.setMultiplayerBattleTransport).toHaveBeenCalledTimes(1);
    expect(fakeGame.startMultiplayerBattle).toHaveBeenCalledWith(
      expect.arrayContaining([
        expect.objectContaining({
          species: expect.objectContaining({ id: "CYNDAQUIL" }),
        }),
      ])
    );

    const originalFetch = global.fetch;
    const fetchMock = jest.fn(async () => new Response(JSON.stringify({ ok: true }), { status: 200 }));
    global.fetch = fetchMock as unknown as typeof fetch;
    const completeBattle = fakeGame.onMultiplayerBattleComplete.mock.calls.at(-1)?.[0] as
      | ((result: { result: number }) => void)
      | undefined;
    await act(async () => {
      completeBattle?.({ result: 0 });
      await flushPromises();
      await flushPromises();
    });
    expect(fetchMock).toHaveBeenCalledWith(
      "/api/multiplayer/matches",
      expect.objectContaining({
        method: "POST",
        body: expect.stringContaining('"peerUserId":"peer-1"'),
      })
    );
    global.fetch = originalFetch;

    await act(async () => {
      root.unmount();
    });
    container.remove();
  });

  it("starts and completes a trade session after accepting a request", async () => {
    window.localStorage.setItem("pokecrystal.play.playIntro", "false");
    const fakeGame = createFakeGame();
    const container = document.createElement("div");
    document.body.appendChild(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(<PlayPanel />);
      await flushPromises();
      await flushPromises();
    });

    const gameCanvasCall = mockGameCanvasSpy.mock.calls.at(-1)?.[0];
    await act(async () => {
      gameCanvasCall?.onGameReady?.(fakeGame);
      await flushPromises();
    });

    const lobbyButton = findButtonByLabel(container, "Lobby");
    await act(async () => {
      lobbyButton?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await flushPromises();
    });

    const menuProps = mockMultiplayerMenuSpy.mock.calls.at(-1)?.[0];
    await act(async () => {
      menuProps?.onConnect?.();
      await flushPromises();
      await flushPromises();
    });

    const manager = mockPresenceManagers[0];
    const originalFetch = global.fetch;
    const fetchMock = jest.fn(async () => new Response(JSON.stringify({ ok: true }), { status: 200 }));
    global.fetch = fetchMock as unknown as typeof fetch;
    mockTrade.mockResolvedValueOnce({
      cancelled: false,
      receivedPokemon: {
        species: { id: "TOTODILE", int_id: 158 },
        nickname: "TOTO",
        original_trainer_name: "LEAF",
        original_trainer_id: 2,
        moves: [{ name: "SCRATCH", current_pp: 35 }],
        level: 5,
        hp: 21,
        max_hp: 21,
        attack: 11,
        defense: 10,
        speed: 10,
        special_attack: 10,
        special_defense: 10,
        dvs: { attack: 1, defense: 1, speed: 1, special: 1, hp: 1 },
        experience: 100,
        happiness: 70,
        hp_exp: 0,
        attack_exp: 0,
        defense_exp: 0,
        speed_exp: 0,
        special_exp: 0,
        pokerus: false,
      },
    });

    await act(async () => {
      manager.emitInteractionRequest({
        requestId: "trade-req",
        fromUserId: "peer-1",
        fromPlayerName: "Leaf",
        toUserId: "local-1",
        kind: "trade",
        timestampMs: Date.now(),
      });
      await flushPromises();
    });

    const updatedMenuProps = mockMultiplayerMenuSpy.mock.calls.at(-1)?.[0];
    await act(async () => {
      updatedMenuProps?.onAcceptRequest?.();
      await flushPromises();
    });

    expect(manager.sendInteractionResponse).toHaveBeenCalledWith(
      expect.objectContaining({ requestId: "trade-req", kind: "trade" }),
      true
    );
    expect(mockWebRtcConnections).toHaveLength(1);
    const connection = mockWebRtcConnections[0];
    expect(connection.config).toEqual({ matchId: "trade-req", isHost: false });

    await act(async () => {
      connection.emitConnect();
      await flushPromises();
    });

    await act(async () => {
      connection.emitData({
        type: "session:hello",
        data: {
          kind: "trade",
          playerName: "Leaf",
          party: [],
        },
      });
      await flushPromises();
    });

    expect(mockTrade).toHaveBeenCalledWith(
      expect.objectContaining({
        species: expect.objectContaining({ id: "CHIKORITA" }),
      }),
      { confirm: true }
    );
    expect(fakeGame.replacePartyPokemon).toHaveBeenCalledWith(
      0,
      expect.objectContaining({
        species: expect.objectContaining({ id: "TOTODILE" }),
      })
    );
    expect(fetchMock).toHaveBeenCalledWith(
      "/api/multiplayer/matches",
      expect.objectContaining({
        method: "POST",
        body: expect.stringContaining('"mode":"trade"'),
      })
    );
    global.fetch = originalFetch;

    await act(async () => {
      root.unmount();
    });
    container.remove();
  });
});
