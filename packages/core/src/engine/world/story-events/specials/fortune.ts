import type { LinkBattleStats } from "@pokecrystal/core/core/models";

export enum FortuneTellerState {
  IDLE = 0,
  SCROLLING = 1,
  READY = 2,
}

export type FortuneOutcome = {
  label: string;
  description: string;
};

export const CELADON_OUTCOMES: FortuneOutcome[] = [
  {
    label: "CeladonFortuneTellerGreatText",
    description: "A spectacular twist of fate awaits.",
  },
  {
    label: "CeladonFortuneTellerGoodText",
    description: "Your days will stay steady and prosperous.",
  },
  {
    label: "CeladonFortuneTellerNeutralText",
    description: "Things will be just right-neither peril nor windfall.",
  },
  {
    label: "CeladonFortuneTellerCautionText",
    description: "Temper your zeal; a stumbling block may appear.",
  },
];

export const GOLDENROD_OUTCOMES: FortuneOutcome[] = [
  {
    label: "GoldenrodFortuneTellerWinsText",
    description: "Fierce victories are on your horizon.",
  },
  {
    label: "GoldenrodFortuneTellerEvenText",
    description: "Balance is the key-recent wins and losses even out.",
  },
  {
    label: "GoldenrodFortuneTellerLossesText",
    description: "Guard your pride; tricky battles could follow.",
  },
];

export const chooseCeladonOutcome = (
  personality: number | null | undefined
): [number, FortuneOutcome] => {
  const value = personality ?? 0;
  const mixed = value ^ (value >> 16);
  const index = mixed % CELADON_OUTCOMES.length;
  return [index, CELADON_OUTCOMES[index]];
};

export const chooseGoldenrodOutcome = (
  stats: LinkBattleStats | null | undefined
): [number, FortuneOutcome] => {
  const safeStats = stats ?? { wins: 0, losses: 0, draws: 0 };
  const total = safeStats.wins + safeStats.losses;
  const seed =
    total === 0
      ? safeStats.draws
      : safeStats.wins * 3 - safeStats.losses * 2 + safeStats.draws;
  const index = (seed & 0xff) % GOLDENROD_OUTCOMES.length;
  return [index, GOLDENROD_OUTCOMES[index]];
};
