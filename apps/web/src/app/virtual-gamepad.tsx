"use client";

import React, { useCallback, useMemo, useRef } from "react";
import { Space_Grotesk, Space_Mono } from "next/font/google";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import {
  faArrowDown,
  faArrowLeft,
  faArrowRight,
  faArrowUp,
  faCircleDot,
  faPlay,
} from "@fortawesome/free-solid-svg-icons";
import { defaultKeyBindings, GameButton } from "@pokecrystal/core/input/config";
import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import type { GameEngineEvent } from "@pokecrystal/core/ui/game-engine";

const gamepadFont = Space_Grotesk({
  subsets: ["latin"],
  weight: ["400", "500", "600", "700"],
});
const keycapFont = Space_Mono({
  subsets: ["latin"],
  weight: ["400", "700"],
});

type Control = "up" | "down" | "left" | "right" | "a" | "b" | "start" | "select";

const DIRECTION_KEYS: Record<"up" | "down" | "left" | "right", string> = {
  up: "ArrowUp",
  down: "ArrowDown",
  left: "ArrowLeft",
  right: "ArrowRight",
};

const BUTTON_KEYS: Record<"a" | "b" | "start" | "select", string[]> = {
  a: defaultKeyBindings[GameButton.A],
  b: defaultKeyBindings[GameButton.B],
  start: defaultKeyBindings[GameButton.Start],
  select: defaultKeyBindings[GameButton.Select],
};

const CONTROL_LABELS: Record<Control, string> = {
  up: "Up",
  down: "Down",
  left: "Left",
  right: "Right",
  a: "A",
  b: "B",
  start: "Start",
  select: "Select",
};

const CONTROL_ICONS: Partial<Record<Control, typeof faArrowUp>> = {
  up: faArrowUp,
  down: faArrowDown,
  left: faArrowLeft,
  right: faArrowRight,
  start: faPlay,
  select: faCircleDot,
};

const KEY_LABEL_OVERRIDES: Record<string, string> = {
  ArrowUp: "Up",
  ArrowDown: "Down",
  ArrowLeft: "Left",
  ArrowRight: "Right",
  Space: "Space",
  Enter: "Enter",
  NumpadEnter: "NumEnter",
  Backspace: "Backspace",
  ShiftLeft: "LShift",
  ShiftRight: "RShift",
  Escape: "Esc",
};

const formatKeyLabel = (key: string | number): string => {
  if (typeof key === "number") {
    return String(key);
  }
  if (key in KEY_LABEL_OVERRIDES) {
    return KEY_LABEL_OVERRIDES[key];
  }
  if (key.startsWith("Key")) {
    return key.slice(3).toUpperCase();
  }
  return key;
};

const primaryKeyForControl = (control: Control): string => {
  if (control in DIRECTION_KEYS) {
    return DIRECTION_KEYS[control as "up" | "down" | "left" | "right"];
  }
  const keys = BUTTON_KEYS[control as "a" | "b" | "start" | "select"];
  return keys[0] ?? "Enter";
};

const createInputEvent = (control: Control, pressed: boolean): GameEngineEvent => {
  const key = primaryKeyForControl(control);
  const opts: GameEngineEvent = {
    type: pressed ? gameEngine.KEYDOWN : gameEngine.KEYUP,
    key,
    code: key,
    is_press: pressed,
  };
  if (control in DIRECTION_KEYS) {
    opts.direction = control;
  } else {
    opts.button = control;
  }
  const { type, ...rest } = opts;
  return new gameEngine.event.Event(type, rest);
};

type GamepadButtonProps = {
  control: Control;
  pressed: boolean;
  compact?: boolean;
  onPressChange: (control: Control, pressed: boolean) => void;
};

