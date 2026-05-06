import type { DV } from "./models/pokemon";

export const SHINY_ATTACK_DVS = [2, 3, 6, 7, 10, 11, 14, 15] as const;

export const FORCED_SHINY_DVS: DV = {
  attack: 14,
  defense: 10,
  speed: 10,
  special: 10,
  hp: 0,
};

export const deriveHpDv = (dvs: Pick<DV, "attack" | "defense" | "speed" | "special">): number => {
  const attack = Math.trunc(dvs.attack) & 0xf;
  const defense = Math.trunc(dvs.defense) & 0xf;
  const speed = Math.trunc(dvs.speed) & 0xf;
  const special = Math.trunc(dvs.special) & 0xf;
  return ((attack & 1) << 3) | ((defense & 1) << 2) | ((speed & 1) << 1) | (special & 1);
};

export const normalizeDvs = (dvs: Pick<DV, "attack" | "defense" | "speed" | "special">): DV => {
  const normalized = {
    attack: Math.trunc(dvs.attack) & 0xf,
    defense: Math.trunc(dvs.defense) & 0xf,
    speed: Math.trunc(dvs.speed) & 0xf,
    special: Math.trunc(dvs.special) & 0xf,
  };
  return {
    ...normalized,
    hp: deriveHpDv(normalized),
  };
};

export const isShinyDvs = (dvs: Pick<DV, "attack" | "defense" | "speed" | "special"> | null | undefined): boolean => {
  if (!dvs) {
    return false;
  }
  const attack = Math.trunc(dvs.attack) & 0xf;
  return (
    (Math.trunc(dvs.defense) & 0xf) === 10 &&
    (Math.trunc(dvs.speed) & 0xf) === 10 &&
    (Math.trunc(dvs.special) & 0xf) === 10 &&
    SHINY_ATTACK_DVS.includes(attack as (typeof SHINY_ATTACK_DVS)[number])
  );
};
