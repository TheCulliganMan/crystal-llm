# Matchmaker Edge Function

Pairs authenticated users from `public.matchmaking_queue` into `public.matches`.

## Local

```sh
supabase start
supabase functions serve matchmaker --env-file supabase/.env.local
curl -X POST http://127.0.0.1:54321/functions/v1/matchmaker \
  -H "Authorization: Bearer $SUPABASE_SERVICE_ROLE_KEY"
```

## Production

Deploy with:

```sh
supabase functions deploy matchmaker
```

Schedule it with Supabase Cron or any external scheduler that can invoke the
function every few seconds. The function is idempotent for duplicate queue rows
because `matchmaking_queue` is unique on `(user_id, mode)` and stale entries are
cleaned before every matching cycle.
