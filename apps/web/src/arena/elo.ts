export type EloOutcome = "a" | "b" | "draw";

export type EloSnapshot = {
  ratingA: number;
  ratingB: number;
  nextRatingA: number;
  nextRatingB: number;
  deltaA: number;
  deltaB: number;
  expectedA: number;
  expectedB: number;
};

const DEFAULT_K_FACTOR = 32;
const MIN_ELO_RATING = 100;

const toFiniteNumber = (value: number, label: string): number => {
  if (!Number.isFinite(value)) {
    throw new Error(`${label} must be a finite number.`);
  }
  return value;
};

const clampFloor = (value: number, floor: number): number => Math.max(floor, value);

const roundRating = (value: number): number => Math.round(value);

const outcomeScore = (outcome: EloOutcome): [number, number] => {
  if (outcome === "a") {
    return [1, 0];
  }
  if (outcome === "b") {
    return [0, 1];
  }
  return [0.5, 0.5];
};

const expectedScore = (self: number, opponent: number): number =>
  1 / (1 + 10 ** ((opponent - self) / 400));

export const applyEloRating = (
  ratingA: number,
  ratingB: number,
  outcome: EloOutcome,
  kFactor: number = DEFAULT_K_FACTOR,
): EloSnapshot => {
  const normalizedRatingA = toFiniteNumber(ratingA, "ratingA");
  const normalizedRatingB = toFiniteNumber(ratingB, "ratingB");
  const normalizedK = toFiniteNumber(kFactor, "kFactor");
  if (normalizedK <= 0) {
    throw new Error("kFactor must be positive.");
  }

  const [scoreA, scoreB] = outcomeScore(outcome);
  const expectedA = expectedScore(normalizedRatingA, normalizedRatingB);
  const expectedB = expectedScore(normalizedRatingB, normalizedRatingA);

  const deltaA = normalizedK * (scoreA - expectedA);
  const deltaB = normalizedK * (scoreB - expectedB);

  const nextRatingA = clampFloor(roundRating(normalizedRatingA + deltaA), MIN_ELO_RATING);
  const nextRatingB = clampFloor(roundRating(normalizedRatingB + deltaB), MIN_ELO_RATING);

  return {
    ratingA: normalizedRatingA,
    ratingB: normalizedRatingB,
    nextRatingA,
    nextRatingB,
    deltaA,
    deltaB,
    expectedA,
    expectedB,
  };
};

export const DEFAULT_ARENA_ELO = 1000;
