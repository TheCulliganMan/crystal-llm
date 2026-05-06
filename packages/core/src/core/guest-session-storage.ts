export const GUEST_SESSION_PREFIX = "pokecrystal:guest-save:";
export const GUEST_SESSION_METADATA_PREFIX = "pokecrystal:guest-save-meta:";
const LEGACY_FS_PREFIX = "fs:";
const LEGACY_SAVE_SUFFIX = ".sav";

type StorageCandidate = {
  storage: Storage;
  label: "local" | "session";
};

export type GuestSessionSlotCandidate = {
  payload: string;
  metadata: string | null;
  sourceKey: string;
  storageLabel: "local" | "session" | "legacy";
};

const resolveGuestStorageCandidates = (): StorageCandidate[] => {
  if (typeof window === "undefined") {
    return [];
  }
  const candidates: StorageCandidate[] = [];
  try {
    candidates.push({ storage: window.localStorage, label: "local" });
  } catch {}
  try {
    if (!candidates.some((candidate) => candidate.storage === window.sessionStorage)) {
      candidates.push({ storage: window.sessionStorage, label: "session" });
    }
  } catch {}
  return candidates;
};

const resolveGuestStorage = (): StorageCandidate | null => {
  const [candidate] = resolveGuestStorageCandidates();
  if (!candidate) {
    return null;
  }
  return candidate;
};

const resolveLegacyStorage = (): Storage | null => {
  if (typeof window === "undefined") {
    return null;
  }
  try {
    return window.localStorage;
  } catch {
    return null;
  }
};

const slotFromLegacyKey = (key: string): string | null => {
  if (!key.startsWith(LEGACY_FS_PREFIX)) {
    return null;
  }
  const path = key.slice(LEGACY_FS_PREFIX.length);
  const separator = path.lastIndexOf("/");
  const slot = separator >= 0 ? path.slice(separator + 1) : path;
  return slot.length > 0 ? slot : null;
};

const listLegacySlots = (): string[] => {
  const storage = resolveLegacyStorage();
  if (!storage) {
    return [];
  }
  const slots = new Set<string>();
  for (let index = 0; index < storage.length; index += 1) {
    const key = storage.key(index);
    if (!key) {
      continue;
    }
    const slot = slotFromLegacyKey(key);
    if (!slot || !slot.endsWith(LEGACY_SAVE_SUFFIX)) {
      continue;
    }
    slots.add(slot);
  }
  return Array.from(slots);
};

const findLegacyKeyForSlot = (slot: string): string | null => {
  const storage = resolveLegacyStorage();
  if (!storage) {
    return null;
  }
  for (let index = 0; index < storage.length; index += 1) {
    const key = storage.key(index);
    if (!key) {
      continue;
    }
    const candidate = slotFromLegacyKey(key);
    if (candidate === slot) {
      return key;
    }
  }
  return null;
};

export const getGuestSessionStorage = (): Storage | null => {
  return resolveGuestStorage()?.storage ?? null;
};

export const getGuestSessionStorageLabel = (): "local" | "session" | "none" => {
  return resolveGuestStorage()?.label ?? "none";
};

export const guestSessionKey = (slot: string): string => {
  return `${GUEST_SESSION_PREFIX}${slot}`;
};

export const guestSessionMetadataKey = (slot: string): string => {
  return `${GUEST_SESSION_METADATA_PREFIX}${slot}`;
};

const resolveGuestSessionAliases = (slot: string): string[] => {
  const aliases = new Set<string>([slot]);
  if (slot.endsWith(LEGACY_SAVE_SUFFIX)) {
    aliases.add(slot.slice(0, -LEGACY_SAVE_SUFFIX.length));
  } else {
    aliases.add(`${slot}${LEGACY_SAVE_SUFFIX}`);
  }
  return Array.from(aliases);
};