const GamepadButton = React.memo(({ control, pressed, compact = false, onPressChange }: GamepadButtonProps) => {
  const isDirectional =
    control === "up" || control === "down" || control === "left" || control === "right";
  const isAction = control === "a" || control === "b";
  const isSystem = control === "start" || control === "select";
  const icon = CONTROL_ICONS[control];
  const ariaLabel = isDirectional
    ? `D-pad ${CONTROL_LABELS[control]}`
    : `${CONTROL_LABELS[control]} button`;
  const handlePointerDown = useCallback(
    (event: React.PointerEvent<HTMLButtonElement>) => {
      event.preventDefault();
      event.currentTarget.setPointerCapture(event.pointerId);
      onPressChange(control, true);
    },
    [control, onPressChange]
  );

  const handlePointerUp = useCallback(
    (event: React.PointerEvent<HTMLButtonElement>) => {
      event.preventDefault();
      if (event.currentTarget.hasPointerCapture(event.pointerId)) {
        event.currentTarget.releasePointerCapture(event.pointerId);
      }
      onPressChange(control, false);
    },
    [control, onPressChange]
  );

  const handlePointerLeave = useCallback(
    (event: React.PointerEvent<HTMLButtonElement>) => {
      if (event.currentTarget.hasPointerCapture(event.pointerId)) {
        event.currentTarget.releasePointerCapture(event.pointerId);
      }
      onPressChange(control, false);
    },
    [control, onPressChange]
  );

  const accentBorder = pressed ? "var(--pad-accent)" : "var(--pad-edge)";
  const minimumTapWidth = compact
    ? isSystem
      ? "54px"
      : "44px"
    : "0px";
  const minimumTapHeight = compact ? "44px" : "0px";
  const buttonWidth = isSystem
    ? compact
      ? "max(54px, calc(var(--control-size) * 1.55))"
      : "calc(var(--control-size) * 1.3)"
    : isAction
      ? compact
        ? "max(44px, calc(var(--control-size) * 1.18))"
        : "calc(var(--control-size) * 1.1)"
      : compact
        ? "max(44px, var(--control-size))"
        : "var(--control-size)";
  const buttonHeight = isSystem
    ? compact
      ? "max(44px, calc(var(--control-size) * 0.96))"
      : "calc(var(--control-size) * 0.6)"
    : isAction
      ? compact
        ? "max(44px, calc(var(--control-size) * 1.18))"
        : "calc(var(--control-size) * 1.1)"
      : "var(--control-size)";
  const labelSize = isSystem ? "calc(var(--control-label-size) * 0.9)" : "var(--control-label-size)";

  return (
    <button
      type="button"
      className="btn btn-square border-0 p-0"
      aria-pressed={pressed}
      aria-label={ariaLabel}
      onPointerDown={handlePointerDown}
      onPointerUp={handlePointerUp}
      onPointerLeave={handlePointerLeave}
      onPointerCancel={handlePointerLeave}
      style={{
        cursor: "pointer",
        userSelect: "none",
        touchAction: "none",
        WebkitTapHighlightColor: "transparent",
        borderRadius: "var(--radius-sm)",
        borderColor: accentBorder,
        backgroundColor: pressed ? "var(--pad-accent-soft)" : "var(--pad-surface)",
        minWidth: buttonWidth,
        minHeight: minimumTapHeight,
        width: buttonWidth,
        height: buttonHeight,
        maxWidth: "100%",
        padding: 0,
        display: "inline-flex",
        flexDirection: "column",
        alignItems: "center",
        justifyContent: "center",
        position: "relative",
        color: "var(--pad-ink)",
        transition: "background-color 140ms ease, border-color 140ms ease",
      }}
    >
      <span
        style={{
          fontSize: labelSize,
          fontWeight: 700,
          letterSpacing: "0.08em",
          textTransform: "uppercase",
          color: "var(--pad-ink)",
          display: "inline-flex",
          alignItems: "center",
          justifyContent: "center",
          pointerEvents: "none",
          lineHeight: 1,
        }}
      >
        {icon ? <FontAwesomeIcon icon={icon} /> : CONTROL_LABELS[control]}
      </span>
    </button>
  );
});

GamepadButton.displayName = "GamepadButton";

const KeyBadge = ({ label }: { label: string }) => (
  <span
    className="badge badge-outline"
    style={{
      fontSize: "0.68rem",
      letterSpacing: "0.08em",
      fontFamily: keycapFont.style.fontFamily,
      textTransform: "uppercase",
      borderColor: "var(--pad-edge)",
      backgroundColor: "var(--pad-surface-weak)",
      color: "var(--pad-ink)",
    }}
  >
    {label}
  </span>
);

type VirtualGamepadProps = {
  pressedButtons: string[];
  pressedKeys: Array<string | number>;
  onVirtualButtonsChange?: (buttons: string[]) => void;
  postEvent: (event: GameEngineEvent) => void;
  embedded?: boolean;
  showHeader?: boolean;
  compact?: boolean;
  layout?: "standard" | "fullscreen";
  systemControl?: React.ReactNode;
};

