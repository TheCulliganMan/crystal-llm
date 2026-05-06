/**
 * Matchmaking Service
 *
 * Client-side service for joining matchmaking queues and receiving
 * match notifications via Supabase Realtime.
 *
 * Flow:
 * 1. User joins queue (inserts row in matchmaking_queue table)
 * 2. Matchmaker Edge Function pairs players and creates match
 * 3. Client receives match notification via Realtime subscription
 * 4. Client updates MultiplayerStore with match details
 */

import { createMultiplayerClient } from '@pokecrystal/core/adapters/multiplayer-client';
import { useMultiplayerStore } from './multiplayer-store';
import type { MultiplayerRealtimeChannel } from '@pokecrystal/core/adapters/multiplayer-client';

export type MatchmakingMode = 'battle' | 'trade' | 'time_capsule';

export interface MatchmakingRequest {
  mode: MatchmakingMode;
  rating?: number;
  partyPreview?: {
    species: string;
    level: number;
  }[];
  preferences?: {
    ratingRange?: number; // ±50, ±100, etc.
    ruleset?: string; // 'standard', 'ubers', 'little-cup'
    level?: 50 | 100 | 'auto';
  };
}

export interface Match {
  id: string;
  player1_id: string;
  player2_id: string;
  mode: MatchmakingMode;
  channel_name: string;
  created_at: string;
}

export class MatchmakingService {
  private supabase = createMultiplayerClient();
  private channel: MultiplayerRealtimeChannel | null = null;
  private userId: string | null = null;

  /**
   * Join the matchmaking queue
   * @param request - Matchmaking parameters (mode, rating, preferences)
   */
  async joinQueue(request: MatchmakingRequest): Promise<void> {
    if (!this.supabase) {
      throw new Error('Supabase not initialized');
    }

    // Get current user
    const {
      data: { user },
    } = await this.supabase.auth.getUser();
    if (!user) {
      throw new Error('User not authenticated');
    }
    this.userId = user.id;

    // Insert into matchmaking queue
    const { error } = await this.supabase.from('matchmaking_queue').upsert({
      user_id: user.id,
      mode: request.mode,
      rating: request.rating ?? 1000,
      party_preview: request.partyPreview ?? null,
      preferences: request.preferences ?? {},
      expires_at: new Date(Date.now() + 5 * 60_000).toISOString(),
    }, {
      onConflict: 'user_id,mode',
    });

    if (error) {
      console.error('[Matchmaking] Failed to join queue:', error);
      throw new Error(`Failed to join queue: ${error.message}`);
    }

    // Update store
    useMultiplayerStore.getState().setInQueue(true, request.mode);

    // Subscribe to match notifications
    this.subscribeToMatches(user.id);

    console.log(`[Matchmaking] Joined ${request.mode} queue`);
  }

  /**
   * Leave the matchmaking queue
   */
  async leaveQueue(): Promise<void> {
    if (!this.supabase) {
      throw new Error('Supabase not initialized');
    }

    const {
      data: { user },
    } = await this.supabase.auth.getUser();
    if (!user) {
      throw new Error('User not authenticated');
    }

    // Delete from matchmaking queue
    const { error } = await this.supabase
      .from('matchmaking_queue')
      .delete()
      .eq('user_id', user.id);

    if (error) {
      console.error('[Matchmaking] Failed to leave queue:', error);
      throw new Error(`Failed to leave queue: ${error.message}`);
    }

    // Update store
    useMultiplayerStore.getState().setInQueue(false);

    // Unsubscribe from notifications
    this.unsubscribe();

    console.log('[Matchmaking] Left queue');
  }

  /**
   * Subscribe to match creation notifications
   * Listens for INSERT events on matches table where user is a player
   *
   * @param userId - Current user's ID
   */
  private subscribeToMatches(userId: string): void {
    if (!this.supabase) return;

    this.channel = this.supabase
      .channel('matchmaking-notifications')
      .on(
        'postgres_changes',
        {
          event: 'INSERT',
          schema: 'public',
          table: 'matches',
          filter: `player1_id=eq.${userId}`,
        },
        (payload) => this.handleMatchFound(payload.new as Match, true)
      )
      .on(
        'postgres_changes',
        {
          event: 'INSERT',
          schema: 'public',
          table: 'matches',
          filter: `player2_id=eq.${userId}`,
        },
        (payload) => this.handleMatchFound(payload.new as Match, false)
      )
      .subscribe();

    console.log('[Matchmaking] Subscribed to match notifications');
  }

  /**
   * Handle match found event
   * @param match - Match record from database
   * @param isHost - Whether this user is player1 (host)
   */
  private async handleMatchFound(match: Match, isHost: boolean): Promise<void> {
    console.log('[Matchmaking] Match found!', match);

    if (!this.supabase) return;

    // Determine opponent ID
    const opponentId = isHost ? match.player2_id : match.player1_id;

    // Fetch opponent profile
    const { data: opponentProfile, error } = await this.supabase
      .from('arena_profiles')
      .select('handle, display_name')
      .eq('id', opponentId)
      .single();

    if (error) {
      console.error('[Matchmaking] Failed to fetch opponent profile:', error);
    }

    const opponentName =
      opponentProfile?.display_name ||
      opponentProfile?.handle ||
      'Unknown Player';

    // Update multiplayer store with match details
    useMultiplayerStore.getState().setMatch(
      match.id,
      opponentId,
      opponentName,
      match.mode,
      isHost
    );

    console.log(
      `[Matchmaking] Matched with ${opponentName} (${isHost ? 'host' : 'client'})`
    );

    // Unsubscribe from future match notifications
    this.unsubscribe();
  }

  /**
   * Unsubscribe from match notifications
   */
  private unsubscribe(): void {
    if (this.channel && this.supabase) {
      this.supabase.removeChannel(this.channel);
      this.channel = null;
      console.log('[Matchmaking] Unsubscribed from notifications');
    }
  }

  /**
   * Get current queue position (estimated)
   * @param mode - Matchmaking mode
   * @returns Number of players ahead in queue
   */
  async getQueuePosition(mode: MatchmakingMode): Promise<number> {
    if (!this.supabase) {
      throw new Error('Supabase not initialized');
    }

    const {
      data: { user },
    } = await this.supabase.auth.getUser();
    if (!user) {
      throw new Error('User not authenticated');
    }

    // Get user's queue entry
    const { data: userEntry, error: userError } = await this.supabase
      .from('matchmaking_queue')
      .select('created_at, rating')
      .eq('user_id', user.id)
      .eq('mode', mode)
      .single();

    if (userError || !userEntry) {
      return 0;
    }

    // Count players ahead (joined earlier with similar rating)
    const { count, error } = await this.supabase
      .from('matchmaking_queue')
      .select('*', { count: 'exact', head: true })
      .eq('mode', mode)
      .lt('created_at', userEntry.created_at)
      .gte('rating', (userEntry.rating ?? 1000) - 100)
      .lte('rating', (userEntry.rating ?? 1000) + 100);

    if (error) {
      console.error('[Matchmaking] Failed to get queue position:', error);
      return 0;
    }

    return count ?? 0;
  }

  /**
   * Get estimated wait time
   * @param mode - Matchmaking mode
   * @returns Estimated wait time in seconds
   */
  async getEstimatedWaitTime(mode: MatchmakingMode): Promise<number> {
    // Simple estimation: assume 1 match per 10 seconds
    const position = await this.getQueuePosition(mode);
    return Math.max(position * 5, 10); // Minimum 10 seconds
  }

  /**
   * Clean up resources
   */
  destroy(): void {
    this.unsubscribe();
    this.userId = null;
  }
}
