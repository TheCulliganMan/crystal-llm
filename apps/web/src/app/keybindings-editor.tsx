"use client";

import React, { useCallback, useEffect, useMemo, useState } from "react";

import { GameButton } from "@pokecrystal/core/input/config";
import {
  getActiveKeyBindings,
  getKeyBindingsChangeEventName,
  resetKeyBindings,
  updateBindingForButton,
} from "@pokecrystal/core/input/user-bindings";

const KEY_LABEL_OVERRIDES: Record<string, string> = {
  Space: "Space",
  Enter: "Enter",
  NumpadEnter: "Numpad Enter",
  Backspace: "Backspace",
  ShiftLeft: "Shift",
  ShiftRight: "Shift",
  Escape: "Escape",
};

const formatKeyLabel = (value: string): string => {
  if (value.startsWith("Key") && value.length > 3) {
    return value.slice(3);
  }
  return KEY_LABEL_OVERRIDES[value] ?? value;
};

const BUTTON_LABELS: Record<GameButton, string> = {
  [GameButton.A]: "A",
  [GameButton.B]: "B",
  [GameButton.Start]: "Start",
  [GameButton.Select]: "Select",
};

type CaptureState = {
  button: GameButton;
  mode: "primary" | "add";
};

export const KeybindingsEditor = React.memo(() => {
  const [bindingsVersion, setBindingsVersion] = useState(0);
  const [capture, setCapture] = useState<CaptureState | null>(null);
  const [lastCaptured, setLastCaptured] = useState<string | null>(null);

  const bindings = useMemo(() => {
    // Re-evaluate when changed.
    void bindingsVersion;
    return getActiveKeyBindings();
  }, [bindingsVersion]);

  useEffect(() => {
    if (typeof window === "undefined") {
      return;
    }
    const eventName = getKeyBindingsChangeEventName();
    const handler = () => setBindingsVersion((value) => value + 1);
    window.addEventListener(eventName, handler);
    window.addEventListener("storage", handler);
    return () => {
      window.removeEventListener(eventName, handler);
      window.removeEventListener("storage", handler);
    };
  }, []);

  useEffect(() => {
    if (!capture || typeof window === "undefined") {
      return;
    }

    const handleKeyDown = (event: KeyboardEvent) => {
      if (
        event.code === "ShiftLeft" ||
        event.code === "ShiftRight" ||
        event.code === "ControlLeft" ||
        event.code === "ControlRight" ||
        event.code === "AltLeft" ||
        event.code === "AltRight" ||
        event.code === "MetaLeft" ||
        event.code === "MetaRight"
      ) {
        return;
      }

      const code = event.code ?? "";
      if (!code) {
        return;
      }

      const current = getActiveKeyBindings();
      const existing = current[capture.button] ?? [];
      const next =
        capture.mode === "primary"
          ? [code, ...existing.filter((value) => value !== code)]
          : [...existing.filter((value) => value !== code), code];

      updateBindingForButton(capture.button, next);
      setLastCaptured(code);
      setCapture(null);
      event.preventDefault();
      event.stopPropagation();
    };

    window.addEventListener("keydown", handleKeyDown, { capture: true });
    return () => {
      window.removeEventListener("keydown", handleKeyDown, { capture: true } as never);
    };
  }, [capture]);

  const beginCapture = useCallback((button: GameButton, mode: CaptureState["mode"]) => {
    setLastCaptured(null);
    setCapture({ button, mode });
  }, []);

  const clearButton = useCallback((button: GameButton) => {
    updateBindingForButton(button, []);
  }, []);

  const resetAll = useCallback(() => {
    if (typeof window !== "undefined") {
      const ok = window.confirm("Reset key bindings to defaults?");
      if (!ok) {
        return;
      }
    }
    resetKeyBindings();
  }, []);

  return (
    <section className="card card-bordered bg-base-200">
      <div className="card-body space-y-4">
        <header className="flex flex-wrap items-center justify-between gap-3">
          <div>
            <p className="text-xs font-semibold uppercase tracking-wide text-base-content/60">Controls</p>
            <h2 className="card-title">Key Bindings</h2>
          </div>
          <button type="button" className="btn btn-sm btn-outline" onClick={resetAll}>
            Reset
          </button>
        </header>

        <p className="text-sm text-base-content/70">
          {capture
            ? `Press a key for ${BUTTON_LABELS[capture.button]} (${capture.mode === "primary" ? "primary" : "add"}).`
            : "Remap A/B/Start/Select. D-pad is fixed to the arrow keys."}
        </p>

        {lastCaptured ? (
          <p className="text-xs text-base-content/70" aria-live="polite">
            Captured: {formatKeyLabel(lastCaptured)}
          </p>
        ) : null}

        <div className="divider" />

        <div className="space-y-2">
          {(Object.values(GameButton) as GameButton[]).map((button) => {
            const keys = bindings[button] ?? [];
            return (
                <div key={button} className="card card-bordered bg-base-200">
                  <div className="card-body gap-2 p-3">
                  <div>
                    <p className="font-semibold">{BUTTON_LABELS[button]}</p>
                    <p className="text-sm text-base-content/70">
                      {keys.length ? keys.map(formatKeyLabel).join(" · ") : "Unbound"}
                    </p>
                  </div>
                  <div className="flex flex-wrap gap-2">
                    <button
                      type="button"
                      className={`btn btn-sm ${
                        capture?.button === button && capture.mode === "primary" ? "btn-primary" : "btn-outline"
                      }`}
                      onClick={() => beginCapture(button, "primary")}
                    >
                      Set primary
                    </button>
                    <button
                      type="button"
                      className={`btn btn-sm ${
                        capture?.button === button && capture.mode === "add" ? "btn-primary" : "btn-outline"
                      }`}
                      onClick={() => beginCapture(button, "add")}
                    >
                      Add
                    </button>
                    <button
                      type="button"
                      className="btn btn-sm btn-error btn-outline"
                      onClick={() => clearButton(button)}
                    >
                      Clear
                    </button>
                  </div>
                </div>
              </div>
            );
          })}
        </div>
      </div>
    </section>
  );
});

KeybindingsEditor.displayName = "KeybindingsEditor";

export default KeybindingsEditor;
