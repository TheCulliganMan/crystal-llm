export const normalise_trainer_id = (trainerClass: string): string => {
  const raw = trainerClass.trim().toLowerCase();
  if (!raw) {
    return raw;
  }
  if (raw.endsWith("m") && !raw.endsWith("_m")) {
    return `${raw.slice(0, -1)}_m`;
  }
  if (raw.endsWith("f") && !raw.endsWith("_f")) {
    return `${raw.slice(0, -1)}_f`;
  }
  return raw;
};
