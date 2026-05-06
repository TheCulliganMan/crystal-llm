// ASM mapping:
// - pokecrystal_disassembly/constants/ram_constants.asm (NUM_JOHTO_BADGES/NUM_KANTO_BADGES)
// - pokecrystal_disassembly/constants/engine_flags.asm (badge engine-flag order)

export const NUM_JOHTO_BADGES = 8;
export const NUM_KANTO_BADGES = 8;
export const NUM_BADGES = NUM_JOHTO_BADGES + NUM_KANTO_BADGES;

export const BADGE_ENGINE_FLAG_ORDER: ReadonlyArray<string> = [
  "ENGINE_ZEPHYRBADGE",
  "ENGINE_HIVEBADGE",
  "ENGINE_PLAINBADGE",
  "ENGINE_FOGBADGE",
  "ENGINE_MINERALBADGE",
  "ENGINE_STORMBADGE",
  "ENGINE_GLACIERBADGE",
  "ENGINE_RISINGBADGE",
  "ENGINE_BOULDERBADGE",
  "ENGINE_CASCADEBADGE",
  "ENGINE_THUNDERBADGE",
  "ENGINE_RAINBOWBADGE",
  "ENGINE_SOULBADGE",
  "ENGINE_MARSHBADGE",
  "ENGINE_VOLCANOBADGE",
  "ENGINE_EARTHBADGE",
];

type BadgeCollectionLike = {
  johto?: readonly boolean[];
  kanto?: readonly boolean[];
};

const assertBadgeBank = (
  bank: readonly boolean[] | undefined,
  expectedLength: number,
  label: string,
  context: string
): readonly boolean[] => {
  if (!Array.isArray(bank)) {
    throw new Error(`${context}: ${label} badge bank must be an array.`);
  }
  if (bank.length !== expectedLength) {
    throw new Error(`${context}: ${label} badge bank must contain exactly ${expectedLength} badges.`);
  }
  for (let index = 0; index < bank.length; index++) {
    if (typeof bank[index] !== "boolean") {
      throw new Error(`${context}: ${label} badge ${index} must be boolean.`);
    }
  }
  return bank;
};

export const assertAsmJohtoBadgeBank = (
  johto: readonly boolean[] | undefined,
  context = "Badge state"
): readonly boolean[] => assertBadgeBank(johto, NUM_JOHTO_BADGES, "johto", context);

export const assertAsmBadgeBanks = (
  badges: BadgeCollectionLike | null | undefined,
  context = "Badge state"
): { johto: readonly boolean[]; kanto: readonly boolean[] } => {
  if (!badges) {
    throw new Error(`${context}: badge state is missing.`);
  }
  return {
    johto: assertBadgeBank(badges.johto, NUM_JOHTO_BADGES, "johto", context),
    kanto: assertBadgeBank(badges.kanto, NUM_KANTO_BADGES, "kanto", context),
  };
};

export const countOwnedBadgesAsm = (badges: BadgeCollectionLike | null | undefined, context = "Badge state"): number => {
  const { johto, kanto } = assertAsmBadgeBanks(badges, context);
  let count = 0;
  for (const owned of johto) {
    if (owned) {
      count++;
    }
  }
  for (const owned of kanto) {
    if (owned) {
      count++;
    }
  }
  return count;
};

export const hasOwnedBadgeAsm = (
  badges: BadgeCollectionLike | null | undefined,
  badgeId: number,
  context = "Badge state"
): boolean => {
  if (!Number.isInteger(badgeId)) {
    throw new Error(`${context}: badge id ${badgeId} must be an integer.`);
  }
  if (badgeId < 0 || badgeId >= NUM_BADGES) {
    throw new Error(`${context}: badge id ${badgeId} is out of ASM range 0-${NUM_BADGES - 1}.`);
  }
  const { johto, kanto } = assertAsmBadgeBanks(badges, context);
  if (badgeId < NUM_JOHTO_BADGES) {
    return johto[badgeId];
  }
  return kanto[badgeId - NUM_JOHTO_BADGES];
};

export const setOwnedBadgeByEngineFlagAsm = (
  badges: BadgeCollectionLike | null | undefined,
  flagName: string,
  owned: boolean,
  context = "Badge state"
): boolean => {
  const normalizedFlagName = String(flagName ?? "").replace(/,+$/, "").trim().toUpperCase();
  const badgeId = BADGE_ENGINE_FLAG_ORDER.indexOf(normalizedFlagName);
  if (badgeId < 0) {
    return false;
  }
  const { johto, kanto } = assertAsmBadgeBanks(badges, context);
  if (badgeId < NUM_JOHTO_BADGES) {
    (johto as boolean[])[badgeId] = owned;
  } else {
    (kanto as boolean[])[badgeId - NUM_JOHTO_BADGES] = owned;
  }
  return true;
};

export const johtoBadgeMaskAsm = (johto: readonly boolean[] | undefined, context = "Badge state"): number => {
  const johtoBadges = assertAsmJohtoBadgeBank(johto, context);
  let mask = 0;
  for (let index = 0; index < NUM_JOHTO_BADGES; index++) {
    if (johtoBadges[index]) {
      mask |= 1 << index;
    }
  }
  return mask;
};
