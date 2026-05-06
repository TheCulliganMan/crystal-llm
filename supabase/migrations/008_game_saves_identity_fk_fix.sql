-- ============================================================================
-- Game Saves Identity FK Fix
-- Created: 2026-02-19
-- Purpose: Allow MCP identity UUIDs to persist in game_saves.user_id
-- ============================================================================

do $$
begin
  if exists (
    select 1
    from pg_constraint
    where conname = 'game_saves_user_id_fkey'
      and conrelid = 'public.game_saves'::regclass
  ) then
    alter table public.game_saves drop constraint game_saves_user_id_fkey;
  end if;
end;
$$;
