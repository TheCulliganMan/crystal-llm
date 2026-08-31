-- Persistent moderation queue and moderator actions for realtime multiplayer chat.

create table if not exists public.multiplayer_chat_reports (
  id uuid primary key default gen_random_uuid(),
  reporter_user_id uuid not null references auth.users(id) on delete cascade,
  reported_user_id uuid not null references auth.users(id) on delete cascade,
  message_id text not null,
  player_name text not null check (char_length(player_name) between 1 and 32),
  channel text not null check (channel in ('local', 'trade', 'whisper')),
  message_text text not null check (char_length(message_text) between 1 and 240),
  status text not null default 'open' check (status in ('open', 'reviewed', 'dismissed', 'actioned')),
  created_at timestamptz not null default now(),
  unique (reporter_user_id, message_id)
);

create table if not exists public.multiplayer_chat_moderation_actions (
  id uuid primary key default gen_random_uuid(),
  moderator_user_id uuid not null references auth.users(id) on delete cascade,
  target_user_id uuid not null references auth.users(id) on delete cascade,
  action text not null check (action in ('mute', 'ban')),
  reason text not null check (char_length(reason) between 1 and 500),
  expires_at timestamptz,
  revoked_at timestamptz,
  created_at timestamptz not null default now()
);

create index if not exists idx_multiplayer_chat_reports_open
  on public.multiplayer_chat_reports (created_at desc)
  where status = 'open';

create index if not exists idx_multiplayer_chat_actions_active
  on public.multiplayer_chat_moderation_actions (target_user_id, created_at desc)
  where revoked_at is null;

alter table public.multiplayer_chat_reports enable row level security;
alter table public.multiplayer_chat_moderation_actions enable row level security;

drop policy if exists "Users submit their own chat reports" on public.multiplayer_chat_reports;
create policy "Users submit their own chat reports"
  on public.multiplayer_chat_reports for insert
  to authenticated
  with check (reporter_user_id = auth.uid() and reported_user_id <> auth.uid());

grant insert on public.multiplayer_chat_reports to authenticated;
grant all on public.multiplayer_chat_reports to service_role;
grant all on public.multiplayer_chat_moderation_actions to service_role;

-- The application now uses bounded world/modpack topics instead of the legacy
-- singleton overworld topic. Keep Realtime private and authenticated-only.
do $$
begin
  if to_regclass('realtime.messages') is not null then
    execute 'drop policy if exists "Authenticated multiplayer realtime read" on realtime.messages';
    execute 'create policy "Authenticated multiplayer realtime read"
      on realtime.messages for select
      using (
        auth.role() = ''service_role''
        or (
          auth.role() = ''authenticated''
          and (
            topic like ''overworld:%''
            or topic like ''chat:%''
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
            topic like ''overworld:%''
            or topic like ''chat:%''
            or topic like ''match:%''
            or topic like ''match_%''
          )
        )
      )';
  end if;
end;
$$;
