-- Harden live-world multiplayer queueing, match lifecycle, and realtime access.

delete from public.matchmaking_queue
where expires_at < now();

delete from public.matchmaking_queue queue
using (
  select id
  from (
    select
      id,
      row_number() over (
        partition by user_id, mode
        order by created_at desc, id desc
      ) as row_number
    from public.matchmaking_queue
  ) ranked
  where ranked.row_number > 1
) duplicates
where queue.id = duplicates.id;

create unique index if not exists uq_matchmaking_queue_user_mode
  on public.matchmaking_queue (user_id, mode);

update public.matches
set completed_at = coalesce(completed_at, now())
where status in ('completed', 'cancelled')
  and completed_at is null;

do $$
begin
  if not exists (
    select 1
    from pg_constraint
    where conname = 'matches_completion_timestamps'
      and conrelid = 'public.matches'::regclass
  ) then
    alter table public.matches
      add constraint matches_completion_timestamps check (
        (status in ('completed', 'cancelled') and completed_at is not null)
        or (status not in ('completed', 'cancelled') and completed_at is null)
      );
  end if;

  if not exists (
    select 1
    from pg_constraint
    where conname = 'matches_result_object'
      and conrelid = 'public.matches'::regclass
  ) then
    alter table public.matches
      add constraint matches_result_object check (
        result is null or jsonb_typeof(result) = 'object'
      );
  end if;
end;
$$;

create index if not exists idx_matches_channel_name
  on public.matches (channel_name);

create index if not exists idx_matches_completed
  on public.matches (completed_at desc)
  where status in ('completed', 'cancelled');

-- Realtime authorization is evaluated against realtime.messages when private
-- Realtime channels are enabled in the Supabase dashboard. These policies keep
-- live-world presence/broadcast authenticated-only while still allowing MCP/API
-- traffic through service_role.
do $$
begin
  if to_regclass('realtime.messages') is not null then
    execute 'alter table realtime.messages enable row level security';

    execute 'drop policy if exists "Authenticated multiplayer realtime read" on realtime.messages';
    execute 'create policy "Authenticated multiplayer realtime read"
      on realtime.messages for select
      using (
        auth.role() = ''service_role''
        or (
          auth.role() = ''authenticated''
          and (
            topic = ''overworld:presence''
            or topic like ''match:%''
            or topic like ''match_%''
          )
        )
      )';

    execute 'drop policy if exists "Authenticated multiplayer realtime write" on realtime.messages';
    execute 'create policy "Authenticated multiplayer realtime write"
      on realtime.messages for insert
      with check (
        auth.role() = ''service_role''
        or (
          auth.role() = ''authenticated''
          and (
            topic = ''overworld:presence''
            or topic like ''match:%''
            or topic like ''match_%''
          )
        )
      )';
  end if;
end;
$$;
