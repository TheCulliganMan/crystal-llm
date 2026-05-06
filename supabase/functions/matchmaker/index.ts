/**
 * Matchmaker Edge Function
 *
 * Supabase Edge Function (Deno) that runs periodically to pair waiting
 * players from the matchmaking queue.
 *
 * Triggered by:
 * - Cron job (every 5 seconds via pg_cron)
 * - Manual invocation
 *
 * Algorithm:
 * 1. Fetch all non-expired queue entries
 * 2. Group by mode (battle/trade/time_capsule)
 * 3. Sort by rating for fair matchmaking
 * 4. Pair players with compatible ratings (±100)
 * 5. Create match records
 * 6. Delete matched players from queue
 */

import { serve } from 'https://deno.land/std@0.168.0/http/server.ts';
import { createClient } from 'https://esm.sh/@supabase/supabase-js@2';

// Types
type MatchmakingMode = 'battle' | 'trade' | 'time_capsule';

interface QueueEntry {
  id: string;
  user_id: string;
  mode: MatchmakingMode;
  rating: number;
  party_preview: any;
  preferences: {
    ratingRange?: number;
    ruleset?: string;
    level?: number | 'auto';
  };
  created_at: string;
}

interface Match {
  player1_id: string;
  player2_id: string;
  mode: MatchmakingMode;
  channel_name: string;
}

const corsHeaders = {
  'Access-Control-Allow-Origin': '*',
  'Access-Control-Allow-Headers': 'authorization, x-client-info, apikey, content-type',
  'Access-Control-Allow-Methods': 'POST, OPTIONS',
};

const jsonResponse = (body: Record<string, unknown>, status = 200) =>
  new Response(JSON.stringify(body), {
    status,
    headers: { ...corsHeaders, 'Content-Type': 'application/json' },
  });

serve(async (req) => {
  if (req.method === 'OPTIONS') {
    return new Response('ok', { headers: corsHeaders });
  }

  try {
    // Initialize Supabase client with service role key (admin access)
    const supabase = createClient(
      Deno.env.get('SUPABASE_URL')!,
      Deno.env.get('SUPABASE_SERVICE_ROLE_KEY')!
    );

    console.log('[Matchmaker] Running matchmaking cycle...');

    const { data: cleaned, error: cleanError } = await supabase.rpc(
      'clean_expired_queue_entries'
    );
    if (cleanError) {
      console.warn('[Matchmaker] Failed to clean expired queue entries:', cleanError);
    } else {
      console.log(`[Matchmaker] Cleaned ${cleaned ?? 0} expired queue entries`);
    }

    // Fetch all waiting players (not expired)
    const { data: queue, error: queueError } = await supabase
      .from('matchmaking_queue')
      .select('*')
      .gt('expires_at', new Date().toISOString())
      .order('rating');

    if (queueError) {
      console.error('[Matchmaker] Failed to fetch queue:', queueError);
      return jsonResponse({ ok: false, error: queueError.message }, 500);
    }

    if (!queue || queue.length < 2) {
      console.log(`[Matchmaker] Not enough players (${queue?.length ?? 0})`);
      return jsonResponse({ ok: true, created: 0, message: 'Not enough players' });
    }

    console.log(`[Matchmaker] Processing ${queue.length} players in queue`);

    // Group players by mode
    const byMode: Record<string, QueueEntry[]> = queue.reduce(
      (acc, entry) => {
        if (!acc[entry.mode]) acc[entry.mode] = [];
        acc[entry.mode].push(entry as QueueEntry);
        return acc;
      },
      {} as Record<string, QueueEntry[]>
    );

    const createdMatches: Match[] = [];
    const matchedPlayerIds: Set<string> = new Set();

    // Process each mode
    for (const [mode, players] of Object.entries(byMode)) {
      console.log(`[Matchmaker] Processing ${players.length} players in ${mode} mode`);

      // Sort by rating for fair matchmaking
      players.sort((a, b) => a.rating - b.rating);

      // Find compatible pairs
      for (let i = 0; i < players.length - 1; i++) {
        const p1 = players[i];

        // Skip if already matched
        if (matchedPlayerIds.has(p1.user_id)) continue;

        // Find a compatible opponent
        for (let j = i + 1; j < players.length; j++) {
          const p2 = players[j];

          // Skip if already matched
          if (matchedPlayerIds.has(p2.user_id)) continue;

          // Check compatibility
          if (areCompatible(p1, p2)) {
            // Create match
            const channelName = `match_${crypto.randomUUID()}`;

            const match: Match = {
              player1_id: p1.user_id,
              player2_id: p2.user_id,
              mode: mode as MatchmakingMode,
              channel_name: channelName,
            };

            // Insert match into database
            const { data: createdMatch, error: matchError } = await supabase
              .from('matches')
              .insert(match)
              .select()
              .single();

            if (matchError) {
              console.error('[Matchmaker] Failed to create match:', matchError);
              continue;
            }

            console.log(
              `[Matchmaker] Created match: ${p1.user_id} vs ${p2.user_id}`
            );

            createdMatches.push(match);
            matchedPlayerIds.add(p1.user_id);
            matchedPlayerIds.add(p2.user_id);

            break; // Move to next p1
          }
        }
      }
    }

    // Remove matched players from queue
    if (matchedPlayerIds.size > 0) {
      const { error: deleteError } = await supabase
        .from('matchmaking_queue')
        .delete()
        .in('user_id', Array.from(matchedPlayerIds));

      if (deleteError) {
        console.error('[Matchmaker] Failed to clean queue:', deleteError);
      } else {
        console.log(`[Matchmaker] Removed ${matchedPlayerIds.size} players from queue`);
      }
    }

    console.log(`[Matchmaker] Created ${createdMatches.length} matches`);

    return jsonResponse({
      ok: true,
      created: createdMatches.length,
      matchedPlayers: matchedPlayerIds.size,
    });
  } catch (error) {
    console.error('[Matchmaker] Unexpected error:', error);
    return jsonResponse(
      { ok: false, error: error instanceof Error ? error.message : 'Unexpected error' },
      500
    );
  }
});

/**
 * Check if two players are compatible for matchmaking
 */
function areCompatible(p1: QueueEntry, p2: QueueEntry): boolean {
  // Must be same mode
  if (p1.mode !== p2.mode) return false;

  // Check rating range
  const ratingDiff = Math.abs(p1.rating - p2.rating);
  const maxRatingRange = Math.max(
    p1.preferences?.ratingRange ?? 100,
    p2.preferences?.ratingRange ?? 100
  );

  if (ratingDiff > maxRatingRange) return false;

  // Check ruleset compatibility (if specified)
  const p1Ruleset = p1.preferences?.ruleset;
  const p2Ruleset = p2.preferences?.ruleset;

  if (p1Ruleset && p2Ruleset && p1Ruleset !== p2Ruleset) {
    return false;
  }

  // Check level compatibility (if specified)
  const p1Level = p1.preferences?.level;
  const p2Level = p2.preferences?.level;

  if (p1Level && p2Level && p1Level !== p2Level && p1Level !== 'auto' && p2Level !== 'auto') {
    return false;
  }

  return true;
}
