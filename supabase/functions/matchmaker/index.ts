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
 * Pairing runs inside Postgres with row locks. This function is only a small,
 * horizontally-safe trigger for that atomic operation.
 */

import { serve } from 'https://deno.land/std@0.168.0/http/server.ts';
import { createClient } from 'https://esm.sh/@supabase/supabase-js@2';

// Types
interface Match {
  id: string;
  player1_id: string;
  player2_id: string;
  mode: 'battle' | 'trade' | 'time_capsule';
  channel_name: string;
  modpack_id: string;
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

    const { data, error } = await supabase.rpc('matchmake_queue', { max_pairs: 500 });
    if (error) {
      console.error('[Matchmaker] Atomic matchmaking failed:', error);
      return jsonResponse({ ok: false, error: error.message }, 500);
    }
    const createdMatches = (data ?? []) as Match[];

    console.log(`[Matchmaker] Created ${createdMatches.length} matches`);

    return jsonResponse({
      ok: true,
      created: createdMatches.length,
      matchedPlayers: createdMatches.length * 2,
    });
  } catch (error) {
    console.error('[Matchmaker] Unexpected error:', error);
    return jsonResponse(
      { ok: false, error: error instanceof Error ? error.message : 'Unexpected error' },
      500
    );
  }
});
