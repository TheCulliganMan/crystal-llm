-- ============================================================================
-- Add Kingler as a valid Play Panel brand theme
-- Created: 2026-02-20
-- Purpose: Keep brand_theme persistence aligned with frontend theme options
-- ============================================================================

ALTER TABLE public.play_user_settings
  DROP CONSTRAINT IF EXISTS play_user_settings_brand_theme_check;

ALTER TABLE public.play_user_settings
  ADD CONSTRAINT play_user_settings_brand_theme_check
  CHECK (
    brand_theme IN (
      'krabby',
      'kingler',
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
