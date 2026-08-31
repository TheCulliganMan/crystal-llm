import { useMultiplayerStore } from './multiplayer-store';

describe('multiplayer-store', () => {
  beforeEach(() => {
    useMultiplayerStore.getState().reset();
  });

  test('initial state', () => {
    const state = useMultiplayerStore.getState();
    expect(state.connectionState).toBe('disconnected');
    expect(state.currentMatchId).toBeNull();
    expect(state.inQueue).toBe(false);
    expect(state.lastError).toBeNull();
  });

  test('setMatch preserves world connection and clears queue', () => {
    useMultiplayerStore.getState().setConnectionState('connected');
    useMultiplayerStore.getState().setInQueue(true, 'battle');
    useMultiplayerStore
      .getState()
      .setMatch('match1', 'opp1', 'Opponent', 'battle', true);

    const state = useMultiplayerStore.getState();
    expect(state.currentMatchId).toBe('match1');
    expect(state.opponentId).toBe('opp1');
    expect(state.opponentName).toBe('Opponent');
    expect(state.currentMode).toBe('battle');
    expect(state.isHost).toBe(true);
    expect(state.connectionState).toBe('connected');
    expect(state.inQueue).toBe(false);
  });

  test('clearMatch resets match state', () => {
    useMultiplayerStore
      .getState()
      .setMatch('match1', 'opp1', 'Opponent', 'battle', true);
    useMultiplayerStore.getState().setConnectionState('connected');
    useMultiplayerStore.getState().clearMatch();

    const state = useMultiplayerStore.getState();
    expect(state.currentMatchId).toBeNull();
    expect(state.opponentId).toBeNull();
    expect(state.connectionState).toBe('connected');
  });
});
