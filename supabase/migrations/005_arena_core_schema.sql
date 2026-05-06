-- ============================================================================
-- Arena Core Schema
-- Created: 2026-02-18
-- Purpose: Ensure arena core tables/views/policies exist in all environments
-- ============================================================================

create extension if not exists "uuid-ossp";
create extension if not exists "pgcrypto";

create table if not exists public.arena_profiles (
  id uuid primary key references auth.users(id) on delete cascade,
  handle text not null unique,
  display_name text,
  avatar_url text,
  bio text,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create table if not exists public.arena_agents (
  id uuid primary key default gen_random_uuid(),
  owner_id uuid not null references auth.users(id) on delete cascade,
  name text not null,
  slug text not null,
  description text,
  repo_url text,
  mcp_endpoint text,
  runtime text not null default 'mcp-http',
  visibility text not null default 'private' check (visibility in ('private', 'public')),
  config jsonb not null default '{}'::jsonb,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

do $$
begin
  if not exists (
    select 1
    from pg_constraint
    where conname = 'arena_agents_owner_name_unique'
      and conrelid = 'public.arena_agents'::regclass
  ) then
    alter table public.arena_agents
      add constraint arena_agents_owner_name_unique unique (owner_id, name);
  end if;
end;
$$;

create table if not exists public.arena_runs (
  id uuid primary key default gen_random_uuid(),
  agent_id uuid not null references public.arena_agents(id) on delete cascade,
  created_by uuid not null references auth.users(id) on delete cascade,
  status text not null default 'queued' check (status in ('queued', 'running', 'completed', 'failed', 'cancelled')),
  queue text not null default 'main',
  seed text,
  mcp_session_url text,
  spectator_frame_url text,
  started_at timestamptz,
  finished_at timestamptz,
  frame_count integer,
  badge_count integer default 0,
  pokedex_seen integer,
  pokedex_caught integer,
  error text,
  metrics jsonb not null default '{}'::jsonb,
  notes text,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create index if not exists idx_arena_runs_agent_status on public.arena_runs (agent_id, status);
create index if not exists idx_arena_runs_queue_status on public.arena_runs (queue, status);

create table if not exists public.arena_run_events (
  id bigserial primary key,
  run_id uuid not null references public.arena_runs(id) on delete cascade,
  frame integer,
  label text,
  payload jsonb,
  created_at timestamptz not null default now()
);

create index if not exists idx_arena_run_events_run_frame on public.arena_run_events (run_id, frame);

create or replace view public.arena_leaderboard as
select
  agent_id,
  min(coalesce(finished_at, now()) - coalesce(started_at, finished_at)) as best_duration,
  max(badge_count) as max_badges,
  avg(frame_count) as avg_frames,
  count(*) as total_runs
from public.arena_runs
where status = 'completed'
group by agent_id;

alter table public.arena_profiles enable row level security;
alter table public.arena_agents enable row level security;
alter table public.arena_runs enable row level security;
alter table public.arena_run_events enable row level security;

drop policy if exists "Arena profiles are self-service" on public.arena_profiles;
create policy "Arena profiles are self-service" on public.arena_profiles
  for select using (true);
drop policy if exists "Users manage their profile" on public.arena_profiles;
create policy "Users manage their profile" on public.arena_profiles
  for insert with check (auth.uid() = id);
drop policy if exists "Users update their profile" on public.arena_profiles;
create policy "Users update their profile" on public.arena_profiles
  for update using (auth.uid() = id);

drop policy if exists "Public agents are visible" on public.arena_agents;
create policy "Public agents are visible" on public.arena_agents
  for select using (visibility = 'public' or auth.uid() = owner_id);
drop policy if exists "Owners insert agents" on public.arena_agents;
create policy "Owners insert agents" on public.arena_agents
  for insert with check (auth.uid() = owner_id);
drop policy if exists "Owners update agents" on public.arena_agents;
create policy "Owners update agents" on public.arena_agents
  for update using (auth.uid() = owner_id);

drop policy if exists "Readable runs" on public.arena_runs;
create policy "Readable runs" on public.arena_runs
  for select using (
    exists (
      select 1 from public.arena_agents a
      where a.id = arena_runs.agent_id
        and (a.visibility = 'public' or a.owner_id = auth.uid())
    )
  );
drop policy if exists "Owners insert runs" on public.arena_runs;
create policy "Owners insert runs" on public.arena_runs
  for insert with check (
    auth.uid() = created_by
    and exists (select 1 from public.arena_agents a where a.id = agent_id and a.owner_id = auth.uid())
  );
drop policy if exists "Owners update runs" on public.arena_runs;
create policy "Owners update runs" on public.arena_runs
  for update using (
    auth.uid() = created_by
    or auth.role() = 'service_role'
  );

drop policy if exists "Readable run events" on public.arena_run_events;
create policy "Readable run events" on public.arena_run_events
  for select using (
    exists (
      select 1 from public.arena_runs r
      join public.arena_agents a on a.id = r.agent_id
      where r.id = arena_run_events.run_id
        and (a.visibility = 'public' or a.owner_id = auth.uid())
    )
  );
drop policy if exists "Owners insert run events" on public.arena_run_events;
create policy "Owners insert run events" on public.arena_run_events
  for insert with check (
    exists (
      select 1 from public.arena_runs r
      join public.arena_agents a on a.id = r.agent_id
      where r.id = run_id
        and (a.owner_id = auth.uid() or auth.role() = 'service_role')
    )
  );
