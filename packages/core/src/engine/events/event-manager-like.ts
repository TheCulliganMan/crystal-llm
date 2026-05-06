export type EventManagerLike = {
  on(eventName: string, listener: (event: unknown, gameState: unknown) => void): void;
  dispatch(event: unknown, options?: { priority?: number; delay?: number }): void;
};
