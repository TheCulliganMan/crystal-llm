export type StoryEventScriptConstantsPayload = {
  global?: Record<string, unknown>;
  maps?: Record<string, Record<string, unknown>>;
};

export type CurrencyConstantsPayload = Record<string, number>;

type ConstantSource = {
  scope: string;
  value: number;
};

const currencyValueOrNull = (name: string, source: string, value: unknown): number | null => {
  if (typeof value !== "number" || !Number.isInteger(value)) {
    throw new Error(`Script constant ${source}.${name} must be an explicit integer.`);
  }
  if (value < 0) {
    return null;
  }
  if (value > 0xffffffff) {
    throw new Error(`Currency constant ${source}.${name} exceeds u32 range.`);
  }
  return value;
};

export function exportCurrencyConstants(
  storyEventScriptConstants: StoryEventScriptConstantsPayload
): CurrencyConstantsPayload {
  const constants = new Map<string, ConstantSource>();

  const addConstant = (name: string, source: string, rawValue: unknown): void => {
    const value = currencyValueOrNull(name, source, rawValue);
    if (value === null) {
      return;
    }
    const existing = constants.get(name);
    if (existing) {
      throw new Error(
        `Currency constant ${name} is defined by both ${existing.scope} and ${source}; constants must be exported once.`
      );
    }
    constants.set(name, { scope: source, value });
  };

  for (const [name, value] of Object.entries(storyEventScriptConstants.global ?? {})) {
    addConstant(name, "global", value);
  }
  for (const [mapName, mapConstants] of Object.entries(storyEventScriptConstants.maps ?? {})) {
    for (const [name, value] of Object.entries(mapConstants)) {
      addConstant(name, mapName, value);
    }
  }

  return Object.fromEntries(
    [...constants.entries()]
      .sort(([left], [right]) => left.localeCompare(right))
      .map(([name, source]) => [name, source.value])
  );
}
