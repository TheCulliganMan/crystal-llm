"use client";

import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  clearGuestSessionSlots,
  deleteGuestSessionSlot,
  GUEST_SESSION_PREFIX,
  getGuestSessionStorageLabel,
  listGuestSessionSlots,
  readGuestSessionSlot,
  writeGuestSessionSlot,
} from "@pokecrystal/core/core/guest-session-storage";
import { normalizeSaveSnapshot } from "@pokecrystal/core/core/save";
import { AUTOSAVE_SLOT, MANUAL_SAVE_SLOTS, stripSaveExtension } from "@pokecrystal/core/core/save-slots";

type GuestSlotSummary = {
  slot: string;
  playerName: string | null;
  sizeBytes: number;
  invalid: boolean;
  exists: boolean;
};

const areSlotsEqual = (left: GuestSlotSummary[], right: GuestSlotSummary[]): boolean => {
  if (left === right) {
    return true;
  }
  if (left.length !== right.length) {
    return false;
  }
  for (let idx = 0; idx < left.length; idx += 1) {
    const leftSlot = left[idx];
    const rightSlot = right[idx];
    if (
      leftSlot.slot !== rightSlot.slot ||
      leftSlot.playerName !== rightSlot.playerName ||
      leftSlot.sizeBytes !== rightSlot.sizeBytes ||
      leftSlot.invalid !== rightSlot.invalid ||
      leftSlot.exists !== rightSlot.exists
    ) {
      return false;
    }
  }
  return true;
};

const formatBytes = (bytes: number): string => {
  if (!Number.isFinite(bytes) || bytes <= 0) {
    return "0 B";
  }
  if (bytes < 1024) {
    return `${bytes} B`;
  }
  return `${(bytes / 1024).toFixed(1)} KB`;
};

const parseGuestSlot = (slot: string): GuestSlotSummary => {
  const raw = readGuestSessionSlot(slot);
  if (!raw) {
    return { slot, playerName: null, sizeBytes: 0, invalid: false, exists: false };
  }
  let playerName: string | null = null;
  let invalid = false;
  try {
    const parsed = JSON.parse(raw) as { sram?: { player_name?: string } };
    playerName = parsed?.sram?.player_name ?? null;
  } catch {
    invalid = true;
  }
  return { slot, playerName, sizeBytes: raw.length, invalid, exists: true };
};

const SaveRow = ({
  children,
  actions,
}: {
  children: React.ReactNode;
  actions: React.ReactNode;
}) => (
  <div className="card card-bordered bg-base-200">
    <div className="card-body gap-3 p-3">
      <div className="min-w-0 space-y-1">{children}</div>
      <div className="flex flex-wrap gap-2">{actions}</div>
    </div>
  </div>
);

const SaveTitle = ({ children }: { children: React.ReactNode }) => (
  <h4 className="text-sm font-semibold text-base-content">{children}</h4>
);

const SaveSubtext = ({ children }: { children: React.ReactNode }) => (
  <p className="text-xs text-base-content/70">{children}</p>
);

type GuestSavePanelProps = {
  onLoadSave?: () => void;
};

