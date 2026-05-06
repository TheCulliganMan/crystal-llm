-- ============================================================================
-- Multiplayer Schema Migration
-- Created: 2026-02-08
-- Purpose: Add tables for matchmaking, active matches, and friendships
-- ============================================================================

-- Enable UUID generation extension if not already enabled
CREATE EXTENSION IF NOT EXISTS pgcrypto;

-- ============================================================================
-- Enums
-- ============================================================================

CREATE TYPE matchmaking_mode AS ENUM ('battle', 'trade', 'time_capsule');
CREATE TYPE match_status AS ENUM ('waiting', 'active', 'completed', 'cancelled');
CREATE TYPE friendship_status AS ENUM ('pending', 'accepted', 'blocked');

-- ============================================================================
-- Matchmaking Queue
-- ============================================================================

CREATE TABLE public.matchmaking_queue (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id UUID REFERENCES auth.users(id) ON DELETE CASCADE NOT NULL,
  mode matchmaking_mode NOT NULL,
  rating INTEGER DEFAULT 1000,
  party_preview JSONB,  -- Pokemon levels/species for matchmaking preview
  preferences JSONB DEFAULT '{}',  -- {ratingRange: 100, ruleset: 'standard', level: 50}
  created_at TIMESTAMPTZ DEFAULT NOW() NOT NULL,
  expires_at TIMESTAMPTZ DEFAULT (NOW() + INTERVAL '5 minutes') NOT NULL
);

-- Indexes for efficient matchmaking queries
CREATE INDEX idx_matchmaking_mode_rating
  ON public.matchmaking_queue(mode, rating);

CREATE INDEX idx_matchmaking_expires
  ON public.matchmaking_queue(expires_at);

-- Row Level Security
ALTER TABLE public.matchmaking_queue ENABLE ROW LEVEL SECURITY;

CREATE POLICY "Users can view own queue entries"
  ON public.matchmaking_queue FOR SELECT
  USING (auth.uid() = user_id);

CREATE POLICY "Users can join queue"
  ON public.matchmaking_queue FOR INSERT
  WITH CHECK (auth.uid() = user_id);

CREATE POLICY "Users can leave queue"
  ON public.matchmaking_queue FOR DELETE
  USING (auth.uid() = user_id);

-- ============================================================================
-- Active Matches
-- ============================================================================

CREATE TABLE public.matches (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  player1_id UUID REFERENCES auth.users(id) ON DELETE CASCADE NOT NULL,
  player2_id UUID REFERENCES auth.users(id) ON DELETE CASCADE NOT NULL,
  mode matchmaking_mode NOT NULL,
  status match_status DEFAULT 'waiting' NOT NULL,
  channel_name TEXT UNIQUE NOT NULL,  -- Supabase Realtime channel name
  result JSONB,  -- {winner: UUID, player1Score: number, player2Score: number, ...}
  created_at TIMESTAMPTZ DEFAULT NOW() NOT NULL,
  started_at TIMESTAMPTZ,
  completed_at TIMESTAMPTZ,

  -- Ensure players are different
  CONSTRAINT different_players CHECK (player1_id != player2_id)
);

-- Index for querying user matches
CREATE INDEX idx_matches_player1 ON public.matches(player1_id);
CREATE INDEX idx_matches_player2 ON public.matches(player2_id);
CREATE INDEX idx_matches_status ON public.matches(status);

-- Row Level Security
ALTER TABLE public.matches ENABLE ROW LEVEL SECURITY;

CREATE POLICY "Players can view own matches"
  ON public.matches FOR SELECT
  USING (auth.uid() IN (player1_id, player2_id));

CREATE POLICY "Players can update own matches"
  ON public.matches FOR UPDATE
  USING (auth.uid() IN (player1_id, player2_id));

-- ============================================================================
-- Friendships
-- ============================================================================

CREATE TABLE public.friendships (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id UUID REFERENCES auth.users(id) ON DELETE CASCADE NOT NULL,
  friend_id UUID REFERENCES auth.users(id) ON DELETE CASCADE NOT NULL,
  status friendship_status DEFAULT 'pending' NOT NULL,
  created_at TIMESTAMPTZ DEFAULT NOW() NOT NULL,
  updated_at TIMESTAMPTZ DEFAULT NOW() NOT NULL,

  -- Ensure unique friendship pairs
  UNIQUE(user_id, friend_id),

  -- Ensure user cannot friend themselves
  CONSTRAINT no_self_friend CHECK (user_id != friend_id)
);

-- Index for querying user friendships
CREATE INDEX idx_friendships_user ON public.friendships(user_id);
CREATE INDEX idx_friendships_friend ON public.friendships(friend_id);
CREATE INDEX idx_friendships_status ON public.friendships(status);

