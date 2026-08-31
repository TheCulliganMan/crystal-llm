/** @jest-environment jsdom */
import React, { act } from "react";
import { createRoot } from "react-dom/client";
import { MultiplayerChat } from "./multiplayer-chat";
import { useMultiplayerStore } from "@pokecrystal/core/multiplayer/multiplayer-store";

const flushPromises = async () => Promise.resolve();

describe("MultiplayerChat", () => {
  beforeAll(() => {
    (globalThis as any).IS_REACT_ACT_ENVIRONMENT = true;
  });

  beforeEach(() => {
    useMultiplayerStore.getState().reset();
  });

  it("switches WoW-style channels, sends messages, and exposes moderation actions", async () => {
    const onSend = jest.fn();
    const onReport = jest.fn();
    const onToggleBlock = jest.fn();
    const container = document.createElement("div");
    document.body.appendChild(container);
    const root = createRoot(container);
    const message = {
      messageId: "m1",
      userId: "peer-1",
      playerName: "Leaf",
      text: "Anyone want to battle?",
      outgoing: false,
      channel: "trade" as const,
      timestampMs: 1,
    };

    await act(async () => {
      useMultiplayerStore.getState().setConnectionState("connected");
      root.render(
        <MultiplayerChat
          messages={[message]}
          remotePlayers={[{
            userId: "peer-1", playerName: "Leaf", entityType: "player", mapName: "NewBarkTown",
            tileX: 1, tileY: 1, direction: "down", updatedAtMs: 1,
          }]}
          selectedRemoteUserId="peer-1"
          onSend={onSend}
          onReport={onReport}
          onToggleBlock={onToggleBlock}
        />,
      );
      await flushPromises();
    });

    await act(async () => {
      (container.querySelector('[data-testid="chat-tab-trade"]') as HTMLButtonElement).click();
      await flushPromises();
    });
    expect(container.querySelector('[data-testid="chat-log"]')?.textContent).toContain("Anyone want to battle?");

    const input = container.querySelector('[data-testid="chat-input"]') as HTMLInputElement;
    await act(async () => {
      const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")?.set;
      setter?.call(input, "Trading Cyndaquil");
      input.dispatchEvent(new Event("input", { bubbles: true }));
      await flushPromises();
    });
    await act(async () => {
      (container.querySelector('[data-testid="chat-form"]') as HTMLFormElement)
        .dispatchEvent(new Event("submit", { bubbles: true, cancelable: true }));
      await flushPromises();
    });
    expect(onSend).toHaveBeenCalledWith("trade", "Trading Cyndaquil");

    await act(async () => {
      (container.querySelector('[data-testid="block-m1"]') as HTMLButtonElement).click();
      (container.querySelector('[data-testid="report-m1"]') as HTMLButtonElement).click();
    });
    expect(onToggleBlock).toHaveBeenCalledWith("peer-1");
    expect(onReport).toHaveBeenCalledWith(message);

    await act(async () => root.unmount());
    container.remove();
  });
});
