/**
 * @jest-environment jsdom
 */

import {
  deleteGuestSessionSlot,
  guestSessionKey,
  guestSessionMetadataKey,
  listGuestSessionSlots,
  readGuestSessionSlot,
  readGuestSessionMetadata,
  writeGuestSessionSlot,
  writeGuestSessionMetadata,
} from "../guest-session-storage";

describe("guest session storage", () => {
  const slot = "guest-fallback-test.sav";
  const key = guestSessionKey(slot);
  const payload = JSON.stringify({ sram: { player_name: "GuestFallback" } });

  beforeEach(() => {
    jest.restoreAllMocks();
    window.localStorage.clear();
    window.sessionStorage.clear();
  });

  afterEach(() => {
    jest.restoreAllMocks();
    window.localStorage.clear();
    window.sessionStorage.clear();
  });

  it("falls back to sessionStorage when localStorage setItem exceeds quota", () => {
    const quotaError = new Error("storage quota exceeded");
    quotaError.name = "QuotaExceededError";

    const originalSetItem = Storage.prototype.setItem;
    let callCount = 0;
    jest.spyOn(Storage.prototype, "setItem").mockImplementation(function (
      this: Storage,
      nextKey: string,
      nextValue: string
    ) {
      callCount += 1;
      if (callCount === 1) {
        throw quotaError;
      }
      return originalSetItem.call(this, nextKey, nextValue);
    });

    expect(writeGuestSessionSlot(slot, payload)).toBe(true);
    expect(window.localStorage.getItem(key)).toBeNull();
    expect(window.sessionStorage.getItem(key)).toBe(payload);
    expect(readGuestSessionSlot(slot)).toBe(payload);
    expect(listGuestSessionSlots()).toContain(slot);
  });

  it("lists and deletes slots written to sessionStorage", () => {
    window.sessionStorage.setItem(key, payload);

    expect(listGuestSessionSlots()).toContain(slot);
    expect(deleteGuestSessionSlot(slot)).toBe(true);
    expect(readGuestSessionSlot(slot)).toBeNull();
  });

  it("stores save metadata separately from the snapshot payload", () => {
    const savedAt = "2026-03-30T12:00:00.000Z";

    expect(writeGuestSessionMetadata(slot, JSON.stringify({ saved_at: savedAt }))).toBe(true);
    expect(window.localStorage.getItem(guestSessionMetadataKey(slot))).toContain(savedAt);
    expect(readGuestSessionMetadata(slot)).toContain(savedAt);

    expect(deleteGuestSessionSlot(slot)).toBe(true);
    expect(readGuestSessionMetadata(slot)).toBeNull();
  });

  it("reads legacy bare guest session slots through the canonical extensionful name", () => {
    const legacySlot = "legacy-browser-save";
    const legacyPayload = JSON.stringify({ sram: { player_name: "LegacyGuest" } });

    window.localStorage.setItem(guestSessionKey(legacySlot), legacyPayload);
    window.localStorage.setItem(
      guestSessionMetadataKey(legacySlot),
      JSON.stringify({ saved_at: "2026-03-30T12:00:00.000Z" })
    );

    expect(readGuestSessionSlot(`${legacySlot}.sav`)).toBe(legacyPayload);
    expect(readGuestSessionMetadata(`${legacySlot}.sav`)).toContain("2026-03-30T12:00:00.000Z");

    expect(deleteGuestSessionSlot(`${legacySlot}.sav`)).toBe(true);
    expect(window.localStorage.getItem(guestSessionKey(legacySlot))).toBeNull();
    expect(window.localStorage.getItem(guestSessionMetadataKey(legacySlot))).toBeNull();
  });
});