-- Row Level Security
ALTER TABLE public.friendships ENABLE ROW LEVEL SECURITY;

CREATE POLICY "Users can view own friendships"
  ON public.friendships FOR SELECT
  USING (auth.uid() IN (user_id, friend_id));

CREATE POLICY "Users can send friend requests"
  ON public.friendships FOR INSERT
  WITH CHECK (auth.uid() = user_id);

CREATE POLICY "Users can update friendships involving them"
  ON public.friendships FOR UPDATE
  USING (auth.uid() IN (user_id, friend_id));

CREATE POLICY "Users can delete own friendships"
  ON public.friendships FOR DELETE
  USING (auth.uid() IN (user_id, friend_id));

-- ============================================================================
-- Extend arena_profiles with Link Battle Stats
-- ============================================================================

-- Add link battle statistics only when arena_profiles exists in the target DB.
DO $$
BEGIN
  IF to_regclass('public.arena_profiles') IS NOT NULL THEN
    ALTER TABLE public.arena_profiles
      ADD COLUMN IF NOT EXISTS link_battle_wins INTEGER DEFAULT 0,
      ADD COLUMN IF NOT EXISTS link_battle_losses INTEGER DEFAULT 0,
      ADD COLUMN IF NOT EXISTS link_battle_rating INTEGER DEFAULT 1000,
      ADD COLUMN IF NOT EXISTS total_trades INTEGER DEFAULT 0;
  ELSE
    RAISE NOTICE 'Skipping arena_profiles multiplayer stat columns because public.arena_profiles does not exist.';
  END IF;
END;
$$;

-- ============================================================================
-- Functions
-- ============================================================================

-- Function to clean expired queue entries (called by cron or manually)
CREATE OR REPLACE FUNCTION clean_expired_queue_entries()
RETURNS INTEGER AS $$
DECLARE
  deleted_count INTEGER;
BEGIN
  DELETE FROM public.matchmaking_queue WHERE expires_at < NOW();
  GET DIAGNOSTICS deleted_count = ROW_COUNT;
  RETURN deleted_count;
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;

-- Function to update friendship updated_at timestamp
CREATE OR REPLACE FUNCTION update_friendship_updated_at()
RETURNS TRIGGER AS $$
BEGIN
  NEW.updated_at = NOW();
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER friendships_updated_at
  BEFORE UPDATE ON public.friendships
  FOR EACH ROW EXECUTE FUNCTION update_friendship_updated_at();

-- ============================================================================
-- Views for Leaderboards
-- ============================================================================

-- Multiplayer Battle Leaderboard
DO $$
BEGIN
  IF to_regclass('public.arena_profiles') IS NOT NULL THEN
    EXECUTE $view$
      CREATE OR REPLACE VIEW public.multiplayer_leaderboard AS
      SELECT
        ap.id,
        ap.handle,
        ap.display_name,
        ap.avatar_url,
        ap.link_battle_wins,
        ap.link_battle_losses,
        ap.link_battle_rating,
        (ap.link_battle_wins::FLOAT / NULLIF(ap.link_battle_wins + ap.link_battle_losses, 0)) * 100 AS win_rate,
        ap.link_battle_wins + ap.link_battle_losses AS total_battles,
        RANK() OVER (ORDER BY ap.link_battle_rating DESC) AS rank
      FROM public.arena_profiles ap
      WHERE ap.link_battle_wins + ap.link_battle_losses > 0
      ORDER BY ap.link_battle_rating DESC
    $view$;

    EXECUTE 'COMMENT ON VIEW public.multiplayer_leaderboard IS ''Leaderboard showing player rankings by link battle rating''';
  ELSE
    RAISE NOTICE 'Skipping multiplayer_leaderboard view because public.arena_profiles does not exist.';
  END IF;
END;
$$;

-- ============================================================================
-- Comments
-- ============================================================================

COMMENT ON TABLE public.matchmaking_queue IS 'Queue for players waiting to be matched for multiplayer battles/trades';
COMMENT ON TABLE public.matches IS 'Active and historical multiplayer match records';
COMMENT ON TABLE public.friendships IS 'Friend relationships between users for direct challenges';
COMMENT ON FUNCTION clean_expired_queue_entries IS 'Removes expired matchmaking queue entries';

-- ============================================================================
-- Grants (ensure service role can manage matchmaking)
-- ============================================================================

-- Service role needs full access to matchmaking queue for the matchmaker function
GRANT ALL ON public.matchmaking_queue TO service_role;
GRANT ALL ON public.matches TO service_role;
GRANT ALL ON public.friendships TO service_role;