export const listGuestSessionSlots = (): string[] => {
  const storages = resolveGuestStorageCandidates().map((candidate) => candidate.storage);
  if (storages.length === 0) {
    return [];
  }
  const slots = new Set<string>();
  for (const storage of storages) {
    for (let index = 0; index < storage.length; index += 1) {
      const key = storage.key(index);
      if (!key || !key.startsWith(GUEST_SESSION_PREFIX)) {
        continue;
      }
      slots.add(key.slice(GUEST_SESSION_PREFIX.length));
    }
  }
  for (const slot of listLegacySlots()) {
    slots.add(slot);
  }
  return Array.from(slots).sort();
};

export const readGuestSessionSlot = (slot: string): string | null => {
  return listGuestSessionSlotCandidates(slot)[0]?.payload ?? null;
};

export const writeGuestSessionSlot = (slot: string, payload: string): boolean => {
  const storages = resolveGuestStorageCandidates().map((candidate) => candidate.storage);
  if (storages.length === 0) {
    return false;
  }
  const key = guestSessionKey(slot);
  for (const storage of storages) {
    try {
      storage.setItem(key, payload);
      return true;
    } catch {}
  }
  return false;
};

export const readGuestSessionMetadata = (slot: string): string | null => {
  const payloadCandidateMetadata = listGuestSessionSlotCandidates(slot)[0]?.metadata;
  if (payloadCandidateMetadata !== undefined) {
    return payloadCandidateMetadata;
  }
  const keys = resolveGuestSessionAliases(slot).map(guestSessionMetadataKey);
  for (const { storage } of resolveGuestStorageCandidates()) {
    for (const key of keys) {
      const direct = storage.getItem(key);
      if (direct !== null) {
        return direct;
      }
    }
  }
  return null;
};

export const listGuestSessionSlotCandidates = (slot: string): GuestSessionSlotCandidate[] => {
  const candidates: GuestSessionSlotCandidate[] = [];
  const seen = new Set<string>();
  const aliases = resolveGuestSessionAliases(slot);
  for (const { storage, label } of resolveGuestStorageCandidates()) {
    for (const alias of aliases) {
      const key = guestSessionKey(alias);
      const dedupeKey = `${label}:${key}`;
      if (seen.has(dedupeKey)) {
        continue;
      }
      const payload = storage.getItem(key);
      if (payload === null) {
        continue;
      }
      seen.add(dedupeKey);
      candidates.push({
        payload,
        metadata: storage.getItem(guestSessionMetadataKey(alias)),
        sourceKey: key,
        storageLabel: label,
      });
    }
  }
  const legacyKey = findLegacyKeyForSlot(slot);
  if (legacyKey) {
    const payload = resolveLegacyStorage()?.getItem(legacyKey) ?? null;
    if (payload !== null) {
      candidates.push({
        payload,
        metadata: null,
        sourceKey: legacyKey,
        storageLabel: "legacy",
      });
    }
  }
  return candidates;
};

export const writeGuestSessionMetadata = (slot: string, payload: string): boolean => {
  const storages = resolveGuestStorageCandidates().map((candidate) => candidate.storage);
  if (storages.length === 0) {
    return false;
  }
  const key = guestSessionMetadataKey(slot);
  for (const storage of storages) {
    try {
      storage.setItem(key, payload);
      return true;
    } catch {}
  }
  return false;
};

export const deleteGuestSessionSlot = (slot: string): boolean => {
  const storages = resolveGuestStorageCandidates().map((candidate) => candidate.storage);
  let deleted = false;
  const keys = resolveGuestSessionAliases(slot).map(guestSessionKey);
  const metadataKeys = resolveGuestSessionAliases(slot).map(guestSessionMetadataKey);
  for (const storage of storages) {
    for (const key of keys) {
      storage.removeItem(key);
    }
    for (const metadataKey of metadataKeys) {
      storage.removeItem(metadataKey);
    }
    deleted = true;
  }
  const legacyKey = findLegacyKeyForSlot(slot);
  if (legacyKey) {
    resolveLegacyStorage()?.removeItem(legacyKey);
    deleted = true;
  }
  return deleted;
};

export const clearGuestSessionSlots = (): number => {
  const slots = listGuestSessionSlots();
  let cleared = 0;
  for (const slot of slots) {
    if (deleteGuestSessionSlot(slot)) {
      cleared += 1;
    }
  }
  return cleared;
};
