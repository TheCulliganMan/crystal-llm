/**
 * Multiplayer State Store
 *
 * Zustand store for managing multiplayer connection state, matchmaking,
 * and current match information.
 *
 * Pattern: Follows existing GameState store pattern from /src/app/game.ts
 */

import { create } from 'zustand';
import { useShallow } from 'zustand/shallow';

export type MultiplayerMode = 'battle' | 'trade' | 'time_capsule' | null;
export type ConnectionState = 'disconnected' | 'connecting' | 'connected' | 'error';

export interface MultiplayerState {
  // Connection state
  connectionState: ConnectionState;
  isHost: boolean;

  // Current match
  currentMatchId: string | null;
  currentMatchChannelName: string | null;
  currentModpackId: string | null;
  currentMode: MultiplayerMode;
  opponentId: string | null;
  opponentName: string | null;

  // Matchmaking
  inQueue: boolean;
  queueMode: MultiplayerMode;

  // Error state
  lastError: string | null;
  onlinePlayerCount: number;
  onlineAiCount: number;
  remoteSpritesVisible: boolean;
  crowdViewEnabled: boolean;

  // Actions
  setConnectionState: (state: ConnectionState) => void;
  setMatch: (
    matchId: string,
    opponentId: string,
    opponentName: string,
    mode: MultiplayerMode,
    isHost: boolean,
    modpackId?: string,
    channelName?: string,
  ) => void;
  clearMatch: () => void;
  setInQueue: (inQueue: boolean, mode?: MultiplayerMode) => void;
  setError: (error: string | null) => void;
  setOnlineCounts: (players: number, ai: number) => void;
  setRemoteSpritesVisible: (visible: boolean) => void;
  setCrowdViewEnabled: (enabled: boolean) => void;
  reset: () => void;
}

const initialState = {
  connectionState: 'disconnected' as ConnectionState,
  isHost: false,
  currentMatchId: null,
  currentMatchChannelName: null,
  currentModpackId: null,
  currentMode: null,
  opponentId: null,
  opponentName: null,
  inQueue: false,
  queueMode: null,
  lastError: null,
  onlinePlayerCount: 0,
  onlineAiCount: 0,
  remoteSpritesVisible: true,
  crowdViewEnabled: false,
};

export const useMultiplayerStore = create<MultiplayerState>((set) => ({
  ...initialState,

  setConnectionState: (connectionState) => set({ connectionState }),

  setMatch: (
    matchId,
    opponentId,
    opponentName,
    mode,
    isHost,
    modpackId = 'core-modular',
    channelName = matchId,
  ) =>
    set({
      currentMatchId: matchId,
      currentMatchChannelName: channelName,
      currentModpackId: modpackId,
      opponentId,
      opponentName,
      currentMode: mode,
      isHost,
      inQueue: false,
      lastError: null,
    }),

  clearMatch: () =>
    set({
      currentMatchId: null,
      currentMatchChannelName: null,
      currentModpackId: null,
      opponentId: null,
      opponentName: null,
      currentMode: null,
      isHost: false,
    }),

  setInQueue: (inQueue, mode) =>
    set((state) => ({
      inQueue,
      queueMode: inQueue ? (mode ?? state.queueMode) : null,
      lastError: null,
    })),

  setError: (lastError) => set({ lastError }),
  setOnlineCounts: (onlinePlayerCount, onlineAiCount) =>
    set({
      onlinePlayerCount: Math.max(0, Math.trunc(onlinePlayerCount)),
      onlineAiCount: Math.max(0, Math.trunc(onlineAiCount)),
    }),
  setRemoteSpritesVisible: (remoteSpritesVisible) => set({ remoteSpritesVisible }),
  setCrowdViewEnabled: (crowdViewEnabled) => set({ crowdViewEnabled }),

  reset: () => set(initialState),
}));

/**
 * Selector hooks for specific pieces of state
 * (Optimizes React re-renders by only subscribing to needed state)
 */

export const useIsInMultiplayer = () =>
  useMultiplayerStore((state) => state.currentMatchId !== null);

export const useConnectionState = () =>
  useMultiplayerStore((state) => state.connectionState);

export const useIsInQueue = () =>
  useMultiplayerStore((state) => state.inQueue);

export const useCurrentOpponent = () =>
  useMultiplayerStore(
    useShallow((state) => ({
      id: state.opponentId,
      name: state.opponentName,
    }))
  );

export const useMultiplayerError = () =>
  useMultiplayerStore((state) => state.lastError);

export const useOnlinePlayerCount = () =>
  useMultiplayerStore((state) => state.onlinePlayerCount);

export const useOnlineAiCount = () =>
  useMultiplayerStore((state) => state.onlineAiCount);

export const useRemoteSpritesVisible = () =>
  useMultiplayerStore((state) => state.remoteSpritesVisible);

export const useCrowdViewEnabled = () =>
  useMultiplayerStore((state) => state.crowdViewEnabled);
