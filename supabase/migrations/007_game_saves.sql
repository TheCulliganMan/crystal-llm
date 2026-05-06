-- ============================================================================
-- Game Saves
-- Created: 2026-02-19
-- Purpose: Persist per-user save snapshots for Play + MCP identity sessions
-- ============================================================================

create extension if not exists "pgcrypto";

create table if not exists public.game_saves (
  id uuid primary key default gen_random_uuid(),
  user_id uuid not null references auth.users(id) on delete cascade,
  slot text not null,
  payload jsonb not null,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create unique index if not exists game_saves_user_slot_key
  on public.game_saves (user_id, slot);

create index if not exists idx_game_saves_user_updated
  on public.game_saves (user_id, updated_at desc);

alter table public.game_saves enable row level security;

drop policy if exists "Users can view own game saves" on public.game_saves;
create policy "Users can view own game saves"
  on public.game_saves for select
  using (auth.uid() = user_id);

drop policy if exists "Users can insert own game saves" on public.game_saves;
create policy "Users can insert own game saves"
  on public.game_saves for insert
  with check (auth.uid() = user_id);

drop policy if exists "Users can update own game saves" on public.game_saves;
create policy "Users can update own game saves"
  on public.game_saves for update
  using (auth.uid() = user_id);

drop policy if exists "Users can delete own game saves" on public.game_saves;
create policy "Users can delete own game saves"
  on public.game_saves for delete
  using (auth.uid() = user_id);

create or replace function update_game_saves_updated_at()
returns trigger as $$
begin
  new.updated_at = now();
  return new;
end;
$$ language plpgsql;

drop trigger if exists game_saves_updated_at on public.game_saves;
create trigger game_saves_updated_at
  before update on public.game_saves
  for each row execute function update_game_saves_updated_at();

comment on table public.game_saves is 'Per-user game saves used by Play panel and MCP identity sessions';