export const VirtualGamepad = React.memo(({
  pressedButtons,
  pressedKeys,
  onVirtualButtonsChange,
  postEvent,
  embedded = false,
  showHeader = true,
  compact = false,
  layout = "standard",
  systemControl,
}: VirtualGamepadProps) => {
  const heldVirtual = useRef<Set<string>>(new Set());
  const pressedButtonSet = useMemo(() => new Set(pressedButtons), [pressedButtons]);

  const handlePressChange = useCallback(
    (control: Control, pressed: boolean) => {
      const updated = new Set(heldVirtual.current);
      if (pressed) {
        updated.add(control);
      } else {
        updated.delete(control);
      }
      heldVirtual.current = updated;
      onVirtualButtonsChange?.(Array.from(updated));
      postEvent(createInputEvent(control, pressed));
    },
    [onVirtualButtonsChange, postEvent]
  );

  const pressedButtonLabels = useMemo(
    () => pressedButtons.map((button) => CONTROL_LABELS[button as Control] ?? button),
    [pressedButtons]
  );

  const pressedKeyLabels = useMemo(
    () => pressedKeys.map(formatKeyLabel),
    [pressedKeys]
  );
  const isEmbeddedCompact = embedded && compact;
  const isFullscreenLayout = layout === "fullscreen";
  const controlSize = compact
    ? "clamp(26px, 7.4vw, 40px)"
    : isFullscreenLayout
      ? "clamp(54px, 9vw, 108px)"
      : "clamp(46px, 12vw, 88px)";
  const controlGap = compact
    ? "clamp(2px, 0.9vw, 5px)"
    : isFullscreenLayout
      ? "clamp(8px, 1.8vw, 14px)"
      : "clamp(6px, 1.8vw, 12px)";
  const labelSize = compact
    ? "clamp(0.54rem, 1.8vw, 0.84rem)"
    : isFullscreenLayout
      ? "clamp(0.8rem, 2.2vw, 1.15rem)"
      : "clamp(0.75rem, 2.4vw, 1.05rem)";
  const cardPadding = compact
    ? "calc(var(--control-gap, 8px) * 0.55)"
    : "calc(var(--control-gap, 8px) * 1.05)";

  return (
    <div
      className={`${gamepadFont.className} card card-bordered`}
      style={{
        position: "relative",
        overflow: "hidden",
        width: "100%",
        margin: embedded ? 0 : "0.8rem",
        padding: embedded ? (compact ? "0.25rem" : "0.75rem") : "1rem",
        paddingBottom: embedded ? (compact ? "0.25rem" : "0.75rem") : "calc(env(safe-area-inset-bottom) + 12px)",
        borderRadius: embedded ? 0 : "var(--radius-lg)",
        borderColor: embedded ? "transparent" : "var(--pad-edge)",
        touchAction: "none",
        overscrollBehavior: "contain",
        backgroundColor: embedded ? "transparent" : "var(--pad-surface)",
        boxShadow: embedded ? "none" : "0 14px 34px rgba(3, 8, 24, 0.12)",
        color: "var(--pad-ink)",
        border: embedded ? "none" : undefined,
        ["--control-size" as string]: controlSize,
        ["--control-gap" as string]: controlGap,
        ["--control-label-size" as string]: labelSize,
        ["--pad-ink" as string]: "var(--color-ink)",
        ["--pad-ink-muted" as string]: "var(--color-muted)",
        ["--pad-edge" as string]: "var(--color-panel-border)",
        ["--pad-accent" as string]: "var(--color-accent)",
        ["--pad-accent-soft" as string]: "var(--color-accent-soft)",
        ["--pad-surface" as string]: "var(--color-panel-soft)",
        ["--pad-surface-weak" as string]: "var(--color-panel-ghost)",
      }}
    >
      <div className={isEmbeddedCompact ? "space-y-1.5" : compact ? "space-y-2" : "space-y-4"}>
        {showHeader ? (
          <div className="flex flex-col items-start justify-between gap-2 md:flex-row md:items-center">
            <div className="card card-compact w-fit rounded-[var(--radius-sm)]">
              <div className="card-body p-2.5 pt-2">
                <p className="hidden text-xs font-semibold uppercase tracking-[0.2em] text-[var(--pad-ink-muted)] sm:inline-flex">
                  Control deck
                </p>
                <h3 className="text-[1.35rem] font-bold tracking-[-0.02em] sm:text-[1.75rem]">Play bar</h3>
              </div>
            </div>
          </div>
        ) : null}
        <div
          className={`grid w-full ${isEmbeddedCompact ? "gap-2 p-1" : "gap-3 p-2"}`}
          style={{
            gridTemplateColumns: "minmax(0, 1fr) minmax(0, 1fr)",
            gridTemplateAreas: '"dpad ab" "system system"',
          }}
        >
          <div
            className="card card-bordered w-full border border-[var(--pad-edge)] shadow-sm"
            style={{
              display: "flex",
              flexDirection: "column",
              alignItems: "center",
              gap: "var(--control-gap, 8px)",
                justifyContent: "flex-start",
                gridArea: "dpad",
                padding: cardPadding,
                borderRadius: "var(--radius-sm)",
                backgroundColor: "var(--pad-surface)",
              }}
          >
            <p className="hidden text-xs font-semibold uppercase tracking-[0.18em] text-[var(--pad-ink-muted)] sm:inline-flex">D-pad</p>
            <div
              style={{
                display: "grid",
                gridTemplateColumns: "repeat(3, var(--control-size, 64px))",
                gridTemplateRows: "repeat(3, var(--control-size, 64px))",
                gap: "var(--control-gap, 8px)",
              }}
            >
              <div />
              <GamepadButton control="up" pressed={pressedButtonSet.has("up")} compact={compact} onPressChange={handlePressChange} />
              <div />
              <GamepadButton control="left" pressed={pressedButtonSet.has("left")} compact={compact} onPressChange={handlePressChange} />
              <div />
              <GamepadButton control="right" pressed={pressedButtonSet.has("right")} compact={compact} onPressChange={handlePressChange} />
              <div />
              <GamepadButton control="down" pressed={pressedButtonSet.has("down")} compact={compact} onPressChange={handlePressChange} />
              <div />
            </div>
          </div>

          <div
            className="card card-bordered w-full border border-[var(--pad-edge)] shadow-sm"
            style={{
              display: "flex",
              flexDirection: "column",
              alignItems: "center",
              gap: "var(--control-gap, 8px)",
              justifyContent: "flex-start",
              gridArea: "ab",
              padding: cardPadding,
              borderRadius: "var(--radius-sm)",
              backgroundColor: "var(--pad-surface)",
            }}
          >
            <p className="hidden text-xs font-semibold uppercase tracking-[0.18em] text-[var(--pad-ink-muted)] sm:inline-flex">Action</p>
            <div
              style={{
                width: "100%",
                display: "grid",
                gridTemplateColumns: "repeat(3, var(--control-size, 64px))",
                gridTemplateRows: "repeat(3, var(--control-size, 64px))",
                gap: "var(--control-gap)",
              }}
            >
              <div />
              <div />
              <GamepadButton control="a" pressed={pressedButtonSet.has("a")} compact={compact} onPressChange={handlePressChange} />
              <div />
              <GamepadButton control="b" pressed={pressedButtonSet.has("b")} compact={compact} onPressChange={handlePressChange} />
              <div />
              <div />
              <div />
              <div />
            </div>
          </div>

          <div
            className="card card-bordered w-full border border-[var(--pad-edge)] shadow-sm"
            style={{
              display: "flex",
              flexDirection: isEmbeddedCompact ? "row" : "column",
              alignItems: "center",
              gap: "var(--control-gap, 8px)",
              justifyContent: isEmbeddedCompact ? "space-between" : "flex-start",
              gridArea: "system",
              padding: cardPadding,
              borderRadius: "var(--radius-sm)",
              backgroundColor: "var(--pad-surface)",
            }}
          >
            <p className="hidden text-xs font-semibold uppercase tracking-[0.18em] text-[var(--pad-ink-muted)] sm:inline-flex">System</p>
            <div className="flex items-center justify-center" style={{ gap: "var(--control-gap)" }}>
              <GamepadButton control="select" pressed={pressedButtonSet.has("select")} compact={compact} onPressChange={handlePressChange} />
              <GamepadButton control="start" pressed={pressedButtonSet.has("start")} compact={compact} onPressChange={handlePressChange} />
            </div>
            {systemControl ? (
              <div
                style={{
                  width: isEmbeddedCompact ? "auto" : "100%",
                  flex: isEmbeddedCompact ? "1 1 0" : undefined,
                  maxWidth: isEmbeddedCompact
                    ? "min(100%, calc(var(--control-size, 64px) * 4.25))"
                    : "calc(var(--control-size, 64px) * 3.2)",
                }}
              >
                {systemControl}
              </div>
            ) : null}
          </div>
        </div>

        <div className="hidden flex-col gap-2 sm:flex sm:flex-row sm:items-center">
          <div
            className="card card-bordered border border-[var(--pad-edge)] bg-[var(--pad-surface)] p-3 shadow-sm"
            style={{ minWidth: 180 }}
          >
            <p className="text-xs font-semibold uppercase tracking-[0.14em] text-[var(--pad-ink-muted)]">Pressed</p>
            <div className="mt-2 flex flex-wrap gap-2">
              {pressedButtonLabels.length ? (
                pressedButtonLabels.map((label, index) => <KeyBadge key={`${label}-${index}`} label={label} />)
              ) : (
                <KeyBadge label="None" />
              )}
            </div>
          </div>
          <div
            className="card card-bordered border border-[var(--pad-edge)] bg-[var(--pad-surface)] p-3 shadow-sm"
            style={{ minWidth: 180 }}
          >
            <p className="text-xs font-semibold uppercase tracking-[0.14em] text-[var(--pad-ink-muted)]">Held keys</p>
            <div className="mt-2 flex flex-wrap gap-2">
              {pressedKeyLabels.length ? (
                pressedKeyLabels.map((label, index) => <KeyBadge key={`${label}-${index}`} label={label} />)
              ) : (
                <KeyBadge label="None" />
              )}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
});

VirtualGamepad.displayName = "VirtualGamepad";

export default VirtualGamepad;
