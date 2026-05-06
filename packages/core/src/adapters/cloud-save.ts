export type CloudSaveSnapshot = Record<string, unknown>;

export type CloudSaveMetadata = {
  updated_at?: string | null;
  saved_at?: string | null;
};

export type CloudSaveRecord = {
  payload: CloudSaveSnapshot;
} & CloudSaveMetadata;

export type BrowserCloudSaveAdapter = {
  isConfigured: () => boolean;
  load: (slot: string) => Promise<CloudSaveRecord | null>;
  save: (
    slot: string,
    snapshot: CloudSaveSnapshot,
    savedAt?: string,
  ) => Promise<boolean>;
  delete: (slot: string) => Promise<boolean>;
};

export type IdentityCloudSaveAdapter = {
  loadForIdentity: (slot: string, playerId: string) => Promise<CloudSaveSnapshot | null>;
  saveForIdentity: (
    slot: string,
    playerId: string,
    snapshot: CloudSaveSnapshot,
  ) => Promise<void>;
  deleteForIdentity: (slot: string, playerId: string) => Promise<boolean>;
};

let browserCloudSaveAdapter: BrowserCloudSaveAdapter | null = null;
let identityCloudSaveAdapter: IdentityCloudSaveAdapter | null = null;

export const setBrowserCloudSaveAdapter = (
  adapter: BrowserCloudSaveAdapter | null,
): void => {
  browserCloudSaveAdapter = adapter;
};

export const setIdentityCloudSaveAdapter = (
  adapter: IdentityCloudSaveAdapter | null,
): void => {
  identityCloudSaveAdapter = adapter;
};

export const resetCloudSaveAdapters = (): void => {
  browserCloudSaveAdapter = null;
  identityCloudSaveAdapter = null;
};

export const isBrowserCloudSaveConfigured = (): boolean =>
  Boolean(browserCloudSaveAdapter?.isConfigured());

export const loadBrowserCloudSave = async (
  slot: string,
): Promise<CloudSaveRecord | null> => {
  if (!browserCloudSaveAdapter || !browserCloudSaveAdapter.isConfigured()) {
    return null;
  }
  return browserCloudSaveAdapter.load(slot);
};

export const saveBrowserCloudSave = async (
  slot: string,
  snapshot: CloudSaveSnapshot,
  savedAt?: string,
): Promise<boolean> => {
  if (!browserCloudSaveAdapter || !browserCloudSaveAdapter.isConfigured()) {
    return false;
  }
  return browserCloudSaveAdapter.save(slot, snapshot, savedAt);
};

export const deleteBrowserCloudSave = async (slot: string): Promise<boolean> => {
  if (!browserCloudSaveAdapter || !browserCloudSaveAdapter.isConfigured()) {
    return false;
  }
  return browserCloudSaveAdapter.delete(slot);
};

export const loadIdentityCloudSave = async (
  slot: string,
  playerId: string,
): Promise<CloudSaveSnapshot | null> => {
  if (!identityCloudSaveAdapter) {
    throw new Error(
      `[save] identity cloud-save adapter unavailable for identity load ${playerId}:${slot}.`,
    );
  }
  return identityCloudSaveAdapter.loadForIdentity(slot, playerId);
};

export const saveIdentityCloudSave = async (
  slot: string,
  playerId: string,
  snapshot: CloudSaveSnapshot,
): Promise<void> => {
  if (!identityCloudSaveAdapter) {
    throw new Error(
      `[save] identity cloud-save adapter unavailable for identity save ${playerId}:${slot}.`,
    );
  }
  await identityCloudSaveAdapter.saveForIdentity(slot, playerId, snapshot);
};

export const deleteIdentityCloudSave = async (
  slot: string,
  playerId: string,
): Promise<boolean> => {
  if (!identityCloudSaveAdapter) {
    throw new Error(
      `[save] identity cloud-save adapter unavailable for identity delete ${playerId}:${slot}.`,
    );
  }
  return identityCloudSaveAdapter.deleteForIdentity(slot, playerId);
};
