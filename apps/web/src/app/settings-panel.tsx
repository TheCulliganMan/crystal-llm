"use client";

import React, { useCallback, useId } from "react";
import type { BrandThemeKey } from "./theme-preferences";

export type BrandTheme = BrandThemeKey;

type SettingsPanelProps = {
  playerName: string;
  onPlayerNameChange?: (name: string) => void;
  brandTheme: BrandTheme;
  onBrandThemeChange?: (theme: BrandTheme) => void;
};

const BRAND_THEME_OPTIONS: Array<{ value: BrandTheme; label: string }> = [
  { value: "krabby", label: "Krabby" },
  { value: "kingler", label: "Kingler" },
  { value: "heracross", label: "Heracross" },
  { value: "gligar", label: "Gligar" },
  { value: "scizor", label: "Scizor" },
  { value: "sneasel", label: "Sneasel" },
  { value: "teddiursa", label: "Teddiursa" },
  { value: "ursaring", label: "Ursaring" },
  { value: "totodile", label: "Totodile" },
  { value: "croconaw", label: "Croconaw" },
  { value: "feraligatr", label: "Feraligatr" },
  { value: "pinsir", label: "Pinsir" },
];

export const SettingsPanel = React.memo(({
  playerName,
  onPlayerNameChange,
  brandTheme,
  onBrandThemeChange,
}: SettingsPanelProps) => {
  const playerNameFieldId = useId();

  const handleNameChange = useCallback(
    (event: React.ChangeEvent<HTMLInputElement>) => {
      onPlayerNameChange?.(event.target.value);
    },
    [onPlayerNameChange]
  );

  const handleBrandThemeChange = useCallback(
    (value: BrandTheme) => {
      onBrandThemeChange?.(value);
    },
    [onBrandThemeChange]
  );

  return (
    <section className="space-y-3 text-base-content">
      <div className="card border border-base-300/80 bg-gradient-to-b from-base-100/95 to-base-200/80 shadow-md">
        <div className="card-body gap-2.5 p-4">
          <p className="text-xs font-semibold uppercase tracking-[0.16em] text-base-content/65">Player Identity</p>
          <label htmlFor={playerNameFieldId} className="label p-0 pt-0.5">
            <span className="label-text font-medium">Player Name</span>
          </label>
          <input
            id={playerNameFieldId}
            type="text"
            value={playerName}
            onChange={handleNameChange}
            maxLength={10}
            className="input input-bordered w-full"
          />
          <p className="text-xs text-base-content/70">Defaults to Ryan when blank.</p>
        </div>
      </div>

      <div className="card border border-base-300/80 bg-gradient-to-b from-base-100/95 to-base-200/80 shadow-md">
        <div className="card-body gap-2.5 p-4">
          <p className="text-xs font-semibold uppercase tracking-[0.16em] text-base-content/65">Pokemon Theme</p>
          <div className="mb-2 flex items-center gap-2 text-xs text-base-content/75">
            <span className="kc-brand-sprite rounded-full" aria-hidden="true" />
            <span>Current theme: {brandTheme}</span>
          </div>
          <div className="grid gap-2 grid-cols-2 sm:grid-cols-3">
            {BRAND_THEME_OPTIONS.map((option) => (
              <button
                key={option.value}
                type="button"
                onClick={() => handleBrandThemeChange(option.value)}
                className={`btn btn-sm min-w-0 ${brandTheme === option.value ? "btn-primary" : "btn-outline"}`}
              >
                {option.label}
              </button>
            ))}
          </div>
          <p className="text-xs text-base-content/70">Updates mascot icon and accent colors.</p>
        </div>
      </div>
    </section>
  );
});

SettingsPanel.displayName = "SettingsPanel";

export default SettingsPanel;
