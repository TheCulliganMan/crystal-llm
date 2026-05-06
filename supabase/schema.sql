-- Arena schema for Supabase.
-- Apply with `supabase db push` or paste into the SQL editor.

-- Extensions (Supabase enables these by default, keep for local dev parity)
create extension if not exists "uuid-ossp";
create extension if not exists "pgcrypto";

-- Profiles keyed to auth.users
create table if not exists public.arena_profiles (
  id uuid primary key references auth.users(id) on delete cascade,
  handle text not null unique,
  display_name text,
  avatar_url text,
  bio text,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

-- Agents owned by a user. Each agent corresponds to a specific MCP entrypoint/runtime.
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

alter table public.arena_agents
  add constraint arena_agents_owner_name_unique unique (owner_id, name);

-- Runs capture a single playthrough attempt for an agent.
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

-- Optional event log for granular playback / auditing.
create table if not exists public.arena_run_events (
  id bigserial primary key,
  run_id uuid not null references public.arena_runs(id) on delete cascade,
  frame integer,
  label text,
  payload jsonb,
  created_at timestamptz not null default now()
);

create index if not exists idx_arena_run_events_run_frame on public.arena_run_events (run_id, frame);

-- Leaderboard materialized from completed runs.
create view if not exists public.arena_leaderboard as
select
  agent_id,
  min(coalesce(finished_at, now()) - coalesce(started_at, finished_at)) as best_duration,
  max(badge_count) as max_badges,
  avg(frame_count) as avg_frames,
  count(*) as total_runs
from public.arena_runs
where status = 'completed'
group by agent_id;

-- RLS
alter table public.arena_profiles enable row level security;
alter table public.arena_agents enable row level security;
alter table public.arena_runs enable row level security;
alter table public.arena_run_events enable row level security;

-- Profiles: owner can read/update; everyone can read public handles for leaderboards.
create policy if not exists "Arena profiles are self-service" on public.arena_profiles
  for select using (true);
create policy if not exists "Users manage their profile" on public.arena_profiles
  for insert with check (auth.uid() = id);
create policy if not exists "Users update their profile" on public.arena_profiles
  for update using (auth.uid() = id);

-- Agents: owners can manage; public agents are readable.
create policy if not exists "Public agents are visible" on public.arena_agents
  for select using (visibility = 'public' or auth.uid() = owner_id);
create policy if not exists "Owners insert agents" on public.arena_agents
  for insert with check (auth.uid() = owner_id);
create policy if not exists "Owners update agents" on public.arena_agents
  for update using (auth.uid() = owner_id);

-- Runs: readable when agent is public or owner; owners can insert/update.
create policy if not exists "Readable runs" on public.arena_runs
  for select using (
    exists (
      select 1 from public.arena_agents a
      where a.id = arena_runs.agent_id
        and (a.visibility = 'public' or a.owner_id = auth.uid())
    )
  );
create policy if not exists "Owners insert runs" on public.arena_runs
  for insert with check (
    auth.uid() = created_by
    and exists (select 1 from public.arena_agents a where a.id = agent_id and a.owner_id = auth.uid())
  );
create policy if not exists "Owners update runs" on public.arena_runs
  for update using (
    auth.uid() = created_by
    or auth.role() = 'service_role'
  );

-- Run events: same visibility as runs.
create policy if not exists "Readable run events" on public.arena_run_events
  for select using (
    exists (
      select 1 from public.arena_runs r
      join public.arena_agents a on a.id = r.agent_id
      where r.id = arena_run_events.run_id
        and (a.visibility = 'public' or a.owner_id = auth.uid())
    )
  );
create policy if not exists "Owners insert run events" on public.arena_run_events
  for insert with check (
    exists (
      select 1 from public.arena_runs r
      join public.arena_agents a on a.id = r.agent_id
      where r.id = run_id
        and (a.owner_id = auth.uid() or auth.role() = 'service_role')
    )
  );

-- KrabbyClawArena ratings + head-to-head battles.
create table if not exists public.krabbyclaw_arena_ratings (
  agent_id uuid primary key references public.arena_agents(id) on delete cascade,
  rating integer not null default 1000,
  games_played integer not null default 0,
  wins integer not null default 0,
  losses integer not null default 0,
  draws integer not null default 0,
  last_match_at timestamptz,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  check (rating >= 100),
  check (games_played >= 0 and wins >= 0 and losses >= 0 and draws >= 0),
  check (games_played = wins + losses + draws)
);

create table if not exists public.krabbyclaw_arena_matches (
  id uuid primary key default gen_random_uuid(),
  challenger_agent_id uuid not null references public.arena_agents(id) on delete cascade,
  opponent_agent_id uuid not null references public.arena_agents(id) on delete cascade,
  created_by uuid not null references auth.users(id) on delete cascade,
  queue text not null default 'krabbyclaw-arena',
  status text not null default 'running' check (status in ('pending', 'running', 'completed', 'cancelled')),
  outcome text check (outcome is null or outcome in ('challenger', 'opponent', 'draw', 'cancelled')),
  winner_agent_id uuid references public.arena_agents(id) on delete set null,
  challenger_session_id text,
  opponent_session_id text,
  challenger_score integer,
  opponent_score integer,
  notes text,
  metadata jsonb not null default '{}'::jsonb,
  started_at timestamptz not null default now(),
  finished_at timestamptz,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  check (challenger_agent_id <> opponent_agent_id),
  check (winner_agent_id is null or winner_agent_id = challenger_agent_id or winner_agent_id = opponent_agent_id),
  check ((challenger_score is null or challenger_score >= 0) and (opponent_score is null or opponent_score >= 0)),
  check ((status <> 'completed') or finished_at is not null)
);

create index if not exists idx_krabbyclaw_arena_matches_status_created
  on public.krabbyclaw_arena_matches (status, created_at desc);

create index if not exists idx_krabbyclaw_arena_matches_queue_status
  on public.krabbyclaw_arena_matches (queue, status);

create index if not exists idx_krabbyclaw_arena_ratings_rating
  on public.krabbyclaw_arena_ratings (rating desc, games_played desc);

create or replace view public.krabbyclaw_arena_leaderboard as
select
  ratings.agent_id,
  agents.name as agent_name,
  agents.slug as agent_slug,
  agents.runtime,
  ratings.rating,
  ratings.games_played,
  ratings.wins,
  ratings.losses,
  ratings.draws,
  case
    when ratings.games_played = 0 then 0::double precision
    else round((ratings.wins::numeric / ratings.games_played::numeric) * 100, 2)::double precision
  end as win_rate,
  rank() over (
    order by ratings.rating desc, ratings.wins desc, ratings.games_played desc, agents.name asc
  ) as rank
from public.krabbyclaw_arena_ratings ratings
join public.arena_agents agents
  on agents.id = ratings.agent_id
where agents.visibility = 'public';

alter table public.krabbyclaw_arena_ratings enable row level security;
alter table public.krabbyclaw_arena_matches enable row level security;

create policy if not exists "Krabby arena ratings read" on public.krabbyclaw_arena_ratings
  for select using (true);
create policy if not exists "Krabby arena ratings write" on public.krabbyclaw_arena_ratings
  for all using (auth.role() = 'service_role') with check (auth.role() = 'service_role');

create policy if not exists "Krabby arena matches read" on public.krabbyclaw_arena_matches
  for select using (true);
create policy if not exists "Krabby arena matches write" on public.krabbyclaw_arena_matches
  for all using (auth.role() = 'service_role') with check (auth.role() = 'service_role');