export const GuestSavePanel = React.memo(({ onLoadSave }: GuestSavePanelProps) => {
  const [slots, setSlots] = useState<GuestSlotSummary[]>([]);
  const [storageLabel, setStorageLabel] = useState<"local" | "session" | "none">("none");
  const uploadInputsRef = useRef<Record<string, HTMLInputElement | null>>({});

  const refresh = useCallback(() => {
    const slotNames = listGuestSessionSlots();
    const nextSlots = slotNames.map(parseGuestSlot);
    setSlots((prev) => (areSlotsEqual(prev, nextSlots) ? prev : nextSlots));
  }, []);

  useEffect(() => {
    if (typeof window === "undefined") {
      return undefined;
    }
    const label = getGuestSessionStorageLabel();
    setStorageLabel(label);
    refresh();
    const handleStorage = (event: StorageEvent) => {
      if (!event.key || !event.key.startsWith(GUEST_SESSION_PREFIX)) {
        return;
      }
      refresh();
    };
    window.addEventListener("storage", handleStorage);
    const interval = label === "none" ? null : window.setInterval(refresh, 1500);
    return () => {
      window.removeEventListener("storage", handleStorage);
      if (interval !== null) {
        window.clearInterval(interval);
      }
    };
  }, [refresh]);

  const handleDeleteSlot = useCallback(
    (slot: string) => {
      deleteGuestSessionSlot(slot);
      refresh();
    },
    [refresh]
  );

  const handleClearAll = useCallback(() => {
    if (typeof window === "undefined") {
      return;
    }
    if (!window.confirm("Delete all guest saves stored in this browser?")) {
      return;
    }
    clearGuestSessionSlots();
    refresh();
  }, [refresh]);

  const hasStorage = storageLabel !== "none";
  const canLoad = hasStorage && slots.length > 0;

  const handleDownloadSlot = useCallback((slot: string) => {
    if (typeof window === "undefined") {
      return;
    }
    const raw = readGuestSessionSlot(slot);
    if (!raw) {
      return;
    }
    const safeName = `${stripSaveExtension(slot)}.sav.json`;
    const blob = new Blob([raw], { type: "application/json" });
    const url = window.URL.createObjectURL(blob);
    const anchor = document.createElement("a");
    anchor.href = url;
    anchor.download = safeName;
    anchor.click();
    window.URL.revokeObjectURL(url);
  }, []);

  const handleUploadSlot = useCallback(
    async (slot: string, file: File) => {
      if (!hasStorage) {
        return;
      }
      let rawText = "";
      try {
        rawText = await file.text();
      } catch (error) {
        window.alert(`Failed to read ${file.name}: ${String(error)}`);
        return;
      }
      let parsed: unknown;
      try {
        parsed = JSON.parse(rawText);
      } catch (error) {
        window.alert(`Save upload failed: ${String(error)}`);
        return;
      }
      let normalized: Record<string, unknown>;
      try {
        normalized = normalizeSaveSnapshot(parsed, `upload:${slot}`);
      } catch (error) {
        window.alert(`Save upload failed: ${String(error)}`);
        return;
      }
      const payload = JSON.stringify(normalized);
      if (!writeGuestSessionSlot(slot, payload)) {
        window.alert("Save upload failed: guest storage unavailable.");
        return;
      }
      refresh();
    },
    [hasStorage, refresh]
  );

  const handleUploadChange = useCallback(
    (slot: string, event: React.ChangeEvent<HTMLInputElement>) => {
      const file = event.target.files?.[0] ?? null;
      event.target.value = "";
      if (!file) {
        return;
      }
      void handleUploadSlot(slot, file);
    },
    [handleUploadSlot]
  );

  const handleUploadButtonClick = useCallback((slot: string) => {
    uploadInputsRef.current[slot]?.click();
  }, []);

  const manualSlots = useMemo(
    () => MANUAL_SAVE_SLOTS.map(parseGuestSlot),
    [slots]
  );
  const autosaveSlot = useMemo(() => parseGuestSlot(AUTOSAVE_SLOT), [slots]);
  const managedSlots = useMemo(() => new Set([...MANUAL_SAVE_SLOTS, AUTOSAVE_SLOT]), []);
  const otherSlots = useMemo(
    () => slots.filter((slot) => !managedSlots.has(slot.slot)),
    [managedSlots, slots]
  );

  return (
    <section className="card card-bordered card-body min-w-0 space-y-3 text-base-content">
      <div className="flex flex-col gap-3">
        <div className="space-y-1">
          <div className="text-xs font-semibold uppercase tracking-wide text-base-content/70">Guest Saves</div>
          <h2 className="text-lg font-semibold">Local save snapshots</h2>
        </div>
        <div className="flex flex-wrap gap-2">
          <button
            type="button"
            className="btn btn-sm btn-outline"
            onClick={refresh}
          >
            Refresh
          </button>
          <button
            type="button"
            className="btn btn-sm btn-outline"
            onClick={onLoadSave}
            disabled={!canLoad || !onLoadSave}
          >
            Reload Save
          </button>
          <button
            type="button"
            className="btn btn-sm btn-error"
            onClick={handleClearAll}
            disabled={!hasStorage || slots.length === 0}
          >
            Clear All
          </button>
        </div>
      </div>

      <p className="text-sm text-base-content/70">
        {hasStorage
          ? `Guest save data lives in ${storageLabel}Storage on this device.`
          : "Guest save storage is unavailable in this session."}
      </p>

      <div className="divider" />

      <section className="space-y-2">
        <h3 className="text-sm font-semibold">Manual saves (most recent 3)</h3>
        {manualSlots.every((slot) => !slot.exists && !slot.invalid) ? (
          <p className="text-sm text-base-content/70">No manual saves found.</p>
        ) : null}
          {manualSlots.map((slot) => (
            <SaveRow
              key={slot.slot}
              actions={
                <>
                  <button
                    type="button"
                    className="btn btn-sm btn-ghost"
                    onClick={() => handleDownloadSlot(slot.slot)}
                    disabled={!slot.exists || slot.invalid}
                  >
                    Download
                  </button>
                  <button
                    type="button"
                    className="btn btn-sm btn-outline"
                    onClick={() => handleUploadButtonClick(slot.slot)}
                    disabled={!hasStorage}
                  >
                    Upload
                  </button>
                    <input
                      className="sr-only"
                      type="file"
                      accept="application/json,.json"
                      ref={(node) => {
                        uploadInputsRef.current[slot.slot] = node;
                      }}
                      onChange={(event) => handleUploadChange(slot.slot, event)}
                      disabled={!hasStorage}
                    />
                  <button
                    type="button"
                    className="btn btn-sm btn-error"
                    onClick={() => handleDeleteSlot(slot.slot)}
                    disabled={!slot.exists}
                  >
                    Delete
                  </button>
                </>
              }
            >
              <SaveTitle>{stripSaveExtension(slot.slot)}</SaveTitle>
              <SaveSubtext>
                {slot.exists
                  ? slot.invalid
                    ? "Invalid data"
                    : `Player: ${slot.playerName ?? "Unknown"} - ${formatBytes(slot.sizeBytes)}`
                  : "Empty"}
              </SaveSubtext>
            </SaveRow>
          ))}
        </section>

        <div className="divider" />

        <section className="space-y-2">
          <h3 className="text-sm font-semibold">Autosave</h3>
          {!autosaveSlot.exists ? (
            <p className="text-sm text-base-content/70">No autosave snapshot available.</p>
          ) : (
            <SaveRow
              actions={
                <button
                  type="button"
                  className="btn btn-sm btn-error"
                  onClick={() => handleDeleteSlot(autosaveSlot.slot)}
                >
                  Delete
                </button>
              }
            >
              <SaveTitle>{stripSaveExtension(autosaveSlot.slot)}</SaveTitle>
              <SaveSubtext>
                {autosaveSlot.invalid
                  ? "Invalid data"
                  : `Player: ${autosaveSlot.playerName ?? "Unknown"} - ${formatBytes(
                      autosaveSlot.sizeBytes
                    )}`}
              </SaveSubtext>
            </SaveRow>
          )}
        </section>

          {otherSlots.length > 0 ? (
            <>
              <div className="divider" />
              <section className="space-y-2">
                <h3 className="text-sm font-semibold">Other saves</h3>
                {otherSlots.map((slot) => (
                  <SaveRow
                    key={slot.slot}
                    actions={
                      <button
                        type="button"
                        className="btn btn-sm btn-error"
                        onClick={() => handleDeleteSlot(slot.slot)}
                      >
                        Delete
                      </button>
                    }
                  >
                    <SaveTitle>{stripSaveExtension(slot.slot)}</SaveTitle>
                    <SaveSubtext>
                      {slot.invalid
                        ? "Invalid data"
                        : `Player: ${slot.playerName ?? "Unknown"} - ${formatBytes(slot.sizeBytes)}`}
                    </SaveSubtext>
                  </SaveRow>
                ))}
              </section>
            </>
          ) : null}
    </section>
  );
});

GuestSavePanel.displayName = "GuestSavePanel";

export default GuestSavePanel;
