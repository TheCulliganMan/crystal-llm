export type MultiplayerUser = {
  id: string;
};

export type MultiplayerRealtimeChannel = {
  state?: string;
  on: (
    type: string,
    filter: { event: string } & Record<string, unknown>,
    callback: (payload: any) => void,
  ) => MultiplayerRealtimeChannel;
  subscribe: (callback?: (status: string) => void) => MultiplayerRealtimeChannel;
  send: (message: { type: string; event: string; payload: unknown }) => Promise<unknown>;
  track?: (payload: Record<string, unknown>) => Promise<unknown>;
  presenceState?: () => Record<string, Array<Record<string, unknown>>>;
};

export type MultiplayerClient = {
  auth: {
    getUser: () => Promise<{ data: { user: MultiplayerUser | null } }>;
  };
  channel: (name: string, options?: unknown) => MultiplayerRealtimeChannel;
  removeChannel: (channel: MultiplayerRealtimeChannel) => Promise<unknown>;
  from: (table: string) => any;
};

export type MultiplayerClientFactory = () => MultiplayerClient | null;

let multiplayerClientFactory: MultiplayerClientFactory = () => null;

export const setMultiplayerClientFactory = (
  factory: MultiplayerClientFactory | null,
): void => {
  multiplayerClientFactory = factory ?? (() => null);
};

export const resetMultiplayerClientFactory = (): void => {
  multiplayerClientFactory = () => null;
};

export const createMultiplayerClient = (): MultiplayerClient | null =>
  multiplayerClientFactory();
