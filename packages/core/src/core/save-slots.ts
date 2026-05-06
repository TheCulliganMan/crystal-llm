export const SAVE_SLOT_EXTENSION = ".sav";

export const MANUAL_SAVE_SLOT = `savegame${SAVE_SLOT_EXTENSION}`;
export const MANUAL_SAVE_HISTORY_SLOTS = [
  `savegame-recent-1${SAVE_SLOT_EXTENSION}`,
  `savegame-recent-2${SAVE_SLOT_EXTENSION}`,
];
export const MANUAL_SAVE_SLOTS = [MANUAL_SAVE_SLOT, ...MANUAL_SAVE_HISTORY_SLOTS];

export const AUTOSAVE_SLOT = `autosave${SAVE_SLOT_EXTENSION}`;

export const stripSaveExtension = (slot: string): string => {
  if (slot.endsWith(SAVE_SLOT_EXTENSION)) {
    return slot.slice(0, -SAVE_SLOT_EXTENSION.length);
  }
  return slot;
};
