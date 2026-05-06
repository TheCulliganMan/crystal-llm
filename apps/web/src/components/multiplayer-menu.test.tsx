/** @jest-environment jsdom */
import React, { act } from "react";
import { createRoot } from "react-dom/client";
import { MultiplayerMenu } from "./multiplayer-menu";
import { useMultiplayerStore } from "@pokecrystal/core/multiplayer/multiplayer-store";

const flushPromises = async (): Promise<void> => {
  await Promise.resolve();
};

describe("MultiplayerMenu", () => {
  beforeAll(() => {
    (globalThis as any).IS_REACT_ACT_ENVIRONMENT = true;
  });

  afterAll(() => {
    delete (globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT;
  });

  beforeEach(() => {
    useMultiplayerStore.getState().reset();
  });

  it("connect/disconnect button calls callbacks", async () => {
    const onConnect = jest.fn();
    const onDisconnect = jest.fn();

    const container = document.createElement("div");
    document.body.appendChild(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(<MultiplayerMenu onConnect={onConnect} onDisconnect={onDisconnect} />);
      await flushPromises();
    });

    const toggle = container.querySelector('[data-testid="toggle-connection"]') as HTMLButtonElement;
    expect(toggle).toBeTruthy();

    await act(async () => {
      toggle.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await flushPromises();
    });
    expect(onConnect).toHaveBeenCalledTimes(1);

    await act(async () => {
      useMultiplayerStore.getState().setConnectionState("connected");
      await flushPromises();
    });

    await act(async () => {
      toggle.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await flushPromises();
    });
    expect(onDisconnect).toHaveBeenCalledTimes(1);

    await act(async () => {
      root.unmount();
    });
    container.remove();
  });

  it("shows online counts and toggles", async () => {
    const onToggleRemoteSprites = jest.fn();
    const onToggleCrowdView = jest.fn();
    const onRequestBattle = jest.fn();
    const onRequestTrade = jest.fn();
    const onAcceptRequest = jest.fn();
    const onDeclineRequest = jest.fn();

    const container = document.createElement("div");
    document.body.appendChild(container);
    const root = createRoot(container);

    await act(async () => {
      useMultiplayerStore.getState().setConnectionState("connected");
      root.render(
        <MultiplayerMenu
          remoteSpritesVisible
          crowdViewEnabled={false}
          onlinePlayerCount={500}
          onlineAiCount={12}
          remotePlayers={[
            {
              userId: "peer-1",
              playerName: "Leaf",
              entityType: "player",
              mapName: "New Bark Town",
              tileX: 4,
              tileY: 8,
              direction: "down",
              updatedAtMs: Date.now(),
            },
          ]}
          selectedRemoteUserId="peer-1"
          onToggleRemoteSprites={onToggleRemoteSprites}
          onToggleCrowdView={onToggleCrowdView}
          canRequestInteraction
          onRequestBattle={onRequestBattle}
          onRequestTrade={onRequestTrade}
          pendingOutgoingLabel="Waiting for Opponent to accept battle..."
          incomingRequestLabel="Opponent requests a trade."
          onAcceptRequest={onAcceptRequest}
          onDeclineRequest={onDeclineRequest}
          interactionStatusLabel="Last request accepted."
        />
      );
      await flushPromises();
    });

    expect(container.querySelector('[data-testid="online-players-count"]')?.textContent).toContain("500");
    expect(container.querySelector('[data-testid="online-ai-count"]')?.textContent).toContain("12");
    expect(container.querySelector('[data-testid="selected-remote-player"]')?.textContent).toContain("Leaf");

    const toggleSprites = container.querySelector('[data-testid="toggle-remote-sprites"]') as HTMLButtonElement;
    const toggleCrowd = container.querySelector('[data-testid="toggle-crowd-view"]') as HTMLButtonElement;
    const requestBattle = container.querySelector('[data-testid="request-battle"]') as HTMLButtonElement;
    const requestTrade = container.querySelector('[data-testid="request-trade"]') as HTMLButtonElement;
    const acceptRequest = container.querySelector('[data-testid="accept-request"]') as HTMLButtonElement;
    const declineRequest = container.querySelector('[data-testid="decline-request"]') as HTMLButtonElement;

    await act(async () => {
      toggleSprites.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      toggleCrowd.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      requestBattle.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      requestTrade.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      acceptRequest.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      declineRequest.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await flushPromises();
    });

    expect(onToggleRemoteSprites).toHaveBeenCalledTimes(1);
    expect(onToggleCrowdView).toHaveBeenCalledTimes(1);
    expect(onRequestBattle).toHaveBeenCalledTimes(1);
    expect(onRequestTrade).toHaveBeenCalledTimes(1);
    expect(onAcceptRequest).toHaveBeenCalledTimes(1);
    expect(onDeclineRequest).toHaveBeenCalledTimes(1);

    await act(async () => {
      root.unmount();
    });
    container.remove();
  });

  it("requires authentication before connecting", async () => {
    const onConnect = jest.fn();

    const container = document.createElement("div");
    document.body.appendChild(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(
        <MultiplayerMenu
          isAuthenticated={false}
          authLabel="Sign in first."
          onConnect={onConnect}
        />
      );
      await flushPromises();
    });

    expect(container.querySelector('[data-testid="mp-auth-required"]')?.textContent).toContain("Sign in first.");
    const toggle = container.querySelector('[data-testid="toggle-connection"]') as HTMLButtonElement;
    expect(toggle.disabled).toBe(true);

    await act(async () => {
      toggle.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await flushPromises();
    });
    expect(onConnect).not.toHaveBeenCalled();

    await act(async () => {
      root.unmount();
    });
    container.remove();
  });
});
