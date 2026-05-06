-- Enforce public-only agents and arena run consistency for API-based progress ingestion.

update public.arena_agents
set visibility = 'public'
where visibility <> 'public';

do $$
begin
  if not exists (
    select 1
    from pg_constraint
    where conname = 'arena_agents_public_only'
      and conrelid = 'public.arena_agents'::regclass
  ) then
    alter table public.arena_agents
      add constraint arena_agents_public_only check (visibility = 'public');
  end if;
end $$;

do $$
begin
  if not exists (
    select 1
    from pg_constraint
    where conname = 'arena_runs_nonnegative_counters'
      and conrelid = 'public.arena_runs'::regclass
  ) then
    alter table public.arena_runs
      add constraint arena_runs_nonnegative_counters check (
        (frame_count is null or frame_count >= 0)
        and (badge_count is null or badge_count >= 0)
        and (pokedex_seen is null or pokedex_seen >= 0)
        and (pokedex_caught is null or pokedex_caught >= 0)
      );
  end if;
end $$;

do $$
begin
  if not exists (
    select 1
    from pg_constraint
    where conname = 'arena_runs_time_consistency'
      and conrelid = 'public.arena_runs'::regclass
  ) then
    alter table public.arena_runs
      add constraint arena_runs_time_consistency check (
        (status <> 'queued' or started_at is null)
        and (status not in ('completed', 'failed', 'cancelled') or finished_at is not null)
        and (finished_at is null or started_at is null or finished_at >= started_at)
      );
  end if;
end $$;

do $$
begin
  if not exists (
    select 1
    from pg_constraint
    where conname = 'arena_runs_metrics_progress_valid'
      and conrelid = 'public.arena_runs'::regclass
  ) then
    alter table public.arena_runs
      add constraint arena_runs_metrics_progress_valid check (
        (metrics is null or jsonb_typeof(metrics) = 'object')
        and (
          metrics is null
          or not (metrics ? 'session_id')
          or (metrics->>'session_id') ~ '^[A-Za-z0-9][A-Za-z0-9_-]{0,63}$'
        )
        and (
          metrics is null
          or not (metrics ? 'step_count')
          or (metrics->>'step_count') ~ '^[0-9]+$'
        )
        and (
          metrics is null
          or not (metrics ? 'steps_taken')
          or (metrics->>'steps_taken') ~ '^[0-9]+$'
        )
        and (
          metrics is null
          or not (metrics ? 'command_count')
          or (metrics->>'command_count') ~ '^[0-9]+$'
        )
      );
  end if;
end $$;

create index if not exists idx_arena_runs_session_id
  on public.arena_runs ((metrics->>'session_id'));

with duplicates as (
  select id
  from (
    select
      id,
      row_number() over (
        partition by agent_id, (metrics->>'session_id')
        order by updated_at desc nulls last, created_at desc
      ) as row_rank
    from public.arena_runs
    where status in ('queued', 'running')
      and metrics ? 'session_id'
  ) ranked
  where row_rank > 1
)
update public.arena_runs
set status = 'cancelled',
    finished_at = coalesce(finished_at, now()),
    updated_at = now()
where id in (select id from duplicates);

create unique index if not exists uq_arena_runs_active_agent_session
  on public.arena_runs (agent_id, (metrics->>'session_id'))
  where status in ('queued', 'running')
    and metrics ? 'session_id';
