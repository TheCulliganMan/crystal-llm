export type BooleanFlagCollection = Record<string, boolean> | Map<string, boolean>;

const isBooleanFlagMap = (value: unknown): value is Map<string, boolean> =>
  value instanceof Map ||
  (typeof value === "object" &&
    value !== null &&
    typeof (value as Map<string, boolean>).get === "function" &&
    typeof (value as Map<string, boolean>).set === "function" &&
    typeof (value as Map<string, boolean>).clear === "function");

export const getBooleanFlag = (
  collection: BooleanFlagCollection | null | undefined,
  flag: string,
): boolean | undefined => {
  if (!collection) {
    return undefined;
  }
  if (isBooleanFlagMap(collection)) {
    return collection.get(flag);
  }
  return collection[flag];
};

export const setBooleanFlag = (
  collection: BooleanFlagCollection | null | undefined,
  flag: string,
  value: boolean,
): void => {
  if (!collection) {
    return;
  }
  if (isBooleanFlagMap(collection)) {
    collection.set(flag, value);
    return;
  }
  collection[flag] = value;
};

export const clearBooleanFlags = (collection: BooleanFlagCollection | null | undefined): void => {
  if (!collection) {
    return;
  }
  if (isBooleanFlagMap(collection)) {
    collection.clear();
    return;
  }
  Object.keys(collection).forEach((key) => {
    delete collection[key];
  });
};
