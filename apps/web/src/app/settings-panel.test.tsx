/** @jest-environment jsdom */
import React, { act } from "react";
import { createRoot } from "react-dom/client";
import { SettingsPanel } from "./settings-panel";

describe("SettingsPanel", () => {
  beforeAll(() => {
    (globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
  });

  afterAll(() => {
    delete (globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT;
  });

  it("updates brand theme when toggled", async () => {
    const container = document.createElement("div");
    document.body.appendChild(container);
    const root = createRoot(container);
    const onBrandThemeChange = jest.fn();

    await act(async () => {
      root.render(
        <SettingsPanel
          playerName="Ryan"
          brandTheme="krabby"
          onBrandThemeChange={onBrandThemeChange}
        />
      );
    });

    const pinsirButton = Array.from(container.querySelectorAll("button")).find(
      (button) => button.textContent?.trim() === "Pinsir"
    );
    expect(pinsirButton).toBeTruthy();

    await act(async () => {
      pinsirButton?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });

    expect(onBrandThemeChange).toHaveBeenCalledWith("pinsir");

    await act(async () => {
      root.unmount();
    });
    container.remove();
  });

  it("renders all pokemon theme options", async () => {
    const container = document.createElement("div");
    document.body.appendChild(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(
        <SettingsPanel
          playerName="Ryan"
          brandTheme="krabby"
        />
      );
    });

    expect(container.textContent).toContain("Pokemon Theme");
    [
      "Krabby",
      "Kingler",
      "Heracross",
      "Gligar",
      "Scizor",
      "Sneasel",
      "Teddiursa",
      "Ursaring",
      "Totodile",
      "Croconaw",
      "Feraligatr",
      "Pinsir",
    ].forEach((label) => {
      expect(container.textContent).toContain(label);
    });

    await act(async () => {
      root.unmount();
    });
    container.remove();
  });

  it("does not render toggle controls or reset player name", async () => {
    const container = document.createElement("div");
    document.body.appendChild(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(
        <SettingsPanel
          playerName="Ryan"
          brandTheme="krabby"
        />
      );
    });

    expect(container.textContent).not.toContain("Player Gender");
    expect(container.textContent).not.toContain("Time of Day");
    expect(container.textContent).not.toContain("Sound");
    expect(container.textContent).not.toContain("Instant Mode");
    expect(container.textContent).not.toContain("Skip to play");
    expect(container.textContent).not.toContain("Reset player name");

    await act(async () => {
      root.unmount();
    });
    container.remove();
  });

});
