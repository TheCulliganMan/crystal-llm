/** @jest-environment jsdom */
import React, { act } from "react";
import { createRoot } from "react-dom/client";
import ReactDOMServer from "react-dom/server";
import { GuestSavePanel } from "./guest-save-panel";
import { GUEST_SESSION_PREFIX } from "@pokecrystal/core/core/guest-session-storage";
import { MANUAL_SAVE_SLOT } from "@pokecrystal/core/core/save-slots";

jest.mock("@pokecrystal/core/core/save", () => ({
  normalizeSaveSnapshot: jest.fn((data: unknown) => data),
}));

const actEnv = globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean };
actEnv.IS_REACT_ACT_ENVIRONMENT = true;

describe("GuestSavePanel", () => {
  it("renders unavailable storage on the server", () => {
    const html = ReactDOMServer.renderToString(<GuestSavePanel />);
    expect(html).toContain("Guest save storage is unavailable in this session.");
    expect(html).toContain("Reload Save");
  });

  it("updates to localStorage on the client after hydration", async () => {
    const container = document.createElement("div");
    document.body.appendChild(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(<GuestSavePanel />);
    });

    expect(container.textContent).toContain("localStorage");
    expect(container.textContent).toContain("Reload Save");

    await act(async () => {
      root.unmount();
    });
    container.remove();
  });

  it("invokes the load callback when Reload Save is clicked", async () => {
    const loadSave = jest.fn();
    const container = document.createElement("div");
    document.body.appendChild(container);
    const root = createRoot(container);
    const slotKey = `${GUEST_SESSION_PREFIX}${MANUAL_SAVE_SLOT}`;
    window.localStorage.setItem(slotKey, JSON.stringify({ sram: { player_name: "Jules" } }));

    await act(async () => {
      root.render(<GuestSavePanel onLoadSave={loadSave} />);
    });
    await act(async () => {
      await Promise.resolve();
    });

    const button = Array.from(container.querySelectorAll("button")).find(
      (node) => node.textContent === "Reload Save"
    );
    expect(button).toBeTruthy();

    await act(async () => {
      button?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });

    expect(loadSave).toHaveBeenCalledTimes(1);

    await act(async () => {
      root.unmount();
    });
    window.localStorage.removeItem(slotKey);
    container.remove();
  });

  it("shows upload and download actions for manual saves", async () => {
    const container = document.createElement("div");
    document.body.appendChild(container);
    const root = createRoot(container);
    const slotKey = `${GUEST_SESSION_PREFIX}${MANUAL_SAVE_SLOT}`;
    window.localStorage.setItem(slotKey, JSON.stringify({ sram: { player_name: "Jules" } }));

    await act(async () => {
      root.render(<GuestSavePanel />);
    });
    await act(async () => {
      await Promise.resolve();
    });

    const text = container.textContent ?? "";
    expect(text).toContain("Download");
    expect(text).toContain("Upload");

    await act(async () => {
      root.unmount();
    });
    window.localStorage.removeItem(slotKey);
    container.remove();
  });

  it("opens the file picker when Upload is clicked", async () => {
    const container = document.createElement("div");
    document.body.appendChild(container);
    const root = createRoot(container);
    const slotKey = `${GUEST_SESSION_PREFIX}${MANUAL_SAVE_SLOT}`;
    window.localStorage.setItem(slotKey, JSON.stringify({ sram: { player_name: "Jules" } }));

    await act(async () => {
      root.render(<GuestSavePanel />);
    });
    await act(async () => {
      await Promise.resolve();
    });

    const uploadButton = Array.from(container.querySelectorAll("button")).find(
      (node) => node.textContent === "Upload"
    ) as HTMLButtonElement | undefined;
    const uploadInput = container.querySelector('input[type="file"]') as HTMLInputElement | null;

    expect(uploadButton).toBeTruthy();
    expect(uploadInput).toBeTruthy();

    const clickSpy = jest.spyOn(uploadInput as HTMLInputElement, "click");

    await act(async () => {
      uploadButton?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });

    expect(clickSpy).toHaveBeenCalledTimes(1);

    await act(async () => {
      root.unmount();
    });
    window.localStorage.removeItem(slotKey);
    container.remove();
  });

  it("imports an uploaded save into the selected slot", async () => {
    const container = document.createElement("div");
    document.body.appendChild(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(<GuestSavePanel />);
    });
    await act(async () => {
      await Promise.resolve();
    });

    const uploadInput = container.querySelector('input[type="file"]') as HTMLInputElement | null;
    expect(uploadInput).toBeTruthy();

    const file = new File(
      [JSON.stringify({ sram: { player_name: "Gold" } })],
      "manual.sav.json",
      { type: "application/json" }
    );

    await act(async () => {
      Object.defineProperty(uploadInput, "files", {
        configurable: true,
        value: [file],
      });
      uploadInput?.dispatchEvent(new Event("change", { bubbles: true }));
      await Promise.resolve();
    });

    expect(window.localStorage.getItem(`${GUEST_SESSION_PREFIX}${MANUAL_SAVE_SLOT}`)).toBe(
      JSON.stringify({ sram: { player_name: "Gold" } })
    );

    await act(async () => {
      root.unmount();
    });
    window.localStorage.removeItem(`${GUEST_SESSION_PREFIX}${MANUAL_SAVE_SLOT}`);
    container.remove();
  });
});
