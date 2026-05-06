export type PendingEventFlagUpdate = [string, boolean] | [string, boolean, boolean];

export function normalizePendingEventFlagUpdate(
  update: PendingEventFlagUpdate
): [string, boolean, boolean] {
  if (update.length === 3) {
    const [eventName, value, shouldRefresh] = update;
    return [eventName, value, shouldRefresh];
  }

  const [eventName, value] = update;
  return [eventName, value, true];
}
