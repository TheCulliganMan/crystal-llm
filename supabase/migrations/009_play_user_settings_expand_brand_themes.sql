-- ============================================================================
-- Expand Play Panel brand theme options
-- Created: 2026-02-19
-- Purpose: Allow additional claw-themed mascot themes in play_user_settings
-- ============================================================================

ALTER TABLE public.play_user_settings
  DROP CONSTRAINT IF EXISTS play_user_settings_brand_theme_check;

ALTER TABLE public.play_user_settings
  ADD CONSTRAINT play_user_settings_brand_theme_check
  CHECK (
    brand_theme IN (
      'krabby',
      'heracross',
      'gligar',
      'scizor',
      'sneasel',
      'teddiursa',
      'ursaring',
      'totodile',
      'croconaw',
      'feraligatr',
      'pinsir'
    )
  );
