/** @jest-environment jsdom */
import React, { act } from "react";
import { createRoot } from "react-dom/client";
import { SettingsPanel } from "./settings-panel";
import { PlayerGender, TimeOfDay } from "@pokecrystal/core/core/enums";

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
          playerGender={PlayerGender.MALE}
          timeOfDay={TimeOfDay.DAY}
          playerName="Ryan"
          soundEnabled={false}
          instantModeEnabled={false}
          brandTheme="krabby"
          playIntroEnabled={false}
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
          playerGender={PlayerGender.MALE}
          timeOfDay={TimeOfDay.DAY}
          playerName="Ryan"
          soundEnabled={false}
          instantModeEnabled={false}
          brandTheme="krabby"
          playIntroEnabled={false}
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

  it("updates skip to play toggle", async () => {
    const container = document.createElement("div");
    document.body.appendChild(container);
    const root = createRoot(container);
    const onPlayIntroEnabledChange = jest.fn();

    await act(async () => {
      root.render(
        <SettingsPanel
          playerGender={PlayerGender.MALE}
          timeOfDay={TimeOfDay.DAY}
          playerName="Ryan"
          soundEnabled={false}
          instantModeEnabled={false}
          brandTheme="krabby"
          playIntroEnabled={false}
          onPlayIntroEnabledChange={onPlayIntroEnabledChange}
        />
      );
    });

    const onButton = Array.from(container.querySelectorAll("button")).find(
      (button) => button.getAttribute("aria-label") === "Skip to play on"
    );
    expect(onButton).toBeTruthy();

    await act(async () => {
      onButton?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });

    expect(onPlayIntroEnabledChange).toHaveBeenCalledWith(true);

    await act(async () => {
      root.unmount();
    });
    container.remove();
  });

});
