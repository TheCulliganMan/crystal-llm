-- ============================================================================
-- Play Panel User Settings
-- Created: 2026-02-14
-- Purpose: Persist Play panel preferences per authenticated Supabase user
-- ============================================================================

CREATE TABLE IF NOT EXISTS public.play_user_settings (
  user_id UUID PRIMARY KEY REFERENCES auth.users(id) ON DELETE CASCADE,
  player_name TEXT NOT NULL DEFAULT 'Ryan',
  player_gender SMALLINT NOT NULL DEFAULT 0 CHECK (player_gender IN (0, 1)),
  time_of_day TEXT NOT NULL DEFAULT 'DAY' CHECK (time_of_day IN ('MORN', 'DAY', 'NIGHT')),
  sound_enabled BOOLEAN NOT NULL DEFAULT false,
  instant_mode_enabled BOOLEAN NOT NULL DEFAULT false,
  brand_theme TEXT NOT NULL DEFAULT 'krabby' CHECK (brand_theme IN ('krabby', 'heracross')),
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

ALTER TABLE public.play_user_settings ENABLE ROW LEVEL SECURITY;

CREATE POLICY "Users can view own play settings"
  ON public.play_user_settings FOR SELECT
  USING (auth.uid() = user_id);

CREATE POLICY "Users can insert own play settings"
  ON public.play_user_settings FOR INSERT
  WITH CHECK (auth.uid() = user_id);

CREATE POLICY "Users can update own play settings"
  ON public.play_user_settings FOR UPDATE
  USING (auth.uid() = user_id);

CREATE OR REPLACE FUNCTION update_play_user_settings_updated_at()
RETURNS TRIGGER AS $$
BEGIN
  NEW.updated_at = NOW();
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS play_user_settings_updated_at ON public.play_user_settings;
CREATE TRIGGER play_user_settings_updated_at
  BEFORE UPDATE ON public.play_user_settings
  FOR EACH ROW EXECUTE FUNCTION update_play_user_settings_updated_at();

COMMENT ON TABLE public.play_user_settings IS 'Per-user Play panel settings persisted via Supabase auth identity';
