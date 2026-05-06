"use client";

import React, { useCallback, useId } from "react";
import { PlayerGender, TimeOfDay } from "@pokecrystal/core/core/enums";
import type { BrandThemeKey } from "./theme-preferences";

export type BrandTheme = BrandThemeKey;

type SettingsPanelProps = {
  playerGender: PlayerGender;
  onPlayerGenderChange?: (gender: PlayerGender) => void;
  timeOfDay: TimeOfDay;
  onTimeOfDayChange?: (timeOfDay: TimeOfDay) => void;
  playerName: string;
  onPlayerNameChange?: (name: string) => void;
  soundEnabled: boolean;
  onSoundEnabledChange?: (enabled: boolean) => void;
  instantModeEnabled: boolean;
  onInstantModeEnabledChange?: (enabled: boolean) => void;
  brandTheme: BrandTheme;
  onBrandThemeChange?: (theme: BrandTheme) => void;
  playIntroEnabled: boolean;
  onPlayIntroEnabledChange?: (enabled: boolean) => void;
  showBootModeControl?: boolean;
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

type ToggleOption<T extends string | number | boolean> = {
  value: T;
  label: string;
};

const ToggleStrip = <T extends string | number | boolean>({
  value,
  options,
  onChange,
  ariaLabelPrefix,
}: {
  value: T;
  options: ToggleOption<T>[];
  onChange: (next: T) => void;
  ariaLabelPrefix?: string;
}) => {
  const useJoin = options.length <= 3;
  const columns =
    options.length <= 2
      ? "grid-cols-2"
      : options.length <= 3
        ? "grid-cols-2 sm:grid-cols-3"
        : "grid-cols-2 sm:grid-cols-3";

  return (
    <div className={useJoin ? "join" : `grid gap-2 ${columns}`}>
      {options.map((option) => {
        const isActive = value === option.value;
        const optionLabel = option.label;

        return (
          <button
            key={String(option.value)}
            type="button"
            onClick={() => onChange(option.value)}
            className={[
              "btn btn-sm",
              isActive ? "btn-primary" : "btn-outline",
              useJoin ? "join-item" : "",
            ].join(" ").trim()}
            aria-pressed={isActive}
            aria-label={ariaLabelPrefix ? `${ariaLabelPrefix} ${String(optionLabel).toLowerCase()}` : undefined}
          >
            <span className="text-xs sm:text-sm">{optionLabel}</span>
          </button>
        );
      })}
    </div>
  );
};

export const SettingsPanel = React.memo(({
  playerGender,
  onPlayerGenderChange,
  timeOfDay,
  onTimeOfDayChange,
  playerName,
  onPlayerNameChange,
  soundEnabled,
  onSoundEnabledChange,
  instantModeEnabled,
  onInstantModeEnabledChange,
  brandTheme,
  onBrandThemeChange,
  playIntroEnabled,
  onPlayIntroEnabledChange,
  showBootModeControl = true,
}: SettingsPanelProps) => {
  const playerNameFieldId = useId();

  const handleGenderChange = useCallback(
    (value: PlayerGender) => {
      onPlayerGenderChange?.(value);
    },
    [onPlayerGenderChange]
  );

  const handleTimeOfDayChange = useCallback(
    (value: TimeOfDay) => {
      onTimeOfDayChange?.(value);
    },
    [onTimeOfDayChange]
  );

  const handleNameChange = useCallback(
    (event: React.ChangeEvent<HTMLInputElement>) => {
      onPlayerNameChange?.(event.target.value);
    },
    [onPlayerNameChange]
  );

  const handleSoundChange = useCallback(
    (value: boolean) => {
      onSoundEnabledChange?.(value);
    },
    [onSoundEnabledChange]
  );

  const handleInstantModeChange = useCallback(
    (value: boolean) => {
      onInstantModeEnabledChange?.(value);
    },
    [onInstantModeEnabledChange]
  );

  const handleBrandThemeChange = useCallback(
    (value: BrandTheme) => {
      onBrandThemeChange?.(value);
    },
    [onBrandThemeChange]
  );

  const handlePlayIntroChange = useCallback(
    (value: boolean) => {
      onPlayIntroEnabledChange?.(value);
    },
    [onPlayIntroEnabledChange]
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
          <p className="text-xs font-semibold uppercase tracking-[0.16em] text-base-content/65">Player Gender</p>
          <ToggleStrip
            value={playerGender}
            onChange={handleGenderChange}
            options={[
              { value: PlayerGender.MALE, label: "Male" },
              { value: PlayerGender.FEMALE, label: "Female" },
            ]}
          />
        </div>
      </div>

      <div className="card border border-base-300/80 bg-gradient-to-b from-base-100/95 to-base-200/80 shadow-md">
        <div className="card-body gap-2.5 p-4">
          <p className="text-xs font-semibold uppercase tracking-[0.16em] text-base-content/65">Time of Day</p>
          <p className="text-xs text-base-content/70">Pick your time-of-day palette.</p>
          <ToggleStrip
            value={timeOfDay}
            onChange={handleTimeOfDayChange}
            options={[
              { value: TimeOfDay.MORN, label: "Morning" },
              { value: TimeOfDay.DAY, label: "Day" },
              { value: TimeOfDay.NIGHT, label: "Night" },
            ]}
            ariaLabelPrefix="Time of day"
          />
        </div>
      </div>

      <div className="card border border-base-300/80 bg-gradient-to-b from-base-100/95 to-base-200/80 shadow-md">
        <div className="card-body gap-2.5 p-4">
          <p className="text-xs font-semibold uppercase tracking-[0.16em] text-base-content/65">Sound</p>
          <ToggleStrip
            value={soundEnabled}
            onChange={handleSoundChange}
            options={[
              { value: true, label: "On" },
              { value: false, label: "Muted" },
            ]}
          />
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

      <div className="card border border-base-300/80 bg-gradient-to-b from-base-100/95 to-base-200/80 shadow-md">
        <div className="card-body gap-2.5 p-4">
          <p className="text-xs font-semibold uppercase tracking-[0.16em] text-base-content/65">Instant Mode</p>
          <ToggleStrip
            value={instantModeEnabled}
            onChange={handleInstantModeChange}
            options={[
              { value: true, label: "On" },
              { value: false, label: "Off" },
            ]}
          />
          <p className="text-xs text-base-content/70">
            Instantly process one-frame actions when available.
          </p>
        </div>
      </div>

      {showBootModeControl ? (
        <div className="card border border-base-300/80 bg-gradient-to-b from-base-100/95 to-base-200/80 shadow-md">
          <div className="card-body gap-2.5 p-4">
            <p className="text-xs font-semibold uppercase tracking-[0.16em] text-base-content/65">Skip to play</p>
            <ToggleStrip
              value={playIntroEnabled}
              onChange={handlePlayIntroChange}
              ariaLabelPrefix="Skip to play"
              options={[
                { value: true, label: "On" },
                { value: false, label: "Off" },
              ]}
            />
            <p className="text-xs text-base-content/70">
              When enabled, the game starts immediately instead of showing the title session overlay.
            </p>
          </div>
        </div>
      ) : null}

      <div className="pt-1">
        <button
          type="button"
          className="btn btn-outline btn-sm w-full"
          onClick={onPlayerNameChange ? () => onPlayerNameChange("Ryan") : undefined}
        >
          Reset player name
        </button>
      </div>
    </section>
  );
});

SettingsPanel.displayName = "SettingsPanel";

export default SettingsPanel;
