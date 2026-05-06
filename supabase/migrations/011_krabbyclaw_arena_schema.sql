-- KrabbyClawArena head-to-head ELO schema.

create table if not exists public.krabbyclaw_arena_ratings (
  agent_id uuid primary key references public.arena_agents(id) on delete cascade,
  rating integer not null default 1000,
  games_played integer not null default 0,
  wins integer not null default 0,
  losses integer not null default 0,
  draws integer not null default 0,
  last_match_at timestamptz,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create table if not exists public.krabbyclaw_arena_matches (
  id uuid primary key default gen_random_uuid(),
  challenger_agent_id uuid not null references public.arena_agents(id) on delete cascade,
  opponent_agent_id uuid not null references public.arena_agents(id) on delete cascade,
  created_by uuid not null references auth.users(id) on delete cascade,
  queue text not null default 'krabbyclaw-arena',
  status text not null default 'running',
  outcome text,
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
  updated_at timestamptz not null default now()
);

do $$
begin
  if not exists (
    select 1
    from pg_constraint
    where conname = 'krabbyclaw_arena_ratings_counts_consistent'
      and conrelid = 'public.krabbyclaw_arena_ratings'::regclass
  ) then
    alter table public.krabbyclaw_arena_ratings
      add constraint krabbyclaw_arena_ratings_counts_consistent check (
        rating >= 100
        and games_played >= 0
        and wins >= 0
        and losses >= 0
        and draws >= 0
        and games_played = wins + losses + draws
      );
  end if;
end $$;

do $$
begin
  if not exists (
    select 1
    from pg_constraint
    where conname = 'krabbyclaw_arena_matches_status_check'
      and conrelid = 'public.krabbyclaw_arena_matches'::regclass
  ) then
    alter table public.krabbyclaw_arena_matches
      add constraint krabbyclaw_arena_matches_status_check check (
        status in ('pending', 'running', 'completed', 'cancelled')
      );
  end if;
end $$;

do $$
begin
  if not exists (
    select 1
    from pg_constraint
    where conname = 'krabbyclaw_arena_matches_outcome_check'
      and conrelid = 'public.krabbyclaw_arena_matches'::regclass
  ) then
    alter table public.krabbyclaw_arena_matches
      add constraint krabbyclaw_arena_matches_outcome_check check (
        outcome is null or outcome in ('challenger', 'opponent', 'draw', 'cancelled')
      );
  end if;
end $$;

do $$
begin
  if not exists (
    select 1
    from pg_constraint
    where conname = 'krabbyclaw_arena_matches_agents_distinct'
      and conrelid = 'public.krabbyclaw_arena_matches'::regclass
  ) then
    alter table public.krabbyclaw_arena_matches
      add constraint krabbyclaw_arena_matches_agents_distinct check (
        challenger_agent_id <> opponent_agent_id
      );
  end if;
end $$;

do $$
begin
  if not exists (
    select 1
    from pg_constraint
    where conname = 'krabbyclaw_arena_matches_winner_valid'
      and conrelid = 'public.krabbyclaw_arena_matches'::regclass
  ) then
    alter table public.krabbyclaw_arena_matches
      add constraint krabbyclaw_arena_matches_winner_valid check (
        winner_agent_id is null
        or winner_agent_id = challenger_agent_id
        or winner_agent_id = opponent_agent_id
      );
  end if;
end $$;

do $$
begin
  if not exists (
    select 1
    from pg_constraint
    where conname = 'krabbyclaw_arena_matches_scores_nonnegative'
      and conrelid = 'public.krabbyclaw_arena_matches'::regclass
  ) then
    alter table public.krabbyclaw_arena_matches
      add constraint krabbyclaw_arena_matches_scores_nonnegative check (
        (challenger_score is null or challenger_score >= 0)
        and (opponent_score is null or opponent_score >= 0)
      );
  end if;
end $$;

do $$
begin
  if not exists (
    select 1
    from pg_constraint
    where conname = 'krabbyclaw_arena_matches_completion_timestamps'
      and conrelid = 'public.krabbyclaw_arena_matches'::regclass
  ) then
    alter table public.krabbyclaw_arena_matches
      add constraint krabbyclaw_arena_matches_completion_timestamps check (
        (status = 'completed' and finished_at is not null)
        or status <> 'completed'
      );
  end if;
end $$;

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

drop policy if exists "Krabby arena ratings read" on public.krabbyclaw_arena_ratings;
create policy "Krabby arena ratings read" on public.krabbyclaw_arena_ratings
  for select using (true);

drop policy if exists "Krabby arena ratings write" on public.krabbyclaw_arena_ratings;
create policy "Krabby arena ratings write" on public.krabbyclaw_arena_ratings
  for all using (auth.role() = 'service_role') with check (auth.role() = 'service_role');

drop policy if exists "Krabby arena matches read" on public.krabbyclaw_arena_matches;
create policy "Krabby arena matches read" on public.krabbyclaw_arena_matches
  for select using (true);

drop policy if exists "Krabby arena matches write" on public.krabbyclaw_arena_matches;
create policy "Krabby arena matches write" on public.krabbyclaw_arena_matches
  for all using (auth.role() = 'service_role') with check (auth.role() = 'service_role');

grant select on public.krabbyclaw_arena_leaderboard to anon, authenticated, service_role;
grant all on public.krabbyclaw_arena_ratings to service_role;
grant all on public.krabbyclaw_arena_matches to service_role;
